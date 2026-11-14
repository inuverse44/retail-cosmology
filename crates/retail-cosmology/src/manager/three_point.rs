//! 使用例：店舗の三点相関関数を測る（ADR 007、notes/retail-cosmology-beyond.tex）。
//! 一つのチェーンについて、人口に比例するランダムカタログ（`footprint` を渡せば幅 R の選択関数も入れる）と比べ、
//! 三角形のビンごとに DDD, DDR, DRR, RRR と Szapudi–Szalay の ζ を CSV に書く。
//! `mock` を渡すと、店の代わりに基準と同じ重みで独立に撒いた点を使う（ζ = 0 の帰無仮説で揺らぎを見る）。
//! jackknife の誤差はまだ入れていない。
//!
//! 国ごとの読み込み先の選び方は `manager::correlation` と同じ。Manager どうしは呼び合わないので、ここにも持つ（ADR 007）。

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use crate::access::{boundary, population, results, stores};
use crate::contract::{Bins, Chain, PairCounts, PopulationGrid, TripleCounts};
use crate::engine::estimator::mock_summary;
use crate::engine::random_catalog::{Weight, sample, sample_reweighted, sample_selected};
use crate::engine::selection_function::{
    RegionalRatio, SmoothRatio, Tabulated, fit_density_exponent, mean_log_density,
};
use crate::engine::triple_count::TriangleBins;
use crate::util::geo::{LatLon, Unit, chord2, km_to_chord2};
use crate::util::rng::SplitMix64;

/// 行政区による区分一定の選択関数の細かさ。
#[derive(Clone, Copy, Debug)]
pub enum RegionalScale {
    Region,
    Prefecture,
}

/// 測り方の設定。
#[derive(Clone, Debug)]
pub struct Params {
    pub date: String,
    /// jp（e-Stat のメッシュ）か us（GHS-POP を米国本土に限ったもの）
    pub country: String,
    /// 店のファイル（リポジトリの根からの相対パス）
    pub stores: String,
    /// 測るチェーン（例：COSMOS、WALMART）
    pub chain: String,
    pub n_random: usize,
    /// 選択関数の人口の密度を表す参照の点の数
    pub n_ref: usize,
    pub seed: u64,
    /// 選択関数をならす幅 R [km]。regional も None なら人口だけに比例するランダムカタログ
    pub footprint: Option<f64>,
    /// 選択関数を約 10 km の粗い格子ごとに一度だけ求める近似を使う
    pub selection_grid: bool,
    /// 地方別または都道府県別の店舗数 / 人口を使う（clustered では追加の proc）
    pub regional: Option<RegionalScale>,
    /// 同じランダムカタログによる ξ も保存する
    pub write_xi: bool,
    /// 店の代わりに、基準と同じ重みで独立に撒いた点を使う（その種）
    pub mock: Option<u64>,
    /// 模擬データの重みに人口密度 ρ の γ 乗も掛け、人の密集地への寄り方を実際の店にそろえる
    pub mock_density: bool,
    /// 辺のビン：[lo, hi) km を対数で n 等分
    pub sides_km: (f64, f64, usize),
    /// 出力のファイル名につける名前（空でなければ "_名前" の形）
    pub tag: String,
}

impl Params {
    pub fn new(date: String) -> Self {
        Self {
            date,
            country: "jp".into(),
            stores: String::new(),
            chain: String::new(),
            n_random: 2_000,
            n_ref: 50_000,
            seed: 1,
            footprint: None,
            selection_grid: false,
            regional: None,
            write_xi: false,
            mock: None,
            mock_density: false,
            sides_km: (2.0, 32.0, 4),
            tag: String::new(),
        }
    }
}

/// 国ごとの人口の格子と、出力の名前の前置き。
fn grid_of(
    root: &Path,
    country: &str,
) -> Result<(PopulationGrid, &'static str), Box<dyn std::error::Error>> {
    match country {
        "jp" => Ok((population::read_estat(&root.join("data/estat"))?, "jp_")),
        "us" => {
            let states = boundary::read_us_states(
                &root.join("data/naturalearth/ne_10m_admin_1_states_provinces.geojson"),
            )
            .map_err(|e| format!("scripts/naturalearth/fetch.sh を先に：{e}"))?;
            let grid = population::read_ghsl(
                &root.join("data/ghsl/GHS_POP_E2020_GLOBE_R2023A_54009_1000_V1_0.tif"),
                &states,
            )
            .map_err(|e| format!("scripts/ghsl/fetch.sh を先に：{e}"))?;
            Ok((grid, "us_"))
        }
        c => Err(format!("知らない国：{c}（jp か us）").into()),
    }
}

/// 外部の行政メッシュ対応と人口から地方の対応と人口和を作る。
/// 地方は既存の七区分（中国・四国は合同、沖縄は九州）。
struct RegionalBasis {
    mesh_region: std::collections::HashMap<u64, usize>,
    population: Vec<f64>,
}

impl RegionalBasis {
    fn read(
        root: &Path,
        country: &str,
        grid: &PopulationGrid,
        stores: &[LatLon],
        scale: RegionalScale,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if country != "jp" {
            return Err("地方別選択関数は jp のみ".into());
        }
        let names: Vec<String> = match scale {
            RegionalScale::Region => [
                "北海道",
                "東北",
                "関東",
                "中部",
                "近畿",
                "中国・四国",
                "九州",
            ]
            .iter()
            .map(|n| n.to_string())
            .collect(),
            RegionalScale::Prefecture => (1..=47).map(|n| format!("JP-{n:02}")).collect(),
        };
        let prefectures = population::read_mesh_prefecture_map(&root.join("data/estat/mesh_pref"))?;
        let mut mesh_region: std::collections::HashMap<_, _> = prefectures
            .iter()
            .map(|(&code, pref)| {
                let r = match scale {
                    RegionalScale::Region => {
                        let name = crate::engine::selection::region(
                            Chain::Trial,
                            &format!("JP-{pref:02}"),
                        );
                        names.iter().position(|n| n == name).expect("地方の定義")
                    }
                    RegionalScale::Prefecture => (*pref - 1) as usize,
                };
                (code, r)
            })
            .collect();
        // 行政対応一覧にない新しい人口メッシュ・店舗メッシュは、外部の既知メッシュの
        // 最寄り中心（25 km 以内）に寄せる。店舗密度から境界を推定しない。
        let mut known: Vec<_> = mesh_region.iter().map(|(&code, &r)| (code, r)).collect();
        known.sort_unstable();
        let centres: Vec<Unit> = known
            .iter()
            .map(|(code, _)| {
                let (lat, lon) = crate::util::grid::code_to_sw(*code);
                Unit::from(&LatLon {
                    lat: lat + 1.0 / 240.0,
                    lon: lon + 1.0 / 160.0,
                })
            })
            .collect();
        let cutoff = km_to_chord2(25.0).sqrt();
        let spatial = crate::util::spatial::UnitGrid::build(&centres, cutoff);
        let mut missing = std::collections::BTreeSet::new();
        for i in 0..grid.values.len() {
            if grid.values[i] > 0.0 {
                let code = crate::util::grid::code_of(&grid.geometry.point(i, 0.5, 0.5));
                if !mesh_region.contains_key(&code) {
                    missing.insert(code);
                }
            }
        }
        for q in stores {
            let code = crate::util::grid::code_of(q);
            if !mesh_region.contains_key(&code) {
                missing.insert(code);
            }
        }
        eprintln!(
            "行政対応一覧にないメッシュ {} 個（外部メッシュの最寄り中心に補完、上限 25 km）",
            missing.len()
        );
        for code in missing {
            let (lat, lon) = crate::util::grid::code_to_sw(code);
            let u = Unit::from(&LatLon {
                lat: lat + 1.0 / 240.0,
                lon: lon + 1.0 / 160.0,
            });
            let mut best = (cutoff * cutoff, usize::MAX);
            spatial.for_each_near(&u, cutoff, |i| {
                let candidate = (chord2(&u, &centres[i]), i);
                if candidate < best {
                    best = candidate;
                }
            });
            if best.1 == usize::MAX {
                return Err(format!("メッシュ {code} の 25 km 以内に行政対応がない").into());
            }
            eprintln!(
                "補完メッシュ {code} → {}（中心間 {:.3} km）",
                known[best.1].0,
                2.0 * crate::util::geo::EARTH_RADIUS_KM * (best.0.sqrt() / 2.0).asin()
            );
            mesh_region.insert(code, known[best.1].1);
        }
        let mut population = vec![0.0; names.len()];
        for (i, &v) in grid.values.iter().enumerate() {
            if v <= 0.0 {
                continue;
            }
            let code = crate::util::grid::code_of(&grid.geometry.point(i, 0.5, 0.5));
            let r = mesh_region
                .get(&code)
                .ok_or_else(|| format!("人口メッシュ {code} に県の対応がない"))?;
            population[*r] += v as f64;
        }
        for (name, pop) in names.iter().zip(&population) {
            eprintln!("地方 {name}：人口 {pop:.0}");
        }
        Ok(Self {
            mesh_region,
            population,
        })
    }

    fn fit(&self, points: &[LatLon]) -> Result<RegionalRatio, String> {
        RegionalRatio::new(points, &self.mesh_region, &self.population)
    }

    fn fit_units(&self, points: &[Unit]) -> Result<RegionalRatio, String> {
        self.fit(
            &points
                .iter()
                .map(|u| LatLon {
                    lat: u.0[2].clamp(-1.0, 1.0).asin().to_degrees(),
                    lon: u.0[1].atan2(u.0[0]).to_degrees(),
                })
                .collect::<Vec<_>>(),
        )
    }
}

fn draw_regional(
    grid: &PopulationGrid,
    sr: &RegionalRatio,
    n: usize,
    rng: &mut SplitMix64,
) -> Vec<LatLon> {
    sample_reweighted(
        grid,
        n,
        Weight::Population,
        &sr.cell_weights(&grid.geometry, grid.values.len()),
        rng,
    )
}

/// 三点相関関数を測り、data/zeta/zeta_<前置き><日付>_<チェーン><名前>.csv を書く。
/// 途中経過と、数え上げにかかった時間は標準エラー出力に出す。書いたファイルのパスを返す。
pub fn run(root: &Path, p: &Params) -> Result<String, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let (grid, prefix) = grid_of(root, &p.country)?;
    let pts: Vec<LatLon> = stores::read_selected(&root.join(&p.stores))
        .map_err(|e| format!("{} がない：{e}", p.stores))?
        .into_iter()
        .filter(|(c, _)| *c == p.chain)
        .map(|(_, q)| q)
        .collect();
    if pts.is_empty() {
        return Err(format!("{} に {} の店がない", p.stores, p.chain).into());
    }

    // 基準の重み：人口 × 選択関数（幅 R）。選択関数は、測る点（実際の店か模擬の店）から作る。
    // 模擬データでは、実際の店の選択関数に従って店を撒き直し、その模擬の店から選択関数を作り直す
    // （実際の店と同じ手順。選択関数が店の固まりの一部を吸い込む分も、模擬データに入る）。
    let mut rng_ref = SplitMix64::new(p.seed ^ 0x3E7A_0000_0000_0000);
    let refs_ll = sample(&grid, p.n_ref, Weight::Population, &mut rng_ref);
    let refs: Vec<Unit> = refs_ll.iter().map(Unit::from).collect();
    let units: Vec<Unit> = pts.iter().map(Unit::from).collect();
    if p.regional.is_some() && (p.footprint.is_some() || p.selection_grid) {
        return Err("--regional on は --footprint / --selection-grid と同時に使えない".into());
    }
    let basis = p
        .regional
        .map(|scale| RegionalBasis::read(root, &p.country, &grid, &pts, scale))
        .transpose()?;
    let regional_real = basis.as_ref().map(|b| b.fit(&pts)).transpose()?;
    let selection = |u: &[Unit]| p.footprint.map(|r| SmoothRatio::new(u, &refs, r));
    // 選択関数の値。selection_grid なら約 10 km の格子の表で引く（表は使う所で作る）
    let draw = |sel: &Option<SmoothRatio>, n: usize, rng: &mut SplitMix64| -> Vec<LatLon> {
        match sel {
            Some(sr) if p.selection_grid => {
                let tab = Tabulated::new(sr, &grid.geometry);
                sample_selected(&grid, n, Weight::Population, |q| tab.eval(q), 2 * n, rng).points
            }
            Some(sr) => {
                sample_selected(
                    &grid,
                    n,
                    Weight::Population,
                    |q| sr.eval(&Unit::from(q)),
                    2 * n,
                    rng,
                )
                .points
            }
            None => sample(&grid, n, Weight::Population, rng),
        }
    };
    let sel_real = selection(&units);
    let tab_real = sel_real
        .as_ref()
        .filter(|_| p.selection_grid)
        .map(|sr| Tabulated::new(sr, &grid.geometry));
    let s_of = |q: &LatLon| match (&sel_real, &tab_real) {
        (_, Some(tab)) => tab.eval(q),
        (Some(sr), None) => sr.eval(&Unit::from(q)),
        _ => regional_real.as_ref().map_or(1.0, |sr| sr.eval(q)),
    };

    let data: Vec<Unit> = match p.mock {
        None => units,
        Some(s) if p.mock_density => {
            // 密集地への寄り方もそろえた模擬データ：重みは人口 × S_R × ρ^γ。γ は店の位置の ln ρ の平均が
            // 実際の店と同じになるように決める（二点のノートと同じ決め方、engine::selection_function）
            let rho = |q: &LatLon| -> Option<f64> {
                let v = grid.values[grid.geometry.cell_of(q)?] as f64;
                (v > 0.0).then_some(v)
            };
            let (lr, w): (Vec<f64>, Vec<f64>) = refs_ll
                .iter()
                .filter_map(|q| rho(q).map(|r| (r.ln(), s_of(q))))
                .unzip();
            let ls: Vec<f64> = pts.iter().filter_map(|q| rho(q).map(f64::ln)).collect();
            let target = ls.iter().sum::<f64>() / ls.len() as f64;
            let gamma = fit_density_exponent(&lr, &w, target, 3.0);
            eprintln!(
                "{}：店の ln ρ の平均 {target:.3}、基準の平均 {:.3}、γ = {gamma:.3}",
                p.chain,
                mean_log_density(&lr, &w, 0.0)
            );
            let n = pts.len();
            let mut rng_m = SplitMix64::new(s ^ 0x3E7A_4D0C_0000_0000);
            sample_selected(
                &grid,
                n,
                Weight::Population,
                |q| s_of(q) * rho(q).map_or(0.0, |r| r.powf(gamma)),
                100 * n,
                &mut rng_m,
            )
            .points
            .iter()
            .map(Unit::from)
            .collect()
        }
        Some(s) => {
            let mut rng_m = SplitMix64::new(s ^ 0x3E7A_4D0C_0000_0000);
            let points = match &regional_real {
                Some(sr) => draw_regional(&grid, sr, pts.len(), &mut rng_m),
                None => draw(&sel_real, pts.len(), &mut rng_m),
            };
            points.iter().map(Unit::from).collect()
        }
    };
    let mut rng = SplitMix64::new(p.seed);
    let randoms: Vec<Unit> = if let Some(basis) = &basis {
        let sr = basis.fit_units(&data)?;
        eprintln!("地方別の店舗数：{:?}", sr.counts);
        draw_regional(&grid, &sr, p.n_random, &mut rng)
    } else if p.mock.is_some() {
        draw(&selection(&data), p.n_random, &mut rng)
    } else {
        draw(&sel_real, p.n_random, &mut rng)
    }
    .iter()
    .map(Unit::from)
    .collect();
    eprintln!(
        "{}：店 {}{}、ランダム {} 点{}（{:.1} 秒）",
        p.chain,
        data.len(),
        p.mock
            .map_or(String::new(), |s| format!("（模擬データ、種 {s}）")),
        randoms.len(),
        p.footprint
            .map_or(String::new(), |r| format!("、選択関数 R = {r} km")),
        t0.elapsed().as_secs_f64()
    );

    let (lo, hi, n) = p.sides_km;
    let sides_km = Bins::log(lo, hi, n);
    let bins = TriangleBins::new(sides_km.map(km_to_chord2));
    let t1 = Instant::now();
    let tc = TripleCounts::compute(&data, &randoms, &bins);
    eprintln!("三つ組の数え上げ（{:.1} 秒）", t1.elapsed().as_secs_f64());
    let zeta = tc.szapudi_szalay();

    let c = sides_km.centers();
    let mut out = String::from("k1,k2,k3,r1_km,r2_km,r3_km,ddd,ddr,drr,rrr,zeta\n");
    for b in bins.used() {
        let (a, m, z) = bins.unflatten(b);
        writeln!(
            out,
            "{a},{m},{z},{:.3},{:.3},{:.3},{},{},{},{},{:.6}",
            c[a], c[m], c[z], tc.ddd[b], tc.ddr[b], tc.drr[b], tc.rrr[b], zeta[b]
        )?;
    }
    let path = format!(
        "data/zeta/zeta_{prefix}{}_{}{}.csv",
        p.date,
        p.chain.to_lowercase(),
        p.tag
    );
    results::write_text(&root.join(&path), &out)?;
    if p.write_xi {
        let xi = PairCounts::compute(&data, &randoms, &sides_km.map(km_to_chord2), chord2)
            .landy_szalay();
        let mut xo = String::from("k,r_km,xi\n");
        for (k, x) in xi.iter().enumerate() {
            writeln!(xo, "{k},{:.3},{x:.6}", c[k])?;
        }
        let xi_path = path.replace(".csv", "_xi.csv");
        results::write_text(&root.join(&xi_path), &xo)?;
        eprintln!("{xi_path} を書いた");
    }
    eprintln!(
        "{path} を書いた（全体 {:.1} 秒）",
        t0.elapsed().as_secs_f64()
    );
    Ok(path)
}

/// toy model の範囲：(南端の緯度, 緯度の幅, 西端の経度, 経度の幅) [度]。
type Patch = (f64, f64, f64, f64);
/// 北緯 33–34 度、東経 130–131.2 度（約 110 km 四方）
const SMALL_PATCH: Patch = (33.0, 1.0, 130.0, 1.2);
/// 北緯 33–36 度、東経 130–133.6 度（約 330 km 四方）。選択関数の幅 50 km より十分広い
const LARGE_PATCH: Patch = (33.0, 3.0, 130.0, 3.6);

/// 範囲 `patch` に一様に撒いた点（緯度と経度について一様）。
fn patch_uniform(n: usize, patch: Patch, rng: &mut SplitMix64) -> Vec<LatLon> {
    let (lat0, dlat, lon0, dlon) = patch;
    (0..n)
        .map(|_| LatLon {
            lat: lat0 + dlat * rng.next_f64(),
            lon: lon0 + dlon * rng.next_f64(),
        })
        .collect()
}

/// 小さい範囲（約 110 km 四方）に一様に撒いた点。
fn toy_uniform(n: usize, rng: &mut SplitMix64) -> Vec<LatLon> {
    patch_uniform(n, SMALL_PATCH, rng)
}

/// 小さい範囲で、親 `n_p` 個（一様）のまわりに、標準偏差 `s` km の正規分布で `m` 個ずつ子を撒く（Neyman–Scott の模型）。
fn toy_clustered(n_p: usize, m: usize, s: f64, rng: &mut SplitMix64) -> Vec<LatLon> {
    patch_clustered(n_p, m, s, SMALL_PATCH, rng)
}

/// 範囲 `patch` で、親 `n_p` 個（一様）のまわりに、標準偏差 `s` km の正規分布で `m` 個ずつ子を撒く。
/// 子は親ごとに続けて並ぶ（番号 c·m から m 個が親 c の子）。
fn patch_clustered(
    n_p: usize,
    m: usize,
    s: f64,
    patch: Patch,
    rng: &mut SplitMix64,
) -> Vec<LatLon> {
    let parents = patch_uniform(n_p, patch, rng);
    let mut out = Vec::with_capacity(n_p * m);
    for p in &parents {
        for _ in 0..m {
            let (u1, u2) = (rng.next_f64().max(1e-12), rng.next_f64());
            let r = s * (-2.0 * u1.ln()).sqrt();
            let a = std::f64::consts::TAU * u2;
            out.push(LatLon {
                lat: p.lat + r * a.sin() / 111.2,
                lon: p.lon + r * a.cos() / (111.2 * p.lat.to_radians().cos()),
            });
        }
    }
    out
}

/// toy model で推定量を確かめ、data/toy/zeta_toy.csv に一回ごとの ζ を書く。
/// - uniform：一様な 600 点、辺 2–20 km の一つのビン。見込みは 0。
/// - neyman_scott：親 60 個のまわりに標準偏差 2 km で 10 個ずつ子を撒いた点、辺 1–6 km の一つのビン。
///   見込みは「三点が同じ親の子である三角形の数 ÷ 一様に撒いたときに見込まれる三角形の数」（一回ごとに求める）。
///
/// ランダムカタログは同じ範囲に一様な 2000 点。engine::estimator のテストと同じ設定で、種も同じ（11 と 12）。
pub fn toy(root: &Path, runs: usize) -> Result<String, Box<dyn std::error::Error>> {
    let c3 = |n: f64| n * (n - 1.0) * (n - 2.0) / 6.0;
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let one_bin =
        |lo: f64, hi: f64| TriangleBins::new(Bins::new(vec![km_to_chord2(lo), km_to_chord2(hi)]));
    let mut out = String::from("model,run,zeta,expected\n");

    let mut rng = SplitMix64::new(11);
    let bins = one_bin(2.0, 20.0);
    for run in 0..runs {
        let d = units(&toy_uniform(600, &mut rng));
        let r = units(&toy_uniform(2000, &mut rng));
        let z = TripleCounts::compute(&d, &r, &bins).szapudi_szalay()[0];
        writeln!(out, "uniform,{run},{z:.6},0")?;
    }

    let mut rng = SplitMix64::new(12);
    let bins = one_bin(1.0, 6.0);
    let r = units(&toy_uniform(2000, &mut rng));
    let m = 10;
    for run in 0..runs {
        let d = units(&toy_clustered(60, m, 2.0, &mut rng));
        let t = TripleCounts::compute(&d, &r, &bins);
        let same: u64 = same_parent_triples(&d, m, &bins).iter().sum();
        let base = t.rrr[0] as f64 * c3(d.len() as f64) / c3(r.len() as f64);
        writeln!(
            out,
            "neyman_scott,{run},{:.6},{:.6}",
            t.szapudi_szalay()[0],
            same as f64 / base
        )?;
    }
    let path = "data/toy/zeta_toy.csv".to_string();
    results::write_text(&root.join(&path), &out)?;
    Ok(path)
}

/// 同じ親の子である三つ組のうち、三角形のビンに入るものをビンごとに数える（子は親ごとに `m` 個続けて並ぶ）。
fn same_parent_triples(d: &[Unit], m: usize, bins: &TriangleBins) -> Vec<u64> {
    let mut out = vec![0u64; bins.len()];
    for g in d.chunks(m) {
        for i in 0..g.len() {
            for j in i + 1..g.len() {
                for k in j + 1..g.len() {
                    if let Some(b) = bins.index(
                        chord2(&g[i], &g[j]),
                        chord2(&g[i], &g[k]),
                        chord2(&g[j], &g[k]),
                    ) {
                        out[b] += 1;
                    }
                }
            }
        }
    }
    out
}

/// 選択関数 `sr` に比例する密度で、範囲 `patch` に `n` 点を撒く（一様な候補を S/S_max の確率で残す棄却法）。
/// S_max は最初の候補 4n 点での最大値。
fn patch_selected(sr: &SmoothRatio, n: usize, patch: Patch, rng: &mut SplitMix64) -> Vec<LatLon> {
    let first = patch_uniform(4 * n, patch, rng);
    let s: Vec<f64> = first.iter().map(|q| sr.eval(&Unit::from(q))).collect();
    let smax = s.iter().copied().fold(0.0, f64::max);
    let mut out = Vec::with_capacity(n);
    for (q, v) in first.iter().zip(&s) {
        if out.len() == n {
            return out;
        }
        if rng.next_f64() < v / smax {
            out.push(*q);
        }
    }
    while out.len() < n {
        let q = patch_uniform(1, patch, rng)[0];
        if rng.next_f64() < sr.eval(&Unit::from(&q)) / smax {
            out.push(q);
        }
    }
    out
}

/// 店から作った選択関数が、二点で固まった店の ζ̂ をどれだけずらすかを toy model で測る（#81）。
///
/// 約 330 km 四方に、親 350 個のまわりに標準偏差 25 km で 5 個ずつ子を撒いた点（Neyman–Scott の模型、1,750 点）を作り、
/// 同じ点に二通りのランダムカタログ（8,000 点）を当てる。
/// - uniform：範囲に一様（この toy の人口は一様なので、正しい基準）。
/// - selection：点そのものから作った幅 50 km の選択関数に比例（実際の店と同じ手順）。参照の点は一様な 20,000 点。
///
/// 三角形のビンは実際の店と同じ辺 2–32 km の 4 ビン。見込みの ζ は、同じ親の子である三角形の数を、
/// 一様に撒いたときに見込まれる三角形の数（uniform の RRR を点の数で写したもの）で割った値。
/// data/toy/zeta_selection_bias.csv に、一回・ビンごとの値を書く。
pub fn selection_bias_toy(root: &Path, runs: usize) -> Result<String, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let c3 = |n: f64| n * (n - 1.0) * (n - 2.0) / 6.0;
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let sides_km = Bins::log(2.0, 32.0, 4);
    let bins = TriangleBins::new(sides_km.map(km_to_chord2));
    let c = sides_km.centers();
    let (n_p, m, s_km, r_km, n_r, n_ref) = (350, 5, 5.0, 50.0, 8_000, 20_000);

    let mut rng = SplitMix64::new(13);
    let refs = units(&patch_uniform(n_ref, LARGE_PATCH, &mut rng));
    let r_uni = units(&patch_uniform(n_r, LARGE_PATCH, &mut rng));
    let mut out =
        String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,ddd,zeta_uniform,zeta_selection,expected\n");
    // ビンごとに、一回ごとの (ζ̂ uniform, ζ̂ selection, 見込み) を集める
    let mut per_bin: Vec<Vec<(f64, f64, f64)>> = vec![Vec::new(); bins.len()];
    for run in 0..runs {
        let d = units(&patch_clustered(n_p, m, s_km, LARGE_PATCH, &mut rng));
        let t_u = TripleCounts::compute(&d, &r_uni, &bins);
        let sr = SmoothRatio::new(&d, &refs, r_km);
        let r_sel = units(&patch_selected(&sr, n_r, LARGE_PATCH, &mut rng));
        let t_s = TripleCounts::compute(&d, &r_sel, &bins);
        let (zu, zs) = (t_u.szapudi_szalay(), t_s.szapudi_szalay());
        let same = same_parent_triples(&d, m, &bins);
        let scale = c3(d.len() as f64) / c3(r_uni.len() as f64);
        for b in bins.used() {
            if t_u.rrr[b] == 0 || t_s.rrr[b] == 0 {
                continue;
            }
            let (x, y, z) = bins.unflatten(b);
            let expected = same[b] as f64 / (t_u.rrr[b] as f64 * scale);
            per_bin[b].push((zu[b], zs[b], expected));
            writeln!(
                out,
                "{run},{x},{y},{z},{:.3},{:.3},{:.3},{},{:.6},{:.6},{expected:.6}",
                c[x], c[y], c[z], t_u.ddd[b], zu[b], zs[b]
            )?;
        }
        eprintln!("{} 回目（{:.1} 秒）", run + 1, t0.elapsed().as_secs_f64());
    }
    let path = "data/toy/zeta_selection_bias.csv".to_string();
    results::write_text(&root.join(&path), &out)?;

    // ビンごとのまとめ：回数、平均と標準偏差（ζ̂ uniform、ζ̂ selection、見込み）、ずれ（selection − uniform）の平均と平均の誤差
    let mean_sd = |v: &[f64]| {
        let n = v.len() as f64;
        let m = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        (m, sd)
    };
    let mut sum = String::from(
        "k1,k2,k3,r1_km,r2_km,r3_km,runs,uniform_mean,uniform_sd,selection_mean,selection_sd,expected_mean,shift_mean,shift_err\n",
    );
    for b in bins.used() {
        let v = &per_bin[b];
        if v.len() < 2 {
            continue;
        }
        let (x, y, z) = bins.unflatten(b);
        let (um, us) = mean_sd(&v.iter().map(|t| t.0).collect::<Vec<_>>());
        let (sm, ss) = mean_sd(&v.iter().map(|t| t.1).collect::<Vec<_>>());
        let (em, _) = mean_sd(&v.iter().map(|t| t.2).collect::<Vec<_>>());
        let (dm, ds) = mean_sd(&v.iter().map(|t| t.1 - t.0).collect::<Vec<_>>());
        writeln!(
            sum,
            "{x},{y},{z},{:.3},{:.3},{:.3},{},{um:.6},{us:.6},{sm:.6},{ss:.6},{em:.6},{dm:.6},{:.6}",
            c[x],
            c[y],
            c[z],
            v.len(),
            ds / (v.len() as f64).sqrt()
        )?;
    }
    results::write_text(&root.join("data/toy/zeta_selection_bias_summary.csv"), &sum)?;
    // 本文に書くビンの数は手で数えず、ここで数える（振り返り 2026-11-14 の T-03）
    let shifts: Vec<(f64, f64)> = sum
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            Some((f.get(12)?.parse().ok()?, f.get(13)?.parse().ok()?))
        })
        .collect();
    let neg = shifts.iter().filter(|(m, _)| *m < 0.0).count();
    let beyond3: Vec<f64> = shifts
        .iter()
        .filter(|(m, e)| m.abs() > 3.0 * e)
        .map(|(m, _)| *m)
        .collect();
    eprintln!(
        "ビン {} 個：ずれが負 {neg}、平均の誤差の 3 倍を超える {}（{:+.3}〜{:+.3}）",
        shifts.len(),
        beyond3.len(),
        beyond3.iter().copied().fold(f64::INFINITY, f64::min),
        beyond3.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    );
    Ok(path)
}

/// 二点の固まりを実際の店にそろえ、ζ がわかっている模擬データ（Neyman–Scott の模型）を作り、
/// 選択関数の偏りを実際の地形の上で測る（#81）。
///
/// 親は実際の店から作った選択関数と人口に比例して撒き、その親のまわりに標準偏差 `sigma_km` で `m` 個ずつ子を撒く。
/// 子の数の合計は実際の店の数に合わせる（親の数は店の数 ÷ `m`、端数は切り上げて余りを捨てる）。
/// 一つの模擬データを二通りに測る。
/// - true：模擬データを作った重み（実際の店の選択関数 × 人口）で撒いたランダムカタログ。正しい基準にあたる。
/// - proc：模擬データそのものから選択関数を作り直したランダムカタログ。実際の店と同じ手順。
/// - regional（指定時）：同じ模擬データの行政区別店舗数 / 人口で作ったランダムカタログ。
///
/// 見込みの ζ は、同じ親の子である三角形の数を、true の RRR を点の数で写した値で割ったもの。
/// 実際の店の ξ（true と同じランダムカタログ）も測り、run = -1 の行に書く。
/// 書くもの：data/zeta/clustered_<前置き><日付>_<チェーン>_m<m>_s<σ>.csv（ζ）と、同じ名前の _xi.csv（ξ）。
pub fn clustered_mocks(
    root: &Path,
    p: &Params,
    m: usize,
    sigma_km: f64,
    runs: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let r_km = p.footprint.ok_or("--footprint R が要る")?;
    let (grid, prefix) = grid_of(root, &p.country)?;
    let pts: Vec<LatLon> = stores::read_selected(&root.join(&p.stores))
        .map_err(|e| format!("{} がない：{e}", p.stores))?
        .into_iter()
        .filter(|(c, _)| *c == p.chain)
        .map(|(_, q)| q)
        .collect();
    if pts.is_empty() {
        return Err(format!("{} に {} の店がない", p.stores, p.chain).into());
    }
    if runs == 0 || m == 0 || sigma_km <= 0.0 {
        return Err("runs, children, sigma は正の値が要る".into());
    }
    let basis = p
        .regional
        .map(|scale| RegionalBasis::read(root, &p.country, &grid, &pts, scale))
        .transpose()?;
    let mut rng_ref = SplitMix64::new(p.seed ^ 0x3E7A_0000_0000_0000);
    let refs: Vec<Unit> = sample(&grid, p.n_ref, Weight::Population, &mut rng_ref)
        .iter()
        .map(Unit::from)
        .collect();
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let draw = |sr: &SmoothRatio, n: usize, rng: &mut SplitMix64| -> Vec<LatLon> {
        sample_selected(
            &grid,
            n,
            Weight::Population,
            |q| sr.eval(&Unit::from(q)),
            2 * n,
            rng,
        )
        .points
    };
    let real = units(&pts);
    let sr_real = SmoothRatio::new(&real, &refs, r_km);
    let mut rng = SplitMix64::new(p.seed);
    let r_true = units(&draw(&sr_real, p.n_random, &mut rng));

    let (lo, hi, n) = p.sides_km;
    let sides_km = Bins::log(lo, hi, n);
    let cb = sides_km.map(km_to_chord2);
    let bins = TriangleBins::new(cb.clone());
    let c = sides_km.centers();
    let c3 = |n: f64| n * (n - 1.0) * (n - 2.0) / 6.0;
    let xi_of = |d: &[Unit], r: &[Unit]| PairCounts::compute(d, r, &cb, chord2).landy_szalay();

    let regional_real_randoms = basis
        .as_ref()
        .map(|basis| {
            let sr = basis.fit(&pts)?;
            let mut rng_r = SplitMix64::new(p.seed);
            Ok::<_, String>(units(&draw_regional(&grid, &sr, p.n_random, &mut rng_r)))
        })
        .transpose()?;
    let real_xi_regional = regional_real_randoms.as_ref().map(|r| xi_of(&real, r));
    let mut xi_out = if p.regional.is_some() {
        String::from("run,k,r_km,xi_true,xi_proc,xi_regional\n")
    } else {
        String::from("run,k,r_km,xi_true,xi_proc\n")
    };
    for (k, v) in xi_of(&real, &r_true).iter().enumerate() {
        if let Some(xi) = &real_xi_regional {
            writeln!(xi_out, "-1,{k},{:.3},{v:.6},,{:.6}", c[k], xi[k])?;
        } else {
            writeln!(xi_out, "-1,{k},{:.3},{v:.6},", c[k])?;
        }
    }
    let mut z_out = String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected\n");
    if p.regional.is_some() {
        z_out = z_out.replace("expected\n", "expected,zeta_regional\n");
    }
    let n_p = pts.len().div_ceil(m);
    let mut rng_m = SplitMix64::new(p.seed ^ 0xC1A5_7E00_0000_0000);
    // 点の池：基準の重み（人口 × 実際の店の選択関数）で撒いた点。親も子もこの池から選ぶので、
    // 模擬の店は人の住む格子にしか置かれず、基準とのずれ（ζ̂ ≈ ⟨(w−1)³⟩）が入らない。
    let pool = units(&draw(&sr_real, 20 * p.n_random, &mut rng_m));
    let cut = km_to_chord2(3.0 * sigma_km);
    let two_s2 = 2.0 * km_to_chord2(sigma_km);
    let g_pool = crate::util::spatial::UnitGrid::build(&pool, cut.sqrt());
    eprintln!(
        "点の池 {} 点（{:.1} 秒）",
        pool.len(),
        t0.elapsed().as_secs_f64()
    );
    for run in 0..runs {
        // 親を池から一様に選び、子は親から 3σ 以内の池の点を、距離の正規分布の重みで m 個選ぶ（同じ点は選ばない）。
        // 子は親ごとに m 個続けて並ぶ。店の数を超えた分は最後の親の子から捨てる
        // 近くの池の点が m 個に満たない親は使わず、選び直す（正解の数え上げで、子を m 個ずつ区切るため）
        let mut d = Vec::with_capacity(n_p * m);
        while d.len() < n_p * m {
            let parent = pool[(rng_m.next_f64() * pool.len() as f64) as usize];
            let mut near: Vec<(usize, f64)> = Vec::new();
            g_pool.for_each_near(&parent, cut.sqrt(), |i| {
                let c2 = chord2(&parent, &pool[i]);
                if c2 < cut {
                    near.push((i, (-c2 / two_s2).exp()));
                }
            });
            if near.len() < m {
                continue;
            }
            for _ in 0..m {
                let total: f64 = near.iter().map(|x| x.1).sum();
                let mut u = rng_m.next_f64() * total;
                let mut pick = near.len() - 1;
                for (j, x) in near.iter().enumerate() {
                    if u < x.1 {
                        pick = j;
                        break;
                    }
                    u -= x.1;
                }
                d.push(pool[near.swap_remove(pick).0]);
            }
        }
        d.truncate(pts.len());
        let sr_mock = SmoothRatio::new(&d, &refs, r_km);
        let r_proc = units(&draw(&sr_mock, p.n_random, &mut rng_m));
        let r_regional = basis
            .as_ref()
            .map(|basis| {
                let sr = basis.fit_units(&d)?;
                let mut rng_r = SplitMix64::new(p.seed ^ 0xAE61_0000_0000_0000 ^ (run as u64 + 1));
                Ok::<_, String>(units(&draw_regional(&grid, &sr, p.n_random, &mut rng_r)))
            })
            .transpose()?;
        let xr = r_regional.as_ref().map(|r| xi_of(&d, r));
        for (k, (a, b)) in xi_of(&d, &r_true)
            .iter()
            .zip(xi_of(&d, &r_proc))
            .enumerate()
        {
            write!(xi_out, "{run},{k},{:.3},{a:.6},{b:.6}", c[k])?;
            if let Some(xr) = &xr {
                write!(xi_out, ",{:.6}", xr[k])?;
            }
            writeln!(xi_out)?;
        }
        let t_true = TripleCounts::compute(&d, &r_true, &bins);
        let t_proc = TripleCounts::compute(&d, &r_proc, &bins);
        let (zt, zp) = (t_true.szapudi_szalay(), t_proc.szapudi_szalay());
        let tr = r_regional
            .as_ref()
            .map(|r| TripleCounts::compute(&d, r, &bins));
        let zr = tr.as_ref().map(TripleCounts::szapudi_szalay);
        let same = same_parent_triples(&d, m, &bins);
        let scale = c3(d.len() as f64) / c3(r_true.len() as f64);
        for b in bins.used() {
            if t_true.rrr[b] == 0
                || t_proc.rrr[b] == 0
                || tr.as_ref().is_some_and(|counts| counts.rrr[b] == 0)
            {
                continue;
            }
            let (x, y, z) = bins.unflatten(b);
            write!(
                z_out,
                "{run},{x},{y},{z},{:.3},{:.3},{:.3},{:.6},{:.6},{:.6}",
                c[x],
                c[y],
                c[z],
                zt[b],
                zp[b],
                same[b] as f64 / (t_true.rrr[b] as f64 * scale)
            )?;
            if let Some(zr) = &zr {
                write!(z_out, ",{:.6}", zr[b])?;
            }
            writeln!(z_out)?;
        }
        eprintln!("{} 回目（{:.1} 秒）", run + 1, t0.elapsed().as_secs_f64());
    }
    let stem = format!(
        "data/zeta/clustered_{prefix}{}_{}_m{m}_s{sigma_km}{}",
        p.date,
        p.chain.to_lowercase(),
        p.tag
    );
    results::write_text(&root.join(format!("{stem}_xi.csv")), &xi_out)?;
    let path = format!("{stem}.csv");
    results::write_text(&root.join(&path), &z_out)?;
    Ok(path)
}

/// ζ = 0 の模擬データ（ガウス場で密度を揺らす模型）を、日本の実際の地形の上で作って測る（#81）。
///
/// 店の密度を「基準 × max(0, 1 + δ)」とする。δ は、点の池から選んだ `n_centers` 個の中心に、正負の符号を半々に
/// つけた幅 `sigma_km` のガウス関数を足し、各点で sqrt(Σ K²) で割って分散をそろえ、`amp` を掛けたもの。
/// 符号が対称なので δ の三次のモーメントは 0 で、三点相関関数の見込みは 0（1 + δ < 0 を 0 で切った分だけ、わずかに正）。
/// 二点相関関数はおよそ `amp`² × （幅 √2 σ のガウス関数）。
/// 店は、基準の重み（人口 × 実際の店の選択関数）で撒いた点の池から、重み max(0, 1 + δ) で店の数だけ選ぶ（同じ点は選ばない）。
/// 測り方と書くものの形は [`clustered_mocks`] と同じ（expected の列は 0）。
/// 書くもの：data/zeta/field_<前置き><日付>_<チェーン>_a<amp>_s<σ>.csv と _xi.csv
pub fn field_mocks(
    root: &Path,
    p: &Params,
    amp: f64,
    sigma_km: f64,
    n_centers: usize,
    runs: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let r_km = p.footprint.ok_or("--footprint R が要る")?;
    let (grid, prefix) = grid_of(root, &p.country)?;
    let pts: Vec<LatLon> = stores::read_selected(&root.join(&p.stores))
        .map_err(|e| format!("{} がない：{e}", p.stores))?
        .into_iter()
        .filter(|(c, _)| *c == p.chain)
        .map(|(_, q)| q)
        .collect();
    if pts.is_empty() {
        return Err(format!("{} に {} の店がない", p.stores, p.chain).into());
    }
    let mut rng_ref = SplitMix64::new(p.seed ^ 0x3E7A_0000_0000_0000);
    let refs: Vec<Unit> = sample(&grid, p.n_ref, Weight::Population, &mut rng_ref)
        .iter()
        .map(Unit::from)
        .collect();
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let draw = |sr: &SmoothRatio, n: usize, rng: &mut SplitMix64| -> Vec<LatLon> {
        sample_selected(
            &grid,
            n,
            Weight::Population,
            |q| sr.eval(&Unit::from(q)),
            2 * n,
            rng,
        )
        .points
    };
    let real = units(&pts);
    let sr_real = SmoothRatio::new(&real, &refs, r_km);
    let mut rng = SplitMix64::new(p.seed);
    let r_true = units(&draw(&sr_real, p.n_random, &mut rng));

    let (lo, hi, n) = p.sides_km;
    let sides_km = Bins::log(lo, hi, n);
    let cb = sides_km.map(km_to_chord2);
    let bins = TriangleBins::new(cb.clone());
    let c = sides_km.centers();
    let xi_of = |d: &[Unit], r: &[Unit]| PairCounts::compute(d, r, &cb, chord2).landy_szalay();

    let mut xi_out = String::from("run,k,r_km,xi_true,xi_proc\n");
    for (k, v) in xi_of(&real, &r_true).iter().enumerate() {
        writeln!(xi_out, "-1,{k},{:.3},{v:.6},", c[k])?;
    }
    let mut z_out = String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected\n");
    let mut rng_m = SplitMix64::new(p.seed ^ 0xF1E1_D000_0000_0000);
    let pool = units(&draw(&sr_real, 20 * p.n_random, &mut rng_m));
    let cut = km_to_chord2(3.0 * sigma_km);
    let two_s2 = 2.0 * km_to_chord2(sigma_km);
    eprintln!(
        "点の池 {} 点（{:.1} 秒）",
        pool.len(),
        t0.elapsed().as_secs_f64()
    );
    for run in 0..runs {
        // 中心と符号を選び、池の各点で δ を求める
        let centers: Vec<(Unit, f64)> = (0..n_centers)
            .map(|_| {
                let q = pool[(rng_m.next_f64() * pool.len() as f64) as usize];
                (q, if rng_m.next_f64() < 0.5 { 1.0 } else { -1.0 })
            })
            .collect();
        let cu: Vec<Unit> = centers.iter().map(|x| x.0).collect();
        let g = crate::util::spatial::UnitGrid::build(&cu, cut.sqrt());
        let mut clipped = 0usize;
        let w: Vec<f64> = pool
            .iter()
            .map(|q| {
                let (mut s, mut s2) = (0.0, 0.0);
                g.for_each_near(q, cut.sqrt(), |j| {
                    let c2 = chord2(q, &cu[j]);
                    if c2 < cut {
                        let k = (-c2 / two_s2).exp();
                        s += centers[j].1 * k;
                        s2 += k * k;
                    }
                });
                let delta = if s2 > 0.0 { amp * s / s2.sqrt() } else { 0.0 };
                if 1.0 + delta < 0.0 {
                    clipped += 1;
                }
                (1.0 + delta).max(0.0)
            })
            .collect();
        // 重み w で、池から店の数だけ選ぶ（棄却法、同じ点は選ばない）
        let wmax = w.iter().copied().fold(0.0, f64::max);
        let mut taken = vec![false; pool.len()];
        let mut d = Vec::with_capacity(pts.len());
        while d.len() < pts.len() {
            let i = (rng_m.next_f64() * pool.len() as f64) as usize;
            if !taken[i] && rng_m.next_f64() < w[i] / wmax {
                taken[i] = true;
                d.push(pool[i]);
            }
        }
        let sr_mock = SmoothRatio::new(&d, &refs, r_km);
        let r_proc = units(&draw(&sr_mock, p.n_random, &mut rng_m));
        for (k, (a, b)) in xi_of(&d, &r_true)
            .iter()
            .zip(xi_of(&d, &r_proc))
            .enumerate()
        {
            writeln!(xi_out, "{run},{k},{:.3},{a:.6},{b:.6}", c[k])?;
        }
        let t_true = TripleCounts::compute(&d, &r_true, &bins);
        let t_proc = TripleCounts::compute(&d, &r_proc, &bins);
        let (zt, zp) = (t_true.szapudi_szalay(), t_proc.szapudi_szalay());
        for b in bins.used() {
            if t_true.rrr[b] == 0 || t_proc.rrr[b] == 0 {
                continue;
            }
            let (x, y, z) = bins.unflatten(b);
            writeln!(
                z_out,
                "{run},{x},{y},{z},{:.3},{:.3},{:.3},{:.6},{:.6},0",
                c[x], c[y], c[z], zt[b], zp[b]
            )?;
        }
        eprintln!(
            "{} 回目（1 + δ < 0 で切った点 {:.1}%、{:.1} 秒）",
            run + 1,
            100.0 * clipped as f64 / pool.len() as f64,
            t0.elapsed().as_secs_f64()
        );
    }
    let stem = format!(
        "data/zeta/field_{prefix}{}_{}_a{amp}_s{sigma_km}{}",
        p.date,
        p.chain.to_lowercase(),
        p.tag
    );
    results::write_text(&root.join(format!("{stem}_xi.csv")), &xi_out)?;
    let path = format!("{stem}.csv");
    results::write_text(&root.join(&path), &z_out)?;
    Ok(path)
}

/// zeta_clustered が書いた模擬データ（`stem`.csv と `stem`_xi.csv）を、ビンごとにまとめて `stem`_summary.csv と
/// `stem`_xi_summary.csv に書く。実際の店の ζ̂（zeta が書いた `real`、同じランダムカタログと設定のもの）も並べ、
/// 実際と同じ手順（proc）の模擬データの分布の中でどこにあるかを数える。
pub fn summarize_clustered(
    root: &Path,
    stem: &str,
    real: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let rows = results::read_numeric_csv(&root.join(format!("{stem}.csv")))?;
    if rows
        .first()
        .is_some_and(|r| r.contains_key("zeta_regional"))
    {
        let mut out = String::from(
            "k1,k2,k3,r1_km,r2_km,r3_km,runs,smooth_bias,smooth_se,regional_bias,regional_se,regional_minus_smooth,regional_minus_smooth_se\n",
        );
        let keys: std::collections::BTreeSet<_> = rows
            .iter()
            .map(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize))
            .collect();
        for (a, b, c) in keys {
            let v: Vec<_> = rows
                .iter()
                .filter(|r| r["k1"] as usize == a && r["k2"] as usize == b && r["k3"] as usize == c)
                .collect();
            let stats = |xs: Vec<f64>| {
                let summary = mock_summary(0.0, &xs);
                (summary.mean, summary.sd / (summary.n as f64).sqrt())
            };
            let (sm, ss) = stats(v.iter().map(|r| r["zeta_true"] - r["zeta_proc"]).collect());
            let (rm, rs) = stats(
                v.iter()
                    .map(|r| r["zeta_true"] - r["zeta_regional"])
                    .collect(),
            );
            let (im, is) = stats(
                v.iter()
                    .map(|r| r["zeta_regional"] - r["zeta_proc"])
                    .collect(),
            );
            writeln!(
                out,
                "{a},{b},{c},{:.3},{:.3},{:.3},{},{sm:.6},{ss:.6},{rm:.6},{rs:.6},{im:.6},{is:.6}",
                v[0]["r1_km"],
                v[0]["r2_km"],
                v[0]["r3_km"],
                v.len()
            )?;
        }
        results::write_text(&root.join(format!("{stem}_bias_summary.csv")), &out)?;
    }
    let xis = results::read_numeric_csv(&root.join(format!("{stem}_xi.csv")))?;
    let real = results::read_zeta(&root.join(real))?;
    let mean_sd = |v: &[f64]| {
        let n = v.len() as f64;
        let m = v.iter().sum::<f64>() / n;
        (
            m,
            (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt(),
        )
    };
    let key = |r: &std::collections::HashMap<String, f64>| {
        (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize)
    };

    let mut out = String::from(
        "k1,k2,k3,r1_km,r2_km,r3_km,runs,true_mean,true_sd,proc_mean,proc_sd,expected_mean,real,real_sigma_proc,n_le_real\n",
    );
    // 本文に書くビンの数は手で数えず、ここで数える（振り返り 2026-11-14 の T-03）
    let (mut n_bins, mut n_below_all, mut n_within_1s, mut n_proc_below_exp) = (0, 0, 0, 0);
    let (mut sig_min, mut sig_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for z in &real {
        let v: Vec<_> = rows.iter().filter(|r| key(r) == z.k).collect();
        if v.len() < 2 {
            continue;
        }
        let col = |name: &str| v.iter().map(|r| r[name]).collect::<Vec<f64>>();
        let (tm, ts) = mean_sd(&col("zeta_true"));
        let (pm, ps) = mean_sd(&col("zeta_proc"));
        let (em, _) = mean_sd(&col("expected"));
        let n_le = col("zeta_proc").iter().filter(|x| **x <= z.zeta).count();
        let sig = (z.zeta - pm) / ps;
        n_bins += 1;
        n_below_all += usize::from(n_le == 0);
        n_within_1s += usize::from(sig.abs() <= 1.0);
        n_proc_below_exp += usize::from(pm < em);
        sig_min = sig_min.min(sig);
        sig_max = sig_max.max(sig);
        let (a, b, c) = z.r_km;
        writeln!(
            out,
            "{},{},{},{a:.3},{b:.3},{c:.3},{},{tm:.6},{ts:.6},{pm:.6},{ps:.6},{em:.6},{:.6},{:.2},{n_le}",
            z.k.0,
            z.k.1,
            z.k.2,
            v.len(),
            z.zeta,
            (z.zeta - pm) / ps
        )?;
    }
    results::write_text(&root.join(format!("{stem}_summary.csv")), &out)?;
    eprintln!(
        "ビン {n_bins} 個：実際の値が模擬データのどれよりも小さい {n_below_all}、平均から 1σ 以内 {n_within_1s}、\
         模擬データの平均が見込みより小さい {n_proc_below_exp}、平均から {sig_min:+.2}σ〜{sig_max:+.2}σ"
    );

    let mut xo =
        String::from("k,r_km,real,mock_true_mean,mock_true_sd,mock_proc_mean,mock_proc_sd\n");
    let ks: std::collections::BTreeSet<usize> = xis.iter().map(|r| r["k"] as usize).collect();
    for k in ks {
        let real_xi = xis
            .iter()
            .find(|r| r["run"] < 0.0 && r["k"] as usize == k)
            .map_or(f64::NAN, |r| r["xi_true"]);
        let m: Vec<_> = xis
            .iter()
            .filter(|r| r["run"] >= 0.0 && r["k"] as usize == k)
            .collect();
        let r_km = m.first().map_or(f64::NAN, |r| r["r_km"]);
        let (tm, ts) = mean_sd(&m.iter().map(|r| r["xi_true"]).collect::<Vec<_>>());
        let (pm, ps) = mean_sd(&m.iter().map(|r| r["xi_proc"]).collect::<Vec<_>>());
        writeln!(
            xo,
            "{k},{r_km:.3},{real_xi:.6},{tm:.6},{ts:.6},{pm:.6},{ps:.6}"
        )?;
    }
    results::write_text(&root.join(format!("{stem}_xi_summary.csv")), &xo)?;
    Ok(format!("{stem}_summary.csv"))
}

/// 実際の店の ζ（`real`）を、帰無仮説の模擬データの ζ（`mocks`）とビンごとに比べた表を、文字列で返す。
/// 列：辺のビンの中点 [km]、DDD、実際の ζ、模擬データの平均と標準偏差、平均から何 σ、実際以上・以下の個数。
pub fn compare_mocks(
    root: &Path,
    real: &str,
    mocks: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let r = results::read_zeta(&root.join(real))?;
    let ms: Vec<Vec<_>> = mocks
        .iter()
        .map(|m| results::read_zeta(&root.join(m)))
        .collect::<Result<_, _>>()?;
    let mut out = format!(
        "{real} と模擬データ {} 個\nr1,r2,r3 [km]      DDD    ζ        模擬の平均±σ        σ      以上/以下\n",
        ms.len()
    );
    for row in &r {
        let vals: Vec<f64> = ms
            .iter()
            .filter_map(|m| m.iter().find(|x| x.k == row.k).map(|x| x.zeta))
            .collect();
        if vals.len() < 2 {
            continue;
        }
        let s = mock_summary(row.zeta, &vals);
        let (a, b, c) = row.r_km;
        writeln!(
            out,
            "{a:>4.0},{b:>4.0},{c:>4.0}  {:>8}  {:+.4}  {:+.4} ± {:.4}  {:+6.1}  {:>3}/{:<3}",
            row.ddd,
            row.zeta,
            s.mean,
            s.sd,
            (row.zeta - s.mean) / s.sd,
            s.n_ge,
            s.n_le
        )?;
    }
    Ok(out)
}

/// 同じ mock 上の偏り、および実際の店の二点・三点の新旧を一つの Markdown にまとめる。
/// 誤差は paired true−proc の標本標準偏差 / sqrt(runs)。実際の値の誤差ではない。
pub fn summarize_external_selection(
    root: &Path,
    stem: &str,
    smooth_real: &str,
    regional_real: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let bias = results::read_numeric_csv(&root.join(format!("{stem}_bias_summary.csv")))?;
    let old = results::read_zeta(&root.join(smooth_real))?;
    let new = results::read_zeta(&root.join(regional_real))?;
    let ox = results::read_numeric_csv(&root.join(smooth_real.replace(".csv", "_xi.csv")))?;
    let nx = results::read_numeric_csv(&root.join(regional_real.replace(".csv", "_xi.csv")))?;
    let mut out = String::from(
        "# External selection comparison\n\nBias is mean(true − proc) ± standard error of the paired differences across mocks. The generating baseline remains SmoothRatio R=50; regional differences also include baseline model mismatch. Real measurements have no error estimate here.\n\n",
    );
    writeln!(
        out,
        "Mock stem: `{stem}`.\n\nReal SmoothRatio: `{smooth_real}`.\n\nReal regional: `{regional_real}`.\n"
    )?;
    out.push_str(
        "| sides (km) | runs | R=50 true−proc | regional true−proc |\n|---|---:|---:|---:|\n",
    );
    let mut smaller = 0;
    for b in &bias {
        writeln!(
            out,
            "| {:.3}, {:.3}, {:.3} | {:.0} | {:+.6} ± {:.6} | {:+.6} ± {:.6} |",
            b["r1_km"],
            b["r2_km"],
            b["r3_km"],
            b["runs"],
            b["smooth_bias"],
            b["smooth_se"],
            b["regional_bias"],
            b["regional_se"]
        )?;
        smaller += usize::from(b["regional_bias"].abs() < b["smooth_bias"].abs());
    }
    let rms =
        |key: &str| (bias.iter().map(|b| b[key].powi(2)).sum::<f64>() / bias.len() as f64).sqrt();
    writeln!(
        out,
        "\nSmaller absolute mean bias: {smaller}/{} bins. RMS mean bias: R=50 {:.6}; regional {:.6}.\n",
        bias.len(),
        rms("smooth_bias"),
        rms("regional_bias")
    )?;
    out.push_str(
        "| sides (km) | real ζ R=50 | real ζ regional | regional − R=50 |\n|---|---:|---:|---:|\n",
    );
    for z in &old {
        let r = new
            .iter()
            .find(|r| r.k == z.k)
            .ok_or("新旧の三点のビンが異なる")?;
        writeln!(
            out,
            "| {:.3}, {:.3}, {:.3} | {:+.6} | {:+.6} | {:+.6} |",
            z.r_km.0,
            z.r_km.1,
            z.r_km.2,
            z.zeta,
            r.zeta,
            r.zeta - z.zeta
        )?;
    }
    writeln!(
        out,
        "\nNegative real ζ: R=50 {}/{}; regional {}/{}.\n",
        old.iter().filter(|r| r.zeta < 0.0).count(),
        old.len(),
        new.iter().filter(|r| r.zeta < 0.0).count(),
        new.len()
    )?;
    out.push_str(
        "| r (km) | real ξ R=50 | real ξ regional | regional − R=50 |\n|---|---:|---:|---:|\n",
    );
    for x in &ox {
        let r = nx
            .iter()
            .find(|r| r["k"] == x["k"])
            .ok_or("新旧の二点のビンが異なる")?;
        writeln!(
            out,
            "| {:.3} | {:+.6} | {:+.6} | {:+.6} |",
            x["r_km"],
            x["xi"],
            r["xi"],
            r["xi"] - x["xi"]
        )?;
    }
    let mocks = results::read_numeric_csv(&root.join(format!("{stem}_xi.csv")))?;
    out.push_str("\nMock ξ mean ± sample standard deviation (not standard error):\n\n| r (km) | true | R=50 proc | regional proc |\n|---|---:|---:|---:|\n");
    for x in &ox {
        let rows: Vec<_> = mocks
            .iter()
            .filter(|r| r["run"] >= 0.0 && r["k"] == x["k"])
            .collect();
        let stat = |key: &str| mock_summary(0.0, &rows.iter().map(|r| r[key]).collect::<Vec<_>>());
        let (t, s, r) = (stat("xi_true"), stat("xi_proc"), stat("xi_regional"));
        writeln!(
            out,
            "| {:.3} | {:.6} ± {:.6} | {:.6} ± {:.6} | {:.6} ± {:.6} |",
            x["r_km"], t.mean, t.sd, s.mean, s.sd, r.mean, r.sd
        )?;
    }
    let path = format!("{stem}_comparison.md");
    results::write_text(&root.join(&path), &out)?;
    Ok(out)
}

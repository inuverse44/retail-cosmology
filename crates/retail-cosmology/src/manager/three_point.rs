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
use crate::engine::estimator::{density_ratio_summary, mock_summary};
use crate::engine::random_catalog::{
    Weight, count_coarse_cells, sample, sample_conditional_children, sample_reweighted,
    sample_selected,
};
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
    /// clustered_mocks に一点密度をそろえた第三の基準を足す
    pub match_density: bool,
    /// 辺のビン：[lo, hi) km を対数で n 等分
    pub sides_km: (f64, f64, usize),
    /// 出力のファイル名につける名前（空でなければ "_名前" の形）
    pub tag: String,
    /// selection_mismatch：池を N 個に 1 個に間引いた点で三角形を数え、ビンごとの w の平均も求める
    pub tri_thin: Option<usize>,
    /// clustered_mocks：基準ごとに ζ̂ の分子と分母（組の数で割ったもの）も書く（比の偏りの切り分け、#94）
    pub counts: bool,
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
            match_density: false,
            sides_km: (2.0, 32.0, 4),
            tag: String::new(),
            tri_thin: None,
            counts: false,
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

/// 親を `patch` のまわりに幅 `margin_sd` × `s` km 広げた範囲に撒き（親の密度は `patch` の中で `n_p` 個になる割合）、
/// 子は `patch` の中に落ちたものだけを残す。窓で切り取った、境界のない Neyman–Scott の模型にあたる。
/// 返すのは点と、それぞれの点の親の番号。
fn patch_clustered_windowed(
    n_p: usize,
    m: usize,
    s: f64,
    margin_sd: f64,
    patch: Patch,
    rng: &mut SplitMix64,
) -> (Vec<LatLon>, Vec<usize>) {
    let (lat0, dlat, lon0, dlon) = patch;
    let mlat = margin_sd * s / 111.2;
    let mlon = margin_sd * s / (111.2 * lat0.to_radians().cos());
    let ext: Patch = (
        lat0 - mlat,
        dlat + 2.0 * mlat,
        lon0 - mlon,
        dlon + 2.0 * mlon,
    );
    let n_ext = (n_p as f64 * ext.1 * ext.3 / (dlat * dlon)).round() as usize;
    let all = patch_clustered(n_ext, m, s, ext, rng);
    let inside =
        |q: &LatLon| q.lat >= lat0 && q.lat < lat0 + dlat && q.lon >= lon0 && q.lon < lon0 + dlon;
    let mut pts = Vec::new();
    let mut parent = Vec::new();
    for (i, q) in all.iter().enumerate() {
        if inside(q) {
            pts.push(*q);
            parent.push(i / m);
        }
    }
    (pts, parent)
}

/// 親の番号 `parent` が同じ三点の三角形を、ビンごとに数える（`parent` は親ごとに続けて並んでいるとする）。
fn same_parent_triples_labeled(d: &[Unit], parent: &[usize], bins: &TriangleBins) -> Vec<u64> {
    let mut out = vec![0u64; bins.len()];
    let mut start = 0;
    while start < d.len() {
        let mut end = start + 1;
        while end < d.len() && parent[end] == parent[start] {
            end += 1;
        }
        let g = &d[start..end];
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
        start = end;
    }
    out
}

/// 一様な基準でも ζ̂ が見込みより低く出る原因を、toy model の境界で確かめる（#81）。
///
/// `selection_bias_toy` と同じ親・子・範囲・ビン・ランダムカタログで、模擬データを二通りに作る。
/// - leaky：親を範囲の中だけに撒き、子は範囲の外にも落ちる（`selection_bias_toy` と同じ）。
///   範囲の縁では子の密度が下がるが、ランダムカタログは範囲の中で一様なので、縁が「店の少ない場所」に見える。
/// - windowed：親を範囲の外 4σ まで撒き、範囲の中に落ちた子だけを残す。縁でも密度は下がらない。
///
/// 一回ごとに ζ̂ − 見込み（同じ点で求めるので、対にして差をとる）を求め、ビンごとに平均と平均の誤差を書く。
/// 書くもの：data/toy/zeta_edge.csv
pub fn edge_toy(root: &Path, runs: usize) -> Result<String, Box<dyn std::error::Error>> {
    use crate::engine::triple_count::{count_aaa, count_aab};
    let t0 = Instant::now();
    let c3 = |n: f64| n * (n - 1.0) * (n - 2.0) / 6.0;
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let sides_km = Bins::log(2.0, 32.0, 4);
    let bins = TriangleBins::new(sides_km.map(km_to_chord2));
    let c = sides_km.centers();
    let (n_p, m, s_km, n_r) = (350, 5, 5.0, 8_000);

    let mut rng = SplitMix64::new(13);
    let r = units(&patch_uniform(n_r, LARGE_PATCH, &mut rng));
    // RRR は一回ごとに変わらないので一度だけ数える
    let rrr = count_aaa(&r, &bins);
    let measure = |d: &[Unit]| -> Vec<f64> {
        TripleCounts {
            n_d: d.len(),
            n_r: r.len(),
            ddd: count_aaa(d, &bins),
            ddr: count_aab(d, &r, &bins),
            drr: count_aab(&r, d, &bins),
            rrr: rrr.clone(),
        }
        .szapudi_szalay()
    };
    let expected = |same: &[u64], n_d: usize, b: usize| {
        same[b] as f64 / (rrr[b] as f64 * c3(n_d as f64) / c3(n_r as f64))
    };

    // ビンごとに、一回ごとの (見込み, ζ̂ − 見込み) を、leaky と windowed で集める
    let mut leaky: Vec<Vec<(f64, f64)>> = vec![Vec::new(); bins.len()];
    let mut windowed: Vec<Vec<(f64, f64)>> = vec![Vec::new(); bins.len()];
    let mut n_win = 0usize;
    for run in 0..runs {
        let d = units(&patch_clustered(n_p, m, s_km, LARGE_PATCH, &mut rng));
        let z = measure(&d);
        let same = same_parent_triples(&d, m, &bins);
        let (dw, parent) = patch_clustered_windowed(n_p, m, s_km, 4.0, LARGE_PATCH, &mut rng);
        let dw = units(&dw);
        n_win += dw.len();
        let zw = measure(&dw);
        let same_w = same_parent_triples_labeled(&dw, &parent, &bins);
        for b in bins.used() {
            if rrr[b] == 0 {
                continue;
            }
            let e = expected(&same, d.len(), b);
            leaky[b].push((e, z[b] - e));
            let ew = expected(&same_w, dw.len(), b);
            windowed[b].push((ew, zw[b] - ew));
        }
        eprintln!("{} 回目（{:.1} 秒）", run + 1, t0.elapsed().as_secs_f64());
    }
    eprintln!(
        "windowed の点の数の平均：{:.0}（leaky は {}）",
        n_win as f64 / runs as f64,
        n_p * m
    );

    let mean_err = |v: &[f64]| {
        let n = v.len() as f64;
        let mu = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        (mu, sd / n.sqrt())
    };
    let mut out =
        String::from("model,k1,k2,k3,r1_km,r2_km,r3_km,runs,expected_mean,diff_mean,diff_err\n");
    // ノートの表（ビンごとに、windowed の ζ_NS と、二通りの ζ̂_U − ζ_NS。leaky = NS_in、windowed = NS_out）
    let mut tex = format!(
        "% cargo run --release --bin zeta_edge_toy -- {runs} が書いた表\n\\begin{{tabular}}{{rrrrrr}}\n\\toprule\n\
         $r_1$ & $r_2$ & $r_3$ & $\\zeta_{{\\mathrm{{NS}}}}$ & \\multicolumn{{2}}{{c}}{{$\\hat\\zeta_U - \\zeta_{{\\mathrm{{NS}}}}$}} \\\\\n\
         & & & & $\\mathrm{{NS}}_{{\\mathrm{{in}}}}$ & $\\mathrm{{NS}}_{{\\mathrm{{out}}}}$ \\\\\n\\midrule\n"
    );
    for b in bins.used() {
        if leaky[b].len() < 2 || windowed[b].len() < 2 {
            continue;
        }
        let (x, y, z) = bins.unflatten(b);
        let (em, _) = mean_err(&windowed[b].iter().map(|t| t.0).collect::<Vec<_>>());
        let (lm, le) = mean_err(&leaky[b].iter().map(|t| t.1).collect::<Vec<_>>());
        let (wm, we) = mean_err(&windowed[b].iter().map(|t| t.1).collect::<Vec<_>>());
        writeln!(
            tex,
            "{:.0} & {:.0} & {:.0} & ${em:.3}$ & ${lm:+.3} \\pm {le:.3}$ & ${wm:+.3} \\pm {we:.3}$ \\\\",
            c[x], c[y], c[z]
        )?;
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/zeta_edge.tex"), &tex)?;
    for (name, per_bin) in [("leaky", &leaky), ("windowed", &windowed)] {
        let (mut neg, mut beyond2, mut total) = (0, 0, 0);
        for b in bins.used() {
            let v = &per_bin[b];
            if v.len() < 2 {
                continue;
            }
            let (x, y, z) = bins.unflatten(b);
            let (em, _) = mean_err(&v.iter().map(|t| t.0).collect::<Vec<_>>());
            let (dm, de) = mean_err(&v.iter().map(|t| t.1).collect::<Vec<_>>());
            total += 1;
            neg += (dm < 0.0) as usize;
            beyond2 += (dm.abs() > 2.0 * de) as usize;
            writeln!(
                out,
                "{name},{x},{y},{z},{:.3},{:.3},{:.3},{},{em:.6},{dm:.6},{de:.6}",
                c[x],
                c[y],
                c[z],
                v.len()
            )?;
        }
        // 本文に書くビンの数は手で数えず、ここで数える（振り返り 2026-11-14 の T-03）
        eprintln!(
            "{name}：ビン {total} 個、ζ̂ − 見込みが負 {neg}、平均の誤差の 2 倍を超える {beyond2}"
        );
    }
    let path = "data/toy/zeta_edge.csv".to_string();
    results::write_text(&root.join(&path), &out)?;
    Ok(path)
}

/// 二点の固まりを実際の店にそろえ、ζ がわかっている模擬データ（Neyman–Scott の模型）を作り、
/// 選択関数の偏りを実際の地形の上で測る（#81）。
///
/// 親は実際の店から作った選択関数と人口に比例して撒き、その親のまわりに標準偏差 `sigma_km` で `m` 個ずつ子を撒く。
/// 子の数の合計は実際の店の数に合わせる（親の数は店の数 ÷ `m`、端数は切り上げて余りを捨てる）。
/// 同じ模擬データを true・proc で測り、指定した基準も追加する。
/// - true：従来の列名。親を作った重み（実際の店の選択関数 × 人口）を基準にする。子の一点密度と一致する保証はない。
/// - proc：模擬データそのものから選択関数を作り直したランダムカタログ。実際の店と同じ手順。
/// - regional（指定時）：同じ模擬データの行政区別店舗数 / 人口で作ったランダムカタログ。
/// - match（指定時）：親ごとに最初の子を一個だけ選び直す独立な基準。
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
    let wall0 = std::time::SystemTime::now();
    if m == 0 || runs == 0 || p.n_random < 3 || !sigma_km.is_finite() || sigma_km <= 0.0 {
        return Err("children, runs, sigma は正、n-random は3以上が必要".into());
    }
    if p.match_density && p.tag.is_empty() {
        return Err("--match on では新規の --tag が必要".into());
    }
    let mut timing = String::from("stage,run,instant_s,wall_s\n");
    let mut completed = 0usize;
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
    let mut xi_out = String::from("run,k,r_km,xi_true,xi_proc");
    if p.regional.is_some() {
        xi_out.push_str(",xi_regional");
    }
    if p.match_density {
        xi_out.push_str(",xi_match");
    }
    xi_out.push('\n');
    for (k, v) in xi_of(&real, &r_true).iter().enumerate() {
        write!(xi_out, "-1,{k},{:.3},{v:.6},", c[k])?;
        if let Some(xi) = &real_xi_regional {
            write!(xi_out, ",{:.6}", xi[k])?;
        }
        if p.match_density {
            write!(xi_out, ",")?;
        }
        writeln!(xi_out)?;
    }
    let mut z_out = String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected");
    if p.regional.is_some() {
        z_out.push_str(",zeta_regional");
    }
    if p.match_density {
        z_out.push_str(",zeta_match,expected_match");
    }
    if p.counts {
        z_out.push_str(",num_true,rrr_true,num_proc,rrr_proc");
        if p.match_density {
            z_out.push_str(",num_match,rrr_match");
        }
    }
    z_out.push('\n');
    let n_p = pts.len().div_ceil(m);
    let mut rng_m = SplitMix64::new(p.seed ^ 0xC1A5_7E00_0000_0000);
    // 点の池：基準の重み（人口 × 実際の店の選択関数）で撒いた点。親も子もこの池から選ぶので、
    // 子は人の住む格子だけに置かれるが、親ごとの正規化で一点密度が池の密度とずれる可能性はある。
    let pool = units(&draw(&sr_real, 20 * p.n_random, &mut rng_m));
    let cut = km_to_chord2(3.0 * sigma_km);
    let two_s2 = 2.0 * km_to_chord2(sigma_km);
    let g_pool = crate::util::spatial::UnitGrid::build(&pool, cut.sqrt());
    eprintln!(
        "点の池 {} 点（Instant {:.1} 秒、壁時計 {:.1} 秒）",
        pool.len(),
        t0.elapsed().as_secs_f64(),
        wall0.elapsed()?.as_secs_f64()
    );
    writeln!(
        timing,
        "pool,-1,{:.3},{:.3}",
        t0.elapsed().as_secs_f64(),
        wall0.elapsed()?.as_secs_f64()
    )?;
    let pool_cells = if p.match_density {
        count_coarse_cells(&pool, &grid.geometry)
    } else {
        Default::default()
    };
    let true_cells = if p.match_density {
        count_coarse_cells(&r_true, &grid.geometry)
    } else {
        Default::default()
    };
    let mut match_a = std::collections::BTreeMap::<u64, u64>::new();
    let mut match_b = std::collections::BTreeMap::<u64, u64>::new();
    for run in 0..runs {
        let step_wall = std::time::SystemTime::now();
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
        let r_match = if p.match_density {
            let mut rng_match = SplitMix64::new(p.seed ^ 0x4D44_454E_5300_0000 ^ run as u64);
            let r = sample_conditional_children(
                &pool,
                &g_pool,
                sigma_km,
                m,
                p.n_random,
                &mut rng_match,
            )?;
            let mid = r.len() / 2;
            for (key, count) in count_coarse_cells(&r[..mid], &grid.geometry) {
                *match_a.entry(key).or_default() += count;
            }
            for (key, count) in count_coarse_cells(&r[mid..], &grid.geometry) {
                *match_b.entry(key).or_default() += count;
            }
            Some(r)
        } else {
            None
        };
        let xm = r_match.as_ref().map(|r| xi_of(&d, r));
        for (k, (a, b)) in xi_of(&d, &r_true)
            .iter()
            .zip(xi_of(&d, &r_proc))
            .enumerate()
        {
            write!(xi_out, "{run},{k},{:.3},{a:.6},{b:.6}", c[k])?;
            if let Some(xr) = &xr {
                write!(xi_out, ",{:.6}", xr[k])?;
            }
            if let Some(xm) = &xm {
                write!(xi_out, ",{:.6}", xm[k])?;
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
        let t_match = r_match
            .as_ref()
            .map(|r| TripleCounts::compute(&d, r, &bins));
        let zm = t_match.as_ref().map(TripleCounts::szapudi_szalay);
        let same = same_parent_triples(&d, m, &bins);
        let scale = c3(d.len() as f64) / c3(r_true.len() as f64);
        // ζ̂ の分子と分母（--counts on のときだけ）。match の基準は空のビンがありうる
        let parts = p.counts.then(|| {
            (
                t_true.szapudi_szalay_parts(),
                t_proc.szapudi_szalay_parts(),
                t_match.as_ref().map(TripleCounts::szapudi_szalay_parts),
            )
        });
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
            if let (Some(tm), Some(zm)) = (&t_match, &zm) {
                let em = if tm.rrr[b] > 0 {
                    same[b] as f64 / (tm.rrr[b] as f64 * scale)
                } else {
                    f64::NAN
                };
                write!(z_out, ",{:.6},{em:.6}", zm[b])?;
            }
            if let Some((pt, pp, pm)) = &parts {
                write!(
                    z_out,
                    ",{:.9e},{:.9e},{:.9e},{:.9e}",
                    pt[b].0, pt[b].1, pp[b].0, pp[b].1
                )?;
                if let Some(pm) = pm {
                    write!(z_out, ",{:.9e},{:.9e}", pm[b].0, pm[b].1)?;
                }
            }
            writeln!(z_out)?;
        }
        completed = run + 1;
        let wall = wall0.elapsed()?.as_secs_f64();
        eprintln!(
            "{} 回目（Instant {:.1} 秒、壁時計 {wall:.1} 秒）",
            completed,
            t0.elapsed().as_secs_f64()
        );
        writeln!(
            timing,
            "run,{run},{:.3},{wall:.3}",
            t0.elapsed().as_secs_f64()
        )?;
        if p.match_density
            && completed < runs
            && wall + 1.2 * step_wall.elapsed()?.as_secs_f64() > 3600.0
        {
            eprintln!("壁時計で1時間を超える見込みのため {completed}/{runs} 回で保存して終了");
            break;
        }
    }
    let stem = format!(
        "data/zeta/clustered_{prefix}{}_{}_m{m}_s{sigma_km}{}",
        p.date,
        p.chain.to_lowercase(),
        p.tag
    );
    if p.match_density {
        let mut cells = String::from(
            "coarse_id,pool_count,true_count,match_a,match_b,g_match_over_pool,g_match_over_true\n",
        );
        let mut pc = Vec::new();
        let mut ac = Vec::new();
        let mut bc = Vec::new();
        let nm = match_a.values().sum::<u64>() + match_b.values().sum::<u64>();
        for (&key, &np) in &pool_cells {
            let nt = true_cells.get(&key).copied().unwrap_or(0);
            let a = match_a.get(&key).copied().unwrap_or(0);
            let b = match_b.get(&key).copied().unwrap_or(0);
            let gp = (a + b) as f64 / nm as f64 / (np as f64 / pool.len() as f64);
            let gt = if nt > 0 {
                (a + b) as f64 / nm as f64 / (nt as f64 / r_true.len() as f64)
            } else {
                f64::NAN
            };
            writeln!(cells, "{key},{np},{nt},{a},{b},{gp:.9},{gt:.9}")?;
            pc.push(np);
            ac.push(a);
            bc.push(b);
        }
        results::write_text(&root.join(format!("{stem}_density_cells.csv")), &cells)?;
        let ds = density_ratio_summary(&pc, &ac, &bc, 20);
        let density = format!(
            "min_pool_count,pool_points,match_points,cells,mass_fraction,g_mean,g_variance,split_cross_mse,g_q10,g_q50,g_q90\n20,{},{nm},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9}\n",
            pool.len(),
            ds.cells,
            ds.mass_fraction,
            ds.mean,
            ds.variance,
            ds.cross_mse,
            ds.q10,
            ds.q50,
            ds.q90
        );
        results::write_text(&root.join(format!("{stem}_density_summary.csv")), &density)?;
        writeln!(
            timing,
            "completed,{completed},{:.3},{:.3}",
            t0.elapsed().as_secs_f64(),
            wall0.elapsed()?.as_secs_f64()
        )?;
        results::write_text(&root.join(format!("{stem}_timing.csv")), &timing)?;
    }
    results::write_text(&root.join(format!("{stem}_xi.csv")), &xi_out)?;
    let path = format!("{stem}.csv");
    results::write_text(&root.join(&path), &z_out)?;
    Ok(path)
}

/// ζ = 0 の模擬データ（ガウス場で密度を揺らす模型）を、日本の実際の地形の上で作って測る（#81）。
///
/// 店の密度を「基準 × max(0, 1 + δ)」とする。δ は、点の池から選んだ `n_centers` 個の中心に、正負の符号を半々に
/// つけた幅 `sigma_km` のガウス関数を足し、各点で sqrt(Σ K²) で割って分散をそろえ、`amp` を掛けたもの。
/// 符号が対称なので切る前の δ の三次のモーメントは 0。切り捨てと固定点数の抽出後は true の平均も測って確かめる。
/// 二点相関関数はおよそ `amp`² × （幅 √2 σ のガウス関数）。
/// 店は、基準の重み（人口 × 実際の店の選択関数）で撒いた点の池から、重み max(0, 1 + δ) で店の数だけ選ぶ（同じ点は選ばない）。
/// 測り方と書くものの形は [`clustered_mocks`] と同じ（expected の列は 0）。
/// 書くもの：data/zeta/field_<前置き><日付>_<チェーン>_a<amp>_s<σ>.csv と _xi.csv
#[derive(Clone, Debug)]
pub struct FieldModel {
    pub amp: f64,
    pub sigma_km: f64,
    /// 独立な符号を持つ二つ目の成分 (標準偏差, 幅 km)。
    pub second: Option<(f64, f64)>,
    pub n_centers: usize,
    pub runs: usize,
    /// 明示すると run ごとの種で分割する。None かつ second=None なら旧来の連続した乱数列。
    pub start: Option<usize>,
    /// 店を半分ずつ A・B に分け、B を正しい基準・B 自身の選択関数・A の選択関数の三通りで測る列を足す（#94）。
    /// 分け方は run ごとの別の乱数列で決めるので、もとの列は変わらない。
    pub split: bool,
    /// ζ の分子と分母（組の数で割ったもの）を true・proc それぞれ書く列を足す（#94）。もとの列は変わらない。
    pub counts: bool,
}

/// [`FieldModel`] の密度場を生成し、true/proc の ξ、ζ と切り捨て割合を書く。
pub fn field_mocks(
    root: &Path,
    p: &Params,
    model: &FieldModel,
) -> Result<String, Box<dyn std::error::Error>> {
    let FieldModel {
        amp,
        sigma_km,
        n_centers,
        runs,
        start,
        ..
    } = *model;
    if !amp.is_finite()
        || amp < 0.0
        || !sigma_km.is_finite()
        || sigma_km <= 0.0
        || n_centers == 0
        || runs == 0
        || model
            .second
            .is_some_and(|(a, s)| !a.is_finite() || a < 0.0 || !s.is_finite() || s <= 0.0)
    {
        return Err("振幅は有限で非負、幅・centers・runs は正であること".into());
    }
    let second_tag = model
        .second
        .map_or(String::new(), |(a, s)| format!("_a2{a}_s2{s}"));
    let stem = format!(
        "data/zeta/field_{}{}_{}_a{amp}_s{sigma_km}{second_tag}{}",
        if p.country == "jp" { "jp_" } else { "us_" },
        p.date,
        p.chain.to_lowercase(),
        p.tag
    );
    for suffix in [".csv", "_xi.csv", "_diagnostics.csv"] {
        if root.join(format!("{stem}{suffix}")).exists() {
            return Err(format!("既存の出力を上書きしない：{stem}{suffix}").into());
        }
    }
    eprintln!(
        "field model {model:?}; seed={}, n_ref={}, n_random={}, output={stem}",
        p.seed, p.n_ref, p.n_random
    );
    let t0 = Instant::now();
    let r_km = p.footprint.ok_or("--footprint R が要る")?;
    let (grid, _) = grid_of(root, &p.country)?;
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
    let draw_sel = |sr: &SmoothRatio, n: usize, rng: &mut SplitMix64| {
        sample_selected(
            &grid,
            n,
            Weight::Population,
            |q| sr.eval(&Unit::from(q)),
            2 * n,
            rng,
        )
    };
    let draw = |sr: &SmoothRatio, n: usize, rng: &mut SplitMix64| -> Vec<LatLon> {
        draw_sel(sr, n, rng).points
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

    let split = model.split;
    let mut xi_out = String::from(if split {
        "run,k,r_km,xi_true,xi_proc,xi_half_true,xi_half_proc,xi_half_split\n"
    } else {
        "run,k,r_km,xi_true,xi_proc\n"
    });
    for (k, v) in xi_of(&real, &r_true).iter().enumerate() {
        writeln!(
            xi_out,
            "-1,{k},{:.3},{v:.6},{}",
            c[k],
            if split { ",,," } else { "" }
        )?;
    }
    let mut z_out = String::from(if split {
        "run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected,zeta_half_true,zeta_half_proc,zeta_half_split"
    } else {
        "run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected"
    });
    if model.counts {
        z_out.push_str(",num_true,rrr_true,num_proc,rrr_proc,num_true_icf,num_proc_icf");
    }
    z_out.push('\n');
    let mut rng_legacy = SplitMix64::new(p.seed ^ 0xF1E1_D000_0000_0000);
    let pool = units(&draw(&sr_real, 20 * p.n_random, &mut rng_legacy));
    eprintln!(
        "点の池 {} 点（{:.1} 秒）",
        pool.len(),
        t0.elapsed().as_secs_f64()
    );
    let mut diagnostics = String::from(if model.counts {
        "run,clipped_fraction,elapsed_seconds,alpha,beta,proc_cap_clipped,proc_candidates\n"
    } else {
        "run,clipped_fraction,elapsed_seconds\n"
    });
    let first_run = start.unwrap_or(0);
    let legacy_stream = model.second.is_none() && start.is_none();
    for run in first_run
        ..first_run
            .checked_add(runs)
            .ok_or("run の範囲が大きすぎる")?
    {
        let mut run_seed = SplitMix64::new(p.seed ^ 0xF1E1_D100_0000_0000 ^ run as u64);
        let mut rng_run = SplitMix64::new(run_seed.next_u64());
        // 単一幅の従来のコマンドは、池→中心と符号→店→proc基準→次runと同じ列を消費する。
        let rng_m = if legacy_stream {
            &mut rng_legacy
        } else {
            &mut rng_run
        };
        let mut delta = vec![0.0; pool.len()];
        for (amplitude, width) in std::iter::once((amp, sigma_km)).chain(model.second) {
            if amplitude == 0.0 && !legacy_stream {
                continue;
            }
            let cut = km_to_chord2(3.0 * width);
            let two_s2 = 2.0 * km_to_chord2(width);
            // 中心と符号を選び、池の各点で δ を求める
            let centers: Vec<(Unit, f64)> = (0..n_centers)
                .map(|_| {
                    let q = pool[(rng_m.next_f64() * pool.len() as f64) as usize];
                    (q, if rng_m.next_f64() < 0.5 { 1.0 } else { -1.0 })
                })
                .collect();
            let cu: Vec<Unit> = centers.iter().map(|x| x.0).collect();
            let g = crate::util::spatial::UnitGrid::build(&cu, cut.sqrt());
            for (i, q) in pool.iter().enumerate() {
                let (mut s, mut s2) = (0.0, 0.0);
                g.for_each_near(q, cut.sqrt(), |j| {
                    let c2 = chord2(q, &cu[j]);
                    if c2 < cut {
                        let k = (-c2 / two_s2).exp();
                        s += centers[j].1 * k;
                        s2 += k * k;
                    }
                });
                if s2 > 0.0 {
                    delta[i] += amplitude * s / s2.sqrt();
                }
            }
        }
        let clipped = delta.iter().filter(|d| 1.0 + **d < 0.0).count();
        let w: Vec<f64> = delta.iter().map(|d| (1.0 + d).max(0.0)).collect();
        if w.iter().filter(|w| **w > 0.0).count() < pts.len() {
            return Err("正の重みを持つ池の点が店の数より少ない".into());
        }
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
        let sel_proc = draw_sel(&sr_mock, p.n_random, rng_m);
        let r_proc = units(&sel_proc.points);

        // 全体で割る規格化の量（#94、--counts on のときだけ）。
        // 店：池から重み w で店の数だけ選ぶので、店の割合の密度は W(1+δ)/(1+α)、1+α = 池での w の平均。
        // ランダム（店と同じ手順）：割合の密度は W(1+δ_R)/(1+β)。1+δ_R = (1+α) S_mock/S_real とおくと
        // 1+β = (1+α)⟨S_mock/S_real⟩_W。⟨⟩_W は池（W に比例して撒いた点）の 8 個に 1 個で平均する。
        let norm = if model.counts {
            let alpha = w.iter().sum::<f64>() / w.len() as f64 - 1.0;
            let (mut s, mut m) = (0.0, 0usize);
            for q in pool.iter().step_by(8) {
                let den = sr_real.eval(q);
                if den > 0.0 {
                    s += sr_mock.eval(q) / den;
                    m += 1;
                }
            }
            let beta = (1.0 + alpha) * s / m as f64 - 1.0;
            Some((alpha, beta))
        } else {
            None
        };

        // 半分に分けた測り方（#94）。もとの乱数列を使わないよう、run ごとの別の列で分ける。
        let halves = if split {
            let mut rng_split = SplitMix64::new(p.seed ^ 0x5A17_0000_0000_0000 ^ run as u64);
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for q in &d {
                if rng_split.next_f64() < 0.5 {
                    a.push(*q);
                } else {
                    b.push(*q);
                }
            }
            let r_self = units(&draw(
                &SmoothRatio::new(&b, &refs, r_km),
                p.n_random,
                &mut rng_split,
            ));
            let r_cross = units(&draw(
                &SmoothRatio::new(&a, &refs, r_km),
                p.n_random,
                &mut rng_split,
            ));
            let xis = [xi_of(&b, &r_true), xi_of(&b, &r_self), xi_of(&b, &r_cross)];
            let ts = [
                TripleCounts::compute(&b, &r_true, &bins),
                TripleCounts::compute(&b, &r_self, &bins),
                TripleCounts::compute(&b, &r_cross, &bins),
            ];
            Some((xis, ts))
        } else {
            None
        };

        for (k, (a, b)) in xi_of(&d, &r_true)
            .iter()
            .zip(xi_of(&d, &r_proc))
            .enumerate()
        {
            write!(xi_out, "{run},{k},{:.3},{a:.6},{b:.6}", c[k])?;
            if let Some((xis, _)) = &halves {
                write!(
                    xi_out,
                    ",{:.6},{:.6},{:.6}",
                    xis[0][k], xis[1][k], xis[2][k]
                )?;
            }
            writeln!(xi_out)?;
        }
        let t_true = TripleCounts::compute(&d, &r_true, &bins);
        let t_proc = TripleCounts::compute(&d, &r_proc, &bins);
        let (zt, zp) = (t_true.szapudi_szalay(), t_proc.szapudi_szalay());
        let zh = halves
            .as_ref()
            .map(|(_, ts)| ts.each_ref().map(TripleCounts::szapudi_szalay));
        let parts = model
            .counts
            .then(|| (t_true.szapudi_szalay_parts(), t_proc.szapudi_szalay_parts()));
        // 規格化を割り戻した分子（店は 1+α、店と同じ手順のランダムは 1+β を掛ける。正しい基準のランダムは 1）
        let icf = norm.map(|(a, bt)| {
            (
                t_true.szapudi_szalay_numerator_scaled(1.0 + a, 1.0),
                t_proc.szapudi_szalay_numerator_scaled(1.0 + a, 1.0 + bt),
            )
        });
        for b in bins.used() {
            if t_true.rrr[b] == 0 || t_proc.rrr[b] == 0 {
                continue;
            }
            let (x, y, z) = bins.unflatten(b);
            write!(
                z_out,
                "{run},{x},{y},{z},{:.3},{:.3},{:.3},{:.6},{:.6},0",
                c[x], c[y], c[z], zt[b], zp[b]
            )?;
            if let (Some(zh), Some((_, ts))) = (&zh, &halves) {
                // 半分の点では空のビンがありうる。そのときは NaN を書く
                for (j, t) in ts.iter().enumerate() {
                    let v = if t.rrr[b] == 0 { f64::NAN } else { zh[j][b] };
                    write!(z_out, ",{v:.6}")?;
                }
            }
            if let Some((pt, pp)) = &parts {
                write!(
                    z_out,
                    ",{:.9e},{:.9e},{:.9e},{:.9e}",
                    pt[b].0, pt[b].1, pp[b].0, pp[b].1
                )?;
            }
            if let Some((it, ip)) = &icf {
                write!(z_out, ",{:.9e},{:.9e}", it[b], ip[b])?;
            }
            writeln!(z_out)?;
        }
        eprintln!(
            "{} 回目（1 + δ < 0 で切った点 {:.1}%、{:.1} 秒）",
            run + 1,
            100.0 * clipped as f64 / pool.len() as f64,
            t0.elapsed().as_secs_f64()
        );
        write!(
            diagnostics,
            "{run},{:.6},{:.3}",
            clipped as f64 / pool.len() as f64,
            t0.elapsed().as_secs_f64()
        )?;
        if let Some((a, bt)) = norm {
            write!(
                diagnostics,
                ",{a:.6},{bt:.6},{},{}",
                sel_proc.clipped, sel_proc.candidates
            )?;
        }
        writeln!(diagnostics)?;
    }
    results::write_text(&root.join(format!("{stem}_diagnostics.csv")), &diagnostics)?;
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
        "k1,k2,k3,r1_km,r2_km,r3_km,runs,true_mean,true_sd,proc_mean,proc_sd,expected_mean,real,real_sigma_proc,n_le_real,true_sem,proc_sem\n",
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
            "{},{},{},{a:.3},{b:.3},{c:.3},{},{tm:.6},{ts:.6},{pm:.6},{ps:.6},{em:.6},{:.6},{:.2},{n_le},{:.6},{:.6}",
            z.k.0,
            z.k.1,
            z.k.2,
            v.len(),
            z.zeta,
            (z.zeta - pm) / ps,
            ts / (v.len() as f64).sqrt(),
            ps / (v.len() as f64).sqrt()
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

/// 分割した field の独立な run を結合し、proc 同士で全体検定と一律シフトの感度を調べる。
/// 入力 run の重複・欠損ビン・ξ の実測値の不一致はエラーにする。
pub fn analyze_field(
    root: &Path,
    real_path: &str,
    output: &str,
    stems: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::engine::estimator::{chi2, chi2_sf, ensemble, extreme_rank_test, gls_mean};
    use std::collections::{BTreeMap, BTreeSet};
    for suffix in [
        ".csv",
        "_xi.csv",
        "_summary.csv",
        "_xi_summary.csv",
        "_cov.csv",
        "_tests.csv",
        "_sensitivity.csv",
        "_true_summary.csv",
        "_diagnostics.csv",
        "_report.md",
    ] {
        if root.join(format!("{output}{suffix}")).exists() {
            return Err(format!("既存の出力を上書きしない：{output}{suffix}").into());
        }
    }
    let real = results::read_zeta(&root.join(real_path))?;
    if real.is_empty() {
        return Err("実測のビンがない".into());
    }
    type TriangleKey = (usize, usize, usize);
    type ZetaValues = (f64, f64, f64);
    let mut mocks: BTreeMap<usize, BTreeMap<TriangleKey, ZetaValues>> = BTreeMap::new();
    let mut xi: BTreeMap<(i64, usize), (f64, f64, f64)> = BTreeMap::new();
    let mut source_runs = BTreeSet::new();
    let mut clipping = BTreeMap::new();
    for stem in stems {
        let rows = results::read_numeric_csv(&root.join(format!("{stem}.csv")))?;
        let mut local_runs = BTreeSet::new();
        for r in rows {
            let run_value = r["run"];
            if !run_value.is_finite() || run_value < 0.0 || run_value.fract() != 0.0 {
                return Err("run は非負整数".into());
            }
            let run = run_value as usize;
            let k = (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize);
            if mocks
                .entry(run)
                .or_default()
                .insert(k, (r["zeta_true"], r["zeta_proc"], r["expected"]))
                .is_some()
            {
                return Err(format!("重複した run/bin：{stem} {run} {k:?}").into());
            }
            local_runs.insert(run);
        }
        if !source_runs.is_disjoint(&local_runs) {
            return Err("入力の run 範囲が重複".into());
        }
        source_runs.extend(local_runs);
        let diagnostic_path = root.join(format!("{stem}_diagnostics.csv"));
        if diagnostic_path.exists() {
            for r in results::read_numeric_csv(&diagnostic_path)? {
                let run = r["run"] as usize;
                let fraction = r["clipped_fraction"];
                if !fraction.is_finite()
                    || !(0.0..=1.0).contains(&fraction)
                    || clipping.insert(run, fraction).is_some()
                {
                    return Err("切り捨て割合の欠損・重複・範囲外".into());
                }
            }
        }
        for r in results::read_numeric_csv(&root.join(format!("{stem}_xi.csv")))? {
            let key = (r["run"] as i64, r["k"] as usize);
            let value = (r["r_km"], r["xi_true"], r["xi_proc"]);
            if let Some(previous) = xi.insert(key, value)
                && (key.0 >= 0 || previous.0 != value.0 || (previous.1 - value.1).abs() > 1e-6)
            {
                return Err("ξ の run 重複、または実測の基準が異なる".into());
            }
        }
    }
    let n = mocks.len();
    let p = real.len();
    let expected_keys: BTreeSet<_> = real.iter().map(|r| r.k).collect();
    if expected_keys.len() != p
        || mocks
            .values()
            .any(|v| v.keys().copied().collect::<BTreeSet<_>>() != expected_keys)
    {
        return Err("実測と模擬のビンが一致しない".into());
    }
    let xi_keys: BTreeSet<_> = xi
        .keys()
        .filter(|(run, _)| *run == -1)
        .map(|(_, k)| *k)
        .collect();
    if xi_keys.is_empty()
        || source_runs.iter().any(|run| {
            xi.keys()
                .filter(|(r, _)| *r == *run as i64)
                .map(|(_, k)| *k)
                .collect::<BTreeSet<_>>()
                != xi_keys
        })
        || xi
            .keys()
            .any(|(run, _)| *run >= 0 && !source_runs.contains(&(*run as usize)))
    {
        return Err("ξ に run またはビンの欠損・不一致がある".into());
    }
    let vectors: Vec<Vec<f64>> = mocks
        .values()
        .map(|v| real.iter().map(|r| v[&r.k].1).collect())
        .collect();
    if !clipping.is_empty() && clipping.keys().copied().collect::<BTreeSet<_>>() != source_runs {
        return Err("切り捨て割合の run が模擬データと一致しない".into());
    }
    let stats = ensemble(&vectors).ok_or("模擬に非有限値がある")?;
    let true_vectors: Vec<Vec<f64>> = mocks
        .values()
        .map(|v| real.iter().map(|r| v[&r.k].0).collect())
        .collect();
    let true_stats = ensemble(&true_vectors).ok_or("true の模擬に非有限値がある")?;
    let delta: Vec<f64> = real
        .iter()
        .zip(&stats.value)
        .map(|(r, m)| r.zeta - m)
        .collect();
    let mut out = String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected\n");
    for (run, v) in &mocks {
        for r in &real {
            let (a, b, c) = v[&r.k];
            writeln!(
                out,
                "{run},{},{},{},{:.3},{:.3},{:.3},{a:.6},{b:.6},{c:.6}",
                r.k.0, r.k.1, r.k.2, r.r_km.0, r.r_km.1, r.r_km.2
            )?;
        }
    }
    let mut xo = String::from("run,k,r_km,xi_true,xi_proc\n");
    for ((run, k), (r, a, b)) in &xi {
        writeln!(xo, "{run},{k},{r:.3},{a:.6},{b:.6}")?;
    }
    results::write_text(&root.join(format!("{output}.csv")), &out)?;
    results::write_text(&root.join(format!("{output}_xi.csv")), &xo)?;
    summarize_clustered(root, output, real_path)?;
    let mut audit_report = String::new();
    if !clipping.is_empty() {
        let cmin = clipping.values().copied().fold(f64::INFINITY, f64::min);
        let cmax = clipping.values().copied().fold(f64::NEG_INFINITY, f64::max);
        let cm = clipping.values().sum::<f64>() / n as f64;
        let mut co = String::from("run,clipped_fraction\n");
        for (run, fraction) in &clipping {
            writeln!(co, "{run},{fraction:.6}")?;
        }
        results::write_text(&root.join(format!("{output}_diagnostics.csv")), &co)?;
        writeln!(
            audit_report,
            "Clipped pool fraction across catalogues: {:.4}% to {:.4}%; mean {:.4}%. Individual values: {output}_diagnostics.csv.\n",
            100.0 * cmin,
            100.0 * cmax,
            100.0 * cm
        )?;
    }
    audit_report.push_str("| sides km | true mean | true SD | true mean SE | true mean / SE | proc mean | proc mean SE |\n|---|---:|---:|---:|---:|---:|---:|\n");
    let mut true_out = String::from(
        "k1,k2,k3,runs,true_mean,true_sd,true_sem,true_mean_over_sem,proc_mean,proc_sem\n",
    );
    for (i, r) in real.iter().enumerate() {
        let tm = true_stats.value[i];
        let ts = true_stats.sigma[i];
        let te = ts / (n as f64).sqrt();
        let pm = stats.value[i];
        let pe = stats.sigma[i] / (n as f64).sqrt();
        writeln!(
            true_out,
            "{},{},{},{n},{tm:.9},{ts:.9},{te:.9},{:.6},{pm:.9},{pe:.9}",
            r.k.0,
            r.k.1,
            r.k.2,
            tm / te
        )?;
        writeln!(
            audit_report,
            "| {:.0},{:.0},{:.0} | {tm:.6} | {ts:.6} | {te:.6} | {:+.3} | {pm:.6} | {pe:.6} |",
            r.r_km.0,
            r.r_km.1,
            r.r_km.2,
            tm / te
        )?;
    }
    results::write_text(&root.join(format!("{output}_true_summary.csv")), &true_out)?;
    let mut xi_report = String::from(
        "| r km | real xi | true mean | proc mean | proc SD | (real-mean)/SD |\n|---:|---:|---:|---:|---:|---:|\n",
    );
    let mut xi_within = 0;
    for k in &xi_keys {
        let r = xi[&(-1, *k)].0;
        let actual = xi[&(-1, *k)].1;
        let vals: Vec<_> = source_runs
            .iter()
            .map(|run| xi[&(*run as i64, *k)].2)
            .collect();
        let s = mock_summary(actual, &vals);
        let tm = source_runs
            .iter()
            .map(|run| xi[&(*run as i64, *k)].1)
            .sum::<f64>()
            / n as f64;
        let z = (actual - s.mean) / s.sd;
        xi_within += usize::from(z.abs() <= 1.0);
        writeln!(
            xi_report,
            "| {r:.0} | {actual:.6} | {tm:.6} | {:.6} | {:.6} | {z:+.3} |",
            s.mean, s.sd
        )?;
    }
    writeln!(
        xi_report,
        "\nXi within one catalogue SD: {xi_within}/{} (counted by Rust). This is not agreement within the error on the ensemble mean.\n",
        xi_keys.len()
    )?;
    if n <= p + 2 {
        let path = format!("{output}_report.md");
        results::write_text(
            &root.join(&path),
            &format!(
                "# Small field pilot\n\nN={n}, bins={p}. Hartlap requires N > p+2; global covariance tests skipped.\n\n{xi_report}\n{audit_report}"
            ),
        )?;
        return Ok(path);
    }
    let hartlap = (n - p - 2) as f64 / (n - 1) as f64;
    let raw = chi2(&delta, &stats.cov).ok_or("共分散が正定値でない")?;
    let corrected = hartlap * raw;
    let predictive = corrected / (1.0 + 1.0 / n as f64);
    let real_values: Vec<_> = real.iter().map(|r| r.zeta).collect();
    let (pl, pu, erl) = extreme_rank_test(&real_values, &vectors).ok_or("順位検定の入力が不正")?;
    let (offset, offset_sd) = gls_mean(&delta, &stats.cov, hartlap).ok_or("GLS が解けない")?;
    let mut cov_out = String::from("i,j,cov_proc\n");
    for (i, row) in stats.cov.iter().enumerate() {
        for (j, c) in row.iter().enumerate() {
            writeln!(cov_out, "{i},{j},{c:.12}")?;
        }
    }
    results::write_text(&root.join(format!("{output}_cov.csv")), &cov_out)?;
    let tests = format!(
        "runs,bins,hartlap,chi2_raw,chi2_hartlap,p_chi2_approx,chi2_predictive,p_predictive_approx,rank_p_lower,rank_p_upper,rank_p_erl,constant_offset,offset_sd_single,offset_sd_mean,offset_sd_predictive\n{n},{p},{hartlap:.9},{raw:.9},{corrected:.9},{:.9},{predictive:.9},{:.9},{pl:.9},{pu:.9},{erl:.9},{offset:.9},{offset_sd:.9},{:.9},{:.9}\n",
        chi2_sf(corrected, p),
        chi2_sf(predictive, p),
        offset_sd / (n as f64).sqrt(),
        offset_sd * (1.0 + 1.0 / n as f64).sqrt()
    );
    results::write_text(&root.join(format!("{output}_tests.csv")), &tests)?;
    let mut sensitivity = String::from(
        "shift,sigma_single,sigma_one_mean_N,sigma_two_means_N,target_sigma,n_one_mean,n_each_two_means\n",
    );
    for shift in [0.01f64, 0.02, 0.03, 0.05] {
        for target in [1.0f64, 3.0, 5.0] {
            let required = (target * offset_sd / shift).powi(2);
            writeln!(
                sensitivity,
                "{shift:.3},{:.6},{:.6},{:.6},{target:.0},{:.0},{:.0}",
                shift / offset_sd,
                shift * (n as f64).sqrt() / offset_sd,
                shift * (n as f64 / 2.0).sqrt() / offset_sd,
                required.ceil(),
                (2.0 * required).ceil()
            )?;
        }
    }
    results::write_text(
        &root.join(format!("{output}_sensitivity.csv")),
        &sensitivity,
    )?;
    let mut report = format!(
        "# Field mock comparison (#81)\n\nN={n}, bins={p}. All comparisons use proc. Standard deviations below describe one catalogue, not the error on the mock mean.\n\n"
    );
    report.push_str(&xi_report);
    report.push_str(&audit_report);
    let true_nonzero = true_stats
        .value
        .iter()
        .zip(&true_stats.sigma)
        .filter(|(m, s)| m.abs() > 3.0 * **s / (n as f64).sqrt())
        .count();
    let true_min = true_stats
        .value
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let true_max = true_stats
        .value
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let bias: Vec<_> = stats
        .value
        .iter()
        .zip(&true_stats.value)
        .map(|(p, t)| p - t)
        .collect();
    writeln!(
        report,
        "True mean range: {true_min:.6} to {true_max:.6}; bins with |true mean| > 3 SE: {true_nonzero}/{p} (uncorrected marginal diagnostics). Proc-minus-true mean range: {:.6} to {:.6}. Nominal symmetry of the pre-clipping field does not guarantee zero measured true zeta.\n",
        bias.iter().copied().fold(f64::INFINITY, f64::min),
        bias.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    )?;
    writeln!(
        report,
        "Hartlap factor: {hartlap:.6}; chi-square: {corrected:.6}; approximate chi-square p: {:.6}. Including uncertainty on the mock mean: {predictive:.6}, approximate p={:.6}.\n\nExtreme-rank two-sided global p interval: ({pl:.6}, {pu:.6}]; ERL refinement p={erl:.6}.\n\nUniform offset GLS: {offset:.6} +/- {:.6} (one observed catalogue versus estimated mock mean). Single-catalogue offset SD: {offset_sd:.6}; mock-mean SE: {:.6}.\n",
        chi2_sf(corrected, p),
        chi2_sf(predictive, p),
        offset_sd * (1.0 + 1.0 / n as f64).sqrt(),
        offset_sd / (n as f64).sqrt()
    )?;
    report.push_str("| sides km | real | true mean | proc mean | proc SD | deviation/SD | mocks <= real | rank among N+1 |\n|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let mut within = 0;
    for (i, r) in real.iter().enumerate() {
        let tm = mocks.values().map(|v| v[&r.k].0).sum::<f64>() / n as f64;
        let le = vectors.iter().filter(|v| v[i] <= r.zeta).count();
        let z = delta[i] / stats.sigma[i];
        within += usize::from(z.abs() <= 1.0);
        writeln!(
            report,
            "| {:.0},{:.0},{:.0} | {:.6} | {tm:.6} | {:.6} | {:.6} | {z:+.3} | {le}/{n} | {}/{} |",
            r.r_km.0,
            r.r_km.1,
            r.r_km.2,
            r.zeta,
            stats.value[i],
            stats.sigma[i],
            le + 1,
            n + 1
        )?;
    }
    writeln!(
        report,
        "\nWithin 1 SD: {within}/{p} (counted by Rust). Rank uses <=, including ties.\n\nSensitivity assumes a uniform additive shift and unchanged covariance; it does not generate a physical negative-zeta point process. n_one_mean treats the reference mean as known; n_each_two_means uses two independent ensembles of equal size. These sample sizes count independent catalogues, not extra mocks improving a single observed catalogue. More mocks only reduce calibration uncertainty.\n\nThe chi-square p values assume approximately Gaussian vectors and are approximate with estimated covariance. Hartlap is unbiased only for Gaussian independent realizations. Rank calibration is conditional on the fitted selection and chosen xi parameters (plug-in null, not an unconditional composite-null test). A shared finite pool and reference catalogue are conditioned on. Clipping and fixed-count sampling can make true zeta nonzero; consult true means, not just the nominal expected=0 column.\n\nSources: https://arxiv.org/abs/astro-ph/0608064 ; https://arxiv.org/abs/1307.0239\n\nInputs:\n"
    )?;
    writeln!(report, "- real: {real_path}")?;
    for stem in stems {
        writeln!(report, "- {stem}")?;
    }
    let report_path = format!("{output}_report.md");
    results::write_text(&root.join(&report_path), &report)?;
    eprintln!("{tests}");
    Ok(report_path)
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

/// matched-density の paired residual、二点、約10km格子の密度比を Rust でまとめる。
pub fn summarize_mock_density(
    root: &Path,
    stem: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let rows = results::read_numeric_csv(&root.join(format!("{stem}.csv")))?;
    let xis = results::read_numeric_csv(&root.join(format!("{stem}_xi.csv")))?;
    let density = results::read_numeric_csv(&root.join(format!("{stem}_density_summary.csv")))?;
    let timing = results::read_numeric_csv(&root.join(format!("{stem}_timing.csv")))?;
    let key = |r: &std::collections::HashMap<String, f64>| {
        (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize)
    };
    let keys: std::collections::BTreeSet<_> = rows.iter().map(key).collect();
    let mut out = format!(
        "# Mock density test\n\nStem: `{stem}`.\n\nζ differences are paired within each mock; errors are sample SD / sqrt(N). Match uses its own RRR for expected_match. Columns true/proc use the original expected.\n\n| sides (km) | runs | match − expected_match | true − expected | proc − expected |\n|---|---:|---:|---:|---:|\n"
    );
    let mut csv = String::from(
        "k1,k2,k3,r1_km,r2_km,r3_km,runs,match_diff_mean,match_diff_se,true_diff_mean,true_diff_se,proc_diff_mean,proc_diff_se\n",
    );
    let mut bins = 0usize;
    let mut se_bins = 0usize;
    let mut beyond = [0usize; 3];
    for k in keys {
        let v: Vec<_> = rows
            .iter()
            .filter(|r| key(r) == k)
            .filter(|r| {
                [
                    "zeta_match",
                    "expected_match",
                    "zeta_true",
                    "zeta_proc",
                    "expected",
                ]
                .iter()
                .all(|name| r.get(*name).is_some_and(|x| x.is_finite()))
            })
            .collect();
        if v.is_empty() {
            continue;
        }
        if v.len() >= 2 {
            se_bins += 1;
        }
        let residual = |name: &str, expected: &str| {
            let values: Vec<_> = v.iter().map(|r| r[name] - r[expected]).collect();
            let stat = mock_summary(0.0, &values);
            (stat.mean, stat.sd / (stat.n as f64).sqrt())
        };
        let stats = [
            residual("zeta_match", "expected_match"),
            residual("zeta_true", "expected"),
            residual("zeta_proc", "expected"),
        ];
        let (a, b, c) = (v[0]["r1_km"], v[0]["r2_km"], v[0]["r3_km"]);
        write!(out, "| {a:.3}, {b:.3}, {c:.3} | {}", v.len())?;
        write!(
            csv,
            "{},{},{},{a:.3},{b:.3},{c:.3},{}",
            k.0,
            k.1,
            k.2,
            v.len()
        )?;
        for (i, (mean, se)) in stats.iter().enumerate() {
            write!(out, " | {mean:+.6} ± {se:.6}")?;
            write!(csv, ",{mean:.6},{se:.6}")?;
            if se.is_finite() && mean.abs() > 2.0 * se {
                beyond[i] += 1;
            }
        }
        writeln!(out, " |")?;
        writeln!(csv)?;
        bins += 1;
    }
    writeln!(
        out,
        "\nBins: {bins}; bins with at least two runs: {se_bins}. Bins exceeding 2 standard errors (absolute residual): match {}/{se_bins}; true {}/{se_bins}; proc {}/{se_bins}. Gaussian null expected count: {:.3} (bins are correlated; this is not a global test).\n",
        beyond[0],
        beyond[1],
        beyond[2],
        se_bins as f64 * 0.045_500_263_896_358_4
    )?;
    results::write_text(&root.join(format!("{stem}_density_test_summary.csv")), &csv)?;
    out.push_str("ξ means ± standard error across mocks:\n\n| r (km) | runs | true | match | proc |\n|---|---:|---:|---:|---:|\n");
    let ks: std::collections::BTreeSet<_> = xis
        .iter()
        .filter(|r| r["run"] >= 0.0)
        .map(|r| r["k"] as usize)
        .collect();
    let mut xc = String::from(
        "k,r_km,runs,true_mean,true_se,true_sd,match_mean,match_se,match_sd,proc_mean,proc_se,proc_sd\n",
    );
    for k in ks {
        let v: Vec<_> = xis
            .iter()
            .filter(|r| r["run"] >= 0.0 && r["k"] as usize == k)
            .collect();
        let r = v[0]["r_km"];
        write!(out, "| {r:.3} | {}", v.len())?;
        write!(xc, "{k},{r:.3},{}", v.len())?;
        for col in ["xi_true", "xi_match", "xi_proc"] {
            let values: Vec<_> = v.iter().map(|r| r[col]).collect();
            let stat = mock_summary(0.0, &values);
            let se = stat.sd / (stat.n as f64).sqrt();
            write!(out, " | {:.6} ± {se:.6}", stat.mean)?;
            write!(xc, ",{:.6},{se:.6},{:.6}", stat.mean, stat.sd)?;
        }
        writeln!(out, " |")?;
        writeln!(xc)?;
    }
    results::write_text(
        &root.join(format!("{stem}_density_test_xi_summary.csv")),
        &xc,
    )?;
    let d = density.first().ok_or("密度の診断がない")?;
    out.push_str("\nDensity ratio g on the approximately 10 km grid:\n\n");
    writeln!(
        out,
        "Pool = {:.0} points (large sample of the true baseline); aggregate match = {:.0} points. Retain cells with pool count ≥ {:.0}: {:.0} cells, {:.6} of pool mass. All ratios normalized by catalog size and weighted by pool counts.\n",
        d["pool_points"], d["match_points"], d["min_pool_count"], d["cells"], d["mass_fraction"]
    )?;
    writeln!(
        out,
        "g mean {:.6}; raw variance {:.6}; weighted 10/50/90% quantiles {:.6} / {:.6} / {:.6}.\n",
        d["g_mean"], d["g_variance"], d["g_q10"], d["g_q50"], d["g_q90"]
    )?;
    writeln!(
        out,
        "Split-sample cross mean-square displacement from g=1: {:.6}; sqrt(max(0, cross)) = {:.6}. Independent match halves remove the self product of sampling noise; the pool stays fixed. Raw ratios against the smaller true random catalog are also saved in density_cells.csv.\n",
        d["split_cross_mse"],
        d["split_cross_mse"].max(0.0).sqrt()
    )?;
    if let Some(t) = timing.last() {
        writeln!(
            out,
            "Completed {:.0} runs. Instant {:.3} s; wall clock {:.3} s.\n",
            t["run"], t["instant_s"], t["wall_s"]
        )?;
    }
    results::write_text(&root.join(format!("{stem}_density_test.md")), &out)?;
    Ok(out)
}

/// 対にした標本 (x_i, y_i) の要約：差 x − y の平均と標準誤差、y を x に回帰した傾き・その標準誤差・切片。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairedStats {
    pub n: usize,
    pub x_mean: f64,
    pub x_sem: f64,
    pub y_mean: f64,
    pub y_sem: f64,
    pub diff_mean: f64,
    pub diff_sem: f64,
    pub slope: f64,
    pub slope_se: f64,
    pub intercept: f64,
}

pub fn paired_stats(x: &[f64], y: &[f64]) -> PairedStats {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    let nf = n as f64;
    let mean = |v: &[f64]| v.iter().sum::<f64>() / nf;
    let sem = |v: &[f64], m: f64| {
        (v.iter().map(|a| (a - m).powi(2)).sum::<f64>() / (nf - 1.0) / nf).sqrt()
    };
    let (xm, ym) = (mean(x), mean(y));
    let d: Vec<f64> = x.iter().zip(y).map(|(a, b)| a - b).collect();
    let dm = mean(&d);
    let sxx: f64 = x.iter().map(|a| (a - xm).powi(2)).sum();
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - xm) * (b - ym)).sum();
    let slope = sxy / sxx;
    let intercept = ym - slope * xm;
    let rss: f64 = x
        .iter()
        .zip(y)
        .map(|(a, b)| (b - intercept - slope * a).powi(2))
        .sum();
    PairedStats {
        n,
        x_mean: xm,
        x_sem: sem(x, xm),
        y_mean: ym,
        y_sem: sem(y, ym),
        diff_mean: dm,
        diff_sem: sem(&d, dm),
        slope,
        slope_se: (rss / (nf - 2.0) / sxx).sqrt(),
        intercept,
    }
}

/// 選択関数がどれだけ吸い込むかを、模擬データの「正しい基準」と「店と同じ手順（proc）」の対から測る（#94）。
///
/// `files` は模擬データの ζ の CSV（run ごと）。同じ名前の `_xi.csv` も読む。`baseline` は正しい基準の列の名前の後半
/// （"true" なら zeta_true／xi_true、"match" なら zeta_match／xi_match）。"基準:比べる列" の形なら比べる列も選べる
/// （例："half_true:half_split" は zeta_half_true と zeta_half_split）。既定の比べる列は proc。
/// どちらかが NaN の行（半分の点で空になったビン）は除く。
/// 書くもの：data/zeta/absorption_`label`.csv（ζ）と absorption_`label`_xi.csv（ξ）。
/// 相対的な吸い込み（吸い込み ÷ 正しい基準の平均）は、正しい基準の平均が 2 SE を超えるビンだけ数える。
pub fn absorption(
    root: &Path,
    label: &str,
    baseline: &str,
    files: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    type Row = std::collections::HashMap<String, f64>;
    let mut zrows: Vec<Row> = Vec::new();
    let mut xrows: Vec<Row> = Vec::new();
    for f in files {
        let p = root.join(f);
        zrows.extend(
            results::read_numeric_csv(&p)?
                .into_iter()
                .filter(|r| r["run"] >= 0.0),
        );
        let xp = p.with_file_name(format!(
            "{}_xi.csv",
            p.file_stem()
                .and_then(|s| s.to_str())
                .ok_or("ファイル名が読めない")?
        ));
        xrows.extend(
            results::read_numeric_csv(&xp)?
                .into_iter()
                .filter(|r| r["run"] >= 0.0),
        );
    }
    let (base, other) = baseline.split_once(':').unwrap_or((baseline, "proc"));
    let (zt, zp) = (format!("zeta_{base}"), format!("zeta_{other}"));
    let (xt, xp) = (format!("xi_{base}"), format!("xi_{other}"));
    for col in [&zt, &zp] {
        if !zrows.first().is_some_and(|r| r.contains_key(col)) {
            return Err(format!("列 {col} がない").into());
        }
    }
    let finite = |r: &Row, a: &str, b: &str| r[a].is_finite() && r[b].is_finite();
    zrows.retain(|r| finite(r, &zt, &zp));
    xrows.retain(|r| finite(r, &xt, &xp));

    let head = "runs,true_mean,true_sem,proc_mean,proc_sem,absorb_mean,absorb_sem,rel_absorb,slope,slope_se,intercept";
    let fmt = |s: &PairedStats| {
        let rel = if s.x_mean.abs() > 2.0 * s.x_sem {
            s.diff_mean / s.x_mean
        } else {
            f64::NAN
        };
        format!(
            "{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{rel:.4},{:.4},{:.4},{:.6}",
            s.n,
            s.x_mean,
            s.x_sem,
            s.y_mean,
            s.y_sem,
            s.diff_mean,
            s.diff_sem,
            s.slope,
            s.slope_se,
            s.intercept
        )
    };

    // ξ
    let mut xo = format!("k,r_km,{head}\n");
    let mut xi_rel = std::collections::BTreeMap::new();
    let ks: std::collections::BTreeSet<usize> = xrows.iter().map(|r| r["k"] as usize).collect();
    for k in ks {
        let v: Vec<_> = xrows.iter().filter(|r| r["k"] as usize == k).collect();
        let s = paired_stats(
            &v.iter().map(|r| r[&xt]).collect::<Vec<_>>(),
            &v.iter().map(|r| r[&xp]).collect::<Vec<_>>(),
        );
        xi_rel.insert(k, (v[0]["r_km"], s));
        writeln!(xo, "{k},{:.3},{}", v[0]["r_km"], fmt(&s))?;
    }
    results::write_text(
        &root.join(format!("data/zeta/absorption_{label}_xi.csv")),
        &xo,
    )?;

    // ζ
    let mut zo = format!("k1,k2,k3,r1_km,r2_km,r3_km,{head}\n");
    let keys: std::collections::BTreeSet<(usize, usize, usize)> = zrows
        .iter()
        .map(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize))
        .collect();
    let (mut n_bins, mut n_sig_absorb, mut n_pos_absorb, mut n_slope_lt1) = (0, 0, 0, 0);
    let mut summary = String::new();
    for (a, b, c) in keys {
        let v: Vec<_> = zrows
            .iter()
            .filter(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize) == (a, b, c))
            .collect();
        let s = paired_stats(
            &v.iter().map(|r| r[&zt]).collect::<Vec<_>>(),
            &v.iter().map(|r| r[&zp]).collect::<Vec<_>>(),
        );
        n_bins += 1;
        n_sig_absorb += usize::from(s.diff_mean.abs() > 2.0 * s.diff_sem);
        n_pos_absorb += usize::from(s.diff_mean > 0.0);
        n_slope_lt1 += usize::from(s.slope + 2.0 * s.slope_se < 1.0);
        writeln!(
            zo,
            "{a},{b},{c},{:.3},{:.3},{:.3},{}",
            v[0]["r1_km"],
            v[0]["r2_km"],
            v[0]["r3_km"],
            fmt(&s)
        )?;
        // 正三角形のビンは、同じ辺の ξ の相対的な吸い込みと並べる（Hui & Gaztañaga 1999 の 3c12/S3 との比較）
        if a == b
            && b == c
            && let Some((r, sx)) = xi_rel.get(&a)
        {
            let rz = s.diff_mean / s.x_mean;
            let rx = sx.diff_mean / sx.x_mean;
            writeln!(
                summary,
                "  正三角形 {r:.0} km：ζ 吸い込み {:+.4}±{:.4}（正しい基準 {:+.4}±{:.4}、相対 {rz:+.2}）、\
                 ξ 吸い込み {:+.4}±{:.4}（相対 {rx:+.3}）、比 {:+.2}、傾き {:.2}±{:.2}、切片 {:+.4}",
                s.diff_mean,
                s.diff_sem,
                s.x_mean,
                s.x_sem,
                sx.diff_mean,
                sx.diff_sem,
                rz / rx,
                s.slope,
                s.slope_se,
                s.intercept
            )?;
        }
    }
    results::write_text(&root.join(format!("data/zeta/absorption_{label}.csv")), &zo)?;
    let msg = format!(
        "{label}（{} run 分の行、基準 {baseline}）：ビン {n_bins} 個のうち、吸い込みが 2 SE を超える {n_sig_absorb}、\
         吸い込みが正（proc が小さい）{n_pos_absorb}、傾きが 1 より 2 SE 以上小さい {n_slope_lt1}\n{summary}",
        zrows.len() / n_bins.max(1)
    );
    Ok(msg)
}

/// ノート（#94）の表を二つ書く。どちらも [`absorption`] を走らせ直してから、その CSV を読む。
///
/// - docs/tables/zeta_absorption.tex：模型ごとの、正三角形のビンでの ζ と同じ辺の ξ の相対的な吸い込み
/// - docs/tables/zeta_gauss_offset.tex：0 で切る点がほぼないガウス場で、店と同じ手順の ζ̂ が負になる大きさ
///
/// あわせて、負の ζ̂ の平均が 3 km の ξ の何乗で増えるか、それをコスモスの ξ に直した大きさ、
/// 店を半分に分けた測り方（#94）の結果を、文字で返す。
pub fn absorption_tables(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    const G: &str = "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_a21.2_s220_m81_20261114_mainG";
    const E: &str =
        "data/zeta/field_jp_2026-11-12_cosmos_a0.6_s10_a20.9_s220_m81_20261114_compareE";
    let four = |stem: &str| {
        (0..4)
            .map(|w| format!("{stem}_w{w}.csv"))
            .collect::<Vec<_>>()
    };
    let models: Vec<(&str, &str, &str, Vec<String>)> = vec![
        (
            "clumps_match",
            "NS",
            "match",
            vec!["data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20.csv".into()],
        ),
        (
            "field90",
            "$G_{10}$",
            "true",
            vec!["data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10.csv".into()],
        ),
        ("mainG", "$G_{10,20}$", "true", four(G)),
        ("compareE", "$G'_{10,20}$", "true", four(E)),
    ];
    // (ラベル, 振幅, 乱数の種, 前置き)。種 1 の三つは同じ中心と符号の列（場の形が同じ）で、振幅だけ違う。
    let gauss: [(&str, &str, &str, &str); 5] = [
        (
            "gauss015",
            "0.15",
            "1",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.15_s10_m94_gauss",
        ),
        (
            "gauss03",
            "0.3",
            "1",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_gauss",
        ),
        (
            "gauss04",
            "0.4",
            "1",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.4_s10_m94_gauss",
        ),
        (
            "gauss03_seed2",
            "0.3",
            "2",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_seed2",
        ),
        (
            "gauss03_seed3",
            "0.3",
            "3",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_seed3",
        ),
    ];
    // ζ̂ の分子と分母を書いたもの（--counts on）。比の偏りの切り分けに使う。
    let counts: [(&str, &str); 4] = [
        (
            "counts03",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_counts.csv",
        ),
        (
            "counts04",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.4_s10_m94_counts.csv",
        ),
        (
            "seed2",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_seed2.csv",
        ),
        (
            "seed3",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_seed3.csv",
        ),
    ];
    let split = "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_split.csv".to_string();
    let read = |label: &str, xi: bool| {
        results::read_numeric_csv(&root.join(format!(
            "data/zeta/absorption_{label}{}.csv",
            if xi { "_xi" } else { "" }
        )))
    };
    let cell = |v: f64| {
        if v.is_finite() {
            format!("${v:.2}$")
        } else {
            "—".to_string()
        }
    };
    let mut log = String::new();

    // 表 1：相対的な吸い込み
    let mut t1 = String::from(
        "% cargo run --release --bin zeta_absorption_tables が書いた表\n\
         \\begin{tabular}{lrrrrrrrrr}\n\\toprule\n\
         & & \\multicolumn{4}{c}{$\\Delta\\hat\\xi / \\hat\\xi_W$, $r$ [km]} & \\multicolumn{4}{c}{$\\Delta\\hat\\zeta / \\hat\\zeta_W$, $r_1 = r_2 = r_3$ [km]} \\\\\n\
         & $N$ & 3 & 6 & 12 & 24 & 3 & 6 & 12 & 24 \\\\\n\\midrule\n",
    );
    for (label, name, baseline, files) in &models {
        absorption(root, label, baseline, files)?;
        let (z, x) = (read(label, false)?, read(label, true)?);
        let runs = z.first().map_or(0.0, |r| r["runs"]);
        let xs: Vec<String> = x.iter().map(|r| cell(r["rel_absorb"])).collect();
        let zs: Vec<String> = z
            .iter()
            .filter(|r| r["k1"] == r["k2"] && r["k2"] == r["k3"])
            .map(|r| cell(r["rel_absorb"]))
            .collect();
        writeln!(
            t1,
            "{name} & {runs:.0} & {} & {} \\\\",
            xs.join(" & "),
            zs.join(" & ")
        )?;
        // 小さい尺度の ζ が吸い込まれないことは、相対値でなく差そのもので見る
        for r in z
            .iter()
            .filter(|r| r["k1"] == r["k2"] && r["k2"] == r["k3"])
        {
            writeln!(
                log,
                "{label} 正三角形 {:.0} km：正しい基準 {:+.4}±{:.4}、吸い込み {:+.4}±{:.4}、傾き {:.2}±{:.2}",
                r["r1_km"],
                r["true_mean"],
                r["true_sem"],
                r["absorb_mean"],
                r["absorb_sem"],
                r["slope"],
                r["slope_se"]
            )?;
        }
        let n_sig = z
            .iter()
            .filter(|r| r["absorb_mean"].abs() > 2.0 * r["absorb_sem"])
            .count();
        writeln!(
            log,
            "{label}：吸い込みが 2 SE を超えるビン {n_sig}/{}",
            z.len()
        )?;
    }
    t1.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/zeta_absorption.tex"), &t1)?;

    // 表 2：ガウス場の負の ζ̂
    let mut t2 = String::from(
        "% cargo run --release --bin zeta_absorption_tables が書いた表\n\
         \\begin{tabular}{rrrrrrrr}\n\\toprule\n\
         & & & & \\multicolumn{3}{c}{$\\hat\\zeta_S$} & $\\hat\\zeta_W$ \\\\\n\
         $A$ & 種 & $\\max f_{\\mathrm{clip}}$ & $\\hat\\xi_W(3\\,\\mathrm{km})$ & $\\overline{\\hat\\zeta_S}$ & $< 0$ & $< -2\\,\\mathrm{SE}$ & $|t| > 2$ \\\\\n\\midrule\n",
    );
    let mut pts = Vec::new();
    for (label, amp, seed, stem) in gauss {
        absorption(root, label, "true", &[format!("{stem}.csv")])?;
        let (z, x) = (read(label, false)?, read(label, true)?);
        let diag = results::read_numeric_csv(&root.join(format!("{stem}_diagnostics.csv")))?;
        let clip = diag
            .iter()
            .map(|r| r["clipped_fraction"])
            .fold(0.0, f64::max);
        let xi3 = x.iter().find(|r| r["k"] == 0.0).ok_or("3 km の ξ がない")?["true_mean"];
        let n = z.len() as f64;
        let mean = z.iter().map(|r| r["proc_mean"]).sum::<f64>() / n;
        let neg = z.iter().filter(|r| r["proc_mean"] < 0.0).count();
        let sig = z
            .iter()
            .filter(|r| r["proc_mean"] < -2.0 * r["proc_sem"])
            .count();
        let tsig = z
            .iter()
            .filter(|r| r["true_mean"].abs() > 2.0 * r["true_sem"])
            .count();
        let (lo, hi) = z
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| {
                (a.min(r["proc_mean"]), b.max(r["proc_mean"]))
            });
        writeln!(
            t2,
            "${amp}$ & {seed} & ${:.1}\\%$ & ${xi3:.3}$ & ${mean:+.4}$ & {neg}/16 & {sig}/16 & {tsig}/16 \\\\",
            100.0 * clip
        )?;
        let tmean = z.iter().map(|r| r["true_mean"]).sum::<f64>() / n;
        writeln!(
            log,
            "振幅 {amp}・種 {seed}：ξ(3km) {xi3:.4}、店と同じ手順の ζ̂ の平均 {mean:+.5}、範囲 {lo:+.4}〜{hi:+.4}、正しい基準の平均 {tmean:+.5}"
        )?;
        if seed == "1" {
            pts.push((xi3, mean, lo, hi));
        }
    }
    for (label, file) in counts {
        log.push_str(&ratio_decomposition(root, label, file)?);
    }
    t2.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/zeta_gauss_offset.tex"), &t2)?;

    // 何乗で増えるか（両端の振幅から）と、コスモスの ξ に直した大きさ
    let real_xi3 = results::read_numeric_csv(&root.join(format!("{}_xi.csv", gauss[0].3)))?
        .into_iter()
        .find(|r| r["run"] < 0.0 && r["k"] == 0.0)
        .ok_or("コスモスの ξ がない")?["xi_true"];
    let slope =
        |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| (b.1 / a.1).ln() / (b.0 / a.0).ln();
    let (p0, p1, p2) = (pts[0], pts[1], pts[2]);
    let scale = (real_xi3 / p2.0).powi(2);
    writeln!(
        log,
        "負の ζ̂ の平均の、3 km の ξ に対する冪：{:.2}（0.15→0.3）、{:.2}（0.3→0.4）、{:.2}（0.15→0.4）\n\
         振幅 0.4 を ξ² でコスモス（3 km の ξ {real_xi3:.3}）に直すと：平均 {:+.4}、範囲 {:+.4}〜{:+.4}",
        slope(p0, p1),
        slope(p1, p2),
        slope(p0, p2),
        p2.1 * scale,
        p2.2 * scale,
        p2.3 * scale
    )?;

    // 店を半分に分けた測り方
    for (label, cols) in [
        ("split_self", "half_true:half_proc"),
        ("split_cross", "half_true:half_split"),
        ("split_self_vs_cross", "half_proc:half_split"),
    ] {
        absorption(root, label, cols, std::slice::from_ref(&split))?;
        let z = read(label, false)?;
        let n = z.len() as f64;
        let mean = z.iter().map(|r| r["proc_mean"]).sum::<f64>() / n;
        let neg = z.iter().filter(|r| r["proc_mean"] < 0.0).count();
        let sig = z
            .iter()
            .filter(|r| r["proc_mean"] < -2.0 * r["proc_sem"])
            .count();
        let pos_diff = z.iter().filter(|r| r["absorb_mean"] > 0.0).count();
        let sig_diff = z
            .iter()
            .filter(|r| r["absorb_mean"].abs() > 2.0 * r["absorb_sem"])
            .count();
        writeln!(
            log,
            "{label}（{cols}）：比べる列の平均 {mean:+.5}、負 {neg}/16、−2 SE 未満 {sig}/16、差が正 {pos_diff}/16、差が 2 SE 超 {sig_diff}/16"
        )?;
    }

    // 表 3：ガウス場の負の ζ̂ の内訳（二度ならしたずれの見込みと、比の偏り）。
    // ずれの係数は、三角形の数そのもので求めたもの（zeta_mismatch --seed 1 --tri-thin 4 が書いたもの）。
    const MISMATCH: &str = "data/zeta/selection_mismatch_tri_seed1_thin4.csv";
    const F: &str = "data/zeta/field_jp_2026-11-12_cosmos";
    let budget: [(&str, &str, &str, String); 7] = [
        ("gauss0", "0", "1", format!("{F}_a0_s10_m94_amp0.csv")),
        (
            "gauss03",
            "0.3",
            "1",
            format!("{F}_a0.3_s10_m94_counts.csv"),
        ),
        (
            "gauss03_seed2",
            "0.3",
            "2",
            format!("{F}_a0.3_s10_m94_seed2.csv"),
        ),
        (
            "gauss03_seed3",
            "0.3",
            "3",
            format!("{F}_a0.3_s10_m94_seed3.csv"),
        ),
        (
            "gauss04",
            "0.4",
            "1",
            format!("{F}_a0.4_s10_m94_counts.csv"),
        ),
        (
            "gauss04_seed2",
            "0.4",
            "2",
            format!("{F}_a0.4_s10_m94_seed2.csv"),
        ),
        (
            "gauss04_seed3",
            "0.4",
            "3",
            format!("{F}_a0.4_s10_m94_seed3.csv"),
        ),
    ];
    let mut t3 = String::from(
        "% cargo run --release --bin zeta_absorption_tables が書いた表\n\
         \\begin{tabular}{rrrrrrr}\n\\toprule\n\
         & & & \\multicolumn{4}{c}{16 ビンの平均} \\\\\n\
         $A$ & 種 & $\\max f_{\\mathrm{clip}}$ & $\\hat\\zeta_S$ & $\\zeta_w$ & $\\zeta_{N/D}$ & $\\hat\\zeta_S - \\zeta_w - \\zeta_{N/D}$ \\\\\n\\midrule\n",
    );
    let labels: Vec<String> = budget.iter().map(|b| b.0.to_string()).collect();
    for (label, _, _, file) in &budget {
        absorption(root, label, "true", std::slice::from_ref(file))?;
    }
    log.push_str(&mismatch_prediction(root, MISMATCH, &labels)?);
    for (label, amp, seed, file) in &budget {
        let pred = results::read_numeric_csv(
            &root.join(format!("data/zeta/mismatch_pred_{label}_tri.csv")),
        )?;
        let n = pred.len() as f64;
        let obs = pred.iter().map(|r| r["observed"]).sum::<f64>() / n;
        let mis = pred.iter().map(|r| r["predicted"]).sum::<f64>() / n;
        ratio_decomposition(root, &format!("budget_{label}"), file)?;
        let rb =
            results::read_numeric_csv(&root.join(format!("data/zeta/ratio_budget_{label}.csv")))?;
        // ratio_ の CSV は基準ごとに 16 行ずつ（true、proc、…の順）。proc の 16 行を使う
        let ratio = rb[pred.len()..2 * pred.len()]
            .iter()
            .map(|r| r["ratio_bias_pred"])
            .sum::<f64>()
            / n;
        let diag = results::read_numeric_csv(&root.join(file.replace(".csv", "_diagnostics.csv")))?;
        let clip = diag
            .iter()
            .map(|r| r["clipped_fraction"])
            .fold(0.0, f64::max);
        writeln!(
            t3,
            "${amp}$ & {seed} & ${:.1}\\%$ & ${obs:+.4}$ & ${mis:+.4}$ & ${ratio:+.4}$ & ${:+.4}$ \\\\",
            100.0 * clip,
            obs - mis - ratio
        )?;
        writeln!(
            log,
            "内訳 振幅 {amp}・種 {seed}：実測 {obs:+.5}、ずれ {mis:+.5}、比の偏り {ratio:+.5}、残り {:+.5}、(ずれ+比の偏り)/実測 {:.0}%、切った点 最大 {:.1}%",
            obs - mis - ratio,
            100.0 * (mis + ratio) / obs,
            100.0 * clip
        )?;
    }
    t3.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/zeta_gauss_budget.tex"), &t3)?;
    Ok(log)
}

/// 場がなくても残る、店から作った選択関数のずれ（二度ならしたずれ）を測る（#94）。
///
/// [`field_mocks`] と同じ種で、同じ参照の点・コスモスの選択関数 S_real・点の池（W = 人口 × S_real に比例）を作る。
/// 池そのもの（16 万点）から店と同じ手順の選択関数 S_pool を作ると、模擬の店から作る選択関数の期待値になる。
/// 店の割合の密度と、店と同じ手順のランダムの割合の密度の比は
/// w(x) = ⟨w₀⟩_W / w₀(x)、w₀ = S_pool / S_real で、式 eq:zeta-edge の w にあたる。
/// 三角形の重み W³ は、池の点に「半径 r の中の池の点の数」の二乗を掛けて近づける（r は辺のビンの中点）。
/// 池の 8 個に 1 個で平均し、r ごとに ⟨w²(1−w)⟩₃ と ⟨(w−1)³⟩₃ を返す。
pub fn selection_mismatch(root: &Path, p: &Params) -> Result<String, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let r_km = p.footprint.ok_or("--footprint R が要る")?;
    let (grid, _) = grid_of(root, &p.country)?;
    let pts: Vec<LatLon> = stores::read_selected(&root.join(&p.stores))
        .map_err(|e| format!("{} がない：{e}", p.stores))?
        .into_iter()
        .filter(|(c, _)| *c == p.chain)
        .map(|(_, q)| q)
        .collect();
    let units = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    // field_mocks と同じ乱数列で、参照の点と点の池を作る
    let mut rng_ref = SplitMix64::new(p.seed ^ 0x3E7A_0000_0000_0000);
    let refs: Vec<Unit> = units(&sample(&grid, p.n_ref, Weight::Population, &mut rng_ref));
    let real = units(&pts);
    let sr_real = SmoothRatio::new(&real, &refs, r_km);
    let mut rng_legacy = SplitMix64::new(p.seed ^ 0xF1E1_D000_0000_0000);
    let pool = units(
        &sample_selected(
            &grid,
            20 * p.n_random,
            Weight::Population,
            |q| sr_real.eval(&Unit::from(q)),
            40 * p.n_random,
            &mut rng_legacy,
        )
        .points,
    );
    let sr_pool = SmoothRatio::new(&pool, &refs, r_km);
    let sub: Vec<Unit> = pool.iter().step_by(8).copied().collect();
    let w0: Vec<f64> = sub
        .iter()
        .map(|q| {
            let den = sr_real.eval(q);
            if den > 0.0 {
                sr_pool.eval(q) / den
            } else {
                f64::NAN
            }
        })
        .collect();
    let ok: Vec<usize> = (0..sub.len())
        .filter(|&i| w0[i].is_finite() && w0[i] > 0.0)
        .collect();
    let mean_w0 = ok.iter().map(|&i| w0[i]).sum::<f64>() / ok.len() as f64;
    let w: Vec<f64> = w0.iter().map(|x| mean_w0 / x).collect();
    let sd_w = (ok.iter().map(|&i| (w[i] - 1.0).powi(2)).sum::<f64>() / ok.len() as f64).sqrt();
    let mut log = format!(
        "池 {} 点、平均に使った点 {}（{:.1} 秒）。⟨w₀⟩_W = {mean_w0:.4}、w の 1 からのずれの二乗平均の平方根 {sd_w:.4}\n",
        pool.len(),
        ok.len(),
        t0.elapsed().as_secs_f64()
    );
    let mut out = String::from("r_km,w2_1mw,wm1_cubed,w_mean\n");
    for r in [3.0, 6.0, 12.0, 24.0] {
        let rc = km_to_chord2(r).sqrt();
        let g = crate::util::spatial::UnitGrid::build(&pool, rc);
        let (mut s2, mut s3, mut s1, mut sw) = (0.0, 0.0, 0.0, 0.0);
        for &i in &ok {
            let mut n = 0usize;
            g.for_each_near(&sub[i], rc, |j| {
                if chord2(&sub[i], &pool[j]) < rc * rc {
                    n += 1;
                }
            });
            let wt = (n as f64).powi(2);
            s2 += wt * w[i] * w[i] * (1.0 - w[i]);
            s3 += wt * (w[i] - 1.0).powi(3);
            s1 += wt * w[i];
            sw += wt;
        }
        let (a2, a3, a1) = (s2 / sw, s3 / sw, s1 / sw);
        writeln!(out, "{r},{a2:.6},{a3:.6},{a1:.6}")?;
        writeln!(
            log,
            "r = {r} km：⟨w²(1−w)⟩₃ = {a2:+.5}、⟨(w−1)³⟩₃ = {a3:+.6}、⟨w⟩₃ = {a1:.4}"
        )?;
    }
    results::write_text(
        &root.join(format!("data/zeta/selection_mismatch_seed{}.csv", p.seed)),
        &out,
    )?;

    // 三角形の数そのもので求める：池を thin 個に 1 個に間引いた点で三角形を数え、
    // 三角形の w を三頂点の w の平均として、ビンごとに ⟨w²(1−w)⟩₃、⟨(w−1)³⟩₃、⟨w⟩₃ を求める。
    if let Some(thin) = p.tri_thin {
        let t1 = Instant::now();
        let mut tp: Vec<Unit> = Vec::new();
        let mut tw: Vec<f64> = Vec::new();
        for q in pool.iter().step_by(thin) {
            let den = sr_real.eval(q);
            if den > 0.0 {
                let x = sr_pool.eval(q) / den;
                if x.is_finite() && x > 0.0 {
                    tp.push(*q);
                    tw.push(mean_w0 / x);
                }
            }
        }
        let (lo, hi, n) = p.sides_km;
        let sides_km = Bins::log(lo, hi, n);
        let bins = TriangleBins::new(sides_km.map(km_to_chord2));
        let c = sides_km.centers();
        let (cnt, sums) = crate::engine::triple_count::accumulate_aaa(&tp, &bins, |i, j, k| {
            let w = (tw[i] + tw[j] + tw[k]) / 3.0;
            [w * w * (1.0 - w), (w - 1.0).powi(3), w]
        });
        let mut tout = String::from("k1,k2,k3,r1_km,r2_km,r3_km,n_tri,w2_1mw,wm1_cubed,w_mean\n");
        // 三辺がそろわない（三角形がない）ビンは書かない
        for b in bins.used().into_iter().filter(|&b| cnt[b] > 0) {
            let (k1, k2, k3) = bins.unflatten(b);
            let nt = cnt[b] as f64;
            let [a2, a3, a1] = sums[b].map(|s| s / nt);
            writeln!(
                tout,
                "{k1},{k2},{k3},{:.3},{:.3},{:.3},{},{a2:.6},{a3:.6},{a1:.6}",
                c[k1], c[k2], c[k3], cnt[b]
            )?;
            writeln!(
                log,
                "三角形 ({:.1}, {:.1}, {:.1}) km：{} 個、⟨w²(1−w)⟩₃ = {a2:+.5}、⟨(w−1)³⟩₃ = {a3:+.6}、⟨w⟩₃ = {a1:.4}",
                c[k1], c[k2], c[k3], cnt[b]
            )?;
        }
        writeln!(
            log,
            "三角形の計算：点 {} 個（池の {thin} 個に 1 個）、{:.1} 秒",
            tp.len(),
            t1.elapsed().as_secs_f64()
        )?;
        results::write_text(
            &root.join(format!(
                "data/zeta/selection_mismatch_tri_seed{}_thin{thin}.csv",
                p.seed
            )),
            &tout,
        )?;
    }
    Ok(log)
}

/// 選択関数の幅 R を変えたコスモスの ξ̂ と ζ̂ を並べる（#94）。
///
/// 入力は zeta の --footprint R --xi on --tag fp<R>_t8000 の出力
/// （R = 50 は ζ が zeta_jp_2026-11-12_cosmos_fp50_t8000.csv、ξ が tag fp50_t8000_xi の _xi_xi.csv）。
/// 書くもの：docs/tables/zeta_vary_r.tex（R ごとの ξ̂ と三辺の等しいビンの ζ̂、16 ビンの平均と負のビンの数）
pub fn vary_r_table(root: &Path, rs: &[u32]) -> Result<String, Box<dyn std::error::Error>> {
    const P: &str = "data/zeta/zeta_jp_2026-11-12_cosmos";
    let mut t = String::from(
        "% cargo run --release --bin zeta_vary_r が書いた表\n\
         \\begin{tabular}{rrrrrrrrrrr}\n\\toprule\n\
         & \\multicolumn{4}{c}{$\\hat\\xi_S$, $r$ [km]} & \\multicolumn{4}{c}{$\\hat\\zeta_S$, $r_1 = r_2 = r_3$ [km]} & \\multicolumn{2}{c}{16 ビン} \\\\\n\
         $R$ [km] & 3 & 6 & 12 & 24 & 3 & 6 & 12 & 24 & $\\overline{\\hat\\zeta_S}$ & $< 0$ \\\\\n\\midrule\n",
    );
    let mut log = String::new();
    for &r in rs {
        let (zp, xp) = if r == 50 {
            (
                format!("{P}_fp50_t8000.csv"),
                format!("{P}_fp50_t8000_xi_xi.csv"),
            )
        } else {
            (
                format!("{P}_fp{r}_t8000.csv"),
                format!("{P}_fp{r}_t8000_xi.csv"),
            )
        };
        let z: Vec<_> = results::read_numeric_csv(&root.join(&zp))?
            .into_iter()
            .filter(|r| r["zeta"].is_finite())
            .collect();
        let x = results::read_numeric_csv(&root.join(&xp))?;
        let xs: Vec<String> = x.iter().map(|r| format!("${:.3}$", r["xi"])).collect();
        let eq: Vec<String> = z
            .iter()
            .filter(|r| r["k1"] == r["k2"] && r["k2"] == r["k3"])
            .map(|r| format!("${:+.3}$", r["zeta"]))
            .collect();
        let n = z.len();
        let mean = z.iter().map(|r| r["zeta"]).sum::<f64>() / n as f64;
        let neg = z.iter().filter(|r| r["zeta"] < 0.0).count();
        let (lo, hi) = z
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| {
                (a.min(r["zeta"]), b.max(r["zeta"]))
            });
        writeln!(
            t,
            "{r} & {} & {} & ${mean:+.3}$ & {neg}/{n} \\\\",
            xs.join(" & "),
            eq.join(" & ")
        )?;
        writeln!(
            log,
            "R = {r} km：ξ̂ {}、ζ̂ の 16 ビンの平均 {mean:+.4}（{lo:+.4}〜{hi:+.4}）、負 {neg}/{n}",
            x.iter()
                .map(|r| format!("{:.3}", r["xi"]))
                .collect::<Vec<_>>()
                .join("・")
        )?;
    }
    t.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/zeta_vary_r.tex"), &t)?;
    Ok(log)
}

/// 二度ならしたずれ w（[`selection_mismatch`]）から式 eq:zeta-edge で見込む ζ̂ と、
/// ガウス場の模擬データを店と同じ手順で測った ζ̂ を、ビンごとに比べる（#94）。
///
/// 見込み = ⟨(w−1)³⟩₃ − ⟨w²(1−w)⟩₃ (ξ̂₁ + ξ̂₂ + ξ̂₃)。ξ̂ は同じ模擬データを店と同じ手順で測った ξ の平均
/// （場の揺らぎのうち選択関数に吸い込まれなかった部分）。係数は三角形の三辺の平均の r の値を、
/// 隣の r の値との対数での線形補間で求める。`labels` は [`absorption`] が書いた data/zeta/absorption_`label`.csv。
pub fn mismatch_prediction(
    root: &Path,
    mismatch_csv: &str,
    labels: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let mm = results::read_numeric_csv(&root.join(mismatch_csv))?;
    // 係数の CSV に k1 の列があれば（zeta_mismatch --tri-thin の出力）、三角形のビンごとに引く。
    // 出力のファイル名には _tri をつけ、r で補間した前の結果と分ける。
    let per_bin = mm.first().is_some_and(|m| m.contains_key("k1"));
    let suffix = if per_bin { "_tri" } else { "" };
    let coef_bin = |r: &std::collections::HashMap<String, f64>, col: &str| -> f64 {
        mm.iter()
            .find(|m| m["k1"] == r["k1"] && m["k2"] == r["k2"] && m["k3"] == r["k3"])
            .map_or(f64::NAN, |m| m[col])
    };
    let coef_r = |r: f64, col: &str| -> f64 {
        let xs: Vec<(f64, f64)> = mm.iter().map(|m| (m["r_km"].ln(), m[col])).collect();
        let x = r.ln();
        if x <= xs[0].0 {
            return xs[0].1;
        }
        for w in xs.windows(2) {
            if x <= w[1].0 {
                let t = (x - w[0].0) / (w[1].0 - w[0].0);
                return w[0].1 + t * (w[1].1 - w[0].1);
            }
        }
        xs[xs.len() - 1].1
    };
    let coef = |r: &std::collections::HashMap<String, f64>, col: &str| -> f64 {
        if per_bin {
            coef_bin(r, col)
        } else {
            coef_r((r["r1_km"] + r["r2_km"] + r["r3_km"]) / 3.0, col)
        }
    };
    let mut log = String::new();
    for label in labels {
        let z = results::read_numeric_csv(&root.join(format!("data/zeta/absorption_{label}.csv")))?;
        let x =
            results::read_numeric_csv(&root.join(format!("data/zeta/absorption_{label}_xi.csv")))?;
        let xi = |k: f64| {
            x.iter()
                .find(|r| r["k"] == k)
                .map_or(f64::NAN, |r| r["proc_mean"])
        };
        let mut out = String::from("k1,k2,k3,r1_km,r2_km,r3_km,observed,observed_sem,predicted\n");
        let (mut so, mut sp, mut n_within, mut n_same_sign) = (0.0, 0.0, 0, 0);
        for r in &z {
            let sum_xi = xi(r["k1"]) + xi(r["k2"]) + xi(r["k3"]);
            let pred = coef(r, "wm1_cubed") - coef(r, "w2_1mw") * sum_xi;
            let (obs, sem) = (r["proc_mean"], r["proc_sem"]);
            so += obs;
            sp += pred;
            n_within += usize::from((obs - pred).abs() <= 2.0 * sem);
            n_same_sign += usize::from(obs.signum() == pred.signum());
            writeln!(
                out,
                "{:.0},{:.0},{:.0},{:.3},{:.3},{:.3},{obs:.6},{sem:.6},{pred:.6}",
                r["k1"], r["k2"], r["k3"], r["r1_km"], r["r2_km"], r["r3_km"]
            )?;
        }
        let n = z.len() as f64;
        results::write_text(
            &root.join(format!("data/zeta/mismatch_pred_{label}{suffix}.csv")),
            &out,
        )?;
        writeln!(
            log,
            "{label}：16 ビンの平均で、実測 {:+.5}、見込み {:+.5}。実測と見込みが 2 SE 以内 {n_within}/{}、符号が同じ {n_same_sign}/{}",
            so / n,
            sp / n,
            z.len(),
            z.len()
        )?;
    }

    // コスモス：同じ係数に、コスモスの ξ̂（コスモスから作った選択関数で測ったもの）を入れた見込み
    const COSMOS_XI: &str = "data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_m94_gauss_xi.csv";
    const COSMOS_ZETA: &str = "data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv";
    let cx = results::read_numeric_csv(&root.join(COSMOS_XI))?;
    let cxi = |k: f64| {
        cx.iter()
            .find(|r| r["run"] < 0.0 && r["k"] == k)
            .map_or(f64::NAN, |r| r["xi_true"])
    };
    // 測っていないビン（ζ が NaN）は除く
    let cz: Vec<_> = results::read_numeric_csv(&root.join(COSMOS_ZETA))?
        .into_iter()
        .filter(|r| r["zeta"].is_finite())
        .collect();
    let mut out = String::from("k1,k2,k3,r1_km,r2_km,r3_km,cosmos_zeta,predicted\n");
    let (mut lo, mut hi, mut sum) = (f64::INFINITY, f64::NEG_INFINITY, 0.0);
    let (mut n_below, mut diff) = (0usize, 0.0);
    let mut each = Vec::new();
    for r in &cz {
        let pred =
            coef(r, "wm1_cubed") - coef(r, "w2_1mw") * (cxi(r["k1"]) + cxi(r["k2"]) + cxi(r["k3"]));
        lo = lo.min(pred);
        hi = hi.max(pred);
        sum += pred;
        n_below += usize::from(r["zeta"] < pred);
        diff += r["zeta"] - pred;
        each.push(format!(
            "({:.0},{:.0},{:.0}) km：コスモス {:+.4}、見込み {pred:+.4}",
            r["r1_km"], r["r2_km"], r["r3_km"], r["zeta"]
        ));
        writeln!(
            out,
            "{:.0},{:.0},{:.0},{:.3},{:.3},{:.3},{:.6},{pred:.6}",
            r["k1"], r["k2"], r["k3"], r["r1_km"], r["r2_km"], r["r3_km"], r["zeta"]
        )?;
    }
    results::write_text(
        &root.join(format!("data/zeta/mismatch_pred_cosmos{suffix}.csv")),
        &out,
    )?;
    writeln!(
        log,
        "コスモスの ξ̂ での見込み：ビンごとに {lo:+.4}〜{hi:+.4}、平均 {:+.4}（{} ビン）",
        sum / cz.len() as f64,
        cz.len()
    )?;
    writeln!(
        log,
        "コスモスの ζ̂ が見込みより小さいビン {n_below}/{}、差の平均 {:+.4}",
        cz.len(),
        diff / cz.len() as f64
    )?;
    for e in each {
        writeln!(log, "  {e}")?;
    }
    Ok(log)
}

/// ζ̂ = 分子 ÷ 分母 の、比の平均と平均の比を比べる（#94）。`zeta_field --counts on` の出力を読む。
///
/// run ごとの分子 N と分母 D（どちらも組の数で割ったもの）から、ビンごとに
/// - 比の平均 E[N/D] とその平均の誤差
/// - 平均の比 E[N]/E[D] と、その jackknife（run を一つずつ抜く）の誤差
/// - 比の偏りの一次の見積もり −Cov(N, D)/E[D]² + E[N] Var(D)/E[D]³
///
/// を求め、data/zeta/ratio_`label`.csv に書く。基準は true と proc の両方。
pub fn ratio_decomposition(
    root: &Path,
    label: &str,
    file: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let rows: Vec<_> = results::read_numeric_csv(&root.join(file))?
        .into_iter()
        .filter(|r| r["run"] >= 0.0)
        .collect();
    if !rows.first().is_some_and(|r| r.contains_key("num_proc")) {
        return Err(format!("{file} に num_proc の列がない（--counts on で作る）").into());
    }
    let keys: std::collections::BTreeSet<(usize, usize, usize)> = rows
        .iter()
        .map(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize))
        .collect();
    let mut out = String::from(
        "k1,k2,k3,r1_km,r2_km,r3_km,baseline,runs,mean_ratio,mean_ratio_sem,ratio_of_means,ratio_of_means_se,ratio_bias_pred,d_cv\n",
    );
    let mut log = String::new();
    // 規格化を割り戻した分子（--counts on の新しい出力）があれば、正しい基準の RRR で割ったものも比べる
    let mut bases = vec![
        ("true", "num_true", "rrr_true"),
        ("proc", "num_proc", "rrr_proc"),
    ];
    // 丸い塊の模擬データ（zeta_clustered --match on --counts on）は、親ごとに子を一つ選ぶ基準も
    if rows[0].contains_key("num_match") {
        bases.push(("match", "num_match", "rrr_match"));
    }
    if rows[0].contains_key("num_proc_icf") {
        bases.push(("true_icf", "num_true_icf", "rrr_true"));
        bases.push(("proc_icf", "num_proc_icf", "rrr_true"));
    }
    for (base, cn, cd) in bases {
        let (cn, cd) = (cn.to_string(), cd.to_string());
        let (mut sum_mr, mut sum_rm, mut sum_pred) = (0.0, 0.0, 0.0);
        let (mut n_rm_sig, mut n_mr_sig, mut n_pred_match) = (0, 0, 0);
        for &(a, b, c) in &keys {
            let v: Vec<_> = rows
                .iter()
                .filter(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize) == (a, b, c))
                .collect();
            let n: Vec<f64> = v.iter().map(|r| r[&cn]).collect();
            let d: Vec<f64> = v.iter().map(|r| r[&cd]).collect();
            let m = n.len() as f64;
            let mean = |x: &[f64]| x.iter().sum::<f64>() / x.len() as f64;
            let ratio: Vec<f64> = n.iter().zip(&d).map(|(x, y)| x / y).collect();
            let mr = mean(&ratio);
            let mr_sem =
                (ratio.iter().map(|x| (x - mr).powi(2)).sum::<f64>() / (m - 1.0) / m).sqrt();
            let (en, ed) = (mean(&n), mean(&d));
            let rm = en / ed;
            // jackknife：run を一つずつ抜いた平均の比
            let (sn, sd): (f64, f64) = (n.iter().sum(), d.iter().sum());
            let jk: Vec<f64> = n.iter().zip(&d).map(|(x, y)| (sn - x) / (sd - y)).collect();
            let jm = mean(&jk);
            let rm_se = ((m - 1.0) / m * jk.iter().map(|x| (x - jm).powi(2)).sum::<f64>()).sqrt();
            let cov = n
                .iter()
                .zip(&d)
                .map(|(x, y)| (x - en) * (y - ed))
                .sum::<f64>()
                / (m - 1.0);
            let var_d = d.iter().map(|y| (y - ed).powi(2)).sum::<f64>() / (m - 1.0);
            let pred = -cov / (ed * ed) + en * var_d / ed.powi(3);
            let d_cv = var_d.sqrt() / ed;
            sum_mr += mr;
            sum_rm += rm;
            sum_pred += pred;
            n_rm_sig += usize::from(rm.abs() > 2.0 * rm_se);
            n_mr_sig += usize::from(mr.abs() > 2.0 * mr_sem);
            n_pred_match += usize::from(((mr - rm) - pred).abs() <= 2.0 * mr_sem);
            writeln!(
                out,
                "{a},{b},{c},{:.3},{:.3},{:.3},{base},{},{mr:.6},{mr_sem:.6},{rm:.6},{rm_se:.6},{pred:.6},{d_cv:.5}",
                v[0]["r1_km"],
                v[0]["r2_km"],
                v[0]["r3_km"],
                n.len()
            )?;
        }
        let k = keys.len() as f64;
        writeln!(
            log,
            "{label} {base}：16 ビンの平均で、比の平均 {:+.5}、平均の比 {:+.5}、比の偏りの見積もり {:+.5}。\
             比の平均が 2 SE を超えるビン {n_mr_sig}/{}、平均の比が 2 SE を超えるビン {n_rm_sig}/{}、\
             （比の平均 − 平均の比）が見積もりと 2 SE 以内 {n_pred_match}/{}",
            sum_mr / k,
            sum_rm / k,
            sum_pred / k,
            keys.len(),
            keys.len(),
            keys.len()
        )?;
    }
    results::write_text(&root.join(format!("data/zeta/ratio_{label}.csv")), &out)?;
    Ok(log)
}

/// 親の数と子の数を決めた丸い塊の模擬データで、$\hat\zeta$ が $\zeta_{\mathrm{NS}}$ から決まってずれる分の見込み（#94）。
///
/// 店の数 N を親 ⌈N/m⌉ 個の子に分ける（最後の親は余り）。二点が同じ親の子である割合を p、
/// 三点が同じ親の子である割合を q₃ とすると、密度が正しい基準（match）で
/// ξ̂ ≃ ξ_NS − p、ζ̂ − ζ_NS ≃ −(q₃/p) (ξ_NS,12 + ξ_NS,23 + ξ_NS,31) + 2 q₃（ξ_NS = ξ̂ + p）。
/// `stem` は zeta_clustered --match on の出力の前置き（`stem`.csv と `stem`_xi.csv）。
/// 書くもの：`stem`_ic.csv（ビンごとの実測 ζ̂_match − ζ_NS の平均と誤差、見込み）
pub fn clustered_ic(
    root: &Path,
    stem: &str,
    m: usize,
    stores_path: &str,
    chain: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let n = stores::read_selected(&root.join(stores_path))
        .map_err(|e| format!("{stores_path} がない：{e}"))?
        .into_iter()
        .filter(|(c, _)| c == chain)
        .count();
    if m < 3 || n < m {
        return Err("--children は 3 以上で、店の数以下".into());
    }
    let (c2, c3) = (
        |k: f64| k * (k - 1.0) / 2.0,
        |k: f64| k * (k - 1.0) * (k - 2.0) / 6.0,
    );
    let (full, rest) = (n / m, n % m);
    let pairs = full as f64 * c2(m as f64) + c2(rest as f64);
    let triples = full as f64 * c3(m as f64) + c3(rest as f64);
    let p = pairs / c2(n as f64);
    let q3 = triples / c3(n as f64);
    let xi = results::read_numeric_csv(&root.join(format!("{stem}_xi.csv")))?;
    let xi_mean = |k: f64| {
        let v: Vec<f64> = xi
            .iter()
            .filter(|r| r["run"] >= 0.0 && r["k"] == k)
            .map(|r| r["xi_match"])
            .collect();
        v.iter().sum::<f64>() / v.len() as f64
    };
    let rows: Vec<_> = results::read_numeric_csv(&root.join(format!("{stem}.csv")))?
        .into_iter()
        .filter(|r| {
            r["run"] >= 0.0 && r["zeta_match"].is_finite() && r["expected_match"].is_finite()
        })
        .collect();
    let keys: std::collections::BTreeSet<(usize, usize, usize)> = rows
        .iter()
        .map(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize))
        .collect();
    let mut out = String::from("k1,k2,k3,r1_km,r2_km,r3_km,runs,observed,observed_sem,predicted\n");
    let mut log = format!(
        "N = {n}、m = {m}：p = {p:.6}、q₃ = {q3:.3e}、q₃/p = {:.6}\n",
        q3 / p
    );
    let (mut so, mut sp, mut n_within, mut n_same_sign) = (0.0, 0.0, 0, 0);
    for &(a, b, c) in &keys {
        let v: Vec<f64> = rows
            .iter()
            .filter(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize) == (a, b, c))
            .map(|r| r["zeta_match"] - r["expected_match"])
            .collect();
        let r0 = rows
            .iter()
            .find(|r| (r["k1"] as usize, r["k2"] as usize, r["k3"] as usize) == (a, b, c))
            .unwrap();
        let k = v.len() as f64;
        let obs = v.iter().sum::<f64>() / k;
        let sem = (v.iter().map(|x| (x - obs).powi(2)).sum::<f64>() / (k - 1.0) / k).sqrt();
        let sum_xi_ns = [a, b, c]
            .iter()
            .map(|&j| xi_mean(j as f64) + p)
            .sum::<f64>();
        let pred = -(q3 / p) * sum_xi_ns + 2.0 * q3;
        so += obs;
        sp += pred;
        n_within += usize::from((obs - pred).abs() <= 2.0 * sem);
        n_same_sign += usize::from(obs.signum() == pred.signum());
        writeln!(
            out,
            "{a},{b},{c},{:.3},{:.3},{:.3},{},{obs:.6},{sem:.6},{pred:.6}",
            r0["r1_km"],
            r0["r2_km"],
            r0["r3_km"],
            v.len()
        )?;
        writeln!(
            log,
            "({:.0},{:.0},{:.0}) km：実測 {obs:+.4} ± {sem:.4}、見込み {pred:+.4}",
            r0["r1_km"], r0["r2_km"], r0["r3_km"]
        )?;
    }
    let nb = keys.len();
    writeln!(
        log,
        "{} ビンの平均で、実測 {:+.5}、見込み {:+.5}。2 SE 以内 {n_within}/{nb}、符号が同じ {n_same_sign}/{nb}",
        nb,
        so / nb as f64,
        sp / nb as f64
    )?;
    results::write_text(&root.join(format!("{stem}_ic.csv")), &out)?;
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_stats_recovers_exact_line() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y: Vec<f64> = x.iter().map(|a| 0.5 * a - 0.1).collect();
        let s = paired_stats(&x, &y);
        assert!((s.slope - 0.5).abs() < 1e-12);
        assert!((s.intercept + 0.1).abs() < 1e-12);
        assert!(s.slope_se < 1e-12);
        assert!((s.diff_mean - (2.0 - 0.9)).abs() < 1e-12);
    }

    #[test]
    fn field_analysis_merges_disjoint_runs_and_rejects_duplicate_or_incomplete_inputs() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("field-analysis-{}-{id}", std::process::id()));
        results::write_text(&root.join("real.csv"),
            "k1,k2,k3,r1_km,r2_km,r3_km,ddd,ddr,drr,rrr,zeta\n0,0,0,3,3,3,1,1,1,1,-0.02\n1,1,1,6,6,6,1,1,1,1,0.01\n").unwrap();
        for (name, range) in [("a", 0..10), ("b", 10..20)] {
            let mut z =
                String::from("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected\n");
            let mut xi = String::from("run,k,r_km,xi_true,xi_proc\n-1,0,3,0.2,\n-1,1,6,0.1,\n");
            let mut diagnostics = String::from("run,clipped_fraction\n");
            for run in range {
                writeln!(diagnostics, "{run},0.1").unwrap();
                let vals = [
                    (run as f64 - 10.0) * 0.01,
                    ((run * 7 % 11) as f64 - 5.0) * 0.02,
                ];
                for (k, value) in vals.iter().enumerate() {
                    let r = 3 * (k + 1);
                    writeln!(
                        z,
                        "{run},{k},{k},{k},{r},{r},{r},{},{value},0",
                        value + 0.03
                    )
                    .unwrap();
                    writeln!(xi, "{run},{k},{r},0.2,0.1").unwrap();
                }
            }
            results::write_text(&root.join(format!("{name}.csv")), &z).unwrap();
            results::write_text(&root.join(format!("{name}_xi.csv")), &xi).unwrap();
            results::write_text(&root.join(format!("{name}_diagnostics.csv")), &diagnostics)
                .unwrap();
        }
        let sources = ["a".to_string(), "b".to_string()];
        analyze_field(&root, "real.csv", "combined", &sources).unwrap();
        let tests = results::read_numeric_csv(&root.join("combined_tests.csv")).unwrap();
        assert_eq!((tests[0]["runs"], tests[0]["bins"]), (20.0, 2.0));
        assert!((tests[0]["hartlap"] - 16.0 / 19.0).abs() < 1e-9);
        assert!(tests[0]["chi2_hartlap"].is_finite());
        let true_summary =
            results::read_numeric_csv(&root.join("combined_true_summary.csv")).unwrap();
        assert!(
            true_summary
                .iter()
                .all(|r| r["true_sem"].is_finite() && r["true_sem"] > 0.0)
        );
        let clipping = results::read_numeric_csv(&root.join("combined_diagnostics.csv")).unwrap();
        assert_eq!(clipping.len(), 20);
        assert!(clipping.iter().all(|r| r["clipped_fraction"] == 0.1));
        assert!(
            analyze_field(
                &root,
                "real.csv",
                "duplicate",
                &["a".to_string(), "a".to_string()]
            )
            .is_err()
        );
        // 消えた ξ の run は、三点のベクトルが完全でも結合を拒む。
        results::write_text(
            &root.join("b_xi.csv"),
            "run,k,r_km,xi_true,xi_proc\n-1,0,3,0.2,\n-1,1,6,0.1,\n",
        )
        .unwrap();
        assert!(analyze_field(&root, "real.csv", "missing", &sources).is_err());
        assert!(!root.join("missing.csv").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod combined_reference_tests {
    use super::*;

    fn project_csv(text: &str, names: &[&str]) -> String {
        let mut lines = text.lines();
        let header: Vec<_> = lines.next().unwrap().split(',').collect();
        let columns: Vec<_> = names
            .iter()
            .map(|name| header.iter().position(|h| h == name).unwrap())
            .collect();
        let mut projected = names.join(",");
        projected.push('\n');
        for line in lines {
            let values: Vec<_> = line.split(',').collect();
            projected.push_str(
                &columns
                    .iter()
                    .map(|&i| values[i])
                    .collect::<Vec<_>>()
                    .join(","),
            );
            projected.push('\n');
        }
        projected
    }

    #[test]
    fn combined_references_preserve_each_standalone_catalog_and_legacy_columns() {
        // 小さな実ファイルを通し、4通りの設定で乱数列と出力列の独立性を確かめる。
        let root = std::env::temp_dir().join(format!(
            "retail-combined-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let popdir = root.join("data/estat");
        let prefdir = popdir.join("mesh_pref");
        std::fs::create_dir_all(&prefdir).unwrap();
        let codes = [50303500u64, 50303501, 50303502];
        let mut pop = String::from("header\nheader\n");
        for code in codes {
            writeln!(pop, "{code},0,0,0,1000").unwrap();
        }
        std::fs::write(popdir.join("tblT001140S5030.txt"), pop).unwrap();
        for pref in 1..=47 {
            let mut rows = String::from("header\n");
            if pref == 40 {
                for code in codes {
                    writeln!(rows, "40000,city,{code}").unwrap();
                }
            }
            std::fs::write(prefdir.join(format!("{pref:02}.csv")), rows).unwrap();
        }
        let mut stores = String::from("chain,region,state,lat,lon,kind\n");
        for code in codes {
            let (lat, lon) = crate::util::grid::code_to_sw(code);
            for i in 0..4 {
                let f = 0.1 + i as f64 * 0.2;
                writeln!(
                    stores,
                    "COSMOS,九州,JP-40,{},{},site",
                    lat + f / 120.0,
                    lon + f / 80.0
                )
                .unwrap();
            }
        }
        std::fs::write(root.join("stores.csv"), stores).unwrap();
        let mut outputs = Vec::new();
        for (name, regional, matched) in [
            ("base", None, false),
            ("regional", Some(RegionalScale::Prefecture), false),
            ("matched", None, true),
            ("both", Some(RegionalScale::Prefecture), true),
        ] {
            let mut p = Params::new("fixture".into());
            p.stores = "stores.csv".into();
            p.chain = "COSMOS".into();
            p.n_random = 64;
            p.n_ref = 128;
            p.footprint = Some(50.0);
            p.regional = regional;
            p.match_density = matched;
            p.sides_km = (0.1, 8.0, 1);
            p.tag = format!("_{name}");
            let path = clustered_mocks(&root, &p, 3, 1.0, 1).unwrap();
            outputs.push((
                std::fs::read_to_string(root.join(&path)).unwrap(),
                std::fs::read_to_string(root.join(path.replace(".csv", "_xi.csv"))).unwrap(),
            ));
        }
        for reference in &outputs[..3] {
            for (expected, actual) in [(&reference.0, &outputs[3].0), (&reference.1, &outputs[3].1)]
            {
                let names: Vec<_> = expected.lines().next().unwrap().split(',').collect();
                assert_eq!(project_csv(actual, &names), *expected);
            }
        }
        assert!(outputs[3].0.starts_with("run,k1,k2,k3,r1_km,r2_km,r3_km,zeta_true,zeta_proc,expected,zeta_regional,zeta_match,expected_match\n"));
        assert!(
            outputs[3]
                .1
                .starts_with("run,k,r_km,xi_true,xi_proc,xi_regional,xi_match\n")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

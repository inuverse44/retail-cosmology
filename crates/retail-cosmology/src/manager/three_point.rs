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
use crate::contract::{Bins, PopulationGrid, TripleCounts};
use crate::engine::estimator::mock_summary;
use crate::engine::random_catalog::{Weight, sample, sample_selected};
use crate::engine::selection_function::{
    SmoothRatio, Tabulated, fit_density_exponent, mean_log_density,
};
use crate::engine::triple_count::TriangleBins;
use crate::util::geo::{LatLon, Unit, km_to_chord2};
use crate::util::rng::SplitMix64;

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
    /// 選択関数をならす幅 R [km]。None なら人口だけに比例するランダムカタログ
    pub footprint: Option<f64>,
    /// 選択関数を約 10 km の粗い格子ごとに一度だけ求める近似を使う
    pub selection_grid: bool,
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
            mock: None,
            mock_density: false,
            sides_km: (2.0, 32.0, 4),
            tag: String::new(),
        }
    }
}

/// 国ごとの人口の格子と、出力の名前の前置き。
fn grid_of(root: &Path, country: &str) -> Result<(PopulationGrid, &'static str), Box<dyn std::error::Error>> {
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
    let selection = |u: &[Unit]| p.footprint.map(|r| SmoothRatio::new(u, &refs, r));
    // 選択関数の値。selection_grid なら約 10 km の格子の表で引く（表は使う所で作る）
    let draw = |sel: &Option<SmoothRatio>, n: usize, rng: &mut SplitMix64| -> Vec<LatLon> {
        match sel {
            Some(sr) if p.selection_grid => {
                let tab = Tabulated::new(sr, &grid.geometry);
                sample_selected(&grid, n, Weight::Population, |q| tab.eval(q), 2 * n, rng).points
            }
            Some(sr) => {
                sample_selected(&grid, n, Weight::Population, |q| sr.eval(&Unit::from(q)), 2 * n, rng)
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
        _ => 1.0,
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
            draw(&sel_real, pts.len(), &mut rng_m).iter().map(Unit::from).collect()
        }
    };
    let mut rng = SplitMix64::new(p.seed);
    let randoms: Vec<Unit> = if p.mock.is_some() {
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
        p.mock.map_or(String::new(), |s| format!("（模擬データ、種 {s}）")),
        randoms.len(),
        p.footprint.map_or(String::new(), |r| format!("、選択関数 R = {r} km")),
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
    eprintln!("{path} を書いた（全体 {:.1} 秒）", t0.elapsed().as_secs_f64());
    Ok(path)
}

/// 実際の店の ζ（`real`）を、帰無仮説の模擬データの ζ（`mocks`）とビンごとに比べた表を、文字列で返す。
/// 列：辺のビンの中点 [km]、DDD、実際の ζ、模擬データの平均と標準偏差、平均から何 σ、実際以上・以下の個数。
pub fn compare_mocks(root: &Path, real: &str, mocks: &[String]) -> Result<String, Box<dyn std::error::Error>> {
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

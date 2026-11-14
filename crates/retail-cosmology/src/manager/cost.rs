//! 診断：模擬データ 1 個の計算量を見積もるための数を測る（notes/algorithm-cost.tex）。
//!
//! 測るもの：人が住む 1 km 格子と 10 km 格子（2 次メッシュ）の数、選択関数 S を 1 回求めるときに調べる点の数と
//! 足し合わせる点の数、1 回あたりの時間、棄却法で候補が残る割合、2 点の距離を 1 回計算してビンに入れる時間。

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use crate::access::{population, results, stores};
use crate::contract::Bins;
use crate::engine::pair_count::count_auto;
use crate::engine::random_catalog::{Weight, sample};
use crate::engine::selection_function::{SmoothRatio, Tabulated};
use crate::util::geo::LatLon;
use crate::util::geo::{Unit, chord2, km_to_chord2};
use crate::util::grid::GridGeometry;
use crate::util::rng::SplitMix64;

/// この診断で用いる参照点の数（correlation の既定と同じ設定）。
const N_REF: usize = 50_000;

/// 測った数を key,value の CSV（data/cost/cost_R<R>.csv）に書き、要約の文字列を返す。
/// `store_files` はチェーンごとの店のファイル（リポジトリの根からの相対パス）。
pub fn run(
    root: &Path,
    store_files: &[String],
    r_km: f64,
    n_probe: usize,
    seed: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let grid = population::read_estat(&root.join("data/estat"))?;
    let mut out = String::from("key,value\n");
    let mut put = |k: &str, v: f64| writeln!(out, "{k},{v}").unwrap();

    let n1 = grid.values.iter().filter(|v| **v > 0.0).count();
    let n10 = match &grid.geometry {
        GridGeometry::Mesh { index, .. } => index
            .iter()
            .filter(|(_, i)| grid.values[**i] > 0.0)
            .map(|(code, _)| code / 100)
            .collect::<HashSet<u64>>()
            .len(),
        GridGeometry::Planar { .. } => 0,
    };
    put("cells_1km", n1 as f64);
    put("cells_10km", n10 as f64);
    put("r_km", r_km);
    put("n_ref", N_REF as f64);
    put("n_probe", n_probe as f64);

    let mut rng = SplitMix64::new(seed);
    let refs: Vec<Unit> = sample(&grid, N_REF, Weight::Population, &mut rng)
        .iter()
        .map(Unit::from)
        .collect();
    let probes_ll: Vec<LatLon> = sample(&grid, n_probe, Weight::Population, &mut rng);
    let probes: Vec<Unit> = probes_ll.iter().map(Unit::from).collect();

    let mut report = format!(
        "人が住む 1 km 格子 {n1}、10 km 格子 {n10}。参照の点 {N_REF}、人口に比例した試しの点 {n_probe}、R = {r_km} km\n"
    );
    for path in store_files {
        let st = stores::read_selected(&root.join(path))?;
        let chain = st.first().map(|(c, _)| c.clone()).unwrap_or_default();
        let units: Vec<Unit> = st.iter().map(|(_, p)| Unit::from(p)).collect();
        let t = Instant::now();
        let sr = SmoothRatio::new(&units, &refs, r_km);
        let build_s = t.elapsed().as_secs_f64();
        let (mut vis, mut ins) = (0usize, 0usize);
        for p in &probes {
            let (v, i) = sr.eval_cost(p);
            vis += v;
            ins += i;
        }
        let t = Instant::now();
        let s: Vec<f64> = probes.iter().map(|p| sr.eval(p)).collect();
        let per_eval = t.elapsed().as_secs_f64() / n_probe as f64;
        let mean_s = s.iter().sum::<f64>() / n_probe as f64;
        let max_s = s.iter().copied().fold(0.0, f64::max);
        let acc = mean_s / max_s;
        let pre = format!("{chain}_");
        put(&format!("{pre}n_stores"), units.len() as f64);
        put(&format!("{pre}visited_mean"), vis as f64 / n_probe as f64);
        put(&format!("{pre}inside_mean"), ins as f64 / n_probe as f64);
        put(&format!("{pre}sec_per_eval"), per_eval);
        put(&format!("{pre}acceptance"), acc);
        put(&format!("{pre}build_sec"), build_s);

        // 粗い格子（約 10 km）で表にした S と、厳密な S の差（#12）
        let tab = Tabulated::new(&sr, &grid.geometry);
        let t = Instant::now();
        let st: Vec<f64> = probes_ll.iter().map(|q| tab.eval(q)).collect();
        let tab_s = t.elapsed().as_secs_f64();
        let mut rel: Vec<f64> = s
            .iter()
            .zip(&st)
            .filter(|(e, _)| **e > 0.0)
            .map(|(e, a)| (a - e).abs() / e)
            .collect();
        rel.sort_by(f64::total_cmp);
        let q = |f: f64| rel[((rel.len() - 1) as f64 * f).round() as usize];
        let abs_max = s
            .iter()
            .zip(&st)
            .map(|(e, a)| (a - e).abs())
            .fold(0.0, f64::max)
            / max_s;
        put(&format!("{pre}tab_cells"), tab.n_cells() as f64);
        put(&format!("{pre}tab_sec"), tab_s);
        put(&format!("{pre}tab_rel_median"), q(0.5));
        put(&format!("{pre}tab_rel_p95"), q(0.95));
        put(&format!("{pre}tab_rel_max"), q(1.0));
        put(&format!("{pre}tab_abs_max_over_smax"), abs_max);
        writeln!(
            report,
            "{chain}：10 km 格子の表：S を求めた格子 {}、時間 {:.2} 秒（厳密 {:.2} 秒）。相対誤差 中央値 {:.4}、95% 点 {:.4}、最大 {:.4}。誤差の最大 ÷ S_max {:.4}",
            tab.n_cells(),
            tab_s,
            per_eval * n_probe as f64,
            q(0.5),
            q(0.95),
            q(1.0),
            abs_max
        )?;
        writeln!(
            report,
            "{chain}（{} 店）：S を 1 回求めるとき、調べる点 平均 {:.0}、足し合わせる点 平均 {:.0}、1 回 {:.3} ミリ秒。残る割合 {:.3}（2 万点に要る候補 {:.0}）",
            units.len(),
            vis as f64 / n_probe as f64,
            ins as f64 / n_probe as f64,
            per_eval * 1e3,
            acc,
            20_000.0 / acc
        )?;
    }

    // 2 点の距離を 1 回計算してビンに入れる時間（RR と同じ数え方）
    let n_r = 5_000;
    let pts: Vec<Unit> = sample(&grid, n_r, Weight::Population, &mut rng)
        .iter()
        .map(Unit::from)
        .collect();
    let bins = Bins::log(2.0, 2000.0, 15).map(km_to_chord2);
    let t = Instant::now();
    let c = count_auto(&pts, &bins, chord2);
    let pairs = (n_r * (n_r - 1) / 2) as f64;
    let per_pair = t.elapsed().as_secs_f64() / pairs;
    put("sec_per_pair", per_pair);
    writeln!(
        report,
        "距離を 1 回計算してビンに入れる時間 {:.2} ナノ秒（{n_r} 点の中の {pairs:.0} 組、ビンに入った組 {}）",
        per_pair * 1e9,
        c.iter().sum::<u64>()
    )?;
    results::write_text(&root.join(format!("data/cost/cost_R{r_km}.csv")), &out)?;
    Ok(report)
}

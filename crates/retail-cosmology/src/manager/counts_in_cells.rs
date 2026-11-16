//! Manager：店を約 10 km の粗い格子（日本では 2 次メッシュ）に数え、店と同じ手順で作った期待数からの
//! ずれの統計を、コスモスと密度場の模擬データで比べる（ADR 011）。
//!
//! 期待数 λ_c は、カタログ（実データか模擬データ一つ）から幅 R_W の選択関数 S を作り、
//! λ_c = N × 人口_c S(c) / Σ 人口 S とする（人口_c はマスの中の人口の格子の和、S はマスの中心で求める）。
//! 統計は `engine::counts_in_cells`。模擬データの店の位置は `zeta_field --save-points on` が書いたもの。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::access::{population, results, stores};
use crate::engine::counts_in_cells::{CellStats, cell_stats};
use crate::engine::random_catalog::{Weight, sample};
use crate::engine::selection_function::SmoothRatio;
use crate::util::geo::{LatLon, Unit};
use crate::util::grid::GridGeometry;
use crate::util::rng::SplitMix64;

/// 比べ方の設定。
pub struct CellParams<'a> {
    /// コスモスの店のファイルとチェーン名
    pub stores: &'a str,
    pub chain: &'a str,
    /// 期待数を作る選択関数の幅 R_W [km]（模擬データの --footprint と同じにする）
    pub r_w_km: f64,
    /// 人口の参照の点の数と、それを撒く乱数の種（全カタログで同じ参照の点を使う）
    pub n_ref: usize,
    pub seed: u64,
    /// 統計に使うマスの λ の下限（複数）。表には最初の値を使う
    pub lambda_mins: &'a [f64],
}

/// 粗い格子ごとの人口の和と中心。
struct CoarseCells {
    pop: BTreeMap<u64, f64>,
    center: BTreeMap<u64, Unit>,
}

impl CoarseCells {
    fn new(grid: &crate::contract::PopulationGrid) -> Self {
        let mut pop = BTreeMap::new();
        let mut center = BTreeMap::new();
        for (i, &v) in grid.values.iter().enumerate() {
            let w = Weight::Population.of(v as f64);
            if w <= 0.0 {
                continue;
            }
            // 人口の格子（3 次メッシュ）は粗い格子（2 次メッシュ）の中にちょうど入るので、中心で決めてよい
            if let Some((key, c)) = grid.geometry.coarse_cell(&grid.geometry.point(i, 0.5, 0.5)) {
                *pop.entry(key).or_insert(0.0) += w;
                center.entry(key).or_insert_with(|| Unit::from(&c));
            }
        }
        Self { pop, center }
    }

    /// カタログ `pts` の（N_c, λ_c）。λ_c > 0 のマスをすべて返す。
    fn counts(
        &self,
        geometry: &GridGeometry,
        pts: &[LatLon],
        refs: &[Unit],
        r_w_km: f64,
    ) -> Vec<(u64, f64)> {
        let units: Vec<Unit> = pts.iter().map(Unit::from).collect();
        let sr = SmoothRatio::new(&units, refs, r_w_km);
        let w: BTreeMap<u64, f64> = self
            .pop
            .iter()
            .map(|(k, p)| (*k, p * sr.eval(&self.center[k])))
            .collect();
        let total: f64 = w.values().sum();
        let mut n: BTreeMap<u64, u64> = BTreeMap::new();
        for q in pts {
            if let Some((key, _)) = geometry.coarse_cell(q) {
                *n.entry(key).or_insert(0) += 1;
            }
        }
        let scale = pts.len() as f64 / total;
        w.iter()
            .filter(|(_, wc)| **wc > 0.0)
            .map(|(k, wc)| (n.get(k).copied().unwrap_or(0), wc * scale))
            .collect()
    }
}

/// `{stem}_points.csv`（run,lat,lon）を run ごとに読む。
fn read_points(root: &Path, stem: &str) -> Result<BTreeMap<i64, Vec<LatLon>>, String> {
    let path = root.join(format!("{stem}_points.csv"));
    let rows = results::read_numeric_csv(&path)
        .map_err(|e| format!("{} がない（zeta_field --save-points on で作る）：{e}", path.display()))?;
    let mut out: BTreeMap<i64, Vec<LatLon>> = BTreeMap::new();
    for r in rows {
        out.entry(r["run"] as i64).or_default().push(LatLon {
            lat: r["lat"],
            lon: r["lon"],
        });
    }
    Ok(out)
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    (m, sd)
}

/// 統計の名前（ログ）、表の見出し、取り出し方。
type Stat = (&'static str, &'static str, fn(&CellStats) -> f64);

/// 表の列の順に並べた統計。
const STATS: [Stat; 4] = [
    ("kappa2", "$\\hat\\kappa_2$", |s| s.kappa2),
    ("kappa3", "$\\hat\\kappa_3$", |s| s.kappa3),
    ("void_ratio", "$V$", |s| s.void_ratio()),
    ("high_ratio", "$H$", |s| s.high_ratio()),
];

/// コスモスと模擬データ（`models` は（表の名前、`_points.csv` の前の stem））の統計を比べる。
/// 書くもの：data/zeta/cells_stats.csv（カタログ・run・λ_min ごと、コスモスは model = COSMOS、run = -1）と
/// docs/tables/cells_stats.tex（最初の λ_min、模擬データは平均 ± 標準偏差、コスモスとの差は標準偏差の何倍か）。
pub fn cells_table(
    root: &Path,
    p: &CellParams,
    models: &[(&str, &str)],
) -> Result<String, Box<dyn std::error::Error>> {
    if p.lambda_mins.is_empty() {
        return Err("λ_min を一つ以上渡す".into());
    }
    let grid = population::read_estat(&root.join("data/estat"))?;
    let coarse = CoarseCells::new(&grid);
    let mut rng_ref = SplitMix64::new(p.seed ^ 0xCE11_0000_0000_0000);
    let refs: Vec<Unit> = sample(&grid, p.n_ref, Weight::Population, &mut rng_ref)
        .iter()
        .map(Unit::from)
        .collect();
    let cosmos: Vec<LatLon> = stores::read_selected(&root.join(p.stores))
        .map_err(|e| format!("{} がない：{e}", p.stores))?
        .into_iter()
        .filter(|(c, _)| c == p.chain)
        .map(|(_, q)| q)
        .collect();
    if cosmos.is_empty() {
        return Err(format!("{} に {} の店がない", p.stores, p.chain).into());
    }

    let mut log = String::new();
    let mut csv = String::from(
        "model,run,lambda_min,cells,n_sum,lambda_sum,kappa2,kappa3,empty,empty_poisson,high,high_poisson\n",
    );
    let row = |csv: &mut String, name: &str, run: i64, lmin: f64, s: &CellStats| {
        writeln!(
            csv,
            "{},{run},{lmin},{},{},{:.4},{:.6},{:.6},{},{:.4},{},{:.4}",
            if name.contains(',') {
                format!("\"{name}\"")
            } else {
                name.to_string()
            },
            s.cells,
            s.n_sum,
            s.lambda_sum,
            s.kappa2,
            s.kappa3,
            s.empty,
            s.empty_poisson,
            s.high,
            s.high_poisson
        )
    };

    let c_cells = coarse.counts(&grid.geometry, &cosmos, &refs, p.r_w_km);
    let c_stats: Vec<CellStats> = p
        .lambda_mins
        .iter()
        .map(|&l| cell_stats(&c_cells, l))
        .collect();
    for (&lmin, s) in p.lambda_mins.iter().zip(&c_stats) {
        row(&mut csv, "COSMOS", -1, lmin, s)?;
        writeln!(
            log,
            "COSMOS λ ≥ {lmin}：マス {}（店 {}、λ の和 {:.1}）、κ̂₂ {:+.4}、κ̂₃ {:+.4}、V {:.3}（{} / {:.1}）、H {:.3}（{} / {:.1}）",
            s.cells,
            s.n_sum,
            s.lambda_sum,
            s.kappa2,
            s.kappa3,
            s.void_ratio(),
            s.empty,
            s.empty_poisson,
            s.high_ratio(),
            s.high,
            s.high_poisson
        )?;
    }

    // 表：行は模型、列は四つの統計（最初の λ_min）
    let lmin0 = p.lambda_mins[0];
    let mut t = format!(
        "% cargo run --release --bin cells_stats が書いた表（λ ≥ {lmin0}）\n\
         \\begin{{tabular}}{{l{}}}\n\\toprule\n& {} \\\\\n\\midrule\n",
        "r".repeat(STATS.len()),
        STATS.iter().map(|s| s.1).collect::<Vec<_>>().join(" & ")
    );
    writeln!(
        t,
        "コスモス & {} \\\\\n\\midrule",
        STATS
            .iter()
            .map(|(_, _, f)| format!("${:.3}$", f(&c_stats[0])))
            .collect::<Vec<_>>()
            .join(" & ")
    )?;

    for (name, stem) in models {
        let runs = read_points(root, stem)?;
        let mut per_lmin: Vec<Vec<CellStats>> = vec![Vec::new(); p.lambda_mins.len()];
        for (run, pts) in &runs {
            let cells = coarse.counts(&grid.geometry, pts, &refs, p.r_w_km);
            for (j, &lmin) in p.lambda_mins.iter().enumerate() {
                let s = cell_stats(&cells, lmin);
                row(&mut csv, name, *run, lmin, &s)?;
                per_lmin[j].push(s);
            }
        }
        for (j, &lmin) in p.lambda_mins.iter().enumerate() {
            write!(log, "{name} λ ≥ {lmin}（{} 回）：", per_lmin[j].len())?;
            let mut cells_tex = Vec::new();
            for (key, _, f) in STATS {
                let v: Vec<f64> = per_lmin[j].iter().map(f).collect();
                let (m, sd) = mean_sd(&v);
                let c = f(&c_stats[j]);
                let below = v.iter().filter(|x| **x <= c).count();
                write!(
                    log,
                    "{key} {m:+.4} ± {sd:.4}（コスモス {c:+.4}、以下 {below}/{}、{:+.2}σ）　",
                    v.len(),
                    (c - m) / sd
                )?;
                if j == 0 {
                    // 小数 1 桁に丸めてから 0.0 を足し、−0.0 を +0.0 にする（IEEE 754 で −0 + 0 = +0）
                    let z = ((c - m) / sd * 10.0).round() / 10.0 + 0.0;
                    cells_tex.push(format!("${m:.3} \\pm {sd:.3}$ (${z:+.1}$)"));
                }
            }
            writeln!(log)?;
            if j == 0 {
                writeln!(t, "{name} & {} \\\\", cells_tex.join(" & "))?;
            }
        }
    }
    t.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/cells_stats.tex"), &t)?;
    results::write_text(&root.join("data/zeta/cells_stats.csv"), &csv)?;
    Ok(log)
}

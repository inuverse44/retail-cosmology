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
use crate::engine::census::{nearest_other_km, quantile};
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
    /// マスの面積 [km²]
    area: BTreeMap<u64, f64>,
}

impl CoarseCells {
    fn new(grid: &crate::contract::PopulationGrid) -> Self {
        let mut pop = BTreeMap::new();
        let mut center = BTreeMap::new();
        let mut area = BTreeMap::new();
        for (i, &v) in grid.values.iter().enumerate() {
            let w = Weight::Population.of(v as f64);
            if w <= 0.0 {
                continue;
            }
            // 人口の格子（3 次メッシュ）は粗い格子（2 次メッシュ）の中にちょうど入るので、中心で決めてよい
            if let Some((key, c)) = grid.geometry.coarse_cell(&grid.geometry.point(i, 0.5, 0.5)) {
                *pop.entry(key).or_insert(0.0) += w;
                center.entry(key).or_insert_with(|| Unit::from(&c));
                area.entry(key)
                    .or_insert_with(|| grid.geometry.coarse_area_km2(&c));
            }
        }
        Self { pop, center, area }
    }

    /// カタログ `pts` の λ_c。λ_c > 0 のマスだけを、マスの番号の順に返す。
    fn lambdas(&self, pts: &[LatLon], refs: &[Unit], r_w_km: f64) -> BTreeMap<u64, f64> {
        let units: Vec<Unit> = pts.iter().map(Unit::from).collect();
        let sr = SmoothRatio::new(&units, refs, r_w_km);
        let w: BTreeMap<u64, f64> = self
            .pop
            .iter()
            .map(|(k, p)| (*k, p * sr.eval(&self.center[k])))
            .collect();
        let total: f64 = w.values().sum();
        let scale = pts.len() as f64 / total;
        w.into_iter()
            .filter(|(_, wc)| *wc > 0.0)
            .map(|(k, wc)| (k, wc * scale))
            .collect()
    }

    /// カタログ `pts` の（N_c, λ_c）。λ_c > 0 のマスをすべて返す。
    fn counts(
        &self,
        geometry: &GridGeometry,
        pts: &[LatLon],
        refs: &[Unit],
        r_w_km: f64,
    ) -> Vec<(u64, f64)> {
        let lam = self.lambdas(pts, refs, r_w_km);
        let mut n: BTreeMap<u64, u64> = BTreeMap::new();
        for q in pts {
            if let Some((key, _)) = geometry.coarse_cell(q) {
                *n.entry(key).or_insert(0) += 1;
            }
        }
        lam.iter()
            .map(|(k, l)| (n.get(k).copied().unwrap_or(0), *l))
            .collect()
    }

    /// 最近接距離の統計（名前、値）。d は自分以外で最も近い店までの距離 [km]、
    /// u = π n_c d²（n_c = λ_c / マスの面積、店のいるマスの期待密度）。期待密度のとおりのポアソン点なら
    /// u は平均 1 の指数分布に従う。λ_c のないマス（人口 0 か S = 0）にいる店は u と層から外し、その数を no_lambda に書く。
    fn nearest_stats(
        &self,
        geometry: &GridGeometry,
        pts: &[LatLon],
        refs: &[Unit],
        r_w_km: f64,
    ) -> Vec<(String, f64)> {
        let lam = self.lambdas(pts, refs, r_w_km);
        let d = nearest_other_km(pts);
        let mut out: Vec<(String, f64)> = Vec::new();
        let frac = |v: &[f64], x: f64| v.iter().filter(|y| **y < x).count() as f64 / v.len() as f64;

        let mut dv = d.clone();
        for q in NN_QUANTILES {
            out.push((format!("d_q{:02}", (q * 100.0).round()), quantile(&mut dv, q)));
        }
        for x in NN_D_KM {
            out.push((format!("G_d<{x}"), frac(&d, x)));
        }

        // 店ごとの λ_c と u
        let mut u_all = Vec::new();
        let mut by_class: Vec<(Vec<f64>, Vec<f64>)> = vec![(Vec::new(), Vec::new()); NN_CLASSES.len()];
        let mut no_lambda = 0usize;
        for (q, &di) in pts.iter().zip(&d) {
            let Some((key, _)) = geometry.coarse_cell(q) else {
                no_lambda += 1;
                continue;
            };
            let (Some(&l), Some(&a)) = (lam.get(&key), self.area.get(&key)) else {
                no_lambda += 1;
                continue;
            };
            let u = std::f64::consts::PI * l / a * di * di;
            u_all.push(u);
            if let Some(j) = NN_CLASSES.iter().position(|(lo, hi, _)| l >= *lo && l < *hi) {
                by_class[j].0.push(di);
                by_class[j].1.push(u);
            }
        }
        out.push(("no_lambda".into(), no_lambda as f64));
        let mut uv = u_all.clone();
        out.push(("u_q50".into(), quantile(&mut uv, 0.5)));
        for x in NN_U {
            out.push((format!("F_u<{x}"), frac(&u_all, x)));
        }
        for (&(lo, hi, name), (mut dc, mut uc)) in NN_CLASSES.iter().zip(by_class) {
            out.push((format!("{name}_n"), dc.len() as f64));
            // 層に入るマスの λ_c の和（店の数 n がこれと比べて多いか少ないか）
            let lam_sum: f64 = lam.values().filter(|l| **l >= lo && **l < hi).sum();
            out.push((format!("{name}_lam_sum"), lam_sum));
            // 層の中で、d < 1 km の割合と u < 0.1 の割合（避け合う距離が km で一定か、密度で縮むかを見る）
            out.push((format!("{name}_G_d<1"), frac(&dc, 1.0)));
            out.push((format!("{name}_F_u<0.1"), frac(&uc, 0.1)));
            // 層に店がなければ NaN
            let med = |v: &mut Vec<f64>| {
                if v.is_empty() {
                    f64::NAN
                } else {
                    quantile(v, 0.5)
                }
            };
            out.push((format!("{name}_d_q50"), med(&mut dc)));
            out.push((format!("{name}_u_q50"), med(&mut uc)));
        }
        out
    }
}

/// 最近接距離の表に入れる統計（`nearest_stats` の名前、表の見出し）
const NN_TABLE: [(&str, &str); 4] = [
    ("G_d<0.5", "$G(0.5)$"),
    ("G_d<1", "$G(1)$"),
    ("d_q90", "$d_{90}$"),
    ("lam>=3_F_u<0.1", "$F_{\\ge 3}(0.1)$"),
];
/// 最近接距離 d の分位点
const NN_QUANTILES: [f64; 5] = [0.1, 0.25, 0.5, 0.75, 0.9];
/// d がこれより短い店の割合 G を求める距離 [km]
const NN_D_KM: [f64; 5] = [0.5, 1.0, 2.0, 3.0, 5.0];
/// u がこれより小さい店の割合 F を求める値（ポアソンなら 1 − e^{−x}）
const NN_U: [f64; 4] = [0.1, 0.25, 0.5, 1.0];
/// 店のいるマスの λ_c で分ける層（下限、上限、名前）
const NN_CLASSES: [(f64, f64, &str); 3] = [
    (0.0, 1.0, "lam<1"),
    (1.0, 3.0, "lam1-3"),
    (3.0, f64::INFINITY, "lam>=3"),
];

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

/// どの比べ方にも共通の準備：人口の格子、粗い格子、参照の点、コスモスの店。
struct Setup {
    grid: crate::contract::PopulationGrid,
    coarse: CoarseCells,
    refs: Vec<Unit>,
    cosmos: Vec<LatLon>,
}

fn setup(root: &Path, p: &CellParams) -> Result<Setup, Box<dyn std::error::Error>> {
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
    Ok(Setup {
        grid,
        coarse,
        refs,
        cosmos,
    })
}

/// コスモスと模擬データ（`models` は（ログの名前、`_points.csv` の前の stem））の最近接距離の分布を比べる。
/// 統計は `CoarseCells::nearest_stats`。`p.lambda_mins` は使わない。
/// 書くもの：data/zeta/nearest_stats.csv（model,run,stat,value、コスモスは model = COSMOS、run = -1）と
/// docs/tables/nearest_stats.tex（`NN_TABLE` の統計）。
/// ログには統計ごとに、コスモスの値と、模型ごとの平均 ± 標準偏差・コスモス以下の数・標準偏差の何倍か。
pub fn nearest_table(
    root: &Path,
    p: &CellParams,
    models: &[(&str, &str)],
) -> Result<String, Box<dyn std::error::Error>> {
    let Setup {
        grid,
        coarse,
        refs,
        cosmos,
    } = setup(root, p)?;
    let mut csv = String::from("model,run,stat,value\n");
    let c = coarse.nearest_stats(&grid.geometry, &cosmos, &refs, p.r_w_km);
    for (k, v) in &c {
        writeln!(csv, "COSMOS,-1,{k},{v}")?;
    }
    // 模型ごとに、統計ごとの値の並び
    let mut per_model: Vec<(String, Vec<Vec<f64>>)> = Vec::new();
    for (name, stem) in models {
        let runs = read_points(root, stem)?;
        let mut vals: Vec<Vec<f64>> = vec![Vec::new(); c.len()];
        for (run, pts) in &runs {
            let s = coarse.nearest_stats(&grid.geometry, pts, &refs, p.r_w_km);
            for (j, (k, v)) in s.iter().enumerate() {
                if *k != c[j].0 {
                    return Err(format!("統計の並びが違う：{k} と {}", c[j].0).into());
                }
                writeln!(
                    csv,
                    "{},{run},{k},{v}",
                    if name.contains(',') {
                        format!("\"{name}\"")
                    } else {
                        name.to_string()
                    }
                )?;
                vals[j].push(*v);
            }
        }
        per_model.push((name.to_string(), vals));
    }
    // 表：行は模型、列は NN_TABLE の統計（模擬データは平均 ± 標準偏差、括弧はコスモスとの差が標準偏差の何倍か）
    let col = |k: &str| c.iter().position(|(n, _)| n == k);
    let cols: Vec<(usize, &str)> = NN_TABLE
        .iter()
        .map(|(k, h)| col(k).map(|j| (j, *h)).ok_or(format!("統計 {k} がない")))
        .collect::<Result<_, _>>()?;
    let mut t = format!(
        "% cargo run --release --bin nearest_stats が書いた表\n\
         \\begin{{tabular}}{{l{}}}\n\\toprule\n& {} \\\\\n\\midrule\n",
        "r".repeat(cols.len()),
        cols.iter().map(|c| c.1).collect::<Vec<_>>().join(" & ")
    );
    writeln!(
        t,
        "コスモス & {} \\\\\n\\midrule",
        cols.iter()
            .map(|(j, _)| format!("${:.3}$", c[*j].1))
            .collect::<Vec<_>>()
            .join(" & ")
    )?;
    for (name, vals) in &per_model {
        let cells: Vec<String> = cols
            .iter()
            .map(|(j, _)| {
                let (m, sd) = mean_sd(&vals[*j]);
                // 小数 1 桁に丸めてから 0.0 を足し、−0.0 を +0.0 にする（cells_table と同じ）
                let z = ((c[*j].1 - m) / sd * 10.0).round() / 10.0 + 0.0;
                format!("${m:.3} \\pm {sd:.3}$ (${z:+.1}$)")
            })
            .collect();
        writeln!(t, "{name} & {} \\\\", cells.join(" & "))?;
    }
    t.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/nearest_stats.tex"), &t)?;

    let mut log = format!("COSMOS の店 {}\n", cosmos.len());
    for (j, (k, cv)) in c.iter().enumerate() {
        write!(log, "{k:<14} COSMOS {cv:>8.4}")?;
        for (name, vals) in &per_model {
            let (m, sd) = mean_sd(&vals[j]);
            let below = vals[j].iter().filter(|x| **x <= *cv).count();
            write!(
                log,
                " | {name} {m:.4} ± {sd:.4}（以下 {below}/{}、{:+.1}σ）",
                vals[j].len(),
                (cv - m) / sd
            )?;
        }
        writeln!(log)?;
    }
    results::write_text(&root.join("data/zeta/nearest_stats.csv"), &csv)?;
    Ok(log)
}

/// ADR 012 の値を選ぶための短い比べ方。何も書かず、ログだけを返す。
/// 統計は λ_c ≥ `p.lambda_mins[0]` の V・κ̂₂（`cells_table` と同じ）と、`nearest_stats` のうち `keys` に挙げたもの。
pub fn local_scan(
    root: &Path,
    p: &CellParams,
    models: &[(&str, &str)],
    keys: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let lmin = *p.lambda_mins.first().ok_or("λ_min を一つ以上渡す")?;
    let Setup {
        grid,
        coarse,
        refs,
        cosmos,
    } = setup(root, p)?;
    // カタログ一つの（名前、値）
    let one = |pts: &[LatLon]| -> Vec<(String, f64)> {
        let s = cell_stats(&coarse.counts(&grid.geometry, pts, &refs, p.r_w_km), lmin);
        let mut v = vec![
            ("V".to_string(), s.void_ratio()),
            ("kappa2".to_string(), s.kappa2),
        ];
        v.extend(
            coarse
                .nearest_stats(&grid.geometry, pts, &refs, p.r_w_km)
                .into_iter()
                .filter(|(k, _)| keys.contains(&k.as_str())),
        );
        v
    };
    let c = one(&cosmos);
    let mut log = String::from("COSMOS");
    for (k, v) in &c {
        write!(log, "  {k} {v:.4}")?;
    }
    writeln!(log)?;
    for (name, stem) in models {
        let runs = read_points(root, stem)?;
        let per_run: Vec<Vec<(String, f64)>> = runs.values().map(|pts| one(pts)).collect();
        write!(log, "{name}（{} 回）", per_run.len())?;
        for (j, (k, cv)) in c.iter().enumerate() {
            let v: Vec<f64> = per_run.iter().map(|r| r[j].1).collect();
            let (m, sd) = mean_sd(&v);
            write!(log, "  {k} {m:.4} ± {sd:.4}（{:+.1}σ）", (cv - m) / sd)?;
        }
        writeln!(log)?;
    }
    Ok(log)
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
    let Setup {
        grid,
        coarse,
        refs,
        cosmos,
    } = setup(root, p)?;

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

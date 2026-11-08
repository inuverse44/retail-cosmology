//! オーストラリアの DD の分散：ランダムカタログから幾何の因子 G_p、G_t を求め、
//! Coles と Woolworths の店の数で、店の数を固定した分散とポアソン分布で揺らした分散を比べる（#3）。

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use crate::access::{population, stores};
use crate::engine::pair_count::Bins;
use crate::engine::random_catalog::{Weight, sample};
use crate::engine::variance::{geometric_factors, var_dd_fixed, var_dd_poisson};
use crate::util::geo::{Unit, chord2, km_to_chord2};
use crate::util::rng::SplitMix64;

/// 一回の計算：基準、ランダムカタログの点の数、乱数の種。
#[derive(Clone, Copy, Debug)]
pub struct Run {
    pub weight: Weight,
    pub n_random: usize,
    pub seed: u64,
}

/// 結果：CSV（行はランダムカタログごと・ビンごと・チェーンごと）と、ノートの表（TeX の行）。
pub struct Output {
    pub csv: String,
    pub table: String,
}

/// 途中経過は標準エラー出力に出す。表には、人口に比例する基準のうち点の数が最も多く、
/// 種が最も小さいランダムカタログを使う。
pub fn run(root: &Path, date: &str, runs: &[Run]) -> Result<Output, Box<dyn std::error::Error>> {
    let grid = population::read_abs(&root.join("data/abs/apgr25_1_0_0.tif"))?;
    let path = root.join(format!("data/osm/stores_clean_{date}.csv"));
    let all =
        stores::read_selected(&path).map_err(|e| format!("{} がない：{e}", path.display()))?;
    let chains: Vec<(&str, f64)> = ["Coles", "Woolworths"]
        .iter()
        .map(|c| (*c, all.iter().filter(|(n, _)| n == c).count() as f64))
        .collect();
    for (c, n) in &chains {
        eprintln!("{c}：{n} 店");
    }

    // 二点相関関数と同じビン（manager::correlation）
    let bins = Bins::log(2.0, 2000.0, 15);
    let cbins = bins.map(km_to_chord2);
    let mut out = String::from(
        "weight,n_random,seed,r_lo_km,r_hi_km,gp,gt,gt_minus_gp2,chain,n,mean_dd,var_fixed,var_poisson,n3_over_n2_fixed\n",
    );
    let table_run = runs
        .iter()
        .filter(|r| r.weight == Weight::Population)
        .min_by_key(|r| (std::cmp::Reverse(r.n_random), r.seed))
        .copied();
    let mut table = String::new();
    for r in runs {
        let t0 = Instant::now();
        let mut rng = SplitMix64::new(r.seed);
        let pts: Vec<Unit> = sample(&grid, r.n_random, r.weight, &mut rng)
            .iter()
            .map(Unit::from)
            .collect();
        let g = geometric_factors(&pts, &cbins, chord2);
        eprintln!(
            "{:?}、{} 点、種 {}：{:.1} 秒",
            r.weight,
            r.n_random,
            r.seed,
            t0.elapsed().as_secs_f64()
        );
        let e = bins.edges();
        let is_table = table_run
            .is_some_and(|t| (t.weight, t.n_random, t.seed) == (r.weight, r.n_random, r.seed));
        for k in 0..bins.len() {
            let (gp, gt) = (g.gp[k], g.gt[k]);
            if is_table {
                // 距離、G_p、(G_t - G_p^2)/G_p^2、チェーンごとに DD の相対標準偏差（固定、ポアソン）
                write!(
                    table,
                    "${:.1}$--${:.1}$ & ${:.2}$ & ${:.2}$",
                    e[k],
                    e[k + 1],
                    gp * 1e3,
                    (gt - gp * gp) / (gp * gp)
                )?;
                for (_, n) in &chains {
                    let mean = 0.5 * n * (n - 1.0) * gp;
                    write!(
                        table,
                        " & ${:.3}$ & ${:.3}$",
                        var_dd_fixed(*n, gp, gt).sqrt() / mean,
                        var_dd_poisson(*n, gp, gt).sqrt() / mean
                    )?;
                }
                table.push_str(" \\\\\n");
            }
            for (c, n) in &chains {
                let n3 = n * (n - 1.0) * (n - 2.0) * (gt - gp * gp);
                let n2 = 0.5 * n * (n - 1.0) * gp * (1.0 - gp);
                writeln!(
                    out,
                    "{:?},{},{},{},{},{gp:.6e},{gt:.6e},{:.6e},{c},{n},{:.3},{:.3},{:.3},{:.4}",
                    r.weight,
                    r.n_random,
                    r.seed,
                    e[k],
                    e[k + 1],
                    gt - gp * gp,
                    0.5 * n * (n - 1.0) * gp,
                    var_dd_fixed(*n, gp, gt),
                    var_dd_poisson(*n, gp, gt),
                    n3 / n2
                )?;
            }
        }
    }
    let table = if table.is_empty() {
        table
    } else {
        let t = table_run.unwrap();
        format!(
            "% cargo run --release --bin var_dd -- {date} が書いた表（人口に比例する基準、{} 点、種 {}）\n\
             \\begin{{tabular}}{{lrrrrrr}}\n\\toprule\n\
             & & & \\multicolumn{{2}}{{c}}{{{}（{} 店）}} & \\multicolumn{{2}}{{c}}{{{}（{} 店）}} \\\\\n\
             \\cmidrule(lr){{4-5}} \\cmidrule(lr){{6-7}}\n\
             $r$ [km] & $10^3 G_p$ & $(G_t - G_p^2)/G_p^2$ & 固定 & ポアソン & 固定 & ポアソン \\\\\n\\midrule\n\
             {table}\\bottomrule\n\\end{{tabular}}\n",
            t.n_random, t.seed, chains[0].0, chains[0].1, chains[1].0, chains[1].1
        )
    };
    Ok(Output { csv: out, table })
}

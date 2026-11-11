//! 使用例：8 km より短い距離の選択関数つきの ξ_LS と相互相関を、帰無仮説の模擬データと比べる（#50）。
//!
//! 実際の値は xi が書いた data/xi/xi_jp_<日付>_crossfp.csv、模擬データの値は
//! scripts/null_mocks_cross.sh が書いた …_bins.csv から読む。
//! 比べる量は二つ：3.2–8.0 km の 2 ビンの単純な平均と、2.0–3.2 km のビン。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::results;
use crate::contract::BinValue;
use crate::engine::estimator::mock_summary;

/// 比べる距離の範囲：表に出す名前と、ビンの下端・上端 [km] の範囲。
const RANGES: [(&str, f64, f64); 2] = [("3.2--8.0", 3.0, 8.5), ("2.0--3.2", 1.9, 3.3)];

/// 模擬データの組：ファイルの接尾辞、表に出す名前、比べる標本。
/// 一方のチェーンを残す組では、残したチェーンの ξ は実際の値と同じなので、相互相関だけを比べる。
fn mock_sets() -> Vec<(
    &'static str,
    &'static str,
    Vec<(&'static str, &'static str)>,
)> {
    let all = vec![
        ("ls", "TRIAL"),
        ("ls", "SEIYU"),
        ("ls_cross", "TRIALxSEIYU"),
    ];
    let cross = vec![("ls_cross", "TRIALxSEIYU")];
    vec![
        ("", "地方だけ", all.clone()),
        ("_density", "密集地も", all),
        ("_density_grid_keepTRIAL", "トライアルを残す", cross.clone()),
        ("_density_grid_keepSEIYU", "西友を残す", cross),
    ]
}

fn mean_in(bins: &[&BinValue], lo: f64, hi: f64) -> Option<f64> {
    let v: Vec<f64> = bins
        .iter()
        .filter(|b| b.r_lo_km >= lo && b.r_hi_km <= hi)
        .map(|b| b.value)
        .collect();
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// 表を docs/tables/short_range.tex に書き、同じ内容を文字列で返す。`jp` は実際の値を書いた xi の日付。
pub fn run(root: &Path, jp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let real_path = format!("data/xi/xi_jp_{jp}_crossfp.csv");
    let real = results::read_bins(&root.join(&real_path))
        .map_err(|e| format!("{real_path} がない：{e}"))?;
    let mut report = String::new();
    let mut tex = format!(
        "% cargo run --release --bin short_range -- {jp} が書いた表\n\
         \\begin{{tabular}}{{llllrrrr}}\n\\toprule\n\
         $R$ & 模擬データ & 標本 & 範囲 [km] & 実際 & 模擬データ & 以上 & 以下 \\\\\n\\midrule\n"
    );
    for (ri, r) in [50_u32, 100].into_iter().enumerate() {
        if ri > 0 {
            tex.push_str("\\midrule\n");
        }
        let random = format!("footprint_{r}");
        for (suffix, set_name, samples) in mock_sets() {
            let path = format!("data/xi/null_mocks_cross_TRIALxSEIYU_{r}{suffix}_bins.csv");
            let mocks =
                results::read_bins(&root.join(&path)).map_err(|e| format!("{path} がない：{e}"))?;
            let mut seeds: Vec<u64> = mocks.iter().filter_map(|m| m.seed).collect();
            seeds.sort_unstable();
            seeds.dedup();
            for (q, s) in samples {
                let rb: Vec<&BinValue> = real
                    .iter()
                    .filter(|b| b.quantity == q && b.sample == s && b.random == random)
                    .collect();
                for (range, lo, hi) in RANGES {
                    let obs = mean_in(&rb, lo, hi)
                        .ok_or(format!("{s} の {range} km の実際の値がない"))?;
                    let vals: Vec<f64> = seeds
                        .iter()
                        .filter_map(|seed| {
                            let mb: Vec<&BinValue> = mocks
                                .iter()
                                .filter(|m| {
                                    m.seed == Some(*seed) && m.quantity == q && m.sample == s
                                })
                                .collect();
                            mean_in(&mb, lo, hi)
                        })
                        .collect();
                    let m = mock_summary(obs, &vals);
                    writeln!(
                        report,
                        "R = {r} km、{set_name}、{s}、{range} km：実際 {obs:+.3}、模擬データ {} 個の平均 {:+.3}、標準偏差 {:.3}、最小 {:+.3}、最大 {:+.3}、実際以上 {} 個、実際以下 {} 個（平均から {:+.1}σ）",
                        m.n,
                        m.mean,
                        m.sd,
                        m.min,
                        m.max,
                        m.n_ge,
                        m.n_le,
                        (obs - m.mean) / m.sd
                    )?;
                    let sample = match s {
                        "TRIAL" => "トライアル",
                        "SEIYU" => "西友",
                        _ => "相互相関",
                    };
                    writeln!(
                        tex,
                        "{r} & {set_name} & {sample} & {range} & ${obs:+.3}$ & ${:+.3} \\pm {:.3}$ & {} & {} \\\\",
                        m.mean, m.sd, m.n_ge, m.n_le
                    )?;
                }
            }
        }
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/short_range.tex"), &tex)?;
    Ok(report)
}

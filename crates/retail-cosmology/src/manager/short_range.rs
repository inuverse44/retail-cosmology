//! 使用例：選択関数つきの ξ_LS と相互相関を、距離の範囲ごとに帰無仮説の模擬データと比べる。
//!
//! [`run`]（#50）：トライアルと西友の 8 km より短い距離。比べる量は 3.2–8.0 km の 2 ビンの単純な平均と、2.0–3.2 km のビン。
//! [`run_cosmos`]（#51）：トライアルとコスモスの、全国と九州の中だけ。比べる量は 8 km から R/2 までの単純な平均と、3.2–8.0 km の平均。
//!
//! 実際の値は xi が書いた data/xi/xi_jp_<日付>_<名前>.csv、模擬データの値は
//! scripts/null_mocks_cross.sh が書いた …_bins.csv から読む。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::results;
use crate::contract::BinValue;
use crate::engine::estimator::mock_summary;

/// 距離の範囲：表に出す名前と、ビンの下端の下限・上端の上限 [km]。上限が None なら R/2。
type Range = (&'static str, f64, Option<f64>);

/// 模擬データの組：ファイルの接尾辞、表に出す名前、比べる（量、標本）。
type MockSet = (
    &'static str,
    &'static str,
    Vec<(&'static str, &'static str)>,
);

/// 比べ方の設定。
struct Compare<'a> {
    /// 実際の値の CSV（リポジトリの根からの相対パス）
    real: String,
    /// 模擬データのファイルの前置き（例：null_mocks_cross_TRIALxSEIYU）
    mock_prefix: &'a str,
    /// 模擬データのファイルの最後につける名前（例：_kyushu）
    area_suffix: &'a str,
    /// 表に出す、測った範囲の名前（空なら列を出さない）
    area_name: &'a str,
    sets: Vec<MockSet>,
    ranges: Vec<Range>,
}

fn mean_in(bins: &[&BinValue], lo: f64, hi: f64) -> Option<f64> {
    let v: Vec<f64> = bins
        .iter()
        .filter(|b| b.r_lo_km >= lo && b.r_hi_km <= hi)
        .map(|b| b.value)
        .collect();
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn sample_name(s: &str) -> &'static str {
    match s {
        "TRIAL" => "トライアル",
        "SEIYU" => "西友",
        "COSMOS" => "コスモス",
        _ => "相互相関",
    }
}

/// R = 50、100 km について、設定どおりに比べ、報告の文字列と表の行を足す。
fn compare(
    root: &Path,
    c: &Compare,
    report: &mut String,
    tex: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let real =
        results::read_bins(&root.join(&c.real)).map_err(|e| format!("{} がない：{e}", c.real))?;
    for (ri, r) in [50_u32, 100].into_iter().enumerate() {
        if ri > 0 {
            tex.push_str("\\midrule\n");
        }
        let random = format!("footprint_{r}");
        for (suffix, set_name, samples) in &c.sets {
            let path = format!(
                "data/xi/{}_{r}{suffix}{}_bins.csv",
                c.mock_prefix, c.area_suffix
            );
            let mocks =
                results::read_bins(&root.join(&path)).map_err(|e| format!("{path} がない：{e}"))?;
            let mut seeds: Vec<u64> = mocks.iter().filter_map(|m| m.seed).collect();
            seeds.sort_unstable();
            seeds.dedup();
            for (q, s) in samples {
                let rb: Vec<&BinValue> = real
                    .iter()
                    .filter(|b| b.quantity == *q && b.sample == *s && b.random == random)
                    .collect();
                for (range, lo, hi) in &c.ranges {
                    let hi = hi.unwrap_or(r as f64 / 2.0 + 0.1);
                    let range = range.replace("R/2", &format!("{:.1}", r as f64 / 2.0));
                    let obs = mean_in(&rb, *lo, hi)
                        .ok_or(format!("{s} の {range} km の実際の値がない"))?;
                    let vals: Vec<f64> = seeds
                        .iter()
                        .filter_map(|seed| {
                            let mb: Vec<&BinValue> = mocks
                                .iter()
                                .filter(|m| {
                                    m.seed == Some(*seed) && m.quantity == *q && m.sample == *s
                                })
                                .collect();
                            mean_in(&mb, *lo, hi)
                        })
                        .collect();
                    let m = mock_summary(obs, &vals);
                    writeln!(
                        report,
                        "{}R = {r} km、{set_name}、{s}、{range} km：実際 {obs:+.3}、模擬データ {} 個の平均 {:+.3}、標準偏差 {:.3}、最小 {:+.3}、最大 {:+.3}、実際以上 {} 個、実際以下 {} 個（平均から {:+.1}σ）",
                        if c.area_name.is_empty() {
                            String::new()
                        } else {
                            format!("{}、", c.area_name)
                        },
                        m.n,
                        m.mean,
                        m.sd,
                        m.min,
                        m.max,
                        m.n_ge,
                        m.n_le,
                        (obs - m.mean) / m.sd
                    )?;
                    let area = if c.area_name.is_empty() {
                        String::new()
                    } else {
                        format!("{} & ", c.area_name)
                    };
                    writeln!(
                        tex,
                        "{area}{r} & {set_name} & {} & {range} & ${obs:+.3}$ & ${:+.3} \\pm {:.3}$ & {} & {} \\\\",
                        sample_name(s),
                        m.mean,
                        m.sd,
                        m.n_ge,
                        m.n_le
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// トライアルと西友の 8 km より短い距離（#50）。表を docs/tables/short_range.tex に書く。
/// `jp` は実際の値を書いた xi の日付。
pub fn run(root: &Path, jp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let all = vec![
        ("ls", "TRIAL"),
        ("ls", "SEIYU"),
        ("ls_cross", "TRIALxSEIYU"),
    ];
    // 一方のチェーンを残す組では、残したチェーンの ξ は実際の値と同じなので、相互相関だけを比べる
    let cross = vec![("ls_cross", "TRIALxSEIYU")];
    let c = Compare {
        real: format!("data/xi/xi_jp_{jp}_crossfp.csv"),
        mock_prefix: "null_mocks_cross_TRIALxSEIYU",
        area_suffix: "",
        area_name: "",
        sets: vec![
            ("", "地方だけ", all.clone()),
            ("_density", "密集地も", all),
            ("_density_grid_keepTRIAL", "トライアルを残す", cross.clone()),
            ("_density_grid_keepSEIYU", "西友を残す", cross),
        ],
        ranges: vec![
            ("3.2--8.0", 3.0, Some(8.5)),
            ("2.0--3.2", 1.9, Some(3.3)),
        ],
    };
    let mut report = String::new();
    let mut tex = format!(
        "% cargo run --release --bin short_range -- {jp} が書いた表\n\
         \\begin{{tabular}}{{llllrrrr}}\n\\toprule\n\
         $R$ & 模擬データ & 標本 & 範囲 [km] & 実際 & 模擬データ & 以上 & 以下 \\\\\n\\midrule\n"
    );
    compare(root, &c, &mut report, &mut tex)?;
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/short_range.tex"), &tex)?;
    Ok(report)
}

/// トライアルとコスモスの、全国と九州の中だけ（#51）。表を docs/tables/cosmos_mocks.tex に書く。
/// 実際の値は data/xi/xi_jp_<jp>_cosmosfp.csv（全国）と xi_jp_<jp>_kyushu.csv（九州）。
pub fn run_cosmos(root: &Path, jp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let all = vec![
        ("ls", "TRIAL"),
        ("ls", "COSMOS"),
        ("ls_cross", "TRIALxCOSMOS"),
    ];
    let mut report = String::new();
    let mut tex = format!(
        "% cargo run --release --bin short_range -- {jp} cosmos が書いた表\n\
         \\begin{{tabular}}{{lllllrrrr}}\n\\toprule\n\
         範囲 & $R$ & 模擬データ & 標本 & 距離 [km] & 実際 & 模擬データ & 以上 & 以下 \\\\\n\\midrule\n"
    );
    for (i, (tag, area_suffix, area_name)) in [
        ("cosmosfp", "", "全国"),
        ("kyushu", "_kyushu", "九州"),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            tex.push_str("\\midrule\n");
        }
        let c = Compare {
            real: format!("data/xi/xi_jp_{jp}_{tag}.csv"),
            mock_prefix: "null_mocks_cross_TRIALxCOSMOS",
            area_suffix,
            area_name,
            sets: vec![
                ("_grid", "地方だけ", all.clone()),
                ("_density_grid", "密集地も", all.clone()),
            ],
            ranges: vec![
                ("8.0--R/2", 7.9, None),
                ("3.2--8.0", 3.0, Some(8.5)),
            ],
        };
        compare(root, &c, &mut report, &mut tex)?;
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/cosmos_mocks.tex"), &tex)?;
    Ok(report)
}

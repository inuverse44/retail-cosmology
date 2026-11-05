//! 使用例：jackknife の共分散から求めた χ² の上側確率を、帰無仮説の模擬データで校正する（#9）。
//! 観測の χ² と、模擬データ（scripts/chi2_mocks.sh、scripts/null_mocks_cross.sh が書いたもの）の χ² を突き合わせ、
//! 模擬データのうち χ² が観測以上のものの割合を、模擬データと比べた上側確率とする。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::results;
use crate::contract::Chi2Record;
use crate::engine::estimator::mock_sf;

/// 一つの国の入力：観測の χ² のファイルと、R ごとの模擬データのファイル（R = 0 は人口が基準）。
struct Case {
    country: &'static str,
    observed: String,
    mocks: Vec<(u32, String)>,
}

/// 表の一行。
struct Row {
    country: &'static str,
    obs: Chi2Record,
    n_mocks: usize,
    exceed: usize,
    p_mock: f64,
    /// 模擬データのうち、χ² 分布から求めた上側確率が 0.05 未満だった割合
    mock_small_p: f64,
}

fn cases(au: &str, jp: &str) -> Vec<Case> {
    let x = |f: &str| format!("data/xi/{f}");
    vec![
        Case {
            country: "オーストラリア",
            observed: x(&format!("chi2_{au}_fp.txt")),
            mocks: vec![
                (0, x("chi2_mocks_au_0.txt")),
                (50, x("chi2_mocks_au_50.txt")),
                (100, x("chi2_mocks_au_100.txt")),
                (200, x("chi2_mocks_au_200.txt")),
            ],
        },
        Case {
            country: "日本",
            observed: x(&format!("chi2_jp_{jp}_crossfp.txt")),
            mocks: vec![
                (0, x("chi2_mocks_jp_0.txt")),
                (50, x("null_mocks_cross_TRIALxSEIYU_50.txt")),
                (100, x("null_mocks_cross_TRIALxSEIYU_100.txt")),
                (200, x("chi2_mocks_jp_200.txt")),
            ],
        },
    ]
}

fn random_of(r: u32) -> String {
    if r == 0 {
        "population".into()
    } else {
        format!("footprint_{r}")
    }
}

/// 校正した表を docs/tables/chi2_calibrated.tex に書き、同じ内容を文字列で返す。
/// `au`、`jp` は観測の χ² を書いた xi の日付（オーストラリアと日本）。
pub fn run(root: &Path, au: &str, jp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for case in cases(au, jp) {
        let observed = results::read_chi2(&root.join(&case.observed))
            .map_err(|e| format!("{} がない：{e}", case.observed))?;
        for (r, path) in &case.mocks {
            let mocks =
                results::read_chi2(&root.join(path)).map_err(|e| format!("{path} がない：{e}"))?;
            let random = random_of(*r);
            for obs in observed.iter().filter(|o| o.random == random) {
                let m: Vec<&Chi2Record> = mocks
                    .iter()
                    .filter(|m| m.sample == obs.sample && m.random == obs.random)
                    .collect();
                if m.is_empty() {
                    return Err(format!(
                        "{path} に {} / {} の模擬データがない",
                        obs.sample, obs.random
                    )
                    .into());
                }
                let chi2: Vec<f64> = m.iter().map(|m| m.chi2).collect();
                let (exceed, p_mock) = mock_sf(obs.chi2, &chi2);
                let small = m.iter().filter(|m| m.p < 0.05).count();
                rows.push(Row {
                    country: case.country,
                    obs: obs.clone(),
                    n_mocks: m.len(),
                    exceed,
                    p_mock,
                    mock_small_p: small as f64 / m.len() as f64,
                });
            }
        }
    }

    let mut report = String::new();
    let mut tex = format!(
        "% cargo run --release --bin chi2_calibrate -- {au} {jp} が書いた表\n\
         \\begin{{tabular}}{{lllrrrr}}\n\\toprule\n\
         国 & 標本 & 基準 & ビン & $\\chi^2$ & \\multicolumn{{2}}{{c}}{{上側確率}} \\\\\n\
         & & & & & $\\chi^2$ 分布 & 模擬データ \\\\\n\\midrule\n"
    );
    let mut last = "";
    for row in &rows {
        let o = &row.obs;
        let p_mock = if row.exceed == 0 {
            format!("$< {:.2}$", 1.0 / row.n_mocks as f64)
        } else {
            format!("{:.2}", row.p_mock)
        };
        let p_chi2 = if o.p < 0.001 {
            "$< 0.001$".to_string()
        } else {
            format!("{:.3}", o.p)
        };
        writeln!(
            report,
            "{} {} / {}（{}）：χ² = {:.1}、χ² 分布の上側確率 {:.3}、模擬データ {} 個のうち χ² 以上が {} 個（{:.2}）、模擬データの {:.0}% が χ² 分布で 0.05 未満",
            row.country,
            o.sample,
            o.random,
            o.bins,
            o.chi2,
            o.p,
            row.n_mocks,
            row.exceed,
            row.p_mock,
            100.0 * row.mock_small_p
        )?;
        if row.country != last && !last.is_empty() {
            tex.push_str("\\midrule\n");
        }
        let country = if row.country != last { row.country } else { "" };
        last = row.country;
        let random = match o.random.strip_prefix("footprint_") {
            Some(r) => format!("$R = {r}\\,\\mathrm{{km}}$"),
            None => "人口".into(),
        };
        let sample = o.sample.replace('x', "$\\times$");
        let nbins = o
            .bins
            .rsplit_once(" の ")
            .map_or(o.bins.as_str(), |(_, b)| b);
        writeln!(
            tex,
            "{country} & {sample} & {random} & {} & {:.1} & {p_chi2} & {p_mock} \\\\",
            nbins.trim_end_matches(" ビン"),
            o.chi2
        )?;
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    results::write_text(&root.join("docs/tables/chi2_calibrated.tex"), &tex)?;
    Ok(report)
}

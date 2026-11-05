//! ResourceAccess：結果のファイル（CSV、テキスト、TeX の表）を書く。xi が書いた χ² の行を読む。

use std::path::Path;

use crate::contract::Chi2Record;

/// 文字列をファイルに書く。親のフォルダがなければ作る。
pub fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

/// xi が書いた χ² の行を読む。ほかの行（まとめた値、「== 種 s」など）は飛ばす。
/// 行の形：「TRIAL / footprint_50：3–25 km の 4 ビン、χ² = 18.3（Hartlap 0.921 をかけた値）、上側確率 0.001」
/// 相互相関では「：」のあとに「相互相関、」が入る。
pub fn read_chi2(path: &Path) -> std::io::Result<Vec<Chi2Record>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter_map(parse_chi2_line)
        .collect())
}

fn parse_chi2_line(line: &str) -> Option<Chi2Record> {
    let (head, rest) = line.split_once('：')?;
    let (sample, random) = head.split_once(" / ")?;
    let (bins, rest) = rest.split_once("、χ² = ")?;
    let (chi2, rest) = rest.split_once('（')?;
    let p = rest.split_once("上側確率 ")?.1;
    Some(Chi2Record {
        sample: sample.into(),
        random: random.into(),
        bins: bins.trim_start_matches("相互相関、").into(),
        chi2: chi2.parse().ok()?,
        p: p.trim().parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_auto_and_cross_lines_and_skips_others() {
        let a = parse_chi2_line(
            "TRIAL / footprint_50：3–25 km の 4 ビン、χ² = 18.3（Hartlap 0.921 をかけた値）、上側確率 0.001",
        )
        .unwrap();
        assert_eq!(a.sample, "TRIAL");
        assert_eq!(a.random, "footprint_50");
        assert_eq!(a.bins, "3–25 km の 4 ビン");
        assert_eq!((a.chi2, a.p), (18.3, 0.001));
        let c = parse_chi2_line(
            "TRIALxSEIYU / population：相互相関、3–2000 km の 14 ビン、χ² = 46.8（Hartlap 0.762 をかけた値）、上側確率 0.000",
        )
        .unwrap();
        assert_eq!(c.bins, "3–2000 km の 14 ビン");
        assert_eq!(c.chi2, 46.8);
        assert!(parse_chi2_line("== 種 1").is_none());
        assert!(
            parse_chi2_line(
                "TRIAL の ξ_LS（footprint_50）：8.0–20.0 km の 2 ビンをまとめた値 0.118 ± 0.061"
            )
            .is_none()
        );
    }
}

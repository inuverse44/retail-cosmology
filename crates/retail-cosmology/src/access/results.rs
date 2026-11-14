//! ResourceAccess：結果のファイル（CSV、テキスト、TeX の表）を書く。xi が書いた χ² の行とビンごとの値を読む。

use std::path::Path;

use crate::contract::{BinValue, Chi2Record, ZetaRow};

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

/// ビンごとの値の CSV を読む。見出しが seed で始まれば模擬データの形
/// （seed,quantity,sample,r_lo_km,r_hi_km,value,sigma）、そうでなければ xi の形
/// （quantity,sample,random,r_lo_km,r_hi_km,value,sigma）として読む。
pub fn read_bins(path: &Path) -> std::io::Result<Vec<BinValue>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let mock = lines.next().is_some_and(|h| h.starts_with("seed,"));
    Ok(lines.filter_map(|l| parse_bin_line(l, mock)).collect())
}

/// zeta が書いた CSV（k1,k2,k3,r1_km,r2_km,r3_km,ddd,ddr,drr,rrr,zeta）を読む。ζ が NaN の行（使わないビン）は飛ばす。
pub fn read_zeta(path: &Path) -> std::io::Result<Vec<ZetaRow>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .skip(1)
        .filter_map(parse_zeta_line)
        .collect())
}

fn parse_zeta_line(line: &str) -> Option<ZetaRow> {
    let f: Vec<&str> = line.split(',').collect();
    if f.len() != 11 {
        return None;
    }
    let zeta: f64 = f[10].parse().ok()?;
    if zeta.is_nan() {
        return None;
    }
    Some(ZetaRow {
        k: (f[0].parse().ok()?, f[1].parse().ok()?, f[2].parse().ok()?),
        r_km: (f[3].parse().ok()?, f[4].parse().ok()?, f[5].parse().ok()?),
        ddd: f[6].parse().ok()?,
        zeta,
    })
}

/// 見出しつきの数値の CSV を読み、見出しの名前ごとの列を返す。数でない欄（空欄など）は NaN にする。
/// zeta_clustered が書いた CSV（ζ と ξ）を読むのに使う。
pub fn read_numeric_csv(path: &Path) -> std::io::Result<Vec<std::collections::HashMap<String, f64>>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let head: Vec<String> = lines.next().unwrap_or("").split(',').map(String::from).collect();
    Ok(lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            head.iter()
                .cloned()
                .zip(l.split(',').map(|v| v.parse().unwrap_or(f64::NAN)))
                .collect()
        })
        .collect())
}

fn parse_bin_line(line: &str, mock: bool) -> Option<BinValue> {
    let f: Vec<&str> = line.split(',').collect();
    if f.len() < 7 {
        return None;
    }
    let (seed, random) = if mock {
        (Some(f[0].parse().ok()?), String::new())
    } else {
        (None, f[2].to_string())
    };
    let (q, s) = if mock { (f[1], f[2]) } else { (f[0], f[1]) };
    Some(BinValue {
        seed,
        quantity: q.into(),
        sample: s.into(),
        random,
        r_lo_km: f[3].parse().ok()?,
        r_hi_km: f[4].parse().ok()?,
        value: f[5].parse().ok()?,
    })
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

    #[test]
    fn parses_xi_and_mock_bin_lines() {
        let x = parse_bin_line(
            "ls_cross,TRIALxSEIYU,footprint_50,2,3.17,-0.145,0.264",
            false,
        )
        .unwrap();
        assert_eq!(x.seed, None);
        assert_eq!(
            (x.quantity.as_str(), x.random.as_str()),
            ("ls_cross", "footprint_50")
        );
        assert_eq!((x.r_lo_km, x.r_hi_km, x.value), (2.0, 3.17, -0.145));
        let m = parse_bin_line("7,ls,SEIYU,3.17,5.02,0.25,0.3", true).unwrap();
        assert_eq!(m.seed, Some(7));
        assert_eq!(
            (m.quantity.as_str(), m.sample.as_str(), m.random.as_str()),
            ("ls", "SEIYU", "")
        );
        assert_eq!(m.value, 0.25);
        assert!(parse_bin_line("quantity,sample", false).is_none());
    }
}

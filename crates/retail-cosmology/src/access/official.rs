//! ResourceAccess：決算資料にある店舗数（`reference/official_store_counts.csv`）を読む。

use std::path::Path;

use crate::contract::OfficialCount;

/// 列は chain,region,count,as_at,source。
pub fn read_official(path: &Path) -> std::io::Result<Vec<OfficialCount>> {
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .skip(1)
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            OfficialCount {
                chain: f[0].to_string(),
                region: f[1].to_string(),
                count: f[2].parse().unwrap(),
            }
        })
        .collect())
}

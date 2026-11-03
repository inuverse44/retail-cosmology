//! トライアルの公式サイトの店舗情報（`scripts/trial_site/fetch.sh` が取った JSON）を読む。

use std::path::Path;

use crate::LatLon;

#[derive(Clone, Debug)]
pub struct SiteStore {
    pub id: String,
    pub name: String,
    pub category: String,
    pub pos: LatLon,
    pub prefecture: String,
}

/// フォルダの中の batch_*.json をすべて読む。
pub fn read_dir(dir: &Path) -> Result<Vec<SiteStore>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("batch_"))
        })
        .collect();
    files.sort();
    for f in files {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&f)?)?;
        for s in v.as_array().ok_or("配列ではない")? {
            let get = |k: &str| s[k].as_str().unwrap_or_default().trim().to_string();
            out.push(SiteStore {
                id: get("store_id"),
                name: get("store_name"),
                category: get("store_category"),
                pos: LatLon {
                    lat: get("store_lat").parse()?,
                    lon: get("store_lon").parse()?,
                },
                prefecture: get("store_prefecture"),
            });
        }
    }
    Ok(out)
}

/// 数える業態か。トライアルの名前の業態を数え、西友（トライアル西友）、ドラッグストア（トライウェル、TRIDRUG）、
/// 別ブランド（タイヨー）、非表示の店を除く。決算資料のトライアルの店舗数（西友を含まない）に合わせる。
pub fn is_trial_format(category: &str) -> bool {
    let up = category.to_ascii_uppercase();
    let excluded = category.contains("西友")
        || category.contains("トライウェル")
        || up.contains("TRIDRUG")
        || category.contains("タイヨー")
        || category == "非表示";
    !excluded
        && (category.contains("トライアル") || up.contains("TRIAL") || up.contains("TRIFOODS"))
}

const PREFECTURES: [&str; 47] = [
    "北海道",
    "青森県",
    "岩手県",
    "宮城県",
    "秋田県",
    "山形県",
    "福島県",
    "茨城県",
    "栃木県",
    "群馬県",
    "埼玉県",
    "千葉県",
    "東京都",
    "神奈川県",
    "新潟県",
    "富山県",
    "石川県",
    "福井県",
    "山梨県",
    "長野県",
    "岐阜県",
    "静岡県",
    "愛知県",
    "三重県",
    "滋賀県",
    "京都府",
    "大阪府",
    "兵庫県",
    "奈良県",
    "和歌山県",
    "鳥取県",
    "島根県",
    "岡山県",
    "広島県",
    "山口県",
    "徳島県",
    "香川県",
    "愛媛県",
    "高知県",
    "福岡県",
    "佐賀県",
    "長崎県",
    "熊本県",
    "大分県",
    "宮崎県",
    "鹿児島県",
    "沖縄県",
];

/// 都道府県名 → ISO 3166-2 のコード（JP-01〜JP-47）
pub fn prefecture_code(name: &str) -> Option<String> {
    PREFECTURES
        .iter()
        .position(|p| *p == name)
        .map(|i| format!("JP-{:02}", i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_prefectures() {
        assert!(is_trial_format("スーパーセンタートライアル"));
        assert!(is_trial_format("TRIAL GO"));
        assert!(is_trial_format("TRIALsmart"));
        assert!(is_trial_format("TRIFOODS GO"));
        assert!(!is_trial_format("トライアル西友"));
        assert!(!is_trial_format("ドラッグストアトライウェル"));
        assert!(!is_trial_format("TRIDRUG GO"));
        assert!(!is_trial_format("スーパーセンタータイヨー"));
        assert!(!is_trial_format("非表示"));
        assert_eq!(prefecture_code("北海道").as_deref(), Some("JP-01"));
        assert_eq!(prefecture_code("福岡県").as_deref(), Some("JP-40"));
        assert_eq!(prefecture_code("沖縄県").as_deref(), Some("JP-47"));
        assert_eq!(prefecture_code("どこか"), None);
    }
}

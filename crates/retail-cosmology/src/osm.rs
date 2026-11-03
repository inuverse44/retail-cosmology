//! `scripts/osm/fetch.sh` が書いた TSV を読み、Coles と Woolworths（オーストラリア）、
//! トライアル（日本）の店舗を選び出す。

use crate::{LatLon, great_circle_km};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Chain {
    Coles,
    Woolworths,
    Trial,
}

/// トライアルの Wikidata の項目
pub const TRIAL_QID: &str = "Q11321723";

impl Chain {
    pub fn name(self) -> &'static str {
        match self {
            Chain::Coles => "Coles",
            Chain::Woolworths => "Woolworths",
            Chain::Trial => "TRIAL",
        }
    }

    /// 各社が決算資料で使う地域の分け方に、州・特別地域のコードを寄せる。
    pub fn region(self, state: &str) -> &'static str {
        match (self, state) {
            (_, "AU-NSW" | "AU-ACT") => "NSW & ACT",
            (_, "AU-QLD") => "QLD",
            (_, "AU-SA" | "AU-NT") => "SA & NT",
            (_, "AU-WA") => "WA",
            (Chain::Coles, "AU-VIC" | "AU-TAS") => "VIC & TAS",
            (Chain::Woolworths, "AU-VIC") => "VIC",
            (Chain::Woolworths, "AU-TAS") => "TAS",
            (Chain::Trial, s) => jp_region(s),
            _ => "other",
        }
    }
}

/// 都道府県のコード（JP-01〜JP-47）を、トライアルの決算資料の地方に寄せる。
/// 資料に地方の定義は書かれていないので、一般的な 8 地方区分（中国と四国はまとめる）を仮定する。
fn jp_region(code: &str) -> &'static str {
    let Some(n) = code.strip_prefix("JP-").and_then(|n| n.parse::<u32>().ok()) else {
        return "other";
    };
    match n {
        1 => "北海道",
        2..=7 => "東北",
        8..=14 => "関東",
        15..=23 => "中部",
        24..=30 => "近畿",
        31..=39 => "中国・四国",
        40..=47 => "九州",
        _ => "other",
    }
}

/// TSV の一行（店舗の候補）。
#[derive(Clone, Debug)]
pub struct Element {
    pub kind: String,
    pub id: u64,
    pub pos: LatLon,
    pub state: String,
    pub shop: String,
    pub brand: String,
    pub wikidata: String,
    pub name: String,
}

/// 選び方。
///
/// Coles と Woolworths：`shop=supermarket` で、`brand` が次のどれか。
/// Coles は Coles・Coles Local・Coles Central、Woolworths は Woolworths・Woolworths Metro。
/// 公式の数が、Coles はスーパー全体、Woolworths はスーパーと Metro の合計なので、それに合わせる。
///
/// トライアル：`shop` が小売の店を表すタグで、`brand:wikidata` がトライアルの項目であるか、
/// `brand` か `name` がトライアルの名前で始まるもの。西友の店（トライアル西友）は除く。
/// 公式の数（378 店）は西友を含まないので、それに合わせる。
pub fn classify(e: &Element) -> Option<Chain> {
    if is_trial(e) {
        return Some(Chain::Trial);
    }
    if e.shop != "supermarket" {
        return None;
    }
    match e.brand.as_str() {
        "Coles" | "Coles Local" | "Coles Central" => Some(Chain::Coles),
        "Woolworths" | "Woolworths Metro" => Some(Chain::Woolworths),
        _ => None,
    }
}

fn is_trial(e: &Element) -> bool {
    // `yes`（種類の決まっていない店）と `mall`（店の建物を描いたもの）も数える。
    // 同じ店が別に描かれていれば、近すぎる組として後で一方を落とす。
    const SHOPS: [&str; 8] = [
        "supermarket",
        "variety_store",
        "convenience",
        "department_store",
        "general",
        "wholesale",
        "yes",
        "mall",
    ];
    SHOPS.contains(&e.shop.as_str())
        && !e.name.contains("西友")
        && (e.wikidata == TRIAL_QID || trial_name(&e.brand) || trial_name(&e.name))
}

/// 店の名前がトライアルの名前で始まるか。業態の前置き（スーパーセンター、メガセンターなど）は外して見る。
/// 英字は大文字小文字を区別しない。
fn trial_name(s: &str) -> bool {
    const PREFIXES: [&str; 8] = [
        "スーパーセンター",
        "メガセンター",
        "ディスカウントストア",
        "ディスカウントショップ",
        "スーパー",
        "SUPERCENTER",
        "MEGACENTER",
        "MEGASTORE",
    ];
    let mut t = s.trim().to_string();
    loop {
        let up = t.to_ascii_uppercase();
        let Some(p) = PREFIXES.iter().find(|p| up.starts_with(*p)) else {
            break;
        };
        t = t[p.len()..].trim_start().to_string();
    }
    t.starts_with("トライアル") || t.to_ascii_uppercase().starts_with("TRIAL")
}

/// 名前からわかるトライアルの業態。名前に業態が書かれていなければ「不明」。
pub fn trial_format(e: &Element) -> &'static str {
    let s = format!("{} {}", e.brand, e.name);
    let up = s.to_ascii_uppercase();
    if s.contains("メガセンター") || up.contains("MEGACENTER") {
        "メガセンター"
    } else if s.contains("スーパーセンター") || up.contains("SUPERCENTER") {
        "スーパーセンター"
    } else if up.contains("TRIAL GO") || s.contains("トライアルGO") || s.contains("トライアルゴー")
    {
        "小型店"
    } else if up.contains("SMART") || s.contains("スマート") {
        "smart"
    } else {
        "不明"
    }
}

/// TSV を読む。`area` の行のあとに続く行に、その州のコードをつける。
/// 州の境界にかかる way は二つの州に出ることがあるので、(種類, id) で最初の一つだけ残す。
pub fn parse_tsv(text: &str) -> Vec<Element> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut state = String::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 {
            continue;
        }
        if f[0] == "area" {
            state = f[4].to_string();
            continue;
        }
        let (Ok(id), Ok(lat), Ok(lon)) = (f[1].parse(), f[2].parse(), f[3].parse()) else {
            continue;
        };
        if !seen.insert((f[0].to_string(), id)) {
            continue;
        }
        out.push(Element {
            kind: f[0].to_string(),
            id,
            pos: LatLon { lat, lon },
            state: state.clone(),
            shop: f[5].to_string(),
            brand: f[6].to_string(),
            wikidata: f[7].to_string(),
            name: f[8].to_string(),
        });
    }
    out
}

/// 同じチェーンで `radius_km` より近い組を返す。同じ店が node と way の両方で描かれている候補。
pub fn near_pairs<'a>(
    stores: &[(&'a Element, Chain)],
    radius_km: f64,
) -> Vec<(&'a Element, &'a Element, f64)> {
    let mut out = Vec::new();
    for (i, (a, ca)) in stores.iter().enumerate() {
        for (b, cb) in &stores[i + 1..] {
            if ca == cb {
                let d = great_circle_km(&a.pos, &b.pos);
                if d < radius_km {
                    out.push((*a, *b, d));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "@type\t@id\t@lat\t@lon\tISO3166-2\tshop\tbrand\tbrand:wikidata\tname
area\t1\t\t\tAU-ACT\t\t\t\tACT
node\t10\t-35.28\t149.13\t\tsupermarket\tColes\tQ1108172\tColes Civic
way\t11\t-35.2801\t149.1301\t\tsupermarket\tColes\tQ1108172\tColes Civic
node\t12\t-35.30\t149.10\t\toutpost\tColes\tQ1108172\tColes outpost
area\t2\t\t\tAU-NSW\t\t\t\tNSW
way\t11\t-35.2801\t149.1301\t\tsupermarket\tColes\tQ1108172\tColes Civic
node\t13\t-33.87\t151.21\t\tsupermarket\tWoolworths Metro\tQ111772555\tMetro
";

    #[test]
    fn parse_assigns_state_and_drops_repeats() {
        let e = parse_tsv(SAMPLE);
        assert_eq!(e.len(), 4);
        assert_eq!(e[1].state, "AU-ACT"); // way 11 は最初に出た ACT のまま
        assert_eq!(e[3].state, "AU-NSW");
    }

    #[test]
    fn classify_and_regions() {
        let e = parse_tsv(SAMPLE);
        let chains: Vec<_> = e.iter().map(classify).collect();
        assert_eq!(
            chains,
            vec![
                Some(Chain::Coles),
                Some(Chain::Coles),
                None,
                Some(Chain::Woolworths)
            ]
        );
        assert_eq!(Chain::Coles.region("AU-TAS"), "VIC & TAS");
        assert_eq!(Chain::Woolworths.region("AU-TAS"), "TAS");
        assert_eq!(Chain::Woolworths.region("AU-ACT"), "NSW & ACT");
    }

    fn el(shop: &str, brand: &str, qid: &str, name: &str) -> Element {
        Element {
            kind: "node".into(),
            id: 1,
            pos: LatLon {
                lat: 33.6,
                lon: 130.4,
            },
            state: "JP-40".into(),
            shop: shop.into(),
            brand: brand.into(),
            wikidata: qid.into(),
            name: name.into(),
        }
    }

    #[test]
    fn classify_trial() {
        let t = |e: Element| classify(&e) == Some(Chain::Trial);
        assert!(t(el(
            "supermarket",
            "トライアル",
            TRIAL_QID,
            "トライアル 香椎店"
        )));
        assert!(t(el(
            "supermarket",
            "",
            "",
            "スーパーセンタートライアル 新宮店"
        )));
        assert!(t(el(
            "variety_store",
            "",
            "",
            "メガセンタートライアル 田川店"
        )));
        assert!(t(el("convenience", "", "", "TRIAL GO 千早駅前店")));
        assert!(!t(el("supermarket", "", "", "トライアル西友 二俣川店")));
        assert!(!t(el("bicycle", "", "", "トライアルバイク専門店")));
        assert!(t(el("supermarket", "", "", "スーパートライアル武雄店")));
        assert!(t(el("supermarket", "", "", "SUPERCENTER TRIAL")));
        assert!(t(el("supermarket", "", "", "megastore TRIAL")));
        assert!(t(el(
            "supermarket",
            "",
            "",
            "ディスカウントショップ トライアル"
        )));
        assert!(t(el("mall", "", "", "メガセンタートライアル 上里店")));
        assert!(t(el("yes", "", "", "トライアル苅田店")));
        assert!(!t(el(
            "variety_store",
            "",
            "",
            "ザ・ダイソー・トライアル上磯店"
        )));
        assert!(!t(el("", "トライアル", TRIAL_QID, "トライアル")));
        assert_eq!(
            trial_format(&el(
                "supermarket",
                "",
                "",
                "スーパーセンタートライアル 新宮店"
            )),
            "スーパーセンター"
        );
        assert_eq!(
            trial_format(&el("convenience", "", "", "TRIAL GO 千早駅前店")),
            "小型店"
        );
        assert_eq!(Chain::Trial.region("JP-40"), "九州");
        assert_eq!(Chain::Trial.region("JP-24"), "近畿");
        assert_eq!(Chain::Trial.region("JP-47"), "九州");
    }

    #[test]
    fn near_pairs_finds_node_and_way_of_same_store() {
        let e = parse_tsv(SAMPLE);
        let stores: Vec<_> = e
            .iter()
            .filter_map(|x| classify(x).map(|c| (x, c)))
            .collect();
        let p = near_pairs(&stores, 0.1);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].0.id, p[0].1.id), (10, 11));
    }
}

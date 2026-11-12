//! Engine：店の選び方。チェーンごとの規則で OSM の要素や公式サイトの店を選び、
//! 決算資料の地域の分け方に寄せ、同じ店が二重に描かれたものを落とす。

use std::collections::HashSet;

use crate::contract::{Chain, OsmElement};
use crate::util::geo::great_circle_km;

/// トライアルの Wikidata の項目
pub const TRIAL_QID: &str = "Q11321723";
/// OSM のコスモス薬品（ドラッグストアコスモス）の項目。
pub const COSMOS_QID: &str = "Q11302127";

/// 各社が決算資料で使う地域の分け方に、州・都道府県のコードを寄せる。
pub fn region(chain: Chain, state: &str) -> &'static str {
    match (chain, state) {
        (_, "AU-NSW" | "AU-ACT") => "NSW & ACT",
        (_, "AU-QLD") => "QLD",
        (_, "AU-SA" | "AU-NT") => "SA & NT",
        (_, "AU-WA") => "WA",
        (Chain::Coles, "AU-VIC" | "AU-TAS") => "VIC & TAS",
        (Chain::Woolworths, "AU-VIC") => "VIC",
        (Chain::Woolworths, "AU-TAS") => "TAS",
        (Chain::Trial | Chain::Seiyu, s) => jp_region(s),
        (Chain::Cosmos, s) => cosmos_region(s),
        _ => "other",
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

/// 都道府県のコードを、コスモス薬品の決算短信の地区（関東、中部、関西、中国、四国、九州）に寄せる。
/// 地区の分け方は公式サイトの店のページの URL（…/shop/<地区>/<県>/）に従う。
/// 8 地方区分と違うのは三重県で、サイトでは中部（chubu/mie）に入る。近畿のほかの府県は関西、中国と四国は分ける。
fn cosmos_region(code: &str) -> &'static str {
    if code == "JP-24" {
        return "中部";
    }
    match jp_region(code) {
        "近畿" => "関西",
        "中国・四国" => match code.strip_prefix("JP-").and_then(|n| n.parse::<u32>().ok()) {
            Some(31..=35) => "中国",
            _ => "四国",
        },
        r => r,
    }
}

/// コスモスの公式サイトの地区（URL の地区）を、決算短信の地区の名前にする。
pub fn cosmos_area(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "tohoku" => "東北",
        "kanto" => "関東",
        "chubu" => "中部",
        "kansai" => "関西",
        "chugoku" => "中国",
        "shikoku" => "四国",
        "kyushu" => "九州",
        _ => return None,
    })
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
pub fn classify(e: &OsmElement) -> Option<Chain> {
    if is_trial(e) {
        return Some(Chain::Trial);
    }
    if is_cosmos(e) {
        return Some(Chain::Cosmos);
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

/// コスモス：`shop` が小売の店を表すタグで、`brand:wikidata` がコスモス薬品の項目であるか、
/// `brand` か `name` がチェーンの店名（ドラッグストアコスモス、ディスカウントドラッグコスモス）を含むもの。
/// 名前の空白は無視する。「予定地」「跡地」は開いていない店なので除く。
fn is_cosmos(e: &OsmElement) -> bool {
    const SHOPS: [&str; 6] = [
        "chemist",
        "supermarket",
        "variety_store",
        "convenience",
        "yes",
        "mall",
    ];
    let named = |s: &str| {
        let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        t.contains("ドラッグストアコスモス") || t.contains("ディスカウントドラッグコスモス") || t == "コスモス"
    };
    SHOPS.contains(&e.shop.as_str())
        && !e.name.contains("予定地")
        && !e.name.contains("跡地")
        && (e.wikidata == COSMOS_QID || named(&e.brand) || named(&e.name))
}

fn is_trial(e: &OsmElement) -> bool {
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
pub fn trial_format(e: &OsmElement) -> &'static str {
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

/// 同じチェーンで `radius_km` より近い組を返す。同じ店が node と way の両方で描かれている候補。
pub fn near_pairs<'a>(
    stores: &[(&'a OsmElement, Chain)],
    radius_km: f64,
) -> Vec<(&'a OsmElement, &'a OsmElement, f64)> {
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

/// 二重に描かれた店を落とした結果。`pairs` は近すぎた組（a, b, 距離 [km]、落とした方）。
pub struct Dedup<'a> {
    pub kept: Vec<(&'a OsmElement, Chain)>,
    pub pairs: Vec<(&'a OsmElement, &'a OsmElement, f64, &'a OsmElement)>,
}

/// 同じチェーンで `radius_km` より近い組は同じ店とみなし、一方を落とす。
/// way（建物の輪郭）を残して node を落とし、どちらも同じ種類なら後の方を落とす。
pub fn deduplicate<'a>(stores: &[(&'a OsmElement, Chain)], radius_km: f64) -> Dedup<'a> {
    let mut pairs = Vec::new();
    let mut drop: HashSet<(String, u64)> = HashSet::new();
    for (a, b, d) in near_pairs(stores, radius_km) {
        let victim = if a.kind == "node" && b.kind != "node" {
            a
        } else {
            b
        };
        drop.insert((victim.kind.clone(), victim.id));
        pairs.push((a, b, d, victim));
    }
    let kept = stores
        .iter()
        .filter(|(e, _)| !drop.contains(&(e.kind.clone(), e.id)))
        .copied()
        .collect();
    Dedup { kept, pairs }
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

/// 住所（〒の番号があってもよい）の頭にある都道府県のコード。
pub fn prefecture_of_address(address: &str) -> Option<String> {
    let a = address.trim_start_matches(|c: char| {
        c == '〒' || c == '-' || c.is_ascii_digit() || c.is_whitespace() || c == '　'
    });
    PREFECTURES
        .iter()
        .find(|p| a.starts_with(**p))
        .and_then(|p| prefecture_code(p))
}

/// 西友の公式サイトの店のうち、数えるもの。店名に【閉店】とある店は除く。
/// 【改装中】の店は、一時的に閉めているだけで店舗網に入っているので数える。
/// ロゴが seiyu（西友）、livin（リヴィン、ザ・モール）、trial（トライアル西友）のどれでもよい。
/// トライアルの決算資料は、トライアル西友を西友の数に入れているので、それに合わせる。
pub fn is_open_seiyu(name: &str) -> bool {
    !name.contains("【閉店】")
}

/// 店名の頭の【…】（【改装中】など）。なければ空。
pub fn seiyu_status(name: &str) -> &str {
    name.strip_prefix('【')
        .and_then(|r| r.split_once('】'))
        .map(|(s, _)| s)
        .unwrap_or("")
}

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
    use crate::access::stores::parse_tsv;
    use crate::util::geo::LatLon;

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
        assert_eq!(region(Chain::Coles, "AU-TAS"), "VIC & TAS");
        assert_eq!(region(Chain::Woolworths, "AU-TAS"), "TAS");
        assert_eq!(region(Chain::Woolworths, "AU-ACT"), "NSW & ACT");
    }

    fn el(shop: &str, brand: &str, qid: &str, name: &str) -> OsmElement {
        OsmElement {
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
        let t = |e: OsmElement| classify(&e) == Some(Chain::Trial);
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
        assert_eq!(region(Chain::Trial, "JP-40"), "九州");
        assert_eq!(region(Chain::Trial, "JP-24"), "近畿");
        assert_eq!(region(Chain::Trial, "JP-47"), "九州");
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
        assert_eq!(
            prefecture_of_address("〒120-0012　東京都足立区青井3-1-1").as_deref(),
            Some("JP-13")
        );
        assert_eq!(
            prefecture_of_address("神奈川県川崎市").as_deref(),
            Some("JP-14")
        );
        assert_eq!(prefecture_of_address("〒000-0000　どこか"), None);
        assert!(!is_open_seiyu("【閉店】西友山科店"));
        assert!(is_open_seiyu("【改装中】西友巣鴨店"));
        assert_eq!(seiyu_status("【改装中】西友巣鴨店"), "改装中");
        assert_eq!(seiyu_status("西友青井店"), "");
        assert_eq!(region(Chain::Seiyu, "JP-13"), "関東");
    }
}

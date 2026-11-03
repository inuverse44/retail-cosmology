//! `scripts/osm/fetch.sh` が書いた TSV を読み、Coles と Woolworths の店舗を選び出す。

use crate::{LatLon, great_circle_km};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Chain {
    Coles,
    Woolworths,
}

impl Chain {
    pub fn name(self) -> &'static str {
        match self {
            Chain::Coles => "Coles",
            Chain::Woolworths => "Woolworths",
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
            _ => "other",
        }
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
    pub name: String,
}

/// 選び方：`shop=supermarket` で、`brand` が次のどれか。
/// Coles は Coles・Coles Local・Coles Central、Woolworths は Woolworths・Woolworths Metro。
/// 公式の数が、Coles はスーパー全体、Woolworths はスーパーと Metro の合計なので、それに合わせる。
pub fn classify(e: &Element) -> Option<Chain> {
    if e.shop != "supermarket" {
        return None;
    }
    match e.brand.as_str() {
        "Coles" | "Coles Local" | "Coles Central" => Some(Chain::Coles),
        "Woolworths" | "Woolworths Metro" => Some(Chain::Woolworths),
        _ => None,
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

//! ResourceAccess：店のカタログを読み書きする。
//! OSM の TSV（`scripts/osm/fetch.sh` が書く）、トライアルの公式サイトの JSON（`scripts/trial_site/fetch.sh` が書く）、
//! 選び出した店の CSV（`*_clean_*.csv`）と、二重とみなした組の CSV。

use std::fmt::Write as _;
use std::path::Path;

use crate::contract::{OsmElement, SiteStore, Store};
use crate::util::geo::LatLon;

/// OSM の TSV のファイルを読む。
pub fn read_osm_tsv(path: &Path) -> std::io::Result<Vec<OsmElement>> {
    Ok(parse_tsv(&std::fs::read_to_string(path)?))
}

/// TSV を読む。`area` の行のあとに続く行に、その州のコードをつける。
/// 州の境界にかかる way は二つの州に出ることがあるので、(種類, id) で最初の一つだけ残す。
pub fn parse_tsv(text: &str) -> Vec<OsmElement> {
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
        out.push(OsmElement {
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

/// フォルダの中の batch_*.json をすべて読む。
pub fn read_site(dir: &Path) -> Result<Vec<SiteStore>, Box<dyn std::error::Error>> {
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

/// 選び出した店を CSV に書く。列は chain,region,state,lat,lon,kind,id,brand,name。
pub fn write_selected(path: &Path, stores: &[Store]) -> std::io::Result<()> {
    let mut csv = String::from("chain,region,state,lat,lon,kind,id,brand,name\n");
    for s in stores {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},\"{}\"",
            s.chain.name(),
            s.region,
            s.state,
            s.pos.lat,
            s.pos.lon,
            s.kind,
            s.id,
            s.brand,
            s.name
        )
        .unwrap();
    }
    std::fs::write(path, csv)
}

/// 選び出した店の CSV から、チェーンの名前と位置を読む。
pub fn read_selected(path: &Path) -> std::io::Result<Vec<(String, LatLon)>> {
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .skip(1)
        .map(|line| {
            let f: Vec<&str> = line.splitn(6, ',').collect();
            (
                f[0].to_string(),
                LatLon {
                    lat: f[3].parse().unwrap(),
                    lon: f[4].parse().unwrap(),
                },
            )
        })
        .collect())
}

/// 二重とみなした組の一行：(チェーン, a, b, 距離 [km], 落とした方)
pub type NearPairRow<'a> = (&'a str, &'a OsmElement, &'a OsmElement, f64, &'a OsmElement);

/// 二重とみなした組を CSV に書く。
pub fn write_near_pairs(path: &Path, rows: &[NearPairRow]) -> std::io::Result<()> {
    let mut csv = String::from("chain,distance_m,kind_a,id_a,name_a,kind_b,id_b,name_b,dropped\n");
    for (chain, a, b, d, victim) in rows {
        writeln!(
            csv,
            "{chain},{:.0},{},{},\"{}\",{},{},\"{}\",{}/{}",
            d * 1000.0,
            a.kind,
            a.id,
            a.name,
            b.kind,
            b.id,
            b.name,
            victim.kind,
            victim.id
        )
        .unwrap();
    }
    std::fs::write(path, csv)
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
}

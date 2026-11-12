//! ResourceAccess：店のカタログを読み書きする。
//! OSM の TSV（`scripts/osm/fetch.sh` が書く）、トライアルの公式サイトの JSON（`scripts/trial_site/fetch.sh` が書く）、
//! 西友の公式サイトの HTML（`scripts/seiyu_site/fetch.sh` が書く）、
//! コスモスの公式サイトの都道府県ごとの一覧の HTML（ブラウザで保存したもの）と、住所から推定した座標の TSV
//! （`scripts/cosmos_site/geocode.sh` が書く）、
//! 選び出した店の CSV（`*_clean_*.csv`）と、二重とみなした組の CSV。

use std::fmt::Write as _;
use std::path::Path;

use crate::contract::{Geocoded, ListedStore, OsmElement, SiteStore, Store};
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
                address: String::new(),
            });
        }
    }
    Ok(out)
}

/// 西友の公式サイトの HTML を読む。
/// 店舗検索の一覧（searchshop_*.html、searchtrialshop_*.html）の店と、一覧にない店のページ（shop_*.html）。
/// `id` は店のページの名前（一覧の data-link、店のページの題）。
pub fn read_seiyu_site(dir: &Path) -> Result<Vec<SiteStore>, Box<dyn std::error::Error>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "html"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let text = std::fs::read_to_string(&f)?;
        if name.starts_with("shop_") {
            out.push(parse_seiyu_shop_page(&text).ok_or_else(|| format!("{name} を読めない"))?);
        } else {
            out.extend(parse_seiyu_list(&text)?);
        }
    }
    Ok(out)
}

/// `key="値"`（または `key='値'`）の、最初に見つかった値。
fn attr<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    for q in ['"', '\''] {
        let pat = format!("{key}={q}");
        if let Some(i) = s.find(&pat) {
            let rest = &s[i + pat.len()..];
            return rest.find(q).map(|j| &rest[..j]);
        }
    }
    None
}

fn parse_latlng(v: &str) -> Option<LatLon> {
    let (a, b) = v.split_once(',')?;
    Some(LatLon {
        lat: a.trim().parse().ok()?,
        lon: b.trim().parse().ok()?,
    })
}

/// 店舗検索の一覧のページ。店ごとの `<li class="shop_search_individual">` の中に、
/// 住所（shop_search_individual_info）と、地図のボタン（data-logotype・data-title・data-link・data-latlng）がある。
pub fn parse_seiyu_list(text: &str) -> Result<Vec<SiteStore>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for block in text.split("<li class=\"shop_search_individual\">").skip(1) {
        let Some(latlng) = attr(block, "data-latlng") else {
            continue;
        };
        let address = block
            .split_once("shop_search_individual_info\">")
            .and_then(|(_, r)| r.split_once('<'))
            .map(|(a, _)| a.trim().to_string())
            .unwrap_or_default();
        let link = attr(block, "data-link").unwrap_or_default();
        out.push(SiteStore {
            id: link
                .trim_start_matches("/shop/")
                .trim_end_matches('/')
                .to_string(),
            name: attr(block, "data-title").unwrap_or_default().to_string(),
            category: attr(block, "data-logotype").unwrap_or_default().to_string(),
            pos: parse_latlng(latlng).ok_or_else(|| format!("座標を読めない：{latlng}"))?,
            prefecture: String::new(),
            address,
        });
    }
    Ok(out)
}

/// 店のページ。題（`<title>店名 - 店舗詳細 | SEIYU`）、地図の data-brand と data-ll、〒で始まる住所。
pub fn parse_seiyu_shop_page(text: &str) -> Option<SiteStore> {
    let title = text.split_once("<title>")?.1.split_once(" - ")?.0.trim();
    // 地図の要素（data-ll を持つタグ）の頭から読む
    let i = text.find("data-ll=")?;
    let map = &text[text[..i].rfind('<')?..];
    let address = text
        .split_once('〒')?
        .1
        .split(['<', '"'])
        .next()?
        .split('　')
        .take(2)
        .collect::<Vec<_>>()
        .join("　");
    Some(SiteStore {
        id: title.to_string(),
        name: title.to_string(),
        category: attr(map, "data-brand")?.to_string(),
        pos: parse_latlng(attr(map, "data-ll")?)?,
        prefecture: String::new(),
        address: format!("〒{address}"),
    })
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

/// コスモスの公式サイトの、都道府県ごとの店舗一覧のページ（ブラウザで保存した HTML）を読む。
/// 同じ店が二つのファイルに載っていれば一つにする（`id` は店のページの番号）。
pub fn read_cosmos_site(dir: &Path) -> Result<Vec<ListedStore>, Box<dyn std::error::Error>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "html"))
        .collect();
    files.sort();
    let mut out: Vec<ListedStore> = Vec::new();
    for f in files {
        for s in parse_cosmos_list(&std::fs::read_to_string(&f)?) {
            if !out.iter().any(|o| o.id == s.id) {
                out.push(s);
            }
        }
    }
    Ok(out)
}

/// 一覧のページ。店ごとの `<div class="mix shop-list_box …">` の中に、店のページへのリンク
/// （…/shop/<地方>/<県>/<番号>.html）、店名（`<h3>` の中のルビつきの名前）、〒のあとの住所がある。
/// 店名のあとに「オープン予定」とある店は `planned` にする。
pub fn parse_cosmos_list(text: &str) -> Vec<ListedStore> {
    let mut out = Vec::new();
    for block in text.split(r#"<div class="mix shop-list_box"#).skip(1) {
        let Some(href) = attr(block, "href") else {
            continue;
        };
        let mut parts = href.rsplit('/');
        let _file = parts.next();
        let _pref = parts.next();
        let area = parts.next().unwrap_or_default().to_string();
        let id = href
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".html")
            .to_string();
        let h3 = block
            .split_once("<h3>")
            .and_then(|(_, r)| r.split_once("</h3>"))
            .map(|(h, _)| h)
            .unwrap_or_default();
        let planned = h3.contains("オープン予定");
        // ルビの読み（<rt>…</rt>）と、店名のあとの注記（<span class="sub">…</span>）を外してから、タグを消す
        let mut name = String::new();
        let mut rest = h3.split(r#"<span class="sub">"#).next().unwrap_or_default();
        while let Some((a, b)) = rest.split_once("<rt>") {
            name.push_str(a);
            rest = b.split_once("</rt>").map(|(_, c)| c).unwrap_or_default();
        }
        name.push_str(rest);
        let name = strip_tags(&name);
        let address = block
            .split_once(r#"<span class="zip">"#)
            .and_then(|(_, r)| r.split_once("<br>"))
            .and_then(|(_, r)| r.split_once("</dd>"))
            .map(|(a, _)| strip_tags(a).trim().to_string())
            .unwrap_or_default();
        out.push(ListedStore {
            id,
            area,
            name,
            address,
            planned,
        });
    }
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

/// 住所を引く店の一覧を TSV に書く（id、住所）。`scripts/cosmos_site/geocode.sh` がこれを読む。
pub fn write_addresses(path: &Path, stores: &[ListedStore]) -> std::io::Result<()> {
    let mut s = String::new();
    for st in stores {
        let _ = writeln!(s, "{}\t{}", st.id, st.address);
    }
    std::fs::write(path, s)
}

/// `scripts/cosmos_site/geocode.sh` が書いた TSV（id、経度、緯度、当たった住所）を読む。
pub fn read_geocoded(path: &Path) -> std::io::Result<Vec<Geocoded>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4).then_some(())?;
            Some(Geocoded {
                id: f[0].to_string(),
                pos: LatLon {
                    lon: f[1].parse().ok()?,
                    lat: f[2].parse().ok()?,
                },
                matched: f[3].to_string(),
            })
        })
        .collect())
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

    const LIST: &str = r#"<ul><li class="shop_search_individual">
  <a class="shop_search_individual_link" href="x">西友青井店</a>
  <p class="shop_search_individual_info">
    〒120-0012　東京都足立区青井3-1-1        </p>
  <div class="shop_search_individual_map_button" data-logotype="seiyu" data-title="西友青井店" data-link='/shop/西友青井店/' data-latlng="35.77019783046176,139.81713473796844">MAP</div>
</li>
<li class="shop_search_individual">
  <p class="shop_search_individual_info">〒000-0000　どこか</p>
  <div data-logotype="trial" data-title="【10/23(金)オープン】トライアル西友町田店" data-link='/shop/x/' data-latlng="35.5, 139.4">MAP</div>
</li></ul>"#;

    const SHOP: &str = r#"<title>トライアル西友浦安店 - 店舗詳細 | SEIYU</title>
<p>〒279-0002　千葉県浦安市北栄1-14-1　047-354-2611</p>
<div class="shop_information_map js-shop_map pc_appear" data-brand="trial" data-ll="35.6657545,139.8944871"></div>"#;

    #[test]
    fn parse_seiyu_pages() {
        let v = parse_seiyu_list(LIST).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].id, "西友青井店");
        assert_eq!(v[0].category, "seiyu");
        assert_eq!(v[0].address, "〒120-0012　東京都足立区青井3-1-1");
        assert_eq!(v[0].pos.lat, 35.77019783046176);
        assert_eq!(v[1].pos.lon, 139.4);
        let s = parse_seiyu_shop_page(SHOP).unwrap();
        assert_eq!(s.name, "トライアル西友浦安店");
        assert_eq!(s.category, "trial");
        assert_eq!(s.address, "〒279-0002　千葉県浦安市北栄1-14-1");
        assert_eq!((s.pos.lat, s.pos.lon), (35.6657545, 139.8944871));
    }

    #[test]
    fn parse_assigns_state_and_drops_repeats() {
        let e = parse_tsv(SAMPLE);
        assert_eq!(e.len(), 4);
        assert_eq!(e[1].state, "AU-ACT"); // way 11 は最初に出た ACT のまま
        assert_eq!(e[3].state, "AU-NSW");
    }
}

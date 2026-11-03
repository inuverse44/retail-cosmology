//! 使用例：店舗の数を公式の数と比べる。
//!
//! [`osm`]：OSM の店を選び出し、地域ごとに公式の数と比べる（Coles と Woolworths、またはトライアル）。
//! [`trial_site`]：トライアルの公式サイトの店を数え、公式の数と OSM の店と比べる。
//! どちらも、選び出した店の CSV と、ノートに載せる TeX の表を書き、要約の文字列を返す。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::access::{official, results, stores};
use crate::contract::{Chain, OsmElement, Store};
use crate::engine::census::{MATCH_KM, nearest_km, quantile};
use crate::engine::selection::{
    classify, deduplicate, is_trial_format, prefecture_code, region, trial_format,
};
use crate::util::geo::LatLon;

/// 同じチェーンの二点がこれより近ければ、同じ店を二重に描いたものとみなす [km]。
const DUP_RADIUS_KM: f64 = 0.1;

/// トライアルの決算資料の地方（表の順）
const TRIAL_REGIONS: [&str; 7] = [
    "北海道",
    "東北",
    "関東",
    "中部",
    "近畿",
    "中国・四国",
    "九州",
];

/// OSM の店を選び出し、公式の数と比べる。`dataset` は stores（Coles と Woolworths）か trial。
pub fn osm(root: &Path, date: &str, dataset: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (chains, near_name, table_name): (&[Chain], String, &str) = match dataset {
        "stores" => (
            &[Chain::Coles, Chain::Woolworths],
            format!("near_pairs_{date}.csv"),
            "osm_census.tex",
        ),
        "trial" => (
            &[Chain::Trial],
            format!("trial_near_pairs_{date}.csv"),
            "osm_census_jp.tex",
        ),
        _ => return Err("stores か trial を渡す".into()),
    };
    let elements = stores::read_osm_tsv(&root.join(format!("data/osm/{dataset}_{date}.tsv")))
        .map_err(|e| format!("TSV がない。scripts/osm/fetch.sh を先に：{e}"))?;

    // 選ばなかったもの：(shop, brand) ごとの数
    let mut excluded: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut selected: Vec<(&OsmElement, Chain)> = Vec::new();
    for e in &elements {
        match classify(e).filter(|c| chains.contains(c)) {
            Some(c) => selected.push((e, c)),
            None => {
                *excluded
                    .entry((e.shop.clone(), e.brand.clone()))
                    .or_default() += 1
            }
        }
    }

    let dedup = deduplicate(&selected, DUP_RADIUS_KM);
    let rows: Vec<stores::NearPairRow> = dedup
        .pairs
        .iter()
        .map(|(a, b, d, v)| (classify(a).unwrap().name(), *a, *b, *d, *v))
        .collect();

    let mut clean: Vec<Store> = Vec::new();
    let mut osm: BTreeMap<(Chain, &str), usize> = BTreeMap::new();
    for (e, c) in &dedup.kept {
        let reg = region(*c, &e.state);
        *osm.entry((*c, reg)).or_default() += 1;
        clean.push(Store {
            chain: *c,
            region: reg.to_string(),
            state: e.state.clone(),
            pos: e.pos,
            kind: e.kind.clone(),
            id: e.id,
            brand: e.brand.clone(),
            name: e.name.clone(),
        });
    }
    stores::write_selected(
        &root.join(format!("data/osm/{dataset}_clean_{date}.csv")),
        &clean,
    )?;
    stores::write_near_pairs(&root.join(format!("data/osm/{near_name}")), &rows)?;

    // 公式の数と並べる
    let mut report = String::new();
    let mut table = String::new();
    let mut totals: BTreeMap<Chain, (usize, usize)> = BTreeMap::new();
    writeln!(
        report,
        "{:<11} {:<10} {:>5} {:>5} {:>7}",
        "chain", "region", "OSM", "公式", "OSM/公式"
    )?;
    for o in official::read_official(&root.join("reference/official_store_counts.csv"))? {
        let Some(chain) = chains.iter().copied().find(|c| c.name() == o.chain) else {
            continue;
        };
        let n_off = o.count;
        let n_osm = osm.get(&(chain, o.region.as_str())).copied().unwrap_or(0);
        let t = totals.entry(chain).or_default();
        t.0 += n_osm;
        t.1 += n_off;
        writeln!(
            report,
            "{:<11} {:<10} {n_osm:>5} {n_off:>5} {:>7.3}",
            o.chain,
            o.region,
            n_osm as f64 / n_off as f64
        )?;
        writeln!(
            table,
            "{} & {} & {n_osm} & {n_off} & {:.2} \\\\",
            o.chain,
            o.region.replace('&', "\\&"),
            n_osm as f64 / n_off as f64
        )?;
    }
    table.push_str("\\midrule\n");
    for (c, (a, b)) in &totals {
        writeln!(
            report,
            "{:<11} {:<10} {a:>5} {b:>5} {:>7.3}",
            c.name(),
            "計",
            *a as f64 / *b as f64
        )?;
        writeln!(
            table,
            "{} & 計 & {a} & {b} & {:.2} \\\\",
            c.name(),
            *a as f64 / *b as f64
        )?;
    }
    let other: usize = osm
        .iter()
        .filter(|((_, r), _)| *r == "other")
        .map(|(_, n)| n)
        .sum();
    writeln!(report, "\n州が決まらなかった店舗：{other}")?;
    writeln!(
        report,
        "重複とみなして落とした数：{}（{} m 以内の組 {} 個）",
        selected.len() - dedup.kept.len(),
        DUP_RADIUS_KM * 1000.0,
        dedup.pairs.len()
    )?;
    writeln!(report, "選ばなかった要素：")?;
    for ((shop, brand), n) in &excluded {
        writeln!(report, "  {n:>4}  shop={shop:<12} brand={brand}")?;
    }
    if chains.contains(&Chain::Trial) {
        let mut formats: BTreeMap<&str, usize> = BTreeMap::new();
        for (e, _) in &dedup.kept {
            *formats.entry(trial_format(e)).or_default() += 1;
        }
        writeln!(report, "名前からわかる業態：{formats:?}")?;
    }
    results::write_text(
        &root.join(format!("docs/tables/{table_name}")),
        &format!(
            "% cargo run --release --bin osm_census -- {date} {dataset} が書いた表\n\\begin{{tabular}}{{llrrr}}\n\\toprule\nチェーン & 地域 & OSM & 公式 & OSM/公式 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )?;
    Ok(report)
}

/// トライアルの公式サイトの店を数え、公式の数と OSM の店と比べる。
pub fn trial_site(
    root: &Path,
    day: &str,
    osm_date: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let all = stores::read_site(&root.join(format!("data/trial_site/{day}")))
        .map_err(|e| format!("scripts/trial_site/fetch.sh を先に：{e}"))?;
    let mut excluded: BTreeMap<String, usize> = BTreeMap::new();
    let mut site: Vec<Store> = Vec::new();
    for s in &all {
        if !is_trial_format(&s.category) {
            *excluded.entry(s.category.clone()).or_default() += 1;
            continue;
        }
        let code = prefecture_code(&s.prefecture)
            .ok_or_else(|| format!("知らない都道府県：{}", s.prefecture))?;
        let reg = region(Chain::Trial, &code);
        site.push(Store {
            chain: Chain::Trial,
            region: reg.to_string(),
            state: code,
            pos: s.pos,
            kind: "site".into(),
            id: 0,
            brand: s.category.clone(),
            name: format!("{}（{}）", s.name, s.id),
        });
    }

    let osm: Vec<LatLon> =
        stores::read_selected(&root.join(format!("data/osm/trial_clean_{osm_date}.csv")))
            .map_err(|e| format!("osm_census を先に：{e}"))?
            .into_iter()
            .map(|(_, p)| p)
            .collect();
    let site_pos: Vec<LatLon> = site.iter().map(|s| s.pos).collect();

    // 地方ごとの数
    let official: BTreeMap<String, usize> =
        official::read_official(&root.join("reference/official_store_counts.csv"))?
            .into_iter()
            .filter(|o| o.chain == "TRIAL")
            .map(|o| (o.region, o.count))
            .collect();
    let mut n_site: BTreeMap<&str, usize> = BTreeMap::new();
    let mut n_found: BTreeMap<&str, usize> = BTreeMap::new();
    let mut offsets = Vec::new();
    for s in &site {
        *n_site.entry(s.region.as_str()).or_default() += 1;
        let d = nearest_km(&s.pos, &osm);
        if d < MATCH_KM {
            *n_found.entry(s.region.as_str()).or_default() += 1;
            offsets.push(d * 1000.0);
        }
    }
    let osm_unmatched = osm
        .iter()
        .filter(|p| nearest_km(p, &site_pos) >= MATCH_KM)
        .count();

    let mut report = String::new();
    writeln!(
        report,
        "{:<10} {:>5} {:>5} {:>8} {:>10} {:>8}",
        "地方", "サイト", "公式", "サイト/公式", "OSM で一致", "OSM の割合"
    )?;
    let mut table = String::new();
    let (mut ts, mut to, mut tf) = (0, 0, 0);
    for r in TRIAL_REGIONS {
        let (s, o, f) = (
            n_site.get(r).copied().unwrap_or(0),
            official[r],
            n_found.get(r).copied().unwrap_or(0),
        );
        ts += s;
        to += o;
        tf += f;
        writeln!(
            report,
            "{r:<10} {s:>5} {o:>5} {:>8.3} {f:>10} {:>8.3}",
            s as f64 / o as f64,
            f as f64 / s as f64
        )?;
        writeln!(
            table,
            "{r} & {s} & {o} & {:.2} & {f} & {:.2} \\\\",
            s as f64 / o as f64,
            f as f64 / s as f64
        )?;
    }
    writeln!(
        report,
        "{:<10} {ts:>5} {to:>5} {:>8.3} {tf:>10} {:>8.3}",
        "計",
        ts as f64 / to as f64,
        tf as f64 / ts as f64
    )?;
    writeln!(
        table,
        "\\midrule\n計 & {ts} & {to} & {:.2} & {tf} & {:.2} \\\\",
        ts as f64 / to as f64,
        tf as f64 / ts as f64
    )?;
    writeln!(
        report,
        "\nOSM の位置のずれ（一致した {} 店）：中央値 {:.0} m、90% 点 {:.0} m",
        offsets.len(),
        quantile(&mut offsets.clone(), 0.5),
        quantile(&mut offsets, 0.9)
    )?;
    writeln!(
        report,
        "OSM の店のうち、{MATCH_KM} km 以内にサイトの店がないもの：{osm_unmatched} / {}",
        osm.len()
    )?;
    writeln!(report, "除いた業態：{excluded:?}")?;

    stores::write_selected(
        &root.join(format!("data/trial_site/trial_site_clean_{day}.csv")),
        &site,
    )?;
    results::write_text(
        &root.join("docs/tables/trial_site_census.tex"),
        &format!(
            "% cargo run --release --bin trial_census -- {day} {osm_date} が書いた表\n\\begin{{tabular}}{{lrrrrr}}\n\\toprule\n地方 & サイト & 公式 & サイト/公式 & OSM で一致 & OSM の割合 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )?;
    Ok(report)
}

//! 使用例：店舗の数を公式の数と比べる。
//!
//! [`osm`]：OSM の店を選び出し、地域ごとに公式の数と比べる（Coles と Woolworths、またはトライアル）。
//! [`trial_site`]：トライアルの公式サイトの店を数え、公式の数と OSM の店と比べる。
//! [`seiyu_site`]：西友の公式サイトの店を数え、公式の数と、トライアルのサイトに載るトライアル西友と比べる。
//! どちらも、選び出した店の CSV と、ノートに載せる TeX の表を書き、要約の文字列を返す。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::access::{official, results, stores};
use crate::contract::{Chain, OsmElement, Store};
use crate::engine::census::{MATCH_KM, nearest_km, quantile};
use crate::engine::selection::{
    classify, deduplicate, is_open_seiyu, is_trial_format, prefecture_code, prefecture_of_address,
    region, seiyu_status, trial_format,
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

/// OSM の店を選び出し、公式の数と比べる。`dataset` は stores（Coles と Woolworths）、trial、cosmos。
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
        "cosmos" => (
            &[Chain::Cosmos],
            format!("cosmos_near_pairs_{date}.csv"),
            "osm_census_cosmos.tex",
        ),
        _ => return Err("stores か trial か cosmos を渡す".into()),
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

/// 西友の公式サイトの店を数え、地方ごとに公式の数と比べる。
/// あわせて、トライアルの公式サイト（`trial_day` に取ったもの）に業態「トライアル西友」として載る店が、
/// 西友のサイトのどの店にあたるかを、いちばん近い店までの距離で確かめる（同じ店を二社に数えていないか、座標が合うか）。
pub fn seiyu_site(
    root: &Path,
    day: &str,
    trial_day: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let all = stores::read_seiyu_site(&root.join(format!("data/seiyu_site/{day}")))
        .map_err(|e| format!("scripts/seiyu_site/fetch.sh を先に：{e}"))?;
    let mut report = String::new();
    let mut seen = std::collections::HashSet::new();
    let mut site: Vec<Store> = Vec::new();
    let mut status: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut logos: BTreeMap<String, usize> = BTreeMap::new();
    for s in &all {
        if !seen.insert(s.id.clone()) {
            writeln!(report, "二度出た店（一つ目だけ残す）：{}", s.id)?;
            continue;
        }
        let st = seiyu_status(&s.name);
        if !st.is_empty() {
            status
                .entry(st.to_string())
                .or_default()
                .push(s.name.clone());
        }
        if !is_open_seiyu(&s.name) {
            continue;
        }
        *logos.entry(s.category.clone()).or_default() += 1;
        let code = prefecture_of_address(&s.address)
            .ok_or_else(|| format!("住所から都道府県がわからない：{} {}", s.name, s.address))?;
        site.push(Store {
            chain: Chain::Seiyu,
            region: region(Chain::Seiyu, &code).to_string(),
            state: code,
            pos: s.pos,
            kind: "site".into(),
            id: 0,
            brand: s.category.clone(),
            name: s.name.clone(),
        });
    }

    let official: BTreeMap<String, usize> =
        official::read_official(&root.join("reference/official_store_counts.csv"))?
            .into_iter()
            .filter(|o| o.chain == "SEIYU")
            .map(|o| (o.region, o.count))
            .collect();
    let mut n_site: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &site {
        *n_site.entry(s.region.as_str()).or_default() += 1;
    }
    writeln!(
        report,
        "{:<10} {:>5} {:>5} {:>8}",
        "地方", "サイト", "公式", "サイト/公式"
    )?;
    let mut table = String::new();
    let (mut ts, mut to) = (0, 0);
    for r in TRIAL_REGIONS {
        let (s, o) = (
            n_site.get(r).copied().unwrap_or(0),
            official.get(r).copied().unwrap_or(0),
        );
        if s == 0 && o == 0 {
            continue;
        }
        ts += s;
        to += o;
        writeln!(report, "{r:<10} {s:>5} {o:>5} {:>8.3}", s as f64 / o as f64)?;
        writeln!(table, "{r} & {s} & {o} & {:.2} \\\\", s as f64 / o as f64)?;
    }
    writeln!(
        report,
        "{:<10} {ts:>5} {to:>5} {:>8.3}",
        "計",
        ts as f64 / to as f64
    )?;
    writeln!(
        table,
        "\\midrule\n計 & {ts} & {to} & {:.2} \\\\",
        ts as f64 / to as f64
    )?;
    let others: Vec<&str> = n_site
        .keys()
        .copied()
        .filter(|r| !official.contains_key(*r))
        .collect();
    if !others.is_empty() {
        writeln!(report, "公式の数がない地方にある店：{others:?}")?;
    }
    writeln!(report, "ロゴの種類（数えた店）：{logos:?}")?;
    for (st, names) in &status {
        writeln!(report, "【{st}】{} 店：{names:?}", names.len())?;
    }

    // トライアルのサイトのトライアル西友と照らす
    let site_pos: Vec<LatLon> = site.iter().map(|s| s.pos).collect();
    let trial_dir = root.join(format!("data/trial_site/{trial_day}"));
    let ts_trial: Vec<_> = stores::read_site(&trial_dir)
        .map_err(|e| format!("scripts/trial_site/fetch.sh を先に：{e}"))?
        .into_iter()
        .filter(|s| s.category == "トライアル西友")
        .collect();
    let mut far = Vec::new();
    let mut d_m: Vec<f64> = Vec::new();
    for t in &ts_trial {
        let d = nearest_km(&t.pos, &site_pos);
        d_m.push(d * 1000.0);
        if d >= MATCH_KM {
            far.push(format!("{}（{:.2} km）", t.name, d));
        }
    }
    writeln!(
        report,
        "トライアルのサイトのトライアル西友 {} 店：西友のサイトの店までの距離 最大 {:.0} m。{MATCH_KM} km より遠い店 {far:?}",
        ts_trial.len(),
        d_m.iter().copied().fold(0.0, f64::max)
    )?;

    stores::write_selected(
        &root.join(format!("data/seiyu_site/seiyu_site_clean_{day}.csv")),
        &site,
    )?;
    results::write_text(
        &root.join("docs/tables/seiyu_site_census.tex"),
        &format!(
            "% cargo run --release --bin seiyu_census -- {day} {trial_day} が書いた表\n\\begin{{tabular}}{{lrrr}}\n\\toprule\n地方 & サイト & 公式 & サイト/公式 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )?;
    Ok(report)
}

/// コスモスの決算短信の地区（表の順）
const COSMOS_REGIONS: [&str; 6] = ["関東", "中部", "関西", "中国", "四国", "九州"];

/// 推定した座標がどの細かさで当たったか。住所検索が返した住所が「番」か「号」で終われば街区まで、
/// そうでなければ町・字（大字）までしか当たっていない。
fn geocode_level(matched: &str) -> &'static str {
    if matched.ends_with('番') || matched.ends_with('号') {
        "街区"
    } else {
        "町字"
    }
}

/// コスモスの公式サイトの店を数え、公式の数と OSM の店と比べる（#51）。
/// サイトには座標がないので、住所から推定した座標（scripts/cosmos_site/geocode.sh）を使う。
/// まず住所の一覧（data/cosmos_site/<日>/addresses.tsv）を書き、推定がまだそろっていなければそこで止まる。
pub fn cosmos_site(
    root: &Path,
    day: &str,
    osm_date: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let dir = root.join(format!("data/cosmos_site/{day}"));
    let all = stores::read_cosmos_site(&dir)?;
    let (open, planned): (Vec<_>, Vec<_>) = all.into_iter().partition(|s| !s.planned);
    stores::write_addresses(&dir.join("addresses.tsv"), &open)?;
    let geo: BTreeMap<String, crate::contract::Geocoded> =
        stores::read_geocoded(&dir.join("geocode.tsv"))
            .unwrap_or_default()
            .into_iter()
            .map(|g| (g.id.clone(), g))
            .collect();
    let missing = open.iter().filter(|s| !geo.contains_key(&s.id)).count();
    if missing > 0 {
        return Err(format!(
            "{missing} 店の座標がまだない。scripts/cosmos_site/geocode.sh {day} を先に（住所の一覧は addresses.tsv に書いた）"
        )
        .into());
    }

    let mut site: Vec<Store> = Vec::new();
    let mut levels: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &open {
        let g = &geo[&s.id];
        let reg = crate::engine::selection::cosmos_area(&s.area)
            .ok_or_else(|| format!("知らない地区：{}", s.area))?;
        let code = prefecture_of_address(&s.address)
            .ok_or_else(|| format!("住所から都道府県がわからない：{}", s.address))?;
        let level = geocode_level(&g.matched);
        *levels.entry(level).or_default() += 1;
        site.push(Store {
            chain: Chain::Cosmos,
            region: reg.to_string(),
            state: code,
            pos: g.pos,
            kind: "site".into(),
            id: 0,
            brand: level.into(),
            name: format!("{}（{}）", s.name, s.id),
        });
    }

    let osm: Vec<LatLon> =
        stores::read_selected(&root.join(format!("data/osm/cosmos_clean_{osm_date}.csv")))
            .map_err(|e| format!("osm_census を先に：{e}"))?
            .into_iter()
            .map(|(_, p)| p)
            .collect();
    let site_pos: Vec<LatLon> = site.iter().map(|s| s.pos).collect();
    // OSM の店からいちばん近いサイトの店までの距離。OSM の位置を正しいとみなせば、推定した座標のずれの目安になる
    let offsets: Vec<f64> = osm
        .iter()
        .map(|p| nearest_km(p, &site_pos) * 1000.0)
        .collect();
    let mut offsets_by_level: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for p in &osm {
        let (i, d) = site_pos
            .iter()
            .enumerate()
            .map(|(i, q)| (i, crate::util::geo::great_circle_km(p, q)))
            .fold((0, f64::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
        offsets_by_level
            .entry(if site[i].brand == "街区" { "街区" } else { "町字" })
            .or_default()
            .push(d * 1000.0);
    }

    let official: BTreeMap<String, usize> =
        official::read_official(&root.join("reference/official_store_counts.csv"))?
            .into_iter()
            .filter(|o| o.chain == "COSMOS")
            .map(|o| (o.region, o.count))
            .collect();
    let mut n_site: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &site {
        *n_site.entry(s.region.as_str()).or_default() += 1;
    }
    let mut report = String::new();
    let mut table = String::new();
    writeln!(report, "{:<6} {:>5} {:>5} {:>8}", "地区", "サイト", "公式", "サイト/公式")?;
    let (mut ts, mut to) = (0, 0);
    for r in COSMOS_REGIONS {
        let (s, o) = (n_site.get(r).copied().unwrap_or(0), official[r]);
        ts += s;
        to += o;
        writeln!(report, "{r:<6} {s:>5} {o:>5} {:>8.3}", s as f64 / o as f64)?;
        writeln!(table, "{r} & {s} & {o} & {:.2} \\\\", s as f64 / o as f64)?;
    }
    for (r, s) in &n_site {
        if !COSMOS_REGIONS.contains(r) {
            ts += s;
            writeln!(report, "{r:<6} {s:>5}     -        -")?;
            writeln!(table, "{r} & {s} & --- & --- \\\\")?;
        }
    }
    writeln!(report, "{:<6} {ts:>5} {to:>5} {:>8.3}", "計", ts as f64 / to as f64)?;
    writeln!(
        table,
        "\\midrule\n計 & {ts} & {to} & {:.2} \\\\",
        ts as f64 / to as f64
    )?;
    writeln!(report, "\nオープン予定で除いた店：{}", planned.len())?;
    writeln!(report, "座標の細かさ：{levels:?}")?;
    writeln!(
        report,
        "OSM の店（{} 店）からいちばん近いサイトの店まで：中央値 {:.0} m、90% 点 {:.0} m、1 km 以内 {} 店",
        osm.len(),
        quantile(&mut offsets.clone(), 0.5),
        quantile(&mut offsets.clone(), 0.9),
        offsets.iter().filter(|d| **d < 1000.0).count()
    )?;
    for (lv, v) in offsets_by_level.iter_mut() {
        writeln!(
            report,
            "  いちばん近いサイトの店が{lv}まで当たったもの（{} 店）：中央値 {:.0} m、90% 点 {:.0} m",
            v.len(),
            quantile(&mut v.clone(), 0.5),
            quantile(v, 0.9)
        )?;
    }
    stores::write_selected(
        &root.join(format!("data/cosmos_site/cosmos_site_clean_{day}.csv")),
        &site,
    )?;
    results::write_text(
        &root.join("docs/tables/cosmos_site_census.tex"),
        &format!(
            "% cargo run --release --bin cosmos_census -- {day} {osm_date} が書いた表\n\\begin{{tabular}}{{lrrr}}\n\\toprule\n地区 & サイト & 公式 & サイト/公式 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )?;
    Ok(report)
}

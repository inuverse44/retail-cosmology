//! OSM の店舗を選び出し、公式の店舗数と地域ごとに比べる。
//!
//! cargo run --release --bin osm_census -- 2026-11-01 [stores|trial]
//!   stores：Coles と Woolworths（既定）、trial：トライアル
//!
//! 読むもの：data/osm/<名前>_<日付>.tsv、reference/official_store_counts.csv
//! 書くもの：data/osm/<名前>_clean_<日付>.csv（重複を除いた店舗）
//!           data/osm/near_pairs_<日付>.csv（stores）、data/osm/trial_near_pairs_<日付>.csv（trial）
//!           docs/tables/osm_census.tex（stores）、docs/tables/osm_census_jp.tex（trial）

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use retail_cosmology::osm::{Chain, Element, classify, near_pairs, parse_tsv, trial_format};

/// 同じチェーンの二点がこれより近ければ、同じ店を二重に描いたものとみなす [km]。
const DUP_RADIUS_KM: f64 = 0.1;

fn main() {
    let date = std::env::args().nth(1).expect("日付（YYYY-MM-DD）を渡す");
    let ds = std::env::args().nth(2).unwrap_or_else(|| "stores".into());
    let (chains, near_name, table_name): (&[Chain], String, &str) = match ds.as_str() {
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
        _ => panic!("stores か trial を渡す"),
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tsv = fs::read_to_string(root.join(format!("data/osm/{ds}_{date}.tsv")))
        .expect("TSV がない。scripts/osm/fetch.sh を先に");
    let elements = parse_tsv(&tsv);

    // 選ばなかったもの：(shop, brand) ごとの数
    let mut excluded: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut stores: Vec<(&Element, Chain)> = Vec::new();
    for e in &elements {
        match classify(e).filter(|c| chains.contains(c)) {
            Some(c) => stores.push((e, c)),
            None => {
                *excluded
                    .entry((e.shop.clone(), e.brand.clone()))
                    .or_default() += 1
            }
        }
    }

    // 近すぎる組は、way（建物の輪郭）を残して node を落とす。どちらも同じ種類なら後の方を落とす。
    let pairs = near_pairs(&stores, DUP_RADIUS_KM);
    let mut drop: HashSet<(String, u64)> = HashSet::new();
    let mut near_csv =
        String::from("chain,distance_m,kind_a,id_a,name_a,kind_b,id_b,name_b,dropped\n");
    for (a, b, d) in &pairs {
        let victim = if a.kind == "node" && b.kind != "node" {
            a
        } else {
            b
        };
        drop.insert((victim.kind.clone(), victim.id));
        let chain = classify(a).unwrap().name();
        writeln!(
            near_csv,
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
    let clean: Vec<_> = stores
        .iter()
        .filter(|(e, _)| !drop.contains(&(e.kind.clone(), e.id)))
        .collect();

    let mut clean_csv = String::from("chain,region,state,lat,lon,kind,id,brand,name\n");
    let mut osm: BTreeMap<(Chain, &str), usize> = BTreeMap::new();
    for (e, c) in &clean {
        let region = c.region(&e.state);
        *osm.entry((*c, region)).or_default() += 1;
        writeln!(
            clean_csv,
            "{},{region},{},{},{},{},{},{},\"{}\"",
            c.name(),
            e.state,
            e.pos.lat,
            e.pos.lon,
            e.kind,
            e.id,
            e.brand,
            e.name
        )
        .unwrap();
    }
    fs::write(
        root.join(format!("data/osm/{ds}_clean_{date}.csv")),
        clean_csv,
    )
    .unwrap();
    fs::write(root.join(format!("data/osm/{near_name}")), near_csv).unwrap();

    // 公式の数と並べる
    let official = fs::read_to_string(root.join("reference/official_store_counts.csv")).unwrap();
    let mut table = String::new();
    let mut totals: BTreeMap<Chain, (usize, usize)> = BTreeMap::new();
    println!(
        "{:<11} {:<10} {:>5} {:>5} {:>7}",
        "chain", "region", "OSM", "公式", "OSM/公式"
    );
    for line in official.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let Some(chain) = chains.iter().copied().find(|c| c.name() == f[0]) else {
            continue;
        };
        let n_off: usize = f[2].parse().unwrap();
        let n_osm = osm.get(&(chain, f[1])).copied().unwrap_or(0);
        let t = totals.entry(chain).or_default();
        t.0 += n_osm;
        t.1 += n_off;
        println!(
            "{:<11} {:<10} {n_osm:>5} {n_off:>5} {:>7.3}",
            f[0],
            f[1],
            n_osm as f64 / n_off as f64
        );
        writeln!(
            table,
            "{} & {} & {n_osm} & {n_off} & {:.2} \\\\",
            f[0],
            f[1].replace('&', "\\&"),
            n_osm as f64 / n_off as f64
        )
        .unwrap();
    }
    table.push_str("\\midrule\n");
    for (c, (a, b)) in &totals {
        println!(
            "{:<11} {:<10} {a:>5} {b:>5} {:>7.3}",
            c.name(),
            "計",
            *a as f64 / *b as f64
        );
        writeln!(
            table,
            "{} & 計 & {a} & {b} & {:.2} \\\\",
            c.name(),
            *a as f64 / *b as f64
        )
        .unwrap();
    }
    let other: usize = osm
        .iter()
        .filter(|((_, r), _)| *r == "other")
        .map(|(_, n)| n)
        .sum();
    println!("\n州が決まらなかった店舗：{other}");
    println!(
        "重複とみなして落とした数：{}（{} m 以内の組 {} 個）",
        drop.len(),
        DUP_RADIUS_KM * 1000.0,
        pairs.len()
    );
    println!("選ばなかった要素：");
    for ((shop, brand), n) in &excluded {
        println!("  {n:>4}  shop={shop:<12} brand={brand}");
    }
    if chains.contains(&Chain::Trial) {
        let mut formats: BTreeMap<&str, usize> = BTreeMap::new();
        for (e, _) in &clean {
            *formats.entry(trial_format(e)).or_default() += 1;
        }
        println!("名前からわかる業態：{formats:?}");
    }
    fs::create_dir_all(root.join("docs/tables")).unwrap();
    fs::write(
        root.join(format!("docs/tables/{table_name}")),
        format!(
            "% cargo run --release --bin osm_census -- {date} {ds} が書いた表\n\\begin{{tabular}}{{llrrr}}\n\\toprule\nチェーン & 地域 & OSM & 公式 & OSM/公式 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )
    .unwrap();
}

//! トライアルの公式サイトの店舗を数え、決算資料の店舗数と OSM の店舗と比べる。
//!
//! cargo run --release --bin trial_census -- 2026-11-03 2026-11-01
//!   1 つ目：サイトから取った日（data/trial_site/<日>/）、2 つ目：OSM の日付（data/osm/trial_clean_<日付>.csv）
//!
//! 書くもの：data/trial_site/trial_site_clean_<日>.csv（xi が読む。列は OSM の *_clean と同じ）
//!           docs/tables/trial_site_census.tex（ノートに載せる表）

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use retail_cosmology::osm::Chain;
use retail_cosmology::trial_site::{is_trial_format, prefecture_code, read_dir};
use retail_cosmology::{LatLon, great_circle_km};

/// サイトの店と OSM の店を同じ店とみなす距離 [km]
const MATCH_KM: f64 = 0.3;

const REGIONS: [&str; 7] = [
    "北海道",
    "東北",
    "関東",
    "中部",
    "近畿",
    "中国・四国",
    "九州",
];

fn quantile(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn main() {
    let day = std::env::args()
        .nth(1)
        .expect("サイトから取った日（YYYY-MM-DD）");
    let osm_date = std::env::args().nth(2).expect("OSM の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let all = read_dir(&root.join(format!("data/trial_site/{day}")))
        .expect("scripts/trial_site/fetch.sh を先に");
    let mut excluded: BTreeMap<String, usize> = BTreeMap::new();
    let mut site: Vec<(String, &str, LatLon, String, String)> = Vec::new(); // (都道府県コード, 地方, 位置, 業態, 店名)
    for s in &all {
        if !is_trial_format(&s.category) {
            *excluded.entry(s.category.clone()).or_default() += 1;
            continue;
        }
        let code = prefecture_code(&s.prefecture)
            .unwrap_or_else(|| panic!("知らない都道府県：{}", s.prefecture));
        let region = Chain::Trial.region(&code);
        site.push((
            code,
            region,
            s.pos,
            s.category.clone(),
            format!("{}（{}）", s.name, s.id),
        ));
    }

    // OSM の店
    let osm_csv = fs::read_to_string(root.join(format!("data/osm/trial_clean_{osm_date}.csv")))
        .expect("osm_census を先に");
    let osm: Vec<LatLon> = osm_csv
        .lines()
        .skip(1)
        .map(|l| {
            let f: Vec<&str> = l.splitn(6, ',').collect();
            LatLon {
                lat: f[3].parse().unwrap(),
                lon: f[4].parse().unwrap(),
            }
        })
        .collect();
    let nearest = |p: &LatLon, set: &[LatLon]| {
        set.iter()
            .map(|q| great_circle_km(p, q))
            .fold(f64::MAX, f64::min)
    };
    let site_pos: Vec<LatLon> = site.iter().map(|s| s.2).collect();

    // 地方ごとの数
    let official: BTreeMap<String, usize> =
        fs::read_to_string(root.join("reference/official_store_counts.csv"))
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("TRIAL,"))
            .map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                (f[1].to_string(), f[2].parse().unwrap())
            })
            .collect();
    let mut n_site: BTreeMap<&str, usize> = BTreeMap::new();
    let mut n_found: BTreeMap<&str, usize> = BTreeMap::new();
    let mut offsets = Vec::new();
    for s in &site {
        *n_site.entry(s.1).or_default() += 1;
        let d = nearest(&s.2, &osm);
        if d < MATCH_KM {
            *n_found.entry(s.1).or_default() += 1;
            offsets.push(d * 1000.0);
        }
    }
    let osm_unmatched = osm
        .iter()
        .filter(|p| nearest(p, &site_pos) >= MATCH_KM)
        .count();

    println!(
        "{:<10} {:>5} {:>5} {:>8} {:>10} {:>8}",
        "地方", "サイト", "公式", "サイト/公式", "OSM で一致", "OSM の割合"
    );
    let mut table = String::new();
    let (mut ts, mut to, mut tf) = (0, 0, 0);
    for r in REGIONS {
        let (s, o, f) = (
            n_site.get(r).copied().unwrap_or(0),
            official[r],
            n_found.get(r).copied().unwrap_or(0),
        );
        ts += s;
        to += o;
        tf += f;
        println!(
            "{r:<10} {s:>5} {o:>5} {:>8.3} {f:>10} {:>8.3}",
            s as f64 / o as f64,
            f as f64 / s as f64
        );
        writeln!(
            table,
            "{r} & {s} & {o} & {:.2} & {f} & {:.2} \\\\",
            s as f64 / o as f64,
            f as f64 / s as f64
        )
        .unwrap();
    }
    println!(
        "{:<10} {ts:>5} {to:>5} {:>8.3} {tf:>10} {:>8.3}",
        "計",
        ts as f64 / to as f64,
        tf as f64 / ts as f64
    );
    writeln!(
        table,
        "\\midrule\n計 & {ts} & {to} & {:.2} & {tf} & {:.2} \\\\",
        ts as f64 / to as f64,
        tf as f64 / ts as f64
    )
    .unwrap();
    println!(
        "\nOSM の位置のずれ（一致した {} 店）：中央値 {:.0} m、90% 点 {:.0} m",
        offsets.len(),
        quantile(&mut offsets.clone(), 0.5),
        quantile(&mut offsets, 0.9)
    );
    println!(
        "OSM の店のうち、{MATCH_KM} km 以内にサイトの店がないもの：{osm_unmatched} / {}",
        osm.len()
    );
    println!("除いた業態：{excluded:?}");

    let mut csv = String::from("chain,region,state,lat,lon,kind,id,brand,name\n");
    for (code, region, p, cat, name) in &site {
        writeln!(
            csv,
            "TRIAL,{region},{code},{},{},site,0,{cat},\"{name}\"",
            p.lat, p.lon
        )
        .unwrap();
    }
    fs::write(
        root.join(format!("data/trial_site/trial_site_clean_{day}.csv")),
        csv,
    )
    .unwrap();
    fs::write(
        root.join("docs/tables/trial_site_census.tex"),
        format!(
            "% cargo run --release --bin trial_census -- {day} {osm_date} が書いた表\n\\begin{{tabular}}{{lrrrrr}}\n\\toprule\n地方 & サイト & 公式 & サイト/公式 & OSM で一致 & OSM の割合 \\\\\n\\midrule\n{table}\\bottomrule\n\\end{{tabular}}\n"
        ),
    )
    .unwrap();
}

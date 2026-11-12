//! Client：コスモスの公式サイトの店舗を数え、決算短信の店舗数と OSM の店舗と比べる（#51）。
//!
//! cargo run --release --bin cosmos_census -- 2026-11-12 2026-11-01
//!   1 つ目：サイトのページを保存した日（data/cosmos_site/<日>/ の HTML）、2 つ目：OSM の日付（data/osm/cosmos_clean_<日付>.csv）
//!
//! 1 回目は住所の一覧（data/cosmos_site/<日>/addresses.tsv）を書いて止まる。
//! scripts/cosmos_site/geocode.sh <日> で座標を推定してから、もう一度走らせる。
//! 書くもの：data/cosmos_site/cosmos_site_clean_<日>.csv（xi が読む）、docs/tables/cosmos_site_census.tex

use std::path::PathBuf;

fn main() {
    let day = std::env::args()
        .nth(1)
        .expect("サイトのページを保存した日（YYYY-MM-DD）");
    let osm_date = std::env::args().nth(2).expect("OSM の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::census::cosmos_site(&root, &day, &osm_date) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

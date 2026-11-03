//! Client：トライアルの公式サイトの店舗を数え、決算資料の店舗数と OSM の店舗と比べる。
//!
//! cargo run --release --bin trial_census -- 2026-11-03 2026-11-01
//!   1 つ目：サイトから取った日（data/trial_site/<日>/）、2 つ目：OSM の日付（data/osm/trial_clean_<日付>.csv）
//!
//! 書くもの：data/trial_site/trial_site_clean_<日>.csv（xi が読む。列は OSM の *_clean と同じ）
//!           docs/tables/trial_site_census.tex（ノートに載せる表）

use std::path::PathBuf;

fn main() {
    let day = std::env::args()
        .nth(1)
        .expect("サイトから取った日（YYYY-MM-DD）");
    let osm_date = std::env::args().nth(2).expect("OSM の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::census::trial_site(&root, &day, &osm_date) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

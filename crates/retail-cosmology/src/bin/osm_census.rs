//! Client：OSM の店舗を選び出し、公式の店舗数と地域ごとに比べる。
//!
//! cargo run --release --bin osm_census -- 2026-11-01 [stores|trial]
//!   stores：Coles と Woolworths（既定）、trial：トライアル
//!
//! 読むもの：data/osm/<名前>_<日付>.tsv、reference/official_store_counts.csv
//! 書くもの：data/osm/<名前>_clean_<日付>.csv（重複を除いた店舗）
//!           data/osm/near_pairs_<日付>.csv（stores）、data/osm/trial_near_pairs_<日付>.csv（trial）
//!           docs/tables/osm_census.tex（stores）、docs/tables/osm_census_jp.tex（trial）

use std::path::PathBuf;

fn main() {
    let date = std::env::args().nth(1).expect("日付（YYYY-MM-DD）を渡す");
    let dataset = std::env::args().nth(2).unwrap_or_else(|| "stores".into());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::census::osm(&root, &date, &dataset) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

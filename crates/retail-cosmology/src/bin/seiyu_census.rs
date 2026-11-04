//! Client：西友の公式サイトの店舗を数え、決算資料の店舗数と、トライアルのサイトのトライアル西友と比べる。
//!
//! cargo run --release --bin seiyu_census -- 2026-11-04 2026-11-03
//!   1 つ目：西友のサイトから取った日（data/seiyu_site/<日>/）
//!   2 つ目：トライアルのサイトから取った日（data/trial_site/<日>/）
//!
//! 書くもの：data/seiyu_site/seiyu_site_clean_<日>.csv（xi が読む。列は OSM の *_clean と同じ）
//!           docs/tables/seiyu_site_census.tex（ノートに載せる表）

use std::path::PathBuf;

fn main() {
    let day = std::env::args()
        .nth(1)
        .expect("西友のサイトから取った日（YYYY-MM-DD）");
    let trial_day = std::env::args()
        .nth(2)
        .expect("トライアルのサイトから取った日（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::census::seiyu_site(&root, &day, &trial_day) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

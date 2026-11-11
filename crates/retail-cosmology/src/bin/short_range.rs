//! Client：8 km より短い距離の選択関数つきの ξ_LS と相互相関を、帰無仮説の模擬データと比べる（#50）。
//!
//! cargo run --release --bin short_range -- 2026-11-03
//!   引数：日本の xi の日付
//!
//! 読むもの：data/xi/xi_jp_<日付>_crossfp.csv（実際の値）、
//!           data/xi/null_mocks_cross_TRIALxSEIYU_<R>[_density][_grid][_keep…]_bins.csv（scripts/null_mocks_short.sh）
//! 書くもの：docs/tables/short_range.tex（ノートに載せる表）

use std::path::PathBuf;

fn main() {
    let jp = std::env::args()
        .nth(1)
        .expect("日本の xi の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::short_range::run(&root, &jp) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

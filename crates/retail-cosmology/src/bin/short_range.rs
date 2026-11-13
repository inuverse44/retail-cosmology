//! Client：8 km より短い距離の選択関数つきの ξ_LS と相互相関を、帰無仮説の模擬データと比べる（#50）。
//!
//! cargo run --release --bin short_range -- 2026-11-03 [cosmos|walmart]
//!   1 つ目：xi の日付。2 つ目に cosmos を渡すと、トライアルとコスモスの全国と九州の中だけを比べる（#51）。
//!   walmart を渡すと、米国の Walmart を比べる（日付は米国の xi の日付、#51）
//!
//! 読むもの：data/xi/xi_jp_<日付>_crossfp.csv（実際の値）、
//!           data/xi/null_mocks_cross_TRIALxSEIYU_<R>[_density][_grid][_keep…]_bins.csv（scripts/null_mocks_short.sh）
//! 書くもの：docs/tables/short_range.tex（cosmos なら docs/tables/cosmos_mocks.tex、walmart なら docs/tables/walmart_mocks.tex）

use std::path::PathBuf;

fn main() {
    let jp = std::env::args()
        .nth(1)
        .expect("日本の xi の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let which = std::env::args().nth(2).unwrap_or_default();
    let result = if which == "cosmos" {
        retail_cosmology::manager::short_range::run_cosmos(&root, &jp)
    } else if which == "walmart" {
        retail_cosmology::manager::short_range::run_walmart(&root, &jp)
    } else {
        retail_cosmology::manager::short_range::run(&root, &jp)
    };
    match result {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

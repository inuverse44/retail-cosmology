//! Client：χ² の上側確率を、帰無仮説の模擬データと比べて校正する（#9）。
//!
//! cargo run --release --bin chi2_calibrate -- 2026-11-01 2026-11-03
//!   1 つ目：オーストラリアの xi の日付、2 つ目：日本の xi の日付
//!
//! 読むもの：data/xi/chi2_<日付>_fp.txt、data/xi/chi2_jp_<日付>_crossfp.txt（観測の χ²）、
//!           data/xi/chi2_mocks_<国>_<R>.txt（scripts/chi2_mocks.sh）、
//!           data/xi/null_mocks_cross_TRIALxSEIYU_<R>.txt（scripts/null_mocks_cross.sh、R = 50、100）
//! 書くもの：docs/tables/chi2_calibrated.tex（ノートに載せる表）

use std::path::PathBuf;

fn main() {
    let au = std::env::args()
        .nth(1)
        .expect("オーストラリアの xi の日付（YYYY-MM-DD）");
    let jp = std::env::args()
        .nth(2)
        .expect("日本の xi の日付（YYYY-MM-DD）");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::calibration::run(&root, &au, &jp) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}

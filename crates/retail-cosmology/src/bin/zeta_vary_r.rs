//! Client：選択関数の幅 R を変えたコスモスの ξ̂ と ζ̂ を表にする（#94）。
//!
//! 先に R ごとに zeta を走らせる（どれも 20 秒ほど）：
//!   cargo run --release --bin zeta -- 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv
//!     --chain COSMOS --n-random 8000 --footprint R --xi on --tag fp<R>_t8000
//!   （R = 50 は --tag fp50_t8000_xi。ζ は既存の fp50_t8000 とバイト一致）
//! cargo run --release --bin zeta_vary_r
//! 書くもの：docs/tables/zeta_vary_r.tex

use std::path::PathBuf;

use retail_cosmology::manager::three_point::vary_r_table;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match vary_r_table(&root, &[15, 25, 35, 50, 70, 100, 150]) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

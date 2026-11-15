//! Client：模擬データ（NS と G10）を、W の幅 50 km のまま S の幅 R だけを変えて測った ξ̂_S と ζ̂_S を表にする（#94）。
//!
//! 先に --s-widths をつけて模擬データを作る（どちらも 20 回で約 1 時間）：
//!   zeta_clustered 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv --chain COSMOS
//!     --footprint 50 --children 14 --sigma 15 --runs 20 --n-random 8000 --n-ref 50000 --seed 1
//!     --tag swidth --s-widths 15,25,35,50,70,100,150
//!   zeta_field 2026-11-12 --country jp --stores （同じ） --chain COSMOS --footprint 50 --amp 0.7 --sigma 10
//!     --centers 20000 --runs 20 --n-random 8000 --n-ref 50000 --seed 1 --tag swidth --s-widths 15,25,35,50,70,100,150
//! cargo run --release --bin zeta_vary_r_mocks
//! 書くもの：docs/tables/zeta_vary_r_mocks.tex

use std::path::PathBuf;

use retail_cosmology::manager::three_point::vary_s_mocks_table;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let models = [
        (
            "NS",
            "data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_swidth",
        ),
        (
            "$G_{10}$",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_swidth",
        ),
    ];
    match vary_s_mocks_table(&root, &models) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

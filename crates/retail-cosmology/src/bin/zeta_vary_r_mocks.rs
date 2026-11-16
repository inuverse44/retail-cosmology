//! Client：模擬データ（NS と G）を、W の幅 R_W は決めたまま S の幅 R だけを変えて測った ξ̂_S と ζ̂_S を表にする（#94）。
//!
//! 先に --s-widths をつけて模擬データを作る（20 回で 45 分〜1 時間 40 分。1 回分を測ってから流す）：
//!   zeta_clustered 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv --chain COSMOS
//!     --footprint 50 --children 14 --sigma 15 --runs 20 --n-random 8000 --n-ref 50000 --seed 1
//!     --tag swidth --s-widths 15,25,35,50,70,100,150
//!   zeta_field 2026-11-12 --country jp --stores （同じ） --chain COSMOS --footprint 50 --amp 0.7 --sigma 10
//!     --centers 20000 --runs 20 --n-random 8000 --n-ref 50000 --seed 1 --tag swidth --s-widths 15,25,35,50,70,100,150
//! cargo run --release --bin zeta_vary_r_mocks
//! 書くもの（一つの表に入れるとページに収まらないので二つに分ける）：
//!   docs/tables/zeta_vary_r_mocks.tex と data/zeta/vary_r_mocks_drop.csv（NS と G10、R_W = 50・25 km）
//!   docs/tables/zeta_vary_r_mocks_g.tex と data/zeta/vary_r_mocks_drop_g.csv（G の R_W・振幅・場の幅を変えたもの）
//!   docs/tables/zeta_vary_r_mocks_skew.tex と data/zeta/vary_r_mocks_drop_skew.csv（ζ の符号を変えたもの、ADR 010）
//! （模型ごとに、R の両端の 16 ビンの平均の差を run ごとに求め、コスモスの同じ差と標準偏差で比べたもの）

use std::path::PathBuf;

use retail_cosmology::manager::three_point::vary_s_mocks_table;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // 模型の名前の括弧の中は、模擬データの W を作った幅 R_W [km]（--footprint）。
    // R_W = 25 km の二つは、上のコマンドの --footprint を 25、--tag を w25_swidth にしたもの。
    let base = [
        (
            "NS (50)",
            "data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_swidth",
        ),
        (
            "$G_{10}$ (50)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_swidth",
        ),
        (
            "NS (25)",
            "data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_w25_swidth",
        ),
        (
            "$G_{10}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w25_swidth",
        ),
    ];
    let variants = [
        // R_W = 15 km（--footprint 15 --seed 3 --tag w15_swidth）
        (
            "$G_{10}$ (15)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w15_swidth",
        ),
        // 振幅 0.4、R_W = 25 km（--amp 0.4 --footprint 25 --seed 4 --tag w25_swidth）
        (
            "$G_{10}^{0.4}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.4_s10_w25_swidth",
        ),
        // 場の幅 5 km・25 km、R_W = 25 km（--sigma 5 --seed 5、--sigma 25 --seed 6、--tag w25_swidth）
        (
            "$G_{5}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s5_w25_swidth",
        ),
        (
            "$G_{25}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s25_w25_swidth",
        ),
    ];
    // ζ の符号だけを変えた三つ（ADR 010）。比べ元の G10 (25) と、正と負の歪みの変換
    // （--transform chi --seed 7、--transform negchi --seed 8、--tag w25_swidth）。
    let skew = [
        (
            "$G_{10}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w25_swidth",
        ),
        (
            "$G_{10}^{+}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tchi_w25_swidth",
        ),
        (
            "$G_{10}^{-}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_w25_swidth",
        ),
        // 負の歪みで振幅を 0.5 にしたもの（--amp 0.5 --seed 9）と、場の幅を 15 km にしたもの（--sigma 15 --seed 10）
        (
            "$G_{10}^{-,0.5}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.5_s10_tnegchi_w25_swidth",
        ),
        (
            "$G_{15}^{-}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s15_tnegchi_w25_swidth",
        ),
    ];
    for (models, tex, drop) in [
        (
            &base[..],
            "docs/tables/zeta_vary_r_mocks.tex",
            "data/zeta/vary_r_mocks_drop.csv",
        ),
        (
            &variants[..],
            "docs/tables/zeta_vary_r_mocks_g.tex",
            "data/zeta/vary_r_mocks_drop_g.csv",
        ),
        (
            &skew[..],
            "docs/tables/zeta_vary_r_mocks_skew.tex",
            "data/zeta/vary_r_mocks_drop_skew.csv",
        ),
    ] {
        match vary_s_mocks_table(&root, models, tex, drop) {
            Ok(log) => print!("{log}"),
            Err(e) => panic!("{e}"),
        }
    }
}

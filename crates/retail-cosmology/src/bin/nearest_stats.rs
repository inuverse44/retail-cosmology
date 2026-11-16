//! Client：自分以外で最も近い店までの距離 d の分布を、コスモスと模擬データ（対照の A = 0 と密度場の模型）で比べる（#94）。
//! 店どうしが近づかない模型（候補 3）を設計する前に、どの距離で、どの密度の所で避け合っているかを見る。
//! 生の距離 d [km] と、店のいるマスの期待密度で割った u = π n_c d²（ポアソンなら平均 1 の指数分布）を出す。
//!
//! 模擬データの店の位置は cells_stats と同じもの（zeta_field --save-points on --skip-measure on、作り方は cells_stats.rs）。
//! cargo run --release --bin nearest_stats
//! 書くもの：data/zeta/nearest_stats.csv と docs/tables/nearest_stats.tex

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, nearest_table};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // cells_stats と同じ期待数（R_W = 25 km、参照の点 50000、種 1）
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[],
    };
    let models = [
        (
            "$A = 0$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0_s10_w25_swidth",
        ),
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
        (
            "$G_{10}^{-,0.5}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.5_s10_tnegchi_w25_swidth",
        ),
        (
            "$G_{15}^{-}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s15_tnegchi_w25_swidth",
        ),
        // ADR 012：G10− に r = 1 km、γ = 0.5 の近づかない条件（--inhibit 1:0.5 --seed 12）
        (
            "$G_{10}^{-}I_{1}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_inh1x0.5_w25_swidth",
        ),
    ];
    match nearest_table(&root, &p, &models) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

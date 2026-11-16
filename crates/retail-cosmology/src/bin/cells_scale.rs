//! Client：店を数えるマスの幅 k（3 次メッシュ k×k 個、k = 10 が 2 次メッシュ）と、期待数を作る選択関数の幅 R_W を
//! 変えて、空のマスの比 V と κ̂₂ をコスモスと模擬データで比べる（ADR 013、#115）。
//!
//! 模擬データの店の位置は cells_stats と同じもの（zeta_field --save-points on の出力）を使い、作り直さない。
//! cargo run --release --bin cells_scale
//! 書くもの：docs/tables/cells_scale.tex と data/zeta/cells_scale.csv

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, scale_table};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // cells_stats と同じ参照の点と λ の下限。r_w_km と block は settings の値に置き換わる
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[1.0, 0.5, 2.0],
        block: 10,
    };
    // (k, R_W [km])：R_W = 25 km でマスの幅を変える組と、k = 10 で R_W を変える組
    let settings = [
        (3, 25.0),
        (5, 25.0),
        (7, 25.0),
        (10, 25.0),
        (14, 25.0),
        (20, 25.0),
        (10, 15.0),
        (10, 50.0),
        (10, 100.0),
    ];
    // 場の揺らぎのない対照、ガウス場（ζ ≈ 0）、負の歪みの場、それに近づかない条件を足したもの
    let models = [
        (
            "$A = 0$",
            "data/zeta/field_jp_2026-11-12_cosmos_a0_s10_w25_swidth",
        ),
        (
            "$G_{10}$",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w25_swidth",
        ),
        (
            "$G_{10}^{-}$",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_w25_swidth",
        ),
        (
            "$G_{10}^{-}I_{1}$",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_inh1x0.5_w25_swidth",
        ),
    ];
    match scale_table(&root, &p, &models, &settings) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

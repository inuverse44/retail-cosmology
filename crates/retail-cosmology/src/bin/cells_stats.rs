//! Client：店を約 10 km のマス（2 次メッシュ）に数え、店と同じ手順の期待数からのずれ（κ̂₂、κ̂₃、
//! 空のマスの比 V、多いマスの比 H）を、コスモスと密度場の模擬データで比べる（ADR 011、#94）。
//!
//! 先に模擬データの店の位置を作る（元と同じ引数に --save-points on --skip-measure on を足す。1 模型 7 分ほど）：
//!   zeta_field 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv --chain COSMOS
//!     --footprint 25 --amp 0.7 --sigma 10 --centers 20000 --runs 20 --n-random 8000 --n-ref 50000
//!     --seed 1 --tag w25_swidth --save-points on --skip-measure on
//!   （対照の A = 0 は --amp 0 --seed 11、G10+ は --transform chi --seed 7、G10− は --transform negchi --seed 8、
//!     G10−(0.5) は --amp 0.5 --transform negchi --seed 9、G15− は --sigma 15 --transform negchi --seed 10）
//! cargo run --release --bin cells_stats
//! 書くもの：docs/tables/cells_stats.tex と data/zeta/cells_stats.csv

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, cells_table};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        // 表には最初の値を使う。残りは λ_min でどれだけ変わるかを見るため
        lambda_mins: &[1.0, 0.5, 2.0],
    };
    let models = [
        // 対照：場の揺らぎなし（A = 0、--amp 0 --seed 11）。W だけのポアソン点で、手順そのものの偏りを見る
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
    ];
    match cells_table(&root, &p, &models) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

//! Client：ADR 012 の値（近づかない条件の r・γ、人口の冪 α）を選ぶための短い比べ方。
//! 店の位置のファイル（zeta_field --save-points on --skip-measure on）を渡すと、コスモスと並べて
//! V・κ̂₂（λ_c ≥ 1）と、最も近い店までの距離の主な量をログに出す。ファイルは書かない。
//!
//! cargo run --release --bin local_scan -- 名前=stem [名前=stem ...]
//!   stem は `_points.csv` の前まで（例：data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_inh1x0_scan）

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, local_scan};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(!args.is_empty(), "名前=stem を一つ以上渡す");
    let models: Vec<(&str, &str)> = args
        .iter()
        .map(|a| a.split_once('=').expect("名前=stem の形で渡す"))
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // cells_stats・nearest_stats と同じ期待数
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[1.0],
    };
    let keys = [
        "G_d<0.5",
        "G_d<1",
        "G_d<2",
        "d_q90",
        "lam<1_n",
        "lam1-3_n",
        "lam>=3_n",
    ];
    match local_scan(&root, &p, &models, &keys) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

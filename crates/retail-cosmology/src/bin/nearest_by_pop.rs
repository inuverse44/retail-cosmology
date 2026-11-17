//! Client：店のいるマス（2 次メッシュ）の人口の区間ごとに、店の数と最も近い店までの距離の中央値・90% 点を、
//! コスモスと模擬データで比べる（#115）。d_90 の差がどの人口の所から来るかを見る。ログだけで、ファイルは書かない。
//!
//! cargo run --release --bin nearest_by_pop -- 名前=stem [名前=stem ...]（stem は `_points.csv` の前まで）

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, nearest_by_pop};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(!args.is_empty(), "名前=stem を一つ以上渡す");
    let models: Vec<(&str, &str)> = args
        .iter()
        .map(|a| a.split_once('=').expect("名前=stem の形で渡す"))
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // pop_response と同じマスと区間
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[],
        block: 10,
    };
    let edges = [3_000.0, 10_000.0, 30_000.0, 100_000.0, 300_000.0];
    match nearest_by_pop(&root, &p, &models, &edges) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

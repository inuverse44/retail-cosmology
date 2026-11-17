//! Client：店のいるマス（2 次メッシュ）の人口の区間ごとに、店のまわり 5 km の人口 P と、
//! P が c 倍以上の場所までの距離 δ_c（より大きな町までの距離）を、コスモスと模擬データで比べる（#115、ADR 017）。
//! ログだけで、ファイルは書かない。
//!
//! cargo run --release --bin town_distance -- 名前=stem [名前=stem ...]（stem は `_points.csv` の前まで）

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, town_distance_by_pop};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(!args.is_empty(), "名前=stem を一つ以上渡す");
    let models: Vec<(&str, &str)> = args
        .iter()
        .map(|a| a.split_once('=').expect("名前=stem の形で渡す"))
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // pop_response・nearest_by_pop と同じマスと区間
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
    // P の半径は ADR 015 の R_p と同じ 5 km。c は 2 と 4、距離の上限 50 km、遠いとみなす距離 10 km
    match town_distance_by_pop(&root, &p, &models, &edges, 5.0, &[2.0, 4.0], 50.0, 10.0) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

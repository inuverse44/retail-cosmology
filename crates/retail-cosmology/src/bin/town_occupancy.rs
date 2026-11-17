//! Client：人口の格子をまわり 5 km の人口の山で町に分け、町の期待数 E_t の区間ごとに、店のある町の割合、
//! 町あたりの店の数、二軒以上ある町の割合を、コスモスと模擬データで比べる（#115、ADR 018）。ログだけ。
//!
//! cargo run --release --bin town_occupancy -- 中心どうしの最小距離[km] 名前=stem [名前=stem ...]
//!   stem は `_points.csv` の前まで

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, town_occupancy};

fn main() {
    let mut args = std::env::args().skip(1);
    let d_peak: f64 = args
        .next()
        .expect("町の中心どうしの最小距離 [km] を渡す")
        .parse()
        .expect("数で渡す");
    let rest: Vec<String> = args.collect();
    assert!(!rest.is_empty(), "名前=stem を一つ以上渡す");
    let models: Vec<(&str, &str)> = rest
        .iter()
        .map(|a| a.split_once('=').expect("名前=stem の形で渡す"))
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // コスモスの店のファイル（期待数の S は使わない）
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[],
        block: 10,
    };
    // P の半径は ADR 015 の R_p と同じ 5 km
    let edges = [0.1, 0.3, 1.0, 3.0];
    match town_occupancy(&root, &p, &models, 5.0, d_peak, &edges) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

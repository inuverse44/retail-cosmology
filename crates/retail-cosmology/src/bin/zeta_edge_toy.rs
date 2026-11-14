//! Client：一様な基準でも ζ̂ が見込みより低く出る原因を、toy model の範囲の縁で確かめる（#81）。
//!
//! cargo run --release --bin zeta_edge_toy [-- 回数（既定 40）]
//! 書くもの：data/toy/zeta_edge.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::edge_toy;

fn main() {
    let runs: usize = std::env::args().nth(1).map_or(40, |v| v.parse().unwrap());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match edge_toy(&root, runs) {
        Ok(p) => eprintln!("{p} を書いた"),
        Err(e) => panic!("{e}"),
    }
}

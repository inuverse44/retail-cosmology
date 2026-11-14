//! Client：三点相関関数の推定量を toy model で確かめ、一回ごとの ζ を書く（ADR 007）。
//!
//! cargo run --release --bin zeta_toy [-- 回数（既定 40）]
//! 書くもの：data/toy/zeta_toy.csv（model,run,zeta,expected）

use std::path::PathBuf;

use retail_cosmology::manager::three_point::toy;

fn main() {
    let runs: usize = std::env::args().nth(1).map_or(40, |v| v.parse().unwrap());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match toy(&root, runs) {
        Ok(p) => eprintln!("{p} を書いた"),
        Err(e) => panic!("{e}"),
    }
}

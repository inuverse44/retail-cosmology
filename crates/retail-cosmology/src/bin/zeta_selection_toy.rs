//! Client：店から作った選択関数が、二点で固まった店の三点相関関数をどれだけずらすかを toy model で測る（#81）。
//!
//! cargo run --release --bin zeta_selection_toy [-- 回数（既定 40）]
//! 書くもの：data/toy/zeta_selection_bias.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::selection_bias_toy;

fn main() {
    let runs: usize = std::env::args().nth(1).map_or(40, |v| v.parse().unwrap());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match selection_bias_toy(&root, runs) {
        Ok(p) => eprintln!("{p} を書いた"),
        Err(e) => panic!("{e}"),
    }
}

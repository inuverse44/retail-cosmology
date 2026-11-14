//! Client：実際の店の三点相関関数を、帰無仮説の模擬データ（scripts/zeta_mocks.sh が書く）とビンごとに比べて表示する。
//!
//! cargo run --release --bin zeta_compare -- data/zeta/実際.csv data/zeta/模擬1.csv data/zeta/模擬2.csv ...
//! パスはリポジトリの根からの相対パス。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::compare_mocks;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(args.len() >= 3, "実際の CSV と、模擬データの CSV を 2 つ以上渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match compare_mocks(&root, &args[0], &args[1..]) {
        Ok(t) => print!("{t}"),
        Err(e) => panic!("{e}"),
    }
}

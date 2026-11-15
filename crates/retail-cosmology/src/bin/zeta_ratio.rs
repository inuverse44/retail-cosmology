//! Client：ζ̂ = 分子 ÷ 分母 の、比の平均と平均の比を比べる（#94）。
//!
//! cargo run --release --bin zeta_ratio -- <ラベル> <zeta_field --counts on が書いた CSV>
//! 書くもの：data/zeta/ratio_<ラベル>.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::ratio_decomposition;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "ラベルと、--counts on で作った CSV を渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match ratio_decomposition(&root, &args[0], &args[1]) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

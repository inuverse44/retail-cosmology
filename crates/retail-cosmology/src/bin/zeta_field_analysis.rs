//! Client: 独立な run 範囲をまとめ、proc の共分散・全体検定・感度を報告する。
//! zeta_field_analysis REAL.csv OUTPUT_STEM MOCK_STEM [MOCK_STEM ...]
use retail_cosmology::manager::three_point::analyze_field;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        args.len() >= 3,
        "REAL.csv OUTPUT_STEM MOCK_STEM [MOCK_STEM ...]"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match analyze_field(&root, &args[0], &args[1], &args[2..]) {
        Ok(path) => eprintln!("{path} ほかを書いた"),
        Err(e) => panic!("{e}"),
    }
}

//! Client：二度ならしたずれ w から式 eq:zeta-edge で見込む ζ̂ と、ガウス場の実測を比べる（#94）。
//!
//! cargo run --release --bin zeta_mismatch_pred -- data/zeta/selection_mismatch_seed1.csv <ラベル>...
//! ラベルは zeta_absorption が書いた data/zeta/absorption_<ラベル>.csv（と _xi.csv）。
//! 書くもの：data/zeta/mismatch_pred_<ラベル>.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::mismatch_prediction;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(args.len() >= 2, "ずれの CSV と、ラベル（1 つ以上）を渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match mismatch_prediction(&root, &args[0], &args[1..]) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

//! Client：選択関数がどれだけ吸い込むかを、模擬データの「正しい基準」と「店と同じ手順」の対から測る（#94）。
//!
//! cargo run --release --bin zeta_absorption -- <ラベル> <基準[:比べる列]> <模擬データの ζ の CSV>...
//! 例：cargo run --release --bin zeta_absorption -- field90 true data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10.csv
//! 基準は true か match。比べる列の既定は proc（--split on の出力なら half_true:half_proc、half_true:half_split も）。
//! 書くもの：data/zeta/absorption_<ラベル>.csv と absorption_<ラベル>_xi.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::absorption;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        args.len() >= 3,
        "ラベル、基準（true か match、:比べる列 も可）、模擬データの CSV（1 つ以上）を渡す"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match absorption(&root, &args[0], &args[1], &args[2..]) {
        Ok(msg) => print!("{msg}"),
        Err(e) => panic!("{e}"),
    }
}

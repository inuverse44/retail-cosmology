//! Client：zeta_clustered が書いた模擬データをビンごとにまとめ、実際の店の ζ̂ と並べる（#81）。
//!
//! cargo run --release --bin zeta_clustered_summary -- data/zeta/clustered_jp_2026-11-12_cosmos_m6_s12 \
//!     data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv
//! 1 つ目は .csv を除いた名前、2 つ目は実際の店の ζ（同じランダムカタログと設定で zeta が書いたもの）。
//! 書くもの：<名前>_summary.csv と <名前>_xi_summary.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::summarize_clustered;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "模擬データの名前（.csv を除く）と、実際の店の ζ の CSV を渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match summarize_clustered(&root, &args[0], &args[1]) {
        Ok(p) => eprintln!("{p} ほかを書いた"),
        Err(e) => panic!("{e}"),
    }
}

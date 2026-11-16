//! Client：ADR 010 の密度場の変換を説明する図の数を書く（ノートの図 fig:skewed-field）。
//!
//! cargo run --release --bin field_transform_demo
//! uv run scripts/plot_skewed_field.py
//! 書くもの：data/zeta/field_transform_line.csv、_pdf.csv、_expect.csv
//! 振幅と幅は、模擬データ（zeta_field --transform chi・negchi）と同じ A = 0.7、σ = 10 km。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::field_transform_demo;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // 種は引数で変えられる。線が 300 km と短く（相関の長さの約 8.5 倍）、一つの実現の g の平均と分散は
    // 0・1 から揺らぐ（種 1〜60 で平均は −0.763〜+0.724、分散は 0.231〜1.744）。既定の種 33 は、この中から
    // 平均 0.037・分散 0.966 と典型に近いものを図の見本に選んだ。
    let seed = std::env::args()
        .nth(1)
        .map_or(33, |s| s.parse().expect("種は整数"));
    match field_transform_demo(&root, 0.7, 10.0, seed) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

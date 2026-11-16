//! Client：同じ場で、店どうしが近づかない条件だけが違う二つの模擬データを、run ごとに対にして Q̄_S と ξ̂_S を比べる（ADR 012、#94）。
//!
//! 先に二つを run ごとの乱数（--start 0）で作る。条件は店を選ぶときに乱数を余分に使うので、run をまたいで一続きの乱数
//! （--start なし）では 2 回目以降の場がずれる。run ごとの乱数なら、各 run の場は二つで同じになる：
//!   zeta_field 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv --chain COSMOS
//!     --footprint 25 --amp 0.7 --sigma 10 --centers 20000 --runs 20 --start 0 --n-random 8000 --n-ref 50000
//!     --seed 8 --tag w25_pair --transform negchi --s-widths 15,25,35,50,70,100,150 --save-points on
//!   （b は同じ引数に --inhibit 1:0.5 を足す）
//! cargo run --release --bin zeta_pair_q
//! ファイルは書かない。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::paired_q;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let a = (
        "G10-",
        "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_w25_pair",
    );
    let b = (
        "G10-I1",
        "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_inh1x0.5_w25_pair",
    );
    match paired_q(&root, a, b) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

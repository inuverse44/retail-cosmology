//! Client：親の数と子の数を決めた丸い塊の模擬データで、ζ̂ が ζ_NS から決まってずれる分の見込みと実測を比べる（#94）。
//!
//! cargo run --release --bin zeta_clustered_ic -- <前置き> --children 14 --stores パス --chain COSMOS
//! 前置きは zeta_clustered --match on の出力（例：data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20）。
//! 店の数 N は店のファイルから数える。書くもの：<前置き>_ic.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::clustered_ic;

fn main() {
    let mut it = std::env::args().skip(1);
    let stem = it.next().expect("zeta_clustered の出力の前置きを渡す");
    let (mut m, mut path, mut chain) = (0usize, String::new(), String::new());
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--children" => m = v.parse().unwrap(),
            "--stores" => path = v,
            "--chain" => chain = v,
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match clustered_ic(&root, &stem, m, &path, &chain) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

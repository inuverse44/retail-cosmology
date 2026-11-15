//! Client：丸い塊の模擬データの平均の密度と、match の基準（親ごとに最初の子だけ）の密度の比 w を求め、
//! 式 eq:zeta-edge の ζ_w で、ζ̂_match − ζ_NS の残りを見込む（#94）。
//!
//! cargo run --release --bin zeta_clustered_wchild -- 2026-11-12 --country jp --stores パス --chain COSMOS
//!     --footprint 50 --children 14 --sigma 15 --n-random 8000 --n-ref 50000 --seed 1
//!     --stem data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20 [--reps 10] [--tri-thin 8]
//! --stem の _ic.csv（zeta_clustered_ic の出力）と _xi.csv を読む。書くもの：<stem>_wchild.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{Params, clustered_child_density};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    let (mut m, mut sigma, mut reps, mut stem) = (5usize, 8.0f64, 10usize, String::new());
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => p.country = v,
            "--stores" => p.stores = v,
            "--chain" => p.chain = v,
            "--n-random" => p.n_random = v.parse().unwrap(),
            "--n-ref" => p.n_ref = v.parse().unwrap(),
            "--seed" => p.seed = v.parse().unwrap(),
            "--footprint" => p.footprint = Some(v.parse().unwrap()),
            "--children" => m = v.parse().unwrap(),
            "--sigma" => sigma = v.parse().unwrap(),
            "--reps" => reps = v.parse().unwrap(),
            "--tri-thin" => p.tri_thin = Some(v.parse().unwrap()),
            "--stem" => stem = v,
            _ => panic!("知らない引数：{k}"),
        }
    }
    assert!(!stem.is_empty(), "--stem が要る");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match clustered_child_density(&root, &p, m, sigma, reps, &stem) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

//! Client：縁のない球面で、親の数と子の数を決めた Neyman–Scott の toy を作り、
//! 数を決めたことによる ζ̂ のずれの見込み（zeta_clustered_ic と同じ式）を確かめる（#94）。
//!
//! cargo run --release --bin zeta_ic_toy -- <名前> [--radius 175] [--n 1735] [--children 14] [--sigma 15]
//!     [--n-random 8000] [--runs 200] [--seed 1]
//! 書くもの：data/toy/ic_sphere_<名前>.csv、_xi.csv、_ic.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::ic_sphere_toy;

fn main() {
    let mut it = std::env::args().skip(1);
    let name = it.next().expect("名前を渡す");
    let (mut radius, mut n, mut m, mut sigma, mut n_random, mut runs, mut seed) = (
        175.0f64, 1735usize, 14usize, 15.0f64, 8000usize, 200usize, 1u64,
    );
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--radius" => radius = v.parse().unwrap(),
            "--n" => n = v.parse().unwrap(),
            "--children" => m = v.parse().unwrap(),
            "--sigma" => sigma = v.parse().unwrap(),
            "--n-random" => n_random = v.parse().unwrap(),
            "--runs" => runs = v.parse().unwrap(),
            "--seed" => seed = v.parse().unwrap(),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match ic_sphere_toy(&root, &name, radius, n, m, sigma, n_random, runs, seed) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

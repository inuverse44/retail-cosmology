//! Client：jackknife の領域の分け方を CSV に書く（ノートの図、#44）。
//!
//! cargo run --release --bin jk_regions -- --country au|jp [--seed 1] [--n-random 20000] [--depth 6]
//!   既定は xi の既定と同じで、ノートの数字に使った領域と同じ領域が出る。
//!   読むもの：data/abs/apgr25_1_0_0.tif（au）、data/estat（jp）
//!   書くもの：data/random/jk_regions_<国>_seed<種>.csv（lon,lat,region）

use std::path::PathBuf;

use retail_cosmology::manager::jk_regions::run;

fn main() {
    let (mut country, mut seed, mut n_random, mut depth) =
        (String::from("au"), 1_u64, 20_000_usize, 6_u32);
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => country = v,
            "--seed" => seed = v.parse().unwrap(),
            "--n-random" => n_random = v.parse().unwrap(),
            "--depth" => depth = v.parse().unwrap(),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let csv = run(&root, &country, seed, n_random, depth).unwrap_or_else(|e| panic!("{e}"));
    let dir = root.join("data/random");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("jk_regions_{country}_seed{seed}.csv"));
    std::fs::write(&path, csv).unwrap();
    eprintln!("{} に書いた", path.display());
}

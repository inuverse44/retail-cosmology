//! Client：オーストラリアのランダムカタログ（陸地に一様、人口に比例）と人口そのものの点を CSV に書く（ノートの図、#42）。
//!
//! cargo run --release --bin random_points -- [--seed 1] [--n-random 20000] [--n-pop 2000]
//!   既定は xi の既定と同じで、ノートの数字に使った点と同じ点が出る。
//!   読むもの：data/abs/apgr25_1_0_0.tif
//!   書くもの：data/random/random_au_seed<種>.csv（kind,lon,lat）

use std::path::PathBuf;

use retail_cosmology::manager::random_points::run;

fn main() {
    let (mut seed, mut n_random, mut n_pop) = (1_u64, 20_000_usize, 2_000_usize);
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--seed" => seed = v.parse().unwrap(),
            "--n-random" => n_random = v.parse().unwrap(),
            "--n-pop" => n_pop = v.parse().unwrap(),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let csv = run(&root, seed, n_random, n_pop).unwrap_or_else(|e| panic!("{e}"));
    let dir = root.join("data/random");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("random_au_seed{seed}.csv"));
    std::fs::write(&path, csv).unwrap();
    eprintln!("{} に書いた", path.display());
}

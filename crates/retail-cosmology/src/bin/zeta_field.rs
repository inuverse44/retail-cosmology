//! Client：ζ = 0 の模擬データ（ガウス場で店の密度を揺らす模型）を日本の実際の地形の上で作り、
//! 店と同じ手順で三点相関関数を測る（#81）。
//!
//! cargo run --release --bin zeta_field -- 2026-11-12 --country jp --stores パス --chain COSMOS
//!     --footprint 50 --amp 0.5 --sigma 15 --centers 20000 --runs 3 [--n-random 8000] [--seed 1] [--sides 2:32:4]
//!
//! --amp A：δ の標準偏差。--sigma σ：δ を作るガウス関数の幅 [km]。--centers：ガウス関数の中心の数。
//! 書くもの：data/zeta/field_<jp_|us_><日付>_<チェーン>_a<A>_s<σ>[_名前].csv と _xi.csv
//! まとめは zeta_clustered_summary で作る（形は同じ）。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{Params, field_mocks};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    let (mut amp, mut sigma, mut centers, mut runs) = (0.5f64, 15.0f64, 20_000usize, 3usize);
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
            "--amp" => amp = v.parse().unwrap(),
            "--sigma" => sigma = v.parse().unwrap(),
            "--centers" => centers = v.parse().unwrap(),
            "--runs" => runs = v.parse().unwrap(),
            "--sides" => {
                let s: Vec<&str> = v.split(':').collect();
                assert_eq!(s.len(), 3, "--sides lo:hi:n");
                p.sides_km = (s[0].parse().unwrap(), s[1].parse().unwrap(), s[2].parse().unwrap());
            }
            "--tag" => p.tag = format!("_{v}"),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match field_mocks(&root, &p, amp, sigma, centers, runs) {
        Ok(path) => eprintln!("{path} を書いた"),
        Err(e) => panic!("{e}"),
    }
}

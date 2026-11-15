//! Client：場がなくても残る、店から作った選択関数のずれ（二度ならしたずれ）w を測る（#94）。
//!
//! cargo run --release --bin zeta_mismatch -- 2026-11-12 --country jp --stores パス --chain COSMOS
//!     --footprint 50 [--n-random 8000] [--seed 1]
//! zeta_field と同じ種なら、同じ参照の点と点の池を使う。
//! 書くもの：data/zeta/selection_mismatch_seed<種>.csv（r ごとの ⟨w²(1−w)⟩₃、⟨(w−1)³⟩₃、⟨w⟩₃）

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{Params, selection_mismatch};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
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
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match selection_mismatch(&root, &p) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

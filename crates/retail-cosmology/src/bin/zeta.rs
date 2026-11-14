//! Client：一つのチェーンの店舗の三点相関関数を測る（ADR 007）。
//!
//! cargo run --release --bin zeta -- 2026-11-03 --country jp --stores パス --chain COSMOS
//!                                    [--n-random 2000] [--n-ref 50000] [--seed 1] [--footprint R]
//!                                    [--selection-grid on] [--mock 種] [--sides 2:32:4] [--tag 名前]
//!
//! --footprint R：出店している地方を、幅 R [km] の選択関数としてランダムカタログに入れる。
//! --mock 種：店の代わりに、基準と同じ重みで独立に撒いた点を使う（ζ = 0 の帰無仮説）。
//! --mock-density on：--mock のとき、重みに人口密度 ρ の γ 乗も掛け、人の密集地への寄り方を実際の店にそろえる。
//! --sides lo:hi:n：三角形の辺のビン（[lo, hi) km を対数で n 等分）。
//! 時間はランダムの点の数の 3 乗くらいで増える。先に --n-random を小さくして時間を測る。
//!
//! jp：人口は e-Stat の 1 km メッシュ。us：人口は GHS-POP を米国本土 48 州と D.C. で切り出したもの。
//! 書くもの：data/zeta/zeta_<jp_|us_><日付>_<チェーン>[_名前].csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{Params, run};

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
            "--selection-grid" => p.selection_grid = v == "on",
            "--mock" => p.mock = Some(v.parse().unwrap()),
            "--mock-density" => p.mock_density = v == "on",
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
    if let Err(e) = run(&root, &p) {
        panic!("{e}");
    }
}

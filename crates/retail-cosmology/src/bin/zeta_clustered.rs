//! Client：二点の固まりを実際の店にそろえ、ζ がわかっている模擬データ（Neyman–Scott の模型）で、
//! 店から作った選択関数の偏りを実際の地形の上で測る（#81）。
//!
//! cargo run --release --bin zeta_clustered -- 2026-11-12 --country jp --stores パス --chain COSMOS
//!     --footprint 50 --children 5 --sigma 8 --runs 3 [--n-random 8000] [--n-ref 50000] [--seed 1] [--sides 2:32:4]
//!
//! --children m：親ひとつあたりの子の数。--sigma σ：子を撒く標準偏差 [km]。--runs：模擬データの数。
//! 書くもの：data/zeta/clustered_<jp_|us_><日付>_<チェーン>_m<m>_s<σ>[_名前].csv と _xi.csv

//! --regional on|prefecture：地方別または都道府県別の店舗数 / 人口を使う（clustered は従来の proc に追加して比較）。
//! --match on：親ごとに子を一つ選ぶ独立の基準を追加。新規の --tag が必要。
//! --counts on：基準ごとに ζ̂ の分子と分母も書く（num_true,rrr_true,num_proc,rrr_proc[,num_match,rrr_match]）。
//!   zeta_ratio で比の偏りを切り分ける（#94）。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{Params, RegionalScale, clustered_mocks};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    let (mut m, mut sigma, mut runs) = (5usize, 8.0f64, 3usize);
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => p.country = v,
            "--stores" => p.stores = v,
            "--chain" => p.chain = v,
            "--n-random" => p.n_random = v.parse().unwrap(),
            "--n-ref" => p.n_ref = v.parse().unwrap(),
            "--seed" => p.seed = v.parse().unwrap(),
            "--regional" => {
                p.regional = match v.as_str() {
                    "on" => Some(RegionalScale::Region),
                    "prefecture" => Some(RegionalScale::Prefecture),
                    "off" => None,
                    _ => panic!("--regional on|prefecture|off"),
                }
            }
            "--match" => p.match_density = v == "on",
            "--counts" => p.counts = v == "on",
            "--footprint" => p.footprint = Some(v.parse().unwrap()),
            "--children" => m = v.parse().unwrap(),
            "--sigma" => sigma = v.parse().unwrap(),
            "--runs" => runs = v.parse().unwrap(),
            "--sides" => {
                let s: Vec<&str> = v.split(':').collect();
                assert_eq!(s.len(), 3, "--sides lo:hi:n");
                p.sides_km = (
                    s[0].parse().unwrap(),
                    s[1].parse().unwrap(),
                    s[2].parse().unwrap(),
                );
            }
            "--tag" => p.tag = format!("_{v}"),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match clustered_mocks(&root, &p, m, sigma, runs) {
        Ok(path) => eprintln!("{path} を書いた"),
        Err(e) => panic!("{e}"),
    }
}

//! Client：店舗の二点相関関数を、いくつかのランダムカタログで測り、jackknife で誤差をつける。
//!
//! cargo run --release --bin xi -- 2026-11-01 [--country au|jp] [--stores パス] [--n-random 20000]
//!                                            [--n-pop 2000] [--depth 6] [--seed 1] [--tag 名前]
//!
//! au（既定）：Coles と Woolworths。人口は ABS の格子人口。基準は陸地・人が住む格子・人口。
//!   読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//!   書くもの：data/xi/xi_<日付>[_名前].csv、data/xi/chi2_<日付>[_名前].txt
//! jp：トライアル。人口は e-Stat の 1 km メッシュ。基準は人が住む格子・人口
//!   （メッシュのファイルには人口 0 の陸地が載っていないので、陸地の基準は作れない）。
//!   店は公式サイトから取ったもの（日付はサイトから取った日）。--stores で OSM の店などに替えられる。
//!   読むもの：data/trial_site/trial_site_clean_<日付>.csv、data/estat/tblT001140S*.txt
//!   書くもの：data/xi/xi_jp_<日付>[_名前].csv、data/xi/chi2_jp_<日付>[_名前].txt

use std::path::PathBuf;

use retail_cosmology::manager::correlation::{Params, run};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => p.country = v,
            "--stores" => p.stores = v,
            "--n-random" => p.n_random = v.parse().unwrap(),
            "--n-pop" => p.n_pop = v.parse().unwrap(),
            "--depth" => p.depth = v.parse().unwrap(),
            "--seed" => p.seed = v.parse().unwrap(),
            "--tag" => p.tag = format!("_{v}"),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Err(e) = run(&root, &p) {
        panic!("{e}");
    }
}

//! Client：模擬データ 1 個の計算量を見積もるための数を測る（notes/algorithm-cost.tex）。
//!
//! cargo run --release --bin cost -- [R [km]] [試しの点の数] [種]
//!   既定は R = 50、試しの点 2000、種 1。店はトライアル（公式サイト 2026-11-03）と西友（公式サイト 2026-11-04）。
//!   書くもの：data/cost/cost_R<R>.csv

use std::path::PathBuf;

fn main() {
    let mut it = std::env::args().skip(1);
    let r_km: f64 = it.next().map_or(50.0, |v| v.parse().unwrap());
    let n_probe: usize = it.next().map_or(2000, |v| v.parse().unwrap());
    let seed: u64 = it.next().map_or(1, |v| v.parse().unwrap());
    let files = vec![
        "data/trial_site/trial_site_clean_2026-11-03.csv".to_string(),
        "data/seiyu_site/seiyu_site_clean_2026-11-04.csv".to_string(),
    ];
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match retail_cosmology::manager::cost::run(&root, &files, r_km, n_probe, seed) {
        Ok(r) => print!("{r}"),
        Err(e) => panic!("{e}"),
    }
}

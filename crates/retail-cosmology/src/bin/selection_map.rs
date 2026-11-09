//! Client：日本のチェーンの選択関数 S_R を、参照の点ごとに CSV に書く（ノートの図、#43）。
//!
//! cargo run --release --bin selection_map -- 2026-11-03 [--chain TRIAL] [--seed 1] [--n-ref 50000]
//!                                                       [--footprint 50,100,200]
//!   既定は xi の既定と同じで、ノートの数字に使った参照の点と同じ点を使う。
//!   読むもの：data/trial_site/trial_site_clean_<日付>.csv、data/estat/tblT001140S*.txt
//!   書くもの：data/selection/selection_<チェーン>_<日付>.csv（lon,lat,S_<R>…）

use std::path::PathBuf;

use retail_cosmology::manager::selection_map::run;

fn main() {
    let mut it = std::env::args().skip(1);
    let date = it.next().expect("日付（YYYY-MM-DD）を渡す");
    let (mut chain, mut seed, mut n_ref) = ("TRIAL".to_string(), 1_u64, 50_000_usize);
    let mut widths = vec![50.0, 100.0, 200.0];
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--chain" => chain = v,
            "--seed" => seed = v.parse().unwrap(),
            "--n-ref" => n_ref = v.parse().unwrap(),
            "--footprint" => widths = v.split(',').map(|x| x.parse().unwrap()).collect(),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = format!("data/trial_site/trial_site_clean_{date}.csv");
    let csv = run(&root, &file, &chain, seed, n_ref, &widths).unwrap_or_else(|e| panic!("{e}"));
    let dir = root.join("data/selection");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("selection_{chain}_{date}.csv"));
    std::fs::write(&path, csv).unwrap();
    eprintln!("{} に書いた", path.display());
}

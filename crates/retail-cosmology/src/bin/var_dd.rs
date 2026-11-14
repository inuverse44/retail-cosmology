//! Client：オーストラリアの DD の分散を、ランダムカタログから求めた幾何の因子 G_p、G_t で見積もる（#3、ノートの付録「DD の分散」）。
//!
//! cargo run --release --bin var_dd -- 2026-11-01 [基準:点の数:種 ...]
//!   基準は land（陸地に一様）か population（人口に比例）。既定は収束を見る組：
//!   population:10000:1 population:20000:1 population:40000:1 population:20000:2 population:20000:3 land:20000:1
//!   読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//!   書くもの：data/var/var_dd_au_<日付>.csv、docs/tables/var_dd_au.tex（ノートの表の中身）

use std::path::PathBuf;

use retail_cosmology::manager::variance::{Run, Weight, run};

fn main() {
    let mut it = std::env::args().skip(1);
    let date = it.next().expect("日付（YYYY-MM-DD）を渡す");
    let mut specs: Vec<String> = it.collect();
    if specs.is_empty() {
        specs = [
            "population:10000:1",
            "population:20000:1",
            "population:40000:1",
            "population:20000:2",
            "population:20000:3",
            "land:20000:1",
        ]
        .map(String::from)
        .to_vec();
    }
    let runs: Vec<Run> = specs
        .iter()
        .map(|s| {
            let f: Vec<&str> = s.split(':').collect();
            assert_eq!(f.len(), 3, "基準:点の数:種 の形で渡す：{s}");
            let weight = match f[0] {
                "land" => Weight::Land,
                "population" => Weight::Population,
                w => panic!("知らない基準：{w}"),
            };
            Run {
                weight,
                n_random: f[1].parse().unwrap(),
                seed: f[2].parse().unwrap(),
            }
        })
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = run(&root, &date, &runs).unwrap_or_else(|e| panic!("{e}"));
    let dir = root.join("data/var");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("var_dd_au_{date}.csv"));
    std::fs::write(&path, out.csv).unwrap();
    eprintln!("{} に書いた", path.display());
    if !out.table.is_empty() {
        let tab = root.join("docs/tables/var_dd_au.tex");
        std::fs::write(&tab, out.table).unwrap();
        eprintln!("{} に書いた", tab.display());
    }
}

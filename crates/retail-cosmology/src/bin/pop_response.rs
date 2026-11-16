//! Client：2 次メッシュの人口の区間ごとに、店の数と期待数の和の比 ΣN/Σλ と空のマスの比を、
//! コスモスと模擬データで比べる（#115）。店が人口に比例して置かれているかを見る。ファイルは書かない。
//!
//! cargo run --release --bin pop_response [-- 名前=stem ...]（stem は `_points.csv` の前まで）

use std::path::PathBuf;

use retail_cosmology::manager::counts_in_cells::{CellParams, pop_response};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // cells_stats と同じ期待数
    let p = CellParams {
        stores: "data/cosmos_site/cosmos_site_clean_2026-11-12.csv",
        chain: "COSMOS",
        r_w_km: 25.0,
        n_ref: 50_000,
        seed: 1,
        lambda_mins: &[],
        block: 10,
    };
    // 引数（名前=stem）があればそれを、なければ対照と二つの密度場の模型を比べる
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut models: Vec<(&str, &str)> = args
        .iter()
        .map(|a| a.split_once('=').expect("名前=stem の形で渡す"))
        .collect();
    if models.is_empty() {
        models = vec![
            (
                "$A = 0$",
                "data/zeta/field_jp_2026-11-12_cosmos_a0_s10_w25_swidth",
            ),
            (
                "$G_{10}^{-}$",
                "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_w25_swidth",
            ),
            (
                "$G_{10}^{-}I_{1}$",
                "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_inh1x0.5_w25_swidth",
            ),
        ];
    }
    // 2 次メッシュ（約 90 km²）の人口
    let edges = [3_000.0, 10_000.0, 30_000.0, 100_000.0, 300_000.0];
    match pop_response(&root, &p, &models, &edges) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

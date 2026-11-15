//! Client：ノート（#94）の表 docs/tables/zeta_absorption.tex と zeta_gauss_offset.tex を書く。
//!
//! cargo run --release --bin zeta_absorption_tables
//!
//! 先に要る模擬データ（data/zeta/、どれも zeta_clustered か zeta_field が書いたもの）：
//! - 丸い塊（match の基準）：clustered_jp_2026-11-12_cosmos_m14_s15_mdens20.csv（docs/experiments/mock-density-81.md）
//! - 密度の揺らぎ：field_jp_2026-11-12_cosmos_a0.7_s10.csv（PR #90）
//! - 二つの幅 G・E：docs/field-mocks-81.md の手順
//! - ガウス場（--sigma 10 --centers 20000 --runs 40）：--amp 0.15・0.3・0.4 に --tag m94_gauss
//! - 店を半分に分けた測り方：--amp 0.3 --tag m94_split --split on
//!   共通：zeta_field -- 2026-11-12 --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv
//!   --chain COSMOS --footprint 50 --n-random 8000

use std::path::PathBuf;

use retail_cosmology::manager::three_point::absorption_tables;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match absorption_tables(&root) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

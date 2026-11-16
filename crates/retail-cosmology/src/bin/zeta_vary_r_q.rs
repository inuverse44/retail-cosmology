//! Client：三点を二点の二乗で割った Q̄_S を、コスモスと模擬データ（NS と G10、W の幅 50・25 km）で R ごとに比べる（#94）。
//!
//! Q̄_S = Σ ζ̂_S / Σ (ξ̂₁ξ̂₂ + ξ̂₂ξ̂₃ + ξ̂₃ξ̂₁)（16 ビンの和の比）。入力は zeta_vary_r_mocks と同じ
//! --s-widths の出力と、zeta_vary_r と同じコスモスの出力。新しい計算は流さない。
//! cargo run --release --bin zeta_vary_r_q
//! 書くもの：docs/tables/zeta_vary_r_q.tex と data/zeta/vary_r_mocks_q.csv

use std::path::PathBuf;

use retail_cosmology::manager::three_point::vary_s_mocks_q;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // 模型の名前の括弧の中は、模擬データの W を作った幅 R_W [km]（zeta_vary_r_mocks と同じ）。
    let models = [
        (
            "NS (50)",
            "data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_swidth",
        ),
        (
            "$G_{10}$ (50)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_swidth",
        ),
        (
            "NS (25)",
            "data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_w25_swidth",
        ),
        (
            "$G_{10}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w25_swidth",
        ),
        // R_W = 15 km（--footprint 15 --seed 3 --tag w15_swidth）
        (
            "$G_{10}$ (15)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_w15_swidth",
        ),
        // 振幅 0.4、R_W = 25 km（--amp 0.4 --footprint 25 --seed 4 --tag w25_swidth）
        (
            "$G_{10}^{0.4}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.4_s10_w25_swidth",
        ),
        // 場の幅 5 km・25 km、R_W = 25 km（--sigma 5 --seed 5、--sigma 25 --seed 6、--tag w25_swidth）
        (
            "$G_{5}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s5_w25_swidth",
        ),
        (
            "$G_{25}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s25_w25_swidth",
        ),
        // 正と負の歪みの変換（ADR 010、--transform chi --seed 7、--transform negchi --seed 8、--tag w25_swidth）
        (
            "$G_{10}^{+}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tchi_w25_swidth",
        ),
        (
            "$G_{10}^{-}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10_tnegchi_w25_swidth",
        ),
        // 負の歪みで振幅を 0.5 にしたもの（--amp 0.5 --seed 9）と、場の幅を 15 km にしたもの（--sigma 15 --seed 10）
        (
            "$G_{10}^{-,0.5}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.5_s10_tnegchi_w25_swidth",
        ),
        (
            "$G_{15}^{-}$ (25)",
            "data/zeta/field_jp_2026-11-12_cosmos_a0.7_s15_tnegchi_w25_swidth",
        ),
    ];
    match vary_s_mocks_q(&root, &models) {
        Ok(log) => print!("{log}"),
        Err(e) => panic!("{e}"),
    }
}

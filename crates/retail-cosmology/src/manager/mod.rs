//! Manager：使用例の手順（ADR 001）。Engine と ResourceAccess を呼んで手順を組み立てる。
//! Manager どうしは呼び合わない。
//!
//! 使用例は二つ：店舗の二点相関関数を測る（[`correlation`]）、店舗の数を公式の数と比べる（[`census`]）。
//! [`toy`] は推定量の確かめに使う toy model。[`cost`] は模擬データの計算量を測る診断。

pub mod census;
pub mod correlation;
pub mod cost;
pub mod toy;

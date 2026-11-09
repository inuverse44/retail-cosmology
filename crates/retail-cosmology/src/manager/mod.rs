//! Manager：使用例の手順（ADR 001）。Engine と ResourceAccess を呼んで手順を組み立てる。
//! Manager どうしは呼び合わない。
//!
//! 使用例は二つ：店舗の二点相関関数を測る（[`correlation`]）、店舗の数を公式の数と比べる（[`census`]）。
//! [`calibration`] は χ² の上側確率を模擬データで校正する。[`toy`] は推定量の確かめに使う toy model。[`cost`] は模擬データの計算量を測る診断。
//! [`random_points`] と [`selection_map`] は、ノートの図のためにランダムカタログの点と選択関数の値を書き出す。

pub mod calibration;
pub mod census;
pub mod correlation;
pub mod cost;
pub mod random_points;
pub mod selection_map;
pub mod toy;
pub mod variance;

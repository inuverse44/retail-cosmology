//! Manager：使用例の手順（ADR 001）。Engine と ResourceAccess を呼んで手順を組み立てる。
//! Manager どうしは呼び合わない。
//!
//! 使用例は二つ：店舗の二点相関関数を測る（[`correlation`]）、店舗の数を公式の数と比べる（[`census`]）。
//! [`calibration`] は χ² の上側確率を模擬データで校正し、[`short_range`] は 8 km より短い距離の値を模擬データと比べる。[`toy`] は推定量の確かめに使う toy model。[`cost`] は模擬データの計算量を測る診断。
//! [`random_points`]、[`selection_map`]、[`jk_regions`] は、ノートの図のためにランダムカタログの点、選択関数の値、jackknife の領域を書き出す。

pub mod calibration;
pub mod census;
pub mod correlation;
pub mod cost;
pub mod jk_regions;
pub mod random_points;
pub mod selection_map;
pub mod short_range;
pub mod toy;
pub mod variance;

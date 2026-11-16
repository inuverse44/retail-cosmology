//! Engine：計算の規則（ADR 001）。受け取ったデータだけで計算し、ファイルに触れない。
//! Engine どうしは呼び合わない（データの型を共有するだけ）。

pub mod census;
pub mod counts_in_cells;
pub mod density_field;
pub mod estimator;
pub mod pair_count;
pub mod random_catalog;
pub mod selection;
pub mod selection_function;
pub mod triple_count;
pub mod variance;

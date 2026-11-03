//! ResourceAccess：外のデータの読み書き（ADR 001）。
//! データの出どころの違い（ABS と e-Stat、OSM と公式サイト）はこの層の中に閉じ、
//! 上の層には契約の型（`crate::contract`）で同じ形を見せる。ResourceAccess どうしは呼び合わない。

pub mod official;
pub mod population;
pub mod results;
pub mod stores;

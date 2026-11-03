//! 店舗の座標に、宇宙論で使う二点相関関数の推定量を当てる。
//!
//! コードは変わりやすさごとに層を分けている（`docs/adr/001-layering-by-volatility.md`）。
//! 呼ぶ向きは上から下だけ。
//!
//! - Client：`src/bin/` の実行ファイル。引数を読み、Manager を呼ぶ。
//! - [`manager`]：使用例の手順（二点相関関数を測る、店舗の数を公式と比べる）。
//! - [`engine`]：計算の規則（店の選び方、ランダムカタログ、対の数え上げ、推定量と誤差）。ファイルに触れない。
//! - [`access`]：外のデータの読み書き（人口の格子、店のカタログ、公式の店舗数、結果）。
//! - [`util`]：乱数、幾何、地図の投影、格子の形。
//! - [`contract`]：層の間で受け渡すデータの形。

pub mod access;
pub mod contract;
pub mod engine;
pub mod manager;
pub mod util;

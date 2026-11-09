//! ノートの図のために、オーストラリアのランダムカタログと人口の点を書き出す（#42）。
//!
//! 乱数の引き方は [`super::correlation`] のオーストラリアの場合と同じにする：
//! 陸地に一様な基準、人口に比例する基準（どちらも `n_random` 点）、人口そのものの点（`n_pop` 点）の順。
//! したがって同じ種なら、xi の計算に使った点と同じ点が出る。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::population;
use crate::engine::random_catalog::{Weight, sample};
use crate::util::rng::SplitMix64;

/// `kind,lon,lat` の CSV の文字列を返す。kind は land、population、pop_data。
pub fn run(
    root: &Path,
    seed: u64,
    n_random: usize,
    n_pop: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let grid = population::read_abs(&root.join("data/abs/apgr25_1_0_0.tif"))?;
    let mut rng = SplitMix64::new(seed);
    let land = sample(&grid, n_random, Weight::Land, &mut rng);
    let pop = sample(&grid, n_random, Weight::Population, &mut rng);
    let pop_data = sample(&grid, n_pop, Weight::Population, &mut rng);
    let mut out = String::from("kind,lon,lat\n");
    for (kind, pts) in [
        ("land", &land),
        ("population", &pop),
        ("pop_data", &pop_data),
    ] {
        for p in pts.iter() {
            writeln!(out, "{kind},{:.5},{:.5}", p.lon, p.lat)?;
        }
    }
    Ok(out)
}

//! ノートの図のために、jackknife の領域の分け方を書き出す（#44）。
//!
//! 領域は [`super::correlation`] と同じく、人口に比例する基準の点から [`Regions::build`] で作る。
//! 乱数の引き方も同じにする：オーストラリアは陸地に一様な基準のあと、日本は人が住む格子の基準のあとに、
//! 人口に比例する基準（`n_random` 点）を引く。したがって同じ種と深さなら、xi の計算と同じ領域が出る。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::population;
use crate::engine::pair_count::Regions;
use crate::engine::random_catalog::{Weight, sample};
use crate::util::rng::SplitMix64;

/// `lon,lat,region` の CSV の文字列を返す。点は人口に比例する基準の点、region は 0 から始まる領域の番号。
pub fn run(
    root: &Path,
    country: &str,
    seed: u64,
    n_random: usize,
    depth: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let (grid, first) = match country {
        "au" => (
            population::read_abs(&root.join("data/abs/apgr25_1_0_0.tif"))?,
            Weight::Land,
        ),
        "jp" => (
            population::read_estat(&root.join("data/estat"))?,
            Weight::Inhabited,
        ),
        c => return Err(format!("知らない国：{c}").into()),
    };
    let mut rng = SplitMix64::new(seed);
    let _ = sample(&grid, n_random, first, &mut rng);
    let pop = sample(&grid, n_random, Weight::Population, &mut rng);
    let xy: Vec<(f64, f64)> = pop.iter().map(|q| grid.geometry.plane(q)).collect();
    let regions = Regions::build(&xy, depth);
    let mut out = String::from("lon,lat,region\n");
    for (p, q) in pop.iter().zip(&xy) {
        writeln!(out, "{:.5},{:.5},{}", p.lon, p.lat, regions.assign(*q))?;
    }
    Ok(out)
}

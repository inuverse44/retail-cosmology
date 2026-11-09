//! ノートの図のために、日本のチェーンの選択関数 S_R を、人口に比例して撒いた参照の点ごとに書き出す（#43）。
//!
//! 参照の点の撒き方は [`super::correlation`] の選択関数と同じにする（種 `seed ^ 0xF007_9417_0000_0000`、
//! `n_ref` 点）。したがって同じ種なら、xi の計算で S_R を求めたときと同じ参照の点を使う。
//! S_R は格子の表ではなく点ごとに求める（ノートと同じ方法）。

use std::fmt::Write as _;
use std::path::Path;

use crate::access::{population, stores};
use crate::engine::random_catalog::{Weight, sample};
use crate::engine::selection_function::SmoothRatio;
use crate::util::geo::Unit;
use crate::util::rng::SplitMix64;

/// `lon,lat,S_<R>…` の CSV の文字列を返す。行は参照の点。
pub fn run(
    root: &Path,
    store_file: &str,
    chain: &str,
    seed: u64,
    n_ref: usize,
    widths_km: &[f64],
) -> Result<String, Box<dyn std::error::Error>> {
    let grid = population::read_estat(&root.join("data/estat"))?;
    let shops: Vec<Unit> = stores::read_selected(&root.join(store_file))?
        .iter()
        .filter(|(c, _)| c == chain)
        .map(|(_, p)| Unit::from(p))
        .collect();
    if shops.is_empty() {
        return Err(format!("{store_file} に {chain} の店がない").into());
    }
    let mut rng = SplitMix64::new(seed ^ 0xF007_9417_0000_0000);
    let refs_ll = sample(&grid, n_ref, Weight::Population, &mut rng);
    let refs: Vec<Unit> = refs_ll.iter().map(Unit::from).collect();
    let srs: Vec<SmoothRatio> = widths_km
        .iter()
        .map(|r| SmoothRatio::new(&shops, &refs, *r))
        .collect();
    let mut out = String::from("lon,lat");
    for r in widths_km {
        write!(out, ",S_{r}")?;
    }
    out.push('\n');
    for (ll, u) in refs_ll.iter().zip(&refs) {
        write!(out, "{:.5},{:.5}", ll.lon, ll.lat)?;
        for sr in &srs {
            write!(out, ",{:.5}", sr.eval(u))?;
        }
        out.push('\n');
    }
    Ok(out)
}

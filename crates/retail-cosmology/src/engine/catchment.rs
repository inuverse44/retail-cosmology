//! Engine：店のまわりの人口と、より大きな町までの距離（#115、ADR 015・017）。
//!
//! 人口は格子の中心（単位ベクトル）と人口の組で受け取る。点 q のまわりの人口 P(q) は、
//! 中心が q から半径 R 以内にある格子の人口の和。より大きな町までの距離 δ_c(q) は、
//! P が c P(q) 以上の格子の中心のうち、q に最も近いものまでの距離（上限 `cap_km`）。

use crate::util::geo::{EARTH_RADIUS_KM, Unit, chord2, km_to_chord2};
use crate::util::spatial::UnitGrid;

/// 人口の格子と、半径 R のまわりの人口を求める仕組み。
pub struct PopField {
    centres: Vec<Unit>,
    pops: Vec<f64>,
    rc2: f64,
    grid: UnitGrid,
}

impl PopField {
    /// `centres`・`pops` は人口のある格子の中心と人口、`r_km` はまわりの人口を足す半径。
    pub fn new(centres: Vec<Unit>, pops: Vec<f64>, r_km: f64) -> Self {
        assert_eq!(centres.len(), pops.len());
        let rc2 = km_to_chord2(r_km);
        let grid = UnitGrid::build(&centres, rc2.sqrt());
        Self {
            centres,
            pops,
            rc2,
            grid,
        }
    }

    /// 点 q から半径 R 以内にある格子の人口の和 P(q)。
    pub fn around(&self, q: &Unit) -> f64 {
        let mut p = 0.0;
        self.grid.for_each_near(q, self.rc2.sqrt(), |j| {
            if chord2(q, &self.centres[j]) < self.rc2 {
                p += self.pops[j];
            }
        });
        p
    }

    /// 格子の中心ごとの P（[`TownDistance`] に渡す）。
    pub fn at_centres(&self) -> Vec<f64> {
        self.centres.iter().map(|c| self.around(c)).collect()
    }

    pub fn centres(&self) -> &[Unit] {
        &self.centres
    }
}

/// より大きな町までの距離 δ_c を求める仕組み。
pub struct TownDistance<'a> {
    centres: &'a [Unit],
    p_centre: &'a [f64],
    grid: UnitGrid,
    /// 探す半径 [km]（小さい順、最後が上限）
    steps_km: Vec<f64>,
}

impl<'a> TownDistance<'a> {
    /// `p_centre` は [`PopField::at_centres`]、`cap_km` は距離の上限。
    pub fn new(centres: &'a [Unit], p_centre: &'a [f64], cap_km: f64) -> Self {
        assert_eq!(centres.len(), p_centre.len());
        assert!(cap_km > 0.0);
        let cell_km = 5.0f64.min(cap_km);
        let grid = UnitGrid::build(centres, km_to_chord2(cell_km).sqrt());
        // 近くで見つかる点が多いので、半径を倍にしながら探す
        let mut steps_km = Vec::new();
        let mut r = cell_km;
        while r < cap_km {
            steps_km.push(r);
            r *= 2.0;
        }
        steps_km.push(cap_km);
        Self {
            centres,
            p_centre,
            grid,
            steps_km,
        }
    }

    /// P が `threshold` 以上の格子の中心のうち、q に最も近いものまでの距離 [km]。上限までになければ上限を返す。
    pub fn to_at_least(&self, q: &Unit, threshold: f64) -> f64 {
        for &r in &self.steps_km {
            let rc2 = km_to_chord2(r);
            let mut best = f64::INFINITY;
            self.grid.for_each_near(q, rc2.sqrt(), |j| {
                if self.p_centre[j] >= threshold {
                    let c2 = chord2(q, &self.centres[j]);
                    if c2 < rc2 && c2 < best {
                        best = c2;
                    }
                }
            });
            if best.is_finite() {
                return chord2_to_km(best);
            }
        }
        *self.steps_km.last().expect("上限がある")
    }
}

/// 弦の長さの 2 乗 → 大円距離 [km]
fn chord2_to_km(c2: f64) -> f64 {
    2.0 * EARTH_RADIUS_KM * (c2.sqrt() / 2.0).min(1.0).asin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::LatLon;

    fn at(lat: f64, lon: f64) -> Unit {
        Unit::from(&LatLon { lat, lon })
    }

    #[test]
    fn around_sums_cells_inside_the_radius() {
        // 経度 0.01 度 ≈ 0.91 km（北緯 35 度）
        let c = vec![at(35.0, 135.0), at(35.0, 135.03), at(35.0, 135.2)];
        let f = PopField::new(c, vec![100.0, 10.0, 1.0], 5.0);
        assert_eq!(f.around(&at(35.0, 135.0)), 110.0);
        assert_eq!(f.around(&at(35.0, 135.2)), 1.0);
    }

    #[test]
    fn town_distance_finds_the_nearest_larger_place() {
        // 大きな町（P = 1000）と、東へ約 18 km の小さな町（P = 10）、さらに東へ約 73 km の点（P = 1）
        let c = vec![at(35.0, 135.0), at(35.0, 135.2), at(35.0, 136.0)];
        let p = vec![1000.0, 10.0, 1.0];
        let td = TownDistance::new(&c, &p, 50.0);
        let d_small = td.to_at_least(&c[1], 2.0 * p[1]);
        let expect = chord2_to_km(chord2(&c[0], &c[1]));
        assert!((d_small - expect).abs() < 1e-9 && (expect - 18.2).abs() < 0.3);
        // いちばん大きな町には、より大きな所がないので上限
        assert_eq!(td.to_at_least(&c[0], 2.0 * p[0]), 50.0);
        // 上限より遠い所しかなければ上限
        assert_eq!(td.to_at_least(&c[2], 1000.0), 50.0);
    }
}

//! 単位球面上の点の近傍を探す。地心の単位ベクトルを一辺 `cell` の立方体の格子に分けておき、
//! 弦の長さ `radius` 以内の点を、まわりの格子だけ調べて探す（`radius <= cell` のとき 27 個の格子で足りる）。

use std::collections::HashMap;

use crate::util::geo::Unit;

pub struct UnitGrid {
    cell: f64,
    map: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl UnitGrid {
    pub fn build(points: &[Unit], cell: f64) -> Self {
        assert!(cell > 0.0);
        let mut map: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            map.entry(key(p, cell)).or_default().push(i);
        }
        Self { cell, map }
    }

    /// 点 `p` から弦の長さで `radius` 以内にありうる点の番号を、`f` に渡す（距離は呼び出し側で確かめる）。
    pub fn for_each_near(&self, p: &Unit, radius: f64, mut f: impl FnMut(usize)) {
        let reach = (radius / self.cell).ceil() as i32;
        let (x, y, z) = key(p, self.cell);
        for dx in -reach..=reach {
            for dy in -reach..=reach {
                for dz in -reach..=reach {
                    if let Some(v) = self.map.get(&(x + dx, y + dy, z + dz)) {
                        for i in v {
                            f(*i);
                        }
                    }
                }
            }
        }
    }
}

fn key(p: &Unit, cell: f64) -> (i32, i32, i32) {
    (
        (p.0[0] / cell).floor() as i32,
        (p.0[1] / cell).floor() as i32,
        (p.0[2] / cell).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::{LatLon, chord2};
    use crate::util::rng::SplitMix64;

    #[test]
    fn finds_the_same_neighbours_as_brute_force() {
        let mut rng = SplitMix64::new(4);
        let pts: Vec<Unit> = (0..3000)
            .map(|_| {
                Unit::from(&LatLon {
                    lat: -44.0 + 34.0 * rng.next_f64(),
                    lon: 113.0 + 41.0 * rng.next_f64(),
                })
            })
            .collect();
        let radius = 0.02; // 約 130 km
        let g = UnitGrid::build(&pts, radius);
        for q in pts.iter().take(50) {
            let mut fast: Vec<usize> = Vec::new();
            g.for_each_near(q, radius, |i| {
                if chord2(q, &pts[i]) < radius * radius {
                    fast.push(i);
                }
            });
            fast.sort();
            let slow: Vec<usize> = (0..pts.len())
                .filter(|i| chord2(q, &pts[*i]) < radius * radius)
                .collect();
            assert_eq!(fast, slow);
        }
    }
}

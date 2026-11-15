//! Engine：三つ組（三角形）の数え上げ（ADR 007）。DDD, DDR, DRR, RRR を三角形のビンごとに数える。
//!
//! 三角形のビンは、三辺の長さをそれぞれ同じ距離のビンに入れ、辺のビンの番号を小さい順に並べた組
//! （k₁ ≤ k₂ ≤ k₃）で決める。頂点の区別をしないので、Szapudi–Szalay の推定量の係数 3 がそのまま使える。
//! 距離は単位球の弦の長さの 2 乗で測る（ビンの端は [`crate::util::geo::km_to_chord2`] で写す）。
//!
//! 近くの点は Utility の空間格子（[`UnitGrid`]）で探し、いちばん外のビンの端より長い辺を持つ三つ組は数えない。
//! 点 i を三つ組のうち番号のいちばん小さい点（`count_aab` では A の二点のうち小さい方）として、
//! i から上限の内側にある点の組だけを調べる。三角形の三辺がどれも上限より短いことは、ビンの番号で確かめる。
//!
//! RRR を数える工夫と、jackknife で三つ組をどの領域に割り当てるかは、まだ入れていない（ADR 007 で決めないこと）。

use crate::contract::{Bins, TripleCounts};
use crate::util::geo::{Unit, chord2};
use crate::util::spatial::UnitGrid;

/// 三角形のビン。辺のビン（`sides`、弦の長さの 2 乗）を三つ並べた組を、平らな番号にする。
#[derive(Clone, Debug)]
pub struct TriangleBins {
    sides: Bins,
}

impl TriangleBins {
    /// `sides` は弦の長さの 2 乗で切った辺のビン。
    pub fn new(sides: Bins) -> Self {
        Self { sides }
    }

    pub fn sides(&self) -> &Bins {
        &self.sides
    }

    /// 平らな番号の数（n³。k₁ ≤ k₂ ≤ k₃ でない番号は使わない）。
    pub fn len(&self) -> usize {
        self.sides.len().pow(3)
    }

    pub fn is_empty(&self) -> bool {
        self.sides.is_empty()
    }

    /// 平らな番号から、辺のビンの番号の組（k₁ ≤ k₂ ≤ k₃）。
    pub fn unflatten(&self, b: usize) -> (usize, usize, usize) {
        let n = self.sides.len();
        (b / (n * n), (b / n) % n, b % n)
    }

    /// 平らな番号のうち、k₁ ≤ k₂ ≤ k₃ のものだけを並べる。
    pub fn used(&self) -> Vec<usize> {
        (0..self.len())
            .filter(|b| {
                let (a, c, d) = self.unflatten(*b);
                a <= c && c <= d
            })
            .collect()
    }

    /// 弦の長さの 2 乗で表した三辺から、平らな番号。どれかの辺がビンの外なら `None`。
    pub fn index(&self, c12: f64, c13: f64, c23: f64) -> Option<usize> {
        let mut k = [
            self.sides.index(c12)?,
            self.sides.index(c13)?,
            self.sides.index(c23)?,
        ];
        k.sort_unstable();
        let n = self.sides.len();
        Some((k[0] * n + k[1]) * n + k[2])
    }

    /// 辺の上限（いちばん外のビンの端、弦の長さの 2 乗）。
    fn max(&self) -> f64 {
        *self.sides.edges().last().unwrap()
    }
}

/// i より番号の大きい点のうち、i から上限の内側にある点（番号の小さい順）。
fn near_after(g: &UnitGrid, pts: &[Unit], i: usize, max: f64) -> Vec<usize> {
    let mut v = Vec::new();
    g.for_each_near(&pts[i], max.sqrt(), |j| {
        if j > i && chord2(&pts[i], &pts[j]) < max {
            v.push(j);
        }
    });
    v.sort_unstable();
    v
}

/// 点 p から上限の内側にある、もう一方のカタログの点。
fn near_other(g: &UnitGrid, p: &Unit, pts: &[Unit], max: f64) -> Vec<usize> {
    let mut v = Vec::new();
    g.for_each_near(p, max.sqrt(), |j| {
        if chord2(p, &pts[j]) < max {
            v.push(j);
        }
    });
    v
}

/// 同じカタログの中の三つ組（i < j < k）を数える。DDD と RRR に使う。
pub fn count_aaa(a: &[Unit], bins: &TriangleBins) -> Vec<u64> {
    let mut counts = vec![0u64; bins.len()];
    let max = bins.max();
    let g = UnitGrid::build(a, max.sqrt());
    for i in 0..a.len() {
        let nb = near_after(&g, a, i, max);
        for (x, &j) in nb.iter().enumerate() {
            let cij = chord2(&a[i], &a[j]);
            for &k in &nb[x + 1..] {
                if let Some(b) = bins.index(cij, chord2(&a[i], &a[k]), chord2(&a[j], &a[k])) {
                    counts[b] += 1;
                }
            }
        }
    }
    counts
}

/// A の二点（i < j）と B の一点の三つ組を数える。DDR は `count_aab(D, R)`、DRR は `count_aab(R, D)`。
pub fn count_aab(a: &[Unit], b: &[Unit], bins: &TriangleBins) -> Vec<u64> {
    let mut counts = vec![0u64; bins.len()];
    let max = bins.max();
    let ga = UnitGrid::build(a, max.sqrt());
    let gb = UnitGrid::build(b, max.sqrt());
    for i in 0..a.len() {
        let na = near_after(&ga, a, i, max);
        if na.is_empty() {
            continue;
        }
        let nb = near_other(&gb, &a[i], b, max);
        for &j in &na {
            let cij = chord2(&a[i], &a[j]);
            for &k in &nb {
                if let Some(t) = bins.index(cij, chord2(&a[i], &b[k]), chord2(&a[j], &b[k])) {
                    counts[t] += 1;
                }
            }
        }
    }
    counts
}

impl TripleCounts {
    /// データ `data` とランダムカタログ `randoms` から DDD, DDR, DRR, RRR を数える。
    pub fn compute(data: &[Unit], randoms: &[Unit], bins: &TriangleBins) -> Self {
        Self {
            n_d: data.len(),
            n_r: randoms.len(),
            ddd: count_aaa(data, bins),
            ddr: count_aab(data, randoms, bins),
            drr: count_aab(randoms, data, bins),
            rrr: count_aaa(randoms, bins),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::{LatLon, km_to_chord2};
    use crate::util::rng::SplitMix64;

    fn patch(n: usize, seed: u64) -> Vec<Unit> {
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| {
                Unit::from(&LatLon {
                    lat: 33.0 + 1.0 * rng.next_f64(),
                    lon: 130.0 + 1.2 * rng.next_f64(),
                })
            })
            .collect()
    }

    fn bins() -> TriangleBins {
        TriangleBins::new(Bins::linear(2.0, 40.0, 5).map(km_to_chord2))
    }

    fn brute_aaa(a: &[Unit], bins: &TriangleBins) -> Vec<u64> {
        let mut c = vec![0u64; bins.len()];
        for i in 0..a.len() {
            for j in i + 1..a.len() {
                for k in j + 1..a.len() {
                    if let Some(t) = bins.index(
                        chord2(&a[i], &a[j]),
                        chord2(&a[i], &a[k]),
                        chord2(&a[j], &a[k]),
                    ) {
                        c[t] += 1;
                    }
                }
            }
        }
        c
    }

    fn brute_aab(a: &[Unit], b: &[Unit], bins: &TriangleBins) -> Vec<u64> {
        let mut c = vec![0u64; bins.len()];
        for i in 0..a.len() {
            for j in i + 1..a.len() {
                for p in b {
                    if let Some(t) =
                        bins.index(chord2(&a[i], &a[j]), chord2(&a[i], p), chord2(&a[j], p))
                    {
                        c[t] += 1;
                    }
                }
            }
        }
        c
    }

    #[test]
    fn aaa_matches_brute_force() {
        let a = patch(300, 1);
        let b = bins();
        let fast = count_aaa(&a, &b);
        assert!(
            fast.iter().sum::<u64>() > 1000,
            "三つ組が少なすぎて確かめにならない"
        );
        assert_eq!(fast, brute_aaa(&a, &b));
    }

    #[test]
    fn aab_matches_brute_force() {
        let (a, r) = (patch(200, 2), patch(250, 3));
        let b = bins();
        assert_eq!(count_aab(&a, &r, &b), brute_aab(&a, &r, &b));
        assert_eq!(count_aab(&r, &a, &b), brute_aab(&r, &a, &b));
    }

    #[test]
    fn index_ignores_vertex_order() {
        let b = bins();
        let (x, y, z) = (km_to_chord2(5.0), km_to_chord2(15.0), km_to_chord2(30.0));
        let t = b.index(x, y, z).unwrap();
        for (p, q, r) in [(x, z, y), (y, x, z), (y, z, x), (z, x, y), (z, y, x)] {
            assert_eq!(b.index(p, q, r), Some(t));
        }
        let (k1, k2, k3) = b.unflatten(t);
        assert!(k1 <= k2 && k2 <= k3);
        assert!(b.used().contains(&t));
    }
}

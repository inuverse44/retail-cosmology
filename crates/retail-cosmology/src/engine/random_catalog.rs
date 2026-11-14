//! Engine：ランダムカタログを撒く。人口の格子を重みに比例する確率で選び、その格子の中に一様に点を置く。
//! 重みは宇宙論の選択関数にあたる。

use crate::contract::PopulationGrid;
use crate::util::geo::{LatLon, Unit, chord2, km_to_chord2};
use crate::util::rng::SplitMix64;
use crate::util::spatial::UnitGrid;

/// ランダムカタログの格子の重み。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    /// 陸地の格子すべてに等しい（人口 0 の陸地の格子を含む格子だけで使える）
    Land,
    /// 人が住む格子（人口 > 0）すべてに等しい
    Inhabited,
    /// 格子の人口に比例
    Population,
}

impl Weight {
    pub fn name(self) -> &'static str {
        match self {
            Weight::Land => "land",
            Weight::Inhabited => "inhabited",
            Weight::Population => "population",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Weight::Land => "陸地",
            Weight::Inhabited => "人が住む格子",
            Weight::Population => "人口",
        }
    }

    /// 格子の人口 v に対する重み（陸地の外は呼ばれない）
    pub fn of(self, v: f64) -> f64 {
        match self {
            Weight::Land => 1.0,
            Weight::Inhabited => (v > 0.0) as u8 as f64,
            Weight::Population => v.max(0.0),
        }
    }
}

/// 格子を重みに比例する確率で選ぶための累積の重み。
struct Sampler {
    idx: Vec<usize>,
    cum: Vec<f64>,
    acc: f64,
}

impl Sampler {
    fn new(grid: &PopulationGrid, w: Weight) -> Self {
        let mut idx = Vec::new();
        let mut cum = Vec::new();
        let mut acc = 0.0;
        for (i, v) in grid.values.iter().enumerate() {
            if *v < 0.0 {
                continue;
            }
            let wt = w.of(*v as f64);
            if wt > 0.0 {
                acc += wt;
                idx.push(i);
                cum.push(acc);
            }
        }
        Self { idx, cum, acc }
    }

    /// 一点を撒く。乱数は、格子を選ぶ 1 つと、格子の中の位置の 2 つの順に引く。
    fn draw(&self, grid: &PopulationGrid, rng: &mut SplitMix64) -> LatLon {
        let u = rng.next_f64() * self.acc;
        let k = self
            .cum
            .partition_point(|c| *c <= u)
            .min(self.cum.len() - 1);
        let a = rng.next_f64();
        let b = rng.next_f64();
        grid.geometry.point(self.idx[k], a, b)
    }
}

/// 格子を重みに比例する確率で選び、その格子の中に一様に `n` 点を撒く。陸地の外（値が負）の格子は選ばない。
pub fn sample(grid: &PopulationGrid, n: usize, w: Weight, rng: &mut SplitMix64) -> Vec<LatLon> {
    let s = Sampler::new(grid, w);
    (0..n).map(|_| s.draw(grid, rng)).collect()
}

/// 親を池から選び、その 3σ 以内からガウス重みで子を一つだけ選ぶ。
/// 各出力点に独立な親を選ぶ。近傍に `min_neighbors` 個未満しかない親は、
/// clustered_mocks と同じく使わない。これは重複なし m 個抽出の「最初の子」の分布。
/// 全候補は自身を含むので、非空の池と min_neighbors=1 なら必ず抽出できる。
pub fn sample_conditional_children(
    pool: &[Unit],
    spatial: &UnitGrid,
    sigma_km: f64,
    min_neighbors: usize,
    n: usize,
    rng: &mut SplitMix64,
) -> Result<Vec<Unit>, String> {
    if pool.is_empty()
        || min_neighbors == 0
        || min_neighbors > pool.len()
        || !sigma_km.is_finite()
        || sigma_km <= 0.0
    {
        return Err("条件付き抽出の池・近傍数・σが不正".into());
    }
    let cut = km_to_chord2(3.0 * sigma_km);
    let two_s2 = 2.0 * km_to_chord2(sigma_km);
    let mut out = Vec::with_capacity(n);
    let mut attempts = 0usize;
    while out.len() < n {
        attempts += 1;
        if attempts > 100_000 * n.max(1) {
            return Err("条件付き抽出で採用できる親が少なすぎる".into());
        }
        let parent = pool[(rng.next_f64() * pool.len() as f64) as usize];
        let mut near = Vec::new();
        spatial.for_each_near(&parent, cut.sqrt(), |i| {
            let c2 = chord2(&parent, &pool[i]);
            if c2 < cut {
                near.push((i, (-c2 / two_s2).exp()));
            }
        });
        if near.len() < min_neighbors {
            continue;
        }
        let total: f64 = near.iter().map(|x| x.1).sum();
        let mut u = rng.next_f64() * total;
        let mut pick = near.len() - 1;
        for (j, x) in near.iter().enumerate() {
            if u < x.1 {
                pick = j;
                break;
            }
            u -= x.1;
        }
        out.push(pool[near[pick].0]);
    }
    Ok(out)
}

/// 単位球面上の点を、人口格子と同じ約 10 km の粗い格子に数える。
pub fn count_coarse_cells(
    points: &[Unit],
    geometry: &crate::util::grid::GridGeometry,
) -> std::collections::BTreeMap<u64, u64> {
    let mut counts = std::collections::BTreeMap::new();
    for u in points {
        let q = LatLon {
            lat: u.0[2].clamp(-1.0, 1.0).asin().to_degrees(),
            lon: u.0[1].atan2(u.0[0]).to_degrees(),
        };
        if let Some((key, _)) = geometry.coarse_cell(&q) {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

/// 選択関数をかけて撒いた結果。
pub struct Selected {
    pub points: Vec<LatLon>,
    /// 撒いた候補の数
    pub candidates: usize,
    /// 選択関数が、最初の候補で決めた上限を超えた候補の数（上限で打ち切って受け入れた）
    pub clipped: usize,
}

/// 重み `w` × 選択関数 `s` に比例する分布から `n` 点を撒く（棄却法）。
/// `w` に比例して候補を撒き、確率 s(候補) / s_max で受け入れる。s_max は最初の `pilot` 個の候補の最大値。
pub fn sample_selected(
    grid: &PopulationGrid,
    n: usize,
    w: Weight,
    s: impl Fn(&LatLon) -> f64,
    pilot: usize,
    rng: &mut SplitMix64,
) -> Selected {
    let sp = Sampler::new(grid, w);
    let first: Vec<(LatLon, f64)> = (0..pilot)
        .map(|_| {
            let p = sp.draw(grid, rng);
            let v = s(&p);
            (p, v)
        })
        .collect();
    let s_max = first.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    assert!(s_max > 0.0, "選択関数が候補のどこでも 0");
    let mut out = Selected {
        points: Vec::with_capacity(n),
        candidates: 0,
        clipped: 0,
    };
    let mut fresh = std::iter::from_fn(|| {
        let p = sp.draw(grid, rng);
        let v = s(&p);
        Some((p, v))
    });
    let mut accept_rng = SplitMix64::new(0x5EED_5E1E_C7ED);
    let mut i = 0;
    while out.points.len() < n {
        let (p, v) = if i < first.len() {
            first[i]
        } else {
            fresh.next().unwrap()
        };
        i += 1;
        out.candidates += 1;
        if v > s_max {
            out.clipped += 1;
        }
        if accept_rng.next_f64() * s_max < v {
            out.points.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::albers::Albers;
    use crate::util::grid::{GridGeometry, Projection};

    #[test]
    fn conditional_children_follow_the_parent_normalized_kernel() {
        // 非一様な有限の池。親を平均した厳密な一点の確率と比較する。
        let pool: Vec<Unit> = [0.0, 0.01, 0.04, 0.09]
            .iter()
            .map(|lon| {
                Unit::from(&LatLon {
                    lat: 33.0,
                    lon: 130.0 + lon,
                })
            })
            .collect();
        let sigma = 4.0;
        let cut = km_to_chord2(3.0 * sigma);
        let spatial = UnitGrid::build(&pool, cut.sqrt());
        let mut exact = vec![0.0; pool.len()];
        for parent in &pool {
            let weights: Vec<_> = pool
                .iter()
                .map(|q| {
                    let c2 = chord2(parent, q);
                    if c2 < cut {
                        (-c2 / (2.0 * km_to_chord2(sigma))).exp()
                    } else {
                        0.0
                    }
                })
                .collect();
            let total: f64 = weights.iter().sum();
            for (p, w) in exact.iter_mut().zip(weights) {
                *p += w / total / pool.len() as f64;
            }
        }
        let mut rng = SplitMix64::new(91);
        let draws =
            sample_conditional_children(&pool, &spatial, sigma, 1, 80_000, &mut rng).unwrap();
        let mut counts = vec![0usize; pool.len()];
        for q in draws {
            counts[pool.iter().position(|p| *p == q).unwrap()] += 1;
        }
        for (count, p) in counts.iter().zip(exact) {
            assert!((*count as f64 / 80_000.0 - p).abs() < 0.006);
        }
    }

    #[test]
    fn conditional_children_reject_parents_with_too_few_neighbors() {
        let pool: Vec<Unit> = [130.0, 130.001, 140.0]
            .iter()
            .map(|lon| {
                Unit::from(&LatLon {
                    lat: 33.0,
                    lon: *lon,
                })
            })
            .collect();
        let spatial = UnitGrid::build(&pool, km_to_chord2(3.0).sqrt());
        let mut rng = SplitMix64::new(8);
        let out = sample_conditional_children(&pool, &spatial, 1.0, 2, 500, &mut rng).unwrap();
        assert!(out.iter().all(|q| *q == pool[0] || *q == pool[1]));
        assert!(sample_conditional_children(&pool, &spatial, 1.0, 4, 1, &mut rng).is_err());
    }

    fn tiny() -> PopulationGrid {
        // 2 × 2 の格子：左上は海、右上は無人、左下に 1 人、右下に 3 人
        PopulationGrid {
            values: vec![-1.0, 0.0, 1.0, 3.0],
            geometry: GridGeometry::Planar {
                width: 2,
                height: 2,
                x0: 0.0,
                y0: -3_000_000.0,
                dx: 1000.0,
                dy: 1000.0,
                proj: Projection::Albers(Albers::epsg3577()),
            },
            has_land: true,
        }
    }

    #[test]
    fn samples_follow_weights() {
        let g = tiny();
        let mut rng = SplitMix64::new(7);
        let n = 40_000;
        let count = |pts: &[LatLon]| {
            let mut c = [0usize; 4];
            for p in pts {
                c[g.geometry.cell_of(p).unwrap()] += 1;
            }
            c
        };
        let c = count(&sample(&g, n, Weight::Population, &mut rng));
        assert_eq!((c[0], c[1]), (0, 0));
        let frac = c[3] as f64 / n as f64; // 期待値 3/4、標準偏差 0.002
        assert!((frac - 0.75).abs() < 0.01, "{c:?}");

        let c = count(&sample(&g, n, Weight::Land, &mut rng));
        assert_eq!(c[0], 0);
        for k in 1..4 {
            let frac = c[k] as f64 / n as f64; // 期待値 1/3
            assert!((frac - 1.0 / 3.0).abs() < 0.01, "{c:?}");
        }

        let c = count(&sample(&g, n, Weight::Inhabited, &mut rng));
        assert_eq!((c[0], c[1]), (0, 0));
        let frac = c[3] as f64 / n as f64; // 期待値 1/2
        assert!((frac - 0.5).abs() < 0.01, "{c:?}");
    }

    #[test]
    fn selection_reweights_cells() {
        // 人口に比例（左下 1、右下 3）× 選択関数（左下 3、右下 1）→ 左下と右下が同じ重み
        let g = tiny();
        let mut rng = SplitMix64::new(9);
        let n = 40_000;
        let s = |p: &LatLon| {
            if g.geometry.cell_of(p) == Some(2) {
                3.0
            } else {
                1.0
            }
        };
        let sel = sample_selected(&g, n, Weight::Population, s, 1000, &mut rng);
        assert_eq!(sel.points.len(), n);
        assert_eq!(sel.clipped, 0);
        let left = sel
            .points
            .iter()
            .filter(|p| g.geometry.cell_of(p) == Some(2))
            .count();
        let frac = left as f64 / n as f64; // 期待値 1/2
        assert!((frac - 0.5).abs() < 0.01, "{frac}");
    }
}

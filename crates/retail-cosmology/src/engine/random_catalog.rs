//! Engine：ランダムカタログを撒く。人口の格子を重みに比例する確率で選び、その格子の中に一様に点を置く。
//! 重みは宇宙論の選択関数にあたる。

use crate::contract::PopulationGrid;
use crate::util::geo::LatLon;
use crate::util::rng::SplitMix64;

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

/// 格子を重みに比例する確率で選び、その格子の中に一様に `n` 点を撒く。陸地の外（値が負）の格子は選ばない。
/// 乱数は一点ごとに、格子を選ぶ 1 つと、格子の中の位置の 2 つの順に引く。
pub fn sample(grid: &PopulationGrid, n: usize, w: Weight, rng: &mut SplitMix64) -> Vec<LatLon> {
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
    (0..n)
        .map(|_| {
            let u = rng.next_f64() * acc;
            let k = cum.partition_point(|c| *c <= u).min(cum.len() - 1);
            let a = rng.next_f64();
            let b = rng.next_f64();
            grid.geometry.point(idx[k], a, b)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::albers::Albers;
    use crate::util::grid::GridGeometry;

    fn tiny() -> PopulationGrid {
        // 2 × 2 の格子：左上は海、右上は無人、左下に 1 人、右下に 3 人
        PopulationGrid {
            values: vec![-1.0, 0.0, 1.0, 3.0],
            geometry: GridGeometry::Albers {
                width: 2,
                height: 2,
                x0: 0.0,
                y0: -3_000_000.0,
                dx: 1000.0,
                dy: 1000.0,
                proj: Albers::epsg3577(),
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
}

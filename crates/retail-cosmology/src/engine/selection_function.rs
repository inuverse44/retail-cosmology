//! Engine：選択関数。店の密度と人口の密度を、それぞれ幅 R のガウス関数でならし、その比
//! S(x) = n̄_店(x) / n̄_人口(x) を返す。S を人口に掛けた重みでランダムカタログを撒くと、
//! 「どの地方に、人口あたりどれだけ出店しているか」が基準に入り、R より短い距離の店どうしの並び方だけが残る。
//! 人口の密度は、人口に比例して撒いた点（参照の点）で表す。

use crate::util::geo::{Unit, chord2, km_to_chord2};
use crate::util::spatial::UnitGrid;

/// ガウス関数を打ち切る位置（標準偏差の何倍か）
const CUTOFF_SIGMA: f64 = 3.0;

pub struct SmoothRatio {
    stores: Vec<Unit>,
    refs: Vec<Unit>,
    g_stores: UnitGrid,
    g_refs: UnitGrid,
    /// 弦の長さで表した幅の 2 乗の 2 倍（2σ²）と、打ち切りの半径
    two_sigma2: f64,
    cutoff: f64,
}

impl SmoothRatio {
    /// `r_km` はならす幅（ガウス関数の標準偏差）[km]。
    pub fn new(stores: &[Unit], refs: &[Unit], r_km: f64) -> Self {
        let sigma2 = km_to_chord2(r_km);
        let cutoff = CUTOFF_SIGMA * sigma2.sqrt();
        Self {
            stores: stores.to_vec(),
            refs: refs.to_vec(),
            g_stores: UnitGrid::build(stores, cutoff),
            g_refs: UnitGrid::build(refs, cutoff),
            two_sigma2: 2.0 * sigma2,
            cutoff,
        }
    }

    fn smooth(&self, p: &Unit, pts: &[Unit], g: &UnitGrid) -> f64 {
        let c2max = self.cutoff * self.cutoff;
        let mut s = 0.0;
        g.for_each_near(p, self.cutoff, |i| {
            let c2 = chord2(p, &pts[i]);
            if c2 < c2max {
                s += (-c2 / self.two_sigma2).exp();
            }
        });
        s / pts.len() as f64
    }

    /// S(p)。近くに参照の点がなければ 0。
    pub fn eval(&self, p: &Unit) -> f64 {
        let den = self.smooth(p, &self.refs, &self.g_refs);
        if den <= 0.0 {
            0.0
        } else {
            self.smooth(p, &self.stores, &self.g_stores) / den
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::LatLon;
    use crate::util::rng::SplitMix64;

    fn uniform(n: usize, lon: (f64, f64), rng: &mut SplitMix64) -> Vec<Unit> {
        (0..n)
            .map(|_| {
                Unit::from(&LatLon {
                    lat: -35.0 + 4.0 * rng.next_f64(),
                    lon: lon.0 + (lon.1 - lon.0) * rng.next_f64(),
                })
            })
            .collect()
    }

    #[test]
    fn recovers_a_step_in_stores_per_person() {
        // 人口は経度 140°〜150° に一様。店は西半分（140°〜145°）に東半分の 3 倍の密度で置く。
        let mut rng = SplitMix64::new(8);
        let refs = uniform(40_000, (140.0, 150.0), &mut rng);
        let mut stores = uniform(3000, (140.0, 145.0), &mut rng);
        stores.extend(uniform(1000, (145.0, 150.0), &mut rng));
        let s = SmoothRatio::new(&stores, &refs, 30.0);
        let at = |lon: f64| s.eval(&Unit::from(&LatLon { lat: -33.0, lon }));
        // 境界から十分離れた所では、比は 3 になる（ならした幅 30 km は経度 0.3° ほど）
        let ratio = at(142.0) / at(148.0);
        assert!((ratio - 3.0).abs() < 0.3, "比 {ratio}");
        // 人口のない所は 0
        assert_eq!(
            s.eval(&Unit::from(&LatLon {
                lat: -33.0,
                lon: 120.0
            })),
            0.0
        );
    }
}

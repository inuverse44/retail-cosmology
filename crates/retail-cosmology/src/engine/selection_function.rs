//! Engine：選択関数。店の密度と人口の密度を、それぞれ幅 R のガウス関数でならし、その比
//! S(x) = n̄_店(x) / n̄_人口(x) を返す。S を人口に掛けた重みでランダムカタログを撒くと、
//! 「どの地方に、人口あたりどれだけ出店しているか」が基準に入り、R より短い距離の店どうしの並び方だけが残る。
//! 人口の密度は、人口に比例して撒いた点（参照の点）で表す。

use std::cell::RefCell;
use std::collections::HashMap;

use crate::util::geo::{LatLon, Unit, chord2, km_to_chord2};
use crate::util::grid::GridGeometry;
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

    /// S(p) を求めるときに調べる点の数と、そのうち打ち切りの半径の内側にあって足し合わせる点の数
    /// （店と参照の点の合計）。計算量を見積もるための診断。
    pub fn eval_cost(&self, p: &Unit) -> (usize, usize) {
        let c2max = self.cutoff * self.cutoff;
        let (mut visited, mut inside) = (0, 0);
        for (pts, g) in [(&self.refs, &self.g_refs), (&self.stores, &self.g_stores)] {
            g.for_each_near(p, self.cutoff, |i| {
                visited += 1;
                if chord2(p, &pts[i]) < c2max {
                    inside += 1;
                }
            });
        }
        (visited, inside)
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

/// 地方内では一定の店舗数 / 人口。局所的な店の並び方には依存しない。
/// メッシュの地方と地方人口は Manager が外部データから用意する。
pub struct RegionalRatio {
    mesh_region: HashMap<u64, usize>,
    ratios: Vec<f64>,
    pub counts: Vec<usize>,
}

impl RegionalRatio {
    pub fn new(
        stores: &[LatLon],
        mesh_region: &HashMap<u64, usize>,
        population: &[f64],
    ) -> Result<Self, String> {
        if population.iter().any(|p| !p.is_finite() || *p < 0.0)
            || mesh_region.values().any(|r| *r >= population.len())
        {
            return Err("地方の人口または対応が不正".into());
        }
        let mut counts = vec![0usize; population.len()];
        for q in stores {
            let r = mesh_region
                .get(&crate::util::grid::code_of(q))
                .ok_or_else(|| format!("店舗のメッシュに地方の対応がない：{},{}", q.lat, q.lon))?;
            if *r >= population.len() || population[*r] <= 0.0 {
                return Err(format!("店舗がある地方 {r} の人口がない"));
            }
            counts[*r] += 1;
        }
        let ratios = counts
            .iter()
            .zip(population)
            .map(|(n, pop)| if *pop > 0.0 { *n as f64 / pop } else { 0.0 })
            .collect();
        Ok(Self {
            mesh_region: mesh_region.clone(),
            ratios,
            counts,
        })
    }

    pub fn eval(&self, q: &LatLon) -> f64 {
        self.mesh_region
            .get(&crate::util::grid::code_of(q))
            .map_or(0.0, |r| self.ratios[*r])
    }

    /// 格子ごとの追加重み。人口は抽出 Engine 側で掛ける。
    pub fn cell_weights(&self, geometry: &GridGeometry, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| self.eval(&geometry.point(i, 0.5, 0.5)))
            .collect()
    }
}

/// 選択関数を約 10 km の粗い格子ごとに一度だけ求め、格子の中では同じ値を返す（#12）。
/// S は幅 R（50 km 以上）でならした関数なので、格子の中ではほとんど変わらない、という近似。
/// 格子の値は、初めて聞かれたときに格子の中心で求めて覚えておく。
pub struct Tabulated<'a> {
    sr: &'a SmoothRatio,
    geometry: &'a GridGeometry,
    cache: RefCell<HashMap<u64, f64>>,
}

impl<'a> Tabulated<'a> {
    pub fn new(sr: &'a SmoothRatio, geometry: &'a GridGeometry) -> Self {
        Self {
            sr,
            geometry,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// 点 p が入る粗い格子の S。粗い格子の外なら、その点で厳密に求める。
    pub fn eval(&self, p: &LatLon) -> f64 {
        match self.geometry.coarse_cell(p) {
            Some((key, center)) => *self
                .cache
                .borrow_mut()
                .entry(key)
                .or_insert_with(|| self.sr.eval(&Unit::from(&center))),
            None => self.sr.eval(&Unit::from(p)),
        }
    }

    /// これまでに S を求めた粗い格子の数。
    pub fn n_cells(&self) -> usize {
        self.cache.borrow().len()
    }
}

/// 重み `w` をつけた点で、ln ρ の重みつき平均。重みにはさらに ρ^γ を掛ける（`log_rho` は ln ρ）。
pub fn mean_log_density(log_rho: &[f64], w: &[f64], gamma: f64) -> f64 {
    let (mut num, mut den) = (0.0, 0.0);
    for (l, wi) in log_rho.iter().zip(w) {
        let wt = wi * (gamma * l).exp();
        num += wt * l;
        den += wt;
    }
    num / den
}

/// 店が人の密集地にどれだけ寄るかを表す指数 γ を求める。
/// 基準（重み `w` の点）に ρ^γ を掛けたときの ln ρ の平均が、店の ln ρ の平均 `target` に等しくなる γ。
/// 平均は γ について単調に増える（γ で微分すると重みつきの分散になる）ので、二分法で解く。
/// 範囲 [-`limit`, `limit`] で解けなければ、近い方の端を返す。
pub fn fit_density_exponent(log_rho: &[f64], w: &[f64], target: f64, limit: f64) -> f64 {
    let (mut lo, mut hi) = (-limit, limit);
    if mean_log_density(log_rho, w, lo) >= target {
        return lo;
    }
    if mean_log_density(log_rho, w, hi) <= target {
        return hi;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if mean_log_density(log_rho, w, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::LatLon;
    use crate::util::rng::SplitMix64;

    #[test]
    fn regional_ratio_uses_only_totals_and_conserves_store_count() {
        use crate::util::grid::{code_of, code_to_sw};
        let point = |code, a: f64| {
            let (lat, lon) = code_to_sw(code);
            LatLon {
                lat: lat + a / 120.0,
                lon: lon + a / 80.0,
            }
        };
        let a = point(53394611, 0.2);
        let b = point(53394612, 0.2);
        let c = point(53394613, 0.2);
        let map = HashMap::from([(code_of(&a), 0), (code_of(&b), 0), (code_of(&c), 1)]);
        let pop = [100.0, 300.0, 50.0];
        let first = RegionalRatio::new(&[a, a, c], &map, &pop).unwrap();
        let moved = RegionalRatio::new(&[b, b, c], &map, &pop).unwrap();
        assert_eq!(first.counts, vec![2, 1, 0]);
        assert_eq!(first.ratios, moved.ratios);
        assert_eq!(first.eval(&a), first.eval(&b));
        assert!(
            (first
                .ratios
                .iter()
                .zip(pop)
                .map(|(s, p)| s * p)
                .sum::<f64>()
                - 3.0)
                .abs()
                < 1e-12
        );
        assert!(RegionalRatio::new(&[point(53394614, 0.2)], &map, &pop).is_err());
        assert!(RegionalRatio::new(&[a], &map, &[0.0, 300.0, 50.0]).is_err());
    }

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
    fn tabulated_returns_the_exact_value_at_the_cell_centre() {
        // 同じ粗い格子の点はすべて、その格子の中心で厳密に求めた値になり、計算は格子ごとに 1 回。
        use crate::util::grid::GridGeometry;
        use std::collections::HashMap;
        let mut rng = SplitMix64::new(12);
        let mk = |n: usize, rng: &mut SplitMix64| -> Vec<Unit> {
            (0..n)
                .map(|_| {
                    Unit::from(&LatLon {
                        lat: 35.0 + rng.next_f64(),
                        lon: 139.0 + rng.next_f64(),
                    })
                })
                .collect()
        };
        let refs = mk(5000, &mut rng);
        let stores = mk(300, &mut rng);
        let sr = SmoothRatio::new(&stores, &refs, 30.0);
        let g = GridGeometry::Mesh {
            sw: Vec::new(),
            index: HashMap::new(),
        };
        let tab = Tabulated::new(&sr, &g);
        let a = LatLon {
            lat: 35.681,
            lon: 139.767,
        };
        let b = LatLon {
            lat: 35.70,
            lon: 139.80,
        }; // 同じ 2 次メッシュ 533946
        let (_, c) = g.coarse_cell(&a).unwrap();
        let exact = sr.eval(&Unit::from(&c));
        assert_eq!(tab.eval(&a), exact);
        assert_eq!(tab.eval(&b), exact);
        assert_eq!(tab.n_cells(), 1);
    }

    #[test]
    fn density_exponent_recovers_the_power_used_to_draw() {
        // ln ρ が 0〜10 に一様な点を基準にし、ρ^γ の重みで選んだ点の平均を目標にすると γ が戻る。
        let mut rng = SplitMix64::new(4);
        let lr: Vec<f64> = (0..20_000).map(|_| 10.0 * rng.next_f64()).collect();
        let w = vec![1.0; lr.len()];
        for g in [-0.5, 0.0, 0.3] {
            let target = mean_log_density(&lr, &w, g);
            let fit = fit_density_exponent(&lr, &w, target, 3.0);
            assert!((fit - g).abs() < 1e-9, "{g} {fit}");
        }
        // 平均が範囲の外なら端を返す
        assert_eq!(fit_density_exponent(&lr, &w, 20.0, 3.0), 3.0);
        assert_eq!(fit_density_exponent(&lr, &w, -1.0, 3.0), -3.0);
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

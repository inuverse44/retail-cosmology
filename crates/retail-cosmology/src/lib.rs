//! 店舗座標の二点相関関数を、宇宙論の推定量で数える。
//!
//! 対の数 DD, DR, RR を距離のビンごとに数え、それぞれ全対数で割ってから
//! 推定量（natural, Davis-Peebles, Hamilton, Landy-Szalay）に入れる。
//! 距離の測り方は呼び出し側が関数で渡す（平面の toy model では [`euclid`]、
//! 地表の店舗では [`great_circle_km`]）。

pub mod albers;
pub mod jackknife;
pub mod mesh;
pub mod osm;
pub mod popgrid;
pub mod rng;
pub mod trial_site;

/// 地球の平均半径 [km]（IUGG の平均半径 R1）。
pub const EARTH_RADIUS_KM: f64 = 6371.0088;

/// 緯度・経度 [度]。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

/// 平面上の点。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xy {
    pub x: f64,
    pub y: f64,
}

/// 平面のユークリッド距離。
pub fn euclid(a: &Xy, b: &Xy) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// 大円距離 [km]（haversine）。
pub fn great_circle_km(a: &LatLon, b: &LatLon) -> f64 {
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dp = p2 - p1;
    let dl = (b.lon - a.lon).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().min(1.0).asin()
}

/// 単位球面上の点（地心の単位ベクトル）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit(pub [f64; 3]);

impl From<&LatLon> for Unit {
    fn from(p: &LatLon) -> Self {
        let (la, lo) = (p.lat.to_radians(), p.lon.to_radians());
        Unit([la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()])
    }
}

/// 弦の長さの 2 乗。大円距離の単調増加関数なので、ビンの端を [`km_to_chord2`] で写せば
/// 三角関数を使わずに大円距離のビンで数えられる。
pub fn chord2(a: &Unit, b: &Unit) -> f64 {
    let d = [a.0[0] - b.0[0], a.0[1] - b.0[1], a.0[2] - b.0[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// 大円距離 [km] → 単位球の弦の長さの 2 乗
pub fn km_to_chord2(d: f64) -> f64 {
    (2.0 * (d / (2.0 * EARTH_RADIUS_KM)).sin()).powi(2)
}

/// 距離のビン。`edges` は狭義単調増加で、ビン k は `[edges[k], edges[k+1])`。
#[derive(Clone, Debug)]
pub struct Bins {
    edges: Vec<f64>,
}

impl Bins {
    pub fn new(edges: Vec<f64>) -> Self {
        assert!(edges.len() >= 2, "ビンの端は 2 つ以上");
        assert!(
            edges.windows(2).all(|w| w[0] < w[1]),
            "ビンの端は狭義単調増加"
        );
        Self { edges }
    }

    /// `[lo, hi)` を `n` 等分する。
    pub fn linear(lo: f64, hi: f64, n: usize) -> Self {
        Self::new(
            (0..=n)
                .map(|k| lo + (hi - lo) * k as f64 / n as f64)
                .collect(),
        )
    }

    /// `[lo, hi)` を対数で `n` 等分する。
    pub fn log(lo: f64, hi: f64, n: usize) -> Self {
        let (a, b) = (lo.ln(), hi.ln());
        Self::new(
            (0..=n)
                .map(|k| (a + (b - a) * k as f64 / n as f64).exp())
                .collect(),
        )
    }

    /// 端を単調増加の関数 `f` で写したビン。距離の代わりに、その単調な関数で数えるときに使う。
    pub fn map(&self, f: impl Fn(f64) -> f64) -> Self {
        Self::new(self.edges.iter().map(|e| f(*e)).collect())
    }

    pub fn len(&self) -> usize {
        self.edges.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn edges(&self) -> &[f64] {
        &self.edges
    }

    /// ビンの中点。
    pub fn centers(&self) -> Vec<f64> {
        self.edges.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect()
    }

    /// 距離 `r` が入るビンの番号。範囲外なら `None`。
    pub fn index(&self, r: f64) -> Option<usize> {
        if !(r >= self.edges[0] && r < self.edges[self.edges.len() - 1]) {
            return None;
        }
        // edges[k] <= r となる最大の k
        Some(self.edges.partition_point(|&e| e <= r) - 1)
    }
}

/// 同じカタログの中の対（i < j）を数える。DD と RR に使う。
pub fn count_auto<P>(points: &[P], bins: &Bins, dist: impl Fn(&P, &P) -> f64) -> Vec<u64> {
    let mut counts = vec![0u64; bins.len()];
    for (i, a) in points.iter().enumerate() {
        for b in &points[i + 1..] {
            if let Some(k) = bins.index(dist(a, b)) {
                counts[k] += 1;
            }
        }
    }
    counts
}

/// 2 つのカタログの間の対を数える。DR と、Coles × Woolworths の相互相関に使う。
pub fn count_cross<P>(a: &[P], b: &[P], bins: &Bins, dist: impl Fn(&P, &P) -> f64) -> Vec<u64> {
    let mut counts = vec![0u64; bins.len()];
    for p in a {
        for q in b {
            if let Some(k) = bins.index(dist(p, q)) {
                counts[k] += 1;
            }
        }
    }
    counts
}

/// ビンごとの DD, DR, RR と、データ数 `n_d`・ランダム数 `n_r`。
#[derive(Clone, Debug)]
pub struct PairCounts {
    pub n_d: usize,
    pub n_r: usize,
    pub dd: Vec<u64>,
    pub dr: Vec<u64>,
    pub rr: Vec<u64>,
}

impl PairCounts {
    pub fn compute<P>(
        data: &[P],
        randoms: &[P],
        bins: &Bins,
        dist: impl Fn(&P, &P) -> f64,
    ) -> Self {
        Self {
            n_d: data.len(),
            n_r: randoms.len(),
            dd: count_auto(data, bins, &dist),
            dr: count_cross(data, randoms, bins, &dist),
            rr: count_auto(randoms, bins, &dist),
        }
    }

    /// 全対数で割った (dd, dr, rr)。全対数は n_d(n_d-1)/2, n_d n_r, n_r(n_r-1)/2。
    fn normalized(&self, k: usize) -> (f64, f64, f64) {
        let (nd, nr) = (self.n_d as f64, self.n_r as f64);
        (
            self.dd[k] as f64 / (nd * (nd - 1.0) / 2.0),
            self.dr[k] as f64 / (nd * nr),
            self.rr[k] as f64 / (nr * (nr - 1.0) / 2.0),
        )
    }

    fn map(&self, f: impl Fn(f64, f64, f64) -> f64) -> Vec<f64> {
        (0..self.dd.len())
            .map(|k| {
                let (dd, dr, rr) = self.normalized(k);
                f(dd, dr, rr)
            })
            .collect()
    }

    /// natural: dd/rr - 1
    pub fn natural(&self) -> Vec<f64> {
        self.map(|dd, _, rr| dd / rr - 1.0)
    }

    /// Davis-Peebles: dd/dr - 1
    pub fn davis_peebles(&self) -> Vec<f64> {
        self.map(|dd, dr, _| dd / dr - 1.0)
    }

    /// Hamilton: dd rr / dr^2 - 1
    pub fn hamilton(&self) -> Vec<f64> {
        self.map(|dd, dr, rr| dd * rr / (dr * dr) - 1.0)
    }

    /// Landy-Szalay: (dd - 2 dr + rr) / rr
    pub fn landy_szalay(&self) -> Vec<f64> {
        self.map(|dd, dr, rr| (dd - 2.0 * dr + rr) / rr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn uniform_square(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
    }

    #[test]
    fn bins_index_is_half_open() {
        let b = Bins::linear(0.0, 1.0, 4);
        assert_eq!(b.index(0.0), Some(0));
        assert_eq!(b.index(0.25), Some(1));
        assert_eq!(b.index(0.999), Some(3));
        assert_eq!(b.index(1.0), None);
        assert_eq!(b.index(-0.1), None);
        assert_eq!(b.index(f64::NAN), None);
    }

    #[test]
    fn counts_by_hand() {
        // 3 点：距離 1, 1, sqrt(2)
        let p = [
            Xy { x: 0.0, y: 0.0 },
            Xy { x: 1.0, y: 0.0 },
            Xy { x: 0.0, y: 1.0 },
        ];
        let b = Bins::new(vec![0.5, 1.2, 2.0]);
        assert_eq!(count_auto(&p, &b, euclid), vec![2, 1]);
        assert_eq!(count_cross(&p[..1], &p[1..], &b, euclid), vec![2, 0]);
    }

    #[test]
    fn great_circle_quarter_meridian() {
        // 赤道から北極まで：円周の 1/4
        let d = great_circle_km(
            &LatLon { lat: 0.0, lon: 0.0 },
            &LatLon {
                lat: 90.0,
                lon: 0.0,
            },
        );
        assert!((d - std::f64::consts::FRAC_PI_2 * EARTH_RADIUS_KM).abs() < 1e-9);
    }

    #[test]
    fn chord_bins_agree_with_great_circle() {
        let mut rng = SplitMix64::new(3);
        let pts: Vec<LatLon> = (0..300)
            .map(|_| LatLon {
                lat: -44.0 + 34.0 * rng.next_f64(),
                lon: 113.0 + 41.0 * rng.next_f64(),
            })
            .collect();
        let units: Vec<Unit> = pts.iter().map(Unit::from).collect();
        let bins = Bins::log(1.0, 4000.0, 12);
        let a = count_auto(&pts, &bins, great_circle_km);
        let b = count_auto(&units, &bins.map(km_to_chord2), chord2);
        assert_eq!(a, b);
    }

    #[test]
    fn landy_szalay_vanishes_for_poisson_toy_model() {
        // 正方形に一様に撒いた点（相関なし）では、どの推定量も 0 のまわりに散る。
        let mut rng = SplitMix64::new(1);
        let data = uniform_square(2000, &mut rng);
        let randoms = uniform_square(4000, &mut rng);
        let bins = Bins::linear(0.02, 0.2, 9);
        let c = PairCounts::compute(&data, &randoms, &bins, euclid);
        for (k, xi) in c.landy_szalay().iter().enumerate() {
            // 各ビンの DD は 1e4 対以上あり、ポアソン的な揺らぎは 1e-2 程度
            assert!(xi.abs() < 0.05, "bin {k}: xi_LS = {xi}");
        }
    }

    #[test]
    fn estimators_agree_on_exact_counts() {
        // 期待値どおりの対数（dd = (1+xi) rr, dr = rr）なら、全推定量が xi を返す。
        let xi = 0.3;
        let (nd, nr) = (101usize, 201usize);
        let rr_frac = 0.01;
        let c = PairCounts {
            n_d: nd,
            n_r: nr,
            dd: vec![((1.0 + xi) * rr_frac * (nd * (nd - 1) / 2) as f64).round() as u64],
            dr: vec![(rr_frac * (nd * nr) as f64).round() as u64],
            rr: vec![(rr_frac * (nr * (nr - 1) / 2) as f64).round() as u64],
        };
        for est in [
            c.natural(),
            c.davis_peebles(),
            c.hamilton(),
            c.landy_szalay(),
        ] {
            assert!((est[0] - xi).abs() < 0.01, "{est:?}");
        }
    }
}

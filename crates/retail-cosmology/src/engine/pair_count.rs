//! Engine：対の数え上げ。距離のビンごとに DD, DR, RR を数える。
//! jackknife のために、領域の組ごとにも数える（領域 k を除いた数は、数え直さずに求まる）。

use crate::contract::PairCounts;

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
}

/// 平面を、与えた点の数が等しくなるように縦横交互に二等分して作る領域。
/// 2^depth 個の領域ができる。
#[derive(Clone, Debug)]
pub struct Regions {
    nodes: Vec<Node>,
    k: usize,
}

#[derive(Clone, Debug)]
enum Node {
    Split {
        axis: usize,
        at: f64,
        lo: usize,
        hi: usize,
    },
    Leaf(usize),
}

impl Regions {
    /// `pts` の点の数が等しくなるように `depth` 回二等分する。
    /// 分ける向きは、そのときの点の広がりが大きい方の軸にする。
    pub fn build(pts: &[(f64, f64)], depth: u32) -> Self {
        let mut r = Regions {
            nodes: Vec::new(),
            k: 0,
        };
        let mut v: Vec<(f64, f64)> = pts.to_vec();
        r.split(&mut v, depth);
        r
    }

    fn split(&mut self, v: &mut [(f64, f64)], depth: u32) -> usize {
        let id = self.nodes.len();
        if depth == 0 || v.len() < 2 {
            self.nodes.push(Node::Leaf(self.k));
            self.k += 1;
            return id;
        }
        let spread = |f: fn(&(f64, f64)) -> f64| {
            let (lo, hi) = v
                .iter()
                .map(f)
                .fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
            hi - lo
        };
        let axis = if spread(|p| p.0) >= spread(|p| p.1) {
            0
        } else {
            1
        };
        let key = |p: &(f64, f64)| if axis == 0 { p.0 } else { p.1 };
        v.sort_by(|a, b| key(a).total_cmp(&key(b)));
        let mid = v.len() / 2;
        let at = key(&v[mid]);
        self.nodes.push(Node::Leaf(usize::MAX)); // 子を作ってから置き換える
        let (left, right) = v.split_at_mut(mid);
        let lo = self.split(left, depth - 1);
        let hi = self.split(right, depth - 1);
        self.nodes[id] = Node::Split { axis, at, lo, hi };
        id
    }

    pub fn len(&self) -> usize {
        self.k
    }

    pub fn is_empty(&self) -> bool {
        self.k == 0
    }

    /// 点が入る領域の番号。
    pub fn assign(&self, p: (f64, f64)) -> usize {
        let mut i = 0;
        loop {
            match self.nodes[i] {
                Node::Leaf(k) => return k,
                Node::Split { axis, at, lo, hi } => {
                    let x = if axis == 0 { p.0 } else { p.1 };
                    i = if x < at { lo } else { hi };
                }
            }
        }
    }
}

/// 領域の組ごとの対の数。添字は [ビン][a][b]。
#[derive(Clone, Debug)]
pub struct RegionCounts {
    nb: usize,
    k: usize,
    c: Vec<u64>,
}

impl RegionCounts {
    fn new(nb: usize, k: usize) -> Self {
        Self {
            nb,
            k,
            c: vec![0; nb * k * k],
        }
    }

    fn add(&mut self, bin: usize, a: usize, b: usize) {
        self.c[(bin * self.k + a) * self.k + b] += 1;
    }

    fn get(&self, bin: usize, a: usize, b: usize) -> u64 {
        self.c[(bin * self.k + a) * self.k + b]
    }

    /// ビンごとの合計。`omit` があれば、その領域の点を含む組を除く。
    pub fn total(&self, omit: Option<usize>) -> Vec<u64> {
        (0..self.nb)
            .map(|bin| {
                let mut s = 0;
                for a in 0..self.k {
                    for b in 0..self.k {
                        if Some(a) != omit && Some(b) != omit {
                            s += self.get(bin, a, b);
                        }
                    }
                }
                s
            })
            .collect()
    }
}

/// 同じカタログの中の対（i < j）を、領域の組ごとに数える。
pub fn count_auto_by_region<P>(
    pts: &[P],
    reg: &[usize],
    k: usize,
    bins: &Bins,
    dist: impl Fn(&P, &P) -> f64,
) -> RegionCounts {
    let mut rc = RegionCounts::new(bins.len(), k);
    for i in 0..pts.len() {
        for j in i + 1..pts.len() {
            if let Some(bin) = bins.index(dist(&pts[i], &pts[j])) {
                rc.add(bin, reg[i], reg[j]);
            }
        }
    }
    rc
}

/// 2 つのカタログの間の対を、領域の組ごとに数える。
pub fn count_cross_by_region<P>(
    a: &[P],
    ra: &[usize],
    b: &[P],
    rb: &[usize],
    k: usize,
    bins: &Bins,
    dist: impl Fn(&P, &P) -> f64,
) -> RegionCounts {
    let mut rc = RegionCounts::new(bins.len(), k);
    for (i, p) in a.iter().enumerate() {
        for (j, q) in b.iter().enumerate() {
            if let Some(bin) = bins.index(dist(p, q)) {
                rc.add(bin, ra[i], rb[j]);
            }
        }
    }
    rc
}

/// データとランダムカタログの、領域ごとの対の数と点の数。
#[derive(Clone, Debug)]
pub struct JkCounts {
    pub k: usize,
    pub dd: RegionCounts,
    pub dr: RegionCounts,
    pub rr: RegionCounts,
    pub nd: Vec<usize>,
    pub nr: Vec<usize>,
}

impl JkCounts {
    pub fn compute<P>(
        data: &[P],
        rd: &[usize],
        randoms: &[P],
        rr_reg: &[usize],
        k: usize,
        bins: &Bins,
        dist: impl Fn(&P, &P) -> f64,
    ) -> Self {
        let count = |r: &[usize]| {
            let mut n = vec![0; k];
            for x in r {
                n[*x] += 1;
            }
            n
        };
        Self {
            k,
            dd: count_auto_by_region(data, rd, k, bins, &dist),
            dr: count_cross_by_region(data, rd, randoms, rr_reg, k, bins, &dist),
            rr: count_auto_by_region(randoms, rr_reg, k, bins, &dist),
            nd: count(rd),
            nr: count(rr_reg),
        }
    }

    /// 全体（`None`）か、領域 `omit` を除いた対の数。
    pub fn pair_counts(&self, omit: Option<usize>) -> PairCounts {
        let n = |v: &[usize]| {
            v.iter()
                .enumerate()
                .filter(|(i, _)| Some(*i) != omit)
                .map(|(_, x)| x)
                .sum()
        };
        PairCounts {
            n_d: n(&self.nd),
            n_r: n(&self.nr),
            dd: self.dd.total(omit),
            dr: self.dr.total(omit),
            rr: self.rr.total(omit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::geo::{LatLon, Unit, Xy, chord2, euclid, great_circle_km, km_to_chord2};
    use crate::util::rng::SplitMix64;

    fn uniform(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
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
    fn regions_split_evenly_and_assign_consistently() {
        let mut rng = SplitMix64::new(5);
        let pts: Vec<(f64, f64)> = (0..4096)
            .map(|_| (rng.next_f64(), rng.next_f64().powi(3)))
            .collect();
        let reg = Regions::build(&pts, 4);
        assert_eq!(reg.len(), 16);
        let mut n = [0; 16];
        for p in &pts {
            n[reg.assign(*p)] += 1;
        }
        assert!(n.iter().all(|c| *c == 256), "{n:?}");
    }

    #[test]
    fn leave_out_counts_match_recount() {
        // 領域 k を除いた対の数が、その領域の点を除いて数え直した値と一致する。
        let mut rng = SplitMix64::new(9);
        let data = uniform(300, &mut rng);
        let rand = uniform(600, &mut rng);
        let reg = Regions::build(&rand.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(), 2);
        let rd: Vec<usize> = data.iter().map(|p| reg.assign((p.x, p.y))).collect();
        let rr: Vec<usize> = rand.iter().map(|p| reg.assign((p.x, p.y))).collect();
        let bins = Bins::linear(0.02, 0.3, 7);
        let jk = JkCounts::compute(&data, &rd, &rand, &rr, reg.len(), &bins, euclid);
        for omit in 0..reg.len() {
            let d: Vec<Xy> = data
                .iter()
                .zip(&rd)
                .filter(|(_, r)| **r != omit)
                .map(|(p, _)| *p)
                .collect();
            let r: Vec<Xy> = rand
                .iter()
                .zip(&rr)
                .filter(|(_, x)| **x != omit)
                .map(|(p, _)| *p)
                .collect();
            let direct = PairCounts::compute(&d, &r, &bins, euclid);
            let fast = jk.pair_counts(Some(omit));
            assert_eq!((fast.n_d, fast.n_r), (direct.n_d, direct.n_r));
            assert_eq!(fast.dd, direct.dd);
            assert_eq!(fast.dr, direct.dr);
            assert_eq!(fast.rr, direct.rr);
        }
    }
}

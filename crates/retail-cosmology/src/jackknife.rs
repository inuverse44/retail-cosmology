//! jackknife で二点相関関数の誤差を見積もる。
//!
//! 領域を K 個に分け、対の数を「どの領域の点とどの領域の点の組か」ごとに数えておく。
//! 領域 k を除いた対の数は、合計から領域 k を含む組を引けば求まるので、数え直さずに済む。

use crate::{Bins, PairCounts};

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
pub fn count_auto<P>(
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
pub fn count_cross<P>(
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
            dd: count_auto(data, rd, k, bins, &dist),
            dr: count_cross(data, rd, randoms, rr_reg, k, bins, &dist),
            rr: count_auto(randoms, rr_reg, k, bins, &dist),
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

/// jackknife の結果：全体での値、ビンごとの標準偏差、共分散行列（行ごと）。
#[derive(Clone, Debug)]
pub struct Estimate {
    pub value: Vec<f64>,
    pub sigma: Vec<f64>,
    pub cov: Vec<Vec<f64>>,
}

/// 領域を一つずつ除いて `f(omit)` を計算し、jackknife の共分散を求める。
/// C_ij = (K-1)/K Σ_k (f_k,i - f̄_i)(f_k,j - f̄_j)
pub fn jackknife(k: usize, f: impl Fn(Option<usize>) -> Vec<f64>) -> Estimate {
    let value = f(None);
    let subs: Vec<Vec<f64>> = (0..k).map(|i| f(Some(i))).collect();
    let nb = value.len();
    let mean: Vec<f64> = (0..nb)
        .map(|b| subs.iter().map(|s| s[b]).sum::<f64>() / k as f64)
        .collect();
    let fac = (k as f64 - 1.0) / k as f64;
    let cov: Vec<Vec<f64>> = (0..nb)
        .map(|i| {
            (0..nb)
                .map(|j| {
                    fac * subs
                        .iter()
                        .map(|s| (s[i] - mean[i]) * (s[j] - mean[j]))
                        .sum::<f64>()
                })
                .collect()
        })
        .collect();
    let sigma = (0..nb).map(|i| cov[i][i].sqrt()).collect();
    Estimate { value, sigma, cov }
}

/// 定数を共分散つきで当てはめた値（GLS の平均）と、その標準偏差。
/// m = 1ᵀC⁻¹x / 1ᵀC⁻¹1、σ_m² = 1 / 1ᵀC⁻¹1。`hartlap` は C⁻¹ にかける補正因子。
pub fn gls_mean(x: &[f64], cov: &[Vec<f64>], hartlap: f64) -> Option<(f64, f64)> {
    let ones = vec![1.0; x.len()];
    let cinv_x = solve(cov, x)?;
    let cinv_1 = solve(cov, &ones)?;
    let a: f64 = cinv_1.iter().sum::<f64>() * hartlap;
    let b: f64 = cinv_x.iter().sum::<f64>() * hartlap;
    Some((b / a, (1.0 / a).sqrt()))
}

/// C y = b を Cholesky 分解で解く。C が正定値でなければ None。
pub fn solve(cov: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let l = cholesky(cov)?;
    let n = b.len();
    let mut z = vec![0.0; n];
    for i in 0..n {
        z[i] = (b[i] - (0..i).map(|m| l[i][m] * z[m]).sum::<f64>()) / l[i][i];
    }
    let mut y = vec![0.0; n];
    for i in (0..n).rev() {
        y[i] = (z[i] - (i + 1..n).map(|m| l[m][i] * y[m]).sum::<f64>()) / l[i][i];
    }
    Some(y)
}

fn cholesky(cov: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = cov.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s: f64 = cov[i][j] - (0..j).map(|m| l[i][m] * l[j][m]).sum::<f64>();
            if i == j {
                if s <= 0.0 {
                    return None;
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    Some(l)
}

/// χ² = x^T C^{-1} x を、Cholesky 分解で解いて求める。C が正定値でなければ None。
pub fn chi2(x: &[f64], cov: &[Vec<f64>]) -> Option<f64> {
    let n = x.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s: f64 = cov[i][j] - (0..j).map(|m| l[i][m] * l[j][m]).sum::<f64>();
            if i == j {
                if s <= 0.0 {
                    return None;
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    // L y = x を解くと χ² = |y|²
    let mut y = vec![0.0; n];
    for i in 0..n {
        y[i] = (x[i] - (0..i).map(|m| l[i][m] * y[m]).sum::<f64>()) / l[i][i];
    }
    Some(y.iter().map(|v| v * v).sum())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::{Xy, euclid};

    fn uniform(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
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

    #[test]
    fn gls_mean_of_correlated_pair() {
        // 独立なら逆分散の重みつき平均になる
        let cov = vec![vec![1.0, 0.0], vec![0.0, 4.0]];
        let (m, s) = gls_mean(&[1.0, 2.0], &cov, 1.0).unwrap();
        assert!((m - 1.2).abs() < 1e-12 && (s - (0.8f64).sqrt()).abs() < 1e-12);
        // 完全に近い相関なら、平均の誤差はほとんど減らない
        let cov = vec![vec![1.0, 0.999], vec![0.999, 1.0]];
        let (_, s) = gls_mean(&[1.0, 1.0], &cov, 1.0).unwrap();
        assert!(s > 0.99);
    }

    #[test]
    fn chi2_of_diagonal_covariance() {
        let cov = vec![vec![4.0, 0.0], vec![0.0, 1.0]];
        assert!((chi2(&[2.0, 3.0], &cov).unwrap() - 10.0).abs() < 1e-12);
        assert!(chi2(&[1.0], &[vec![-1.0]]).is_none());
    }

    #[test]
    fn jackknife_sigma_matches_scatter_of_realizations() {
        // 相関のない点を何度も撒き直したときの Landy–Szalay のばらつきと、
        // 一回分のデータから jackknife で見積もった標準偏差を比べる。
        let bins = Bins::linear(0.02, 0.2, 3);
        let (nd, nr, k_depth, real) = (400, 800, 4, 60);
        let mut rng = SplitMix64::new(11);
        let mut ls_all: Vec<Vec<f64>> = Vec::new();
        let mut sig_all: Vec<Vec<f64>> = Vec::new();
        for _ in 0..real {
            let data = uniform(nd, &mut rng);
            let rand = uniform(nr, &mut rng);
            let reg = Regions::build(
                &rand.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(),
                k_depth,
            );
            let rd: Vec<usize> = data.iter().map(|p| reg.assign((p.x, p.y))).collect();
            let rr: Vec<usize> = rand.iter().map(|p| reg.assign((p.x, p.y))).collect();
            let jk = JkCounts::compute(&data, &rd, &rand, &rr, reg.len(), &bins, euclid);
            let est = jackknife(reg.len(), |o| jk.pair_counts(o).landy_szalay());
            ls_all.push(est.value);
            sig_all.push(est.sigma);
        }
        for b in 0..bins.len() {
            let m = ls_all.iter().map(|v| v[b]).sum::<f64>() / real as f64;
            let sd = (ls_all.iter().map(|v| (v[b] - m).powi(2)).sum::<f64>() / (real as f64 - 1.0))
                .sqrt();
            let jk = sig_all.iter().map(|v| v[b]).sum::<f64>() / real as f64;
            // 60 回のばらつき自体に約 9% の揺らぎがあるので、比が 0.75〜1.33 なら合っているとみなす
            let ratio = jk / sd;
            eprintln!(
                "bin {b}: jackknife {jk:.4}、撒き直したときのばらつき {sd:.4}、比 {ratio:.2}"
            );
            assert!(
                (0.75..1.33).contains(&ratio),
                "bin {b}: jackknife {jk:.4} vs 実際のばらつき {sd:.4}"
            );
        }
    }
}

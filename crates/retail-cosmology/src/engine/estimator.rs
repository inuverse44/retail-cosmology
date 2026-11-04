//! Engine：推定量と誤差。対の数から二点相関関数の推定量を求め、jackknife で誤差をつけ、
//! 共分散を使って χ² と、ビンをまとめた値（GLS）を求める。

use crate::contract::{CrossPairCounts, PairCounts};

impl PairCounts {
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

    /// dr/rr - 1。natural と Landy-Szalay の差は 2(dr/rr - 1) になる。
    pub fn dr_over_rr_minus_1(&self) -> Vec<f64> {
        let (nd, nr) = (self.n_d as f64, self.n_r as f64);
        (0..self.dd.len())
            .map(|b| {
                (self.dr[b] as f64 / (nd * nr)) / (self.rr[b] as f64 / (nr * (nr - 1.0) / 2.0))
                    - 1.0
            })
            .collect()
    }
}

impl CrossPairCounts {
    /// 相互相関の Landy–Szalay：(d1d2 - d1r2 - d2r1 + r1r2) / r1r2。
    /// それぞれ全対数（n_d1 n_d2, n_d1 n_r2, n_d2 n_r1, n_r1r2）で割った値。
    /// R1 と R2 が同じカタログなら (d1d2 - d1r - d2r + rr) / rr になる。
    pub fn landy_szalay(&self) -> Vec<f64> {
        let (n1, n2) = (self.n_d1 as f64, self.n_d2 as f64);
        let (m1, m2) = (self.n_r1 as f64, self.n_r2 as f64);
        (0..self.d1d2.len())
            .map(|b| {
                let d1d2 = self.d1d2[b] as f64 / (n1 * n2);
                let d1r2 = self.d1r2[b] as f64 / (n1 * m2);
                let d2r1 = self.d2r1[b] as f64 / (n2 * m1);
                let r1r2 = self.r1r2[b] as f64 / self.n_r1r2;
                (d1d2 - d1r2 - d2r1 + r1r2) / r1r2
            })
            .collect()
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

/// χ² 分布の上側確率 P(χ² > x)（自由度 dof は任意）。正則化された上側不完全ガンマ関数 Q(dof/2, x/2)。
/// 級数（x < a + 1）と連分数（それ以外）で求める（Numerical Recipes の gammq と同じ方法）。
pub fn chi2_sf(x: f64, dof: usize) -> f64 {
    let a = dof as f64 / 2.0;
    let x = x / 2.0;
    if x <= 0.0 {
        return 1.0;
    }
    let gln = ln_gamma(a);
    if x < a + 1.0 {
        let (mut ap, mut sum) = (a, 1.0 / a);
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        1.0 - sum * (-x + a * x.ln() - gln).exp()
    } else {
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..1000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        (-x + a * x.ln() - gln).exp() * h
    }
}

/// ln Γ(a)（Lanczos 近似、a > 0）
fn ln_gamma(a: f64) -> f64 {
    const C: [f64; 6] = [
        76.18009172947146,
        -86.50532032941677,
        24.01409824083091,
        -1.231739572450155,
        0.1208650973866179e-2,
        -0.5395239384953e-5,
    ];
    let mut y = a;
    let tmp = a + 5.5;
    let tmp = tmp - (a + 0.5) * tmp.ln();
    let mut ser = 1.000000000190015;
    for c in C {
        y += 1.0;
        ser += c / y;
    }
    -tmp + (2.5066282746310005 * ser / a).ln()
}

/// 自由度が偶数のときの χ² 分布の上側確率 P(χ² > x) = e^{-x/2} Σ_{i<k/2} (x/2)^i / i!
pub fn chi2_sf_even(x: f64, dof: usize) -> f64 {
    assert!(dof.is_multiple_of(2));
    let h = x / 2.0;
    let mut term = 1.0;
    let mut s = 1.0;
    for i in 1..dof / 2 {
        term *= h / i as f64;
        s += term;
    }
    (-h).exp() * s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::pair_count::{Bins, JkCounts, Regions};
    use crate::util::geo::{Xy, euclid};
    use crate::util::rng::SplitMix64;

    fn uniform_square(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
    }

    fn uniform(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
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

    #[test]
    fn chi2_sf_agrees_with_tables_and_even_formula() {
        // χ² 分布表の 95% 点：自由度 1 は 3.841、5 は 11.070、14 は 23.685
        for (x, k) in [(3.841, 1), (11.070, 5), (23.685, 14)] {
            assert!(
                (chi2_sf(x, k) - 0.05).abs() < 2e-4,
                "dof {k}: {}",
                chi2_sf(x, k)
            );
        }
        // 自由度が偶数なら、閉じた式と一致する
        for (x, k) in [(1.0, 2), (10.4, 14), (72.9, 14), (3.0, 4)] {
            assert!((chi2_sf(x, k) - chi2_sf_even(x, k)).abs() < 1e-10);
        }
    }

    #[test]
    fn chi2_survival_matches_known_values() {
        // 自由度 2 では P(χ² > x) = e^{-x/2}
        assert!((chi2_sf_even(2.0, 2) - (-1.0f64).exp()).abs() < 1e-12);
        // 自由度 14 の 95% 点は 23.685（χ² 分布表）
        assert!((chi2_sf_even(23.685, 14) - 0.05).abs() < 1e-4);
    }
}

//! DD の分散：幾何の因子 G_p、G_t と、店の数を固定した分散とポアソン分布で揺らした分散
//! （ノートの付録「DD の分散」）。

use crate::contract::Bins;

/// ビンごとの幾何の因子。
///
/// - `gp[k]`：一つの組がビン k に入る確率 E[θ12]。
/// - `gt[k]`：一点を共有する二つの組がともにビン k に入る確率 E[θ12 θ13]。
#[derive(Clone, Debug, PartialEq)]
pub struct GeometricFactors {
    pub gp: Vec<f64>,
    pub gt: Vec<f64>,
}

/// 同じ密度 p から独立に撒いた点（ランダムカタログ）から、G_p と G_t を偏りなく求める。
///
/// 点 i から見てビン k に入る点の数を c_i とすると、
/// G_p = Σ_i c_i / [N(N-1)]、G_t = Σ_i c_i (c_i - 1) / [N(N-1)(N-2)] である。
/// どちらも、すべての異なる点の組（三つ組）について平均をとった値になる。
pub fn geometric_factors<P>(
    points: &[P],
    bins: &Bins,
    dist: impl Fn(&P, &P) -> f64,
) -> GeometricFactors {
    let n = points.len();
    assert!(n >= 3, "G_t には 3 点以上が要る");
    let nb = bins.len();
    // c[i * nb + k]：点 i から見てビン k に入る点の数
    let mut c = vec![0u64; n * nb];
    for i in 0..n {
        for j in i + 1..n {
            if let Some(k) = bins.index(dist(&points[i], &points[j])) {
                c[i * nb + k] += 1;
                c[j * nb + k] += 1;
            }
        }
    }
    let nf = n as f64;
    let mut gp = vec![0.0; nb];
    let mut gt = vec![0.0; nb];
    for i in 0..n {
        for k in 0..nb {
            let ci = c[i * nb + k] as f64;
            gp[k] += ci;
            gt[k] += ci * (ci - 1.0);
        }
    }
    for k in 0..nb {
        gp[k] /= nf * (nf - 1.0);
        gt[k] /= nf * (nf - 1.0) * (nf - 2.0);
    }
    GeometricFactors { gp, gt }
}

/// 店の数 n を固定した DD の分散（ノートの式 (14)）。
pub fn var_dd_fixed(n: f64, gp: f64, gt: f64) -> f64 {
    n * (n - 1.0) * (n - 2.0) * (gt - gp * gp) + 0.5 * n * (n - 1.0) * gp * (1.0 - gp)
}

/// 店の数が平均 `mean` のポアソン分布に従うときの DD の全分散（ノートの式 (20)）。
pub fn var_dd_poisson(mean: f64, gp: f64, gt: f64) -> f64 {
    mean.powi(3) * gt + 0.5 * mean * mean * gp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::pair_count::count_auto;
    use crate::util::geo::{Xy, euclid};
    use crate::util::rng::SplitMix64;
    use std::f64::consts::PI;

    const R1: f64 = 0.05;
    const R2: f64 = 0.10;

    fn uniform(n: usize, rng: &mut SplitMix64) -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
    }

    /// 平均 `mean` のポアソン乱数（Knuth の方法。mean が数百までなら足りる）。
    fn poisson(mean: f64, rng: &mut SplitMix64) -> usize {
        let limit = (-mean).exp();
        let mut k = 0;
        let mut prod = rng.next_f64();
        while prod > limit {
            k += 1;
            prod *= rng.next_f64();
        }
        k
    }

    /// 単位正方形に一様な密度での G_p と G_t を、乱数を使わずに数値積分で求める。
    ///
    /// 点 x から角度 φ の向きに、正方形の端までの距離を t(φ) とすると、
    /// f(x) = ∫ dφ ∫_{R1}^{min(t, R2)} r dr である（正方形は凸なので、向きごとに区間は一つ）。
    /// G_p = ∫ f d²x、G_t = ∫ f² d²x を、x の格子の中点で和をとって求める。
    fn square_factors(grid: usize, angles: usize) -> (f64, f64) {
        let dirs: Vec<(f64, f64)> = (0..angles)
            .map(|a| {
                let phi = 2.0 * PI * (a as f64 + 0.5) / angles as f64;
                (phi.cos(), phi.sin())
            })
            .collect();
        let dphi = 2.0 * PI / angles as f64;
        let (mut gp, mut gt) = (0.0, 0.0);
        for ix in 0..grid {
            let x = (ix as f64 + 0.5) / grid as f64;
            for iy in 0..grid {
                let y = (iy as f64 + 0.5) / grid as f64;
                let mut f = 0.0;
                for &(c, s) in &dirs {
                    let tx = if c > 0.0 { (1.0 - x) / c } else { -x / c };
                    let ty = if s > 0.0 { (1.0 - y) / s } else { -y / s };
                    let t = tx.min(ty).min(R2);
                    if t > R1 {
                        f += 0.5 * (t * t - R1 * R1) * dphi;
                    }
                }
                gp += f;
                gt += f * f;
            }
        }
        let cell = 1.0 / (grid * grid) as f64;
        (gp * cell, gt * cell)
    }

    #[test]
    fn geometric_factors_by_hand() {
        // 正方形の 4 頂点：辺 4 組（距離 1）、対角線 2 組（距離 sqrt 2）
        let p = [
            Xy { x: 0.0, y: 0.0 },
            Xy { x: 1.0, y: 0.0 },
            Xy { x: 1.0, y: 1.0 },
            Xy { x: 0.0, y: 1.0 },
        ];
        let b = Bins::new(vec![0.5, 1.2, 2.0]);
        let g = geometric_factors(&p, &b, euclid);
        // 辺のビン：どの点からも 2 点。G_p = 4·2/12、G_t = 4·2·1/24
        // 対角線のビン：どの点からも 1 点。G_p = 4·1/12、G_t = 0
        assert_eq!(g.gp, vec![8.0 / 12.0, 4.0 / 12.0]);
        assert_eq!(g.gt, vec![8.0 / 24.0, 0.0]);
    }

    #[test]
    fn geometric_factors_match_quadrature_on_square() {
        let (gp_q, gt_q) = square_factors(200, 2000);
        // 単位正方形の二点の距離の累積分布 F(r) = πr² − 8r³/3 + r⁴/2（r ≤ 1）
        let cdf = |r: f64| PI * r * r - 8.0 * r.powi(3) / 3.0 + 0.5 * r.powi(4);
        let gp_exact = cdf(R2) - cdf(R1);
        assert!((gp_q / gp_exact - 1.0).abs() < 1e-4, "{gp_q} vs {gp_exact}");
        let mut rng = SplitMix64::new(11);
        let pts = uniform(4000, &mut rng);
        let g = geometric_factors(&pts, &Bins::new(vec![R1, R2]), euclid);
        let (gp, gt) = (g.gp[0], g.gt[0]);
        // 4000 点の U 統計量の揺らぎは G_p で 1% 未満、G_t で 2% 未満の程度
        assert!((gp / gp_q - 1.0).abs() < 0.02, "G_p {gp} vs {gp_q}");
        assert!((gt / gt_q - 1.0).abs() < 0.03, "G_t {gt} vs {gt_q}");
    }

    /// 撒き直した DD の標本分散と、その標準誤差（DD が正規分布に近いとして sqrt(2/(M-1))·分散）。
    fn sample_var(dd: &[f64]) -> (f64, f64) {
        let m = dd.len() as f64;
        let mean = dd.iter().sum::<f64>() / m;
        let var = dd.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (m - 1.0);
        (var, var * (2.0 / (m - 1.0)).sqrt())
    }

    #[test]
    fn var_dd_matches_mocks_with_fixed_and_poisson_counts() {
        const N: usize = 200;
        const M: usize = 4000;
        let (gp, gt) = square_factors(200, 2000);
        let bins = Bins::new(vec![R1, R2]);
        let mut rng = SplitMix64::new(7);
        let mut dd_fixed = Vec::with_capacity(M);
        let mut dd_poisson = Vec::with_capacity(M);
        for _ in 0..M {
            dd_fixed.push(count_auto(&uniform(N, &mut rng), &bins, euclid)[0] as f64);
            let n = poisson(N as f64, &mut rng);
            dd_poisson.push(count_auto(&uniform(n, &mut rng), &bins, euclid)[0] as f64);
        }
        let nf = N as f64;
        let (v_fixed, se_fixed) = sample_var(&dd_fixed);
        let (v_poisson, se_poisson) = sample_var(&dd_poisson);
        let fixed = var_dd_fixed(nf, gp, gt);
        let poisson_total = var_dd_poisson(nf, gp, gt);
        // 境界の項を落とした式（どの点から見ても f が同じとしたとき）
        let no_edge = var_dd_fixed(nf, gp, gp * gp);
        eprintln!(
            "G_p={gp:.6e} G_t={gt:.6e} G_t-G_p^2={:.3e}\n\
             fixed: mocks {v_fixed:.1} ± {se_fixed:.1}, eq.(14) {fixed:.1}, without edge term {no_edge:.1}, eq.(20) {poisson_total:.1}\n\
             poisson: mocks {v_poisson:.1} ± {se_poisson:.1}, eq.(20) {poisson_total:.1}",
            gt - gp * gp
        );
        assert!((v_fixed - fixed).abs() < 4.0 * se_fixed);
        assert!((v_poisson - poisson_total).abs() < 4.0 * se_poisson);
        // 二つの式の取り違えや、境界の項の落としを見分けられること
        assert!((v_fixed - poisson_total).abs() > 10.0 * se_fixed);
        assert!((v_fixed - no_edge).abs() > 4.0 * se_fixed);
    }
}

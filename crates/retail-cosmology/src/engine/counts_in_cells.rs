//! Engine：マスごとの店の数 N と期待数 λ から、ポアソンの揺らぎを差し引いたずれの統計を求める（ADR 011）。
//!
//! N が平均 λ(1 + δ) のポアソン分布で、δ がマスの中の密度の平均のずれなら、
//! E[(N − λ)² − N] = λ² κ₂、E[(N − λ)³ − 3(N − λ)² + 2N] = λ³ κ₃（κ は δ のキュムラント）。
//! 乱数を使わない純粋な関数だけを置く。

/// 一つのカタログの、λ ≥ λ_min のマスについての統計。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellStats {
    /// 使ったマスの数
    pub cells: usize,
    /// 使ったマスの店の数の合計と、期待数の合計
    pub n_sum: u64,
    pub lambda_sum: f64,
    /// κ̂₂ = Σ[(N − λ)² − N] / Σλ²
    pub kappa2: f64,
    /// κ̂₃ = Σ[(N − λ)³ − 3(N − λ)² + 2N] / Σλ³
    pub kappa3: f64,
    /// 空のマスの数と、ポアソン分布での見込み Σ e^{−λ}
    pub empty: usize,
    pub empty_poisson: f64,
    /// N ≥ 2λ のマスの数と、ポアソン分布での見込み Σ P(N ≥ 2λ; λ)
    pub high: usize,
    pub high_poisson: f64,
}

impl CellStats {
    /// 空のマスの比 V = #(N = 0) / Σ e^{−λ}
    pub fn void_ratio(&self) -> f64 {
        self.empty as f64 / self.empty_poisson
    }

    /// 多いマスの比 H = #(N ≥ 2λ) / Σ P(N ≥ 2λ; λ)
    pub fn high_ratio(&self) -> f64 {
        self.high as f64 / self.high_poisson
    }
}

/// 平均 `lambda` のポアソン分布で N ≥ k となる確率。
pub fn poisson_tail(lambda: f64, k: u64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    // P(N < k) = Σ_{n<k} e^{−λ} λⁿ / n!。λ は小さい（マスあたり数個）ので、そのまま足す。
    let mut term = (-lambda).exp();
    let mut below = term;
    for n in 1..k {
        term *= lambda / n as f64;
        below += term;
    }
    (1.0 - below).max(0.0)
}

/// `cells` は（店の数 N、期待数 λ）。λ ≥ `lambda_min` のマスだけを使う。
pub fn cell_stats(cells: &[(u64, f64)], lambda_min: f64) -> CellStats {
    let mut s = CellStats {
        cells: 0,
        n_sum: 0,
        lambda_sum: 0.0,
        kappa2: 0.0,
        kappa3: 0.0,
        empty: 0,
        empty_poisson: 0.0,
        high: 0,
        high_poisson: 0.0,
    };
    let (mut l2, mut l3) = (0.0, 0.0);
    for &(n, lambda) in cells.iter().filter(|c| c.1 >= lambda_min) {
        let nf = n as f64;
        let d = nf - lambda;
        s.cells += 1;
        s.n_sum += n;
        s.lambda_sum += lambda;
        s.kappa2 += d * d - nf;
        s.kappa3 += d * d * d - 3.0 * d * d + 2.0 * nf;
        l2 += lambda * lambda;
        l3 += lambda * lambda * lambda;
        if n == 0 {
            s.empty += 1;
        }
        s.empty_poisson += (-lambda).exp();
        let k = (2.0 * lambda).ceil() as u64;
        if n >= k {
            s.high += 1;
        }
        s.high_poisson += poisson_tail(lambda, k);
    }
    s.kappa2 /= l2;
    s.kappa3 /= l3;
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::rng::SplitMix64;

    /// 平均 λ のポアソン分布の乱数（λ が小さいので逆関数法）。
    fn poisson(lambda: f64, rng: &mut SplitMix64) -> u64 {
        let u = rng.next_f64();
        let mut p = (-lambda).exp();
        let mut c = p;
        let mut n = 0;
        while u > c {
            n += 1;
            p *= lambda / n as f64;
            c += p;
        }
        n
    }

    #[test]
    fn poisson_tail_matches_direct_sums() {
        let l: f64 = 1.7;
        assert!((poisson_tail(l, 0) - 1.0).abs() < 1e-15);
        assert!((poisson_tail(l, 1) - (1.0 - (-l).exp())).abs() < 1e-15);
        let p2 = 1.0 - (-l).exp() * (1.0 + l);
        assert!((poisson_tail(l, 2) - p2).abs() < 1e-15);
    }

    /// 店がポアソン分布（δ = 0）なら、κ̂₂・κ̂₃ は 0、V・H は 1 のまわりに来る。
    #[test]
    fn poisson_cells_give_null_values() {
        let mut rng = SplitMix64::new(11);
        let cells: Vec<(u64, f64)> = (0..200_000)
            .map(|i| {
                let lambda = 0.5 + (i % 7) as f64 * 0.5;
                (poisson(lambda, &mut rng), lambda)
            })
            .collect();
        let s = cell_stats(&cells, 0.0);
        assert_eq!(s.cells, 200_000);
        assert!(s.kappa2.abs() < 0.01, "κ̂₂ = {}", s.kappa2);
        assert!(s.kappa3.abs() < 0.01, "κ̂₃ = {}", s.kappa3);
        assert!((s.void_ratio() - 1.0).abs() < 0.02, "V = {}", s.void_ratio());
        assert!((s.high_ratio() - 1.0).abs() < 0.02, "H = {}", s.high_ratio());
    }

    /// 平均が λ(1 + δ) で、δ が ±a を半々にとる（κ₂ = a²、κ₃ = 0）場合と、
    /// δ が a·(2 か −1 を 1:2 で)（κ₂ = 2a²、κ₃ = 2a³）の場合に、κ̂ が κ を返す。
    #[test]
    fn mixed_poisson_recovers_cumulants() {
        let mut rng = SplitMix64::new(12);
        let lambda = 3.0;
        let a = 0.3;
        let sym: Vec<(u64, f64)> = (0..400_000)
            .map(|i| {
                let d = if i % 2 == 0 { a } else { -a };
                (poisson(lambda * (1.0 + d), &mut rng), lambda)
            })
            .collect();
        let s = cell_stats(&sym, 0.0);
        assert!((s.kappa2 - a * a).abs() < 0.005, "κ̂₂ = {}", s.kappa2);
        assert!(s.kappa3.abs() < 0.01, "κ̂₃ = {}", s.kappa3);

        let skew: Vec<(u64, f64)> = (0..600_000)
            .map(|i| {
                let d = if i % 3 == 0 { 2.0 * a } else { -a };
                (poisson(lambda * (1.0 + d), &mut rng), lambda)
            })
            .collect();
        let s = cell_stats(&skew, 0.0);
        assert!((s.kappa2 - 2.0 * a * a).abs() < 0.005, "κ̂₂ = {}", s.kappa2);
        assert!(
            (s.kappa3 - 2.0 * a * a * a).abs() < 0.01,
            "κ̂₃ = {}",
            s.kappa3
        );
    }

    #[test]
    fn lambda_min_drops_small_cells() {
        let cells = [(0, 0.2), (3, 1.0), (1, 2.0)];
        let s = cell_stats(&cells, 1.0);
        assert_eq!(s.cells, 2);
        assert_eq!(s.n_sum, 4);
        assert_eq!(s.empty, 0);
        // N ≥ 2λ：3 ≥ 2 は入り、1 ≥ 4 は入らない
        assert_eq!(s.high, 1);
    }
}

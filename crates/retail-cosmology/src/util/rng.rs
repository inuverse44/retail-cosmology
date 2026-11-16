//! 再現できる乱数（SplitMix64）。toy model とランダムカタログに使う。

pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// [0, 1) の一様乱数（上位 53 ビット）。
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// 標準正規乱数（Box–Muller 法。一様乱数を二つ使い、一つだけ返す）。
    pub fn next_normal(&mut self) -> f64 {
        // 1 − u は (0, 1] なので対数が発散しない
        let u1 = 1.0 - self.next_f64();
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_has_zero_mean_unit_variance_and_no_skew() {
        let mut r = SplitMix64::new(42);
        let n = 400_000;
        let (mut m1, mut m2, mut m3) = (0.0, 0.0, 0.0);
        for _ in 0..n {
            let x = r.next_normal();
            m1 += x;
            m2 += x * x;
            m3 += x * x * x;
        }
        let n = n as f64;
        // 許容は標準誤差（1/√n ≈ 0.0016、二次は √2/√n、三次は √15/√n）の 5 倍ほど
        assert!((m1 / n).abs() < 0.008, "{}", m1 / n);
        assert!((m2 / n - 1.0).abs() < 0.012, "{}", m2 / n);
        assert!((m3 / n).abs() < 0.035, "{}", m3 / n);
    }
}

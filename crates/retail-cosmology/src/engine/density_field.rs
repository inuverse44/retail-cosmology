//! Engine：密度場の模型の、点ごとの変換と、その二点・三点の見込み（ADR 010）。
//!
//! g は平均 0・分散 1 のガウス場、ρ はその相関係数。正と負の歪みの変換 T(g) = ±(g² − 1)/√2 は
//! 平均 0・分散 1 で、Wick の定理から ⟨T₁T₂⟩ = ρ²、⟨T₁T₂T₃⟩ = ±2√2 ρ₁₂ρ₂₃ρ₃₁ になる（切る前）。

use std::f64::consts::SQRT_2;

use crate::contract::FieldTransform;

/// 単位分散の場の値 g を変換する。
pub fn transform(t: FieldTransform, g: f64) -> f64 {
    match t {
        FieldTransform::Identity => g,
        FieldTransform::Chi => (g * g - 1.0) / SQRT_2,
        FieldTransform::NegChi => (1.0 - g * g) / SQRT_2,
    }
}

/// 幅 `sigma_km` のガウス関数を足した場の、距離 `r_km` での相関係数 ρ。
/// 中心の分布を一様とみなすと、ガウス関数の重なりから ρ = exp(−r² / (4σ²))。
pub fn gaussian_rho(r_km: f64, sigma_km: f64) -> f64 {
    (-r_km * r_km / (4.0 * sigma_km * sigma_km)).exp()
}

/// δ = A T(g) の二点相関関数（切る前）。
pub fn expected_xi(t: FieldTransform, amp: f64, rho: f64) -> f64 {
    match t {
        FieldTransform::Identity => amp * amp * rho,
        FieldTransform::Chi | FieldTransform::NegChi => amp * amp * rho * rho,
    }
}

/// δ = A T(g) の三点相関関数（切る前）。恒等変換は 0。
pub fn expected_zeta(t: FieldTransform, amp: f64, rho12: f64, rho23: f64, rho31: f64) -> f64 {
    let sign = match t {
        FieldTransform::Identity => return 0.0,
        FieldTransform::Chi => 1.0,
        FieldTransform::NegChi => -1.0,
    };
    sign * 2.0 * SQRT_2 * amp.powi(3) * rho12 * rho23 * rho31
}

/// δ = A T(g) の確率密度（g は標準正規）。切る前。
/// 恒等：平均 0・標準偏差 A の正規分布。正の歪み：u = g² = 1 + √2 δ/A が自由度 1 のカイ二乗に従うので、
/// p(δ) = (√2/A) exp(−u/2)/√(2πu)（u > 0）。負の歪みは δ → −δ の鏡映。
pub fn pdf_delta(t: FieldTransform, amp: f64, delta: f64) -> f64 {
    let chi = |d: f64| {
        let u = 1.0 + SQRT_2 * d / amp;
        if u <= 0.0 {
            0.0
        } else {
            (SQRT_2 / amp) * (-u / 2.0).exp() / (2.0 * std::f64::consts::PI * u).sqrt()
        }
    };
    match t {
        FieldTransform::Identity => {
            (-delta * delta / (2.0 * amp * amp)).exp() / (amp * (2.0 * std::f64::consts::PI).sqrt())
        }
        FieldTransform::Chi => chi(delta),
        FieldTransform::NegChi => chi(-delta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_integrates_to_one_with_zero_mean() {
        // 正負の歪みの密度は u = 0 で u^(−1/2) のように発散するので、δ の等間隔の格子では端の確率を落とす。
        // δ = A T(g)（g > 0 の枝、|dδ/dg| = A√2 g）に置き換えて g で積分する。恒等変換は δ で積分する。
        let amp = 0.7;
        let n = 200_000;
        let h = 10.0 / n as f64;
        for t in [FieldTransform::Chi, FieldTransform::NegChi] {
            let (mut m0, mut m1) = (0.0, 0.0);
            for i in 0..n {
                let g = (i as f64 + 0.5) * h;
                let d = amp * transform(t, g);
                let jac = amp * SQRT_2 * g;
                let p = pdf_delta(t, amp, d) * jac;
                m0 += p * h;
                m1 += d * p * h;
            }
            assert!((m0 - 1.0).abs() < 1e-6, "{t:?} {m0}");
            assert!(m1.abs() < 1e-6, "{t:?} {m1}");
        }
        let (mut m0, mut m1) = (0.0, 0.0);
        for i in 0..n {
            let d = -8.0 + (i as f64 + 0.5) * (16.0 / n as f64);
            let p = pdf_delta(FieldTransform::Identity, amp, d);
            m0 += p * 16.0 / n as f64;
            m1 += d * p * 16.0 / n as f64;
        }
        assert!((m0 - 1.0).abs() < 1e-6, "{m0}");
        assert!(m1.abs() < 1e-6, "{m1}");
    }

    #[test]
    fn identity_is_unchanged_and_has_no_zeta() {
        assert_eq!(transform(FieldTransform::Identity, 1.7), 1.7);
        assert_eq!(
            expected_zeta(FieldTransform::Identity, 0.7, 0.5, 0.5, 0.5),
            0.0
        );
    }

    #[test]
    fn skews_flip_sign_with_shared_xi() {
        for g in [-2.0, -0.3, 0.0, 1.1, 2.5] {
            assert_eq!(
                transform(FieldTransform::Chi, g),
                -transform(FieldTransform::NegChi, g)
            );
        }
        let (a, r) = (0.6, 0.4);
        assert_eq!(
            expected_xi(FieldTransform::Chi, a, r),
            expected_xi(FieldTransform::NegChi, a, r)
        );
        let z = expected_zeta(FieldTransform::Chi, a, r, r, r);
        assert!((z - 2.0 * SQRT_2 * a.powi(3) * r.powi(3)).abs() < 1e-15);
        assert_eq!(expected_zeta(FieldTransform::NegChi, a, r, r, r), -z);
    }

    #[test]
    fn transform_has_zero_mean_unit_variance() {
        // 標準正規の Gauss–Hermite 求積（5 点、g⁴ まで厳密）で ⟨T⟩ = 0、⟨T²⟩ = 1。
        let nodes = [
            (0.0, 8.0 / 15.0),
            (1.355_626_179_974_266, 0.222_075_922_005_612_6),
            (-1.355_626_179_974_266, 0.222_075_922_005_612_6),
            (2.856_970_013_872_806, 0.011_257_411_327_720_69),
            (-2.856_970_013_872_806, 0.011_257_411_327_720_69),
        ];
        for t in [FieldTransform::Chi, FieldTransform::NegChi] {
            let m1: f64 = nodes.iter().map(|(x, w)| w * transform(t, *x)).sum();
            let m2: f64 = nodes
                .iter()
                .map(|(x, w)| w * transform(t, *x).powi(2))
                .sum();
            assert!(m1.abs() < 1e-12, "{m1}");
            assert!((m2 - 1.0).abs() < 1e-12, "{m2}");
        }
    }

    #[test]
    fn rho_is_one_at_zero_and_decays() {
        assert_eq!(gaussian_rho(0.0, 10.0), 1.0);
        assert!((gaussian_rho(20.0, 10.0) - (-1.0f64).exp()).abs() < 1e-15);
    }
}

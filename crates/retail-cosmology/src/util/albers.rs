//! 楕円体の Albers 正積円錐図法（Snyder 1987, Map Projections — A Working Manual, pp. 101–102）。
//! ABS の格子人口が使う EPSG:3577（GDA94 / Australian Albers）を、緯度・経度と行き来する。

use crate::util::geo::LatLon;

#[derive(Clone, Copy, Debug)]
pub struct Albers {
    a: f64,
    e: f64,
    e2: f64,
    lon0: f64,
    n: f64,
    c: f64,
    rho0: f64,
}

impl Albers {
    /// 楕円体の長半径 `a` と扁平率の逆数 `inv_f`、中央子午線、二つの標準緯線、原点の緯度 [度]。
    pub fn new(a: f64, inv_f: f64, lon0: f64, lat1: f64, lat2: f64, lat0: f64) -> Self {
        let f = 1.0 / inv_f;
        let e2 = 2.0 * f - f * f;
        let e = e2.sqrt();
        let m = |p: f64| p.cos() / (1.0 - e2 * p.sin().powi(2)).sqrt();
        let (p0, p1, p2) = (lat0.to_radians(), lat1.to_radians(), lat2.to_radians());
        let (q0, q1, q2) = (q(p0, e, e2), q(p1, e, e2), q(p2, e, e2));
        let (m1, m2) = (m(p1), m(p2));
        let n = (m1 * m1 - m2 * m2) / (q2 - q1);
        let c = m1 * m1 + n * q1;
        let rho0 = a * (c - n * q0).sqrt() / n;
        Self {
            a,
            e,
            e2,
            lon0: lon0.to_radians(),
            n,
            c,
            rho0,
        }
    }

    /// EPSG:3577（GRS80、中央子午線 132°E、標準緯線 18°S と 36°S、原点の緯度 0°、偽東距・偽北距 0）。
    pub fn epsg3577() -> Self {
        Self::new(6_378_137.0, 298.257_222_101, 132.0, -18.0, -36.0, 0.0)
    }

    /// 緯度・経度 → 平面座標 (x, y) [m]
    pub fn forward(&self, p: &LatLon) -> (f64, f64) {
        let phi = p.lat.to_radians();
        let rho = self.a * (self.c - self.n * q(phi, self.e, self.e2)).sqrt() / self.n;
        let theta = self.n * (p.lon.to_radians() - self.lon0);
        (rho * theta.sin(), self.rho0 - rho * theta.cos())
    }

    /// 平面座標 (x, y) [m] → 緯度・経度
    pub fn inverse(&self, x: f64, y: f64) -> LatLon {
        let (n, e, e2) = (self.n, self.e, self.e2);
        // n < 0（南半球の標準緯線）のときは ρ と θ の符号を n に合わせる（Snyder 式 14-10, 14-11）
        let s = n.signum();
        let rho = s * x.hypot(self.rho0 - y);
        let theta = (s * x).atan2(s * (self.rho0 - y));
        let qq = (self.c - (rho * n / self.a).powi(2)) / n;
        // 緯度を反復で求める（Snyder 式 3-16）
        let mut phi = (qq / 2.0).asin();
        for _ in 0..20 {
            let sp = phi.sin();
            let w = 1.0 - e2 * sp * sp;
            let dphi = w * w / (2.0 * phi.cos())
                * (qq / (1.0 - e2) - sp / w
                    + 1.0 / (2.0 * e) * ((1.0 - e * sp) / (1.0 + e * sp)).ln());
            phi += dphi;
            if dphi.abs() < 1e-13 {
                break;
            }
        }
        LatLon {
            lat: phi.to_degrees(),
            lon: (self.lon0 + theta / n).to_degrees(),
        }
    }
}

/// Snyder 式 3-12
fn q(phi: f64, e: f64, e2: f64) -> f64 {
    let s = phi.sin();
    (1.0 - e2) * (s / (1.0 - e2 * s * s) - 1.0 / (2.0 * e) * ((1.0 - e * s) / (1.0 + e * s)).ln())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// pyproj 3.8.0（PROJ 9.8.1）で EPSG:4283 → EPSG:3577 に変換した値。
    const CASES: [(f64, f64, f64, f64); 4] = [
        (151.2093, -33.8688, 1760981.5286, -3828749.3898), // Sydney
        (115.8605, -31.9505, -1508207.7715, -3577187.3367), // Perth
        (130.8456, -12.4634, -127745.1888, -1315170.0758), // Darwin
        (147.3272, -42.8821, 1288433.0251, -4764346.3966), // Hobart
    ];

    #[test]
    fn forward_matches_proj() {
        let a = Albers::epsg3577();
        for (lon, lat, x, y) in CASES {
            let (fx, fy) = a.forward(&LatLon { lat, lon });
            assert!(
                (fx - x).abs() < 1e-3 && (fy - y).abs() < 1e-3,
                "({lon},{lat}): ({fx},{fy}) vs ({x},{y})"
            );
        }
    }

    #[test]
    fn inverse_matches_proj() {
        let a = Albers::epsg3577();
        for (lon, lat, x, y) in CASES {
            let p = a.inverse(x, y);
            // 1e-8 度は約 1 mm
            assert!(
                (p.lat - lat).abs() < 1e-8 && (p.lon - lon).abs() < 1e-8,
                "{p:?} vs ({lat},{lon})"
            );
        }
    }
}

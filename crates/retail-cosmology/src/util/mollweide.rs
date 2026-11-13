//! 球面の Mollweide 正積図法（Snyder 1987, Map Projections — A Working Manual, pp. 249–252）。
//! JRC の GHS-POP が使う ESRI:54009（World Mollweide）を、緯度・経度と行き来する。
//! ESRI:54009 は楕円体ではなく、半径を WGS84 の長半径 6,378,137 m とした球で計算する（PROJ の moll も同じ）。

use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

use crate::util::geo::LatLon;

#[derive(Clone, Copy, Debug)]
pub struct Mollweide {
    r: f64,
    lon0: f64,
}

impl Mollweide {
    /// ESRI:54009（球の半径 6,378,137 m、中央子午線 0°、偽東距・偽北距 0）。
    pub fn esri54009() -> Self {
        Self {
            r: 6_378_137.0,
            lon0: 0.0,
        }
    }

    /// 緯度・経度 → 平面座標 (x, y) [m]
    pub fn forward(&self, p: &LatLon) -> (f64, f64) {
        let phi = p.lat.to_radians();
        // 補助角 θ：2θ + sin 2θ = π sin φ を Newton 法で解く（Snyder 式 31-4）。
        // 極の近くでは収束が遅いので、そこは θ = ±π/2 とする。
        let theta = if (FRAC_PI_2 - phi.abs()) < 1e-10 {
            FRAC_PI_2.copysign(phi)
        } else {
            let target = PI * phi.sin();
            let mut t = phi;
            for _ in 0..50 {
                let d = -(2.0 * t + (2.0 * t).sin() - target) / (2.0 + 2.0 * (2.0 * t).cos());
                t += d;
                if d.abs() < 1e-15 {
                    break;
                }
            }
            t
        };
        let lam = p.lon.to_radians() - self.lon0;
        (
            2.0 * SQRT_2 / PI * self.r * lam * theta.cos(),
            SQRT_2 * self.r * theta.sin(),
        )
    }

    /// 平面座標 (x, y) [m] → 緯度・経度（Snyder 式 31-6〜31-8）
    pub fn inverse(&self, x: f64, y: f64) -> LatLon {
        let theta = (y / (SQRT_2 * self.r)).clamp(-1.0, 1.0).asin();
        let phi = ((2.0 * theta + (2.0 * theta).sin()) / PI)
            .clamp(-1.0, 1.0)
            .asin();
        let lam = PI * x / (2.0 * SQRT_2 * self.r * theta.cos());
        LatLon {
            lat: phi.to_degrees(),
            lon: (self.lon0 + lam).to_degrees(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let m = Mollweide::esri54009();
        for (lat, lon) in [
            (40.7128, -74.006),   // New York
            (34.0522, -118.2437), // Los Angeles
            (36.3729, -94.2088),  // Bentonville
            (25.7617, -80.1918),  // Miami
            (-33.8688, 151.2093), // Sydney
            (0.0, 0.0),
        ] {
            let (x, y) = m.forward(&LatLon { lat, lon });
            let p = m.inverse(x, y);
            assert!(
                (p.lat - lat).abs() < 1e-9 && (p.lon - lon).abs() < 1e-9,
                "{p:?} vs ({lat},{lon})"
            );
        }
    }

    #[test]
    fn edges_of_the_map() {
        // 赤道の経度 180° は x = 2√2 R、北極は y = √2 R（Snyder 式 31-1, 31-2）
        let m = Mollweide::esri54009();
        let r = 6_378_137.0;
        let (x, y) = m.forward(&LatLon {
            lat: 0.0,
            lon: 180.0,
        });
        assert!((x - 2.0 * SQRT_2 * r).abs() < 1e-6 && y.abs() < 1e-6);
        let (x, y) = m.forward(&LatLon {
            lat: 90.0,
            lon: 0.0,
        });
        assert!(x.abs() < 1e-6 && (y - SQRT_2 * r).abs() < 1e-6);
    }

    #[test]
    fn equal_area() {
        // 正積図法なので、緯度 1°・経度 1° の四角形の平面での面積は、球の上の面積に等しい
        let m = Mollweide::esri54009();
        let r: f64 = 6_378_137.0;
        let (lat, lon) = (40.0f64, -100.0f64);
        let c = [
            (lat, lon),
            (lat, lon + 1.0),
            (lat + 1.0, lon + 1.0),
            (lat + 1.0, lon),
        ]
        .map(|(a, b)| m.forward(&LatLon { lat: a, lon: b }));
        // 四角形の辺は直線ではない（緯線は直線、経線は曲線）が、1° なら差は小さい
        let mut area = 0.0;
        for i in 0..4 {
            let (p, q) = (c[i], c[(i + 1) % 4]);
            area += p.0 * q.1 - q.0 * p.1;
        }
        let area = area.abs() / 2.0;
        let sphere =
            r * r * 1f64.to_radians() * ((lat + 1.0).to_radians().sin() - lat.to_radians().sin());
        assert!((area / sphere - 1.0).abs() < 1e-4, "{area} vs {sphere}");
    }
}

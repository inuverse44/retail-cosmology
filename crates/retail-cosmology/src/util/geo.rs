//! 点と距離：緯度・経度、平面の点、大円距離、弦の長さ。

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

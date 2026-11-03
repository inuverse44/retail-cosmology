//! Engine：店の数を比べるための計算。最も近い店までの距離と、分位点。

use crate::util::geo::{LatLon, great_circle_km};

/// サイトの店と OSM の店を同じ店とみなす距離 [km]
pub const MATCH_KM: f64 = 0.3;

/// `p` から `set` の中で最も近い点までの大円距離 [km]。`set` が空なら f64::MAX。
pub fn nearest_km(p: &LatLon, set: &[LatLon]) -> f64 {
    set.iter()
        .map(|q| great_circle_km(p, q))
        .fold(f64::MAX, f64::min)
}

/// 分位点（並べ替えて、最も近い順位の値をとる）
pub fn quantile(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q).round() as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_and_quantile() {
        let a = LatLon { lat: 0.0, lon: 0.0 };
        let set = [LatLon { lat: 0.0, lon: 1.0 }, LatLon { lat: 0.0, lon: 0.5 }];
        assert!((nearest_km(&a, &set) - great_circle_km(&a, &set[1])).abs() < 1e-12);
        assert_eq!(nearest_km(&a, &[]), f64::MAX);
        let mut v = vec![3.0, 1.0, 2.0, 5.0, 4.0];
        assert_eq!(quantile(&mut v, 0.5), 3.0);
        assert_eq!(quantile(&mut v, 0.9), 5.0);
    }
}

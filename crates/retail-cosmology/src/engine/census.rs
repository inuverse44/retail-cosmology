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

/// カタログ `pts` の点ごとに、自分以外で最も近い点までの大円距離 [km]（総当たり）。
/// 点が一つなら f64::MAX。同じ位置に二つあれば 0。
pub fn nearest_other_km(pts: &[LatLon]) -> Vec<f64> {
    let mut d = vec![f64::MAX; pts.len()];
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            let r = great_circle_km(&pts[i], &pts[j]);
            d[i] = d[i].min(r);
            d[j] = d[j].min(r);
        }
    }
    d
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

    #[test]
    fn nearest_other_excludes_self() {
        let pts = [
            LatLon { lat: 0.0, lon: 0.0 },
            LatLon { lat: 0.0, lon: 0.1 },
            LatLon { lat: 0.0, lon: 0.5 },
        ];
        let d = nearest_other_km(&pts);
        for (i, p) in pts.iter().enumerate() {
            let others: Vec<LatLon> = pts
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| *q)
                .collect();
            assert_eq!(d[i], nearest_km(p, &others));
        }
        assert_eq!(nearest_other_km(&pts[..1]), vec![f64::MAX]);
    }
}

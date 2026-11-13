//! 格子の形：格子の番号と緯度・経度を行き来する。
//!
//! ABS の格子人口は EPSG:3577（Albers 図法）の 1 km の正方形、JRC の GHS-POP は ESRI:54009
//! （Mollweide 図法）の 1 km の正方形、e-Stat の 3 次メッシュは緯度 30 秒・経度 45 秒の長方形。
//! どれも「格子の中の点」「点が入る格子」「jackknife の領域を作る平面座標」を同じ形で返す。

use std::collections::HashMap;

use crate::util::albers::Albers;
use crate::util::geo::LatLon;
use crate::util::mollweide::Mollweide;

/// 3 次メッシュの大きさ：緯度 30 秒、経度 45 秒
pub const DLAT: f64 = 30.0 / 3600.0;
pub const DLON: f64 = 45.0 / 3600.0;

/// 正方形の格子が載る地図の投影。どちらも正積図法なので、格子の面積はどこでも同じ。
#[derive(Clone, Copy, Debug)]
pub enum Projection {
    /// EPSG:3577（オーストラリア、ABS の格子人口）
    Albers(Albers),
    /// ESRI:54009（米国、GHS-POP）
    Mollweide(Mollweide),
}

impl Projection {
    /// 緯度・経度 → 平面座標 (x, y) [m]
    pub fn forward(&self, p: &LatLon) -> (f64, f64) {
        match self {
            Projection::Albers(a) => a.forward(p),
            Projection::Mollweide(m) => m.forward(p),
        }
    }

    /// 平面座標 (x, y) [m] → 緯度・経度
    pub fn inverse(&self, x: f64, y: f64) -> LatLon {
        match self {
            Projection::Albers(a) => a.inverse(x, y),
            Projection::Mollweide(m) => m.inverse(x, y),
        }
    }
}

pub enum GridGeometry {
    /// 正積図法の平面に、行ごとに並んだ正方形の格子（左上の格子の左上の角が (x0, y0)）
    Planar {
        width: usize,
        height: usize,
        x0: f64,
        y0: f64,
        dx: f64,
        dy: f64,
        proj: Projection,
    },
    /// 3 次メッシュ。`sw` は格子ごとの南西の角、`index` はメッシュの番号から格子の番号へ
    Mesh {
        sw: Vec<(f64, f64)>,
        index: HashMap<u64, usize>,
    },
}

impl GridGeometry {
    /// 格子 i の中の点。u, v は [0, 1) の数で、平面の格子では東西・南北、メッシュでは南北・東西の位置。
    pub fn point(&self, i: usize, u: f64, v: f64) -> LatLon {
        match self {
            GridGeometry::Planar {
                width,
                x0,
                y0,
                dx,
                dy,
                proj,
                ..
            } => {
                let (row, col) = (i / width, i % width);
                let x = x0 + (col as f64 + u) * dx;
                let y = y0 - (row as f64 + v) * dy;
                proj.inverse(x, y)
            }
            GridGeometry::Mesh { sw, .. } => {
                let (lat0, lon0) = sw[i];
                LatLon {
                    lat: lat0 + u * DLAT,
                    lon: lon0 + v * DLON,
                }
            }
        }
    }

    /// 点が入る約 10 km の粗い格子の番号と、その中心の緯度・経度。
    /// メッシュでは 2 次メッシュ（3 次メッシュ 10×10 個、緯度 5 分・経度 7.5 分）、平面の格子では 1 km の格子 10×10 個。
    /// 平面の格子で範囲の外なら None。
    pub fn coarse_cell(&self, p: &LatLon) -> Option<(u64, LatLon)> {
        match self {
            GridGeometry::Planar {
                width,
                height,
                x0,
                y0,
                dx,
                dy,
                proj,
            } => {
                let (x, y) = proj.forward(p);
                let col = ((x - x0) / dx).floor();
                let row = ((y0 - y) / dy).floor();
                if col < 0.0 || row < 0.0 || col >= *width as f64 || row >= *height as f64 {
                    return None;
                }
                let (bc, br) = ((col / 10.0).floor(), (row / 10.0).floor());
                let key = br as u64 * 1_000_000 + bc as u64;
                let center = proj.inverse(x0 + (bc * 10.0 + 5.0) * dx, y0 - (br * 10.0 + 5.0) * dy);
                Some((key, center))
            }
            GridGeometry::Mesh { .. } => {
                let key = code_of(p) / 100;
                let (lat0, lon0) = code_to_sw(key * 100);
                Some((
                    key,
                    LatLon {
                        lat: lat0 + 2.5 / 60.0,
                        lon: lon0 + 3.75 / 60.0,
                    },
                ))
            }
        }
    }

    /// 点が入る格子の番号。格子の外（メッシュでは載っていない格子）なら None。
    pub fn cell_of(&self, p: &LatLon) -> Option<usize> {
        match self {
            GridGeometry::Planar {
                width,
                height,
                x0,
                y0,
                dx,
                dy,
                proj,
            } => {
                let (x, y) = proj.forward(p);
                let col = ((x - x0) / dx).floor();
                let row = ((y0 - y) / dy).floor();
                if col < 0.0 || row < 0.0 || col >= *width as f64 || row >= *height as f64 {
                    return None;
                }
                Some(row as usize * width + col as usize)
            }
            GridGeometry::Mesh { index, .. } => index.get(&code_of(p)).copied(),
        }
    }

    /// jackknife の領域を作る平面座標。平面の格子ではその平面座標 [m]、
    /// 日本では北緯 36° で経度を縮めた正距円筒図法 [km]（領域を分けるだけなので、これで足りる）。
    pub fn plane(&self, p: &LatLon) -> (f64, f64) {
        match self {
            GridGeometry::Planar { proj, .. } => proj.forward(p),
            GridGeometry::Mesh { .. } => {
                (p.lon * 36f64.to_radians().cos() * 111.32, p.lat * 110.57)
            }
        }
    }
}

/// 3 次メッシュの番号 → 南西の角の緯度・経度。
/// 番号は pq rs t u v w の 8 桁で、緯度 = pq/1.5 + t·5' + v·30"、経度 = rs + 100 + u·7.5' + w·45"。
pub fn code_to_sw(code: u64) -> (f64, f64) {
    let pq = (code / 1_000_000) as f64;
    let rs = ((code / 10_000) % 100) as f64;
    let t = ((code / 1000) % 10) as f64;
    let u = ((code / 100) % 10) as f64;
    let v = ((code / 10) % 10) as f64;
    let w = (code % 10) as f64;
    (
        pq / 1.5 + t * 5.0 / 60.0 + v * DLAT,
        rs + 100.0 + u * 7.5 / 60.0 + w * DLON,
    )
}

/// 緯度・経度 → それが入る 3 次メッシュの番号。
pub fn code_of(p: &LatLon) -> u64 {
    let lat_min = p.lat * 60.0;
    let pq = (lat_min / 40.0).floor();
    let r1 = lat_min - pq * 40.0;
    let t = (r1 / 5.0).floor();
    let v = ((r1 - t * 5.0) * 60.0 / 30.0).floor();
    let rs = p.lon.floor() - 100.0;
    let l1 = (p.lon - p.lon.floor()) * 60.0;
    let u = (l1 / 7.5).floor();
    let w = ((l1 - u * 7.5) * 60.0 / 45.0).floor();
    (pq as u64) * 1_000_000
        + (rs as u64) * 10_000
        + (t as u64) * 1000
        + (u as u64) * 100
        + (v as u64) * 10
        + w as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokyo_station_is_in_53394611() {
        // 東京駅（北緯 35.681°、東経 139.767°）は 3 次メッシュ 5339-46-11
        assert_eq!(
            code_of(&LatLon {
                lat: 35.681,
                lon: 139.767
            }),
            53394611
        );
        let (lat, lon) = code_to_sw(53394611);
        assert!(lat <= 35.681 && 35.681 < lat + DLAT && lon <= 139.767 && 139.767 < lon + DLON);
    }

    #[test]
    fn coarse_cell_is_the_second_level_mesh() {
        // 東京駅は 2 次メッシュ 533946。中心はその 2 次メッシュの中にあり、同じ番号を返す。
        let g = GridGeometry::Mesh {
            sw: Vec::new(),
            index: HashMap::new(),
        };
        let p = LatLon {
            lat: 35.681,
            lon: 139.767,
        };
        let (key, c) = g.coarse_cell(&p).unwrap();
        assert_eq!(key, 533946);
        assert_eq!(code_of(&c) / 100, key);
        let (lat0, lon0) = code_to_sw(key * 100);
        assert!((c.lat - (lat0 + 2.5 / 60.0)).abs() < 1e-12);
        assert!((c.lon - (lon0 + 3.75 / 60.0)).abs() < 1e-12);
    }

    #[test]
    fn code_round_trips_at_cell_centres() {
        for code in [53394611u64, 30360000, 68414579, 36221137] {
            let (lat, lon) = code_to_sw(code);
            let c = LatLon {
                lat: lat + DLAT / 2.0,
                lon: lon + DLON / 2.0,
            };
            assert_eq!(code_of(&c), code);
        }
    }

    #[test]
    fn points_fall_in_their_cells() {
        let albers = GridGeometry::Planar {
            width: 3,
            height: 2,
            x0: 0.0,
            y0: -3_000_000.0,
            dx: 1000.0,
            dy: 1000.0,
            proj: Projection::Albers(Albers::epsg3577()),
        };
        let (lat0, lon0) = code_to_sw(53394611);
        let mesh = GridGeometry::Mesh {
            sw: vec![(lat0, lon0)],
            index: HashMap::from([(53394611, 0)]),
        };
        for (g, n) in [(&albers, 6), (&mesh, 1)] {
            for i in 0..n {
                for (u, v) in [(0.01, 0.01), (0.5, 0.5), (0.99, 0.99)] {
                    assert_eq!(g.cell_of(&g.point(i, u, v)), Some(i));
                }
            }
        }
    }
}

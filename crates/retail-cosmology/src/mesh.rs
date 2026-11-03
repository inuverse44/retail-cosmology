//! 日本の地域メッシュ（3 次メッシュ、約 1 km）の人口を読み、ランダムカタログを撒く。
//!
//! 入力は e-Stat の令和2年国勢調査 1 km メッシュ（T001140、世界測地系）の CSV。
//! ファイルに載っているのは人が住むメッシュだけで、人口 0 の陸地は載っていない。

use std::collections::HashMap;
use std::path::Path;

use crate::LatLon;
use crate::rng::SplitMix64;

/// 3 次メッシュの大きさ：緯度 30 秒、経度 45 秒
pub const DLAT: f64 = 30.0 / 3600.0;
pub const DLON: f64 = 45.0 / 3600.0;

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub code: u64,
    /// 南西の角
    pub lat0: f64,
    pub lon0: f64,
    pub pop: f64,
}

pub struct MeshGrid {
    pub cells: Vec<Cell>,
    index: HashMap<u64, usize>,
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

impl MeshGrid {
    /// フォルダの中の tblT001140S*.txt をすべて読む。人口は 5 列目（T001140001、人口総数）。
    pub fn read_dir(dir: &Path) -> std::io::Result<Self> {
        let mut cells = Vec::new();
        let mut names: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("tblT001140S"))
            })
            .collect();
        names.sort();
        for path in names {
            // 見出しの 2 行は Shift_JIS の日本語。データの行は ASCII なので、そのまま読める。
            let bytes = std::fs::read(&path)?;
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines().skip(2) {
                let f: Vec<&str> = line.split(',').collect();
                if f.len() < 5 {
                    continue;
                }
                let (Ok(code), Ok(pop)) = (f[0].parse::<u64>(), f[4].parse::<f64>()) else {
                    continue;
                };
                let (lat0, lon0) = code_to_sw(code);
                cells.push(Cell {
                    code,
                    lat0,
                    lon0,
                    pop,
                });
            }
        }
        Ok(Self::new(cells))
    }

    pub fn new(cells: Vec<Cell>) -> Self {
        let index = cells.iter().enumerate().map(|(i, c)| (c.code, i)).collect();
        Self { cells, index }
    }

    /// 緯度・経度が入るメッシュ。ファイルに載っていない（人が住まない）なら None。
    pub fn cell_of(&self, p: &LatLon) -> Option<&Cell> {
        self.index.get(&code_of(p)).map(|i| &self.cells[*i])
    }

    pub fn total(&self) -> f64 {
        self.cells.iter().map(|c| c.pop).sum()
    }

    /// メッシュを重み `weight(人口)` に比例する確率で選び、その中に一様に `n` 点を撒く。
    /// メッシュは緯度 30 秒の帯なので、帯の中の面積の違い（cos 緯度）は無視できる。
    pub fn sample(
        &self,
        n: usize,
        weight: impl Fn(f64) -> f64,
        rng: &mut SplitMix64,
    ) -> Vec<LatLon> {
        let mut cum = Vec::with_capacity(self.cells.len());
        let mut acc = 0.0;
        for c in &self.cells {
            acc += weight(c.pop).max(0.0);
            cum.push(acc);
        }
        (0..n)
            .map(|_| {
                let u = rng.next_f64() * acc;
                let k = cum.partition_point(|x| *x <= u).min(cum.len() - 1);
                let c = &self.cells[k];
                LatLon {
                    lat: c.lat0 + rng.next_f64() * DLAT,
                    lon: c.lon0 + rng.next_f64() * DLON,
                }
            })
            .collect()
    }
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
    fn samples_stay_in_weighted_cells() {
        let mk = |code, pop| {
            let (lat0, lon0) = code_to_sw(code);
            Cell {
                code,
                lat0,
                lon0,
                pop,
            }
        };
        let g = MeshGrid::new(vec![
            mk(53394611, 1.0),
            mk(53394612, 3.0),
            mk(53394613, 0.0),
        ]);
        let mut rng = SplitMix64::new(3);
        let pts = g.sample(40_000, |p| p, &mut rng);
        let mut n = HashMap::new();
        for p in &pts {
            *n.entry(g.cell_of(p).unwrap().code).or_insert(0usize) += 1;
        }
        assert_eq!(n.get(&53394613), None);
        let frac = n[&53394612] as f64 / pts.len() as f64; // 期待値 3/4
        assert!((frac - 0.75).abs() < 0.01, "{n:?}");
    }
}

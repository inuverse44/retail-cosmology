//! ABS の格子人口（Australian population grid、1 km 格子、EPSG:3577）を読み、
//! ランダムカタログを撒く。

use std::path::Path;

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use crate::LatLon;
use crate::albers::Albers;
use crate::rng::SplitMix64;

/// GeoTIFF の ModelPixelScaleTag と ModelTiepointTag
const MODEL_PIXEL_SCALE: u16 = 33550;
const MODEL_TIEPOINT: u16 = 33922;

pub struct PopGrid {
    pub width: usize,
    pub height: usize,
    /// 左上の格子の左上の角 [m]
    pub x0: f64,
    pub y0: f64,
    /// 格子の幅 [m]
    pub dx: f64,
    pub dy: f64,
    /// 行ごとに並んだ人口。陸地の外は負（欠損値 −1）。
    pub values: Vec<f32>,
    /// 展開で出力が埋まらなかったタイルの数（0 であるべき）
    pub short_tiles: usize,
    proj: Albers,
}

impl PopGrid {
    /// GeoTIFF を読む。タイルごとの LZW 圧縮（予測なし）、32 ビット浮動小数、1 バンドの形だけを扱う。
    ///
    /// ABS の GeoTIFF はタイルの LZW の列に終わりの印（EOI）がなく、`tiff` crate の展開は失敗する。
    /// そこでタイルを `weezl` で直接展開し、出力が埋まった時点で終わりとみなす。
    /// 埋まらなかったタイルの数は [`PopGrid::short_tiles`] に残す。
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        let mut dec = Decoder::new(std::io::Cursor::new(&bytes))?;
        let (w, h) = dec.dimensions()?;
        let (w, h) = (w as usize, h as usize);
        let compression = dec.get_tag_u32(Tag::Compression)?;
        let predictor = dec.find_tag_unsigned::<u32>(Tag::Predictor)?.unwrap_or(1);
        let bits = dec.get_tag_u32(Tag::BitsPerSample)?;
        let format = dec.get_tag_u32(Tag::SampleFormat)?;
        if (compression, predictor, bits, format) != (5, 1, 32, 3) {
            return Err(format!(
                "想定外の形式：圧縮 {compression}、予測 {predictor}、{bits} ビット、型 {format}"
            )
            .into());
        }
        let tw = dec.get_tag_u32(Tag::TileWidth)? as usize;
        let th = dec.get_tag_u32(Tag::TileLength)? as usize;
        let offsets = dec.get_tag_u64_vec(Tag::TileOffsets)?;
        let counts = dec.get_tag_u64_vec(Tag::TileByteCounts)?;
        // TIFF の先頭 2 バイトがバイト順を示す（"II" はリトルエンディアン、"MM" はビッグエンディアン）
        let little = bytes.starts_with(b"II");
        let scale = dec.get_tag_f64_vec(Tag::Unknown(MODEL_PIXEL_SCALE))?;
        let tie = dec.get_tag_f64_vec(Tag::Unknown(MODEL_TIEPOINT))?;
        // tie = [i, j, k, X, Y, Z]：画素 (i, j) の左上の角が平面座標 (X, Y)
        let (dx, dy) = (scale[0], scale[1]);
        let x0 = tie[3] - tie[0] * dx;
        let y0 = tie[4] + tie[1] * dy;

        let across = w.div_ceil(tw);
        let mut values = vec![-1.0f32; w * h];
        let mut tile = vec![0u8; tw * th * 4];
        let mut short_tiles = 0;
        for (t, (&off, &len)) in offsets.iter().zip(&counts).enumerate() {
            let input = &bytes[off as usize..(off + len) as usize];
            let mut lzw =
                weezl::decode::Configuration::with_tiff_size_switch(weezl::BitOrder::Msb, 8)
                    .build();
            let (mut i, mut o) = (0, 0);
            while o < tile.len() {
                let r = lzw.decode_bytes(&input[i..], &mut tile[o..]);
                i += r.consumed_in;
                o += r.consumed_out;
                match r.status? {
                    weezl::LzwStatus::Ok => {}
                    weezl::LzwStatus::Done | weezl::LzwStatus::NoProgress => break,
                }
            }
            if o < tile.len() {
                short_tiles += 1;
                tile[o..].fill(0);
            }
            let (tr, tc) = (t / across, t % across);
            for r in 0..th {
                let row = tr * th + r;
                if row >= h {
                    break;
                }
                for c in 0..tw {
                    let col = tc * tw + c;
                    if col >= w {
                        break;
                    }
                    let k = (r * tw + c) * 4;
                    let b = [tile[k], tile[k + 1], tile[k + 2], tile[k + 3]];
                    values[row * w + col] = if little {
                        f32::from_le_bytes(b)
                    } else {
                        f32::from_be_bytes(b)
                    };
                }
            }
        }
        Ok(Self {
            width: w,
            height: h,
            x0,
            y0,
            dx,
            dy,
            values,
            short_tiles,
            proj: Albers::epsg3577(),
        })
    }

    /// 緯度・経度が入る格子の番号。格子の外なら None。
    pub fn cell_of(&self, p: &LatLon) -> Option<usize> {
        let (x, y) = self.proj.forward(p);
        let col = ((x - self.x0) / self.dx).floor();
        let row = ((self.y0 - y) / self.dy).floor();
        if col < 0.0 || row < 0.0 || col >= self.width as f64 || row >= self.height as f64 {
            return None;
        }
        Some(row as usize * self.width + col as usize)
    }

    /// 陸地（値が 0 以上）の格子の数と、人口の合計。
    pub fn land_and_total(&self) -> (usize, f64) {
        let land = self.values.iter().filter(|v| **v >= 0.0).count();
        let total = self
            .values
            .iter()
            .filter(|v| **v > 0.0)
            .map(|v| *v as f64)
            .sum();
        (land, total)
    }

    /// 格子を重み `weight(人口)` に比例する確率で選び、その格子の中に一様に `n` 点を撒く。
    /// 陸地の外（値が負）の格子は選ばない。
    pub fn sample(
        &self,
        n: usize,
        weight: impl Fn(f32) -> f64,
        rng: &mut SplitMix64,
    ) -> Vec<LatLon> {
        let mut idx = Vec::new();
        let mut cum = Vec::new();
        let mut acc = 0.0;
        for (i, v) in self.values.iter().enumerate() {
            if *v < 0.0 {
                continue;
            }
            let w = weight(*v);
            if w > 0.0 {
                acc += w;
                idx.push(i);
                cum.push(acc);
            }
        }
        (0..n)
            .map(|_| {
                let u = rng.next_f64() * acc;
                let k = cum.partition_point(|c| *c <= u).min(cum.len() - 1);
                let (row, col) = (idx[k] / self.width, idx[k] % self.width);
                let x = self.x0 + (col as f64 + rng.next_f64()) * self.dx;
                let y = self.y0 - (row as f64 + rng.next_f64()) * self.dy;
                self.proj.inverse(x, y)
            })
            .collect()
    }
}

/// 一様：陸地の格子をすべて同じ重みで選ぶ。
pub fn uniform_land(_: f32) -> f64 {
    1.0
}

/// 人口に比例する重み。
pub fn population(v: f32) -> f64 {
    v as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> PopGrid {
        // 2 × 2 の格子：左上は海、右上は無人、左下に 1 人、右下に 3 人
        PopGrid {
            width: 2,
            height: 2,
            x0: 0.0,
            y0: -3_000_000.0,
            dx: 1000.0,
            dy: 1000.0,
            values: vec![-1.0, 0.0, 1.0, 3.0],
            short_tiles: 0,
            proj: Albers::epsg3577(),
        }
    }

    #[test]
    fn samples_follow_weights() {
        let g = tiny();
        let mut rng = SplitMix64::new(7);
        let n = 40_000;
        let count = |pts: &[LatLon]| {
            let mut c = [0usize; 4];
            for p in pts {
                c[g.cell_of(p).unwrap()] += 1;
            }
            c
        };
        let c = count(&g.sample(n, population, &mut rng));
        assert_eq!((c[0], c[1]), (0, 0));
        let frac = c[3] as f64 / n as f64; // 期待値 3/4、標準偏差 0.002
        assert!((frac - 0.75).abs() < 0.01, "{c:?}");

        let c = count(&g.sample(n, uniform_land, &mut rng));
        assert_eq!(c[0], 0);
        for k in 1..4 {
            let frac = c[k] as f64 / n as f64; // 期待値 1/3
            assert!((frac - 1.0 / 3.0).abs() < 0.01, "{c:?}");
        }
    }
}

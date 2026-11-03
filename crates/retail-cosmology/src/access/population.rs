//! ResourceAccess：人口の格子を読む。
//! オーストラリアは ABS の格子人口（GeoTIFF、1 km、EPSG:3577）、日本は e-Stat の令和2年国勢調査
//! 1 km メッシュ（T001140）。どちらも `PopulationGrid` で返す。

use std::collections::HashMap;
use std::path::Path;

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use crate::contract::PopulationGrid;
use crate::util::albers::Albers;
use crate::util::grid::{GridGeometry, code_to_sw};

/// GeoTIFF の ModelPixelScaleTag と ModelTiepointTag
const MODEL_PIXEL_SCALE: u16 = 33550;
const MODEL_TIEPOINT: u16 = 33922;

/// GeoTIFF を読む。タイルごとの LZW 圧縮（予測なし）、32 ビット浮動小数、1 バンドの形だけを扱う。
///
/// ABS の GeoTIFF はタイルの LZW の列に終わりの印（EOI）がなく、`tiff` crate の展開は失敗する。
/// そこでタイルを `weezl` で直接展開し、出力が埋まった時点で終わりとみなす。
/// 埋まらなかったタイルがあれば誤りを返す。
pub fn read_abs(path: &Path) -> Result<PopulationGrid, Box<dyn std::error::Error>> {
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
            weezl::decode::Configuration::with_tiff_size_switch(weezl::BitOrder::Msb, 8).build();
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
    if short_tiles > 0 {
        return Err(format!("GeoTIFF のタイルを {short_tiles} 個展開しきれなかった").into());
    }
    Ok(PopulationGrid {
        values,
        geometry: GridGeometry::Albers {
            width: w,
            height: h,
            x0,
            y0,
            dx,
            dy,
            proj: Albers::epsg3577(),
        },
        has_land: true,
    })
}

/// フォルダの中の tblT001140S*.txt をすべて読む。人口は 5 列目（T001140001、人口総数）。
pub fn read_estat(dir: &Path) -> std::io::Result<PopulationGrid> {
    let (mut values, mut sw, mut index) = (Vec::new(), Vec::new(), HashMap::new());
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
            index.insert(code, values.len());
            sw.push(code_to_sw(code));
            values.push(pop as f32);
        }
    }
    // e-Stat のメッシュは人が住む格子だけで、人口 0 の陸地の格子を含まない
    Ok(PopulationGrid {
        values,
        geometry: GridGeometry::Mesh { sw, index },
        has_land: false,
    })
}

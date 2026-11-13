//! ResourceAccess：人口の格子を読む。
//! オーストラリアは ABS の格子人口（GeoTIFF、1 km、EPSG:3577）、日本は e-Stat の令和2年国勢調査
//! 1 km メッシュ（T001140）、米国は JRC の GHS-POP（GeoTIFF、1 km、ESRI:54009）を本土 48 州と D.C. の境界で切り出したもの。
//! どれも `PopulationGrid` で返す。

use std::collections::HashMap;
use std::path::Path;

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use crate::contract::{PopulationGrid, StateBoundary};
use crate::util::albers::Albers;
use crate::util::grid::{GridGeometry, Projection, code_to_sw};
use crate::util::mollweide::Mollweide;
use crate::util::polygon::cells_inside;

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
        geometry: GridGeometry::Planar {
            width: w,
            height: h,
            x0,
            y0,
            dx,
            dy,
            proj: Projection::Albers(Albers::epsg3577()),
        },
        has_land: true,
    })
}

/// GHS-POP（BigTIFF、タイルごとの LZW 圧縮、予測なし、64 ビット浮動小数、1 バンド、ESRI:54009）から、
/// `states` の境界を囲む範囲だけを読み、どの州の内側にも中心がない格子を陸地の外（−1）にする。
/// GHS-POP は海が「値なし」（−200）、陸地は人口 0 以上なので、州の内側の陸地の格子だけが 0 以上になる。
/// 州ごとに塗ってから重ねる（隣り合う州の境界の頂点がずれていても、境界の上の格子を落とさない）。
pub fn read_ghsl(
    path: &Path,
    states: &[StateBoundary],
) -> Result<PopulationGrid, Box<dyn std::error::Error>> {
    let proj = Mollweide::esri54009();
    let bytes = std::fs::read(path)?;
    let mut dec = Decoder::new(std::io::Cursor::new(&bytes))?;
    let (w, h) = dec.dimensions()?;
    let (w, h) = (w as usize, h as usize);
    let compression = dec.get_tag_u32(Tag::Compression)?;
    let predictor = dec.find_tag_unsigned::<u32>(Tag::Predictor)?.unwrap_or(1);
    let bits = dec.get_tag_u32(Tag::BitsPerSample)?;
    let format = dec.get_tag_u32(Tag::SampleFormat)?;
    if (compression, predictor, bits, format) != (5, 1, 64, 3) {
        return Err(format!(
            "想定外の形式：圧縮 {compression}、予測 {predictor}、{bits} ビット、型 {format}"
        )
        .into());
    }
    let tw = dec.get_tag_u32(Tag::TileWidth)? as usize;
    let th = dec.get_tag_u32(Tag::TileLength)? as usize;
    let offsets = dec.get_tag_u64_vec(Tag::TileOffsets)?;
    let counts = dec.get_tag_u64_vec(Tag::TileByteCounts)?;
    let little = bytes.starts_with(b"II");
    let scale = dec.get_tag_f64_vec(Tag::Unknown(MODEL_PIXEL_SCALE))?;
    let tie = dec.get_tag_f64_vec(Tag::Unknown(MODEL_TIEPOINT))?;
    let (dx, dy) = (scale[0], scale[1]);
    let gx0 = tie[3] - tie[0] * dx;
    let gy0 = tie[4] + tie[1] * dy;

    // 境界を平面に移し、それを囲む格子の範囲（列 c0..c1、行 r0..r1）を求める
    let plane: Vec<Vec<Vec<(f64, f64)>>> = states
        .iter()
        .map(|s| {
            s.rings
                .iter()
                .map(|r| r.iter().map(|p| proj.forward(p)).collect())
                .collect()
        })
        .collect();
    let (mut xmin, mut xmax, mut ymin, mut ymax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &(x, y) in plane.iter().flatten().flatten() {
        (xmin, xmax, ymin, ymax) = (xmin.min(x), xmax.max(x), ymin.min(y), ymax.max(y));
    }
    let c0 = (((xmin - gx0) / dx).floor().max(0.0) as usize).min(w);
    let c1 = (((xmax - gx0) / dx).ceil().max(0.0) as usize).min(w);
    let r0 = (((gy0 - ymax) / dy).floor().max(0.0) as usize).min(h);
    let r1 = (((gy0 - ymin) / dy).ceil().max(0.0) as usize).min(h);
    let (cw, ch) = (c1 - c0, r1 - r0);
    let (x0, y0) = (gx0 + c0 as f64 * dx, gy0 - r0 as f64 * dy);

    let across = w.div_ceil(tw);
    let mut values = vec![-1.0f32; cw * ch];
    let mut tile = vec![0u8; tw * th * 8];
    for tr in r0 / th..r1.div_ceil(th) {
        for tc in c0 / tw..c1.div_ceil(tw) {
            let t = tr * across + tc;
            let (off, len) = (offsets[t] as usize, counts[t] as usize);
            if len == 0 {
                continue;
            }
            let input = &bytes[off..off + len];
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
                return Err(format!("GeoTIFF のタイル {t} を展開しきれなかった").into());
            }
            for r in 0..th {
                let row = tr * th + r;
                if row < r0 || row >= r1 {
                    continue;
                }
                for c in 0..tw {
                    let col = tc * tw + c;
                    if col < c0 || col >= c1 {
                        continue;
                    }
                    let k = (r * tw + c) * 8;
                    let b: [u8; 8] = tile[k..k + 8].try_into().unwrap();
                    let v = if little {
                        f64::from_le_bytes(b)
                    } else {
                        f64::from_be_bytes(b)
                    };
                    values[(row - r0) * cw + (col - c0)] = if v >= 0.0 { v as f32 } else { -1.0 };
                }
            }
        }
    }
    let mut inside = vec![false; cw * ch];
    for rings in &plane {
        for (k, b) in cells_inside(rings, cw, ch, (x0, y0), (dx, dy))
            .into_iter()
            .enumerate()
        {
            inside[k] |= b;
        }
    }
    for (v, keep) in values.iter_mut().zip(&inside) {
        if !keep {
            *v = -1.0;
        }
    }
    Ok(PopulationGrid {
        values,
        geometry: GridGeometry::Planar {
            width: cw,
            height: ch,
            x0,
            y0,
            dx,
            dy,
            proj: Projection::Mollweide(proj),
        },
        has_land: true,
    })
}

/// フォルダの中の tblT001140S*.txt をすべて読む。人口は 5 列目（T001140001、人口総数）。
pub fn read_estat(dir: &Path) -> std::io::Result<PopulationGrid> {
    read_estat_in(dir, |_| true)
}

/// `read_estat` と同じだが、`keep` が真を返す 3 次メッシュの番号の格子だけを読む（#51、九州の中だけの測定）。
pub fn read_estat_in(dir: &Path, keep: impl Fn(u64) -> bool) -> std::io::Result<PopulationGrid> {
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
            if !keep(code) {
                continue;
            }
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

/// 総務省統計局の「市区町村別メッシュ・コード一覧」（scripts/estat/fetch_mesh_prefecture.sh が書いた
/// data/estat/mesh_pref/<県>.csv）から、`prefectures`（都道府県の番号 1〜47）に入る 3 次メッシュの番号を集める。
/// 県境にまたがる格子は両方の県に載っているので、どちらの県を選んでも入る。
pub fn read_mesh_prefectures(
    dir: &Path,
    prefectures: &[u32],
) -> std::io::Result<std::collections::HashSet<u64>> {
    let mut out = std::collections::HashSet::new();
    for n in prefectures {
        let bytes = std::fs::read(dir.join(format!("{n:02}.csv")))?;
        // 見出しの 1 行は Shift_JIS の日本語。データの行の 1 列目と 3 列目は数字なので、そのまま読める。
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            if let Some(code) = f.get(2).and_then(|c| c.trim().parse::<u64>().ok()) {
                out.insert(code);
            }
        }
    }
    Ok(out)
}

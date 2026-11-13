//! ResourceAccess：州の境界を読む。米国は Natural Earth の州境（1:10m Admin 1、GeoJSON）。
//! 人口の格子を米国本土に限るのと、OSM の店を州に割り振るのに使う（#51）。

use std::path::Path;

use crate::contract::StateBoundary;
use crate::util::geo::LatLon;

/// 米国本土の外の州（アラスカとハワイ）
const OUTSIDE_CONTIGUOUS: [&str; 2] = ["US-AK", "US-HI"];

/// Natural Earth の州境（GeoJSON、ne_10m_admin_1_states_provinces）から、米国本土 48 州とワシントン D.C. の境界を読む。
pub fn read_us_states(path: &Path) -> Result<Vec<StateBoundary>, Box<dyn std::error::Error>> {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut out = Vec::new();
    for f in json["features"].as_array().ok_or("features がない")? {
        let props = &f["properties"];
        let state = props["iso_3166_2"].as_str().unwrap_or_default();
        if props["adm0_a3"] != "USA" || OUTSIDE_CONTIGUOUS.contains(&state) {
            continue;
        }
        let geom = &f["geometry"];
        let polys: Vec<&serde_json::Value> = match geom["type"].as_str() {
            Some("Polygon") => vec![&geom["coordinates"]],
            Some("MultiPolygon") => geom["coordinates"]
                .as_array()
                .ok_or("MultiPolygon の座標がない")?
                .iter()
                .collect(),
            t => return Err(format!("{state} の形が想定外：{t:?}").into()),
        };
        let mut rings = Vec::new();
        for poly in polys {
            for ring in poly.as_array().ok_or("多角形の形が違う")? {
                rings.push(
                    ring.as_array()
                        .ok_or("輪の形が違う")?
                        .iter()
                        .map(|c| LatLon {
                            lon: c[0].as_f64().unwrap_or(f64::NAN),
                            lat: c[1].as_f64().unwrap_or(f64::NAN),
                        })
                        .collect(),
                );
            }
        }
        out.push(StateBoundary {
            state: state.to_string(),
            rings,
        });
    }
    if out.len() != 49 {
        return Err(format!("米国本土の州が {} 個（49 個のはず）", out.len()).into());
    }
    out.sort_by(|a, b| a.state.cmp(&b.state));
    Ok(out)
}

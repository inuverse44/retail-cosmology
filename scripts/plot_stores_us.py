"""Walmart の店舗を、海岸線つきの米国本土の地図に打つ（#51）。店は OSM から取ったもの。

scripts/osm/fetch.sh 2026-11-13 walmart
cargo run --release --bin osm_census -- 2026-11-13 walmart
scripts/naturalearth/fetch.sh
uv run scripts/plot_stores_us.py 2026-11-13
"""

import csv
import json
import sys

import coastline
import figstyle as fs
import numpy as np

ROOT = fs.ROOT
STATES = ROOT / "data" / "naturalearth" / "ne_10m_admin_1_states_provinces.geojson"
# 米国本土 [deg]
LON = (-125.5, -66.0)
LAT = (24.0, 50.0)


def draw_states(ax) -> None:
    """米国本土の州境を、海岸線より薄い線で描く（店を州に割り振った境界と同じ、Natural Earth の Admin 1）。"""
    for f in json.load(open(STATES, encoding="utf-8"))["features"]:
        p = f["properties"]
        if p["adm0_a3"] != "USA" or p["iso_3166_2"] in ("US-AK", "US-HI"):
            continue
        g = f["geometry"]
        polys = [g["coordinates"]] if g["type"] == "Polygon" else g["coordinates"]
        for poly in polys:
            for ring in poly:
                ax.plot([q[0] for q in ring], [q[1] for q in ring], color="0.82", lw=0.3, zorder=0)


def main() -> None:
    date = sys.argv[1]
    rows = csv.DictReader(open(ROOT / "data" / "osm" / f"walmart_clean_{date}.csv", encoding="utf-8"))
    pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows])
    fig, ax = fs.figure(1, 1, width=0.8, height=2.8)
    draw_states(ax)
    coastline.draw(ax, LON, LAT)
    ax.scatter(pts[:, 0], pts[:, 1], s=1.2, color=fs.COLOR["WALMART"], lw=0)
    fs.panel(ax, None, f"WALMART ({len(pts)} stores)")
    # 北緯 37° 付近で、経度 1° と緯度 1° の長さをそろえる
    ax.set_aspect(1 / np.cos(np.radians(37)))
    ax.set_xlabel("longitude [deg]")
    ax.set_ylabel("latitude [deg]")
    credit = "coastline and states: Natural Earth"
    ax.text(0.01, 0.02, credit, transform=ax.transAxes, fontsize=6, color="0.4", ha="left")
    fs.save(fig, "stores_us")


if __name__ == "__main__":
    main()

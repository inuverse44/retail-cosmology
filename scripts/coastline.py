"""Natural Earth の海岸線を読み、地図の枠に入る線だけを描く（scripts/naturalearth/fetch.sh で取る）。"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "data" / "naturalearth" / "ne_10m_coastline.geojson"
CREDIT = "coastline: Natural Earth"


def lines(lon: tuple[float, float], lat: tuple[float, float]) -> list[tuple[list[float], list[float]]]:
    """経度・緯度の範囲に一部でも入る海岸線を、(経度の列, 緯度の列) の一覧で返す。"""
    out = []
    for f in json.load(open(PATH, encoding="utf-8"))["features"]:
        g = f["geometry"]
        parts = g["coordinates"] if g["type"] == "MultiLineString" else [g["coordinates"]]
        for part in parts:
            xs = [p[0] for p in part]
            ys = [p[1] for p in part]
            if max(xs) >= lon[0] and min(xs) <= lon[1] and max(ys) >= lat[0] and min(ys) <= lat[1]:
                out.append((xs, ys))
    return out


def draw(ax, lon: tuple[float, float], lat: tuple[float, float]) -> None:
    """海岸線を細い灰色の線で描き、枠を範囲に合わせる。"""
    for xs, ys in lines(lon, lat):
        ax.plot(xs, ys, color="0.6", lw=0.4, zorder=0)
    ax.set_xlim(*lon)
    ax.set_ylim(*lat)

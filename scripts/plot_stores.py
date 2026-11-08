"""OSM から選び出した Coles と Woolworths の店舗を、海岸線つきの地図に打つ。

cargo run --release --bin osm_census -- 2026-11-01
scripts/naturalearth/fetch.sh
uv run scripts/plot_stores.py 2026-11-01
"""

import csv
import sys

import coastline
import figstyle as fs
import numpy as np

ROOT = fs.ROOT
CHAINS = ("Coles", "Woolworths")
# 本土とタスマニアが入る範囲 [deg]
LON = (112.0, 155.0)
LAT = (-44.5, -9.5)


def main() -> None:
    date = sys.argv[1]
    rows = list(csv.DictReader(open(ROOT / "data" / "osm" / f"stores_clean_{date}.csv", encoding="utf-8")))
    fig, axes = fs.figure(1, 2, height=3.0, sharex=True, sharey=True)
    for ax, tag, chain in zip(axes, "ab", CHAINS, strict=True):
        pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows if r["chain"] == chain])
        coastline.draw(ax, LON, LAT)
        ax.scatter(pts[:, 0], pts[:, 1], s=1.5, color=fs.COLOR[chain], lw=0)
        fs.panel(ax, tag, f"{chain} ({len(pts)} stores)")
        # 緯度 27° S 付近で、経度 1° と緯度 1° の長さをそろえる
        ax.set_aspect(1 / np.cos(np.radians(27)))
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    credit = f"© OpenStreetMap contributors (ODbL)\n{coastline.CREDIT}"
    axes[1].text(0.03, 0.03, credit, transform=axes[1].transAxes, fontsize=6, color="0.4", va="bottom")
    fs.save(fig, "stores_osm")


if __name__ == "__main__":
    main()

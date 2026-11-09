"""jackknife の 64 領域を、人口に比例するランダムの点の色で地図に打つ（#44）。

cargo run --release --bin jk_regions -- --country au
cargo run --release --bin jk_regions -- --country jp
scripts/naturalearth/fetch.sh
uv run scripts/plot_jk_regions.py
"""

import csv

import coastline
import figstyle as fs
import matplotlib as mpl
import numpy as np

PANELS = [
    ("a", "au", "Australia", (112.0, 155.0), (-44.5, -9.5), 27, "Population: ABS (CC BY 4.0)"),
    ("b", "jp", "Japan", (128.5, 146.0), (30.5, 45.7), 38, "Population: e-Stat"),
]
PALETTE = mpl.colormaps["tab20"].colors


def main() -> None:
    fig, axes = fs.figure(1, 2, height=2.9)
    for ax, (tag, country, name, lon, lat, lat0, credit) in zip(axes, PANELS, strict=True):
        src = fs.ROOT / "data" / "random" / f"jk_regions_{country}_seed1.csv"
        rows = list(csv.DictReader(open(src, encoding="utf-8")))
        xy = np.array([(float(r["lon"]), float(r["lat"])) for r in rows])
        reg = np.array([int(r["region"]) for r in rows])
        # 番号の近い領域は地図でも隣り合うので、色は番号を 7 ずつ飛ばして選ぶ
        colors = [PALETTE[(k * 7) % len(PALETTE)] for k in reg]
        coastline.draw(ax, lon, lat)
        ax.scatter(xy[:, 0], xy[:, 1], s=0.6, c=colors, lw=0, rasterized=True)
        fs.panel(ax, tag, f"{name} ({reg.max() + 1} regions)")
        ax.set_aspect(1 / np.cos(np.radians(lat0)))
        ax.set_xlabel("longitude [deg]")
        # オーストラリアは右下にタスマニアがあるので、出典は左上に置く
        x, y, ha, va = (0.03, 0.97, "left", "top") if country == "au" else (0.97, 0.03, "right", "bottom")
        ax.text(x, y, f"{credit}\n{coastline.CREDIT}", transform=ax.transAxes,
                fontsize=5, color="0.4", ha=ha, va=va)
    axes[0].set_ylabel("latitude [deg]")
    fs.save(fig, "jk_regions")


if __name__ == "__main__":
    main()

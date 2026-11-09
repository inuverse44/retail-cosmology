"""オーストラリアのランダムカタログ（陸地に一様、人口に比例）と、人口そのものの点を地図に打つ（#42）。

cargo run --release --bin random_points
scripts/naturalearth/fetch.sh
uv run scripts/plot_random.py
"""

import csv

import coastline
import figstyle as fs
import numpy as np

SRC = fs.ROOT / "data" / "random" / "random_au_seed1.csv"
LON = (112.0, 155.0)
LAT = (-44.5, -9.5)
PANELS = [
    ("a", "land", "land"),
    ("b", "population", "population"),
    ("c", "pop_data", "population points"),
]
SIZE = {"land": 0.3, "population": 0.6, "pop_data": 1.0}


def main() -> None:
    rows = list(csv.DictReader(open(SRC, encoding="utf-8")))
    fig, axes = fs.figure(1, 3, height=2.3, sharex=True, sharey=True)
    for ax, (tag, kind, name) in zip(axes, PANELS, strict=True):
        pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows if r["kind"] == kind])
        coastline.draw(ax, LON, LAT)
        color = fs.COLOR["population"] if kind != "land" else fs.COLOR["data"]
        ax.scatter(pts[:, 0], pts[:, 1], s=SIZE[kind], color=color, lw=0)
        fs.panel(ax, tag, f"{name} ({len(pts):,})")
        ax.set_aspect(1 / np.cos(np.radians(27)))
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    credit = f"Population: ABS (CC BY 4.0)\n{coastline.CREDIT}"
    axes[2].text(0.03, 0.03, credit, transform=axes[2].transAxes, fontsize=5, color="0.4", va="bottom")
    fs.save(fig, "random_au")


if __name__ == "__main__":
    main()

"""OSM から選び出した Coles と Woolworths の店舗を地図に打つ。

cargo run --release --bin osm_census -- 2026-11-01
uv run scripts/plot_stores.py 2026-11-01
"""

import csv
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
COLORS = {"Coles": "#d62728", "Woolworths": "#2ca02c"}


def main() -> None:
    date = sys.argv[1]
    rows = list(csv.DictReader(open(ROOT / "data" / "osm" / f"stores_clean_{date}.csv", encoding="utf-8")))
    fig, axes = plt.subplots(1, 2, figsize=(9, 4.2), sharex=True, sharey=True)
    for ax, chain in zip(axes, COLORS, strict=True):
        pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows if r["chain"] == chain])
        ax.scatter(pts[:, 0], pts[:, 1], s=2, color=COLORS[chain], lw=0)
        ax.set_title(f"{chain} ({len(pts)} stores)")
        # 緯度 27° S 付近で、経度 1° と緯度 1° の長さをそろえる
        ax.set_aspect(1 / np.cos(np.radians(27)))
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    credit = f"© OpenStreetMap contributors (ODbL)\n{date}"
    axes[1].text(0.03, 0.03, credit, transform=axes[1].transAxes, fontsize=7, color="0.4", va="bottom")
    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "stores_osm"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

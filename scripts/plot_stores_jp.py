"""トライアルと西友の店舗を、海岸線つきの日本の地図に打つ。店は公式サイトから取ったもの。

cargo run --release --bin trial_census -- 2026-11-03 2026-11-01
cargo run --release --bin seiyu_census -- 2026-11-04 2026-11-03
scripts/naturalearth/fetch.sh
uv run scripts/plot_stores_jp.py 2026-11-03 2026-11-04
"""

import csv
import sys
from pathlib import Path

import coastline
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
# (チェーン, 図の名前, 色, 店のファイルの置き場所)
CHAINS = [
    ("TRIAL", "TRIAL", "#1f77b4", "trial_site/trial_site_clean_{}.csv"),
    ("SEIYU", "SEIYU", "#ff7f0e", "seiyu_site/seiyu_site_clean_{}.csv"),
]
# 九州から北海道まで（店のない沖縄は入れない）[deg]
LON = (128.5, 146.0)
LAT = (30.5, 45.7)


def main() -> None:
    dates = sys.argv[1:3]
    fig, axes = plt.subplots(1, 2, figsize=(9, 4.8), sharex=True, sharey=True)
    for ax, (chain, title, color, path), date in zip(axes, CHAINS, dates, strict=True):
        rows = csv.DictReader(open(ROOT / "data" / path.format(date), encoding="utf-8"))
        pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows if r["chain"] == chain])
        coastline.draw(ax, LON, LAT)
        ax.scatter(pts[:, 0], pts[:, 1], s=3, color=color, lw=0)
        ax.set_title(f"{title} ({len(pts)} stores, {date})")
        # 北緯 38° 付近で、経度 1° と緯度 1° の長さをそろえる
        ax.set_aspect(1 / np.cos(np.radians(38)))
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    axes[1].text(0.97, 0.03, coastline.CREDIT, transform=axes[1].transAxes, fontsize=7, color="0.4", ha="right")
    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "stores_jp"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200, bbox_inches="tight")


if __name__ == "__main__":
    main()

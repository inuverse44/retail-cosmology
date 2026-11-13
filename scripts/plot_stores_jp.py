"""トライアル、西友、コスモスの店舗を、海岸線つきの日本の地図に打つ。店は公式サイトから取ったもの。

cargo run --release --bin trial_census -- 2026-11-03 2026-11-01
cargo run --release --bin seiyu_census -- 2026-11-04 2026-11-03
cargo run --release --bin cosmos_census -- 2026-11-12 2026-11-01
scripts/naturalearth/fetch.sh
uv run scripts/plot_stores_jp.py 2026-11-03 2026-11-04 2026-11-12
"""

import csv
import sys

import coastline
import figstyle as fs
import numpy as np

ROOT = fs.ROOT
# (チェーン, 店のファイルの置き場所)
CHAINS = [
    ("TRIAL", "trial_site/trial_site_clean_{}.csv"),
    ("SEIYU", "seiyu_site/seiyu_site_clean_{}.csv"),
    ("COSMOS", "cosmos_site/cosmos_site_clean_{}.csv"),
]
# 九州から北海道まで（店のない沖縄は入れない）[deg]
LON = (128.5, 146.0)
LAT = (30.5, 45.7)


def main() -> None:
    dates = sys.argv[1:4]
    fig, axes = fs.figure(1, 3, height=2.6, sharex=True, sharey=True)
    for ax, tag, (chain, path), date in zip(axes, "abc", CHAINS, dates, strict=True):
        rows = csv.DictReader(open(ROOT / "data" / path.format(date), encoding="utf-8"))
        pts = np.array([(float(r["lon"]), float(r["lat"])) for r in rows if r["chain"] == chain])
        coastline.draw(ax, LON, LAT)
        ax.scatter(pts[:, 0], pts[:, 1], s=2, color=fs.COLOR[chain], lw=0)
        fs.panel(ax, tag, f"{chain} ({len(pts)} stores)")
        # 北緯 38° 付近で、経度 1° と緯度 1° の長さをそろえる
        ax.set_aspect(1 / np.cos(np.radians(38)))
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    axes[-1].text(0.97, 0.03, coastline.CREDIT, transform=axes[-1].transAxes, fontsize=6, color="0.4", ha="right")
    fs.save(fig, "stores_jp")


if __name__ == "__main__":
    main()

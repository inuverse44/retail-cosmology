"""選択関数の説明の図（#43）。
(a) 単位球の弦の長さ c と大円距離の関係（説明のための絵）。
(b)--(d) トライアルの選択関数 S_R を、人口に比例して撒いた参照の点ごとに色で打つ（R = 50, 100, 200 km）。

cargo run --release --bin selection_map -- 2026-11-03
scripts/naturalearth/fetch.sh
uv run scripts/plot_selection.py 2026-11-03
"""

import csv
import sys

import coastline
import figstyle as fs
import matplotlib.colors as mcolors
import numpy as np

LON = (128.5, 146.0)
LAT = (30.5, 45.7)
WIDTHS = (50, 100, 200)


def geometry(ax) -> None:
    """単位円（球の断面）の上の二点 x, y と、弧（大円距離）と弦 c。"""
    t = np.linspace(0, 2 * np.pi, 400)
    ax.plot(np.cos(t), np.sin(t), color="0.6", lw=0.8)
    a, b = np.radians(35), np.radians(145)
    arc = np.linspace(a, b, 100)
    ax.plot(np.cos(arc), np.sin(arc), color=fs.COLOR["TRIAL"], lw=2.0)
    ax.plot([np.cos(a), np.cos(b)], [np.sin(a), np.sin(b)], color=fs.COLOR["data"], lw=1.0, ls="--")
    ax.plot([0, np.cos(a)], [0, np.sin(a)], color="0.6", lw=0.6)
    ax.plot([0, np.cos(b)], [0, np.sin(b)], color="0.6", lw=0.6)
    for ang, name in ((a, r"$\bm{x}$"), (b, r"$\bm{y}$")):
        ax.plot(np.cos(ang), np.sin(ang), "o", color=fs.COLOR["data"], ms=3)
        ax.text(1.15 * np.cos(ang), 1.15 * np.sin(ang), name, ha="center", va="center")
    ax.text(0, 1.06, r"$d$", ha="center", va="bottom", color=fs.COLOR["TRIAL"])
    ax.text(0, np.sin(a) + 0.04, r"$c$", ha="center", va="bottom")
    ax.text(0, 0.12, r"$d/R_\oplus$", ha="center", va="bottom", color="0.4", fontsize=fs.SMALL)
    ax.text(0, -1.25, r"$c = 2\sin(d/2R_\oplus)$", ha="center", va="top", fontsize=fs.SMALL)
    ax.set(xlim=(-1.3, 1.3), ylim=(-1.6, 1.35), aspect="equal", xticks=[], yticks=[])
    ax.minorticks_off()
    for s in ax.spines.values():
        s.set_visible(False)


def main() -> None:
    date = sys.argv[1]
    src = fs.ROOT / "data" / "selection" / f"selection_TRIAL_{date}.csv"
    rows = list(csv.DictReader(open(src, encoding="utf-8")))
    lon = np.array([float(r["lon"]) for r in rows])
    lat = np.array([float(r["lat"]) for r in rows])
    fig = fs.plt.figure(figsize=(fs.TEXTWIDTH, 2.6), layout="constrained")
    gs = fig.add_gridspec(1, 4, width_ratios=[0.8, 1, 1, 1])
    geometry(fig.add_subplot(gs[0]))
    fs.panel(fig.axes[0], "a", "")
    norm = mcolors.LogNorm(vmin=0.1, vmax=10)
    axes = [fig.add_subplot(gs[i + 1]) for i in range(3)]
    for ax, tag, w in zip(axes, "bcd", WIDTHS, strict=True):
        s = np.array([float(r[f"S_{w}"]) for r in rows])
        s = np.clip(s, 0.1, 10)
        coastline.draw(ax, LON, LAT)
        order = np.argsort(s)
        sc = ax.scatter(lon[order], lat[order], c=s[order], s=0.4, lw=0, cmap="RdBu_r", norm=norm, rasterized=True)
        fs.panel(ax, tag, rf"$R = {w}\,\mathrm{{km}}$")
        ax.set_aspect(1 / np.cos(np.radians(38)))
        ax.set_xticks([130, 140])
        if ax is not axes[0]:
            ax.set_yticklabels([])
        ax.set_xlabel("longitude [deg]")
    axes[0].set_ylabel("latitude [deg]")
    cb = fig.colorbar(sc, ax=axes, shrink=0.8, pad=0.01)
    cb.set_label(r"$S_R$")
    fs.save(fig, "selection_trial")


if __name__ == "__main__":
    main()

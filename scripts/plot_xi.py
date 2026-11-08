"""店舗と人口の二点相関関数を、jackknife の誤差棒つきで描く。

cargo run --release --bin xi -- 2026-11-01
uv run scripts/plot_xi.py 2026-11-01
"""

import csv
import sys

import figstyle as fs
import numpy as np
from matplotlib.lines import Line2D

ROOT = fs.ROOT


def load(date: str) -> dict:
    out: dict = {}
    for r in csv.DictReader(open(ROOT / "data" / "xi" / f"xi_{date}.csv", encoding="utf-8")):
        out.setdefault((r["quantity"], r["sample"], r["random"]), []).append(r)
    res = {}
    for key, rows in out.items():
        lo = np.array([float(r["r_lo_km"]) for r in rows])
        hi = np.array([float(r["r_hi_km"]) for r in rows])
        res[key] = (np.sqrt(lo * hi), np.array([float(r["value"]) for r in rows]), np.array([float(r["sigma"]) for r in rows]))
    return res


def main() -> None:
    date = sys.argv[1]
    d = load(date)
    fig, axes = fs.figure(1, 3, height=2.3)

    ax = axes[0]
    for s in ("population", "Coles", "Woolworths"):
        r, v, e = d[("ls", s, "land")]
        ok = v > 0
        fs.errorbar(ax, r[ok] * fs.SHIFT[s], v[ok], e[ok], s)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "a", "uniform on land")

    ax = axes[1]
    fs.ref_line(ax, 1)
    for s in ("Coles", "Woolworths"):
        r, v, e = d[("ratio_to_population", s, "land")]
        ok = r < 300  # これより長い距離は人口の ξ が 0 に近く、比が定まらない
        fs.errorbar(ax, r[ok] * fs.SHIFT[s], v[ok], e[ok], s, label="_")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{store}} / \hat\xi_{\mathrm{population}}$")
    fs.panel(ax, "b", "stores / population")

    ax = axes[2]
    fs.ref_line(ax, 0)
    for s in ("Coles", "Woolworths"):
        r, v, e = d[("ls", s, "population")]
        fs.errorbar(ax, r * fs.SHIFT[s], v, e, s, label="_")
        r, v, e = d[("dr_over_rr_minus_1", s, "population")]
        ax.plot(r * fs.SHIFT[s], v, color=fs.COLOR[s], ls=":", lw=1.0)
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi$")
    fs.panel(ax, "c", "weighted by population")
    ax.legend(
        handles=[
            Line2D([], [], color="0.2", marker="o", label=r"$\hat\xi_{\mathrm{LS}}$"),
            Line2D([], [], color="0.2", ls=":", label=r"$dr/rr - 1$"),
        ],
        loc="lower right",
    )

    fig.legend(*axes[0].get_legend_handles_labels(), loc="outside upper center", ncol=3)
    fs.save(fig, "xi_stores")


if __name__ == "__main__":
    main()

"""店舗と人口の二点相関関数を、jackknife の誤差棒つきで描く。

cargo run --release --bin xi -- 2026-11-01
uv run scripts/plot_xi.py 2026-11-01
"""

import csv
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
COLORS = {"Coles": "#d62728", "Woolworths": "#2ca02c", "population": "0.3"}
# 同じビンの点が重ならないよう、横に少しずらす
SHIFT = {"Coles": 0.97, "Woolworths": 1.03, "population": 1.0}


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
    fig, axes = plt.subplots(1, 3, figsize=(13, 4))

    ax = axes[0]
    for s in ("population", "Coles", "Woolworths"):
        r, v, e = d[("ls", s, "land")]
        ok = v > 0
        ax.errorbar(r[ok] * SHIFT[s], v[ok], yerr=e[ok], color=COLORS[s], marker="o", ms=3, lw=1, capsize=2, label=s)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\rm LS}(r)$")
    ax.set_title("(a) random: uniform on land")
    ax.legend(frameon=False, fontsize=8)

    ax = axes[1]
    ax.axhline(1, color="0.6", lw=0.8)
    for s in ("Coles", "Woolworths"):
        r, v, e = d[("ratio_to_population", s, "land")]
        ok = r < 300  # これより長い距離は人口の ξ が 0 に近く、比が定まらない
        ax.errorbar(r[ok] * SHIFT[s], v[ok], yerr=e[ok], color=COLORS[s], marker="o", ms=3, lw=1, capsize=2, label=s)
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\rm store} / \hat\xi_{\rm population}$")
    ax.set_title("(b) stores relative to population")
    ax.legend(frameon=False, fontsize=8)

    ax = axes[2]
    ax.axhline(0, color="0.6", lw=0.8)
    for s in ("Coles", "Woolworths"):
        r, v, e = d[("ls", s, "population")]
        ax.errorbar(r * SHIFT[s], v, yerr=e, color=COLORS[s], marker="o", ms=3, lw=1, capsize=2, label=f"{s}, Landy–Szalay")
        r, v, e = d[("dr_over_rr_minus_1", s, "population")]
        ax.plot(r * SHIFT[s], v, color=COLORS[s], ls=":", lw=1.2, label=f"{s}, $dr/rr-1$")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi(r)$")
    ax.set_title("(c) random: weighted by population")
    ax.legend(frameon=False, fontsize=7)

    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "xi_stores"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

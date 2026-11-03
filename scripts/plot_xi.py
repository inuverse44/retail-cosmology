"""店舗と人口の二点相関関数を描く。

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


def load(date: str) -> dict:
    rows = list(csv.DictReader(open(ROOT / "data" / "xi" / f"xi_{date}.csv", encoding="utf-8")))
    out: dict = {}
    for r in rows:
        out.setdefault((r["sample"], r["random"]), []).append(r)
    return out


def arrays(rows: list) -> dict:
    a = {k: np.array([float(r[k]) for r in rows]) for k in ("r_lo_km", "r_hi_km", "n_d", "n_r", "dd", "dr", "rr", "xi_ls")}
    nd, nr = a["n_d"], a["n_r"]
    dd = a["dd"] / (nd * (nd - 1) / 2)
    rr = a["rr"] / (nr * (nr - 1) / 2)
    a["r"] = np.sqrt(a["r_lo_km"] * a["r_hi_km"])
    a["natural"] = dd / rr - 1
    return a


def main() -> None:
    date = sys.argv[1]
    d = load(date)
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(10, 4))

    for sample in ("population", "Coles", "Woolworths"):
        a = arrays(d[(sample, "land")])
        ok = a["xi_ls"] > 0
        style = {"ls": "--", "marker": None} if sample == "population" else {"ls": "-", "marker": "o", "ms": 3}
        ax1.plot(a["r"][ok], a["xi_ls"][ok], color=COLORS[sample], lw=1.2, label=sample, **style)
    ax1.set_xscale("log")
    ax1.set_yscale("log")
    ax1.set_xlabel(r"$r$ [km]")
    ax1.set_ylabel(r"$\hat\xi_{\rm LS}(r)$")
    ax1.set_title("random: uniform on land")
    ax1.legend(frameon=False, fontsize=8)

    ax2.axhline(0, color="0.6", lw=0.8)
    for sample in ("Coles", "Woolworths"):
        a = arrays(d[(sample, "population")])
        ax2.plot(a["r"], a["xi_ls"], color=COLORS[sample], marker="o", ms=3, lw=1.2, label=f"{sample}, Landy–Szalay")
        ax2.plot(a["r"], a["natural"], color=COLORS[sample], ls=":", lw=1.2, label=f"{sample}, natural")
    ax2.set_xscale("log")
    ax2.set_xlabel(r"$r$ [km]")
    ax2.set_ylabel(r"$\hat\xi(r)$")
    ax2.set_title("random: weighted by population")
    ax2.legend(frameon=False, fontsize=7)

    fig.text(0.99, 0.01, "no error bars yet", ha="right", va="bottom", fontsize=7, color="0.4")
    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "xi_stores"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

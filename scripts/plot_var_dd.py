"""オーストラリアの店について、ξ の jackknife の誤差と、DD の分散の式（ノートの付録 B）を比べる図を描く。

cargo run --release --bin xi -- 2026-11-01
cargo run --release --bin var_dd -- 2026-11-01
uv run scripts/plot_var_dd.py
"""

import csv
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DATE = "2026-11-01"
XI = ROOT / "data" / "xi" / f"xi_{DATE}.csv"
VAR = ROOT / "data" / "var" / f"var_dd_au_{DATE}.csv"
OUT = ROOT / "docs" / "figures" / "var_dd_au"
# 表と同じランダムカタログ（人口に比例、40000 点、種 1）
RUN = ("Population", "40000", "1")
NATURAL = "#d9661f"  # natural と、店の数を固定した DD の分散（式 14）
LS = "#2a6fdb"  # Landy–Szalay と、n^2 の項だけ


def jackknife(xi: list[dict], chain: str, quantity: str, lo: np.ndarray) -> np.ndarray:
    """ビンの下端 lo ごとの jackknife の誤差。"""
    d = {float(x["r_lo_km"]): float(x["sigma"]) for x in xi if x["sample"] == chain and x["quantity"] == quantity}
    return np.array([d[x] for x in lo])


def main() -> None:
    xi = [r for r in csv.DictReader(XI.open()) if r["random"] == "population"]
    var = [r for r in csv.DictReader(VAR.open()) if (r["weight"], r["n_random"], r["seed"]) == RUN]
    fig, axes = plt.subplots(1, 2, figsize=(7.5, 3.4), sharey=True)
    for ax, chain in zip(axes, ("Coles", "Woolworths"), strict=True):
        v = [r for r in var if r["chain"] == chain]
        lo = np.array([float(r["r_lo_km"]) for r in v])
        hi = np.array([float(r["r_hi_km"]) for r in v])
        r = np.sqrt(lo * hi)
        n = float(v[0]["n"])
        gp = np.array([float(x["gp"]) for x in v])
        mean = np.array([float(x["mean_dd"]) for x in v])
        fixed = np.sqrt([float(x["var_fixed"]) for x in v]) / mean
        shot = np.sqrt(0.5 * n * (n - 1) * gp * (1 - gp)) / mean

        ax.plot(r, fixed, color=NATURAL, lw=2, label=r"$\sigma(DD)/DD$, $n$ fixed")
        ax.plot(r, shot, color=LS, lw=2, ls="--", label=r"$n^2$ term only")
        ax.plot(r, jackknife(xi, chain, "natural", lo), "o", color=NATURAL, ms=5, mec="white", mew=1, label=r"natural, jackknife")
        ax.plot(r, jackknife(xi, chain, "ls", lo), "s", color=LS, ms=5, mec="white", mew=1, label=r"Landy–Szalay, jackknife")
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlabel(r"$r$ [km]")
        ax.set_title(f"{chain} ({int(n)} stores)", fontsize=10)
        ax.grid(True, which="major", color="0.9", lw=0.6)
        for s in ("top", "right"):
            ax.spines[s].set_visible(False)
    axes[0].set_ylabel(r"$\sigma$")
    axes[0].legend(frameon=False, fontsize=7.5, loc="lower left")
    fig.tight_layout()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    for ext in ("pdf", "png"):
        fig.savefig(OUT.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

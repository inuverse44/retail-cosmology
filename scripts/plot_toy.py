"""toy model（相関なしの一様分布）で 4 つの推定量を比べる図を描く。

cargo run --release --bin toy > data/toy/poisson.csv
uv run scripts/plot_toy.py
"""

from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "data" / "toy" / "poisson.csv"
OUT = ROOT / "docs" / "figures" / "toy_poisson"

LABELS = {
    "natural": "natural",
    "davis_peebles": "Davis–Peebles",
    "hamilton": "Hamilton",
    "landy_szalay": "Landy–Szalay",
}


def main() -> None:
    d = np.genfromtxt(SRC, delimiter=",", names=True)
    fig, ax = plt.subplots(figsize=(5.5, 3.6))
    ax.axhline(0.0, color="0.6", lw=0.8)
    for key, label in LABELS.items():
        ax.plot(d["r"], d[key], marker="o", ms=3, lw=1, label=label)
    ax.set_xlabel(r"$r$")
    ax.set_ylabel(r"$\hat\xi(r)$")
    ax.set_title("toy model: Poisson points in a unit square")
    ax.legend(frameon=False, fontsize=8)
    fig.tight_layout()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    for ext in ("pdf", "png"):
        fig.savefig(OUT.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

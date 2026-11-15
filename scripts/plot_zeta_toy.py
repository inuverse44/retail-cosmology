"""三点相関関数の推定量を toy model で確かめた図（ADR 007）。

cargo run --release --bin zeta_toy
uv run scripts/plot_zeta_toy.py
"""

import csv

import figstyle as fs
import numpy as np

TOY = fs.ROOT / "data" / "toy" / "zeta_toy.csv"


def load() -> dict[str, tuple[np.ndarray, np.ndarray]]:
    """model ごとに、一回ごとの ζ と見込み。"""
    rows: dict[str, list[tuple[float, float]]] = {}
    for r in csv.DictReader(open(TOY, encoding="utf-8")):
        rows.setdefault(r["model"], []).append((float(r["zeta"]), float(r["expected"])))
    return {m: (np.array([z for z, _ in v]), np.array([e for _, e in v])) for m, v in rows.items()}


def main() -> None:
    d = load()
    fig, axes = fs.figure(1, 2, width=0.85, height=2.3)

    ax = axes[0]
    z, _ = d["uniform"]
    ax.hist(z, bins=12, color=fs.COLOR["mock"], ec="white", lw=0.4)
    ax.axvline(0.0, color=fs.COLOR["data"], lw=0.9)
    ax.set_xticks([-0.005, 0.0, 0.005])
    ax.set_xlabel(r"$\hat\zeta$")
    ax.set_ylabel(r"$N$")
    fs.panel(ax, "a", "Poisson")

    ax = axes[1]
    z, e = d["neyman_scott"]
    lim = (0.0, max(z.max(), e.max()) * 1.1)
    ax.plot(lim, lim, color="0.75", lw=0.6, zorder=0)
    ax.plot(e, z, "o", color=fs.COLOR["data"], ms=3.0, mfc="white", mew=0.7)
    ax.plot(e.mean(), z.mean(), "s", color=fs.COLOR["data"], ms=5.0, label=rf"$\langle\hat\zeta\rangle$, $N = {len(z)}$")
    ax.set(xlim=lim, ylim=lim, aspect="equal")
    ax.set_xlabel(r"$\zeta_{\mathrm{NS}}$")
    ax.set_ylabel(r"$\hat\zeta$")
    ax.legend(loc="lower right")
    fs.panel(ax, "b", "Neyman--Scott")
    fs.save(fig, "zeta_toy")


if __name__ == "__main__":
    main()

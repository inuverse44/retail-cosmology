"""ζ の符号だけを変えた三つの模型とコスモスを比べる図（ADR 010、ノートの図 fig:skewed-results）。

cargo run --release --bin zeta_vary_r_q
uv run scripts/plot_skewed_results.py
"""

import csv

import figstyle as fs
import numpy as np
from matplotlib.ticker import NullFormatter

DATA = fs.ROOT / "data" / "zeta"

# 模型の名前（Rust が表に書く名前）、凡例、色、線の形
MODELS = [
    ("$G_{10}^{+}$ (25)", r"$G_{10}^{+}$ ($\zeta > 0$)", fs.COLOR["pos_skew"], "-"),
    ("$G_{10}$ (25)", r"$G_{10}$ ($\zeta \approx 0$)", fs.COLOR["no_skew"], (0, (4, 2))),
    ("$G_{10}^{-}$ (25)", r"$G_{10}^{-}$ ($\zeta < 0$)", fs.COLOR["neg_skew"], "-"),
]
R_MIN = 35.0  # これより小さい R は Q の分母が小さく使えない（表と同じ）
R_XI = 150.0  # (b) の ξ̂_S を描く R


def rows(name: str) -> list[dict[str, str]]:
    return list(csv.DictReader(open(DATA / name, encoding="utf-8")))


def main() -> None:
    q = rows("vary_r_mocks_q.csv")
    xi = rows("vary_r_mocks_xi.csv")
    fig, axes = fs.figure(1, 2, width=1.0, height=2.5)

    # (a) Q̄_S と R。帯は模擬データ 20 個の標準偏差
    ax = axes[0]
    for key, label, color, ls in MODELS:
        sel = sorted((float(r["s_km"]), float(r["q_mean"]), float(r["q_sd"])) for r in q if r["model"] == key)
        sel = [t for t in sel if t[0] >= R_MIN]
        s, m, sd = (np.array(v) for v in zip(*sel))
        ax.fill_between(s, m - sd, m + sd, color=color, alpha=0.15, lw=0)
        ax.plot(s, m, color=color, ls=ls, label=label)
    cos = sorted({(float(r["s_km"]), float(r["cosmos_q"])) for r in q})
    cos = [t for t in cos if t[0] >= R_MIN]
    ax.plot(*zip(*cos), "o", color=fs.COLOR["data"], ms=4.0, mfc="white", mew=0.8, label="COSMOS")
    fs.ref_line(ax, 0.0)
    ax.set_xscale("log")
    ax.set_xticks([35, 50, 70, 100, 150], labels=["35", "50", "70", "100", "150"])
    ax.xaxis.set_minor_formatter(NullFormatter())
    ax.set_xlabel(r"$R$ [km]")
    ax.set_ylabel(r"$\bar Q_S$")
    ax.legend(loc="lower right")
    fs.panel(ax, "a", "")

    # (b) R = 150 km の ξ̂_S（模擬データは 20 個の平均）
    ax = axes[1]
    for key, _, color, ls in MODELS:
        sel = sorted((float(r["r_km"]), float(r["xi"])) for r in xi if r["model"] == key and float(r["s_km"]) == R_XI)
        ax.plot(*zip(*sel), color=color, ls=ls, marker="o", ms=3.0)
    sel = sorted((float(r["r_km"]), float(r["xi"])) for r in xi if r["model"] == "COSMOS" and float(r["s_km"]) == R_XI)
    ax.plot(*zip(*sel), "o", color=fs.COLOR["data"], ms=4.0, mfc="white", mew=0.8)
    ax.set_xscale("log")
    ax.set_xticks([3, 6, 12, 24], labels=["3", "6", "12", "24"])
    ax.xaxis.set_minor_formatter(NullFormatter())
    ax.set_ylim(bottom=0.0)
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(rf"$\hat\xi_S(r)$, $R = {R_XI:.0f}$ km")
    fs.panel(ax, "b", "")

    fs.save(fig, "skewed_results")


if __name__ == "__main__":
    main()

"""ζ の符号を変えた模型とコスモスを比べる図（ADR 010、ノートの図 fig:skewed-results）。

上の段は ζ の符号だけを変えた三つ、下の段は負の歪み T_- の振幅 A と場の幅 σ を変えた三組。

cargo run --release --bin zeta_vary_r_q
uv run scripts/plot_skewed_results.py
"""

import csv

import figstyle as fs
import numpy as np
from matplotlib.ticker import NullFormatter

DATA = fs.ROOT / "data" / "zeta"

# 模型の名前（Rust が表に書く名前）、凡例、色、線の形
SIGN = [
    ("$G_{10}^{+}$ (25)", r"$G_{10}^{+}$ ($\zeta > 0$)", fs.COLOR["pos_skew"], "-"),
    ("$G_{10}$ (25)", r"$G_{10}$ ($\zeta \approx 0$)", fs.COLOR["no_skew"], (0, (4, 2))),
    ("$G_{10}^{-}$ (25)", r"$G_{10}^{-}$ ($\zeta < 0$)", fs.COLOR["neg_skew"], "-"),
]
NEG = [
    ("$G_{10}^{-}$ (25)", r"$G_{10}^{-}$ ($A = 0.7$)", fs.COLOR["neg_skew"], "-"),
    ("$G_{10}^{-,0.5}$ (25)", r"$G_{10}^{-,0.5}$ ($A = 0.5$)", fs.shade(fs.COLOR["neg_skew"], 0.45), (0, (4, 2))),
    ("$G_{15}^{-}$ (25)", r"$G_{15}^{-}$ ($\sigma = 15$ km)", fs.shade(fs.COLOR["neg_skew"], -0.45), (0, (1, 1.2))),
]
R_MIN = 35.0  # これより小さい R は Q の分母が小さく使えない（表と同じ）
R_XI = 150.0  # (b)(d) の ξ̂_S を描く R


def rows(name: str) -> list[dict[str, str]]:
    return list(csv.DictReader(open(DATA / name, encoding="utf-8")))


def q_panel(ax, q, models, tag: str) -> None:
    """Q̄_S と R。線は模擬データ 20 個の平均、帯は標準偏差。"""
    for key, label, color, ls in models:
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
    fs.panel(ax, tag, "")


def xi_panel(ax, xi, models, tag: str) -> None:
    """R = R_XI の ξ̂_S。模擬データは 20 個の平均。"""
    for key, _, color, ls in models:
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
    fs.panel(ax, tag, "")


def main() -> None:
    q = rows("vary_r_mocks_q.csv")
    xi = rows("vary_r_mocks_xi.csv")
    fig, axes = fs.figure(2, 2, width=1.0, height=5.0)
    q_panel(axes[0, 0], q, SIGN, "a")
    xi_panel(axes[0, 1], xi, SIGN, "b")
    q_panel(axes[1, 0], q, NEG, "c")
    xi_panel(axes[1, 1], xi, NEG, "d")
    for col in range(2):
        lo = min(axes[0, col].get_ylim()[0], axes[1, col].get_ylim()[0])
        hi = max(axes[0, col].get_ylim()[1], axes[1, col].get_ylim()[1])
        axes[0, col].set_ylim(lo, hi)
        axes[1, col].set_ylim(lo, hi)
    fs.save(fig, "skewed_results")


if __name__ == "__main__":
    main()

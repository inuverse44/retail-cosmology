"""模擬データ 1 個の計算量のノート（notes/algorithm-cost.tex）の図を描く。

target/release/xi 2026-11-03 --country jp \
    --stores data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv \
    --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --tag costlog 2> data/cost/mock_log_R50.txt
cargo run --release --bin cost -- 50
uv run scripts/plot_cost.py

書くもの：docs/figures/cost_breakdown.pdf、cost_selection.pdf、cost_pairs_grid.pdf（と png）
"""

import re
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle, Rectangle

ROOT = Path(__file__).resolve().parents[1]
COST = ROOT / "data" / "cost"
FIG = ROOT / "docs" / "figures"

C_S = "#0f7c78"  # 選択関数 S の計算
C_PAIR = "#b55d12"  # 対の数え上げ
C_GREY = "#9aa3b0"
C_POP = "#7f9bc4"


def save(fig, name: str) -> None:
    fig.tight_layout()
    for ext in ("pdf", "png"):
        fig.savefig(FIG / f"{name}.{ext}", dpi=200)


def stage_times(log: Path) -> list[tuple[str, float, str]]:
    """xi の標準エラー出力の「（… 秒）」の時刻から、工程ごとの時間を求める。"""
    t = {}
    for line in open(log, encoding="utf-8"):
        m = re.search(r"（(\d+\.\d) 秒）\s*$", line)
        if not m:
            continue
        sec = float(m.group(1))
        if line.startswith("ランダム"):
            t["mock"] = sec
        elif line.startswith("人口：数え終えた"):
            t["auto"] = sec
        elif "選択関数" in line:
            t["select"] = sec
        elif "相互相関を数え終えた" in line:
            t["cross"] = sec
    return [
        ("(1) mock stores", t["mock"], C_S),
        ("(2) auto pairs", t["auto"] - t["mock"], C_PAIR),
        ("(3) selected randoms", t["select"] - t["auto"], C_S),
        ("(4) cross pairs", t["cross"] - t["select"], C_PAIR),
    ]


def breakdown() -> None:
    parts = stage_times(COST / "mock_log_R50.txt")
    total = sum(p[1] for p in parts)
    fig, ax = plt.subplots(figsize=(7, 1.6))
    x = 0.0
    for label, sec, color in parts:
        ax.barh(0, sec, left=x, color=color, edgecolor="white", height=0.6)
        ax.text(x + sec / 2, 0.45, label, ha="center", va="bottom", fontsize=8)
        ax.text(x + sec / 2, 0, f"{sec:.1f} s", ha="center", va="center", fontsize=9, color="white")
        x += sec
    ax.set_xlim(0, total)
    ax.set_ylim(-0.4, 0.9)
    ax.set_yticks([])
    ax.set_xlabel(f"time [s] (total {total:.1f} s, one mock, R = 50 km)")
    for s in ("left", "right", "top"):
        ax.spines[s].set_visible(False)
    save(fig, "cost_breakdown")


def selection() -> None:
    fig, axes = plt.subplots(1, 2, figsize=(9, 4.2))
    rng = np.random.default_rng(7)

    # (a) S(x) を 1 回求める：一辺 c の立方体の格子 3x3 を調べ、半径 c の円の中だけ足す
    ax = axes[0]
    c = 1.0
    # 人口に比例した参照の点：左上に人の多い所を置く
    n = 1400
    pts = np.vstack(
        [
            rng.normal([-0.9, 0.8], 0.55, size=(n * 2 // 3, 2)),
            rng.uniform(-2.2, 2.2, size=(n // 3, 2)),
        ]
    )
    x0 = np.array([0.15, 0.1])
    cell = np.floor(x0 / c)
    lo = (cell - 1) * c
    hi = (cell + 2) * c
    in_box = np.all((pts >= lo) & (pts < hi), axis=1)
    d = np.hypot(*(pts - x0).T)
    in_circ = d < c
    ax.scatter(*pts[~in_box].T, s=3, color=C_GREY, alpha=0.4, lw=0)
    ax.scatter(*pts[in_box & ~in_circ].T, s=4, color=C_POP, alpha=0.8, lw=0)
    w = np.exp(-(d[in_circ] ** 2) / (2 * (c / 3) ** 2))
    ax.scatter(*pts[in_circ].T, s=6, c=w, cmap="Greens", vmin=-0.3, vmax=1, lw=0)
    for i in range(3):
        for j in range(3):
            ax.add_patch(Rectangle(lo + np.array([i, j]) * c, c, c, fill=False, ec="0.4", lw=0.6, ls=":"))
    ax.add_patch(Rectangle(lo, 3 * c, 3 * c, fill=False, ec="0.2", lw=1.0))
    ax.add_patch(Circle(x0, c, fill=False, ec=C_S, lw=1.6))
    ax.plot(*x0, "k*", ms=10)
    ax.annotate("", xy=x0 + [c, 0], xytext=x0, arrowprops=dict(arrowstyle="<->", lw=0.8))
    ax.text(*(x0 + [c / 2, 0.06]), r"$3\sigma_R$", ha="center", fontsize=9)
    ax.set_xlim(-2.3, 2.3)
    ax.set_ylim(-2.3, 2.3)
    ax.set_aspect("equal")
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_title(r"(a) one evaluation of $S(\mathbf{x})$", fontsize=10)
    ax.text(-2.2, -2.2, "grey: not visited / blue: visited (3x3 cells)\ngreen: summed (inside the circle)", fontsize=7, va="bottom")

    # (b) 棄却法：人口に比例して候補を投げ、S に比例する確率で残す
    ax = axes[1]
    popv = np.array([1, 2, 4, 8, 9, 6, 3, 2, 1, 1, 2, 1], float)
    selv = np.array([0.2, 0.3, 0.5, 0.9, 1, 1, 0.8, 0.6, 0.3, 0.1, 0.1, 0.1])
    for i, p in enumerate(popv):
        ax.add_patch(Rectangle((i, 0), 0.96, 1, color=C_POP, alpha=0.15 + p / 12, lw=0))
    k = rng.choice(len(popv), size=80, p=popv / popv.sum())
    xs = k + 0.1 + 0.76 * rng.random(80)
    ys = 0.08 + 0.84 * rng.random(80)
    keep = rng.random(80) < selv[k]
    ax.scatter(xs[keep], ys[keep], s=18, color=C_S, label="kept")
    ax.scatter(xs[~keep], ys[~keep], s=18, color=C_GREY, marker="x", label="rejected")
    ax.plot(np.arange(len(selv)) + 0.48, 1.08 + 0.3 * selv, color=C_S, lw=1.2)
    ax.text(0, 1.45, r"$S(\mathbf{x})/S_{\max}$", color=C_S, fontsize=9)
    ax.set_xlim(-0.2, len(popv) + 0.2)
    ax.set_ylim(-0.25, 1.6)
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_title("(b) rejection sampling (shade = population)", fontsize=10)
    ax.legend(frameon=False, fontsize=8, loc="lower right", ncol=2, bbox_to_anchor=(1.0, -0.12))
    for a in axes:
        for s in a.spines.values():
            s.set_visible(False)
    save(fig, "cost_selection")


def pairs_grid() -> None:
    fig, axes = plt.subplots(1, 3, figsize=(10, 3.2), gridspec_kw={"width_ratios": [1.2, 1, 1]})
    # (a) 対の数え上げ：全部の組の距離をビンに入れる
    ax = axes[0]
    p = np.array([[0.1, 0.2], [0.35, 0.8], [0.6, 0.35], [0.85, 0.75], [0.5, 0.6]])
    for i in range(len(p)):
        for j in range(i + 1, len(p)):
            ax.plot(*p[[i, j]].T, color=C_PAIR, lw=0.8, alpha=0.5)
    ax.scatter(*p.T, color=C_PAIR, s=30, zorder=3)
    ax.set_title(r"(a) all $N(N-1)/2$ pairs $\to$ distance bins", fontsize=10)
    ax.set_aspect("equal")
    # (b)(c) 1 km 格子と 10 km 格子
    for ax, n, title in [(axes[1], 30, "(b) S on every 1 km cell"), (axes[2], 3, "(c) S on every 10 km cell")]:
        for i in range(n + 1):
            ax.plot([0, n], [i, i], color="0.6", lw=0.4 if n > 10 else 1.0)
            ax.plot([i, i], [0, n], color="0.6", lw=0.4 if n > 10 else 1.0)
        ax.set_aspect("equal")
        ax.set_title(title, fontsize=10)
    for a in axes:
        a.set_xticks([])
        a.set_yticks([])
        for s in a.spines.values():
            s.set_visible(False)
    save(fig, "cost_pairs_grid")


def main() -> None:
    FIG.mkdir(parents=True, exist_ok=True)
    breakdown()
    selection()
    pairs_grid()


if __name__ == "__main__":
    main()

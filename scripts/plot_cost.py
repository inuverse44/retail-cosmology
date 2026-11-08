"""模擬データ 1 個の計算量のノート（notes/algorithm-cost.tex）の図を描く。

target/release/xi 2026-11-03 --country jp \
    --stores data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv \
    --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --tag costlog 2> data/cost/mock_log_R50.txt
target/release/xi ...（上と同じ引数に --selection-grid on）2> data/cost/mock_log_R50_grid.txt
cargo run --release --bin cost -- 50 40000
uv run scripts/plot_cost.py

書くもの：docs/figures/cost_breakdown.pdf、cost_selection.pdf、cost_pairs_grid.pdf（と png）
"""

import re
from pathlib import Path

import figstyle as fs
import numpy as np
from matplotlib.patches import Circle, Rectangle

COST = fs.ROOT / "data" / "cost"

C_S = fs.COLOR["selection"]  # 選択関数 S の計算
C_PAIR = fs.COLOR["pairs"]  # 対の数え上げ
C_GREY = "0.75"  # 調べない点、捨てた候補
C_POP = fs.COLOR["population"]  # 人口（濃さで表す）


def bare(ax) -> None:
    """模式図のパネル：枠と目盛りを消す。"""
    ax.set_xticks([])
    ax.set_yticks([])
    ax.minorticks_off()
    for sp in ax.spines.values():
        sp.set_visible(False)


def save(fig, name: str) -> None:
    fs.save(fig, name)


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
    runs = [("exact S", COST / "mock_log_R50.txt"), ("S on 10 km cells", COST / "mock_log_R50_grid.txt")]
    runs = [(n, p) for n, p in runs if p.exists()]
    fig, ax = fs.figure(height=0.9 + 0.55 * len(runs))
    xmax = 0.0
    for row, (_name, path) in enumerate(runs):
        parts = stage_times(path)
        total = sum(p[1] for p in parts)
        xmax = max(xmax, total)
        y = len(runs) - 1 - row
        x = 0.0
        for label, sec, color in parts:
            ax.barh(y, sec, left=x, color=color, edgecolor="white", height=0.55)
            if sec > 2.5:
                ax.text(x + sec / 2, y, f"{sec:.1f}", ha="center", va="center", color="white" if color == C_S else "black")
            if row == 0:
                ax.text(x + sec / 2, y + 0.38, label, ha="center", va="bottom", fontsize=fs.SMALL)
            x += sec
        ax.text(total + 0.6, y, f"{total:.1f} s", va="center")
    ax.set_yticks(range(len(runs)))
    ax.set_yticklabels([n for n, _ in reversed(runs)])
    ax.tick_params(axis="y", which="both", left=False, right=False)
    ax.minorticks_off()
    ax.set_xlim(0, xmax * 1.12)
    ax.set_ylim(-0.5, len(runs) - 0.1)
    ax.set_xlabel(r"time [s] (one mock, $R = 50$ km)")
    save(fig, "cost_breakdown")


def selection() -> None:
    fig, axes = fs.figure(1, 2, height=2.9)
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
    ax.scatter(*pts[in_circ].T, s=6, c=w, cmap="Purples", vmin=-0.3, vmax=1, lw=0)
    for i in range(3):
        for j in range(3):
            ax.add_patch(Rectangle(lo + np.array([i, j]) * c, c, c, fill=False, ec="0.4", lw=0.6, ls=":"))
    ax.add_patch(Rectangle(lo, 3 * c, 3 * c, fill=False, ec="0.2", lw=1.0))
    ax.add_patch(Circle(x0, c, fill=False, ec=C_S, lw=1.6))
    ax.plot(*x0, "k*", ms=10)
    ax.annotate("", xy=x0 + [c, 0], xytext=x0, arrowprops=dict(arrowstyle="<->", lw=0.8))
    ax.text(*(x0 + [c / 2, 0.12]), r"$3\sigma_R$", ha="center", va="bottom")
    ax.set_xlim(-2.3, 2.3)
    ax.set_ylim(-2.3, 2.3)
    ax.set_aspect("equal")
    fs.panel(ax, "a", r"one evaluation of $S(\bm{x})$")

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
    ax.text(0, 1.45, r"$S(\bm{x})/S_{\max}$", color=C_S)
    ax.set_xlim(-0.2, len(popv) + 0.2)
    ax.set_ylim(-0.25, 1.6)
    fs.panel(ax, "b", "rejection sampling")
    ax.legend(loc="lower right", ncol=2, bbox_to_anchor=(1.0, -0.12))
    for a in axes:
        bare(a)
    save(fig, "cost_selection")


def pairs_grid() -> None:
    fig, axes = fs.figure(1, 3, height=2.2, gridspec_kw={"width_ratios": [1.2, 1, 1]})
    # (a) 対の数え上げ：全部の組の距離をビンに入れる
    ax = axes[0]
    p = np.array([[0.1, 0.2], [0.35, 0.8], [0.6, 0.35], [0.85, 0.75], [0.5, 0.6]])
    for i in range(len(p)):
        for j in range(i + 1, len(p)):
            ax.plot(*p[[i, j]].T, color=C_PAIR, lw=0.8, alpha=0.5)
    ax.scatter(*p.T, color=C_PAIR, s=30, zorder=3)
    fs.panel(ax, "a", r"all $N(N-1)/2$ pairs")
    ax.set_aspect("equal")
    # (b)(c) 1 km 格子と 10 km 格子
    for ax, tag, n, name in [(axes[1], "b", 30, "1 km cells"), (axes[2], "c", 3, "10 km cells")]:
        for i in range(n + 1):
            ax.plot([0, n], [i, i], color="0.6", lw=0.4 if n > 10 else 1.0)
            ax.plot([i, i], [0, n], color="0.6", lw=0.4 if n > 10 else 1.0)
        ax.set_aspect("equal")
        fs.panel(ax, tag, name)
    for a in axes:
        bare(a)
    save(fig, "cost_pairs_grid")


def main() -> None:
    breakdown()
    selection()
    pairs_grid()


if __name__ == "__main__":
    main()

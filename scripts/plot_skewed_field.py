"""ζ の符号を選べる密度場の模型（ADR 010）を説明する図。

cargo run --release --bin field_transform_demo
uv run scripts/plot_skewed_field.py
"""

import csv

import figstyle as fs
import matplotlib.pyplot as plt
import numpy as np

DATA = fs.ROOT / "data" / "zeta"

# 変換ごとの列の名前、凡例、色、線の形
TRANSFORMS = [
    ("none", r"$T = g$", fs.COLOR["no_skew"], (0, (4, 2))),
    ("chi", r"$T = (g^2 - 1)/\sqrt{2}$", fs.COLOR["pos_skew"], "-"),
    ("negchi", r"$T = (1 - g^2)/\sqrt{2}$", fs.COLOR["neg_skew"], "-"),
]


def load(name: str) -> dict[str, np.ndarray]:
    rows = list(csv.DictReader(open(DATA / f"field_transform_{name}.csv", encoding="utf-8")))
    return {k: np.array([float(r[k]) for r in rows]) for k in rows[0]}


def main() -> None:
    line, pdf, ex = load("line"), load("pdf"), load("expect")
    fig = plt.figure(figsize=(fs.TEXTWIDTH, 4.3), layout="constrained")
    axes = fig.subplot_mosaic([["a", "a", "a"], ["b", "c", "d"]], height_ratios=[1.0, 1.0])

    # (a) 同じ g から作った三つの重み max(0, 1 + δ)
    ax = axes["a"]
    for key, label, color, ls in TRANSFORMS:
        w = np.maximum(0.0, 1.0 + line[f"delta_{key}"])
        ax.plot(line["x_km"], w, color=color, ls=ls, lw=0.9, label=label)
    fs.ref_line(ax, 1.0)
    ax.set_xlim(0, 300)
    ax.set_ylim(0.0, 3.9)  # 凡例を山の上に逃がす
    ax.set_xlabel(r"$x$ [km]")
    ax.set_ylabel(r"$\max(0, 1 + A\,T(g))$")
    ax.legend(loc="upper right", ncol=3)
    fs.panel(ax, "a", "")

    # (b) 1 + δ の確率密度（切る前）
    ax = axes["b"]
    x = 1.0 + pdf["delta"]
    for key, label, color, ls in TRANSFORMS:
        ax.plot(x, pdf[f"p_{key}"], color=color, ls=ls, lw=0.9)
    ax.axvline(0.0, color="0.75", lw=0.6, zorder=0)
    ax.set_xlim(-0.5, 3.0)
    ax.set_ylim(0.0, 1.6)
    ax.set_xlabel(r"$1 + \delta$")
    ax.set_ylabel(r"$p(1 + \delta)$")
    fs.panel(ax, "b", "")

    # (c) 二点相関関数：正と負の歪みで同じ
    ax = axes["c"]
    ax.plot(ex["r_km"], ex["xi_none"], color=fs.COLOR["no_skew"], ls=(0, (4, 2)), lw=0.9)
    ax.plot(ex["r_km"], ex["xi_skew"], color="0.1", lw=0.9, label=r"$T = \pm(g^2 - 1)/\sqrt{2}$")
    ax.set_xlim(0, 40)
    ax.set_ylim(0.0, 0.68)  # 凡例を曲線の上に逃がす
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\xi(r)$")
    ax.legend(loc="upper right")
    fs.panel(ax, "c", "")

    # (d) 三辺 r の正三角形の三点相関関数：符号だけが逆
    ax = axes["d"]
    for key, _, color, ls in TRANSFORMS:
        ax.plot(ex["r_km"], ex[f"zeta_eq_{key}"], color=color, ls=ls, lw=0.9)
    fs.ref_line(ax, 0.0)
    ax.set_xlim(0, 40)
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\zeta(r, r, r)$")
    fs.panel(ax, "d", "")

    fs.save(fig, "skewed_field")


if __name__ == "__main__":
    main()

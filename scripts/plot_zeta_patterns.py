"""三点相関関数で何がわかるかを直感で示す図：丸い塊、一列の並び、大きな店のまわりの衛星。

三つとも、近くに店の組が多い（二点相関関数は正になる）が、近い三点が作る三角形の形が違う。
説明のための絵で、ζ は測っていない（点の位置は乱数の種を固定して作る）。

uv run scripts/plot_zeta_patterns.py
"""

import figstyle as fs
import matplotlib.patches as mpatches
import numpy as np


def clumps(rng: np.random.Generator) -> np.ndarray:
    """親の点のまわりに子の点を正規分布で置く（丸い塊）。"""
    parents = rng.uniform(0.15, 0.85, size=(8, 2))
    return np.concatenate([p + 0.035 * rng.standard_normal((14, 2)) for p in parents])


def lines(rng: np.random.Generator) -> np.ndarray:
    """何本かの線（街道や鉄道）に沿って、少しずらして置く（一列の並び）。"""
    out = []
    for _ in range(4):
        a, b = rng.uniform(0.05, 0.95, size=(2, 2))
        t = rng.uniform(size=28)[:, None]
        out.append(a + t * (b - a) + 0.008 * rng.standard_normal((28, 2)))
    return np.concatenate(out)


def satellites(rng: np.random.Generator) -> tuple[np.ndarray, list[np.ndarray]]:
    """大きな店のまわりの輪の上に、小さな店を置く（衛星）。大きな店と、店ごとの衛星を返す。"""
    hubs = np.array([[0.25, 0.72], [0.72, 0.75], [0.3, 0.27], [0.75, 0.3]]) + 0.04 * rng.standard_normal((4, 2))
    sats = []
    for h in hubs:
        k = rng.integers(4, 8)
        ang = rng.uniform(0, 2 * np.pi, size=k)
        rad = 0.13 + 0.012 * rng.standard_normal(k)
        sats.append(h + rad[:, None] * np.stack([np.cos(ang), np.sin(ang)], 1))
    return hubs, sats


def triangle(ax, pts: np.ndarray, marks: np.ndarray | None = None) -> None:
    """三点を結ぶ三角形を描き、頂点（`marks` があればその点だけ）に白抜きの丸をつける。"""
    ax.add_patch(mpatches.Polygon(pts, closed=True, fill=False, ec="0.35", lw=1.2, zorder=3))
    m = pts if marks is None else marks
    ax.plot(m[:, 0], m[:, 1], "o", ms=5.0, mfc="white", mec=fs.COLOR["data"], mew=0.9, zorder=4)


def nearest(pts: np.ndarray, c: np.ndarray, k: int) -> np.ndarray:
    """点 c にいちばん近い k 点。"""
    return pts[np.argsort(((pts - c) ** 2).sum(1))[:k]]


def main() -> None:
    rng = np.random.default_rng(20261114)
    fig, axes = fs.figure(1, 3, width=1.0, height=2.25)

    ax = axes[0]
    pts = clumps(rng)
    ax.plot(pts[:, 0], pts[:, 1], "o", ms=1.8, color=fs.COLOR["data"], mew=0)
    # 同じ塊の中で、塊の大きさくらい離れた三点（近い 14 点のうち 1、7、13 番目）
    near = nearest(pts, pts[0], 14)
    triangle(ax, near[[0, 6, 13]])
    fs.panel(ax, "a")

    ax = axes[1]
    pts = lines(rng)
    ax.plot(pts[:, 0], pts[:, 1], "o", ms=1.8, color=fs.COLOR["data"], mew=0)
    # いちばん長い線の上の、端寄り・真ん中・反対の端寄りの三点（細長い三角形になる）
    segs = [pts[28 * i : 28 * (i + 1)] for i in range(4)]
    seg = max(segs, key=lambda s: np.ptp(s, axis=0).sum())
    axis = np.argmax(np.ptp(seg, axis=0))
    order = np.argsort(seg[:, axis])
    triangle(ax, seg[order][[3, 14, 25]])
    fs.panel(ax, "b")

    ax = axes[2]
    hubs, sats = satellites(rng)
    allsats = np.concatenate(sats)
    ax.plot(allsats[:, 0], allsats[:, 1], "o", ms=1.8, color=fs.COLOR["data"], mew=0)
    ax.plot(hubs[:, 0], hubs[:, 1], "s", ms=4.0, mfc="white", mec=fs.COLOR["data"], mew=0.8, zorder=5)
    # 衛星のいちばん多い大きな店と、その衛星のうち大きな店から見た角度が直角にいちばん近い二つ
    # （二辺が輪の半径の二等辺三角形になる）
    h = max(range(len(hubs)), key=lambda i: len(sats[i]))
    ring = sats[h]
    ang = np.arctan2(ring[:, 1] - hubs[h][1], ring[:, 0] - hubs[h][0])
    pairs = [(a, b) for a in range(len(ring)) for b in range(a + 1, len(ring))]
    a, b = min(pairs, key=lambda p: abs(abs(np.angle(np.exp(1j * (ang[p[0]] - ang[p[1]])))) - np.pi / 2))
    triangle(ax, np.vstack([hubs[h], ring[a], ring[b]]), marks=ring[[a, b]])
    fs.panel(ax, "c")

    for ax in axes:
        ax.set(xlim=(0, 1), ylim=(0, 1), aspect="equal", xticks=[], yticks=[])
        ax.minorticks_off()
    fs.save(fig, "zeta_patterns")


if __name__ == "__main__":
    main()

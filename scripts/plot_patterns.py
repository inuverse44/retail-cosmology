"""二点相関関数の符号を直感で示す図：固まる点、ポアソン点、離れて並ぶ点。

説明のための絵で、ξ は測っていない（点の位置は乱数の種を固定して作る）。

uv run scripts/plot_patterns.py
"""

import figstyle as fs
import matplotlib.patches as mpatches
import numpy as np

N = 200  # 各パネルの点の数（目安）
R = 0.1  # 輪の半径（単位正方形の辺を 1 とする）


def clustered(rng: np.random.Generator) -> np.ndarray:
    """親の点のまわりに子の点を正規分布で置く（Thomas 過程）。はみ出した点は反対側へ回す。"""
    parents = rng.uniform(size=(15, 2))
    kids = [p + 0.03 * rng.standard_normal((rng.poisson(N / 15), 2)) for p in parents]
    return np.concatenate(kids) % 1.0


def poisson(rng: np.random.Generator) -> np.ndarray:
    """互いに独立に、一様に置く。"""
    return rng.uniform(size=(rng.poisson(N), 2))


def regular(rng: np.random.Generator) -> np.ndarray:
    """格子の点を少しずつずらす（近すぎる組ができにくい）。"""
    k = int(round(np.sqrt(N)))
    g = (np.stack(np.meshgrid(np.arange(k), np.arange(k)), -1).reshape(-1, 2) + 0.5) / k
    return (g + rng.uniform(-0.2, 0.2, size=g.shape) / k) % 1.0


def pick(pts: np.ndarray, densest: bool) -> np.ndarray:
    """輪を描く点。densest なら中央の 0.3--0.7 の範囲で輪の中のほかの点がいちばん多い点（固まりの中の点）、
    そうでなければ中央にいちばん近い点。"""
    if not densest:
        return pts[np.argmin(((pts - 0.5) ** 2).sum(1))]
    inner = pts[(np.abs(pts - 0.5) < 0.2).all(1)]
    d2 = ((inner[:, None, :] - pts[None, :, :]) ** 2).sum(-1)
    return inner[np.argmax((d2 < R**2).sum(1))]


def main() -> None:
    rng = np.random.default_rng(20261110)
    cases = [
        ("a", r"$\xi>0$", clustered(rng)),
        ("b", r"$\xi=0$", poisson(rng)),
        ("c", r"$\xi<0$", regular(rng)),
    ]
    fig, axes = fs.figure(1, 3, width=1.0, height=2.25)
    for ax, (tag, name, pts) in zip(axes, cases, strict=True):
        c = pick(pts, densest=(tag == "a"))
        ax.add_patch(mpatches.Circle(c, R, fill=False, ec="0.45", lw=0.8, ls="--"))
        ax.plot(pts[:, 0], pts[:, 1], "o", ms=1.8, color=fs.COLOR["data"], mew=0)
        ax.plot(*c, "o", ms=4.0, mfc="white", mec=fs.COLOR["data"], mew=0.8)
        ax.set(xlim=(0, 1), ylim=(0, 1), aspect="equal", xticks=[], yticks=[])
        ax.minorticks_off()
        fs.panel(ax, tag, name)
    fs.save(fig, "point_patterns")


if __name__ == "__main__":
    main()

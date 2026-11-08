"""toy model（相関なしの一様分布）で 4 つの推定量を比べる図を描く。

cargo run --release --bin toy > data/toy/poisson.csv
uv run scripts/plot_toy.py
"""

import figstyle as fs
import numpy as np

SRC = fs.ROOT / "data" / "toy" / "poisson.csv"

LABELS = {
    "natural": "natural",
    "davis_peebles": "Davis--Peebles",
    "hamilton": "Hamilton",
    "landy_szalay": "Landy--Szalay",
}


def main() -> None:
    d = np.genfromtxt(SRC, delimiter=",", names=True)
    fig, ax = fs.figure(width=0.8, height=2.5)
    fs.ref_line(ax, 0)
    for key, label in LABELS.items():
        ax.plot(d["r"], d[key], lw=0.8, label=label, **fs.ESTIMATOR[key])
    ax.set_xlabel(r"$r$")
    ax.set_ylabel(r"$\hat\xi(r)$")
    ax.legend(ncol=4, loc="upper center", columnspacing=1.0)
    ax.set_ylim(top=ax.get_ylim()[1] * 1.4)  # 凡例の場所を空ける
    fs.save(fig, "toy_poisson")


if __name__ == "__main__":
    main()

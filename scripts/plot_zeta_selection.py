"""店から作った選択関数が、二点で固まった点の三点相関関数をどれだけずらすかを toy model で測った図（#81）。

Neyman--Scott の模型（親 350、子 5、標準偏差 5 km、約 330 km 四方）に、一様なランダムカタログと、
点から作った幅 50 km の選択関数のランダムカタログを当てた 40 回の平均。

cargo run --release --bin zeta_selection_toy -- 40
uv run scripts/plot_zeta_selection.py
"""

import csv

import figstyle as fs
import numpy as np

SUMMARY = fs.ROOT / "data" / "toy" / "zeta_selection_bias_summary.csv"


def main() -> None:
    rows = list(csv.DictReader(open(SUMMARY, encoding="utf-8")))
    runs = int(rows[0]["runs"])
    x = np.arange(len(rows))
    labels = [",".join(f"{float(r[c]):.0f}" for c in ("r1_km", "r2_km", "r3_km")) for r in rows]
    col = {k: np.array([float(r[k]) for r in rows]) for k in rows[0] if k not in ("k1", "k2", "k3")}
    se = np.sqrt(runs)

    fig, axes = fs.figure(2, 1, height=4.4, sharex=True)
    ax = axes[0]
    fs.ref_line(ax, 0)
    ax.plot(x, col["expected_mean"], "_", color=fs.COLOR["data"], ms=10, mew=1.2, label=r"$\zeta_{\mathrm{NS}}$")
    ax.errorbar(
        x - 0.12,
        col["uniform_mean"],
        yerr=col["uniform_sd"] / se,
        fmt="o",
        color="0.45",
        mfc="white",
        label=r"$\hat\zeta_U$",
    )
    ax.errorbar(
        x + 0.12,
        col["selection_mean"],
        yerr=col["selection_sd"] / se,
        fmt="s",
        color=fs.COLOR["selection"],
        label=r"$\hat\zeta_S$, $R = 50$ km",
    )
    ax.set_yscale("symlog", linthresh=0.05)
    ax.set_ylabel(rf"$\langle\hat\zeta\rangle$, $N = {runs}$")
    ax.legend(loc="upper right")
    fs.panel(ax, "a")

    ax = axes[1]
    fs.ref_line(ax, 0)
    ax.errorbar(x, col["shift_mean"], yerr=col["shift_err"], fmt="s", color=fs.COLOR["selection"])
    ax.set_ylabel(r"$\langle\hat\zeta_S - \hat\zeta_U\rangle$")
    fs.panel(ax, "b")
    ax.set_xticks(x, labels, rotation=60, ha="right")
    ax.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax.set_xlabel(r"$(r_1, r_2, r_3)$ [km]")
    fs.save(fig, "zeta_selection")


if __name__ == "__main__":
    main()

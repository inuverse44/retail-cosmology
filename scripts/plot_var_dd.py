"""オーストラリアの店について、ξ の jackknife の誤差と、DD の分散の式（ノートの付録「DD の分散」）を比べる図を描く。

cargo run --release --bin xi -- 2026-11-01
cargo run --release --bin var_dd -- 2026-11-01
uv run scripts/plot_var_dd.py
"""

import csv

import figstyle as fs
import numpy as np

DATE = "2026-11-01"
XI = fs.ROOT / "data" / "xi" / f"xi_{DATE}.csv"
VAR = fs.ROOT / "data" / "var" / f"var_dd_au_{DATE}.csv"
# 表と同じランダムカタログ（人口に比例、40000 点、種 1）
RUN = ("Population", "40000", "1")


def jackknife(xi: list[dict], chain: str, quantity: str, lo: np.ndarray) -> np.ndarray:
    """ビンの下端 lo ごとの jackknife の誤差。"""
    d = {float(x["r_lo_km"]): float(x["sigma"]) for x in xi if x["sample"] == chain and x["quantity"] == quantity}
    return np.array([d[x] for x in lo])


def main() -> None:
    xi = [r for r in csv.DictReader(XI.open()) if r["random"] == "population"]
    var = [r for r in csv.DictReader(VAR.open()) if (r["weight"], r["n_random"], r["seed"]) == RUN]
    fig, axes = fs.figure(1, 2, height=2.5, sharey=True)
    for ax, tag, chain in zip(axes, "ab", ("Coles", "Woolworths"), strict=True):
        v = [r for r in var if r["chain"] == chain]
        lo = np.array([float(r["r_lo_km"]) for r in v])
        hi = np.array([float(r["r_hi_km"]) for r in v])
        r = np.sqrt(lo * hi)
        n = float(v[0]["n"])
        gp = np.array([float(x["gp"]) for x in v])
        mean = np.array([float(x["mean_dd"]) for x in v])
        fixed = np.sqrt([float(x["var_fixed"]) for x in v]) / mean
        shot = np.sqrt(0.5 * n * (n - 1) * gp * (1 - gp)) / mean

        ax.plot(r, fixed, color="0.1", lw=1.2, label=r"$\sigma(DD)/DD$, $n$ fixed")
        ax.plot(r, shot, color="0.1", lw=1.2, ls="--", label=r"$n^2$ term only")
        ax.plot(r, jackknife(xi, chain, "natural", lo), ls="none", label="natural, jackknife", **fs.ESTIMATOR["natural"])
        ax.plot(r, jackknife(xi, chain, "ls", lo), ls="none", label="Landy--Szalay, jackknife", **fs.ESTIMATOR["landy_szalay"])
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlabel(r"$r$ [km]")
        fs.panel(ax, tag, f"{chain} ({int(n)} stores)")
    axes[0].set_ylabel(r"$\sigma$")
    fig.legend(*axes[0].get_legend_handles_labels(), loc="outside upper center", ncol=4, columnspacing=1.0)
    fs.save(fig, "var_dd_au")


if __name__ == "__main__":
    main()

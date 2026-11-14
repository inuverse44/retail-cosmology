"""二点の固まりをコスモスにそろえた模擬データ（Neyman--Scott の模型）と、コスモスの三点相関関数を比べた図（#81）。

模擬データも実際の店と同じ手順（模擬データそのものから選択関数を作り直す）で測った値を使う。

C=data/cosmos_site/cosmos_site_clean_2026-11-12.csv
cargo run --release --bin zeta_clustered -- 2026-11-12 --country jp --stores $C --chain COSMOS --footprint 50 \
  --n-random 8000 --children M --sigma S --runs 20
cargo run --release --bin zeta_clustered_summary -- data/zeta/clustered_jp_2026-11-12_cosmos_mM_sS \
  data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv
uv run scripts/plot_zeta_clustered.py clustered_jp_2026-11-12_cosmos_mM_sS
"""

import csv
import sys

import figstyle as fs
import numpy as np

ZETA = fs.ROOT / "data" / "zeta"


def load(path) -> dict[str, np.ndarray]:
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    return {k: np.array([float(r[k]) for r in rows]) for k in rows[0]}


def main() -> None:
    stem = ZETA / sys.argv[1]
    xi = load(f"{stem}_xi_summary.csv")
    z = load(f"{stem}_summary.csv")
    runs = int(z["runs"][0])

    fig = fs.plt.figure(figsize=(fs.TEXTWIDTH, 2.9), layout="constrained")
    gs = fig.add_gridspec(1, 4)
    ax = fig.add_subplot(gs[0, 0])
    fs.ref_line(ax, 0)
    ax.errorbar(
        xi["r_km"] * 1.05,
        xi["mock_proc_mean"],
        yerr=xi["mock_proc_sd"],
        fmt="s",
        color=fs.COLOR["selection"],
        label="模擬データ",
    )
    ax.plot(xi["r_km"], xi["real"], fs.MARKER["COSMOS"], color=fs.COLOR["COSMOS"], label="COSMOS")
    ax.set_xscale("log")
    ax.set_xticks([3, 6, 12, 24], ["3", "6", "12", "24"])
    ax.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    ax.legend(loc="upper right")
    fs.panel(ax, "a")

    ax = fig.add_subplot(gs[0, 1:])
    fs.ref_line(ax, 0)
    x = np.arange(len(z["real"]))
    labels = [f"{a:.0f},{b:.0f},{c:.0f}" for a, b, c in zip(z["r1_km"], z["r2_km"], z["r3_km"], strict=True)]
    ax.plot(x, z["expected_mean"], "_", color=fs.COLOR["data"], ms=9, mew=1.2, label="見込み（模擬データ）")
    ax.errorbar(
        x,
        z["proc_mean"],
        yerr=z["proc_sd"],
        fmt="s",
        color=fs.COLOR["selection"],
        label=rf"模擬データ {runs} 個（平均 $\pm 1\sigma$）",
    )
    ax.plot(x, z["real"], fs.MARKER["COSMOS"], color=fs.COLOR["COSMOS"], ms=5, label="COSMOS")
    ax.set_xticks(x, labels, rotation=60, ha="right")
    ax.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax.set_xlabel("三角形の三辺 [km]")
    ax.set_ylabel(r"$\hat\zeta$")
    ax.legend(loc="upper right")
    fs.panel(ax, "b")
    fs.save(fig, "zeta_clustered")


if __name__ == "__main__":
    main()

"""二点の固まりをコスモスにそろえた模擬データとコスモスの三点相関関数を比べた図（#81）。

模擬データは三つの模型：親のまわりの丸い塊（Neyman--Scott、ζ > 0）、ガウス場で密度を揺らす模型（幅 10 km、ζ ≈ 0）、
二つの幅（10 km と 20 km）のガウス場で 24 km の ξ までそろえた模型（0 で切る点が多く、ζ > 0）。
どれも実際の店と同じ手順（模擬データそのものから選択関数を作り直す）で測った値を使う。

C=data/cosmos_site/cosmos_site_clean_2026-11-12.csv
cargo run --release --bin zeta_clustered -- 2026-11-12 --country jp --stores $C --chain COSMOS --footprint 50 \
  --n-random 8000 --children 14 --sigma 15 --runs 20
cargo run --release --bin zeta_field -- 2026-11-12 --country jp --stores $C --chain COSMOS --footprint 50 \
  --n-random 8000 --amp 0.7 --sigma 10 --centers 20000 --runs 20
R=data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv
cargo run --release --bin zeta_clustered_summary -- data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15 $R
cargo run --release --bin zeta_clustered_summary -- data/zeta/field_jp_2026-11-12_cosmos_a0.7_s10 $R
# 二つの幅の模型（G、100 個）：docs/field-mocks-81.md の手順で field_two_scale_m81_20261114_mainG_* を作る
uv run scripts/plot_zeta_clustered.py
"""

import csv

import figstyle as fs
import numpy as np

ZETA = fs.ROOT / "data" / "zeta"
# (ファイル名の前半, 凡例, 色, マーカー, 横のずらし)
MODELS = [
    ("clustered_jp_2026-11-12_cosmos_m14_s15", "丸い塊", fs.COLOR["selection"], "s", -0.22),
    ("field_jp_2026-11-12_cosmos_a0.7_s10", "密度の揺らぎ", "0.45", "o", 0.0),
    ("field_two_scale_m81_20261114_mainG", "二つの幅の揺らぎ", "0.1", "^", 0.22),
]


def load(path) -> dict[str, np.ndarray]:
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    return {k: np.array([float(r[k]) for r in rows]) for k in rows[0]}


def main() -> None:
    fig = fs.plt.figure(figsize=(fs.TEXTWIDTH, 3.4), layout="constrained")
    gs = fig.add_gridspec(1, 4)
    ax_xi = fig.add_subplot(gs[0, 0])
    ax_z = fig.add_subplot(gs[0, 1:])
    fs.ref_line(ax_xi, 0)
    fs.ref_line(ax_z, 0)

    for stem, label, color, marker, dx in MODELS:
        xi = load(ZETA / f"{stem}_xi_summary.csv")
        z = load(ZETA / f"{stem}_summary.csv")
        runs = int(z["runs"][0])
        ax_xi.errorbar(
            xi["r_km"] * (1 + dx / 2),
            xi["mock_proc_mean"],
            yerr=xi["mock_proc_sd"],
            fmt=marker,
            color=color,
            mfc="white" if marker == "o" else color,
            label=label,
        )
        x = np.arange(len(z["real"]))
        ax_z.errorbar(
            x + dx,
            z["proc_mean"],
            yerr=z["proc_sd"],
            fmt=marker,
            color=color,
            mfc="white" if marker == "o" else color,
            label=rf"{label}（{runs} 個、平均 $\pm 1\sigma$）",
        )
        if stem.startswith("clustered"):
            ax_z.plot(x + dx, z["expected_mean"], "_", color=color, ms=8, mew=1.2, label="丸い塊の見込み")

    ax_xi.plot(xi["r_km"], xi["real"], fs.MARKER["COSMOS"], color=fs.COLOR["COSMOS"], label="COSMOS")
    ax_xi.set_xscale("log")
    ax_xi.set_xticks([3, 6, 12, 24], ["3", "6", "12", "24"])
    ax_xi.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax_xi.set_xlabel(r"$r$ [km]")
    ax_xi.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax_xi, "a")

    labels = [f"{a:.0f},{b:.0f},{c:.0f}" for a, b, c in zip(z["r1_km"], z["r2_km"], z["r3_km"], strict=True)]
    ax_z.plot(x, z["real"], fs.MARKER["COSMOS"], color=fs.COLOR["COSMOS"], ms=5, label="COSMOS")
    ax_z.set_xticks(x, labels, rotation=60, ha="right")
    ax_z.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax_z.set_xlabel("三角形の三辺 [km]")
    ax_z.set_ylabel(r"$\hat\zeta$")
    # 凡例は (a)(b) で共通なので、図の上に一つだけ置く（点に重ならないように）
    fig.legend(*ax_z.get_legend_handles_labels(), loc="outside upper center", ncols=3)
    fs.panel(ax_z, "b")
    fs.save(fig, "zeta_clustered")


if __name__ == "__main__":
    main()

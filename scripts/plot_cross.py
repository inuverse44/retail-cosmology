"""西友の二点相関関数と、トライアルと西友の相互相関関数を描く。帰無仮説の模擬データの分布と比べる。

S=data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv
cargo run --release --bin xi -- 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50,100,200 --tag crossfp
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6 density
for R in 50 100; do for K in TRIAL SEIYU; do
  scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU $R 100 6 density,grid,keep:$K
done; done
uv run scripts/plot_cross.py 2026-11-03
"""

import re
import sys
from pathlib import Path

import figstyle as fs
from plot_footprint import load, r_series

ROOT = fs.ROOT
XI = ROOT / "data" / "xi"
CROSS = "TRIALxSEIYU"


def summaries(path: Path, key: str) -> list[float]:
    """`key` を含む「まとめた値」の行から値を取り出す（模擬データなら種ごとに一つ）。"""
    out = []
    for line in open(path, encoding="utf-8"):
        if key in line and "まとめた値" in line:
            out.append(float(re.search(r"まとめた値 (-?[0-9.]+) ±", line).group(1)))
    return out


# 模擬データの種類：(ファイル名の接尾辞, 凡例, 体裁)
NULLS = [
    ("", "both redrawn (region)", {"color": fs.COLOR["mock"]}),
    ("_density", "both redrawn (region + density)", {"color": "0.3", "ls": "--"}),
    ("_density_grid_keepTRIAL", "TRIAL kept, SEIYU redrawn", {"color": fs.COLOR["TRIAL"]}),
    ("_density_grid_keepSEIYU", "SEIYU kept, TRIAL redrawn", {"color": fs.COLOR["SEIYU"]}),
]


def main() -> None:
    date = sys.argv[1]
    d = load(XI / f"xi_jp_{date}_crossfp.csv")
    real_txt = XI / f"chi2_jp_{date}_crossfp.txt"
    fig, axes = fs.figure(2, 2, height=4.6)

    for ax, tag, q, s, entity, name in [
        (axes[0, 0], "a", "ls", "SEIYU", "SEIYU", "SEIYU"),
        (axes[0, 1], "b", "ls_cross", CROSS, "cross", r"TRIAL $\times$ SEIYU"),
    ]:
        fs.ref_line(ax, 0)
        r_series(ax, d, q, s, entity)
        ax.set_xscale("log")
        ax.set_xlabel(r"$r$ [km]")
        ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$" if q == "ls" else r"$\hat\xi_{\mathrm{LS}}^{\times}$")
        fs.panel(ax, tag, name)
        if q == "ls":
            ax.legend(loc="upper right")
        else:
            ax.set_ylim(bottom=-1.9)  # 凡例の場所を空ける
            ax.legend(loc="lower right", ncol=2, columnspacing=0.8)

    key_cross = f"{CROSS} の相互相関 ξ_LS"
    for ax, r_km, label, tag in [(axes[1, 0], 50, "8--20 km", "c"), (axes[1, 1], 100, "8--32 km", "d")]:
        key = f"{key_cross}（footprint_{r_km}）"
        real = summaries(real_txt, key)[0]
        for suffix, name, style in NULLS:
            path = XI / f"null_mocks_cross_{CROSS}_{r_km}{suffix}.txt"
            if not path.exists():
                continue
            m = summaries(path, key)
            ax.hist(m, bins=20, histtype="step", lw=1.2, label=f"null mocks: {name}", **style)
        ax.axvline(real, color=fs.COLOR["data"], lw=1.5, label="data")
        ax.set_xlabel(rf"$\hat\xi_{{\mathrm{{LS}}}}^{{\times}}$ ({label})")
        ax.set_ylabel("count")
        fs.panel(ax, tag, rf"$R = {r_km}$ km")

    fig.legend(*axes[1, 0].get_legend_handles_labels(), loc="outside lower center", ncol=2)
    fs.save(fig, "xi_cross")


if __name__ == "__main__":
    main()

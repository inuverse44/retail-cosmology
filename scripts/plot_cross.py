"""西友の二点相関関数と、トライアルと西友の相互相関関数を描く。帰無仮説の模擬データの分布と比べる。

S=data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv
cargo run --release --bin xi -- 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50,100,200 --tag crossfp
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6 density
uv run scripts/plot_cross.py 2026-11-03
"""

import re
import sys
from pathlib import Path

import matplotlib.pyplot as plt

from plot_footprint import load

ROOT = Path(__file__).resolve().parents[1]
XI = ROOT / "data" / "xi"
CROSS = "TRIALxSEIYU"


def summaries(path: Path, key: str) -> list[float]:
    """`key` を含む「まとめた値」の行から値を取り出す（模擬データなら種ごとに一つ）。"""
    out = []
    for line in open(path, encoding="utf-8"):
        if key in line and "まとめた値" in line:
            out.append(float(re.search(r"まとめた値 (-?[0-9.]+) ±", line).group(1)))
    return out


def main() -> None:
    date = sys.argv[1]
    d = load(XI / f"xi_jp_{date}_crossfp.csv")
    real_txt = XI / f"chi2_jp_{date}_crossfp.txt"
    styles = [
        ("population", "population only", "0.5", 1.0),
        ("footprint_50", "R = 50 km", "#1f77b4", 0.97),
        ("footprint_100", "R = 100 km", "#ff7f0e", 1.0),
        ("footprint_200", "R = 200 km", "#2ca02c", 1.03),
    ]
    fig, axes = plt.subplots(2, 2, figsize=(10, 7.5))

    for ax, q, s, title in [
        (axes[0, 0], "ls", "SEIYU", "(a) SEIYU"),
        (axes[0, 1], "ls_cross", CROSS, "(b) TRIAL x SEIYU (cross)"),
    ]:
        ax.axhline(0, color="0.6", lw=0.8)
        for rnd, label, color, shift in styles:
            r, v, e = d[(q, s, rnd)]
            ok = r < 200
            ax.errorbar(r[ok] * shift, v[ok], yerr=e[ok], color=color, marker="o", ms=3, lw=1, capsize=2, label=label)
        ax.set_xscale("log")
        ax.set_xlabel(r"$r$ [km]")
        ax.set_ylabel(r"$\hat\xi_{\rm LS}$" if q == "ls" else r"$\hat\xi_{\rm LS}^{\times}$")
        ax.set_title(title)
        ax.legend(frameon=False, fontsize=8)

    key_cross = f"{CROSS} の相互相関 ξ_LS"
    for ax, r_km, label, tag in [(axes[1, 0], 50, "8–20 km", "c"), (axes[1, 1], 100, "8–32 km", "d")]:
        key = f"{key_cross}（footprint_{r_km}）"
        real = summaries(real_txt, key)[0]
        for suffix, color, name in [("", "0.6", "region only"), ("_density", "#9467bd", "region + density")]:
            path = XI / f"null_mocks_cross_{CROSS}_{r_km}{suffix}.txt"
            if not path.exists():
                continue
            m = summaries(path, key)
            ax.hist(m, bins=20, histtype="step", color=color, lw=1.5, label=f"null mocks, {name} ({len(m)})")
        ax.axvline(real, color="#9467bd", lw=2.5, label="data")
        ax.set_xlabel(rf"$\hat\xi_{{\rm LS}}^{{\times}}$ ({label}, R = {r_km} km)")
        ax.set_ylabel("count")
        ax.set_title(f"({tag}) TRIAL x SEIYU vs null mocks, R = {r_km} km")
        ax.legend(frameon=False, fontsize=7, loc="upper right")

    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "xi_cross"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

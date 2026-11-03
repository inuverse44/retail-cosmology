"""オーストラリア（Coles、Woolworths）と日本（トライアル）の二点相関関数を並べて描く。

cargo run --release --bin xi -- 2026-11-01
cargo run --release --bin xi -- 2026-11-03 --country jp
cargo run --release --bin xi -- 2026-11-03 --country jp --stores data/osm/trial_clean_2026-11-01.csv --tag osm
uv run scripts/plot_compare.py 2026-11-01 2026-11-03
"""

import csv
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.ticker import NullFormatter, ScalarFormatter

ROOT = Path(__file__).resolve().parents[1]
STYLE = {
    "Coles": {"color": "#d62728", "marker": "o"},
    "Woolworths": {"color": "#2ca02c", "marker": "s"},
    "TRIAL": {"color": "#1f77b4", "marker": "D"},
    "TRIAL (OSM)": {"color": "#1f77b4", "marker": "D", "mfc": "none", "ls": ":"},
}
SHIFT = {"Coles": 0.94, "Woolworths": 0.98, "TRIAL": 1.02, "TRIAL (OSM)": 1.06}


def load(path: Path) -> dict:
    res: dict = {}
    for r in csv.DictReader(open(path, encoding="utf-8")):
        res.setdefault((r["quantity"], r["sample"], r["random"]), []).append(r)
    out = {}
    for k, rows in res.items():
        lo = np.array([float(r["r_lo_km"]) for r in rows])
        hi = np.array([float(r["r_hi_km"]) for r in rows])
        out[k] = (np.sqrt(lo * hi), np.array([float(r["value"]) for r in rows]), np.array([float(r["sigma"]) for r in rows]))
    return out


def main() -> None:
    au_date, jp_date = sys.argv[1], sys.argv[2]
    au = load(ROOT / "data" / "xi" / f"xi_{au_date}.csv")
    jp = load(ROOT / "data" / "xi" / f"xi_jp_{jp_date}.csv")
    jp_osm = load(ROOT / "data" / "xi" / f"xi_jp_{jp_date}_osm.csv")
    series = [("Coles", au, "Coles"), ("Woolworths", au, "Woolworths"), ("TRIAL", jp, "TRIAL"), ("TRIAL (OSM)", jp_osm, "TRIAL")]

    fig, axes = plt.subplots(1, 3, figsize=(13, 4))
    panels = [
        ("dr_over_rr_minus_1", "population", r"$dr/rr - 1$", "(a) stores per person vs density"),
        ("ls", "population", r"$\hat\xi_{\rm LS}$", "(b) random: weighted by population"),
        ("ratio_to_population", "inhabited", r"$\hat\xi_{\rm store} / \hat\xi_{\rm population}$", "(c) random: inhabited cells"),
    ]
    for ax, (q, rnd, ylabel, title) in zip(axes, panels, strict=True):
        ax.axhline(1 if q.startswith("ratio") else 0, color="0.6", lw=0.8)
        for label, data, sample in series:
            key = (q, sample, rnd)
            if key not in data:
                continue
            r, v, e = data[key]
            # 比は人口の ξ が大きい 50 km までに絞る（日本の人口の ξ は 100 km 付近で 0 をまたぎ、比が発散する）
            ok = r < 55 if q.startswith("ratio") else r < 1200
            st = STYLE[label]
            ax.errorbar(
                r[ok] * SHIFT[label],
                v[ok],
                yerr=e[ok],
                color=st["color"],
                marker=st["marker"],
                mfc=st.get("mfc", st["color"]),
                ls=st.get("ls", "-"),
                ms=3.5,
                lw=1,
                capsize=2,
                label=label,
            )
        ax.set_xscale("log")
        if q.startswith("ratio"):
            ax.set_xticks([3, 10, 30])
            ax.xaxis.set_major_formatter(ScalarFormatter())
            ax.xaxis.set_minor_formatter(NullFormatter())
        ax.set_xlabel(r"$r$ [km]")
        ax.set_ylabel(ylabel)
        ax.set_title(title)
    axes[0].legend(frameon=False, fontsize=8)
    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "xi_compare"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

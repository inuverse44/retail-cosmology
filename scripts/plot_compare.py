"""オーストラリア（Coles、Woolworths）と日本（トライアル）の二点相関関数を並べて描く。

cargo run --release --bin xi -- 2026-11-01
cargo run --release --bin xi -- 2026-11-03 --country jp
cargo run --release --bin xi -- 2026-11-03 --country jp --stores data/osm/trial_clean_2026-11-01.csv --tag osm
uv run scripts/plot_compare.py 2026-11-01 2026-11-03
"""

import csv
import sys
from pathlib import Path

import figstyle as fs
import numpy as np
from matplotlib.ticker import NullFormatter, ScalarFormatter

ROOT = fs.ROOT
# (凡例の名前, データ, 系列, 横のずらし, 追加の体裁)
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
    series = [
        ("Coles", au, "Coles", {}),
        ("Woolworths", au, "Woolworths", {}),
        ("TRIAL", jp, "TRIAL", {}),
        ("TRIAL (OSM)", jp_osm, "TRIAL", {"mfc": "white", "ls": ":"}),
    ]

    fig, axes = fs.figure(1, 3, height=2.3)
    panels = [
        ("dr_over_rr_minus_1", "population", r"$dr/rr - 1$", "weighted by population"),
        ("ls", "population", r"$\hat\xi_{\mathrm{LS}}$", "weighted by population"),
        ("ratio_to_population", "inhabited", r"$\hat\xi_{\mathrm{store}} / \hat\xi_{\mathrm{population}}$", "inhabited cells"),
    ]
    for ax, tag, (q, rnd, ylabel, name) in zip(axes, "abc", panels, strict=True):
        fs.ref_line(ax, 1 if q.startswith("ratio") else 0)
        for label, data, sample, extra in series:
            key = (q, sample, rnd)
            if key not in data:
                continue
            r, v, e = data[key]
            # 比は人口の ξ が大きい 50 km までに絞る（日本の人口の ξ は 100 km 付近で 0 をまたぎ、比が発散する）
            ok = r < 55 if q.startswith("ratio") else r < 1200
            fs.errorbar(ax, r[ok] * SHIFT[label], v[ok], e[ok], sample, label=label, **extra)
        ax.set_xscale("log")
        if q.startswith("ratio"):
            ax.set_xticks([3, 10, 30])
            ax.xaxis.set_major_formatter(ScalarFormatter())
            ax.xaxis.set_minor_formatter(NullFormatter())
        ax.set_xlabel(r"$r$ [km]")
        ax.set_ylabel(ylabel)
        fs.panel(ax, tag, name)
    fig.legend(*axes[0].get_legend_handles_labels(), loc="outside upper center", ncol=4)
    fs.save(fig, "xi_compare")


if __name__ == "__main__":
    main()

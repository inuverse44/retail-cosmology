"""選択関数（出店している地方）を入れた二点相関関数と、帰無仮説の模擬データの分布を描く。

cargo run --release --bin xi -- 2026-11-03 --country jp --footprint 50,100,200 --tag fp
cargo run --release --bin xi -- 2026-11-01 --footprint 50,100,200 --tag fp
scripts/null_mocks.sh 2026-11-03 jp 50 100
uv run scripts/plot_footprint.py 2026-11-01 2026-11-03
"""

import csv
import re
import sys
from pathlib import Path

import figstyle as fs
import numpy as np
from matplotlib.ticker import MaxNLocator

ROOT = fs.ROOT
XI = ROOT / "data" / "xi"


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


def summary_value(text: str) -> float:
    m = re.search(r"まとめた値 (-?[0-9.]+) ±", text)
    return float(m.group(1)) if m else float("nan")


def r_series(ax, d: dict, quantity: str, sample: str, entity: str) -> None:
    """人口だけの基準（灰色）と、選択関数の幅 R ごとの基準（系列の色の濃淡）を描く。"""
    r, v, e = d[(quantity, sample, "population")]
    ok = r < 200
    fs.errorbar(ax, r[ok], v[ok], e[ok], "population", label="population only")
    for r_km, shift in [(50, 0.97), (100, 1.0), (200, 1.03)]:
        r, v, e = d[(quantity, sample, f"footprint_{r_km}")]
        ok = r < 200
        fs.errorbar(
            ax,
            r[ok] * shift,
            v[ok],
            e[ok],
            entity,
            label=rf"$R = {r_km}$ km",
            color=fs.r_color(entity, r_km),
            marker=fs.R_MARKER[r_km],
        )


def main() -> None:
    au_date, jp_date = sys.argv[1], sys.argv[2]
    jp = load(XI / f"xi_jp_{jp_date}_fp.csv")
    au = load(XI / f"xi_{au_date}_fp.csv")
    fig, axes = fs.figure(1, 3, height=2.4)

    ax = axes[0]
    fs.ref_line(ax, 0)
    r_series(ax, jp, "ls", "TRIAL", "TRIAL")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "a", "TRIAL")
    ax.set_ylim(top=2.6)  # 凡例の場所を空ける
    ax.legend(loc="upper right")

    ax = axes[1]
    fs.ref_line(ax, 0)
    for sample in ("Coles", "Woolworths"):
        r, v, e = au[("ls", sample, "footprint_100")]
        ok = r < 200
        fs.errorbar(ax, r[ok] * fs.SHIFT[sample], v[ok], e[ok], sample)
    r, v, e = jp[("ls", "TRIAL", "footprint_100")]
    ok = r < 200
    fs.errorbar(ax, r[ok], v[ok], e[ok], "TRIAL")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "b", r"$R = 100$ km")
    ax.set_ylim(top=0.7)  # 凡例の場所を空ける
    ax.legend(loc="upper right")

    ax = axes[2]
    mocks = [summary_value(line) for line in open(XI / "null_mocks_jp_50.txt", encoding="utf-8") if "まとめた値" in line]
    real = summary_value(open(XI / f"chi2_jp_{jp_date}_fp.txt", encoding="utf-8").read().split("footprint_50）")[1])
    ax.hist(mocks, bins=20, color=fs.COLOR["mock"], label=f"null mocks ({len(mocks)})")
    ax.axvline(real, color=fs.COLOR["TRIAL"], lw=1.5, label="TRIAL")
    ax.set_xlabel(r"$\hat\xi_{\mathrm{LS}}$ (8--20 km)")
    ax.set_ylabel("count")
    fs.panel(ax, "c", r"TRIAL, $R = 50$ km")
    ax.xaxis.set_major_locator(MaxNLocator(4))
    ax.set_ylim(top=ax.get_ylim()[1] * 1.35)  # 凡例の場所を空ける
    ax.legend(loc="upper left")
    fs.save(fig, "xi_footprint")


if __name__ == "__main__":
    main()

"""Walmart（米国、#51）の選択関数つきの二点相関関数を描き、帰無仮説の模擬データと比べる。

cargo run --release --bin xi -- 2026-11-13 --country us --footprint 50,100,200 --tag fp
cargo run --release --bin xi -- 2026-11-03 --country jp --footprint 50,100,200 --tag fp
cargo run --release --bin xi -- 2026-11-01 --footprint 50,100,200 --tag fp
scripts/null_mocks_walmart.sh 2026-11-13 100 6
uv run scripts/plot_walmart.py 2026-11-13 2026-11-01 2026-11-03
"""

import csv
import sys

import figstyle as fs
import numpy as np
from plot_footprint import load, r_series

ROOT = fs.ROOT
XI = ROOT / "data" / "xi"
R_MAX = 200  # 描く距離の上限 [km]


def mock_band(path, sample: str) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """模擬データのビンごとの値から、ビンの中点と、平均・標準偏差を返す。"""
    by_bin: dict[tuple[float, float], list[float]] = {}
    for r in csv.DictReader(open(path, encoding="utf-8")):
        if r["quantity"] == "ls" and r["sample"] == sample:
            by_bin.setdefault((float(r["r_lo_km"]), float(r["r_hi_km"])), []).append(float(r["value"]))
    keys = sorted(by_bin)
    mid = np.array([np.sqrt(lo * hi) for lo, hi in keys])
    vals = [np.array(by_bin[k]) for k in keys]
    return mid, np.array([v.mean() for v in vals]), np.array([v.std(ddof=1) for v in vals])


def main() -> None:
    us_date, au_date, jp_date = sys.argv[1:4]
    us = load(XI / f"xi_us_{us_date}_fp.csv")
    au = load(XI / f"xi_{au_date}_fp.csv")
    jp = load(XI / f"xi_jp_{jp_date}_fp.csv")
    fig, axes = fs.figure(1, 3, height=2.4)

    ax = axes[0]
    fs.ref_line(ax, 0)
    r_series(ax, us, "ls", "WALMART", "WALMART")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "a", "WALMART")
    ax.legend(loc="lower right")

    ax = axes[1]
    fs.ref_line(ax, 0)
    mid, mean, sd = mock_band(XI / "null_mocks_us_WALMART_50_density_grid_bins.csv", "WALMART")
    ok = mid < R_MAX
    band = r"null mocks (mean $\pm 1\sigma$)"
    ax.fill_between(mid[ok], (mean - sd)[ok], (mean + sd)[ok], color=fs.COLOR["mock"], alpha=0.5, lw=0, label=band)
    r, v, e = us[("ls", "WALMART", "footprint_50")]
    ok = r < R_MAX
    fs.errorbar(ax, r[ok], v[ok], e[ok], "WALMART")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "b", r"$R = 50$ km")
    ax.legend(loc="lower right")

    ax = axes[2]
    fs.ref_line(ax, 0)
    for d, sample in [(au, "Coles"), (au, "Woolworths"), (jp, "TRIAL"), (us, "WALMART")]:
        r, v, e = d[("ls", sample, "footprint_100")]
        ok = r < R_MAX
        fs.errorbar(ax, r[ok] * fs.SHIFT[sample], v[ok], e[ok], sample)
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\mathrm{LS}}$")
    fs.panel(ax, "c", r"$R = 100$ km")
    ax.legend(loc="lower right")
    fs.save(fig, "xi_walmart")


if __name__ == "__main__":
    main()

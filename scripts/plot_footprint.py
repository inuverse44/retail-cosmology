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

import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
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


def main() -> None:
    au_date, jp_date = sys.argv[1], sys.argv[2]
    jp = load(XI / f"xi_jp_{jp_date}_fp.csv")
    au = load(XI / f"xi_{au_date}_fp.csv")
    fig, axes = plt.subplots(1, 3, figsize=(13, 4))

    ax = axes[0]
    ax.axhline(0, color="0.6", lw=0.8)
    styles = [
        ("population", "population only", "0.5", 1.0),
        ("footprint_50", "R = 50 km", "#1f77b4", 0.97),
        ("footprint_100", "R = 100 km", "#ff7f0e", 1.0),
        ("footprint_200", "R = 200 km", "#2ca02c", 1.03),
    ]
    for rnd, label, color, shift in styles:
        r, v, e = jp[("ls", "TRIAL", rnd)]
        ok = r < 200
        ax.errorbar(r[ok] * shift, v[ok], yerr=e[ok], color=color, marker="o", ms=3, lw=1, capsize=2, label=label)
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\rm LS}$")
    ax.set_title("(a) TRIAL: random with selection function")
    ax.legend(frameon=False, fontsize=8)

    ax = axes[1]
    ax.axhline(0, color="0.6", lw=0.8)
    for sample, color, shift in [("Coles", "#d62728", 0.97), ("Woolworths", "#2ca02c", 1.03)]:
        r, v, e = au[("ls", sample, "footprint_100")]
        ok = r < 200
        ax.errorbar(r[ok] * shift, v[ok], yerr=e[ok], color=color, marker="o", ms=3, lw=1, capsize=2, label=sample)
    r, v, e = jp[("ls", "TRIAL", "footprint_100")]
    ok = r < 200
    ax.errorbar(r[ok], v[ok], yerr=e[ok], color="#1f77b4", marker="D", ms=3, lw=1, capsize=2, label="TRIAL")
    ax.set_xscale("log")
    ax.set_xlabel(r"$r$ [km]")
    ax.set_ylabel(r"$\hat\xi_{\rm LS}$")
    ax.set_title("(b) R = 100 km")
    ax.legend(frameon=False, fontsize=8)

    ax = axes[2]
    mocks_path = XI / "null_mocks_jp_50.txt"
    mocks = [summary_value(line) for line in open(mocks_path, encoding="utf-8") if "まとめた値" in line]
    real = summary_value(open(XI / f"chi2_jp_{jp_date}_fp.txt", encoding="utf-8").read().split("footprint_50）")[1])
    ax.hist(mocks, bins=20, color="0.7", label=f"null mocks ({len(mocks)})")
    ax.axvline(real, color="#1f77b4", lw=2, label="TRIAL")
    ax.set_xlabel(r"$\hat\xi_{\rm LS}$ (8–20 km, R = 50 km)")
    ax.set_ylabel("count")
    ax.set_title("(c) TRIAL vs null mocks")
    ax.xaxis.set_major_locator(plt.MaxNLocator(5))
    ax.legend(frameon=False, fontsize=8, loc="upper left")

    fig.tight_layout()
    out = ROOT / "docs" / "figures" / "xi_footprint"
    for ext in ("pdf", "png"):
        fig.savefig(out.with_suffix(f".{ext}"), dpi=200)


if __name__ == "__main__":
    main()

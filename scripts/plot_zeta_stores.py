"""コスモスと Walmart の三点相関関数を、三角形のビンごとに描き、帰無仮説の模擬データと比べる（#81）。

辺のビンは 2–32 km を対数で 4 等分（中点 3、6、12、24 km）。横軸は三角形のビンで、ラベルは三辺の中点 [km]。

C=data/cosmos_site/cosmos_site_clean_2026-11-12.csv
W=data/osm/walmart_clean_2026-11-13.csv
cargo run --release --bin zeta -- 2026-11-12 --country jp --stores $C --chain COSMOS --n-random 8000 --tag t8000
cargo run --release --bin zeta -- 2026-11-12 --country jp --stores $C --chain COSMOS --n-random 8000 \
  --footprint 50 --tag fp50_t8000
scripts/zeta_mocks.sh 2026-11-12 jp $C COSMOS 1 10 --n-random 8000 --footprint 50 --tag fp50_t8000
cargo run --release --bin zeta -- 2026-11-13 --country us --stores $W --chain WALMART --n-random 40000 \
  --footprint 50 --tag fp50_t40000
scripts/zeta_mocks.sh 2026-11-13 us $W WALMART 1 10 --n-random 40000 --footprint 50 --tag fp50_t40000
uv run scripts/plot_zeta_stores.py
"""

import csv

import figstyle as fs
import numpy as np

ZETA = fs.ROOT / "data" / "zeta"
N_MOCK = 10


def load(name: str) -> dict[tuple[int, int, int], tuple[str, float]]:
    """三角形のビン（辺のビンの番号の組）ごとに、ラベルと ζ。ζ が NaN のビンは除く。"""
    out = {}
    for r in csv.DictReader(open(ZETA / name, encoding="utf-8")):
        z = float(r["zeta"])
        if np.isnan(z):
            continue
        k = (int(r["k1"]), int(r["k2"]), int(r["k3"]))
        label = ",".join(f"{float(r[c]):.0f}" for c in ("r1_km", "r2_km", "r3_km"))
        out[k] = (label, z)
    return out


def mocks(stem: str) -> dict[tuple[int, int, int], np.ndarray]:
    """模擬データの ζ を三角形のビンごとに集める。"""
    vals: dict[tuple[int, int, int], list[float]] = {}
    for s in range(1, N_MOCK + 1):
        for k, (_, z) in load(f"{stem}_mock{s}.csv").items():
            vals.setdefault(k, []).append(z)
    return {k: np.array(v) for k, v in vals.items()}


def with_mocks(ax, stem: str, entity: str) -> list[str]:
    """実際の ζ と、模擬データの平均 ± 標準偏差（灰色）を描く。ビンのラベルを返す。"""
    real = load(f"{stem}.csv")
    m = mocks(stem)
    keys = sorted(real)
    x = np.arange(len(keys))
    mean = np.array([m[k].mean() for k in keys])
    sd = np.array([m[k].std(ddof=1) for k in keys])
    ax.errorbar(
        x, mean, yerr=sd, color=fs.COLOR["mock"], fmt="_", ms=6, label=rf"$\langle\hat\zeta_0\rangle \pm 1\sigma$, $N = {N_MOCK}$"
    )
    ax.plot(x, [real[k][1] for k in keys], fs.MARKER[entity], color=fs.COLOR[entity], label=entity)
    return [real[k][0] for k in keys]


def main() -> None:
    fig, axes = fs.figure(3, 1, height=6.0, sharex=True)

    ax = axes[0]
    fs.ref_line(ax, 0)
    pop = load("zeta_jp_2026-11-12_cosmos_t8000.csv")
    fp = load("zeta_jp_2026-11-12_cosmos_fp50_t8000.csv")
    keys = sorted(fp)
    x = np.arange(len(keys))
    ax.plot(x, [pop[k][1] for k in keys], fs.MARKER["population"], color=fs.COLOR["population"], label=r"$\hat\zeta_\rho$")
    ax.plot(
        x, [fp[k][1] for k in keys], fs.MARKER["COSMOS"], color=fs.COLOR["COSMOS"], label=r"$\hat\zeta_S$, $R = 50$ km"
    )
    ax.set_ylabel(r"$\hat\zeta$")
    ax.legend(loc="center right")
    fs.panel(ax, "a", "COSMOS")

    ax = axes[1]
    fs.ref_line(ax, 0)
    with_mocks(ax, "zeta_jp_2026-11-12_cosmos_fp50_t8000", "COSMOS")
    ax.set_ylabel(r"$\hat\zeta$")
    ax.legend(loc="lower right")
    fs.panel(ax, "b", r"COSMOS, $R = 50$ km")

    ax = axes[2]
    fs.ref_line(ax, 0)
    labels = with_mocks(ax, "zeta_us_2026-11-13_walmart_fp50_t40000", "WALMART")
    ax.set_ylabel(r"$\hat\zeta$")
    ax.legend(loc="upper right")
    fs.panel(ax, "c", r"WALMART, $R = 50$ km")
    ax.set_xticks(np.arange(len(labels)), labels, rotation=60, ha="right")
    ax.xaxis.set_minor_locator(fs.plt.NullLocator())
    ax.set_xlabel(r"$(r_1, r_2, r_3)$ [km]")
    fs.save(fig, "zeta_stores")


if __name__ == "__main__":
    main()

"""トライアルとコスモスの選択関数つきの二点相関関数と相互相関を、全国と九州の中だけで並べて描く（#51）。

cargo run --release --bin xi -- 2026-11-03 --country jp --stores $T,$C --cross TRIAL,COSMOS --footprint 50,100 --tag cosmosfp
cargo run --release --bin xi -- 2026-11-03 --country jp --stores $T,$C --cross TRIAL,COSMOS --footprint 50,100 \
  --prefectures 40-46 --tag kyushu
  $T = data/trial_site/trial_site_clean_2026-11-03.csv、$C = data/cosmos_site/cosmos_site_clean_2026-11-12.csv
uv run scripts/plot_cosmos.py 2026-11-03
"""

import sys

import figstyle as fs
from plot_footprint import load

ROOT = fs.ROOT
XI = ROOT / "data" / "xi"
R_MAX = 200  # 描く距離の上限 [km]
RANDOM = "footprint_50"


def main() -> None:
    jp_date = sys.argv[1]
    fig, axes = fs.figure(1, 2, width=0.8, height=2.4, sharey=True)
    for ax, tag, (name, title) in zip(axes, "ab", [("cosmosfp", "Japan"), ("kyushu", "Kyushu")], strict=True):
        d = load(XI / f"xi_jp_{jp_date}_{name}.csv")
        fs.ref_line(ax, 0)
        for quantity, sample, entity, label in [
            ("ls", "TRIAL", "TRIAL", "TRIAL"),
            ("ls", "COSMOS", "COSMOS", "COSMOS"),
            ("ls_cross", "TRIALxCOSMOS", "cross", r"TRIAL $\times$ COSMOS"),
        ]:
            r, v, e = d[(quantity, sample, RANDOM)]
            ok = r < R_MAX
            fs.errorbar(ax, r[ok] * fs.SHIFT[entity], v[ok], e[ok], entity, label=label)
        ax.set_xscale("log")
        ax.set_xlabel(r"$r$ [km]")
        fs.panel(ax, tag, rf"{title}, $R = 50$ km")
    axes[0].set_ylabel(r"$\hat\xi$")
    axes[0].legend(loc="lower right")
    fs.save(fig, "xi_cosmos")


if __name__ == "__main__":
    main()

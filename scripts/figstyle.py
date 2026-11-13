"""ノートの図の体裁（ADR 004）。描画のスクリプトはこれを読み込み、何を描くかだけを書く。

- 図はノートに載る大きさで描き、縮めずに載せる（`figure` の幅は本文の幅の割合）。
- 文字と数式は、本文と同じ LuaLaTeX とフォントで組む（matplotlib の pgf の出力）。
- 色は表すものに固定する（`COLOR`、`MARKER`）。順序のある量（選択関数の幅 R）は、その系列の色の濃淡で表す（`shade`）。
- パネルの見出しは (a)、(b) と短い名前だけ（`panel`）。説明はキャプションに書く。
- グリッドは引かない。枠は四辺で、目盛りは内向き。
"""

from pathlib import Path

import matplotlib

matplotlib.use("pgf")

import matplotlib.colors as mcolors  # noqa: E402
import matplotlib.pyplot as plt  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FIG = ROOT / "docs" / "figures"

# 本文の幅：ltjsarticle、A4、11 pt で 445.5 pt（TeX の pt は 1/72.27 in）
TEXTWIDTH = 445.5 / 72.27

# 図をまたいで出てくるものの色。同じパネルに並ぶ組（オーストラリアの 2 社、日本の 2 社と相互相関、
# 比較の図の Coles・Woolworths・トライアル）は、色覚の検査（dataviz の validate_palette.js、--pairs all）を通した。
# コスモス（#51）はトライアル・相互相関と、Walmart（#51）は Coles・Woolworths・トライアルと並べて通した（2026-11-13）。
COLOR = {
    "Coles": "#eb6834",
    "Woolworths": "#1baf7a",
    "TRIAL": "#2a78d6",
    "SEIYU": "#e34948",
    "COSMOS": "#c2398f",
    "WALMART": "#9c5b00",
    "cross": "#008300",  # 相互相関（トライアル × 西友、トライアル × コスモス。一つの図に二つの組は並べない）
    "population": "0.45",  # 人口そのもの、人口だけを基準にしたもの
    "mock": "0.65",  # 帰無仮説の模擬データ
    "data": "0.0",
    # 計算量のノートの図（チェーンの色とは別）
    "selection": "#6250d6",  # 選択関数を求める工程
    "pairs": "#eda100",  # 対を数える工程
}
MARKER = {
    "Coles": "o",
    "Woolworths": "s",
    "TRIAL": "D",
    "SEIYU": "^",
    "COSMOS": "P",
    "WALMART": "X",
    "cross": "v",
    "population": "x",
}
# 推定量は色ではなく、灰色の濃さとマーカー（塗りつぶすか白抜きか）で区別する
ESTIMATOR = {
    "natural": {"color": "0.1", "marker": "o"},
    "davis_peebles": {"color": "0.55", "marker": "^"},
    "hamilton": {"color": "0.55", "marker": "D", "mfc": "white"},
    "landy_szalay": {"color": "0.1", "marker": "s", "mfc": "white"},
}
# 同じビンの点が重ならないよう横にずらす倍率
SHIFT = {
    "Coles": 0.97,
    "Woolworths": 1.03,
    "TRIAL": 1.0,
    "SEIYU": 1.0,
    "COSMOS": 1.03,
    "WALMART": 1.0,
    "cross": 0.97,
    "population": 1.0,
}

# 選択関数の幅 R [km] の濃淡：小さい R ほど濃い
R_SHADE = {50: -0.35, 100: 0.0, 200: 0.45}
R_MARKER = {50: "o", 100: "s", 200: "D"}

BASE = 9  # 軸のラベルとパネルの見出し [pt]
SMALL = 8  # 目盛りと凡例 [pt]

plt.rcParams.update(
    {
        "pgf.texsystem": "lualatex",
        "pgf.rcfonts": False,
        "pgf.preamble": r"\usepackage{luatexja}\usepackage{amsmath,amssymb,bm}",
        "text.usetex": False,
        "font.family": "serif",
        "font.size": BASE,
        "axes.labelsize": BASE,
        "axes.titlesize": BASE,
        "axes.titlelocation": "left",
        "axes.titlepad": 3,
        "xtick.labelsize": SMALL,
        "ytick.labelsize": SMALL,
        "legend.fontsize": SMALL,
        "legend.frameon": False,
        "legend.handlelength": 1.6,
        "legend.borderaxespad": 0.3,
        "axes.grid": False,
        "axes.linewidth": 0.6,
        "xtick.direction": "in",
        "ytick.direction": "in",
        "xtick.top": True,
        "ytick.right": True,
        "xtick.minor.visible": True,
        "ytick.minor.visible": True,
        "xtick.major.size": 3.5,
        "ytick.major.size": 3.5,
        "xtick.minor.size": 1.8,
        "ytick.minor.size": 1.8,
        "xtick.major.width": 0.6,
        "ytick.major.width": 0.6,
        "xtick.minor.width": 0.5,
        "ytick.minor.width": 0.5,
        "lines.linewidth": 1.0,
        "lines.markersize": 3.0,
        "errorbar.capsize": 1.8,
        "axes.unicode_minus": True,
        "figure.dpi": 100,
        "savefig.dpi": 200,
    }
)


def figure(nrows: int = 1, ncols: int = 1, width: float = 1.0, height: float = 2.3, **kw):
    """本文の幅の `width` 倍、高さ `height` [in] の図を作る。"""
    return plt.subplots(nrows, ncols, figsize=(width * TEXTWIDTH, height), layout="constrained", **kw)


def panel(ax, tag: str | None, name: str = "") -> None:
    """パネルの左上に「(a) 名前」をつける。"""
    label = f"({tag})" if tag else ""
    ax.set_title(f"{label} {name}".strip())


def ref_line(ax, y: float = 0.0) -> None:
    """基準の値（ξ = 0、比 = 1 など）に薄い線を引く。"""
    ax.axhline(y, color="0.75", lw=0.6, zorder=0)


def shade(color: str, amount: float) -> tuple[float, float, float]:
    """`amount` > 0 なら白に、< 0 なら黒に近づけた色。"""
    rgb = mcolors.to_rgb(color)
    target = (1.0, 1.0, 1.0) if amount > 0 else (0.0, 0.0, 0.0)
    a = abs(amount)
    return tuple((1 - a) * c + a * t for c, t in zip(rgb, target, strict=True))


def r_color(entity: str, r_km: int):
    """系列 `entity` の、選択関数の幅 `r_km` に当たる濃淡。"""
    return shade(COLOR[entity], R_SHADE[r_km])


def errorbar(ax, r, v, e, entity: str, label: str | None = None, **kw):
    """系列 `entity` の色とマーカーで誤差棒つきの点と線を描く。"""
    style = {"color": COLOR.get(entity), "marker": MARKER.get(entity, "o"), "label": label or entity}
    style.update(kw)
    return ax.errorbar(r, v, yerr=e, **style)


def save(fig, name: str) -> None:
    """docs/figures/<name>.pdf と .png に書く。大きさは変えない（bbox を詰めない）。"""
    FIG.mkdir(parents=True, exist_ok=True)
    for ext in ("pdf", "png"):
        fig.savefig(FIG / f"{name}.{ext}")
    plt.close(fig)

# retail-cosmology

オーストラリアの Coles と Woolworths の店舗座標に、宇宙論で銀河の分布を測るときの二点相関関数（Landy–Szalay 推定量ほか）を当てる研究。
小売に宇宙論をどこまで適用できるかを調べる。

- 発表予定：[TRIAL＆RetailAI Advent Calendar 2026](https://qiita.com/advent-calendar/2026/trial-retail-ai) の 12 月 23 日
- 理論の下書き（書きかけ）：[Landy-Szalay推定量①](https://inuverse.dev/posts/2026-04-12-retail/)、[Landy-Szalay推定量②](https://inuverse.dev/posts/2026-04-13-retail/)
- ノート：[`notes/retail-cosmology.pdf`](notes/retail-cosmology.pdf)

## 構成

```
crates/retail-cosmology/  対の数え上げと推定量、OSM の店舗の選び出し（Rust）
scripts/                  描画（Python, matplotlib）と OSM の取得（scripts/osm/）
reference/                公式の店舗数と出典
notes/                    ノート（TeX）
docs/figures/             ノートに載せる図
data/                     取得したデータと計算結果（コミットしない）
```

## 動かす

```sh
cargo test
cargo run --release --bin toy > data/toy/poisson.csv   # 先に mkdir -p data/toy
uv run scripts/plot_toy.py                             # docs/figures/toy_poisson.{pdf,png}
scripts/osm/fetch.sh 2026-11-01                         # data/osm/stores_2026-11-01.tsv
cargo run --release --bin osm_census -- 2026-11-01     # 公式の数と比べ、docs/tables/osm_census.tex を書く
uv run scripts/plot_stores.py 2026-11-01               # docs/figures/stores_osm.{pdf,png}
cd notes && latexmk -lualatex retail-cosmology.tex
```

## データ

- 店舗の座標：[OpenStreetMap](https://www.openstreetmap.org)（© OpenStreetMap contributors、ODbL）。Overpass API で 2026-11-01T00:00:00Z 時点のデータを取り出す。選び方と限界はノートの「店舗データ」の節。
- 公式の店舗数：[`reference/`](reference/README.md)。Coles と Woolworths の決算資料（2026年6月28日時点）。

OSM から数えた店舗数は、Coles が 865／870 店、Woolworths が 1,116／1,130 店（公式との比）。

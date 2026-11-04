# retail-cosmology

オーストラリアの Coles と Woolworths、日本のトライアルと西友の店舗座標に、宇宙論で銀河の分布を測るときの二点相関関数（Landy–Szalay 推定量ほか）を当てる研究。
小売に宇宙論をどこまで適用できるかを調べる。

- 発表予定：[TRIAL＆RetailAI Advent Calendar 2026](https://qiita.com/advent-calendar/2026/trial-retail-ai) の 12 月 23 日
- 理論の下書き（書きかけ）：[Landy-Szalay推定量①](https://inuverse.dev/posts/2026-04-12-retail/)、[Landy-Szalay推定量②](https://inuverse.dev/posts/2026-04-13-retail/)
- ノート：[`notes/retail-cosmology.pdf`](notes/retail-cosmology.pdf)

## 構成

```
crates/retail-cosmology/  計算（Rust）。層の分け方は ADR 001
scripts/                  描画（Python, matplotlib）とデータの取得（シェル）
reference/                公式の店舗数と出典
notes/                    ノート（TeX）
docs/adr/                 設計判断の記録（ADR）
docs/retrospectives/      振り返りと活動のログ
docs/figures/             ノートに載せる図
docs/tables/              ノートに載せる表（Rust が書く）
data/                     取得したデータと計算結果（コミットしない）
```

Rust のコードは、変わりやすさごとに層を分けている（[ADR 001](docs/adr/001-layering-by-volatility.md)）。呼ぶ向きは上から下だけ。

| 層 | 役割 | 置き場所 |
|---|---|---|
| Client | 引数を読み、Manager を呼ぶ | `src/bin/` |
| Manager | 使用例の手順（二点相関関数を測る、店舗の数を公式と比べる） | `src/manager/` |
| Engine | 計算の規則（店の選び方、ランダムカタログ、対の数え上げ、推定量と誤差） | `src/engine/` |
| ResourceAccess | 外のデータの読み書き（人口の格子、店のカタログ、公式の店舗数、結果） | `src/access/` |
| Utility | 乱数、幾何、地図の投影、格子の形 | `src/util/` |

層の間で受け渡すデータの形（データ契約）は `src/contract.rs` に置く。

`scripts/` の描画とデータの取得は、Rust の層の外にある。描画のスクリプトは Rust が書いた結果を読むだけで、計算はしない。

## 動かす

```sh
cargo test
cargo run --release --bin toy > data/toy/poisson.csv   # 先に mkdir -p data/toy
uv run scripts/plot_toy.py                             # docs/figures/toy_poisson.{pdf,png}
scripts/osm/fetch.sh 2026-11-01                         # data/osm/stores_2026-11-01.tsv
cargo run --release --bin osm_census -- 2026-11-01     # 公式の数と比べ、docs/tables/osm_census.tex を書く
uv run scripts/plot_stores.py 2026-11-01               # docs/figures/stores_osm.{pdf,png}
scripts/abs/fetch.sh                                   # data/abs/apgr25_1_0_0.tif（ABS の格子人口）
cargo run --release --bin xi -- 2026-11-01             # data/xi/xi_2026-11-01.csv（jackknife の誤差つき）
scripts/xi_robustness.sh 2026-11-01                    # 設定を変えて計算し直す（ノートの表 2）
uv run scripts/plot_xi.py 2026-11-01                   # docs/figures/xi_stores.{pdf,png}
# 日本（トライアル）
scripts/osm/fetch.sh 2026-11-01 trial                  # data/osm/trial_2026-11-01.tsv
cargo run --release --bin osm_census -- 2026-11-01 trial
scripts/trial_site/fetch.sh                            # data/trial_site/<取得日>/（公式サイトの店舗）
scripts/estat/fetch.sh                                 # data/estat/（e-Stat の 1 km メッシュ人口）
cargo run --release --bin trial_census -- 2026-11-03 2026-11-01
cargo run --release --bin xi -- 2026-11-03 --country jp
cargo run --release --bin xi -- 2026-11-03 --country jp --stores data/osm/trial_clean_2026-11-01.csv --tag osm
uv run scripts/plot_compare.py 2026-11-01 2026-11-03   # docs/figures/xi_compare.{pdf,png}

# 選択関数（出店している地方）を入れる
cargo run --release --bin xi -- 2026-11-03 --country jp --footprint 50,100,200 --tag fp
cargo run --release --bin xi -- 2026-11-01 --footprint 50,100,200 --tag fp
scripts/null_mocks.sh 2026-11-03 jp 50 100             # 帰無仮説の模擬データ 100 個
uv run scripts/plot_footprint.py 2026-11-01 2026-11-03 # docs/figures/xi_footprint.{pdf,png}

# 西友と、トライアルとの相互相関
scripts/seiyu_site/fetch.sh                            # data/seiyu_site/<取得日>/（公式サイトの店舗）
cargo run --release --bin seiyu_census -- 2026-11-04 2026-11-03
S=data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv
cargo run --release --bin xi -- 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50,100,200 --tag crossfp
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6    # 帰無仮説の模擬データ 100 個（6 並列）
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density   # 人の密集地への寄り方もそろえた模擬データ
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6 density
# 模擬データは選択関数を 10 km 格子の表で求めると約 6 分の 1 の時間で済む（結果の差は揺らぎより小さい、notes/algorithm-cost.tex）
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density,grid
uv run scripts/plot_cross.py 2026-11-03                # docs/figures/xi_cross.{pdf,png}

# 模擬データの計算量（notes/algorithm-cost.tex）
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --tag costlog 2> data/cost/mock_log_R50.txt
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --tag costreal 2> data/cost/real_log_R50.txt
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --selection-grid on --tag costlog 2> data/cost/mock_log_R50_grid.txt
cargo run --release --bin cost -- 50 40000             # data/cost/cost_R50.csv
cargo run --release --bin cost -- 100 40000            # data/cost/cost_R100.csv
uv run scripts/plot_cost.py                            # docs/figures/cost_*.{pdf,png}
cd notes && latexmk -lualatex algorithm-cost.tex
cd notes && latexmk -lualatex retail-cosmology.tex

# コードの構成を変える前と後で、すべての出力が変わらないことを確かめる
scripts/check_outputs.sh > before.txt   # 変える前
scripts/check_outputs.sh > after.txt    # 変えた後
diff before.txt after.txt
```

## データ

- 店舗の座標：[OpenStreetMap](https://www.openstreetmap.org)（© OpenStreetMap contributors、ODbL）。Overpass API で 2026-11-01T00:00:00Z 時点のデータを取り出す。選び方と限界はノートの「店舗データ」の節。
- 人口：オーストラリア統計局（ABS）の [2025 年の格子人口](https://www.abs.gov.au/statistics/people/population/regional-population/2024-25)（1 km 格子、EPSG:3577、CC BY 4.0）。ランダムカタログに使う。
- トライアルの店舗：公式サイトの[店舗検索](https://www.trial-net.co.jp/shops)（業態・緯度・経度・都道府県）。著作権はトライアルグループにあるので、取ったデータはコミットしない。
- 西友の店舗：公式サイトの[店舗検索](https://www.seiyu.co.jp/searchshop/)（店名・住所・緯度・経度）。一覧に載っていないトライアル西友は、サイトマップから店のページを取る。著作権は西友にあるので、取ったデータはコミットしない。
- 日本の人口：e-Stat の令和2年国勢調査 1 km メッシュ（T001140、政府標準利用規約 第2.0版）。
- 公式の店舗数：[`reference/`](reference/README.md)。Coles と Woolworths の決算資料（2026年6月28日時点）。

OSM から数えた店舗数は、Coles が 865 店（公式 870 店）、Woolworths が 1,116 店（公式 1,130 店）、トライアルが 232 店（公式 378 店）。
公式サイトから取った店舗数は、トライアルが 375 店（公式 378 店）、西友が 243 店（公式 243 店）。

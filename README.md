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
lean/                     ノートの式の導出の検証（Lean）。ADR 003、005
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
図の体裁（大きさ、色、フォント）は `scripts/figstyle.py` の一か所で決め、文字と数式は本文と同じ LuaLaTeX で組む（[ADR 004](docs/adr/004-figure-style.md)）。描画には LuaLaTeX（`luatexja` と `underscore.sty`）が要る。
`lean/` も Rust の層の外にある。点を独立に撒く模型から、ノートの付録 B の式までを Lean で検証し、数値は Rust のテストで確かめる（置き場所は [ADR 003](docs/adr/003-lean-proofs-in-repo.md)、範囲は [ADR 005](docs/adr/005-lean-model-in-scope.md)）。

## 動かす

```sh
cargo test
cargo run --release --bin toy > data/toy/poisson.csv   # 先に mkdir -p data/toy
uv run scripts/plot_toy.py                             # docs/figures/toy_poisson.{pdf,png}
scripts/osm/fetch.sh 2026-11-01                         # data/osm/stores_2026-11-01.tsv
cargo run --release --bin osm_census -- 2026-11-01     # 公式の数と比べ、docs/tables/osm_census.tex を書く
scripts/naturalearth/fetch.sh                          # data/naturalearth/（地図に描く海岸線）
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
scripts/null_mocks.sh 2026-11-03 jp 50 100 6           # 帰無仮説の模擬データ 100 個（6 並列）
uv run scripts/plot_footprint.py 2026-11-01 2026-11-03 # docs/figures/xi_footprint.{pdf,png}

# 西友と、トライアルとの相互相関
scripts/seiyu_site/fetch.sh                            # data/seiyu_site/<取得日>/（公式サイトの店舗）
cargo run --release --bin seiyu_census -- 2026-11-04 2026-11-03
uv run scripts/plot_stores_jp.py 2026-11-03 2026-11-04 # docs/figures/stores_jp.{pdf,png}
S=data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv
cargo run --release --bin xi -- 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50,100,200 --tag crossfp
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6    # 帰無仮説の模擬データ 100 個（6 並列）
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density   # 人の密集地への寄り方もそろえた模擬データ
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 100 100 6 density
# 模擬データは選択関数を 10 km 格子の表で求めると約 6 分の 1 の時間で済む（結果の差は揺らぎより小さい、notes/algorithm-cost.tex）
scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU 50 100 6 density,grid
# 片方のチェーンを実際の位置のまま残し、もう片方だけを撒き直す（店の固まりを残した帰無仮説、#13）
for R in 50 100; do for K in TRIAL SEIYU; do scripts/null_mocks_cross.sh 2026-11-03 $S TRIAL,SEIYU $R 100 6 density,grid,keep:$K; done; done
uv run scripts/plot_cross.py 2026-11-03                # docs/figures/xi_cross.{pdf,png}
# 選択関数つきのランダムカタログの点の数・参照の点の数・種で、まとめた値がどれだけ動くか（#11、6 並列で約 27 分）
scripts/random_points.sh 2026-11-03 $S TRIAL,SEIYU 6 100000:50000:5 50000:200000:5 50000:50000:10 20000:200000:10 20000:50000:10
# χ² の上側確率を、帰無仮説の模擬データ 100 個と比べて校正する（#9、6 並列で約 30 分）
scripts/chi2_mocks.sh 2026-11-03 jp 0 100 6 $S TRIAL,SEIYU     # 人口だけに比例して撒いた模擬データ
scripts/chi2_mocks.sh 2026-11-03 jp 200 100 6 $S TRIAL,SEIYU   # R = 50、100 km は null_mocks_cross.sh の結果を使う
scripts/chi2_mocks.sh 2026-11-01 au 0 100 6
for R in 50 100 200; do scripts/chi2_mocks.sh 2026-11-01 au $R 100 6; done
cargo run --release --bin chi2_calibrate -- 2026-11-01 2026-11-03   # docs/tables/chi2_calibrated.tex

# 米国（Walmart、#51）
scripts/osm/fetch.sh 2026-11-13 walmart                # data/osm/walmart_2026-11-13.tsv（日付を固定できないので取り出した日。時刻は .timestamp）
scripts/naturalearth/fetch.sh                          # 州境（店を州に割り振り、人口の格子を米国本土に限る）
scripts/ghsl/fetch.sh                                  # data/ghsl/（GHS-POP、2020 年、1 km、約 320 MB）
scripts/osm/fetch.sh 2026-11-13 walmart_outside        # 本土の外（アラスカ、ハワイ、プエルトリコ）。10-K の全米の数と比べるだけ
cargo run --release --bin osm_census -- 2026-11-13 walmart   # data/osm/walmart_clean_2026-11-13.csv、docs/tables/osm_census_walmart.tex
cargo run --release --bin xi -- 2026-11-13 --country us --footprint 50,100,200 --tag fp
scripts/null_mocks_walmart.sh 2026-11-13 100 6         # 帰無仮説の模擬データ 100 個 ×4 組（6 並列で約 32 分）
cargo run --release --bin short_range -- 2026-11-13 walmart  # docs/tables/walmart_mocks.tex

# 模擬データの計算量（notes/algorithm-cost.tex）
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --tag costlog 2> data/cost/mock_log_R50.txt
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --tag costreal 2> data/cost/real_log_R50.txt
target/release/xi 2026-11-03 --country jp --stores $S --cross TRIAL,SEIYU --footprint 50 --mock 50:1 --mock-density on --selection-grid on --tag costlog 2> data/cost/mock_log_R50_grid.txt
cargo run --release --bin cost -- 50 40000             # data/cost/cost_R50.csv
cargo run --release --bin cost -- 100 40000            # data/cost/cost_R100.csv
uv run scripts/plot_cost.py                            # docs/figures/cost_*.{pdf,png}
cd notes && latexmk -lualatex algorithm-cost.tex
cd notes && latexmk -lualatex retail-cosmology.tex
scripts/check_note.sh   # 組んで、「付録 付録C」の重なり・はみ出し・未定義の参照を探す
git config core.hooksPath .githooks   # クローンごとに一度。ノートの tex を変えた push の前に check_note.sh が走り、誤りがあれば止まる

# コードの構成を変える前と後で、すべての出力が変わらないことを確かめる
scripts/check_outputs.sh > before.txt   # 変える前
scripts/check_outputs.sh > after.txt    # 変えた後
diff before.txt after.txt

# ノートの付録 B の式を、点を独立に撒く模型から Lean で検証する（ADR 005、lean/README.md）
cd lean && lake exe cache get   # Mathlib のビルド済みファイル（数 GB）を取る。初回だけ
cd lean && lake build           # 証明を検査する。通れば付録 B の式が検証されている
```

## データ

- 店舗の座標：[OpenStreetMap](https://www.openstreetmap.org)（© OpenStreetMap contributors、ODbL）。Overpass API で 2026-11-01T00:00:00Z 時点のデータを取り出す。選び方と限界はノートの「店舗データ」の節。
- 人口：オーストラリア統計局（ABS）の [2025 年の格子人口](https://www.abs.gov.au/statistics/people/population/regional-population/2024-25)（1 km 格子、EPSG:3577、CC BY 4.0）。ランダムカタログに使う。
- トライアルの店舗：公式サイトの[店舗検索](https://www.trial-net.co.jp/shops)（業態・緯度・経度・都道府県）。著作権はトライアルグループにあるので、取ったデータはコミットしない。
- 西友の店舗：公式サイトの[店舗検索](https://www.seiyu.co.jp/searchshop/)（店名・住所・緯度・経度）。一覧に載っていないトライアル西友は、サイトマップから店のページを取る。著作権は西友にあるので、取ったデータはコミットしない。
- 日本の人口：e-Stat の令和2年国勢調査 1 km メッシュ（T001140、政府標準利用規約 第2.0版）。
- 米国の店舗：OpenStreetMap。ほかの国と違い日付を固定できなかったので、取り出したときのデータを使い、その時刻を `data/osm/walmart_<日付>.timestamp` に残す（2026-11-12T23:56:10Z）。
- 米国の人口：JRC の [GHS-POP R2023A](https://data.jrc.ec.europa.eu/dataset/2ff68a52-5b5b-4a22-8f40-c41da8332cfe)（2020 年、1 km 格子、ESRI:54009、CC BY 4.0）を、[Natural Earth](https://www.naturalearthdata.com/) の州境（1:10m Admin 1、パブリックドメイン）で米国本土 48 州と D.C. に切り出す。
- 公式の店舗数：[`reference/`](reference/README.md)。Coles と Woolworths の決算資料（2026年6月28日時点）、Walmart の 10-K（2026年1月31日時点）など。

OSM から数えた店舗数は、Coles が 865 店（公式 870 店）、Woolworths が 1,116 店（公式 1,130 店）、トライアルが 232 店（公式 378 店）。
公式サイトから取った店舗数は、トライアルが 375 店（公式 378 店）、西友が 243 店（公式 243 店）。
米国本土の Walmart は OSM から 4,564 店。本土の外の 39 店を足した 4,603 店は、10-K の全米の 4,611 店の 99.8%。

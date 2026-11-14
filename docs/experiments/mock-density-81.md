# 丸い塊の模擬データの一点密度を調べる（Issue #81）

密度のずれは確認できたが、それだけで見込みとの差を全て説明できたわけではない。
match では小さい三角形の大きな正のずれが縮まった。一方、`|mean(match−expected_match)| > 2 SE` は
Rust 集計で 6/16 ビン（true は 11/16、proc は 14/16）に残った。
正規近似での偶然の個数の目安は 0.728 個で、ビン間の相関もあるため全体の検定とは扱わないが、
「match は見込みと合い、true だけが合わない」という見立てはこの測定では完全には満たされない。

三辺 3 km では match−expected_match = −0.001171 ± 0.021577、true−expected = +0.176200 ± 0.048392。
三辺 24 km では前者も −0.017530 ± 0.007269 と負の残差がある。
g の 10 km 格子での、match 二群の相互積から求めたずれの RMS の目安は 0.156420。
密度が従来の基準のままとは言えないことと、残る負の残差があることを分けて結論とする。

2026-11-14。main のノート第 5 節に加わった式 (5) と表 1 を踏まえ、
日本のコスモスの clustered_mocks の平均密度が生成時の基準と一致するかを測る。
前の行政区別の選択関数の実験は別ブランチに残し、この実験では origin/main から新しいブランチを作った。
ノートの TeX は変更していない。計算・集計は Rust。

従来の true は親を作るときの基準（実際のコスモスの選択関数 R=50 × 人口）、
proc は模擬店舗から同じ R=50 の選択関数を再推定した基準。
各親から必ず m 個の子を選ぶときの近傍ごとの正規化により、子の一点密度は親の基準に一致する保証がない。
今回の match は、同じ池から親を独立に選び、子を一個だけ選ぶことをランダムカタログの点数だけ繰り返す。
親は従来と同じく 3σ 以内に m 個以上の候補がある場合だけ使う。ガウス重みと 3σ の打切りも同じ。
親ごとに独立なので、同じ親を共有する子の集団としての相関は基準に入れない。
match 用の乱数は元の mock の乱数から分離し、各回で新しいカタログを作る。

m=14、σ=15 km、seed=1、店舗 1,735、ランダム 8,000、参照 50,000、池 160,000。
三角形の辺は 2–32 km を対数で 4 等分。元の店舗生成コードと true/proc/expected の計算は維持した。
expected は同じ親の三角形を true の RRR で写した値、expected_match は同じ三角形を match の RRR で写した値。
報告する三点の差は各回で対にして引いてから平均し、標本標準偏差 / sqrt(N) を平均の誤差とする。
ξ も各基準の平均と平均の誤差を報告し、CSV には標準偏差も保存する。

密度比 g は人口格子の約 10 km の粗い格子（日本では二次メッシュ）で調べる。
池を大きな true の標本として使い、池の個数が 20 以上の格子だけを集計する。
カタログの点数で正規化した match / pool の比を、池の個数で重み付けする。
match を各回で二群に分け、独立な二群の (g_A−1)(g_B−1) の重み付き平均も出す。
この相互積は match の標本雑音の自己積を含まないが、固定した有限の池の影響は除かない。
小さな true ランダムカタログ（8,000 点）との比も格子ごとの CSV に併記する。

再現コマンド（既存ファイルを守るため、再実行時には未使用のタグを選ぶ）：

```sh
cargo build --release --jobs 2 --bin zeta_clustered --bin zeta_density_summary
C=data/cosmos_site/cosmos_site_clean_2026-11-12.csv
# 先に1回だけ実行して時間を測る。
target/release/zeta_clustered 2026-11-12 --country jp --stores "$C" --chain COSMOS --footprint 50 --children 14 --sigma 15 --runs 1 --n-random 8000 --n-ref 50000 --seed 1 --match on --tag mdens_bench
```

[ADR 009](../adr/009-matched-mock-density.md) に設計と比較の限界を記した。
`check_layers.sh` に既存の違反が二つあったので、診断の定数は診断 Manager 内へ、
分散 Client の Weight は Manager を通した公開へ整理した。値と演算は変えていない。

時間測定 1 回：壁時計 313.2 秒、Instant 313.1 秒（外部 time の real は 313.81 秒）。
池の生成までが壁時計 255.3 秒、以後の追加が約 58 秒だった。
20 回は共通初期化を一度だけ行うため約 24 分と見積もり、20 回を選んだ。
重い計算は 1 本ずつ実行し、別作業との並列は指定の 2 本以内に抑えた。

```sh
target/release/zeta_clustered 2026-11-12 --country jp --stores "$C" --chain COSMOS --footprint 50 --children 14 --sigma 15 --runs 20 --n-random 8000 --n-ref 50000 --seed 1 --match on --tag mdens20
S=data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20
target/release/zeta_density_summary "$S"
```

各回で壁時計と Instant を併記し、次の一回で壁時計 1 時間を超える見込みなら、完了した回数まで保存する。
時間測定の従来列は既存の 20 回の run=0 と `cmp` でバイト一致した。

20 回を削減せず完了した。壁時計 1,137.893 秒、Instant 1,137.899 秒。
外部 time は real 1,137.90 秒、user 1,095.60 秒、sys 3.41 秒。
時間測定 1 回と本計算を合わせても壁時計で 1 時間以内であり、時計間の大きな不一致はなかった。

検証は `cargo test --jobs 1 -- --test-threads=1`（58 件）、
`cargo clippy --all-targets --jobs 1 -- -D warnings`、`scripts/check_layers.sh crates/retail-cosmology/src` が全て通過。
従来の ζ の最初の 10 列、ξ の最初の 5 列は、既存の同条件の 20 回データと `cmp` でバイト一致した。
既存出力の値が変わる変更はない。共有出力を上書きする `check_outputs.sh` は実行せず、専用タグの列を直接照合した。

基点は origin/main の `e63444c`。

## 計測結果（Rust による集計）

Stem: `data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20`.

ζ differences are paired within each mock; errors are sample SD / sqrt(N). Match uses its own RRR for expected_match. Columns true/proc use the original expected.

| sides (km) | runs | match − expected_match | true − expected | proc − expected |
|---|---:|---:|---:|---:|
| 3.000, 3.000, 3.000 | 20 | -0.001171 ± 0.021577 | +0.176200 ± 0.048392 | -0.020001 ± 0.022204 |
| 3.000, 3.000, 6.000 | 20 | -0.001036 ± 0.016993 | +0.133077 ± 0.034584 | -0.018352 ± 0.012104 |
| 3.000, 6.000, 6.000 | 20 | -0.007609 ± 0.011780 | +0.085066 ± 0.020230 | -0.020256 ± 0.006838 |
| 3.000, 6.000, 12.000 | 20 | -0.015566 ± 0.007526 | +0.053661 ± 0.011596 | -0.020405 ± 0.005155 |
| 3.000, 12.000, 12.000 | 20 | -0.010311 ± 0.008330 | +0.031315 ± 0.011524 | -0.021922 ± 0.003639 |
| 3.000, 12.000, 24.000 | 20 | -0.012410 ± 0.011492 | +0.017681 ± 0.016443 | -0.028763 ± 0.005228 |
| 3.000, 24.000, 24.000 | 20 | -0.020049 ± 0.005594 | -0.020384 ± 0.007927 | -0.031323 ± 0.003381 |
| 6.000, 6.000, 6.000 | 20 | -0.009305 ± 0.012380 | +0.060733 ± 0.017569 | -0.024595 ± 0.008383 |
| 6.000, 6.000, 12.000 | 20 | -0.014851 ± 0.006725 | +0.034706 ± 0.011476 | -0.018851 ± 0.003665 |
| 6.000, 12.000, 12.000 | 20 | -0.008470 ± 0.008343 | +0.030219 ± 0.012291 | -0.018084 ± 0.002143 |
| 6.000, 12.000, 24.000 | 20 | -0.012644 ± 0.009843 | +0.015956 ± 0.014611 | -0.023425 ± 0.002400 |
| 6.000, 24.000, 24.000 | 20 | -0.017248 ± 0.006041 | -0.015367 ± 0.007226 | -0.025471 ± 0.002159 |
| 12.000, 12.000, 12.000 | 20 | -0.010975 ± 0.008930 | +0.014370 ± 0.013651 | -0.019156 ± 0.002327 |
| 12.000, 12.000, 24.000 | 20 | -0.017197 ± 0.009432 | -0.000946 ± 0.012574 | -0.024983 ± 0.001793 |
| 12.000, 24.000, 24.000 | 20 | -0.017125 ± 0.006889 | -0.014683 ± 0.007748 | -0.019381 ± 0.001429 |
| 24.000, 24.000, 24.000 | 20 | -0.017530 ± 0.007269 | -0.018112 ± 0.006137 | -0.010850 ± 0.001969 |

Bins: 16; bins with at least two runs: 16. Bins exceeding 2 standard errors (absolute residual): match 6/16; true 11/16; proc 14/16. Gaussian null expected count: 0.728 (bins are correlated; this is not a global test).

ξ means ± standard error across mocks:

| r (km) | runs | true | match | proc |
|---|---:|---:|---:|---:|
| 3.000 | 20 | 0.371889 ± 0.017727 | 0.254122 ± 0.015856 | 0.193930 ± 0.010715 |
| 6.000 | 20 | 0.305709 ± 0.013753 | 0.217589 ± 0.013450 | 0.158756 ± 0.006970 |
| 12.000 | 20 | 0.233392 ± 0.010115 | 0.197258 ± 0.010932 | 0.115149 ± 0.005883 |
| 24.000 | 20 | 0.131208 ± 0.009028 | 0.142804 ± 0.009379 | 0.041166 ± 0.004024 |

Density ratio g on the approximately 10 km grid:

Pool = 160000 points (large sample of the true baseline); aggregate match = 160000 points. Retain cells with pool count ≥ 20: 995 cells, 0.948125 of pool mass. All ratios normalized by catalog size and weighted by pool counts.

g mean 1.013982; raw variance 0.030159; weighted 10/50/90% quantiles 0.771300 / 1.050481 / 1.202540.

Split-sample cross mean-square displacement from g=1: 0.024467; sqrt(max(0, cross)) = 0.156420. Independent match halves remove the self product of sampling noise; the pool stays fixed. Raw ratios against the smaller true random catalog are also saved in density_cells.csv.

Completed 20 runs. Instant 1137.899 s; wall clock 1137.893 s.


## 解釈と次の候補

- 小さい三角形の true の正の残差は match で大きく縮まり、親の基準と子の一点密度がずれるという見立てを支持する。
- 同時に、match でも 6/16 ビンが 2 SE を超える残差を持ち、大きな三角形には負の値が残る。密度を合わせれば全てが同じ親の三角形の見込みに合う、とは結論しない。
- 親の数を店の数 / m の切上げで固定している。この固定親数の模型で、connected な三点の見込みを同じ親の三角形だけから与えてよいかを次に調べる。親数を Poisson にした模型との比較や、複数の親にまたがる項の有限標本補正が候補になる。
- match は指示された「各親の最初の子一個」の分布。模擬店舗の m 個は重み付き重複なし抽出なので、全 m 個の周辺分布は厳密には同じとは限らない。全 m 個を抽出してその一個を一様に残す独立な基準との比較が次の候補になる。
- 有限の池、別の親が同じ池の点を選ぶこと、最後の親の端数の扱いも残る候補。ここでは切り分けていない。
- 元の proc−expected は代数上 `(true−expected) + (proc−true)`。その小ささだけで、店舗からの再推定による吸い込みの大きさを決めることはできない。match と true では RRR も expected の換算も違うため、基準の変更を一つの加法的な偏りだけで説明しない。
- 表の誤差は 20 個の mock に対する差の平均の誤差であり、ビン間の共分散を考えた全体の検定ではない。g は池を固定した条件での点推定で、池や参照点の有限標本の不確かさは除いていない。

## 出力

共有 `data/zeta/` の stem は `clustered_jp_2026-11-12_cosmos_m14_s15_mdens20`。

- `.csv`：従来の true/proc/expected と追加した zeta_match/expected_match。
- `_xi.csv`：従来の true/proc と xi_match。
- `_density_test_summary.csv`：三つの paired residual の平均と平均の誤差。
- `_density_test_xi_summary.csv`：ξ の平均・平均の誤差・標準偏差。
- `_density_cells.csv`：約 10 km 格子ごとの池・true・match 二群の個数と密度比。
- `_density_summary.csv`：g の分布、重み付き分散、二群の相互積。
- `_timing.csv`：各回の壁時計と Instant。
- `_density_test.md`：Rust が生成した全比較表。

時間測定の stem は同じ接頭辞の `_mdens_bench`。1 回なので標準誤差は推定できず、そちらの集計では NaN と表示する。
全て新規の mdens タグで保存し、既存の共有ファイルは上書きしていない。

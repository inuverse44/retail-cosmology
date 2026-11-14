# 二つの幅を持つ密度場でコスモスと比較する（#81）

Gを100個、振幅を抑えた比較Eを40個作った。24 kmを含むξは、両方とも
コスモスが平均±1標本SDに入った。procの全体検定もどちらも棄却されない。
しかしtrueのζは両方とも平均が正で、Gでは16/16ビン、Eでは2/16ビンが
0から3標準誤差以上ずれた。**ζ≈0の帰無模型を検証できたとは言えず、
コスモスの物理的なζが0か少し負かの判別は未解決である。**

## 再現方法

人口・店舗・実測三点相関の入力は PR #90 と同じ。
`data/` は共有ディレクトリなので、出力名には毎回固有のタグを付ける。

```sh
cargo build --release --bin zeta_field --bin zeta_field_analysis

# 先に一つの所要時間を測り、少数の独立な run で ξ を調整する。
/usr/bin/time -p target/release/zeta_field 2026-11-12 \
  --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv \
  --chain COSMOS --footprint 50 --n-random 8000 --n-ref 50000 --seed 1 \
  --amp 0.5 --sigma 10 --amp2 0.65 --sigma2 35 --centers 20000 \
  --runs 1 --tag UNIQUE_benchmark

# 選んだ G：探索に使った run 0–3 と重ならない run 100–199。
# 25 個ずつ 4 本。ランダムカタログと点の池の設定は共通。
caffeinate -i bash scripts/field_mocks_two_scale.sh UNIQUE_mainG 0.3 10 1.2 20 100 100
```

`--amp2` と `--sigma2` は両方渡す。各成分は独立な中心と符号を持ち、
切る前の各点での分散は `amp² + amp2²`。
`--start` は模擬データの番号であり、同じ `--seed` の run を重複させない。
run ごとの乱数を番号から作るので、分割の仕方で結果は変わらない。
この run ごとの乱数の方式は従来の単一の乱数列から変更している。

`zeta_field_analysis REAL.csv OUTPUT_STEM MOCK_STEM ...` は分割結果を結合する。
run・ビンの重複、欠損、実測の ξ の不一致を拒否し、既存出力を上書きしない。
少数の探索でも表は出すが、`N <= p+2` では Hartlap の補正を使う検定を行わない。

## 計算時間と探索の条件

最初の1個は wall 435.85 秒。点の池まで358.1秒、追加の1個は77.3秒。
同じ初期費用で25個ずつ4本なら約38.2分という見積もりを得てから本計算を開始した。
候補Gの探索では初期267.3秒、その後約43–62秒/個で、約27分の見積もり。
いずれも本計算の90分の上限以内で、個数は100のままとした。
探索・本計算中にマシンの休止に相当する長い中断があり、wall 時間と
Rust の単調時計で測った累積計算時間に大きな差が生じた。
本計算には `caffeinate -i` を併用したが中断は再発した。
個数の見積もりには累積計算時間を使い、実際の wall 時間は別に記録する。

探索はA–G各4個、すべて seed 1、run 0–3。
Aのみ、時間を測ったrun 0と追加のrun 1–3を結合した。
同じ乱数番号を使う比較であり、候補間の値は独立ではない。
模型の選択には proc の ξ だけを使い、コスモスの ζ は使わない。
「ξ がそろう」の基準は4距離それぞれで目標が模擬の平均±1標本SDに入ること。
平均の誤差（SD/√N）以内に一致する、という意味ではない。

表の値は各候補の `_xi_summary.csv` から、適合したビン数は Rust の
`_report.md` の計数から転記した。第一成分の幅はすべて10 km。

| 候補 | A1 | A2 | 幅2 km | ξ 3 km | ξ 6 km | ξ 12 km | ξ 24 km | 1 SD内のビン |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| コスモス | | | | 0.1941 | 0.1667 | 0.1458 | 0.0850 | |
| A | 0.5 | 0.65 | 35 | 0.1058±0.0092 | 0.1046±0.0323 | 0.0821±0.0299 | 0.0242±0.0155 | 0/4 |
| B | 0.3 | 0.75 | 35 | 0.0550±0.0712 | 0.0707±0.0268 | 0.0519±0.0283 | 0.0317±0.0204 | 0/4 |
| C | 0.5 | 0.8 | 40 | 0.0844±0.0244 | 0.0950±0.0288 | 0.0748±0.0343 | 0.0220±0.0137 | 0/4 |
| D | 0.3 | 1.0 | 30 | 0.1034±0.0703 | 0.0995±0.0488 | 0.0761±0.0456 | 0.0570±0.0348 | 1/4 |
| E | 0.6 | 0.9 | 20 | 0.1824±0.0869 | 0.1926±0.0641 | 0.1550±0.0507 | 0.0629±0.0372 | 4/4 |
| F | 0.6 | 1.2 | 30 | 0.1432±0.0679 | 0.1516±0.0660 | 0.1116±0.0354 | 0.0480±0.0278 | 3/4 |
| G | 0.3 | 1.2 | 20 | 0.2023±0.1186 | 0.2113±0.1026 | 0.1682±0.0845 | 0.0834±0.0531 | 4/4 |

EとGがこの基準を満たし、24 kmの平均が目標に近いGを本計算に選んだ。
切り捨て後の三点相関については、ξの適合とは別に以下で評価する。
探索の集計ファイルは `data/zeta/field_two_scale_m81_20261114_pilot{A,...,G}_xi_summary.csv`。

## 検定と感度の読み方

コスモスと模擬の比較は、どちらも店から選択関数を作り直す proc 同士。
共分散は不偏標本共分散、精度行列には Hartlap の `(N-p-2)/(N-1)` を掛ける。
実測と推定された模擬平均の差には `1+1/N` の分散の増分も併記する。
χ²分布によるp値は正規近似。共分散を推定したことによるp値のずれは残る。

両側の extreme-rank 検定は実測を含むN+1本を対称に順位付けし、
最小の点ごとの順位でp値の区間を求める。全点の順位を小さい順に並べた
辞書順（ERL）のp値も出す。同順位を含める保守的な定義。
選択関数・ξのパラメータ・有限の点の池に条件付けた比較であり、
パラメータを再推定する無条件の複合帰無仮説の検定ではない。

感度は全ビンを同じ量だけずらし、共分散を変えない場合のGLSの定数成分から求める。
`n_one_mean` は既知の基準平均と比較する独立なカタログの個数、
`n_each_two_means` は二つの独立な模擬集団を比較する際の各集団の個数。
模擬データを増やすことだけでは、コスモス一標本のばらつきは小さくならない。
一律の負シフトは感度評価のためのベクトルの操作であり、負のζを持つ点過程の生成ではない。

切る前の密度場の符号対称性は、切り捨て後・固定点数で抽出したカタログの
真の三点相関が0であることを保証しない。必ず true の平均とその誤差を確認する。
出力の `expected=0` は切る前の模型の名目値であり、true の測定値ではない。
切り捨て割合は池の点のうち `1+δ<0` となった割合で、店の割合ではない。

## 出力の列

| 接尾辞 | 内容 |
|---|---|
| `.csv` | run と三辺のビンごとの true/proc の ζ |
| `_xi.csv` | run と距離ビンごとの true/proc の ξ、run=-1はコスモス |
| `_summary.csv` | ζ の平均・SD・SE、実測の偏差/SD、実測以下の模擬の数 |
| `_xi_summary.csv` | ξ の平均と1標本SD |
| `_true_summary.csv` | true 平均・SD・SE・平均/SE、proc 平均・SE |
| `_diagnostics.csv` | 各runの切り捨て割合（生成時の原本には経過秒数もある） |
| `_cov.csv` | proc の共分散。行列番号は `_summary.csv` の行順 |
| `_tests.csv` | Hartlap χ²、近似p値、順位p区間、ERL、定数シフトの推定 |
| `_sensitivity.csv` | 一律シフト0.01、0.02、0.03、0.05の感度と必要個数 |
| `_report.md` | Rustが集計した表と注意点 |

## 一次資料

- [Hartlap, Simon & Schneider: inverse covariance correction](https://arxiv.org/abs/astro-ph/0608064)
- [Myllymäki et al.: Global envelope tests for spatial processes](https://arxiv.org/abs/1307.0239)

## G 100個の結果

生成模型は `--amp 0.3 --sigma 10 --amp2 1.2 --sigma2 20 --centers 20000`。
run 100–199、4本×25個。各本の累積計算時間は1802.9–1860.1秒で、
最大約31.0分と見積もり内だった。休止を含むwallは12731.96–12789.16秒
（最大約3時間33分）になった。個数は減らしていない。

Gはprocでコスモスと棄却されないが、**ζ≈0の模型ではない**。
trueの平均は全ビンで正で、16/16ビンが0から3標準誤差以上離れた。
切り捨て割合はrun間で5.6606–40.8500%、平均20.4707%。
この模型との一致から、コスモスの物理的なζが0か少し負かを結論することはできない。

ξは4/4ビンで1標本SD以内だが、模型の平均は4ビンともコスモスより大きい。
平均の標準誤差以内の精密なξの適合は達成していない。
procのζは16/16ビンで1標本SD以内（−0.953〜−0.350 SD）。
Hartlap χ²=8.5391、16自由度の正規近似p=0.9312。
模擬平均の不確かさを含めるとχ²=8.4546、近似p=0.9342。
全体の順位p区間は(0.1782, 0.2376]、ERL p=0.2178。
分布の歪みが大きいので、χ²分布による近似pだけを強く解釈しない。

一律シフトの推定値は−0.05108±0.02680（コスモス一標本対、推定された模擬平均）。
同じ共分散を仮定して真の平均を−0.01/−0.02/−0.03ずらす場合、
100個の独立な模擬集団の平均対既知の平均なら3.75/7.50/11.25σ、
100個ずつの独立な二集団なら2.65/5.30/7.95σになる。
一標本の感度は0.375/0.750/1.125σであり、100個の校正用模擬を増やした分だけ
コスモス一標本の判別力が上がるわけではない。

| 負シフトの大きさ | 3σに必要：基準既知の平均 | 3σに必要：二集団の各個数 | 5σに必要：基準既知の平均 | 5σに必要：二集団の各個数 |
|---:|---:|---:|---:|---:|
| 0.01 | 65 | 129 | 178 | 356 |
| 0.02 | 17 | 33 | 45 | 89 |
| 0.03 | 8 | 15 | 20 | 40 |
| 0.05 | 3 | 6 | 8 | 15 |

必要個数はRustの `_sensitivity.csv` の切り上げ値であり、共分散を固定した見積もり。
実際にこの小さな個数で16次元の共分散を校正できる、という意味ではない。
Gのtrueが大きく正にずれたため、比較用に振幅の低いEを独立なrun 300–339で
40個追加し、同じ監査を行う。

以下は `data/zeta/field_two_scale_m81_20261114_mainG_report.md` のRust出力。
### G の Rust 集計表

N=100, bins=16. All comparisons use proc. Standard deviations below describe one catalogue, not the error on the mock mean.

| r km | real xi | true mean | proc mean | proc SD | (real-mean)/SD |
|---:|---:|---:|---:|---:|---:|
| 3 | 0.194078 | 0.803260 | 0.249798 | 0.131230 | -0.425 |
| 6 | 0.166658 | 0.755077 | 0.226704 | 0.127627 | -0.470 |
| 12 | 0.145789 | 0.680913 | 0.198463 | 0.116242 | -0.453 |
| 24 | 0.085019 | 0.509961 | 0.105531 | 0.064276 | -0.319 |

Xi within one catalogue SD: 4/4 (counted by Rust). This is not agreement within the error on the ensemble mean.

Clipped pool fraction across catalogues: 5.6606% to 40.8500%; mean 20.4707%. Individual values: data/zeta/field_two_scale_m81_20261114_mainG_diagnostics.csv.

| sides km | true mean | true SD | true mean SE | true mean / SE | proc mean | proc mean SE |
|---|---:|---:|---:|---:|---:|---:|
| 3,3,3 | 0.676506 | 1.357858 | 0.135786 | +4.982 | 0.053145 | 0.018838 |
| 3,3,6 | 0.634767 | 1.349666 | 0.134967 | +4.703 | 0.045631 | 0.016382 |
| 3,6,6 | 0.564646 | 1.261250 | 0.126125 | +4.477 | 0.039749 | 0.015544 |
| 3,6,12 | 0.495831 | 1.226902 | 0.122690 | +4.041 | 0.034859 | 0.016493 |
| 3,12,12 | 0.411911 | 1.106955 | 0.110696 | +3.721 | 0.023808 | 0.012883 |
| 3,12,24 | 0.414354 | 0.933949 | 0.093395 | +4.437 | 0.022575 | 0.011339 |
| 3,24,24 | 0.278871 | 0.606121 | 0.060612 | +4.601 | 0.008998 | 0.006506 |
| 6,6,6 | 0.510901 | 1.208680 | 0.120868 | +4.227 | 0.029113 | 0.014066 |
| 6,6,12 | 0.429827 | 1.138881 | 0.113888 | +3.774 | 0.023628 | 0.014041 |
| 6,12,12 | 0.364349 | 0.997002 | 0.099700 | +3.654 | 0.018060 | 0.011705 |
| 6,12,24 | 0.369162 | 0.847120 | 0.084712 | +4.358 | 0.016335 | 0.009420 |
| 6,24,24 | 0.247829 | 0.562892 | 0.056289 | +4.403 | 0.004362 | 0.005393 |
| 12,12,12 | 0.283130 | 0.883000 | 0.088300 | +3.206 | 0.008693 | 0.009829 |
| 12,12,24 | 0.303095 | 0.684270 | 0.068427 | +4.429 | 0.011343 | 0.006749 |
| 12,24,24 | 0.211381 | 0.503974 | 0.050397 | +4.194 | 0.002249 | 0.004364 |
| 24,24,24 | 0.172449 | 0.437732 | 0.043773 | +3.940 | 0.000553 | 0.004090 |
True mean range: 0.172449 to 0.676506; bins with |true mean| > 3 SE: 16/16 (uncorrected marginal diagnostics). Proc-minus-true mean range: -0.623362 to -0.171896. Nominal symmetry of the pre-clipping field does not guarantee zero measured true zeta.

Hartlap factor: 0.828283; chi-square: 8.539115; approximate chi-square p: 0.931175. Including uncertainty on the mock mean: 8.454570, approximate p=0.934168.

Extreme-rank two-sided global p interval: (0.178218, 0.237624]; ERL refinement p=0.217822.

Uniform offset GLS: -0.051080 +/- 0.026800 (one observed catalogue versus estimated mock mean). Single-catalogue offset SD: 0.026667; mock-mean SE: 0.002667.

| sides km | real | true mean | proc mean | proc SD | deviation/SD | mocks <= real | rank among N+1 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 3,3,3 | -0.080903 | 0.676506 | 0.053145 | 0.188383 | -0.712 | 5/100 | 6/101 |
| 3,3,6 | -0.110569 | 0.634767 | 0.045631 | 0.163817 | -0.953 | 4/100 | 5/101 |
| 3,6,6 | -0.087835 | 0.564646 | 0.039749 | 0.155435 | -0.821 | 3/100 | 4/101 |
| 3,6,12 | -0.077160 | 0.495831 | 0.034859 | 0.164933 | -0.679 | 5/100 | 6/101 |
| 3,12,12 | -0.059576 | 0.411911 | 0.023808 | 0.128834 | -0.647 | 6/100 | 7/101 |
| 3,12,24 | -0.036192 | 0.414354 | 0.022575 | 0.113391 | -0.518 | 12/100 | 13/101 |
| 3,24,24 | -0.013774 | 0.278871 | 0.008998 | 0.065058 | -0.350 | 23/100 | 24/101 |
| 6,6,6 | -0.090577 | 0.510901 | 0.029113 | 0.140661 | -0.851 | 5/100 | 6/101 |
| 6,6,12 | -0.073017 | 0.429827 | 0.023628 | 0.140414 | -0.688 | 6/100 | 7/101 |
| 6,12,12 | -0.063863 | 0.364349 | 0.018060 | 0.117048 | -0.700 | 6/100 | 7/101 |
| 6,12,24 | -0.050605 | 0.369162 | 0.016335 | 0.094203 | -0.711 | 6/100 | 7/101 |
| 6,24,24 | -0.020099 | 0.247829 | 0.004362 | 0.053929 | -0.454 | 15/100 | 16/101 |
| 12,12,12 | -0.066376 | 0.283130 | 0.008693 | 0.098291 | -0.764 | 6/100 | 7/101 |
| 12,12,24 | -0.040570 | 0.303095 | 0.011343 | 0.067490 | -0.769 | 6/100 | 7/101 |
| 12,24,24 | -0.029223 | 0.211381 | 0.002249 | 0.043640 | -0.721 | 9/100 | 10/101 |
| 24,24,24 | -0.037573 | 0.172449 | 0.000553 | 0.040903 | -0.932 | 5/100 | 6/101 |

Within 1 SD: 16/16 (counted by Rust). Rank uses <=, including ties.

Sensitivity assumes a uniform additive shift and unchanged covariance; it does not generate a physical negative-zeta point process. n_one_mean treats the reference mean as known; n_each_two_means uses two independent ensembles of equal size. These sample sizes count independent catalogues, not extra mocks improving a single observed catalogue. More mocks only reduce calibration uncertainty.

The chi-square p values assume approximately Gaussian vectors and are approximate with estimated covariance. Hartlap is unbiased only for Gaussian independent realizations. Rank calibration is conditional on the fitted selection and chosen xi parameters (plug-in null, not an unconditional composite-null test). A shared finite pool and reference catalogue are conditioned on. Clipping and fixed-count sampling can make true zeta nonzero; consult true means, not just the nominal expected=0 column.

Sources: https://arxiv.org/abs/astro-ph/0608064 ; https://arxiv.org/abs/1307.0239

Inputs:

- real: data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv
- data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_a21.2_s220_m81_20261114_mainG_w0
- data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_a21.2_s220_m81_20261114_mainG_w1
- data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_a21.2_s220_m81_20261114_mainG_w2
- data/zeta/field_jp_2026-11-12_cosmos_a0.3_s10_a21.2_s220_m81_20261114_mainG_w3

## 比較E 40個の結果

`--amp 0.6 --sigma 10 --amp2 0.9 --sigma2 20 --centers 20000`、
run 300–339、4本×10個。Gおよび探索とは重ならない番号を使った。
累積計算時間778.8–857.4秒、wall 778.85–857.38秒（最大約14.3分）。
GとEの本計算の累積時間を合わせても最大約45.3分で、90分以内だった。

Eの切り捨ては8.9463–27.7463%、平均16.8279%。
true平均は0.106112–0.341792で全ビン正、2/16ビンが0から3SE以上。
Gから正のずれは小さくなるが、Eを検証済みのζ≈0模型として扱うこともできない。
procの三点相関は8/16ビンが1標本SD以内、全体のERL p=0.1951で棄却されない。
Hartlap係数0.5641、χ²=10.7773、正規近似p=0.8230。
40個では特に精度行列の校正に不確かさが残る。

一律シフトの推定は−0.039208±0.018562（一標本対、推定された模擬平均）。
Eの共分散を固定すれば、−0.02の差は40個の平均対既知の基準で6.899σ、
40個ずつの独立な二集団で4.878σ、一標本では1.091σ。
3σを見込む個数は基準既知で8、二集団なら各16、5σなら22と各43。
これも物理的な負のζを持つ模型の検定ではなく、定数シフトの感度評価である。

以下は `data/zeta/field_two_scale_m81_20261114_compareE_report.md` のRust出力。
### E の Rust 集計表

N=40, bins=16. All comparisons use proc. Standard deviations below describe one catalogue, not the error on the mock mean.

| r km | real xi | true mean | proc mean | proc SD | (real-mean)/SD |
|---:|---:|---:|---:|---:|---:|
| 3 | 0.194078 | 0.672802 | 0.273405 | 0.150098 | -0.529 |
| 6 | 0.166658 | 0.633975 | 0.249559 | 0.129489 | -0.640 |
| 12 | 0.145789 | 0.564959 | 0.205908 | 0.106666 | -0.564 |
| 24 | 0.085019 | 0.391282 | 0.087266 | 0.058718 | -0.038 |

Xi within one catalogue SD: 4/4 (counted by Rust). This is not agreement within the error on the ensemble mean.

Clipped pool fraction across catalogues: 8.9463% to 27.7463%; mean 16.8279%. Individual values: data/zeta/field_two_scale_m81_20261114_compareE_diagnostics.csv.

| sides km | true mean | true SD | true mean SE | true mean / SE | proc mean | proc mean SE |
|---|---:|---:|---:|---:|---:|---:|
| 3,3,3 | 0.341792 | 0.625658 | 0.098925 | +3.455 | 0.044594 | 0.018387 |
| 3,3,6 | 0.307299 | 0.574613 | 0.090854 | +3.382 | 0.029174 | 0.016285 |
| 3,6,6 | 0.257012 | 0.548159 | 0.086672 | +2.965 | 0.021226 | 0.017498 |
| 3,6,12 | 0.200851 | 0.488846 | 0.077293 | +2.599 | 0.013296 | 0.014713 |
| 3,12,12 | 0.185101 | 0.437461 | 0.069169 | +2.676 | 0.019280 | 0.010560 |
| 3,12,24 | 0.213435 | 0.456934 | 0.072248 | +2.954 | 0.012980 | 0.008844 |
| 3,24,24 | 0.171881 | 0.461165 | 0.072917 | +2.357 | -0.003630 | 0.006342 |
| 6,6,6 | 0.250275 | 0.559481 | 0.088462 | +2.829 | 0.021462 | 0.018967 |
| 6,6,12 | 0.178890 | 0.468551 | 0.074084 | +2.415 | 0.012680 | 0.014533 |
| 6,12,12 | 0.163028 | 0.429159 | 0.067856 | +2.403 | 0.011511 | 0.009803 |
| 6,12,24 | 0.190624 | 0.459526 | 0.072657 | +2.624 | 0.005379 | 0.007142 |
| 6,24,24 | 0.155268 | 0.464413 | 0.073430 | +2.115 | -0.005771 | 0.005842 |
| 12,12,12 | 0.106112 | 0.404927 | 0.064025 | +1.657 | 0.002477 | 0.008881 |
| 12,12,24 | 0.161819 | 0.445091 | 0.070375 | +2.299 | 0.000703 | 0.005874 |
| 12,24,24 | 0.130193 | 0.436573 | 0.069028 | +1.886 | -0.005708 | 0.004627 |
| 24,24,24 | 0.135552 | 0.491200 | 0.077665 | +1.745 | -0.000910 | 0.004313 |
True mean range: 0.106112 to 0.341792; bins with |true mean| > 3 SE: 2/16 (uncorrected marginal diagnostics). Proc-minus-true mean range: -0.297199 to -0.103635. Nominal symmetry of the pre-clipping field does not guarantee zero measured true zeta.

Hartlap factor: 0.564103; chi-square: 10.777279; approximate chi-square p: 0.823019. Including uncertainty on the mock mean: 10.514419, approximate p=0.838421.

Extreme-rank two-sided global p interval: (0.146341, 0.292683]; ERL refinement p=0.195122.

Uniform offset GLS: -0.039208 +/- 0.018562 (one observed catalogue versus estimated mock mean). Single-catalogue offset SD: 0.018335; mock-mean SE: 0.002899.

| sides km | real | true mean | proc mean | proc SD | deviation/SD | mocks <= real | rank among N+1 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 3,3,3 | -0.080903 | 0.341792 | 0.044594 | 0.116292 | -1.079 | 1/40 | 2/41 |
| 3,3,6 | -0.110569 | 0.307299 | 0.029174 | 0.102992 | -1.357 | 1/40 | 2/41 |
| 3,6,6 | -0.087835 | 0.257012 | 0.021226 | 0.110667 | -0.985 | 2/40 | 3/41 |
| 3,6,12 | -0.077160 | 0.200851 | 0.013296 | 0.093056 | -0.972 | 4/40 | 5/41 |
| 3,12,12 | -0.059576 | 0.185101 | 0.019280 | 0.066790 | -1.181 | 3/40 | 4/41 |
| 3,12,24 | -0.036192 | 0.213435 | 0.012980 | 0.055937 | -0.879 | 4/40 | 5/41 |
| 3,24,24 | -0.013774 | 0.171881 | -0.003630 | 0.040112 | -0.253 | 9/40 | 10/41 |
| 6,6,6 | -0.090577 | 0.250275 | 0.021462 | 0.119957 | -0.934 | 3/40 | 4/41 |
| 6,6,12 | -0.073017 | 0.178890 | 0.012680 | 0.091912 | -0.932 | 3/40 | 4/41 |
| 6,12,12 | -0.063863 | 0.163028 | 0.011511 | 0.061999 | -1.216 | 2/40 | 3/41 |
| 6,12,24 | -0.050605 | 0.190624 | 0.005379 | 0.045172 | -1.239 | 3/40 | 4/41 |
| 6,24,24 | -0.020099 | 0.155268 | -0.005771 | 0.036945 | -0.388 | 9/40 | 10/41 |
| 12,12,12 | -0.066376 | 0.106112 | 0.002477 | 0.056170 | -1.226 | 3/40 | 4/41 |
| 12,12,24 | -0.040570 | 0.161819 | 0.000703 | 0.037152 | -1.111 | 4/40 | 5/41 |
| 12,24,24 | -0.029223 | 0.130193 | -0.005708 | 0.029267 | -0.803 | 6/40 | 7/41 |
| 24,24,24 | -0.037573 | 0.135552 | -0.000910 | 0.027281 | -1.344 | 3/40 | 4/41 |

Within 1 SD: 8/16 (counted by Rust). Rank uses <=, including ties.

Sensitivity assumes a uniform additive shift and unchanged covariance; it does not generate a physical negative-zeta point process. n_one_mean treats the reference mean as known; n_each_two_means uses two independent ensembles of equal size. These sample sizes count independent catalogues, not extra mocks improving a single observed catalogue. More mocks only reduce calibration uncertainty.

The chi-square p values assume approximately Gaussian vectors and are approximate with estimated covariance. Hartlap is unbiased only for Gaussian independent realizations. Rank calibration is conditional on the fitted selection and chosen xi parameters (plug-in null, not an unconditional composite-null test). A shared finite pool and reference catalogue are conditioned on. Clipping and fixed-count sampling can make true zeta nonzero; consult true means, not just the nominal expected=0 column.

Sources: https://arxiv.org/abs/astro-ph/0608064 ; https://arxiv.org/abs/1307.0239

Inputs:

- real: data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv
- data/zeta/field_jp_2026-11-12_cosmos_a0.6_s10_a20.9_s220_m81_20261114_compareE_w0
- data/zeta/field_jp_2026-11-12_cosmos_a0.6_s10_a20.9_s220_m81_20261114_compareE_w1
- data/zeta/field_jp_2026-11-12_cosmos_a0.6_s10_a20.9_s220_m81_20261114_compareE_w2
- data/zeta/field_jp_2026-11-12_cosmos_a0.6_s10_a20.9_s220_m81_20261114_compareE_w3

## 残る課題と保存先

- 0で切った後・点数固定の抽出後にも、trueのζが十分0に近いと確認できる帰無模型が必要。
  対称なδを用意するだけでは不十分だった。今回の一致は、その帰無仮説の採択ではない。
- ξの一致は1標本SD内という基準で、平均の誤差以内の精密な一致ではない。
  ξで条件付ける比較や、パラメータを再推定する検定は行っていない。
- χ²のp値と感度のσは正規近似で、特にGの歪んだ分布では限界がある。
  共分散の推定誤差、GとEの模型差も残る。
- 選択関数と有限の点の池・参照カタログを固定した条件付きの比較。
  店とは独立な選択関数の構成、コスモスの座標の精度の問題は今回の対象外。

主な出力の接頭辞：

```text
data/zeta/field_two_scale_m81_20261114_mainG
data/zeta/field_two_scale_m81_20261114_compareE
data/zeta/field_two_scale_m81_20261114_pilot{A,B,C,D,E,F,G}
data/zeta/field_two_scale_m81_20261114_pilot{E,G}_audit
data/zeta/field_m81_20261114_legacy20
```

主な表は上記の `_summary.csv`、`_xi_summary.csv`、`_true_summary.csv`、
`_diagnostics.csv`、`_tests.csv`、`_sensitivity.csv`。
GとEの原本は各Rust出力末尾のInputsに記載した4本ずつのCSV。
共有データの実体は `/Users/tatsukikodama/inuverse/repos/retail-cosmology/data/zeta/`。
各workerの実行ログは、このworktreeの `target/field-mocks-logs/m81_20261114_{mainG,compareE}_w{0,1,2,3}.log`。
既存データ・既存図・ノートのTeXは上書きしていない。

実装コミット：`4de5e96`、ブランチ `feat/81-more-mocks`。
`cargo test -j 2 -- --test-threads=2`（58件）、
`cargo clippy -j 2 --all-targets -- -D warnings`、
`scripts/check_layers.sh crates/retail-cosmology/src`、`bash -n`、`git diff --check`を通過。
層の検査で見つかった既存の2件は、costの既定参照点数を自身の設定とし、
var_ddのWeightをManager経由の参照にする最小修正で解消した。
新しいEngine・ResourceAccessは作っていない。push・PRは行っていない。

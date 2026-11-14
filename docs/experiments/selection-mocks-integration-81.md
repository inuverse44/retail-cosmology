# 選択関数と模擬データの統合確認

`feat/81-selection-and-mocks` の `1e5206d` を基点に `feat/81-integrate` を作り、
`feat/81-mock-density` の `853dc39` をマージした。

`zeta_clustered` は `--regional on|prefecture` と `--match on` を併用できる。
従来の生成・true/proc/expected の乱数列を保ち、regional と match はそれぞれの独立な乱数列を使う。
CSV の列は、従来列、regional の列、match の列の順。
単独指定時の形式も保つ。rc-mocks 側の FieldModel、場の二成分、全体検定、監査とテストは維持した。

層違反の修正は統合ブランチ側にそろえた（cost の定数、variance を通した Weight の公開）。
ADR 008 は統合ブランチ、ADR 009 は mock-density ブランチのファイルとバイト一致し、本文は変更していない。
ADR の一覧には両方を載せた。

小さな人口・行政メッシュ・店舗の実ファイルを作るテストで、基準を指定しない場合、regional のみ、match のみ、
両方の場合を測定し、単独時の全ての列が併用時の同名の列とバイト一致することを確認した。

確認コマンド：

```sh
cargo test --jobs 2 -- --test-threads=1
cargo clippy --all-targets --jobs 2 -- -D warnings
scripts/check_layers.sh crates/retail-cosmology/src
cargo build --release --jobs 2 --bin zeta --bin zeta_clustered
```

テスト 65 件、clippy、層チェックは全て通過した。
実データの重い計算は次の二つを順番に実行し、同時には一つだけ動かした。
共有 data の既存ファイルを守るため、再実行時には未使用のタグを使う。

```sh
C=data/cosmos_site/cosmos_site_clean_2026-11-12.csv
target/release/zeta 2026-11-12 --country jp --stores "$C" --chain COSMOS --footprint 50 --xi on --n-random 8000 --n-ref 50000 --seed 1 --tag integrate_fp50
target/release/zeta_clustered 2026-11-12 --country jp --stores "$C" --chain COSMOS --footprint 50 --regional prefecture --match on --children 14 --sigma 15 --runs 1 --n-random 8000 --n-ref 50000 --seed 1 --tag integrate_both
```

R=50 の実測 ζ は、既存の `data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv` とバイト一致した。

併用した runs=1 で次の全てを `cmp` によりバイト一致で確認した。

- 従来の先頭 10 列：mdens20 の run=0 の先頭 10 列と一致。
- regional を含む先頭 11 列：ext_prefecture_mocks の run=0 の全列と一致。
- match を含む列（1–10、12–13）：mdens20 の run=0 の全列と一致。
- ξ の regional を含む先頭 6 列：ext_prefecture_mocks の run=-1 と run=0 と一致。
- ξ の match を含む列（1–5、7）：mdens20 の run=-1 と run=0 と一致。
- `_density_cells.csv` と `_density_summary.csv`：mdens_bench の 1 回分と一致。

照合の手順（比較用の一時ファイルは /private/tmp に置く）：

```sh
A=data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_integrate_both
M=data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_mdens20
R=data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_ext_prefecture_mocks
awk -F, 'NR==1 || $1==0' "$M.csv" > /private/tmp/ref_mdens.csv
awk -F, 'NR==1 || $1==0' "$R.csv" > /private/tmp/ref_regional.csv
cut -d, -f1-10 "$A.csv" > /private/tmp/actual10.csv
cut -d, -f1-10 /private/tmp/ref_mdens.csv > /private/tmp/reference10.csv
cmp /private/tmp/actual10.csv /private/tmp/reference10.csv
cut -d, -f1-11 "$A.csv" > /private/tmp/actual_regional.csv
cmp /private/tmp/actual_regional.csv /private/tmp/ref_regional.csv
cut -d, -f1-10,12-13 "$A.csv" > /private/tmp/actual_mdens.csv
cmp /private/tmp/actual_mdens.csv /private/tmp/ref_mdens.csv
```

R=50 の実測は外部 time で real 14.16 秒、併用の 1 回は real 195.41 秒。
併用の内部タイマーは Instant 約 195.0 秒、壁時計約 195.0 秒。
すべて新規の integrate タグで出力し、既存の共有出力は上書きしていない。

出力は以下の接頭辞。併用の `_timing.csv` には内部タイマーと壁時計の記録がある。

- `data/zeta/zeta_jp_2026-11-12_cosmos_integrate_fp50.csv` と `_xi.csv`
- `data/zeta/clustered_jp_2026-11-12_cosmos_m14_s15_integrate_both.csv`、`_xi.csv`、`_density_cells.csv`、`_density_summary.csv`、`_timing.csv`

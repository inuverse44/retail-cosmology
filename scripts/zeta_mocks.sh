#!/usr/bin/env bash
# 三点相関関数の帰無仮説の模擬データを、種を変えて続けて作る（#81、ADR 007）。
# 店の代わりに、基準（人口 × 選択関数）と同じ重みで独立に撒いた点を使う。ランダムカタログは実際の店のときと同じ。
# 使い方：scripts/zeta_mocks.sh <日付> <国> <店のファイル> <チェーン> <最初の種> <最後の種> [zeta に渡す引数 ...]
# 例：scripts/zeta_mocks.sh 2026-11-12 jp data/cosmos_site/cosmos_site_clean_2026-11-12.csv COSMOS 1 10 --n-random 8000 --footprint 50
# 書くもの：data/zeta/zeta_<国>_<日付>_<チェーン>_mock<種>[...].csv（--tag を渡したときはその後ろに _mock<種>）
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
date="$1" country="$2" stores="$3" chain="$4" first="$5" last="$6"
shift 6
tag="mock"
args=()
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--tag" ]; then tag="$2_mock"; shift 2; else args+=("$1"); shift; fi
done
cargo build -q --release --bin zeta
for s in $(seq "$first" "$last"); do
  target/release/zeta "$date" --country "$country" --stores "$stores" --chain "$chain" \
    "${args[@]}" --mock "$s" --tag "${tag}${s}" 2>&1 | grep -v '^COSMOS\|^WALMART' || true
done

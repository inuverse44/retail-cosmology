#!/usr/bin/env bash
# 帰無仮説の模擬データ（選択関数に従って独立に撒いた店）で、選択関数つきの ξ を何度も測り、
# 実際の店の値と比べるための分布を作る。
# 使い方：scripts/null_mocks.sh <日付> <国> <R [km]> <個数>
#   例：scripts/null_mocks.sh 2026-11-03 jp 50 100
# 出力：data/xi/null_mocks_<国>_<R>.txt（模擬データごとに、χ² の行とまとめた値の行）
set -euo pipefail
date="${1:?日付}"; country="${2:?国}"; r="${3:?R}"; n="${4:?個数}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
out="data/xi/null_mocks_${country}_${r}.txt"
: > "$out"
for s in $(seq 1 "$n"); do
  target/release/xi "$date" --country "$country" --footprint "$r" --mock "$r:$s" --tag "nullmock" 2>/dev/null
  { echo "== 種 $s"; grep "footprint_${r}" "data/xi/chi2_$( [ "$country" = jp ] && echo jp_ )${date}_nullmock.txt"; } >> "$out"
done
echo "$out"

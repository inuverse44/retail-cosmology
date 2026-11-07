#!/usr/bin/env bash
# 帰無仮説の模擬データ（選択関数に従って独立に撒いた店）で、選択関数つきの ξ を何度も測り、
# 実際の店の値と比べるための分布を作る。
# 使い方：scripts/null_mocks.sh <日付> <国> <R [km]> <個数> [並列の数]
#   例：scripts/null_mocks.sh 2026-11-03 jp 50 100 6
# 出力：data/xi/null_mocks_<国>_<R>.txt（模擬データごとに、χ² の行とまとめた値の行）
set -euo pipefail
export LC_ALL=C  # 日本語の行を、バイト列のまま grep する
date="${1:?日付}"; country="${2:?国}"; r="${3:?R}"; n="${4:?個数}"; jobs="${5:-1}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
prefix=""; [ "$country" = jp ] && prefix="jp_"
work="$(mktemp -d)"
export date country r prefix work
one() {
  local s="$1"
  target/release/xi "$date" --country "$country" --footprint "$r" --mock "$r:$s" --tag "nullmock$s" 2>/dev/null
  { echo "== 種 $s"; grep -F "footprint_${r}" "data/xi/chi2_${prefix}${date}_nullmock$s.txt"; } > "$work/$s.txt"
  rm "data/xi/chi2_${prefix}${date}_nullmock$s.txt" "data/xi/xi_${prefix}${date}_nullmock$s.csv"
}
export -f one
seq 1 "$n" | xargs -P "$jobs" -n 1 bash -c 'one "$0"'
out="data/xi/null_mocks_${country}_${r}.txt"
for s in $(seq 1 "$n"); do cat "$work/$s.txt"; done > "$out"
rm -r "$work"
echo "$out"

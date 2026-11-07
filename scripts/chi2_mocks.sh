#!/usr/bin/env bash
# χ² の上側確率を校正するための、帰無仮説の模擬データの χ² を集める（#9）。
# R = 0 なら店を人口だけに比例して撒き（人口が基準の χ²）、R > 0 なら幅 R の選択関数に従って撒く（選択関数つきの χ²）。
# 選択関数は約 10 km の格子の表で求める（--selection-grid on、#12）。
# 使い方：scripts/chi2_mocks.sh <日付> <au|jp> <R [km]> <個数> <並列の数> [店のファイル,…] [A,B]
#   例：scripts/chi2_mocks.sh 2026-11-01 au 0 100 6
#       scripts/chi2_mocks.sh 2026-11-03 jp 200 100 6 $S TRIAL,SEIYU
# 出力：data/xi/chi2_mocks_<国>_<R>.txt（模擬データごとに「== 種 s」と、その R の χ² の行）
set -euo pipefail
export LC_ALL=C  # 日本語の行を、バイト列のまま grep する
date="${1:?日付}"; country="${2:?国}"; r="${3:?R}"; n="${4:?個数}"; jobs="${5:?並列の数}"
stores="${6:-}"; pair="${7:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
extra="--country $country"
[ -n "$stores" ] && extra="$extra --stores $stores"
[ -n "$pair" ] && extra="$extra --cross $pair"
if [ "$r" = 0 ]; then key="population"; else extra="$extra --footprint $r --selection-grid on"; key="footprint_$r"; fi
prefix=""; [ "$country" = jp ] && prefix="jp_"
work="$(mktemp -d)"
export date r extra key prefix work
one() {
  local s="$1" tag="chi2mock_${country:-}_${r}_$1"
  target/release/xi "$date" $extra --mock "$r:$s" --tag "$tag" 2>/dev/null
  { echo "== 種 $s"; grep -F "/ $key：" "data/xi/chi2_${prefix}${date}_$tag.txt"; } > "$work/$s.txt"
  rm "data/xi/chi2_${prefix}${date}_$tag.txt" "data/xi/xi_${prefix}${date}_$tag.csv"
}
export -f one
export country
seq 1 "$n" | xargs -P "$jobs" -n 1 bash -c 'one "$0"'
out="data/xi/chi2_mocks_${country}_${r}.txt"
for s in $(seq 1 "$n"); do cat "$work/$s.txt"; done > "$out"
rm -r "$work"
echo "$out"

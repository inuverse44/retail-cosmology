#!/usr/bin/env bash
# ビンのまとめ方（GLS、単純な平均、分散の逆数の重みの平均）を、乱数の種を変えて比べる（#18）。
# 使い方：scripts/bin_means.sh <日付> <au|jp> <種の数> <並列の数> [店のファイル,…] [A,B]
#   例：scripts/bin_means.sh 2026-11-03 jp 6 6 $S TRIAL,SEIYU
#       scripts/bin_means.sh 2026-11-01 au 3 3
# 選択関数は点ごとに求める（ノートと同じ方法）。
# 出力：data/xi/bin_means_<国>.tsv（種, 量, R, まとめ方, 値, 誤差）
set -euo pipefail
export LC_ALL=en_US.UTF-8
date="${1:?日付}"; country="${2:?国}"; ns="${3:?種の数}"; jobs="${4:?並列の数}"; stores="${5:-}"; pair="${6:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
extra="--country $country --footprint 50,100,200"
[ -n "$stores" ] && extra="$extra --stores $stores"
[ -n "$pair" ] && extra="$extra --cross $pair"
prefix=""; [ "$country" = jp ] && prefix="jp_"
work="$(mktemp -d)"
export date extra prefix work country
one() {
  local s="$1" tag="binmeans_${country}_$1"
  target/release/xi "$date" $extra --seed "$s" --tag "$tag" 2>/dev/null
  scripts/bin_means_parse.pl < "data/xi/chi2_${prefix}${date}_$tag.txt" | sed "s/^/$s\t/" > "$work/$s.tsv"
  rm "data/xi/chi2_${prefix}${date}_$tag.txt" "data/xi/xi_${prefix}${date}_$tag.csv"
}
export -f one
seq 1 "$ns" | xargs -P "$jobs" -n 1 bash -c 'one "$0"'
out="data/xi/bin_means_${country}.tsv"
{ printf 'seed\tquantity\tR_km\tmethod\tvalue\tsigma\n'; cat "$work"/*.tsv; } > "$out"
rm -r "$work"
echo "$out"

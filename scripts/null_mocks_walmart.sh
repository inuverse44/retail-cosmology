#!/usr/bin/env bash
# Walmart（米国、#51）の選択関数つきの ξ を、帰無仮説の模擬データと比べるための分布を作る。
# R = 50、100 km、地方だけ・密集地への寄り方もそろえた模擬データを 100 個ずつ。
# 選択関数は 10 km 格子の表で求める（付録 app:grid の近似）。模擬データ 1 個は 1 本で走らせて約 16 秒（2026-11-13 に測った）。
# 使い方：scripts/null_mocks_walmart.sh <日付> [個数] [並列の数]
#   例：scripts/null_mocks_walmart.sh 2026-11-13 100 6（6 並列で約 32 分。並列では 1 個が遅くなる）
# 出力：data/xi/null_mocks_us_WALMART_<R>[_density]_grid.txt（模擬データごとに、χ² の行とまとめた値の行）
#       同じ名前の _bins.csv（模擬データごとの、選択関数つきの ξ_LS のビンごとの値）
set -euo pipefail
date="${1:?日付}"; n="${2:-100}"; jobs="${3:-6}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
work="$(mktemp -d)"
export date work
one() {
  local r="$1" mode="$2" s="$3" extra="--selection-grid on"
  [ "$mode" = density ] && extra="$extra --mock-density on"
  local tag="usmock_${r}_${mode}_$s"
  # $extra は空白で分けて渡す（bash なので、引用符なしで単語に分かれる）
  target/release/xi "$date" --country us --footprint "$r" --mock "$r:$s" $extra --tag "$tag" 2>/dev/null
  { echo "== 種 $s"; grep "footprint_$r" "data/xi/chi2_us_${date}_$tag.txt"; } > "$work/${r}_${mode}_$s.txt"
  grep -E "^ls,[^,]+,footprint_$r," "data/xi/xi_us_${date}_$tag.csv" \
    | cut -d, -f1,2,4- | sed "s/^/$s,/" > "$work/${r}_${mode}_$s.csv"
  rm "data/xi/chi2_us_${date}_$tag.txt" "data/xi/xi_us_${date}_$tag.csv"
}
export -f one
for r in 50 100; do for mode in plain density; do for s in $(seq 1 "$n"); do echo "$r $mode $s"; done; done; done \
  | xargs -P "$jobs" -n 3 bash -c 'one "$0" "$1" "$2"'
for r in 50 100; do
  for mode in plain density; do
    suffix="_grid"; [ "$mode" = density ] && suffix="_density_grid"
    out="data/xi/null_mocks_us_WALMART_${r}${suffix}.txt"
    for s in $(seq 1 "$n"); do cat "$work/${r}_${mode}_$s.txt"; done > "$out"
    { echo "seed,quantity,sample,r_lo_km,r_hi_km,value,sigma"; for s in $(seq 1 "$n"); do cat "$work/${r}_${mode}_$s.csv"; done; } > "${out%.txt}_bins.csv"
    echo "${out}（$(grep -c '^== 種' "$out") 個）"
  done
done
rm -r "$work"

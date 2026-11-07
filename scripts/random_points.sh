#!/usr/bin/env bash
# 選択関数つきのランダムカタログの点の数（--n-random）、参照の点の数（--n-ref）、種（--seed）を変えて、
# トライアル、西友、相互相関のまとめた値がどれだけ動くかを測る（#11）。
# 使い方：scripts/random_points.sh <日付> <店のファイル,…> <A,B> <並列の数> <n_random:n_ref:種の数> …
#   例：scripts/random_points.sh 2026-11-03 $S TRIAL,SEIYU 6 20000:50000:10 50000:50000:10
#   環境変数 EXACT=1 なら選択関数を格子の表ではなく点ごとに求める（ノートと同じ方法、遅い）。
# 出力：data/xi/random_points[_exact].tsv（n_random, n_ref, 種, 量, R, 値, 誤差）
set -euo pipefail
export LC_ALL=en_US.UTF-8
date="${1:?日付}"; stores="${2:?店のファイル}"; pair="${3:?A,B}"; jobs="${4:?並列の数}"; shift 4
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
grid="--selection-grid on"; suffix=""
if [ "${EXACT:-0}" = 1 ]; then grid=""; suffix="_exact"; fi
work="$(mktemp -d)"
export date stores pair work grid
one() {
  local nr="$1" nref="$2" s="$3" tag="rp_${1}_${2}_${3}${suffix:-}"
  target/release/xi "$date" --country jp --stores "$stores" --cross "$pair" --footprint 50,100,200 \
    --n-random "$nr" --n-ref "$nref" --seed "$s" $grid --tag "$tag" 2>/dev/null
  # 例：「TRIAL の ξ_LS（footprint_50）：8.0–20.0 km の 2 ビンをまとめた値 0.118 ± 0.061」
  perl -CSD -Mutf8 -ne 'print "$1\t$3\t$4\t$5\n" if /^([A-Za-z]+) の ?(相互相関 )?ξ_LS（footprint_(\d+)）：.*まとめた値 (-?[\d.]+) ± ([\d.]+)$/' \
    "data/xi/chi2_jp_${date}_${tag}.txt" | sed "s/^/$nr\t$nref\t$s\t/" > "$work/$tag.tsv"
  rm "data/xi/chi2_jp_${date}_${tag}.txt" "data/xi/xi_jp_${date}_${tag}.csv"
}
export -f one
export suffix
for spec in "$@"; do
  IFS=: read -r nr nref ns <<< "$spec"
  for s in $(seq 1 "$ns"); do echo "$nr $nref $s"; done
done | xargs -P "$jobs" -n 3 bash -c 'one "$0" "$1" "$2"'
out="data/xi/random_points${suffix}.tsv"
{ printf 'n_random\tn_ref\tseed\tquantity\tR_km\tvalue\tsigma\n'; cat "$work"/*.tsv | sort -n -k1,1 -k2,2 -k3,3; } > "$out"
rm -r "$work"
echo "$out"

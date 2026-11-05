#!/usr/bin/env bash
# 帰無仮説の模擬データで、二つのチェーンの相互相関（選択関数つき）の分布を作る。
# 模擬データでは、チェーンごとに、実際の店から作った幅 R の選択関数に従って、店と同じ数の点を独立に撒く。
# つまり「二社は出店する地方を選ぶが、地方の中では互いに無関係に店を置く」という帰無仮説である。
# 使い方：scripts/null_mocks_cross.sh <日付> <店のファイル,…> <A,B> <R [km]> <個数> [並列の数] [density,grid]
#   7 つ目の引数は、density、grid、keep:<チェーン> をカンマで区切って並べる（どれか一つでもよい）。
#   keep:TRIAL を渡すと、トライアルは撒き直さず実際の位置のまま使う（xi の --mock-keep、#13）。
#   実際の店の固まりを残したまま、もう一方のチェーンを無関係に撒いた帰無仮説になる。
#   grid を渡すと、選択関数を約 10 km の格子ごとに一度だけ求める近似を使う（xi の --selection-grid on、#12）。
#   結果の差はランダムカタログの揺らぎより小さく、模擬データ 1 個が約 6 分の 1 の時間になる（notes/algorithm-cost.tex）。
#   density を渡すと、模擬データの重みに人口密度 ρ の γ 乗も掛け、店の位置の ln ρ の平均を実際の店にそろえる
#   （xi の --mock-density on）。「二社は出店する地方と、人の密集地への寄り方を選ぶが、互いには無関係に店を置く」
#   という帰無仮説になる。
#   例：scripts/null_mocks_cross.sh 2026-11-03 data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv TRIAL,SEIYU 50 100 6
# 出力：data/xi/null_mocks_cross_<A>x<B>_<R>[_density][_grid][_keep<チェーン>].txt（模擬データごとに、A・B それぞれの ξ と相互相関の、χ² の行とまとめた値の行）
set -euo pipefail
date="${1:?日付}"; stores="${2:?店のファイル}"; pair="${3:?A,B}"; r="${4:?R}"; n="${5:?個数}"; jobs="${6:-4}"; mode="${7:-}"
extra=""; suffix=""
case ",$mode," in *,density,*) extra="$extra --mock-density on"; suffix="${suffix}_density";; esac
case ",$mode," in *,grid,*) extra="$extra --selection-grid on"; suffix="${suffix}_grid";; esac
keep="$(echo ",$mode," | grep -oE ',keep:[A-Za-z]+,' | tr -d ',' | cut -d: -f2 || true)"
if [ -n "$keep" ]; then extra="$extra --mock-keep $keep"; suffix="${suffix}_keep${keep}"; fi
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
label="${pair/,/x}"
work="$(mktemp -d)"
export date stores pair r work extra
one() {
  local s="$1"
  target/release/xi "$date" --country jp --stores "$stores" --cross "$pair" --footprint "$r" --mock "$r:$s" $extra --tag "crossmock$s" 2>/dev/null
  { echo "== 種 $s"; grep "footprint_$r" "data/xi/chi2_jp_${date}_crossmock$s.txt"; } > "$work/$s.txt"
  rm "data/xi/chi2_jp_${date}_crossmock$s.txt" "data/xi/xi_jp_${date}_crossmock$s.csv"
}
export -f one
seq 1 "$n" | xargs -P "$jobs" -n 1 bash -c 'one "$0"'
out="data/xi/null_mocks_cross_${label}_${r}${suffix}.txt"
for s in $(seq 1 "$n"); do cat "$work/$s.txt"; done > "$out"
rm -r "$work"
echo "$out"

#!/usr/bin/env bash
# トライアルの公式サイトの店舗情報（店名、業態、緯度・経度、都道府県）を取ってくる。
# 使い方：scripts/trial_site/fetch.sh
#
# 店の番号は店舗一覧（https://www.trial-net.co.jp/shops）のリンクから集め、
# サイトの地図が使う API（/wp-json/api/store?store_ids=…）に 50 店ずつ問い合わせる。
# サイトの利用条件（https://trial-holdings.inc/legal/terms/）に従い、間隔をあけて少ない回数で取る。
# 取ったデータの著作権はトライアルグループにあるので、リポジトリにはコミットしない（data/ に置く）。
#
# 出力：data/trial_site/<取得日>/ids.txt、batch_*.json
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
day="$(date +%Y-%m-%d)"
out="$root/data/trial_site/$day"
mkdir -p "$out"
ua="retail-cosmology research script"
base="https://www.trial-net.co.jp"

curl -sSL -A "$ua" "$base/shops" | grep -oE 'href="/shops/[0-9]+"' | grep -oE '[0-9]+' | sort -n -u > "$out/ids.txt"
echo "店の番号 $(wc -l < "$out/ids.txt" | tr -d ' ') 個"

split -l 50 "$out/ids.txt" "$out/ids_part_"
i=0
for part in "$out"/ids_part_*; do
  sleep 1
  ids="$(paste -sd, "$part")"
  curl -sSL -A "$ua" "$base/wp-json/api/store?store_ids=$ids" -o "$out/batch_$i.json"
  i=$((i + 1))
done
rm "$out"/ids_part_*
echo "$out に $i 個のファイル"

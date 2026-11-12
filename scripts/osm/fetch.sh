#!/usr/bin/env bash
# OpenStreetMap から店舗を取り出す。
# 使い方：scripts/osm/fetch.sh 2026-11-01 [stores|trial]
#   stores：Coles と Woolworths（オーストラリア、既定）
#   trial：トライアル（日本）
#   cosmos：コスモス（日本、#51）
# 出力：data/osm/<名前>_<日付>.tsv（OSM のデータは ODbL。© OpenStreetMap contributors）
set -euo pipefail
date="${1:?取り出す日付（YYYY-MM-DD）}"
name="${2:-stores}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/osm/${name}_${date}.tsv"
mkdir -p "$(dirname "$out")"
query="$(sed "s/{{DATE}}/${date}T00:00:00Z/" "$root/scripts/osm/${name}.overpassql")"
endpoint="${OVERPASS_ENDPOINT:-https://overpass-api.de/api/interpreter}"
wait=30
for attempt in 1 2 3 4 5; do
  # Overpass の時間切れ（1800 秒）より少し長く待って、返ってこなければやり直す
  curl -sS --max-time 2000 -A "retail-cosmology research script" --data-urlencode "data=$query" "$endpoint" -o "$out.part" || true
  if head -1 "$out.part" | grep -q '^@type'; then
    # 時間切れのときも途中までの結果が返るので、州・都道府県の数がそろっているかを確かめる
    areas=$(awk -F'\t' '$1=="area"' "$out.part" | wc -l | tr -d ' ')
    case "$name" in stores) want=8 ;; trial|cosmos) want=47 ;; *) want=$areas ;; esac
    if [ "$areas" != "$want" ]; then
      echo "領域が $areas 個しかない（$want 個のはず）。時間切れの可能性" >&2
      sleep "$wait"; wait=$((wait * 2)); continue
    fi
    mv "$out.part" "$out"
    echo "$out ($(($(wc -l < "$out") - 1)) 行)"
    exit 0
  fi
  echo "Overpass が混んでいる（$attempt 回目）。${wait} 秒待つ" >&2
  sleep "$wait"; wait=$((wait * 2))
done
echo "取り出せなかった。$out.part を見る" >&2
exit 1

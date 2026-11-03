#!/usr/bin/env bash
# OpenStreetMap から店舗を取り出す。使い方：scripts/osm/fetch.sh 2026-11-01
# 出力：data/osm/stores_<日付>.tsv（OSM のデータは ODbL。© OpenStreetMap contributors）
set -euo pipefail
date="${1:?取り出す日付（YYYY-MM-DD）}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/osm/stores_${date}.tsv"
mkdir -p "$(dirname "$out")"
query="$(sed "s/{{DATE}}/${date}T00:00:00Z/" "$root/scripts/osm/stores.overpassql")"
endpoint="${OVERPASS_ENDPOINT:-https://overpass-api.de/api/interpreter}"
wait=30
for attempt in 1 2 3 4 5; do
  curl -sS -A "retail-cosmology research script" --data-urlencode "data=$query" "$endpoint" -o "$out.part"
  if head -1 "$out.part" | grep -q '^@type'; then
    mv "$out.part" "$out"
    echo "$out ($(($(wc -l < "$out") - 1)) 行)"
    exit 0
  fi
  echo "Overpass が混んでいる（$attempt 回目）。${wait} 秒待つ" >&2
  sleep "$wait"; wait=$((wait * 2))
done
echo "取り出せなかった。$out.part を見る" >&2
exit 1

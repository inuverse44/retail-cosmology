#!/usr/bin/env bash
# e-Stat の令和2年国勢調査 1 km メッシュ（世界測地系、人口及び世帯、T001140）を、1 次メッシュごとに取ってくる。
# 出典：総務省統計局「令和2年国勢調査 地域メッシュ統計」（e-Stat、政府標準利用規約 第2.0版）
#
# 1 次メッシュの番号の一覧（mesh1_codes.txt）は、OSM の集落の点（places.overpassql）が入る 1 次メッシュとして作った。
# 出力：data/estat/tblT001140S<番号>.txt（Shift_JIS の CSV）
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/estat"
mkdir -p "$out"
while read -r code; do
  [ -z "$code" ] && continue
  if [ -f "$out/tblT001140S$code.txt" ]; then continue; fi
  url="https://www.e-stat.go.jp/gis/statmap-search/data?statsId=T001140&code=$code&downloadType=2"
  curl -sSL -A "retail-cosmology research script" -o "$out/$code.zip" "$url"
  if unzip -o -q "$out/$code.zip" -d "$out" 2>/dev/null; then
    rm "$out/$code.zip"
  else
    echo "${code}：データなし" >&2
    rm -f "$out/$code.zip"
  fi
  sleep 1
done < "$root/scripts/estat/mesh1_codes.txt"
echo "$(ls "$out"/tblT001140S*.txt | wc -l) 個の 1 次メッシュ"

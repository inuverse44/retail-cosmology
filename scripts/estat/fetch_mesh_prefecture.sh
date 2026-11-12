#!/usr/bin/env bash
# 総務省統計局「市区町村別メッシュ・コード一覧」（https://www.stat.go.jp/data/mesh/m_itiran.html）を、都道府県ごとに取ってくる（#51）。
# 1 km 格子（3 次メッシュ）がどの都道府県に入るかを決め、九州の中だけの測定でランダムカタログを九州の格子に限るのに使う。
# 出典：総務省統計局（政府標準利用規約 第2.0版）
# 出力：data/estat/mesh_pref/<01〜47>.csv（Shift_JIS の CSV）
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/estat/mesh_pref"
mkdir -p "$out"
for n in $(seq -w 1 47); do
  if [ -f "$out/$n.csv" ]; then continue; fi
  curl -sSf --max-time 60 -A "retail-cosmology research script" -o "$out/$n.csv" "https://www.stat.go.jp/data/mesh/csv/$n.csv"
  sleep 1
done
echo "$(ls "$out"/*.csv | wc -l | tr -d ' ') 個（${out}）"

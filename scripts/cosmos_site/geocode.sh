#!/usr/bin/env bash
# コスモスの店の住所から座標を推定する（#51）。国土地理院の住所検索 API（https://msearch.gsi.go.jp/address-search/）を使う。
# 使い方：scripts/cosmos_site/geocode.sh <日>
#   読むもの：data/cosmos_site/<日>/addresses.tsv（id、住所。cargo run --release --bin cosmos_census が書く）
#   書くもの：data/cosmos_site/<日>/geocode.tsv（id、経度、緯度、当たった住所）。すでに引いた id は飛ばすので、途中で止めても続きから引ける。
# 住所検索は、当たった住所の最初の候補を使う。丁目のある住所は街区（番）まで、大字と地番の住所は大字までしか当たらないことが多い。
# 間隔を 1 秒あけて、1 件ずつ問い合わせる。
#
# コスモスの公式サイト（https://www.cosmospc.co.jp/shop/）はスクリプトからの取得を受け付けないので、
# 店舗一覧のページはブラウザで保存して data/cosmos_site/<日>/ に置いた。サイトの内容はリポジトリにコミットしない（data/ に置く）。
set -euo pipefail
day="${1:?ページを保存した日（YYYY-MM-DD）}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
dir="$root/data/cosmos_site/$day"
in="$dir/addresses.tsv"
out="$dir/geocode.tsv"
touch "$out"
ua="retail-cosmology research script"
n=0
fail=0
while IFS=$'\t' read -r id addr; do
  if grep -q "^${id}"$'\t' "$out"; then continue; fi
  json="$(curl -sS --max-time 30 -A "$ua" -G --data-urlencode "q=$addr" https://msearch.gsi.go.jp/address-search/AddressSearch || true)"
  line="$(printf '%s' "$json" | python3 -I -c '
import json, sys
try:
    r = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if r:
    lon, lat = r[0]["geometry"]["coordinates"]
    title = r[0]["properties"]["title"]
    print(f"{lon}\t{lat}\t{title}")
')"
  if [ -n "$line" ]; then
    printf '%s\t%s\n' "$id" "$line" >> "$out"
    fail=0
  else
    echo "当たらなかった：$id $addr" >&2
    fail=$((fail + 1))
    # 続けて当たらないときは、住所ではなくサーバーの側の問題とみて止める（あとで続きから引ける）
    if [ "$fail" -ge 5 ]; then echo "5 件続けて当たらなかったので止める" >&2; exit 1; fi
  fi
  n=$((n + 1))
  sleep 1
done < "$in"
echo "$n 件を問い合わせた。$(wc -l < "$out" | tr -d ' ') / $(wc -l < "$in" | tr -d ' ') 件の座標がある（${out}）"

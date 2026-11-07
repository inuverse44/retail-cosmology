#!/usr/bin/env bash
# 店の地図に描く海岸線（Natural Earth 1:10m Coastline、v5.1.2）を取ってくる。
# 出典：Natural Earth（パブリックドメイン）。https://www.naturalearthdata.com/
# 出力：data/naturalearth/ne_10m_coastline.geojson
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/naturalearth"
url="https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/ne_10m_coastline.geojson"
sha256="6f75ae0e0de157b14946e2255eb1f5486d9a13819032e26d4610852d296788f6"
mkdir -p "$out"
curl -sSL -A "retail-cosmology research script" -o "$out/ne_10m_coastline.geojson" "$url"
echo "$sha256  $out/ne_10m_coastline.geojson" | shasum -a 256 -c -

#!/usr/bin/env bash
# 店の地図に描く海岸線（Natural Earth 1:10m Coastline、v5.1.2）と、
# 米国の州境（1:10m Admin 1 – States, Provinces、v5.1.2。人口の格子を米国本土に限り、店を州に割り振るのに使う、#51）を取ってくる。
# 出典：Natural Earth（パブリックドメイン）。https://www.naturalearthdata.com/
# 出力：data/naturalearth/ne_10m_coastline.geojson、data/naturalearth/ne_10m_admin_1_states_provinces.geojson
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/naturalearth"
url="https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/ne_10m_coastline.geojson"
sha256="6f75ae0e0de157b14946e2255eb1f5486d9a13819032e26d4610852d296788f6"
mkdir -p "$out"
curl -sSL --max-time 300 -A "retail-cosmology research script" -o "$out/ne_10m_coastline.geojson" "$url"
echo "$sha256  $out/ne_10m_coastline.geojson" | shasum -a 256 -c -
url="https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/ne_10m_admin_1_states_provinces.geojson"
sha256="22d0e3ad85eb3e27f17cabf8ba2d50e554fbc27a87796ff891d958185da62fb5"
curl -sSL --max-time 300 -A "retail-cosmology research script" -o "$out/ne_10m_admin_1_states_provinces.geojson" "$url"
echo "$sha256  $out/ne_10m_admin_1_states_provinces.geojson" | shasum -a 256 -c -

#!/usr/bin/env bash
# ABS の格子人口（Australian population grid 2025、1 km 格子、EPSG:3577）を取ってくる。
# 出典：Australian Bureau of Statistics, Regional population 2024-25（CC BY 4.0）
# 出力：data/abs/apgr25_1_0_0.tif
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/abs"
url="https://www.abs.gov.au/statistics/people/population/regional-population/2024-25/GEOTIFF.zip"
sha256="e1645cb662f09749d369827dca9f02419181b118058a48ca3f7ddb102df8eacf"
mkdir -p "$out"
curl -sSL -A "retail-cosmology research script" -o "$out/GEOTIFF.zip" "$url"
echo "$sha256  $out/GEOTIFF.zip" | shasum -a 256 -c -
unzip -o -q "$out/GEOTIFF.zip" -d "$out"
ls "$out"

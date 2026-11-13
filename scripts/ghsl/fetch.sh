#!/usr/bin/env bash
# JRC の GHS-POP（R2023A、2020 年、1 km 格子、ESRI:54009 = Mollweide）を取ってくる（#51、米国のランダムカタログ）。
# 出典：Schiavina, M. et al., GHS-POP R2023A - GHS population grid multitemporal (1975-2030),
#   European Commission, Joint Research Centre (JRC), doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE（CC BY 4.0）
# 世界全体の一枚（約 320 MB）。米国本土の部分だけを Rust の側で切り出して読む。
# 出力：data/ghsl/GHS_POP_E2020_GLOBE_R2023A_54009_1000_V1_0.tif
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$root/data/ghsl"
url="https://jeodpp.jrc.ec.europa.eu/ftp/jrc-opendata/GHSL/GHS_POP_GLOBE_R2023A/GHS_POP_E2020_GLOBE_R2023A_54009_1000/V1-0/GHS_POP_E2020_GLOBE_R2023A_54009_1000_V1_0.zip"
sha256="846c03076da12d9d998e6b9946a0774ca134ea76b0731d11483058a7f9185bbf"
mkdir -p "$out"
curl -sS --max-time 1800 -A "retail-cosmology research script" -o "$out/ghs_pop_2020_1km.zip" "$url"
echo "$sha256  $out/ghs_pop_2020_1km.zip" | shasum -a 256 -c -
unzip -o -q "$out/ghs_pop_2020_1km.zip" -d "$out"
ls "$out"

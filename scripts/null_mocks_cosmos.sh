#!/usr/bin/env bash
# トライアルとコスモスの ξ と相互相関を、帰無仮説の模擬データと比べるための分布を作る（#51）。
# 全国と九州の中だけ（--prefectures 40-46）のそれぞれで、R = 50、100 km、地方だけ・密集地への寄り方もそろえた模擬データを 100 個ずつ。
# 選択関数は 10 km 格子の表で求める（付録 app:grid の近似）。
# 使い方：scripts/null_mocks_cosmos.sh [並列の数]
set -euo pipefail
jobs="${1:-6}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
S=data/trial_site/trial_site_clean_2026-11-03.csv,data/cosmos_site/cosmos_site_clean_2026-11-12.csv
for area in "" ",kyushu"; do
  for r in 50 100; do
    scripts/null_mocks_cross.sh 2026-11-03 "$S" TRIAL,COSMOS "$r" 100 "$jobs" "grid$area"
    scripts/null_mocks_cross.sh 2026-11-03 "$S" TRIAL,COSMOS "$r" 100 "$jobs" "density,grid$area"
  done
done

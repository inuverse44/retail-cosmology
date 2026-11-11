#!/usr/bin/env bash
# 8 km より短い距離を帰無仮説の模擬データと比べるために、相互相関の模擬データをビンごとの値つきで作り直す（#50）。
# 種は今までと同じなので、まとめた値と χ² の行（.txt）は前と同じものが出る。新しく _bins.csv が増える。
# 使い方：scripts/null_mocks_short.sh [並列の数]
set -euo pipefail
jobs="${1:-6}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
S=data/trial_site/trial_site_clean_2026-11-03.csv,data/seiyu_site/seiyu_site_clean_2026-11-04.csv
for r in 50 100; do
  scripts/null_mocks_cross.sh 2026-11-03 "$S" TRIAL,SEIYU "$r" 100 "$jobs"
  scripts/null_mocks_cross.sh 2026-11-03 "$S" TRIAL,SEIYU "$r" 100 "$jobs" density
  for k in TRIAL SEIYU; do
    scripts/null_mocks_cross.sh 2026-11-03 "$S" TRIAL,SEIYU "$r" 100 "$jobs" "density,grid,keep:$k"
  done
done

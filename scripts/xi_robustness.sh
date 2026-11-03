#!/usr/bin/env bash
# 設定を一つずつ変えて xi を計算し直し、ノートの表（設定を変えても結果が変わらないか）の値を出す。
# 使い方：scripts/xi_robustness.sh 2026-11-01
set -euo pipefail
date="${1:?日付（YYYY-MM-DD）}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release --bin xi
for spec in "base:" "nr50k:--n-random 50000" "npop10k:--n-pop 10000" "k32:--depth 5" "k16:--depth 4" "seed2:--seed 2" "seed3:--seed 3"; do
  tag=${spec%%:*}
  opts=${spec#*:}
  echo "== $tag ${opts}"
  # shellcheck disable=SC2086
  target/release/xi "$date" $opts --tag "$tag" 2>/dev/null
  cat "data/xi/chi2_${date}_${tag}.txt"
done

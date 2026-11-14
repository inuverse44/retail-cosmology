#!/usr/bin/env bash
# #81: two-scale field mocks, at most four concurrent processes.
# First time one full run, then tune on disjoint pilot runs before production.
# Usage: field_mocks_two_scale.sh TAG AMP SIGMA AMP2 SIGMA2 [RUNS=100] [START=100]
# Requires release builds of zeta_field and zeta_field_analysis.
set -euo pipefail
cd "$(dirname "$0")/.."
tag="${1:?unique output tag}"
amp="${2:?first amplitude}"
sigma="${3:?first width km}"
amp2="${4:?second amplitude}"
sigma2="${5:?second width km}"
runs="${6:-100}"
start="${7:-100}"
[[ "$runs" =~ ^[1-9][0-9]*$ && "$start" =~ ^[0-9]+$ ]] || exit 2
mkdir -p target/field-mocks-logs
shopt -s nullglob
worker_tags=()
pids=()
for worker in 0 1 2 3; do
  count=$((runs / 4 + (worker < runs % 4)))
  [[ "$count" -gt 0 ]] || continue
  worker_tag="${tag}_w${worker}"
  worker_tags+=("$worker_tag")
  existing=(data/zeta/field_jp_*_"${worker_tag}"*.csv)
  log="target/field-mocks-logs/${worker_tag}.log"
  [[ ! -e "$log" && "${#existing[@]}" == 0 ]] || {
    echo "Output already exists: $worker_tag" >&2
    exit 1
  }
  # tee preserves every progress line while retaining a timing record.
  (
    /usr/bin/time -p target/release/zeta_field 2026-11-12 \
      --country jp --stores data/cosmos_site/cosmos_site_clean_2026-11-12.csv \
      --chain COSMOS --footprint 50 --n-random 8000 --n-ref 50000 --seed 1 \
      --amp "$amp" --sigma "$sigma" --amp2 "$amp2" --sigma2 "$sigma2" \
      --centers 20000 --start "$start" --runs "$count" --tag "$worker_tag" 2>&1 | tee "$log"
  ) &
  pids+=("$!")
  start=$((start + count))
done
failed=0
for pid in "${pids[@]}"; do
  wait "$pid" || failed=1
done
[[ "$failed" == 0 ]] || exit 1
stems=()
for worker_tag in "${worker_tags[@]}"; do
  files=(data/zeta/field_jp_*_"${worker_tag}".csv)
  [[ "${#files[@]}" == 1 ]] || { echo "Missing or ambiguous output: $worker_tag" >&2; exit 1; }
  stems+=("${files[0]%.csv}")
done
target/release/zeta_field_analysis \
  data/zeta/zeta_jp_2026-11-12_cosmos_fp50_t8000.csv \
  "data/zeta/field_two_scale_${tag}" "${stems[@]}"

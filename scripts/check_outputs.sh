#!/usr/bin/env bash
# すべての実行ファイルを走らせ、出力ファイルと表示の SHA-256 を一覧にする。
# コードの構成を変える前と後で一覧を比べ、結果が 1 バイトも変わらないことを確かめる（ADR 001）。
# 使い方：scripts/check_outputs.sh > before.txt（変更前）、scripts/check_outputs.sh > after.txt（変更後）、diff before.txt after.txt
# 前提：data/ に OSM、ABS、e-Stat、公式サイトのデータがあること（README の「動かす」）。
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build -q --release
log="$(mktemp -d)"
b=target/release
mkdir -p data/toy
$b/toy > data/toy/poisson.csv
$b/osm_census 2026-11-01 > "$log/osm_census_stores.txt"
$b/osm_census 2026-11-01 trial > "$log/osm_census_trial.txt"
$b/trial_census 2026-11-03 2026-11-01 > "$log/trial_census.txt"
$b/xi 2026-11-01 2>/dev/null
$b/xi 2026-11-01 --depth 5 --tag k32 2>/dev/null
$b/xi 2026-11-03 --country jp 2>/dev/null
$b/xi 2026-11-03 --country jp --stores data/osm/trial_clean_2026-11-01.csv --tag osm 2>/dev/null
shasum -a 256 \
  data/toy/poisson.csv \
  data/osm/stores_clean_2026-11-01.csv data/osm/near_pairs_2026-11-01.csv \
  data/osm/trial_clean_2026-11-01.csv data/osm/trial_near_pairs_2026-11-01.csv \
  data/trial_site/trial_site_clean_2026-11-03.csv \
  docs/tables/osm_census.tex docs/tables/osm_census_jp.tex docs/tables/trial_site_census.tex \
  data/xi/xi_2026-11-01.csv data/xi/chi2_2026-11-01.txt \
  data/xi/xi_2026-11-01_k32.csv data/xi/chi2_2026-11-01_k32.txt \
  data/xi/xi_jp_2026-11-03.csv data/xi/chi2_jp_2026-11-03.txt \
  data/xi/xi_jp_2026-11-03_osm.csv data/xi/chi2_jp_2026-11-03_osm.txt
(cd "$log" && shasum -a 256 osm_census_stores.txt osm_census_trial.txt trial_census.txt)
rm -r "$log"

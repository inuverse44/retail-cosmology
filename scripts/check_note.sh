#!/usr/bin/env bash
# ノートを組み、PR の前に機械的に見つけられる誤りを探す（振り返り 2026-11-13 の T-01）。
# 見るもの：
#   - 「付録~\ref{…}」：\ref が「付録C」と出すので、本文でも「付録」と書くと「付録 付録C」になる（11/09 に 9 か所、11/13 に 6 か所）
#   - 組んだログの Overfull \hbox、未定義の参照・文献、二重に定義したラベル
# 使い方：scripts/check_note.sh [notes/retail-cosmology.tex]
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
tex="${1:-notes/retail-cosmology.tex}"
cd "$root"
latexmk -lualatex -interaction=nonstopmode -halt-on-error -cd -silent "$tex" > /dev/null || { echo "組めなかった：${tex%.tex}.log を見る"; exit 1; }
log="${tex%.tex}.log"
bad=0
if grep -n '付録~\\ref\|付録 \\ref' "$tex"; then echo "↑「付録」と \\ref が重なる（\\ref だけにする）"; bad=1; fi
if grep -n 'Overfull \\hbox' "$log"; then echo "↑行からはみ出している"; bad=1; fi
if grep -n 'undefined\|multiply defined' "$log"; then echo "↑未定義か二重定義"; bad=1; fi
[ "$bad" = 0 ] && echo "問題なし（${tex}）"
exit "$bad"

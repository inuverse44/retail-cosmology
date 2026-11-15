#!/usr/bin/env bash
# ノートを組み、PR の前に機械的に見つけられる誤りを探す（振り返り 2026-11-13 の T-01）。
# 見るもの：
#   - 「付録~\ref{…}」：\ref が「付録C」と出すので、本文でも「付録」と書くと「付録 付録C」になる（11/09 に 9 か所、11/13 に 6 か所）
#   - 組んだログの Overfull \hbox、未定義の参照・文献、二重に定義したラベル
# 中間ファイルは notes/build/ に出る（notes/.latexmkrc、ADR 006）。latexmk は notes/ の中で走らせないと .latexmkrc を読まない。
# 使い方：scripts/check_note.sh [notes/<ノート>.tex ...]（引数がなければ notes/ のノートをすべて）
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
if [ "$#" -gt 0 ]; then texs=("$@"); else texs=(notes/*.tex); fi
bad=0
for tex in "${texs[@]}"; do
  name="$(basename "$tex" .tex)"
  log="notes/build/${name}.log"
  if ! (cd notes && latexmk -interaction=nonstopmode -halt-on-error -silent "${name}.tex" > /dev/null); then
    echo "組めなかった：${log} を見る"; bad=1; continue
  fi
  this=0
  if grep -n '付録~\\ref\|付録 \\ref' "$tex"; then echo "↑「付録」と \\ref が重なる（\\ref だけにする）"; this=1; fi
  if grep -n 'Overfull \\hbox' "$log"; then echo "↑行からはみ出している"; this=1; fi
  if grep -n 'undefined\|multiply defined' "$log"; then echo "↑未定義か二重定義"; this=1; fi
  if [ "$this" = 0 ]; then echo "問題なし（${tex}）"; else echo "↑ ${tex}"; bad=1; fi
done
# docs/figures の図は、どれかのノートに載せる（振り返り 2026-11-14 の T-02。結果を PR にだけ書き、ノートに載せなかった）
for fig in docs/figures/*.pdf; do
  name="$(basename "$fig")"
  if ! grep -q "{${name}}" notes/*.tex; then
    echo "${fig} がどのノートにも載っていない（\\includegraphics{${name}}）"; bad=1
  fi
  # 図の中に日本語を入れない（2026-11-15 の指示）。凡例・軸・注記は記号と式で書く
  ja="$(pdftotext "$fig" - 2>/dev/null | perl -CSD -ne 'print if /[\p{Hiragana}\p{Katakana}\p{Han}]/')"
  if [ -n "$ja" ]; then
    echo "${fig} に日本語がある（記号と式に置き直す）："; echo "$ja" | sed 's/^/    /'; bad=1
  fi
done
exit "$bad"

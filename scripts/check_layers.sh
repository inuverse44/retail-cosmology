#!/usr/bin/env bash
# The Method の呼ぶ向きを grep で確かめる（Rust の crate の src フォルダを渡す）。
# 前提の配置：src/bin（Client）、src/manager、src/engine、src/access（ResourceAccess）、src/util、src/contract.rs。
# 使い方：check_layers.sh crates/my-crate/src [crate の名前（bin から参照する名前。既定は src の親フォルダ名の - を _ にしたもの）]
# 違反があれば一覧を出し、終了コード 1 を返す。テスト（#[cfg(test)] より後ろ）の参照は数えない。
# macOS の bash 3.2 でも動くように、globstar や連想配列は使わない。
set -uo pipefail
src="${1:?src のフォルダ}"
crate="${2:-$(basename "$(dirname "$src")" | tr '-' '_')}"
fail=0

# 層のフォルダの中の .rs の、テストより前の行を「ファイル:行: 中身」で出す
body() {
  local dir="$src/$1"
  [ -d "$dir" ] || return 0
  find "$dir" -name '*.rs' -print0 | xargs -0 awk 'FNR==1{skip=0} /#\[cfg\(test\)\]/{skip=1} !skip{print FILENAME":"FNR": "$0}'
}

check() {
  local title="$1" hits="$2"
  if [ -n "$hits" ]; then
    echo "NG：$title"
    printf '%s\n' "$hits" | sed 's/^/  /'
    fail=1
  else
    echo "OK：$title"
  fi
}

check "Engine が ResourceAccess・Manager・ファイルに触れていない" "$(body engine | grep -E 'crate::(access|manager)|std::fs|File::')"
check "Engine どうしで呼び合っていない（型はデータ契約に置く）" "$(body engine | grep -E 'crate::engine::')"
check "ResourceAccess どうしで呼び合っていない" "$(body access | grep -E 'crate::access::')"
check "Manager どうしで呼び合っていない" "$(body manager | grep -E 'crate::manager::')"
check "Utility が上の層を呼んでいない" "$(body util | grep -E 'crate::(engine|access|manager|contract)')"
check "Client が Manager だけを呼んでいる" "$(body bin | grep -E "${crate}::" | grep -vE "${crate}::manager::")"
exit $fail

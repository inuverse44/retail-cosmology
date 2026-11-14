---
id: "006"
status: Accepted
date: "2026-11-14"
---
# ADR 006: notes/ にはノートのソースと PDF だけを置き、中間ファイルは notes/build/ に出す

## Context

ノートは `notes/` の TeX から LuaLaTeX と latexmk で組み、PDF をコミットしている。
これまでは中間ファイル（`.aux`、`.log`、`.bbl`、`.toc`、`.fls`、`.fdb_latexmk` など）が TeX と同じ `notes/` に出ていた。
ノート 2 本で中間ファイルが 14 個あり、ソースと PDF より多かった（2026-11-14 に確認）。
`.gitignore` で拡張子ごとに外していたのでコミットはされないが、手元で `notes/` を開くと、読むための PDF が中間ファイルに埋もれる。
本人が読むのは PDF である。

さらに、二点相関の先の研究（三点相関関数、九州の主なチェーンの比較）を、発表するノート（`notes/retail-cosmology.tex`）とは別のノートで進めることにした。
ノートが 3 本になると、中間ファイルはさらに増える。

latexmk（TeX Live 2026 の 4.88 で確認）は `$aux_dir` で中間ファイルの置き場所を、`$out_dir` で PDF の置き場所を分けられる。
設定ファイル `.latexmkrc` は、latexmk を走らせたディレクトリにあるものが読まれる。
リポジトリの根から `-cd` で `notes/` のノートを組むと、読んだ設定ファイルは「NONE」と表示され、`notes/.latexmkrc` は読まれなかった（2026-11-14 に確認）。

## Decision

1. `notes/` に置くのは、ノートの TeX、文献の `references.bib`、組んだ PDF、latexmk の設定 `notes/.latexmkrc` だけにする。
2. 中間ファイルはすべて `notes/build/` に出し、`notes/build/` を git の管理から外す。設定は `notes/.latexmkrc` に一か所で書く（LuaLaTeX、`$aux_dir`、`$out_dir`）。
3. ノートは `notes/` の中で latexmk を走らせて組む。検査のスクリプトと git の hook もこの形で組む。
4. 二点相関の先の研究は、同じリポジトリの `notes/` に別のノートとして置く。データの取得とコードの層（ADR-001 の Decision 項 1–5（層の分け方））は、今のノートと共有する。文献の `references.bib` も共有する。

### 本 ADR で決定しないこと

- 先の研究のノートの題と節の分け方。ノートの TeX で決める。
- 三点相関関数の数え上げをコードのどの層に置くか。コードに入る前に別の ADR で決める。

## Consequences

- `notes/` を開くと、ソースと PDF だけが並ぶ。
- `notes/` の外から `latexmk -cd notes/…` で組むと `.latexmkrc` が読まれず、中間ファイルが `notes/` に出てしまう。手順は README の「動かす」と `scripts/check_note.sh` に書く。
- 先の研究のノートは今のノートと同じデータとコードを使うので、二つのノートの数値の出どころがそろう。その代わり、リポジトリには発表済みのノートと書きかけのノートが並ぶ。README でどちらかを書き分ける。

## Alternatives Considered

- **ソースを `notes/src/` に移し、PDF だけを `notes/` に置く**：PDF はさらに見つけやすいが、図と表の相対パス（`../docs/figures/`、`../docs/tables/`）、検査のスクリプト、README、ほかのリポジトリへ PDF を写す手順をすべて変える必要がある。中間ファイルを分けるだけで「PDF が埋もれる」問題は解けるので、選ばなかった。
- **先の研究を新しいリポジトリにする**：公開物としては分かれるが、データの取得とコードを複製するか、依存させる必要がある。選ばなかった。
- **今のノートに章を足す**：発表する PDF が書きかけの内容で長くなる。今のノートはすでに長さを見直したところなので、選ばなかった。

## 関連する ADR

- ADR-001（変わりやすさで層に分ける）：先の研究のコードも同じ層に置く。
- ADR-004（ノートの図の体裁）：先の研究のノートの図も同じ体裁で描く。

## References

1. John Collins, latexmk（Version 4.88）。`latexmk --help` の `-auxdir`、`-outdir`、`-emulate-aux-dir`（2026-11-14 に確認）。設定ファイルでの名前（`$aux_dir`、`$out_dir`、`$emulate_aux`）は CTAN の文書による。https://ctan.org/pkg/latexmk

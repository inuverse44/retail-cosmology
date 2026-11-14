# notes/ のノートを組むときの設定（ADR 006）。
# 中間ファイル（.aux、.log、.bbl など）は build/ に出し、notes/ にはソースと PDF だけを残す。
# 使い方：cd notes && latexmk retail-cosmology.tex（ほかのノートも同じ）
$pdf_mode = 4;          # LuaLaTeX で PDF を作る
$aux_dir = 'build';     # 中間ファイルの置き場所（git の管理外）
$out_dir = '.';         # PDF は notes/ に置く
$emulate_aux = 1;       # aux_dir と out_dir を分けるとき、latexmk が自分で振り分ける

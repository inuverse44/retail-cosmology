# ADR（Architecture Decision Record）

このリポジトリの設計判断を残す。一つの ADR に一つの判断を書く。

## 一覧

| ID | 題 | 状態 |
|---|---|---|
| [001](001-layering-by-volatility.md) | 変わりやすさで層に分ける（The Method） | Accepted |
| [002](002-combine-bins-with-plain-mean.md) | ビンをまとめた値は、単純な平均に jackknife の誤差をつける | Accepted |
| [003](003-lean-proofs-in-repo.md) | ノートの式の導出を Lean で検証し、証明はこのリポジトリの lean/ に置く | Superseded by ADR 005 |
| [004](004-figure-style.md) | ノートの図の体裁を一か所で決め、本文と同じ処理系で組む | Accepted |
| [005](005-lean-model-in-scope.md) | 点を独立に撒く模型から、分散の式までを Lean で検証する | Accepted |
| [006](006-notes-sources-and-build-dir.md) | notes/ にはノートのソースと PDF だけを置き、中間ファイルは notes/build/ に出す | Accepted |
| [007](007-three-point-counting-engine.md) | 三つ組の数え上げは新しい Engine として足し、二点の数え上げには手を入れない | Accepted |
| [008](008-regional-selection-function.md) | 地方の店舗数と人口から区分一定の選択関数を作る | Accepted |
| [009](009-matched-mock-density.md) | 親ごとの子の抽出で一点密度を調べる基準を作る | Accepted |
| [010](010-skewed-density-field.md) | ガウス場を点ごとに変換して、ζ の符号を選べる密度場の模型を作る | Accepted |
| [011](011-counts-in-cells.md) | 店をマスに数え、期待数からのずれの分布で負の歪みの出どころを調べる | Accepted |
| [012](012-inhibition-and-pop-power.md) | 密度場の模型に、店どうしが近づかない条件と、人口に比例するより緩い増え方を足す | Accepted |
| [013](013-cell-size-scan.md) | 店を数えるマスの大きさと期待数の幅を変えて、空のマスの不足が現れる規模を調べる | Accepted |
| [014](014-coverage-condition.md) | 密度場の模型に、近くに店のない所を先に埋める条件を足す | Accepted |
| [015](015-catchment-response.md) | 密度場の模型に、店のまわりの人口に対する置き方の反応を足す | Accepted |

## 書き方

TOKIUM の ADR のガイドライン（References の 1）に沿う。テンプレートは [000-format.md](000-format.md)。

- ファイル名は `NNN-題を表す英語.md`（NNN は 3 桁の通し番号）。番号は一覧の最後の次を使う。
- 状態（frontmatter の `status`）は `Proposed`、`Accepted`、`Deprecated`、`Superseded by ADR NNN` のどれか。状態を変えたら、この一覧と、その ADR を参照している文書も合わせて直す。
- Accepted の ADR の判断を変えるときは、本文を書き換えず、新しい ADR を書いて古い方を `Superseded by ADR NNN` にする。誤字の修正と、出典の追加はこの限りではない。
- 本文にチケット・PR・Issue の番号を書かない。コードは行番号ではなく、ファイルとモジュールの名前で指す。ほかの ADR は「ADR-NNN の Decision 項 N（題）」の形で指す。
- 事実はコードや一次資料と照らし、出典と確認日を書く。ADR だけを読んで判断がわかるように書く。
- 移行の計画、進み具合、作業の表は書かない。それはコミットと Issue に残す。
- Decision の最後に「本 ADR で決定しないこと」を置き、決めないことと、それを委ねる先（このリポジトリにある文書かコード）を書く。
- Decision は実装の自由度を奪わない粒度で書く。実現の方法を別の方法に替えたときに ADR を書き換える必要があるなら、その記述は ADR ではなく、コードの文書コメントかコミットに書く。
- 最終更新日のような、すぐ古くなる情報は本文に書かない（日付は frontmatter の `date` だけ）。
- 未確定のことを残したまま Accepted にしない。

## References

1. にしこりさぶろ〜「AIが書く「読みづらいADR」に発狂しかけたので、ADRのガイドラインを作った」、TOKIUM（Zenn）、2026. https://zenn.dev/tokium_dev/articles/adr-authoring-guideline
2. Michael Nygard, "Documenting Architecture Decisions", 2011. https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions

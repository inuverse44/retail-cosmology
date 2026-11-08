---
id: "003"
status: Proposed
date: "2026-11-08"
---
# ADR 003: ノートの式の導出を Lean で検証し、証明はこのリポジトリの lean/ に置く

## Context

ノート（`notes/retail-cosmology.tex`）の付録 B で、データ同士の組の数 $DD$ の分散を二通りに導いている。
店の数 $n$ を固定した場合の式（ラベル `eq:var-fixed`）と、店の数をポアソン分布で揺らした場合の全分散の式（ラベル `eq:var-poisson`）である。
導出は三つの段階からなる。

1. 二つの組の関係（同じ組、一点を共有、共有しない）ごとの数え上げと、その和が組み合わせの総数になる恒等式（ラベル `eq:count-identity`）。
2. 関係ごとの期待値 $G_p$、$G_t$、$G_q = G_p^2$ から分散を組み立てる代数。
3. ポアソン分布の階乗モーメント（ラベル `eq:poisson-factorial`）と、全分散の公式。

この導出は 2026-11-08 に二通りで確かめた。

- 数式処理（sympy）で、恒等式、固定した場合の式、ポアソン分布の全分散を展開し、差が 0 になることを確かめた。このスクリプトはコミットしていない。
- Rust のテスト（`engine::variance` のテスト）で、単位正方形に点を撒き直した $DD$ の標本分散を二つの式と比べた。$n = 200$ と平均 200 のポアソン分布について、標準誤差の 4 倍以内で合う。

数値のテストは決まった $n$ と乱数の種について、模擬データの揺らぎの範囲で合うことしか示さない。
数式処理の展開は一般の $n$ についての数え上げ（段階 1）や、無限和の階乗モーメント（段階 3）を証明しない。
ノートは PDF として外に出すので、導出そのものが一般の $n$ で正しいことを、機械に検査させたい。

Lean の数学ライブラリ Mathlib には、ポアソン分布（`ProbabilityTheory.poissonPMFReal`、`poissonMeasure`。`Mathlib/Probability/Distributions/Poisson/Basic.lean`）と、全分散の公式（`ProbabilityTheory.integral_condVar_add_variance_condExp`。`Mathlib/Probability/CondVar.lean`）がある（2026-11-08 に Mathlib のコミット 5ed2965 で確認）。

## Decision

1. ノート付録 B の導出のうち、式の形を Lean で検証する。対象は Context の段階 1–3 である。
   検証する命題は関係ごとの期待値 $G_p$、$G_t$、$G_q$ を実数の変数として受け取り、そこから分散の式が出ることとする。
   点が独立に撒かれていれば関係ごとの期待値がこれらの値になる、という模型の部分は検証の対象に入れない。
   $G_p$ と $G_t$ の数値、模型が店の分布に合うかどうかは、これまでどおり Rust で扱う（`engine::variance` と `manager::variance`）。
2. 証明はこのリポジトリの `lean/` に、独立した Lake のプロジェクトとして置く。
   Rust の Cargo のワークスペースとは依存し合わない。
   ADR-001 の Decision 項 1–5（層の分け方）の外にあり、`scripts/` と同じく Rust の層には入らない。
   ここに置く理由は、証明がノートの式と同じ理由で変わるからである。
   ノートの式や記号を直せば証明も直すことになるので、同じコミットで一緒に変えられる場所に置く。
3. 定理はノートの式とラベルで対応づける。
   各定理の文書コメントに、対応するノートの式のラベル（`eq:var-fixed` など）を書く。
   式の番号は書き足しで変わるので使わない。
   ノートの側でも、Lean で検証した式とその定理の名前を示す。
4. Lake と Mathlib のビルド結果（`lean/.lake`）はコミットしない。

### 本 ADR で決定しないこと

- 点が独立に撒かれていることから関係ごとの期待値を導く部分（測度論の上での形式化）まで Lean で示すかどうか。段階 1–3 を終えてから判断し、`lean/README.md` に書く。
- Lean と Mathlib のバージョン。`lean/lean-toolchain` と `lean/lake-manifest.json` で決める。
- 定理の分け方とファイルの構成。`lean/` のコードで決める。
- 検証を自動で走らせるかどうか。このリポジトリには今 CI がない。走らせ方は `README.md` の「動かす」の節に書く。

## Consequences

- 付録 B の式の形が一般の $n$ と平均 $N$ について Lean のカーネルで検査される。
- 付録 B の式を変えると、`lean/` の証明も直す必要がある。証明が通らない間は、ノートの式を変えたことになっていない。
- `lean/` で作業するには Lean の処理系と、Mathlib のビルド済みファイル（数 GB）の取得が要る。Rust の計算とノートのビルドには要らない。
- 計算の言語（Rust）、描画の言語（Python）、ノート（TeX）に、検証の言語として Lean が加わる。
- このリポジトリは非公開なので、証明も非公開になる。ノートの PDF だけを外に出すとき、証明は PDF から参照できない。

## Alternatives Considered

- **証明を別のリポジトリに置く**：公開でき、独立して CI を走らせられる。選ばなかったのは、ノートの式と証明が別々の場所で変わることになり、式を直すたびに二つのリポジトリを合わせて変える必要があるからである。ラベルによる対応づけもリポジトリをまたぐと崩れても気づきにくい。
- **数式処理（sympy）のテストをコミットする**：段階 2 の代数は確かめられる。選ばなかったのは、Context に書いたとおり段階 1 と 3 を証明しないことと、このリポジトリでは Python を描画とデータの取得だけに使っているからである（`README.md` の「構成」の節）。
- **Rust の数値のテストだけにする**：すでにある。決まった $n$ について模擬データの揺らぎの範囲で合うことしか示さず、式の小さな誤り（揺らぎより小さい項の係数など）を見分けられない。数値のテストは残し、Lean の検証と役割を分ける。

## 関連する ADR

- ADR-001（変わりやすさで層に分ける）：`lean/` はその層の外に置く。置き換えの関係はない。

## References

1. The Mathlib Community, "The Lean Mathematical Library", Proceedings of the 9th ACM SIGPLAN International Conference on Certified Programs and Proofs (CPP 2020), 367–381. https://doi.org/10.1145/3372885.3373824
2. Mathlib, `Mathlib/Probability/Distributions/Poisson/Basic.lean` と `Mathlib/Probability/CondVar.lean`（コミット 5ed2965）. https://github.com/leanprover-community/mathlib4

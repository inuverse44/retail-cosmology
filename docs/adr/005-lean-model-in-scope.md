---
id: "005"
status: Accepted
date: "2026-11-08"
---
# ADR 005: 点を独立に撒く模型から、分散の式までを Lean で検証する

## Context

ADR-003 の Decision 項 1（Lean で検証する範囲）では、ノートの付録 B のうち式の形だけを Lean で検証すると決めた。
関係ごとの期待値 $G_p$、$G_t$、$G_q$ は実数の変数として受け取り、点が独立に撒かれていれば期待値がこれらの値になる、という模型の部分は対象から外した。

2026-11-08 の時点で、`lean/RetailCosmology/VarDD.lean` は付録 B で証明が要る式 10 本のうち 5 本を検査している（`eq:dd-second`、`eq:count-identity`、`eq:var-fixed`、`eq:poisson-factorial`、`eq:var-poisson`）。
残りの 5 本は次のとおりである。

- `eq:g-def`：点を独立に撒くと、関係ごとの期待値が $G_p$、$G_t$、$G_p^2$ になる。
- `eq:dd-mean`：$\mathbb{E}[DD \mid n] = n(n-1) G_p / 2$。
- `eq:gt-var`：$G_t - G_p^2$ は点の位置で見た $f$ の分散に等しい。
- `eq:total-var`：全分散の公式。
- `eq:g-estimate`：ランダムカタログから $G_p$ と $G_t$ を偏りなく求める式。

`eq:total-var` 以外の 4 本は点の配置を確率変数として書かないと述べられない。
Lean の数学ライブラリ Mathlib には、直積測度（`MeasureTheory.Measure.pi`）、その座標が互いに独立であること（`ProbabilityTheory.iIndepFun_pi`）、独立な量の積の期待値（`ProbabilityTheory.IndepFun.integral_fun_comp_mul_comp`）、分散（`ProbabilityTheory.variance`）がある（2026-11-08 に Mathlib v4.35.0 で確認）。
2026-11-08 に、模型の部分まで検証の範囲を広げることにした。

## Decision

1. ADR-003 の Decision 項 1 を、次のとおり置き換える。ノートの付録 B のうち、点を独立に撒く模型から分散の式までを Lean で検証する。
   模型は、$n$ 個の点が同じ確率分布 $\mu$ から独立に撒かれるものとし、ビンに入るかを表す関数 $\theta$ は 0 か 1 をとり、二点について対称とする。
   $G_p$、$G_t$ は $\mu$ と $\theta$ から積分として定義し、変数として受け取らない。
2. 店の数をポアソン分布で揺らす場合も、店の数と点の配置を一つの確率空間の上の確率変数として書き、全分散を計算する。
3. $G_p$ と $G_t$ の数値と、模型が店の分布に合うかどうかは、これまでどおり Rust で扱う（`engine::variance` と `manager::variance`）。
4. ADR-003 の Decision 項 2（証明の置き場所）、項 3（ノートの式とラベルで対応づける）、項 4（ビルド結果をコミットしない）は、そのまま引き継ぐ。

### 本 ADR で決定しないこと

- 定理の分け方、確率空間の具体的な作り方（直積測度を使うか、一般の確率空間の上の独立な確率変数として書くか）。`lean/` のコードで決める。
- 付録 B の外（推定量 $\hat\xi_{\mathrm{LS}}$ の分散など）を検証するかどうか。`lean/README.md` に書く。

## Consequences

- 付録 B の「点を独立に撒く」という仮定から分散の式までが、Lean のカーネルで検査される。
- 段階 1–3 の定理は関係ごとの期待値を受け取る形のまま残し、模型の定理はそれを使って組み立てる。
- 確率論の定理を使うので、証明は代数と数え上げだけの段階より長くなり、Mathlib の確率論の部分の名前の変更の影響を受けやすくなる。

## Alternatives Considered

- **ADR-003 のまま、模型の部分を検証しない**：数値での確認（toy model、オーストラリアの値と jackknife の誤差の比較）はある。選ばなかったのは、導出の出発点が検査されないまま残り、付録 B の式の半分が対象外になるからである。
- **模型だけを検証し、ポアソン分布で揺らす場合は代数のままにする**：手間は減るが、全分散の公式を店の数という確率変数につながず、「公式の右辺の計算」で止まる。

## 関連する ADR

- ADR-003（ノートの式の導出を Lean で検証し、証明はこのリポジトリの lean/ に置く）：本 ADR がその Decision 項 1 を置き換える。ADR-003 は Superseded by ADR 005 とする。

## References

1. Mathlib, `Mathlib/MeasureTheory/Constructions/Pi.lean`、`Mathlib/Probability/Independence/Basic.lean`、`Mathlib/Probability/Independence/Integration.lean`、`Mathlib/Probability/Moments/Variance.lean`（v4.35.0）. https://github.com/leanprover-community/mathlib4

# lean/

ノート（`notes/retail-cosmology.tex`）の付録「DD の分散」で導いた $DD$ の分散の式を、Lean 4 と Mathlib で検証する。
置き場所は [ADR 003](../docs/adr/003-lean-proofs-in-repo.md)、検証の範囲（点を独立に撒く模型から）は [ADR 005](../docs/adr/005-lean-model-in-scope.md) に書いた。

## 動かす

```sh
cd lean
lake exe cache get   # Mathlib のビルド済みファイルを取る（初回だけ。数 GB）
lake build           # 証明を検査する
```

Lean のバージョンは `lean-toolchain`、Mathlib のバージョンは `lakefile.toml` と `lake-manifest.json` で決まる。
`lake build` が通れば、下の定理はすべて Lean のカーネルで検査されている。
使っている公理は Lean の標準の三つ（`propext`、`Classical.choice`、`Quot.sound`）だけである（`#print axioms` で確かめられる）。

## ノートの式と定理

ノートの式はラベルで指す。ファイルは `RetailCosmology/` の下にある。

- `VarDD.lean`：段階 1–3。関係ごとの期待値を実数の変数として受け取り、数え上げと代数とポアソン分布の計算を示す。
- `Model.lean`：段階 4。$n$ 個の点を同じ確率分布 $\mu$ から独立に撒く模型（直積測度）を置き、$G_p$、$G_t$ を積分で定義して、店の数を固定した場合を示す。
- `Poisson.lean`：段階 4。店の数をポアソン分布で選び、点を独立に撒く確率空間 `Σ m, (Fin m → E)` を置き、全分散を示す。

| ノートの式 | 定理 | 内容 |
|---|---|---|
| `eq:g-def` | `integral_pair`、`integral_triple`、`integral_quad` | 独立に撒いた点で、関係ごとの期待値が $G_p$、$G_t$、$G_p^2$ になる |
| `eq:dd-mean` | `integral_DD` | $\mathbb{E}[DD \mid n] = n(n-1)G_p/2$ |
| `eq:dd-second` | `integral_DD_sq`（数え上げは `dd_second_moment`） | $\mathbb{E}[DD^2 \mid n]$ |
| `eq:count-identity` | `count_identity` | 三通りの数を足すと組み合わせの総数になる |
| `eq:var-fixed` | `variance_DD`（代数は `var_fixed`） | 店の数を固定した分散 |
| `eq:gt-var` | `Gtriple_sub_sq` | $G_t - G_p^2$ は点の位置で見た $f$ の分散 |
| `eq:poisson-factorial` | `poisson_factorial_moment` | Mathlib のポアソン分布 `Po(N)` での階乗モーメント |
| `eq:var-poisson` | `variance_DDmix`（代数は `var_poisson`） | 店の数をポアソン分布で揺らしたときの全分散 |
| `eq:total-var` | `variance_DDmix_eq_total` | 全分散が、固定した分散のポアソン平均と条件つき期待値の分散の和に等しい |
| `eq:g-estimate` | `integral_sum_binCount`、`integral_sum_binCount_mul` | ランダムカタログから $G_p$、$G_t$ を偏りなく求める |

## 検証していないこと

- $G_p$ と $G_t$ の数値と、模型（点を独立に撒く）が店の分布に合うかどうか。これは Rust（`engine::variance`、`manager::variance`）で扱う。
- 付録「DD の分散」の本文の近似（ビンの距離が短いとき $f(\bm{x}) \simeq A\,p(\bm{x})$）は、式のラベルのない定性的な説明で、検証していない。
- 付録「DD の分散」の外（推定量 $\hat\xi_{\mathrm{LS}}$ の分散など）は、まだ対象にしていない（ADR 005 で決めないことにしたもの）。

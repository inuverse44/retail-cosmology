# lean/

ノート（`notes/retail-cosmology.tex`）の付録 B で導いた $DD$ の分散の式を、Lean 4 と Mathlib で検証する。
置き場所と役割の分け方は [ADR 003](../docs/adr/003-lean-proofs-in-repo.md) に書いた。

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

定理は `RetailCosmology/VarDD.lean` にある。ノートの式はラベルで指す。

| ノートの式 | 定理 | 内容 |
|---|---|---|
| `eq:dd-second` | `dd_second_moment` | 二つの対の関係（同じ組、一点を共有、共有しない）ごとの数え上げから $\mathbb{E}[DD^2 \mid n]$ を組み立てる |
| `eq:count-identity` | `count_identity` | 三通りの数を足すと組み合わせの総数になる |
| `eq:var-fixed` | `var_fixed`、`var_fixed_of_counting` | 店の数を固定した分散。後者は数え上げからつないだもの |
| `eq:poisson-factorial` | `poisson_factorial_moment` | Mathlib のポアソン分布 `Po(N)` での階乗モーメント |
| `eq:var-poisson` | `var_poisson` | `Po(N)` での $\mathbb{E}[\mathrm{Var}[DD \mid n]] + \mathrm{Var}[\mathbb{E}[DD \mid n]]$ |

## 検証していないこと

- 関係ごとの期待値 $G_p$、$G_t$、$G_q = G_p^2$ は、実数の変数として受け取っている。点が独立に撒かれていれば期待値がこれらの値になることは示していない。
- 全分散の公式そのもの。Mathlib に `ProbabilityTheory.integral_condVar_add_variance_condExp` としてあるが、$DD$ を確率変数として定義していないので、つないでいない。`var_poisson` は、公式の右辺の二つの項を計算したものである。
- $G_p$ と $G_t$ の数値と、模型が店の分布に合うかどうか。これは Rust（`engine::variance`、`manager::variance`）で扱う。

上の一つ目と二つ目まで Lean で示す（点の配置を確率変数として測度論の上で書く）かどうかは、まだ決めていない。決めたらここに書く。

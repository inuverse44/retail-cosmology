import Mathlib.Probability.Distributions.Poisson.Basic

/-!
# DD の分散（ノート `notes/retail-cosmology.tex` の付録「DD の分散」）

ノートの付録「DD の分散」の導出のうち、式の形を検証する（ADR 003）。
関係ごとの期待値 `Gp`、`Gt`、`Gq` は実数の変数として受け取る。
点が独立に撒かれていれば期待値がこれらの値になる、という模型の部分は扱わない。

組は順序つきの対 `(i, j)`（`i ≠ j`）で表す。
順序のない組の和は順序つきの対の和の半分なので、`DD = (1/2) Σ_{i ≠ j} θ_{ij}`、
`DD² = (1/4) Σ_P Σ_Q θ_P θ_Q` である（`P`、`Q` は順序つきの対）。
-/

open Finset

namespace RetailCosmology

/-! ## 段階 1：二つの組の関係ごとの数え上げ -/

section Counting

variable {n : ℕ}

/-- `Q` は `P` と同じ組（順序を入れ替えたものを含む）。 -/
def SamePair (P Q : Fin n × Fin n) : Prop := Q = P ∨ Q = P.swap

/-- `Q` は `P` と点を共有しない。 -/
def Disjoint' (P Q : Fin n × Fin n) : Prop :=
  Q.1 ≠ P.1 ∧ Q.1 ≠ P.2 ∧ Q.2 ≠ P.1 ∧ Q.2 ≠ P.2

instance (P Q : Fin n × Fin n) : Decidable (SamePair P Q) := by
  unfold SamePair; infer_instance

instance (P Q : Fin n × Fin n) : Decidable (Disjoint' P Q) := by
  unfold Disjoint'; infer_instance

/-- 対 `P` と同じ組は、`P` 自身と順序を入れ替えたものの 2 個。 -/
theorem card_samePair {P : Fin n × Fin n} (hP : P.1 ≠ P.2) :
    ((univ : Finset (Fin n)).offDiag.filter (SamePair P)).card = 2 := by
  have h : (univ : Finset (Fin n)).offDiag.filter (SamePair P) = {P, P.swap} := by
    ext Q
    simp only [mem_filter, mem_offDiag, mem_univ, true_and, SamePair, mem_insert,
      mem_singleton]
    constructor
    · exact fun h => h.2
    · rintro (rfl | rfl)
      · exact ⟨hP, Or.inl rfl⟩
      · exact ⟨fun h => hP h.symm, Or.inr rfl⟩
  rw [h, card_pair]
  intro h
  exact hP (by simpa [Prod.ext_iff] using congrArg Prod.fst h)

/-- 対 `P` と点を共有しない対は、残りの `n - 2` 点から作る順序つきの対で、`(n-2)(n-3)` 個。 -/
theorem card_disjoint {P : Fin n × Fin n} (hP : P.1 ≠ P.2) :
    (((univ : Finset (Fin n)).offDiag.filter (Disjoint' P)).card : ℝ)
      = ((n : ℝ) - 2) * ((n : ℝ) - 3) := by
  have h : (univ : Finset (Fin n)).offDiag.filter (Disjoint' P)
      = (univ \ {P.1, P.2}).offDiag := by
    ext Q
    simp only [mem_filter, mem_offDiag, mem_univ, true_and, Disjoint', mem_sdiff,
      mem_insert, mem_singleton, not_or]
    tauto
  have hs : (univ \ {P.1, P.2} : Finset (Fin n)).card = n - 2 := by
    rw [card_sdiff_of_subset (subset_univ _), card_univ, Fintype.card_fin, card_pair hP]
  have hn : 2 ≤ n := by
    have := card_le_univ ({P.1, P.2} : Finset (Fin n))
    rw [card_pair hP, Fintype.card_fin] at this
    exact this
  rw [h, offDiag_card, hs]
  push_cast [Nat.cast_sub hn, Nat.cast_sub (Nat.le_mul_self (n - 2))]
  ring

/-- 順序つきの対の総数は `n(n-1)`。 -/
theorem card_offDiag :
    (((univ : Finset (Fin n)).offDiag.card : ℕ) : ℝ) = (n : ℝ) * ((n : ℝ) - 1) := by
  rw [offDiag_card, card_univ, Fintype.card_fin]
  push_cast [Nat.cast_sub (Nat.le_mul_self n)]
  ring

/-- 対 `P` を固定したときの、もう一つの対 `Q` についての和。
`Q` が同じ組なら `Gp`、点を共有しなければ `Gq`、それ以外（一点を共有）なら `Gt` をとる関数を足す。 -/
theorem inner_sum (f : Fin n × Fin n → ℝ) (Gp Gt Gq : ℝ) {P : Fin n × Fin n} (hP : P.1 ≠ P.2)
    (hsame : ∀ Q, Q.1 ≠ Q.2 → SamePair P Q → f Q = Gp)
    (hdisj : ∀ Q, Q.1 ≠ Q.2 → Disjoint' P Q → f Q = Gq)
    (hone : ∀ Q, Q.1 ≠ Q.2 → ¬ SamePair P Q → ¬ Disjoint' P Q → f Q = Gt) :
    ∑ Q ∈ (univ : Finset (Fin n)).offDiag, f Q
      = 2 * Gp + 4 * ((n : ℝ) - 2) * Gt + ((n : ℝ) - 2) * ((n : ℝ) - 3) * Gq := by
  set s := (univ : Finset (Fin n)).offDiag
  have hsplit := sum_filter_add_sum_filter_not s (SamePair P) f
  have hsplit2 := sum_filter_add_sum_filter_not (s.filter fun Q => ¬ SamePair P Q) (Disjoint' P) f
  -- 点を共有しない対は、同じ組ではない
  have hdisj_not_same : ∀ Q, Disjoint' P Q → ¬ SamePair P Q := by
    rintro Q ⟨h1, h2, -, -⟩ (rfl | rfl)
    · exact h1 rfl
    · exact h2 rfl
  have hfd : (s.filter fun Q => ¬ SamePair P Q).filter (Disjoint' P) = s.filter (Disjoint' P) := by
    rw [filter_filter]
    exact filter_congr fun Q _ => ⟨fun h => h.2, fun h => ⟨hdisj_not_same Q h, h⟩⟩
  have hS : ∑ Q ∈ s.filter (SamePair P), f Q = 2 * Gp := by
    rw [sum_congr rfl fun Q hQ => hsame Q (mem_offDiag.1 (mem_filter.1 hQ).1).2.2 (mem_filter.1 hQ).2,
      sum_const, card_samePair hP]
    simp
  have hD : ∑ Q ∈ s.filter (Disjoint' P), f Q = ((n : ℝ) - 2) * ((n : ℝ) - 3) * Gq := by
    rw [sum_congr rfl fun Q hQ => hdisj Q (mem_offDiag.1 (mem_filter.1 hQ).1).2.2 (mem_filter.1 hQ).2,
      sum_const, nsmul_eq_mul,
      card_disjoint hP]
  -- 一点を共有する対の数は、総数から残りを引いて求める
  have hcard : ((((s.filter fun Q => ¬ SamePair P Q).filter fun Q => ¬ Disjoint' P Q).card : ℕ) : ℝ)
      = 4 * ((n : ℝ) - 2) := by
    have e1 := card_filter_add_card_filter_not (s := s) (SamePair P)
    have e2 := card_filter_add_card_filter_not
      (s := s.filter fun Q => ¬ SamePair P Q) (Disjoint' P)
    rw [hfd] at e2
    have e1' : ((s.filter (SamePair P)).card : ℝ) + ((s.filter fun Q => ¬ SamePair P Q).card : ℝ)
        = (s.card : ℝ) := by exact_mod_cast e1
    have e2' : ((s.filter (Disjoint' P)).card : ℝ)
        + (((s.filter fun Q => ¬ SamePair P Q).filter fun Q => ¬ Disjoint' P Q).card : ℝ)
        = ((s.filter fun Q => ¬ SamePair P Q).card : ℝ) := by exact_mod_cast e2
    rw [card_samePair hP] at e1'
    rw [card_disjoint hP] at e2'
    rw [card_offDiag] at e1'
    push_cast at e1'
    linarith
  have hT : ∑ Q ∈ (s.filter fun Q => ¬ SamePair P Q).filter (fun Q => ¬ Disjoint' P Q), f Q
      = 4 * ((n : ℝ) - 2) * Gt := by
    rw [sum_congr rfl fun Q hQ => by
        simp only [mem_filter] at hQ
        exact hone Q (mem_offDiag.1 hQ.1.1).2.2 hQ.1.2 hQ.2,
      sum_const, nsmul_eq_mul, hcard]
  rw [← hsplit, ← hsplit2, hfd, hS, hD, hT]
  ring

/-- ノートの式 `eq:dd-second`：`E[DD² | n]`。
`f P Q` は `E[θ_P θ_Q]`（`P`、`Q` は順序つきの対）で、二つの対の関係ごとに `Gp`、`Gt`、`Gq` をとる。 -/
theorem dd_second_moment (f : Fin n × Fin n → Fin n × Fin n → ℝ) (Gp Gt Gq : ℝ)
    (hsame : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → SamePair P Q → f P Q = Gp)
    (hdisj : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → Disjoint' P Q → f P Q = Gq)
    (hone : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → ¬ SamePair P Q → ¬ Disjoint' P Q → f P Q = Gt) :
    (1 / 4 : ℝ) * ∑ P ∈ (univ : Finset (Fin n)).offDiag, ∑ Q ∈ (univ : Finset (Fin n)).offDiag, f P Q
      = (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * ((n : ℝ) - 3) / 4 * Gq
        + (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * Gt
        + (n : ℝ) * ((n : ℝ) - 1) / 2 * Gp := by
  rw [sum_congr rfl fun P hP => inner_sum (f P) Gp Gt Gq (mem_offDiag.1 hP).2.2
      (fun Q => hsame P Q (mem_offDiag.1 hP).2.2) (fun Q => hdisj P Q (mem_offDiag.1 hP).2.2)
      (fun Q => hone P Q (mem_offDiag.1 hP).2.2),
    sum_const, nsmul_eq_mul, card_offDiag]
  ring

end Counting

/-! ## 段階 2：店の数を固定した分散 -/

/-- ノートの式 `eq:count-identity`：三通りの数を足すと組み合わせの総数になる。 -/
theorem count_identity (n : ℝ) :
    (n * (n - 1) / 2) ^ 2
      = n * (n - 1) * (n - 2) * (n - 3) / 4 + n * (n - 1) * (n - 2) + n * (n - 1) / 2 := by
  ring

/-- ノートの式 `eq:var-fixed`：`E[DD²|n]`（式 `eq:dd-second`）から `E[DD|n]²`（式 `eq:dd-mean`）を
引き、`Gq = Gp²` とすると、店の数を固定した分散になる。 -/
theorem var_fixed (n Gp Gt : ℝ) :
    (n * (n - 1) * (n - 2) * (n - 3) / 4 * Gp ^ 2 + n * (n - 1) * (n - 2) * Gt
        + n * (n - 1) / 2 * Gp) - (n * (n - 1) / 2 * Gp) ^ 2
      = n * (n - 1) * (n - 2) * (Gt - Gp ^ 2) + n * (n - 1) / 2 * Gp * (1 - Gp) := by
  ring

/-- 式 `eq:dd-second` と式 `eq:var-fixed` をつないだもの：数え上げから、店の数を固定した分散まで。 -/
theorem var_fixed_of_counting {n : ℕ} (f : Fin n × Fin n → Fin n × Fin n → ℝ) (Gp Gt : ℝ)
    (hsame : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → SamePair P Q → f P Q = Gp)
    (hdisj : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → Disjoint' P Q → f P Q = Gp ^ 2)
    (hone : ∀ P Q, P.1 ≠ P.2 → Q.1 ≠ Q.2 → ¬ SamePair P Q → ¬ Disjoint' P Q → f P Q = Gt) :
    (1 / 4 : ℝ) * ∑ P ∈ (univ : Finset (Fin n)).offDiag, ∑ Q ∈ (univ : Finset (Fin n)).offDiag, f P Q
        - ((n : ℝ) * ((n : ℝ) - 1) / 2 * Gp) ^ 2
      = (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * (Gt - Gp ^ 2)
        + (n : ℝ) * ((n : ℝ) - 1) / 2 * Gp * (1 - Gp) := by
  rw [dd_second_moment f Gp Gt (Gp ^ 2) hsame hdisj hone]
  exact var_fixed n Gp Gt

/-! ## 段階 3：店の数をポアソン分布で揺らした分散 -/

open ProbabilityTheory NNReal Real Nat

/-- 平均 `N` のポアソン分布の重み `e^{-N} N^n / n!`（Mathlib の `poissonMeasure` の各点の重み）。 -/
noncomputable def poissonWeight (N : ℝ≥0) (n : ℕ) : ℝ := exp (-(N : ℝ)) * (N : ℝ) ^ n / (n)!

/-- ノートの式 `eq:poisson-factorial`：ポアソン分布の階乗モーメント `E[n(n-1)⋯(n-k+1)] = N^k`
（重みつきの和として）。 -/
theorem hasSum_poisson_factorial_moment (N : ℝ≥0) (k : ℕ) :
    HasSum (fun n => poissonWeight N n * (n.descFactorial k : ℝ)) ((N : ℝ) ^ k) := by
  rw [← hasSum_nat_add_iff' k]
  have hzero : ∑ i ∈ range k, poissonWeight N i * (i.descFactorial k : ℝ) = 0 := by
    refine sum_eq_zero fun i hi => ?_
    rw [Nat.descFactorial_eq_zero_iff_lt.2 (mem_range.1 hi)]
    simp
  rw [hzero, sub_zero]
  have h := (hasSum_one_poissonMeasure N).mul_left ((N : ℝ) ^ k)
  rw [mul_one] at h
  convert h using 1
  funext m
  have hf : ((m)! : ℝ) * ((m + k).descFactorial k : ℝ) = ((m + k)! : ℝ) := by
    have := Nat.factorial_mul_descFactorial (Nat.le_add_left k m)
    rw [Nat.add_sub_cancel] at this
    exact_mod_cast this
  have hm : ((m)! : ℝ) ≠ 0 := by exact_mod_cast m.factorial_ne_zero
  simp only [poissonWeight]
  rw [← hf]
  field_simp
  ring

/-- 式 `eq:poisson-factorial` を、Mathlib のポアソン分布 `Po(N)` での期待値として書いたもの。 -/
theorem poisson_factorial_moment (N : ℝ≥0) (k : ℕ) :
    ∫ n, (n.descFactorial k : ℝ) ∂Po(N) = (N : ℝ) ^ k := by
  rw [integral_poissonMeasure]
  exact (hasSum_poisson_factorial_moment N k).tsum_eq

/-- `[n(n-1)]² = n(n-1)(n-2)(n-3) + 4 n(n-1)(n-2) + 2 n(n-1)`（式 `eq:count-identity` に 4 をかけたもの）を、
下降階乗で書いたもの。 -/
theorem descFactorial_two_sq (n : ℕ) :
    ((n.descFactorial 2 : ℕ) : ℝ) ^ 2
      = (n.descFactorial 4 : ℝ) + 4 * (n.descFactorial 3 : ℝ) + 2 * (n.descFactorial 2 : ℝ) := by
  rcases lt_or_ge n 4 with hn | hn
  · interval_cases n <;> norm_num [Nat.descFactorial]
  · have h4 : (n.descFactorial 4 : ℝ) = n * (n - 1) * (n - 2) * (n - 3) := by
      simp only [Nat.descFactorial_succ, Nat.descFactorial_zero, Nat.sub_zero]
      push_cast [Nat.cast_sub (by omega : 3 ≤ n), Nat.cast_sub (by omega : 2 ≤ n),
        Nat.cast_sub (by omega : 1 ≤ n)]
      ring
    have h3 : (n.descFactorial 3 : ℝ) = n * (n - 1) * (n - 2) := by
      simp only [Nat.descFactorial_succ, Nat.descFactorial_zero, Nat.sub_zero]
      push_cast [Nat.cast_sub (by omega : 2 ≤ n), Nat.cast_sub (by omega : 1 ≤ n)]
      ring
    have h2 : (n.descFactorial 2 : ℝ) = n * (n - 1) := by
      simp only [Nat.descFactorial_succ, Nat.descFactorial_zero, Nat.sub_zero]
      push_cast [Nat.cast_sub (by omega : 1 ≤ n)]
      ring
    rw [h4, h3, h2]
    ring

/-- ノートの式 `eq:var-poisson`：店の数 `n` が平均 `N` のポアソン分布 `Po(N)` に従うとき、
全分散の公式の右辺 `E[Var[DD|n]] + Var[E[DD|n]]` は `N³ Gt + N²/2 Gp` になる。
`Var[DD|n]` は式 `eq:var-fixed`、`E[DD|n] = n(n-1)/2 Gp` は式 `eq:dd-mean` による。 -/
theorem var_poisson (N : ℝ≥0) (Gp Gt : ℝ) :
    let mean := fun n : ℕ => (n.descFactorial 2 : ℝ) / 2 * Gp
    let varFixed := fun n : ℕ =>
      (n.descFactorial 3 : ℝ) * (Gt - Gp ^ 2) + (n.descFactorial 2 : ℝ) / 2 * Gp * (1 - Gp)
    (∫ n, varFixed n ∂Po(N)) + ((∫ n, mean n ^ 2 ∂Po(N)) - (∫ n, mean n ∂Po(N)) ^ 2)
      = (N : ℝ) ^ 3 * Gt + (N : ℝ) ^ 2 / 2 * Gp := by
  intro mean varFixed
  have m2 := hasSum_poisson_factorial_moment N 2
  have m3 := hasSum_poisson_factorial_moment N 3
  have m4 := hasSum_poisson_factorial_moment N 4
  have hV : HasSum (fun n => poissonWeight N n * varFixed n)
      ((N : ℝ) ^ 3 * (Gt - Gp ^ 2) + (N : ℝ) ^ 2 / 2 * Gp * (1 - Gp)) := by
    convert (m3.mul_right (Gt - Gp ^ 2)).add (m2.mul_right (Gp * (1 - Gp) / 2)) using 1
    · funext n; simp only [varFixed]; ring
    · ring
  have hM : HasSum (fun n => poissonWeight N n * mean n) ((N : ℝ) ^ 2 / 2 * Gp) := by
    convert m2.mul_right (Gp / 2) using 1
    · funext n; simp only [mean]; ring
    · ring
  have hM2 : HasSum (fun n => poissonWeight N n * mean n ^ 2)
      (Gp ^ 2 / 4 * ((N : ℝ) ^ 4 + 4 * (N : ℝ) ^ 3 + 2 * (N : ℝ) ^ 2)) := by
    convert ((m4.add (m3.mul_left 4)).add (m2.mul_left 2)).mul_left (Gp ^ 2 / 4) using 1
    funext n
    simp only [mean]
    have := descFactorial_two_sq n
    calc poissonWeight N n * ((n.descFactorial 2 : ℝ) / 2 * Gp) ^ 2
        = Gp ^ 2 / 4 * (poissonWeight N n * ((n.descFactorial 2 : ℕ) : ℝ) ^ 2) := by ring
      _ = _ := by rw [this]; ring
  simp only [integral_poissonMeasure, smul_eq_mul]
  rw [show (fun n : ℕ => exp (-(N : ℝ)) * (N : ℝ) ^ n / (n)! * varFixed n)
      = fun n => poissonWeight N n * varFixed n from rfl,
    show (fun n : ℕ => exp (-(N : ℝ)) * (N : ℝ) ^ n / (n)! * mean n ^ 2)
      = fun n => poissonWeight N n * mean n ^ 2 from rfl,
    show (fun n : ℕ => exp (-(N : ℝ)) * (N : ℝ) ^ n / (n)! * mean n)
      = fun n => poissonWeight N n * mean n from rfl,
    hV.tsum_eq, hM.tsum_eq, hM2.tsum_eq]
  ring

end RetailCosmology

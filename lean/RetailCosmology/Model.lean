import RetailCosmology.VarDD
import Mathlib.Probability.Independence.Integration
import Mathlib.Probability.Moments.Variance
import Mathlib.MeasureTheory.Integral.Pi
import Mathlib.MeasureTheory.Integral.Prod

/-!
# 点を独立に撒く模型（ノート `notes/retail-cosmology.tex` の付録「DD の分散」、ADR 005）

`n` 個の点が、同じ確率分布 `μ` から独立に撒かれる。確率空間は直積 `Fin n → E` に
直積測度 `Measure.pi (fun _ => μ)` を入れたもので、点 `i` の位置は座標 `ω i` である。
ビンに入るかを表す関数 `θ` は 0 か 1 をとり、二点について対称とする（`BinFn`）。

`binProb`（ノートの `f`）、`Gpair`（`G_p`）、`Gtriple`（`G_t`）は `μ` と `θ` から積分で定義する。
段階 1–3（`VarDD.lean`）では関係ごとの期待値を変数として受け取っていたが、ここではそれを導く。
-/

open MeasureTheory ProbabilityTheory Finset

namespace RetailCosmology

section Model

variable {E : Type*} [MeasurableSpace E]

/-- ビンに入るかを表す関数の条件：可測、0 か 1、二点について対称。 -/
structure BinFn (θ : E → E → ℝ) : Prop where
  meas : Measurable (Function.uncurry θ)
  zero_one : ∀ x y, θ x y = 0 ∨ θ x y = 1
  symm : ∀ x y, θ x y = θ y x

variable (μ : Measure E) [IsProbabilityMeasure μ] (θ : E → E → ℝ)

/-- ノートの式 `eq:f-def`：点 `x` から見て、もう一点がビンに入る確率 `f(x)`。 -/
noncomputable def binProb (x : E) : ℝ := ∫ y, θ x y ∂μ

/-- ノートの式 `eq:g-def` の `G_p = ∫ f dμ`。 -/
noncomputable def Gpair : ℝ := ∫ x, binProb μ θ x ∂μ

/-- ノートの式 `eq:g-def` の `G_t = ∫ f² dμ`。 -/
noncomputable def Gtriple : ℝ := ∫ x, binProb μ θ x ^ 2 ∂μ

/-- `n` 点の確率空間の測度。 -/
noncomputable abbrev pts (n : ℕ) : Measure (Fin n → E) := Measure.pi fun _ => μ

/-- ノートの式 `eq:dd-sum`：`DD = Σ_{i<j} θ_{ij} = (1/2) Σ_{i ≠ j} θ_{ij}`。 -/
noncomputable def DD (n : ℕ) (ω : Fin n → E) : ℝ :=
  (1 / 2 : ℝ) * ∑ P ∈ (univ : Finset (Fin n)).offDiag, θ (ω P.1) (ω P.2)

variable {μ θ}

namespace BinFn

variable (hθ : BinFn θ)
include hθ

lemma nonneg (x y : E) : 0 ≤ θ x y := by
  rcases hθ.zero_one x y with h | h <;> simp [h]

lemma le_one (x y : E) : θ x y ≤ 1 := by
  rcases hθ.zero_one x y with h | h <;> simp [h]

lemma abs_le_one (x y : E) : |θ x y| ≤ 1 :=
  abs_le.2 ⟨by linarith [hθ.nonneg x y], hθ.le_one x y⟩

lemma sq_eq (x y : E) : θ x y * θ x y = θ x y := by
  rcases hθ.zero_one x y with h | h <;> simp [h]

end BinFn

/-! ## 座標の独立性と、二点・三点・四点の期待値 -/

lemma iIndepFun_coord (n : ℕ) :
    iIndepFun (fun (i : Fin n) (ω : Fin n → E) => ω i) (pts μ n) :=
  iIndepFun_pi (X := fun _ (x : E) => x) (fun _ => aemeasurable_id)

lemma map_coord (n : ℕ) (i : Fin n) : (pts μ n).map (fun ω => ω i) = μ :=
  (measurePreserving_eval (fun _ : Fin n => μ) i).map_eq

lemma map_pair (n : ℕ) {i j : Fin n} (hij : i ≠ j) :
    (pts μ n).map (fun ω => (ω i, ω j)) = μ.prod μ := by
  have h := (iIndepFun_coord (μ := μ) n).indepFun hij
  rw [(indepFun_iff_map_prod_eq_prod_map_map (measurable_pi_apply i).aemeasurable
    (measurable_pi_apply j).aemeasurable).1 h, map_coord, map_coord]

/-- 絶対値が 1 以下の二つの数の積も、絶対値が 1 以下。 -/
lemma abs_mul_le_one {a b : ℝ} (ha : |a| ≤ 1) (hb : |b| ≤ 1) : |a * b| ≤ 1 := by
  rw [abs_mul]; nlinarith [abs_nonneg a, abs_nonneg b]

/-- 有界で可測な関数は、確率測度について可積分。 -/
lemma integrable_of_abs_le {α : Type*} [MeasurableSpace α] {ν : Measure α} [IsFiniteMeasure ν]
    {g : α → ℝ} (hg : Measurable g) (C : ℝ) (hC : ∀ a, |g a| ≤ C) : Integrable g ν :=
  Integrable.of_bound hg.aestronglyMeasurable C (ae_of_all _ fun a => by simpa using hC a)

lemma measurable_binProb (hθ : BinFn θ) : Measurable (binProb μ θ) := by
  have h := hθ.meas.stronglyMeasurable.integral_prod_right' (ν := μ)
  exact h.measurable

omit [IsProbabilityMeasure μ] in
lemma binProb_nonneg (hθ : BinFn θ) (x : E) : 0 ≤ binProb μ θ x :=
  integral_nonneg fun y => hθ.nonneg x y

lemma binProb_le_one (hθ : BinFn θ) (x : E) : binProb μ θ x ≤ 1 := by
  calc binProb μ θ x ≤ ∫ _y, (1 : ℝ) ∂μ :=
        integral_mono (integrable_of_abs_le (hθ.meas.comp measurable_prodMk_left) 1
          fun y => hθ.abs_le_one x y) (integrable_const 1) fun y => hθ.le_one x y
    _ = 1 := by simp

/-- ノートの式 `eq:g-def` の一つ目：異なる二点がビンに入る確率は `G_p`。 -/
theorem integral_pair (hθ : BinFn θ) (n : ℕ) {i j : Fin n} (hij : i ≠ j) :
    ∫ ω, θ (ω i) (ω j) ∂(pts μ n) = Gpair μ θ := by
  have hm : Measurable fun ω : Fin n → E => (ω i, ω j) :=
    (measurable_pi_apply i).prodMk (measurable_pi_apply j)
  have hint : Integrable (Function.uncurry θ) (μ.prod μ) :=
    integrable_of_abs_le hθ.meas 1 fun p => hθ.abs_le_one p.1 p.2
  calc ∫ ω, θ (ω i) (ω j) ∂(pts μ n)
      = ∫ p, Function.uncurry θ p ∂((pts μ n).map fun ω => (ω i, ω j)) := by
        rw [integral_map hm.aemeasurable hθ.meas.aestronglyMeasurable]; rfl
    _ = ∫ p, Function.uncurry θ p ∂(μ.prod μ) := by rw [map_pair n hij]
    _ = Gpair μ θ := by rw [integral_prod _ hint]; rfl

/-- ノートの式 `eq:g-def` の二つ目：一点 `i` を共有する二つの組がともにビンに入る確率は `G_t`。 -/
theorem integral_triple (hθ : BinFn θ) (n : ℕ) {i j k : Fin n} (hij : i ≠ j) (hik : i ≠ k)
    (hjk : j ≠ k) :
    ∫ ω, θ (ω i) (ω j) * θ (ω i) (ω k) ∂(pts μ n) = Gtriple μ θ := by
  have hind : IndepFun (fun ω : Fin n → E => (ω j, ω k)) (fun ω => ω i) (pts μ n) :=
    (iIndepFun_coord (μ := μ) n).indepFun_prodMk (fun l => measurable_pi_apply l) j k i
      (Ne.symm hij) (Ne.symm hik)
  have hjk' : Measurable fun ω : Fin n → E => (ω j, ω k) :=
    (measurable_pi_apply j).prodMk (measurable_pi_apply k)
  have hlaw : (pts μ n).map (fun ω => ((ω j, ω k), ω i)) = (μ.prod μ).prod μ := by
    rw [(indepFun_iff_map_prod_eq_prod_map_map hjk'.aemeasurable
      (measurable_pi_apply i).aemeasurable).1 hind, map_pair n hjk, map_coord]
  set g : (E × E) × E → ℝ := fun q => θ q.2 q.1.1 * θ q.2 q.1.2 with hg
  have hgm : Measurable g :=
    (hθ.meas.comp (measurable_snd.prodMk (measurable_fst.comp measurable_fst))).mul
      (hθ.meas.comp (measurable_snd.prodMk (measurable_snd.comp measurable_fst)))
  have hgint : Integrable g ((μ.prod μ).prod μ) :=
    integrable_of_abs_le hgm 1 fun q => by
      exact abs_mul_le_one (hθ.abs_le_one _ _) (hθ.abs_le_one _ _)
  calc ∫ ω, θ (ω i) (ω j) * θ (ω i) (ω k) ∂(pts μ n)
      = ∫ q, g q ∂((pts μ n).map fun ω => ((ω j, ω k), ω i)) := by
        rw [integral_map (hjk'.prodMk (measurable_pi_apply i)).aemeasurable
          hgm.aestronglyMeasurable]
    _ = ∫ q, g q ∂((μ.prod μ).prod μ) := by rw [hlaw]
    _ = ∫ x, ∫ yz, g (yz, x) ∂(μ.prod μ) ∂μ := integral_prod_symm g hgint
    _ = ∫ x, binProb μ θ x ^ 2 ∂μ := by
        refine integral_congr_ae (ae_of_all _ fun x => ?_)
        simp only [hg]
        rw [integral_prod_mul (fun y => θ x y) (fun z => θ x z), binProb, sq]

/-- ノートの式 `eq:g-def` の三つ目：点を共有しない二つの組がともにビンに入る確率は `G_p²`。 -/
theorem integral_quad (hθ : BinFn θ) (n : ℕ) {i j k l : Fin n} (hij : i ≠ j) (hkl : k ≠ l)
    (hik : i ≠ k) (hil : i ≠ l) (hjk : j ≠ k) (hjl : j ≠ l) :
    ∫ ω, θ (ω i) (ω j) * θ (ω k) (ω l) ∂(pts μ n) = Gpair μ θ ^ 2 := by
  have hind := (iIndepFun_coord (μ := μ) n).indepFun_prodMk_prodMk
    (fun m => measurable_pi_apply m) i j k l hik hil hjk hjl
  have h1 : Measurable fun ω : Fin n → E => (ω i, ω j) :=
    (measurable_pi_apply i).prodMk (measurable_pi_apply j)
  have h2 : Measurable fun ω : Fin n → E => (ω k, ω l) :=
    (measurable_pi_apply k).prodMk (measurable_pi_apply l)
  have := hind.integral_fun_comp_mul_comp (f := Function.uncurry θ) (g := Function.uncurry θ)
    h1.aemeasurable h2.aemeasurable hθ.meas.aestronglyMeasurable hθ.meas.aestronglyMeasurable
  simp only [Function.uncurry_apply_pair] at this
  rw [this, integral_pair hθ n hij, integral_pair hθ n hkl, sq]

/-! ## 店の数を固定した `DD` の期待値と分散 -/

lemma measurable_theta_coord (hθ : BinFn θ) (n : ℕ) (i j : Fin n) :
    Measurable fun ω : Fin n → E => θ (ω i) (ω j) := by
  have hm : Measurable fun ω : Fin n → E => (ω i, ω j) :=
    (measurable_pi_apply i).prodMk (measurable_pi_apply j)
  exact hθ.meas.comp hm

lemma integrable_theta_coord (hθ : BinFn θ) (n : ℕ) (i j : Fin n) :
    Integrable (fun ω : Fin n → E => θ (ω i) (ω j)) (pts μ n) :=
  integrable_of_abs_le (measurable_theta_coord hθ n i j) 1 fun _ => hθ.abs_le_one _ _

lemma integrable_theta_mul (hθ : BinFn θ) (n : ℕ) (P Q : Fin n × Fin n) :
    Integrable (fun ω : Fin n → E => θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2)) (pts μ n) :=
  integrable_of_abs_le ((measurable_theta_coord hθ n _ _).mul (measurable_theta_coord hθ n _ _)) 1
    fun _ => abs_mul_le_one (hθ.abs_le_one _ _) (hθ.abs_le_one _ _)

/-- ノートの式 `eq:dd-mean`：`E[DD | n] = n(n-1)/2 · G_p`。 -/
theorem integral_DD (hθ : BinFn θ) (n : ℕ) :
    ∫ ω, DD θ n ω ∂(pts μ n) = (n : ℝ) * ((n : ℝ) - 1) / 2 * Gpair μ θ := by
  simp only [DD]
  rw [integral_const_mul, integral_finsetSum _ fun P _ => integrable_theta_coord hθ n P.1 P.2,
    sum_congr rfl fun P hP => integral_pair hθ n (mem_offDiag.1 hP).2.2, sum_const, nsmul_eq_mul,
    card_offDiag]
  ring

/-- 二つの組 `P`、`Q` の関係ごとの期待値 `E[θ_P θ_Q]`（段階 1 の数え上げに渡す）。 -/
lemma integral_theta_mul_cases (hθ : BinFn θ) (n : ℕ) (P Q : Fin n × Fin n) (hP : P.1 ≠ P.2)
    (hQ : Q.1 ≠ Q.2) :
    (SamePair P Q → ∫ ω, θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2) ∂(pts μ n) = Gpair μ θ) ∧
    (Disjoint' P Q → ∫ ω, θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2) ∂(pts μ n) = Gpair μ θ ^ 2) ∧
    (¬ SamePair P Q → ¬ Disjoint' P Q →
      ∫ ω, θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2) ∂(pts μ n) = Gtriple μ θ) := by
  obtain ⟨a, b⟩ := P
  obtain ⟨c, d⟩ := Q
  simp only at hP hQ ⊢
  -- θ の対称性で、座標の順を入れ替える
  have sw : ∀ u v : Fin n, (fun ω : Fin n → E => θ (ω u) (ω v)) = fun ω => θ (ω v) (ω u) :=
    fun u v => funext fun ω => hθ.symm _ _
  refine ⟨?_, ?_, ?_⟩
  · intro h
    simp only [SamePair, Prod.mk.injEq, Prod.swap_prod_mk] at h
    rcases h with ⟨hc, hd⟩ | ⟨hc, hd⟩
    · rw [hc, hd]
      simp_rw [hθ.sq_eq]
      exact integral_pair hθ n hP
    · rw [hc, hd]
      have : ∀ ω : Fin n → E, θ (ω a) (ω b) * θ (ω b) (ω a) = θ (ω a) (ω b) := fun ω => by
        rw [hθ.symm (ω b) (ω a), hθ.sq_eq]
      simp_rw [this]
      exact integral_pair hθ n hP
  · rintro ⟨h1, h2, h3, h4⟩
    exact integral_quad hθ n hP hQ (Ne.symm h1) (Ne.symm h3) (Ne.symm h2) (Ne.symm h4)
  · intro hns hnd
    simp only [SamePair, Prod.mk.injEq, Prod.swap_prod_mk, not_or, not_and] at hns
    simp only [Disjoint', not_and, not_not] at hnd
    by_cases hca : c = a
    · rw [hca] at hQ ⊢
      have hdb : d ≠ b := hns.1 hca
      exact integral_triple hθ n hP hQ (Ne.symm hdb)
    · by_cases hcb : c = b
      · rw [hcb] at hQ ⊢
        have hda : d ≠ a := hns.2 hcb
        have : ∀ ω : Fin n → E, θ (ω a) (ω b) * θ (ω b) (ω d) = θ (ω b) (ω a) * θ (ω b) (ω d) :=
          fun ω => by rw [hθ.symm (ω a) (ω b)]
        simp_rw [this]
        exact integral_triple hθ n (Ne.symm hP) hQ (Ne.symm hda)
      · by_cases hda : d = a
        · rw [hda] at hQ ⊢
          have : ∀ ω : Fin n → E, θ (ω a) (ω b) * θ (ω c) (ω a) = θ (ω a) (ω b) * θ (ω a) (ω c) :=
            fun ω => by rw [hθ.symm (ω c) (ω a)]
          simp_rw [this]
          exact integral_triple hθ n hP (Ne.symm hQ) (Ne.symm hcb)
        · have hdb : d = b := hnd hca hcb hda
          rw [hdb] at hQ ⊢
          have : ∀ ω : Fin n → E,
              θ (ω a) (ω b) * θ (ω c) (ω b) = θ (ω b) (ω a) * θ (ω b) (ω c) := fun ω => by
            rw [hθ.symm (ω a) (ω b), hθ.symm (ω c) (ω b)]
          simp_rw [this]
          exact integral_triple hθ n (Ne.symm hP) (Ne.symm hQ) (Ne.symm hca)

/-- ノートの式 `eq:dd-second` を、模型から導いたもの：`E[DD² | n]`。 -/
theorem integral_DD_sq (hθ : BinFn θ) (n : ℕ) :
    ∫ ω, DD θ n ω ^ 2 ∂(pts μ n)
      = (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * ((n : ℝ) - 3) / 4 * Gpair μ θ ^ 2
        + (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * Gtriple μ θ
        + (n : ℝ) * ((n : ℝ) - 1) / 2 * Gpair μ θ := by
  have hsq : ∀ ω : Fin n → E, DD θ n ω ^ 2 = (1 / 4 : ℝ) * ∑ P ∈ (univ : Finset (Fin n)).offDiag,
      ∑ Q ∈ (univ : Finset (Fin n)).offDiag, θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2) := by
    intro ω
    simp only [DD]
    rw [mul_pow, sq (∑ P ∈ _, _), sum_mul_sum]
    norm_num
  simp_rw [hsq]
  rw [integral_const_mul, integral_finsetSum _ fun P _ =>
    integrable_finsetSum _ fun Q _ => integrable_theta_mul hθ n P Q]
  simp_rw [integral_finsetSum _ fun Q _ => integrable_theta_mul hθ n _ Q]
  exact dd_second_moment
    (fun P Q => ∫ ω, θ (ω P.1) (ω P.2) * θ (ω Q.1) (ω Q.2) ∂(pts μ n))
    (Gpair μ θ) (Gtriple μ θ) (Gpair μ θ ^ 2)
    (fun P Q hP hQ => (integral_theta_mul_cases hθ n P Q hP hQ).1)
    (fun P Q hP hQ => (integral_theta_mul_cases hθ n P Q hP hQ).2.1)
    (fun P Q hP hQ => (integral_theta_mul_cases hθ n P Q hP hQ).2.2)

lemma measurable_DD (hθ : BinFn θ) (n : ℕ) : Measurable (DD θ n) :=
  measurable_const.mul (Finset.measurable_sum _ fun P _ => measurable_theta_coord hθ n P.1 P.2)

/-- `|DD| ≤ n²`（組の数で抑える）。 -/
lemma abs_DD_le (hθ : BinFn θ) (n : ℕ) (ω : Fin n → E) : |DD θ n ω| ≤ (n : ℝ) ^ 2 := by
  rw [DD, abs_mul]
  calc |(1 / 2 : ℝ)| * |∑ P ∈ (univ : Finset (Fin n)).offDiag, θ (ω P.1) (ω P.2)|
      ≤ 1 * ∑ P ∈ (univ : Finset (Fin n)).offDiag, |θ (ω P.1) (ω P.2)| := by
        gcongr
        · norm_num
        · exact abs_sum_le_sum_abs _ _
    _ ≤ 1 * ∑ _P ∈ (univ : Finset (Fin n)).offDiag, (1 : ℝ) := by
        gcongr with P; exact hθ.abs_le_one _ _
    _ ≤ (n : ℝ) ^ 2 := by
        rw [one_mul, sum_const, nsmul_one, offDiag_card, card_univ, Fintype.card_fin]
        have : n * n - n ≤ n ^ 2 := by rw [sq]; exact Nat.sub_le _ _
        exact_mod_cast this

lemma memLp_DD (hθ : BinFn θ) (n : ℕ) : MemLp (DD θ n) 2 (pts μ n) :=
  MemLp.of_bound (measurable_DD hθ n).aestronglyMeasurable ((n : ℝ) ^ 2)
    (ae_of_all _ fun ω => by simpa [Real.norm_eq_abs] using abs_DD_le hθ n ω)

/-- ノートの式 `eq:var-fixed` を、点を独立に撒く模型から導いたもの。 -/
theorem variance_DD (hθ : BinFn θ) (n : ℕ) :
    variance (DD θ n) (pts μ n)
      = (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * (Gtriple μ θ - Gpair μ θ ^ 2)
        + (n : ℝ) * ((n : ℝ) - 1) / 2 * Gpair μ θ * (1 - Gpair μ θ) := by
  rw [variance_eq_sub (memLp_DD hθ n)]
  simp only [Pi.pow_apply]
  rw [integral_DD_sq hθ n, integral_DD hθ n]
  ring

/-- ノートの式 `eq:gt-var`：`G_t - G_p²` は、点の位置で見た `f` の分散。 -/
theorem Gtriple_sub_sq (hθ : BinFn θ) :
    Gtriple μ θ - Gpair μ θ ^ 2 = ∫ x, (binProb μ θ x - Gpair μ θ) ^ 2 ∂μ := by
  have hb := measurable_binProb (μ := μ) hθ
  have hint1 : Integrable (binProb μ θ) μ :=
    integrable_of_abs_le hb 1 fun x => abs_le.2 ⟨by linarith [binProb_nonneg (μ := μ) hθ x],
      binProb_le_one hθ x⟩
  have hint2 : Integrable (fun x => binProb μ θ x ^ 2) μ :=
    integrable_of_abs_le (hb.pow_const 2) 1 fun x => by
      rw [abs_of_nonneg (sq_nonneg _)]
      exact pow_le_one₀ (binProb_nonneg hθ x) (binProb_le_one hθ x)
  have : ∀ x, (binProb μ θ x - Gpair μ θ) ^ 2
      = binProb μ θ x ^ 2 - 2 * Gpair μ θ * binProb μ θ x + Gpair μ θ ^ 2 := fun x => by ring
  simp_rw [this]
  have hsub : Integrable (fun x => binProb μ θ x ^ 2 - 2 * Gpair μ θ * binProb μ θ x) μ :=
    hint2.sub (hint1.const_mul _)
  calc Gtriple μ θ - Gpair μ θ ^ 2
      = (∫ x, binProb μ θ x ^ 2 ∂μ - 2 * Gpair μ θ * ∫ x, binProb μ θ x ∂μ) + Gpair μ θ ^ 2 := by
        simp only [Gtriple, Gpair]; ring
    _ = (∫ x, binProb μ θ x ^ 2 - 2 * Gpair μ θ * binProb μ θ x ∂μ) + ∫ _x, Gpair μ θ ^ 2 ∂μ := by
        rw [integral_sub hint2 (hint1.const_mul _), integral_const_mul]; simp
    _ = ∫ x, binProb μ θ x ^ 2 - 2 * Gpair μ θ * binProb μ θ x + Gpair μ θ ^ 2 ∂μ :=
        (integral_add hsub (integrable_const _)).symm

/-! ## ランダムカタログから `G_p`、`G_t` を偏りなく求める（ノートの式 `eq:g-estimate`） -/

/-- 点 `i` から見てビンに入る点の数 `c_i`。 -/
noncomputable def binCount (θ : E → E → ℝ) (n : ℕ) (ω : Fin n → E) (i : Fin n) : ℝ :=
  ∑ j ∈ (univ : Finset (Fin n)).erase i, θ (ω i) (ω j)

lemma card_erase_univ {n : ℕ} (i : Fin n) : (((univ : Finset (Fin n)).erase i).card : ℝ) = n - 1 := by
  rw [card_erase_of_mem (mem_univ i), card_univ, Fintype.card_fin,
    Nat.cast_sub (Nat.one_le_iff_ne_zero.2 (Fin.pos i).ne')]
  simp

lemma card_erase_erase {n : ℕ} {i j : Fin n} (hj : j ∈ (univ : Finset (Fin n)).erase i) :
    ((((univ : Finset (Fin n)).erase i).erase j).card : ℝ) = n - 2 := by
  have hji : j ≠ i := (mem_erase.1 hj).1
  have hn : 2 ≤ n := by
    have := card_le_univ ({i, j} : Finset (Fin n))
    rw [card_pair hji.symm, Fintype.card_fin] at this
    exact this
  rw [card_erase_of_mem hj, card_erase_of_mem (mem_univ i), card_univ, Fintype.card_fin]
  rw [Nat.cast_sub (by omega), Nat.cast_sub (by omega)]
  ring

/-- `E[Σ_i c_i] = n(n-1) G_p`。`Σ_i c_i / [n(n-1)]` は `G_p` を偏りなく求める。 -/
theorem integral_sum_binCount (hθ : BinFn θ) (n : ℕ) :
    ∫ ω, ∑ i, binCount θ n ω i ∂(pts μ n) = (n : ℝ) * ((n : ℝ) - 1) * Gpair μ θ := by
  simp only [binCount]
  rw [integral_finsetSum _ fun i _ => integrable_finsetSum _ fun j _ => integrable_theta_coord hθ n i j]
  simp_rw [integral_finsetSum _ fun j _ => integrable_theta_coord hθ n _ j]
  rw [sum_congr rfl fun i _ => sum_congr rfl fun j hj =>
    integral_pair hθ n (Ne.symm (mem_erase.1 hj).1)]
  simp only [sum_const, nsmul_eq_mul, card_erase_univ, card_univ, Fintype.card_fin]
  ring

/-- `c_i (c_i - 1) = Σ_{j ≠ i} Σ_{k ≠ i, j} θ_{ij} θ_{ik}`（`θ² = θ` を使う）。 -/
lemma binCount_mul_sub_one (hθ : BinFn θ) (n : ℕ) (ω : Fin n → E) (i : Fin n) :
    binCount θ n ω i * (binCount θ n ω i - 1)
      = ∑ j ∈ (univ : Finset (Fin n)).erase i, ∑ k ∈ ((univ : Finset (Fin n)).erase i).erase j,
          θ (ω i) (ω j) * θ (ω i) (ω k) := by
  have h : ∀ j ∈ (univ : Finset (Fin n)).erase i,
      ∑ k ∈ (univ : Finset (Fin n)).erase i, θ (ω i) (ω j) * θ (ω i) (ω k)
        = θ (ω i) (ω j) + ∑ k ∈ ((univ : Finset (Fin n)).erase i).erase j,
            θ (ω i) (ω j) * θ (ω i) (ω k) := fun j hj => by
    rw [← add_sum_erase _ _ hj, hθ.sq_eq]
  simp only [binCount]
  rw [mul_sub, mul_one, sum_mul_sum, sum_congr rfl h, sum_add_distrib]
  ring

/-- `E[Σ_i c_i (c_i - 1)] = n(n-1)(n-2) G_t`。`Σ_i c_i (c_i - 1) / [n(n-1)(n-2)]` は `G_t` を偏りなく求める。 -/
theorem integral_sum_binCount_mul (hθ : BinFn θ) (n : ℕ) :
    ∫ ω, ∑ i, binCount θ n ω i * (binCount θ n ω i - 1) ∂(pts μ n)
      = (n : ℝ) * ((n : ℝ) - 1) * ((n : ℝ) - 2) * Gtriple μ θ := by
  simp_rw [binCount_mul_sub_one hθ n]
  have hint : ∀ i j k : Fin n,
      Integrable (fun ω : Fin n → E => θ (ω i) (ω j) * θ (ω i) (ω k)) (pts μ n) :=
    fun i j k => integrable_theta_mul hθ n (i, j) (i, k)
  rw [integral_finsetSum _ fun i _ => integrable_finsetSum _ fun j _ =>
    integrable_finsetSum _ fun k _ => hint i j k]
  simp_rw [integral_finsetSum _ fun j _ => integrable_finsetSum _ fun k _ => hint _ j k]
  simp_rw [integral_finsetSum _ fun k _ => hint _ _ k]
  rw [sum_congr rfl fun i _ => sum_congr rfl fun j hj => sum_congr rfl fun k hk =>
    integral_triple hθ n (Ne.symm (mem_erase.1 hj).1) (Ne.symm (mem_erase.1 (mem_erase.1 hk).2).1)
      (Ne.symm (mem_erase.1 hk).1)]
  rw [sum_congr rfl fun i _ => sum_congr rfl fun j hj => by
    rw [sum_const, nsmul_eq_mul, card_erase_erase hj]]
  simp only [sum_const, nsmul_eq_mul, card_erase_univ, card_univ, Fintype.card_fin]
  ring

end Model

end RetailCosmology

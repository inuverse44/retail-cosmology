import RetailCosmology.Model

/-!
# 店の数をポアソン分布で揺らす模型（ノートの付録「DD の分散」、ADR 005 の Decision 項 2）

確率空間は、店の数 `m` と、`m` 個の点の配置の組 `Σ m, (Fin m → E)` である。
測度 `mix N μ` は、`m` を平均 `N` のポアソン分布で選び、`m` 個の点を `μ` から独立に撒くもの：
`mix N μ = Σ_m Po(m; N) · (m 点の直積測度を Σ 型に送ったもの)`。
店の数 `s.1` と `DD` は、この一つの確率空間の上の確率変数である。
-/

open MeasureTheory ProbabilityTheory Finset NNReal

namespace RetailCosmology

section Poisson

variable {E : Type*} [MeasurableSpace E]

/-- 店の数と点の配置の組。 -/
abbrev Conf (E : Type*) := Σ m : ℕ, (Fin m → E)

lemma measurable_sigmaMk' (m : ℕ) : Measurable fun ω : Fin m → E => (⟨m, ω⟩ : Conf E) :=
  fun _ ht => MeasurableSpace.measurableSet_iInf.1 ht m

/-- 店の数ごとに可測なら、`Conf E` の上で可測。 -/
lemma measurable_of_fiber {g : Conf E → ℝ} (hg : ∀ m, Measurable fun ω => g ⟨m, ω⟩) :
    Measurable g :=
  fun _ hs => MeasurableSpace.measurableSet_iInf.2 fun m => hg m hs

variable (μ : Measure E) [IsProbabilityMeasure μ]

/-- 店の数を平均 `N` のポアソン分布で選び、点を `μ` から独立に撒く測度。 -/
noncomputable def mix (N : ℝ≥0) : Measure (Conf E) :=
  Measure.sum fun m => ENNReal.ofReal (poissonWeight N m) • (pts μ m).map fun ω => (⟨m, ω⟩ : Conf E)

/-- `Conf E` の上の `DD`：店の数 `s.1` の点の配置 `s.2` について数える。 -/
noncomputable def DDmix (θ : E → E → ℝ) (s : Conf E) : ℝ := DD θ s.1 s.2

variable {μ}

lemma poissonWeight_nonneg (N : ℝ≥0) (m : ℕ) : 0 ≤ poissonWeight N m := by
  unfold poissonWeight; positivity

/-- 下降階乗を実数の積で書く：`m(m-1)⋯(m-k+1)`。 -/
lemma descFactorial_cast (m k : ℕ) :
    (m.descFactorial k : ℝ) = ∏ i ∈ range k, ((m : ℝ) - i) := by
  induction k with
  | zero => simp
  | succ k ih =>
    rw [prod_range_succ, ← ih, Nat.descFactorial_succ, Nat.cast_mul]
    by_cases hk : k ≤ m
    · rw [Nat.cast_sub hk]; ring
    · have h0 : m.descFactorial k = 0 := Nat.descFactorial_eq_zero_iff_lt.2 (by omega)
      simp [h0]

/-- ポアソン分布の重みをかけた多項式の和。 -/
lemma hasSum_poisson_poly (N : ℝ≥0) (a b c d : ℝ) :
    HasSum (fun m : ℕ => poissonWeight N m *
        (a * (m.descFactorial 4 : ℝ) + b * (m.descFactorial 3 : ℝ) + c * (m.descFactorial 2 : ℝ)
          + d * (m.descFactorial 1 : ℝ)))
      (a * (N : ℝ) ^ 4 + b * (N : ℝ) ^ 3 + c * (N : ℝ) ^ 2 + d * (N : ℝ) ^ 1) := by
  have h := ((((hasSum_poisson_factorial_moment N 4).mul_left a).add
    ((hasSum_poisson_factorial_moment N 3).mul_left b)).add
    ((hasSum_poisson_factorial_moment N 2).mul_left c)).add
    ((hasSum_poisson_factorial_moment N 1).mul_left d)
  convert h using 1
  funext m; ring

/-- `Po(m) m^4` は和が有限。 -/
lemma summable_poisson_pow_four (N : ℝ≥0) :
    Summable fun m : ℕ => poissonWeight N m * (m : ℝ) ^ 4 := by
  convert (hasSum_poisson_poly N 1 6 7 1).summable using 1
  funext m
  simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
  ring

lemma summable_poisson_sq (N : ℝ≥0) :
    Summable fun m : ℕ => poissonWeight N m * (m : ℝ) ^ 2 := by
  convert (hasSum_poisson_poly N 0 0 1 1).summable using 1
  funext m
  simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
  ring

/-- 店の数ごとに有界で、その上界に重みをかけた和が有限なら、`mix` について可積分で、
積分は店の数ごとの積分の重みつきの和になる。 -/
lemma integrable_integral_mix (N : ℝ≥0) {g : Conf E → ℝ}
    (hg : ∀ m, Measurable fun ω => g ⟨m, ω⟩) (B : ℕ → ℝ) (hB : ∀ m ω, |g ⟨m, ω⟩| ≤ B m)
    (hsum : Summable fun m => poissonWeight N m * B m) :
    Integrable g (mix μ N) ∧
      ∫ s, g s ∂(mix μ N) = ∑' m, poissonWeight N m * ∫ ω, g ⟨m, ω⟩ ∂(pts μ m) := by
  have hgm := measurable_of_fiber hg
  have hfib : ∀ m, Integrable (fun ω => g ⟨m, ω⟩) (pts μ m) := fun m =>
    integrable_of_abs_le (hg m) (B m) (hB m)
  have hmap : ∀ m, Integrable g ((pts μ m).map fun ω => (⟨m, ω⟩ : Conf E)) := fun m =>
    (integrable_map_measure hgm.aestronglyMeasurable (measurable_sigmaMk' m).aemeasurable).2
      (hfib m)
  have hint_fib : ∀ m,
      ∫ s, g s ∂(ENNReal.ofReal (poissonWeight N m) • (pts μ m).map fun ω => (⟨m, ω⟩ : Conf E))
      = poissonWeight N m * ∫ ω, g ⟨m, ω⟩ ∂(pts μ m) := fun m => by
    rw [integral_smul_measure, integral_map (measurable_sigmaMk' m).aemeasurable
      hgm.aestronglyMeasurable, ENNReal.toReal_ofReal (poissonWeight_nonneg N m), smul_eq_mul]
  have hnorm_fib : ∀ m,
      ∫ s, ‖g s‖ ∂(ENNReal.ofReal (poissonWeight N m) • (pts μ m).map fun ω => (⟨m, ω⟩ : Conf E))
        = poissonWeight N m * ∫ ω, |g ⟨m, ω⟩| ∂(pts μ m) := fun m => by
    rw [integral_smul_measure, integral_map (measurable_sigmaMk' m).aemeasurable
      hgm.norm.aestronglyMeasurable, ENNReal.toReal_ofReal (poissonWeight_nonneg N m), smul_eq_mul]
    rfl
  have hI : Integrable g (mix μ N) := by
    refine integrable_sum_measure (fun m => (hmap m).smul_measure ENNReal.ofReal_ne_top) ?_
    refine Summable.of_nonneg_of_le (fun m => integral_nonneg fun _ => norm_nonneg _)
      (fun m => ?_) hsum
    rw [hnorm_fib]
    gcongr
    · exact poissonWeight_nonneg N m
    · calc ∫ ω, |g ⟨m, ω⟩| ∂(pts μ m) ≤ ∫ _ω, B m ∂(pts μ m) :=
            integral_mono (hfib m).abs (integrable_const _) (hB m)
        _ = B m := by simp
  refine ⟨hI, ?_⟩
  rw [mix, integral_sum_measure hI]
  exact tsum_congr hint_fib

instance (N : ℝ≥0) : IsProbabilityMeasure (mix μ N) := by
  constructor
  rw [mix, Measure.sum_apply _ MeasurableSet.univ]
  simp only [Measure.smul_apply, Measure.map_apply (measurable_sigmaMk' _) MeasurableSet.univ,
    Set.preimage_univ, measure_univ, smul_eq_mul, mul_one]
  -- 重み `poissonWeight` と Mathlib のポアソン分布の重みは同じ式
  have h1 : HasSum (fun n => poissonWeight N n) 1 := hasSum_one_poissonMeasure N
  rw [← ENNReal.ofReal_tsum_of_nonneg (poissonWeight_nonneg N) h1.summable, h1.tsum_eq]
  simp

variable {θ : E → E → ℝ}

lemma integrable_integral_DDmix (hθ : BinFn θ) (N : ℝ≥0) :
    Integrable (DDmix θ) (mix μ N) ∧
      ∫ s, DDmix θ s ∂(mix μ N) = (N : ℝ) ^ 2 / 2 * Gpair μ θ := by
  obtain ⟨hI, hE⟩ := integrable_integral_mix (μ := μ) N (g := DDmix θ)
    (fun m => measurable_DD hθ m) (fun m => (m : ℝ) ^ 2) (fun m ω => abs_DD_le hθ m ω)
    (summable_poisson_sq N)
  refine ⟨hI, ?_⟩
  rw [hE]
  simp only [DDmix, integral_DD hθ]
  have h := hasSum_poisson_poly N 0 0 (Gpair μ θ / 2) 0
  convert h.tsum_eq using 1
  · congr 1; funext m
    simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
    ring
  · ring

lemma integrable_integral_DDmix_sq (hθ : BinFn θ) (N : ℝ≥0) :
    Integrable (fun s => DDmix θ s ^ 2) (mix μ N) ∧
      ∫ s, DDmix θ s ^ 2 ∂(mix μ N)
        = (N : ℝ) ^ 4 / 4 * Gpair μ θ ^ 2 + (N : ℝ) ^ 3 * Gtriple μ θ
          + (N : ℝ) ^ 2 / 2 * Gpair μ θ := by
  obtain ⟨hI, hE⟩ := integrable_integral_mix (μ := μ) N (g := fun s => DDmix θ s ^ 2)
    (fun m => (measurable_DD hθ m).pow_const 2) (fun m => (m : ℝ) ^ 4)
    (fun m ω => by
      rw [abs_pow, show (m : ℝ) ^ 4 = ((m : ℝ) ^ 2) ^ 2 by ring]
      exact pow_le_pow_left₀ (abs_nonneg _) (abs_DD_le hθ m ω) 2)
    (summable_poisson_pow_four N)
  refine ⟨hI, ?_⟩
  rw [hE]
  simp only [DDmix, integral_DD_sq hθ]
  have h := hasSum_poisson_poly N (Gpair μ θ ^ 2 / 4) (Gtriple μ θ) (Gpair μ θ / 2) 0
  convert h.tsum_eq using 1
  · congr 1; funext m
    simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
    ring
  · ring

/-- ノートの式 `eq:var-poisson` を、模型から導いたもの：店の数をポアソン分布で揺らしたときの
`DD` の全分散は `N³ G_t + N²/2 · G_p`。 -/
theorem variance_DDmix (hθ : BinFn θ) (N : ℝ≥0) :
    variance (DDmix θ) (mix μ N) = (N : ℝ) ^ 3 * Gtriple μ θ + (N : ℝ) ^ 2 / 2 * Gpair μ θ := by
  obtain ⟨hI2, hE2⟩ := integrable_integral_DDmix_sq (μ := μ) hθ N
  obtain ⟨_, hE1⟩ := integrable_integral_DDmix (μ := μ) hθ N
  have hm : Measurable (DDmix θ : Conf E → ℝ) := measurable_of_fiber fun m => measurable_DD hθ m
  have hL2 : MemLp (DDmix θ) 2 (mix μ N) :=
    (memLp_two_iff_integrable_sq hm.aestronglyMeasurable).2 hI2
  rw [variance_eq_sub hL2]
  simp only [Pi.pow_apply]
  rw [hE2, hE1]
  ring

/-- ノートの式 `eq:total-var`（全分散の公式）を、この模型で確かめたもの：
`DD` の全分散は、店の数を固定した分散のポアソン平均と、条件つき期待値のポアソン分布での分散の和。 -/
theorem variance_DDmix_eq_total (hθ : BinFn θ) (N : ℝ≥0) :
    variance (DDmix θ) (mix μ N)
      = (∫ m, variance (DD θ m) (pts μ m) ∂(poissonMeasure N))
        + ((∫ m, (∫ ω, DD θ m ω ∂(pts μ m)) ^ 2 ∂(poissonMeasure N))
          - (∫ m, ∫ ω, DD θ m ω ∂(pts μ m) ∂(poissonMeasure N)) ^ 2) := by
  rw [variance_DDmix hθ N]
  have hv : ∀ m : ℕ, variance (DD θ m) (pts μ m)
      = (m.descFactorial 3 : ℝ) * (Gtriple μ θ - Gpair μ θ ^ 2)
        + (m.descFactorial 2 : ℝ) / 2 * Gpair μ θ * (1 - Gpair μ θ) := fun m => by
    rw [variance_DD hθ m]
    simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
    ring
  have he : ∀ m : ℕ, ∫ ω, DD θ m ω ∂(pts μ m) = (m.descFactorial 2 : ℝ) / 2 * Gpair μ θ :=
    fun m => by
      rw [integral_DD hθ m]
      simp only [descFactorial_cast, prod_range_succ, prod_range_zero]
      ring
  simp_rw [hv, he]
  exact (var_poisson N (Gpair μ θ) (Gtriple μ θ)).symm

end Poisson

end RetailCosmology

import Aeneas
import CertoraVault

/-! # Reusable `@[step]` specs for the `certora_vault` primitives

Exhaustive (per Rust outcome) specifications of the arithmetic helper `mul_div_floor`, the
conversions, the share/asset mutators and the fee helpers. Every function here is total in the
abort monad; each spec says exactly which Rust outcome is produced and what the vault becomes. -/

open Aeneas Aeneas.Std Result
open certora_vault

namespace CertoraVault.VaultLemmas

attribute [local step] UScalar.cast_inBounds_spec
attribute [-step] UScalar.cast.step_spec

theorem U64_max_eq : U64.max = 18446744073709551615 := by simp [U64.max_def, U64.numBits]

theorem U128_max_eq : U128.max = 340282366920938463463374607431768211455 := by
  simp [U128.max_def, U128.numBits]

theorem U64_rMax_eq : U64.rMax = U64.max := by simp [U64_max_eq, U64.rMax]

theorem u64_mul_le (a b : U64) : a.val * b.val ≤ U128.max := by
  have ha : a.val ≤ U64.max := by scalar_tac
  have hb : b.val ≤ U64.max := by scalar_tac
  have := Nat.mul_le_mul ha hb
  simp only [U64_max_eq, U128_max_eq] at *
  agrind

@[simp]
theorem from_residual_FromSame_Err (T E : Type) (e : E) :
    core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual T
      (core.convert.FromSame E) (.Err e) = ok (.Err e) := by
  simp [core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual,
    core.convert.FromSame.from]

@[simp]
theorem branch_Ok {T E : Type} (x : T) :
    core.result.Result.Insts.CoreOpsTry.branch (.Ok x : core.result.Result T E) =
      ok (.Continue x) := rfl

@[simp]
theorem branch_Err {T E : Type} (e : E) :
    core.result.Result.Insts.CoreOpsTry.branch (.Err e : core.result.Result T E) =
      ok (.Break (.Err e)) := rfl

@[simp] theorem num_shares_eq (v : state.Vault) : state.Vault.num_shares v = ok v.shares := rfl
@[simp] theorem num_assets_eq (v : state.Vault) : state.Vault.num_assets v = ok v.assets := rfl
@[simp] theorem fee_amount_eq (v : state.Vault) :
    state.Vault.impl.fee_amount v = ok v.fee_amount := rfl
@[simp] theorem clear_fee_amount_eq (v : state.Vault) :
    state.Vault.clear_fee_amount v = ok { v with fee_amount := 0#u64 } := rfl

/-- The all-zero effect returned by `VaultEffect::default`. -/
def zeroEffect : operations.vault_operations.VaultEffect :=
  { shares_to_burn := 0#u64, shares_to_user := 0#u64, assets_to_vault := 0#u64,
    assets_to_user := 0#u64, assets_to_fee := 0#u64 }

@[simp] theorem zeroEffect_shares_to_burn : zeroEffect.shares_to_burn.val = 0 := rfl
@[simp] theorem zeroEffect_shares_to_user : zeroEffect.shares_to_user.val = 0 := rfl
@[simp] theorem zeroEffect_assets_to_vault : zeroEffect.assets_to_vault.val = 0 := rfl
@[simp] theorem zeroEffect_assets_to_user : zeroEffect.assets_to_user.val = 0 := rfl
@[simp] theorem zeroEffect_assets_to_fee : zeroEffect.assets_to_fee.val = 0 := rfl

@[simp]
theorem VaultEffect_default_eq :
    operations.vault_operations.VaultEffect.Insts.CoreDefaultDefault.default = ok zeroEffect := rfl

/-- **Spec theorem for `certora_vault::utils::math::math_native::mul_div_floor`**
`floor(a * b / c)` computed in `u128`: `Ok` exactly when `c ≠ 0` and the quotient fits in `u64`,
otherwise `Err(MathOverflow)`. -/
@[step]
theorem mul_div_floor.spec (a b c : U64) :
    utils.math.math_native.mul_div_floor a b c ⦃ (r : core.result.Result U64 errors.VaultError) =>
      match r with
      | .Ok z => c.val ≠ 0 ∧ z.val = a.val * b.val / c.val
      | .Err e => e = errors.VaultError.MathOverflow ∧
          (c.val = 0 ∨ U64.max < a.val * b.val / c.val) ⦄ := by
  have hab := u64_mul_le a b
  unfold utils.math.math_native.mul_div_floor
  step as ⟨i, hi⟩
  step as ⟨i1, hi1⟩
  step as ⟨o, ho⟩
  rcases o with _ | p
  · simp only [hi, hi1] at ho
    agrind
  · simp only [core.result.Result.Insts.CoreOpsTry.branch, core.option.Option.ok_or]
    simp only at ho
    step as ⟨i2, hi2⟩
    step as ⟨o1, ho1⟩
    rcases o1 with _ | q
    · simp [core.result.Result.Insts.CoreOpsTry_traitFromResidualResult.from_residual]
      simp only at ho1
      agrind
    · simp only
      simp only at ho1
      step as ⟨i3, hi3⟩
      · simp [core.num.U64.MAX, U64_rMax_eq, U128_max_eq, U64_max_eq]
      rw [U64_rMax_eq] at hi3
      split
      · have : q.val > i3.val := by scalar_tac
        simp only [WP.spec_ok, true_and]
        right
        rw [← hi3, ← hi, ← hi1, ← hi2, ← ho.2.1, ← ho1.2.1]
        exact this
      · have : q.val ≤ i3.val := by scalar_tac
        step as ⟨i4, hi4⟩
        refine ⟨by agrind, ?_⟩
        rw [hi4, ho1.2.1, ho.2.1, hi, hi1, hi2]

/-- **Spec theorem for `Vault::convert_assets_to_shares`** -/
@[step]
theorem convert_assets_to_shares.spec (v : state.Vault) (x : U64) :
    state.Vault.convert_assets_to_shares v x ⦃ (r : core.result.Result U64 errors.VaultError) =>
      match r with
      | .Ok y =>
        (v.shares.val = v.assets.val → y = x) ∧
        (v.shares.val ≠ v.assets.val →
          v.assets.val ≠ 0 ∧ y.val = x.val * v.shares.val / v.assets.val)
      | .Err e => e = errors.VaultError.MathOverflow ∧ v.shares.val ≠ v.assets.val ∧
          (v.assets.val = 0 ∨ U64.max < x.val * v.shares.val / v.assets.val) ⦄ := by
  unfold state.Vault.convert_assets_to_shares
  simp only [num_shares_eq, num_assets_eq, Std.bind_ok]
  split
  · rename_i h
    simp [h]
  · rename_i h
    have h' : v.shares.val ≠ v.assets.val := by scalar_tac
    step as ⟨r, hr⟩
    rcases r with y | e
    · simp only [core.result.Result.Insts.CoreOpsTry.branch, Std.bind_ok, WP.spec_ok]
      simp only at hr
      agrind
    · simp only [core.result.Result.Insts.CoreOpsTry.branch, Std.bind_ok, WP.spec_ok,
        from_residual_FromSame_Err]
      simp only at hr
      agrind

/-- **Spec theorem for `Vault::convert_shares_to_assets`** -/
@[step]
theorem convert_shares_to_assets.spec (v : state.Vault) (x : U64) :
    state.Vault.convert_shares_to_assets v x ⦃ (r : core.result.Result U64 errors.VaultError) =>
      match r with
      | .Ok y =>
        (v.shares.val = v.assets.val → y = x) ∧
        (v.shares.val ≠ v.assets.val →
          v.shares.val ≠ 0 ∧ y.val = x.val * v.assets.val / v.shares.val)
      | .Err e => e = errors.VaultError.MathOverflow ∧ v.shares.val ≠ v.assets.val ∧
          (v.shares.val = 0 ∨ U64.max < x.val * v.assets.val / v.shares.val) ⦄ := by
  unfold state.Vault.convert_shares_to_assets
  simp only [num_shares_eq, num_assets_eq, Std.bind_ok]
  split
  · rename_i h
    simp [h]
  · rename_i h
    have h' : v.shares.val ≠ v.assets.val := by scalar_tac
    step as ⟨r, hr⟩
    rcases r with y | e
    · simp only [core.result.Result.Insts.CoreOpsTry.branch, Std.bind_ok, WP.spec_ok]
      simp only at hr
      agrind
    · simp only [core.result.Result.Insts.CoreOpsTry.branch, Std.bind_ok, WP.spec_ok,
        from_residual_FromSame_Err]
      simp only at hr
      agrind


/-- **Spec theorem for `Vault::mint_shares`** -/
@[step]
theorem mint_shares.spec (v : state.Vault) (amt : U64) :
    state.Vault.mint_shares v amt ⦃ (r : core.result.Result Unit errors.VaultError)
        (v' : state.Vault) =>
      match r with
      | .Ok _ => 0 < amt.val ∧ v.shares.val + amt.val ≤ U64.max ∧
          v'.shares.val = v.shares.val + amt.val ∧ v'.assets = v.assets ∧
          v'.fee_bps = v.fee_bps ∧ v'.fee_amount = v.fee_amount
      | .Err e => v' = v ∧
          ((amt.val = 0 ∧ e = errors.VaultError.GuardFail) ∨
           (0 < amt.val ∧ U64.max < v.shares.val + amt.val ∧
             e = errors.VaultError.MathOverflow)) ⦄ := by
  unfold state.Vault.mint_shares
  split
  · rename_i h
    have h' : 0 < amt.val := by scalar_tac
    simp only [num_shares_eq, Std.bind_ok]
    step as ⟨o, ho⟩
    rcases o with _ | z
    · simp only at ho
      simp [core.result.Result.Insts.CoreOpsTry.branch]
      agrind
    · simp only at ho
      simp [core.result.Result.Insts.CoreOpsTry.branch]
      agrind
  · rename_i h
    have h' : amt.val = 0 := by scalar_tac
    simp [h']

/-- **Spec theorem for `Vault::add_token`** -/
@[step]
theorem add_token.spec (v : state.Vault) (amt : U64) :
    state.Vault.add_token v amt ⦃ (r : core.result.Result Unit errors.VaultError)
        (v' : state.Vault) =>
      match r with
      | .Ok _ => 0 < amt.val ∧ v.assets.val + amt.val ≤ U64.max ∧
          v'.assets.val = v.assets.val + amt.val ∧ v'.shares = v.shares ∧
          v'.fee_bps = v.fee_bps ∧ v'.fee_amount = v.fee_amount
      | .Err e => v' = v ∧
          ((amt.val = 0 ∧ e = errors.VaultError.GuardFail) ∨
           (0 < amt.val ∧ U64.max < v.assets.val + amt.val ∧
             e = errors.VaultError.MathOverflow)) ⦄ := by
  unfold state.Vault.add_token
  split
  · rename_i h
    have h' : 0 < amt.val := by scalar_tac
    simp only [num_assets_eq, Std.bind_ok]
    step as ⟨o, ho⟩
    rcases o with _ | z
    · simp only at ho
      simp [core.result.Result.Insts.CoreOpsTry.branch]
      agrind
    · simp only at ho
      simp [core.result.Result.Insts.CoreOpsTry.branch]
      agrind
  · rename_i h
    have h' : amt.val = 0 := by scalar_tac
    simp [h']

/-- **Spec theorem for `Vault::burn_shares`** -/
@[step]
theorem burn_shares.spec (v : state.Vault) (amt : U64) :
    state.Vault.burn_shares v amt ⦃ (r : core.result.Result Unit errors.VaultError)
        (v' : state.Vault) =>
      match r with
      | .Ok _ => amt.val ≤ v.shares.val ∧
          v'.shares.val + amt.val = v.shares.val ∧ v'.assets = v.assets ∧
          v'.fee_bps = v.fee_bps ∧ v'.fee_amount = v.fee_amount
      | .Err e => v' = v ∧ v.shares.val < amt.val ∧ e = errors.VaultError.MathOverflow ⦄ := by
  unfold state.Vault.burn_shares
  simp only [num_shares_eq, Std.bind_ok]
  step as ⟨o, ho⟩
  rcases o with _ | z
  · simp only at ho
    simp [core.result.Result.Insts.CoreOpsTry.branch]
    agrind
  · simp only at ho
    simp [core.result.Result.Insts.CoreOpsTry.branch]
    agrind

/-- **Spec theorem for `Vault::check_invariant`** -/
@[step]
theorem check_invariant.spec (v : state.Vault) :
    state.Vault.check_invariant v ⦃ (r : core.result.Result Unit errors.VaultError) =>
      match r with
      | .Ok _ => v.shares.val ≤ v.assets.val
      | .Err e => v.assets.val < v.shares.val ∧ e = errors.VaultError.GuardFail ⦄ := by
  unfold state.Vault.check_invariant state.Vault.is_solvent
  simp only [num_shares_eq, num_assets_eq, Std.bind_ok]
  split
  · rename_i h
    simp only [decide_eq_true_eq] at h
    simp only [WP.spec_ok]
    scalar_tac
  · rename_i h
    simp only [decide_eq_true_eq] at h
    simp only [WP.spec_ok, and_true]
    scalar_tac

/-- **Spec theorem for `Vault::del_token`** -/
@[step]
theorem del_token.spec (v : state.Vault) (amt : U64) :
    state.Vault.del_token v amt ⦃ (r : core.result.Result Unit errors.VaultError)
        (v' : state.Vault) =>
      match r with
      | .Ok _ => amt.val ≤ v.assets.val ∧ v.shares.val + amt.val ≤ v.assets.val ∧
          v'.assets.val + amt.val = v.assets.val ∧ v'.shares = v.shares ∧
          v'.fee_bps = v.fee_bps ∧ v'.fee_amount = v.fee_amount
      | .Err e =>
          (v' = v ∧ v.assets.val < amt.val ∧ e = errors.VaultError.MathOverflow) ∨
          (amt.val ≤ v.assets.val ∧ v.assets.val < v.shares.val + amt.val ∧
            e = errors.VaultError.GuardFail ∧
            v'.assets.val + amt.val = v.assets.val ∧ v'.shares = v.shares ∧
            v'.fee_bps = v.fee_bps ∧ v'.fee_amount = v.fee_amount) ⦄ := by
  unfold state.Vault.del_token
  simp only [num_assets_eq, Std.bind_ok]
  step as ⟨o, ho⟩
  rcases o with _ | z
  · simp only at ho
    simp [core.result.Result.Insts.CoreOpsTry.branch]
    agrind
  · simp only at ho
    simp only [core.option.Option.ok_or, core.result.Result.Insts.CoreOpsTry.branch,
      Std.bind_ok]
    step as ⟨r1, hr1⟩
    rcases r1 with _ | e
    · simp only at hr1
      simp only [Std.bind_ok, WP.spec_ok]
      refine ⟨ho.1, ?_, ?_, rfl, rfl, rfl⟩
      · scalar_tac
      · scalar_tac
    · simp only at hr1
      simp only [Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
      right
      refine ⟨ho.1, ?_, hr1.2, ?_, rfl, rfl, rfl⟩
      · scalar_tac
      · scalar_tac

/-- **Spec theorem for `Vault::fee_in_bps`** -/
@[step]
theorem fee_in_bps.spec (v : state.Vault) :
    state.Vault.fee_in_bps v ⦃ (r : core.result.Result utils.math.FeeBps errors.VaultError) =>
      match r with
      | .Ok f => f = v.fee_bps ∧ v.fee_bps.val ≤ 10000
      | .Err e => 10000 < v.fee_bps.val ∧ e = errors.VaultError.GuardFail ⦄ := by
  unfold state.Vault.fee_in_bps utils.math.FeeBps.Insts.CoreConvertTryFromU64VaultError.try_from
  have h1 : utils.math.ONE_IN_BPS.val = 10000 := by
    simp [utils.math.ONE_IN_BPS]
  split
  · rename_i h
    simp only [WP.spec_ok, true_and]
    scalar_tac
  · rename_i h
    simp only [WP.spec_ok, and_true]
    scalar_tac

theorem fee_le (amt bps : Nat) (h : bps ≤ 10000) : amt * bps / 10000 ≤ amt := by
  apply Nat.div_le_of_le_mul
  have := Nat.mul_le_mul_left amt h
  agrind

/-- **Spec theorem for `FeeBps::apply`** (with an in-range basis-point setting it always
succeeds): `fee = floor(amt * bps / 10000)`, `net = amt - fee`. -/
@[step]
theorem FeeBps.apply.spec (bps : utils.math.FeeBps) (amt : U64)
    (hbps : bps.val ≤ 10000) :
    utils.math.FeeBps.apply bps amt ⦃ (r : core.result.Result utils.math.GrossAmount
        errors.VaultError) =>
      match r with
      | .Ok g => g.fee.val = amt.val * bps.val / 10000 ∧
          g.net_amount.val + g.fee.val = amt.val
      | .Err _ => False ⦄ := by
  unfold utils.math.FeeBps.apply
  have h1 : utils.math.ONE_IN_BPS.val = 10000 := by
    simp [utils.math.ONE_IN_BPS]
  step as ⟨r, hr⟩
  rcases r with f | e
  · simp only at hr
    rw [h1] at hr
    have hf : f.val ≤ amt.val := by
      rw [hr.2]
      exact fee_le _ _ hbps
    simp only [core.result.Result.Insts.CoreOpsTry.branch, Std.bind_ok]
    step as ⟨o, ho⟩
    rcases o with _ | z
    · simp only at ho
      agrind
    · simp only at ho
      simp
      agrind
  · simp only at hr
    rw [h1] at hr
    have := fee_le amt.val bps.val hbps
    have : amt.val ≤ U64.max := by scalar_tac
    agrind

end CertoraVault.VaultLemmas

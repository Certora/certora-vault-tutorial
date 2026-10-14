import Aeneas
import CertoraVault
import CertoraVault.LusternaSchemas
import CertoraVault.VaultLemmas

/-! # Spec campaign: single-asset share vault (`certora_vault`)

Statement-only formal specification of the vault operations in `operations/` and the solvency
anchor `Vault::is_solvent`. The central property is solvency, `shares ≤ assets`.

Every translated operation is TOTAL in the Aeneas abort monad (all arithmetic is `checked_*`, so
overflow surfaces as a Rust `Err(MathOverflow)` value, never as `fail`), so each checked property
is an unconditional-totality WP triple. The produced value `r` is `(Rust outcome, post-vault)`.

On a Rust `.Err` the in-memory vault may be partially updated (e.g. `mint_shares` succeeded before
`add_token` overflowed); Solana reverts the account when the instruction fails. Solvency
preservation is therefore stated over the COMMITTED state (`committed`): the post-vault on `.Ok`,
the pre-vault on `.Err`. -/

open Aeneas Aeneas.Std Result
open certora_vault
open CertoraVault.VaultLemmas

set_option linter.dupNamespace false

namespace CertoraVault.Spec.Spec

/-- The solvency predicate `Vault::is_solvent`, as a plain proposition over the projected fields. -/
def Solvent (v : state.Vault) : Prop := v.shares.val ≤ v.assets.val

/-- Pure model of `Vault::convert_assets_to_shares`: identity when `shares = assets`, otherwise
`floor(x * shares / assets)`. (Division by zero is a Rust `Err`, never reached on an `.Ok`.) -/
def assetsToShares (v : state.Vault) (x : Nat) : Nat :=
  if v.shares.val = v.assets.val then x else x * v.shares.val / v.assets.val

/-- Pure model of `Vault::convert_shares_to_assets`: identity when `shares = assets`, otherwise
`floor(y * assets / shares)`. -/
def sharesToAssets (v : state.Vault) (y : Nat) : Nat :=
  if v.shares.val = v.assets.val then y else y * v.assets.val / v.shares.val

/-- The fee of `FeeBps::apply`: `floor(amt * bps / 10000)`. -/
def feeOf (bps amt : Nat) : Nat := amt * bps / 10000

/-- The vault state a transaction COMMITS after an operation returning `r`: the post-vault on a Rust
`.Ok`, the pre-vault on a Rust `.Err` (the runtime reverts the account). Panics are excluded by the
totality of the triple. -/
def committed {T E : Type} (v : state.Vault) (r : core.result.Result T E × state.Vault) :
    state.Vault :=
  match r.1 with
  | .Ok _ => r.2
  | .Err _ => v

/-! ## Anchor bridge: `is_solvent` -/

/-- `Vault::is_solvent` returns `true` exactly when `shares ≤ assets` (ties `Solvent` to the real
getter). -/
@[lusterna]
theorem is_solvent_spec (v : state.Vault) :
    state.Vault.is_solvent v ⦃ b => (b = true ↔ Solvent v) ⦄ := by
  unfold state.Vault.is_solvent
  simp only [num_shares_eq, num_assets_eq, Std.bind_ok, WP.spec_ok, decide_eq_true_eq, Solvent]
  constructor
  · intro h; scalar_tac
  · intro h; scalar_tac

/-- Base case of the solvency invariant: the default / new vault (all fields zero) is solvent. -/
@[lusterna_lemma "base case of the solvency invariant: the Rust crate has no translated vault constructor, so this is a pure fact about the zero-initialised state"]
theorem default_vault_solvent :
    Solvent { shares := 0#u64, assets := 0#u64, fee_bps := 0#u64, fee_amount := 0#u64 } := by
  simp [Solvent]

/-! ## `vault_deposit_assets` -/

/-- On a solvent vault, a successful deposit of `t` mints `convert_assets_to_shares(t) > 0` shares,
adds exactly `t` assets, leaves the fee fields alone, and leaves the vault solvent. -/
@[lusterna]
theorem vault_deposit_assets_ok (v : state.Vault) (t : Std.U64) (hsolv : Solvent v) :
    operations.vault_operations.vault_deposit_assets v t ⦃ r =>
      match r.1 with
      | .Ok e =>
        e.shares_to_user.val = assetsToShares v t.val ∧
        e.assets_to_vault.val = t.val ∧
        0 < e.shares_to_user.val ∧
        e.shares_to_burn.val = 0 ∧ e.assets_to_user.val = 0 ∧ e.assets_to_fee.val = 0 ∧
        r.2.shares.val = v.shares.val + e.shares_to_user.val ∧
        r.2.assets.val = v.assets.val + t.val ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount ∧
        Solvent r.2
      | .Err _ => True ⦄ := by
  have hsolv' := hsolv
  unfold Solvent at hsolv'
  unfold operations.vault_operations.vault_deposit_assets
  step as ⟨r, hr⟩
  rcases r with y | e
  · simp only at hr
    have hy : y.val = assetsToShares v t.val := by
      simp only [assetsToShares]
      by_cases h : v.shares.val = v.assets.val
      · simp [h, hr.1 h]
      · simp only [h, if_false]; exact (hr.2 h).2
    have hyt : y.val ≤ t.val := by
      rw [hy]; simp only [assetsToShares]
      split
      · exact le_refl _
      · exact mul_div_le_self_of_le _ _ _ hsolv'
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, v1, hr1⟩
    rcases r1 with _ | e1
    · simp only at hr1
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, v2, hr2⟩
      rcases r2 with _ | e2
      · simp only at hr2
        simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
          zeroEffect_shares_to_burn, zeroEffect_assets_to_user, zeroEffect_assets_to_fee,
          true_and, Solvent]
        obtain ⟨h1, _, h3, h4, h5, h6⟩ := hr1
        obtain ⟨_, _, k3, k4, k5, k6⟩ := hr2
        refine ⟨hy, h1, ?_, ?_, ?_, ?_, ?_⟩
        · rw [k4, h3]
        · rw [k3, h4]
        · rw [k5, h5]
        · rw [k6, h6]
        · rw [k4, k3, h3, h4]; scalar_tac
      · simp
    · simp
  · simp

/-- Whenever the share conversion is defined (`shares = assets`, or `assets > 0`), a deposit whose
converted share amount is zero (e.g. `t = 0`, or `t * shares < assets`) fails with `GuardFail`
before writing anything — solvency is not required. -/
@[lusterna]
theorem vault_deposit_assets_zero_shares (v : state.Vault) (t : Std.U64)
    (hdef : v.shares.val = v.assets.val ∨ 0 < v.assets.val)
    (hzero : assetsToShares v t.val = 0) :
    operations.vault_operations.vault_deposit_assets v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by
  unfold operations.vault_operations.vault_deposit_assets
  step as ⟨r, hr⟩
  rcases r with y | e
  · simp only at hr
    have hy : y.val = 0 := by
      rw [← hzero]; simp only [assetsToShares]
      by_cases h : v.shares.val = v.assets.val
      · simp [h, hr.1 h]
      · simp only [h, if_false]; exact (hr.2 h).2
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, v1, hr1⟩
    rcases r1 with _ | e1
    · simp only at hr1
      scalar_tac
    · simp only at hr1
      simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
      obtain ⟨hv, h | h⟩ := hr1
      · simp [h.2, hv]
      · scalar_tac
  · simp only at hr
    obtain ⟨_, hne, h | h⟩ := hr
    · scalar_tac
    · simp only [assetsToShares, hne, if_false] at hzero
      scalar_tac

/-- Solvency preservation (committed state) for `vault_deposit_assets`. -/
@[lusterna]
theorem vault_deposit_assets_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_deposit_assets v t ⦃ r => Solvent (committed v r) ⦄ := by
  apply WP.spec_mono (vault_deposit_assets_ok v t hsolv)
  rintro ⟨r, v'⟩ h
  rcases r with e | e
  · simp only [committed]
    simp only at h
    exact h.2.2.2.2.2.2.2.2.2.2
  · simpa [committed] using hsolv

/-! ## `vault_deposit_assets_with_fee` -/

/-- A successful fee deposit has `fee_bps ≤ 10000`, charges `fee = floor(t * fee_bps / 10000)`,
deposits `net = t - fee`, mints `convert_assets_to_shares(net) > 0` shares, and does not touch the
vault's `fee_amount` / `fee_bps`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_ok (v : state.Vault) (t : Std.U64) :
    operations.vault_fee_operations.vault_deposit_assets_with_fee v t ⦃ r =>
      match r.1 with
      | .Ok e =>
        v.fee_bps.val ≤ 10000 ∧
        e.assets_to_fee.val = feeOf v.fee_bps.val t.val ∧
        e.assets_to_vault.val + e.assets_to_fee.val = t.val ∧
        e.shares_to_user.val = assetsToShares v e.assets_to_vault.val ∧
        0 < e.shares_to_user.val ∧
        e.shares_to_burn.val = 0 ∧ e.assets_to_user.val = 0 ∧
        r.2.shares.val = v.shares.val + e.shares_to_user.val ∧
        r.2.assets.val = v.assets.val + e.assets_to_vault.val ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount
      | .Err _ => True ⦄ := by
  unfold operations.vault_fee_operations.vault_deposit_assets_with_fee
  step as ⟨r, hr⟩
  rcases r with f | e
  · simp only at hr
    obtain ⟨rfl, hbps⟩ := hr
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨g, hg⟩
    rcases g with g | e
    · simp only at hg
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, hr2⟩
      rcases r2 with y | e
      · simp only at hr2
        have hy : y.val = assetsToShares v g.net_amount.val := a2s_val hr2
        simp only [branch_Ok, Std.bind_ok]
        step as ⟨r3, v1, hr3⟩
        rcases r3 with _ | e
        · simp only at hr3
          simp only [branch_Ok, Std.bind_ok]
          step as ⟨r4, v2, hr4⟩
          rcases r4 with _ | e
          · simp only at hr4
            simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
              zeroEffect_shares_to_burn, zeroEffect_assets_to_user, true_and]
            obtain ⟨h1, _, h3, h4, h5, h6⟩ := hr3
            obtain ⟨_, _, k3, k4, k5, k6⟩ := hr4
            refine ⟨hbps, hg.1, hg.2, hy, h1, ?_, ?_, ?_, ?_⟩
            · rw [k4, h3]
            · rw [k3, h4]
            · rw [k5, h5]
            · rw [k6, h6]
          · simp
        · simp
      · simp
    · simp at hg
  · simp

/-- A fee basis-point setting above `10000` makes the fee deposit fail with `GuardFail` before
writing anything. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_bad_bps (v : state.Vault) (t : Std.U64)
    (hbps : 10000 < v.fee_bps.val) :
    operations.vault_fee_operations.vault_deposit_assets_with_fee v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by
  unfold operations.vault_fee_operations.vault_deposit_assets_with_fee
  step as ⟨r, hr⟩
  rcases r with f | e
  · simp only at hr
    scalar_tac
  · simp only at hr
    simp [hr.2]

/-- Solvency preservation (committed state) for `vault_deposit_assets_with_fee`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_fee_operations.vault_deposit_assets_with_fee v t ⦃ r =>
      Solvent (committed v r) ⦄ := by
  unfold Solvent at hsolv
  apply WP.spec_mono (vault_deposit_assets_with_fee_ok v t)
  rintro ⟨r, v'⟩ h
  rcases r with e | e
  · simp only [committed, Solvent]
    simp only at h
    obtain ⟨_, _, _, hy, _, _, _, hs, ha, _, _⟩ := h
    have : e.shares_to_user.val ≤ e.assets_to_vault.val := by
      rw [hy]; simp only [assetsToShares]
      split
      · exact le_refl _
      · exact mul_div_le_self_of_le _ _ _ hsolv
    rw [hs, ha]; scalar_tac
  · simpa [committed, Solvent] using hsolv

/-! ## `vault_deposit_assets_exact` -/

/-- On a solvent vault, a successful exact deposit mints `y = convert_assets_to_shares(t) > 0`
shares, takes `convert_shares_to_assets(y)` assets with `y ≤ assets_to_vault ≤ t`, and leaves the
vault solvent. -/
@[lusterna]
theorem vault_deposit_assets_exact_ok (v : state.Vault) (t : Std.U64) (hsolv : Solvent v) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_exact v t ⦃ r =>
      match r.1 with
      | .Ok e =>
        e.shares_to_user.val = assetsToShares v t.val ∧
        e.assets_to_vault.val = sharesToAssets v e.shares_to_user.val ∧
        0 < e.shares_to_user.val ∧
        e.shares_to_user.val ≤ e.assets_to_vault.val ∧
        e.assets_to_vault.val ≤ t.val ∧
        e.shares_to_burn.val = 0 ∧ e.assets_to_user.val = 0 ∧ e.assets_to_fee.val = 0 ∧
        r.2.shares.val = v.shares.val + e.shares_to_user.val ∧
        r.2.assets.val = v.assets.val + e.assets_to_vault.val ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount ∧
        Solvent r.2
      | .Err _ => True ⦄ := by
  have hsolv' := hsolv
  unfold Solvent at hsolv'
  unfold operations.vault_exact_deposit_operations.vault_deposit_assets_exact
  step as ⟨r, hr⟩
  rcases r with y | e
  · simp only at hr
    have hy : y.val = assetsToShares v t.val := a2s_val hr
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, hr1⟩
    rcases r1 with z | e
    · simp only at hr1
      have hz : z.val = sharesToAssets v y.val := s2a_val hr1
      have hyz : y.val ≤ z.val := by
        by_cases h : v.shares.val = v.assets.val
        · rw [hr1.1 h]
        · obtain ⟨hs, hz'⟩ := hr1.2 h
          rw [hz']; exact le_mul_div_of_le _ _ _ (by scalar_tac) hsolv'
      have hzt : z.val ≤ t.val := by
        by_cases h : v.shares.val = v.assets.val
        · rw [hr1.1 h, hr.1 h]
        · obtain ⟨_, hz'⟩ := hr1.2 h
          obtain ⟨_, hy'⟩ := hr.2 h
          rw [hz', hy']; exact round_trip_le _ _ _
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, v1, hr2⟩
      rcases r2 with _ | e
      · simp only at hr2
        simp only [branch_Ok, Std.bind_ok]
        step as ⟨r3, v2, hr3⟩
        rcases r3 with _ | e
        · simp only at hr3
          simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
            zeroEffect_shares_to_burn, zeroEffect_assets_to_user, zeroEffect_assets_to_fee,
            true_and, Solvent]
          obtain ⟨h1, _, h3, h4, h5, h6⟩ := hr2
          obtain ⟨_, _, k3, k4, k5, k6⟩ := hr3
          refine ⟨hy, hz, h1, hyz, hzt, ?_, ?_, ?_, ?_, ?_⟩
          · rw [k4, h3]
          · rw [k3, h4]
          · rw [k5, h5]
          · rw [k6, h6]
          · rw [k4, k3, h3, h4]; scalar_tac
        · simp
      · simp
    · simp
  · simp

/-- Solvency preservation (committed state) for `vault_deposit_assets_exact`. -/
@[lusterna]
theorem vault_deposit_assets_exact_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_exact v t ⦃ r =>
      Solvent (committed v r) ⦄ := by
  apply WP.spec_mono (vault_deposit_assets_exact_ok v t hsolv)
  rintro ⟨r, v'⟩ h
  rcases r with e | e
  · simp only [committed]
    simp only at h
    exact h.2.2.2.2.2.2.2.2.2.2.2.2
  · simpa [committed] using hsolv

/-! ## `vault_deposit_assets_with_fee_exact` -/

/-- `vault_deposit_assets_with_fee_exact` never modifies the vault, whatever its outcome. On `.Ok`
it reports `shares_to_user = convert_assets_to_shares(t - fee(t))`,
`assets_to_vault = convert_shares_to_assets(shares_to_user)` and
`assets_to_fee = fee(assets_to_vault)`, with `fee_bps ≤ 10000`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_exact_spec (v : state.Vault) (t : Std.U64) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact v t ⦃ r =>
      r.2 = v ∧
      match r.1 with
      | .Ok e =>
        v.fee_bps.val ≤ 10000 ∧
        e.shares_to_user.val = assetsToShares v (t.val - feeOf v.fee_bps.val t.val) ∧
        e.assets_to_vault.val = sharesToAssets v e.shares_to_user.val ∧
        e.assets_to_fee.val = feeOf v.fee_bps.val e.assets_to_vault.val ∧
        e.shares_to_burn.val = 0 ∧ e.assets_to_user.val = 0
      | .Err _ => True ⦄ := by
  unfold operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact
  step as ⟨r, hr⟩
  rcases r with f | e
  · simp only at hr
    obtain ⟨rfl, hbps⟩ := hr
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨g, hg⟩
    rcases g with g | e
    · simp only at hg
      have hnet : g.net_amount.val = t.val - feeOf v.fee_bps.val t.val := by
        simp only [feeOf]; scalar_tac
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, hr2⟩
      rcases r2 with y | e
      · simp only at hr2
        have hy : y.val = assetsToShares v g.net_amount.val := a2s_val hr2
        simp only [branch_Ok, Std.bind_ok]
        step as ⟨r3, hr3⟩
        rcases r3 with z | e
        · simp only at hr3
          have hz : z.val = sharesToAssets v y.val := s2a_val hr3
          simp only [branch_Ok, Std.bind_ok]
          step as ⟨g2, hg2⟩
          rcases g2 with g2 | e
          · simp only at hg2
            simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
              zeroEffect_shares_to_burn, zeroEffect_assets_to_user, true_and, and_true]
            refine ⟨hbps, ?_, hz, hg2.1⟩
            rw [hy, hnet]
          · simp at hg2
        · simp
      · simp
    · simp at hg
  · simp

/-- A fee basis-point setting above `10000` makes `vault_deposit_assets_with_fee_exact` fail with
`GuardFail`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_exact_bad_bps (v : state.Vault) (t : Std.U64)
    (hbps : 10000 < v.fee_bps.val) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by
  unfold operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact
  step as ⟨r, hr⟩
  rcases r with f | e
  · simp only at hr
    scalar_tac
  · simp only at hr
    simp [hr.2]

/-- Solvency preservation (committed state) for `vault_deposit_assets_with_fee_exact`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_exact_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact v t ⦃ r =>
      Solvent (committed v r) ⦄ := by
  apply WP.spec_mono (vault_deposit_assets_with_fee_exact_spec v t)
  rintro ⟨r, v'⟩ ⟨h, _⟩
  simp only at h
  rcases r with e | e
  · simpa [committed, h] using hsolv
  · simpa [committed] using hsolv

/-! ## `vault_redeem_shares` -/

/-- A successful redeem of `n` shares burns exactly `n` shares, pays out
`convert_shares_to_assets(n)` assets, and leaves the vault solvent — from ANY starting state
(enforced by `del_token`'s solvency check). -/
@[lusterna]
theorem vault_redeem_shares_ok (v : state.Vault) (n : Std.U64) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      match r.1 with
      | .Ok e =>
        e.shares_to_burn = n ∧
        e.assets_to_user.val = sharesToAssets v n.val ∧
        e.shares_to_user.val = 0 ∧ e.assets_to_vault.val = 0 ∧ e.assets_to_fee.val = 0 ∧
        r.2.shares.val + n.val = v.shares.val ∧
        r.2.assets.val + e.assets_to_user.val = v.assets.val ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount ∧
        Solvent r.2
      | .Err _ => True ⦄ := by
  unfold operations.vault_operations.vault_redeem_shares
  step as ⟨r, hr⟩
  rcases r with z | e
  · simp only at hr
    have hz : z.val = sharesToAssets v n.val := s2a_val hr
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, v1, hr1⟩
    rcases r1 with _ | e
    · simp only at hr1
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, v2, hr2⟩
      rcases r2 with _ | e
      · simp only at hr2
        simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
          zeroEffect_shares_to_user, zeroEffect_assets_to_vault, zeroEffect_assets_to_fee,
          true_and, Solvent]
        obtain ⟨_, h1, h3, h4, h5⟩ := hr1
        obtain ⟨_, k2, k3, k4, k5, k6⟩ := hr2
        refine ⟨hz, ?_, ?_, ?_, ?_, ?_⟩
        · rw [k4, h1]
        · rw [← h3, ← k3]
        · rw [k5, h4]
        · rw [k6, h5]
        · rw [k4]; scalar_tac
      · simp
    · simp
  · simp

/-- Liveness of redeem: on a solvent vault whose conversion is defined (`shares = assets`, or
`shares > 0`), redeeming `n ≤ shares` always succeeds — no overflow, and the solvency check never
fires. -/
@[lusterna]
theorem vault_redeem_shares_succeeds (v : state.Vault) (n : Std.U64) (hsolv : Solvent v)
    (hconv : v.shares.val = v.assets.val ∨ 0 < v.shares.val) (hn : n.val ≤ v.shares.val) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      match r.1 with
      | .Ok e =>
        e.shares_to_burn = n ∧
        e.assets_to_user.val = sharesToAssets v n.val ∧
        r.2.shares.val + n.val = v.shares.val ∧
        r.2.assets.val + e.assets_to_user.val = v.assets.val
      | .Err _ => False ⦄ := by
  unfold Solvent at hsolv
  have hle : sharesToAssets v n.val ≤ v.assets.val := by
    simp only [sharesToAssets]
    split
    · scalar_tac
    · exact redeem_le_assets _ _ _ hn
  have hsol : sharesToAssets v n.val + v.shares.val ≤ v.assets.val + n.val := by
    simp only [sharesToAssets]
    split
    · scalar_tac
    · exact redeem_solvent _ _ _ hn hsolv
  unfold operations.vault_operations.vault_redeem_shares
  step as ⟨r, hr⟩
  rcases r with z | e
  · simp only at hr
    have hz : z.val = sharesToAssets v n.val := s2a_val hr
    simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, v1, hr1⟩
    rcases r1 with _ | e
    · simp only at hr1
      simp only [branch_Ok, Std.bind_ok]
      step as ⟨r2, v2, hr2⟩
      rcases r2 with _ | e
      · simp only at hr2
        simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok, true_and]
        obtain ⟨_, h1, h3, _, _⟩ := hr1
        obtain ⟨_, _, k3, k4, _, _⟩ := hr2
        refine ⟨hz, ?_, ?_⟩
        · rw [k4, h1]
        · rw [← h3, ← k3]
      · simp only at hr2
        obtain ⟨_, h1, h3, _, _⟩ := hr1
        rcases hr2 with k | k
        · rw [h3] at k; scalar_tac
        · rw [h3] at k; scalar_tac
    · simp only at hr1
      scalar_tac
  · simp only at hr
    obtain ⟨_, hne, h | h⟩ := hr
    · scalar_tac
    · simp only [sharesToAssets, hne, if_false] at hle
      scalar_tac

/-- Redeeming more shares than exist fails with `MathOverflow` (from the conversion or the
`burn_shares` underflow) before any write. -/
@[lusterna]
theorem vault_redeem_shares_too_many (v : state.Vault) (n : Std.U64)
    (hn : v.shares.val < n.val) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by
  unfold operations.vault_operations.vault_redeem_shares
  step as ⟨r, hr⟩
  rcases r with z | e
  · simp only [branch_Ok, Std.bind_ok]
    step as ⟨r1, v1, hr1⟩
    rcases r1 with _ | e
    · simp only at hr1
      scalar_tac
    · simp only at hr1
      simp [hr1.1, hr1.2.2]
  · simp only at hr
    simp [hr.1]

/-- Redeeming from a vault with no shares but positive assets divides by zero in
`convert_shares_to_assets` and fails with `MathOverflow` before any write. -/
@[lusterna]
theorem vault_redeem_shares_zero_shares (v : state.Vault) (n : Std.U64)
    (hs : v.shares.val = 0) (ha : 0 < v.assets.val) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by
  unfold operations.vault_operations.vault_redeem_shares
  step as ⟨r, hr⟩
  rcases r with z | e
  · simp only at hr
    have := (hr.2 (by scalar_tac)).1
    scalar_tac
  · simp only at hr
    simp [hr.1]

/-- Solvency preservation (committed state) for `vault_redeem_shares`. -/
@[lusterna]
theorem vault_redeem_shares_preserves_solvent (v : state.Vault) (n : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r => Solvent (committed v r) ⦄ := by
  apply WP.spec_mono (vault_redeem_shares_ok v n)
  rintro ⟨r, v'⟩ h
  rcases r with e | e
  · simp only [committed]
    simp only at h
    exact h.2.2.2.2.2.2.2.2.2
  · simpa [committed] using hsolv

/-! ## `vault_update_reward` -/

/-- An observed balance below the recorded assets is rejected with `MathOverflow` and the vault is
unchanged. -/
@[lusterna]
theorem vault_update_reward_below (v : state.Vault) (b : Std.U64) (hb : b.val < v.assets.val) :
    operations.vault_operations.vault_update_reward v b ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by
  unfold operations.vault_operations.vault_update_reward
  simp only [num_assets_eq, Std.bind_ok]
  step as ⟨o, ho⟩
  rcases o with _ | z
  · simp
  · simp only at ho
    scalar_tac

/-- An observed balance at least the recorded assets always succeeds with the default effect,
sets `assets := b`, and leaves shares and fee fields unchanged. -/
@[lusterna]
theorem vault_update_reward_ok (v : state.Vault) (b : Std.U64) (hb : v.assets.val ≤ b.val) :
    operations.vault_operations.vault_update_reward v b ⦃ r =>
      match r.1 with
      | .Ok e =>
        e.shares_to_burn.val = 0 ∧ e.shares_to_user.val = 0 ∧ e.assets_to_vault.val = 0 ∧ e.assets_to_user.val = 0 ∧ e.assets_to_fee.val = 0 ∧
        r.2.assets.val = b.val ∧
        r.2.shares = v.shares ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount
      | .Err _ => False ⦄ := by
  unfold operations.vault_operations.vault_update_reward
  simp only [num_assets_eq, Std.bind_ok]
  step as ⟨o, ho⟩
  rcases o with _ | z
  · simp only at ho
    scalar_tac
  · simp only at ho
    simp only [core.option.Option.ok_or, branch_Ok, Std.bind_ok]
    split
    · step as ⟨r1, v1, hr1⟩
      rcases r1 with _ | e
      · simp only at hr1
        simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
          zeroEffect_shares_to_burn, zeroEffect_shares_to_user, zeroEffect_assets_to_vault,
          zeroEffect_assets_to_user, zeroEffect_assets_to_fee, true_and]
        obtain ⟨_, _, h1, h2, h3, h4⟩ := hr1
        exact ⟨by scalar_tac, h2, h3, h4⟩
      · simp only at hr1
        obtain ⟨_, h | h⟩ := hr1
        · scalar_tac
        · scalar_tac
    · simp [zeroEffect]
      scalar_tac

/-- `vault_update_reward` preserves solvency in every outcome (it never decreases assets and never
touches shares; on `.Err` the vault is untouched). -/
@[lusterna]
theorem vault_update_reward_preserves_solvent (v : state.Vault) (b : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_update_reward v b ⦃ r => Solvent r.2 ⦄ := by
  unfold operations.vault_operations.vault_update_reward
  simp only [num_assets_eq, Std.bind_ok]
  step as ⟨o, ho⟩
  rcases o with _ | z
  · simpa using hsolv
  · simp only at ho
    simp only [core.option.Option.ok_or, branch_Ok, Std.bind_ok]
    split
    · step as ⟨r1, v1, hr1⟩
      rcases r1 with _ | e
      · simp only at hr1
        simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok, Solvent]
        obtain ⟨_, _, h1, h2, _⟩ := hr1
        unfold Solvent at hsolv
        rw [h2]; scalar_tac
      · simp only at hr1
        simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
        rw [hr1.1]; exact hsolv
    · simpa using hsolv

/-! ## `vault_process_slash` -/

/-- `vault_process_slash v a` succeeds iff `a ≤ assets` and `shares ≤ assets - a`; on success it
removes exactly `a` assets, leaves shares and fees alone, reports `assets_to_user = a`, and the
vault is solvent. -/
@[lusterna]
theorem vault_process_slash_spec (v : state.Vault) (a : Std.U64) :
    operations.vault_operations.vault_process_slash v a ⦃ r =>
      match r.1 with
      | .Ok e =>
        v.shares.val + a.val ≤ v.assets.val ∧
        r.2.assets.val + a.val = v.assets.val ∧
        r.2.shares = v.shares ∧
        r.2.fee_bps = v.fee_bps ∧
        r.2.fee_amount = v.fee_amount ∧
        e.assets_to_user = a ∧
        e.shares_to_burn.val = 0 ∧ e.shares_to_user.val = 0 ∧ e.assets_to_vault.val = 0 ∧ e.assets_to_fee.val = 0 ∧
        Solvent r.2
      | .Err _ => ¬ (v.shares.val + a.val ≤ v.assets.val) ⦄ := by
  unfold operations.vault_operations.vault_process_slash
  step as ⟨r, v1, hr⟩
  rcases r with _ | e
  · simp only at hr
    simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok,
      zeroEffect_shares_to_burn, zeroEffect_shares_to_user, zeroEffect_assets_to_vault,
      zeroEffect_assets_to_fee, true_and, Solvent]
    obtain ⟨_, h0, h1, h2, h3, h4⟩ := hr
    refine ⟨h0, h1, h2, h3, h4, ?_⟩
    rw [h2]; scalar_tac
  · simp only at hr
    simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
    rcases hr with h | h
    · scalar_tac
    · scalar_tac

/-- A slash larger than the recorded assets fails with `MathOverflow` before any write. -/
@[lusterna]
theorem vault_process_slash_exceeds_assets (v : state.Vault) (a : Std.U64)
    (ha : v.assets.val < a.val) :
    operations.vault_operations.vault_process_slash v a ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by
  unfold operations.vault_operations.vault_process_slash
  step as ⟨r, v1, hr⟩
  rcases r with _ | e
  · simp only at hr
    scalar_tac
  · simp only at hr
    simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
    rcases hr with h | h
    · simp [h.1, h.2.2]
    · scalar_tac

/-- A slash within the assets that would leave the vault insolvent fails with `GuardFail`; the
function has already written the reduced assets in memory (reverted by the runtime). -/
@[lusterna]
theorem vault_process_slash_insolvent (v : state.Vault) (a : Std.U64)
    (ha : a.val ≤ v.assets.val) (hins : v.assets.val < v.shares.val + a.val) :
    operations.vault_operations.vault_process_slash v a ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧
      r.2.assets.val + a.val = v.assets.val ∧
      r.2.shares = v.shares ⦄ := by
  unfold operations.vault_operations.vault_process_slash
  step as ⟨r, v1, hr⟩
  rcases r with _ | e
  · simp only at hr
    scalar_tac
  · simp only at hr
    simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok]
    rcases hr with h | h
    · scalar_tac
    · obtain ⟨_, _, h1, h2, h3, _⟩ := h
      simp [h1, h2, h3]

/-- Solvency preservation (committed state) for `vault_process_slash`. -/
@[lusterna]
theorem vault_process_slash_preserves_solvent (v : state.Vault) (a : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_process_slash v a ⦃ r => Solvent (committed v r) ⦄ := by
  unfold operations.vault_operations.vault_process_slash
  step as ⟨r, v1, hr⟩
  rcases r with _ | e
  · simp only at hr
    simp only [branch_Ok, Std.bind_ok, VaultEffect_default_eq, WP.spec_ok, committed, Solvent]
    obtain ⟨_, h0, h1, h2, _⟩ := hr
    rw [h2]; scalar_tac
  · simp only [branch_Err, Std.bind_ok, from_residual_FromSame_Err, WP.spec_ok, committed]
    exact hsolv

/-! ## `vault_collect_fee` -/

/-- `vault_collect_fee` always succeeds, pays out the accrued `fee_amount`, zeroes it, and leaves
shares, assets and `fee_bps` unchanged. -/
@[lusterna]
theorem vault_collect_fee_spec (v : state.Vault) :
    operations.vault_fee_operations.vault_collect_fee v ⦃ r =>
      r.2.fee_amount.val = 0 ∧
      r.2.shares = v.shares ∧
      r.2.assets = v.assets ∧
      r.2.fee_bps = v.fee_bps ∧
      match r.1 with
      | .Ok e => e.assets_to_user = v.fee_amount ∧ e.shares_to_burn.val = 0 ∧ e.shares_to_user.val = 0 ∧ e.assets_to_vault.val = 0 ∧ e.assets_to_fee.val = 0
      | .Err _ => False ⦄ := by
  unfold operations.vault_fee_operations.vault_collect_fee
  simp [zeroEffect]

/-- `vault_collect_fee` preserves solvency. -/
@[lusterna]
theorem vault_collect_fee_preserves_solvent (v : state.Vault) (hsolv : Solvent v) :
    operations.vault_fee_operations.vault_collect_fee v ⦃ r => Solvent r.2 ⦄ := by
  unfold operations.vault_fee_operations.vault_collect_fee
  simpa [Solvent] using hsolv

/-! ## Conversions (helpers, not declared targets) -/

/-- On a solvent vault `convert_assets_to_shares x` always succeeds, matches `assetsToShares`, and
never exceeds `x`. -/
@[lusterna_lemma "conversion helper: convert_assets_to_shares is not a declared campaign target; supports the deposit properties"]
theorem convert_assets_to_shares_spec (v : state.Vault) (x : Std.U64) (hsolv : Solvent v) :
    state.Vault.convert_assets_to_shares v x ⦃ r =>
      match r with
      | .Ok y => y.val = assetsToShares v x.val ∧ y.val ≤ x.val
      | .Err _ => False ⦄ := by
  unfold Solvent at hsolv
  step as ⟨r, hr⟩
  rcases r with y | e
  · simp only at hr
    simp only [assetsToShares]
    by_cases h : v.shares.val = v.assets.val
    · simp [h, hr.1 h]
    · simp only [h, if_false]
      rw [(hr.2 h).2]
      exact ⟨rfl, mul_div_le_self_of_le _ _ _ hsolv⟩
  · simp only at hr
    obtain ⟨_, hne, h | h⟩ := hr
    · scalar_tac
    · have := mul_div_le_self_of_le x.val v.shares.val v.assets.val hsolv
      scalar_tac

/-- On a solvent vault with `shares ≠ assets`, a successful `convert_shares_to_assets y` returns
`floor(y * assets / shares) ≥ y`. -/
@[lusterna_lemma "conversion helper: convert_shares_to_assets is not a declared campaign target; supports the redeem / exact-deposit properties"]
theorem convert_shares_to_assets_spec (v : state.Vault) (y : Std.U64) (hsolv : Solvent v)
    (hne : v.shares.val ≠ v.assets.val) :
    state.Vault.convert_shares_to_assets v y ⦃ r =>
      match r with
      | .Ok z => z.val = y.val * v.assets.val / v.shares.val ∧ y.val ≤ z.val
      | .Err _ => True ⦄ := by
  unfold Solvent at hsolv
  step as ⟨r, hr⟩
  rcases r with z | e
  · simp only at hr ⊢
    obtain ⟨hs, hz⟩ := hr.2 hne
    rw [hz]
    exact ⟨rfl, le_mul_div_of_le _ _ _ (by scalar_tac) hsolv⟩
  · simp

end CertoraVault.Spec.Spec

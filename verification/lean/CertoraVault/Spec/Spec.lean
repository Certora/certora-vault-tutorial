import Aeneas
import CertoraVault
import CertoraVault.LusternaSchemas

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
    state.Vault.is_solvent v ⦃ b => (b = true ↔ Solvent v) ⦄ := by sorry

/-- Base case of the solvency invariant: the default / new vault (all fields zero) is solvent. -/
@[lusterna_lemma "base case of the solvency invariant: the Rust crate has no translated vault constructor, so this is a pure fact about the zero-initialised state"]
theorem default_vault_solvent :
    Solvent { shares := 0#u64, assets := 0#u64, fee_bps := 0#u64, fee_amount := 0#u64 } := by sorry

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
      | .Err _ => True ⦄ := by sorry

/-- Whenever the share conversion is defined (`shares = assets`, or `assets > 0`), a deposit whose
converted share amount is zero (e.g. `t = 0`, or `t * shares < assets`) fails with `GuardFail`
before writing anything — solvency is not required. -/
@[lusterna]
theorem vault_deposit_assets_zero_shares (v : state.Vault) (t : Std.U64)
    (hdef : v.shares.val = v.assets.val ∨ 0 < v.assets.val)
    (hzero : assetsToShares v t.val = 0) :
    operations.vault_operations.vault_deposit_assets v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_deposit_assets`. -/
@[lusterna]
theorem vault_deposit_assets_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_deposit_assets v t ⦃ r => Solvent (committed v r) ⦄ := by
  sorry

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
      | .Err _ => True ⦄ := by sorry

/-- A fee basis-point setting above `10000` makes the fee deposit fail with `GuardFail` before
writing anything. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_bad_bps (v : state.Vault) (t : Std.U64)
    (hbps : 10000 < v.fee_bps.val) :
    operations.vault_fee_operations.vault_deposit_assets_with_fee v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_deposit_assets_with_fee`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_fee_operations.vault_deposit_assets_with_fee v t ⦃ r =>
      Solvent (committed v r) ⦄ := by sorry

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
      | .Err _ => True ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_deposit_assets_exact`. -/
@[lusterna]
theorem vault_deposit_assets_exact_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_exact v t ⦃ r =>
      Solvent (committed v r) ⦄ := by sorry

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
      | .Err _ => True ⦄ := by sorry

/-- A fee basis-point setting above `10000` makes `vault_deposit_assets_with_fee_exact` fail with
`GuardFail`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_exact_bad_bps (v : state.Vault) (t : Std.U64)
    (hbps : 10000 < v.fee_bps.val) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact v t ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧ r.2 = v ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_deposit_assets_with_fee_exact`. -/
@[lusterna]
theorem vault_deposit_assets_with_fee_exact_preserves_solvent (v : state.Vault) (t : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact v t ⦃ r =>
      Solvent (committed v r) ⦄ := by sorry

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
      | .Err _ => True ⦄ := by sorry

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
      | .Err _ => False ⦄ := by sorry

/-- Redeeming more shares than exist fails with `MathOverflow` (from the conversion or the
`burn_shares` underflow) before any write. -/
@[lusterna]
theorem vault_redeem_shares_too_many (v : state.Vault) (n : Std.U64)
    (hn : v.shares.val < n.val) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by sorry

/-- Redeeming from a vault with no shares but positive assets divides by zero in
`convert_shares_to_assets` and fails with `MathOverflow` before any write. -/
@[lusterna]
theorem vault_redeem_shares_zero_shares (v : state.Vault) (n : Std.U64)
    (hs : v.shares.val = 0) (ha : 0 < v.assets.val) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_redeem_shares`. -/
@[lusterna]
theorem vault_redeem_shares_preserves_solvent (v : state.Vault) (n : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_redeem_shares v n ⦃ r => Solvent (committed v r) ⦄ := by
  sorry

/-! ## `vault_update_reward` -/

/-- An observed balance below the recorded assets is rejected with `MathOverflow` and the vault is
unchanged. -/
@[lusterna]
theorem vault_update_reward_below (v : state.Vault) (b : Std.U64) (hb : b.val < v.assets.val) :
    operations.vault_operations.vault_update_reward v b ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by sorry

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
      | .Err _ => False ⦄ := by sorry

/-- `vault_update_reward` preserves solvency in every outcome (it never decreases assets and never
touches shares; on `.Err` the vault is untouched). -/
@[lusterna]
theorem vault_update_reward_preserves_solvent (v : state.Vault) (b : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_update_reward v b ⦃ r => Solvent r.2 ⦄ := by sorry

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
      | .Err _ => ¬ (v.shares.val + a.val ≤ v.assets.val) ⦄ := by sorry

/-- A slash larger than the recorded assets fails with `MathOverflow` before any write. -/
@[lusterna]
theorem vault_process_slash_exceeds_assets (v : state.Vault) (a : Std.U64)
    (ha : v.assets.val < a.val) :
    operations.vault_operations.vault_process_slash v a ⦃ r =>
      r.1 = .Err errors.VaultError.MathOverflow ∧ r.2 = v ⦄ := by sorry

/-- A slash within the assets that would leave the vault insolvent fails with `GuardFail`; the
function has already written the reduced assets in memory (reverted by the runtime). -/
@[lusterna]
theorem vault_process_slash_insolvent (v : state.Vault) (a : Std.U64)
    (ha : a.val ≤ v.assets.val) (hins : v.assets.val < v.shares.val + a.val) :
    operations.vault_operations.vault_process_slash v a ⦃ r =>
      r.1 = .Err errors.VaultError.GuardFail ∧
      r.2.assets.val + a.val = v.assets.val ∧
      r.2.shares = v.shares ⦄ := by sorry

/-- Solvency preservation (committed state) for `vault_process_slash`. -/
@[lusterna]
theorem vault_process_slash_preserves_solvent (v : state.Vault) (a : Std.U64)
    (hsolv : Solvent v) :
    operations.vault_operations.vault_process_slash v a ⦃ r => Solvent (committed v r) ⦄ := by
  sorry

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
      | .Err _ => False ⦄ := by sorry

/-- `vault_collect_fee` preserves solvency. -/
@[lusterna]
theorem vault_collect_fee_preserves_solvent (v : state.Vault) (hsolv : Solvent v) :
    operations.vault_fee_operations.vault_collect_fee v ⦃ r => Solvent r.2 ⦄ := by sorry

/-! ## Conversions (helpers, not declared targets) -/

/-- On a solvent vault `convert_assets_to_shares x` always succeeds, matches `assetsToShares`, and
never exceeds `x`. -/
@[lusterna_lemma "conversion helper: convert_assets_to_shares is not a declared campaign target; supports the deposit properties"]
theorem convert_assets_to_shares_spec (v : state.Vault) (x : Std.U64) (hsolv : Solvent v) :
    state.Vault.convert_assets_to_shares v x ⦃ r =>
      match r with
      | .Ok y => y.val = assetsToShares v x.val ∧ y.val ≤ x.val
      | .Err _ => False ⦄ := by sorry

/-- On a solvent vault with `shares ≠ assets`, a successful `convert_shares_to_assets y` returns
`floor(y * assets / shares) ≥ y`. -/
@[lusterna_lemma "conversion helper: convert_shares_to_assets is not a declared campaign target; supports the redeem / exact-deposit properties"]
theorem convert_shares_to_assets_spec (v : state.Vault) (y : Std.U64) (hsolv : Solvent v)
    (hne : v.shares.val ≠ v.assets.val) :
    state.Vault.convert_shares_to_assets v y ⦃ r =>
      match r with
      | .Ok z => z.val = y.val * v.assets.val / v.shares.val ∧ y.val ≤ z.val
      | .Err _ => True ⦄ := by sorry

end CertoraVault.Spec.Spec

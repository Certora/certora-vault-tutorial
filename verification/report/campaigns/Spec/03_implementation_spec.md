# 3. The properties, in Rust terms

All theorems are in `verification/lean/CertoraVault/Spec/Spec.lean` (cited below as `Spec.lean:N`).
Their dependency lemmas are in `verification/lean/CertoraVault/VaultLemmas.lean`. **Lean build:
success** (`lean_build_success: true`, `sorry_remaining: 0`).

## How to read a statement

Every `@[lusterna]` theorem has the form `f args ⦃ r => P r ⦄`. This is notation for
`Aeneas.Std.WP.spec (f args) (fun r => P r)` (macro at
`/opt/aeneas/backends/lean/Aeneas/Std/WP.lean:358-361`). `WP.spec` is an inductive with a
**single** constructor, `ret : p x → spec (.ok x) p` (`WP.lean:30-31`), so it can only hold if the
call returns `ok`. Every theorem therefore says two things: **the translated function terminates
without panicking** (in the Lean model, a Rust panic would be `fail`, and `spec` of `fail` is
unprovable), and its result satisfies `P`.

The result `r` of an operation taking `&mut Vault` is a pair. `r.1` is the Rust return value
(`core.result.Result VaultEffect VaultError`, i.e. `Ok e` / `Err err`). `r.2` is the `Vault` as it
stands when the function returns, including any in-place writes made before an `Err`. `x.val` is
the integer value of a `u64` as a natural number (§4).

Helper definitions used in the statements:
- `Solvent v := v.shares.val ≤ v.assets.val` (`Spec.lean:29`), the Rust `Vault::is_solvent`.
- `assetsToShares v x := if shares = assets then x else x * shares / assets` (`Spec.lean:33-34`),
  and `sharesToAssets v y := if shares = assets then y else y * assets / shares` (`:38-39`). `/` is
  floor division on naturals.
- `feeOf bps amt := amt * bps / 10000` (`:42`).
- `committed v r` is `r.2` if `r.1 = Ok _`, and the pre-state `v` if `r.1 = Err _` (`:47-51`).

## Theorem by theorem

| # | Theorem (`Spec.lean` line) | What it claims about the Rust code | Discharges INFER |
| --- | --- | --- | --- |
| 1 | `is_solvent_spec` (:58) | `Vault::is_solvent` never panics and returns `true` exactly when `shares ≤ assets`. | anchor bridge for `is_solvent` |
| 2 | `default_vault_solvent` (:68) — *surface-only* | An all-zero `Vault` value satisfies `shares ≤ assets`. Runs no code. | Invariant 2 (partially, see gap G1) |
| 3 | `vault_deposit_assets_ok` (:77) | From a solvent vault: no panic. On `Ok(e)`: `e.shares_to_user = assetsToShares(t) > 0`, `e.assets_to_vault = t`, the other effect fields are 0, `shares += e.shares_to_user`, `assets += t`, `fee_bps` and `fee_amount` unchanged, result solvent. Says nothing on `Err`. | P1 |
| 4 | `vault_deposit_assets_zero_shares` (:134) | If the share conversion is *defined* (`shares = assets` or `assets > 0`) and gives 0 shares, the call returns `Err(GuardFail)` and the vault is byte-for-byte unchanged. No solvency needed. | P2 |
| 5 | `vault_deposit_assets_preserves_solvent` (:166) | From a solvent vault, the committed state is solvent. | P1, Invariant 1 |
| 6 | `vault_deposit_assets_with_fee_ok` (:183) | For **any** vault: no panic. On `Ok(e)`: `fee_bps ≤ 10000`, `e.assets_to_fee = floor(t·fee_bps/10000)`, `e.assets_to_vault + e.assets_to_fee = t`, `e.shares_to_user = assetsToShares(e.assets_to_vault) > 0`, `shares += e.shares_to_user`, `assets += e.assets_to_vault`, `fee_bps` and `fee_amount` unchanged. | P3 |
| 7 | `vault_deposit_assets_with_fee_bad_bps` (:238) | `fee_bps > 10000` ⇒ `Err(GuardFail)` and the vault is unchanged. | P3 |
| 8 | `vault_deposit_assets_with_fee_preserves_solvent` (:252) | From a solvent vault, the committed state is solvent. | P3, Invariant 1 |
| 9 | `vault_deposit_assets_exact_ok` (:277) | From a solvent vault: no panic. On `Ok(e)`: `e.shares_to_user = assetsToShares(t) > 0`, `e.assets_to_vault = sharesToAssets(e.shares_to_user)`, `e.shares_to_user ≤ e.assets_to_vault ≤ t`, state updated by exactly those amounts, fees unchanged, result solvent. | P4 |
| 10 | `vault_deposit_assets_exact_preserves_solvent` (:342) | From a solvent vault, the committed state is solvent. | P4, Invariant 1 |
| 11 | `vault_deposit_assets_with_fee_exact_spec` (:361) | For any vault: no panic, and **the vault is returned unchanged whatever the outcome** (`r.2 = v`). On `Ok(e)`: `fee_bps ≤ 10000`, `e.shares_to_user = assetsToShares(t − feeOf(t))`, `e.assets_to_vault = sharesToAssets(e.shares_to_user)`, `e.assets_to_fee = feeOf(e.assets_to_vault)`. | P5 |
| 12 | `vault_deposit_assets_with_fee_exact_bad_bps` (:410) | `fee_bps > 10000` ⇒ `Err(GuardFail)` and the vault is unchanged. | P5 |
| 13 | `vault_deposit_assets_with_fee_exact_preserves_solvent` (:424) | From a solvent vault, the committed state is solvent (trivially: nothing changes). | P5, Invariant 1 |
| 14 | `vault_redeem_shares_ok` (:441) | For **any** vault (no solvency assumed): no panic. On `Ok(e)`: `e.shares_to_burn = n`, `e.assets_to_user = sharesToAssets(n)`, the other effect fields are 0, `shares' + n = shares`, `assets' + e.assets_to_user = assets`, fees unchanged, **result solvent**. | P6, Invariant 1 ("redeem establishes solvency on Ok") |
| 15 | `vault_redeem_shares_succeeds` (:486) | Solvent vault, conversion defined (`shares = assets` or `shares > 0`), `n ≤ shares` ⇒ the call returns `Ok` (the `Err` branch is `False`) with the amounts above. | P7 (with extra hypothesis, see note N2) |
| 16 | `vault_redeem_shares_too_many` (:542) | `n > shares` ⇒ `Err(MathOverflow)` and the vault is unchanged. | P7 ("when n > shares it returns Err"), P8 second half |
| 17 | `vault_redeem_shares_zero_shares` (:562) | `shares = 0`, `assets > 0` ⇒ `Err(MathOverflow)` (division by zero) and the vault is unchanged. | P8 |
| 18 | `vault_redeem_shares_preserves_solvent` (:577) | From a solvent vault, the committed state is solvent. | Invariant 1 |
| 19 | `vault_update_reward_below` (:593) | `b < assets` ⇒ `Err(MathOverflow)` and the vault is unchanged. | P9 |
| 20 | `vault_update_reward_ok` (:607) | `b ≥ assets` ⇒ returns `Ok` (the `Err` branch is `False`) with the all-zero effect, `assets' = b`, and `shares`, `fee_bps`, `fee_amount` unchanged. | P9 |
| 21 | `vault_update_reward_preserves_solvent` (:644) | From a solvent vault, the returned vault `r.2` is solvent **in every outcome**. This is stated over `r.2` directly, not `committed`, so no revert assumption is needed. | P9, Invariant 1 |
| 22 | `vault_process_slash_spec` (:673) | For any vault: no panic. `Ok(e)` ⇒ `shares + a ≤ assets`, `assets' + a = assets`, shares and fees unchanged, `e.assets_to_user = a`, other effect fields 0, result solvent. `Err` ⇒ `¬(shares + a ≤ assets)`. Together: **`Ok` ⇔ `shares + a ≤ assets`**. | P10, Invariant 1 ("slash establishes solvency on Ok") |
| 23 | `vault_process_slash_exceeds_assets` (:704) | `a > assets` ⇒ `Err(MathOverflow)` and the vault is unchanged. | P10 |
| 24 | `vault_process_slash_insolvent` (:722) | `a ≤ assets < shares + a` ⇒ `Err(GuardFail)`, **and the returned vault already has `assets` reduced by `a`** (the partial write before the check). | edge case "slash … partial mutation on Err" |
| 25 | `vault_process_slash_preserves_solvent` (:742) | From a solvent vault, the committed state is solvent. | Invariant 1 |
| 26 | `vault_collect_fee_spec` (:760) | For any vault: always `Ok` (the `Err` branch is `False`), `e.assets_to_user = old fee_amount`, `fee_amount' = 0`, `shares`/`assets`/`fee_bps` unchanged, other effect fields 0. | P11 |
| 27 | `vault_collect_fee_preserves_solvent` (:774) | From a solvent vault, `r.2` is solvent. | Invariant 1 |
| 28 | `convert_assets_to_shares_spec` (:784), lemma | On a solvent vault, `convert_assets_to_shares(x)` always returns `Ok(y)` with `y = assetsToShares(x) ≤ x`. | P12 (first half) |
| 29 | `convert_shares_to_assets_spec` (:808), lemma | On a solvent vault with `shares ≠ assets`, **if** `convert_shares_to_assets(y)` returns `Ok(z)` then `z = floor(y·assets/shares) ≥ y`. The `Err` branch is unconstrained. | P12 (second half) |

## INFER coverage

INFER (`verification/infer/campaigns/Spec.json`) lists 12 properties (P1–P12 in the order they
appear in `properties`), 2 invariants and 11 edge cases. **Every property and both invariants have
at least one corresponding theorem.** There is no property with no theorem. The table above gives
the mapping. Points where the theorem is *not literally* the INFER sentence:

- **N1 (P2, refinement, not a gap).** INFER: "returns `Err(GuardFail)` … when the computed share
  amount is 0". Theorem 4 adds `hdef : shares = assets ∨ 0 < assets`. That hypothesis is necessary.
  When `shares ≠ assets` and `assets = 0`, the Rust conversion divides by zero and returns
  `Err(MathOverflow)`, not `GuardFail` (`mul_div_floor`, `CertoraVault.lean:114-115`). Meanwhile the
  Lean formula `x * shares / 0` evaluates to `0`, because natural division by zero is zero. Without
  `hdef`, the statement would wrongly predict `GuardFail` there.
- **N2 (P7, refinement; INFER's wording is false in one corner).** INFER: "on a solvent vault with
  `n <= shares` always succeeds". Theorem 15 adds `hconv : shares = assets ∨ 0 < shares`. Take
  `shares = 0 < assets`: this is solvent, and `n ≤ shares` forces `n = 0`. Then
  `convert_shares_to_assets` divides by `shares = 0` and returns `Err(MathOverflow)`. So **redeeming
  0 shares from a vault with 0 shares and positive assets fails**, and INFER's sentence is false
  there. Theorem 17 (`vault_redeem_shares_zero_shares`) proves the failure explicitly. This is real,
  harmless behaviour of the code, and the theorem set states it precisely.
- **N3 (P12 second half, weaker than INFER's phrasing).** INFER says `convert_shares_to_assets(y)`
  "returns" `floor(y*assets/shares) ≥ y`. Theorem 29 only constrains the `Ok` case. That is correct:
  the conversion *can* fail with `MathOverflow` when `y·assets/shares > u64::MAX`, so it does not
  always return.
- **G1 (Invariant 2, partial coverage).** "The default/new vault is solvent." This is discharged
  only by the surface-only theorem 2, about a hand-written zero literal. `Vault::new()` /
  `Vault::default()` was not translated (accountability §Projections 1), so no theorem runs the real
  constructor. By reading: `#[derive(Default)]` on the original `Vault`
  (`programs/vault/src/state.rs:13`) zero-initialises every `PodU64`, which is the same as the
  literal. The link is by reading, not proof.
- **Edge cases.** The partial mutation on `Err` in the deposit operations (mint succeeds, then
  `add_token` overflows) is not characterised: the `_ok` theorems put `True` on `Err`. This is the
  intended design. Those states are covered by the committed-state solvency theorems, which rely on
  the runtime revert (§4). The slash partial write *is* characterised (theorem 24).

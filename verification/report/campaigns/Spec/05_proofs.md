# 5. Proof status per theorem

**Authoritative verdict: `#print axioms`**, as recorded in `verification/report/axioms.json`. A
theorem is *established* only if its proof rests on nothing beyond `propext`, `Classical.choice` and
`Quot.sound`. All 29 meet this bar. I independently re-ran `#print axioms` on all 29 against the
built `.olean`s (`verification/lean/.lake/build/lib/lean/CertoraVault/Spec/Spec.olean`), and each
printed exactly `[propext, Classical.choice, Quot.sound]`. **No discrepancy with `axioms.json`.**

- **REFUTED:** none (`axioms.refutations = []`; `verification/prove/assumptions.md`: "No theorem was
  found false").
- **Tainted:** none (`axioms.tainted = []`, `sorry_remaining = 0`, no `native_decide`, no `axiom`
  in any campaign file).
- **Verified modulo trusted base:** none (`impl_verified_assumed = []`, `declared_assumptions = []`).

Proof technique, common to all of them: unfold the translated function, then advance through each
call with Aeneas's `step` tactic. `step` uses the per-helper specifications in
`verification/lean/CertoraVault/VaultLemmas.lean` (`mul_div_floor.spec` :75, the two conversion
specs :115/:143, `mint_shares.spec` :172, `add_token.spec` :202, `burn_shares.spec` :232,
`check_invariant.spec` :253, `del_token.spec` :272, `fee_in_bps.spec` :311, `FeeBps.apply.spec`
:335). Each helper spec states **exactly which Rust outcome occurs and what the vault becomes** for
every input. The proof then splits on `Ok`/`Err` at each `?` and closes the arithmetic with
`scalar_tac`/`agrind` and the pure `Nat` facts at `VaultLemmas.lean:371-409`. These helper lemmas
are not separately listed in `axioms.json`, but they are transitively inside every theorem's
`#print axioms` closure, so their soundness is covered by the same verdict.

| Theorem (`Spec.lean` line) | Status | Sketch |
| --- | --- | --- |
| `is_solvent_spec` (:58) | **implementation-verified** | unfold, `num_shares_eq`/`num_assets_eq` simp, `decide_eq_true_eq`, `scalar_tac` both ways |
| `default_vault_solvent` (:68) | **surface-only** | `simp [Solvent]` on a literal; runs no translated function |
| `vault_deposit_assets_ok` (:77) | **implementation-verified** | step through convert → `mint_shares` → `add_token`; `mul_div_le_self_of_le` for `y ≤ t`, which gives solvency |
| `vault_deposit_assets_zero_shares` (:134) | **implementation-verified** | conversion yields 0, so `mint_shares` hits its `amt = 0` guard and returns `GuardFail`, `v' = v`; the divide-by-zero branch is excluded by `hdef` |
| `vault_deposit_assets_preserves_solvent` (:166) | **implementation-verified** | `WP.spec_mono` from `_ok`; `Err` case uses `committed = v` |
| `vault_deposit_assets_with_fee_ok` (:183) | **implementation-verified** | `fee_in_bps` → `FeeBps.apply` (total when `bps ≤ 10000`) → convert → mint → add |
| `vault_deposit_assets_with_fee_bad_bps` (:238) | **implementation-verified** | `fee_in_bps.spec` `Err` branch; vault untouched |
| `vault_deposit_assets_with_fee_preserves_solvent` (:252) | **implementation-verified** | from `_with_fee_ok` plus `mul_div_le_self_of_le` |
| `vault_deposit_assets_exact_ok` (:277) | **implementation-verified** | two conversions; `le_mul_div_of_le` (`y ≤ z`), `round_trip_le` (`z ≤ t`) |
| `vault_deposit_assets_exact_preserves_solvent` (:342) | **implementation-verified** | `WP.spec_mono` from `_exact_ok` |
| `vault_deposit_assets_with_fee_exact_spec` (:361) | **implementation-verified** | every path returns the input `vault`; `Ok` values from the helper specs |
| `vault_deposit_assets_with_fee_exact_bad_bps` (:410) | **implementation-verified** | `fee_in_bps.spec` `Err` branch |
| `vault_deposit_assets_with_fee_exact_preserves_solvent` (:424) | **implementation-verified** | from `r.2 = v` |
| `vault_redeem_shares_ok` (:441) | **implementation-verified** | convert → `burn_shares` → `del_token`; solvency from `del_token.spec`'s `Ok` branch (`check_invariant` passed) |
| `vault_redeem_shares_succeeds` (:486) | **implementation-verified** | `redeem_le_assets` rules out `del_token` underflow; `redeem_solvent` rules out the `GuardFail` branch; every `Err` branch is contradicted |
| `vault_redeem_shares_too_many` (:542) | **implementation-verified** | `burn_shares` underflow gives `MathOverflow`, `v' = v`; or the conversion already erred with `MathOverflow` |
| `vault_redeem_shares_zero_shares` (:562) | **implementation-verified** | `shares ≠ assets` and divisor `shares = 0`, so `mul_div_floor` returns `Err` before any write |
| `vault_redeem_shares_preserves_solvent` (:577) | **implementation-verified** | `WP.spec_mono` from `_ok` |
| `vault_update_reward_below` (:593) | **implementation-verified** | `checked_sub` is `none`, so `MathOverflow`, vault untouched |
| `vault_update_reward_ok` (:607) | **implementation-verified** | case split on `reward > 0`; `add_token` cannot overflow because `assets + (b − assets) = b ≤ u64::MAX` |
| `vault_update_reward_preserves_solvent` (:644) | **implementation-verified** | assets only grow; on `Err` the vault is unchanged (`add_token.spec`) |
| `vault_process_slash_spec` (:673) | **implementation-verified** | `del_token.spec`, both branches |
| `vault_process_slash_exceeds_assets` (:704) | **implementation-verified** | `del_token.spec` `MathOverflow` branch |
| `vault_process_slash_insolvent` (:722) | **implementation-verified** | `del_token.spec` `GuardFail` branch (with the partial write) |
| `vault_process_slash_preserves_solvent` (:742) | **implementation-verified** | `Ok`: `shares ≤ assets − a`; `Err`: `committed = v` |
| `vault_collect_fee_spec` (:760) | **implementation-verified** | straight-line; `simp [zeroEffect]` |
| `vault_collect_fee_preserves_solvent` (:774) | **implementation-verified** | shares and assets untouched |
| `convert_assets_to_shares_spec` (:784), lemma | **implementation-verified** | `convert_assets_to_shares.spec`; the `Err` branch is impossible because `x·s/a ≤ x ≤ u64::MAX` and `a ≠ 0` when `s ≤ a`, `s ≠ a` |
| `convert_shares_to_assets_spec` (:808), lemma | **implementation-verified** | `convert_shares_to_assets.spec` `Ok` branch plus `le_mul_div_of_le` |

**Totals:** 28 implementation-verified, 1 surface-only, 0 abstract-only, 0 modulo-base, 0 tainted,
0 refuted, 0 open.

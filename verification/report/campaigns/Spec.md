# Campaign report — Spec

# Campaign `Spec` — overview

*Review stage, written for the owner of `programs/vault` (`certora_vault_tutorial`). All paths are
relative to the repository root (`/workspace/repo`) unless they start with `/`. The Aeneas standard
library is cited at its installed location, `/opt/aeneas/backends/lean/Aeneas/Std/…`.*

## Executive summary

This campaign covered the **eight pure vault operations** in `programs/vault/src/operations/`
(`vault_deposit_assets`, `vault_deposit_assets_with_fee`, `vault_deposit_assets_exact`,
`vault_deposit_assets_with_fee_exact`, `vault_redeem_shares`, `vault_update_reward`,
`vault_process_slash`, `vault_collect_fee`) plus the solvency check `Vault::is_solvent`. Charon and
Aeneas translated them into Lean, and 29 theorems were proved about the translation. **28 of the 29
verify the implementation:** each one runs a translated Rust function and constrains its result, and
each proof depends only on Lean's three standard axioms. There is no `sorry`, no declared assumption,
no tainted theorem and no refuted property. The 29th theorem (`default_vault_solvent`) is
**surface-only**: it mentions the translated `Vault` type but runs no function. The main result is
that **every operation, when it returns `Ok` from a solvent vault, leaves the vault solvent
(`shares ≤ assets`)**. In addition, `vault_redeem_shares` and `vault_process_slash` establish solvency
on `Ok` from *any* starting state. Every operation is also proved to be panic-free. The one
measurement the properties work through, `is_solvent`, is **grounded**: a proved theorem ties it to
the real getter.

There are three caveats a reader should hold onto:

1. The proofs show the code does what it does. They also confirm one behaviour that looks like a
   **business-logic defect**: `vault_deposit_assets_with_fee_exact` *never* updates
   `Vault.shares`/`Vault.assets` (theorem `vault_deposit_assets_with_fee_exact_spec`,
   `verification/lean/CertoraVault/Spec/Spec.lean:361`), but its processor still moves tokens into
   the vault and mints shares (`programs/vault/src/processor/deposit_with_fee_exact.rs:38-62`). For
   that path, solvency holds only trivially and says nothing about the real token flows (§4, §6).
2. Solvency on a failed call is stated over a *committed* state, meaning the pre-state when the call
   returns `Err`. That relies on Solana reverting the account when the instruction fails. This is a
   modelling assumption outside the kernel. I confirmed by reading that every processor propagates
   the operation's `Err` with `?` (§4).
3. The model is built from an **extraction crate**, not the original crate. Its `Vault` keeps only
   four fields, and three hand-made code edits (E1–E3) were checked by differential testing, not by
   proof (§2).

## Overview table (authoritative: `verification/report/axioms.json`)

| Category | Count | Theorems |
| --- | ---: | --- |
| **Refuted** (`axioms.refutations`) | **0** | — |
| **Implementation-verified** (`axioms.impl_verified`): standard axioms only, runs a translated function | **28** | all `@[lusterna]` theorems plus the two conversion lemmas (list in §5) |
| Verified modulo trusted base (`axioms.impl_verified_assumed`) | 0 | — (`declared_assumptions` = `[]`) |
| **Surface-only** (`axioms.surface_only`): names a translated type, runs nothing | 1 | `default_vault_solvent` |
| Abstract-only (`axioms.abstract_only`) | 0 | — |
| Tainted (`axioms.tainted`) | 0 | — |
| Illegitimate assumptions | 0 | — |
| `sorry` remaining | 0 | — |
| Lean build | success | `lean_build_success: true` |

**Headline: 28 theorems verify the implementation.** `default_vault_solvent`
(`Spec.lean:68`) is weaker and is **not** part of the headline. It proves that a hand-written
all-zero `state.Vault` literal satisfies `shares ≤ assets`. The real constructor `Vault::new()`
(`programs/vault/src/state.rs:33-35`) was not translated, so this theorem is not tied to any code
that runs.

## Grounding (rung 3)

| Anchor (INFER `anchor_functions`) | Status | Bridge theorem |
| --- | --- | --- |
| `is_solvent` | **grounded** | `is_solvent_spec` (`Spec.lean:58`): `state.Vault.is_solvent v ⦃ b => (b = true ↔ Solvent v) ⦄` |

That is 1 of 1 anchors grounded, with `axioms.grounding.ungrounded` = `[]`. Every other projection
the properties use (`assetsToShares`, `sharesToAssets`, `feeOf`, `committed`) appears *inside* a
theorem that runs the real operation. §4 explains why those need no separate bridge, and why
`committed` is the exception: it carries a runtime assumption.

## Other harness verdicts

- **Pre-proof spec-judge screen:** `spec_judge.defects` = `[]`, so no defects were found in the
  *statements* before proving. This is a screen of the statements, not the post-proof fidelity
  review in §4.
- **Statement drift:** all 29 statements are `stable` and none are `unaccounted`. The proved
  statements are the ones that were screened.
- **Independent cross-check:** I re-ran `#print axioms` on all 29 theorems against the built
  `.olean`s. Each reports exactly `[propext, Classical.choice, Quot.sound]`, which agrees with
  `axioms.json`.
- **Overflow posture:** `overflow_posture.block = false`. The deployed `[profile.release]`
  (`Cargo.toml:5-7`, `overflow-checks = true`) was copied into the extraction manifest. However,
  `model_checked_ops = false`: the translated code has *no* overflow-checked operator at all. Every
  arithmetic step is an explicit `checked_*` call, so the model does not depend on the overflow
  profile (§2, §4).


# 2. Translation: what was translated, and what that costs in trust

## Entry point, scope, output

| Item | Value | Source |
| --- | --- | --- |
| INFER entry file | `src/lib.rs` of `programs/vault` | `verification/infer/campaigns/Spec.json` (`entry_file`) |
| Targets (9) | the 8 operations + `state::_::is_solvent` | `axioms.json` → `target_patterns` |
| Strategy | **scoped extraction crate** `verification/translate/extract` (package/lib `certora_vault`, its own `[workspace]`) | `verification/translate/campaigns/Spec/accountability.md` §Decision |
| Charon | `charon cargo --preset=aeneas -- --release`, run in `translate/extract` → `certora_vault.llbc` | accountability.md §Decision |
| Aeneas | `aeneas -backend lean -dest verification/lean certora_vault.llbc` → `verification/lean/CertoraVault.lean` (single module, 715 lines) | accountability.md; file header `CertoraVault.lean:1-2` |
| Emitted axioms | **none** (`emitted_axioms: []`) | `translate/campaigns/Spec/verdict.json` |
| Opaqued functions / holes | **none** (`opaque_assumptions: []`, `holes: []`) | `axioms.json` |
| `--opaque` / `--exclude` | none used | accountability.md §Decision |

All nine targets are real Lean `def`s with bodies: for example `state.Vault.is_solvent` is at
`CertoraVault.lean:543-546`, `operations.vault_operations.vault_deposit_assets` at `:499-538`, and
`utils.math.math_native.mul_div_floor` at `:102-130`. Everything they call was also translated as a
body: `mint_shares` `:79-97`, `add_token` `:50-68`, `burn_shares` `:592-607`, `del_token`
`:563-587`, `check_invariant` `:551-558`, the conversions `:135-172`, `FeeBps::apply` `:271-291`,
and `FeeBps::try_from` `:296-302`. **Nothing in the verified call graph is an opaque stub.**

## Summary of the translation agent's accountability trail

This reports the agent's trail as written in `verification/translate/campaigns/Spec/accountability.md`.

**Why an extraction crate.** An in-place translation of `programs/vault` would pull in seven
`Pubkey` fields plus bump seeds, `bytemuck::Pod`, `spl_pod::PodU64`, `solana_program`, and the
loaders and processors. None of these are part of the state the properties are about
(`relevant_state`). So the agent built a standalone crate containing only the operation bodies and
their helpers. The original program directory was not modified.

**What was copied verbatim** (per the trail, and confirmed by my `diff` of
`programs/vault/src` against `verification/translate/extract/src`): `operations/vault_operations.rs`
(minus its `#[cfg(test)]` module), `operations/vault_fee_operations.rs`,
`operations/vault_exact_deposit_operations.rs`, `operations/mod.rs`, `utils/guards.rs`, and
`utils/mod.rs`. The `diff` shows **no change** to any of the three operations files apart from the
`VaultEffect` derive list and the removed test module.

**Projections and exclusions (ABSENT, not opaqued):**
1. `Vault` is projected to `shares, assets, fee_bps, fee_amount`. The dropped fields (`admin`,
   `slash_admin`, `shares_mint`, `assets_mint`, `fee_token_account`, `vault_assets_account`, and two
   bump bytes) are read by no target. Compare the original `programs/vault/src/state.rs:14-30` with
   the extraction `translate/extract/src/state.rs:9-15`.
2. `VaultEffect` keeps the same five `u64` fields and derives only `Default`.
3. `VaultError` keeps the same four variants in the same order, and drops
   `impl From<VaultError> for ProgramError`.
4. Absent entirely: `solana_program`, `spl_pod`, `bytemuck`, `spl_token`, `num_enum`, `loaders/`,
   `processor/`, `instruction`, `constants`. Unreachable helpers were also dropped: `mul_div_ceil`,
   `FeeBps::apply_ceil`, `FeeBps::is_zero`, the `*_ceil` conversions, `gross_assets`, `validate`, and
   `Vault::new`. I checked that none of the dropped `_ceil` helpers or `gross_assets` is called by an
   operation or a processor: the only `_ceil` call sites are the dropped helpers themselves
   (`programs/vault/src/state.rs:82,101`, `utils/math.rs:23,59`).

**Rung-3 behaviour-preserving edits, with the agent's stated rationale:**
- **E1: `PodU64` → `u64`** for the four vault fields. Rationale: `PodU64` is a little-endian
  `[u8; 8]` that the program only reads with `u64::from(pod)` and writes with `.into()`. I confirmed
  the type at `spl-pod-0.2.5/src/primitives.rs:107-108`, whose `impl_int_conversion!` (`:50-64`) is
  exactly `to_le_bytes`/`from_le_bytes`. `PodU64` has **no arithmetic of its own**: every `+`/`-`
  in the vault happens on the `u64` after conversion.
- **E2: `Vault::check_invariant`.** `self.is_solvent().then_some(()).ok_or(GuardFail)`
  (`programs/vault/src/state.rs:66-68`) was rewritten as an explicit `if`
  (`translate/extract/src/state.rs:42-49`). Rationale: `bool::then_some` is not in Aeneas's model
  library, and Aeneas would have emitted it as an `axiom`, tainting `del_token` and everything that
  calls it.
- **E3: `mul_div_floor`.** The trailing `.try_into().map_err(|_| MathOverflow)` (u128→u64,
  `programs/vault/src/utils/math.rs:15-16`) was rewritten as
  `if q > u64::MAX as u128 { Err(MathOverflow) } else { Ok(q as u64) }`
  (`translate/extract/src/utils/math.rs:17-21`). Rationale: `TryFrom<u128> for u64` is not modelled.
  `try_from` fails exactly when `q > u64::MAX`, and otherwise `q as u64` is exact.

**Evidence offered for E1–E3: differential tests, not proofs.**
`translate/extract/src/equivalence_tests.rs` runs the extraction and the *original* crate (as a
dev-dependency, using the real `PodU64` and the full `Vault` with its `Pubkey` fields) on identical
inputs. It compares the `Ok` effect or `Err` variant **and** the post-state of the four kept fields,
including after an `Err`. The five tests are `ops_agree_on_edge_grid` (`:195`),
`ops_agree_on_random_inputs` (`:233`, 300 000 cases), `mul_div_floor_agrees` (`:257`),
`fee_bps_agrees` (`:282`) and `solvency_boundary` (`:305`). The agent reports **5 passed, 0
failed**. I did not re-run them. This is testing evidence and is not covered by the kernel.

## Trust boundary: what the proofs rest on but did not establish

| Item | Kind | Shows up in `#print axioms`? | Comment |
| --- | --- | --- | --- |
| Lean kernel + `propext`, `Classical.choice`, `Quot.sound` | standard axioms | yes, for all 29 theorems | the normal base |
| Opaqued primitives | none | — | `opaque_assumptions: []` |
| Untranslated holes | none | — | `holes: []` |
| Declared assumptions | none | — | `declared_assumptions: []`; `verification/prove/assumptions.md` says "None." |
| **Charon + Aeneas faithfulness** (Rust MIR → LLBC → Lean) | toolchain trust | **no** | true of every Aeneas-based result; the Aeneas `Std` models of `checked_*`, `as` casts and `Option::ok_or` are what §4 reads |
| **Extraction crate = original crate** (projection + E1–E3) | translation fidelity | **no** | evidence is the differential test suite above; not proved |
| **Solana revert-on-error** (the meaning of `committed`) | runtime semantics | **no** | used by the five "committed-state" solvency theorems; see §4 |

Because there are no opaques, holes or declared assumptions, **nothing appears as tainted or
modulo-base in the `#print axioms` verdict, and that is expected**. The three rows marked "no" are
trust you extend *outside* the axiom report. The kernel verdict can neither confirm nor refute them.

## Arithmetic overflow posture

- **Deployed build:** the workspace manifest `Cargo.toml:5-7` has `[profile.release]
  overflow-checks = true` (with `lto = 'thin'`) and no per-package overrides (`overflow_posture.deploy_profile`).
- **Compiled build:** the extraction manifest `verification/translate/extract/Cargo.toml` copies
  that block verbatim (`overflow_posture.compiled_profile` is identical), and Charon was run with
  `-- --release`. The harness verified that both profiles match (`block: false`).
- **What the model actually contains:** `overflow_posture.model_checked_ops = false`. I confirmed
  this by pretty-printing `translate/extract/certora_vault.llbc`. It contains **zero** `panic.`/
  `wrap.`/`checked.` binops and no bare `+ - * /` at all. The only arithmetic is calls to
  `core::num::{u64}::checked_add` / `checked_sub` and `{u128}::checked_mul` / `checked_div`, plus
  four widening `cast<u64, u128>` and one guarded `cast<u128, u64>`. This matches the Rust: every
  arithmetic step in `state.rs` / `math.rs` is written with `.checked_*().ok_or(MathOverflow)?`.

**Consequence:** in this crate, overflow is never a panic. It is an ordinary
`Err(VaultError::MathOverflow)` value in both the deployed binary and the model, whatever the
profile says. The release profile was reproduced anyway, as the rule requires. Even if it were
wrong it would not affect any theorem, because no translated operator depends on it. The one
hand-modelled value type is the E1 swap from `PodU64` to `u64`, and it carries no arithmetic, so it
has no overflow behaviour to match. §4 checks this.

## A repository change outside `verification/` that the owner should know about

`translate/campaigns/Spec/verdict.json` → `source_files_changed: ["rust-toolchain.toml"]`. The
pipeline **deleted the repository's `rust-toolchain.toml`** (which pinned `channel = "1.79"`) in
commit `59413bb` ("feat(spec): informal specification (pre-translate)"). The file no longer exists
at the repository root. The accountability note says the *original program directory* was not
modified, and that is true of `programs/vault/`, but this root-level deletion is a real change to
the shipped repo. It does not affect any theorem: Charon compiles with its own toolchain, and the
semantics of `checked_*` and `as` casts are stable across Rust versions. **But it changes how the
real program builds and should be restored** before anything is merged.


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


# 4. Fidelity review: do the proved statements mean what they claim about the real code?

The kernel shows that each theorem is a true statement about the Lean model. This section asks
whether that statement **says what it appears to say about `programs/vault`**. I answer it by
reading three layers against each other: the Rust (`programs/vault/src/…`, and the extraction copy
`verification/translate/extract/src/…`), the Aeneas translation (`verification/lean/CertoraVault.lean`),
and the Aeneas standard library that defines what a `u64`, a `checked_add` or a `⦃ ⦄` *is*
(`/opt/aeneas/backends/lean/Aeneas/Std/…`, cited below as `Std/…`).

I start with the gaps and the worries, because they decide how much the wins are worth. Then I go
through each projection.

---

## 4.0 Findings first

| # | Finding | Severity for the owner | Where |
| --- | --- | --- | --- |
| F1 | **`vault_deposit_assets_with_fee_exact` never updates the vault, yet its processor moves tokens and mints shares.** The theorem proves the first half (`r.2 = v` in every outcome). The second half is in the processor. The proofs faithfully mirror code that looks wrong. | **High: candidate defect** | `Spec.lean:361-371`; `programs/vault/src/operations/vault_exact_deposit_operations.rs:19-39`; `programs/vault/src/processor/deposit_with_fee_exact.rs:31-62` |
| F2 | Six theorems state solvency over `committed` (`Spec.lean:166,252,342,424,577,742`). Five of them (deposit ×3, redeem, slash) actually depend on it; the fee-exact one does not, because it has `r.2 = v`. These theorems read `Err` as "the pre-state survives". That is **Solana's revert-on-failure semantics**, which no Lean theorem establishes. I confirmed by reading that every processor propagates the operation's `Err` with `?`. | Modelling assumption, sound for Solana | `Spec.lean:47-51`; processors cited in §4.4 |
| F3 | `default_vault_solvent` runs no code (surface-only). The real constructor `Vault::new()` was not translated. | Low | `Spec.lean:68-70`; `programs/vault/src/state.rs:33-35` |
| F4 | The model is of an extraction crate (4-field `Vault`, edits E1–E3). Equivalence to the original is supported by differential tests, **not proof**. | Translation trust (see §2) | `translate/extract/src/equivalence_tests.rs` |
| F5 | The deposit `_ok` theorems assume a solvent starting vault. Nothing is proved about deposits into an insolvent vault, and the deposit code does not check solvency itself. | By design; note it | `Spec.lean:77,277` (`hsolv`) |
| F6 | Out-of-scope observation: no in-scope operation ever increases `fee_amount` (proved: every operation except `collect_fee` keeps it equal, and `collect_fee` zeroes it). Yet `process_collect_fee` transfers `fee_amount` tokens **from the vault's assets account** without reducing `Vault.assets`. | Low / latent | `programs/vault/src/processor/collect_fee.rs:16-22` |

No **ungrounded** projection was found. `axioms.grounding` reports `is_solvent` as grounded, and my
reading below agrees.

---

## 4.1 What a `u64` and a "property" are in this model (checked in `Std`)

These encoding facts are what every projection below depends on. I read them from their
definitions rather than inferring them from names.

- **A Rust `u64` is `Std.U64 = UScalar .U64`** (`Std/Scalar/Core.lean:666`), a structure wrapping a
  64-bit `BitVec` (`:71-73`). **`.val` is `x.bv.toNat`, a natural number `ℕ`** (`:76`). Every
  integer in this crate is unsigned (`u64`, plus `u128` inside `mul_div_floor`). So every `.val` in
  the statements is a `Nat`, no statement mixes `Nat` with `Int`, and there is no signed `.val`
  (`IScalar.val = bv.toInt`, `:84`) anywhere.
- **Comparison and equality are value comparisons.** `instance LE (UScalar ty)` is
  `a.val ≤ b.val` (`Core.lean:897`), `LT` is `a.val < b.val` (`:894-895`), and two `UScalar`s are
  equal iff their `.val`s are (`UScalar.eq_equiv`, `:906`). So Rust's `self.num_shares() ==
  self.num_assets()` and `<=` translate to exactly the `Nat` facts the statements use.
- **`⦃ ⦄` is total correctness.** `f x ⦃ r => P ⦄` unfolds to `WP.spec (f x) P` (`Std/WP.lean:358-361`).
  `WP.spec` has the single constructor `ret : p x → spec (.ok x) p` (`WP.lean:30-31`), so it
  **cannot hold if the call `fail`s (panics) or diverges**. Every `@[lusterna]` theorem therefore
  also proves that the function does not panic under its hypotheses.
- **`Nat` subtraction truncates at 0 and `Nat` division by 0 is 0.** Both can make a
  faithful-looking formula lie. Where each could matter is addressed in §4.3.

---

## 4.2 The anchor: `Solvent` ↔ `Vault::is_solvent` (grounded)

**The three parts.**
- *Projection:* `Solvent v := v.shares.val ≤ v.assets.val` (`Spec.lean:29`).
- *Real function (INFER anchor `is_solvent`):* `Vault::is_solvent`
  (`programs/vault/src/state.rs:62-64`: `self.num_shares() <= self.num_assets()`).
- *Bridge:* `is_solvent_spec` (`Spec.lean:58-59`):
  `state.Vault.is_solvent v ⦃ b => (b = true ↔ Solvent v) ⦄`. It is **established**: it is in
  `axioms.impl_verified`, `axioms.grounding.grounded = ["is_solvent"]`, and my `#print axioms`
  cross-check shows only the standard axioms.

**Walking the code, term by term.**
1. Rust: `self.num_shares() <= self.num_assets()`, where `num_shares` is `self.shares.into()` and
   `num_assets` is `self.assets.into()` (`state.rs:37-43`). On the real account, `shares`/`assets`
   are `PodU64` and `.into()` is `u64::from_le_bytes` (`spl-pod-0.2.5/src/primitives.rs:50-64,107-108`).
2. Extraction: identical except that the fields are already `u64` (`translate/extract/src/state.rs:18-24,38-40`).
   This is edit E1, discussed in §4.5.
3. Lean: `state.Vault.is_solvent self = do let i ← num_shares self; let i1 ← num_assets self; ok (i <= i1)`
   (`CertoraVault.lean:543-546`), with `num_shares self = ok self.shares` (`:73-74`) and
   `num_assets self = ok self.assets` (`:44-45`). The `Bool` `i <= i1` decides the `LE` instance,
   i.e. `self.shares.val ≤ self.assets.val` (`Std/Scalar/Core.lean:897`).
4. That is exactly `Solvent self`. Both sides are `Nat`, with no coercion, no scale factor and no
   subtraction.

**What this buys.** Every solvency theorem concludes `Solvent r.2` or `Solvent (committed v r)`. By
the bridge, that is the same as saying **the program's own `is_solvent()` would return `true` on
that vault**. The program uses the same predicate as its guard: `check_invariant` calls
`is_solvent` (`CertoraVault.lean:551-558`, from `state.rs:66-68` via edit E2), and `del_token`
calls `check_invariant` after writing (`CertoraVault.lean:563-587`). So the property is stated in
the code's own notion of solvency, not an approximation of it. Stating it over `.val` is also the
only way to state it at the value level: the machine stores a 64-bit pattern, `≤` on patterns is
*defined* as `≤` on their `toNat` readings, and that is the reading the program means by `<=`.

---

## 4.3 The result formulas: `assetsToShares`, `sharesToAssets`, `feeOf`

These are **not** stand-ins for a separate measurement that the properties reason about instead of
the code. They appear only as the right-hand side of an equation about the real output, for example
`e.shares_to_user.val = assetsToShares v t.val` inside a triple that runs
`vault_deposit_assets`. **The theorem that states the equation is itself the bridge.** If the
formula were unfaithful, the theorem would be false and would not have been proved. Fidelity still
matters in one sense: a reader takes the formula as *documentation* of what the code computes, so
it should mean what it looks like. I checked it term by term.

### `mul_div_floor(a, b, c)`: the arithmetic core (`CertoraVault.lean:102-130`)

| Step | Lean (line) | `Std` definition | Exact behaviour |
| --- | --- | --- | --- |
| `a as u128`, `b as u128` | `:106-107` `UScalar.cast .U128` | zero-extend (`Std/Scalar/Casts.lean:20-22`), `.val` = `x.val % 2^128` (`:209-210`) | lossless (a `u64` is < 2^64) |
| `checked_mul` | `:108` `U128.checked_mul` | `Option.ofResult (UScalar.mul x y)` (`Std/Scalar/CheckedOps/Mul.lean:15-16`). `UScalar.mul = tryMk (x.val * y.val)` (`Std/Scalar/Ops/Mul.lean:18-19`), so it is `none` iff the product > `U128.max` | **never `none` here:** `a.val·b.val ≤ (2^64−1)^2 < 2^128` (`VaultLemmas.lean:25-30`) |
| `checked_div` | `:114` `U128.checked_div` | `Option.ofResult (UScalar.div x y)` (`CheckedOps/Div.lean:15-16`). `UScalar.div` is `fail divisionByZero` if `y = 0`, else `BitVec.udiv` (`Ops/Div.lean:17-18`) | `none` iff `c = 0`, otherwise floor division |
| `ok_or(MathOverflow)?` | `:109-111`, `:115-117` | `some v ↦ ok (.Ok v)`, `none ↦ ok (.Err e)` (`Std/Core/CoreOption.lean:24-28`) | a `None` becomes **a Rust `Err` value, not a panic** |
| `q > u64::MAX` guard (edit E3) | `:119-121` | `core.num.U64.MAX = 2^64−1` (`Std/Scalar/Core.lean:863`) | `Err(MathOverflow)` iff `q > 2^64−1` |
| `q as u64` | `:123` `UScalar.cast .U64` | truncate, `.val` = `q.val % 2^64` (`Casts.lean:209-210`) | lossless here because the guard ensured `q ≤ 2^64−1` |

Result, proved as `mul_div_floor.spec` (`VaultLemmas.lean:75-80`): **`Ok z` with `z.val = a·b / c`
(floor) iff `c ≠ 0` and the quotient fits in `u64`, and otherwise `Err(MathOverflow)`.** That is
exactly the Rust `mul_div_floor` (`programs/vault/src/utils/math.rs:9-17`).

### The conversions: argument order checked

- `convert_assets_to_shares(token)`: if `num_shares == num_assets` it returns `token`, otherwise
  `mul_div_floor(token, num_shares, num_assets)` (`state.rs:87-95` → `CertoraVault.lean:141-144`).
  `assetsToShares v x = if shares = assets then x else x * shares / assets` (`Spec.lean:33-34`)
  matches in both branches and in argument order (numerator `shares`, denominator `assets`).
- `convert_shares_to_assets(shares)` calls `mul_div_floor(shares, num_assets, num_shares)`
  (`state.rs:70-77` → `CertoraVault.lean:162-165`). `sharesToAssets v y = … y * assets / shares`
  (`Spec.lean:38-39`) matches, with numerator `assets` and denominator `shares`.

**Worry: `Nat` division by zero.** `x * shares / assets` with `assets = 0` evaluates to `0` in Lean,
while Rust returns `Err(MathOverflow)`. *Answer:* on every `Ok` branch the formula is reached only
after `mul_div_floor` succeeded, which implies the divisor is non-zero (`VaultLemmas.lean:78`,
`c.val ≠ 0`), so the two agree. The single statement where the formula is used *outside* an `Ok`
branch is `vault_deposit_assets_zero_shares` (`Spec.lean:134-136`, hypothesis
`assetsToShares v t.val = 0`). It carries the explicit guard `hdef : shares = assets ∨ 0 < assets`,
which removes exactly the divide-by-zero case. Without `hdef` the theorem would claim
`Err(GuardFail)` where the code gives `Err(MathOverflow)`. The authors caught this, and the
statement is precise.

### `feeOf` and `FeeBps::apply`

`FeeBps::apply(amt)` is `fee = mul_div_floor(amt, bps, ONE_IN_BPS)` followed by
`net = amt.checked_sub(fee)` (`programs/vault/src/utils/math.rs:52-56` → `CertoraVault.lean:271-291`).
`ONE_IN_BPS = 10000#u64` (`CertoraVault.lean:259`), and `FeeBps` is a reducible alias of `Std.U64`
(`:255`), so `bps.val` is the plain basis-point value with no hidden scale factor.
`feeOf bps amt = amt * bps / 10000` (`Spec.lean:42`) matches.

**Worry: `Nat` subtraction truncates.** `vault_deposit_assets_with_fee_exact_spec` writes
`t.val - feeOf …` (`Spec.lean:367`). *Answer:* this appears only on the `Ok` branch, where
`fee_bps ≤ 10000` is part of the conclusion, and `feeOf bps t ≤ t` whenever `bps ≤ 10000`
(`fee_le`, `VaultLemmas.lean:327-330`). The truncation never applies. The proof also derives the
value from the real `checked_sub`'s success, not from the formula (`Spec.lean:381-382`). The other
statements avoid `Nat` subtraction altogether by using additive forms:
`assets_to_vault + assets_to_fee = t` (`:189`), `r.2.shares.val + n.val = v.shares.val` (`:448`),
and `r.2.assets.val + a.val = v.assets.val` (`:678`). These are exact over `Nat`.

### Effect fields and unchanged fields

Statements like `r.2.fee_bps = v.fee_bps`, `e.shares_to_burn = n` and `r.2 = v` are equalities of
`UScalar`s or of whole `state.Vault` structures (all 4 fields), so they are exact. The zero effect
fields come from `VaultEffect::default` (`CertoraVault.lean:187-196`), whose fields are
`core.default.DefaultU64.default`. `VaultLemmas.lean:68-69` proves by `rfl` that this is the
all-zero record.

---

## 4.4 The one projection with an extra-kernel assumption: `committed`

**The three parts.**
- *Projection:* `committed v r` is `r.2` on `Ok` and `v` on `Err` (`Spec.lean:47-51`).
- *Real thing it stands for:* the `Vault` account data **as persisted on-chain** after the
  instruction.
- *Bridge:* **none in Lean, and none is possible in this model.** It is Solana's transaction
  atomicity: if a processor returns `Err`, every account write the instruction made is discarded.

**Is it load-bearing?** Yes, for some `Err` paths. On `Err` the Rust functions can leave the
`&mut Vault` partially written:
- deposit: `mint_shares` succeeds, then `add_token` overflows. The Lean shows the post-mint vault
  being returned (`CertoraVault.lean:526`, `ok (r3, vault2)` / `:532`).
- redeem: `burn_shares` succeeds, then `del_token` fails (`:639`, `:645`).
- slash: `del_token` writes the reduced `assets` and then fails `check_invariant`. This is
  **proved** to leave `r.2.assets = v.assets − a`, an insolvent state, in
  `vault_process_slash_insolvent` (`Spec.lean:722-727`).

So `Solvent (committed v r)` is true about the chain only if those `Err`s abort the instruction.

**Confirmed by reading.** Every processor propagates the operation's error with `?`:
`processor/deposit.rs:31`, `deposit_exact.rs:32`, `deposit_with_fee.rs:33-34`,
`deposit_with_fee_exact.rs:33-34`, `redeem_shares.rs:31`, `slash.rs:19`, `update_reward.rs:22`,
`collect_fee.rs:13`. An `Err` therefore becomes the instruction's `ProgramResult::Err`, and the
runtime reverts. **The assumption is sound for the deployed program**, but it is a fact about
Solana and the processors (which are out of scope), not something the kernel checked.

**Where it is *not* needed.** `vault_update_reward_preserves_solvent` (`Spec.lean:644-646`) and
`vault_collect_fee_preserves_solvent` (`:774-775`) state `Solvent r.2` in **every** outcome. The
`Err`-path theorems `…_zero_shares`, `…_bad_bps`, `…_too_many`, `…_below` and `…_exceeds_assets`
prove `r.2 = v` outright, which is stronger than needing a revert.

---

## 4.5 Overflow posture and the hand-modelled type

**Why the model's arithmetic is exact, whatever the profile.** The deployed workspace sets
`[profile.release] overflow-checks = true` (`Cargo.toml:5-7`), and the extraction manifest copies
it verbatim (`verification/translate/extract/Cargo.toml`, last block). The harness verified that the
two match (`overflow_posture.block = false`). In this crate the profile turns out not to matter:
`model_checked_ops = false`, and the pretty-printed LLBC contains **no** `panic.+`/`wrap.+` operator
at all, only calls to `checked_add`/`checked_sub` (u64) and `checked_mul`/`checked_div` (u128).
Each is modelled in `Std` as an `Option` that is `none` exactly when Rust's returns `None`:
- `checked_add` is `Option.ofResult (x + y)` (`Std/Scalar/CheckedOps/Add.lean:15-16`), where
  `x + y = UScalar.add = tryMk (x.val + y.val)` (`Std/Scalar/Ops/Add.lean:16-17`), failing iff the
  sum exceeds `2^64−1`.
- `checked_sub` uses `UScalar.sub`, which fails iff `x.val < y.val` (`Std/Scalar/Ops/Sub.lean:17-19`;
  `CheckedOps/Sub.lean:15-16`).
- `checked_mul` and `checked_div` are as tabulated in §4.3.

They are wrapped in `lift`, which is plain `Result.ok` (`Std/Primitives.lean:456`). So in the model,
as in the binary, **overflow is never a panic. It is the value `Err(VaultError::MathOverflow)`.**
This is why every operation can be proved total with no overflow side conditions. It is also why no
`= ok` property here can be hiding a wrapping execution: there is no wrapping arithmetic anywhere in
the verified core.

**The hand-modelled type: `PodU64 → u64` (E1).** The harness cannot enforce an overflow profile on
a hand model, so I checked the agent's claim by reading. `PodU64(pub [u8; 8])` has no arithmetic:
its only operations are `From<u64>` (`to_le_bytes`) and `From<PodU64> for u64` (`from_le_bytes`)
(`spl-pod-0.2.5/src/primitives.rs:50-64,107-108`). Every arithmetic step in the original converts
to `u64` first and back after (`programs/vault/src/state.rs:107-146`). So the swap has **no overflow
behaviour to match**: it neither aborts nor wraps, and the `u64` arithmetic that does happen is
translated from the crate itself. E2 (`then_some` → `if`) involves no arithmetic. E3 (`try_into` →
range guard) reproduces `u64::try_from(u128)` exactly: it fails iff `q > u64::MAX`, as tabulated in
§4.3. **No hand-modelled overflow gap.**

---

## 4.6 Operation by operation: what the proved property means for the real code

**Deposits (`vault_deposit_assets`, `_with_fee`, `_exact`).** The Lean bodies
(`CertoraVault.lean:499-538`, `:394-458`, `:201-249`) follow the Rust line for line: convert, then
`mint_shares`, then `add_token` (`programs/vault/src/operations/vault_operations.rs:14-28`,
`vault_fee_operations.rs:5-20`, `vault_exact_deposit_operations.rs:5-17`). The theorems say that a
successful deposit mints exactly `floor(amount·shares/assets)` shares (or `amount` at a 1:1 ratio),
books exactly the amount the effect reports, and leaves `shares ≤ assets`. Why solvency survives:
from `shares ≤ assets`, `floor(t·s/a) ≤ t` (`mul_div_le_self_of_le`, `VaultLemmas.lean:371-376`),
so assets grow at least as fast as shares. For the exact deposit there is the additional bound
`y ≤ assets_to_vault ≤ t` (`le_mul_div_of_le`, `round_trip_le`, `VaultLemmas.lean:379-389`). This
means the user is never charged more than they offered and never receives more shares than the
assets booked. **Not covered:** deposits into an already-insolvent vault (F5). The deposit code does
no solvency check, and `vault_deposit_assets_with_fee_ok` (`Spec.lean:183`) does not conclude
`Solvent` at all.

**`vault_deposit_assets_with_fee_exact` (F1).** The Lean body (`CertoraVault.lean:324-389`) returns
the input `vault` unmodified on **every** path, at `:359, :365, :371, :377, :383, :389`. That
matches the Rust, which never calls `mint_shares` or `add_token`
(`vault_exact_deposit_operations.rs:19-39`), and the theorem `vault_deposit_assets_with_fee_exact_spec`
proves `r.2 = v`. The processor, however, then **transfers `assets_to_vault` into the vault's token
account, transfers `assets_to_fee` to the fee account, and mints `shares_to_user` SPL shares**
(`processor/deposit_with_fee_exact.rs:38-62`). Compare the non-exact fee path, which does update the
vault (`vault_fee_operations.rs:11-12`). Consequences, which follow from reading and are not proved:
the SPL share supply grows while `Vault.shares` does not, and the vault's token balance grows while
`Vault.assets` does not. The permissionless `process_update_reward` sets `Vault.assets` from the
token-account balance (`processor/update_reward.rs:18-22`), so it would later book the deposit as a
"reward" to the existing shareholders. Meanwhile `Vault.shares` undercounts the outstanding shares,
so the last redeemers can hit `vault_redeem_shares_too_many` (`n > Vault.shares`, `Spec.lean:542`).
The solvency theorem for this operation (`Spec.lean:424`) is **true only because nothing changes**,
and it says nothing about the real token flows. INFER recorded this as an edge case ("mutates
nothing even though its processor transfers assets and mints shares") rather than a defect. **I
report it as a candidate defect for the owner to decide.**

**`vault_redeem_shares`.** The Lean (`CertoraVault.lean:612-651`) is convert, then `burn_shares`,
then `del_token`, matching `vault_operations.rs:30-40`. `vault_redeem_shares_ok` (`Spec.lean:441`)
needs **no** solvency hypothesis. Its `Solvent r.2` on `Ok` comes from `del_token` running
`check_invariant` on the post-write vault (`CertoraVault.lean:573`). So a successful redeem is solvent
because the program checks. Liveness (`vault_redeem_shares_succeeds`, `Spec.lean:486`) is the
non-trivial direction: on a solvent vault, redeeming `n ≤ shares` **never trips** that check and
never overflows. The arithmetic fact behind it is `n·a/s + s ≤ a + n` for `n ≤ s ≤ a`
(`redeem_solvent`, `VaultLemmas.lean:400-409`): floor rounding is always in the vault's favour. The
one excluded corner (`shares = 0 < assets`) really does fail, and `vault_redeem_shares_zero_shares`
proves it (see §3 N2).

**`vault_update_reward`.** The Lean (`CertoraVault.lean:656-691`) matches `vault_operations.rs:42-52`,
including the `reward > 0` skip at `:667`/`:682-685`. `checked_sub(new_amt, assets)` fails iff
`new_amt < assets`, and in that case the theorem says `Err(MathOverflow)` with the vault unchanged.
Otherwise `assets' = new_amt`. Reward can only increase `assets`, so solvency holds in every outcome.

**`vault_process_slash`.** The Lean (`CertoraVault.lean:696-713`) is a single `del_token`. The theorem
gives an exact characterisation: `Ok` ⇔ `shares + a ≤ assets` (`Spec.lean:673-685`). It also pins
the `Err` sub-cases: `a > assets` gives `MathOverflow` with no write (`:704`), and
`assets < shares + a` gives `GuardFail` **after** the write (`:722`).

**`vault_collect_fee`.** The Lean (`CertoraVault.lean:476-485`) reads `fee_amount`, zeroes it, and
returns it as `assets_to_user`. There is no failure path, and the theorem has `.Err _ => False`
(`Spec.lean:768`). F6 notes a latent oddity in its processor that is outside the proof scope.

---

## 4.7 What "proved" does **not** mean here

- It does not cover the **processors, loaders, SPL CPIs, signer/owner checks, PDA derivation, or
  `Vault::validate`**. None were translated (§2). F1 and F6 are exactly the kind of issue that lives
  at that boundary.
- It does not prove the **extraction crate equals the original**. That rests on differential tests
  (F4).
- It does not prove anything about the **real constructor** (F3), or about deposits from an
  **insolvent** vault (F5).
- The `_ok` theorems put `True` on `Err`, so they say **nothing about the in-memory state after a
  failed deposit or redeem**. Those states are covered only through `committed`, i.e. through the
  Solana revert (F2).


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


# 6. Bottom line

## Open obligations

**None.** `sorry_remaining = 0`, `axioms.tainted = []`, `axioms.refutations = []`. No theorem is
waiting on a proof.

## Refuted properties

**None.** No counterexample was found to any stated property.

## Ungrounded projections (rung 3)

**None.** The single INFER anchor, `is_solvent`, is grounded by the established bridge
`is_solvent_spec` (`verification/lean/CertoraVault/Spec/Spec.lean:58`), and §4.2 confirms by
reading that the projection `shares.val ≤ assets.val` is term-for-term what the translated getter
computes (`verification/lean/CertoraVault.lean:543-546`, `Std/Scalar/Core.lean:897`). The result
formulas (`assetsToShares`, `sharesToAssets`, `feeOf`) appear only as equations on the outputs of
the real functions, so the theorems themselves are their bridges. §4.3 checked them term by term,
including argument order, `Nat` division by zero and `Nat` subtraction.

## Discrepancies and gaps for the owner to weigh

1. **Candidate defect: `vault_deposit_assets_with_fee_exact` does not update vault accounting.**
   It is *proved* to leave `Vault` unchanged in every outcome (`vault_deposit_assets_with_fee_exact_spec`,
   `Spec.lean:361-371`). Its processor still transfers assets into the vault and mints shares
   (`programs/vault/src/processor/deposit_with_fee_exact.rs:38-62`), while the sibling
   `vault_deposit_assets_with_fee` does call `mint_shares`/`add_token`
   (`programs/vault/src/operations/vault_fee_operations.rs:11-12`). By reading, this lets SPL share
   supply and the vault token balance drift away from `Vault.shares`/`Vault.assets`. The
   permissionless `update_reward` would then book the deposit as reward to other holders. The
   solvency theorem for this path is true only because nothing changes. **Suggested next step:**
   decide whether this is intended. If not, add `vault.mint_shares(shares_to_user)?;
   vault.add_token(assets_to_vault)?;` (mirroring `vault_deposit_assets_exact`) and re-run the
   pipeline. The existing `…_with_fee_exact_spec` would then fail, which is expected, and should be
   restated like `vault_deposit_assets_exact_ok`.
2. **Modelling assumption: revert on `Err`.** Five solvency theorems rely on `committed`, meaning
   the account is restored when an operation returns `Err` (`Spec.lean:47-51`). This is Solana
   semantics and is checked only by reading the processors, which all propagate with `?` (§4.4). It
   is not kernel-checked.
3. **Translation trust: extraction crate.** The model is of `verification/translate/extract`, with
   a 4-field `Vault` and edits E1 (`PodU64`→`u64`), E2 (`then_some`→`if`) and E3
   (`try_into`→range guard). The evidence that it matches the original is the differential test
   suite (5/5 reported passing, not re-run by me). It is not proved.
4. **Coverage limits.** Deposits from an insolvent vault are not covered. The deposit code does not
   check solvency, and the `_ok` theorems assume it. The real constructor is not covered:
   `default_vault_solvent` is surface-only. Processors, loaders, CPIs and account validation are out
   of scope.
5. **Latent, out of scope:** `fee_amount` is never increased by any operation (proved), yet
   `process_collect_fee` pays it out of the vault's assets token account without decrementing
   `Vault.assets` (`processor/collect_fee.rs:16-22`). It is harmless while `fee_amount = 0`. Worth
   a look if fee accrual is ever added.
6. **Repository hygiene:** the pipeline deleted the root `rust-toolchain.toml` (Rust 1.79 pin) in
   commit `59413bb` (`translate/campaigns/Spec/verdict.json` → `source_files_changed`). This has no
   effect on the proofs, but it changes the real build. **Restore it.**

## Overall verdict

For the eight vault operations and `is_solvent`, **28 theorems verify the implementation**
unconditionally: each runs the Aeneas translation of the real Rust and depends only on Lean's
standard axioms, with no declared assumptions, no opaque stubs, no `sorry` and no refutation. One
further theorem is surface-only. Taken together, they establish that **every operation is
panic-free; every successful operation from a solvent vault leaves `shares ≤ assets`, as judged by
the program's own `is_solvent`; and redeem and slash enforce solvency on success from any state.**
They also give exact closed forms for every amount minted, burned, booked and paid, plus precise
error conditions. Overflow is handled by explicit `checked_*` calls that return `Err`, so the
result does not depend on the overflow profile. The deployed `overflow-checks = true` profile was
reproduced anyway.

Three caveats limit what this means for the deployed program. It holds modulo the Charon/Aeneas
toolchain and the differentially tested (not proved) extraction crate. Solvency on failed calls
assumes Solana's revert. And one result describes behaviour the owner should probably not want:
the fee-exact deposit leaves the vault's books untouched while its processor moves tokens and mints
shares. **The proofs are faithful to the code. On that path the code, or its processor, appears to
be wrong.**

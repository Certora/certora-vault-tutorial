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

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

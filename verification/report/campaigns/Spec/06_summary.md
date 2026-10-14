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

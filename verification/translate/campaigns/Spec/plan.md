# TRANSLATE plan — campaign `Spec` (certora_vault)

## Targets (each must be a real Lean `def`)
vault_deposit_assets, vault_redeem_shares, vault_update_reward, vault_process_slash
(operations::vault_operations); vault_deposit_assets_with_fee, vault_collect_fee
(operations::vault_fee_operations); vault_deposit_assets_exact,
vault_deposit_assets_with_fee_exact (operations::vault_exact_deposit_operations); Vault::is_solvent (state).

## Strategy: EXTRACTION
In-place would drag in `Vault`'s 7 `Pubkey` fields + bumps, `bytemuck::Pod`, `spl_pod::PodU64`
(byte-array LE repr + From impls), `solana_program` (`declare_id!`, `ProgramError`), the
loaders/processors and derived `Debug` — all outside `relevant_state`. So: a standalone crate
`/workspace/out/translate/extract` (package/lib `certora_vault`, own `[workspace]`) holding ONLY:

* `state::Vault { shares, assets, fee_bps, fee_amount : u64 }` — projected (Pubkeys/bumps dropped);
  `PodU64` → `u64` (rung-3 representation swap: PodU64 is a LE `[u8;8]` read/written only via
  `u64::from`/`.into()`, so the observable value is the u64).
  Methods kept: num_shares, num_assets, fee_amount, clear_fee_amount, fee_in_bps, is_solvent,
  check_invariant, convert_shares_to_assets, convert_assets_to_shares, burn_shares, mint_shares,
  add_token, del_token. (Dropped: new, gross_assets, *_ceil, validate, PDA helpers — unreachable.)
* `operations::{vault_operations, vault_fee_operations, vault_exact_deposit_operations}` — target
  bodies verbatim + `VaultEffect` (5 u64 fields, derive Default only).
* `utils::math::{mul_div_floor, FeeBps, GrossAmount, ONE_IN_BPS}` + `TryFrom<u64> for FeeBps`.
* `errors::VaultError` (4 variants, no Debug/PartialEq in the translated build; `From<VaultError> for
  ProgramError` dropped — ProgramError is solana surface), `VaultResult`.
* guards: `require_gt!/require_le!` macros (expand to plain if/return).

## Dependency classification (single pass)
* translatable core: all of the above.
* opaque_boundary: solana_program, spl_pod, bytemuck, spl_token, loaders/processors — ABSENT.
* leaves: Debug/Display — ABSENT (only derived under `cfg(test)`).
* must_model: PodU64 → u64 (value-only, see above). No other modelling needed.

## Expected Aeneas friction → behaviour-preserving rewrites (only if they actually break)
* `Option::ok_or`, `map_err(closure)`, `try_into` u128→u64, `bool::then_some` → explicit `match`/`if`.
* `?` on Result — keep if Aeneas accepts, else explicit match.

## Overflow posture
Workspace `[profile.release] overflow-checks = true` (lto='thin'); no per-package overrides.
Copy into the extraction manifest; charon with `-- --release`. Confirm `panic.+`-style / checked
ops (the code uses checked_* explicitly anyway).

## Equivalence evidence
`cargo test` in the extraction crate with dev-dependency on the ORIGINAL crate
(`certora_vault_tutorial`, path dep) — differential tests running every target on both
implementations over a grid of representative + edge values (0, 1, u64::MAX, shares==assets,
shares==0, fee_bps 0/500/10000/10001, overflow), comparing result (Ok effect / Err variant) and
the post-state of the 4 kept fields.

## Pipeline
charon cargo --preset=aeneas -- --release  (in extract/)  → certora_vault.llbc
rm -rf /workspace/out/lean/* ; aeneas -backend lean -dest /workspace/out/lean certora_vault.llbc
lake env lean CertoraVault.lean

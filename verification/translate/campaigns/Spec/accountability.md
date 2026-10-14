# TRANSLATE accountability — campaign `Spec` (certora_vault)

## Decision
No prior translation existed in `/workspace/out/lean`. This campaign **created** the shared translation
`lean/CertoraVault.lean` (single module, no `-split-files`) from a **standalone extraction crate**
at `/workspace/out/translate/extract` (= `/workspace/repo/verification/translate/extract`; package and
lib `certora_vault`, its own `[workspace]`). The original program (`programs/vault`) is **not
modified** (`git status` shows only new files under `verification/`).

Result: all 9 targets are real `def`s with bodies; the file contains **0 `axiom`s and 0 `sorry`s**;
`lake env lean CertoraVault.lean` (run from the harness's lake project at `/opt/lean-template`, since
`/workspace/out/lean` holds no lakefile yet) exits 0 with no errors.

| target pattern | Lean def |
| --- | --- |
| vault_operations::vault_deposit_assets | `operations.vault_operations.vault_deposit_assets` |
| vault_operations::vault_redeem_shares | `operations.vault_operations.vault_redeem_shares` |
| vault_operations::vault_update_reward | `operations.vault_operations.vault_update_reward` |
| vault_operations::vault_process_slash | `operations.vault_operations.vault_process_slash` |
| vault_fee_operations::vault_deposit_assets_with_fee | `operations.vault_fee_operations.vault_deposit_assets_with_fee` |
| vault_fee_operations::vault_collect_fee | `operations.vault_fee_operations.vault_collect_fee` |
| vault_exact_deposit_operations::vault_deposit_assets_exact | `operations.vault_exact_deposit_operations.vault_deposit_assets_exact` |
| vault_exact_deposit_operations::vault_deposit_assets_with_fee_exact | `operations.vault_exact_deposit_operations.vault_deposit_assets_with_fee_exact` |
| state::_::is_solvent | `state.Vault.is_solvent` |

Pipeline (from `translate/extract`):
```
charon cargo --preset=aeneas -- --release          # -> certora_vault.llbc
rm -rf /workspace/out/lean/* && aeneas -backend lean -dest /workspace/out/lean certora_vault.llbc
```
No `--opaque` / `--exclude` flags were needed (nothing outside the core is in scope).

## What the extraction crate contains (and why it is the same program)
Copied **verbatim** from `programs/vault/src`:
* `operations/vault_operations.rs` (minus its `#[cfg(test)]` module, which uses `Pubkey`),
  `operations/vault_fee_operations.rs`, `operations/vault_exact_deposit_operations.rs`,
  `operations/mod.rs`, `utils/guards.rs` (the `require_*!` macros), `utils/mod.rs`.
* `utils/math.rs`: `mul_div_floor`, `GrossAmount`, `ONE_IN_BPS`, `FeeBps`, `TryFrom<u64> for FeeBps`,
  `FeeBps::apply` (verbatim except the `try_into` edit below). Dropped (unreachable from targets):
  `mul_div_ceil`, `FeeBps::is_zero`, `FeeBps::apply_ceil`.
* `state.rs`: `Vault` methods num_shares, num_assets, fee_amount, clear_fee_amount, fee_in_bps,
  is_solvent, check_invariant, convert_shares_to_assets, convert_assets_to_shares, burn_shares,
  mint_shares, add_token, del_token — bodies verbatim except the two edits below and dropping the
  `.into()` PodU64 conversions. Dropped (unreachable): `new`, `gross_assets`, `*_ceil` conversions,
  `validate`, the PDA seed macros / `create_*_address` helpers.
* `errors.rs`: `VaultError` (same 4 variants, same order), `VaultResult`.

## Projections / exclusions (out of scope — ABSENT, not opaqued)
1. **`Vault` projected** to `shares, assets, fee_bps, fee_amount` (= `relevant_state`). Dropped:
   `admin, slash_admin, shares_mint, assets_mint, fee_token_account, vault_assets_account` (Pubkeys),
   `vault_shares_mint_bump, vault_assets_account_bump` (u8). No target reads or writes them
   (only `validate`/PDA helpers/loaders do, none of which are targets). `#[repr(C)]`, `Pod`,
   `Zeroable`, `Copy`, `Clone` derives dropped (byte-layout plumbing). `Default` is derived only
   under `cfg(test)` (used by the equivalence test `solvency_boundary`); no target calls it
   (`Vault::new`, its only Rust caller, is dropped), so it never reaches Charon and
   `state.Vault.Insts.CoreDefaultDefault` is absent from the Lean. (Fix after TRANSLATE-JUDGE
   `irrelevant_surface`; charon+aeneas re-run, equivalence tests still 5/5.)
2. **`VaultEffect`**: same 5 u64 fields; derives reduced to `Default` (the only one the targets use,
   via `..Default::default()`); `Debug`/`PartialEq` only under `cfg(test)` so they never reach Charon.
3. **`VaultError`**: `Debug`/`PartialEq` derives only under `cfg(test)`; `impl From<VaultError> for
   ProgramError` dropped (solana surface, used only by processors).
4. **Absent entirely**: `solana_program` (`declare_id!`, `Pubkey`, `ProgramError`), `spl_pod`,
   `bytemuck`, `spl_token`, `num_enum`, `loaders/`, `processor/`, `instruction`, `constants`.

## Rung-3 behaviour-preserving edits (with evidence)
* **E1 — `PodU64` → `u64`** for `Vault.shares/assets/fee_bps/fee_amount`. `PodU64` is a
  little-endian `[u8; 8]` wrapper whose only use in the program is `u64::from(pod)` / `x.into()`
  round-tripping a u64; the observable value is that u64. Accessors become `self.shares` etc. and
  writes drop the trailing `.into()`.
* **E2 — `Vault::check_invariant`**: `self.is_solvent().then_some(()).ok_or(VaultError::GuardFail)`
  → `if self.is_solvent() { Ok(()) } else { Err(VaultError::GuardFail) }`.
  Reason: `core::bool::then_some` is not in Aeneas's builtin registry → emitted as an `axiom`
  (seen in the first Aeneas run), which would taint `del_token` → `vault_redeem_shares` /
  `vault_process_slash`.
* **E3 — `mul_div_floor`**: the trailing `.try_into().map_err(|_| VaultError::MathOverflow)` (u128→u64)
  → `if q > u64::MAX as u128 { Err(VaultError::MathOverflow) } else { Ok(q as u64) }`.
  Reason: `TryFrom<u128> for u64` is not modelled (emitted as an `axiom` in the first run). The
  `checked_mul`/`checked_div`/`ok_or`/`?` part is unchanged (all modelled). `try_from` fails exactly
  when `q > u64::MAX`, and `q as u64` is exact otherwise.

**Behavioural-equivalence tests** — `translate/extract/src/equivalence_tests.rs`, run with
`cargo test --release` in `translate/extract` (dev-dependency on the ORIGINAL crate
`certora_vault_tutorial`, renamed `original`): **5 passed, 0 failed.** Each test runs our
extraction and the original (with real `PodU64` + Pubkey-laden `Vault`) on identical inputs and
compares the full outcome — `Ok(effect)` (all 5 fields) or `Err(variant)` — **and** the post-state
of the four projected fields, including on the `Err` path (so partial in-place mutation agrees):
* `ops_agree_on_edge_grid` — exhaustive cross product of shares × assets × amount over
  {0,1,2,3,7,100,101,9999,10000,10001,2^32,MAX/2,MAX/2+1,MAX−1,MAX}, fee_bps over
  {0,1,500,5000,9999,10000,10001,MAX}, fee_amount over {0,5,MAX}; all 8 targets + `is_solvent` +
  `check_invariant` (E1, E2, E3). Covers shares==assets (incl. 0==0 identity path), shares==0 with
  assets>0 (div-by-zero), insolvent states, amount 0 (GuardFail), u64 add overflow, quotient > u64::MAX,
  fee_bps = 10000 / 10001.
* `ops_agree_on_random_inputs` — 300 000 deterministic xorshift cases mixing tiny/small/u32/near-MAX/
  full-range values, biased toward solvent vaults.
* `mul_div_floor_agrees` — E3 directly vs the original `mul_div_floor` on the 15³ edge grid + 300 000
  random triples, plus pinned boundaries: quotient exactly `u64::MAX` (Ok), `u64::MAX+1` (Err),
  `2^64` (Err), divisor 0 (Err).
* `fee_bps_agrees` — `FeeBps::try_from` guard and `apply` (floor fee, net) vs original.
* `solvency_boundary` — zero vault (also via the test-only `Default`) solvent; shares = assets, ±1, MAX boundary (E1, E2).

## Overflow posture
* Deployed: workspace `/workspace/repo/Cargo.toml` `[profile.release] overflow-checks = true`
  (`lto = 'thin'`); no per-package overrides; no fixed-point/bignum dependency.
* Extraction manifest: that `[profile.release]` block copied verbatim; Charon run with `-- --release`.
* Confirmation: `charon pretty-print certora_vault.llbc` contains **no** bare `+`/`-`/`*` binop
  (0 `panic.`/`wrap.` ops) — every arithmetic step in the core is an explicit
  `u64::checked_add/checked_sub`, `u128::checked_mul/checked_div` (returning `Option`, modelled
  in Lean as `U64.checked_add` etc. → `Option`), plus the lossless `u64→u128` cast and the
  range-guarded `u128→u64` cast. Hence the model's arithmetic is identical under either posture; the
  checked release profile is reproduced anyway.

## Notes for downstream
* `lean/CertoraVault.lean` imports only `Aeneas`. Cold-cache `lake env lean` takes ~10+ min of wall
  time on this box purely from I/O (user CPU ≈ 3.5 s); it is not elaboration cost.
* `translate/extract/target/` is git-ignored (build output).

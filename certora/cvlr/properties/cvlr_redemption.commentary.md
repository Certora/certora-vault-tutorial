## What this harness drives

Seventeen properties of the Redemption component of `certora_vault`, a **native** (non-Anchor)
Solana program. Fifteen rules drive `crate::processor::process_redeem_shares` directly with a
`Box`ed array of nondeterministic accounts; two drive the vault accounting core
(`crate::operations::vault_redeem_shares`, `vault_deposit_assets`); two drive the program's SPL
instruction builders. Nineteen rules VERIFIED; three are marked expected-to-fail and are real
findings. Nothing was skipped.

Prover conf: `loop_iter` was raised from 2 to 4 (sound — `optimistic_loop` stays off). At 2, thirteen
rules stopped on the unwinding assertion in the fixed-trip-count loops of `spl_token::instruction::*`
and the CPI account arrays; nothing nondeterministic determines those counts, so constraining inputs
had nothing to constrain. At 4 every loop discharges and no unwinding violation remains.

## Program munges (two, both gated on `certora` + `unit_redemption`)

The Prover treats `solana_program::program::{invoke, invoke_signed}` as **external**: they write
nothing and return an arbitrary result. Measured directly — across a full successful
`process_redeem_shares` every token-account amount and both mint supplies were byte-identical
(`user_shares_tok_before = 1697` -> `1697` with `amount = 1`, while `vault.assets` correctly went
802 -> 801). A `swap_import` on those two names was refused (they sit inside a nested
`use solana_program::{...}` tree, which the editor's charter cannot rewrite a single leaf of).

Instead the editor applied two `extract_function` munges to `processor/spl_token_utils.rs`, each
replacement also carrying `cvlr::mock_fn`:

* `spl_burn_shares` and `spl_transfer_assets_from_vault` are replaced, under this unit's feature, by
  recording stand-ins in this module. They record the amount, the target program account and the
  keys the handler routed into each account parameter, and return `Ok(())`.
* The `spl_token::instruction::{burn,transfer}` construction each wrapper performed was lifted
  verbatim into `build_burn_shares_ix` / `build_transfer_assets_from_vault_ix`, which two rules call
  directly so the instruction's account layout is still checked.

The deployed arm is byte-identical to what the project ships.

## (C2) CPI fidelity — what no rule here proves

Nothing in this unit claims anything about shares actually burned, assets actually moved, or any
balance on either side of the transfer; the Prover does not model the CPI and the stand-ins do not
perform it. The transfer stand-in also discards `vault_assets_signer_seeds`, so **the signer seeds
the handler signs the asset transfer with are not checked by any rule in this unit.** Properties 3,
11 and 17 are stated over the *request* — what the handler asked the token program to do, and what
instruction the program built — not over its effect. The stand-ins' unconditional `Ok(())` removes
failure paths only, so every "success implies ..." rule is checked on a superset of the real
successful executions.

## (C3) The P11/P17 chain has one unverified joint

Property 17 is established by two rules in composition: `rule_redeem_cpi_accounts_and_direction`
shows the handler routes `vault_assets_account`/`user_assets_account` (and
`user_shares_account`/`shares_mint`/`authority`) into the wrapper's account parameters in that
order; `rule_redeem_transfer_ix_builder` / `rule_redeem_burn_ix_builder` show the builder puts its
n-th account parameter into the instruction's n-th meta (source, destination, authority — never the
reverse), targeting the token program it was handed. The line that joins them — the wrapper
forwarding its parameters to `build_*_ix` — lives in the wrapper body, which `mock_fn` replaces in
the verified build, so no rule executes it; the composition rests on reading the munged source.
Also note `cvlr_assert!(xfer.k2 == *accounts[2].key)` records a wrapper parameter (`_mint`) that the
SPL `Transfer` instruction does not carry, so it is a claim about the handler's routing, not about
the CPI.

## (C1) Narrowed subjects

* **P13** (`rule_redeem_reachable_for_legitimate_user`) drives `crate::operations::vault_redeem_shares`.
  The handler-level formulation **timed out**. What is proved is the substance of the property: the
  accounting core — the `checked_sub`s in `burn_shares`/`del_token` and the post-decrement
  `check_invariant` inside `del_token` — never rejects a redemption with `0 < amount <= shares` on a
  solvent vault. It is *not* "the instruction succeeds": `RedeemSharesContext::load`, the
  `Signer`/`SplTokenProgramInfo`/`VaultInfo` loaders, the three `require_eq!` checks, the
  self-transfer guard and both CPIs are outside this rule.
* **P14** (`rule_deposit_then_redeem_round_trip_no_profit`) drives `vault_deposit_assets` then
  `vault_redeem_shares` on one `Vault`. `vault_deposit_assets` is `process_deposit`'s only vault
  mutation; `process_deposit_exact` and the two fee-bearing deposit handlers are **not** covered by
  this round-trip result, and the same account-layer exclusions as P13 apply.

## (C4) The two PDA findings, and what their green companions do not mean

`Pubkey::create_program_address` is summarized by the Prover to an unconstrained address, so the
VIOLATED verdicts on `rule_redeem_vault_assets_account_is_canonical_pda` and
`rule_redeem_shares_mint_is_canonical_pda` are **not by themselves evidence** of the defect. The
finding is structural and readable in `loaders/vault_loaders.rs`: `RedeemSharesContext::validate`
performs exactly three `require_eq!` against fields *inside the vault account's own bytes* and never
calls `VaultAssetsAccount::check_vault` or `VaultSharesMint::check_vault`, unlike
`DepositContext::validate` and `SlashContext::validate`; `RedeemSharesContext` does not even hold a
`VaultAssetsAccount`. The final run's `key_is_canonical_pda: '0'` logs confirm the comparison is
genuinely false on an accepted execution. The green companions
(`rule_redeem_vault_assets_account_matches_vault_record`,
`rule_redeem_shares_mint_matches_vault_record`) establish only equality against the vault's own
recorded field and **must not be read as satisfying properties 8 and 9** — they are published to show
exactly where the gap sits.

The third finding, `rule_redeem_requires_vault_info_owned_by_program`, needs no such caveat:
`VaultInfo::try_from` carries the comments "// owned by vault program" and "// has discriminant" and
implements neither, no line on the path reads `vault_info.owner`, and the counterexample's derived
boolean `owner_is_program: '0'` is unambiguous.

## (C5) Scope of the green accounting results

P1-P5, P12, P15 and P16 are proved about the `Vault` held in whatever account was passed as
`vault_info`. Because of the P7/P8/P9 findings a successful execution need not involve the program's
real vault, so these should be read as **"the handler's accounting is internally correct on the state
it was given"**, not as "the on-chain vault is always solvent".

## (C6) Modelling assumptions a reader should see

* `Vault::shares`/`assets` are `PodU64`. Under `-solanaTACMathInt true` the Prover's model of a
  byte-level read does **not** re-impose that width — early counterexamples carried stored totals of
  2^64 .. 2^256 and triggered `Imprecision detected` notes on the division. Every arithmetic rule
  therefore re-states the width with `cvlr_assume!(NativeInt::from(x).is_u64())` on the shares/assets
  read before and after the call. This excludes only states the on-chain type cannot represent.
* P2 is stated multiplicatively (`payout * s0 <= amount * a0 < (payout + 1) * s0`) rather than with a
  symbolic division, which is what made it decidable.
* `cvlr_assume!(s0 == a0 || s0 != 0)` in P2/P4/P15 is implied by success — `mul_div_floor`'s
  `checked_div` errors on a zero divisor — and is stated up front only so the solver never builds a
  division-by-zero term.
* The nonlinear step in P13/P14 (`mul_div_floor` against the post-decrement invariant) is discharged
  **inline** in those rules; there is no separately verified `cvlr_lemma!`, so a reader has no
  isolated algebraic obligation to check on its own.
* Rules that build the 16-account array end with `core::mem::forget(accounts)` so the array's drop
  loop cannot make the vacuity check unreachable.
* The Prover reported `alloc::raw_vec::RawVec::grow_one` and two `drop_in_place` shims as external.
  No rule's property depends on what those do.

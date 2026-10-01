## What this harness proves, and what it assumes

35 rules over the Standard Deposits component of `certora_vault`. All 18 properties are formalized;
nothing is skipped. Eleven rules are marked expected-to-fail and record genuine defects in the
shipped program (details at the end).

### Shape of the harness

Every handler/operation call is made through a `macro_rules!` expansion so the call is a **direct**
call to a named program function. An earlier draft routed handlers through `fn` pointers; the prover
runs with `-solanaSkipCallRegInst true`, which skips indirect (register) calls, and several rules
then reported verdicts about an execution in which the handler never ran — including a VERIFIED
verdict on a property that is mathematically false. Any future edit to this module must preserve the
direct-call shape.

Handler-level rules build `cvlr_deserialize_nondet_accounts()` on the heap, pin the `data_len()` of
the five accounts the program parses (vault, both asset token accounts, the shares mint, the user
shares account), call the handler, and read the vault back through `bytemuck::from_bytes` on the same
live account bytes the handler wrote through `VaultInfo::get_mut` — the state is zero-copy, so no
write-back step is involved. Every rule ends with `core::mem::forget(accounts)`.

### The two `mock_fn` munges, and what they cost

`crate::processor::spl_token_utils::spl_transfer_assets_from_user` and `spl_mint_shares` are replaced
by stand-ins in this module. Each records the requested amount and the account keys into `static mut`
ghost state, then applies `cvlr-solana`'s own SPL Token state transition (`spl_token_transfer`,
`spl_mint_to`) so the token account `amount` and the mint `supply` really move. Without this the
Prover's CPI model moves nothing and Properties 5, 6, 16 and 18 would be hollow.

**Token-balance verdicts rest on these stand-ins.** This applies to `rule_cpi_token_effects_*`,
`rule_backed_by_token_balance_*`, `rule_shares_match_mint_supply_*`, `rule_stale_balance_pricing_*`,
`rule_exact_deposit_never_takes_more_than_requested` and `rule_deposit_must_reject_self_transfer`.
What the real wrapper bodies did is outside the proof:

* the SPL instruction construction and the real `invoke` / `invoke_signed` are gone;
* the *direction* of each transfer and mint is fixed by the stand-in from the wrapper's parameter
  positions — the program's own instruction-building code is not analyzed;
* the shares-mint PDA signer seeds are accepted and ignored, so **which PDA signs the mint is not
  proved**;
* all callee-side SPL Token checks are unmodelled: authority/delegate ownership of the source,
  mint–account agreement for the destination, and sufficient source balance (which the model
  *assumes* rather than branches on).

`cvlr-solana`'s transfer model is a documented no-op when source and destination keys are equal,
which is exactly what the real Token program does; this is what makes the self-transfer
counterexamples real rather than modelling artefacts.

### Per-property scope notes

**Property 5 is split.** `rule_cpi_token_effects_*` observes the amounts that actually moved between
the four named token accounts and is conditional on `user_assets_account != vault_assets_account`.
`rule_cpi_arguments_*` carries the property's literal claim — the amount *requested* in each CPI, and
all the named keys including the signing `authority`, which the token model itself ignores — with no
exclusion.

**Property 6's scope differs per handler.** `rule_backed_by_token_balance_deposit` is conditional on
non-self-transfer. `rule_backed_by_token_balance_deposit_exact` is unconditional, because
`process_deposit_exact` enforces `require_ne!` itself and the assumption is redundant there.
`rule_backed_by_token_balance_self_transfer_deposit` records that the **unconditional** claim is false
for `process_deposit`.

**Property 10 is validation-level.** `rule_reject_foreign_owned_vault` drives
`crate::loaders::DepositContext::load`, the shared first statement of both handlers, and shows it
*accepting* a foreign-owned `vault_info` — not a completed exploit transaction. The acceptance is
reached with `Pubkey::create_program_address` summarized, so the two PDA re-derivation checks inside
`check_vault` are satisfied against an unconstrained address; whether an attacker can also furnish
accounts at the derived PDAs is not settled by this rule. What carries the finding is the source-level
fact: `VaultInfo::try_from` contains the comments `// owned by vault program` and `// has
discriminant` with neither check written, and nothing downstream in either handler reads
`vault_info.owner`. The handler-level form of this rule did not terminate within the solver budget.

**Narrowed subjects.** `rule_no_fee_bypass_*`, `rule_legitimate_deposit_is_reachable`,
`rule_*_not_dead_when_shares_zero` and `rule_deposit_*_then_redeem_not_profitable` drive
`crate::operations::…` functions rather than the handlers. Account validation, the signer check, the
self-transfer guard and both token CPIs are **not executed** by these rules. Each was narrowed for a
stated reason: for Properties 12 and 14 the computation the property is about (the `fee_bps` read that
does not happen; `convert_assets_to_shares` and the zero-share rejection) lives entirely in the
operations layer, and the handler-level form of the fee rules did not terminate; for Property 17 the
redeem handler takes the same accounts in a different order and nothing in the property is about
account validation.

**Property 17's pre-state is assumed, not proved here:** `shares <= assets` (the program's own
`Vault::is_solvent`, maintained by `add_token`/`del_token`), `shares > 0`, `assets > 0`, and two `u64`
ceilings restating the `checked_add`s in `mint_shares`/`add_token`. Without them the solver explores
post-states above `u64::MAX` that the `PodU64` fields cannot hold and reports an imprecise division as
a counterexample. The rule also speaks only about executions in which the subsequent redeem itself
succeeds.

**Property 13's handler half is existential.** `rule_legitimate_deposit_handler_path_live` is a
`cvlr_satisfy!` witness under `shares == assets` and distinct asset accounts; it does not `.unwrap()`
the handler. A *universal* handler-level acceptance claim is not statable here, because the build
summarizes `Pubkey::create_program_address` and the two PDA constraints therefore compare against an
unconstrained address that may refuse (or accept) any account. The universal half of the property is
carried by `rule_legitimate_deposit_is_reachable` at the accounting core.

**Property 4's witness is branch-restricted.** `rule_deposit_must_reject_self_transfer` narrows to a
signing authority, a positive amount, and the `shares == assets` 1:1 pricing branch; each narrowing
only removes executions, so none can manufacture the violation. The missing guard is not
branch-specific, but the exhibited counterexample is.

**Property 18's rules** assume `shares_before > 0` — part of the property's own premise, since a vault
with no outstanding shares has no existing shareholders to dilute — and exclude the self-transfer
input so the rules report their own property rather than Property 4's defect. Both exclusions only
narrow rules that fail anyway.

### The eleven findings

1. **P4 / P6** `process_deposit`'s self-transfer `require_ne!` is commented out. Witness: shares =
   assets = 10342, token balance 12107, amount 1 → `Ok`, both counters rise to 10343, balance
   unchanged. `process_deposit_exact` has the identical check live.
2. **P2** `vault_deposit_assets_exact` closes its round trip with floor `convert_shares_to_assets`
   where `convert_shares_to_assets_ceil` exists: 7/10, amount 6 → 4 shares for 5 assets, backing per
   share falls from 10/7 to 15/11.
3. **P15** `vault_deposit_assets` mints `floor(amount·shares/assets)` but adds the full `amount`:
   4/9, amount 6 → 2 shares for 6 assets against a fair price of 5.
4. **P14** the bootstrap branch is keyed on `shares == assets` rather than `shares == 0`, so from
   `shares == 0, assets > 0` every amount prices at 0 shares and `mint_shares`'s `require_gt!` rejects
   — deposits permanently bricked, residual assets stranded.
5. **P12** the `require!(fee.is_zero(), …)` guard is commented out in `vault_deposit_assets` and
   absent entirely from `vault_deposit_assets_exact`; neither reads `vault.fee_bps`.
6. **P10** no owner or discriminant check on `vault_info`; every other check in
   `DepositContext::validate` reads its left-hand side out of the same attacker-controlled bytes.
7. **P18** shares are priced off the recorded `vault.assets`, never the vault token account's real
   balance, which only the permissionless `process_update_reward` reconciles. Witness: 108 shares,
   108 recorded assets, 0 fee, 123 real tokens (15 unrecognized), amount 1 → one share minted where
   the fair count against the real balance is 0.

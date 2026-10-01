## What this harness drives

`certora_vault` is a *native* Solana program (not Anchor): each handler takes `&[AccountInfo]` and
loads its accounts through a `*Context::load`. Every rule below calls one of the program's own
handlers, through `crate::processor::<handler>`; none reimplements a state transition.

**The collect-fee handler had to be mounted before it could be driven.**
`programs/vault/src/processor/collect_fee.rs` holds the handler every property in this batch is
about, but `processor/mod.rs` never declared it as a module and the file carries no `use` lines of
its own, so `crate::processor::process_collect_fee` did not resolve. A `redirect_module` munge on
`processor` re-mounts the eight developer modules unchanged (same names, same tree position, via
`#[path]`) and adds an inline `collect_fee` module that supplies the five missing imports — all of
them bindings to the program's own items — and then `include!`s the developers' file verbatim. No
handler, loader, operation or arithmetic is re-authored.

**Standing of that handler.** It follows that `process_collect_fee` is *not* compiled into the
shipped program and is not named by `CertoraVaultInstruction`; there is no dispatch to it. Further,
no handler in this snapshot ever increments `vault.fee_amount` (the fee-based deposit paths send
their fee directly to a separate `fee_token_account`). So the four defects reported below live in
presently dormant code, and each additionally requires a vault whose `fee_amount` was populated by
code not present in this snapshot. They are findings about the handler as written, not about an
instruction a user can invoke today.

## The CPI stand-ins, and what they cost

The four wrappers in `processor/spl_token_utils.rs` build a real `spl_token::instruction::*` value
whose encoder contains loops the Prover cannot discharge at `loop_iter: 2`. All four are replaced,
under `unit_fee_collection` only, by stand-ins of the same name in this module (`mock_fn`). Three
return `Ok(())`. The fourth, `spl_transfer_assets_from_vault`, performs no CPI but *records* its
arguments — amount, source key, destination key, seed count, seed prefix, seed vault key, bump — in
`static mut` scalars that each rule resets before the call.

Consequences a reader must weigh:

* Property 2's verdict is about **the arguments the handler passed to the wrapper**, not about the
  encoded `spl_token::instruction::transfer` nor about any token that moved. The wrapper's own
  mapping of those arguments onto source / destination / PDA authority was checked by inspection
  (arg 1 is the source, arg 2 the destination, and `invoke_signed` passes the supplied seeds) but
  the prover did not check it.
* The stand-ins cannot fail, so each wrapper's encoder error path is removed and
  `process_collect_fee` has one fewer way to return `Err`. No rule here rests on that route.
* The Prover does not model a CPI's effects in any case, so no property in this batch asserts over
  a real SPL token-account balance.
* The seven sibling-handler invariant rules (properties 5/6) run under the same stand-ins. Removing
  the wrappers' failure paths *widens* the set of success paths examined rather than narrowing it,
  and the assertions are only over vault state those handlers write themselves.

## How every property is stated, and why

**Every rule consumes the handler's `Result` with `.unwrap()`.** This was forced empirically: a
first submission stated the rejection properties as `cvlr_assert!(handler(..).is_err())`, and all
five rules written that way came back TIMEOUT while the twelve `.unwrap()` rules in the same job
finished. Keeping the failure path live merges the whole error-return chain back into the rule and
the solver does not finish. So each property is stated in the "success implies" direction, which is
the contrapositive of the rejection statement and is checked on exactly the executions the handler
accepts. The "reverts, leaves `fee_amount` unchanged and issues no transfer" halves of properties 1
and 5/6 then rest on the Solana runtime's rollback of a failed instruction, not on a rule.

Note also that property 1's "equals the recorded admin" is only as strong as the authenticity of the
vault account the admin was read from — which property 8 reports as unchecked.

The vault state is `bytemuck` `Pod` written directly into the account's own buffer by
`VaultInfo::get_mut`, so a post-call re-read of the account data really is the post-state; there is
no Anchor-style write-back to miss. Account `data_len()` is pinned wherever a deserializer runs:
`size_of::<Vault>()` on the vault account in every rule, and `spl_token::state::Account::LEN` on
`accounts[1]` for `process_update_reward`, the only handler that unpacks a token account.

## Caveats attaching to individual rules

* `rule_collect_fee_must_not_spend_share_backing_assets` — `balance` is a free `u64` constrained by
  the rule, not a value read from the vault assets token account. That is sound here precisely
  because the handler provably never reads the balance, so no execution is excluded by letting the
  rule choose it. What the rule demonstrates is "the payout is `fee_amount` irrespective of any
  balance". The premise's on-chain reachability — `process_update_reward` syncing `vault.assets` to
  the raw token balance, fee included — is argued from the source, not demonstrated by a rule.
* `rule_collect_fee_requires_derived_vault_assets_pda` — `Pubkey::create_program_address` is covered
  by the default Solana summaries, so the `derived` address the rule compares against is not the
  real derivation. The finding does not rest on the counterexample's address values: it rests on the
  source-level fact that `CollectFeeContext` never calls `VaultAssetsAccount::check_vault` at all,
  which `DepositContext` and `SlashContext` both do. A consequence of the summary is that the rule
  cannot distinguish "non-canonical bump" from "mis-recorded account"; it shows only that neither is
  rejected.
* `rule_collect_fee_reachable_for_admin` — the reachability shown is reachability *in the mocked
  program*: the CPI wrappers cannot fail and the PDA derivation is summarized. The rule establishes
  that `CollectFeeContext`'s own checks do not make fee collection unreachable (no dead handler,
  no permanently stuck fees); it does not establish that a real on-chain execution would succeed
  inside the token program. The satisfied witness is substantive — `transfers == 1 && paid > 0 &&
  fee_after == 0` — rather than `satisfy!(true)`.
* The prover reported `core::ptr::drop_in_place<Signer>` and
  `core::ptr::drop_in_place<AccountInfo>` as external. Neither writes any state these rules read.

## Findings

Four rules are marked expected-to-fail, each a genuine defect of `process_collect_fee` or its loader:

1. **No balance bound on the payout** (property 7) — the fee is paid from a bookkeeping counter with
   no read of the asset token account; witness `assets = 0, fee = 1, balance = 0, paid = 1`.
   `vault_update_reward` adding `balance - vault.assets` to `vault.assets` while ignoring
   `fee_amount` is the source-level producer of the premise state.
2. **No owner check on the vault account** (property 8) — `VaultInfo::try_from` carries the comments
   "owned by vault program" and "has discriminant" and implements neither, so all three equality
   checks in `CollectFeeContext::validate` compare passed accounts against attacker-chosen bytes.
3. **No vault-assets PDA re-derivation** (property 9) — `CollectFeeContext` declares the field as a
   bare `AccountInfo` instead of `VaultAssetsAccount` and so cannot call `check_vault`; the transfer
   source and the seeds the handler signs with are never tied together.
4. **No self-transfer guard** (property 10) — the `require_ne!(.., VaultError::SelfTransfer)` that
   four sibling handlers carry is absent, and `vault_collect_fee` clears the counter unconditionally
   before the CPI, so naming the vault's own asset account as the destination destroys the claim.

## Munges in force (all gated on `unit_fee_collection`)

* `redirect_module` on `processor` → `src/certora/mocks/processor.rs`.
* `mock_fn` on `spl_transfer_assets_from_user`, `spl_transfer_assets_from_vault`,
  `spl_mint_shares`, `spl_burn_shares` → the stand-ins in this module.

No prover-configuration change was needed: `loop_iter` stays at 2 and `optimistic_loop` stays off,
so no verdict here is conditional on a loop assumption, and no lemma or unproved arithmetic
assumption was used.
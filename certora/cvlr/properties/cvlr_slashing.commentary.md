## What this harness drives

`certora_vault` is a native Solana program (no Anchor). Nine of the twelve rules call
`crate::processor::process_slash` directly on a `Box`ed array of unconstrained `AccountInfo`s, in the
order `SlashContext::load` expects: 0 `vault_info`, 1 `vault_assets_account`, 2 `user_token_account`,
3 `assets_mint`, 4 `authority`, 5 `spl_token_program`. Vault state is read by copying the bytes of
`accounts[0]` with `bytemuck` before and after the call; this program mutates the vault through a
`RefMut` on those same bytes, so there is no deferred write-back to work around.

One munge is in force, scoped to this unit's `unit_slashing` feature (the deployed build and every
other unit compile the file as written). It mirrors what the `unit_redemption` unit already has for
the same wrapper: `crate::processor::spl_token_utils::spl_transfer_assets_from_vault` is replaced by
the recording stand-in in this module, and its `spl_token::instruction::transfer(..)` construction is
lifted verbatim into `build_transfer_assets_from_vault_ix`. Without it every rule through the handler
stops on the loop-unwinding assertion inside the instruction encoder.

## What the verdicts are about, and what they are not

**CPI fidelity.** No rule in this unit claims anything about tokens actually moved, or about either
token account's balance. The stand-in records its arguments and returns `Ok(())`. The second clause of
Property 2 (`x.amount == amount`) and all of Property 7 are statements about the *request* the handler
makes — the amount and accounts routed into the wrapper's parameters, and the instruction the builder
constructs from them — not about its effect; the real SPL Token program's own failure modes are not
modelled. "No asset transfer performed" in the two rejection rules means the wrapper was never called,
not a claim about on-chain balances.

**One unverified joint in Property 7.** The property is proved as a composition of two rules:
`rule_slash_cpi_uses_token_program_and_own_pda_seeds` covers handler → wrapper parameters (including
all three PDA signer seeds), and `rule_slash_transfer_ix_builder` covers builder parameters →
instruction metas (source, destination, authority). The line that joins them — the wrapper forwarding
its parameters to `build_transfer_assets_from_vault_ix` and passing `&[vault_assets_signer_seeds]` to
`invoke_signed` — is in the body `mock_fn` replaces, so no rule executes it. The composition rests on
reading the munged source, which is the developer's code verbatim.

**Scope of the green results.** Properties 1–4 and 7 are proved about the `Vault` held in whatever
account was passed as `vault_info`. Given the Property 5 and Property 6 findings below, a successful
execution need not involve the program's real vault, so these read as "the handler's accounting and
its transfer request are internally correct on the state it was given", not as "the on-chain vault is
always solvent".

**Property 9 is a partial result.** `rule_slash_accounting_accepts_legitimate_amount` drives
`crate::operations::vault_process_slash`, not the handler. What is proved is that the accounting core
— `Vault::del_token`'s `checked_sub` and its post-decrement `check_invariant` — never rejects
`0 < amount <= assets - shares` on a solvent vault. `SlashContext::load`, the
`Signer`/`SplTokenProgramInfo`/`VaultInfo` loaders, the `slash_admin` `require_eq!`,
`VaultAssetsAccount::check_vault` and the transfer are all outside the rule. The narrowing has a
specific cause: `Pubkey::create_program_address` is summarized to an unconstrained address, so the
address a rule derives and the address `check_vault` derives from the same inputs are unrelated values
in the model. An instruction-level "must succeed" rule was written, run, and came back VIOLATED on a
witness where validation refused with every precondition satisfied; it was removed as a modelling
artefact rather than reported. No instruction-level acceptance property is statable through
`check_vault` under this configuration.

**Property 6's seed-derivation half is likewise out of reach.** The property asks for the passed
`vault_assets_account` to equal both `vault.vault_assets_account` and the seed-derived address. The
first half is stated and is the finding below. A rule for the second half was written, run, and came
back VIOLATED with two different "derived" addresses for the same inputs — the same PDA-summary
artefact — and was removed. Neither acceptance nor rejection *by `check_vault` itself* can be
demonstrated here.

## Findings (three rules are expected to fail)

Each is a missing check, visible directly in `programs/vault/src/loaders/vault_loaders.rs`, and each
was confirmed by a counterexample showing an accepted execution that violates the property.

* **Property 5 —** `impl TryFrom<&AccountInfo> for VaultInfo` carries the comments "owned by vault
  program" and "has discriminant" and implements neither. No line on the `process_slash` path reads
  `vault_info.owner`. The remedy is an owner comparison against `crate::ID` (plus a length and
  discriminator check) in `VaultInfo::try_from`, which would protect every context that loads a vault.
* **Property 6 —** `SlashContext::validate` performs only `check_vault` where `DepositContext`,
  `RedeemSharesContext` and `UpdateRewardContext` all additionally `require_eq!` the passed key against
  the recorded `vault.vault_assets_account`.
* **Property 8 —** `SlashContext::validate` never compares `assets_mint` to `vault.assets_mint`, which
  `DepositContext` and `RedeemSharesContext` both do. Today the account is forwarded into the transfer
  wrapper's unread `_mint` parameter, so this is a latent binding gap rather than an immediate theft —
  but any move to `transfer_checked`, or any consumer trusting the instruction's account list, makes
  the attacker-chosen value live.

A caveat a reader should carry into the Property 6 verdict in particular: because
`create_program_address` is summarized, `check_vault` can be satisfied by any key in the model, so the
VIOLATED verdict is not by itself proof of exploitability. The finding is structural and readable in
the source — the omitted `require_eq!`. The same caveat applies to the *witness* for Property 5, though
that finding stands on its own, since no code on the path reads the owner at all.

None of these were repaired by a munge: doing so would have turned a finding into a green verdict while
the shipped program stayed vulnerable.

## Modelling assumptions a reader should see

* Every handler rule assumes `accounts[0].data_len() == size_of::<Vault>()`. Vault accounts of other
  sizes are outside the proof — and note this is a shape constraint the deserializer needs, not a check
  the program performs (see the Property 5 finding).
* The arithmetic rules re-state the `PodU64` storage width with `NativeInt::is_u64()` on the shares and
  assets reads both before and after the call: under `-solanaTACMathInt true` the Prover's model of a
  byte-level read does not re-impose the eight-byte width, and without this restatement counterexamples
  come back with totals no `u64` field can hold. It excludes only unrepresentable states.
* `loop_iter` is 2 with `optimistic_loop` off, so no verdict rests on a loop assumption. No
  `summarize_for_prover` directives were added.
* The Prover left `alloc::raw_vec::RawVec::grow_one` and two `drop_in_place` shims external. The only
  rule that reads a heap `Vec` back is `rule_slash_transfer_ix_builder` (`ix.accounts`), whose verdict
  therefore covers the allocation paths the analysis did keep; the other rules do not depend on what
  those functions do.
* Property 3's solvency invariant is instantiated here for `process_slash` only; the batch's note about
  sharing it with redemption and fee collection is outside this unit.

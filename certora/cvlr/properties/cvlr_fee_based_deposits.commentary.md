## What this harness drives, and what it therefore proves

The batch is covered by 32 rules in three layers.

**Accounting-core rules (properties 1–8, 12, 13, 16, 17, 18, 19).** These drive the program's own
state-transition functions `crate::operations::vault_deposit_assets_with_fee`,
`crate::operations::vault_deposit_assets_with_fee_exact` and (for property 17)
`crate::operations::vault_redeem_shares`, on a `nondet()`-built `Vault` value. The returned
`VaultEffect`'s `assets_to_vault`, `assets_to_fee` and `shares_to_user` are, by inspection of
`processor/deposit_with_fee.rs` and `processor/deposit_with_fee_exact.rs`, exactly the three amounts
each handler hands to the two SPL transfers and the `mint_to`. **That link is established by reading
the handlers, not by any rule.**

**Account-validation rules (properties 10, 11, 14).** These drive the program's own
`crate::loaders::DepositWithFeeContext::load` on nondeterministic accounts — the real validation, not
a hand-built context.

**Guard rules (property 9).** These drive `crate::processor::deposit_with_fee_load_and_guard` /
`deposit_with_fee_exact_load_and_guard`, the extracted prefix of each handler (account load followed
by the self-transfer `require_ne!`, which is everything the handler does before touching vault state).

**No rule in this batch drives `process_deposit_with_fee` or `process_deposit_with_fee_exact` end to
end.** Outside every verdict here, therefore: the `vault_info.get()?` bump read, and the issuing of
the three CPIs. For the accounting-core rules, account validation and the self-transfer guard are also
outside; for the guard rules, the accounting core and the CPI tail are.

## Eight things a reader must weigh beside the verdicts

1. **No rule observes which account received which amount.** Properties 1, 3, 12, 13 and 19 name
   `vault_fee_account` and `vault_assets_account`; the rules assert over `VaultEffect::assets_to_fee`
   and `assets_to_vault` only. A handler that transposed the two transfer destinations would satisfy
   every rule in this batch. Property 4's "`mint_to` to `user_shares_account`" is unverified in the
   same way.

2. **The `_exact` verdicts for properties 5, 6, 7, 8, 12 and 18 are green *because* of the defect
   properties 3 and 4 report.** `vault_deposit_assets_with_fee_exact` performs no state transition at
   all, so solvency preservation, non-dilution, `fee_amount` unchanged and configuration unchanged all
   hold trivially on a function that leaves `vault.shares` and `vault.assets` at their pre-call values
   while directing the handler to mint shares and pull assets. "Solvency preserved by
   `process_deposit_with_fee_exact`" is a restatement of the bug, not reassurance.

3. **The CPI stand-ins are this unit's choice, not a platform limit.** Two `mock_fn` munges replace
   `spl_transfer_assets_from_user` and `spl_mint_shares` with inert `Ok(())` stand-ins (they build an
   `spl_token::instruction::*` value and `invoke`, which is what made four handler-level rules time
   out; both are inlined into the handlers, so no `summarize_for_prover` directive could name them —
   four such directives were tried and matched no symbol, and remain registered but inert). The
   honest statement is narrow: *this unit* records nothing about balances, supplies or CPI arguments
   as issued. `cvlr-solana` 0.5.0 does ship an effect-preserving token model, and a sibling unit of
   this project uses it; this unit does not. No rule here rests on the stand-ins — none drives a full
   handler — so no verdict is weakened by them, but equally no conservation or "nothing moved"
   property is checked.

4. **Property 2's second clause is argued, not exercised.** The rules show the accounting core returns
   `Err` having written nothing to the `Vault` it was handed. That maps onto "Vault account
   byte-identical, no transfer or mint" via two source facts: the handler propagates the core's error
   with `?` before any CPI, and the core is handed a `RefMut<Vault>` aliasing the account's own bytes.
   Handler-level versions of these two rules were written and timed out on four consecutive prover
   submissions, which is why they are not in the final draft. Property 9 has the same shape: the
   extracted prefix returning `Err` means the handler returns `Err` and so issues no CPI, but no rule
   observes the CPI's absence.

5. **Property 15 is demonstrated for `process_deposit_with_fee` only.** The claim that
   `process_deposit_with_fee_exact` is dead on the same state by a different route
   (`convert_shares_to_assets` dividing by `num_shares() == 0`) is source reading with no rule behind
   it.

6. **Solvency is assumed in properties whose text does not mention it** — 1, 7, 8, 12, 13, 18, 19 all
   assume `vault.shares <= vault.assets` of the pre-state. Solvency is an inductive invariant of every
   one of the program's own transitions, so this is not an invented restriction; but it is
   load-bearing for `rule_no_user_assets_debited_without_positive_shares_exact`, where without it the
   exact core can return `shares_to_user > 0` with `assets_to_vault == 0`. Note the interaction with
   the property 10 finding: because `vault_info` carries no owner or discriminator check, a caller can
   supply a fabricated non-solvent Vault buffer, so these verdicts are conditional on the vault account
   being a genuine vault — something the program does not enforce.

7. **Property 14's verdict is a witness for a composition**, not for a single handler call:
   `DepositWithFeeContext::load` accepts ∧ not self-transfer ∧ the plain accounting core succeeds with
   `shares_to_user > 0`.

8. **Properties 13 and 18 partly restate the program's own conversion helpers**, as their property
   texts define them to (`assets_to_vault == pre.convert_shares_to_assets(shares_to_user)`;
   `shares_to_user >= pre.convert_assets_to_shares(assets_to_vault)`). The non-trivial content of 13 is
   the `assets_to_vault + assets_to_fee <= amount` bound.

## The seven findings (eight failing rules)

* **Properties 3, 4 and 17 (`_exact`)** — one root cause:
  `crate::operations::vault_deposit_assets_with_fee_exact` takes `&mut Vault` and never writes to it.
  It omits both `vault.mint_shares(shares_to_user)` and `vault.add_token(assets_to_vault)`, which both
  of its siblings (`vault_deposit_assets_with_fee`, `vault_deposit_assets_exact`) perform. The handler
  nonetheless issues all three CPIs from the returned effect. Witnesses:
  `assets_before = 8, amount = 3 → assets_to_vault = 2, assets_after = 8`;
  `shares_before = 0, amount = 1 → shares_to_user = 1, shares_after = 0`;
  and empty vault, `fee_bps = 0`, `amount = 1924 → s = n = 1924`, then `vault_redeem_shares(1924)`
  fails with `MathOverflow` because `burn_shares` underflows — the depositor's assets are stranded.
  Remediation: add the two missing calls after both `convert_*` calls, leaving `fee_amount` alone.

* **Property 10** — `impl TryFrom<&AccountInfo> for VaultInfo` carries the comments
  `// owned by vault program` and `// has discriminant` and implements neither; it calls only
  `Vault::validate`, i.e. `assets_mint != shares_mint`. `load` accepts a `vault_info` whose owner is
  not `crate::ID`, which makes all four `require_eq!`s in `DepositWithFeeContext::validate`
  self-referential. The rule exhibits the *owner* half directly; the discriminator half only
  vacuously, since `Vault` has no discriminator field at all.

* **Property 11** — `DepositWithFeeContext` types `vault_assets_account` and `shares_mint` as bare
  `AccountInfo` and never calls `create_vault_*_address` / `check_vault`, unlike
  `DepositContext::validate`. **Caveat a reviewer must have:** the Prover summarizes
  `Pubkey::create_program_address`, so the address the rule derives is unconstrained and the trace
  alone would not distinguish "no check" from "a check the Prover cannot model". The trace establishes
  that `load` returns `Ok` on an account set constrained only by the vault's self-reported fields; the
  *source* establishes that there is no derivation call on this path to consult. Both halves are
  needed.

* **Property 15** — with `shares == 0 < assets`, `convert_assets_to_shares` takes
  `mul_div_floor(net, 0, assets) = 0` for every `net`, and `mint_shares(0)` fails `require_gt!`. The
  satisfy rule is the formal statement (refuted, hence no trace); the `_witness` rule is its exhibit
  and returns a populated counterexample (`assets = 2^64-1`, `fee_bps = 9999`, `amount = 2^64-1`,
  `shares_to_user = 0`, `ok = false`). The state is reachable permissionlessly via a one-token donation
  plus `process_update_reward`.

## Program modifications in this build (verification-only, feature-gated)

* Two `mock_fn` munges on `spl_transfer_assets_from_user` / `spl_mint_shares` → inert harness
  stand-ins (see point 3).
* Two `extract_function` munges splitting the `load` + self-transfer-guard prefix out of each
  fee-deposit handler into `deposit_with_fee_load_and_guard` / `deposit_with_fee_exact_load_and_guard`.
  The diff is behaviour-preserving (same `load`, same `require_ne!`, same error, same order), and both
  originals remain under `#[cfg(not(feature = "unit_fee_based_deposits"))]`. Property 9's rules drive
  the extracted prefix, which contains the whole of the validation the property is about; what stayed
  behind (the accounting core and the CPI tail) is unreachable once the guard fires.

Prover settings were not changed: the shipped `loop_iter: 2` and `optimistic_loop: false` are in force,
so no verdict here rests on a loop assumption.

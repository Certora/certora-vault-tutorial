# AutoProver vs. the hand-written spec

A comparison of the CVLR spec AutoProver generated for this vault (branch
`eric/autoprover-2026-10-01`, run of 2026-10-01) with the hand-written spec on `main`
(`programs/vault/src/certora`). It covers the properties each side checks, the rules each wrote to
check them, and the violations each found.

**"CI" in this document** means the Prover regression test this repo's CircleCI workflow runs on
`main` (`.circleci/config.yml`). Every Sunday at 04:00 GMT, or when triggered manually, it runs
the 44 rules listed in `programs/vault/src/certora/ci/rules.conf` with `certoraSolanaProver`. It
checks each verdict against `ci/expected.json`, which expects 34 to pass (SUCCESS) and 10 to fail
(FAIL). The hand-written verdicts quoted here are those expectations; they were not re-run for this
comparison. The four `utils_math.rs` rules are not in `rules.conf`.

## Summary

- **Scale.** The hand-written spec has 48 rules, 44 of them in CI (34 expected to pass, 10 expected
  to fail). AutoProver stated 85 properties across six components and checked them with 130 rules:
  96 hold and 34 fail, each failure recorded as a defect.
- **Agreement.** Where the two specs check the same property, they reach the same verdict. That
  covers solvency on every handler, no-dilution on every handler except slash, deposit and
  redemption integrity, and vault consistency for deposits.
- **Hand-written defects AutoProver found.** The hand-written CI expects four failures that point
  at defects rather than demonstrations. AutoProver found three of them: the self-transfer in
  `process_deposit`, the standard deposit path skipping a configured fee, and dilution by
  `process_deposit_exact`. It missed the fourth, a fee that rounds down to zero on small deposits,
  because its fee property states the floor formula as correct.
- **New defects.** AutoProver found about a dozen distinct defects the hand-written spec does not
  check for. The most serious:
  - No handler checks that `vault_info` is owned by the program.
  - Several handlers never re-derive the vault's PDAs.
  - `process_deposit_with_fee_exact` never writes its state change.
  - A vault with zero shares and positive assets bricks every deposit, and the permissionless
    `process_update_reward` can put a vault in that state.
  - Deposits price shares against a stale asset balance.
  - Fee collection can spend assets that back shares.
- **Hand-written coverage AutoProver lacks.** AutoProver does not have:
  - the inflation-attack scenarios (attacker and victim across four operations);
  - the equivalence of fee and fee-free deposits at a zero fee;
  - fee-computation liveness;
  - vault consistency for redeem and slash;
  - slash no-dilution, which is expected to fail by design.
- **What a hand-written SUCCESS can hide.** The hand-written solvency and no-dilution rules pass on
  `process_deposit_with_fee_exact`, but only because that handler leaves the vault's recorded state
  unchanged. AutoProver checks recorded state against the token CPIs, and that check is what
  exposes the bug.

## What was compared

Both specs verify the same program logic. The benchmark baseline (`a81123d`) differs from `main`
outside `src/certora` in only two ways:

- **Real CPIs.** The SPL Token helpers in `processor/spl_token_utils.rs` perform real CPIs (`invoke`
  / `invoke_signed`) instead of returning `Ok(())`, and their callers pass the PDA signer seeds.
- **No verification hooks.** The `certora`-feature hooks in `utils/guards.rs` and `utils/math.rs`
  are removed.

Each side verifies under its own simplifications, and none of them shows in the program source:

| | Hand-written (`main`) | AutoProver (this branch) |
| --- | --- | --- |
| Token CPIs | `mock_fn` on each helper, with stand-ins built on `cvlr_solana::token` transfer, mint and burn | Per-unit stand-ins for the same helpers, mostly built on the same `cvlr_solana::token` models. Redemption records the instruction built instead |
| Error paths | `require!` and its relatives panic under `certora`, so rejection paths are pruned | The program's own `require!`, so rejection is checkable |
| Arithmetic | `mul_div_floor`/`ceil` replaced by `NativeInt` versions, proved equivalent by four rules in `utils_math.rs`; those rules are not in CI | Native arithmetic |
| Level | Mostly the pure `operations::vault_*` functions; processor-level rules for no-dilution, solvency, consistency and one access check | Every rule drives a `process_*` handler |
| Unreached code | — | `process_collect_fee` is never declared in `processor/mod.rs`, so it is not compiled. Fee Collection compiles the program's own `collect_fee.rs` into the harness with `include!` |

The verdicts above are about the modified program on each side. AutoProver lists its edits in
`certora/cvlr/reports/report.json` under `source_edits`.

AutoProver was given the program and this README with its Specifications section removed. It never
saw the hand-written spec or the property list.

## Property by property

Each hand-written property family, with its CI verdicts and AutoProver's closest counterpart.

| Hand-written family | Rules (CI expectation) | AutoProver counterpart | Verdict |
| --- | --- | --- | --- |
| **Access control:** `process_deposit` binds `vault_assets_account`, `assets_mint` and `shares_mint` to the vault record | 1 (pass) | Not stated for `process_deposit`. For redeem, binding to the record holds but the PDA re-derivation fails. For slash, the account and mint are not bound (fails). For fee deposits, the PDAs are not re-derived (fails) | Partial; AutoProver extends the check to other handlers and finds gaps there |
| **Fees assessed:** `fee_bps > 0 ⇒ fee > 0` on the fee-free path | 1 (fail) | `standard_deposit_must_not_bypass_configured_fee` fails on both handlers | Agree |
| **Fees assessed:** `fee_bps > 0 ⇒ fee > 0` on the fee path | 1 (fail) | `fee_is_exact_floor_bps_and_split_sums_to_amount` holds. It states `floor(amount × bps / 10 000)` as correct, so a fee of zero on small deposits does not surface | **Disagree on intent;** AutoProver misses this one |
| **Fee liveness:** `deposit_assets_with_fee` always succeeds on a solvent vault without overflow | 1 (pass) | `legitimate_fee_deposit_is_reachable` holds, but it only shows one successful execution exists | Weaker |
| **Fee / fee-free equivalence** when `fee_bps = 0`, results and errors | 2 (pass) | None. The nearest is `exact_fee_deposit_is_a_refinement_of_plain_fee_deposit`, which compares two other handlers | Not covered |
| **Integrity:** deposit raises assets and shares; deposit and redeem are monotone | 4 (pass) | `deposit_mints_shares_at_current_ratio`, `redeem_payout_equals_floor_of_pre_ratio` and `redeem_burns_exactly_requested_shares…` all hold, asserting the exact deltas | Agree; AutoProver's checks are stronger |
| **Slash no-dilution:** fails by design, since a slash dilutes | 3 (fail) | Not stated. `slash_decrements_assets_by_exactly_amount` and `slash_preserves_vault_solvency` hold | Not covered (a design property) |
| **No-dilution** for deposit, fee deposit, redeem and update reward (operation and processor) | 9 (pass) | Deposit, both fee deposits, redeem and update reward all hold | Agree |
| **No-dilution** for `process_deposit_exact` | 1 (fail) | `deposit_never_dilutes_existing_shareholders` fails for the exact handler. Finding: double floor rounding in `vault_deposit_assets_exact` | Agree |
| **Solvency** `shares ≤ assets` on all five operations and seven handlers | 12 (pass) | Holds for deposits, fee deposits, redeem, update reward and slash | Agree |
| **Vault consistency** (`assets ≤ balance ∧ shares = supply`) on `process_deposit` | 1 (fail) | `recorded_assets_remain_backed_by_token_balance` (stated over `assets + fee_amount`) holds for distinct accounts and fails for a self-transfer. `vault_shares_equals_shares_mint_supply` holds | Agree, and AutoProver's version is stronger |
| **Vault consistency** on redeem, update reward and slash | 3 (pass) | Update reward: `syncs_assets_exactly_to_token_balance` holds, and the fee-inclusive form fails. Redeem and slash: not stated | Partial; gaps on redeem and slash |
| **Inflation attack:** attacker profit, victim loss bounds, the exact-deposit mitigation | 5 (3 fail, 2 pass) | No multi-party scenario. Related checks: `deposit_then_redeem_is_not_profitable` holds; `deposit_never_overcharges_depositor_for_shares` fails (the rounding loss behind `rule_inflation_alice_loss_bound`); `deposit_prices_shares_against_stale_asset_balance` fails (donation capture) | Partial |
| **Math equivalence** of `NativeInt` and native `mul_div` | 4 (not in CI) | Not needed, since AutoProver verifies the native arithmetic | n/a |

### What AutoProver checks that the hand-written spec does not

These are the classes of property only AutoProver states. The violated ones are listed under
[Violations](#violations).

- **Account validation:**
  - owner of `vault_info`;
  - PDA re-derivation of the asset account and shares mint;
  - binding of passed accounts to the vault record;
  - signer requirements for deposit, redeem, collect and slash;
  - rejection of self-transfers on every token-moving handler.
- **CPI arguments:** that the amount, source, destination, authority, token program and signer
  seeds of each transfer, mint and burn match the vault's recorded state change.
- **Configuration immutability:** that no handler rewrites `admin`, `slash_admin`, the mints, the
  bumps, `fee_bps` or `fee_token_account`.
- **Fee accounting:** `fee_amount` is changed only by fee collection, fee deposits leave it alone,
  and the fee cap is enforced before any effect.
- **Fee collection,** which the hand-written spec does not cover at all.
- **Reachability:** every handler has a legitimate successful path, so no guard stack is dead.
- **Two-step sequences:** a deposit followed by redeeming the same shares, for every deposit
  handler.
- **Rejection properties:** over-redemption, a fee above 100%, and a balance below recorded assets
  on update reward. These are possible because AutoProver keeps the program's real error paths.

## Violations

### Hand-written expected failures

| Hand-written rule | What it shows | Found by AutoProver? |
| --- | --- | --- |
| `rule_vault_consistency_process_deposit` | Self-transfer in `process_deposit` mints shares with no tokens moving; the guard is commented out | **Yes:** `deposit_must_reject_self_transfer` and `recorded_assets_remain_backed_by_token_balance` (critical) |
| `rule_fees_assessed_deposit_assets` | The fee-free deposit succeeds on a fee-configured vault; the `require!` is commented out | **Yes:** `standard_deposit_must_not_bypass_configured_fee`, on both standard handlers (high) |
| `rule_no_dilution_process_deposit_exact` | The exact deposit dilutes existing holders | **Yes:** double floor rounding in `vault_deposit_assets_exact` (high) |
| `rule_fees_assessed_deposit_assets_with_fee` | The fee rounds to zero when `amount × fee_bps < 10 000`, so a depositor can split deposits to avoid the fee | **No:** AutoProver's fee property states the floor formula as correct |
| `rule_slash_no_dilution`, `rule_no_dilution_slash`, `rule_no_dilution_process_slash` | A slash dilutes, by design | Not stated |
| `rule_inflation_attack` | With no virtual shares, an attacker profits by donating before the victim deposits | Not stated as a scenario. Adjacent findings: stale pricing, and update reward crediting donations to a share-less vault |
| `rule_inflation_alice_loss_bound` | A depositor's loss to rounding is unbounded | **Yes, in substance:** `deposit_never_overcharges_depositor_for_shares` fails, because the floor-rounded remainder is donated to existing holders (high) |
| `rule_inflation_bob_loss` | The attacker can harm the victim without losing anything | Not stated |

Of the ten expected failures, four are defects: self-transfer, fee bypass, exact-deposit dilution
and the zero fee. AutoProver found three of them. The other six demonstrate designed behaviour
(slash dilution) or teach the inflation attack. AutoProver states one of those in substance (the
rounding loss) and touches the attack itself only through adjacent findings.

### Defects only AutoProver reports

The report has 34 findings: 14 critical, 15 high, 3 medium and 2 low. Several components found the
same root cause, so these collapse to the issues below. The severities are AutoProver's own.

1. **No owner or discriminator check on `vault_info`** (critical, all six components).
   `VaultInfo::try_from` reinterprets the bytes with `bytemuck` and checks only
   `assets_mint != shares_mint`; nothing in `loaders/` checks the owner. An attacker-owned
   look-alike account decides the shares, assets, recorded accounts, bumps and admins, and every
   handler accepts it.
2. **Vault PDAs never re-derived** (critical / high). `DepositWithFeeContext`, the redeem context,
   `CollectFeeContext` and `UpdateRewardContext` never call `check_vault`, so the asset account and
   the shares mint can be substituted. On the paths that sign with the vault's seeds, that steers a
   signed transfer.
3. **`process_deposit_with_fee_exact` never commits its state change** (critical).
   `vault_deposit_assets_with_fee_exact` computes the effect but never calls `mint_shares` or
   `add_token`. Tokens are transferred and shares minted, but `vault.shares` and `vault.assets`
   stay the same, stranding the deposit. The hand-written solvency and no-dilution rules pass on
   this handler *because* nothing changes.
4. **Deposits brick at zero shares with positive assets** (critical / high, every deposit handler).
   `convert_assets_to_shares` returns `floor(amount × 0 / assets) = 0`, and `mint_shares` rejects 0.
   **`process_update_reward` can create that state** (critical): it is permissionless and will
   credit a donation to a vault with no shares.
5. **Stale pricing** (high, both standard handlers). Deposits price shares against the recorded
   `vault.assets`, not the token balance, so a depositor captures donations and rewards that
   `update_reward` has not yet recognised.
6. **Update reward trusts an unvalidated asset account** (critical / high). It checks neither that
   the account is the canonical PDA nor that its owner is the SPL Token program before reading its
   balance.
7. **Update reward counts pending fees as reward** (medium). It reconciles against `vault.assets`
   and not `assets + fee_amount`, so a later fee collection takes assets that back shares.
8. **Fee collection is unbounded by the vault's balance** (high). It pays the recorded `fee_amount`
   without reading the vault's token balance, so it can spend assets that back shares.
9. **Fee collection has no self-transfer guard** (medium): an aliased destination zeroes
   `fee_amount` and moves nothing.
10. **Slash does not bind its accounts** (medium / low). The asset account is not checked against
    `vault.vault_assets_account`, and the assets mint is not checked against `vault.assets_mint`.

One more observation from the run is not a finding in the report: `process_collect_fee` is
unreachable as the program stands, as noted in [What was compared](#what-was-compared). The fee
bypass on the fee-free exact deposit is a finding, counted in the expected-failures table above.

## Effort

The AutoProver run that produced these results took 3 h 24 m of wall-clock time, about $101–116
of model spend and 21 Prover jobs (7 h 19 m of Prover time). All six components delivered with the
judge's approval. There were no skips and no budget cuts. That figure leaves out the earlier
development runs on this program, which were spent fixing AutoProver itself.

## Caveats

- **AutoProver's findings need review.** They are claims backed by counterexamples, and each was
  marked expected-to-fail by an author that read the counterexample, but no person has triaged them.
  Severity is AutoProver's own assessment.
- **Several defects look seeded.** Some are commented-out guards (the self-transfer guard in
  `process_deposit`, the fee check in `vault_deposit_assets`), which suggests this tutorial includes
  bugs on purpose. The comparison does not try to tell intended teaching bugs from accidental ones.
- **AutoProver's verdicts are about stand-ins.** Each CPI is replaced by a stand-in. A property
  about tokens moved is only as faithful as the model behind the stand-in. Those are the
  `cvlr_solana::token` models the hand-written spec also uses.
- **Not every AutoProver rule targets the same function as its hand-written counterpart.**
  Hand-written rules mostly drive the pure `operations::vault_*` functions; AutoProver's always
  drive the handlers. Where the table says "agree", the property matches, but the two rules may
  call different functions.

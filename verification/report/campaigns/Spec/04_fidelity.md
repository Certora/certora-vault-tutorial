# 4. Fidelity review: do the proved statements mean what they claim about the real code?

The kernel shows that each theorem is a true statement about the Lean model. This section asks
whether that statement **says what it appears to say about `programs/vault`**. I answer it by
reading three layers against each other: the Rust (`programs/vault/src/…`, and the extraction copy
`verification/translate/extract/src/…`), the Aeneas translation (`verification/lean/CertoraVault.lean`),
and the Aeneas standard library that defines what a `u64`, a `checked_add` or a `⦃ ⦄` *is*
(`/opt/aeneas/backends/lean/Aeneas/Std/…`, cited below as `Std/…`).

I start with the gaps and the worries, because they decide how much the wins are worth. Then I go
through each projection.

---

## 4.0 Findings first

| # | Finding | Severity for the owner | Where |
| --- | --- | --- | --- |
| F1 | **`vault_deposit_assets_with_fee_exact` never updates the vault, yet its processor moves tokens and mints shares.** The theorem proves the first half (`r.2 = v` in every outcome). The second half is in the processor. The proofs faithfully mirror code that looks wrong. | **High: candidate defect** | `Spec.lean:361-371`; `programs/vault/src/operations/vault_exact_deposit_operations.rs:19-39`; `programs/vault/src/processor/deposit_with_fee_exact.rs:31-62` |
| F2 | Six theorems state solvency over `committed` (`Spec.lean:166,252,342,424,577,742`). Five of them (deposit ×3, redeem, slash) actually depend on it; the fee-exact one does not, because it has `r.2 = v`. These theorems read `Err` as "the pre-state survives". That is **Solana's revert-on-failure semantics**, which no Lean theorem establishes. I confirmed by reading that every processor propagates the operation's `Err` with `?`. | Modelling assumption, sound for Solana | `Spec.lean:47-51`; processors cited in §4.4 |
| F3 | `default_vault_solvent` runs no code (surface-only). The real constructor `Vault::new()` was not translated. | Low | `Spec.lean:68-70`; `programs/vault/src/state.rs:33-35` |
| F4 | The model is of an extraction crate (4-field `Vault`, edits E1–E3). Equivalence to the original is supported by differential tests, **not proof**. | Translation trust (see §2) | `translate/extract/src/equivalence_tests.rs` |
| F5 | The deposit `_ok` theorems assume a solvent starting vault. Nothing is proved about deposits into an insolvent vault, and the deposit code does not check solvency itself. | By design; note it | `Spec.lean:77,277` (`hsolv`) |
| F6 | Out-of-scope observation: no in-scope operation ever increases `fee_amount` (proved: every operation except `collect_fee` keeps it equal, and `collect_fee` zeroes it). Yet `process_collect_fee` transfers `fee_amount` tokens **from the vault's assets account** without reducing `Vault.assets`. | Low / latent | `programs/vault/src/processor/collect_fee.rs:16-22` |

No **ungrounded** projection was found. `axioms.grounding` reports `is_solvent` as grounded, and my
reading below agrees.

---

## 4.1 What a `u64` and a "property" are in this model (checked in `Std`)

These encoding facts are what every projection below depends on. I read them from their
definitions rather than inferring them from names.

- **A Rust `u64` is `Std.U64 = UScalar .U64`** (`Std/Scalar/Core.lean:666`), a structure wrapping a
  64-bit `BitVec` (`:71-73`). **`.val` is `x.bv.toNat`, a natural number `ℕ`** (`:76`). Every
  integer in this crate is unsigned (`u64`, plus `u128` inside `mul_div_floor`). So every `.val` in
  the statements is a `Nat`, no statement mixes `Nat` with `Int`, and there is no signed `.val`
  (`IScalar.val = bv.toInt`, `:84`) anywhere.
- **Comparison and equality are value comparisons.** `instance LE (UScalar ty)` is
  `a.val ≤ b.val` (`Core.lean:897`), `LT` is `a.val < b.val` (`:894-895`), and two `UScalar`s are
  equal iff their `.val`s are (`UScalar.eq_equiv`, `:906`). So Rust's `self.num_shares() ==
  self.num_assets()` and `<=` translate to exactly the `Nat` facts the statements use.
- **`⦃ ⦄` is total correctness.** `f x ⦃ r => P ⦄` unfolds to `WP.spec (f x) P` (`Std/WP.lean:358-361`).
  `WP.spec` has the single constructor `ret : p x → spec (.ok x) p` (`WP.lean:30-31`), so it
  **cannot hold if the call `fail`s (panics) or diverges**. Every `@[lusterna]` theorem therefore
  also proves that the function does not panic under its hypotheses.
- **`Nat` subtraction truncates at 0 and `Nat` division by 0 is 0.** Both can make a
  faithful-looking formula lie. Where each could matter is addressed in §4.3.

---

## 4.2 The anchor: `Solvent` ↔ `Vault::is_solvent` (grounded)

**The three parts.**
- *Projection:* `Solvent v := v.shares.val ≤ v.assets.val` (`Spec.lean:29`).
- *Real function (INFER anchor `is_solvent`):* `Vault::is_solvent`
  (`programs/vault/src/state.rs:62-64`: `self.num_shares() <= self.num_assets()`).
- *Bridge:* `is_solvent_spec` (`Spec.lean:58-59`):
  `state.Vault.is_solvent v ⦃ b => (b = true ↔ Solvent v) ⦄`. It is **established**: it is in
  `axioms.impl_verified`, `axioms.grounding.grounded = ["is_solvent"]`, and my `#print axioms`
  cross-check shows only the standard axioms.

**Walking the code, term by term.**
1. Rust: `self.num_shares() <= self.num_assets()`, where `num_shares` is `self.shares.into()` and
   `num_assets` is `self.assets.into()` (`state.rs:37-43`). On the real account, `shares`/`assets`
   are `PodU64` and `.into()` is `u64::from_le_bytes` (`spl-pod-0.2.5/src/primitives.rs:50-64,107-108`).
2. Extraction: identical except that the fields are already `u64` (`translate/extract/src/state.rs:18-24,38-40`).
   This is edit E1, discussed in §4.5.
3. Lean: `state.Vault.is_solvent self = do let i ← num_shares self; let i1 ← num_assets self; ok (i <= i1)`
   (`CertoraVault.lean:543-546`), with `num_shares self = ok self.shares` (`:73-74`) and
   `num_assets self = ok self.assets` (`:44-45`). The `Bool` `i <= i1` decides the `LE` instance,
   i.e. `self.shares.val ≤ self.assets.val` (`Std/Scalar/Core.lean:897`).
4. That is exactly `Solvent self`. Both sides are `Nat`, with no coercion, no scale factor and no
   subtraction.

**What this buys.** Every solvency theorem concludes `Solvent r.2` or `Solvent (committed v r)`. By
the bridge, that is the same as saying **the program's own `is_solvent()` would return `true` on
that vault**. The program uses the same predicate as its guard: `check_invariant` calls
`is_solvent` (`CertoraVault.lean:551-558`, from `state.rs:66-68` via edit E2), and `del_token`
calls `check_invariant` after writing (`CertoraVault.lean:563-587`). So the property is stated in
the code's own notion of solvency, not an approximation of it. Stating it over `.val` is also the
only way to state it at the value level: the machine stores a 64-bit pattern, `≤` on patterns is
*defined* as `≤` on their `toNat` readings, and that is the reading the program means by `<=`.

---

## 4.3 The result formulas: `assetsToShares`, `sharesToAssets`, `feeOf`

These are **not** stand-ins for a separate measurement that the properties reason about instead of
the code. They appear only as the right-hand side of an equation about the real output, for example
`e.shares_to_user.val = assetsToShares v t.val` inside a triple that runs
`vault_deposit_assets`. **The theorem that states the equation is itself the bridge.** If the
formula were unfaithful, the theorem would be false and would not have been proved. Fidelity still
matters in one sense: a reader takes the formula as *documentation* of what the code computes, so
it should mean what it looks like. I checked it term by term.

### `mul_div_floor(a, b, c)`: the arithmetic core (`CertoraVault.lean:102-130`)

| Step | Lean (line) | `Std` definition | Exact behaviour |
| --- | --- | --- | --- |
| `a as u128`, `b as u128` | `:106-107` `UScalar.cast .U128` | zero-extend (`Std/Scalar/Casts.lean:20-22`), `.val` = `x.val % 2^128` (`:209-210`) | lossless (a `u64` is < 2^64) |
| `checked_mul` | `:108` `U128.checked_mul` | `Option.ofResult (UScalar.mul x y)` (`Std/Scalar/CheckedOps/Mul.lean:15-16`). `UScalar.mul = tryMk (x.val * y.val)` (`Std/Scalar/Ops/Mul.lean:18-19`), so it is `none` iff the product > `U128.max` | **never `none` here:** `a.val·b.val ≤ (2^64−1)^2 < 2^128` (`VaultLemmas.lean:25-30`) |
| `checked_div` | `:114` `U128.checked_div` | `Option.ofResult (UScalar.div x y)` (`CheckedOps/Div.lean:15-16`). `UScalar.div` is `fail divisionByZero` if `y = 0`, else `BitVec.udiv` (`Ops/Div.lean:17-18`) | `none` iff `c = 0`, otherwise floor division |
| `ok_or(MathOverflow)?` | `:109-111`, `:115-117` | `some v ↦ ok (.Ok v)`, `none ↦ ok (.Err e)` (`Std/Core/CoreOption.lean:24-28`) | a `None` becomes **a Rust `Err` value, not a panic** |
| `q > u64::MAX` guard (edit E3) | `:119-121` | `core.num.U64.MAX = 2^64−1` (`Std/Scalar/Core.lean:863`) | `Err(MathOverflow)` iff `q > 2^64−1` |
| `q as u64` | `:123` `UScalar.cast .U64` | truncate, `.val` = `q.val % 2^64` (`Casts.lean:209-210`) | lossless here because the guard ensured `q ≤ 2^64−1` |

Result, proved as `mul_div_floor.spec` (`VaultLemmas.lean:75-80`): **`Ok z` with `z.val = a·b / c`
(floor) iff `c ≠ 0` and the quotient fits in `u64`, and otherwise `Err(MathOverflow)`.** That is
exactly the Rust `mul_div_floor` (`programs/vault/src/utils/math.rs:9-17`).

### The conversions: argument order checked

- `convert_assets_to_shares(token)`: if `num_shares == num_assets` it returns `token`, otherwise
  `mul_div_floor(token, num_shares, num_assets)` (`state.rs:87-95` → `CertoraVault.lean:141-144`).
  `assetsToShares v x = if shares = assets then x else x * shares / assets` (`Spec.lean:33-34`)
  matches in both branches and in argument order (numerator `shares`, denominator `assets`).
- `convert_shares_to_assets(shares)` calls `mul_div_floor(shares, num_assets, num_shares)`
  (`state.rs:70-77` → `CertoraVault.lean:162-165`). `sharesToAssets v y = … y * assets / shares`
  (`Spec.lean:38-39`) matches, with numerator `assets` and denominator `shares`.

**Worry: `Nat` division by zero.** `x * shares / assets` with `assets = 0` evaluates to `0` in Lean,
while Rust returns `Err(MathOverflow)`. *Answer:* on every `Ok` branch the formula is reached only
after `mul_div_floor` succeeded, which implies the divisor is non-zero (`VaultLemmas.lean:78`,
`c.val ≠ 0`), so the two agree. The single statement where the formula is used *outside* an `Ok`
branch is `vault_deposit_assets_zero_shares` (`Spec.lean:134-136`, hypothesis
`assetsToShares v t.val = 0`). It carries the explicit guard `hdef : shares = assets ∨ 0 < assets`,
which removes exactly the divide-by-zero case. Without `hdef` the theorem would claim
`Err(GuardFail)` where the code gives `Err(MathOverflow)`. The authors caught this, and the
statement is precise.

### `feeOf` and `FeeBps::apply`

`FeeBps::apply(amt)` is `fee = mul_div_floor(amt, bps, ONE_IN_BPS)` followed by
`net = amt.checked_sub(fee)` (`programs/vault/src/utils/math.rs:52-56` → `CertoraVault.lean:271-291`).
`ONE_IN_BPS = 10000#u64` (`CertoraVault.lean:259`), and `FeeBps` is a reducible alias of `Std.U64`
(`:255`), so `bps.val` is the plain basis-point value with no hidden scale factor.
`feeOf bps amt = amt * bps / 10000` (`Spec.lean:42`) matches.

**Worry: `Nat` subtraction truncates.** `vault_deposit_assets_with_fee_exact_spec` writes
`t.val - feeOf …` (`Spec.lean:367`). *Answer:* this appears only on the `Ok` branch, where
`fee_bps ≤ 10000` is part of the conclusion, and `feeOf bps t ≤ t` whenever `bps ≤ 10000`
(`fee_le`, `VaultLemmas.lean:327-330`). The truncation never applies. The proof also derives the
value from the real `checked_sub`'s success, not from the formula (`Spec.lean:381-382`). The other
statements avoid `Nat` subtraction altogether by using additive forms:
`assets_to_vault + assets_to_fee = t` (`:189`), `r.2.shares.val + n.val = v.shares.val` (`:448`),
and `r.2.assets.val + a.val = v.assets.val` (`:678`). These are exact over `Nat`.

### Effect fields and unchanged fields

Statements like `r.2.fee_bps = v.fee_bps`, `e.shares_to_burn = n` and `r.2 = v` are equalities of
`UScalar`s or of whole `state.Vault` structures (all 4 fields), so they are exact. The zero effect
fields come from `VaultEffect::default` (`CertoraVault.lean:187-196`), whose fields are
`core.default.DefaultU64.default`. `VaultLemmas.lean:68-69` proves by `rfl` that this is the
all-zero record.

---

## 4.4 The one projection with an extra-kernel assumption: `committed`

**The three parts.**
- *Projection:* `committed v r` is `r.2` on `Ok` and `v` on `Err` (`Spec.lean:47-51`).
- *Real thing it stands for:* the `Vault` account data **as persisted on-chain** after the
  instruction.
- *Bridge:* **none in Lean, and none is possible in this model.** It is Solana's transaction
  atomicity: if a processor returns `Err`, every account write the instruction made is discarded.

**Is it load-bearing?** Yes, for some `Err` paths. On `Err` the Rust functions can leave the
`&mut Vault` partially written:
- deposit: `mint_shares` succeeds, then `add_token` overflows. The Lean shows the post-mint vault
  being returned (`CertoraVault.lean:526`, `ok (r3, vault2)` / `:532`).
- redeem: `burn_shares` succeeds, then `del_token` fails (`:639`, `:645`).
- slash: `del_token` writes the reduced `assets` and then fails `check_invariant`. This is
  **proved** to leave `r.2.assets = v.assets − a`, an insolvent state, in
  `vault_process_slash_insolvent` (`Spec.lean:722-727`).

So `Solvent (committed v r)` is true about the chain only if those `Err`s abort the instruction.

**Confirmed by reading.** Every processor propagates the operation's error with `?`:
`processor/deposit.rs:31`, `deposit_exact.rs:32`, `deposit_with_fee.rs:33-34`,
`deposit_with_fee_exact.rs:33-34`, `redeem_shares.rs:31`, `slash.rs:19`, `update_reward.rs:22`,
`collect_fee.rs:13`. An `Err` therefore becomes the instruction's `ProgramResult::Err`, and the
runtime reverts. **The assumption is sound for the deployed program**, but it is a fact about
Solana and the processors (which are out of scope), not something the kernel checked.

**Where it is *not* needed.** `vault_update_reward_preserves_solvent` (`Spec.lean:644-646`) and
`vault_collect_fee_preserves_solvent` (`:774-775`) state `Solvent r.2` in **every** outcome. The
`Err`-path theorems `…_zero_shares`, `…_bad_bps`, `…_too_many`, `…_below` and `…_exceeds_assets`
prove `r.2 = v` outright, which is stronger than needing a revert.

---

## 4.5 Overflow posture and the hand-modelled type

**Why the model's arithmetic is exact, whatever the profile.** The deployed workspace sets
`[profile.release] overflow-checks = true` (`Cargo.toml:5-7`), and the extraction manifest copies
it verbatim (`verification/translate/extract/Cargo.toml`, last block). The harness verified that the
two match (`overflow_posture.block = false`). In this crate the profile turns out not to matter:
`model_checked_ops = false`, and the pretty-printed LLBC contains **no** `panic.+`/`wrap.+` operator
at all, only calls to `checked_add`/`checked_sub` (u64) and `checked_mul`/`checked_div` (u128).
Each is modelled in `Std` as an `Option` that is `none` exactly when Rust's returns `None`:
- `checked_add` is `Option.ofResult (x + y)` (`Std/Scalar/CheckedOps/Add.lean:15-16`), where
  `x + y = UScalar.add = tryMk (x.val + y.val)` (`Std/Scalar/Ops/Add.lean:16-17`), failing iff the
  sum exceeds `2^64−1`.
- `checked_sub` uses `UScalar.sub`, which fails iff `x.val < y.val` (`Std/Scalar/Ops/Sub.lean:17-19`;
  `CheckedOps/Sub.lean:15-16`).
- `checked_mul` and `checked_div` are as tabulated in §4.3.

They are wrapped in `lift`, which is plain `Result.ok` (`Std/Primitives.lean:456`). So in the model,
as in the binary, **overflow is never a panic. It is the value `Err(VaultError::MathOverflow)`.**
This is why every operation can be proved total with no overflow side conditions. It is also why no
`= ok` property here can be hiding a wrapping execution: there is no wrapping arithmetic anywhere in
the verified core.

**The hand-modelled type: `PodU64 → u64` (E1).** The harness cannot enforce an overflow profile on
a hand model, so I checked the agent's claim by reading. `PodU64(pub [u8; 8])` has no arithmetic:
its only operations are `From<u64>` (`to_le_bytes`) and `From<PodU64> for u64` (`from_le_bytes`)
(`spl-pod-0.2.5/src/primitives.rs:50-64,107-108`). Every arithmetic step in the original converts
to `u64` first and back after (`programs/vault/src/state.rs:107-146`). So the swap has **no overflow
behaviour to match**: it neither aborts nor wraps, and the `u64` arithmetic that does happen is
translated from the crate itself. E2 (`then_some` → `if`) involves no arithmetic. E3 (`try_into` →
range guard) reproduces `u64::try_from(u128)` exactly: it fails iff `q > u64::MAX`, as tabulated in
§4.3. **No hand-modelled overflow gap.**

---

## 4.6 Operation by operation: what the proved property means for the real code

**Deposits (`vault_deposit_assets`, `_with_fee`, `_exact`).** The Lean bodies
(`CertoraVault.lean:499-538`, `:394-458`, `:201-249`) follow the Rust line for line: convert, then
`mint_shares`, then `add_token` (`programs/vault/src/operations/vault_operations.rs:14-28`,
`vault_fee_operations.rs:5-20`, `vault_exact_deposit_operations.rs:5-17`). The theorems say that a
successful deposit mints exactly `floor(amount·shares/assets)` shares (or `amount` at a 1:1 ratio),
books exactly the amount the effect reports, and leaves `shares ≤ assets`. Why solvency survives:
from `shares ≤ assets`, `floor(t·s/a) ≤ t` (`mul_div_le_self_of_le`, `VaultLemmas.lean:371-376`),
so assets grow at least as fast as shares. For the exact deposit there is the additional bound
`y ≤ assets_to_vault ≤ t` (`le_mul_div_of_le`, `round_trip_le`, `VaultLemmas.lean:379-389`). This
means the user is never charged more than they offered and never receives more shares than the
assets booked. **Not covered:** deposits into an already-insolvent vault (F5). The deposit code does
no solvency check, and `vault_deposit_assets_with_fee_ok` (`Spec.lean:183`) does not conclude
`Solvent` at all.

**`vault_deposit_assets_with_fee_exact` (F1).** The Lean body (`CertoraVault.lean:324-389`) returns
the input `vault` unmodified on **every** path, at `:359, :365, :371, :377, :383, :389`. That
matches the Rust, which never calls `mint_shares` or `add_token`
(`vault_exact_deposit_operations.rs:19-39`), and the theorem `vault_deposit_assets_with_fee_exact_spec`
proves `r.2 = v`. The processor, however, then **transfers `assets_to_vault` into the vault's token
account, transfers `assets_to_fee` to the fee account, and mints `shares_to_user` SPL shares**
(`processor/deposit_with_fee_exact.rs:38-62`). Compare the non-exact fee path, which does update the
vault (`vault_fee_operations.rs:11-12`). Consequences, which follow from reading and are not proved:
the SPL share supply grows while `Vault.shares` does not, and the vault's token balance grows while
`Vault.assets` does not. The permissionless `process_update_reward` sets `Vault.assets` from the
token-account balance (`processor/update_reward.rs:18-22`), so it would later book the deposit as a
"reward" to the existing shareholders. Meanwhile `Vault.shares` undercounts the outstanding shares,
so the last redeemers can hit `vault_redeem_shares_too_many` (`n > Vault.shares`, `Spec.lean:542`).
The solvency theorem for this operation (`Spec.lean:424`) is **true only because nothing changes**,
and it says nothing about the real token flows. INFER recorded this as an edge case ("mutates
nothing even though its processor transfers assets and mints shares") rather than a defect. **I
report it as a candidate defect for the owner to decide.**

**`vault_redeem_shares`.** The Lean (`CertoraVault.lean:612-651`) is convert, then `burn_shares`,
then `del_token`, matching `vault_operations.rs:30-40`. `vault_redeem_shares_ok` (`Spec.lean:441`)
needs **no** solvency hypothesis. Its `Solvent r.2` on `Ok` comes from `del_token` running
`check_invariant` on the post-write vault (`CertoraVault.lean:573`). So a successful redeem is solvent
because the program checks. Liveness (`vault_redeem_shares_succeeds`, `Spec.lean:486`) is the
non-trivial direction: on a solvent vault, redeeming `n ≤ shares` **never trips** that check and
never overflows. The arithmetic fact behind it is `n·a/s + s ≤ a + n` for `n ≤ s ≤ a`
(`redeem_solvent`, `VaultLemmas.lean:400-409`): floor rounding is always in the vault's favour. The
one excluded corner (`shares = 0 < assets`) really does fail, and `vault_redeem_shares_zero_shares`
proves it (see §3 N2).

**`vault_update_reward`.** The Lean (`CertoraVault.lean:656-691`) matches `vault_operations.rs:42-52`,
including the `reward > 0` skip at `:667`/`:682-685`. `checked_sub(new_amt, assets)` fails iff
`new_amt < assets`, and in that case the theorem says `Err(MathOverflow)` with the vault unchanged.
Otherwise `assets' = new_amt`. Reward can only increase `assets`, so solvency holds in every outcome.

**`vault_process_slash`.** The Lean (`CertoraVault.lean:696-713`) is a single `del_token`. The theorem
gives an exact characterisation: `Ok` ⇔ `shares + a ≤ assets` (`Spec.lean:673-685`). It also pins
the `Err` sub-cases: `a > assets` gives `MathOverflow` with no write (`:704`), and
`assets < shares + a` gives `GuardFail` **after** the write (`:722`).

**`vault_collect_fee`.** The Lean (`CertoraVault.lean:476-485`) reads `fee_amount`, zeroes it, and
returns it as `assets_to_user`. There is no failure path, and the theorem has `.Err _ => False`
(`Spec.lean:768`). F6 notes a latent oddity in its processor that is outside the proof scope.

---

## 4.7 What "proved" does **not** mean here

- It does not cover the **processors, loaders, SPL CPIs, signer/owner checks, PDA derivation, or
  `Vault::validate`**. None were translated (§2). F1 and F6 are exactly the kind of issue that lives
  at that boundary.
- It does not prove the **extraction crate equals the original**. That rests on differential tests
  (F4).
- It does not prove anything about the **real constructor** (F3), or about deposits from an
  **insolvent** vault (F5).
- The `_ok` theorems put `True` on `Err`, so they say **nothing about the in-memory state after a
  failed deposit or redeem**. Those states are covered only through `committed`, i.e. through the
  Solana revert (F2).

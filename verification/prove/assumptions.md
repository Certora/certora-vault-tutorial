# Trusted assumptions — Spec campaign (`certora_vault`)

**None.** All 29 theorems in `lean/CertoraVault/Spec/Spec.lean` are proved against the Lean kernel
with no `axiom` and no `sorry`; `#print axioms` reports only `propext`, `Classical.choice`,
`Quot.sound`. No `Assumptions.lean` module was needed.

Supporting `@[step]` specs (`mul_div_floor`, both conversions, `mint_shares`, `add_token`,
`burn_shares`, `del_token`, `check_invariant`, `fee_in_bps`, `FeeBps::apply`) and the pure
arithmetic facts (conversion bounds, round-trip, redeem liveness/solvency) live in
`lean/CertoraVault/VaultLemmas.lean`.

No theorem was found false; no refutations.

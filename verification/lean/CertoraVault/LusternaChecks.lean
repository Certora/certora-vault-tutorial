/-
Lusterna mechanical checks — harness-run GATES over a declaration's ELABORATED type (never its proof
text; they run on FORMALISE's `sorry`-bodied statements and PROVE's finished ones alike). The harness
materialises this file into every crate's lean tree (`lean/<Crate>/LusternaChecks.lean`, see
`lean._write_lint_tool`) — harness-owned like the lakefile, so do not edit or delete it — then drives
each check ITSELF with a throwaway `#eval` and gates on the emitted `LUSTERNA_CHECK {json}` records.
NO agent invokes or interprets these: a finding is a fact the harness acts on, not a prompt for a
judge to weigh. Each check is named by the `"check"` field its records carry.

THE SPEC GATE — `checkSpecGate` (run by `lean.check_spec_gate`) BLOCKS FORMALISE. Blocking is sound
ONLY because each theorem DECLARES its family (see `spec_schemas.lean`): the shape rules run only where
a CHECKED property was claimed (`@[lusterna]`). A finding returns to FORMALISE as a concrete critique
naming the rule, and the stage re-runs.

  • `schema_conformance` — verify a `@[lusterna]` theorem has THE ONE CHECKED SHAPE: an Aeneas WP
    triple over a declared target, `f args ⦃ r => post r ⦄` (`Aeneas.Std.WP.spec`), whose
    postcondition (1) MENTIONS the produced value `r`, (2) is a PURE proposition — no nested
    `Aeneas.Std.Result` measurement that could fail OPEN — and (3) genuinely CONSTRAINS `r` rather than
    assuming its own conclusion (a `Q → Q` tautology, possibly hidden behind a predicate / connective /
    structure field). The triple form is the load-bearing choice: `WP.spec` is `False` on `fail`/`div`
    (`spec_fail`/`spec_div`), so it asserts the call is TOTAL and delivers single-execution,
    fail-safety of the call itself, and a fresh un-pinned output FOR FREE — nothing but the
    postcondition needs vetting, and the check keys only on the stable `WP.spec` name, immune to how
    the `Result` monad is encoded. A single-execution property written the weaker equational way
    (`f args = ok y → …`), or any relational/two-run property, is an exempt `@[lusterna_lemma]`.

THE STANDALONE CHECKS — each driven by its own `lean.py` counterpart, NOT part of the FORMALISE gate:

  • `checkAxioms` (`lean.check_axioms`) — the AUTHORITATIVE established-vs-tainted verdict, via
    `Lean.collectAxioms` (the exact function `#print axioms` calls).
  • `checkDefAxioms` (`lean.target_footprint_gate`) — the TRANSLATE-time taint gate over target `def`s.
  • `dumpStatementTypes` (`lean.check_statement_drift`) — per-theorem type fingerprint (FORMALISE→PROVE).
  • `checkImplReference` (`lean.impl_references`) — three-way: does a theorem run a translated
    function, merely name a translated type/constant, or reference the translation not at all?
  • `checkAnchorReference` (`lean.check_grounding`) — is each INFER anchor measured by some theorem?
  • `checkAnchorCheckedReference` (`lean.check_anchor_coverage`) — is each anchor measured by a CHECKED
    (`@[lusterna]`) triple? The governance floor: the checked family may not be quietly emptied.
  • `checkAssumptionLegitimacy` (`lean.legitimacy_check`) — a trusted axiom may not reference a target.
  • `refutationPurity` (`lean.verify_refutations`) — a `<name>__refuted` counterexample rests on no
    `sorryAx`.

See docs/skills/mechanical-checks.md for the spec-gate rules and worked examples.
-/
import Aeneas
import Lean.Util.CollectAxioms   -- `Lean.collectAxioms`, the exact function `#print axioms` uses
-- `CertoraVault` is substituted with the crate's lib name by `lean._write_lint_tool`. The
-- schemas file is a SUBMODULE of the crate lib (the lakefile globs `.andSubmodules <lib>` and
-- nothing else), so its module path is necessarily crate-specific and cannot be hardcoded here.
import CertoraVault.LusternaSchemas
open Lean Elab Meta

namespace Lusterna.Checks

private def maxTermDepth : Nat := 256

private def jesc (s : String) : String :=
  (((s.replace "\\" "\\\\").replace "\"" "\\\"").replace "\n" " ").replace "\t" " "

private def emit (s : String) : MetaM Unit := IO.println ("LUSTERNA_CHECK " ++ s)

private def lastComp (n : Name) : String := match n with | .str _ s => s | _ => ""

private def ppTrunc (e : Expr) : MetaM String := do
  let s := (← ppExpr e).pretty
  let cs := s.toList
  return if cs.length > 220 then String.ofList (cs.take 220) ++ " ..." else s

/-- Trusted by SOURCE MODULE, never by declaration name. The harness's lakefile builds `lean/<Crate>/**`
and nothing else, so generated code always lands in a `<Crate>.…` module however its namespaces are
spelled. A declaration with NO module — elaborated in an ad hoc driver — is treated as project-local. -/
private def trustedModulePrefixes : Array Name :=
  #[`Mathlib, `Lean, `Init, `Std, `Batteries, `Aeneas, `Qq, `Aesop, `Plausible, `ImportGraph,
    `LeanSearchClient, `Cli]

private def isTrustedDecl (n : Name) : MetaM Bool := do
  let some mod := (← getEnv).getModuleFor? n | return false
  return trustedModulePrefixes.any (fun p => p.isPrefixOf mod)

/-- `c` ends in `t` at a component boundary. Maps INFER's `a::b::c` target patterns onto `a.b.c`;
written on strings so Lean's `_private.<Module>.0.<Name>` mangling still matches. -/
private def nameHasSuffix (c t : Name) : Bool :=
  let cs := toString c
  let ts := toString t
  cs == ts || cs.endsWith ("." ++ ts)

/-- What the predicate walk found at a constant. `notPredicate` means "nothing to look at here";
`uninspectable` means the opposite and must be treated conservatively. -/
private inductive PredBody where
  | body (value : Expr)
  | constructors (ctors : Array Name) (numParams : Nat)
  | notPredicate
  | uninspectable (kind : String)

private def predicateBody? (n : Name) : MetaM PredBody := do
  if ← isTrustedDecl n then return .notPredicate
  let info ← getConstInfo n
  let isPropValued ← forallTelescope info.type fun _ body => do
    match ← whnf body with
    | .sort .zero => return true
    | _ => return false
  unless isPropValued do return .notPredicate
  match info with
  | .defnInfo dv   => return .body dv.value
  | .axiomInfo _   => return .uninspectable "axiom"
  | .opaqueInfo _  => return .uninspectable "opaque"
  | .inductInfo iv => return .constructors iv.ctors.toArray iv.numParams
  | _              => return .notPredicate

private partial def conjuncts (e : Expr) : Array Expr :=
  match e.getAppFnArgs with
  | (``And, #[a, b]) => conjuncts a ++ conjuncts b
  | _ => #[e]

/-- `Aeneas.Std.Result _`, decided by TYPE after `whnf` — never by how the type was spelled, and by
NAME, so it is robust to the `Result` ITree encoding. `core.result.Result` (Rust's own `Result`, a
value INSIDE the monad) is a DIFFERENT constant and is deliberately not matched. -/
private def isResultTy (t : Expr) : MetaM Bool := do
  let t ← try whnf t catch _ => pure t
  match t.getAppFn with
  | .const c _ => return c == ``Aeneas.Std.Result
  | _ => return false

/-- An Aeneas Hoare triple `e ⦃ fun y => … ⦄` = `Aeneas.Std.WP.spec e p` (or `dspec`). Keyed on the
stable `WP.spec` name — the recognition primitive for the checked schema. -/
private def isTripleLike (e : Expr) : Bool :=
  match e.getAppFn with
  | .const n _ => n == ``Aeneas.Std.WP.spec || n == ``Aeneas.Std.WP.dspec
  | _ => false

/-- `true` iff `e` contains a triple `⦃ ⦄` whose computation runs one of `targets`. The triple's
computation is its second-to-last argument (`WP.spec {α} x p`); `x` must use a target constant. -/
private partial def tripleOverTarget (targets : Array Name) (e : Expr) : Bool :=
  if isTripleLike e
     && (let sargs := e.getAppArgs
         sargs.size ≥ 2
         && sargs[sargs.size - 2]!.getUsedConstants.any (fun c => targets.any (nameHasSuffix c)))
  then true
  else match e with
    | .app f a          => tripleOverTarget targets f || tripleOverTarget targets a
    | .forallE _ t b _  => tripleOverTarget targets t || tripleOverTarget targets b
    | .lam _ t b _      => tripleOverTarget targets t || tripleOverTarget targets b
    | .letE _ t v b _   => tripleOverTarget targets t || tripleOverTarget targets v
                             || tripleOverTarget targets b
    | .mdata _ b        => tripleOverTarget targets b
    | .proj _ _ b       => tripleOverTarget targets b
    | _                 => false

/-- The output value(s) of a success token `ok …` — the EXPLICIT arguments of `Result.ok`. Recognised
by ROLE, not by datatype shape: in this toolchain `Aeneas.Std.Result.ok` is a plain `def` into the
`ITree` monad, not an inductive constructor, so we key on the stable name component `ok` and take its
explicit arguments as the produced value. -/
private def okValue? (e : Expr) : MetaM (Option (Array Expr)) := do
  let .const cname _ := e.getAppFn | return none
  unless lastComp cname == "ok" do return none
  let info ← getConstInfo cname
  let args := e.getAppArgs
  forallBoundedTelescope info.type (some args.size) fun bs _ => do
    let mut out : Array Expr := #[]
    for i in [0:args.size] do
      let expl ← if i < bs.size then
          pure ((← bs[i]!.fvarId!.getDecl).binderInfo == .default)
        else pure true
      if expl then out := out.push args[i]!
    return (if out.isEmpty then none else some out)

/-- `some (fn, args, outputs)` iff `e` is `f a1..an = ok y` (either orientation) over the Aeneas
failure monad (`isResultTy`, name-based). The equational MEASUREMENT form — used by the anchor checks;
the CHECKED schema itself keys on the stronger triple form. -/
private def definingEq? (e : Expr) : MetaM (Option (Expr × Array Expr × Array Expr)) := do
  let some (ty, lhs, rhs) := e.eq? | return none
  unless ← isResultTy ty do return none
  if let some outs ← okValue? rhs then return some (lhs.getAppFn, lhs.getAppArgs, outs)
  if let some outs ← okValue? lhs then return some (rhs.getAppFn, rhs.getAppArgs, outs)
  return none

/-- `some detail` iff an `Aeneas.Std.Result`-valued MEASUREMENT is reachable in `e`, unfolding
project-local predicate definitions and Prop-valued inductives. `none` = a PURE proposition: it names
no fallible computation, so nothing in it can fail OPEN. -/
private partial def resultMeasurement? (e : Expr) (seen : NameSet) (fuel : Nat)
    : MetaM (Option String) := do
  if fuel == 0 then return some "the claim nests deeper than the walk budget"
  match ← (try pure (some (← inferType e)) catch _ => pure none) with
  | some ty => if ← isResultTy ty then return some (← ppTrunc e)
  | none    => pure ()
  match e with
  | .mdata _ b      => resultMeasurement? b seen fuel
  | .proj _ _ b     => resultMeasurement? b seen (fuel - 1)
  | .lam n t b bi | .forallE n t b bi =>
    if let some r ← resultMeasurement? t seen (fuel - 1) then return some r
    withLocalDecl n bi t fun x => resultMeasurement? (b.instantiate1 x) seen (fuel - 1)
  | .letE n t v b _ =>
    if let some r ← resultMeasurement? t seen (fuel - 1) then return some r
    if let some r ← resultMeasurement? v seen (fuel - 1) then return some r
    withLetDecl n t v fun x => resultMeasurement? (b.instantiate1 x) seen (fuel - 1)
  | _ =>
    for a in e.getAppArgs do
      if let some r ← resultMeasurement? a seen (fuel - 1) then return some r
    let .const cn _ := e.getAppFn | return none
    if seen.contains cn then return none
    let seen := seen.insert cn
    match ← predicateBody? cn with
    | .body _ =>
      match ← Meta.unfoldDefinition? e with
      | some u => resultMeasurement? u seen (fuel - 1)
      | none   => return none
    | .constructors ctors numParams =>
      let args := e.getAppArgs
      for c in ctors do
        let ct ← instantiateForall (← getConstInfo c).type (args.extract 0 (min numParams args.size))
        if let some r ← resultMeasurement? ct seen (fuel - 1) then return some r
      return none
    | .uninspectable kind => return some s!"a claim rests on an {kind} predicate `{cn}` with no inspectable body"
    | .notPredicate       => return none

/-- Does the postcondition `e` genuinely CONSTRAIN the produced value — more than a tautology the
prover gets for free? Peels through `∧`, named project-local predicates, and Prop-valued structure
fields; a leaf that is `True`, a reflexive equation, or an implication `A → B` whose consequent is
already among its antecedent's conjuncts (`Q → Q`, self-assuming) says nothing. Anything else — a real
equation/inequality, a genuine conditional, an opaque predicate — is taken to constrain (conservative:
a false "constrains" costs a re-read, a false "vacuous" would wrongly block). This is exactly what the
triple's own success requirement does NOT cover. -/
private partial def postConstrains? (e : Expr) (fuel : Nat) : MetaM Bool := do
  if fuel == 0 then return true
  if e.isConstOf ``True then return false
  if let some (_, l, r) := e.eq? then
    if l == r then return false
  match e.getAppFnArgs with
  | (``And, #[a, b]) => return (← postConstrains? a (fuel - 1)) || (← postConstrains? b (fuel - 1))
  | _ =>
    match e with
    | .forallE _ dom body bi =>
      if bi == .default && (← Meta.isProp dom) && !body.hasLooseBVars then
        let ac := conjuncts dom
        let bc := conjuncts body
        if bc.all (fun bx => ac.any (fun ax => ax == bx)) then return false
        return (← postConstrains? body (fuel - 1))
      else
        return true
    | _ =>
      let .const cn _ := e.getAppFn | return true
      match ← predicateBody? cn with
      | .body _ =>
        match ← Meta.unfoldDefinition? e with
        | some u => postConstrains? u (fuel - 1)
        | none   => return true
      | .constructors ctors numParams =>
        let args := e.getAppArgs
        for c in ctors do
          let ct ← instantiateForall (← getConstInfo c).type (args.extract 0 (min numParams args.size))
          let fieldConstrains ← forallTelescope ct fun flds _ => do
            for f in flds do
              if ← postConstrains? (← inferType f) (fuel - 1) then return true
            return false
          if fieldConstrains then return true
        return false
      | .uninspectable _ => return true
      | .notPredicate    => return true

/-! ══ `schema_conformance` — verify the DECLARED intent ═══════════════════════════════════════════

FORMALISE declares each theorem's family (`@[lusterna]` checked, or `@[lusterna_lemma "why"]` exempt),
and this verifies it. A theorem that fails the shape IT DECLARED is a FACT, not a judgement — which is
what lets it block. WHAT THIS DOES NOT CHECK: whether the declared schema is the RIGHT one, and whether
the property is the right property — that stays with SPEC-JUDGE. -/
def checkSchemaConformance (qn : Name) (targets : Array Name) : MetaM Nat := do
  let env ← getEnv
  let report (rule detail : String) : MetaM Unit :=
    emit s!"\{\"check\": \"schema_conformance\", \"theorem\": \"{qn}\", \"schema\": \"checked\", \"rule\": \"{rule}\", \"detail\": \"{jesc detail}\"}"
  let schemas := Lusterna.Schemas.declaredSchemas env qn
  if schemas.isEmpty then
    report "no_schema_declared"
      "every spec theorem must declare a family: a checked property is @[lusterna], a supporting lemma is @[lusterna_lemma \"why\"]"
    return 1
  if schemas.size > 1 then
    report "multiple_schemas_declared"
      s!"declares {schemas.size} families ({String.intercalate ", " schemas.toList}); a theorem is either a checked property or an exempt lemma, not both"
    return 1
  let schema := schemas[0]!
  if (`_private).isPrefixOf qn then
    report "private_declaration"
      "a private theorem's schema annotation may not survive module export; spec theorems must be public"
    return 1
  if schema == "lemma" then
    return 0
  if targets.isEmpty then
    IO.println s!"LUSTERNA_CHECK_SKIPPED \{\"check\": \"schema_conformance\", \"theorem\": \"{qn}\", \"schema\": \"checked\", \"reason\": \"no targets given\"}"
    return 0
  -- ── THE ONE CHECKED SHAPE: a WP triple over a declared target with a mentioning, pure,
  -- non-vacuous postcondition. Single-execution, totality and output-freshness come free from
  -- `WP.spec`; only the postcondition is vetted.
  let ci ← getConstInfo qn
  forallTelescope (← instantiateMVars ci.type) fun _ concl => do
    let concl ← instantiateMVars concl
    unless isTripleLike concl do
      report "not_a_target_triple"
        "a checked property must be an Aeneas WP triple over a declared target — `f args ⦃ r => post r ⦄` (asserting the call is total); got a non-triple conclusion. Write it as a triple, or declare @[lusterna_lemma \"why\"]."
      return 1
    unless tripleOverTarget targets concl do
      report "triple_not_over_target"
        "the triple's computation does not run any declared target function; a checked property must measure a target"
      return 1
    let sargs := concl.getAppArgs
    unless sargs.size ≥ 3 do
      report "not_a_target_triple" "malformed triple (missing its postcondition)"
      return 1
    let α := sargs[sargs.size - 3]!
    let post := sargs[sargs.size - 1]!
    withLocalDeclD `r α fun r => do
      let body := (mkApp post r).headBeta
      let mut n := 0
      unless body.containsFVar r.fvarId! do
        report "conclusion_ignores_output"
          "the postcondition says nothing about the value the target produced"
        n := n + 1
      if let some meas ← resultMeasurement? body {} maxTermDepth then
        report "claim_not_failsafe"
          s!"the postcondition runs a fallible measurement ({meas}); a checked triple's postcondition must be a pure proposition over the produced value — bind the measurement inside the triple's computation, or state it as a @[lusterna_lemma]"
        n := n + 1
      unless ← postConstrains? body maxTermDepth do
        report "vacuous_claim"
          "the postcondition is vacuous — it assumes its own conclusion (a tautology that holds for free); state a claim the produced value must actually satisfy"
        n := n + 1
      return n

/-! ══ THE SPEC GATE — what the harness itself enforces ══════════════════════════════════════════

With the checked shape fixed to a WP triple, the triple structurally rules out the multi-hypothesis
"assumed postcondition" attacks the old fixpoint hunted (there is no execution hypothesis and no free
output variable to pin), so `schema_conformance` — the shape plus the postcondition's fail-safety and
non-vacuity — IS the whole gate. -/
def checkSpecGate (qn : Name) (targets : Array Name) : MetaM Nat :=
  checkSchemaConformance qn targets

/-! ══ `axioms` — the AUTHORITATIVE established-vs-tainted verdict ═════════════════════════════════ -/

def stdAxioms : Array Name := #[``propext, ``Classical.choice, ``Quot.sound]

def checkAxioms (thms : Array Name) (declared : Array Name) : MetaM Unit := do
  for t in thms do
    let axs ← Lean.collectAxioms t
    let nonStd := axs.filter (fun a => !stdAxioms.contains a)
    let status :=
      if nonStd.isEmpty then "clean"
      else if nonStd.all (fun a => declared.contains a) then "assumed"
      else "tainted"
    let used := String.intercalate ", " (nonStd.toList.map (fun a => "\"" ++ toString a ++ "\""))
    emit s!"\{\"check\": \"axioms\", \"theorem\": \"{t}\", \"status\": \"{status}\", \"used\": [{used}]}"

/-! ══ `def_axioms` — the TRANSLATE-time TAINT gate over target defs ═══════════════════════════════ -/

def checkDefAxioms (targets : Array Name) : MetaM Unit := do
  let env ← getEnv
  for (n, info) in env.constants.toList do
    match info with
    | .defnInfo _ =>
      if targets.any (nameHasSuffix n) then
        unless (← isTrustedDecl n) do
          let axs ← Lean.collectAxioms n
          let nonStd := axs.filter (fun a => !stdAxioms.contains a)
          let op := String.intercalate ", " (nonStd.toList.map (fun a => "\"" ++ toString a ++ "\""))
          emit s!"\{\"check\": \"def_axioms\", \"def\": \"{n}\", \"opaque\": [{op}]}"
    | _ => pure ()

/-! ══ `statement_type` — per-theorem type fingerprint for the statement-drift detector ═══════════════ -/

def dumpStatementTypes (module : Name) : MetaM Unit := do
  let env ← getEnv
  for (n, info) in env.constants.toList do
    match info with
    | .thmInfo _ =>
      unless n.isInternal do
        if env.getModuleFor? n == some module then
          let ty ← ppTrunc info.type
          emit s!"\{\"check\": \"statement_type\", \"theorem\": \"{n}\", \"hash\": \"{info.type.hash}\", \"type\": \"{jesc ty}\"}"
    | _ => pure ()

/-! ══ `impl_ref` — does a theorem VERIFY THE IMPLEMENTATION? ═══════════════════════════════════════ -/

def checkImplReference (thms : Array Name) (translationModules : Array Name) : MetaM Unit := do
  let env ← getEnv
  -- A used constant is RUNNABLE translated CODE iff it is a `def`/`opaque` whose type is a function
  -- (takes arguments) or a `Result` computation. A translated TYPE (`Channel`, an inductive) and a
  -- plain VALUE constant (`BPS_DENOMINATOR : U32`, a nullary non-`Result` def) are NOT code: a
  -- statement merely NAMES them without executing anything. (An `axiom` is an assumed primitive, not
  -- established code, so it never counts as `function` either.)
  let codeTy (ty : Expr) : Bool := ty.isForall || ty.getUsedConstants.contains ``Aeneas.Std.Result
  let isCode (c : Name) : Bool :=
    match env.find? c with
    | some (.defnInfo vi)   => codeTy vi.type
    | some (.opaqueInfo vi) => codeTy vi.type
    | _                     => false
  for t in thms do
    let some ci := env.find? t | continue
    let inTrans (c : Name) : Bool :=
      match env.getModuleFor? c with
      | some m => translationModules.contains m
      | none   => false
    let used := ci.type.getUsedConstants
    -- THREE-WAY. `function`: runs a translated function and can constrain its result — the strong
    -- "verifies the implementation" signal. `surface`: names a translated type or value constant but
    -- executes nothing — weaker than the headline implies. `abstract`: no reference to the translation.
    let kind :=
      if used.any (fun c => inTrans c && isCode c) then "function"
      else if used.any inTrans then "surface"
      else "abstract"
    emit s!"\{\"check\": \"impl_ref\", \"theorem\": \"{t}\", \"kind\": \"{kind}\"}"

/-! ══ `anchor_bridge` — a reconstructed measurement must be tied to the real function ═════════════

Each ANCHOR function INFER names must be MEASURED by at least one theorem: run and its result
constrained, either `anchor … = ok …` or a `⦃ ⦄` triple over it. That it exists at all is mechanical;
whether the bridge is faithful stays with SPEC-JUDGE. -/
def checkAnchorReference (thms : Array Name) (anchors : Array Name) : MetaM Unit := do
  let env ← getEnv
  for a in anchors do
    let anchorMeasured (e : Expr) : MetaM Bool := do
      match ← definingEq? e with
      | some (fn, _, _) => match fn with
                           | .const c _ => return nameHasSuffix c a
                           | _          => return false
      | none => return false
    let referenced ← thms.anyM fun t => do
      let some ci := env.find? t | return false
      if tripleOverTarget #[a] ci.type then return true
      forallTelescope ci.type fun xs concl => do
        for x in xs do
          if ← anchorMeasured (← instantiateMVars (← inferType x)) then return true
        anchorMeasured (← instantiateMVars concl)
    emit s!"\{\"check\": \"anchor_bridge\", \"anchor\": \"{a}\", \"referenced\": {referenced}}"

/-! ══ `anchor_checked` — the checked family may not be quietly emptied ═════════════════════════════

The governance floor. `anchor_bridge` accepts a bridge by ANY established theorem, including an exempt
`@[lusterna_lemma]`. This is stricter: each anchor must be measured by a CHECKED (`@[lusterna]`) triple
— so a campaign cannot route its core properties into the exempt family (accidentally, or to dodge the
shape gate) and still report green. An anchor with none surfaces to the harness, which requires a
checked property or a verified refutation. Emits `{check:"anchor_checked", anchor, checked_referenced}`. -/
def checkAnchorCheckedReference (thms : Array Name) (anchors : Array Name) : MetaM Unit := do
  let env ← getEnv
  for a in anchors do
    let referenced ← thms.anyM fun t => do
      unless (Lusterna.Schemas.declaredSchemas env t).contains "checked" do return false
      let some ci := env.find? t | return false
      return tripleOverTarget #[a] ci.type
    emit s!"\{\"check\": \"anchor_checked\", \"anchor\": \"{a}\", \"checked_referenced\": {referenced}}"

/-! ══ `refutation` — purity of a counterexample ════════════════════════════════════════════════════ -/

def refutationPurity (refs : Array Name) : MetaM Unit := do
  for r in refs do
    let axs ← Lean.collectAxioms r
    emit s!"\{\"check\": \"refutation\", \"lemma\": \"{r}\", \"pure\": {!axs.contains ``sorryAx}}"

/-! ══ `assumption_legitimacy` — the trusted base may not relax a goal ═════════════════════════════ -/

def checkAssumptionLegitimacy (axioms : Array Name) (targets : Array Name) : MetaM Unit := do
  let env ← getEnv
  let isCrateDef (c : Name) : MetaM Bool := do
    if ← isTrustedDecl c then return false
    return match env.find? c with | some (.defnInfo _) => true | _ => false
  for ax in axioms do
    let some (.axiomInfo av) := env.find? ax | continue
    let hits ← av.type.getUsedConstants.filterM fun c => do
      if ← isCrateDef c then return targets.isEmpty || targets.any (nameHasSuffix c)
      else return false
    unless hits.isEmpty do
      let tl := String.intercalate ", " (hits.toList.map (fun t => "\"" ++ toString t ++ "\""))
      emit s!"\{\"check\": \"assumption_legitimacy\", \"axiom\": \"{ax}\", \"targets\": [{tl}], \"reason\": \"axiom `{ax}` references target function(s) it may not — the trusted base may hold facts only about the substrate, never a property of a target (that would relax a goal); prove it, do not assume it\"}"

end Lusterna.Checks

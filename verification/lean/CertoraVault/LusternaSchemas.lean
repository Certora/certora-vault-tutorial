/-
Lusterna SPEC SCHEMAS — the attributes FORMALISE uses to DECLARE what each theorem IS.

The author DECLARES the family, and `checkSchemaConformance` verifies the declaration. A conformance
failure on a DECLARED family is a fact rather than a judgement, which is what lets the harness gate on
it — unlike a checker that hunts for a bad shape, whose SILENCE is ambiguous ("clean, or nothing I
recognise").

  @[lusterna]                        -- a CHECKED property: a WP triple over a target
  @[lusterna_lemma "why"]            -- a supporting lemma, deliberately outside the checked shape

These TWO families are the whole taxonomy — there is no third, and no invariant/hoare distinction. A
CHECKED property (`@[lusterna]`) is an Aeneas WP triple over a declared target, `f args ⦃ r => post r ⦄`,
whose postcondition MENTIONS the produced value, is a PURE proposition (no nested `Result` measurement
that could fail OPEN), and genuinely CONSTRAINS the value (not a self-assuming tautology). The triple
form gives single execution, totality and output-freshness for free, so `schema_conformance` need only
vet the postcondition. Whether that one checked property is, on its own or with others, an INVARIANT of
the system is a PROVE/REPORT question, never a per-theorem label: the gate checks properties, not
invariance.

╔══════════════════════════════════════════════════════════════════════════════════════════════╗
║ HARNESS-OWNED, and the attribute set is APPEND-ONLY. Do not edit or delete this file.         ║
║                                                                                              ║
║ `lean._write_lint_tool` rewrites it on every `setup_lake`, while PRIOR campaigns' spec        ║
║ modules are restored immutable from `refs/lusterna/pristine` and reference these attributes   ║
║ by name. So an old spec module is always rebuilt against a NEWLY WRITTEN copy of this file:   ║
║ renaming or removing an attribute breaks every prior campaign, and the pipeline cannot repair ║
║ them, because editing them is exactly what `_restore_prior_specs` forbids. A campaign written  ║
║ before these two families existed (with `@[lusterna_invariant]`/`_hoare`/`_freeform`) must be  ║
║ RE-TAGGED to `@[lusterna]`/`@[lusterna_lemma]` in its pristine baseline before it can rebuild. ║
║                                                                                              ║
║ Attributes may be ADDED. They may never be renamed or removed.                                ║
╚══════════════════════════════════════════════════════════════════════════════════════════════╝

Deliberately depends on `Lean` ALONE — not Aeneas. This is the file spec modules import, so it must
stay tiny, fast and stable; the churn belongs in `spec_checks.lean`, which imports this one.
-/
import Lean

open Lean

/-- `@[lusterna_lemma "reason"]`. Parametric because a supporting lemma opts OUT of the checked
shape, and an opt-out should have to say why — the reason is reported, so a spec cannot quietly
become all-lemma. -/
syntax (name := lusterna_lemma) "lusterna_lemma " str : attr

namespace Lusterna.Schemas

/-- A CHECKED property: a WP triple over a declared target, `f args ⦃ r => post r ⦄`, whose
postcondition mentions the produced value, is a pure `Prop` (no nested `Result` measurement that could
fail OPEN), and genuinely constrains it — see Lusterna `schema_conformance`. Whether the property is an
INVARIANT is a PROVE/REPORT question, not this label. -/
initialize checkedAttr : TagAttribute ←
  registerTagAttribute `lusterna
    "this theorem is a Lusterna-checked property (see Lusterna `schema_conformance`)"

/-- A SUPPORTING lemma, declared out of scope. The payload is the justification. -/
initialize lemmaAttr : ParametricAttribute String ←
  registerParametricAttribute {
    name := `lusterna_lemma
    descr := "a supporting lemma, deliberately outside the checked shape; the string says why"
    getParam := fun _ stx => do
      match stx with
      | `(attr| lusterna_lemma $s:str) =>
        let r := s.getString
        -- `String.trim` is mid-transition to `String.Slice` in this toolchain and warns on use; "no
        -- non-whitespace character" says the same thing without the deprecation noise, which would
        -- otherwise surface in every crate's build.
        if !r.any (fun c => !c.isWhitespace) then
          throwError "@[lusterna_lemma] needs a non-empty reason for opting out of the checked shape"
        return r
      | _ => Elab.throwUnsupportedSyntax
  }

/-- The family `decl` declares: "checked" (`@[lusterna]`) or "lemma" (`@[lusterna_lemma]`). MORE THAN
ONE is a conformance failure the caller reports — this returns what is there rather than picking a
winner, because silently preferring one would let a theorem declare two and be checked against the
laxer. -/
def declaredSchemas (env : Environment) (decl : Name) : Array String := Id.run do
  let mut out : Array String := #[]
  if checkedAttr.hasTag env decl then out := out.push "checked"
  if (lemmaAttr.getParam? env decl).isSome then out := out.push "lemma"
  return out

/-- The `@[lusterna_lemma]` justification, if any. -/
def lemmaReason (env : Environment) (decl : Name) : Option String :=
  lemmaAttr.getParam? env decl

end Lusterna.Schemas

# ADR 0005 — Assertions and Conclusions as distinct model layers

**Status:** Accepted · 2026-09-21

## Context

Every genealogy tool surveyed stores a researcher's conclusion and attaches sources to it. None
can represent *disagreement between sources* as data.

The Genealogical Proof Standard requires that contradictory evidence be resolved and that the
resolution be soundly reasoned and written down. A model with nowhere to put the contradiction
makes that requirement unimplementable.

## Decision

Two layers, both first-class and both persisted:

```
Assertion  { subject, predicate, value, source_citation, confidence,
             asserted_by, asserted_at }

Conclusion { subject, predicate, value, supported_by[], contradicted_by[],
             reasoning, decided_by, decided_at }
```

Assertions record what a source says. Conclusions record what the researcher decided and why.
A predicate with conflicting Assertions and no Conclusion is an **open question**, displayed as
such rather than silently resolved.

## Consequences

**Gained.** Contradictions survive instead of being destroyed by the last edit. Merge conflicts
have an honest resolution — "keep both" becomes two Assertions awaiting a Conclusion. Proof
arguments compile from data the system already holds. Imported data carries its provenance
rather than flattening into the importer's opinion. The Hall can show, without any jargon, that
a fact is contested — which teaches research discipline by making it visible.

**Given up.** Real complexity. Every read path needs a resolution step: take the Conclusion, or
the highest-confidence Assertion, or show the conflict. The UI must present this without
overwhelming a casual viewer. Storage grows.

**Mitigations.** A resolved-value projection is materialised and cached, so ordinary reads cost
what they would in a flat model. The Hall shows the resolved value with a quiet contested
marker; the full apparatus lives in the Workshop.

## Alternatives considered

- **Flat fields with attached sources**, as Gramps and everything else does. Simple, and cannot
  express the problem the GPS exists to solve.
- **Conflicts as notes.** Prose, not data — unqueryable, unmergeable, unexportable.
- **A branch per hypothesis.** Conceptually clean, and a usability disaster for anyone who is not
  already fluent in version control.

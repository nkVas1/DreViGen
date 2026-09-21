# ADR 0003 — Review between identities, automatic merge within one

**Status:** Accepted · 2026-09-21

## Context

Participants edit offline for extended periods and then reconcile with a shared family tree.
CRDTs guarantee convergence but not correctness, and the failure modes in genealogy are
semantic: duplicate people created independently, competing birth dates silently resolved by
last-write-wins, a child attached to the wrong family with no syntactic conflict at all.

The project owner's requirement is explicit: a flow resembling a GitHub pull request, with the
cloud tree treated as the verified truth.

## Decision

- **Within one identity** (a user's own laptop, phone and tablet): automatic CRDT merge, silent.
- **Between identities:** a **Contribution** — semantic diff against the canonical tree, changes
  classified as clean / conflicting / suspicious, human review, then merge.
- Conflict resolutions offered: keep mine, keep theirs, **keep both as competing assertions**,
  or park as an open question.

## Consequences

**Gained.** Correctness where it matters. A genuine audit trail. Conflicts become data —
"keep both" produces two Assertions and an unresolved Conclusion, which is what the
Genealogical Proof Standard asks for anyway. Reviewers learn the tree by reviewing it.

**Given up.** Immediacy. Two relatives editing simultaneously do not see each other's work live.
Implementation cost: semantic diff, classification, a review UI, and a notification system that
would all be unnecessary with naive auto-merge.

**Mitigations.** Presence indicators warn that someone else is editing the same person.
Uncontroversial single-fact corrections use a lightweight path that needs no review UI literacy.

## Alternatives considered

- **Pure CRDT auto-merge.** Simple, converges, produces silently wrong genealogy.
- **Server-authoritative locking.** Kills offline work, which is a core requirement.
- **Review everything, including a user's own devices.** Absurd friction for no correctness gain.

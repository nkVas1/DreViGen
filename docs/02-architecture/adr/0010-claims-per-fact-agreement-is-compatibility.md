# ADR 0010 — Claims attach to facts, and agreement means compatibility

**Status:** Accepted · 2026-09-25

## Context

[ADR 0005](./0005-assertion-conclusion-split.md) split the model into Assertions — what a source
says — and Conclusions — what the researcher decided. It left five questions open, and each one
decides the shape of `drevigen-core`:

1. **What carries claims?** Everything, or only some things?
2. **How is a claim addressed?** The ADR writes `subject, predicate, value`. Stored untyped, that
   lets a date be asserted as someone's sex and the compiler will not object.
3. **What is a conflict?** If two assertions conflict whenever their values differ, then a person
   recorded as born "1871" and later found in a register as born "17 April 1871" has an open
   question. That is not a disagreement, it is a refinement, and a model that treats it as one
   fills the contradiction queue with noise until nobody reads it.
4. **What does the Hall show for an open question?** ADR 0005 says "the resolved value with a
   quiet contested marker". Resolved how?
5. **Is an assertion ever deleted?**

## Decision

**1. Claims attach to facts about people and relationships; the evidence apparatus is plain
data.** Names, sex, attributes (occupation, сословие, religion, residence), an event's date, place
and description, who took part in an event and in what role, and who belongs to a family and how
— these are claims, because each is something a source says and a second source may contradict.
Sources, citations, repositories, notes, media and place definitions are not: they are what claims
*cite*. A source's title is not evidence of anything; it is the thing the evidence is in.

**2. Every fact has an identity and one claim set; the claim set is typed.** A fact is owned by an
entity — a person's name, an event's date — and holds `Claims<V>` for exactly one value type.
Multi-valued properties are several facts: a second name is a second fact, not a conflict with the
first. Single-valued properties have at most one fact per owner. Storage and sync see a generic
`(fact, assertion)` shape, which is what the CRDT and the SQLite tables want; code sees
`Claims<RecordedDate>`, which is what makes a date-shaped sex impossible to write.

**3. Two assertions agree when their values are compatible, not when they are equal.** Each value
type states what compatibility means for it:

| Value | Compatible when |
|---|---|
| A date | The two could describe the same day, allowing each its approximation — the `could_coincide` test in `drevigen-date` |
| A name | Every part present in both is the same after normalisation; a part present in only one is a refinement |
| Sex, a role, a relationship kind | Equal |
| A place | Equal. *Hierarchy-aware compatibility — a village and the uyezd containing it — needs the place graph and is deferred; see Consequences.* |

Agreeing assertions resolve to the **most specific** value among them: "17 April 1871" over
"1871", "Иван Петрович Смирнов" over "Иван Смирнов". Only assertions that cannot all be true
together make an open question.

**4. Resolution runs in a fixed order, and a contested value is never reported as settled.**

1. A Conclusion, if the researcher has written one.
2. Otherwise, if every live assertion is compatible with every other: the most specific value,
   reported as **agreed**.
3. Otherwise: **contested**, carrying a *provisional* value for display — the assertion with the
   highest confidence, then one with a citation over one without, then the earliest recorded.

The provisional value exists so the Hall has something to draw. It is always delivered with the
contested state, and nothing may treat it as final: not the layout, not an export, not a merge.

**5. Assertions are append-only.** Withdrawing one records a retraction; the assertion stays, so
the history of what was believed and why survives. Removing a living person's data on request —
the GDPR erasure the research obligates — is a separate operation on the store that removes
everything about that person, and is deliberately not something an ordinary edit can do.

## Consequences

**Gained.** A refinement is not a conflict, so the contradiction queue contains contradictions.
Concurrent edits from two relatives become two assertions on one fact rather than a merge that
drops one — ADR 0005's promise, now with a place to live. The type system keeps values on the
properties they belong to. And the Hall's contested marker means something: when it appears,
the sources genuinely disagree.

**Given up.** Compatibility is logic per value type, and it has to be written and tested for each
one; getting it wrong in the lenient direction hides a real conflict, and in the strict direction
invents one. Every fact carries an identity and a claim set, which costs storage over a flat
record even for the many facts that only ever have one unsourced assertion.

**Deferred, and recorded so it is not forgotten.** Place compatibility is equality for now. A
baptism recorded in "Туношна" and a census recorded in "Ярославский уезд" are compatible — the
village is in the uyezd — but saying so needs the place hierarchy with its validity periods, which
arrives with the place gazetteer. Until then those two are reported as contested, which errs
towards showing a question rather than hiding one.

## Alternatives considered

- **Conflict means inequality.** Simplest to implement and wrong in the way that matters most: the
  commonest thing a researcher does is refine a date or a name, and every refinement would become
  an open question.
- **GEDCOM X facts with a `primary` flag.** Several facts of one type, one marked preferred. That is
  the attached-sources model ADR 0005 rejected: it records which value won and not why, and it
  cannot tell a refinement from a contradiction either.
- **Untyped subject–predicate–value triples throughout.** Maximally flexible, and it moves every
  type error from the compiler to the data. The generic shape is kept where it earns its place —
  storage and sync — and nowhere else.

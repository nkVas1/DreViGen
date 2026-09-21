# DreViGen — Vision & Product Principles

> **Дре**во · **Vi**sual · **Gen**ealogy — a family archive you can navigate like a map,
> keep like an heirloom, and research like a scholar.

---

## 1. The problem

Genealogy software in 2026 splits into two camps that barely talk to each other.

**Consumer platforms** (MyHeritage, Ancestry) are welcoming and beautiful, but they are
subscription funnels: your data lives on someone else's server, the interesting features sit
behind paywalls, the research apparatus is shallow, and exporting your life's work yields a
lossy GEDCOM file. **Serious tools** (Gramps, Ancestris, webtrees) have genuine scholarly depth
— evidence models, citation hierarchies, proof arguments — and interfaces that assume you will
read the manual first. As the Gramps community itself concedes, its learning curve is steep
because it is built by and for serious researchers, with an interface to match.

A qualitative study of 20 genealogists (Siu et al., *Reexamining Technological Support for
Genealogy Research, Collaboration, and Education*, arXiv:2411.07869) found that the field has
been transformed by mass digitisation and home DNA testing, while HCI research on it stalled a
decade ago. The study highlights the widening gap between amateurs and experts, the rising
importance of standardisation and professionalisation, and the critical role software plays in
how genealogists *learn* — not just in how they record.

Nobody has built the tool that serves both groups at once, in the same document, without
asking either one to compromise.

## 2. What DreViGen is

A genealogy application that is:

- **Local-first.** Your tree is a file on your machine. It opens instantly, works on a train,
  and is never held hostage by a subscription.
- **Collaborative with a verified centre.** Each participant keeps a personal working copy and
  accumulates changes locally. When ready, they open a *Contribution* — a review flow modelled
  on a GitHub pull request — against the family's canonical cloud tree. The cloud tree is the
  proven build; local trees are working branches.
- **Fully attributed.** Every fact carries who asserted it, when, from which source, and with
  what confidence. Every change is reversible and inspectable, in the spirit of a wiki history
  and a git log combined.
- **Dual-profile.** One document, two front doors: a *Hall* for family members who want to look
  at photographs and understand how they are related, and a *Workshop* for researchers who need
  citations, conflict resolution, research logs and proof arguments.
- **Everywhere, natively.** Windows, macOS, Linux, iOS and Android as real installed
  applications, plus an installable web version — from a single codebase.
- **Sovereign.** No mandatory paid dependency, no mandatory third-party cloud. Self-hostable in
  one command on a modest VPS.

## 3. The people we build for

### Персона A — «Дедушка» (the primary UX authority)

Sees perfectly well. Is not confused by technology in principle; he is confused by *interfaces*
— by places where the system's model of what is happening differs from his, and nothing on the
screen tells him so. He wants to open the app and immediately see faces he recognises.

**He is the tie-breaker.** When two designs are otherwise equal, the one he would understand
without being told wins. When a feature cannot be made obvious, it belongs in the Workshop, not
on his path.

Research backs this up: older adults measurably prefer high contrast, skeuomorphic cues and
**icons with text labels**, because unlabelled iconography is a frequent source of confusion.
WCAG 2.2's newer criteria — target size, focus appearance, consistent help, redundant entry,
accessible authentication — are written almost exactly for him. We treat them as a floor, not a
ceiling.

### Персона B — «Хранитель» (the family keeper)

Holds the shoebox of photographs. Wants to scan them, put names to faces, write down what
grandmother said, and share the result so relatives in Berlin and Chicago can see it too.
Needs bulk operations that do not feel like data entry, and wants the result to be *beautiful*
— something worth printing and giving away.

### Персона C — «Исследователь» (the genealogist)

Works to the Genealogical Proof Standard: reasonably exhaustive research, complete and accurate
citation of every fact, skilled correlation and analysis of evidence, resolution of contradictory
evidence, and a soundly reasoned written conclusion. Needs the application to *support the
argument*, not merely store the answer — which means first-class conflicting assertions, source
provenance, negative evidence, research plans and logs, and exportable proof arguments.

## 4. Product principles

1. **Обозримость (legibility) before density.** Any screen must answer "where am I, what is
   this, what can I do" within two seconds, at any zoom level.
2. **Progressive disclosure, never progressive confusion.** The Workshop's power is *reachable*
   from the Hall, never *imposed* on it. Complexity appears when asked for and retreats when not.
3. **Nothing is destroyed.** Every edit is an event in an append-only log. Undo is always
   available; history is always inspectable; deletion is tombstoning, not erasure.
4. **Evidence over assertion.** The data model stores what a source *says* separately from what
   the researcher *concludes*. Two sources may disagree; the tree records the disagreement rather
   than silently picking a winner.
5. **The map, not the diagram.** A tree of 40 000 people is a territory. It gets cartographic
   treatment: semantic zoom, level-of-detail symbology, a minimap, named landmarks, a scale.
6. **Motion explains, never decorates.** Every animation answers a question: where did this come
   from, where did it go, what just changed, is the system still working.
7. **Offline is the normal case.** Online is an enhancement. Nothing in the core loop requires a
   network round trip.
8. **Your data is portable by construction.** Lossless GEDCOM 7 import/export, plus a documented
   open container format. Leaving must be as easy as arriving — that is what makes staying a
   choice.
9. **Localised, not translated.** Names, dates, calendars, name order, patronymics, plural rules
   and address formats are locale-aware data problems, not string lookups.
10. **Zero mandatory cost.** Every feature works with free, open-source components. Optional
    paid enhancements (a user's own LLM API key) are strictly additive.

## 5. Explicit non-goals

- **We are not a records marketplace.** DreViGen does not sell access to archives. It links to
  them, cites them, and stores what you found.
- **We do not host a global shared tree.** Each family runs its own canonical tree. No universal
  merge, no "one tree to rule them all" governance problem.
- **We do not sell or process DNA.** We can *import* raw-data match lists and model genetic
  relationships, but sequencing is someone else's business.
- **We do not animate the dead by default.** Photo restoration is offered; synthetic animation of
  ancestors' faces is deliberately not a headline feature. See
  [art-direction.md](./03-design/art-direction.md), section on ethics.

## 6. How we will know it worked

| Dimension | Target |
|---|---|
| Cold start to first face on screen | < 1.5 s on a 2019 mid-range laptop |
| Pan/zoom on a 50 000-person tree | Sustained 60 fps, < 16 ms frame budget |
| Adding a child to an existing person | <= 3 interactions, no modal stack |
| The «Дедушка» test | Opens the app, finds his grandson, views a photo — unassisted, first try |
| GEDCOM 7 round-trip | Byte-level stable for supported structures; documented losses for the rest |
| Sync conflict | Always resolvable in the UI without editing files or losing either side |
| Cost to run for a family of 30 | 0 ₽/month self-hosted beyond an existing VPS |

---

*This document defines intent. Architecture lives in [`docs/02-architecture/`](./02-architecture/),
the research that justifies these choices in [`docs/01-research/`](./01-research/), and the
delivery plan in [`docs/04-planning/roadmap.md`](./04-planning/roadmap.md).*

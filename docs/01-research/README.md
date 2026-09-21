# R&D Dossier

Everything in `docs/02-architecture/` and `docs/03-design/` is downstream of this folder. If a
decision here is wrong, the decision there is wrong too — so each note states **what we found**,
**how confident we are**, and **what it obliges us to do**.

## Method

1. **Survey the field.** Commercial products, open-source projects, academic literature and
   standards bodies — in that order of breadth, reverse order of authority.
2. **Prefer primary sources.** Specifications over blog posts about specifications; papers over
   summaries; registry metadata over remembered version numbers. A source is cited only if
   someone read the part being cited — an abstract proves a paper exists, not a claim.
3. **Verify versions at the point of decision.** Every library version quoted in this dossier was
   read from the npm registry / crates.io on the date noted, not recalled.
4. **Record the obligation.** A finding that changes nothing is trivia. Each note ends with what
   it forces us to build.

## Contents

| Note | Question it answers |
|---|---|
| [`competitive-landscape.md`](./competitive-landscape.md) | What already exists, what it does well, where it fails |
| [`data-standards.md`](./data-standards.md) | How genealogical data is modelled and exchanged in 2026 |
| [`layout-and-rendering.md`](./layout-and-rendering.md) | How to draw and interact with 50 000 people at 60 fps |
| [`sync-and-collaboration.md`](./sync-and-collaboration.md) | How offline edits merge, and how attribution survives merging |
| [`elder-ux-and-accessibility.md`](./elder-ux-and-accessibility.md) | What the evidence says about designing for older adults |
| [`design-language-2026.md`](./design-language-2026.md) | Where visual and motion design actually is in 2026 |
| [`automation-and-ml.md`](./automation-and-ml.md) | Which parts of genealogical labour can be automated, and how |
| [`library.md`](./library.md) | Papers read in full, what each contributed, and what we still want |
| [`sources.md`](./sources.md) | Full bibliography with access dates |

## Confidence notation

Each claim is tagged where it matters:

- **[established]** — standard, specification, or replicated research. Safe to build on.
- **[current]** — true as of the access date, but in a fast-moving area. Re-check before relying.
- **[judgement]** — our engineering opinion, argued but not externally verified.

---

*Dossier opened 2026-09-21.*

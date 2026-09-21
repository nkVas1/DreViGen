# Contributing to DreViGen

## Where the project is right now

Phase 0 — foundation. The architecture is being laid and the risky assumptions are being
tested. **Code contributions are not open yet**, because there is not yet a stable surface to
contribute against.

What *is* open, and genuinely wanted:

- **Argument about the design.** If an [ADR](./docs/02-architecture/adr/) is wrong, say so in an
  issue. A decision that survives a serious objection is stronger; one that does not should
  change.
- **Genealogical domain expertise.** Especially from people who do this professionally, and
  especially about record types, archival practice and research method outside Russia and the
  former Soviet Union.
- **GEDCOM test files.** Real exports from real applications, with anything private removed, are
  the most valuable thing you can send. They go straight into the round-trip test corpus.
- **Accessibility review.** Particularly from people who use assistive technology daily, or who
  support older relatives with software.
- **Translation.** Once the interface strings exist — see [Localisation](#localisation).

Open an issue. There is no template requirement for discussion.

## When code opens

From Phase 1, the following applies.

### Getting set up

```bash
# Toolchain — exact versions, pinned
rustup toolchain install 1.96.1
node --version   # 24.11.1
corepack enable && corepack prepare pnpm@11.10.0 --activate

git clone https://github.com/nkVas1/DreViGen.git
cd DreViGen
pnpm install
cargo build --workspace
pnpm dev
```

A UI-only contribution does **not** require the Rust toolchain: `pnpm dev:ui` builds
`packages/` against a prebuilt core artefact. This is deliberate, and if it ever stops working
it is a bug.

### Standards

These are not negotiable, because they are what keeps the project maintainable by one person.

| | |
|---|---|
| **Rust** | `cargo fmt`, `cargo clippy -- -D warnings`. `Result<T, E>` over panics; `.unwrap()` only in tests. Public items documented. |
| **TypeScript** | `strict: true`. No `any` without a comment explaining why. ESM only. `const` by default. |
| **Types** | Annotated everywhere. Not optional, in either language. |
| **Errors** | Never swallowed. Specific types, informative messages that state what happened, what the input was, and what was expected. |
| **Dependencies** | Exact versions, never ranges. Lock files committed. MIT / Apache-2.0 / BSD / OFL only — no exceptions, and no dependency that requires payment to function. |
| **Accessibility** | Every interactive element keyboard-reachable, labelled and focus-visible. **No icon without a visible text label.** CI enforces what it can; the rest is on review. |
| **Tests** | Where they protect something that cannot be checked more cheaply. No coverage theatre — but domain logic, parsers and merge semantics are never untested. |

### Commits

[Conventional Commits](https://www.conventionalcommits.org/), atomic, one logical change each.
Never mix a refactor with a feature.

```
feat(layout): add anchored incremental layout for sibling insertion
fix(gedcom): preserve unknown substructures on 5.5.1 import
perf(canvas): batch edge geometry into one instanced buffer
docs(adr): supersede 0006 with the Automerge fallback decision
```

Extended bodies go in a second `-m`. Never write a literal `\n` inside a message — it is
recorded verbatim and looks like a mistake, because it is one.

Do not add `Co-Authored-By` trailers or tool attribution.

### Pull requests

- Branch from `main` as `feat/…`, `fix/…`, `docs/…`, `perf/…`.
- CI must be green before review: format, clippy, tests, type-check, lint, `axe-core`, audit,
  WASM size budget.
- A PR that changes behaviour says what a user will notice.
- A PR that touches the canvas or layout includes the benchmark numbers before and after.
- A PR that adds a dependency justifies it and names the licence.

### Architectural changes

Anything that changes how a subsystem works needs an **ADR** first — context, decision,
consequences, alternatives considered. A decision that cannot state what it gives up has not
been made yet. Open the ADR as its own PR and let it be argued before the code exists.

## Localisation

Interface copy is ICU MessageFormat, with the source catalogue in English.

Translating is more than replacing strings. Genealogy is unusually hostile to naive
interpolation: Russian has four plural categories and gendered past-tense verbs
(«родился» / «родилась»), German compounds and inflects, kinship vocabularies do not map one to
one — Russian distinguishes *шурин*, *деверь* and *свояк* where English has one
"brother-in-law". The relationship calculator emits a structured kinship path and each locale
renders it; a translator working on kinship terms is defining a function, not filling a table.

If you want to add a locale, open an issue first so the structural work can be done with you.

## Reporting bugs

Include: what you did, what happened, what you expected, your platform and version, and — if it
involves a tree — an anonymised fixture if you can produce one. A reproducible fixture turns a
week of guessing into an afternoon of fixing.

Security issues go to [SECURITY.md](./SECURITY.md), not to the issue tracker.

## The standard everything is held to

The primary user of this application is an older man who sees perfectly well and gets lost in
interfaces. Every contribution is measured against whether he could still use the result.

That is not a slogan. It is the tie-breaker, and it has already decided several architectural
questions in this repository.

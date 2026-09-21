## What this changes

<!-- One paragraph. What a user or a maintainer will notice. -->

## Why

<!-- The problem being solved. Link the issue or ADR. -->

## How

<!-- Only the parts that are not obvious from the diff. -->

---

## Checklist

- [ ] One logical change. No refactor smuggled in alongside a feature.
- [ ] Commit messages follow Conventional Commits, with no literal `\n`.
- [ ] CI is green.
- [ ] Types annotated; no new `any` without a comment explaining it.
- [ ] Errors are specific and their messages say what happened, what the input was, and what
      was expected.
- [ ] Any new dependency is MIT / Apache-2.0 / BSD / OFL, pinned exactly, and justified above.
- [ ] Documentation updated if behaviour, CLI, configuration or data format changed.
- [ ] `CHANGELOG.md` updated under Unreleased if a user would notice this.

### If this touches the interface

- [ ] Keyboard-reachable, labelled, focus-visible.
- [ ] **No icon without a visible text label.**
- [ ] Targets at least 44×44 px.
- [ ] Contrast checked with APCA as well as WCAG 2.
- [ ] `prefers-reduced-motion` honoured.
- [ ] Would the primary persona still find his way? Say how you know.

### If this touches the canvas, layout or storage

- [ ] Benchmark numbers before and after, at the relevant tree size.
- [ ] Layout stability preserved — adding one person does not move the others.

### If this changes how a subsystem works

- [ ] An ADR exists and is linked. Architectural changes are argued before they are written.

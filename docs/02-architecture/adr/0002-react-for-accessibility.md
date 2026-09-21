# ADR 0002 — React 19 as the UI framework, chosen for React Aria

**Status:** Accepted · 2026-09-21

## Context

The primary persona is an older adult who sees well but becomes disoriented in interfaces.
Accessibility is therefore not a compliance exercise but the product's hardest constraint.

On raw performance, Solid and Svelte 5 both beat React: Solid leads the framework benchmarks
with fine-grained reactivity and ships under 4 KB gzipped; Svelte 5's compiler output is
smaller than React's; both outscored React on developer satisfaction in the 2025 State of JS.
React 19 carries virtual-DOM overhead that shows up under frequent updates.

## Decision

React 19.3 with the React Compiler, using **React Aria Components** as the behavioural
foundation of the design system.

## Consequences

**Gained.** WAI-ARIA-correct roles and states, keyboard interaction, focus management,
collection semantics, screen-reader announcement handling and internationalised interactions —
maintained by a dedicated team, for every component we need. The React Compiler removes the
manual memoisation that was React's main ergonomic cost. The broadest ecosystem for routing,
forms, tables, virtualisation and testing.

**Given up.** A larger runtime than Solid or Svelte, and worse microbenchmark numbers.

**Why the trade is right here.** The performance-critical path in DreViGen is the canvas and
the Rust core; React is not in that loop. It renders panels, forms and lists. Meanwhile the
accessibility gap between React Aria and anything available for Solid or Svelte is large, and
it maps directly onto our primary constraint. Choosing a faster framework would mean writing
accessible combobox, listbox, date-field, menu and dialog behaviour ourselves — which we would
do worse, and which would consume the time the project needs for genealogy.

**Mitigation.** UI state stays out of React where it would cause churn: canvas interaction has
its own loop, and domain state lives in the core.

## Alternatives considered

- **Solid 1.9.** Fastest, smallest. No accessibility primitive library of comparable depth.
- **Svelte 5.57.** Excellent DX with runes; the same accessibility gap.
- **Web Components with no framework.** Maximum longevity, and a large amount of behaviour to
  build before writing a single genealogical feature.

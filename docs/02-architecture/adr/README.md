# Architecture Decision Records

One file per decision, numbered, never edited after acceptance — superseded instead.

Format: context, decision, consequences, alternatives considered. A decision that cannot state
what it gives up has not been made yet.

| # | Decision | Status |
|---|---|---|
| [0001](./0001-rust-core-on-every-platform.md) | A shared Rust core compiled natively and to WASM | Accepted |
| [0002](./0002-react-for-accessibility.md) | React 19 as the UI framework, chosen for React Aria | Accepted |
| [0003](./0003-contribution-review-over-auto-merge.md) | Review between identities, auto-merge within one | Accepted |
| [0004](./0004-own-layout-engine.md) | Own Sugiyama-derived layout engine rather than elkjs or d3-dag | Accepted |
| [0005](./0005-assertion-conclusion-split.md) | Assertions and Conclusions as distinct model layers | Accepted |
| [0006](./0006-loro-as-crdt-substrate.md) | Loro as the CRDT substrate | Accepted — pending on-device validation |
| [0007](./0007-single-eager-wasm-module.md) | Ship the web core as one eager WASM module, no lazy split | Accepted — re-measured each phase |
| [0008](./0008-opfs-sahpool-single-connection.md) | One SQLite connection on `opfs-sahpool`; no cross-origin isolation | Accepted |
| [0009](./0009-msdf-atlas-tiers.md) | Pre-bake one MSDF coverage tier; load the rest on demand | Accepted |

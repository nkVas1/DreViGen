# ADR 0001 — A shared Rust core compiled natively and to WASM

**Status:** Accepted · 2026-09-21

## Context

DreViGen must run as a native application on Windows, macOS, Linux, iOS and Android, and as an
installable web application, with identical behaviour. Several subsystems are genuinely hard:
GEDCOM 7 parsing with byte-stable round-tripping, date arithmetic across four calendars with
interval semantics, graph layout over tens of thousands of nodes, phonetic matching, record
linkage, an append-only operation log, and CRDT merge.

Implementing these once per platform guarantees divergence. Implementing them in TypeScript
guarantees they will be too slow at our target scale.

## Decision

All domain logic lives in Rust crates under `crates/`. They are compiled natively for the Tauri
targets and to WebAssembly for the web target, behind a single typed bridge interface with two
transports (Tauri IPC via `tauri-specta`, and `wasm-bindgen`).

The UI layer holds no domain logic. It dispatches intents and renders projections.

## Consequences

**Gained.** One implementation of correctness. Native performance on the paths that need it.
A core testable at speed without a UI, including property-based tests over generated family
graphs. Type definitions generated from the Rust types, so the bridge cannot drift.

**Given up.** A higher barrier for contributors, who need a Rust toolchain. Longer cold builds.
A WASM bundle that must be size-budgeted. Debugging across the bridge is harder than debugging
within one language.

**Mitigations.** Contribution paths that touch only the UI must not require Rust — enforced by
keeping `packages/` buildable against a prebuilt core artefact. WASM size is tracked in CI with
a failing budget.

## Alternatives considered

- **TypeScript everywhere.** Simplest, and fails the 50 000-node performance target.
- **Rust on native, TypeScript on web.** Two implementations of every hard algorithm. Rejected
  outright — this is precisely the divergence the decision exists to prevent.
- **Go core.** Good WASM story, weaker at zero-cost abstraction and at the SIMD-adjacent work
  the layout and matching passes want; also a poorer fit with Tauri.

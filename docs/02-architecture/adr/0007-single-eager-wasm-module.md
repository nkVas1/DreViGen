# ADR 0007 — Ship the web core as one eager WASM module

**Status:** Accepted · 2026-09-21 · **re-measured at every phase boundary**

## Context

[ADR 0001](./0001-rust-core-on-every-platform.md) compiles the domain core twice: natively for
Tauri and to WebAssembly for the web. The native build pays nothing for that. The web build pays
in bytes the user downloads before anything appears on screen.

The roadmap set the budget at **2.5 MB gzipped** and named the fallback in advance: split the
core into an eager kernel and lazily loaded modules (ML, GEDCOM, layout) if the budget is missed.
Spike **S2** existed to find out which world we are in.

Splitting is not free. It means an async boundary inside the core, a loading state the UI has to
express, a module graph to keep coherent across two transports, and a class of bug where a
feature works on desktop and stalls on the web because its module has not arrived. It is worth
paying for if the budget demands it and worth refusing if it does not.

## The measurement

`spikes/s2-wasm-size`, measured by `tools/measure-wasm.sh` on 2026-09-21. The full shipping
pipeline — cargo → wasm-bindgen 0.2.128 → wasm-opt 132 at `-Oz` → gzip -9 — built at three
feature levels so the cost is attributed rather than reported as one number.

The exported surface is deliberately close to what the web front end will really call (open a
document, read a projection, edit, exchange updates), so nothing measured here would be
dead-code eliminated in a real build.

| Build | cargo | wasm-bindgen | wasm-opt -Oz | **gzipped** | JS glue |
|---|---|---|---|---|---|
| baseline — `wasm-bindgen` only | 36 KiB | 19 KiB | 17 KiB | **8 KiB** | 7 KiB |
| `crdt` — plus Loro | 2 183 KiB | 1 690 KiB | 1 522 KiB | **597 KiB** | 22 KiB |
| `crdt,demo` — plus testkit | 2 208 KiB | 1 714 KiB | 1 541 KiB | **606 KiB** | 24 KiB |

**606 KiB against a 2 500 KiB budget: 1 894 KiB of headroom.**

The attribution is unambiguous. The floor for any Rust/WASM module at all is **8 KiB**. Loro
costs **589 KiB** — 97 % of the total. The testkit adds **9 KiB**, which is a pleasant surprise
and a consequence of it carrying algorithms and short string tables rather than data.

## Decision

**One eager module. No lazy split.** The core compiles to a single WebAssembly artefact loaded
before first paint.

## Consequences

**Gained.** No async boundary inside the core, no module graph, no loading states for
functionality that should simply be present, and no divergence between the native and web
builds beyond the transport. The bridge stays one typed interface with two implementations,
which is what ADR 0001 promised.

**Given up.** The option of deferring rarely used code. We are choosing to pay 589 KiB for Loro
on every first load rather than defer it — correctly, since the CRDT is needed to open a tree at
all, which is the first thing anyone does.

**Accepted risk.** This measures the *dependencies already chosen*, not the finished core.
Still unweighed:

| Component | Expected cost | Why |
|---|---|---|
| `drevigen-core` domain model | small | Structs and invariants, no tables |
| `drevigen-date` | small–moderate | Calendar conversion is arithmetic; the GEDCOM date grammar is a parser |
| `drevigen-gedcom` | moderate | A full parser, serializer and validator |
| `drevigen-layout` | moderate | Sugiyama with genealogical constraints |
| `drevigen-match` | **the one to watch** | Beider–Morse carries per-language phonetic rule tables for 16 languages. Data, not code, and data does not shrink the way code does |
| `drevigen-store` | excluded | SQLite WASM loads separately |
| ML models | excluded | Fetched on demand, never bundled |
| Locale data | excluded | The browser's `Intl` provides CLDR; we do not ship our own |

1 894 KiB of headroom for that list is comfortable, but **Beider–Morse is the component that
could change the answer**, and it is the one to measure first when `drevigen-match` lands.

**Obligation.** `tools/measure-wasm.sh` is extended to the real core as each crate is written,
and the gzipped figure becomes a **CI budget that fails the build** rather than a number someone
remembers to check. If the budget is ever breached, this ADR is superseded by the split it
declines today — the fallback is not discarded, only deferred.

## Alternatives considered

- **Eager kernel plus lazy modules**, as the roadmap's fallback describes. Declined because the
  measurement removes the reason for it, and its costs — async inside the core, loading states,
  platform divergence — are real and permanent while the benefit is currently zero.
- **`opt-level = "s"` instead of `"z"`**, trading a little size for speed. Not explored: with
  this much headroom the question is whether `-Oz` is costing us runtime performance
  unnecessarily, which is a Phase 1 question to settle with a benchmark rather than a guess.
- **Dropping wasm-opt.** It removes 169 KiB before compression on the full build, for one step
  in the pipeline. Kept.

## Reproducing

```sh
tools/measure-wasm.sh [path-to-wasm-opt]
```

Requires the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.128 (the version must match
the `wasm-bindgen` crate), and Binaryen's `wasm-opt`. The script reports unoptimised sizes and
says so if `wasm-opt` is absent, rather than silently measuring the wrong thing.

//! **Spike S1 — can a CRDT carry a 50 000-person family tree?**
//!
//! This decides the sync architecture. [ADR 0003](../../../docs/02-architecture/adr/0003-contribution-review-over-auto-merge.md)
//! settled *that* we review between identities and converge within one; ADR 0006 is still
//! provisional on *which* library does the converging. The sync research note names Loro 1.16
//! with Automerge as the validated fallback, conditional on this measurement.
//!
//! **Pass criterion** (from the roadmap): a 50 000-person tree with a ~100 000-operation history
//! must fork, diff and merge in under 2 s using under 300 MB, on a mid-range Android device.
//!
//! **What this spike can and cannot answer.** It runs on a desktop, so it produces desktop
//! numbers. A mid-range phone is roughly 3–5× slower on single-threaded work and far tighter on
//! memory, so the desktop figure must clear the budget with a wide margin to be believable. The
//! on-device run is a separate task and the ADR does not close until it has happened.
//!
//! Run with `cargo run -p s1-crdt-scale --release`.

#![allow(clippy::print_stdout, clippy::expect_used, clippy::panic)]

mod alloc;
mod automerge_bench;
mod harness;
mod loro_bench;

use drevigen_testkit::{SyntheticTree, TreeSpec};
use harness::{FIELDS, Kind, Phase, mib, thousands};

#[global_allocator]
static ALLOCATOR: alloc::Tracking = alloc::Tracking;

/// Tree sizes to measure, and how many people each branch edits before merging.
const CASES: &[(usize, usize)] = &[(1_000, 50), (10_000, 250), (50_000, 1_000)];

/// The budget the roadmap sets, restated here so the program can check itself.
const MERGE_BUDGET_MS: f64 = 2_000.0;
const MEMORY_BUDGET_MIB: f64 = 300.0;

fn main() {
    println!("Spike S1 — CRDT scale for genealogical documents");
    println!("Loro 1.16.2 vs Automerge 0.12.0, {FIELDS} fields per person\n");

    let mut verdicts = Vec::new();

    for &(people, edits) in CASES {
        let tree = SyntheticTree::generate(TreeSpec::sized_for(people, 0x51A1 ^ people as u64));
        let stats = tree.stats();

        println!("{}", "═".repeat(86));
        println!(
            "{} people · {} families · {} generations · {} in-tree unions · {} edits per branch",
            thousands(stats.people as u64),
            thousands(stats.families as u64),
            stats.generations,
            thousands(stats.endogamous_unions as u64),
            thousands(edits as u64)
        );
        println!("{}", "═".repeat(86));
        println!(
            "{:<22} {:>12} {:>10} {:>10} {:>10} {:>10}",
            "phase", "time", "retained", "peak", "bytes out", "ops"
        );
        println!("{}", "─".repeat(86));

        let loro = loro_bench::run(&tree, edits);
        for phase in &loro {
            println!("{}", phase.render());
        }
        println!("{}", "─".repeat(86));

        let automerge = automerge_bench::run(&tree, edits);
        for phase in &automerge {
            println!("{}", phase.render());
        }
        println!();

        verdicts.push((
            people,
            summarise("Loro", &loro),
            summarise("Automerge", &automerge),
        ));
    }

    println!("{}", "═".repeat(86));
    println!(
        "VERDICT — budget: merge < {MERGE_BUDGET_MS:.0} ms, peak < {MEMORY_BUDGET_MIB:.0} MiB (desktop)"
    );
    println!("{}", "═".repeat(86));
    println!(
        "{:<8} {:<11} {:>10} {:>10} {:>11} {:>11} {:>8}",
        "people", "library", "build", "merge", "retained", "snapshot", "verdict"
    );
    println!("{}", "─".repeat(86));
    for (people, loro, automerge) in &verdicts {
        for s in [loro, automerge] {
            println!(
                "{:<8} {:<11} {:>7.0} ms {:>7.1} ms {:>11} {:>11} {:>8}",
                thousands(*people as u64),
                s.library,
                s.build_ms,
                s.merge_ms,
                mib(s.retained as f64),
                mib(s.snapshot_bytes as f64),
                if s.passes() { "pass" } else { "FAIL" }
            );
        }
    }

    // Encoded size is the one axis where Automerge leads, so report what trimming history buys
    // Loro: it is the difference between shipping the log inside the .dvg container or beside it.
    println!("\nLoro snapshot size, full history vs shallow:");
    for (people, loro, automerge) in &verdicts {
        let Some(shallow) = loro.shallow_bytes else {
            continue;
        };
        println!(
            "  {:>8} people   full {:>10}   shallow {:>10}   automerge {:>10}",
            thousands(*people as u64),
            mib(loro.snapshot_bytes as f64),
            mib(shallow as f64),
            mib(automerge.snapshot_bytes as f64)
        );
    }

    println!("\nDesktop numbers only. A mid-range phone runs single-threaded work roughly 3–5×");
    println!(
        "slower, so treat anything above {:.0} ms merge here as a failure on device.",
        MERGE_BUDGET_MS / 4.0
    );
}

struct Summary {
    library: &'static str,
    merge_ms: f64,
    build_ms: f64,
    retained: isize,
    snapshot_bytes: usize,
    shallow_bytes: Option<usize>,
}

impl Summary {
    fn passes(&self) -> bool {
        self.merge_ms < MERGE_BUDGET_MS
            && (self.retained as f64) / (1024.0 * 1024.0) < MEMORY_BUDGET_MIB
    }
}

fn summarise(library: &'static str, phases: &[Phase]) -> Summary {
    let find = |kind: Kind| phases.iter().find(|p| p.kind == kind);
    Summary {
        library,
        merge_ms: find(Kind::Merge).map_or(f64::NAN, |p| p.duration.as_secs_f64() * 1000.0),
        build_ms: find(Kind::Build).map_or(f64::NAN, |p| p.duration.as_secs_f64() * 1000.0),
        retained: find(Kind::Build).map_or(0, |p| p.live_delta),
        snapshot_bytes: find(Kind::Export).and_then(|p| p.bytes_out).unwrap_or(0),
        shallow_bytes: find(Kind::ExportShallow).and_then(|p| p.bytes_out),
    }
}

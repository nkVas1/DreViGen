//! Prints the shape of generated trees at several sizes.
//!
//! Run with `cargo run -p drevigen-testkit --example stats --release`.
//!
//! This exists so the generator can be eyeballed rather than only asserted about. Numbers that
//! look wrong here — a mean family size of 12, nobody living, no pedigree collapse — mean the
//! benchmarks downstream are measuring the wrong shape.

#![allow(clippy::print_stdout, clippy::unwrap_used)]

use drevigen_testkit::{SyntheticTree, TreeSpec};
use std::time::Instant;

fn main() {
    println!(
        "{:>9}  {:>8}  {:>9}  {:>4}  {:>7}  {:>8}  {:>8}  {:>6}  {:>5}  {:>8}",
        "target",
        "people",
        "families",
        "gen",
        "living",
        "in-tree",
        "marry-in",
        "remar",
        "kids",
        "time"
    );
    println!("{}", "─".repeat(96));

    for target in [1_000_usize, 10_000, 50_000, 200_000] {
        let started = Instant::now();
        let tree =
            SyntheticTree::generate(TreeSpec::sized_for(target, 0x_D2E7_u64 ^ target as u64));
        let elapsed = started.elapsed();
        let s = tree.stats();

        println!(
            "{:>9}  {:>8}  {:>9}  {:>4}  {:>6.1}%  {:>7}  {:>8}  {:>6}  {:>5.2}  {:>7.0}ms",
            target,
            s.people,
            s.families,
            s.generations,
            100.0 * s.living as f64 / s.people as f64,
            s.endogamous_unions,
            s.married_in,
            s.remarried,
            s.mean_children,
            elapsed.as_secs_f64() * 1000.0
        );
    }

    println!("\nA sample of the default tree:\n");
    let tree = SyntheticTree::generate(TreeSpec::default());
    for person in tree.people.iter().take(12) {
        let death = person
            .death_year
            .map_or_else(|| "—    ".to_owned(), |d| d.to_string());
        println!(
            "  gen {:<2} {:<38} {} – {:<6} {}",
            person.generation,
            person.full_name(),
            person.birth_year,
            death,
            person.birth_place
        );
    }
}

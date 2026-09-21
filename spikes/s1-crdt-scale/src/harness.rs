//! Shared measurement plumbing and the person record both candidates encode.

use std::time::{Duration, Instant};

use drevigen_testkit::{Person, SyntheticTree};

use crate::alloc;

/// Which step of the scenario a measurement belongs to.
///
/// A tagged enum rather than a string match on the label: `label.contains("merge")` also matches
/// "automerge", which silently reported build times as merge times in the first run of this
/// spike. Substring matching on human-readable labels is not a lookup key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Build,
    Export,
    Load,
    Fork,
    Edit,
    Merge,
    ExportAfter,
    ExportShallow,
}

/// One measured phase.
#[derive(Debug, Clone)]
pub struct Phase {
    pub kind: Kind,
    pub label: &'static str,
    pub duration: Duration,
    /// Peak bytes allocated during the phase, above the live figure when it started.
    ///
    /// This is allocation *churn*, which is what stresses an allocator but is not what fills a
    /// phone's heap. For that, see [`Phase::live_delta`].
    pub peak_delta: usize,
    /// Bytes still held when the phase finished, minus what was held when it began.
    ///
    /// The number that matters on a memory-constrained device: how much the document actually
    /// costs to keep resident, as opposed to how much was touched building it.
    pub live_delta: isize,
    /// Bytes produced, where the phase produces something (a snapshot, an update blob).
    pub bytes_out: Option<usize>,
    /// Operation count after the phase, where the library reports one.
    pub ops: Option<usize>,
}

impl Phase {
    pub fn render(&self) -> String {
        let out = self
            .bytes_out
            .map_or_else(|| "—".to_owned(), |b| mib(b as f64));
        let ops = self
            .ops
            .map_or_else(|| "—".to_owned(), |o| thousands(o as u64));
        format!(
            "{:<22} {:>9.1} ms {:>10} {:>10} {:>10} {:>10}",
            self.label,
            self.duration.as_secs_f64() * 1000.0,
            mib(self.live_delta as f64),
            mib(self.peak_delta as f64),
            out,
            ops
        )
    }
}

/// Runs `f`, timing it and measuring the peak allocation it causes.
pub fn measure<T>(kind: Kind, label: &'static str, f: impl FnOnce() -> T) -> (T, Phase) {
    let before = alloc::live();
    alloc::reset_peak();
    let started = Instant::now();
    let value = f();
    let duration = started.elapsed();
    let peak_delta = alloc::peak().saturating_sub(before);
    let live_delta = alloc::live() as isize - before as isize;
    (
        value,
        Phase {
            kind,
            label,
            duration,
            peak_delta,
            live_delta,
            bytes_out: None,
            ops: None,
        },
    )
}

pub fn mib(bytes: f64) -> String {
    format!("{:.1} MiB", bytes / (1024.0 * 1024.0))
}

pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The fields a person contributes to the document.
///
/// Six scalar fields per person is deliberately close to what DreViGen will really store on the
/// hot path — the fields the canvas reads at z2 and z3. Encoding one blob per person would
/// understate the operation count by an order of magnitude and make the merge look easy.
pub const FIELDS: usize = 6;

pub struct Record<'a> {
    pub key: String,
    pub given: &'a str,
    pub patronymic: &'a str,
    pub surname: &'a str,
    pub birth: i64,
    pub death: i64,
    pub place: &'a str,
}

pub fn record(person: &Person) -> Record<'_> {
    Record {
        key: format!("p{}", person.id.0),
        given: &person.given,
        patronymic: person.patronymic.as_deref().unwrap_or(""),
        surname: &person.surname,
        birth: i64::from(person.birth_year),
        death: person.death_year.map_or(0, i64::from),
        place: person.birth_place,
    }
}

/// Picks the people an editing session touches, spread across the tree rather than clustered.
pub fn edit_targets(tree: &SyntheticTree, count: usize, offset: usize) -> Vec<usize> {
    let n = tree.people.len();
    if n == 0 || count == 0 {
        return Vec::new();
    }
    // A coprime stride walks the whole population without repeating, which models a researcher
    // working across the tree rather than hammering one family.
    let stride = 7919;
    (0..count).map(|i| (offset + i * stride) % n).collect()
}

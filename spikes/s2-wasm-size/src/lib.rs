//! **Spike S2 — how big is the core once it is WebAssembly?**
//!
//! [ADR 0001](../../../docs/02-architecture/adr/0001-rust-core-on-every-platform.md) puts all
//! domain logic in Rust and compiles it twice: natively for Tauri and to WASM for the web. The
//! native side pays nothing for that. The web side pays in bytes the user downloads before the
//! first paint, and the roadmap sets the budget at **2.5 MB gzipped**, with an eager/lazy split
//! as the fallback if it is missed.
//!
//! The core does not exist yet, so this spike cannot weigh it. What it *can* do — and what
//! actually decides the answer — is weigh the parts already chosen, because the dependency is
//! where the bytes are. The surface below is deliberately close to what the web front end will
//! really call, so nothing is dead-code eliminated that would survive in the shipped build.
//!
//! Three feature combinations are built and compared, so the cost is attributed rather than
//! reported as one number:
//!
//! | Build | Contains |
//! |---|---|
//! | baseline | `wasm-bindgen` glue and a trivial API — the floor for *any* Rust/WASM module |
//! | `crdt` | the above plus Loro, the substrate chosen in ADR 0006 |
//! | `crdt,demo` | the above plus `drevigen-testkit`, which the demo tree needs |
//!
//! Measured by `tools/measure-wasm.sh`.

use wasm_bindgen::prelude::*;

/// Returns the build's feature set, so a measured artefact can identify itself.
#[wasm_bindgen]
#[must_use]
pub fn build_profile() -> String {
    let mut parts = vec!["baseline"];
    if cfg!(feature = "crdt") {
        parts.push("crdt");
    }
    if cfg!(feature = "demo") {
        parts.push("demo");
    }
    parts.join("+")
}

/// A relationship between two people, as the canvas consumes it.
///
/// Present in every build so the baseline is not artificially empty: any real module carries
/// some serialisation surface, and measuring a module that does nothing would understate the
/// floor.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Edge {
    /// Dense index of the parent.
    pub parent: u32,
    /// Dense index of the child.
    pub child: u32,
}

#[wasm_bindgen]
impl Edge {
    /// Creates an edge.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(parent: u32, child: u32) -> Self {
        Self { parent, child }
    }
}

#[cfg(feature = "crdt")]
mod crdt {
    use loro::{ExportMode, LoroDoc, LoroMap};
    use wasm_bindgen::prelude::*;

    /// A genealogical document backed by the CRDT substrate.
    ///
    /// The method set mirrors what the bridge in
    /// [the architecture overview](../../../docs/02-architecture/overview.md) will expose:
    /// open, read a projection, edit, and exchange updates. Exporting them through
    /// `wasm_bindgen` is what keeps the Loro code reachable, so the measurement reflects a
    /// module that genuinely uses the library rather than merely linking it.
    #[wasm_bindgen]
    pub struct Tree {
        doc: LoroDoc,
    }

    #[wasm_bindgen]
    impl Tree {
        /// Creates an empty document.
        #[wasm_bindgen(constructor)]
        #[must_use]
        pub fn new() -> Self {
            Self {
                doc: LoroDoc::new(),
            }
        }

        /// Opens a document from a snapshot.
        pub fn open(snapshot: &[u8]) -> Result<Tree, JsError> {
            let doc = LoroDoc::new();
            doc.import(snapshot)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            Ok(Self { doc })
        }

        /// Number of people in the document.
        #[must_use]
        pub fn person_count(&self) -> usize {
            self.doc.get_map("people").len()
        }

        /// Adds a person, returning their key.
        pub fn add_person(&self, id: u32, given: &str, surname: &str) -> Result<String, JsError> {
            let key = format!("p{id}");
            let people = self.doc.get_map("people");
            let m = people
                .insert_container(&key, LoroMap::new())
                .map_err(|e| JsError::new(&format!("{e}")))?;
            m.insert("g", given)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            m.insert("s", surname)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            self.doc.commit();
            Ok(key)
        }

        /// Sets one field on one person.
        pub fn set_field(&self, key: &str, field: &str, value: &str) -> Result<(), JsError> {
            let people = self.doc.get_map("people");
            let m = people
                .get(key)
                .and_then(|v| v.into_container().ok())
                .and_then(|c| c.into_map().ok())
                .ok_or_else(|| JsError::new("no such person"))?;
            m.insert(field, value)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            self.doc.commit();
            Ok(())
        }

        /// Exports a full snapshot.
        pub fn snapshot(&self) -> Result<Vec<u8>, JsError> {
            self.doc
                .export(ExportMode::snapshot())
                .map_err(|e| JsError::new(&format!("{e}")))
        }

        /// Exports everything the peer at `since` has not seen.
        pub fn updates_since(&self, since: &[u8]) -> Result<Vec<u8>, JsError> {
            let other = LoroDoc::new();
            other
                .import(since)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            self.doc
                .export(ExportMode::updates(&other.oplog_vv()))
                .map_err(|e| JsError::new(&format!("{e}")))
        }

        /// Applies updates from another replica.
        pub fn merge(&self, updates: &[u8]) -> Result<(), JsError> {
            self.doc
                .import(updates)
                .map_err(|e| JsError::new(&format!("{e}")))?;
            Ok(())
        }

        /// Forks a working copy.
        #[must_use]
        pub fn fork(&self) -> Tree {
            Self {
                doc: self.doc.fork(),
            }
        }

        /// Operations in the log, for diagnostics.
        #[must_use]
        pub fn op_count(&self) -> usize {
            self.doc.len_ops()
        }
    }

    impl Default for Tree {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "demo")]
mod demo {
    use drevigen_testkit::{SyntheticTree, TreeSpec};
    use wasm_bindgen::prelude::*;

    /// Generates a demo genealogy and returns it as flat arrays the canvas can consume without
    /// per-person object allocation.
    ///
    /// Returned as `[parent, child, parent, child, …]` over dense indices.
    #[wasm_bindgen]
    #[must_use]
    pub fn demo_edges(people: usize, seed: u64) -> Vec<u32> {
        let tree = SyntheticTree::generate(TreeSpec::sized_for(people, seed));
        let mut flat = Vec::new();
        for family in &tree.families {
            for &child in &family.children {
                for parent in [family.husband, family.wife].into_iter().flatten() {
                    flat.push(parent.0);
                    flat.push(child.0);
                }
            }
        }
        flat
    }

    /// Generates a demo genealogy and returns the display names, newline separated.
    #[wasm_bindgen]
    #[must_use]
    pub fn demo_names(people: usize, seed: u64) -> String {
        let tree = SyntheticTree::generate(TreeSpec::sized_for(people, seed));
        tree.people
            .iter()
            .map(drevigen_testkit::Person::full_name)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

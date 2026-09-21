//! The Loro 1.16 side of the S1 spike.

use drevigen_testkit::SyntheticTree;
use loro::{ExportMode, LoroDoc, LoroMap};

use crate::harness::{Kind, Phase, edit_targets, measure, record};

pub fn run(tree: &SyntheticTree, edits_per_branch: usize) -> Vec<Phase> {
    let mut phases = Vec::new();

    // ── build ──────────────────────────────────────────────────────────────
    let (doc, mut build) = measure(Kind::Build, "loro · build", || {
        let doc = LoroDoc::new();
        doc.set_peer_id(1).expect("peer id 1 is free on a new doc");
        let people = doc.get_map("people");
        for person in &tree.people {
            let r = record(person);
            let m = people
                .insert_container(&r.key, LoroMap::new())
                .expect("fresh key insert cannot conflict");
            m.insert("g", r.given).expect("scalar insert");
            m.insert("p", r.patronymic).expect("scalar insert");
            m.insert("s", r.surname).expect("scalar insert");
            m.insert("b", r.birth).expect("scalar insert");
            m.insert("d", r.death).expect("scalar insert");
            m.insert("pl", r.place).expect("scalar insert");
        }
        doc.commit();
        doc
    });
    build.ops = Some(doc.len_ops());
    phases.push(build);

    // ── snapshot ───────────────────────────────────────────────────────────
    let (snapshot, mut export) = measure(Kind::Export, "loro · snapshot", || {
        doc.export(ExportMode::snapshot()).expect("export snapshot")
    });
    export.bytes_out = Some(snapshot.len());
    phases.push(export);

    // ── shallow snapshot ───────────────────────────────────────────────────
    // History is what makes a Loro snapshot large. A shallow snapshot keeps the state and drops
    // history before the given frontiers, which is what our .dvg container would store if the
    // full log lives alongside it rather than inside it.
    let (shallow, mut shallow_export) =
        measure(Kind::ExportShallow, "loro · shallow snap", || {
            let frontiers = doc.state_frontiers();
            doc.export(ExportMode::shallow_snapshot(&frontiers))
                .expect("export shallow snapshot")
        });
    shallow_export.bytes_out = Some(shallow.len());
    phases.push(shallow_export);

    // ── load a fresh replica ───────────────────────────────────────────────
    let (loaded, load) = measure(Kind::Load, "loro · load snapshot", || {
        let fresh = LoroDoc::new();
        fresh.import(&snapshot).expect("import our own snapshot");
        fresh
    });
    debug_assert_eq!(loaded.get_map("people").len(), tree.people.len());
    phases.push(load);

    // ── fork two working copies ────────────────────────────────────────────
    let (branches, fork) = measure(Kind::Fork, "loro · fork ×2", || {
        let a = doc.fork();
        a.set_peer_id(2).expect("set peer on fork");
        let b = doc.fork();
        b.set_peer_id(3).expect("set peer on fork");
        (a, b)
    });
    let (branch_a, branch_b) = branches;
    phases.push(fork);

    // ── edit both, offline ─────────────────────────────────────────────────
    let (_, edits) = measure(Kind::Edit, "loro · edit both", || {
        for (branch, offset, marker) in [(&branch_a, 0_usize, "A"), (&branch_b, 3_usize, "B")] {
            let people = branch.get_map("people");
            for index in edit_targets(tree, edits_per_branch, offset) {
                let key = format!("p{index}");
                let m = people
                    .get(&key)
                    .and_then(|v| v.into_container().ok())
                    .and_then(|c| c.into_map().ok())
                    .expect("person container exists");
                // A plausible edit: correct a date and note where it came from.
                m.insert("b", i64::from(tree.people[index].birth_year) + 1)
                    .expect("scalar insert");
                m.insert("src", marker).expect("scalar insert");
            }
            branch.commit();
        }
    });
    phases.push(edits);

    // ── merge both branches back ───────────────────────────────────────────
    let (_, mut merge) = measure(Kind::Merge, "loro · merge both", || {
        let from_a = branch_a
            .export(ExportMode::updates(&doc.oplog_vv()))
            .expect("export updates from A");
        doc.import(&from_a).expect("import A");
        let from_b = branch_b
            .export(ExportMode::updates(&doc.oplog_vv()))
            .expect("export updates from B");
        doc.import(&from_b).expect("import B");
    });
    merge.ops = Some(doc.len_ops());
    phases.push(merge);

    // ── snapshot after merge, to see history growth ────────────────────────
    let (after, mut export_after) = measure(Kind::ExportAfter, "loro · snapshot after", || {
        doc.export(ExportMode::snapshot()).expect("export snapshot")
    });
    export_after.bytes_out = Some(after.len());
    phases.push(export_after);

    phases
}

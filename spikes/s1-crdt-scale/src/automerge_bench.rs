//! The Automerge 0.12 side of the S1 spike — the fallback named in the sync research note.

use automerge::{AutoCommit, ObjType, ROOT, ReadDoc, transaction::Transactable};
use drevigen_testkit::SyntheticTree;

use crate::harness::{Kind, Phase, edit_targets, measure, record};

pub fn run(tree: &SyntheticTree, edits_per_branch: usize) -> Vec<Phase> {
    let mut phases = Vec::new();

    // ── build ──────────────────────────────────────────────────────────────
    let (mut doc, build) = measure(Kind::Build, "automerge · build", || {
        let mut doc = AutoCommit::new();
        let people = doc
            .put_object(ROOT, "people", ObjType::Map)
            .expect("root map insert");
        for person in &tree.people {
            let r = record(person);
            let m = doc
                .put_object(&people, &r.key, ObjType::Map)
                .expect("person map insert");
            doc.put(&m, "g", r.given).expect("scalar put");
            doc.put(&m, "p", r.patronymic).expect("scalar put");
            doc.put(&m, "s", r.surname).expect("scalar put");
            doc.put(&m, "b", r.birth).expect("scalar put");
            doc.put(&m, "d", r.death).expect("scalar put");
            doc.put(&m, "pl", r.place).expect("scalar put");
        }
        doc
    });
    phases.push(build);

    // ── save ───────────────────────────────────────────────────────────────
    let (saved, mut save) = measure(Kind::Export, "automerge · save", || doc.save());
    save.bytes_out = Some(saved.len());
    phases.push(save);

    // ── load a fresh replica ───────────────────────────────────────────────
    let (loaded, load) = measure(Kind::Load, "automerge · load", || {
        AutoCommit::load(&saved).expect("load our own save")
    });
    debug_assert!(loaded.get(ROOT, "people").is_ok());
    phases.push(load);

    // ── fork two working copies ────────────────────────────────────────────
    let (branches, fork) = measure(Kind::Fork, "automerge · fork ×2", || {
        let a = doc.fork();
        let b = doc.fork();
        (a, b)
    });
    let (mut branch_a, mut branch_b) = branches;
    phases.push(fork);

    // ── edit both, offline ─────────────────────────────────────────────────
    let (_, edits) = measure(Kind::Edit, "automerge · edit both", || {
        for (branch, offset, marker) in
            [(&mut branch_a, 0_usize, "A"), (&mut branch_b, 3_usize, "B")]
        {
            let Ok(Some((_, people))) = branch.get(ROOT, "people") else {
                panic!("people map missing");
            };
            for index in edit_targets(tree, edits_per_branch, offset) {
                let key = format!("p{index}");
                let Ok(Some((_, person))) = branch.get(&people, &key) else {
                    panic!("person {key} missing");
                };
                branch
                    .put(&person, "b", i64::from(tree.people[index].birth_year) + 1)
                    .expect("scalar put");
                branch.put(&person, "src", marker).expect("scalar put");
            }
        }
    });
    phases.push(edits);

    // ── merge both branches back ───────────────────────────────────────────
    let (_, merge) = measure(Kind::Merge, "automerge · merge both", || {
        doc.merge(&mut branch_a).expect("merge A");
        doc.merge(&mut branch_b).expect("merge B");
    });
    phases.push(merge);

    // ── save after merge ───────────────────────────────────────────────────
    let (after, mut save_after) =
        measure(Kind::ExportAfter, "automerge · save after", || doc.save());
    save_after.bytes_out = Some(after.len());
    phases.push(save_after);

    phases
}

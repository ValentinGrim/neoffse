//! The proof runner of the M3 exit criterion: an edit, then encode and
//! decode, yields the intended change and no other difference.
//!
//! [`prove`] drives the document through `Box<dyn SaveDocument>`. The decode
//! after the edit is `container::decode` plus the tree of `json_tree`, and no
//! `Ff1Document`, `Ff1Model` or `Document` code reads the result before the
//! diff. A failure names the subject, the case and the interface and never
//! prints a save value (`docs/architecture.md#test-policy`).

use std::fmt::Display;

use neoffse_core::{CapPolicy, ChangeSet, Edit, SaveDocument, SaveFiles, SaveView};
use neoffse_format_pr::container::{self, Decoded};
use neoffse_format_pr::document::Document;
use neoffse_format_pr::ff1::{Ff1Model, GAME};

pub(crate) use super::edit_subject::{Subject, open_files};
pub(crate) use super::edit_view::ViewChange;
use super::edit_view::{mismatch, raises_worn_count};
use super::ff1_rules::violations;
use super::json_diff::{Change, assert_changes, diff_trees, layout_chain};
use super::json_tree::Tree;

/// The interface an edit goes through: `apply_game`, or `apply` under `Enforce`
/// (`apply_all` when a case holds several edits, to cover that override).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum How {
    Game,
    Raw,
}

#[derive(Debug, Clone)]
pub(crate) struct Proof {
    /// The expected diff, before the `Layout` chain is added.
    pub changes: Vec<Change>,
    pub view: Vec<ViewChange>,
    /// `(path, exact text)` that the tree after the edit holds at `path`.
    pub texts: Vec<(String, String)>,
}

/// The edits as applied: the redo and the undo.
struct Pair {
    forward: ChangeSet,
    inverse: ChangeSet,
}

/// Applies `edits` to a fresh document of `subject` and proves, step by step,
/// that the file changed as `proof` says and nowhere else.
///
/// # Panics
///
/// At the first step that fails, with `{subject} {case} {how:?}: {step}`.
pub(crate) fn prove(subject: &Subject, case: &str, edits: &[Edit], how: How, proof: &Proof) {
    let run = Run {
        subject,
        proof,
        label: format!("{} {case} {how:?}", subject.label),
    };
    let mut document = subject.open();
    let before = run.view(&mut *document);
    let pairs = run.apply(&mut *document, edits, how);
    let (files, decoded) = run.encode(&*document);
    run.check_container(&*document, &files, &decoded);
    let after = run.check_diff(&decoded.json);
    run.check_texts(&after);
    let mut reopened = run.open(&files);
    let view_after = run.view(&mut *reopened);
    run.check_view(&before, &view_after);
    run.undo(&mut *document, &pairs, "undo");
    run.expect_restored(&*document, "undo does not restore the text");
    run.redo(&mut *document, &pairs);
    if run.encode(&*document).1.json != decoded.json {
        run.fail("redo differs");
    }
    run.undo(&mut *reopened, &pairs, "undo after reopen");
    run.expect_restored(&*reopened, "undo after reopen differs");
    if how == How::Game {
        run.check_rules(&decoded.json, &before);
    }
}

struct Run<'a> {
    subject: &'a Subject,
    proof: &'a Proof,
    label: String,
}

impl Run<'_> {
    fn fail(&self, what: impl Display) -> ! {
        panic!("{}: {what}", self.label)
    }

    /// Adds the error text only for a synthetic subject.
    fn fail_with(&self, what: &str, error: &dyn Display) -> ! {
        if self.subject.private {
            self.fail(what);
        }
        self.fail(format!("{what}: {error}"))
    }

    fn refused(&self, index: usize, error: &dyn Display) -> ! {
        self.fail_with(&format!("edit {index} refused"), error)
    }

    fn open(&self, files: &SaveFiles) -> Box<dyn SaveDocument> {
        open_files(files).unwrap_or_else(|error| self.fail_with("reopen failed", &error))
    }

    fn view(&self, document: &mut dyn SaveDocument) -> SaveView {
        document.view().unwrap_or_else(|error| self.fail_with("view failed", &error))
    }

    /// Step 2. A `Raw` run with several edits goes through one `apply_all`.
    fn apply(&self, document: &mut dyn SaveDocument, edits: &[Edit], how: How) -> Vec<Pair> {
        match (how, edits) {
            (How::Game, _) => edits
                .iter()
                .enumerate()
                .map(|(index, edit)| match document.apply_game(edit) {
                    Ok(change) => Pair {
                        forward: change.forward,
                        inverse: change.inverse,
                    },
                    Err(error) => self.refused(index, &error),
                })
                .collect(),
            (How::Raw, [edit]) => match document.apply(edit, CapPolicy::Enforce) {
                Ok(inverse) => vec![Pair {
                    forward: ChangeSet::from_iter([edit.clone()]),
                    inverse: ChangeSet::from_iter([inverse]),
                }],
                Err(error) => self.refused(0, &error),
            },
            (How::Raw, _) => {
                let forward: ChangeSet = edits.iter().cloned().collect();
                match document.apply_all(&forward, CapPolicy::Enforce) {
                    Ok(inverse) => vec![Pair { forward, inverse }],
                    Err(error) => self.refused(0, &error),
                }
            }
        }
    }

    /// The file bytes of `document` through the independent decode.
    fn encode(&self, document: &dyn SaveDocument) -> (SaveFiles, Decoded) {
        let files = document.encode().unwrap_or_else(|error| self.fail_with("encode failed", &error));
        let Some(bytes) = files.get(&self.subject.name) else {
            self.fail("the encoding lacks the slot file")
        };
        let decoded = container::decode(bytes).unwrap_or_else(|error| self.fail_with("decode failed", &error));
        (files, decoded)
    }

    /// Step 3: the container, `verify` and the document layer's load.
    fn check_container(&self, document: &dyn SaveDocument, files: &SaveFiles, decoded: &Decoded) {
        if decoded.container != self.subject.container {
            self.fail("container changed");
        }
        if let Err(error) = document.verify(files) {
            self.fail_with("verify refuses the encoding", &error);
        }
        match Document::parse(&decoded.json) {
            Ok(parsed) if parsed.to_json() == decoded.json => {}
            Ok(_) => self.fail("the document layer rewrites the text on load"),
            Err(_) => self.fail("the document layer refuses the text"),
        }
    }

    /// Step 4: the structural diff against the expected list plus the
    /// `Layout` chain of the rewritten ancestors.
    fn check_diff(&self, json: &str) -> Tree {
        let after = Tree::parse(json);
        let actual = diff_trees(&self.subject.tree, &after);
        let mut expected = self.proof.changes.clone();
        expected.extend(layout_chain(&self.subject.tree, &self.proof.changes));
        assert_changes(&self.label, &actual, &expected);
        after
    }

    /// Step 5.
    fn check_texts(&self, after: &Tree) {
        for (path, text) in &self.proof.texts {
            if after.raw_at(path) != Some(text.as_str()) {
                self.fail(format!("wrong text at {path}"));
            }
        }
    }

    /// Step 6: the reopened view is the view before with `proof.view` applied.
    fn check_view(&self, before: &SaveView, after: &SaveView) {
        if let Some(what) = mismatch(before, after, &self.proof.view) {
            self.fail(what);
        }
    }

    /// Steps 7 and 9: the inverses, newest pair first.
    fn undo(&self, document: &mut dyn SaveDocument, pairs: &[Pair], step: &str) {
        for pair in pairs.iter().rev() {
            if let Err(error) = document.apply_all(&pair.inverse, CapPolicy::StorageOnly) {
                self.fail_with(&format!("{step} refused"), &error);
            }
        }
    }

    /// Step 8.
    fn redo(&self, document: &mut dyn SaveDocument, pairs: &[Pair]) {
        for pair in pairs {
            if let Err(error) = document.apply_all(&pair.forward, CapPolicy::StorageOnly) {
                self.fail_with("redo refused", &error);
            }
        }
    }

    /// After an undo the text is the original. A rewritten level is compact
    /// (`docs/formats/pixel-remaster.md#write-rules`), so on a spaced text
    /// the only difference allowed is the `Layout` chain of the edit.
    fn expect_restored(&self, document: &dyn SaveDocument, what: &str) {
        let (_, decoded) = self.encode(document);
        let same_text = decoded.json == self.subject.json;
        let chain = layout_chain(&self.subject.tree, &self.proof.changes);
        let same_up_to_layout = !chain.is_empty() && diff_trees(&self.subject.tree, &Tree::parse(&decoded.json)) == chain;
        if !(same_text || same_up_to_layout) || decoded.container != self.subject.container {
            self.fail(what);
        }
    }

    /// Step 10: the edited slot obeys the game rules. The copy rule is off
    /// when the edit raised the inventory count of a worn item: the other
    /// wearers keep the count they copied when they equipped it
    /// (`docs/formats/pixel-remaster.md#equipment-slots`, "does not refresh").
    fn check_rules(&self, json: &str, before: &SaveView) {
        let Some(tables) = neoffse_data::tables_for(GAME) else {
            self.fail("no FF I tables")
        };
        let Ok(parsed) = Document::parse(json) else {
            self.fail("the document layer refuses the text")
        };
        let copy_counts = !self.subject.private && !raises_worn_count(before, &self.proof.view);
        let found = violations(&mut Ff1Model::new(parsed, tables), copy_counts);
        if !found.is_empty() {
            self.fail(found.join("; "));
        }
    }
}

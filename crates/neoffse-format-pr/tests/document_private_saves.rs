//! `Document` over the real saves: parse, walk every value, expect the text back.
//!
//! The folder is read-only input. The test edits nothing and prints nothing:
//! a failure names the game, the file and the rendered path, never a value.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Step, segments};
use neoffse_format_pr::container;
use neoffse_format_pr::document::{Document, DocumentError, render_path};
use serde_json::Value;

fn is_document(content: &str) -> bool {
    content.starts_with(['{', '[']) && serde_json::from_str::<Value>(content).is_ok()
}

/// Walks the oracle `Value` and the document side by side. `entered` counts
/// the `Doc` steps the document accepted.
struct Walk<'a> {
    document: &'a mut Document,
    label: &'a str,
    entered: usize,
}

impl Walk<'_> {
    fn visit(&mut self, value: &Value, path: &mut Vec<Step>) {
        let steps = segments(path);
        let rendered = render_path(&steps);
        let raw = self.document.raw(&steps);
        assert!(raw.is_ok(), "{} {rendered}: raw failed", self.label);
        match value {
            Value::Object(members) => {
                for (key, member) in members {
                    path.push(Step::Key(key.clone()));
                    self.visit(member, path);
                    path.pop();
                }
            }
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    path.push(Step::Index(index));
                    self.visit(item, path);
                    path.pop();
                }
            }
            Value::String(content) => self.visit_string(content, path, &rendered),
            _ => {}
        }
    }

    fn visit_string(&mut self, content: &str, path: &mut Vec<Step>, rendered: &str) {
        let label = self.label;
        path.push(Step::Doc);
        let doc_steps = segments(path);
        let step = self.document.raw(&doc_steps).map(|raw| raw.len());
        if content.is_empty() {
            assert!(
                matches!(step, Err(DocumentError::EmptyDocument { .. })),
                "{label} {rendered}: an empty string must give EmptyDocument"
            );
        } else if is_document(content) {
            assert!(step.is_ok(), "{label} {rendered}: Doc step failed");
            self.entered += 1;
            if let Ok(inner) = serde_json::from_str::<Value>(content) {
                self.visit(&inner, path);
            }
        } else {
            assert!(
                matches!(step, Err(DocumentError::NotADocument { .. } | DocumentError::InvalidJson { .. })),
                "{label} {rendered}: a plain string must give NotADocument or InvalidJson"
            );
        }
        path.pop();
    }
}

#[test]
fn private_saves_round_trip_without_edits() {
    for game in common::GAMES {
        let Some(files) = common::save_files(game) else {
            continue;
        };
        let mut total = 0;
        for file in files {
            let Some(name) = file.file_name() else {
                panic!("{game}: a save path has no file name");
            };
            let label = format!("{game} {}", name.to_string_lossy());
            let Ok(bytes) = std::fs::read(&file) else { panic!("{label}: read failed") };
            let Ok(decoded) = container::decode(&bytes) else {
                panic!("{label}: decode failed")
            };
            let Ok(mut document) = Document::parse(&decoded.json) else {
                panic!("{label}: parse failed")
            };
            assert!(document.to_json() == decoded.json, "{label}: before access");

            let Ok(oracle) = serde_json::from_str::<Value>(&decoded.json) else {
                panic!("{label}: oracle parse failed")
            };
            let mut walk = Walk {
                document: &mut document,
                label: &label,
                entered: 0,
            };
            walk.visit(&oracle, &mut Vec::new());
            let entered = walk.entered;
            assert!(document.to_json() == decoded.json, "{label}: after the walk");
            total += entered;
        }
        assert!(total > 0, "{game}: no document descended");
    }
}

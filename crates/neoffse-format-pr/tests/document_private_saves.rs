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

/// The string documents of `value`, counted by `serde_json` alone: a
/// non-empty string that starts with `{` or `[` and parses, then the
/// documents inside it. The walk below must enter exactly this many.
fn count_oracle_docs(value: &Value) -> usize {
    match value {
        Value::Object(members) => members.values().map(count_oracle_docs).sum(),
        Value::Array(items) => items.iter().map(count_oracle_docs).sum(),
        Value::String(content) if is_document(content) => 1 + serde_json::from_str::<Value>(content).map_or(0, |inner| count_oracle_docs(&inner)),
        _ => 0,
    }
}

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
        }
        path.pop();
    }
}

#[test]
fn private_saves_round_trip_without_edits() {
    for game in ["ff1", "ff2", "ff3", "ff4", "ff5", "ff6"] {
        let Some(files) = neoffse_testkit::private_saves(game) else {
            continue;
        };
        let mut total = 0;
        for file in files {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            if name == "steam_autocloud.vdf" {
                continue;
            }
            let label = format!("{game} {name}");
            let bytes = std::fs::read(&file).unwrap();
            let decoded = container::decode(&bytes).unwrap();
            let mut document = Document::parse(&decoded.json).unwrap();
            assert!(document.to_json() == decoded.json, "{label}: before access");

            let oracle: Value = serde_json::from_str(&decoded.json).unwrap();
            let oracle_docs = count_oracle_docs(&oracle);
            let mut walk = Walk {
                document: &mut document,
                label: &label,
                entered: 0,
            };
            walk.visit(&oracle, &mut Vec::new());
            let entered = walk.entered;
            assert!(entered == oracle_docs, "{label}: documents entered differ from the oracle");
            assert!(document.to_json() == decoded.json, "{label}: after the walk");
            total += entered;
        }
        assert!(total > 0, "{game}: no document descended");
    }
}

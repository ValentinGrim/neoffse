//! `Session` on a copy of the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only). The test skips when the folder is absent and prints slot labels,
//! never file names.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::Registry;
use neoffse::core::{Edit, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::pr::ff1::names;
use neoffse::pr::slots::SLOT_FILES;

#[test]
fn an_edit_then_an_undo_restores_every_private_slot() {
    let Some(paths) = neoffse_testkit::private_saves("ff1") else { return };
    let Some(dir) = paths.first().and_then(|path| path.parent()) else { return };
    let registry = Registry::new(Language::En);
    let gil = FieldId::new(names::GIL);

    for path in &paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !SLOT_FILES.iter().any(|file| file.name == name) {
            continue;
        }
        let mut session = registry.open(dir, name, Some(Game::Ff1Pr)).unwrap();
        let slot = session.summary().unwrap().slot;
        let view = session.view().unwrap();
        let (_, field) = view.find(&gil).expect("gil is in the view");
        let Value::Int(before) = field.value else { panic!("gil is an integer") };

        let next = if before > 0 { before - 1 } else { 1 };
        session.edit(&Edit::set(gil.clone(), Value::Int(next))).unwrap();
        assert!(session.is_dirty(), "slot {slot:?} must be dirty after an edit");
        assert!(session.undo().unwrap(), "slot {slot:?} must undo");
        assert!(!session.is_dirty(), "slot {slot:?} must be clean after the undo");
        assert_eq!(session.view().unwrap(), view, "slot {slot:?} view must come back");
    }
}

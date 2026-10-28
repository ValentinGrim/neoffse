//! `Session` on a copy of the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only). The test skips when the folder holds no slot file and prints slot
//! positions, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{must, must_some, private_slot_paths};
use neoffse::Registry;
use neoffse::core::{Edit, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::pr::ff1::names;

#[test]
fn an_edit_then_an_undo_restores_every_private_slot() {
    let Some(slots) = private_slot_paths() else { return };
    let dir = must_some(slots.first().and_then(|(_, path)| path.parent()), "a slot file has a folder");
    let registry = Registry::new(Language::En);
    let gil = FieldId::new(names::GIL);

    for (index, (name, _)) in slots.iter().enumerate() {
        let mut session = must(registry.open(dir, name, Some(Game::Ff1Pr)), "open a private slot");
        let view = must(session.view(), "read the view");
        let (_, field) = view.find(&gil).expect("gil is in the view");
        let Value::Int(before) = field.value else { panic!("gil is an integer") };

        let next = if before > 0 { before - 1 } else { 1 };
        must(session.edit(&Edit::set(gil.clone(), Value::Int(next))), "edit the gil");
        assert!(session.is_dirty(), "slot {index} must be dirty after an edit");
        assert!(must(session.undo(), "undo the gil edit"), "slot {index} must undo");
        assert!(!session.is_dirty(), "slot {index} must be clean after the undo");
        let after = must(session.view(), "read the view after the undo");
        assert!(after == view, "slot {index}: the view differs after the undo");
    }
}

//! Synthetic slot files.

use neoffse::pr::container::{self, Container};
use neoffse::pr::document::{Document, Scalar, Segment};
use neoffse::pr::slots::SLOT_FILES;

/// The synthetic FF I slot with its top-level `id` set to `id`, in the
/// game's container.
pub(crate) fn slot_bytes(id: i64) -> Vec<u8> {
    let json = neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap();
    let mut document = Document::parse(&String::from_utf8(json).unwrap()).unwrap();
    document.set(&[Segment::Key("id")], &Scalar::Int(id)).unwrap();
    container::encode(&document.to_json(), &Container::game()).unwrap()
}

/// The 44-character file name of the slot with JSON id `id`.
pub(crate) fn slot_file(id: u32) -> &'static str {
    SLOT_FILES.iter().find(|file| file.id == id).unwrap().name
}

//! Helpers shared by the portrait tests of FF I and FF II: a party view whose
//! characters hold given job ids and the pixels of an image.

use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, SaveView, Section, Shape, Value};
use neoffse::pr::ff1::names;

/// The size and the bytes of the pixels, `None` for an empty image.
pub(crate) fn pixels(image: &slint::Image) -> Option<(u32, u32, Vec<u8>)> {
    image.to_rgba8().map(|buffer| (buffer.width(), buffer.height(), buffer.as_bytes().to_vec()))
}

pub(crate) fn size_of(image: &slint::Image) -> (u32, u32) {
    let size = image.size();
    (size.width, size.height)
}

/// A view of characters named `labels`, character ids from 1, each holding
/// the job in `jobs` at its position, `None` for no job field. FF II shares
/// the names `character` and `job`, so one builder serves both games.
pub(crate) fn party(labels: &[&str], jobs: &[Option<Value>]) -> SaveView {
    let entities = labels
        .iter()
        .zip(jobs)
        .enumerate()
        .map(|(index, (label, job))| {
            let id = FieldId::new(names::CHARACTER).with_id(u32::try_from(index + 1).unwrap());
            let fields = job
                .iter()
                .map(|value| {
                    let meta = FieldMeta::new("Job", Shape::Int { min: 0, max: 99 });
                    FieldView::new(id.clone().with_name(names::JOB), meta, value.clone())
                })
                .collect();
            Entity::new(id, *label, fields)
        })
        .collect();
    SaveView::new(vec![Section::new(Capability::Party, entities)])
}

pub(crate) fn jobs(ids: &[i64]) -> Vec<Option<Value>> {
    ids.iter().map(|id| Some(Value::Int(*id))).collect()
}

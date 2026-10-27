//! The overview over a copy of the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only). Every slot is copied into a temporary folder first. The test skips
//! when the folder is absent. A failure names a slot number and a field
//! label, never a value, a file name or a path. No edit, no save.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Flow, Outcomes, Part, PrivateSlot, field, fields, items, pages, private_slot_copies};
use neoffse_app::home::OpenOutcome;
use neoffse_app::{EntityPage, FieldRow};

/// The field labelled `label` on `page`. Matches the page itself, so two
/// characters with one name cannot cross.
fn on_page(page: &EntityPage, label: &str) -> FieldRow {
    fields(page)
        .into_iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no field labelled {label}"))
}

#[test]
fn every_private_slot_builds_its_cards() {
    let Some(slots) = private_slot_copies() else { return };
    for (position, PrivateSlot { name, site, copy }) in slots.iter().enumerate() {
        let opened = position + 1;
        let bytes = fs::read(copy).unwrap();

        let flow = Flow::new(site, None);
        let outcomes = Outcomes::new();
        flow.click(name, &outcomes);
        flow.manual.run_all();
        assert_eq!(outcomes.outcomes(), vec![OpenOutcome::Opened], "private slot {opened} did not open");

        let data = flow.editor.data().unwrap_or_else(|_| panic!("private slot {opened}: data failed"));
        let built = flow.editor.overview();
        let cards = items(&built.cards);
        let party = pages(&data.party);
        assert!(built.present, "private slot {opened}: no overview");
        assert_eq!(cards.len(), party.len(), "private slot {opened}: cards and pages differ in number");
        for (index, card) in cards.iter().enumerate() {
            let page = &party[usize::try_from(card.page).unwrap()];
            let label = &page.label;
            assert_eq!(
                usize::try_from(card.page).unwrap(),
                index,
                "private slot {opened}: card {index} opens another page"
            );
            assert_eq!(card.name, *label, "private slot {opened}: card {index} name");
            for (what, shown, source) in [("HP", &card.hp, "HP"), ("Level", &card.level, "Level"), ("Experience", &card.exp, "Experience")] {
                let page_text = on_page(page, source).text;
                assert_ne!(shown.as_str(), "", "private slot {opened}: card {index} has no {what}");
                assert_eq!(*shown, page_text, "private slot {opened}: card {index} {what}");
            }
            let charges = items(&card.charges);
            assert_eq!(charges.len(), 8, "private slot {opened}: card {index} charges");
            for (level, shown) in charges.iter().enumerate() {
                let page_text = on_page(page, &format!("Charges, level {}", level + 1)).text;
                assert_eq!(*shown, page_text, "private slot {opened}: card {index} charges, level {}", level + 1);
            }
            assert_eq!(card.job, on_page(page, "Job").shown, "private slot {opened}: card {index} job");
            assert_ne!(card.hp_max.as_str(), "", "private slot {opened}: card {index} has no Max HP");
            assert_eq!(card.hp_max, on_page(page, "Max HP").text, "private slot {opened}: card {index} Max HP");
            let next = fields(page)
                .into_iter()
                .find(|row| row.label == "Next level")
                .map_or_else(String::new, |row| row.text.to_string());
            assert_eq!(card.next_exp.as_str(), next, "private slot {opened}: card {index} Next level");
        }
        let gil = field(&data, Part::Inventory, "Gil", "Gil").text;
        assert_eq!(built.gil, gil, "private slot {opened}: gil");
        assert!(!flow.editor.is_dirty(), "private slot {opened} is dirty");
        assert_eq!(fs::read(copy).unwrap(), bytes, "private slot {opened}: the copy changed");
    }
}

//! The fixture's ability keys obey the measured FF I rules of
//! `docs/formats/pixel-remaster.md#abilities`, and each rule check fails on a
//! slot that breaks its rule.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff1_ability_rules::ability_violations;
use common::ff1_magic::{ability, level_element, model_with_slots};
use common::ff1_save::SlotData;
use common::json_tree::Tree;
use neoffse_format_pr::document::{Document, Scalar, Segment};

const CHARACTER: usize = 2;

/// The path to the document `list` of the character that holds spells
/// (index 2), then `rest`.
fn path<'a>(list: &'a str, rest: &[Segment<'a>]) -> Vec<Segment<'a>> {
    let mut path = character_path(CHARACTER, &[Segment::Doc, Segment::Key(list), Segment::Doc]);
    path.extend_from_slice(rest);
    path
}

/// The violations of the fixture after `edit` changed one token.
fn after(edit: impl FnOnce(&mut Document)) -> Vec<String> {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    edit(&mut document);
    ability_violations(&Tree::parse(&document.to_json()))
}

#[test]
fn the_fixture_obeys_the_ability_rules() {
    let tree = Tree::parse(&SlotData::standard().render());
    assert_eq!(ability_violations(&tree), Vec::<String>::new());
}

#[test]
fn the_rules_are_not_vacuous() {
    let tree = Tree::parse(&SlotData::standard().render());
    let list = "userData.ownedCharacterList.target[2].abilityList.target";
    assert_eq!(tree.array_len(list), Some(2));
    assert_eq!(tree.int(&format!("{list}[0].contentId")), Some(213));
    assert_eq!(tree.int(&format!("{list}[1].contentId")), Some(214));
    assert_eq!(tree.int("userData.learnedAbilityList.target[0]"), Some(5));
    assert_eq!(tree.int("userData.learnedAbilityList.target[1]"), Some(6));
    let slot = "userData.ownedCharacterList.target[2].abilitySlotDataList.target[0].slotInfo.values";
    assert_eq!(tree.int(&format!("{slot}[0].abilityId")), Some(5));
    assert_eq!(tree.int(&format!("{slot}[1].abilityId")), Some(6));
    assert_eq!(tree.raw_at(&format!("{slot}[2]")), Some("\"\""));
}

#[test]
fn a_skill_level_of_one_is_caught() {
    let found = after(|document| {
        let path = path(
            "abilityList",
            &[Segment::Key("target"), Segment::Index(0), Segment::Doc, Segment::Key("skillLevel")],
        );
        document.set(&path, &Scalar::Int(1)).unwrap();
    });
    assert!(found.iter().any(|text| text.ends_with("skillLevel: not 0")), "{found:?}");
}

#[test]
fn a_content_id_off_the_offset_is_caught() {
    let found = after(|document| {
        let path = path(
            "abilityList",
            &[Segment::Key("target"), Segment::Index(0), Segment::Doc, Segment::Key("contentId")],
        );
        document.set(&path, &Scalar::Int(300)).unwrap();
    });
    assert!(found.iter().any(|text| text.ends_with("not abilityId + 208")), "{found:?}");
}

#[test]
fn an_emptied_slot_is_caught() {
    let found = after(|document| {
        let rest = [Segment::Key("target"), Segment::Index(0), Segment::Doc, Segment::Key("slotInfo"), Segment::Doc];
        let mut path = path("abilitySlotDataList", &rest);
        path.extend([Segment::Key("values"), Segment::Index(0)]);
        document.set(&path, &Scalar::Text(String::new())).unwrap();
    });
    assert!(found.iter().any(|text| text.ends_with("not in exactly one slot")), "{found:?}");
}

#[test]
fn a_wrong_slot_key_is_caught() {
    let found = after(|document| {
        let rest = [Segment::Key("target"), Segment::Index(0), Segment::Doc, Segment::Key("slotInfo"), Segment::Doc];
        let mut path = path("abilitySlotDataList", &rest);
        path.extend([Segment::Key("keys"), Segment::Index(0)]);
        document.set(&path, &Scalar::Int(9)).unwrap();
    });
    assert!(found.iter().any(|text| text.ends_with("not keys 1 to 3 with three values")), "{found:?}");
}

#[test]
fn a_wrong_addition_order_is_caught() {
    let found = after(|document| {
        let path = path("additionOrderOwnedAbilityIds", &[Segment::Key("target"), Segment::Index(0)]);
        document.set(&path, &Scalar::Int(999)).unwrap();
    });
    assert!(found.iter().any(|text| text.ends_with("not the contentIds of abilityList")), "{found:?}");
}

#[test]
fn a_wrong_learned_list_is_caught() {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    let path = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("learnedAbilityList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(0),
    ];
    document.set(&path, &Scalar::Int(99)).unwrap();
    let found = ability_violations(&Tree::parse(&document.to_json()));
    assert!(found.iter().any(|text| text.contains("learnedAbilityList")), "{found:?}");
}

/// The eight elements of the spell caster (character 3), the two learned
/// spells (abilities 5 and 6, both level 1) in the element whose own level is
/// `holder`. The elements follow `order`.
fn elements(order: [u32; 8], holder: u32) -> Vec<String> {
    let empty = "\"\"";
    order
        .iter()
        .map(|&level| {
            if level == holder {
                level_element(level, &[1, 2, 3], &[&ability(5), &ability(6), empty])
            } else {
                level_element(level, &[1, 2, 3], &[empty, empty, empty])
            }
        })
        .collect()
}

#[test]
fn elements_out_of_order_are_judged_by_their_own_level() {
    let (_, text) = model_with_slots(3, &elements([2, 1, 3, 4, 5, 6, 7, 8], 1));
    assert_eq!(ability_violations(&Tree::parse(&text)), Vec::<String>::new());
}

#[test]
fn a_spell_in_an_element_of_another_level_is_caught() {
    let (_, text) = model_with_slots(3, &elements([1, 2, 3, 4, 5, 6, 7, 8], 2));
    let found = ability_violations(&Tree::parse(&text));
    assert!(found.iter().any(|text| text.ends_with("sits at the wrong spell level")), "{found:?}");
}

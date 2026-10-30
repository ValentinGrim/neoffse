//! The `type_id` to kind rule of `tools/pr-tables/prtables/kinds.py`, restated
//! once so a drift between the tool and the parser shows in a test
//! (`docs/formats/pixel-remaster.md#ability-types`).

use neoffse_data::AbilityKind;

/// The kind of an `ability.csv` `type_id`; `None` outside the closed table.
pub(crate) fn kind_of(type_id: u32) -> Option<AbilityKind> {
    Some(match type_id {
        1 => AbilityKind::WhiteMagic,
        2 => AbilityKind::BlackMagic,
        3 | 13 => AbilityKind::Monster,
        9 => AbilityKind::Summon,
        10 => AbilityKind::Ninjutsu,
        11 => AbilityKind::Other,
        501 => AbilityKind::TimeMagic,
        502 => AbilityKind::MagicSword,
        504 => AbilityKind::BlueMagic,
        601 => AbilityKind::Hissatsu,
        602 => AbilityKind::SpecialAbility,
        603 => AbilityKind::Dance,
        604 => AbilityKind::Madou,
        625 => AbilityKind::Rampage,
        12 | 34 | 35 | 40 | 41 | 627..=629 | 638 | 639 => AbilityKind::Internal,
        4..=8 | 14..=33 | 36..=38 | 301..=306 | 503 | 505..=508 | 512..=516 | 605..=623 | 637 => AbilityKind::Command,
        _ => return None,
    })
}

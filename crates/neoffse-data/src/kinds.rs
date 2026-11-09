//! The ability kinds: the strings of `abilities.toml` and the `type_id` table.

use crate::entry::AbilityKind;

/// The kind a `kind` string of `abilities.toml` names; `None` for any other string.
pub(crate) fn ability_kind(name: &str) -> Option<AbilityKind> {
    Some(match name {
        "white_magic" => AbilityKind::WhiteMagic,
        "black_magic" => AbilityKind::BlackMagic,
        "monster" => AbilityKind::Monster,
        "command" => AbilityKind::Command,
        "summon" => AbilityKind::Summon,
        "ninjutsu" => AbilityKind::Ninjutsu,
        "other" => AbilityKind::Other,
        "time_magic" => AbilityKind::TimeMagic,
        "magic_sword" => AbilityKind::MagicSword,
        "blue_magic" => AbilityKind::BlueMagic,
        "hissatsu" => AbilityKind::Hissatsu,
        "special_ability" => AbilityKind::SpecialAbility,
        "dance" => AbilityKind::Dance,
        "madou" => AbilityKind::Madou,
        "rampage" => AbilityKind::Rampage,
        "internal" => AbilityKind::Internal,
        _ => return None,
    })
}

/// The kind of an `ability.csv` `type_id`; `None` outside the closed table.
/// The table restates `tools/pr-tables/prtables/kinds.py`
/// (`docs/formats/pixel-remaster.md#ability-types`): a drift between the two
/// fails the parse of a regenerated file.
pub(crate) fn kind_of_type(type_id: u32) -> Option<AbilityKind> {
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

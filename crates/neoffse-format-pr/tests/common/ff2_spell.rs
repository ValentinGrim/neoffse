//! Helpers of the FF II spell Level tests: the edit and the document paths one
//! Level edit can change.

use std::fmt::Write;

use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, Game, Value};
use neoffse_data::{GameTables, Sources};
use neoffse_format_pr::ff2::{Ff2Model, names};

use super::json_diff::{Change, value};

/// `Set character.<id>.spell.<chain>` to `level`.
pub(crate) fn level_edit(id: u32, chain: u32, level: i64) -> Edit {
    Edit::set(names::spell_field(id, chain), Value::Int(level))
}

/// Ability rows 1 to `count`, each a white magic row of level 0 with the
/// content id `100 + id`. Row `named` carries a name; 0 names none.
pub(crate) fn spell_rows(count: u32, named: u32) -> String {
    let mut rows = String::new();
    for id in 1..=count {
        let name = if id == named { "name_en = \"Cure\"\nname_fr = \"Soin\"\n" } else { "" };
        write!(
            rows,
            "[[ability]]\nid = {id}\ncontent_id = {}\ntype_id = 1\nkind = \"white_magic\"\nlevel = 0\n{name}\n",
            100 + id
        )
        .unwrap();
    }
    rows
}

/// FF II tables of one character, two curve totals and the ability rows
/// `abilities`. `chains` is the body of `[spell_chains]` (`first` and
/// `length`) and `bounds` the whole `[bounds]` table.
pub(crate) fn small_tables(abilities: &str, chains: &str, bounds: &str) -> &'static GameTables {
    let skills = format!(
        "[[status]]\nid = 1\nhp = 10\nmp = 5\npower = 1\nvitality = 2\nagility = 3\nintelligence = 4\nspirit = 5\nmagic = 6\n\n\
         [skill_curve]\ntotal_exp = [0, 7]\n\n[spell_chains]\n{chains}"
    );
    let sources = Sources {
        content: "[[content]]\nid = 5\nkind = \"item\"\nname_en = \"Ether\"\nname_fr = \"Éther\"\n",
        abilities,
        jobs: "[[job]]\nid = 1\nname_en = \"Warrior\"\nname_fr = \"Guerrier\"\n",
        commands: "[[command]]\nid = 1\nname_en = \"Attack\"\nname_fr = \"Attaque\"\n",
        monsters: None,
        bounds,
        growth: None,
        skills: Some(&skills),
    };
    // The model keeps the tables for its whole life: leak them for `'static`.
    Box::leak(Box::new(GameTables::parse(Game::Ff2Pr, &sources).unwrap()))
}

/// Replays `set` on the model under `StorageOnly`, the way a session replays
/// an undo or a redo.
pub(crate) fn replay(model: &mut Ff2Model, set: &ChangeSet) -> Result<ChangeSet, EditError> {
    set.apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy))
}

/// Where the places of one spell sit: the character's position in the party
/// list, the entry index in `abilityList`, the index of the key-3 value in
/// the dictionary and the entry index in its list, and the position of the
/// old content id in each order list when it holds it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Where {
    pub character: usize,
    pub list: usize,
    pub copy: usize,
    pub addition: Option<usize>,
    pub sort: Option<usize>,
}

impl Where {
    /// The paths a Level edit changes when it moves `parts` of the two
    /// entries (`abilityId`, `contentId`, `skillLevel`) and the order places.
    /// The key-3 value is the first of the dictionary.
    pub(crate) fn changes(&self, parts: &[&str]) -> Vec<Change> {
        self.changes_of(parts, parts, 0)
    }

    /// As [`changes`](Self::changes), with the parts of each entry apart and
    /// the index of the key-3 value in the dictionary.
    pub(crate) fn changes_of(&self, list_parts: &[&str], copy_parts: &[&str], values: usize) -> Vec<Change> {
        let base = format!("userData.ownedCharacterList.target[{}]", self.character);
        let mut paths = Vec::new();
        for part in list_parts {
            paths.push(format!("{base}.abilityList.target[{}].{part}", self.list));
        }
        for part in copy_parts {
            paths.push(format!("{base}.abilityDictionary.values[{values}].target[{}].{part}", self.copy));
        }
        if let Some(position) = self.addition {
            paths.push(format!("{base}.additionOrderOwnedAbilityIds.target[{position}]"));
        }
        if let Some(position) = self.sort {
            paths.push(format!("{base}.sortOrderOwnedAbilityIds.target[{position}]"));
        }
        paths.iter().map(|path| value(path)).collect()
    }
}

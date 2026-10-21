//! Helpers shared by the test files of `neoffse-data`.
#![allow(dead_code, reason = "each test file uses a different part of these helpers")]

use neoffse_core::Game;
use neoffse_data::{GameTables, Sources, tables_for};

/// The cached tables of FF I.
pub(crate) fn ff1() -> &'static GameTables {
    tables_for(Game::Ff1Pr).unwrap()
}

pub(crate) const CONTENT: &str = r#"# minimal/content.toml
# Written by hand for the tests: two kinds of content and a placeholder.

[[content]]
id = 5
kind = "item"
name_en = "Ether"
name_fr = "Éther"

[[content]]
id = 8
kind = "weapon"
name_en = "Dagger"
name_fr = "Dague"

[[content]]
id = 9
kind = "weapon"
placeholder = true
name_en = "Empty"
name_fr = "---"
"#;

pub(crate) const ABILITIES: &str = r#"# minimal/abilities.toml

[[ability]]
id = 1
content_id = 209
kind = "command"
level = 0
name_en = "Attack"
name_fr = "Attaque"

[[ability]]
id = 3
content_id = 400
kind = "white_magic"
level = 1
name_en = "Cure"
name_fr = "Soin"
"#;

pub(crate) const JOBS: &str = r#"# minimal/jobs.toml

[[job]]
id = 1
name_en = "Warrior"
name_fr = "Guerrier"

[[job]]
id = 2
name_en = "Thief"
name_fr = "Voleur"
"#;

pub(crate) const COMMANDS: &str = r#"# minimal/commands.toml

[[command]]
id = 1
name_en = "Attack"
name_fr = "Attaque"

[[command]]
id = 2
name_en = "Defend"
name_fr = "Défense"
"#;

pub(crate) const MONSTERS: &str = r#"# minimal/monsters.toml

[[monster]]
id = 1
name_en = "Goblin"
name_fr = "Gobelin"

[[monster]]
id = 2
name_en = "Lich"
name_fr = "Liche"
"#;

pub(crate) const BOUNDS: &str = r"# minimal/bounds.toml

[bounds]
MAX_GIL = 999999
MAX_HP = 999
MAX_LV = 99
";

/// Six small valid texts: two rows per array file (three in `content`) and
/// three caps.
pub(crate) fn minimal() -> Sources<'static> {
    Sources {
        content: CONTENT,
        abilities: ABILITIES,
        jobs: JOBS,
        commands: COMMANDS,
        monsters: MONSTERS,
        bounds: BOUNDS,
    }
}

//! Helpers shared by the test files of `neoffse-data`.
#![allow(dead_code, reason = "each test file uses a different part of these helpers")]

pub(crate) mod kind_rule;

use neoffse_core::Game;
use neoffse_data::{DataError, GameTables, Sources, tables_for};

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
type_id = 4
kind = "command"
level = 0
name_en = "Attack"
name_fr = "Attaque"

[[ability]]
id = 3
content_id = 400
type_id = 1
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
MAX_LV = 4
";

/// `BOUNDS` without the `MAX_LV` line: the level count is unchecked.
pub(crate) const BOUNDS_NO_LEVEL: &str = r"# minimal/bounds.toml

[bounds]
MAX_GIL = 999999
MAX_HP = 999
";

pub(crate) const GROWTH: &str = r"# minimal/growth.toml

[[character_status]]
id = 1
hp = 10
vitality = 7

[[character_status]]
id = 2
hp = 20
vitality = 8

[[character_status]]
id = 4
hp = 30
vitality = 9

[exp_table]
total_exp = [0, 5, 12, 40]

[[intermediate_growth_curve]]
job_id = 1
growth_curve_group_id = 1

[[intermediate_growth_curve]]
job_id = 2
growth_curve_group_id = 2

[[intermediate_growth_curve]]
job_id = 5
growth_curve_group_id = 2

[[growth_curve]]
group_id = 1
first_lv = 2
hp_value1 = [0, 1, 0]
strength = [1, 1, 1]
vitality = [0, 1, 2]
agility = [0, 0, 0]
intelligence = [0, 0, 0]
spirit = [0, 0, 0]
magic = [0, 0, 0]
luck = [1, 1, 1]
accuracy_rate = [4, 4, 4]
magic_evasion_rate = [5, 5, 5]
magical_times1 = [1, 1, 1]
magical_times2 = [2, 2, 2]
magical_times3 = [3, 3, 3]
magical_times4 = [4, 4, 4]
magical_times5 = [5, 5, 5]
magical_times6 = [6, 6, 6]
magical_times7 = [7, 7, 7]
magical_times8 = [8, 8, 8]

[[growth_curve]]
group_id = 2
first_lv = 2
hp_value1 = [0, 1, 0]
strength = [2, 2, 2]
vitality = [0, 1, 2]
agility = [0, 0, 0]
intelligence = [0, 0, 0]
spirit = [0, 0, 0]
magic = [0, 0, 0]
luck = [1, 1, 1]
accuracy_rate = [5, 5, 5]
magic_evasion_rate = [6, 6, 6]
magical_times1 = [1, 1, 1]
magical_times2 = [2, 2, 2]
magical_times3 = [3, 3, 3]
magical_times4 = [4, 4, 4]
magical_times5 = [5, 5, 5]
magical_times6 = [6, 6, 6]
magical_times7 = [7, 7, 7]
magical_times8 = [8, 8, 8]
";

/// `[a, b, c]` of `value(level)` for the levels 2 to `last`.
fn column(last: u32, value: impl Fn(u32) -> u32) -> String {
    let values: Vec<String> = (2..=last).map(|level| value(level).to_string()).collect();
    format!("[{}]", values.join(", "))
}

/// One `[[growth_curve]]` entry of the minimal formula (see `GROWTH`) for
/// group `g`, covering the levels 2 to `last`.
pub(crate) fn group_text(g: u32, last: u32) -> String {
    let mut lines = vec!["[[growth_curve]]".to_string(), format!("group_id = {g}"), "first_lv = 2".to_string()];
    let mut line = |key: &str, value: &dyn Fn(u32) -> u32| lines.push(format!("{key} = {}", column(last, value)));
    line("hp_value1", &|level| level % 2);
    line("strength", &|_| g);
    line("vitality", &|level| level - 2);
    line("agility", &|_| 0);
    line("intelligence", &|_| 0);
    line("spirit", &|_| 0);
    line("magic", &|_| 0);
    line("luck", &|_| 1);
    line("accuracy_rate", &|_| g + 3);
    line("magic_evasion_rate", &|_| g + 4);
    for k in 1..=8 {
        line(&format!("magical_times{k}"), &|_| k);
    }
    lines.join("\n") + "\n\n"
}

/// `(job_id, group)` pairs as `[[intermediate_growth_curve]]` entries.
pub(crate) fn job_text(jobs: &[(u32, u32)]) -> String {
    let rows: Vec<String> = jobs
        .iter()
        .map(|(job, group)| format!("[[intermediate_growth_curve]]\njob_id = {job}\ngrowth_curve_group_id = {group}\n\n"))
        .collect();
    rows.concat()
}

/// Valid job and group entries: job `g` reads group `g` for every `g` in
/// `groups`, and each group covers the levels 2 to `last`.
pub(crate) fn curve_text(groups: &[u32], last: u32) -> String {
    let jobs: Vec<(u32, u32)> = groups.iter().map(|g| (*g, *g)).collect();
    job_text(&jobs) + &groups.iter().map(|g| group_text(*g, last)).collect::<String>()
}

/// Seven small valid texts, optional ones included: two rows per array file (three in `content`) and
/// three caps.
pub(crate) fn minimal() -> Sources<'static> {
    Sources {
        content: CONTENT,
        abilities: ABILITIES,
        jobs: JOBS,
        commands: COMMANDS,
        monsters: Some(MONSTERS),
        bounds: BOUNDS,
        growth: Some(GROWTH),
    }
}

/// `GameTables::parse` over the minimal sources with `growth` replaced and no
/// `MAX_LV`, so the growth tests choose their own level count.
pub(crate) fn parse_growth(growth: &str) -> Result<GameTables, DataError> {
    GameTables::parse(
        Game::Ff1Pr,
        &Sources {
            growth: Some(growth),
            bounds: BOUNDS_NO_LEVEL,
            ..minimal()
        },
    )
}

/// The file of the `Toml` error a failed parse raises.
pub(crate) fn toml_file(result: Result<GameTables, DataError>) -> &'static str {
    match result.unwrap_err() {
        DataError::Toml { file, .. } => file,
        other => panic!("expected a Toml error, got {other}"),
    }
}

/// The file and id of the `IdOrder` error a failed parse raises.
pub(crate) fn id_order(result: Result<GameTables, DataError>) -> (&'static str, u32) {
    match result.unwrap_err() {
        DataError::IdOrder { file, id } => (file, id),
        other => panic!("expected an IdOrder error, got {other}"),
    }
}

/// The file, row and rule of the `Rule` error a failed parse raises.
pub(crate) fn rule(result: Result<GameTables, DataError>) -> (&'static str, String, &'static str) {
    match result.unwrap_err() {
        DataError::Rule { file, row, rule } => (file, row, rule),
        other => panic!("expected a Rule error, got {other}"),
    }
}

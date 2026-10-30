# SPDX-License-Identifier: MIT OR Apache-2.0
"""The FF II row of the table list (docs/formats/pixel-remaster.md#master-data-per-game)."""

from ..kinds import ABILITY_KINDS, ABILITY_NOTES
from ..specs import (
    AbilitySpec,
    BoundsSpec,
    ContentSpec,
    DirectSpec,
    Game,
    NamedSpec,
    NamePolicy,
    SkillsSpec,
    SourceSpec,
)

GAME: Game = Game(
    id="ff2",
    label="FINAL FANTASY II PR (FF II)",
    steam_app_id=1173780,  # pixel-remaster.md#games
    # pixel-remaster.md#game-master-data: bundle folder and asset prefixes
    bundle_subdir="FINAL FANTASY II_Data/StreamingAssets/aa/StandaloneWindows64",
    master_dir="Assets/GameAssets/Serial/Data/Master/",
    message_dir="Assets/GameAssets/Serial/Data/Message/",
    message_file="system_{lang}.txt",
    languages=("en", "fr"),
    content=ContentSpec(
        file="content.toml",
        csv="content.csv",
        sources={
            # pixel-remaster.md#master-data-per-game (Content types): item or key item
            1: SourceSpec("item.csv", "type_id", {1: "item", 2: "key_item"}),
            # pixel-remaster.md#equipment-slots: parts_group_id is the slot key
            2: SourceSpec("weapon.csv", "parts_group_id", {1: "weapon"}),
            3: SourceSpec(
                "armor.csv",
                "parts_group_id",
                {2: "shield", 3: "armor", 4: "helmet", 5: "gloves"},
            ),
        },
        # pixel-remaster.md#content-types-5-to-9: type 6 is the keyword list, no detail CSV
        direct={6: DirectSpec("keyword", None)},
        # pixel-remaster.md#ability-job-and-command-tables: abilities have their file
        skipped_types=frozenset({4, 5}),
        # pixel-remaster.md#master-data-per-game (Equipment groups): empty-slot rows
        placeholder_keys=frozenset({"MSG_WEAPON_NAME_01", "MSG_SYSTEM_075"}),
        placeholder_kinds=("weapon", "shield", "armor", "helmet", "gloves"),
        notes=(
            "Items, key items, equipment and keywords share one contentId space "
            "(docs/formats/pixel-remaster.md#id-spaces-and-tables).",
            "kind names the table of the entry: item, key_item, weapon, shield, armor, "
            "helmet, gloves or keyword.",
            "The equipment kinds are FF I's readings of parts_group_id 1 to 5; the "
            "group is the fact (docs/formats/pixel-remaster.md#master-data-per-game).",
            "placeholder = true marks the empty-slot row of an equipment table "
            "(docs/formats/pixel-remaster.md#equipment-slots).",
            "keyword rows are the content rows of type 6 "
            "(docs/formats/pixel-remaster.md#content-types-5-to-9).",
            "Names repeat: look an entry up by id. Gil and abilities are not listed here.",
        ),
    ),
    abilities=AbilitySpec(
        file="abilities.toml",
        csv="ability.csv",
        content_csv="content.csv",
        content_type=4,
        kinds=ABILITY_KINDS,
        notes=ABILITY_NOTES,
        names=NamePolicy(none_ok=True),
    ),
    named=(
        NamedSpec(
            file="jobs.toml",
            array="job",
            csv="job.csv",
            name_column="mes_id_name",
            notes=(
                "Look an entry up by id. The nine rows name characters "
                "(docs/formats/pixel-remaster.md#master-data-per-game).",
            ),
        ),
        NamedSpec(
            file="commands.toml",
            array="command",
            csv="command.csv",
            name_column="mes_id_name",
            notes=("Look an entry up by id.",),
        ),
    ),
    monsters=None,
    bounds=BoundsSpec(
        file="bounds.toml",
        csv="game_constant_int.csv",
        key_prefix="MAX_",
        notes=(
            "One entry per game_constant_int.csv row whose key_name starts with MAX_: "
            "the key name as the game spells it, and value1.",
            "docs/formats/pixel-remaster.md#bounds-in-the-master-data says which save "
            "field each named cap bounds; the others have no known save field.",
        ),
    ),
    growth=None,
    skills=SkillsSpec(
        file="skills.toml",
        status_csv="character_status.csv",
        exp_csv="exp_table.csv",
        ability_csv="ability.csv",
        # pixel-remaster.md#spells-in-ff-ii: the chain table, 640 pairs of 40 chains of 16 rows
        misc_asset="serial_misc_assets.json",
        chain_table="magic_level_acquisition_table",
        first_ability_id=5,
        chains=40,
        chain_length=16,
        # pixel-remaster.md#skill-levels-in-ff-ii: the curve length is the highest level
        magical_cap_key="MAX_MAGICAL_LEVEL",
        weapon_cap_key="MAX_WEAPON_LEVEL",
        notes=(
            "status: the base of each character_status.csv row; power reads the column strength "
            "(docs/formats/pixel-remaster.md#character-growth-in-ff-ii).",
            "skill_curve.total_exp: the proficiency needed for each skill level, level 1 first "
            "(docs/formats/pixel-remaster.md#skill-levels-in-ff-ii).",
            "spell_chains: the first ability id of each chain and the rows per chain; the "
            "level of an ability is its id minus the first id of its chain, plus 1 "
            "(docs/formats/pixel-remaster.md#spells-in-ff-ii).",
        ),
    ),
)

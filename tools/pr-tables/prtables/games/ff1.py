# SPDX-License-Identifier: MIT OR Apache-2.0
"""The FF I row of the table list (docs/formats/pixel-remaster.md)."""

from ..kinds import ABILITY_KINDS, ABILITY_NOTES
from ..specs import (
    AbilitySpec,
    BoundsSpec,
    ContentSpec,
    Game,
    GrowthSpec,
    MonsterSpec,
    NamedSpec,
    SourceSpec,
)

GAME: Game = Game(
    id="ff1",
    label="FINAL FANTASY PR (FF I)",
    steam_app_id=1173770,  # pixel-remaster.md#games
    # pixel-remaster.md#game-master-data: bundle folder and asset prefixes
    bundle_subdir="FINAL FANTASY_Data/StreamingAssets/aa/StandaloneWindows64",
    master_dir="Assets/GameAssets/Serial/Data/Master/",
    message_dir="Assets/GameAssets/Serial/Data/Message/",
    message_file="system_{lang}.txt",
    languages=("en", "fr"),
    content=ContentSpec(
        file="content.toml",
        csv="content.csv",
        sources={
            # pixel-remaster.md#game-master-data: item or key item by item.csv type_id
            1: SourceSpec("item.csv", "type_id", {1: "item", 2: "key_item"}),
            # pixel-remaster.md#equipment-slots: parts_group_id is the slot key
            2: SourceSpec("weapon.csv", "parts_group_id", {1: "weapon"}),
            3: SourceSpec(
                "armor.csv",
                "parts_group_id",
                {2: "shield", 3: "armor", 4: "helmet", 5: "gloves"},
            ),
        },
        # pixel-remaster.md#ability-job-and-command-tables: abilities have their file
        skipped_types=frozenset({4, 5}),
        # pixel-remaster.md#id-spaces-and-tables (Placeholders)
        placeholder_keys=frozenset({"MSG_SYSTEM_069", "MSG_SYSTEM_070"}),
        placeholder_kinds=("weapon", "shield", "armor", "helmet", "gloves"),
        notes=(
            "Items, key items and equipment share one contentId space "
            "(docs/formats/pixel-remaster.md#id-spaces-and-tables).",
            "kind names the table of the entry: item, key_item, weapon, shield, armor, "
            "helmet or gloves.",
            "placeholder = true marks the empty-slot row of an equipment table "
            "(docs/formats/pixel-remaster.md#equipment-slots).",
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
    ),
    named=(
        NamedSpec(
            file="jobs.toml",
            array="job",
            csv="job.csv",
            name_column="mes_id_name",
            notes=("Look an entry up by id.",),
        ),
        NamedSpec(
            file="commands.toml",
            array="command",
            csv="command.csv",
            name_column="mes_id_name",
            notes=(
                "Look an entry up by id. A saved commandList holds ids 1 to 5 "
                "(docs/formats/pixel-remaster.md#ability-job-and-command-tables).",
            ),
        ),
    ),
    monsters=MonsterSpec(
        file="monsters.toml",
        array="monster",
        csv="monster.csv",
        name_column="mes_id_name",
        book_csv="picture_book.csv",
        group_csv="picture_book_group.csv",
        notes=(
            "The monsters of the game's bestiary: one monster.csv row per "
            "picture_book_group.csv group, the group's entry with the lowest "
            "picture-book id.",
            "A monster that shares a group with a lower entry is not listed: FF I has "
            "one, the boss-fight Vampire (docs/formats/pixel-remaster.md#bestiary).",
            "Names repeat: look an entry up by id "
            "(docs/formats/pixel-remaster.md#map-and-monster-tables).",
        ),
    ),
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
    growth=GrowthSpec(
        file="growth.toml",
        status_csv="character_status.csv",
        exp_csv="exp_table.csv",
        curve_csv="intermediate_growth_curve.csv",
        step_csv="growth_curve.csv",
        level_cap_key="MAX_LV",
        notes=(
            "character_status: one row per character_status.csv row. id is the "
            "characterStatusId of a save, hp the base HP, vitality the base vitality "
            "of the HP rule "
            "(docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables, "
            "#level-up-in-ff-i).",
            "exp_table: total_exp lists the total experience a character needs to "
            "reach each level, in level order from 1, so the first value is 0.",
            "intermediate_growth_curve: one row per job. job_id is the jobId of a "
            "save, growth_curve_group_id the growth group its level-ups read.",
            "growth_curve: one entry per growth group. Each column is a list in level "
            "order from first_lv, so index 0 is level first_lv. hp_value1 of 1 forces "
            "the strong HP bonus, a stat column of 0 rolls 1 in 8, magical_times1 to "
            "magical_times8 are the spell charges of the level "
            "(docs/formats/pixel-remaster.md#level-up-in-ff-i).",
            "The tool stops when exp_table.csv holds more than one curve (type_id or "
            "group_id) or intermediate_growth_curve.csv names another one.",
            "The tool stops when a growth_curve.csv row uses a mode the rule does not "
            "cover: insert_type_1 or insert_type_2 other than 3, insert_type_3 other "
            "than 1, or hp_value2, mp_value1 or mp_value2 other than 0.",
        ),
    ),
)

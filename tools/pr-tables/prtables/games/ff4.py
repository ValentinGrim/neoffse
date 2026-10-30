# SPDX-License-Identifier: MIT OR Apache-2.0
"""The FF IV row of the table list (docs/formats/pixel-remaster.md#master-data-per-game)."""

from ..kinds import ABILITY_KINDS, ABILITY_NOTES
from ..specs import (
    AbilitySpec,
    BoundsSpec,
    ContentSpec,
    Game,
    NamedSpec,
    NamePolicy,
    SourceSpec,
)

GAME: Game = Game(
    id="ff4",
    label="FINAL FANTASY IV PR (FF IV)",
    steam_app_id=1173800,  # pixel-remaster.md#games
    # pixel-remaster.md#game-master-data: bundle folder and asset prefixes
    bundle_subdir="FINAL FANTASY IV_Data/StreamingAssets/aa/StandaloneWindows64",
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
            # pixel-remaster.md#equipment-slots: parts_group_id is the slot key;
            # group 6 holds arrows and bows and reads as a weapon
            2: SourceSpec("weapon.csv", "parts_group_id", {1: "weapon", 6: "weapon"}),
            3: SourceSpec(
                "armor.csv",
                "parts_group_id",
                {2: "shield", 3: "armor", 4: "helmet", 5: "gloves"},
            ),
        },
        # pixel-remaster.md#ability-job-and-command-tables: abilities have their file
        skipped_types=frozenset({4, 5}),
        # pixel-remaster.md#master-data-per-game (Equipment groups): empty-slot rows
        placeholder_keys=frozenset({"MSG_SYSTEM_166", "MSG_SYSTEM_167"}),
        placeholder_kinds=("weapon", "shield", "armor", "helmet", "gloves"),
        notes=(
            "Items, key items and equipment share one contentId space "
            "(docs/formats/pixel-remaster.md#id-spaces-and-tables).",
            "kind names the table of the entry: item, key_item, weapon, shield, armor, "
            "helmet or gloves.",
            "The equipment kinds are FF I's readings of parts_group_id 1 to 5; the "
            "group is the fact (docs/formats/pixel-remaster.md#master-data-per-game).",
            "weapon rows are parts_group_id 1 and 6; group 6 reads as a weapon "
            "(docs/formats/pixel-remaster.md#master-data-per-game).",
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
        names=NamePolicy(none_ok=True),
    ),
    named=(
        NamedSpec(
            file="jobs.toml",
            array="job",
            csv="job.csv",
            name_column="mes_id_name",
            notes=(
                "Look an entry up by id. Job 15 has no name "
                "(docs/formats/pixel-remaster.md#rows-without-a-name).",
            ),
            names=NamePolicy(none_ok=True),
        ),
        NamedSpec(
            file="commands.toml",
            array="command",
            csv="command.csv",
            name_column="mes_id_name",
            notes=(
                "Look an entry up by id. Command 4 has no name "
                "(docs/formats/pixel-remaster.md#rows-without-a-name).",
            ),
            names=NamePolicy(none_ok=True),
        ),
    ),
    monsters=None,
    bounds=BoundsSpec(
        file="bounds.toml",
        csv="game_constant_int.csv",
        key_prefix="MAX_",
        value2_keys=frozenset({"MAX_GRADUAL_PETRIFICATION"}),
        notes=(
            "One entry per game_constant_int.csv row whose key_name starts with MAX_: "
            "the key name as the game spells it, and value1.",
            "MAX_GRADUAL_PETRIFICATION has a second value, 24, that this file does not carry "
            "(docs/formats/pixel-remaster.md#bounds-in-the-master-data).",
            "docs/formats/pixel-remaster.md#bounds-in-the-master-data says which save "
            "field each named cap bounds; the others have no known save field.",
        ),
    ),
    growth=None,
)

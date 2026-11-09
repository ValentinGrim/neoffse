# SPDX-License-Identifier: MIT OR Apache-2.0
"""The FF VI row of the table list (docs/formats/pixel-remaster.md#master-data-per-game)."""

from ..kinds import ABILITY_KINDS, ABILITY_NOTES
from ..specs import (
    AbilitySpec,
    BoundsSpec,
    ContentSpec,
    DirectSpec,
    Game,
    NamedSpec,
    NamePolicy,
    SourceSpec,
    StorySpec,
)

GAME: Game = Game(
    id="ff6",
    label="FINAL FANTASY VI PR (FF VI)",
    steam_app_id=1173820,  # pixel-remaster.md#games
    # pixel-remaster.md#game-master-data: bundle folder and asset prefixes
    bundle_subdir="FINAL FANTASY VI_Data/StreamingAssets/aa/StandaloneWindows64",
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
        # pixel-remaster.md#content-types-5-to-9: type 7 points to command.csv and the row
        # keeps its own key (type_value is not unique); type 9 is the 27 espers, an ordinal
        direct={
            7: DirectSpec("command_content", "command.csv"),
            9: DirectSpec("magic_stone", None),
        },
        # pixel-remaster.md#rows-without-a-name: content 1289 has key None and points to
        # command 4, which has no name either
        none_types=frozenset({7}),
        # pixel-remaster.md#ability-job-and-command-tables: abilities have their file
        skipped_types=frozenset({4, 5}),
        # pixel-remaster.md#master-data-per-game (Equipment groups): empty-slot rows
        placeholder_keys=frozenset({"MSG_SYSTEM_187", "MSG_SYSTEM_188"}),
        placeholder_kinds=("weapon", "shield", "armor", "helmet", "gloves"),
        notes=(
            "Items, key items, equipment and command rows share one contentId space "
            "(docs/formats/pixel-remaster.md#id-spaces-and-tables).",
            "kind names the table of the entry: item, key_item, weapon, shield, armor, "
            "helmet, gloves, command_content or magic_stone.",
            "The equipment kinds are FF I's readings of parts_group_id 1 to 5; the "
            "group is the fact, so group 5 (50 rings, 5 gloves) reads as gloves "
            "(docs/formats/pixel-remaster.md#master-data-per-game).",
            "command_content rows are the content rows of type 7, keyed by contentId under "
            "their own key, not by command.csv id "
            "(docs/formats/pixel-remaster.md#content-types-5-to-9).",
            "Content 1289 has no name "
            "(docs/formats/pixel-remaster.md#rows-without-a-name).",
            "magic_stone rows are the content rows of type 9, the 27 espers, contents 62 to 88 "
            "(docs/formats/pixel-remaster.md#content-types-5-to-9).",
            "The content rows of type 4 are not listed here: abilities.toml carries them "
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
        notes=(
            *ABILITY_NOTES,
            "Content ids 1861 and 1863 repeat in content.csv and the game keeps the last "
            "row of each, so abilities 1525, 1527, 1528 and 1529 have no content_id "
            "(docs/formats/pixel-remaster.md#master-data-per-game).",
            "The 89 abilities whose name key sits in a story file only have no name "
            "(docs/formats/pixel-remaster.md#rows-without-a-name).",
        ),
        names=NamePolicy(none_ok=True, unresolved_ok=True),
        # pixel-remaster.md#master-data-per-game (Abilities): 145 abilities have no content row
        # in the file; the last-row rule adds 1525, 1527, 1528 and 1529
        contentless_ok=True,
        last_row_wins=True,
    ),
    named=(
        NamedSpec(
            file="jobs.toml",
            array="job",
            csv="job.csv",
            name_column="mes_id_name",
            notes=(
                "Look an entry up by id. Jobs 20 and 22 take their name from the story file "
                "(docs/formats/pixel-remaster.md#rows-without-a-name).",
            ),
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
        notes=(
            "One entry per game_constant_int.csv row whose key_name starts with MAX_: "
            "the key name as the game spells it, and value1.",
            "docs/formats/pixel-remaster.md#bounds-in-the-master-data says which save "
            "field each named cap bounds; the others have no known save field.",
        ),
    ),
    growth=None,
    # pixel-remaster.md#rows-without-a-name: jobs 20 and 22 (SPEAKER_31, SPEAKER_19) hold
    # their short names in story_mes; the tool keeps those two lines and drops the rest
    story=StorySpec("story_mes_{lang}.txt", (("job.csv", 20), ("job.csv", 22))),
)

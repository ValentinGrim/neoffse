# SPDX-License-Identifier: MIT OR Apache-2.0
"""The table list that drives the export: one `Game` row per game.

Every value comes from docs/formats/pixel-remaster.md. A new game adds a row
and changes no code.
"""

from collections.abc import Mapping
from dataclasses import dataclass


@dataclass(frozen=True)
class SourceSpec:
    """How one content type_id reaches its detail CSV and its kind."""

    csv: str
    kind_column: str  # column of the detail CSV that decides the kind
    kinds: Mapping[int, str]


@dataclass(frozen=True)
class ContentSpec:
    file: str
    csv: str
    sources: Mapping[int, SourceSpec]  # content type_id -> detail CSV
    skipped_types: frozenset[int]  # type_ids another file or nobody exports
    placeholder_keys: frozenset[str]  # name keys of the empty-slot rows
    placeholder_kinds: tuple[str, ...]  # kinds that hold exactly one placeholder
    notes: tuple[str, ...]  # file-specific header lines


@dataclass(frozen=True)
class AbilitySpec:
    file: str
    csv: str
    content_csv: str
    content_type: int  # content type_id of ability rows
    kinds: Mapping[int, str]  # ability.csv type_id -> kind
    notes: tuple[str, ...]


@dataclass(frozen=True)
class NamedSpec:
    """A plain id + name table (jobs, commands)."""

    file: str
    array: str
    csv: str
    name_column: str
    notes: tuple[str, ...]


@dataclass(frozen=True)
class MonsterSpec:
    """The monsters of the bestiary."""

    file: str
    array: str
    csv: str
    name_column: str
    book_csv: str  # columns id, monster_id
    group_csv: str  # columns group, picture_book_id
    notes: tuple[str, ...]


@dataclass(frozen=True)
class BoundsSpec:
    file: str
    csv: str
    key_prefix: str
    notes: tuple[str, ...]


@dataclass(frozen=True)
class GrowthSpec:
    """Base HP per class and experience per level."""

    file: str
    status_csv: str  # columns id, hp
    exp_csv: str  # columns type_id, group_id, lv, total_exp
    curve_csv: str  # column exp_table_group_id
    notes: tuple[str, ...]


@dataclass(frozen=True)
class Game:
    id: str
    label: str
    steam_app_id: int
    bundle_subdir: str  # install folder -> bundle folder, forward slashes
    master_dir: str  # asset path prefixes inside the bundles, trailing "/"
    message_dir: str
    message_file: str
    languages: tuple[str, ...]  # the order of name_en, name_fr
    content: ContentSpec
    abilities: AbilitySpec
    named: tuple[NamedSpec, ...]  # jobs, commands, in this order
    monsters: MonsterSpec
    bounds: BoundsSpec
    growth: GrowthSpec

    def csv_files(self) -> tuple[str, ...]:
        """Every CSV the specs read, sorted, no repeat."""
        names = {self.content.csv, self.abilities.csv, self.abilities.content_csv}
        names.update(source.csv for source in self.content.sources.values())
        names.update(spec.csv for spec in self.named)
        names.update((self.monsters.csv, self.monsters.book_csv, self.monsters.group_csv))
        names.add(self.bounds.csv)
        names.update((self.growth.status_csv, self.growth.exp_csv, self.growth.curve_csv))
        return tuple(sorted(names))


_FF1 = Game(
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
        # pixel-remaster.md#ability-job-and-command-tables
        kinds={
            1: "white_magic",
            2: "black_magic",
            3: "monster",
            13: "monster",
            4: "command",
            5: "command",
            6: "command",
            8: "command",
        },
        notes=(
            "One row per ability.csv row. content_id is the contentId of the ability "
            "object: the content.csv row of type 4 whose type_value is the id.",
            "kind reads ability.csv type_id: white_magic (1), black_magic (2), "
            "monster (3, 13), command (4, 5, 6, 8).",
            "The game names none of these kinds "
            "(docs/formats/pixel-remaster.md#ability-job-and-command-tables).",
            "level is ability_lv: 1 to 8 for a spell, 0 for any other ability. "
            "Names repeat: look an entry up by id.",
        ),
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
        notes=(
            "character_status: one row per character_status.csv row. id is the "
            "characterStatusId of a save, hp the base HP "
            "(docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables).",
            "exp_table: one row per level. total_exp is the total experience a "
            "character needs to reach lv, so level 1 holds 0.",
            "The tool stops when exp_table.csv holds more than one curve (type_id or "
            "group_id) or intermediate_growth_curve.csv names another one.",
        ),
    ),
)

GAMES: tuple[Game, ...] = (_FF1,)

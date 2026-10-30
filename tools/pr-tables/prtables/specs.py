# SPDX-License-Identifier: MIT OR Apache-2.0
"""The specs behind the table list: one `Game` row describes one game.

Every value comes from docs/formats/pixel-remaster.md. A new game adds a row
in prtables/games/ and changes no code.
"""

from collections.abc import Mapping
from dataclasses import dataclass, field


@dataclass(frozen=True)
class NamePolicy:
    """The name keys a table accepts without a name. Both flags default to strict:
    a game turns one on only for rows docs/formats/pixel-remaster.md#rows-without-a-name lists."""

    none_ok: bool = False  # a key "None" exports the row with no name
    unresolved_ok: bool = False  # a key found in no system file exports the row with no name


@dataclass(frozen=True)
class DirectSpec:
    """A content type with no detail CSV: the type fixes the kind
    (docs/formats/pixel-remaster.md#content-types-5-to-9)."""

    kind: str  # the kind string written to content.toml
    target_csv: str | None  # a CSV whose ids type_value must name; None: an ordinal


@dataclass(frozen=True)
class StorySpec:
    """Rows whose name key sits in a story file, and nowhere else the tool reads
    (docs/formats/pixel-remaster.md#rows-without-a-name)."""

    file: str  # story file name with {lang}, under Game.message_dir
    rows: tuple[tuple[str, int], ...]  # (csv name, row id); each csv is a NamedSpec table


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
    direct: Mapping[int, DirectSpec] = field(default_factory=dict)  # type_id -> fixed kind
    none_types: frozenset[int] = frozenset()  # type_ids whose rows may have key "None"


@dataclass(frozen=True)
class AbilitySpec:
    file: str
    csv: str
    content_csv: str
    content_type: int  # content type_id of ability rows
    kinds: Mapping[int, str]  # ability.csv type_id -> kind
    notes: tuple[str, ...]
    names: NamePolicy = NamePolicy()
    contentless_ok: bool = False  # an ability with no type-4 content row exports bare
    last_row_wins: bool = False  # a repeated type-4 content id keeps its last row, as the game does


@dataclass(frozen=True)
class NamedSpec:
    """A plain id + name table (jobs, commands)."""

    file: str
    array: str
    csv: str
    name_column: str
    notes: tuple[str, ...]
    names: NamePolicy = NamePolicy()


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
    value2_keys: frozenset[str] = frozenset()  # keys allowed a non-zero value2; value1 is written


@dataclass(frozen=True)
class GrowthSpec:
    """Base HP and vitality per class, experience per level, growth rows per job."""

    file: str
    status_csv: str  # columns id, hp, vitality
    exp_csv: str  # columns type_id, group_id, lv, total_exp
    curve_csv: str  # columns job_id, growth_curve_group_id, exp_table_group_id
    step_csv: str  # columns group_id, lv, hp_value1, the stat columns, the mode columns
    level_cap_key: str  # game_constant_int.csv key that bounds the level
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
    monsters: MonsterSpec | None  # None: the game writes no monsters file
    bounds: BoundsSpec
    growth: GrowthSpec | None  # None: the game writes no growth file
    story: StorySpec | None = None  # None: the game reads no story file

    def csv_files(self) -> tuple[str, ...]:
        """Every CSV the specs read, sorted, no repeat."""
        names = {self.content.csv, self.abilities.csv, self.abilities.content_csv}
        names.update(source.csv for source in self.content.sources.values())
        names.update(
            direct.target_csv
            for direct in self.content.direct.values()
            if direct.target_csv is not None
        )
        names.update(spec.csv for spec in self.named)
        names.add(self.bounds.csv)
        if self.monsters is not None:
            names.update((self.monsters.csv, self.monsters.book_csv, self.monsters.group_csv))
        if self.growth is not None:
            names.update((self.growth.status_csv, self.growth.exp_csv, self.growth.curve_csv))
            names.add(self.growth.step_csv)
        return tuple(sorted(names))

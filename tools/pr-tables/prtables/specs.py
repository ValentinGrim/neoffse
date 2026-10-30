# SPDX-License-Identifier: MIT OR Apache-2.0
"""The specs behind the table list: one `Game` row describes one game.

Every value comes from docs/formats/pixel-remaster.md. A new game adds a row
in prtables/games/ and changes no code.
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

    def csv_files(self) -> tuple[str, ...]:
        """Every CSV the specs read, sorted, no repeat."""
        names = {self.content.csv, self.abilities.csv, self.abilities.content_csv}
        names.update(source.csv for source in self.content.sources.values())
        names.update(spec.csv for spec in self.named)
        names.add(self.bounds.csv)
        if self.monsters is not None:
            names.update((self.monsters.csv, self.monsters.book_csv, self.monsters.group_csv))
        if self.growth is not None:
            names.update((self.growth.status_csv, self.growth.exp_csv, self.growth.curve_csv))
            names.add(self.growth.step_csv)
        return tuple(sorted(names))

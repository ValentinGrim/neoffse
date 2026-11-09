# SPDX-License-Identifier: MIT OR Apache-2.0
"""Build the rows of skills.toml.

The base stats come from character_status.csv. The proficiency curve is the one
experience curve of exp_table.csv (docs/formats/pixel-remaster.md#skill-levels-in-ff-ii).
The chain law comes from docs/formats/pixel-remaster.md#spells-in-ff-ii: the 640 pairs of
the misc asset make 40 chains of 16 rows, `next` is `current + 1` except on the last row
of a chain, where it is 0, and the level of a row is `(id - first) mod 16 + 1`, which is
the `use_value` of ability.csv. The file stores the compact form: the first id of each
chain and the chain length.
"""

import json
from collections.abc import Mapping
from typing import NamedTuple

from . import ExportError
from .csvtext import Csvs, Row, _index, _int
from .growth import exp_curve
from .specs import SkillsSpec

# pixel-remaster.md#character-growth-in-ff-ii: the columns that hold the base. The first
# is the key written, the second the CSV column; Power reads strength.
_STATS = (
    ("hp", "hp"),
    ("mp", "mp"),
    ("power", "strength"),
    ("vitality", "vitality"),
    ("agility", "agility"),
    ("intelligence", "intelligence"),
    ("spirit", "spirit"),
    ("magic", "magic"),
)
_LUCK = "luck"  # the ninth column of the document; the tool checks it and does not export it


class ChainTable(NamedTuple):
    container: str  # container path of the JSON asset
    nesting: str  # JSON path of the list, `$` for the root
    pairs: list[tuple[int, int]]  # (current_ability_id, next_ability_id), file order


def _lists_named(node: object, name: str, path: str) -> list[tuple[str, object]]:
    """Every value under a key `name`, at any depth, with its JSON path."""
    found: list[tuple[str, object]] = []
    if isinstance(node, dict):
        for key, value in node.items():
            here = f"{path}.{key}"
            if key == name:
                found.append((here, value))
            found.extend(_lists_named(value, name, here))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            found.extend(_lists_named(value, name, f"{path}[{index}]"))
    return found


def _is_id(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def find_chain_table(spec: SkillsSpec, container: str, text: str) -> ChainTable:
    """The one list named `spec.chain_table` in the JSON text of the misc asset."""
    where = container
    try:
        document = json.loads(text)
    except ValueError as error:
        raise ExportError(f"{where}: not JSON: {error}") from error
    found = _lists_named(document, spec.chain_table, "$")
    if len(found) != 1:
        raise ExportError(f"{where}: {len(found)} entries named {spec.chain_table}, expected 1")
    nesting, table = found[0]
    if not isinstance(table, list):
        raise ExportError(f"{where}: {spec.chain_table} is not a list")
    pairs: list[tuple[int, int]] = []
    for index, pair in enumerate(table):
        keys = ("current_ability_id", "next_ability_id")
        if not isinstance(pair, dict) or sorted(pair) != sorted(keys):
            raise ExportError(f"{where}: {spec.chain_table}[{index}] is not a pair of ids")
        current, following = pair[keys[0]], pair[keys[1]]
        if not (_is_id(current) and _is_id(following)):
            raise ExportError(f"{where}: {spec.chain_table}[{index}] holds a non-integer id")
        pairs.append((current, following))
    return ChainTable(container, nesting, pairs)


def check_chains(spec: SkillsSpec, pairs: list[tuple[int, int]], csvs: Csvs) -> list[int]:
    """Check the chain law and return the first ability id of each chain."""
    table, length = spec.chain_table, spec.chain_length
    total = spec.chains * length
    if len(pairs) != total:
        raise ExportError(f"{table}: {len(pairs)} pairs, expected {total}")
    first = spec.first_ability_id
    ids = sorted(current for current, _ in pairs)
    if ids != list(range(first, first + total)):
        raise ExportError(
            f"{table}: current_ability_id values are not {first} to {first + total - 1} each once"
        )
    for current, following in pairs:
        last_row = (current - first) % length == length - 1
        if last_row and following != 0:
            raise ExportError(f"{table}: id {current} ends a chain and its next is {following}")
        if not last_row and following != current + 1:
            raise ExportError(f"{table}: id {current} has next {following}, expected {current + 1}")
    abilities = _index(csvs[spec.ability_csv], spec.ability_csv)
    for current in ids:
        row = abilities.get(current)
        if row is None:
            raise ExportError(f"{spec.ability_csv}: no row for chain id {current}")
        use_value = _int(row, "use_value", where=f"{spec.ability_csv}, id {current}")
        level = (current - first) % length + 1
        if use_value != level:
            raise ExportError(
                f"{spec.ability_csv}, id {current}: use_value {use_value}, "
                f"the chain law gives {level}"
            )
    return list(range(first, first + total, length))


def _status_rows(spec: SkillsSpec, csvs: Csvs) -> list[Row]:
    indexed = _index(csvs[spec.status_csv], spec.status_csv)
    if not indexed:
        raise ExportError(f"{spec.status_csv}: no row")
    rows: list[Row] = []
    for row_id in sorted(indexed):
        if row_id < 1:
            raise ExportError(f"{spec.status_csv}: id {row_id} is below 1")
        where = f"{spec.status_csv}, id {row_id}"
        source = indexed[row_id]
        _int(source, _LUCK, where=where)
        entry: Row = {"id": row_id}
        for key, column in _STATS:
            entry[key] = _int(source, column, where=where)
        rows.append(entry)
    return rows


def _curve(spec: SkillsSpec, csvs: Csvs, bounds: Mapping[str, int]) -> list[int]:
    totals = [int(row["total_exp"]) for row in exp_curve(spec.exp_csv, csvs[spec.exp_csv]).rows]
    for key in (spec.magical_cap_key, spec.weapon_cap_key):
        cap = bounds.get(key)
        if cap is None:
            raise ExportError(f"bounds: no {key}")
        if len(totals) != cap:
            raise ExportError(f"{spec.exp_csv}: {len(totals)} levels, {key} is {cap}")
    return totals


class Skills(NamedTuple):
    sections: dict[str, list[Row] | Row]  # the sections of skills.toml by name, in file order
    statuses: int
    levels: int
    chains: int


def build_skills(
    spec: SkillsSpec, csvs: Csvs, chains: ChainTable, bounds: Mapping[str, int]
) -> Skills:
    first = check_chains(spec, chains.pairs, csvs)
    status = _status_rows(spec, csvs)
    totals = _curve(spec, csvs, bounds)
    sections: dict[str, list[Row] | Row] = {
        "status": status,
        "skill_curve": {"total_exp": totals},
        "spell_chains": {"first": first, "length": spec.chain_length},
    }
    return Skills(sections, len(status), len(totals), len(first))

# SPDX-License-Identifier: MIT OR Apache-2.0
"""Build the rows of each table from the parsed master CSVs and message files.

Every builder stops with an `ExportError` on a fact it does not expect: no
default kind, no skipped row, no guessed name
(docs/formats/pixel-remaster.md#game-master-data). A row exports without a
name only where the game's spec allows it (prtables/names.py).
"""

import re
from collections.abc import Mapping

from . import ExportError
from .csvtext import Csvs, Names, Row, _index, _int, _text
from .names import NO_CONTENT, Tally, name_fields
from .specs import (
    AbilitySpec,
    BoundsSpec,
    ContentSpec,
    DirectSpec,
    MonsterSpec,
    NamedSpec,
    NamePolicy,
    SourceSpec,
)

_BOUND_KEY = re.compile(r"[A-Z][A-Z0-9_]*")


def _source_kind(
    cid: int, source: SourceSpec, type_value: int, details: Mapping[str, dict[int, dict[str, str]]]
) -> str:
    detail = details[source.csv].get(type_value)
    if detail is None:
        raise ExportError(f"content {cid}: {source.csv} has no row {type_value}")
    value = _int(detail, source.kind_column, where=f"{source.csv} id {type_value}")
    kind = source.kinds.get(value)
    if kind is None:
        raise ExportError(
            f"content {cid}: {source.csv} column {source.kind_column} holds {value}, "
            "which no kind maps"
        )
    return kind


def _direct_kind(
    cid: int, direct: DirectSpec, type_value: int, targets: Mapping[str, dict[int, dict[str, str]]]
) -> str:
    if direct.target_csv is not None and type_value not in targets[direct.target_csv]:
        raise ExportError(f"content {cid}: {direct.target_csv} has no row {type_value}")
    return direct.kind


def _check_placeholders(spec: ContentSpec, rows: list[Row]) -> None:
    counts = {kind: 0 for kind in spec.placeholder_kinds}
    for entry in rows:
        if entry.get("placeholder"):
            kind = str(entry["kind"])
            if kind not in counts:
                raise ExportError(f"content {entry['id']}: placeholder of kind {kind}")
            counts[kind] += 1
    for kind, count in counts.items():
        if count != 1:
            raise ExportError(f"kind {kind} holds {count} placeholders, expected 1")


def build_content(spec: ContentSpec, csvs: Csvs, names: Names, tally: Tally) -> list[Row]:
    def exported(row: dict[str, str]) -> bool:
        where = f"{spec.csv} id {row.get('id')}"
        return _int(row, "type_id", where=where) not in spec.skipped_types

    both = spec.sources.keys() & spec.direct.keys()
    if both:
        raise ExportError(f"content type {min(both)} has a detail CSV and a fixed kind")
    # FF VI repeats ids on type-4 rows, which another file exports: index only these rows
    content = _index(csvs[spec.csv], spec.csv, exported)
    details = {
        source.csv: _index(csvs[source.csv], source.csv) for source in spec.sources.values()
    }
    targets = {
        direct.target_csv: _index(csvs[direct.target_csv], direct.target_csv)
        for direct in spec.direct.values()
        if direct.target_csv is not None
    }
    rows: list[Row] = []
    for cid in sorted(content):
        row = content[cid]
        where = f"{spec.csv} id {cid}"
        type_id = _int(row, "type_id", where=where)
        source, direct = spec.sources.get(type_id), spec.direct.get(type_id)
        if source is not None:
            kind = _source_kind(cid, source, _int(row, "type_value", where=where), details)
        elif direct is not None:
            kind = _direct_kind(cid, direct, _int(row, "type_value", where=where), targets)
        else:
            raise ExportError(f"content {cid}: type {type_id} has no source and is not skipped")
        key = _text(row, "mes_id_name", where=where)
        entry: Row = {"id": cid, "kind": kind}
        if key in spec.placeholder_keys:
            entry["placeholder"] = True
        policy = NamePolicy(none_ok=type_id in spec.none_types)
        entry.update(
            name_fields(names, key, label=f"content {cid}", policy=policy, story=None, tally=tally)
        )
        rows.append(entry)
    _check_placeholders(spec, rows)
    return rows


def _ability_contents(spec: AbilitySpec, csvs: Csvs) -> dict[int, dict[str, str]]:
    """The content rows of the ability type, by `type_value` (the ability id).

    A repeated content id stops the run unless `spec.last_row_wins`: then the later row
    replaces the earlier one before `type_value` is read, as the game's loader does
    (docs/formats/pixel-remaster.md#master-data-per-game, repeated IDs)."""
    by_id: dict[int, dict[str, str]] = {}
    for row in csvs[spec.content_csv]:
        where = f"{spec.content_csv} id {row.get('id')}"
        if _int(row, "type_id", where=where) != spec.content_type:
            continue
        content_id = _int(row, "id", where=spec.content_csv)
        if content_id in by_id and not spec.last_row_wins:
            raise ExportError(
                f"{spec.content_csv}: id {content_id} repeats in type {spec.content_type}"
            )
        by_id[content_id] = row
    by_value: dict[int, dict[str, str]] = {}
    for content_id, row in by_id.items():
        value = _int(row, "type_value", where=f"{spec.content_csv} id {content_id}")
        if value in by_value:
            raise ExportError(
                f"{spec.content_csv}: type_value {value} repeats in type {spec.content_type}"
            )
        by_value[value] = row
    return by_value


def build_abilities(
    spec: AbilitySpec, csvs: Csvs, names: Names, tally: Tally
) -> tuple[list[Row], int]:
    by_value = _ability_contents(spec, csvs)
    abilities = _index(csvs[spec.csv], spec.csv)
    rows: list[Row] = []
    for aid in sorted(abilities):
        row = abilities[aid]
        where = f"{spec.csv} id {aid}"
        content_row = by_value.get(aid)
        if content_row is None and not spec.contentless_ok:
            raise ExportError(
                f"ability {aid}: no {spec.content_csv} row of type {spec.content_type}"
            )
        type_id = _int(row, "type_id", where=where)
        kind = spec.kinds.get(type_id)
        if kind is None:
            raise ExportError(f"ability {aid}: type {type_id} of {spec.csv} maps to no kind")
        entry: Row = {"id": aid}
        if content_row is None:
            tally[NO_CONTENT] += 1
        else:
            entry["content_id"] = _int(content_row, "id", where=spec.content_csv)
        entry.update({"type_id": type_id, "kind": kind})
        entry["level"] = _int(row, "ability_lv", where=where)
        if content_row is not None:
            content_where = f"{spec.content_csv} id {entry['content_id']}"
            key = _text(content_row, "mes_id_name", where=content_where)
            entry.update(
                name_fields(
                    names, key, label=f"ability {aid}", policy=spec.names, story=None, tally=tally
                )
            )
        rows.append(entry)
    return rows, len(by_value.keys() - abilities.keys())


def build_named(
    spec: NamedSpec,
    csvs: Csvs,
    names: Names,
    tally: Tally,
    story: Mapping[int, Names] | None = None,
) -> list[Row]:
    """`story` maps a row id to its names from the story file; other rows get `None`."""
    indexed = _index(csvs[spec.csv], spec.csv)
    rows: list[Row] = []
    for rid in sorted(indexed):
        key = _text(indexed[rid], spec.name_column, where=f"{spec.csv} id {rid}")
        fields = name_fields(
            names,
            key,
            label=f"{spec.csv} {rid}",
            policy=spec.names,
            story=story.get(rid) if story else None,
            tally=tally,
        )
        rows.append({"id": rid, **fields})
    return rows


def build_monsters(
    spec: MonsterSpec, csvs: Csvs, names: Names
) -> tuple[list[Row], list[tuple[int, int, int]]]:
    monsters = _index(csvs[spec.csv], spec.csv)
    book = _index(csvs[spec.book_csv], spec.book_csv)
    monster_of: dict[int, int] = {}  # picture-book entry -> monster
    entry_of: dict[int, int] = {}  # monster -> picture-book entry
    for entry, row in book.items():
        monster = _int(row, "monster_id", where=f"{spec.book_csv} id {entry}")
        if monster not in monsters:
            raise ExportError(
                f"{spec.book_csv} id {entry}: monster {monster} is not in {spec.csv}"
            )
        if monster in entry_of:
            raise ExportError(
                f"monster {monster} is listed by entries {entry_of[monster]} and {entry}"
            )
        monster_of[entry] = monster
        entry_of[monster] = entry
    for monster in sorted(monsters):
        if monster not in entry_of:
            raise ExportError(f"monster {monster} is in no {spec.book_csv} entry")
    groups: dict[int, list[int]] = {}
    group_of: dict[int, int] = {}  # entry -> group
    for row in csvs[spec.group_csv]:
        where = f"{spec.group_csv} line of id {row.get('id')}"
        group = _int(row, "group", where=where)
        entry = _int(row, "picture_book_id", where=where)
        if entry not in book:
            raise ExportError(f"group {group}: picture-book entry {entry} does not exist")
        if entry in group_of:
            raise ExportError(f"picture-book entry {entry} is in two groups")
        group_of[entry] = group
        groups.setdefault(group, []).append(entry)
    for entry in sorted(book):
        if entry not in group_of:
            raise ExportError(f"picture-book entry {entry} is in no group")
    kept: list[int] = []
    dropped: list[tuple[int, int, int]] = []
    for group in sorted(groups):
        entries = sorted(groups[group])
        kept.append(monster_of[entries[0]])
        dropped.extend((monster_of[entry], monster_of[entries[0]], group) for entry in entries[1:])
    rows: list[Row] = []
    for monster in sorted(kept):
        key = _text(monsters[monster], spec.name_column, where=f"{spec.csv} id {monster}")
        fields = name_fields(
            names, key, label=f"monster {monster}", policy=NamePolicy(), story=None, tally=Tally()
        )
        rows.append({"id": monster, **fields})
    return rows, sorted(dropped)


def build_bounds(spec: BoundsSpec, csvs: Csvs) -> dict[str, int]:
    """The `key_prefix` rows by key. Rows of other keys are not indexed, so their ids may repeat."""

    def bound(row: dict[str, str]) -> bool:
        where = f"{spec.csv} id {row.get('id')}"
        return _text(row, "key_name", where=where).startswith(spec.key_prefix)

    indexed = _index(csvs[spec.csv], spec.csv, bound)
    bounds: dict[str, int] = {}
    for bid in sorted(indexed):
        row = indexed[bid]
        where = f"{spec.csv} id {bid}"
        key = _text(row, "key_name", where=where)
        if not _BOUND_KEY.fullmatch(key):
            raise ExportError(f"{where}: key {key!r} is not upper-case snake case")
        if key in bounds:
            raise ExportError(f"{where}: key {key} repeats")
        if _int(row, "value2", where=where) != 0 and key not in spec.value2_keys:
            raise ExportError(f"{where}: key {key} has a non-zero value2")
        bounds[key] = _int(row, "value1", where=where)
    return bounds

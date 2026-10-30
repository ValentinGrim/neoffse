# SPDX-License-Identifier: MIT OR Apache-2.0
"""Parse the master CSVs and message files and build the rows of each table.

Every builder stops with an `ExportError` on a fact it does not expect: no
default kind, no skipped row, no guessed name
(docs/formats/pixel-remaster.md#game-master-data).
"""

import csv
import io
import re
import unicodedata
from collections.abc import Mapping

from . import ExportError
from .specs import AbilitySpec, BoundsSpec, ContentSpec, MonsterSpec, NamedSpec

Row = dict[str, str | int | bool | list[int]]  # field order is insertion order
Csvs = Mapping[str, list[dict[str, str]]]  # CSV file name -> rows
Names = Mapping[str, Mapping[str, str]]  # language -> message key -> text

_ICON_TAG = re.compile(r"<IC_[A-Z]+>")  # pixel-remaster.md#game-master-data, Icon tags
_DIGITS = re.compile(r"[0-9]+")
_BOUND_KEY = re.compile(r"[A-Z][A-Z0-9_]*")


def parse_csv(text: str, *, name: str) -> list[dict[str, str]]:
    """Read a CSV with a header line. Values stay as read, rows keep file order."""
    text = text.removeprefix("﻿")
    reader = csv.reader(io.StringIO(text, newline=""), strict=True)
    header: list[str] | None = None
    rows: list[dict[str, str]] = []
    try:
        for fields in reader:
            if not fields:
                continue
            if header is None:
                header = fields
                if any(not column for column in header):
                    raise ExportError(f"{name}, line {reader.line_num}: empty column name")
                if len(set(header)) != len(header):
                    raise ExportError(f"{name}, line {reader.line_num}: duplicate column name")
                continue
            if len(fields) != len(header):
                raise ExportError(
                    f"{name}, line {reader.line_num}: {len(fields)} fields, "
                    f"the header has {len(header)}"
                )
            rows.append(dict(zip(header, fields)))
    except csv.Error as error:
        raise ExportError(f"{name}, line {reader.line_num}: {error}") from error
    if header is None:
        raise ExportError(f"{name}: no header line")
    return rows


def parse_messages(text: str, *, name: str) -> dict[str, str]:
    """Read `key<TAB>text` lines. The text keeps everything after the first tab."""
    text = text.removeprefix("﻿")
    messages: dict[str, str] = {}
    lines: dict[str, int] = {}
    for number, line in enumerate(re.split(r"\r?\n", text), start=1):
        if line == "":
            continue
        key, tab, value = line.partition("\t")
        if not tab or not key:
            raise ExportError(f"{name}, line {number}: no key and tab")
        if key in messages:
            raise ExportError(f"{name}: key {key} repeats at lines {lines[key]} and {number}")
        messages[key] = value
        lines[key] = number
    return messages


def clean_name(raw: str, *, where: str) -> str:
    """Drop one leading icon tag and check the rest is a plain name."""
    rest = raw[match.end() :] if (match := _ICON_TAG.match(raw)) else raw
    reason = None
    if not rest:
        reason = "empty name"
    elif rest != rest.strip():
        reason = "space at an end"
    elif "<" in rest or ">" in rest:
        reason = "angle bracket"
    elif any(unicodedata.category(char) == "Cc" for char in rest):
        reason = "control character"
    else:
        try:
            rest.encode("utf-8")
        except UnicodeEncodeError:
            reason = "not encodable as UTF-8"
    if reason:
        raise ExportError(f"{where}: {reason}: {raw!r}")
    return rest


def _text(row: Mapping[str, str], column: str, *, where: str) -> str:
    try:
        return row[column]
    except KeyError:
        raise ExportError(f"{where}: no column {column}") from None


def _int(row: Mapping[str, str], column: str, *, where: str) -> int:
    value = _text(row, column, where=where)
    if not _DIGITS.fullmatch(value):
        raise ExportError(f"{where}, column {column}: not a non-negative integer: {value!r}")
    return int(value)


def _index(rows: list[dict[str, str]], csv_name: str) -> dict[int, dict[str, str]]:
    indexed: dict[int, dict[str, str]] = {}
    for row in rows:
        row_id = _int(row, "id", where=csv_name)
        if row_id in indexed:
            raise ExportError(f"{csv_name}: id {row_id} repeats")
        indexed[row_id] = row
    return indexed


def _names(names: Names, key: str, *, label: str) -> Row:
    """The `name_<lang>` fields of one message key."""
    if key in ("", "None"):
        raise ExportError(f"{label}: no name key ({key!r})")
    fields: Row = {}
    for lang, messages in names.items():
        if key not in messages:
            raise ExportError(f"{label}: key {key} is missing from the {lang} messages")
        fields[f"name_{lang}"] = clean_name(messages[key], where=f"{label}, {lang}, {key}")
    return fields


def build_content(spec: ContentSpec, csvs: Csvs, names: Names) -> list[Row]:
    content = _index(csvs[spec.csv], spec.csv)
    details = {
        source.csv: _index(csvs[source.csv], source.csv) for source in spec.sources.values()
    }
    rows: list[Row] = []
    for cid in sorted(content):
        row = content[cid]
        where = f"{spec.csv} id {cid}"
        type_id = _int(row, "type_id", where=where)
        if type_id in spec.skipped_types:
            continue
        source = spec.sources.get(type_id)
        if source is None:
            raise ExportError(f"content {cid}: type {type_id} has no source and is not skipped")
        type_value = _int(row, "type_value", where=where)
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
        key = _text(row, "mes_id_name", where=where)
        entry: Row = {"id": cid, "kind": kind}
        if key in spec.placeholder_keys:
            entry["placeholder"] = True
        entry.update(_names(names, key, label=f"content {cid}"))
        rows.append(entry)
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
    return rows


def build_abilities(spec: AbilitySpec, csvs: Csvs, names: Names) -> tuple[list[Row], int]:
    by_value: dict[int, dict[str, str]] = {}
    for row in csvs[spec.content_csv]:
        where = f"{spec.content_csv} id {row.get('id')}"
        if _int(row, "type_id", where=where) != spec.content_type:
            continue
        value = _int(row, "type_value", where=where)
        if value in by_value:
            raise ExportError(
                f"{spec.content_csv}: type_value {value} repeats in type {spec.content_type}"
            )
        by_value[value] = row
    abilities = _index(csvs[spec.csv], spec.csv)
    rows: list[Row] = []
    for aid in sorted(abilities):
        row = abilities[aid]
        where = f"{spec.csv} id {aid}"
        content_row = by_value.get(aid)
        if content_row is None:
            raise ExportError(
                f"ability {aid}: no {spec.content_csv} row of type {spec.content_type}"
            )
        type_id = _int(row, "type_id", where=where)
        kind = spec.kinds.get(type_id)
        if kind is None:
            raise ExportError(f"ability {aid}: type {type_id} of {spec.csv} maps to no kind")
        entry: Row = {
            "id": aid,
            "content_id": _int(content_row, "id", where=spec.content_csv),
            "type_id": type_id,
            "kind": kind,
            "level": _int(row, "ability_lv", where=where),
        }
        content_where = f"{spec.content_csv} id {entry['content_id']}"
        key = _text(content_row, "mes_id_name", where=content_where)
        entry.update(_names(names, key, label=f"ability {aid}"))
        rows.append(entry)
    return rows, len(by_value.keys() - abilities.keys())


def build_named(spec: NamedSpec, csvs: Csvs, names: Names) -> list[Row]:
    indexed = _index(csvs[spec.csv], spec.csv)
    rows: list[Row] = []
    for rid in sorted(indexed):
        key = _text(indexed[rid], spec.name_column, where=f"{spec.csv} id {rid}")
        rows.append({"id": rid, **_names(names, key, label=f"{spec.csv} {rid}")})
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
        rows.append({"id": monster, **_names(names, key, label=f"monster {monster}")})
    return rows, sorted(dropped)


def build_bounds(spec: BoundsSpec, csvs: Csvs) -> dict[str, int]:
    indexed = _index(csvs[spec.csv], spec.csv)
    bounds: dict[str, int] = {}
    for bid in sorted(indexed):
        row = indexed[bid]
        where = f"{spec.csv} id {bid}"
        key = _text(row, "key_name", where=where)
        if not key.startswith(spec.key_prefix):
            continue
        if not _BOUND_KEY.fullmatch(key):
            raise ExportError(f"{where}: key {key!r} is not upper-case snake case")
        if key in bounds:
            raise ExportError(f"{where}: key {key} repeats")
        if _int(row, "value2", where=where) != 0:
            raise ExportError(f"{where}: key {key} has a non-zero value2")
        bounds[key] = _int(row, "value1", where=where)
    return bounds

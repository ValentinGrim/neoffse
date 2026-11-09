# SPDX-License-Identifier: MIT OR Apache-2.0
"""Read the CSV and message texts of a game and clean a name.

The row, CSV and message types every builder shares live here too.
"""

import csv
import io
import re
import unicodedata
from collections.abc import Callable, Mapping

from . import ExportError

Row = dict[str, str | int | bool | list[int]]  # field order is insertion order
Csvs = Mapping[str, list[dict[str, str]]]  # CSV file name -> rows
Names = Mapping[str, Mapping[str, str]]  # language -> message key -> text

_ICON_TAG = re.compile(r"<IC_[A-Z]+>")  # pixel-remaster.md#game-master-data, Icon tags
_DIGITS = re.compile(r"[0-9]+")


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


def _index(
    rows: list[dict[str, str]],
    csv_name: str,
    keep: Callable[[dict[str, str]], bool] | None = None,
) -> dict[int, dict[str, str]]:
    """Rows by `id`. A row `keep` rejects is not indexed, so its id may repeat."""
    indexed: dict[int, dict[str, str]] = {}
    for row in rows:
        if keep is not None and not keep(row):
            continue
        row_id = _int(row, "id", where=csv_name)
        if row_id in indexed:
            raise ExportError(f"{csv_name}: id {row_id} repeats")
        indexed[row_id] = row
    return indexed

# SPDX-License-Identifier: MIT OR Apache-2.0
"""Resolve the name of a row from the message files.

A name comes from the `system_*` files. The one exception is the explicit
story list of a game (docs/formats/pixel-remaster.md#rows-without-a-name):
the tool reads those rows' keys in the story file and no other line of it.
A row the game names with nothing exports no name, but only where the game's
spec allows it.
"""

import re
import unicodedata
from collections import Counter
from collections.abc import Collection, Mapping, Sequence

from . import ExportError
from .csvtext import Csvs, Names, Row, _index, _text, clean_name
from .specs import NamePolicy, NamedSpec, StorySpec

Tally = Counter[str]  # reason -> rows; one Counter per output file

NONE_KEY = "with key None"
UNRESOLVED = "with a key found in no system file"
NO_CONTENT = "with no content row"
FROM_STORY = "names read from the story file"


def _check_name_shape(raw: str, *, label: str, lang: str) -> None:
    """Refuse a story text that reads as a line of dialogue: it holds a space or ends in
    punctuation. The message names the row and never quotes the text."""
    if raw and (any(char.isspace() for char in raw) or unicodedata.category(raw[-1])[0] == "P"):
        raise ExportError(
            f"{label}, {lang}: the story text holds a space or ends in punctuation, "
            "so it is not a name"
        )


def _story_fields(names: Names, key: str, *, label: str, story: Names, tally: Tally) -> Row:
    if key == "None":
        raise ExportError(f"{label}: a story-list row has no name key")
    held = [lang for lang, messages in names.items() if key in messages]
    if held:
        raise ExportError(
            f"{label}: key {key} is in the {held[0]} system messages, "
            "so the story list entry is stale"
        )
    fields: Row = {}
    for lang in names:
        text = story.get(lang, {}).get(key)
        if text is None:
            raise ExportError(f"{label}: key {key} is missing from the {lang} story file")
        _check_name_shape(text, label=label, lang=lang)
        fields[f"name_{lang}"] = clean_name(text, where=f"{label}, {lang}, {key}")
    tally[FROM_STORY] += 1
    return fields


def name_fields(
    names: Names,
    key: str,
    *,
    label: str,
    policy: NamePolicy,
    story: Names | None,
    tally: Tally,
) -> Row:
    """The `name_<lang>` fields of one message key; `{}` for a row that has no name.

    `story` is not `None` for a row on the game's story list only."""
    if key == "":
        raise ExportError(f"{label}: no name key ({key!r})")
    if story is not None:
        return _story_fields(names, key, label=label, story=story, tally=tally)
    if key == "None":
        if not policy.none_ok:
            raise ExportError(f"{label}: no name key ({key!r})")
        tally[NONE_KEY] += 1
        return {}
    held = [lang for lang, messages in names.items() if key in messages]
    if not held and policy.unresolved_ok:
        tally[UNRESOLVED] += 1
        return {}
    fields: Row = {}
    for lang, messages in names.items():
        if key not in messages:
            raise ExportError(f"{label}: key {key} is missing from the {lang} messages")
        fields[f"name_{lang}"] = clean_name(messages[key], where=f"{label}, {lang}, {key}")
    return fields


def read_story(text: str, wanted: Collection[str], *, name: str) -> dict[str, str]:
    """The texts of the `wanted` keys of one story file, and nothing else.

    Every other line is dropped unread, so no dialogue leaves this function."""
    keys = set(wanted)
    found: dict[str, str] = {}
    lines: dict[str, int] = {}
    for number, line in enumerate(re.split(r"\r?\n", text.removeprefix("﻿")), start=1):
        key, tab, value = line.partition("\t")
        if not tab or key not in keys:
            continue
        if key in found:
            raise ExportError(f"{name}: key {key} repeats at lines {lines[key]} and {number}")
        found[key] = value
        lines[key] = number
    missing = sorted(keys - found.keys())
    if missing:
        raise ExportError(f"{name}: key {missing[0]} is missing")
    return found


def check_story_tables(spec: StorySpec | None, named: Sequence[NamedSpec]) -> None:
    """A listed CSV that no `NamedSpec` writes is a stale entry."""
    written = {table.csv for table in named}
    for csv_name, row_id in spec.rows if spec else ():
        if csv_name not in written:
            raise ExportError(f"story list: {csv_name} row {row_id} is in no table the tool writes")


def story_rows(
    spec: StorySpec | None, named: NamedSpec, csvs: Csvs, texts: Mapping[str, str]
) -> Mapping[int, Names]:
    """Per listed row id of `named.csv`: `{lang: {key: text}}` for its one key.

    `texts` maps a language to its story file text."""
    if spec is None:
        return {}
    ids = [row_id for csv_name, row_id in spec.rows if csv_name == named.csv]
    if not ids:
        return {}
    if len(set(ids)) != len(ids):
        raise ExportError(f"story list: a {named.csv} row is listed twice")
    indexed = _index(csvs[named.csv], named.csv)
    keys: dict[int, str] = {}
    for row_id in ids:
        row = indexed.get(row_id)
        if row is None:
            raise ExportError(f"story list: {named.csv} has no row {row_id}")
        keys[row_id] = _text(row, named.name_column, where=f"{named.csv} id {row_id}")
    found = {
        lang: read_story(text, set(keys.values()), name=spec.file.format(lang=lang))
        for lang, text in texts.items()
    }
    return {
        row_id: {lang: {key: found[lang][key]} for lang in found} for row_id, key in keys.items()
    }


def tally_notices(game_id: str, file: str, tally: Tally) -> list[str]:
    """The stdout lines of one file's tally, in the order the reasons first occurred."""
    lines = []
    for reason, count in tally.items():
        if reason == FROM_STORY:
            lines.append(f"{game_id}: {file}: {count} {reason}")
        else:
            lines.append(f"{game_id}: {file}: {count} entries {reason}, exported with no name")
    return lines

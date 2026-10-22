# SPDX-License-Identifier: MIT OR Apache-2.0
"""Build the base HP and experience rows of growth.toml.

The rules come from docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables:
one experience curve, levels 1 to N without a gap, total experience rising
strictly from 0 at level 1.
"""

from . import ExportError
from .build import Csvs, Row, _index, _int
from .tables import GrowthSpec


def _status_rows(spec: GrowthSpec, csvs: Csvs) -> list[Row]:
    indexed = _index(csvs[spec.status_csv], spec.status_csv)
    if not indexed:
        raise ExportError(f"{spec.status_csv}: no row")
    rows: list[Row] = []
    for row_id in sorted(indexed):
        if row_id < 1:
            raise ExportError(f"{spec.status_csv}: id {row_id} is below 1")
        where = f"{spec.status_csv}, id {row_id}"
        rows.append({"id": row_id, "hp": _int(indexed[row_id], "hp", where=where)})
    return rows


def _curve_group(spec: GrowthSpec, csvs: Csvs) -> set[int]:
    curve_rows = csvs[spec.curve_csv]
    if not curve_rows:
        raise ExportError(f"{spec.curve_csv}: no row")
    return {
        _int(row, "exp_table_group_id", where=f"{spec.curve_csv}, row {number}")
        for number, row in enumerate(curve_rows, start=1)
    }


def _exp_rows(spec: GrowthSpec, csvs: Csvs) -> list[Row]:
    exp_rows = csvs[spec.exp_csv]
    if not exp_rows:
        raise ExportError(f"{spec.exp_csv}: no row")
    types: set[int] = set()
    groups: set[int] = set()
    totals: dict[int, int] = {}
    for number, row in enumerate(exp_rows, start=1):
        where = f"{spec.exp_csv}, row {number}"
        types.add(_int(row, "type_id", where=where))
        groups.add(_int(row, "group_id", where=where))
        level = _int(row, "lv", where=where)
        if level in totals:
            raise ExportError(f"{spec.exp_csv}: level {level} repeats")
        totals[level] = _int(row, "total_exp", where=f"{spec.exp_csv}, level {level}")
    if len(types) > 1:
        raise ExportError(
            f"{spec.exp_csv}: {len(types)} distinct type_id values, one curve expected"
        )
    if len(groups) > 1:
        raise ExportError(
            f"{spec.exp_csv}: {len(groups)} distinct group_id values, one curve expected"
        )
    if _curve_group(spec, csvs) != groups:
        raise ExportError(
            f"{spec.curve_csv}: exp_table_group_id does not name the one group of {spec.exp_csv}"
        )
    levels = sorted(totals)
    if levels != list(range(1, len(levels) + 1)):
        raise ExportError(f"{spec.exp_csv}: levels do not run 1 to {len(levels)} without a gap")
    if totals[1] != 0:
        raise ExportError(f"{spec.exp_csv}: level 1 does not hold 0 experience")
    for level in levels[1:]:
        if totals[level] <= totals[level - 1]:
            raise ExportError(f"{spec.exp_csv}: total_exp does not rise at level {level}")
    return [{"lv": level, "total_exp": totals[level]} for level in levels]


def build_growth(spec: GrowthSpec, csvs: Csvs) -> tuple[list[Row], list[Row]]:
    """The `character_status` rows, then the `exp_table` rows."""
    return _status_rows(spec, csvs), _exp_rows(spec, csvs)

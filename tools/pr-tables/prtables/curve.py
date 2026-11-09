# SPDX-License-Identifier: MIT OR Apache-2.0
"""Build the job and growth rows of growth.toml.

The rules come from docs/formats/pixel-remaster.md#level-up-in-ff-i: a job
reads one growth group, a group holds one row per level from 2 to the last
level of the experience table, and the rows use the one mode the rule covers.
"""

from . import ExportError
from .build import Csvs, Row, _int
from .specs import GrowthSpec

# Column -> required value. Any other value is a mode the rule does not cover.
_MODES = {
    "insert_type_1": 3,
    "insert_type_2": 3,
    "insert_type_3": 1,
    "hp_value2": 0,
    "mp_value1": 0,
    "mp_value2": 0,
}
# Output order after group_id, lv, hp_value1 (stats in the rule's order).
_VALUES = (
    "strength",
    "vitality",
    "agility",
    "intelligence",
    "spirit",
    "magic",
    "luck",
    "accuracy_rate",
    "magic_evasion_rate",
    *(f"magical_times{number}" for number in range(1, 9)),
)


def _job_rows(spec: GrowthSpec, csvs: Csvs) -> list[Row]:
    rows = csvs[spec.curve_csv]
    if not rows:
        raise ExportError(f"{spec.curve_csv}: no row")
    jobs: dict[int, int] = {}
    for number, row in enumerate(rows, start=1):
        where = f"{spec.curve_csv}, row {number}"
        job = _int(row, "job_id", where=where)
        if job < 1:
            raise ExportError(f"{spec.curve_csv}: job_id {job} is below 1")
        if job in jobs:
            raise ExportError(f"{spec.curve_csv}: job_id {job} repeats")
        jobs[job] = _int(row, "growth_curve_group_id", where=f"{spec.curve_csv}, job {job}")
    return [{"job_id": job, "growth_curve_group_id": jobs[job]} for job in sorted(jobs)]


def _step_row(spec: GrowthSpec, row: dict[str, str], number: int) -> Row:
    where = f"{spec.step_csv}, row {number}"
    group = _int(row, "group_id", where=where)
    level = _int(row, "lv", where=where)
    where = f"{spec.step_csv}, group {group}, level {level}"
    for column, required in _MODES.items():
        mode = _int(row, column, where=where)
        if mode != required:
            raise ExportError(
                f"{where}: {column} is {mode}, the rule covers {required} only"
            )
    strong = _int(row, "hp_value1", where=where)
    if strong > 1:
        raise ExportError(f"{where}: hp_value1 is {strong}, expected 0 or 1")
    out: Row = {"group_id": group, "lv": level, "hp_value1": strong}
    for column in _VALUES:
        out[column] = _int(row, column, where=where)
    return out


def _step_rows(spec: GrowthSpec, csvs: Csvs, last_level: int) -> list[Row]:
    rows = csvs[spec.step_csv]
    if not rows:
        raise ExportError(f"{spec.step_csv}: no row")
    seen: set[tuple[int, int]] = set()
    steps: list[Row] = []
    for number, row in enumerate(rows, start=1):
        step = _step_row(spec, row, number)
        key = (int(step["group_id"]), int(step["lv"]))
        if key in seen:
            raise ExportError(f"{spec.step_csv}: group {key[0]} level {key[1]} repeats")
        seen.add(key)
        steps.append(step)
    steps.sort(key=lambda step: (int(step["group_id"]), int(step["lv"])))
    levels: dict[int, list[int]] = {}
    for step in steps:
        levels.setdefault(int(step["group_id"]), []).append(int(step["lv"]))
    for group, found in levels.items():
        if found != list(range(2, found[-1] + 1)):
            raise ExportError(
                f"{spec.step_csv}: group {group} levels do not run 2 up without a gap"
            )
        if found[-1] != last_level:
            raise ExportError(
                f"{spec.step_csv}: group {group} ends at level {found[-1]}, "
                f"the experience table ends at {last_level}"
            )
    return steps


def _pack(steps: list[Row]) -> list[Row]:
    """One row per group: group_id, first_lv and one list per column, level order.

    `steps` is sorted by (group, level) and every group runs 2 up without a gap."""
    columns = ("hp_value1", *_VALUES)
    lists: dict[int, dict[str, list[int]]] = {}
    firsts: dict[int, int] = {}
    for step in steps:
        group = int(step["group_id"])
        if group not in lists:
            lists[group] = {column: [] for column in columns}
            firsts[group] = int(step["lv"])
        for column in columns:
            lists[group][column].append(int(step[column]))
    packed: list[Row] = []
    for group, values in lists.items():
        row: Row = {"group_id": group, "first_lv": firsts[group]}
        row.update(values)
        packed.append(row)
    return packed


def build_curve(
    spec: GrowthSpec, csvs: Csvs, last_level: int
) -> tuple[list[Row], list[Row]]:
    """The `intermediate_growth_curve` rows, then the `growth_curve` rows."""
    jobs = _job_rows(spec, csvs)
    steps = _step_rows(spec, csvs, last_level)
    groups = {int(step["group_id"]) for step in steps}
    for job in jobs:
        if job["growth_curve_group_id"] not in groups:
            raise ExportError(
                f"{spec.curve_csv}: job {job['job_id']} names group "
                f"{job['growth_curve_group_id']}, which has no row in {spec.step_csv}"
            )
    return jobs, _pack(steps)

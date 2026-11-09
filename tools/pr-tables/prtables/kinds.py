# SPDX-License-Identifier: MIT OR Apache-2.0
"""The one reading of ability.csv type_id as a kind, for every game.

The table is closed: a value outside it stops the tool, so a build that adds a
type asks for a decision instead of landing under "command"
(docs/formats/pixel-remaster.md#ability-types).
"""

from collections.abc import Mapping

# kind -> the type_id values the six builds use for it, one entry per run of values
_KIND_VALUES: tuple[tuple[str, tuple[range, ...]], ...] = (
    ("white_magic", (range(1, 2),)),
    ("black_magic", (range(2, 3),)),
    ("monster", (range(3, 4), range(13, 14))),
    ("summon", (range(9, 10),)),
    ("ninjutsu", (range(10, 11),)),
    ("other", (range(11, 12),)),
    ("time_magic", (range(501, 502),)),
    ("magic_sword", (range(502, 503),)),
    ("blue_magic", (range(504, 505),)),
    ("hissatsu", (range(601, 602),)),
    ("special_ability", (range(602, 603),)),
    ("dance", (range(603, 604),)),
    ("madou", (range(604, 605),)),
    ("rampage", (range(625, 626),)),
    (
        "internal",
        (
            range(12, 13),
            range(34, 36),
            range(40, 42),
            range(627, 630),
            range(638, 640),
        ),
    ),
    (
        "command",
        (
            range(4, 9),
            range(14, 34),
            range(36, 39),
            range(301, 307),
            range(503, 504),
            range(505, 509),
            range(512, 517),
            range(605, 624),
            range(637, 638),
        ),
    ),
)


def _closed_table() -> Mapping[int, str]:
    table: dict[int, str] = {}
    for kind, runs in _KIND_VALUES:
        for run in runs:
            for value in run:
                if value in table:
                    raise ValueError(f"type_id {value} is in {table[value]} and {kind}")
                table[value] = kind
    return table


ABILITY_KINDS: Mapping[int, str] = _closed_table()

ABILITY_NOTES: tuple[str, ...] = (
    "One row per ability.csv row. content_id, when present, is the contentId of the "
    "ability object: the content.csv row of type 4 whose type_value is the id.",
    "type_id is the ability.csv type_id: a value of the game's AbilityType enum "
    "(docs/formats/pixel-remaster.md#ability-types).",
    "kind is one reading of type_id for all six games: white_magic (1), black_magic (2), "
    "monster (3, 13), summon (9), ninjutsu (10), other (11), time_magic (501), "
    "magic_sword (502), blue_magic (504), hissatsu (601), special_ability (602), "
    "dance (603), madou (604), rampage (625), "
    "internal (12, 34, 35, 40, 41, 627, 628, 629, 638, 639), "
    "command (4 to 8, 14 to 33, 36 to 38, 301 to 306, 503, 505 to 508, 512 to 516, "
    "605 to 623, 637).",
    "A row without name_en and name_fr has a name key None, a key in no system message "
    "file, or no content row (docs/formats/pixel-remaster.md#rows-without-a-name).",
    "level is ability_lv. Names repeat: look an entry up by id.",
)

# SPDX-License-Identifier: MIT OR Apache-2.0
"""The table list that drives the export: one `Game` row per game."""

from .games import ff1
from .specs import Game

GAMES: tuple[Game, ...] = (ff1.GAME,)

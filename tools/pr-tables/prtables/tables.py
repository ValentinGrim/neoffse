# SPDX-License-Identifier: MIT OR Apache-2.0
"""The table list that drives the export: one `Game` row per game."""

from .games import ff1, ff2, ff3, ff4, ff5
from .specs import Game

GAMES: tuple[Game, ...] = (ff1.GAME, ff2.GAME, ff3.GAME, ff4.GAME, ff5.GAME)

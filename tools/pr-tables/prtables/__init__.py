# SPDX-License-Identifier: MIT OR Apache-2.0
"""Export the master data of the Pixel Remaster games as TOML tables."""


class ExportError(Exception):
    """A fact the export cannot go on from. The message names the file, row or id."""

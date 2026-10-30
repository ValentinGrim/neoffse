# SPDX-License-Identifier: MIT OR Apache-2.0
"""Write the master data of a Pixel Remaster game as TOML under data/pr/<game>/.

Run it by hand from tools/pr-tables, in a venv with requirements.txt installed,
on a machine where the game is installed. CI never runs it; the repository
commits its output.

    python3 -m prtables --game ff1 --install "<library>/steamapps/common/FINAL FANTASY PR"
    python3 -m prtables --game ff1 --bundles <folder holding the two bundles> --build-id 20249397

`--install` reads the bundles from the game's install folder and the Steam
build id from the appmanifest next to it. `--bundles` takes a folder that
holds the two bundle files directly, for a copy of them. The tool only reads
the bundles. It writes the seven files under <out>/<game>/, and only when
their bytes change.
"""

import argparse
import os
import re
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import NamedTuple

if sys.version_info < (3, 11):  # tomllib, used by the self-check, arrives in 3.11
    raise SystemExit("error: tools/pr-tables needs Python 3.11 or later")

from . import ExportError  # noqa: E402
from .build import (  # noqa: E402
    Row,
    build_abilities,
    build_bounds,
    build_content,
    build_monsters,
    build_named,
    parse_csv,
    parse_messages,
)
from .bundles import find_bundle, parse_build_id, read_assets, sha256_of  # noqa: E402
from .growth import build_growth  # noqa: E402
from .render import (  # noqa: E402
    Provenance,
    check_roundtrip,
    file_header,
    render_array,
    render_arrays,
    render_table,
)
from .tables import GAMES, Game  # noqa: E402

_MASTER_PREFIX = "master_assets_all_"
_MESSAGE_PREFIX = "message_assets_all_"


def _within(inner: Path, outer: Path) -> bool:
    """True when inner is outer or lies below it. Both paths are resolved."""
    return inner == outer or outer in inner.parents


def check_out_dir(out: Path, game_id: str, sources: Sequence[Path]) -> Path:
    """The resolved output folder. It ends in data/pr. Neither it nor the folder
    the tool writes into, <out>/<game_id>, lies inside a source, and no source
    lies inside <out>/<game_id>."""
    resolved = out.resolve()
    if resolved.parts[-2:] != ("data", "pr"):
        raise ExportError(f"output folder must end in data/pr: {out}")
    target = resolved / game_id
    for source in sources:
        folder = source.resolve()
        if _within(resolved, folder) or _within(target, folder):
            raise ExportError(f"output folder {out} lies inside the source folder {source}")
        if _within(folder, target):
            raise ExportError(f"source folder {source} lies inside the output folder {target}")
    return resolved


def _build_id_arg(value: str) -> str:
    if not re.fullmatch(r"[0-9]+", value):
        raise argparse.ArgumentTypeError(f"digits only: {value!r}")
    return value


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="prtables", description=__doc__.split("\n")[0])
    parser.add_argument("--game", required=True, choices=[game.id for game in GAMES])
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--install", type=Path, help="the game's install folder")
    source.add_argument("--bundles", type=Path, help="a folder holding the two bundle files")
    parser.add_argument("--build-id", type=_build_id_arg, help="Steam build id, digits only")
    default_out = Path(__file__).resolve().parents[3] / "data" / "pr"
    parser.add_argument("--out", type=Path, default=default_out, help="default: <repo>/data/pr")
    return parser


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise ExportError(f"cannot read {path.name}: {error}") from error


class _Output(NamedTuple):
    file: str
    text: str
    count: int
    notices: list[str]  # extra stdout lines


def _array_output(
    game: Game, prov: Provenance, file: str, array: str, rows: list[Row],
    notes: Sequence[str], notices: list[str],
) -> _Output:
    text = render_array(file_header(game, file, prov, notes), array, rows)
    check_roundtrip(text, {array: rows})
    return _Output(file, text, len(rows), notices)


def _render_all(
    game: Game, prov: Provenance, texts: Mapping[str, str], messages: Mapping[str, str]
) -> list[_Output]:
    """Build and render the seven files, in order. Nothing touches the disk."""
    csvs = {
        csv: parse_csv(texts[game.master_dir + csv], name=game.master_dir + csv)
        for csv in game.csv_files()
    }
    names = {
        lang: parse_messages(text, name=game.message_dir + game.message_file.format(lang=lang))
        for lang, text in messages.items()
    }
    outputs = []
    spec = game.content
    rows = build_content(spec, csvs, names)
    outputs.append(_array_output(game, prov, spec.file, "content", rows, spec.notes, []))
    spec = game.abilities
    rows, unlinked = build_abilities(spec, csvs, names)
    notices = []
    if unlinked:
        notices.append(
            f"{game.id}: {unlinked} type-{spec.content_type} content rows "
            "link to no ability, not exported"
        )
    outputs.append(_array_output(game, prov, spec.file, "ability", rows, spec.notes, notices))
    for named in game.named:
        rows = build_named(named, csvs, names)
        outputs.append(
            _array_output(game, prov, named.file, named.array, rows, named.notes, [])
        )
    spec = game.monsters
    rows, dropped = build_monsters(spec, csvs, names)
    notices = [
        f"{game.id}: monster {gone} shares group {group} with monster {kept}, not exported"
        for gone, kept, group in dropped
    ]
    outputs.append(_array_output(game, prov, spec.file, spec.array, rows, spec.notes, notices))
    values = build_bounds(game.bounds, csvs)
    header = file_header(game, game.bounds.file, prov, game.bounds.notes)
    text = render_table(header, "bounds", values)
    check_roundtrip(text, {"bounds": values})
    outputs.append(_Output(game.bounds.file, text, len(values), []))
    tables = build_growth(game.growth, csvs)
    header = file_header(game, game.growth.file, prov, game.growth.notes)
    text = render_arrays(header, list(tables.items()))
    check_roundtrip(text, tables)
    count = sum(1 if isinstance(rows, dict) else len(rows) for rows in tables.values())
    outputs.append(_Output(game.growth.file, text, count, []))
    return outputs


def _write_all(game: Game, out_dir: Path, outputs: Sequence[_Output]) -> list[str]:
    """Write each file whose bytes changed. Returns the stdout lines."""
    target = out_dir / game.id
    try:
        target.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise ExportError(f"cannot create {game.id}/: {error}") from error
    lines = []
    notices = []
    for output in outputs:
        data = output.text.encode("utf-8")
        path = target / output.file
        try:
            same = path.is_file() and path.read_bytes() == data
            if not same:
                path.write_bytes(data)
        except OSError as error:
            raise ExportError(f"{game.id}/{output.file}: {error}") from error
        state = "unchanged" if same else "written"
        lines.append(f"{game.id}/{output.file}: {output.count} entries, {state}")
        notices.extend(output.notices)
    return lines + notices


def _build_id(install: Path | None, game: Game, given: str | None) -> str | None:
    if given is not None or install is None:
        return given
    steamapps = Path(os.path.abspath(install)).parent.parent
    manifest = steamapps / f"appmanifest_{game.steam_app_id}.acf"
    return parse_build_id(_read_text(manifest)) if manifest.is_file() else None


def _run(args: argparse.Namespace) -> int:
    game = next(game for game in GAMES if game.id == args.game)
    install: Path | None = args.install
    bundle_dir: Path = install / game.bundle_subdir if install else args.bundles
    if not bundle_dir.is_dir():
        raise ExportError(f"bundle folder not found: {bundle_dir}")
    out_dir = check_out_dir(args.out, game.id, [bundle_dir] + ([install] if install else []))
    try:
        listing = [entry.name for entry in bundle_dir.iterdir()]
    except OSError as error:
        raise ExportError(f"cannot list {bundle_dir}: {error}") from error
    master = bundle_dir / find_bundle(listing, _MASTER_PREFIX)
    message = bundle_dir / find_bundle(listing, _MESSAGE_PREFIX)
    prov = Provenance(
        master.name,
        sha256_of(master),
        message.name,
        sha256_of(message),
        _build_id(install, game, args.build_id),
    )
    texts = read_assets(master, {game.master_dir + csv for csv in game.csv_files()})
    messages = {}
    for lang in game.languages:
        path = game.message_dir + game.message_file.format(lang=lang)
        messages[lang] = read_assets(message, {path})[path]
    outputs = _render_all(game, prov, texts, messages)
    print("\n".join(_write_all(game, out_dir, outputs)))
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return _run(args)
    except ExportError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

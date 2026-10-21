# SPDX-License-Identifier: MIT OR Apache-2.0
"""Find the two Addressables bundles and read TextAssets from them.

Bundle names and asset paths: docs/formats/pixel-remaster.md#game-master-data.
Nothing here writes a file.
"""

import hashlib
import re
from collections.abc import Collection, Iterable
from pathlib import Path
from typing import Any

from . import ExportError

PINNED_UNITYPY = "1.25.4"

_BUILD_ID = re.compile(r'^\s*"buildid"\s+"([0-9]+)"\s*$', re.MULTILINE)


def find_bundle(names: Iterable[str], prefix: str) -> str:
    """The one file name `<prefix><hex>.bundle`. The anchor keeps `common_<prefix>...` out."""
    pattern = re.compile(rf"^{re.escape(prefix)}[0-9a-f]+\.bundle$")
    found = sorted(name for name in names if pattern.match(name))
    if len(found) != 1:
        raise ExportError(f"expected one bundle named {prefix}<hash>.bundle, found {len(found)}")
    return found[0]


def read_assets(bundle: Path, wanted: Collection[str]) -> dict[str, str]:
    """Text of each wanted asset path, keyed as the caller wrote it.

    Container paths compare in lower case: Unity stores them lower-cased
    in some builds. UnityPy is imported here only.
    """
    try:
        import UnityPy
    except ImportError as error:
        raise ExportError(
            "UnityPy is not installed: pip install -r tools/pr-tables/requirements.txt"
        ) from error
    if UnityPy.__version__ != PINNED_UNITYPY:
        raise ExportError(
            f"UnityPy {UnityPy.__version__} is installed, {PINNED_UNITYPY} is pinned: "
            "pip install -r tools/pr-tables/requirements.txt"
        )
    by_lower = {path.lower(): path for path in wanted}
    matches: dict[str, list[Any]] = {path: [] for path in wanted}
    try:
        env = UnityPy.load(str(bundle))
        for container_path, pptr in env.container.items():
            path = by_lower.get(container_path.lower())
            if path is not None:
                matches[path].append(pptr)
        texts: dict[str, str] = {}
        for path, found in matches.items():
            if len(found) != 1:
                raise ExportError(f"{bundle.name}: {path} matches {len(found)} container entries")
            reader = found[0].deref()
            if reader.type.name != "TextAsset":
                raise ExportError(f"{bundle.name}: {path} is a {reader.type.name}, not a TextAsset")
            script = reader.parse_as_object().m_Script
            raw = script.encode("utf-8", "surrogateescape")
            try:
                texts[path] = raw.decode("utf-8-sig")
            except UnicodeDecodeError as error:
                raise ExportError(f"{bundle.name}: {path} is not UTF-8: {error}") from error
        return texts
    except ExportError:
        raise
    except Exception as error:  # UnityPy raises many types on a damaged bundle
        raise ExportError(f"{bundle.name}: cannot read the bundle: {error!r}") from error


def parse_build_id(manifest_text: str) -> str | None:
    """The Steam build id of an `appmanifest_*.acf`, or None."""
    match = _BUILD_ID.search(manifest_text)
    return match.group(1) if match else None


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()

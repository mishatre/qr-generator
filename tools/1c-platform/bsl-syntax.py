#!/usr/bin/env python3
"""Look up platform syntax (1C:Enterprise) from the language extension's dictionary.

The VS Code extension `1c-syntax.language-1c-bsl` ships `lib/bslGlobals.json`, a syntax
dictionary with Russian and English names, descriptions, return types and signatures for
global functions, global variables, system enumerations and the constructible classes.
That file is the reason this project can resolve a platform API name without guessing:
`ПривестиЗначение` is `AdjustValue`, not `CastValue`, and the platform accepts either
spelling at compile time.

The dictionary is partial. It covers the constructible classes, not the whole platform
API: object properties, form elements, metadata objects and events are not in it.

Usage:
    bsl-syntax.py AdjustValue                # a global function, class, method or enum
    bsl-syntax.py ПривестиЗначение
    bsl-syntax.py TypeDescription            # a class, with its methods and constructors
    bsl-syntax.py --list classes
    bsl-syntax.py --search версия
    bsl-syntax.py --json AdjustValue

Exit codes: 0 found, 1 nothing matched, 2 the dictionary could not be read.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import sys
from typing import Any

DEFAULT_PATTERNS = (
    "/root/.vscode-server/extensions/1c-syntax.language-1c-bsl-*/lib/bslGlobals.json",
    "~/.vscode/extensions/1c-syntax.language-1c-bsl-*/lib/bslGlobals.json",
)

SECTIONS = (
    "globalfunctions",
    "globalvariables",
    "systemEnum",
    "classes",
)


def find_dictionary(explicit: str | None) -> str:
    """Return the path to bslGlobals.json, preferring an explicit path or BSL_GLOBALS_JSON."""
    candidates: list[str] = []

    if explicit:
        candidates.append(explicit)

    from_env = os.environ.get("BSL_GLOBALS_JSON")
    if from_env:
        candidates.append(from_env)

    for pattern in DEFAULT_PATTERNS:
        candidates.extend(sorted(glob.glob(os.path.expanduser(pattern)), reverse=True))

    for candidate in candidates:
        if os.path.isfile(candidate):
            return candidate

    raise FileNotFoundError(
        "bslGlobals.json not found. Install the 1c-syntax.language-1c-bsl extension, "
        "or pass --dict / set BSL_GLOBALS_JSON."
    )


def describe(entry: dict[str, Any], indent: str = "") -> list[str]:
    """Render one dictionary entry as readable lines."""
    lines: list[str] = []

    name = entry.get("name", "")
    name_en = entry.get("name_en", "")
    if name_en and name_en != name:
        lines.append(f"{indent}{name}  ({name_en})")
    else:
        lines.append(f"{indent}{name}")

    description = (entry.get("description") or "").strip()
    if description:
        lines.append(f"{indent}  {description}")

    returns = (entry.get("returns") or "").strip()
    if returns:
        lines.append(f"{indent}  returns: {returns}")

    signature = entry.get("signature")
    if isinstance(signature, dict):
        for variant in signature.values():
            if isinstance(variant, dict) and variant.get("СтрокаПараметров"):
                lines.append(f"{indent}  signature: {variant['СтрокаПараметров']}")

    for key in ("methods", "properties", "constructors"):
        children = entry.get(key)
        if isinstance(children, dict) and children:
            lines.append(f"{indent}  {key}: {', '.join(children)}")

    values = entry.get("values")
    if isinstance(values, dict) and values:
        lines.append(f"{indent}  values: {', '.join(list(values)[:20])}")

    return lines


def matches(entry: dict[str, Any], query: str) -> bool:
    """True when the query equals the entry's Russian or English name, case-insensitively."""
    wanted = query.casefold()
    return wanted in {
        str(entry.get("name", "")).casefold(),
        str(entry.get("name_en", "")).casefold(),
    }


def look_up(dictionary: dict[str, Any], query: str) -> list[str]:
    """Return readable lines for every exact match, searching each section in turn."""
    lines: list[str] = []

    for section in ("globalfunctions", "globalvariables", "systemEnum"):
        for entry in dictionary.get(section, {}).values():
            if matches(entry, query):
                lines.append(f"[{section}]")
                lines.extend(describe(entry))
                lines.append("")

    for class_entry in dictionary.get("classes", {}).values():
        if matches(class_entry, query):
            lines.append("[class]")
            lines.extend(describe(class_entry))
            for method in (class_entry.get("methods") or {}).values():
                lines.append("")
                lines.extend(describe(method, indent="  "))
            lines.append("")

        for method in (class_entry.get("methods") or {}).values():
            if matches(method, query):
                lines.append(f"[method of {class_entry.get('name')}]")
                lines.extend(describe(method))
                lines.append("")

    return lines


def search(dictionary: dict[str, Any], needle: str, limit: int) -> list[str]:
    """Return 'section name (name_en) - description' lines for names containing needle."""
    wanted = needle.casefold()
    hits: list[str] = []

    def consider(section: str, entry: dict[str, Any], owner: str = "") -> None:
        name = str(entry.get("name", ""))
        name_en = str(entry.get("name_en", ""))
        if wanted in name.casefold() or wanted in name_en.casefold():
            where = f"{section}/{owner}" if owner else section
            description = (entry.get("description") or "").strip()
            hits.append(f"{where:28} {name} ({name_en}) - {description[:70]}")

    for section, entries in dictionary.items():
        if section not in SECTIONS:
            continue
        for entry in entries.values():
            consider(section, entry)
            for method in (entry.get("methods") or {}).values():
                consider(section, method, owner=str(entry.get("name", "")))

    return hits[:limit]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("query", nargs="?", help="Russian or English name to look up")
    parser.add_argument("--dict", dest="dictionary_path", help="path to bslGlobals.json")
    parser.add_argument("--list", dest="list_section", choices=SECTIONS, help="list a whole section")
    parser.add_argument("--search", help="substring search across names")
    parser.add_argument("--json", action="store_true", help="print matches as JSON")
    parser.add_argument("--limit", type=int, default=40, help="maximum search results")
    arguments = parser.parse_args()

    try:
        path = find_dictionary(arguments.dictionary_path)
        with open(path, encoding="utf-8") as handle:
            dictionary = json.load(handle)
    except (FileNotFoundError, json.JSONDecodeError) as error:
        print(f"cannot read the syntax dictionary: {error}", file=sys.stderr)
        return 2

    if arguments.list_section:
        for entry in dictionary[arguments.list_section].values():
            print(f"{entry.get('name')}  ({entry.get('name_en')})")
        return 0

    if arguments.search:
        results = search(dictionary, arguments.search, arguments.limit)
        for line in results:
            print(line)
        if not results:
            print(f"nothing matched {arguments.search!r}", file=sys.stderr)
            return 1
        return 0

    if not arguments.query:
        parser.print_help()
        return 2

    lines = look_up(dictionary, arguments.query)
    if not lines:
        print(f"nothing matched {arguments.query!r}", file=sys.stderr)
        print("try --search to match on a substring", file=sys.stderr)
        return 1

    if arguments.json:
        print(json.dumps({"query": arguments.query, "path": path, "text": "\n".join(lines).strip()}, ensure_ascii=False, indent=2))
    else:
        print("\n".join(lines).strip())
        print(f"\nsource: {path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

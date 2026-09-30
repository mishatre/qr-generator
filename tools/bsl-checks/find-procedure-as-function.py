#!/usr/bin/env python3
"""Find calls that use a BSL procedure as a function.

A procedure cannot appear in an expression. This scan collects every top-level
Procedure/Function declaration per common module, then looks for call sites of a
*procedure* that sit in an expression position, which the platform rejects with
"Обращение к процедуре как к функции".

It is deliberately textual: it only needs the module name, the member name and
the surrounding operator, which is enough to flag candidates for manual review.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

DECLARATION = re.compile(
    r"^[ \t]*(?:Procedure|Процедура|Function|Функция)[ \t]+([A-Za-z\u0410-\u044f_][\w\u0410-\u044f]*)[ \t]*\(",
    re.MULTILINE,
)
DECLARATION_KIND = re.compile(
    r"^[ \t]*(Procedure|Процедура|Function|Функция)[ \t]+([A-Za-z\u0410-\u044f_][\w\u0410-\u044f]*)[ \t]*\(",
    re.MULTILINE,
)
# `Module.Member(` — the module part is what we resolve against declarations.
QUALIFIED_CALL = re.compile(
    r"([A-Za-z\u0410-\u044f_][\w\u0410-\u044f]*)\.([A-Za-z\u0410-\u044f_][\w\u0410-\u044f]*)[ \t]*\("
)

EXPRESSION_TAIL = re.compile(r"(?:=\s*|Return\s+|Возврат\s+|\+\s*|\(\s*|,\s*)$", re.IGNORECASE)


def module_of(path: Path) -> str | None:
    """Return the common-module name for a Module.bsl under CommonModules/<name>/Ext/."""
    parts = path.parts
    if "CommonModules" not in parts:
        return None
    index = parts.index("CommonModules")
    if index + 1 >= len(parts):
        return None
    return parts[index + 1]


def collect_declarations(root: Path) -> dict[str, dict[str, str]]:
    modules: dict[str, dict[str, str]] = {}
    for path in root.rglob("Module.bsl"):
        name = module_of(path)
        if name is None:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        members: dict[str, str] = {}
        for kind, member in DECLARATION_KIND.findall(text):
            members[member] = "procedure" if kind.lower() in {"procedure", "процедура"} else "function"
        modules[name] = members
    return modules


def scan_file(path: Path, modules: dict[str, dict[str, str]]) -> list[tuple[int, str]]:
    findings: list[tuple[int, str]] = []
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        for module, member in QUALIFIED_CALL.findall(line):
            declared = modules.get(module)
            if declared is None or declared.get(member) != "procedure":
                continue
            before = line[: line.index(f"{module}.{member}(")]
            if EXPRESSION_TAIL.search(before):
                findings.append((number, stripped))
    return findings


def main() -> int:
    roots = [Path(arg) for arg in sys.argv[1:]] or [Path("src")]
    modules = {}
    for root in roots:
        modules.update(collect_declarations(root))

    print(f"# common modules discovered: {len(modules)}")

    total = 0
    for root in roots:
        for path in sorted(root.rglob("*.bsl")):
            for number, text in scan_file(path, modules):
                total += 1
                print(f"{path}:{number}: {text}")
    print(f"# candidates: {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

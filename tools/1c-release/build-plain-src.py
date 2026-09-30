#!/usr/bin/env python3
"""Build a plain-configuration source tree out of the extension sources.

Why this exists: the extension is the convenient way to ship the modules, but a
configuration that already exists wants them merged into it, and the configurator's
compare-and-merge tool reads configurations, not extension sources. So the same
objects are produced a second time - without the extension's name prefix and
without the extension-only properties - and `vrunner cf compile` turns that tree
into a `.cf` whose objects can be compared one by one with the target configuration.

The modules are not copied by hand: this script rewrites the sources, so the
extension and the merge carrier cannot drift apart.

Usage:
  tools/1c-release/build-plain-src.py [--out build/plain/QRCode]
                                      [--zip build/dist/QrGeneratorNative.zip]

The configuration is named after its directory (vrunner derives it that way), so the
default output directory and the <Name> in Configuration.xml have to agree.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import uuid
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

EXTENSION = REPO_ROOT / "src/cfe/QrGenerator"
EXTRA = REPO_ROOT / "src/plain-config"

CONFIGURATION_NAME = "QRCode"

# Longest first: qr_QRCodeСерверПовтИсп has to be renamed before qr_QRCode, or the
# shorter pattern would match inside the longer name and leave a half-renamed file.
RENAMES = [
    ("qr_QRCodeСерверПовтИсп", "QRCodeСерверПовтИсп"),
    ("qr_QrGeneratorNative", "QrGeneratorNative"),
    ("qr_QRCode", "QRCode"),
]

# Properties that only exist on an extension and would be rejected or wrong in a
# plain configuration.
EXTENSION_ONLY = [
    "ObjectBelonging",
    "ConfigurationExtensionPurpose",
    "KeepMappingToExtendedConfigurationObjectsByIDs",
    "NamePrefix",
    "ConfigurationExtensionCompatibilityMode",
]

MODULES = ["QRCode", "QRCodeСерверПовтИсп", "QRCodeПример"]
TEMPLATE = "QrGeneratorNative"


def drop_element(text: str, name: str) -> str:
    """Remove a one-line element, including the whitespace that carries it."""
    return re.sub(rf"^[ \t]*<{name}>.*?</{name}>[ \t]*\n", "", text, flags=re.M)


def drop_block(text: str, name: str) -> str:
    """Remove a possibly multi-line element."""
    return re.sub(rf"^[ \t]*<{name}>.*?</{name}>[ \t]*\n", "", text, flags=re.M | re.S)


def fresh_uuid(text: str, count: int = 1) -> str:
    """Give the first `count` uuid attributes new values, keeping the file's text."""
    for _ in range(count):
        text = re.sub(r'uuid="[0-9a-f-]{36}"', f'uuid="{uuid.uuid4()}"', text, count=1)
    return text


def rename(text: str) -> str:
    for old, new in RENAMES:
        text = text.replace(old, new)
    return text


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def build_configuration_xml() -> str:
    text = fresh_uuid(read(EXTENSION / "Configuration.xml"))
    for name in EXTENSION_ONLY:
        text = drop_element(text, name)

    # InternalInfo stays: the importer refuses a configuration without it
    # ("Отсутствует внутренняя информация (узел InternalInfo) для объекта
    # Configuration"), and the seven root-object entries it carries are the standard
    # set the platform writes into every configuration.

    text = text.replace("<Name>QrGenerator</Name>", f"<Name>{CONFIGURATION_NAME}</Name>")
    # The module text is written with Russian keywords, so the configuration says so.
    text = text.replace("<ScriptVariant>English</ScriptVariant>", "<ScriptVariant>Russian</ScriptVariant>")

    children = "\n".join(f"\t\t\t<CommonModule>{module}</CommonModule>" for module in MODULES)
    children += f"\n\t\t\t<CommonTemplate>{TEMPLATE}</CommonTemplate>"
    text = re.sub(
        r"^[ \t]*<ChildObjects>.*?</ChildObjects>[ \t]*\n",
        f"\t\t<ChildObjects>\n{children}\n\t\t</ChildObjects>\n",
        text,
        flags=re.M | re.S,
    )

    return text


def build_object_xml(path: Path) -> str:
    return rename(fresh_uuid(read(path)))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="build/plain/QRCode", help="configuration tree to write")
    parser.add_argument("--zip", default="build/dist/QrGeneratorNative.zip", help="component archive for the template")
    arguments = parser.parse_args()

    out = (REPO_ROOT / arguments.out).resolve()
    archive = (REPO_ROOT / arguments.zip).resolve()

    if not archive.is_file():
        print(f"the component archive is missing: {archive}", file=sys.stderr)
        print("run tools/native/pack-addin.sh first.", file=sys.stderr)
        return 1

    if out.name != CONFIGURATION_NAME:
        print(f"the output directory has to be named {CONFIGURATION_NAME}: {out.name}", file=sys.stderr)
        return 1

    if out.exists():
        shutil.rmtree(out)

    write(out / "Configuration.xml", build_configuration_xml())

    # The modules and the template come from the extension sources, rewritten. A
    # module that only exists for this route (the example) comes from src/plain-config.
    extension_names = {new: old for old, new in RENAMES}

    for module in MODULES:
        in_extension = extension_names.get(module)
        if in_extension is not None and (EXTENSION / "CommonModules" / f"{in_extension}.xml").is_file():
            object_xml = EXTENSION / "CommonModules" / f"{in_extension}.xml"
            module_bsl = EXTENSION / "CommonModules" / in_extension / "Ext" / "Module.bsl"
        else:
            object_xml = EXTRA / "CommonModules" / f"{module}.xml"
            module_bsl = EXTRA / "CommonModules" / module / "Ext" / "Module.bsl"

        write(out / "CommonModules" / f"{module}.xml", build_object_xml(object_xml))
        write(out / "CommonModules" / module / "Ext" / "Module.bsl", rename(read(module_bsl)))

    write(
        out / "CommonTemplates" / f"{TEMPLATE}.xml",
        build_object_xml(EXTENSION / "CommonTemplates" / "qr_QrGeneratorNative.xml"),
    )
    template = out / "CommonTemplates" / TEMPLATE / "Ext"
    template.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(archive, template / "Template.bin")

    print(f"plain configuration sources: {out}")
    for path in sorted(out.rglob("*")):
        if path.is_file():
            print(f"  {path.relative_to(out)}  {path.stat().st_size} B")

    return 0


if __name__ == "__main__":
    sys.exit(main())

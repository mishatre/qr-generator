#!/usr/bin/env bash
#
# Compile the extension from a staged source tree into a CFE.
#
# `vrunner cfe compile --ibcmd` is not self-contained in vanessa-runner 3.0. It
# loads the extension into the infobase named by autumn-properties.json
# (/F./build/ib) and fails with "Отсутствует файл базы данных" when that base does
# not exist yet. Creating the base is cheap and, like the rest of the ibcmd path,
# needs no 1C licence, so this script creates it when it is missing.
#
# The directory name check is here rather than left to vrunner because its failure
# arrives after the staging work and is worded in a way that does not say which
# directory is at fault. `vrunner cfe compile` takes the extension name from the
# basename of `--src` and validates it against `^[A-Za-zА-Яа-яЁё_][A-Za-zА-Яа-яЁё0-9_]*$`,
# so `build/release-src` is rejected and `build/release/QrGenerator` is not.
#
# Usage:
#   tools/1c-release/compile-extension.sh              # stage, create the base, compile
#   tools/1c-release/compile-extension.sh --no-stage   # compile a tree that is ready
#   tools/1c-release/compile-extension.sh --src build/test/QrGenerator --out build/test/QrGeneratorTests.cfe
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

SRC="build/release/QrGenerator"
OUT="build/out/QrGenerator-v0.1.0.cfe"
BASE="build/ib"
EXPECTED_NAME="QrGenerator"
STAGE=1

while [ $# -gt 0 ]; do
	case "$1" in
		--src)
			SRC="${2:?--src needs a path}"
			shift
			;;
		--out)
			OUT="${2:?--out needs a path}"
			shift
			;;
		--base)
			BASE="${2:?--base needs a path}"
			shift
			;;
		--name)
			EXPECTED_NAME="${2:?--name needs a value}"
			shift
			;;
		--no-stage)
			STAGE=0
			;;
		*)
			echo "unknown option: $1" >&2
			exit 2
			;;
	esac
	shift
done

require() {
	command -v "$1" >/dev/null || {
		echo "required tool not found on PATH: $1" >&2
		exit 1
	}
}

require vrunner

if [ "$STAGE" -eq 1 ]; then
	echo "==> staging the release source tree"
	tools/1c-release/build-release-src.sh --out "$SRC" >/dev/null
fi

[ -d "$SRC" ] || {
	echo "no staged source tree at $SRC" >&2
	echo "run tools/1c-release/build-release-src.sh first, or drop --no-stage." >&2
	exit 1
}

basename_of_src="$(basename "$SRC")"
case "$basename_of_src" in
	*[!A-Za-z0-9_]*|[0-9]*)
		echo "the source directory must be named after the extension: letters, digits" >&2
		echo "and underscores only, and it may not start with a digit." >&2
		echo "  got:      $(basename "$SRC")" >&2
		echo "  expected: $EXPECTED_NAME (the <Name> in Configuration.xml)" >&2
		exit 1
		;;
esac

CONFIG_XML="$SRC/Configuration.xml"
[ -f "$CONFIG_XML" ] || {
	echo "not an extension source tree, no Configuration.xml in $SRC" >&2
	exit 1
}

# The name the compiler will use has to be the name inside the extension, or the
# compiled artefact and the `--ext` argument of `vrunner test yaxunit` disagree.
if ! grep -q "<Name>$basename_of_src</Name>" "$CONFIG_XML"; then
	echo "$CONFIG_XML does not declare the name $basename_of_src" >&2
	echo "the directory name and the extension name must match; see the note above." >&2
	exit 1
fi

if [ ! -f "$BASE/1Cv8.1CD" ]; then
	echo "==> creating the disposable infobase $BASE"
	rm -rf "$BASE"
	vrunner infobase init --ibcmd --ibconnection "/F$REPO_ROOT/$BASE" --v8version 8.3
fi

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"

echo "==> compiling $SRC -> $OUT"
vrunner cfe compile --src "$SRC" --ibcmd --v8version 8.3 "$OUT"

[ -f "$OUT" ] || {
	echo "the compiler reported success but $OUT does not exist" >&2
	exit 1
}

# A .cfe is a 1C container, not a ZIP: it begins with the four bytes FF FF FF 7F.
# A truncated container would otherwise only be discovered when a consumer tried to
# load it.
magic="$(head -c 4 "$OUT" | od -An -tx1 | tr -d ' \n')"
if [ "$magic" != "ffffff7f" ]; then
	echo "$OUT is not a 1C extension container (first bytes: $magic)" >&2
	exit 1
fi

echo
echo "$OUT"
ls -l "$OUT"

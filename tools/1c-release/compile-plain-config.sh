#!/usr/bin/env bash
#
# Build the merge carrier: a .cf whose objects can be compared with a target
# configuration object by object.
#
# Why a second artefact next to the extension: the extension installs the modules
# alongside a configuration, while a configuration that already exists wants them
# merged into it. The configurator's compare-and-merge tool reads configurations
# and extension files, so the objects are produced a second time as a plain
# configuration - without the extension's name prefix - and compiled into a .cf.
# Opening that .cf as the comparison source shows three common modules and one
# common template, each of which can be taken or left.
#
# Usage:
#   tools/1c-release/compile-plain-config.sh [--src build/plain/QRCode] [--out build/out/QRCode.cf]
#
# The sources are generated from the extension sources (tools/1c-release/build-plain-src.py),
# so the two cannot drift.
#
# Compiling with `--ibcmd` does not need a 1C licence, but it does need vrunner and
# the platform, which live in the devcontainer.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

SRC="build/plain/QRCode"
OUT="build/out/QRCode.cf"
EXPECTED_NAME="QRCode"
STAGE=1

while [ $# -gt 0 ]; do
	case "$1" in
		--src)
			SRC="${2:?--src needs a directory}"
			shift 2
			;;
		--out)
			OUT="${2:?--out needs a file}"
			shift 2
			;;
		--no-stage)
			STAGE=0
			shift
			;;
		-h | --help)
			sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
			exit 0
			;;
		*)
			echo "unknown argument: $1" >&2
			exit 2
			;;
	esac
done

if [ "$STAGE" -eq 1 ]; then
	echo "==> generating the plain configuration sources"
	tools/1c-release/build-plain-src.py --out "$SRC" >/dev/null
fi

[ -f "$SRC/Configuration.xml" ] || {
	echo "no Configuration.xml in $SRC" >&2
	echo "run tools/1c-release/build-plain-src.py first, or drop --no-stage." >&2
	exit 1
}

# vrunner takes the configuration name from the directory, so a mismatch has to be
# caught here rather than after a confusing load error.
basename_of_src="$(basename "$SRC")"
if [ "$basename_of_src" != "$EXPECTED_NAME" ]; then
	echo "the source directory has to be named $EXPECTED_NAME: got $basename_of_src" >&2
	exit 1
fi
grep -q "<Name>$basename_of_src</Name>" "$SRC/Configuration.xml" || {
	echo "$SRC/Configuration.xml does not declare the name $basename_of_src" >&2
	exit 1
}

command -v vrunner >/dev/null || {
	echo "vrunner is not on PATH. Run this inside the dev container:" >&2
	echo "  tools/1c-release/compile-plain-config.sh" >&2
	echo "(VS Code: Dev Containers: Reopen in Container)" >&2
	exit 1
}

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"

echo "==> compiling $SRC -> $OUT"
vrunner cf compile --src "$SRC" --ibcmd --v8version 8.3 "$OUT"

[ -f "$OUT" ] || {
	echo "the compiler reported success but $OUT does not exist" >&2
	exit 1
}

echo
echo "merge carrier: $OUT ($(stat -c%s "$OUT" 2>/dev/null || stat -f%z "$OUT") bytes)"
echo "open it in the configurator as the configuration to compare with yours:"
echo "  Конфигурация -> Сравнить, объединить с конфигурацией из файла -> $OUT"

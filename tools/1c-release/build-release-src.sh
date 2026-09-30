#!/usr/bin/env bash
#
# Build the release source tree of the extension: the sources in
# src/cfe/QrGenerator plus the packed component, added as the binary body of the
# qr_QrGeneratorNative template.
#
# The archive is deliberately not a source file. It is a build product that
# changes with every code change, and committing it would mean committing a binary
# that nothing can review. So the template ships as an XML skeleton in src/, and
# this script injects the archive into a staged copy under build/.
#
# Usage:
#   tools/1c-release/build-release-src.sh
#   tools/1c-release/build-release-src.sh --zip /path/to/QrGeneratorNative.zip
#   tools/1c-release/build-release-src.sh --out build/other/QrGenerator
#
# The directory name matters. `vrunner cfe compile` derives the extension name from
# the basename of `--src` when it is not told otherwise, and validates it against
# `^[A-Za-zА-Яа-яЁё_][A-Za-zА-Яа-яЁё0-9_]*$` — letters, digits and underscores
# only. A hyphen is rejected outright, which is why the default is
# `build/release/QrGenerator` rather than something like `build/release-src`.
# Keeping the basename equal to the extension's own <Name> also keeps the compiled
# extension and the `--ext` argument of `vrunner test yaxunit` in agreement.
#
# The staged tree is what `vrunner cfe compile --src` consumes:
#
#   vrunner cfe compile --src build/release/QrGenerator --ibcmd --v8version 8.3 \
#       build/out/QrGenerator-v0.1.0.cfe
#
# Compiling with `--ibcmd` does not need a 1C licence, so this whole path can be
# verified without one. Running the result does need a licence.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

SOURCE="src/cfe/QrGenerator"
TEMPLATE_NAME="qr_QrGeneratorNative"
ZIP="build/dist/QrGeneratorNative.zip"
OUT="build/release/QrGenerator"

while [ $# -gt 0 ]; do
	case "$1" in
		--zip)
			ZIP="${2:?--zip needs a path}"
			shift
			;;
		--out)
			OUT="${2:?--out needs a path}"
			shift
			;;
		*)
			echo "unknown option: $1" >&2
			exit 2
			;;
	esac
	shift
done

[ -d "$SOURCE" ] || {
	echo "missing extension sources: $SOURCE" >&2
	exit 1
}

[ -f "$ZIP" ] || {
	echo "missing the packed component: $ZIP" >&2
	echo "build it first:" >&2
	echo "  tools/native/pack-addin.sh" >&2
	exit 1
}

# The template has to exist in the source tree, otherwise the injected archive
# would sit in a directory no metadata object points at, and the extension would
# compile without the component in it.
TEMPLATE_XML="$SOURCE/CommonTemplates/$TEMPLATE_NAME.xml"
[ -f "$TEMPLATE_XML" ] || {
	echo "missing the template metadata: $TEMPLATE_XML" >&2
	exit 1
}

echo "==> staging $OUT from $SOURCE"
rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$SOURCE/." "$OUT/"

# A BinaryData template stores its content verbatim in Ext/Template.bin. The
# platform hands that byte stream to ПодключитьВнешнююКомпоненту as the archive.
TEMPLATE_BIN="$OUT/CommonTemplates/$TEMPLATE_NAME/Ext/Template.bin"
mkdir -p "$(dirname "$TEMPLATE_BIN")"
cp "$ZIP" "$TEMPLATE_BIN"

# Confirm the archive survived the copy, and that it is the archive we expect
# rather than an empty file left behind by a failed build.
if ! unzip -tq "$TEMPLATE_BIN" >/dev/null; then
	echo "the injected template is not a readable archive: $TEMPLATE_BIN" >&2
	exit 1
fi

echo "==> staged tree"
find "$OUT" -type f | sort

echo
echo "template body: $(wc -c < "$TEMPLATE_BIN") bytes"

# The compile command writes into build/out, and the staged tree is not there yet.
mkdir -p build/out

echo "compile it with:"
echo "  vrunner cfe compile --src $OUT --ibcmd --v8version 8.3 build/out/QrGenerator-v0.1.0.cfe"

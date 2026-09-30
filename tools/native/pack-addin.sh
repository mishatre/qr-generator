#!/usr/bin/env bash
#
# Pack the built component into the ZIP archive that a 1C template carries.
#
# The archive holds manifest.xml plus the two libraries it names. The platform
# reads the manifest, picks the entry matching the operating system and
# architecture, and loads that file out of the archive — so the file names must
# match the manifest exactly, which this script verifies rather than assumes.
#
# Usage:
#   tools/native/pack-addin.sh
#
# The outputs are build/dist/QrGeneratorNative.zip and a staging directory beside
# it. Both are gitignored: the ZIP is a release asset, never a source file. The
# extension source tree in src/cfe/QrGenerator ships without it; the archive is
# injected into a staged copy by tools/1c-release/build-release-src.sh.
#
# Requires both targets to have been built:
#   tools/native/build-addin.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

MANIFEST="tools/native/manifest.xml"
STAGING="build/native/bundle"
OUTPUT="build/dist/QrGeneratorNative.zip"

# Where cargo may have put each artefact, most specific first. The bare
# `release/` path is what a build without `--target` produces on the host, which
# is the only one a machine without the container can make.
LINUX_SOURCES=(
	"rust/target/x86_64-unknown-linux-gnu/release/libqr_generator_addin.so"
	"rust/target/release/libqr_generator_addin.so"
)
WINDOWS_SOURCES=(
	"rust/target/x86_64-pc-windows-gnu/release/qr_generator_addin.dll"
)

for tool in zip unzip; do
	command -v "$tool" >/dev/null || {
		echo "required tool not found on PATH: $tool" >&2
		exit 1
	}
done

# Collect the file names the manifest promises. These are the names 1C will look
# for inside the archive, so the packer renames the artefacts to match.
manifest_paths() {
	grep -o 'path="[^"]*"' "$MANIFEST" | sed 's/^path="//; s/"$//'
}

find_artifact() {
	local description="$1"
	shift

	for candidate in "$@"; do
		if [ -f "$candidate" ]; then
			printf '%s' "$candidate"
			return 0
		fi
	done

	echo "missing the $description. Build it first:" >&2
	echo "  tools/native/build-addin.sh" >&2
	echo "searched:" >&2
	for candidate in "$@"; do
		echo "  $candidate" >&2
	done
	return 1
}

linux_artifact="$(find_artifact 'Linux component' "${LINUX_SOURCES[@]}")"
windows_artifact="$(find_artifact 'Windows component' "${WINDOWS_SOURCES[@]}")"

echo "==> staging $STAGING"
rm -rf "$STAGING"
mkdir -p "$STAGING"

cp "$MANIFEST" "$STAGING/manifest.xml"
cp "$linux_artifact" "$STAGING/QrGenerator_linux_x86_64.so"
cp "$windows_artifact" "$STAGING/QrGenerator_win_x86_64.dll"

echo "==> packing $OUTPUT"
mkdir -p "$(dirname "$OUTPUT")"
rm -f "$OUTPUT"

# -X keeps platform-specific extra attributes out, so the archive is byte-stable
# for the same inputs.
(
	cd "$STAGING"
	zip -X -q -9 "$REPO_ROOT/$OUTPUT" manifest.xml QrGenerator_linux_x86_64.so QrGenerator_win_x86_64.dll
)

echo "==> verifying the archive against the manifest"
if ! unzip -tq "$OUTPUT" >/dev/null; then
	echo "the archive is not readable: $OUTPUT" >&2
	exit 1
fi

listing="$(unzip -Z1 "$OUTPUT")"
missing=0

while IFS= read -r path; do
	if ! printf '%s\n' "$listing" | grep -qx -- "$path"; then
		echo "manifest lists $path but the archive does not contain it" >&2
		missing=1
	fi
done < <(manifest_paths)

if [ "$missing" -ne 0 ]; then
	echo "the manifest and the archive disagree; 1C would fail to load the component" >&2
	exit 1
fi

echo
echo "$OUTPUT"
unzip -l "$OUTPUT"

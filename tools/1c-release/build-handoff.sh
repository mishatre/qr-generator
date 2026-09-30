#!/usr/bin/env bash
#
# Assemble the set that goes to the 1C developer: the component archive, the
# compiled extension, the module sources and the consumer guide.
#
# Why the module text travels next to the extension: a consumer who cannot install
# an extension, or who wants the module inside their own configuration, needs the
# code as plain files - unpacking an XML dump to get at it is a step nobody should
# have to take. They are the prefix-free texts from the merge carrier, so what is
# read here is exactly what a merge would put into a configuration. All three travel
# together because the wrapper calls the cache module; the example one is optional.
#
# Usage:
#   tools/1c-release/build-handoff.sh [--out <dir>]     # default: build/handoff
#
# Exits non-zero when a required artefact is missing rather than leaving a partial
# set behind: a handoff that looks complete and is not is worse than none.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

OUT="build/handoff"
ZIP="build/dist/QrGeneratorNative.zip"
CFE="build/out/QrGenerator-v0.1.0.cfe"
# The merge carrier and its module texts are a plain configuration, generated from
# the extension sources: no prefix, no extension-only properties.
PLAIN="build/plain/QRCode"
CF="build/out/QRCode.cf"
GUIDE="docs/consumer-guide.md"
MODULES=(
	"$PLAIN/CommonModules/QRCode/Ext/Module.bsl"
	"$PLAIN/CommonModules/QRCodeСерверПовтИсп/Ext/Module.bsl"
	"$PLAIN/CommonModules/QRCodeПример/Ext/Module.bsl"
)

while [ $# -gt 0 ]; do
	case "$1" in
		--out)
			OUT="${2:?--out needs a directory}"
			shift 2
			;;
		-h | --help)
			sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
			exit 0
			;;
		*)
			echo "unknown argument: $1" >&2
			exit 2
			;;
	esac
done

filesize() {
	if stat -f%z "$1" >/dev/null 2>&1; then stat -f%z "$1"; else stat -c%s "$1"; fi
}

# The compile step names the extension after its version, so the newest match wins
# and any older ones stay where they are.
CFE="$(ls -t build/out/QrGenerator-*.cfe 2>/dev/null | head -1 || true)"

problems=()
[ -f "$ZIP" ] || problems+=("the component archive is missing: run tools/native/pack-addin.sh")
[ -f "$CFE" ] || problems+=("the compiled extension is missing: run tools/1c-release/compile-extension.sh")
[ -f "$CF" ] || problems+=("the merge carrier is missing: run tools/1c-release/compile-plain-config.sh")
[ -f "$GUIDE" ] || problems+=("$GUIDE is missing: the handoff carries it as its README")
for module in "${MODULES[@]}"; do
	[ -f "$module" ] || problems+=("a module source is missing: $module")
done

if [ "${#problems[@]}" -gt 0 ]; then
	echo "cannot assemble a complete handoff:" >&2
	for problem in "${problems[@]}"; do
		echo "  - $problem" >&2
	done
	exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

cp "$ZIP" "$OUT/$(basename "$ZIP")"
cp "$CFE" "$OUT/QrGenerator.cfe"
cp "$CF" "$OUT/QRCode.cf"
cp "$GUIDE" "$OUT/README.md"

for module in "${MODULES[@]}"; do
	name="$(basename "$(dirname "$(dirname "$module")")")"
	cp "$module" "$OUT/$name.bsl"
done

# The set is made to be handed to someone else; the sources may have come out of a
# container as root with a group-only read bit.
chmod -R a+rX "$OUT"

echo "handoff: $OUT"
printf '  %-32s %9s B\n' "QrGeneratorNative.zip" "$(filesize "$OUT/QrGeneratorNative.zip")"
printf '  %-32s %9s B\n' "QrGenerator.cfe" "$(filesize "$OUT/QrGenerator.cfe")"
printf '  %-32s %9s B\n' "QRCode.cf" "$(filesize "$OUT/QRCode.cf")"
for module in "${MODULES[@]}"; do
	name="$(basename "$(dirname "$(dirname "$module")")")"
	printf '  %-32s %9s B\n' "$name.bsl" "$(filesize "$OUT/$name.bsl")"
done
printf '  %-32s %9s B\n' "README.md" "$(filesize "$OUT/README.md")"
echo "  total                            $(du -sk "$OUT" | cut -f1) KiB"

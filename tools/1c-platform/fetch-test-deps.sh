#!/usr/bin/env bash
#
# Fetch the YAxUnit extension that tests/bsl/run-tests.sh loads into the
# disposable infobase. YAxUnit is third-party; it is downloaded rather than
# vendored, and the download lands in build/ so it never enters the repository.
#
# Usage:
#   tools/1c-platform/fetch-test-deps.sh
#   tools/1c-platform/fetch-test-deps.sh --from /path/to/YAxUnit-25.12.cfe
#   YAXUNIT_VERSION=25.12 tools/1c-platform/fetch-test-deps.sh
#
# Verified 2026-09-29: the release tag has no `v` prefix, and the asset is named
# after the version. `Smoke-25.12.cfe` sits beside it and is not used here: it is a
# small self-test of the engine, not the engine.
YAXUNIT_VERSION="${YAXUNIT_VERSION:-25.12}"
YAXUNIT_URL="${YAXUNIT_URL:-https://github.com/bia-technologies/yaxunit/releases/download/${YAXUNIT_VERSION}/YAxUnit-${YAXUNIT_VERSION}.cfe}"

# The release file name contains a hyphen and `vrunner cfe load` rejects an
# extension name containing one, so the artefact is copied to a hyphen-free name
# (YAxUnit.cfe) — the same problem the sibling project documents.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

VENDOR_DIR="build/vendor/yaxunit"
TARGET="$VENDOR_DIR/YAxUnit.cfe"

FROM=""

while [ $# -gt 0 ]; do
	case "$1" in
		--from)
			FROM="${2:?--from needs a path}"
			shift
			;;
		*)
			echo "unknown option: $1" >&2
			exit 2
			;;
	esac
	shift
done

mkdir -p "$VENDOR_DIR"

if [ -n "$FROM" ]; then
	[ -f "$FROM" ] || {
		echo "no such file: $FROM" >&2
		exit 1
	}
	echo "==> copying $FROM"
	cp "$FROM" "$TARGET"
else
	echo "==> downloading YAxUnit $YAXUNIT_VERSION"
	echo "    $YAXUNIT_URL"
	if ! curl --fail --location --silent --show-error --output "$TARGET" "$YAXUNIT_URL"; then
		echo "download failed. Confirm the release asset name at" >&2
		echo "  https://github.com/bia-technologies/yaxunit/releases" >&2
		echo "then pass it with --from, or set YAXUNIT_URL." >&2
		rm -f "$TARGET"
		exit 1
	fi
fi

# A .cfe is a 1C container, not a ZIP: it begins with the four bytes FF FF FF 7F.
# Checking the magic rather than trying to unzip means an HTML error page or a
# truncated download is rejected here, instead of surfacing much later as a load
# failure inside the platform.
magic="$(head -c 4 "$TARGET" | od -An -tx1 | tr -d ' \n')"
if [ "$magic" != "ffffff7f" ]; then
	echo "not a usable 1C extension container: $TARGET (first bytes: $magic)" >&2
	exit 1
fi

echo "==> $TARGET"
ls -l "$TARGET"

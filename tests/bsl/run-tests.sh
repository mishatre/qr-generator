#!/usr/bin/env bash
#
# Build the component, build a test build of the extension, and run the YAxUnit
# suite inside 1C.
#
# The work is split into two tiers, because only the first one can run without a
# 1C licence:
#
#   1. build   — pack the component, inject it into a staged copy of the
#                extension, and compile that extension with ibcmd. ibcmd does not
#                need a licence, and compiling type-checks the BSL of both the
#                wrapper and the test suite.
#   2. run     — create a disposable infobase, load YAxUnit and the test build,
#                and execute the suite. This needs a licence, and it needs the
#                extension's Безопасный режим and Защита от опасных действий
#                flags cleared, because a component cannot be connected while
#                safe mode is on. YAxUnit's own documentation says the same.
#                Only the run itself needs the licence: loading and the flag
#                change both go through ibcmd.
#
# Two things the image does not provide, both handled below: the platform client
# needs a licence file in /var/1C/licenses (measured — ~/.1cv8/1C is not read), and
# it needs a display, which xvfb supplies when DISPLAY is unset.
#
# Usage:
#   tests/bsl/run-tests.sh --build-only     # tier 1, no licence needed
#   tests/bsl/run-tests.sh --rebuild-base   # tier 1 then tier 2
#
# Exits 3 when the platform refuses to run for want of a licence, so that a caller
# can tell "the environment is not ready" apart from "the tests failed".
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

BASE="build/ib"
# The basename of --src must be the extension name — vrunner derives it from the
# directory and accepts letters, digits and underscores only — hence
# build/test/QrGenerator. The .cfe file name is free to differ; it is only an
# artefact name. The name the platform sees is read back from the staged tree
# below, because that is what `--ext` has to match.
TEST_SRC="build/test/QrGenerator"
TEST_CFE="build/test/QrGeneratorTests.cfe"
YAXUNIT_CFE="build/vendor/yaxunit/YAxUnit.cfe"
# Extension names are matched case-sensitively: this is the name inside
# YAxUnit.cfe, which is not the name of the file that carries it.
YAXUNIT_EXTENSION_NAME="YAXUNIT"
LOG="build/test/run.log"
REPORT="build/test/reports/yaxunit.xml"
EXITCODE_FILE="build/test/yaxunit-exitcode.txt"

BUILD_ONLY=0
REBUILD_BASE=0

while [ $# -gt 0 ]; do
	case "$1" in
		--build-only)
			BUILD_ONLY=1
			;;
		--rebuild-base)
			REBUILD_BASE=1
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
require ibcmd

# The platform client needs a display. The container has xvfb and no X server, so a
# headless session goes through xvfb-run; an interactive one uses the real DISPLAY.
run_platform() {
	if [ -n "${DISPLAY:-}" ]; then
		"$@"
		return
	fi

	if command -v xvfb-run >/dev/null; then
		xvfb-run -a "$@"
		return
	fi

	echo "no DISPLAY and no xvfb-run: the platform client cannot start" >&2
	return 1
}

echo "==> packing the component"
tools/native/pack-addin.sh >/dev/null

echo "==> staging the test build in $TEST_SRC"
# The release staging script owns the template injection, so there is one place
# that knows how the archive reaches the extension.
tools/1c-release/build-release-src.sh --out "$TEST_SRC" >/dev/null

cp -R tests/bsl/CommonModules/. "$TEST_SRC/CommonModules/"

CONFIG_XML="$TEST_SRC/Configuration.xml"
grep -q '<CommonModule>qr_QRCode</CommonModule>' "$CONFIG_XML" || {
	echo "cannot find the qr_QRCode registration in $CONFIG_XML" >&2
	exit 1
}

# `vrunner cfe load` imports the compiled extension, so the platform registers it
# under the name it was compiled with — the <Name> in the staged Configuration.xml —
# not under the name of the .cfe file that carried it. `vrunner test yaxunit --ext`
# filters on the registered name, so read that name back from the staged tree
# instead of guessing it from the artefact file name.
EXTENSION_NAME="$(sed -n 's:.*<Name>\([^<]*\)</Name>.*:\1:p' "$CONFIG_XML" | head -1)"
if [ -z "$EXTENSION_NAME" ]; then
	echo "cannot read the extension name from $CONFIG_XML" >&2
	exit 1
fi

# Register the suite. A test module must live in the same extension as the code it
# calls, because a common module of one extension is not reachable from another, so
# the suite is compiled into a test build of the product extension rather than into
# an extension of its own. This build is a test artefact and is never released.
sed -i.bak 's#<CommonModule>qr_QRCode</CommonModule>#<CommonModule>qr_QRCode</CommonModule>\n\t\t\t<CommonModule>qr_QRCodesTests</CommonModule>#' "$CONFIG_XML"
rm -f "$CONFIG_XML.bak"

grep -q '<CommonModule>qr_QRCodesTests</CommonModule>' "$CONFIG_XML" || {
	echo "failed to register the test module in $CONFIG_XML" >&2
	exit 1
}

echo "==> compiling the test build"
# The compiler loads the extension into an infobase and needs one to exist, so this
# also creates build/ib on the first run. Still no licence: everything on the ibcmd
# path is licence-free.
tools/1c-release/compile-extension.sh --no-stage --src "$TEST_SRC" --out "$TEST_CFE" --base "$BASE"

echo "compiled $TEST_CFE"
echo "the extension and its test suite compile, and the component archive is inside the template"

if [ "$BUILD_ONLY" -eq 1 ]; then
	echo
	echo "stopping before the in-platform run (--build-only)"
	exit 0
fi

if [ ! -f "$YAXUNIT_CFE" ]; then
	echo "missing $YAXUNIT_CFE" >&2
	echo "fetch it with: tools/1c-platform/fetch-test-deps.sh" >&2
	echo "or copy a YAxUnit.cfe there by hand." >&2
	exit 1
fi

if [ "$REBUILD_BASE" -eq 1 ]; then
	# A clean base for the run. The compile above left the extension loaded in
	# build/ib, and the point of rebuilding is to load exactly the test build plus
	# YAxUnit into a base with nothing else in it.
	echo "==> recreating the disposable infobase $BASE"
	rm -rf "$BASE"
	vrunner infobase init --ibcmd --ibconnection "/F$REPO_ROOT/$BASE" --v8version 8.3
fi

echo "==> loading YAxUnit and the test build"
vrunner cfe load --ibconnection "/F$REPO_ROOT/$BASE" --ibcmd "$YAXUNIT_CFE"
vrunner cfe load --ibconnection "/F$REPO_ROOT/$BASE" --ibcmd "$TEST_CFE"

echo "==> clearing the extension protection flags"
# YAxUnit's documentation requires this, and it is not a formality: while an extension
# runs under Безопасный режим its code cannot load a native component at all, and the
# client cannot run the suite. `infobase init` presets these flags to false, but only
# for extensions that already exist — both of these are loaded after it, so they carry
# the platform default instead and have to be cleared explicitly.
#
# `ibcmd extension update` sets both flags directly. The obvious alternative,
# `vrunner infobase extensions set-options`, opens an external data processor in the
# thin client; the platform asks for confirmation before opening it, and on a virtual
# display that question has nobody to answer, so the run hangs there. ibcmd needs no
# licence, no display and no confirmation.
for extension in "$YAXUNIT_EXTENSION_NAME" "$EXTENSION_NAME"; do
	ibcmd extension update \
		--db-path="$REPO_ROOT/$BASE" \
		--name="$extension" \
		--safe-mode=no \
		--unsafe-action-protection=no
done

echo "==> licence"
tools/1c-platform/install-licence.sh

echo "==> running the suite"
mkdir -p "$(dirname "$LOG")" "$(dirname "$REPORT")"
rm -f "$REPORT" "$EXITCODE_FILE"

set +e
run_platform vrunner test yaxunit \
	--ibconnection "/F$REPO_ROOT/$BASE" \
	--v8version 8.3 \
	--ext "$EXTENSION_NAME" \
	--report "$REPORT" \
	--report-format jUnit \
	--exitcode "$EXITCODE_FILE" 2>&1 | tee "$LOG"
exit_code="${PIPESTATUS[0]}"
set -e

if [ "$exit_code" -eq 0 ]; then
	# A run that discovered no tests also exits 0, and that is the one green result
	# that means nothing: it is exactly what a mismatched extension name, module name
	# or missing registration looks like. Treat it as a failure.
	if [ -f "$REPORT" ]; then
		test_count="$(grep -c '<testcase' "$REPORT" 2>/dev/null || true)"
	else
		test_count=""
	fi

	case "$test_count" in
		'' | 0)
			echo >&2
			echo "the run reported success but $REPORT holds no test cases" >&2
			echo "the suite was not discovered. Check that --ext matches the extension as it" >&2
			echo "was loaded, and that the suite module is registered in the staged tree:" >&2
			echo "  --ext        $EXTENSION_NAME" >&2
			echo "  test modules $(ls -1 tests/bsl/CommonModules/*.xml 2>/dev/null | tr '\n' ' ')" >&2
			echo "  staged tree  $TEST_SRC/Configuration.xml" >&2
			exit 1
			;;
	esac

	echo
	echo "suite passed: $test_count test cases, report in $REPORT"
	exit 0
fi

# A missing licence is an environment problem, not a test failure. It is worth
# detecting explicitly: the platform reports it in prose, and the message differs
# between a missing licence, an expired one and a rejected developer licence.
if grep -qiE "лиценз|licen" "$LOG"; then
	echo
	echo "the platform refused to run for want of a 1C licence; see $LOG" >&2
	echo "activate one interactively, once:" >&2
	echo "  tools/1c-platform/open-infobase.sh client ib" >&2
	echo "the licence is stored in /root/.1cv8, which the dev container keeps in a" >&2
	echo "named volume so it survives a rebuild." >&2
	exit 3
fi

echo
echo "the suite failed; see $LOG" >&2
exit "$exit_code"

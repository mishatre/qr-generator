#!/usr/bin/env bash
#
# Open one of the disposable infobases in the 1C client or designer.
#
# The 1C platform has no command-line licence activation, so the first run has to
# be interactive: start the client, follow the licence prompt, and let the platform
# store the licence under ~/.1cv8/1C/.  After that, headless `1cv8`-based commands
# (`vrunner validate syntax-check`, `vrunner test ...`) work.
#
# Usage:
#   open-infobase.sh [client|designer|launcher] [ib|standalone|<path>]
#
# Defaults to `client ib`, which opens the extension-mode base.
#
set -euo pipefail

PLATFORM_DIR="${PLATFORM_DIR:-/opt/1cv8/current}"

MODE="${1:-client}"
TARGET="${2:-ib}"

case "$TARGET" in
    ib | extension)
        INFOBASE=/workspace/build/ib
        ;;
    standalone | ib-standalone)
        INFOBASE=/workspace/build/ib-standalone
        ;;
    /*)
        INFOBASE="$TARGET"
        ;;
    *)
        echo "Unknown infobase '$TARGET'. Use ib, standalone or an absolute path." >&2
        exit 2
        ;;
esac

if [ ! -d "$INFOBASE" ]; then
    echo "Infobase directory not found: $INFOBASE" >&2
    exit 1
fi

echo "Infobase: $INFOBASE"
echo "If the platform asks for a licence, activate it from the dialog; it is stored under ~/.1cv8/1C/."

case "$MODE" in
    client)
        exec "$PLATFORM_DIR/1cv8c" "/F$INFOBASE"
        ;;
    designer)
        exec "$PLATFORM_DIR/1cv8" DESIGNER "/F$INFOBASE"
        ;;
    launcher)
        exec "$PLATFORM_DIR/1cv8"
        ;;
    *)
        echo "Unknown mode '$MODE'. Use client, designer or launcher." >&2
        exit 2
        ;;
esac

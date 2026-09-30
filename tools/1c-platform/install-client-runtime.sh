#!/usr/bin/env bash
#
# Make the 1C:Enterprise client binaries loadable on Ubuntu 24.04.
#
# `1cv8` and `1cv8c` fail to start with:
#
#     libwebkit2gtk-4.0.so.37       => not found
#     libjavascriptcoregtk-4.0.so.18 => not found
#     libsoup-2.4.so.1              => not found
#
# Ubuntu 24.04 ships only the 4.1/3.0 generations of those libraries, and the 4.0
# ABI the platform links against is not in its archive.  The packages are therefore
# taken from Ubuntu 22.04 (jammy), which still provides them, and installed with
# dpkg so that no other package in the image is touched.
#
# The platform also bundles its own libstdc++.so.6.0.28, which is older than the
# WebKit libraries require.  It is moved aside and replaced with a symlink to the
# system libstdc++, which is backwards compatible.
#
# This script only makes the binary start.  The client still needs a 1C licence:
# either a 1Cv8Licence file in ~/.1cv8/1C/ or /var/1C/licenses/, or a HASP key.
#
# Usage: install-client-runtime.sh [platform-dir]
#
set -euo pipefail

PLATFORM_DIR="${1:-/opt/1cv8/x86_64/8.3.24.1667}"
JAMMY_LIST=/etc/apt/sources.list.d/one-c-jammy.list
DEB_DIR=/tmp/one-c-jammy-debs

# Order matters: dpkg only has to see the dependencies before the dependants.
PACKAGES=(
    libicu70
    libwoff1
    libjavascriptcoregtk-4.0-18
    libsoup2.4-common
    libsoup2.4-1
    libwebkit2gtk-4.0-37
)

install_runtime_libraries() {
    if dpkg -s libwebkit2gtk-4.0-37 >/dev/null 2>&1; then
        echo "1C client runtime libraries are already installed"
        return
    fi

    echo "Fetching the WebKitGTK 4.0 runtime from Ubuntu 22.04"

    cat > "$JAMMY_LIST" <<'SOURCES'
deb http://archive.ubuntu.com/ubuntu jammy main universe
deb http://archive.ubuntu.com/ubuntu jammy-updates main universe
deb http://security.ubuntu.com/ubuntu jammy-security main universe
SOURCES

    local apt_options=(-o Dir::Etc::SourceList="$JAMMY_LIST" -o Dir::Etc::SourceParts=/dev/null)

    apt-get update -qq "${apt_options[@]}"

    mkdir -p "$DEB_DIR"
    (
        cd "$DEB_DIR"
        apt-get download "${apt_options[@]}" "${PACKAGES[@]}"
    )

    dpkg -i "$DEB_DIR"/*.deb

    # Leave no trace in the image's package sources or indexes.
    rm -f "$JAMMY_LIST"
    rm -rf "$DEB_DIR"
    rm -f /var/lib/apt/lists/*jammy*
}

point_platform_at_system_libstdcxx() {
    local bundled="$PLATFORM_DIR/libstdc++.so.6.0.28"

    if [ ! -e "$bundled" ]; then
        echo "Platform libstdc++ is already the system one"
        return
    fi

    echo "Replacing the platform's bundled libstdc++ with the system one"
    mv "$bundled" "$bundled.distrib"
    rm -f "$PLATFORM_DIR/libstdc++.so.6"
    ln -s /usr/lib/x86_64-linux-gnu/libstdc++.so.6 "$PLATFORM_DIR/libstdc++.so.6"
}

verify() {
    local missing
    missing="$(ldd "$PLATFORM_DIR/1cv8c" | grep 'not found' || true)"

    if [ -n "$missing" ]; then
        echo "Some libraries are still missing:" >&2
        echo "$missing" >&2
        exit 1
    fi

    echo "1C client runtime libraries are complete."
    echo "A 1C licence is still required to start the client."
}

install_runtime_libraries
point_platform_at_system_libstdcxx
verify

#!/usr/bin/env bash
#
# Build the two release libraries that the component archive carries.
#
# The configuration is size-first, and it is inherited whole from the sibling
# project (yaml-parser) rather than derived here. That project measured each
# setting against its own component; THIS component has not been measured, and the
# trade is not obviously the same — the YAML parser is dominated by libyaml and is
# called once per session, while this one rasterises a font outline per label
# character and is called once per code. T001 in docs/plan/index.md is the task
# that repeats the measurement here. Until it has run, treat the numbers below as
# the sibling project's, not as this one's.
#
#   stable, as shipped                        612 264 B   1 322 496 B   (yaml-parser)
#   + "z" + -Z build-std + optimize_for_size  359 424 B     789 504 B
#   + -Cpanic=immediate-abort                 255 240 B     690 688 B
#
# Three settings are not expressible in a profile and need nightly:
#
#   * `-Z build-std=std,panic_abort` compiles core/std/alloc from source, which is
#     the only way for the profile and the panic strategy to reach std at all: the
#     std shipped for a target cannot be reshaped. On its own it makes the Windows
#     library *bigger*, so it is never used alone.
#   * `-Z build-std-features=optimize_for_size` makes std and compiler-builtins
#     prefer size. Most of the Windows saving comes from here.
#   * `-Cpanic=immediate-abort` is the panic strategy the profile cannot name.
#     Cargo accepts `panic = "immediate-abort"` in a profile only when the manifest
#     opts in with `cargo-features = ["panic-immediate-abort"]`, and stable cargo
#     then refuses to parse the manifest at all — which would break `cargo test`
#     and `clippy` on the pinned toolchain. Passing it as a compiler flag keeps
#     those working.
#
# What the trade costs: a panic aborts without unwinding or formatting, and std's
# internals are compiled size-first. Nothing on this path panics — a failure travels
# back as an empty result plus a message — and the ABI is unchanged. The five
# exported entry points survive all three settings; `cargo test -p
# qr_generator_addin --test ffi_smoke` is the check that says so.
#
# Usage:
#   tools/native/build-addin.sh
#
# The libraries land where tools/native/pack-addin.sh looks for them.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT/rust"

if ! rustup toolchain list | grep -q '^nightly'; then
	echo "the nightly toolchain is required. Install it with:" >&2
	echo "  rustup toolchain install nightly --profile minimal --component rust-src" >&2
	exit 1
fi

if ! rustup component list --toolchain nightly --installed | grep -q '^rust-src'; then
	echo "nightly needs its rust-src component for -Z build-std:" >&2
	echo "  rustup component add rust-src --toolchain nightly" >&2
	exit 1
fi

# `win7` rather than `pc-windows-gnu` on purpose. Both rustc's std and getrandom
# take ProcessPrng from bcryptprimitives.dll as their first choice, and that
# export only exists from Windows 10 on; on Windows 7, 8 and 8.1 the library is
# present but has no such entry point, so the DLL fails to load before 1C runs a
# single line ("Точка входа в процедуру ProcessPrng не найдена"). The win7 target
# vendor switches both to RtlGenRandom, which those systems do export, and keeps
# the same ABI: one library still serves every x86_64 Windows. The target is
# tier 3, which is why the build-std path below is not optional.
TARGETS=(
	x86_64-unknown-linux-gnu
	x86_64-win7-windows-gnu
)

for target in "${TARGETS[@]}"; do
	echo "==> $target"
	# RUSTFLAGS is added to the target's own rustflags in .cargo/config.toml, which
	# is where the Windows build takes +crt-static from.
	RUSTFLAGS="-Zunstable-options -Cpanic=immediate-abort" cargo +nightly build \
		-Z unstable-options \
		--release -p qr_generator_addin --target "$target" \
		-Z build-std=std,panic_abort \
		-Z build-std-features=optimize_for_size
done

echo
echo "built:"
stat -c '%s  %n' \
	target/x86_64-unknown-linux-gnu/release/libqr_generator_addin.so \
	target/x86_64-win7-windows-gnu/release/qr_generator_addin.dll

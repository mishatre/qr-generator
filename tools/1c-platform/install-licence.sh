#!/usr/bin/env bash
#
# Install a 1C software licence file so that the platform can start.
#
# Measured on 2026-09-29 with platform 8.3.24.1667 in this container:
#
#   /var/1C/licenses/        ACCEPTED — the client starts and /CheckModules runs
#   ~/.1cv8/1C/              NOT accepted — "Не найдена лицензия…"
#
# The second path is the one the sibling project's notes recommend, so this is a
# correction rather than a repetition: for a `.lic` file issued by the 1C licence
# centre, /var/1C/licenses is the directory the platform reads. Nothing has to be
# renamed; the file is picked up under its own name.
#
# Usage:
#   tools/1c-platform/install-licence.sh                  # every *.lic in the repo root
#   tools/1c-platform/install-licence.sh path/to/x.lic
#
# The licence is a secret tied to an organisation. It is gitignored (see *.lic in
# .gitignore) and must never be committed. For the same reason this script only
# copies: it never prints the file's contents.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

DESTINATION="/var/1C/licenses"

if [ "$#" -gt 0 ]; then
	sources=("$@")
else
	# `nullglob` so a repository with no licence at all is not an error: the suite
	# reports the missing licence itself, with a better message than this script can.
	shopt -s nullglob
	sources=(./*.lic)
	shopt -u nullglob
fi

if [ "${#sources[@]}" -eq 0 ]; then
	echo "no .lic file found; nothing to install." >&2
	echo "put the licence file in the repository root, or pass its path." >&2
	exit 0
fi

mkdir -p "$DESTINATION"

for source in "${sources[@]}"; do
	[ -f "$source" ] || {
		echo "not a file: $source" >&2
		exit 1
	}

	name="$(basename "$source")"
	install -m 0644 "$source" "$DESTINATION/$name"
	echo "installed $name -> $DESTINATION/"
done

echo "the platform reads $DESTINATION; the file name does not matter"

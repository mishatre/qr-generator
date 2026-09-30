# Bundled label font

`IBMPlexSans-Bold.ttf` is the typeface the component draws the label in. It is
embedded in the library with `include_bytes!`, so the component carries its own
font and never reads one from the file system of the machine running 1C.

| | |
|---|---|
| File | `IBMPlexSans-Bold.ttf` |
| Size | 200 872 bytes |
| SHA-256 | `9e6c74a889a700d707613d24548fe4ffa6bc59559a0689d2cf9e133bdcdafb2f` |
| Source | <https://github.com/IBM/plex>, `packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Bold.ttf` |
| Licence | SIL Open Font License 1.1, `OFL.txt` in this directory |

Why this font: it is a static (non-variable) TrueType file, it is licensed for
redistribution including inside a compiled binary, it is a bold face — which is
what makes a four-character label legible at a glance — and it covers Latin and
Cyrillic, which a 1C label needs.

The Reserved Font Name in the licence forbids distributing a *renamed* version.
The file is embedded and shipped unmodified, so the name is untouched; do not
subset or rename it without reading `OFL.txt` first.

Replacing the font is a one-line change: point `include_bytes!` in
`src/font.rs` at another file in this directory. Check the checksum above first if
you think the file may have been altered — nothing in the build verifies it.

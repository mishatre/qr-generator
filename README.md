# qr-generator

A 1C:Enterprise Native API component that renders a QR code with a short label drawn in
the middle, plus the extension that wraps it. The component returns the image as PNG
binary data: nothing is written to disk, no service is called.

The label is the visual check code — the last four characters of the record — so a printed
code can be recognised at a glance. That is why error correction defaults to `H`: the plate
covers modules, and the redundancy is what recovers them.

Built for 1C 8.3.24 on Windows 7 and later (x86_64) and on Linux x86_64. One Windows
library serves every version: it is built for the `win7` target vendor, because the default
one imports `ProcessPrng`, which Windows 7, 8 and 8.1 do not export.

## Using it

Install the extension `QrGenerator.cfe`:

1. Configurator → «Расширения конфигурации» → «Добавить из файла» → `QrGenerator.cfe`.
2. Clear **«Безопасный режим»** and **«Защита от опасных действий»** in its properties —
   the platform refuses to load a component while safe mode is on.
3. Check that it is up: `qr_QRCode.ВерсияКомпоненты()` returns the version string.

```bsl
Запись = "2|2000020302|0085F0226|01022031|5RCD6VPEAX4R";
Изображение = qr_QRCode.СоздатьQRКод(Запись, qr_QRCode.ВизуальныйКод(Запись));
Картинка = Новый Картинка(Изображение);
```

| Public function (server only) | Purpose |
| --- | --- |
| `СоздатьQRКод(Данные, Текст = "", Настройки = Неопределено)` | PNG as binary data; raises on empty data or data too large |
| `НастройкиПоУмолчанию(Язык = "ru")` | Settings structure, Russian or English property names |
| `ВизуальныйКод(Запись)` | The four characters printed on the label |
| `ПроверочныйКод(Основа)`, `КодВерен(Запись)` | Compute / verify the code at the end of a record |
| `ИдентификаторЭкземпляра()` | Eight-character identifier for a new record |
| `ВерсияКомпоненты()`, `ПолноеИмяОбъектаКомпоненты()`, `ПодключитьКомпоненту(...)` | Diagnostics and manual connection |

Settings, measured limits and the failure modes are in the
[consumer guide](docs/consumer-guide.md) — in Russian, because that is what ships to the
1C developer. Merging into a main configuration uses `QRCode.cf` (the same objects without
the `qr_` prefix that only an extension needs); the guide covers it.

## Developing

Prerequisites: Rust 1.92 (pinned in `rust-toolchain.toml`) and a 1C platform installation.
Release builds also need the devcontainer in `.devcontainer/` for the mingw and GNU linkers,
and a nightly toolchain with `rust-src` for the size-first `-Z build-std` build. A macOS
host can run the tests but cannot produce a shippable library.

```sh
# Unit and generation tests, no 1C involved. The generation tests decode the
# rendered PNG with a real QR decoder.
cd rust && cargo test --workspace

# Both release libraries, then the archive the 1C template carries.
# The Windows one goes to the win7 target vendor on purpose — see the note above.
tools/native/build-addin.sh          # devcontainer only: nightly -Z build-std
tools/native/pack-addin.sh           # -> build/dist/QrGeneratorNative.zip

# Extension and merge carrier (no 1C licence needed, compiled with ibcmd).
tools/1c-release/compile-extension.sh
tools/1c-release/compile-plain-config.sh

# 1C test suite. Tier 1 compiles the extension and type-checks the BSL;
# tier 2 adds YAxUnit and needs a licence (exit code 3 = licence missing).
tests/bsl/run-tests.sh --build-only
tests/bsl/run-tests.sh --rebuild-base

# Visual check: 14 sample PNGs plus a page showing transparency.
tools/native/render-examples.sh      # -> build/examples/

# The set handed to a consumer: archive, .cfe, .cf, module texts, guide.
tools/1c-release/build-handoff.sh    # -> build/handoff/
```

In the devcontainer, keep `CARGO_TARGET_DIR` outside the mounted tree and restore
ownership of anything written into `build/` (`chmod -R a+rwX build`) — the container runs
as root.

## Layout

| Path | Contents |
| --- | --- |
| `rust/crates/addin` | The component: QR matrix, label rasterisation, PNG encoding, the Native API surface |
| `rust/vendor` | The vendored `ones_addin` ABI runtime and its derive macro |
| `src/cfe/QrGenerator` | Extension sources: `qr_QRCode`, `qr_QRCodeСерверПовтИсп`, the binary template |
| `src/plain-config` | The prefix-free twin used to build the merge carrier |
| `tests/bsl` | YAxUnit suite and the two-tier runner |
| `tools` | Build, packing and release scripts |
| `docs/plan` | Design record, open questions and evidence; `docs/workflow.md` is the working agreement |

## Licence

MIT, see [LICENSE](LICENSE). Two bundled items keep their own terms: the label font is IBM
Plex Sans under the SIL Open Font License (see `rust/crates/addin/assets/README.md`), and
the reference modules in `docs/examples/` are 1C's own, CC BY 4.0, with their headers
retained.

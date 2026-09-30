//! A 1C:Enterprise Native API component that renders a QR code with a label in
//! the middle of it, and returns the picture as binary data.
//!
//! ## Why a component at all
//!
//! 1C has no drawing API and no QR encoder. The platform can put a picture on a
//! form, print it in a layout and store it in a value storage, but producing one
//! means either an external service — a network hop and something else to deploy —
//! or a BSL implementation of both a QR encoder and a font renderer, which is slow
//! and hard to keep correct. A Native API component is a first-class platform
//! object that the configuration carries inside a binary template, so there is
//! nothing to install beside the extension.
//!
//! ## Why the label is the interesting part
//!
//! A bar code carries a human-readable line of text underneath it; a QR code has
//! nowhere to put one, so the readable copy of the code goes in the middle of the
//! symbol. That is a real trade — the label covers modules, and the modules it
//! covers are data — and the component pays for it twice: it encodes at error
//! correction level `H` by default so that a third of the symbol can be lost, and
//! it chooses the label's layout to make the characters as large as the plate
//! allows. Both are described where they happen, in `options` and `label`.
//!
//! ## Shape of the thing
//!
//! `СоздатьQRКод` takes the payload, the label text and a JSON object of options,
//! and answers with a PNG. The picture cannot travel any other way: the platform's
//! `tVariant` mapping has no array or structure return, so binary data is what a
//! component can hand back besides a string — and a string would mean base64, a
//! third more bytes and a decode step in every caller.
//!
//! ## Layout
//!
//! * [`options`] — the options JSON the BSL wrapper passes to the component
//! * [`qr`] — the payload to module matrix, and nothing about drawing it
//! * [`label`] — which rows the text goes in, and how large the characters end up
//! * [`paint`] — compositing, rectangles, and the plate the label sits on
//! * [`render`] — the composition of all three into a PNG
//! * [`code`] — the instance identifier and the visual/check code of a record
//! * [`error`] — the failure values, which travel back as a message rather than a raised error
//! * [`addin`] — the platform-facing object and the methods 1C calls
//!
//! Everything except [`addin`] is ordinary safe Rust with no knowledge of 1C,
//! which is what lets `cargo test` cover the whole drawing path — including
//! decoding the rendered picture back with a real scanner — without a platform or a
//! licence.

mod addin;
pub mod code;
pub mod color;
mod error;
mod font;
pub mod label;
pub mod options;
pub mod paint;
pub mod qr;
pub mod render;

pub use addin::{version, QrGenerator};
pub use code::{check_code, instance_id};
pub use color::Color;
pub use error::{ErrorKind, QrError};
pub use options::Options;
pub use render::{render, render_with};

// The five functions the platform looks for in the library are exported by this
// macro: `GetClassNames`, `GetClassObject`, `DestroyObject`,
// `SetPlatformCapabilities` and `GetAttachType`. It is the single place that
// decides the ABI, so it is invoked once, here, and it names the one component the
// library carries. Invoking it twice, or naming two components, would emit
// duplicate `#[no_mangle]` symbols.
//
// Two of those answers are worth knowing, because they are what the platform uses
// to decide how to talk to the object at all: `GetAttachType` is `CanAttachAny`,
// and `SetPlatformCapabilities` returns `3`, the highest Native API interface
// level. Both are fixed in the vendored framework; `rust/vendor/README.md` records
// where the framework came from and why it, rather than `native_api_1c`, is the
// ABI this component is built on.
use ones_addin::export_addin;

export_addin!(QrGenerator);

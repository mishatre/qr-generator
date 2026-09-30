//! The options JSON the BSL wrapper passes to the component.
//!
//! The parameters are a JSON object rather than eight positional arguments,
//! because almost every one of them has a sensible default and a 1C call site
//! should not have to spell out the ones it does not care about. It also means a
//! new option can be added without changing the 1C-facing method signature.
//!
//! Three rules this module enforces, all of them measured decisions:
//!
//! * An unknown key is an error, not something ignored. A caller who writes
//!   `"lableSize"` has made a mistake that silently produces a default-looking
//!   code, and the mistake is only visible if it is rejected.
//! * A value outside its range is an error, reported with the value that arrived.
//!   Clamping `size: 8` to 64 would produce a picture the caller did not ask for.
//! * An empty string means "all defaults", which is how the wrapper says "no
//!   options" — the platform hands an absent optional parameter over as an empty
//!   value.

use serde::Deserialize;

use crate::color::Color;
use crate::error::QrError;
use crate::qr::EcLevel;

/// The smallest image the component will produce.
///
/// A code with a label needs room for both. Below this the label is a smear even
/// at the smallest label size, and the quiet zone and module grid together cannot
/// land on whole pixels.
pub const MIN_SIZE: u32 = 64;

/// The largest image the component will produce.
///
/// 4096 is four times the default and about eight megabytes of pixel buffer. The
/// limit exists so that a mistyped size cannot ask the server for a gigabyte.
pub const MAX_SIZE: u32 = 4096;

/// The largest quiet zone, in modules. The specification asks for four; ten is
/// generous and beyond that the caller is asking for a margin, not a quiet zone.
pub const MAX_QUIET_ZONE: u32 = 10;

/// The largest label, as a fraction of the image side.
///
/// This is a sanity bound, not a guarantee of scannability. Measured, the useful
/// ceiling is much lower and depends on the symbol: a label at 0.4 of the image
/// reads back on a 41 module symbol and does not on a 25 module one, because what
/// the error correction has to pay for is the *share of the symbol* the label
/// covers. `docs/plan/qr-contract.md` carries the measurement and the rule of
/// thumb; the bound here only rejects a label that leaves no code at all.
pub const MAX_LABEL_SIZE: f64 = 0.75;

/// How the code should be drawn.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    /// Side of the square image, in pixels.
    pub size: u32,

    /// How much redundancy the code carries.
    pub error_correction_level: EcLevel,

    /// Empty space around the code, in modules.
    pub quiet_zone: u32,

    /// Side of the label plate, as a fraction of the image side.
    pub label_size: f64,

    /// Padding between the label plate and the text, in pixels.
    pub label_margin: u32,

    /// Corner radius of the label plate, as a fraction of its side.
    pub label_radius: f64,

    /// Rows to lay the label out in. `0` chooses the number that makes the
    /// characters largest, which is what a caller with a short code wants.
    pub label_rows: u32,

    /// Colour of the dark modules.
    pub foreground: Color,

    /// Colour behind the code. Transparent by default, so the code can be placed
    /// on any surface.
    pub background: Color,

    /// Colour of the label text.
    pub label_color: Color,

    /// Colour of the label plate. Opaque, because the plate is what hides the
    /// modules underneath it and makes the text readable; set it to `transparent`
    /// to punch a hole instead.
    pub label_background: Color,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            size: 600,
            // High, not medium: the label covers modules, and the whole point of
            // the error correction is to pay for that damage. This is what makes
            // the component's output scannable at all.
            error_correction_level: EcLevel::H,
            quiet_zone: 0,
            // 0.3 and not 0.4, which is what the implementation this component is
            // modelled on uses. Measured with `tests/generation.rs`: on the smallest
            // symbol a realistic payload produces (25 modules, from a 15 character
            // order code) a 0.4 label leaves a hole the error correction cannot pay
            // for — `rqrr` locates the code and then fails with a data error — while
            // 0.35 reads back. 0.3 keeps a margin under that boundary for payloads
            // and settings the tests do not cover.
            label_size: 0.3,
            label_margin: 8,
            label_radius: 0.12,
            label_rows: 0,
            foreground: Color::rgb(0x11, 0x11, 0x11),
            background: Color::TRANSPARENT,
            label_color: Color::rgb(0x00, 0x00, 0x00),
            label_background: Color::rgb(0xff, 0xff, 0xff),
        }
    }
}

impl Options {
    /// Read the options the script passed.
    ///
    /// An empty string — which is what the wrapper sends when the caller passed no
    /// options at all — and `"{}"` both mean "every default".
    pub fn from_json(text: &str) -> Result<Self, QrError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }

        let options: Self = serde_json::from_str(text).map_err(|error| {
            QrError::options(format!(
                "the options are not a valid set of options: {error}"
            ))
        })?;

        options.validate()?;

        Ok(options)
    }

    /// Check the values serde cannot: the ones whose acceptable range depends on
    /// another value, and the ones that are legitimate JSON but not a code.
    fn validate(&self) -> Result<(), QrError> {
        if !(MIN_SIZE..=MAX_SIZE).contains(&self.size) {
            return Err(QrError::options(format!(
                "size must be between {MIN_SIZE} and {MAX_SIZE} pixels, but is {}",
                self.size
            )));
        }

        if self.quiet_zone > MAX_QUIET_ZONE {
            return Err(QrError::options(format!(
                "quietZone must be between 0 and {MAX_QUIET_ZONE} modules, but is {}",
                self.quiet_zone
            )));
        }

        if !(0.05..=MAX_LABEL_SIZE).contains(&self.label_size) {
            return Err(QrError::options(format!(
                "labelSize must be a fraction between 0.05 and {MAX_LABEL_SIZE} of the image \
                 side, but is {}",
                self.label_size
            )));
        }

        if !(0.0..=0.5).contains(&self.label_radius) {
            return Err(QrError::options(format!(
                "labelRadius must be a fraction between 0 and 0.5 of the label side, but is {}",
                self.label_radius
            )));
        }

        if self.label_rows > 4 {
            return Err(QrError::options(format!(
                "labelRows must be between 0 (choose automatically) and 4, but is {}",
                self.label_rows
            )));
        }

        // The margin is inside the plate, so it has to leave room for the text on
        // both sides. Half the plate side is already unusable; this catches the
        // configuration that leaves literally nothing.
        let plate_side = self.size as f64 * self.label_size;
        if f64::from(self.label_margin) * 2.0 >= plate_side {
            return Err(QrError::options(format!(
                "labelMargin {} leaves no room for the text in a {plate_side:.0} pixel label",
                self.label_margin
            )));
        }

        if self.foreground.is_transparent() {
            return Err(QrError::options(
                "foreground cannot be transparent: the code would be invisible",
            ));
        }

        Ok(())
    }
}

/// The options that make a test or a caller explicitly ask for "the same as
/// before", for tests that need a named value rather than a literal.
impl Options {
    /// Options with the label laid out in exactly `rows` rows.
    pub fn with_label_rows(mut self, rows: u32) -> Self {
        self.label_rows = rows;
        self
    }

    /// Options with a given image side.
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Options;
    use crate::error::ErrorKind;
    use crate::qr::EcLevel;

    #[test]
    fn no_options_means_the_defaults() {
        assert_eq!(Options::from_json(""), Ok(Options::default()));
        assert_eq!(Options::from_json("   "), Ok(Options::default()));
        assert_eq!(Options::from_json("{}"), Ok(Options::default()));
    }

    #[test]
    fn the_defaults_are_the_ones_the_documentation_promises() {
        let options = Options::default();

        assert_eq!(options.size, 600);
        assert_eq!(options.error_correction_level, EcLevel::H);
        assert_eq!(options.quiet_zone, 0);
        assert_eq!(options.label_size, 0.3);
        assert!(options.background.is_transparent());
        assert!(!options.label_background.is_transparent());
    }

    #[test]
    fn a_key_that_is_not_an_option_is_an_error() {
        let error = Options::from_json(r#"{"lableSize": 0.3}"#)
            .expect_err("a misspelled key must not be ignored");

        assert_eq!(error.kind, ErrorKind::Options);
        assert!(
            error.message.contains("lableSize"),
            "the message must name the key, but reads: {}",
            error.message
        );
    }

    #[test]
    fn a_value_outside_its_range_is_an_error_that_names_it() {
        let error = Options::from_json(r#"{"size": 8}"#).expect_err("8 pixels is not a code");

        assert_eq!(error.kind, ErrorKind::Options);
        assert!(error.message.contains("size"), "reads: {}", error.message);
        assert!(error.message.contains('8'), "reads: {}", error.message);
    }

    #[test]
    fn a_label_margin_that_swallows_the_label_is_an_error() {
        let error = Options::from_json(r#"{"size": 100, "labelSize": 0.2, "labelMargin": 12}"#)
            .expect_err("the plate is 20 pixels and the margin is 12 on each side");

        assert_eq!(error.kind, ErrorKind::Options);
        assert!(
            error.message.contains("labelMargin"),
            "reads: {}",
            error.message
        );
    }

    #[test]
    fn an_invisible_code_is_refused() {
        let error = Options::from_json(r#"{"foreground": "transparent"}"#)
            .expect_err("a transparent foreground cannot be seen");

        assert_eq!(error.kind, ErrorKind::Options);
    }

    #[test]
    fn json_that_is_not_an_object_is_an_error_rather_than_a_default() {
        assert!(Options::from_json("[").is_err());
        assert!(Options::from_json("42").is_err());
    }

    #[test]
    fn options_that_are_valid_are_read_through() {
        let options = Options::from_json(
            r##"{"size": 1024, "errorCorrectionLevel": "Q", "labelColor": "#336699"}"##,
        )
        .expect("valid options");

        assert_eq!(options.size, 1024);
        assert_eq!(options.error_correction_level, EcLevel::Q);
        assert_eq!(
            options.label_color,
            crate::color::Color::rgb(0x33, 0x66, 0x99)
        );
    }
}

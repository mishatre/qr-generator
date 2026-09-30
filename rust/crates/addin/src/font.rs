//! The label font.
//!
//! The font is compiled into the library rather than read from the machine
//! running 1C. A server has no fonts installed, a `Сервер` module cannot rely on a
//! file path, and a component that silently falls back to "no label" when the font
//! is missing would lose the one thing this project is for. See
//! `assets/README.md` for the provenance and the licence of the file.

use ab_glyph::FontRef;

use crate::error::QrError;

/// The bundled font: IBM Plex Sans Bold, SIL Open Font License 1.1.
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/IBMPlexSans-Bold.ttf");

/// Parse the bundled font.
///
/// Parsing reads the font tables, not the glyphs — rasterising happens per
/// character, lazily, as the label is drawn — so calling this once per render is
/// the intended use.
///
/// The failure is reported rather than panicked on. A panic in a release build of
/// this library aborts the process, which inside 1C would take the whole session
/// down with no explanation; a `QrError` reaches the caller as a message.
pub fn font() -> Result<FontRef<'static>, QrError> {
    FontRef::try_from_slice(FONT_BYTES).map_err(|error| {
        QrError::internal(format!(
            "the bundled label font cannot be read ({error}); the library was built with a \
             damaged assets/IBMPlexSans-Bold.ttf"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::font;
    use ab_glyph::{Font, ScaleFont};

    #[test]
    fn the_bundled_font_parses_and_carries_the_characters_a_label_needs() {
        let font = font().expect("the bundled font");

        // Digits, Latin and Cyrillic, because a 1C label is as likely to be a
        // Cyrillic word as a four-digit code.
        for character in "0123456789ABZzАБЯЁ".chars() {
            let id = font.glyph_id(character);

            assert_ne!(
                id.0, 0,
                "the bundled font has no glyph for {character:?}; the label would render \
                 a blank or a box"
            );
        }
    }

    #[test]
    fn the_advance_widths_are_usable_metrics() {
        // Guards the two facts the layout arithmetic depends on: an advance is
        // positive, and a digit's ink is shorter than the em box.
        let font = font().expect("the bundled font");
        let scaled = font.as_scaled(100.0);
        let digit = font.glyph_id('8');

        assert!(scaled.h_advance(digit) > 0.0);
        let ink = font
            .outline_glyph(digit.with_scale_and_position(100.0, ab_glyph::point(0.0, 0.0)))
            .expect("a digit has an outline")
            .px_bounds();

        assert!(ink.height() > 0.0 && ink.height() < 100.0, "{ink:?}");
    }
}

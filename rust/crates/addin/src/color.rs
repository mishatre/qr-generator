//! A colour that arrived as a string in the options JSON.
//!
//! Two things separate this from `image::Rgba<u8>`. It deserialises — the options
//! are read by `serde`, and a colour there is a string — and it distinguishes
//! "transparent" from "black with no alpha", because both of them are the value
//! `[0, 0, 0, 0]` and only one of them is a decision the caller made.

use image::Rgba;
use serde::Deserialize;

/// An RGBA colour, parsed from `#rgb`, `#rrggbb` or `transparent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    /// No colour at all. Used as the background of the image by default, so the
    /// code blends into whatever it is placed on.
    pub const TRANSPARENT: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };

    /// An opaque colour.
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// The pixel this colour is.
    pub const fn pixel(self) -> Rgba<u8> {
        Rgba([self.red, self.green, self.blue, self.alpha])
    }

    /// Whether this colour is fully invisible.
    pub const fn is_transparent(self) -> bool {
        self.alpha == 0
    }

    /// The same colour, with its opacity multiplied by `coverage`.
    ///
    /// This is how anti-aliasing is expressed: the rasteriser reports how much of
    /// a pixel a shape covers, and a partially covered pixel is the shape's colour
    /// at that opacity.
    pub fn with_coverage(self, coverage: f32) -> Self {
        let scaled = (f32::from(self.alpha) * coverage.clamp(0.0, 1.0)).round();

        Self {
            alpha: scaled as u8,
            ..self
        }
    }

    /// Parse `#rgb`, `#rrggbb` or `transparent`.
    ///
    /// The two hex forms are the ones a CSS author already knows. Three-digit form
    /// is expanded the way CSS expands it (`#f00` is `#ff0000`), because a caller
    /// who writes that form expects that and not `#0f0000`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();

        if trimmed.eq_ignore_ascii_case("transparent") {
            return Ok(Self::TRANSPARENT);
        }

        let Some(digits) = trimmed.strip_prefix('#') else {
            return Err(describe(trimmed));
        };

        let expanded = match digits.len() {
            3 => digits
                .chars()
                .flat_map(|digit| [digit, digit])
                .collect::<String>(),
            6 => digits.to_string(),
            _ => return Err(describe(trimmed)),
        };

        if !expanded.chars().all(|digit| digit.is_ascii_hexdigit()) {
            return Err(describe(trimmed));
        }

        // Six characters, all hex digits, so each pair is two hex digits: the
        // conversions cannot fail and the unwraps are unreachable. Written with
        // `from_str_radix` rather than a hand-rolled conversion because the error
        // path above has already made them infallible.
        let red = u8::from_str_radix(&expanded[0..2], 16).expect("two hex digits");
        let green = u8::from_str_radix(&expanded[2..4], 16).expect("two hex digits");
        let blue = u8::from_str_radix(&expanded[4..6], 16).expect("two hex digits");

        Ok(Self::rgb(red, green, blue))
    }
}

/// The message serde reports when a colour cannot be parsed.
fn describe(text: &str) -> String {
    format!("{text:?} is not a colour: use \"#rgb\", \"#rrggbb\" or \"transparent\"")
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn hex_colours_are_read_in_both_forms() {
        assert_eq!(Color::parse("#112233"), Ok(Color::rgb(0x11, 0x22, 0x33)));
        // The three-digit form expands each digit, as CSS does.
        assert_eq!(Color::parse("#f00"), Ok(Color::rgb(0xff, 0x00, 0x00)));
        assert_eq!(Color::parse("#abc"), Ok(Color::rgb(0xaa, 0xbb, 0xcc)));
        assert_eq!(Color::parse("  #000000 "), Ok(Color::rgb(0, 0, 0)));
    }

    #[test]
    fn transparent_is_a_colour_of_its_own() {
        assert!(Color::parse("transparent")
            .expect("a colour")
            .is_transparent());
        assert!(Color::parse("Transparent")
            .expect("a colour")
            .is_transparent());
    }

    #[test]
    fn anything_else_names_what_arrived() {
        let message = Color::parse("red").expect_err("a word is not a colour");

        assert!(message.contains("red"), "the message reads: {message}");
        assert!(Color::parse("#12345").is_err());
        assert!(Color::parse("#gggggg").is_err());
        assert!(Color::parse("").is_err());
    }

    #[test]
    fn coverage_scales_the_opacity_and_leaves_the_colour_alone() {
        let half = Color::rgb(0x11, 0x22, 0x33).with_coverage(0.5);

        assert_eq!(
            half,
            Color {
                red: 0x11,
                green: 0x22,
                blue: 0x33,
                alpha: 128
            }
        );
        // Opaque stays opaque when fully covered, and a colour that is already
        // invisible cannot become visible.
        assert_eq!(Color::rgb(1, 2, 3).with_coverage(1.0).alpha, 255);
        assert!(Color::TRANSPARENT.with_coverage(1.0).is_transparent());
    }
}

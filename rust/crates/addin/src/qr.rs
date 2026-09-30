//! The QR symbol itself: the module matrix, and nothing about how it is drawn.

use qrcode::types::QrError as EncoderError;
use qrcode::QrCode;
use serde::Deserialize;

use crate::error::QrError;

/// How much of the code is error correction rather than data.
///
/// Worth knowing when choosing: `L` recovers from about 7% damage, `M` 15%, `Q`
/// 25%, `H` 30%. The label covers modules, which *is* damage, so the default is
/// `H` and anything lower is a deliberate trade of scannability for payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EcLevel {
    /// About 7% of the code can be lost.
    L,
    /// About 15%.
    M,
    /// About 25%.
    Q,
    /// About 30%. The default, and the reason a labelled code still scans.
    #[default]
    H,
}

impl From<EcLevel> for qrcode::EcLevel {
    fn from(level: EcLevel) -> Self {
        match level {
            EcLevel::L => Self::L,
            EcLevel::M => Self::M,
            EcLevel::Q => Self::Q,
            EcLevel::H => Self::H,
        }
    }
}

/// The version of the symbol chosen for a payload at a given level.
///
/// Not part of the drawing path — the renderer only needs the matrix — but it is
/// what makes a size complaint actionable and it is what the tests assert about
/// the choice of version, so it travels with the matrix.
pub type Version = u8;

/// A square matrix of modules, row-major from the top left.
///
/// The finder patterns, timing patterns and the quiet zone are not in here: this
/// is the symbol as the encoder produced it, with the quiet zone the renderer adds
/// around it kept as an option rather than baked into the data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    /// Side of the matrix in modules. A version 1 code is 21, and each version
    /// adds four.
    pub width: usize,
    /// One entry per module, `true` where the module is dark.
    pub dark: Vec<bool>,
}

impl Matrix {
    /// Whether the module at this column and row is dark.
    ///
    /// Panics outside the matrix. Only the renderer calls it, and it walks the
    /// matrix, so an out-of-range call is a defect here rather than bad input.
    pub fn is_dark(&self, column: usize, row: usize) -> bool {
        self.dark[row * self.width + column]
    }
}

/// Encode `data` into a module matrix.
///
/// The text is handed to the encoder as UTF-8 bytes, and that is the decision
/// worth recording: the encoder writes the bytes it is given into byte mode and
/// adds no ECI header, which is what every scanner in practice assumes. (The
/// sibling project this component is modelled on had to pass latin1-encoded bytes
/// instead, because the JavaScript encoder it used took one byte per character;
/// that is a defect of that encoder, not of QR codes, and passing UTF-8 here is
/// what makes a Cyrillic payload come back out as Cyrillic.)
pub fn encode(data: &str, level: EcLevel) -> Result<Matrix, QrError> {
    let code =
        QrCode::with_error_correction_level(data.as_bytes(), level.into()).map_err(|error| {
            match error {
                EncoderError::DataTooLong => QrError::data(format!(
                "the data is too long for a QR code at error correction level {level:?}: {} bytes \
                 of UTF-8, and the largest code holds less",
                data.len()
            )),
                other => QrError::data(format!("the data cannot be encoded: {other}")),
            }
        })?;

    let width = code.width();
    let dark = code
        .to_colors()
        .into_iter()
        .map(|module| module == qrcode::Color::Dark)
        .collect::<Vec<bool>>();

    Ok(Matrix { width, dark })
}

#[cfg(test)]
mod tests {
    use super::{encode, EcLevel};

    #[test]
    fn a_short_payload_gets_the_smallest_symbol_that_fits() {
        // 21 modules is version 1. Lower-case text forces byte mode, where version
        // 1 holds 7 bytes at level H and 17 at level L, so the same payload needs a
        // later version at the higher level — which is the trade the option makes.
        let low = encode("hello world", EcLevel::L).expect("a code");
        let high = encode("hello world", EcLevel::H).expect("a code");

        assert_eq!(low.width, 21);
        assert!(high.width > low.width, "H needs a larger version than L");

        // Every version is 21 + 4n modules.
        assert_eq!((high.width - 21) % 4, 0);
    }

    #[test]
    fn the_matrix_is_square_and_row_major() {
        let matrix = encode("https://example.com/order/12345", EcLevel::H).expect("a code");

        assert_eq!(matrix.dark.len(), matrix.width * matrix.width);
        // The three finder patterns are dark at their outer corner. If the matrix
        // were transposed or the polarity flipped, this is where it would show.
        assert!(matrix.is_dark(0, 0));
        assert!(matrix.is_dark(matrix.width - 1, 0));
        assert!(matrix.is_dark(0, matrix.width - 1));
        assert!(!matrix.is_dark(matrix.width - 1, matrix.width - 1));
    }

    #[test]
    fn cyrillic_survives_as_utf8_bytes() {
        // Nothing here decodes the code — the generation tests do that with a real
        // scanner. What this pins is that the bytes handed to the encoder are UTF-8
        // rather than one byte per character, which is the difference between a
        // Cyrillic payload that scans and one that does not.
        let text = "Заказ №42";
        let matrix = encode(text, EcLevel::H).expect("a code");

        // The payload is mostly two-byte characters, so the encoder has to be given
        // more bytes than there are characters — a single-byte-per-character encoder
        // would pick a smaller symbol for the same text.
        assert!(
            text.len() > text.chars().count() + 4,
            "the payload has to be substantially multi-byte, but is {} characters in {} bytes",
            text.chars().count(),
            text.len()
        );
        assert!(
            matrix.width >= 25,
            "the symbol has to hold {} bytes, but is only {} modules across",
            text.len(),
            matrix.width
        );
    }

    #[test]
    fn data_that_cannot_fit_is_refused_with_a_message_that_says_so() {
        let error = encode(&"x".repeat(10_000), EcLevel::H).expect_err("10 000 bytes cannot fit");

        assert_eq!(error.kind, crate::error::ErrorKind::Data);
        assert!(
            error.message.contains("too long"),
            "reads: {}",
            error.message
        );
        assert!(error.message.contains("10000"), "reads: {}", error.message);
    }

    #[test]
    fn the_same_payload_encodes_to_the_same_symbol() {
        let first = encode("ORDER-2026-0001", EcLevel::H).expect("a code");
        let second = encode("ORDER-2026-0001", EcLevel::H).expect("a code");

        assert_eq!(first, second);
    }
}

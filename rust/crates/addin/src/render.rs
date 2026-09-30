//! Composing the picture: the quiet zone, the modules, the label, and the PNG.
//!
//! The result is a PNG in memory, which is what the component returns to 1C as
//! binary data. Nothing is written to a file and nothing is read from one: a
//! `Сервер` module in 1C may run where no writable directory exists, and the
//! caller can put the bytes wherever it wants them.
//!
//! Two drawing decisions that the reference implementation makes and this one
//! keeps, both for scannability:
//!
//! * The label sits on a plate that *erases* the modules underneath it rather than
//!   blending over them. Blending leaves the modules visible through the text at
//!   partial coverage, which reads as noise around every character.
//! * Modules are whole pixels, drawn with no anti-aliasing, and their size is
//!   floored so the grid always fits. A module rendered as a grey rectangle with
//!   soft edges is a module a scanner has to guess at.

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use crate::error::QrError;
use crate::label;
use crate::options::Options;
use crate::paint;
use crate::{font, qr};

/// Render a QR code with a label in the middle, from the options JSON the script
/// passed.
pub fn render(data: &str, text: &str, options_json: &str) -> Result<Vec<u8>, QrError> {
    let options = Options::from_json(options_json)?;

    render_with(data, text, &options)
}

/// Render with options that have already been read.
///
/// This is the entry point the tests and the example use, and the one the platform
/// method ends up in: reading the options is the only difference between the two.
pub fn render_with(data: &str, text: &str, options: &Options) -> Result<Vec<u8>, QrError> {
    let matrix = qr::encode(data, options.error_correction_level)?;
    let mut image = RgbaImage::from_pixel(options.size, options.size, options.background.pixel());

    draw_modules(&mut image, &matrix, options)?;

    if !text.trim().is_empty() {
        draw_label(&mut image, text, options)?;
    }

    encode_png(&image)
}

/// Paint the modules onto the background.
///
/// The quiet zone is drawn as the *absence* of modules inside an image that is
/// larger than the symbol, which is why the origin is offset rather than the image
/// padded: the background colour already fills the margin.
fn draw_modules(
    image: &mut RgbaImage,
    matrix: &qr::Matrix,
    options: &Options,
) -> Result<(), QrError> {
    let side = options.size;
    let across = matrix.width as u32 + 2 * options.quiet_zone;
    let module = side / across;

    // A symbol whose modules do not all fit in whole pixels would be scaled down
    // by the platform displaying it, which softens every edge. Saying so is more
    // useful than silently producing a blurry code: the caller can raise `size`.
    if module == 0 {
        return Err(QrError::data(format!(
            "a {side} pixel image cannot hold a {} module code with a quiet zone of {} modules: \
             a module would be smaller than one pixel",
            matrix.width, options.quiet_zone
        )));
    }

    // Whatever is left over after the whole modules is split between the two
    // sides, so the symbol stays centred.
    let origin = (side - module * across) / 2;

    for row in 0..matrix.width {
        for column in 0..matrix.width {
            if !matrix.is_dark(column, row) {
                continue;
            }

            let x = origin + (column as u32 + options.quiet_zone) * module;
            let y = origin + (row as u32 + options.quiet_zone) * module;

            paint::fill_rect(image, x, y, module, options.foreground);
        }
    }

    Ok(())
}

/// Draw the plate and the text on it.
fn draw_label(image: &mut RgbaImage, text: &str, options: &Options) -> Result<(), QrError> {
    let font = font::font()?;
    let side = options.size as f32;
    let plate_side = (side * options.label_size as f32).round();
    let plate_left = ((side - plate_side) / 2.0).round();
    let plate_top = plate_left;
    let margin = options.label_margin as f32;

    let layout =
        label::layout(&font, text, plate_side, margin, options.label_rows).ok_or_else(|| {
            QrError::data(format!(
                "the label {text:?} cannot be drawn in a {plate_side:.0} pixel plate with a \
                 {margin:.0} pixel margin"
            ))
        })?;

    // The plate first, because it erases the modules it covers; the text second,
    // composited over the plate.
    paint::fill_rounded_rect(
        image,
        plate_left,
        plate_top,
        plate_side,
        plate_side * options.label_radius as f32,
        options.label_background,
    );
    label::draw(
        image,
        &font,
        &layout,
        plate_left,
        plate_top,
        plate_side,
        options.label_color,
    );

    Ok(())
}

/// Encode the picture as a PNG.
///
/// Adaptive filtering with the default deflate level: a QR code is large flat
/// areas and repeated pixels, so adaptive filtering is where the size comes from,
/// and the highest deflate level buys a few more percent for a much larger
/// multiple of the time. The error is reported rather than unwrapped because a
/// failure here is the one thing that would otherwise take the whole 1C session
/// down with it.
fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, QrError> {
    let mut bytes = Vec::new();

    PngEncoder::new_with_quality(&mut bytes, CompressionType::Default, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| {
            QrError::internal(format!("the picture cannot be encoded as PNG: {error}"))
        })?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::render_with;
    use crate::error::ErrorKind;
    use crate::options::Options;

    /// The PNG signature: eight bytes every PNG begins with.
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    #[test]
    fn the_result_is_a_png() {
        let bytes = render_with("HELLO", "AB12", &Options::default()).expect("a picture");

        assert_eq!(bytes[..8], PNG_SIGNATURE);
    }

    #[test]
    fn the_same_input_produces_the_same_picture() {
        // PNG encoding is deterministic and the drawing is arithmetic, so this has
        // to hold. It is also what lets a consumer compare or cache the result.
        let options = Options::default();

        let first = render_with("ORDER-1", "AB12", &options).expect("a picture");
        let second = render_with("ORDER-1", "AB12", &options).expect("a picture");

        assert_eq!(first, second);
    }

    #[test]
    fn a_plate_with_no_room_for_the_text_is_reported_rather_than_drawn_badly() {
        let options = Options {
            size: 100,
            label_size: 0.1,
            label_margin: 5,
            ..Options::default()
        };

        let error = render_with("HELLO", "AB12", &options).expect_err("a 10 pixel plate");

        assert_eq!(error.kind, ErrorKind::Data);
        assert!(error.message.contains("AB12"), "reads: {}", error.message);
    }

    #[test]
    fn a_size_too_small_for_the_module_grid_is_reported() {
        // 64 pixels is the smallest allowed size, and a version 10 symbol at level
        // H is 57 modules across, so with a quiet zone it cannot fit in whole
        // pixels. The message has to say which of the three is at fault.
        let options = Options {
            size: 64,
            quiet_zone: 4,
            ..Options::default()
        };

        let error = render_with(&"x".repeat(300), "", &options).expect_err("no room for modules");

        assert_eq!(error.kind, ErrorKind::Data);
        assert!(
            error.message.contains("quiet zone"),
            "reads: {}",
            error.message
        );
    }

    #[test]
    fn an_empty_label_is_not_an_error_and_leaves_no_plate() {
        // A caller who wants a plain QR code passes no text, and gets no white
        // square in the middle of it.
        let plain = render_with("HELLO", "", &Options::default()).expect("a picture");
        let spaced = render_with("HELLO", "   ", &Options::default()).expect("a picture");

        assert_eq!(plain, spaced);
    }
}

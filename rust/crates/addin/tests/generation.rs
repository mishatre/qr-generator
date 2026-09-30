//! What the component produces, checked by reading the picture back.
//!
//! These tests are the reason this project can claim its output is usable without
//! a 1C licence: `rqrr` is a real QR decoder, so "the code with a label in the
//! middle still scans" is measured rather than assumed. A test that only checked
//! the PNG's size and a few pixels would pass just as happily on a picture no
//! scanner can read — which is exactly the failure mode a label in the middle of a
//! symbol invites.
//!
//! The pictures are flattened onto white before decoding. That is not a
//! convenience: the default background is transparent, the decoder reads the
//! luminance channel, and a transparent pixel's luminance is black — so decoding
//! the raw buffer would read every empty pixel as a module. Whoever displays the
//! code puts something behind it, white in every case this component is for, and
//! that is what the tests put there too.

use image::{GrayImage, ImageFormat, Luma, Rgba, RgbaImage};
use qr_generator_addin::{options::Options, qr, render_with, Color};

/// The payloads the tests scan back. Deliberately a spread: short, long, Cyrillic,
/// and one that needs the largest symbols the encoder will pick.
const PAYLOADS: [&str; 6] = [
    "ORDER-2026-0001",
    "https://example.com/orders/2026/0001?source=qr",
    "Заказ №42 от 30.09.2026",
    "1234567890",
    "A",
    "The quick brown fox jumps over the lazy dog, twice, to make the payload long \
     enough that the symbol needs a high version and the label covers more modules.",
];

/// Decode a rendered PNG and insist it carries the payload.
fn decode(png: &[u8]) -> String {
    match try_decode(png) {
        Some(content) => content,
        None => panic!("the decoder could not read the code back"),
    }
}

/// Decode a rendered PNG, or `None` when it cannot be read.
///
/// A located-but-unreadable code comes back as `None` too: for the caller of this
/// component, "the scanner found it and gave an error" and "the scanner found
/// nothing" are the same failure.
fn try_decode(png: &[u8]) -> Option<String> {
    let mut prepared = rqrr::PreparedImage::prepare(flattened(png));
    let grids = prepared.detect_grids();

    assert!(
        grids.len() <= 1,
        "the decoder found {} codes in a picture of one",
        grids.len()
    );

    grids
        .first()
        .and_then(|grid| grid.decode().ok())
        .map(|(_metadata, content)| content)
}

/// The rendered picture as RGBA.
fn picture(png: &[u8]) -> RgbaImage {
    image::load_from_memory_with_format(png, ImageFormat::Png)
        .expect("the component returns a PNG")
        .to_rgba8()
}

/// Composite a rendered picture onto white and convert it to the luminance the
/// decoder works on.
fn flattened(png: &[u8]) -> GrayImage {
    let picture = picture(png);
    let mut gray = GrayImage::new(picture.width(), picture.height());

    for (x, y, pixel) in picture.enumerate_pixels() {
        gray.put_pixel(x, y, Luma([onto_white(*pixel)]));
    }

    gray
}

/// The luminance of a pixel over a white surface.
fn onto_white(pixel: Rgba<u8>) -> u8 {
    let alpha = f32::from(pixel[3]) / 255.0;
    // Every colour the component draws is a grey, so one channel carries the
    // luminance; the blue channel is used so that a caller who sets a coloured
    // background cannot make this helper disagree with the decoder for free.
    let value = f32::from(pixel[2]) * alpha + 255.0 * (1.0 - alpha);

    value.round().clamp(0.0, 255.0) as u8
}

#[test]
fn a_labelled_code_scans_back_to_the_payload() {
    for payload in PAYLOADS {
        let options = Options::default();
        let png = render_with(payload, "AB12", &options).expect("a picture");

        assert_eq!(
            decode(&png),
            payload,
            "a code carrying {payload:?} with a label over its middle did not read back"
        );
    }
}

#[test]
fn a_code_scans_without_a_label_too() {
    // The label is the feature, but a plain code has to work as well: a caller
    // that wants one should not have to know that the layout search is skipped.
    let png = render_with("ORDER-2026-0001", "", &Options::default()).expect("a picture");

    assert_eq!(decode(&png), "ORDER-2026-0001");
}

#[test]
fn the_label_does_not_have_to_be_four_characters() {
    // The layout search is the difference between this and a hard-coded 2×2 label:
    // every one of these has to come back, and the label has to be large enough to
    // read, which is checked by the size assertions further down.
    for label in ["A", "AB", "AB12", "ORDER", "Заказ", "12/34", "ABC DEF GHI"] {
        let png = render_with("ORDER-2026-0001", label, &Options::default()).expect("a picture");

        assert_eq!(
            decode(&png),
            "ORDER-2026-0001",
            "the label {label:?} broke the scan"
        );
    }
}

#[test]
fn the_payloads_from_the_live_service_scan_back() {
    // The record format, as the consumer's own code documents it:
    //
    // ```text
    // Тип | Код/Артикул | Серия | Срок | XXXXXXXXYYYY
    //
    // XXXXXXXX — идентификатор экземпляра
    // YYYY     — визуальный/проверочный код
    // ```
    //
    // The label drawn in the middle of the code is that four character visual code.
    // The service this component replaces takes the last four *characters* of the
    // payload (`code.slice(-4)`), which is the same thing for a well-formed record
    // and crosses a field boundary for a truncated one — which is what the first
    // payload below is: a record the service answered with on 2026-09-30, whose
    // last field had been cut down to "1", so its label reads "27|1".
    const AS_SERVED: &str =
        "RUNZE-%20DC-032-1/4-28|124021-0094И68-06 %200124-00184Й42-1|09.03.2027|1";

    // A well-formed record of the same shape, as the consumer's own code writes
    // them: type 2, a ten digit article, a nine character series, a date, and then
    // eight characters of instance identifier followed by the four character visual
    // code `AX4R` that belongs in the middle of the symbol.
    const WELL_FORMED: &str = "2|2000020302|0085F0226|01022031|5RCD6VPEAX4R";

    // Both need a mid-sized symbol rather than the smallest one, and that matters:
    // what the error correction has to pay for is the share of the symbol the label
    // covers, so a symbol this size affords a larger label than a short order code
    // does.
    for (payload, label) in [(AS_SERVED, "27|1"), (WELL_FORMED, "AX4R")] {
        let width = qr::encode(payload, Options::default().error_correction_level)
            .expect("a code")
            .width;

        assert!(
            width >= 33,
            "the payload {payload:?} produced a {width} module symbol, which is too small for the \
             label this test is about"
        );

        let png = render_with(payload, label, &Options::default()).expect("a picture");

        assert_eq!(
            decode(&png),
            payload,
            "the payload with the label {label:?} did not read back"
        );
    }

    // And the same payload with a hole instead of the plate, which is what a caller
    // migrating from the service would ask for: the service draws no plate, leaving
    // the label on whatever surface the code is printed on. This is one of the two
    // deliberate differences from that service's picture; the other is that this
    // component floors the module size to whole pixels where the service renders its
    // SVG at 600/41 = 14.63 pixels per module and lets every edge be anti-aliased.
    let hole = Options {
        label_background: Color::TRANSPARENT,
        ..Options::default()
    };
    let png = render_with(AS_SERVED, "27|1", &hole).expect("a picture");

    assert_eq!(decode(&png), AS_SERVED);
}

#[test]
fn a_label_at_four_tenths_of_the_image_does_not_scan_on_a_small_symbol() {
    // The measured ceiling, kept as a test so that a change which makes the label
    // cheaper to pay for shows up here rather than in a customer's scanner.
    //
    // Measured on 2026-09-30 with rqrr over the payloads above, at size 600 and no
    // quiet zone: a 25 module symbol (a 15 character order code) reads back at a
    // label of 0.35 and fails with a data error at 0.4, and a 41 module symbol
    // reads back at 0.4. What the error correction has to pay for is the share of
    // the *symbol* the label covers, and a bigger symbol can afford a bigger share
    // because the perimeter of the hole is a smaller part of it.
    //
    // This is why the default label is 0.3 rather than the 0.4 the implementation
    // this component is modelled on uses.
    let options = Options {
        label_size: 0.4,
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &options).expect("a picture");

    assert_ne!(
        try_decode(&png).as_deref(),
        Some("ORDER-2026-0001"),
        "a 0.4 label on a 25 module symbol now reads back, which means the ceiling moved: the \
         default label of 0.3 can probably be raised, and this test and the note in options.rs \
         should be revisited"
    );
}

#[test]
fn a_quiet_zone_leaves_less_of_the_symbol_for_the_label() {
    // The same label size is more damaging with a quiet zone, and this is the trap
    // worth pinning: the label is sized against the *image*, while what matters is
    // the share of the *symbol* it covers — and a quiet zone shrinks the symbol
    // inside the same image, and its modules with it. Measured: 0.3 reads back
    // without a quiet zone and does not with four modules of one, while 0.25 reads
    // back either way.
    let too_big = Options {
        label_size: 0.3,
        quiet_zone: 4,
        ..Options::default()
    };
    let comfortable = Options {
        label_size: 0.25,
        quiet_zone: 4,
        ..Options::default()
    };

    let too_big_png = render_with("ORDER-2026-0001", "AB12", &too_big).expect("a picture");
    let comfortable_png = render_with("ORDER-2026-0001", "AB12", &comfortable).expect("a picture");

    assert_ne!(
        try_decode(&too_big_png).as_deref(),
        Some("ORDER-2026-0001"),
        "the note in docs/plan/qr-contract.md about the quiet zone is now wrong"
    );
    assert_eq!(
        decode(&comfortable_png),
        "ORDER-2026-0001",
        "and the documented fix for it is now wrong too"
    );
}

#[test]
fn the_quiet_zone_is_drawn_where_the_option_says() {
    // With no quiet zone the finder pattern starts at the very edge of the drawn
    // symbol; with four modules of margin the same pixel is background. Both are
    // compared against the module grid the encoder chose rather than a guessed
    // pixel, so this cannot pass because the symbol happened to be a different size.
    let payload = "ORDER-2026-0001";
    let width = qr::encode(payload, Options::default().error_correction_level)
        .expect("a code")
        .width as u32;
    let size = 600;

    let flush = Options {
        size,
        quiet_zone: 0,
        ..Options::default()
    };
    let quiet = Options {
        size,
        quiet_zone: 4,
        ..Options::default()
    };

    let flush_module = size / width;
    let flush_origin = (size - flush_module * width) / 2;
    let quiet_module = size / (width + 8);
    let quiet_origin = (size - quiet_module * (width + 8)) / 2;

    let flush_png = render_with(payload, "", &flush).expect("a picture");
    let quiet_png = render_with(payload, "", &quiet).expect("a picture");

    // The top-left module of a QR code is the corner of a finder pattern: dark.
    assert!(
        dark_at(&flush_png, flush_origin + 1, flush_origin + 1),
        "with no quiet zone the symbol has to start at the image origin"
    );

    // With a quiet zone, everything between the origin and the first module is
    // background — and the background here is transparent, so "background" means
    // "not dark".
    assert!(
        !dark_at(&quiet_png, quiet_origin + 1, quiet_origin + 1),
        "the quiet zone has to be empty"
    );
    assert!(
        dark_at(
            &quiet_png,
            quiet_origin + 4 * quiet_module + 1,
            quiet_origin + 4 * quiet_module + 1
        ),
        "and the symbol has to start after it"
    );
}

#[test]
fn the_picture_is_the_size_the_option_asked_for() {
    for size in [200, 600, 1024] {
        let options = Options {
            size,
            ..Options::default()
        };
        let picture = image::load_from_memory_with_format(
            &render_with("ORDER-2026-0001", "AB12", &options).expect("a picture"),
            ImageFormat::Png,
        )
        .expect("a PNG")
        .to_rgba8();

        assert_eq!(picture.width(), size);
        assert_eq!(picture.height(), size, "the picture is square");
    }
}

#[test]
fn the_label_plate_hides_the_modules_under_it() {
    // This is hideBackgroundDots: without it the modules show through the plate and
    // every character is fringed. Measured on the pixels with a red label, so that
    // "a grey pixel inside the plate" can only be a module — a red character over a
    // white plate produces reds and pinks, never a grey.
    let options = Options {
        label_color: Color::parse("#ff0000").expect("a colour"),
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &options).expect("a picture");
    let picture = picture(&png);
    let side = (options.size as f32 * options.label_size as f32).round();
    let left = ((options.size as f32 - side) / 2.0).round() as u32;
    let last = left + side as u32 - 1;
    let radius = (side * options.label_radius as f32).round() as u32;

    let mut inspected = 0usize;
    for y in left..=last {
        for x in left..=last {
            // A rounded corner is outside the plate, so a module may legitimately
            // show through there; the four corner boxes are the only place skipped.
            let near_horizontal_corner = x < left + radius || x + radius > last;
            let near_vertical_corner = y < left + radius || y + radius > last;

            if near_horizontal_corner && near_vertical_corner {
                continue;
            }

            let pixel = picture.get_pixel(x, y);

            assert!(
                !(pixel[0] == pixel[1] && pixel[1] == pixel[2] && pixel[0] < 250),
                "the pixel at ({x}, {y}) is {pixel:?}, which is a module or its edge surviving \
                 inside the label plate"
            );
            inspected += 1;
        }
    }

    assert!(
        inspected > 10_000,
        "only {inspected} pixels were inspected, which proves little"
    );
}

#[test]
fn a_transparent_plate_leaves_a_hole_instead_of_a_plate() {
    // The label background may be "transparent", and what that means is a hole, not
    // an invisible plate: the modules under the label are removed and nothing is
    // painted in their place, so whatever the picture ends up on shows through. That
    // is what a code printed onto a coloured card needs - an invisible-plate reading
    // would leave the modules behind the text, and a white plate would punch a white
    // rectangle out of the card.
    let transparent = Options {
        label_background: Color::parse("transparent").expect("a colour"),
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &transparent).expect("a picture");
    let image = picture(&png);
    let side = (transparent.size as f32 * transparent.label_size as f32).round();
    let left = ((transparent.size as f32 - side) / 2.0).round() as u32;
    let last = left + side as u32 - 1;
    let centre = (left + last) / 2;

    // A few pixels inside the left edge of the plate: inside it, past the rounded
    // corner, and short of where the text starts (the plate leaves the text a margin).
    let probe = (left + 3, centre);

    let opaque_png =
        render_with("ORDER-2026-0001", "AB12", &Options::default()).expect("a picture");
    let opaque = picture(&opaque_png);

    assert_eq!(
        opaque.get_pixel(probe.0, probe.1)[3],
        255,
        "the default plate is opaque at {probe:?}, which is what makes this comparison mean something"
    );
    assert_eq!(
        image.get_pixel(probe.0, probe.1)[3],
        0,
        "the transparent plate left {probe:?} opaque, so the modules were not removed"
    );

    // The text is still drawn where the plate was, and the code still reads when the
    // hole is put on white - the hole costs the symbol the same share an opaque plate
    // does, so the measured label ceiling applies to both.
    assert!(
        image.pixels().any(|pixel| pixel[3] == 255),
        "nothing in the picture is opaque, so the label text is missing too"
    );
    assert_eq!(decode(&png), "ORDER-2026-0001");
}

#[test]
fn the_label_text_is_drawn_in_the_label_colour() {
    let options = Options {
        label_color: Color::parse("#ff0000").expect("a colour"),
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &options).expect("a picture");
    let picture = image::load_from_memory_with_format(&png, ImageFormat::Png)
        .expect("a PNG")
        .to_rgba8();

    let red = picture
        .pixels()
        .filter(|pixel| pixel[0] > 128 && pixel[1] < 128 && pixel[2] < 128)
        .count();

    assert!(
        red > 100,
        "only {red} pixels are label-red, so the text was not drawn in the colour asked for"
    );

    // And the same code still scans with a coloured label on it.
    assert_eq!(decode(&png), "ORDER-2026-0001");
}

#[test]
fn an_opaque_background_covers_the_whole_picture() {
    // A quiet zone is asked for here so that the corner of the picture is
    // background rather than the corner of the symbol.
    let options = Options {
        quiet_zone: 4,
        background: Color::parse("#ffffff").expect("a colour"),
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &options).expect("a picture");
    let picture = picture(&png);

    assert!(
        picture.pixels().all(|pixel| pixel[3] == 255),
        "an opaque background option has to leave no transparent pixel behind"
    );
    assert_eq!(picture.get_pixel(0, 0), &Color::rgb(255, 255, 255).pixel());
}

#[test]
fn the_default_background_is_transparent_so_the_code_can_be_placed_on_anything() {
    let options = Options {
        quiet_zone: 4,
        ..Options::default()
    };
    let png = render_with("ORDER-2026-0001", "AB12", &options).expect("a picture");
    let picture = picture(&png);

    assert_eq!(picture.get_pixel(0, 0)[3], 0, "the margin is transparent");
    assert!(
        picture.pixels().any(|pixel| pixel[3] == 0),
        "and so is everything that is not a module"
    );
    assert!(
        picture.pixels().any(|pixel| pixel[3] == 255),
        "while the modules and the plate are opaque"
    );
}

#[test]
fn a_picture_of_a_realistic_size_is_small_enough_to_store_in_a_value_storage() {
    // Not a hard limit — a 1C value storage holds megabytes — but a QR code that
    // came out at tens of kilobytes would mean the encoder had stopped compressing,
    // and a consumer would notice it as a slow attachment.
    let png = render_with(
        "https://example.com/orders/2026/0001",
        "AB12",
        &Options::default(),
    )
    .expect("a picture");

    assert!(
        png.len() < 32 * 1024,
        "a 600 pixel labelled code came out at {} bytes",
        png.len()
    );
}

/// Whether the pixel at this position is dark, by the decoder's own luminance.
fn dark_at(png: &[u8], x: u32, y: u32) -> bool {
    onto_white(
        *image::load_from_memory_with_format(png, ImageFormat::Png)
            .expect("a PNG")
            .to_rgba8()
            .get_pixel(x, y),
    ) < 128
}

//! Pixel operations: compositing, rectangles, and the rounded plate the label
//! sits on.
//!
//! Everything here works in image coordinates with the origin at the top left, and
//! every entry point tolerates coordinates that fall outside the picture, because
//! a glyph rasterised near an edge reports pixels that do not exist. Clamping
//! belongs here rather than in the callers, which know only about geometry.

use image::{Rgba, RgbaImage};

use crate::color::Color;

/// The area drawing is allowed to touch, in pixels.
///
/// The label is clipped to its own plate: without this, a glyph whose side bearing
/// makes it a fraction wider than the row would put dark pixels on the modules
/// beside the plate, which is exactly the damage the plate exists to contain.
#[derive(Debug, Clone, Copy)]
pub struct Clip {
    /// Left edge, inclusive.
    pub left: f32,
    /// Top edge, inclusive.
    pub top: f32,
    /// Right edge, exclusive.
    pub right: f32,
    /// Bottom edge, exclusive.
    pub bottom: f32,
}

impl Clip {
    /// The whole picture.
    pub fn picture(width: u32, height: u32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: width as f32,
            bottom: height as f32,
        }
    }

    /// A square area.
    pub fn square(left: f32, top: f32, side: f32) -> Self {
        Self {
            left,
            top,
            right: left + side,
            bottom: top + side,
        }
    }

    /// Whether the centre of this pixel is inside the area.
    fn contains(&self, x: i32, y: i32) -> bool {
        let x = x as f32 + 0.5;
        let y = y as f32 + 0.5;

        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// Composite `color` over the pixel at `(x, y)`.
pub fn over(image: &mut RgbaImage, x: i32, y: i32, color: Color, clip: Clip) {
    let Some(pixel) = pixel_at(image, x, y, clip) else {
        return;
    };

    *pixel = source_over(color.pixel(), *pixel);
}

/// Paint the label plate over whatever is already there.
///
/// Two behaviours, and which one applies is the caller's decision:
///
/// * An **opaque** colour replaces what is under it. That is what removes the
///   modules the plate covers, and it is why the plate needs no separate "hide the
///   modules" step: a module painted over at full coverage is gone, and only the
///   anti-aliased rim of a rounded corner blends with it at all.
/// * A **transparent** colour means "remove what is here". The caller wants a hole
///   the size of the plate, not a plate, and compositing an invisible colour over
///   opaque pixels does nothing at all — so a transparent plate drawn that way
///   would leave every module in place behind the text. The hole is made by
///   multiplying the destination's opacity instead.
pub fn stamp(image: &mut RgbaImage, x: i32, y: i32, color: Color, coverage: f32) {
    let coverage = coverage.clamp(0.0, 1.0);

    let Some(pixel) = pixel_at(image, x, y, Clip::picture(image.width(), image.height())) else {
        return;
    };

    if color.is_transparent() {
        pixel[3] = (f32::from(pixel[3]) * (1.0 - coverage)).round() as u8;

        return;
    }

    *pixel = source_over(color.with_coverage(coverage).pixel(), *pixel);
}

/// Fill an axis-aligned rectangle with an opaque colour.
///
/// No anti-aliasing and no blending: a QR module is either there or not, and a
/// half-covered module at an edge would be a grey module, which a scanner reads as
/// noise. The caller sizes the modules so that this never has to round.
pub fn fill_rect(image: &mut RgbaImage, left: u32, top: u32, side: u32, color: Color) {
    let right = left.saturating_add(side).min(image.width());
    let bottom = top.saturating_add(side).min(image.height());

    for y in top.min(image.height())..bottom {
        for x in left.min(image.width())..right {
            *image.get_pixel_mut(x, y) = color.pixel();
        }
    }
}

/// Fill a square with rounded corners, anti-aliased at the corners and the edges.
pub fn fill_rounded_rect(
    image: &mut RgbaImage,
    left: f32,
    top: f32,
    side: f32,
    radius: f32,
    color: Color,
) {
    let first_x = (left - 1.0).floor() as i32;
    let last_x = (left + side + 1.0).ceil() as i32;
    let first_y = (top - 1.0).floor() as i32;
    let last_y = (top + side + 1.0).ceil() as i32;

    for y in first_y..last_y {
        for x in first_x..last_x {
            let coverage = rounded_rect_coverage(x, y, left, top, side, radius);

            if coverage > 0.0 {
                stamp(image, x, y, color, coverage);
            }
        }
    }
}

/// How much of the pixel at `(x, y)` is inside a rounded square.
///
/// The signed distance to the shape, turned into a coverage: a pixel centre a
/// half-pixel outside is half covered, which is the standard way to anti-alias a
/// shape without supersampling.
fn rounded_rect_coverage(x: i32, y: i32, left: f32, top: f32, side: f32, radius: f32) -> f32 {
    let centre_x = x as f32 + 0.5;
    let centre_y = y as f32 + 0.5;
    let half = side / 2.0;
    let straight_x = (centre_x - (left + half)).abs() - (half - radius);
    let straight_y = (centre_y - (top + half)).abs() - (half - radius);

    let outside = (straight_x.max(0.0).powi(2) + straight_y.max(0.0).powi(2)).sqrt();
    let inside = straight_x.max(straight_y).min(0.0);
    let distance = outside + inside - radius;

    (0.5 - distance).clamp(0.0, 1.0)
}

/// The pixel at `(x, y)`, or `None` when it is outside the picture or the clip.
fn pixel_at(image: &mut RgbaImage, x: i32, y: i32, clip: Clip) -> Option<&mut Rgba<u8>> {
    if x < 0 || y < 0 || !clip.contains(x, y) {
        return None;
    }

    let (x, y) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);

    if x >= image.width() || y >= image.height() {
        return None;
    }

    Some(image.get_pixel_mut(x, y))
}

/// Composite `source` over `destination` in straight (non-premultiplied) alpha.
fn source_over(source: Rgba<u8>, destination: Rgba<u8>) -> Rgba<u8> {
    let source_alpha = f32::from(source[3]) / 255.0;

    if source_alpha <= 0.0 {
        return destination;
    }

    if source_alpha >= 1.0 {
        return source;
    }

    let destination_alpha = f32::from(destination[3]) / 255.0;
    let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    let channel = |index: usize| {
        let value = (f32::from(source[index]) * source_alpha
            + f32::from(destination[index]) * destination_alpha * (1.0 - source_alpha))
            / out_alpha;

        value.round().clamp(0.0, 255.0) as u8
    };

    Rgba([
        channel(0),
        channel(1),
        channel(2),
        (out_alpha * 255.0).round() as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::{fill_rect, fill_rounded_rect, over, rounded_rect_coverage, Clip};
    use crate::color::Color;
    use image::{Rgba, RgbaImage};

    #[test]
    fn a_pixel_outside_the_picture_is_left_alone_rather_than_panicking() {
        let mut image = RgbaImage::from_pixel(4, 4, Color::TRANSPARENT.pixel());
        let clip = Clip::picture(4, 4);

        over(&mut image, -1, 0, Color::rgb(255, 0, 0), clip);
        over(&mut image, 0, -1, Color::rgb(255, 0, 0), clip);
        over(&mut image, 4, 0, Color::rgb(255, 0, 0), clip);
        over(&mut image, 999, 999, Color::rgb(255, 0, 0), clip);

        assert!(image.pixels().all(|pixel| pixel[3] == 0));
    }

    #[test]
    fn the_clip_keeps_drawing_where_the_caller_says() {
        let mut image = RgbaImage::from_pixel(4, 4, Color::TRANSPARENT.pixel());
        let clip = Clip::square(1.0, 1.0, 2.0);

        over(&mut image, 0, 1, Color::rgb(255, 0, 0), clip);
        over(&mut image, 1, 1, Color::rgb(255, 0, 0), clip);

        assert_eq!(image.get_pixel(0, 1)[3], 0, "outside the clip");
        assert_eq!(image.get_pixel(1, 1), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn a_rect_fills_without_anti_aliasing() {
        let mut image = RgbaImage::from_pixel(4, 4, Color::TRANSPARENT.pixel());

        fill_rect(&mut image, 1, 1, 2, Color::rgb(0, 0, 0));

        assert_eq!(image.get_pixel(1, 1), &Rgba([0, 0, 0, 255]));
        assert_eq!(image.get_pixel(2, 2), &Rgba([0, 0, 0, 255]));
        // The module beside it is untouched: a half-covered module would be grey.
        assert_eq!(image.get_pixel(0, 1)[3], 0);
        assert_eq!(image.get_pixel(3, 1)[3], 0);
    }

    #[test]
    fn a_rect_that_runs_off_the_edge_is_clipped_rather_than_fatal() {
        let mut image = RgbaImage::from_pixel(4, 4, Color::TRANSPARENT.pixel());

        fill_rect(&mut image, 3, 3, 10, Color::rgb(0, 0, 0));

        assert_eq!(image.get_pixel(3, 3), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn an_opaque_plate_replaces_what_is_under_it() {
        // This is hideBackgroundDots: the plate has to remove the modules it
        // covers, and an opaque colour at full coverage does exactly that.
        let mut image = RgbaImage::from_pixel(8, 8, Color::TRANSPARENT.pixel());
        fill_rect(&mut image, 0, 0, 8, Color::rgb(0x11, 0x11, 0x11));

        fill_rounded_rect(&mut image, 2.0, 2.0, 4.0, 0.0, Color::rgb(255, 255, 255));

        assert_eq!(image.get_pixel(3, 3), &Rgba([255, 255, 255, 255]));
        assert_eq!(
            image.get_pixel(1, 1),
            &Rgba([0x11, 0x11, 0x11, 255]),
            "outside the plate"
        );
    }

    #[test]
    fn an_opaque_plate_leaves_nothing_behind_that_is_transparent() {
        // Measured the hard way: an earlier version of this painted the plate by
        // erasing the pixel first and then compositing the colour over it, which
        // made the anti-aliased rim of the plate partly transparent even when both
        // the plate and the background were opaque. On a picture whose whole point
        // is to be printed on something, a hairline of transparency around the label
        // shows up as a white halo on a coloured page.
        let mut image = RgbaImage::from_pixel(16, 16, Color::rgb(0xff, 0xff, 0xff).pixel());
        fill_rect(&mut image, 0, 0, 16, Color::rgb(0x11, 0x11, 0x11));
        fill_rounded_rect(
            &mut image,
            0.0,
            0.0,
            16.0,
            5.0,
            Color::rgb(0xff, 0xff, 0xff),
        );

        for (x, y, pixel) in image.enumerate_pixels() {
            assert_eq!(
                pixel[3], 255,
                "the pixel at ({x}, {y}) is {pixel:?}, which is transparent in a picture whose \
                 background is opaque"
            );
        }
    }

    #[test]
    fn a_transparent_plate_punches_a_hole_instead_of_painting_one() {
        let mut image = RgbaImage::from_pixel(8, 8, Color::TRANSPARENT.pixel());
        fill_rect(&mut image, 0, 0, 8, Color::rgb(0x11, 0x11, 0x11));

        fill_rounded_rect(&mut image, 2.0, 2.0, 4.0, 0.0, Color::TRANSPARENT);

        assert_eq!(
            image.get_pixel(3, 3)[3],
            0,
            "the modules under the plate are gone"
        );
        assert_eq!(image.get_pixel(1, 1)[3], 255, "and only those");
    }

    #[test]
    fn the_corners_are_rounded_and_anti_aliased() {
        let middle = rounded_rect_coverage(5, 5, 0.0, 0.0, 10.0, 2.0);

        // The corner circle has its centre at (8, 8) and a radius of 2, so the
        // centre of the corner-most pixel, (9.5, 9.5), is 2.12 from it: outside the
        // shape, and only partly covered by the anti-aliasing ramp.
        let corner = rounded_rect_coverage(0, 0, 0.0, 0.0, 10.0, 2.0);
        let far_corner = rounded_rect_coverage(9, 9, 0.0, 0.0, 10.0, 2.0);
        let outside = rounded_rect_coverage(20, 20, 0.0, 0.0, 10.0, 2.0);

        assert_eq!(middle, 1.0);
        assert!(
            corner > 0.0 && corner < 1.0,
            "a rounded corner is partly covered, got {corner}"
        );
        assert_eq!(far_corner, corner, "the plate is symmetric");
        assert_eq!(outside, 0.0);
    }

    #[test]
    fn a_plate_on_whole_pixels_has_crisp_edges_and_soft_corners() {
        // The anti-aliasing ramp is a half pixel either side of the outline, so a
        // plate whose edges fall on whole pixels has no half-lit pixel along them —
        // only the rounded corners are partly covered. That is the behaviour worth
        // having: a soft edge on a plate the size of a module would read as a
        // misprint.
        assert_eq!(
            rounded_rect_coverage(5, 9, 0.0, 0.0, 10.0, 2.0),
            1.0,
            "inside"
        );
        assert_eq!(
            rounded_rect_coverage(5, 10, 0.0, 0.0, 10.0, 2.0),
            0.0,
            "outside"
        );

        let corner = rounded_rect_coverage(0, 0, 0.0, 0.0, 10.0, 2.0);

        assert!(
            corner > 0.0 && corner < 1.0,
            "a rounded corner is partly covered, got {corner}"
        );
    }

    #[test]
    fn a_square_plate_is_not_cut_at_all() {
        // A radius of zero has to behave like a rectangle, or `labelRadius: 0`
        // would shave the plate's corners for no reason.
        let corner = rounded_rect_coverage(0, 0, 0.0, 0.0, 10.0, 0.0);

        assert!(corner > 0.0, "got {corner}");
    }
}

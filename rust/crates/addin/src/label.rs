//! Where the label's characters go.
//!
//! Two problems live here, and they are separate. One is *how big* the text can
//! be: the plate is a fixed square and the text has to fill as much of it as it
//! can without touching the edges. The other is *how many rows* to use, which is
//! what decides how big the characters end up.
//!
//! The row count is searched rather than guessed, because the answer is not
//! obvious: four characters on one row are limited by half the plate's width,
//! while the same four in two rows of two are limited by half its height — and at
//! the plate proportions this component renders, two rows win by a factor of about
//! 1.6. That is not a coincidence of one example; it is why the sibling service
//! this component replaces hard-codes a 2×2 label. Searching means a three-
//! character label, a six-character one and a sixteen-character one each get the
//! layout that suits them, instead of one hard-coded shape.
//!
//! The unit everything is measured in is the *em*: the amount the pen advances
//! per character at a font size of one, with the ink extent of a row in the same
//! units. Measuring at a font size of one and scaling afterwards is what makes the
//! search cheap — the metrics are computed once per candidate split, not once per
//! candidate size.

use ab_glyph::{point, Font, Glyph, PxScale, ScaleFont};
use image::RgbaImage;

use crate::color::Color;
use crate::paint::{self, Clip};

/// How many rows the automatic search will consider.
///
/// Four is the most that ever wins: beyond that the characters are so small that
/// a caller wanting them should be asking for a larger `size` instead.
const MAX_AUTO_ROWS: usize = 4;

/// The gap between rows, as a fraction of the character height.
///
/// Enough that the rows read as separate lines, small enough that it does not cost
/// font size. A fifth of the cap height is the usual typographic starting point for
/// a display face with no descenders.
const ROW_GAP: f32 = 0.2;

/// The font size the ink extents are measured at, in pixels per em.
///
/// Not `1.0`, and this is the one trap in this module: the bounds the rasteriser
/// reports are the glyph's outline box **expanded to whole pixels**, so at a font
/// size of one pixel every extent comes back rounded up to one. A cap height of
/// 0.54 em and a digit's advance of 0.46 em both measure as 1.0, and the layout
/// search then works from numbers up to twice the truth — which is enough to make
/// it prefer one cramped row of text over two comfortable ones.
///
/// A thousand pixels per em puts the rounding at a thousandth of an em: a fifth of
/// a pixel on the plate this component renders by default. The rounding is always
/// outwards, so the error is in the safe direction — the text comes out a fraction
/// smaller than it could be, never larger than the plate.
///
/// Computing the outline at this size is not expensive: `outline_glyph` scales the
/// curve's control points, and only `draw` rasterises a bitmap.
const REFERENCE_SCALE: f32 = 1000.0;

/// The extent of a row's ink, in em units, relative to the pen and the baseline.
///
/// "Ink" is the pixels the glyphs actually cover, not the advance boxes: a space
/// advances the pen and covers nothing, and a digit's ink starts a little inside
/// its advance box. Using ink rather than advances is what centres the text
/// optically instead of arithmetically — which is the difference between a label
/// that looks centred and one that looks slightly shoved to the left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ink {
    /// Leftmost covered coordinate, relative to the pen.
    pub min_x: f32,
    /// Rightmost covered coordinate.
    pub max_x: f32,
    /// Topmost covered coordinate, relative to the baseline (negative is above it).
    pub min_y: f32,
    /// Bottommost covered coordinate.
    pub max_y: f32,
}

/// The value of "nothing has been covered yet".
const NO_INK: Ink = Ink {
    min_x: f32::INFINITY,
    max_x: f32::NEG_INFINITY,
    min_y: f32::INFINITY,
    max_y: f32::NEG_INFINITY,
};

impl Ink {
    /// Whether this row has no visible characters at all.
    pub fn is_empty(self) -> bool {
        self.min_x > self.max_x
    }

    /// Width of the ink, in em units.
    pub fn width(self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.max_x - self.min_x
        }
    }

    /// Height of the ink, in em units.
    pub fn height(self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.max_y - self.min_y
        }
    }

    /// Grow this extent to include a rectangle.
    fn union(self, other: Ink) -> Self {
        if other.is_empty() {
            return self;
        }

        Self {
            min_x: self.min_x.min(other.min_x),
            max_x: self.max_x.max(other.max_x),
            min_y: self.min_y.min(other.min_y),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// The same extent in different units.
    ///
    /// Empty extents hold infinities, and scaling leaves them infinite, so this is
    /// safe on a row of spaces.
    fn scale(self, factor: f32) -> Self {
        Self {
            min_x: self.min_x * factor,
            max_x: self.max_x * factor,
            min_y: self.min_y * factor,
            max_y: self.max_y * factor,
        }
    }
}

/// One row of the label, with the extents it was measured at.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The characters in this row.
    pub text: String,
    /// The row's ink extents at a font size of one.
    pub ink: Ink,
}

/// A label that fits: which rows, how large, and how far apart.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The font size to draw at, in pixels.
    pub font_size: f32,
    /// The rows, top to bottom.
    pub rows: Vec<Row>,
    /// The gap between rows, in pixels.
    pub gap: f32,
    /// The tallest row's ink height in em units, so the caller does not have to
    /// search for it again.
    pub ink_height: f32,
}

impl Layout {
    /// The total height of the text block, in pixels.
    pub fn height(&self) -> f32 {
        let rows = self.rows.len() as f32;

        rows * self.ink_height * self.font_size + (rows - 1.0) * self.gap
    }
}

/// Lay out `text` inside a square plate.
///
/// `rows_wanted` of zero asks for the row count that makes the characters largest.
/// A non-zero value is honoured exactly, which is how a caller gets the 2×2 label
/// regardless of what the search would have chosen. An explicit `\n` in the text
/// wins over both: a caller who broke the lines themselves meant it.
///
/// Returns `None` when nothing can be drawn — an empty text, or a plate with no
/// room left after the margin.
pub fn layout<F: Font>(
    font: &F,
    text: &str,
    plate_side: f32,
    margin: f32,
    rows_wanted: u32,
) -> Option<Layout> {
    let available = plate_side - 2.0 * margin;

    if available <= 0.0 {
        return None;
    }

    let text = text.trim();

    if text.is_empty() {
        return None;
    }

    let candidates = candidate_rows(text, rows_wanted);

    candidates
        .into_iter()
        .filter_map(|lines| fit(font, lines, available))
        .max_by(|left, right| {
            // Largest text wins. On a tie the layout with fewer rows wins, so that
            // a label that reads as one line is not split into two equally large
            // fragments for no gain.
            left.font_size
                .total_cmp(&right.font_size)
                .then(right.rows.len().cmp(&left.rows.len()))
        })
}

/// The row splits worth trying, best guess first.
fn candidate_rows(text: &str, rows_wanted: u32) -> Vec<Vec<String>> {
    if text.contains('\n') {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect::<Vec<String>>();

        return vec![lines];
    }

    if rows_wanted > 0 {
        return vec![balanced_rows(text, rows_wanted as usize)];
    }

    (1..=MAX_AUTO_ROWS)
        .map(|rows| balanced_rows(text, rows))
        .collect()
}

/// Split the text into `rows` rows of as equal a length as the characters allow.
///
/// By characters, not by bytes: a Cyrillic label is two bytes per character in
/// UTF-8, and splitting on bytes would cut a character in half.
fn balanced_rows(text: &str, rows: usize) -> Vec<String> {
    let characters = text.chars().collect::<Vec<char>>();
    let per_row = characters.len() / rows;
    let longer_rows = characters.len() % rows;

    let mut result = Vec::with_capacity(rows);
    let mut taken = 0;

    for row in 0..rows {
        // The remainder is spread over the first rows, so "ABC" in two rows is
        // "AB" and "C" rather than "A" and "BC".
        let length = per_row + usize::from(row < longer_rows);

        result.push(characters[taken..taken + length].iter().collect());
        taken += length;
    }

    result
}

/// Find the largest font size at which these rows fit the available square.
fn fit<F: Font>(font: &F, lines: Vec<String>, available: f32) -> Option<Layout> {
    let rows = lines
        .into_iter()
        .map(|text| Row {
            ink: measure(font, &text),
            text,
        })
        .collect::<Vec<Row>>();

    let ink_width = rows
        .iter()
        .map(|row| row.ink.width())
        .fold(0.0_f32, f32::max);
    let ink_height = rows
        .iter()
        .map(|row| row.ink.height())
        .fold(0.0_f32, f32::max);

    // A split that produced a row of spaces has no ink to size; there is nothing
    // to draw, so this candidate is not a candidate.
    if ink_width <= 0.0 || ink_height <= 0.0 {
        return None;
    }

    let gap_em = ROW_GAP * ink_height;
    let row_count = rows.len() as f32;
    let block_height_em = row_count * ink_height + (row_count - 1.0) * gap_em;

    let font_size = (available / ink_width).min(available / block_height_em);

    if !font_size.is_finite() || font_size <= 0.0 {
        return None;
    }

    Some(Layout {
        font_size,
        rows,
        gap: gap_em * font_size,
        ink_height,
    })
}

/// The ink extents of a row, in em units.
fn measure<F: Font>(font: &F, text: &str) -> Ink {
    let scaled = font.as_scaled(PxScale::from(REFERENCE_SCALE));
    let mut ink = NO_INK;
    let mut pen = 0.0_f32;
    let mut previous = None;

    for character in text.chars() {
        let id = font.glyph_id(character);

        if let Some(previous) = previous {
            pen += scaled.kern(previous, id);
        }

        if let Some(outlined) = font.outline_glyph(glyph(id, pen, 0.0, REFERENCE_SCALE)) {
            let bounds = outlined.px_bounds();

            ink = ink.union(Ink {
                min_x: bounds.min.x,
                max_x: bounds.max.x,
                min_y: bounds.min.y,
                max_y: bounds.max.y,
            });
        }

        pen += scaled.h_advance(id);
        previous = Some(id);
    }

    // Back to em units, which is what every caller works in.
    ink.scale(1.0 / REFERENCE_SCALE)
}

/// Draw the laid-out label centred in a square plate.
///
/// The plate is the clip: the caller has already painted it (and erased whatever
/// was under it), and the text must not reach past its edge.
pub fn draw<F: Font>(
    image: &mut RgbaImage,
    font: &F,
    layout: &Layout,
    plate_left: f32,
    plate_top: f32,
    plate_side: f32,
    color: Color,
) {
    let clip = Clip::square(plate_left, plate_top, plate_side);
    let centre_x = plate_left + plate_side / 2.0;
    let centre_y = plate_top + plate_side / 2.0;
    let size = layout.font_size;
    let scaled = font.as_scaled(PxScale::from(size));
    let mut ink_top = centre_y - layout.height() / 2.0;

    for row in &layout.rows {
        // Centre the row's ink in its band, and the band in the plate. Centring the
        // ink rather than the line box is what makes a row of digits sit in the
        // middle instead of floating above it: the line box reserves room for
        // descenders that "1234" does not have.
        let row_ink_height = row.ink.height() * size;
        let baseline =
            ink_top + row_ink_height / 2.0 - size * (row.ink.min_y + row.ink.max_y) / 2.0;
        let mut pen = centre_x - size * (row.ink.min_x + row.ink.max_x) / 2.0;
        let mut previous = None;

        for character in row.text.chars() {
            let id = font.glyph_id(character);

            if let Some(previous) = previous {
                pen += scaled.kern(previous, id);
            }

            if let Some(outlined) = font.outline_glyph(glyph(id, pen, baseline, size)) {
                let bounds = outlined.px_bounds();

                // `draw` reports the glyph's pixels relative to the floor of its
                // own bounding box, and the bounding box is already in image
                // coordinates — the position handed to `glyph` above is included.
                // Adding the two back together is what the rasteriser's contract
                // says; going through `i32` rather than `u32` keeps a glyph whose
                // box reaches past the top-left corner from wrapping to a huge
                // coordinate and landing somewhere it does not belong.
                outlined.draw(|x, y, coverage| {
                    let pixel_x = bounds.min.x.floor() as i32 + x as i32;
                    let pixel_y = bounds.min.y.floor() as i32 + y as i32;

                    paint::over(image, pixel_x, pixel_y, color.with_coverage(coverage), clip);
                });
            }

            pen += scaled.h_advance(id);
            previous = Some(id);
        }

        ink_top += row_ink_height + layout.gap;
    }
}

/// Position a glyph.
fn glyph(id: ab_glyph::GlyphId, pen: f32, baseline: f32, size: f32) -> Glyph {
    id.with_scale_and_position(PxScale::from(size), point(pen, baseline))
}

#[cfg(test)]
mod tests {
    use super::{layout, Layout};
    use crate::color::Color;
    use crate::font;
    use image::RgbaImage;

    /// The default plate: 600 pixels of image, a label 0.3 of the side.
    const PLATE: f32 = 180.0;
    const MARGIN: f32 = 8.0;

    fn lay_out(text: &str, rows_wanted: u32) -> Layout {
        let font = font::font().expect("the bundled font");

        layout(&font, text, PLATE, MARGIN, rows_wanted).expect("a layout")
    }

    #[test]
    fn a_four_character_label_is_laid_out_in_two_rows_of_two() {
        // This is the shape the reference implementation hard-codes, and the reason
        // this component searches for the row count instead: on a square plate two
        // rows of two give characters about 1.6 times the size of one row of four,
        // because one row is limited by the plate's width and two by its height.
        let layout = lay_out("AB12", 0);

        assert_eq!(layout.rows.len(), 2);
        assert_eq!(layout.rows[0].text, "AB");
        assert_eq!(layout.rows[1].text, "12");
    }

    #[test]
    fn two_rows_really_are_larger_than_one_here() {
        // The claim above, measured rather than asserted by construction.
        let auto = lay_out("AB12", 0);
        let one_row = lay_out("AB12", 1);

        assert!(
            auto.font_size > one_row.font_size,
            "the automatic layout chose {} pixels against {} for one row",
            auto.font_size,
            one_row.font_size
        );
    }

    #[test]
    fn a_single_character_is_not_split() {
        let layout = lay_out("7", 0);

        assert_eq!(layout.rows.len(), 1);
        assert_eq!(layout.rows[0].text, "7");
    }

    #[test]
    fn a_long_label_gets_more_rows_than_a_short_one() {
        // The search is what keeps a long label from shrinking to an unreadable
        // size on one line.
        let short = lay_out("AB12", 0);
        let long = lay_out("ORDER-2026-0001", 0);

        assert!(
            long.rows.len() > short.rows.len(),
            "a 15 character label was laid out in {} rows, the same as a 4 character one",
            long.rows.len()
        );
    }

    #[test]
    fn an_explicit_line_break_is_honoured() {
        // A caller who broke the lines themselves meant it, including when they
        // asked for a shape the search would not have chosen.
        let layout = lay_out("AB\n1234", 0);

        assert_eq!(layout.rows.len(), 2);
        assert_eq!(layout.rows[0].text, "AB");
        assert_eq!(layout.rows[1].text, "1234");
    }

    #[test]
    fn an_explicit_line_break_beats_the_row_option() {
        let layout = lay_out("A\nB\nC", 1);

        assert_eq!(layout.rows.len(), 3);
    }

    #[test]
    fn the_row_option_is_honoured_exactly() {
        for rows in 1..=4 {
            let layout = lay_out("ORDER-2026-0001", rows);

            assert_eq!(layout.rows.len(), rows as usize, "asked for {rows} rows");
        }
    }

    #[test]
    fn the_text_fills_the_plate_without_touching_the_edges() {
        // The point of the search is that the characters are as large as they can
        // be, so at least one dimension has to come out at exactly the available
        // space — and neither may come out larger.
        for text in ["7", "AB12", "ORDER-2026-0001", "Заказ №42"] {
            let layout = lay_out(text, 0);
            let available = PLATE - 2.0 * MARGIN;
            let width = layout
                .rows
                .iter()
                .map(|row| row.ink.width() * layout.font_size)
                .fold(0.0_f32, f32::max);
            let height = layout.height();

            assert!(
                width <= available + 0.01 && height <= available + 0.01,
                "{text:?} laid out {}x{} in {available}",
                width,
                height
            );
            assert!(
                width.max(height) > available * 0.999,
                "{text:?} laid out {}x{}, which leaves {} unused",
                width,
                height,
                available - width.max(height)
            );
        }
    }

    #[test]
    fn the_rows_are_separated_and_the_text_stays_inside_the_margin() {
        // Rendered rather than computed, because the arithmetic above is the thing
        // under test: what matters is that the pixels come out in two bands with a
        // gap between them, inside the margin the caller asked for.
        let (image, layout) = rendered("AB12", 0);

        assert_eq!(
            layout.rows.len(),
            2,
            "a four character label wants two rows"
        );

        let inked = inked_rows(&image);

        assert!(
            inked[0] as f32 >= MARGIN - 1.0,
            "the text starts at y={}, inside the {} pixel margin",
            inked[0],
            MARGIN
        );
        assert!(
            *inked.last().expect("ink") as f32 <= PLATE - MARGIN + 1.0,
            "the text ends at y={}, past the margin",
            inked.last().expect("ink")
        );

        let gaps = inked
            .windows(2)
            .filter(|pair| pair[1] > pair[0] + 1)
            .count();

        assert_eq!(gaps, 1, "the two rows have to be separated by clear space");
    }

    #[test]
    fn the_text_is_as_tall_as_the_plate_allows() {
        // The search picks the largest size that fits, so for a label that is
        // limited by height the ink has to fill the available square. A layout that
        // came out smaller would mean the search had settled for less.
        let (image, layout) = rendered("AB12", 0);
        let inked = inked_rows(&image);
        let span = (inked.last().expect("ink") - inked[0]) as f32;
        let available = PLATE - 2.0 * MARGIN;

        assert!(
            span >= available - 3.0,
            "the text spans {span} pixels of {available} available, so it is not as large as it \
             could be (chosen size: {})",
            layout.font_size
        );
    }

    #[test]
    fn the_text_is_centred_in_the_plate() {
        // Optically centred, not arithmetically: the ink is centred, so a row of
        // digits sits in the middle rather than slightly above it. Measured on the
        // pixels, and allowed a couple of pixels because the rasteriser rounds its
        // bounds outwards.
        let (image, _) = rendered("AB12", 0);
        let columns = (0..image.width())
            .filter(|x| (0..image.height()).any(|y| image.get_pixel(*x, y)[0] < 128))
            .collect::<Vec<u32>>();
        let left = *columns.first().expect("ink") as f32;
        let right = *columns.last().expect("ink") as f32;

        assert!(
            (left + right - (PLATE - 1.0)).abs() <= 3.0,
            "the ink runs from {left} to {right} in a {PLATE} pixel plate, which is not centred"
        );
    }

    #[test]
    fn cyrillic_is_split_by_characters_not_by_bytes() {
        // Splitting by bytes would cut a two-byte character in half and render a
        // replacement glyph; splitting by characters keeps the text whole. The
        // remainder of an odd count goes to the earlier rows.
        let layout = lay_out("Заказ", 2);

        assert_eq!(layout.rows.len(), 2);
        assert_eq!(
            layout
                .rows
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<&str>>(),
            vec!["Зак", "аз"]
        );
    }

    /// Render a label onto a white plate of the default size.
    fn rendered(text: &str, rows_wanted: u32) -> (RgbaImage, Layout) {
        let font = font::font().expect("the bundled font");
        let layout = layout(&font, text, PLATE, MARGIN, rows_wanted).expect("a layout");
        let mut image = RgbaImage::from_pixel(
            PLATE as u32,
            PLATE as u32,
            Color::rgb(0xff, 0xff, 0xff).pixel(),
        );

        super::draw(
            &mut image,
            &font,
            &layout,
            0.0,
            0.0,
            PLATE,
            Color::rgb(0x00, 0x00, 0x00),
        );

        (image, layout)
    }

    /// The rows of the picture that hold any ink at all.
    fn inked_rows(image: &RgbaImage) -> Vec<u32> {
        (0..image.height())
            .filter(|y| (0..image.width()).any(|x| image.get_pixel(x, *y)[0] < 128))
            .collect()
    }

    #[test]
    fn a_plate_with_no_room_is_refused_rather_than_laid_out_anyway() {
        let font = font::font().expect("the bundled font");

        assert!(
            layout(&font, "AB12", 20.0, 10.0, 0).is_none(),
            "no room at all"
        );
        assert!(
            layout(&font, "AB12", PLATE, PLATE, 0).is_none(),
            "the margin eats the plate"
        );
        assert!(
            layout(&font, "", PLATE, MARGIN, 0).is_none(),
            "nothing to draw"
        );
        assert!(
            layout(&font, "   ", PLATE, MARGIN, 0).is_none(),
            "nothing to draw"
        );
    }
}

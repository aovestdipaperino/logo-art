use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use std::fmt::Write;

/// Converts image data (PNG, etc.) to a string of ANSI escape codes that render
/// the image in the terminal using Unicode half-block characters (`▄`/`▀`) and
/// 24-bit true color sequences.
///
/// Each output character cell encodes two vertical pixels: one via the background
/// color and one via the foreground color of a half-block character.
///
/// # Arguments
/// * `image_data` — Raw image bytes (e.g. from `include_bytes!` or `std::fs::read`)
/// * `width` — Desired output width in terminal columns. Height is derived
///   proportionally from the source image's aspect ratio.
pub fn image_to_ansi(image_data: &[u8], width: u32) -> String {
    let img = image::load_from_memory(image_data).expect("Failed to decode image");
    let (orig_w, orig_h) = img.dimensions();
    let height = ((orig_h as f64 * width as f64) / orig_w as f64).round() as u32;
    let img = img.resize_exact(width, height, FilterType::Lanczos3);
    render(&img)
}

/// Convenience wrapper: converts and prints the image directly to stdout.
pub fn print_image(image_data: &[u8], width: u32) {
    print!("{}", image_to_ansi(image_data, width));
}

/// How finely a single terminal cell is subdivided.
///
/// A cell still carries only two colors (its foreground and its background), but
/// the glyph inside it can split that cell into more than the two samples a
/// half-block gives. More samples per cell means the same picture needs fewer
/// cells: because a terminal cell is about twice as tall as it is wide,
/// [`Cell::Octant`] makes the sample grid square again and renders an image at
/// half the width *and* half the height of [`Cell::HalfBlock`] with an identical
/// sample count.
///
/// The trade-off is font coverage, not fidelity: [`Cell::Quadrant`] and
/// [`Cell::Braille`] are drawable essentially everywhere, sextants need a font
/// from the last few years, and octants are Unicode 16 (2024) — perfect in
/// terminals that draw the legacy-computing blocks themselves (Kitty, Ghostty),
/// tofu in a terminal that leaves it to the font. Use [`Cell::best_supported`]
/// to pick from the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    /// `▀`/`▄`, 1x2 samples per cell. The classic, universally supported.
    #[default]
    HalfBlock,
    /// `▘▝▖▗▌▐▚▞…`, 2x2 samples per cell (U+2596..U+259F).
    Quadrant,
    /// 2x3 samples per cell (U+1FB00.., Symbols for Legacy Computing).
    Sextant,
    /// 2x4 samples per cell (U+1CD00.., Symbols for Legacy Computing Supplement).
    Octant,
    /// Braille dots, 2x4 samples per cell (U+2800..). Same geometry as
    /// [`Cell::Octant`] with universal font coverage, but the dots are thin, so
    /// the cell reads as its background tinted by the foreground.
    Braille,
}

impl Cell {
    /// Samples per cell, as `(horizontal, vertical)`.
    #[must_use]
    pub const fn grid(self) -> (u32, u32) {
        match self {
            Cell::HalfBlock => (1, 2),
            Cell::Quadrant => (2, 2),
            Cell::Sextant => (2, 3),
            Cell::Octant => (2, 4),
            Cell::Braille => (2, 4),
        }
    }

    /// Total samples per cell.
    #[must_use]
    pub const fn samples(self) -> u32 {
        let (x, y) = self.grid();
        x * y
    }

    /// The finest subdivision the current terminal is known to draw correctly.
    ///
    /// Only terminals that rasterize the legacy-computing blocks themselves are
    /// given the modern glyphs; everything else falls back to [`Cell::Quadrant`],
    /// which every font in practice covers.
    #[must_use]
    pub fn best_supported() -> Self {
        Self::for_term(
            std::env::var("TERM_PROGRAM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
        )
    }

    /// [`Cell::best_supported`] with the environment passed in, for testing.
    #[must_use]
    pub fn for_term(term_program: Option<&str>, term: Option<&str>) -> Self {
        let id = term_program.unwrap_or_default().to_ascii_lowercase();
        let term = term.unwrap_or_default().to_ascii_lowercase();
        // Kitty and Ghostty draw the block-element ranges from their own
        // geometry, so octants are exact regardless of the font.
        if id.contains("kitty") || id.contains("ghostty") || term.contains("kitty") {
            Cell::Octant
        } else if id.contains("wezterm") {
            // WezTerm draws sextants natively; octant coverage is newer.
            Cell::Sextant
        } else {
            Cell::Quadrant
        }
    }

    /// The glyph covering `mask`, whose bit `i` marks sample `i` as foreground
    /// (samples counted row-major from the cell's top-left).
    fn glyph(self, mask: u32) -> char {
        let mask = mask as usize;
        match self {
            Cell::HalfBlock => [' ', '\u{2580}', '\u{2584}', '\u{2588}'][mask & 0b11],
            Cell::Quadrant => QUADRANTS[mask & 0b1111],
            Cell::Sextant => SEXTANTS[mask & 0b11_1111],
            Cell::Octant => OCTANTS[mask & 0xFF],
            Cell::Braille => braille(mask as u8),
        }
    }
}

/// Converts image data to ANSI art, subdividing each terminal cell as `cell`
/// describes.
///
/// `width` is the output width in terminal columns and the height follows the
/// source aspect ratio, exactly as in [`image_to_ansi`] — a finer `cell` buys
/// resolution inside the same footprint rather than changing the footprint. To
/// trade that resolution back for a smaller banner, ask for fewer columns:
/// [`Cell::Octant`] at half the width matches [`Cell::HalfBlock`]'s detail.
///
/// [`Cell::HalfBlock`] delegates to [`image_to_ansi`] and is byte-identical to it.
pub fn image_to_ansi_with(image_data: &[u8], width: u32, cell: Cell) -> String {
    if cell == Cell::HalfBlock {
        return image_to_ansi(image_data, width);
    }
    let img = image::load_from_memory(image_data).expect("Failed to decode image");
    let (orig_w, orig_h) = img.dimensions();
    let width = width.max(1);
    // A cell is ~2x as tall as it is wide, so the row count follows the aspect
    // ratio alone; only the sample grid inside those cells depends on `cell`.
    let rows = (((orig_h as f64 * width as f64) / orig_w as f64) / 2.0).round() as u32;
    let rows = rows.max(1);
    let (sx, sy) = cell.grid();
    let img = img.resize_exact(width * sx, rows * sy, FilterType::Lanczos3);
    render_cells(&img, cell, width, rows)
}

/// Convenience wrapper: converts and prints the image directly to stdout.
pub fn print_image_with(image_data: &[u8], width: u32, cell: Cell) {
    print!("{}", image_to_ansi_with(image_data, width, cell));
}

/// Squared distance between two RGB samples.
fn dist2(a: [u8; 4], b: [u8; 4]) -> i32 {
    (0..3)
        .map(|i| {
            let d = i32::from(a[i]) - i32::from(b[i]);
            d * d
        })
        .sum()
}

/// Perceptual-ish brightness, used only to order two candidate cluster seeds.
fn luma(p: [u8; 4]) -> i32 {
    i32::from(p[0]) * 299 + i32::from(p[1]) * 587 + i32::from(p[2]) * 114
}

/// Mean color of the samples selected by `pick`, or `None` if none are.
fn mean(samples: &[[u8; 4]], pick: impl Fn(usize) -> bool) -> Option<[u8; 4]> {
    let (mut sum, mut n) = ([0u32; 3], 0u32);
    for (i, s) in samples.iter().enumerate() {
        if pick(i) {
            for c in 0..3 {
                sum[c] += u32::from(s[c]);
            }
            n += 1;
        }
    }
    if n == 0 {
        return None;
    }
    Some([
        (sum[0] / n) as u8,
        (sum[1] / n) as u8,
        (sum[2] / n) as u8,
        255,
    ])
}

/// Renders one cell's worth of samples as `(mask, foreground, background)`.
///
/// With more than two samples per cell the two available colors have to be
/// chosen: transparency always claims the background (so the terminal shows
/// through, as in the half-block path), and otherwise the samples are split into
/// a dark and a light cluster and each side is averaged.
fn cell_colors(samples: &[[u8; 4]]) -> (u32, Option<[u8; 4]>, Option<[u8; 4]>) {
    let visible: Vec<usize> = (0..samples.len())
        .filter(|&i| !is_transparent(samples[i][3]))
        .collect();
    if visible.is_empty() {
        return (0, None, None);
    }
    if visible.len() < samples.len() {
        // Mixed with transparency: the background must stay transparent, which
        // leaves a single color for everything visible.
        let mut mask = 0u32;
        for &i in &visible {
            mask |= 1 << i;
        }
        return (
            mask,
            mean(samples, |i| !is_transparent(samples[i][3])),
            None,
        );
    }
    // Fully opaque: two-means on the darkest and brightest samples as seeds.
    let lo = *visible.iter().min_by_key(|&&i| luma(samples[i])).unwrap();
    let hi = *visible.iter().max_by_key(|&&i| luma(samples[i])).unwrap();
    let (mut seed_lo, mut seed_hi) = (samples[lo], samples[hi]);
    let mut mask = 0u32;
    for _ in 0..2 {
        mask = 0;
        for (i, s) in samples.iter().enumerate() {
            if dist2(*s, seed_hi) <= dist2(*s, seed_lo) {
                mask |= 1 << i;
            }
        }
        let fg = mean(samples, |i| mask >> i & 1 == 1);
        let bg = mean(samples, |i| mask >> i & 1 == 0);
        match (fg, bg) {
            (Some(f), Some(b)) => (seed_hi, seed_lo) = (f, b),
            // One cluster swallowed everything: the cell is a flat color.
            _ => return (0, None, Some(seed_hi)),
        }
    }
    let fg = mean(samples, |i| mask >> i & 1 == 1);
    let bg = mean(samples, |i| mask >> i & 1 == 0);
    (mask, fg, bg)
}

/// Core rendering loop for the sub-cell modes: gathers each cell's samples,
/// picks its two colors, and emits the glyph that covers the chosen samples.
fn render_cells(img: &DynamicImage, cell: Cell, cols: u32, rows: u32) -> String {
    let (sx, sy) = cell.grid();
    let mut samples = Vec::with_capacity(cell.samples() as usize);
    let mut out = String::new();

    for cy in 0..rows {
        for cx in 0..cols {
            samples.clear();
            for j in 0..sy {
                for i in 0..sx {
                    samples.push(img.get_pixel(cx * sx + i, cy * sy + j).0);
                }
            }
            let (mask, fg, bg) = cell_colors(&samples);
            let fg_seq = fg.map_or_else(|| "39".to_string(), |c| ansi_fg(c[0], c[1], c[2], c[3]));
            let bg_seq = bg.map_or_else(|| "49".to_string(), |c| ansi_bg(c[0], c[1], c[2], c[3]));
            let _ = write!(out, "\x1b[{bg_seq};{fg_seq}m{}", cell.glyph(mask));
        }
        out.push_str("\x1b[m\n");
    }
    out
}

/// Braille dot for `mask`: dots 1-3 and 7 run down the left column, 4-6 and 8
/// down the right, so the row-major mask has to be permuted.
fn braille(mask: u8) -> char {
    const DOTS: [u8; 8] = [0x01, 0x08, 0x02, 0x10, 0x04, 0x20, 0x40, 0x80];
    let mut bits = 0u32;
    for (i, dot) in DOTS.iter().enumerate() {
        if mask >> i & 1 == 1 {
            bits |= u32::from(*dot);
        }
    }
    char::from_u32(0x2800 + bits).unwrap_or(' ')
}

/// Glyph for each 2x2 coverage mask (bit `i` = sample `i`, row-major).
const QUADRANTS: [char; 16] = [
    '\u{20}', '\u{2598}', '\u{259D}', '\u{2580}', '\u{2596}', '\u{258C}', '\u{259E}', '\u{259B}',
    '\u{2597}', '\u{259A}', '\u{2590}', '\u{259C}', '\u{2584}', '\u{2599}', '\u{259F}', '\u{2588}',
];

/// Glyph for each of the 64 sub-cell coverage masks (bit `i` = sample `i`,
/// row-major from the top-left). Derived from `UnicodeData.txt`: masks the
/// dedicated Unicode block covers use it, the rest fall back to the older
/// half/quarter block characters that draw the same shape.
const SEXTANTS: [char; 64] = [
    '\u{20}',
    '\u{1FB00}',
    '\u{1FB01}',
    '\u{1FB02}',
    '\u{1FB03}',
    '\u{1FB04}',
    '\u{1FB05}',
    '\u{1FB06}',
    '\u{1FB07}',
    '\u{1FB08}',
    '\u{1FB09}',
    '\u{1FB0A}',
    '\u{1FB0B}',
    '\u{1FB0C}',
    '\u{1FB0D}',
    '\u{1FB0E}',
    '\u{1FB0F}',
    '\u{1FB10}',
    '\u{1FB11}',
    '\u{1FB12}',
    '\u{1FB13}',
    '\u{258C}',
    '\u{1FB14}',
    '\u{1FB15}',
    '\u{1FB16}',
    '\u{1FB17}',
    '\u{1FB18}',
    '\u{1FB19}',
    '\u{1FB1A}',
    '\u{1FB1B}',
    '\u{1FB1C}',
    '\u{1FB1D}',
    '\u{1FB1E}',
    '\u{1FB1F}',
    '\u{1FB20}',
    '\u{1FB21}',
    '\u{1FB22}',
    '\u{1FB23}',
    '\u{1FB24}',
    '\u{1FB25}',
    '\u{1FB26}',
    '\u{1FB27}',
    '\u{2590}',
    '\u{1FB28}',
    '\u{1FB29}',
    '\u{1FB2A}',
    '\u{1FB2B}',
    '\u{1FB2C}',
    '\u{1FB2D}',
    '\u{1FB2E}',
    '\u{1FB2F}',
    '\u{1FB30}',
    '\u{1FB31}',
    '\u{1FB32}',
    '\u{1FB33}',
    '\u{1FB34}',
    '\u{1FB35}',
    '\u{1FB36}',
    '\u{1FB37}',
    '\u{1FB38}',
    '\u{1FB39}',
    '\u{1FB3A}',
    '\u{1FB3B}',
    '\u{2588}',
];

/// Glyph for each of the 256 sub-cell coverage masks (bit `i` = sample `i`,
/// row-major from the top-left). Derived from `UnicodeData.txt`: masks the
/// dedicated Unicode block covers use it, the rest fall back to the older
/// half/quarter block characters that draw the same shape.
const OCTANTS: [char; 256] = [
    '\u{20}',
    '\u{1CEA8}',
    '\u{1CEAB}',
    '\u{1FB82}',
    '\u{1CD00}',
    '\u{2598}',
    '\u{1CD01}',
    '\u{1CD02}',
    '\u{1CD03}',
    '\u{1CD04}',
    '\u{259D}',
    '\u{1CD05}',
    '\u{1CD06}',
    '\u{1CD07}',
    '\u{1CD08}',
    '\u{2580}',
    '\u{1CD09}',
    '\u{1CD0A}',
    '\u{1CD0B}',
    '\u{1CD0C}',
    '\u{1FBE6}',
    '\u{1CD0D}',
    '\u{1CD0E}',
    '\u{1CD0F}',
    '\u{1CD10}',
    '\u{1CD11}',
    '\u{1CD12}',
    '\u{1CD13}',
    '\u{1CD14}',
    '\u{1CD15}',
    '\u{1CD16}',
    '\u{1CD17}',
    '\u{1CD18}',
    '\u{1CD19}',
    '\u{1CD1A}',
    '\u{1CD1B}',
    '\u{1CD1C}',
    '\u{1CD1D}',
    '\u{1CD1E}',
    '\u{1CD1F}',
    '\u{1FBE7}',
    '\u{1CD20}',
    '\u{1CD21}',
    '\u{1CD22}',
    '\u{1CD23}',
    '\u{1CD24}',
    '\u{1CD25}',
    '\u{1CD26}',
    '\u{1CD27}',
    '\u{1CD28}',
    '\u{1CD29}',
    '\u{1CD2A}',
    '\u{1CD2B}',
    '\u{1CD2C}',
    '\u{1CD2D}',
    '\u{1CD2E}',
    '\u{1CD2F}',
    '\u{1CD30}',
    '\u{1CD31}',
    '\u{1CD32}',
    '\u{1CD33}',
    '\u{1CD34}',
    '\u{1CD35}',
    '\u{1FB85}',
    '\u{1CEA3}',
    '\u{1CD36}',
    '\u{1CD37}',
    '\u{1CD38}',
    '\u{1CD39}',
    '\u{1CD3A}',
    '\u{1CD3B}',
    '\u{1CD3C}',
    '\u{1CD3D}',
    '\u{1CD3E}',
    '\u{1CD3F}',
    '\u{1CD40}',
    '\u{1CD41}',
    '\u{1CD42}',
    '\u{1CD43}',
    '\u{1CD44}',
    '\u{2596}',
    '\u{1CD45}',
    '\u{1CD46}',
    '\u{1CD47}',
    '\u{1CD48}',
    '\u{258C}',
    '\u{1CD49}',
    '\u{1CD4A}',
    '\u{1CD4B}',
    '\u{1CD4C}',
    '\u{259E}',
    '\u{1CD4D}',
    '\u{1CD4E}',
    '\u{1CD4F}',
    '\u{1CD50}',
    '\u{259B}',
    '\u{1CD51}',
    '\u{1CD52}',
    '\u{1CD53}',
    '\u{1CD54}',
    '\u{1CD55}',
    '\u{1CD56}',
    '\u{1CD57}',
    '\u{1CD58}',
    '\u{1CD59}',
    '\u{1CD5A}',
    '\u{1CD5B}',
    '\u{1CD5C}',
    '\u{1CD5D}',
    '\u{1CD5E}',
    '\u{1CD5F}',
    '\u{1CD60}',
    '\u{1CD61}',
    '\u{1CD62}',
    '\u{1CD63}',
    '\u{1CD64}',
    '\u{1CD65}',
    '\u{1CD66}',
    '\u{1CD67}',
    '\u{1CD68}',
    '\u{1CD69}',
    '\u{1CD6A}',
    '\u{1CD6B}',
    '\u{1CD6C}',
    '\u{1CD6D}',
    '\u{1CD6E}',
    '\u{1CD6F}',
    '\u{1CD70}',
    '\u{1CEA0}',
    '\u{1CD71}',
    '\u{1CD72}',
    '\u{1CD73}',
    '\u{1CD74}',
    '\u{1CD75}',
    '\u{1CD76}',
    '\u{1CD77}',
    '\u{1CD78}',
    '\u{1CD79}',
    '\u{1CD7A}',
    '\u{1CD7B}',
    '\u{1CD7C}',
    '\u{1CD7D}',
    '\u{1CD7E}',
    '\u{1CD7F}',
    '\u{1CD80}',
    '\u{1CD81}',
    '\u{1CD82}',
    '\u{1CD83}',
    '\u{1CD84}',
    '\u{1CD85}',
    '\u{1CD86}',
    '\u{1CD87}',
    '\u{1CD88}',
    '\u{1CD89}',
    '\u{1CD8A}',
    '\u{1CD8B}',
    '\u{1CD8C}',
    '\u{1CD8D}',
    '\u{1CD8E}',
    '\u{1CD8F}',
    '\u{2597}',
    '\u{1CD90}',
    '\u{1CD91}',
    '\u{1CD92}',
    '\u{1CD93}',
    '\u{259A}',
    '\u{1CD94}',
    '\u{1CD95}',
    '\u{1CD96}',
    '\u{1CD97}',
    '\u{2590}',
    '\u{1CD98}',
    '\u{1CD99}',
    '\u{1CD9A}',
    '\u{1CD9B}',
    '\u{259C}',
    '\u{1CD9C}',
    '\u{1CD9D}',
    '\u{1CD9E}',
    '\u{1CD9F}',
    '\u{1CDA0}',
    '\u{1CDA1}',
    '\u{1CDA2}',
    '\u{1CDA3}',
    '\u{1CDA4}',
    '\u{1CDA5}',
    '\u{1CDA6}',
    '\u{1CDA7}',
    '\u{1CDA8}',
    '\u{1CDA9}',
    '\u{1CDAA}',
    '\u{1CDAB}',
    '\u{2582}',
    '\u{1CDAC}',
    '\u{1CDAD}',
    '\u{1CDAE}',
    '\u{1CDAF}',
    '\u{1CDB0}',
    '\u{1CDB1}',
    '\u{1CDB2}',
    '\u{1CDB3}',
    '\u{1CDB4}',
    '\u{1CDB5}',
    '\u{1CDB6}',
    '\u{1CDB7}',
    '\u{1CDB8}',
    '\u{1CDB9}',
    '\u{1CDBA}',
    '\u{1CDBB}',
    '\u{1CDBC}',
    '\u{1CDBD}',
    '\u{1CDBE}',
    '\u{1CDBF}',
    '\u{1CDC0}',
    '\u{1CDC1}',
    '\u{1CDC2}',
    '\u{1CDC3}',
    '\u{1CDC4}',
    '\u{1CDC5}',
    '\u{1CDC6}',
    '\u{1CDC7}',
    '\u{1CDC8}',
    '\u{1CDC9}',
    '\u{1CDCA}',
    '\u{1CDCB}',
    '\u{1CDCC}',
    '\u{1CDCD}',
    '\u{1CDCE}',
    '\u{1CDCF}',
    '\u{1CDD0}',
    '\u{1CDD1}',
    '\u{1CDD2}',
    '\u{1CDD3}',
    '\u{1CDD4}',
    '\u{1CDD5}',
    '\u{1CDD6}',
    '\u{1CDD7}',
    '\u{1CDD8}',
    '\u{1CDD9}',
    '\u{1CDDA}',
    '\u{2584}',
    '\u{1CDDB}',
    '\u{1CDDC}',
    '\u{1CDDD}',
    '\u{1CDDE}',
    '\u{2599}',
    '\u{1CDDF}',
    '\u{1CDE0}',
    '\u{1CDE1}',
    '\u{1CDE2}',
    '\u{259F}',
    '\u{1CDE3}',
    '\u{2586}',
    '\u{1CDE4}',
    '\u{1CDE5}',
    '\u{2588}',
];

/// Alpha below this threshold is treated as fully transparent (matches the
/// reference JS implementation which uses `a < 13`).
#[inline]
fn is_transparent(a: u8) -> bool {
    a < 13
}

/// Format an RGB(A) value as a foreground ANSI parameter string.
/// Returns `"39"` (default fg) for transparent pixels.
fn ansi_fg(r: u8, g: u8, b: u8, a: u8) -> String {
    if is_transparent(a) {
        "39".into()
    } else {
        format!("38;2;{r};{g};{b}")
    }
}

/// Format an RGB(A) value as a background ANSI parameter string.
/// Returns `"49"` (default bg) for transparent pixels.
fn ansi_bg(r: u8, g: u8, b: u8, a: u8) -> String {
    if is_transparent(a) {
        "49".into()
    } else {
        format!("48;2;{r};{g};{b}")
    }
}

/// Core rendering loop. Iterates pixel rows in pairs (top/bottom) and emits
/// the appropriate half-block character with combined fg+bg escape sequences.
fn render(img: &DynamicImage) -> String {
    let (width, height) = img.dimensions();
    let mut out = String::new();

    let mut y = 0u32;
    while y < height {
        for x in 0..width {
            let [tr, tg, tb, ta] = img.get_pixel(x, y).0;
            let (br, bg, bb, ba) = if y + 1 < height {
                let p = img.get_pixel(x, y + 1).0;
                (p[0], p[1], p[2], p[3])
            } else {
                (0, 0, 0, 0) // treat out-of-bounds as transparent
            };

            let top_t = is_transparent(ta);
            let bot_t = is_transparent(ba);

            if (tr == br && tg == bg && tb == bb && !top_t && !bot_t) || (top_t && bot_t) {
                // Both pixels same color, or both transparent → space with bg
                let _ = write!(out, "\x1b[{}m ", ansi_bg(tr, tg, tb, ta));
            } else if bot_t && !top_t {
                // Top visible, bottom transparent → ▀ (upper half block)
                // fg = top color, bg = bottom (default)
                let _ = write!(
                    out,
                    "\x1b[{};{}m▀",
                    ansi_bg(br, bg, bb, ba),
                    ansi_fg(tr, tg, tb, ta)
                );
            } else {
                // General case → ▄ (lower half block)
                // fg = bottom color, bg = top color
                let _ = write!(
                    out,
                    "\x1b[{};{}m▄",
                    ansi_fg(br, bg, bb, ba),
                    ansi_bg(tr, tg, tb, ta)
                );
            }
        }
        out.push_str("\x1b[m\n");
        y += 2;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG of `w` x `h` pixels, painted by `f`.
    fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut img = image::RgbaImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            px.0 = f(x, y);
        }
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    #[test]
    fn every_mask_maps_to_a_distinct_glyph() {
        for (name, table) in [
            ("quadrants", &QUADRANTS[..]),
            ("sextants", &SEXTANTS[..]),
            ("octants", &OCTANTS[..]),
        ] {
            let mut seen = table.to_vec();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), table.len(), "{name} has a duplicate glyph");
        }
        let mut braille: Vec<char> = (0..=255u8).map(braille).collect();
        braille.sort_unstable();
        braille.dedup();
        assert_eq!(braille.len(), 256);
    }

    // The shapes shared with the older block-element characters have to be
    // spelled with those characters, since the dedicated ranges omit them.
    #[test]
    fn legacy_shapes_use_legacy_characters() {
        assert_eq!(Cell::Octant.glyph(0), ' ');
        assert_eq!(Cell::Octant.glyph(0xFF), '█');
        assert_eq!(Cell::Octant.glyph(0b0000_1111), '▀'); // top two rows
        assert_eq!(Cell::Octant.glyph(0b1111_0000), '▄');
        assert_eq!(Cell::Octant.glyph(0b0101_0101), '▌'); // left column
        assert_eq!(Cell::Octant.glyph(0b1010_1010), '▐');
        assert_eq!(Cell::Sextant.glyph(0b010101), '▌');
        assert_eq!(Cell::Sextant.glyph(0b101010), '▐');
        assert_eq!(Cell::Sextant.glyph(0b111111), '█');
        assert_eq!(Cell::Quadrant.glyph(0b0011), '▀');
        assert_eq!(Cell::Quadrant.glyph(0b0101), '▌');
        assert_eq!(Cell::HalfBlock.glyph(0b01), '▀');
        assert_eq!(Cell::HalfBlock.glyph(0b10), '▄');
    }

    // The in-block glyphs are assigned in mask order, which is what makes the
    // tables reproducible from UnicodeData.txt.
    #[test]
    fn dedicated_ranges_ascend_with_the_mask() {
        for (table, first, last) in [
            (&SEXTANTS[..], '\u{1FB00}', '\u{1FB3B}'),
            (&OCTANTS[..], '\u{1CD00}', '\u{1CDE5}'),
        ] {
            let in_block: Vec<char> = table
                .iter()
                .copied()
                .filter(|c| (first..=last).contains(c))
                .collect();
            assert_eq!(in_block.first(), Some(&first));
            assert_eq!(in_block.last(), Some(&last));
            assert!(in_block.windows(2).all(|w| w[1] as u32 == w[0] as u32 + 1));
        }
    }

    #[test]
    fn braille_dots_follow_the_unicode_layout() {
        assert_eq!(braille(0), '\u{2800}');
        assert_eq!(braille(0b0000_0001), '\u{2801}'); // top-left -> dot 1
        assert_eq!(braille(0b0000_0010), '\u{2808}'); // top-right -> dot 4
        assert_eq!(braille(0b0100_0000), '\u{2840}'); // bottom-left -> dot 7
        assert_eq!(braille(0xFF), '\u{28FF}');
    }

    #[test]
    fn half_block_mode_is_the_original_renderer() {
        let data = png(8, 16, |x, y| if (x + y) % 3 == 0 { BLACK } else { WHITE });
        assert_eq!(
            image_to_ansi_with(&data, 8, Cell::HalfBlock),
            image_to_ansi(&data, 8)
        );
    }

    // The footprint follows the aspect ratio alone: a finer cell buys resolution
    // inside the same rows and columns, which is what lets a caller shrink the
    // art by asking for fewer columns.
    #[test]
    fn row_count_is_independent_of_the_cell() {
        let data = png(100, 200, |_, _| BLACK);
        for cell in [Cell::Quadrant, Cell::Sextant, Cell::Octant, Cell::Braille] {
            let art = image_to_ansi_with(&data, 18, cell);
            assert_eq!(art.lines().count(), 18, "{cell:?}"); // 200/100 * 18 / 2
        }
        // Half the width, half the height, same sample count as a half-block
        // render at twice the columns.
        let small = image_to_ansi_with(&data, 18, Cell::Octant);
        let large = image_to_ansi(&data, 36);
        assert_eq!(small.lines().count() * 2, large.lines().count());
    }

    /// Reverses `Cell::glyph`, so a render can be compared against its source.
    fn mask_of(cell: Cell, ch: char) -> u32 {
        let table: Vec<char> = match cell {
            Cell::Quadrant => QUADRANTS.to_vec(),
            Cell::Sextant => SEXTANTS.to_vec(),
            Cell::Octant => OCTANTS.to_vec(),
            Cell::Braille => (0..=255u8).map(braille).collect(),
            Cell::HalfBlock => vec![' ', '▀', '▄', '█'],
        };
        table.iter().position(|c| *c == ch).unwrap() as u32
    }

    /// The samples each cell of `art` claims as foreground, as a bitmap.
    fn decode(art: &str, cell: Cell, cols: u32) -> Vec<Vec<bool>> {
        let (sx, sy) = cell.grid();
        let mut rows = Vec::new();
        for line in art.lines() {
            let glyphs: Vec<char> = strip_ansi(line);
            assert_eq!(glyphs.len(), cols as usize);
            let mut band = vec![vec![false; (cols * sx) as usize]; sy as usize];
            for (cx, ch) in glyphs.iter().enumerate() {
                let mask = mask_of(cell, *ch);
                for j in 0..sy {
                    for i in 0..sx {
                        band[j as usize][cx * sx as usize + i as usize] =
                            mask >> (j * sx + i) & 1 == 1;
                    }
                }
            }
            rows.extend(band);
        }
        rows
    }

    fn strip_ansi(line: &str) -> Vec<char> {
        let mut out = Vec::new();
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    // End to end: a two-color image is exactly what two colors per cell can
    // carry, so the sub-cell modes must reproduce it sample for sample.
    #[test]
    fn two_color_art_round_trips_through_the_glyphs() {
        let cols = 8;
        for cell in [Cell::Quadrant, Cell::Sextant, Cell::Octant, Cell::Braille] {
            let (sx, sy) = cell.grid();
            // Width is exact, so the stripes land on sample boundaries; the
            // vertical resize cannot disturb a column-only pattern.
            let data = png(
                cols * sx,
                cols * sy,
                |x, _| if x % 2 == 0 { WHITE } else { BLACK },
            );
            let art = image_to_ansi_with(&data, cols, cell);
            let bits = decode(&art, cell, cols);
            assert_eq!(bits.len(), art.lines().count() * sy as usize, "{cell:?}");
            for (y, row) in bits.iter().enumerate() {
                for (x, on) in row.iter().enumerate() {
                    assert_eq!(*on, x % 2 == 0, "{cell:?} sample ({x},{y})");
                }
            }
        }
    }

    // Transparency keeps the background, exactly as in the half-block path: a
    // fully transparent cell is a bare space and a mixed cell paints only its
    // visible samples.
    #[test]
    fn transparency_shows_the_terminal_through() {
        let cell = Cell::Octant;
        let (sx, sy) = cell.grid();
        let cols = 4;
        let data = png(cols * sx, cols * sy, |x, y| {
            if y < sy {
                CLEAR
            } else if x % 2 == 0 {
                WHITE
            } else {
                CLEAR
            }
        });
        let art = image_to_ansi_with(&data, cols, cell);
        let first = art.lines().next().unwrap();
        assert!(strip_ansi(first).iter().all(|c| *c == ' '));
        assert!(
            first.contains("49;39m"),
            "fully clear row keeps both defaults"
        );
        let second = art.lines().nth(1).unwrap();
        assert!(
            second.contains("[49;38;2;"),
            "mixed cell keeps the default bg"
        );
        for row in decode(&art, cell, cols).iter().skip(sy as usize) {
            for (x, on) in row.iter().enumerate() {
                assert_eq!(*on, x % 2 == 0);
            }
        }
    }

    #[test]
    fn best_supported_only_trusts_terminals_that_draw_the_blocks() {
        assert_eq!(Cell::for_term(Some("ghostty"), None), Cell::Octant);
        assert_eq!(Cell::for_term(None, Some("xterm-kitty")), Cell::Octant);
        assert_eq!(Cell::for_term(Some("WezTerm"), None), Cell::Sextant);
        assert_eq!(Cell::for_term(Some("Apple_Terminal"), None), Cell::Quadrant);
        assert_eq!(Cell::for_term(Some("iTerm.app"), None), Cell::Quadrant);
        assert_eq!(Cell::for_term(None, None), Cell::Quadrant);
    }

    #[test]
    fn grids_match_their_sample_counts() {
        for (cell, samples) in [
            (Cell::HalfBlock, 2),
            (Cell::Quadrant, 4),
            (Cell::Sextant, 6),
            (Cell::Octant, 8),
            (Cell::Braille, 8),
        ] {
            assert_eq!(cell.samples(), samples, "{cell:?}");
        }
    }
}

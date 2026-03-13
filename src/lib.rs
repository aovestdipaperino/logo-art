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

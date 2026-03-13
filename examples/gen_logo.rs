use image::{Rgba, RgbaImage};

const SIZE: u32 = 256;
const BLOCKS: u32 = 8;
const BLOCK_PX: u32 = SIZE / BLOCKS;
const GAP: u32 = 3;
const CORNER_RADIUS: f64 = 36.0;

fn main() {
    let mut img = RgbaImage::new(SIZE, SIZE);
    let center = SIZE as f64 / 2.0;
    let half = center - 1.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = rounded_rect_sdf(x as f64 + 0.5, y as f64 + 0.5, center, center, half, half);

            if d > 0.5 {
                continue;
            }

            // Gap between blocks (transparent grid lines)
            let lx = x % BLOCK_PX;
            let ly = y % BLOCK_PX;
            if lx < GAP || ly < GAP {
                // Gaps inside the shape stay transparent — gives the mosaic look
                continue;
            }

            let bx = x / BLOCK_PX;
            let by = y / BLOCK_PX;

            // Diagonal rainbow: top-left red → bottom-right purple
            let t = (bx as f64 + by as f64) / (2.0 * (BLOCKS - 1) as f64);
            let hue = t * 300.0;

            // Subtle per-tile vignette: brighter center, slightly darker edges
            let bcx = BLOCK_PX as f64 / 2.0;
            let dx = (lx as f64 - bcx) / bcx;
            let dy = (ly as f64 - bcx) / bcx;
            let vignette = 1.0 - (dx * dx + dy * dy).sqrt() * 0.12;

            let (r, g, b) = hsv_to_rgb(hue, 0.80, 0.95 * vignette);

            // Anti-alias the outer rounded-rect edge
            let alpha = if d < -0.5 { 255 } else { ((0.5 - d) * 255.0) as u8 };

            img.put_pixel(x, y, Rgba([r, g, b, alpha]));
        }
    }

    std::fs::create_dir_all("assets").expect("Failed to create assets/");
    img.save("assets/logo.png").expect("Failed to save logo");
    println!("Saved assets/logo.png ({SIZE}x{SIZE}, {BLOCKS}x{BLOCKS} mosaic)");
}

/// Signed distance to a rounded rectangle centered at (cx, cy) with half-extents
/// (hw, hh) and corner radius CORNER_RADIUS. Negative = inside.
fn rounded_rect_sdf(px: f64, py: f64, cx: f64, cy: f64, hw: f64, hh: f64) -> f64 {
    let dx = (px - cx).abs() - hw + CORNER_RADIUS;
    let dy = (py - cy).abs() - hh + CORNER_RADIUS;
    dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - CORNER_RADIUS
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (((r + m) * 255.0) as u8,
     ((g + m) * 255.0) as u8,
     ((b + m) * 255.0) as u8)
}

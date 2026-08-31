use logo_art::Cell;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <image_path> [width] [cell]", args[0]);
        eprintln!("  image_path  Path to a PNG image");
        eprintln!("  width       Output width in columns (default: 80)");
        eprintln!(
            "  cell        half | quad | sextant | octant | braille (default: auto-detected)"
        );
        std::process::exit(1);
    }

    let image_data = std::fs::read(&args[1]).expect("Failed to read image file");
    let width: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(80);
    let cell = match args.get(3).map(String::as_str) {
        Some("half") => Cell::HalfBlock,
        Some("quad") => Cell::Quadrant,
        Some("sextant") => Cell::Sextant,
        Some("octant") => Cell::Octant,
        Some("braille") => Cell::Braille,
        Some(other) => {
            eprintln!("Unknown cell mode: {other}");
            std::process::exit(1);
        }
        None => Cell::best_supported(),
    };

    logo_art::print_image_with(&image_data, width, cell);
}

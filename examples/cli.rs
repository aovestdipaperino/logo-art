use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <image_path> [width]", args[0]);
        eprintln!("  image_path  Path to a PNG image");
        eprintln!("  width       Output width in columns (default: 80)");
        std::process::exit(1);
    }

    let image_data = std::fs::read(&args[1]).expect("Failed to read image file");
    let width: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(80);

    logo_art::print_image(&image_data, width);
}

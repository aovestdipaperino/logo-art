# logo-art

Convert images to Unicode/True Color ANSI art for terminal display.

Uses half-block characters (`▄`/`▀`) to pack two pixel rows per terminal line with 24-bit true color escape sequences. Algorithm ported from [dom111/image-to-ansi](https://github.com/dom111/image-to-ansi).

## Usage

```rust
use logo_art::{image_to_ansi, print_image};

// Embed a PNG at compile time
const LOGO: &[u8] = include_bytes!("logo.png");

fn main() {
    // Print directly (width = 40 columns, height proportional)
    print_image(LOGO, 40);

    // Or get the ANSI string
    let ansi = image_to_ansi(LOGO, 80);
    print!("{ansi}");
}
```

## API

- **`image_to_ansi(image_data: &[u8], width: u32) -> String`** — Convert image bytes to an ANSI escape code string. `width` sets the output width in terminal columns; height is derived proportionally.
- **`print_image(image_data: &[u8], width: u32)`** — Convenience wrapper that prints directly to stdout.

## Example CLI

```sh
cargo run --example cli -- path/to/image.png 60
```

## Features

- True color (24-bit) ANSI output
- Transparency support (renders transparent pixels with default terminal colors)
- Proportional scaling from a single width parameter
- Minimal dependencies (`image` crate with PNG feature only)

## License

MIT

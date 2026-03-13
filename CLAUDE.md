# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

logo-art is a Rust library crate (edition 2024) that converts images to Unicode/True Color ANSI art for terminal display. It uses half-block characters (`▄`/`▀`) to pack two pixel rows per terminal line, with 24-bit true color escape sequences. The algorithm is ported from [dom111/image-to-ansi](https://github.com/dom111/image-to-ansi).

## Build & Run Commands

- **Build:** `cargo build`
- **Test:** `cargo test`
- **Single test:** `cargo test <test_name>`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Check (fast compile check):** `cargo check`
- **Run CLI example:** `cargo run --example cli -- <image_path> [width]`
- **Regenerate logo:** `cargo run --example gen_logo`

## Architecture

- `src/lib.rs` — Library crate. Public API: `image_to_ansi(image_data, width) -> String` and `print_image(image_data, width)`. Takes raw image bytes and a target column width; height is derived proportionally.
- `examples/cli.rs` — Example CLI binary that reads a PNG from disk and prints ANSI art.
- `examples/gen_logo.rs` — Generates `assets/logo.png`, a 256x256 rainbow mosaic with transparency used as the crate logo and demo image.
- `assets/logo.png` — Embedded demo image (8x8 colored tile mosaic in a rounded rectangle with transparent corners).

## Key Design Details

- Transparency threshold: alpha < 13 is treated as fully transparent (matching the JS reference).
- Pixel pairing: rows are iterated in pairs (top/bottom). When both colors match → space with bg. When bottom is transparent → `▀` with fg=top. Otherwise → `▄` with fg=bottom, bg=top.
- Only the `png` feature of the `image` crate is enabled (minimal dependency footprint).

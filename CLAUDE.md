# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

logo-art is a Rust library crate (edition 2024) that converts images to Unicode/True Color ANSI art for terminal display. It uses half-block characters (`▄`/`▀`) to pack two pixel rows per terminal line, with 24-bit true color escape sequences. The algorithm is ported from [dom111/image-to-ansi](https://github.com/dom111/image-to-ansi). Since 0.3 a cell can also be subdivided into quadrants, sextants, octants or Braille dots, which fits the same picture into fewer cells.

## Build & Run Commands

- **Build:** `cargo build`
- **Test:** `cargo test`
- **Single test:** `cargo test <test_name>`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Check (fast compile check):** `cargo check`
- **Run CLI example:** `cargo run --example cli -- <image_path> [width] [half|quad|sextant|octant|braille]`
- **Regenerate logo:** `cargo run --example gen_logo`

## Architecture

- `src/lib.rs` — Library crate. Public API: `image_to_ansi(image_data, width) -> String`, `print_image(image_data, width)`, their `_with(.., Cell)` variants, and the `Cell` enum. Takes raw image bytes and a target column width; height is derived proportionally.
- `examples/cli.rs` — Example CLI binary that reads a PNG from disk and prints ANSI art.
- `examples/gen_logo.rs` — Generates `assets/logo.png`, a 256x256 rainbow mosaic with transparency used as the crate logo and demo image.
- `assets/logo.png` — Embedded demo image (8x8 colored tile mosaic in a rounded rectangle with transparent corners).

## Key Design Details

- Transparency threshold: alpha < 13 is treated as fully transparent (matching the JS reference).
- Pixel pairing: rows are iterated in pairs (top/bottom). When both colors match → space with bg. When bottom is transparent → `▀` with fg=top. Otherwise → `▄` with fg=bottom, bg=top.
- Only the `png` feature of the `image` crate is enabled (minimal dependency footprint).
- Sub-cell modes (`Cell::Quadrant` and finer) go through `render_cells`, not `render`; `Cell::HalfBlock` delegates to the original `render` so its output stays byte-identical.
- Row count depends only on the aspect ratio, never on the cell: a finer cell buys resolution inside the same footprint. `Cell::Octant`'s 2x4 grid is square given a 2:1 cell, so it renders at half the width *and* half the height of a half-block render with the same sample count.
- The `SEXTANTS`/`OCTANTS` tables are indexed by coverage mask (bit `i` = sample `i`, row-major) and are derived from `UnicodeData.txt`. The dedicated Unicode ranges deliberately omit the masks that older characters already draw (empty, full, halves, quadrants, quarter blocks, and `U+1FBE6`/`U+1FBE7` for the middle-half masks), so those slots hold the legacy character instead. `dedicated_ranges_ascend_with_the_mask` guards the ordering.
- Two colors per cell is the hard constraint. With more than two samples the cell's colors are chosen by a two-means split on brightness; transparency always claims the background so the terminal shows through.

# P38: Mosaic and deep-zoom pyramid core

- Status: Done
- Pull request: [#51](https://github.com/lucaswewa/microscope/pull/51)
- ADRs: [ADR-0030](../../adr/0030-position-based-stitching-with-dzi-output.md)
- Spec: [phases.md#p38](../phases.md#p38)

## Summary

Stitching's core, in `microscope-core`.

- **Placement:** scan images are placed where the stage was, through the camera–stage mapping.
- **Blending:** they're blended where they overlap.
- **Output:** a preview at a reduced scale, or a deep-zoom pyramid as the gallery's viewer reads it, written a row of tiles at a time so memory doesn't grow with the mosaic's area.

P39 uses it for scans: a live preview while one runs, then the final mosaic.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/mosaic.rs` | `Layout`: each tile's place in the mosaic from the stage positions, through the mapping's inverse, scaled to the tiles' size; the tiles overlapping a rectangle; `preview`. The blending, and `MosaicError` |
| `crates/microscope-core/src/dzi.rs` | `write`: the DZI pyramid, its `.dzi` file and its tiles, given to a function the caller supplies, with the overview and thumbnail. `top_level`, `level_size` and `jpeg` |
| `crates/microscope-core/examples/mosaic.rs` | A made-up scan of 80 large tiles, written as a DZI to a folder, and timed |
| `crates/microscope-core/tests/mosaic.rs` | The tests |
| `crates/microscope-core/Cargo.toml` | `image`, the crate's first dependency, for images and JPEG |
| `docs/adr/0030-…` | ADR-0030 |

## How to try it

```powershell
cargo run -p microscope-core --release --example mosaic -- target/mosaic
```

1. **It prints** "11540 × 6972 pixels from 80 tiles, 15 levels, in about 8 s, holding at most 20 tiles".
2. **`target/mosaic/`** has:
   - `mosaic.dzi`, which says the size, 256-pixel JPEG tiles and no overlap;
   - `mosaic_files/0` to `mosaic_files/14`, a folder of tiles for each level;
   - `overview.jpeg`, the rings and gradient the tiles were drawn from, with no seams;
   - `thumbnail.jpeg`.
3. **More tiles:** add the columns and rows, such as `-- target/mosaic 20 20`. The tiles held at once grow with the columns, not the rows.

## Design notes and deviations from the plan

- **No I/O in `microscope-core`** (ADR-0005). The caller gives a function that reads a tile by its index, and one that writes each file. P39's will read a scan's images and write into its folder.
  - **Errors** say which tile couldn't be read, or why a file couldn't be written, in the caller's words.
- **Placement** is relative to the first tile, then shifted so the mosaic starts at the leftmost and topmost.
  - **Larger captures:** the mapping is calibrated on preview frames, so a full-size capture's placement is scaled by its width over theirs.
  - **Rounding:** tiles land on whole pixels.
- **Blending:** each tile's weight is the product of two ramps, rising from 1 at its edges by 1 a pixel. A seam between two tiles fades across their overlap.
- **Memory:**
  - **The most detailed level** is drawn a row of 256-pixel tiles at a time.
    - **Which images:** only those overlapping the row, each read when first needed and dropped once the rows have passed it.
    - **Blending,** a tile at a time, to keep its sums small.
  - **Each level** holds at most one row, waiting for the next, before both are halved into the level below.
  - **The overview,** the largest level that fits 2048 pixels, is kept whole.
  - **In the example,** 80 tiles of 1640 × 1232 in 10 columns: at most 20 held, two rows' worth.
- **The DZI's layout** is the viewer's fixture's (P33), as its notes asked: `.jpeg` tiles of 256 pixels with no overlap, and the same descriptor.
- **The overview and thumbnail** are images in the result, for P39 to save where the scan's folder needs them. `jpeg` encodes them as the tiles are.
- **Checked on a real simulated scan:** P37's histo scan of the tissue sample, 30 full-size images.
  - **How:** read with their sidecars' stage positions and mapping, in a throwaway test that isn't committed.
  - **The result:** a 5903 × 6038 mosaic in 14 levels, holding at most 10 images.
  - **At full size,** fibres run on across the seams, and no nucleus is doubled.
- **Size:** 830 changed lines of hand-written code, within L's 900, and above my estimate of about 630.

  | Part | Lines |
  |---|---|
  | `mosaic.rs` | 225 |
  | `dzi.rs`, with its unit tests | 273 |
  | The example | 79 |
  | Tests | 248 |
  | `Cargo.toml` and `lib.rs` | 5 |

## Tests

- **`tests/mosaic.rs`** (7 tests). Tiles are cut from a known picture, at stage positions through a quarter-turned mapping of two steps a pixel, captured at twice the calibrated size.
  - **Placement:** each tile's origin and the mosaic's size, also with the tiles in reverse order; refusals for no tiles and a mapping that can't be inverted.
  - **A preview** at full size is the picture, to within 1. Shrunk to 210 pixels, it's the picture shrunk, to within 3 on average.
  - **Seams fade:** two flat tiles, 100 and 200, blend monotonically across their overlap. Apart, the gap between them is white.
  - **A pyramid:**
    - every tile asked for once;
    - the 11 levels, each tile the right size, and no other files;
    - the descriptor;
    - its most detailed level, decoded and put back together, is the picture within JPEG's error, under 2 on average;
    - its overview and thumbnail.
  - **A tall mosaic,** 30 tiles in a column: at most 4 held, and an overview from the level below the top, since the top is too tall.
  - **Levels:** for 600 × 400, as the viewer's fixture has them, and at powers of two.
  - **Errors:** a tile of the wrong size, a tile that can't be read, and a file that can't be written.
- **Unit tests in `dzi.rs`:** halving averages two by two, and the edges alone; stacking.

**Mutation checks:** I broke 17 behaviours, and a test failed for each.

- **Placement:** not inverting the mapping, not scaling to the tiles, not shifting to the mosaic's corner, and not checking a tile's size.
- **Blending:** flat weights, and a black background.
- **The preview:** not shrinking.
- **The pyramid:**
  - **memory:** tiles read again, or never dropped;
  - **the levels:** one too many, the smallest overview, the last row never written, halving by the nearest pixel, or rows stacked on top of each other;
  - **the files:** the descriptor saying overlap, an unshrunk thumbnail, and a low JPEG quality.

At first "not shifting" survived, since every test listed the top-left tile first. The tiles in reverse order catch it.

## Follow-ups and known gaps

- **P39** stitches scans with it: a live preview while one runs, the final pyramid after, and the gallery's deep-zoom view of them.
- **Correlation refinement** of the placement waits for milestone 2 (ADR-0030).
- **Placement is to the whole pixel.** Subpixel placement would need resampling every tile.
- **A tile is drawn from every image that overlaps it,** however many. A scan with much overlap blends more images a tile, and takes longer.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.

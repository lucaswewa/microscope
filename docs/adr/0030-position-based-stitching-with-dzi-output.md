# ADR-0030: Position-based stitching with DZI output

- Status: Accepted
- Date: 2026-10-14
- Phase: P38

## Context

- **A scan's images** each record where the stage was (ADR-0025), and the camera–stage mapping (P23) turns a stage move into a move of the image in pixels. That's enough to place each image in a mosaic.
- **Plan D2:** place tiles by stage position. Refining the placement by image correlation waits for milestone 2, when a real stage drifts and the simulator's doesn't.
- **The gallery's viewer** (ADR-0027) shows deep-zoom images: a `.dzi` file, with its tiles at `<name>_files/<level>/<column>_<row>.<format>`. Its fixture, which P33's notes ask P38 to follow, has 256-pixel tiles without overlap.
- **Mosaics can be large.** A 20 × 20 scan of 1640 × 1232 images is about 26,000 × 20,000 pixels: 1.5 GB as RGB, too much to hold at once.
- **`microscope-core`** is free of I/O (ADR-0005), so it can be tested quickly and reused with real hardware.

## Decision

- **Placement:** each image goes where the stage was, through the inverse of the mapping's image-to-stage matrix, relative to the first.
  - **Larger captures:** images larger than the frames the mapping was calibrated on are scaled to match.
  - **Rounding:** each lands on the nearest whole pixel.
  - **Origin:** the mosaic starts at the leftmost and topmost image.
- **Blending:** where images overlap, each pixel is their weighted mean.
  - **The weight** falls linearly from an image's middle to its edges, in x and in y. Seams fade rather than show.
  - **Where no image is,** the mosaic is white, like the empty slide.
- **The pyramid** is a DZI image, as the viewer reads it:
  - **Its files:** `<name>.dzi`, and the tiles at `<name>_files/<level>/<column>_<row>.jpeg`;
  - **its tiles:** 256 pixels square, without overlap, JPEG at quality 90;
  - **its levels:** each halves the one above, rounding up, down to one pixel at level 0.
- **Bounded memory:** the pyramid is written a row of tiles at a time.
  - **The most detailed level** is drawn a row at a time, from only the images that overlap the row. An image is read once, when it's first needed, and dropped once the rows have passed it.
  - **Each level's rows,** two at a time, are halved into the next level's row.
  - **So memory grows with the mosaic's width,** not its area.
- **The overview** is the largest level that fits 2048 pixels, kept whole as it's written. The thumbnail is the overview shrunk to fit 320 pixels.
- **A preview** draws the whole mosaic shrunk to a given size, for the live preview during a scan (P39). Each image is read once, shrunk, and blended in.
- **No I/O in `microscope-core`:** the caller gives a function that reads an image by its index, and one that writes each file. `microscope-core` now depends on `image`, for its images and its JPEG encoder.

## Consequences

- **Stitching is as good as the stage's reported positions and the mapping.** On the simulator they're exact enough that a stitched scan shows no doubled features.
- **The same code** writes a final mosaic (P39) and could write one from any folder of placed images.
- **A real stage** that loses steps will show seams. Milestone 2 refines the placement by correlation, with P22's registration, before the same blending and writing.
- **The tiles are JPEGs,** smaller than PNGs, at a small loss in each tile.

## Alternatives considered

- **Refine placement by correlation now:** it's slower, and the simulator's stage doesn't drift, so there would be nothing to correct (plan D2).
- **Hold the whole mosaic and cut the pyramid from it:** simple, but a large scan needs gigabytes.
- **Draw each level straight from the images:** each image would be read once for every level, about fifteen times.
- **Pyramidal TIFF, as OpenFlexure's `stitch_tiff`:** deferred (plan §2). DZI and an overview cover viewing and download.
- **Tiles with a pixel of overlap:** the viewer's fixture has none, and OpenSeadragon draws tiles without overlap well.
- **No blending, the last image on top:** simpler, but every seam shows a step in brightness or focus.

# P12: Correlation placement

- Status: Done
- Pull request: [#81](https://github.com/lucaswewa/microscope/pull/81)
- ADRs: [ADR-0046](../../adr/0046-correlation-stitching.md)
- Spec: [phases.md#p12](../phases.md#p12)

## Summary

Where each image of a mosaic really is, from its overlaps with its neighbours. Each overlapping pair is registered by phase correlation. Then one least-squares solve, weighted by each pair's confidence, places every image at once. The simulator's stage can now drift, losing steps and creeping, and can be switched to drift at run time, so CI can test the correction. P13 brings it to the stitch.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/placement.rs` | `PlacementOptions`, `Pair`, `Placement` (with `moves`), `place`, `pair` and `solve` |
| `crates/microscope-core/src/registration.rs` | `Gray::crop` |
| `crates/microscope-sim/src/stage.rs` | `StageConfig::lost_steps` and `creep`: the sample strays from the carriage |
| `crates/microscope-things/src/stage.rs` | `Drift`, and `drift` and `drift_size` in the simulated stage's `simulation` setting |
| `crates/microscope-sim/tests/placement.rs` | A drifting stage's scan, placed by correlation |
| `web/src/api/generated/` | The setting's new fields, regenerated |

### Placement

| Step | What |
|---|---|
| Pairs | Two images whose stage placements overlap by at least 5% of a side each way, and 32 pixels, are registered on that overlap. Then they're registered again on the overlap the first registration found, which takes out its shortfall on large shifts |
| Rejects | A pair less than 0.3 sure, or whose correction is over 50 pixels, isn't used |
| Solve | Least squares over the pairs used, weighted by confidence, with each image held to its stage placement at 1/10,000 of that weight. An image with no pair used stays where the stage put it |

`place` takes images in memory. `pair` and `solve` are the same steps one at a time, so P13 can read each image as it's needed.

### The drifting stage

| Setting | What | Default, when `drift` is on |
|---|---|---|
| `drift_size.lost_steps` | The share of each move's steps each axis loses | 0.5% in x and y |
| `drift_size.creep` | How fast the sample creeps, in steps a second | 0.2 in x, −0.1 in y |

Drift is off by default, and a `simulation` setting saved before P12 has none. Switching it off keeps how far the sample has strayed, as a real stage would. The simulated camera draws where the sample really is, and each capture's sidecar records it, in `simulation.true_position_um`.

## How to try it

1. **The tests:**

   ```powershell
   cargo test -p microscope-core placement
   cargo test -p microscope-sim --test placement --test stage
   cargo test -p microscope-things --test stage drifting
   ```

2. **A drifting stage in the app:** P12 doesn't change the stitch yet, so the drift shows as seams, which P13 removes.
   - Run the server and the web app as usual, and calibrate the camera–stage mapping.
   - Turn the drift on:

     ```powershell
     $url = 'http://127.0.0.1:5000/api/v1/stage/simulation'
     $simulation = Invoke-RestMethod $url
     $simulation.drift = $true
     Invoke-RestMethod $url -Method Put -ContentType 'application/json' -Body ($simulation | ConvertTo-Json -Depth 5)
     ```

   - **Scan** a 3 × 3 snake on the default sample, and open the stitched mosaic in the gallery.
   - **Zoom in where images overlap:** blobs there have doubled edges, a few pixels apart.
   - **Turn the drift off** (`$simulation.drift = $false`, and the same `Put`), and scan again. The seams are clean.

## Hardware

None: the phase has no hardware. It was tried on the simulator, on Windows.

## Design notes and deviations from the plan

- **Twice a pair:** phase correlation falls a few percent short on a large shift in a small square: 6.7 pixels for 7 in a 64-pixel square. Registering again on the overlap the first registration found makes the second shift small, so the shortfall goes. ADR-0046 has the reasons.
- **The corners count:** images at a corner of each other overlap too, and are registered like the rest.
- **A weak anchor:** each image is held to its stage placement at 1/10,000 of a sure pair's weight. Much stronger, and a slow drift across a large scan would be held back. Weaker, and nothing would pin a group to where it is.
- **The textured test scene:** my first synthetic scene was soft spots with no fine detail. Phase correlation weighs every frequency alike, so in small overlaps it missed by up to 3 pixels, even in 128-pixel squares. Stained tissue, and the simulator's blobs with their noise, have detail at many scales. The tests' scene now does too, with smooth noise at three scales. The simulator test uses the default sample.
- **The simulator test scans at 20×:** at 40× the default sample has about three blobs in an overlap, which is a scan of mostly empty slide.
- **Lost steps are a share of each move,** not random, so a test knows exactly where the sample is. With the creep, the error still varies across a scan.
- **Size:** 622 changed lines (611 added, 11 removed), within L's 900. The estimate was about 630. `placement.rs` is 371 of them, with its tests.

## Tests

- **Placement** (`microscope-core`), on a 3 × 3 grid of 512 × 384 images overlapping by a quarter:
  - known shifts of up to 25 pixels are recovered within 0.3 pixels, from all 20 pairs;
  - featureless overlaps are all rejected, and every image keeps its stage placement;
  - with a 5-pixel limit, an image 12 pixels off has its pairs rejected and keeps its stage placement, while the rest are placed within 0.3 pixels;
  - images too far apart aren't paired;
  - the solve is exact for a chain of consistent pairs, and an image with no pair stays put.
- **The drifting stage** (`microscope-sim`):
  - it loses its share of a move's steps, part way through a move and at its end, and the sample creeps;
  - switched off, it keeps what it strayed and strays no further.
- **A drifting stage's scan** (`microscope-sim`): a 4 × 3 snake at 20× on the default sample, with 1% of the steps lost and a creep of a few steps a second. Where the stage says the images are is 15.3 pixels out. Correlation places them within 0.21, using 28 of the 29 pairs; the one rejected was 0.09 sure. The test asks for 1.5.
- **The simulated stage** (`microscope-things`): with the drift on, 10 of 1,000 steps are lost. A setting without the drift fields turns it off.
- **In the app**, on a server of my own: a 3 × 3 snake at the default settings, with the drift on. Its images were up to 23 pixels from where the stage said, and neighbours up to 16 pixels apart, and the stitched mosaic showed doubled blob edges at the seams. With the drift off, the seams were clean.

## Follow-ups and known gaps

- **P13** uses `pair` and `solve` in the final stitch, and records both placements in `tiles.json`. It depends on P10, which is skipped for now, so the owner chooses what comes next.
- **A second solve without the pairs that disagree with the first,** if real scans show confident wrong matches.
- **A sparse solver,** if scans grow to thousands of images.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.

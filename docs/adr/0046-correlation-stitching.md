# ADR-0046: Correlation stitching

- Status: Accepted
- Date: 2026-10-21
- Phase: M2 P12
- Supersedes: [ADR-0030](0030-position-based-stitching-with-dzi-output.md), in part: images placed by the stage's positions alone

## Context

- **ADR-0030 places each image where the stage said it was.** A real stage loses steps and drifts, so features at the seams show twice. The simulator's stage didn't drift, so nothing could test a correction.
- **P22's phase correlation** (ADR-0023):
  - finds how far one image's content moved in another, to a fraction of a pixel, with a confidence;
  - compares the centred power-of-two square of two images the same size, at most 256 pixels after halving;
  - finds a shift only if it's less than half of that square;
  - falls a few percent short on a large shift in a small square: 6.7 for 7 pixels in a 64-pixel square.
- **Overlaps are strips.** At the scans' default 35%, two 1640 × 1232 captures overlap by 574 × 1232 side by side and 1640 × 431 one above the other. Images at a corner overlap too.
- **Phase correlation weighs every frequency alike.** Content with detail at many scales, such as stained tissue or the simulator's blobs with their noise, registers well even in small overlaps. Smooth content with no fine detail registers poorly in them.
- **A scan has hundreds of images.** One of 20 × 20 has 400 images and about 1,500 overlapping pairs.

## Decision

- **`microscope_core::placement`:**
  - **Pairs:** two images whose stage placements overlap by at least 5% of a side each way, and 32 pixels, are registered on that overlap.
  - **Twice:** first on the overlap the stage gives, then on the overlap the first registration found. The second shift is small, so the first's shortfall goes. The pair's correction and confidence are the second's.
  - **Rejects:** a pair is used only if its confidence is at least 0.3 and its correction is at most 50 pixels.
  - **The solve:**
    - every image's position at once, by least squares, x and y separately;
    - each pair used gives an equation, `origin[b] − origin[a] = expected[b] − expected[a] + correction`, weighted by its confidence;
    - each image also gives `origin = expected`, weighted by 1/10,000;
    - the equations are solved through their normal equations, by a dense Cholesky factorisation.
  - **So:** an image with no pair used keeps its stage placement, and a group of images joined by pairs keeps its stage placement on average. The anchor is weak, so a slow drift across a large scan is corrected, not held back.
  - **Three functions:** `place` does it all for images in memory. `pair` and `solve` do it a step at a time, for images read as they're needed.
- **A drifting stage in the simulator:**
  - each move loses a share of its steps;
  - and the sample creeps, in steps per second.

  On the simulated stage, these are `drift` and `drift_size` in its `simulation` setting, switched at run time. Drift is off by default. A setting saved before P12 has no drift.
- **The final stitch uses it** from P13. The live preview keeps the stage's positions, for speed.

## Consequences

- **Neighbours up to 50 pixels apart** from where the stage put them are corrected. Further than that is more likely a wrong match, at the scans' overlaps.
- **An empty slide keeps the stage's positions.** A scan that's mostly empty slide is corrected only where there's a sample.
- **Only translations are corrected.** An error in the camera–stage mapping's rotation or scale is corrected only as far as moving each image whole can.
- **Each pair costs two registrations,** each of at most a 256-pixel square.
- **The dense solve grows with the square of the images' count** in memory, and with its cube in time. 400 images take 1.3 MB and a few milliseconds; 2,500 would take 50 MB and seconds. A sparse solver can replace it if scans grow that large.

## Alternatives considered

- **One registration a pair:** half the cost, but a few percent short on large shifts, which is pixels at the scans' drifts.
- **Registering the whole images, not their overlaps:** at 35% overlap, neighbours are 65% of a frame apart, beyond phase correlation's half a square. The rest of each image would also blur the match.
- **Each image against the one before:** simpler, but errors add up along the scan. The solve shares them out over every pair.
- **Dropping the pairs that disagree with a first solve, and solving again:** sturdier against a confident wrong match. The confidence and the 50-pixel limit reject those seen so far, so it waits for scans that need it.
- **Lost steps at random:** more like a real stage, but tests couldn't know the truth so simply. A share of each move, with the creep, still varies the error across a scan.

# ADR-0037: Autofocus accuracy: focus at the curve's centroid

- Status: Accepted
- Date: 2026-10-16
- Phase: P54
- Supersedes: [ADR-0022](0022-sharpness-metric-and-fast-autofocus.md), in where a sweep's focus is and what happens when there's none; its metric, monitor and sweeps stand

## Context

- **ADR-0022 takes the sharpest frame's z as focus.** Frames come about 50 steps (2.5 µm) apart in a sweep, so P21 expected errors up to about 1.25 µm.
- **P54's benchmark** (`tests/autofocus_bench.rs`) measured more than that, from many offsets, on the default sample, a sparse one and a tilted one, with backlash:
  - **on the default sample:** fast autofocus a mean of 1.48 µm out, but 5.8 µm at the 95th percentile and 8.2 µm at worst; looping much the same; the smart stack's kept image 1.94 µm on average and 8.15 at worst;
  - **on a tilted, wavy sample:** the same outliers, at the same places;
  - **on a sparse sample:** 11 µm on average and up to 37 µm, with no run reporting that it found nothing.
- **The outliers come from the metric's shape.** With the stage still, the preview's JPEG size near focus has a narrow peak, about ±1 µm wide, on a broad hump whose sides peak about 6.5 µm either side of focus. A sweep's frames can miss the narrow peak. The sharpest frame is then one of the sides, 6 µm out, at the same places every time.
- **Empty fields have no peak.** With nothing in view, a sweep's sharpness varied by 2.9% at most, which is noise. Autofocus then moved to wherever the noise was highest: up to 37 µm away.
- **Backlash isn't the cause:** with it on and off, the errors were alike once the outliers were understood.

## Decision

- **Focus is the curve's centroid:** the frames above half the curve's height, each weighted by how far above (`microscope_core::focus::peak`).
  - **The hump's sides are symmetric about focus,** so the centroid finds it whether or not a frame caught the narrow peak.
  - **It's between frames,** not on one, so it isn't limited by their spacing.
- **A curve that rises by less than 3% has nothing to focus on.** This is the threshold smart stacks already use (P36).
  - **Fast autofocus** goes back to where it began, approached from below as usual, and its curve has no focus. The Control tab says there's nothing to focus on.
  - **Looping autofocus** stops after that sweep, with "nothing to focus on", rather than sweeping ten times.
  - **A smart stack** that needs to refocus stops trying, as it did when looping autofocus failed.
- **A smart stack keeps the image nearest the curve's focus,** found the same way from its images, rather than the sharpest one. Its images are a stack step apart, so the kept one is within half a step of focus.
- **The benchmark stays in the repository,** ignored by default, to measure changes and, in milestone 2, real hardware.

## Consequences

- **More accurate,** in the same benchmark:

  | Method | Sample | Mean (µm) | 95th percentile | Worst |
  |---|---|---|---|---|
  | fast | default | 1.48 → 0.19 | 5.80 → 1.15 | 8.20 → 1.30 |
  | fast | tilted | 1.30 → 0.18 | 6.18 → 0.61 | 9.11 → 1.53 |
  | looping | default | 1.27 → 0.21 | 5.85 → 0.65 | 8.75 → 1.40 |
  | looping | tilted | 1.40 → 0.20 | 6.43 → 0.81 | 7.21 → 1.28 |
  | smart stack | default | 1.94 → 1.20 | 6.50 → 1.60 | 8.15 → 8.15 |

  Fast and looping autofocus now end within the objective's depth of field (about 1.5 µm) wherever there's something to focus on.
- **Sparse fields:** 7 or 8 of 20 now say there's nothing to focus on, and stay where they were. Two that found focus were wrong (by 6 and 23 µm): on one, sharpness is lowest at focus. A smart stack on a sparse field takes 16 s rather than 28, as its refocusing gives up after one sweep.
- **Honest about empty fields:** a scan whose start has nothing in view now fails at its first autofocus with "nothing to focus on", instead of scanning at a random height. Histo scans already refuse to start on background.
- **The same speed:** the same sweeps and moves; only the arithmetic on the curve changed.
- **The threshold is the simulator's, with a thin margin:** 3% sits between empty fields (up to 2.9%) and fields with anything in them (4.2% or more). A real camera's noise may need another, which the benchmark can find.
- **A smart stack's kept image is still up to half a stack step from focus.** Taking one more image at the curve's focus would close that, at the cost of an image. It isn't done.
- **A smart stack can still keep a side of the hump** when its window holds only one side: once in 20 runs, 8 µm out. Its window would need to reach past the hump on both sides, which is more images.

## Alternatives considered

- **A parabola through the sharpest frame and its neighbours:** better than the sharpest frame on average (0.45 µm against 0.64), but no help when the sharpest is one of the hump's sides.
- **A Gaussian through three frames:** much the same as the parabola.
- **A slower or finer second sweep:** `StageApi` has no speed, so a finer sweep would be stepped, taking a still frame at each step. That's seconds more per autofocus, for less than the centroid gains.
- **A gradient metric** (Tenengrad, Brenner) beside the JPEG size: it wouldn't help empty fields, which have nothing to measure, and the centroid already finds focus on the others. It would cost computing on every frame.
- **Moving to the sharpest frame on a flat curve, as before:** that leaves the stage at random, up to 37 µm away, and reports success.

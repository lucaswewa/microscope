# P54: Autofocus accuracy

- Status: Done
- Pull request: [#68](https://github.com/lucaswewa/microscope/pull/68)
- ADRs: [ADR-0037](../../adr/0037-autofocus-accuracy.md), superseding [ADR-0022](../../adr/0022-sharpness-metric-and-fast-autofocus.md) in part
- Spec: [phases.md#p54](../phases.md#p54)

## Summary

How accurately autofocus finds focus, measured, then made much better.

- **A benchmark** (`crates/microscope-things/tests/autofocus_bench.rs`), run on demand:
  - fast autofocus, looping autofocus and a scan's smart stack;
  - from 20 offsets each, on the default sample, a sparse one, and a tilted, wavy one, with backlash on;
  - the error measured from the simulator's true position and focal surface.
- **It found occasional 6–9 µm errors** on ordinary samples, where P21 expected about 1.25 µm at worst. **On empty fields, it found jumps of up to 46 µm** to wherever the noise was highest.
- **The cause:** near focus, the preview's sharpness has a narrow peak, about ±1 µm wide, on a broad hump with sides about 6.5 µm either side. A sweep's frames, 2.5 µm apart, can miss the peak, and the sharpest frame is then a side.
- **The fix** (ADR-0037), with the same sweeps:
  - **Focus is the curve's centroid,** above half its height, between frames.
  - **A curve that rises by less than 3% has nothing to focus on:**
    - fast autofocus goes back where it began, and the Control tab says so;
    - looping autofocus stops after that sweep, rather than ten.
  - **A smart stack keeps the image nearest its curve's centroid,** rather than its sharpest.
- **The result:** fast and looping autofocus now end a mean of 0.2 µm from focus, and 1.5 µm at worst, wherever there's something to focus on.

## The measurements

`cargo test --release -p microscope-things --test autofocus_bench -- --ignored --nocapture`, 20 runs for each method and sample. Each runs from up to 15 µm out (5 µm for stacks), at sites 200 µm apart, with backlash. Errors are in µm, over the runs that found focus; times are seconds a run.

**Before** (`main` at 7257f79):

| Method | Sample | No focus | Mean | 95th % | Worst | Time |
|---|---|---|---|---|---|---|
| fast | default | 0 | 1.48 | 5.80 | 8.20 | 6.2 |
| fast | sparse | 0 | 11.13 | 28.35 | 36.45 | 6.2 |
| fast | tilted | 0 | 1.30 | 6.18 | 9.11 | 6.2 |
| looping | default | 0 | 1.27 | 5.85 | 8.75 | 6.5 |
| looping | sparse | 0 | 8.94 | 21.75 | 45.70 | 8.0 |
| looping | tilted | 0 | 1.40 | 6.43 | 7.21 | 6.8 |
| smart stack | default | 0 | 1.94 | 6.50 | 8.15 | 6.9 |
| smart stack | sparse | 9 | 4.13 | 17.05 | 17.05 | 27.9 |
| smart stack | tilted | 0 | 1.68 | 6.52 | 8.14 | 6.9 |

**After:**

| Method | Sample | No focus | Mean | 95th % | Worst | Time |
|---|---|---|---|---|---|---|
| fast | default | 0 | 0.19 | 1.15 | 1.30 | 6.2 |
| fast | sparse | 7 | 2.43 | 23.40 | 23.40 | 6.2 |
| fast | tilted | 0 | 0.18 | 0.61 | 1.53 | 6.2 |
| looping | default | 0 | 0.21 | 0.65 | 1.40 | 6.8 |
| looping | sparse | 8 | 2.68 | 24.20 | 24.20 | 6.2 |
| looping | tilted | 0 | 0.20 | 0.81 | 1.28 | 6.8 |
| smart stack | default | 0 | 1.20 | 1.60 | 8.15 | 7.0 |
| smart stack | sparse | 9 | 3.93 | 21.55 | 21.55 | 16.0 |
| smart stack | tilted | 1 | 1.27 | 8.14 | 8.14 | 7.0 |

**How much one run varies:** the old code, run twice, gave fast autofocus on the default sample a mean of 1.48 µm, then 1.56.

## How it was found

1. **The benchmark,** with every run more than 2 µm out printed, showed the outliers at the same sites every time: (2000, 0), (6000, 2000) and (0, 8000). That was so on the default and the tilted sample, which share their fields.
2. **Recorded sweeps,** without backlash, showed their curves had two peaks there: the sharpest frames sat either side of focus, with a dip between.
3. **With the stage still,** stepping 10 steps at a time through focus at those sites, the preview's JPEG size has a peak only about 20 steps (1 µm) wide. It sits on a hump whose sides peak about 130 steps (6.5 µm) out, so a sweep's frames, 50 steps apart, can step over it.
4. **Ways to find the peak, compared offline** on 20 recorded sweeps of the default sample, without backlash:

   | Method | Mean (µm) | Worst (µm) |
   |---|---|---|
   | the sharpest frame | 0.64 | 1.18 |
   | a parabola through it and its neighbours | 0.45 | 0.85 |
   | a Gaussian, the same way | 0.44 | 0.85 |
   | the centroid above 70% | 0.22 | 0.51 |
   | **the centroid above 50%** | **0.13** | **0.32** |

   On the sweeps whose sharpest frame was a side of the hump, the centroid was 0.91 µm out at worst, against 6.25.
5. **Empty fields:** sweeps rose by 2.9% at most, and anything else by 4.2% or more. A 3% threshold, the one smart stacks already use (P36), tells them apart.

## The spec's candidates

| Candidate | Outcome |
|---|---|
| The focus between frames, from a fitted curve | **Adopted, as the centroid.** The parabola and the Gaussian fits did less well, and not at all when the sharpest frame was a side of the hump. |
| A short, slower second sweep | **Not adopted.** `StageApi` has no speed, so it would be a stepped search, seconds more per autofocus, and the centroid already reaches 0.2 µm. |
| Each frame's z, more precisely | **Not needed:** without backlash, the centroid is 0.13 µm out on average, so the frames' z adds little. |
| A gradient metric beside the JPEG size | **Not adopted:** it wouldn't help empty fields, and the JPEG size, read by its centroid, already finds focus elsewhere. |
| The stage's backlash and settling | **No change:** with and without backlash, the errors were alike: a mean of 0.58 µm and 0.64 by the sharpest frame, 0.14 and 0.13 by the centroid. |

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/focus.rs` | `peak`: the centroid above half height, or none if the curve rises by less than `MIN_RISE`, 3% |
| `crates/microscope-things/src/autofocus.rs` | The sweep's focus from `peak`, and back to the start without one; looping stops on an empty field; a smart stack keeps the image nearest its curve's centroid |
| `crates/microscope-things/tests/autofocus_bench.rs` | The benchmark |
| `crates/microscope-things/tests/autofocus.rs` | A test on an empty slide |
| `web/src/control/AutofocusControl.vue` | Says when there's nothing to focus on |

## How to try it

- **The benchmark:** `cargo test --release -p microscope-things --test autofocus_bench -- --ignored --nocapture --test-threads=1`. It takes about 35 minutes, and prints the tables above.
- **In the app:**
  1. In **Settings → Camera**, set the blob density to 0, for an empty slide. In **Control**, press **Autofocus**: after the sweep, the stage goes back where it was, and "Nothing to focus on here" shows.
  2. Set the density back to 1500, move z by a few hundred steps, and press **Autofocus**: the image is sharp.

## Design notes and deviations from the plan

- **The cause was the metric's shape, not the sampling alone.** With finer sampling the narrow peak would be caught. But the hump's symmetry makes the centroid right either way, without slowing the sweep.
- **Fast autofocus no longer moves on an empty field.** Before, it moved to the sharpest frame wherever it was, and reported success. A scan whose first autofocus sees nothing now fails with "nothing to focus on", rather than scanning at a random height.
- **The threshold's margin is thin** on the simulator (2.9% against 4.2%). A real camera will need its own, which the benchmark is there to find.
- **The benchmark's runs are fixed:** the same offsets and sites each time, so changes compare like with like. It's ignored by default, as it takes half an hour.
- **Two mistakes cost runs, and are kept out of the results:**
  - **A shared build folder:** I first ran the "after" benchmark from a build folder shared with a worktree of `main`, and cargo reused `main`'s test binary. That run is a second "before", which gave the variation above.
  - **Smart App Control,** turned on by a Windows update the day before, blocked the test binaries until the owner turned it off.
- **Size:** 507 changed lines of hand-written code, within L's 900. The benchmark is 257 of them, the other tests 118 (60 of them `focus.rs`'s unit tests), and the change itself about 130.

## Tests

- **`focus.rs`** (unit):
  - focus between samples;
  - focus where the samples miss the peak, on a curve shaped as measured;
  - nothing on a flat curve or none;
  - the same answer in any order.
- **`tests/autofocus.rs`:**
  - on an empty slide, fast autofocus finds no focus and goes back where it began, with backlash on;
  - looping autofocus says "nothing to focus on" after one sweep.

  The earlier tests still pass, within their 2 µm.
- **`tests/stack.rs`:** smart stacks on a tilted sample keep an image within 1.3 µm of focus, as before.
- **`ControlPage.spec.ts`:** the Control tab says when there's nothing to focus on.
- **Mutation:** without the 3% check, the flat-curve unit test and the empty-slide test both fail.
- **End to end:** the whole suite (130), whose workflows autofocus, passes on a server built from this branch, and so do the contract tests.
- **In a browser,** on my own server: on an empty slide, the Control tab said there was nothing to focus on and z stayed at 3. From 300 steps out on the default slide, autofocus ended at z 4, sharp.
- **The benchmark,** before and after, above.

## Follow-ups and known gaps

- **Sparse fields:** two runs in 20 found a focus that was wrong, by 6 and 23 µm. On one, sharpness is lowest at focus, which no peak-finding can correct. Real hardware may differ.
- **The smart stack** can still keep a side of the hump when its window holds only one side: once in 20, 8 µm out. A wider window would fix it, at more images per site.
- **A smart stack's kept image** is up to half a stack step (1.25 µm) from focus. One more image at the centroid would close that.
- **On real hardware** (milestone 2), the benchmark needs a way to know true focus, such as a manual focus or a calibration slide.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.

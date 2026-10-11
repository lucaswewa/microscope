# ADR-0023: Image registration by phase correlation

- Status: Accepted
- Date: 2026-10-10
- Phase: P22

## Context

- **What needs it:** several later features ask one question, "how far has the image moved between these two frames?":
  - camera–stage mapping (P23) moves the stage and measures how the image moves;
  - stage measurement (P30) does the same over longer travel;
  - correlation stitching (milestone 2) aligns overlapping tiles.
- **What the answer must be:**
  - accurate to a fraction of a pixel, since the mapping's error grows with each step it extrapolates;
  - quick enough to run between moves;
  - honest about when it can't tell, for example on an empty slide or a regular grating.
- **Where it lives:** `microscope-core` holds pure computation with no I/O (ADR-0005), and so far has no dependencies at all. ADR-0017 kept the simulator free of image-processing crates.

## Decision

- **Phase correlation**, in `microscope_core::registration::register(a, b, options)`. It returns the shift of `b`'s content from `a`'s, in pixels, with a confidence. The steps:
  1. **Shrink** both images by a whole factor, averaging blocks (`Options::downsample`, 2 by default), for speed.
  2. **Cut** the same centred power-of-two square from both, at most `Options::max_size` (256 by default).
  3. **Subtract** each square's mean, and **taper** it to zero at the edges with a Hann window, so the jump at the edges doesn't dominate.
  4. **Transform:** take the normalised cross-power spectrum of the two Fourier transforms, `conj(A)·B / |conj(A)·B|`. It keeps only the phase difference, so brightness and contrast don't matter.
  5. **Invert** it: a sharp peak sits at the shift.
- **Sub-pixel:** fit a parabola through the peak and its two neighbours on each axis. Over synthetic shifts it's within 0.05 px on average, and 0.09 at worst. A formula that assumes the unwindowed peak shape did worse (0.07 and 0.12), because the window changes the peak's shape.
- **Confidence:** `(p₁ − p₂) / p₁`, where p₁ is the peak and p₂ the highest point more than 2 pixels from it. It's about 1 for one clear peak. It falls towards 0 when another peak rises as high: a periodic pattern (whose shift is known only modulo its period), unrelated images, or an empty field.
- **Our own radix-2 FFT** (`microscope_core::fft`), about 70 lines, on power-of-two squares, so `microscope-core` still has no dependencies.
- **Images** are `Gray`: f32 brightness by rows, with `from_rgb8` for camera frames (Rec. 601 luminance) and `downsample`. The core doesn't depend on `image`.

## Consequences

- **Measured:**
  - **Speed:** two 820 × 616 frames at the default settings take about 9.5 ms in a release build.
  - **Accuracy:** on frames the simulator draws at 40×, the measured shift is within 0.2 px of the stage's move (0.02 µm), with confidence 0.8–0.9. An empty slide scores below 0.3.
- **The range** is less than half the compared square: ±128 shrunk pixels by default, which is ±256 camera pixels. Larger moves need a bigger square or less shrinking, and the images must still overlap well.
- **Shrinking:** each shrink factor trades precision for speed. At 4×, the error is a fraction of a shrunk pixel (up to 2 camera pixels).
- **Rotation and scale** aren't measured: phase correlation finds translations. Camera–stage mapping measures small moves, where the image barely rotates, and fits the rotation from several of them (P23).
- **Thresholds:** callers should check the confidence. P23 and P30 will set their own thresholds from it.

## Alternatives considered

- **`rustfft`,** which is fast and mixed-radix. It would be the core's first dependency, for a transform that only ever runs on power-of-two squares of up to 512. It can replace `fft` without changing the API, if speed ever matters.
- **Direct cross-correlation** (sums over every offset). It's far slower, and its peak is broad, so sub-pixel estimates are worse.
- **Feature matching** (corners and descriptors). It handles rotation and scale, but it's much more code, and it struggles on smooth, blob-like specimens. Mapping only needs translation.
- **Upsampled cross-correlation** (a refined DFT around the peak). It's more precise, but more code and time, and the parabola already gives 0.05 px.
- **Confidence as the peak's height alone.** A regular grating gives a tall peak at the wrong shift; the ratio to the next peak catches it.

# ADR-0031: Sequence model

- Status: Accepted
- Date: 2026-10-14
- Phase: P41

## Context

- **OpenFlexure's timelapse** captures every `dt` seconds for a `duration`, with an optional autofocus before each capture (`autofocus_enabled`, `autofocus_dz`).
- **Plan D1:** timelapse parity now, modelled so that z-stacks and multiple positions, which multi-dimensional acquisition (MDA) needs, can be added later without a rewrite.
- **Long sequences** run for hours. One slow step, such as an autofocus or a slow disk, shouldn't derail the schedule, and one failed autofocus shouldn't end the sequence.
- **The gallery** (ADR-0026) shows each kind of item through a provider. A sequence's images are viewed one at a time, as P33's sequence viewer does.

## Decision

- **A plan,** `microscope_core::sequence::SequencePlan`, pure and tested with a fake clock:
  - **A time axis:** an interval and a duration. The points are at 0, dt, 2·dt, …, within the duration, counted in whole nanoseconds, so tenths of a second add up exactly.
  - **Steps at each point,** in order: `Autofocus { dz }`, then `Capture`.
  - **Reserved for later:**
    - **`ZStack { images, dz }`,** a step;
    - **`positions`,** a list of stage positions visited at each point. Empty means where the stage is.

    A plan with either is refused until they're built.
- **The schedule:** `SequencePlan::due(elapsed, next)` says what to do next.
  - **Wait** until the next point is due.
  - **Run a point** within its own interval, even if it's a little late.
  - **Skip** the points whose whole interval has passed.
  - **Done** after the last point.

  A sequence never runs late points back to back to catch up, and it counts what it skipped.
- **The `sequence` Thing** (plan §11, point 2) has OpenFlexure's settings, `dt`, `duration`, `autofocus_enabled` and `autofocus_dz`.
  - **`run_sequence(name)`** runs a timelapse, one at a time.
  - **`download_zip(name)`** gives its images and record as a ZIP archive.
- **Capturing:** each capture is kept in memory and handed to a save queue, a task beside the schedule that writes each JPEG and its sidecar on a blocking thread.
  - **At most 4 wait.** A capture that finds the queue full waits for room, which bounds memory if the disk falls behind.
  - **The sequence ends** only once the queue is empty, and a cancelled sequence still saves what it captured.
- **Failures:** a failed autofocus is logged and the capture still taken. Any other failure ends the sequence.
- **On disk:** `sequences/<name>/` in the data folder (plan §5.9).
  - **The images:** `image_<k>.jpeg` for time point k, five digits, each with its sidecar (ADR-0025), so skipped points leave gaps.
  - **The record,** `sequence.json`: the plan, when it started, how long it ran, the images, the points skipped and the result.
  - **Names** follow a scan's rules, now shared in `data::new_folder`.
- **In the gallery:** a `Sequence` card type.
  - **Its card:** the directory viewer with its slider, the images, the interval and how long it ran, the scans' messages for how it ended, and its ZIP and record.
  - **Deleting** `gallery/sequences/{name}` is refused, with 409, for the sequence running.

## Consequences

- **A z-stack at each point** becomes one more step, run with P36's basic stack. Multiple positions become an axis around the steps. Neither changes the schedule or the record.
- **The schedule is tested** without waiting, with a fake clock, while the Thing's tests run short real sequences on the simulator.
- **A sequence holds the global lock** while it runs, as a scan does, so the stage and camera can't be moved under it.
- **The images in memory** are bounded by the queue: four full-size captures, about 24 MB.

## Alternatives considered

- **A loop of autofocus and capture, as OpenFlexure's timelapse is:** simplest, but z-stacks and positions would mean rewriting it (plan D1).
- **Running late points at once to catch up:** a slow step would bunch the next captures together, and the intervals would no longer mean what they say.
- **Saving each capture before the next point:** a slow disk would delay the schedule.
- **Numbering images by capture rather than by time point:** skipped points would vanish from the names, and the images' times would be harder to read.

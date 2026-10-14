# P41: Sequence engine (MDA-ready)

- Status: In review
- Pull request: [#54](https://github.com/lucaswewa/microscope/pull/54)
- ADRs: [ADR-0031](../../adr/0031-sequence-model.md)
- Spec: [phases.md#p41](../phases.md#p41)

## Summary

Timelapses, as OpenFlexure's: a capture every `dt` seconds for a `duration`, after an autofocus if asked.

- **The plan is a list of steps** at each time point, so z-stacks and multiple positions can join later.
- **A late point** runs within its own interval or is skipped, with a warning.
- **Captures** go to memory and are saved by a queue beside the schedule.
- **Sequences show in the gallery,** each with the sequence viewer's slider, its images, interval and duration, and a ZIP.

## Behaviour spec

Written before the code, in our own words, from what OpenFlexure's timelapse does.

### The plan

A sequence is a plan: a **time axis**, and the **steps** run at each of its time points.

1. **The time axis:**
   - **The time points:** an interval, dt, and a duration, both in seconds, give points at 0, dt, 2·dt, … for as long as they're within the duration.
   - **The count:** a duration of 60 s at 10 s gives 7 points, one at the start and one at the end. A duration shorter than dt gives one.
2. **The steps,** in order, at each time point:
   - **an autofocus,** if asked: the fast autofocus over a range, centred on where z is (P21);
   - **then a capture.**
3. **Reserved for later:** a z-stack as a step, and a list of positions to visit at each time point. A plan with either is refused, "z-stacks and multiple positions aren't supported yet".

### The `sequence` Thing

1. **Its settings,** with OpenFlexure's timelapse names:
   - **`dt`:** the interval, 30 s by default, from 0.5 s;
   - **`duration`:** 600 s by default, from 0;
   - **`autofocus_enabled`:** off;
   - **`autofocus_dz`:** 2000 steps, from 100.
2. **`run_sequence(name)`** runs the plan its settings make.
   - **Its folder:** `sequences/<name>/` in the data folder. The name follows a scan's rules: letters, digits, spaces, `-`, `_` and `.`, not starting with `.`. Without one, it's `sequence-<UTC time>`. A name that's taken gets `-2`, `-3`, …
   - **Its record,** `sequence.json`: the plan, when it started, how long it ran, the images, the time points skipped, and the result. It's written as it starts and again as it ends.
   - **It returns** the sequence's name.
3. **`download_zip(name)`:** the sequence's images, with their sidecars and its record, as a ZIP archive.

### Running a sequence

1. **Time is counted from the start.** Each time point is due at its time, and the sequence waits for it.
2. **A point runs within its own interval:**
   - **A little late,** while the next point isn't due yet, it runs at once.
   - **Later than that:** if the points before have taken so long that its whole interval has passed, it's skipped. The log says how many were skipped, and why: the steps took longer than the interval. A sequence never runs late points back to back to catch up.
3. **At each point,** the steps run in order.
   - **A failed autofocus** is logged, and the capture is still taken, so a long sequence doesn't stop for one bad focus.
   - **Any other failure** ends the sequence.
4. **Capturing:**
   - **Into memory:** each capture is kept in memory, and saved by a queue that works beside the schedule, so a slow disk doesn't delay the next point.
   - **A full queue:** at most 4 captures wait to be saved. If the disk falls that far behind, the next capture waits for room.
   - **Names:** each image is `image_<k>.jpeg`, k being its time point's number, five digits from 00000, with its sidecar (ADR-0025). A skipped point leaves a gap in the numbers.
5. **The end:** after the last point, once every capture is saved.
   - **Cancelled:** it stops at once, even while waiting for a point. The captures taken so far are still saved.
   - **The record's result:** `success`, `cancelled` or `error: …`.
6. **One at a time:** while one runs, another is refused.

### In the gallery

- **A Sequence card type** (ADR-0026) for the folders in `sequences/`:
  - **its thumbnail:** a small copy of the first image;
  - **View:** its images one at a time, with the sequence viewer's slider (P33);
  - **its info:** the images, the interval, and how long it ran;
  - **its messages:** a cancelled, failed or unfinished sequence says so, as a scan's card does;
  - **its downloads:** the images as a ZIP, and the record;
  - **Delete:** `DELETE gallery/sequences/{name}`, refused for the sequence running.

### Where it differs from OpenFlexure

- **The Thing is `sequence`,** not `timelapse` (plan §11, point 2). Its settings keep OpenFlexure's names.
- **The plan is a list of steps,** so z-stacks and positions can join later without a new model (plan D1).
- **Late points are skipped** only once their interval has passed, rather than run late.
- **Images are named by their time point,** so skipped points show.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/sequence.rs` | `SequencePlan`, `Step` and `Due`: the time axis and steps, the check for what isn't supported yet, and when the next point is due |
| `crates/microscope-things/src/sequence.rs` | The `sequence` Thing: its settings, `run_sequence` with its schedule and save queue, `download_zip`, and `running` |
| `crates/microscope-things/src/gallery.rs` | The `Sequence` card type and `DELETE gallery/sequences/{name}`. What scan and sequence cards share: their record's messages, start time and thumbnail, and deleting a run that isn't running |
| `crates/microscope-things/src/data.rs` | `new_folder`, a run's new folder by the scans' naming rules, for scans and sequences |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.sequence:Sequence`, as `sequence` |
| `docs/adr/0031-sequence-model.md` | ADR-0031 |
| `crates/microscope-core/tests/sequence.rs`, `crates/microscope-things/tests/sequence.rs`, `tests/gallery.rs` | The tests |
| `web/tests/e2e/`, `web/tests/visual/` | The rail now has Sequence: the tests that list its tabs, and the visual baselines |
| `web/src/api/generated/` | The new Thing's types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

At `http://127.0.0.1:5090/docs`:

1. **A short timelapse:** `PUT /api/v1/sequence/dt` to `2` and `/api/v1/sequence/duration` to `10`. Then `POST /api/v1/sequence/run_sequence` with `{"name": "try"}`. It takes about 10 s.
   - `sequences/try/` in the data folder has `image_00000.jpeg` to `image_00005.jpeg`, each with its sidecar, and `sequence.json`, its `result` `success`.
2. **In the gallery,** `http://127.0.0.1:5090/#/gallery`, its card shows 6 images, an interval of 2 s and its duration. **View** steps through them with the slider, and **Download** offers the ZIP and the record.
3. **Skipping:** `PUT /api/v1/sequence/autofocus_enabled` to `true` and `dt` to `1`, and run another.
   - Each autofocus takes several seconds, so points are skipped.
   - The invocation's log says "Skipped … time points", and `sequence.json` counts them. Where a later point runs, the images' numbers have a gap.
4. **Cancelling:** set `duration` to `600`, run one, and after a few images `DELETE` its invocation. Its record says `cancelled`, and the images taken are there.

## Design notes and deviations from the plan

- **The plan is pure,** in `microscope-core`, and its schedule is a function: given the time since the start and the next point not yet run, wait, run a point, or end.
  - **Tested on a fake clock:** points on time, a slow point, and points past the end.
  - **Counted in whole nanoseconds,** so 0.1 s for 1 s gives exactly 11 points.
- **Reserved steps:** `Step::ZStack` and `SequencePlan::positions` exist, and `check` refuses them, so adding them later is a change to the Thing, not to the model (ADR-0031).
- **The save queue** is a tokio task that saves each capture on a blocking thread, its JPEG and sidecar as captures are (P26).
  - **At most 4 wait.** A capture that finds it full waits for room.
  - **The sequence's result** includes the queue's: a capture that couldn't be saved ends the sequence with that error.
- **The image count** in the record is the images on disk once the queue is done, so it's right however the sequence ended.
- **Shared with scans:**
  - **The folder naming,** `data::new_folder`, with the kind in the default name and the error;
  - **in the gallery:** the record's messages, start time and thumbnail, and refusing to delete what's running. The scan's 404 and 409 messages now name neither, "There's nothing by that name." and "`…` is still running.".
- **One at a time:** the global lock refuses a second sequence first, as it does a second scan. The Thing's own lock refuses one on a server without the global lock.
- **Size:** 1147 changed lines of hand-written code, over L's 900 and my estimate of 750–880.
  - **Moved code counts twice:** sharing the folder naming, the run cards and the deletes appears as lines removed and added.
  - **One pull request:** the project owner chose to keep it, rather than split off the gallery's cards.

  | Part | Lines |
  |---|---|
  | `microscope-core`'s `sequence.rs`, with its tests | 259 |
  | The `sequence` Thing | 277 |
  | The gallery | 264 |
  | `data.rs` and `smart_scan.rs`: the shared folder naming | 71 |
  | Registry, configuration and modules | 10 |
  | The Things' tests | 259 |
  | The end-to-end tests, for the rail | 7 |

## Tests

- **`microscope-core/tests/sequence.rs`** (6 tests), on a fake clock:
  - the points' count and times, tenths of a second included;
  - on time, every point at its time, and the wait until the next;
  - a slow point: the next runs late within its interval, and the one between is skipped. Points never run back to back to catch up;
  - points past the end skipped;
  - a timelapse's steps, with and without autofocus;
  - the check: a zero interval, a z-stack and positions refused.
- **`microscope-things/tests/sequence.rs`** (4 tests), on the simulator:
  - **a timelapse,** 1 s for 2 s:
    - its three images with their sidecars, taken a second apart;
    - its record;
    - its gallery card: viewer, thumbnail, info, ZIP button and delete endpoint;
    - its ZIP's files;
    - deleted through the gallery;
  - **skipping:** an autofocus at each point every 0.5 s for 4 s. Two images run, named for their points, and seven are skipped;
  - **cancelling:**
    - a second sequence is refused;
    - the gallery won't delete the one running;
    - once cancelled, its record says so, and every image taken is saved with its sidecar;
  - **names:** only plain names, for runs and ZIPs.
- **`tests/gallery.rs`:** the card types now include Sequence, and the test of an unknown one uses "Timelapse".
- **The rail** shows Sequence now that the simulation configures the Thing; its page comes in P42.
  - **The smoke test** lists it among the tabs.
  - **The connection test** starts its server without a sequence, so it still checks that a tab whose Thing is missing is hidden.
  - **The visual baselines** are refreshed.

**Mutation checks:** I broke 13 behaviours, and a test failed for each.

- **The plan:**
  - the start's point missing;
  - a late point run late rather than within its interval;
  - points past the end not skipped;
  - a wait of a whole interval;
  - z-stacks, positions or a zero interval allowed;
  - the autofocus step left out.
- **The Thing:** points skipped mid-sequence not counted, images numbered by count, the final record not written.
- **The gallery:** the running sequence deletable, the interval missing from the card.

The skipping test was first only 1 s long, so nothing was skipped mid-sequence. It now runs for 4 s, so a late point is skipped part-way through too.

## Follow-ups and known gaps

- **The Sequence tab** (P42) sets these and runs them, with the log and Cancel, as the Slide Scan tab does for scans.
- **The invocation's log** isn't captured by the test harness, so the skip warning is checked by the record's count instead.
- **Untested:**
  - a failed autofocus logged while the capture is still taken: the simulator's autofocus doesn't fail;
  - a capture that can't be saved ending the sequence.
- **The ZIP** is built in memory, as a scan's is (P35b).
- **A timing-sensitive P36 test:** `without_focus_a_smart_stack_saves_only_if_asked` needs a preview frame taken during a 100-step sweep, which lasts 0.2 s.
  - **It failed four times in a row** on my machine, on `main` too: "no frames arrived during the sweep". Then it passed three times in a row, with nothing changed.
  - A longer sweep would make it robust; this phase doesn't change it.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.

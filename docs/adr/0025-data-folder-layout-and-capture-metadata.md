# ADR-0025: Data folder layout and capture metadata

- Status: Accepted
- Date: 2026-10-11
- Phase: P26

## Context

- **What's in the data folder:** captures, scans, sequences and calibration data. It's the configuration's `data_folder`, or `.microscope/data` (P17b).
- **Who reads it back:** the gallery (P31) lists what's there and shows each capture's details. Scans (P34 onwards) add their own folders.
- **The source of truth:** the files themselves; there's no database (plan §5.9, ADR-0026 in P31). So each image must carry what the gallery and a reader need: when it was taken, where, and how.
- **Capturing in two ways:** a capture is kept on the microscope ("Save to gallery"), or only downloaded, as OpenFlexure's `capture` does with `retain_image`.

## Decision

- **The layout,** inside the data folder:

  | Folder | What | From |
  |---|---|---|
  | `captures/` | One JPEG per capture, with a JSON sidecar of the same name | P26 |
  | `scans/<scan>/` | A scan's images, its record and its stitched result | P34 onward |
  | `sequences/<name>/` | A timelapse's images and its record | P41 |
  | `calibration/` | Calibration data, such as `camera_stage_mapping_<time>.json` | P23 |

  Only a Thing writes there. Paths from the API are resolved inside the data folder and can't leave it (`data::resolve`, P17b).
- **Naming captures:** `capture-<time>.jpeg`, with the time in UTC to the second, such as `capture-2026-10-11T07-35-12Z.jpeg`, as calibration files are named (P23).
  - **Same-second captures:** a second capture in the same second gets `-2`, then `-3`, and so on.
  - **No overwriting:** the image is created only if no file of that name exists, so even two Things capturing at once never overwrite each other.
  - **The sidecar** has the image's name, with `.json` for `.jpeg`.
- **The sidecar,** our own layout, versioned by `format` (1):
  - **The image:** `image` (its file name), `acquired` (RFC 3339, UTC), `width` and `height`, and `um_per_pixel` if known.
  - **`stage`:**
    - `steps`, the position;
    - `um`, the position in µm on each axis whose scale is known;
    - `um_per_step`.
  - **`camera`:** the camera's settings by name: the objective, exposure, gain, and whatever else the driver reports.
  - **`camera_stage_mapping`:** the image-to-stage matrix and the image size it was calibrated at, if it's calibrated.
  - **`simulation`:** what decides the frame, for a simulated camera. That's the sample, the optics and the true stage position. It's null for real cameras.
- **`camera.capture(capture_mode, retain_image)`:**
  - **`capture_mode`:** `standard` captures at full resolution (1640 × 1232 in the simulator); `quick` captures at the stream's size, without changing it.
  - **With `retain_image`** (the default), the image and its sidecar are saved to `captures/`, and the action returns the saved image as a Blob.
  - **Without it,** nothing is saved, and the action returns the image as an in-memory Blob, for download.
- **Sharing between drivers:** saving and naming live in `microscope_things::capture`, so every camera driver writes the same layout. A driver supplies the image and what it knows; the capture looks up the camera–stage mapping itself.

## Consequences

- **Files are self-describing:** a capture can be copied anywhere and still says how it was taken. The gallery builds its index from the sidecars (P31).
- **Sorting by name sorts by time,** within a day and across days, since the names are UTC and zero-padded.
- **The format can grow:** readers ignore fields they don't know, and `format` changes only when a field's meaning does.
- **UTC names** read less naturally than local time. The gallery shows local times.

## Alternatives considered

- **Metadata in the JPEG's EXIF.** A standard place for it, but the camera–stage mapping and the simulator state don't fit EXIF's fields. It would need a library to read and write, and other tools may strip it. A sidecar is plain JSON.
- **A database of captures** (OpenFlexure's SQLite). It's faster to query, but it can disagree with the files. The plan keeps the files as the truth (ADR-0026).
- **Names in local time,** like OpenFlexure's. They're friendlier, but they're ambiguous when the clocks change, and they differ from the calibration files' names.
- **A sequence number** instead of the time. That's shorter, but it needs a counter that survives restarts, and it says nothing about when.

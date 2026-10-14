# ADR-0029: Scan engine and workflow Things

- Status: Accepted
- Date: 2026-10-13
- Phase: P35

## Context

- **Four workflows,** raster, snake, C-Chip and the Histo smart spiral (plan D3), each plan their path and do their own work at each site. They differ in whether to autofocus, whether to take a z-stack (P36), and whether to skip background (P37).
- **Everything else is shared:** one scan at a time, a folder for it, a record of its settings and how it went, a log, live progress for the Slide Scan tab (P40), and a return to the start however it ends.
- **The planners** are pure code (P34, ADR-0028). They need driving: something moves the stage, focuses and captures, and tells them what happened.
- **OpenFlexure** has a `smart_scan` Thing that runs whichever workflow Thing is chosen. We mirror its names (ADR-0007).

## Decision

- **A `ScanWorkflow` interface** (a `teta-wot` interface, ADR-0018) for the workflow Things:
  - `display_name()` and `ready()`;
  - `begin(start)`: it refuses if it can't start, else gets ready (such as by autofocusing), fixes its settings for the scan, and returns them with its planner;
  - `acquire(folder)`: with the stage at a site, it focuses and saves its images, and says how many it saved and where it found focus. Since P36, `folder` is a folder in the data folder, such as `scans/<name>/images`, as the stack actions take it.
- **`smart_scan`** (`SmartScan`) holds every workflow in a `SlotMap<dyn ScanWorkflow>`, by Thing name.
  - **The choice:** its `workflow_name` setting chooses one.
  - **The names:** `all_workflow_names` and `workflow_display_names` list them.
- **`sample_scan(scan_name)`** runs a scan:
  - **One at a time:** the global lock refuses another action while it runs, and a lock of its own refuses another scan on a server without the global lock.
  - **Refused** when the workflow is unknown or not ready.
  - **The name:** letters, digits, spaces, `-`, `_` and `.`, not starting with `.`; or `scan-<UTC time>` without one. It's made unique with `-2`, `-3`, … The folder is made with the name, so two scans can't share one.
  - **The folder:** `scans/<name>/` in the data folder (plan §5.9), holding:
    - `images/`: each image a JPEG with its sidecar (ADR-0025);
    - `scan.json`: the record, with `format` 1, the name, the workflow, its settings, the start, when it started, its duration, the image count and the result;
    - `scan.log`: timed lines, which also go to the action's log.
  - **The loop:** for each site the planner gives, the stage moves to it, at the z the planner suggests (or where z is). The workflow acquires there, and the planner is told what happened: the z where focus was found (else where the stage moved), whether an image was taken, and whether focus was found.
  - **The end:** however it ends (it finishes, it's cancelled, or it fails), the stage goes back to the start, z included. The record's result says how: `success`, `cancelled` or `error: …`. The log says why a finished scan stopped (P34's `Finished`).
- **`latest_scan_live_details`:** the latest scan's name, phase (`setup`, `scanning` or `complete`), image count, duration, workflow, settings and result, while it runs and after.
- **The grid workflows are one Thing type,** `GridWorkflow` (`microscope.workflows:GridWorkflow`), configured with its `order` (`snake` or `raster`) in `kwargs`.
  - **Settings,** OpenFlexure's defaults: `overlap` 0.35 (0.1–0.7), `autofocus_dz` 1000 (400–3000), `x_count` 3, `y_count` 2, and `capture_mode`. Also `ready`, `display_name` and a `settings_ui` panel (ADR-0024).
  - **`begin`:** it needs the camera–stage mapping. It plans a grid from the start, with the step from the overlap through the mapping (P34), then autofocuses, repeating until it's in focus.
  - **`acquire`:** a fast autofocus, then a capture saved as `img_<x>_<y>_<z>.jpeg`. Since P36, a smart stack instead, with the stack settings `stack_images_to_save`, `stack_min_images_to_test` and `stack_dz`. It saves even without focus, and the planner is told of focus only if the stack found it.
- **The camera interface:** `CameraApi::capture_to_memory` takes a `CaptureMode`, now part of the interface, so a workflow can capture `standard` or `quick`.

## Consequences

- **A new workflow** (P37's Histo and C-Chip) is a Thing that provides `ScanWorkflow`, named in the configuration. `smart_scan` finds it without changes.
- **A scan's folder stands on its own:** its images say where they were taken, and its record says how it was run. The gallery (P35b) and the stitcher (P38, P39) read it without the server's help.
- **If the server stops mid-scan,** the record has no result, and the stage isn't returned.
- **The settings are fixed** when the scan begins, so changing them during a scan affects only the next one.
- **The stage stays put for the slowest part:** the workflow focuses and captures with the stage at each site. Moving on while saving could come later.

## Alternatives considered

- **A Thing type for each grid workflow,** as OpenFlexure's classes are. `teta-wot` Things can't be generic, so they'd repeat their fields and code. Configuration tells them apart just as well.
- **Workflows that run the whole loop.** Each would then repeat the folder, the record, the cancellation and the return to the start.
- **Finding workflows by looking through the server's Things.** `teta-wot`'s server doesn't offer it. A slot map is its way, and the configuration can override it.

# P17b: Capture buffer and settling

- Status: In review
- Pull request: [#23](https://github.com/lucaswewa/microscope/pull/23)
- ADRs: none new; it follows [ADR-0018](../../adr/0018-hardware-abstraction-through-teta-wot-interfaces.md) (shared structs) and [ADR-0020](../../adr/0020-camera-frame-pipeline.md) (grabbing and capturing)
- Spec: [phases.md#p17b](../phases.md#p17b), split from P17 (plan Appendix D)

## Summary

The camera can now hold captures in memory and save them later, and give a fresh frame after a move:

- **`capture_to_memory`** draws a 1640 × 1232 capture and returns its id. At most `buffer_max` captures are kept.
- **`save_from_memory`** saves one to a path in the data folder, as a JPEG or PNG, without the global lock.
- **`clear_buffers`** empties memory.
- **`settle`** waits `settling_time`, then calls `discard_frames`. That waits for any frame being drawn, so the next frame grabbed shows the stage where it is now.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/hardware/capture_buffer.rs` | `CaptureBuffer`, the shared struct ADR-0018 promised, with its unit tests |
| `crates/microscope-things/src/data.rs` | `DEFAULT_DATA_FOLDER`, `data_folder()` (from the configuration's `application_config`) and `resolve()`, which keeps paths inside it |
| `crates/microscope-things/src/camera.rs` | The actions `capture_to_memory`, `save_from_memory`, `clear_buffers`, `settle` and `discard_frames`, and the setting `settling_time` |
| `crates/microscope-things/src/stage.rs` | `position_now()`, now public, so a capture can record the position without awaiting |
| `crates/microscope-server/src/config.rs` | The default data folder comes from `microscope_things::data` |
| `crates/microscope-things/tests/camera.rs`, `tests/stage.rs` | The tests below, and the global lock switched on in the harness |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

Then, in another terminal, also in the repository's root. The data folder, `.microscope/data`, is relative to the folder the server started in.

A POST only starts an action, so `Invoke-Action` waits for each to end. Without it, `save_from_memory`, which doesn't wait for the global lock, could run before the capture is done:

```powershell
$api = 'http://127.0.0.1:5090/api/v1'
function Invoke-Action($path, $body = '{}') {
    $run = Invoke-RestMethod -Method Post "$api/$path" -ContentType application/json -Body $body
    while ($run.status -in 'pending', 'running') { Start-Sleep -Milliseconds 100; $run = Invoke-RestMethod $run.href }
    $run
}
Invoke-Action camera/capture_to_memory                            # status completed, output 1
Invoke-Action camera/save_from_memory '{"path": "tries/first.jpg"}'
Get-Item .microscope/data/tries/first.jpg
Invoke-Action camera/settle
Invoke-Action camera/save_from_memory '{"path": "tries/again.jpg"}'   # error: memory is empty now
```

## Design notes and deviations from the plan

- **The buffer follows OpenFlexure's:**
  - ids count from 1;
  - `buffer_max` (at least 1) is the most held once a capture is added, and the oldest go first;
  - saving takes the capture out of memory;
  - saving without an id takes the latest and empties memory, so older captures can't be saved out of order by mistake.
- **`save_from_memory`:**
  - **The path** is relative to the data folder. Anything with a root, a drive or `..` is refused.
  - **The format** comes from the extension (`.jpg`, `.jpeg` or `.png`, in any case). JPEGs are saved at quality 95.
  - **Order:** the path and format are checked before the capture is taken from memory, so a refused save loses nothing. A failure while writing loses the capture, as in OpenFlexure.
  - **Folders** are created as needed, and an existing file is overwritten. P26 brings the data folder's layout, the naming scheme and the JSON sidecar with the metadata, so for now only the image is saved.
  - **Threads:** it runs on a blocking thread and doesn't take the global lock, so it can save while, for example, a scan moves on.
- **The data folder** is `application_config.data_folder` from the configuration file, read through `teta-wot`'s `Server::application_config`. The default, `.microscope/data`, is one constant shared with the server.
- **`discard_frames`** is OpenFlexure's name. Our simulator draws each frame where the stage is when drawing starts, so after a move, the only stale frame is one being drawn at the time. `discard_frames` waits for it. OpenFlexure's simulator does nothing here; a real camera drops the frames in its queue.
- **`settle`** waits `settling_time` (0.2 s by default, up to 60 s, cancellable), then discards frames. Nothing in the simulation needs the wait itself, since the stage doesn't wobble, but camera–stage mapping (P23) expects it.
- **The harness tests now switch the global lock on,** as the shipped configuration does. `teta-wot`'s harness leaves it off by default, so P16's test that jogs and `halt` interrupt a move holding the lock didn't really test the lock. Now it does: taking the lock in either action fails it.
- **Size:** 490 changed lines of hand-written code (about 230 of them tests), within M's budget of 600.

## Tests

- **`CaptureBuffer`** (2 unit tests):
  - ids count up;
  - `buffer_max` drops the oldest, and 0 is taken as 1;
  - taking by id removes that capture;
  - taking the latest empties the buffer, as does `clear`.
- **`resolve`** (1 unit test) accepts paths inside the folder. It refuses an empty path, `.`, `..`, `../a.jpg`, `captures/../../a.jpg`, `/a.jpg`, `C:/a.jpg`, `C:a.jpg` and `\\pc\share\a.jpg`.
- **In `tests/camera.rs`** (3 new):
  - **Captures wait in memory until saved:**
    - with `buffer_max` 2, three captures leave ids 2 and 3;
    - saving id 2 writes a 1640 × 1232 JPEG in the data folder, and saving it again fails;
    - paths outside the folder and other formats are refused, write nothing, and leave the capture in memory;
    - saving the latest as a PNG empties memory, as does `clear_buffers`.
  - **Saving needs no global lock:** it completes while a flash holds the lock.
  - **After settling, the next frame shows where the stage is:** four moves, each followed by `settle` and one grab, in full-resolution mode, where a frame is nearly always being drawn during a move. `settle` waits `settling_time`.

**Mutation checks:** I broke each of the following, and a test failed every time:

- discarding (three runs);
- waiting in `settle`;
- saving without the lock;
- the path check;
- the format check;
- emptying the buffer on taking the latest;
- dropping the oldest;
- removing on take;
- the lock-free `jog` and `halt` from P16.

**Checked by hand:** a server run with a configured data folder saved its capture there, and refused `../escape.jpg`. The camera and stage tests passed eight runs in a row.

## Follow-ups and known gaps

- **P21** (autofocus) and **P23** (camera–stage mapping) use `settle` and grabs. If they reach the camera through `CameraApi`, the interface gains `settle`, an additive change (ADR-0018).
- **P26** saves captures with their metadata in a JSON sidecar, and lays out the data folder.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The buffer's behaviour and the action names follow OpenFlexure's (ADR-0007), from reading its source for behaviour only.

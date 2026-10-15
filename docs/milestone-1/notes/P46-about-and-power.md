# P46: About and Power tabs

- Status: In review
- Pull request: [#59](https://github.com/lucaswewa/microscope/pull/59)
- ADRs: none new
- Spec: [phases.md#p46](../phases.md#p46)

## Summary

The last two tabs: status, and the server's lifecycle, from the app.

- **About:**
  - the microscope's computer, its address, the server's version and where it came from, the operating system, and whether the launcher manages it;
  - the camera, stage and illumination, by their Things' types, and Flash illumination, to find which microscope this is;
  - what the software is for, in our own words, and links to its source and its issues.
- **Power:**
  - **Shut down,** after asking. The app then shows the microscope has shut down, with Reconnect, and stops trying to reach it;
  - **Restart,** after asking, only when the launcher manages the server (ADR-0033);
  - **Disconnect,** for another microscope.
- **Every tab is now built,** so the placeholder page and each destination's `builtIn` phase are gone.
- **A launcher fix:** after a restart, the server keeps the port it served on, so the app finds it again.

## What was built

| Where | What |
|---|---|
| `web/src/system/AboutPage.vue` | The About tab |
| `web/src/system/PowerPage.vue` | The Power tab |
| `web/src/connection/store.ts` | `shutDown` and `markShutDown`: the server was shut down from this app, so it stops trying to reach it |
| `web/src/connection/OfflineOverlay.vue` | "The microscope has shut down", with Reconnect |
| `web/src/router/index.ts`, `web/src/app/navigation.ts` | The two routes; the placeholder page, its layouts and `builtIn` removed |
| `crates/microscope-launcher/src/supervise.rs` | The port the server served on is preferred when it starts again |

## How to try it

```powershell
npm --prefix web run build
cargo build -p microscope-server -p microscope-launcher
target\debug\microscope-launcher.exe --root $env:TEMP\microscope-try
```

1. **About:** the computer's name, `127.0.0.1:5000` (or the port the launcher chose), the server's version with its `git describe`, the operating system, and "Managed by the launcher: Yes: it can restart". The hardware is the simulator's: SimulatedCamera, SimulatedStage, SimulatedIllumination.
2. **Flash illumination:** the live image (View) blinks dark ten times.
3. **The links** open the repository and its issues in a new tab.
4. **Power, Restart:** after the question, the server restarts; the app shows the connection lost, then reconnects.
5. **Power, Shut down:** after the question, the app shows "The microscope has shut down", and the launcher ends. Start the launcher again, then Reconnect.
6. **Without the launcher:** `cargo run -p microscope-server -- -c configs/simulation.json` gives "Managed by the launcher: No", and Power offers no Restart.
7. **Disconnect:** connected to another microscope by its address (Settings, Connection), Power offers Disconnect, which goes to the Connect screen.

## Design notes and deviations from the plan

- **The server's facts** are read once, when the tab opens: they don't change while it runs.
- **Hardware types** are the Things' descriptions' titles, as the spec asks, so a real camera shows its own type. A Thing the server lacks shows "None".
- **Flash illumination** is a server-described action button (ADR-0024), with its progress and Cancel.
- **The links** open through the host (ADR-0010), as rich text's links do, so a desktop app can open them in the system's browser.
- **Shut down** marks the app's connection as shut down as soon as the server has the request. Its invocation's end isn't awaited, since the server is stopping. The app then stops its heartbeat and retries, rather than reporting the connection lost, until Reconnect or another connection.
- **Restart** leaves the app's usual reconnection to the server's return; a toast says it's restarting.
- **The app's own version** is on About too, beside the server's, to tell the two builds apart.
- **The launcher kept the port only by luck (P45).** It chose the port again at each start: 5000 if free, or else whichever the server picked. So when 5000 was taken, a restart from this tab moved the server, and the app couldn't follow it. Now the port the server served on is preferred for its next start. Writing this phase's "How to try it" found it.
- **No visual baselines:** About shows the computer's name and the build's version, which differ between machines and commits.
- **Size:** 638 changed lines of hand-written code, 38 over M's 600, within my estimate of 570–650.
  - **Removing the placeholder page** accounts for 81 of them. It's done here because P46 builds the last two tabs, which left it unused.
  - **The launcher's fix** accounts for 28, with its test.

  | Part | Lines |
  |---|---|
  | `AboutPage.vue` | 156 |
  | `PowerPage.vue` | 122 |
  | The shut-down screen: the store and the overlay | 28 |
  | Routing, and removing the placeholder page | 81 |
  | The launcher's port | 15 |
  | Tests | 236 |

## Tests

- **`tests/unit/system/SystemPages.spec.ts`,** against a fake server:
  - **About:** the facts, the managed status, the hardware with one Thing missing, the intended use, Flash illumination, and a link opened through the host;
  - **Power:**
    - Shut down, cancelled and then confirmed; the shut-down screen; no requests while shut down; Reconnect;
    - a shutdown that fails;
    - Restart, when managed;
    - Disconnect, for another microscope, and Restart and Disconnect hidden when they don't apply.
- **`tests/unit/app.spec.ts`:** every destination opens its own page.
- **`tests/e2e/system.spec.ts`:** About against the real server.
- **`tests/e2e/connection.spec.ts`:** a fresh microscope shut down from the Power tab, its process ending with 0, then started again and reconnected.
- **`crates/microscope-launcher/tests/launcher.rs`:** with the preferred port taken, a restart keeps the port the server chose. Without the fix, the server moved, from port 60580 to 60582.

**Mutation checks:** I broke 13 behaviours, and a test failed for each.

- **Power:** not marked as shut down, shut down without asking, Restart shown unmanaged, Disconnect shown for this server, Disconnect not going to the Connect screen, Restart without its toast, and a failed shutdown not reported.
- **The connection:** still trying once shut down, the shut-down screen kept after reconnecting, and no shut-down screen.
- **About:** never managed, a missing Thing blank rather than "None", and the links not opened.

**By hand,** on my own server, in both themes: About, Power, the shut-down question, and the shut-down screen.

## Follow-ups and known gaps

- **Restart through the launcher** is tested in P44 and P45, but not from this tab end to end: the end-to-end tests' servers aren't managed.
- **The connection indicator** says "Not connected" once the server is shut down, as after Disconnect.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.

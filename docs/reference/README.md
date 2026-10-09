# Reference screenshots

The web app's visual design is reviewed against the OpenFlexure Microscope's interface, in both themes. This folder holds the process. The screenshots stay out of git: OpenFlexure is GPL-3.0, and this repository is MIT ([ADR-0002](../adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md)).

| Path | What | In git |
|---|---|---|
| `README.md` | This guide | Yes |
| `ofm/` | OpenFlexure's screenshots, and `measurements.json` | No |
| `microscope/` | This app's screenshots, with the same file names | No |
| `compare.html` | Both side by side, written by each capture run | No |

Screenshots are named `<state>.<theme>.<width>x<height>.png`: for example `control.dark.1280x800.png`. They come in the light and dark themes, at 1280×800 and 1920×1080.

## Running OpenFlexure's simulator

Once per machine. The setup uses [uv](https://docs.astral.sh/uv/) for Python, and Node.js for OpenFlexure's web app. Everything it creates in the OpenFlexure clone (`.venv/`, `webapp/node_modules/`, `src/openflexure_microscope_server/static/`, `openflexure/`) is git-ignored there.

1. Clone [OpenFlexure's server](https://gitlab.com/openflexure/openflexure-microscope-server) (the `v3` branch) next to this repository, as `../openflexure-microscope-server`.
2. Install uv, if it isn't installed. For example, with a Python already on the machine:

   ```powershell
   python -m pip install --user --uploaded-prior-to <date two weeks ago> "uv>=0.9,<1"
   ```

3. Create OpenFlexure's environment, with a uv-managed Python 3.12. Pillow 10.4, which OpenFlexure pins, has no Windows wheels for Python 3.13.

   ```powershell
   cd ..\openflexure-microscope-server
   python -m uv venv .venv --python 3.12
   python -m uv pip install --python .venv --exclude-newer <date two weeks ago> -e .
   ```

4. Build its web app, which goes to `src/openflexure_microscope_server/static/`. OpenFlexure asks for Node 26, but Node 24 builds it, with a warning.

   ```powershell
   cd webapp
   npm ci
   npm run build
   ```

`--uploaded-prior-to` and `--exclude-newer` keep anything published in the last two weeks out, as this project's own dependencies are ([ADR-0009](../adr/0009-frontend-toolchain.md)).

To run the simulator, from the OpenFlexure clone, on a port nothing else uses:

```powershell
.venv\Scripts\activate
openflexure-microscope-server --fallback -c ofm_config_simulation.json --port 5095
```

Activating the environment matters: scans stitch by running `openflexure-stitch` from it, and fail with "The system cannot find the file specified" if it isn't on `PATH`.

Its settings, data and logs go to `openflexure/` in the clone. Delete that folder to start from an uncalibrated microscope, whose first page is the calibration wizard's welcome.

## Capturing

From `web/`, with Playwright's Chromium installed (`npx playwright install chromium`):

```powershell
npm run capture:reference -- --target ofm --url http://127.0.0.1:5095/
npm run capture:reference -- --target microscope --url http://localhost:5173/
```

Use `localhost` for Vite's development server: on Windows it listens on IPv6 (`::1`) only, so `127.0.0.1` is refused.

**With `--target ofm`**, the script ([`web/tests/reference/capture.ts`](../../web/tests/reference/capture.ts)):

1. captures the calibration wizard's welcome page (`calibration-wizard`), if the microscope is uncalibrated; otherwise it keeps an earlier run's;
2. calibrates the camera and the camera–stage mapping, takes three captures, and sets the snake workflow to three by two fields, all through OpenFlexure's API;
3. captures every tab and Settings section, then the wizard as Settings launches it (`calibration-wizard-from-settings`);
4. starts a snake scan and captures Slide Scan while it runs;
5. waits for the scan, switches back to the Histo workflow, and captures the Gallery;
6. measures sizes and colours of the main elements into `ofm/measurements.json`.

**With `--target microscope`**, it opens this app's routes for the same states (`/#/control`, `/#/settings/camera`, …) and captures them under the same names. Routes that don't exist yet show as blank pages until the phase that builds them.

Both runs rewrite `compare.html`. Open it in a browser to see the two apps side by side.

## Reviewing

Look at proportions and roles, not pixels:

- the rail's width, its icon and label sizes, and how the active tab stands out;
- the widths of the control panes;
- the heights and spacing of controls, and the type sizes;
- which surfaces are light, dark and accent, in each theme.

The phases that change the look ([P06](../milestone-1/phases.md#p06), [P07](../milestone-1/phases.md#p07), [P47](../milestone-1/phases.md#p47)) record what they compared, and the differences they keep, in their notes. Never trace or copy OpenFlexure's assets from the screenshots.

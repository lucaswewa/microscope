# OPTIMA CAD

FreeCAD models of the [OPTIMA hardware design](../../docs/misc/optima_hardware_design.md), built by Python scripts. The [CAD plan](../../docs/misc/optima_cad_plan.md) sets out the phases and conventions.

## Looking at the models

Open any file in [`models/`](models/) with FreeCAD 1.1. Each has its parts in groups, and three sheets:

- **Parameters:** the values it was built from;
- **Facts:** the positions and sizes derived from them;
- **Checks:** what was checked, and the results.

[`models/REPORT.md`](models/REPORT.md) has every model's checks and facts as text.

## Changing them

Don't edit the `.FCStd` files: the build replaces them.

1. Change [`optima_cad/params.py`](optima_cad/params.py) or the scripts.
2. Run the build, with FreeCAD 1.1 installed:

   ```sh
   python cad/optima/make.py
   ```

   It runs FreeCAD offscreen, in safe mode, so your own FreeCAD settings play no part. It takes a few seconds and prints every check. If FreeCAD hangs on its way out, `make.py` stops it 30 seconds after the build has finished. Set `FREECAD` to FreeCAD's program if `make.py` doesn't find it.
3. Commit the scripts with the rebuilt `models/`.

`python cad/optima/make.py --check` builds into a temporary folder instead. It fails if any check fails or if `models/` isn't what the scripts build. Run it before every pull request.

## Files

| File | What it does |
|---|---|
| `make.py` | Finds FreeCAD, runs the build, writes `models/` and `REPORT.md`, or checks them |
| `freecad_main.py` | What FreeCAD runs: builds the models and reports back |
| `optima_cad/params.py` | Every dimension, with its source |
| `optima_cad/optics.py` | First-order optics: positions along the axis, apertures, the LED matrix's NAs |
| `optima_cad/geom.py` | Shape helpers |
| `optima_cad/document.py` | Adding coloured shapes and sheets, and saving files reproducibly |
| `optima_cad/layout.py` | The optical layout and its checks |
| `optima_cad/stages.py` | The XY stage and the Z lift, with their moving parts, and their checks |
| `optima_cad/cameras.py` | The mono, colour and overview cameras, and their checks |
| `optima_cad/parts.py` | The objectives, the tube lens, the mirrors and camera port, the epi parts, the guides and steppers, the LED matrix and the electronics, and their checks |
| `optima_cad/head.py` | The optical head: its custom parts, with the bought parts in place, and its checks across every moving part's travel |
| `optima_cad/stand.py` | The stand: the base, the column, the stage stack on the Z lift and the overview camera's bracket, and their checks across the lift's and the stage's travel |
| `optima_cad/inserts.py` | The sample inserts: the adapter, the inserts and their labware, and their checks |
| `optima_cad/assembly.py` | The whole instrument: the head on the stand in its enclosure, and its checks across every moving part's travel, its envelope and its mass |
| `optima_cad/enclosure.py` | The enclosure and the electronics: the cover, its door, interlock, intake and fan, the electronics tray and the cable ducts, and their checks |
| `optima_cad/checks.py` | Checks, and the overlaps they look for |
| `optima_cad/build.py` | The list of models, and how each is built and saved |

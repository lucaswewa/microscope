# OPTIMA CAD

FreeCAD models of the [OPTIMA hardware design](../../docs/misc/optima_hardware_design.md), built by Python scripts. The [CAD plan](../../docs/misc/optima_cad_plan.md) sets out the phases and conventions.

## Looking at the models

Open any file in [`models/`](models/) with FreeCAD 1.1. Each has its parts in groups, and three sheets:

- **Parameters:** the values it was built from;
- **Facts:** the positions and sizes derived from them;
- **Checks:** what was checked, and the results.

[`models/REPORT.md`](models/REPORT.md) has every model's checks and facts as text.

## Making the parts

[`outputs/`](outputs/) holds what a workshop or a supplier needs, built from the same scripts:

- **`drawings/`:** a PDF drawing of every custom part. Each shows the part in first-angle projection with an isometric view, its overall sizes and notes, and a title block with its number, material, finish and quantity. Its number, such as OPTIMA-H03, is on its files and in the bill of materials.
- **`step/`:** a STEP file of every custom part, as modelled, moved so that its box starts at the origin. Take any size a drawing doesn't show from it.
- **`bill-of-materials.csv`:** the custom parts, with their sizes, and the bought parts, with the lengths to cut them to.

Every screw that joins the parts is modelled, with the holes it needs; each drawing notes its part's tapped and counterbored holes.

## Changing them

Don't edit the `.FCStd` files: the build replaces them.

1. Change [`optima_cad/params.py`](optima_cad/params.py) or the scripts.
2. Run the build, with FreeCAD 1.1 installed:

   ```sh
   python cad/optima/make.py
   ```

   It runs FreeCAD offscreen, in safe mode, so your own FreeCAD settings play no part. It takes about a minute and prints every check. If FreeCAD hangs on its way out, `make.py` stops it 30 seconds after the build has finished. Set `FREECAD` to FreeCAD's program if `make.py` doesn't find it.
3. Commit the scripts with the rebuilt `models/` and `outputs/`.

`python cad/optima/make.py --check` builds into a temporary folder instead. It fails if any check fails or if `models/` or `outputs/` isn't what the scripts build. Run it before every pull request.

Every file is compared byte for byte, except the drawings. TechDraw paints a page's items in an order that changes from run to run, and places some text a millimetre apart, so a drawing is compared by what it draws and says. It also counts as changed whenever its part's STEP file does, which pins the part's sizes exactly. `make.py` writes only the files that changed, so a build that changes nothing leaves `outputs/` alone.

## Files

| File | What it does |
|---|---|
| `make.py` | Finds FreeCAD, runs the build, writes `models/`, `REPORT.md` and `outputs/`, or checks them |
| `freecad_main.py` | What FreeCAD runs: builds the models and the outputs, and reports back |
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
| `optima_cad/fasteners.py` | Screws: where each goes, the holes it needs, its shape, and the checks that it fits |
| `optima_cad/drawings.py` | The outputs: the custom parts' list, their drawings and STEP files, and the bill of materials, with their checks |
| `optima_cad/sheet.svg` | The drawings' A3 sheet: a border and a title block |
| `optima_cad/checks.py` | Checks, and the overlaps they look for |
| `optima_cad/build.py` | The list of models, and how each is built and saved |

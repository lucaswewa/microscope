# OPTIMA CAD plan

How the [OPTIMA hardware design](optima_hardware_design.md) becomes FreeCAD models, in phases.

Started 2026-10-31.

## 1. Decisions

| Decision | Choice |
|---|---|
| Format | FreeCAD 1.1 documents (`.FCStd`) |
| Source of truth | Python scripts in [`cad/optima/`](../../cad/optima/README.md) build every document. The built files are committed, so they open without a build, but they're never edited by hand. |
| Bought parts | This project's own simplified models, from the vendors' published drawings: outlines, mounting holes, travel and optical planes. No vendor CAD files are kept in the repository. |
| Colour camera | The colour IMX492, as the owner decided on 2026-10-31 |
| Stages | Standa 8MTF-102LS05 (XY) and 8MVT100-25-1 (Z), as the owner decided on 2026-10-31 |
| Checks | Each model computes its own checks, such as clearances, optical path lengths and apertures. They're stored in the file and in `models/REPORT.md`, and a failing check fails the build. |

## 2. Conventions

- **Units and axes.** Millimetres. The origin is on the optical axis at the sample's focal plane.
  - +Z points up, toward the objectives.
  - +X points right, seen from the front: the side the epi arm leaves on.
  - +Y points to the back, toward the column.
  - The optics stay fixed, so this origin holds in every phase. The stage's position is a parameter of the assemblies.
- **Parameters.** Every dimension lives in [`params.py`](../../cad/optima/optima_cad/params.py), with its source. "Assumed" marks a value no vendor publishes.
- **What each document holds:**
  - its objects in named groups, coloured by kind;
  - a **Parameters** sheet, the values it was built from;
  - a **Facts** sheet, the positions and sizes derived from them;
  - a **Checks** sheet, the checks and their results.
- **Colours.**
  - Light: imaging green, colour-camera light orange, excitation violet, transmitted light amber.
  - Parts: optics translucent blue, camera bodies translucent black.
  - Reserved space for moving parts: faint red.
- **Reproducible files.**
  - The build runs FreeCAD offscreen, in safe mode, so nobody's FreeCAD settings play a part.
  - It removes the dates, IDs and orderings that change from save to save.
  - The same scripts and FreeCAD 1.1 give byte-identical files, which lets `make.py --check` tell whether `models/` is up to date.

## 3. How work is delivered

The repository's rules for phases apply, with these differences:

- **Branches and pull requests.** One branch, `optima/cNN-short-name`, and one pull request, `CNN: <title>`, per phase. The owner approves each pull request before it's merged, and the next phase starts from an up-to-date `main`.
- **Each pull request contains:**
  - the scripts;
  - the rebuilt `models/` and `REPORT.md`;
  - the phase's status in §5;
  - any change to the design document that the modelling forced.
- **Size budget.** The size budget counts the scripts; built files, the report and docs don't count.
- **Done.** A phase is Done once the owner has opened its models in FreeCAD and approved the pull request. The next phase's pull request marks it Done here.
- **Before each pull request,** run `python cad/optima/make.py --check`. CI doesn't run FreeCAD; a CI job can be added later if wanted.

## 4. Phases

| Phase | Title | Size | What it adds |
|---|---|---|---|
| C01 | Tooling and the optical layout | L | The build tooling (`make.py`, reproducible files, `--check`), the parameters, and `optical-layout.FCStd` with its checks ([§6](#6-c01-the-optical-layout)) |
| C02 | Bought parts: stages and cameras | M | Simplified models of the Standa 8MTF-102LS05, with its moving plates and travel; the Standa 8MVT100-25-1; and the mono, colour and overview cameras, each in its own document |
| C03 | Bought parts: optics, motion and electronics | M | The Mitutoyo objectives, the TTL200-A, the mirrors and the MFF102/M, the cage parts used, the mounted LEDs and filters, the MGN12 rails, the steppers, the LED matrix, the controller boards, the supply |
| C04 | The optical head | L | The custom parts: the objective changer with its detents, the filter slider, the head plate, the tube-lens and mirror mounts, the camera mounts with focus adjustment, and the epi arm. Then the head's assembly, checked against the optical layout. |
| C05 | Stage stack, base and column | M | The base plate, the Z-lift plate and stage spacers, the LED matrix's post, the 160 × 110 mm insert adapter and inserts (plate, four slides, dishes), the column, and the overview camera's bracket |
| C06 | The assembly and its motion | M | The whole instrument, and its checks across the XY travel, the Z range and the objective changes against labware heights. The envelope and the mass. |
| C07 | Enclosure and electronics | M | The frame, panels, door and interlock; the electronics tray (supply, both controllers, LED drivers); cable routes and vents |
| C08 | Manufacturing outputs | M | A TechDraw drawing (PDF) and a STEP file for every custom part, and a bill of materials (CSV) taken from the model |

## 5. Status

| Phase | Status |
|---|---|
| C01 | In review |
| C02–C08 | Not started |

## 6. C01: the optical layout

`optical-layout.FCStd` holds the optics without their mechanics. It shows:

- the sample, with each objective's field of view;
- the three objectives on the changer, with their exit pupils;
- the tube lens, the fold mirror and the flip mirror;
- the light for the image's centre and corners, folded to both cameras;
- the sensors and the camera bodies;
- the epi illuminator and its light;
- the LED matrix and its brightfield light;
- the space the objective changer and the filter slider sweep.

Its checks:

- The image corners clear the tube lens's pupil, for every objective.
- The sensor lies within the tube lens's rated field.
- The light stays inside both mirrors' clear apertures.
- Nothing collides.
- The changer, slider and tube lens stack in order.
- The LED image fits every back aperture.
- The LED matrix's NA groups match the design.

**What the layout changed in the design.** The design's f = 125 mm epi lens put the LEDs about 380 mm to the side of the axis. C01 uses an f = 100 mm lens instead, and folds the arm back toward the column after the combiner. The LEDs then sit about 225 mm to the side, the field stop is 11.5 mm, and the LED image is 5 mm. The design document's §5.2 and bill of materials say so.

**Still assumed.**

- The exit pupil sits 20 mm below each objective's shoulder. At that depth, the 5x and 10x corners have 6.6 mm of margin before the tube lens's pupil clips them.
- The tube lens is a thin lens at its entrance pupil.
- The colour camera's body is the mono camera's size, until C02.
- The mirrors' clear aperture is 90 % of their diameter.

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
  - Each bought part is built in its own frame, which its module describes, and the assemblies place it. A part with moving parts takes their positions, so the assemblies can check it anywhere in its travel.
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
| C05 | Stage stack, base and column | M | The base plate, the Z-lift plate and stage spacers, the LED matrix's mount, the 160 × 110 mm insert adapter and inserts (plate, four slides, dishes), the column, and the overview camera's bracket |
| C06 | The assembly and its motion | M | The whole instrument, and its checks across the XY travel, the Z range and the objective changes against labware heights. The envelope and the mass. |
| C07 | Enclosure and electronics | M | The frame, panels, door and interlock; the electronics tray (supply, both controllers, LED drivers); cable routes and vents |
| C08 | Manufacturing outputs | M | A TechDraw drawing (PDF) and a STEP file for every custom part, and a bill of materials (CSV) taken from the model |

## 5. Status

| Phase | Status |
|---|---|
| C01 | Done |
| C02 | Done |
| C03 | Done |
| C04 | In review |
| C05–C08 | Not started |

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
- The colour camera's body is the mono camera's size. C02 replaced it with the real one.
- The mirrors' clear aperture is 90 % of their diameter.

## 7. C02: stages and cameras

Each bought part has its own document, built from its vendor's drawing:

| Document | Part | From | Its checks |
|---|---|---|---|
| `xy-stage.FCStd` | Standa 8MTF-102LS05 | Standa's PDF drawing, whose vector outlines give exact positions | The footprint (□276.5 mm), the height (45 mm) and the clear aperture (□115 mm at the centre, □64 mm at a corner of the travel) match the drawing. The travel covers a 96-well plate's wells. The moving parts clear the fixed ones across the travel. |
| `z-lift.FCStd` | Standa 8MVT100-25-1 | Standa's drawing, an image | The length with the connector (140 mm) and the height over the travel (100–125 mm) match the drawing. The top clears the column. |
| `mono-camera.FCStd` | ToupTek CTR3CMOS45000KMA | ToupTek's dimensioned picture | The image corners' light clears the C-mount's bore, with every objective. |
| `colour-camera.FCStd` | ToupTek E3ISPM45000KPB, the colour IMX492 | ToupTek's E3ISPM drawing | The same as the mono camera |
| `overview-camera.FCStd` | A 12 MP UVC module, still to be chosen | Assumed | The lens's image circle covers the sensor. |

**How the XY stage works.** Its three 175 mm plates share the 115 mm opening. The Y drive is fixed to the base on the left, with its knob at the front. The X drive rides on the middle plate at the back, with its knob on the right. The top plate has 4 mm-deep pockets at the opening's corners. The stage and its drives sweep 289 × 328 mm over the travel, which the document shows as reserved space.

**The optical layout** now has the real camera bodies, with their mounting faces down and room for their plugs. Its collision check includes the plugs, and each camera against its own light.

**What C02 changed in the design document.**

- The stage's plates are 175 mm square, not 214 mm; 276.5 mm is its square with the drives.
- It sweeps about 289 × 328 mm over its travel, not 316 mm.
- Its top plate has pockets at the opening's corners, not a recess around it.
- The colour camera's model is the E3ISPM45000KPB.

**Still assumed,** until the parts are measured:

- The XY stage's plates are 15 mm each, a third of its height.
- The lift's column is 90 mm square; the drawing shows about 91 × 88 mm.
- The mono camera's nose is 60 mm across and its fan housing 8 mm deep: the picture has no dimension for the first, and a blurred one for the second.
- The colour camera's ring is 40 mm across, its edges have a 6 mm radius, and its 1/4-20 hole is centred.
- The C-mount's bore is 24 mm, a little inside the thread's minor diameter.
- The room each camera's plugs need.
- The whole overview camera: an IMX577 on the common 38 mm board, with a 3.2 mm M12 lens.

## 8. C03: optics, motion and electronics

| Document | Parts | Its checks |
|---|---|---|
| `objectives.FCStd` | The three Mitutoyo objectives on the changer's 40 mm pitch | Each one's length and working distance make the 95 mm parfocal length. Neighbours don't touch: 6 mm between their knurled rings. |
| `imaging-optics.FCStd` | The TTL200-A's housing, the fold mirror in a 60 mm cage cube, and the MFF102/M camera port in both positions | Position 2 takes the ring 49 mm from the light's centre, which the light reaches only 15 mm from. The tube lens's housing clears the filter slider. |
| `epi-optics.FCStd` | A mounted LED, a dichroic cage cube, a cage plate, the epi fold mirror's cube, and the Chroma 89402 filters | The dichroic fits the cube's holder. A mounted LED fits between the 30 mm cage's rods. |
| `motion-parts.FCStd` | An MGN12 rail and MGN12H carriage, with its travel, and a NEMA 11 lead-screw stepper | The rail gives the changer and the filter slider their 80 mm of travel, with 5 mm at each end. |
| `led-matrix.FCStd` | The Adafruit 2734 | Some LEDs lie outside every objective's NA, for darkfield. |
| `electronics.FCStd` | The SKR Pico, the Pico 2, four LDD-H drivers and the LRS-150-24 supply | The supply covers the peak draw. |

The optical layout now uses the objectives, the tube lens's housing and the LED matrix from these models.

**What C03 changed in the design.**

- **The LED matrix.** Adafruit's page gives the 2734 as 80 mm square, with 5 mm LEDs 5 mm apart: a 10 mm pitch, not 5 mm. At 40 mm below the sample, no LED would fall inside the 5x's NA. At 80 mm, the LEDs fall into the same NAs the design chose, 4, 16 and 44 for the three objectives. The matrix now sits 80 mm below the sample, under the stage, instead of on a post in its opening.
- **The stage's opening.** That post wouldn't have fitted anyway: the opening moves with the stage, and at a plate's outer columns its edge comes within 8 mm of the axis. The same edge can clip the outer LEDs' light, which C06 checks.
- **The changer's rail.** A 120 mm rail with an MGN12H carriage gives only 74.6 mm of travel, short of the 80 mm the changer needs. The design now uses a 150 mm rail.
- **The objectives.** Measured against the 95 mm parfocal length, Mitutoyo's catalog drawings show a 34 mm knurled ring on a 31.5 mm barrel. Edmund Optics lists 30 mm. The models keep 34 mm, the larger.

**Sources and what's assumed.**

- **Thorlabs.** Its product pages render in the browser, so the cage cubes' sizes, the mounted LEDs' length, the TTL200-A's diameter and the cage plate's thickness are assumed. Thorlabs publishes them all. The MFF102/M comes from the drawing in its manual. The TTL200-A's 28 mm length and the mounted LEDs' 30.5 mm diameter come from Thorlabs' tables.
- **Chroma.** The filters' thickness.
- **The boards.** The SKR Pico's holes follow the Raspberry Pi's pattern, as a retailer says. The parts' heights on each board are assumed.

## 9. C04: the optical head

`optical-head.FCStd` holds the head's custom parts, with the bought parts in place, in the instrument's frame. The moving parts are shown with the 10x and the open cell on the axis, and a group shows them at their other stops.

| Part | What it does |
|---|---|
| Head plate | A 15 mm plate at the back that everything else hangs from. The column meets its back face, 185 mm behind the axis. |
| Lower deck | Carries the changer's rail, its motor on the left, the detent and the home switch. |
| Changer plate | Holds the objectives in M26 threads, 40 mm apart, on an MGN12H. A nut block on its back edge takes the lead screw, and three V-grooves in that edge take the detent's ball. |
| Upper deck | The tube lens screws into an SM2 thread through it. The slider hangs under it on an inverted rail, with its motor at the back, and the fold mirror's mount and the cameras sit on it. |
| Slider's carrier | A block with three 30 mm cells, open below and to the right, under a flange that the guide's carriage carries. Cell (a) holds the exciter, the dichroic and the emitter, and (c) the 50:50 plate. |
| Fold mirror's mount | A block behind the mirror, whose 45° face holds it. |
| Camera port's bracket | The MFF102/M hangs from it, upside down. |
| Camera mounts | A base on the upper deck, and a focus plate under the camera that slides on a key, with ±3 mm of travel and a block for the focus screw. |
| Epi plate | Carries the epi arm's 30 mm cage: the field stop's plate, the fold's cube, the combiners' cubes, the collectors' tubes and the LEDs. The epi lens sits in an SM1 lens tube that reaches from the first cage plate over the changer. |

Its checks:

- The fixed parts don't overlap each other.
- The moving parts clear everything: the changer and the slider every 10 mm of their travel, the camera port every 15° of its swing, and the cameras over their focus travel.
- The light clears the head's mechanical parts, with every objective and every slider cell on the axis, and with the camera port in both positions. The excitation clears them with cells (a) and (c) on the axis.
- The detent's ball seats in a groove at each objective, and rides on the plate's edge between them.
- The carriages stay on their rails, 5 mm from each end, and the lead screws, 110 and 160 mm, reach their nuts across the travel.
- The tube lens takes 12.8 mm of the deck's thread.

The head spans x −150 to 255 mm, y −84 to 255 mm, and z 60 to 345 mm above the sample. The imaging light isn't stored in its file, to keep the file small: the checks use the optical layout's.

**What C04 changed in the design.**

- **The camera port hangs upside down, and the colour camera moves to the right.** Standing on its base, the MFF102/M's body reaches 104.5 mm below its mirror, into the filter slider's travel, which runs 100 mm each side of the axis. Hung upside down, its body is above the light. With the colour camera on the left, its swing would then pass over the fold mirror's mount; with the colour camera on the right, it swings up and back, toward the mono camera.
- **The flip mirror sits 42 mm after the fold mirror, not 50.** That gives the swing 6.8 mm of room past the mono camera, or 3.8 mm with the camera focused 3 mm toward it. Each camera's sensor is now 74.8 mm past the flip mirror.
- **The slider and the epi arm sit 5 mm higher.** The filter cube's centre is 130 mm above the sample, 8 mm above the changer instead of 3, so the changer passes 7.75 mm under the epi lens's tube. The infinity space stays 70 mm.
- **The combiner's cubes are 50 mm apart, and the epi fold mirror 30 mm past the field stop.** The 30 mm cage cubes are 41.9 mm across, so a 40 mm pitch made them overlap, and 25 mm left no room for the stop's cage plate.
- **The cameras focus on slides,** not with lens tubes and C-mount spacers.

Also, the models' dichroics and plates now lie with their length in the plane the light turns in. Before, each was turned 45° in its own plane, and a combiner's corner stuck out of its cube.

**Still assumed.**

- The plates' and decks' thicknesses, 15 mm, and that the decks are stiff enough hanging from the head plate. The upper deck reaches 260 mm in front of it.
- That the MFF102/M works upside down. Check it with Thorlabs.
- The ball plunger, an M8 with a 4 mm ball, and the grooves' 3 mm depth.
- The home switches' size.
- The lead screws, cut to 110 and 160 mm.

**For C05.** The column meets the head plate's back face: y = 185 mm, x −150 to 145 mm, z 60 to 345 mm. It has to stay behind the XY stage's sweep, which reaches y = 177 mm at the stage's height. The epi arm passes the head plate's right edge and reaches y = 255 mm, so behind the head plate the column has to stay left of x = 145 mm.

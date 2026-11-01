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
  - the rebuilt `models/`, `REPORT.md` and, since C08, `outputs/`;
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
| C09 | Screw holes | M | The screws that join the parts, and the holes they need, so the drawings are ready to order from. Added after C08, at the owner's request (2026-11-01). |

## 5. Status

| Phase | Status |
|---|---|
| C01 | Done |
| C02 | Done |
| C03 | Done |
| C04 | Done |
| C05 | Done |
| C06 | Done |
| C07 | Done |
| C08 | Done |
| C09 | Done |

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

## 10. C05: the stand and the sample inserts

`stand.FCStd` holds the stand in the instrument's frame, with a slide in focus and the stage centred, and the optical head's plate and decks for reference. `sample-inserts.FCStd` holds the adapter and the three inserts, side by side, with their labware.

| Part | What it does |
|---|---|
| Base plate | 465 × 425 × 12 mm, on four rubber feet. Its top is 221 mm below the sample. The lift stands on it under the axis, and the column at the back. |
| Column | Two 40 × 80 mm extrusions, 566 mm long and 220 mm apart, 80 mm deep. The head plate is bolted to their front faces, 185 mm behind the axis. |
| Plate on the lift | 160 × 110 × 15 mm, held to the lift's top by four counterbored M6 screws under the matrix's mount |
| Spacers | Four, 20 mm across and 35 mm tall. A screw from under the plate goes through each into one of the XY stage's M6 holes. |
| The matrix's mount | A 100 mm square, 4 mm plate that the flexible matrix is stuck to. It puts the LEDs 80 mm below the sample. |
| Insert adapter | A 6 mm plate as big as the top plate. Tabs under it sit in the top plate's pockets, and four M3 screws hold it. An insert sits flush in its recess, over a 140 × 100 mm opening. |
| Inserts | 3 mm plates. Four slides lie along Y, 27.5 mm apart, on ledges under their ends over one window. A microplate's skirt sits in a pocket. A 60 mm and two 35 mm dishes each sit on a 4 mm ledge round a window. |
| Overview camera's bracket | A flange on the upper deck's front, a leg down 12 mm in front of the filter slider's travel, and a plate that holds the camera's board on four standoffs. The camera sits 125 mm in front of the axis and 105 mm above the sample, tilted 35° from straight down. |

The stand's checks:

- The lift brings every sample to the focal plane: a slide with the lift 20 mm up its 25 mm travel, a microplate at 18 mm, the dishes at 20.
- The fixed parts don't overlap each other, and nor do the stage stack's.
- The stage stack clears the fixed parts with the lift at 0, 20 and 25 mm, and the stage at its centre and at each corner of its travel.
- The column meets the head plate's back face, within its width. The stage's sweep reaches y = 177.4 mm, 7.6 mm in front of it.
- The lift carries the stage stack: 5.1 kg with a 0.3 kg sample, of its 8 kg.
- With the stage at the front of its travel, the overview camera sees the whole insert, its corners 87 % of the way to the field's edge. Its view passes under each objective.
- The overview camera and its bracket clear the head's parts, with the changer and the slider every 10 mm of their travel.

The inserts' checks:

- The adapter sits in the top plate's pockets, each insert in the adapter, and the labware in its pockets.
- The stage brings every well, every slide's cover glass and each dish's middle 20 mm into the 5x's field: within ±52.9 × ±52.3 mm of the centre.
- The windows let the matrix's light reach every target. Light from the whole matrix to a point spreads 0.47 mm for every millimetre below it.

**What C05 changed in the design.**

- **The height.** The matrix's 80 mm below the sample, its mount and plate, the lift and the base take 253 mm below the sample. With the head's 345 mm above it, the stand is 598 mm tall, not about 430 mm.
- **The column** is two 40 × 80 mm extrusions rather than a ribbed plate.
- **The overview camera** hangs from the upper deck, in front of the filter slider. With the stage at the front of its travel, it looks at the insert from the front, under the objectives.
- **The slides lie along Y,** 27.5 mm apart over one window. A wider pitch would put the outer cover glasses out of the stage's reach.
- **A counting chamber** needs its own insert, unless it fits a slide's 25.4 × 75.4 mm pocket.

**Still assumed.**

- The feet: 40 mm across and 20 mm high.
- The labware's heights: a microplate's wells' floor 3.5 mm above its base, and a dish's floor 1.5 mm above its base.
- The sample's mass, 0.3 kg, and the 20 mm middle of a dish that's imaged.
- That the column is stiff enough: it stands 566 mm tall, and the head hangs in front of it.
- The overview camera itself, as in C02.

**For C06.**

- **The labware against the objectives.** Above the insert's top, the samples' planes are 0 mm for slides and dishes and 2 mm for a microplate. Above their planes, slides reach 0.17 mm, the 60 mm dish 13.5 mm and a microplate 10.85 mm. `sample-inserts.FCStd` gives these as facts.
- **The matrix's light against the stage's openings,** as C03 noted.
- **The envelope.** The epi arm reaches x = 255 mm, inside the base's 265 mm edge. The overview camera's plug reaches y = −156 mm, inside its −160 mm edge.

## 11. C06: the instrument

`instrument.FCStd` holds the whole instrument. The optical head's parts are under **Optical head** and the stand's under **Stand**, in the same groups as in their own documents, and boxes show the space that the stage stack, the changer and the slider sweep. The same scripts build all three documents, so they always agree.

Its checks:

- The head and the stand don't overlap, with the head's moving parts across their travel.
- **Each insert and its labware clear the head and the overview camera.** The lift is at 0 mm, at its focus for that labware, and at the top of its travel; the stage is at its centre and at each corner; the changer and the slider are swept every 10 mm. The closest anything comes is the 60 mm dish's rim: with the lift at the top of its travel, it's 1.5 mm below the 20x's front.
- **The stage's openings pass the brightfield light** of the 5x and the 10x at every target, and of the 20x at every target's centre. That's 72 stage positions, each with its insert in place, with the light traced from each LED to the field's centre.
- **Everything stays over the base plate,** the moving parts across their travel. What stands on the base reaches x −150 to 255 mm and y −156 to 265 mm; the base spans −200 to 265 and −160 to 265.

| Fact | Value |
|---|---|
| Envelope | 465 × 425 × 598 mm, from the table to the head's top |
| Mass, without the enclosure and the electronics | 30.4 kg: 17.2 kg of custom aluminium parts and 13.2 kg of bought ones, with 3.2 kg of extrusions |

**What C06 found.**

- **The instrument weighs about 30 kg, not 18,** before the enclosure and the electronics. The heaviest custom parts are the base plate (6.4 kg), the head plate (3.4 kg), and the upper and lower decks (1.8 and 1.5 kg). The design document's §14 asks whether to lighten them before C08 draws them.
- **The stage's opening blocks the matrix's steepest light near the ends of the X travel.** There the top plate's opening edge comes within 6.5 mm of the axis, 21 mm below the sample. So it blocks light steeper than NA 0.30 from the far side. The 20x's brightfield light is whole within ±48.3 mm of the centre along X. At the ends of the travel the 20x loses up to 8 of its 44 brightfield LEDs, and 16 of the 64 LEDs are blocked in all. That affects the outer 4 mm of the outer slides' cover glasses and a microplate's outer columns, where the 20x is only for dry samples anyway. The design document's §5.1 says so.

**Still assumed.**

- The bought parts' masses, except the stages', the mono camera's and the 10x's. `params.py` lists them.
- The light is traced to the field's centre. The field's edges, up to 1.9 mm away with the 5x, see slightly different angles.
- Lids aren't modelled. With only 1.5 mm between the 20x and a 60 mm dish at the top of the lift's travel, keep the lift lower for a lidded dish.

**For C07.** The enclosure goes round 465 × 425 × 598 mm. The column's back is flush with the base's back edge, and the epi arm's LEDs reach x = 255 mm, 10 mm inside the base's right edge.

## 12. C07: the enclosure and the electronics

C07 adds the enclosure and the electronics to `instrument.FCStd`, under **Enclosure**. The cover's panels are see-through in the model.

| Part | What it does |
|---|---|
| Cover | A frame of 20 × 20 mm extrusions, 548 × 513 × 640 mm outside, with 1.5 mm black-anodized aluminium panels. It stands on the table over the instrument, apart from it, on a 5 mm foam strip, and lifts off. Inside, it leaves 5 mm round the instrument at least. |
| Door | The whole front, hinged on the left, with a handle by its right edge |
| Interlock | A switch on a bracket from the right front upright, whose plunger the shut door presses |
| Intake | 250 × 40 mm, low in the right panel, with a baffle inside |
| Fan | 120 mm, high in the back panel behind the mono camera, blowing out, with a baffle inside. An opening in the head plate lets the camera's own fan blow straight to it. |
| Electronics tray | On standoffs on the base plate, right of the lift and under the stage. It holds the supply, the SKR Pico, the Pico 2 on a carrier, and the four LDD-H drivers on another. |
| Cable ducts | One along the base plate from the tray, and one up behind the head plate, between the column's extrusions. Two slots in the head plate, by the decks, open into it. |
| The cables' room | 40 mm behind each epi LED for its cable |

Its checks, in the instrument model:

- **The cover clears the instrument,** its cables' room and its moving parts across their travel, and lifts straight off, with 5 mm round them at least.
- **The door swings 120 degrees clear of the cover.** It presses the interlock only when shut: 0.34 degrees open releases it.
- **The intake and the fan are baffled:** no straight line within 72 degrees of their axis gets past.
- **The electronics tray and the cable ducts** clear the instrument and each other, with the stage stack across its travel.

| Fact | Value |
|---|---|
| Cover, outside, the door shut | 548 × 513 × 640 mm |
| Air's warming inside, with the fan | 3.0 K at the typical 40 W, 7.5 K at the 100 W peak |
| Cover | 10.0 kg: about 3 kg of frame and 7 kg of panels |
| Electronics and their tray | 1.4 kg |
| The instrument in its enclosure | 41.6 kg |

**What C07 changed in the design.**

- **The enclosure is a cover,** standing on the table apart from the instrument, rather than panels on its frame. It lifts off for service, and its fan's vibration doesn't reach the instrument.
- **A fan, not just vents.** Natural convection through vents small enough to baffle would let the air inside warm 10 K or more. So a fan blows out behind the mono camera, which dissipates the most heat, up to 36 W.
- **The head plate gets three openings:** two slots for the cables and one behind the mono camera's fan.
- **The design document's §2, §8, §9, §12 and §14** give the enclosure and the new mass. The lid interlock is now a door interlock.

**Still assumed.**

- The fan's flow through the baffles, 40 m³/h, and so the warming. The baffles' mounts aren't modelled.
- The room each LED's cable needs, and the electronics' and the wiring's mass.
- The cables to the moving stage, which need slack for its travel, aren't modelled. They run short from the tray, which sits under the stage.

## 13. C08: manufacturing outputs

[`cad/optima/outputs/`](../../cad/optima/README.md#making-the-parts) holds what a workshop or a supplier needs. The same scripts as the models build it.

| Output | What it holds |
|---|---|
| `drawings/` | A PDF drawing of each of the 32 custom parts: 15 for the head, 5 for the stand, 4 for the inserts and 8 for the enclosure |
| `step/` | A STEP file of each, as modelled, moved so that its box starts at the origin |
| `bill-of-materials.csv` | The 32 custom parts, with their quantities, materials and sizes, and 33 lines of bought parts, with their quantities and the lengths to cut them to |

**The drawings.** Each is a TechDraw page on an A3 sheet of this project's own (`sheet.svg`).

- **Views.** First-angle projection, with an isometric view. Each part is drawn as it sits in the instrument: from above if it's thinnest in Z, otherwise from the front or the right.
- **Scale.** 2:1, 1:1, 1:2, 1:5 or 1:10, whichever fits.
- **Sizes.** The overall sizes only. Take any other size from the STEP file of the same number.
- **Notes.**
  - The general notes: sizes in mm, tolerances to ISO 2768-mK, and sharp edges broken.
  - The part's own, such as its threads.
  - A warning that not every screw hole is modelled yet.
- **Title block.** Number, name, quantity, scale, material and finish.

Its checks:

- Every custom part has a drawing, a STEP file and a row in the bill of materials.
- Each drawing's overall sizes, measured from its views by TechDraw, are its part's.
- Each STEP file reads back as its part: the same solids and volume.

**Reproducible files.**

- The STEP files and the bill of materials are byte for byte the same from build to build.
- The drawings aren't, quite. TechDraw paints a page's items in an order that changes from run to run, and places some text a millimetre apart.
- So `make.py` compares a drawing by what it draws and says, and treats it as changed whenever its part's STEP file changes. It writes only the files that changed.

**Before ordering.**

- **Screw holes.** The holes that join the parts aren't all modelled: between the decks, the brackets and the head plate, for the rails and carriages, and for the motors, among others. The stand's are mostly there. A phase that adds them would make the drawings ready to order from.
- **Weight.** The design document's §14 decision 8, whether to lighten the plates and the panels, changes several of these parts.
- **Materials and finishes.** 6061-T6 for machined parts and 5052 sheet for the panels, all black anodized. These are suggestions.

## 14. C09: screw holes

C08's drawings warned that the screws joining the parts weren't all modelled. C09 adds them, after the owner asked for it on 2026-11-01. The same day the owner accepted the instrument's weight, about 42 kg, so the parts keep their sizes.

**Screws as data.** [`fasteners.py`](../../cad/optima/optima_cad/fasteners.py) describes a socket-head screw:

- its size;
- the point its head seats on;
- the axis it goes in along;
- how far it goes through the parts it clamps;
- how deep it goes into the part it's screwed into, if any;
- those parts' names.

`head.py`, `stand.py` and `enclosure.py` each list their screws, 197 in all: 8 M2.5, 64 M3, 93 M4, 2 M5, 20 M6 and 10 M8. Each part's builder cuts the holes that name it: clearance holes where the part is clamped, counterbores for sunk heads, and tapping holes where a screw goes in. So the models, the drawings and the STEP files all have them.

| Where | Screws |
|---|---|
| Head plate | Six M8 into T-nuts in the column's front slots. Seven M6 from its back into the decks' and the camera port's bracket's back edges, clear of the column and the cable duct. |
| Epi plate | A new angle bracket, H16, screwed to the head plate's front with two M5s; the epi plate sits on it, on two M4s. Six M4s from below hold the cage's cubes and plates. |
| Guides | The rails' M3s sunk in their counterbores, into the decks. The changer's carriage's four M3s are sunk in the changer plate, and the slider's come up through its flange. |
| Motors and brackets | M2.5s through the brackets into the motors. M3s through the lower deck into the changer's bracket and the detent block. Sunk M3s down through the upper deck into the slider's bracket. |
| On the upper deck | M4s from below into the fold mirror's mount, M3s into the camera mounts, and an M6 down into the camera port's base. The overview bracket's flange takes two M4s. |
| Stand | The lift's M6s into the base plate, the column's M8s up into its bores, and the tray's M3s through its spacers. The plate on the lift: sunk M6s into the lift, and M6s up through the spacers into the XY stage. The matrix's mount on four M3s. |
| Enclosure | The panels' M4s into T-nuts in the frame, at most 150 mm apart. The door's hinge and handle. The baffles on spacers, the fan, and the supply and the boards on the tray. |

Its checks, in the instrument model:

- **Every screw clears the parts.** The screws on fixed parts also clear the moving parts across their travel.
- **Every screw's head bears on its part, and its thread has material all round.** At least 1 mm round each tapping hole.

**What else C09 changed.**

- **Drawings.** There are 33 now, with H16. Each one's thread and counterbore notes come from the screws, such as "Tapped: 3 x M6 x 12 deep". The general note now says the holes are clearance holes unless a note taps them.
- **Bill of materials.** It counts the screws by size and length, and the T-nuts, nuts and spacers they need.
- **Parts.**
  - The guides' rails are counterbored, as MGN12 rails are.
  - The slider's nut block is 12 mm long, not 16, so the carriage's screws clear it.
  - The tray's spacers are plain spacers on screws into the base plate.
- **Build time.** The builders now keep their results for the rest of a run, so the build still takes about a minute.

**Still assumed.**

- **Bought parts' holes.**
  - An M6 at the middle of the camera port's base.
  - Two M3s, 110 mm apart, in the supply's base.
  - An M4 at the bottom of each cage cube and plate.
  - Corner holes in the boards' carriers.

  Check each against the part before ordering.
- **Not modelled.** The home switches' and the interlock's screws, and the lead nuts' set screws.
- **Strength.** The upper deck hangs 260 mm in front of the head plate on two M6s and its fit against the plate; that's a fit to check on the first build.

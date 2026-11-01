# OPTIMA CAD build report

Written by `make.py` with FreeCAD 1.1. Don't edit it: the build replaces it.

## optical-layout

| Check | Result | Detail |
|---|---|---|
| Infinity space, 5x: corners clear the tube lens's pupil | pass | 70 mm of 76.6 mm allowed |
| Infinity space, 10x: corners clear the tube lens's pupil | pass | 70 mm of 76.6 mm allowed |
| Infinity space, 20x: corners clear the tube lens's pupil | pass | 70 mm of 101.0 mm allowed |
| Sensor within the tube lens's rated field | pass | 0.41 % of the sensor is outside the 22 mm field |
| Fold mirror: light within its clear aperture | pass | 12.3 mm from the centre, of 22.9 mm |
| Flip mirror: light within its clear aperture | pass | 14.2 mm from the centre, of 22.9 mm |
| Nothing collides | pass | no overlaps |
| Stack along the axis: changer, slider, tube lens | pass | 95.0 < 107.0 < 115.0 < 145.0 < 145.0 |
| LED image fits the smallest back aperture | pass | 5.0 mm in 8.4 mm |
| LED matrix: LEDs inside each NA match the design | pass | 5x 4, 10x 16, 20x 44 |

| Fact | Value |
|---|---|
| Objective shoulder, z | 95.0 |
| Objective exit pupil, z (assumed) | 75.0 |
| Changer carriage top, z | 107.0 |
| Filter cube centre, z | 130.0 |
| Tube lens entrance pupil, z | 145.0 |
| Tube lens back, z | 193.2 |
| Fold mirror centre, z | 228.2 |
| Mono sensor centre, x y z | 0.0 116.8 228.2 |
| Colour sensor centre, x y z | 74.8 42.0 228.2 |
| Epi lens, field stop, epi fold: x | 45.0 145.0 175.0 |
| Field stop diameter | 11.48 |
| LED image diameter | 5.0 |
| Corner angle in the infinity space, degrees | 3.29 |

## xy-stage

| Check | Result | Detail |
|---|---|---|
| Footprint with the motors, x | pass | 276.50 mm, the drawing says 276.5 |
| Footprint with the motors, y | pass | 276.50 mm, the drawing says 276.5 |
| Height | pass | 45.00 mm, the drawing says 45 |
| Clear aperture at the centre | pass | 115.00 mm, the drawing says 115 |
| Clear aperture at a corner of the travel | pass | 64.00 mm, the drawing says 64 |
| The travel covers a 96-well plate's well centres | pass | 102 mm of travel, 99 x 63 mm of wells |
| The moving parts clear the fixed ones across the travel | pass | no overlaps at the centre or the corners |

| Fact | Value |
|---|---|
| Top surface, z | 45.0 |
| Travel, each axis | 102 mm, +-51 |
| Footprint swept over the travel, x | -138.5 to 150.1 |
| Footprint swept over the travel, y | -150.1 to 177.4 |
| Mass, kg | 3.6 |

## z-lift

| Check | Result | Detail |
|---|---|---|
| Length with the connector | pass | 140.00 mm, the drawing says 140 |
| Height at the bottom of the travel | pass | 100.00 mm, the drawing says 100 |
| Height at the top of the travel | pass | 125.00 mm, the drawing says 125 |
| The top clears the column over the travel | pass | no overlaps |

| Fact | Value |
|---|---|
| Base | 100 x 115 mm |
| Top plate | 100 x 100 mm, 17 M6 holes |
| Travel | 25.0 |
| Load, kg | 8.0 |
| Mass, kg | 2.2 |

## mono-camera

| Check | Result | Detail |
|---|---|---|
| The image corners' light clears the C-mount's bore | pass | 11.32 mm from the axis at the lens's seat, of 12.00 mm |

| Fact | Value |
|---|---|
| Envelope without the plugs, x y z | 80.0 x 80.0 x 61.4 |
| Lens seat to sensor | 17.526 |
| Lens seat to the camera's back | 61.4 |
| Mounting holes | 2 x M4 20 mm apart, a 1/4-20 between, 24 mm behind the front |
| Mass, kg | 0.397 |

## colour-camera

| Check | Result | Detail |
|---|---|---|
| The image corners' light clears the C-mount's bore | pass | 11.32 mm from the axis at the lens's seat, of 12.00 mm |

| Fact | Value |
|---|---|
| Envelope without the plugs, x y z | 68.0 x 68.0 x 46.2 |
| Lens seat to sensor | 17.526 |
| Lens seat to the camera's back | 46.2 |
| Mounting hole | 1/4-20, centred (assumed) |

## overview-camera

| Check | Result | Detail |
|---|---|---|
| The lens's image circle covers the sensor | pass | 7.90 mm circle, 7.86 mm diagonal |

| Fact | Value |
|---|---|
| Field of view, degrees: horizontal, vertical, diagonal | 89.0 72.7 101.7 |
| Lens front, z | -22.0 |

## objectives

| Check | Result | Detail |
|---|---|---|
| 5x: length and working distance make the parfocal length | pass | 95.00 mm, the drawing says 95 |
| 10x: length and working distance make the parfocal length | pass | 95.00 mm, the drawing says 95 |
| 20x: length and working distance make the parfocal length | pass | 95.00 mm, the drawing says 95 |
| Neighbours on the changer's pitch don't touch | pass | 6 mm between their knurled rings |

| Fact | Value |
|---|---|
| Barrel, knurled ring | 31.5 mm, 34 mm across |
| Thread | M26 x 0.706, 7.4 mm long |

## imaging-optics

| Check | Result | Detail |
|---|---|---|
| Position 2 takes the ring clear of the light | pass | the ring 49.1 mm from the light's centre; the light reaches 15.3 mm |
| The tube lens's housing clears the filter slider | pass | housing from z 165.2, slider up to 145.0 |

| Fact | Value |
|---|---|
| Tube lens housing, z | 165.2 to 193.2 |
| Camera port: the mirror's centre moves | 77.1 mm |

## epi-optics

| Check | Result | Detail |
|---|---|---|
| The dichroic fits the cage cube's holder | pass | 35.6 x 25.2 x 1 mm in 36 x 26 x 3 mm |
| A mounted LED fits between the 30 mm cage's rods | pass | 30.5 mm across, 36.4 mm between the rods |

| Fact | Value |
|---|---|
| LED housing | 30.5 mm across, 41 mm long |

## motion-parts

| Check | Result | Detail |
|---|---|---|
| The rail gives the changer and the slider their travel | pass | 104.6 mm on a 150 mm rail; they need 90 mm with margins |

| Fact | Value |
|---|---|
| Carriage travel | 104.6 |

## led-matrix

| Check | Result | Detail |
|---|---|---|
| Some LEDs lie outside every objective's NA, for darkfield | pass | 20 LEDs between NA 0.42 and 0.53 |

| Fact | Value |
|---|---|
| Pitch, height below the sample | 10 mm, 80 mm |

## electronics

| Check | Result | Detail |
|---|---|---|
| The supply covers the peak draw | pass | 156 W for about 100 W |

| Fact | Value |
|---|---|
| Supply | 159 x 97 x 30 mm |

## optical-head

| Check | Result | Detail |
|---|---|---|
| The fixed parts don't overlap each other | pass | no overlaps |
| The moving parts clear everything across their travel | pass | the changer and the slider every 10 mm, the port every 15 degrees, the cameras over +-3 mm of focus |
| The light clears the head's parts | pass | with every objective and every slider cell on the axis |
| The detent's ball seats in a groove at each objective, and rides on the edge between | pass | a 4 mm ball, 3 mm-deep 90-degree grooves |
| The carriages stay on their rails, 5 mm from each end | pass | +-40 mm of +-52.3 mm |
| The lead screws reach the nuts across the travel | pass | the changer's 110 mm screw, the slider's 160 mm |
| The tube lens takes at least 5 mm of the deck's thread | pass | 12.8 mm |

| Fact | Value |
|---|---|
| Head plate's back face, y: the column meets it | 185.0 |
| Lower deck's top, z | 82.0 |
| Upper deck, z | 163.0 to 178.0 |
| Changer's offsets, 5x 10x 20x | +40 +0 -40 |
| Slider's offsets, cells a b c | +40 +0 -40 |
| Camera port's base, z (upside down) | 332.7 |
| Focus travel, each camera | +-3 mm |
| The head, x | -150.0 to 255.0 |
| The head, y | -90.0 to 255.0 |
| The head, z | 60.0 to 345.0 |

## stand

| Check | Result | Detail |
|---|---|---|
| The lift brings every sample to the focal plane | pass | slides at 20 mm, microplate at 18 mm, dishes at 20 mm of its 25 |
| The fixed parts don't overlap each other | pass | no overlaps |
| The stage stack's parts don't overlap each other | pass | no overlaps |
| The stage stack clears the fixed parts across the lift's and the stage's travel | pass | the lift at 0, 20, 25 mm, the stage at its centre and corners |
| The column meets the head plate's back face, behind the stage's sweep | pass | its front at y = 185, x -130 to 130; the stage reaches y = 177.4 |
| The lift carries the stage stack | pass | 5.1 kg with a 0.3 kg sample, of 8 kg |
| The overview camera sees the whole insert, the stage at the front of its travel | pass | its corners reach 87 % of the way to the field's edge |
| Its view of the insert passes under the objectives | pass | with each objective on the axis |
| The overview camera and its bracket clear the head's parts | pass | the changer and the slider every 10 mm; the bracket is one piece |

| Fact | Value |
|---|---|
| Base plate | 465 x 425 x 12 mm, its top at z = -221.1 |
| Plate on the lift, z | -101.1 to -86.1 |
| Matrix's LEDs, z | -80.0 |
| Spacers' height | 35.1 |
| XY stage's underside, z | -51.0 |
| Column | 2 x 40 x 80 mm extrusions, 566.1 mm long, at x = +-110, y 185 to 265 |
| Table to the head plate's top | 598.1 |
| Stage stack's mass, kg | 5.14 |
| Overview camera's sensor | 0, -125, 105 |
| Overview camera's view | 35.2 degrees from straight down, 125 mm to the insert's centre |

## sample-inserts

| Check | Result | Detail |
|---|---|---|
| The adapter sits in the top plate's pockets, each insert in the adapter, and the labware in its pockets | pass | no overlaps |
| The travel brings every well, cover glass and dish's middle into the 5x's field | pass | all within +-52.9 x +-52.3 mm of the centre |
| The windows let the matrix's light reach every target | pass | its light spreads 0.47 mm per mm below a target |

| Fact | Value |
|---|---|
| Adapter | 175 x 175 x 6 mm, an insert's recess 3 mm deep, an opening 140 x 100 mm |
| Inserts | 160 x 110 x 3 mm |
| Reach into the 5x's field, from the centre | +-52.9 x +-52.3 mm |
| Four-slide insert: the sample plane above the insert's top, the labware's top above it | 0.00, 0.17 mm |
| Microplate insert: the sample plane above the insert's top, the labware's top above it | 2.00, 10.85 mm |
| Dish insert: a 60 mm and two 35 mm dishes: the sample plane above the insert's top, the labware's top above it | 0.00, 13.50 mm |

## instrument

| Check | Result | Detail |
|---|---|---|
| The head and the stand don't overlap, the head's moving parts across their travel | pass | no overlaps |
| Each insert and its labware clear the head across the stage's and the lift's travel, and the objectives as they change | pass | the objectives' fronts 1.5 mm above the dishes with the lift at the top of its travel |
| The stage's openings pass the brightfield light of the 5x and the 10x at every target, and of the 20x at every target's centre | pass | 72 stage positions, to the field's centre |
| Everything stays over the base plate, the moving parts across their travel | pass | x -150.0 to 255.0, y -156.1 to 265.0; the base -200 to 265, -160 to 265 |
| The cover clears the instrument, its cables' room and its moving parts across their travel, and lifts straight off | pass | 5.0 mm round it at least, inside the frame |
| The door swings open clear of the cover, and presses the interlock only when shut | pass | it swings 120 degrees; 0.34 degrees open releases the switch |
| The intake and the fan are baffled against light | pass | no straight line within 72 degrees of their axis gets past the baffles |
| The electronics tray and the cable ducts clear the instrument, the stage stack across its travel | pass | no overlaps |

| Fact | Value |
|---|---|
| Envelope, x y z | -200.0 to 265.0, -160.0 to 265.0, -253.1 to 345.0 |
| Envelope, mm | 465 x 425 x 598 |
| The 20x's brightfield light is whole | within +-48.3 mm of the centre along X |
| At the worst target, the 20x's brightfield LEDs blocked | 8 of 44 |
| At the worst target, all LEDs blocked | 16 of 64, from NA 0.30 |
| Mass without the enclosure and the electronics, kg | 30.3 |
| Custom aluminium parts, kg | 17.1 |
| Heaviest custom parts, kg | the base plate 6.4, the head plate 3.3, the upper deck 1.8, the lower deck 1.5 |
| Bought parts, kg | 13.2, with 3.2 for the column's extrusions |
| Mass with the enclosure and the electronics, kg | 41.6 |
| Cover, outside, the door shut | 548 x 513 x 640 mm |
| Intake; fan | 100 cm2; 120 mm, 40 m3/h through the baffles |
| Air's warming inside, typical and peak draw | 3.0 K at 40 W, 7.5 K at 100 W |
| Cover, kg | 10.0 |
| Electronics and their tray, kg | 1.4 |

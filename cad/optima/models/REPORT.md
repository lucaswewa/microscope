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
| Flip mirror: light within its clear aperture | pass | 14.3 mm from the centre, of 22.9 mm |
| Nothing collides | pass | no overlaps |
| Stack along the axis: changer, slider, tube lens | pass | 95.0 < 107.0 < 110.0 < 140.0 < 145.0 |
| LED image fits the smallest back aperture | pass | 5.0 mm in 8.4 mm |
| LED matrix: LEDs inside each NA match the design | pass | 5x 4, 10x 16, 20x 44 |

| Fact | Value |
|---|---|
| Objective shoulder, z | 95.0 |
| Objective exit pupil, z (assumed) | 75.0 |
| Changer carriage top, z | 107.0 |
| Filter cube centre, z | 125.0 |
| Tube lens entrance pupil, z | 145.0 |
| Tube lens back, z | 193.2 |
| Fold mirror centre, z | 228.2 |
| Mono sensor centre, x y z | 0.0 116.8 228.2 |
| Colour sensor centre, x y z | -66.8 50.0 228.2 |
| Epi lens, field stop, epi fold: x | 50.0 150.0 175.0 |
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
| Position 2 takes the ring clear of the light | pass | the ring 49.1 mm from the light's centre; the light reaches 15.4 mm |
| The tube lens's housing clears the filter slider | pass | housing from z 165.2, slider up to 140.0 |

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

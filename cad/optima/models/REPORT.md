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

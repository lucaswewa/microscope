"""Every dimension the models use, in millimetres unless the name says otherwise.

The values come from docs/misc/optima_hardware_design.md and the vendor pages in its §16.
"Assumed" marks a value no vendor publishes, to be measured in the design's build step 1.
"""


def _grid(xs, ys):
    return tuple((x, y) for x in xs for y in ys)


# Both cameras' sensor: the Sony IMX492 (ToupTek's product pages).
SENSOR_PIXEL = 0.002315
SENSOR_PIXELS = (8176, 5616)
SENSOR_SIZE = (18.93, 13.00)
C_MOUNT_FLANGE = 17.526  # the lens's seat to the sensor, the C-mount standard
C_MOUNT_BORE = 24.0  # assumed: a little inside the 1"-32 thread's 24.5 mm minor diameter

# Mono camera: ToupTek CTR3CMOS45000KMA (ToupTek's drawing).
MONO_BODY = (80.0, 80.0, 45.5)  # width, height, depth
MONO_NOSE = (60.0, 7.9)  # the C-mount boss: diameter (assumed, from the picture), length
MONO_FAN = (45.0, 45.0, 8.0)  # the depth is assumed: the drawing's label is blurred
MONO_MOUNT = (24.0, 20.0)  # 2 x M4 and a 1/4-20 between them: their line from the front, M4 pitch
MONO_MASS = 0.397  # kg

# Colour camera: ToupTek E3ISPM45000KPB, a colour IMX492 (ToupTek's E3ISPM drawing).
COLOUR_BODY = (68.0, 68.0, 40.5)
COLOUR_NOSE = (40.0, 5.7)  # the C-mount ring: diameter (assumed), length (46.2 - 40.5)
COLOUR_EDGE = 6.0  # assumed: the radius of the body's rounded edges

# Overview camera, all assumed until one is bought: a 12 MP IMX577 UVC module on the common
# 38 mm square board, with a wide M12 lens.
OVERVIEW_SENSOR = (6.29, 4.71)
OVERVIEW_BOARD = (38.0, 38.0, 1.6)
OVERVIEW_HOLES = 34.0  # M2, on a square
OVERVIEW_HOLDER = (16.0, 16.0, 12.0)
OVERVIEW_LENS = (14.0, 18.0)  # barrel diameter, length
OVERVIEW_LENS_F = 3.2
OVERVIEW_IMAGE_CIRCLE = 7.9  # a 1/2.3-inch lens

# XY stage: Standa 8MTF-102LS05 (Standa's drawing). Its frame: the origin at the centre of the
# base's underside, +X and +Y as in the instrument. The Y drive is fixed to the base on the left;
# the X drive rides on the middle plate, at the back.
XY_PLATE = 175.0  # the base, middle and top plates are this square
XY_HEIGHT = 45.0  # the plates are assumed equally thick
XY_OPENING = (115.0, 10.0)  # through every plate: side, corner radius
XY_POCKETS = (20.0, 5.0, 4.0)  # in the top plate's corners: length, width past the opening, depth
XY_TRAVEL = 51.0  # each way from the centre, on both axes
XY_MASS = 3.6  # kg
# The drives at the centre of the travel, as boxes (x0, y0, z0, x1, y1, z1), with each knob's
# start, direction, and its hub's and knob's diameter and length.
XY_Y_DRIVE = ((-117.5, -86.0, 1.0, -88.5, 81.5, 29.0), (-126.4, -132.5, 1.0, -88.5, -86.0, 29.0))
XY_X_DRIVE = ((-81.5, 88.5, 16.0, 86.0, 117.5, 44.0), (86.0, 88.5, 16.0, 132.5, 126.4, 44.0))
XY_Y_KNOB = ((-102.5, -132.5, 15.0), (0, -1, 0))
XY_X_KNOB = ((132.5, 102.5, 30.0), (1, 0, 0))
XY_KNOB = ((16.6, 4.6), (27.5, 13.0))
# The top plate's holes as (diameter, depth or None for through, centres). The base's underside
# has the same pattern, turned 90 degrees.
XY_HOLES = (
    (3.0, 4.0, _grid((-62.5, 62.5), (-62.5, -37.5, -12.5, 12.5, 37.5, 62.5))),
    (6.0, 9.0, _grid((-37.5, -12.5, 12.5, 37.5), (-62.5, 62.5)) + _grid((-12.5, 12.5), (-75, 75))),
    (6.6, None, _grid((-37.5, 37.5), (-75, 75))),
)

# Z lift: Standa 8MVT100-25-1 (Standa's drawing). Its frame: the origin at the centre of the
# base's underside, turned from the drawing so that its connector is at the back (+Y).
LIFT_BASE = (100.0, 115.0, 8.0)
LIFT_COLUMN = (90.0, 90.0)  # assumed: the drawing shows about 91 x 88
LIFT_TOP = (100.0, 100.0, 40.0)  # the hood that carries the top plate
LIFT_HEIGHT = 100.0  # the top plate, at the bottom of the travel
LIFT_TRAVEL = 25.0
LIFT_CONNECTOR = (-25.5, 45.0, 8.0, 5.5, 82.5, 21.0)  # a box, as for the XY stage's drives
LIFT_BASE_HOLES = ((6.6, None, _grid((-37.5, 37.5), (-50, 50))),)
LIFT_TOP_HOLES = ((6.0, 8.0, _grid((-37.5, 37.5), (-37.5, -12.5, 12.5, 37.5))
                   + _grid((-12.5, 12.5), (-37.5, 37.5)) + _grid((-25, 25), (-25, 25)) + ((0, 0),)),)
LIFT_LOAD = 8.0  # kg
LIFT_MASS = 2.2  # kg

# Objectives: Mitutoyo M Plan Apo (Edmund Optics' listings). DOF is the depth of field, and the
# length runs from the shoulder to the front.
OBJECTIVES = {
    "5x": {"na": 0.14, "wd": 34.0, "f": 40.0, "dof": 0.014, "length": 61.0},
    "10x": {"na": 0.28, "wd": 34.0, "f": 20.0, "dof": 0.0035, "length": 61.0},
    "20x": {"na": 0.42, "wd": 20.0, "f": 10.0, "dof": 0.0016, "length": 75.0},
}
CHANGER_ORDER = ("5x", "10x", "20x")
ACTIVE_OBJECTIVE = "10x"  # the one the layout shows on the axis
PARFOCAL = 95.0  # focal plane to the objective's shoulder
# Measured from Mitutoyo's catalog drawings against the 95 mm parfocal length. Edmund Optics
# lists the 10x as 30 mm across at most; the drawings show a 34 mm knurled ring.
OBJECTIVE_DIAMETER = 34.0  # the knurled ring
OBJECTIVE_KNURL = (27.0, 42.0)  # the ring's ends, below the shoulder
OBJECTIVE_BARREL = 31.5
OBJECTIVE_THREAD = (26.0, 7.4)  # M26 x 0.706: diameter, length
PUPIL_DEPTH = 20.0  # assumed: the exit pupil's depth below the shoulder

# Tube lens: Thorlabs TTL200-A (Thorlabs' item page).
TUBE_F = 200.0
TUBE_PUPIL = 20.0  # entrance pupil diameter
TUBE_FIELD = 22.0  # rated field at the image
TUBE_BACK = 151.8  # the lens's back to the image
TUBE_HOUSING = (51.7, 28.0)  # diameter (assumed: the SM2 thread's), length

# The head's stack along the axis, and the folds after the tube lens (this design).
INFINITY_SPACE = 70.0  # objective exit pupil to the tube lens's entrance pupil
CHANGER_PITCH = 40.0
CHANGER_THICKNESS = 12.0
FILTER_GAP = 3.0  # changer carriage to filter slider
FILTER_CUBE = 30.0
FILTER_PITCH = 40.0
FOLD_AFTER_LENS = 35.0  # the tube lens's back to the fold mirror
FLIP_AFTER_FOLD = 50.0
MIRROR_DIAMETER = 50.8
MIRROR_THICKNESS = 12.0
MIRROR_CLEAR = 45.7  # assumed: 90 % of the diameter

# Epi illumination: a Köhler illuminator whose arm folds back toward the column.
EPI_LENS_F = 100.0
COLLECTOR_F = 20.0
LED_DIE = 1.0  # assumed
LEDS_NM = (395, 470, 565, 625)  # the last sits on the combiner's axis
EPI_FOLD_AFTER_STOP = 25.0
COMBINER_PITCH = 40.0
SMALL_OPTIC = 25.4
DICHROIC = (35.6, 25.2, 1.0)  # Chroma 89402's beamsplitter

# Transmitted light: Adafruit 2734, an 8 × 8 APA102 matrix (Adafruit's product page): 5 mm LEDs
# 5 mm apart, 2.5 mm from the board's edge.
MATRIX_COUNT = 8
MATRIX_PITCH = 10.0
MATRIX_BOARD = (80.0, 80.0, 2.1)
MATRIX_LED = 5.0
MATRIX_BELOW_SAMPLE = 80.0  # twice the 5 mm pitch's 40 mm, for the same NAs
MATRIX_NA_GROUPS = {"5x": 4, "10x": 16, "20x": 44}  # LEDs inside each NA, design §5.1

# Sample: a slide with a #1.5 cover glass.
SLIDE = (75.0, 25.0, 1.0)
COVER_GLASS = (22.0, 22.0, 0.17)

# The mirrors (this design): the fold mirror in a 60 mm cage cube (Thorlabs KCB2C/M), the epi fold
# mirror in a 30 mm one (KCB1C/M), and the dichroics in 30 mm cage cubes (CM1-DCH/M). Thorlabs
# publishes the cubes' sizes, but its pages couldn't be read here: they're assumed.
CAGE_60 = (60.0, 6.0, 76.2)  # rod spacing, rod diameter, cube
CAGE_30 = (30.0, 6.0, 41.9)
CAGE_PLATE_30 = 8.9  # thickness, assumed
DICHROIC_HOLDER = (36.0, 26.0, 3.0)  # the CM1-DCH/M's largest optic (Thorlabs)

# The camera port: Thorlabs MFF102/M with its LMR2 ring (the MFF101/MFF102 manual's drawing).
# Its frame: the origin at the optic's centre in position 1, the optic facing -Y, the base below.
FLIP_BODY = (30.0, 70.0, 30.0)  # width, height, depth
FLIP_PIVOT = 50.0  # above the base; position 2 swings the optic 90 degrees about it, to -X
FLIP_OPTIC = 104.5  # the optic's centre in position 1, above the base
FLIP_RING = (56.0, 10.0)  # outer diameter (the drawing's 132.5 mm height), thickness (assumed)

# Epi parts. Thorlabs mounted LEDs (M395L5, M470L5, M565L3, M625L4) share one housing: a heatsink
# 30.5 mm across with a 6 mm-deep SM1 thread in front of the LED (Thorlabs). Its length is assumed.
LED_HOUSING = (30.5, 41.0)
SM1 = 25.4  # the SM1 thread's bore, about 1 inch
FILTER_ROUND = (25.0, 5.0)  # Chroma's exciter and emitter: diameter, thickness (assumed)

# Motion: HIWIN MGN12 guides (HIWIN's catalogue, from its distributors) and NEMA 11 lead-screw
# steppers. A 120 mm rail with an MGN12H gives only 74.6 mm of travel, short of the changer's 80.
MGN12_RAIL = (12.0, 8.0, 25.0, 150.0)  # width, height, hole pitch, length (this design)
MGN12H = (27.0, 45.4, 13.0, 3.0, 20.0)  # width, length, height, clearance under it, M3 holes' square
TRAVEL_MARGIN = 5.0  # each end, for homing and overtravel (this design)
NEMA11 = (28.2, 34.0, 23.0, 22.0)  # face, body length (assumed), M2.5 holes' square, pilot diameter
LEAD_SCREW = (6.0, 150.0)  # assumed: Tr6, as these motors are sold

# Electronics, each from its maker's or distributors' listings. The SKR Pico's holes follow the
# Raspberry Pi's pattern, as a retailer says; the parts' heights over their boards are assumed.
SKR_PICO = (85.0, 56.0, 15.0, (58.0, 49.0))  # width, depth, height, holes' rectangle
PICO2 = (51.0, 21.0, 4.0, (47.0, 11.4))
LDD_H = (31.8, 20.3, 12.2)  # Mean Well LDD-H, pin style
SUPPLY = (159.0, 97.0, 30.0)  # Mean Well LRS-150-24
SUPPLY_W = 156.0
PEAK_W = 100.0  # the design's estimate of the peak draw (its §8)

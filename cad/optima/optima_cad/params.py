"""Every dimension the models use, in millimetres unless the name says otherwise.

The values come from docs/misc/optima_hardware_design.md and the vendor pages in its §16.
"Assumed" marks a value no vendor publishes, to be measured in the design's build step 1.
"""

# Mono camera: ToupTek CTR3CMOS45000KMA (ToupTek's product page).
SENSOR_PIXEL = 0.002315
SENSOR_PIXELS = (8176, 5616)
SENSOR_SIZE = (18.93, 13.00)
CAMERA_BODY = (80.0, 80.0, 45.5)  # width, height, depth
C_MOUNT_FLANGE = 17.526  # flange to sensor, the C-mount standard

# Colour camera: ToupTek colour IMX492. Assumed the mono camera's body until C02 models it.
COLOUR_CAMERA_BODY = CAMERA_BODY

# Objectives: Mitutoyo M Plan Apo (Edmund Optics' listings). DOF is the depth of field.
OBJECTIVES = {
    "5x": {"na": 0.14, "wd": 34.0, "f": 40.0, "dof": 0.014},
    "10x": {"na": 0.28, "wd": 34.0, "f": 20.0, "dof": 0.0035},
    "20x": {"na": 0.42, "wd": 20.0, "f": 10.0, "dof": 0.0016},
}
CHANGER_ORDER = ("5x", "10x", "20x")
ACTIVE_OBJECTIVE = "10x"  # the one the layout shows on the axis
PARFOCAL = 95.0  # focal plane to the objective's shoulder
OBJECTIVE_DIAMETER = 34.0
PUPIL_DEPTH = 20.0  # assumed: the exit pupil's depth below the shoulder

# Tube lens: Thorlabs TTL200-A (Thorlabs' item page).
TUBE_F = 200.0
TUBE_PUPIL = 20.0  # entrance pupil diameter
TUBE_FIELD = 22.0  # rated field at the image
TUBE_BACK = 151.8  # the lens's back to the image

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

# Transmitted light: Adafruit 2734, an 8 × 8 APA102 matrix.
MATRIX_COUNT = 8
MATRIX_PITCH = 5.0
MATRIX_BELOW_SAMPLE = 40.0
MATRIX_NA_GROUPS = {"5x": 4, "10x": 16, "20x": 44}  # LEDs inside each NA, design §5.1

# Sample: a slide with a #1.5 cover glass.
SLIDE = (75.0, 25.0, 1.0)
COVER_GLASS = (22.0, 22.0, 0.17)

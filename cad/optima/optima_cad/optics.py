"""First-order optics: where each element sits, and the numbers the checks use.

Positions are on the unfolded axis: z from the sample's focal plane, up through the objective
to the image the tube lens forms. The tube lens is a thin lens at its entrance pupil.
"""

import math

from . import params as P

SHOULDER = P.PARFOCAL
PUPIL = SHOULDER - P.PUPIL_DEPTH
CARRIAGE_TOP = SHOULDER + P.CHANGER_THICKNESS
FILTER = CARRIAGE_TOP + P.FILTER_GAP + P.FILTER_CUBE / 2
TUBE = PUPIL + P.INFINITY_SPACE
TUBE_BACK = TUBE + P.TUBE_F - P.TUBE_BACK
FOLD = TUBE_BACK + P.FOLD_AFTER_LENS
IMAGE = TUBE + P.TUBE_F

# The epi arm, along +X at the filter cube's height, then folded toward +Y.
EPI_LENS_X = P.EPI_LENS_F - (FILTER - PUPIL)
STOP_X = EPI_LENS_X + P.EPI_LENS_F
EPI_FOLD_X = STOP_X + P.EPI_FOLD_AFTER_STOP


def sensor_diagonal():
    return math.hypot(*P.SENSOR_SIZE)


def back_aperture(objective):
    o = P.OBJECTIVES[objective]
    return 2 * o["na"] * o["f"]


def corner_angle():
    """The angle, in radians, of the light for an image corner in the infinity space."""
    return math.atan(sensor_diagonal() / 2 / P.TUBE_F)


def exit_pupil():
    """How far in front of the image the exit pupil is: the objective's pupil, as the tube lens
    images it."""
    f, gap = P.TUBE_F, P.INFINITY_SPACE
    return f + gap * f / (f - gap)


def corner_light(distance):
    """How far from the axis the image corners' light reaches `distance` in front of the image,
    with the objective whose light converges the most."""
    r = sensor_diagonal() / 2
    na = max(o["na"] * o["f"] / P.TUBE_F for o in P.OBJECTIVES.values())
    return r * (1 - distance / exit_pupil()) + distance * math.tan(math.asin(na))


def max_infinity_space(objective):
    """The longest pupil-to-tube-lens distance at which the image corners aren't vignetted."""
    return (P.TUBE_PUPIL - back_aperture(objective)) * P.TUBE_F / sensor_diagonal()


def field_stop():
    """The field stop that frames the camera's field with every objective."""
    return sensor_diagonal() * P.EPI_LENS_F / P.TUBE_F


def led_image():
    """The LED die's image in the objective's back aperture."""
    return P.LED_DIE * P.EPI_LENS_F / P.COLLECTOR_F


def sensor_outside_field():
    """The fraction of the sensor's area outside the tube lens's rated field."""
    w, h = P.SENSOR_SIZE
    n, outside = 400, 0
    for i in range(n):
        for j in range(n):
            x = (i + 0.5) / n * w - w / 2
            y = (j + 0.5) / n * h - h / 2
            outside += math.hypot(x, y) > P.TUBE_FIELD / 2
    return outside / n**2


def matrix_leds():
    """The LED matrix's LED centres, (x, y), around the optical axis."""
    half = (P.MATRIX_COUNT - 1) / 2
    steps = [(i - half) * P.MATRIX_PITCH for i in range(P.MATRIX_COUNT)]
    return [(x, y) for x in steps for y in steps]


def led_na(x, y):
    return math.sin(math.atan(math.hypot(x, y) / P.MATRIX_BELOW_SAMPLE))


def leds_inside(na):
    return sum(led_na(x, y) <= na for x, y in matrix_leds())

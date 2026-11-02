"""The motion controller's settings: grblHAL on the SKR Pico, worked out from the stages' data.

Each axis's steps per mm come from its lead and microsteps, and its soft limit from the distance
between its switches. Writes the settings as a file that load_settings.py sends to the board, and
checks the speeds, currents, steps and limits against the design.
"""

import math
import os
from collections import namedtuple

from . import params as P
from .checks import Check

Axis = namedtuple("Axis", "lead micro span short top speed accel hold")

ORDER = "XYZA"
AXES = {k: Axis(*v) for k, v in P.MOTION_AXES.items()}
# The changer has one switch, a little past its first position, and stops short of its rail's end.
CHANGER_SPAN = P.MOTION_TRIP + 2 * P.CHANGER_PITCH + P.TRAVEL_MARGIN
AXES["A"] = AXES["A"]._replace(span=CHANGER_SPAN)

AMPS, OHMS, MILLIHENRIES, TORQUE, STEPS = P.MOTION_MOTOR
PULSE = 5.0  # microseconds, grblHAL's default
SEEK, LOCATE = 300.0, 30.0  # homing's rates, mm/min
PULLOFF = 1.0
# Homing: on, single axes allowed, required at start, zero at the switch, X and Y's two switches
# share an input, and a soft reset lets $X unlock, to jog an axis off its far switch.
HOMING = 1 + 2 + 4 + 8 + 16 + 64


def mask(letters):
    """grblHAL's axis mask for the given axes."""
    return sum(1 << ORDER.index(k) for k in letters)


def steps_per_mm(axis):
    return STEPS * axis.micro / axis.lead


def envelope(axis):
    """How far an axis may go from its homed zero. grblHAL takes the pull-off from max travel."""
    return axis.span - axis.short - PULLOFF


def supply_speed(lead):
    """Roughly the speed, in mm/s, at which the supply can just hold the rated current in a motor,
    against its resistance, its back EMF and its inductance. Its torque falls away above that."""
    ke = TORQUE / (math.sqrt(2) * AMPS)  # V per rad/s; the holding torque has both phases on
    reactance = STEPS / 4 * MILLIHENRIES * 1e-3 * AMPS
    drop = AMPS * OHMS
    a, b, c = ke * ke + reactance * reactance, 2 * drop * ke, drop * drop - P.MOTION_SUPPLY ** 2
    return (-b + math.sqrt(b * b - 4 * a * c)) / (2 * a) / (2 * math.pi) * lead


def move_time(distance, speed, accel):
    """Seconds for a move from rest to rest."""
    ramps = speed * speed / accel
    if distance < ramps:
        return 2 * math.sqrt(distance / accel)
    return 2 * speed / accel + (distance - ramps) / speed


def objectives():
    """Each objective's nominal position on the changer. It homes towards the last one's position,
    and the switch trips past it."""
    start = P.MOTION_TRIP - PULLOFF
    return {name: start + k * P.CHANGER_PITCH for k, name in enumerate(reversed(P.CHANGER_ORDER))}


def per_axis(base, value):
    return [(base + i, value(AXES[k])) for i, k in enumerate(ORDER)]


def settings():
    """The groups of settings, each (what it is, [(number, value)]), in the order to send them.
    Homing comes before the soft limits, which grblHAL refuses without it."""
    phases = [(44 + i, mask(p)) for i, p in enumerate(P.MOTION_HOMING)]
    return [
        ("The A axis is the changer's slide, not a rotary axis", [(376, 0)]),
        ("Step pulse, us; idle delay, ms; which motors stay on when idle: not the changer's",
         [(0, PULSE), (1, 250), (37, mask("XYZ"))]),
        ("Direction invert: set on the bench", [(3, mask(P.MOTION_DIR_INVERT))]),
        ("Steps per mm", per_axis(100, steps_per_mm)),
        ("Top speed, mm/min", per_axis(110, lambda a: a.speed * 60)),
        ("Acceleration, mm/s2", per_axis(120, lambda a: a.accel)),
        ("Max travel, mm", per_axis(130, lambda a: a.span - a.short)),
        ("Motor current, mA RMS", per_axis(140, lambda a: P.MOTION_RUN_MA)),
        ("Microsteps", per_axis(150, lambda a: a.micro)),
        ("Hold current, % of run", per_axis(210, lambda a: a.hold)),
        ("Homing: options, all axes towards negative, locate and seek rates, debounce, pull-off, "
         "passes", [(22, HOMING), (23, mask(ORDER)), (24, LOCATE), (25, SEEK), (26, 250),
                    (27, PULLOFF), (43, 1)]),
        ("Homing's phases: the lift, then the changer, then the stage", phases),
        ("Limits: X and Y's switches close when pushed; soft limits, hard limits, jogs kept inside",
         [(5, mask("XY")), (6, 0), (20, 1), (21, 1), (40, 1)]),
    ]


def text():
    lines = ["; OPTIMA's motion controller: grblHAL settings for the SKR Pico, axes in the order "
             "X Y Z A.",
             "; Built by cad/optima/make.py from optima_cad/params.py. Change them there.",
             "; Send them with cad/optima/load_settings.py."]
    for what, values in settings():
        lines.append("; " + what)
        lines += ["$%d=%g" % v for v in values]
    return "\n".join(lines) + "\n"


def checks():
    peak = P.MOTION_RUN_MA / 1000 * math.sqrt(2)
    speeds, fast = [], []
    for k in ORDER:
        a = AXES[k]
        limit = supply_speed(a.lead)
        speeds.append("%s %g of %.1f on %g V%s" % (k, a.speed, limit, P.MOTION_SUPPLY,
                                                   " and its maker's %g" % a.top if a.top else ""))
        fast.append(a.speed > limit or (a.top is not None and a.speed > a.top))
    rates = {k: AXES[k].speed * steps_per_mm(AXES[k]) for k in ORDER}
    fine = {k: AXES[k].lead / STEPS / AXES[k].micro * 1000 for k in ORDER}
    power = len(ORDER) * 2 * (P.MOTION_RUN_MA / 1000) ** 2 * OHMS
    z, y = AXES["Z"], AXES["Y"]
    focus = P.STAND_FOCUS - PULLOFF
    wells = (P.MICROPLATE_WELLS[0] - 1) * P.MICROPLATE_WELLS[2]
    spots = objectives().values()
    tile = move_time(P.MOTION_TILE[0], y.speed, y.accel)
    return [
        Check("Each axis's speed is within what its maker gives and what the supply can drive",
              not any(fast), "mm/s: " + ", ".join(speeds)),
        Check("Each motor's peak current is within its rating, and the drivers need no fan",
              peak <= AMPS and P.MOTION_RUN_MA / 1000 <= P.MOTION_NO_FAN,
              "%d mA RMS is %.2f A peak, of %g A; the board's maker advises a fan above %g A"
              % (P.MOTION_RUN_MA, peak, AMPS, P.MOTION_NO_FAN)),
        Check("The steps are as fine as the design's",
              all(fine[k] <= v for k, v in P.MOTION_STEP.items()),
              ", ".join("%s %.3f um, of %g" % (k, fine[k], v) for k, v in P.MOTION_STEP.items())),
        Check("The step pulses fit the fastest steps", max(rates.values()) <= 1e6 / (2 * PULSE),
              "%.0f kHz at most, of %.0f" % (max(rates.values()) / 1000, 1e3 / (2 * PULSE))),
        Check("The motors stay within the electrical design's budget", power <= P.MOTION_BUDGET_W,
              "%.0f W with all four running, of %g W" % (power, P.MOTION_BUDGET_W)),
        Check("The soft limits stop short of the far switches, and reach what the instrument needs",
              all(AXES[k].short > 0 for k in ORDER)
              and envelope(AXES["X"]) >= wells and envelope(y) >= wells
              and 0 <= focus <= envelope(z)
              and all(0 <= s <= envelope(AXES["A"]) for s in spots),
              "X and Y %g mm, for a plate's %g; Z %g, for a slide's focus at %g; A %g, for the "
              "objectives at %s" % (envelope(y), wells, envelope(z), focus, envelope(AXES["A"]),
                                    ", ".join("%g" % s for s in spots))),
        Check("Homing lowers the lift first, then moves the changer, and homes every axis once",
              P.MOTION_HOMING[:2] == ("Z", "A")
              and sorted("".join(P.MOTION_HOMING)) == sorted(ORDER),
              ", then ".join(P.MOTION_HOMING)),
        Check("A tile step takes no longer than the design's", tile <= P.MOTION_TILE[1],
              "%.2f s for %g mm, of %g s" % (tile, P.MOTION_TILE[0], P.MOTION_TILE[1])),
    ]


def facts():
    y, a = AXES["Y"], AXES["A"]
    spots = objectives()
    wells = (P.MICROPLATE_WELLS[0] - 1) * P.MICROPLATE_WELLS[2]
    return [
        ("Steps per mm, X Y Z A", " ".join("%g" % steps_per_mm(AXES[k]) for k in ORDER)),
        ("Soft limits, mm, X Y Z A", " ".join("%g" % envelope(AXES[k]) for k in ORDER)),
        ("Across the plate's wells at full speed, s", "%.1f" % move_time(wells, y.speed, y.accel)),
        ("One objective change, s", "%.1f" % move_time(P.CHANGER_PITCH, a.speed, a.accel)),
        ("Longest homing seek, s", "%.0f" % (max(AXES[k].span for k in ORDER) / SEEK * 60)),
        ("The objectives on the A axis, mm", ", ".join("%s %g" % s for s in spots.items())),
    ]


def write(out_dir):
    """Writes grblhal-settings.txt into `out_dir`. Returns its checks and facts."""
    path = os.path.join(out_dir, "grblhal-settings.txt")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text())
    return checks(), facts()

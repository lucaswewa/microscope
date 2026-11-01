"""The bought optics, motion parts and electronics, simplified from their makers' drawings.

Each is built in its own frame, which its function describes. The objectives and the tube lens
use the optical layout's unfolded axis: the origin at the sample's focal plane, +Z up the axis.
A document shows its parts side by side, each moved along X from its own frame.
"""

import math

import FreeCAD as App
import Part

from . import geom as G
from . import optics as O
from . import params as P
from .checks import Check, clashes, matches
from .document import add, group

V = App.Vector
X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
SMALL_MIRROR = 6.0  # a 1 in. mirror's thickness, assumed
LED_PACKAGE = 1.6  # a 5050 LED's height, assumed


def objective(name):
    """An objective on the axis, its front its working distance above the focal plane."""
    o = P.OBJECTIVES[name]
    shoulder = o["wd"] + o["length"]
    near, far = P.OBJECTIVE_KNURL
    thread_d, thread_l = P.OBJECTIVE_THREAD
    return Part.makeCylinder(P.OBJECTIVE_BARREL / 2, o["length"], V(0, 0, o["wd"])).fuse([
        Part.makeCylinder(P.OBJECTIVE_DIAMETER / 2, far - near, V(0, 0, shoulder - far)),
        Part.makeCylinder(thread_d / 2, thread_l, V(0, 0, shoulder)),
    ]).removeSplitter()


def tube_lens():
    """The TTL200-A's housing, its back where the design puts it, and its entrance pupil."""
    d, length = P.TUBE_HOUSING
    return {"housing": Part.makeCylinder(d / 2, length, V(0, 0, O.TUBE_BACK - length)),
            "pupil": G.disc(P.TUBE_PUPIL, 1, V(0, 0, O.TUBE), Z)}


def right_angle_mount(cage, mirror_d, thickness):
    """A right-angle kinematic cage cube and its mirror. The light arrives along +Z and leaves
    along +Y, and the mirror's face is on the origin. The cube is shown as its part behind it."""
    side = cage[2]
    n = V(0, 1, -1).normalize()
    cube = G.split(G.box((side, side, side), V(0, 0, 0)), n * -thickness, n, n * -(thickness + 1))[0]
    return {"cube": cube, "mirror": G.mirror(mirror_d, thickness, V(0, 0, 0), Z, Y)}


def flip_mount(position=1):
    """The MFF102/M with its ring and a 2 in. mirror, in position 1 or 2. Its frame: the origin at
    the optic's centre in position 1, the optic facing -Y, the body behind it and the base below."""
    w, h, depth = P.FLIP_BODY
    ring_d, ring_t = P.FLIP_RING
    base = -P.FLIP_OPTIC
    pivot = V(0, 0, base + P.FLIP_PIVOT)
    body = G.span(V(-w / 2, ring_t / 2, base), V(w / 2, ring_t / 2 + depth, base + h))
    ring = Part.makeCylinder(ring_d / 2, ring_t, V(0, -ring_t / 2, 0), Y)
    hole = Part.makeCylinder(P.MIRROR_DIAMETER / 2, ring_t + 2, V(0, -ring_t / 2 - 1, 0), Y)
    swing = ring.cut(hole).fuse(G.span(V(-6, -ring_t / 2, pivot.z), V(6, ring_t / 2, -ring_d / 2)))
    mirror = G.disc(P.MIRROR_DIAMETER, P.MIRROR_THICKNESS, V(0, 0, 0), Y)
    if position == 2:
        swing, mirror = swing.copy(), mirror.copy()
        for s in (swing, mirror):
            s.rotate(pivot, Y, -90)
    return {"body": body, "swing": swing.removeSplitter(), "mirror": mirror}


def mounted_led():
    """A Thorlabs mounted LED: its die on the origin, shining along +Z."""
    d, length = P.LED_HOUSING
    housing = Part.makeCylinder(d / 2, length, V(0, 0, 6 - length))
    housing = housing.cut(Part.makeCylinder(P.SM1 / 2, 7, V(0, 0, 0)))
    return {"housing": housing, "die": G.box((P.LED_DIE, P.LED_DIE, 0.2), V(0, 0, 0.1))}


def dichroic_cube():
    """A 30 mm cage cube with a dichroic on the origin: light along +X passes, and light arriving
    along -Y turns into +X."""
    side = P.CAGE_30[2]
    cube = G.box((side, side, side), V(0, 0, 0))
    for axis in (X, Y):
        cube = cube.cut(Part.makeCylinder(P.SM1 / 2, side + 2, axis * -(side / 2 + 1), axis))
    return {"cube": cube, "dichroic": G.plate(P.DICHROIC, V(0, 0, 0), -Y, X)}


def cage_plate():
    """A 30 mm cage plate with an SM1 bore, centred on the origin and facing +Z."""
    spacing, rod, side = P.CAGE_30
    t = P.CAGE_PLATE_30
    s = spacing / 2
    holes = [(P.SM1, None, [(0, 0)]), (rod, None, [(x, y) for x in (-s, s) for y in (-s, s)])]
    return G.drill(G.box((side, side, t), V(0, 0, 0)), holes, t / 2, -Z)


@G.kept
def guide(position=0.0):
    """An MGN12 rail along X, the origin at the centre of its underside, with an MGN12H carriage
    `position` from the rail's centre."""
    rw, rh, pitch, length = P.MGN12_RAIL
    cw, cl, ch, clear, square = P.MGN12H
    n = int(length // pitch)
    holes = [((k - (n - 1) / 2) * pitch, 0) for k in range(n)]
    rail = G.drill(G.box((length, rw, rh), V(0, 0, rh / 2)), [(3.5, None, holes), (6.5, 4.5, holes)],
                   rh, -Z)  # counterbored for M3
    carriage = G.box((cl, cw, ch - clear), V(position, 0, (ch + clear) / 2))
    slot = G.span(V(position - cl, -rw / 2 - 0.5, 0), V(position + cl, rw / 2 + 0.5, rh + 0.5))
    carriage = carriage.cut(slot)
    s = square / 2
    holes = [(position + x, y) for x in (-s, s) for y in (-s, s)]
    carriage = G.drill(carriage, [(3.0, 4.0, holes)], ch, -Z)
    return {"rail": rail, "carriage": carriage}


def stepper(screw_length=P.LEAD_SCREW[1]):
    """A NEMA 11 lead-screw stepper: its face on the origin, its screw along +Z."""
    face, length, square, pilot = P.NEMA11
    s = square / 2
    motor = G.box((face, face, length), V(0, 0, -length / 2))
    motor = motor.fuse(Part.makeCylinder(pilot / 2, 2, V(0, 0, 0)))
    motor = G.drill(motor, [(2.5, 4.0, [(x, y) for x in (-s, s) for y in (-s, s)])], 0, -Z)
    return {"motor": motor, "screw": Part.makeCylinder(P.LEAD_SCREW[0] / 2, screw_length, V(0, 0, 2))}


def matrix_leds(positions):
    """The matrix's LEDs at `positions`, their faces on z = 0."""
    s = P.MATRIX_LED
    return Part.makeCompound([G.box((s, s, LED_PACKAGE), V(x, y, -LED_PACKAGE / 2))
                              for x, y in positions])


def led_matrix():
    """The Adafruit 2734: its LEDs' faces on z = 0, centred on the origin."""
    w, d, t = P.MATRIX_BOARD
    board = G.box((w, d, t - LED_PACKAGE), V(0, 0, -(t + LED_PACKAGE) / 2))
    return {"board": board, "leds": matrix_leds(O.matrix_leds())}


def circuit(size):
    """A board and the envelope of the parts on it, the origin at the centre of its underside.
    `size` is (width, depth, height, (the mounting holes' rectangle))."""
    w, d, h, (hx, hy) = size
    holes = [(3.2, None, [(x, y) for x in (-hx / 2, hx / 2) for y in (-hy / 2, hy / 2)])]
    pcb = G.drill(G.box((w, d, 1.6), V(0, 0, 0.8)), holes, 1.6, -Z)
    return {"pcb": pcb, "parts": G.drill(G.box((w, d, h - 1.6), V(0, 0, (h + 1.6) / 2)), holes, h, -Z)}


def _show(doc, g, prefix, parts, styles, x):
    for name, shape in parts.items():
        add(doc, prefix + name.title(), shape.translated(V(x, 0, 0)), styles[name], g)


def build_objectives(doc):
    g = group(doc, "Objectives", "Mitutoyo M Plan Apo, on the changer's pitch")
    shapes = {}
    for k, name in enumerate(P.CHANGER_ORDER):
        shapes[name] = objective(name).translated(V((k - 1) * P.CHANGER_PITCH, 0, 0))
        add(doc, "Objective" + name, shapes[name], "objective", g,
            "%s, NA %.2f" % (name, P.OBJECTIVES[name]["na"]))
    checks = [matches("%s: length and working distance make the parfocal length" % n,
                      o["length"] + o["wd"], P.PARFOCAL) for n, o in P.OBJECTIVES.items()]
    order = P.CHANGER_ORDER
    found = clashes([("%s and %s" % (a, b), shapes[a], shapes[b]) for a, b in zip(order, order[1:])])
    checks.append(Check("Neighbours on the changer's pitch don't touch", not found,
                        ", ".join(found) or "%g mm between their knurled rings"
                        % (P.CHANGER_PITCH - P.OBJECTIVE_DIAMETER)))
    facts = [("Barrel, knurled ring",
              "%g mm, %g mm across" % (P.OBJECTIVE_BARREL, P.OBJECTIVE_DIAMETER)),
             ("Thread", "M26 x 0.706, %g mm long" % P.OBJECTIVE_THREAD[1])]
    return checks, facts


def build_imaging(doc):
    g = group(doc, "TubeLens", "Tube lens: Thorlabs TTL200-A, on the unfolded axis")
    _show(doc, g, "Tube", tube_lens(), {"housing": "objective", "pupil": "optic"}, 0)
    g = group(doc, "FoldMount", "Fold mirror in a 60 mm cage cube, shown 150 mm to the right")
    styles = {"cube": "metal", "mirror": "mirror"}
    fold = right_angle_mount(P.CAGE_60, P.MIRROR_DIAMETER, P.MIRROR_THICKNESS)
    _show(doc, g, "Fold", fold, styles, 150)
    g = group(doc, "CameraPort", "Camera port: Thorlabs MFF102/M, shown 300 mm to the right")
    styles = {"body": "drive", "swing": "metal", "mirror": "mirror"}
    _show(doc, g, "Port", flip_mount(1), styles, 300)
    out = flip_mount(2)
    swung = Part.makeCompound([out["swing"], out["mirror"]]).translated(V(300, 0, 0))
    add(doc, "PortOut", swung, "reserved", g,
        "Position 2: the mirror out")
    # The light on the flip mirror, as the layout has it, and the mirror's ring in position 2.
    to_image = O.IMAGE - O.FOLD - P.FLIP_AFTER_FOLD
    light = math.sqrt(2) * O.corner_light(to_image)
    clear = out["mirror"].BoundBox.Center.Length - P.FLIP_RING[0] / 2
    lo, hi = O.TUBE_BACK - P.TUBE_HOUSING[1], O.TUBE_BACK
    slider = O.FILTER + P.FILTER_CUBE / 2
    checks = [
        Check("Position 2 takes the ring clear of the light", clear >= light,
              "the ring %.1f mm from the light's centre; the light reaches %.1f mm" % (clear, light)),
        Check("The tube lens's housing clears the filter slider", lo >= slider,
              "housing from z %.1f, slider up to %.1f" % (lo, slider)),
    ]
    facts = [("Tube lens housing, z", "%.1f to %.1f" % (lo, hi)),
             ("Camera port: the mirror's centre moves",
              "%.1f mm" % out["mirror"].BoundBox.Center.Length)]
    return checks, facts


def build_epi(doc):
    g = group(doc, "EpiParts", "Epi parts, side by side")
    _show(doc, g, "Led", mounted_led(), {"housing": "drive", "die": "led"}, 0)
    _show(doc, g, "Combiner", dichroic_cube(), {"cube": "metal", "dichroic": "optic"}, 60)
    add(doc, "CagePlate", cage_plate().translated(V(120, 0, 0)), "metal", g, "30 mm cage plate")
    _show(doc, g, "EpiFold", right_angle_mount(P.CAGE_30, P.SMALL_OPTIC, SMALL_MIRROR),
          {"cube": "metal", "mirror": "mirror"}, 180)
    d, t = P.FILTER_ROUND
    filters = [G.disc(d, t, V(240, 0, z), Z) for z in (-20, 20)]
    filters.append(G.plate(P.DICHROIC, V(240, 0, 0), -Y, X))
    add(doc, "Filters", Part.makeCompound(filters), "optic", g,
        "Chroma 89402: exciter, dichroic, emitter")
    spacing, rod = P.CAGE_30[:2]
    between = spacing * math.sqrt(2) - rod
    checks = [
        Check("The dichroic fits the cage cube's holder",
              all(a <= b for a, b in zip(P.DICHROIC, P.DICHROIC_HOLDER)),
              "%g x %g x %g mm in %g x %g x %g mm" % (P.DICHROIC + P.DICHROIC_HOLDER)),
        Check("A mounted LED fits between the 30 mm cage's rods", P.LED_HOUSING[0] <= between,
              "%g mm across, %.1f mm between the rods" % (P.LED_HOUSING[0], between)),
    ]
    return checks, [("LED housing", "%g mm across, %g mm long" % P.LED_HOUSING)]


def build_motion(doc):
    length = P.MGN12_RAIL[3]
    travel = length - P.MGN12H[1]
    g = group(doc, "Guide", "MGN12 rail and MGN12H carriage")
    _show(doc, g, "Guide", guide(), {"rail": "metal", "carriage": "metal"}, 0)
    reach = Part.makeCompound([guide(s * travel / 2)["carriage"] for s in (-1, 1)])
    add(doc, "Travel", G.bounds(reach), "reserved", g, "Swept by the carriage")
    g = group(doc, "Stepper", "NEMA 11 lead-screw stepper, shown 120 mm to the right")
    _show(doc, g, "Stepper", stepper(), {"motor": "drive", "screw": "metal"}, 120)
    need = 2 * max(P.CHANGER_PITCH, P.FILTER_PITCH) + 2 * P.TRAVEL_MARGIN
    checks = [Check("The rail gives the changer and the slider their travel", travel >= need,
                    "%.1f mm on a %g mm rail; they need %g mm with margins" % (travel, length, need))]
    return checks, [("Carriage travel", round(travel, 1))]


def build_matrix(doc):
    g = group(doc, "Matrix", "LED matrix: Adafruit 2734")
    _show(doc, g, "Matrix", led_matrix(), {"board": "board", "leds": "led"}, 0)
    widest = max(o["na"] for o in P.OBJECTIVES.values())
    dark = sum(O.led_na(x, y) > widest for x, y in O.matrix_leds())
    reach = max(O.led_na(x, y) for x, y in O.matrix_leds())
    checks = [Check("Some LEDs lie outside every objective's NA, for darkfield", dark > 0,
                    "%d LEDs between NA %.2f and %.2f" % (dark, widest, reach))]
    facts = [("Pitch, height below the sample",
              "%g mm, %g mm" % (P.MATRIX_PITCH, P.MATRIX_BELOW_SAMPLE))]
    return checks, facts


def build_electronics(doc):
    g = group(doc, "Electronics", "Electronics, side by side")
    styles = {"pcb": "board", "parts": "body"}
    _show(doc, g, "SkrPico", circuit(P.SKR_PICO), styles, 0)
    _show(doc, g, "Pico2", circuit(P.PICO2), styles, 80)
    for k in range(4):
        add(doc, "LddH%d" % (k + 1), G.box(P.LDD_H, V(130 + 35 * k, 0, P.LDD_H[2] / 2)), "board", g,
            "LED driver %d: Mean Well LDD-H" % (k + 1))
    add(doc, "Supply", G.box(P.SUPPLY, V(340, 0, P.SUPPLY[2] / 2)), "metal", g, "Mean Well LRS-150-24")
    checks = [Check("The supply covers the peak draw", P.SUPPLY_W >= P.PEAK_W,
                    "%g W for about %g W" % (P.SUPPLY_W, P.PEAK_W))]
    return checks, [("Supply", "%g x %g x %g mm" % P.SUPPLY)]

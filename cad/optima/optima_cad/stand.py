"""The stand: the base plate on its feet, the column that carries the optical head, the stage stack
on the Z lift, and the overview camera on its bracket at the head's front.

Everything is in the instrument's frame. The stack's parts take the lift's height from the bottom
of its travel and the stage's position from the centre of its travel: with the lift at STAND_FOCUS
and the stage centred, a slide's sample is on the focal plane.
"""

import math
from functools import cache

import FreeCAD as App
import Part

from . import cameras, fasteners, head, inserts, parts, stages
from . import geom as G
from . import params as P
from .checks import Check, clashes
from .document import add, group
from .fasteners import Screw

V = App.Vector
X, Z = V(1, 0, 0), V(0, 0, 1)

XY_Z = -P.INSERT_ADAPTER[0] - P.XY_HEIGHT  # the XY stage's underside
MATRIX_Z = -P.MATRIX_BELOW_SAMPLE  # the matrix's LEDs' faces
PLATE_TOP = MATRIX_Z - P.MATRIX_BOARD[2] - P.STAND_MATRIX[1]  # the plate on the lift
BASE_TOP = PLATE_TOP - P.STAND_PLATE[2] - P.LIFT_HEIGHT - P.STAND_FOCUS
COLUMN_Y = P.HEAD_PLATE[4] + P.STAND_COLUMN[1] / 2  # the extrusions' centres, behind the head plate
MOVES_Y = ("middle", "x_drive")  # with the stage's Y, as well as the lift
MOVES_XY = ("top", "adapter", "insert", "slides")
AIM = V(0, -P.XY_TRAVEL, 0)  # the insert's centre, with the stage at the front of its travel
STACK = {  # each of the stack's parts: its style and label
    "lift": ("metal", "Lift's top"),
    "plate": ("metal", "Plate on the lift"),
    "spacers": ("metal", "Spacers under the XY stage"),
    "mount": ("metal", "LED matrix's mount"),
    "board": ("board", "LED matrix: Adafruit 2734"),
    "leds": ("led", "LED matrix's LEDs"),
    "base": ("metal", "XY stage's base plate"),
    "y_drive": ("drive", "XY stage's Y drive"),
    "middle": ("metal", "XY stage's middle plate"),
    "x_drive": ("drive", "XY stage's X drive"),
    "top": ("metal", "XY stage's top plate"),
    "adapter": ("metal", "Insert adapter"),
    "insert": ("metal", "Four-slide insert"),
    "slides": ("sample", "Slides and cover glasses"),
}


def column_screws():
    """The two M8 screws up through the base plate into each extrusion's bores."""
    _, d, _, x = P.STAND_COLUMN
    return [(sx * x, COLUMN_Y + dy) for sx in (-1, 1) for dy in (-d / 4, d / 4)]


@cache
def screws():
    """The stand's screws: each a fasteners.Screw."""
    t, (pw, pd, pt), (side, mt) = P.STAND_BASE[4], P.STAND_PLATE, P.STAND_MATRIX
    up, down = (0, 0, 1), (0, 0, -1)
    out = [Screw(6, (x, y, BASE_TOP + P.LIFT_BASE[2]), down, P.LIFT_BASE[2], 10, "lift", "base plate")
           for x, y in P.LIFT_BASE_HOLES[0][2]]
    out += [Screw(8, (x, y, BASE_TOP - t), up, t, 16, "base plate", "column")
            for x, y in column_screws()]
    # The electronics tray, on spacers, screwed down into the base plate.
    x0, y0, x1, y1, tt, ts = P.TRAY
    out += [Screw(3, (x, y, BASE_TOP + ts + tt), down, ts + tt, 6, ("tray", "standoffs"), "base plate")
            for x in (x0 + 8, x1 - 8) for y in (y0 + 8, y1 - 8)]
    # The plate on the lift: sunk into four of the lift top's M6 holes, at its corners, under the
    # matrix's mount; and up through the spacers into the XY stage.
    corner = max(x for x, _ in P.LIFT_TOP_HOLES[0][2])
    out += [Screw(6, (x, y, PLATE_TOP - 6.5), down, pt - 6.5, 7, "lift plate", "lift top", sunk=True)
            for x, y in P.LIFT_TOP_HOLES[0][2] if abs(x) == abs(y) == corner]
    out += [Screw(6, (x, y, PLATE_TOP - pt), up, XY_Z - PLATE_TOP + pt, 8, ("lift plate", "spacers"),
                  "xy base") for x, y in P.STAND_SPACERS[1]]
    s = side / 2 - 5
    out += [Screw(3, (x, y, PLATE_TOP + mt), down, mt, 6, "matrix mount", "lift plate")
            for x in (-s, s) for y in (-s, s)]
    # The insert adapter, its counterbores drilled with it.
    out += [Screw(3, (x, y, -3.0), down, 3.0, 4, "adapter", "xy top", drilled=True)
            for x, y in P.XY_HOLES[0][2] if abs(y) > P.INSERT[1] / 2 + P.INSERT_ADAPTER[1]]
    return tuple(out)


@G.kept
def base():
    """The base plate, with tapped holes for the feet and the holes for its screws, and its feet."""
    x0, y0, x1, y1, t = P.STAND_BASE
    d, h, inset = P.STAND_FEET
    feet = [(x, y) for x in (x0 + inset, x1 - inset) for y in (y0 + inset, y1 - inset)]
    plate = G.drill(G.span(V(x0, y0, BASE_TOP - t), V(x1, y1, BASE_TOP)), [(6.8, None, feet)],
                    BASE_TOP - t, Z)
    plate = fasteners.cut(plate, screws(), "base plate")
    under = BASE_TOP - t - h
    return {"plate": plate,
            "feet": Part.makeCompound([Part.makeCylinder(d / 2, h, V(x, y, under)) for x, y in feet])}


def extrusion(length, at):
    """A 40 x 80 mm extrusion standing on `at`, the centre of its foot, 80 mm along Y: its slots,
    9 mm deep, and the bores its end screws tap into."""
    w, d, slot, _ = P.STAND_COLUMN
    mid = at + V(0, 0, length / 2)
    tools = [G.box((slot, 18, length + 2), mid + V(0, sy * d / 2, 0)) for sy in (-1, 1)]
    tools += [G.box((18, slot, length + 2), mid + V(sx * w / 2, dy, 0))
              for sx in (-1, 1) for dy in (-d / 4, d / 4)]
    tools += [Part.makeCylinder(3.4, length + 2, at + V(0, dy, -1)) for dy in (-d / 4, d / 4)]
    return G.box((w, d, length), mid).cut(Part.makeCompound(tools))


@G.kept
def column():
    """Two extrusions from the base plate to the head plate's top, bolted to its back face."""
    length = P.HEAD_PLATE[5] - BASE_TOP
    return [extrusion(length, V(sx * P.STAND_COLUMN[3], COLUMN_Y, BASE_TOP)) for sx in (-1, 1)]


@G.kept
def stack():
    """The parts the lift moves, with a slide in focus and the stage centred: the lift's top, the
    plate on it, the spacers, the matrix and its mount, the XY stage's parts, the adapter, and the
    four-slide insert and its slides."""
    pw, pd, pt = P.STAND_PLATE
    sd, spacers = P.STAND_SPACERS
    s = screws()
    plate = fasteners.cut(G.box((pw, pd, pt), V(0, 0, PLATE_TOP - pt / 2)), s, "lift plate")
    tube = Part.makeCompound([Part.makeCylinder(sd / 2, XY_Z - PLATE_TOP, V(x, y, PLATE_TOP))
                              for x, y in spacers])
    tube = fasteners.cut(tube, s, "spacers")
    side, mt = P.STAND_MATRIX
    mount = fasteners.cut(G.box((side, side, mt), V(0, 0, PLATE_TOP + mt / 2)), s, "matrix mount")
    matrix = parts.led_matrix()
    out = {"lift": stages.lift(P.STAND_FOCUS)["top"].translated(V(0, 0, BASE_TOP)), "plate": plate,
           "spacers": tube, "mount": mount,
           "board": matrix["board"].translated(V(0, 0, MATRIX_Z)),
           "leds": matrix["leds"].translated(V(0, 0, MATRIX_Z))}
    out.update({k: v.translated(V(0, 0, XY_Z)) for k, v in stages.xy_stage().items()})
    slides = inserts.slides()
    out.update(adapter=inserts.adapter(), insert=slides["insert"], slides=slides["labware"])
    return out


def move(shapes, h, x=0.0, y=0.0):
    """The stack's `shapes` with the lift `h` from the bottom of its travel, and the stage at (x, y)."""
    dz = h - P.STAND_FOCUS
    return {k: s.translated(V(x if k in MOVES_XY else 0, y if k in MOVES_XY + MOVES_Y else 0, dz))
            for k, s in shapes.items()}


@G.kept
def overview():
    """The overview camera, and its bracket: a flange on the upper deck's front, a leg down in front
    of the filter slider's travel, and a plate that holds the camera's board from behind on four
    standoffs. The camera looks at AIM, its sensor's long side along X. Returns the camera's parts,
    the bracket, and the point the camera sees from."""
    at = V(*P.STAND_OVERVIEW)
    back = (at - AIM).normalize()
    up = back.cross(X)
    w, front, lt, flange, t = P.STAND_OVERVIEW_ARM
    s = P.OVERVIEW_HOLES / 2
    holes = [(x, y) for x in (-s, s) for y in (-s, s)]
    rear = 0.5 + P.OVERVIEW_BOARD[2]  # the board's back, in the camera's frame
    plate = G.span(V(-w / 2, -w / 2, rear + 6), V(w / 2, w / 2 + 4, rear + 6 + t))
    plate = plate.cut(G.span(V(-7, -w / 2 - 1, rear), V(7, -10, rear + 7 + t)))  # round the plug
    plate = plate.fuse([Part.makeCylinder(2, 6, V(x, y, rear)) for x, y in holes])
    plate = G.drill(plate, [(2.2, None, holes)], rear + 6 + t, -Z)
    top = head.DECK_TOP
    leg = G.span(V(-w / 2, front, at.z), V(w / 2, front + lt, top + t))
    seat = at + back * (rear + 5 + t)  # 1 mm inside the plate: the leg's foot lies on it
    leg = G.split(leg, seat, back, seat + back)[0]
    bracket = leg.fuse([G.placed(plate, at, back, up),
                        G.span(V(-w / 2, front, top), V(w / 2, flange, top + t))]).removeSplitter()
    bracket = fasteners.cut(bracket, head.screws(), "overview bracket")
    camera = {k: G.placed(v, at, back, up) for k, v in cameras.overview().items()}
    return camera, bracket, at - back * P.OVERVIEW_LENS_F


def place(doc, parent=None):
    """Adds the stand's parts to `doc`, in groups, under `parent` if it's given. Returns them: the
    base, the column, the stack, the lift's base, the overview camera and its bracket, the point
    the camera sees from, and its view of the insert."""
    b, cols, st = base(), column(), stack()
    lift = stages.lift()["fixed"].translated(V(0, 0, BASE_TOP))
    camera, bracket, eye = overview()
    w, d, _ = P.INSERT
    view = G.rect_loft((0.5, 0.5), eye, (w, d), AIM)

    def new_group(name, label):
        g = group(doc, name, label)
        if parent is not None:
            parent.addObject(g)
        return g

    g = new_group("Base", "Base plate and feet")
    add(doc, "BasePlate", b["plate"], "metal", g, "Base plate")
    add(doc, "Feet", b["feet"], "drive", g, "Rubber feet")
    g = new_group("Column", "Column: two 40 x 80 mm extrusions")
    for side, c in zip(("Left", "Right"), cols):
        add(doc, "Column" + side, c, "metal", g, "Column, %s" % side.lower())
    g = new_group("Stack", "Z lift and stage stack, a slide in focus and the stage centred")
    add(doc, "LiftBase", lift, "drive", g, "Z lift: Standa 8MVT100-25-1")
    for k, s in st.items():
        add(doc, "Stack" + k.title().replace("_", ""), s, STACK[k][0], g, STACK[k][1])
    g = new_group("Overview", "Overview camera and its bracket")
    for k, s in camera.items():
        add(doc, "Overview" + k.title(), s, cameras.STYLES[k], g,
            "Room for the overview camera's plug" if k == "plugs" else "Overview camera's " + k)
    add(doc, "OverviewBracket", bracket, "metal", g, "Overview camera's bracket")
    add(doc, "OverviewView", view, "imaging", g,
        "The overview camera's view of the insert, the stage at the front of its travel")
    return b, cols, st, lift, camera, bracket, eye, view


def build_stand(doc):
    b, cols, st, lift, camera, bracket, eye, view = place(doc)
    w, d, _ = P.INSERT
    structure = head.structure()
    g = group(doc, "Head", "The optical head's plate, decks and brackets (optical-head.FCStd)")
    for k, s in structure.items():
        add(doc, "Head" + k.title(), s, "metal", g, {
            "plate": "Head plate", "lower": "Lower deck", "upper": "Upper deck",
            "bracket": "Camera port's bracket", "epi": "Epi plate",
            "epi_bracket": "Epi plate's bracket"}[k])

    s = P.XY_TRAVEL
    spots = [(0, 0)] + [(sx * s, sy * s) for sx in (-1, 1) for sy in (-1, 1)]
    heights = (0.0, P.STAND_FOCUS, P.LIFT_TRAVEL)
    positions = [(h, x, y) for h in heights for x, y in spots]
    sweep = [move(st, *p) for p in positions]
    add(doc, "StackTravel", G.bounds(Part.makeCompound([p for m in sweep for p in m.values()])),
        "reserved", group(doc, "Travel", "Swept by the stage stack"),
        "Swept by the stage stack over the lift's and the stage's travel")

    # The head's parts, fixed and swept, for the overview camera and its bracket.
    steps = [10.0 * k for k in range(-4, 5)]
    head_parts = {
        "the head's fixed parts": Part.makeCompound(
            list(structure.values()) + list(head.changer_drive().values())
            + list(head.slider_drive().values()) + [head.fold_mount()]),
        "the changer": Part.makeCompound([head.changer(p)[k] for p in steps
                                          for k in ("plate", "objectives", "carriage")]),
        "the slider": Part.makeCompound([head.slider(p)[k] for p in steps
                                         for k in ("carrier", "filters", "carriage")]),
    }
    fixed = {"the base plate": b["plate"], "the feet": b["feet"], "the column": Part.makeCompound(cols),
             "the lift's base": lift, "the head plate": structure["plate"],
             "the overview camera": Part.makeCompound(list(camera.values()) + [bracket])}
    names = list(fixed)
    found = clashes([("%s and %s" % (a, c), fixed[a], fixed[c])
                     for i, a in enumerate(names) for c in names[i + 1:]])
    keys = list(st)
    inside = clashes([("the %s and the %s" % (STACK[a][1].lower(), STACK[c][1].lower()), st[a], st[c])
                      for i, a in enumerate(keys) for c in keys[i + 1:]])
    moved = []
    for (h, x, y), m in zip(positions, sweep):
        moved += clashes([("the %s and %s, the lift at %g and the stage at %g, %g"
                           % (STACK[k][1].lower(), f, h, x, y), p, fixed[f])
                          for k, p in m.items() for f in names])
    lifts = {n: P.STAND_FOCUS - z for n, z in inserts.PLANES.items()}
    stops = [head.changer(o)["objectives"] for o in head.CHANGES]
    hidden = clashes([("the %s on the axis" % n, view, o) for n, o in zip(P.CHANGER_ORDER, stops)]
                     + [("the head's fixed parts", view, head_parts["the head's fixed parts"])])
    hits = clashes([("the camera and %s" % k, fixed["the overview camera"], v)
                    for k, v in head_parts.items()])

    # How far into its field the camera sees the insert's corners: 1 is the field's edge.
    look = (AIM - eye).normalize()
    up = (-look).cross(X)
    half = [x / 2 / P.OVERVIEW_LENS_F for x in P.OVERVIEW_SENSOR]
    seen = 0
    for sx in (-1, 1):
        for sy in (-1, 1):
            v = AIM + V(sx * w / 2, sy * d / 2, 0) - eye
            depth = v.dot(look)
            seen = max(seen, abs(v.dot(X)) / depth / half[0], abs(v.dot(up)) / depth / half[1])

    hp = P.HEAD_PLATE
    box = Part.makeCompound(cols).BoundBox
    reach = max(p.BoundBox.YMax for m in sweep for p in m.values())
    metal = ("plate", "spacers", "mount", "adapter", "insert")
    mass = P.XY_MASS + P.STAND_SAMPLE + P.ALUMINIUM * sum(st[k].Volume for k in metal)
    checks = [
        Check("The lift brings every sample to the focal plane", all(0 <= v <= P.LIFT_TRAVEL
                                                                     for v in lifts.values()),
              ", ".join("%s at %g mm" % (n, v) for n, v in lifts.items())
              + " of its %g" % P.LIFT_TRAVEL),
        Check("The fixed parts don't overlap each other", not found, ", ".join(found) or "no overlaps"),
        Check("The stage stack's parts don't overlap each other", not inside,
              ", ".join(inside) or "no overlaps"),
        Check("The stage stack clears the fixed parts across the lift's and the stage's travel",
              not moved, ", ".join(moved[:4]) or "the lift at %s mm, the stage at its centre and "
              "corners" % ", ".join("%g" % h for h in heights)),
        Check("The column meets the head plate's back face, behind the stage's sweep",
              abs(box.YMin - hp[4]) < 1e-9 and box.XMin >= hp[0] and box.XMax <= hp[3]
              and box.ZMax >= hp[5] and reach < hp[4],
              "its front at y = %g, x %g to %g; the stage reaches y = %.1f"
              % (box.YMin, box.XMin, box.XMax, reach)),
        Check("The lift carries the stage stack", mass <= P.LIFT_LOAD,
              "%.1f kg with a %g kg sample, of %g kg" % (mass, P.STAND_SAMPLE, P.LIFT_LOAD)),
        Check("The overview camera sees the whole insert, the stage at the front of its travel",
              seen <= 1, "its corners reach %.0f %% of the way to the field's edge" % (100 * seen)),
        Check("Its view of the insert passes under the objectives", not hidden,
              ", ".join(hidden) or "with each objective on the axis"),
        Check("The overview camera and its bracket clear the head's parts", not hits
              and len(bracket.Solids) == 1,
              ", ".join(hits) or "the changer and the slider every 10 mm; the bracket is one piece"),
    ]
    x0, y0, x1, y1, t = P.STAND_BASE
    cw, cd, _, cx = P.STAND_COLUMN
    facts = [
        ("Base plate", "%g x %g x %g mm, its top at z = %.1f" % (x1 - x0, y1 - y0, t, BASE_TOP)),
        ("Plate on the lift, z", "%.1f to %.1f" % (PLATE_TOP - P.STAND_PLATE[2], PLATE_TOP)),
        ("Matrix's LEDs, z", MATRIX_Z),
        ("Spacers' height", round(XY_Z - PLATE_TOP, 1)),
        ("XY stage's underside, z", XY_Z),
        ("Column", "2 x %g x %g mm extrusions, %.1f mm long, at x = +-%g, y %g to %g"
         % (cw, cd, box.ZLength, cx, box.YMin, box.YMax)),
        ("Table to the head plate's top", round(hp[5] - (BASE_TOP - t - P.STAND_FEET[1]), 1)),
        ("Stage stack's mass, kg", round(mass, 2)),
        ("Overview camera's sensor", "%g, %g, %g" % P.STAND_OVERVIEW),
        ("Overview camera's view", "%.1f degrees from straight down, %.0f mm to the insert's centre"
         % (math.degrees(math.acos(-look.z)), (AIM - eye).Length)),
    ]
    return checks, facts

"""The Standa stages: the 8MTF-102LS05 XY stage and the 8MVT100-25-1 lift, from Standa's drawings.

Each is built in its own frame: the origin at the centre of its base's underside, +Z up, and +X
and +Y as in the instrument. Their moving parts take a position from the centre or the bottom of
the travel, so the assemblies can check them anywhere in it.
"""

import FreeCAD as App
import Part

from . import geom as G
from . import params as P
from .checks import Check, clashes, matches
from .document import add, group

V = App.Vector
Z = V(0, 0, 1)


def opening(height):
    """A prism of the plates' opening, from z = -1 to `height`."""
    side, radius = P.XY_OPENING
    return G.rounded(G.box((side, side, height + 1), V(0, 0, (height - 1) / 2)), radius)


def pocket_faces(z):
    """The top plate's pockets' outlines at height `z`: squares at the opening's corners, reaching
    `width` past it, their outer corners cut off."""
    length, width, _ = P.XY_POCKETS
    far = P.XY_OPENING[0] / 2 + width
    near = far - length
    outline = [(near, near), (far, near), (far, far - width), (far - width, far), (near, far),
               (near, near)]
    return [Part.Face(Part.makePolygon([V(sx * a, sy * b, z) for a, b in outline]))
            for sx in (-1, 1) for sy in (-1, 1)]


def _knob(start, direction):
    start, direction = V(*start), V(*direction)
    (hub_d, hub_l), (knob_d, knob_l) = P.XY_KNOB
    hub = Part.makeCylinder(hub_d / 2, hub_l, start, direction)
    return hub.fuse(Part.makeCylinder(knob_d / 2, knob_l, start + direction * hub_l, direction))


def xy_stage(x=0.0, y=0.0):
    """The XY stage's parts, with its top plate at (x, y) from the centre of the travel."""
    side, t = P.XY_PLATE, P.XY_HEIGHT / 3

    def plate(level):
        return G.box((side, side, t), V(0, 0, (level + 0.5) * t)).cut(opening(P.XY_HEIGHT + 1))

    turned = [(d, depth, [(b, a) for a, b in centres]) for d, depth, centres in P.XY_HOLES]
    base = G.drill(plate(0), turned, 0, Z)
    depth = P.XY_POCKETS[2]
    pockets = [f.extrude(V(0, 0, depth + 1)) for f in pocket_faces(P.XY_HEIGHT - depth)]
    top =G.drill(plate(2).cut(Part.makeCompound(pockets)), P.XY_HOLES, P.XY_HEIGHT, -Z)
    y_drive = Part.makeCompound([G.corners(b) for b in P.XY_Y_DRIVE] + [_knob(*P.XY_Y_KNOB)])
    x_drive = Part.makeCompound([G.corners(b) for b in P.XY_X_DRIVE] + [_knob(*P.XY_X_KNOB)])
    return {
        "base": base,
        "y_drive": y_drive,
        "middle": plate(1).translated(V(0, y, 0)),
        "x_drive": x_drive.translated(V(0, y, 0)),
        "top": top.translated(V(x, y, 0)),
    }


def aperture(x, y):
    """The square through every plate's opening, with the top plate at (x, y)."""
    prism = opening(P.XY_HEIGHT)
    clear = prism.common(prism.translated(V(0, y, 0))).common(prism.translated(V(x, y, 0)))
    return min(clear.BoundBox.XLength, clear.BoundBox.YLength)


def lift(h=0.0):
    """The lift's parts, with its top raised `h` from the bottom of the travel."""
    bt, th = P.LIFT_BASE[2], P.LIFT_TOP[2]
    cw, cl = P.LIFT_COLUMN
    top_z = P.LIFT_HEIGHT
    # The base's corners are rounded, about 5 mm in the drawing.
    base = G.drill(G.rounded(G.box(P.LIFT_BASE, V(0, 0, bt / 2)), 5), P.LIFT_BASE_HOLES, 0, Z)
    column = G.span(V(-cw / 2, -cl / 2, bt), V(cw / 2, cl / 2, top_z - 6))
    fixed = base.fuse([column, G.corners(P.LIFT_CONNECTOR)]).removeSplitter()
    # The hood: a cap over the column, with 1 mm around it and a 5 mm top.
    hood = G.box(P.LIFT_TOP, V(0, 0, top_z - th / 2))
    hood = hood.cut(G.span(V(-cw / 2 - 1, -cl / 2 - 1, top_z - th - 1),
                           V(cw / 2 + 1, cl / 2 + 1, top_z - 5)))
    hood = G.drill(hood, P.LIFT_TOP_HOLES, top_z, -Z)
    return {"fixed": fixed, "top": hood.translated(V(0, 0, h))}


def build_xy(doc):
    s = P.XY_TRAVEL
    parts = xy_stage()
    g = group(doc, "Fixed", "Fixed: base and Y drive")
    add(doc, "Base", parts["base"], "metal", g)
    add(doc, "YDrive", parts["y_drive"], "drive", g, "Y drive")
    g = group(doc, "MovesInY", "Moves in Y: middle plate and X drive")
    add(doc, "Middle", parts["middle"], "metal", g, "Middle plate")
    add(doc, "XDrive", parts["x_drive"], "drive", g, "X drive")
    g = group(doc, "MovesInXY", "Moves in X and Y: top plate")
    add(doc, "Top", parts["top"], "metal", g, "Top plate")

    corners = [(sx * s, sy * s) for sx in (-1, 1) for sy in (-1, 1)]
    swept = {k: G.bounds(Part.makeCompound([xy_stage(*c)[k] for c in corners]))
             for k in ("middle", "x_drive", "top")}
    g = group(doc, "Travel", "Swept by the moving parts")
    for k, shape in swept.items():
        add(doc, "Sweep" + k.title().replace("_", ""), shape, "reserved", g,
            "Swept by the %s" % k.replace("_", " "))

    found = []
    for c in [(0, 0)] + corners:
        p = xy_stage(*c)
        fixed = Part.makeCompound([p["base"], p["y_drive"]])
        found += clashes([("the %s at %g, %g" % (k.replace("_", " "), *c), fixed, p[k])
                          for k in ("middle", "x_drive", "top")]
                         + [("the top and the X drive at %g, %g" % c, p["top"], p["x_drive"])])
    whole = Part.makeCompound(list(parts.values())).BoundBox
    plate = (99.0, 63.0)  # a 96-well plate's well centres span 99 x 63 mm
    checks = [
        matches("Footprint with the motors, x", whole.XLength, 276.5),
        matches("Footprint with the motors, y", whole.YLength, 276.5),
        matches("Height", whole.ZLength, P.XY_HEIGHT),
        matches("Clear aperture at the centre", aperture(0, 0), 115),
        matches("Clear aperture at a corner of the travel", aperture(s, s), 64),
        Check("The travel covers a 96-well plate's well centres", 2 * s >= max(plate),
              "%g mm of travel, %g x %g mm of wells" % (2 * s, *plate)),
        Check("The moving parts clear the fixed ones across the travel", not found,
              ", ".join(found) or "no overlaps at the centre or the corners"),
    ]
    sweep = Part.makeCompound(list(swept.values()) + [parts["base"], parts["y_drive"]]).BoundBox
    facts = [
        ("Top surface, z", P.XY_HEIGHT),
        ("Travel, each axis", "%g mm, +-%g" % (2 * s, s)),
        ("Footprint swept over the travel, x", "%.1f to %.1f" % (sweep.XMin, sweep.XMax)),
        ("Footprint swept over the travel, y", "%.1f to %.1f" % (sweep.YMin, sweep.YMax)),
        ("Mass, kg", P.XY_MASS),
    ]
    return checks, facts


def build_lift(doc):
    low, high = lift(), lift(P.LIFT_TRAVEL)
    g = group(doc, "Lift", "Z lift: Standa 8MVT100-25-1")
    add(doc, "Fixed", low["fixed"], "drive", g, "Base, column and connector")
    add(doc, "Top", low["top"], "metal", g, "Top, at the bottom of the travel")
    add(doc, "Travel", G.bounds(Part.makeCompound([low["top"], high["top"]])), "reserved",
        group(doc, "Swept", "Swept by the top"), "Swept by the top")
    whole = Part.makeCompound([low["fixed"], low["top"]]).BoundBox
    found = clashes([("at %s" % name, p["fixed"], p["top"])
                     for name, p in (("the bottom", low), ("the top", high))])
    checks = [
        matches("Length with the connector", whole.YLength, 140),
        matches("Height at the bottom of the travel", low["top"].BoundBox.ZMax, 100),
        matches("Height at the top of the travel", high["top"].BoundBox.ZMax, 125),
        Check("The top clears the column over the travel", not found, ", ".join(found) or "no overlaps"),
    ]
    facts = [
        ("Base", "%g x %g mm" % P.LIFT_BASE[:2]),
        ("Top plate", "%g x %g mm, %d M6 holes"
         % (P.LIFT_TOP[0], P.LIFT_TOP[1], len(P.LIFT_TOP_HOLES[0][2]))),
        ("Travel", P.LIFT_TRAVEL),
        ("Load, kg", P.LIFT_LOAD),
        ("Mass, kg", P.LIFT_MASS),
    ]
    return checks, facts

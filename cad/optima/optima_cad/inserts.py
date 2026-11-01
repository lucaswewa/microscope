"""The sample inserts: the adapter on the XY stage's top plate, the 160 x 110 mm inserts that sit
flush in it, and the labware they hold.

Everything is built in the inserts' frame: the origin on the insert's top, at the centre of the
stage's travel, which is the instrument's frame with the stage centred and a slide in focus. Each
insert gives its labware's sample plane, and the areas on that plane the stage has to bring under
the objective: its targets, as (x, y, width, depth, round).
"""

import FreeCAD as App
import Part

from . import geom as G
from . import params as P
from . import stages
from .checks import Check, clashes
from .document import add, group

V = App.Vector
Z = V(0, 0, 1)
CLEAR = P.INSERT_ADAPTER[1]
# How far the matrix's light to a point spreads, per millimetre below it.
SPREAD = ((P.MATRIX_COUNT - 1) / 2 * P.MATRIX_PITCH + P.MATRIX_LED / 2) / P.MATRIX_BELOW_SAMPLE
# Each labware's sample plane above the insert's top: a slide's top is flush with it.
PLANES = {"slides": 0.0, "microplate": P.MICROPLATE[3] - P.INSERT_POCKET[0],
          "dishes": P.DISH_FLOOR - P.INSERT_POCKET[0]}


def adapter():
    """A plate the size of the top plate, with a recess that takes an insert flush with its top, an
    opening under the insert, tabs that sit in the top plate's pockets, and four counterbored M3
    holes over the top plate's."""
    t, c, (ow, od) = P.INSERT_ADAPTER
    w, d, it = P.INSERT
    side, depth = P.XY_PLATE, P.XY_POCKETS[2]
    plate = G.box((side, side, t), V(0, 0, -t / 2)).cut(Part.makeCompound([
        G.box((w + 2 * c, d + 2 * c, it + 1), V(0, 0, (1 - it) / 2)),
        G.box((ow, od, t + 2), V(0, 0, -t / 2))]))
    # The tabs: the pockets' outlines, smaller all round, under the plate outside its opening.
    tabs = [f.makeOffset2D(-c, 2).extrude(V(0, 0, depth - c))
            for f in stages.pocket_faces(-t - depth + c)]
    under = G.box((side, side, depth), V(0, 0, -t - depth / 2))
    under = under.cut(G.box((ow, od, depth + 2), V(0, 0, -t - depth / 2)))
    plate = plate.fuse(Part.makeCompound(tabs).common(under))
    holes = [(x, y) for x, y in P.XY_HOLES[0][2] if abs(y) > d / 2 + c]
    return G.drill(plate, [(3.4, None, holes), (6.5, 3.0, holes)], 0, -Z).removeSplitter()


def _insert(tools):
    w, d, t = P.INSERT
    return G.box((w, d, t), V(0, 0, -t / 2)).cut(Part.makeCompound(tools))


def slides():
    """Four slides along Y, in pockets as deep as they're thick, on ledges under their ends over one
    window, with a notch at each pocket's front end to lift a slide by. The targets are the cover
    glasses, at the slides' centres."""
    t = P.INSERT[2]
    n, pitch, ledge = P.INSERT_SLIDES
    sl, sw, st = P.SLIDE  # its length, along Y here
    pw, pl = sw + 2 * CLEAR, sl + 2 * CLEAR
    xs = [(k - (n - 1) / 2) * pitch for k in range(n)]
    tools = [G.box((pw, pl, st + 1), V(x, 0, (1 - st) / 2)) for x in xs]
    tools.append(G.box((xs[-1] - xs[0] + pw, pl - 2 * ledge, t + 2), V(0, 0, -t / 2)))
    tools += [Part.makeCylinder(5, t, V(x, -pl / 2, -st - 1)) for x in xs]
    gw, gd, gt = P.COVER_GLASS
    glass = ([G.box((sw, sl, st), V(x, 0, -st / 2)) for x in xs]
             + [G.box(P.COVER_GLASS, V(x, 0, gt / 2)) for x in xs])
    return {"insert": _insert(tools), "labware": Part.makeCompound(glass), "plane": PLANES["slides"],
            "targets": [(x, 0.0, gw, gd, False) for x in xs]}


def microplate():
    """A 96-well microplate, its base on the origin: hollow under its wells, inside a 1.5 mm skirt."""
    w, d, h, floor, ft = P.MICROPLATE
    cols, rows, pitch, wd = P.MICROPLATE_WELLS
    wells = [Part.makeCylinder(wd / 2, h, V((i - (cols - 1) / 2) * pitch, (j - (rows - 1) / 2) * pitch,
                                            floor)) for i in range(cols) for j in range(rows)]
    under = G.box((w - 3, d - 3, floor - ft + 1), V(0, 0, (floor - ft - 1) / 2))
    return G.box((w, d, h), V(0, 0, h / 2)).cut(Part.makeCompound(wells + [under]))


def plate_insert():
    """A pocket that the microplate's skirt sits in, on a ledge round a window. The target is the
    wells."""
    depth, ledge = P.INSERT_POCKET
    t = P.INSERT[2]
    w, d = P.MICROPLATE[0] + 2 * CLEAR, P.MICROPLATE[1] + 2 * CLEAR
    cols, rows, pitch, wd = P.MICROPLATE_WELLS
    insert = _insert([G.box((w, d, depth + 1), V(0, 0, (1 - depth) / 2)),
                      G.box((w - 2 * ledge, d - 2 * ledge, t + 2), V(0, 0, -t / 2))])
    return {"insert": insert, "labware": microplate().translated(V(0, 0, -depth)),
            "plane": PLANES["microplate"],
            "targets": [(0.0, 0.0, (cols - 1) * pitch + wd, (rows - 1) * pitch + wd, False)]}


def dish(diameter, height):
    """A dish, its base on the origin: a 1 mm wall, and a 1 mm floor whose underside is 0.5 mm up."""
    r = diameter / 2
    return Part.makeCylinder(r, height).cut(Part.makeCompound([
        Part.makeCylinder(r - 1, height, V(0, 0, P.DISH_FLOOR)),
        Part.makeCylinder(r - 2, P.DISH_FLOOR, V(0, 0, -1))]))


def dish_insert():
    """A pocket for each dish, on a ledge round a round window. The targets are the dishes'
    middles, as big as a glass-bottom dish's glass."""
    depth, ledge = P.INSERT_POCKET
    t = P.INSERT[2]
    tools, dishes = [], []
    for d, h, (x, y) in P.DISHES:
        r = d / 2 + CLEAR
        tools += [Part.makeCylinder(r, depth + 1, V(x, y, -depth)),
                  Part.makeCylinder(r - ledge, t + 2, V(x, y, -t - 1))]
        dishes.append(dish(d, h).translated(V(x, y, -depth)))
    return {"insert": _insert(tools), "labware": Part.makeCompound(dishes), "plane": PLANES["dishes"],
            "targets": [(x, y, P.DISH_GLASS, P.DISH_GLASS, True) for _, _, (x, y) in P.DISHES]}


INSERTS = (("Slides", "Four-slide insert", slides),
           ("Microplate", "Microplate insert", plate_insert),
           ("Dishes", "Dish insert: a 60 mm and two 35 mm dishes", dish_insert))


def reach():
    """How far from the travel's centre the stage brings a point into the widest field, along X and
    Y: the travel, and half the 5x's field."""
    m = P.TUBE_F / max(o["f"] for o in P.OBJECTIVES.values())
    return [P.XY_TRAVEL + s / m / 2 for s in P.SENSOR_SIZE]


def light(insert, bottom):
    """The matrix's light to every target of `insert`, from its sample plane down to `bottom`."""
    z = insert["plane"]
    grow = 2 * SPREAD * (z - bottom)
    out = []
    for x, y, w, d, round_ in insert["targets"]:
        top, low = V(x, y, z), V(x, y, bottom)
        out.append(G.taper(w, top, w + grow, low) if round_
                   else G.rect_loft((w, d), top, (w + grow, d + grow), low))
    return Part.makeCompound(out)


def build_inserts(doc):
    a = adapter()
    add(doc, "AdapterPlate", a, "metal", group(doc, "Adapter", "Adapter, on the XY stage's top plate"),
        "Insert adapter")
    built = {}
    for k, (name, label, make) in enumerate(INSERTS):
        built[name] = b = make()
        g = group(doc, name, label)
        add(doc, name + "Insert", b["insert"].translated(V(200.0 * (k + 1), 0, 0)), "metal", g, label)
        add(doc, name + "Labware", b["labware"].translated(V(200.0 * (k + 1), 0, 0)), "sample", g,
            name.lower() if name != "Slides" else "Slides and cover glasses")

    t = P.INSERT_ADAPTER[0]
    top = stages.xy_stage()["top"].translated(V(0, 0, -t - P.XY_HEIGHT))
    sits = clashes([("the adapter and the top plate", a, top)]
                   + [("the %s and its insert" % n.lower(), b["labware"], b["insert"])
                      for n, b in built.items()]
                   + [("the %s insert and the adapter" % n.lower(), b["insert"], a)
                      for n, b in built.items()])
    rx, ry = reach()
    far = ["the %s at %g, %g" % (n.lower(), x, y) for n, b in built.items()
           for x, y, w, d, _ in b["targets"] if abs(x) + w / 2 > rx or abs(y) + d / 2 > ry]
    dark = clashes([("the %s insert" % n.lower(), light(b, -t), Part.makeCompound([b["insert"], a]))
                    for n, b in built.items()])
    checks = [
        Check("The adapter sits in the top plate's pockets, each insert in the adapter, and the "
              "labware in its pockets", not sits, ", ".join(sits) or "no overlaps"),
        Check("The travel brings every well, cover glass and dish's middle into the 5x's field",
              not far, ", ".join(far) or "all within +-%.1f x +-%.1f mm of the centre" % (rx, ry)),
        Check("The windows let the matrix's light reach every target", not dark,
              ", ".join(dark) or "its light spreads %.2f mm per mm below a target" % SPREAD),
    ]
    w, d, it = P.INSERT
    _, _, (ow, od) = P.INSERT_ADAPTER
    facts = [
        ("Adapter", "%g x %g x %g mm, an insert's recess %g mm deep, an opening %g x %g mm"
         % (P.XY_PLATE, P.XY_PLATE, t, it, ow, od)),
        ("Inserts", "%g x %g x %g mm" % (w, d, it)),
        ("Reach into the 5x's field, from the centre", "+-%.1f x +-%.1f mm" % (rx, ry)),
    ]
    for name, label, _ in INSERTS:
        b = built[name]
        facts.append(("%s: the sample plane above the insert's top, the labware's top above it" % label,
                      "%.2f, %.2f mm" % (b["plane"], b["labware"].BoundBox.ZMax - b["plane"])))
    return checks, facts

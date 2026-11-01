"""The whole instrument: the optical head on the stand.

Its checks cover the head and the stand together; the stage stack with each insert and its labware
across the stage's and the lift's travel, against the head's parts across theirs; and the LED
matrix's light to each insert's targets through the stage's openings. Its facts give the envelope
and the mass.

Everything is in the instrument's frame, with the head's moving parts at their stops as in
optical-head.FCStd, and a slide in focus as in stand.FCStd.
"""

import FreeCAD as App
import Part

from . import geom as G
from . import enclosure, head, inserts, stand
from . import optics as O
from . import params as P
from .checks import Check, clashes
from .document import add, group

V = App.Vector
STAGE = ("base", "middle", "top", "adapter")  # what the matrix's light passes, besides an insert
LEDS = {(round(x, 1), round(y, 1)) for x, y in O.matrix_leds()}
BRIGHT = {n: {led for led in LEDS if O.led_na(*led) <= o["na"]} for n, o in P.OBJECTIVES.items()}


def blocked(st, insert, h, x, y):
    """The LEDs whose light to the field's centre the stage's plates, the adapter or `insert` (None
    for none) block, with the lift `h` from the bottom of its travel and the stage at (x, y). The
    matrix rides on the lift."""
    m = stand.move(st, h, x, y)
    dz = h - P.STAND_FOCUS
    solids = [m[k] for k in STAGE] + ([insert.translated(V(x, y, dz))] if insert is not None else [])
    z = stand.MATRIX_Z + dz
    rays = Part.makeCompound([Part.makeLine(V(lx, ly, z), V(0, 0, 0)) for lx, ly in O.matrix_leds()])
    out = set()
    for e in Part.makeCompound(solids).common(rays).Edges:
        p = e.valueAt((e.FirstParameter + e.LastParameter) / 2)
        out.add((round(p.x * z / p.z, 1), round(p.y * z / p.z, 1)))
    return out


def spots(target):
    """The stage's positions, within its travel, that bring a target's centre to the axis, and the
    middles of its sides and its corners, or a round target's edge every 45 degrees. Each comes
    with whether it's the centre."""
    x, y, w, d, round_ = target
    s = P.XY_TRAVEL
    out = []
    for i in (-1, 0, 1):
        for j in (-1, 0, 1):
            f = 0.5 ** 0.5 if round_ and i and j else 1.0
            px, py = x + i * f * w / 2, y + j * f * d / 2
            at = [round(max(-s, min(s, -c)), 2) + 0.0 for c in (px, py)]  # + 0.0: no -0
            out.append((tuple(at), not (i or j)))
    return out


def build_instrument(doc):
    hs, cd, sd, arm, ch, sl, pt, cams, tube, fold, mirror = head.place(doc, group(doc, "Head",
                                                                                 "Optical head"))
    b, cols, st, lift, camera, bracket, _, _ = stand.place(doc, group(doc, "Stand", "Stand"))
    inside = [o.Shape for o in doc.Objects if o.TypeId == "Part::Feature"]
    named = [o.Name for o in doc.Objects if o.TypeId == "Part::Feature"]
    cover, tr, cables = enclosure.place(doc, arm["leds"], group(doc, "Enclosure", "Enclosure"))
    swept = head.sweeps()
    s = P.XY_TRAVEL
    corners = [(0, 0)] + [(sx * s, sy * s) for sx in (-1, 1) for sy in (-1, 1)]
    travel = [stand.move(st, h, x, y) for h in (0.0, P.LIFT_TRAVEL) for x, y in corners]
    g = group(doc, "Travel", "Swept by the moving parts")
    add(doc, "StackTravel", G.bounds(Part.makeCompound([p for m in travel for p in m.values()])),
        "reserved", g, "Swept by the stage stack over the lift's and the stage's travel")
    add(doc, "ChangerTravel", G.bounds(swept["the changer"]), "reserved", g, "Swept by the changer")
    add(doc, "SliderTravel", G.bounds(swept["the slider"]), "reserved", g, "Swept by the slider")

    # The head's fixed parts, and its moving ones over their travel.
    on_head = {"the head's fixed parts": Part.makeCompound(
        list(hs.values()) + list(cd.values()) + list(sd.values()) + list(arm.values())
        + [tube, fold, mirror, pt["body"]] + [c["base"] for c in cams.values()])}
    on_head.update(swept)
    on_stand = {"the base plate": b["plate"], "the feet": b["feet"],
                "the column": Part.makeCompound(cols), "the lift's base": lift,
                "the overview camera": Part.makeCompound(list(camera.values()) + [bracket])}
    apart = clashes([("%s and %s" % (a, c), on_head[a], on_stand[c]) for a in on_head for c in on_stand])

    # Each insert and its labware on the stage, across the travel, against the head and the
    # overview camera; and the gap between the objectives and the labware at the lift's top.
    near = dict(on_head, **{"the overview camera": on_stand["the overview camera"]})
    carried = {k: v for k, v in st.items() if k not in ("insert", "slides")}
    front = swept["the changer"].BoundBox.ZMin
    hits, gaps, light = [], [], {}
    for name, _, make in inserts.INSERTS:
        ins = make()
        focus = P.STAND_FOCUS - ins["plane"]
        for h in (0.0, focus, P.LIFT_TRAVEL):
            dz = h - P.STAND_FOCUS
            for x, y in corners:
                moving = Part.makeCompound(list(stand.move(carried, h, x, y).values())
                                           + [ins[k].translated(V(x, y, dz))
                                              for k in ("insert", "labware")])
                hits += clashes([("the %s insert and %s, the lift at %g mm and the stage at %g, %g"
                                  % (name.lower(), k, h, x, y), moving, v) for k, v in near.items()])
        top = ins["labware"].BoundBox.ZMax + P.LIFT_TRAVEL - P.STAND_FOCUS
        gaps.append((front - top, name.lower()))
        for target in ins["targets"]:
            for (x, y), centre in spots(target):
                key = (name.lower(), x, y)
                if key not in light:
                    light[key] = [blocked(st, ins["insert"], focus, x, y), False]
                light[key][1] |= centre

    # How far the 20x's brightfield light stays whole along X, with nothing on the stage.
    widest = max(P.OBJECTIVES, key=lambda n: P.OBJECTIVES[n]["na"])
    lo, hi = 0.0, s
    if not blocked(st, None, P.STAND_FOCUS, hi, 0) & BRIGHT[widest]:
        lo = hi
    while hi - lo > 0.05:
        mid = (lo + hi) / 2
        lo, hi = (lo, mid) if blocked(st, None, P.STAND_FOCUS, mid, 0) & BRIGHT[widest] else (mid, hi)
    short = ["the %s at %g, %g, the %s" % (key + (o,)) for key, (out, centre) in light.items()
             for o in P.OBJECTIVES if out & BRIGHT[o] and (centre or o != widest)]
    worst = max(light.values(), key=lambda v: len(v[0]))[0]
    lost = max(len(out & BRIGHT[widest]) for out, _ in light.values())

    # What stands on the base, with the moving parts across their travel; and the envelope.
    box, whole = App.BoundBox(), App.BoundBox()
    for name, shape in zip(named, inside):
        (whole if name in ("BasePlate", "Feet") else box).add(shape.BoundBox)
    for shape in list(swept.values()) + [p for m in travel for p in m.values()]:
        box.add(shape.BoundBox)
    whole.add(box)
    x0, y0, x1, y1, _ = P.STAND_BASE

    # The mass, without the enclosure and the electronics.
    aluminium = {"base plate": b["plate"], "head plate": hs["plate"], "lower deck": hs["lower"],
                 "upper deck": hs["upper"], "epi plate": hs["epi"],
                 "camera port's bracket": hs["bracket"], "changer's plate": ch["plate"],
                 "slider's carrier": sl["carrier"], "fold mirror's mount": fold,
                 "plate on the lift": st["plate"], "insert adapter": st["adapter"]}
    rest = ([cd["bracket"], cd["block"], sd["bracket"], bracket]
            + [c[k] for c in cams.values() for k in ("base", "plate")]
            + [st[k] for k in ("spacers", "mount", "insert")])
    kg = {k: P.ALUMINIUM * v.Volume for k, v in aluminium.items()}
    metal = sum(kg.values()) + P.ALUMINIUM * sum(v.Volume for v in rest)
    column = len(cols) * cols[0].BoundBox.ZLength / 1000 * P.EXTRUSION_KG_PER_M
    bought = P.XY_MASS + P.LIFT_MASS + P.MONO_MASS + sum(P.MASSES.values()) + column
    heavy = sorted(kg, key=kg.get, reverse=True)[:4]

    gap, tight = min(gaps)
    fine = [n for n in P.OBJECTIVES if n != widest]
    checks = [
        Check("The head and the stand don't overlap, the head's moving parts across their travel",
              not apart, ", ".join(apart) or "no overlaps"),
        Check("Each insert and its labware clear the head across the stage's and the lift's travel, "
              "and the objectives as they change", not hits and gap > 0,
              ", ".join(hits[:4]) or "the objectives' fronts %.1f mm above the %s with the lift at the "
              "top of its travel" % (gap, tight)),
        Check("The stage's openings pass the brightfield light of the %s at every target, and of the "
              "%s at every target's centre" % (" and the ".join(fine), widest), not short,
              ", ".join(short[:4]) or "%d stage positions, to the field's centre" % len(light)),
        Check("Everything stays over the base plate, the moving parts across their travel",
              box.XMin >= x0 and box.XMax <= x1 and box.YMin >= y0 and box.YMax <= y1,
              "x %.1f to %.1f, y %.1f to %.1f; the base %g to %g, %g to %g"
              % (box.XMin, box.XMax, box.YMin, box.YMax, x0, x1, y0, y1)),
    ]
    facts = [
        ("Envelope, x y z", "%.1f to %.1f, %.1f to %.1f, %.1f to %.1f"
         % (whole.XMin, whole.XMax, whole.YMin, whole.YMax, whole.ZMin, whole.ZMax)),
        ("Envelope, mm", "%.0f x %.0f x %.0f" % (whole.XLength, whole.YLength, whole.ZLength)),
        ("The %s's brightfield light is whole" % widest, "within +-%.1f mm of the centre along X" % lo),
        ("At the worst target, the %s's brightfield LEDs blocked" % widest,
         "%d of %d" % (lost, len(BRIGHT[widest]))),
        ("At the worst target, all LEDs blocked", "%d of %d, from NA %.2f"
         % (len(worst), len(LEDS), min(O.led_na(*led) for led in worst)) if worst else "none"),
        ("Mass without the enclosure and the electronics, kg", round(metal + bought, 1)),
        ("Custom aluminium parts, kg", round(metal, 1)),
        ("Heaviest custom parts, kg", ", ".join("the %s %.1f" % (k, kg[k]) for k in heavy)),
        ("Bought parts, kg", "%.1f, with %.1f for the column's extrusions" % (bought, column)),
    ]
    stack = Part.makeCompound([p for m in travel for p in m.values()])
    more, extra, kept = enclosure.checks(cover, tr, cables, inside, list(swept.values()) + [stack])
    facts.append(("Mass with the enclosure and the electronics, kg", round(metal + bought + kept, 1)))
    return checks + more, facts + extra

"""The enclosure and the electronics: the cover over the instrument, with its door, interlock, intake
and fan; the electronics trays under the stage, and the mains inlet; and the cable ducts.

Everything is in the instrument's frame. The cover stands on the table over the instrument, apart
from it, on a foam strip, and lifts off: a frame of extrusions, panels outside it, and a door over
the whole front, hinged at its left. The door takes an angle in degrees, open.
"""

import math
from functools import cache

import FreeCAD as App
import Part

from . import geom as G
from . import fasteners, layout, parts, stand
from . import params as P
from .checks import Check, clashes
from .document import add, group
from .fasteners import Screw

V = App.Vector
X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
X0, Y0, X1, Y1, TOP = P.ENCLOSURE
F, T = P.ENCLOSURE_FRAME[0], P.ENCLOSURE_PANEL
TABLE = stand.BASE_TOP - P.STAND_BASE[4] - P.STAND_FEET[1]
FOOT = TABLE + P.ENCLOSURE_SEAL  # the frame's foot, on the foam strip
OUT = (X0 - F - T, Y0 - F - T, X1 + F + T, Y1 + F + T, TOP + F + T)  # outside, the door shut
HINGE = V(OUT[0], OUT[1], 0)  # the door's hinge axis, upright at the front left corner
INLET_LEG = 3.0  # the inlet's bracket's thickness
INLET_Z = stand.BASE_TOP + INLET_LEG + 1 + (P.INLET[3] - 6) / 2  # its body just over the bracket


def _ring(a0, b0, a1, b1):
    """Points round a rectangle, its corners among them, at most ENCLOSURE_SCREWS apart."""
    corners = [(a0, b0), (a1, b0), (a1, b1), (a0, b1)]
    out = []
    for (pa, pb), (qa, qb) in zip(corners, corners[1:] + corners[:1]):
        n = math.ceil(max(abs(qa - pa), abs(qb - pb)) / P.ENCLOSURE_SCREWS)
        out += [(pa + (qa - pa) * k / n, pb + (qb - pb) * k / n) for k in range(n)]
    return out


@cache
def screws():
    """The enclosure's screws: the panels' into T-nuts in the frame, the door's hinge and handle,
    the baffles' on their spacers, the fan's, and the supply's and the boards' on the tray."""
    m = F / 2
    out = []
    for y, z in _ring(Y0 - m, FOOT + m, Y1 + m, TOP + m):
        out += [Screw(4, (OUT[0], y, z), (1, 0, 0), T, 0, "left panel", "frame"),
                Screw(4, (OUT[2], y, z), (-1, 0, 0), T, 0, "right panel", "frame")]
    out += [Screw(4, (x, OUT[3], z), (0, -1, 0), T, 0, "back panel", "frame")
            for x, z in _ring(X0 - m, FOOT + m, X1 + m, TOP + m)]
    out += [Screw(4, (x, y, OUT[4]), (0, 0, -1), T, 0, "top panel", "frame")
            for x, y in _ring(X0 - m, Y0 - m, X1 + m, Y1 + m)]
    span = OUT[4] - FOOT - 80
    n = math.ceil(span / P.ENCLOSURE_SCREWS)
    out += [Screw(4, (OUT[0] + 8, OUT[1], FOOT + 40 + k * span / n), (0, 1, 0), T, 0, "door", None)
            for k in range(n + 1)]
    out += [Screw(4, (OUT[2] - 50, Y0 - F, z), (0, -1, 0), T, 0, "door", None) for z in (10, 70)]
    (iy, iz), (iw, ih) = P.INTAKE
    side, depth, _ = P.FAN
    gap, overlap = P.BAFFLE
    e = overlap * gap - 6  # the spacers, 6 mm in from the baffles' corners
    out += [Screw(3, (OUT[2], iy + dy, iz + dz), (-1, 0, 0), 2 * T + gap, 0,
                  ("right panel", "intake baffle"), None)
            for dy in (-iw / 2 - e, iw / 2 + e) for dz in (-ih / 2 - e, ih / 2 + e)]
    f = fan_at()
    out += [Screw(3, (f.x + dx, OUT[3], f.z + dz), (0, -1, 0), OUT[3] - f.y + depth / 2 + gap + T, 0,
                  ("back panel", "fan baffle"), None)
            for dx in (-side / 2 - e, side / 2 + e) for dz in (-side / 2 - e, side / 2 + e)]
    k = side / 2 - 7.5  # a 120 mm fan's holes, 105 mm apart
    out += [Screw(4, (f.x + dx, OUT[3], f.z + dz), (0, -1, 0), T + depth, 0, ("back panel", "fan"), None)
            for dx in (-k, k) for dz in (-k, k)]
    # The supply into its base's M3s, and the boards on 5 mm spacers, from under their trays. The
    # DIN rail and the fuse block on M3s and M4s through them, with nuts under the tray.
    def at(name):
        tray, x, y = P.TRAY_PARTS[name]
        return tray + " tray", x, y, stand.BASE_TOP + P.TRAYS[tray][5], P.TRAYS[tray][4]

    tray, sx, sy, under, tt = at("supply")
    out += [Screw(3, (sx, sy + dy, under), (0, 0, 1), tt, 4, tray, "supply")
            for dy in (-P.TRAY_SUPPLY[0] / 2, P.TRAY_SUPPLY[0] / 2)]
    a, b = P.TRAY_CARRIER
    for name, across, along in [("SKR Pico", P.SKR_PICO[3][1] / 2, P.SKR_PICO[3][0] / 2),
                                ("carrier board", b / 2 - 4, a / 2 - 4)]:
        tray, cx, cy, under, tt = at(name)
        out += [Screw(3, (cx + dx, cy + dy, under), (0, 0, 1), tt + 5 + 1.6, 0, (tray, "boards"), None)
                for dx in (-across, across) for dy in (-along, along)]
    tray, rx, ry, under, tt = at("DIN rail")
    end, h = P.TRAY_RAIL[1] / 2 - 6, P.TRAY_RAIL[2]
    out += [Screw(3, (rx + dx, ry, under + tt + h), (0, 0, -1), h + tt, 0, ("rail", tray), None)
            for dx in (-end, end)]
    tray, fx, fy, under, tt = at("fuse block")
    h = P.TRAY_FUSES[2]
    out += [Screw(4, (fx + dx, fy, under + tt + h), (0, 0, -1), h + tt, 0, ("fuses", tray), None)
            for dx in (-40, 40)]
    # The inlet through its bracket's upright leg, with nuts behind it.
    x, face = P.INLET[:2]
    out += [Screw(3, (x + dx, face, INLET_Z), (0, -1, 0), 2 * INLET_LEG, 0, ("inlet", "inlet bracket"),
                  None) for dx in (-20, 20)]
    return tuple(out)


def frame():
    """An upright at each corner, and rails round the foot and the top."""
    out = [G.span(V(x, y, FOOT), V(x + F, y + F, TOP + F)) for x in (X0 - F, X1) for y in (Y0 - F, Y1)]
    for z in (FOOT, TOP):
        out += [G.span(V(X0, y, z), V(X1, y + F, z + F)) for y in (Y0 - F, Y1)]
        out += [G.span(V(x, Y0, z), V(x + F, Y1, z + F)) for x in (X0 - F, X1)]
    return out


def fan_at():
    """The fan's centre, in the back panel behind the mono camera."""
    c = layout.camera_frames()["mono"][0]
    return V(c.x, Y1 + F - P.FAN[1] / 2, c.z)


@G.kept
def panels():
    """The left, right, back and top panels outside the frame: the right with the intake, the back
    with the fan's opening and, just above its bottom rail, the notch the cables leave by. And the
    baffles inside the intake and the fan, and the fan."""
    x0, _, x1, y1, top = OUT
    (iy, iz), (iw, ih) = P.INTAKE
    side, depth, _ = P.FAN
    gap, overlap = P.BAFFLE
    m = 2 * overlap * gap
    f = fan_at()
    nw, nh = P.ENCLOSURE_NOTCH
    back = G.span(V(X0 - F, Y1 + F, FOOT), V(X1 + F, y1, TOP + F)).cut(Part.makeCompound([
        Part.makeCylinder(side / 2 - 2, 3 * T, V(f.x, Y1 + F - T, f.z), Y),
        G.box((nw, 3 * T, nh), V(0, Y1 + F + T / 2, FOOT + F + nh / 2))]))
    bore = Part.makeCylinder(side / 2 - 2, depth + 2, f - Y * (depth / 2 + 1), Y)
    fan = G.box((side, depth, side), f).cut(bore)
    s = screws()
    return {k: fasteners.cut(v, s, k if k in ("intake baffle", "fan baffle", "fan") else k + " panel")
            for k, v in (
                ("left", G.span(V(x0, Y0 - F, FOOT), V(X0 - F, y1, TOP + F))),
                ("right", G.span(V(X1 + F, Y0 - F, FOOT), V(x1, y1, TOP + F))
                 .cut(G.box((3 * T, iw, ih), V(X1 + F + T / 2, iy, iz)))),
                ("back", back),
                ("top", G.span(V(x0, Y0 - F, TOP + F), V(x1, y1, top))),
                ("intake baffle", G.box((T, iw + m, ih + m), V(X1 + F - gap - T / 2, iy, iz))),
                ("fan baffle", G.box((side + m, T, side + m), f - Y * (depth / 2 + gap + T / 2))),
                ("fan", fan))}


@G.kept
def door(angle=0.0, handle=True):
    """The door over the whole front, with a handle by its right edge, `angle` degrees open."""
    x0, y0, x1, _, top = OUT
    d = G.span(V(x0, y0, FOOT), V(x1, Y0 - F, top))
    if handle:
        d = d.fuse(G.span(V(x1 - 60, y0 - 25, -20), V(x1 - 40, y0, 80)))
    d = fasteners.cut(d, screws(), "door")
    d.rotate(HINGE, Z, -angle)
    return d


def interlock():
    """The door's switch, on a bracket from the right front upright, and its plunger, out by its
    travel to where the shut door presses it."""
    w, h, depth, travel = P.INTERLOCK
    face = Y0 - F
    switch = G.span(V(X1 - 5 - w, face, -h / 2), V(X1 - 5, face + depth, h / 2))
    bracket = G.span(V(X1 - 5, face, -h / 2), V(X1, face + depth, h / 2))
    plunger = Part.makeCylinder(2, travel + 1, V(X1 - 5 - w / 2, face + 1, 0), -Y)
    return {"switch": switch.fuse(bracket), "plunger": plunger}


def inlet():
    """The mains inlet, its face in the cover's notch, and its bracket: an angle screwed to the base
    plate, its upright leg past the plate's back edge."""
    x, face, w, h, depth = P.INLET
    t, z, base = INLET_LEG, INLET_Z, stand.BASE_TOP
    body = G.span(V(x - w / 2, face - t, z - h / 2), V(x + w / 2, face, z + h / 2)).fuse(
        G.box((w - 18, depth, h - 6), V(x, face - t - depth / 2, z)))
    leg = face - 2 * t
    bracket = G.span(V(x - w / 2 - 6, P.STAND_BASE[3] - 35, base), V(x + w / 2 + 6, leg + t, base + t))
    bracket = bracket.fuse(G.span(V(x - w / 2 - 6, leg, base), V(x + w / 2 + 6, leg + t, z + h / 2 + 5)))
    bracket = bracket.cut(G.box((w - 17, 3 * t, h - 5), V(x, leg + t / 2, z))).removeSplitter()
    return {"inlet": body, "inlet bracket": bracket}


@G.kept
def tray():
    """The electronics trays on their standoffs on the base plate. On the power tray: the supply,
    the fuse block, and the 12 V and 15 V converters on their DIN rail. On the control tray: the
    motion controller (the SKR Pico); the carrier board with the optics controller (the Pico 2) and
    the four LED drivers; and the USB hub. Each part is turned with its first size along Y. And the
    mains inlet on its bracket."""
    def floor(tray):
        return stand.BASE_TOP + P.TRAYS[tray][5] + P.TRAYS[tray][4]

    def turned(shapes, name, lift=0.0):
        tray, x, y = P.TRAY_PARTS[name]
        return Part.makeCompound([G.placed(v, V(x, y, floor(tray) + lift), Z, -X) for v in shapes])

    def block(size, lift=0.0):
        return G.box(size, V(0, 0, lift + size[2] / 2))

    out = {k + " tray": G.span(V(x0, y0, floor(k) - t), V(x1, y1, floor(k)))
           for k, (x0, y0, x1, y1, t, _) in P.TRAYS.items()}
    out["standoffs"] = Part.makeCompound([Part.makeCylinder(4, s, V(x, y, stand.BASE_TOP))
                                          for x0, y0, x1, y1, _, s in P.TRAYS.values()
                                          for x in (x0 + 8, x1 - 8) for y in (y0 + 8, y1 - 8)])
    a, b = P.TRAY_CARRIER
    lw, ld, lh = P.LDD_H
    carrier = [G.box((a, b, 1.6), V(0, 0, 0.8))]
    carrier += [G.box(P.LDD_H, V(sx * (lw / 2 + 4) - 28, sy * (ld / 2 + 2), 1.6 + lh / 2))
                for sx in (-1, 1) for sy in (-1, 1)]
    carrier += [v.translated(V(36, 0, 1.6)) for v in parts.circuit(P.PICO2).values()]
    out.update({
        "supply": turned([block(P.SUPPLY)], "supply"),
        "fuses": turned([block(P.TRAY_FUSES)], "fuse block"),
        "rail": turned([block(P.TRAY_RAIL)], "DIN rail"),
        "converters": Part.makeCompound([turned([block(v, P.TRAY_RAIL[2])], k)
                                         for k, v in P.TRAY_CONVERTERS.items()]),
        "boards": Part.makeCompound([turned(parts.circuit(P.SKR_PICO).values(), "SKR Pico", 5),
                                     turned(carrier, "carrier board", 5)]),
        "hub": turned([block(P.TRAY_HUB)], "USB hub"),
    })
    out.update(inlet())
    held = stand.screws() + screws()
    return {k: fasteners.cut(v, held, k) for k, v in out.items()}


def ducts():
    return Part.makeCompound([G.span(V(x0, y0, stand.BASE_TOP), V(x1, y1, top))
                              for x0, y0, x1, y1, top in P.DUCTS])


def plugs(leds):
    """The room each mounted LED's cable needs, behind it along its axis."""
    out = []
    for led, (*_, axis) in zip(leds.Solids, layout.epi_leds()):
        b = led.BoundBox
        lo, hi = V(b.XMin, b.YMin, b.ZMin), V(b.XMax, b.YMax, b.ZMax)
        if axis.x > 0.5:
            lo.x, hi.x = b.XMax, b.XMax + P.EPI_PLUG
        else:
            lo.y, hi.y = b.YMax, b.YMax + P.EPI_PLUG
        out.append(G.span(lo, hi))
    return Part.makeCompound(out)


def place(doc, leds, parent=None):
    """Adds the cover, the electronics and the cables' room to `doc`, in groups under `parent` if
    it's given. `leds` are the epi arm's mounted LEDs. Returns them."""
    def new_group(name, label):
        g = group(doc, name, label)
        if parent is not None:
            parent.addObject(g)
        return g

    x0, y0, x1, y1, _ = OUT
    seal = G.span(V(x0, y0 + T, TABLE), V(x1, y1, FOOT))
    seal = seal.cut(G.span(V(X0, Y0, TABLE - 1), V(X1, Y1, FOOT + 1)))
    cover = {"frame": Part.makeCompound(frame()), "seal": seal, **panels(), "door": door(),
             **interlock()}
    g = new_group("Cover", "Cover: frame, panels, door and interlock, intake and fan")
    for k, s in cover.items():
        style = {"frame": "metal", "seal": "drive", "fan": "drive", "switch": "drive",
                 "plunger": "drive"}.get(k, "panel")
        add(doc, "Cover" + k.title().replace(" ", ""), s, style, g, "Cover's " + k)
    tr = tray()
    g = new_group("Electronics", "Electronics trays, their parts, and the mains inlet")
    for k, label in (("power tray", "Power tray"), ("control tray", "Control tray"),
                     ("standoffs", "Trays' standoffs"), ("supply", "Supply: Mean Well LRS-150-24"),
                     ("fuses", "Fuse block"), ("rail", "DIN rail"),
                     ("converters", "12 V and 15 V converters: Mean Well DDR-60G-12 and DDR-30G-15"),
                     ("boards", "SKR Pico; carrier board with the Pico 2 and four LDD-1000H"),
                     ("hub", "USB 3 hub"), ("inlet", "Mains inlet"), ("inlet bracket", "Inlet's bracket")):
        style = {"boards": "board", "converters": "body", "fuses": "body", "hub": "body",
                 "inlet": "body"}.get(k, "metal")
        add(doc, k.title().replace(" ", ""), tr[k], style, g, label)
    cables = {"ducts": ducts(), "plugs": plugs(leds)}
    g = new_group("Cables", "Cable ducts, and the epi LEDs' cables' room")
    add(doc, "Ducts", cables["ducts"], "drive", g, "Cable ducts")
    add(doc, "EpiPlugs", cables["plugs"], "reserved", g, "Room for the epi LEDs' cables")
    return cover, tr, cables


def checks(cover, tr, cables, inside, swept):
    """The enclosure's checks and facts. `inside` are the instrument's parts, the head's moving ones
    at their stops; `swept`, its moving parts across their travel."""
    walls = {k: v for k, v in cover.items() if k not in ("door", "plunger")}
    every = inside + swept + [cables["plugs"]]
    found = clashes([("the cover's %s and the instrument" % k, v, s)
                     for k, v in dict(walls, door=cover["door"]).items()
                     for s in every + list(tr.values())])
    box = App.BoundBox()
    for s in every + list(tr.values()) + [cables["ducts"]]:
        box.add(s.BoundBox)
    room = min(box.XMin - X0, X1 - box.XMax, box.YMin - Y0, Y1 - box.YMax, TOP - box.ZMax)

    shut = cover["door"].common(cover["plunger"]).Volume > 1e-6
    r = cover["plunger"].BoundBox.Center.x - HINGE.x
    ajar = math.degrees(math.asin((P.INTERLOCK[3] + 1) / r))
    released = door(ajar).common(cover["plunger"]).Volume < 1e-9
    frame_ = Part.makeCompound([walls[k] for k in ("frame", "left", "right", "back", "top")])
    swing = clashes([("the door at %d degrees" % a, door(a), frame_) for a in range(15, 121, 15)])

    gap = P.BAFFLE[0]
    iw, ih = P.INTAKE[1]
    ib = cover["intake baffle"].BoundBox
    fb = cover["fan baffle"].BoundBox
    side = P.FAN[0]
    margins = [(ib.YLength - iw) / 2, (ib.ZLength - ih) / 2, (fb.XLength - side) / 2]
    angle = math.degrees(math.atan(min(margins) / gap))

    names = list(tr)
    stack = clashes([("the %s and the instrument" % k, v, s) for k, v in tr.items() for s in every]
                    + [("the %s and the ducts" % k, v, cables["ducts"]) for k, v in tr.items()]
                    + [("the ducts and the instrument", cables["ducts"], s) for s in every]
                    + [("the %s and the %s" % (a, b), tr[a], tr[b])
                       for i, a in enumerate(names) for b in names[i + 1:]])

    length = 4 * (TOP + F - FOOT) + 4 * (X1 - X0) + 4 * (Y1 - Y0)  # the frame's extrusions
    frame_m = length / 1000 * P.ENCLOSURE_FRAME[1]
    sheet = P.ALUMINIUM * sum(cover[k].Volume for k in ("left", "right", "back", "top", "door",
                                                          "intake baffle", "fan baffle"))
    electronics = P.ELECTRONICS_KG + P.ALUMINIUM * sum(tr[k].Volume for k in (
        "power tray", "control tray", "inlet bracket"))
    flow = P.FAN[2] / 3600 * 1.2 * 1005  # watts per kelvin of air the fan moves
    checks_ = [
        Check("The cover clears the instrument, its cables' room and its moving parts across their "
              "travel, and lifts straight off", not found and room >= 0,
              ", ".join(found[:4]) or "%.1f mm round it at least, inside the frame" % room),
        Check("The door swings open clear of the cover, and presses the interlock only when shut",
              shut and released and not swing,
              ", ".join(swing) or "it swings 120 degrees; %.2f degrees open releases the switch" % ajar),
        Check("The intake and the fan are baffled against light", angle >= 70,
              "no straight line within %.0f degrees of their axis gets past the baffles" % angle),
        Check("The electronics, the inlet and the cable ducts clear each other and the instrument, "
              "the stage stack across its travel", not stack, ", ".join(stack[:4]) or "no overlaps"),
    ]
    facts = [
        ("Cover, outside, the door shut", "%.0f x %.0f x %.0f mm"
         % (OUT[2] - OUT[0], OUT[3] - OUT[1], OUT[4] - TABLE)),
        ("Intake; fan", "%.0f cm2; %g mm, %g m3/h through the baffles"
         % (iw * ih / 100, side, P.FAN[2])),
        ("Air's warming inside, typical and peak draw", "%.1f K at %g W, %.1f K at %g W"
         % (P.TYPICAL_W / flow, P.TYPICAL_W, P.PEAK_W / flow, P.PEAK_W)),
        ("Cover, kg", round(frame_m + sheet, 1)),
        ("Electronics and their trays, kg", round(electronics, 1)),
    ]
    return checks_, facts, frame_m + sheet + electronics

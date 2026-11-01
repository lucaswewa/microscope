"""The enclosure and the electronics: the cover over the instrument, with its door, interlock, intake
and fan; the electronics tray under the stage; and the cable ducts.

Everything is in the instrument's frame. The cover stands on the table over the instrument, apart
from it, on a foam strip, and lifts off: a frame of extrusions, panels outside it, and a door over
the whole front, hinged at its left. The door takes an angle in degrees, open.
"""

import math

import FreeCAD as App
import Part

from . import geom as G
from . import layout, parts, stand
from . import params as P
from .checks import Check, clashes
from .document import add, group

V = App.Vector
X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
X0, Y0, X1, Y1, TOP = P.ENCLOSURE
F, T = P.ENCLOSURE_FRAME[0], P.ENCLOSURE_PANEL
TABLE = stand.BASE_TOP - P.STAND_BASE[4] - P.STAND_FEET[1]
FOOT = TABLE + P.ENCLOSURE_SEAL  # the frame's foot, on the foam strip
OUT = (X0 - F - T, Y0 - F - T, X1 + F + T, Y1 + F + T, TOP + F + T)  # outside, the door shut
HINGE = V(OUT[0], OUT[1], 0)  # the door's hinge axis, upright at the front left corner


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
    return {
        "left": G.span(V(x0, Y0 - F, FOOT), V(X0 - F, y1, TOP + F)),
        "right": G.span(V(X1 + F, Y0 - F, FOOT), V(x1, y1, TOP + F))
        .cut(G.box((3 * T, iw, ih), V(X1 + F + T / 2, iy, iz))),
        "back": back,
        "top": G.span(V(x0, Y0 - F, TOP + F), V(x1, y1, top)),
        "intake baffle": G.box((T, iw + m, ih + m), V(X1 + F - gap - T / 2, iy, iz)),
        "fan baffle": G.box((side + m, T, side + m), f - Y * (depth / 2 + gap + T / 2)),
        "fan": fan,
    }


def door(angle=0.0):
    """The door over the whole front, with a handle by its right edge, `angle` degrees open."""
    x0, y0, x1, _, top = OUT
    handle = G.span(V(x1 - 60, y0 - 25, -20), V(x1 - 40, y0, 80))
    d = G.span(V(x0, y0, FOOT), V(x1, Y0 - F, top)).fuse(handle)
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


def tray():
    """The electronics tray on its standoffs on the base plate, with the supply, the motion
    controller (the SKR Pico), the optics controller (the Pico 2) on its carrier, and the four LED
    drivers on theirs. Each is turned with its long side along Y."""
    x0, y0, x1, y1, t, s = P.TRAY
    floor = stand.BASE_TOP + s + t
    feet = [Part.makeCylinder(4, s, V(x, y, stand.BASE_TOP)) for x in (x0 + 8, x1 - 8)
            for y in (y0 + 8, y1 - 8)]
    plate = G.span(V(x0, y0, floor - t), V(x1, y1, floor)).fuse(feet)

    def turned(shapes, name, lift=0.0):
        x, y = P.TRAY_PARTS[name]
        return Part.makeCompound([G.placed(v, V(x, y, floor + lift), Z, -X) for v in shapes])

    carriers = {k: G.box((a, b, 1.6), V(0, 0, 0.8)) for k, (a, b) in P.CARRIERS.items()}
    lw, ld, lh = P.LDD_H
    drivers = [G.box(P.LDD_H, V(sx * (lw / 2 + 4), sy * (ld / 2 + 2), 1.6 + lh / 2))
               for sx in (-1, 1) for sy in (-1, 1)]
    return {
        "tray": plate,
        "supply": turned([G.box(P.SUPPLY, V(0, 0, P.SUPPLY[2] / 2))], "supply"),
        "boards": Part.makeCompound([
            turned(parts.circuit(P.SKR_PICO).values(), "motion", 5),
            turned([carriers["optics"]], "optics", 5),
            turned(parts.circuit(P.PICO2).values(), "optics", 6.6),
            turned([carriers["drivers"]] + drivers, "drivers", 5)]),
    }


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
    g = new_group("Electronics", "Electronics tray: supply, controllers and LED drivers")
    add(doc, "Tray", tr["tray"], "metal", g, "Electronics tray")
    add(doc, "Supply", tr["supply"], "metal", g, "Supply: Mean Well LRS-150-24")
    add(doc, "Boards", tr["boards"], "board", g, "SKR Pico, Pico 2 and LDD-H drivers on carriers")
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
                     for k, v in dict(walls, door=cover["door"]).items() for s in every])
    box = App.BoundBox()
    for s in every + [tr["tray"], tr["supply"], tr["boards"], cables["ducts"]]:
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

    parts_ = {"tray": tr["tray"], "supply": tr["supply"], "boards": tr["boards"]}
    stack = clashes([("the %s and the instrument" % k, v, s) for k, v in parts_.items() for s in every]
                    + [("the %s and the ducts" % k, v, cables["ducts"]) for k, v in parts_.items()]
                    + [("the ducts and the instrument", cables["ducts"], s) for s in every])

    length = 4 * (TOP + F - FOOT) + 4 * (X1 - X0) + 4 * (Y1 - Y0)  # the frame's extrusions
    frame_m = length / 1000 * P.ENCLOSURE_FRAME[1]
    sheet = P.ALUMINIUM * sum(cover[k].Volume for k in ("left", "right", "back", "top", "door",
                                                          "intake baffle", "fan baffle"))
    electronics = P.ELECTRONICS_KG + P.ALUMINIUM * tr["tray"].Volume
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
        Check("The electronics tray and the cable ducts clear the instrument, the stage stack across "
              "its travel", not stack, ", ".join(stack[:4]) or "no overlaps"),
    ]
    facts = [
        ("Cover, outside, the door shut", "%.0f x %.0f x %.0f mm"
         % (OUT[2] - OUT[0], OUT[3] - OUT[1], OUT[4] - TABLE)),
        ("Intake; fan", "%.0f cm2; %g mm, %g m3/h through the baffles"
         % (iw * ih / 100, side, P.FAN[2])),
        ("Air's warming inside, typical and peak draw", "%.1f K at %g W, %.1f K at %g W"
         % (P.TYPICAL_W / flow, P.TYPICAL_W, P.PEAK_W / flow, P.PEAK_W)),
        ("Cover, kg", round(frame_m + sheet, 1)),
        ("Electronics and their tray, kg", round(electronics, 1)),
    ]
    return checks_, facts, frame_m + sheet + electronics

"""The cables: each one's two ends and the connectors there, its conductors, and its length, taken
from the instrument model.

A cable between two places at the base runs across the base plate, in its ducts where they lead.
One to the head runs up the duct behind the head plate and out through the slot by its deck. A
cable's route is the sum of its runs along X, Y and Z between those points, at its longest across
its ends' travel. Its length adds each end's allowance, a loop if an end moves, and is rounded up to
5 cm, or to the next length its kind is bought in.

Writes the cable list, and checks the conductors' ratings, the power runs' voltage drops, the bought
cables' lengths and the USB cables' lengths.
"""

import csv
import math
import os
from collections import defaultdict, namedtuple
from functools import cache

import FreeCAD as App

from . import enclosure, head, stand
from . import params as P
from .checks import Check

V = App.Vector
# A cable: its code; what it carries; each end's place, and the connector there; its conductors,
# their count and size in mm², if they're ours to choose; for power, its peak current, the current
# its fuse or source allows, and its voltage; and its kind: "made" for ours, else what it's bought as.
Cable = namedtuple("Cable", "code what a a_end b b_end wires peak limit volts kind",
                   defaults=(None, None, None, None, "made"))
CABLES = [
    # Mains, and the protective earth from the stud on the base plate.
    Cable("W01", "Mains: live and neutral", "mains inlet", "6.3 mm push-on terminals", "supply",
          "L and N terminals", (2, 0.75), 1.3, 4.0, 115.0),
    Cable("W02", "Earth: the inlet's", "mains inlet", "6.3 mm push-on terminal", "earth stud",
          "M6 ring terminal", (1, 0.75), limit=4.0),
    Cable("W03", "Earth: the supply's", "earth stud", "M6 ring terminal", "supply", "earth terminal",
          (1, 0.75), limit=4.0),
    Cable("W04", "Earth: the column's", "earth stud", "M6 ring terminal", "column",
          "M8 ring terminal, under a column screw's head", (1, 0.75), limit=4.0),
    Cable("W05", "Earth: the cover's", "earth stud", "M6 ring terminal", "cover's plug",
          "6.3 mm push-on terminal, onto the cover frame's tab", (1, 0.75), limit=4.0),
    # 24 V from the supply through the fuse block, and the 12 V and 15 V from the converters.
    Cable("W06", "24 V: the supply to the fuse block", "supply", "+V and -V terminals",
          "fuse block", "its input and 0 V terminals", (2, 1.0), 5.4, 8.8, 24.0),
    Cable("W07", "24 V: the SKR Pico, F1", "fuse block", "F1 and 0 V", "SKR Pico",
          "its power terminal", (2, 0.75), 1.3, 3.0, 24.0),
    Cable("W08", "24 V: the carrier board, F2", "fuse block", "F2 and 0 V", "carrier board",
          "its 24 V terminal", (2, 0.75), 1.3, 2.0, 24.0),
    Cable("W09", "24 V: the 12 V converter, F3", "fuse block", "F3 and 0 V", "12 V converter",
          "+Vin and -Vin", (2, 0.75), 2.2, 4.0, 24.0),
    Cable("W10", "24 V: the 15 V converter, F5", "fuse block", "F5 and 0 V", "15 V converter",
          "+Vin and -Vin", (2, 0.75), 0.8, 2.0, 24.0),
    Cable("W11", "24 V: the fan, F4", "fuse block", "F4 and 0 V", "cover's plug", "pins 1 and 2",
          (2, 0.25), 0.15, 0.5, 24.0),
    Cable("W12", "12 V: the mono camera", "12 V converter", "+Vo and -Vo", "mono camera",
          "DC plug, as its adapter's", (2, 0.75), 3.0, 5.0, 12.0),
    Cable("W13", "12 V: the USB hub", "12 V converter", "+Vo and -Vo", "USB hub",
          "DC plug, as its adapter's", (2, 0.75), 0.7, 5.0, 12.0),
    Cable("W14", "15 V: the camera port", "15 V converter", "+Vo and -Vo", "camera port",
          "DC plug, as Thorlabs' supply's", (2, 0.5), 1.0, 2.0, 15.0),
    # Motion, to the SKR Pico's headers.
    Cable("W15", "X motor and limits: the stage's own cable", "X motor", "the stage's", "SKR Pico",
          "HDB15 (M), into W18", kind="stage"),
    Cable("W16", "Y motor and limits: the stage's own cable", "Y motor", "the stage's", "SKR Pico",
          "HDB15 (M), into W19", kind="stage"),
    Cable("W17", "Z lift: motor and limits", "Z lift", "DB9 (F), onto the lift's", "SKR Pico",
          "Z motor and Z-STOP headers, and a spare input", (8, 0.34), limit=2.0),
    Cable("W18", "X adapter", "SKR Pico", "HDB15 (F)", "SKR Pico", "X motor and X-STOP headers",
          (6, 0.34), limit=2.0),
    Cable("W19", "Y adapter", "SKR Pico", "HDB15 (F)", "SKR Pico", "Y motor and Y-STOP headers",
          (6, 0.34), limit=2.0),
    Cable("W20", "Changer's motor", "SKR Pico", "E0 motor header", "changer's motor",
          "the motor's plug", (4, 0.34), limit=2.0),
    Cable("W21", "Changer's home switch", "SKR Pico", "E0-STOP header", "changer's switch",
          "soldered to its tabs", (2, 0.14)),
    Cable("W22", "SKR Pico: USB", "SKR Pico", "USB-C", "USB hub", "USB-A", kind="USB"),
    # Optics, from the carrier board.
    *[Cable("W%02d" % (23 + k), "Epi LED, %g nm" % nm, "carrier board",
            "LED driver %d's output" % (k + 1), "epi LED, %g nm" % nm, "M8 4-pin (F), onto the LED's",
            (4, 0.25), 1.0, 1.0, kind="M8") for k, nm in enumerate(P.LEDS_NM)],
    Cable("W27", "Epi LEDs' thermistors", "carrier board", "8-way header", "epi LEDs' heatsinks",
          "a pair to each LED's heatsink", (8, 0.14)),
    Cable("W28", "Slider's motor", "carrier board", "the TMC2209's motor header", "slider's motor",
          "the motor's plug", (4, 0.34), limit=2.0),
    Cable("W29", "Slider's home switch", "carrier board", "2-way header", "slider's switch",
          "soldered to its tabs", (2, 0.14)),
    Cable("W30", "LED matrix: 5 V, clock and data", "carrier board", "4-way terminal", "LED matrix",
          "the matrix's input plug", (4, 0.75), 3.8, 5.0, 5.0),
    Cable("W31", "Mono camera: trigger and exposure", "carrier board", "4-way header",
          "mono camera", "its trigger connector", (4, 0.14)),
    Cable("W32", "Colour camera: trigger and exposure", "carrier board", "4-way header",
          "colour camera", "its trigger connector", (4, 0.14)),
    Cable("W33", "Camera port: DIG I/O 1", "carrier board", "2-way header", "camera port",
          "SMA (M)", kind="SMA"),
    Cable("W34", "Camera port: DIG I/O 2", "carrier board", "2-way header", "camera port",
          "SMA (M)", kind="SMA"),
    Cable("W35", "Door interlock", "carrier board", "2-way header", "cover's plug", "pins 3 and 4",
          (2, 0.25)),
    Cable("W36", "Pico 2: USB", "carrier board", "micro-USB", "USB hub", "USB-A", kind="USB"),
    # USB, from the hub. The PC's cable leaves through the notch.
    Cable("W37", "USB to the PC", "USB hub", "its upstream port", "cover's notch",
          "USB-A, at the PC", kind="USB 3"),
    Cable("W38", "Mono camera: USB 3", "USB hub", "USB-A", "mono camera", "its USB 3 port",
          kind="USB 3"),
    Cable("W39", "Colour camera: USB 3", "USB hub", "USB-A", "colour camera", "its USB 3 port",
          kind="USB 3"),
    Cable("W40", "Overview camera: USB", "USB hub", "USB-A", "overview camera", "its USB port",
          kind="USB"),
    # On the cover, from its plug.
    Cable("W41", "Fan, on the cover", "cover's plug", "pins 1 and 2", "fan",
          "the fan's leads, spliced", (2, 0.25), 0.15, 0.5, 24.0),
    Cable("W42", "Door switch, on the cover", "cover's plug", "pins 3 and 4", "door's switch",
          "its terminals", (2, 0.25)),
]


@cache
def ducts():
    """The riser's foot, and by each slot in the head plate, the riser's point and the slot's
    front."""
    x0, y0, x1, y1, _ = P.DUCTS[0]
    x, y = (x0 + x1) / 2, (y0 + y1) / 2
    foot = V(x, y, (stand.BASE_TOP + P.DUCTS[1][4]) / 2)
    slots = {name: [V(x, y, (z0 + z1) / 2), V((sx0 + sx1) / 2, P.HEAD_PLATE[1], (z0 + z1) / 2)]
             for name, (sx0, sx1, z0, z1) in zip(("low", "high"), P.HEAD_SLOTS)}
    return foot, slots


def _length(points):
    return sum(abs(q.x - p.x) + abs(q.y - p.y) + abs(q.z - p.z) for p, q in zip(points, points[1:]))


@cache
def places():
    """Where cables end: each place's point; the slot in the head plate its cables pass, if it's on
    the head; and the shifts it moves through, if it moves."""
    base = stand.BASE_TOP
    out = {name: (V(x, y, base + P.TRAYS[tray][5] + P.TRAYS[tray][4] + 15), None, ())
           for name, (tray, x, y) in P.TRAY_PARTS.items()}
    cd, sd = head.changer_drive(), head.slider_drive()
    m, n = cd["motor"].BoundBox, sd["motor"].BoundBox
    out.update({
        "changer's motor": (V(m.XMin, m.Center.y, m.Center.z), "low", ()),
        "changer's switch": (cd["switch"].BoundBox.Center, "low", ()),
        "slider's motor": (V(n.Center.x, n.YMax, n.Center.z), "low", ()),
        "slider's switch": (sd["switch"].BoundBox.Center, "low", ()),
        "camera port": (head.port()["body"].BoundBox.Center, "high", ()),
        "overview camera": (V(*P.STAND_OVERVIEW), "high", ()),
    })
    for name in ("mono", "colour"):
        out[name + " camera"] = (head.camera(name)["plugs"].BoundBox.Center, "high", ())
    leds = [s.BoundBox.Center for s in enclosure.plugs(head.epi_arm()["leds"]).Solids]
    out.update({"epi LED, %g nm" % nm: (p, "low", ()) for nm, p in zip(P.LEDS_NM, leds)})
    slot = ducts()[1]["low"][1]
    out["epi LEDs' heatsinks"] = (max(leds, key=lambda p: _length([slot, p])), "low", ())

    # The stage's motors, and the matrix, move with the lift from the bottom of its travel to the top;
    # the X motor with the stage's Y too.
    lift = [V(0, 0, h - P.STAND_FOCUS) for h in (0.0, P.LIFT_TRAVEL)]
    x0, y0, z0, x1, y1, z1 = P.XY_X_DRIVE[1]
    out["X motor"] = (V(x1, (y0 + y1) / 2, stand.XY_Z + (z0 + z1) / 2), None,
                      [d + V(0, k * P.XY_TRAVEL, 0) for d in lift for k in (-1, 1)])
    x0, y0, z0, x1, y1, z1 = P.XY_Y_DRIVE[1]
    out["Y motor"] = (V((x0 + x1) / 2, y0, stand.XY_Z + (z0 + z1) / 2), None, lift)
    x0, y0, z0, x1, y1, z1 = P.LIFT_CONNECTOR
    out["Z lift"] = (V((x0 + x1) / 2, y1, base + (z0 + z1) / 2), None, ())
    out["LED matrix"] = (V(0, P.MATRIX_BOARD[1] / 2, stand.MATRIX_Z - P.MATRIX_BOARD[2]), None, lift)

    x, face, _, _, depth = P.INLET
    out.update({
        "cover's notch": (V(0, enclosure.OUT[3], enclosure.FOOT + P.ENCLOSURE_FRAME[0]
                             + P.ENCLOSURE_NOTCH[1] / 2), None, ()),
        "cover's plug": (V(*P.CABLE_PLUG, base + 5), None, ()),
        "fan": (enclosure.fan_at(), None, ()),
        "door's switch": (enclosure.interlock()["switch"].BoundBox.Center, None, ()),
        "mains inlet": (V(x, face - enclosure.INLET_LEG - depth, enclosure.INLET_Z), None, ()),
        "earth stud": (V(*P.INLET_EARTH, base), None, ()),
        "column": (V(*stand.column_screws()[0], base - P.STAND_BASE[4]), None, ()),
    })
    return out


def route(a, b, da=V(), db=V()):
    """The points a cable from place `a` to place `b` passes, its ends shifted by `da` and `db`:
    straight across the base, or up the riser and through a slot to the head."""
    (pa, sa, _), (pb, sb, _) = places()[a], places()[b]
    foot, slots = ducts()

    def out_to(slot):
        return [foot] if slot is None else slots[slot]

    middle = [] if sa == sb else out_to(sa)[::-1] + out_to(sb)
    return [pa + da] + middle + [pb + db]


def sizes(c):
    """A cable's route at its longest across its ends' travel; what it needs with its ends, any loop
    and, for the PC's, its run outside; and the length to cut or buy, or None if none is long
    enough."""
    pl = places()
    shifts = [(d, e) for d in pl[c.a][2] or (V(),) for e in pl[c.b][2] or (V(),)]
    run = max(_length(route(c.a, c.b, d, e)) for d, e in shifts)
    need = (run + 2 * P.CABLE_END + (P.CABLE_LOOP if len(shifts) > 1 else 0)
            + (P.CABLE_PC if c.b == "cover's notch" else 0))
    if c.kind == "made":
        return run, need, 50 * math.ceil(need / 50)
    return run, need, next((n for n in P.CABLE_STOCK[c.kind] if n >= need), None)


def write(out_dir):
    """Writes the cable list, cables.csv, into `out_dir`. Returns its checks and facts."""
    rows, hot, drops, short, usb = [], [], [], [], []
    stock, own = defaultdict(float), []
    load = (0.0, "")
    for c in CABLES:
        run, need, cut = sizes(c)
        n, size = c.wires or (None, None)
        if c.limit and size:
            share = c.limit / P.CABLE_RATINGS[size]
            load = max(load, (share, c.code))
            if share > 1:
                hot.append("%s: %g A on %g mm²" % (c.code, c.limit, size))
        drop = 2 * (cut / 1000) * c.peak * P.CABLE_COPPER / size / c.volts if c.volts else None
        if drop is not None and drop > P.CABLE_DROP:
            drops.append("%s: %.1f %%" % (c.code, 100 * drop))
        if cut is None:
            short.append("%s needs %.0f mm" % (c.code, need))
        if c.kind in P.CABLE_USB and (cut or need) > P.CABLE_USB[c.kind]:
            usb.append("%s: %.0f mm" % (c.code, cut or need))
        if c.kind == "made":
            stock["%d x %g mm²" % (n, size)] += cut / 1000
        if c.kind == "stage":
            own.append("%s %.2f m" % (c.code, need / 1000))
        kind = {"made": "Made", "stage": "The stage's own"}.get(c.kind, "Bought: " + c.kind)
        rows.append([c.code, c.what, c.a, c.a_end, c.b, c.b_end,
                     "%d x %g mm²" % c.wires if c.wires else "", kind, "%.0f" % run,
                     "%d" % cut if cut else "none long enough", "%g" % c.peak if c.peak else "",
                     "%g" % c.limit if c.limit else "", "%.1f" % (100 * drop) if drop else ""])
    with open(os.path.join(out_dir, "cables.csv"), "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["Cable", "Carries", "From", "Its end", "To", "Its end", "Conductors", "Kind",
                    "Route, mm", "Length, mm", "Peak, A", "Fuse or limit, A", "Drop, %"])
        w.writerows(rows)

    worst = max((r for r in rows if r[12]), key=lambda r: float(r[12]))
    pc = next(c for c in CABLES if c.b == "cover's notch")
    _, pc_need, pc_cut = sizes(pc)
    made = sum(c.kind == "made" for c in CABLES)
    checks = [
        Check("Each conductor's rating covers the current its fuse or source allows", not hot,
              ", ".join(hot[:4]) or "at most %.0f %% of its rating, %s" % (100 * load[0], load[1])),
        Check("The power runs' voltage drops stay within %g %% at their peak currents"
              % (100 * P.CABLE_DROP), not drops,
              ", ".join(drops[:4]) or "at most %s %%, %s" % (worst[12], worst[0])),
        Check("Every bought cable comes in a length that reaches, the stages' own across their travel",
              not short, ", ".join(short[:4]) or "the stages' own need %s, of %.1f m"
              % (" and ".join(own), P.CABLE_STOCK["stage"][0] / 1000)),
        Check("The USB cables stay within USB's lengths: %g m for USB 3, %g m for USB 2"
              % (P.CABLE_USB["USB 3"] / 1000, P.CABLE_USB["USB"] / 1000), not usb,
              ", ".join(usb[:4]) or "the PC's needs %.1f m, %.1f m of it outside the cover (assumed)"
              % (pc_need / 1000, P.CABLE_PC / 1000)),
    ]
    facts = [
        ("Cables", "%d: %d made, %d bought, %d the stages' own"
         % (len(CABLES), made, len(CABLES) - made - len(own), len(own))),
        ("Cable for the made ones, by its conductors", "; ".join(
            "%s: %.2f m" % (k, v) for k, v in sorted(stock.items()))),
        ("The longest route", "%s, %s mm" % max(((r[0], r[8]) for r in rows),
                                                key=lambda r: float(r[1]))),
        ("The PC's USB cable", "%.1f m" % (pc_cut / 1000) if pc_cut else "none long enough"),
    ]
    return checks, facts

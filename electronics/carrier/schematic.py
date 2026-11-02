"""Draws the circuit as a KiCad schematic, and writes its symbol library and project.

Each pin gets a net label at its tip, so the parts need no wires between them. Parts sit in columns
inside their block. Every UUID comes from a part's name, so the same circuit gives the same files.
"""

import json
import uuid

from circuit import BOXES, FLAGS, PARTS

VERSION = 20260306
LIB_VERSION = 20251024  # KiCad 10 reads its libraries at this version, not the schematic's
GRID = 2.54
ROW = 4 * GRID  # between two-pin parts
SHAPES = ("R", "C", "CP", "D", "Q", "FLAG")
FONT = "(effects (font (size 1.27 1.27)))"
HIDDEN = "(effects (font (size 1.27 1.27)) (hide yes))"
STROKE = "(stroke (width 0.254) (type default))"
NS = uuid.UUID("6f3c2a10-8d57-4f8e-9f70-4a1d5b1c0e04")
ROOT = str(uuid.uuid5(NS, "root"))

# Where each block starts on the A2 sheet, in mm, how tall it is, and a line saying what it does.
LAYOUT = {
    "Power": (25.4, 40.64, 90, "A diode guards against reversed 24 V. The buck feeds the matrix and the logic."),
    "Pico 2": (25.4, 137.16, 125, "The diode lets USB and the board's 5 V both feed it."),
    "Filter slider": (25.4, 276.86, 125, "UART address 0. EN's pull-up keeps it off while the Pico starts."),
    "Interlock and LED drivers": (162.56, 40.64, 165, "The door's switch alone powers U3: door open, every DIM "
                                                      "falls to its pull-down."),
    "LED matrix and camera port": (162.56, 233.68, 170, "5 V signals, through a buffer that's always on."),
    "Cameras": (368.3, 40.64, 165, "Open-collector triggers, and exposure inputs with pull-ups."),
    "LED temperatures": (368.3, 233.68, 170, "Each thermistor divides 3.3 V against 10k, into the I2C ADC at 0x48."),
}
COLUMN = 63.5


def uid(*names):
    return str(uuid.uuid5(NS, "/".join(str(n) for n in names)))


def sid(kind):
    """A symbol's name in the library."""
    return kind.replace(" ", "_")


def is_jack(kind):
    return kind[0] == "J" and kind[1:].isdigit()


def size(kind):
    """A box's width and height."""
    if is_jack(kind):
        return 5.08, GRID * (int(kind[1:]) + 1)
    left, right = BOXES[kind]
    width = max(len(p.name) for p in left) + max(len(p.name) for p in right) + 6
    return (int(width / 2 / GRID) + 1) * 2 * GRID, GRID * (max(len(left), len(right)) + 1)


def pins_of(kind):
    """Each pin as (number, x, y, angle, type, name), in the library's coordinates, y upwards. Its
    tip, where the label goes, is at x, y; angle points from the tip into the symbol."""
    if kind in ("R", "C", "CP"):
        return [(1, -5.08, 0, 0, "passive", "~"), (2, 5.08, 0, 180, "passive", "~")]
    if kind == "D":
        return [(2, -5.08, 0, 0, "passive", "~"), (1, 5.08, 0, 180, "passive", "~")]
    if kind == "Q":
        return [(1, 2.54, -5.08, 90, "passive", "E"), (2, -5.08, 0, 0, "passive", "B"),
                (3, 2.54, 5.08, 270, "passive", "C")]
    if kind == "FLAG":
        return [(1, 0, 0, 90, "power_out", "pwr")]
    w, h = size(kind)
    if is_jack(kind):
        return [(i + 1, 5.08, h / 2 - GRID * (i + 1), 180, "passive", "~") for i in range(int(kind[1:]))]
    left, right = BOXES[kind]
    return ([(p.number, -w / 2 - GRID, h / 2 - GRID * (i + 1), 0, p.type, p.name) for i, p in enumerate(left)]
            + [(p.number, w / 2 + GRID, h / 2 - GRID * (i + 1), 180, p.type, p.name) for i, p in enumerate(right)])


def line(*points):
    return "(polyline (pts %s) %s (fill (type none)))" % (" ".join("(xy %g %g)" % p for p in points), STROKE)


def body(kind):
    plates = [line((-0.635, 1.524), (-0.635, -1.524)), line((0.635, 1.524), (0.635, -1.524))]
    if kind == "R":
        return ["(rectangle (start -2.54 1.016) (end 2.54 -1.016) %s (fill (type none)))" % STROKE]
    if kind == "C":
        return plates
    if kind == "CP":
        return plates + [line((-1.778, 1.524), (-1.778, 0.508)), line((-2.286, 1.016), (-1.27, 1.016))]
    if kind == "D":
        return [line((-1.27, 1.27), (-1.27, -1.27), (1.27, 0), (-1.27, 1.27)), line((1.27, 1.27), (1.27, -1.27))]
    if kind == "Q":
        return [line((-1.27, 1.905), (-1.27, -1.905)), line((-1.27, 0.635), (2.54, 2.54)),
                line((-1.27, -0.635), (2.54, -2.54)), line((1.524, -2.286), (2.54, -2.54), (1.905, -1.524))]
    if kind == "FLAG":
        return [line((0, 0), (0, 1.27), (-1.016, 1.905), (0, 2.54), (1.016, 1.905), (0, 1.27))]
    w, h = size(kind)
    return ["(rectangle (start %g %g) (end %g %g) %s (fill (type background)))"
            % (-w / 2, h / 2, w / 2, -h / 2, STROKE)]


def in_bom(kind):
    return "(in_bom no) (on_board no)" if kind == "FLAG" else "(in_bom yes) (on_board yes)"


def symbol(kind, prefix=""):
    length = 0 if kind == "FLAG" else GRID
    pins = " ".join('(pin %s line (at %g %g %d) (length %g) (name "%s" %s) (number "%d" %s))'
                    % (t, x, y, a, length, name, FONT, n, FONT) for n, x, y, a, t, name in pins_of(kind))
    names = "(pin_names (offset 0) (hide yes))" if kind in SHAPES or is_jack(kind) else "(pin_names (offset 1.016))"
    numbers = "(pin_numbers (hide yes)) " if kind in ("R", "C", "FLAG") else ""
    props = "".join('(property "%s" "%s" (at 0 0 0) %s) ' % (k, v, HIDDEN) for k, v in (
        ("Reference", "#FLG" if kind == "FLAG" else kind), ("Value", "PWR_FLAG" if kind == "FLAG" else kind),
        ("Footprint", ""), ("Datasheet", "")))
    return ('(symbol "%s%s" %s%s%s (exclude_from_sim no) %s %s(symbol "%s_0_1" %s) (symbol "%s_1_1" %s))'
            % (prefix, sid(kind), "(power) " if kind == "FLAG" else "", numbers, names, in_bom(kind), props,
               sid(kind), " ".join(body(kind)), sid(kind), pins))


def kinds():
    return sorted({p.kind for p in PARTS} | {"FLAG"})


def label(net, x, y, facing_left, key):
    angle, justify = (180, "right") if facing_left else (0, "left")
    return ('(label "%s" (at %g %g %d) (effects (font (size 1.27 1.27)) (justify %s bottom)) (uuid "%s"))'
            % (net, x, y, angle, justify, uid("label", key)))


def placed(kind, ref, value, footprint, x, y):
    """A placed symbol, its text above and below it."""
    above, below = (5.08 if kind == "FLAG" else 2.54), 3.81
    if kind not in SHAPES:
        above = below = size(kind)[1] / 2 + 1.27
    shift = -3.81 if kind == "Q" else 0
    text = "".join('(property "%s" "%s" (at %g %g 0) %s) ' % (k, v, x + shift, py, FONT)
                   for k, v, py in (("Reference", ref, y - above), ("Value", value, y + below)))
    text += '(property "Footprint" "%s" (at %g %g 0) %s) (property "Datasheet" "" (at %g %g 0) %s) ' % (
        footprint, x, y, HIDDEN, x, y, HIDDEN)
    pins = " ".join('(pin "%d" (uuid "%s"))' % (p[0], uid("pin", ref, p[0])) for p in pins_of(kind))
    return ('(symbol (lib_id "carrier:%s") (at %g %g 0) (unit 1) (exclude_from_sim no) %s (dnp no) (uuid "%s") '
            '%s%s (instances (project "carrier" (path "/%s" (reference "%s") (unit 1)))))'
            % (sid(kind), x, y, in_bom(kind), uid("part", ref), text, pins, ROOT, ref))


def height(part):
    if part.kind in SHAPES:
        return 2 * ROW if part.kind == "Q" else ROW
    return size(part.kind)[1] + ROW


def schematic():
    items, cursors = [], {}
    for name, (x, y, _, note) in LAYOUT.items():
        for text, dy, font in ((name, 10.16, 2.54), (note, 6.35, 1.27)):
            items.append('(text "%s" (at %g %g 0) (effects (font (size %g %g)) (justify left bottom)) (uuid "%s"))'
                         % (text, x, y - dy, font, font, uid("title", text)))
        cursors[name] = (x + 22.86, y)
    for part in PARTS:
        _, y0, tall, _ = LAYOUT[part.block]
        x, y = cursors[part.block]
        need = height(part)
        if y + need > y0 + tall:
            x, y = x + COLUMN, y0
        cx, cy = round(x / GRID) * GRID, round((y + need / 2) / GRID) * GRID
        items.append(placed(part.kind, part.ref, part.value, part.footprint, cx, cy))
        for number, dx, dy, _, _, _ in pins_of(part.kind):
            net = part.pins.get(number)
            if net:
                items.append(label(net, cx + dx, cy - dy, dx < 0, (part.ref, number)))
            else:
                items.append('(no_connect (at %g %g) (uuid "%s"))' % (cx + dx, cy - dy, uid("nc", part.ref, number)))
        cursors[part.block] = (x, y + need)
    for i, net in enumerate(FLAGS):
        x, y = 25.4 + i * 22.86, 20.32
        items.append(placed("FLAG", "#FLG%d" % (i + 1), "PWR_FLAG", "", x, y))
        items.append(label(net, x, y, False, ("flag", net)))
    return ('(kicad_sch (version %d) (generator "optima") (generator_version "10.0") (uuid "%s") (paper "A2")\n'
            '(title_block (title "OPTIMA carrier board") (comment 1 "Drawn by electronics/carrier/make.py; '
            'change circuit.py, not this file"))\n(lib_symbols\n%s\n)\n%s\n'
            '(sheet_instances (path "/" (page "1")))\n(embedded_fonts no))\n'
            % (VERSION, ROOT, "\n".join(symbol(k, "carrier:") for k in kinds()), "\n".join(items)))


def library():
    return ('(kicad_symbol_lib (version %d) (generator "optima") (generator_version "10.0")\n%s\n)\n'
            % (LIB_VERSION, "\n".join(symbol(k) for k in kinds())))


def project():
    # E05 adds this board's own footprints, and then their links can be checked too.
    return json.dumps({"meta": {"filename": "carrier.kicad_pro", "version": 3},
                       "erc": {"rule_severities": {"footprint_link_issues": "ignore"}}}, indent=2) + "\n"


SYM_TABLE = ('(sym_lib_table\n  (version 7)\n  (lib (name "carrier")(type "KiCad")'
             '(uri "${KIPRJMOD}/carrier.kicad_sym")(options "")(descr "This board\'s symbols"))\n)\n')

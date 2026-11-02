"""The board's own footprints: the two modules KiCad's libraries don't have.

Written from each maker's drawing. Pad numbers match the symbols' pins in circuit.py.
"""

PIN_DRILL, PIN_PAD = 1.0, 1.8
PITCH = 2.54


def footprint(name, description, pads, body):
    """A through-hole footprint's text. Each pad is (number, x, y), and body is (x0, y0, x1, y1)."""
    x0, y0, x1, y1 = body
    box = '(fp_rect (start %g %g) (end %g %g) (stroke (width %g) (type default)) (fill no) (layer "%s"))'
    lines = ['(footprint "%s" (version 20241229) (generator "optima") (layer "F.Cu")' % name,
             '(descr "%s") (attr through_hole)' % description,
             '(property "Reference" "REF**" (at %g %g 0) (layer "F.SilkS") '
             '(effects (font (size 1 1) (thickness 0.15))))' % ((x0 + x1) / 2, y0 - 1.2),
             '(property "Value" "%s" (at %g %g 0) (layer "F.Fab") '
             '(effects (font (size 1 1) (thickness 0.15))))' % (name, (x0 + x1) / 2, y1 + 1.2),
             box % (x0, y0, x1, y1, 0.12, "F.SilkS"), box % (x0, y0, x1, y1, 0.1, "F.Fab"),
             box % (x0 - 0.25, y0 - 0.25, x1 + 0.25, y1 + 0.25, 0.05, "F.CrtYd")]
    for i, (number, x, y) in enumerate(pads):
        shape = "rect" if i == 0 else "circle"
        lines.append('(pad "%s" thru_hole %s (at %g %g) (size %g %g) (drill %g) (layers "*.Cu" "*.Mask"))'
                     % (number, shape, x, y, PIN_PAD, PIN_PAD, PIN_DRILL))
    return "\n".join(lines) + "\n)\n"


def ldd():
    """Mean Well's LDD-300 to 1000H, seen from above: two rows 15.24 apart in a 24-pin pattern. Pin
    n on the left is (n - 1) pitches down; pin 25 - n sits across from it."""
    left = [(n, 0, (n - 1) * PITCH) for n in (1, 2, 11, 12)]
    right = [(n, 15.24, (24 - n) * PITCH) for n in (13, 14, 22, 23, 24)]
    x = (15.24 - 20.3) / 2
    y = (11 * PITCH - 31.8) / 2
    return footprint("LDD-H", "Mean Well LDD-300 to 1000H, pin style", left + right, (x, y, x + 20.3, y + 31.8))


def buck():
    """Pololu's D36V50F5, from above, its pins along its lower edge: six positions on two rows. From
    the left: VOUT, GND, GND, VIN, VRP, then EN on the edge's row and PG on the inner one. VRP, the
    input after its reverse protection, isn't used, and has no number."""
    numbers = [("6", "7"), ("4", "5"), ("4", "5"), ("1", "2"), ("", ""), ("3", "8")]
    pads = [(pair[r], 6.35 + k * PITCH, 25.4 - 1.27 - r * PITCH) for k, pair in enumerate(numbers) for r in (0, 1)]
    return footprint("D36V50F5", "Pololu D36V50F5 on a 2 x 6 header", pads, (0, 0, 25.4, 25.4))


def write(folder):
    folder.mkdir(exist_ok=True)
    (folder / "LDD-H.kicad_mod").write_text(ldd(), encoding="utf-8", newline="\n")
    (folder / "D36V50F5.kicad_mod").write_text(buck(), encoding="utf-8", newline="\n")

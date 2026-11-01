"""Manufacturing outputs, from the same scripts as the models: a drawing (PDF) and a STEP file for
every custom part, and the bill of materials (CSV).

Each drawing is a TechDraw page on sheet.svg: the part in first-angle projection with an isometric
view, its overall sizes, its notes and a title block. Any size not shown comes from its STEP file,
which holds the part as modelled, moved so that its box starts at the origin. TechDraw computes its
views in the background, and draws a page's sheet only once the page is open in FreeCAD's GUI,
which make.py runs offscreen. The files are rewritten without the dates and ids that change from
run to run.
"""

import csv
import io
from collections import Counter
import os
import re
import time
import uuid

import FreeCAD as App
import Part

from . import enclosure, fasteners, head, inserts, stand
from . import params as P
from .checks import Check

V = App.Vector
# How each part is drawn, by its thinnest axis: as it sits in the instrument, seen from above, the
# front or the right. The view's direction, its horizontal, and the axes across, up and through it.
VIEWING = {2: (V(0, 0, 1), V(1, 0, 0), (0, 1, 2)), 1: (V(0, -1, 0), V(1, 0, 0), (0, 2, 1)),
           0: (V(1, 0, 0), V(0, 1, 0), (1, 2, 0))}
SHEET = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sheet.svg")
SCALES = ((2, 1), (1, 1), (1, 2), (1, 5), (1, 10))
VIEWS = (15.0, 60.0, 290.0, 282.0)  # where the projections go on the page: x0, y0, x1, y1
ALUMINIUM = "6061-T6 aluminium"
SHEET_METAL = "5052 aluminium sheet, %g mm" % P.ENCLOSURE_PANEL
FINISH = "Black anodized"
NOTES = ["Sizes in mm. Tolerances to ISO 2768-mK unless stated.",
         "Take any size not shown from the STEP file of the same number.",
         "Break sharp edges 0.2 mm.",
         "Holes for screws are clearance holes, unless a note taps them."]


def screws():
    """Every screw in the instrument."""
    return head.screws() + stand.screws() + enclosure.screws()


def made():
    """Every custom part: its number, name, shape, quantity, material and notes, with its tapped and
    counterbored holes from the screws."""
    s, cd, sd = head.structure(), head.changer_drive(), head.slider_drive()
    cams = {n: head.camera(n) for n in ("mono", "colour")}
    st = stand.stack()
    ins = {name: make()["insert"] for name, _, make in inserts.INSERTS}
    cover = enclosure.panels()
    a, m = ALUMINIUM, SHEET_METAL
    rows = [
        ("H01", "Head plate", s["plate"], 1, a, "head plate",
         "Slots for the cables, and an opening behind the mono camera's fan."),
        ("H02", "Lower deck", s["lower"], 1, a, "lower deck", ""),
        ("H03", "Upper deck", s["upper"], 1, a, "upper deck",
         "SM2 thread (2.035-40 UNS) through it, for the TTL200-A."),
        ("H04", "Camera port's bracket", s["bracket"], 1, a, "port bracket", ""),
        ("H05", "Epi plate", s["epi"], 1, a, "epi plate", ""),
        ("H06", "Changer plate", head.changer()["plate"], 1, a, "changer plate",
         "M26 x 0.706 threads for the objectives; V-grooves, 90 degrees and 3 mm deep, for the detent."),
        ("H07", "Changer's motor bracket", cd["bracket"], 1, a, "changer bracket", ""),
        ("H08", "Detent block", cd["block"], 1, a, "detent block",
         "M8 x 1 thread for the ball plunger."),
        ("H09", "Slider's carrier", head.slider()["carrier"], 1, a, "slider carrier",
         "SM1 thread (1.035-40 UNS) in the flange's bores."),
        ("H10", "Slider's motor bracket", sd["bracket"], 1, a, "slider bracket", ""),
        ("H11", "Fold mirror's mount", head.fold_mount(), 1, a, "fold mount", ""),
        ("H12", "Mono camera's mount", cams["mono"]["base"], 1, a, "mono mount", ""),
        ("H13", "Mono camera's focus plate", cams["mono"]["plate"], 1, a, "", ""),
        ("H14", "Colour camera's mount", cams["colour"]["base"], 1, a, "colour mount", ""),
        ("H15", "Colour camera's focus plate", cams["colour"]["plate"], 1, a, "", ""),
        ("H16", "Epi plate's bracket", s["epi_bracket"], 1, a, "epi bracket", ""),
        ("S01", "Base plate", stand.base()["plate"], 1, a, "base plate",
         "Tap the 6.8 mm holes M8, for the feet."),
        ("S02", "Plate on the lift", st["plate"], 1, a, "lift plate", ""),
        ("S03", "Spacer", st["spacers"].Solids[0], 4, a, "", ""),
        ("S04", "LED matrix's mount", st["mount"], 1, a, "matrix mount", ""),
        ("S05", "Overview camera's bracket", stand.overview()[1], 1, a, "overview bracket", ""),
        ("I01", "Insert adapter", st["adapter"], 1, a, "adapter",
         "Counterbored for four M3 screws into the stage."),
        ("I02", "Four-slide insert", ins["Slides"], 1, a, "", ""),
        ("I03", "Microplate insert", ins["Microplate"], 1, a, "", ""),
        ("I04", "Dish insert", ins["Dishes"], 1, a, "", ""),
        ("E01", "Cover's left panel", cover["left"], 1, m, "left panel", ""),
        ("E02", "Cover's right panel", cover["right"], 1, m, "right panel", "The intake's opening."),
        ("E03", "Cover's back panel", cover["back"], 1, m, "back panel",
         "The fan's opening, and the cables' notch."),
        ("E04", "Cover's top panel", cover["top"], 1, m, "top panel", ""),
        ("E05", "Door", enclosure.door(handle=False), 1, m, "door", ""),
        ("E06", "Intake's baffle", cover["intake baffle"], 1, m, "intake baffle", ""),
        ("E07", "Fan's baffle", cover["fan baffle"], 1, m, "fan baffle", ""),
        ("E08", "Electronics tray", enclosure.tray()["tray"], 1, a, "tray", ""),
    ]
    every = screws()
    return [(code, name, shape, n, material,
             " ".join(filter(None, (note, fasteners.holes(every, part)))))
            for code, name, shape, n, material, part, note in rows]


def bought():
    """The bought parts: name, quantity, model and note, with their lengths from the model."""
    x0, y0, x1, y1, top = P.ENCLOSURE
    f = P.ENCLOSURE_FRAME[0]
    ducts = sorted(max(b - a, d - c, t - stand.BASE_TOP) for a, c, b, d, t in P.DUCTS)
    seal = 2 * (x1 - x0 + 2 * f) + 2 * (y1 - y0 + 2 * f)
    out = [
        ("XY stage", 1, "Standa 8MTF-102LS05", ""),
        ("Z lift", 1, "Standa 8MVT100-25-1", ""),
        ("Objectives", 3, "Mitutoyo M Plan Apo 5x (378-802-6), 10x (378-803-3) and 20x (378-804-3)", ""),
        ("Tube lens", 1, "Thorlabs TTL200-A", ""),
        ("Fold mirror", 1, "2 in. protected-silver mirror in a 60 mm cage cube", ""),
        ("Camera port", 1, "Thorlabs MFF102/M flip mount and a 2 in. mirror", "Hung upside down."),
        ("Mono camera", 1, "ToupTek CTR3CMOS45000KMA", ""),
        ("Colour camera", 1, "ToupTek E3ISPM45000KPB", ""),
        ("Overview camera", 1, "12 MP USB (UVC) module, M12 wide lens", "To be chosen."),
        ("Filter set", 1, "Chroma 89402", ""),
        ("Reflected-light splitter", 1, "50:50 plate beamsplitter, 25 x 36 mm", ""),
        ("Epi LEDs", 4, "Thorlabs M395L5, M470L5, M565L3 and M625L4", ""),
        ("Epi optics", 1, "Four f = 20 mm aspheric collectors; DMLP425, DMLP505 and DMLP605; an "
         "iris; an f = 100 mm achromat; a 1 in. fold mirror; 30 mm cage cubes, plates, rods and SM1 "
         "tubes", ""),
        ("LED drivers", 4, "Mean Well LDD-H", ""),
        ("LED matrix", 1, "Adafruit 2734", ""),
        ("Guides", 2, "HIWIN MGN12 rail, %g mm, with an MGN12H carriage" % P.MGN12_RAIL[3], ""),
        ("Lead-screw steppers", 2, "NEMA 11 with a Tr6 lead screw and nut",
         "Cut the screws to %g and %g mm." % (P.CHANGER_SCREW[3], P.SLIDER_SCREW[2])),
        ("Ball plunger", 1, "M8, 4 mm ball", ""),
        ("Home switches", 2, "Miniature switch, %g x %g x %g mm" % P.SWITCH, ""),
        ("Motion controller", 1, "BTT SKR Pico", "Runs grblHAL."),
        ("Optics controller", 1, "Raspberry Pi Pico 2, on a carrier board", ""),
        ("Supply", 1, "Mean Well LRS-150-24", ""),
        ("Fan", 1, "%g mm, 24 V" % P.FAN[0], ""),
        ("Interlock switch", 1, "Safety switch, normally open",
         "In series with the LED drivers' enables."),
        ("Column", 2, "40 x 80 mm extrusion, 8 mm slots",
         "Cut to %.1f mm. Tap the bores at its foot M8." % (P.HEAD_PLATE[5] - stand.BASE_TOP)),
        ("Cover's uprights", 4, "20 x 20 mm extrusion", "Cut to %.1f mm." % (top + f - enclosure.FOOT)),
        ("Cover's rails along X", 4, "20 x 20 mm extrusion", "Cut to %g mm." % (x1 - x0)),
        ("Cover's rails along Y", 4, "20 x 20 mm extrusion", "Cut to %g mm." % (y1 - y0)),
        ("Door's hinge and handle", 1, "Piano hinge and a handle",
         "Cut the hinge to %.0f mm." % (enclosure.OUT[4] - enclosure.FOOT)),
        ("Foam strip", 1, "Closed-cell foam, %g mm high" % P.ENCLOSURE_SEAL,
         "%.0f mm, round the cover's foot." % seal),
        ("Feet", 4, "Rubber, %g mm across and %g mm high" % P.STAND_FEET[:2], ""),
        ("Cable ducts", len(ducts), "Slotted cable duct",
         "Cut to %s mm." % " and ".join("%.0f" % d for d in ducts)),
        ("Tray's standoffs", 4, "M3, %g mm" % P.TRAY[5], ""),
    ]
    # The screws, by size and length; the T-nuts and nuts they go into; the spacers they go through.
    every = screws()
    for (size, length), n in sorted(fasteners.counts(every).items()):
        out.append(("Socket-head screws", n, "M%g x %d, ISO 4762" % (size, length), ""))
    tnuts = Counter(s.size for s in every if s.into in ("column", "frame") and not s.depth)
    nuts = Counter(s.size for s in every if s.into is None and not s.depth)
    slots = {8: "the column's 8 mm slots", 4: "the frame's 5 mm slots"}
    out += [("T-nuts", n, "M%g, for %s" % (k, slots[k]), "") for k, n in sorted(tnuts.items())]
    out += [("Nuts", n, "M%g" % k, "") for k, n in sorted(nuts.items())]
    spaced = Counter(fasteners.clamped(s)[1] for s in every if s.into is None
                     and fasteners.clamped(s)[1:] and fasteners.clamped(s)[1] != "fan")
    lengths = {"intake baffle": P.BAFFLE[0], "fan baffle": P.FAN[1] + P.BAFFLE[0], "boards": 5}
    out.append(("Spacers", sum(spaced.values()), "M3 clearance", "; ".join(
        "%d x %g mm for the %s" % (n, lengths[k], k) for k, n in sorted(spaced.items()))))
    return out


def _name(code, name):
    return "OPTIMA-%s-%s" % (code, re.sub(r"[^a-z0-9]+", "-", name.lower().replace("'", "")).strip("-"))


def _sizes(shape):
    """The part's box, close round curved faces, as the drawing shows it: the front view's
    direction and horizontal, and the part's sizes across it, up it and through it."""
    b = shape.optimalBoundingBox(False)
    size = (b.XLength, b.YLength, b.ZLength)
    look, across, axes = VIEWING[size.index(min(size))]
    return look, across, [size[k] for k in axes]


def _page(doc, code, name, shape, quantity, material, note):
    """A drawing page for one part. Returns it, its front and left views, and their scale."""
    obj = doc.addObject("Part::Feature", "Part" + code)
    obj.Shape = shape
    page = doc.addObject("TechDraw::DrawPage", "Page" + code)
    page.Label = "OPTIMA-%s %s" % (code, name)
    sheet = doc.addObject("TechDraw::DrawSVGTemplate", "Sheet" + code)
    sheet.Template = SHEET
    page.Template = sheet
    look, across, (w, h, thin) = _sizes(shape)
    x0, y0, x1, y1 = VIEWS
    num, den = next((s for s in SCALES if s[0] / s[1] * (w + thin) + 45 <= x1 - x0
                     and s[0] / s[1] * (h + thin) + 45 <= y1 - y0), SCALES[-1])
    k = num / den
    # First-angle projection: the view from above below the front view, the left one to its right.
    # TechDraw's own projection group would place them, but how depends on its background work.
    up = look.cross(across)
    gap = 15 + k * thin / 2
    cx, cy = x0 + 15 + k * w / 2, y1 - 15 - k * h / 2
    views = {}
    for key, direction, xdir, dx, dy in (("Front", look, across, 0, 0),
                                         ("Top", up, across, 0, -(k * h / 2 + gap)),
                                         ("Left", -across, look, k * w / 2 + gap, 0)):
        v = views[key] = doc.addObject("TechDraw::DrawViewPart", key + code)
        page.addView(v)
        v.Source = [obj]
        v.Direction, v.XDirection = direction, xdir
        v.ScaleType = "Custom"
        v.Scale = k
        v.X, v.Y = cx + dx, cy + dy
    iso = doc.addObject("TechDraw::DrawViewPart", "Iso" + code)
    page.addView(iso)
    iso.Source = [obj]
    iso.Direction, iso.XDirection = V(1, -1, 1), V(1, 1, 0)
    iso.ScaleType = "Custom"
    iso.Scale = min(k, 95 / shape.BoundBox.DiagonalLength)
    iso.X, iso.Y = 350, 170
    _text(doc, page, "Notes" + code, NOTES + ([note] if note else []), 118, 36, 3.5)
    # The title block's cells, each centred on its place in sheet.svg.
    for i, (x, y, label, value) in enumerate((
            (320, 45, "Part", name), (275, 35, "Drawing", "OPTIMA-" + code),
            (342.5, 35, "Quantity", str(quantity)), (387.5, 35, "Scale", "%d:%d" % (num, den)),
            (275, 25, "Material", material), (365, 25, "Finish", FINISH))):
        _text(doc, page, "Cell%s%d" % (code, i), [label, value], x, y, 2.8)
    _text(doc, page, "Project" + code, ["OPTIMA microscope. Sizes in mm. First-angle projection."],
          320, 15, 2.8)
    return page, views["Front"], views["Left"], k


def _text(doc, page, name, lines, x, y, size):
    """Lines of text on `page`, centred on (x, y)."""
    a = doc.addObject("TechDraw::DrawViewAnnotation", name)
    page.addView(a)
    a.Text, a.TextSize = lines, size
    a.X, a.Y = x, y


def _export(page, path):
    """Exports `page` as a PDF, through its view in the GUI, which draws its sheet; then closes the
    view."""
    import FreeCADGui as Gui
    import TechDrawGui
    from PySide6 import QtWidgets

    page.ViewObject.doubleClicked()
    for _ in range(10):
        QtWidgets.QApplication.processEvents()
    area = Gui.getMainWindow().findChild(QtWidgets.QMdiArea)
    window = next(w for w in area.subWindowList()
                  if w.widget().findChild(QtWidgets.QGraphicsView, page.Name + "View"))
    TechDrawGui.exportPageAsPdf(page, path)
    window.close()


def _settle(doc, timeout=120):
    """Waits for TechDraw's background work: every view's lines."""
    from PySide6 import QtWidgets

    views = [o for o in doc.Objects if o.isDerivedFrom("TechDraw::DrawViewPart")]
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        QtWidgets.QApplication.processEvents()
        if all(v.getVisibleEdges() for v in views):
            return
        time.sleep(0.05)
    raise RuntimeError("TechDraw's views weren't done after %d s" % timeout)


def _blank(text, start):
    """A date `text` with its date and time replaced by `start` and its time zone's digits by
    zeros. It keeps its length, since the PDF's offsets depend on it."""
    return start + re.sub(rb"\d", b"0", text[len(start):]).replace(b"-", b"+")


def _pdf(path, name):
    data = open(path, "rb").read()
    fixed = str(uuid.uuid5(uuid.NAMESPACE_URL, "optima-cad/" + name)).encode()
    data = re.sub(rb"D:\d{14}[^)]*", lambda m: _blank(m.group(0), b"D:20260101000000"), data)
    data = re.sub(rb'(?<==")\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d[^"]*',
                  lambda m: _blank(m.group(0), b"2026-01-01T00:00:00"), data)
    data = re.sub(rb"uuid:[0-9a-f-]{36}", b"uuid:" + fixed, data)
    data = re.sub(rb"/ID \[ <[0-9a-f]+> <[0-9a-f]+> \]",
                  b"/ID [ <" + fixed.hex().encode() + b"> <" + fixed.hex().encode() + b"> ]", data)
    with open(path, "wb") as f:
        f.write(data)


def _step(shape, path, code):
    shape.exportStep(path)
    with open(path, encoding="utf-8") as f:
        text = f.read()
    text = re.sub(r"'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d'", "'2026-01-01T00:00:00'", text, count=1)
    text = re.sub(r"Open CASCADE STEP translator [\d.]+ \d+", "OPTIMA-" + code, text)
    # Trailing spaces don't matter between tokens, only inside a string that runs on.
    text = "\n".join(line if line.count("'") % 2 else line.rstrip(" ") for line in text.split("\n"))
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def write(out_dir):
    """Writes the drawings, the STEP files and the bill of materials into `out_dir`. Returns their
    checks and facts."""
    import TechDraw

    # White pages and faces, for this run only: FreeCAD's settings are a safe-mode copy.
    colours = App.ParamGet("User parameter:BaseApp/Preferences/Mod/TechDraw/Colors")
    for name in ("PageColor", "FaceColor"):
        colours.SetUnsigned(name, 0xFFFFFFFF)
    rows = made()
    for sub in ("drawings", "step"):
        os.makedirs(os.path.join(out_dir, sub), exist_ok=True)
    doc = App.newDocument("drawings")
    pages = [_page(doc, *row) for row in rows]
    doc.recompute()
    _settle(doc)
    dims = []
    for (_, front, left, k), row in zip(pages, rows):
        ds = [TechDraw.makeExtentDim(front, [], 0), TechDraw.makeExtentDim(front, [], 1),
              TechDraw.makeExtentDim(left, [], 0)]
        # Their labels outside the views: the width's and the thickness's above, the height's left.
        _, _, (w, h, _) = _sizes(row[2])
        ds[0].X, ds[0].Y = 0, k * h / 2 + 7
        ds[1].X, ds[1].Y = -k * w / 2 - 7, 0
        ds[2].X, ds[2].Y = 0, k * h / 2 + 7
        dims.append(ds)
    doc.recompute()
    _settle(doc)

    wrong, unread, facts = [], [], []
    for (page, *_), ds, (code, name, shape, quantity, material, _) in zip(pages, dims, rows):
        sizes = _sizes(shape)[2]
        drawn = [d.getRawValue() for d in ds]
        if any(abs(d - s) > 0.01 for d, s in zip(drawn, sizes)):
            wrong.append("OPTIMA-%s (%s)" % (code, ", ".join("%.2f" % d for d in drawn)))
        base = _name(code, name)
        pdf = os.path.join(out_dir, "drawings", base + ".pdf")
        _export(page, pdf)
        _pdf(pdf, base)
        b = shape.optimalBoundingBox(False)
        moved = shape.translated(V(-b.XMin, -b.YMin, -b.ZMin))
        step = os.path.join(out_dir, "step", base + ".step")
        _step(moved, step, code)
        back = Part.read(step)
        if (len(back.Solids) != len(moved.Solids)
                or abs(back.Volume - moved.Volume) > 1e-6 * moved.Volume):
            unread.append("OPTIMA-" + code)
        facts.append(("OPTIMA-%s %s" % (code, name), "%d, %s, %s mm"
                      % (quantity, material, " x ".join("%.1f" % s for s in sizes))))
    App.closeDocument(doc.Name)

    table = io.StringIO()
    w = csv.writer(table, lineterminator="\n")
    w.writerow(["Item", "Kind", "Part", "Quantity", "Model or material", "Size, mm", "Note", "Drawing"])
    for k, (code, name, shape, quantity, material, note) in enumerate(rows, start=1):
        size = " x ".join("%.1f" % s for s in _sizes(shape)[2])
        w.writerow([k, "Made", name, quantity, "%s, %s" % (material, FINISH.lower()), size, note,
                    _name(code, name)])
    for k, (name, quantity, model, note) in enumerate(bought(), start=len(rows) + 1):
        w.writerow([k, "Bought", name, quantity, model, "", note, ""])
    with open(os.path.join(out_dir, "bill-of-materials.csv"), "w", encoding="utf-8", newline="") as f:
        f.write(table.getvalue())

    files = [len(os.listdir(os.path.join(out_dir, sub))) for sub in ("drawings", "step")]
    checks = [
        Check("Every custom part has a drawing, a STEP file and a row in the bill of materials",
              files == [len(rows)] * 2, "%d parts, %d drawings, %d STEP files" % (len(rows), *files)),
        Check("Each drawing's overall sizes are its part's", not wrong,
              ", ".join(wrong) or "its width, height and thickness, from its views"),
        Check("Each STEP file reads back as its part", not unread,
              ", ".join(unread) or "the same solids and volume"),
    ]
    facts.append(("Bought parts in the bill of materials", len(bought())))
    return checks, facts

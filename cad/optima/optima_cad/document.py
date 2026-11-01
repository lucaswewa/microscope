"""FreeCAD documents: adding coloured shapes and sheets, and saving files reproducibly."""

import os
import re
import tempfile
import uuid
import zipfile

import FreeCAD as App
import Part

# Colour (RGB) and transparency (%) for each kind of object.
STYLES = {
    "sample": ((0.80, 0.90, 1.00), 40),
    "field": ((0.20, 0.45, 0.95), 0),
    "objective": ((0.62, 0.62, 0.65), 30),
    "optic": ((0.55, 0.75, 0.95), 50),
    "mirror": ((0.85, 0.85, 0.90), 0),
    "sensor": ((0.20, 0.20, 0.25), 0),
    "body": ((0.15, 0.15, 0.15), 60),
    "imaging": ((0.30, 0.80, 0.30), 70),
    "colour": ((0.95, 0.60, 0.20), 70),
    "excitation": ((0.55, 0.35, 0.95), 70),
    "transmitted": ((1.00, 0.85, 0.30), 75),
    "led": ((1.00, 1.00, 1.00), 0),
    "board": ((0.10, 0.10, 0.10), 0),
    "metal": ((0.62, 0.64, 0.68), 0),
    "drive": ((0.20, 0.20, 0.22), 0),
    "reserved": ((0.90, 0.30, 0.30), 85),
    "panel": ((0.12, 0.12, 0.14), 85),
}


def new(name):
    doc = App.newDocument(name.replace("-", "_"))
    doc.License = "MIT"
    doc.LicenseURL = "https://opensource.org/licenses/MIT"
    doc.Comment = "Built by cad/optima/make.py. Change the scripts, not this file."
    return doc


def add(doc, name, shape, style, group=None, label=None):
    obj = doc.addObject("Part::Feature", name)
    # Stored without its element map, whose names change from run to run after a fillet.
    plain = Part.Shape()
    plain.importBrepFromString(shape.exportBrepToString())
    obj.Shape = plain
    obj.Label = label or name
    colour, transparency = STYLES[style]
    obj.ViewObject.ShapeColor = colour
    obj.ViewObject.Transparency = transparency
    if group is not None:
        group.addObject(obj)
    return obj


def group(doc, name, label=None):
    g = doc.addObject("App::DocumentObjectGroup", name)
    g.Label = label or name
    return g


def sheet(doc, name, rows):
    """A spreadsheet of text and numbers, one row per tuple."""
    s = doc.addObject("Spreadsheet::Sheet", name)
    for r, row in enumerate(rows, start=1):
        for c, value in enumerate(row):
            cell = "%s%d" % ("ABCDEFGH"[c], r)
            s.set(cell, repr(value) if isinstance(value, (int, float)) else "'" + str(value))
    return s


def save(doc, path, name):
    """Saves `doc` so that building it again gives the same bytes."""
    doc.recompute()
    raw = os.path.join(tempfile.mkdtemp(), os.path.basename(path))
    doc.saveAs(raw)
    normalize(raw, path, name)


def normalize(src, dst, name):
    """Rewrites a saved document without the dates, ids and orders that change every save."""
    with zipfile.ZipFile(src) as z:
        files = {i.filename: z.read(i.filename) for i in z.infolist()}

    t = files["Document.xml"].decode("utf-8")
    t = re.sub(r'ProgramVersion="(\d+\.\d+)[^"]*"', r'ProgramVersion="\1"', t)
    t = re.sub(
        r'(name="(?:CreationDate|LastModifiedDate)"[^>]*>\s*<String value=")[^"]*',
        r"\g<1>2026-01-01T00:00:00Z",
        t,
    )
    uid = str(uuid.uuid5(uuid.NAMESPACE_URL, "optima-cad/" + name))
    t = re.sub(r'(name="Uid"[^>]*>\s*<Uuid value=")[^"]*', lambda m: m.group(1) + uid, t)
    ids = iter(range(1, 1 << 20))
    t = re.sub(r'(<Object type="[^"]+" name="[^"]+" id=")\d+', lambda m: m.group(1) + str(next(ids)), t)
    files["Document.xml"] = t.encode("utf-8")

    # The view providers come out in no fixed order: sort them, and renumber the files they
    # name, such as ShapeAppearance1, in the new order.
    t = files["GuiDocument.xml"].decode("utf-8")
    blocks = re.findall(r'[ \t]*<ViewProvider name="[^"]*".*?</ViewProvider>\n', t, re.S)
    if blocks:
        start, end = t.index(blocks[0]), t.index(blocks[-1]) + len(blocks[-1])
        blocks.sort(key=lambda b: re.search(r'name="([^"]*)"', b).group(1))
        t = t[:start] + "".join(blocks) + t[end:]
    renames, counts = {}, {}

    def rename(m):
        if m.group(1) not in renames:
            base = re.sub(r"\d+$", "", m.group(1))
            counts[base] = counts.get(base, -1) + 1
            renames[m.group(1)] = base + (str(counts[base]) if counts[base] else "")
        return 'file="%s"' % renames[m.group(1)]

    t = re.sub(r'file="([^"]+)"', rename, t)
    moved = {renames[k]: files.pop(k) for k in list(files) if k in renames}
    files.update(moved)
    files["GuiDocument.xml"] = t.encode("utf-8")

    # FreeCAD reads the files in the order the XML names them.
    order = []
    for xml in ("Document.xml", "GuiDocument.xml"):
        order.append(xml)
        for n in re.findall(r'file="([^"]+)"', files[xml].decode("utf-8")):
            if n not in order:
                order.append(n)
    assert set(order) == set(files), "unreferenced files: %s" % sorted(set(files) - set(order))
    with zipfile.ZipFile(dst, "w") as z:
        for n in order:
            info = zipfile.ZipInfo(n, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, files[n], compresslevel=6)

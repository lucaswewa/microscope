"""The cameras, from ToupTek's drawings, and an assumed overview camera.

Each is built in its own frame: the origin at the sensor's centre, the light arriving along +Z
(the lens is on the -Z side), the sensor's long side along X, and the mounting face at -Y.
"""

import math

import FreeCAD as App
import Part

from . import geom as G
from . import optics as O
from . import params as P
from .checks import Check
from .document import add, group

V = App.Vector
Y, Z = V(0, 1, 0), V(0, 0, 1)
SEAT = -P.C_MOUNT_FLANGE  # where a C-mount lens seats
STYLES = {"sensor": "sensor", "body": "body", "board": "board", "holder": "body", "lens": "optic",
          "plugs": "reserved"}


def _sensor(size, thickness=0.5):
    return G.box((size[0], size[1], thickness), V(0, 0, 0))


def _side_holes(body, face_y, holes):
    """Holes into the -Y face: (diameter, depth, x, z)."""
    tools = [Part.makeCylinder(d / 2, depth + 1, V(x, face_y - 1, z), Y) for d, depth, x, z in holes]
    return body.cut(Part.makeCompound(tools))


def _bored(body):
    """The body with the C-mount's bore, from the seat to the sensor."""
    return body.cut(Part.makeCylinder(P.C_MOUNT_BORE / 2, P.C_MOUNT_FLANGE + 1, V(0, 0, SEAT - 1)))


def mono():
    """The mono camera: its sensor, its body, and the room its plugs need."""
    w, h, depth = P.MONO_BODY
    nose_d, nose_l = P.MONO_NOSE
    front = SEAT + nose_l
    back = front + depth
    body = G.box(P.MONO_BODY, V(0, 0, front + depth / 2)).fuse([
        G.disc(nose_d, nose_l, V(0, 0, SEAT + nose_l / 2), Z),
        G.box(P.MONO_FAN, V(0, 0, back + P.MONO_FAN[2] / 2)),
    ])
    line, pitch = P.MONO_MOUNT
    # M4, 1/4-20, M4, each 6 mm deep (assumed).
    body = _side_holes(body, -h / 2, [(4.0, 6, -pitch / 2, front + line), (6.35, 6, 0, front + line),
                                      (4.0, 6, pitch / 2, front + line)])
    # The trigger, USB and power sockets sit on the opposite face, by the back (assumed: 16 mm of
    # it, with 35 mm for the plugs and their cables).
    plugs = G.span(V(-w / 2, h / 2, back - 16), V(w / 2, h / 2 + 35, back))
    return {"sensor": _sensor(P.SENSOR_SIZE), "body": _bored(body.removeSplitter()), "plugs": plugs}


def colour():
    """The colour camera: its sensor, its body, and the room its USB plug needs."""
    w, h, depth = P.COLOUR_BODY
    ring_d, ring_l = P.COLOUR_NOSE
    front = SEAT + ring_l
    back = front + depth
    body = G.rounded(G.box(P.COLOUR_BODY, V(0, 0, front + depth / 2)), P.COLOUR_EDGE)
    body = body.fuse(G.disc(ring_d, ring_l, V(0, 0, SEAT + ring_l / 2), Z))
    body = _side_holes(body, -h / 2, [(6.35, 6, 0, front + depth / 2)])  # 1/4-20, place assumed
    plugs = G.span(V(-8, -7, back), V(8, 7, back + 40))  # USB-B on the back, assumed
    return {"sensor": _sensor(P.SENSOR_SIZE), "body": _bored(body.removeSplitter()), "plugs": plugs}


def overview():
    """The overview camera: its sensor, board, lens holder, lens, and the room its plug needs."""
    bw, bh, bt = P.OVERVIEW_BOARD
    hd = P.OVERVIEW_HOLDER[2]
    lens_d, lens_l = P.OVERVIEW_LENS
    s = P.OVERVIEW_HOLES / 2
    board = G.drill(G.box(P.OVERVIEW_BOARD, V(0, 0, 0.5 + bt / 2)),
                    [(2.2, None, [(x, y) for x in (-s, s) for y in (-s, s)])], 0.5, Z)
    holder = G.box(P.OVERVIEW_HOLDER, V(0, 0, 0.5 - hd / 2))
    holder = holder.cut(Part.makeCylinder(lens_d / 2, hd + 2, V(0, 0, -hd)))
    lens = G.disc(lens_d, lens_l, V(0, 0, -4 - lens_l / 2), Z)  # its back 4 mm from the sensor
    plugs = G.span(V(-6, -bh / 2, 0.5 + bt), V(6, -bh / 2 + 8, 25.5 + bt))
    return {"sensor": _sensor(P.OVERVIEW_SENSOR, 1.0), "board": board, "holder": holder, "lens": lens,
            "plugs": plugs}


def _add(doc, label, parts):
    g = group(doc, "Camera", label)
    for name, shape in parts.items():
        add(doc, name.capitalize(), shape, STYLES[name], g,
            "Room for the plugs" if name == "plugs" else None)


def _bore_check():
    r = O.corner_light(P.C_MOUNT_FLANGE)
    return Check("The image corners' light clears the C-mount's bore", r <= P.C_MOUNT_BORE / 2,
                 "%.2f mm from the axis at the lens's seat, of %.2f mm" % (r, P.C_MOUNT_BORE / 2))


def _envelope(parts):
    b = parts["body"].BoundBox
    return [
        ("Envelope without the plugs, x y z", "%.1f x %.1f x %.1f" % (b.XLength, b.YLength, b.ZLength)),
        ("Lens seat to sensor", P.C_MOUNT_FLANGE),
        ("Lens seat to the camera's back", round(b.ZMax - SEAT, 2)),
    ]


def build_mono(doc):
    parts = mono()
    _add(doc, "Mono camera: ToupTek CTR3CMOS45000KMA", parts)
    line, pitch = P.MONO_MOUNT
    facts = _envelope(parts) + [
        ("Mounting holes",
         "2 x M4 %g mm apart, a 1/4-20 between, %g mm behind the front" % (pitch, line)),
        ("Mass, kg", P.MONO_MASS),
    ]
    return [_bore_check()], facts


def build_colour(doc):
    parts = colour()
    _add(doc, "Colour camera: ToupTek E3ISPM45000KPB", parts)
    return [_bore_check()], _envelope(parts) + [("Mounting hole", "1/4-20, centred (assumed)")]


def build_overview(doc):
    parts = overview()
    _add(doc, "Overview camera (assumed)", parts)
    sw, sh = P.OVERVIEW_SENSOR
    diagonal = math.hypot(sw, sh)
    fov = [2 * math.degrees(math.atan(s / 2 / P.OVERVIEW_LENS_F)) for s in (sw, sh, diagonal)]
    checks = [Check("The lens's image circle covers the sensor", P.OVERVIEW_IMAGE_CIRCLE >= diagonal,
                    "%.2f mm circle, %.2f mm diagonal" % (P.OVERVIEW_IMAGE_CIRCLE, diagonal))]
    facts = [
        ("Field of view, degrees: horizontal, vertical, diagonal", "%.1f %.1f %.1f" % tuple(fov)),
        ("Lens front, z", round(parts["lens"].BoundBox.ZMin, 2)),
    ]
    return checks, facts

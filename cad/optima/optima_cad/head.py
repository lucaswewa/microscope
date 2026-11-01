"""The optical head: the custom parts that hold the optics where the layout puts them, with the
bought parts in place, checked against each other and against the layout's light.

Everything is in the instrument's frame. Moving parts take their positions: the changer's offset
along X (+40 puts the 5x on the axis), the slider's along Y (+40 puts the quad cube on the axis),
the camera port's swing in degrees (90 is position 2), and each camera's focus along its light.
"""

import math
from functools import cache

import FreeCAD as App
import Part

from . import cameras, fasteners, layout, parts
from . import geom as G
from . import optics as O
from . import params as P
from .checks import Check, clashes
from .document import add, group
from .fasteners import Screw

V = App.Vector
X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)

SPOTS = [(k - 1) * P.CHANGER_PITCH for k in range(len(P.CHANGER_ORDER))]  # objectives' x on the plate
CELLS = [(k - 1) * P.FILTER_PITCH for k in range(3)]  # cells (a), (b), (c): their y on the slider
CHANGES = [0.0 - x for x in SPOTS]  # the changer's offsets that put each objective on the axis
SLIDES = [0.0 - y for y in CELLS]
FRONT = P.HEAD_PLATE[1]  # the head plate's front face, where the decks end
RAIL_H = P.MGN12H[2]
LOWER_TOP = O.SHOULDER - RAIL_H  # the changer's rail sits on the lower deck
SLIDER_Z = (O.FILTER - P.FILTER_CUBE / 2, O.FILTER + P.FILTER_CUBE / 2)
UPPER_Z = SLIDER_Z[1] + P.SLIDER_FLANGE[1] + RAIL_H  # the slider's rail hangs from the upper deck
DECK_TOP = UPPER_Z + P.DECK_THICKNESS
PORT_FACE = V(1, -1, 0).normalize()  # the way the camera port's mirror faces
NUT = 6.0  # half the length of the slider's nut block along its screw
MOUNT_HOLES = {  # each camera's holes into its mounting face: (diameter, x, from its front)
    "mono": [(4.5, x, P.MONO_MOUNT[0]) for x in (-P.MONO_MOUNT[1] / 2, P.MONO_MOUNT[1] / 2)]
    + [(6.6, 0, P.MONO_MOUNT[0])],
    "colour": [(6.6, 0, P.COLOUR_BODY[2] / 2)],
}
EPI_BOTTOM = O.FILTER - P.CAGE_30[2] / 2 - P.EPI_PLATE[4]  # the epi plate's underside
PORT_BASE = layout.FLIP_POINT.z + P.FLIP_OPTIC  # the camera port's base, on its bracket's underside


@cache
def screws():
    """The head's screws, from the head plate's to the bought parts': each a fasteners.Screw."""
    back, t = P.HEAD_PLATE[4], P.DECK_THICKNESS
    up, down, front, aft = (0, 0, 1), (0, 0, -1), (0, 1, 0), (0, -1, 0)
    out = [Screw(8, (sx * P.STAND_COLUMN[3], FRONT, z), front, 15, 0, "head plate", "column")
           for sx in (-1, 1) for z in P.HEAD_SCREWS["column"]]
    for name, z in (("lower deck", LOWER_TOP - t / 2), ("upper deck", UPPER_Z + t / 2),
                    ("port bracket", PORT_BASE + P.PORT_BRACKET[3] / 2)):
        out += [Screw(6, (x, back, z), aft, 15, 12, "head plate", name) for x in P.HEAD_SCREWS[name]]
    x0, x1, yf, z0, bt = P.EPI_BRACKET
    for x in (x0 + 8, x1 - 8):
        out += [Screw(5, (x, FRONT - bt, z0 + 6), front, bt, 10, "epi bracket", "head plate"),
                Screw(4, (x, (yf + FRONT - bt) / 2, EPI_BOTTOM - bt), up, bt, 8, "epi bracket",
                      "epi plate")]
    cage = [layout.TURN] + [at for _, at, *_ in layout.epi_leds() if at is not None]
    cage += [V(P.EPI_TUBE[2], 0, 0), V(O.STOP_X, 0, 0)]
    out += [Screw(4, (c.x, c.y, EPI_BOTTOM), up, P.EPI_PLATE[4], 6, "epi plate", "epi cage")
            for c in cage]
    # The guides: their rails' counterbored M3s, and the carriages' four.
    rh, pitch, n = P.MGN12_RAIL[1], P.MGN12_RAIL[2], int(P.MGN12_RAIL[3] // P.MGN12_RAIL[2])
    along = [(k - (n - 1) / 2) * pitch for k in range(n)]
    sx, s = P.SLIDER_RAIL, P.MGN12H[4] / 2
    out += [Screw(3, (x, P.CHANGER_RAIL, LOWER_TOP + rh - 4.5), down, 3.5, 6, "changer rail",
                  "lower deck") for x in along]
    out += [Screw(3, (sx, y, UPPER_Z - rh + 4.5), up, 3.5, 6, "slider rail", "upper deck")
            for y in along]
    grip = O.CARRIAGE_TOP - 3.5 - O.SHOULDER
    out += [Screw(3, (dx, P.CHANGER_RAIL + dy, O.CARRIAGE_TOP - 3.5), down, grip, 4, "changer plate",
                  "changer carriage", sunk=True) for dx in (-s, s) for dy in (-s, s)]
    out += [Screw(3, (sx + dx, dy, SLIDER_Z[1]), up, P.SLIDER_FLANGE[1], 4, "slider carrier",
                  "slider carriage") for dx in (-s, s) for dy in (-s, s)]
    # The motors' brackets, the motors, and the detent's block.
    face, sy, sz, _ = P.CHANGER_SCREW
    m = P.NEMA11[2] / 2
    out += [Screw(3, (face + 3, sy + dy, LOWER_TOP - t), up, t, 8, "lower deck", "changer bracket")
            for dy in (-10, 10)]
    out += [Screw(2.5, (face + 6, sy + dy, sz + dz), (-1, 0, 0), 6, 3.5, "changer bracket",
                  "changer motor") for dy in (-m, m) for dz in (-m, m)]
    out += [Screw(3, (P.DETENT[0] + dx, P.CHANGER_PLATE[3] + 11.5, LOWER_TOP - t), up, t, 8,
                  "lower deck", "detent block") for dx in (-5, 5)]
    face, sz, _ = P.SLIDER_SCREW
    out += [Screw(3, (sx + dx, face - 3, DECK_TOP - 3.5), down, t - 3.5, 8, "upper deck",
                  "slider bracket", sunk=True) for dx in (-10, 10)]
    out += [Screw(2.5, (sx + dx, face - 6, sz + dz), front, 6, 3.5, "slider bracket", "slider motor")
            for dx in (-m, m) for dz in (-m, m)]
    # What stands on the upper deck, screwed from below, or down into it.
    out += [Screw(4, (dx, P.FOLD_MOUNT[1] + 6, UPPER_Z), up, t, 10, "upper deck", "fold mount")
            for dx in (-12, 12)]
    for name, body, nose, depth in (("mono", P.MONO_BODY, P.MONO_NOSE[1], 4.5),
                                    ("colour", P.COLOUR_BODY, P.COLOUR_NOSE[1], 6)):
        origin, axis = layout.camera_frames()[name]
        mid = origin + axis * (nose - P.C_MOUNT_FLANGE + (body[2] + 9) / 2)  # the base's middle
        across = Z.cross(axis) * (body[0] / 2 - 12)
        out += [Screw(3, (c.x, c.y, UPPER_Z), up, t, depth, "upper deck", name + " mount")
                for c in (mid - across, mid + across)]
    # The camera port's base: an M6 hole at its middle (assumed).
    c = layout.FLIP_POINT - PORT_FACE * (P.MIRROR_THICKNESS / 2 + P.FLIP_RING[1] / 2
                                         + P.FLIP_BODY[2] / 2)
    out.append(Screw(6, (c.x, c.y, PORT_BASE + P.PORT_BRACKET[3]), down, P.PORT_BRACKET[3], 8,
                     "port bracket", "camera port"))
    _, _, _, y1, ft = P.STAND_OVERVIEW_ARM
    out += [Screw(4, (dx, (P.UPPER_DECK[1] + y1) / 2, DECK_TOP + ft), down, ft, 8, "overview bracket",
                  "upper deck") for dx in (-12, 12)]
    return tuple(out)


@G.kept
def structure():
    """The head plate, and the decks, the camera port's bracket and the epi plate it carries."""
    t = P.DECK_THICKNESS
    x0, y0, x1 = P.LOWER_DECK
    lower = G.span(V(x0, y0, LOWER_TOP - t), V(x1, FRONT, LOWER_TOP))
    x0, y0, x1 = P.UPPER_DECK
    upper = G.span(V(x0, y0, UPPER_Z), V(x1, FRONT, DECK_TOP))
    # The tube lens screws into an SM2 thread through the deck.
    upper = upper.cut(Part.makeCylinder(P.TUBE_HOUSING[0] / 2, t + 2, V(0, 0, UPPER_Z - 1)))
    x0, y0, x1, bt = P.PORT_BRACKET
    base = layout.FLIP_POINT.z + P.FLIP_OPTIC
    bracket = G.span(V(x0, y0, base), V(x1, FRONT, base + bt))
    # The epi plate reaches the head plate's front face, and passes its right edge.
    x0, y0, x1, y1, pt = P.EPI_PLATE
    top = O.FILTER - P.CAGE_30[2] / 2
    epi = G.span(V(x0, y0, top - pt), V(x1, y1, top))
    epi = epi.cut(G.span(V(x0 - 1, FRONT, top - pt - 1), V(P.HEAD_PLATE[3], y1 + 1, top + 1)))
    # An angle under the epi plate, screwed to the head plate's front.
    x0, x1, yf, z0, bt = P.EPI_BRACKET
    angle = G.span(V(x0, FRONT - bt, z0), V(x1, FRONT, EPI_BOTTOM)).fuse(
        G.span(V(x0, yf, EPI_BOTTOM - bt), V(x1, FRONT, EPI_BOTTOM))).removeSplitter()
    # The head plate's openings: slots for the cables from the duct behind it, and an opening behind
    # the mono camera's fan, toward the enclosure's.
    y0, y1 = FRONT - 1, P.HEAD_PLATE[4] + 1
    holes = [G.span(V(x0, y0, z0), V(x1, y1, z1)) for x0, x1, z0, z1 in P.HEAD_SLOTS]
    fan = layout.camera_frames()["mono"][0]
    side = P.MONO_FAN[0] + 5
    holes.append(G.box((side, y1 - y0, side), V(fan.x, (y0 + y1) / 2, fan.z)))
    plate = G.corners(P.HEAD_PLATE).cut(Part.makeCompound(holes))
    s = screws()
    return {k: fasteners.cut(v, s, name) for k, v, name in (
        ("plate", plate, "head plate"), ("lower", lower, "lower deck"), ("upper", upper, "upper deck"),
        ("bracket", bracket, "port bracket"), ("epi", epi, "epi plate"),
        ("epi_bracket", angle, "epi bracket"))}


@G.kept
def changer(offset=0.0):
    """The changer's moving parts: its plate, with the nut block and the detent's grooves, the
    objectives, and the guide's carriage."""
    x0, y0, x1, y1 = P.CHANGER_PLATE
    z0, z1 = O.SHOULDER, O.CARRIAGE_TOP
    nx0, nx1, ny1 = P.CHANGER_NUT
    _, sy, sz, _ = P.CHANGER_SCREW
    plate = G.span(V(x0, y0, z0), V(x1, y1, z1)).fuse(G.span(V(nx0, y1, z0), V(nx1, ny1, z1)))
    tools = [Part.makeCylinder(5, nx1 - nx0 + 2, V(nx0 - 1, sy, sz), X)]
    tools += [Part.makeCylinder(P.OBJECTIVE_THREAD[0] / 2, z1 - z0 + 2, V(x, 0, z0 - 1)) for x in SPOTS]
    d = P.DETENT[2]
    for x in SPOTS:
        gx = P.DETENT[0] + x
        v = [V(gx - d - 1, y1 + 1, z0 - 1), V(gx + d + 1, y1 + 1, z0 - 1), V(gx, y1 - d, z0 - 1)]
        tools.append(Part.Face(Part.makePolygon(v + v[:1])).extrude(V(0, 0, z1 - z0 + 2)))
    plate = fasteners.cut(plate.cut(Part.makeCompound(tools)).removeSplitter(), screws(),
                          "changer plate")
    objectives = [parts.objective(n).translated(V(x, 0, 0)) for n, x in zip(P.CHANGER_ORDER, SPOTS)]
    carriage = parts.guide()["carriage"].translated(V(0, P.CHANGER_RAIL, LOWER_TOP))
    shapes = {"plate": plate, "objectives": Part.makeCompound(objectives), "carriage": carriage}
    return {k: s.translated(V(offset, 0, 0)) for k, s in shapes.items()}


def detent_ball(shrink=0.0):
    """The plunger's ball, seated in a groove: its centre r·√2 from the V's bottom."""
    _, ball, depth = P.DETENT
    y = P.CHANGER_PLATE[3] - depth + ball / 2 * math.sqrt(2)
    return Part.makeSphere(ball / 2 - shrink, V(P.DETENT[0], y, (O.SHOULDER + O.CARRIAGE_TOP) / 2))


@G.kept
def changer_drive():
    """The changer's fixed parts: its rail, the motor and its bracket, the lead screw, the
    detent's block and plunger, and the home switch, which the guide's carriage trips."""
    face, sy, sz, length = P.CHANGER_SCREW
    motor = {k: G.placed(v, V(face, sy, sz), X, Z) for k, v in parts.stepper(length).items()}
    bracket = G.span(V(face, sy - 16, LOWER_TOP), V(face + 6, sy + 16, sz + 16))
    bracket = bracket.cut(Part.makeCylinder(P.NEMA11[3] / 2 + 0.5, 8, V(face - 1, sy, sz), X))
    dx, y1 = P.DETENT[0], P.CHANGER_PLATE[3]
    mid = (O.SHOULDER + O.CARRIAGE_TOP) / 2
    plunger = Part.makeCylinder(4, 16, V(dx, y1 + 1, mid), Y)  # an M8 ball plunger
    block = G.span(V(dx - 10, y1 + 1, LOWER_TOP), V(dx + 10, y1 + 22, O.CARRIAGE_TOP)).cut(plunger)
    s = screws()
    bracket = fasteners.cut(bracket, s, "changer bracket")
    block = fasteners.cut(block, s, "detent block")
    sw, st, sh = P.SWITCH
    end = min(CHANGES) - P.MGN12H[1] / 2 - 1
    side = P.CHANGER_RAIL + P.MGN12H[0] / 2 + 1
    return {"rail": parts.guide()["rail"].translated(V(0, P.CHANGER_RAIL, LOWER_TOP)),
            "motor": motor["motor"], "screw": motor["screw"], "bracket": bracket, "block": block,
            "plunger": plunger,
            "switch": G.span(V(end - sw, side, LOWER_TOP), V(end, side + st, LOWER_TOP + sh))}


@G.kept
def slider(offset=0.0):
    """The slider's moving parts: the carrier, its filters and the guide's carriage. Each cell is
    open below and to the right, where the excitation enters."""
    w, length = P.SLIDER_BLOCK
    z0, z1 = SLIDER_Z
    c = P.FILTER_CUBE / 2
    fx0, ft = P.SLIDER_FLANGE
    sx, sz = P.SLIDER_RAIL, P.SLIDER_SCREW[1]
    d, t = P.FILTER_ROUND
    block = G.span(V(-w / 2, -length / 2, z0), V(w / 2, length / 2, z1)).fuse([
        G.span(V(fx0, -length / 2, z1), V(w / 2, length / 2, z1 + ft)),
        G.span(V(sx - 8, -NUT, sz - 10), V(sx + 8, NUT, z1)),  # the nut block, under the flange
    ])
    tools = [Part.makeCylinder(5, 2 * NUT + 4, V(sx, -NUT - 2, sz), Y)]
    for y in CELLS:
        tools += [G.span(V(-c, y - c, z0 - 1), V(c, y + c, z1)),
                  Part.makeCylinder(d / 2, w, V(0, y, O.FILTER), X),
                  Part.makeCylinder(P.SM1 / 2, ft + 2, V(0, y, z1 - 1))]
    carrier = fasteners.cut(block.cut(Part.makeCompound(tools)).removeSplitter(), screws(),
                            "slider carrier")
    a, b = CELLS[0], CELLS[2]
    # (a): the exciter in the side bore, the dichroic, the emitter in the flange; (c): the 50:50.
    filters = [G.disc(d, t, V(c + t / 2, a, O.FILTER), X), G.disc(d, t, V(0, a, z1 + t / 2), Z),
               G.plate(P.DICHROIC, V(0, a, O.FILTER), -X, -Z),
               G.plate(P.DICHROIC, V(0, b, O.FILTER), -X, -Z)]
    carriage = G.placed(parts.guide()["carriage"], V(sx, 0, UPPER_Z), -Z, X)
    shapes = {"carrier": carrier, "filters": Part.makeCompound(filters), "carriage": carriage}
    return {k: s.translated(V(0, offset, 0)) for k, s in shapes.items()}


@G.kept
def slider_drive():
    """The slider's fixed parts, under the upper deck: its inverted rail, the motor and its
    bracket, the lead screw, and the home switch."""
    face, sz, length = P.SLIDER_SCREW
    sx = P.SLIDER_RAIL
    motor = {k: G.placed(v, V(sx, face, sz), -Y, Z) for k, v in parts.stepper(length).items()}
    bracket = G.span(V(sx - 16, face - 6, sz - 16), V(sx + 16, face, UPPER_Z))
    bracket = bracket.cut(Part.makeCylinder(P.NEMA11[3] / 2 + 0.5, 8, V(sx, face + 1, sz), -Y))
    bracket = fasteners.cut(bracket, screws(), "slider bracket")
    sw, st, sh = P.SWITCH
    end = min(SLIDES) - P.MGN12H[1] / 2 - 1
    side = sx + P.MGN12H[0] / 2 + 1
    return {"rail": G.placed(parts.guide()["rail"], V(sx, 0, UPPER_Z), -Z, X),
            "motor": motor["motor"], "screw": motor["screw"], "bracket": bracket,
            "switch": G.span(V(side, end - sw, UPPER_Z - sh), V(side + st, end, UPPER_Z))}


@G.kept
def fold_mount():
    """A block on the upper deck, behind the fold mirror: its 45-degree face holds the mirror."""
    w, front, top = P.FOLD_MOUNT
    n = V(0, -1, 1).normalize()  # out of the mirror's back
    back = layout.FOLD_POINT + n * P.MIRROR_THICKNESS
    block = G.split(G.span(V(-w / 2, front, DECK_TOP), V(w / 2, 25, top)), back, n, back + n)[0]
    leg_y = front + 12
    leg = G.span(V(-w / 2, front, DECK_TOP), V(w / 2, leg_y, back.z + leg_y - back.y + 1))
    return fasteners.cut(block.fuse(leg).removeSplitter(), screws(), "fold mount")


@G.kept
def port(angle=0.0):
    """The camera port, an MFF102/M, hung upside down from its bracket so that its body clears the
    slider, its mirror's face on the layout's flip point. `angle` swings it toward position 2."""
    m = parts.flip_mount(1)
    pivot = V(0, 0, P.FLIP_PIVOT - P.FLIP_OPTIC)
    for k in ("swing", "mirror"):
        m[k] = m[k].copy()
        m[k].rotate(pivot, Y, -angle)
    origin = layout.FLIP_POINT - PORT_FACE * (P.MIRROR_THICKNESS / 2)
    out = {k: G.placed(v, origin, -Z, -PORT_FACE) for k, v in m.items()}
    out["body"] = fasteners.cut(out["body"], screws(), "camera port")
    return out


def camera_mount(body, front, holes, drop):
    """A camera's mount, in the camera's frame: a base on the upper deck, `drop` below the sensor,
    with a key along the light; a focus plate under the camera's mounting face, which slides on
    the key; and, behind it, a block for the focus screw."""
    w, h, depth = body
    back, top, f = front + depth, -h / 2 - 5, P.CAMERA_FOCUS
    base = G.span(V(5 - w / 2, -drop, front - 3), V(w / 2 - 5, top, back + 12)).fuse([
        G.span(V(-4, top, front - 3), V(4, top + 2, back - 4 + f)),
        G.span(V(-10, top, back - 4 + f), V(10, -h / 2 - 0.5, back + 12)),
    ])
    plate = G.span(V(5 - w / 2, top, front), V(w / 2 - 5, -h / 2, back - 5))
    tools = [G.span(V(-4.2, top - 1, front - 1), V(4.2, top + 2.2, back))]
    tools += [Part.makeCylinder(hd / 2, 7, V(x, top - 1, front + z), Y) for hd, x, z in holes]
    return {"base": base.removeSplitter(), "plate": plate.cut(Part.makeCompound(tools))}


@G.kept
def camera(name, focus=0.0):
    """A camera on its mount at its sensor's place, the camera and its focus plate moved `focus`
    along its light: its sensor, body, plugs, plate and base."""
    build, body, nose = {"mono": (cameras.mono, P.MONO_BODY, P.MONO_NOSE[1]),
                         "colour": (cameras.colour, P.COLOUR_BODY, P.COLOUR_NOSE[1])}[name]
    origin, axis = layout.camera_frames()[name]
    front = nose - P.C_MOUNT_FLANGE
    mount = camera_mount(body, front, MOUNT_HOLES[name], origin.z - DECK_TOP)
    out = {k: G.placed(v, origin + axis * focus, axis, Z) for k, v in build().items()}
    out["plate"] = G.placed(mount["plate"], origin + axis * focus, axis, Z)
    out["base"] = fasteners.cut(G.placed(mount["base"], origin, axis, Z), screws(), name + " mount")
    return out


@G.kept
def epi_arm():
    """The epi arm's bought parts, on the epi plate: the epi lens's tube, the 30 mm cage with the
    field stop's plate, the fold's cube and the combiners' cubes, the collectors' tubes and the
    mounted LEDs."""
    z, half = O.FILTER, P.CAGE_30[2] / 2
    d, start, held = P.EPI_TUBE
    near = held - P.CAGE_PLATE_30 / 2
    s = P.CAGE_30[0] / 2
    rod = P.CAGE_30[1] / 2
    turn = layout.TURN
    centres = [turn] + [at for _, at, *_ in layout.epi_leds() if at is not None]
    rods = [Part.makeCylinder(rod, turn.x - half - near, V(near, dy, z + dz), X)
            for dy in (-s, s) for dz in (-s, s)]
    for a, b in zip(centres, centres[1:]):
        rods += [Part.makeCylinder(rod, b.y - a.y - 2 * half, V(a.x + dx, a.y + half, z + dz), Y)
                 for dx in (-s, s) for dz in (-s, s)]
    cube = parts.dichroic_cube()
    tubes, leds = [], []
    for _, at, _, die, axis in layout.epi_leds():
        a = (at if at is not None else centres[-1]) + axis * half
        e = die - axis * 6  # the LED's housing starts 6 mm in front of its die
        tubes.append(G.ring(d, P.SM1, (e - a).Length, (a + e) * 0.5, axis))
        leds.append(G.placed(parts.mounted_led()["housing"], die, -axis, Z))
    return {
        "tube": G.ring(d, P.SM1, near - start, V((start + near) / 2, 0, z), X),
        "plates": fasteners.cut(Part.makeCompound([G.placed(parts.cage_plate(), V(x, 0, z), X, Z)
                                                   for x in (held, O.STOP_X)]), screws(), "epi cage"),
        "rods": Part.makeCompound(rods),
        "cubes": fasteners.cut(Part.makeCompound(
            [cube["cube"].translated(turn)] + [G.placed(cube["cube"], at, Z, X) for at in centres[1:]]),
            screws(), "epi cage"),
        "dichroics": Part.makeCompound([G.placed(cube["dichroic"], at, Z, X) for at in centres[1:]]),
        "mirror": G.mirror(P.SMALL_OPTIC, parts.SMALL_MIRROR, turn, -Y, -X),
        "tubes": Part.makeCompound(tubes),
        "leds": Part.makeCompound(leds),
    }


def light():
    """The layout's light, with the widest objective: to each camera, and the excitation."""
    parallel, converging = layout.bundles(max(P.OBJECTIVES, key=O.back_aperture))
    head, mid, mono = layout.route(converging, False)
    colour = layout.route(converging, True)[2]
    return {"colour": Part.makeCompound([parallel, head, mid, colour]),
            "mono": Part.makeCompound([parallel, head, mid, mono]),
            "excitation": layout.excitation()}


def fold_mirror():
    """The fold mirror, its face on the layout's fold point."""
    return G.mirror(P.MIRROR_DIAMETER, P.MIRROR_THICKNESS, layout.FOLD_POINT, Z, Y)


def _states(make, positions, keys):
    """A compound of the parts `keys` of `make(p)` at each position."""
    return Part.makeCompound([make(p)[k] for p in positions for k in keys])


def sweeps():
    """The moving parts over their travel: the changer and the slider every 10 mm, the camera port
    every 15 degrees of its swing, and each camera over its focus travel."""
    steps = [10.0 * k for k in range(-4, 5)]
    out = {
        "the changer": _states(changer, steps, ("plate", "objectives", "carriage")),
        "the slider": _states(slider, steps, ("carrier", "filters", "carriage")),
        "the camera port's swing": _states(port, range(0, 91, 15), ("swing", "mirror")),
    }
    f = P.CAMERA_FOCUS
    for n in ("mono", "colour"):
        out["the %s camera" % n] = _states(lambda p, n=n: camera(n, p), (-f, 0, f),
                                           ("body", "plugs", "plate"))
    return out


def place(doc, parent=None):
    """Adds the head's parts to `doc`, in groups, under `parent` if it's given. Returns them: the
    structure, the drives, the epi arm, the changer, the slider and the camera port at their stops,
    the cameras, the tube lens, and the fold mirror's mount and mirror."""
    s, cd, sd, arm = structure(), changer_drive(), slider_drive(), epi_arm()
    ch, sl, pt = changer(), slider(), port()
    cams = {n: camera(n) for n in ("mono", "colour")}
    tube = parts.tube_lens()["housing"]
    fold, mirror = fold_mount(), fold_mirror()

    def new_group(name, label):
        g = group(doc, name, label)
        if parent is not None:
            parent.addObject(g)
        return g

    g = new_group("Structure", "Head plate, decks and brackets")
    for name, k, label in (("HeadPlate", "plate", "Head plate"),
                           ("LowerDeck", "lower", "Lower deck: the changer"),
                           ("UpperDeck", "upper", "Upper deck: the tube lens, slider and cameras"),
                           ("PortBracket", "bracket", "Camera port's bracket"),
                           ("EpiPlate", "epi", "Epi plate"),
                           ("EpiBracket", "epi_bracket", "Epi plate's bracket")):
        add(doc, name, s[k], "metal", g, label)
    g = new_group("Changer", "Objective changer, the 10x on the axis")
    add(doc, "ChangerPlate", ch["plate"], "metal", g, "Changer plate, with its nut block and grooves")
    add(doc, "Objectives", ch["objectives"], "objective", g, "Objectives: 5x, 10x, 20x")
    add(doc, "ChangerCarriage", ch["carriage"], "metal", g, "Changer's MGN12H carriage")
    for k in ("rail", "motor", "screw", "bracket", "block", "plunger", "switch"):
        style = "drive" if k in ("motor", "plunger", "switch") else "metal"
        add(doc, "Changer" + k.title(), cd[k], style, g,
            "Changer's %s" % {"block": "detent block", "plunger": "ball plunger"}.get(k, k))
    g = new_group("Slider", "Filter slider, cell (b) on the axis")
    add(doc, "SliderCarrier", sl["carrier"], "metal", g,
        "Slider's carrier: (a) quad cube, (b) open, (c) 50:50")
    add(doc, "SliderFilters", sl["filters"], "optic", g, "Chroma 89402 in (a), a 50:50 plate in (c)")
    add(doc, "SliderCarriage", sl["carriage"], "metal", g, "Slider's MGN12H carriage")
    for k in ("rail", "motor", "screw", "bracket", "switch"):
        add(doc, "Slider" + k.title(), sd[k], "drive" if k in ("motor", "switch") else "metal", g,
            "Slider's %s" % k)
    g = new_group("TubeLensAndMirrors", "Tube lens, fold mirror and camera port")
    add(doc, "TubeLens", tube, "objective", g, "Tube lens: Thorlabs TTL200-A, in the deck's SM2 thread")
    add(doc, "FoldMount", fold, "metal", g, "Fold mirror's mount")
    add(doc, "FoldMirror", mirror, "mirror", g, "Fold mirror")
    add(doc, "PortBody", pt["body"], "drive", g, "Camera port: Thorlabs MFF102/M, upside down")
    add(doc, "PortSwing", pt["swing"], "metal", g, "Camera port's arm and ring, position 1")
    add(doc, "PortMirror", pt["mirror"], "mirror", g, "Camera port's mirror: light to the colour camera")
    g = new_group("Cameras", "Cameras on their focus mounts")
    for n, c in cams.items():
        t = n.title()
        add(doc, t + "Camera", c["body"], "body", g, "%s camera" % t)
        add(doc, t + "Sensor", c["sensor"], "sensor", g, "%s sensor" % t)
        add(doc, t + "Plugs", c["plugs"], "reserved", g, "Room for the %s camera's plugs" % n)
        add(doc, t + "Focus", c["plate"], "metal", g, "%s camera's focus plate" % t)
        add(doc, t + "Mount", c["base"], "metal", g, "%s camera's mount" % t)
    g = new_group("EpiArm", "Epi arm")
    for k, style in (("tube", "metal"), ("plates", "metal"), ("rods", "metal"), ("cubes", "metal"),
                     ("dichroics", "optic"), ("mirror", "mirror"), ("tubes", "metal"),
                     ("leds", "drive")):
        add(doc, "Epi" + k.title(), arm[k], style, g, {
            "tube": "Epi lens's SM1 tube", "plates": "Cage plates: the lens tube's, the field stop's",
            "rods": "Cage rods", "cubes": "Cage cubes: the fold's and the combiners'",
            "dichroics": "Combiners' dichroics", "mirror": "Epi fold mirror",
            "tubes": "Collectors' SM1 tubes", "leds": "Mounted LEDs"}[k])
    return s, cd, sd, arm, ch, sl, pt, cams, tube, fold, mirror


def build(doc):
    s, cd, sd, arm, ch, sl, pt, cams, tube, fold, fold_mirror = place(doc)
    lit = light()
    # The imaging light is optical-layout.FCStd's; the checks below use it, and the file leaves it out.
    add(doc, "Excitation", lit["excitation"], "excitation", group(doc, "Light"),
        "Excitation light (schematic)")

    g = group(doc, "Stops", "The moving parts at their other stops")
    add(doc, "ChangerStops", _states(changer, CHANGES, ("plate", "objectives")), "reserved", g,
        "The changer at each objective")
    add(doc, "SliderStops", _states(slider, SLIDES, ("carrier",)), "reserved", g,
        "The slider at each cell")
    add(doc, "PortPosition2", _states(port, (90,), ("swing", "mirror")), "reserved", g,
        "The camera port in position 2")

    swept = sweeps()
    f = P.CAMERA_FOCUS
    fixed = {
        "the structure": list(s.values()),
        "the changer's drive": list(cd.values()),
        "the slider's drive": list(sd.values()),
        "the tube lens": [tube],
        "the fold mirror's mount": [fold, fold_mirror],
        "the camera port's body": [pt["body"]],
        "the mono camera's mount": [cams["mono"]["base"]],
        "the colour camera's mount": [cams["colour"]["base"]],
        "the epi arm": list(arm.values()),
    }
    fixed = {k: Part.makeCompound(v) for k, v in fixed.items()}
    names, moving = list(fixed), list(swept)
    found = clashes([("%s and %s" % (a, b), fixed[a], fixed[b])
                     for i, a in enumerate(names) for b in names[i + 1:]])
    moved = clashes([("%s and %s" % (m, k), swept[m], fixed[k]) for m in moving for k in names]
                    + [("%s and %s" % (a, b), swept[a], swept[b])
                       for i, a in enumerate(moving) for b in moving[i + 1:]])

    # The light against the mechanical parts: everything but the optics.
    optics = ("the tube lens", "the fold mirror's mount", "the epi arm")
    mech = Part.makeCompound([fixed[k] for k in names if k not in optics] + [fold]
                             + [v for k, v in arm.items() if k not in ("dichroics", "mirror")]
                             + [cams[n]["plate"] for n in cams])
    both = Part.makeCompound([lit["colour"], lit["mono"]])
    lit_found = clashes(
        [("the changer's plate, the %s on the axis" % n, both, changer(o)["plate"])
         for n, o in zip(P.CHANGER_ORDER, CHANGES)]
        + [("the slider, cell (%s) on the axis" % c, both, slider(o)["carrier"])
           for c, o in zip("abc", SLIDES)]
        + [("the port's arm in position 1", lit["colour"], port(0)["swing"]),
           ("the port's arm in position 2", lit["mono"], port(90)["swing"]),
           ("the fixed parts", both, mech)]
        + [("the excitation and the parts, cell (%s) on the axis" % c, lit["excitation"],
            Part.makeCompound([mech, slider(o)["carrier"]]))
           for c, o in (("a", SLIDES[0]), ("c", SLIDES[2]))])

    # Seated, the ball touches the groove's walls: test it a hair smaller.
    ball = detent_ball(0.01)
    seated = [changer(o)["plate"].common(ball).Volume < 1e-6 for o in CHANGES]
    pushed = [changer((a + b) / 2)["plate"].common(ball).Volume > 1e-3
              for a, b in zip(CHANGES, CHANGES[1:])]
    travel = (P.MGN12_RAIL[3] - P.MGN12H[1]) / 2
    face, _, _, ch_len = P.CHANGER_SCREW
    nx0, nx1, _ = P.CHANGER_NUT
    sface, _, sl_len = P.SLIDER_SCREW
    screws = [(face + 2, face + 2 + ch_len, nx0 + min(CHANGES), nx1 + max(CHANGES)),
              (sface - 2 - sl_len, sface - 2, min(SLIDES) - NUT, max(SLIDES) + NUT)]
    engaged = DECK_TOP - max(UPPER_Z, tube.BoundBox.ZMin)
    checks = [
        Check("The fixed parts don't overlap each other", not found, ", ".join(found) or "no overlaps"),
        Check("The moving parts clear everything across their travel", not moved,
              ", ".join(moved) or "the changer and the slider every 10 mm, the port every 15 degrees, "
              "the cameras over +-%g mm of focus" % f),
        Check("The light clears the head's parts", not lit_found,
              ", ".join(lit_found) or "with every objective and every slider cell on the axis"),
        Check("The detent's ball seats in a groove at each objective, and rides on the edge between",
              all(seated) and all(pushed), "a %g mm ball, %g mm-deep 90-degree grooves" % P.DETENT[1:]),
        Check("The carriages stay on their rails, %g mm from each end" % P.TRAVEL_MARGIN,
              max(CHANGES + SLIDES) + P.TRAVEL_MARGIN <= travel,
              "+-%g mm of +-%.1f mm" % (max(CHANGES + SLIDES), travel)),
        Check("The lead screws reach the nuts across the travel",
              all(lo <= a and b <= hi for lo, hi, a, b in screws),
              "the changer's %g mm screw, the slider's %g mm" % (ch_len, sl_len)),
        Check("The tube lens takes at least 5 mm of the deck's thread", engaged >= 5,
              "%.1f mm" % engaged),
    ]
    whole = Part.makeCompound(list(fixed.values()) + [ch["plate"], sl["carrier"]]
                              + [c[k] for c in cams.values() for k in ("body", "plugs")]).BoundBox
    facts = [
        ("Head plate's back face, y: the column meets it", P.HEAD_PLATE[4]),
        ("Lower deck's top, z", LOWER_TOP),
        ("Upper deck, z", "%.1f to %.1f" % (UPPER_Z, DECK_TOP)),
        ("Changer's offsets, 5x 10x 20x", " ".join("%+g" % o for o in CHANGES)),
        ("Slider's offsets, cells a b c", " ".join("%+g" % o for o in SLIDES)),
        ("Camera port's base, z (upside down)", round(layout.FLIP_POINT.z + P.FLIP_OPTIC, 1)),
        ("Focus travel, each camera", "+-%g mm" % f),
        ("The head, x", "%.1f to %.1f" % (whole.XMin, whole.XMax)),
        ("The head, y", "%.1f to %.1f" % (whole.YMin, whole.YMax)),
        ("The head, z", "%.1f to %.1f" % (whole.ZMin, whole.ZMax)),
    ]
    return checks, facts

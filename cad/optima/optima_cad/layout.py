"""The optical layout: the axis, apertures, mirrors, sensors and the light's envelopes.

Millimetres. The origin is on the optical axis at the sample's focal plane. +Z points up toward
the objectives, +X to the right seen from the front (the epi arm's side), +Y to the back.

Shapes on the imaging path are built on the unfolded axis, then folded by the fold mirror and
the flip mirror, so the light, the sensors and the camera bodies stay where the optics put them.
The fold mirror sends the light back, to the mono camera; the flip mirror, when it's in, sends it
right, to the colour camera.
"""

import math

import FreeCAD as App
import Part

from . import cameras, parts
from . import geom as G
from . import optics as O
from . import params as P
from .checks import Check, clashes
from .document import add, group

V = App.Vector

FOLD_POINT = V(0, 0, O.FOLD)
FOLD_NORMAL = V(0, 1, -1)
FLIP_POINT = V(0, P.FLIP_AFTER_FOLD, O.FOLD)
FLIP_NORMAL = V(1, -1, 0)
DICHROIC = V(0, 0, O.FILTER)
TURN = V(O.EPI_FOLD_X, 0, O.FILTER)  # the epi fold mirror
X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
LED_COLOURS = {395: (0.55, 0.0, 1.0), 470: (0.0, 0.45, 1.0), 565: (0.7, 1.0, 0.0), 625: (1.0, 0.2, 0.0)}


def route(shape, flip_in):
    """Folds a shape on the unfolded axis: the part before the fold, before the flip, after it."""
    head, tail = G.split(shape, FOLD_POINT, FOLD_NORMAL, V(0, 0, O.TUBE))
    tail = tail.mirror(FOLD_POINT, FOLD_NORMAL)
    mid, end = G.split(tail, FLIP_POINT, FLIP_NORMAL, FOLD_POINT)
    return head, mid, end.mirror(FLIP_POINT, FLIP_NORMAL) if flip_in else end


def field_points():
    w, h = P.SENSOR_SIZE
    return [(0, 0)] + [(sx * w / 2, sy * h / 2) for sx in (-1, 1) for sy in (-1, 1)]


def bundles(objective):
    """Light for the image's centre and corners: in the infinity space, and converging."""
    d, k = O.back_aperture(objective), P.INFINITY_SPACE / P.TUBE_F
    parallel = [G.tilted_cylinder(d, V(0, 0, O.PUPIL), V(u, v, P.TUBE_F) * k) for u, v in field_points()]
    converging = [G.taper(d, V(u * k, v * k, O.TUBE), 0.1, V(u, v, O.IMAGE)) for u, v in field_points()]
    return Part.makeCompound(parallel), Part.makeCompound(converging)


@G.kept
def camera_frames():
    """Each camera's sensor centre, and the direction its light arrives along."""
    sensor = G.box((*P.SENSOR_SIZE, 0.5), V(0, 0, O.IMAGE))
    s, c = (route(sensor, flip_in)[2].BoundBox.Center for flip_in in (False, True))
    return {"mono": (s, Y), "colour": (c, X)}


def epi_leds():
    """Each LED's wavelength, its combiner's centre (None for the LED on the combiner's axis),
    its collector's and its die's centres, and the axis it shines along."""
    out = []
    for k, nm in enumerate(P.LEDS_NM):
        if k < len(P.LEDS_NM) - 1:
            at = TURN + Y * (P.COMBINER_PITCH * (k + 1))
            out.append((nm, at, at + X * 25, at + X * (25 + P.COLLECTOR_F), X))
        else:
            end = TURN + Y * (P.COMBINER_PITCH * len(P.LEDS_NM))
            out.append((nm, None, end, end + Y * P.COLLECTOR_F, Y))
    return out


def excitation():
    """The excitation light, schematic: from the collectors to the objective's back aperture."""
    stop_d, lens_at = O.field_stop(), V(O.EPI_LENS_X, 0, O.FILTER)
    cone = G.taper(stop_d, lens_at, O.led_image(), lens_at - X * P.EPI_LENS_F, X)
    before, after = G.split(cone, DICHROIC, V(1, 0, -1), lens_at)
    beam = [before, after.mirror(DICHROIC, V(1, 0, -1)),
            Part.makeCylinder(stop_d / 2, O.EPI_FOLD_X - O.EPI_LENS_X, lens_at, X),
            Part.makeCylinder(stop_d / 2, P.COMBINER_PITCH * len(P.LEDS_NM), TURN, Y)]
    beam += [Part.makeCylinder(stop_d / 2, 25, at, X) for _, at, *_ in epi_leds() if at is not None]
    return G.solids(*beam)


def reach(shape, point, normal):
    """How far from `point` the shape crosses the plane through it."""
    plane = G.facing(Part.makePlane(1000, 1000, V(-500, -500, 0)), point, normal)
    return max((p - point).Length for e in shape.section(plane).Edges for p in e.discretize(48))


def build(doc):
    checks, facts = [], []
    active = P.ACTIVE_OBJECTIVE
    w, h = P.SENSOR_SIZE

    g = group(doc, "Sample")
    add(doc, "Slide", G.box(P.SLIDE, V(0, 0, -P.SLIDE[2] / 2)), "sample", g)
    add(doc, "CoverGlass", G.box(P.COVER_GLASS, V(0, 0, P.COVER_GLASS[2] / 2)), "sample", g)
    for name, o in P.OBJECTIVES.items():
        m = P.TUBE_F / o["f"]
        add(doc, "Field" + name, G.box((w / m, h / m, 0.02), V(0, 0, 0.18)), "field", g,
            "Field of view, %s: %.2f x %.2f mm" % (name, w / m, h / m))

    g = group(doc, "Objectives", "Objectives on the changer")
    k_active = P.CHANGER_ORDER.index(active)
    for k, name in enumerate(P.CHANGER_ORDER):
        o, x = P.OBJECTIVES[name], (k - k_active) * P.CHANGER_PITCH
        body = parts.objective(name).translated(V(x, 0, 0))
        add(doc, "Objective" + name, body, "objective", g, "Objective %s, NA %.2f" % (name, o["na"]))
        add(doc, "Pupil" + name, G.disc(O.back_aperture(name), 0.5, V(x, 0, O.PUPIL), Z), "optic", g,
            "Exit pupil %s (assumed depth)" % name)

    g = group(doc, "TubeLensAndMirrors", "Tube lens and mirrors")
    tube, pupil = parts.tube_lens().values()
    add(doc, "TubeLens", tube, "optic", g, "Tube lens: Thorlabs TTL200-A")
    add(doc, "TubePupil", pupil, "optic", g, "Tube lens entrance pupil")
    fold = G.mirror(P.MIRROR_DIAMETER, P.MIRROR_THICKNESS, FOLD_POINT, Z, Y)
    flip = G.mirror(P.MIRROR_DIAMETER, P.MIRROR_THICKNESS, FLIP_POINT, Y, X)
    add(doc, "FoldMirror", fold, "mirror", g, "Fold mirror")
    add(doc, "FlipMirror", flip, "mirror", g, "Flip mirror (in: light to the colour camera)")

    parallel, converging = bundles(active)
    head, mid, mono = route(converging, False)
    colour = route(converging, True)[2]
    g = group(doc, "Imaging", "Imaging light, %s" % active)
    add(doc, "InfinitySpace", parallel, "imaging", g, "Infinity space: centre and corners")
    add(doc, "ToFold", head, "imaging", g, "Tube lens to fold mirror")
    add(doc, "ToFlip", mid, "imaging", g, "Fold mirror to flip mirror")
    add(doc, "ToMono", mono, "imaging", g, "To the mono camera (flip mirror out)")
    add(doc, "ToColour", colour, "colour", g, "To the colour camera (flip mirror in)")

    (s, mono_axis), (c, colour_axis) = camera_frames().values()
    # Each camera at its sensor, facing the light, its mounting face down.
    mono_parts = {k: G.placed(v, s, mono_axis, Z) for k, v in cameras.mono().items()}
    colour_parts = {k: G.placed(v, c, colour_axis, Z) for k, v in cameras.colour().items()}
    g = group(doc, "Cameras")
    add(doc, "SensorMono", mono_parts["sensor"], "sensor", g, "Mono sensor")
    add(doc, "SensorColour", colour_parts["sensor"], "sensor", g, "Colour sensor")
    add(doc, "CameraMono", mono_parts["body"], "body", g, "Mono camera: ToupTek CTR3CMOS45000KMA")
    add(doc, "CameraColour", colour_parts["body"], "body", g, "Colour camera: ToupTek E3ISPM45000KPB")
    add(doc, "PlugsMono", mono_parts["plugs"], "reserved", g, "Room for the mono camera's plugs")
    add(doc, "PlugsColour", colour_parts["plugs"], "reserved", g, "Room for the colour camera's plug")
    mono_cam = Part.makeCompound([mono_parts["body"], mono_parts["plugs"]])
    colour_cam = Part.makeCompound([colour_parts["body"], colour_parts["plugs"]])

    g = group(doc, "Epi", "Epi illumination")
    stop_d, image_d = O.field_stop(), O.led_image()
    lens_at, stop_at = (V(x, 0, O.FILTER) for x in (O.EPI_LENS_X, O.STOP_X))
    lens = G.disc(P.SMALL_OPTIC, 4, lens_at, X)
    stop = G.ring(P.SMALL_OPTIC, stop_d, 1, stop_at, X)
    add(doc, "Dichroic", G.plate(P.DICHROIC, DICHROIC, -X, -Z), "optic", g, "Quad dichroic")
    add(doc, "EpiLens", lens, "optic", g, "Epi lens, f = %g mm" % P.EPI_LENS_F)
    add(doc, "FieldStop", stop, "mirror", g, "Field stop, %.1f mm" % stop_d)
    add(doc, "EpiFold", G.mirror(P.SMALL_OPTIC, 6, TURN, -Y, -X), "mirror", g, "Epi fold mirror")
    for nm, at, collector, die, axis in epi_leds():
        if at is not None:
            add(doc, "Combiner%d" % nm, G.plate((25.4, 25.4, 1), at, -X, -Y), "optic", g,
                "Combiner dichroic for %d nm" % nm)
        add(doc, "Collector%d" % nm, G.disc(P.SMALL_OPTIC, 5, collector, axis), "optic", g,
            "Collector, %d nm LED" % nm)
        led = add(doc, "Led%d" % nm, G.disc(12, 8, die, axis), "led", g, "LED, %d nm" % nm)
        led.ViewObject.ShapeColor = LED_COLOURS[nm]
    add(doc, "Excitation", excitation(), "excitation", g, "Excitation light (schematic)")

    g = group(doc, "Transmitted", "Transmitted light")
    na = P.OBJECTIVES[active]["na"]
    top = -P.MATRIX_BELOW_SAMPLE
    inside = [(x, y) for x, y in O.matrix_leds() if O.led_na(x, y) <= na]
    outside = [(x, y) for x, y in O.matrix_leds() if O.led_na(x, y) > na]
    board = parts.led_matrix()["board"].translated(V(0, 0, top))
    add(doc, "MatrixBoard", board, "board", g, "LED matrix: Adafruit 2734")
    for name, leds, style in (("LedsInside", inside, "transmitted"), ("LedsOutside", outside, "led")):
        shape = parts.matrix_leds(leds).translated(V(0, 0, top))
        add(doc, name, shape, style, g, "LEDs %s the %s's NA" % (name[4:].lower(), active))
    spread = 2 * max(abs(c) for xy in inside for c in xy) + P.MATRIX_LED
    m = P.TUBE_F / P.OBJECTIVES[active]["f"]
    light = G.rect_loft((spread, spread), V(0, 0, top), (w / m, h / m), V(0, 0, 0))
    add(doc, "Brightfield", light, "transmitted", g, "Brightfield light, %s" % active)

    g = group(doc, "Reserved", "Reserved for moving parts")
    reach_x = 2 * P.CHANGER_PITCH + P.OBJECTIVE_DIAMETER / 2
    min_wd = min(o["wd"] for o in P.OBJECTIVES.values())
    changer = G.span(V(-reach_x - 8, -25, O.SHOULDER), V(reach_x + 8, 25, O.CARRIAGE_TOP))
    radius = P.OBJECTIVE_DIAMETER / 2
    sweep = G.span(V(-reach_x, -radius, min_wd), V(reach_x, radius, O.SHOULDER))
    half, slide = P.FILTER_CUBE / 2, 2.5 * P.FILTER_PITCH
    slider = G.span(V(-20, -slide, O.FILTER - half), V(20, slide, O.FILTER + half))
    add(doc, "ChangerTravel", changer, "reserved", g, "Objective changer carriage, all positions")
    add(doc, "ObjectiveTravel", sweep, "reserved", g, "Objectives, all positions")
    add(doc, "SliderTravel", slider, "reserved", g, "Filter slider, all positions")

    for name in P.CHANGER_ORDER:
        longest = O.max_infinity_space(name)
        margin = longest - P.INFINITY_SPACE
        checks.append(Check("Infinity space, %s: corners clear the tube lens's pupil" % name,
                            margin >= 0, "%.0f mm of %.1f mm allowed" % (P.INFINITY_SPACE, longest)))
    out = O.sensor_outside_field()
    checks.append(Check("Sensor within the tube lens's rated field", out < 0.01,
                        "%.2f %% of the sensor is outside the %g mm field" % (out * 100, P.TUBE_FIELD)))
    after_fold = Part.makeCompound([mid, mono])
    for name, point, normal, shape in (("Fold", FOLD_POINT, FOLD_NORMAL, converging),
                                       ("Flip", FLIP_POINT, FLIP_NORMAL, after_fold)):
        r = reach(shape, point, normal)
        checks.append(Check("%s mirror: light within its clear aperture" % name, r <= P.MIRROR_CLEAR / 2,
                            "%.1f mm from the centre, of %.1f mm" % (r, P.MIRROR_CLEAR / 2)))
    shared = Part.makeCompound([parallel, head, mid])
    found = clashes([
        ("the cameras", mono_cam, colour_cam),
        ("the mono camera and its light", mono_cam, mono),
        ("the colour camera and its light", colour_cam, colour),
        ("the mono camera and the colour light", mono_cam, colour),
        ("the colour camera and the mono light", colour_cam, mono),
        ("a camera and the shared light", Part.makeCompound([mono_cam, colour_cam]), shared),
        ("the fold mirror and the tube lens", fold, tube),
        ("the two mirrors", fold, flip),
        ("the epi optics and the moving parts", Part.makeCompound([lens, stop]),
         Part.makeCompound([changer, sweep, slider])),
    ])
    checks.append(Check("Nothing collides", not found, ", ".join(found) or "no overlaps"))
    stack = [O.SHOULDER, O.CARRIAGE_TOP, O.FILTER - half, O.FILTER + half, O.TUBE]
    checks.append(Check("Stack along the axis: changer, slider, tube lens", stack == sorted(stack),
                        " < ".join("%.1f" % z for z in stack)))
    smallest = min(O.back_aperture(n) for n in P.OBJECTIVES)
    checks.append(Check("LED image fits the smallest back aperture", image_d <= smallest,
                        "%.1f mm in %.1f mm" % (image_d, smallest)))
    groups = {n: O.leds_inside(o["na"]) for n, o in P.OBJECTIVES.items()}
    checks.append(Check("LED matrix: LEDs inside each NA match the design", groups == P.MATRIX_NA_GROUPS,
                        ", ".join("%s %d" % kv for kv in groups.items())))

    facts += [
        ("Objective shoulder, z", O.SHOULDER),
        ("Objective exit pupil, z (assumed)", O.PUPIL),
        ("Changer carriage top, z", O.CARRIAGE_TOP),
        ("Filter cube centre, z", O.FILTER),
        ("Tube lens entrance pupil, z", O.TUBE),
        ("Tube lens back, z", O.TUBE_BACK),
        ("Fold mirror centre, z", O.FOLD),
        ("Mono sensor centre, x y z", "%.1f %.1f %.1f" % (s.x, s.y, s.z)),
        ("Colour sensor centre, x y z", "%.1f %.1f %.1f" % (c.x, c.y, c.z)),
        ("Epi lens, field stop, epi fold: x", "%.1f %.1f %.1f" % (O.EPI_LENS_X, O.STOP_X, O.EPI_FOLD_X)),
        ("Field stop diameter", round(stop_d, 2)),
        ("LED image diameter", round(image_d, 2)),
        ("Corner angle in the infinity space, degrees", round(math.degrees(O.corner_angle()), 2)),
    ]
    return checks, facts

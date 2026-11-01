"""Shape helpers on top of FreeCAD's Part module."""

import FreeCAD as App
import Part

V = App.Vector
Z = V(0, 0, 1)


def box(size, center):
    sx, sy, sz = size
    return Part.makeBox(sx, sy, sz, center - V(sx / 2, sy / 2, sz / 2))


def span(lo, hi):
    """A box between two corners."""
    return Part.makeBox(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z, lo)


def disc(diameter, thickness, center, normal):
    """A disc centred on `center`, its faces perpendicular to `normal`."""
    n = V(normal).normalize()
    return Part.makeCylinder(diameter / 2, thickness, center - n * (thickness / 2), n)


def ring(outer, inner, thickness, center, normal):
    return disc(outer, thickness, center, normal).cut(disc(inner, thickness * 2, center, normal))


def facing(shape, center, normal):
    """Turns a shape built around the origin, facing +Z, to face `normal` at `center`."""
    shape = shape.copy()
    shape.Placement = App.Placement(center, App.Rotation(Z, V(normal)))
    return shape


def mirror(diameter, thickness, center, light_in, light_out):
    """A round mirror whose face, on `center`, turns `light_in` into `light_out`."""
    face = (V(light_out) - V(light_in)).normalize()
    return disc(diameter, thickness, center - face * (thickness / 2), face)


def plate(size, center, light_in, light_out):
    """A flat plate, such as a dichroic, centred on `center`, at the mirror's angle."""
    return facing(box(size, V(0, 0, 0)), center, V(light_out) - V(light_in))


def tilted_cylinder(diameter, base, offset):
    """A cylinder whose end faces stay horizontal while its axis follows `offset`."""
    return Part.Face(Part.Wire(Part.makeCircle(diameter / 2, base, Z))).extrude(offset)


def taper(d1, c1, d2, c2, normal=Z):
    """A solid between two parallel circles, which needn't share an axis."""
    wires = [Part.Wire(Part.makeCircle(d / 2, c, normal)) for d, c in ((d1, c1), (d2, c2))]
    return Part.makeLoft(wires, True)


def rect_loft(size1, c1, size2, c2):
    """A solid between two horizontal rectangles."""

    def rect(size, c):
        w, h = size[0] / 2, size[1] / 2
        pts = [c + V(x, y, 0) for x, y in ((-w, -h), (w, -h), (w, h), (-w, h), (-w, -h))]
        return Part.makePolygon(pts)

    return Part.makeLoft([rect(size1, c1), rect(size2, c2)], True)


def corners(lo_hi):
    """A box from its corners as (x0, y0, z0, x1, y1, z1)."""
    return span(V(*lo_hi[:3]), V(*lo_hi[3:]))


def bounds(shape):
    """The box that bounds `shape`."""
    b = shape.BoundBox
    return span(V(b.XMin, b.YMin, b.ZMin), V(b.XMax, b.YMax, b.ZMax))


def rounded(shape, radius, axis=Z):
    """`shape` with its edges along `axis` rounded."""
    edges = [e for e in shape.Edges if abs(abs(e.tangentAt(e.FirstParameter).dot(axis)) - 1) < 1e-9]
    return shape.makeFillet(radius, edges)


def drill(shape, holes, z, inward):
    """Round holes into a horizontal face at height `z`, going `inward` (Z or -Z). Each of
    `holes` is (diameter, depth or None for through, centres)."""
    tools = [Part.makeCylinder(d / 2, (depth or 1000) + 1, V(x, y, z) - inward, inward)
             for d, depth, centres in holes for x, y in centres]
    return shape.cut(Part.makeCompound(tools))


def placed(shape, origin, z_axis, y_axis):
    """`shape`, built in its own frame, with that frame's origin moved to `origin` and its Z and
    Y axes turned to `z_axis` and `y_axis`."""
    x, y, z = V(y_axis).cross(V(z_axis)), V(y_axis), V(z_axis)
    m = App.Matrix(x.x, y.x, z.x, origin.x, x.y, y.y, z.y, origin.y, x.z, y.z, z.z, origin.z)
    moved = shape.copy()
    moved.transformShape(m)
    return moved


def split(shape, point, normal, source):
    """The parts of `shape` on `source`'s side of a plane, and beyond it."""
    n = V(normal).normalize()
    if (V(source) - V(point)).dot(n) < 0:
        n = -n
    half = facing(Part.makeBox(4000, 4000, 2000, V(-2000, -2000, 0)), point, n)
    return shape.common(half), shape.cut(half)


def solids(*shapes):
    """A compound of the non-empty solids among `shapes`."""
    kept = [s for s in shapes if not s.isNull() and s.Volume > 1e-9]
    return Part.makeCompound(kept)

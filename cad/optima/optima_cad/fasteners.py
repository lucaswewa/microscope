"""Screws: where each one goes, the holes it needs, its shape, and the checks that it fits.

A Screw is a socket-head screw: its size; the point its head seats on; the axis it goes in along;
how far it goes through the parts it clamps, its grip; how far into the part it's screwed into,
its depth, or 0 for a nut or a T-nut; and those parts' names. `clamps` is one name, or several for
a screw through a stack. A sunk head sits in a counterbore. A part's builder cuts the holes of the
screws that name it with `cut`, unless they're `drilled` already.
"""

from collections import Counter, namedtuple

import FreeCAD as App
import Part

from . import params as P
from .checks import Check

V = App.Vector
Screw = namedtuple("Screw", "size seat axis grip depth clamps into sunk drilled",
                   defaults=(False, False))


def clamped(s):
    return (s.clamps,) if isinstance(s.clamps, str) else tuple(s.clamps)


def cut(shape, screws, name):
    """`shape`, the part `name`, with the holes for its screws: a clearance hole where it's clamped,
    a counterbore where a head sinks into it, and a tapping hole where a screw goes into it."""
    tools = []
    for s in screws:
        if s.drilled:
            continue
        clear, tap, head, h = P.SCREWS[s.size]
        seat, axis = V(*s.seat), V(*s.axis)
        if name in clamped(s):
            tools.append(Part.makeCylinder(clear / 2, s.grip + 2, seat - axis, axis))
            if s.sunk and name == clamped(s)[0]:
                tools.append(Part.makeCylinder(head / 2 + 0.5, h + 1.5, seat - axis * (h + 1.5), axis))
        if name == s.into and s.depth:
            tools.append(Part.makeCylinder(tap / 2, s.depth + 1, seat + axis * (s.grip - 0.5), axis))
    return shape.cut(Part.makeCompound(tools)).removeSplitter() if tools else shape


def solid(s):
    """The screw: its head on its seat, and its shank, a little under its tapping drill, through
    its grip and depth."""
    _, tap, head, h = P.SCREWS[s.size]
    seat, axis = V(*s.seat), V(*s.axis)
    # One solid: a boolean with a compound whose pieces touch can miss an overlap.
    return Part.makeCylinder(head / 2, h, seat - axis * h, axis).fuse(
        Part.makeCylinder(tap / 2 - 0.05, s.grip + s.depth, seat, axis))


def length(s):
    """The shortest standard length that reaches through its grip and its depth, or a nut."""
    need = s.grip + (s.depth or s.size)
    return next(n for n in P.SCREW_LENGTHS if n >= need)


def counts(screws):
    """How many screws of each size and length: {(size, length): count}."""
    return Counter((s.size, length(s)) for s in screws)


def holes(screws, name):
    """A note of the part `name`'s threaded and counterbored holes."""
    tapped = Counter((s.size, s.depth) for s in screws if s.into == name and s.depth)
    sunk = Counter(s.size for s in screws if s.sunk and clamped(s)[0] == name)
    out = ["%d x M%g x %g deep" % (n, size, depth) for (size, depth), n in sorted(tapped.items())]
    out = ["Tapped: " + ", ".join(out) + "."] if out else []
    if sunk:
        out.append("Counterbored for %s."
                   % ", ".join("%d x M%g" % (n, k) for k, n in sorted(sunk.items())))
    return " ".join(out)


def _inside(shape, point):
    return any(s.isInside(point, 0.01, True) for s in shape.Solids)


def check(screws, named, fixed, swept, moving):
    """Checks that every screw clears the parts in `fixed`, and those on fixed parts the moving
    parts' sweeps in `swept`; and that its head bears on the part it clamps and its thread has
    material all round it, or reaches its T-nut's part. `named` maps the parts' names to their
    shapes, and `moving` names the moving ones."""
    boxes = [(p, p.BoundBox) for p in fixed]
    sweeps = [(p, p.BoundBox) for p in swept]
    hit, loose = [], []
    for s in screws:
        screw = solid(s)
        box = screw.BoundBox
        near = boxes if moving & set(clamped(s) + (s.into,)) else boxes + sweeps
        if any(b.intersect(box) and screw.common(p).Volume > 1e-6 for p, b in near):
            hit.append("the M%g at %.0f, %.0f, %.0f" % (s.size, *s.seat))
        _, tap, head, _ = P.SCREWS[s.size]
        seat, axis = V(*s.seat), V(*s.axis)
        u = axis.cross(V(0, 0, 1)) if abs(axis.z) < 0.9 else axis.cross(V(1, 0, 0))
        u.normalize()
        rim = [u, -u, axis.cross(u), -axis.cross(u)]
        bears = all(_inside(named[clamped(s)[0]], seat + axis * 0.3 + r * (head / 2 - 0.4)) for r in rim)
        if s.depth:
            mid = seat + axis * (s.grip + s.depth / 2)
            held = all(_inside(named[s.into], mid + r * (tap / 2 + 1)) for r in rim)
        else:
            held = s.into is None or named[s.into].BoundBox.isInside(seat + axis * (s.grip + 1))
        if not (bears and held):
            loose.append("the M%g at %.0f, %.0f, %.0f" % (s.size, *s.seat))
    return [
        Check("Every screw clears the parts, the moving ones across their travel", not hit,
              ", ".join(hit[:4]) or "%d screws" % len(screws)),
        Check("Every screw's head bears on its part, and its thread has material all round", not loose,
              ", ".join(loose[:4]) or "1 mm round each tapping hole, at least"),
    ]

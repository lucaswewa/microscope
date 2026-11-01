"""Checks, and the overlaps they look for."""

from collections import namedtuple

Check = namedtuple("Check", "name ok detail")


def clashes(pairs):
    """The names of the (name, shape, shape) pairs whose shapes overlap. Pairs whose bounding
    boxes don't meet are skipped."""
    return [name for name, a, b in pairs
            if a.BoundBox.intersect(b.BoundBox) and a.common(b).Volume > 1e-6]


def matches(name, value, expected, unit="mm", tolerance=0.05):
    """A check that a modelled size is what the vendor's drawing says."""
    return Check(name, abs(value - expected) <= tolerance,
                 "%.2f %s, the drawing says %g" % (value, unit, expected))

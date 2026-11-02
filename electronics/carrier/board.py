"""Builds the board. KiCad's own Python runs this, for its pcbnew module, in separate runs, because
KiCad reads the project's net classes only when it loads a saved board in a new process:

    python board.py place out_folder footprints_folder    places the parts and saves the board
    python board.py finish out_folder                     adds routes.ses's tracks, and fills the pour
    python board.py next out_folder session               loads a router's session for its next job

Parts and fixed tracks go where placement.py says. The GND pour fills the bottom layer. It writes
carrier.kicad_pcb, and carrier.dsn, which is what the router takes in.
"""

import re
import sys
import uuid
from pathlib import Path

import pcbnew

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True

from circuit import PARTS  # noqa: E402
from placement import FIXED, HOLES, LABELS, PLACES, SIZE  # noqa: E402
from schematic import uid  # noqa: E402

NS = uuid.UUID("6f3c2a10-8d57-4f8e-9f70-4a1d5b1c0e05")
MARGIN = 10  # um, on the router's clearances: it keeps the router's rounding inside KiCad's rules
DOOR_GAP = 1000  # um, the door's rule: the router's file can't say it's only from 5 V
# Vias at this cost: at FreeRouting's own, it leaves crossings unrouted sooner than use one.
COSTS = "(fanout off) (autoroute on) (postroute on) (vias on) (via_costs 10) (plane_via_costs 5) (start_ripup_costs 100) (start_pass_no 1)"
mm = pcbnew.FromMM


def point(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def load(name, library, out):
    lib, fp = name.split(":")
    folder = out / "carrier.pretty" if lib == "carrier" else library / (lib + ".pretty")
    footprint = pcbnew.FootprintLoad(str(folder), fp)
    if footprint is None:
        sys.exit("No footprint %s in %s" % (fp, folder))
    footprint.SetFPID(pcbnew.LIB_ID(lib, fp))
    return footprint


def add_outline(board):
    shape = pcbnew.PCB_SHAPE(board)
    shape.SetShape(pcbnew.SHAPE_T_RECT)
    shape.SetStart(point(0, 0))
    shape.SetEnd(point(*SIZE))
    shape.SetLayer(pcbnew.Edge_Cuts)
    shape.SetWidth(mm(0.1))
    board.Add(shape)


def add_part(board, part, library, out, nets, free):
    footprint = load(part.footprint, library, out)
    x, y, angle = PLACES[part.ref]
    footprint.SetReference(part.ref)
    footprint.SetValue(part.value)
    footprint.SetPath(pcbnew.KIID_PATH("/" + uid("part", part.ref)))
    footprint.SetPosition(point(x, y))
    footprint.SetOrientationDegrees(angle)
    board.Add(footprint)
    if part.ref in LABELS:
        dx, dy = LABELS[part.ref]
        footprint.Reference().SetPosition(point(x + dx, y + dy))
    for pad in footprint.Pads():
        number = pad.GetNumber()
        net = part.pins.get(int(number)) if number.isdigit() else None
        if net:
            pad.SetNet(nets[net])
        elif (part.ref, number) in free:
            pad.SetNet(free[(part.ref, number)])


def add_pour(board, net):
    zone = pcbnew.ZONE(board)
    zone.SetLayer(pcbnew.B_Cu)
    zone.SetNet(net)
    outline = zone.Outline()
    outline.NewOutline()
    for x, y in ((0, 0), (SIZE[0], 0), SIZE, (0, SIZE[1])):
        outline.Append(mm(x), mm(y))
    zone.SetMinThickness(mm(0.3))
    zone.SetThermalReliefGap(mm(0.5))
    zone.SetThermalReliefSpokeWidth(mm(0.6))
    board.Add(zone)


def add_fixed(board, nets):
    """The tracks placement.py lays out. They're locked, so the router leaves them alone."""
    for net, layer, width, points in FIXED:
        for a, b in zip(points, points[1:]):
            track = pcbnew.PCB_TRACK(board)
            track.SetStart(point(*a))
            track.SetEnd(point(*b))
            track.SetWidth(mm(width))
            track.SetLayer(getattr(pcbnew, layer.replace(".", "_")))
            track.SetNet(nets[net])
            track.SetLocked(True)
            board.Add(track)


def route_file(text):
    """What the router takes in, from KiCad's export. GND isn't given as a plane, or the router counts
    it as joined while its own tracks cut the pour into islands."""
    text = re.sub(r"^\(pcb .*$", "(pcb carrier", text, count=1, flags=re.M)
    text = re.sub(r"^\s*\(plane .*\n", "", text, flags=re.M)
    text = re.sub(r'^(\s*)\(via "', lambda m: '%s(autoroute_settings %s)\n%s(via "' % (m[1], COSTS, m[1]), text, count=1, flags=re.M)
    text = text.replace("(clearance 200)", "(clearance %d)" % (200 + MARGIN))
    return re.sub(r"(\(class Door .*?\(clearance )\d+", lambda m: "%s%d" % (m[1], DOOR_GAP + MARGIN), text, count=1, flags=re.S)


def write_route_file(board, out):
    dsn = out / "carrier.dsn"
    pcbnew.ExportSpecctraDSN(board, str(dsn))
    dsn.write_text(route_file(dsn.read_text(encoding="utf-8")), encoding="utf-8", newline="\n")


def sort_blocks(text, pattern, key):
    """Sorts the file's run of blocks that match."""
    blocks = re.findall(pattern, text, re.S | re.M)
    start, end = text.index(blocks[0]), text.index(blocks[-1]) + len(blocks[-1])
    assert text[start:end] == "".join(blocks), pattern
    return text[:start] + "".join(sorted(blocks, key=key)) + text[end:]


def steady(text):
    """Puts the parts and tracks in order, and gives every UUID a value from its place in the file, so
    that the same board gives the same file. KiCad saves those in the order of their random UUIDs, and
    the Pico's drawing groups, which nothing needs, in any order."""
    text = re.sub(r'^\t\t\(group ".*?^\t\t\)\n', "", text, flags=re.S | re.M)
    text = sort_blocks(text, r'^\t\(footprint .*?^\t\)\n', lambda b: re.search(r'\(property "Reference" "([^"]*)"', b)[1])
    text = sort_blocks(text, r'^\t\((?:segment|via)\b.*?^\t\)\n', lambda b: re.sub(r'\(uuid "[^"]*"\)', "", b))
    count = iter(range(1 << 24))
    return re.sub(r'\(uuid "[^"]*"\)', lambda m: '(uuid "%s")' % uuid.uuid5(NS, str(next(count))), text)


def place(out, library):
    board = pcbnew.BOARD()
    board.GetTitleBlock().SetTitle("OPTIMA carrier board")
    add_outline(board)
    board.GetDesignSettings().SetAuxOrigin(point(0, SIZE[1]))  # placement files measure from the bottom-left corner
    nets = {}
    for name in sorted({net for p in PARTS for net in p.pins.values() if net}):
        nets[name] = pcbnew.NETINFO_ITEM(board, "/" + name)  # as KiCad names a schematic's local nets
        board.Add(nets[name])
    free = {}  # the free pins' nets, named as the schematic's netlist names them
    netlist = (out / "carrier.net").read_text(encoding="utf-8")
    for name, ref, number in re.findall(r'\(name "(unconnected-[^"]*)"\)\s+\(class "[^"]*"\)\s+\(node\s+\(ref "([^"]+)"\)\s+'
                                        r'\(pin "(\d+)"', netlist):
        free[(ref, number)] = pcbnew.NETINFO_ITEM(board, name)
        board.Add(free[(ref, number)])
    for part in PARTS:
        add_part(board, part, library, out, nets, free)
    for i, (x, y) in enumerate(HOLES):
        hole = load("MountingHole:MountingHole_3.2mm_M3", library, out)
        hole.SetReference("H%d" % (i + 1))
        hole.Reference().SetVisible(False)
        hole.SetBoardOnly(True)
        hole.SetPosition(point(x, y))
        board.Add(hole)
    add_pour(board, nets["GND"])
    add_fixed(board, nets)
    pcbnew.SaveBoard(str(out / "carrier.kicad_pcb"), board)


def finish(out):
    path = out / "carrier.kicad_pcb"
    board = pcbnew.LoadBoard(str(path))
    routes = HERE / "routes.ses"
    if routes.exists():
        pcbnew.ImportSpecctraSES(board, str(routes))
    write_route_file(board, out)
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())
    pcbnew.SaveBoard(str(path), board)
    path.write_text(steady(path.read_text(encoding="utf-8")), encoding="utf-8", newline="\n")


def next_job(out, session):
    """Writes the router's file with a session's tracks in it, and prints how many connections are open."""
    board = pcbnew.LoadBoard(str(out / "carrier.kicad_pcb"))
    pcbnew.ImportSpecctraSES(board, session)
    board.BuildConnectivity()
    write_route_file(board, out)
    print("open", board.GetConnectivity().GetUnconnectedCount(False))


if __name__ == "__main__":
    if sys.argv[1] == "place":
        place(Path(sys.argv[2]), Path(sys.argv[3]))
    elif sys.argv[1] == "finish":
        finish(Path(sys.argv[2]))
    else:
        next_job(Path(sys.argv[2]), sys.argv[3])

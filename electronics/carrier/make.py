#!/usr/bin/env python3
"""Builds the carrier board from circuit.py and placement.py into electronics/carrier/kicad, and checks it.

    python electronics/carrier/make.py           build, and update kicad/ and REPORT.md
    python electronics/carrier/make.py --check   build into a temporary folder, and fail if they differ
    python electronics/carrier/make.py --route   route the board again with FreeRouting, then build

It needs KiCad 10's command line. Set KICAD_CLI to it if this doesn't find it.
"""

import argparse
import csv
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
KICAD = HERE / "kicad"
REPORT = HERE / "REPORT.md"
ROUTES = HERE / "routes.ses"
LAYERS = ("F.Cu", "B.Cu", "F.Paste", "F.Silkscreen", "B.Silkscreen", "F.Mask", "B.Mask", "Edge.Cuts")
JOBS = 8
# FreeRouting sends usage data unless it's told not to. Its automatic narrowing and its fan-out make
# tracks thinner than their class. The clearance from the edge is the DRC's 0.5 mm, and a little more.
ROUTER = ["--profile.allow_telemetry=false", "--usage_and_diagnostic_data.disable_analytics=true", "--gui.enabled=false",
          "--router.automatic_neckdown=false", "--router.fanout.enabled=false", "--router.copper_to_edge_clearance_um=550",
          "--router.autorouter.max_passes=1"]
FILL = re.compile(rb"^\t\t\(filled_polygon\n.*?^\t\t\)\n", re.S | re.M)
ADC_LCSC = "C37593"  # the ADS1115IDGSR, to confirm when ordering
CABLES = HERE.parent.parent / "cad" / "optima" / "outputs" / "cables.csv"

sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True

import checks  # noqa: E402
import footprints  # noqa: E402
import schematic  # noqa: E402
from circuit import PARTS  # noqa: E402


def find_kicad():
    if os.environ.get("KICAD_CLI"):
        return os.environ["KICAD_CLI"]
    if sys.platform == "win32":
        found = sorted(glob.glob(r"C:\Program Files\KiCad\*\bin\kicad-cli.exe"))
        if found:
            return found[-1]
    return shutil.which("kicad-cli") or sys.exit("KiCad's kicad-cli wasn't found. Install KiCad 10, "
                                                 "or set KICAD_CLI to it.")


def write(path, text):
    path.write_text(text, encoding="utf-8", newline="\n")


def clean_netlist(text):
    """Takes out what changes from run to run or between folders: the date and the paths."""
    text = text.replace("\r\n", "\n")
    text = re.sub(r'\(date "[^"]*"\)', '(date "")', text)
    text = re.sub(r'\(source "[^"]*"\)', '(source "carrier.kicad_sch")', text)
    text = re.sub(r'\(tool "[^"]*"\)', '(tool "KiCad")', text)
    return re.sub(r'\(uri "[^"]*"\)', '(uri "")', text)


def board_step(cli, *args):
    """Runs a step of board.py under KiCad's Python, and returns what it prints."""
    done = subprocess.run([str(Path(cli).with_name("python.exe" if os.name == "nt" else "python3")),
                           str(HERE / "board.py")] + list(args), capture_output=True, text=True)
    if done.returncode:
        sys.exit("board.py %s failed:\n%s%s" % (args[0], done.stdout, done.stderr))
    return done.stdout


def kicad(cli, out, *args):
    done = subprocess.run([cli] + list(args), capture_output=True, text=True, cwd=out)
    if done.returncode:
        sys.exit("kicad-cli %s failed:\n%s%s" % (" ".join(args[:3]), done.stdout, done.stderr))


def fabrication(cli, out):
    """What the maker needs: Gerbers and drills in a zip, and the one assembled part's BOM and
    placement in JLCPCB's columns. Also the list of every part, and a picture of the board."""
    pcb, fab = str(out / "carrier.kicad_pcb"), out / "fab"
    gerbers = out / "gerbers"
    gerbers.mkdir()
    fab.mkdir()
    kicad(cli, out, "pcb", "export", "gerbers", "-l", ",".join(LAYERS), "-o", str(gerbers), pcb)
    kicad(cli, out, "pcb", "export", "drill", "-o", str(gerbers) + os.sep, pcb)
    with zipfile.ZipFile(fab / "carrier-gerbers.zip", "w", zipfile.ZIP_DEFLATED) as z:
        for path in sorted(gerbers.iterdir()):
            z.write(path, path.name)
    shutil.rmtree(gerbers)
    placed = out / "placed.csv"
    kicad(cli, out, "pcb", "export", "pos", "--format", "csv", "--units", "mm", "--side", "front",
          "--smd-only", "--use-drill-file-origin", "-o", str(placed), pcb)
    with open(placed, encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    placed.unlink()
    write(fab / "carrier-cpl.csv", "Designator,Mid X,Mid Y,Layer,Rotation\n" + "".join(
        "%s,%s,%s,%s,%g\n" % (r["Ref"], r["PosX"], r["PosY"], r["Side"].capitalize(), float(r["Rot"]) % 360) for r in rows))
    write(fab / "carrier-bom.csv", "Comment,Designator,Footprint,LCSC Part #\n"
          + "ADS1115IDGSR,U10,TSSOP-10 3 x 3 mm,%s\n" % ADC_LCSC)
    groups = defaultdict(list)
    for part in PARTS:
        groups[(part.value, part.footprint.split(":")[1])].append(part.ref)
    write(fab / "carrier-parts.csv", "Quantity,Part,Footprint,References\n" + "".join(
        '%d,"%s","%s","%s"\n' % (len(refs), value, fp, " ".join(refs)) for (value, fp), refs in sorted(groups.items())))
    kicad(cli, out, "pcb", "export", "pdf", "-l", "F.Cu,F.Silkscreen,Edge.Cuts", "--mode-single",
          "-o", str(fab / "carrier-top.pdf"), pcb)
    kicad(cli, out, "pcb", "export", "pdf", "-l", "B.Cu,Edge.Cuts", "--mode-single",
          "-o", str(fab / "carrier-bottom.pdf"), pcb)
    kicad(cli, out, "pcb", "render", "--side", "top", "--width", "1600", "--height", "1150",
          "-o", str(fab / "carrier-top.png"), pcb)


def build(out):
    """Writes the KiCad files into `out`, runs KiCad, and returns the netlist and the checks' results."""
    write(out / "carrier.kicad_sch", schematic.schematic())
    write(out / "carrier.kicad_sym", schematic.library())
    write(out / "carrier.kicad_pro", schematic.project())
    write(out / "sym-lib-table", schematic.SYM_TABLE)
    write(out / "fp-lib-table", schematic.FP_TABLE)
    write(out / "carrier.kicad_dru", schematic.RULES)
    footprints.write(out / "carrier.pretty")
    cli, sheet = find_kicad(), str(out / "carrier.kicad_sch")
    erc_path, drc_path = out / "erc.json", out / "drc.json"
    kicad(cli, out, "sch", "erc", "--format", "json", "--severity-all", "-o", str(erc_path), sheet)
    kicad(cli, out, "sch", "export", "netlist", "-o", str(out / "carrier.net"), sheet)
    kicad(cli, out, "sch", "export", "pdf", "-o", str(out / "carrier.pdf"), sheet)
    library = os.environ.get("KICAD_FOOTPRINTS") or str(Path(cli).parents[1] / "share" / "kicad" / "footprints")
    board_step(cli, "place", str(out), library)
    board_step(cli, "finish", str(out))
    kicad(cli, out, "pcb", "drc", "--schematic-parity", "--format", "json", "--severity-all", "-o", str(drc_path),
          str(out / "carrier.kicad_pcb"))
    erc, drc = (json.loads(p.read_text(encoding="utf-8")) for p in (erc_path, drc_path))
    erc_path.unlink()
    drc_path.unlink()
    fabrication(cli, out)
    for name in ("carrier.kicad_prl", "fp-info-cache"):
        (out / name).unlink(missing_ok=True)  # KiCad's own view settings and cache
    netlist = clean_netlist((out / "carrier.net").read_text(encoding="utf-8"))
    write(out / "carrier.net", netlist)
    return netlist, erc, drc


def route(out):
    """Routes the board with FreeRouting, and keeps the result as routes.ses. It runs a chain of short
    jobs, each from the last one's tracks, because a job's first pass does the work and its later passes
    undo it. FREEROUTING names its .jar, and JAVA the Java 25 that runs it, if that isn't on the path."""
    jar = os.environ.get("FREEROUTING") or sys.exit("Set FREEROUTING to FreeRouting's .jar.")
    ROUTES.unlink(missing_ok=True)
    build(out)
    session, left = out / "routes.ses", None
    for _ in range(JOBS):
        with open(out / "router.log", "w", encoding="utf-8") as log:
            subprocess.run([os.environ.get("JAVA", "java"), "-jar", jar] + ROUTER + [
                "-de", str(out / "carrier.dsn"), "-do", str(session)], check=True, timeout=600, stdout=log, stderr=log)
        left = int(board_step(find_kicad(), "next", str(out), str(session)).split()[-1])
        print("  %d connections open" % left)
        if not left:
            break
    write(ROUTES, session.read_text(encoding="utf-8"))
    shutil.rmtree(out)
    out.mkdir()


def report(results):
    lines = ["# The carrier board's checks", "", "Written by `make.py`. Don't edit.", "",
             "| Check | Result | Detail |", "|---|---|---|"]
    lines += ["| %s | %s | %s |" % (c.name, "pass" if c.ok else "FAIL", c.detail) for c in results]
    return "\n".join(lines) + "\n"


def tree(root):
    """{relative path: bytes} of the built files that come out the same each time: the PDFs, the Gerbers'
    zip and the picture are made from the rest, which stand for them. KiCad's fill of the pour moves by
    a few points between runs, so the board is compared without it."""
    return {p.relative_to(root).as_posix(): FILL.sub(b"", p.read_bytes()) for p in root.rglob("*")
            if p.is_file() and p.suffix not in (".pdf", ".zip", ".png", ".dsn")}


def differs(new, have):
    a, b = tree(new), tree(have) if have.exists() else {}
    return sorted(n for n in set(a) | set(b) if a.get(n) != b.get(n))


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if kicad/ isn't what this builds")
    parser.add_argument("--route", action="store_true", help="route the board again with FreeRouting")
    args = parser.parse_args()
    temp = Path(tempfile.mkdtemp(prefix="carrier-"))
    try:
        if args.route:
            route(temp)
        netlist, erc, drc = build(temp)
        results = checks.run(netlist, erc, CABLES, drc, (temp / "carrier.kicad_pcb").read_text(encoding="utf-8"))
        for c in results:
            print("  %-5s %s: %s" % ("ok" if c.ok else "FAIL", c.name, c.detail))
        text = report(results)
        if args.check:
            stale = differs(temp, KICAD)
            if not REPORT.exists() or REPORT.read_text(encoding="utf-8") != text:
                stale.append("REPORT.md")
            if stale:
                print("Not what the scripts build: %s. Run make.py and commit them." % ", ".join(stale))
                return 1
            print("kicad/ and REPORT.md are up to date.")
        else:
            shutil.rmtree(KICAD, ignore_errors=True)
            (temp / "carrier.dsn").unlink()
            shutil.copytree(temp, KICAD)
            write(REPORT, text)
        return 0 if all(c.ok for c in results) else 1
    finally:
        shutil.rmtree(temp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())

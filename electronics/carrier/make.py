#!/usr/bin/env python3
"""Builds the carrier board's schematic from circuit.py into electronics/carrier/kicad, and checks it.

    python electronics/carrier/make.py           build, and update kicad/ and REPORT.md
    python electronics/carrier/make.py --check   build into a temporary folder, and fail if they differ

It needs KiCad 10's command line. Set KICAD_CLI to it if this doesn't find it.
"""

import argparse
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
KICAD = HERE / "kicad"
REPORT = HERE / "REPORT.md"
CABLES = HERE.parent.parent / "cad" / "optima" / "outputs" / "cables.csv"

sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True

import checks  # noqa: E402
import schematic  # noqa: E402


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


def build(out):
    """Writes the KiCad files into `out`, runs KiCad, and returns the netlist and the ERC's result."""
    write(out / "carrier.kicad_sch", schematic.schematic())
    write(out / "carrier.kicad_sym", schematic.library())
    write(out / "carrier.kicad_pro", schematic.project())
    write(out / "sym-lib-table", schematic.SYM_TABLE)
    cli, sheet = find_kicad(), str(out / "carrier.kicad_sch")
    erc_path = out / "erc.json"
    for args in (["sch", "erc", "--format", "json", "--severity-all", "-o", str(erc_path), sheet],
                 ["sch", "export", "netlist", "-o", str(out / "carrier.net"), sheet],
                 ["sch", "export", "pdf", "-o", str(out / "carrier.pdf"), sheet]):
        done = subprocess.run([cli] + args, capture_output=True, text=True, cwd=out)
        if done.returncode:
            sys.exit("kicad-cli %s failed:\n%s%s" % (" ".join(args[:3]), done.stdout, done.stderr))
    erc = json.loads(erc_path.read_text(encoding="utf-8"))
    erc_path.unlink()
    (out / "carrier.kicad_prl").unlink(missing_ok=True)  # KiCad's own view settings
    netlist = clean_netlist((out / "carrier.net").read_text(encoding="utf-8"))
    write(out / "carrier.net", netlist)
    return netlist, erc


def report(results):
    lines = ["# The carrier board's checks", "", "Written by `make.py`. Don't edit.", "",
             "| Check | Result | Detail |", "|---|---|---|"]
    lines += ["| %s | %s | %s |" % (c.name, "pass" if c.ok else "FAIL", c.detail) for c in results]
    return "\n".join(lines) + "\n"


def differs(new, have):
    """The built files that aren't as committed. A PDF holds its date, so the schematic stands for it."""
    names = {p.name for p in new.iterdir()} | {p.name for p in have.iterdir()} if have.exists() else {"all"}
    return sorted(n for n in names if not n.endswith(".pdf") and not (
        (new / n).exists() and (have / n).exists() and (new / n).read_bytes() == (have / n).read_bytes()))


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if kicad/ isn't what this builds")
    args = parser.parse_args()
    temp = Path(tempfile.mkdtemp(prefix="carrier-"))
    try:
        netlist, erc = build(temp)
        results = checks.run(netlist, erc, CABLES)
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
            KICAD.mkdir(exist_ok=True)
            for path in temp.iterdir():
                shutil.copy(path, KICAD / path.name)
            write(REPORT, text)
        return 0 if all(c.ok for c in results) else 1
    finally:
        shutil.rmtree(temp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Builds OPTIMA's FreeCAD models into cad/optima/models, and their manufacturing outputs into
cad/optima/outputs, and checks them.

    python cad/optima/make.py           build, and update models/, its REPORT.md and outputs/
    python cad/optima/make.py --check   build into a temporary folder, and fail if they differ

FreeCAD 1.1 runs offscreen, in safe mode, so your own FreeCAD settings play no part. Set FREECAD
to FreeCAD's program if this doesn't find it. Any Python 3.8 or later runs this script.
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
import time
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODELS = HERE / "models"
OUTPUTS = HERE / "outputs"


def find_freecad():
    if os.environ.get("FREECAD"):
        return os.environ["FREECAD"]
    if sys.platform == "win32":
        found = sorted(glob.glob(r"C:\Program Files\FreeCAD *\bin\freecad.exe"))
        if found:
            return found[-1]
    if sys.platform == "darwin" and os.path.exists("/Applications/FreeCAD.app"):
        return "/Applications/FreeCAD.app/Contents/MacOS/FreeCAD"
    for name in ("freecad", "FreeCAD"):
        if shutil.which(name):
            return shutil.which(name)
    sys.exit("FreeCAD wasn't found. Install FreeCAD 1.1, or set FREECAD to its program.")


def build(freecad, out, outputs, result_path):
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", OPTIMA_SRC=str(HERE), OPTIMA_OUT=str(out),
               OPTIMA_OUTPUTS=str(outputs), OPTIMA_RESULT=str(result_path))
    command = [freecad, "--safe-mode", str(HERE / "freecad_main.py")]
    log = out.parent / "freecad.log"
    with open(log, "wb") as f:
        run = subprocess.Popen(command, env=env, stdout=f, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 900
        while run.poll() is None and not result_path.exists():
            if time.monotonic() > deadline:
                run.kill()
                sys.exit("FreeCAD didn't finish within 15 minutes.")
            time.sleep(0.5)
        # FreeCAD sometimes hangs on its way out, after it has written the result.
        try:
            run.wait(timeout=30)
        except subprocess.TimeoutExpired:
            run.kill()
            run.wait()
    if not result_path.exists():
        print(log.read_text(encoding="utf-8", errors="replace")[-8000:])
        sys.exit("FreeCAD stopped without writing a result.")
    return json.loads(result_path.read_text(encoding="utf-8"))


def report(result):
    version = ".".join(result["freecad"].split(".")[:2])
    lines = ["# OPTIMA CAD build report", "",
             "Written by `make.py` with FreeCAD %s. Don't edit it: the build replaces it." % version, ""]
    for model in result["models"]:
        lines += ["## %s" % model["name"], "", "| Check | Result | Detail |", "|---|---|---|"]
        for c in model["checks"]:
            result_text = "pass" if c["ok"] else "**FAIL**"
            lines.append("| %s | %s | %s |" % (c["name"], result_text, c["detail"]))
        lines += ["", "| Fact | Value |", "|---|---|"]
        lines += ["| %s | %s |" % (k, v) for k, v in model["facts"]] + [""]
    return "\n".join(lines)


def files(root):
    """The files under `root`, by their path from it."""
    if not root.exists():
        return {}
    return {p.relative_to(root).as_posix(): p for p in root.rglob("*") if p.is_file()}


def drawn(pdf):
    """What a PDF draws, and its text, but not exactly where: its streams, decompressed, with the
    page's top-level paint blocks sorted and their numbers masked. TechDraw paints a page's items,
    each in blocks that stand alone, in an order that changes from run to run, and places some text
    a millimetre apart. The STEP files hold the parts' sizes exactly."""
    out = []
    for m in re.finditer(rb"stream\r?\n(.*?)endstream", pdf, re.S):
        try:
            data = zlib.decompress(m.group(1))
        except zlib.error:
            out.append(m.group(1))
            continue
        depth, blocks, loose = 0, [], []
        for token in re.split(rb"\s+", data):
            token = b"#" if re.fullmatch(rb"-?\d*\.?\d+", token) else token
            if token == b"q":
                if not depth:
                    blocks.append([])
                depth += 1
            (blocks[-1] if depth else loose).append(token)
            if token == b"Q" and depth:  # a font's bytes can hold a stray Q
                depth -= 1
        out.append(b" ".join(loose) + b"|" + b"|".join(sorted(b" ".join(b) for b in blocks)))
    return sorted(out)


def same(a, b):
    """Whether two built files are the same: byte for byte, or for PDFs, in what they draw."""
    x, y = a.read_bytes(), b.read_bytes()
    return x == y or (a.suffix == ".pdf" and drawn(x) == drawn(y))


def changed(new, have):
    """The paths of the files that differ between two trees. A drawing differs if what it draws
    does, or if its part's STEP file does."""
    out = {n for n in new.keys() | have.keys()
           if n not in new or n not in have or not same(new[n], have[n])}
    parts = {Path(n).stem for n in out if n.endswith(".step")}
    return out | {n for n in new if n.endswith(".pdf") and Path(n).stem in parts}


def main():
    parser = argparse.ArgumentParser(description="Build and check OPTIMA's FreeCAD models.")
    parser.add_argument("--check", action="store_true",
                        help="fail if models/ or outputs/ isn't what the scripts build")
    args = parser.parse_args()

    temp = Path(tempfile.mkdtemp(prefix="optima-cad-"))
    out, outputs = temp / "models", temp / "outputs"
    out.mkdir()
    outputs.mkdir()
    result = build(find_freecad(), out, outputs, temp / "result.json")
    if result.get("error"):
        print(result["error"])
        return 1
    if not result["freecad"].startswith("1.1."):
        print("Warning: the models are built with FreeCAD 1.1; this is %s." % result["freecad"])
    with open(out / "REPORT.md", "w", encoding="utf-8", newline="\n") as f:
        f.write(report(result))

    failed = 0
    for model in result["models"]:
        print(model["name"])
        for c in model["checks"]:
            failed += not c["ok"]
            print("  %s  %s: %s" % ("ok  " if c["ok"] else "FAIL", c["name"], c["detail"]))

    trees = [(out, MODELS), (outputs, OUTPUTS)]
    if args.check:
        differ = []
        for built, target in trees:
            differ += ["%s/%s" % (target.name, n) for n in sorted(changed(files(built), files(target)))]
        if differ:
            print("Not what the scripts build: %s. Run make.py and commit models/ and outputs/."
                  % ", ".join(differ))
            failed += 1
        else:
            print("models/ and outputs/ are up to date.")
    else:
        for built, target in trees:
            new, have = files(built), files(target)
            update = changed(new, have)
            for n in update:
                if n not in new:
                    have[n].unlink()
                else:
                    (target / n).parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(new[n], target / n)
            print("Updated %d of the %d files in %s." % (len(update), len(new), target))
    shutil.rmtree(temp, ignore_errors=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

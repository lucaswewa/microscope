#!/usr/bin/env python3
"""Builds OPTIMA's FreeCAD models into cad/optima/models, and checks them.

    python cad/optima/make.py           build, and update models/ and its REPORT.md
    python cad/optima/make.py --check   build into a temporary folder, and fail if models/ differs

FreeCAD 1.1 runs offscreen, in safe mode, so your own FreeCAD settings play no part. Set FREECAD
to FreeCAD's program if this doesn't find it. Any Python 3.8 or later runs this script.
"""

import argparse
import glob
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODELS = HERE / "models"


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


def build(freecad, out, result_path):
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", OPTIMA_SRC=str(HERE), OPTIMA_OUT=str(out),
               OPTIMA_RESULT=str(result_path))
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


def main():
    parser = argparse.ArgumentParser(description="Build and check OPTIMA's FreeCAD models.")
    parser.add_argument("--check", action="store_true",
                        help="fail if models/ isn't what the scripts build")
    args = parser.parse_args()

    temp = Path(tempfile.mkdtemp(prefix="optima-cad-"))
    out = temp / "models"
    out.mkdir()
    result = build(find_freecad(), out, temp / "result.json")
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

    built = {p.name for p in out.iterdir()}
    if args.check:
        have = {p.name for p in MODELS.iterdir()} if MODELS.exists() else set()
        differ = sorted(n for n in built | have if n not in built or n not in have
                        or (out / n).read_bytes() != (MODELS / n).read_bytes())
        if differ:
            print("Not what the scripts build: %s. Run make.py and commit models/." % ", ".join(differ))
            failed += 1
        else:
            print("models/ is up to date.")
    else:
        MODELS.mkdir(exist_ok=True)
        for p in MODELS.iterdir():
            if p.name not in built:
                p.unlink()
        for name in built:
            shutil.copyfile(out / name, MODELS / name)
        print("Wrote %d files to %s." % (len(built), MODELS))
    shutil.rmtree(temp, ignore_errors=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

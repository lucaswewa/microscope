#!/usr/bin/env python3
"""Sends the motion controller's settings to grblHAL on the SKR Pico over USB, and checks them.

    python cad/optima/load_settings.py COM7            send them, then read them back
    python cad/optima/load_settings.py COM7 --verify   only read them back

It needs pyserial: pip install pyserial. It waits for each setting's reply, because grblHAL writes
settings to flash as they arrive.
"""

import argparse
import re
import sys
from pathlib import Path

FILE = Path(__file__).resolve().parent / "outputs" / "grblhal-settings.txt"
SETTING = re.compile(r"^\$(\d+)=(-?[\d.]+)$")
TRIES = 50  # reads of the port's timeout, for one reply


def parse(text):
    """The settings in a settings file: {number: value}, in file order. ; starts a comment."""
    found = {}
    for line in text.splitlines():
        match = SETTING.match(line.strip())
        if match:
            found[int(match[1])] = float(match[2])
    return found


def ask(port, line):
    """Sends a line, and returns the lines of its reply, the last being ok or error:n. Raises
    TimeoutError if grblHAL doesn't finish replying."""
    port.write((line + "\n").encode("ascii"))
    reply = []
    for _ in range(TRIES):
        got = port.readline().decode("ascii", "replace").strip()
        if got:
            reply.append(got)
        if got == "ok" or got.startswith("error"):
            return reply
    raise TimeoutError("no reply to %s" % line)


def run(port, wanted, verify=False):
    """Sends the settings, unless only verifying, and reads them back. Returns the problems found,
    as lines for the reader."""
    problems = []
    if not verify:
        for number, value in wanted.items():
            last = ask(port, "$%d=%g" % (number, value))[-1]
            if last != "ok":
                problems.append("$%d was refused: %s" % (number, last))
    read = parse("\n".join(ask(port, "$$")))
    for number, value in wanted.items():
        if number not in read or abs(read[number] - value) > 1e-3:
            problems.append("$%d is %s, not %g" % (number, read.get(number), value))
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("port", help="the board's serial port, such as COM7 or /dev/ttyACM0")
    parser.add_argument("--verify", action="store_true", help="only read the settings back")
    args = parser.parse_args()

    import serial  # only here, so that the rest can run without pyserial

    wanted = parse(FILE.read_text(encoding="utf-8"))
    with serial.Serial(args.port, 115200, timeout=0.2) as port:
        problems = run(port, wanted, args.verify)
    for problem in problems:
        print(problem)
    if problems:
        return 1
    print("%d settings match. If you just sent them, power the board off and on, then run this "
          "again with --verify." % len(wanted))
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""The board's checks: that KiCad reads back this circuit, that the interlock can't be bypassed on the
board, and that the pins, levels, currents and connectors are what the design asks for."""

import csv
import re
from collections import namedtuple

from circuit import BOXES, PARTS, nets

Check = namedtuple("Check", "name ok detail")

HIGH_5V = 5.0
PICO_MAX, PICO_HIGH = 3.3, 2.2  # V: the Pico's supply, and a safe logic high at its inputs
E9_PULLDOWN = 8200  # ohms: the most that holds an RP2350 input low, by its erratum E9
LDD_OFF = 0.8  # V at DIM: below this, Mean Well's driver is off
DIM_SOURCE = 0.0005  # A: assumed; Mean Well says only that an open DIM reads as on
PTC_HOLD = 0.1  # A
# The pins that have each function on an RP2350 (its function table).
FUNCTIONS = {"UART_TX": (0, 12, 16, 28), "SLIDE_UART": (1, 13, 17), "I2C_SDA": (0, 4, 8, 12, 16, 20, 28),
             "I2C_SCL": (1, 5, 9, 13, 17, 21), "MATRIX_SCK": (10, 14, 26), "MATRIX_TX": (11, 15, 27)}
GATES = {3: 2, 6: 5, 8: 9, 11: 12}  # the buffer's outputs, and their inputs


def ohms(value):
    """A resistor's value in ohms, from its text."""
    m = re.match(r"([\d.]+)(k?)", value)
    return float(m[1]) * (1000 if m[2] else 1)


def resistor(a, b):
    """The value of a resistor joining two nets, or None."""
    found = [p for p in PARTS if p.ref[0] == "R" and {a, b} == set(p.pins.values())]
    return ohms(found[0].value) if found else None


def pico_gpio():
    """{GPIO number: net}."""
    names = {p.number: p.name for s in BOXES["Pico 2"] for p in s}
    pico = next(p for p in PARTS if p.kind == "Pico 2")
    return {int(names[n][2:]): net for n, net in pico.pins.items() if names[n].startswith("GP")}


def read_netlist(text):
    """{net: set of (ref, pin number)}, from KiCad's netlist. Free pins' nets are left out."""
    found = {}
    for block in re.split(r"\(net\s+\(code ", text[text.index("(nets"):])[1:]:
        name = re.search(r'\(name "([^"]*)"', block)[1].lstrip("/")
        if not name.startswith("unconnected-"):
            nodes = re.findall(r'\(node\s+\(ref "([^"]+)"\)\s+\(pin "(\d+)"', block)
            found[name] = {(r, int(n)) for r, n in nodes}
    return found


def interlock():
    """What keeps the LEDs off with the door open, as a list of what's wrong."""
    wrong, all_nets, refs = [], nets(), {p.ref: p for p in PARTS}
    for dim in sorted(p.pins[22] for p in PARTS if p.kind == "LDD-1000H"):
        on_it = sorted(refs[r].kind for r, _ in all_nets[dim])
        if on_it != ["74AHCT125", "LDD-1000H", "R"]:
            wrong.append("%s joins %s" % (dim, on_it))
            continue
        buffer = next(refs[r] for r, _ in all_nets[dim] if refs[r].kind == "74AHCT125")
        gate_in = buffer.pins[GATES[next(n for n, net in buffer.pins.items() if net == dim)]]
        if buffer.pins[14] != "DOOR_5V":
            wrong.append("%s's buffer runs from %s" % (dim, buffer.pins[14]))
        if not resistor(dim, "GND") or DIM_SOURCE * resistor(dim, "GND") >= LDD_OFF:
            wrong.append("%s has no pull-down that holds it off" % dim)
        if not resistor(gate_in, "GND"):
            wrong.append("%s's input can float" % dim)
    # The door's rail is reached from 5 V only through the connector, and so through the switch.
    seen, frontier = {"+5V"}, ["+5V"]
    while frontier:
        net = frontier.pop()
        for p in PARTS:
            if len(p.pins) == 2 and p.ref[0] != "J" and net in p.pins.values():
                frontier += [n for n in p.pins.values() if n not in seen and n != "GND"]
                seen |= set(p.pins.values()) - {"GND"}
    if "DOOR_5V" in seen:
        wrong.append("the door's rail is joined to 5 V on the board")
    return wrong


def connectors(cables_path):
    """What's wrong between the board's connectors and the cable list, and how many were matched."""
    wrong, seen = [], set()
    jacks = {p.ref: p for p in PARTS if p.ref[0] == "J"}
    with open(cables_path, encoding="utf-8", newline="") as f:
        for row in list(csv.reader(f))[1:]:
            cable, kind, wires = row[0], row[7], row[6]
            for place, end in ((row[2], row[3]), (row[4], row[5])):
                if place != "carrier board" or kind == "Bought: USB":
                    continue
                jack = jacks.get(end.split(",")[0])
                if jack is None or not jack.value.startswith(cable):
                    wrong.append("%s ends at %r" % (cable, end))
                    continue
                seen.add(jack.ref)
                count = re.match(r"(\d+) x", wires)
                # The epi LEDs' bought cordsets have two more wires, which are cut back.
                if count and kind == "Made" and int(count[1]) != len(jack.pins):
                    wrong.append("%s has %s wires for %d ways" % (cable, count[1], len(jack.pins)))
    wrong += ["%s has no cable" % r for r, j in sorted(jacks.items()) if j.value[0] == "W" and r not in seen]
    return wrong, len(seen)


def run(netlist, erc, cables_path):
    drawn, wanted = read_netlist(netlist), nets()
    differ = sorted(n for n in set(drawn) | set(wanted) if drawn.get(n) != wanted.get(n))
    gpio = pico_gpio()
    used = [n for n in gpio.values() if not n.startswith("SPARE")]
    bad_pins = [n for n, pins in FUNCTIONS.items() if not any(g in pins and gpio[g] == n for g in gpio)]
    # 5 V signals: the supplies, the camera port's output and the buffers' outputs.
    buffers = [p for p in PARTS if p.kind == "74AHCT125"]
    high = {"+5V", "DOOR_5V", "DOOR_FEED", "PORT_AT_5V"} | {p.pins[y] for p in buffers for y in GATES}
    high &= set(gpio.values())
    levels = {}
    for top, net in (("DOOR_5V", "DOOR_SENSE"), ("PORT_AT_5V", "PORT_AT")):
        levels[net] = HIGH_5V * resistor(net, "GND") / (resistor(top, net) + resistor(net, "GND"))
    weak = [n for n in gpio.values() if (resistor(n, "GND") or 0) > E9_PULLDOWN]
    sense = resistor("DOOR_5V", "DOOR_SENSE") + resistor("DOOR_SENSE", "GND")
    door = 4 * HIGH_5V / (resistor("DIM1", "GND") or 1e6) + HIGH_5V / sense
    jacks_wrong, matched = connectors(cables_path)
    found = [v["description"] for sheet in erc["sheets"] for v in sheet["violations"]]
    lock_wrong = interlock()
    return [
        Check("KiCad's electrical rules check is clean", not found, "; ".join(found[:6]) or "no violations"),
        Check("KiCad reads back the circuit in circuit.py, net for net", not differ,
              ", ".join(differ[:8]) or "%d nets, %d parts" % (len(wanted), len(PARTS))),
        Check("With the door open, nothing on the board can power the LEDs' DIM lines", not lock_wrong,
              "; ".join(lock_wrong) or "each DIM has only its buffer and a pull-down; the buffer runs "
              "only from the door's rail; 5 V reaches that rail only through the switch"),
        Check("Every peripheral sits on a pin that has it, and %d of 26 pins are used" % len(used),
              not bad_pins and len(used) == 21, ", ".join(bad_pins) or "UART0, I2C0 and SPI1; 5 spares"),
        Check("No 5 V signal reaches a Pico pin, and the dividers read as logic highs",
              not high and all(PICO_HIGH <= v <= PICO_MAX for v in levels.values()),
              ", ".join(high) or ", ".join("%s %.2f V" % i for i in levels.items())),
        Check("Pull-downs on the Pico's pins hold them low despite the RP2350's erratum E9", not weak,
              ", ".join(weak) or "all at most %.1f kohm" % (E9_PULLDOWN / 1000)),
        Check("The door's PTC carries the DIM lines' current", door <= PTC_HOLD,
              "%.0f mA with all four on, of its %.0f mA" % (door * 1000, PTC_HOLD * 1000)),
        Check("Each connector matches its cable in the cable list", not jacks_wrong,
              "; ".join(jacks_wrong) or "%d connectors matched" % matched),
    ]

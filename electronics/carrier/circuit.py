"""The carrier board's circuit: its parts, and the nets that join their pins.

This is the source of the schematic. make.py draws it in KiCad, and checks that what KiCad reads
back is this circuit. Parts come from each maker's data; docs/misc/optima_carrier_board.md says why.
"""

from collections import defaultdict, namedtuple

Part = namedtuple("Part", "ref kind value footprint block pins")  # pins: {number: net}, None if free
Pin = namedtuple("Pin", "number name type")

IN, OUT, BI, POWER, SUPPLY, PASSIVE = "input", "output", "bidirectional", "power_in", "power_out", "passive"

R_FP = "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P2.54mm_Vertical"
C_FP = "Capacitor_THT:C_Disc_D4.7mm_W2.5mm_P5.00mm"
CP_FP = "Capacitor_THT:CP_Radial_D10.0mm_P5.00mm"


def xh(ways):
    return "Connector_JST:JST_XH_B%dB-XH-A_1x%02d_P2.50mm_Vertical" % (ways, ways)


def terminal(ways):
    return ("TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-%d-5.08_1x%02d_P5.08mm_Horizontal"
            % (ways, ways))


def side(*pins):
    return [Pin(*p) for p in pins]


def pico_side(numbers, names):
    kinds = {"GND": POWER, "AGND": POWER, "VSYS": POWER, "3V3": SUPPLY, "VBUS": PASSIVE,
             "3V3_EN": PASSIVE, "ADC_VREF": PASSIVE, "RUN": PASSIVE}
    return [Pin(n, name, kinds.get(name, BI)) for n, name in zip(numbers, names)]


# Each box's pins: its left side and its right side, in the order drawn.
BOXES = {
    "Pico 2": (
        pico_side(range(1, 21), "GP0 GP1 GND GP2 GP3 GP4 GP5 GND GP6 GP7 GP8 GP9 GND GP10 GP11 GP12 "
                                "GP13 GND GP14 GP15".split()),
        pico_side(range(40, 20, -1), "VBUS VSYS GND 3V3_EN 3V3 ADC_VREF GP28 AGND GP27 GP26 RUN GP22 "
                                     "GND GP21 GP20 GP19 GP18 GND GP17 GP16".split())),
    "74AHCT125": (
        side((1, "1OE", IN), (2, "1A", IN), (4, "2OE", IN), (5, "2A", IN), (10, "3OE", IN),
             (9, "3A", IN), (13, "4OE", IN), (12, "4A", IN), (7, "GND", POWER)),
        side((14, "VCC", POWER), (3, "1Y", "tri_state"), (6, "2Y", "tri_state"), (8, "3Y", "tri_state"),
             (11, "4Y", "tri_state"))),
    # Mean Well's table for the 300-1000 mA pin-style drivers: DIM is pin 22 on these, 21 above them.
    "LDD-1000H": (
        side((23, "+VIN", POWER), (24, "+VIN", POWER), (1, "-VIN", POWER), (2, "-VIN", POWER),
             (22, "DIM", IN)),
        side((13, "+VOUT", PASSIVE), (14, "+VOUT", PASSIVE), (11, "-VOUT", PASSIVE), (12, "-VOUT", PASSIVE))),
    # BTT's TMC2209 V1.3, in the socket's positions, numbered as KiCad's StepStick footprint. Position
    # 13 is a second UART leg, not joined by default; DIAG is on a side pin, outside the socket.
    "TMC2209 StepStick": (
        side((9, "EN", IN), (10, "MS1", IN), (11, "MS2", IN), (12, "PDN_UART", BI), (13, "UART2", PASSIVE),
             (14, "CLK", IN), (15, "STEP", IN), (16, "DIR", IN)),
        side((8, "VS", POWER), (7, "GND", POWER), (6, "A2", OUT), (5, "A1", OUT), (4, "B1", OUT),
             (3, "B2", OUT), (2, "VIO", POWER), (1, "GND", POWER))),
    # Pololu's page gives each power connection two pins; E05 takes their order from its drawing.
    "D36V50F5": (
        side((1, "VIN", POWER), (2, "VIN", POWER), (3, "EN", IN), (4, "GND", POWER), (5, "GND", POWER)),
        side((6, "VOUT", SUPPLY), (7, "VOUT", PASSIVE), (8, "PG", "open_collector"))),
    "ADS1115": (
        side((4, "AIN0", IN), (5, "AIN1", IN), (6, "AIN2", IN), (7, "AIN3", IN), (1, "ADDR", IN),
             (3, "GND", POWER)),
        side((8, "VDD", POWER), (9, "SDA", BI), (10, "SCL", IN), (2, "ALERT", "open_collector"))),
}

PARTS = []
COUNTS = defaultdict(int)
FLAGS = ("GND", "+24V", "VSYS", "DOOR_5V")  # supplies that reach their pins through passive parts


def add(prefix, kind, value, footprint, block, pins):
    COUNTS[prefix] += 1
    ref = "%s%d" % (prefix, COUNTS[prefix])
    PARTS.append(Part(ref, kind, value, footprint, block, pins))
    return ref


def r(value, a, b, block):
    return add("R", "R", value, R_FP, block, {1: a, 2: b})


def c(value, a, block, volts=None):
    if volts:
        return add("C", "CP", "%s %d V" % (value, volts), CP_FP, block, {1: a, 2: "GND"})
    return add("C", "C", value, C_FP, block, {1: a, 2: "GND"})


def connector(number, what, footprint, block, nets):
    """A connector with the number the cable list calls it by."""
    PARTS.append(Part("J%d" % number, "J%d" % len(nets), what, footprint, block, dict(enumerate(nets, 1))))


def buffer(block, vcc, gates):
    """A 74AHCT125, its gates always on. Each gate is (its input's net, its output's net), or None
    for a spare, whose input is held low and whose output is free."""
    pins = {14: vcc, 7: "GND"}
    for (oe, a, y), gate in zip(((1, 2, 3), (4, 5, 6), (10, 9, 8), (13, 12, 11)), gates):
        pins[oe] = "GND" if gate else vcc
        pins[a] = gate[0] if gate else "GND"
        pins[y] = gate[1] if gate else None
    return add("U", "74AHCT125", "74AHCT125", "Package_DIP:DIP-14_W7.62mm", block, pins)


# Power. A diode guards against a reversed 24 V; the buck feeds the matrix and the logic.
BLOCK = "Power"
connector(1, "W08: 24 V in, fuse F2", terminal(2), BLOCK, ["IN_24V", "GND"])
add("D", "D", "1N5822", "Diode_THT:D_DO-201AD_P15.24mm_Horizontal", BLOCK, {2: "IN_24V", 1: "+24V"})
c("100uF", "+24V", BLOCK, 35)
add("U", "D36V50F5", "D36V50F5, 5 V", "carrier:D36V50F5", BLOCK,
    {1: "+24V", 2: "+24V", 3: None, 4: "GND", 5: "GND", 6: "+5V", 7: "+5V", 8: None})
c("470uF", "+5V", BLOCK, 16)

# The controller. The diode lets USB and the board's 5 V both feed it. Spare pins go to a header.
BLOCK = "Pico 2"
SIGNALS = {"GP0": "UART_TX", "GP1": "SLIDE_UART", "GP2": "SLIDE_STEP", "GP3": "SLIDE_DIR",
           "GP4": "SLIDE_EN", "GP5": "SLIDE_HOME", "GP6": "DOOR_SENSE", "GP7": "SPARE_GP7",
           "GP8": "I2C_SDA", "GP9": "I2C_SCL", "GP10": "MATRIX_SCK", "GP11": "MATRIX_TX",
           "GP12": "LED1_EN", "GP13": "LED2_EN", "GP14": "LED3_EN", "GP15": "LED4_EN",
           "GP16": "TRIG1", "GP17": "EXPO1", "GP18": "TRIG2", "GP19": "EXPO2",
           "GP20": "PORT_SET_3V3", "GP21": "PORT_AT", "GP22": "SPARE_GP22", "GP26": "SPARE_GP26",
           "GP27": "SPARE_GP27", "GP28": "SPARE_GP28", "GND": "GND", "AGND": "GND", "VSYS": "VSYS",
           "3V3": "+3V3", "RUN": "RUN", "VBUS": None, "3V3_EN": None, "ADC_VREF": None}
add("U", "Pico 2", "Raspberry Pi Pico 2", "Module:RaspberryPi_Pico_Common_THT", BLOCK,
    {p.number: SIGNALS[p.name] for s in BOXES["Pico 2"] for p in s})
add("D", "D", "1N5817", "Diode_THT:D_DO-41_SOD81_P10.16mm_Horizontal", BLOCK, {2: "+5V", 1: "VSYS"})
connector(15, "Spare pins", "Connector_PinHeader_2.54mm:PinHeader_1x08_P2.54mm_Vertical", BLOCK,
          ["+3V3", "GND", "SPARE_GP28", "SPARE_GP7", "SPARE_GP27", "SPARE_GP26", "RUN", "SPARE_GP22"])

# The interlock (the electrical design's §4). The door's switch alone powers the DIM buffer, so with
# the door open or the cover off, every DIM line falls to its pull-down and every LED is off.
BLOCK = "Interlock and LED drivers"
connector(2, "W35: door switch", xh(2), BLOCK, ["DOOR_FEED", "DOOR_5V"])
add("F", "R", "PTC, 0.1 A hold", "Capacitor_THT:C_Disc_D7.5mm_W2.5mm_P5.00mm", BLOCK,
    {1: "+5V", 2: "DOOR_FEED"})
c("100nF", "DOOR_5V", BLOCK)
r("3.9k", "DOOR_5V", "DOOR_SENSE", BLOCK)
r("6.8k", "DOOR_SENSE", "GND", BLOCK)
# The third and fourth gates in this order, so that the tracks to them don't cross on the board.
buffer(BLOCK, "DOOR_5V", [("LED%d_EN" % n, "DIM%d" % n) for n in (1, 2, 4, 3)])
for n, wire in zip(range(1, 5), ("W23: 395 nm", "W24: 470 nm", "W25: 565 nm", "W26: 625 nm")):
    r("4.7k", "LED%d_EN" % n, "GND", BLOCK)
    r("1k", "DIM%d" % n, "GND", BLOCK)
    add("U", "LDD-1000H", "LDD-1000H", "carrier:LDD-H", BLOCK,
        {23: "+24V", 24: "+24V", 1: "GND", 2: "GND", 22: "DIM%d" % n, 13: "LED%d+" % n,
         14: "LED%d+" % n, 11: "LED%d-" % n, 12: "LED%d-" % n})
    connector(n + 2, wire, terminal(2), BLOCK, ["LED%d+" % n, "LED%d-" % n])

# The 5 V signals: the matrix's clock and data, and the camera port's position.
BLOCK = "LED matrix and camera port"
buffer(BLOCK, "+5V", [("MATRIX_SCK", "MATRIX_SCK_5V"), ("MATRIX_TX", "MATRIX_TX_5V"),
                      ("PORT_SET_3V3", "PORT_SET_5V"), None])
c("100nF", "+5V", BLOCK)
for gate_in, gate_out, line in (("MATRIX_SCK", "MATRIX_SCK_5V", "MATRIX_CLK"),
                                ("MATRIX_TX", "MATRIX_TX_5V", "MATRIX_DATA"),
                                ("PORT_SET_3V3", "PORT_SET_5V", "PORT_SET")):
    r("4.7k", gate_in, "GND", BLOCK)
    r("47", gate_out, line, BLOCK)
connector(7, "W30: LED matrix", terminal(4), BLOCK, ["+5V", "GND", "MATRIX_CLK", "MATRIX_DATA"])
connector(8, "W33: port, DIG I/O 1", xh(2), BLOCK, ["PORT_SET", "GND"])
connector(9, "W34: port, DIG I/O 2", xh(2), BLOCK, ["PORT_AT_5V", "GND"])
r("3.9k", "PORT_AT_5V", "PORT_AT", BLOCK)
r("6.8k", "PORT_AT", "GND", BLOCK)

# The slider: one UART wire, with the transmit side through a resistor as Trinamic's datasheet
# draws it. MS1 and MS2 low give it UART address 0. EN's pull-up keeps it off while the Pico starts.
BLOCK = "Filter slider"
add("U", "TMC2209 StepStick", "BTT TMC2209 V1.3", "Module:Pololu_Breakout-16_15.2x20.3mm", BLOCK,
    {9: "SLIDE_EN", 10: "GND", 11: "GND", 12: "SLIDE_UART", 13: None, 14: None, 15: "SLIDE_STEP",
     16: "SLIDE_DIR", 8: "+24V", 7: "GND", 6: "MOTOR_A2", 5: "MOTOR_A1", 4: "MOTOR_B1", 3: "MOTOR_B2",
     2: "+3V3", 1: "GND"})
c("100uF", "+24V", BLOCK, 35)
r("10k", "SLIDE_EN", "+3V3", BLOCK)
r("1k", "UART_TX", "SLIDE_UART", BLOCK)
connector(10, "W28: slider's motor", xh(4), BLOCK, ["MOTOR_A1", "MOTOR_A2", "MOTOR_B1", "MOTOR_B2"])
connector(11, "W29: slider's home switch", xh(2), BLOCK, ["HOME_SW", "GND"])
r("10k", "HOME_SW", "+3V3", BLOCK)
r("1k", "HOME_SW", "SLIDE_HOME", BLOCK)
c("100nF", "SLIDE_HOME", BLOCK)

# The cameras: an open-collector trigger, and an exposure input with a pull-up, each to a header.
BLOCK = "Cameras"
for n, wire in ((1, "W31: mono camera"), (2, "W32: colour camera")):
    r("4.7k", "TRIG%d" % n, "GND", BLOCK)
    r("2.2k", "TRIG%d" % n, "TRIG%d_BASE" % n, BLOCK)
    add("Q", "Q", "2N3904", "Package_TO_SOT_THT:TO-92_Inline_Wide", BLOCK,
        {1: "GND", 2: "TRIG%d_BASE" % n, 3: "TRIG%d_OUT" % n})
    connector(n + 11, wire, xh(4), BLOCK, ["TRIG%d_OUT" % n, "GND", "EXPO%d_IN" % n, "GND"])
    r("4.7k", "EXPO%d_IN" % n, "+3V3", BLOCK)
    r("1k", "EXPO%d_IN" % n, "EXPO%d" % n, BLOCK)

# The LEDs' temperatures: each thermistor divides 3.3 V against 10k, into the I2C ADC at 0x48.
BLOCK = "LED temperatures"
add("U", "ADS1115", "ADS1115", "Package_SO:TSSOP-10_3x3mm_P0.5mm", BLOCK,
    {4: "AIN1", 5: "AIN2", 6: "AIN3", 7: "AIN4", 1: "GND", 3: "GND", 8: "+3V3", 9: "I2C_SDA",
     10: "I2C_SCL", 2: None})
c("100nF", "+3V3", BLOCK)
r("4.7k", "I2C_SDA", "+3V3", BLOCK)
r("4.7k", "I2C_SCL", "+3V3", BLOCK)
connector(14, "W27: thermistors", xh(8), BLOCK, [net for n in range(1, 5) for net in ("TH%d" % n, "GND")])
for n in range(1, 5):
    r("10k 1%", "+3V3", "TH%d" % n, BLOCK)
    r("1k", "TH%d" % n, "AIN%d" % n, BLOCK)
    c("100nF", "AIN%d" % n, BLOCK)


def nets():
    """{net: set of (ref, pin number)}, from the parts."""
    found = defaultdict(set)
    for p in PARTS:
        for number, net in p.pins.items():
            if net:
                found[net].add((p.ref, number))
    return found

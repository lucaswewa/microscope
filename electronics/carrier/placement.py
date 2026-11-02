"""Where each part goes on the board: (x, y, angle) of its footprint's origin, in mm from the board's
top-left corner, with y downwards. Most footprints' origin is their pin 1.

The terminals' wire entries face off the board. Resistors stand upright, to save room.
"""

SIZE = (140.0, 100.0)
HOLES = [(4, 4), (136, 4), (4, 96), (136, 96)]  # M3, as the CAD's tray has them

PLACES = {}


def row(refs, x, y, dx=6.35, dy=0.0, angle=0):
    for i, ref in enumerate(refs):
        PLACES[ref] = (x + i * dx, y + i * dy, angle)


# The epi LEDs' drivers along the top, their outputs towards their terminals at the edge.
for n, left in enumerate((26, 49, 72, 95)):
    PLACES["U%d" % (4 + n)] = (left + 17.77, 41.97, 180)
    PLACES["J%d" % (3 + n)] = (left + 12.7, 6.0, 180)

# DIM's pull-downs: below the first two drivers, and by the buffer for the others.
row(["R4"], 30.5, 47.5)
row(["R6"], 53.5, 47.5)
row(["R8"], 99, 47.5)
row(["R10"], 109.5, 47)

# Power at the left: the input and its diode, the buck, then the matrix's terminal and its buffer.
PLACES["J1"] = (19.5, 6.0, 180)
PLACES["D1"] = (6, 28, 90)
row(["C1"], 15.5, 24, angle=270)
PLACES["U1"] = (3, 45.5, 0)
row(["C2"], 32, 57)
PLACES["J7"] = (5, 74.2, 270)
PLACES["U8"] = (17, 73, 0)
row(["C4"], 28, 73, angle=270)
row(["R11", "R13"], 31, 74, dx=0, dy=5.08)
PLACES["R15"] = (27.5, 85.7, 0)
PLACES["R12"] = (12.5, 78.5, 270)  # the matrix's, between the buffer and its terminal
PLACES["R14"] = (12.5, 85.5, 270)
PLACES["R16"] = (22, 91.6, 180)
PLACES["J8"] = (10.5, 96, 0)
PLACES["J9"] = (20, 96, 0)
row(["R17", "R18"], 28, 91)

# The Pico at the bottom's centre, its USB at the edge, and the diode to its VSYS.
PLACES["U2"] = (83.9, 97.7, 180)
PLACES["D2"] = (56.16, 53, 180)  # its anode to the left, towards 5 V
PLACES["J15"] = (61.5, 88, 180)  # each pin level with the Pico's pin it joins

# The cameras, to the left of the Pico.
PLACES["Q1"] = (43, 60, 0)
row(["R23", "R24", "R25", "R26"], 36, 66)
PLACES["Q2"] = (43, 72, 0)
row(["R27", "R28", "R29", "R30"], 38.5, 78, dx=5.4)
PLACES["J12"] = (36, 96, 0)
PLACES["J13"] = (50, 96, 0)

# The interlock beside the Pico's LED pins, with the door's connector and its parts.
PLACES["U3"] = (93, 58, 90)
row(["R3", "R5"], 92, 62)
PLACES["R7"] = (88.2, 53, 270)  # the enables' pull-downs that reach the top row, by the Pico
PLACES["R9"] = (88.2, 47.5, 0)
PLACES["F1"] = (41, 49, 0)  # by the buck: this keeps 5 V at the board's left, far from the door's rail
row(["C3"], 114, 55)
row(["R1", "R2"], 114, 60)
PLACES["J2"] = (128, 55, 0)

# The slider, to the right of the Pico, its motor and switch at the bottom edge.
PLACES["U9"] = (106, 92, 180)
row(["C5"], 111, 72)
row(["R20", "R19"], 88.4, 69)
PLACES["J10"] = (111, 96, 0)
PLACES["J11"] = (125, 96, 0)
row(["R21", "R22"], 111, 86)
row(["C6"], 124, 86)

# The temperatures in the free corner, the thermistors' connector on the right edge.
for n in range(4):
    row(["R%d" % (33 + 2 * n), "R%d" % (34 + 2 * n)], 118, 12 + 5.08 * n)
    row(["C%d" % (8 + n)], 125.5, 33 + 3.81 * n, angle=180)  # the inputs towards the ADC
PLACES["J14"] = (134, 12, 270)
PLACES["U10"] = (130, 39, 270)
row(["C7"], 129, 44, angle=270)
row(["R32", "R31"], 127.5, 68, dx=0, dy=3.2, angle=180)  # the I2C pull-ups, in the open, on the lines' way to the Pico

# Reference labels moved off other parts' outlines: their offsets from the footprint's origin.
LABELS = {"J%d" % n: (5.5, 2.5) for n in (1, 3, 4, 5, 6)}
LABELS.update({"R%d" % n: (4.7, 0) for n in (4, 6)})
LABELS.update({"C%d" % n: (-7.9, 0) for n in (8, 9, 10, 11)})
LABELS.update({"U8": (3.81, 7.62), "C4": (0, 8), "U10": (4.8, 0), "C7": (-3, 5), "R31": (4.7, 0), "R32": (4.7, 0), "R16": (-5.5, 0), "J9": (6.5, 0), "U3": (7.62, -3.8)})

# Tracks laid out here, so that the router works around them, as (net, layer, width, points). 5 V stays
# on the top layer, at the board's left. The matrix's 4 A returns from J7 to the buck's ground pins on
# the bottom, where no signal can cut it. The door's feed and the longest runs take clear paths first.
FIXED = [
    ("+5V", "F.Cu", 2.0, [(5, 74.2), (9.35, 69.85), (9.35, 64), (25, 64), (32, 57), (36, 53), (46, 53)]),
    ("+5V", "F.Cu", 2.0, [(38, 53), (41, 50), (41, 49)]),
    ("+5V", "F.Cu", 2.0, [(25, 64), (28, 67), (28, 73), (24.62, 73), (24.62, 75.54)]),
    ("GND", "B.Cu", 2.0, [(5, 79.28), (7, 79.28), (10.7, 75.58), (10.7, 72), (11.89, 70.81), (11.89, 67.09)]),
    ("GND", "B.Cu", 2.0, [(11.89, 69.63), (14.43, 69.63), (14.43, 67.09)]),
    ("DOOR_FEED", "F.Cu", 0.4, [(46, 49), (47.5, 47.5), (47.5, 34), (116, 34), (116, 50), (124, 50), (128, 54), (128, 55)]),
    ("VSYS", "B.Cu", 0.5, [(56.16, 53), (63.8, 60.64), (63.8, 92.84), (66.12, 95.16)]),
    ("DIM1", "B.Cu", 0.25, [(98.08, 58), (96.81, 56.73), (96.81, 46.5), (95.81, 45.5), (32, 45.5), (30.5, 47), (30.5, 47.5)]),
    ("DIM2", "F.Cu", 0.25, [(53.5, 47.5), (55.5, 45.5), (105.5, 45.5), (106.97, 46.97), (106.97, 56.73), (105.7, 58)]),
    ("+24V", "B.Cu", 1.0, [(6, 28), (16.97, 38.97), (16.97, 69.63)]),
    ("+24V", "B.Cu", 1.0, [(16.97, 39.43), (28.53, 39.43), (28.53, 41.97)]),
    ("+24V", "B.Cu", 1.0, [(28.53, 41.97), (30.26, 43.7), (49.8, 43.7), (51.53, 41.97)]),
    ("+24V", "F.Cu", 1.0, [(51.53, 41.97), (53.26, 43.7), (72.8, 43.7), (74.53, 41.97), (76.26, 43.7), (95.8, 43.7), (97.53, 41.97)]),
    ("+24V", "B.Cu", 1.0, [(97.53, 41.97), (99.26, 43.7), (113.6, 43.7), (113.6, 50.5), (112.4, 51.7), (112.4, 70), (111, 71.4), (111, 72)]),
    ("DIM2", "F.Cu", 0.25, [(53.5, 47.5), (49.5, 43.5), (49.5, 38), (50.61, 36.89), (51.53, 36.89)]),
    # The door's rail, south of its buffer. The ADC's two ground pins are too fine for GND's width.
    ("DOOR_5V", "F.Cu", 0.4, [(130.5, 55), (130.5, 57), (130, 57.5), (116.5, 57.5), (114, 55), (114, 60), (91.3, 60), (91.3, 52.08), (93, 50.38)]),
    ("GND", "F.Cu", 0.25, [(130, 36.85), (130, 35.7), (131, 35.7), (131, 36.85)]),
    ("GND", "F.Cu", 0.5, [(131, 35.7), (134, 32.7), (134, 29.5)]),
    ("I2C_SCL", "F.Cu", 0.25, [(131, 41.15), (131, 41.9), (133.4, 44.3), (133.4, 61), (127.5, 66.9), (127.5, 68)]),
    ("EXPO1_IN", "B.Cu", 0.25, [(48.7, 66), (46.81, 67.89), (46.81, 74), (47.87, 75.06), (47.87, 82), (41, 88.87), (41, 96)]),
]

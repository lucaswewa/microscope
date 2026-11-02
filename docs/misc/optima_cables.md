# OPTIMA cables

Every cable in the instrument, for phase E02 of the [electronics plan](optima_electronics_plan.md). The [electrical design](optima_electrical_design.md) sets the power, the fuses, the grounding and each board's inputs and outputs. This document gives each cable's two ends and its pins.

The cable list is [`cad/optima/outputs/cables.csv`](../../cad/optima/outputs/cables.csv). The CAD build writes it from [`cables.py`](../../cad/optima/optima_cad/cables.py). For each cable it gives:

- its ends and their connectors;
- its conductors;
- its route through the model, and the length to cut or buy;
- for power, its peak current, its fuse and its voltage drop.

Started 2026-11-01. "Assumed" marks what a datasheet or the part in hand still has to confirm.

## 1. Where they run

**The two trays.** E02 splits the electronics tray in two, both under the stage. The single carrier board replaces the CAD's two carriers (E01 §1).

| Tray | Where | Its parts |
|---|---|---|
| Power tray, OPTIMA-E08 | Left of the lift | The supply, its terminals toward the back. The fuse block. The 12 V and 15 V converters on a DIN rail. |
| Control tray, OPTIMA-E09 | Right of the lift | The SKR Pico, the carrier board (130 × 80 mm, assumed until E05), the USB hub |

**The mains inlet** sits on a bracket, OPTIMA-E10, from the base plate's back edge. Its face is in the cover's notch, which is now 40 mm high. So the mains cord plugs in through the notch, and the inlet's switch is reached through it. The earth stud is an M6 up through the base plate, beside the inlet.

**The routes.**

- **At the base.** A cable between two places runs across the base plate, in its ducts where they lead. A new duct runs behind the lift, from the power tray to the foot of the riser.
- **To the head.** The riser is the duct behind the head plate. A cable to the head runs up it and out through the slot by its deck:
  - the lower slot, for the changer, the slider and the epi LEDs;
  - the upper slot, for the cameras and the camera port.
- **Out of the cover.** Only the PC's USB cable and the mains cord leave, through the notch.
- **The cover's own wiring.** The fan and the door switch join the instrument at a plug on the cover's back rail, beside the notch.

**The lengths.** `cables.py` adds up each route's runs along X, Y and Z between those points. Where an end moves, it takes the route at its longest across that end's travel. Each length then gets:

- 100 mm for each end, to dress it to its connector;
- a 50 mm loop if an end moves;
- for the PC's cable, 1.5 m outside the cover (assumed).

A made cable's length is rounded up to 50 mm. A bought one gets the next length it's sold in.

**The checks.** The build checks that:

- each conductor's rating covers the current its fuse or its source allows;
- every power run drops at most 3 % at its peak current;
- every bought cable comes long enough, and the stages' own 1.6 m cables reach the SKR Pico across the stage's travel;
- the USB cables stay within 3 m for USB 3 and 5 m for USB 2.

They pass. The matrix's 5 V run drops the most, 2.0 %. The PC's cable needs 2.2 m, so it's a 3 m one. The [build report](../../cad/optima/models/REPORT.md#cables) has the details.

## 2. Making them

- **Labels.** Each cable carries its code, such as W12, on a heat-shrink marker at both ends.
- **Colours.**
  - Mains: brown for live, blue for neutral, green-and-yellow for earth (IEC 60445).
  - DC: red for positive and black for 0 V. The label gives the voltage.
- **Pairs.** Each motor coil's two wires, each camera trigger's, and each thermistor's are twisted together.
- **Moving parts.** The cables to the X and Y motors and to the LED matrix move with the stage and the lift. Each is clamped to the part it moves with and to the base, with its loop between them. Use highly flexible wire for the matrix's cable.
- **Mains.** Sleeve the crimps on the inlet and the supply. Keep the mains wires apart from the rest, between the inlet and the supply's terminals.
- **The earth stud.** Put a serrated washer under each ring terminal, so that it bites through the base plate's anodizing.

## 3. The harnesses

Pins marked "assumed" wait for the part in hand or its maker's sheet. Check each before crimping.

### Mains, earth and power: W01–W14

| Cable | From | To | Wires |
|---|---|---|---|
| W01 | Inlet: L and N tabs | Supply: L and N | Brown L, blue N |
| W02 | Inlet: earth tab | Earth stud | Green-and-yellow |
| W03 | Earth stud | Supply: earth terminal | Green-and-yellow |
| W04 | Earth stud | Column: under the head of a column screw | Green-and-yellow |
| W05 | Earth stud | Cover: a 6.3 mm tab on its frame | Green-and-yellow, on a push-on terminal, which comes off with the cover |
| W06 | Supply: +V and −V | Fuse block: input and 0 V | 1.0 mm² |
| W07 | Fuse block: F1 (3 A) and 0 V | SKR Pico: power input | |
| W08 | Fuse block: F2 (2 A) and 0 V | Carrier board: 24 V input | |
| W09 | Fuse block: F3 (4 A) and 0 V | 12 V converter: +Vin and −Vin | |
| W10 | Fuse block: F5 (2 A) and 0 V | 15 V converter: +Vin and −Vin | |
| W11 | Fuse block: F4 (0.5 A) and 0 V | Cover's plug: pins 1 (+) and 2 (0 V) | |
| W12 | 12 V converter: +Vo and −Vo | Mono camera: a DC plug like its own adapter's (assumed) | |
| W13 | 12 V converter: +Vo and −Vo | USB hub: a DC plug like its own adapter's (assumed) | |
| W14 | 15 V converter: +Vo and −Vo | Camera port: a DC plug like Thorlabs' supply's (assumed) | 0.5 mm² |

### The XY stage: W15, W16, W18 and W19

Each of the stage's motors has its own 1.6 m cable, ending in an HDB15 (M). It plugs into an adapter, W18 for X and W19 for Y, beside the SKR Pico. The routes need 0.69 m and 0.73 m, so about 0.9 m of each coils on the control tray. The pins come from Standa's pinout sheet for the 8MTF stages.

| HDB15 pin | The stage's signal | To the SKR Pico |
|---|---|---|
| 4 | Motor phase A | Motor header, coil 1 (1A) |
| 3 | Motor phase /A | Motor header, coil 1 (1B) |
| 2 | Motor phase B | Motor header, coil 2 (2A) |
| 1 | Motor phase /B | Motor header, coil 2 (2B) |
| 9 | Limit switch 1 | The STOP header's signal, joined with pin 8 |
| 8 | Limit switch 2 | The STOP header's signal |
| 7 | Logic ground | The STOP header's ground |
| 6 | The stage's EEPROM | Not connected |
| 5, 10–15 | Not used by this stage | Not connected |

The motor header's labels, 1A, 1B, 2A and 2B, are as BTT's pin diagram names them. Swapping a coil's two wires only reverses the motor, which E03's settings can undo. The stage's limit switches are mechanical, and close when pushed. So joining both ends on one input works: either one pulls it low.

### The Z lift: W17

The lift has a DB9 (M) on its body, not an HDB15, and Standa publishes no pinout for it. So its pins wait for the lift in hand, or for Standa. Its two limit switches are optocouplers, and open when pushed (Standa's table). Two such switches can't share an input: the one that isn't pushed still holds the line. So W17 brings these to the SKR Pico:

- the motor's four phases, to the Z motor header;
- limit switch 1 to Z-STOP;
- limit switch 2 to the PROBE header, which E03 picked ([the motion controller](optima_motion_controller.md#1-the-pin-map));
- the switches' supply and ground, if they need one, from the Z-STOP header's 5 V and ground.

That's 8 conductors of 0.34 mm².

### The changer and the slider: W20, W21, W28 and W29

| Cable | From | To |
|---|---|---|
| W20 | SKR Pico: E0 motor header | The changer's NEMA 11: its own plug or leads (assumed: a 4-wire bipolar motor) |
| W21 | SKR Pico: E0-STOP | The changer's home switch: common and normally closed |
| W28 | Carrier board: J10 | The slider's NEMA 11 |
| W29 | Carrier board: J11 | The slider's home switch: common and normally closed |

The home switches are wired normally closed, so a broken wire reads as pressed and stops the axis. E03 and E07 set the inputs' sense.

### The epi LEDs: W23–W27

W23–W26 are bought M8 cordsets, a 4-pin female M8 on a 2 m cable with open ends, 4 × 0.25 mm² (assumed). Thorlabs' mounted LEDs have a 4-pin male M8.

| M8 pin | The LED's (assumed) | To the carrier board |
|---|---|---|
| 1 | LED anode | The LED driver's output + |
| 2 | LED cathode | The LED driver's output − |
| 3, 4 | The LED's EEPROM | Not connected: cut back and insulated |

Check the pins against Thorlabs' sheet before the first LED is lit. W27 is an 8-core cable of four twisted pairs, one to the thermistor on each LED's heatsink. It starts at the carrier board's J14, and fans out at the epi arm.

### The LED matrix: W30

Four conductors of 0.75 mm², highly flexible: 5 V, 0 V, clock and data. The far end matches the matrix's own input plug (assumed: a 4-pin JST SM, as Adafruit's matrices have). At 3.8 A its run drops 2.0 %. Keep the clock at 4 MHz or less on this unshielded run (E07).

### The cameras' triggers: W31 and W32

Four conductors of 0.14 mm², as two twisted pairs:

- the trigger: the carrier's open-collector output, and its return;
- the exposure signal: the carrier's input, with its pull-up, and its return.

ToupTek doesn't publish the cameras' trigger pinouts, so the far ends wait for the cameras or ToupTek's manual (E01 §6).

### The camera port: W14, W33 and W34

The MFF102/M has two SMA connectors, DIG I/O 1 and DIG I/O 2, at TTL levels (Thorlabs' user guide). W33 and W34 are bought SMA (M) leads on RG174 with open ends: the centre carries the signal, the shield is the ground.

| Lead | The port's connector | Its mode | The carrier board |
|---|---|---|---|
| W33 | DIG I/O 1 | Input, *Goto Position*, *Logic Edge Input*: a rising edge moves to position 2, a falling edge to position 1 | An output: its level sets the port |
| W34 | DIG I/O 2 | Output, *At Position*, *Logic Level Output*: high at position 2, low at position 1 | An input. The carrier board's divider brings its 5 V to the Pico 2's 3.3 V. |

Set these modes once, with Thorlabs' software over USB, and save them in the unit with *Persist Settings to Hardware*. The port then needs no USB cable in use. This makes the port follow a level rather than toggle on each pulse, as E01 §5 had it, so the firmware always knows where it is.

The port runs on 15 V only, which E01 missed. E02 adds the 15 V converter, a Mean Well DDR-30G-15, behind fuse F5. W14 feeds the port through a DC plug like the one on Thorlabs' supply. Confirm that plug on the unit.

### The cover's plug: W11, W35, W41 and W42

The cover's fan and door switch join the instrument through one 4-way connector (assumed: Molex Mini-Fit Jr.), on the cover's back rail beside the notch. Its earth is W05's push-on terminal. To lift the cover off, unplug both.

| Pin | Instrument side | Cover side | Carries |
|---|---|---|---|
| 1 | W11, from F4 | W41, to the fan's red lead | +24 V |
| 2 | W11, 0 V | W41, to the fan's black lead | 0 V |
| 3 | W35, from the carrier board's J2, 5 V | W42, to the door switch | 5 V, to the switch |
| 4 | W35, to J2 and the interlock's buffer | W42, from the door switch | 5 V when the door is shut |

The door switch is normally open, and closed when the shut door presses it (E01 §4). With the cover off, pin 4 has no supply, so every epi LED is off.

### USB: W22 and W36–W40

All bought:

| Cable | From | To | Kind |
|---|---|---|---|
| W22 | SKR Pico: USB-C | Hub | USB 2, 0.5 m |
| W36 | Pico 2: micro-USB | Hub | USB 2, 0.3 m |
| W37 | Hub: upstream port | The PC, through the notch | USB 3, 3 m |
| W38 | Hub | Mono camera | USB 3, 1.5 m |
| W39 | Hub | Colour camera | USB 3, 1.5 m |
| W40 | Hub | Overview camera | USB 2, 1.5 m |

The cameras' sockets, and the hub's upstream one, set each cable's far connector. Check them on the parts.

## 4. What's still open

- **The lift's DB9 pinout**, and whether its optical switches need a supply: from the lift, or Standa.
- **The cameras' trigger connectors**, from ToupTek (E01 §6).
- **The plugs on the DC leads:** the mono camera's, the hub's and the camera port's. Each matches the part's own adapter.
- **The epi LEDs' M8 pins**, from Thorlabs' sheet.

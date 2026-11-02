# OPTIMA motion controller

How the SKR Pico runs OPTIMA's stage, lift and objective changer, for phase E03 of the [electronics plan](optima_electronics_plan.md). The board runs grblHAL, unchanged. This project keeps only its settings and these notes.

- The settings are in [`cad/optima/outputs/grblhal-settings.txt`](../../cad/optima/outputs/grblhal-settings.txt).
- The CAD build writes them from [`motion.py`](../../cad/optima/optima_cad/motion.py) and the `MOTION_` values in [`params.py`](../../cad/optima/optima_cad/params.py), and checks them.
- [`load_settings.py`](../../cad/optima/load_settings.py) sends them to the board and reads them back.

Started 2026-11-01. No part of this has run on hardware yet: [§6](#6-the-bench-test) is how to try it, and the phase isn't Done until the owner has. "Assumed" marks what the parts in hand still have to confirm.

## 1. The pin map

The pins come from grblHAL's board map for the SKR Pico 1.0 with four axes. The headers come from BTT's pin diagram.

| Axis | Drives | Motor header | Step, direction, enable pins | Limit header | Limit pin |
|---|---|---|---|---|---|
| X | The stage's X | X | 11, 10, 12 | X-STOP | 4 |
| Y | The stage's Y | Y | 6, 5, 7 | Y-STOP | 3 |
| Z | The lift | Z1 | 19, 28, 2 | Z-STOP | 25 |
| A | The objective changer | E | 14, 13, 15 | E0-STOP | 16 |
| None | The lift's upper limit | None | None | PROBE | 22 |

**The headers.**

- Each motor header's pins are 2B, 1B, 1A and 2A, left to right on BTT's diagram. Z2 is a second connector on Z1's driver, and stays empty.
- Each STOP header has three pins: signal, ground and 5 V. The schematic shows a 10 kΩ pull-up to 3.3 V on each signal, and a filter.
- PROBE has two pins: signal and ground. It reaches GPIO 22 through a three-pin jumper, marked P_S, IO22 and PROBE on the diagram.

**The jumpers.**

- **The four DIAG jumpers stay off.** BTT says a fitted one rules out an external endstop on that axis.
- **The probe's jumper joins IO22 and PROBE.**
- **BOOT stays off,** except while flashing.

**The lift's upper limit.** Its two switches can't share an input ([the cables](optima_cables.md#the-z-lift-w17)), and grblHAL's map for this board has no input for a second limit on Z. Giving it one would need a custom build, and this project uses grblHAL as it is. So:

- the lower switch, which homing uses, goes to Z-STOP, and is a true hard limit;
- the upper switch goes to PROBE, which grblHAL only reports: `Pn:P` in its status;
- the soft limit stops the lift 0.5 mm below the upper switch once it's homed, and the server stops if it ever sees `P`.

The objectives don't depend on this switch. The CAD model shows the design's samples still 1.5 mm below the objectives' fronts with the lift at its very top, and the soft limit stops it lower than that. A taller sample relies on the server's limit for that sample ([the design's §6.3](optima_hardware_design.md#63-objective-changer)), wherever the switch is.

## 2. The axes

| | X and Y | Z | A |
|---|---|---|---|
| Lead, mm | 0.5 | 1 | 2 (assumed: Tr6 × 2) |
| Microsteps | 1/16 | 1/32 | 1/16 |
| Steps per mm | 6400 | 6400 | 1600 |
| One step, µm | 0.156 | 0.156 | 0.625 |
| Speed, mm/s | 8 | 5 | 10 |
| Maker's top speed, mm/s | 10 | 20 | None |
| What 24 V can drive, mm/s (estimate) | 8.6 | 17 (assumed motor) | 34 (assumed motor) |
| Acceleration, mm/s² | 100 | 50 | 50 |
| Soft limit from zero, mm | 100 | 23.5 | 85 |

**Speed.** Standa's table gives the XY stage's motor (its "28") 6.8 Ω, 4.8 mH and 0.106 N m. From those, a 24 V supply can hold the motor's current up to about 8.6 mm/s, below the stage's 10 mm/s. Standa doesn't say what supply its figure assumes, and its own controller takes up to 48 V. So X and Y start at 8 mm/s:

- a 1.7 mm tile step takes 0.29 s, as the design expected;
- crossing a plate's wells takes 12.5 s, not the design's 10 s.

The estimate is rough, and above it the torque falls away rather than stopping at once. [§6](#6-the-bench-test) tries 10 mm/s.

**Lift and changer.** Standa doesn't publish the lift's motor. This design hasn't chosen the changer's, so the bill of materials now asks for a 2 mm lead and at least 0.67 A. Both are assumed to match the XY stage's motor. The lift starts at 5 mm/s, well inside its 20 mm/s, because focus moves are short.

**Where things are after homing (nominal).**

| Axis | Position |
|---|---|
| X and Y | The centre of their travel at 50 |
| Z | A slide in focus at about 19 |
| A | The 20x at 1, the 10x at 41, the 5x at 81 |

## 3. Currents

| | X, Y, Z and A |
|---|---|
| Run current | 470 mA RMS (`$140`–`$143`) |
| Its peak in a coil | 0.66 A, of the motor's rated 0.67 A |
| Hold current | 50 % for X, Y and Z, 20 % for A (`$210`–`$213`) |

- **No fan.** BTT advises a fan on the drivers above 0.8 A, and these run at 470 mA.
- **Heat.** The four motors dissipate about 12 W with all running, within the electrical design's 30 W. Heat near the sample is why the peak only meets the rating.
- **The sense resistors.** The schematic shows 0.11 Ω, which is what grblHAL assumes for a TMC2209.
- **The changer's motor switches off when idle.** `$37` keeps only X, Y and Z on, so 250 ms after a move the changer's motor stops fighting its detent. The design's §6.3 asked for a lower holding current there; off is simpler, and its lead screw doesn't back-drive.

## 4. Limits and homing

| Setting | Value | Why |
|---|---|---|
| `$5`, invert limits | X and Y | Their switches close when pushed. The lift's and the changer's open when pushed, which is grblHAL's default. |
| `$21`, hard limits | On | A switch stops the axis at once, with `ALARM:1`. |
| `$20`, soft limits | On | Moves beyond the limits are refused, with `ALARM:2`, once homed. |
| `$40`, jogs inside the limits | On | A jog stops at the limit instead of raising an alarm. |
| `$22`, homing | 95 | See below. |
| `$23`, homing direction | All negative | With `$22`'s origin option, positions run from 0 at the switch's pull-off. |
| `$44`–`$46`, homing's phases | Z, A, then X and Y | The lift goes down first, so the objectives clear the sample while the changer moves. |
| `$24`, `$25`, `$27` | 30 and 300 mm/min, 1 mm | Locate, seek and pull-off. The longest seek takes about 20 s. |

`$22` is the sum of six options:

- homing on (1);
- single-axis homing, such as `$HZ` (2);
- homing required at start (4);
- zero at the switch (8);
- two switches sharing an input (16);
- a soft reset lets `$X` unlock (64).

**Why the shared-input option matters.** X and Y each have one input for both ends, so a pressed switch could be either end. With this option, grblHAL refuses to home while any limit is pressed, instead of driving further into the far end. To get out, use the rescue in [§6, step 13](#6-the-bench-test). The server must never do that by itself: until an axis is homed, nothing limits it but its switches.

**What the server's driver has to do.** That's a later phase, but these come from here:

- home after every start, and treat `P` in `Pn` as the lift at its top: stop;
- lower the stage before any objective change;
- keep to the X and Y limits with `$J` or `G53`, in millimetres from the homed zero.

## 5. Building and flashing grblHAL

grblHAL is GPL, and the firmware file stays out of this repository.

1. Open the [Web Builder](https://svn.io-engineering.com:8443/?driver=RP2040). Choose the board **BTT SKR Pico 1.0** and **4 axes**. Keep the rest as it comes, with the probe input on, and generate the `.uf2` file.

   Or build it yourself from [grblHAL's RP2040 driver](https://github.com/grblHAL/RP2040) with the Pico SDK: enable `BOARD_BTT_SKR_PICO_10` in its `my_machine.h`, and set `N_AXIS` to 4 in its `grbl/config.h`.
2. With the power off, put a jumper on the BOOT pins.
3. Connect USB-C to the PC, and press Reset. A drive called `RPI-RP2` appears. (BTT's manual spells it `RPI-PR2`.)
4. Copy the `.uf2` file onto it. The board restarts by itself.
5. Take the jumper off, and press Reset.

## 6. The bench test

This is how to try the phase. You'll need a multimeter, a dial gauge reading 0.01 mm or a good ruler, and a serial terminal. pyserial includes one:

```sh
pip install pyserial
python -m serial.tools.miniterm COM7 115200 --eol LF -e
```

Exit it with Ctrl-] before running `load_settings.py`. Wire only with the power off, as BTT says.

**The board alone: USB and 24 V, nothing else connected.**

1. **Flash** as in [§5](#5-building-and-flashing-grblhal). `$I` should name the board and show `[AXS:4:XYZA]`.
2. **Load.** Run `python cad/optima/load_settings.py COM7`, power the board off and on, then run it again with `--verify`. It should say every setting matches.
3. **Pins.** `$pins` should list what [§1](#1-the-pin-map) lists.
4. **Drivers.** `M122` should show all four drivers, each at about 470 mA. If one is missing, send `M122 I`: a driver takes no settings until it has motor power.

**The motors, one axis at a time.** Start with the XY stage's.

5. **The coils.** With the power off, measure across HDB15 pins 4–3 and 2–1: about 6.8 Ω, ±15 %.
6. **The lift and the changer's labels.** Read each motor's rated current. For the lift, Standa's page gives none, so look in its papers for its current and its DB9 pinout: Standa's page for another stage, the 8MT167, says units come with their motors' data and connection schemes. Set `$142` or `$143` to no more than the rating ÷ 1.41.
7. **The lift's outputs, before they reach the board.** The RP2040's pins take at most 3.3 V, and the schematic doesn't clearly show a buffer on PROBE as it does on the STOP inputs. With the switches powered, check that neither limit line rises above 3.3 V when its switch is pushed.
8. **Direction.** Press Ctrl-X, send `$X`, then jog: `$J=G91 G21 X1 F60`. Positive should move each axis as below. If one doesn't, add its letter to `MOTION_DIR_INVERT` and reload, or for now set `$3`.

   | Axis | Positive moves |
   |---|---|
   | X | The stage to the right |
   | Y | The stage to the back |
   | Z | The stage up |
   | A | The changer's carriage to the right, away from its switch |

9. **Distance.** Jog 10 mm, and measure: within 0.05 mm on X, Y and Z, and 0.1 mm on A.
10. **The switches.** Jog towards each end in steps of 1 mm or less, sending `?` between them.
    - **X, Y, the lift's bottom:** `Pn:` should show the axis's letter, and grblHAL should stop with `ALARM:1`. Press Ctrl-X, send `$X`, and jog back.
    - **The lift's top:** `Pn:P` should show with no alarm. Stop there.
    - **The changer:** press its switch by hand. `Pn:A` should show when pressed, and should show too if you unplug it.
    - **Each axis:** note `MPos` at each end. If the distance between the switches differs from `MOTION_AXES` by more than 0.5 mm, change it there.

**Homing and limits.**

11. **Each axis alone.** From mid-travel: `$HZ`, `$HA`, `$HX`, `$HY`. Each should find its switch, back off 1 mm and read 0.
12. **All together.** Power off and on: grblHAL should report that it needs homing. `$H` should move the lift, then the changer, then X and Y, and finish `Idle` at `MPos:0,0,0,0`. Do this five times, reading the dial gauge at the same position after each, and record the spread.
13. **On a far switch.** Jog X to its far end, onto the switch, and power off and on. `$H` should give `ALARM:1` and move nothing. The rescue: Ctrl-X, `$X`, jog off the switch, then `$H`.
14. **Soft limits.** After homing, `G53 G0 X101` should give `ALARM:2` and not move, and `$J=G53 G21 X200 F240` should stop at 100. Do the same for Y, for Z at 23.5 and for A at 85.

**The changer.**

15. **The detents.** `G53 G0 A41`, then A81, then A1. The ball should drop into its groove at each, and the carriage should settle by itself once the motor switches off. Record the A at which each one seats. Its home switch must trip beyond the first detent: if `$HA` stops short of it, adjust the switch's lever.

**Speed and heat, with the whole stack on the lift.**

16. **Repeat moves.** Move each axis end to end ten times at full speed, then return to the first position. The dial gauge should read the same within a few micrometres, with no alarm and no sound of missed steps. For the lift, go down and up across its range.
17. **Heat.** Run the moves for ten minutes. `M911` should report no pre-warning, and the motors should stay cool enough to keep a hand on.
18. **More speed.** Set `$110=600` and `$111=600`, 10 mm/s, and repeat step 16. If it runs cleanly with room to spare, change `MOTION_AXES`. Otherwise stay at 8.

**Record the results.** Put what you found back into `params.py` and [the cables](optima_cables.md), in a pull request: the motors' currents, the lift's DB9 pinout, the directions, the switches' distances and the changer's positions. Then run the build, and load the rebuilt settings.

## 7. What's still open

- **The lift's motor and its DB9 pinout**, from the lift's papers or Standa.
- **The changer's motor**, to be chosen: a NEMA 11 with a Tr6 × 2 screw and at least 0.67 A a phase.
- **The chopper mode.** grblHAL's default is SpreadCycle. StealthChop is quieter and may suit a microscope better, but it's untested here, and grblHAL doesn't store the `M569` that sets it. Try it in the bench test if the motors' noise or vibration shows.
- **The XY limit switches' wiring.** They close when pushed, so a broken wire reads as not pressed. The soft limits still protect a homed axis, and the stage's cable is its own, so this stays as it is.

## 8. Sources

- [grblHAL's board map for the SKR Pico](https://github.com/grblHAL/RP2040/blob/master/boards/btt_skr_pico_10_map.h), [its settings](https://github.com/grblHAL/core/blob/master/settings.h), [homing](https://github.com/grblHAL/core/blob/master/machine_limits.c) and the [Trinamic plugin's settings](https://github.com/grblHAL/Plugins_motor), read on 2026-11-01.
- BTT's [SKR Pico repository](https://github.com/bigtreetech/SKR-Pico), with its pin diagram, schematic and manual, and its [wiki page](https://global.bttwiki.com/SKR%20Pico.html).
- Standa's [stepper motor table](https://www.standa.lt/products/catalog/motorised_positioners?item=346&prod=ordering_info), [8MTF stages](https://www.standa.lt/products/catalog/motorised_positioners?item=311), [8MVT100-25-1 lift](https://www.standa.lt/products/catalog/motorised_positioners?item=407), [8MT167 stage](https://www.standa.lt/products/catalog/translation_rotation?item=64) and [8SMC5-USB's specifications](https://doc.xisupport.com/en/8smc5-usb/8SMCn-USB/About/Table_of_specifications.html).

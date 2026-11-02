# OPTIMA optics controller: the serial protocol

How a client, by hand or from the server, talks to the optics controller: the Pico 2 on the [carrier board](optima_carrier_board.md). This is version 1, from phases E06 and E07 of the [electronics plan](optima_electronics_plan.md). [ADR-0050](../adr/0050-the-optics-serial-protocol.md) says why it's shaped like this, [ADR-0051](../adr/0051-the-optics-sequences.md) why sequences work as they do, and [ADR-0052](../adr/0052-the-filter-sliders-drive.md) how the slider is driven.

The firmware that speaks it hasn't run on a chip yet. Its logic is tested on a PC, and [its README](../../firmware/optics/README.md) has the steps for trying it on a Pico 2.

## 1. The connection

- **USB CDC serial.** It appears as a serial port, such as `COM5` or `/dev/ttyACM0`. The baud rate doesn't matter.
- **Lines of printable ASCII,** up to 120 bytes, each ended by a line feed. A carriage return before the line feed is ignored, and so are empty lines.
- **Words are separated by spaces,** and they're case-sensitive.
- **One command at a time.** Each command gets one reply. Wait for it before sending the next.

Lines that the firmware sends start with one of these:

| Start | Meaning |
|---|---|
| `ok` | The command worked. |
| `err <code>` | It didn't, and nothing changed. |
| `!` | An event, which can arrive at any time, even between a command and its reply. |

## 2. Commands

Channels are numbered as the cables are: 1 is 395 nm, 2 is 470 nm, 3 is 565 nm and 4 is 625 nm. Camera 1 is the mono camera, and camera 2 the colour one.

| Command | Reply | What it does |
|---|---|---|
| `version` | `ok optima-optics protocol=1 firmware=0.2.0` | Names the protocol's and the firmware's versions. |
| `status` | `ok door=shut leds=1000 triggers=01 exposing=00 exposures=12,0 dropped=0` | See below. |
| `led <1-4> on` | `ok`, `err door-open` or `err busy` | Lights a channel, if the door counts as shut. |
| `led <1-4> off` | `ok` or `err busy` | Puts out a channel. |
| `led all off` | `ok` | Puts out every channel, and stops a running sequence with its light. It always works. |
| `trigger <1-2> <width>` | `ok`, or `err busy` | Pulses a camera's trigger for `<width>` µs, from 1 to 1,000,000. A camera's pulse must end before its next one, and a sequence that triggers the camera keeps the pins to itself. |

`status`'s fields:

| Field | Meaning |
|---|---|
| `door` | `shut` or `open`, as the firmware counts it (§4) |
| `leds` | Channels 1 to 4, `1` for lit |
| `triggers` | Cameras 1 and 2, `1` during a pulse |
| `exposing` | Cameras 1 and 2, `1` during an exposure |
| `exposures` | How many exposures each camera has started since power-up |
| `dropped` | How many lines the firmware couldn't queue because the client wasn't reading |

Error codes:

| Code | Meaning |
|---|---|
| `unknown-command` | No such command |
| `bad-argument` | A known command, but with wrong, missing or extra arguments, or a number out of range |
| `door-open` | The door doesn't count as shut |
| `busy` | That camera's pulse hasn't ended, the slider or the camera port is moving, or a sequence is running |
| `bad-line` | The line was longer than 120 bytes, or wasn't printable ASCII |
| `not-homed` | The slider must home before it can move to a cell |
| `no-steps` | A sequence has no steps to run |
| `full` | A sequence has its 64 steps |

### The LED matrix

| Command | Reply | What it does |
|---|---|---|
| `matrix <red> <green> <blue>` | `ok` or `err busy` | Shows a pattern, about half a millisecond later. |
| `matrix off` | `ok` or `err busy` | Puts out every LED. |
| `matrix status` | `ok red=0000000000000000 green=0000001818000000 blue=0000000000000000` | The pattern showing now. |

Each mask has up to 16 hex digits, a bit per LED: row 1 is the top byte, and column 1 is the top bit of its byte. So `matrix 0 0000001818000000 0` lights the centre 2 × 2 green, and `matrix 0 f0f0f0f0f0f0f0f0 0` the left half.

Each colour of each LED is fully on or fully off, because the design doesn't dim during an exposure ([§5.1](optima_hardware_design.md#51-transmitted-light-the-led-matrix)). So the dose is set by the exposure time.

The door doesn't affect the matrix, as [the electrical design](optima_electrical_design.md) says. Losing USB puts it out, and so does the firmware starting up, since the panel keeps its last pattern through a reset.

### The camera port

| Command | Reply | What it does |
|---|---|---|
| `port <1-2>` | `ok` or `err busy` | Sends the light to that camera. `! port <1-2>` follows when it's there, even if it never had to move. |
| `port status` | `ok port=1 state=ready` | Where the port last reported itself, and whether it's `ready`, `moving` or at `fault`. |

A flip that doesn't report arriving within 3 s gives `! port fault`, until the next `port` command. At power-up, the firmware sets its level to match where the mirror is, so the mirror stays put. A reset of the chip lets the level fall, and that can flip the mirror, so read `port status` after reconnecting. The port's own settings are in [the cables](optima_cables.md#the-camera-port-w14-w33-and-w34).

### The filter slider

| Command | Reply | What it does |
|---|---|---|
| `slider home` | `ok` or `err busy` | Sets up the driver, then finds the home switch. `! slider homed` follows. |
| `slider <a, b or c>` | `ok`, `err not-homed` or `err busy` | Moves to a cell: (a) the quad cube, (b) open or (c) the 50:50 plate. `! slider <cell>` follows, even if it never had to move. |
| `slider stop` | `ok` | Stops at once. If it was moving, it must home again. |
| `slider status` | `ok state=idle homed=1 cell=a fault=none` | `state` is `idle`, `homing` or `moving`. `fault` is the last fault, until the next `slider home` or move. |

**Homing.** If the switch is already pressed, the slider leaves it and goes 1 mm further. Then it moves toward the switch at 5 mm/s, for up to 90 mm, which takes up to 18 s, and stops dead when the switch trips. Position 0 is there. The cells are nominally at 1 mm (c), 41 mm (b) and 81 mm (a).

**Moves.** A move runs at up to 10 mm/s with 50 mm/s² either end, so the 40 mm between cells takes about 4.2 s. The motor switches off 250 ms after each move. Losing USB lets a move finish.

**Faults,** as `! slider fault <why>`. Each leaves the motor off, and homing needed again:

| Why | Meaning |
|---|---|
| `driver` | The driver didn't answer over its UART, or missed a write. It needs its 24 V for that. |
| `stuck` | The switch didn't release within 5 mm. The wire may be broken: the switch is normally closed, so that reads as pressed. |
| `no-home` | The switch didn't trip within 90 mm. The motor may be turning the wrong way. |
| `home-hit` | The switch tripped on the way to a cell, so the slider wasn't where the firmware thought. |

### Sequences

A sequence is a list of light states, stepped by a camera's exposure signal, with no round trip to the PC between frames ([§5.4](optima_hardware_design.md#54-timing-with-a-rolling-shutter)).

| Command | Reply | What it does |
|---|---|---|
| `seq add <leds> <red> <green> <blue>` | `ok`, `err full` or `err busy` | Adds a step: a digit per epi LED channel, such as `1000`, and the matrix's three masks. |
| `seq clear` | `ok` or `err busy` | Forgets the steps. |
| `seq start <camera> <cycles> <delay> <trigger>` | `ok`, `err no-steps`, `err door-open` or `err busy` | Runs the steps. See below. |
| `seq stop` | `ok` | Stops a running sequence, and puts out its light. |
| `seq status` | `ok steps=4 state=running camera=1 step=2 cycle=3` | How many steps there are, and how far a run has got. `state=idle` has only the steps. |

`seq start`'s arguments:

| Argument | Meaning |
|---|---|
| `<camera>` | The camera whose exposure signal steps it, 1 or 2 |
| `<cycles>` | How many times through the steps. 0 runs until stopped. |
| `<delay>` | µs from an exposure's end to the next step, up to 1,000,000. It gives the last rows time to read out. |
| `<trigger>` | 0 leaves the camera to run by itself, or be triggered by the client. Otherwise, the firmware pulses the camera's trigger for this many µs, up to 1,000,000, 2 ms after each step's light comes on. |

For example, four fluorescence channels, ten times, on the mono camera, with the firmware triggering it and 120 ms for its readout:

```text
seq clear
seq add 1000 0 0 0
seq add 0100 0 0 0
seq add 0010 0 0 0
seq add 0001 0 0 0
seq start 1 10 120000 1000
```

How it runs:

- **It starts at once,** replacing whatever is lit, and ends with every LED and the matrix off, however it ends.
- **It starts only between frames.** `seq start` gets `err busy` if that camera is exposing or being pulsed, or if the slider or the port is moving. So start the sequence before the camera.
- **While it runs, it owns the light path,** whose light must stay constant through a frame. `led`, `matrix`, `port`, `slider home`, moving the slider and changing the steps all get `err busy`. `led all off` and `seq stop` stop it.
- **`! seq step <n> <time>` says step `<n>` has lit.** The camera's next exposure is the one it lights.
- **An exposure's end, while waiting for the delay, changes nothing.**
- **Losing USB stops it,** with its light.

## 3. Events

| Event | Meaning |
|---|---|
| `! door open` | The door opened, and every channel is now off. |
| `! door shut` | The door counts as shut. Channels stay off until a command lights them. |
| `! exposure <1-2> start <time>` | A camera's exposure signal began. |
| `! exposure <1-2> end <time>` | It ended. |
| `! port <1-2>` | The camera port reports it's there. |
| `! port fault` | It didn't report arriving in time. |
| `! slider homed` | The slider found its switch, and can move to cells. |
| `! slider <a, b or c>` | The slider reached that cell. |
| `! slider fault <why>` | The slider stopped. See its section. |
| `! seq step <n> <time>` | A sequence's step `<n>`, counted from 1, has just lit. |
| `! seq done` | A sequence finished its cycles, and its light is off. |
| `! seq stopped` | The door stopped a sequence, and its light is off. |

`<time>` is in µs since power-up, read when the firmware noticed the edge, so it's a little after the edge itself. The firmware may send `! door shut` before a client connects, so use `status` to find the door's state.

The firmware holds up to 16 lines for a client that isn't reading, and after that it drops new ones and counts them. A client should read all the time, and treat a rising `dropped` as lost events. A dropped reply looks like a command with no answer.

## 4. The interlock

The door's switch cuts the LED drivers' enables in hardware, whatever the firmware does ([the carrier board](optima_carrier_board.md#4-how-the-circuits-work)). The firmware adds its own rules, so that its state matches the hardware's:

- **It starts with the door counted as open,** and with everything off.
- **Opening counts at once.** Any low reading of the door's signal puts out every channel, and sends `! door open`.
- **Shutting counts after 50 ms.** The signal must stay high that long, so a bouncing switch can't count as shut.
- **Nothing relights by itself.** After the door shuts, a client must light each channel again.
- **Sequences obey the door too.** One with an epi LED in any step won't start with the door open, and the door opening stops it, with `! seq stopped`. One that uses only the matrix doesn't depend on the door.
- **Losing USB puts out every channel,** since nothing could then switch them off. Closing the serial port doesn't, because USB stays connected.
- **A stalled firmware resets the chip,** after 500 ms, and so does a panic. The board's pull-downs hold the enables low meanwhile.

The door is read on every change, and at least every 10 ms. USB can't delay it, however slowly a client reads.

## 5. The pins

These are the carrier board's, from [its pin table](optima_carrier_board.md#2-the-picos-pins):

| Signal | GPIO | Level |
|---|---|---|
| LED channels 1 to 4 | 12, 13, 14, 15 | High to light. Through U3, which the door's switch powers. |
| Door | 6 | High when the door is shut: 3.18 V through a divider |
| Triggers 1 and 2 | 16, 18 | High during a pulse. A transistor then pulls the camera's input low. |
| Exposure 1 and 2 | 17, 19 | Low counts as exposing (assumed) |
| Matrix clock, data | 10, 11 | SPI1 at 4 MHz, through U8 to 5 V |
| Camera port's level | 20 | High for position 2, through U8 to 5 V |
| Camera port's report | 21 | High at position 2 |
| Driver's UART | 0, 1 | UART0 at 115,200 baud, on one wire |
| Slider's step, direction, enable | 2, 3, 4 | Enable is low to run |
| Slider's home switch | 5 | High when pressed |

## 6. What's assumed

Each of these is a constant in `optics-core`, to correct when the part is in hand.

| What | Assumed | Where it's set |
|---|---|---|
| The exposure signal's sense | ToupTek publishes no pinout, so a camera's output is taken to conduct during an exposure, which pulls the input low. If not, `start` and `end` swap, and `exposures` counts ends. | `Inputs::exposure_low` |
| What the exposure signal covers | That its end comes before the last rows finish reading out, which is why `<delay>` exists. The bench should measure it. | The client's `<delay>` |
| How long a step's light takes to settle | 2 ms, for the matrix's frame and the LED drivers | `LIGHT_SETTLE_US` |
| The panel's layout | Serpentine rows, as Pololu says of its 2532. That row 1, column 1 is the chain's first LED isn't stated, and nothing confirms the Adafruit 2734's layout at all. | `matrix.rs`'s `chain_bit` |
| The slider's lead | 2 mm, as for the objective changer | `STEPS_PER_MM` |
| The slider's direction | A high DIR moves away from the switch, which depends on the motor's wiring | `AWAY_IS_HIGH` |
| The cells' positions | From the CAD: the switch 1 mm past (c), then 40 mm apart. The design needs them within about ±3 mm. | `CELLS` |
| The slider's run current | 459 mA RMS, as the changer's matching motor, with no motor chosen | `tmc2209.rs` |
| A flip's longest time | 3 s | `PORT_TIMEOUT_US` |

**The matrix's heat.** The firmware doesn't limit how many LEDs light. Pololu says its 2532 can overheat well below its 3.8 A full power, and advises dimming to keep under 1.5 A, which this design can't do. At about 20 mA per colour per LED, the design's white brightfield at 20x, 44 LEDs, draws about 2.6 A. The panel's plate may be enough, but check its temperature on the bench, up to Pololu's limit of 70 °C. Green alone, as DPC uses, is a third of that.

## 7. Where the driver's facts come from

Analog Devices' [TMC-API](https://github.com/analogdevicesinc/TMC-API/tree/master/tmc/ic/TMC2209) gives the TMC2209's datagrams, CRC, registers and fields. BTT's [schematic](https://github.com/bigtreetech/BIGTREETECH-Stepper-Motor-Driver/blob/master/TMC2209/V1.3/Schematic/TMC2209%20V1.3-SCH.pdf) gives the 0.11 Ω sense resistors. Analog's datasheet wouldn't load, so the current formula and its 0.18 V range are from the [TMCStepper](https://github.com/teemuatlut/TMCStepper) library's notes, and the bench should measure the current. Pololu's [2532 page](https://www.pololu.com/product/2532) gives the APA102's frames, its colour order and its clock's sense.

## 8. What isn't here yet

The LEDs' temperatures, which a later phase can add without breaking this protocol. `protocol=` changes only if a change could break a client.

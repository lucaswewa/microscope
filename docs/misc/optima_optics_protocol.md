# OPTIMA optics controller: the serial protocol

How a client, by hand or from the server, talks to the optics controller: the Pico 2 on the [carrier board](optima_carrier_board.md). This is version 1, for phase E06 of the [electronics plan](optima_electronics_plan.md). E07 adds the matrix, the filter slider, the camera port and sequences. [ADR-0050](../adr/0050-the-optics-serial-protocol.md) says why it's shaped like this.

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
| `version` | `ok optima-optics protocol=1 firmware=0.1.0` | Names the protocol's and the firmware's versions. |
| `status` | `ok door=shut leds=1000 triggers=01 exposing=00 exposures=12,0 dropped=0` | See below. |
| `led <1-4> on` | `ok`, or `err door-open` | Lights a channel, if the door counts as shut. |
| `led <1-4> off` | `ok` | Puts out a channel. |
| `led all off` | `ok` | Puts out every channel. |
| `trigger <1-2> <width>` | `ok`, or `err busy` | Pulses a camera's trigger for `<width>` µs, from 1 to 1,000,000. A camera's pulse must end before its next one. |

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
| `busy` | That camera's pulse hasn't ended |
| `bad-line` | The line was longer than 120 bytes, or wasn't printable ASCII |

## 3. Events

| Event | Meaning |
|---|---|
| `! door open` | The door opened, and every channel is now off. |
| `! door shut` | The door counts as shut. Channels stay off until a command lights them. |
| `! exposure <1-2> start <time>` | A camera's exposure signal began. |
| `! exposure <1-2> end <time>` | It ended. |

`<time>` is in µs since power-up, read when the firmware noticed the edge, so it's a little after the edge itself. The firmware may send `! door shut` before a client connects, so use `status` to find the door's state.

The firmware holds up to 16 lines for a client that isn't reading, and after that it drops new ones and counts them. A client should read all the time, and treat a rising `dropped` as lost events. A dropped reply looks like a command with no answer.

## 4. The interlock

The door's switch cuts the LED drivers' enables in hardware, whatever the firmware does ([the carrier board](optima_carrier_board.md#4-how-the-circuits-work)). The firmware adds its own rules, so that its state matches the hardware's:

- **It starts with the door counted as open,** and with everything off.
- **Opening counts at once.** Any low reading of the door's signal puts out every channel, and sends `! door open`.
- **Shutting counts after 50 ms.** The signal must stay high that long, so a bouncing switch can't count as shut.
- **Nothing relights by itself.** After the door shuts, a client must light each channel again.
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

**The exposure signal's sense is assumed.** ToupTek publishes no pinout, so the firmware assumes that a camera's output conducts during an exposure, which pulls the input low. If the cameras do the opposite, `start` and `end` swap, and `exposures` counts ends, until `Inputs::exposure_low` is corrected.

## 6. What isn't here yet

The LED matrix, the filter slider, the camera port, the LEDs' temperatures and preloaded sequences. They're added to this protocol without breaking it, as E07 and the plan's later phases need them. `protocol=` changes only if a change could break a client.

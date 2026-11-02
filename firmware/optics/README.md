# The optics controller's firmware

The Rust firmware for the Pico 2 on the [carrier board](../../docs/misc/optima_carrier_board.md). [The protocol](../../docs/misc/optima_optics_protocol.md) says what it does: the LED channels with the interlock, the camera triggers and the exposure inputs (phase E06 of the [electronics plan](../../docs/misc/optima_electronics_plan.md)), and the LED matrix, the camera port, the filter slider and sequences (E07).

**It hasn't run on a chip yet.** Its logic is tested on a PC, and `picotool` reads the built image correctly, but the pins and USB are untested until someone follows "Trying it" below.

## The parts

| Path | What it is |
|---|---|
| `core/` | `optics-core`: every decision, with no hardware in it, and tested on a PC, with a fake TMC2209 at its UART. It has no `unsafe`, and no dependencies. |
| `fw/` | `optics-fw`: connects that to the pins and USB, with Embassy |
| `make_uf2.py` | Turns the built firmware into a file a Pico 2 takes as a drive |

It's a workspace of its own, with its own toolchain, apart from the server's ([ADR-0048](../../docs/adr/0048-the-optics-firmware-workspace.md)).

## Building and testing

Run these in this folder. `rustup toolchain install` gets the pinned toolchain and the RP2350's target, once.

```sh
cargo test --locked                               # the logic, on this PC
cargo clippy --all-targets --locked -- -D warnings
cargo fw-clippy                                   # the firmware
python test_make_uf2.py
cargo fmt --all --check

cargo fw                                          # builds the firmware
python make_uf2.py                                # writes optics-fw.uf2 beside it
```

CI does all this on every pull request, and keeps the `.uf2` file as a build artifact called `optics-firmware`.

## Trying it

This needs only a bare Pico 2 and a few jumper wires, and covers everything but the matrix and the slider's motor. The carrier board's pull-up and divider aren't there, so the jumpers stand in for them, and for the door.

1. **Wire it.** Join these to the Pico's 3V3(OUT), pin 36:
   - GP6, pin 9: the door, shut;
   - GP17, pin 22, and GP19, pin 25: the exposure inputs, not exposing.

   Join GP5, pin 7, to GND, pin 8: the slider's home switch, not pressed. Join GP20, pin 26, to GP21, pin 27: the camera port, which then reports wherever it's told to go.

   Don't leave them unconnected: the RP2350's erratum E9 can hold a floating input near 2 V, where it might read as either level. To change one, move its jumper between 3V3 and GND.
2. **Load it.** Hold BOOTSEL while plugging in USB, and copy `target/thumbv8m.main-none-eabihf/release/optics-fw.uf2` to the drive that appears. The Pico restarts by itself.
3. **Connect.** A new serial port appears. Open it with any terminal, such as `python -m serial.tools.miniterm --echo COM5`, using your own port.
4. **Talk to it,** with a meter or an LED and resistor on the pins:

| Type | Expect |
|---|---|
| `version` | `ok optima-optics protocol=1 firmware=0.2.0` |
| `status` | `ok door=shut leds=0000 triggers=00 exposing=00 exposures=0,0 dropped=0` |
| `led 1 on` | `ok`, and GP12 (pin 16) goes to 3.3 V |
| `led 4 on` | `ok`, and GP15 (pin 20) goes to 3.3 V |
| `led all off` | `ok`, and both go low |
| `trigger 1 1000000` | `ok`, and GP16 (pin 21) is at 3.3 V for a second |
| `trigger 2 1000000` | `ok`, and GP18 (pin 24) likewise |
| `led 1 on`, then move GP6's jumper to GND | `! door open`, and GP12 goes low at once |
| `led 1 on` | `err door-open` |
| Move GP6's jumper back to 3V3 | `! door shut`, and GP12 stays low |
| Move GP17's jumper to GND, then back to 3V3 | `! exposure 1 start <time>`, then `! exposure 1 end <time>` |
| `port 2` | `ok`, then `! port 2`, and GP20 goes to 3.3 V |
| `port 1` | `ok`, then `! port 1`, and GP20 goes low |
| Move GP21's end of the jumper to 3V3, then `port 1` | `ok`, then `! port fault` after 3 s. Put the jumper back. |
| `slider a` | `err not-homed` |
| `slider home` | `ok`, then `! slider fault driver`: there's no driver to answer |
| `seq add 1000 0 0 0`, then `seq add 0100 0 0 0` | `ok` each time |
| `seq start 1 1 0 0` | `ok`, then `! seq step 1 <time>`, and GP12 goes to 3.3 V |
| Move GP17's jumper to GND, then back to 3V3 | With the exposure's events, `! seq step 2 <time>`: GP12 goes low, and GP13 high |
| The same again | `! seq done`, and both go low |
| `seq start 1 1 0 100000` | `! seq step 1 <time>`, GP12 high, and GP16 pulses for 0.1 s |
| `seq stop` | `ok`, and GP12 goes low |

Losing USB puts out the LEDs too, but a bare Pico 2 runs from USB, so that waits for the carrier board, in E08.

### On the carrier board

These need the board, or at least a TMC2209 StepStick and an APA102 panel behind a 5 V buffer. E08 will make them a full bench test.

1. **The matrix.** Run `matrix 8000000000000000 0 0`: one LED, red, should light at row 1, column 1. Then `matrix 0 ff00000000000000 0` lights row 1 green. If the corner or the rows are wrong, correct `chain_bit` in `core/src/matrix.rs`. If the colours are wrong, correct the frame's order just above it. Check the panel's temperature with a full white pattern ([the protocol's §6](../../docs/misc/optima_optics_protocol.md#6-whats-assumed)).
2. **The slider.** With the motor's coupling free, or the slider away from both ends, run `slider home`. If it moves away from the switch, send `slider stop` and correct `AWAY_IS_HIGH` in `core/src/slider.rs`. Then press the switch by hand: it should stop, with `! slider homed`. Run `slider c`, `slider b` and `slider a`, and check each cell is on the axis. Listen for rough moves, and try changing the matrix during one ([ADR-0052](../../docs/adr/0052-the-filter-sliders-drive.md)).
3. **The camera port.** Set its modes as [the cables](../../docs/misc/optima_cables.md#the-camera-port-w14-w33-and-w34) say, then run `port 2` and `port 1`, and time the flips against the 3 s limit.
4. **A sequence with a camera.** Measure the exposure signal against the rows' timing, and choose the delay from that.

## Working on it

- **A move's pulses and the matrix's frames are the only timing in `fw/`.** Their schedule is in `core/`.
- **Decisions go in `core/`,** with a test. `fw/` should stay too simple to get wrong: it reads pins, passes on what they say, and sets them.
- **`unsafe`** is allowed only with a `SAFETY:` comment ([ADR-0049](../../docs/adr/0049-unsafe-code-in-the-firmware.md)). `core/` forbids it.
- **Dependencies** must be at least two weeks old and allow MIT distribution, including what they pull in. `Cargo.lock` is committed.
- **USB's IDs** are pid.codes' testing ones. Before this leaves the project, it needs its own ([ADR-0050](../../docs/adr/0050-the-optics-serial-protocol.md)).

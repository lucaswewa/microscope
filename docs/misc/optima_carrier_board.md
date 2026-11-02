# OPTIMA carrier board

The board under the Pico 2, for phase E04 of the [electronics plan](optima_electronics_plan.md). It holds the four LED drivers, the door's interlock, the filter slider's driver, the 5 V supply and the connectors for its cables. The [electrical design](optima_electrical_design.md) sets what it has to do. This is its schematic; E05 lays it out.

- The circuit is Python: [`electronics/carrier/circuit.py`](../../electronics/carrier/circuit.py).
- The schematic to read is [`carrier.pdf`](../../electronics/carrier/kicad/carrier.pdf). The KiCad project and the netlist are beside it.
- [The README](../../electronics/carrier/README.md) says how to rebuild them, and [`REPORT.md`](../../electronics/carrier/REPORT.md) has the checks' results.

Started 2026-11-01. Nothing here has been built or powered. "Assumed" marks what the parts in hand still have to confirm.

## 1. The parts

| Part | Ref | What it does |
|---|---|---|
| Raspberry Pi Pico 2 | U2 | The controller. It runs on the board's 5 V, through a Schottky diode (D2), so USB can stay plugged in. |
| Pololu D36V50F5 | U1 | The 24-to-5 V buck, for the matrix and the logic. It's rated 5.5 A at 36 V in, and gives each power connection two pins. |
| Mean Well LDD-1000H, four | U4–U7 | The epi LEDs' drivers, 1 A each |
| 74AHCT125, two | U3, U8 | 5 V buffers: U3 for the LED drivers' DIM lines, U8 for the matrix and the camera port |
| BTT TMC2209 V1.3 StepStick | U9 | The filter slider's driver |
| TI ADS1115 | U10 | An I2C ADC for the LEDs' four thermistors. It's the one surface-mount part, which JLCPCB assembles. |
| 2N3904, two | Q1, Q2 | The cameras' open-collector trigger outputs |
| 1N5822 | D1 | Guards everything against a reversed 24 V |

The rest are through-hole resistors, capacitors and connectors. The modules sit on headers.

**Why the D36V50F5.** E01 asked for a module rated 5 A. Pololu's D24V50F5 is rated 5 A too, but has one output pin. Pololu rates a pin like these at 3 A, so 4 A through one would be marginal. Pololu gives the D36V50F5's rating at 36 V in, and says its current depends on the input voltage; its graph, which I couldn't read, gives the figure at 24 V.

## 2. The Pico's pins

| GPIO | Pin | Net | Use |
|---|---|---|---|
| 0, 1 | 1, 2 | UART_TX, SLIDE_UART | UART0, to the TMC2209's one UART wire. TX goes through 1 kΩ. |
| 2, 3, 4 | 4, 5, 6 | SLIDE_STEP, SLIDE_DIR, SLIDE_EN | The slider's driver |
| 5 | 7 | SLIDE_HOME | The slider's home switch |
| 6 | 9 | DOOR_SENSE | The door's rail, through a divider |
| 8, 9 | 11, 12 | I2C_SDA, I2C_SCL | I2C0, to the ADC at address 0x48 |
| 10, 11 | 14, 15 | MATRIX_SCK, MATRIX_TX | SPI1, to the matrix through U8 |
| 12–15 | 16, 17, 19, 20 | LED1_EN–LED4_EN | The LEDs' enables, through U3 |
| 16, 17 | 21, 22 | TRIG1, TRIG2 | The triggers: 1 mono, 2 colour |
| 18, 19 | 24, 25 | EXPO1, EXPO2 | The exposure signals |
| 20 | 26 | PORT_SET_3V3 | The camera port's position, through U8 |
| 21 | 27 | PORT_AT | The camera port's position report |
| 7, 22, 26, 27, 28 | 10, 29, 31, 32, 34 | SPARE_… | To J15, with 3.3 V, ground and RUN |

That's 21 pins used, and 5 spare. GPIOs 26–28 are the ADC-capable ones, kept free in case a later phase wants them. The LEDs are numbered as the cables are: 1 is 395 nm, 2 is 470 nm, 3 is 565 nm and 4 is 625 nm.

## 3. Its connectors

| Ref | Kind | Cable | Pins |
|---|---|---|---|
| J1 | 2-way terminal | W08 | 1: 24 V in; 2: 0 V |
| J2 | 2-way JST XH | W35 | 1: 5 V to the door's switch; 2: back from it |
| J3–J6 | 2-way terminals | W23–W26 | 1: LED +; 2: LED − |
| J7 | 4-way terminal | W30 | 1: 5 V; 2: 0 V; 3: clock; 4: data |
| J8 | 2-way JST XH | W33 | 1: the port's DIG I/O 1; 2: 0 V |
| J9 | 2-way JST XH | W34 | 1: the port's DIG I/O 2; 2: 0 V |
| J10 | 4-way JST XH | W28 | 1, 2: coil A; 3, 4: coil B |
| J11 | 2-way JST XH | W29 | 1: the home switch; 2: 0 V |
| J12, J13 | 4-way JST XH | W31, W32 | 1: trigger; 2: 0 V; 3: exposure; 4: 0 V |
| J14 | 8-way JST XH | W27 | A pair per thermistor: 1 signal, 2 0 V, and so on |
| J15 | 8-way pin header | None | 3.3 V, 0 V, GPIOs 7, 22, 26, 27, 28, RUN |

The Pico's own micro-USB takes W36.

## 4. How the circuits work

**The interlock (U3).** This is the electrical design's §4, in parts.

- The door's switch alone powers U3. 5 V reaches it only through J2, the cable and the switch.
- With the door open, or the cover off, U3 has no supply. Each DIM line then falls to its own 1 kΩ pull-down, below the 0.8 V at which the driver turns off.
- U3's inputs have 4.7 kΩ pull-downs, so the LEDs stay off while the Pico starts or resets.
- The Pico can drive U3's inputs while it has no supply. TI's limit on an input is 7 V, with its supply at any level, and an input has a clamp only below ground.
- F1, a resettable fuse, protects the 5 V rail if the cable to the door shorts: the cable leaves the board and runs into the cover.
- The firmware reads the door through a 3.9 kΩ and 6.8 kΩ divider, which gives 3.18 V.

This is a good interlock, not a certified safety circuit. A short on the board between 5 V and the door's rail would defeat it; the layout in E05 should keep them apart.

**Pull-downs and the RP2350.** The Pico 2's earlier chips have a fault, erratum E9: an input pin can sit at about 2.1 V unless something holds it down. As Hackaday quotes the datasheet, an external pull-down of 8.2 kΩ or less is needed. So every pull-down on a Pico pin here is at most 6.8 kΩ. The door's sense needed this: a larger divider would read as "door shut" with the door open.

**The 5 V signals (U8).** The matrix's clock and data, and the camera port's position, go through U8, which is always on, then 47 Ω to tame the cables. The spare gate is held off.

**The camera port's report.** It's 5 V TTL, so the same divider as the door's brings it to 3.18 V.

**The slider (U9).** MS1 and MS2 are low, which gives it UART address 0, and EN's 10 kΩ pull-up keeps it off while the Pico starts.

- **One UART wire.** RX joins PDN_UART directly and TX goes through 1 kΩ, so the firmware hears its own bytes and has to discard them. I couldn't reach Trinamic's datasheet to check this against its figure; it's how other projects' documents wire it.
- **The socket.** Its legs follow BTT's schematic for the V1.3. StepStick modules aren't all the same, so check a different make against this one.
- **No DIAG.** E01 listed its diagnostic output as a Pico input. BTT's module has it on a side pin, outside the socket, so it would need a loose wire. The home switch finds home, and the firmware can read faults over the UART, so I left it out. That frees a pin.

**The cameras (Q1, Q2).** 3.3 V through 2.2 kΩ drives the base with about 1.2 mA, enough for the 10 mA E01 asks for. A 4.7 kΩ pull-down holds each trigger off at start. The exposure inputs have a 4.7 kΩ pull-up to 3.3 V and 1 kΩ in series, which suits an opto-isolated or open-collector output. They aren't made for a camera that drives its output above 3.3 V.

**The temperatures (U10).** Each thermistor, assumed 10 kΩ at 25 °C, forms a divider with a 1 % 10 kΩ resistor from 3.3 V, then a 1 kΩ and 100 nF filter into the ADC. The ADC's reference is its own, not the 3.3 V, so the 3.3 V rail's accuracy limits the reading. That's enough for watching heatsinks.

**Bulk capacitors.** 100 µF at the 24 V input and at the slider's driver, as StepStick boards usually have, and 470 µF on the 5 V rail for the matrix.

## 5. The checks

`make.py` runs KiCad's electrical rules check (ERC) and these checks of its own. All eight pass:

| Check | Result |
|---|---|
| The ERC is clean | No violations. It ignores footprint links until E05 adds this board's own footprints. |
| KiCad reads back the circuit in `circuit.py`, net for net | 73 nets, 81 parts |
| With the door open, nothing on the board can power the DIM lines | Each DIM has only its buffer and a pull-down. The buffer runs only from the door's rail, which 5 V reaches only through the switch. |
| Every peripheral sits on a pin that has it | UART0, I2C0 and SPI1, on pins confirmed against Raspberry Pi's SDK |
| No 5 V signal reaches a Pico pin, and the dividers read as highs | 3.18 V on both |
| Pull-downs hold the Pico's pins low, despite E9 | All at most 8.2 kΩ |
| The door's fuse carries the DIM lines' current | 20 mA with all four on, of 100 mA |
| Each connector matches its cable | 14 connectors, against [`cables.csv`](../../cad/optima/outputs/cables.csv) |

These catch mistakes in the wiring. They can't catch a wrong fact in the parts' data, so I read the facts that matter from the makers' sheets. One of them mattered: the LDD-1000H's DIM is pin 22, while the 1200 and 1500 mA models' is pin 21.

## 6. What's still open

- **The DIM pin's current.** Mean Well says an open DIM reads as on, but not what it sources. I assumed 0.5 mA, which 1 kΩ holds at 0.5 V. E08 measures DIM with the door open: it must be below 0.8 V.
- **The D36V50F5's pin order.** Pololu's page shows it only in a picture. E05 takes it from Pololu's drawing, along with the LDD-1000H's footprint, and these two are the board's own.
- **The buck's current at 24 V,** from Pololu's graph. The board needs about 4 A.
- **The cameras' trigger wiring,** which waits for ToupTek's pinouts (E01 §6).
- **The thermistors,** still to be chosen. The divider assumes 10 kΩ at 25 °C.
- **Which Pico 2 you buy.** Newer chips (stepping A3 or A4) don't have E9, but the board works with either.

## 7. Sources

- Mean Well's [LDD-H datasheet](https://www.farnell.com/datasheets/3169447.pdf), from which I read the pin tables.
- TI's [SN74AHCT125 datasheet](https://www.ti.com/lit/ds/symlink/sn74ahct125.pdf) and KiCad's symbol for the ADS1115.
- BTT's [TMC2209 V1.3 schematic](https://github.com/bigtreetech/BIGTREETECH-Stepper-Motor-Driver/blob/master/TMC2209/V1.3/Schematic/TMC2209%20V1.3-SCH.pdf) and [module page](https://neo.bttwiki.com/en/docs/accessories-docs/tmc-driver/tmc-2209/).
- Pololu's [D36V50F5](https://www.pololu.com/product/4091) and [D24V50F5](https://www.pololu.com/product/2851).
- KiCad's Pico symbol, and Raspberry Pi's [SDK register file](https://github.com/raspberrypi/pico-sdk/blob/master/src/rp2350/hardware_regs/include/hardware/regs/io_bank0.h) for the pins' functions.
- On powering the Pico 2 and on E9, these are second-hand, because I couldn't read Raspberry Pi's PDFs: [a pinout guide](https://mischianti.org/raspberry-pi-pico-2-high-resolution-pinout-datasheet-and-specs/), [Hackaday on E9](https://hackaday.com/2024/09/04/the-worsening-raspberry-pi-rp2350-e9-erratum-situation/) and [on the A4 stepping](https://hackaday.com/2025/07/31/).
- On the TMC2209's UART wiring, also second-hand: [a Rust driver's notes](https://docs.rs/crate/tmc2209-uart/latest) and [a Raspberry Pi project](https://pypi.org/project/TMC-2209-Raspberry-Pi/0.2.2).

# The carrier board's checks

Written by `make.py`. Don't edit.

| Check | Result | Detail |
|---|---|---|
| KiCad's electrical rules check is clean | pass | no violations |
| KiCad reads back the circuit in circuit.py, net for net | pass | 73 nets, 81 parts |
| With the door open, nothing on the board can power the LEDs' DIM lines | pass | each DIM has only its buffer and a pull-down; the buffer runs only from the door's rail; 5 V reaches that rail only through the switch |
| Every peripheral sits on a pin that has it, and 21 of 26 pins are used | pass | UART0, I2C0 and SPI1; 5 spares |
| No 5 V signal reaches a Pico pin, and the dividers read as logic highs | pass | DOOR_SENSE 3.18 V, PORT_AT 3.18 V |
| Pull-downs on the Pico's pins hold them low despite the RP2350's erratum E9 | pass | all at most 8.2 kohm |
| The door's PTC carries the DIM lines' current | pass | 20 mA with all four on, of its 100 mA |
| Each connector matches its cable in the cable list | pass | 14 connectors matched |
| KiCad's design rules check is clean | pass | no violations, and thin thermals only on pads that carry no power |
| Every net is routed | pass | 0 connections unrouted |
| The board matches its schematic | pass | no differences |
| Each power net's tracks are as wide as its current needs, with no via on a supply | pass | 617 tracks |
| The parts are where placement.py says | pass | 81 parts |
| The board is the size the CAD gives it | pass | 140 x 100 mm, the CAD's 140 x 100 |

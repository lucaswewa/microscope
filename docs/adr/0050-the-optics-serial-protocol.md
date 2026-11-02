# ADR-0050: The optics controller's serial protocol

- Status: Accepted
- Date: 2026-11-02
- Phase: OPTIMA E06

## Context

The server and a person at a bench both need to talk to the optics controller, over USB. The [hardware design](../misc/optima_hardware_design.md#65-motion-and-optics-controllers) asks for a line-based protocol, with the timing-critical work in preloaded sequences on the chip, so that frames need no round trip to the PC. The interlock has to keep working whatever the USB side does.

## Decision

[The protocol's document](../misc/optima_optics_protocol.md) is the specification. Its main choices:

- **Text lines over USB CDC:** printable ASCII, up to 120 bytes, ended by a line feed. Anyone can use it from a terminal.
- **One reply per command,** starting `ok` or `err <code>`, and **events** starting `!`, which can arrive at any time. The first word of a line says which it is.
- **A version:** `version` reports `protocol=1`, which changes only if a change could break a client. Adding commands doesn't.
- **The door counts as open until shown otherwise.** Opening counts at once. Shutting counts after 50 ms high, and relights nothing.
- **USB can't hold up the interlock.** The control loop owns the pins and talks to USB only through queues, and it never waits for one. If a queue is full, it drops the line and counts it, in `status`.
- **Losing USB puts out the LEDs,** since nothing could switch them off then. Closing the serial port doesn't, because a client may reopen it.
- **USB's IDs are pid.codes' testing pair,** `1209:0001`, for now. They suit a bench, not a product.

## Consequences

- **It's easy to try and to debug,** with no tool beyond a terminal, and easy to test on a PC, as strings.
- **It's slower than a binary protocol,** which doesn't matter: commands set up the light path, and E07's sequences run on the chip.
- **A client must read all the time,** and route lines by their first word, because events can land between a command and its reply. A dropped line is reported only by a count.
- **The exposure signal's sense is assumed,** as is the rest of the cameras' interface, until they're in hand.
- **The IDs must change** before the firmware leaves this project: a free PID from pid.codes suits an open-source project.

## Alternatives considered

- **Binary frames, such as COBS with `postcard`:** they're compact and strictly checked, but need a tool to read or type, and there's no speed to gain here.
- **G-code, like the motion controller's grblHAL:** it's familiar, but it describes machine motion, and LEDs and triggers would be a poor fit. GPL code isn't an issue here, but borrowing its shape isn't needed either.
- **Only polling, with no events:** it's simpler for a client, but it would miss short exposures, and a door opening wouldn't be seen until the next poll.
- **Keeping LEDs on across a USB loss:** a sequence could run on, but nothing could then stop an LED except the door. E07 can revisit this for sequences.
- **A Raspberry Pi USB ID:** those are for products that apply for one under Raspberry Pi's VID.

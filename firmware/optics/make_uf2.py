#!/usr/bin/env python3
"""Turns the firmware's ELF file into a UF2 file, which a Pico 2 in BOOTSEL mode takes as a drive.

    python firmware/optics/make_uf2.py [elf] [uf2]
"""

import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ELF = HERE / "target" / "thumbv8m.main-none-eabihf" / "release" / "optics-fw"
UF2 = ELF.with_suffix(".uf2")

MAGIC_START = (0x0A324655, 0x9E5D5157)
MAGIC_END = 0x0AB16F30
FAMILY_FLAG = 0x00002000
RP2350_ARM_SECURE = 0xE48BFF59
PAGE = 256  # the data in a block, which is a flash page
BLOCK_DATA = 476  # the room for it


def load_segments(elf):
    """The (address, bytes) of what an ELF file puts in memory, by the address it's loaded at."""
    if elf[:6] != b"\x7fELF\x01\x01":
        sys.exit("Not a little-endian 32-bit ELF file.")
    table, = struct.unpack_from("<I", elf, 28)
    size, count = struct.unpack_from("<HH", elf, 42)
    for i in range(count):
        kind, offset, _, address, length = struct.unpack_from("<5I", elf, table + i * size)
        if kind == 1 and length:  # PT_LOAD, with something in the file
            yield address, elf[offset:offset + length]


def pages(segments):
    """{page's address: its 256 bytes}, with zeros wherever nothing is placed."""
    found = {}
    for address, data in segments:
        for i, byte in enumerate(data):
            start = (address + i) // PAGE * PAGE
            found.setdefault(start, bytearray(PAGE))[address + i - start] = byte
    return found


def uf2(found):
    blocks = []
    for number, start in enumerate(sorted(found)):
        head = struct.pack("<8I", *MAGIC_START, FAMILY_FLAG, start, PAGE, number, len(found), RP2350_ARM_SECURE)
        blocks.append(head + bytes(found[start]).ljust(BLOCK_DATA, b"\0") + struct.pack("<I", MAGIC_END))
    return b"".join(blocks)


def main():
    elf = Path(sys.argv[1]) if len(sys.argv) > 1 else ELF
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else UF2
    found = pages(load_segments(elf.read_bytes()))
    out.write_bytes(uf2(found))
    print("%s: %d kB, from 0x%08x" % (out, len(found) * PAGE // 1024, min(found)))


if __name__ == "__main__":
    main()

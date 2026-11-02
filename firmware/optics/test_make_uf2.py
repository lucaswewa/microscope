#!/usr/bin/env python3
"""Tests make_uf2.py on a small ELF file made here: python firmware/optics/test_make_uf2.py"""

import struct
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import make_uf2  # noqa: E402


def elf(*segments):
    """An ELF file with these (kind, load address, file bytes) program headers."""
    table, size = 52, 32
    offset = table + size * len(segments)
    headers, body = b"", b""
    for kind, address, data in segments:
        # Its virtual address differs, as the firmware's .data does: only the load address counts.
        headers += struct.pack("<8I", kind, offset + len(body), 0x20000000, address, len(data), len(data), 0, 4)
        body += data
    head = b"\x7fELF\x01\x01" + bytes(22) + struct.pack("<I", table) + bytes(10) + struct.pack("<HH", size, len(segments))
    return head.ljust(table, b"\0") + headers + body


class MakeUf2(unittest.TestCase):
    def blocks(self, *segments):
        data = make_uf2.uf2(make_uf2.pages(make_uf2.load_segments(elf(*segments))))
        self.assertEqual(len(data) % 512, 0)
        return [data[i:i + 512] for i in range(0, len(data), 512)]

    def test_a_block_holds_a_page_with_its_markings(self):
        block, = self.blocks((1, 0x10000000, b"\xAA\xBB"))
        words = struct.unpack_from("<8I", block)
        self.assertEqual(words, (0x0A324655, 0x9E5D5157, 0x2000, 0x10000000, 256, 0, 1, 0xE48BFF59))
        self.assertEqual(block[32:35], b"\xAA\xBB\0")
        self.assertEqual(struct.unpack_from("<I", block, 508), (0x0AB16F30,))

    def test_segments_that_share_or_cross_pages_land_in_their_places(self):
        blocks = self.blocks((1, 0x10000100, b"\x11" * 300), (1, 0x10000000 + 0x100 + 300, b"\x22"))
        self.assertEqual([struct.unpack_from("<I", b, 12)[0] for b in blocks], [0x10000100, 0x10000200])
        self.assertEqual([struct.unpack_from("<2I", b, 20) for b in blocks], [(0, 2), (1, 2)])
        self.assertEqual(blocks[1][32:32 + 45], b"\x11" * 44 + b"\x22")

    def test_only_loaded_segments_with_bytes_count(self):
        self.assertEqual(len(self.blocks((4, 0x10000000, b"note"), (1, 0x20000000, b""), (1, 0x10000400, b"x"))), 1)

    def test_it_refuses_a_file_that_is_not_an_elf_file(self):
        with self.assertRaises(SystemExit):
            list(make_uf2.load_segments(b"not an ELF file"))


if __name__ == "__main__":
    unittest.main()

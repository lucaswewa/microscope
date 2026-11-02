/* The Pico 2: 4 MB of flash, and the RP2350's 512 kB of main SRAM. */
MEMORY {
    FLASH : ORIGIN = 0x10000000, LENGTH = 4096K
    RAM : ORIGIN = 0x20000000, LENGTH = 512K
}

/* The boot ROM looks for the image's definition in the first 4 kB of flash. picotool looks for
   the header of the program's description in the first 512 bytes, so both follow the vectors. */
SECTIONS {
    .start_block : ALIGN(4)
    {
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH
} INSERT AFTER .vector_table;

_stext = ALIGN(ADDR(.start_block) + SIZEOF(.start_block), 8);

/* The program's description, which the header above points to. */
SECTIONS {
    .bi_entries : ALIGN(4)
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries));
        . = ALIGN(4);
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

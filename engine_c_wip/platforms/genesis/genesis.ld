/* engine_c/platforms/genesis/genesis.ld — Mega Drive / Genesis linker script.
   2 MB ROM (code + embedded card data) at 0x000000; 64 KB work RAM at 0xFF0000. */
OUTPUT_FORMAT("elf32-m68k")
ENTRY(_start)

MEMORY {
    rom (rx)  : ORIGIN = 0x000000, LENGTH = 0x200000   /* 2 MB */
    ram (rwx) : ORIGIN = 0xFF0000, LENGTH = 0x010000   /* 64 KB WRAM */
}

__stack_top = ORIGIN(ram) + LENGTH(ram) - 2;           /* 0xFFFFFE */

SECTIONS {
    .text : {
        *(.text .text.*)
        *(.rodata .rodata.*)
        . = ALIGN(4);
    } > rom

    /* ROM-embedded data blobs (cards.bin / abilities_strings.bin) live here,
       read-only in the cartridge; never copied to RAM. */
    .romdata : {
        *(.romdata .romdata.*)
        . = ALIGN(4);
    } > rom

    .data : {
        _sidata = LOADADDR(.data);
        _sdata  = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        . = ALIGN(4);
        _edata  = .;
    } > ram AT > rom

    .bss (NOLOAD) : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > ram
}

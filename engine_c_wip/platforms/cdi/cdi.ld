/* engine_c/platforms/cdi/cdi.ld — CD-i linker script.
   The CD-i serial stub loads the module at 0x8000 and resets into it.
   1 MB total RAM: 0x0000–0x8000 reserved by the stub/OS-9, so our image
   lives at 0x8000 and the stack grows down from the top of 1 MB (0x100000). */
OUTPUT_FORMAT("elf32-m68k")
ENTRY(_start)

MEMORY {
    ram (rwx) : ORIGIN = 0x00008000, LENGTH = 0x000F8000   /* 992 KB */
}

SECTIONS {
    .text : {
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.got .got.*)
    } > ram

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
        _sidata = LOADADDR(.data);
        _sdata = .;
    } > ram

    .bss (NOLOAD) : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > ram

    /* bump arena (alloc.c) + C heap sit in .bss/.COMMON; keep them in range */
    . = ALIGN(8);
    __stack = ORIGIN(ram) + LENGTH(ram);   /* top of RAM */
}

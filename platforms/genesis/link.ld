/* Sega Genesis / Mega Drive - 4MB cart ROM + 64KB work RAM */
ENTRY(_start)

MEMORY
{
    rom (rx)  : ORIGIN = 0x00000000, LENGTH = 0x400000
    ram (rwx) : ORIGIN = 0x00FF0000, LENGTH = 0x010000
}

SECTIONS
{
    /* 68000 vector table: 8 longwords at 0x000000 - KEEP so linker doesn't GC */
    .vectors 0x00000000 : {
        KEEP(*(.vectors))
        . = ALIGN(0x4);
    } > rom

    /* Genesis ROM header at 0x000100 */
    .header 0x000100 : {
        KEEP(*(.header))
    } > rom

    .text 0x00000200 :
    {
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        . = ALIGN(4);
        _etext = .;
    } > rom

    /* .data is stored in ROM (_etext) but copied to RAM at startup by crt0 */
    .data : AT(ADDR(.text) + SIZEOF(.text))
    {
        _sdata = .;
        *(.data .data.* .sdata .sdata.*)
        . = ALIGN(4);
        _edata = .;
    } > ram

    .bss (NOLOAD) :
    {
        _sbss = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > ram

    /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) }
}

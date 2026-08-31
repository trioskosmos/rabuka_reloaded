/* Sega Genesis / Mega Drive link script: 4 MB cart ROM + 64 KB work RAM. */
ENTRY(_start)

MEMORY
{
    rom (rx)  : ORIGIN = 0x00000000, LENGTH = 0x400000
    ram (rwx) : ORIGIN = 0x00FF0000, LENGTH = 0x010000
}

SECTIONS
{
    .vectors 0x00000000 : {
        KEEP(*(.vectors))
        . = ALIGN(0x4);
    } > rom

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

    .romdata : {
        *(.romdata)
        . = ALIGN(4);
    } > rom

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

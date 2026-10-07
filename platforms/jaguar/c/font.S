/* light8x8 font as a linkable object (same .fnt the POC used:
 * 8-byte header, then 8 bytes per glyph). */
    .section .rodata
    .globl font_light8x8
font_light8x8:
    .incbin "light8x8.fnt"

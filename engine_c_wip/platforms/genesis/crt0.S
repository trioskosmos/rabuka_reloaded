/* engine_c/platforms/genesis/crt0.S — Mega Drive / Genesis bare-metal entry.
   68000 reset vector table + the Sega ROM header, then the C runtime start. */
    .section .text
    .global _start

    /* ── 68000 exception vector table (first 0x100 bytes) ── */
    .long __stack_top        /* 0x000: initial SSP */
    .long _start             /* 0x004: reset PC */
    .rept 62
    .long _trap              /* vectors 2..63 -> spurious/exception handler */
    .endr

    /* ── Sega Mega Drive ROM header (0x100..0x1FF) ── */
    .ascii "SEGA"            /* 0x100  TMSS signature */
    .fill 12, 1, 0x20        /* 0x104  reserved (spaces) */
    .ascii "SEGA"            /* 0x110 */
    .fill 12, 1, 0x20        /* 0x114 */
    .ascii "SEGA"            /* 0x120 */
    .fill 12, 1, 0x20        /* 0x124 */
    .ascii "SEGA"            /* 0x130 */
    .fill 12, 1, 0x20        /* 0x134 */
    .fill 16, 1, 0x20        /* 0x140  reserved */
    .ascii "RABUKA ENGINE                        "  /* 0x150  domestic (48) */
    .ascii "RABUKA ENGINE                        "  /* 0x180  overseas (48) */
    .ascii "GM 00000000-00"  /* 0x1C0  serial (14) */
    .word 0x0000             /* 0x1CE  checksum (patched by build_genesis.sh) */
    .byte 0x20, 0x00        /* 0x1D0  I/O support (joypad) */
    .byte 0x4A, 0x44        /* 0x1D2  "JD" */
    .byte 0x00, 0x00        /* 0x1D4 */
    .byte 0x00, 0x00        /* 0x1D6 */
    .byte 0x00, 0x00        /* 0x1D8 */
    .byte 0x00, 0x00        /* 0x1DA */
    .byte 0x47, 0x4D        /* 0x1DC  "GM" (game machine) */
    .byte 0x00, 0x00        /* 0x1DE */
    .fill 16, 1, 0x20       /* 0x1E0  reserved */
    .fill 16, 1, 0x00       /* 0x1F0  reserved -> end of header (0x200) */

    .text
_trap:
    jra _trap               /* any unhandled exception just spins */

_start:
    /* copy .data ROM image -> RAM (initialised globals) */
    move.l  #_sidata, %a0
    move.l  #_sdata, %a1
    move.l  #_edata, %a2
1:  cmp.l   %a1, %a2
    jbeq    2f
    move.l  (%a0)+, (%a1)+
    jra     1b
2:
    /* zero .bss */
    move.l  #__bss_start, %a0
    move.l  #__bss_end, %a1
    moveq.l #0, %d0
3:  cmp.l   %a0, %a1
    jbeq    4f
    move.l  %d0, (%a0)+
    jra     3b
4:
    jbsr    genesis_main
    jra     _trap           /* halt; nothing to return to */

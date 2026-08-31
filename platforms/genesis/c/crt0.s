/* Sega Genesis / Mega Drive bootstrap (m68k, GNU as). */
.section .vectors, "aw"
.long 0x00FFFE00          /* SSP (top of 64 KB work RAM) */
.long _start               /* PC reset */
.rept 62
.long _start
.endr

.section .header, "aw"
.ascii "SEGA GENESIS    "   /* 0x100 console name */
.ascii "JUE             "   /* region */
.ascii "                "
.ascii "                "
.ascii "RABUKA ENGINE C "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "GM 2026.AUG     "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "
.ascii "                "

.section .text
.global _start
_start:
    move.l #0x00FFFE00, %sp

    /* copy .data (ROM image) -> RAM */
    move.l #_sdata, %a0
    move.l #_edata, %a1
    move.l #_etext, %a2
1:
    cmp.l  %a0, %a1
    beq.s  2f
    move.l (%a2)+, (%a0)+
    bra.s  1b
2:
    /* zero .bss */
    move.l #_sbss, %a0
    move.l #_ebss, %a1
    clr.l  %d0
3:
    cmp.l  %a0, %a1
    beq.s  4f
    move.b %d0, (%a0)+
    bra.s  3b
4:
    jbsr   genesis_main
5:
    bra.s  5b

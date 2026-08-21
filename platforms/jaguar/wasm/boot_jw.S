/* Rabuka Jaguar boot for the wasm2c pipeline.
 * Same proven GPU/OP init as the rustc_codegen_gcc POC boot.S (draw_test
 * lineage), plus .data copy + .bss clear (the C shell needs them, the old
 * Rust staticlib didn't), then jag_main().
 */

    .set OPL_BASE,    0x00001000
    .set TEXT_SCREEN, 0x00042000
    .set SCREEN_MODE, 0x06C7
    .set GPU_SRAM,    0x00F03000
    .set GPU_PARAM,   0x00F03FF0

    .text
    .globl _start
_start:
    /* ---- canonical init (startup.inc) ---- */
    move #0x2700,%sr
    lea 0x1FFF00,%sp             /* top of DRAM, above the malloc arena */
    lea 0x00F02100,%a0
    move.l #0x00070007,0x0C(%a0)
    move.l #0x00070007,0x00F1A10C
    clr.l 0x14(%a0)
    clr.l 0x00F1A114
    clr.l 0x00(%a0)
    move.l #0x00011F00,0x00F1A100
    move.w #0x3F00,0x00F10020
    clr.l 0x00000000
    move.l #4,0x00000004
    clr.l 0x00F00020
    clr.w 0x00F00026
    clr.w 0x00F0002A
    clr.w 0x00F0002C
    clr.w 0x00F00056
    clr.w 0x00F00058
    clr.l 0x00F00050
    move.l #0x1F000000,0x00F000E0
    move.w #0x7FFF,0x00F0004E

    /* ---- zero .bss, copy .data (linker-script symbols) ---- */
    lea __bss_start,%a0
    lea __bss_end,%a1
1:  cmp.l %a1,%a0
    bhs.s 2f
    clr.b (%a0)+
    bra.s 1b
2:  lea __data_lma,%a0
    lea __data_vma,%a1
    lea __data_end,%a2
3:  cmp.l %a2,%a1
    bhs.s 4f
    move.b (%a0)+,(%a1)+
    bra.s 3b
4:

    /* ---- zero GPU SRAM (mirror draw_test) ---- */
    lea GPU_SRAM+0x8000,%a0
    move.l #4096,%d0
clr_gpu:
    clr.l (%a0)+
    subq.l #1,%d0
    bne.s clr_gpu

    /* ---- copy GPU program to GPU SRAM (InitGPU) ---- */
    lea 0x00F02110,%a4
    lea gpu_blob+4,%a0        /* skip BS94 magic */
    clr.l 4(%a4)              /* GPU halt */
    movea.l (%a0)+,%a1        /* a1 = dest (0xF03000) */
    lea 0x10(%a1),%a3         /* GPU PC = dest+$10 */
    move.l %a3,(%a4)
    move.l (%a0)+,%d0         /* length */
    adda.l #0x8000,%a1        /* copy to dest+$8000 */
    bsr memcpy
    moveq.l #1,%d0
    move.l %d0,4(%a4)         /* start GPU */

    /* ---- copy object list into RAM at OPL_BASE ---- */
    lea op_list,%a0
    lea OPL_BASE,%a1
    move.l #op_list_end-op_list,%d0
5:  move.b (%a0)+,(%a1)+
    subq.l #1,%d0
    bne.s 5b

    /* ---- GPU param block at $f03ff0 ---- */
    move.l #OPL_BASE,0x00F03FF0        /* obl0 */
    move.l #OPL_BASE,0x00F03FF4        /* obl1 */
    move.l #(op_list_end-op_list)/8,0x00F03FF8  /* obl_size (phrases) */

    /* ---- VideoInit (NTSC 320) ---- */
    move.w #0x007B,0x00F00038    /* HDB1 */
    move.w #0x007B,0x00F0003A    /* HDB2 */
    move.w #0x06BF,0x00F0003C    /* HDE */
    move.w #25,0x00F00046        /* VDB */
    move.w #507,0x00F00048       /* VDE */
    move.w #0x7FFF,0x00F0004E    /* VI */

    /* ---- InitOP: object list at OPL_BASE, INT1 enable, real my_irq ---- */
    move.l #OPL_BASE,%d0
    swap %d0
    move.l %d0,0x00F00020        /* OLP */
    clr.w 0x00F00026             /* OBF */
    move.w #SCREEN_MODE,0x00F00028
    clr.w 0x00F0002A
    clr.w 0x00F0002C
    move.l #0x1F02<<16,0x00F000E0  /* INT1: enable VI + GPU */
    lea my_irq,%a0
    move.l %a0,0x00000100

    /* ---- CLUT: black bg / white text for the 1bpp bitmap ---- */
    lea 0x00F00400,%a0
    moveq #0,%d0
6:  move.w %d0,(%a0)+
    subq.b #1,%d0
    bne.s 6b
    clr.w 0x00F00400             /* CLUT[0] black */
    move.w #0xFFFF,0x00F00402    /* CLUT[1] white */

    /* ---- enable interrupts, then run the engine ---- */
    move #0x2000,%sr
    jsr jag_main
7:  bra.s 7b

/* ---- real IRQ: ack VI + GPU (mirror draw_test my_irq exactly) ---- */
    .globl my_irq
my_irq:
    move.l #0x0202<<16,0x00F000E0
    rte

/* ---- 68k-to-GPU memcpy via blitter: a0=src, a1=dest, d0=len ---- */
memcpy:
    lea 0x00F02200,%a2
    move.l %a1,(%a2)          /* A1_BASE = dest */
    move.l %a0,0x24(%a2)      /* A2_BASE = src */
    move.l #0x0000005E28,%d1  /* PITCH1|PIXEL32|WID3584|XADDPHR */
    move.l %d1,0x04(%a2)      /* A1_FLAGS */
    move.l %d1,0x28(%a2)      /* A2_FLAGS */
    moveq #0,%d1
    move.l %d1,0x0C(%a2)      /* A1_PIXEL_PTR */
    move.l %d1,0x30(%a2)      /* A2_PIXEL_PTR */
    lsr.w #2,%d0
    bset #16,%d0
    move.l %d0,0x3C(%a2)      /* COUNTER */
    move.l #0x01800001,%d0    /* SRCEN|LFU_REPLACE */
    move.l %d0,0x38(%a2)      /* COMMAND */
mc_wbl1:
    move.l 0x38(%a2),%d0
    btst #0,%d0
    beq.s mc_wbl1
    rts

/* ---- GPU program (draw_test's, proven): [BS94][dest=0xF03000][len][code] ---- */
    .align 8
gpu_blob:
    .incbin "gpu_blob.bin"

/* ---- object list: bitmap @ $42000, 1bpp idx0 (proven POC list) ---- */
    .align 8
op_list:
    .long 0x00000080, 0x03004183
    .long 0x00000080, 0x03009083
    .long 0x00000000, 0x0000000A   /* gpuobj */
    .long 0x00000000, 0x00000004   /* stop */
    .long 0x04200080, 0x063c0180   /* bitmap DATA=TEXT_SCREEN(0x42000) */
    .long 0x00010000, 0x50148000
    .long 0x00000000, 0x00000004   /* stop */
op_list_end:

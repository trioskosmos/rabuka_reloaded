// Copyright (c) 2024 Matheus C. França
// SPDX-License-Identifier: Apache-2.0
//
// SNES startup: W65C816S emulation→native mode switch.
// Compiled with -mcpu=mosw65816 so the assembler accepts 65816 instructions.

    .section .init.000,"ax",@progbits
    .global _start
_start:
    sei
    cld
    ldx #0xff
    txs
    clc
    xce
    pea 0x0000
    pld
    lda #0x00
    pha
    plb
    lda __snes_memsel
    sta 0x420d

    .extern __snes_memsel
    .extern vblank_flag
    .extern pad_keys
    .extern pad_keysold
    .extern pad_keysdown

    .section .text.nmi_handler,"ax",@progbits
    .global nmi_handler
nmi_handler:
    rep #0x30
    pha
    phx
    phy
    sep #0x20
    phb
    phd
    lda #0x00
    pha
    plb
    lda #0x01
    sta vblank_flag
    rep #0x30
    lda pad_keys
    sta pad_keysold
    lda 0x4218
    sta pad_keys
    eor pad_keysold
    and pad_keys
    sta pad_keysdown
    lda pad_keys+2
    sta pad_keysold+2
    lda 0x421a
    sta pad_keys+2
    eor pad_keysold+2
    and pad_keys+2
    sta pad_keysdown+2
    pld
    plb
    ply
    plx
    pla
    rti

    .section .vectors,"a",@progbits
    .word 0x0000
    .word 0x0000
    .word 0x0000
    .word nmi_handler
    .word 0x0000
    .word 0x0000
    .fill 10, 1, 0
    .word 0x0000
    .word _start
    .word 0x0000

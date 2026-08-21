/* Minimal m68k setjmp/longjmp for the wasm2c trap mechanism.
 * jmp_buf layout (first 13 longwords of glibc's m68k __jmp_buf are unused
 * beyond what we store; the buffer is only ever touched by these two):
 *   [0..5]  d2-d7        (callee-saved data regs)
 *   [6..10] a2-a6        (callee-saved address regs)
 *   [11]    caller's sp  (value of %sp in setjmp's caller frame)
 *   [12]    return pc
 * Callee-saved restoration is sufficient: all caller-saved state between
 * the setjmp frame and the longjmp call site is dead by definition.
 */

    .text
    .globl _setjmp
    .globl setjmp
    .globl __sigsetjmp
_setjmp:
setjmp:
__sigsetjmp:
    move.l 4(%sp),%a0          /* buf */
    movem.l %d2-%d7,0(%a0)     /* buf[0..5] */
    movem.l %a2-%a6,24(%a0)    /* buf[6..10] */
    move.l %sp,%a1
    addq.l #4,%a1              /* caller's sp (past ret addr + buf arg) */
    move.l %a1,44(%a0)         /* buf[11] */
    move.l 4(%sp),48(%a0)      /* buf[12] = return address */
    moveq.l #0,%d0             /* return 0 from direct call */
    rts

    .globl longjmp
    .globl _longjmp
    .globl siglongjmp
longjmp:
_longjmp:
siglongjmp:
    move.l 4(%sp),%a0          /* buf */
    move.l 8(%sp),%d0          /* val */
    bne.s 1f
    moveq.l #1,%d0             /* longjmp(buf,0) returns 1 */
1:  movem.l 0(%a0),%d2-%d7
    movem.l 24(%a0),%a2-%a6
    move.l 44(%a0),%sp         /* back to setjmp caller's sp */
    move.l 48(%a0),%a0         /* return address */
    jmp (%a0)

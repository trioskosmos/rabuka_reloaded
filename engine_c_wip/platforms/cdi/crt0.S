/* engine_c/platforms/cdi/crt0.S — bare-metal entry for Philips CD-i (SCC68070)
   Loaded by the CD-i Link/serial stub at 0x8000. Sets up the stack and
   calls cdi_main(); loops forever on return (no OS to return to). */
    .text
    .globl _start
_start:
    lea     __stack, %sp
    jbsr    cdi_main
1:
    jmp     1b

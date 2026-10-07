#ifndef JAGUAR_CONSOLE_H
#define JAGUAR_CONSOLE_H

/* GPU-rendered 1-bpp text console (320x240, 40x30 chars of 8x8).
 * Display and input logic ported from the proven wasm2c shell
 * (platforms/jaguar/wasm/jag_main.c), which was itself ported
 * from the working rustc_codegen_gcc POC. */

void console_init(void);
void console_clear(void);
void console_putc(char c);
void console_puts(const char *s);
void console_printf(const char *fmt, ...);

/* printf family in sys.c routes here */
void __console_out(int c);

/* Jagpad: returns the engine button mask
 * (A=1, B=2, Up=4, Down=8, Start=16) */
unsigned int console_poll_buttons(void);

/* Spin ~1 frame (single-buffered; the POC's pacing) */
void console_frame_wait(void);

#endif

#ifndef GENESIS_CONSOLE_H
#define GENESIS_CONSOLE_H

/* Text console rendered into VDP Plane A (40x28 cells, 8x8 font). */
void console_init(void);
void console_putchar(int c);
void console_puts(const char *s);
void console_printf(const char *fmt, ...);

/* Low-level output sink used by the printf family in sys.c. */
void __console_out(int c);

#endif

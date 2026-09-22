/* engine_c/platforms/cdi/cdi_main.c — CD-i shell over the C engine.
   Bare-metal entry: no argc/argv, app loaded at 0x8000 by the serial stub.
   Data (cards.bin / abilities_strings.bin) is streamed from the CD via
   cdi_read(); on a hosted build cdi_read falls back to the PC filesystem
   so the exact same image can be smoke-tested under an m68k Linux runner. */
#include "rabuka.h"

/* CD sector read shim. Replace cdi_read with a real CD-RTOS/BIOS read for
   hardware; the host fallback below lets the same binary run on a PC. */
#ifndef CDI_HOST
static unsigned char *cdi_read(const char *path, long *out_len) {
    (void)path; (void)out_len;
    return NULL;   /* forces rb_load_streaming -> rb_load (reveals libc surface) */
}
#else
#include <stdio.h>
static unsigned char *cdi_read(const char *path, long *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
    unsigned char *b = malloc(n ? n : 1);
    if (!b) { fclose(f); return NULL; }
    if (fread(b, 1, n, f) != (size_t)n) { free(b); fclose(f); return NULL; }
    fclose(f); *out_len = n; return b;
}
#endif

int cdi_main(void) {
    if (rb_load_streaming("engine_c/src", cdi_read) != 0) return 1;
    GameState g;
    uint32_t d0[20] = {0}, d1[20] = {0};
    for (int i = 0; i < 20; i++) { d0[i] = (uint32_t)i; d1[i] = (uint32_t)(20 + i); }
    rb_seed(0x1234);
    rb_game_init(&g, d0, 20, d1, 20);
    for (int t = 0; t < 100 && g.winner < 0; t++) {
        rb_turn(&g);
        while (rb_has_pending_choice(&g)) rb_resume_with_choice(&g, -1);
    }
    return g.winner;
}

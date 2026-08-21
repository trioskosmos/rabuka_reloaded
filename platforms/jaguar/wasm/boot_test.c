/* Minimal boot test: no wasm, just prove display works. */
#include <stdint.h>
typedef uint8_t u8;
#define TEXT_SCREEN 0x00042000u
#define BYTES_PER_ROW 40
#define MAX_Y 240
#define FONT_BASE 8
extern const unsigned char font_light8x8[];
static volatile unsigned char * const text_screen = (volatile unsigned char *)TEXT_SCREEN;
static int cur_col = 0, cur_row = 0;

static void disp_clear(void) {
    for (int i = 0; i < BYTES_PER_ROW * MAX_Y; i++) text_screen[i] = 0;
    cur_col = cur_row = 0;
}
static void disp_newline(void) { cur_col = 0; if (++cur_row >= 30) cur_row = 0; }
static void disp_put_char(unsigned char c) {
    if (cur_col >= 40) disp_newline();
    unsigned int off = FONT_BASE + (unsigned int)c * 8;
    for (int r = 0; r < 8; r++)
        text_screen[(unsigned int)(cur_row*8+r)*BYTES_PER_ROW + cur_col] = font_light8x8[off+r];
    cur_col++;
}
static void disp_print(const unsigned char *s, unsigned n) {
    for (unsigned i = 0; i < n; i++) {
        if (s[i] == '\n') disp_newline();
        else if (s[i] >= 0x20 && s[i] <= 0x7E) disp_put_char(s[i]);
    }
}
void jag_main(void) {
    disp_clear();
    static const unsigned char m[] = "BOOT OK - MINIMAL TEST";
    disp_print(m, sizeof(m)-1);
    for (;;) { volatile unsigned n = 0; while (n < 200000u) n++; }
}

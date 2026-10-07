/* Rabuka Reloaded — Atari Jaguar port (native m68k C engine). SHELL.
 *
 * engine_c_wip compiled straight to 68000 machine code with
 * m68k-unknown-linux-gnu-gcc, executing in place from cartridge
 * ROM at $802000. Card/ability/bytecode data is ROM-resident
 * (rb_load_streaming + RB_ROM_STRINGS: pack.py NUL-terminates
 * the string blobs and emits their pointer tables into ROM, so
 * no per-string copy ever costs the 2 MB DRAM budget).
 *
 * Display/input: the proven 1-bpp GPU text console from the
 * wasm2c shell (console.c), Jagpad at $F14000/$F14002.
 * ASCII only — the 8x8 font has no Japanese glyphs.
 */
#include "rabuka.h"
#include "romdata.h"
#include "console.h"
#include <string.h>

/* data.c's RB_ROM_STRINGS externs: RAM pointer variables aimed
   at the ROM pointer tables (the tables themselves live in cart
   ROM, see romdata.s). rb_load_streaming -> parse_cards /
   parse_strings read these as the live string tables. */
char **g_card_strings_rom = (char **)card_ptrs_table;
char **g_strings_rom = (char **)abstr_ptrs_table;

/* Storage read callback: serves the ROM-resident blobs. The
   engine frees what this returns (rb_load_streaming), so
   sys.c's free() ignores addresses outside its heap — cart
   ROM is never freed. */
static unsigned char *my_read(const char *path, long *out_len) {
    if (strstr(path, "cards.bin"))             { *out_len = (long)cards_bin_len;       return (unsigned char *)(uintptr_t)cards_bin; }
    if (strstr(path, "abilities_strings.bin")) { *out_len = (long)abstr_bin_len;       return (unsigned char *)(uintptr_t)abstr_bin; }
    if (strstr(path, "bytecode.bin"))          { *out_len = (long)RBKA_BYTECODE_LEN;   return (unsigned char *)(uintptr_t)RBKA_BYTECODE; }
    if (strstr(path, "gen_data.bin"))          { *out_len = (long)gen_data_jag_len;    return (unsigned char *)(uintptr_t)gen_data_jag_bin; }
    /* anything else: a 1-byte dummy so the non-NULL check passes */
    static unsigned char dummy[1];
    *out_len = 1;
    return dummy;
}

void jag_native_main(void) {
    console_init();
    console_puts("RABUKA ENGINE C\r\n");
    console_puts("ATARI JAGUAR native m68k\r\n");

    if (rb_load_streaming("rom", my_read) != 0) {
        console_puts("DATA LOAD FAIL\r\n");
        for (;;)
            console_frame_wait();
    }
    console_printf("CARDS=%lu ABIL=%lu\r\n",
                   (unsigned long)rb_num_cards(), (unsigned long)rb_num_abilities());

    /* Prove the bytecode VM runs on the 68000: decode ability #0. */
    static Ability a;
    if (rb_decode_ability(0, &a)) {
        console_puts("ABIL0: ");
        if (a.full_text) console_puts(a.full_text);
        console_puts("\r\n");
        rb_free_ability(&a);
    }

    /* Build two decks of cards that actually have abilities and play. */
    static uint32_t deck0[40], deck1[40];
    int n0 = 0, n1 = 0;
    uint32_t nc = rb_num_cards();
    for (uint32_t i = 0; i < nc && (n0 < 40 || n1 < 40); i++) {
        if (rb_card_ability_idx(i) == 0xFFFF) continue;
        if (n0 < 40) deck0[n0++] = i;
        else if (n1 < 40) deck1[n1++] = i;
    }
    console_printf("DECK P0=%d P1=%d\r\n", n0, n1);

    /* GameState is ~1.04 MB: static in .bss, never a stack local
       (the Jaguar stack is a few hundred KB). */
    static GameState g;
    rb_seed(0xCAFE);
    rb_game_init(&g, deck0, n0, deck1, n1);

    int steps = 0;
    while (g.winner < 0 && steps < 30) {
        rb_turn(&g);
        while (rb_has_pending_choice(&g)) {
            rb_resume_with_choice(&g, -1);   /* auto-skip optional picks */
        }
        steps++;
        if (steps <= 12) rb_print_state(&g);
        console_frame_wait();
    }
    console_printf("END steps=%d winner=%d\r\n", steps, g.winner);
    for (;;)
        console_frame_wait();
}

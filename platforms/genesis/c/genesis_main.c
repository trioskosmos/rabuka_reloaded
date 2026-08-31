/*
 * Genesis entry point for engine_c.
 * Boots the VDP console, loads the card/ability data straight from ROM
 * (rb_load_streaming -> no filesystem), then runs a short automated match
 * and prints the engine state to the screen.
 */
#include "rabuka.h"
#include "romdata.h"
#include "console.h"
#include <string.h>

static unsigned char *my_read(const char *path, long *out_len) {
    if (strstr(path, "cards.bin"))            { *out_len = (long)cards_bin_len;   return (unsigned char *)cards_bin; }
    if (strstr(path, "abilities_strings.bin")){ *out_len = (long)abstr_bin_len;    return (unsigned char *)abstr_bin; }
    if (strstr(path, "bytecode.bin"))         { *out_len = (long)RBKA_BYTECODE_LEN; return (unsigned char *)(uintptr_t)RBKA_BYTECODE; }
    /* gen_data.bin: offsets are already embedded in gen_data.c; serve a dummy
       so rb_load_streaming's non-NULL check passes (rb_load_gen_data is a no-op). */
    static unsigned char dummy[1];
    *out_len = 1;
    return dummy;
}

void genesis_main(void) {
    console_init();
    console_puts("RABUKA ENGINE C\r\n");
    console_puts("SEGA GENESIS / MD\r\n");

    if (rb_load_streaming("rom", my_read) != 0) {
        console_puts("DATA LOAD FAIL\r\n");
        for (;;);
    }
    console_printf("CARDS=%lu ABIL=%lu\r\n",
                   (unsigned long)rb_num_cards(), (unsigned long)rb_num_abilities());

    /* Prove the bytecode VM runs on the 68000: decode ability #0. */
    Ability a;
    if (rb_decode_ability(0, &a)) {
        console_puts("ABIL0: ");
        if (a.full_text) console_puts(a.full_text);
        console_puts("\r\n");
        rb_free_ability(&a);
    }

    /* Build two decks of cards that actually have abilities and play. */
    uint32_t deck0[40], deck1[40];
    int n0 = 0, n1 = 0;
    uint32_t nc = rb_num_cards();
    for (uint32_t i = 0; i < nc && (n0 < 40 || n1 < 40); i++) {
        if (rb_card_ability_idx(i) == 0xFFFF) continue;
        if (n0 < 40) deck0[n0++] = i;
        else if (n1 < 40) deck1[n1++] = i;
    }
    console_printf("DECK P0=%d P1=%d\r\n", n0, n1);

    GameState g;
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
    }
    console_printf("END steps=%d winner=%d\r\n", steps, g.winner);
    for (;;);
}

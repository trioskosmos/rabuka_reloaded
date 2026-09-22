/* engine_c/platforms/genesis/genesis_main.c — Genesis shell over the C engine.
   Embeds the card/string blobs (see build_genesis.sh) and runs a deterministic
   game, then renders the result on the VDP and pokes it into battery-backed
   SRAM so it can be inspected in an emulator. */
#include <stddef.h>
#include <stdint.h>
#include "rabuka.h"
#include "vdp.h"

/* ROM-embedded data blobs (created by build_genesis.sh via objcopy -b binary). */
extern const unsigned char _binary_cards_bin_start[];
extern const unsigned char _binary_cards_bin_end[];
extern const unsigned char _binary_abilities_strings_bin_start[];
extern const unsigned char _binary_abilities_strings_bin_end[];

/* Decks supplied as card-record indices (same shape as the CD-i smoke test). */
static const uint32_t deck0[20] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
};
static const uint32_t deck1[20] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
};

static void print_int(int row, int col, int v) {
    char buf[8];
    int i = 0, n = v;
    if (n == 0) buf[i++] = '0';
    else { int t = n; while (t) { buf[i++] = (char)('0' + (t % 10)); t /= 10; } }
    /* reverse */
    for (int a = 0, b = i - 1; a < b; a++, b--) { char c = buf[a]; buf[a] = buf[b]; buf[b] = c; }
    buf[i] = 0;
    vdp_puts(row, col, buf);
}

int genesis_main(void) {
    vdp_init();
    vdp_puts(0, 2, "RABUKA ENGINE");
    vdp_puts(1, 2, "GENESIS PORT");
    vdp_puts(3, 2, "LOADING DATA");

    long cards_len = (long)(_binary_cards_bin_end - _binary_cards_bin_start);
    long abstr_len = (long)(_binary_abilities_strings_bin_end - _binary_abilities_strings_bin_start);

    int rc = rb_load_rom(_binary_cards_bin_start, cards_len,
                         _binary_abilities_strings_bin_start, abstr_len);
    if (rc != 0) {
        vdp_puts(3, 2, "LOAD FAIL");
        vdp_write_result_word(0, 0xFEEE);
        vdp_write_result_word(1, (uint16_t)(rc & 0xFFFF));
        return rc;
    }
    vdp_puts(3, 2, "DATA OK");
    vdp_puts(4, 2, "RUNNING");

    GameState g;
    rb_seed(0x1234);
    rb_game_init(&g, deck0, 20, deck1, 20);

    int turns = 0;
    for (int t = 0; t < 300 && g.winner < 0; t++) {
        rb_turn(&g);
        while (rb_has_pending_choice(&g)) rb_resume_with_choice(&g, -1);
        turns = t + 1;
    }

    vdp_clear(0);
    vdp_puts(0, 2, "RABUKA ENGINE");
    vdp_puts(1, 2, "GENESIS PORT");
    vdp_puts(3, 2, "TURN");
    print_int(3, 8, turns);
    if (g.winner < 0) {
        vdp_puts(5, 2, "NO WINNER");
    } else {
        vdp_puts(5, 2, "WINNER P");
        print_int(5, 10, g.winner + 1);
    }
    vdp_puts(7, 2, "GAME OVER");

    vdp_write_result_word(0, (uint16_t)(g.winner & 0xFFFF));
    vdp_write_result_word(1, (uint16_t)(turns & 0xFFFF));
    return g.winner;
}

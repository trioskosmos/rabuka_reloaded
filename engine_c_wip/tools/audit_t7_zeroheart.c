/* Cluster T7 — do any cards carry a base_heart entry with amount 0?
 *
 * resource.rs:289 filters `if amt > 0` when collecting distinct discarded
 * colours; misc.c:669 h_bp6_pattern has NO such filter and would count a
 * zero-amount base heart. This decides whether that is reachable.
 */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    uint32_t n = rb_num_cards();
    long zero_amt = 0, zero_amt_member = 0, zero_amt_live = 0, cards = 0;
    long max_base = 0;
    long zero_in_first_base = 0;
    for (uint32_t i = 0; i < n; i++) {
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        cards++;
        if (c.num_base > (uint8_t)max_base) max_base = c.num_base;
        for (int h = 0; h < c.n_hearts && h < c.num_base; h++) {
            if (c.heart_count[h] == 0) {
                zero_amt++;
                if (rb_card_is_member((int)i)) zero_amt_member++;
                else if (rb_card_is_live((int)i)) zero_amt_live++;
                break;
            }
        }
        if (c.n_hearts > 0 && c.num_base > 0) {
            for (int h = 0; h < c.n_hearts && h < c.num_base; h++)
                if (c.heart_count[h] == 0) { zero_in_first_base++; break; }
        }
        rb_free_card(&c);
    }
    printf("cards=%ld max_num_base=%ld zero_amount_base_entries=%ld (member=%ld live=%ld) cards_with_zero=%ld\n",
           cards, max_base, zero_amt, zero_amt_member, zero_amt_live, zero_in_first_base);
    rb_unload();
    return 0;
}
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "database load failed\n"); return 1; }
    const char *nos[] = {"PL!SP-pb1-001-R","PL!-sd1-010-SD","PL!SP-bp2-006-R＋","PL!SP-bp4-024-L","LL-E-001-SD"};
    for (int i = 0; i < 5; i++) {
        int id = rb_find_card_by_no(nos[i]);
        printf("=== %s -> id=%d\n", nos[i], id);
        if (id < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)id, &c)) { printf("  decode fail\n"); continue; }
        printf("  name=%s group=%s unit=%s cost=%d blade=%d score=%d type_flags=%d n_hearts=%d num_base=%d n_abilities=%d\n",
               c.name ? c.name : "?", c.group_idx ? rb_card_string(c.group_idx) : "-",
               c.unit_idx ? rb_card_string(c.unit_idx) : "-", c.cost, c.blade, c.score, c.type_flags,
               c.n_hearts, c.num_base, rb_card_num_abilities((uint32_t)id));
        for (int h = 0; h < c.n_hearts; h++)
            printf("    heart[%d] color=%d count=%d\n", h, c.heart_color[h], c.heart_count[h]);
        printf("    abilities=%d\n", rb_card_num_abilities((uint32_t)id));
        rb_free_card(&c);
    }
    rb_unload();
    return 0;
}

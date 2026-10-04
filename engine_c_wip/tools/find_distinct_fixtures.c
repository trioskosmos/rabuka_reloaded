/* find_distinct_fixtures.c <group> -- list the printed NAME of every card in a
 * group, so a 「名前の異なる」 fixture is chosen from the corpus and not guessed. */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"

int main(int argc, char **argv)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) { fprintf(stderr, "load fail\n"); return 2; }
    const char *grp = argc > 1 ? argv[1] : "BiBi";
    int n = rb_num_cards();
    for (int id = 0; id < n; id++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)id, &c)) continue;
        const char *g = c.group_idx ? rb_get_string(c.group_idx) : NULL;
        if (rb_card_matches_group_str(id, grp) || (g && strstr(g, grp))) {
            const char *no = rb_card_string(c.card_no_idx);
            const char *nm = c.name ? c.name : "(null)";
            char nbuf[256];
            rb_card_normalize_name(nm, nbuf, sizeof nbuf);
            printf("id=%-5d no=%-22s name=%-20s normalized=%-20s member=%d live=%d cost=%d\n",
                   id, no ? no : "-", nm, nbuf, rb_card_is_member(id), rb_card_is_live(id), c.cost);
        }
        rb_free_card(&c);
    }
    rb_unload();
    return 0;
}
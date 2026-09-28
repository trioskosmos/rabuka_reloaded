#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static void dump(const char *label, const char *no)
{
    int cid = rb_find_card_by_no(no);
    printf("=== %s  '%s'  -> cid=%d\n", label, no, cid);
    if (cid < 0) { printf("    (NOT FOUND)\n"); return; }
    int n = rb_card_num_abilities((uint32_t)cid);
    printf("    nAb=%d\n", n);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers) {
            printf("    #%d trig hex:", i);
            for (const unsigned char *q = (const unsigned char *)ab.triggers; *q; q++)
                printf(" %02X", *q);
            printf("  text='%s'\n", ab.triggers);
        } else {
            printf("    #%d (no trigger)\n", i);
        }
        rb_free_ability(&ab);
    }
}

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) {
        fprintf(stderr, "load failed\n"); return 2;
    }
    dump("ren  ", "PL!SP-bp7-005-R\xEF\xBC\x8B");
    dump("ren16", "PL!SP-bp7-016-N");
    dump("haz4", "PL!SP-bp4-016-N");
    dump("kahor", "PL!SP-pb1-005-R");
    dump("sumir", "PL!SP-bp5-004-R\xEF\xBC\x8B");
    dump("shiki", "PL!SP-bp2-008-R");
    dump("kinko", "PL!SP-bp5-006-R");
    dump("keke ", "PL!SP-bp4-013-N");
    dump("natsm", "PL!SP-pb1-020-N");
    dump("chika", "PL!S-bp5-010-N");
    dump("fillo", "PL!-sd1-010-SD");
    dump("riko ", "PL!S-bp2-002-R");
    dump("kinak", "PL!SP-bp7-006-R\xEF\xBC\x8B");
    dump("opplv", "PL!N-bp1-026-L");
    rb_unload();
    return 0;
}

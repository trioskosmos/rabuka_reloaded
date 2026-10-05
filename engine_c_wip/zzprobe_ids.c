#include "rabuka.h"
#include <stdio.h>

int main(void) {
    const char *nos[] = { "PL!HS-bp5-022-L", "PL!N-sd1-028-SD",
                          "PL!S-PR-014-PR", "PL!SP-pb2-005-R",
                          "PL!N-bp5-007-R＋", "PL!SP-pb4-004-P",
                          "PL!-sd1-019-SD" };
    for (int i = 0; i < 7; i++)
        printf("%-20s -> %d\n", nos[i], rb_find_card_by_no(nos[i]));
    return 0;
}

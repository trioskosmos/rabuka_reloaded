#include "rabuka.h"

#include <stdint.h>
#include <stdio.h>

int main(void) {
    uint32_t before = rb_decode_fallback_count();
    uint32_t abilities[8] = {0};

    rb_note_decode_fallback(3, "test", "first");
    rb_note_decode_fallback(3, "test", "second");
    rb_note_decode_fallback(-1, "test", "unattributed");

    if (rb_decode_fallback_count() != before + 3) {
        fprintf(stderr, "fallback count did not increase by 3\n");
        return 1;
    }

    int recorded = rb_decode_fallback_abilities(abilities, 8);
    if (recorded < 1 || abilities[0] != 3) {
        fprintf(stderr, "fallback ability index was not recorded first\n");
        return 1;
    }

    printf("ok: decode fallback accounting\n");
    return 0;
}

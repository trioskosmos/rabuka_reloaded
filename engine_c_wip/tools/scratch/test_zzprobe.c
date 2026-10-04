#include "rabuka.h"
#include <stdio.h>
#include <string.h>

extern const uint32_t RBKA_NUM_ABILITIES;
extern uint32_t g_probe_seen, g_probe_retained, g_probe_effects;
void rb_probe_reset(void);
uint32_t rb_probe_ndropped_keys(void);
const char *rb_probe_dropped_key(uint32_t i);
uint8_t rb_probe_dropped_tag(uint32_t i);
uint32_t rb_probe_nkeys(void);
const char *rb_probe_key(uint32_t i);
uint32_t rb_probe_keycount(uint32_t i);

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "database load failed\n"); return 1; }
    rb_probe_reset();
    uint32_t before = rb_decode_fallback_count();
    uint32_t abl[64];
    for (uint32_t i = 0; i < RBKA_NUM_ABILITIES; i++) {
        Ability a;
        rb_get_ability(i, &a);
        rb_free_ability(&a);
    }
    printf("abilities=%u effects=%u seen=%u retained=%u dropped=%u distinct_keys=%u\n",
           RBKA_NUM_ABILITIES, g_probe_effects, g_probe_seen, g_probe_retained,
           rb_probe_ndropped_keys(), rb_probe_nkeys());
    printf("decode_fallbacks=%u (delta)  abilities_with_fallbacks=%d\n",
           rb_decode_fallback_count() - before,
           rb_decode_fallback_abilities(abl, 64));

    printf("\n== DROPPED (key, tag) histogram ==\n");
    for (uint32_t i = 0; i < rb_probe_ndropped_keys(); i++) {
        const char *k = rb_probe_dropped_key(i);
        uint8_t t = rb_probe_dropped_tag(i);
        int seen = 0;
        for (uint32_t j = 0; j < i; j++)
            if (rb_probe_dropped_tag(j) == t && !strcmp(rb_probe_dropped_key(j), k)) { seen = 1; break; }
        if (seen) continue;
        int cnt = 0;
        for (uint32_t j = 0; j < rb_probe_ndropped_keys(); j++)
            if (rb_probe_dropped_tag(j) == t && !strcmp(rb_probe_dropped_key(j), k)) cnt++;
        printf("  %-40s tag=%02x  n=%d\n", k, t, cnt);
    }
    printf("\n== ALL KEYS SEEN ==\n");
    for (uint32_t i = 0; i < rb_probe_nkeys(); i++)
        printf("  %-40s n=%u\n", rb_probe_key(i), rb_probe_keycount(i));
    return 0;
}

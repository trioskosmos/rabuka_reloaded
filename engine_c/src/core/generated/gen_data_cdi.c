/* engine_c/src/core/generated/gen_data_cdi.c — CD-i variant of the offset
   tables. Instead of embedding the ~24 KB tables in the ROM image, they are
   streamed from storage (gen_data.bin) by rb_load_streaming(). gen_data.bin is
   laid out little-endian ([936 x u16 deltas][5717 x u32 string offsets]); the
   SCC68070 is big-endian, so we byte-swap on load. */
#include "rabuka.h"
#include "gen_data.h"

const uint32_t RBKA_NUM_ABILITIES = RBKA_NUM_ABILITIES_C;   /* 936 */

uint16_t *g_offset_deltas = NULL;
uint32_t *g_strings_offsets = NULL;
uint16_t *g_card_ability_pairs = NULL;

int rb_load_gen_data(const unsigned char *buf, long len) {
    (void)len;
    /* offset deltas: 936 x uint16, little-endian -> big-endian */
    g_offset_deltas = (uint16_t *)buf;
    for (int i = 0; i < RBKA_NUM_ABILITIES_C; i++) {
        uint16_t v = g_offset_deltas[i];
        g_offset_deltas[i] = (uint16_t)(((v >> 8) & 0xFF) | ((v & 0xFF) << 8));
    }
    /* string offsets: 5717 x uint32, little-endian -> big-endian */
    const unsigned char *p = buf + (size_t)RBKA_NUM_ABILITIES_C * 2u;
    g_strings_offsets = (uint32_t *)p;
    for (int i = 0; i < RBKA_NUM_STRING_OFFSETS; i++) {
        uint32_t v = g_strings_offsets[i];
        v = ((v & 0xFF000000u) >> 24) | ((v & 0x00FF0000u) >> 8) |
            ((v & 0x0000FF00u) << 8)  | ((v & 0x000000FFu) << 24);
        g_strings_offsets[i] = v;
    }
    /* card/ability pairs: 4022 x uint16, little-endian -> big-endian */
    const unsigned char *q = p + (size_t)RBKA_NUM_STRING_OFFSETS * 4u;
    g_card_ability_pairs = (uint16_t *)q;
    for (int i = 0; i < RBKA_NUM_CARD_ABILITY_PAIRS * 2; i++) {
        uint16_t v = g_card_ability_pairs[i];
        g_card_ability_pairs[i] = (uint16_t)(((v >> 8) & 0xFF) | ((v & 0xFF) << 8));
    }
    return 0;
}

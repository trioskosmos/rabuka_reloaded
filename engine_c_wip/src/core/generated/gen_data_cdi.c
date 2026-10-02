/* engine_c/src/core/generated/gen_data_cdi.c — CD-i variant of the offset
   tables. Instead of embedding the ~34 KB tables in the ROM image, they are
   streamed from storage (gen_data.bin) by rb_load_streaming(). gen_data.bin is
   laid out little-endian ([985 x u16 deltas][6033 x u32 string offsets]
   [4232 x u16 card-ability pairs]); the SCC68070 is big-endian, so we byte-swap
   on load.

   ── WHAT THIS FILE OWNS ───────────────────────────────────────────────────
   rb_load_gen_data() and the three g_* offset pointers. gen_data_rom.c owns
   the same set on the host/PC target; exactly one of the two may be compiled
   (see gen_data_rom.c's PLATFORM VARIANT CONTRACT).

   This variant does NOT define RBKA_NUM_ABILITIES. It used to, from
   RBKA_NUM_ABILITIES_C, and that was a multiple-definition link error the
   moment build_cdi.sh also compiled generated/gen_data.c -- which it has to,
   because src/core/card.c and src/ability/vm.c reference RBKA_CARD_ABILITY_PAIRS,
   RBKA_STRINGS_FNV and RBKA_STRINGS_TABLE_LEN by name and only gen_data.c
   defines them. The value is identical either way (gen_data.c:5 emits the same
   number gen_data.h declares as RBKA_NUM_ABILITIES_C), so the definition now
   lives in exactly one place. */
#include "rabuka.h"
#include "gen_data.h"
#include <stdlib.h>
#include <string.h>

uint16_t *g_offset_deltas = NULL;
uint32_t *g_strings_offsets = NULL;
uint16_t *g_card_ability_pairs = NULL;

/* The three pointers above aim into this buffer. It is deliberately NOT the
   buffer the caller handed us: rb_load_streaming() frees that one on the way
   out (data.c:250, `free(buf); return 0;`), so pointing at it left every
   ability offset and string offset dangling for the rest of the process. That
   was a use-after-free, and it is silent -- rb_bc_slice() summed offsets out of
   freed heap and returned a slice into whatever the allocator handed back
   next. A private copy is the only lifetime the caller cannot revoke. */
static unsigned char *s_gen_data_owned = NULL;

/* Bytes a streamed gen_data.bin must carry for this table set, in the order
   tools/gen_gen_data_bin.py writes them:
       [RBKA_NUM_ABILITIES_C          x u16 LE]
       [RBKA_NUM_STRING_OFFSETS       x u32 LE]
       [RBKA_NUM_CARD_ABILITY_PAIRS*2 x u16 LE]
   = 985*2 + 6033*4 + 4232*2 = 1970 + 24132 + 8464 = 34566 bytes. */
#define RBKA_GEN_DATA_BLOB_BYTES \
    ((long)RBKA_NUM_ABILITIES_C * 2L + \
     (long)RBKA_NUM_STRING_OFFSETS * 4L + \
     (long)RBKA_NUM_CARD_ABILITY_PAIRS * 4L)

/* returns   0  tables ready, swapped into a buffer this file owns
            -1  buf is NULL or len <= 0 -- nothing was streamed
            -2  buf is shorter than the streamed layout -- a truncated read
            -4  no memory for the owned copy
   The single caller (data.c:249) collapses all of them into its own -6.

   `len` used to be discarded with `(void)len`, so a zero-length or short read
   walked off the end of the buffer on the assumption that the caller always
   handed over a full blob. It is checked now, against the layout this file
   decodes. */
int rb_load_gen_data(const unsigned char *buf, long len) {
    if (!buf || len <= 0) return -1;
    if (len < RBKA_GEN_DATA_BLOB_BYTES) return -2;

    unsigned char *own = (unsigned char *)malloc((size_t)RBKA_GEN_DATA_BLOB_BYTES);
    if (!own) return -4;
    memcpy(own, buf, (size_t)RBKA_GEN_DATA_BLOB_BYTES);

    /* Release the PREVIOUS owned copy only once the replacement exists, so a
       failed allocation leaves the tables already in use untouched. */
    free(s_gen_data_owned);
    s_gen_data_owned = own;

    /* offset deltas: RBKA_NUM_ABILITIES_C x uint16, little-endian -> big-endian */
    g_offset_deltas = (uint16_t *)own;
    for (int i = 0; i < RBKA_NUM_ABILITIES_C; i++) {
        uint16_t v = g_offset_deltas[i];
        g_offset_deltas[i] = (uint16_t)(((v >> 8) & 0xFF) | ((v & 0xFF) << 8));
    }
    /* string offsets: RBKA_NUM_STRING_OFFSETS x uint32, little-endian -> big-endian */
    const unsigned char *p = own + (size_t)RBKA_NUM_ABILITIES_C * 2u;
    g_strings_offsets = (uint32_t *)p;
    for (int i = 0; i < RBKA_NUM_STRING_OFFSETS; i++) {
        uint32_t v = g_strings_offsets[i];
        v = ((v & 0xFF000000u) >> 24) | ((v & 0x00FF0000u) >> 8) |
            ((v & 0x0000FF00u) << 8)  | ((v & 0x000000FFu) << 24);
        g_strings_offsets[i] = v;
    }
    /* card/ability pairs: RBKA_NUM_CARD_ABILITY_PAIRS*2 x uint16, LE -> BE */
    const unsigned char *q = p + (size_t)RBKA_NUM_STRING_OFFSETS * 4u;
    g_card_ability_pairs = (uint16_t *)q;
    for (int i = 0; i < RBKA_NUM_CARD_ABILITY_PAIRS * 2; i++) {
        uint16_t v = g_card_ability_pairs[i];
        g_card_ability_pairs[i] = (uint16_t)(((v >> 8) & 0xFF) | ((v & 0xFF) << 8));
    }
    return 0;
}

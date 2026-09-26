/* Host/PC (non-CD-i) side of the generated data tables.
   Kept OUT of src/core/generated/gen_data.c on purpose: gen_data.c is
   regenerated wholesale by tools/gen_from_rs.py, so anything hand-written in
   it is destroyed on the next `make regen`. */
#include "rabuka.h"
#include "gen_data.h"

extern const uint16_t RBKA_OFFSET_DELTAS[];
extern const uint32_t RBKA_STRINGS_OFFSETS[];
extern const uint16_t RBKA_CARD_ABILITY_PAIRS[];

/* The CD-i/ROM path points these at the streamed blob; on the host build they
   alias the embedded arrays. */
uint16_t *g_offset_deltas = (uint16_t *)RBKA_OFFSET_DELTAS;
uint32_t *g_strings_offsets = (uint32_t *)RBKA_STRINGS_OFFSETS;
uint16_t *g_card_ability_pairs = (uint16_t *)RBKA_CARD_ABILITY_PAIRS;

int rb_load_gen_data(const unsigned char *buf, long len) {
    /* The tables are already embedded, so there is nothing to stream in. The
        blob is still validated so a null/empty buffer fails loudly instead of
        silently leaving the offset tables unusable. */
    if (!buf || len <= 0) return -1;
    return 0;
}

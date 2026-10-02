/* Host/PC (native-endian, tables-embedded-in-the-image) side of the generated
   data tables. Kept OUT of src/core/generated/gen_data.c on purpose: gen_data.c
   is regenerated wholesale by tools/gen_from_rs.py, so anything hand-written in
   it is destroyed on the next `make regen`.

   ── PLATFORM VARIANT CONTRACT ────────────────────────────────────────────
   rb_load_gen_data() and the three g_* offset pointers below must have EXACTLY
   ONE definition per target:

     src/core/gen_data_rom.c            THIS FILE. Native-endian host/PC.
                                        Tables are compiled in; nothing is
                                        streamed, so rb_load_gen_data only has
                                        to validate what the caller hands it.

     src/core/generated/gen_data_cdi.c  Big-endian SCC68070 (CD-i). Streams
                                        gen_data.bin off storage and
                                        byte-swaps it into place, because the
                                        ROM image cannot afford ~34 KB of
                                        tables it would otherwise embed.

   Compiling BOTH is a multiple-definition link error on rb_load_gen_data, on
   g_offset_deltas / g_strings_offsets / g_card_ability_pairs, and on
   RBKA_NUM_ABILITIES (gen_data_cdi.c:9 redefines the same const that
   generated/gen_data.c:5 already defines). The host Makefile therefore lists
   this file and NOT gen_data_cdi.c; platforms/cdi/build_cdi.sh lists
   gen_data_cdi.c and NOT this file. Do not "just add" the other one -- pick the
   variant that matches the target's endianness and storage model.

   The array sizes below come from gen_data.h's macros, which tools/
   gen_from_rs.py emits from the SAME generator run that writes gen_data.c.
   Declaring the arrays with those sizes turns a stale hand-written count into a
   compile error instead of a silently truncated alias. */
#include "rabuka.h"
#include "gen_data.h"

extern const uint16_t RBKA_OFFSET_DELTAS[RBKA_NUM_ABILITIES_C];
extern const uint32_t RBKA_STRINGS_OFFSETS[RBKA_NUM_STRING_OFFSETS];
extern const uint16_t RBKA_CARD_ABILITY_PAIRS[RBKA_NUM_CARD_ABILITY_PAIRS * 2];

/* The CD-i/ROM path points these at the streamed blob; on the host build they
   alias the embedded arrays.

   g_card_ability_pairs currently has NO reader anywhere in the tree:
   src/core/card.c indexes RBKA_CARD_ABILITY_PAIRS by name (card.c:164, 170,
   173) instead of through this pointer. It is defined here anyway because
   gen_data_cdi.c defines it too and the host must export the same symbol set.
   Consequence for CD-i: card.c reads the array by name, but build_cdi.sh does
   not compile gen_data.c, so that build has no RBKA_CARD_ABILITY_PAIRS at all
   -- and even if it did, the streamed+swapped table would go unread. Until
   card.c reads through this pointer, gen_data_cdi.c's pairs section is loaded,
   swapped, and then never looked at. */
uint16_t *g_offset_deltas = (uint16_t *)RBKA_OFFSET_DELTAS;
uint32_t *g_strings_offsets = (uint32_t *)RBKA_STRINGS_OFFSETS;
uint16_t *g_card_ability_pairs = (uint16_t *)RBKA_CARD_ABILITY_PAIRS;

/* Bytes a streamed gen_data.bin must carry for this table set, in the order
   tools/gen_gen_data_bin.py writes them:
       [RBKA_NUM_ABILITIES_C          x u16 LE]
       [RBKA_NUM_STRING_OFFSETS       x u32 LE]
       [RBKA_NUM_CARD_ABILITY_PAIRS*2 x u16 LE]
   = 985*2 + 6034*4 + 4232*2 = 1970 + 24136 + 8464 = 34570 bytes. */
#define RBKA_GEN_DATA_BLOB_BYTES \
    ((long)RBKA_NUM_ABILITIES_C * 2L + \
     (long)RBKA_NUM_STRING_OFFSETS * 4L + \
     (long)RBKA_NUM_CARD_ABILITY_PAIRS * 4L)

/* Little-endian readers. Written by hand rather than via memcmp against the
   embedded arrays so that (a) no libc dependency is added to a file the ROM
   builds compile with -ffreestanding -nostdlib, and (b) the comparison is
   correct on a big-endian host too: the embedded arrays are native-order
   integers, the streamed blob is always little-endian, and these decode the
   blob explicitly rather than relying on either side's byte order. */
static int rb_le16_is(const unsigned char *p, uint16_t v) {
    return (uint16_t)((uint32_t)p[0] | ((uint32_t)p[1] << 8)) == v;
}

static int rb_le32_is(const unsigned char *p, uint32_t v) {
    return ((uint32_t)p[0] | ((uint32_t)p[1] << 8) |
            ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24)) == v;
}

/* Populate the offset tables from a streamed gen_data.bin.

   The tables are already embedded, so there is nothing to stream IN: the
   pointers above alias the compiled-in arrays and that is the whole point of
   this variant. The buffer the caller handed us is not simply ignored, though.
   rb_load_streaming() streams bytecode.bin alongside it and keeps that buffer
   live as g_bc (data.c:243), so a gen_data.bin from a different generator run
   would silently pair NEW bytecode with COMPILED-IN OLD offsets. Because the
   interned string pool is UNSEPARATED, that skew does not raise an error -- it
   returns a byte splice of two neighbouring entries (`cost_limit` ->
   `cost_limitc`, `left_side` -> `eft_side`), which is the exact failure
   tools/regen_all.sh records having happened twice. So cross-check the blob the
   caller claims to have streamed against what is compiled in, and fail loudly
   instead.

   The host build has no cheap digest of the concatenated arrays, so this walks
   the 34,570 bytes. It runs once per rb_load_streaming(), not per decode.

   returns   0  tables ready (embedded, and the streamed blob agrees)
            -1  buf is NULL or len <= 0
            -2  buf is shorter than the streamed layout -- truncated blob
            -3  blob is full length but its bytes differ from the compiled-in
                tables -- a different generator run, i.e. exactly the skew above
   The single caller (data.c:249) collapses all three into its own -6. */
int rb_load_gen_data(const unsigned char *buf, long len) {
    if (!buf || len <= 0) return -1;
    if (len < RBKA_GEN_DATA_BLOB_BYTES) return -2;

    const unsigned char *p = buf;
    for (uint32_t i = 0; i < RBKA_NUM_ABILITIES_C; i++, p += 2)
        if (!rb_le16_is(p, RBKA_OFFSET_DELTAS[i])) return -3;
    for (uint32_t i = 0; i < RBKA_NUM_STRING_OFFSETS; i++, p += 4)
        if (!rb_le32_is(p, RBKA_STRINGS_OFFSETS[i])) return -3;
    for (uint32_t i = 0; i < (uint32_t)RBKA_NUM_CARD_ABILITY_PAIRS * 2u; i++, p += 2)
        if (!rb_le16_is(p, RBKA_CARD_ABILITY_PAIRS[i])) return -3;

    return 0;
}
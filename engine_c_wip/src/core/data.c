#include "rabuka.h"
#include "gen_data.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* combined bytecode blob defined in bytecode_blob.c */
extern const unsigned char RBKA_BYTECODE[];
extern const uint32_t RBKA_BYTECODE_LEN;

/* generated tables */
extern const uint32_t RBKA_NUM_ABILITIES;
/* These point at the offset tables. For the PC/host build they alias the
   embedded arrays (see gen_data.c); for bare-metal builds they are populated
   from gen_data.bin streamed off storage (see gen_data_cdi.c). */
extern uint16_t *g_offset_deltas;
extern uint32_t *g_strings_offsets;

#ifdef RB_ROM_STRINGS
/* Pointer tables emitted by pack.py into ROM (see romdata.s / *.inc). */
extern char **g_card_strings_rom;
extern char **g_strings_rom;
#endif

/* ── globals ── */
static unsigned char *g_cards_blob = NULL;
static long          g_cards_len = 0;
static uint32_t      g_num_cards = 0;
static char        **g_card_strings = NULL;   /* null-terminated copies */
static uint32_t      g_num_card_strings = 0;  /* entries in g_card_strings */
static uint32_t     *g_card_off = NULL;       /* (num_cards+1) offsets */
static unsigned char *g_card_data = NULL;     /* base of card records */
static uint32_t      g_card_alias_plus_one[RB_MAX_CARD_IDS];
static uint32_t      g_next_copy_id;

static unsigned char *g_abstr_blob = NULL;    /* abilities_strings.bin */
static long           g_abstr_len = 0;
static char         **g_strings = NULL;       /* null-terminated copies */

static unsigned char *g_bc = NULL;            /* concatenated ability bytecode */
static uint32_t       g_bc_len = 0;

static uint32_t le32(const unsigned char *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}
static uint16_t le16(const unsigned char *p) {
    return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

static unsigned char *read_file(const char *path, long *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    unsigned char *buf = malloc(n ? n : 1);
    if (!buf) { fclose(f); return NULL; }
    if (fread(buf, 1, n, f) != (size_t)n) { free(buf); fclose(f); return NULL; }
    fclose(f);
    *out_len = n;
    return buf;
}

uint32_t rb_num_cards(void)   { return g_num_cards; }
uint32_t rb_num_abilities(void){ return RBKA_NUM_ABILITIES; }

const char *rb_get_string(uint32_t idx) {
    static const char empty[] = "";
    uint32_t n = RBKA_NUM_STRING_OFFSETS ? (RBKA_NUM_STRING_OFFSETS - 1) : 0;
    if (g_strings == NULL || idx >= n) return empty;
    return g_strings[idx];
}

/* ── parse cards.bin from an in-memory blob ── */
static int parse_cards(const unsigned char *blob, long len) {
    g_cards_blob = (unsigned char *)blob;
    g_cards_len = len;
    if (g_cards_len < 10 || memcmp(g_cards_blob, "CARD", 4) != 0) return -2;
    g_num_cards = le16(g_cards_blob + 4);
    uint32_t strtab_len = le32(g_cards_blob + 6);
    const unsigned char *strtab = g_cards_blob + 10;
    const unsigned char *p = strtab;

#ifdef RB_ROM_STRINGS
    /* Pointer array lives in ROM (card_ptrs.inc); no arena used. */
    g_card_strings = g_card_strings_rom;
    g_num_card_strings = g_num_cards;
    (void)p; (void)strtab_len;
#else
    size_t cap = 256, n = 0;
    g_card_strings = malloc(cap * sizeof(char *));
    while ((long)(p - strtab) < (long)strtab_len) {
        uint16_t slen = le16(p); p += 2;
        if (n + 1 > cap) { cap *= 2; g_card_strings = realloc(g_card_strings, cap * sizeof(char *)); }
        char *s = malloc(slen + 1);
        memcpy(s, p, slen); s[slen] = 0;
        g_card_strings[n++] = s;
        p += slen;
    }
    g_card_strings = realloc(g_card_strings, (n ? n : 1) * sizeof(char *));
    g_num_card_strings = (uint32_t)n;
#endif

    const unsigned char *lentab = strtab + strtab_len;
    const unsigned char *cardbase = lentab + g_num_cards;
    g_card_off = malloc((g_num_cards + 1) * sizeof(uint32_t));
    uint32_t off = 0;
    for (uint32_t i = 0; i < g_num_cards; i++) {
        g_card_off[i] = off;
        off += lentab[i];
    }
    g_card_off[g_num_cards] = off;
    g_card_data = (unsigned char *)cardbase;
    return 0;
}

/* ── parse abilities_strings.bin from an in-memory blob ── */
static int parse_strings(const unsigned char *blob, long len) {
    g_abstr_blob = (unsigned char *)blob;
    g_abstr_len = len;
    uint32_t n = RBKA_NUM_STRING_OFFSETS ? (RBKA_NUM_STRING_OFFSETS - 1) : 0;
#ifdef RB_ROM_STRINGS
    /* Pointer array lives in ROM (abstr_ptrs.inc); no arena used. */
    g_strings = g_strings_rom;
#else
    g_strings = malloc((n ? n : 1) * sizeof(char *));
    for (uint32_t i = 0; i < n; i++) {
        uint32_t a = g_strings_offsets[i];
        uint32_t b = g_strings_offsets[i + 1];
        uint32_t sl = b - a;
        char *s = malloc(sl + 1);
        memcpy(s, g_abstr_blob + a, sl); s[sl] = 0;
        g_strings[i] = s;
    }
#endif
    (void)n;
    return 0;
}

#ifdef RB_ROM_STRINGS
/* Genesis/ROM build: the card & ability string blobs are embedded in ROM. We
   build two RAM pointer arrays (g_card_strings_rom, g_strings_rom) that index
   directly into those blobs — no per-string copy, so the whole game fits in the
   64 KB work RAM. The engine's parse_cards/parse_strings treat these as the live
   string tables (see the #ifdef RB_ROM_STRINGS branches above). */
int rb_load_rom(const unsigned char *cards_blob, long cards_len,
                const unsigned char *abstr_blob, long abstr_len) {
    /* ── card-name pointer table ── */
    if (cards_len < 10 || memcmp(cards_blob, "CARD", 4) != 0) return -2;
    g_num_cards = le16(cards_blob + 4);
    uint32_t strtab_len = le32(cards_blob + 6);
    const unsigned char *strtab = cards_blob + 10;
    const unsigned char *p = strtab;
    size_t n = 0;
    while ((long)(p - strtab) < (long)strtab_len) {
        uint16_t slen = le16(p); p += 2 + slen; n++;
    }
    g_card_strings_rom = (char **)malloc((n ? n : 1) * sizeof(char *));
    if (!g_card_strings_rom) return -7;
    p = strtab; n = 0;
    while ((long)(p - strtab) < (long)strtab_len) {
        uint16_t slen = le16(p); p += 2;
        g_card_strings_rom[n++] = (char *)p;
        p += slen;
    }
    if (parse_cards(cards_blob, cards_len) != 0) return -2;

    /* ── ability-string pointer table ── */
    g_abstr_blob = (unsigned char *)abstr_blob;
    g_abstr_len = abstr_len;
    uint32_t sn = RBKA_NUM_STRING_OFFSETS ? (RBKA_NUM_STRING_OFFSETS - 1) : 0;
    g_strings_rom = (char **)malloc((sn ? sn : 1) * sizeof(char *));
    if (!g_strings_rom) return -8;
    for (uint32_t i = 0; i < sn; i++)
        g_strings_rom[i] = (char *)(abstr_blob + g_strings_offsets[i]);
    if (parse_strings(abstr_blob, abstr_len) != 0) return -3;

    g_bc = (unsigned char *)RBKA_BYTECODE;   /* ROM-embedded bytecode blob */
    g_bc_len = RBKA_BYTECODE_LEN;
    return 0;
}
#endif

/* ── load cards.bin ── */
static int load_cards(const char *dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/cards.bin", dir);
    g_cards_blob = read_file(path, &g_cards_len);
    if (!g_cards_blob) return -1;
    if (rb_ability_debug_enabled()) fprintf(stderr, "[dbg] cards.bin loaded len=%ld\n", g_cards_len);
    return parse_cards(g_cards_blob, g_cards_len);
}

/* ── load abilities_strings.bin + build null-terminated copies ── */
static int load_strings(const char *dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/abilities_strings.bin", dir);
    g_abstr_blob = read_file(path, &g_abstr_len);
    if (!g_abstr_blob) return -1;
    return parse_strings(g_abstr_blob, g_abstr_len);
}

/* ── point at the combined bytecode blob (static, no free) ── */
static int load_bytecode(void) {
    g_bc = (unsigned char *)RBKA_BYTECODE;
    g_bc_len = RBKA_BYTECODE_LEN;
    return 0;
}

int rb_load(const char *data_dir) {
    if (load_cards(data_dir) != 0) { fprintf(stderr, "load_cards failed\n"); return -1; }
    if (load_strings(data_dir) != 0) { fprintf(stderr, "load_strings failed\n"); return -1; }
    if (load_bytecode() != 0) { fprintf(stderr, "load_bytecode failed\n"); return -1; }
    rb_card_copy_pool_reset();
    return 0;
}

int rb_load_streaming(const char *dir,
                      unsigned char *(*read_fn)(const char *path, long *out_len)) {
    /* Bare-metal hook: caller supplies read_fn that streams from ROM/CD/flash.
       When read_fn is provided we read both data blobs through it and parse
       them in place — no fopen, no host filesystem required. */
    if (read_fn) {
        char path[1024];
        long n = 0;
        unsigned char *buf;
        snprintf(path, sizeof(path), "%s/cards.bin", dir);
        buf = read_fn(path, &n);
        if (!buf) return -1;
        if (parse_cards(buf, n) != 0) { free(buf); return -2; }
        free(buf);
        snprintf(path, sizeof(path), "%s/abilities_strings.bin", dir);
        buf = read_fn(path, &n);
        if (!buf) return -1;
        if (parse_strings(buf, n) != 0) { free(buf); return -3; }
        free(buf);
        /* Ability bytecode: stream it from storage too (don't embed 90+ KB in
           the ROM image). Keep the buffer live — g_bc points at it. */
        snprintf(path, sizeof(path), "%s/bytecode.bin", dir);
        buf = read_fn(path, &n);
        if (!buf) return -4;
        g_bc = buf;        /* owned, live for the match */
        g_bc_len = (uint32_t)n;
        /* Offset tables: stream from storage too (don't embed ~24 KB). */
        snprintf(path, sizeof(path), "%s/gen_data.bin", dir);
        buf = read_fn(path, &n);
        if (!buf) return -5;
        if (rb_load_gen_data(buf, n) != 0) { free(buf); return -6; }
        free(buf);
        return 0;
    }
    /* No read_fn supplied: bare-metal ports must provide one. The hosted
       PC build calls rb_load() directly instead of via this path. */
    return -1;
}

void rb_unload(void) {
    /* minimal: free top-level blobs; per-card/ability freed by callers */
    free(g_cards_blob); g_cards_blob = NULL;
    free(g_abstr_blob); g_abstr_blob = NULL;
    free(g_card_off);   g_card_off = NULL;
    if (g_card_strings) { /* leak individual strings on unload is acceptable for now */ free(g_card_strings); g_card_strings = NULL; }
    if (g_strings) { free(g_strings); g_strings = NULL; }
    /* g_bc points at the static RBKA_BYTECODE blob; do not free */
    g_bc = NULL;
    /* g_card_data points INTO g_cards_blob and the blob is freed above, so the
     * counter must go with it: leaving g_num_cards / g_card_data / g_num_card_strings
     * populated made rb_card_record() and rb_card_string() dereference freed
     * memory between rb_unload() and the next rb_load(). rb_load() resets all
     * three (parse_cards/parse_strings), so nothing that re-loads is affected. */
    g_cards_len = 0;
    g_abstr_len = 0;
    g_num_cards = 0;
    g_num_card_strings = 0;
    g_card_data = NULL;
    memset(g_card_alias_plus_one, 0, sizeof(g_card_alias_plus_one));
    g_next_copy_id = 0;
}

/* expose internal card data accessors for cards.c / vm.c */
int rb_register_card_copy(int copy_id, int template_id) {
    if (copy_id <= 0 || copy_id >= RB_MAX_CARD_IDS || template_id < 0 ||
        (uint32_t)template_id >= g_num_cards) return 0;
    g_card_alias_plus_one[copy_id] = (uint32_t)template_id + 1;
    return 1;
}

void rb_card_copy_pool_reset(void)
{
    memset(g_card_alias_plus_one, 0, sizeof(g_card_alias_plus_one));
    g_next_copy_id = g_num_cards;
}

int rb_create_card_copy(int template_id)
{
    if (template_id < 0 || (uint32_t)template_id >= g_num_cards) return -1;
    if (g_next_copy_id == 0) g_next_copy_id = g_num_cards;
    if (g_next_copy_id >= RB_MAX_CARD_IDS) return -1;
    int copy_id = (int)g_next_copy_id++;
    if (!rb_register_card_copy(copy_id, template_id)) {
        g_next_copy_id--;
        return -1;
    }
    return copy_id;
}
static uint32_t rb_card_template_id(uint32_t i) {
    if (i < g_num_cards) return i;
    if (i >= RB_MAX_CARD_IDS) return i;
    return g_card_alias_plus_one[i] ? g_card_alias_plus_one[i] - 1 : i;
}
const unsigned char *rb_card_record(uint32_t i) {
    i = rb_card_template_id(i);
    if (i >= g_num_cards) return NULL;
    return g_card_data + g_card_off[i];
}
uint32_t rb_card_record_len(uint32_t i) {
    i = rb_card_template_id(i);
    if (i >= g_num_cards) return 0;
    return g_card_off[i + 1] - g_card_off[i];
}
/* Card NAME string index (record offset 2, card_binary.rs:120). The name is
 * misleading -- the card_no index lives at offset 0 and is read directly by
 * card_no_at() below -- but the out-of-range value must still be the SAME
 * "absent" marker compile_cards.py writes for a missing field
 * (cards/compile_cards.py:85 and :89 use 0xFFFF for no `unit` / no `ability`),
 * not 0. Index 0 is a REAL interned string, so the old `return 0` made
 * `rb_card_string(rb_card_stridx(id))` hand back string 0 for any unknown id --
 * a silent wrong name rather than an empty one, which is the one thing Rust's
 * `get_str(...)` on the same field cannot produce. */
uint16_t rb_card_stridx(uint32_t i) {
    i = rb_card_template_id(i);
    if (i >= g_num_cards) return 0xFFFF;
    return le16(rb_card_record(i) + 2); /* name_idx at offset 2 */
}
uint16_t rb_card_ability_idx(uint32_t i) {
    i = rb_card_template_id(i);
    if (i >= g_num_cards) return 0xFFFF;
    return le16(rb_card_record(i) + 16); /* ability_idx at offset 16 */
}
const char *rb_card_string(uint16_t idx) {
    /* Group/unit/series indices come from the same cards.bin string table, but
        a malformed or foreign record could carry an index past the end of the
        table. Clamp to the real size so callers (card_matches_group_str,
        condition evaluators, etc.) never dereference an out-of-bounds slot. */
    if (g_card_strings && idx < g_num_card_strings) return g_card_strings[idx];
    return "";
}

/* ── CANONICAL UNIT ACCESSOR (Card::unit, card_binary.rs:175-182) ───────────
   compile_cards.py writes the card's `unit` field at record offset 8 and uses
   0xFFFF as the "absent" marker (cards.json has no `unit` key for 441 of 2,639
   cards), which is exactly the None that card_binary.rs builds. Before this
   accessor the only ways to reach the unit were `Card.unit_idx` +
   `rb_card_string` — which silently yields "" for both "absent" and "index
   0xFFFF", and which callers under ability/ were not using consistently.

   Unlike `group` (which is the EMPTY STRING for every one of the 2,639 cards
   and is therefore only usable through rb_card_group_name, the series-derived
   accessor in core/card.c), `unit` is real data on 2,198 cards and names the
   unit the printed text brackets as 『X』. Every group-name comparison must be
   reachable from here. Returns a pointer into the card string table (or the
   shared "" literal), so it is stable for the process lifetime. */
const char *rb_card_unit_name(int card_id) {
    if (card_id < 0) return "";
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    if (!r || rb_card_record_len((uint32_t)card_id) < 10) return "";
    uint16_t unit_idx = le16(r + 8);
    if (unit_idx == 0xFFFF) return "";       /* compile_cards.py's "no unit" */
    return rb_card_string(unit_idx);
}

/* ability slice access for vm.c */
const unsigned char *rb_bc_slice(uint32_t idx, uint32_t *out_len) {
    if (idx >= RBKA_NUM_ABILITIES) return NULL;
    uint32_t start = 0;
    for (uint32_t i = 0; i < idx; i++) start += g_offset_deltas[i];
    uint32_t len = g_offset_deltas[idx];
    *out_len = len;
    if (start + len > g_bc_len) return NULL;
    return g_bc + start;
}

/* ── CardDatabase::get_card_id (card.rs:576-629) ──────────────────────────────
   The lookup is a five-step chain, reproduced here in Rust's order:

     1. exact match against the stored card_no
     2. normalized match (uppercase, fullwidth → halfwidth)
     3. split at the LAST dash into (base, requested_rarity) and try
        a. the exact base+rarity key, then
        b. the equivalent rarities (R+ / R2 / R＋ all name the R＋ print), then
        c. ANY rarity of that base
     4. strip the rarity suffix and retry the bare base
     5. substring contains, on the raw keys

   Steps 3c and 5 take the LOWEST matching key, mirroring Rust's
   `min_by(|a, b| a.0.cmp(b.0))`. Rust spells that rule out because taking the
   first HashMap entry made get_card_id non-deterministic; here the records
   happen to be stored sorted by card_no so the first match would coincide, but
   the comparison is kept so the two cannot drift apart if that ever changes.
   ── */

/* The longest card_no in cards.bin is 25 bytes ("LL-PR-007-PRLoveLive!Days"),
   so a folded copy of a STORED key always fits this stack buffer. The lookup
   KEY is a different matter: it is derived from the caller's string, and
   sizing it from strlen(card_no) is what stops a long key from being
   silently truncated — a truncated key would make the base/rarity split below
   run on a different string than Rust's parse_base_and_rarity sees, and "any
   rarity of this base" would then fire where Rust returns None. */
#define CARD_NO_FOLD_MAX 256
/* Enough for the common case without touching the heap; longer keys fall back
   to an exact-size allocation. */
#define CARD_NO_STACK_SZ 512

static const char *card_no_at(uint32_t i) {
    const unsigned char *r = rb_card_record(i);
    if (!r) return NULL;
    return rb_card_string(le16(r));
}

static int card_no_exact(const char *query) {
    for (uint32_t i = 0; i < g_num_cards; i++) {
        const char *no = card_no_at(i);
        if (no && !strcmp(no, query)) return (int)i;
    }
    return -1;
}

/* Compare a stored card_no against an ALREADY normalized key. */
static int card_no_folded_eq(const char *stored, const char *folded) {
    char candidate[CARD_NO_FOLD_MAX];
    rb_card_normalize_no(stored, candidate, sizeof(candidate));
    return strcmp(candidate, folded) == 0;
}

static int card_no_folded_lookup(const char *folded) {
    for (uint32_t i = 0; i < g_num_cards; i++) {
        const char *no = card_no_at(i);
        if (no && card_no_folded_eq(no, folded)) return (int)i;
    }
    return -1;
}

/* Step 3c: the lowest normalized card_no starting with `prefix`. */
static int card_no_folded_prefix_lowest(const char *prefix) {
    int best = -1;
    char best_no[CARD_NO_FOLD_MAX];
    size_t prefix_len = strlen(prefix);
    for (uint32_t i = 0; i < g_num_cards; i++) {
        const char *no = card_no_at(i);
        if (!no) continue;
        char candidate[CARD_NO_FOLD_MAX];
        rb_card_normalize_no(no, candidate, sizeof(candidate));
        if (strncmp(candidate, prefix, prefix_len) != 0) continue;
        if (best < 0 || strcmp(candidate, best_no) < 0) {
            best = (int)i;
            /* bounded copy: the tail of candidate is untouched padding */
            memcpy(best_no, candidate, strlen(candidate) + 1);
        }
    }
    return best;
}

/* Step 5: the lowest RAW card_no containing either needle. */
static int card_no_contains_lowest(const char *needle, const char *narrow) {
    int best = -1;
    const char *best_no = NULL;
    for (uint32_t i = 0; i < g_num_cards; i++) {
        const char *no = card_no_at(i);
        if (!no) continue;
        if (!strstr(no, needle) && !strstr(no, narrow)) continue;
        if (!best_no || strcmp(no, best_no) < 0) { best = (int)i; best_no = no; }
    }
    return best;
}

/* Steps 2-5 of get_card_id. `scratch` holds four NUL-terminated buffers laid
   out back to back — normalized, base, variant, narrow — and must be at least
   card_no_scratch_bytes(len) bytes. Every buffer is sized from the caller's
   string, never a fixed one, so no step ever sees a truncated key.

   THE LAYOUT, and why `-Wrestrict` on the three snprintf calls below is a
   false positive that must NOT be silenced. Byte offsets from `scratch`, for
   any len >= 0 (offsets are written in terms of len, not of the data, so this
   is a property of the arithmetic and not of any particular key):

       normalized  [0,        len      ]   len+1 bytes, NUL at len
       base        [len+1,    2*len+1  ]   len+1 bytes, NUL at len+1+base_len
       variant     [2*len+2,  3*len+3  ]   len+2 bytes, NUL at <= 3*len+3
       narrow      [3*len+4,  6*len+7  ]   3*len+4 bytes, NUL at <= 6*len+7

   `base` ends at 2*len+1 and `variant` begins at 2*len+2 — a one-byte gap, for
   EVERY len, including len == 0. `base` cannot be longer than `len` because
   rb_card_normalize_no (card.c:223) never expands: it copies n bytes unchanged
   or writes exactly 1 byte for a folded code point, so strlen(normalized) <=
   len, hence base_len = dash - normalized <= len. `snprintf(variant, len+2,
   ...)` is itself bounded to variant's own slot, so it cannot reach back into
   `base` either. And `narrow` begins past variant's last writable byte, so no
   step's writes touch a buffer another step still reads.

   GCC's -Wrestrict reasons about the whole OBJECT (`scratch`), not about
   sub-ranges of it: `base` and `variant` are both derived from the same
   parameter, so it warns without asking whether the derived ranges intersect.
   That heuristic is exactly right for `memcpy(dst, dst+n, k)` and exactly wrong
   here. Measured: it fires under the project's own `-Wall` (gcc 15, data.c
   :535/:563/:570), not only under -Wextra.

   Do NOT silence it. There is no in-source way to tell GCC "same object, but
   disjoint sub-ranges" — it only compares object identity — so the options are
   a `#pragma GCC diagnostic` (destroys the diagnostic for exactly the range
   arithmetic documented above) or hand-rolling the join with memcpy (replaces
   a well-defined snprintf with 10 lines of truncation logic on the hottest
   lookup path in the file). Neither is worth it: the code is correct for every
   len, and the warning is the only tripwire that would notice a future edit to
   the offsets making it incorrect. Noise that catches a real corruption class
   is cheaper than silence that cannot. */
static size_t card_no_scratch_bytes(size_t len) {
    return 3 * (len + 1) + 2 + 3 * len + 4 + 1;
}

static int card_no_resolve_chain(const char *card_no, char *scratch, size_t len) {
    char *normalized = scratch;
    char *base       = scratch + (len + 1);
    char *variant    = base + (len + 1);
    char *narrow     = variant + (len + 2);

    /* 2. normalized (rb_card_normalize_no is the card.rs:634 port; folding a
       3-byte fullwidth character must write exactly ONE destination byte) */
    rb_card_normalize_no(card_no, normalized, len + 1);
    int id = card_no_folded_lookup(normalized);
    if (id >= 0) return id;

    /* 3. base + requested rarity */
    const char *dash = strrchr(normalized, '-');
    if (dash) {
        size_t base_len = (size_t)(dash - normalized);
        memcpy(base, normalized, base_len);
        base[base_len] = 0;
        const char *requested = dash + 1;
        int n;

        /* 3a. exact base + requested rarity */
        n = snprintf(variant, len + 2, "%s-%s", base, requested);
        if (n > 0 && (size_t)n < len + 2) {
            id = card_no_folded_lookup(variant);
            if (id >= 0) return id;
        }
        /* 3b. equivalent rarities. Rust builds `format!("{base}-{eq}")` with the
           CANONICAL rarity — which keeps the fullwidth ＋ ("P＋"/"R＋"/"L＋"/"N＋"
           /"PR＋", card.rs:533-547) — and looks THAT key up in
           `normalized_no_to_id`. Every key in that map went through
           normalize_card_no, which folds ＋ to '+', so no key in it can contain
           a fullwidth ＋: this arm cannot hit, in Rust or here. Port it as
           written, against `variant`, rather than "fixing" it by folding first.

           The folded form is what used to be looked up here, and it was built
           into `variant + n + 1` with `out_sz = len - n`. `n` is the length of
           `base` + '-' + the canonical, and `len` is the length of the CALLER's
           string, whose rarity suffix is what normalized DOWN to `requested`.
           Every input that reaches this arm has a fullwidth suffix in the
           caller's string (an already-ASCII suffix fails the `n < len + 2`
           guard above, so 3b never runs for it), and a fullwidth suffix has
           exactly as many bytes as the canonical. So n == len, `out_sz` is 0,
           rb_card_normalize_no returns without writing (card.c:224), and
           `folded` — uninitialised scratch — was handed to card_no_folded_lookup
           as the lookup key. With real card data (max card_no 25 bytes,
           verified against cards/cards.json) that is EVERY call to this arm. */
        char canonical[32];
        if (base_len + 1 < sizeof(canonical) &&
            rb_card_equivalent_rarity(requested, canonical, sizeof(canonical))) {
            n = snprintf(variant, len + 2, "%s-%s", base, canonical);
            if (n > 0 && (size_t)n < len + 2) {
                id = card_no_folded_lookup(variant);
                if (id >= 0) return id;
            }
        }
        /* 3c. any rarity of this base, lowest key wins */
        n = snprintf(variant, len + 2, "%s-", base);
        if (n > 0 && (size_t)n < len + 2) {
            id = card_no_folded_prefix_lowest(variant);
            if (id >= 0) return id;
        }
        /* 4. strip the rarity suffix and retry the bare base */
        id = card_no_folded_lookup(base);
        if (id >= 0) return id;
    }

    /* 5. contains step on the raw keys, narrow form widens '+' to '＋' */
    size_t w = 0;
    for (const char *p = normalized; *p; p++) {
        if (*p == '+') { memcpy(narrow + w, "\xef\xbc\x8b", 3); w += 3; }
        else narrow[w++] = *p;
    }
    narrow[w] = 0;
    return card_no_contains_lowest(normalized, narrow);
}

int rb_find_card_by_no(const char *card_no) {
    if (!card_no || !g_cards_blob) return -1;

    /* 1. exact */
    int id = card_no_exact(card_no);
    if (id >= 0) return id;

    size_t len = strlen(card_no);
    size_t need = card_no_scratch_bytes(len);
    char stack_scratch[CARD_NO_STACK_SZ];
    if (need <= sizeof(stack_scratch))
        return card_no_resolve_chain(card_no, stack_scratch, len);
    char *heap_scratch = rb_malloc(need);
    if (!heap_scratch) return -1;
    id = card_no_resolve_chain(card_no, heap_scratch, len);
    rb_free(heap_scratch);
    return id;
}

void rb_effect_data_free(RbEffectData *d) {
    if (!d) return;
    switch (d->kind) {
        case RB_EFFECT_DATA_HEART_OVERRIDE:
            rb_free(d->value.heart_override.color);
            break;
        case RB_EFFECT_DATA_SINGLE_CARD:
            rb_free(d->value.single_card.color);
            break;
        case RB_EFFECT_DATA_MULTI_CARD:
            for (size_t i = 0; i < d->value.multi_card.count; ++i)
                rb_free(d->value.multi_card.items[i].color);
            rb_free(d->value.multi_card.items);
            break;
        default:
            break;
    }
    memset(d, 0, sizeof(*d));
    d->kind = RB_EFFECT_DATA_MULTI_CARD;
}

int rb_effect_data_clone(const RbEffectData *source, RbEffectData *out) {
    RbEffectData copy;
    if (!source || !out) return -1;
    if (source == out) return 0;
    copy = *source;
    switch (source->kind) {
        case RB_EFFECT_DATA_HEART_OVERRIDE:
            if (!source->value.heart_override.color) return -1;
            copy.value.heart_override.color = rb_strdup2(source->value.heart_override.color);
            if (!copy.value.heart_override.color) return -1;
            break;
        case RB_EFFECT_DATA_SINGLE_CARD:
            if (source->value.single_card.color) {
                copy.value.single_card.color = rb_strdup2(source->value.single_card.color);
                if (!copy.value.single_card.color) return -1;
            }
            break;
        case RB_EFFECT_DATA_MULTI_CARD: {
            size_t n = source->value.multi_card.count;
            copy.value.multi_card.items = NULL;
            copy.value.multi_card.count = 0;
            if (!n) break;
            if (!source->value.multi_card.items || n > SIZE_MAX / sizeof(RbCardEffectItem)) return -1;
            copy.value.multi_card.items = rb_malloc(n * sizeof(RbCardEffectItem));
            if (!copy.value.multi_card.items) return -1;
            for (size_t i = 0; i < n; ++i) {
                const RbCardEffectItem *item = &source->value.multi_card.items[i];
                RbCardEffectItem *dest = &copy.value.multi_card.items[i];
                *dest = *item;
                dest->color = item->color ? rb_strdup2(item->color) : NULL;
                if (item->color && !dest->color) {
                    rb_effect_data_free(&copy);
                    return -1;
                }
                ++copy.value.multi_card.count;
            }
            break;
        }
        case RB_EFFECT_DATA_ALL_CARDS:
        case RB_EFFECT_DATA_SET_BLADE_COUNT:
        case RB_EFFECT_DATA_SURPLUS_HEART:
        case RB_EFFECT_DATA_GAIN_ABILITY:
            break;
        default:
            return -1;
    }
    *out = copy;
    return 0;
}

const char *rb_turn_phase_display(RbTurnPhase phase) {
    switch (phase) {
        case RB_TURNP_NORMAL_FIRST: return "FirstAttackerNormal";
        case RB_TURNP_NORMAL_SECOND: return "SecondAttackerNormal";
        case RB_TURNP_LIVE: return "Live";
        default: return "";
    }
}

/* ── RB_PHASE_DONE: A C-ONLY RbPhase VARIANT. DO NOT "FIX" IT AWAY ────────
 * Rust's `Phase` has 13 variants and NO "done": game-over is not a phase
 * there, it is `GameResult != Ongoing` (types.rs:137-151 vs the GameResult enum
 * beside it). The C host loop needs a terminal phase to `break` out of on, so
 * rabuka.h:782 appends RB_PHASE_DONE as a 14th variant, AFTER the 13 Rust ones
 * so nothing renumbers.
 *
 * That makes this the exact shape that produced the two historical drifts the
 * tables in this file exist to catch:
 *   1. RB_PHASE_DONE HAS NO RUST DISPLAY ARM, so "Done" / "終了" below are
 *      C-only strings. They are the load-bearing part of the variant, not
 *      decoration — do not treat them as drift and do not delete them with it.
 *   2. It is also why `rb_phase_display` answers for 14 ordinals while
 *      describe.rs/enums.rs enumerate 13. A future auditor counting rows and
 *      expecting 13 will "find" a 14th and remove the wrong thing.
 * So: the two arms below are deliberate, `rb_phase_is_live_phase` correctly
 * returns false for it (game-over is not inside the live window), and it must
 * survive as long as host code compares against it. */

const char *rb_phase_display(RbPhase phase) {
    switch (phase) {
        case RB_PHASE_RPS: return "RPS";
        case RB_PHASE_OPENING: return "Choose 1st";
        case RB_PHASE_MULLIGAN_FIRST: return "Mulligan (1st)";
        case RB_PHASE_MULLIGAN_SECOND: return "Mulligan (2nd)";
        case RB_PHASE_ACTIVE: return "Active";
        case RB_PHASE_ENERGY: return "Energy";
        case RB_PHASE_DRAW: return "Draw";
        case RB_PHASE_MAIN: return "Main";
        case RB_PHASE_LIVE_SET: return "LiveCardSet (1st)";
        case RB_PHASE_LIVE_SET_SECOND: return "LiveCardSet (2nd)";
        case RB_PHASE_PERFORMANCE: return "Perform (1st)";
        case RB_PHASE_PERFORMANCE_SECOND: return "Perform (2nd)";
        case RB_PHASE_VICTORY: return "Live Result";
        case RB_PHASE_DONE: return "Done";   /* C-only; see the note above */
        default: return "";
    }
}

int rb_effect_data_card_id(const RbEffectData *d, int16_t *out_card_id) {
    if (!d || !out_card_id) return 0;
    switch (d->kind) {
        case RB_EFFECT_DATA_HEART_OVERRIDE:
            *out_card_id = d->value.heart_override.card_id;
            return 1;
        case RB_EFFECT_DATA_SINGLE_CARD:
            *out_card_id = d->value.single_card.card_id;
            return 1;
        case RB_EFFECT_DATA_SET_BLADE_COUNT:
            *out_card_id = d->value.set_blade_count.card_id;
            return 1;
        case RB_EFFECT_DATA_GAIN_ABILITY:
            *out_card_id = d->value.gain_ability.card_id;
            return 1;
        default:
            return 0;
    }
}

int rb_effect_data_items(const RbEffectData *d, RbCardEffectItemRef **out_items, size_t *out_n) {
    const RbCardEffectItem *items;
    size_t n;
    RbCardEffectItemRef *refs;
    if (!out_items || !out_n) return -1;
    *out_items = NULL;
    *out_n = 0;
    if (!d) return -1;
    switch (d->kind) {
        case RB_EFFECT_DATA_SINGLE_CARD:
            items = &d->value.single_card;
            n = 1;
            break;
        case RB_EFFECT_DATA_MULTI_CARD:
            items = d->value.multi_card.items;
            n = d->value.multi_card.count;
            break;
        default:
            return 0;
    }
    if (!n) return 0;
    if (!items || n > SIZE_MAX / sizeof(*refs)) return -1;
    refs = rb_malloc(n * sizeof(*refs));
    if (!refs) return -1;
    for (size_t i = 0; i < n; ++i) {
        refs[i].card_id = items[i].card_id;
        refs[i].amount = items[i].amount;
        refs[i].color = items[i].color;
    }
    *out_items = refs;
    *out_n = n;
    return 0;
}

int rb_effect_data_is_p1(const RbEffectData *d, bool *out_is_p1) {
    if (!d || !out_is_p1 || d->kind != RB_EFFECT_DATA_SURPLUS_HEART) return 0;
    *out_is_p1 = d->value.surplus_heart.is_p1;
    return 1;
}

int rb_effect_data_old_value(const RbEffectData *d, uint8_t *out_old_value) {
    if (!d || !out_old_value || d->kind != RB_EFFECT_DATA_SURPLUS_HEART) return 0;
    *out_old_value = d->value.surplus_heart.old_value;
    return 1;
}

int rb_effect_data_count(const RbEffectData *d, uint8_t *out_count) {
    if (!d || !out_count || d->kind != RB_EFFECT_DATA_HEART_OVERRIDE) return 0;
    *out_count = d->value.heart_override.count;
    return 1;
}

int rb_effect_data_color(const RbEffectData *d, const char **out_color) {
    if (!d || !out_color) return 0;
    switch (d->kind) {
        case RB_EFFECT_DATA_HEART_OVERRIDE:
            *out_color = d->value.heart_override.color;
            return *out_color != NULL;
        case RB_EFFECT_DATA_SINGLE_CARD:
            *out_color = d->value.single_card.color;
            return *out_color != NULL;
        default:
            return 0;
    }
}

const char *rb_phase_label_jp(int phase) {
    switch (phase) {
        case RB_PHASE_RPS: return "ジャンケン";
        case RB_PHASE_OPENING: return "先攻選択";
        case RB_PHASE_MULLIGAN_FIRST: return "マリガン（先攻）";
        case RB_PHASE_MULLIGAN_SECOND: return "マリガン（後攻）";
        case RB_PHASE_ACTIVE: return "アクティブ";
        case RB_PHASE_ENERGY: return "エネルギー";
        case RB_PHASE_DRAW: return "ドロー";
        case RB_PHASE_MAIN: return "メイン";
        case RB_PHASE_LIVE_SET: return "ライブセット（先攻）";
        case RB_PHASE_LIVE_SET_SECOND: return "ライブセット（後攻）";
        case RB_PHASE_PERFORMANCE: return "パフォーマンス（先攻）";
        case RB_PHASE_PERFORMANCE_SECOND: return "パフォーマンス（後攻）";
        case RB_PHASE_VICTORY: return "ライブ勝敗判定";
        case RB_PHASE_DONE: return "終了";   /* C-only; see the note above */
        default: return "不明";
    }
}

const char *rb_effect_type_as_str(RbEffectType t) {
    switch (t) {
        case RB_EFFECT_HEART_BONUS: return "heart_bonus";
        case RB_EFFECT_BLADE_BONUS: return "blade_bonus";
        case RB_EFFECT_SCORE_BONUS: return "score_bonus";
        case RB_EFFECT_SCORE_SET: return "score_set";
        case RB_EFFECT_TRANSFORM: return "transform";
        case RB_EFFECT_NEED_HEART_MOD: return "need_heart_mod";
        case RB_EFFECT_HEART_OVERRIDE: return "heart_override";
        case RB_EFFECT_COST_BONUS: return "cost_bonus";
        case RB_EFFECT_COST_SET: return "cost_set";
        case RB_EFFECT_BLADE_SET: return "blade_set";
        case RB_EFFECT_BLADE_TYPE_SET: return "blade_type_set";
        default: return "";
    }
}

static const struct {
    RbAbilityZone ability;
    RbZoneId core;
} rb_ability_zone_pairs[] = {
    {RB_ABILITY_ZONE_STAGE, RB_ZONEID_STAGE},
    {RB_ABILITY_ZONE_HAND, RB_ZONEID_HAND},
    {RB_ABILITY_ZONE_DECK, RB_ZONEID_DECK},
    {RB_ABILITY_ZONE_DISCARD, RB_ZONEID_DISCARD},
    {RB_ABILITY_ZONE_ENERGY, RB_ZONEID_ENERGY},
    {RB_ABILITY_ZONE_LIVE_CARD_ZONE, RB_ZONEID_LIVE_CARD_ZONE},
    {RB_ABILITY_ZONE_SUCCESS_LIVE_ZONE, RB_ZONEID_SUCCESS_LIVE_ZONE},
    {RB_ABILITY_ZONE_REVEALED_CARDS, RB_ZONEID_REVEALED_CARDS},
    {RB_ABILITY_ZONE_DECK_TOP, RB_ZONEID_DECK_TOP},
    {RB_ABILITY_ZONE_DECK_BOTTOM, RB_ZONEID_DECK_BOTTOM},
    {RB_ABILITY_ZONE_DECK_TOP_OR_BOTTOM, RB_ZONEID_DECK_TOP_OR_BOTTOM},
    {RB_ABILITY_ZONE_FRONT, RB_ZONEID_FRONT},
    {RB_ABILITY_ZONE_LIVE_TOTAL, RB_ZONEID_LIVE_TOTAL},
    {RB_ABILITY_ZONE_THOSE_CARDS, RB_ZONEID_THOSE_CARDS},
    {RB_ABILITY_ZONE_PRECEDING_MOVED, RB_ZONEID_PRECEDING_MOVED},
    {RB_ABILITY_ZONE_RECENTLY_MOVED, RB_ZONEID_RECENTLY_MOVED},
    {RB_ABILITY_ZONE_LOOKED_AT_REMAINING, RB_ZONEID_LOOKED_AT_REMAINING},
    {RB_ABILITY_ZONE_EMPTY_AREA, RB_ZONEID_EMPTY_AREA},
    {RB_ABILITY_ZONE_SAME_AREA, RB_ZONEID_SAME_AREA},
    {RB_ABILITY_ZONE_UNDER_MEMBER, RB_ZONEID_UNDER_MEMBER},
    {RB_ABILITY_ZONE_LOOKED_AT, RB_ZONEID_LOOKED_AT}
};

RbZoneId rb_zone_id_from_ability_zone(RbAbilityZone ability_zone) {
    for (size_t i = 0; i < sizeof(rb_ability_zone_pairs) / sizeof(rb_ability_zone_pairs[0]); ++i) {
        if (rb_ability_zone_pairs[i].ability == ability_zone) return rb_ability_zone_pairs[i].core;
    }
    return RB_ZONEID_UNKNOWN;
}

int rb_zone_id_to_ability_zone(RbZoneId z, RbAbilityZone *out_ability_zone) {
    if (!out_ability_zone) return -1;
    switch (z) {
        case RB_ZONEID_WAITROOM: z = RB_ZONEID_DISCARD; break;
        case RB_ZONEID_ENERGY_ZONE: z = RB_ZONEID_ENERGY; break;
        case RB_ZONEID_SUCCESS_ZONE: z = RB_ZONEID_SUCCESS_LIVE_ZONE; break;
        default: break;
    }
    for (size_t i = 0; i < sizeof(rb_ability_zone_pairs) / sizeof(rb_ability_zone_pairs[0]); ++i) {
        if (rb_ability_zone_pairs[i].core == z) {
            *out_ability_zone = rb_ability_zone_pairs[i].ability;
            return 0;
        }
    }
    return -1;
}

static const struct {
    RbZoneId id;
    const char *name;
} rb_zone_names[] = {
    {RB_ZONEID_STAGE, "stage"},
    {RB_ZONEID_HAND, "hand"},
    {RB_ZONEID_DECK, "deck"},
    {RB_ZONEID_DECK_TOP, "deck_top"},
    {RB_ZONEID_DECK_BOTTOM, "deck_bottom"},
    {RB_ZONEID_DECK_TOP_OR_BOTTOM, "deck_top_or_bottom"},
    {RB_ZONEID_DISCARD, "discard"},
    {RB_ZONEID_WAITROOM, "waitroom"},
    {RB_ZONEID_ENERGY, "energy"},
    {RB_ZONEID_ENERGY_ZONE, "energy_zone"},
    {RB_ZONEID_ENERGY_DECK, "energy_deck"},
    {RB_ZONEID_SUCCESS_ZONE, "success_zone"},
    {RB_ZONEID_LIVE_CARD_ZONE, "live_card_zone"},
    {RB_ZONEID_SUCCESS_LIVE_ZONE, "success_live_zone"},
    {RB_ZONEID_EMPTY_AREA, "empty_area"},
    {RB_ZONEID_SAME_AREA, "same_area"},
    {RB_ZONEID_UNDER_MEMBER, "under_member"},
    {RB_ZONEID_LOOKED_AT, "looked_at"},
    {RB_ZONEID_LOOKED_AT_REMAINING, "looked_at_remaining"},
    {RB_ZONEID_REVEALED_CARDS, "revealed_cards"},
    {RB_ZONEID_SELECTED_CARDS, "selected_cards"},
    {RB_ZONEID_THOSE_CARDS, "those_cards"},
    {RB_ZONEID_PRECEDING_MOVED, "preceding_moved"},
    {RB_ZONEID_RECENTLY_MOVED, "recently_moved"},
    {RB_ZONEID_FRONT, "front"},
    {RB_ZONEID_LIVE_TOTAL, "live_total"},
    {RB_ZONEID_RESOLUTION, "resolution"},
    {RB_ZONEID_EXCLUSION_ZONE, "exclusion_zone"},
    {RB_ZONEID_UNKNOWN, "unknown"}
};

/* ── ZoneId::from_str ALIAS table (types.rs:755-786) ─────────────────────────
   Every wire string Rust's `ZoneId::from_str` accepts that is NOT itself the
   canonical `as_str()` of the variant it decodes to.

   These MUST be consulted before rb_zone_names, because `energy_zone` is
   simultaneously
     * the canonical as_str() of ZoneId::EnergyZone, and
     * an alias arm of ZoneId::Energy ("energy" | "energy_zone"),
   and Rust's match resolves the alias: `"energy" | "energy_zone" => ZoneId::Energy`
   (types.rs:765). A single table walked in declaration order would hand
   `energy_zone` to whichever row came first; keeping the five alias rows in
   their own table makes the precedence a property of the data instead of a
   property of the order two `if`s happen to be written in.

   The five rows below are the complete alias set — one per `"a" | "b"` arm in
   Rust's from_str:
     "stage" | "ステージ"              -> Stage
     "energy" | "energy_zone"           -> Energy
     "success_live_zone" | "success_live_card_zone" -> SuccessLiveZone
     "under_member" | "under"           -> UnderMember
     "resolution" | "resolution_zone"   -> Resolution
   A sixth alias arm would be a silent mis-decode here exactly as it would be
   in Rust: the string would fall through to the `_ => Unknown` arm. */
static const struct {
    const char *alias;
    RbZoneId    id;
} rb_zone_id_aliases[] = {
    { "ステージ",                RB_ZONEID_STAGE },
    { "energy_zone",             RB_ZONEID_ENERGY },
    { "success_live_card_zone",  RB_ZONEID_SUCCESS_LIVE_ZONE },
    { "under",                   RB_ZONEID_UNDER_MEMBER },
    { "resolution_zone",         RB_ZONEID_RESOLUTION }
};
#define N_RB_ZONE_ID_ALIASES (sizeof(rb_zone_id_aliases) / sizeof(rb_zone_id_aliases[0]))

RbZoneId rb_zone_id_from_str(const char *s) {
    if (!s) return RB_ZONEID_UNKNOWN;
    for (size_t i = 0; i < N_RB_ZONE_ID_ALIASES; ++i) {
        if (!strcmp(s, rb_zone_id_aliases[i].alias)) return rb_zone_id_aliases[i].id;
    }
    for (size_t i = 0; i < sizeof(rb_zone_names) / sizeof(rb_zone_names[0]); ++i) {
        if (!strcmp(s, rb_zone_names[i].name)) return rb_zone_names[i].id;
    }
    /* Rust's `_ => ZoneId::Unknown` arm (types.rs:784). Total by design: this
       is a decode table with a deliberate default, not a fallible parser. */
    return RB_ZONEID_UNKNOWN;
}

const char *rb_zone_id_as_str(RbZoneId z) {
    for (size_t i = 0; i < sizeof(rb_zone_names) / sizeof(rb_zone_names[0]); ++i) {
        if (rb_zone_names[i].id == z) return rb_zone_names[i].name;
    }
    return "unknown";
}

int rb_zone_equivalent(RbZoneId a, RbZoneId b) {
    return a == b ||
        (a == RB_ZONEID_DISCARD && b == RB_ZONEID_WAITROOM) ||
        (a == RB_ZONEID_WAITROOM && b == RB_ZONEID_DISCARD) ||
        (a == RB_ZONEID_ENERGY && b == RB_ZONEID_ENERGY_ZONE) ||
        (a == RB_ZONEID_ENERGY_ZONE && b == RB_ZONEID_ENERGY);
}

int rb_zone_matches_source(RbZoneId zone, const char *source) {
    RbZoneId requested = rb_zone_id_from_str(source);
    switch (requested) {
        case RB_ZONEID_DECK:
            return zone == RB_ZONEID_DECK || zone == RB_ZONEID_DECK_TOP ||
                   zone == RB_ZONEID_DECK_BOTTOM;
        case RB_ZONEID_DECK_TOP:
            return zone == RB_ZONEID_DECK_TOP;
        case RB_ZONEID_DECK_BOTTOM:
            return zone == RB_ZONEID_DECK_BOTTOM;
        case RB_ZONEID_DISCARD:
        case RB_ZONEID_WAITROOM:
            return zone == RB_ZONEID_DISCARD || zone == RB_ZONEID_WAITROOM;
        default:
            return zone == requested;
    }
}

/* ZoneId::matches_dest (types.rs:899-901) -- the DESTINATION twin of
   rb_zone_matches_source, and deliberately NOT the same rule.

   The asymmetry is the whole point of the pair being two functions:
   matches_source widens the generic `deck` to its subzones (a card milled off
   the top of the deck is still "placed from the deck"), but matches_dest must
   NOT, because a condition asking to place a card INTO the deck must not fire
   for a move into one specific subzone. So this is a plain
   `equivalent(from_str(dest))` with no per-zone widening at all.

   Returns 0 (false) for a NULL destination, matching Rust's
   `ZoneId::from_str(dest)` on a `&str` — a NULL cannot name a zone, so it
   cannot be equivalent to one. */
int rb_zone_matches_dest(RbZoneId zone, const char *dest) {
    if (!dest) return 0;
    return rb_zone_equivalent(zone, rb_zone_id_from_str(dest));
}

/* Phase::is_live_phase (types.rs:187-196) -- the single definition of the
   "live window": both live-card-set phases, both performances, and the victory
   determination that closes them.

   Rust keeps this as ONE `matches!` precisely because the negation was
   hand-written out in full at ability/condition/state.rs (`before_live`, as
   five separate `!matches!` clauses) and drifted when a sixth live phase was
   added: the positive sites got updated and the silent negation kept the old
   answer. Use this for BOTH polarities; never re-list the variants. */
int rb_phase_is_live_phase(int phase) {
    switch (phase) {
        case RB_PHASE_LIVE_SET:
        case RB_PHASE_LIVE_SET_SECOND:
        case RB_PHASE_PERFORMANCE:
        case RB_PHASE_PERFORMANCE_SECOND:
        case RB_PHASE_VICTORY:
            return 1;
        default:
            return 0;
    }
}

int rb_effect_data_amount(const RbEffectData *d, int16_t *out_amount) {
    if (!d || !out_amount) return 0;
    switch (d->kind) {
        case RB_EFFECT_DATA_SINGLE_CARD:
            *out_amount = d->value.single_card.amount;
            return 1;
        case RB_EFFECT_DATA_ALL_CARDS:
            *out_amount = d->value.all_cards.amount;
            return 1;
        default:
            return 0;
    }
}

/* ══════════════════════════════════════════════════════════════════════════
   engine/src/core/constants.rs — the numeric helpers that did NOT survive the
   port as inline header macros.

   rb_saturate_u8 / rb_saturate_i16 already exist as `static inline` in
   rabuka.h; everything below was simply absent, so every call site spelled the
   arithmetic out longhand and the two failure modes the Rust doc comments call
   out by name were live: a truncating cast that wraps past a bound, and an
   out-of-range score that becomes an arbitrary number and then WINS a
   comparison it should have lost.
   ══════════════════════════════════════════════════════════════════════════ */

/* score_to_i32 (constants.rs:46-51) — narrow a score-space f64 to i32 by
   TRUNCATION, saturating out-of-range and non-finite inputs.

   The NaN guard comes first and is not redundant with the clamp: every
   comparison against NaN is false, so an unclamped NaN would slip past a
   range check and then land on 0 on one path and an arbitrary value on
   another. Rust's `as i32` is saturating; a C cast is undefined, so the
   clamp has to happen in f64 space before the narrowing. */
int rb_score_to_i32(double v) {
    if (v != v) return 0;                       /* NaN */
    if (v >= 2147483647.0) return 2147483647;
    if (v <= -2147483648.0) return -2147483647 - 1;
    return (int)v;                              /* truncates toward zero */
}

/* score_to_i32_rounded (constants.rs:61-75) — same contract, round-to-nearest
   first, and DISTINCT from rb_score_to_i32 on purpose.

   The crate has two different narrowing idioms, `(x * f).round() as i32` and a
   bare `x as i32`, and they disagree on half-values. Pick the one that
   matches the call site; this is the `round()` one. Rust spells it
   round-half-away-from-zero (`f64::round`), which is sign-symmetric add-0.5
   followed by the truncating cast — so -0.5 goes to -1, not to 0. */
int rb_score_to_i32_rounded(double v) {
    if (v != v) return 0;                       /* NaN */
    double half_away = (v >= 0.0) ? (v + 0.5) : (v - 0.5);
    if (half_away >= 2147483647.0) return 2147483647;
    if (half_away <= -2147483648.0) return -2147483647 - 1;
    return (int)half_away;
}

/* effective_stat (constants.rs:87-89) — a PRINTED u8 stat plus its modifier,
   saturating to u8. The `printed + modifier` chain that condition evaluation
   and cost math all spell out. */
uint8_t rb_effective_stat(uint8_t printed, int16_t modifier) {
    return rb_saturate_u8((int)printed + (int)modifier);
}

/* overridden_stat (constants.rs:102-104) — the SIBLING of rb_effective_stat for
   the other case: a figure that already has an absolute set-override standing
   in for its printed value.

   Kept separate rather than folded in because the two inputs are not the same
   kind of number. A printed stat is a u8 — it came off the card. A set
   override is a modifier i16: 「ブレードの値が5になる」 stores whatever the
   ability said, and it stacks additively on top like any other modifier. */
uint8_t rb_overridden_stat(int16_t set, int16_t modifier) {
    return rb_saturate_u8((int)set + (int)modifier);
}

/* floored_cost (constants.rs:116-118) — effective playable cost with the
   floor-at-1 of the baton-touch price formula: `(base + modifier).max(1)`.

   The canonical home for a formula that core/player.rs (the payment itself),
   game_setup.rs (the estimates) and turn/phases.rs (the double-baton path)
   previously each spelled three different ways, with `as u8` wrapping in one
   and `try_from(..).unwrap_or(0)` in another. Saturates rather than wraps. */
uint8_t rb_floored_cost(uint8_t base, int16_t modifier) {
    uint8_t v = rb_saturate_u8((int)base + (int)modifier);
    return v < 1 ? 1 : v;
}

/* count_u32 (constants.rs:347-349) — widen a count to u32, saturating at the
   top. The old `as u32` on a huge count produced a value that then collided
   with nothing and silently failed to match its own entry. */
uint32_t rb_count_u32(size_t v) {
    return (v > (size_t)UINT32_MAX) ? UINT32_MAX : (uint32_t)v;
}

/* id_u32 (constants.rs:357-359) — widen an i16 identifier to u32 for hash
   keys, FLOORING at zero.

   Card and ability indices are non-negative by construction; the old `as u32`
   on a negative value produced a huge key instead, which then matched nothing. */
uint32_t rb_id_u32(int16_t v) {
    return (v < 0) ? 0u : (uint32_t)v;
}

/* f32_count (constants.rs:501-512) — saturating f64 -> f32.

   A plain `(float)` cast turns an out-of-range density into an infinity, which
   then propagates through the feature vector as a NaN rather than as a
   saturated 1.0. */
float rb_f32_count(double v) {
    float narrowed = (float)v;
    if (narrowed == narrowed && narrowed <= 3.40282346638528859812e+38f &&
        narrowed >= -3.40282346638528859812e+38f) {
        return narrowed;
    }
    if (v != v) return 0.0f;                    /* NaN */
    return (v > 0.0) ? 3.40282346638528859812e+38f
                     : -3.40282346638528859812e+38f;
}


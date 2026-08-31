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

    load_bytecode();
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
}

/* expose internal card data accessors for cards.c / vm.c */
const unsigned char *rb_card_record(uint32_t i) {
    if (i >= g_num_cards) return NULL;
    return g_card_data + g_card_off[i];
}
uint16_t rb_card_stridx(uint32_t i) {
    if (i >= g_num_cards) return 0;
    return le16(rb_card_record(i) + 2); /* name_idx at offset 2 */
}
uint16_t rb_card_ability_idx(uint32_t i) {
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

int rb_find_card_by_no(const char *card_no) {
    if (!card_no || !g_cards_blob) return -1;
    for (uint32_t i = 0; i < g_num_cards; i++) {
        const unsigned char *rec = rb_card_record(i);
        if (!rec) continue;
        uint16_t no_idx = le16(rec + 0); /* card_no_idx at offset 0 per cards.c */
        const char *no = rb_card_string(no_idx);
        if (no && strcmp(no, card_no) == 0) return (int)i;
    }
    return -1;
}

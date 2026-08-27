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
extern const uint16_t RBKA_OFFSET_DELTAS[];
extern const uint32_t RBKA_STRINGS_OFFSETS[];

/* ── globals ── */
static unsigned char *g_cards_blob = NULL;
static long          g_cards_len = 0;
static uint32_t      g_num_cards = 0;
static char        **g_card_strings = NULL;   /* null-terminated copies */
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

/* ── load cards.bin ── */
static int load_cards(const char *dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/cards.bin", dir);
    g_cards_blob = read_file(path, &g_cards_len);
    if (!g_cards_blob) return -1;
    fprintf(stderr, "[dbg] cards.bin loaded len=%ld\n", g_cards_len);
    if (g_cards_len < 12 || memcmp(g_cards_blob, "CARD", 4) != 0) return -2;

    /* header: "<4sHI" => magic(4) + num_cards(u16) + strtab_len(u32) */
    if (g_cards_len < 10 || memcmp(g_cards_blob, "CARD", 4) != 0) return -2;
    g_num_cards = le16(g_cards_blob + 4);
    uint32_t strtab_len = le32(g_cards_blob + 6);
    const unsigned char *strtab = g_cards_blob + 10;
    const unsigned char *p = strtab;

    /* string table: u16 len + bytes per entry */
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

    /* length table: one u8 per card, then concatenated card records */
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
    fprintf(stderr, "[dbg] load_cards ok num_cards=%u strtab=%u\n", g_num_cards, strtab_len);
    return 0;
}

/* ── load abilities_strings.bin + build null-terminated copies ── */
static int load_strings(const char *dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/abilities_strings.bin", dir);
    g_abstr_blob = read_file(path, &g_abstr_len);
    if (!g_abstr_blob) return -1;

    uint32_t n = RBKA_NUM_STRING_OFFSETS ? (RBKA_NUM_STRING_OFFSETS - 1) : 0;
    g_strings = malloc((n ? n : 1) * sizeof(char *));
    for (uint32_t i = 0; i < n; i++) {
        uint32_t a = RBKA_STRINGS_OFFSETS[i];
        uint32_t b = RBKA_STRINGS_OFFSETS[i + 1];
        uint32_t len = b - a;
        char *s = malloc(len + 1);
        memcpy(s, g_abstr_blob + a, len); s[len] = 0;
        g_strings[i] = s;
    }
    return 0;
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
    if (g_card_strings && idx < (uint32_t)-1) return g_card_strings[idx];
    return "";
}

/* ability slice access for vm.c */
const unsigned char *rb_bc_slice(uint32_t idx, uint32_t *out_len) {
    if (idx >= RBKA_NUM_ABILITIES) return NULL;
    uint32_t start = 0;
    for (uint32_t i = 0; i < idx; i++) start += RBKA_OFFSET_DELTAS[i];
    uint32_t len = RBKA_OFFSET_DELTAS[idx];
    *out_len = len;
    if (start + len > g_bc_len) return NULL;
    return g_bc + start;
}

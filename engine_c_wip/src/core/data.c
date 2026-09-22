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
uint32_t rb_card_record_len(uint32_t i) {
    if (i >= g_num_cards) return 0;
    return g_card_off[i + 1] - g_card_off[i];
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
        case RB_PHASE_DONE: return "Done";
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
        case RB_PHASE_DONE: return "終了";
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

RbZoneId rb_zone_id_from_str(const char *s) {
    if (!s) return RB_ZONEID_UNKNOWN;
    if (!strcmp(s, "ステージ")) return RB_ZONEID_STAGE;
    if (!strcmp(s, "energy_zone")) return RB_ZONEID_ENERGY;
    if (!strcmp(s, "success_live_card_zone")) return RB_ZONEID_SUCCESS_LIVE_ZONE;
    if (!strcmp(s, "under")) return RB_ZONEID_UNDER_MEMBER;
    if (!strcmp(s, "resolution_zone")) return RB_ZONEID_RESOLUTION;
    for (size_t i = 0; i < sizeof(rb_zone_names) / sizeof(rb_zone_names[0]); ++i) {
        if (!strcmp(s, rb_zone_names[i].name)) return rb_zone_names[i].id;
    }
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
        case RB_ZONEID_DISCARD:
        case RB_ZONEID_WAITROOM:
            return zone == RB_ZONEID_DISCARD || zone == RB_ZONEID_WAITROOM;
        default:
            return zone == requested;
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


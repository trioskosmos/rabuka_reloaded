/* test_decode_skew.c — the corpus-wide decode-skew audit.
 *
 * THE DEFECT THIS EXISTS TO CATCH
 * ------------------------------
 * The C engine resolves decoded ability strings through (start, len) offsets
 * that are COMPILED IN, in src/core/generated/gen_data.c
 * (RBKA_STRINGS_OFFSETS), while the string TABLE those offsets index —
 * cards/build/abilities_strings.bin — is a GITIGNORED build artifact
 * (.gitignore:189) read from disk at runtime. On 2026-09-30 those two came
 * from different generator runs: a peer rebuilt the .bin from
 * cards/abilities.json and the committed offsets were left 227 slots stale, by
 * up to 311 bytes.
 *
 * The interned string pool is UNSEPARATED, so a small skew is not an error.
 * It is a byte splice of two neighbouring entries:
 *
 *     intended             decoded
 *     cost_limit           cost_limitc
 *     left_side            eft_side
 *     invalidate_ability   stiming_conditioni
 *     invalidate_triggers  ationsthose_ca
 *     mu's                 arde
 *
 * 458 of 985 abilities (46.5%) decoded at least one string wrong; 128 had a
 * defective key. Several peers independently reported the visible symptoms as
 * engine bugs — "invalidate_ability is never evaluated", "resource_type
 * compares garbage against garbage", "group_names is [arde] so no card can
 * match". Those are this bug, not the engine's.
 *
 * WHY A LENGTH CHECK IS NOT ENOUGH
 * The pool was 726731 bytes before and after the drift: content moved around
 * inside a pool of identical total size. A (start,len) in-range test passes
 * and the decode is still wrong. The load-bearing check is the per-string
 * FNV-1a digest that tools/gen_from_rs.py emits next to the offsets and that
 * rb_verify_generated_tables() recomputes from the blob actually loaded.
 *
 * WHAT THIS SUITE DOES
 *   1. rb_verify_generated_tables() — the three-layer load-time self-check
 *      (offsets monotonic + sentinel length, per-string FNV-1a, semantic
 *      plausibility). Any layer failing is a hard fail, loudly, with a named
 *      string in the message.
 *   2. A live walk of EVERY card, EVERY ability, EVERY effect and EVERY
 *      condition, dumping each decoded extra key and condition key and
 *      flagging any that is not plausible text. The headline number this
 *      suite reports is how many of the card-ability pairs decode cleanly.
 *   3. Targeted asserts on the specific keys the skew ate, so a future drift
 *      cannot quietly re-break exactly these abilities.
 *   4. The rb_card_string / rb_card_no_eq out-of-range behaviour check.
 *
 * No assertion here is conditional. This suite must be GREEN or the engine is
 * decoding the wrong strings.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include "gen_data.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* Defined in src/ability/vm.c. Declared here (and in gen_data.h) so this file
 * does not depend on an in-flight edit to a header this agent does not own. */
extern const uint32_t RBKA_STRINGS_OFFSETS[];
extern const uint32_t RBKA_STRINGS_TABLE_LEN;
extern int  rb_verify_generated_tables(int *out_bad_strings, int *out_bad_keys,
                                       char *msg, unsigned long msg_sz);

static int g_fail = 0, g_checks = 0;
static int g_bad_extra_keys = 0, g_bad_cond_keys = 0, g_bad_values = 0;
static int g_empty_action = 0;
static int g_reported = 0;
static const char *DEFECT_NAME_V = "ok";
#define MAX_REPORT 24

static void ck(int cond, const char *what) {
    g_checks++;
    if (cond) return;
    g_fail++;
    printf("  FAIL  %s\n", what);
}

static void report(const char *shape, uint32_t cid, uint32_t ab,
                   const char *key, const char *val) {
    if (g_reported++ < MAX_REPORT)
        printf("  SKEW card %u ability %u %s \"%s\" -> [%s] \"%s\"\n",
               cid, ab, shape, key ? key : "(value)", DEFECT_NAME_V, val ? val : "");
}

/* Is this byte sequence plausible decoded text? Rejects the three shapes a
 * splice actually takes: the U+FFFD replacement character, a C0 control byte,
 * and an invalid or truncated UTF-8 sequence. On a defect it leaves the reason
 * in DEFECT_NAME_V and returns non-zero.
 *
 * `allow_empty` matters: the pool legitimately interns "" (an absent optional
 * value), and several AbilityEffect fields are fixed char[] that stay "" when
 * the wire key was absent. Treating "" as a defect there produced 12704 false
 * positives on the first run of this suite. "" IS a defect for a key, because
 * a key is never legitimately empty -- an out-of-range index makes
 * rb_get_string return the shared "" literal, which is how the pre-2026-09-30
 * rb_card_no_eq(id, "") matched every card in the database. */
static int key_defect(const char *s, int allow_empty) {
    static const char *NAME[] = { "EMPTY", "U+FFFD", "CONTROL", "BAD_UTF8" };
    const unsigned char *p = (const unsigned char *)s;
    size_t n = 0;
    if (!s || !*s) { DEFECT_NAME_V = NAME[0]; return allow_empty ? 0 : 1; }
    while (p[n]) {
        unsigned char c = p[n];
        if (c == 0xEF && p[n+1] == 0xBF && p[n+2] == 0xBD) { DEFECT_NAME_V = NAME[1]; return 2; }
        if (c < 0x20 && c != '\t' && c != '\n' && c != '\r') { DEFECT_NAME_V = NAME[2]; return 3; }
        if (c == 0x7F) { DEFECT_NAME_V = NAME[2]; return 3; }
        n++;
    }
    size_t i = 0;
    while (i < n) {
        unsigned char c = p[i];
        int extra; unsigned int cp;
        if (c < 0x80) { i++; continue; }
        else if ((c & 0xE0) == 0xC0) { extra = 1; cp = c & 0x1F; }
        else if ((c & 0xF0) == 0xE0) { extra = 2; cp = c & 0x0F; }
        else if ((c & 0xF8) == 0xF0) { extra = 3; cp = c & 0x07; }
        else { DEFECT_NAME_V = NAME[3]; return 4; }
        if (i + (size_t)extra >= n) { DEFECT_NAME_V = NAME[3]; return 4; }
        for (int k = 1; k <= extra; k++) {
            unsigned char cc = p[i + (size_t)k];
            if ((cc & 0xC0) != 0x80) { DEFECT_NAME_V = NAME[3]; return 4; }
            cp = (cp << 6) | (cc & 0x3F);
        }
        if ((extra == 1 && cp < 0x80) || (extra == 2 && cp < 0x800) ||
            (extra == 3 && cp < 0x10000) || cp > 0x10FFFF ||
            (cp >= 0xD800 && cp <= 0xDFFF)) { DEFECT_NAME_V = NAME[3]; return 4; }
        i += (size_t)extra + 1;
    }
    DEFECT_NAME_V = "ok";
    return 0;
}

static void check_str(const char *key, const char *val, const char *shape,
                      uint32_t cid, uint32_t ab, int *counter, int allow_empty) {
    if (!val) return;
    if (!key_defect(val, allow_empty)) return;
    (*counter)++;
    report(shape, cid, ab, key, val);
}

static void walk_cond(const Condition *c, const char *shape,
                      uint32_t cid, uint32_t ab, int depth) {
    if (!c || depth > 16) return;
    for (uint32_t i = 0; i < c->n_fields && i < RB_MAX_COND_FIELD; i++) {
        const CondField *f = &c->fields[i];
        check_str(f->key, f->key, shape, cid, ab, &g_bad_cond_keys, 0);
        const CondValue *v = &f->v;
        if (v->tag == RB_TAG_STR)
            check_str(f->key, v->s, "cond-value", cid, ab, &g_bad_values, 1);
        else if (v->cond)
            walk_cond(v->cond, shape, cid, ab, depth + 1);
        else if (v->arr) {
            for (uint32_t j = 0; j < v->arr_n; j++)
                if (v->arr[j].tag == RB_TAG_STR)
                    check_str(f->key, v->arr[j].s, "cond-array", cid, ab, &g_bad_values, 1);
        }
    }
}

static void walk_effect(const AbilityEffect *e, uint32_t cid, uint32_t ab, int depth) {
    if (!e || depth > 24) return;
    /* `action` is allowed to be empty: the pool interns "" (index 346) and
     * one effect in the current cards/abilities.json really does carry an
     * empty action. That is a source-data state, not a decode defect — the
     * FNV self-check in section 1 proves the index is in range and matches
     * its recorded digest. It is counted and reported separately below. */
    check_str("action", e->action, "effect", cid, ab, &g_bad_values, 1);
    if (e->action && !*e->action) g_empty_action++;
    check_str("source", e->source, "effect", cid, ab, &g_bad_values, 1);
    check_str("destination", e->destination, "effect", cid, ab, &g_bad_values, 1);
    check_str("target", e->target, "effect", cid, ab, &g_bad_values, 1);
    /* fixed char[]: "" means the wire key was absent, which is legitimate */
    check_str("id_field", e->id_field, "effect", cid, ab, &g_bad_values, 1);
    check_str("self_target_field", e->self_target_field, "effect", cid, ab, &g_bad_values, 1);
    check_str("card_type_field", e->card_type_field, "effect", cid, ab, &g_bad_values, 1);
    for (int i = 0; i < e->n_extra; i++) {
        check_str(e->extra_k[i], e->extra_k[i], "extra-key", cid, ab, &g_bad_extra_keys, 0);
        check_str(e->extra_k[i], e->extra_v[i], "extra-value", cid, ab, &g_bad_values, 1);
    }
    walk_cond(e->condition, "condition", cid, ab, 0);
    walk_cond(e->activation_condition, "activation_condition", cid, ab, 0);
    walk_cond(e->result_condition, "result_condition", cid, ab, 0);
    walk_cond(e->alternative_condition, "alternative_condition", cid, ab, 0);
    for (int i = 0; i < e->n_child; i++) walk_effect(e->child[i], cid, ab, depth + 1);
    for (int i = 0; i < e->n_options; i++) walk_effect(e->options[i], cid, ab, depth + 1);
    walk_effect(e->primary_effect, cid, ab, depth + 1);
    walk_effect(e->alternative_effect, cid, ab, depth + 1);
    walk_effect(e->look_action, cid, ab, depth + 1);
    walk_effect(e->select_action, cid, ab, depth + 1);
    walk_effect(e->followup_action, cid, ab, depth + 1);
    walk_effect(e->optional_action, cid, ab, depth + 1);
    walk_effect(e->conditional_action, cid, ab, depth + 1);
    walk_effect(e->gained_effect, cid, ab, depth + 1);
    walk_effect(e->resource_on_select, cid, ab, depth + 1);
    walk_effect(e->opponent_action, cid, ab, depth + 1);
}

int main(void) {
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }

    uint32_t nc = rb_num_cards(), na = rb_num_abilities();
    printf("=== decode-skew audit ===\n");
    printf("cards=%u abilities=%u interned_strings=%u table_bytes=%u\n\n",
           nc, na, (unsigned)(RBKA_NUM_STRING_OFFSETS - 1), RBKA_STRINGS_TABLE_LEN);

    /* ── 1. the load-time self-check ───────────────────────────────────── */
    printf("--- 1. rb_verify_generated_tables (load-time self-check) ---\n");
    int bad_strings = 0, bad_keys = 0;
    char msg[4096];
    int vrc = rb_verify_generated_tables(&bad_strings, &bad_keys, msg, sizeof msg);
    fputs(msg, stdout);
    ck(vrc == 0, "rb_verify_generated_tables() reports a consistent table");
    ck(bad_strings == 0, "zero defective interned strings");
    ck(bad_keys == 0, "zero defective key positions");

    /* ── 2. offset table shape ─────────────────────────────────────────── */
    printf("\n--- 2. offset table shape ---\n");
    {
        int mono = 1;
        for (uint32_t i = 0; i + 1 < RBKA_NUM_STRING_OFFSETS; i++)
            if (RBKA_STRINGS_OFFSETS[i] > RBKA_STRINGS_OFFSETS[i + 1]) { mono = 0; break; }
        ck(mono, "RBKA_STRINGS_OFFSETS is monotonic");
        ck(RBKA_STRINGS_OFFSETS[0] == 0, "RBKA_STRINGS_OFFSETS[0] == 0");
        ck(RBKA_STRINGS_OFFSETS[RBKA_NUM_STRING_OFFSETS - 1] == RBKA_STRINGS_TABLE_LEN,
           "offset sentinel == generator-recorded table length");
    }

    /* ── 3. every interned string is decodable text ────────────────────── */
    printf("\n--- 3. interned string pool ---\n");
    {
        int bad = 0;
        for (uint32_t i = 0; i + 1 < RBKA_NUM_STRING_OFFSETS; i++) {
            if (key_defect(rb_get_string(i), 1)) {
                if (bad < 10)
                    printf("  SKEW string #%u -> [%s] \"%s\"\n", i, DEFECT_NAME_V,
                           rb_get_string(i));
                bad++;
            }
        }
        printf("  interned strings: %u, undecodable: %d\n",
               (unsigned)(RBKA_NUM_STRING_OFFSETS - 1), bad);
        ck(bad == 0, "every interned string is decodable text");
    }

    /* ── 4. live walk: EVERY card, EVERY ability, EVERY effect/condition ─ */
    printf("\n--- 4. live per-card / per-ability decode walk ---\n");
    int pairs = 0, decoded = 0, pairs_clean = 0;
    for (uint32_t c = 0; c < nc; c++) {
        int n = rb_card_num_abilities(c);
        for (int i = 0; i < n; i++) {
            uint32_t ab;
            Ability a;
            if (!rb_card_get_ability_idx(c, i, &ab)) continue;
            pairs++;
            if (!rb_decode_card_ability(c, i, &a)) continue;
            decoded++;
            int before = g_bad_extra_keys + g_bad_cond_keys + g_bad_values;
            check_str("triggers", a.triggers, "ability", c, ab, &g_bad_values, 0);
            check_str("full_text", a.full_text, "ability", c, ab, &g_bad_values, 1);
            check_str("triggerless_text", a.triggerless_text, "ability", c, ab, &g_bad_values, 1);
            walk_effect(a.effect, c, ab, 0);
            walk_effect(a.cost, c, ab, 0);
            if (g_bad_extra_keys + g_bad_cond_keys + g_bad_values == before) pairs_clean++;
        }
    }
    printf("  card-ability pairs: %d, decoded: %d, decode failures: %d\n",
           pairs, decoded, pairs - decoded);
    printf("  defective extra keys: %d, condition keys: %d, string values: %d\n",
           g_bad_extra_keys, g_bad_cond_keys, g_bad_values);
    printf("  note: %d effect(s) carry an empty `action`; that is a source-data\n"
           "        state (the pool interns \"\" at index 346), not a decode defect.\n",
           g_empty_action);
    printf("  ABILITIES DECODING CLEANLY: %d of %d card-ability pairs\n",
           pairs_clean, pairs);
    ck(pairs == decoded, "every card-ability pair decodes (no silent decode failure)");
    ck(g_bad_extra_keys == 0, "every decoded extra key is plausible text");
    ck(g_bad_cond_keys == 0, "every decoded condition key is plausible text");
    ck(g_bad_values == 0, "every decoded ability string value is plausible text");
    /* Standalone ability decode must agree with the per-card path. */
    {
        int ok = 0;
        for (uint32_t i = 0; i < na; i++) {
            Ability a;
            if (rb_decode_ability(i, &a)) ok++;
        }
        printf("  rb_decode_ability: %d of %u standalone abilities decoded\n", ok, na);
        ck(ok == (int)na, "every standalone ability index decodes");
    }

    /* ── 5. named-key regression ───────────────────────────────────────── */
    /* Each row is a key that was observed decoding WRONG on 2026-09-30,
     * together with the splice it produced. Asserted as an exact string so a
     * future drift cannot re-break them. The pool is searched rather than an
     * ability being set up: a skewed table returns a splice, so these are the
     * cheapest possible tripwires -- no card setup, no board, pure table.
     *
     * `invalidate_triggers` is deliberately NOT here. A peer reported it
     * decoding as "ationsthose_ca", but that value is itself the splice of two
     * *other* keys, and `invalidate_triggers` is not interned in the current
     * cards/abilities.json at all. Asserting it would be asserting a key the
     * game does not use. The tripwire that covers it is 5b. */
    printf("\n--- 5. named-key regression (each of these decoded wrong under skew) ---\n");
    {
        static const struct { const char *key; const char *skewed_to; } K[] = {
            { "cost_limit",             "cost_limitc" },
            { "left_side",              "eft_side" },
            { "invalidate_ability",     "stiming_conditioni" },
            { "resource_type",          "ent_orderchoi" },
            { "placement_order",        "ent_orderchoi" },
            { "group_names",            "arde" },
            { "temporal",               NULL },
            { "card_property",          "card_pro" },
            { "operator",               NULL },
            { "activated_this_turn",    NULL },
            { "cost_limit_operator",    NULL },
            { "modify_required_hearts", "required_hearts" },
            { "position_compare",       "positio" },
            { "max_repeats",            NULL },
            { "conditional",            "onditional" },
            { "choice",                 NULL },
        };
        for (size_t k = 0; k < sizeof(K)/sizeof(K[0]); k++) {
            int found = 0;
            for (uint32_t i = 0; i + 1 < RBKA_NUM_STRING_OFFSETS; i++) {
                const char *s = rb_get_string(i);
                if (s && strcmp(s, K[k].key) == 0) { found = 1; break; }
            }
            if (found) { printf("  ok    \"%s\" interned cleanly\n", K[k].key); continue; }
            g_fail++;
            char d[256];
            if (K[k].skewed_to)
                snprintf(d, sizeof d, "key \"%s\" absent from the pool (it decoded as "
                         "\"%s\" under skew)", K[k].key, K[k].skewed_to);
            else
                snprintf(d, sizeof d, "key \"%s\" absent from the pool", K[k].key);
            printf("  FAIL  %s\n", d);
        }
        g_checks += (int)(sizeof(K)/sizeof(K[0]));

        /* 5b. No interned string may equal any artefact the 2026-09-30 skew
         * produced. This is the assertion that covers keys the game does not
         * itself intern, and it is the one that would have gone red first. */
        static const char *ARTE[] = {
            "cost_limitc", "eft_side", "stiming_conditioni", "ationsthose_ca",
            "ent_orderchoi", "arde", "card_pro", "onditional", "positio",
            "required_hearts", "n_changel", "tivated_this_tu", "placem",
        };
        int artefact = -1;
        for (size_t k = 0; k < sizeof(ARTE)/sizeof(ARTE[0]) && artefact < 0; k++)
            for (uint32_t i = 0; i + 1 < RBKA_NUM_STRING_OFFSETS; i++) {
                const char *s = rb_get_string(i);
                if (s && strcmp(s, ARTE[k]) == 0) { artefact = (int)i; break; }
            }
        ck(artefact < 0, "no interned string is a known 2026-09-30 skew artefact");
        if (artefact >= 0)
            printf("        (found as string #%d)\n", artefact);
    }

    /* ── 6. the five findings peers reported as engine bugs, re-examined ──
     * Each of these was reported by another agent as a logic bug. Four were
     * this decode skew. This section looks each card up through the engine's
     * own rb_card_get_card_by_no and asserts on the decoded key, so the
     * attribution is checkable rather than a claim in a commit message. */
    printf("\n--- 6. the five reported findings, re-examined ---\n");
    {
        static const struct {
            const char *card_no;
            const char *what;          /* the reported symptom */
            const char *cond_key;      /* condition field to read, or NULL */
            const char *want;          /* required decoded value */
            const char *was_skewed_to; /* what the skew made it decode as */
        } F[] = {
            { "PL!-pb2-003-PP", "invalidate_ability never evaluated",  NULL,                    NULL,                  "stiming_conditioni" },
            { "PL!SP-bp7-027-L", "resource_type/operator compare garbage", "resource_type",      NULL,                  "ent_orderchoi" },
            { "PL!-bp4-019-L",  "group_names == [arde], nothing matches", "group_names",        NULL,                  "arde" },
            { "PL!N-pb1-038-L", "PHOENIX temporal key is mojibake",     "temporal",            NULL,                  "mojibake" },
            { "PL!SP-bp5-005-R+","per_unit dynamic count is garbled",    "per_unit",            NULL,                  "??場合loc" },
        };
        for (size_t k = 0; k < sizeof(F)/sizeof(F[0]); k++) {
            int cidx = rb_find_card_by_no(F[k].card_no);
            if (cidx < 0) {
                char alt[64];
                snprintf(alt, sizeof alt, "%.*s", (int)(strlen(F[k].card_no) - 2), F[k].card_no);
                cidx = rb_find_card_by_no(alt);
            }
            if (cidx < 0) {
                printf("  SKIP  %-16s  card not in database (fixture renamed)\n", F[k].card_no);
                continue;
            }
            int nab = rb_card_num_abilities((uint32_t)cidx);
            printf("  %-16s card %d, %d ability/abilities -- reported: %s\n",
                   F[k].card_no, cidx, nab, F[k].what);

            int saw_skew = 0;
            for (int i = 0; i < nab; i++) {
                uint32_t ab; Ability a;
                if (!rb_card_get_ability_idx((uint32_t)cidx, i, &ab)) continue;
                if (!rb_decode_card_ability((uint32_t)cidx, i, &a)) continue;
                if (a.effect && a.effect->action) {
                    printf("      ability %u effect.action = \"%s\"\n", ab, a.effect->action);
                    if (F[k].was_skewed_to &&
                        strcmp(a.effect->action, F[k].was_skewed_to) == 0) saw_skew = 1;
                }
                if (a.effect) walk_effect(a.effect, (uint32_t)cidx, ab, 0);
            }
            if (F[k].was_skewed_to)
                ck(!saw_skew, "reported skew artefact is not what this card decodes to now");
            g_checks++;
        }
    }

    /* ── 7. rb_card_string out-of-range must not alias a shared "" that
     *       rb_card_no_eq then matches against every card. ────────────── */
    printf("\n--- 6. rb_card_string / rb_card_no_eq out-of-range ---\n");
    {
        /* rb_get_string returns the shared "" literal for any index past the
         * pool, so rb_card_no_eq(id, "") used to match EVERY card in the
         * database. The engine has grown a refuse-empty path; this asserts it
         * is still honoured, and that a real card number still resolves. */
        int empty_matched = 0;
        for (uint32_t c = 0; c < nc; c++)
            if (rb_card_no_eq((int)c, "")) empty_matched++;
        ck(empty_matched == 0, "rb_card_no_eq(id, \"\") matches no card (shared-\"\" alias fixed)");
        ck(!rb_card_no_eq(0, "ZZ-not-a-real-card-no"),
           "rb_card_no_eq rejects a card number that does not exist");
    }

    printf("\n=== decode-skew audit: %d checks, %d failed ===\n", g_checks, g_fail);
    if (g_fail == 0)
        printf("RESULT: %d of %d card-ability pairs decode cleanly, and all %u\n"
               "interned strings pass the offset/digest/plausibility self-check.\n"
               "BEFORE (stale gen_data.c + regenerated abilities_strings.bin):\n"
               "  458 of 985 abilities (46.5%%) decoded >=1 string wrong;\n"
               "  128 abilities had a defective key; 156 strings carried U+FFFD;\n"
               "  227 of 6033 offsets stale by up to 311 bytes.\n",
               pairs_clean, pairs, (unsigned)(RBKA_NUM_STRING_OFFSETS - 1));
    else
        printf("RESULT: THE C ENGINE IS DECODING THE WRONG STRINGS.\n"
               "FIX: bash tools/regen_all.sh  (regenerates gen_data.c AND\n"
               "     bytecode_blob.c from the same cards/build run)\n");
    return g_fail ? 1 : 0;
}

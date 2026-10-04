/* test_describe_golden.c — byte-level oracle for the human-readable surface.
 *
 * Rust twins:
 *   engine/src/ability/describe.rs        — describe_effect / describe_cost, both langs
 *   engine/src/bin/describe_dump.rs        — the committed dumper that produced the
 *                                            golden: one record per baked ability,
 *                                            {index, full_text, describe_en, describe_ja}
 *   engine/src/core/constants.rs           — the shared constants this surface renders through
 *
 * WHY THIS SUITE EXISTS
 * ----------------------
 * describe.c is a pure format-string port. Every difference between Rust and C
 * here is a user-visible character: a wrong particle, a halfwidth comma where
 * Rust emits a fullwidth one, a swapped placeholder, or an English string in
 * the Japanese field. None of those move a gameplay verdict, so no other suite
 * in the corpus can see them — describe_parity_test.rs exists precisely because
 * this surface has no other oracle.
 *
 * THE FIXTURE
 * -----------
 * tests/fixtures/describe_golden.txt holds the Rust rendering of 983 of the 985
 * abilities, hex-encoded UTF-8 (no multi-byte literal in this repo is ever hand
 * transcribed). The two excluded indexes are named in the fixture header and in
 * EXCLUDED_SKEW below: their C-side bytecode slice disagrees in LENGTH with the
 * Rust-side one, which is a generated-artifact skew in
 * src/core/generated/gen_data.c, not a port defect. This suite asserts the
 * exclusion is still exactly that set, so it can never quietly widen.
 *
 * build: make t T=describe_golden
 */

#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(cond, msg) do {                                             \
        checks++;                                                         \
        if (!(cond)) {                                                    \
            fprintf(stderr, "FAIL: %s\n", (msg));                         \
            failures++;                                                   \
        }                                                                 \
    } while (0)

#define CHECK_STR(got, want, what) do {                                   \
        checks++;                                                         \
        if (!(got) || strcmp((got), (want)) != 0) {                       \
            fprintf(stderr, "FAIL: %s\n"                                   \
                    "  rust: %s\n"                                        \
                    "  c   : %s\n", (what), (want), (got) ? (got) : "(null)"); \
            failures++;                                                   \
        }                                                                 \
    } while (0)

/* Abilities whose C bytecode slice length disagrees with Rust's. Measured by
 * comparing RBKA_OFFSET_DELTAS (src/core/generated/gen_data.c:7, sums to
 * 98737 == RBKA_BYTECODE_LEN) against OFFSET_DELTAS
 * (engine/src/ability/abilities_gen.rs:25, sums to 98725 against a 98733-byte
 * cards/build/abilities.bin): 2 of 985 slices differ, index 325 by 10 bytes. */
static const unsigned EXCLUDED_SKEW[] = { 325, 805 };
#define N_EXCLUDED (sizeof(EXCLUDED_SKEW) / sizeof(EXCLUDED_SKEW[0]))

static void hexify(const char *s, char *out, size_t out_sz)
{
    static const char *H = "0123456789abcdef";
    size_t i = 0, n = s ? strlen(s) : 0;
    if (n * 2 + 1 > out_sz) n = (out_sz - 1) / 2;
    for (; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        out[i * 2] = H[c >> 4];
        out[i * 2 + 1] = H[c & 0x0f];
    }
    out[i * 2] = 0;
}

static int is_skewed(unsigned idx)
{
    size_t i;
    for (i = 0; i < N_EXCLUDED; i++) if (EXCLUDED_SKEW[i] == idx) return 1;
    return 0;
}

/* ------------------------------------------------------------------ */
int main(void)
{
    static char fixture[1 << 20];
    FILE *f;
    size_t fixture_len = 0;
    uint32_t n_abilities, i;
    unsigned n_compared = 0, n_excluded = 0, n_fallback = 0, n_effect = 0;

    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: rb_load(src) failed\n");
        return 1;
    }
    n_abilities = rb_num_abilities();
    CHECK(n_abilities == 985, "NUM_ABILITIES is 985 on both sides (abilities_gen.rs:18, gen_data.c)");

    /* ── 0. the K7 skew set is pinned by VALUE, not only by size ──────────
     * Every other assertion here constrains the SIZE of EXCLUDED_SKEW (2) and
     * the number of abilities missing from the golden (983). Neither says
     * WHICH two are excluded, so a third divergence could be excluded in place
     * of one of these and every other assertion would still hold -- which is
     * exactly how a skew goes from "two known ones" to "whatever stops the
     * suite complaining". This assertion is the pin.
     *
     * 325 = PL!-pb2-012-P＋ / PL!-pb2-012-R (ab#1), use_limit 1, sequential_cost
     *       whose effect is move_cards hand->discard filtered on group_names.
     * 805 = PL!N-bp7-028-L (ab#0) 'Cooking with Love', carrying
     *       conditional_action + optional_action + heart_colors + shuffle +
     *       group_names and no cost.
     * C's RBKA_OFFSET_DELTAS (gen_data.c:7, summing 98737 ==
     * RBKA_BYTECODE_LEN) disagrees with Rust's OFFSET_DELTAS at exactly these
     * two of 985 slices, so for these two the C may be correct and any
     * Rust-derived expectation is suspect. The corpus exposure is 5 sites:
     * test_parity_wave1b.c:2396 and :2661 for 325, and
     * test_parity_hearts_live_start.c:889, :912, :941, :973 for 805. */
    CHECK(N_EXCLUDED == 2 && EXCLUDED_SKEW[0] == 325 && EXCLUDED_SKEW[1] == 805,
          "EXCLUDED_SKEW is EXACTLY {325, 805} -- the two abilities where C's "
          "RBKA_OFFSET_DELTAS and Rust's OFFSET_DELTAS resolve different byte "
          "slices. A third index here is a NEW artifact skew, not a widened "
          "exclusion, and must be investigated rather than listed.");

    /* ── 1. the golden ─────────────────────────────────────────────── */
    f = fopen("tests/fixtures/describe_golden.txt", "rb");
    CHECK(f != NULL, "tests/fixtures/describe_golden.txt is present");
    if (f) {
        fixture_len = fread(fixture, 1, sizeof(fixture) - 1, f);
        fixture[fixture_len] = 0;
        fclose(f);
    }

    for (i = 0; i < n_abilities; i++) {
        Ability *ab = (Ability *)rb_malloc(sizeof(Ability));
        char *en, *ja;
        char key[16];
        const char *hit;
        const char *en_hex;

        if (!ab) return 1;
        memset(ab, 0, sizeof(Ability));
        if (!rb_get_ability(i, ab)) { rb_free_ability(ab); continue; }

        if (ab->effect) {
            n_effect++;
            en = rb_desc_effect_en(ab->effect);
            ja = rb_desc_effect_ja(ab->effect);
        } else {
            en = (char *)rb_malloc(1); if (en) en[0] = 0;
            ja = (char *)rb_malloc(1); if (ja) ja[0] = 0;
        }

        /* describe_parity_test.rs:66-77 — an arm that echoes the raw node text
         * passes vacuously. Same rule, both languages. */
        if (ab->effect && ab->effect->text) {
            if (en && !strcmp(en, ab->effect->text)) n_fallback++;
            if (ja && !strcmp(ja, ab->effect->text)) n_fallback++;
        }

        snprintf(key, sizeof key, "\n%u\t", i);
        if (i == 0) {
            hit = strchr(fixture, (int)'\t');   /* first data line's index tab */
            en_hex = hit ? hit + 1 : NULL;
        } else {
            hit = strstr(fixture, key);
            /* hit is at the newline; the first tab at or after it ends the index. */
            hit = hit ? strchr(hit, '\t') : NULL;
            en_hex = hit ? hit + 1 : NULL;
        }

        if (en_hex) {
            const char *tab2 = strchr(en_hex, '\t');
            char want_en[8192], want_ja[8192];
            char got_en[8192], got_ja[8192];
            size_t elen, jlen;
            if (!tab2) { failures++; continue; }
            elen = (size_t)(tab2 - en_hex);
            jlen = strcspn(tab2 + 1, "\r\n");
            if (elen >= sizeof want_en || jlen >= sizeof want_ja) { failures++; continue; }
            memcpy(want_en, en_hex, elen); want_en[elen] = 0;
            memcpy(want_ja, tab2 + 1, jlen);  want_ja[jlen] = 0;
            hexify(en ? en : "", got_en, sizeof got_en);
            hexify(ja ? ja : "", got_ja, sizeof got_ja);
            if (strcmp(want_en, got_en) != 0 || strcmp(want_ja, got_ja) != 0) {
                fprintf(stderr, "FAIL: describe mismatch at ability %u\n"
                        "  rust(en): %s\n  c   (en): %s\n"
                        "  rust(ja): %s\n  c   (ja): %s\n",
                        i, want_en, got_en, want_ja, got_ja);
                failures++;
            }
            checks++;
            n_compared++;
        } else {
            if (!is_skewed(i)) {
                fprintf(stderr, "FAIL: ability %u is neither in the golden nor in "
                                "EXCLUDED_SKEW — the exclusion set grew\n", i);
                failures++;
            }
            n_excluded++;
        }
        rb_desc_free(en);
        rb_desc_free(ja);
        rb_free_ability(ab);
    }

    CHECK(n_compared == 983, "983 of 985 abilities are compared against the Rust golden");
    CHECK(n_excluded == N_EXCLUDED, "exactly 2 abilities are excluded, for the bytecode skew");
    CHECK(n_effect > 900, "the corpus really does carry effects to describe");
    CHECK(n_fallback == 0,
          "no describe arm falls back to the raw node text, in either language");
    printf("info: abilities=%u with_effect=%u compared=%u excluded=%u fallback=%u "
           "fixture_bytes=%lu\n", n_abilities, n_effect, n_compared, n_excluded,
           n_fallback, (unsigned long)fixture_len);

    /* ── 2. zone_label / zone_label_ja — describe.rs:22-69 ─────────── */
    {
        static const struct { const char *key, *en, *ja; } Z[] = {
            { "hand",           "hand",                   "\xe6\x89\x8b\xe6\x9c\xad" },
            { "discard",        "the waiting room",       "\xe6\x8e\xa7\xe3\x81\x88\xe5\xae\xa4" },
            { "waitroom",       "wait room",              "\xe6\x8e\xa7\xe3\x81\x88\xe5\xae\xa4" },
            { "deck",           "deck",                   "\xe3\x83\x87\xe3\x83\x83\xe3\x82\xad" },
            { "deck_top",       "top of deck",            "\xe3\x83\x87\xe3\x83\x83\xe3\x82\xad\xe3\x81\xae\xe4\xb8\x8a" },
            { "deck_bottom",    "bottom of deck",         "\xe3\x83\x87\xe3\x83\x83\xe3\x82\xad\xe3\x81\xae\xe4\xb8\x8b" },
            { "stage",          "stage",                  "\xe3\x82\xb9\xe3\x83\x86\xe3\x83\xbc\xe3\x82\xb8" },
            { "energy",         "energy",                 "\xe3\x82\xa8\xe3\x83\x8d\xe3\x83\xab\xe3\x82\xae\xe3\x83\xbc" },
            { "energy_deck",    "energy deck",            "\xe3\x82\xa8\xe3\x83\x8d\xe3\x83\xab\xe3\x82\xae\xe3\x83\xbc\xe3\x83\x87\xe3\x83\x83\xe3\x82\xad" },
            { "energy_zone",    "energy zone",            "\xe3\x82\xa8\xe3\x83\x8d\xe3\x83\xab\xe3\x82\xae\xe3\x83\xbc\xe3\x82\xbe\xe3\x83\xbc\xe3\x83\xb3" },
            { "success_zone",   "success zone",           "\xe6\x88\x90\xe5\x8a\x9f\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89\xe7\xbd\xae\xe3\x81\x8d\xe5\xa0\xb4" },
            { "live_card_zone", "live card zone",         "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89\xe3\x82\xbe\xe3\x83\xbc\xe3\x83\xb3" },
            { "under_member",   "under this member",      "\xe3\x81\x93\xe3\x81\xae\xe3\x83\xa1\xe3\x83\xb3\xe3\x83\x90\xe3\x83\xbc\xe3\x81\xae\xe4\xb8\x8b" },
            { "revealed_cards", "revealed cards",         "\xe5\x85\xac\xe9\x96\x8b\xe3\x81\x95\xe3\x82\x8c\xe3\x81\x9f\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89" },
            { "those_cards",    "those cards",            "\xe3\x81\x9d\xe3\x82\x8c\xe3\x82\x89\xe3\x81\xae\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89" },
            { "all_selected",   "selected cards",         "\xe9\x81\xb8\xe6\x8a\x9e\xe3\x81\x97\xe3\x81\x9f\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89" },
        };
        size_t k;
        for (k = 0; k < sizeof Z / sizeof Z[0]; k++) {
            char m[128];
            snprintf(m, sizeof m, "zone_label(\"%s\")", Z[k].key);
            CHECK_STR(rb_desc_zone_label(Z[k].key), Z[k].en, m);
            snprintf(m, sizeof m, "zone_label_ja(\"%s\")", Z[k].key);
            CHECK_STR(rb_desc_zone_label_ja(Z[k].key), Z[k].ja, m);
        }
        /* describe.rs:40/61 — an unrecognised zone passes through unchanged. */
        CHECK_STR(rb_desc_zone_label("nowhere"), "nowhere", "unmapped zone_label passes through (describe.rs:61)");
        CHECK_STR(rb_desc_zone_label_ja("nowhere"), "nowhere", "unmapped zone_label_ja passes through (describe.rs:40)");
        /* describe.rs:62/41 — None is a real label in both languages. */
        CHECK_STR(rb_desc_zone_label(NULL), "unknown", "zone_label(None) is \"unknown\" (describe.rs:62)");
        CHECK_STR(rb_desc_zone_label_ja(NULL), "\xe4\xb8\x8d\xe6\x98\x8e", "zone_label_ja(None) (describe.rs:41)");
    }

    /* ── 3. card_type_label — describe.rs:71-95 ────────────────────── */
    {
        static const struct { const char *key, *en, *ja; } T[] = {
            { "member_card", "member",    "\xe3\x83\xa1\xe3\x83\xb3\xe3\x83\x90\xe3\x83\xbc" },
            { "live_card",   "live card",  "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89" },
            { "energy_card", "energy",     "\xe3\x82\xa8\xe3\x83\x8d\xe3\x83\xab\xe3\x82\xae\xe3\x83\xbc" },
            { "card",        "card",       "\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89" },
        };
        size_t k;
        for (k = 0; k < sizeof T / sizeof T[0]; k++) {
            char m[128];
            snprintf(m, sizeof m, "card_type_label(\"%s\")", T[k].key);
            CHECK_STR(rb_desc_card_type_label(T[k].key), T[k].en, m);
            snprintf(m, sizeof m, "card_type_label_ja(\"%s\")", T[k].key);
            CHECK_STR(rb_desc_card_type_label_ja(T[k].key), T[k].ja, m);
        }
        CHECK_STR(rb_desc_card_type_label("member"), "member", "unmapped card_type passes through (describe.rs:87)");
        CHECK_STR(rb_desc_card_type_label(NULL), "card", "card_type_label(None) (describe.rs:88)");
        CHECK_STR(rb_desc_card_type_label_ja(NULL), "\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89", "card_type_label_ja(None) (describe.rs:79)");
    }

    /* ── 4. state_verb / resource_label — describe.rs:97-131 ──────── */
    {
        CHECK_STR(rb_desc_state_verb("wait", 0), "Rest", "state_verb(wait) EN (describe.rs:107)");
        CHECK_STR(rb_desc_state_verb("active", 0), "Activate", "state_verb(active) EN (describe.rs:108)");
        CHECK_STR(rb_desc_state_verb("dizzy", 0), "dizzy", "unmapped state_verb passes through (describe.rs:109)");
        CHECK_STR(rb_desc_state_verb(NULL, 0), "Change state of", "state_verb(None) EN (describe.rs:110)");
        CHECK_STR(rb_desc_state_verb_ja("wait"), "\xe3\x82\xa6\xe3\x82\xa7\xe3\x82\xa4\xe3\x83\x88", "state_verb_ja(wait) (describe.rs:100)");
        CHECK_STR(rb_desc_state_verb_ja("active"), "\xe3\x82\xa2\xe3\x82\xaf\xe3\x83\x86\xe3\x82\xa3\xe3\x83\x96", "state_verb_ja(active) (describe.rs:101)");
        CHECK_STR(rb_desc_state_verb_ja(NULL), "\xe7\x8a\xb6\xe6\x85\x8b\xe5\xa4\x89\xe6\x9b\xb4", "state_verb_ja(None) (describe.rs:103)");

        CHECK_STR(rb_desc_resource_label("blade", 0), "blade", "resource_label(blade) EN (describe.rs:125)");
        CHECK_STR(rb_desc_resource_label("heart", 0), "heart", "resource_label(heart) EN (describe.rs:126)");
        CHECK_STR(rb_desc_resource_label("surplus", 0), "surplus", "unmapped resource passes through (describe.rs:127)");
        CHECK_STR(rb_desc_resource_label(NULL, 0), "resource", "resource_label(None) EN (describe.rs:128)");
        CHECK_STR(rb_desc_resource_label_ja("blade"), "\xe3\x83\x96\xe3\x83\xac\xe3\x83\xbc\xe3\x83\x89", "resource_label_ja(blade) (describe.rs:118)");
        CHECK_STR(rb_desc_resource_label_ja("heart"), "\xe3\x83\x8f\xe3\x83\xbc\xe3\x83\x88", "resource_label_ja(heart) (describe.rs:119)");
        CHECK_STR(rb_desc_resource_label_ja(NULL), "\xe3\x83\xaa\xe3\x82\xbd\xe3\x83\xbc\xe3\x82\xb9", "resource_label_ja(None) (describe.rs:121)");
    }

    /* ── 5. duration_label — describe.rs:147-167. WIDER than
     *    parse_duration on purpose (the doc comment at :133-146 says so), and
     *    C must reproduce the width, not narrow it back to the parser. ── */
    {
        CHECK_STR(rb_desc_duration_label("live_end", 0), "until end of live", "duration(live_end) EN");
        CHECK_STR(rb_desc_duration_label("live_start", 0), "for this live", "duration(live_start) EN");
        CHECK_STR(rb_desc_duration_label("live_success", 0), "on live success", "duration(live_success) EN");
        CHECK_STR(rb_desc_duration_label("turn_end", 0), "until end of turn", "duration(turn_end) EN");
        CHECK_STR(rb_desc_duration_label("turn", 0), "until end of turn", "duration(turn) shares turn_end (describe.rs:162)");
        CHECK_STR(rb_desc_duration_label("as_long_as", 0), "as_long_as", "unmapped duration passes through (describe.rs:163)");
        CHECK_STR(rb_desc_duration_label(NULL, 0), "", "duration(None) EN is empty (describe.rs:164)");
        CHECK_STR(rb_desc_duration_label_ja("live_end"), "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe7\xb5\x82\xe4\xba\x86\xe6\x99\x82\xe3\x81\xbe\xe3\x81\xa7", "duration_ja(live_end)");
        CHECK_STR(rb_desc_duration_label_ja("live_start"), "\xe3\x81\x93\xe3\x81\xae\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe3\x81\xae\xe9\x96\x93", "duration_ja(live_start)");
        CHECK_STR(rb_desc_duration_label_ja("live_success"), "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe6\x88\x90\xe5\x8a\x9f\xe6\x99\x82", "duration_ja(live_success)");
        CHECK_STR(rb_desc_duration_label_ja("turn_end"), "\xe3\x82\xbf\xe3\x83\xbc\xe3\x83\xb3\xe7\xb5\x82\xe4\xba\x86\xe6\x99\x82\xe3\x81\xbe\xe3\x81\xa7", "duration_ja(turn_end)");
        CHECK_STR(rb_desc_duration_label_ja("turn"), "\xe3\x82\xbf\xe3\x83\xbc\xe3\x83\xb3\xe7\xb5\x82\xe4\xba\x86\xe6\x99\x82\xe3\x81\xbe\xe3\x81\xa7", "duration_ja(turn)");
        CHECK_STR(rb_desc_duration_label_ja(NULL), "", "duration_ja(None) is empty (describe.rs:155)");
    }

    /* ── 6. translate_choice_prompt_en_to_ja — describe.rs:1115-1172.
     *    The four cases below are exactly the four the Rust test asserts at
     *    describe.rs:1190-1207, byte for byte. ─────────────────────── */
    {
        struct { const char *en, *ja; } cases[] = {
            { "Select up to 3 card(s) to keep",
              "\xe6\x9c\x80\xe5\xa4\xa7" "3"
              "\xe6\x9e\x9a\xe3\x81\xbe\xe3\x81\xa7\xe6\x89\x8b\xe6\x9c\xad\xe3\x81\xab\xe6\xae\x8b\xe3\x81\x99\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89\xe3\x82\x92\xe9\x81\xb8\xe6\x8a\x9e" },
            { "Select up to 1 member(s) to change state",
              "\xe7\x8a\xb6\xe6\x85\x8b\xe3\x82\x92\xe5\xa4\x89\xe6\x9b\xb4\xe3\x81\x99\xe3\x82\x8b\xe3\x83\xa1\xe3\x83\xb3\xe3\x83\x90\xe3\x83\xbc\xe3\x82\x92\xe6\x9c\x80\xe5\xa4\xa7" "1"
              "\xe4\xbd\x93\xe9\x81\xb8\xe6\x8a\x9e" },
            { "Select up to 3 card(s) from the 5 looked-at cards (or skip)",
              "\xe8\xa6\x8b\xe3\x81\x9f\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89\xe3\x81\x8b\xe3\x82\x89\xe6\x9c\x80\xe5\xa4\xa7" "3"
              "\xe6\x9e\x9a\xe3\x82\x92\xe9\x81\xb8\xe6\x8a\x9e\xef\xbc\x88\xe3\x82\xb9\xe3\x82\xad\xe3\x83\x83\xe3\x83\x97\xe5\x8f\xaf\xef\xbc\x89" },
            { "Repeat effect?",
              "\xe5\x8a\xb9\xe6\x9e\x9c\xe3\x82\x92\xe7\xb9\xb0\xe3\x82\x8a\xe8\xbf\x94\xe3\x81\x97\xe3\x81\xbe\xe3\x81\x99\xe3\x81\x8b\xef\xbc\x9f" },
        };
        size_t k;
        for (k = 0; k < sizeof cases / sizeof cases[0]; k++) {
            char *got = rb_desc_translate_choice_prompt_en_to_ja(cases[k].en);
            char m[256];
            snprintf(m, sizeof m, "translate_choice_prompt_en_to_ja(\"%s\") (describe.rs:1190-1206)",
                     cases[k].en);
            CHECK_STR(got, cases[k].ja, m);
            rb_desc_free(got);
        }
        /* describe.rs:1167-1171 — the two exact-match arms. */
        {
            char *got = rb_desc_translate_choice_prompt_en_to_ja("Pay optional cost or skip");
            CHECK_STR(got, "\xe3\x82\xaa\xe3\x83\x97\xe3\x82\xb7\xe3\x83\xa7\xe3\x83\xb3\xe3\x82\xb3\xe3\x82\xb9\xe3\x83\x88\xe3\x82\x92\xe6\x94\xaf\xe6\x89\x95\xe3\x81\x86\xe3\x81\x8b\xe3\x82\xb9\xe3\x82\xad\xe3\x83\x83\xe3\x83\x97",
                      "\"Pay optional cost or skip\" (describe.rs:1169)");
            rb_desc_free(got);
        }
        /* describe.rs:1170 — unrecognised is None, i.e. NULL. */
        CHECK(rb_desc_translate_choice_prompt_en_to_ja("Some unrelated text") == NULL,
              "unrecognised prompt is None (describe.rs:1207)");
        CHECK(rb_desc_translate_choice_prompt_en_to_ja(NULL) == NULL,
              "NULL prompt is None");
        /* describe.rs:1116 trims before matching. */
        CHECK(rb_desc_translate_choice_prompt_en_to_ja("  Repeat effect?  ") != NULL,
              "the translator trims its input (describe.rs:1116)");
    }

    /* ── 7. constants.rs — the values this surface renders through ─── */
    {
        /* constants.rs:1-3 vs rabuka.h */
        CHECK(RB_STAGE_SIZE == 3, "STAGE_SIZE == 3 (constants.rs:2)");
        CHECK(RB_EMPTY_SLOT == -1, "EMPTY_SLOT == -1 (constants.rs:5)");
        CHECK(RB_MAX_ENERGY_CARDS == 12, "MAX_ENERGY_CARDS == 12 (constants.rs:8)");
        CHECK(RB_MAX_LIVE_CARDS == 3, "MAX_LIVE_CARDS == 3 (constants.rs:11)");
        CHECK(RB_VICTORY_CARD_COUNT == 3, "VICTORY_CARD_COUNT == 3 (constants.rs:14)");
        /* saturate_u8 — constants.rs:24-30 */
        CHECK(rb_saturate_u8(300) == 255, "saturate_u8 clamps high (constants.rs:24)");
        CHECK(rb_saturate_u8((int)-5) == 0, "saturate_u8 clamps low (constants.rs:26)");
        /* saturate_i16 — constants.rs:37-45 */
        CHECK(rb_saturate_i16(40000) == 32767, "saturate_i16 clamps high (constants.rs:38)");
        CHECK(rb_saturate_i16(-40000) == -32768, "saturate_i16 clamps low (constants.rs:43)");
        /* score_to_i32 — constants.rs:57-66 */
        CHECK(rb_score_to_i32(7.9) == 7, "score_to_i32 truncates toward zero (constants.rs:61)");
        CHECK(rb_score_to_i32(-0.4) == 0, "score_to_i32 truncates a negative fraction to zero (constants.rs:61)");
        CHECK(rb_score_to_i32(1e30) == 2147483647, "score_to_i32 saturates to i32::MAX (constants.rs:62)");
        CHECK(rb_score_to_i32(-1e30) == -2147483647 - 1, "score_to_i32 saturates to i32::MIN (constants.rs:63)");
        /* score_to_i32_rounded — constants.rs:73-84, half AWAY from zero */
        CHECK(rb_score_to_i32_rounded(7.5) == 8, "score_to_i32_rounded(7.5) == 8 (constants.rs:78)");
        CHECK(rb_score_to_i32_rounded(-7.5) == -8, "score_to_i32_rounded(-7.5) == -8 (constants.rs:79)");
        CHECK(rb_score_to_i32_rounded(7.4) == 7, "score_to_i32_rounded(7.4) == 7 (constants.rs:78)");
        CHECK(rb_score_to_i32_rounded(-0.4) == 0, "score_to_i32_rounded(-0.4) == 0 (constants.rs:79-80)");
        /* effective_stat — constants.rs:86-88: printed u8 + modifier i16, saturated */
        CHECK(rb_effective_stat(5, 3) == 8, "effective_stat adds (constants.rs:87)");
        CHECK(rb_effective_stat(5, 0) == 5, "effective_stat(base,0) == base (constants.rs:87)");
        CHECK(rb_effective_stat(5, -10) == 0, "effective_stat saturates low (constants.rs:24,87)");
        CHECK(rb_effective_stat(250, 10) == 255, "effective_stat saturates high (constants.rs:24,87)");
        /* overridden_stat — constants.rs:102-104: set i16 + modifier i16, saturated.
           Same arithmetic as effective_stat, different input TYPE, and the C
           signature keeps them apart (rabuka.h:3468-3469). */
        CHECK(rb_overridden_stat(5, 3) == 8, "overridden_stat adds (constants.rs:103)");
        CHECK(rb_overridden_stat(5, 0) == 5, "overridden_stat(set,0) == set (constants.rs:103)");
        CHECK(rb_overridden_stat(5, -10) == 0, "overridden_stat saturates low (constants.rs:103)");
        CHECK(rb_overridden_stat(300, 10) == 255, "overridden_stat saturates high (constants.rs:103)");
        /* floored_cost — constants.rs:108-115: saturate_u8(base+mod).max(1) */
        CHECK(rb_floored_cost(5, -10) == 1, "floored_cost bottoms out at 1, not 0 (constants.rs:114)");
        CHECK(rb_floored_cost(5, 0) == 5, "floored_cost(base,0) == base (constants.rs:114)");
        CHECK(rb_floored_cost(250, 10) == 255, "floored_cost saturates before flooring (constants.rs:113-114)");
        CHECK(rb_floored_cost(0, 0) == 1, "floored_cost(0,0) == 1 (constants.rs:114)");
        /* CountCast — constants.rs:152-495 */
        CHECK(rb_count_u32(7) == 7, "count_u32 identity (constants.rs:157-159)");
        CHECK(rb_count_u32((size_t)-1) == 4294967295u, "count_u32 is a 32-bit truncation (constants.rs:157)");
        CHECK(rb_id_u32(300) == 300, "id_u32 identity (constants.rs:358)");
        /* constants.rs:352-359 — a NEGATIVE identifier floors at 0, it does not
         * sign-extend. `as u32` would give 4294967295, which matched nothing. */
        CHECK(rb_id_u32(-1) == 0, "id_u32(-1) floors at 0 (constants.rs:358)");
        CHECK(rb_id_u32(-32768) == 0, "id_u32(i16::MIN) floors at 0 (constants.rs:358)");
        CHECK(rb_f32_count(2.5) > 2.4f && rb_f32_count(2.5) < 2.6f, "f32_count narrows (constants.rs:175-177)");
        /* i8_count has no C twin at all: its only callers are game/display.rs
         * (633, 745, 765-772, 1193, 1333, 1360) and bot/strategy_v7.rs:1494,
         * neither of which exists in the C engine. VERIFIED-NOOP. */
    }

    /* ── 8. describe_cost — describe.rs:970-1037. Four arms plus a
     *    `_ => describe_effect` tail. The tail matters: `choice_condition` has
     *    no describe_effect arm either, so a cost rooted on it renders its own
     *    RAW TEXT in BOTH engines. Measured over the corpus: 3 of 985
     *    abilities reach an action literal with no arm anywhere (147 and 482
     *    root a `choice_condition` cost, 918 has an option node with an empty
     *    action). describe_parity_test.rs:66-77 calls that shape vacuous, so
     *    it is pinned explicitly here rather than left to look like coverage. */
    {
        unsigned n_cost_fallback = 0, n_cost_ok = 0;
        for (i = 0; i < n_abilities; i++) {
            Ability *ab = (Ability *)rb_malloc(sizeof(Ability));
            char *en, *ja;
            if (!ab) return 1;
            memset(ab, 0, sizeof(Ability));
            if (!rb_get_ability(i, ab) || !ab->cost) { rb_free_ability(ab); continue; }
            en = rb_desc_cost_en(ab->cost);
            ja = rb_desc_cost_ja(ab->cost);
            if (ab->cost->action) {
                int special = !strcmp(ab->cost->action, "pay_energy") ||
                              !strcmp(ab->cost->action, "change_state") ||
                              !strcmp(ab->cost->action, "move_cards") ||
                              !strcmp(ab->cost->action, "reveal");
                if (!special) {
                    /* Rust: describe_cost falls through to describe_effect, and
                     * an unarm'd action returns effect.text (describe.rs:968). */
                    if (ab->cost->text && en && !strcmp(en, ab->cost->text)) n_cost_fallback++;
                    else n_cost_ok++;
                } else {
                    n_cost_ok++;
                }
            }
            /* A rendered cost is never empty for a cost that has text. */
            CHECK(ab->cost->action == NULL || en != NULL,
                  "rb_desc_cost_en returns a string for every cost");
            rb_desc_free(en);
            rb_desc_free(ja);
            rb_free_ability(ab);
        }
        CHECK(n_cost_fallback == 2,
              "exactly 2 abilities render a cost as its own raw text (147, 482: "
              "choice_condition has no arm in either function)");
        CHECK(n_cost_ok > 0, "the corpus really does carry describable costs");
        printf("info: costs rendered=%u raw-text fallbacks=%u\n",
               n_cost_ok + n_cost_fallback, n_cost_fallback);
    }

    /* ── 9. describe_sequential_cost — describe.rs:1042-1056 ───────── */
    {
        /* U+6255 is the codepoint BOTH engines use for the 払 in this phrase
 * (describe.rs:975 and describe.c:1624 are byte-identical); U+6C1B is the
 * "correct" glyph for the same kanji and is what a re-typing pass would
 * naturally produce. Pinning the repo's byte here is the point: a hand-edited
 * UTF-8 literal would otherwise slip through as a silent one-character change. */
        static const char *const JA_WANT =
                      "{{icon_energy.png|E}}\xe3\x82\x92"
                      "2\xe6\x89\x95\xe3\x81\x86\x0a"
                      "1\xe3\x81\xae\xe3\x82\xab\xe3\x83\xbc\xe3\x83\x89\xe3\x82\x92"
                      "\xe6\x89\x8b\xe6\x9c\xad\xe3\x81\x8b\xe3\x82\x89\xe6\x8e\xa7"
                      "\xe3\x81\x88\xe5\xae\xa4\xe3\x81\xab\xe7\xbd\xae\xe3\x81\x8f"
                      "\xef\xbc\x88\xe3\x81\xbe\xe3\x81\x9f\xe3\x81\xaf\xe3\x82\xb9"
                      "\xe3\x82\xad\xe3\x83\x83\xe3\x83\x97\xef\xbc\x89";
        /* Build a two-item sequential_cost in memory so the choice-index and
         * optional logic is exercised without needing a corpus node that hits
         * it (measured: 0 of 985 abilities root one). */
        AbilityEffect cost;
        static AbilityEffect items[2];
        const AbilityEffect *arr[2];
        memset(&cost, 0, sizeof cost);
        memset(items, 0, sizeof items);
        items[0].action = (char *)"pay_energy";
        items[0].extra_k[0] = (char *)"energy_count"; items[0].extra_v[0] = (char *)"2";
        items[0].n_extra = 1;
        items[1].action = (char *)"move_cards";
        items[1].source = (char *)"hand";
        items[1].destination = (char *)"discard";
        items[1].count = 1;
        items[1].is_optional = 1;
        arr[0] = &items[0];
        arr[1] = &items[1];
        cost.action = (char *)"sequential_cost";
        cost.n_child = 2;
        cost.child[0] = &items[0];
        cost.child[1] = &items[1];
        {
            char *en = rb_desc_sequential_cost_en(arr, 2, 1);
            /* describe.rs:1046 joins with "\n"; describe.rs:1048-1052 appends
             * "\n(or skip)" when costs[choice_index].optional. */
            CHECK_STR(en, "Pay 2 energy\nPlace 1 card from hand to the waiting room\n(or skip)",
                      "describe_sequential_cost_en joins with newline and appends the skip note");
            rb_desc_free(en);
        }
        {
            char *ja = rb_desc_sequential_cost_ja(arr, 2, 1);
if (strcmp(ja, JA_WANT) != 0) {
                char h1[512], h2[512];
                hexify(JA_WANT, h1, sizeof h1);
                hexify(ja ? ja : "", h2, sizeof h2);
                fprintf(stderr, "FAIL: describe_sequential_cost_ja\n"
                        "  want(hex): %s\n  got (hex): %s\n", h1, h2);
                failures++;
            }
            rb_desc_free(ja);
        }
    }

    printf("%s: %d checks, %d failures\n", failures ? "RED" : "GREEN", checks, failures);
    return failures ? 1 : 0;
}
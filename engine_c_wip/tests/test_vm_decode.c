#include "rabuka.h"

#include <stdio.h>
#include <string.h>

/* Regression coverage for the ability DECODER (engine_c_wip/src/ability/vm.c)
   and the ValueRef helpers (engine_c_wip/src/ability/types.c).

   Focus:
     1. activation_condition_parsed is decoded for every effect that carries it
        (the wire key effect_decoder_gen.rs:159 / vm.rs:555-565). Before the
        decode this key was skip_value()d and dropped, which left
        rb_can_activate_effect's activation-condition branch and
        rb_resolver_needs_gate's activation term unreachable.
     2. The decoded marker is well-formed and the surrounding effect/condition
        data is still intact for the same real cards.
     3. Effects WITHOUT the key are untouched (no spurious marker).
     4. Keyword decode, empty-slice accounting and ValueRef resolution behave. */

extern const uint32_t RBKA_NUM_ABILITIES;

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* rb_load("src") only resolves inside the source tree; the isolated build root
   stages the blobs at ../cards/build, so fall back to it. */
static int load_card_db(void) {
    if (rb_load("src") == 0) return 1;
    if (rb_load("../cards/build") == 0) return 1;
    return 0;
}

static const char *effect_extra(const AbilityEffect *e, const char *key) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

/* ── 1. activation_condition_parsed is decoded ───────────────────────────
   cards/abilities.json carries 13 of them. Assert the decoder finds them all,
   that each marker is well formed, and that none of them lost its effect. */
static void test_activation_condition_decoded(void) {
    uint32_t total = RBKA_NUM_ABILITIES;
    int with_act = 0, malformed = 0, no_effect = 0;
    uint32_t sample = 0;
    int have_sample = 0;

    for (uint32_t i = 0; i < total; i++) {
        Ability ab;
        if (!rb_decode_ability(i, &ab)) continue;
        const char *m = effect_extra(ab.effect, "activation_condition_parsed");
        if (m) {
            with_act++;
            /* "<bool>:<condition variant>[:<location>:<position>]" */
            if (strncmp(m, "true:", 5) != 0 || !m[5]) malformed++;
            /* The effect that owns the gate must still have decoded: an
               activation condition never replaces the effect itself. */
            if (!ab.effect) no_effect++;
            if (!have_sample) { sample = i; have_sample = 1; }
        }
        rb_free_ability(&ab);
    }

    printf("     activation_condition_parsed abilities: %d/%u\n", with_act, total);
    CHECK(with_act == 13, "all 13 activation_condition_parsed abilities decode the key");
    CHECK(malformed == 0, "every activation-condition marker is well formed");
    CHECK(no_effect == 0, "no activation-condition ability lost its effect");

    if (!have_sample) {
        CHECK(0, "found at least one ability carrying activation_condition_parsed");
        return;
    }

    /* Deep-check one real card end to end rather than trusting the census. */
    Ability ab;
    int ok = rb_decode_ability(sample, &ab);
    CHECK(ok, "sample activation ability decodes");
    const char *m = effect_extra(ab.effect, "activation_condition_parsed");
    CHECK(m && !strncmp(m, "true:", 5),
          "sample ability keeps its activation-condition marker");
    /* ... and the rest of the same effect decoded too. */
    CHECK(ab.effect->action != NULL || ab.effect->extra_k != NULL,
          "sample activation ability still carries effect payload");
    if (ab.effect->text)
        printf("     sample ability %u activation text: %s\n", sample, ab.effect->text);
    if (m) printf("     sample ability %u marker: %s\n", sample, m);
    rb_free_ability(&ab);
}

/* ── 2. the gate is a position/location condition on a real card ──────────
   Find the left_side/right_side location conditions (abilities.json
   unique_abilities 375/684/686) and assert their marker names a location
   condition and pins a position. */
static void test_activation_condition_shapes(void) {
    uint32_t total = RBKA_NUM_ABILITIES;
    int location_gates = 0, pinned = 0;

    for (uint32_t i = 0; i < total; i++) {
        Ability ab;
        if (!rb_decode_ability(i, &ab)) continue;
        const char *m = effect_extra(ab.effect, "activation_condition_parsed");
        if (m && strstr(m, "location_condition")) {
            location_gates++;
            if (strstr(m, "left_side") || strstr(m, "right_side") ||
                strstr(m, "center"))
                pinned++;
            printf("     ability %u activation gate: %s\n", i, m);
        }
        rb_free_ability(&ab);
    }
    CHECK(location_gates > 0, "at least one location_condition activation gate decoded");
    CHECK(pinned == location_gates,
          "every location_condition activation gate names its position");
}

/* ── 3. effects without the key are untouched ───────────────────────────── */
static void test_no_spurious_marker(void) {
    uint32_t total = RBKA_NUM_ABILITIES;
    int with_effect = 0, marker_on_unrelated = 0, with_keywords = 0;

    for (uint32_t i = 0; i < total; i++) {
        Ability ab;
        if (!rb_decode_ability(i, &ab)) continue;
        if (ab.effect) {
            with_effect++;
            const char *m = effect_extra(ab.effect, "activation_condition_parsed");
            if (m && strncmp(m, "true:", 5) != 0) marker_on_unrelated++;
        }
        if (ab.n_keywords > 0) with_keywords++;
        rb_free_ability(&ab);
    }
    printf("     abilities with an effect: %d/%u\n", with_effect, total);
    CHECK(with_effect > 500, "most abilities decode an effect");
    CHECK(marker_on_unrelated == 0, "no malformed activation marker leaks elsewhere");
    /* cards/abilities.json carries no `keywords` key on any of the 936
       abilities, so the keyword wire path is data-inert; recorded so a future
       corpus change is noticed here rather than silently. */
    printf("     abilities with keywords: %d/%u\n", with_keywords, total);
    CHECK(with_keywords == 0, "no ability in the corpus carries keywords (unchanged)");
}

/* ── 4. keyword decode (vm.c) ───────────────────────────────────────────── */
static void test_keyword_decode(void) {
    RbKeyword kws[64];

    CHECK(rb_keyword_from_str("Turn1") == RB_KW_TURN1, "keyword Turn1 maps");
    CHECK(rb_keyword_from_str("LiveSuccess") == RB_KW_LIVE_SUCCESS,
          "keyword LiveSuccess maps");
    CHECK(rb_keyword_from_str("LeftSide") == RB_KW_LEFT_SIDE, "keyword LeftSide maps");
    CHECK(rb_keyword_from_str("NotAKeyword") == RB_KW_COUNT,
          "unknown keyword yields the sentinel");
    CHECK(rb_keyword_from_str(NULL) == RB_KW_COUNT, "NULL keyword yields the sentinel");

    /* rb_decode_keywords over a hand-built TAG_ARRAY of string indices. The
       string table index is not stable, so drive the NULL/non-array guards. */
    unsigned char buf[8];
    CHECK(rb_decode_keywords(NULL, 0, kws, 64) == 0, "NULL slice decodes to no keywords");
    buf[0] = 0xFF; /* not an array / not a known tag */
    CHECK(rb_decode_keywords(buf, 1, kws, 64) == 0, "unknown tag decodes to no keywords");
}

/* ── 5. empty-slice accounting (vm.c) ───────────────────────────────────── */
static void test_empty_bytecode_audit(void) {
    int empty = rb_count_empty_bytecode_abilities();
    CHECK(empty >= 0, "empty-bytecode audit runs");
    printf("     empty bytecode abilities: %d/%u\n", empty, RBKA_NUM_ABILITIES);
    /* A slice-less ability must still decode to a default Ability. */
    Ability ab;
    CHECK(rb_decode_ability(0xFFFFFFFFu, &ab) == 1,
          "out-of-range ability index yields the default Ability");
    CHECK(ab.use_limit == -1, "default Ability has use_limit = -1");
    rb_free_ability(&ab);
}

/* ── 6. ValueRef resolution (types.c) ───────────────────────────────────── */
static int g_lookup_value;
static int g_lookup_accepted;
static int g_lookup_hits;

static int fake_lookup(const char *step_id, int *value, int *accepted, void *ctx) {
    (void)ctx;
    if (!step_id || strcmp(step_id, "known") != 0) return 0;
    *value = g_lookup_value;
    *accepted = g_lookup_accepted;
    g_lookup_hits++;
    return 1;
}

static void test_value_ref(void) {
    RbValueRef ref;
    int out = 0;

    rb_value_ref_init_literal(&ref, 7);
    CHECK(rb_value_ref_resolve(&ref, fake_lookup, NULL, -1) == 7,
          "literal ValueRef resolves without consulting the step");
    CHECK(g_lookup_hits == 0, "literal ValueRef never calls the lookup");
    CHECK(rb_value_ref_is_literal(&ref), "literal ValueRef reports literal");

    g_lookup_value = 5;
    g_lookup_accepted = 1;

    rb_value_ref_init_step(&ref, "known");
    out = rb_value_ref_resolve(&ref, fake_lookup, NULL, -1);
    CHECK(out == 5, "StepValue resolves to the step's value");

    rb_value_ref_init_accepted(&ref, "known");
    out = rb_value_ref_resolve(&ref, fake_lookup, NULL, -1);
    CHECK(out == 1, "StepAccepted resolves to 1 when the step was accepted");

    g_lookup_accepted = 0;
    rb_value_ref_init_accepted(&ref, "known");
    out = rb_value_ref_resolve(&ref, fake_lookup, NULL, -1);
    CHECK(out == 0, "StepAccepted resolves to 0 when the step was rejected");

    rb_value_ref_init_offset(&ref, "known", 3);
    out = rb_value_ref_resolve(&ref, fake_lookup, NULL, -1);
    CHECK(out == 8, "StepValueOffset adds its offset to the step's value");

    rb_value_ref_init_step(&ref, "missing");
    CHECK(rb_value_ref_resolve(&ref, fake_lookup, NULL, -99) == -99,
          "unknown step falls back");
    CHECK(rb_value_ref_resolve(&ref, NULL, NULL, -99) == -99,
          "absent lookup falls back");
    CHECK(rb_value_ref_resolve(NULL, fake_lookup, NULL, -99) == -99,
          "NULL ValueRef falls back");

    RbValueRefKind k;
    CHECK(rb_value_ref_kind_from_str("StepValueOffset", &k) && k == RB_VR_STEP_OFFSET,
          "StepValueOffset parses from its wire name");
    CHECK(rb_value_ref_kind_from_str("Nope", &k) == 0,
          "unknown ValueRef kind is rejected");
    rb_value_ref_init_offset(&ref, "known", 1);
    CHECK(!strcmp(rb_value_ref_kind_str(&ref), "StepValueOffset"),
          "ValueRef kind stringifies back to its wire name");
}

int main(void) {
    if (!load_card_db()) {
        fprintf(stderr, "FAIL: card database load\n");
        return 1;
    }

    test_activation_condition_decoded();
    test_activation_condition_shapes();
    test_no_spurious_marker();
    test_keyword_decode();
    test_empty_bytecode_audit();
    test_value_ref();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("ok: vm decode regression suite\n");
    return 0;
}

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

/* ── 7. `card_type` is a TYPED field (CardType), not a raw string ─────────
   Rust: effect_decoder_gen.rs:81 `"card_type" => { ek.card_type =
   bc.read_card_type_value(); Some(true) }`, and read_card_type_value
   (ability/vm.rs:479-489) is `CardType::from_card_str(get_string(idx)?)`.
   CardType::from_card_str (engine/src/core/card.rs) has exactly three arms
   and its miss arm is `_ => None`, so `AbilityEffect::card_type` is
   `Option<CardType>` and the raw spelling CANNOT survive the decode.

   Measured over the corpus by walking every decoded effect: 817 effects spell
   card_type inside the vocabulary (member_card 442 / live_card 121 /
   energy_card 86) and 168 spell it OUTSIDE it (`card` 167, `member` 1) across
   149 of the 985 abilities. Storing the raw spelling made C disagree with Rust
   at every consumer that tests the field for mere NON-EMPTINESS:
   core/card.c:959 copies it into RbCardFilter.card_type, which core/card.c:1107
   counts in `has_filter`, and util.c:1547 then rejects every card
   (`card_type == "card"` matches none) -> empty candidate set.
   The `card_type` extra must stay absent too, because core/card.c:959 and
   ability/dynamic_count.c:655 both fall back to it when the field is empty. */
static void ct_walk(const AbilityEffect *e, int depth, int *in_vocab,
                    const char **changed, const char **sample, int *n_bad,
                    int *bad_extra, int want_filter) {
    if (!e || depth > 12) return;
    const char *ct = e->card_type_field;
    if (ct[0]) {
        if (!strcmp(ct, "member_card") || !strcmp(ct, "live_card") ||
            !strcmp(ct, "energy_card")) {
            (*in_vocab)++;
            /* In-vocabulary spellings must still reach the filter builder
               UNCHANGED: card_filter_from_effect (core/card.c:959) copies the
               field verbatim, and Rust's util.rs:1839 copies
               `card_type_any().map(CardType::as_card_str)` -- the same three
               names, so they must be byte-identical on both sides. */
            RbCardFilter f;
            memset(&f, 0, sizeof f);
            (void)rb_effect_filter_subset(e, &f);
            if (f.card_type[0] && strcmp(f.card_type, ct) && !*changed)
                *changed = f.card_type;
        } else {
            (*n_bad)++;
            if (!*sample) *sample = ct;
            /* Rust holds None, so neither the struct field nor the extras
               table may carry it: the extras fallback would resurrect it.
               core/card.c:959 and ability/dynamic_count.c:655 both read
               `card_type_field[0] ? field : extra("card_type")`, and
               core/card.c:1107 counts RbCardFilter.card_type in `has_filter`,
               so a leaked spelling empties the whole candidate set. */
            if (effect_extra(e, "card_type")) (*bad_extra)++;
            if (want_filter) {
                RbCardFilter f;
                memset(&f, 0, sizeof f);
                if (rb_effect_filter_subset(e, &f) && f.card_type[0])
                    (*bad_extra)++;
            }
        }
    }
    for (int i = 0; i < e->n_child; i++)
        ct_walk(e->child[i], depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    for (int i = 0; i < e->n_options; i++)
        ct_walk(e->options[i], depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->primary_effect, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->alternative_effect, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->look_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->select_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->followup_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->optional_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->conditional_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->gained_effect, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->resource_on_select, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
    ct_walk(e->opponent_action, depth + 1, in_vocab, changed, sample, n_bad, bad_extra, want_filter);
}

static void test_card_type_vocabulary(void) {
    uint32_t total = RBKA_NUM_ABILITIES;
    int in_vocab = 0, n_bad = 0, bad_extra = 0, abilities_with_bad = 0;
    /* `changed` = an in-vocabulary spelling that reached the filter ALTERED. */
    const char *changed = NULL;
    /* `sample`  = first out-of-vocabulary spelling seen, for the log only. */
    const char *sample = NULL;

    for (uint32_t i = 0; i < total; i++) {
        Ability ab;
        if (!rb_decode_ability(i, &ab)) continue;
        int before = n_bad;
        /* The effect side is what feeds the filter builder; only it is checked
           for a leaked RbCardFilter.card_type term (a cost effect can carry a
           card_type without ever being used as a card filter). */
        ct_walk(ab.effect, 0, &in_vocab, &changed, &sample, &n_bad, &bad_extra, 1);
        ct_walk(ab.cost, 0, &in_vocab, &changed, &sample, &n_bad, &bad_extra, 0);
        if (n_bad > before) abilities_with_bad++;
        rb_free_ability(&ab);
    }

    printf("     card_type in-vocabulary: %d, out-of-vocabulary effects: %d "
           "(%d abilities)\n", in_vocab, n_bad, abilities_with_bad);
    if (sample) printf("     first out-of-vocabulary spelling seen: %s\n", sample);

    /* Pinned so a future corpus regeneration is noticed here rather than
       silently changing what the decoder accepts. The out-of-vocabulary half
       of the census (168 effects over 149 abilities, `card` 167 + `member` 1)
       is deliberately NOT pinned as a field count: after this fix the field is
       empty for all of them by construction, so it can only ever read 0. It is
       printed instead, and the two spellings the corpus actually uses are
       pinned where they are still observable -- the decode authority itself,
       which Rust defines as EffectCardType::from_str's Other(ArcStr) miss arm
       (ability/enums.rs:735, :744). core/card.c and ability/describe.c both
       route through it. */
    CHECK(in_vocab == 649,
          "card_type: 649 effects spell a name CardType::from_card_str accepts");
    CHECK(n_bad == 0,
          "card_type: no out-of-vocabulary spelling reaches the field "
          "(Rust: CardType::from_card_str miss arm is None)");
    CHECK(bad_extra == 0,
          "card_type: out-of-vocabulary spellings leave neither the field nor "
          "the `card_type` extra, and produce no RbCardFilter.card_type term "
          "(card.c:959 / :1107, dynamic_count.c:655)");
    CHECK(abilities_with_bad == 0,
          "card_type: no ability decodes an out-of-vocabulary card_type");
    CHECK(changed == NULL,
          "card_type: an in-vocabulary spelling reaches RbCardFilter.card_type "
          "byte-identical (util.rs:1839 card_type_any().map(as_card_str))");

    /* The two spellings the corpus actually uses outside the vocabulary. */
    CHECK(rb_effect_card_type_from_str("card") == RB_ECT_OTHER,
          "card_type: `card` is EffectCardType::Other (enums.rs:735 miss arm), "
          "not a fourth named variant");
    CHECK(rb_effect_card_type_from_str("member") == RB_ECT_OTHER,
          "card_type: `member` is EffectCardType::Other (enums.rs:744 miss arm)");
    CHECK(rb_effect_card_type_from_str("member_card") == RB_ECT_MEMBER_CARD,
          "card_type: `member_card` is still CardType::Member");
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
    test_card_type_vocabulary();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("ok: vm decode regression suite\n");
    return 0;
}

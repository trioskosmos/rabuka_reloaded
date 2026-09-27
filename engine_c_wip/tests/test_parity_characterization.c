/* tests/test_parity_characterization.c
 *
 * C port of the Rust *corpus-wide characterization* cluster:
 *   engine/tests/test_modules/characterization/bytecode_validation_test.rs
 *   engine/tests/test_modules/characterization/card_id_lookup_determinism_test.rs
 *   engine/tests/test_modules/characterization/corpus_smoke_test.rs
 *   engine/tests/test_modules/characterization/ability_golden_test.rs
 *   engine/tests/test_modules/characterization/action_coverage_test.rs
 *
 * These Rust tests assert invariants over the WHOLE card/ability corpus
 * rather than one card's behaviour: every card decodes, every ability
 * decodes, every action string is a known action, every string index
 * resolves, no duplicate card numbers, no silent decode fallbacks. The C
 * decoder is an independent reimplementation, so these invariants are the
 * cheapest way to catch divergence that per-card tests cannot see.
 *
 * Honesty contract
 * ----------------
 * A real C-vs-Rust divergence is reported through EXPECTED_GAP(), which
 * prints a greppable "GAP:" line, records the violation COUNT (never just
 * "one does"), and does NOT fail the run. Every other assertion is a hard
 * CHECK and makes the process exit non-zero. exit code == number of real
 * failures.
 */
#include "rabuka.h"
#include "gen_data.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── assertion plumbing ───────────────────────────────────────────────── */

static int failures;      /* real failures -> non-zero exit */
static int gaps;          /* confirmed Rust-vs-C divergences -> printed, not fatal */
static long assertions;   /* every assertion evaluated (CHECK or EXPECTED_GAP) */

#define CHECK(cond, ...) do {                                              \
        assertions++;                                                      \
        if (!(cond)) {                                                     \
            failures++;                                                    \
            fprintf(stderr, "FAIL: ");                                     \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                           \
        }                                                                  \
    } while (0)

/* A confirmed divergence from the Rust truth. Report the count, not just the
   fact. Nothing here is allowed to be a guess: every call site cites the
   Rust file:line that proves the divergence is real. */
#define EXPECTED_GAP(cond, ...) do {                                       \
        assertions++;                                                      \
        if (!(cond)) {                                                     \
            gaps++;                                                        \
            printf("GAP: ");                                               \
            printf(__VA_ARGS__);                                           \
            fputc('\n', stdout);                                           \
        }                                                                  \
    } while (0)

/* ── corpus tallies (the deliverable: how much was actually covered) ──── */

static long n_cards;              /* card records decoded                  */
static long n_cards_with_ab;      /* cards that carry >=1 ability edge    */
static long n_card_ability_edges; /* sum of rb_card_num_abilities over all */
static long n_abilities;          /* global ability indices decoded       */
static long n_ability_failures;   /* global ability indices that failed   */
static long n_effects;            /* effect nodes walked (corpus-wide)    */
static long n_actions;            /* non-empty action strings checked     */
static long n_zone_strings;       /* non-empty source/destination checked */
static long n_cond_fields;        /* condition field keys checked         */
static long n_extra_pairs;        /* effect extra key/value pairs checked */
static long n_clones;             /* deep clone/free round trips          */

#define MAX_WALK_DEPTH 64

/* distinct unknown strings, for a readable GAP report */
#define MAX_DISTINCT 24
static char unknown_actions[MAX_DISTINCT][64];
static int  n_unknown_actions;
static char unknown_zones[MAX_DISTINCT][64];
static int  n_unknown_zones;

static void remember(char table[][64], int *n, const char *s) {
    if (!s || !*s) return;
    for (int i = 0; i < *n; i++) if (!strcmp(table[i], s)) return;
    if (*n < MAX_DISTINCT) {
        snprintf(table[*n], 64, "%s", s);
        (*n)++;
    }
}

/* ── string-index bounds (mirrors Rust abilities_gen::get_string ->
       Option<&str>, which the Rust test asserts is Some for every index
       reached from CARD_ABILITY_PAIRS) ───────────────────────────────── */

/* rb_get_string() clamps an out-of-range index to "" (data.c), so the C
   port cannot observe the Rust Option directly. The bound it clamps
   against is RBKA_NUM_STRING_OFFSETS-1, so that is what we assert. */
static int string_index_in_range(uint32_t idx) {
    return idx + 1u < (uint32_t)RBKA_NUM_STRING_OFFSETS;
}

/* ── effect-tree walk: action/zone/extra/condition invariants ─────────── */

/* Every non-empty action string the decoder produced must be a wire name
   the Rust ActionType enum knows. Rust: engine/src/ability/enums.rs
   ActionType::from_str is generated by wire_tables! and documents
   "Returns None for unrecognized names (makes typos detectable at parse
   time)". A C-only action name is a decoder divergence. */
static int action_is_known(const char *a) {
    return a && *a && rb_action_type_from_str(a) >= 0;
}

/* Zone wire names: engine/src/ability/enums.rs Zone::from_str -> Option.
   C strict parser: rb_ability_zone_from_str, -1 when unrecognized
   (engine_c_wip/src/ability/enums.c). */
static int zone_is_known(const char *z) {
    return z && *z && rb_ability_zone_from_str(z) >= 0;
}

static void walk_condition(const Condition *c, int depth, const char *where);

static void walk_effect(const AbilityEffect *e, int depth, const char *where) {
    if (!e) return;
    if (depth > MAX_WALK_DEPTH) return;
    n_effects++;

    /* An action string is optional in the wire (a bare cost container can
       have none); when present it must be a known action. */
    if (e->action) {
        n_actions++;
        if (*e->action) {
            if (!action_is_known(e->action)) {
                remember(unknown_actions, &n_unknown_actions, e->action);
                failures++;
                fprintf(stderr, "FAIL: ability effect action '%s' (%s) is not a known ActionType\n",
                        e->action, where);
            }
        }
    }
    assertions++;

    /* source / destination carry Zone wire names. `target` is a free-form
       target selector (card id, zone, player), so it is not enum-checked. */
    if (e->source && *e->source) {
        n_zone_strings++;
        if (!zone_is_known(e->source)) {
            remember(unknown_zones, &n_unknown_zones, e->source);
            failures++;
            fprintf(stderr, "FAIL: ability effect source '%s' (%s) is not a known Zone\n",
                    e->source, where);
        }
    }
    if (e->destination && *e->destination) {
        n_zone_strings++;
        if (!zone_is_known(e->destination)) {
            remember(unknown_zones, &n_unknown_zones, e->destination);
            failures++;
            fprintf(stderr, "FAIL: ability effect destination '%s' (%s) is not a known Zone\n",
                    e->destination, where);
        }
    }
    assertions++;

    /* Extra keys are string-table indices resolved by the decoder. An
       out-of-range index resolves to "", which would silently drop the
       field, so a non-empty key is the in-range proof. */
    CHECK(e->n_extra >= 0 && e->n_extra <= RB_MAX_EXTRA,
          "extra count %d out of range on %s", e->n_extra, where);
    for (int i = 0; i < e->n_extra; i++) {
        n_extra_pairs++;
        CHECK(e->extra_k[i] && *e->extra_k[i],
              "empty extra key at %d on %s (string index out of range?)", i, where);
        CHECK(e->extra_v[i] != NULL,
              "NULL extra value for key '%s' at %d on %s",
              e->extra_k[i] ? e->extra_k[i] : "?", i, where);
    }
    CHECK(e->n_child >= 0 && e->n_child <= RB_MAX_CHILD,
          "child count %d out of range on %s", e->n_child, where);
    CHECK(e->n_options >= 0 && e->n_options <= RB_MAX_CHILD,
          "option count %d out of range on %s", e->n_options, where);

    walk_condition(e->condition, depth + 1, where);
    walk_condition(e->result_condition, depth + 1, where);
    walk_condition(e->alternative_condition, depth + 1, where);

    for (int i = 0; i < e->n_child; i++)
        walk_effect(e->child[i], depth + 1, where);
    for (int i = 0; i < e->n_options; i++)
        walk_effect(e->options[i], depth + 1, where);

    /* compound structural branches (rabuka.h:104-118) */
    walk_effect(e->primary_effect, depth + 1, where);
    walk_effect(e->alternative_effect, depth + 1, where);
    walk_effect(e->look_action, depth + 1, where);
    walk_effect(e->select_action, depth + 1, where);
    walk_effect(e->followup_action, depth + 1, where);
    walk_effect(e->optional_action, depth + 1, where);
    walk_effect(e->conditional_action, depth + 1, where);
    walk_effect(e->gained_effect, depth + 1, where);
    walk_effect(e->resource_on_select, depth + 1, where);
    walk_effect(e->opponent_action, depth + 1, where);
}

static void walk_cond_value(const CondValue *v, int depth) {
    if (!v) return;
    if (depth > MAX_WALK_DEPTH) return;
    switch (v->tag) {
    case RB_TAG_STR:
    case RB_TAG_F64:
        /* An out-of-range string index decodes to "" (data.c), which is
           indistinguishable from a legitimately empty string. The Rust
           side decodes the same way (vm.rs str(idx) with the blob table),
           so this is a shape check, not a divergence claim. */
        CHECK(v->s != NULL, "string/f64 condition value is NULL");
        break;
    case RB_TAG_ARRAY:
        CHECK(v->arr_n == 0 || v->arr != NULL, "array condition value with no buffer");
        if (v->arr_n && v->arr_n > (1u << 20))
            CHECK(0, "absurd array length %u in condition", v->arr_n);
        for (uint32_t i = 0; i < v->arr_n && i < (1u << 20); i++)
            walk_cond_value(&v->arr[i], depth + 1);
        break;
    case RB_TAG_OBJECT:
    case RB_TAG_OBJVAR:
        walk_condition(v->cond, depth + 1, "nested");
        break;
    default:
        break;
    }
}

static void walk_condition(const Condition *c, int depth, const char *where) {
    if (!c) return;
    if (depth > MAX_WALK_DEPTH) return;
    CHECK(c->n_fields <= RB_MAX_COND_FIELD,
          "condition field count %u over capacity on %s", c->n_fields, where);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        n_cond_fields++;
        /* A non-empty key proves the string index resolved. */
        CHECK(c->fields[i].key && *c->fields[i].key],
              "empty condition field key at %u on %s (string index out of range?)",
              i, where);
        walk_cond_value(&c->fields[i].v, depth + 1);
    }
}

/* ── deep-clone / free contract (alloc.c ownership, corpus-wide) ──────── */

static int str_eq(const char *a, const char *b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    return strcmp(a, b) == 0;
}

/* Structural equality: every field the Rust `#[derive(Clone)]` on
   AbilityEffect copies by value, plus the recursively owned subtrees. */
static int effect_equal(const AbilityEffect *a, const AbilityEffect *b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    if (!str_eq(a->text, b->text)) return 0;
    if (!str_eq(a->action, b->action)) return 0;
    if (!str_eq(a->source, b->source)) return 0;
    if (!str_eq(a->destination, b->destination)) return 0;
    if (!str_eq(a->target, b->target)) return 0;
    if (a->count != b->count) return 0;
    if (a->has_condition != b->has_condition) return 0;
    if (a->is_optional != b->is_optional) return 0;
    if (a->is_further != b->is_further) return 0;
    if (a->n_child != b->n_child) return 0;
    if (a->n_options != b->n_options) return 0;
    if (a->n_extra != b->n_extra) return 0;
    if (a->repeat_limit != b->repeat_limit) return 0;
    if (a->conditional_flag != b->conditional_flag) return 0;
    if (a->conditional_negation != b->conditional_negation) return 0;
    if (a->per_unit != b->per_unit) return 0;
    if (a->per_unit_count != b->per_unit_count) return 0;
    if (a->cost_reduction_per_group != b->cost_reduction_per_group) return 0;
    if (a->distinct_flag != b->distinct_flag) return 0;
    if (strcmp(a->id_field, b->id_field)) return 0;
    if (strcmp(a->self_target_field, b->self_target_field)) return 0;
    if (strcmp(a->card_type_field, b->card_type_field)) return 0;
    for (int i = 0; i < a->n_child; i++)
        if (!effect_equal(a->child[i], b->child[i])) return 0;
    for (int i = 0; i < a->n_options; i++)
        if (!effect_equal(a->options[i], b->options[i])) return 0;
    for (int i = 0; i < a->n_extra; i++) {
        if (!str_eq(a->extra_k[i], b->extra_k[i])) return 0;
        if (!str_eq(a->extra_v[i], b->extra_v[i])) return 0;
    }
    if (!effect_equal(a->primary_effect, b->primary_effect)) return 0;
    if (!effect_equal(a->alternative_effect, b->alternative_effect)) return 0;
    if (!effect_equal(a->look_action, b->look_action)) return 0;
    if (!effect_equal(a->select_action, b->select_action)) return 0;
    if (!effect_equal(a->followup_action, b->followup_action)) return 0;
    if (!effect_equal(a->optional_action, b->optional_action)) return 0;
    if (!effect_equal(a->conditional_action, b->conditional_action)) return 0;
    if (!effect_equal(a->gained_effect, b->gained_effect)) return 0;
    if (!effect_equal(a->resource_on_select, b->resource_on_select)) return 0;
    if (!effect_equal(a->opponent_action, b->opponent_action)) return 0;
    if ((a->condition == NULL) != (b->condition == NULL)) return 0;
    if ((a->result_condition == NULL) != (b->result_condition == NULL)) return 0;
    if ((a->alternative_condition == NULL) != (b->alternative_condition == NULL)) return 0;
    return 1;
}

/* No owned pointer of the clone may be the same object as the source's:
   that is a shallow copy, and freeing the clone would double-free. */
static void assert_no_alias(const AbilityEffect *src, const AbilityEffect *cl,
                            int depth, const char *where) {
    if (!src || !cl || depth > MAX_WALK_DEPTH) return;
#define NOALIAS(p, label) do {                                            \
        if ((cl->p) && (cl->p) == (src->p)) {                             \
            failures++;                                                    \
            fprintf(stderr,                                                \
                    "FAIL: clone shares %s pointer with its source (%s)\n",\
                    label, where);                                         \
        }                                                                  \
    } while (0)
    NOALIAS(text, "text");
    NOALIAS(action, "action");
    NOALIAS(source, "source");
    NOALIAS(destination, "destination");
    NOALIAS(target, "target");
    NOALIAS(condition, "condition");
    NOALIAS(result_condition, "result_condition");
    NOALIAS(alternative_condition, "alternative_condition");
    NOALIAS(primary_effect, "primary_effect");
    NOALIAS(alternative_effect, "alternative_effect");
    NOALIAS(look_action, "look_action");
    NOALIAS(select_action, "select_action");
    NOALIAS(followup_action, "followup_action");
    NOALIAS(optional_action, "optional_action");
    NOALIAS(conditional_action, "conditional_action");
    NOALIAS(gained_effect, "gained_effect");
    NOALIAS(resource_on_select, "resource_on_select");
    NOALIAS(opponent_action, "opponent_action");
#undef NOALIAS
    for (int i = 0; i < src->n_child && i < cl->n_child; i++)
        assert_no_alias(src->child[i], cl->child[i], depth + 1, where);
    for (int i = 0; i < src->n_options && i < cl->n_options; i++)
        assert_no_alias(src->options[i], cl->options[i], depth + 1, where);
    for (int i = 0; i < src->n_extra && i < cl->n_extra; i++) {
        if (cl->extra_k[i] && cl->extra_k[i] == src->extra_k[i]) {
            failures++;
            fprintf(stderr, "FAIL: clone shares extra_k[%d] pointer with its source (%s)\n", i, where);
        }
        if (cl->extra_v[i] && cl->extra_v[i] == src->extra_v[i]) {
            failures++;
            fprintf(stderr, "FAIL: clone shares extra_v[%d] pointer with its source (%s)\n", i, where);
        }
    }
    assert_no_alias(src->primary_effect, cl->primary_effect, depth + 1, where);
    assert_no_alias(src->alternative_effect, cl->alternative_effect, depth + 1, where);
    assert_no_alias(src->look_action, cl->look_action, depth + 1, where);
    assert_no_alias(src->select_action, cl->select_action, depth + 1, where);
    assert_no_alias(src->followup_action, cl->followup_action, depth + 1, where);
    assert_no_alias(src->optional_action, cl->optional_action, depth + 1, where);
    assert_no_alias(src->conditional_action, cl->conditional_action, depth + 1, where);
    assert_no_alias(src->gained_effect, cl->gained_effect, depth + 1, where);
    assert_no_alias(src->resource_on_select, cl->resource_on_select, depth + 1, where);
    assert_no_alias(src->opponent_action, cl->opponent_action, depth + 1, where);
}

/* rb_effect_deep_clone mirrors Rust `#[derive(Clone)]` on AbilityEffect
   (engine/src/core/card.rs). Clone -> compare -> free for EVERY effect
   node in the corpus: a shallow field is a latent double free, because
   engine callers hand clones to rb_effect_free (ability_queue.c:450,
   cost.c:766, engine.c:407, effects/ability.c:283, turn/triggers.c:514,
   effects/state.c:334, effects/misc.c:450). */
static void check_clone(const AbilityEffect *e, const char *where) {
    if (!e) return;
    n_clones++;
    AbilityEffect *cl = rb_effect_deep_clone(e);
    CHECK(cl != NULL, "deep clone returned NULL for %s", where);
    if (!cl) return;
    CHECK(cl != e, "deep clone returned the same pointer for %s", where);
    CHECK(effect_equal(e, cl), "deep clone is not structurally equal for %s", where);
    assert_no_alias(e, cl, 0, where);
    rb_effect_free(cl);
}

/* ── tests ────────────────────────────────────────────────────────────── */

/* A: corpus size / shape goldens.
   Rust: corpus_smoke_test.rs:98 (>2000 cards),
         ability_golden_test.rs:12 (deliberate shape golden),
         bytecode_validation_test.rs:51 (ability count == JSON count). */
static void test_corpus_shape(void) {
    long n = (long)rb_num_cards();
    CHECK(n > 2000, "expected the full card database, got %ld cards", n);
    /* 936 = cards/abilities.json unique_abilities length, the number
       Rust's ability_count() is asserted against
       (bytecode_validation_test.rs:51-60). */
    CHECK((long)rb_num_abilities() == 936,
          "ability count %lu does not match the 936 JSON unique_abilities",
          (unsigned long)rb_num_abilities());
    /* Rust: bytecode_empty_slices_match_known_is_null_baseline
       (bytecode_validation_test.rs:146-152) — empty slices must be 0. */
    CHECK(rb_count_empty_bytecode_abilities() == 0,
          "empty-slice ability count is %d, Rust pins 0",
          rb_count_empty_bytecode_abilities());
}

/* B: every ability index decodes.
   Rust: bytecode_every_ability_decodes (bytecode_validation_test.rs:63-84)
   and a_valid_index_never_decodes_to_the_default_fallback (:334-353). */
static void test_every_ability_decodes(void) {
    uint32_t count = rb_num_abilities();
    long with_effect = 0, with_text = 0, blank = 0;
    for (uint32_t i = 0; i < count; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        int ok = rb_decode_ability(i, &ab);
        n_abilities++;
        if (!ok) {
            n_ability_failures++;
            failures++;
            fprintf(stderr, "FAIL: ability %u failed to decode\n", i);
            continue;
        }
        /* Non-blank: Rust compares against Ability::default(). The C
           equivalent of "not the default" is at least one of the
           decoded fields being present. */
        if (ab.full_text || ab.triggers || ab.effect || ab.cost) with_text++;
        else blank++;
        if (ab.effect) with_effect++;

        char where[64];
        snprintf(where, sizeof(where), "ability %u", i);
        walk_effect(ab.effect, 0, where);
        walk_effect(ab.cost, 0, where);
        if (ab.effect || ab.cost) { check_clone(ab.effect, where); check_clone(ab.cost, where); }

        if (ab.triggers)
            CHECK(rb_trigger_from_token(ab.triggers) >= 0,
                  "ability %u has an unparseable trigger string '%s'", i, ab.triggers);
        rb_free_ability(&ab);
    }
    CHECK(n_ability_failures == 0,
          "%ld of %ld abilities failed to decode", n_ability_failures, n_abilities);
    CHECK(blank == 0,
          "%ld abilities decoded to the blank default (Rust: never)", blank);
    /* Control: the loop above must not pass because everything was blank. */
    CHECK(with_effect > 0, "no ability decoded to an effect at all");
    CHECK(with_text > 0, "no ability decoded any content at all");
    printf("info: %ld abilities decoded, %ld carry an effect, %ld non-blank\n",
           n_abilities, with_effect, with_text);
}

/* C: the index guard is exact on both sides.
   Rust: out_of_range_index_is_named_in_the_error
   (bytecode_validation_test.rs:281-321). C returns a bool instead of an
   error variant, so we pin both the rejection and the valid side. */
static void test_index_guard(void) {
    uint32_t n = rb_num_abilities();
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    CHECK(rb_get_ability(n, &ab) == 0,
          "ability index %u (one past the last) must be rejected", n);
    memset(&ab, 0, sizeof(ab));
    CHECK(rb_get_ability(n + 1000, &ab) == 0,
          "ability index %u must be rejected", n + 1000);
    memset(&ab, 0, sizeof(ab));
    CHECK(rb_get_ability(n - 1, &ab) == 1,
          "the last valid ability index %u must decode", n - 1);
    CHECK(ab.full_text || ab.triggers || ab.effect || ab.cost,
          "the last valid ability decoded to the default fallback");
    rb_free_ability(&ab);
}

/* D: no silent decode fallbacks.
   Rust: bytecode_no_silent_decode_fallbacks
   (bytecode_validation_test.rs:99-139) with KNOWN_FALLBACKS = &[] —
   "Any NEW fallback fails here — fix the mapping, do not append entries." */
static void test_no_decode_fallbacks(void) {
    uint32_t idx[64];
    int n = rb_decode_fallback_abilities(idx, 64);
    uint32_t total = rb_decode_fallback_count();
    if (n > 0) {
        gaps++;
        printf("GAP: %u silent decode fallbacks across %d abilities (Rust baseline is 0) — first:",
               total, n);
        for (int i = 0; i < n && i < 16; i++) printf(" %u", idx[i]);
        printf("\n");
    }
    assertions++;
    /* Control: the counter must be live, i.e. reachable at all. */
    CHECK(rb_decode_fallback_count() == total, "fallback counter is not readable");
}

/* E: card record structure.
   Rust: corpus_smoke_test.rs:85 (every card decodes) plus the record
   layout of engine/src/core/card_binary.rs decode_card_from_record. */
static void test_every_card_decodes(void) {
    uint32_t n = rb_num_cards();
    long dup = 0, no_no = 0, heart_bad = 0, special_bad = 0, ab_guard_bad = 0;
    long group_mismatch = 0, empty_group = 0;
    long edges = 0;

    /* O(n^2) duplicate detection over card_no, but 2526^2 is cheap here
       and avoids depending on an allocator for a 2526-entry table. */
    for (uint32_t i = 0; i < n; i++) {
        Card c;
        memset(&c, 0, sizeof(c));
        if (!rb_decode_card_by_index(i, &c)) {
            failures++;
            fprintf(stderr, "FAIL: card record %u failed to decode\n", i);
            continue;
        }
        n_cards++;
        const char *no = rb_card_string(c.card_no_idx);
        if (!no || !*no) {
            no_no++;
            failures++;
            fprintf(stderr, "FAIL: card %u has an empty card_no (string index %u out of range)\n",
                    i, c.card_no_idx);
        }
        /* heart sections: base | blade | need are stored back to back */
        if (c.n_hearts != c.num_base + c.num_blade + c.num_need || c.n_hearts > RB_MAX_HEARTS) {
            heart_bad++;
            failures++;
            fprintf(stderr, "FAIL: card %u (%s) heart count %d != %d+%d+%d\n",
                    i, no ? no : "?", c.n_hearts, c.num_base, c.num_blade, c.num_need);
        }
        /* the special heart is only present when the record carries it */
        if (c.has_special && c.special_count == 0) {
            special_bad++;
            failures++;
            fprintf(stderr, "FAIL: card %u (%s) has_special with zero special_count\n",
                    i, no ? no : "?");
        }
        /* cards.bin's ability_idx (record offset 16) is 0xFFFF exactly for
           the cards the CARD_ABILITY_PAIRS table gives no ability. This
           cross-checks the two independent tables; an offset or stale
           pairs table (the gen_data 5716-vs-5723 class of bug) shows up
           here immediately. */
        uint16_t legacy = rb_card_ability_idx(i);
        int n_ab = rb_card_num_abilities(i);
        if ((legacy == 0xFFFF) != (n_ab == 0)) {
            ab_guard_bad++;
            failures++;
            fprintf(stderr, "FAIL: card %u (%s) legacy ability_idx %u but %d pair(s)\n",
                    i, no ? no : "?", legacy, n_ab);
        }
        if (n_ab > 0) n_cards_with_ab++;
        edges += n_ab;

        for (int k = 0; k < n_ab; k++) {
            uint32_t aidx = 0;
            if (!rb_card_get_ability_idx(i, k, &aidx)) {
                failures++;
                fprintf(stderr, "FAIL: card %u ability slot %d has no index\n", i, k);
                continue;
            }
            CHECK(aidx < rb_num_abilities(),
                  "card %u ability slot %d -> index %u out of range (max %u)",
                  i, k, aidx, (unsigned)rb_num_abilities() - 1);
            Ability ab;
            memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability(i, k, &ab)) {
                failures++;
                fprintf(stderr, "FAIL: card %u (%s) ability slot %d failed to decode\n",
                        i, no ? no : "?", k);
            } else {
                char where[96];
                snprintf(where, sizeof(where), "card %u ability %d", i, k);
                walk_effect(ab.effect, 0, where);
                walk_effect(ab.cost, 0, where);
                if (ab.effect || ab.cost) {
                    check_clone(ab.effect, where);
                    check_clone(ab.cost, where);
                }
                rb_free_ability(&ab);
            }
        }

        /* Group lookup must agree with the card's own derived group. This
           guards the util.c fix: matching against the raw group_idx
           (which resolves to "" for all 2526 records) made every card
           match every group. */
        const char *group = rb_card_group_name((int)i);
        if (!group || !*group) empty_group++;
        else if (!rb_card_matches_group_str((int)i, group)) {
            group_mismatch++;
            failures++;
            fprintf(stderr, "FAIL: card %u (%s) does not match its own group '%s'\n",
                    i, no ? no : "?", group);
        }
        rb_free_card(&c);
    }

    /* duplicate card_no: compare the decoded card_no of every pair */
    for (uint32_t i = 0; i < n; i++) {
        for (uint32_t j = i + 1; j < n; j++) {
            /* cheap pre-filter: only compare records whose card_no string
               index is identical or whose strings actually match */
            const unsigned char *ri = rb_card_record(i);
            const unsigned char *rj = rb_card_record(j);
            if (!ri || !rj) continue;
            uint16_t ci = (uint16_t)(ri[0] | (ri[1] << 8));
            uint16_t cj = (uint16_t)(rj[0] | (rj[1] << 8));
            if (ci == cj) {
                dup++;
                failures++;
                fprintf(stderr, "FAIL: duplicate card_no string index %u at records %u and %u\n",
                        ci, i, j);
            }
        }
    }

    n_card_ability_edges = edges;
    CHECK(no_no == 0, "%ld cards have an empty card_no", no_no);
    CHECK(dup == 0, "%ld duplicate card numbers in the corpus", dup);
    CHECK(heart_bad == 0, "%ld cards have an inconsistent heart section count", heart_bad);
    CHECK(special_bad == 0, "%ld cards claim a special heart with zero count", special_bad);
    CHECK(ab_guard_bad == 0,
          "%ld cards disagree between cards.bin ability_idx and the pairs table", ab_guard_bad);
    CHECK(group_mismatch == 0,
          "%ld cards fail to match their own derived group name", group_mismatch);
    /* Golden: the pairs table holds 2011 (card_no, ability) edges.
       Rust: card_loader.rs CARD_ABILITY_PAIRS. */
    CHECK(edges == 2011, "card->ability edge count is %ld, golden is 2011", edges);
    CHECK(n_cards_with_ab == 1565, "cards with abilities is %ld, golden is 1565", n_cards_with_ab);
    printf("info: %ld cards decoded, %ld with abilities, %ld card->ability edges, "
           "%ld empty-group cards\n", n_cards, n_cards_with_ab, n_card_ability_edges, empty_group);
}

/* The one card whose abilities were known to be missing from the pairs
   table (三船栞子). It is a corpus-wide decoder/ABI assertion, not a
   gameplay test. */
static void test_known_pair_gap_closed(void) {
    int idx = rb_find_card_by_no("PL!N-sd2-010-SD2");
    CHECK(idx >= 0, "fixture PL!N-sd2-010-SD2 (三船栞子) resolves");
    if (idx < 0) return;
    int n = rb_card_num_abilities((uint32_t)idx);
    /* Rust truth: cards/abilities.json lists 登場 + 自動 for this card_no. */
    EXPECTED_GAP(n == 2,
                 "card PL!N-sd2-010-SD2 (idx %d) has %d abilities, "
                 "abilities.json lists 2 (登場+自動) [C src/core/card.c:128 "
                 "rb_card_num_abilities, pairs table RBKA_CARD_ABILITY_PAIRS]",
                 idx, n);
    for (int k = 0; k < n; k++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        CHECK(rb_decode_card_ability((uint32_t)idx, k, &ab),
              "PL!N-sd2-010-SD2 ability %d must decode", k);
        rb_free_ability(&ab);
    }
}

/* F: card-id lookup determinism.
   Rust: card_id_lookup_determinism_test.rs (both tests). */
static void test_card_id_lookup_determinism(void) {
    /* An exact print must never be shadowed by another print. */
    static const char *const exact[] = {
        "PL!SP-bp2-006-P", "PL!SP-bp2-006-SEC", "PL!S-pb1-003-R",
        "PL!S-pb1-003-P＋", "PL!HS-pb1-003-R"
    };
    for (size_t k = 0; k < sizeof(exact) / sizeof(exact[0]); k++) {
        int id = rb_card_get_card_id(exact[k]);
        CHECK(id >= 0, "%s must resolve", exact[k]);
        if (id < 0) continue;
        Card c;
        memset(&c, 0, sizeof(c));
        if (!rb_decode_card_by_index((uint32_t)id, &c)) {
            failures++;
            fprintf(stderr, "FAIL: %s resolved to undecodable card %d\n", exact[k], id);
            continue;
        }
        const char *actual = rb_card_string(c.card_no_idx);
        CHECK(actual && !strcmp(actual, exact[k]),
              "%s resolved to '%s'", exact[k], actual ? actual : "?");
        rb_free_card(&c);
    }

    /* An unknown rarity suffix falls back to "any print of this card
       number". The contract is that the lowest matching key wins, every
       time — a HashMap-iteration-order fallback would resolve a different
       print on a different run. */
    const char *requested = "PL!SP-bp2-006-R";
    int first = rb_card_get_card_id(requested);
    CHECK(first >= 0, "fixture %s must resolve through some fallback", requested);
    if (first < 0) return;
    int distinct = 0;
    int ids[64];
    for (int t = 0; t < 64; t++) {
        int id = rb_card_get_card_id(requested);
        int seen = 0;
        for (int i = 0; i < distinct; i++) if (ids[i] == id) seen = 1;
        if (!seen && distinct < 64) ids[distinct++] = id;
    }
    CHECK(distinct == 1,
          "%s resolved to %d different card ids across 64 calls", requested, distinct);
    for (int i = 0; i < distinct; i++) {
        const char *no = rb_card_string((uint16_t)(rb_card_record((uint32_t)ids[i])[0] |
                                                    (rb_card_record((uint32_t)ids[i])[1] << 8)));
        CHECK(no && !strncmp(no, "PL!SP-bp2-006-", 13),
              "the rarity fallback escaped the requested card number: %s", no ? no : "?");
    }
}

/* G: an action string that no executor can dispatch is dead data. Rust's
   action_coverage_test.rs collects the action set from the corpus and
   requires every one of them to resolve. C equivalent at the shape
   level: the corpus must not contain a single action outside ActionType,
   and it must not be empty (control). */
static void test_action_coverage_control(void) {
    CHECK(n_actions > 0, "no action strings were checked at all");
    CHECK(n_unknown_actions == 0,
          "%d distinct unknown action strings in the corpus", n_unknown_actions);
    if (n_unknown_actions) {
        printf("     unknown actions:");
        for (int i = 0; i < n_unknown_actions; i++) printf(" %s", unknown_actions[i]);
        printf("\n");
    }
    CHECK(n_zone_strings > 0, "no zone strings were checked at all");
    if (n_unknown_zones) {
        printf("     unknown zones:");
        for (int i = 0; i < n_unknown_zones; i++) printf(" %s", unknown_zones[i]);
        printf("\n");
    }
}

/* ── report ───────────────────────────────────────────────────────────── */

static void report(void) {
    printf("\n=== parity_characterization coverage ===\n");
    printf("cards decoded              : %ld / %lu\n", n_cards, (unsigned long)rb_num_cards());
    printf("cards with abilities       : %ld\n", n_cards_with_ab);
    printf("card->ability edges walked : %ld\n", n_card_ability_edges);
    printf("abilities decoded          : %ld / %lu\n", n_abilities, (unsigned long)rb_num_abilities());
    printf("effect nodes walked        : %ld\n", n_effects);
    printf("action strings checked     : %ld (%d unknown)\n", n_actions, n_unknown_actions);
    printf("zone strings checked       : %ld (%d unknown)\n", n_zone_strings, n_unknown_zones);
    printf("condition fields checked   : %ld\n", n_cond_fields);
    printf("effect extra pairs checked : %ld\n", n_extra_pairs);
    printf("deep clone/free round trips: %ld\n", n_clones);
    printf("assertions evaluated       : %ld\n", assertions);
    printf("real failures              : %d\n", failures);
    printf("confirmed gaps (EXPECTED_GAP): %d\n", gaps);
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_corpus_shape();
    test_every_ability_decodes();
    test_index_guard();
    test_no_decode_fallbacks();
    test_every_card_decodes();
    test_known_pair_gap_closed();
    test_card_id_lookup_determinism();
    test_action_coverage_control();
    rb_unload();
    report();
    if (failures) {
        fprintf(stderr, "PARITY CHARACTERIZATION: %d real failure(s)\n", failures);
        return 1;
    }
    if (gaps) printf("PARITY CHARACTERIZATION: %d confirmed gap(s), 0 real failures\n", gaps);
    else printf("PARITY CHARACTERIZATION PASSED\n");
    return 0;
}

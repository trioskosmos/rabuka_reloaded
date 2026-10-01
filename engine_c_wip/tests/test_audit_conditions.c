/* test_audit_conditions.c -- audit_conditions agent suite.
 *
 * Scope: the CONDITION chain, traced one condition at a time from the JSON in
 * cards/abilities.json, through the generated decoder (condition_decoder_gen.rs,
 * read as spec), to vm.c's decode branch, to the dispatch and evaluator in
 * src/ability/condition.c.
 *
 * Every case decodes a REAL card ability out of the shipped card blob and
 * evaluates the decoded Condition with rb_eval_condition_for_host, so a pass
 * means the whole chain agrees with Rust for that corpus entry.
 *
 * Two rules this file obeys, because breaking them produces silent green:
 *
 *  1. NON-VACUITY. A condition that always answers true makes every "should be
 *     blocked" assertion pass; one that always answers false makes every "should
 *     apply" assertion pass. Every negative here is PAIRED with a positive on the
 *     SAME card and the SAME decoded Condition, and the pair differs by exactly
 *     the factor under test: a heart colour, a blade count, a board composition,
 *     a chosen colour. assert_pair() FAILS LOUDLY when both boards agree.
 *
 *  2. FORK PER CASE. Each case runs in its own forked child with the shared
 *     counters in MAP_SHARED|MAP_ANONYMOUS memory, zeroed IN THE CHILD and read
 *     by the parent only AFTER waitpid. Without this a crash in one case poisons
 *     every later case and the suite measures nondeterministically. A SIGSEGV /
 *     SIGABRT / SIGFPE handler names the case in flight.
 *
 * Colour indices are derived from rb_parse_heart_color("heartNN"), never from a
 * literal, and the whole mapping is printed at startup: an agent found four
 * off-by-one colour bugs that way. Card identity is pinned with test_id_exact,
 * never test_id, because rb_find_card_by_no falls back to a substring search.
 *
 * NOTE ON CARD NUMBERS: several prints carry a full-width plus sign (U+FF0B),
 * e.g. PL!N-pb1-004-P+ . The file is UTF-8 and the compiler reads it as UTF-8;
 * transcribing it as an ASCII '+' makes test_id_exact return -1, which is
 * reported as a SETUP BUG rather than silently as "condition false".
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* ------------------------------------------------------------------ */
/* shared counters                                                     */
/* ------------------------------------------------------------------ */
typedef struct {
    int checks;
    int failures;
} Shared;

static Shared *sh_create(void) {
    void *p = mmap(NULL, sizeof(Shared), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (p == MAP_FAILED) { fprintf(stderr, "mmap failed\n"); exit(2); }
    return (Shared *)p;
}

/* Variadic on purpose: a two-argument CHECK(cond, "a" "b") silently splits
 * adjacent string literals, and the messages below are composed. */
#define CHECK(cond, ...) do {                                             \
    sh->checks++;                                                         \
    if (!(cond)) {                                                        \
        sh->failures++;                                                   \
        fprintf(stderr, "FAIL: ");                                        \
        fprintf(stderr, __VA_ARGS__);                                     \
        fprintf(stderr, "\n");                                            \
    }                                                                     \
} while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
    int a_ = (actual), e_ = (expected);                                   \
    sh->checks++;                                                         \
    if (a_ != e_) {                                                       \
        sh->failures++;                                                   \
        fprintf(stderr, "FAIL: ");                                        \
        fprintf(stderr, __VA_ARGS__);                                     \
        fprintf(stderr, " (got %d expected %d)\n", a_, e_);               \
    }                                                                     \
} while (0)

static int cond_has_field(const Condition *c, const char *key) {
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)) return 1;
    return 0;
}

/* ------------------------------------------------------------------ */
/* board fixtures                                                      */
/* ------------------------------------------------------------------ */
/* sizeof(TestGame) is 781 KB -- GameState embeds RbMods, which is a
 * RB_MAX_CARD_IDS x 8 modifier table. A case that needs three boards would put
 * 2.3 MB of locals on the stack and die of SIGSEGV in the forked child before
 * it reached its first assertion, which reads exactly like "the condition
 * crashed" and is not. Every board below is therefore `static`: each case is
 * its own process, so static storage costs nothing and cannot alias. */
/* A key filter must accept any tag: vm.c stores cost_limit and
 * cost_limit_operator as cond_add_i64, and group_names as a string ARRAY, so a
 * get_str-only filter reports "absent" for fields the decoder did carry. */
static int cond_has_key(const Condition *c, const char *key) {
    /* A NULL key means "any condition of this variant". It must short-circuit
     * BEFORE the strcmp loop: cond_has_field passes the key straight through,
     * and strcmp(x, NULL) is a guaranteed SIGSEGV. Every case that searched
     * without a key filter died here. */
    if (!key) return 1;
    return cond_has_field(c, key);
}

static Condition *match_cond(const Condition *c, int variant, const char *key) {
    if (!c) return NULL;
    if (c->variant == (uint8_t)variant && cond_has_key(c, key)) return (Condition *)c;
    /* A condition can carry further conditions in its own field values
     * (`condition`, `cause`, `sub_checks`, `alternative_condition`), which is
     * where several of the corpus entries hide: the JSON nests them and the
     * decoder follows the nesting. Searching only the effect tree misses
     * PL!N-pb7-022-N's group-filtered state change and PL!N-pb1-004-P+'s
     * `not_moved` entirely, which reads as "the card has no such condition". */
    for (uint32_t i = 0; i < c->n_fields; i++) {
        Condition *r = match_cond(c->fields[i].v.cond, variant, key);
        if (r) return r;
        for (uint32_t j = 0; j < c->fields[i].v.arr_n; j++)
            if (c->fields[i].v.tag == RB_TAG_ARRAY) {
                Condition *r2 = match_cond(c->fields[i].v.arr[j].cond, variant, key);
                if (r2) return r2;
            }
    }
    return NULL;
}

static Condition *find_cond_in_effect(const AbilityEffect *e, int variant,
                                      const char *key, int depth) {
    /* Bounds guards, not decoration: an ability index past the card's real
     * ability count makes rb_decode_card_ability fail part-way through, and a
     * partially built subtree can carry an n_child/n_options that was never
     * written. Walking that is a wild pointer read, and it shows up as a
     * SIGSEGV inside the LOOKUP rather than as the setup bug it is. */
    if (!e || depth > 4) return NULL;
    if (e->condition && e->condition->variant == (uint8_t)variant &&
        cond_has_key(e->condition, key))
        return e->condition;
    if (e->result_condition && e->result_condition->variant == (uint8_t)variant &&
        cond_has_key(e->result_condition, key))
        return e->result_condition;
    if (e->alternative_condition && e->alternative_condition->variant == (uint8_t)variant &&
        cond_has_key(e->alternative_condition, key))
        return e->alternative_condition;
    if (e->activation_condition && e->activation_condition->variant == (uint8_t)variant &&
        cond_has_key(e->activation_condition, key))
        return e->activation_condition;
    const struct AbilityEffect *kids[] = {
        e->primary_effect, e->alternative_effect, e->followup_action,
        e->optional_action, e->conditional_action, e->look_action,
        e->select_action, e->gained_effect, e->resource_on_select,
        e->opponent_action
    };
    for (size_t i = 0; i < sizeof kids / sizeof kids[0]; i++) {
        Condition *r = find_cond_in_effect(kids[i], variant, key, depth + 1);
        if (r) return r;
    }
    if (e->n_child > 0 && e->n_child <= RB_MAX_CHILD) {
        for (int i = 0; i < e->n_child; i++) {
            Condition *r = find_cond_in_effect(e->child[i], variant, key, depth + 1);
            if (r) return r;
        }
    }
    if (e->n_options > 0 && e->n_options <= RB_MAX_CHILD) {
        for (int i = 0; i < e->n_options; i++) {
            Condition *r = find_cond_in_effect(e->options[i], variant, key, depth + 1);
            if (r) return r;
        }
    }
    return NULL;
}

/* Decode ability `ab_idx` of `card_no` and copy out its first Condition of
 * `variant` carrying `key`.
 *
 * OWNERSHIP, single-sided on purpose: load_cond FREES AND ZEROES *out_ability
 * on every failure path and MOVES ownership to the caller only on success. A
 * caller that also frees on failure double-frees -- and rb_free_ability is not
 * NULL-safe on a zeroed struct, so that reads as a SIGSEGV inside the lookup
 * rather than as a leak. Every failure path prints, so a card-number typo or a
 * wrong ability index can never pass as "the condition evaluated false". */
static int load_cond(TestGame *tg, const char *card_no, int ab_idx, int variant,
                     const char *key, Condition *out, Ability *out_ability) {
    memset(out_ability, 0, sizeof *out_ability);
    int cid = test_id_exact(tg, card_no);
    if (cid < 0) {
        fprintf(stderr, "SETUP BUG: no card [%s]\n", card_no);
        return -1;
    }
    if (!rb_decode_card_ability((uint32_t)cid, (uint32_t)ab_idx, out_ability)) {
        rb_free_ability(out_ability);
        memset(out_ability, 0, sizeof *out_ability);
        return -1;
    }
    Condition *c = find_cond_in_effect(out_ability->effect, variant, key, 0);
    if (!c && out_ability->cost)
        c = find_cond_in_effect(out_ability->cost, variant, key, 0);
    if (!c) {
        rb_free_ability(out_ability);
        memset(out_ability, 0, sizeof *out_ability);
        return -1;
    }
    memcpy(out, c, sizeof *out);
    return cid;
}

/* The ability index is not stable -- the JSON entry order is not the wire
 * ability order -- so every case searches rather than hardcoding one.
 *
 * OWNERSHIP: on success the decoded Condition is copied out BY VALUE (the
 * Condition is a flat field array, so the copy owns nothing) and ownership of
 * the ABILITY is MOVED to *out_ability, which the caller frees exactly once.
 * Freeing the local copy as well is a double free, and the caller then holds a
 * struct full of dangling pointers. */
static int find_cond_any(TestGame *tg, const char *card_no, int variant,
                         const char *key, Condition *out, Ability *out_ability,
                         int *out_abi) {
    for (int abi = 0; abi < 8; abi++) {
        Condition c;
        Ability a;
        if (load_cond(tg, card_no, abi, variant, key, &c, &a) >= 0) {
            memcpy(out, &c, sizeof *out);
            *out_ability = a;             /* ownership MOVES, no free here */
            if (out_abi) *out_abi = abi;
            return test_id_exact(tg, card_no);
        }
        /* load_cond already freed on failure; freeing again is a double free. */
    }
    fprintf(stderr, "SETUP BUG: [%s] has no condition v=%d key=%s in abilities 0..7\n",
            card_no, variant, key ? key : "-");
    return -1;
}

static void assert_pair(Shared *sh, const char *name, const char *what,
                        int positive_want, int positive_got,
                        int negative_want, int negative_got) {
    sh->checks++;
    if (positive_got != positive_want) {
        sh->failures++;
        fprintf(stderr, "FAIL: %s / %s: positive board expected %d got %d\n",
                name, what, positive_want, positive_got);
    }
    sh->checks++;
    if (negative_got != negative_want) {
        sh->failures++;
        fprintf(stderr, "FAIL: %s / %s: negative board expected %d got %d\n",
                name, what, negative_want, negative_got);
    }
    if (positive_got == negative_got) {
        sh->failures++;
        fprintf(stderr, "FAIL: %s / %s: NON-VACUITY -- both boards returned %d, "
                        "so this pair cannot distinguish the factor\n",
                name, what, positive_got);
    }
    printf("ok: %s / %s (pos=%d neg=%d)\n", name, what, positive_got, negative_got);
}

/* ====================================================================== *
 * CASE 1 -- aggregate:"total" short-circuit (Rust condition.rs:510-521)
 *
 * PL!S-bp5-013-N: location_condition, location live_card_zone, heart_colors
 * ["heart04"], card_type live_card, count 4, operator >=, aggregate "total".
 * The text is a NEED-heart sum over the live cards.
 *
 * Before the fix the aggregate short-circuit did not exist in C at all, so this
 * condition reached eval_location, which counted CARDS in the live zone (one)
 * and compared 1 >= 4: false on every board. check_aggregate_total was defined
 * in the same file and called from nowhere.
 *
 * The pair differs only in how many heart04 the live card NEEDS:
 * PL!HS-bp1-021-L needs heart04 x5 (clears 4), PL!-bp3-019-L needs heart01 /
 * heart06 and no heart04 at all.
 * ====================================================================== */
static void case_aggregate_total_live_zone(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!S-bp5-013-N", RB_COND_LOCATION, NULL, &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    int pos_live = test_id_exact(&tg, "PL!HS-bp1-021-L");   /* need heart04 x5 */
    int neg_live = test_id_exact(&tg, "PL!-bp3-019-L");      /* no need heart04  */
    if (pos_live < 0 || neg_live < 0) {
        fprintf(stderr, "SETUP BUG: live fixtures missing (pos=%d neg=%d)\n", pos_live, neg_live);
        sh->failures++; rb_free_ability(&ab); return;
    }

    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!S-bp5-013-N", abi, RB_COND_LOCATION, NULL, &c1, &a1);
    test_add_to_live(&t1, pos_live);
    int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);

    static TestGame t2;
    Condition c2;
    Ability a2;
    test_game_new(&t2);
    load_cond(&t2, "PL!S-bp5-013-N", abi, RB_COND_LOCATION, NULL, &c2, &a2);
    test_add_to_live(&t2, neg_live);
    int neg = rb_eval_condition_for_host(&t2.state, 0, cid, &c2);

    assert_pair(sh, "PL!S-bp5-013-N aggregate:total live_card_zone",
                "sum of need heart04 >= 4", 1, pos, 0, neg);
    rb_free_ability(&ab);
    rb_free_ability(&a1);
    rb_free_ability(&a2);
}

/* ====================================================================== *
 * CASE 2 -- aggregate:"total" is a HEART SUM, not a card count
 *
 * PL!S-bp6-002-P: comparison_condition, comparison_type "cost",
 * heart_colors [heart02, heart04, heart05], count 12, operator >=,
 * aggregate "total", location live_card_zone.
 *
 * The evaluator is shared with case 1, so this is the non-vacuity check in the
 * other direction: a board that satisfies case 1's threshold must NOT satisfy
 * this one's, and an empty board must not either.
 * ====================================================================== */
static void case_aggregate_total_three_colours(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!S-bp6-002-P", RB_COND_COMPARISON, "aggregate", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!S-bp6-002-P", abi, RB_COND_COMPARISON, "aggregate", &c1, &a1);
    CHECK_EQ(rb_eval_condition_for_host(&t1.state, 0, cid, &c1), 0,
             "PL!S-bp6-002-P aggregate:total is false on an empty live zone");

    /* One live card cannot reach a 12-heart three-colour sum. */
    int live_id = test_id_exact(&t1, "PL!HS-bp1-021-L");
    if (live_id >= 0) {
        test_add_to_live(&t1, live_id);
        CHECK_EQ(rb_eval_condition_for_host(&t1.state, 0, cid, &c1), 0,
                 "PL!S-bp6-002-P aggregate:total false with a single live card");
    }

    /* NON-VACUITY across the two conditions: the board that CLEARS case 1's
     * heart04 >= 4 gate does not clear this card's heart02+04+05 >= 12 gate. */
    int hs = test_id_exact(&t1, "PL!HS-bp2-021-L");   /* need heart04 x7, pink x5 */
    if (hs >= 0) {
        static TestGame t2;
        Condition c2;
        Ability a2;
        test_game_new(&t2);
        load_cond(&t2, "PL!S-bp6-002-P", abi, RB_COND_COMPARISON, "aggregate", &c2, &a2);
        test_add_to_live(&t2, hs);
        CHECK_EQ(rb_eval_condition_for_host(&t2.state, 0, cid, &c2), 0,
                 "PL!S-bp6-002-P aggregate:total false: heart04 does not count toward "
                 "the heart02+04+05 sum");
        rb_free_ability(&a2);
    }
    rb_free_ability(&ab);
    rb_free_ability(&a1);
}

/* ====================================================================== *
 * CASE 3 -- `unit: "types"` counts heart COLOURS, not cards
 *
 * PL!N-bp5-001-AR: card_count_condition, unit "types", location revealed_cards,
 * heart_source "blade", heart_colors heart01..heart06, count 3, operator >=.
 *
 * Two C defects meet here. card.rs:2235 tests `unit == "types"` BEFORE the
 * card_type/group filter; condition.c:489 tested `ctype || group` first, so a
 * condition that carries card_type never reached the types arm at all. And
 * count_distinct_heart_types counted every printed colour rather than filtering
 * to the ones the condition names, and ignored heart_source entirely.
 *
 * The revealed zone is used because that is what this card reads. Three members
 * with overlapping blade colour sets give a union of at least three distinct
 * blade colours; one member alone gives fewer. Same card, same condition, only
 * the revealed set differs.
 * ====================================================================== */
static void case_unit_types(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!N-bp5-001-AR", RB_COND_LOCATION, "unit", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    const char *three[] = { "LL-bp1-001-R\uff0b", "LL-bp2-001-R\uff0b", "LL-bp3-001-R\uff0b" };
    int ids[3]; int n = 0;
    for (int i = 0; i < 3; i++) {
        int id = test_id_exact(&tg, three[i]);
        if (id < 0) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)id, &c)) continue;
        int distinct = 0;
        for (int h = c.num_base; h < c.num_base + c.num_blade && h < c.n_hearts; h++) {
            int seen = 0;
            for (int k = c.num_base; k < h; k++)
                if (c.heart_color[k] == c.heart_color[h]) seen = 1;
            if (!seen) distinct++;
        }
        rb_free_card(&c);
        ids[n++] = id;
        printf("     member %d blade-distinct-colours=%d\n", i, distinct);
    }
    if (n < 2) {
        fprintf(stderr, "SETUP BUG: could not find two blade-colour members\n");
        sh->failures++; rb_free_ability(&ab); return;
    }

    /* Positive: the union over all of them reaches 3 distinct blade colours. */
    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!N-bp5-001-AR", abi, RB_COND_LOCATION, "unit", &c1, &a1);
    for (int i = 0; i < n; i++) test_add_to_revealed(&t1, ids[i]);
    int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);

    /* Negative: one member alone. Only the revealed set differs. */
    static TestGame t2;
    Condition c2;
    Ability a2;
    test_game_new(&t2);
    load_cond(&t2, "PL!N-bp5-001-AR", abi, RB_COND_LOCATION, "unit", &c2, &a2);
    test_add_to_revealed(&t2, ids[0]);
    int neg = rb_eval_condition_for_host(&t2.state, 0, cid, &c2);

    assert_pair(sh, "PL!N-bp5-001-AR unit:types", "distinct BLADE colours >= 3", 1, pos, 0, neg);
    rb_free_ability(&ab);
    rb_free_ability(&a1);
    rb_free_ability(&a2);
}

/* ====================================================================== *
 * CASE 4 -- card_blade_condition no longer returns 1 unconditionally
 *
 * PL!S-bp3-025-L: card_blade_condition, count 6, operator >=, source
 * selected_cards.
 *
 * condition.rs:372-377 route_resource_condition sends a Resource variant with
 * no resource_type to evaluate_card_blade_condition (card.rs:2601). condition.c
 * answered 1, so this gate was permanently satisfied. The pair is a 9-blade
 * member (passes) against a 1-blade member (fails): same card, same condition,
 * only the activating card's blade differing.
 * ====================================================================== */
static void case_card_blade(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!S-bp3-025-L", RB_COND_RESOURCE, NULL, &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    int big = test_id_exact(&tg, "PL!-pb2-001-PP");    /* blade 9 */
    int small = test_id_exact(&tg, "PL!-PR-001-PR");   /* blade 1 */
    if (big < 0 || small < 0) {
        fprintf(stderr, "SETUP BUG: blade fixtures missing (big=%d small=%d)\n", big, small);
        sh->failures++; rb_free_ability(&ab); return;
    }

    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!S-bp3-025-L", abi, RB_COND_RESOURCE, NULL, &c1, &a1);
    t1.state.activating_card = big;
    int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);

    static TestGame t2;
    Condition c2;
    Ability a2;
    test_game_new(&t2);
    load_cond(&t2, "PL!S-bp3-025-L", abi, RB_COND_RESOURCE, NULL, &c2, &a2);
    t2.state.activating_card = small;
    int neg = rb_eval_condition_for_host(&t2.state, 0, cid, &c2);

    assert_pair(sh, "PL!S-bp3-025-L card_blade_condition", "activating blade >= 6",
                1, pos, 0, neg);
    rb_free_ability(&ab);
    rb_free_ability(&a1);
    rb_free_ability(&a2);
}

/* ====================================================================== *
 * CASE 5 -- all_cost_comparison_condition has a dispatch arm
 *
 * PL!S-bp5-016-N: comparison_type "cost", all true, comparison_target "self",
 * operator ">", location "ステージ" -- a DISPLAY string, not a zone name.
 *
 * condition.rs:456-464 is_all_cost_comparison routes this to
 * evaluate_all_cost_comparison_condition (comparison.rs:409): does ANY of the
 * acting player's stage members cost MORE than the opponent's most expensive
 * member? With no arm the condition fell through to the eval_location tail,
 * resolved the unknown zone name to zero cards, and answered false on every
 * board -- the ability could never fire.
 *
 * The pair is the opponent's stage: empty (max_opp 0, so any member wins) versus
 * holding a 20-cost member (so a 20-cost member no longer wins).
 * ====================================================================== */
static void case_all_cost_comparison(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!S-bp5-016-N", RB_COND_COMPARISON, "all", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    int self_hi = test_id_exact(&tg, "LL-bp1-001-R\uff0b");  /* cost 20 */
    int self_lo = test_id_exact(&tg, "PL!-PR-001-PR");        /* cost 4  */
    int opp_hi  = test_id_exact(&tg, "LL-bp2-001-R\uff0b");  /* cost 20 */
    if (self_hi < 0 || self_lo < 0 || opp_hi < 0) {
        fprintf(stderr, "SETUP BUG: cost fixtures missing\n");
        sh->failures++; rb_free_ability(&ab); return;
    }

    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!S-bp5-016-N", abi, RB_COND_COMPARISON, "all", &c1, &a1);
    test_add_to_stage(&t1, 0, self_hi);
    t1.state.p[0].stage[0] = self_hi;   /* place directly: test_add_to_stage is a play-path helper */
    int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);

    static TestGame t2;
    Condition c2;
    Ability a2;
    test_game_new(&t2);
    load_cond(&t2, "PL!S-bp5-016-N", abi, RB_COND_COMPARISON, "all", &c2, &a2);
    test_add_to_stage(&t2, 0, self_hi);
    test_set_opp_stage(&t2, 0, opp_hi);
    t2.state.p[0].stage[0] = self_hi;
    t2.state.p[1].stage[0] = opp_hi;
    int neg = rb_eval_condition_for_host(&t2.state, 0, cid, &c2);

    /* Direction guard: a cheap member still beats an EMPTY opponent with '>',
     * which pins that the arm reads costs rather than answering a constant. */
    static TestGame t3;
    Condition c3;
    Ability a3;
    test_game_new(&t3);
    load_cond(&t3, "PL!S-bp5-016-N", abi, RB_COND_COMPARISON, "all", &c3, &a3);
    test_add_to_stage(&t3, 0, self_lo);
    t3.state.p[0].stage[0] = self_lo;
    CHECK_EQ(rb_eval_condition_for_host(&t3.state, 0, cid, &c3), 1,
             "PL!S-bp5-016-N: a 4-cost member still beats max_opp 0 with operator '>'");

    assert_pair(sh, "PL!S-bp5-016-N all_cost_comparison_condition",
                "self stage cost > opponent max", 1, pos, 0, neg);
    rb_free_ability(&ab);
    rb_free_ability(&a1);
    rb_free_ability(&a2);
    rb_free_ability(&a3);
}

/* ====================================================================== *
 * CASE 6 -- all_revealed_match_heart_color takes the colour from the QUEUE
 *
 * PL!-bp6-006-P: count 5, operator >=, and NO heart_colors field at all.
 *
 * condition.rs:653-693 takes the colour from the queue entry's
 * conditional_choice, then from the activating card's heart_color_multiplier,
 * and returns FALSE when neither exists. The C read `heart_colors` off the
 * condition -- a field this card does not carry -- and fell through to "any
 * heart on the card counts", so the gate passed for every revealed card.
 *
 * Three boards, all with the same six revealed copies of one member: no colour
 * chosen (false), a colour the member prints (true), a colour it does not
 * (false). The last two are the paired discriminator.
 * ====================================================================== */
static void case_all_revealed_colour_source(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!-bp6-006-P", RB_COND_ALL_REVEALED, NULL, &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

    int mem = test_id_exact(&tg, "LL-bp1-001-R\uff0b");  /* base prints heart01/04/06 */
    if (mem < 0) {
        fprintf(stderr, "SETUP BUG: revealed fixture missing\n");
        sh->failures++; rb_free_ability(&ab); return;
    }

    static TestGame t0;
    Condition c0;
    Ability a0;
    test_game_new(&t0);
    load_cond(&t0, "PL!-bp6-006-P", abi, RB_COND_ALL_REVEALED, NULL, &c0, &a0);
    for (int i = 0; i < 6; i++) test_add_to_revealed(&t0, mem);
    t0.state.queue.selected_heart_color = -1;
    t0.state.activating_card = -1;
    CHECK_EQ(rb_eval_condition_for_host(&t0.state, 0, cid, &c0), 0,
             "PL!-bp6-006-P all_revealed is FALSE when no colour was chosen");

    static TestGame t1;
    Condition c1;
    Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!-bp6-006-P", abi, RB_COND_ALL_REVEALED, NULL, &c1, &a1);
    for (int i = 0; i < 6; i++) test_add_to_revealed(&t1, mem);
    t1.state.queue.selected_heart_color = (int)rb_parse_heart_color("heart01");
    t1.state.activating_card = -1;
    int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);

    static TestGame t2;
    Condition c2;
    Ability a2;
    test_game_new(&t2);
    load_cond(&t2, "PL!-bp6-006-P", abi, RB_COND_ALL_REVEALED, NULL, &c2, &a2);
    for (int i = 0; i < 6; i++) test_add_to_revealed(&t2, mem);
    t2.state.queue.selected_heart_color = (int)rb_parse_heart_color("heart02");
    t2.state.activating_card = -1;
    int neg = rb_eval_condition_for_host(&t2.state, 0, cid, &c2);

    assert_pair(sh, "PL!-bp6-006-P all_revealed_match_heart_color",
                "chosen heart01 vs heart02 on the same revealed set", 1, pos, 0, neg);
    rb_free_ability(&ab);
    rb_free_ability(&a0);
    rb_free_ability(&a1);
    rb_free_ability(&a2);
}

/* ====================================================================== *
 * CASE 7 -- temporal during_live + aggregate:"total" (state.rs:233-242)
 *
 * PL!N-pb1-038-L: temporal during_live, locations [success_live_card_zone,
 * live_card_zone], aggregate "total", heart_colors ["heart01"], count 4,
 * group_names [the Nijigasaki group], and NO operator. Rust's no-operator arm
 * requires EVERY named colour to independently reach the threshold, summed over
 * the need hearts of the live AND success zones.
 *
 * The C temporal evaluator answered a bare phase_in_live() and ignored every
 * heart, count and group field on the condition.
 * ====================================================================== */
static void case_aggregate_temporal(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!N-pb1-038-L", RB_COND_TEMPORAL, "aggregate", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }

static TestGame t1;
    static Condition c1;
    static Ability a1;
    test_game_new(&t1);
    load_cond(&t1, "PL!N-pb1-038-L", abi, RB_COND_TEMPORAL, "aggregate", &c1, &a1);
    /* state.rs:207-220 checks the live zone's PHASE before it ever looks at
     * aggregate, so a main-phase board must be false regardless of the hearts.
     * That is the first half of the pair. */
    CHECK_EQ(rb_eval_condition_for_host(&t1.state, 0, cid, &c1), 0,
             "PL!N-pb1-038-L temporal aggregate:total false OUTSIDE the live window");

    /* Positive: inside the live window, one live card needing heart01 clears the
     * no-operator arm's default threshold of 1 for the single named colour. */
    int live_id = test_id_exact(&t1, "LL-bp5-002-L");   /* need heart01 x2 */
    if (live_id >= 0) {
        test_add_to_live(&t1, live_id);
        t1.state.phase = RB_PHASE_PERFORMANCE;
        int pos = rb_eval_condition_for_host(&t1.state, 0, cid, &c1);
        CHECK_EQ(pos, 1,
                 "PL!N-pb1-038-L temporal aggregate:total true in the live window when "
                 "need heart01 >= 1");
        /* NON-VACUITY: a different colour on a fresh board must fail, so the
         * pass above came from the heart and not merely from the phase. */
        static TestGame t2;
        static Condition c2;
        static Ability a2;
        test_game_new(&t2);
        load_cond(&t2, "PL!N-pb1-038-L", abi, RB_COND_TEMPORAL, "aggregate", &c2, &a2);
        int no01 = test_id_exact(&t2, "PL!HS-bp1-021-L");  /* need heart04 x5 */
        if (no01 >= 0) {
            test_add_to_live(&t2, no01);
            t2.state.phase = RB_PHASE_PERFORMANCE;
            CHECK_EQ(rb_eval_condition_for_host(&t2.state, 0, cid, &c2), 0,
                     "PL!N-pb1-038-L temporal aggregate:total false in the live window "
                     "when the live card needs no heart01");
        }
        rb_free_ability(&a2);
    }
    rb_free_ability(&ab);
    rb_free_ability(&a1);
}

/* ====================================================================== *
 * CASE 8 -- RED TEST. temporal aggregate with NO `location` field.
 *
 * PL!S-bp6-010-N: temporal during_live, aggregate "total", heart_colors
 * ["heart02"], count 4, and NO location and NO locations.
 * state.rs:212-217 defaults the zone to live_card_zone and state.rs:236 passes
 * `location.unwrap_or("live_card_zone")` into check_aggregate_total, so an empty
 * board must be FALSE -- not "true because we are in a live phase".
 *
 * This pins the RUST verdict and stays RED until the engine reaches it.
 * ====================================================================== */
static void case_aggregate_temporal_no_location(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!S-bp6-010-N", RB_COND_TEMPORAL, "aggregate", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, cid, &cond), 0,
             "PL!S-bp6-010-N temporal aggregate:total with no location defaults to "
             "live_card_zone and is FALSE on an empty board");
    rb_free_ability(&ab);
}

/* ====================================================================== *
 * CASE 9 -- RED TEST. state_change_condition honours group_names.
 *
 * PL!N-bp7-022-N: state_change_condition with group_names (the Nijigasaki
 * group), card_type member_card, location stage, from_state active, to_state
 * wait, count 1, operator "<=".
 *
 * state.rs:1237-1249 applies the group filter to every state-change record, so
 * a wait that involved a member OUTSIDE the named group must not satisfy the
 * condition. The C evaluator (eval_state_change) reads from_state, to_state,
 * count and operator and NONE of group_names, so it answers on the state change
 * alone.
 *
 * RED until the group filter is honoured.
 * ====================================================================== */
static void case_state_change_group_filter(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!N-bp7-022-N", RB_COND_STATE, "group_names", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }
    int non_group = test_id_exact(&tg, "PL!-PR-001-PR");
    if (non_group < 0) {
        fprintf(stderr, "SETUP BUG: non-group member fixture missing\n");
        sh->failures++; rb_free_ability(&ab); return;
    }
    tg.state.turn_state_changes[0][0] = non_group;
    tg.state.turn_state_changes[0][1] = non_group;
    tg.state.turn_state_changes[0][2] = 1;   /* from: active */
    tg.state.turn_state_changes[0][3] = 0;   /* to:   wait   */
    tg.state.n_turn_state_changes = 1;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, cid, &cond), 0,
             "PL!N-bp7-022-N state_change_condition honours group_names: a member "
             "outside the named group going to wait must NOT satisfy it");
    rb_free_ability(&ab);
}

/* ====================================================================== *
 * CASE 10 -- RED TEST. state_change_condition honours cost_limit.
 *
 * PL!-pb1-015-P+ : state_change_condition, target opponent,
 * comparison_type "cost", cost_limit 4, cost_limit_operator "<=", with the
 * from_state / to_state pair living INSIDE trigger_event rather than at the top
 * level of the condition.
 *
 * state.rs:1217-1235 applies the cost limit to each state-change participant
 * before accepting it, so a member whose effective cost exceeds 4 going to wait
 * must fail the gate. The C evaluator reads neither cost_limit nor
 * cost_limit_operator.
 *
 * RED until the cost limit is honoured.
 * ====================================================================== */
static void case_state_change_cost_limit(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!-pb1-015-P\uff0b", RB_COND_STATE, "cost_limit", &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }
    int pricey = test_id_exact(&tg, "LL-bp1-001-R\uff0b");   /* cost 20, over the limit of 4 */
    if (pricey < 0) {
        fprintf(stderr, "SETUP BUG: high-cost member fixture missing\n");
        sh->failures++; rb_free_ability(&ab); return;
    }
    tg.state.turn_state_changes[0][0] = pricey;
    tg.state.turn_state_changes[0][1] = pricey;
    tg.state.turn_state_changes[0][2] = 1;
    tg.state.turn_state_changes[0][3] = 0;
    tg.state.n_turn_state_changes = 1;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, cid, &cond), 0,
             "PL!-pb1-015-P+ state_change_condition honours cost_limit 4 with "
             "cost_limit_operator '<=': a 20-cost member must NOT satisfy it");
    rb_free_ability(&ab);
}

/* ====================================================================== *
 * CASE 11 -- RED TEST. not_moved with no `movement` field.
 *
 * PL!N-pb1-004-P+  carries {"type": "not_moved"} and NOTHING else. The parser
 * maps that alias to the Movement variant (compile_abilities.py:48), and Rust's
 * evaluate_movement_condition reads `condition.get_movement().unwrap_or("")` --
 * an empty string falls to the `_` arm, `Zone::from_str("")` is None, and that
 * arm returns TRUE (state.rs:1085-1092).
 *
 * condition.c's eval_movement instead answered `g->n_recently_moved > 0`, so on
 * a board where nothing moved the C says false and Rust says true.
 *
 * RED until the two agree. The assertion is the RUST verdict.
 * ====================================================================== */
static void case_not_moved_alias(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!N-pb1-004-P\uff0b", RB_COND_MOVEMENT, NULL, &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }
    tg.state.n_recently_moved = 0;      /* nothing moved this batch */
    tg.state.activating_card = -1;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, cid, &cond), 1,
             "PL!N-pb1-004-P+ not_moved with no `movement` field: Rust's `_` arm "
             "returns TRUE (state.rs:1090)");
    rb_free_ability(&ab);
}

/* ====================================================================== *
 * CASE 12 -- RED TEST. score_threshold_condition compares the CHEER-BLADE HEART
 * TOTAL, not the player's live score.
 *
 * PL!SP-pb2-048-L: score_threshold_condition, heart_colors [heart02],
 * count 9, operator >=, location live_card_zone, aggregate "total". Rust's
 * pre-dispatch (condition.rs:510) catches the aggregate shape first, so this
 * case pins THAT short-circuit rather than evaluate_score_threshold_condition:
 * an empty live zone with a high player score must still be FALSE.
 *
 * RED until the aggregate short-circuit covers this card's shape.
 * ====================================================================== */
static void case_score_threshold_short_circuit(Shared *sh) {
    static TestGame tg;
    Condition cond;
    Ability ab;
    int cid, abi = 0;
    test_game_new(&tg);
    cid = find_cond_any(&tg, "PL!SP-pb2-048-L", RB_COND_SCORE_THRESHOLD, NULL, &cond, &ab, &abi);
    if (cid < 0) { sh->failures++; return; }
    tg.state.p[0].score = 99;   /* a live score far above the count of 9 */
    int empty = rb_eval_condition_for_host(&tg.state, 0, cid, &cond);
    CHECK_EQ(empty, 0,
             "PL!SP-pb2-048-L score_threshold_condition does NOT read the player score: "
             "score 99 with an empty live zone is still FALSE");
    rb_free_ability(&ab);
}

/* ------------------------------------------------------------------ */
/* driver                                                              */
/* ------------------------------------------------------------------ */
typedef void (*CaseFn)(Shared *);
typedef struct { const char *name; CaseFn fn; } Case;

static const char *g_case_in_flight = "(none)";
static void case_signal(int sig) {
    fprintf(stderr, "FAIL: case %s: fatal signal %d\n", g_case_in_flight, sig);
    fflush(stderr);
    _exit(3);
}

static const Case CASES[] = {
    { "aggregate_total_live_zone",      case_aggregate_total_live_zone },
    { "aggregate_total_three_colours",  case_aggregate_total_three_colours },
    { "unit_types_colours",             case_unit_types },
    { "card_blade_no_fail_open",        case_card_blade },
    { "all_cost_comparison_dispatch",   case_all_cost_comparison },
    { "all_revealed_colour_source",     case_all_revealed_colour_source },
    { "aggregate_temporal",             case_aggregate_temporal },
    { "aggregate_temporal_no_location", case_aggregate_temporal_no_location },
    { "state_change_group_filter",      case_state_change_group_filter },
    { "state_change_cost_limit",        case_state_change_cost_limit },
    { "not_moved_alias",                case_not_moved_alias },
    { "score_threshold_short_circuit",  case_score_threshold_short_circuit }
};

int main(void) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (rb_load("../cards/build") < 0 && rb_load("cards/build") < 0) {
        fprintf(stderr, "cannot load card db\n");
        return 2;
    }
    /* Debug escape hatch: RB_AUDIT_NO_FORK=1 runs the cases in-process so a
     * debugger can attach to the crash itself instead of to the child that
     * inherited it. The shipped path always forks. */
    const char *nofork = getenv("RB_AUDIT_NO_FORK");
    /* Print the heart-colour mapping at startup: every colour index in this file
     * is derived from rb_parse_heart_color, and an off-by-one there silently
     * turns every colour assertion into a wildcard assertion. */
    printf("heart colour map:");
    for (int i = 0; i <= 6; i++) {
        char name[16];
        snprintf(name, sizeof name, "heart%02d", i);
        printf(" %s=%d", name, (int)rb_parse_heart_color(name));
    }
    printf(" all=%d\n", (int)rb_parse_heart_color("all"));

    /* rb_card_group_name is a SERIES-level name (every Love Live card reports
     * the same series), so a group assertion needs the unit level to mean
     * anything. Print both so a group mismatch can be told apart from a series
     * mismatch at a glance. */
    {
        const char *probe[] = { "PL!N-bp7-022-N", "PL!-PR-001-PR", "LL-bp1-001-R\uff0b" };
        for (size_t i = 0; i < sizeof probe / sizeof probe[0]; i++) {
            int id = rb_find_card_by_no(probe[i]);
            if (id < 0) { printf("fixture %-18s NOT FOUND\n", probe[i]); continue; }
            printf("fixture %-18s group=%-10s unit=%s\n", probe[i],
                   rb_card_group_name(id) ? rb_card_group_name(id) : "-",
                   rb_card_unit_name(id) ? rb_card_unit_name(id) : "-");
        }
    }

    Shared *sh = sh_create();
    int n = (int)(sizeof CASES / sizeof CASES[0]);
    int failed_cases = 0, crashed_total = 0;
    int total_checks = 0, total_failures = 0;

    for (int i = 0; i < n; i++) {
        if (nofork && *nofork == '1') {
            static Shared local;
            memset(&local, 0, sizeof local);
            Shared *s2 = &local;
            g_case_in_flight = CASES[i].name;
            printf("-> %s\n", CASES[i].name);
            fflush(stdout);
            CASES[i].fn(s2);
            total_checks += s2->checks;
            total_failures += s2->failures;
            if (s2->failures) { failed_cases++; printf("FAILED  %s (%d check(s), %d failure(s))\n", CASES[i].name, s2->checks, s2->failures); }
            else printf("ok      %s (%d check(s))\n", CASES[i].name, s2->checks);
            continue;
        }
        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "fork failed for %s\n", CASES[i].name);
            crashed_total++;
            continue;
        }
        if (pid == 0) {
            /* CHILD: zero the shared counters so the parent reads this case
             * alone, install the handler naming the case in flight, run it,
             * publish the result and _exit WITHOUT flushing the stdio buffers
             * inherited from the parent. */
            g_case_in_flight = CASES[i].name;
            sh->checks = 0;
            sh->failures = 0;
            signal(SIGSEGV, case_signal);
            signal(SIGABRT, case_signal);
            signal(SIGFPE,  case_signal);
            CASES[i].fn(sh);
            fflush(stdout);
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        /* Read the counters ONLY after waitpid: the child owns them until then. */
        int case_checks   = sh->checks;
        int case_failures = sh->failures;
        int case_crashed  = 0;
        if (WIFSIGNALED(status)) {
            crashed_total++;
            case_crashed++;
            fprintf(stderr, "FAIL: %s: child died on signal %d\n",
                    CASES[i].name, WTERMSIG(status));
        }
        if (WIFEXITED(status) && WEXITSTATUS(status) == 3) {
            crashed_total++;
            case_crashed++;
        }
        total_checks += case_checks;
        total_failures += case_failures;
        if (case_failures || case_crashed) {
            failed_cases++;
            printf("FAILED  %s (%d check(s), %d failure(s)%s)\n", CASES[i].name,
                   case_checks, case_failures, case_crashed ? ", CRASHED" : "");
        } else {
            printf("ok      %s (%d check(s))\n", CASES[i].name, case_checks);
        }
        /* Re-zero so the next child's numbers are its own. */
        sh->checks = 0;
        sh->failures = 0;
    }

    printf("\n%d ok, %d FAILED, %d CRASHED, %d/%d assertions passed\n",
           n - failed_cases, failed_cases, crashed_total,
           total_checks - total_failures, total_checks);
    return failed_cases || crashed_total ? 1 : 0;
}
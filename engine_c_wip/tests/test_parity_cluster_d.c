/* Cluster D parity tests - src/ability/condition.c
 *
 * Every CHECK below asserts what RUST asserts, with the Rust line in the
 * message. No assertion is relaxed, skipped or counted as "expected".
 *
 * Fixtures are found by SCANNING the shipped card database rather than
 * hard-coded, so a regeneration that renumbers card indices cannot silently
 * turn a fixture into a different card; each fixture still prints what it is.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); failures++; } \
    else printf("ok: %s\n", msg); \
} while (0)
#define CHECK_EQ(a, e, msg) do { \
    int av = (a), ev = (e); \
    if (av != ev) { fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, av, ev); failures++; } \
    else printf("ok: %s\n", msg); \
} while (0)

/* rb_check_heart_type_all is exported by condition.c but predates its call site,
   so it is not in rabuka.h (main's file). Local extern, as
   tests/test_parity_zones_position.c does for the same reason. */
int rb_check_heart_type_all(const struct GameState *g, int actor, const Condition *c, int negation);

static CondValue cstr(char *v) { CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_STR; x.s = v; return x; }
static CondValue ci64(long long v) { CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_I64; x.i = v; return x; }
static CondValue ctrue(void) { CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_TRUE; return x; }

/* ── fixtures ─────────────────────────────────────────────────────────────── */
typedef struct {
    int member_no_heart00;      /* member, printed base slice has NO Heart00 */
    int member_heart00_outside;  /* Heart00 present, but NOT in the base slice */
    int member_cost20, member_cost7, member_cost9;
    int member_bladeheart2, member_no_bladeheart, member_blade0_for_test;
} Fixtures;

static void scan_fixtures(Fixtures *f) {
    memset(f, 0, sizeof *f);
    f->member_no_heart00 = f->member_heart00_outside = -1;
    f->member_cost20 = f->member_cost7 = f->member_cost9 = -1;
    f->member_bladeheart2 = f->member_no_bladeheart = f->member_blade0_for_test = -1;
    for (uint32_t cid = 0; cid < 3000; cid++) {
        Card c;
        if (!rb_decode_card_by_index(cid, &c)) continue;
        int is_member = (c.type_flags & 3u) == 0u;
        int base00 = 0, any00 = 0, baseall = 0;
        for (int i = 0; i < c.num_base && i < c.n_hearts; i++) {
            if (c.heart_color[i] == RB_HEART_PINK) base00 = 1;
            if (c.heart_color[i] == RB_HEART_ALL) baseall = 1;
        }
        for (int i = 0; i < c.n_hearts; i++) if (c.heart_color[i] == RB_HEART_PINK) any00 = 1;
        if (is_member) {
            if (!base00 && !baseall && f->member_no_heart00 < 0) f->member_no_heart00 = (int)cid;
            if (c.cost == 20 && f->member_cost20 < 0) f->member_cost20 = (int)cid;
            if (c.cost == 7 && f->member_cost7 < 0) f->member_cost7 = (int)cid;
            if (c.cost == 9 && f->member_cost9 < 0) f->member_cost9 = (int)cid;
            /* has_blade_heart needs a BLADE heart (c.num_blade > 0); the D8
               blade_limit comparison reads the printed blade figure. */
            if (c.num_blade > 0 && c.blade >= 2 && f->member_bladeheart2 < 0)
                f->member_bladeheart2 = (int)cid;
            if (c.num_blade == 0 && c.blade >= 2 && f->member_no_bladeheart < 0)
                f->member_no_bladeheart = (int)cid;
            if (c.num_blade > 0 && c.blade == 0 && f->member_blade0_for_test < 0)
                f->member_blade0_for_test = (int)cid;
        }
        if (any00 && !base00 && f->member_heart00_outside < 0) f->member_heart00_outside = (int)cid;
        rb_free_card(&c);
    }
}

/* ══ D2 - card_has_all_heart is THREE legs (predicates.rs:8-26) ═══════════════
 *
 * The corpus carries ZERO printed base-slice Heart00 and ZERO printed base-slice
 * HeartALL entries (measured: base Heart00 = 0 of 2,639 cards, while 314
 * Heart00 entries exist in the blade/need/special sections), so the predicate is
 * carried ENTIRELY by legs 2 and 3. That makes leg 1's absence invisible as a
 * positive test and makes the OLD body's over-permissiveness the sharper
 * assertion: it read heart_count[0] out of the WHOLE heart array, so a card
 * whose Heart00 sits in its need section answered "has all heart".
 */
static TestGame tg_d2;
static void d2_card_has_all_heart_three_legs(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    g->state.p[0].stage[0] = RB_EMPTY_SLOT;
    g->state.p[0].stage[1] = RB_EMPTY_SLOT;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;

    Condition all = {0};
    all.variant = RB_COND_LOCATION;
    all.fields[0] = (CondField){"heart_type", cstr((char *)"all")};
    all.fields[1] = (CondField){"location", cstr((char *)"stage")};
    all.fields[2] = (CondField){"target", cstr((char *)"self")};
    all.n_fields = 3;

    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 0,
             "D2 leg1/2/3: an empty stage has no ALL heart (predicates.rs:61-65 any() over an empty stage)");
    g->state.p[0].stage[0] = f->member_no_heart00;
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 0,
             "D2: a member whose PRINTED hearts carry no Heart00/All is not an ALL-heart card (predicates.rs:11-13)");
    /* leg 2: mods.heart_modifiers[cid][All].total() > 0 */
    rb_mods_add_heart(&g->state.mods, f->member_no_heart00, RB_HEART_ALL, 1);
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 1,
             "D2 leg2: an ability-granted ALL heart satisfies card_has_all_heart (predicates.rs:14-18)");
    rb_mods_add_heart(&g->state.mods, f->member_no_heart00, RB_HEART_ALL, -1);
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 0,
             "D2 leg2: removing the ALL-heart modifier removes the ALL heart again (total() > 0, not >= 0)");
    /* leg 3: mods.constant_heart_bonuses[cid]["all"] > 0 */
    g->state.mods.constant_heart[f->member_no_heart00][RB_HEART_ALL] = 1;
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 1,
             "D2 leg3: a constant-ability ALL heart satisfies card_has_all_heart (predicates.rs:19-25)");
    g->state.mods.constant_heart[f->member_no_heart00][RB_HEART_ALL] = 0;
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 0,
             "D2 leg3: a zero constant bonus is not an ALL heart (predicate.rs:20-25 `> 0`)");
    /* the old body read heart_count[0] over EVERY section */
    if (f->member_heart00_outside >= 0) {
        g->state.p[0].stage[0] = f->member_heart00_outside;
        CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &all, 0), 0,
                 "D2: a Heart00 outside the BASE slice is not a base_heart entry (predicates.rs:11-13 reads base_heart only)");
    }
    /* D2's other half: the per-card form folds the negation in (predicates.rs:68-79) */
    g->state.p[0].stage[0] = f->member_no_heart00;
    Condition neg_all = all;
    neg_all.fields[3] = (CondField){"negation", ctrue()};
    neg_all.n_fields = 4;
    CHECK_EQ(rb_check_heart_type_all(&g->state, 0, &neg_all, 1), 1,
             "D2: the negation arm answers about the TRIGGERING member only (predicates.rs:41-56)");
}

/* ══ D3 - the negation-skip exemptions (condition.rs:643-656) ═══════════════
 *
 * Card 1239 ab#0 is the only corpus condition with heart_type "all", and it is
 * exactly the shape exception 3 names: a plain `location_condition` (no count)
 * with `negation: true`. Without the exemption the C folds negation twice and
 * answers the OPPOSITE of Rust. */
static void d3_negation_exceptions(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    g->state.p[0].stage[0] = RB_EMPTY_SLOT;
    g->state.p[0].stage[1] = RB_EMPTY_SLOT;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;

    Condition c = {0};                       /* the card-1239 shape, verbatim */
    c.variant = RB_COND_LOCATION;
    c.fields[0] = (CondField){"card_type", cstr((char *)"member_card")};
    c.fields[1] = (CondField){"text", cstr((char *)"no all heart")};
    c.fields[2] = (CondField){"location", cstr((char *)"stage")};
    c.fields[3] = (CondField){"negation", ctrue()};
    c.fields[4] = (CondField){"heart_type", cstr((char *)"all")};
    c.fields[5] = (CondField){"target", cstr((char *)"self")};
    c.n_fields = 6;

    /* Rust's evaluate_location_condition answers FALSE for an empty card set
       (card.rs:912-930 -> an empty `cards` never clears the count), and the
       negation is EXEMPT here, so the empty stage is false rather than true. */
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D3 exception 3: an empty stage has no cards, so the clause is false (card.rs:912-930, not negated at all)");
    g->state.p[0].stage[0] = f->member_no_heart00;   /* no ALL heart -> the clause holds */
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D3 exception 3: a member with no ALL heart satisfies 「ハートを持たない」 once, not twice");
    rb_mods_add_heart(&g->state.mods, f->member_no_heart00, RB_HEART_ALL, 1);
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D3 exception 3: a member WITH an ALL heart fails 「ハートを持たない」 (a doubled negation would say true)");
    rb_mods_add_heart(&g->state.mods, f->member_no_heart00, RB_HEART_ALL, -1);

    /* exception 4: plain location + revealed_cards + nothing revealed */
    Condition rv = {0};
    rv.variant = RB_COND_LOCATION;
    rv.fields[0] = (CondField){"card_type", cstr((char *)"member_card")};
    rv.fields[1] = (CondField){"location", cstr((char *)"revealed_cards")};
    rv.fields[2] = (CondField){"negation", ctrue()};
    rv.fields[3] = (CondField){"target", cstr((char *)"self")};
    rv.n_fields = 4;
    g->state.n_revealed = 0;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &rv), 0,
             "D3 exception 4: an empty revealed set makes 「公開カードに〜がない」 FALSE (condition.rs:649-651)");
    g->state.n_revealed = 1;
    g->state.revealed_cards[0] = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &rv), 0,
             "D3 exception 4: revealing a member_card makes the inner predicate TRUE and the single negation FALSE");
    g->state.n_revealed = 0;
}

/* ══ D4 - evaluate_comparison_condition's missing branches ═════════════════
 *
 * CENSUS (2116 card-ability pairs / 985 abilities / 1589 condition nodes):
 *   Comparison + negation .......... 3 nodes, ALL on the double-negating tail
 *   energy_relative ................. 2 nodes (card 1853)
 *   cost + activating_card .......... 3 nodes (cards 705-707)
 *   original_value + score .......... 2 nodes (cards 2469-2470)
 *   position == "front" ............. 2 nodes (cards 844-845 ab#1)
 *   cost_total ...................... 22 nodes / 13 pairs
 */
static void d4_double_negation_cancels(void) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};                    /* cards 1212-1214 ab#0, verbatim */
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"resource_type", cstr((char *)"surplus_heart")};
    c.fields[1] = (CondField){"negation", ctrue()};
    c.n_fields = 2;
    /* count = 0 surplus, threshold = comparison_default_count = 0 (no location,
       no card_type), so compare_counts(">=", 0, 0) is TRUE; Rust negates it
       twice (comparison.rs:153-157 and condition.rs:643) and answers TRUE. */
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4: Rust's OWN second negation cancels the outer one (comparison.rs:153-157 + condition.rs:643)");

    Condition pos = c;
    pos.fields[2] = (CondField){"negation", {.tag = RB_TAG_FALSE}};
    pos.n_fields = 2;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &pos), 1,
             "D4: the negated and un-negated forms AGREE, which is the measurable signature of the double fold");
}

static void d4_energy_relative(void) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"comparison_type", cstr((char *)"energy_relative")};
    c.fields[1] = (CondField){"count", ci64(1)};
    c.fields[2] = (CondField){"operator", cstr((char *)"=")};
    c.fields[3] = (CondField){"target", cstr((char *)"self")};
    c.n_fields = 4;
    g->state.p[1].energy.n = 0;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 energy_relative: level energy fails 「相手が1枚多い」 (comparison.rs:170-187)");
    g->state.p[1].energy.n = 1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 energy_relative: one card ahead passes 「相手が1枚多い」 (card 1853)");
    g->state.p[1].energy.n = 2;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 energy_relative: two ahead fails an EQUALITY of exactly one (comparison.rs:186)");
    g->state.p[0].energy.n = 3;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 energy_relative: being AHEAD floors the gap to zero, so an equality of 1 is false (comparison.rs:175)");
    g->state.p[1].energy.n = 4;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 energy_relative: being one BEHIND is the clause that fires (comparison.rs:170-187)");
}

static void d4_cost_activating_card(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};                    /* cards 705-707, verbatim */
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[1] = (CondField){"location", cstr((char *)"activating_card")};
    c.fields[2] = (CondField){"count", ci64(10)};
    c.fields[3] = (CondField){"operator", cstr((char *)">=")};
    c.fields[4] = (CondField){"target", cstr((char *)"self")};
    c.n_fields = 5;
    g->state.activating_card = -1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 activating_card cost: no activating card fails (comparison.rs:74-80 `if let Some(act)`)");
    g->state.activating_card = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 activating_card cost: a printed 7 is below 10 (comparison.rs:189-206)");
    g->state.activating_card = f->member_cost20;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 activating_card cost: a printed 20 clears 「コストが10以上」 (card 705)");
    rb_mods_add_cost(&g->state.mods, f->member_cost20, -15);
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 activating_card cost: the EFFECTIVE cost is compared, not the printed one (comparison.rs:191-194)");
}

static void d4_cost_total(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    g->state.p[0].stage[0] = RB_EMPTY_SLOT;
    g->state.p[0].stage[1] = RB_EMPTY_SLOT;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    Condition c = {0};                    /* card 1748 shape, verbatim */
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"count", ci64(16)};
    c.fields[1] = (CondField){"operator", cstr((char *)">=")};
    c.fields[2] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[3] = (CondField){"aggregate", cstr((char *)"total")};
    c.fields[4] = (CondField){"cost_total", ci64(16)};
    c.fields[5] = (CondField){"location", cstr((char *)"stage")};
    c.fields[6] = (CondField){"target", cstr((char *)"self")};
    c.fields[7] = (CondField){"card_type", cstr((char *)"member_card")};
    c.n_fields = 8;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 cost_total: an empty stage sums 0 and clears 16 (comparison.rs:341-388)");
    g->state.p[0].stage[0] = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 cost_total: one member of cost 7 sums 7, not 7 cards-vs-16");
    g->state.p[0].stage[1] = f->member_cost9;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 cost_total: 7 + 9 = 16 clears 「合計16以上」 (card 1748)");
    Condition eq = c;
    eq.fields[3] = (CondField){"operator", cstr((char *)"=")};
    eq.n_fields = 8;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &eq), 1,
             "D4 cost_total: operator '=' makes 16 an exact match (comparison.rs:343-347)");
    /* An operator Rust does not know resolves to MET (util.rs:2210
       compare_with_operator's `.unwrap_or(true)`), NOT to ">=". */
    Condition dflt = c;
    dflt.fields[1] = (CondField){"operator", cstr((char *)"X")};
    dflt.fields[4] = (CondField){"cost_total", ci64(17)};
    dflt.n_fields = 8;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &dflt), 1,
             "D4 cost_total: an unknown operator is MET, not '>=' (util.rs:2210 .unwrap_or(true))");
}

static void d4_cost_total_revealed(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"location", cstr((char *)"revealed_cards")};
    c.fields[1] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[2] = (CondField){"aggregate", cstr((char *)"total")};
    c.fields[3] = (CondField){"cost_total", ci64(7)};
    c.fields[4] = (CondField){"operator", cstr((char *)"=")};
    c.fields[5] = (CondField){"count", ci64(7)};
    c.fields[6] = (CondField){"card_type", cstr((char *)"member_card")};
    c.n_fields = 7;
    g->state.n_revealed = 0;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D4 cost_total: nothing revealed sums 0");
    g->state.n_revealed = 1;
    g->state.revealed_cards[0] = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 cost_total: the revealed member's EFFECTIVE cost is the sum (comparison.rs:355-371)");
}

static void d4_position_front(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};                    /* cards 844-845 ab#1, verbatim */
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"target", cstr((char *)"opponent")};
    c.fields[1] = (CondField){"card_type", cstr((char *)"member_card")};
    c.fields[2] = (CondField){"comparison_target", cstr((char *)"opponent")};
    c.fields[3] = (CondField){"operator", cstr((char *)">=")};
    c.fields[4] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[5] = (CondField){"position", cstr((char *)"front")};
    c.n_fields = 6;
    g->state.p[0].stage[0] = f->member_cost7;
    g->state.p[1].stage[0] = f->member_cost7;
    g->state.activating_card = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 front: equal costs satisfy 「正面のコストが this 以上」 (comparison.rs:406-433)");
    g->state.p[1].stage[0] = f->member_cost20;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 front: a dearer opponent front card satisfies the >= comparison");
    Condition gt = c;
    gt.fields[3] = (CondField){"operator", cstr((char *)">")};
    gt.n_fields = 6;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D4 front: the >= shape is unaffected by the operator spelling");
    CHECK_EQ(rb_eval_condition(&g->state, 0, &gt), 1,
             "D4 front: a strictly dearer opponent front card satisfies '>'");
    g->state.p[1].stage[0] = f->member_cost9;
    g->state.activating_card = f->member_cost20;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &gt), 0,
             "D4 front: a cheaper opponent front card fails '>' (comparison.rs:432)");
    g->state.activating_card = -1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &gt), 0,
             "D4 front: no activating card is false (comparison.rs:407-410)");
}

/* ══ D5 - evaluate_highest_cost_on_stage_condition ═════════════════════════ */
static void d5_highest_cost_uses_position_and_effective_cost(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition left = {0};
    left.variant = RB_COND_COMPARISON;
    left.fields[0] = (CondField){"position", cstr((char *)"left_side")};
    left.fields[1] = (CondField){"operator", cstr((char *)">")};
    left.fields[2] = (CondField){"location", cstr((char *)"stage")};
    left.fields[3] = (CondField){"target", cstr((char *)"self")};
    left.fields[4] = (CondField){"card_type", cstr((char *)"member_card")};
    left.n_fields = 5;
    g->state.p[0].stage[0] = f->member_cost7;    /* left  */
    g->state.p[0].stage[1] = f->member_cost20;   /* centre */
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &left), 0,
             "D5: the LEFT card (7) does not out-cost the centre (20) - the old body hardcoded stage[1] and said true");
    g->state.p[0].stage[0] = f->member_cost20;
    g->state.p[0].stage[1] = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &left), 1,
             "D5: the LEFT card (20) out-costs the centre (7) - the named position is the subject");
    /* the EFFECTIVE cost, not the printed one */
    rb_mods_add_cost(&g->state.mods, f->member_cost20, -15);
    CHECK_EQ(rb_eval_condition(&g->state, 0, &left), 0,
             "D5: a -15 cost modifier drops the left card to 5 and it no longer out-costs 7 (comparison.rs:469)");
    /* cards 2104-2105 verbatim: position center, operator ">=" */
    Condition centre = left;
    centre.fields[0] = (CondField){"position", cstr((char *)"center")};
    centre.fields[1] = (CondField){"operator", cstr((char *)">=")};
    centre.n_fields = 5;
    rb_mods_add_cost(&g->state.mods, f->member_cost20, 15);
    g->state.p[0].stage[1] = f->member_cost20;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &centre), 1,
             "D5: with an equal-cost partner present, '>=' passes (cards 2104-2105)");
    g->state.p[0].stage[2] = f->member_cost9;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &centre), 1,
             "D5: the centre still out-costs a cheaper right-side member");
}

/* ══ D6 - get_count_for_condition's split-out sub-helpers ══════════════════ */
static void d6_cost_revealed_sums_effective_cost(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[1] = (CondField){"location", cstr((char *)"revealed_cards")};
    c.fields[2] = (CondField){"count", ci64(9)};
    c.fields[3] = (CondField){"operator", cstr((char *)">=")};
    c.fields[4] = (CondField){"card_type", cstr((char *)"member_card")};
    c.n_fields = 5;
    g->state.n_revealed = 1;
    g->state.revealed_cards[0] = f->member_cost7;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D6: revealed_cards + comparison_type cost sums COSTS, and 7 < 9 (the old body counted 1 card)");
    rb_mods_add_cost(&g->state.mods, f->member_cost7, 3);
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D6: the sum is of EFFECTIVE costs, so +3 lifts 7 to 10 (card.rs:4002-4005)");
}

static void d6_comparison_target_self_uses_max_cost(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    g->state.p[0].stage[0] = RB_EMPTY_SLOT;
    g->state.p[0].stage[1] = RB_EMPTY_SLOT;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    /* 「このメンバーよりコストの大きいメンバーが自分のステージにいる場合」 */
    Condition c = {0};
    c.variant = RB_COND_COMPARISON;
    c.fields[0] = (CondField){"comparison_type", cstr((char *)"cost")};
    c.fields[1] = (CondField){"location", cstr((char *)"stage")};
    c.fields[2] = (CondField){"comparison_target", cstr((char *)"self")};
    c.fields[3] = (CondField){"count", ci64(10)};
    c.fields[4] = (CondField){"operator", cstr((char *)">")};
    c.fields[5] = (CondField){"target", cstr((char *)"self")};
    c.n_fields = 6;
    g->state.p[0].stage[0] = f->member_cost20;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D6 max-cost: the only member (20) is not dearer than itself");
    g->state.p[0].stage[1] = f->member_cost9;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D6 max-cost: MAX(20, 9) = 20 is not > 10's own comparison partner - the sum would have been 29");
}

/* ══ D7 - no_excess_heart_flag is a TWO-VALUE flag (state.rs:15-22) ════════ */
static void d7_no_excess_is_the_turn_flag(void) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    Condition c = {0};                    /* cards 108-109, verbatim */
    c.variant = RB_COND_NO_EXCESS_HEART;
    c.fields[0] = (CondField){"no_excess_heart", ctrue()};
    c.n_fields = 1;
    g->state.first_attacker = 0;
    g->state.self_no_excess_heart_this_turn = 0;
    g->state.opponent_live_no_excess_heart_this_turn = 0;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D7: the first attacker's flag is clear, so 「余剰ハートを持たない」 is false (state.rs:17-18)");
    g->state.self_no_excess_heart_this_turn = 1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D7: the first attacker's own flag is read, with NO snapshot reconstruction (state.rs:18)");
    g->state.first_attacker = 1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D7: for the second attacker the OTHER flag is read, not the first attacker's (state.rs:19-20)");
    g->state.opponent_live_no_excess_heart_this_turn = 1;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D7: the second attacker's flag answers the condition (state.rs:20)");
}

/* ══ D8 - blade_limit / blade_limit_operator / original_value ══════════════
 * 9 corpus pairs carry blade_limit + blade_limit_operator, and all of them hang
 * off `original_value`. Rust reads them per candidate card inside
 * check_card_property (card.rs:234-237), so a candidate that fails them is
 * dropped from the match set rather than negating the verdict. */
static void d8_original_value_blade_limit(Fixtures *f) {
    test_game_new(&tg_d2);
    TestGame *g = &tg_d2;
    g->state.p[0].stage[0] = RB_EMPTY_SLOT;
    g->state.p[0].stage[1] = RB_EMPTY_SLOT;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    Condition c = {0};
    c.variant = RB_COND_LOCATION;
    c.fields[0] = (CondField){"location", cstr((char *)"stage")};
    c.fields[1] = (CondField){"card_type", cstr((char *)"member_card")};
    c.fields[2] = (CondField){"card_property", cstr((char *)"has_blade_heart")};
    c.fields[3] = (CondField){"original_value", ctrue()};
    c.fields[4] = (CondField){"blade_limit", ci64(2)};
    c.fields[5] = (CondField){"blade_limit_operator", cstr((char *)">=")};
    c.n_fields = 6;
    /* blade_limit >= 2 with a member whose PRINTED blade is 0 (card 50 family):
       the card_property arm would pass, blade_limit rejects the candidate. */
    g->state.p[0].stage[0] = f->member_blade0_for_test;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 0,
             "D8: a member whose PRINTED blade is 0 fails blade_limit >= 2 (card.rs:1244-1252)");
    g->state.p[0].stage[0] = f->member_bladeheart2;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D8: a member whose PRINTED blade clears blade_limit >= 2 matches (card.rs:1246-1252)");
    rb_mods_add_blade(&g->state.mods, f->member_bladeheart2, 5);
    CHECK_EQ(rb_eval_condition(&g->state, 0, &c), 1,
             "D8: blade_limit reads the PRINTED blade, so a granted blade does not help (card.rs:1246-1251)");
    Condition lt = c;
    lt.fields[4] = (CondField){"blade_limit", ci64(5)};
    lt.fields[5] = (CondField){"blade_limit_operator", cstr((char *)"<")};
    lt.n_fields = 6;
    g->state.p[0].stage[0] = f->member_no_bladeheart;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &lt), 0,
             "D8: a member with no blade HEART fails card_property even when blade < 5 (card.rs:238)");
    g->state.p[0].stage[0] = f->member_bladeheart2;
    CHECK_EQ(rb_eval_condition(&g->state, 0, &lt), 1,
             "D8: a printed-blade member below a '<' blade_limit and holding a blade heart matches");
}

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "FAIL: database load\n"); return 1; }
    Fixtures f;
    scan_fixtures(&f);
    printf("fixtures: no_heart00=%d heart00_outside_base=%d c20=%d c7=%d c9=%d bladeheart2=%d no_bladeheart=%d blade0=%d\n",
           f.member_no_heart00, f.member_heart00_outside, f.member_cost20,
           f.member_cost7, f.member_cost9, f.member_bladeheart2, f.member_no_bladeheart,
           f.member_blade0_for_test);
    if (f.member_no_heart00 < 0 || f.member_cost20 < 0 || f.member_cost7 < 0 ||
        f.member_cost9 < 0 || f.member_bladeheart2 < 0 || f.member_no_bladeheart < 0 ||
        f.member_blade0_for_test < 0) {
        fprintf(stderr, "FAIL: fixture scan found no usable card\n");
        return 1;
    }
    d2_card_has_all_heart_three_legs(&f);
    d3_negation_exceptions(&f);
    d4_double_negation_cancels();
    d4_energy_relative();
    d4_cost_activating_card(&f);
    d4_cost_total(&f);
    d4_cost_total_revealed(&f);
    d4_position_front(&f);
    d5_highest_cost_uses_position_and_effective_cost(&f);
    d6_cost_revealed_sums_effective_cost(&f);
    d6_comparison_target_self_uses_max_cost(&f);
    d7_no_excess_is_the_turn_flag();
    d8_original_value_blade_limit(&f);
    rb_unload();
    if (failures) { printf("%d FAILURES\n", failures); return 1; }
    printf("ALL CLUSTER D PARITY CHECKS PASSED\n");
    return 0;
}
/* test_parity_cost_compound.c — C parity suite for the Rust clusters
 *   engine/tests/test_modules/effects/compound/{cost_and_effect,
 *   condition_and_effect,sequential_effects}
 *   engine/tests/test_modules/effects/cost_mod (activation_discount, per_card)
 *
 * Every expectation below is derived from the Rust source of truth; the
 * governing Rust reference is cited in a comment next to each block.
 *
 * Owners: src/ability/cost.c, src/ability/compound.c.
 */

#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    checks++; \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* cost.c exports this; it is not in rabuka.h (that header is another agent's). */
int rb_cost_energy_count_any(const AbilityEffect *e);
const AbilityEffect *rb_route_conditional_branch(const AbilityEffect *effect, int chose_yes, int is_negation);

/* ── local helpers ─────────────────────────────────────────────────────── */

static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static void clear_zones(TestGame *tg, int pl) {
    RbPlayer *p = &tg->state.p[pl];
    p->deck.n = 0;
    p->hand.n = 0;
    p->discard.n = 0;
    p->live.n = 0;
    p->success.n = 0;
    p->energy.n = 0;
    p->energy_deck.n = 0;
    p->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { p->stage[i] = RB_EMPTY_SLOT; p->stage_wait[i] = 0; }
}

/* Minimal AbilityEffect builder for the hand-built cost trees. Strings are
   strdup'd and owned by the effect; rb_effect_free releases them. */
static AbilityEffect *mk(const char *action, const char *source,
                         const char *destination, const char *target,
                         int count, int optional) {
    AbilityEffect *e = (AbilityEffect *)calloc(1, sizeof(AbilityEffect));
    e->action = action ? rb_strdup2(action) : NULL;
    e->source = source ? rb_strdup2(source) : NULL;
    e->destination = destination ? rb_strdup2(destination) : NULL;
    e->target = target ? rb_strdup2(target) : NULL;
    e->count = count;
    e->is_optional = optional;
    return e;
}
static void mk_extra(AbilityEffect *e, const char *k, const char *v) {
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = rb_strdup2(k);
    e->extra_v[e->n_extra] = rb_strdup2(v);
    e->n_extra++;
}
static void mk_add_child(AbilityEffect *parent, AbilityEffect *child) {
    if (parent->n_child >= RB_MAX_CHILD) return;
    parent->child[parent->n_child++] = child;
}

static const char *SHIZUKU = "PL!N-pb1-003-R";   /* 桜坂しずく, 起動/2E self-discard */
static const char *CHIKA   = "PL!SP-bp7-003-R＋"; /* 嵐 千砂都, cost-10-or-20 reveal */
static const char *COST20  = "PL!SP-pb2-005-R";   /* 葉月 恋, cost 20 */
static const char *COST10  = "PL!N-bp1-003-R＋";   /* 桜坂しずく+, cost 10 */
static const char *LOWCOST = "PL!-sd1-010-SD";    /* 高坂穂乃果, low cost */
static const char *FILLER  = "PL!-sd1-010-SD";
static const char *ENERGY_CARD = "LL-E-001-SD";
static const char *HANA    = "PL!S-bp3-016-N";    /* 花丸好実, +1 per success card */
static const char *TANG    = "PL!SP-bp7-002-R";   /*  Sites, +2 own cost */
static const char *TRIPLE  = "LL-bp2-001-R＋";     /* hand-count self reduction */

/* ═══════════════════════════════════════════════════════════════════════
 * A. Decoded cost shape — the decoded extras the cost path reads.
 * Rust: engine/src/ability/effect_decoder_gen.rs:38-73 dispatch.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_decoded_cost_shape(void) {
    int id = rb_find_card_by_no(SHIZUKU);
    CHECK(id >= 0, "桜坂しずく fixture resolves");
    if (id < 0) return;
    Ability ab;
    memset(&ab, 0, sizeof ab);
    CHECK(rb_decode_card_ability((uint32_t)id, 0, &ab) && ab.cost,
          "桜坂しずく 起動 decodes with a cost");
    if (!ab.cost) { rb_free_ability(&ab); return; }
    CHECK(ab.cost->action && !strcmp(ab.cost->action, "sequential_cost"),
          "cost root is a sequential_cost");
    CHECK_EQ(ab.cost->n_child, 2, "sequential cost holds two sub-costs");

    const AbilityEffect *pay = NULL, *move = NULL;
    for (int i = 0; i < ab.cost->n_child; i++) {
        const AbilityEffect *c = ab.cost->child[i];
        if (c->action && !strcmp(c->action, "pay_energy")) pay = c;
        if (c->action && !strcmp(c->action, "move_cards")) move = c;
    }
    CHECK(pay != NULL, "first sub-cost is pay_energy");
    CHECK(move != NULL, "second sub-cost is move_cards");
    /* 「2E」 — the decoder stores the count under the "energy" wire key while
       Rust folds it into `energy_count` (effect_decoder_gen.rs:164 and :226).
       Reading only "energy_count" silently paid zero energy. */
    CHECK_EQ(rb_cost_energy_count_any(pay), 2, "pay_energy amount decodes as 2");
    CHECK(move && move->source && !strcmp(move->source, "hand"),
          "self-discard cost reads from hand");
    CHECK(move && move->destination && !strcmp(move->destination, "discard"),
          "self-discard cost pays into the discard zone");
    CHECK(move && eff_extra(move, "self_cost") &&
              !strcmp(eff_extra(move, "self_cost"), "true"),
          "self-discard cost is flagged self_cost");
    rb_free_ability(&ab);

    int chika = rb_find_card_by_no(CHIKA);
    CHECK(chika >= 0, "嵐 千砂都 fixture resolves");
    if (chika < 0) return;
    memset(&ab, 0, sizeof ab);
    CHECK(rb_decode_card_ability((uint32_t)chika, 2, &ab) && ab.cost,
          "嵐 千砂都 ab#2 decodes with a cost");
    if (ab.cost) {
        CHECK(ab.cost->action && !strcmp(ab.cost->action, "reveal"),
              "cost root is a reveal");
        const char *cv = eff_extra(ab.cost, "cost_values");
        CHECK(cv && !strcmp(cv, "10,20"),
              "reveal cost carries cost_values 10,20");
        CHECK_EQ(ab.cost->count, 1, "reveal cost asks for exactly one card");
    }
    rb_free_ability(&ab);
}

/* ═══════════════════════════════════════════════════════════════════════
 * B. Cost paid, then effect applied (ordering), with the real card.
 * Rust: hand_only_self_discard_draw_q196_test.rs:30-55 asserts
 *   "2 energy should have been paid (15-2=13)" and
 *   "1 card should have been drawn from deck".
 * The engine's cost and effect stages are driven separately here so the two
 * halves are observed independently.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_cost_paid_then_effect_applied(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);

    int shizuku = test_id(&tg, SHIZUKU);
    int filler = test_id(&tg, FILLER);
    CHECK(shizuku >= 0 && filler >= 0, "cost+effect fixtures resolve");
    if (shizuku < 0 || filler < 0) return;

    for (int i = 0; i < 3; i++) test_add_to_stage(&tg, i, filler);
    test_add_to_hand(&tg, shizuku);
    test_add_to_hand(&tg, filler);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);
    tg.state.activating_card = shizuku;

    int deck_before = tg.state.p[0].deck.n;

    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)shizuku, 0, &ab) || !ab.cost) {
        CHECK(0, "桜坂しずく cost decodes for the end-to-end cost run");
        rb_free_ability(&ab);
        return;
    }
    /* The full cost tree is a sequential cost: 2E, then 「このカードを手札から控え室に置く」. */
    CHECK_EQ(rb_pay_cost(&tg.state, 0, ab.cost), 1,
             "the 2E + self-discard cost resolves");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 13,
             "the mandatory 2E leg is actually charged (15-2=13)");
    /* handlers.rs:776-820: a mandatory self-cost from hand never opens a
       hand-selection prompt and pays the activating card itself
       (engine/src/ability/move_cards.rs:958-982). */
    CHECK(!test_has_pending_choice(&tg),
          "the self-discard cost opens no hand-selection prompt");
    CHECK(!test_hand_has(&tg, shizuku),
          "the self-discard cost moved the activating card out of hand");
    CHECK(test_zone_has_id(&tg, 0, "discard", shizuku),
          "the self-discarded card lands in the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "only the activating card left the hand for the cost");

    /* Now the effect, with the cost already settled. */
    int eff_ok = 0;
    if (ab.effect) { rb_execute_effect_ex(&tg.state, 0, ab.effect, shizuku); eff_ok = 1; }
    rb_free_ability(&ab);
    CHECK(eff_ok, "the 起動 effect executes after the cost is paid");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 1,
             "the effect draws 1 card from the deck");
}

/* ═══════════════════════════════════════════════════════════════════════
 * C. Mandatory cost gates — validate_cost is filter aware and all-or-nothing.
 * Rust: engine/src/ability/cost/handlers.rs:204-230 (MoveCards leaf).
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_mandatory_move_cost_validation(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);

    int member = test_id(&tg, LOWCOST);   /* member card */
    int live   = test_id(&tg, "PL!HS-bp1-019-L"); /* live card */
    CHECK(member >= 0 && live >= 0, "filter fixtures resolve");
    if (member < 0 || live < 0) return;

    /* 「ライブカードを1枚控え室に置く」 */
    AbilityEffect *cost = mk("move_cards", "hand", "discard", "self", 1, 0);
    mk_extra(cost, "card_type", "live_card");

    /* hand holds only a member → the cost is unpayable even though the hand is
       non-empty (Rust handlers.rs:215-222 counts through the cost filter, not
       the raw zone length). */
    test_add_to_hand(&tg, member);
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 0,
             "a hand cost whose filter matches nothing is unpayable");
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 0,
             "paying that unpayable cost fails");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "a failed hand cost moves no card out of hand");

    /* a matching card in hand makes the same cost payable */
    test_add_to_hand(&tg, live);
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 1,
             "the same cost validates once a matching card is in hand");

    /* Rule 9.4.2.3 / Q56: costs must be paid in full. */
    cost->count = 2;
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 0,
             "Q56 — a cost needing 2 matching cards is unpayable with 1");
    cost->count = 1;
    rb_effect_free(cost);

    /* Rust handlers.rs:209-214 returns Ok(()) for any source zone that is not
       hand/stage/waitroom/energy, so a deck-sourced cost is never validated. */
    AbilityEffect *deck_cost = mk("move_cards", "deck", "hand", "self", 30, 0);
    tg.state.p[0].deck.n = 0;
    CHECK_EQ(rb_validate_cost(&tg.state, 0, deck_cost), 1,
             "a deck-sourced cost is outside the validated zone set");
    rb_effect_free(deck_cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * D. Sequential cost atomicity — nothing is paid when a later sub-cost
 *    cannot be paid (Rule 9.4.2 / Q234).
 *    Rust: engine/src/ability/cost/handlers.rs:968-1023.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_sequential_cost_atomicity(void) {
    /* Unpayable second leg: 1E first, then discard 5 hand cards. */
    {
        TestGame tg;
        test_game_new(&tg);
        clear_zones(&tg, 0);
        clear_zones(&tg, 1);
        int filler = test_id(&tg, FILLER);
        if (filler < 0) return;
        test_give_energy(&tg, 5);
        test_add_to_hand(&tg, filler);
        int hand_before = tg.state.p[0].hand.n;

        AbilityEffect *seq = mk("sequential_cost", NULL, NULL, NULL, -1, 0);
        AbilityEffect *pay = mk("pay_energy", NULL, NULL, "self", -1, 0);
        mk_extra(pay, "energy", "1");
        AbilityEffect *discard = mk("move_cards", "hand", "discard", "self", 5, 0);
        mk_add_child(seq, pay);
        mk_add_child(seq, discard);

        CHECK_EQ(rb_pay_cost(&tg.state, 0, seq), 0,
                 "a sequential cost with an unpayable leg is refused");
        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 5,
                 "nothing is paid when a later leg is unpayable");
        CHECK_EQ(tg.state.p[0].hand.n, hand_before,
                 "no hand card is discarded by the refused sequential cost");
        CHECK(!test_has_pending_choice(&tg),
              "no pay/skip gate is opened by the refused sequential cost");
        rb_effect_free(seq);
    }

    /* Payable second leg: the first leg is charged, then the second opens its
       selection choice — Rule 9.4.2.2, costs execute in order. */
    {
        TestGame tg;
        test_game_new(&tg);
        clear_zones(&tg, 0);
        clear_zones(&tg, 1);
        int filler = test_id(&tg, FILLER);
        if (filler < 0) return;
        test_give_energy(&tg, 5);
        test_add_to_hand(&tg, filler);

        AbilityEffect *seq = mk("sequential_cost", NULL, NULL, NULL, -1, 0);
        AbilityEffect *pay = mk("pay_energy", NULL, NULL, "self", -1, 0);
        mk_extra(pay, "energy", "1");
        AbilityEffect *discard = mk("move_cards", "hand", "discard", "self", 1, 0);
        mk_add_child(seq, pay);
        mk_add_child(seq, discard);

        CHECK_EQ(rb_pay_cost(&tg.state, 0, seq), 1,
                 "a payable sequential cost succeeds");
        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 4,
                 "the first leg is charged before the second leg resolves");
        CHECK(test_has_pending_choice(&tg),
              "the second leg opens its hand selection choice");
        CHECK_EQ(tg.state.p[0].hand.n, 1,
              "the hand card stays until the player actually selects it");
        rb_effect_free(seq);
    }
}

/* ═══════════════════════════════════════════════════════════════════════
 * E. Optional cost gates — pay-or-skip, and auto-skip when unaffordable.
 *    Rust: engine/src/ability/cost/handlers.rs:1130-1154.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_optional_energy_cost_gate(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);
    int energy_card = test_id(&tg, ENERGY_CARD);
    if (energy_card < 0) return;

    /* Optional 2E with no energy at all: auto-skip, nothing is charged and no
       prompt is opened. */
    AbilityEffect *cost = mk("pay_energy", NULL, NULL, "self", -1, 1);
    mk_extra(cost, "energy", "2");
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "an unaffordable optional energy cost resolves as a skip");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0,
             "the auto-skipped optional cost charges no energy");
    CHECK(!test_has_pending_choice(&tg),
          "an unaffordable optional energy cost opens no gate");
    rb_effect_free(cost);

    /* Optional 2E with energy available: a pay-or-skip gate is opened and
       nothing is charged until the player answers. */
    for (int i = 0; i < 4; i++) test_add_to_energy(&tg, 0, energy_card);
    tg.state.p[0].energy_active = 4;

    cost = mk("pay_energy", NULL, NULL, "self", -1, 1);
    mk_extra(cost, "energy", "2");
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "an affordable optional energy cost resolves to a gate");
    CHECK(test_has_pending_choice(&tg),
          "an affordable optional energy cost opens a pay-or-skip gate");
    CHECK_EQ(tg.state.queue.pending.route, RB_ROUTE_OPTIONAL_COST,
             "the optional-cost gate is routed through OptionalCost");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 4,
             "the energy is not tapped until the gate is answered");

    /* Answering the gate "skip" leaves the energy alone. */
    rb_handle_optional_cost_payment(&tg.state, 0, cost, 0);
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 4,
             "skipping the optional cost gate charges no energy");

    /* Answering the gate "pay" taps exactly the printed amount. */
    test_give_energy(&tg, 0);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "the optional energy gate is re-opened for the pay answer");
    CHECK(test_has_pending_choice(&tg), "the re-opened gate is still pending");
    rb_handle_optional_cost_payment(&tg.state, 0, cost, 1);
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 2,
             "paying the optional cost gate taps the printed energy amount");
    rb_effect_free(cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * F. Energy-condition cost — full payment or refusal.
 *    Rust: engine/src/ability/cost/handlers.rs:1174-1198.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_energy_condition_cost(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);
    int energy_card = test_id(&tg, ENERGY_CARD);
    if (energy_card < 0) return;

    AbilityEffect *cost = mk("energy_condition", NULL, NULL, "self", 2, 0);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 0,
             "an energy condition with too few energy cards is refused");
    CHECK_EQ(tg.state.p[0].energy.n, 0,
             "the refused energy condition removes no card");

    test_add_to_energy(&tg, 0, energy_card);
    tg.state.p[0].energy_active = 1;
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 0,
             "an energy condition needing 2 with 1 in the zone is refused");
    CHECK_EQ(tg.state.p[0].energy.n, 1,
             "the second refusal still removes no card");

    test_add_to_energy(&tg, 0, energy_card);
    tg.state.p[0].energy_active = 2;
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "an energy condition is payable once the zone is full enough");
    CHECK_EQ(tg.state.p[0].energy.n, 0,
             "the energy condition removes exactly the required cards");
    CHECK_EQ(tg.state.p[0].energy_deck.n, 2,
             "the energy condition returns the cards to the energy deck");
    rb_effect_free(cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * G. Change-state cost gates (Q137 / Q144).
 *    Rust: engine/src/ability/cost/handlers.rs:246-267 (validation) and
 *    :900-952 (payment).
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_change_state_cost_gate(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);
    int filler = test_id(&tg, FILLER);
    if (filler < 0) return;

    AbilityEffect *cost = mk("change_state", NULL, NULL, "self", 1, 0);
    mk_extra(cost, "state_change", "wait");

    /* Q137: 「ウェイトにする」 means waiting an ACTIVE member. With an empty
       stage there is no candidate, so a mandatory cost is unpayable. */
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 0,
             "Q137 — waiting with an empty stage is unpayable");
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 0,
             "Q137 — the unpayable wait cost is refused at pay time");

    int a = test_new_id(&tg, FILLER);
    int b = test_new_id(&tg, FILLER);
    test_add_to_stage(&tg, 0, a);
    test_add_to_stage(&tg, 2, b);
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 1,
             "an active stage member makes the wait cost payable");

    /* A member that is already waited cannot pay a 「ウェイトにする」 cost. */
    rb_mods_set_orientation(&tg.state.mods, a, "wait");
    rb_mods_set_orientation(&tg.state.mods, b, "wait");
    CHECK_EQ(rb_validate_cost(&tg.state, 0, cost), 0,
             "Q137 — already-waited members cannot pay a wait cost");
    test_clear_mods_for_card(&tg, a);
    test_clear_mods_for_card(&tg, b);

    /* 1 candidate for a cost of 1 → Rust handlers.rs:923-932 waits every
       candidate directly, with no selection choice. */
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    rb_mods_set_orientation(&tg.state.mods, b, "wait");
    test_clear_mods_for_card(&tg, a);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "a wait cost covering every candidate pays without a choice");
    CHECK(!test_has_pending_choice(&tg),
          "auto-waiting every candidate opens no selection choice");
    const char *ori = rb_mods_get_orientation(&tg.state.mods, a);
    CHECK(ori && !strcmp(ori, "wait"),
          "the auto-waited member is put into the wait state");
    CHECK_EQ(tg.state.n_last_cost_waited_members, 1,
             "last_cost_waited_members records the auto-waited member");
    test_clear_mods_for_card(&tg, a);
    test_clear_mods_for_card(&tg, b);

    /* 2 candidates for a cost of 1 → a stage selection is offered. */
    tg.state.n_last_cost_waited_members = 0;
    test_add_to_stage(&tg, 2, b);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "a wait cost below the candidate count offers a selection");
    CHECK(test_has_pending_choice(&tg),
          "waiting fewer members than candidates opens a stage choice");
    CHECK_EQ(tg.state.queue.pending.count, 1,
             "the stage choice asks for exactly the costed count");
    rb_clear_pending_choice(&tg.state);
    rb_effect_free(cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * H. Reveal cost — the eligible set is filtered by cost_values and the
 *    mandatory exact-count case reveals without a prompt.
 *    Rust: engine/src/ability/cost/handlers.rs:1199-1316; the card-level
 *    expectations come from reveal_cost_ten_or_twenty_under_member_draw_two_test.rs.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_reveal_cost_filtering(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);
    int low = test_id(&tg, LOWCOST);
    int c20 = test_id(&tg, COST20);
    int c10 = test_id(&tg, COST10);
    CHECK(low >= 0 && c20 >= 0 && c10 >= 0, "reveal fixtures resolve");
    if (low < 0 || c20 < 0 || c10 < 0) return;

    AbilityEffect *cost = mk("reveal", "hand", NULL, "self", 1, 0);
    mk_extra(cost, "cost_values", "10,20");
    mk_extra(cost, "card_type", "member_card");

    /* Only a low-cost member in hand → nothing is reveal-eligible, so the cost
       fails and nothing is revealed. */
    test_add_to_hand(&tg, low);
    int hand_before = tg.state.p[0].hand.n;
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 0,
             "a low-cost hand card cannot satisfy 「コストが10か20」");
    CHECK(!test_has_pending_choice(&tg),
          "the unpayable reveal cost opens no selection choice");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "the refused reveal cost keeps the hand untouched");
    CHECK_EQ(tg.state.n_revealed, 0,
             "the refused reveal cost reveals nothing");

    /* Exactly one eligible card for a count-1 mandatory cost → revealed
       directly, with no prompt (handlers.rs:1260-1283). */
    test_add_to_hand(&tg, c20);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "a cost-20 hand card satisfies the 10-or-20 reveal cost");
    CHECK(!test_has_pending_choice(&tg),
          "an exact-count mandatory reveal opens no selection choice");
    CHECK_EQ(tg.state.n_revealed, 1,
             "the exact-count mandatory reveal records the revealed card");
    if (tg.state.n_revealed == 1)
        CHECK_EQ(tg.state.revealed_cards[0], c20,
                 "the revealed card is the cost-20 member");
    tg.state.n_revealed = 0;

    /* Two eligible cards for a count-1 cost → a selection is offered. */
    test_add_to_hand(&tg, c10);
    CHECK_EQ(rb_pay_cost(&tg.state, 0, cost), 1,
             "two eligible cards still resolve the reveal cost");
    CHECK(test_has_pending_choice(&tg),
          "an ambiguous reveal cost opens a selection choice");
    CHECK_EQ(tg.state.queue.pending.count, 1,
             "the reveal selection asks for exactly one card");
    rb_clear_pending_choice(&tg.state);
    rb_effect_free(cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * I. Compound conditional branch routing.
 *    Rust: engine/src/ability/compound/conditional.rs
 *    (route_conditional_branch).
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_conditional_branch_routing(void) {
    AbilityEffect *eff = mk("conditional_on_optional", NULL, NULL, NULL, -1, 0);
    AbilityEffect *opt = mk("modify_score", NULL, NULL, "self", 2, 0);
    AbilityEffect *cond = mk("draw_card", NULL, NULL, "self", 1, 0);
    eff->optional_action = opt;
    eff->conditional_action = cond;

    /* no negation: yes -> conditional_action, no -> nothing */
    eff->conditional_negation = 0;
    CHECK(rb_route_conditional_branch(eff, 1, 0) == cond,
          "chose yes without negation routes to the conditional action");
    CHECK(rb_route_conditional_branch(eff, 0, 0) == NULL,
          "chose no without negation routes to no action");

    /* negation: the branches swap */
    eff->conditional_negation = 1;
    CHECK(rb_route_conditional_branch(eff, 1, 1) == opt,
          "chose yes under negation routes to the optional action");
    CHECK(rb_route_conditional_branch(eff, 0, 1) == cond,
          "chose no under negation routes to the conditional action");

    rb_effect_free(eff);
}

/* ═══════════════════════════════════════════════════════════════════════
 * J. modify_cost — the operation split (subtract vs add) and the additive
 *    accumulation rules in compute_play_cost.
 *    Rust: engine/src/ability/util.rs:183-322.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_modify_cost_operation_split(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);

    /* ── subtract / per-unit over hand size (self reduction) ──
       渡辺 曜&鬼塚夏美&大沢瑠璃乃: 「このカード以外の自分の手札1枚につき、1少なくなる」.
       Rust engine/src/ability/util.rs:285 passes hand_count = len + 1 ("+1
       recovers the true hand count"), and util.rs:170-180 then subtracts 1 for
       exclude_self, so the net divisor input is exactly the current hand
       length. The constant-bonus path (mods.constant_cost_bonuses) is a
       different function and is not what rb_compute_play_cost mirrors. */
    int triple = test_id(&tg, TRIPLE);
    int filler = test_id(&tg, FILLER);
    CHECK(triple >= 0 && filler >= 0, "hand-count reduction fixtures resolve");
    if (triple < 0 || filler < 0) return;
    Card c;
    memset(&c, 0, sizeof c);
    CHECK(rb_decode_card_by_index((uint32_t)triple, &c), "card record decodes");
    int base_cost = c.cost;
    rb_free_card(&c);

    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    /* 3 cards in hand → reduction 3 → base - 3. */
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, triple, 0), base_cost - 3,
             "per-unit subtract reduces the cost by the hand-card count");

    /* Alone in hand → reduction 1. */
    tg.state.p[0].hand.n = 0;
    test_add_to_hand(&tg, triple);
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, triple, 0), base_cost - 1,
             "the per-unit subtract still applies with one card in hand");

    /* On stage the same ability must not discount OTHER hand cards: the
       condition requires the aura source to be in hand
       (Rust util.rs:352-358). */
    tg.state.p[0].hand.n = 0;
    test_add_to_stage(&tg, 1, triple);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    Card f;
    memset(&f, 0, sizeof f);
    rb_decode_card_by_index((uint32_t)filler, &f);
    int filler_cost = f.cost;
    rb_free_card(&f);
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, filler, 0), filler_cost,
             "a hand-conditioned cost aura on stage discounts nothing");

    /* ── add / per success-live-zone card (the increase term) ──
       花丸好実: stage cost +1 per card in the success live zone.
       Rust: success_zone_stage_cost_deploy_energy_test.rs (1 success card
       → play cost = base + 1). */
    int hana = test_id(&tg, HANA);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    CHECK(hana >= 0 && live >= 0, "success-zone increase fixtures resolve");
    if (hana < 0 || live < 0) return;
    memset(&c, 0, sizeof c);
    rb_decode_card_by_index((uint32_t)hana, &c);
    int hana_base = c.cost;
    rb_free_card(&c);

    tg.state.p[0].hand.n = 0;
    tg.state.p[0].success.n = 0;
    test_add_to_hand(&tg, hana);
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, 0), hana_base,
             "no success-zone increase with an empty success zone");
    test_add_to_success(&tg, live);
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, 0), hana_base + 1,
             "one success live card adds +1 to the stage cost");
    test_add_to_success(&tg, live);
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, 0), hana_base + 2,
             "the success-zone increase scales with the success-zone count");

    /* The set-override replaces the computed cost outright — Rust util.rs:317-320
       applies it AFTER base - reduction + increase, so it is not additive.
       0 means "no override", and a value equal to the base cost is a no-op. */
    int override_cost = hana_base + 7;
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, override_cost), override_cost,
             "a set override replaces the computed play cost entirely");
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, hana_base), hana_base + 2,
             "a set override equal to the base cost is a no-op");
    CHECK_EQ(rb_compute_play_cost(&tg.state, 0, hana, 0), hana_base + 2,
             "set_override 0 means no override");
}

/* ═══════════════════════════════════════════════════════════════════════
 * K. Constant cost bonuses must be ADDITIVE across sources, not an absolute
 *    override. (Reported to the modifiers.c owner if this fails — that file is
 *    not mine to edit.)
 *    Rust: engine/src/core/modifiers.rs recalculate_constant_cost_modifiers.
 * ═══════════════════════════════════════════════════════════════════════ */
static void test_constant_cost_bonus_is_additive(void) {
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg, 0);
    clear_zones(&tg, 1);
    int tang = test_new_id(&tg, TANG);   /* 「このメンバーのコストを+2」 */
    int tang2 = test_new_id(&tg, TANG);
    if (tang < 0 || tang2 < 0) return;
    int energy_card = test_id(&tg, ENERGY_CARD);
    test_give_energy(&tg, 8);

    test_add_to_stage(&tg, 1, tang);
    test_recalc(&tg);
    int single = test_get_cost_modifier(&tg, tang);
    CHECK_EQ(single, 2, "one 「コストを+2」 source contributes +2");

    test_add_to_stage(&tg, 0, tang2);
    test_recalc(&tg);
    int doubled = test_get_cost_modifier(&tg, tang2);
    CHECK_EQ(doubled, 2, "a second 「コストを+2」 source also contributes +2");
    /* Both are separate cards, so each should see its own +2 — never +4 on
       one of them (an absolute-override accumulation would clobber). */
    CHECK_EQ(test_get_cost_modifier(&tg, tang), single,
             "the first 「コストを+2」 card keeps its own +2");
    (void)energy_card;
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 2;
    }
    test_decoded_cost_shape();
    test_cost_paid_then_effect_applied();
    test_mandatory_move_cost_validation();
    test_sequential_cost_atomicity();
    test_optional_energy_cost_gate();
    test_energy_condition_cost();
    test_change_state_cost_gate();
    test_reveal_cost_filtering();
    test_conditional_branch_routing();
    test_modify_cost_operation_split();
    test_constant_cost_bonus_is_additive();

    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL COST/COMPOUND PARITY CHECKS PASSED\n");
    return failures == 0 ? 0 : 1;
}

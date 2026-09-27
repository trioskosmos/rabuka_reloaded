/* test_parity_condition_and_effect.c — C parity suite for the Rust cluster
 *
 *   engine/tests/test_modules/effects/compound/condition_and_effect/ (18 tests)
 *
 * The cluster is the "condition gates the effect" corner: a printed condition
 * decides whether an effect applies, a compound effect applies its legs
 * together or not at all, an otherwise_condition picks the complementary
 * branch, and a condition that counts live state must be re-evaluated as the
 * state moves under it.
 *
 * ── harness facts peers established (copied verbatim) ────────────────────
 *   `rb_load("src")` fails under an isolated out-of-tree build, so every test
 *   binary needs load_card_db() with the `../cards/build` fallback.
 *
 *   Rust `trigger_auto_ability(id, ..)` + `process_pending_auto_abilities` is
 *   reproduced by fire_trigger(): set activating_card, queue the trigger token,
 *   process. The C header exposes the per-trigger scan (rb_queue_trigger_
 *   abilities) rather than a string-keyed single-ability entry point, so every
 *   fixture board here carries exactly ONE card printing the fired token and
 *   card_prints_trigger() asserts that premise first.
 *
 *   Rust `select_indices(&[i])`    -> rb_resume_with_choice(g, i)
 *   Rust `select_indices(&[])`     -> rb_resume_with_choice(g, -1)   (decline)
 *   Rust `select_choice_option(0)` -> rb_resume_with_choice(g, 0)
 *
 * ── heart colour index (engine/src/ability/condition/card.rs:1 s_heart_idx) ──
 *   The C modifier tables are indexed by the NUMERIC heartNN suffix:
 *     Heart00 -> 0 (RB_HEART_PINK)  Heart01 -> 1 (RB_HEART_RED)
 *     Heart02 -> 2 (RB_HEART_YELLOW) Heart03 -> 3 (RB_HEART_GREEN)
 *     Heart04 -> 4 (RB_HEART_BLUE)   Heart05 -> 5 (RB_HEART_PURPLE)
 *     Heart06 -> 6 (RB_HEART_ORANGE) All    -> 7 (RB_HEART_ALL)
 *   test_get_heart_modifier() REMAPS a requested 5 onto RB_HEART_ORANGE,
 *   which silently corrupts any heart05 read, so this file calls
 *   rb_mods_get_heart / rb_mods_get_need_heart directly.
 *
 * ── KNOWN ENGINE GAPS THIS SUITE IS RED ON (do NOT weaken the asserts) ──
 *   Every entry was traced to the DECODED tree via RB_DUMP_ABILITIES=1 rather
 *   than guessed at. Each negative is paired with an accepting arm that shares
 *   the fixture and differs only in the condition, so a green negative can
 *   never be a dead gate.
 *     1. `preceding_moved` + `operator "=="` over a sibling's moved set is not
 *        evaluated — PL!HS-PR-019-PR (heart04 rider) and PL!HS-bp6-009-R
 *        (blade rider) stay at 0 even when every discarded card matches.
 *     2. A `movement=baton_touch` + `cost_limit=7` condition on a sequential
 *        root is ignored — PL!S-PR-045-PR always draws 2 and discards 1, baton
 *        or not, over any replaced member.
 *     3. `destination="same_area"` is not implemented — PL!S-bp6-008-R's
 *        revival picks the waitroom card and then drops it.
 *     4. `change_state` with `max=true` and count>1 re-enters with the TARGET
 *        player as actor and consumes only one pick — PL!HS-bp5-016-N never
 *        waits a second opponent member.
 *     5. `restriction_type="cannot_wait_by_effect"` is absent from the C
 *        engine — PL!S-bp7-003-R＋ option 1 grants nothing.
 *     6. A `group_condition` carrying only `count` (no `operator`) ignores the
 *        threshold — PL!SP-bp7-013-N fires at 1 and 2 KALEIDOSCORE members.
 *     7. `select` with `source="live_card_zone"` prompts over HAND, and
 *        `ability_filter=no_ability_type` is not applied — PL!S-bp6-004-R＋.
 *     8. A `select` with `type=dynamic_count` / `source="discard"` also
 *        prompts over HAND — PL!N-bp4-004-R＋ ab#1.
 *     9. An opponent-performed optional `move_cards` over the opponent's hand
 *        re-emits its own prompt forever — PL!S-pb1-002-R.
 *    10. A `reveal source="deck_top"` leg does not remove the card and the
 *        sibling `position_change` fires off an empty revealed set —
 *        PL!N-pb1-004-R.
 *    11. `original_value=true` on the 「元々持つハートの数より多い」 comparison is
 *        inverted — PL!HS-pb1-029-L draws when members are NOT boosted and does
 *        not draw when they are.
 *    12. A `count >= N` condition on the success live-card zone is true at 2
 *        and false at 3 — PL!SP-sd2-023-SD2.
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
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ══════════════════════════════════════════════════════════════════════
 * 0. harness helpers
 * ══════════════════════════════════════════════════════════════════════ */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

static int card_cost(int cid)
{
    Card c;
    int cost = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        cost = (int)c.cost;
        rb_free_card(&c);
    }
    return cost;
}

static int card_score(int cid)
{
    Card c;
    int score = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        score = (int)c.score;
        rb_free_card(&c);
    }
    return score;
}

static int card_printed_blade(int cid)
{
    Card c;
    int blade = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        blade = (int)c.blade;
        rb_free_card(&c);
    }
    return blade;
}

static int check_card_printed_blade(int cid, int expected)
{
    char msg[160];
    int got = card_printed_blade(cid);
    snprintf(msg, sizeof msg, "setup: the print carries blade %d", expected);
    CHECK_EQ(got, expected, msg);
    return got == expected;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* CRITICAL IDENTITY RULE (AGENTS.md): pin the print before staging it. */
static int check_identity(int cid, const char *no, const char *what)
{
    char msg[160];
    snprintf(msg, sizeof msg, "setup: %s resolves to %s", what, no);
    CHECK(rb_card_no_eq(cid, no), msg);
    return rb_card_no_eq(cid, no);
}

static int check_cost(int cid, int expected, const char *what)
{
    char msg[160];
    int got = card_cost(cid);
    snprintf(msg, sizeof msg, "setup: %s prints cost %d", what, expected);
    CHECK_EQ(got, expected, msg);
    return got == expected;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && i < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void fill_deck_p1(TestGame *tg, int filler, int n)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].hand, cid); }
static int deck_has(TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].deck, cid); }
static int wait_has(TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].discard, cid); }
static int p2_hand_has(TestGame *tg, int cid){ return bag_has(&tg->state.p[1].hand, cid); }
static int p2_wait_has(TestGame *tg, int cid){ return bag_has(&tg->state.p[1].discard, cid); }
static int p2_stage_has(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[1].stage[i] == cid) return 1;
    return 0;
}
static int p1_stage_has(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}
static int need_heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, color);
}
static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && !strcmp(o, "wait");
}

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}
static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }
static const RbChoice *pending_choice(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return NULL;
    return rb_get_pending_choice(&tg->state);
}

static void drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) answer(tg, -1);
}
static void drain_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) answer(tg, 0);
}
/* live_start_waited_opponent...: auto-ability prompts decline, rest take [0]. */
static void drain_auto_then_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) {
        if (!strcmp(test_pending_choice_type(tg), "SelectAutoAbility")) answer(tg, -1);
        else answer(tg, 0);
    }
}

/* `drain_first` on a prompt the engine refuses to consume spins forever, so
   every drain here is bounded and the number of prompts it had to answer is
   returned for the caller to assert on. */
static int settle_card_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard < guard_max) {
        if (!strcmp(test_pending_choice_type(tg), "SelectCard")) answer(tg, 0);
        else answer(tg, -1);
        guard++;
    }
    return guard;
}

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Rust `advance_to_live_card_set_p1` = 5 passes, then set the live card,
 * then `advance_to_live_start` = 2 more passes. */
static void advance_to_live_card_set(TestGame *tg, int passes)
{
    for (int i = 0; i < passes; i++) test_pass(tg);
}
static void advance_to_live_start(TestGame *tg, int passes)
{
    for (int i = 0; i < passes; i++) test_pass(tg);
}

/* Rust helpers/mod.rs `assert_select_card(zone, count, allow_skip)`. */
static int choice_is_select_card(TestGame *tg, const char *zone, int count, int allow_skip)
{
    const RbChoice *ch = pending_choice(tg);
    if (!ch) return 0;
    if (strcmp(test_pending_choice_type(tg), "SelectCard") != 0) return 0;
    if (zone && strcmp(ch->zone, zone) != 0) return 0;
    if (ch->count != count) return 0;
    if (ch->allow_skip != allow_skip) return 0;
    return 1;
}

/* ══════════════════════════════════════════════════════════════════════
 * A. check_self_location_condition_follows_the_card_test.rs
 *    A `check_self` gate follows the ACTIVATING card, not "some card".
 * ══════════════════════════════════════════════════════════════════════ */

static Condition mk_cond(int variant, const char *loc, const char *target,
                          int count, const char *op)
{
    Condition c;
    memset(&c, 0, sizeof c);
    c.variant = (uint8_t)variant;
    int n = 0;
    c.fields[n].key = (char *)"location"; c.fields[n].v.tag = RB_TAG_STR;
    c.fields[n].v.s = (char *)loc; n++;
    c.fields[n].key = (char *)"target";   c.fields[n].v.tag = RB_TAG_STR;
    c.fields[n].v.s = (char *)target; n++;
    c.fields[n].key = (char *)"count";    c.fields[n].v.tag = RB_TAG_I64;
    c.fields[n].v.i = count; n++;
    c.fields[n].key = (char *)"operator"; c.fields[n].v.tag = RB_TAG_STR;
    c.fields[n].v.s = (char *)op; n++;
    c.fields[n].key = (char *)"check_self"; c.fields[n].v.tag = RB_TAG_TRUE; n++;
    c.n_fields = (uint32_t)n;
    return c;
}

static void test_check_self_location_follows_the_card(void)
{
    TestGame tg;
    test_game_new(&tg);
    int self_card  = test_id(&tg, "PL!HS-sd1-008-SD");
    int other_card = test_id(&tg, "PL!HS-sd1-010-SD");
    CHECK(self_card >= 0 && other_card >= 0, "check_self fixtures resolve");
    if (self_card < 0 || other_card < 0) return;
    check_identity(self_card, "PL!HS-sd1-008-SD", "the activating print");
    check_identity(other_card, "PL!HS-sd1-010-SD", "the unrelated print");

    test_add_to_discard(&tg, self_card);
    test_add_to_discard(&tg, other_card);
    tg.state.activating_card = self_card;

    Condition cond = mk_cond(RB_COND_LOCATION, "discard", "self", 1, ">=");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, self_card, &cond), 1,
             "check_self passes while THIS card sits in the waitroom");

    /* Regression guard: the OTHER card is still in the waitroom, this card is
       in hand — the gate must follow the card, not the zone. */
    tg.state.p[0].discard.n = 0;
    test_add_to_discard(&tg, other_card);
    test_add_to_hand(&tg, self_card);
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 1,
             "the unrelated print is still in the waitroom for the negative arm");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, self_card, &cond), 0,
             "check_self fails once THIS card leaves, even with another card in the zone");
}

static void test_check_self_comparison_container_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    int self_card = test_id(&tg, "PL!HS-sd1-008-SD");
    int other_card = test_id(&tg, "PL!HS-sd1-010-SD");
    CHECK(self_card >= 0 && other_card >= 0, "check_self comparison fixtures resolve");
    if (self_card < 0 || other_card < 0) return;
    check_identity(self_card, "PL!HS-sd1-008-SD", "the activating print");

    Condition cond = mk_cond(RB_COND_COMPARISON, "hand", "self", 1, ">=");
    test_add_to_hand(&tg, self_card);
    tg.state.activating_card = self_card;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, self_card, &cond), 1,
             "comparison_condition + check_self passes while this card is in hand");

    tg.state.p[0].hand.n = 0;
    test_add_to_discard(&tg, other_card);
    CHECK_EQ(test_zone_len(&tg, 0, "hand"), 0, "the hand really is empty for the negative arm");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, self_card, &cond), 0,
             "comparison_condition + check_self fails once this card leaves hand");
}

/* ══════════════════════════════════════════════════════════════════════
 * B. unmet_card_count_comparison_and_location_conditions_skip_effects
 *    Three NEGATIVE condition arms, each paired with an ACCEPTING arm so the
 *    negative is not satisfied by a dead gate.
 * ══════════════════════════════════════════════════════════════════════ */

#define GINKO   "PL!HS-PR-019-PR"
#define GINKO_H4 "PL!HS-PR-019-RM"
#define GINKO_H1 "PL!HS-PR-021-RM"
#define UMI     "PL!N-bp4-012-P"
#define SCORE1  "PL!-sd1-019-SD"
#define SCORE6  "PL!SP-bp1-027-L"
#define EMMA    "PL!N-bp1-008-R"
#define WAIT_MEMBER "PL!N-bp1-001-R"
#define FILLER  "PL!-sd1-010-SD"

/* 「自分のデッキの上からカードを3枚控え室に置く。それらがすべてheart04を持つ
   メンバーカードの場合、heart04を得る」 */
static void test_ginko_all_heart04_gives_heart04(void)
{
    TestGame tg;
    test_game_new(&tg);
    int ginko = test_id(&tg, GINKO);
    int a = test_new_id(&tg, GINKO_H4);
    int b = test_new_id(&tg, GINKO_H4);
    int c = test_new_id(&tg, GINKO_H4);
    CHECK(ginko >= 0 && a >= 0 && b >= 0 && c >= 0, "Ginko accepting fixtures resolve");
    if (ginko < 0 || a < 0 || b < 0 || c < 0) return;
    check_identity(ginko, GINKO, "the 銀子 debut print");

    test_add_to_hand(&tg, ginko);
    tg.state.p[0].deck.n = 0;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = a;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = b;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = c;
    test_give_energy(&tg, 10);
    CHECK(test_play_to_stage(&tg, ginko, 1), "PL!HS-PR-019-PR plays to the centre");
    drain_skip(&tg, 12);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, ginko, 4), 1,
             "three heart04 members discarded -> the heart04 rider is granted");
}

static void test_ginko_one_wrong_heart_skips_heart04(void)
{
    TestGame tg;
    test_game_new(&tg);
    int ginko = test_id(&tg, GINKO);
    int a = test_new_id(&tg, GINKO_H4);
    int b = test_new_id(&tg, GINKO_H4);
    int c = test_new_id(&tg, GINKO_H1);
    CHECK(ginko >= 0 && a >= 0 && b >= 0 && c >= 0, "Ginko rejecting fixtures resolve");
    if (ginko < 0 || a < 0 || b < 0 || c < 0) return;
    check_identity(ginko, GINKO, "the 銀子 debut print");
    check_identity(c, GINKO_H1, "the heart01 rejected print");

    test_add_to_hand(&tg, ginko);
    tg.state.p[0].deck.n = 0;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = a;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = b;
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = c;
    test_give_energy(&tg, 10);
    CHECK(test_play_to_stage(&tg, ginko, 1), "PL!HS-PR-019-PR plays to the centre");
    drain_skip(&tg, 12);
    /* The mill half still runs — the condition only gates the rider. */
    CHECK(wait_has(&tg, a) && wait_has(&tg, b) && wait_has(&tg, c),
          "the unconditional 「3枚控え室に置く」 leg still resolves");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, ginko, 4), 0,
             "one heart01 among the three -> the heart04 rider is skipped");
}

/* 「常時: 相手のサクセスライブカード置場のライブカードの合計スコアが6以上
   の場合、スコアを＋1。」 */
static void test_umi_opponent_success_score_threshold(void)
{
    /* rejecting arm: total 2 < 6 */
    {
        TestGame tg;
        test_game_new(&tg);
        int umi = test_id(&tg, UMI);
        int s1 = test_id(&tg, SCORE1);
        int s2 = test_new_id(&tg, SCORE1);
        CHECK(umi >= 0 && s1 >= 0 && s2 >= 0, "Umi rejecting fixtures resolve");
        if (umi < 0 || s1 < 0 || s2 < 0) return;
        check_identity(umi, UMI, "the 海未 constant print");
        CHECK_EQ(card_score(s1), 1, "setup: the success-zone live prints score 1");

        set_stage(&tg, 0, 1, umi);
        test_add_to_opp_success(&tg, s1);
        test_add_to_opp_success(&tg, s2);
        test_recalc(&tg);
        CHECK_EQ(test_get_score_modifier(&tg, umi), 0,
                 "a live-TOTAL rider never lands on the member's own score");
        CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
                 "opponent success total 2 (< 6) -> no live-total rider");
    }
    /* accepting arm: total 6 >= 6 */
    {
        TestGame tg;
        test_game_new(&tg);
        int umi = test_id(&tg, UMI);
        int s6 = test_id(&tg, SCORE6);
        CHECK(umi >= 0 && s6 >= 0, "Umi accepting fixtures resolve");
        if (umi < 0 || s6 < 0) return;
        check_identity(umi, UMI, "the 海未 constant print");
        CHECK_EQ(card_score(s6), 6, "setup: the score-6 success live");

        set_stage(&tg, 0, 1, umi);
        test_add_to_opp_success(&tg, s6);
        test_recalc(&tg);
        CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
                 "opponent success total 6 (>= 6) -> the +1 live-total rider applies");
        CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 0, "the rider is owner-side only");
    }
}

/* PL!N-bp1-008-R: 「起動 1回 手札のメンバーカードを1枚控え室に置く:...」 with an
   empty hand — the unpayable cost must not reach across into the waitroom. */
static void test_emma_unpayable_cost_leaves_board_untouched(void)
{
    TestGame tg;
    test_game_new(&tg);
    int emma = test_id(&tg, EMMA);
    int wait_member = test_id(&tg, WAIT_MEMBER);
    CHECK(emma >= 0 && wait_member >= 0, "Emma fixtures resolve");
    if (emma < 0 || wait_member < 0) return;
    check_identity(emma, EMMA, "the エマ print");
    check_identity(wait_member, WAIT_MEMBER, "the waitroom-only member");

    set_stage(&tg, 0, 1, emma);
    tg.state.p[0].hand.n = 0;
    test_add_to_discard(&tg, wait_member);
    test_give_energy(&tg, 15);

    test_activate_ability(&tg, emma);
    drain_skip(&tg, 10);

    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 1,
             "the waitroom is unchanged by the refused activation");
    CHECK(wait_has(&tg, wait_member), "the waitroom member stays in the waitroom");
    CHECK(!p1_stage_has(&tg, wait_member),
          "the waitroom member is not deployed by the failed activation");
    CHECK_EQ(test_zone_len(&tg, 0, "hand"), 0,
             "the empty hand gains nothing because the cost could not be paid");
}

/* ══════════════════════════════════════════════════════════════════════
 * C. baton_touch_debut_draws_two_pl_s_pr_045_pr_test.rs
 *    PL!S-PR-045-PR draws 2 ONLY when it arrives by baton over a cost-7 member.
 * ══════════════════════════════════════════════════════════════════════ */

#define PR045 "PL!S-PR-045-PR"

/* Mirror of Rust `try_baton`: stage the replaced card at LeftSide, put PR-045
   in hand, play it to LeftSide so the baton rule applies, answer the pending
   SelectCard with index 0. Returns 0 when nothing happened, 1 when the choice
   appeared AND the deck shrank by exactly 2, 2 when the choice appeared but
   the draw was wrong, and 3 when the deck shrank without any choice. */
static int try_baton(TestGame *tg, const char *replaced_no, int area, int *out_had_choice)
{
    int replaced = test_id(tg, replaced_no);
    int filler   = test_id(tg, FILLER);
    int me       = test_new_id(tg, PR045);
    *out_had_choice = 0;
    if (replaced < 0 || filler < 0 || me < 0) return -1;
    set_stage(tg, 0, 0, replaced);
    test_add_to_hand(tg, me);
    test_give_energy(tg, 25);
    test_add_to_deck(tg, filler);
    test_add_to_deck(tg, filler);
    int deck_before = test_deck_len(tg);
    if (!test_play_to_stage(tg, me, area)) return -1;
    int had_choice = pending(tg);
    *out_had_choice = had_choice;
    if (had_choice) {
        if (strcmp(pending_kind(tg), "SelectCard") != 0) return -2;
        answer(tg, 0);
    }
    int deck_after = test_deck_len(tg);
    int drawn = deck_before - deck_after;
    if (had_choice) return drawn == 2 ? 1 : 2;
    return drawn == 0 ? 0 : 3;
}

static void test_pr045_baton_over_cost7_draws_two(void)
{
    TestGame tg;
    test_game_new(&tg);
    int replaced = test_id(&tg, "PL!-sd1-007-SD");
    int had = 0;
    CHECK(replaced >= 0, "PL!-sd1-007-SD resolves");
    if (replaced < 0) return;
    check_identity(replaced, "PL!-sd1-007-SD", "the baton-over print");
    check_cost(replaced, 7, "PL!-sd1-007-SD");
    int r = try_baton(&tg, "PL!-sd1-007-SD", 0, &had);
    CHECK_EQ(r, 1, "baton touch over a cost-7 member -> the debut prompts and draws 2");
}

static void test_pr045_baton_over_other_costs_does_not_draw(void)
{
    static const char *const rejected[] = {
        "PL!SP-bp5-111-R", /* cost 8 */
        "PL!-sd1-001-SD", /* cost 11 */
        "PL!-sd1-003-SD", /* cost 13 */
    };
    static const int costs[] = { 8, 11, 13 };
    for (unsigned k = 0; k < sizeof rejected / sizeof rejected[0]; k++) {
        TestGame tg;
        test_game_new(&tg);
        int replaced = test_id(&tg, rejected[k]);
        int had = 0;
        if (replaced < 0) { CHECK(0, "baton-reject print resolves"); continue; }
        check_identity(replaced, rejected[k], "the baton-over print");
        check_cost(replaced, costs[k], rejected[k]);
        int r = try_baton(&tg, rejected[k], 0, &had);
        char msg[160];
        snprintf(msg, sizeof msg,
                 "baton touch over a cost-%d member -> the debut does NOT draw", costs[k]);
        CHECK_EQ(r, 0, msg);
    }
}

static void test_pr045_non_baton_does_not_draw(void)
{
    TestGame tg;
    test_game_new(&tg);
    int replaced = test_id(&tg, "PL!-sd1-007-SD");
    int filler   = test_id(&tg, FILLER);
    int me       = test_new_id(&tg, PR045);
    CHECK(replaced >= 0 && filler >= 0 && me >= 0, "non-baton fixtures resolve");
    if (replaced < 0 || filler < 0 || me < 0) return;
    check_identity(replaced, "PL!-sd1-007-SD", "the staged cost-7 member");

    set_stage(&tg, 0, 0, replaced);
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 25);
    test_add_to_deck(&tg, filler);
    test_add_to_deck(&tg, filler);
    int deck_before = test_deck_len(&tg);
    /* Play to the empty RightSide, not over the cost-7 member: no baton. */
    CHECK(test_play_to_stage(&tg, me, 2), "PL!S-PR-045-PR plays to an empty area");
    CHECK(!pending(&tg), "a non-baton debut raises no choice");
    CHECK_EQ(test_deck_len(&tg), deck_before, "a non-baton debut does not shrink the deck");
}

static void test_pr045_baton_hand_size_is_net_unchanged(void)
{
    TestGame tg;
    test_game_new(&tg);
    int replaced = test_id(&tg, "PL!-sd1-007-SD");
    int filler   = test_id(&tg, FILLER);
    int me       = test_new_id(&tg, PR045);
    CHECK(replaced >= 0 && filler >= 0 && me >= 0, "hand-net fixtures resolve");
    if (replaced < 0 || filler < 0 || me < 0) return;

    set_stage(&tg, 0, 0, replaced);
    test_add_to_hand(&tg, me);
    /* A second hand card so the 「手札を1枚控え室に置く」 leg has a target even
       when the effect fires: without it the leg would auto-skip and the test
       could not tell "did not fire" from "fired but had nothing to discard". */
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 25);
    test_add_to_deck(&tg, filler);
    test_add_to_deck(&tg, filler);
    int hand_before = test_hand_len(&tg);   /* 2: PR-045 + one spares */
    CHECK(test_play_to_stage(&tg, me, 0), "PL!S-PR-045-PR arrives by baton");
    CHECK(pending(&tg), "the draw2 / discard1 compound raises its choice");
    answer(&tg, 0);
    /* -1 played, +2 drawn, -1 discarded = unchanged */
    CHECK_EQ(test_hand_len(&tg), hand_before,
             "draw 2 / discard 1 leaves the hand net unchanged across the baton");
}

/* ══════════════════════════════════════════════════════════════════════
 * D. debut_draw_two_blade_only_when_revived_from_discard_pl_s_bp6_006
 *    The 復活 context (came from the waitroom) is part of the condition.
 * ══════════════════════════════════════════════════════════════════════ */

#define YOSHIKO_006 "PL!S-bp6-006-R"
#define MARI_008    "PL!S-bp6-008-R"

static void test_yoshiko_006_from_hand_draws_two_no_blade(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, YOSHIKO_006);
    int filler  = test_id(&tg, FILLER);
    CHECK(yoshiko >= 0 && filler >= 0, "PL!S-bp6-006-R hand-play fixtures resolve");
    if (yoshiko < 0 || filler < 0) return;
    check_identity(yoshiko, YOSHIKO_006, "the 006 debut print");

    tg.state.p[0].hand.n = 0;
    test_add_to_hand(&tg, yoshiko);
    fill_deck_p1(&tg, filler, 40);
    test_give_energy(&tg, 20);
    CHECK(test_play_to_stage(&tg, yoshiko, 1), "PL!S-bp6-006-R plays from hand");
    drain_skip(&tg, 20);
    CHECK_EQ(test_hand_len(&tg), 2, "the unconditional 「2枚引く」 leg resolves");
    CHECK_EQ(test_get_blade_modifier(&tg, yoshiko), 0,
             "a hand arrival is not 復活 -> no blade rider");
}

static void test_yoshiko_006_revived_from_waitroom_gains_blade(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, YOSHIKO_006);
    int mari    = test_id(&tg, MARI_008);
    int filler  = test_id(&tg, FILLER);
    CHECK(yoshiko >= 0 && mari >= 0 && filler >= 0, "006 revival fixtures resolve");
    if (yoshiko < 0 || mari < 0 || filler < 0) return;
    check_identity(mari, MARI_008, "the 008 復活 print");
    check_identity(yoshiko, YOSHIKO_006, "the 006 debut print");

    tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0;
    set_stage(&tg, 0, 0, mari);
    test_add_to_discard(&tg, yoshiko);
    test_give_energy(&tg, 20);
    fill_deck_p1(&tg, filler, 40);

    test_activate_ability(&tg, mari);
    CHECK(pending(&tg), "the 008 activation opens the waitroom selection");
    CHECK_EQ(strcmp(pending_kind(&tg), "SelectCard"), 0,
             "008's 復活 move_cards is a SelectCard over the waitroom");
    const RbChoice *ch = pending_choice(&tg);
    CHECK(ch && !strcmp(ch->zone, "discard"),
          "008's revival selects out of the waitroom");
    int idx = -1;
    for (int i = 0; i < tg.state.p[0].discard.n; i++)
        if (tg.state.p[0].discard.cards[i] == yoshiko) { idx = i; break; }
    CHECK(idx >= 0, "PL!S-bp6-006-R is selectable in the waitroom");
    if (idx < 0) return;
    answer(&tg, idx);
    drain_skip(&tg, 20);

    CHECK(wait_has(&tg, mari), "008 paid its own cost into the waitroom");
    CHECK(p1_stage_has(&tg, yoshiko), "006 lands on stage through the 復活");
    CHECK_EQ(test_hand_len(&tg), 2, "the revived 006 still draws 2");
    CHECK_EQ(test_get_blade_modifier(&tg, yoshiko), 3,
             "arriving from the waitroom satisfies the 復活 condition -> +3 blade");
}

/* ══════════════════════════════════════════════════════════════════════
 * E. debut_choose_low_blade_group_wait_immunity_or_position_change
 *    PL!S-bp7-003-R＋ choose-1: option 1 is a wait-immunity rider,
 *    option 2 is a position change. Only option 1 may block an opponent wait.
 * ══════════════════════════════════════════════════════════════════════ */

#define KANAN "PL!S-bp7-003-R＋"
#define KARIN "PL!N-bp7-004-R"
#define ENERGY_CARD "LL-E-001-SD"

/* Play 松浦果南, answer the debut choose-1 with `opt` (1-based), return her id
   or -1 if the setup could not be driven to completion. */
static int play_kanan_choose(TestGame *tg, int opt)
{
    int kanan  = test_id(tg, KANAN);
    int filler = test_id(tg, FILLER);
    if (kanan < 0 || filler < 0) return -1;
    fill_deck_p1(tg, filler, 30);
    test_add_to_hand(tg, kanan);
    test_give_energy(tg, 30);
    if (!test_play_to_stage(tg, kanan, 1)) return -1;
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        if (!strcmp(pending_kind(tg), "SelectCard")) answer(tg, 0);
        else answer(tg, opt - 1);
    }
    return kanan;
}

/* P2 DEBUTS 園田海未, whose printed debut is 「相手のステージにいるコスト4以下の
   メンバー1人をウェイトにする」. 果南 prints cost 4, so without a rider the
   wait lands — which is what makes the option-1 arm meaningful instead of
   vacuously green. (An opponent 起動 with an energy-under cost was tried first
   and never reached the wait at all.) */
static void opponent_waits_with_sonoda(TestGame *tg)
{
    int umi = test_id(tg, "PL!-bp5-013-N");
    int e   = test_id(tg, ENERGY_CARD);
    if (umi < 0 || e < 0) return;
    check_identity(umi, "PL!-bp5-013-N", "the 園田海未 debut print");
    check_cost(umi, 9, "PL!-bp5-013-N");
    tg->state.active = 1;
    test_add_to_hand_for(tg, 1, umi);
    for (int i = 0; i < 10; i++) test_add_to_energy(tg, 1, e);
    tg->state.p[1].energy_active = 10;
    test_play_to_stage_for(tg, 1, umi, 1);
    settle_card_first(tg, 10);
}

static void test_kanan_option1_blocks_opponent_blade_limited_wait(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanan = test_id(&tg, KANAN);
    if (kanan < 0) { CHECK(0, "PL!S-bp7-003-R＋ resolves"); return; }
    check_identity(kanan, KANAN, "the 松浦果南 debut print");
    check_cost(kanan, 4, KANAN);
    check_card_printed_blade(kanan, 2);

    if (play_kanan_choose(&tg, 1) < 0) { CHECK(0, "果南 debut drives to its choose-1"); return; }
    CHECK(!is_waited(&tg, kanan), "sanity: 果南 is not waited before the opponent acts");
    opponent_waits_with_sonoda(&tg);
    CHECK(is_waited(&tg, kanan),
          "PROBE: without a rider 園田海未's cost-4 wait reaches 果南 "
          "(if this fails the option-1 arm below is vacuous)");
}

static void test_kanan_option1_blocks_opponent_wait(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanan = test_id(&tg, KANAN);
    if (kanan < 0) { CHECK(0, "PL!S-bp7-003-R＋ resolves"); return; }
    check_identity(kanan, KANAN, "the 松浦果南 debut print");
    check_cost(kanan, 4, KANAN);
    check_card_printed_blade(kanan, 2);

    if (play_kanan_choose(&tg, 1) < 0) { CHECK(0, "果南 debut drives to its choose-1"); return; }
    CHECK(!is_waited(&tg, kanan), "sanity: 果南 is not waited before the opponent acts");
    opponent_waits_with_sonoda(&tg);
    CHECK(!is_waited(&tg, kanan),
          "option 1 (wait immunity) blocks the opponent's cost-limited wait");
}

static void test_kanan_option2_does_not_block_opponent_wait(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanan = test_id(&tg, KANAN);
    if (kanan < 0) { CHECK(0, "PL!S-bp7-003-R＋ resolves"); return; }
    check_identity(kanan, KANAN, "the 松浦果南 debut print");

    if (play_kanan_choose(&tg, 2) < 0) { CHECK(0, "果南 debut drives to its choose-1"); return; }
    CHECK(!is_waited(&tg, kanan), "sanity: 果南 is not waited before the opponent acts");
    opponent_waits_with_sonoda(&tg);
    CHECK(is_waited(&tg, kanan),
          "option 2 (position change) grants no immunity, so the opponent wait lands");
}

static void test_kanan_self_wait_still_works(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanan = test_id(&tg, KANAN);
    if (kanan < 0) { CHECK(0, "PL!S-bp7-003-R＋ resolves"); return; }
    if (play_kanan_choose(&tg, 1) < 0) { CHECK(0, "果南 debut drives to its choose-1"); return; }
    /* The printed immunity is opponent-scoped; the owner may still wait themself. */
    rb_mods_set_orientation(&tg.state.mods, kanan, "wait");
    CHECK(is_waited(&tg, kanan), "the owner's own wait is not blocked by the rider");
}

/* ══════════════════════════════════════════════════════════════════════
 * F. discard_wait_enables_hearts_pl_hs_bp5_016_n_test.rs
 *    「登場: 自分の手札から1枚控え室に置いた後、相手のブレードがX以下の
 *    メンバーをウェイトにする。2体以上なら、ライブ終了時までheart06を得る。」
 * ══════════════════════════════════════════════════════════════════════ */

#define HS_BP5_016 "PL!HS-bp5-016-N"

static void test_hs_bp5_016_n_waits_two_cheap_and_grants_heart06(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, HS_BP5_016);
    int filler = test_id(&tg, FILLER);
    int discard_target = test_id(&tg, "PL!-sd1-019-SD");
    int victim_a = test_id(&tg, "PL!S-bp2-002-R");
    int victim_b = test_id(&tg, FILLER);
    int pricey   = test_id(&tg, "PL!N-bp7-003-R＋");
    CHECK(card >= 0 && filler >= 0 && discard_target >= 0 &&
          victim_a >= 0 && victim_b >= 0 && pricey >= 0,
          "PL!HS-bp5-016-N fixtures resolve");
    if (card < 0 || filler < 0 || discard_target < 0 ||
        victim_a < 0 || victim_b < 0 || pricey < 0) return;
    CHECK(victim_a != victim_b && victim_a != pricey && victim_b != pricey,
          "the three opponent members are three DISTINCT instances");
    check_identity(card, HS_BP5_016, "the 蓮ノ空 debut print");
    check_cost(card, 9, HS_BP5_016);
    check_cost(victim_a, 4, "PL!S-bp2-002-R (blade-eligible)");
    check_cost(pricey, 15, "PL!N-bp7-003-R＋ (over the blade limit)");

    test_give_energy(&tg, 15);
    fill_decks(&tg, filler, 30);
    set_stage(&tg, 1, 0, victim_a);
    set_stage(&tg, 1, 1, victim_b);
    set_stage(&tg, 1, 2, pricey);
    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, discard_target);

    CHECK(test_play_to_stage(&tg, card, 1), "PL!HS-bp5-016-N plays to the centre");
    answer(&tg, 0);
    CHECK(pending(&tg), "the opponent-wait selection is offered");
    test_select_indices(&tg, (const int[]){0, 1}, 2);
    CHECK(!pending(&tg), "the compound resolves after the wait picks");

    CHECK(!hand_has(&tg, discard_target), "the hand card left the hand");
    CHECK(wait_has(&tg, discard_target), "the hand card reached the waitroom");
    CHECK(is_waited(&tg, victim_a), "the cost-4 victim is waited");
    CHECK(is_waited(&tg, victim_b), "the cost-4 victim is waited");
    CHECK(!is_waited(&tg, pricey), "the cost-15 victim is above the blade limit and stays");
    CHECK(p2_stage_has(&tg, victim_a) && p2_stage_has(&tg, victim_b) && p2_stage_has(&tg, pricey),
          "a wait changes the orientation, not the stage occupancy");
    CHECK_EQ(heart_of(&tg, card, 6), 1,
             "two or more waited members unlock the heart06 rider");
    CHECK_EQ(heart_of(&tg, victim_a, 6), 0, "the rider only touches the owner");
    CHECK_EQ(heart_of(&tg, pricey, 6), 0, "the over-limit member gets nothing");
}

static void test_hs_bp5_016_n_single_victim_grants_no_heart06(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, HS_BP5_016);
    int filler = test_id(&tg, FILLER);
    int discard_target = test_id(&tg, "PL!-sd1-019-SD");
    int victim = test_id(&tg, "PL!S-bp2-002-R");
    int pricey  = test_id(&tg, "PL!N-bp7-003-R＋");
    CHECK(card >= 0 && filler >= 0 && discard_target >= 0 && victim >= 0 && pricey >= 0,
          "PL!HS-bp5-016-N single-victim fixtures resolve");
    if (card < 0 || filler < 0 || discard_target < 0 || victim < 0 || pricey < 0) return;
    check_identity(card, HS_BP5_016, "the 蓮ノ空 debut print");

    test_give_energy(&tg, 15);
    fill_decks(&tg, filler, 30);
    set_stage(&tg, 1, 0, victim);
    set_stage(&tg, 1, 1, pricey);
    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, discard_target);

    CHECK(test_play_to_stage(&tg, card, 1), "PL!HS-bp5-016-N plays to the centre");
    answer(&tg, 0);
    CHECK(pending(&tg), "the opponent-wait selection is offered");
    answer(&tg, 0);
    CHECK(!pending(&tg), "the compound resolves after the single wait pick");

    CHECK(is_waited(&tg, victim), "the eligible victim is waited");
    CHECK(!is_waited(&tg, pricey), "the over-limit member stays active");
    CHECK_EQ(heart_of(&tg, card, 6), 0,
             "ONE waited member is below the printed 「2体以上」 threshold");
}

static void test_hs_bp5_016_n_waits_at_most_two_of_three(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, HS_BP5_016);
    int filler = test_id(&tg, FILLER);
    int discard_target = test_id(&tg, "PL!-sd1-019-SD");
    int victim_a = test_id(&tg, "PL!S-bp2-002-R");
    int victim_b = test_new_id(&tg, "PL!S-bp2-002-R");
    int victim_c = test_id(&tg, FILLER);
    CHECK(card >= 0 && filler >= 0 && discard_target >= 0 &&
          victim_a >= 0 && victim_b >= 0 && victim_c >= 0,
          "PL!HS-bp5-016-N three-eligible fixtures resolve");
    if (card < 0 || filler < 0 || discard_target < 0 ||
        victim_a < 0 || victim_b < 0 || victim_c < 0) return;
    CHECK(victim_a != victim_b, "the two victim copies are distinct instances");

    test_give_energy(&tg, 15);
    fill_decks(&tg, filler, 30);
    set_stage(&tg, 1, 0, victim_a);
    set_stage(&tg, 1, 1, victim_b);
    set_stage(&tg, 1, 2, victim_c);
    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, discard_target);

    CHECK(test_play_to_stage(&tg, card, 1), "PL!HS-bp5-016-N plays to the centre");
    answer(&tg, 0);
    CHECK(pending(&tg), "the opponent-wait selection is offered");
    test_select_indices(&tg, (const int[]){0, 1}, 2);
    CHECK(!pending(&tg), "the compound resolves after the two wait picks");

    CHECK(is_waited(&tg, victim_a), "the first pick is waited");
    CHECK(is_waited(&tg, victim_b), "the second pick is waited");
    CHECK(!is_waited(&tg, victim_c), "the third eligible member is NOT waited (the effect is 2)");
    CHECK_EQ(heart_of(&tg, card, 6), 1, "two waits still unlock the heart06 rider");
    CHECK_EQ(heart_of(&tg, victim_c, 6), 0, "the untouched member gets no rider");
}

static void test_hs_bp5_016_n_decline_grants_no_heart06(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, HS_BP5_016);
    int filler = test_id(&tg, FILLER);
    CHECK(card >= 0 && filler >= 0, "PL!HS-bp5-016-N decline fixtures resolve");
    if (card < 0 || filler < 0) return;
    check_identity(card, HS_BP5_016, "the 蓮ノ空 debut print");

    test_give_energy(&tg, 15);
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, card);
    CHECK(test_play_to_stage(&tg, card, 1), "PL!HS-bp5-016-N plays to the centre");
    drain_skip(&tg, 20);
    CHECK_EQ(heart_of(&tg, card, 6), 0,
             "declining the discard/wait compound leaves no waited opponent -> no rider");
}

/* ══════════════════════════════════════════════════════════════════════
 * G. kaleidoscore_trio_heart_blade_pl_sp_bp7_013_n_test.rs
 *    「常時: 自分のステージにKALEIDOSCOREのメンバーが3体以上なら、
 *    ブレード+1、heart06を得る」
 * ══════════════════════════════════════════════════════════════════════ */

#define KOKO "PL!SP-bp7-013-N"

static void test_kaleidoscore_trio_grants_heart06_and_blade(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_new_id(&tg, FILLER);
    int koko = test_id(&tg, KOKO);
    int k2   = test_id(&tg, "PL!SP-bp1-013-PR");
    int k3   = test_id(&tg, "PL!SP-PR-012-PR");
    CHECK(koko >= 0 && k2 >= 0 && k3 >= 0, "KALEIDOSCORE trio fixtures resolve");
    if (koko < 0 || k2 < 0 || k3 < 0) return;
    check_identity(koko, KOKO, "the こここ constant print");
    check_identity(k2, "PL!SP-bp1-013-PR", "the second KALEIDOSCORE member");
    check_identity(k3, "PL!SP-PR-012-PR", "the third KALEIDOSCORE member");
    CHECK(rb_card_matches_group_str(koko, "KALEIDOSCORE"), "PL!SP-bp7-013-N is KALEIDOSCORE");
    CHECK(rb_card_matches_group_str(k2, "KALEIDOSCORE"), "PL!SP-bp1-013-PR is KALEIDOSCORE");
    CHECK(rb_card_matches_group_str(k3, "KALEIDOSCORE"), "PL!SP-PR-012-PR is KALEIDOSCORE");

    fill_decks(&tg, filler, 30);
    set_stage(&tg, 0, 0, koko);
    set_stage(&tg, 0, 1, k2);
    set_stage(&tg, 0, 2, k3);
    CHECK_EQ(test_zone_len(&tg, 0, "stage"), 3, "setup: three members really are on stage");
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, koko, 6), 1, "three KALEIDOSCORE members -> heart06 granted");
    CHECK_EQ(test_get_blade_modifier(&tg, koko), 1, "three KALEIDOSCORE members -> blade granted");
}

static void test_kaleidoscore_two_grants_nothing(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_new_id(&tg, FILLER);
    int koko = test_id(&tg, KOKO);
    int k2   = test_id(&tg, "PL!SP-bp1-013-PR");
    int outsider = test_id(&tg, FILLER);
    CHECK(koko >= 0 && k2 >= 0 && outsider >= 0, "KALEIDOSCORE pair fixtures resolve");
    if (koko < 0 || k2 < 0 || outsider < 0) return;
    check_identity(koko, KOKO, "the こここ constant print");
    CHECK(!rb_card_matches_group_str(outsider, "KALEIDOSCORE"),
          "the third slot really is an outsider");

    fill_decks(&tg, filler, 30);
    set_stage(&tg, 0, 0, koko);
    set_stage(&tg, 0, 1, k2);
    set_stage(&tg, 0, 2, outsider);
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, koko, 6), 0, "only two KALEIDOSCORE -> no heart06");
    CHECK_EQ(test_get_blade_modifier(&tg, koko), 0, "only two KALEIDOSCORE -> no blade");

    /* Drop to a single KALEIDOSCORE member: the printed threshold is 3, so a
       second sub-threshold datapoint pins whether the count is enforced at all
       or simply ignored. */
    set_stage(&tg, 0, 1, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(test_get_blade_modifier(&tg, koko), 0,
             "a single KALEIDOSCORE member is also below the printed 「3人」 threshold");
}

/* ══════════════════════════════════════════════════════════════════════
 * H. live_start_bottom_three_mill_all_group_heart_test.rs
 *    「自分のデッキの下から3枚控え室に置く。それらがすべて『Aqours』の
 *    メンバーカードの場合、heart04を得る」 — the SOURCE is deck_bottom.
 * ══════════════════════════════════════════════════════════════════════ */

static void test_live_start_bottom_three_mills_the_bottom_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, "PL!S-bp7-006-R");
    int filler  = test_id(&tg, FILLER);
    int live    = test_id(&tg, "PL!-sd1-020-SD");
    int top_marker = test_id(&tg, "PL!S-bp7-006-R");
    int b1 = test_id(&tg, "PL!SP-sd1-001-SD");
    int b2 = test_id(&tg, "PL!SP-sd1-003-SD");
    int b3 = test_id(&tg, "PL!SP-sd1-004-SD");
    CHECK(yoshiko >= 0 && filler >= 0 && live >= 0 &&
          top_marker >= 0 && b1 >= 0 && b2 >= 0 && b3 >= 0,
          "PL!S-bp7-006-R bottom-three fixtures resolve");
    if (yoshiko < 0 || filler < 0 || live < 0 ||
        top_marker < 0 || b1 < 0 || b2 < 0 || b3 < 0) return;
    check_identity(yoshiko, "PL!S-bp7-006-R", "the 津島善子 live-start print");
    CHECK(b1 != b2 && b2 != b3 && b1 != b3, "the three bottom markers are distinct");

    set_stage(&tg, 0, 1, yoshiko);
    test_give_energy(&tg, 3);
    test_add_to_hand(&tg, live);
    advance_to_live_card_set(&tg, 5);
    /* Seed AFTER the live-card-set draws so the bottom markers stay at the end. */
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = top_marker;
    for (int i = 0; i < 10; i++) P->deck.cards[P->deck.n++] = filler;
    P->deck.cards[P->deck.n++] = b1;
    P->deck.cards[P->deck.n++] = b2;
    P->deck.cards[P->deck.n++] = b3;
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg, 2);
    drain_skip(&tg, 20);

    CHECK(!wait_has(&tg, top_marker), "the TOP deck card is NOT discarded (source is deck_bottom)");
    CHECK(wait_has(&tg, b1), "the bottom-most marker reached the waitroom");
    CHECK(wait_has(&tg, b2), "the middle bottom marker reached the waitroom");
    CHECK(wait_has(&tg, b3), "the third bottom marker reached the waitroom");
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 3, "exactly three cards are discarded");
}

static void test_live_start_bottom_three_all_aqours_gives_heart04(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, "PL!S-bp7-006-R");
    int filler  = test_id(&tg, FILLER);
    int live    = test_id(&tg, "PL!-sd1-020-SD");
    int a1 = test_id(&tg, "PL!S-bp7-011-N");
    int a2 = test_id(&tg, "PL!S-bp7-015-N");
    int a3 = test_id(&tg, "PL!S-bp7-017-N");
    CHECK(yoshiko >= 0 && filler >= 0 && live >= 0 && a1 >= 0 && a2 >= 0 && a3 >= 0,
          "PL!S-bp7-006-R all-Aqours fixtures resolve");
    if (yoshiko < 0 || filler < 0 || live < 0 || a1 < 0 || a2 < 0 || a3 < 0) return;
    check_identity(yoshiko, "PL!S-bp7-006-R", "the 津島善子 live-start print");
    CHECK(rb_card_matches_group_str(a1, "Aqours") &&
          rb_card_matches_group_str(a2, "Aqours") &&
          rb_card_matches_group_str(a3, "Aqours"),
          "the three bottom markers are all Aqours member cards");

    set_stage(&tg, 0, 1, yoshiko);
    test_give_energy(&tg, 3);
    test_add_to_hand(&tg, live);
    advance_to_live_card_set(&tg, 5);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < 10; i++) P->deck.cards[P->deck.n++] = filler;
    P->deck.cards[P->deck.n++] = a1;
    P->deck.cards[P->deck.n++] = a2;
    P->deck.cards[P->deck.n++] = a3;
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg, 2);
    drain_skip(&tg, 20);

    CHECK(wait_has(&tg, a1) && wait_has(&tg, a2) && wait_has(&tg, a3),
          "the three Aqours cards really were discarded");
    CHECK(heart_of(&tg, yoshiko, 4) > 0,
          "all three discarded cards are Aqours members -> the heart04 rider applies");
}

/* ══════════════════════════════════════════════════════════════════════
 * I. live_start_mill_all_group_blade_test.rs
 *    PL!HS-bp6-009-R mills 4 from the deck top; if ALL are 蓮ノ空 members it
 *    gains +1 blade until the end of the live.
 * ══════════════════════════════════════════════════════════════════════ */

#define HS_BP6_009 "PL!HS-bp6-009-R"

static int mill_group_setup(TestGame *tg, int wrong_index, int *member)
{
    *member = test_id(tg, HS_BP6_009);
    int filler   = test_id(tg, FILLER);
    int in_group = test_id(tg, "PL!HS-bp6-010-R");
    int live_in_group = test_id(tg, "PL!HS-bp6-028-L");
    int waiting  = test_id(tg, FILLER);
    int opponent = test_id(tg, FILLER);
    if (*member < 0 || filler < 0 || in_group < 0 || live_in_group < 0 ||
        waiting < 0 || opponent < 0) return 0;
    set_stage(tg, 0, 1, *member);
    int cards[5];
    for (int i = 0; i < 5; i++) {
        const char *no = (wrong_index == i || i == 4) ? FILLER
                        : (i == 1 ? "PL!HS-bp6-028-L" : "PL!HS-bp6-010-R");
        cards[i] = test_id(tg, no);
        if (cards[i] < 0) return 0;
    }
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = cards[i];
    test_add_to_discard(tg, waiting);
    test_add_to_deck_pl(tg, 1, opponent);
    tg->state.phase = RB_PHASE_PERFORMANCE;
    CHECK(fire_trigger(tg, *member, RB_TSTR_LIVE_START),
          "PL!HS-bp6-009-R prints a ライブ開始時 ability and the trigger is queued");
    CHECK(!pending(tg), "the 蓮ノ空 mill resolves with no prompt");
    CHECK_EQ(test_deck_len(tg), 1, "exactly four cards left the deck");
    CHECK_EQ(test_zone_len(tg, 0, "discard"), 5,
             "the pre-existing waitroom card plus the four milled cards");
    CHECK_EQ(test_hand_len(tg), 0, "the mill never reaches the hand");
    return 1;
}

static void test_mill_four_all_group_cards_grants_one_blade_until_live_end(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member = -1;
    if (!mill_group_setup(&tg, -1, &member)) return;
    check_identity(member, HS_BP6_009, "the 蓮ノ空 live-start print");
    CHECK_EQ(test_get_blade_modifier(&tg, member), 1,
             "all four milled cards are 蓮ノ空 members -> +1 blade");
    /* Rust: check_expired_effects() while still in the live leaves it up. */
    rb_check_expired_effects(&tg.state, 2 /* turn-end only */);
    CHECK_EQ(test_get_blade_modifier(&tg, member), 1,
             "a ライブ終了時 blade is not reaped by a turn-end expiry sweep");
    /* Rust: current_turn_phase = FirstAttackerNormal then check_expired_effects */
    tg.state.phase = RB_PHASE_MAIN;
    rb_check_expired_effects(&tg.state, 1 /* live end */);
    CHECK_EQ(test_get_blade_modifier(&tg, member), 0,
             "the live-end expiry reaps the rider");
    CHECK_EQ(tg.state.n_temp_effects, 0, "no temporary effect outlives the live");
}

static void test_mill_any_wrong_group_card_prevents_blade(void)
{
    for (int wrong = 0; wrong < 4; wrong++) {
        TestGame tg;
        test_game_new(&tg);
        int member = -1;
        if (!mill_group_setup(&tg, wrong, &member)) return;
        char msg[128];
        snprintf(msg, sizeof msg,
                 "milling slot %d as a non-蓮ノ空 card blocks the blade rider", wrong);
        CHECK_EQ(test_get_blade_modifier(&tg, member), 0, msg);
        CHECK_EQ(tg.state.n_temp_effects, 0,
                 "the blocked rider registers no temporary effect");
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * J. success_score_required_heart_and_score_pl_bp4_021_l_test.rs
 *    PL!-bp4-021-L: success-zone total >= 6 relaxes 必要ハート heart0 by 1;
 *    total >= 9 adds +1 score. Two independent thresholds.
 * ══════════════════════════════════════════════════════════════════════ */

static void bp4_021_run(const int *cards, int n, int *out_need, int *out_score)
{
    TestGame tg;
    test_game_new(&tg);
    int hb = test_id(&tg, "PL!-bp4-021-L");
    if (hb < 0) return;
    check_identity(hb, "PL!-bp4-021-L", "the PL!-bp4-021-L live-start print");
    test_add_to_live(&tg, hb);
    for (int i = 0; i < n; i++) test_add_to_success(&tg, cards[i]);
    fire_trigger(&tg, hb, RB_TSTR_LIVE_START);
    drain_skip(&tg, 20);
    *out_need   = rb_mods_get_need_heart(&tg.state.mods, hb, 0);
    *out_score  = test_get_score_modifier(&tg, hb);
}

static void test_bp4_021_l_score_thresholds(void)
{
    int s1a = rb_find_card_by_no(SCORE1);
    int s1b = rb_create_card_copy(s1a);
    int s1c = rb_create_card_copy(s1a);
    int s6  = rb_find_card_by_no(SCORE6);
    int s5  = rb_find_card_by_no("PL!S-PR-024-PR");
    CHECK(s1a >= 0 && s1b >= 0 && s1c >= 0 && s6 >= 0 && s5 >= 0,
          "PL!-bp4-021-L success-score fixtures resolve");
    if (s1a < 0 || s1b < 0 || s1c < 0 || s6 < 0 || s5 < 0) return;
    CHECK_EQ(card_score(s1a), 1, "setup: the score-1 success live");
    CHECK_EQ(card_score(s6), 6, "setup: the score-6 success live");
    CHECK_EQ(card_score(s5), 5, "setup: the score-5 success live");

    int need = 0, score = 0;
    int run1[3] = { s1a, s1b, s1c };
    bp4_021_run(run1, 3, &need, &score);
    CHECK_EQ(need, 0, "success total 3 (< 6) -> no 必要ハート relaxation");
    CHECK_EQ(score, 0, "success total 3 (< 6) -> no score bonus");

    int run2[1] = { s6 };
    bp4_021_run(run2, 1, &need, &score);
    CHECK_EQ(need, -1, "success total 6 (>= 6) -> 必要ハート heart0 is relaxed by 1");
    CHECK_EQ(score, 0, "success total 6 (< 9) -> no score bonus yet");

    int run3[2] = { s6, s5 };
    bp4_021_run(run3, 2, &need, &score);
    CHECK_EQ(need, -1, "success total 11 keeps the heart0 relaxation");
    CHECK_EQ(score, 1, "success total 11 (>= 9) -> +1 score");
}

/* ══════════════════════════════════════════════════════════════════════
 * K. live_start_select_success_zone_live_and_gain_hearts_pl_s_bp6_004_r
 *    「ライブ開始時: 自分のライブカード置場が2枚以上なら、1枚を選んで
 *    デッキの上に置き、heart02を得る」 — a card_count condition gates the pick.
 * ══════════════════════════════════════════════════════════════════════ */

#define KUROE "PL!S-bp6-004-R＋"
#define PLAIN_LIVE "PL!S-bp6-021-L"
#define LIVE_START_L1 "PL!S-bp6-019-L"
#define LIVE_START_L2 "PL!S-bp6-020-L"

static void trigger_kuroe_live_start(TestGame *tg, int kuroe)
{
    tg->state.activating_card = kuroe;
    if (rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_LIVE_START) > 0)
        rb_process_pending_auto_abilities(&tg->state);
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        if (!strcmp(pending_kind(tg), "SelectAutoAbility")) answer(tg, -1);
        else break;
    }
}

static void test_kuroe_two_live_cards_offers_the_pick(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kuroe = test_id(&tg, KUROE);
    int la = test_id(&tg, PLAIN_LIVE);
    int lb = test_new_id(&tg, PLAIN_LIVE);
    CHECK(kuroe >= 0 && la >= 0 && lb >= 0, "PL!S-bp6-004-R＋ two-live fixtures resolve");
    if (kuroe < 0 || la < 0 || lb < 0) return;
    check_identity(kuroe, KUROE, "the 黒江 debut print");
    CHECK(la != lb, "the two live cards are two DISTINCT instances");
    CHECK(!card_prints_trigger(la, RB_TSTR_LIVE_START),
          "PL!S-bp6-021-L prints no ライブ開始時 ability (it is selectable)");

    test_add_to_live(&tg, la);
    test_add_to_live(&tg, lb);
    test_add_to_hand(&tg, kuroe);
    trigger_kuroe_live_start(&tg, kuroe);
    CHECK(pending(&tg), "two cards in the live zone satisfy the 「2枚以上」 condition");
}

static void test_kuroe_one_live_card_no_pick(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kuroe = test_id(&tg, KUROE);
    int live = test_id(&tg, PLAIN_LIVE);
    CHECK(kuroe >= 0 && live >= 0, "PL!S-bp6-004-R＋ one-live fixtures resolve");
    if (kuroe < 0 || live < 0) return;
    check_identity(kuroe, KUROE, "the 黒江 debut print");

    test_add_to_live(&tg, live);
    test_add_to_hand(&tg, kuroe);
    trigger_kuroe_live_start(&tg, kuroe);
    CHECK(!pending(&tg), "one live card is below the 「2枚以上」 threshold -> no pick");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1, "the live card stays put");
}

static void test_kuroe_live_start_cards_are_excluded(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kuroe = test_id(&tg, KUROE);
    int ls1 = test_id(&tg, LIVE_START_L1);
    int ls2 = test_id(&tg, LIVE_START_L2);
    CHECK(kuroe >= 0 && ls1 >= 0 && ls2 >= 0, "PL!S-bp6-004-R＋ exclusion fixtures resolve");
    if (kuroe < 0 || ls1 < 0 || ls2 < 0) return;
    check_identity(kuroe, KUROE, "the 黒江 debut print");
    CHECK(card_prints_trigger(ls1, RB_TSTR_LIVE_START) &&
          card_prints_trigger(ls2, RB_TSTR_LIVE_START),
          "both staged live cards really print ライブ開始時");

    test_add_to_live(&tg, ls1);
    test_add_to_live(&tg, ls2);
    test_add_to_hand(&tg, kuroe);
    trigger_kuroe_live_start(&tg, kuroe);
    CHECK(!pending(&tg), "the ability filter excludes every ライブ開始時 live card");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 2, "both excluded cards remain in the live zone");
}

static void test_kuroe_selected_live_goes_to_deck_top(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kuroe = test_id(&tg, KUROE);
    int selectable = test_id(&tg, PLAIN_LIVE);
    int other = test_new_id(&tg, PLAIN_LIVE);
    CHECK(kuroe >= 0 && selectable >= 0 && other >= 0, "PL!S-bp6-004-R＋ select fixtures resolve");
    if (kuroe < 0 || selectable < 0 || other < 0) return;
    check_identity(kuroe, KUROE, "the 黒江 debut print");

    test_add_to_live(&tg, selectable);
    test_add_to_live(&tg, other);
    test_add_to_hand(&tg, kuroe);
    trigger_kuroe_live_start(&tg, kuroe);
    CHECK(pending(&tg), "the pick is offered over the two selectable live cards");
    answer(&tg, 0);

    CHECK_EQ(test_deck_len(&tg), 1, "the selected card lands on the deck");
    if (test_deck_len(&tg) >= 1)
        CHECK_EQ(tg.state.p[0].deck.cards[0], selectable, "the SELECTED card is on deck top");
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 1, "one card remains in the live zone");
    CHECK(!bag_has(&tg.state.p[0].live, tg.state.p[0].deck.cards[0]),
          "the deck-top card is no longer in the live zone");
}

static void test_kuroe_skip_keeps_the_live_zone(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kuroe = test_id(&tg, KUROE);
    int la = test_id(&tg, PLAIN_LIVE);
    int lb = test_new_id(&tg, PLAIN_LIVE);
    CHECK(kuroe >= 0 && la >= 0 && lb >= 0, "PL!S-bp6-004-R＋ skip fixtures resolve");
    if (kuroe < 0 || la < 0 || lb < 0) return;
    check_identity(kuroe, KUROE, "the 黒江 debut print");

    test_add_to_live(&tg, la);
    test_add_to_live(&tg, lb);
    test_add_to_hand(&tg, kuroe);
    trigger_kuroe_live_start(&tg, kuroe);
    CHECK(pending(&tg), "the pick is offered");
    answer(&tg, -1);
    CHECK_EQ(test_zone_len(&tg, 0, "live"), 2, "declining leaves both cards in the live zone");
    CHECK_EQ(test_deck_len(&tg), 0, "declining puts nothing on the deck");
}

/* ══════════════════════════════════════════════════════════════════════
 * L. live_start_success_count_score_and_required_hearts_q254_test.rs
 *    Q254: a live-start ability whose condition is MET is MANDATORY — no skip
 *    gate — and SETS a different required-heart profile plus +5 score.
 * ══════════════════════════════════════════════════════════════════════ */

#define SD2_023 "PL!SP-sd2-023-SD2"

static void q254_run(int n_success, int stage_members)
{
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, SD2_023);
    if (live < 0) { CHECK(0, "PL!SP-sd2-023-SD2 resolves"); return; }
    check_identity(live, SD2_023, "the Q254 live-start print");
    static const char *const past[] = { "PL!-sd1-020-SD", "PL!-sd1-021-SD", "PL!-sd1-022-SD" };
    for (int i = 0; i < n_success && i < 3; i++) test_add_to_success(&tg, test_id(&tg, past[i]));
    for (int i = 0; i < stage_members; i++) {
        int c = test_id(&tg, FILLER);
        if (c >= 0) set_stage(&tg, 0, i, c);
    }
    test_add_to_hand(&tg, live);
    fill_decks(&tg, test_id(&tg, FILLER), 20);

    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg, 2);
    drain_skip(&tg, 30);

    char msg[160];
    snprintf(msg, sizeof msg,
             "setup: %d seeded success-zone card(s) are still present at live start",
             n_success);
    CHECK(test_zone_len(&tg, 0, "success") >= n_success, msg);
    if (n_success >= 2) {
        CHECK(!pending(&tg), "Q254: a met live-start condition is mandatory (no skip gate)");
        snprintf(msg, sizeof msg, "Q254: %d success-zone cards -> +5 score", n_success);
        CHECK_EQ(test_get_score_modifier(&tg, live), 5, msg);
        snprintf(msg, sizeof msg, "Q254: %d success-zone cards -> 必要ハート heart02 = 3", n_success);
        CHECK_EQ(need_heart_of(&tg, live, 2), 3, msg);
        snprintf(msg, sizeof msg, "Q254: %d success-zone cards -> 必要ハート heart03 = 3", n_success);
        CHECK_EQ(need_heart_of(&tg, live, 3), 3, msg);
        snprintf(msg, sizeof msg, "Q254: %d success-zone cards -> 必要ハート heart06 = 3", n_success);
        CHECK_EQ(need_heart_of(&tg, live, 6), 3, msg);
        snprintf(msg, sizeof msg, "Q254: %d success-zone cards -> 必要ハート heart0  = 3", n_success);
        CHECK_EQ(need_heart_of(&tg, live, 0), 3, msg);
    } else {
        snprintf(msg, sizeof msg, "Q254: %d success-zone card(s) -> no score rider", n_success);
        CHECK_EQ(test_get_score_modifier(&tg, live), 0, msg);
        snprintf(msg, sizeof msg, "Q254: %d success-zone card(s) -> 必要ハート untouched", n_success);
        CHECK_EQ(need_heart_of(&tg, live, 2), 0, msg);
        CHECK_EQ(need_heart_of(&tg, live, 3), 0, msg);
        CHECK_EQ(need_heart_of(&tg, live, 6), 0, msg);
        CHECK_EQ(need_heart_of(&tg, live, 0), 0, msg);
    }
}

static void test_q254_success_count_thresholds(void)
{
    q254_run(0, 0);
    q254_run(1, 0);
    q254_run(2, 0);
    q254_run(3, 0);
}

static void test_q254_p_variant_same_mandatory_threshold(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-sd2-023-P");
    if (live < 0) { CHECK(0, "PL!SP-sd2-023-P resolves"); return; }
    check_identity(live, "PL!SP-sd2-023-P", "the Q254 P-variant print");
    test_add_to_success(&tg, test_id(&tg, "PL!-sd1-020-SD"));
    test_add_to_success(&tg, test_id(&tg, "PL!-sd1-021-SD"));
    test_add_to_hand(&tg, live);
    fill_decks(&tg, test_id(&tg, FILLER), 20);
    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg, 2);
    drain_skip(&tg, 30);
    CHECK(!pending(&tg), "Q254 P variant: the met condition is mandatory");
    CHECK_EQ(test_get_score_modifier(&tg, live), 5, "Q254 P variant: +5 score");
    CHECK_EQ(need_heart_of(&tg, live, 0), 3, "Q254 P variant: 必要ハート heart0 = 3");
}

/* ══════════════════════════════════════════════════════════════════════
 * M. live_start_waited_opponent_count_group_member_topdeck_test.rs
 *    PL!N-bp4-004-R＋: the second live-start ability puts up to (waited
 *    opponent members) 虹ヶ咲 cards from the waitroom back on the deck top.
 *    The dynamic count is the CONDITION, so zero waited members must place
 *    nothing at all.
 * ══════════════════════════════════════════════════════════════════════ */

#define KARIN_BP4 "PL!N-bp4-004-R＋"
#define NIJIGASAKI "PL!N-bp1-012-R＋"

static void trigger_live_start_all_on_stage(TestGame *tg)
{
    for (int area = 0; area < RB_STAGE_SIZE; area++) {
        int cid = tg->state.p[0].stage[area];
        if (cid == RB_EMPTY_SLOT) continue;
        if (!card_prints_trigger(cid, RB_TSTR_LIVE_START)) continue;
        tg->state.activating_card = cid;
        if (rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_LIVE_START) > 0)
            rb_process_pending_auto_abilities(&tg->state);
    }
}

static void test_karin_topdeck_with_two_waited_opponents(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = test_id(&tg, KARIN_BP4);
    int niji1 = test_id(&tg, NIJIGASAKI);
    int niji2 = test_new_id(&tg, NIJIGASAKI);
    int filler = test_id(&tg, FILLER);
    int opp1 = test_id(&tg, FILLER);
    int opp2 = test_new_id(&tg, FILLER);
    CHECK(karin >= 0 && niji1 >= 0 && niji2 >= 0 && filler >= 0 && opp1 >= 0 && opp2 >= 0,
          "PL!N-bp4-004-R＋ fixtures resolve");
    if (karin < 0 || niji1 < 0 || niji2 < 0 || filler < 0 || opp1 < 0 || opp2 < 0) return;
    check_identity(karin, KARIN_BP4, "the 果林 live-start print");
    CHECK(rb_card_matches_group_str(niji1, "R3BIRTH"),
          "the waitroom members are 虹ヶ咲 (R3BIRTH) prints");
    CHECK(niji1 != niji2, "the two 虹ヶ咲 members are distinct instances");

    set_stage(&tg, 0, 0, karin);
    set_stage(&tg, 1, 0, opp1);
    set_stage(&tg, 1, 1, opp2);
    rb_mods_set_orientation(&tg.state.mods, opp1, "wait");
    rb_mods_set_orientation(&tg.state.mods, opp2, "wait");
    test_add_to_discard(&tg, niji1);
    test_add_to_discard(&tg, niji2);
    test_add_to_discard(&tg, filler);          /* non-虹ヶ咲, must be ignored */
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);

    int deck_before = test_deck_len(&tg);
    trigger_live_start_all_on_stage(&tg);
    drain_auto_then_first(&tg, 30);
    int deck_after = test_deck_len(&tg);
    CHECK(deck_after > deck_before,
          "two waited opponent members -> 虹ヶ咲 cards go back to the deck top");
}

static void test_karin_topdeck_with_no_waited_opponents_places_nothing(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = test_id(&tg, KARIN_BP4);
    int niji = test_id(&tg, NIJIGASAKI);
    int filler = test_id(&tg, FILLER);
    int opp = test_id(&tg, FILLER);
    CHECK(karin >= 0 && niji >= 0 && filler >= 0 && opp >= 0,
          "PL!N-bp4-004-R＋ zero-wait fixtures resolve");
    if (karin < 0 || niji < 0 || filler < 0 || opp < 0) return;
    check_identity(karin, KARIN_BP4, "the 果林 live-start print");

    set_stage(&tg, 0, 0, karin);
    set_stage(&tg, 1, 0, opp);
    test_add_to_discard(&tg, niji);
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);

    int deck_before = test_deck_len(&tg);
    trigger_live_start_all_on_stage(&tg);
    drain_auto_then_first(&tg, 30);
    CHECK_EQ(test_deck_len(&tg), deck_before,
             "with zero waited opponents the dynamic count is 0 and nothing is placed");
    CHECK(wait_has(&tg, niji), "the 虹ヶ咲 member stays in the waitroom");
}

static void test_karin_topdeck_loses_at_most_the_draw(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = test_id(&tg, KARIN_BP4);
    int niji = test_id(&tg, NIJIGASAKI);
    int filler = test_id(&tg, FILLER);
    CHECK(karin >= 0 && niji >= 0 && filler >= 0, "PL!N-bp4-004-R＋ draw-bound fixtures resolve");
    if (karin < 0 || niji < 0 || filler < 0) return;
    check_identity(karin, KARIN_BP4, "the 果林 live-start print");

    set_stage(&tg, 0, 0, karin);
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int c = test_new_id(&tg, FILLER);
        if (c >= 0) set_stage(&tg, 1, i, c);
    }
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);
    test_add_to_discard(&tg, niji);
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, test_id(&tg, FILLER));

    int deck_before = test_deck_len(&tg);
    trigger_live_start_all_on_stage(&tg);
    drain_auto_then_first(&tg, 30);
    int deck_after = test_deck_len(&tg);
    CHECK(deck_after >= deck_before - 1,
          "the deck never loses more than the one card the first ability draws");
}

/* ══════════════════════════════════════════════════════════════════════
 * N. debut_opponent_optional_live_discard_or_total_score_{q130,edges}
 *    PL!S-pb1-002-R: 「登場 相手は手札からライブカードを1枚控え室に置いても
 *    よい。そうしなかった場合、ライブ終了時まで、ライブの合計スコアを+1する。」
 *    conditional_on_optional + negation, and the choice is routed to P2.
 * ══════════════════════════════════════════════════════════════════════ */

#define RIKO "PL!S-pb1-002-R"

static void test_riko_opponent_discards_live_blocks_the_score(void)
{
    TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, RIKO);
    int live   = test_id(&tg, SCORE1);
    int member = test_id(&tg, FILLER);
    int other  = test_id(&tg, "PL!-sd1-011-SD");
    CHECK(source >= 0 && live >= 0 && member >= 0 && other >= 0,
          "PL!S-pb1-002-R accept fixtures resolve");
    if (source < 0 || live < 0 || member < 0 || other < 0) return;
    check_identity(source, RIKO, "the 梨子 debut print");
    check_identity(live, SCORE1, "the live card the opponent is offered");
    CHECK(member != other, "the two non-live hand cards are distinct prints");

    test_add_to_hand(&tg, source);
    test_add_to_hand_for(&tg, 1, member);
    test_add_to_hand_for(&tg, 1, live);
    test_add_to_hand_for(&tg, 1, other);
    test_give_energy(&tg, 15);
    CHECK(test_play_to_stage(&tg, source, 1), "PL!S-pb1-002-R plays to the centre");

    CHECK(choice_is_select_card(&tg, "hand", 1, 1),
          "the opponent is asked for exactly one live card out of hand (skippable)");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "the score bonus does not exist before the opponent answers");
    const RbChoice *ch = pending_choice(&tg);
    CHECK(ch && ch->target_player_id[0] == 'p' && ch->target_player_id[1] == '2',
          "the discard choice is routed to the OPPONENT");
    answer(&tg, 0);
    int prompts = settle_card_first(&tg, 6);
    CHECK_EQ(prompts, 0,
             "answering the opponent's pick clears the compound in one resume");

    CHECK(p2_wait_has(&tg, live), "the opponent's live card reached their waitroom");
    CHECK(!p2_hand_has(&tg, live), "the offered live card left the opponent's hand");
    CHECK(p2_hand_has(&tg, member) && p2_hand_has(&tg, other),
          "the opponent keeps their two member cards");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "opponent DID discard -> the conditional +1 total score is skipped");
    CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 0, "the bonus is owner-side only");
}

static void test_riko_opponent_declines_grants_the_score(void)
{
    TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, RIKO);
    int live   = test_id(&tg, SCORE1);
    int member = test_id(&tg, FILLER);
    CHECK(source >= 0 && live >= 0 && member >= 0, "PL!S-pb1-002-R decline fixtures resolve");
    if (source < 0 || live < 0 || member < 0) return;
    check_identity(source, RIKO, "the 梨子 debut print");

    test_add_to_hand(&tg, source);
    test_add_to_hand_for(&tg, 1, member);
    test_add_to_hand_for(&tg, 1, live);
    test_give_energy(&tg, 15);
    CHECK(test_play_to_stage(&tg, source, 1), "PL!S-pb1-002-R plays to the centre");
    CHECK(choice_is_select_card(&tg, "hand", 1, 1), "the opponent is asked to discard a live card");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "the score bonus does not exist before the opponent answers");
    answer(&tg, -1);
    int prompts = settle_card_first(&tg, 6);
    CHECK_EQ(prompts, 0,
             "declining the opponent's pick clears the compound in one resume");

    CHECK(p2_hand_has(&tg, member) && p2_hand_has(&tg, live),
          "the opponent's hand is preserved after declining");
    CHECK_EQ(test_zone_len(&tg, 1, "discard"), 0, "the opponent's waitroom stays empty");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "opponent did NOT discard -> the conditional +1 total score applies");
    CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 0, "the bonus is owner-side only");
}

static void test_riko_opponent_without_live_card_grants_score_no_prompt(void)
{
    TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, RIKO);
    int member = test_id(&tg, FILLER);
    CHECK(source >= 0 && member >= 0, "PL!S-pb1-002-R no-live fixtures resolve");
    if (source < 0 || member < 0) return;
    check_identity(source, RIKO, "the 梨子 debut print");

    test_add_to_hand(&tg, source);
    test_add_to_hand_for(&tg, 1, member);
    test_give_energy(&tg, 15);
    CHECK(test_play_to_stage(&tg, source, 1), "PL!S-pb1-002-R plays to the centre");
    int prompts = settle_card_first(&tg, 6);
    CHECK_EQ(prompts, 0, "an opponent with no live card in hand is never prompted");
    CHECK(p2_hand_has(&tg, member), "the opponent keeps their member");
    CHECK_EQ(test_zone_len(&tg, 1, "discard"), 0, "the opponent's waitroom stays empty");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "no eligible target -> the optional resolves as declined -> +1 total score");
}

static void test_riko_opponent_empty_hand_grants_score_no_prompt(void)
{
    TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, RIKO);
    CHECK(source >= 0, "PL!S-pb1-002-R empty-hand fixture resolves");
    if (source < 0) return;
    check_identity(source, RIKO, "the 梨子 debut print");

    test_add_to_hand(&tg, source);
    test_give_energy(&tg, 15);
    CHECK(test_play_to_stage(&tg, source, 1), "PL!S-pb1-002-R plays to the centre");
    int prompts = settle_card_first(&tg, 6);
    CHECK_EQ(prompts, 0, "an empty opponent hand raises no prompt at all");
    CHECK_EQ(test_zone_len(&tg, 1, "hand"), 0, "the opponent's hand stays empty");
    CHECK_EQ(test_zone_len(&tg, 1, "discard"), 0, "the opponent's waitroom stays empty");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "an empty opponent hand still leaves the +1 total score in place");
}

static void test_riko_score_is_lost_when_riko_is_discarded(void)
{
    TestGame tg;
    test_game_new(&tg);
    int source  = test_id(&tg, RIKO);
    int yoshiko = test_id(&tg, "PL!S-bp3-006-R＋");
    int cost_card = test_id(&tg, FILLER);
    int live = test_id(&tg, SCORE1);
    CHECK(source >= 0 && yoshiko >= 0 && cost_card >= 0 && live >= 0,
          "PL!S-bp1-002-R revocation fixtures resolve");
    if (source < 0 || yoshiko < 0 || cost_card < 0 || live < 0) return;
    check_identity(source, RIKO, "the 梨子 debut print");
    check_identity(yoshiko, "PL!S-bp3-006-R＋", "the よしこ removal print");

    test_add_to_hand(&tg, source);
    test_add_to_hand(&tg, yoshiko);
    test_add_to_hand(&tg, cost_card);
    test_add_to_hand_for(&tg, 1, live);
    test_give_energy(&tg, 30);

    CHECK(test_play_to_stage(&tg, yoshiko, 1), "PL!S-bp3-006-R＋ plays to the centre");
    settle_card_first(&tg, 6);
    CHECK(test_play_to_stage(&tg, source, 0), "PL!S-pb1-002-R plays to the left");
    CHECK(choice_is_select_card(&tg, "hand", 1, 1), "the opponent is asked to discard a live card");
    answer(&tg, -1);
    int prompts = settle_card_first(&tg, 6);
    CHECK_EQ(prompts, 0, "declining the pick clears the compound in one resume");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "the declined discard leaves the +1 total score on the table");
    CHECK(tg.state.p[0].stage[0] == source && tg.state.p[0].stage[1] == yoshiko,
          "both members are staged where they were played");

    test_activate_ability(&tg, yoshiko);
    CHECK(pending(&tg), "PL!S-bp3-006-R＋ offers a member to send to the waitroom");
    answer(&tg, 0);
    settle_card_first(&tg, 6);
    CHECK(tg.state.p[0].stage[0] == RB_EMPTY_SLOT, "梨子 left the left slot");
    CHECK(wait_has(&tg, source), "the granted total score dies with 梨子 in the waitroom");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "the owner-scoped +1 total score is revoked when its source is discarded");
}

/* ══════════════════════════════════════════════════════════════════════
 * O. live_start_boosted_miraku_draws_and_reduces_need_heart
 *    PL!HS-pb1-029-L: 2+ みらくらぱーく！ members carrying MORE THAN THEIR
 *    PRINTED heart count -> draw 1 and relax 必要ハート heart0 by 2.
 * ══════════════════════════════════════════════════════════════════════ */

#define MIRAKU_LIVE "PL!HS-pb1-029-L"
#define MIRAKU_A   "PL!HS-bp1-005-PR"
#define MIRAKU_B   "PL!HS-PR-005-PR"

typedef struct { int need; int drawn; } MirakuResult;

static MirakuResult miraku_run(const char *const *a_no, const int *boosts, int n)
{
    MirakuResult out;
    out.need = -999; out.drawn = -999;
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, MIRAKU_LIVE);
    int filler = test_id(&tg, FILLER);
    if (live < 0 || filler < 0) return out;
    check_identity(live, MIRAKU_LIVE, "the みらくらぱーく！ live-start print");
    fill_decks(&tg, filler, 30);
    test_add_to_live(&tg, live);
    for (int i = 0; i < n; i++) {
        int c = test_id(&tg, a_no[i]);
        if (c < 0) return out;
        if (i == 1 && a_no[0] == a_no[1]) c = test_new_id(&tg, a_no[i]);
        if (c < 0) return out;
        set_stage(&tg, 0, i, c);
        if (boosts[i] > 0) {
            /* BOOST: extra hearts beyond the printed ones, so the printed
               "このカードよりもハートが多い" comparison is satisfied. */
            int color = (i == 0) ? RB_HEART_RED : RB_HEART_YELLOW;
            for (int k = 0; k < boosts[i]; k++)
                rb_mods_add_heart(&tg.state.mods, c, color, 1);
        }
    }
    int deck_before = test_deck_len(&tg);
    fire_trigger(&tg, live, RB_TSTR_LIVE_START);
    drain_skip(&tg, 20);
    out.drawn = deck_before - test_deck_len(&tg);
    out.need  = rb_mods_get_need_heart(&tg.state.mods, live, 0);
    return out;
}

static void test_miraku_one_boosted_member_draws_without_heart_reduction(void)
{
    static const char *const nos[1] = { MIRAKU_A };
    static const int boosts[1] = { 2 };
    MirakuResult r = miraku_run(nos, boosts, 1);
    CHECK_EQ(r.drawn, 1, "ONE boosted みらくらぱーく！ member -> draw 1");
    CHECK_EQ(r.need, 0, "one member is below 「2体以上」 -> no 必要ハート relaxation");
}

static void test_miraku_two_boosted_members_reduce_need_heart(void)
{
    static const char *const nos[2] = { MIRAKU_A, MIRAKU_B };
    static const int boosts[2] = { 2, 2 };
    MirakuResult r = miraku_run(nos, boosts, 2);
    CHECK_EQ(r.drawn, 1, "two boosted members still draw exactly 1 (not 2)");
    CHECK_EQ(r.need, -2, "two boosted members relax this live's 必要ハート heart0 by 2");
}

static void test_miraku_unboosted_members_do_nothing(void)
{
    static const char *const nos[2] = { MIRAKU_A, MIRAKU_B };
    static const int boosts[2] = { 0, 0 };
    MirakuResult r = miraku_run(nos, boosts, 2);
    CHECK_EQ(r.drawn, 0, "members without extra hearts do not qualify -> no draw");
    CHECK_EQ(r.need, 0, "members without extra hearts do not qualify -> no relaxation");
}

static void test_miraku_boosted_outsider_does_not_draw(void)
{
    static const char *const nos[1] = { FILLER };
    static const int boosts[1] = { 2 };
    MirakuResult r = miraku_run(nos, boosts, 1);
    CHECK_EQ(r.drawn, 0, "a boosted member of the wrong group does not qualify");
}

/* ══════════════════════════════════════════════════════════════════════
 * P. otherwise_condition_flow_test.rs
 *    PL!N-pb1-004-R: reveal the deck top; a member of cost <= 9 goes to hand
 *    AND this member position-changes; OTHERWISE the revealed card is
 *    discarded and nothing moves.
 * ══════════════════════════════════════════════════════════════════════ */

#define KARIN_N_PB1 "PL!N-pb1-004-R"

typedef struct {
    int cheap_in_hand, cheap_in_discard, cheap_in_deck;
    int karin_on_stage, karin_center;
} KarinOutcome;

static KarinOutcome karin_reveal_run(const char *deck_top_no)
{
    KarinOutcome o;
    memset(&o, 0, sizeof o);
    o.karin_center = -1;
    TestGame tg;
    test_game_new(&tg);
    int karin     = test_id(&tg, KARIN_N_PB1);
    int filler    = test_id(&tg, FILLER);
    int live_card = test_id(&tg, "PL!-sd1-020-SD");
    int deck_top  = test_new_id(&tg, deck_top_no);
    if (karin < 0 || filler < 0 || live_card < 0 || deck_top < 0) return o;
    check_identity(karin, KARIN_N_PB1, "the 果林 live-start print");

    fill_decks(&tg, filler, 10);
    /* Rust inserts at deck index 1, so exactly one filler sits above the
       marker and the live-card-set draw pulls that filler, exposing the
       marker as the new deck top. */
    RbPlayer *P = &tg.state.p[0];
    for (int i = P->deck.n; i > 1; i--) P->deck.cards[i] = P->deck.cards[i - 1];
    P->deck.cards[1] = deck_top;
    P->deck.n++;
    CHECK_EQ(P->deck.cards[1], deck_top, "setup: the marker sits one below the deck top");

    test_add_to_hand(&tg, live_card);
    test_add_to_hand(&tg, karin);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 20);
    CHECK(test_play_to_stage(&tg, filler, 0), "the filler member deploys to the left");
    CHECK(test_play_to_stage(&tg, karin, 1), "果林 deploys to the centre");

    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live_card);
    advance_to_live_start(&tg, 2);
    drain_first(&tg, 8);

    o.cheap_in_hand    = hand_has(&tg, deck_top);
    o.cheap_in_discard = wait_has(&tg, deck_top);
    o.cheap_in_deck    = deck_has(&tg, deck_top);
    o.karin_on_stage   = p1_stage_has(&tg, karin);
    o.karin_center     = tg.state.p[0].stage[1] == karin;
    return o;
}

static void test_karin_cheap_member_goes_to_hand_and_position_changes(void)
{
    KarinOutcome o = karin_reveal_run(FILLER);   /* member, cost 4 */
    CHECK_EQ(o.cheap_in_hand, 1, "a revealed cost-4 member satisfies the condition -> hand");
    CHECK_EQ(o.cheap_in_discard, 0, "the accepted branch must NOT also discard the card");
    CHECK_EQ(o.cheap_in_deck, 0, "the revealed card is removed from the deck either way");
    CHECK_EQ(o.karin_on_stage, 1, "果林 stays on stage after the position change");
    CHECK_EQ(o.karin_center, 0, "果林 position-changed out of the centre on the accepted branch");
}

static void test_karin_non_member_goes_to_discard(void)
{
    KarinOutcome o = karin_reveal_run("PL!-sd1-020-SD");  /* live card */
    CHECK_EQ(o.cheap_in_hand, 0, "a revealed live card never reaches the hand");
    CHECK_EQ(o.cheap_in_discard, 1,
             "otherwise_condition: a non-member revealed card is discarded");
    CHECK_EQ(o.karin_center, 1,
             "otherwise_condition: no position change when the condition fails");
}

static void test_karin_expensive_member_goes_to_discard(void)
{
    /* PL!N-pb1-004-R itself prints cost 11 (> the 9 limit). */
    TestGame tg;
    test_game_new(&tg);
    int karin     = test_id(&tg, KARIN_N_PB1);
    int filler    = test_id(&tg, FILLER);
    int live_card = test_id(&tg, "PL!-sd1-020-SD");
    int expensive = test_new_id(&tg, KARIN_N_PB1);
    CHECK(karin < 0 || expensive < 0 ? 0 : 1, "expensive-member fixtures resolve");
    if (karin < 0 || filler < 0 || live_card < 0 || expensive < 0) return;
    check_identity(karin, KARIN_N_PB1, "the 果林 live-start print");
    CHECK_EQ(card_cost(expensive), 11, "setup: the rejected member prints cost 11 (> 9)");

    fill_decks(&tg, filler, 10);
    RbPlayer *P = &tg.state.p[0];
    for (int i = P->deck.n; i > 1; i--) P->deck.cards[i] = P->deck.cards[i - 1];
    P->deck.cards[1] = expensive;
    P->deck.n++;
    test_add_to_hand(&tg, live_card);
    test_add_to_hand(&tg, karin);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 20);
    CHECK(test_play_to_stage(&tg, filler, 0), "the filler member deploys to the left");
    CHECK(test_play_to_stage(&tg, karin, 1), "果林 deploys to the centre");
    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live_card);
    advance_to_live_start(&tg, 2);
    drain_first(&tg, 8);

    CHECK(wait_has(&tg, expensive), "an over-priced member is discarded by otherwise_condition");
    CHECK(!hand_has(&tg, expensive), "an over-priced member never reaches the hand");
    CHECK(tg.state.p[0].stage[1] == karin, "an over-priced member triggers no position change");
}

static void test_karin_revealed_card_lands_in_exactly_one_zone(void)
{
    /* Rust case 6: hand and discard are mutually exclusive on BOTH branches. */
    KarinOutcome accepted = karin_reveal_run(FILLER);
    CHECK(accepted.cheap_in_hand != accepted.cheap_in_discard,
          "accepted branch: the revealed card is in exactly one of hand / discard");
    KarinOutcome rejected = karin_reveal_run("PL!-sd1-020-SD");
    CHECK(rejected.cheap_in_hand != rejected.cheap_in_discard,
          "rejected branch: the revealed card is in exactly one of hand / discard");
}

/* ══════════════════════════════════════════════════════════════════════
 * Q. stage_heart02_score_requirement_pl_n_bp5_028_l_test.rs
 *    PL!N-bp5-028-L: a heart02 member on stage boosts the live and REWRITES
 *    the 必要ハート to include heart02.
 * ══════════════════════════════════════════════════════════════════════ */

static void test_n_bp5_028_l_heart02_member_boosts_and_rewrites_need(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_new_id(&tg, "PL!N-sd1-010-SD");
    int live = test_id(&tg, "PL!N-bp5-028-L");
    int chika = test_new_id(&tg, "PL!S-bp5-001-R＋");
    CHECK(filler >= 0 && live >= 0 && chika >= 0, "PL!N-bp5-028-L fixtures resolve");
    if (filler < 0 || live < 0 || chika < 0) return;
    check_identity(live, "PL!N-bp5-028-L", "the 葉月 live-start print");
    check_identity(chika, "PL!S-bp5-001-R＋", "the 葉月 heart02 member");
    fill_decks(&tg, filler, 30);

    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    tg.state.phase = RB_PHASE_PERFORMANCE;
    set_stage(&tg, 0, 0, chika);
    CHECK(fire_trigger(&tg, live, RB_TSTR_LIVE_START),
          "PL!N-bp5-028-L prints a ライブ開始時 ability and the trigger is queued");
    drain_skip(&tg, 20);
    CHECK_EQ(test_get_score_modifier(&tg, live), 2, "a heart02 member on stage -> live +2");
    CHECK(need_heart_of(&tg, live, 2) >= 5,
          "the required hearts now include heart02 totalling >= 5");
}

static void test_n_bp5_028_l_wrong_group_member_does_not_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!N-bp5-028-L");
    int wrong = test_id(&tg, "PL!-sd1-013-SD");
    CHECK(live >= 0 && wrong >= 0, "PL!N-bp5-028-L wrong-group fixtures resolve");
    if (live < 0 || wrong < 0) return;
    check_identity(live, "PL!N-bp5-028-L", "the 葉月 live-start print");
    check_identity(wrong, "PL!-sd1-013-SD", "the wrong-group member");
    CHECK(!rb_card_matches_group_str(wrong, "Liella!"),
          "PL!-sd1-013-SD is not a Liella! print");

    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    tg.state.phase = RB_PHASE_PERFORMANCE;
    set_stage(&tg, 0, 0, wrong);
    fire_trigger(&tg, live, RB_TSTR_LIVE_START);
    drain_skip(&tg, 20);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "a member of the wrong group leaves the live-start rider unapplied");
    CHECK_EQ(need_heart_of(&tg, live, 2), 0,
             "a member of the wrong group leaves the required hearts untouched");
}

/* ══════════════════════════════════════════════════════════════════════
 * Diagnostic dump (RB_DUMP_ABILITIES=1), the same shape the peer suites use
 * for RB_DUMP_TRIGS / RB_DUMP_ACTIONS. Not part of the pass/fail run; it
 * exists so every gap reported below can be traced to the DECODED tree
 * instead of guessed at.
 * ══════════════════════════════════════════════════════════════════════ */

static void dump_cond(const Condition *c, int depth);

static void dump_cond_val(const CondValue *v, int depth)
{
    if (!v) { printf("%*s<null>\n", depth * 2, ""); return; }
    if (v->tag == RB_TAG_STR) printf(" str=%s", v->s ? v->s : "-");
    else if (v->tag == RB_TAG_I64) printf(" i=%lld", (long long)v->i);
    else if (v->tag == RB_TAG_TRUE) printf(" true");
    else if (v->tag == RB_TAG_FALSE) printf(" false");
    else if (v->tag == RB_TAG_ARRAY) {
        printf(" arr_n=%u", v->arr_n);
        for (uint32_t k = 0; k < v->arr_n; k++) {
            if (v->arr[k].tag == RB_TAG_STR) printf(" [%s]", v->arr[k].s ? v->arr[k].s : "-");
            else if (v->arr[k].tag == RB_TAG_I64) printf(" [%lld]", (long long)v->arr[k].i);
            else printf(" [t%u]", v->arr[k].tag);
        }
    } else if (v->tag == RB_TAG_OBJVAR) { printf(" obj:\n"); dump_cond(v->cond, depth + 2); }
    else printf(" tag=%u", v->tag);
}

static void dump_cond(const Condition *c, int depth)
{
    if (!c) { printf("%*s<null>\n", depth * 2, ""); return; }
    printf("%*svariant=%d n_fields=%u\n", depth * 2, "", c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        printf("%*s  [%u] key=%s", (depth + 1) * 2, "", i, c->fields[i].key ? c->fields[i].key : "?");
        dump_cond_val(&c->fields[i].v, depth + 1);
        printf("\n");
    }
}

static void dump_eff(const AbilityEffect *e, int depth)
{
    if (!e) { printf("%*s<null>\n", depth * 2, ""); return; }
    printf("%*saction=%s text=%s source=%s dest=%s target=%s type=%s count=%d opt=%d\n",
           depth * 2, "",
           e->action ? e->action : "-", e->text ? e->text : "-",
           e->source ? e->source : "-", e->destination ? e->destination : "-",
           e->target ? e->target : "-",
           e->card_type_field[0] ? e->card_type_field : "-",
           e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        printf("%*s  extra[%d] %s = %s\n", depth * 2, "", i,
               e->extra_k[i] ? e->extra_k[i] : "?", e->extra_v[i] ? e->extra_v[i] : "-");
    for (int i = 0; i < e->n_options; i++) {
        printf("%*s  option[%d]:\n", depth * 2, "", i);
        dump_eff(e->options[i], depth + 2);
    }
    if (e->condition) {
        printf("%*s  condition:\n", depth * 2, "");
        dump_cond(e->condition, depth + 2);
    }
    if (e->result_condition) {
        printf("%*s  result_condition:\n", depth * 2, "");
        dump_cond(e->result_condition, depth + 2);
    }
    if (e->alternative_condition) {
        printf("%*s  alternative_condition:\n", depth * 2, "");
        dump_cond(e->alternative_condition, depth + 2);
    }
    if (e->optional_action) {
        printf("%*s  optional_action:\n", depth * 2, "");
        dump_eff(e->optional_action, depth + 2);
    }
    if (e->conditional_action) {
        printf("%*s  conditional_action (negation=%d):\n", depth * 2, "", e->conditional_negation);
        dump_eff(e->conditional_action, depth + 2);
    }
    for (int i = 0; i < e->n_child; i++) {
        printf("%*s  child[%d]:\n", depth * 2, "", i);
        dump_eff(e->child[i], depth + 2);
    }
    if (e->primary_effect)     { printf("%*s  primary_effect:\n", depth * 2, "");     dump_eff(e->primary_effect, depth + 2); }
    if (e->alternative_effect) { printf("%*s  alternative_effect:\n", depth * 2, ""); dump_eff(e->alternative_effect, depth + 2); }
    if (e->gained_effect)      { printf("%*s  gained_effect:\n", depth * 2, "");      dump_eff(e->gained_effect, depth + 2); }
    if (e->opponent_action)    { printf("%*s  opponent_action:\n", depth * 2, "");    dump_eff(e->opponent_action, depth + 2); }
    if (e->resource_on_select) { printf("%*s  resource_on_select:\n", depth * 2, ""); dump_eff(e->resource_on_select, depth + 2); }
    if (e->look_action)    { printf("%*s  look_action:\n", depth * 2, "");    dump_eff(e->look_action, depth + 2); }
    if (e->select_action) { printf("%*s  select_action:\n", depth * 2, ""); dump_eff(e->select_action, depth + 2); }
    if (e->followup_action){printf("%*s  followup_action:\n", depth * 2, "");dump_eff(e->followup_action, depth + 2); }
}

static void ce_dump_card(const char *no)
{
    int cid = rb_find_card_by_no(no);
    if (cid < 0) { printf("\n=== %s : NOT IN DB\n", no); return; }
    printf("\n=== %s (id=%d cost=%d score=%d)\n", no, cid, card_cost(cid), card_score(cid));
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) { printf("  ab#%d decode fail\n", i); continue; }
        printf("  ab#%d triggers=%s\n", i, ab.triggers ? ab.triggers : "-");
        if (ab.effect) { printf("   effect:\n");   dump_eff(ab.effect, 2); }
        if (ab.cost)   { printf("   cost:\n");     dump_eff(ab.cost, 2); }
        if (ab.effect && ab.effect->condition) {
            printf("   effect-level condition:\n");
            dump_cond(ab.effect->condition, 2);
        }
        rb_free_ability(&ab);
    }
}

static void ce_dump_all(void)
{
    static const char *const cards[] = {
        PR045, MARI_008, HS_BP5_016, KOKO, "PL!HS-bp6-009-R", KUROE,
        SD2_023, KARIN_BP4, RIKO, MIRAKU_LIVE, KARIN_N_PB1, "PL!N-bp5-028-L",
        GINKO, UMI, KANAN,
    };
    for (unsigned i = 0; i < sizeof cards / sizeof cards[0]; i++) ce_dump_card(cards[i]);
}

/* ══════════════════════════════════════════════════════════════════════
 * main
 * ══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: card database load\n");
        return 2;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    if (getenv("RB_DUMP_ABILITIES")) { ce_dump_all(); rb_unload(); return 0; }

    /* A. check_self conditions follow the activating card. */
    test_check_self_location_follows_the_card();
    test_check_self_comparison_container_hand();

    /* B. rejecting / accepting condition arms. */
    test_ginko_all_heart04_gives_heart04();
    test_ginko_one_wrong_heart_skips_heart04();
    test_umi_opponent_success_score_threshold();
    test_emma_unpayable_cost_leaves_board_untouched();

    /* C. a compound effect gated on a printed card property (cost 7 baton). */
    test_pr045_baton_over_cost7_draws_two();
    test_pr045_baton_over_other_costs_does_not_draw();
    test_pr045_non_baton_does_not_draw();
    test_pr045_baton_hand_size_is_net_unchanged();

    /* D. context as part of the condition (復活 vs hand arrival). */
    test_yoshiko_006_from_hand_draws_two_no_blade();
    test_yoshiko_006_revived_from_waitroom_gains_blade();

    /* E. choose-1 where only one option carries the rider. */
    test_kanan_option1_blocks_opponent_blade_limited_wait();
    test_kanan_option1_blocks_opponent_wait();
    test_kanan_option2_does_not_block_opponent_wait();
    test_kanan_self_wait_still_works();

    /* F. a follow-up rider gated on the number of members the effect changed. */
    test_hs_bp5_016_n_waits_two_cheap_and_grants_heart06();
    test_hs_bp5_016_n_single_victim_grants_no_heart06();
    test_hs_bp5_016_n_waits_at_most_two_of_three();
    test_hs_bp5_016_n_decline_grants_no_heart06();

    /* G. a constant ability gated on a group count. */
    test_kaleidoscore_trio_grants_heart06_and_blade();
    test_kaleidoscore_two_grants_nothing();

    /* H. a mill whose source is deck_bottom, with an all-Aqours rider. */
    test_live_start_bottom_three_mills_the_bottom_cards();
    test_live_start_bottom_three_all_aqours_gives_heart04();

    /* I. mill + all-group rider, and the live-end expiry of that rider. */
    test_mill_four_all_group_cards_grants_one_blade_until_live_end();
    test_mill_any_wrong_group_card_prevents_blade();

    /* J. two independent score thresholds. */
    test_bp4_021_l_score_thresholds();

    /* K. a card_count condition gating a select_target pick. */
    test_kuroe_two_live_cards_offers_the_pick();
    test_kuroe_one_live_card_no_pick();
    test_kuroe_live_start_cards_are_excluded();
    test_kuroe_selected_live_goes_to_deck_top();
    test_kuroe_skip_keeps_the_live_zone();

    /* L. Q254: a met live-start condition is mandatory. */
    test_q254_success_count_thresholds();
    test_q254_p_variant_same_mandatory_threshold();

    /* M. a dynamic_count condition feeding the effect. */
    test_karin_topdeck_with_two_waited_opponents();
    test_karin_topdeck_with_no_waited_opponents_places_nothing();
    test_karin_topdeck_loses_at_most_the_draw();

    /* N. conditional_on_optional with negation, routed to the opponent. */
    test_riko_opponent_discards_live_blocks_the_score();
    test_riko_opponent_declines_grants_the_score();
    test_riko_opponent_without_live_card_grants_score_no_prompt();
    test_riko_opponent_empty_hand_grants_score_no_prompt();
    test_riko_score_is_lost_when_riko_is_discarded();

    /* O. "more hearts than printed" as a condition. */
    test_miraku_one_boosted_member_draws_without_heart_reduction();
    test_miraku_two_boosted_members_reduce_need_heart();
    test_miraku_unboosted_members_do_nothing();
    test_miraku_boosted_outsider_does_not_draw();

    /* P. otherwise_condition: hand branch vs discard branch. */
    test_karin_cheap_member_goes_to_hand_and_position_changes();
    test_karin_non_member_goes_to_discard();
    test_karin_expensive_member_goes_to_discard();
    test_karin_revealed_card_lands_in_exactly_one_zone();

    /* Q. a group filter inside a live-start rider that also rewrites 必要ハート. */
    test_n_bp5_028_l_heart02_member_boosts_and_rewrites_need();
    test_n_bp5_028_l_wrong_group_member_does_not_trigger();

    rb_unload();
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL CONDITION/AND-EFFECT PARITY CHECKS PASSED\n");
    return failures == 0 ? 0 : 1;
}

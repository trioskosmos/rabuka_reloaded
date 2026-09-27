/* test_parity_recover_to_hand.c -EC parity suite for the Rust cluster
 * engine/tests/test_modules/effects/recover/to_hand/.
 *
 * Every test pins the staged card's card_no first (the identity trap: prints
 * that share a character+number, e.g. PL!SP-bp2-015-R vs PL!SP-bp2-015-N, are
 * DIFFERENT cards), then drives the real pipeline and asserts the zone
 * contents the Rust twin asserts.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The U+FF0B fullwidth-plus print of the trio card. The Rust twin writes it
 * literally; the C source spells it as an explicit UTF-8 byte escape so the
 * file stays 7-bit clean under the MSVC/MinGW toolchains. */
#define TRIPLE_CARD_NO "LL-bp7-001-R\xef\xbc\x8b"

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

/* -- helpers ----------------------------------------------------------- */

static const char *card_no_of(int cid)
{
    static char buf[64];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static int card_printed_cost(int cid)
{
    Card c;
    int cost = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { cost = c.cost; rb_free_card(&c); }
    return cost;
}

static int card_printed_score(int cid)
{
    Card c;
    int score = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { score = c.score; rb_free_card(&c); }
    return score;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int bag_has(const RbBag *bag, int cid)
{
    for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) return 1;
    return 0;
}

static int waitroom_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(const TestGame *tg, int cid)    { return bag_has(&tg->state.p[0].hand, cid); }

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    P->hand.n = 0;
    P->discard.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

/* Push to the TOP of p1's deck (Rust main_deck.cards.insert(0, x) == draw order). */
static void put_on_deck_top(TestGame *tg, int pl, int cid)
{
    test_insert_deck_top(tg, pl, cid);
}

/* fire_trigger(game, cid, trigger, trig) -Ehelpers/mod.rs:117. The C engine
   exposes the per-trigger scan, so the queue is built from the same trigger
   token; fixture boards carry exactly one card printing it. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Answer the pending choice with a single index (Rust select_indices(&[0])). */
static void pick(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

/* Answer with several indices (Rust select_indices(&[0,1])). */
static void pick_n(TestGame *tg, const int *idx, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Answer with an empty selection (Rust select_indices(&[])). */
static void skip(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

static void drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) pick(tg, 0);
}

static void drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) skip(tg);
}

/* Rust drain_auto_ability_choices: SelectAutoAbility -> [0], anything else -> [] */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 32) {
        const char *t = test_pending_choice_type(tg);
        if (t && !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
        else skip(tg);
    }
}

/* Play a member to stage, answering a play-time alt-cost gate (rb_play_member
   pauses via ptc_active when the card carries a --E----Emodify_cost set).
   Rust's TestGame::play_to_stage drives the same pipeline and the alt-cost
   choice is declined, so answer index 0 unless `accept` is set. */
static int play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, accept);
        if (tg->state.p[0].stage[area] == cid) break;
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

/* Rust fill_decks(game, filler) -E30 cards each. */
static int pending_type_is(TestGame *tg, const char *want)
{
    const char *t = test_pending_choice_type(tg);
    return t && !strcmp(t, want);
}

/* Assert a card_no really is the print the test means to stage (identity rule). */
static void pin_id(int cid, const char *want)
{
    checks++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', resolved '%s' (id=%d)\n",
                want, card_no_of(cid), cid);
        failures++;
    } else {
        printf("ok: identity %s\n", want);
    }
}

/* ---------------------------------------------------------------------- */

static void test_debut_recovers_one_live_card_from_waitroom_to_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int triple = test_id(&tg, TRIPLE_CARD_NO);
    int live = test_id(&tg, "PL!-sd1-020-SD");
    pin_id(triple, TRIPLE_CARD_NO);
    pin_id(live, "PL!-sd1-020-SD");
    CHECK(triple >= 0 && live >= 0, "debut fixtures resolve");
    if (triple < 0 || live < 0) return;
    test_add_to_discard(&tg, live);
    test_add_to_hand(&tg, triple);
    test_give_energy(&tg, 15);
    CHECK_EQ(tg.state.p[0].discard.n, 1, "setup: exactly one card in the waitroom");
    CHECK_EQ(play_to_stage(&tg, triple, 1, 0), 1, "the play to stage succeeds");
    CHECK_EQ(tg.state.p[0].stage[1], triple, "the source member is on stage center");
    CHECK(hand_has(&tg, live),
          "debut should add a live card from waitroom to hand");
    CHECK(!waitroom_has(&tg, live),
          "the live card should leave the waitroom");
}

static void test_debut_live_recovery_leaves_member_in_waitroom(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int triple = test_id(&tg, TRIPLE_CARD_NO);
    int member = test_id(&tg, "PL!-sd1-010-SD");
    pin_id(triple, TRIPLE_CARD_NO);
    pin_id(member, "PL!-sd1-010-SD");
    CHECK(triple >= 0 && member >= 0, "debut fixtures resolve");
    if (triple < 0 || member < 0) return;
    test_add_to_hand(&tg, triple);
    test_add_to_discard(&tg, member);
    test_give_energy(&tg, 15);
    play_to_stage(&tg, triple, 1, 0);
    CHECK(waitroom_has(&tg, member),
          "non-live cards in waitroom are not touched by ab#1");
    CHECK(!hand_has(&tg, member), "the member never reaches the hand");
}

static void test_live_success_recovers_member_from_waitroom(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int triple = test_id(&tg, TRIPLE_CARD_NO);
    int member = test_id(&tg, "PL!-sd1-010-SD");
    pin_id(triple, TRIPLE_CARD_NO);
    CHECK(triple >= 0 && member >= 0, "live-success fixtures resolve");
    if (triple < 0 || member < 0) return;
    tg.state.p[0].stage[1] = triple;
    test_add_to_discard(&tg, member);
    CHECK(fire_trigger(&tg, triple, RB_TSTR_LIVE_SUCCESS),
          "the live-success trigger fires");
    drain_auto(&tg);
    CHECK(hand_has(&tg, member),
          "live success should add a member card from waitroom to hand");
    CHECK(!waitroom_has(&tg, member), "the member card should leave the waitroom");
}

static void test_live_success_member_recovery_ignores_live_cards(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int triple = test_id(&tg, TRIPLE_CARD_NO);
    int live_in_waitroom = test_id(&tg, "PL!-sd1-020-SD");
    CHECK(triple >= 0 && live_in_waitroom >= 0, "live-success fixtures resolve");
    if (triple < 0 || live_in_waitroom < 0) return;
    tg.state.p[0].stage[1] = triple;
    test_add_to_discard(&tg, live_in_waitroom);
    CHECK(fire_trigger(&tg, triple, RB_TSTR_LIVE_SUCCESS),
          "the live-success trigger fires");
    drain_auto(&tg);
    CHECK(waitroom_has(&tg, live_in_waitroom),
          "live cards in waitroom are not touched by ab#2");
    CHECK(!hand_has(&tg, live_in_waitroom),
          "live card must not be added to hand by ab#2");
}

static void test_discard_two_niji_member_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!N-sd1-005-PRproteinbar");
    int niji = test_id(&tg, "PL!N-bp3-004-R");
    pin_id(me, "PL!N-sd1-005-PRproteinbar");
    pin_id(niji, "PL!N-bp3-004-R");
    CHECK(me >= 0 && niji >= 0, "proteinbar fixtures resolve");
    if (me < 0 || niji < 0) return;
    tg.state.p[0].stage[0] = me;
    test_add_to_discard(&tg, niji);
    int f1 = test_new_id(&tg, "PL!-sd1-010-SD");
    int f2 = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, f1);
    test_add_to_hand(&tg, f2);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "setup: exactly two cards in hand for the cost");

    test_activate_ability(&tg, me);
    CHECK(rb_has_pending_choice(&tg.state), "2-card hand-discard cost must be prompted");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard discard-cost prompt");
    int both[2] = { 0, 1 };
    pick_n(&tg, both, 2);
    drain_auto(&tg);
    CHECK(hand_has(&tg, niji), "niji member retrieved from waitroom to hand");
    CHECK(!waitroom_has(&tg, niji), "retrieved member left the waitroom");
}

static void test_discard_cost_mill_then_member_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!N-bp1-009-R");
    int niji = test_id(&tg, "PL!N-bp3-004-R");
    pin_id(me, "PL!N-bp1-009-R");
    pin_id(niji, "PL!N-bp3-004-R");
    CHECK(me >= 0 && niji >= 0, "Rina fixtures resolve");
    if (me < 0 || niji < 0) return;
    int fodder = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, me);
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 15);
    test_add_to_discard(&tg, niji);
    int m1 = test_new_id(&tg, "PL!-sd1-010-SD");
    int m2 = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_deck(&tg, m1);
    test_add_to_deck(&tg, m2);
    int waitroom_before = tg.state.p[0].discard.n;
    play_to_stage(&tg, me, 1, 0);
    drain_pick0(&tg, 32);
    CHECK(hand_has(&tg, niji), "niji member retrieved to hand");
    CHECK(!waitroom_has(&tg, niji), "retrieved member left the waitroom");
    CHECK(waitroom_has(&tg, m1) && waitroom_has(&tg, m2),
          "both deck-top cards were milled to the waitroom");
    CHECK_EQ(tg.state.p[0].discard.n, waitroom_before + 2,
             "+2 milled-in, -1 retrieved-out");
}

static void test_self_wait_discard_niji_live_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!N-bp3-004-R");
    int live = test_id(&tg, "PL!N-bp1-025-L");
    pin_id(me, "PL!N-bp3-004-R");
    pin_id(live, "PL!N-bp1-025-L");
    CHECK(me >= 0 && live >= 0, "niji fixtures resolve");
    if (me < 0 || live < 0) return;
    tg.state.p[0].stage[0] = me;
    test_add_to_discard(&tg, live);
    int f1 = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, f1);
    test_activate_ability(&tg, me);
    CHECK(rb_has_pending_choice(&tg.state), "1-card hand-discard cost must be prompted");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard discard-cost prompt");
    pick(&tg, 0);
    drain_auto(&tg);
    const char *orient = rb_mods_get_orientation(&tg.state.mods, me);
    CHECK(orient && !strcmp(orient, "wait"),
          "this member was rested as part of the cost");
    CHECK(hand_has(&tg, live), "niji live card retrieved to hand");
}

static void test_self_to_waitroom_liella_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!SP-bp4-018-N");
    int liella = test_id(&tg, "PL!SP-pb1-001-PR");
    pin_id(me, "PL!SP-bp4-018-N");
    pin_id(liella, "PL!SP-pb1-001-PR");
    CHECK(me >= 0 && liella >= 0, "CatChu fixtures resolve");
    if (me < 0 || liella < 0) return;
    tg.state.p[0].stage[0] = me;
    test_add_to_discard(&tg, liella);
    test_activate_ability(&tg, me);
    CHECK(rb_has_pending_choice(&tg.state),
          "waitroom retrieval must be prompted (self now in waitroom = 2 candidates)");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard retrieval prompt");
    /* pick the Liella! card, which is waitroom index 1 after self_cost */
    int idx = -1;
    for (int i = 0; i < tg.state.p[0].discard.n; i++)
        if (tg.state.p[0].discard.cards[i] == liella) { idx = i; break; }
    CHECK(idx >= 0, "the Liella! card is still in the waitroom when the prompt is answered");
    if (idx >= 0) pick(&tg, idx);
    drain_auto(&tg);
    CHECK(waitroom_has(&tg, me), "this member moved to the waitroom");
    CHECK(hand_has(&tg, liella), "Liella! card retrieved to hand");
}

static void test_self_only_liella_pool_returns_self_to_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!SP-bp4-018-N");
    pin_id(me, "PL!SP-bp4-018-N");
    if (me < 0) return;
    tg.state.p[0].stage[0] = me;
    test_activate_ability(&tg, me);
    drain_auto(&tg);
    CHECK(!rb_has_pending_choice(&tg.state), "a single-candidate pool needs no prompt");
    CHECK(hand_has(&tg, me), "the only Liella! card in the waitroom is the member itself");
    CHECK(!waitroom_has(&tg, me), "she does not stay in the waitroom");
}

static void test_q123_self_cost_is_paid_without_recoverable_live(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int rin = test_id(&tg, "PL!-sd1-005-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    pin_id(rin, "PL!-sd1-005-SD");
    CHECK(rin >= 0 && filler >= 0, "Rin fixtures resolve");
    if (rin < 0 || filler < 0) return;
    tg.state.p[0].stage[1] = rin;
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 3);
    CHECK_EQ(tg.state.p[0].stage[1], rin, "Rin should be on stage before activation");
    test_activate_ability(&tg, rin);
    drain_skip(&tg, 16);
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT,
             "Rin should be removed from stage after self_cost");
    CHECK(waitroom_has(&tg, rin), "Rin should be in waitroom after self_cost");
}

static void test_q79_self_cost_vacates_center_and_recovers_live(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int rin = test_id(&tg, "PL!-sd1-005-SD");
    int live_card = test_id(&tg, "PL!-sd1-019-SD");
    pin_id(rin, "PL!-sd1-005-SD");
    pin_id(live_card, "PL!-sd1-019-SD");
    CHECK(rin >= 0 && live_card >= 0, "Rin fixtures resolve");
    if (rin < 0 || live_card < 0) return;
    tg.state.p[0].stage[1] = rin;
    test_add_to_hand(&tg, live_card);
    test_add_to_discard(&tg, live_card);
    test_give_energy(&tg, 3);
    test_activate_ability(&tg, rin);
    drain_skip(&tg, 16);
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT,
             "Center area should be empty after self_cost");
    CHECK(waitroom_has(&tg, rin), "Rin should be in waitroom");
    CHECK(hand_has(&tg, live_card), "Live card should be recovered to hand");
}

static void test_q79_self_cost_vacates_area_for_new_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int eli = test_id(&tg, "PL!-sd1-002-SD");
    int target_member = test_id(&tg, "PL!-sd1-001-SD");
    int new_member = test_id(&tg, "PL!-sd1-003-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    pin_id(eli, "PL!-sd1-002-SD");
    pin_id(target_member, "PL!-sd1-001-SD");
    pin_id(new_member, "PL!-sd1-003-SD");
    CHECK(eli >= 0 && target_member >= 0 && new_member >= 0, "Eli fixtures resolve");
    if (eli < 0 || target_member < 0 || new_member < 0) return;
    tg.state.p[0].stage[1] = eli;
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, new_member);
    test_add_to_discard(&tg, target_member);
    test_give_energy(&tg, 15);
    test_activate_ability(&tg, eli);
    CHECK(rb_has_pending_choice(&tg.state), "waitroom recover selection prompt expected");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard (recover from waitroom)");
    int idx = -1;
    for (int i = 0; i < tg.state.p[0].discard.n; i++)
        if (tg.state.p[0].discard.cards[i] == target_member) { idx = i; break; }
    CHECK(idx >= 0, "target_member should be in waitroom");
    if (idx >= 0) pick(&tg, idx);
    drain_auto(&tg);
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT, "Self_cost vacated center area");
    play_to_stage(&tg, new_member, 1, 0);
    CHECK_EQ(tg.state.p[0].stage[1], new_member,
             "New member placed in previously vacated center area");
}

static void test_lilywhite_gated_live_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!-pb1-007-R");
    int mate = test_id(&tg, "PL!-bp3-014-N");
    int mus_live = test_id(&tg, "PL!-sd1-020-SD");
    pin_id(me, "PL!-pb1-007-R");
    pin_id(mate, "PL!-bp3-014-N");
    CHECK(me >= 0 && mate >= 0 && mus_live >= 0, "Lilywhite fixtures resolve");
    if (me < 0 || mate < 0 || mus_live < 0) return;
    tg.state.p[0].stage[0] = me;
    tg.state.p[0].stage[1] = mate;
    test_give_energy(&tg, 15);
    int f1 = test_new_id(&tg, "PL!-sd1-010-SD");
    int f2 = test_new_id(&tg, "PL!-sd1-010-SD");
    int f3 = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, f1);
    test_add_to_hand(&tg, f2);
    test_add_to_hand(&tg, f3);
    test_add_to_discard(&tg, mus_live);
    test_activate_ability(&tg, me);
    CHECK(rb_has_pending_choice(&tg.state), "hand cost prompt expected");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard for the 3-card hand cost");
    int all[3] = { 0, 1, 2 };
    pick_n(&tg, all, 3);
    drain_auto(&tg);
    CHECK(hand_has(&tg, mus_live),
          "lilywhite member on stage -> retrieve mus's live card to hand");
}

static void test_no_lilywhite_member_prevents_live_recovery(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int me = test_id(&tg, "PL!-pb1-007-R");
    int printemps = test_id(&tg, "PL!-sd1-001-SD");
    int mus_live = test_id(&tg, "PL!-sd1-020-SD");
    pin_id(me, "PL!-pb1-007-R");
    pin_id(printemps, "PL!-sd1-001-SD");
    CHECK(me >= 0 && printemps >= 0 && mus_live >= 0, "Lilywhite fixtures resolve");
    if (me < 0 || printemps < 0 || mus_live < 0) return;
    tg.state.p[0].stage[0] = me;
    tg.state.p[0].stage[1] = printemps;
    test_give_energy(&tg, 15);
    int f1 = test_new_id(&tg, "PL!-sd1-010-SD");
    int f2 = test_new_id(&tg, "PL!-sd1-010-SD");
    int f3 = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, f1);
    test_add_to_hand(&tg, f2);
    test_add_to_hand(&tg, f3);
    test_add_to_discard(&tg, mus_live);
    test_activate_ability(&tg, me);
    CHECK(rb_has_pending_choice(&tg.state),
          "hand cost prompt expected even when retrieval condition fails");
    CHECK(pending_type_is(&tg, "SelectCard"), "expected SelectCard for the 3-card hand cost");
    int all[3] = { 0, 1, 2 };
    pick_n(&tg, all, 3);
    drain_auto(&tg);
    CHECK(!hand_has(&tg, mus_live), "no lilywhite member -> no retrieval");
}

static void test_heart01_filtered_live_recovery_recovers(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);
    int me = test_id(&tg, "PL!-PR-004-PR");
    int live = test_new_id(&tg, "PL!N-sd1-028-SD");
    pin_id(me, "PL!-PR-004-PR");
    pin_id(live, "PL!N-sd1-028-SD");
    CHECK(me >= 0 && live >= 0, "heart01 fixtures resolve");
    if (me < 0 || live < 0) return;
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_discard(&tg, live);
    test_activate_ability(&tg, me);
    drain_pick0(&tg, 10);
    CHECK(hand_has(&tg, live), "live with heart01>=3 requirement retrieved");
}

static void test_heart01_filtered_live_recovery_rejects(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);
    int me = test_id(&tg, "PL!-PR-004-PR");
    int live = test_new_id(&tg, "PL!HS-bp2-020-L");
    pin_id(me, "PL!-PR-004-PR");
    pin_id(live, "PL!HS-bp2-020-L");
    CHECK(me >= 0 && live >= 0, "heart01 fixtures resolve");
    if (me < 0 || live < 0) return;
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_discard(&tg, live);
    test_activate_ability(&tg, me);
    drain_pick0(&tg, 10);
    CHECK(!hand_has(&tg, live), "live with heart01<3 must NOT be retrievable");
}

static void test_three_5yncri5e_members_recover_waitroom_live(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);
    int me = test_id(&tg, "PL!SP-bp7-019-N");
    int s1 = test_id(&tg, "PL!SP-PR-005-PR");
    int s2 = test_id(&tg, "PL!SP-PR-008-PR");
    int mus_live = test_id(&tg, "PL!-sd1-020-SD");
    pin_id(me, "PL!SP-bp7-019-N");
    pin_id(s1, "PL!SP-PR-005-PR");
    pin_id(s2, "PL!SP-PR-008-PR");
    CHECK(me >= 0 && s1 >= 0 && s2 >= 0 && mus_live >= 0, "5yncri5e fixtures resolve");
    if (me < 0 || s1 < 0 || s2 < 0 || mus_live < 0) return;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 30);
    tg.state.p[0].stage[0] = s1;
    tg.state.p[0].stage[1] = s2;
    test_add_to_discard(&tg, mus_live);
    play_to_stage(&tg, me, 2, 0);
    drain_pick0(&tg, 10);
    CHECK(hand_has(&tg, mus_live), "3x 5yncri5e! staged -> live card retrieved to hand");
}

static void test_only_two_5yncri5e_members_do_not_recover_live(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);
    int me = test_id(&tg, "PL!SP-bp7-019-N");
    int s1 = test_id(&tg, "PL!SP-PR-005-PR");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int mus_live = test_id(&tg, "PL!-sd1-020-SD");
    pin_id(me, "PL!SP-bp7-019-N");
    pin_id(s1, "PL!SP-PR-005-PR");
    CHECK(me >= 0 && s1 >= 0 && outsider >= 0 && mus_live >= 0, "5yncri5e fixtures resolve");
    if (me < 0 || s1 < 0 || outsider < 0 || mus_live < 0) return;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 30);
    tg.state.p[0].stage[0] = s1;
    tg.state.p[0].stage[1] = outsider;
    test_add_to_discard(&tg, mus_live);
    play_to_stage(&tg, me, 2, 0);
    drain_pick0(&tg, 10);
    CHECK(!hand_has(&tg, mus_live), "only 2x 5yncri5e! -> no retrieval");
}

static void test_series_level_and_unit_level_groups_resolve_by_different_routes(void)
{
    TestGame tg;
    test_game_new(&tg);
    int unitless_live = test_id(&tg, "PL!SP-bp1-023-L");
    int catchu = test_id(&tg, "PL!SP-bp1-004-PR");
    int five = test_id(&tg, "PL!SP-bp1-014-N");
    pin_id(unitless_live, "PL!SP-bp1-023-L");
    pin_id(catchu, "PL!SP-bp1-004-PR");
    pin_id(five, "PL!SP-bp1-014-N");
    CHECK(unitless_live >= 0 && catchu >= 0 && five >= 0, "unit fixtures resolve");
    if (unitless_live < 0 || catchu < 0 || five < 0) return;
    CHECK(rb_card_matches_group_str(unitless_live, "Liella!"),
          "a card with no unit still matches a SERIES-level group name");
    CHECK(rb_card_matches_group_str(catchu, "Liella!"),
          "a card whose unit is something else still matches the series-level name");
    CHECK(!rb_card_matches_group_str(catchu, "5yncri5e!") &&
          !rb_card_matches_group_str(unitless_live, "5yncri5e!"),
          "neither matches a UNIT-level name");
    CHECK(rb_card_matches_group_str(five, "5yncri5e!"),
          "a card whose unit IS 5yncri5e! does match it");
}

/* ---------------------------------------------------------------------- */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* RB_DUMP=<card_no>[,<card_no>...] -Edecode and print every ability the card
   carries, so a red test can be diagnosed without guessing. */
static void dump_effect(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("[%d] action=%s src=%s dst=%s target=%s count=%d opt=%d",
           depth, e->action ? e->action : "-", e->source ? e->source : "-",
           e->destination ? e->destination : "-", e->target ? e->target : "-",
           e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        printf(" [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?", e->extra_v[i] ? e->extra_v[i] : "?");
    printf("\n");
    dump_effect(e->primary_effect, depth + 1);
    dump_effect(e->alternative_effect, depth + 1);
    dump_effect(e->followup_action, depth + 1);
    dump_effect(e->optional_action, depth + 1);
    dump_effect(e->conditional_action, depth + 1);
    dump_effect(e->look_action, depth + 1);
    dump_effect(e->select_action, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_effect(e->options[i], depth + 1);
}

static int run_dump(const char *list)
{
    char buf[512];
    snprintf(buf, sizeof buf, "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
        int cid = rb_find_card_by_no(tok);
        printf("== %s -> id=%d resolved_card_no=%s cost=%d\n", tok, cid, card_no_of(cid),
               card_printed_cost(cid));
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int i = 0; i < n; i++) {
            Ability ab;
            memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            printf("-- ability %d triggers=%s use_limit=%d text=%s\n", i,
                   ab.triggers ? ab.triggers : "-", ab.use_limit,
                   ab.triggerless_text ? ab.triggerless_text : "-");
            if (ab.cost) { printf("  COST:\n"); dump_effect(ab.cost, 1); }
            if (ab.effect) { printf("  EFFECT:\n"); dump_effect(ab.effect, 1); }
            rb_free_ability(&ab);
        }
    }
    return 0;
}

/* RB_ONLY=<substr> runs just the matching cases (diagnostic aid; the default
   `RB_ONLY`-less run executes all of them). */
static void run_all(const char *only)
{
    struct { const char *name; void (*fn)(void); } cases[] = {
        { "debut_live",              test_debut_recovers_one_live_card_from_waitroom_to_hand },
        { "debut_member_untouched",  test_debut_live_recovery_leaves_member_in_waitroom },
        { "live_success_member",     test_live_success_recovers_member_from_waitroom },
        { "live_success_ignores",    test_live_success_member_recovery_ignores_live_cards },
        { "proteinbar",              test_discard_two_niji_member_recovery },
        { "rina_mill",               test_discard_cost_mill_then_member_recovery },
        { "niji_rest_discard",       test_self_wait_discard_niji_live_recovery },
        { "liella_self_wait",        test_self_to_waitroom_liella_recovery },
        { "liella_pool_self",        test_self_only_liella_pool_returns_self_to_hand },
        { "q123",                    test_q123_self_cost_is_paid_without_recoverable_live },
        { "q79_live",                test_q79_self_cost_vacates_center_and_recovers_live },
        { "q79_member",              test_q79_self_cost_vacates_area_for_new_member },
        { "lilywhite_ok",            test_lilywhite_gated_live_recovery },
        { "lilywhite_no",            test_no_lilywhite_member_prevents_live_recovery },
        { "heart01_ok",              test_heart01_filtered_live_recovery_recovers },
        { "heart01_no",              test_heart01_filtered_live_recovery_rejects },
        { "fiveync_ok",              test_three_5yncri5e_members_recover_waitroom_live },
        { "fiveync_no",              test_only_two_5yncri5e_members_do_not_recover_live },
        { "group_routes",            test_series_level_and_unit_level_groups_resolve_by_different_routes },
    };
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        if (only && *only && !strstr(cases[i].name, only)) continue;
        printf("--- case %s\n", cases[i].name);
        cases[i].fn();
    }
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    { const char *d = getenv("RB_DUMP"); if (d && *d) return run_dump(d); }
    run_all(getenv("RB_ONLY"));
    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL RECOVER TO HAND PARITY CHECKS PASSED\n");
    return 0;
}

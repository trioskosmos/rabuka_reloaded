/* test_parity_reveal_extra.c - C parity suite for the Rust cluster
 * engine/tests/test_modules/effects/look_select/reveal/ (14 test files,
 * 50 #[test] functions).
 *
 * SCOPE NOTE. The sibling suites engine_c_wip/tests/test_parity_look_reveal.c
 * and tests/test_look_select.c are EFFECT-LEVEL: they drive
 * rb_execute_effect_ex() with a hand-built or single-decoded AbilityEffect and
 * assert look.c / choice.c in isolation. Every Rust test in this cluster is a
 * GAMEPLAY-LEVEL flow: it stages a real print through the debut trigger, pays
 * (or declines) an optional cost, drains the reveal prompt, and asserts the
 * resulting zones by card identity. Grepping the whole tests/ tree for the 20
 * card_nos this cluster names showed only PL!S-pb1-013-N and PL!S-pb1-014-N
 * present (test_parity_look_reveal.c:1038/1056) and even there only at effect
 * level, with no debut trigger and no optional-cost gate. So this file ports
 * the whole cluster, including the two Dia/黑澤 diamonds, because the
 * trigger + cost-gate + filter path is genuinely uncovered.
 *
 * Every case pins the staged print's card_no first (the identity trap:
 * PL!SP-bp2-011-R and PL!SP-pb2-011-R are DIFFERENT cards, and so are the
 * many same-character / different-rarity prints this cluster relies on).
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* rb_record_card_appearance exists in src/core/modifiers.c but is not declared
 * in include/rabuka.h yet. Declared here so the PL!S-bp6-016-N cases can stage
 * an appearance exactly as the Rust twin does. NOTE: the C definition starts
 * with `(void)source;` - see the engine-gap notes in the run report. */
extern void rb_record_card_appearance(GameState *g, int card_id, int source);

/* U+FF0B FULLWIDTH PLUS SIGN. The print PL!-bp6-007-R+ in the Rust cluster is
 * spelled with the fullwidth plus in cards.json; spelled here as an explicit
 * UTF-8 byte escape so the file stays 7-bit clean for the toolchains. */
#define BP6_007_R_FULLWIDTH_PLUS "PL!-bp6-007-R\xef\xbc\x8b"

/* μ's SD filler, cost 4 - never qualifies for a cost>=9 filter. */
#define FILLER_MUS "PL!-sd1-010-SD"

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

/* ── helpers ───────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

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

static int hand_has(const TestGame *tg, int cid)    { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid)    { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(const TestGame *tg, int cid)    { return bag_has(&tg->state.p[0].deck, cid); }
static int hand_has_pl(const TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].hand, cid); }

static int stage_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

/* Rust: assert_eq!(zone.cards.as_slice(), &[a, b, c]) - exact ordered match. */
static int bag_is(const RbBag *bag, const int *ids, int n)
{
    if (bag->n != n) return 0;
    for (int i = 0; i < n; i++) if (bag->cards[i] != ids[i]) return 0;
    return 1;
}

static void check_bag_is(const RbBag *bag, const int *ids, int n, const char *what)
{
    checks++;
    if (bag_is(bag, ids, n)) {
        printf("ok: %s\n", what);
    } else {
        fprintf(stderr, "FAIL: %s: zone has", what);
        for (int i = 0; i < bag->n; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
        fprintf(stderr, " | expected");
        for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(ids[i]));
        fprintf(stderr, "\n");
        failures++;
    }
}

static void check_bag_count(const RbBag *bag, int n, const char *what)
{
    checks++;
    if (bag->n == n) {
        printf("ok: %s\n", what);
    } else {
        fprintf(stderr, "FAIL: %s (got %d expected %d):", what, bag->n, n);
        for (int i = 0; i < bag->n; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
        fprintf(stderr, "\n");
        failures++;
    }
}

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

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0;
    P->hand.n = 0;
    P->discard.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; }
}

/* Rust fill_decks(game, filler): 30 cards each seat. */
static void fill_decks(TestGame *tg, int filler)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < 30 && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

/* Rust: stock a 40-card deck whose TOP n cards are `top`, filler below. */
static void stock_deck_top(TestGame *tg, const int *top, int n_top, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n_top; i++) P->deck.cards[P->deck.n++] = top[i];
    while (P->deck.n < 40 && P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = filler;
}

/* Rust: main_deck.cards.insert(0, x). */
static void put_on_deck_top(TestGame *tg, int pl, int cid) { test_insert_deck_top(tg, pl, cid); }

/* fire_trigger(game, cid, trigger, trig) - helpers/mod.rs:117 */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Same, for the p2 seat - the Rust twin calls process_pending_auto_abilities("p2"). */
static int fire_trigger_p2(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 1, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static void pick(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

/* Rust select_indices(&[]) - DECLINE. */
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

static int pending_type_is(TestGame *tg, const char *want)
{
    const char *t = test_pending_choice_type(tg);
    return t && !strcmp(t, want);
}

/* Rust assert_select_card(zone, count, allow_skip). */
static void check_choice_is(TestGame *tg, const char *zone, int count, int allow_skip, const char *what)
{
    checks++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->zone[0] && !strcmp(c->zone, zone) && c->count == count &&
        (allow_skip < 0 || c->allow_skip == allow_skip)) {
        printf("ok: %s\n", what);
    } else {
        fprintf(stderr, "FAIL: %s (zone=%s count=%d allow_skip=%d | want zone=%s count=%d allow_skip=%d)\n",
                what, c ? c->zone : "(none)", c ? c->count : -1, c ? c->allow_skip : -1,
                zone, count, allow_skip);
        failures++;
    }
}

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

/* Assert a card_no really is the print the test means to stage (identity rule). */
static int pin_id(int cid, const char *want)
{
    checks++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', resolved '%s' (id=%d)\n",
                want, card_no_of(cid), cid);
        failures++;
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

static void pin_cost(int cid, int want){
    checks++;
    int got = card_printed_cost(cid);
    if (got != want) {
        fprintf(stderr, "FAIL: identity: %s printed cost %d, test expects %d\n",
                card_no_of(cid), got, want);
        failures++;
    } else {
        printf("ok: identity %s cost=%d\n", card_no_of(cid), got);
    }
}

static const char *orientation_of(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static int is_wait(TestGame *tg, int cid)
{
    const char *o = orientation_of(tg, cid);
    return o && !strcmp(o, "wait");
}

/* ══════════════════════════════════════════════════════════════════╁E   1. debut_group_look_three_reveal_test.rs
      PL!N-sd2-009-SD2 (東条 币E cost 11)
      登場 自刁E�EチE��キの上からカードを3枚見る。その中から『虹ヶ咲』�Eカードを
      1枚�E開して手札に�?えてもよぁE��残りを控え室に置く、E   ══════════════════════════════════════════════════════════════════╁E*/

/* Rust debut_group_look(target): deck = [a, b, c, d], p2 deck = 1 filler. */
static TestGame debut_group_look(const char *target_no, int *out4)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int member = test_id(&tg, "PL!N-sd2-009-SD2");
    pin_id(member, "PL!N-sd2-009-SD2");
    pin_cost(member, 11);
    if (member < 0) { memset(&tg, 0, sizeof tg); return tg; }

    int a = test_id(&tg, FILLER_MUS);           /* μ's, never matches 虹ヶ咲 */
    int b = test_id(&tg, target_no);            /* the group card under test */
    int c = test_id(&tg, "PL!S-sd1-001-SD");    /* Aqours, never matches */
    int d = test_id(&tg, FILLER_MUS);
    out4[0] = a; out4[1] = b; out4[2] = c; out4[3] = d;

    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = a;
    P->deck.cards[P->deck.n++] = b;
    P->deck.cards[P->deck.n++] = c;
    P->deck.cards[P->deck.n++] = d;
    int opponent = test_id(&tg, FILLER_MUS);
    RbPlayer *O = &tg.state.p[1];
    O->deck.n = 0;
    O->deck.cards[O->deck.n++] = opponent;

    test_add_to_hand(&tg, member);
    test_give_energy(&tg, 20);
    play_to_stage(&tg, member, 1, 0);
    CHECK_EQ(tg.state.p[0].stage[1], member, "sd2-009: the member is on stage centre");
    return tg;
}

static void test_debut_look_three_reveals_group_member(void)
{
    int cards[4];
    TestGame tg = debut_group_look("PL!N-bp1-004-R", cards);
    int a = cards[0], b = cards[1], c = cards[2], d = cards[3];
    CHECK(rb_has_pending_choice(&tg.state), "sd2-009: the reveal prompt is offered");
    check_choice_is(&tg, "looked_at", 1, 1, "sd2-009: reveal prompt is looked_at/1/skippable");
    pick(&tg, 0);
    CHECK(!rb_has_pending_choice(&tg.state), "sd2-009: no prompt survives the pick");
    CHECK(hand_has(&tg, b), "sd2-009: the revealed 虹ヶ咲 card reaches the hand");
    int wait_expected[2] = { a, c };
    check_bag_is(&tg.state.p[0].discard, wait_expected, 2,
                 "sd2-009: the two unselected looked cards go to the waitroom");
    int deck_expected[1] = { d };
    check_bag_is(&tg.state.p[0].deck, deck_expected, 1,
                 "sd2-009: the deck tail below the looked three is untouched");
}

static void test_debut_look_three_reveals_group_live(void)
{
    int cards[4];
    TestGame tg = debut_group_look("PL!N-sd1-019-SD", cards);
    int a = cards[0], b = cards[1], c = cards[2], d = cards[3];
    CHECK(rb_has_pending_choice(&tg.state), "sd2-009: a group LIVE also opens the prompt");
    pick(&tg, 0);
    CHECK(!rb_has_pending_choice(&tg.state), "sd2-009: no prompt survives the live pick");
    CHECK(hand_has(&tg, b), "sd2-009: a 虹ヶ咲 live card is revealable, not just members");
    int wait_expected[2] = { a, c };
    check_bag_is(&tg.state.p[0].discard, wait_expected, 2,
                 "sd2-009: live-reveal remainder goes to the waitroom");
    int deck_expected[1] = { d };
    check_bag_is(&tg.state.p[0].deck, deck_expected, 1, "sd2-009: deck tail untouched");
}

static void test_debut_look_three_decline_discards_all(void)
{
    int cards[4];
    TestGame tg = debut_group_look("PL!N-bp1-004-R", cards);
    int a = cards[0], b = cards[1], c = cards[2], d = cards[3];
    CHECK(rb_has_pending_choice(&tg.state), "sd2-009: the prompt is offered before declining");
    skip(&tg);
    CHECK(!rb_has_pending_choice(&tg.state), "sd2-009: declining closes the prompt");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "sd2-009: a declined reveal takes nothing to hand");
    int wait_expected[3] = { a, b, c };
    check_bag_is(&tg.state.p[0].discard, wait_expected, 3,
                 "sd2-009: declining discards ALL three looked cards");
    int deck_expected[1] = { d };
    check_bag_is(&tg.state.p[0].deck, deck_expected, 1, "sd2-009: deck tail untouched");
}

static void test_debut_look_three_no_group_card_no_prompt(void)
{
    int cards[4];
    TestGame tg = debut_group_look(FILLER_MUS, cards);
    int a = cards[0], b = cards[1], c = cards[2], d = cards[3];
    CHECK(!rb_has_pending_choice(&tg.state),
          "sd2-009: no 虹ヶ咲 card among the three means no prompt at all");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "sd2-009: nothing is taken when nothing matches");
    int wait_expected[3] = { a, b, c };
    check_bag_is(&tg.state.p[0].discard, wait_expected, 3,
                 "sd2-009: all three non-matching looked cards go to the waitroom");
    int deck_expected[1] = { d };
    check_bag_is(&tg.state.p[0].deck, deck_expected, 1, "sd2-009: deck tail untouched");
}

/* ══════════════════════════════════════════════════════════════════╁E   2. debut_look_liella_cost4_hand_or_debut_test.rs
      PL!SP-pb2-001-R (澁谷か�EめE cost 15) - OPTIONAL cost, look 5,
      cost<=4 Liella! MEMBER, max 1, then debut-to-stage OR add-to-hand.
      NOTE the identity trap: PL!SP-pb2-001-R is this print; the
      PL!SP-bp2-001-R family is a different card and is not staged here.
   ══════════════════════════════════════════════════════════════════╁E*/

/* Rust setup_kanon(game, top_cards): hand = [kanon, filler, filler],
   20 energy, deck = 10 filler then top_cards reversed onto the top. */
static int setup_kanon(TestGame *tg, const int *top, int n_top, int *out_kanon, int *out_filler)
{
    clear_p1(tg);
    clear_p2(tg);
    int kanon = test_id(tg, "PL!SP-pb2-001-R");
    int filler = test_id(tg, FILLER_MUS);
    pin_id(kanon, "PL!SP-pb2-001-R");
    pin_cost(kanon, 15);
    *out_kanon = kanon;
    *out_filler = filler;
    if (kanon < 0 || filler < 0) return 0;
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 10; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = n_top - 1; i >= 0; i--) P->deck.cards[P->deck.n++] = top[i];
    test_add_to_hand(tg, kanon);
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, filler);
    test_give_energy(tg, 20);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
    return 1;
}

/* Rust pay_optional_cost: resume_with_choice(None, Some(vec![0])). */
static void kanon_pay_cost(TestGame *tg) { pick(tg, 0); }

static int choice_filtered_count(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->n_filtered_indices : 0;
}

static void test_kanon_select_liella_cost4_keep_in_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, "PL!SP-PR-003-PR");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(liella, "PL!SP-PR-003-PR");
    pin_cost(liella, 2);
    int top[2] = { liella, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    CHECK(rb_has_pending_choice(&tg.state), "kanon: the looked_at choice appears");
    check_choice_is(&tg, "looked_at", 1, -1, "kanon: the pick is over looked_at");
    CHECK_EQ(choice_filtered_count(&tg), 1,
             "kanon: only the single Liella! cost<=4 card is selectable");
    pick(&tg, 0);
    drain_skip(&tg, 6);
    CHECK(hand_has(&tg, liella), "kanon: skipping the stage followup leaves the card in hand");
    CHECK(!stage_has(&tg, liella), "kanon: the card never reached the stage");
}

static void test_kanon_select_liella_debut_to_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, "PL!SP-PR-003-PR");
    int filler = test_id(&tg, FILLER_MUS);
    int top[2] = { liella, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "kanon: the stage-debut followup is offered");
    CHECK(pending_type_is(&tg, "SelectTarget"),
          "kanon: the stage-debut followup is a SelectTarget");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "kanon: a SelectPosition prompt follows");
    CHECK(pending_type_is(&tg, "SelectPosition"),
          "kanon: the 登場-driven followup is a SelectPosition");
    pick(&tg, 0);
    CHECK(stage_has(&tg, liella), "kanon: the debuted Liella! card is on stage");
    CHECK(!hand_has(&tg, liella), "kanon: the debuted card is no longer in hand");
}

static void test_kanon_skip_cost_effect_not_executed(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, "PL!SP-PR-003-PR");
    int filler = test_id(&tg, FILLER_MUS);
    int top[2] = { liella, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    int deck_before = tg.state.p[0].deck.n;
    play_to_stage(&tg, kanon, 1, 0);
    drain_skip(&tg, 8);
    CHECK(!hand_has(&tg, liella), "kanon: skipping the cost never executes the look");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "kanon: a skipped cost leaves the deck untouched");
}

static void test_kanon_no_matching_cards_discard_all(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, FILLER_MUS);
    int non_mus = test_id(&tg, "PL!HS-sd1-010-SD");   /* Hasunosora cost 4, not Liella! */
    pin_id(non_mus, "PL!HS-sd1-010-SD");
    int top[2] = { filler, non_mus };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    drain_skip(&tg, 8);
    CHECK(wait_has(&tg, filler), "kanon: the μ's filler is discarded when nothing matches");
    CHECK(wait_has(&tg, non_mus), "kanon: the non-Liella! card is discarded when nothing matches");
    CHECK(!hand_has(&tg, non_mus), "kanon: the non-Liella! card is never kept");
}

static void test_kanon_cost_above_4_rejected(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanon_self = test_id(&tg, "PL!SP-pb2-001-R");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(kanon_self, "PL!SP-pb2-001-R");
    pin_cost(kanon_self, 15);
    int top[2] = { kanon_self, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    drain_skip(&tg, 8);
    CHECK(!hand_has(&tg, kanon_self),
          "kanon: a cost-15 Liella! copy is above the cost<=4 gate and is not selectable");
    CHECK(wait_has(&tg, kanon_self),
          "kanon: the rejected cost>4 card goes to the waitroom with the rest");
}

static void test_kanon_non_liella_cost4_rejected(void)
{
    TestGame tg;
    test_game_new(&tg);
    int non_liella = test_id(&tg, "PL!-sd1-008-SD");   /* μ's cost 4 */
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(non_liella, "PL!-sd1-008-SD");
    pin_cost(non_liella, 4);
    int top[2] = { non_liella, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    drain_skip(&tg, 8);
    CHECK(!hand_has(&tg, non_liella),
          "kanon: a cost-4 μ's member fails the Liella! group filter and is not kept");
    CHECK(wait_has(&tg, non_liella), "kanon: the group-rejected card goes to the waitroom");
}

static void test_kanon_max_1_enforced(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella_a = test_id(&tg, "PL!SP-PR-003-PR");
    int liella_b = test_id(&tg, "PL!SP-PR-004-PR");
    pin_id(liella_a, "PL!SP-PR-003-PR");
    pin_id(liella_b, "PL!SP-PR-004-PR");
    int top[2] = { liella_a, liella_b };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    pick(&tg, 0);
    drain_skip(&tg, 8);
    int in_hand = 0;
    for (int i = 0; i < tg.state.p[0].hand.n; i++) {
        if (tg.state.p[0].hand.cards[i] == liella_a || tg.state.p[0].hand.cards[i] == liella_b)
            in_hand++;
    }
    CHECK_EQ(in_hand, 1, "kanon: max=1 - exactly one of the two eligible cards is kept");
}

static void test_kanon_stage_full_falls_back_to_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, "PL!SP-PR-003-PR");
    int filler = test_id(&tg, FILLER_MUS);
    int top[2] = { liella, filler };
    int kanon, f;
    CHECK(setup_kanon(&tg, top, 2, &kanon, &f), "kanon: fixtures resolve");
    if (kanon < 0) return;
    int fc2 = test_new_id(&tg, FILLER_MUS);
    int fc3 = test_new_id(&tg, FILLER_MUS);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = fc2;
    tg.state.p[0].stage[2] = fc3;
    play_to_stage(&tg, kanon, 1, 0);
    kanon_pay_cost(&tg);
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state),
          "kanon: the stage-debut choice is still offered with a full stage");
    CHECK(pending_type_is(&tg, "SelectTarget"),
          "kanon: the full-stage followup is still a SelectTarget");
    pick(&tg, 0);
    CHECK(!rb_has_pending_choice(&tg.state),
          "kanon: no follow-up prompt when the stage is full (the debut fails)");
    CHECK(!stage_has(&tg, liella), "kanon: the card does not reach a full stage");
}

/* ══════════════════════════════════════════════════════════════════╁E   3. empty_hand_skips_optional_discard_look_test.rs
      PL!-bp3-007-R (のぞみ, cost 9)
      ライブ開始時 手札めE枚控え室に置ぁE��もよぁE��デチE��上かめE枚見る、E      そ�E中から1枚を手札に�?え、E枚をチE��キの上に置き、E枚を控え室に置く、E   ══════════════════════════════════════════════════════════════════╁E*/

static void test_nozomi_empty_hand_skips_discard_cost_and_look(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int nozomi = test_id(&tg, "PL!-bp3-007-R");
    pin_id(nozomi, "PL!-bp3-007-R");
    pin_cost(nozomi, 9);
    if (nozomi < 0) return;
    CHECK(card_prints_trigger(nozomi, RB_TSTR_LIVE_START),
          "nozomi: the print carries a ライブ開始時 ability");
    tg.state.p[0].stage[1] = nozomi;
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    int deck_before = tg.state.p[0].deck.n;
    int hand_before = tg.state.p[0].hand.n;
    CHECK_EQ(hand_before, 0, "nozomi: setup really has an empty hand");

    CHECK(fire_trigger(&tg, nozomi, RB_TSTR_LIVE_START), "nozomi: the live-start trigger fires");
    CHECK(!rb_has_pending_choice(&tg.state),
          "nozomi: an empty hand auto-skips the optional 2-discard cost, no prompt");
    drain_skip(&tg, 4);
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "nozomi: a declined cost means no look, so the deck is untouched");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "nozomi: a declined cost means no card joins the hand");
}

/* ══════════════════════════════════════════════════════════════════╁E   4. live_success_reveal_top_member_without_blade_heart_total_score_test.rs
      PL!-bp6-007-R+ (cost 11)
      ライブ�E功時 自刁E�EチE��キの一番上�Eカードを公開し、手札に�?える、E      それがブレードハートを持たなぁE��ンバ�Eカード�E場合、合計スコアを＋１、E   ══════════════════════════════════════════════════════════════════╁E*/

static void test_live_success_reveals_member_without_blade_heart_scores(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int source = test_id(&tg, BP6_007_R_FULLWIDTH_PLUS);
    int revealed = test_id(&tg, "PL!-sd1-001-SD");
    int unrelated = test_id(&tg, "PL!-sd1-001-SD");
    int live = test_id(&tg, "PL!-sd1-020-SD");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(source, BP6_007_R_FULLWIDTH_PLUS);
    pin_id(revealed, "PL!-sd1-001-SD");
    if (source < 0 || revealed < 0 || live < 0) return;
    /* printed score 2 for the live; +1 printed total-score bonus for a
       blade-heart-less member reveal */
    CHECK_EQ(card_printed_cost(source), 11, "bp6-007-R+ : the staged print costs 11");
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = source;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, unrelated);
    RbPlayer *P1 = &tg.state.p[0];
    P1->deck.n = 0;
    for (int i = 0; i < 30; i++) P1->deck.cards[P1->deck.n++] = filler;
    /* Rust: main_deck.cards.insert(5, revealed) - the reveal draws deck index 5. */
    for (int i = P1->deck.n; i > 5; i--) P1->deck.cards[i] = P1->deck.cards[i - 1];
    P1->deck.cards[5] = revealed;
    P1->deck.n++;
    RbPlayer *P2 = &tg.state.p[1];
    P2->deck.n = 0;
    for (int i = 0; i < 30; i++) P2->deck.cards[P2->deck.n++] = filler;

    for (int i = 0; i < 5; i++) test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET,
             "bp6-007-R+: five passes reach the live-card-set phase");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 5; i++) {
        test_pass(&tg);
        CHECK(!rb_has_pending_choice(&tg.state),
              "bp6-007-R+: the reveal-to-hand effect opens no choice");
    }
    CHECK(bag_has(&tg.state.p[0].success, live), "bp6-007-R+: the live card succeeded");
    CHECK(hand_has(&tg, revealed), "bp6-007-R+: the revealed top card reaches the hand");
    CHECK(hand_has(&tg, unrelated), "bp6-007-R+: the pre-existing hand member stays");
    CHECK(!deck_has(&tg, revealed), "bp6-007-R+: the revealed card left the deck");
    CHECK_EQ(tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -1, filler,
             "bp6-007-R+: the deck top is the filler again after the reveal");
    const RbLiveSnapshot *snap = NULL;
    for (int i = 0; i < tg.state.n_snapshots; i++)
        if (tg.state.snapshots[i].turn >= 0) snap = &tg.state.snapshots[i];
    CHECK(snap != NULL, "bp6-007-R+: a p1 performance snapshot exists");
    if (snap) {
        CHECK(snap->success, "bp6-007-R+: the p1 live succeeded");
        CHECK_EQ(snap->n_lives, 1, "bp6-007-R+: exactly one live was set");
        CHECK_EQ(snap->total_score, 2 + 1,
                 "bp6-007-R+: live score 2 plus the printed +1 blade-heart-less-member bonus");
    }
}

static void test_live_success_blade_heart_member_scores_nothing(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int source = test_id(&tg, BP6_007_R_FULLWIDTH_PLUS);
    int revealed = test_id(&tg, FILLER_MUS);        /* μ's cost 4: has a blade heart */
    int unrelated = test_id(&tg, "PL!-sd1-001-SD"); /* a qualifying member, but in HAND */
    int live = test_id(&tg, "PL!-sd1-020-SD");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(revealed, FILLER_MUS);
    if (source < 0 || revealed < 0 || live < 0) return;
    tg.state.p[0].stage[1] = source;
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, unrelated);
    RbPlayer *P1 = &tg.state.p[0];
    P1->deck.n = 0;
    for (int i = 0; i < 30; i++) P1->deck.cards[P1->deck.n++] = filler;
    for (int i = P1->deck.n; i > 5; i--) P1->deck.cards[i] = P1->deck.cards[i - 1];
    P1->deck.cards[5] = revealed;
    P1->deck.n++;
    RbPlayer *P2 = &tg.state.p[1];
    P2->deck.n = 0;
    for (int i = 0; i < 30; i++) P2->deck.cards[P2->deck.n++] = filler;

    for (int i = 0; i < 5; i++) test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET, "bp6-007-R+: reached the live-card-set phase");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 5; i++) { test_pass(&tg); drain_pick0(&tg, 4); }
    CHECK(hand_has(&tg, revealed), "bp6-007-R+: a blade-heart member is still revealed to hand");
    const RbLiveSnapshot *snap = NULL;
    for (int i = 0; i < tg.state.n_snapshots; i++)
        if (tg.state.snapshots[i].turn >= 0) snap = &tg.state.snapshots[i];
    CHECK(snap != NULL, "bp6-007-R+: a p1 performance snapshot exists");
    if (snap)
        CHECK_EQ(snap->total_score, 2,
                 "bp6-007-R+: a blade-heart member gets NO bonus even though a qualifying member sits in hand");
}

static void test_live_success_live_card_scores_nothing(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int source = test_id(&tg, BP6_007_R_FULLWIDTH_PLUS);
    int revealed = test_id(&tg, "PL!N-sd1-019-SD");  /* a LIVE, not a member */
    int unrelated = test_id(&tg, "PL!-sd1-001-SD");
    int live = test_id(&tg, "PL!-sd1-020-SD");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(revealed, "PL!N-sd1-019-SD");
    if (source < 0 || revealed < 0 || live < 0) return;
    tg.state.p[0].stage[1] = source;
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, unrelated);
    RbPlayer *P1 = &tg.state.p[0];
    P1->deck.n = 0;
    for (int i = 0; i < 30; i++) P1->deck.cards[P1->deck.n++] = filler;
    for (int i = P1->deck.n; i > 5; i--) P1->deck.cards[i] = P1->deck.cards[i - 1];
    P1->deck.cards[5] = revealed;
    P1->deck.n++;
    RbPlayer *P2 = &tg.state.p[1];
    P2->deck.n = 0;
    for (int i = 0; i < 30; i++) P2->deck.cards[P2->deck.n++] = filler;

    for (int i = 0; i < 5; i++) test_pass(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_LIVE_SET, "bp6-007-R+: reached the live-card-set phase");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 5; i++) { test_pass(&tg); drain_pick0(&tg, 4); }
    CHECK(hand_has(&tg, revealed), "bp6-007-R+: a live card is still revealed to hand");
    const RbLiveSnapshot *snap = NULL;
    for (int i = 0; i < tg.state.n_snapshots; i++)
        if (tg.state.snapshots[i].turn >= 0) snap = &tg.state.snapshots[i];
    CHECK(snap != NULL, "bp6-007-R+: a p1 performance snapshot exists");
    if (snap)
        CHECK_EQ(snap->total_score, 2,
                 "bp6-007-R+: a revealed LIVE card gets no bonus (the print says member)");
}

static void test_p2_live_success_reveals_only_p2_top(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int source = test_id(&tg, BP6_007_R_FULLWIDTH_PLUS);
    int p1_revealed = test_id(&tg, "PL!-sd1-001-SD");
    int p2_revealed = test_new_id(&tg, "PL!-sd1-001-SD");
    int p2_live = test_new_id(&tg, "PL!-sd1-020-SD");
    int filler = test_id(&tg, FILLER_MUS);
    pin_id(source, BP6_007_R_FULLWIDTH_PLUS);
    if (source < 0 || p2_revealed < 0 || p2_live < 0) return;
    tg.state.p[1].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[1] = source;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
    RbPlayer *P1 = &tg.state.p[0];
    P1->deck.n = 0;
    P1->deck.cards[P1->deck.n++] = p1_revealed;
    for (int i = 0; i < 10; i++) P1->deck.cards[P1->deck.n++] = filler;
    RbPlayer *P2 = &tg.state.p[1];
    P2->deck.n = 0;
    P2->deck.cards[P2->deck.n++] = p2_revealed;
    for (int i = 0; i < 10; i++) P2->deck.cards[P2->deck.n++] = filler;
    RbPlayer *L2 = &tg.state.p[1];
    L2->live.n = 0;
    L2->live.cards[L2->live.n++] = p2_live;

    CHECK(fire_trigger_p2(&tg, source, RB_TSTR_LIVE_SUCCESS),
          "bp6-007-R+: the p2 live-success trigger fires");
    drain_pick0(&tg, 4);
    CHECK(hand_has_pl(&tg, 1, p2_revealed), "bp6-007-R+: p2's own top card is revealed to p2's hand");
    CHECK(!hand_has_pl(&tg, 0, p2_revealed), "bp6-007-R+: p2's reveal never lands in p1's hand");
    CHECK(!hand_has_pl(&tg, 0, p1_revealed), "bp6-007-R+: p1's own top card is NOT revealed");
    CHECK(!bag_has(&P2->deck, p2_revealed), "bp6-007-R+: the revealed card left p2's deck");
    CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 1,
             "bp6-007-R+: only p2 gains the printed +1 total-score bonus");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "bp6-007-R+: p1 gains no total-score bonus from p2's live success");
}

/* ══════════════════════════════════════════════════════════════════╁E   5. look_five_reveal_cost9_group_member_test.rs
      Three sibling prints with the same template (cost 4, 登場):
        PL!S-bp5-006-R  (Aqours)   target PL!S-pb1-005-PR   cost 15
        PL!N-bp5-009-R  (虹ヶ咲)     target PL!N-sd1-005-PRproteinbar cost 11
        PL!SP-bp5-008-R (Liella!)   target PL!SP-bp1-013-PR  cost 9
      4 branches each: take / decline-pick / no-eligible / declined-cost.

      UPSTREAM CARD_NO TYPO, documented rather than silently inherited:
      the Rust file asks for "PL!N-bp1-005-PRproteinbar", which does NOT exist
      in cards/cards.json (only PL!N-bp1-005-R / -P exist; the proteinbar cost-11
      虹ヶ咲 member is PL!N-sd1-005-PRproteinbar, which is also what
      tests/test_parity_recover_to_hand.c already stages). The port uses the
      sd1 print so the branch is exercised against a real 虹ヶ咲 cost>=9 member;
      the identity is pinned below.
   ══════════════════════════════════════════════════════════════════╁E*/

typedef struct {
    TestGame tg;
    int fodder;
    int looked[5];
    int n_looked;
} Cost9Fixture;

/* Stock the deck so the top n prints are `top_nos`, filler below, then play. */
static int setup_cost9_prints(const char *me_no, const char *const *top_nos, int n_top,
                              Cost9Fixture *fx)
{
    TestGame *tg = &fx->tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int me = test_id(tg, me_no);
    if (!pin_id(me, me_no) || me < 0) return 0;
    pin_cost(me, 4);
    int fodder = test_new_id(tg, FILLER_MUS);
    test_add_to_hand(tg, me);
    test_add_to_hand(tg, fodder);
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n_top && i < 5; i++) {
        int id = test_new_id(tg, top_nos[i]);
        fx->looked[fx->n_looked++] = id;
        P->deck.cards[P->deck.n++] = id;
    }
    while (P->deck.n < 40 && P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = fodder;
    test_give_energy(tg, 15);
    play_to_stage(tg, me, 1, 0);
    fx->fodder = fodder;
    return 1;
}

static int cost9_look_count(const Cost9Fixture *fx)
{
    int pool[8];
    return rb_looked_at_pool(0, pool, 8);
}

static void cost9_take_branch(const char *me_no, const char *target_no, int target_cost)
{
    Cost9Fixture fx;
    memset(&fx, 0, sizeof fx);
    const char *top[1] = { target_no };
    if (!setup_cost9_prints(me_no, top, 1, &fx)) return;
    int target = fx.looked[0];
    pin_id(target, target_no);
    pin_cost(target, target_cost);
    CHECK(rb_has_pending_choice(&fx.tg.state), "cost9 look: the optional discard cost is offered");
    check_choice_is(&fx.tg, "hand", 1, 1, "cost9 look: the cost gate prompts over the hand");
    pick(&fx.tg, 0);
    CHECK(rb_has_pending_choice(&fx.tg.state),
          "cost9 look: the looked_at reveal prompt follows the paid cost");
    check_choice_is(&fx.tg, "looked_at", 1, 1, "cost9 look: the reveal prompt is looked_at/1/skippable");
    CHECK_EQ(cost9_look_count(&fx), 5, "cost9 look: five cards are in the looked_at pool");
    pick(&fx.tg, 0);
    CHECK(hand_has(&fx.tg, target),
          "cost9 look: the qualifying cost>=9 group member is revealed to hand");
}

static void cost9_decline_pick_branch(const char *me_no, const char *target_no)
{
    Cost9Fixture fx;
    memset(&fx, 0, sizeof fx);
    const char *top[5] = { target_no, FILLER_MUS, FILLER_MUS, FILLER_MUS, FILLER_MUS };
    if (!setup_cost9_prints(me_no, top, 5, &fx)) return;
    int pool[8];
    CHECK(rb_has_pending_choice(&fx.tg.state), "cost9 look: the cost gate is offered");
    pick(&fx.tg, 0);
    check_choice_is(&fx.tg, "looked_at", 1, 1, "cost9 look: the reveal prompt is offered after paying");
    /* Read the pool AFTER paying the cost. The look runs as part of the paid
       effect, so before `pick(&fx.tg, 0)` nothing has been looked at yet.
       Rust twin, look_five_reveal_cost9_group_member_test.rs:116-119: assert
       the "hand" cost prompt, select_indices(&[0]), assert the "looked_at"
       prompt, and only THEN compare state.looked_at_cards against the five
       looked ids. */
    int n_pool = rb_looked_at_pool(0, pool, 8);
    CHECK_EQ(n_pool, 5, "cost9 look: all five looked-at cards are in the pool");
    skip(&fx.tg);
    CHECK(!rb_has_pending_choice(&fx.tg.state), "cost9 look: declining the pick closes the prompt");
    CHECK_EQ(fx.tg.state.p[0].hand.n, 0, "cost9 look: a declined pick takes nothing to hand");
    /* cost card + all five looked cards land in the waitroom */
    check_bag_count(&fx.tg.state.p[0].discard, 6,
                    "cost9 look: the cost card plus all five looked cards go to the waitroom");
    CHECK(wait_has(&fx.tg, fx.fodder), "cost9 look: the paid cost card is in the waitroom");
    for (int i = 0; i < fx.n_looked; i++)
        CHECK(wait_has(&fx.tg, fx.looked[i]), "cost9 look: a declined pick discards the whole remainder");
    CHECK_EQ(fx.tg.state.p[0].deck.n, 35, "cost9 look: exactly five cards left the 40-card deck");
}

static void cost9_no_eligible_branch(const char *me_no)
{
    Cost9Fixture fx;
    memset(&fx, 0, sizeof fx);
    const char *top[5] = { FILLER_MUS, FILLER_MUS, FILLER_MUS, FILLER_MUS, FILLER_MUS };
    if (!setup_cost9_prints(me_no, top, 5, &fx)) return;
    CHECK(rb_has_pending_choice(&fx.tg.state), "cost9 look: the cost gate is offered");
    pick(&fx.tg, 0);
    CHECK(!rb_has_pending_choice(&fx.tg.state),
          "cost9 look: no eligible card means the look auto-skips without a prompt");
    CHECK_EQ(fx.tg.state.p[0].hand.n, 0, "cost9 look: nothing is taken when nothing qualifies");
    check_bag_count(&fx.tg.state.p[0].discard, 6,
                    "cost9 look: the cost card plus all five non-matching cards hit the waitroom");
    CHECK_EQ(fx.tg.state.p[0].deck.n, 35, "cost9 look: exactly five cards left the 40-card deck");
}

static void cost9_declined_cost_branch(const char *me_no)
{
    TestGame tg2;
    test_game_new(&tg2);
    clear_p1(&tg2);
    clear_p2(&tg2);
    int me2 = test_id(&tg2, me_no);
    if (!pin_id(me2, me_no) || me2 < 0) return;
    int fodder2 = test_new_id(&tg2, FILLER_MUS);
    test_add_to_hand(&tg2, me2);
    test_add_to_hand(&tg2, fodder2);
    RbPlayer *P = &tg2.state.p[0];
    P->deck.n = 0;
    while (P->deck.n < 40 && P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = fodder2;
    test_give_energy(&tg2, 15);
    int deck2 = P->deck.n;
    play_to_stage(&tg2, me2, 1, 0);
    CHECK(rb_has_pending_choice(&tg2.state), "cost9 look: the bundled cost prompt appears");
    check_choice_is(&tg2, "hand", 1, 1, "cost9 look: the bundled rest+discard cost prompts over the hand");
    skip(&tg2);
    CHECK(!rb_has_pending_choice(&tg2.state),
          "cost9 look: a skipped cost means the effect never starts, no look prompt");
    int hand_expected[1] = { fodder2 };
    check_bag_is(&tg2.state.p[0].hand, hand_expected, 1,
                 "cost9 look: a skipped cost leaves only the fodder in hand");
    CHECK(!is_wait(&tg2, me2), "cost9 look: a skipped rest cost leaves the member active");
    CHECK_EQ(tg2.state.p[0].discard.n, 0, "cost9 look: a skipped cost discards nothing");
    CHECK_EQ(tg2.state.p[0].deck.n, deck2, "cost9 look: a skipped cost leaves the deck untouched");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "cost9 look: nothing was looked at");
}

#define COST9_ALL(me_no, target_no, target_cost) \
    do { \
        printf("--- cost9 %s take\n", me_no); cost9_take_branch(me_no, target_no, target_cost); \
        printf("--- cost9 %s decline_pick\n", me_no); cost9_decline_pick_branch(me_no, target_no); \
        printf("--- cost9 %s no_eligible\n", me_no); cost9_no_eligible_branch(me_no); \
        printf("--- cost9 %s declined_cost\n", me_no); cost9_declined_cost_branch(me_no); \
    } while (0)

static void test_cost9_look_five_matrix(void)
{
    COST9_ALL("PL!S-bp5-006-R", "PL!S-pb1-005-PR", 15);          /* Aqours */
    COST9_ALL("PL!N-bp5-009-R", "PL!N-sd1-005-PRproteinbar", 11);/* 虹ヶ咲 */
    COST9_ALL("PL!SP-bp5-008-R", "PL!SP-bp1-013-PR", 9);         /* Liella! */
}

/* ══════════════════════════════════════════════════════════════════╁E   6. look_four_take_live_with_heart_requirement_test.rs
      PL!S-pb1-013-N (黒澤ダイヤ, cost 4) - look 4, take a member with
      heart04 x2+ OR a live needing heart04 x2+. Optional hand-discard cost.
      PL!S-pb1-014-N - the heart02 twin.
      The EFFECT-LEVEL decode of 013-N is already in
      test_parity_look_reveal.c (test_real_decoded_dia_no_match,
      test_real_decoded_dia_live_heart04). What is NOT covered anywhere is
      the 登場 trigger + optional hand-discard cost gate, so that is what this
      file adds.
   ══════════════════════════════════════════════════════════════════╁E*/

/* PL!-sd1-001-SD: base_heart {heart01:1, heart03:2, heart06:1} - no heart04
   at all, so it genuinely fails �u�n�[�g��heart04��2�ȏ㎝�����o�[�J�[�h�v.
   The Rust twin uses PL!N-sd1-010-SD here, but cards.json gives that card
   base_heart {heart01:1, heart03:1, heart04:2} - it carries heart04 TWICE and
   therefore IS selectable. See the section header above. */
#define DIA_PLAIN "PL!-sd1-001-SD"

static void dia_fire_debut_accept(TestGame *tg, int me)
{
    CHECK(fire_trigger(tg, me, RB_TSTR_DEBUT), "dia: the 登場 trigger fires");
    CHECK(rb_has_pending_choice(&tg->state), "dia: the optional discard cost is prompted");
    CHECK(pending_type_is(tg, "SelectCard"), "dia: the cost gate is a SelectCard");
    check_choice_is(tg, "hand", 1, 1, "dia: the cost gate prompts over the hand and allows skip");
    pick(tg, 0);
    drain_pick0(tg, 10);
}

static void test_dia_look_four_takes_live_requiring_two_heart04(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int filler = test_new_id(&tg, DIA_PLAIN);
    fill_decks(&tg, filler);
    int me = test_id(&tg, "PL!S-pb1-013-N");
    pin_id(me, "PL!S-pb1-013-N");
    if (me < 0) return;
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, DIA_PLAIN));
    int seed = test_new_id(&tg, "PL!HS-bp2-020-L");
    pin_id(seed, "PL!HS-bp2-020-L");
    put_on_deck_top(&tg, 0, seed);
    dia_fire_debut_accept(&tg, me);
    CHECK(hand_has(&tg, seed), "dia: a live needing heart04 x2 matches the OR filter and is taken");
}

static void test_dia_look_four_excludes_plain_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int filler = test_new_id(&tg, DIA_PLAIN);
    fill_decks(&tg, filler);
    int me = test_id(&tg, "PL!S-pb1-013-N");
    pin_id(me, "PL!S-pb1-013-N");
    if (me < 0) return;
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!N-sd1-010-SD"));
    int plain = test_new_id(&tg, DIA_PLAIN);
    put_on_deck_top(&tg, 0, plain);
    dia_fire_debut_accept(&tg, me);
    CHECK(!hand_has(&tg, plain), "dia: a member without the heart04 tie is NOT taken");
    CHECK(wait_has(&tg, plain), "dia: the non-matching looked card goes to the waitroom");
}

static void test_dia014_look_four_takes_live_requiring_heart02(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int filler = test_new_id(&tg, DIA_PLAIN);
    fill_decks(&tg, filler);
    int me = test_id(&tg, "PL!S-pb1-014-N");
    pin_id(me, "PL!S-pb1-014-N");
    if (me < 0) return;
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, DIA_PLAIN));
    int seed = test_new_id(&tg, "PL!SP-pb1-023-L");
    pin_id(seed, "PL!SP-pb1-023-L");
    put_on_deck_top(&tg, 0, seed);
    dia_fire_debut_accept(&tg, me);
    CHECK(hand_has(&tg, seed), "dia014: a live needing heart02 x2 matches the OR filter and is taken");
}

/* ══════════════════════════════════════════════════════════════════╁E   7. optional_hand_discard_cost_look_three_take_one_test.rs
      PL!-sd1-011-SD (絢瀬絵釁E cost 4)
      登場 手札めE枚控え室に置ぁE��もよぁE��E枚見る -> 1枚を手札に�?え、E      残りを控え室に置く、E   ══════════════════════════════════════════════════════════════════╁E*/

static void test_eli_cost_look_three_takes_one_to_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int a = test_id(&tg, FILLER_MUS);
    int b = test_id(&tg, "PL!-sd1-020-SD");
    int c = test_id(&tg, "PL!-sd1-014-SD");
    int d = test_id(&tg, "PL!-sd1-015-SD");
    int eli = test_id(&tg, "PL!-sd1-011-SD");
    pin_id(eli, "PL!-sd1-011-SD");
    pin_cost(eli, 4);
    if (a < 0 || b < 0 || c < 0 || d < 0 || eli < 0) return;
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = a;
    P->deck.cards[P->deck.n++] = b;
    P->deck.cards[P->deck.n++] = c;
    P->deck.cards[P->deck.n++] = d;
    int opponent = test_id(&tg, FILLER_MUS);
    RbPlayer *O = &tg.state.p[1];
    O->deck.n = 0;
    O->deck.cards[O->deck.n++] = opponent;
    int cost = test_new_id(&tg, "PL!-sd1-020-SD");
    /* The Rust twin writes `game.id("PL!-sd1-020-SD")` for BOTH the looked card
       `b` and the cost card, and Rust's id() returns the shared template index,
       so its fixture puts ONE id in the deck AND in the hand. A card cannot be
       in two places; the cost card is allocated as a distinct instance here. */
    CHECK(cost != b, "eli: the cost card is a distinct instance from the looked card b");
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, cost);
    test_give_energy(&tg, 12);
    play_to_stage(&tg, eli, 1, 0);
    check_choice_is(&tg, "hand", 1, 1, "eli: the cost gate prompts over the hand");
    pick(&tg, 0);
    CHECK(stage_has(&tg, eli), "eli: the source member stays on stage (only the cost card leaves)");
    CHECK(!hand_has(&tg, cost), "eli: the cost card left the hand");
    check_choice_is(&tg, "looked_at", 1, 0, "eli: the look prompt is looked_at/1 and NOT skippable");
    int pool[8];
    int n_pool = rb_looked_at_pool(0, pool, 8);
    CHECK_EQ(n_pool, 3, "eli: three cards are in the looked_at pool");
    CHECK(n_pool == 3 && pool[0] == a && pool[1] == b && pool[2] == c,
          "eli: the looked_at pool is the deck top three in order");
    pick(&tg, 0);
    CHECK(!rb_has_pending_choice(&tg.state), "eli: no prompt survives the pick");
    CHECK(hand_has(&tg, a), "eli: the selected card is added to the hand");
    int wait_expected[3] = { cost, b, c };
    check_bag_is(&tg.state.p[0].discard, wait_expected, 3,
                 "eli: the cost card plus the unselected remainder hit the waitroom in order");
    int deck_expected[1] = { d };
    check_bag_is(&tg.state.p[0].deck, deck_expected, 1,
                 "eli: the deck tail below the looked-at three is untouched");
    int p2_expected[1] = { opponent };
    check_bag_is(&tg.state.p[1].deck, p2_expected, 1, "eli: the opponent deck is untouched");
}

static void test_eli_declined_cost_skips_look_entirely(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int cards[4];
    for (int i = 0; i < 4; i++) cards[i] = test_id(&tg, FILLER_MUS);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = cards[i];
    int opponent = test_id(&tg, FILLER_MUS);
    RbPlayer *O = &tg.state.p[1];
    O->deck.n = 0;
    O->deck.cards[O->deck.n++] = opponent;
    int eli = test_id(&tg, "PL!-sd1-011-SD");
    int cost = test_new_id(&tg, "PL!-sd1-020-SD");
    pin_id(eli, "PL!-sd1-011-SD");
    if (eli < 0) return;
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, cost);
    test_give_energy(&tg, 12);
    play_to_stage(&tg, eli, 1, 0);
    check_choice_is(&tg, "hand", 1, 1, "eli: the cost gate prompts over the hand");
    skip(&tg);
    CHECK(!rb_has_pending_choice(&tg.state), "eli: a declined cost means the effect never starts");
    int hand_expected[1] = { cost };
    check_bag_is(&tg.state.p[0].hand, hand_expected, 1,
                 "eli: a declined cost leaves the cost card alone in hand");
    CHECK(stage_has(&tg, eli), "eli: a declined cost leaves the member on stage; nothing else moved");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "eli: a declined cost discards nothing");
    check_bag_is(&tg.state.p[0].deck, cards, 4, "eli: a declined cost leaves the whole deck in place");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "eli: a declined cost looks at nothing");
    int p2_expected[1] = { opponent };
    check_bag_is(&tg.state.p[1].deck, p2_expected, 1, "eli: the opponent deck is untouched");
}

/* ══════════════════════════════════════════════════════════════════╁E   8. optional_live_discard_look_five_take_one_test.rs
      PL!SP-bp7-018-N (梁E��极E綴, cost 4)
      登場 手札のライブカードを1枚控え室に置ぁE��もよぁE��E枚見る ->
      1枚を手札に�?え、残りを控え室に置く、E   ══════════════════════════════════════════════════════════════════╁E*/

static void test_mei_paid_live_discard_looks_five_and_takes_one(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!SP-bp7-018-N");
    pin_id(me, "PL!SP-bp7-018-N");
    pin_cost(me, 4);
    if (me < 0) return;
    int live_cost = test_new_id(&tg, "PL!-sd1-019-SD");
    pin_id(live_cost, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, me);
    test_add_to_hand(&tg, live_cost);
    int a = test_new_id(&tg, "PL!S-sd1-001-SD");
    int b = test_new_id(&tg, "PL!-sd1-019-SD");
    pin_id(a, "PL!S-sd1-001-SD");
    test_give_energy(&tg, 20);
    put_on_deck_top(&tg, 0, b);
    put_on_deck_top(&tg, 0, a);
    play_to_stage(&tg, me, 0, 0);
    drain_pick0(&tg, 10);
    CHECK(wait_has(&tg, live_cost), "mei: the paid live cost card is discarded to the waitroom");
    CHECK(hand_has(&tg, a), "mei: one looked card is added to the hand");
}

static void test_mei_declined_live_discard_preserves_cost_card(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!SP-bp7-018-N");
    pin_id(me, "PL!SP-bp7-018-N");
    if (me < 0) return;
    int live_cost = test_new_id(&tg, "PL!-sd1-019-SD");
    test_add_to_hand(&tg, me);
    test_add_to_hand(&tg, live_cost);
    test_give_energy(&tg, 20);
    int deck_before = tg.state.p[0].deck.n;
    play_to_stage(&tg, me, 0, 0);
    CHECK(rb_has_pending_choice(&tg.state), "mei: the optional live-discard cost is prompted");
    skip(&tg);
    drain_skip(&tg, 6);
    CHECK(hand_has(&tg, live_cost), "mei: a declined live-discard cost stays in hand");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "mei: a declined cost performs no look");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "mei: a declined cost discards nothing");
}

/* ══════════════════════════════════════════════════════════════════╁E   9. paid_discard_filtered_look_reveal_to_hand_test.rs
   10. paid_discard_look_reveal_group_card_test.rs
   11. paid_discard_look_two_reveal_one_to_hand_test.rs
   These three are one shared shape: OPTIONAL hand-discard cost, then a
   filtered look, then reveal ONE card from the looked set to hand. They are
   parameterised here by (print, look count, target print, group/type) so each
   distinct filter is covered without three near-duplicate copies.
   ══════════════════════════════════════════════════════════════════╁E*/

typedef struct {
    const char *me_no;
    int         me_cost;
    const char *target_no;
    int         target_cost;   /* -1 = don't assert */
    int         look_allow_skip; /* the looked_at pick: 1 for �u?�Ă��悢�v, 0 for mandatory */
    const char *what;
} PaidLookCase;

static void paid_discard_filtered_look(const PaidLookCase *c)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int me = test_id(&tg, c->me_no);
    pin_id(me, c->me_no);
    if (c->me_cost > 0) pin_cost(me, c->me_cost);
    if (me < 0) return;
    int fodder = test_new_id(&tg, FILLER_MUS);
    test_add_to_hand(&tg, me);
    test_add_to_hand(&tg, fodder);
    int target = test_id(&tg, c->target_no);
    pin_id(target, c->target_no);
    if (c->target_cost > 0) pin_cost(target, c->target_cost);
    if (target < 0) return;
    int top[1] = { target };
    stock_deck_top(&tg, top, 1, fodder);
    test_give_energy(&tg, 15);
    play_to_stage(&tg, me, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "paid look: the optional discard cost is offered");
    check_choice_is(&tg, "hand", 1, 1, "paid look: the cost gate prompts over the hand");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "paid look: the reveal prompt follows the paid cost");
    check_choice_is(&tg, "looked_at", 1, c->look_allow_skip, "paid look: the reveal prompt is looked_at/1");
    pick(&tg, 0);
    CHECK(hand_has(&tg, target), "paid look: the filtered card is revealed from the looked set to hand");
    CHECK(wait_has(&tg, fodder), "paid look: the paid cost card is in the waitroom");
    printf("-- paid look case: %s\n", c->what);
}

static void test_paid_discard_filtered_looks(void)
{
    /* PL!N-bp3-012-R 登場 ...4枚見る...『虹ヶ咲』�Eカードを1枚�E閁E*/
    { PaidLookCase c = { "PL!N-bp3-012-R", 4, "PL!N-bp3-004-R", 13, 1,
                         "niji bp3-012: look 4, take a 虹ヶ咲 card" };
      paid_discard_filtered_look(&c); }
    /* PL!HS-bp1-011-PR 登場 ...5枚見る...ライブカードを1枚�E閁E*/
    { PaidLookCase c = { "PL!HS-bp1-011-PR", 9, "PL!HS-sd1-020-SD", -1, 1,
                         "hasunosora bp1-011: look 5, take a live card" };
      paid_discard_filtered_look(&c); }
    /* PL!SP-pb1-015-N 登場 ...5枚見る...『CatChu!』�Eカードを1枚�E閁E*/
    { PaidLookCase c = { "PL!SP-pb1-015-N", 4, "PL!SP-bp1-004-PR", 15, 1,
                         "catchu pb1-015: look 5, take a CatChu! card" };
      paid_discard_filtered_look(&c); }
    /* PL!SP-pb1-016-N 登場 ...5枚見る...『KALEIDOSCORE』�Eカードを1枚�E閁E*/
    { PaidLookCase c = { "PL!SP-pb1-016-N", 4, "PL!SP-bp1-013-PR", 9, 1,
                         "kaleidoscore pb1-016: look 5, take a KALEIDOSCORE card" };
      paid_discard_filtered_look(&c); }
    /* PL!N-sd2-012-SD2 登場 ...3枚見る...『虹ヶ咲』�Eカードを1枚�E閁E*/
    { PaidLookCase c = { "PL!N-sd2-012-SD2", 4, "PL!N-bp3-004-R", 13, 1,
                         "niji sd2-012: look 3, take a 虹ヶ咲 card" };
      paid_discard_filtered_look(&c); }
    /* PL!N-pb1-028-N 登場 ...2枚見る...1枚を手札に�?ぁE(no group filter) */
    /* PL!N-pb1-028-N �o�� ...2������...1������D�ɉ���. The printed text carries
       no �u�Ă��悢�v on the look, so the looked_at pick is MANDATORY; only the
       discard cost is optional. Rust pins allow_skip=false at
       paid_discard_look_two_reveal_one_to_hand_test.rs:47. */
    { PaidLookCase c = { "PL!N-pb1-028-N", 4, FILLER_MUS, 4, 0,
                         "niji pb1-028: look 2, add the top card to hand (mandatory pick)" };
      paid_discard_filtered_look(&c); }
}

/* ══════════════════════════════════════════════════════════════════╁E   12. sequential_cost_look_five_take_cost9_member_test.rs
       PL!-bp5-002-R (絢瀬絵釁E cost 4) - SEQUENTIAL cost
       こ�Eメンバ�Eをウェイトにし、手札めE枚控え室に置ぁE��もよぁE��E       5枚見る -> コスチE以上�E『μ's』�Eメンバ�Eカードを1枚�E開して手札に、E   ══════════════════════════════════════════════════════════════════╁E*/

static void test_eli_bp5_sequential_wait_then_discard(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int eli = test_id(&tg, "PL!-bp5-002-R");
    pin_id(eli, "PL!-bp5-002-R");
    pin_cost(eli, 4);
    if (eli < 0) return;
    int mus_high = test_id(&tg, "PL!-bp3-002-R");
    pin_id(mus_high, "PL!-bp3-002-R");
    pin_cost(mus_high, 9);
    if (mus_high < 0) return;
    int filler = test_id(&tg, FILLER_MUS);
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 9);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = mus_high;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    play_to_stage(&tg, eli, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "eli bp5: the discard prompt appears after the auto-paid wait");
    check_choice_is(&tg, "hand", 1, 1, "eli bp5: the prompt is a hand discard, not a look");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "eli bp5: the looked_at prompt follows the hand discard");
    check_choice_is(&tg, "looked_at", 1, 1, "eli bp5: the looked_at pick is a SelectCard");
    pick(&tg, 0);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "eli bp5: hand = 1 filler after the cost, +1 revealed");
    CHECK(hand_has(&tg, mus_high), "eli bp5: the cost-9 μ's member is revealed to hand");
    CHECK(is_wait(&tg, eli), "eli bp5: the bundled rest cost is paid, the member is wait");
}

static void test_eli_bp5_skip_costs_skips_look_entirely(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int eli = test_id(&tg, "PL!-bp5-002-R");
    int mus_high = test_id(&tg, "PL!-bp3-002-R");
    pin_id(eli, "PL!-bp5-002-R");
    if (eli < 0) return;
    int filler = test_id(&tg, FILLER_MUS);
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 9);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = mus_high;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    int deck_before = P->deck.n;
    play_to_stage(&tg, eli, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "eli bp5: the cost prompt is expected");
    skip(&tg);
    CHECK(!rb_has_pending_choice(&tg.state),
          "eli bp5: a skipped cost means the effect never starts, no look prompt");
    int hand_expected[1] = { filler };
    check_bag_is(&tg.state.p[0].hand, hand_expected, 1,
                 "eli bp5: a skipped cost leaves only the filler in hand");
    CHECK(!is_wait(&tg, eli), "eli bp5: a skipped rest cost leaves the member active");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "eli bp5: a skipped cost discards nothing");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "eli bp5: a skipped cost leaves the deck untouched");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "eli bp5: a skipped cost looks at nothing");
}

static void test_eli_bp5_no_eligible_look_discards_all(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int eli = test_id(&tg, "PL!-bp5-002-R");
    pin_id(eli, "PL!-bp5-002-R");
    if (eli < 0) return;
    int filler = test_id(&tg, FILLER_MUS);
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 9);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 10; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    int wait_before = P->discard.n;
    play_to_stage(&tg, eli, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "eli bp5: the cost prompt is expected");
    pick(&tg, 0);
    drain_skip(&tg, 5);
    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "eli bp5: the hand ends empty - the only filler was the look's own cost card");
    CHECK_EQ(tg.state.p[0].discard.n - wait_before, 6,
             "eli bp5: exactly the look cost card plus all 5 non-eligible looked cards");
}

static void test_eli_bp5_look_select_optional_skip_keeps_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int eli = test_id(&tg, "PL!-bp5-002-R");
    int mus_high = test_id(&tg, "PL!-bp3-002-R");
    pin_id(eli, "PL!-bp5-002-R");
    if (eli < 0) return;
    int filler = test_id(&tg, FILLER_MUS);
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 9);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < 2; i++) P->deck.cards[P->deck.n++] = mus_high;  /* eligible */
    for (int i = 0; i < 3; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = filler;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    play_to_stage(&tg, eli, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "eli bp5: the cost prompt is expected");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state),
          "eli bp5: an eligible card among the looked five still opens the optional pick");
    drain_skip(&tg, 5);
    CHECK(!hand_has(&tg, mus_high),
          "eli bp5: skipping the optional select takes no looked card even though one is eligible");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "eli bp5: the hand ends empty after the skip");
}

/* ══════════════════════════════════════════════════════════════════╁E   13. sequential_cost_look_five_take_group_member_test.rs
       PL!HS-bp5-008-R (桂城 況E cost 4) - the same sequential_cost template
       with a 『蓮ノ空、Egroup. 5 branches.
   ══════════════════════════════════════════════════════════════════╁E*/

static int izumi_setup(TestGame *tg, int n_eligible, int eligible_is_first)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int izumi = test_id(tg, "PL!HS-bp5-008-R");
    pin_id(izumi, "PL!HS-bp5-008-R");
    pin_cost(izumi, 4);
    if (izumi < 0) return -1;
    int filler = test_id(tg, FILLER_MUS);
    int hasu9 = test_id(tg, "PL!HS-sd1-001-SD");
    pin_id(hasu9, "PL!HS-sd1-001-SD");
    pin_cost(hasu9, 9);
    if (hasu9 < 0) return -1;
    test_add_to_hand(tg, izumi);
    test_add_to_hand(tg, filler);
    test_give_energy(tg, 4);
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    if (n_eligible <= 0) {
        for (int i = 0; i < 10; i++) P->deck.cards[P->deck.n++] = filler;
    } else if (eligible_is_first) {
        for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;
        P->deck.cards[P->deck.n++] = hasu9;
        for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = filler;
    } else {
        for (int i = 0; i < n_eligible; i++) P->deck.cards[P->deck.n++] = hasu9;
        for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = filler;
    }
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
    return izumi;
}

static void test_izumi_pay_cost_waits_and_discards(void)
{
    TestGame tg;
    int izumi = izumi_setup(&tg, 0, 0);
    if (izumi < 0) return;
    int wait_before = tg.state.p[0].discard.n;
    play_to_stage(&tg, izumi, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the combined cost prompt appears");
    check_choice_is(&tg, "hand", 1, 1, "izumi: the combined prompt is a hand discard");
    pick(&tg, 0);
    CHECK(is_wait(&tg, izumi), "izumi: the wait is applied after the cost is paid");
    CHECK(!rb_has_pending_choice(&tg.state),
          "izumi: no prompt remains after the look auto-skips (no matching card)");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "izumi: the hand is empty after the discard");
    CHECK(tg.state.p[0].discard.n > wait_before, "izumi: cards reached the waitroom");
}

static void test_izumi_skip_cost_no_wait_no_discard(void)
{
    TestGame tg;
    int izumi = izumi_setup(&tg, 0, 0);
    if (izumi < 0) return;
    play_to_stage(&tg, izumi, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the combined cost prompt appears");
    skip(&tg);
    CHECK(!is_wait(&tg, izumi), "izumi: no wait is applied when the cost is skipped");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "izumi: the hand still holds the filler");
}

static void test_izumi_look_with_eligible_selects(void)
{
    TestGame tg;
    int izumi = izumi_setup(&tg, 1, 1);
    if (izumi < 0) return;
    int hasu9 = test_id(&tg, "PL!HS-sd1-001-SD");
    play_to_stage(&tg, izumi, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the cost prompt appears");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the look select appears when an eligible is present");
    check_choice_is(&tg, "looked_at", 1, 1, "izumi: the look select is a SelectCard over looked_at");
    pick(&tg, 0);
    CHECK(hand_has(&tg, hasu9), "izumi: the eligible 蓮ノ空 cost-9 member reaches the hand");
    CHECK(!rb_has_pending_choice(&tg.state), "izumi: no prompt after the select");
}

static void test_izumi_look_with_eligible_skip_keeps_hand(void)
{
    TestGame tg;
    int izumi = izumi_setup(&tg, 1, 1);
    if (izumi < 0) return;
    int hasu9 = test_id(&tg, "PL!HS-sd1-001-SD");
    play_to_stage(&tg, izumi, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the cost prompt appears");
    pick(&tg, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the look select appears when an eligible is present");
    skip(&tg);
    CHECK(!hand_has(&tg, hasu9), "izumi: a skipped select does not add the eligible card");
    CHECK(!rb_has_pending_choice(&tg.state), "izumi: no prompt after the skip");
}

static void test_izumi_look_no_eligible_auto_discards(void)
{
    TestGame tg;
    int izumi = izumi_setup(&tg, 0, 0);
    if (izumi < 0) return;
    int wait_before = tg.state.p[0].discard.n;
    play_to_stage(&tg, izumi, 1, 0);
    CHECK(rb_has_pending_choice(&tg.state), "izumi: the cost prompt appears");
    pick(&tg, 0);
    drain_skip(&tg, 5);
    CHECK(!rb_has_pending_choice(&tg.state), "izumi: no eligible means the look auto-ends");
    CHECK_EQ(tg.state.p[0].discard.n - wait_before, 6,
             "izumi: exactly the look's own discard plus all 5 looked-at cards");
}

/* ══════════════════════════════════════════════════════════════════╁E   14. waitroom_debut_look_three_take_one_test.rs
       PL!S-bp6-016-N (�E��E本 4)
       登場 控え室から登場してぁE��場合、デチE��上かめE枚を見る、E       そ�E中から1枚を手札に�?え、残りを控え室に置く、E       The whole card is gated on the appearance SOURCE ZONE.
   ══════════════════════════════════════════════════════════════════╁E*/

static void test_sbp6_016_waitroom_debut_looks_three_takes_one(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int me = test_id(&tg, "PL!S-bp6-016-N");
    pin_id(me, "PL!S-bp6-016-N");
    pin_cost(me, 4);
    if (me < 0) return;
    int a = test_new_id(&tg, FILLER_MUS);
    int b = test_new_id(&tg, "PL!S-sd1-001-SD");
    int c = test_new_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    put_on_deck_top(&tg, 0, c);
    put_on_deck_top(&tg, 0, b);
    put_on_deck_top(&tg, 0, a);
    tg.state.p[0].stage[0] = me;
    rb_record_card_appearance(&tg.state, me, 3 /* waitroom / discard zone */);
    CHECK(fire_trigger(&tg, me, RB_TSTR_DEBUT), "bp6-016: the 登場 trigger fires");
    CHECK(rb_has_pending_choice(&tg.state), "bp6-016: a waitroom debut opens the look prompt");
    pick(&tg, 0);
    int in_hand = 0, in_wait = 0, on_deck = 0;
    int seeds[3] = { a, b, c };
    for (int i = 0; i < 3; i++) {
        if (hand_has(&tg, seeds[i])) in_hand++;
        if (wait_has(&tg, seeds[i])) in_wait++;
        if (deck_has(&tg, seeds[i])) on_deck++;
    }
    CHECK_EQ(in_hand, 1, "bp6-016: exactly one looked card is added to the hand");
    CHECK_EQ(in_wait, 2, "bp6-016: the remaining two looked cards go to the waitroom");
    CHECK_EQ(on_deck, 0, "bp6-016: all three looked cards left the deck");
}

static void test_sbp6_016_hand_debut_no_look(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int me = test_id(&tg, "PL!S-bp6-016-N");
    pin_id(me, "PL!S-bp6-016-N");
    if (me < 0) return;
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    tg.state.p[0].stage[0] = me;
    rb_record_card_appearance(&tg.state, me, 1 /* hand zone */);
    CHECK(fire_trigger(&tg, me, RB_TSTR_DEBUT), "bp6-016: the 登場 trigger fires for a hand debut");
    CHECK(!rb_has_pending_choice(&tg.state), "bp6-016: a HAND debut must NOT open the look prompt");
}

static void test_sbp6_016_no_appearance_record_no_look(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int me = test_id(&tg, "PL!S-bp6-016-N");
    pin_id(me, "PL!S-bp6-016-N");
    if (me < 0) return;
    int filler = test_new_id(&tg, FILLER_MUS);
    fill_decks(&tg, filler);
    tg.state.p[0].stage[0] = me;
    CHECK(fire_trigger(&tg, me, RB_TSTR_DEBUT), "bp6-016: the 登場 trigger fires with no appearance record");
    CHECK(!rb_has_pending_choice(&tg.state), "bp6-016: an unrecorded debut opens no prompt");
}

static void test_sbp6_016_short_deck_takes_one_discards_remainder(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    int me = test_id(&tg, "PL!S-bp6-016-N");
    pin_id(me, "PL!S-bp6-016-N");
    if (me < 0) return;
    int b = test_new_id(&tg, "PL!S-sd1-001-SD");
    int c = test_new_id(&tg, "PL!N-sd1-025-SD");
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = b;
    P->deck.cards[P->deck.n++] = c;
    RbPlayer *O = &tg.state.p[1];
    O->deck.n = 0;
    test_give_energy(&tg, 10);
    tg.state.p[0].stage[0] = me;
    int total_before = test_total_card_count(&tg);
    rb_record_card_appearance(&tg.state, me, 3 /* waitroom */);
    CHECK(fire_trigger(&tg, me, RB_TSTR_DEBUT), "bp6-016: the �o�� trigger fires");
    CHECK(rb_has_pending_choice(&tg.state), "bp6-016: only two cards available, still prompted");
    pick(&tg, 0);
    int got_one = hand_has(&tg, b) || hand_has(&tg, c);
    CHECK(got_one, "bp6-016: one of the two available cards is taken");
    int other_in_wait = hand_has(&tg, b) ? wait_has(&tg, c) : wait_has(&tg, b);
    CHECK(other_in_wait, "bp6-016: the leftover card goes to the waitroom");
    CHECK_EQ(P->deck.n, 0, "bp6-016: the deck is fully consumed by the short look");
    /* Zone census: proves whether the leftover was mis-routed or destroyed. */
    CHECK_EQ(test_total_card_count(&tg), total_before,
             "bp6-016: the short look conserves every physical card");
}

/* ── runner ───────────────────────────────────────────────────────── */

/* RB_DUMP=<card_no>[,<card_no>...] decodes and prints every ability a print
   carries, so a red case can be diagnosed without guessing which of the many
   same-text prints is really being exercised. */
static void dump_effect(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("[%d] action=%s src=%s dst=%s target=%s count=%d opt=%s",
           depth, e->action ? e->action : "-", e->source ? e->source : "-",
           e->destination ? e->destination : "-", e->target ? e->target : "-",
           e->count, e->is_optional ? "1" : "0");
    for (int i = 0; i < e->n_extra; i++)
        printf(" [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?",
               e->extra_v[i] ? e->extra_v[i] : "?");
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
        printf("== %s -> id=%d resolved_card_no=%s cost=%d\n", tok, cid,
               card_no_of(cid), card_printed_cost(cid));
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

static void run_all(const char *only)
{
    struct { const char *name; void (*fn)(void); } cases[] = {
        /* 1. debut_group_look_three_reveal_test.rs */
        { "sd2_009_take",        test_debut_look_three_reveals_group_member },
        { "sd2_009_live",        test_debut_look_three_reveals_group_live },
        { "sd2_009_decline",     test_debut_look_three_decline_discards_all },
        { "sd2_009_no_group",    test_debut_look_three_no_group_card_no_prompt },
        /* 2. debut_look_liella_cost4_hand_or_debut_test.rs */
        { "kanon_keep",          test_kanon_select_liella_cost4_keep_in_hand },
        { "kanon_stage",         test_kanon_select_liella_debut_to_stage },
        { "kanon_skipcost",      test_kanon_skip_cost_effect_not_executed },
        { "kanon_nomatch",       test_kanon_no_matching_cards_discard_all },
        { "kanon_cost4",         test_kanon_cost_above_4_rejected },
        { "kanon_group",         test_kanon_non_liella_cost4_rejected },
        { "kanon_max1",          test_kanon_max_1_enforced },
        { "kanon_fullstage",     test_kanon_stage_full_falls_back_to_hand },
        /* 3. empty_hand_skips_optional_discard_look_test.rs */
        { "nozomi_emptyhand",    test_nozomi_empty_hand_skips_discard_cost_and_look },
        /* 4. live_success_reveal_top_member_without_blade_heart_total_score_test.rs */
        { "bp6_007_member",      test_live_success_reveals_member_without_blade_heart_scores },
        { "bp6_007_blade",       test_live_success_blade_heart_member_scores_nothing },
        { "bp6_007_livecard",    test_live_success_live_card_scores_nothing },
        { "bp6_007_p2only",      test_p2_live_success_reveals_only_p2_top },
        /* 5. look_five_reveal_cost9_group_member_test.rs */
        { "cost9_matrix",        test_cost9_look_five_matrix },
        /* 6. look_four_take_live_with_heart_requirement_test.rs */
        { "dia_live_heart04",    test_dia_look_four_takes_live_requiring_two_heart04 },
        { "dia_plain_member",    test_dia_look_four_excludes_plain_member },
        { "dia014_live_heart02", test_dia014_look_four_takes_live_requiring_heart02 },
        /* 7. optional_hand_discard_cost_look_three_take_one_test.rs */
        { "eli_take",            test_eli_cost_look_three_takes_one_to_hand },
        { "eli_skipcost",        test_eli_declined_cost_skips_look_entirely },
        /* 8. optional_live_discard_look_five_take_one_test.rs */
        { "mei_pay",             test_mei_paid_live_discard_looks_five_and_takes_one },
        { "mei_skip",            test_mei_declined_live_discard_preserves_cost_card },
        /* 9/10/11. paid-discard filtered look-reveal-to-hand family */
        { "paid_looks",          test_paid_discard_filtered_looks },
        /* 12. sequential_cost_look_five_take_cost9_member_test.rs */
        { "eli_bp5_pay",         test_eli_bp5_sequential_wait_then_discard },
        { "eli_bp5_skip",        test_eli_bp5_skip_costs_skips_look_entirely },
        { "eli_bp5_noelig",      test_eli_bp5_no_eligible_look_discards_all },
        { "eli_bp5_optskip",     test_eli_bp5_look_select_optional_skip_keeps_hand },
        /* 13. sequential_cost_look_five_take_group_member_test.rs */
        { "izumi_pay",           test_izumi_pay_cost_waits_and_discards },
        { "izumi_skip",          test_izumi_skip_cost_no_wait_no_discard },
        { "izumi_take",          test_izumi_look_with_eligible_selects },
        { "izumi_optskip",       test_izumi_look_with_eligible_skip_keeps_hand },
        { "izumi_noelig",        test_izumi_look_no_eligible_auto_discards },
        /* 14. waitroom_debut_look_three_take_one_test.rs */
        { "bp6_016_waitroom",    test_sbp6_016_waitroom_debut_looks_three_takes_one },
        { "bp6_016_hand",        test_sbp6_016_hand_debut_no_look },
        { "bp6_016_unrecorded",  test_sbp6_016_no_appearance_record_no_look },
        { "bp6_016_shortdeck",   test_sbp6_016_short_deck_takes_one_discards_remainder },
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
    { const char *d = getenv("RB_DUMP"); if (d && *d) { int r = run_dump(d); rb_unload(); return r; } }
    run_all(getenv("RB_ONLY"));    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL REVEAL EXTRA PARITY CHECKS PASSED\n");
    return 0;
}

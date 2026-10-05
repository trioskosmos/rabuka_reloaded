/* test_parity_selected_cards_overflow.c
 *
 * REGRESSION: `g->selected_cards[]` is `int[RB_MAX_RECENTLY_MOVED]` (8) --
 * rabuka.h:1648 -- but cost.c clamped its population to RB_MAX_ZONE (512).
 * A mandatory reveal cost whose eligible set is >= 9 cards therefore wrote up
 * to 512 ints into an 8-int field, running 2016 bytes past the end of the
 * array and corrupting every GameState member declared after it.
 *
 * The site (src/ability/cost.c, rb_pay_cost reveal arm, the
 * "handlers.rs:1260-1283" reveal-without-prompt fast path):
 *
 *     for (int i = 0; i < n_ids && i < RB_MAX_ZONE; i++)
 *         g->selected_cards[i] = card_ids[i];
 *     g->n_selected_cards = n_ids < RB_MAX_ZONE ? n_ids : RB_MAX_ZONE;
 *
 * `n_selected_cards` is the very next member (rabuka.h:1649), so index 8
 * alone overwrites the length with attacker-chosen data, and the write
 * continues through `live_success`, `live_score`, `live_pre_score`
 * (2 x 4096 ints), `p1_live_won` and onward. That is why the test asserts
 * on the length AND on canaries in the members that follow, rather than only
 * checking that nothing crashed: with a 2 KB overrun the old code corrupted
 * memory silently on the way to whatever it hit next.
 *
 * FIXED to clamp at RB_MAX_RECENTLY_MOVED, which is the faithful bound. Rust
 * carries this relay in a SmallVec whose inline capacity is 8, and the C
 * field's own comment at rabuka.h:1648 describes it as a "cards chosen by a
 * select_cards/select/look_and_select choice" relay -- the same
 * recently-moved-style list as `recently_moved` / `those_cards`, NOT a
 * zone-sized buffer.
 *
 * HOW THE WIDE ELIGIBLE SET IS REACHED. Measured over the whole card DB
 * (throwaway probe walking every (cid, aidx) ability): the widest MANDATORY
 * `reveal` cost carries count == 4, and no card has count >= 5. So no corpus
 * fixture can hand this arm 9 cards and the defect is LATENT, not live --
 * which is precisely why it survived. The test therefore drives the arm
 * through rb_pay_cost with a synthetic cost node whose count is set to the
 * hand width, and installs the queue entry so rb_entry_effect() resolves a
 * REAL card whose effect genuinely reads `selected_cards` -- the gate at
 * cost.c:1927 is `effect_uses_selected_cards(rb_entry_effect(g))`
 * (cost.c:309-314), and rb_entry_effect decodes from the queue entry's
 * (card_id, ability_idx), so it cannot be faked with a hand-built node.
 * cid 437 / ability 0 is one of the measured selected_cards consumers.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int checks;
static int failures;

/* FLAT_VERDICT comes from test_game.h -- do not redefine it. */

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
        FLAT_VERDICT(); \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    long actual_value = (long)(actual); \
    long expected_value = (long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %ld expected %ld)\n", \
                message, actual_value, expected_value); \
        failures++; \
        FLAT_VERDICT(); \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A distinct, easily-recognised card id per slot, so a canary comparison can
 * prove WHICH member got clobbered rather than merely that something did. */
#define CANARY_BASE 0x5AC0DE

/* cid 437 ability 0: measured (throwaway probe over every (cid, aidx)) to
 * carry an effect whose source is the literal "selected_cards", which is what
 * cost.c:309-314 looks for. */
#define SEL_CONSUMER_CID   437
#define SEL_CONSUMER_AIDX  0

/* Empty every zone both players own, mirroring the shared `bare_board`
 * helper in the other parity suites. */
static void bare_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_revealed_cost_cards = 0;
    tg->state.n_selected_cards = 0;
    tg->state.activation_keepalive_valid = 0;
    tg->state.queue.n_entries = 0;
    tg->state.queue.cur = 0;
    rb_clear_pending_choice(&tg->state);
}

/* Seed the GameState members that immediately follow `selected_cards[]` with
 * recognisable canaries. With the old RB_MAX_ZONE clamp a 9-card reveal wrote
 * ids[0..8] into selected_cards[0..7] + n_selected_cards + live_success[0],
 * so `n_selected_cards` and `live_success[0]` both ended up holding a CARD
 * ID rather than the sentinel.
 *
 * `n_selected_cards` itself is NOT seeded as a canary -- it is the member the
 * arm legitimately writes, so a canary there could never hold. It is instead
 * asserted directly (B3/C3), which is the sharper check anyway: the old code
 * left it at 9, or up to 512. */
static void seed_canaries(TestGame *tg)
{
    tg->state.live_success[0]           = CANARY_BASE + 2;
    tg->state.live_success[1]           = CANARY_BASE + 3;
    tg->state.live_score[0]             = CANARY_BASE + 4;
    tg->state.live_score[1]             = CANARY_BASE + 5;
    tg->state.live_batch_mode           = CANARY_BASE + 6;
    tg->state.live_victory_pending      = CANARY_BASE + 7;
    tg->state.live_victory_stage        = CANARY_BASE + 8;
    tg->state.live_pre_valid[0]         = CANARY_BASE + 9;
    tg->state.p1_live_won               = CANARY_BASE + 10;
    tg->state.p2_live_won               = CANARY_BASE + 11;
    tg->state.self_live_surplus_count   = CANARY_BASE + 12;
    tg->state.opponent_live_surplus_count = CANARY_BASE + 13;
    tg->state.live_surplus_ready_this_turn = CANARY_BASE + 14;
    tg->state.p1_live_success_no_excess = CANARY_BASE + 15;
}

static int check_canaries_intact(TestGame *tg, const char *when)
{
    int bad = 0;
    if (tg->state.live_success[0] != CANARY_BASE + 2) bad = 2;
    if (tg->state.live_success[1] != CANARY_BASE + 3) bad = 3;
    if (tg->state.live_score[0] != CANARY_BASE + 4) bad = 4;
    if (tg->state.live_score[1] != CANARY_BASE + 5) bad = 5;
    if (tg->state.live_batch_mode != CANARY_BASE + 6) bad = 6;
    if (tg->state.live_victory_pending != CANARY_BASE + 7) bad = 7;
    if (tg->state.live_victory_stage != CANARY_BASE + 8) bad = 8;
    if (tg->state.live_pre_valid[0] != CANARY_BASE + 9) bad = 9;
    if (tg->state.p1_live_won != CANARY_BASE + 10) bad = 10;
    if (tg->state.p2_live_won != CANARY_BASE + 11) bad = 11;
    if (tg->state.self_live_surplus_count != CANARY_BASE + 12) bad = 12;
    if (tg->state.opponent_live_surplus_count != CANARY_BASE + 13) bad = 13;
    if (tg->state.live_surplus_ready_this_turn != CANARY_BASE + 14) bad = 14;
    if (tg->state.p1_live_success_no_excess != CANARY_BASE + 15) bad = 15;
    if (bad) {
        fprintf(stderr,
                "FAIL: %s: canary #%d was overwritten by the selected_cards write "
                "(n_selected_cards=%d live_success[0]=%d live_score[0]=%d "
                "live_batch_mode=%d live_victory_pending=%d live_victory_stage=%d "
                "live_pre_valid[0]=%d p1_live_won=%d p2_live_won=%d)\n",
                when, bad, tg->state.n_selected_cards, tg->state.live_success[0],
                tg->state.live_score[0], tg->state.live_batch_mode,
                tg->state.live_victory_pending, tg->state.live_victory_stage,
                tg->state.live_pre_valid[0], tg->state.p1_live_won,
                tg->state.p2_live_won);
    }
    return bad == 0;
}

static char *dupstr(const char *s)
{
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* The mandatory reveal cost node the fast path keys on:
 *   action="reveal", source="hand", count=n  -> has_explicit_count, and with
 *   optional=0 and n_ids == count it takes the handlers.rs:1260-1283
 *   reveal-without-prompt arm. No cost_values / group_names / card_type, so
 *   EVERY card in hand is eligible -- which is how the eligible set is pushed
 *   past the 8-slot capacity on demand. */
static AbilityEffect *make_reveal_cost(int n)
{
    AbilityEffect *c = (AbilityEffect *)calloc(1, sizeof(AbilityEffect));
    if (!c) return NULL;
    c->action  = dupstr("reveal");
    c->source  = dupstr("hand");
    c->target  = dupstr("self");
    c->count   = n;
    c->is_optional = 0;
    return c;
}

static void free_effect(AbilityEffect *e)
{
    if (!e) return;
    free(e->action); free(e->source); free(e->destination); free(e->target);
    free(e);
}

/* Prove the corpus fixture really is a selected_cards consumer, so the suite
 * fails loudly if cid 437 ever stops being one instead of silently ceasing to
 * exercise the guarded branch. */
static int sel_consumer_resolves(void)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)SEL_CONSUMER_CID, SEL_CONSUMER_AIDX, &ab)) {
        fprintf(stderr, "FIXTURE: cid %d ability %d does not decode\n",
                SEL_CONSUMER_CID, SEL_CONSUMER_AIDX);
        return 0;
    }
    int ok = 0;
    /* Mirrors cost.c:309-314 effect_uses_selected_cards. */
    const AbilityEffect *stack[32];
    int sp = 0;
    stack[sp++] = ab.effect;
    while (sp > 0) {
        const AbilityEffect *e = stack[--sp];
        if (!e) continue;
        if (e->source && !strcmp(e->source, "selected_cards")) { ok = 1; break; }
        for (int i = 0; i < e->n_child && i < 8 && sp < 32; i++) stack[sp++] = e->child[i];
        for (int i = 0; i < e->n_options && i < 8 && sp < 32; i++) stack[sp++] = e->options[i];
    }
    if (!ok)
        fprintf(stderr, "FIXTURE: cid %d ability %d no longer reads selected_cards\n",
                SEL_CONSUMER_CID, SEL_CONSUMER_AIDX);
    rb_free_ability(&ab);
    return ok;
}

/* Drive the reveal-cost arm with `n_hand` eligible cards in hand. Returns the
 * rb_pay_cost() result. */
static int run_reveal(TestGame *tg, int n_hand)
{
    bare_board(tg);
    seed_canaries(tg);
    tg->state.n_selected_cards = 0;

    /* Distinct ids so the populated relay is checkable and a clobbering write
     * is distinguishable from a coincidental value. Card ids are opaque, so
     * these need not be real prints -- they are only stored, revealed and
     * counted here, never resolved against the card DB. */
    for (int i = 0; i < n_hand; i++)
        tg->state.p[0].hand.cards[i] = 100000 + i;
    tg->state.p[0].hand.n = n_hand;

    /* Install the queue entry rb_entry_effect() decodes from, so the
     * effect_uses_selected_cards gate at cost.c:1927 is actually TRUE.
     * That is what makes this reach the population code at all. */
    tg->state.queue.n_entries = 1;
    tg->state.queue.cur = 0;
    tg->state.queue.entries[0].card_id = SEL_CONSUMER_CID;
    tg->state.queue.entries[0].ability_idx = SEL_CONSUMER_AIDX;

    AbilityEffect *cost = make_reveal_cost(n_hand);
    if (!cost) return -99;

    int rc = rb_pay_cost(&tg->state, 0, cost);

    free_effect(cost);
    rb_clear_pending_choice(&tg->state);
    return rc;
}

static void test_capacity_is_eight(void)
{
    /* The premise of the whole suite, asserted rather than assumed: if a
     * future edit resizes the field, every bound below is re-derived. */
    CHECK_EQ(sizeof(((GameState *)0)->selected_cards) / sizeof(int),
             RB_MAX_RECENTLY_MOVED,
             "A1: selected_cards[] has exactly RB_MAX_RECENTLY_MOVED (8) slots");
    CHECK_EQ(RB_MAX_RECENTLY_MOVED, 8,
             "A2: RB_MAX_RECENTLY_MOVED is still 8, so a 9-card reveal is over capacity");
    CHECK(RB_MAX_RECENTLY_MOVED < RB_MAX_ZONE,
           "A3: the array is smaller than RB_MAX_ZONE, which is why a RB_MAX_ZONE clamp is an overflow");
    CHECK(sel_consumer_resolves(),
          "A4: the cid-437 fixture really does read selected_cards, so the guarded branch is reachable");
}

static void test_nine_cards_does_not_overflow(void)
{
    static TestGame tg;
    test_game_new(&tg);

    int rc = run_reveal(&tg, 9);
    CHECK(rc == 1, "B1: a mandatory 9-card reveal cost is paid (not refused)");
    CHECK(check_canaries_intact(&tg, "B2"),
          "B2: the 9th card did NOT overwrite anything past selected_cards[7]");

    /* The decisive assertion. The old code left this at 9 (and up to 512 for
     * a wider hand), which is already an out-of-bounds length. */
    CHECK(tg.state.n_selected_cards <= RB_MAX_RECENTLY_MOVED,
          "B3: n_selected_cards is clamped to the array's real capacity after a 9-card reveal");

    /* ...and the relay is still populated, i.e. the fix truncated rather
     * than disabled the hand-off the rest of the engine depends on. */
    CHECK(tg.state.n_selected_cards > 0,
          "B4: the selected_cards relay is still populated after the clamp");
    CHECK_EQ(tg.state.selected_cards[0], 100000,
             "B5: selected_cards[0] is the first eligible card");
}

static void test_maximum_width_does_not_corrupt(void)
{
    static TestGame tg;
    test_game_new(&tg);

    /* RB_MAX_HAND is the widest the hand can physically be, so this is the
     * true maximum the arm can ever be handed. */
    int rc = run_reveal(&tg, RB_MAX_HAND);
    CHECK(rc == 1, "C1: a mandatory full-hand reveal cost is paid");
    CHECK(check_canaries_intact(&tg, "C2"),
          "C2: a 40-card reveal left every member after selected_cards[] intact");

    CHECK(tg.state.n_selected_cards <= RB_MAX_RECENTLY_MOVED,
          "C3: n_selected_cards is clamped at RB_MAX_HAND-wide reveal");
    CHECK_EQ(tg.state.n_selected_cards, RB_MAX_RECENTLY_MOVED,
             "C4: the clamp saturates at the capacity, it is not merely 'smaller'");

    /* Every slot holds a value from the eligible set, i.e. the write stayed
     * inside the array instead of scattering past it. */
    int all_from_hand = 1;
    for (int i = 0; i < tg.state.n_selected_cards; i++) {
        int v = tg.state.selected_cards[i];
        if (v < 100000 || v >= 100000 + RB_MAX_HAND) all_from_hand = 0;
    }
    CHECK(all_from_hand,
          "C5: every populated selected_cards slot holds an eligible hand id");
}

static void test_repeated_wide_reveals_stay_stable(void)
{
    static TestGame tg;
    test_game_new(&tg);

    /* The arm is reached once per activation in a real game, so a repeat that
     * re-populates rather than accumulates is the realistic shape. Two runs
     * in a row also prove the clamp is not off-by-one in a way that only shows
     * on the second pass. */
    for (int pass = 1; pass <= 2; pass++) {
        char msg[160];
        int rc = run_reveal(&tg, RB_MAX_HAND);
        snprintf(msg, sizeof msg, "D%d: pass %d pays a full-hand reveal", pass, pass);
        CHECK(rc == 1, msg);
        snprintf(msg, sizeof msg,
                 "D%d: pass %d leaves members after selected_cards[] intact", pass, pass);
        CHECK(check_canaries_intact(&tg, msg), msg);
    }
    CHECK_EQ(tg.state.n_selected_cards, RB_MAX_RECENTLY_MOVED,
             "D3: n_selected_cards does not grow across repeated wide reveals");
}

static void test_narrow_reveal_unchanged(void)
{
    static TestGame tg;
    test_game_new(&tg);

    /* The guard against over-correcting: a reveal that already fits must be
     * recorded in full, byte for byte as before the fix. */
    int rc = run_reveal(&tg, 3);
    CHECK(rc == 1, "E1: a mandatory 3-card reveal cost is paid");
    CHECK_EQ(tg.state.n_selected_cards, 3,
             "E2: an under-capacity reveal records ALL 3 cards (no truncation)");
    CHECK_EQ(tg.state.selected_cards[0], 100000, "E3: slot 0 is the first card");
    CHECK_EQ(tg.state.selected_cards[1], 100001, "E4: slot 1 is the second card");
    CHECK_EQ(tg.state.selected_cards[2], 100002, "E5: slot 2 is the third card");

    /* Every revealed card still reaches the revealed ledger, which is sized
     * RB_MAX_REVEALED_CARDS == RB_MAX_ZONE (512) and is NOT the field that
     * overflowed. A 3-card reveal must still reveal 3. */
    CHECK_EQ(tg.state.n_revealed, 3,
             "E6: all 3 cards reached revealed_cards[] (that array is correctly sized)");
}

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 2;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    test_capacity_is_eight();
    test_nine_cards_does_not_overflow();
    test_maximum_width_does_not_corrupt();
    test_repeated_wide_reveals_stay_stable();
    test_narrow_reveal_unchanged();

    printf("\n==== parity_selected_cards_overflow ====\n");
    printf("tests run: 5\n");
    printf("checks, %d failures\n", failures);
    if (failures > 0) {
        printf("FAILED: parity_selected_cards_overflow\n");
        return 1;
    }
    printf("PASSED: parity_selected_cards_overflow\n");
    return 0;
}
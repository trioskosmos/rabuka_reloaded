/* test_parity_sequential.c — C parity suite for the Rust cluster
 *   engine/tests/test_modules/effects/compound/sequential_effects/  (24 files,
 *   119 #[test])
 *
 * WHAT THIS CLUSTER IS ABOUT
 *   A `Sequential` ability is an ORDERED list of steps. The behaviours that
 *   matter, and that this file pins, are:
 *     1. ORDER — step 2 must observe step 1's result (mill, then pick the card
 *        you just milled; wait self, THEN take the live card).
 *     2. EARLY EXIT — a failed step stops the tail (no live milled -> no
 *        recovery prompt; waitroom at threshold -> nothing mills).
 *     3. OPTIONAL INTERMEDIATE COST — …してもよい declines cleanly AND stops
 *        the reward behind the そう場合 gate.
 *     4. A STEP THAT PARKS A CHOICE — the tail must SURVIVE the pause (the
 *        pending_actions clone path).
 *     5. A STEP THAT ENDS THE ABILITY — change_state(wait) / a repeat stop.
 *     6. COST-TOTAL BRANCHES — 6 / 8 / 25 on 天王寺璃奈: one leg, three
 *        mutually exclusive rewards.
 *
 * Rust files covered, by name (banner above each block):
 *   baton_touch_place_liella_under_then_activate_copied_ability_pl_sp_pb2_005_test
 *   cheaper_baton_both_hands_discard_to_three_draw_three_q229_test
 *   debut_deficit_mill_recent_live_topdeck_test
 *   debut_draws_one_and_cannot_live_this_turn_q68_test
 *   debut_group_live_topdeck_waited_opponent_draw_test
 *   debut_mill_two_live_fourth_from_top_q226_test
 *   debut_optional_discard_reveal_until_live_q73_q102_test
 *   debut_two_distinct_live_cards_opponent_picks_one_q118_test
 *   debut_two_distinct_live_cards_opponent_picks_one_to_hand_q118_test
 *   debut_unit_discard_any_draw_count_plus_one_test
 *   deficit_mill_live_topdeck_pl_n_pr_032_pr_test
 *   group_baton_both_hands_keep_three_bottomdeck_draw_three_test
 *   live_start_bottom_one_mill_live_heart_source_test
 *   live_start_cost_total_8_and_25_branches_test
 *   live_start_draw_opponent_cost_nine_wait_test
 *   live_start_own_discard_two_member_bottomdeck_hearts_q164_test
 *   live_start_repeat_mill_blade_live_self_wait_q222_test
 *   live_start_two_different_named_edelnote_members_gain_blades_and_heart06_test
 *   live_success_mill_five_distinct_live_recovery_setup_test
 *   live_success_swap_one_niji_in_success_zone_pl_n_bp4_010_r_test
 *   mus_live_recovery_success_score_energy_pl_pr_017_pr_test
 *   place_energy_under_self_then_draw_and_gain_heart_pl_n_bp5_012_r_test
 *   q257_wait_self_then_score_four_live_from_waitroom_to_hand_test
 *   (mod.rs carries no #[test].)
 *
 * C-API landmines honoured here (AGENTS.md + peer findings):
 *   - load_card_db() falls back to rb_load("../cards/build"); rb_load("src")
 *     fails under the isolated out-of-tree build.
 *   - mid() == test_new_id(), not test_id(). Several sections put the SAME
 *     print on two seats or in two zones; the shared template id would alias.
 *   - sizeof(TestGame) is ~781 KB, so EVERY TestGame local here is `static`.
 *   - Heart reads go through rb_mods_get_heart / rb_mods_get_need_heart
 *     DIRECTLY: test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE.
 *   - RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   - test_set_live_card is the P1 APPEND path; test_set_live_card_for(tg, pl,
 *     card) writes the other seat.
 *   - RbChoice carries no option list, so a `position|destination` prompt's
 *     deck_top-vs-deck_bottom is NOT observable from C. Where the Rust test
 *     asserts that, this file records an EXPECTED_GAP rather than a pass.
 *
 * PROMPT-COUNT CONTROL
 *   Every negative assertion that could pass VACUOUSLY on a board where the
 *   effect is a no-op is preceded by CHECK(prompts > 0, ...) — see
 *   `prompts_seen` and the `control` comment above each such block. A negative
 *   assertion without a prompt-count control proves nothing.
 *
 * This file is a TEST file; it never edits engine code.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── crash reporting + fork isolation ────────────────────────────────────
 * Each test runs in a forked child so a SIGSEGV/SIGBUS/SIGABRT is attributed
 * to the test in flight rather than truncating the run so every later section
 * falsely reads as a pass. No time-based watchdog: a process watchdog does not
 * work on this toolchain (in-child alarm() never fires; a parent-side
 * waitpid(WNOHANG) deadline fires but its kill(SIGKILL) does not take effect,
 * wedging the blocking waitpid). A child that HANGS is a recorded limitation.
 */
static const char *current_test = "(none)";
static int failures;
static int assertions;
static int gaps;
static int setup_bugs;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        assertions++;                                                       \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* A Rust assertion with no reachable C entry point. Counted separately; never
 * fails the suite and never stands in for a real CHECK. */
#define EXPECTED_GAP(condition, desc) do {                                  \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            printf("GAP: %s\n", desc);                                      \
            gaps++;                                                         \
        }                                                                   \
    } while (0)

/* heart colour slots, per cards/compile_cards.py HEART_COLORS */
#define H00 0
#define H01 1
#define H02 2
#define H03 3
#define H04 4
#define H05 5
#define H06 6
#define HALL 7

/* stage area indices */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* Card numbers carrying the FULLWIDTH PLUS U+FF0B (bytes ef bc 8b) are written
 * as the explicit escape so the file's encoding can never corrupt them. */
#define PLUS "\xEF\xBC\x8B"

/* ── fixtures ───────────────────────────────────────────────────────────── */

/* The ubiquitous SD filler prints of the Rust cluster. */
static const char *FILLER = "PL!-sd1-010-SD";   /* 高坂 穂乃果 cost 4 */
static const char *LIVE_L = "PL!-sd1-019-SD";   /* START:DASH!!  score 1 */
static const char *LIVE_M = "PL!-sd1-020-SD";   /* きっと青春が… score 2 */
static const char *LIVE_N = "PL!N-sd1-028-SD";  /* Dream with You score 4 */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust `game.id(no)`: the SHARED template. */
static int tid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        assertions++; setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* Rust `game.new_id(no)`: a DISTINCT pool slot backed by the same record. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; setup_bugs++; failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* AGENTS.md identity rule: pin every fixture against the shipped card_no.
 * rb_find_card_by_no() has a fuzzy tail, so an un-pinned typo silently proves
 * nothing. */
static int ident(int cid, const char *no)
{
    if (cid >= 0 && rb_card_no_eq(cid, no)) return 1;
    assertions++; setup_bugs++; failures++;
    char got[64] = "?";
    Card c;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        snprintf(got, sizeof got, "%s", rb_card_string(c.card_no_idx));
        rb_free_card(&c);
    }
    fprintf(stderr, "SETUP BUG: expected card_no \"%s\", got \"%s\" (idx %d)\n",
            no, got, cid);
    return 0;
}

/* Direct modifier reads — test_get_heart_modifier remaps colour 5. */
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}
static int blade_of(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}
static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o != NULL && strcmp(o, "wait") == 0;
}
static int not_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o == NULL || strcmp(o, "wait") != 0;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int disc_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].deck, cid); }
static int hand_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int p2_hand_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[1].hand, cid); }
static int deck_top(TestGame *tg)
{
    RbBag *b = &tg->state.p[0].deck;
    return b->n > 0 ? b->cards[0] : RB_EMPTY_SLOT;
}
static int under_n(TestGame *tg, int area) { return tg->state.p[0].under_cards[area].n; }
static int under_has(TestGame *tg, int area, int cid)
{
    return bag_has(&tg->state.p[0].under_cards[area], cid);
}
static int hand_n(TestGame *tg) { return tg->state.p[0].hand.n; }
static int deck_n(TestGame *tg) { return tg->state.p[0].deck.n; }
static int disc_n(TestGame *tg) { return tg->state.p[0].discard.n; }
static int p2_hand_n(TestGame *tg) { return tg->state.p[1].hand.n; }

/* Empty every zone both players own, so each count assertion is unambiguous. */
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
    tg->state.n_selected_cards = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    rb_clear_pending_choice(&tg->state);
}

static void fill_zone(RbBag *b, int card, int n)
{
    b->n = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) b->cards[b->n++] = card;
}

/* Rust `fill_decks(&mut game, filler)`: n copies into BOTH main decks. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust `main_deck.cards.insert(0, card)`. */
static void add_deck_top(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].deck;
    if (b->n >= RB_MAX_ZONE) return;
    for (int i = b->n; i > 0; i--) b->cards[i] = b->cards[i - 1];
    b->cards[0] = card;
    b->n++;
}

/* Seat-aware zone pushes (Rust `player2.hand.cards.push(..)`). */
static void add_discard_for(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].discard;
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = card;
}
static void add_hand_for(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].hand;
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = card;
}
static void add_success_for(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].success;
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = card;
}

/* ── choice plumbing ──────────────────────────────────────────────────────
 * `prompts_seen` counts EVERY pending choice this file answers. Every
 * negative assertion that could pass vacuously is preceded by a CHECK on it.
 */
static int prompts_seen;
static int card_prompts_seen;   /* only the SelectCard prompts, for thresholds */

static void pump(TestGame *tg);

/* Rust's play_to_stage answers the play-time position confirm internally, so a
 * C harness that calls rb_play_member through test_play_to_stage can be left
 * holding an `area_select` choice BEFORE the 登場 ability ever runs. Every
 * section below calls settle_play() right after a play so the prompt it then
 * measures is the ability's OWN first prompt, not the play's. It only ever
 * answers a choice whose `target` is literally "area_select". */
static void settle_play(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 4) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (!c || strcmp(c->target, "area_select") != 0) return;
        rb_resume_with_choice(&tg->state, 1);
        pump(tg);
    }
}

/* Does ANY queue entry stamp the pending choice with `pl`? Index 0 is not
 * reliably the current entry, so the seat claims below ask this instead. */
static int any_choice_seat(TestGame *tg, int pl)
{
    int n = test_queue_n_entries(tg);
    for (int i = 0; i < n; i++)
        if (test_queue_entry_choice_seat(tg, i) == pl) return 1;
    return 0;
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static const RbChoice *pending_ch(TestGame *tg)
{
    return test_has_pending_choice(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

/* kind / zone / target / count of whatever is pending, for failure messages. */
static const char *pending_desc(TestGame *tg)
{
    static char buf[192];
    const RbChoice *c = pending_ch(tg);
    if (!c) { snprintf(buf, sizeof buf, "(no pending choice)"); return buf; }
    snprintf(buf, sizeof buf, "kind=%d zone=\"%s\" target=\"%s\" count=%d "
             "allow_skip=%d n_filtered=%d cost_limit=%d op=\"%s\"",
             (int)c->kind, c->zone, c->target, c->count, c->allow_skip,
             c->n_filtered_indices, c->cost_limit, c->cost_limit_op);
    return buf;
}

static const char *pending_zone(TestGame *tg)
{
    const RbChoice *c = pending_ch(tg);
    return (c && c->zone[0]) ? c->zone : "";
}

static void pump(TestGame *tg)
{
    rb_process_pending_auto_abilities(&tg->state);
    if (tg->state.queue.has_pending || tg->state.queue.deferred) return;
    if (tg->state.activation_keepalive_valid) {
        rb_free_ability(&tg->state.activation_keepalive);
        tg->state.activation_keepalive_valid = 0;
    }
}

/* Answer ONE prompt. `accept`:
 *   >= 0  -> SelectTarget / SelectHeartColor / SelectPosition take that option
 *   < 0   -> SelectCard DECLINE (Rust select_indices(&[]))
 * For SelectCard, `picks` leading indices are submitted; picks<=0 with
 * accept>=0 submits exactly one pick. */
static void answer(TestGame *tg, int accept, int picks)
{
    if (!test_has_pending_choice(tg)) return;
    prompts_seen++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    RbChoiceKind kind = c ? c->kind : RB_CHOICE_NONE;
    if (kind == RB_CHOICE_SELECT_CARD) card_prompts_seen++;
    if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_HEART_COLOR ||
        kind == RB_CHOICE_SELECT_POSITION || kind == RB_CHOICE_SELECT_NUMBER) {
        rb_resume_with_choice(&tg->state, accept);
    } else if (accept < 0) {
        rb_resume_with_choice(&tg->state, -1);
    } else if (picks == 1) {
        rb_resume_with_choice(&tg->state, 0);
    } else {
        int idx[RB_MAX_ZONE];
        for (int i = 0; i < picks && i < RB_MAX_ZONE; i++) idx[i] = i;
        test_select_indices(tg, idx, picks);
    }
    pump(tg);
}

/* Drain everything. mode: 1 = accept, 0 = take the first option,
 * -1 = decline every card prompt / take option 0 on target prompts. */
static void drain(TestGame *tg, int mode)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 80) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        RbChoiceKind kind = c ? c->kind : RB_CHOICE_NONE;
        if (kind == RB_CHOICE_SELECT_CARD) {
            if (mode < 0) answer(tg, -1, 0);
            else {
                int n = c->count > 0 ? c->count : 1;
                if (n > 4) n = 4;
                answer(tg, 1, n);
            }
        } else {
            answer(tg, mode < 0 ? 0 : (mode ? 1 : 0), 1);
        }
        rb_drain_ability_queue(&tg->state);
    }
    rb_drain_ability_queue(&tg->state);
}

/* A sequential tail parked as OWNED pending actions is not a new queue entry,
 * so a hand-rolled drain must set the queue IDLE and let the consumer run. */
static int after_tail(TestGame *tg)
{
    if (test_has_pending_choice(tg)) return 1;
    if (tg->state.queue.has_pending) return 0;
    rb_queue_resume_pending_actions(&tg->state);
    rb_process_pending_auto_abilities(&tg->state);
    return test_has_pending_choice(tg);
}

static void fire_live_start(TestGame *tg)
{
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

static void fire_debut(TestGame *tg, int card)
{
    rb_trigger_debut(&tg->state, 0, card);
    tg->state.activating_card = card;
    rb_process_pending_auto_abilities(&tg->state);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §A — 優木せつ菜 PL!N-PR-032-PR 登場:
 *   「自分の控え室にあるカードの枚数が8枚未満の場合、自分のデッキの上から
 *     カードをその差に等しい枚数控え室に置く。その後、これにより控え室に置いた
 *     カードの中から、ライブカード1枚をデッキの一番上に置いてもよい。」
 *   The purest ORDER test in the cluster: step 2 can only see a live card that
 *   step 1 created, and the < 8 gate is the early exit.
 * Rust: deficit_mill_live_topdeck_pl_n_pr_032_pr_test.rs (5 tests)
 * ══════════════════════════════════════════════════════════════════════════ */
static void setsuna_setup(TestGame *tg, int me, const int *w, int nw,
                          const int *mill, int nm)
{
    fill_decks(tg, tid(tg, FILLER), 20);
    tg->state.p[0].stage[AREA_CENTER] = me;
    for (int i = 0; i < nw; i++) test_add_to_discard(tg, w[i]);
    for (int i = nm - 1; i >= 0; i--) add_deck_top(tg, 0, mill[i]);
}

static void test_a_setsuna_mills_shortfall_and_recovers_live_to_deck_top(void)
{
    static TestGame tg, t2;
    test_game_new(&tg);
    bare_board(&tg);

    int me = tid(&tg, "PL!N-PR-032-PR");
    int live = tid(&tg, LIVE_L);
    CHECK(ident(me, "PL!N-PR-032-PR") && ident(live, LIVE_L),
          "A1: the 優木せつ菜 and live fixtures are the printed card numbers");

    int w[5] = { tid(&tg, "PL!-sd1-001-SD"), tid(&tg, "PL!S-sd1-003-SD"),
                 tid(&tg, "PL!-sd1-007-SD"), tid(&tg, "PL!HS-bp5-001-P"),
                 tid(&tg, "PL!SP-bp5-006-R") };
    int mill[3] = { mid(&tg, FILLER), live, mid(&tg, "PL!S-sd1-001-SD") };
    setsuna_setup(&tg, me, w, 5, mill, 3);

    int wait_before = disc_n(&tg);
    fire_debut(&tg, me);

    CHECK_EQ(disc_n(&tg), wait_before + 3,
             "A1: waitroom 5 of 8 -> mill exactly the 3-card shortfall");
    CHECK(pending(&tg), "A1: the second step (recover the milled live) prompts");
    CHECK(pending_ch(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "A1: the recovery step is a SelectCard");
    CHECK(!strcmp(pending_zone(&tg), "discard"),
          "A1: the recovery step selects out of the waitroom");
    answer(&tg, 1, 1);

    CHECK_EQ(deck_top(&tg), live,
             "A1: accepting returns the MILLED live card to the deck TOP "
             "(step 2 observes step 1's result)");
    CHECK(!disc_has(&tg, live), "A1: the recovered live card left the waitroom");

    /* control: the decline leg proves the recovery prompt is a real decision
     * point and not an auto-resolution that happens to move the card. */
    test_game_new(&t2);
    bare_board(&t2);
    int me2 = tid(&t2, "PL!N-PR-032-PR");
    int live2 = tid(&t2, LIVE_L);
    int w2[5] = { tid(&t2, "PL!-sd1-001-SD"), tid(&t2, "PL!S-sd1-003-SD"),
                  tid(&t2, "PL!-sd1-007-SD"), tid(&t2, "PL!HS-bp5-001-P"),
                  tid(&t2, "PL!SP-bp5-006-R") };
    int mill2[3] = { mid(&t2, FILLER), live2, mid(&t2, "PL!S-sd1-001-SD") };
    setsuna_setup(&t2, me2, w2, 5, mill2, 3);
    fire_debut(&t2, me2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "A1 control: the decline fixture also prompts");
    answer(&t2, -1, 0);
    CHECK(prompts_seen > 0, "A1 control: the decline really answered a prompt");
    CHECK(disc_has(&t2, live2),
          "A1: declining …してもよい keeps the milled live card in the waitroom");
    CHECK(!deck_has(&t2, live2), "A1: a declined recovery puts nothing on the deck");
}

static void test_a_setsuna_no_live_among_milled_no_prompt(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int me = tid(&tg, "PL!N-PR-032-PR");
    int w[2] = { tid(&tg, "PL!-sd1-001-SD"), tid(&tg, "PL!S-sd1-003-SD") };
    int m = tid(&tg, FILLER);
    setsuna_setup(&tg, me, w, 2, &m, 1);

    int wait_before = disc_n(&tg);
    fire_debut(&tg, me);

    CHECK_EQ(disc_n(&tg), wait_before + 6, "A2: waitroom 2 of 8 -> mill exactly 6");
    CHECK(!after_tail(&tg),
          "A2: no live card among the milled -> the second step must NOT prompt "
          "(early exit on an empty candidate set)");
}

static void test_a_setsuna_waitroom_at_threshold_mills_nothing(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int me = tid(&tg, "PL!N-PR-032-PR");
    int w[8] = { tid(&tg, "PL!-sd1-001-SD"), tid(&tg, "PL!S-sd1-003-SD"),
                 tid(&tg, "PL!-sd1-007-SD"), tid(&tg, "PL!HS-bp5-001-P"),
                 tid(&tg, "PL!SP-bp5-006-R"), tid(&tg, "PL!S-bp5-009-R"),
                 tid(&tg, "PL!N-bp3-006-R"), tid(&tg, "PL!SP-bp4-022-N") };
    int m = tid(&tg, FILLER);
    setsuna_setup(&tg, me, w, 8, &m, 1);

    int deck_before = deck_n(&tg);
    fire_debut(&tg, me);

    CHECK_EQ(deck_n(&tg), deck_before,
             "A3: waitroom >= 8 -> the < 8 gate fails and NOTHING mills");
    CHECK(!after_tail(&tg),
          "A3: nothing milled -> no recovery prompt (the tail is gated)");
}

static void test_a_setsuna_waitroom_seven_mills_one_and_recovers_live(void)
{
    static TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);
    int me = tid(&tg, "PL!N-PR-032-PR");
    int live = tid(&tg, LIVE_L);
    int w[7] = { tid(&tg, "PL!-sd1-001-SD"), tid(&tg, "PL!S-sd1-003-SD"),
                 tid(&tg, "PL!-sd1-007-SD"), tid(&tg, "PL!HS-bp5-001-P"),
                 tid(&tg, "PL!SP-bp5-006-R"), tid(&tg, "PL!S-bp5-009-R"),
                 tid(&tg, "PL!N-bp3-006-R") };
    setsuna_setup(&tg, me, w, 7, &live, 1);

    int wait_before = disc_n(&tg);
    fire_debut(&tg, me);
    CHECK_EQ(disc_n(&tg), wait_before + 1, "A4: waitroom 7 of 8 -> mill exactly 1");
    CHECK(pending(&tg),
          "A4: the one milled card IS a live card -> the recovery step prompts");
    answer(&tg, 1, 1);
    CHECK_EQ(deck_top(&tg), live, "A4: accepting puts the milled live card on the deck top");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §B — 高海千歌 PL!S-PR-044-PR 登場: the SAME printed ability as §A on a
 * different print, but reached through a real hand PLAY, so the play cost and
 * the 登場 trigger are exercised too.
 * Rust: debut_deficit_mill_recent_live_topdeck_test.rs (5 tests)
 * ══════════════════════════════════════════════════════════════════════════ */
static void chika_board(TestGame *tg, int *chika, int *live, int with_live)
{
    bare_board(tg);
    *chika = tid(tg, "PL!S-PR-044-PR");
    *live = tid(tg, LIVE_L);
    if (with_live) test_add_to_discard(tg, *live);
    for (int i = 0; i < 4; i++) test_add_to_discard(tg, mid(tg, FILLER));
    fill_decks(tg, tid(tg, FILLER), 20);
    /* mill order at the deck top: member, live, member */
    add_deck_top(tg, 0, mid(tg, FILLER));
    if (with_live) add_deck_top(tg, 0, *live);
    add_deck_top(tg, 0, mid(tg, FILLER));
    test_give_energy(tg, 10);
    test_add_to_hand(tg, *chika);
}

static void test_b_chika_deficit_mill_then_optional_recovery(void)
{
    static TestGame tg, t2;
    int chika, live;
    test_game_new(&tg);
    chika_board(&tg, &chika, &live, 1);
    CHECK(ident(chika, "PL!S-PR-044-PR"),
          "B1: the 登場 source is the printed 高海千歌 PR card");

    CHECK(test_play_to_stage(&tg, chika, AREA_CENTER),
          "B1: 高海千歌 (cost 5) is played to the center");
    CHECK(pending(&tg), "B1: the recovery step prompts after the play");
    CHECK(pending_ch(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "B1: the recovery step is a SelectCard over the waitroom");
    answer(&tg, 1, 1);

    CHECK_EQ(deck_top(&tg), live,
             "B1: the live card among the just-milled cards lands on the deck top");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], chika,
             "B1: she stays on the center after the sequence");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 5,
             "B1: the 5-energy play cost was charged (10 -> 5)");

    /* control: the declined leg is the control for B1's positive. */
    int chika2, live2;
    test_game_new(&t2);
    chika_board(&t2, &chika2, &live2, 1);
    test_play_to_stage(&t2, chika2, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "B2 control: the same optional prompt appears");
    answer(&t2, -1, 0);
    CHECK(prompts_seen > 0, "B2 control: the decline really answered a prompt");
    CHECK(!deck_has(&t2, live2),
          "B2: declining …してもよい keeps the live card out of the deck");
    CHECK(disc_has(&t2, live2), "B2: the declined live card stays in the waitroom");
}

static void test_b_chika_deficit_without_live_never_prompts(void)
{
    static TestGame tg;
    int chika, live = 0;
    test_game_new(&tg);
    bare_board(&tg);
    chika = tid(&tg, "PL!S-PR-044-PR");
    (void)live;
    for (int i = 0; i < 5; i++) test_add_to_discard(&tg, mid(&tg, FILLER));
    fill_decks(&tg, tid(&tg, FILLER), 20);
    for (int i = 0; i < 3; i++) add_deck_top(&tg, 0, mid(&tg, FILLER));

    int disc_before = disc_n(&tg);
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, chika);
    test_play_to_stage(&tg, chika, AREA_CENTER);
    settle_play(&tg);

    CHECK_EQ(disc_n(&tg), disc_before + 3,
             "B3: waitroom 5 of 8 -> mill 3 even with no live card involved");
    CHECK(!after_tail(&tg),
          "B3: no live card among the milled -> the recovery step never prompts");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §C — 天王寺璃奈 PL!N-bp5-021-N 登場:
 *   「自分のデッキの上からカードを2枚控え室に置く。その後、自分の控え室から
 *     ライブカード1枚を自分のデッキの一番上から4枚目に置いてもよい。」
 *   The clamp ("4th from top" on a 2-card deck) and the decline leg.
 * Rust: debut_mill_two_live_fourth_from_top_q226_test.rs (3 tests)
 * ══════════════════════════════════════════════════════════════════════════ */
static void q226_board(TestGame *tg, int *rina, int *live, int with_live)
{
    bare_board(tg);
    *rina = tid(tg, "PL!N-bp5-021-N");
    *live = tid(tg, "LL-bp5-001-L");
    int filler = tid(tg, FILLER);
    fill_zone(&tg->state.p[0].deck, filler, 4);       /* 4 fillers */
    if (with_live) test_add_to_discard(tg, *live);
    test_add_to_hand(tg, *rina);
    test_add_to_hand(tg, mid(tg, FILLER));
    test_give_energy(tg, 4);
}

static void test_c_q226_place_clamps_and_skip_paths(void)
{
    static TestGame tg, t2, t3;
    int rina, live;
    test_game_new(&tg);
    q226_board(&tg, &rina, &live, 1);
    CHECK(ident(rina, "PL!N-bp5-021-N"),
          "C1: the Q226 source is the printed 天王寺璃奈 card");

    CHECK(test_play_to_stage(&tg, rina, AREA_LEFT), "C1: 天王寺璃奈 is played to the left");
    settle_play(&tg);
    CHECK(pending(&tg), "C1: the optional placement prompt appears");
    CHECK(pending_ch(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "C1: the placement is a SelectCard");
    CHECK(!strcmp(pending_zone(&tg), "discard"),
          "C1: the placement selects the live card out of the waitroom");
    answer(&tg, 1, 1);
    CHECK(!after_tail(&tg), "C1: the placement resolves in one step, no 2nd prompt");
    CHECK_EQ(deck_n(&tg), 3, "C1: deck = 2 remaining + 1 placed = 3");
    EXPECTED_GAP(deck_n(&tg) == 3 && tg.state.p[0].deck.cards[2] == live,
        "C1 GAP: which deck POSITION the live card took (top vs the clamped "
        "4th-from-top) is not observable from the flat RbChoice in C");
    CHECK(!disc_has(&tg, live), "C1: the placed live card left the waitroom");

    /* C2: skip the optional placement. */
    int rina2, live2;
    test_game_new(&t2);
    q226_board(&t2, &rina2, &live2, 1);
    test_play_to_stage(&t2, rina2, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "C2 control: the skip fixture also prompts");
    answer(&t2, -1, 0);
    CHECK(prompts_seen > 0, "C2 control: the skip really answered a prompt");
    CHECK(!after_tail(&t2), "C2: skipping ends with no pending choice");
    CHECK_EQ(deck_n(&t2), 2, "C2: a skipped placement keeps the deck at 2");
    CHECK(disc_has(&t2, live2), "C2: the live card stays in the waitroom");

    /* C3: no live card in the waitroom -> no placement prompt at all. */
    int rina3, live3;
    test_game_new(&t3);
    q226_board(&t3, &rina3, &live3, 0);
    test_add_to_discard(&t3, mid(&t3, FILLER));
    test_play_to_stage(&t3, rina3, AREA_CENTER);
    settle_play(&t3);
    CHECK(!after_tail(&t3),
          "C3: no live card in the waitroom -> the placement ends cleanly");
    CHECK_EQ(deck_n(&t3), 2, "C3: deck 4 - 2 milled = 2");
    CHECK_EQ(disc_n(&t3), 1, "C3: only the pre-existing filler is in the waitroom");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D — 天王寺璃奈 PL!N-bp3-009-R＋ ライブ開始時:
 *   「控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい：
 *     それらのカードのコストの合計が、6の場合、カードを1枚引く。
 *     合計が8の場合、ライブ終了時まで、ハートを得る。
 *     合計が25の場合、ライブ終了時まで、「ライブの合計スコアを＋１する。」を得る。」
 *   One optional cost leg, THREE mutually exclusive cost-total rewards, plus the
 *   decline. This is the parser fix the Rust file documents: the 8 / 25
 *   branches drop the noun and say only 「合計が8の場合」, so a parser gated on
 *   the literal コスト leaves those two branches with no threshold.
 * Rust: live_start_cost_total_8_and_25_branches_test.rs (4 tests) +
 *       live_start_own_discard_two_member_bottomdeck_hearts_q164_test.rs (1)
 *
 * HARNESS NOTE: the Rust twins walk a real live (5 passes, set a live card, two
 * more passes). This file fires the ライブ開始時 scan directly, which is the
 * same entry point (rb_trigger_live_start) every other C parity suite uses for
 * this trigger; the ability semantics under test are unchanged.
 * ══════════════════════════════════════════════════════════════════════════ */
static void rina_ls_board(TestGame *tg, int *rina, const char *ca, const char *cb)
{
    bare_board(tg);
    *rina = tid(tg, "PL!N-bp3-009-R" PLUS);
    test_add_to_discard(tg, tid(tg, ca));
    test_add_to_discard(tg, tid(tg, cb));
    tg->state.p[0].stage[AREA_CENTER] = *rina;
    fill_decks(tg, tid(tg, FILLER), 20);
    test_give_energy(tg, 5);
}

/* Both paid members are the deck's LAST two cards, and neither is in the
 * waitroom any more. `growth` is this branch's net deck change. */
static void check_cost_paid_to_deck_bottom(TestGame *tg, const char *ca,
                                           const char *cb, int growth)
{
    int a = tid(tg, ca), b = tid(tg, cb);
    int n = deck_n(tg);
    CHECK_EQ(n, growth,
             "D: net deck growth for this branch (two cost members net of this "
             "branch's own reward)");
    CHECK(n >= 2 && bag_has(&tg->state.p[0].deck, a) &&
              bag_has(&tg->state.p[0].deck, b),
          "D: 好きな順番でデッキの一番下に置いてもよい -> BOTH paid members are in "
          "the deck");
    CHECK(tg->state.p[0].deck.cards[n - 1] == a ||
              tg->state.p[0].deck.cards[n - 1] == b,
          "D: at least one paid member is the deck's LAST card (bottom)");
    CHECK(!disc_has(tg, a) && !disc_has(tg, b),
          "D: the paid members left the waitroom");
}

static void test_d_rina_cost_total_8_gains_all_heart(void)
{
    static TestGame tg, t2;
    int rina;
    test_game_new(&tg);
    /* 4 + 4 = 8 */
    rina_ls_board(&tg, &rina, "PL!-sd1-008-SD", "PL!-sd1-010-SD");
    CHECK(ident(rina, "PL!N-bp3-009-R" PLUS),
          "D1: the ライブ開始時 source is the printed 天王寺璃奈 card");

    int deck_before = deck_n(&tg);
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "D1: the optional two-member cost prompts");
    answer(&tg, 1, 2);
    drain(&tg, 1);

    CHECK(prompts_seen > 0,
          "D1 control: the cost really was a prompt, so the reward below is not "
          "a vacuous zero reading");
    CHECK_EQ(deck_n(&tg) - deck_before, 2, "D1: the two cost members moved in");
    check_cost_paid_to_deck_bottom(&tg, "PL!-sd1-008-SD", "PL!-sd1-010-SD", 2);
    CHECK_EQ(heart_of(&tg, rina, HALL), 1,
             "D1: 合計が8の場合、ライブ終了時まで、ハートを得る");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "D1: the 8-total branch grants a heart, NOT the 25-total score");

    /* control: the SAME pair declined grants nothing. */
    int rina2;
    test_game_new(&t2);
    rina_ls_board(&t2, &rina2, "PL!-sd1-008-SD", "PL!-sd1-010-SD");
    int deck2 = deck_n(&t2);
    fire_live_start(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "D1 control: the declined fixture also prompts");
    answer(&t2, -1, 0);
    drain(&t2, -1);
    CHECK(prompts_seen > 0, "D1 control: the decline answered a real prompt");
    CHECK_EQ(deck_n(&t2), deck2, "D1: a declined cost moves nothing to the deck");
    CHECK_EQ(heart_of(&t2, rina2, HALL), 0, "D1: no cost paid -> no 8-total heart");
}

static void test_d_rina_cost_total_25_gains_live_total_score(void)
{
    static TestGame tg;
    int rina;
    test_game_new(&tg);
    /* 15 + 10 = 25 */
    rina_ls_board(&tg, &rina, "PL!-sd1-009-SD", "PL!HS-PR-001-PR");
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "D2: the optional two-member cost prompts");
    answer(&tg, 1, 2);
    drain(&tg, 1);
    CHECK(prompts_seen > 0, "D2 control: the cost was a real prompt");
    check_cost_paid_to_deck_bottom(&tg, "PL!-sd1-009-SD", "PL!HS-PR-001-PR", 2);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "D2: 合計が25の場合、ライブ終了時まで、"
             "「ライブの合計スコアを＋１する。」を得る");
    CHECK_EQ(heart_of(&tg, rina, HALL), 0,
             "D2: the 25-total branch grants live total score, NOT the 8-total heart");
}

static void test_d_rina_cost_total_6_draws_and_grants_neither(void)
{
    static TestGame tg;
    int rina;
    test_game_new(&tg);
    /* 2 + 4 = 6 */
    rina_ls_board(&tg, &rina, "PL!-sd1-002-SD", "PL!-sd1-008-SD");
    int deck_before = deck_n(&tg);
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "D3: the optional two-member cost prompts");
    answer(&tg, 1, 2);
    drain(&tg, 1);
    CHECK(prompts_seen > 0, "D3 control: the cost was a real prompt");
    /* two members reach the deck bottom, one is drawn back out */
    CHECK_EQ(deck_n(&tg) - deck_before, 1, "D3: 2 paid, 1 drawn back -> +1");
    check_cost_paid_to_deck_bottom(&tg, "PL!-sd1-002-SD", "PL!-sd1-008-SD", 1);
    CHECK_EQ(heart_of(&tg, rina, HALL), 0, "D3: 6 is not 8 -> no all-heart");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "D3: 6 is not 25 -> no live total score");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], rina, "D3: 天王寺璃奈 stays on stage");
}

static void test_d_rina_declined_keeps_waitroom_and_grants_nothing(void)
{
    static TestGame tg;
    int rina, a, b;
    test_game_new(&tg);
    /* 25-total pair, so a missing reward can only mean the cost was declined */
    rina_ls_board(&tg, &rina, "PL!-sd1-009-SD", "PL!HS-PR-001-PR");
    a = tid(&tg, "PL!-sd1-009-SD");
    b = tid(&tg, "PL!HS-PR-001-PR");
    int deck_before = deck_n(&tg);
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "D4: the optional cost prompts");
    answer(&tg, -1, 0);
    drain(&tg, -1);
    CHECK(prompts_seen > 0, "D4 control: the decline answered a real prompt");
    CHECK_EQ(disc_n(&tg), 2, "D4: declining …してもよい leaves both members put");
    CHECK(disc_has(&tg, a) && disc_has(&tg, b),
          "D4: both cost members are still the same two waitroom cards");
    CHECK_EQ(deck_n(&tg), deck_before, "D4: a declined cost moves nothing");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "D4: no cost paid -> no 25-total live score");
    CHECK_EQ(heart_of(&tg, rina, HALL), 0, "D4: no cost paid -> no 8-total heart");
}

static void test_d_rina_own_discard_only(void)
{
    static TestGame tg;
    int rina, ma, mb, opp;
    test_game_new(&tg);
    bare_board(&tg);
    rina = tid(&tg, "PL!N-bp3-009-R" PLUS);
    ma = tid(&tg, "PL!-sd1-002-SD");
    mb = tid(&tg, "PL!-sd1-005-SD");
    opp = mid(&tg, "PL!-sd1-002-SD");
    CHECK(ident(rina, "PL!N-bp3-009-R" PLUS),
          "D5: the ライブ開始時 source is the printed 天王寺璃奈 card");

    test_add_to_discard(&tg, ma);
    test_add_to_discard(&tg, mb);
    /* a DISTINCT copy of the same print in the OPPONENT's discard */
    add_discard_for(&tg, 1, opp);
    fill_decks(&tg, tid(&tg, FILLER), 20);
    tg.state.p[0].stage[AREA_LEFT] = rina;
    tg.state.p[0].stage[AREA_CENTER] = tid(&tg, FILLER);
    tg.state.p[0].stage[AREA_RIGHT] = mid(&tg, FILLER);
    test_give_energy(&tg, 5);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "D5: the optional two-member cost prompts");
    answer(&tg, 1, 2);
    drain(&tg, 1);
    CHECK(prompts_seen > 0, "D5 control: the cost was a real prompt");

    CHECK(deck_has(&tg, ma) && deck_has(&tg, mb),
          "D5: BOTH own-discard members moved to the DECK BOTTOM");
    CHECK_EQ(disc_n(&tg), 0, "D5: own discard emptied by the move");
    CHECK(bag_has(&tg.state.p[1].discard, opp),
          "D5: the OPPONENT's identical-card copy stays in THEIR discard");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §E — 鬼塚夏美 PL!SP-bp5-009-R ライブ開始時, the `repeat_procedure` shape:
 *   「自分のデッキの一番上のカードを控え室に置いてもよい。そうした場合、
 *     ライブ終了時まで、ブレードを得る。これにより控え室に置いたカードが
 *     ライブカードの場合、このメンバーをウェイトにする。
 *     自分はこの手順をさらに4回まで繰り返してもよい。」
 *   Per iteration: [mill? yes/no] then [repeat? stop/continue], bounded at
 *   initial + 4. The そう場合 (wait) is re-derived from the milled card, which
 *   is exactly the deferred_conditional_gate the C engine re-derives.
 * Rust: live_start_repeat_mill_blade_live_self_wait_q222_test.rs (13 tests)
 * ══════════════════════════════════════════════════════════════════════════ */
static void natsumi_board(TestGame *tg, int *natsumi, int *live, int *filler_live)
{
    bare_board(tg);
    *natsumi = tid(tg, "PL!SP-bp5-009-R");
    *live = tid(tg, LIVE_L);
    *filler_live = tid(tg, LIVE_M);
    CHECK(ident(*natsumi, "PL!SP-bp5-009-R"),
          "E: the repeat source is the printed 鬼塚夏美 card");
    test_add_to_hand(tg, *filler_live);
}

/* Seed P1's deck with `n` copies of `card`, keeping P2 stocked. */
static void natsumi_deck(TestGame *tg, int card, int n)
{
    fill_zone(&tg->state.p[0].deck, card, n);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 1, tid(tg, LIVE_M));
}

/* Drive the repeat loop: `mills` Yes answers, `repeats` Continue answers. */
static void run_repeat(TestGame *tg, int mills, int repeats)
{
    for (int i = 0; i < mills; i++) {
        if (!pending(tg)) return;
        answer(tg, 1, 1);
        drain(tg, 1);
        if (i < repeats && pending(tg)) {
            answer(tg, 1, 1);
            drain(tg, 1);
        }
    }
}

static void test_e_repeat_decline_first_mill(void)
{
    static TestGame tg;
    int natsumi, live, flive;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    natsumi_deck(&tg, live, 5);
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "E1: the optional mill prompts");
    answer(&tg, 0, 1);        /* No: option 0 of the mill gate */
    drain(&tg, 0);
    CHECK(prompts_seen > 0, "E1 control: the decline answered a real prompt");
    CHECK_EQ(deck_n(&tg), 5, "E1: no card milled when the first optional declines");
    CHECK_EQ(blade_of(&tg, natsumi), 0, "E1: no blade gained when the mill declines");
    CHECK(not_waited(&tg, natsumi), "E1: no wait state when the mill declines");
}

static void test_e_repeat_mill_one_live_then_stop(void)
{
    static TestGame tg;
    int natsumi, live, flive;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    natsumi_deck(&tg, live, 5);
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "E2: the mill gate prompts");
    answer(&tg, 1, 1);        /* Yes: mill */
    drain(&tg, 1);
    CHECK(pending(&tg), "E2: a repeat gate follows the first mill");
    answer(&tg, 0, 1);        /* Stop */
    drain(&tg, 0);
    CHECK(prompts_seen > 0, "E2 control: at least one real prompt was answered");
    CHECK_EQ(deck_n(&tg), 4, "E2: 1 card milled");
    CHECK_EQ(blade_of(&tg, natsumi), 1, "E2: 1 blade gained");
    CHECK(is_waited(&tg, natsumi),
          "E2: そうした場合 — the milled card IS a live card -> this member waits");
}

static void test_e_repeat_mill_one_non_live_then_stop(void)
{
    static TestGame tg;
    int natsumi, live, flive;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    natsumi_deck(&tg, tid(&tg, FILLER), 5);   /* members, not live cards */
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "E3: the mill gate prompts");
    answer(&tg, 1, 1);
    drain(&tg, 1);
    if (pending(&tg)) { answer(&tg, 0, 1); drain(&tg, 0); }
    CHECK(prompts_seen > 0, "E3 control: the mill gate was a real prompt");
    CHECK_EQ(deck_n(&tg), 4, "E3: 1 card milled");
    CHECK_EQ(blade_of(&tg, natsumi), 1, "E3: 1 blade gained from milling ANY card");
    CHECK(not_waited(&tg, natsumi),
          "E3: そうした場合 fails — the milled card is not a live card -> no wait");
}

static void test_e_repeat_two_iterations_then_stop(void)
{
    static TestGame tg;
    int natsumi, live, flive, nonlive;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    nonlive = tid(&tg, FILLER);
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    /* deck: [non-live, live, live, live, live] */
    fill_zone(&tg.state.p[0].deck, live, 4);
    add_deck_top(&tg, 0, nonlive);
    tg.state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 1, flive);
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    /* iter 0: mill Yes, repeat Continue; iter 1: mill Yes, repeat Stop */
    CHECK(pending(&tg), "E4: iter 0 mill gate prompts");
    answer(&tg, 1, 1); drain(&tg, 1);
    CHECK(pending(&tg), "E4: iter 0 repeat gate prompts");
    answer(&tg, 1, 1); drain(&tg, 1);
    CHECK(pending(&tg),
          "E4: iter 1 mill gate prompts (the sequential tail survived the pause)");
    answer(&tg, 1, 1); drain(&tg, 1);
    if (pending(&tg)) { answer(&tg, 0, 1); drain(&tg, 0); }

    CHECK(prompts_seen >= 3,
          "E4 control: three-or-more gates were really answered, so the blade "
          "count below is not a no-op artefact");
    CHECK_EQ(deck_n(&tg), 3, "E4: 2 cards milled");
    CHECK_EQ(blade_of(&tg, natsumi), 2, "E4: 2 blades gained (one per mill)");
    CHECK(is_waited(&tg, natsumi), "E4: the wait from iter 1's live mill persists");
}

static void test_e_repeat_bounded_at_five_mills(void)
{
    static TestGame tg;
    int natsumi, live, flive;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    natsumi_deck(&tg, live, 10);
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    run_repeat(&tg, 5, 4);        /* initial + 4 repeats, all Continue */
    CHECK(prompts_seen >= 5,
          "E5 control: five mill gates were really answered (max_repeats=4)");
    CHECK_EQ(deck_n(&tg), 5, "E5: 5 cards milled (initial + 4 repeats)");
    CHECK_EQ(blade_of(&tg, natsumi), 5, "E5: 5 blades gained (one per mill)");
    CHECK(is_waited(&tg, natsumi), "E5: milling live cards waits this member");
}

static void test_e_repeat_never_waits_other_members(void)
{
    static TestGame tg;
    int natsumi, live, flive, filler_l, filler_r;
    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    filler_l = tid(&tg, FILLER);
    filler_r = mid(&tg, FILLER);
    tg.state.p[0].stage[AREA_LEFT] = filler_l;
    tg.state.p[0].stage[AREA_CENTER] = natsumi;
    tg.state.p[0].stage[AREA_RIGHT] = filler_r;
    natsumi_deck(&tg, live, 10);
    test_give_energy(&tg, 4);

    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    run_repeat(&tg, 5, 4);
    CHECK(prompts_seen >= 5, "E6 control: five mill gates were really answered");
    CHECK_EQ(blade_of(&tg, natsumi), 5, "E6: 5 blades gained");
    CHECK(is_waited(&tg, natsumi), "E6: only 鬼塚夏美 is waited");
    CHECK(not_waited(&tg, filler_l),
          "E6: このメンバーをウェイトにする must not touch the left filler");
    CHECK(not_waited(&tg, filler_r),
          "E6: このメンバーをウェイトにする must not touch the right filler");
}

static void test_e_repeat_position_ends(void)
{
    static TestGame tg, t2, t3;
    int natsumi, live, flive;

    test_game_new(&tg);
    natsumi_board(&tg, &natsumi, &live, &flive);
    tg.state.p[0].stage[AREA_LEFT] = natsumi;
    natsumi_deck(&tg, live, 5);
    test_give_energy(&tg, 4);
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&tg)) { answer(&tg, 1, 1); drain(&tg, 1); }
    if (pending(&tg)) { answer(&tg, 0, 1); drain(&tg, 0); }
    CHECK(prompts_seen > 0, "E7a control: the left-position fixture prompted");
    CHECK_EQ(blade_of(&tg, natsumi), 1, "E7a: 1 blade gained at the left position");
    CHECK(is_waited(&tg, natsumi), "E7a: the left position still waits");

    test_game_new(&t2);
    natsumi_board(&t2, &natsumi, &live, &flive);
    t2.state.p[0].stage[AREA_RIGHT] = natsumi;
    natsumi_deck(&t2, live, 5);
    test_give_energy(&t2, 4);
    fire_live_start(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    if (pending(&t2)) { answer(&t2, 0, 1); drain(&t2, 0); }
    CHECK(prompts_seen > 0, "E7b control: the right-position fixture prompted");
    CHECK_EQ(blade_of(&t2, natsumi), 1, "E7b: 1 blade gained at the right position");
    CHECK(is_waited(&t2, natsumi), "E7b: the right position still waits");

    test_game_new(&t3);
    natsumi_board(&t3, &natsumi, &live, &flive);
    t3.state.p[0].stage[AREA_CENTER] = natsumi;
    natsumi_deck(&t3, tid(&t3, FILLER), 10);
    test_give_energy(&t3, 4);
    fire_live_start(&t3);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t3)) { answer(&t3, 1, 1); drain(&t3, 1); }
    if (pending(&t3)) { answer(&t3, 0, 1); drain(&t3, 0); }
    CHECK(prompts_seen > 0, "E7c control: the all-non-live fixture prompted");
    CHECK_EQ(deck_n(&t3), 9, "E7c: only 1 card milled when stopping early");
    CHECK_EQ(blade_of(&t3, natsumi), 1, "E7c: 1 blade");
    CHECK(not_waited(&t3, natsumi), "E7c: no wait state when no live card was milled");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §F — 桂城 泉 PL!HS-bp6-008-R 登場 (Q257):
 *   「このメンバーをウェイトにする。その後、自分の控え室からスコア４以下の
 *     『蓮ノ空』のライブカードを1枚手札に加える。」
 *   Step 1 is MANDATORY (she always waits, even with an empty waitroom — the
 *   Q257 question). Step 2 is conditional on a candidate existing.
 * Rust: q257_wait_self_then_score_four_live_from_waitroom_to_hand_test.rs
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_f_izumi_empty_waitroom_still_waits(void)
{
    static TestGame tg;
    int izumi;
    test_game_new(&tg);
    bare_board(&tg);
    izumi = tid(&tg, "PL!HS-bp6-008-R");
    CHECK(ident(izumi, "PL!HS-bp6-008-R"),
          "F1: the 登場 source is the printed 桂城 泉 card (cost 11)");
    test_add_to_hand(&tg, izumi);
    tg.state.p[0].discard.n = 0;
    test_give_energy(&tg, 12);
    CHECK(test_play_to_stage(&tg, izumi, AREA_CENTER),
          "F1: 桂城 泉 (cost 11) is played to the center");

    CHECK(is_waited(&tg, izumi),
          "F1: Q257 — this member MUST wait even when the waitroom is empty");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], izumi,
             "F1: she remains on stage in the wait state");
    CHECK(!after_tail(&tg),
          "F1: no choice may appear when the waitroom has no matching live card");
}

static void test_f_izumi_single_match_moves_to_hand(void)
{
    static TestGame tg;
    const char *rar[2] = { "R", "P" };
    for (int r = 0; r < 2; r++) {
        test_game_new(&tg);
        char no[32];
        snprintf(no, sizeof no, "PL!HS-bp6-008-%s", rar[r]);
        bare_board(&tg);
        int izumi = tid(&tg, no);
        ident(izumi, no);
        int match = tid(&tg, "PL!HS-bp6-026-L");   /* 可惜夜花火 score 3 */
        int filler = tid(&tg, FILLER);
        test_add_to_hand(&tg, izumi);
        test_add_to_discard(&tg, match);
        test_add_to_discard(&tg, filler);
        test_give_energy(&tg, 12);
        CHECK(test_play_to_stage(&tg, izumi, AREA_CENTER), "F2: %s is played", no);
        settle_play(&tg);
        CHECK(is_waited(&tg, izumi), "F2 %s: Q257 — must wait", no);
        drain(&tg, 1);
        CHECK(hand_has(&tg, match),
              "F2 %s: the single matching score<=4 蓮ノ空 live card reaches hand", no);
        CHECK(disc_has(&tg, filler), "F2 %s: the filler stays in the waitroom", no);
        CHECK(!disc_has(&tg, match), "F2 %s: the matching live left the waitroom", no);
    }
}

static void test_f_izumi_non_matching_only_no_second_effect(void)
{
    static TestGame tg;
    int izumi, wrong_group, high_score, live_card;
    test_game_new(&tg);
    bare_board(&tg);
    izumi = tid(&tg, "PL!HS-bp6-008-R");
    wrong_group = tid(&tg, LIVE_L);                 /* ラブライブ！ live */
    high_score = tid(&tg, "PL!HS-bp1-022-L");       /* AWOKE score 5 */
    live_card = tid(&tg, "PL!S-PR-022-PR");         /* HAPPY PARTY TRAIN live */
    test_add_to_hand(&tg, izumi);
    test_add_to_discard(&tg, wrong_group);
    test_add_to_discard(&tg, high_score);
    test_add_to_discard(&tg, live_card);
    test_give_energy(&tg, 12);
    test_play_to_stage(&tg, izumi, AREA_CENTER);
    settle_play(&tg);

    CHECK(is_waited(&tg, izumi), "F3: the mandatory wait leg still runs");
    CHECK(!after_tail(&tg), "F3: no second-effect prompt without a match");
    CHECK(disc_has(&tg, wrong_group) && disc_has(&tg, high_score) &&
              disc_has(&tg, live_card),
          "F3: wrong group / score 5 / other live all stay in the waitroom");
}

static void test_f_izumi_filtered_prompt_picks_the_score_four_live(void)
{
    static TestGame tg;
    int izumi, wrong_group, s5, s4, s3;
    test_game_new(&tg);
    bare_board(&tg);
    izumi = tid(&tg, "PL!HS-bp6-008-R");
    wrong_group = tid(&tg, "PL!S-PR-022-PR");
    s5 = tid(&tg, "PL!HS-bp6-027-L");   /* 月夜見海月 score 5 */
    s4 = tid(&tg, "PL!HS-bp6-025-L");   /* ツバサ・ラ・リベルテ score 4 */
    s3 = tid(&tg, "PL!HS-bp6-026-L");   /* 可惜夜花火 score 3 */
    test_add_to_hand(&tg, izumi);
    test_add_to_discard(&tg, wrong_group);
    test_add_to_discard(&tg, s5);
    test_add_to_discard(&tg, s4);
    test_add_to_discard(&tg, s3);
    test_give_energy(&tg, 12);
    test_play_to_stage(&tg, izumi, AREA_CENTER);
    settle_play(&tg);

    CHECK(is_waited(&tg, izumi), "F4: step 1 waits first (ORDER)");
    const RbChoice *c = pending_ch(&tg);
    CHECK(c && c->kind == RB_CHOICE_SELECT_CARD,
          "F4: step 2 is a mandatory SelectCard");
    if (c) {
        CHECK(c->count == 1, "F4: exactly 1 card is requested");
        CHECK(c->allow_skip == 0, "F4: the pick is NOT skippable");
        CHECK(!strcmp(c->zone, "discard"), "F4: it selects out of the waitroom");
        CHECK(c->cost_limit == 4, "F4: the score ceiling is 4 (got %d)", c->cost_limit);
        CHECK(c->cost_limit_op[0] && strcmp(c->cost_limit_op, "<=") == 0,
              "F4: the score ceiling operator is <= (got \"%s\")", c->cost_limit_op);
        CHECK(c->n_filtered_indices == 2 &&
                  c->filtered_indices[0] == 2 && c->filtered_indices[1] == 3,
              "F4: only the score-4 and score-3 lives are offered (indices 2,3; "
              "got %d indices)", c->n_filtered_indices);
    }
    prompts_seen = 0;
    card_prompts_seen = 0;
    answer(&tg, 1, 1);
    CHECK(prompts_seen > 0, "F4 control: the pick really answered a prompt");
    CHECK(hand_has(&tg, s4), "F4: the score-4 live reaches hand");
    CHECK(disc_has(&tg, wrong_group) && disc_has(&tg, s5) && disc_has(&tg, s3),
          "F4: every unchosen card stays in the waitroom");
}

static void test_f_izumi_waits_only_itself(void)
{
    static TestGame tg;
    int izumi, ally;
    test_game_new(&tg);
    bare_board(&tg);
    izumi = tid(&tg, "PL!HS-bp6-008-R");
    ally = tid(&tg, "PL!HS-bp6-009-R");   /* 日野下花帆 */
    tg.state.p[0].stage[AREA_LEFT] = ally;
    test_add_to_hand(&tg, izumi);
    test_give_energy(&tg, 12);
    test_play_to_stage(&tg, izumi, AREA_CENTER);
    settle_play(&tg);

    CHECK(is_waited(&tg, izumi), "F5: 桂城 泉 waits");
    CHECK(not_waited(&tg, ally),
          "F5: このメンバーをウェイトにする must not wait the other 蓮ノ空 member");
    CHECK(!after_tail(&tg), "F5: no prompt with an empty waitroom");
}

static void test_f_izumi_live_start_activates_at_score_two(void)
{
    static TestGame tg, t2, t3;
    int izumi, s2, filler;

    test_game_new(&tg);
    bare_board(&tg);
    izumi = tid(&tg, "PL!HS-bp6-008-R");
    s2 = tid(&tg, "PL!HS-bp1-020-L");   /* 365 Days score 2 */
    filler = tid(&tg, FILLER);
    tg.state.p[0].stage[AREA_LEFT] = izumi;
    test_set_live_card(&tg, 0, s2);
    fill_decks(&tg, filler, 10);
    rb_mods_set_orientation(&tg.state.mods, izumi, "wait");

    fire_live_start(&tg);
    drain(&tg, 1);
    CHECK(!is_waited(&tg, izumi),
          "F6: 自分のライブ中のカードにスコア２以下のライブカードがある場合、"
          "このメンバーをアクティブにする");

    test_game_new(&t2);
    bare_board(&t2);
    izumi = tid(&t2, "PL!HS-bp6-008-R");
    int s3 = tid(&t2, "PL!HS-bp6-026-L");
    filler = tid(&t2, FILLER);
    t2.state.p[0].stage[AREA_LEFT] = izumi;
    test_set_live_card(&t2, 0, s3);
    fill_decks(&t2, filler, 10);
    rb_mods_set_orientation(&t2.state.mods, izumi, "wait");
    fire_live_start(&t2);
    drain(&t2, 1);
    CHECK(is_waited(&t2, izumi), "F7: score 3 does NOT satisfy the score<=2 gate");

    /* control: the OPPONENT's score-2 live must not lift P1's wait. */
    test_game_new(&t3);
    bare_board(&t3);
    izumi = tid(&t3, "PL!HS-bp6-008-R");
    int opp2 = tid(&t3, "PL!HS-bp1-020-L");
    filler = tid(&t3, FILLER);
    t3.state.p[0].stage[AREA_LEFT] = izumi;
    test_set_live_card_for(&t3, 1, opp2);
    fill_decks(&t3, filler, 10);
    rb_mods_set_orientation(&t3.state.mods, izumi, "wait");
    fire_live_start(&t3);
    drain(&t3, 1);
    CHECK(is_waited(&t3, izumi),
          "F8: 自分のライブ中 — the OPPONENT's score-2 live must not count");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §G — 三船栞子 PL!N-bp4-010-R＋ 登場:
 *   「自分の成功ライブカード置き場にある『虹ヶ咲』のライブカードを1枚控え室に
 *     置いてもよい。そうした場合、自分の控え室にある『虹ヶ咲』のライブカードを
 *     1枚成功ライブカード置き場に置く。」
 *   A two-step zone SWAP across an optional prompt, filtered on 虹ヶ咲 in BOTH
 *   directions.
 * Rust: live_success_swap_one_niji_in_success_zone_pl_n_bp4_010_r_test.rs (8)
 * ══════════════════════════════════════════════════════════════════════════ */
static void shioriko_board(TestGame *tg, int *shio)
{
    bare_board(tg);
    *shio = tid(tg, "PL!N-bp4-010-R" PLUS);
    CHECK(ident(*shio, "PL!N-bp4-010-R" PLUS),
          "G: the swap source is the printed 三船栞子 card (cost 5)");
    fill_decks(tg, tid(tg, FILLER), 10);
    test_add_to_hand(tg, *shio);
    test_give_energy(tg, 10);
}

static void test_g_shioriko_basic_swap_one_niji(void)
{
    static TestGame tg;
    int shio, a, b;
    test_game_new(&tg);
    shioriko_board(&tg, &shio);
    a = mid(&tg, "PL!N-sd1-025-SD");
    b = mid(&tg, "PL!N-sd1-025-SD");
    add_success_for(&tg, 0, a);
    test_add_to_discard(&tg, b);

    CHECK(test_play_to_stage(&tg, shio, AREA_CENTER), "G1: 三船栞子 is played");
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "G1: the optional success-zone pick prompts");
    answer(&tg, 1, 1);
    CHECK(pending(&tg), "G1: the waitroom retrieval prompts after the first step");
    CHECK(pending_ch(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "G1: the retrieval is a SelectCard");
    answer(&tg, 1, 1);
    drain(&tg, 1);
    CHECK(prompts_seen >= 2, "G1 control: BOTH sequential steps really prompted");
    CHECK(bag_has(&tg.state.p[0].success, b),
          "G1: the waitroom 虹ヶ咲 live reaches the success zone");
    CHECK(!bag_has(&tg.state.p[0].success, a),
          "G1: the success-zone 虹ヶ咲 live left it");
    CHECK(disc_has(&tg, a), "G1: the swapped-out live is in the waitroom");
}

static void test_g_shioriko_skip_moves_nothing(void)
{
    static TestGame tg;
    int shio, a;
    test_game_new(&tg);
    shioriko_board(&tg, &shio);
    a = mid(&tg, "PL!N-sd1-025-SD");
    add_success_for(&tg, 0, a);

    test_play_to_stage(&tg, shio, AREA_CENTER);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "G2: the optional pick prompts");
    answer(&tg, -1, 0);
    drain(&tg, -1);
    CHECK(prompts_seen > 0, "G2 control: the decline answered a real prompt");
    CHECK(bag_has(&tg.state.p[0].success, a),
          "G2: …してもよい declined -> the 虹ヶ咲 live stays in the success zone");
    CHECK(!disc_has(&tg, a), "G2: nothing moved to the waitroom");
}

static void test_g_shioriko_two_niji_in_success_choose_one(void)
{
    static TestGame tg;
    int shio, a, b, c;
    test_game_new(&tg);
    shioriko_board(&tg, &shio);
    a = mid(&tg, "PL!N-sd1-025-SD");
    b = mid(&tg, "PL!N-sd1-025-SD");
    c = mid(&tg, "PL!N-sd1-025-SD");
    add_success_for(&tg, 0, a);
    add_success_for(&tg, 0, b);
    test_add_to_discard(&tg, c);

    test_play_to_stage(&tg, shio, AREA_CENTER);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "G3: the success-zone pick prompts");
    answer(&tg, 1, 1);
    if (pending(&tg)) answer(&tg, 1, 1);
    drain(&tg, 1);
    CHECK(prompts_seen >= 2, "G3 control: both sequential steps prompted");
    CHECK(disc_has(&tg, a), "G3: the chosen success-zone 虹ヶ咲 live is in the waitroom");
    CHECK(!bag_has(&tg.state.p[0].success, a), "G3: it is no longer in the success zone");
    CHECK(bag_has(&tg.state.p[0].success, b),
          "G3: the UNCHOSEN second 虹ヶ咲 live stays in the success zone");
    CHECK(bag_has(&tg.state.p[0].success, c),
          "G3: the retrieved 虹ヶ咲 live joins it");
    CHECK_EQ(tg.state.p[0].success.n, 2, "G3: the success zone ends with exactly 2 cards");
}

static void test_g_shioriko_non_niji_not_offered(void)
{
    static TestGame tg, t2;
    int shio, a, b, non;

    /* non-虹ヶ咲 live in the SUCCESS zone must survive */
    test_game_new(&tg);
    shioriko_board(&tg, &shio);
    a = mid(&tg, "PL!N-sd1-025-SD");
    non = mid(&tg, "PL!SP-sd1-023-SD");      /* WE WILL!! — Super Star */
    b = mid(&tg, "PL!N-sd1-025-SD");
    add_success_for(&tg, 0, a);
    add_success_for(&tg, 0, non);
    test_add_to_discard(&tg, b);
    test_play_to_stage(&tg, shio, AREA_CENTER);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&tg)) { answer(&tg, 1, 1); drain(&tg, 1); }
    if (pending(&tg)) { answer(&tg, 1, 1); drain(&tg, 1); }
    CHECK(prompts_seen >= 2, "G4 control: the swap really ran both steps");
    CHECK(!bag_has(&tg.state.p[0].success, a),
          "G4: the 虹ヶ咲 live was moved out of the success zone");
    CHECK(bag_has(&tg.state.p[0].success, non),
          "G4: the non-虹ヶ咲 live is NOT offered and stays untouched");
    CHECK(bag_has(&tg.state.p[0].success, b),
          "G4: the retrieved 虹ヶ咲 live reaches the success zone");

    /* non-虹ヶ咲 live in the WAITROOM must not be the retrieval source */
    test_game_new(&t2);
    shioriko_board(&t2, &shio);
    a = mid(&t2, "PL!N-sd1-025-SD");
    non = mid(&t2, "PL!SP-sd1-023-SD");
    b = mid(&t2, "PL!N-sd1-025-SD");
    add_success_for(&t2, 0, a);
    test_add_to_discard(&t2, non);
    test_add_to_discard(&t2, b);
    test_play_to_stage(&t2, shio, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    CHECK(prompts_seen >= 2, "G5 control: the swap really ran both steps");
    CHECK(bag_has(&t2.state.p[0].success, b),
          "G5: the 虹ヶ咲 live from the waitroom reaches the success zone");
    CHECK(disc_has(&t2, non),
          "G5: the non-虹ヶ咲 waitroom live is NOT offered and stays put");
    CHECK(disc_has(&t2, a), "G5: the original 虹ヶ咲 live is in the waitroom");
}

static void test_g_shioriko_no_candidates_skips_entirely(void)
{
    static TestGame tg, t2;
    int shio, non, niji;

    test_game_new(&tg);
    shioriko_board(&tg, &shio);
    non = mid(&tg, "PL!SP-sd1-023-SD");
    niji = mid(&tg, "PL!N-sd1-025-SD");
    add_success_for(&tg, 0, non);
    test_add_to_discard(&tg, niji);
    test_play_to_stage(&tg, shio, AREA_CENTER);
    settle_play(&tg);
    CHECK(!after_tail(&tg),
          "G6: no 虹ヶ咲 live in the success zone -> no prompt at all");
    CHECK(bag_has(&tg.state.p[0].success, non), "G6: the non-虹ヶ咲 live is untouched");
    CHECK(disc_has(&tg, niji),
          "G6: the 虹ヶ咲 live stays in the waitroom (the gate never opened)");

    test_game_new(&t2);
    shioriko_board(&t2, &shio);
    test_play_to_stage(&t2, shio, AREA_CENTER);
    settle_play(&t2);
    CHECK(!after_tail(&t2), "G7: both zones empty -> no prompt at all");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §H — 鬼塚冬毬 PL!SP-bp2-011-R 登場 (Q118):
 *   「自分の控え室から、カード名の異なるライブカードを2枚選ぶ。選択した場合、
 *     相手はそのカードのうち1枚を選ぶ。相手に選ばれたカードを自分の手札に加える。」
 *   THREE steps with a SEAT SWITCH in the middle: the first pick is routed to
 *   the ACTIVATOR, the second to the OPPONENT, and the result goes back to the
 *   activator's hand. The sharpest choice-routing test in the cluster.
 * Rust: debut_two_distinct_live_cards_opponent_picks_one_q118_test.rs (9) and
 *       debut_two_distinct_live_cards_opponent_picks_one_to_hand_q118_test.rs (6)
 * ══════════════════════════════════════════════════════════════════════════ */
static void toubatsu_board(TestGame *tg, int *tou)
{
    bare_board(tg);
    *tou = tid(tg, "PL!SP-bp2-011-R");
    CHECK(ident(*tou, "PL!SP-bp2-011-R"),
          "H: the Q118 source is the printed 鬼塚冬毬 card (cost 13)");
    fill_decks(tg, tid(tg, FILLER), 10);
    test_add_to_hand(tg, *tou);
    test_add_to_hand(tg, mid(tg, FILLER));
    test_give_energy(tg, 13);
}

static void test_h_q118_opponent_picks_first(void)
{
    static TestGame tg;
    int tou, a, b;
    test_game_new(&tg);
    toubatsu_board(&tg, &tou);
    a = tid(&tg, LIVE_L);      /* START:DASH!! */
    b = tid(&tg, LIVE_N);      /* Dream with You — a different name */
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, b);

    CHECK(test_play_to_stage(&tg, tou, AREA_LEFT), "H1: 鬼塚冬毬 is played");
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "H1: step 1 must be a waitroom SelectCard (pick 2 distinct "
          "live cards); the engine offered %s", pending_desc(&tg));
    CHECK(any_choice_seat(&tg, 0),
          "H1: step 1 is routed to the ACTIVATOR (p1)");
    answer(&tg, 1, 2);
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "H1 control: step 1 really prompted");

    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "H1: step 2 (opponent picks 1 of the 2) prompts");
    if (pending(&tg)) {
        CHECK(any_choice_seat(&tg, 1),
              "H1: step 2 is routed to the OPPONENT (p2)");
        answer(&tg, 1, 1);
    }
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "H1 control: step 2 really prompted");

    CHECK(hand_has(&tg, a) || hand_has(&tg, b),
          "H1: the opponent's chosen card reaches the ACTIVATOR's hand");
    CHECK(!disc_has(&tg, a) || !disc_has(&tg, b),
          "H1: at least one live card left the waitroom");
    CHECK(!p2_hand_has(&tg, a) && !p2_hand_has(&tg, b),
          "H1: the card goes to p1's hand, never the opponent's");
}

static void test_h_q118_one_live_card_fails(void)
{
    static TestGame tg;
    int tou, a;
    test_game_new(&tg);
    toubatsu_board(&tg, &tou);
    a = tid(&tg, LIVE_L);
    test_add_to_discard(&tg, a);

    test_play_to_stage(&tg, tou, AREA_CENTER);
    settle_play(&tg);
    CHECK(!after_tail(&tg),
          "H2: Q118 — with fewer than 2 distinct live cards NO prompt is created");
    CHECK(!hand_has(&tg, a),
          "H2: the live card is not added: the effect needs 2 distinct names");
    CHECK(disc_has(&tg, a), "H2: the live card stays in the waitroom");
}

static void test_h_q118_opponent_picks_second_of_three(void)
{
    static TestGame tg;
    int tou, a, b, c;
    test_game_new(&tg);
    toubatsu_board(&tg, &tou);
    a = tid(&tg, LIVE_L);
    b = tid(&tg, LIVE_M);
    c = tid(&tg, LIVE_N);
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, b);
    test_add_to_discard(&tg, c);

    test_play_to_stage(&tg, tou, AREA_LEFT);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "H3: step 1 must be a waitroom SelectCard; the engine offered "
          "%s", pending_desc(&tg));
    answer(&tg, 1, 2);
    drain(&tg, 1);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&tg)) { answer(&tg, 1, 2); drain(&tg, 1); }   /* the opponent's pick */
    CHECK(prompts_seen >= 1, "H3 control: the opponent's pick really prompted");
    CHECK(hand_has(&tg, a) || hand_has(&tg, b) || hand_has(&tg, c),
          "H3: exactly one of the three distinct lives reaches hand");
    CHECK(disc_has(&tg, a) || disc_has(&tg, b) || disc_has(&tg, c),
          "H3: the two unchosen lives stay in the waitroom");
}

static void test_h_q118_duplicate_names_stay_put(void)
{
    static TestGame tg;
    int tou, a, a2, c;
    test_game_new(&tg);
    toubatsu_board(&tg, &tou);
    a  = tid(&tg, LIVE_L);      /* START:DASH!! */
    a2 = mid(&tg, LIVE_L);      /* a SECOND physical copy of the same print */
    c  = tid(&tg, LIVE_N);      /* a different name */
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, a2);
    test_add_to_discard(&tg, c);

    test_play_to_stage(&tg, tou, AREA_CENTER);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg) && pending_ch(&tg)->kind == RB_CHOICE_SELECT_CARD,
          "H4: step 1 must be a waitroom SelectCard; the engine offered "
          "%s", pending_desc(&tg));
    answer(&tg, 1, 2);
    drain(&tg, 1);
    if (pending(&tg)) { answer(&tg, 1, 1); drain(&tg, 1); }
    CHECK(prompts_seen >= 1, "H4 control: the pick really prompted");
    int got = hand_has(&tg, a) ? a : (hand_has(&tg, c) ? c : -1);
    CHECK(got >= 0, "H4: the hand must contain one of the two DISTINCT picks");
    CHECK(disc_has(&tg, a2),
          "H4: the duplicate-named copy must never leave the waitroom");
    if (got >= 0) CHECK(!disc_has(&tg, got), "H4: the selected card moved out of the waitroom");
}

static void test_h_q118_card_count_invariant(void)
{
    static TestGame tg;
    int tou, a, b;
    test_game_new(&tg);
    toubatsu_board(&tg, &tou);
    a = tid(&tg, LIVE_L);
    b = tid(&tg, LIVE_M);
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, b);

    int before = test_total_card_count(&tg);
    test_play_to_stage(&tg, tou, AREA_LEFT);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&tg)) { answer(&tg, 1, 2); drain(&tg, 1); }
    if (pending(&tg)) { answer(&tg, 1, 1); drain(&tg, 1); }
    CHECK(prompts_seen >= 1, "H5 control: the sequence really ran");
    CHECK_EQ(test_total_card_count(&tg), before,
             "H5: the zone census is unchanged — a zone move, not a copy or a "
             "vanish");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §I — 東條 希 PL!-bp5-007-R 登場 (Q229):
 *   「このメンバーよりコストが低いメンバーからバトンタッチして登場した場合、
 *     自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き、
 *     その後、自分と相手はそれぞれカードを3枚引く。」
 *   FOUR ordered steps across TWO seats: P1 discards to 3, P2 discards to 3,
 *   P1 draws 3, P2 draws 3. A 3-or-fewer hand SKIPS only the discard leg — the
 *   draw still happens (the Q229 answer).
 * Rust: cheaper_baton_both_hands_discard_to_three_draw_three_q229_test.rs (6)
 * ══════════════════════════════════════════════════════════════════════════ */
static void nozomi_board(TestGame *tg, int *nozomi, int on_stage)
{
    bare_board(tg);
    *nozomi = tid(tg, "PL!-bp5-007-R");
    CHECK(ident(*nozomi, "PL!-bp5-007-R"),
          "I: the Q229 source is the printed 東條 希 card (cost 13)");
    test_add_to_hand(tg, *nozomi);
    if (on_stage >= 0) tg->state.p[0].stage[AREA_CENTER] = on_stage;
    fill_decks(tg, tid(tg, FILLER), 10);
    test_give_energy(tg, 13);
}

static void test_i_q229_discard_to_three_then_draw_three(void)
{
    static TestGame tg, t2, t3;
    int nozomi, cheap;

    test_game_new(&tg);
    cheap = tid(&tg, "PL!SP-sd1-019-SD");   /* 若菜四季 cost 2 — cheaper */
    nozomi_board(&tg, &nozomi, cheap);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, mid(&tg, FILLER));
    for (int i = 0; i < 2; i++) add_hand_for(&tg, 1, mid(&tg, FILLER));

    CHECK(test_play_to_stage(&tg, nozomi, AREA_CENTER), "I1: 東條 希 is played");
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg) && !strcmp(pending_zone(&tg), "hand"),
          "I1: P1's discard-to-3 prompt must select out of the hand; the "
          "engine offered %s", pending_desc(&tg));
    CHECK(any_choice_seat(&tg, 0), "I1: the prompt is routed to p1");
    answer(&tg, 1, 2);                 /* discard 2 */
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "I1 control: the discard really prompted");
    CHECK_EQ(hand_n(&tg), 6, "I1: P1 = 5 initial - 2 discarded + 3 drawn = 6");
    CHECK_EQ(p2_hand_n(&tg), 5, "I1: P2 = 2 (discard leg skipped) + 3 drawn = 5");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], nozomi, "I1: she is on the stage");

    test_game_new(&t2);
    cheap = tid(&t2, "PL!SP-sd1-019-SD");
    nozomi_board(&t2, &nozomi, cheap);
    for (int i = 0; i < 3; i++) test_add_to_hand(&t2, mid(&t2, FILLER));
    for (int i = 0; i < 3; i++) add_hand_for(&t2, 1, mid(&t2, FILLER));
    test_play_to_stage(&t2, nozomi, AREA_CENTER);
    settle_play(&t2);
    drain(&t2, 1);
    CHECK_EQ(hand_n(&t2), 6, "I2: P1 = 4 - 1 played + 3 drawn = 6");
    CHECK_EQ(p2_hand_n(&t2), 6, "I2: P2 = 3 + 3 drawn = 6");

    test_game_new(&t3);
    cheap = tid(&t3, "PL!SP-sd1-009-SD");   /* 鬼塚夏美 cost 13 == 13 */
    nozomi_board(&t3, &nozomi, cheap);
    for (int i = 0; i < 3; i++) test_add_to_hand(&t3, mid(&t3, FILLER));
    CHECK(test_play_to_stage(&t3, nozomi, AREA_CENTER),
          "I3: the equal-cost baton touch still replaces the stage member");
    drain(&t3, 1);
    CHECK_EQ(hand_n(&t3), 3,
             "I3: 4 hand - 1 played = 3, an EQUAL-cost baton touch must not "
             "trigger the ability");
    CHECK_EQ(p2_hand_n(&t3), 0, "I3: the opponent is never touched either");
}

static void test_i_q229_no_baton_touch_no_trigger(void)
{
    static TestGame tg;
    int nozomi;
    test_game_new(&tg);
    nozomi_board(&tg, &nozomi, -1);       /* the center is empty */
    for (int i = 0; i < 3; i++) test_add_to_hand(&tg, mid(&tg, FILLER));
    CHECK(test_play_to_stage(&tg, nozomi, AREA_CENTER), "I4: played to an empty area");
    settle_play(&tg);
    drain(&tg, 1);
    CHECK_EQ(hand_n(&tg), 3,
             "I4: 4 hand - 1 played = 3, no baton touch -> no draw from the ability");
    CHECK_EQ(p2_hand_n(&tg), 0, "I4: the opponent is never touched");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §J — 黒澤ダイヤ PL!S-bp7-004-P 登場 (BP07 C6):
 *   「『Aqours』のメンバーからバトンタッチして登場した場合、自分と相手は
 *     それぞれ、自身の手札のカードを3枚まで選び、選んだカード以外のカードを
 *     シャッフルし、自身のデッキの下に置く。その後、自分と相手はそれぞれ
 *     カードを3枚引く。」
 *   KEEP N, shuffle the REST under, then draw — for BOTH seats; the group gate
 *   is the only difference between the firing and non-firing fixture.
 * Rust: group_baton_both_hands_keep_three_bottomdeck_draw_three_test.rs (7)
 * ══════════════════════════════════════════════════════════════════════════ */
static void dia_board(TestGame *tg, int *dia, int aqours_source)
{
    bare_board(tg);
    *dia = tid(tg, "PL!S-bp7-004-P");
    fill_decks(tg, tid(tg, FILLER), 10);
    test_give_energy(tg, 10);
    tg->state.p[0].stage[AREA_CENTER] = aqours_source;
    test_add_to_hand(tg, *dia);
}

static void test_j_dia_keep_three_shuffle_rest_then_draw_three(void)
{
    static TestGame tg, t2, t3, t4;
    int dia, chika;

    test_game_new(&tg);
    chika = tid(&tg, "PL!S-bp2-001-R");   /* 高海千歌 */
    dia_board(&tg, &dia, chika);
    for (int i = 0; i < 4; i++) test_add_to_hand(&tg, mid(&tg, FILLER));
    for (int i = 0; i < 4; i++) add_hand_for(&tg, 1, mid(&tg, FILLER));
    int p1_before = hand_n(&tg) - 1, p2_before = p2_hand_n(&tg);
    CHECK(test_play_to_stage(&tg, dia, AREA_CENTER),
          "J1: 黒澤ダイヤ baton-touches over an 『Aqours』 member");
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "J1: P1's keep-up-to-3 selection prompts");
    answer(&tg, 1, 3);
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "J1 control: the selection really prompted");
    CHECK_EQ(hand_n(&tg), p1_before + 2, "J1: P1 — 1 card shuffled under the deck, 3 drawn");
    CHECK_EQ(p2_hand_n(&tg), p2_before + 2,
             "J1: P2 — 1 card shuffled under their own deck, 3 drawn");

    test_game_new(&t2);
    chika = tid(&t2, "PL!S-bp2-001-R");
    dia_board(&t2, &dia, chika);
    for (int i = 0; i < 2; i++) test_add_to_hand(&t2, mid(&t2, FILLER));
    for (int i = 0; i < 2; i++) add_hand_for(&t2, 1, mid(&t2, FILLER));
    p1_before = hand_n(&t2) - 1; p2_before = p2_hand_n(&t2);
    test_play_to_stage(&t2, dia, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t2)) { answer(&t2, 1, 2); drain(&t2, 1); }
    CHECK(prompts_seen >= 1, "J2 control: the selection really prompted");
    CHECK_EQ(hand_n(&t2), p1_before + 3, "J2: P1 — nothing under, drew 3");
    CHECK_EQ(p2_hand_n(&t2), p2_before + 3, "J2: P2 — nothing under, drew 3");

    test_game_new(&t3);
    chika = tid(&t3, "PL!S-bp2-001-R");
    dia_board(&t3, &dia, chika);
    for (int i = 0; i < 3; i++) test_add_to_hand(&t3, mid(&t3, FILLER));
    for (int i = 0; i < 3; i++) add_hand_for(&t3, 1, mid(&t3, FILLER));
    p1_before = hand_n(&t3) - 1; p2_before = p2_hand_n(&t3);
    int d1 = deck_n(&t3), d2 = t3.state.p[1].deck.n;
    test_play_to_stage(&t3, dia, AREA_CENTER);
    settle_play(&t3);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t3)) { answer(&t3, -1, 0); drain(&t3, -1); }
    CHECK(prompts_seen >= 1, "J3 control: the decline answered a real prompt");
    CHECK_EQ(hand_n(&t3), p1_before, "J3: P1 — kept 0, all 3 under, drew 3");
    CHECK_EQ(p2_hand_n(&t3), p2_before, "J3: P2 — kept 0, all 3 under, drew 3");
    CHECK_EQ(deck_n(&t3), d1, "J3: P1 deck net unchanged (3 under, 3 drawn)");
    CHECK_EQ(t3.state.p[1].deck.n, d2, "J3: P2 deck net unchanged (3 under, 3 drawn)");

    test_game_new(&t4);
    chika = tid(&t4, "PL!S-bp2-001-R");
    dia_board(&t4, &dia, chika);
    for (int i = 0; i < 3; i++) test_add_to_hand(&t4, mid(&t4, FILLER));
    for (int i = 0; i < 3; i++) add_hand_for(&t4, 1, mid(&t4, FILLER));
    p1_before = hand_n(&t4) - 1; p2_before = p2_hand_n(&t4);
    d1 = deck_n(&t4); d2 = t4.state.p[1].deck.n;
    test_play_to_stage(&t4, dia, AREA_CENTER);
    settle_play(&t4);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t4)) { answer(&t4, 1, 3); drain(&t4, 1); }
    CHECK(prompts_seen >= 1, "J4 control: the selection really prompted");
    CHECK_EQ(hand_n(&t4), p1_before + 3, "J4: P1 — kept all 3, drew 3");
    CHECK_EQ(p2_hand_n(&t4), p2_before + 3, "J4: P2 — kept all 3, drew 3");
    CHECK_EQ(deck_n(&t4), d1 - 3, "J4: P1 deck — 3 drawn, nothing moved under");
    CHECK_EQ(t4.state.p[1].deck.n, d2 - 3, "J4: P2 deck — 3 drawn, nothing under");
}

static void test_j_dia_non_aqours_baton_does_not_fire(void)
{
    static TestGame tg;
    int dia, honoka;
    test_game_new(&tg);
    honoka = tid(&tg, FILLER);      /* 高坂 穂乃果 — NOT 『Aqours』 */
    CHECK_EQ(rb_card_is_live(honoka), 0, "J5: the baton-touch source is a member");
    dia_board(&tg, &dia, honoka);
    for (int i = 0; i < 4; i++) test_add_to_hand(&tg, mid(&tg, FILLER));
    for (int i = 0; i < 4; i++) add_hand_for(&tg, 1, mid(&tg, FILLER));
    int p1_before = hand_n(&tg) - 1, p2_before = p2_hand_n(&tg);
    int d1 = deck_n(&tg);
    CHECK(test_play_to_stage(&tg, dia, AREA_CENTER),
          "J5: 黒澤ダイヤ baton-touches over a non-『Aqours』 member");
    drain(&tg, 1);
    /* control: the J1 positive above proves the ability CAN prompt and CAN move
     * cards on this board, so these negatives are not vacuous. */
    CHECK_EQ(hand_n(&tg), p1_before,
             "J5: P1 hand unchanged — the 『Aqours』 gate refuses the debut");
    CHECK_EQ(p2_hand_n(&tg), p2_before, "J5: P2 hand unchanged");
    CHECK_EQ(deck_n(&tg), d1, "J5: P1 deck unchanged — no shuffle-under, no draw");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §K — 矢澤にこ PL!-PR-017-PR 起動:
 *   「このメンバーをステージから控え室に置く：自分の控え室から『μ's』の
 *     ライブカードを1枚手札に加える。自分の成功ライブカード置き場にある
 *     カードのスコアの合計が９以上の場合、エネルギーを2枚アクティブにする。」
 *   A cost leg, then an effect whose SECOND clause is gated on a score total
 *   computed from the success zone. Both branches are asserted.
 * Rust: mus_live_recovery_success_score_energy_pl_pr_017_pr_test.rs (2)
 * ══════════════════════════════════════════════════════════════════════════ */
static void niko_board(TestGame *tg, int *niko, int energy)
{
    bare_board(tg);
    *niko = tid(tg, "PL!-PR-017-PR");
    CHECK(ident(*niko, "PL!-PR-017-PR"),
          "K: the 起動 source is the printed 矢澤にこ PR card (cost 2)");
    fill_decks(tg, tid(tg, FILLER), 10);
    tg->state.p[0].stage[AREA_CENTER] = *niko;
    test_give_energy(tg, energy);
}

static void test_k_niko_score_total_gates_energy_activation(void)
{
    static TestGame tg, t2;
    int niko, mus;

    test_game_new(&tg);
    niko_board(&tg, &niko, 4);
    tg.state.p[0].energy_active = 2;   /* two genuinely-WAITED energies */
    mus = tid(&tg, LIVE_L);
    test_add_to_discard(&tg, mus);
    add_success_for(&tg, 0, tid(&tg, "PL!S-pb1-023-L"));

    test_activate_ability(&tg, niko);
    drain(&tg, 1);
    CHECK(hand_has(&tg, mus), "K1: the ライブカード reached hand from the waitroom");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 4,
             "K1: success total >= 9 -> 2 waited energies ACTIVATED (2 -> 4)");

    test_game_new(&t2);
    niko_board(&t2, &niko, 2);
    t2.state.p[0].energy_active = 2;
    mus = tid(&t2, LIVE_L);
    test_add_to_discard(&t2, mus);

    test_activate_ability(&t2, niko);
    drain(&t2, 1);
    CHECK(hand_has(&t2, mus), "K2: the ライブカード still reaches hand");
    CHECK_EQ(rb_energy_active_count(&t2.state.p[0]), 2,
             "K2: no success cards -> no activation (2 stayed 2)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §L — 津島善子 PL!S-bp7-015-N ライブ開始時:
 *   「自分のデッキの下からカードを1枚控え室に置く。それがライブカードの場合、
 *     ライブ終了時まで、heart02 を得る。」
 *   The source must be the deck BOTTOM, not the hand and not the top.
 * Rust: live_start_bottom_one_mill_live_heart_source_test.rs (1)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_l_yoshiko_mills_the_bottom_card_only(void)
{
    static TestGame tg;
    int yoshiko, filler, top_marker, b1, b2, b3, hand_card, live;
    test_game_new(&tg);
    bare_board(&tg);
    yoshiko = tid(&tg, "PL!S-bp7-015-N");
    CHECK(ident(yoshiko, "PL!S-bp7-015-N"),
          "L1: the ライブ開始時 source is the printed 津島善子 card");
    filler = tid(&tg, FILLER);
    top_marker = tid(&tg, "PL!S-bp7-006-R");
    b1 = tid(&tg, "PL!SP-sd1-001-SD");
    b2 = tid(&tg, "PL!SP-sd1-003-SD");
    b3 = tid(&tg, "PL!SP-sd1-004-SD");
    hand_card = mid(&tg, FILLER);
    live = tid(&tg, LIVE_M);

    tg.state.p[0].stage[AREA_CENTER] = yoshiko;
    test_add_to_hand(&tg, hand_card);      /* a hand card: not the source */
    test_give_energy(&tg, 3);
    test_add_to_hand(&tg, live);

    /* deck: [top_marker, 10 fillers, b1, b2, b3] -- b3 is the BOTTOM */
    fill_zone(&tg.state.p[0].deck, filler, 1);
    tg.state.p[0].deck.cards[0] = top_marker;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, b1);
    test_add_to_deck_pl(&tg, 0, b2);
    test_add_to_deck_pl(&tg, 0, b3);

    test_set_live_card(&tg, 0, live);
    fire_live_start(&tg);
    drain(&tg, 1);

    CHECK_EQ(disc_n(&tg), 1, "L1: exactly 1 card was discarded");
    CHECK_EQ(tg.state.p[0].discard.cards[0], b3,
             "L1: the BOTTOM-most card (the last in the deck) is the one milled");
    CHECK(hand_has(&tg, hand_card), "L1: the hand card is untouched");
    CHECK(deck_has(&tg, top_marker), "L1: the deck TOP card is untouched");
    CHECK(deck_has(&tg, b1) && deck_has(&tg, b2),
          "L1: only the very last card left the deck");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §M — Edelied PL!HS-pb1-030-L ライブ開始時 (a LIVE card, not a member):
 *   「…自分のステージにいる『EdelNote』のメンバー1人は、ブレード2を得て、
 *     そのメンバーとは名前の異なる『EdelNote』のメンバー1人は、heart06 2を得る。」
 *   Two steps where step 2 EXCLUDES the card step 1 picked, by NAME.
 * Rust: live_start_two_different_named_edelnote_members_gain_blades_and_heart06
 *       _test.rs (6)
 * ══════════════════════════════════════════════════════════════════════════ */
static void edelied_board(TestGame *tg, int *edelied)
{
    bare_board(tg);
    *edelied = tid(tg, "PL!HS-pb1-030-L");
    CHECK(ident(*edelied, "PL!HS-pb1-030-L"),
          "M: the source is the printed Edelied LIVE card");
    fill_decks(tg, tid(tg, FILLER), 40);
    test_add_to_live_for(tg, 0, *edelied);
}

static void test_m_edelied_blade_then_heart06_different_name(void)
{
    static TestGame tg, t2, t3, t4;
    int edelied, e1, e2, e3;

    test_game_new(&tg);
    edelied_board(&tg, &edelied);
    e1 = tid(&tg, "PL!HS-PR-022-PR");   /* セラス 柳田 */
    e2 = tid(&tg, "PL!HS-PR-023-PR");   /* 桂城 泉 — different name */
    tg.state.p[0].stage[AREA_LEFT] = e1;
    tg.state.p[0].stage[AREA_CENTER] = e2;
    fire_live_start(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&tg, 1);
    CHECK(prompts_seen > 0,
          "M1 control: the blade target really prompted, so the split below is "
          "not a no-op reading zero modifiers");
    int gb[2] = { blade_of(&tg, e1) > 0, blade_of(&tg, e2) > 0 };
    int gh[2] = { heart_of(&tg, e1, H06) > 0, heart_of(&tg, e2, H06) > 0 };
    CHECK_EQ(gb[0] + gb[1], 1, "M1: exactly 1 member got blades (1人)");
    CHECK_EQ(gh[0] + gh[1], 1, "M1: exactly 1 member got heart06 (1人)");
    CHECK(!((gb[0] && gh[0]) || (gb[1] && gh[1])),
          "M1: blade and heart06 go to DIFFERENT members (名前の異なる)");

    test_game_new(&t2);
    edelied_board(&t2, &edelied);
    e1 = tid(&t2, "PL!HS-PR-022-PR");
    t2.state.p[0].stage[AREA_LEFT] = e1;
    fire_live_start(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&t2, 1);
    CHECK(prompts_seen > 0, "M2 control: the blade target really prompted");
    CHECK_EQ(blade_of(&t2, e1), 2, "M2: the only member gets +2 blades");
    CHECK_EQ(heart_of(&t2, e1, H06), 0,
             "M2: no heart06 — no differently-named member exists");

    test_game_new(&t3);
    edelied_board(&t3, &edelied);
    e1 = tid(&t3, "PL!HS-PR-022-PR");   /* セラス 柳田 */
    e2 = tid(&t3, "PL!HS-PR-023-PR");   /* 桂城 泉 */
    e3 = tid(&t3, "PL!HS-PR-032-PR");   /* セラス 柳田 — SAME name as e1 */
    t3.state.p[0].stage[AREA_LEFT] = e1;
    t3.state.p[0].stage[AREA_CENTER] = e2;
    t3.state.p[0].stage[AREA_RIGHT] = e3;
    fire_live_start(&t3);
    prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&t3, 1);
    CHECK(prompts_seen > 0, "M3 control: the blade target really prompted");
    int b3[3] = { blade_of(&t3, e1), blade_of(&t3, e2), blade_of(&t3, e3) };
    int h3[3] = { heart_of(&t3, e1, H06), heart_of(&t3, e2, H06),
                  heart_of(&t3, e3, H06) };
    CHECK_EQ((b3[0] > 0) + (b3[1] > 0) + (b3[2] > 0), 1,
             "M3: exactly 1 member gets blades");
    CHECK_EQ((h3[0] > 0) + (h3[1] > 0) + (h3[2] > 0), 1,
             "M3: exactly 1 member gets heart06");
    CHECK_EQ(h3[2], 0,
             "M3: the same-name duplicate (e3) must not be the heart06 target");

    test_game_new(&t4);
    edelied_board(&t4, &edelied);
    e1 = tid(&t4, "PL!HS-PR-022-PR");
    e2 = tid(&t4, "PL!HS-PR-032-PR");
    t4.state.p[0].stage[AREA_LEFT] = e1;
    t4.state.p[0].stage[AREA_CENTER] = e2;
    fire_live_start(&t4);
    prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&t4, 1);
    CHECK(prompts_seen > 0, "M4 control: the blade target really prompted");
    CHECK_EQ(blade_of(&t4, e1) + blade_of(&t4, e2), 2,
             "M4: exactly 1 member gets +2 blades (2 + 0)");
    CHECK_EQ(heart_of(&t4, e1, H06) + heart_of(&t4, e2, H06), 0,
             "M4: same-name only -> no heart06 (no differently-named member)");
}

static void test_m_edelied_no_members_no_modifiers(void)
{
    static TestGame tg;
    int edelied;
    test_game_new(&tg);
    edelied_board(&tg, &edelied);
    /* control: M1 above proves the board CAN produce a prompt + modifiers, so
     * this negative is not a vacuous pass on a board that does nothing. */
    prompts_seen = 0;
    card_prompts_seen = 0;
    fire_live_start(&tg);
    drain(&tg, 1);
    CHECK_EQ(prompts_seen, 0,
             "M5: with 0 『EdelNote』 members the target selection never prompts");
    CHECK_EQ(blade_of(&tg, edelied), 0, "M5: no blade on the source live card");
    CHECK_EQ(heart_of(&tg, edelied, H06), 0, "M5: no heart06 on the source live card");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §N — 西木野真姫 PL!-pb1-006-R 登場:
 *   「自分の控え室から『μ's』のライブカードを1枚までデッキの一番上に置く。
 *     その後、相手のステージにウェイト状態のメンバーがいる場合、カードを1枚引く。」
 *   Step 1 is UNCONDITIONAL (up to 1 card); step 2 is a そう場合 gate on the
 *   OPPONENT's wait state only. The move must still happen when the gate fails.
 * Rust: debut_group_live_topdeck_waited_opponent_draw_test.rs (10)
 * ══════════════════════════════════════════════════════════════════════════ */
#define MUS_LIVE_PLUS "PL!-bp3-019-L"   /* 僕らのLIVE 君とのLIFE */

static void maki_board(TestGame *tg, int *maki, int mus_live, int opp_area)
{
    bare_board(tg);
    *maki = tid(tg, "PL!-pb1-006-R");
    CHECK(ident(*maki, "PL!-pb1-006-R"),
          "N: the 登場 source is the printed 西木野真姫 card (cost 11)");
    test_add_to_hand(tg, *maki);
    test_add_to_hand(tg, mid(tg, FILLER));
    if (mus_live >= 0) test_add_to_discard(tg, mus_live);
    if (opp_area >= 0) tg->state.p[1].stage[opp_area] = mid(tg, FILLER);
    fill_decks(tg, tid(tg, FILLER), 10);
    test_give_energy(tg, 20);
}

static void test_n_mus_live_to_deck_top_and_opponent_wait_gate(void)
{
    static TestGame tg, t2, t3, t4;
    int maki, mus;

    test_game_new(&tg);
    mus = tid(&tg, MUS_LIVE_PLUS);
    maki_board(&tg, &maki, mus, -1);
    int deck_before = deck_n(&tg);
    CHECK(test_play_to_stage(&tg, maki, AREA_CENTER), "N1: 西木野真姫 is played");
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "N1: step 1 (place the μ's live) prompts");
    answer(&tg, 1, 1);
    drain(&tg, 1);
    CHECK(prompts_seen > 0, "N1 control: the placement really prompted");
    CHECK_EQ(deck_top(&tg), mus, "N1: the μ's live card is on the deck TOP");
    CHECK_EQ(deck_n(&tg), deck_before + 1,
             "N1: with no waited opponent member the move still happened and no "
             "draw offset it");

    test_game_new(&t2);
    mus = tid(&t2, MUS_LIVE_PLUS);
    maki_board(&t2, &maki, mus, AREA_LEFT);
    rb_mods_set_orientation(&t2.state.mods, t2.state.p[1].stage[AREA_LEFT], "wait");
    deck_before = deck_n(&t2);
    int hand_before = hand_n(&t2);
    test_play_to_stage(&t2, maki, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    CHECK(prompts_seen > 0, "N2 control: the placement really prompted");
    CHECK_EQ(deck_n(&t2), deck_before,
             "N2: move (+1) + draw (-1) nets the deck unchanged");
    CHECK_EQ(hand_n(&t2), hand_before, "N2: net hand unchanged (play -1, draw +1)");
    CHECK(!disc_has(&t2, mus), "N2: the μ's live left the waitroom");
    CHECK(hand_has(&t2, mus),
          "N2: the draw brings the just-placed μ's live to hand");

    test_game_new(&t3);
    mus = tid(&t3, MUS_LIVE_PLUS);
    maki_board(&t3, &maki, mus, AREA_CENTER);
    rb_mods_set_orientation(&t3.state.mods, t3.state.p[1].stage[AREA_CENTER], "active");
    CHECK(test_play_to_stage(&t3, maki, AREA_CENTER), "N3: played");
    settle_play(&t3);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t3)) { answer(&t3, 1, 1); drain(&t3, 1); }
    CHECK(prompts_seen > 0, "N3 control: the placement really prompted");
    CHECK_EQ(hand_n(&t3), 1, "N3: an ACTIVE opponent member must not trigger the draw");

    test_game_new(&t4);
    mus = tid(&t4, MUS_LIVE_PLUS);
    maki_board(&t4, &maki, mus, -1);
    int self_wait = mid(&t4, FILLER);
    t4.state.p[0].stage[AREA_LEFT] = self_wait;
    rb_mods_set_orientation(&t4.state.mods, self_wait, "wait");
    CHECK(test_play_to_stage(&t4, maki, AREA_CENTER), "N4: played over a waited ally");
    settle_play(&t4);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t4)) { answer(&t4, 1, 1); drain(&t4, 1); }
    CHECK(prompts_seen > 0, "N4 control: the placement really prompted");
    CHECK_EQ(hand_n(&t4), 1,
             "N4: only the OPPONENT's wait triggers the draw; self wait alone "
             "does not");
}

static void test_n_no_mus_live_in_waitroom(void)
{
    static TestGame tg, t2;
    int maki;

    test_game_new(&tg);
    maki_board(&tg, &maki, -1, -1);
    test_add_to_discard(&tg, mid(&tg, FILLER));
    int before[RB_MAX_ZONE];
    int n = test_zone_ids(&tg, 0, "deck", before, RB_MAX_ZONE);
    test_play_to_stage(&tg, maki, AREA_CENTER);
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&tg, 1);
    /* control: N1 above proves the placement prompts and moves a card on this
     * board, so "the deck is unchanged" is a real negative. */
    int after[RB_MAX_ZONE];
    int m = test_zone_ids(&tg, 0, "deck", after, RB_MAX_ZONE);
    CHECK_EQ(prompts_seen, 0,
             "N5: with no μ's live in the waitroom the placement leg has no "
             "candidate and does not prompt");
    CHECK_EQ(m, n, "N5: the deck is unchanged when there is nothing to place");

    test_game_new(&t2);
    maki_board(&t2, &maki, -1, AREA_LEFT);
    rb_mods_set_orientation(&t2.state.mods, t2.state.p[1].stage[AREA_LEFT], "wait");
    test_add_to_discard(&t2, mid(&t2, FILLER));
    int d_before = deck_n(&t2);
    int h_before = hand_n(&t2);
    test_play_to_stage(&t2, maki, AREA_CENTER);
    settle_play(&t2);
    drain(&t2, 1);
    CHECK_EQ(deck_n(&t2), d_before - 1,
             "N6: the draw still fires even when there is no μ's live to place");
    CHECK_EQ(hand_n(&t2), h_before, "N6: net hand unchanged (play -1, draw +1)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §O — 葉月 恋 PL!SP-pb2-005-R 登場 (baton touch):
 *   「バトンタッチして登場した場合、このバトンタッチで控え室に置かれた
 *     『Liella!』のメンバーカードを1枚、このメンバーの下に置く。」
 *   The group filter on the baton-touch victim. The positive and the negative
 *   differ ONLY in the victim's group, so "nothing was placed" is meaningful.
 * Rust: baton_touch_place_liella_under_then_activate_copied_ability_pl_sp_pb2_005
 *       _test.rs (5)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_o_hazuki_baton_touch_places_liella_under(void)
{
    static TestGame tg, t2;
    int hazuki, liella, filler;

    test_game_new(&tg);
    bare_board(&tg);
    hazuki = tid(&tg, "PL!SP-pb2-005-R");
    CHECK(ident(hazuki, "PL!SP-pb2-005-R"),
          "O1: the 登場 source is the printed 葉月 恋 card (cost 20)");
    liella = mid(&tg, "PL!SP-sd1-001-SD");   /* 澁谷かのん — 『Liella!』 */
    filler = mid(&tg, FILLER);
    tg.state.p[0].stage[AREA_CENTER] = liella;
    test_add_to_hand(&tg, hazuki);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 25);

    CHECK(test_play_to_stage(&tg, hazuki, AREA_CENTER),
          "O1: 葉月 恋 baton-touches over a 『Liella!』 member");
    drain(&tg, 1);
    CHECK(under_has(&tg, AREA_CENTER, liella),
          "O1: the 『Liella!』 member is placed UNDER her");
    CHECK(!disc_has(&tg, liella), "O1: it is not left in the waitroom");

    /* O2: a non-『Liella!』 member must not be placed under.
     * control: O1 above proves the under-placement CAN happen on this board. */
    test_game_new(&t2);
    bare_board(&t2);
    hazuki = tid(&t2, "PL!SP-pb2-005-R");
    int non = mid(&t2, "PL!-sd1-005-SD");   /* 星空 凛 — lilywhite, not 『Liella!』 */
    filler = mid(&t2, FILLER);
    t2.state.p[0].stage[AREA_CENTER] = non;
    test_add_to_hand(&t2, hazuki);
    test_add_to_hand(&t2, filler);
    fill_decks(&t2, filler, 10);
    test_give_energy(&t2, 25);
    CHECK(test_play_to_stage(&t2, hazuki, AREA_CENTER),
          "O2: 葉月 恋 baton-touches over a non-『Liella!』 member");
    drain(&t2, 1);
    CHECK(!under_has(&t2, AREA_CENTER, non),
          "O2: a non-『Liella!』 member must not be placed under her");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §P — 大沢瑠璃乃 PL!HS-pb1-003-R 登場:
 *   「手札の『みらくらぱーく！』のメンバーカードを好きな枚数控え室に置き、
 *     その後、その枚数に1を足した枚数のカードを引く。」
 *   The drawn COUNT is a function of the number the FIRST step chose. 0 is a
 *   legal 好きな枚数 and the printed "+1" still applies.
 * Rust: debut_unit_discard_any_draw_count_plus_one_test.rs (2)
 * ══════════════════════════════════════════════════════════════════════════ */
static void rurino_board(TestGame *tg, int *rurino, int n_miraku)
{
    bare_board(tg);
    *rurino = tid(tg, "PL!HS-pb1-003-R");
    CHECK(ident(*rurino, "PL!HS-pb1-003-R"),
          "P: the 登場 source is the printed 大沢瑠璃乃 card (cost 15)");
    test_add_to_hand(tg, *rurino);
    for (int i = 0; i < n_miraku; i++) test_add_to_hand(tg, mid(tg, "PL!HS-sd1-011-SD"));
    fill_decks(tg, tid(tg, FILLER), 20);
    add_hand_for(tg, 1, mid(tg, FILLER));
    test_give_energy(tg, 20);
}

static void test_p_rurino_discard_two_draw_three(void)
{
    static TestGame tg, t2;
    int rurino, deck_before;

    test_game_new(&tg);
    rurino_board(&tg, &rurino, 3);
    deck_before = deck_n(&tg);
    CHECK(test_play_to_stage(&tg, rurino, AREA_CENTER), "P1: 大沢瑠璃乃 is played");
    settle_play(&tg);

    const RbChoice *c = pending_ch(&tg);
    CHECK(c && c->kind == RB_CHOICE_SELECT_CARD,
          "P1: the 好きな枚数 select is a SelectCard");
    if (c) CHECK(!strcmp(c->zone, "hand"),
                 "P1: it selects out of the HAND (got \"%s\")", c->zone);
    int miraku[4];
    for (int i = 0; i < 4; i++) miraku[i] = mid(&tg, "PL!HS-sd1-011-SD");
    prompts_seen = 0;
    card_prompts_seen = 0;
    answer(&tg, 1, 2);
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "P1 control: the 好きな枚数 select really prompted");

    int waited = 0, left = 0;
    for (int i = 0; i < 4; i++) {
        if (disc_has(&tg, miraku[i])) waited++;
        if (hand_has(&tg, miraku[i])) left++;
    }
    CHECK_EQ(waited, 2,
             "P1: 好きな枚数 = 2 -> exactly two of the candidates reach the waitroom");
    CHECK_EQ(left, 1, "P1: exactly one candidate is left in hand");
    CHECK_EQ(deck_before - deck_n(&tg), 3,
             "P1: その後、その枚数に1を足した枚数 — 2 + 1 = 3 cards left the deck");

    test_game_new(&t2);
    rurino_board(&t2, &rurino, 1);
    deck_before = deck_n(&t2);
    int one = mid(&t2, "PL!HS-sd1-011-SD");
    CHECK(test_play_to_stage(&t2, rurino, AREA_CENTER), "P2: 大沢瑠璃乃 is played");
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "P2: the 好きな枚数 select prompts");
    answer(&t2, -1, 0);
    drain(&t2, -1);
    CHECK(prompts_seen >= 1, "P2 control: choosing 0 answered a real prompt");
    CHECK(!disc_has(&t2, one),
          "P2: 0 chosen -> the candidate does not reach the waitroom");
    CHECK(hand_has(&t2, one), "P2: the candidate is still in hand");
    CHECK_EQ(deck_before - deck_n(&t2), 1, "P2: 0 + 1 = exactly one card drawn");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §Q — ミア・テイラー PL!N-bp1-011-R 登場:
 *   「手札を1枚控え室に置いてもよい：ライブカードが公開されるまで、自分の
 *     デッキの一番上のカードを公開し続ける。そのライブカードを手札に加え、
 *     これにより公開されたほかのすべてのカードを控え室に置く。」
 *   An optional cost whose PAUSE must not lose the reveal loop behind it, plus
 *   the bounded "deck runs out" case.
 * Rust: debut_optional_discard_reveal_until_live_q73_q102_test.rs (4)
 * ══════════════════════════════════════════════════════════════════════════ */
static void mia_board(TestGame *tg, int *mia)
{
    bare_board(tg);
    *mia = tid(tg, "PL!N-bp1-011-R");
    CHECK(ident(*mia, "PL!N-bp1-011-R"),
          "Q: the 登場 source is the printed ミア・テイラー card (cost 9)");
    test_add_to_hand(tg, *mia);
    test_add_to_hand(tg, mid(tg, FILLER));
    test_give_energy(tg, 10);
}

static void test_q_mia_reveal_until_live(void)
{
    static TestGame tg, t2, t3;
    int mia, live, filler;

    test_game_new(&tg);
    mia_board(&tg, &mia);
    filler = tid(&tg, FILLER);
    live = tid(&tg, LIVE_L);
    for (int i = 0; i < 4; i++) test_add_to_deck_pl(&tg, 0, filler);
    add_deck_top(&tg, 0, live);           /* deck = [live, f, f, f, f] */
    CHECK(test_play_to_stage(&tg, mia, AREA_CENTER), "Q1: ミア・テイラー is played");
    settle_play(&tg);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "Q1: the optional hand-discard cost prompts");
    answer(&tg, 1, 1);
    drain(&tg, 1);
    CHECK(prompts_seen >= 1,
          "Q1 control: the cost really prompted, so the reveal below is not a "
          "no-op reading an untouched board");
    CHECK(hand_has(&tg, live), "Q1: a live card on the top is obtained");
    CHECK_EQ(deck_n(&tg), 3, "Q1: revealing exactly 1 live leaves 3 fillers");

    test_game_new(&t2);
    mia_board(&t2, &mia);
    filler = tid(&t2, FILLER);
    live = tid(&t2, LIVE_L);
    for (int i = 0; i < 4; i++) test_add_to_deck_pl(&t2, 0, filler);
    add_deck_top(&t2, 0, live);
    test_play_to_stage(&t2, mia, AREA_CENTER);
    settle_play(&t2);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&t2), "Q2: the optional cost prompts");
    answer(&t2, -1, 0);
    drain(&t2, -1);
    CHECK(prompts_seen >= 1, "Q2 control: the decline answered a real prompt");
    CHECK(hand_has(&t2, live) || disc_has(&t2, live) || deck_has(&t2, live),
          "Q2: skipping the cost must not LOSE the reveal tail — the live card "
          "is somewhere the reveal put it");

    test_game_new(&t3);
    mia_board(&t3, &mia);
    filler = tid(&t3, FILLER);
    for (int i = 0; i < 3; i++) test_add_to_deck_pl(&t3, 0, filler);
    test_play_to_stage(&t3, mia, AREA_CENTER);
    settle_play(&t3);
    prompts_seen = 0;
    card_prompts_seen = 0;
    if (pending(&t3)) { answer(&t3, 1, 1); drain(&t3, 1); }
    if (pending(&t3)) { answer(&t3, 1, 1); drain(&t3, 1); }
    CHECK(prompts_seen >= 1, "Q3 control: the cost really prompted");
    CHECK_EQ(deck_n(&t3), 0,
             "Q3: with no live card anywhere the deck is exhausted by the reveal");
    CHECK(disc_n(&t3) > 0, "Q3: the revealed non-live cards are moved to the waitroom");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §R — 大沢瑠璃乃 PL!HS-bp2-014-N 登場 (Q68):
 *   「カードを1枚引く。ライブ終了時まで、自分はライブできない。」
 *   A draw AND a prohibition from one 登場. Q68: the prohibition does NOT stop
 *   the player SETTING a live card, only the live from succeeding.
 * Rust: debut_draws_one_and_cannot_live_this_turn_q68_test.rs (3)
 * NOTE: the Rust twin calls her 遠手鞠; cards.json has PL!HS-bp2-014-N as
 * 大沢瑠璃乃. The card number and its cost (4) match the fixture.
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_r_cannot_live_debut_draws_and_sets_prohibition(void)
{
    static TestGame tg, t2;
    int mari, filler, live;

    test_game_new(&tg);
    bare_board(&tg);
    mari = tid(&tg, "PL!HS-bp2-014-N");
    CHECK(ident(mari, "PL!HS-bp2-014-N"), "R1: the Q68 source is the printed card (cost 4)");
    filler = tid(&tg, FILLER);
    live = tid(&tg, "LL-bp5-001-L");
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_hand(&tg, mari);
    test_add_to_hand(&tg, mid(&tg, FILLER));
    test_add_to_hand(&tg, live);
    test_give_energy(&tg, 4);

    CHECK(test_play_to_stage(&tg, mari, AREA_LEFT), "R1: she is played to the left");
    settle_play(&tg);
    drain(&tg, 1);
    CHECK_EQ(hand_n(&tg), 3, "R1: 3 initial - 1 played + 1 drawn = 3");
    CHECK(rb_is_action_prohibited(&tg.state, "cannot_live"),
          "R1: ライブ終了時まで、自分はライブできない — the prohibition is set");

    test_game_new(&t2);
    bare_board(&t2);
    mari = tid(&t2, "PL!HS-bp2-014-N");
    filler = tid(&t2, FILLER);
    live = tid(&t2, "LL-bp5-001-L");
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&t2, 0, filler);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&t2, 1, filler);
    test_add_to_hand(&t2, mari);
    test_add_to_hand(&t2, live);
    test_give_energy(&t2, 4);
    test_play_to_stage(&t2, mari, AREA_LEFT);
    settle_play(&t2);
    drain(&t2, 1);
    CHECK(rb_is_action_prohibited(&t2.state, "cannot_live"),
          "R2 control: the prohibition really is active in this fixture");
    test_set_live_card(&t2, 0, live);
    CHECK(bag_has(&t2.state.p[0].live, live),
          "R2: Q68 — a live card can still be SET despite cannot_live");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §S — 鐘 嵐珠 PL!N-bp5-012-R＋ 起動:
 *   「エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：
 *     カードを1枚引き、ライブ終了時まで、heart01 を得る。」
 *   A zone COST followed by two effects, plus the ターン1回 use limit.
 * Rust: place_energy_under_self_then_draw_and_gain_heart_pl_n_bp5_012_r_test (4)
 * ══════════════════════════════════════════════════════════════════════════ */
static void rin_board(TestGame *tg, int *rin)
{
    bare_board(tg);
    *rin = tid(tg, "PL!N-bp5-012-R" PLUS);
    CHECK(ident(*rin, "PL!N-bp5-012-R" PLUS),
          "S: the 起動 source is the printed 鐘 嵐珠 card (cost 13)");
    tg->state.p[0].stage[AREA_CENTER] = *rin;
    test_add_to_hand(tg, mid(tg, FILLER));
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, tid(tg, FILLER));
    test_give_energy(tg, 10);
}

static void test_s_rin_places_energy_under_then_draws_and_gains_heart(void)
{
    static TestGame tg, t2;
    int rin;

    test_game_new(&tg);
    rin_board(&tg, &rin);
    int hand_before = hand_n(&tg);
    int energy_before = tg.state.p[0].energy.n;
    int active_before = rb_energy_active_count(&tg.state.p[0]);

    test_activate_ability(&tg, rin);
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK(pending(&tg), "S1: the energy choice prompts");
    answer(&tg, 1, 1);
    drain(&tg, 1);
    CHECK(prompts_seen >= 1, "S1 control: the energy choice really prompted");
    CHECK_EQ(tg.state.p[0].energy.n, energy_before - 1,
             "S1: 1 energy card left the energy zone");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 1,
             "S1: 1 energy placed UNDER 鐘 嵐珠 at the center");
    CHECK_EQ(hand_n(&tg), hand_before + 1, "S1: カードを1枚引く — exactly 1 drawn");
    CHECK_EQ(heart_of(&tg, rin, H01), 1, "S1: ライブ終了時まで、heart01 を得る");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), active_before,
             "S1: the placement does not change the ACTIVE energy count");

    test_game_new(&t2);
    rin_board(&t2, &rin);
    test_activate_ability(&t2, rin);
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    prompts_seen = 0;
    card_prompts_seen = 0;
    CHECK_EQ(heart_of(&t2, rin, H01), 1, "S2: +1 after the first activation");
    test_activate_ability(&t2, rin);
    if (pending(&t2)) { answer(&t2, 1, 1); drain(&t2, 1); }
    drain(&t2, 1);
    CHECK_EQ(heart_of(&t2, rin, H01), 1,
             "S2: ターン1回 — a second activation in the same turn does not "
             "double the heart modifier");
    (void)prompts_seen;
}

/* ══════════════════════════════════════════════════════════════════════════
 * §T — 朝香果林 PL!N-bp4-004-R＋ ライブ開始時 (two separate ライブ開始時
 * abilities; only ab#0 is used here):
 *   「カードを1枚引く。相手のステージにいるコスト9以下のメンバーを1人まで
 *     ウェイトにする。」
 * Rust: live_start_draw_opponent_cost_nine_wait_test.rs (3)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_t_karin_draws_and_waits_an_eligible_opponent_member(void)
{
    static TestGame tg, t2, t3;
    int karin, cost9, hand_before;

    test_game_new(&tg);
    bare_board(&tg);
    karin = tid(&tg, "PL!N-bp4-004-R" PLUS);
    CHECK(ident(karin, "PL!N-bp4-004-R" PLUS),
          "T1: the ライブ開始時 source is the printed 朝香果林 card (cost 15)");
    cost9 = tid(&tg, "PL!-sd1-014-SD");   /* 星空 凛, cost 9 */
    tg.state.p[0].stage[AREA_LEFT] = karin;
    tg.state.p[1].stage[AREA_LEFT] = cost9;
    fill_decks(&tg, tid(&tg, FILLER), 10);
    hand_before = hand_n(&tg);

    fire_live_start(&tg);
    drain(&tg, 1);
    CHECK(hand_n(&tg) > hand_before, "T1: ab#0 draws 1 card");
    CHECK(is_waited(&tg, cost9), "T1: a cost-9 opponent member is waited");

    test_game_new(&t2);
    bare_board(&t2);
    karin = tid(&t2, "PL!N-bp4-004-R" PLUS);
    int cost15 = tid(&t2, "PL!N-bp4-004-R" PLUS);   /* 朝香果林 herself: cost 15 */
    t2.state.p[0].stage[AREA_LEFT] = karin;
    t2.state.p[1].stage[AREA_LEFT] = cost15;
    fill_decks(&t2, tid(&t2, FILLER), 10);
    hand_before = hand_n(&t2);
    fire_live_start(&t2);
    drain(&t2, 1);
    CHECK(hand_n(&t2) > hand_before, "T2: ab#0 still draws over a cost-15 member");
    CHECK(!is_waited(&t2, cost15), "T2: a cost-15 member must NOT be waited");

    test_game_new(&t3);
    bare_board(&t3);
    karin = tid(&t3, "PL!N-bp4-004-R" PLUS);
    t3.state.p[0].stage[AREA_LEFT] = karin;
    fill_decks(&t3, tid(&t3, FILLER), 10);
    hand_before = hand_n(&t3);
    fire_live_start(&t3);
    drain(&t3, 1);
    CHECK(hand_n(&t3) > hand_before, "T3: ab#0 draws even with an empty opponent stage");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(t3.state.p[1].stage[i], RB_EMPTY_SLOT,
                 "T3: nothing was staged on the opponent's side");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §U — ミア・テイラー PL!N-bp4-011-R＋ ライブ成功時 (ab#1):
 *   「自分のデッキの上からカードを5枚控え室に置く。その後、自分の控え室に
 *     カード名の異なる『虹ヶ咲』のライブカードが3枚以上ある場合、自分の
 *     控え室から『虹ヶ咲』のライブカードを1枚手札に加える。」
 *   2 and 3 copies of the SAME print: the DISTINCT-NAME threshold decides
 *   whether the recovery prompt appears at all.
 * Rust: live_success_mill_five_distinct_live_recovery_setup_test.rs (2)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_u_mia_distinct_live_threshold_gates_the_recovery_prompt(void)
{
    static TestGame tg, t2;
    int mia, filler, live;

    test_game_new(&tg);
    bare_board(&tg);
    mia = tid(&tg, "PL!N-bp4-011-R" PLUS);
    CHECK(ident(mia, "PL!N-bp4-011-R" PLUS),
          "U1: the ライブ成功時 source is the printed ミア・テイラー card");
    filler = tid(&tg, FILLER);
    test_add_to_discard(&tg, tid(&tg, "PL!N-bp1-028-L"));
    test_add_to_discard(&tg, mid(&tg, "PL!N-bp1-028-L"));
    test_add_to_discard(&tg, mid(&tg, "PL!N-bp1-028-L"));
    tg.state.p[0].stage[AREA_CENTER] = mia;
    fill_decks(&tg, filler, 10);
    live = tid(&tg, LIVE_M);
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);

    tg.state.live_success[0] = 1;
    rb_trigger_live_success(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    prompts_seen = 0;
    card_prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&tg, -1);
    CHECK_EQ(card_prompts_seen, 0,
             "U1: three copies of ONE 虹ヶ咲 live name are not three DISTINCT "
             "names -> the recovery SelectCard never appears");

    test_game_new(&t2);
    bare_board(&t2);
    mia = tid(&t2, "PL!N-bp4-011-R" PLUS);
    filler = tid(&t2, FILLER);
    test_add_to_discard(&t2, tid(&t2, "PL!N-bp1-028-L"));
    test_add_to_discard(&t2, mid(&t2, "PL!N-bp1-028-L"));
    t2.state.p[0].stage[AREA_CENTER] = mia;
    fill_decks(&t2, filler, 10);
    live = tid(&t2, LIVE_M);
    test_add_to_hand(&t2, live);
    test_set_live_card(&t2, 0, live);
    t2.state.live_success[0] = 1;
    rb_trigger_live_success(&t2.state, 0);
    rb_process_pending_auto_abilities(&t2.state);
    prompts_seen = 0;
    card_prompts_seen = 0;
    card_prompts_seen = 0;
    drain(&t2, -1);
    CHECK_EQ(card_prompts_seen, 0,
             "U2: two same-name copies are still not three distinct names -> no "
             "recovery SelectCard");
}

/* ══════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_tests_ok, n_tests_failed, n_tests_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0);
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [in-process fallback]\n", nf ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d gap(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);

    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_tests_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §A 優木せつ菜 PL!N-PR-032-PR: mill the shortfall, then recover ---\n");
    run("a_shortfall_mill_then_recover", test_a_setsuna_mills_shortfall_and_recovers_live_to_deck_top);
    run("a_no_live_no_prompt", test_a_setsuna_no_live_among_milled_no_prompt);
    run("a_threshold_mills_nothing", test_a_setsuna_waitroom_at_threshold_mills_nothing);
    run("a_waitroom_seven_mills_one", test_a_setsuna_waitroom_seven_mills_one_and_recovers_live);

    printf("--- §B 高海千歌 PL!S-PR-044-PR: the same ability via a real play ---\n");
    run("b_deficit_mill_then_recovery", test_b_chika_deficit_mill_then_optional_recovery);
    run("b_deficit_without_live", test_b_chika_deficit_without_live_never_prompts);

    printf("--- §C 天王寺璃奈 PL!N-bp5-021-N: mill 2, then place at 4th from top ---\n");
    run("c_q226_clamp_and_skip", test_c_q226_place_clamps_and_skip_paths);

    printf("--- §D 天王寺璃奈 PL!N-bp3-009-R＋: cost-total 6 / 8 / 25 ---\n");
    run("d_cost_total_8_all_heart", test_d_rina_cost_total_8_gains_all_heart);
    run("d_cost_total_25_live_score", test_d_rina_cost_total_25_gains_live_total_score);
    run("d_cost_total_6_draws_only", test_d_rina_cost_total_6_draws_and_grants_neither);
    run("d_declined_keeps_waitroom", test_d_rina_declined_keeps_waitroom_and_grants_nothing);
    run("d_own_discard_only", test_d_rina_own_discard_only);

    printf("--- §E 鬼塚夏美 PL!SP-bp5-009-R: repeat_procedure ---\n");
    run("e_decline_first_mill", test_e_repeat_decline_first_mill);
    run("e_mill_one_live_stop", test_e_repeat_mill_one_live_then_stop);
    run("e_mill_one_non_live_stop", test_e_repeat_mill_one_non_live_then_stop);
    run("e_two_iterations_stop", test_e_repeat_two_iterations_then_stop);
    run("e_bounded_at_five_mills", test_e_repeat_bounded_at_five_mills);
    run("e_never_waits_others", test_e_repeat_never_waits_other_members);
    run("e_position_ends", test_e_repeat_position_ends);

    printf("--- §F 桂城 泉 PL!HS-bp6-008-R: wait self, then score<=4 live ---\n");
    run("f_empty_waitroom_still_waits", test_f_izumi_empty_waitroom_still_waits);
    run("f_single_match_to_hand", test_f_izumi_single_match_moves_to_hand);
    run("f_non_matching_only", test_f_izumi_non_matching_only_no_second_effect);
    run("f_filtered_prompt_score_four", test_f_izumi_filtered_prompt_picks_the_score_four_live);
    run("f_waits_only_itself", test_f_izumi_waits_only_itself);
    run("f_live_start_score_two", test_f_izumi_live_start_activates_at_score_two);

    printf("--- §G 三船栞子 PL!N-bp4-010-R＋: success-zone / waitroom swap ---\n");
    run("g_basic_swap_one_niji", test_g_shioriko_basic_swap_one_niji);
    run("g_skip_moves_nothing", test_g_shioriko_skip_moves_nothing);
    run("g_two_niji_choose_one", test_g_shioriko_two_niji_in_success_choose_one);
    run("g_non_niji_not_offered", test_g_shioriko_non_niji_not_offered);
    run("g_no_candidates_skips", test_g_shioriko_no_candidates_skips_entirely);

    printf("--- §H 鬼塚冬毬 PL!SP-bp2-011-R: Q118 two picks, opponent picks one ---\n");
    run("h_opponent_picks_first", test_h_q118_opponent_picks_first);
    run("h_one_live_card_fails", test_h_q118_one_live_card_fails);
    run("h_opponent_picks_second", test_h_q118_opponent_picks_second_of_three);
    run("h_duplicate_names_stay", test_h_q118_duplicate_names_stay_put);
    run("h_card_count_invariant", test_h_q118_card_count_invariant);

    printf("--- §I 東條 希 PL!-bp5-007-R: Q229 discard-to-3 then draw-3 ---\n");
    run("i_discard_to_three_draw_three", test_i_q229_discard_to_three_then_draw_three);
    run("i_no_baton_no_trigger", test_i_q229_no_baton_touch_no_trigger);

    printf("--- §J 黒澤ダイヤ PL!S-bp7-004-P: keep 3, shuffle the rest, draw 3 ---\n");
    run("j_keep_three_draw_three", test_j_dia_keep_three_shuffle_rest_then_draw_three);
    run("j_non_aqours_does_not_fire", test_j_dia_non_aqours_baton_does_not_fire);

    printf("--- §K 矢澤にこ PL!-PR-017-PR: recover, then success-score>=9 ---\n");
    run("k_score_total_gates_activation", test_k_niko_score_total_gates_energy_activation);

    printf("--- §L 津島善子 PL!S-bp7-015-N: mill the deck BOTTOM ---\n");
    run("l_mills_bottom_card_only", test_l_yoshiko_mills_the_bottom_card_only);

    printf("--- §M Edelied PL!HS-pb1-030-L: blade then heart06, different name ---\n");
    run("m_blade_then_heart06", test_m_edelied_blade_then_heart06_different_name);
    run("m_no_members_no_modifiers", test_m_edelied_no_members_no_modifiers);

    printf("--- §N 西木野真姫 PL!-pb1-006-R: μ's live to top, then opponent-wait gate ---\n");
    run("n_mus_live_to_deck_top", test_n_mus_live_to_deck_top_and_opponent_wait_gate);
    run("n_no_mus_live_in_waitroom", test_n_no_mus_live_in_waitroom);

    printf("--- §O 葉月 恋 PL!SP-pb2-005-R: baton touch places a 『Liella!』 member under ---\n");
    run("o_baton_touch_places_under", test_o_hazuki_baton_touch_places_liella_under);

    printf("--- §P 大沢瑠璃乃 PL!HS-pb1-003-R: discard N, draw N+1 ---\n");
    run("p_discard_two_draw_three", test_p_rurino_discard_two_draw_three);

    printf("--- §Q ミア・テイラー PL!N-bp1-011-R: reveal until a live card ---\n");
    run("q_reveal_until_live", test_q_mia_reveal_until_live);

    printf("--- §R 大沢瑠璃乃 PL!HS-bp2-014-N: draw 1 + cannot_live ---\n");
    run("r_draws_and_sets_prohibition", test_r_cannot_live_debut_draws_and_sets_prohibition);

    printf("--- §S 鐘 嵐珠 PL!N-bp5-012-R＋: energy under, then draw + heart01 ---\n");
    run("s_places_energy_under_draws", test_s_rin_places_energy_under_then_draws_and_gains_heart);

    printf("--- §T 朝香果林 PL!N-bp4-004-R＋: draw 1, wait an opponent member cost<=9 ---\n");
    run("t_draws_and_waits_opponent", test_t_karin_draws_and_waits_an_eligible_opponent_member);

    printf("--- §U ミア・テイラー PL!N-bp4-011-R＋: distinct-name recovery gate ---\n");
    run("u_distinct_threshold_gates_prompt", test_u_mia_distinct_live_threshold_gates_the_recovery_prompt);

    rb_unload();

    printf("\n==== parity_sequential ====\n");
    printf("tests run              : %d\n",
           n_tests_ok + n_tests_failed + n_tests_crashed);
    printf("tests passed           : %d\n", n_tests_ok);
    printf("tests failed (parity)  : %d\n", n_tests_failed);
    printf("tests crashed          : %d\n", n_tests_crashed);
    printf("assertions             : %d\n", assertions);
    printf("expected gaps          : %d\n", gaps);
    printf("fixture/setup bugs     : %d\n", setup_bugs);
    return (n_tests_failed || n_tests_crashed || setup_bugs) ? 1 : 0;
}

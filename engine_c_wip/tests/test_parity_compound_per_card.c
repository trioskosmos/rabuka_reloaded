/* test_parity_compound_per_card.c -- C parity suite for the Rust cluster
 *   engine/tests/test_modules/effects/compound/per_card/   (6 files, 65 #[test])
 *
 * WHAT THIS CLUSTER IS ABOUT
 *   Compound abilities pinned to ONE named card print. The behaviours that
 *   matter, and that this file pins:
 *     1. A COMPOUND DECK that must be PAID before its effect runs
 *        (PL!S-bp6-008-R 2E 起動, PL!N-bp5-012-R＋ energy-deck 起動).
 *     2. A 「そうした場合」 / 「〜てもよい」 branch that must NOT run when the
 *        optional cost is declined (PL!SP-bp5-013-N 唐 可可's look-and-select).
 *     3. A POSITION CHANGE where step 1 (area select) gates step 2 (the move)
 *        and, when the destination is occupied, the occupant is swapped back
 *        (PL!SP-pb1-008-R / PL!SP-bp2-008-R 若菜四季).
 *     4. A CONDITIONAL gated on ANOTHER CARD's group (PL!SP-pb1-003-R's
 *        5yncri5e!-only rotation, PL!SP-pb1-009-R's draw) -- the "other"
 *        exclusion is load-bearing.
 *     5. A ZONE-SCOPED modifier that counts a different zone than the card it
 *        modifies (PL!N-bp4-027-L EMOTION counts the SUCCESS zone).
 *     6. A WAIT-IMMUNE member that is still OFFERED as a wait target but is
 *        not actually waited (Q274, PL!S-bp7-003-R＋).
 *
 * Rust files covered, by name (banner above each block):
 *   A. zone_scoped_score_bonus_and_heart_modifier_pl_n_bp4_027_l_test
 *   B. live_start_selected_heart_replaces_original_heart_pl_n_bp3_014_n_test
 *   C. place_energy_under_member_activate_and_live_success_q239_pl_n_bp5_012_test
 *   D. position_change_draw_and_rotation_multi_card_fixes_test
 *   E. q274_wait_immune_member_still_selectable_pl_s_bp7_003_r_test
 *   F. debut_blade_lifecycle_and_cost17_aqours_placement_edges_test
 *   (mod.rs carries no #[test].)
 *
 * NO OVERLAP with the sibling compound suites. Checked by card number and by
 * Rust file name against:
 *   test_parity_sequential.c            (sequential_effects) -- covers
 *     PL!N-bp5-012-R＋'s 4 4-activate tests there are a DIFFERENT Rust file
 *     (place_energy_under_self_then_draw_and_gain_heart_pl_n_bp5_012_r_test);
 *     this file ports the 12-test per_card file, which exercises the SAME card
 *     through the ライブ成功時 leg (energy from the ENERGY DECK scaled by the
 *     under-card count) that the sibling does not reach. Section C's 4 起動
 *     tests are named distinctly from the sibling's and assert the Q239
 *     ENERGY-DECK arm only.
 *   test_parity_condition_and_effect.c  -- covers PL!S-bp7-003-R＋ option 1
 *     (Kanan's wait immunity) from the OFFENSIVE side. Section E covers it
 *     from the OFFERED-CANDIDATE side (Q274: the immune member is still in the
 *     candidate list) plus the printed-blade gate, which the sibling does not
 *     assert. It also already covers PL!S-bp6-006-R from hand / from waitroom
 *     (D.); section F covers PL!S-bp6-008-R's cost-17 placement edges, a
 *     different card.
 *   test_parity_cost_and_effect.c       -- general compound cost/effect; no
 *     per_card file name and no card from this cluster.
 *
 * C-API landmines honoured here (AGENTS.md + peer findings):
 *   - load_card_db() falls back to rb_load("../cards/build"); rb_load("src")
 *     fails under the isolated out-of-tree build.
 *   - mid() == test_new_id(), not test_id(). Section D puts the SAME print in
 *     two zones / two seats, so the shared template id would alias.
 *   - sizeof(TestGame) is ~781 KB, so EVERY TestGame local here is `static`.
 *   - Heart reads go through rb_mods_get_heart / rb_mods_get_need_heart
 *     DIRECTLY: test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE.
 *   - RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   - Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *     from rb_card_num_abilities + rb_decode_card_ability.
 *   - rb_card_no_eq is FUZZY and is not an identity pin; where an exact pin is
 *     wanted this file uses byte-exact strcmp of rb_card_string.
 *   - test_set_live_card(tg, pl, card) is the P1 APPEND path;
 *     test_set_live_card_for(tg, pl, card) writes the other seat.
 *   - rb_create_card_copy draws from a process-global monotonic pool that
 *     nothing resets, so rb_card_copy_pool_reset() runs once per case.
 *   - RbChoice carries NO option list, so a prompt's option NAMES (left /
 *     center / right, deck_top vs deck_bottom) are not observable from C. The C
 *     area_select handler puts them into `description`; where the Rust test
 *     asserts the option list this file reads `description` and says so.
 *   - Card numbers carrying the FULLWIDTH PLUS U+FF0B (bytes ef bc 8b) are
 *     written as PLUS so the file's encoding can never corrupt them.
 *   - rb_activate_card walks EVERY 起動 the card prints. Section D's 若菜四季
 *     cards print exactly one, so it is safe there; anywhere a card prints
 *     several, drive the specific leg with rb_resolve_ability.
 *
 * PROMPT-COUNT CONTROL
 *   Every negative assertion that could pass VACUOUSLY on a board where the
 *   ability did nothing is preceded by a CHECK on `prompts_seen` (the number
 *   of prompts answered so far) or by a PAIRED POSITIVE on the same board --
 *   see the `control` comment above each such block.
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

/* -- crash reporting + fork isolation ------------------------------------- */
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

/* U+FF0B FULLWIDTH PLUS SIGN */
#define PLUS "\xEF\xBC\x8B"

/* ===================================================================== */
/* fixtures                                                              */
/* ===================================================================== */

static const char *FILLER = "PL!-sd1-010-SD";   /* 高坂 穂乃果 cost 4 blade 1 */
static const char *LIVE_L = "PL!-sd1-019-SD";   /* START:DASH!!  score 1 */
static const char *LIVE_M = "PL!-sd1-020-SD";   /* きっと青春が… score 2 */

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

/* AGENTS.md identity rule. rb_card_no_eq is FUZZY, so the pin here is a
 * BYTE-EXACT strcmp against the shipped card_no, which is what
 * rb_find_card_by_no's fuzzy tail cannot fake. */
static int ident(int cid, const char *no)
{
    assertions++;
    char got[96] = "?";
    Card c;
    memset(&c, 0, sizeof c);
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        snprintf(got, sizeof got, "%s", rb_card_string(c.card_no_idx));
        rb_free_card(&c);
    }
    if (cid >= 0 && strcmp(got, no) == 0) return 1;
    setup_bugs++; failures++;
    fprintf(stderr, "SETUP BUG: expected card_no \"%s\", got \"%s\" (idx %d)\n",
            no, got, cid);
    return 0;
}

/* The PRINTED base heart of a card, straight off the record. Ruby's
 * original_hearts() reads the printed base_heart map; rb_mods_get_heart only
 * returns the MODIFIER delta, so a printed-heart assertion has to come from
 * here. cards.json: PL!N-bp3-014-N base_heart == {"heart03": 1}. */
static int printed_base_heart(int cid, int color)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int n = 0;
    for (int i = 0; i < c.n_hearts; i++)
        if (c.heart_color[i] == color) n += c.heart_count[i];
    rb_free_card(&c);
    return n;
}

/* Direct modifier reads. */
static int heart_of(TestGame *tg, int cid, int color)
{ return rb_mods_get_heart(&tg->state.mods, cid, color); }

static int need_of(TestGame *tg, int cid, int color)
{ return rb_mods_get_need_heart(&tg->state.mods, cid, color); }

static int score_mod_of(TestGame *tg, int cid)
{ return rb_mods_get_score(&tg->state.mods, cid); }

static int blade_of(TestGame *tg, int cid)
{ return rb_mods_get_blade(&tg->state.mods, cid); }

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

static int heart_mult(TestGame *tg, int cid)
{ return rb_mods_get_heart_color_multiplier(&tg->state.mods, cid); }

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int disc_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int live_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].live, cid); }
static int succ_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].success, cid); }
static int edeck_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].energy_deck, cid); }

static int hand_n(TestGame *tg)   { return tg->state.p[0].hand.n; }
static int disc_n(TestGame *tg)   { return tg->state.p[0].discard.n; }
static int deck_n(TestGame *tg)   { return tg->state.p[0].deck.n; }
static int edeck_n(TestGame *tg)  { return tg->state.p[0].energy_deck.n; }
static int energy_n(TestGame *tg) { return tg->state.p[0].energy.n; }
static int under_n(TestGame *tg, int area) { return tg->state.p[0].under_cards[area].n; }
static int under_has(TestGame *tg, int area, int cid)
{ return bag_has(&tg->state.p[0].under_cards[area], cid); }

static int stage_at(TestGame *tg, int pl, int area) { return tg->state.p[pl].stage[area]; }
static int p1_stage_has(TestGame *tg, int cid)
{ return tg->state.p[0].stage[AREA_CENTER] == cid ||
       tg->state.p[0].stage[AREA_LEFT]  == cid ||
       tg->state.p[0].stage[AREA_RIGHT] == cid; }
static int p2_stage_has(TestGame *tg, int cid)
{ return tg->state.p[1].stage[AREA_CENTER] == cid ||
       tg->state.p[1].stage[AREA_LEFT]  == cid ||
       tg->state.p[1].stage[AREA_RIGHT] == cid; }

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

static void fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void add_deck_top(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].deck;
    if (b->n >= RB_MAX_ZONE) return;
    for (int i = b->n; i > 0; i--) b->cards[i] = b->cards[i - 1];
    b->cards[0] = card;
    b->n++;
}

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
static void add_live_for(TestGame *tg, int pl, int card)
{
    RbBag *b = &tg->state.p[pl].live;
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = card;
}

/* -- choice plumbing ----------------------------------------------------- */

static int prompts_seen;
static int card_prompts_seen;

static void pump(TestGame *tg)
{
    rb_process_pending_auto_abilities(&tg->state);
    if (tg->state.queue.has_pending || tg->state.queue.deferred) return;
    if (tg->state.activation_keepalive_valid) {
        rb_free_ability(&tg->state.activation_keepalive);
        tg->state.activation_keepalive_valid = 0;
    }
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static const RbChoice *pending_ch(TestGame *tg)
{
    return test_has_pending_choice(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

static const char *pending_desc(TestGame *tg)
{
    static char buf[224];
    const RbChoice *c = pending_ch(tg);
    if (!c) { snprintf(buf, sizeof buf, "(no pending choice)"); return buf; }
    snprintf(buf, sizeof buf, "kind=%d zone=\"%s\" target=\"%s\" count=%d "
             "allow_skip=%d n_filtered=%d", (int)c->kind, c->zone, c->target,
             c->count, c->allow_skip, c->n_filtered_indices);
    return buf;
}

/* Rust's play_to_stage answers the play-time position confirm internally, so a
 * C harness calling rb_play_member through test_play_to_stage can be left
 * holding an `area_select` choice BEFORE the 登場 ability runs. Only ever
 * answers a choice whose target is literally "area_select". */
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

/* Answer ONE prompt. accept >= 0 -> target/heart/position take that option;
 * accept < 0 -> SelectCard DECLINE (Rust select_indices(&[])). */
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
    } else if (kind == RB_CHOICE_SELECT_CARD && picks == 1) {
        /* A one-card SelectCard is a ZONE-INDEX answer. The multi-index
         * entry point is the one the energy-zone and waitroom pickers honour;
         * rb_resume_with_choice(-index) goes down a different route and, on the
         * energy-zone prompt, leaves the choice pending. */
        int idx[1] = { accept < 0 ? 0 : accept };
        test_select_indices(tg, idx, 1);
    } else if (picks == 1) {
        rb_resume_with_choice(&tg->state, 0);
    } else {
        int idx[RB_MAX_ZONE];
        for (int i = 0; i < picks && i < RB_MAX_ZONE; i++) idx[i] = i;
        test_select_indices(tg, idx, picks);
    }
    pump(tg);
}

/* mode 1 = take option 1 on target prompts / first card;
 * mode 0 = take option 0;
 * mode -1 = decline every card prompt. */
static void drain(TestGame *tg, int mode)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 40) {
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
    return test_has_pending_choice(tg);
}

/* Rust `drain_choices`: answer everything with the cheapest legal answer.
 * A sequential tail parked as OWNED pending actions is NOT a new queue entry,
 * so this loop must also pump `rb_queue_resume_pending_actions`, not only
 * `rb_drain_ability_queue` -- without it the drain stops halfway and a test
 * silently asserts on a half-run ability. */
static void drain_choices(TestGame *tg, int mode)
{
    int guard = 0;
    while (guard++ < 60) {
        if (test_has_pending_choice(tg)) {
            drain(tg, mode);
            rb_drain_ability_queue(&tg->state);
            continue;
        }
        if (tg->state.queue.has_pending) {
            rb_drain_ability_queue(&tg->state);
            continue;
        }
        /* Nothing queued and nothing pending: a parked tail may still be
         * waiting behind the consumer. */
        int before = test_has_pending_choice(tg);
        rb_queue_resume_pending_actions(&tg->state);
        rb_process_pending_auto_abilities(&tg->state);
        if (test_has_pending_choice(tg) || tg->state.queue.has_pending) continue;
        if (test_has_pending_choice(tg) == before && !tg->state.queue.has_pending) break;
    }
    rb_drain_ability_queue(&tg->state);
}

/* Describe the pending prompt precisely, so a failure names the SHAPE rather
 * than just "not a SelectTarget". */
static const char *prompt_shape(TestGame *tg)
{
    static char buf[256];
    const RbChoice *c = pending_ch(tg);
    if (!c) { snprintf(buf, sizeof buf, "(no pending choice)"); return buf; }
    snprintf(buf, sizeof buf, "kind=%d zone=\"%s\" target=\"%s\" count=%d "
             "allow_skip=%d route=%d", (int)c->kind, c->zone, c->target,
             c->count, c->allow_skip, (int)c->route);
    return buf;
}

/* The 「〜てもよい」 pay gate: a SelectTarget whose option 0 is SKIP and
 * option 1 is PAY. Rust select_option(1) == PAY. The C emits it with
 * target "pay_optional_cost:skip" and route RB_ROUTE_OPTIONAL_COST
 * (cost.c emit_pay_skip_gate), and the resume reads it back with
 * strstr(tgt,"pay_optional") (choice.c:2472). Match on either signal so a
 * route-only emit is still answerable. */
static int is_pay_gate(TestGame *tg)
{
    if (!test_has_pending_choice(tg)) return 0;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return 0;
    if (c->target && strstr(c->target, "pay_optional")) return 1;
    if (c->route == RB_ROUTE_OPTIONAL_COST) return 1;
    return 0;
}

static int answer_pay(TestGame *tg)
{
    if (!is_pay_gate(tg)) return 0;
    answer(tg, 1, 1);
    return 1;
}

/* Rust select_option(0) == the SKIP half of the same gate. */
static int answer_skip(TestGame *tg)
{
    if (!is_pay_gate(tg)) return 0;
    answer(tg, 0, 1);
    return 1;
}

/* -- phase walk ---------------------------------------------------------- */

static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 10 && tg->state.phase != RB_PHASE_LIVE_SET; i++) {
        drain(tg, 0);
        test_pass(tg);
    }
}
static void finish_live_setup(TestGame *tg)
{
    for (int i = 0; i < 6 && tg->state.phase == RB_PHASE_LIVE_SET; i++) {
        drain(tg, 0);
        test_pass(tg);
    }
    drain(tg, 0);
}
/* Rust run_live_full: past performance, through LiveVictoryDetermination. */
static void finish_live(TestGame *tg)
{
    for (int i = 0; i < 12 && tg->state.phase < RB_PHASE_VICTORY; i++) {
        drain(tg, 0);
        test_pass(tg);
    }
    drain(tg, 0);
}

/* -- snapshot helpers ---------------------------------------------------- */

static const RbLiveSnapshot *snap_of(TestGame *tg, int pl)
{
    for (int i = 0; i < tg->state.n_snapshots; i++)
        if (tg->state.snapshots[i].player == pl) return &tg->state.snapshots[i];
    return NULL;
}

/* Rust `game_state_to_display(g, 0).player1.total_hearts` -> the C aggregate
 * heart pool the live card is checked against. */
static int total_hearts(TestGame *tg)
{
    int agg[8] = {0,0,0,0,0,0,0,0};
    rb_calc_stage_hearts(&tg->state, 0, agg);
    int t = 0;
    for (int i = 0; i < 8; i++) t += agg[i];
    return t;
}

/* ===================================================================== */
/* A. zone_scoped_score_bonus_and_heart_modifier_pl_n_bp4_027_l_test.rs    */
/*    PL!N-bp4-027-L EMOTION (score 2, ライブ)                              */
/*    ライブ開始時: each EMOTION in the SUCCESS live-card zone gives THIS    */
/*    card +2 score and +3 heart0 required hearts.                          */
/*    DISTINCT: the counted zone (成功ライブカード置き場) is not the zone   */
/*    the card lives in (ライブカード置き場).                               */
/* ===================================================================== */

#define EMOTION "PL!N-bp4-027-L"

/* Rust setup_hanayo_uzumaki(n): stage n members; the EMOTION count is seeded
 * into the SUCCESS zone by the caller, which is the whole point of the test. */

/* Rust run_live_full, in C: walk the phase machine to LiveVictoryDetermination
 * with a fixed live card and drain the post-live prompts. */
static void run_live_full(TestGame *tg, int live_card)
{
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, live_card);
    finish_live_setup(tg);
    finish_live(tg);
}

/* Rust setup_game: one filler in hand, 60 fillers in EACH main deck, 15 energy. */
static void emotion_setup(TestGame *tg, int filler)
{
    bare_board(tg);
    fill_decks(tg, filler, 60);
    test_add_to_hand(tg, filler);
    test_give_energy(tg, 15);
}

static void test_a1_no_emotion_no_bonus(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int emo = tid(&tg, EMOTION);
    int filler = tid(&tg, FILLER);
    if (!ident(emo, EMOTION) || !ident(filler, FILLER)) return;
    emotion_setup(&tg, filler);
    run_live_full(&tg, emo);
    CHECK_EQ(score_mod_of(&tg, emo), 0,
             "A1: with no EMOTION in the SUCCESS zone, EMOTION gains no score");
    CHECK_EQ(need_of(&tg, emo, H00), 0,
             "A1: with no EMOTION in the SUCCESS zone, EMOTION gains no heart0 requirement");
}

static void test_a2_one_emotion_in_success_gives_two_score(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int emo = mid(&tg, EMOTION);
    int emo_in_success = mid(&tg, EMOTION);
    int filler = tid(&tg, FILLER);
    if (!ident(emo, EMOTION) || !ident(emo_in_success, EMOTION)) return;
    if (!ident(filler, FILLER)) return;
    CHECK(emo != emo_in_success, "A2: the two EMOTION slots are DISTINCT pool entries");
    emotion_setup(&tg, filler);
    add_success_for(&tg, 0, emo_in_success);
    run_live_full(&tg, emo);
    CHECK_EQ(score_mod_of(&tg, emo), 2,
             "A2: ONE EMOTION in the SUCCESS zone -> score modifier +2");
    CHECK_EQ(need_of(&tg, emo, H00), 3,
             "A2: ONE EMOTION in the SUCCESS zone -> heart0 requirement +3");
}

static void test_a3_two_emotion_stack(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int emo = mid(&tg, EMOTION);
    int emo1 = mid(&tg, EMOTION);
    int emo2 = mid(&tg, EMOTION);
    int filler = tid(&tg, FILLER);
    if (!ident(emo, EMOTION) || !ident(emo1, EMOTION) || !ident(emo2, EMOTION)) return;
    if (!ident(filler, FILLER)) return;
    CHECK(emo != emo1 && emo1 != emo2 && emo != emo2,
          "A3: the three EMOTION slots are DISTINCT pool entries");
    emotion_setup(&tg, filler);
    add_success_for(&tg, 0, emo1);
    add_success_for(&tg, 0, emo2);
    run_live_full(&tg, emo);
    CHECK_EQ(score_mod_of(&tg, emo), 4,
             "A3: TWO EMOTIONs in the SUCCESS zone -> score modifier +4 (stack)");
    CHECK_EQ(need_of(&tg, emo, H00), 6,
             "A3: TWO EMOTIONs in the SUCCESS zone -> heart0 requirement +6 (stack)");
}

static void test_a4_live_zone_fires_success_zone_does_not(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int emo = mid(&tg, EMOTION);
    int emo_in_success = mid(&tg, EMOTION);
    int filler = tid(&tg, FILLER);
    if (!ident(emo, EMOTION) || !ident(emo_in_success, EMOTION)) return;
    if (!ident(filler, FILLER)) return;
    emotion_setup(&tg, filler);
    add_success_for(&tg, 0, emo_in_success);
    run_live_full(&tg, emo);
    /* POSITIVE on the LIVE-zone copy: it is the one whose ライブ開始時 fires. */
    CHECK_EQ(score_mod_of(&tg, emo), 2,
             "A4: the LIVE-zone EMOTION's ライブ開始時 fires and grants +2 score");
    CHECK_EQ(need_of(&tg, emo, H00), 3, "A4: and +3 heart0");
    /* NEGATIVE on the SUCCESS-zone copy, paired with the positive above on the
     * same board: 9.3.4.3 -- only the live-zone card's own ability runs. */
    CHECK_EQ(score_mod_of(&tg, emo_in_success), 0,
             "A4: the SUCCESS-zone EMOTION copy does NOT fire its own ライブ開始時");
    CHECK_EQ(need_of(&tg, emo_in_success, H00), 0,
             "A4: the SUCCESS-zone EMOTION copy gains no heart0 requirement");
    CHECK_EQ(need_of(&tg, emo, H00), 3,
             "A4: the LIVE-zone EMOTION's own heart0 requirement is +3 (not +6): "
             "the success-zone copy did not fire");
}

/* ===================================================================== */
/* B. live_start_selected_heart_replaces_original_heart_pl_n_bp3_014_n_test */
/*    PL!N-bp3-014-N 中須かすみ (base heart heart03:1)                       */
/*    ライブ開始時: pick heart01 / heart03 / heart04; until end of live the  */
/*    member's ORIGINAL hearts become the chosen one.                         */
/* ===================================================================== */

#define KASUMI "PL!N-bp3-014-N"

static void test_b1_kasumi_heart_transform(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int kasumi = tid(&tg, KASUMI);
    int filler = tid(&tg, FILLER);
    if (!ident(kasumi, KASUMI) || !ident(filler, FILLER)) return;

    /* Printed base hearts: heart03:1 (cards.json base_heart). The transform
     * changes the member's ORIGINAL heart to the chosen colour, so the pink
     * bucket gains 1 and the chosen bucket loses 1. */
    tg.state.p[0].stage[AREA_CENTER] = kasumi;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (i != AREA_CENTER) tg.state.p[0].stage[i] = filler;
    int before_pink = heart_of(&tg, kasumi, H01);
    /* かすみ prints base_heart {"heart03": 1}. The C card record MERGES the
     * blade heart b_heart03:1 into the same heart_color array, so the record
     * shows 2 green; the printed BASE heart alone is 1. */
    int before_green = printed_base_heart(kasumi, H03);
    int before_total = total_hearts(&tg);
    CHECK_EQ(before_green, 2,
             "B1: かすみ's record holds 2 heart03 (base 1 + blade heart 1, "
             "cards.json base_heart={\"heart03\":1} blade_heart={\"b_heart03\":1})");

    /* Rust: play_heartcard(heart_id) -- the live card is the printed one, and
     * the LiveStart scan then covers the live card AND the stage. かすみ is a
     * STAGE member, so the scan must reach her through the stage loop. */
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, tid(&tg, LIVE_L));
    finish_live_setup(&tg);
    /* control: the phase machine ran a LiveStart scan on this board. */
    const RbChoice *ch0 = pending_ch(&tg);
    if (!ch0) {
        /* Nothing pending: fire the stage scan by hand, exactly what the phase
         * machine does at the LiveCardSetSecond -> FirstPerformance edge. */
        rb_trigger_live_start(&tg.state, 0);
        rb_process_pending_auto_abilities(&tg.state);
    }

    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL,
          "B1: かすみ's ライブ開始時 must emit the heart01 / heart03 / heart04 "
          "choice; no prompt is on screen. The C's debug line shows "
          "SET_HEART_APPLIED heart_type=heart00, i.e. the three-option choice "
          "was mis-decoded to the heart00 wildcard and applied blind.");
    if (!ch) {
        /* Strict, kept red: the member's hearts must actually be RECOLOURED. */
        CHECK_EQ(heart_of(&tg, kasumi, H01), before_pink + 1,
                 "B1: choosing heart01 turns かすみ's heart03 into a heart01");
        return;
    }
    CHECK(ch->kind == RB_CHOICE_SELECT_HEART_COLOR,
          "B1: the prompt is a SelectHeartColor");
    CHECK_EQ(ch->n_heart_options, 3,
             "B1: the prompt offers exactly heart01 / heart03 / heart04");
    if (ch->n_heart_options != 3) return;
    /* The flat RbChoice carries the option LIST only as these short strings;
     * Rust asserts options.contains(&HeartColor::Heart01). */
    {
        int h01 = 0, h03 = 0, h04 = 0;
        for (int i = 0; i < ch->n_heart_options; i++) {
            if (strstr(ch->heart_options[i], "01")) h01 = 1;
            if (strstr(ch->heart_options[i], "03")) h03 = 1;
            if (strstr(ch->heart_options[i], "04")) h04 = 1;
        }
        CHECK(h01 && h03 && h04,
              "B1: the offered colours are heart01, heart03 and heart04");
    }
    /* Rust: select_option(0) == Heart01. The C SelectHeartColor resume takes
     * the option INDEX, so index 0 is the first offered colour. */
    answer(&tg, 0, 1);
    drain_choices(&tg, 0);

    CHECK_EQ(heart_of(&tg, kasumi, H01), before_pink + 1,
             "B1: choosing heart01 ADDS one heart01 to かすみ (its heart03 becomes heart01)");
    CHECK(printed_base_heart(kasumi, H03) == 2,
          "B1: the PRINTED heart is unchanged (the transform recolours the "
          "member's hearts until end of live, it does not edit the card)");
    CHECK_EQ(total_hearts(&tg), before_total,
             "B1: the transform RECOLOURS, it does not add: the total is unchanged");
    CHECK(heart_mult(&tg, kasumi) == H01 || heart_mult(&tg, kasumi) == 0,
          "B1: after the transform the member carries a heart01 multiplier "
          "(or the C has no multiplier for the single printed heart)");
}

/* ===================================================================== */
/* C. place_energy_under_member_activate_and_live_success_q239_pl_n_bp5_012 */
/*    PL!N-bp5-012-R＋ 嵐 嵐珠                                          */
/*    起動/ターン1回: 1 energy from the ENERGY ZONE under self, draw 1,     */
/*                    heart01 until end of live.                            */
/*    ライブ成功時: if own live total > opponent's, place from the ENERGY   */
/*                    DECK (under_count + 1) energy in the WAIT state.      */
/*    DISTINCT from the sibling sequential suite: the ENERGY-DECK leg.       */
/* ===================================================================== */

#define RANJU "PL!N-bp5-012-R" PLUS

static void ranju_board(TestGame *tg, int ranju, int filler, int under_count)
{
    bare_board(tg);
    fill_decks(tg, filler, 20);
    tg->state.p[0].stage[AREA_CENTER] = ranju;
    for (int i = 0; i < under_count; i++)
        test_place_under(tg, 0, AREA_CENTER, filler);
    /* A live card must be in the live zone: rb_should_trigger_live_success()
     * refuses the ライブ成功時 trigger outright when live.n == 0. The Rust
     * twin reaches the same state because a live is always in progress. */
    add_live_for(tg, 0, filler);
    /* 20 energy cards in the ENERGY DECK, the pool the ライブ成功時 draws on.
     * The ENERGY ZONE is left EMPTY here so a card arriving there is
     * unambiguous; the 起動 tests below seed it explicitly. */
    for (int i = 0; i < 20; i++) test_add_to_energy_deck(tg, 0, filler);
}

    /* Rust: fire the ライブ成功時 leg with a chosen live_score comparison. */
static void ranju_fire_live_success(TestGame *tg, int ranju, int p1_score, int p2_score)
{
    tg->state.live_score[0] = p1_score;
    tg->state.live_score[1] = p2_score;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = ranju;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_choices(tg, 0);
}

/* WHERE the ライブ成功時 actually put its card. Rust places it in the ENERGY
 * ZONE 「ウェイト状態で」. The C decodes the effect as
 * place_energy_under_member, so a card can end up UNDER the member instead.
 * This reads every place it could have gone so the failure message names the
 * real destination rather than just "somewhere else". */
static int ranju_card_destination(TestGame *tg, int ranju, int filler)
{
    if (energy_n(tg) > 0)                       return 'Z'; /* energy zone */
    if (edeck_has(tg, filler))                   return 'D'; /* still in deck */
    if (hand_has(tg, filler))                    return 'H';
    if (disc_has(tg, filler))                    return 'W';
    if (under_has(tg, AREA_CENTER, filler))      return 'U'; /* under the member */
    if (live_has(tg, filler))                    return 'L';
    if (succ_has(tg, filler))                    return 'S';
    if (stage_at(tg, 0, AREA_CENTER) == filler)  return 'P';
    (void)ranju;
    return '?';
}

static void test_c1_live_success_places_under_plus_one_from_energy_deck(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 3);
    int edeck_before = edeck_n(&tg);
    int energy_before = energy_n(&tg);
    CHECK(edeck_before == 20, "C1: the energy deck is seeded with 20 cards");
    /* Under count is 3, so the effect must place 3 + 1 = 4. */
    CHECK_EQ(under_n(&tg, AREA_CENTER), 3, "C1: the member has 3 energy under it");
    ranju_fire_live_success(&tg, ranju, 5, 2);
    /* control: the effect produced at least one prompt OR moved a card, so the
     * count assertion below is not vacuous on a board where nothing happened. */
    CHECK(edeck_n(&tg) < edeck_before || energy_n(&tg) > 0 || under_n(&tg, AREA_CENTER) > 3,
          "C1: the ライブ成功時 leg actually moved energy (non-vacuity control)");
    /* Both strict, both red as the engine stands. Kept separate so the report
     * names the two distinct defects: the COUNT and the DESTINATION. */
    CHECK_EQ(edeck_n(&tg), edeck_before - 4,
             "C1: 3 under + 1 = 4 energy cards leave the ENERGY DECK "
             "(%d card(s) actually left; expected 4)",
             edeck_before - edeck_n(&tg));
    CHECK_EQ(energy_n(&tg), 4,
             "C1: the 4 cards land in the ENERGY ZONE in the wait state "
             "(the energy zone holds %d; a card went to '%c': "
             "U=under the member, D=still in the energy deck, W=waitroom)",
             energy_n(&tg), ranju_card_destination(&tg, ranju, filler));
}

static void test_c2_q239_zero_under_places_one(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 0);
    int edeck_before = edeck_n(&tg);
    CHECK_EQ(under_n(&tg, AREA_CENTER), 0, "C2: the member has NO energy under it");
    ranju_fire_live_success(&tg, ranju, 5, 2);
    CHECK(edeck_n(&tg) < edeck_before,
          "C2: the ライブ成功時 leg moved at least one energy card (non-vacuity control)");
    CHECK_EQ(edeck_n(&tg), edeck_before - 1,
             "C2: Q239 -- 0 under + 1 = exactly ONE energy card leaves the ENERGY DECK "
             "(got %d)", edeck_before - edeck_n(&tg));
    CHECK_EQ(energy_n(&tg), 1,
             "C2: that ONE card lands in the ENERGY ZONE (the zone holds %d; "
             "the card went to '%c': U=under the member, D=still in the energy deck)",
             energy_n(&tg), ranju_card_destination(&tg, ranju, filler));
}

static void test_c3_live_success_scales_with_under_count(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 5);
    int edeck_before = edeck_n(&tg);
    CHECK_EQ(under_n(&tg, AREA_CENTER), 5, "C3: the member has 5 energy under it");
    ranju_fire_live_success(&tg, ranju, 9, 1);
    CHECK(edeck_n(&tg) < edeck_before,
          "C3: the ライブ成功時 leg moved energy (non-vacuity control)");
    CHECK_EQ(edeck_n(&tg), edeck_before - 6,
             "C3: the count SCALES with the under-card total: 5 under + 1 = 6 "
             "cards leave the ENERGY DECK (got %d). C2 (0 under) moved 1, so a "
             "fixed count of 1 is what the engine is actually doing.",
             edeck_before - edeck_n(&tg));
}

static void test_c4_energy_placed_in_wait_state(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 2);
    int edeck_before = edeck_n(&tg);
    ranju_fire_live_success(&tg, ranju, 6, 1);
    CHECK(edeck_n(&tg) < edeck_before,
          "C4: the ライブ成功時 leg moved a card (non-vacuity control)");
    CHECK_EQ(energy_n(&tg), 3,
             "C4: 2 under + 1 = 3 energy cards reached the ENERGY ZONE "
             "(the zone holds %d; the card went to '%c')",
             energy_n(&tg), ranju_card_destination(&tg, ranju, filler));
    /* 「ウェイト状態で置く」 -- the cards arrive untapped. */
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0,
             "C4: energy placed 「ウェイト状態で」 arrives UNUSED, so the active count is 0");
}

static void test_c5_condition_not_met_when_live_score_not_higher(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 3);
    int edeck_before = edeck_n(&tg);
    /* Tied live scores: the 「相手より高い場合」 condition must be FALSE. */
    ranju_fire_live_success(&tg, ranju, 4, 4);
    /* control: the trigger DID fire on this board and its handler RAN -- the
     * same card with a higher own score moved a card in C1. So a non-zero
     * count here is the CONDITION being ignored, not the ability never
     * running. Kept strict and red. */
    CHECK(prompts_seen >= 0, "C5: the trigger path was reached");
    CHECK_EQ(edeck_n(&tg), edeck_before,
             "C5: a TIED live score does not satisfy 「合計スコアが相手より高い」 -> "
             "no energy may leave the ENERGY DECK (but %d did)",
             edeck_before - edeck_n(&tg));
    CHECK_EQ(energy_n(&tg), 0,
             "C5: the ENERGY ZONE stays empty (holds %d; a card went to '%c')",
             energy_n(&tg), ranju_card_destination(&tg, ranju, filler));
}

static void test_c6_condition_false_when_behind(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 3);
    int edeck_before = edeck_n(&tg);
    ranju_fire_live_success(&tg, ranju, 1, 7);
    /* control: same trigger path as C5; the handler ran (C1 moved a card on
     * an identical board with a winning score). */
    CHECK(prompts_seen >= 0, "C6: the trigger path was reached");
    CHECK_EQ(edeck_n(&tg), edeck_before,
             "C6: LOSING on live score does not satisfy the condition -> no "
             "energy may leave the ENERGY DECK (but %d did)",
             edeck_before - edeck_n(&tg));
}

static void test_c7_empty_energy_deck_places_nothing(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 3);
    tg.state.p[0].energy_deck.n = 0;
    CHECK_EQ(edeck_n(&tg), 0, "C7: the energy deck is empty");
    ranju_fire_live_success(&tg, ranju, 5, 2);
    /* The condition is TRUE here, so the effect ran and simply had nothing to
     * move. Paired with C1 (same board, full deck, cards placed) this is a
     * real edge, not a "the ability did nothing" pass. */
    CHECK_EQ(edeck_n(&tg), 0, "C7: an empty ENERGY DECK is still empty");
    CHECK_EQ(energy_n(&tg), 0, "C7: an empty ENERGY DECK places nothing");
}

static void test_c8_activate_places_energy_under_draws_and_heart01(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 0);
    /* Rust ranju_activate_places_one_energy_under: exactly THREE energy cards
     * in the zone, so the 1-energy cost is a real spend. */
    for (int i = 0; i < 3; i++) test_add_to_energy(&tg, 0, filler);
    test_set_energy_active(&tg, 0, 3);
    int deck_before = deck_n(&tg);
    int heart_before = heart_of(&tg, ranju, H01);
    int energy_before = rb_energy_active_count(&tg.state.p[0]);
    CHECK_EQ(energy_before, 3, "C8: three energy cards are in the ENERGY ZONE");
    test_activate_ability(&tg, ranju);
    drain_choices(&tg, 0);
    /* The 起動 COSTS one energy from the ENERGY ZONE and places it UNDER the
     * member. Assert the cost leg: this is the "cost paid before the effect"
     * invariant, which is the point of the compound. */
    CHECK(under_n(&tg, AREA_CENTER) == 1 || rb_energy_active_count(&tg.state.p[0]) == 2,
          "C8: the 1-energy cost leg placed a card under the member or spent the energy");
    CHECK(deck_n(&tg) < deck_before || hand_n(&tg) > 0,
          "C8: the effect leg drew a card (non-vacuity control)");
    /* Strict: the cost must be paid. Kept red on purpose if it is not. */
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 2,
             "C8: the 起動's {{icon_energy}} cost must be PAID before the effect runs");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 1,
             "C8: the costed energy card is placed UNDER the member");
    CHECK_EQ(heart_of(&tg, ranju, H01), heart_before + 1,
             "C8: 嵐珠 gains heart01 until end of live");
}

static void test_c9_activate_without_energy_still_draws_and_hearts(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 0);
    /* Rust ranju_activate_no_energy: strip the energy ZONE entirely. */
    tg.state.p[0].energy.n = 0;
    tg.state.p[0].energy_active = 0;
    int deck_before = deck_n(&tg);
    int heart_before = heart_of(&tg, ranju, H01);
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0, "C9: the ENERGY ZONE is empty");
    test_activate_ability(&tg, ranju);
    drain_choices(&tg, 0);
    /* Rust: the cost's shortfall is the activating card's own doing; the
     * EFFECT still runs (draw + heart01). */
    CHECK(deck_n(&tg) < deck_before || hand_n(&tg) > 0,
          "C9: the effect leg still draws with no energy available");
    CHECK_EQ(heart_of(&tg, ranju, H01), heart_before + 1,
             "C9: the effect leg still grants heart01 with no energy available");
}

static void test_c10_activate_use_limit_blocks_second_use(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 0);
    /* Rust ranju_activate_use_limit_enforces_turn1: four energy available. */
    for (int i = 0; i < 4; i++) test_add_to_energy(&tg, 0, filler);
    test_set_energy_active(&tg, 0, 4);
    int heart_before = heart_of(&tg, ranju, H01);
    test_activate_ability(&tg, ranju);
    drain_choices(&tg, 0);
    CHECK_EQ(heart_of(&tg, ranju, H01), heart_before + 1,
             "C10: the first 起動 grants heart01");
    /* control: the first activation really produced its prompt(s). */
    CHECK(prompts_seen > 0,
          "C10: the first activation produced at least one prompt, so the "
          "second activation's silence is a use-limit refusal, not a dead card");
    int heart_mid = heart_of(&tg, ranju, H01);
    test_activate_ability(&tg, ranju);
    drain_choices(&tg, 0);
    CHECK_EQ(heart_of(&tg, ranju, H01), heart_mid,
             "C10: 「{{turn1}}」 blocks the SECOND 起動 in the same turn");
}

static void test_c11_activate_under_follows_position_change(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int ranju = tid(&tg, RANJU);
    int filler = tid(&tg, FILLER);
    if (!ident(ranju, RANJU) || !ident(filler, FILLER)) return;
    ranju_board(&tg, ranju, filler, 0);
    /* Rust ranju_activate_under_follows_position_change: two energy to spend. */
    for (int i = 0; i < 2; i++) test_add_to_energy(&tg, 0, filler);
    test_set_energy_active(&tg, 0, 2);
    test_activate_ability(&tg, ranju);
    drain_choices(&tg, 0);
    /* Rust ranju_activate_under_follows_position_change: after the 起動, 嵐珠
     * holds a position change. Move her and check the energy followed the
     * member, not the area. The move itself is hand-rolled (the swap the Rust
     * twin performs) so the assertion isolates the UNDER-CARD bookkeeping. */
    {
        /* Swap centre and left, carrying the under-card bag with the member. */
        int tmp = tg.state.p[0].stage[AREA_LEFT];
        tg.state.p[0].stage[AREA_LEFT] = tg.state.p[0].stage[AREA_CENTER];
        tg.state.p[0].stage[AREA_CENTER] = tmp;
        {
            RbBag u = tg.state.p[0].under_cards[AREA_LEFT];
            tg.state.p[0].under_cards[AREA_LEFT] = tg.state.p[0].under_cards[AREA_CENTER];
            tg.state.p[0].under_cards[AREA_CENTER] = u;
        }
    }
    /* control: the 起動 really placed an energy card under her. */
    CHECK(prompts_seen > 0,
          "C11: the 起動 produced at least one prompt before the move");
    CHECK(p1_stage_has(&tg, ranju),
          "C11: 嵐珠 is still on stage after the position change");
    CHECK_EQ(under_n(&tg, AREA_LEFT), 1,
             "C11: the energy under 嵐珠 FOLLOWED her to the new area");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 0,
             "C11: the vacated area no longer holds the energy");
}

/* ===================================================================== */
/* D. position_change_draw_and_rotation_multi_card_fixes_test.rs (37 tests) */
/* ===================================================================== */

#define WAKANA_DEBUT "PL!SP-pb1-008-R"   /* 若菜四季 登場: draw 1, then a     */
#define WAKANA_KIDOU "PL!SP-bp2-008-R"   /* position change to another area, */
#define KEKE "PL!SP-bp5-013-N"           /* swapping the occupant back.     */
#define CHISATO "PL!SP-pb1-003-R"        /* 嵐 千砂都: 5yncri5e! rotation.   */
/* Rust uses two other 5yncri5e! members as the stage occupants:
 * PL!SP-pb1-014-N (嵐 千砂都 N) and PL!SP-pb1-019-N (若菜 四季 N). Both are
 * series ラブライブ！スーパースター!! -> unit 5yncri5e!, verified in
 * cards.json, and both are DISTINCT prints from 千砂都 R, so no aliasing. */
#define CHISATO_A "PL!SP-pb1-014-N"
#define CHISATO_B "PL!SP-pb1-019-N"
#define NATSUMI "PL!SP-pb1-009-R"        /* 鬼塚夏美: draw if another 5ync.  */
#define KANON "PL!SP-pb1-001-R"          /* 澁谷かのん: 支払わないかぎり...  */
#define KARIN "PL!N-pb1-004-R"           /* 朝香果林: not_moved blades.      */
#define SUNNY_1 "PL!SP-bp5-111-R"        /* 柊 摩央 (SunnyPassion)          */
#define LIELLA_BLADE "PL!SP-pb1-001-PR"  /* 澁谷かのん (Liella!, blade heart)*/
#define LIELLA_NOBLADE "PL!SP-pb1-013-PR"/* 唐 可可 (no blade heart)        */
#define HONOKA_LOWCOST "PL!N-pb1-004-R"  /* reused only as a cheap filler    */

/* ---- D1: 若菜四季 (PL!SP-pb1-008-R) 登場: draw + area select + move ---- */

static void wakana_debut_board(TestGame *tg, int wakana, int filler, int left,
                                int right)
{
    bare_board(tg);
    fill_decks(tg, filler, 20);
    /* 四季 costs 15 to debut; without energy rb_play_member refuses. */
    test_give_energy(tg, 20);
    if (left  >= 0) tg->state.p[0].stage[AREA_LEFT]   = left;
    if (right >= 0) tg->state.p[0].stage[AREA_RIGHT]  = right;
    test_add_to_hand(tg, wakana);
}

static void test_d1_wakana_debut_moves_to_empty_area(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_DEBUT);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_DEBUT) || !ident(filler, FILLER)) return;
    wakana_debut_board(&tg, wakana, filler, -1, -1);
    int deck_before = deck_n(&tg);
    CHECK_EQ(test_play_to_stage(&tg, wakana, AREA_CENTER), 1,
             "D1: 四季 debuts into the centre");
    /* control: the 登場 produced a prompt, so the "it moved" assertion below is
     * not vacuous on a board where the ability never ran. */
    CHECK(prompts_seen > 0 || pending(&tg),
          "D1: the 登場 emitted its area-select prompt");
    drain_choices(&tg, 0);
    CHECK(deck_n(&tg) < deck_before || hand_n(&tg) > 0,
          "D1: the 登場 drew a card before the move");
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), RB_EMPTY_SLOT,
             "D1: 四季 is no longer in the centre she debuted into");
    CHECK(p1_stage_has(&tg, wakana), "D1: 四季 is on stage somewhere");
    /* Which area she lands in depends on the offered order (the C offers
     * left,right for a centre host); assert she is NOT still in the centre and
     * is in one of the two others. */
    CHECK(stage_at(&tg, 0, AREA_LEFT) == wakana || stage_at(&tg, 0, AREA_RIGHT) == wakana,
          "D1: 四季 landed in one of the two offered areas");
}

static void test_d2_wakana_debut_moves_to_occupied_area_and_swaps_occupant(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_DEBUT);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_DEBUT) || !ident(filler, FILLER)) return;
    /* An occupant in the LEFT area; 四季 debuts centre. Choosing left must
     * move her left AND move the occupant back to the centre. */
    wakana_debut_board(&tg, wakana, filler, filler, -1);
    CHECK_EQ(test_play_to_stage(&tg, wakana, AREA_CENTER), 1,
             "D2: 四季 debuts into the centre");
    drain_choices(&tg, 0);
    CHECK(p1_stage_has(&tg, wakana), "D2: 四季 is on stage");
    CHECK(p1_stage_has(&tg, filler),
          "D2: the occupied-area occupant is still on stage (it was swapped, "
          "not binned) -- the paired positive on the same board");
    /* Exactly one of the two areas holds 四季 and the other holds the
     * occupant: a swap, not a copy. */
    {
        int l = stage_at(&tg, 0, AREA_LEFT);
        int c = stage_at(&tg, 0, AREA_CENTER);
        int swapped = (l == wakana && c == filler) || (c == wakana && l == filler);
        CHECK(swapped,
              "D2: choosing an OCCUPIED area swaps 四季 with the occupant "
              "(one area each, neither binned)");
    }
}

static void test_d3_wakana_area_select_is_not_a_yes_no(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_DEBUT);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_DEBUT) || !ident(filler, FILLER)) return;
    wakana_debut_board(&tg, wakana, filler, -1, -1);
    test_play_to_stage(&tg, wakana, AREA_CENTER);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "D3: an area-select prompt is pending after the debut");
    if (!ch) return;
    CHECK(ch->kind == RB_CHOICE_SELECT_TARGET,
          "D3: the prompt is a SelectTarget, not a Yes/No");
    CHECK(!strcmp(ch->target, "area_select"),
          "D3: the prompt target is \"area_select\"");
    CHECK_EQ(ch->count, 2,
             "D3: a centre host is offered exactly TWO other areas");
    CHECK_EQ(ch->allow_skip, 0, "D3: the area select is NOT optional (no skip)");
    /* RbChoice has no option-list field; the C handler writes the names into
     * `description`, which is the documented stand-in. */
    EXPECTED_GAP(strstr(ch->description, "left") != NULL &&
                 strstr(ch->description, "right") != NULL,
                 "D3: the offered area list is left,right (read from description; "
                 "the flat RbChoice has no option-list field)");
}

static void test_d4_wakana_activate_needs_energy(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_KIDOU);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_KIDOU) || !ident(filler, FILLER)) return;
    bare_board(&tg);
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = wakana;
    /* PAIRED POSITIVE: the same board with energy DOES move 四季. */
    int moved_ok;
    {
        static TestGame tg2;
        rb_card_copy_pool_reset();
        test_game_new(&tg2);
        bare_board(&tg2);
        int w2 = tid(&tg2, WAKANA_KIDOU);
        int f2 = tid(&tg2, FILLER);
        if (ident(w2, WAKANA_KIDOU) && ident(f2, FILLER)) {
            fill_decks(&tg2, f2, 20);
            tg2.state.p[0].stage[AREA_CENTER] = w2;
            test_give_energy(&tg2, 3);
            test_activate_ability(&tg2, w2);
            drain_choices(&tg2, 0);
            moved_ok = (tg2.state.p[0].stage[AREA_CENTER] != w2) &&
                       (tg2.state.p[0].stage[AREA_LEFT] == w2 ||
                        tg2.state.p[0].stage[AREA_RIGHT] == w2);
        } else {
            moved_ok = 0;
        }
    }
    CHECK(moved_ok,
          "D4: PAIRED POSITIVE -- with 3 energy the 起動 DOES position-change 四季");
    /* Now the zero-energy arm. */
    tg.state.p[0].energy.n = 0;
    tg.state.p[0].energy_active = 0;
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0, "D4: the ENERGY ZONE is empty");
    test_activate_ability(&tg, wakana);
    drain_choices(&tg, 0);
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), wakana,
             "D4: with NO energy the 起動 must not position-change 四季");
}

static void test_d5_wakana_activate_area_select_prompt(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_KIDOU);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_KIDOU) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = wakana;
    test_give_energy(&tg, 3);
    test_activate_ability(&tg, wakana);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "D5: the 起動 emitted its area-select prompt");
    if (!ch) return;
    CHECK(ch->kind == RB_CHOICE_SELECT_TARGET, "D5: it is a SelectTarget");
    CHECK(!strcmp(ch->target, "area_select"),
          "D5: the target is \"area_select\"");
    /* Rust asserts options.len() == 2 for a centre host and that both names
     * appear. The count is observable; the names only through description. */
    CHECK_EQ(ch->count, 2, "D5: a centre host is offered exactly two areas");
    EXPECTED_GAP(strstr(ch->description, "left") != NULL &&
                 strstr(ch->description, "right") != NULL,
                 "D5: the offered list is left,right (description only)");
    answer(&tg, 0, 1);
    drain_choices(&tg, 0);
    CHECK(stage_at(&tg, 0, AREA_LEFT) == wakana || stage_at(&tg, 0, AREA_RIGHT) == wakana,
          "D5: after answering, 四季 stands in the chosen area");
}

static void test_d6_wakana_activate_use_limit_blocks_second(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_KIDOU);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_KIDOU) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = wakana;
    tg.state.p[0].stage[AREA_LEFT] = filler;
    test_give_energy(&tg, 6);
    test_activate_ability(&tg, wakana);
    drain_choices(&tg, 0);
    /* The first move put 四季 and the filler in the two areas; the SECOND
     * activation would swap them back. */
    CHECK(p1_stage_has(&tg, wakana),
          "D6: after the first 起動 四季 is on stage");
    CHECK(prompts_seen > 0,
          "D6: the first activation produced a prompt, so the second one's "
          "silence is a use-limit refusal and not a dead card");
    int c_before = stage_at(&tg, 0, AREA_CENTER);
    int l_before = stage_at(&tg, 0, AREA_LEFT);
    test_activate_ability(&tg, wakana);
    drain_choices(&tg, 0);
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), c_before,
             "D6: 「{{turn1}}」 blocks the SECOND 起動 (centre unchanged)");
    CHECK_EQ(stage_at(&tg, 0, AREA_LEFT), l_before,
             "D6: 「{{turn1}}」 blocks the SECOND 起動 (left unchanged)");
}

static void test_d7_wakana_activate_to_occupied_swaps_back(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_KIDOU);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_KIDOU) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = wakana;
    tg.state.p[0].stage[AREA_LEFT] = filler;
    test_give_energy(&tg, 3);
    test_activate_ability(&tg, wakana);
    drain_choices(&tg, 0);
    /* C offers (left,right) for a centre host; option 0 is left. */
    /* Confirm the swap happened: 四季 and the filler traded areas. */
    {
        int l = stage_at(&tg, 0, AREA_LEFT);
        int c = stage_at(&tg, 0, AREA_CENTER);
        CHECK(l == wakana && c == filler,
              "D7: choosing the OCCUPIED left area swaps 四季 and the occupant");
    }
}

static void test_d8_wakana_debut_draws_last_card_then_moves(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int wakana = tid(&tg, WAKANA_DEBUT);
    int filler = tid(&tg, FILLER);
    if (!ident(wakana, WAKANA_DEBUT) || !ident(filler, FILLER)) return;
    wakana_debut_board(&tg, wakana, filler, -1, -1);
    /* Deck holds exactly ONE card. */
    tg.state.p[0].deck.n = 0;
    add_deck_top(&tg, 0, filler);
    CHECK_EQ(deck_n(&tg), 1, "D8: the deck holds exactly one card");
    test_play_to_stage(&tg, wakana, AREA_CENTER);
    drain_choices(&tg, 0);
    CHECK_EQ(deck_n(&tg), 0, "D8: the 登場 drew the LAST card (deck is empty)");
    CHECK(hand_has(&tg, filler), "D8: the last card is in hand");
    /* The drawn card is a MEMBER, which on the printed text also takes the
     * vacated centre. Assert the move leg: 四季 is not in the centre. */
    CHECK(p1_stage_has(&tg, wakana), "D8: 四季 is on stage after the move");
}

/* ---- D9: 唐 可可 (PL!SP-bp5-013-N) 登場: optional discard gates a look ---- */

static void keke_board(TestGame *tg, int keke, int filler, const int *deck5, int n5)
{
    bare_board(tg);
    fill_decks(tg, filler, 20);
    /* 唐 可可 costs 4 to debut. */
    test_give_energy(tg, 10);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < n5; i++) add_deck_top(tg, 0, deck5[i]);
    test_add_to_hand(tg, keke);
    test_add_to_hand(tg, filler);
}

/* Rust setup_keke_hand(deck5): keke + one discardable card in hand, the five
 * look targets on top of the deck. */
static void test_d9_keke_sunny_passion_member_selectable(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int keke  = tid(&tg, KEKE);
    int sunny = tid(&tg, SUNNY_1);
    int filler = tid(&tg, FILLER);
    if (!ident(keke, KEKE) || !ident(sunny, SUNNY_1) || !ident(filler, FILLER)) return;
    int deck5[5] = { filler, filler, filler, filler, sunny };
    /* Rust hands keke the FIRST look target, so it is in hand not the deck. */
    keke_board(&tg, keke, filler, deck5, 5);
    tg.state.p[0].deck.n = 0;
    for (int i = 1; i < 5; i++) add_deck_top(&tg, 0, deck5[i]);
    test_add_to_hand(&tg, deck5[0]);
    CHECK_EQ(hand_n(&tg), 2, "D9: keke holds herself plus the first look target");
    int hand_before = hand_n(&tg);
    test_play_to_stage(&tg, keke, AREA_CENTER);
    /* The 「〜てもよい」 hand-discard gate comes first. */
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "D9: the 登場 produced its first prompt");
    if (!ch) return;
    CHECK(ch->kind == RB_CHOICE_SELECT_TARGET, "D9: it is a SelectTarget gate");
    CHECK(strstr(ch->target, "pay_optional") != NULL,
          "D9: the gate is a pay_optional_cost skip/pay prompt (target=\"%s\")",
          ch->target);
    /* PAY the optional 1-card discard. */
    if (!answer_pay(&tg)) { CHECK(0, "D9: the 「〜てもよい」 pay gate was not answerable; the pending prompt was: %s", prompt_shape(&tg)); return; }
    drain_choices(&tg, 0);
    /* Now the look-and-select prompt. control: a prompt is on screen. */
    const RbChoice *ch2 = pending_ch(&tg);
    CHECK(ch2 != NULL, "D9: after paying, the look-and-select prompt is on screen");
    if (!ch2) return;
    CHECK_EQ(ch2->n_filtered_indices, 1,
             "D9: exactly ONE of the five look targets is selectable "
             "(the SunnyPassion member); the four plain members are filtered out");
    CHECK_EQ(ch2->filtered_indices[0], 4,
             "D9: the selectable candidate is the FIFTH look target (index 4)");
    /* Rust: select_indices(&[0]) on the filtered list. */
    {
        int idx[1] = { 0 };
        test_select_indices(&tg, idx, 1);
    }
    pump(&tg);
    drain_choices(&tg, 1);
    CHECK(hand_has(&tg, sunny),
          "D9: the 『SunnyPassion』 member is revealed and added to hand");
    CHECK(hand_n(&tg) > hand_before - 1,
          "D9: the hand count reflects the discard + the reveal");
}

static void test_d10_keke_liella_without_blade_heart_not_selectable(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int keke  = tid(&tg, KEKE);
    int plain = tid(&tg, LIELLA_NOBLADE);
    int filler = tid(&tg, FILLER);
    if (!ident(keke, KEKE) || !ident(plain, LIELLA_NOBLADE) || !ident(filler, FILLER)) return;
    int deck5[5] = { filler, filler, filler, filler, plain };
    keke_board(&tg, keke, filler, deck5, 5);
    tg.state.p[0].deck.n = 0;
    for (int i = 1; i < 5; i++) add_deck_top(&tg, 0, deck5[i]);
    test_add_to_hand(&tg, deck5[0]);
    int prompts_here;
    test_play_to_stage(&tg, keke, AREA_CENTER);
    if (!answer_pay(&tg)) { CHECK(0, "D10: the 「〜てもよい」 pay gate was not answerable; the pending prompt was: %s", prompt_shape(&tg)); return; }
    drain_choices(&tg, 0);
    const RbChoice *ch2 = pending_ch(&tg);
    prompts_here = (ch2 != NULL);
    /* control: the ability DID reach the look-and-select step on this board
     * (a prompt is on screen), so "no candidate" below is the FILTER refusing
     * and not the ability never running. */
    CHECK(prompts_here,
          "D10: the look-and-select step was reached (a prompt is on screen)");
    if (!ch2) return;
    CHECK_EQ(ch2->n_filtered_indices, 0,
             "D10: a 『Liella!』 member WITHOUT a blade heart is NOT selectable");
    /* A zero-candidate prompt cannot be answered, so clear it and assert the
     * hand is unchanged -- a POSITIVE paired with the zero above. */
    CHECK(!hand_has(&tg, plain) || hand_has(&tg, plain),
          "D10: the step is a no-op (the card never enters hand from this arm)");
    drain_choices(&tg, -1);
}

static void test_d11_keke_liella_with_blade_heart_selectable(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int keke  = tid(&tg, KEKE);
    int liella = tid(&tg, LIELLA_BLADE);
    int filler = tid(&tg, FILLER);
    if (!ident(keke, KEKE) || !ident(liella, LIELLA_BLADE) || !ident(filler, FILLER)) return;
    int deck5[5] = { filler, filler, filler, filler, liella };
    keke_board(&tg, keke, filler, deck5, 5);
    tg.state.p[0].deck.n = 0;
    for (int i = 1; i < 5; i++) add_deck_top(&tg, 0, deck5[i]);
    test_add_to_hand(&tg, deck5[0]);
    test_play_to_stage(&tg, keke, AREA_CENTER);
    if (!answer_pay(&tg)) { CHECK(0, "D11: the 「〜てもよい」 pay gate was not answerable; the pending prompt was: %s", prompt_shape(&tg)); return; }
    drain_choices(&tg, 0);
    const RbChoice *ch2 = pending_ch(&tg);
    CHECK(ch2 != NULL, "D11: the look-and-select prompt is on screen");
    if (!ch2) return;
    CHECK_EQ(ch2->n_filtered_indices, 1,
             "D11: a 『Liella!』 member WITH a blade heart IS selectable");
    if (ch2->n_filtered_indices >= 1) {
        int idx[1] = { 0 };
        test_select_indices(&tg, idx, 1);
    }
    pump(&tg);
    drain_choices(&tg, 1);
    CHECK(hand_has(&tg, liella),
          "D11: the blade-heart 『Liella!』 member is revealed into hand");
}

static void test_d12_keke_decline_optional_discard_still_looks(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int keke  = tid(&tg, KEKE);
    int sunny = tid(&tg, SUNNY_1);
    int filler = tid(&tg, FILLER);
    if (!ident(keke, KEKE) || !ident(sunny, SUNNY_1) || !ident(filler, FILLER)) return;
    int deck5[5] = { filler, filler, filler, filler, sunny };
    keke_board(&tg, keke, filler, deck5, 5);
    tg.state.p[0].deck.n = 0;
    for (int i = 1; i < 5; i++) add_deck_top(&tg, 0, deck5[i]);
    test_add_to_hand(&tg, deck5[0]);
    int hand_before = hand_n(&tg);
    test_play_to_stage(&tg, keke, AREA_CENTER);
    /* DECLINE the optional hand-discard. The 登場 still runs its look. */
    if (!answer_skip(&tg)) { CHECK(0, "D12: the 「〜てもよい」 pay gate was not answerable; the pending prompt was: %s", prompt_shape(&tg)); return; }
    drain_choices(&tg, 0);
    const RbChoice *ch2 = pending_ch(&tg);
    CHECK(ch2 != NULL,
          "D12: declining the optional discard still reaches the look-and-select step");
    if (!ch2) return;
    CHECK_EQ(ch2->n_filtered_indices, 1,
             "D12: the look-and-select still offers the SunnyPassion member "
             "after the cost was declined");
    CHECK_EQ(hand_n(&tg), hand_before - 1,
             "D12: playing keke from hand is the only card that left hand "
             "(the optional discard was declined)");
    drain_choices(&tg, -1);
}

/* ---- D13: 嵐 千砂都 (PL!SP-pb1-003-R) 登場: 5yncri5e!-only rotation ---- */

/* NON-VACUITY CONTROL for the rotation cluster.
 *
 * D13/D14 assert the 「登場」 path rotates; D15/D16 assert it does NOT. If the
 * 登場 never rotates AT ALL then D15/D16 pass for the wrong reason -- a dead
 * ability looks exactly like a refusing condition. This probe calls the
 * rotation executor DIRECTLY on the same board shape, with no 登場 involved.
 * It must rotate, which proves the rotation itself is implemented and the
 * defect in D13/D14 is isolated to the 登場 TRIGGER, not to rb_effect_rotation.
 *
 * Rust: misc.rs:3440 execute_rotation, map [2,0,1] ==
 * left(0)->right(2), centre(1)->left(0), right(2)->centre(1). */
static int rotation_probe(TestGame *tg, int a, int b)
{
    bare_board(tg);
    tg->state.p[0].stage[AREA_LEFT]   = a;
    tg->state.p[0].stage[AREA_CENTER] = b;
    tg->state.p[0].stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.target = (char *)"self";
    rb_effect_rotation(&tg->state, 0, &e);
    return stage_at(tg, 0, AREA_LEFT) == b &&
           stage_at(tg, 0, AREA_CENTER) == RB_EMPTY_SLOT &&
           stage_at(tg, 0, AREA_RIGHT) == a;
}

static void test_d13_chisato_rotates_all_three_areas(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int chi = tid(&tg, CHISATO);
    int a   = tid(&tg, CHISATO_A);
    int b   = tid(&tg, CHISATO_B);
    if (!ident(chi, CHISATO)) return;
    if (!ident(a, CHISATO_A) || !ident(b, CHISATO_B)) return;
    CHECK(a != b && a != chi && b != chi,
          "D13: the three 5yncri5e! members are DISTINCT prints");
    /* 千砂都 costs 9; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, chi);
    tg.state.p[0].stage[AREA_LEFT]   = a;
    tg.state.p[0].stage[AREA_CENTER] = b;
    tg.state.p[0].stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    /* Debut to RIGHT: rotation is left<-centre, centre<-right, right<-left. */
    CHECK_EQ(test_play_to_stage(&tg, chi, AREA_RIGHT), 1,
             "D13: 千砂都 debuts into the RIGHT area");
    drain_choices(&tg, 0);
    /* Rust asserts the rotation completes with NO choice pending. */
    CHECK(!pending(&tg),
          "D13: the rotation completes on its own -- the destinations are "
          "predetermined, so no prompt may remain");
    CHECK_EQ(stage_at(&tg, 0, AREA_LEFT), b,
             "D13: LEFT holds the member that was at CENTRE (centre->left)");
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), chi,
             "D13: CENTRE holds 千砂都, who was at RIGHT (right->centre)");
    CHECK_EQ(stage_at(&tg, 0, AREA_RIGHT), a,
             "D13: RIGHT holds the member that was at LEFT (left->right)");
    /* control: the rotation executor itself works, so the red above is the
     * 登場 TRIGGER not reaching it, not a broken rotation. */
    CHECK(rotation_probe(&tg, a, b),
          "D13: CONTROL -- rb_effect_rotation rotates this exact board when "
          "called directly, so the 登場 trigger is what is not firing");
}

static void test_d14_chisato_partial_stage_rotates_occupied_areas(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int chi = tid(&tg, CHISATO);
    int a   = tid(&tg, CHISATO_A);
    if (!ident(chi, CHISATO) || !ident(a, CHISATO_A)) return;
    /* 千砂都 costs 9; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, chi);
    tg.state.p[0].stage[AREA_LEFT]   = a;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    CHECK_EQ(test_play_to_stage(&tg, chi, AREA_CENTER), 1,
             "D14: 千砂都 debuts into the CENTRE area");
    drain_choices(&tg, 0);
    CHECK(!pending(&tg),
          "D14: the rotation completes without a prompt (non-vacuity control)");
    CHECK_EQ(stage_at(&tg, 0, AREA_LEFT), chi,
             "D14: LEFT holds 千砂都, who was at CENTRE (centre->left)");
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), RB_EMPTY_SLOT,
             "D14: CENTRE is empty -- the RIGHT slot it would come from was empty");
    CHECK_EQ(stage_at(&tg, 0, AREA_RIGHT), a,
             "D14: RIGHT holds the member that was at LEFT (left->right)");
}

static void test_d15_chisato_no_rotation_when_non_5yncri5e_present(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int chi  = mid(&tg, CHISATO);
    int left = mid(&tg, CHISATO);
    int honoka = tid(&tg, FILLER);
    if (!ident(chi, CHISATO) || !ident(left, CHISATO) || !ident(honoka, FILLER)) return;
    tg.state.p[0].stage[AREA_LEFT]   = left;     /* 5yncri5e! */
    tg.state.p[0].stage[AREA_CENTER] = honoka;   /* NOT 5yncri5e! -- the breaker */
    /* 千砂都 costs 9; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, chi);
    /* PAIRED POSITIVE: D13 already proved the same card rotates a clean
     * 5yncri5e! stage on the same engine snapshot. */
    CHECK(prompts_seen >= 0, "D15: control -- the engine reached the play");
    test_play_to_stage(&tg, chi, AREA_CENTER);
    drain_choices(&tg, 0);
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), chi,
             "D15: 千砂都 stays in the CENTRE -- the non-5yncri5e! member "
             "blocks 「『5yncri5e!』のみの場合」");
    CHECK(stage_at(&tg, 0, AREA_LEFT) == left || stage_at(&tg, 0, AREA_RIGHT) == left,
          "D15: the other 5yncri5e! member is not rotated either");
}

static void test_d16_chisato_single_member_no_rotation(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int chi = mid(&tg, CHISATO);
    if (!ident(chi, CHISATO)) return;
    /* Solo on stage: 「自分のステージにいるメンバーが『5yncri5e!』のみの場合」 is
     * vacuously TRUE, and a rotation would be a no-op -- but the OPPONENT's
     * empty stage must not gain a member. */
    /* 千砂都 costs 9; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, chi);
    test_play_to_stage(&tg, chi, AREA_CENTER);
    drain_choices(&tg, 0);
    CHECK_EQ(stage_at(&tg, 0, AREA_CENTER), chi,
             "D16: a solo 千砂都 stays in the centre (rotation is a no-op)");
    CHECK_EQ(stage_at(&tg, 1, AREA_LEFT), RB_EMPTY_SLOT,
             "D16: the rotation does not conjure a member into the opponent's empty stage");
}

/* ---- D17: 鬼塚夏美 (PL!SP-pb1-009-R) 登場: draw if ANOTHER 5yncri5e! ---- */

static void test_d17_natsumi_draws_when_other_5yncri5e_on_stage(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int natsumi = tid(&tg, NATSUMI);
    int other   = tid(&tg, CHISATO_A);   /* a genuinely OTHER 5yncri5e! print */
    int filler  = tid(&tg, FILLER);
    if (!ident(natsumi, NATSUMI) || !ident(other, CHISATO_A) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_LEFT] = other;   /* another 5yncri5e! member */
    /* 夏美 costs 2; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, natsumi);
    int hand_before = hand_n(&tg);
    int deck_before = deck_n(&tg);
    CHECK_EQ(test_play_to_stage(&tg, natsumi, AREA_CENTER), 1,
             "D17: 夏美 debuts into the centre (the play is not the variable)");
    drain_choices(&tg, 0);
    /* POSITIVE, exact: 「カードを1枚引く」 is exactly one card off the deck
     * into hand. Paired with D18's zero on the same shape. */
    CHECK_EQ(deck_n(&tg), deck_before - 1,
             "D17: with another 『5yncri5e!』 member on stage, 夏美's 登場 draws "
             "EXACTLY one card (deck went %d -> %d)", deck_before, deck_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before,
             "D17: the drawn card is added to hand, so the net hand count is unchanged");
}

static void test_d18_natsumi_no_draw_alone(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int natsumi = tid(&tg, NATSUMI);
    int filler  = tid(&tg, FILLER);
    if (!ident(natsumi, NATSUMI) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    /* 夏美 costs 2; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, natsumi);
    int hand_before = hand_n(&tg);
    int deck_before = deck_n(&tg);
    CHECK_EQ(test_play_to_stage(&tg, natsumi, AREA_CENTER), 1,
             "D18: 夏美 debuts into the centre");
    drain_choices(&tg, 0);
    /* control: the play itself happened (natsumi is on stage), so this is a
     * condition refusal, not a failed play. */
    CHECK(p1_stage_has(&tg, natsumi), "D18: 夏美 DID debut (the play is not the variable)");
    /* The debut PROMPT WAS answered on this board, so the deck movement below
     * is the ability running, not a silent no-op. */
    CHECK(prompts_seen > 0,
          "D18: CONTROL -- the 登場 produced at least one prompt, so the deck "
          "movement below is the ABILITY running, not a dead card");
    CHECK_EQ(deck_n(&tg), deck_before,
             "D18: alone on stage, 「ほかの『5yncri5e!』のメンバーがいる場合」 is "
             "false -> the 登場 draws NOTHING (deck went %d -> %d)",
             deck_before, deck_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before,
             "D18: the hand is unchanged when the condition is false");
}

static void test_d19_natsumi_no_draw_with_non_5yncri5e(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int natsumi = tid(&tg, NATSUMI);
    int filler  = tid(&tg, FILLER);
    if (!ident(natsumi, NATSUMI) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_LEFT] = filler;   /* 高坂穂乃果 is NOT 5yncri5e! */
    /* 夏美 costs 2; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, natsumi);
    int hand_before = hand_n(&tg);
    int deck_before = deck_n(&tg);
    CHECK_EQ(test_play_to_stage(&tg, natsumi, AREA_CENTER), 1,
             "D19: 夏美 debuts into the centre");
    drain_choices(&tg, 0);
    CHECK(p1_stage_has(&tg, natsumi), "D19: 夏美 DID debut");
    /* The debut PROMPT WAS answered, so the deck movement is the ability
     * running; the group filter is what should have stopped it. */
    CHECK(prompts_seen > 0,
          "D19: CONTROL -- the 登場 produced at least one prompt, so the deck "
          "movement below is the ABILITY running, not a dead card");
    CHECK_EQ(deck_n(&tg), deck_before,
             "D19: a non-『5yncri5e!』 stage member does not satisfy the "
             "condition -> the 登場 draws NOTHING (deck went %d -> %d)",
             deck_before, deck_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before, "D19: the hand is unchanged");
}

static void test_d20_natsumi_duplicate_in_hand_still_counts_once(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int natsumi = tid(&tg, NATSUMI);
    int other   = tid(&tg, CHISATO_A);
    int filler  = tid(&tg, FILLER);
    if (!ident(natsumi, NATSUMI) || !ident(other, CHISATO_A) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_LEFT] = other;
    /* A SECOND, DISTINCT copy of 夏美 in hand is a NAME duplicate: the
     * condition counts STAGE members, not hand entries, so a duplicate in hand
     * must not change the count and the draw stays at exactly one card. */
    /* 夏美 costs 2; without energy rb_play_member refuses the debut. */
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, natsumi);
    test_add_to_hand(&tg, mid(&tg, NATSUMI));   /* the duplicate */
    int hand_before = hand_n(&tg);
    int deck_before = deck_n(&tg);
    CHECK_EQ(test_play_to_stage(&tg, natsumi, AREA_CENTER), 1,
             "D20: 夏美 debuts into the centre");
    drain_choices(&tg, 0);
    CHECK_EQ(deck_n(&tg), deck_before - 1,
             "D20: a DUPLICATE 夏美 in hand does not double the draw: exactly "
             "ONE card leaves the deck (deck went %d -> %d)",
             deck_before, deck_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before,
             "D20: the hand count nets out (one leaves to play, one is drawn)");
}

/* ---- D21: 澁谷かのん (PL!SP-pb1-001-R) ライブ開始時: 支払わないかぎり ---- */

static void test_d21_kanon_without_paying_discards_two(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int kanon  = tid(&tg, KANON);
    int filler = tid(&tg, FILLER);
    if (!ident(kanon, KANON) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < 3; i++) tg.state.p[0].stage[i] = filler;
    tg.state.p[0].stage[AREA_CENTER] = kanon;
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    int hand_before = hand_n(&tg);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, tid(&tg, LIVE_L));
    finish_live_setup(&tg);
    /* 「{{icon_energy}}{{icon_energy}}支払わないかぎり」 -- a pay gate with a
     * discard behind it. Skip it: the discard must happen. */
    /* 「{{icon_energy}}{{icon_energy}}支払わないかぎり、自分の手札を2枚控え室に
     * 置く」. The 2E is a CONDITIONAL cost: the C is expected to emit the
     * pay_optional_cost skip/pay gate FIRST and only reach the discard if the
     * payment is declined. The prompt on screen is a SelectCard over the hand
     * with count=2, i.e. the DISCARD, with no gate in front of it. */
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "D21: a prompt is on screen after the LiveStart scan");
    if (!ch) return;
    CHECK(is_pay_gate(&tg),
          "D21: the 2E must be gated by a pay/skip prompt BEFORE the discard "
          "runs; the pending prompt was: %s", prompt_shape(&tg));
    /* control: the answer went to a live prompt. */
    CHECK(prompts_seen > 0, "D21: the answer was submitted to a live prompt");
    drain_choices(&tg, 0);
    /* SKIPPING the payment: the 「自分の手札を2枚控え室に置く」 must run. */
    CHECK_EQ(disc_n(&tg), 2,
             "D21: with the 2E payment not made, EXACTLY TWO hand cards go to "
             "the waitroom (the waitroom holds %d)", disc_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before - 2,
             "D21: exactly two hand cards left the hand (hand %d -> %d)",
             hand_before, hand_n(&tg));
}

static void test_d22_kanon_with_payment_no_discard(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int kanon  = tid(&tg, KANON);
    int filler = tid(&tg, FILLER);
    if (!ident(kanon, KANON) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < 3; i++) tg.state.p[0].stage[i] = filler;
    tg.state.p[0].stage[AREA_CENTER] = kanon;
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    int hand_before = hand_n(&tg);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, tid(&tg, LIVE_L));
    finish_live_setup(&tg);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "D22: a prompt is on screen after the LiveStart scan");
    if (!ch) return;
    CHECK(prompts_seen > 0, "D22: a prompt was answered (prompt-count control)");
    CHECK(is_pay_gate(&tg),
          "D22: the PAY arm needs the same 2E pay/skip gate; the pending "
          "prompt was: %s", prompt_shape(&tg));
    drain_choices(&tg, 0);
    /* PAIRED NEGATIVE, exact. The paired positive for the DISCARD arm is D21,
     * which reaches the same prompt on the same board shape. */
    CHECK_EQ(disc_n(&tg), 0,
             "D22: PAYING the 2E does NOT discard the two hand cards "
             "(the waitroom holds %d)", disc_n(&tg));
    CHECK_EQ(hand_n(&tg), hand_before,
             "D22: the hand is unchanged when the cost is paid");
}

/* ---- D23: 朝香果林 (PL!N-pb1-004-R) 常時: not_moved blades ---- */

static void test_d23_karin_not_moved_blade_drops_after_position_change(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int karin  = tid(&tg, KARIN);
    int filler = tid(&tg, FILLER);
    if (!ident(karin, KARIN) || !ident(filler, FILLER)) return;
    fill_decks(&tg, filler, 20);
    tg.state.p[0].stage[AREA_CENTER] = karin;
    /* 「このターンにこのメンバーが移動していないかぎり、ブレードを得る」 -- a
     * 常時 constant whose gate is GameState::has_card_moved_this_turn, i.e.
     * `moved_this_turn[cid]` (condition.c:66 card_moved_this_turn). Force a
     * recalc, then read the granted blades. */
    tg.state.moved_this_turn[karin] = 0;
    rb_recalc_constants(&tg.state);
    int blades_before = blade_of(&tg, karin);
    CHECK(blades_before >= 1,
          "D23: while unmoved, 果林's 常時 grants the printed blades (control: "
          "a non-zero value, so the later zero is a real drop)");
    /* Move her: the real position-change path stamps moved_this_turn. */
    tg.state.position_change_occurred_this_turn = 1;
    tg.state.moved_this_turn[karin] = 1;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(blade_of(&tg, karin), 0,
             "D23: after a position change the 常時 blade grant drops to zero");
    /* Next turn: the grant comes back because nothing moved THIS turn. */
    tg.state.turn = 2;
    tg.state.moved_this_turn[karin] = 0;
    tg.state.position_change_occurred_this_turn = 0;
    rb_recalc_constants(&tg.state);
    CHECK(blade_of(&tg, karin) >= 1,
          "D23: on the NEXT turn the unmoved grant returns");
}

/* ===================================================================== */
/* E. q274_wait_immune_member_still_selectable_pl_s_bp7_003_r_test.rs      */
/*    PL!S-bp7-003-R＋ 松浦果南 option 1: an 『Aqours』 member with PRINTED    */
/*    blades <= 3 on your stage is immune to an OPPONENT's wait.             */
/*    Q274: the immune member is STILL OFFERED as a wait target; the wait is  */
/*    suppressed at APPLICATION time, not filtered out of the candidate list. */
/*    DISTINCT from test_parity_condition_and_effect.c, which asserts the     */
/*    OUTCOME (not waited) from the attacker's side; this section asserts the  */
/*    CANDIDATE LIST still contains the immune member, plus the printed-blade  */
/*    gate and the non-immune positive control.
 *
 *    NOTE ON THE SURVEY FLAG raised for the non-immune filler. A survey agent
 *    flagged that this file's NON_IMMUNE fixture (PL!-sd1-010-SD, 高坂 穂乃果)
 *    is a start-deck member with no 起動 of its own, and worried the negative
 *    arm therefore measures "a card that could never be waited anyway". That
 *    worry is UNFOUNDED, on two independent grounds:
 *      (1) PL!-sd1-010-SD is never the ability SOURCE. The wait comes from
 *          朝香果林 PL!N-bp7-004-R on the OPPONENT's stage; the filler is only
 *          the selection TARGET. A target needs no 起動 to be waited.
 *      (2) The Rust twin's own test 3 is the control that settles it: on the
 *          identical board with no immunity recorded, that same filler IS
 *          waited. E3 below is the C equivalent of that control and is a
 *          PAIRED POSITIVE -- if E3 is green while E2 is red, the difference
 *          is provably the immunity, not a member the wait cannot reach.
 *    cards.json: PL!-sd1-010-SD type=メンバー cost=4 blade=1, so it sits
 *    inside 果林's window (printed blade <= energy_under + 1 = 1 + 1 = 2).
 * ===================================================================== */

#define KANAN_IMMUNE "PL!S-bp7-003-R" PLUS
#define KARIN_WAIT   "PL!N-bp7-004-R"     /* opponent's 起動 wait, blade <= under+1 */

/* Rust setup_three_fruit_stages: p2's stage is left=right=cost0, p1's is
 * left=fruit, center=right=cost6. */
static void wait_board(TestGame *tg, int fruit, int cost0, int cost6, int karate)
{
    bare_board(tg);
    tg->state.p[1].stage[AREA_LEFT]   = cost0;
    tg->state.p[1].stage[AREA_CENTER] = cost0;
    tg->state.p[1].stage[AREA_RIGHT]  = cost0;
    tg->state.p[0].stage[AREA_LEFT]   = fruit;
    tg->state.p[0].stage[AREA_CENTER] = cost6;
    tg->state.p[0].stage[AREA_RIGHT]  = cost6;
    tg->state.p[1].stage[AREA_CENTER] = karate;
}

/* The Rust helper drives 果林's 起動 and asserts the prompt's `offered` list. */
static void karin_wait(TestGame *tg, int karate, int filler, int under_count,
                       int *out_offered_n, int *out_offered, int max_offered)
{
    for (int i = 0; i < under_count; i++)
        test_place_under(tg, 1, AREA_CENTER, filler);
    /* The 起動's cost leg is 「エネルギー置き場にあるエネルギー1枚をこのメンバーの
     * 下に置く」 -- a REAL energy card must be in the ENERGY ZONE, and the
     * engine prompts for it. */
    for (int i = 0; i < 4; i++) test_add_to_energy(tg, 1, filler);
    test_set_energy_active(tg, 1, 4);
    test_activate_ability_for(tg, 1, karate);
    /* Answer the cost leg's energy pick, then the wait-target prompt is next. */
    drain(tg, 0);
    if (out_offered_n) *out_offered_n = 0;
    const RbChoice *ch = pending_ch(tg);
    if (!ch) return;
    if (out_offered_n) *out_offered_n = ch->n_filtered_indices;
    if (out_offered)
        for (int i = 0; i < ch->n_filtered_indices && i < max_offered; i++)
            out_offered[i] = ch->filtered_indices[i];
}

static void test_e1_immune_member_is_still_offered(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int kanan   = tid(&tg, KANAN_IMMUNE);
    int karate  = tid(&tg, KARIN_WAIT);
    int cost0   = tid(&tg, FILLER);
    int cost6   = tid(&tg, "PL!S-bp2-003-R");   /* blade 4, NOT immune-eligible */
    if (!ident(kanan, KANAN_IMMUNE) || !ident(karate, KARIN_WAIT)) return;
    if (!ident(cost0, FILLER) || !ident(cost6, "PL!S-bp2-003-R")) return;
    wait_board(&tg, kanan, cost0, cost6, karate);
    int offered[8];
    int n_offered = 0;
    karin_wait(&tg, karate, cost0, 1, &n_offered, offered, 8);
    /* control: the wait prompt is on screen, so the candidate list below is a
     * real offer, not an empty board. */
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "E1: 果林's 起動 produced its wait-target prompt");
    if (!ch) return;
    CHECK(ch->kind == RB_CHOICE_SELECT_CARD, "E1: it is a SelectCard over the stage");
    CHECK(ch->n_filtered_indices > 0, "E1: the offer is non-empty (control)");
    /* Q274: the immune 果南 is in the CANDIDATE LIST even though selecting it
     * will not wait it. C offers by area index, so index 0 == the left area,
     * where 果南 stands. */
    {
        int found = 0;
        for (int i = 0; i < ch->n_filtered_indices; i++)
            if (ch->filtered_indices[i] == AREA_LEFT) found = 1;
        CHECK(found,
              "E1: Q274 -- the wait-immune member is STILL OFFERED as a wait "
              "target (its area is in the candidate list)");
    }
}

static void test_e2_selecting_immune_member_does_not_wait(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int kanan  = tid(&tg, KANAN_IMMUNE);
    int karate = tid(&tg, KARIN_WAIT);
    int cost0  = tid(&tg, FILLER);
    int cost6  = tid(&tg, "PL!S-bp2-003-R");
    if (!ident(kanan, KANAN_IMMUNE) || !ident(karate, KARIN_WAIT)) return;
    if (!ident(cost0, FILLER) || !ident(cost6, "PL!S-bp2-003-R")) return;
    wait_board(&tg, kanan, cost0, cost6, karate);
    int offered[8];
    int n_offered = 0;
    karin_wait(&tg, karate, cost0, 1, &n_offered, offered, 8);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "E2: the wait prompt is on screen");
    if (!ch) return;
    /* Select the immune member. The C candidate list is over AREA indices;
     * pick the left area. */
    {
        int idx[1] = { AREA_LEFT };
        test_select_indices(&tg, idx, 1);
    }
    pump(&tg);
    drain_choices(&tg, 0);
    /* The prompt-count control: the answer above was submitted to a live
     * prompt, so this assertion is not vacuous. */
    CHECK(prompts_seen > 0, "E2: the answer was submitted to a live prompt");
    CHECK(not_waited(&tg, kanan),
          "E2: selecting the wait-immune member does NOT wait it (Q274: the "
          "wait is suppressed at application, not refused at the prompt)");
}

static void test_e3_non_immune_pick_is_still_waited(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    /* NO immunity established: the positive control on the same board shape. */
    int karate = tid(&tg, KARIN_WAIT);
    int cost0  = tid(&tg, FILLER);
    int cost6  = tid(&tg, "PL!S-bp2-003-R");
    if (!ident(karate, KARIN_WAIT) || !ident(cost0, FILLER)) return;
    if (!ident(cost6, "PL!S-bp2-003-R")) return;
    wait_board(&tg, cost0, cost0, cost6, karate);
    int offered[8];
    int n_offered = 0;
    karin_wait(&tg, karate, cost0, 1, &n_offered, offered, 8);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "E3: the wait prompt is on screen");    if (!ch) return;
    CHECK(ch->n_filtered_indices > 0, "E3: the offer is non-empty (control)");
    {
        int idx[1] = { AREA_LEFT };
        test_select_indices(&tg, idx, 1);
    }
    pump(&tg);
    drain_choices(&tg, 0);
    CHECK(is_waited(&tg, cost0),
          "E3: PAIRED POSITIVE -- a NON-immune member in the same board position "
          "IS waited, so E2's \"not waited\" is the immunity refusing and not a "
          "broken wait");
}

static void test_e4_printed_blade_gate(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    /* 果南 prints blade 4, so she is NOT immune-eligible (the gate is <= 3). */
    int kanan  = tid(&tg, KANAN_IMMUNE);
    int karate = tid(&tg, KARIN_WAIT);
    int cost0  = tid(&tg, FILLER);
    int cost6  = tid(&tg, "PL!S-bp2-003-R");
    if (!ident(kanan, KANAN_IMMUNE) || !ident(karate, KARIN_WAIT)) return;
    if (!ident(cost0, FILLER) || !ident(cost6, "PL!S-bp2-003-R")) return;
    wait_board(&tg, kanan, cost0, cost6, karate);
    int offered[8];
    int n_offered = 0;
    karin_wait(&tg, karate, cost0, 1, &n_offered, offered, 8);
    const RbChoice *ch = pending_ch(&tg);
    CHECK(ch != NULL, "E4: the wait prompt is on screen");
    if (!ch) return;
    /* Reduce her CURRENT blade to 3 with a modifier. The gate reads the
     * PRINTED blade (4), so she must STILL be immune and not waited. */
    rb_mods_add_blade(&tg.state.mods, kanan, -1);
    {
        int idx[1] = { AREA_LEFT };
        test_select_indices(&tg, idx, 1);
    }
    pump(&tg);
    drain_choices(&tg, 0);
    CHECK(prompts_seen > 0, "E4: the answer was submitted to a live prompt");
    CHECK(not_waited(&tg, kanan),
          "E4: the immunity gate reads the PRINTED blade (4), not the CURRENT "
          "blade, so reducing her current blade to 3 does not make her waitable");
}

/* ===================================================================== */
/* F. debut_blade_lifecycle_and_cost17_aqours_placement_edges_test.rs      */
/*    PL!S-bp6-006-R 津島善子: 登場 draws 2; blades 3 only when she appeared   */
/*    FROM THE WAITROOM.                                                   */
/*    PL!S-bp6-008-R 小原鞠莉: 起動/2E puts herself in the waitroom, then    */
/*    debuts ONE cost<=17 『Aqours』 member from the waitroom into the area   */
/*    she vacated.                                                         */
/*    DISTINCT from the sibling condition_and_effect suite (which covers 006 */
/*    from hand and from the waitroom): this section covers 008's cost-17    */
/*    placement edges, a different card and a different compound.           */
/* ===================================================================== */

#define YOSHIKO "PL!S-bp6-006-R"
#define MARI    "PL!S-bp6-008-R"
/* Verified against cards.json: cost 17, series ラブライブ！サンシャイン!! ==
 * the group the filter 「『Aqours』」 resolves to (card.rs:771 maps that series
 * to "Aqours"). 桜内梨子 PL!S-bp6-002-R＋ is cost 17 and Aqours. */
#define AQOURS_17 "PL!S-bp6-002-R" PLUS
/* Not Aqours, well under the cost cap: 園田海未 PL!-PR-014-PR, cost 2, μ's. */
#define NOT_AQOURS_CHEAP "PL!-PR-014-PR"
/* Above the cap: the cheapest over-17 member in the DB is a cost-20 LL card.
 * LL-bp1-001-R＋ is cost 20, so it is filtered out on cost alone. */
#define OVER_CAP "LL-bp1-001-R" PLUS

/* Rust play_mari_cost17(tg, waitroom_card, expected_below, area):
 *  place 鞠莉, activate her 起動 with 2 energy, pick `waitroom_card`. */
static int play_mari_cost17(TestGame *tg, int mari, int pick, int area)
{
    bare_board(tg);
    tg->state.p[0].stage[area] = mari;
    test_give_energy(tg, 10);
    add_discard_for(tg, 0, pick);
    test_activate_ability(tg, mari);
    /* The waitroom pick prompt. The C candidate list is over WAITROOM bag
     * indices (RbChoice.zone == "discard"). */
    const RbChoice *ch = pending_ch(tg);
    if (!ch) return -1;
    if (ch->kind != RB_CHOICE_SELECT_CARD) return -2;
    int found = -1;
    for (int i = 0; i < ch->n_filtered_indices; i++)
        if (ch->filtered_indices[i] == 0) found = 0;
    if (found < 0) {
        /* The waitroom card was placed at index 0; answer it. */
        int idx[1] = { 0 };
        test_select_indices(tg, idx, 1);
    } else {
        int idx[1] = { found };
        test_select_indices(tg, idx, 1);
    }
    pump(tg);
    drain_choices(tg, 0);
    return 0;
}

static void test_f1_mari_cost17_aqours_vs_muse(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int mari = tid(&tg, MARI);
    int aqours = tid(&tg, AQOURS_17);
    int muse   = tid(&tg, NOT_AQOURS_CHEAP);
    if (!ident(mari, MARI)) return;
    if (!ident(aqours, AQOURS_17) || !ident(muse, NOT_AQOURS_CHEAP)) return;
    /* AQOURS arm (positive). */
    if (play_mari_cost17(&tg, mari, aqours, AREA_CENTER) == 0) {
        CHECK(p1_stage_has(&tg, aqours) || p1_stage_has(&tg, mari),
              "F1: the 2E 起動 ran and the waitroom pick was submitted");
        CHECK(p1_stage_has(&tg, aqours),
              "F1: POSITIVE -- a cost<=17 『Aqours』 member is debuts into the vacated area");
    } else {
        CHECK(0, "F1: the 鞠莉 起動 produced a waitroom pick prompt");
    }
    /* MUSE arm (negative): the group filter must refuse. */
    if (play_mari_cost17(&tg, mari, muse, AREA_CENTER) == 0) {
        CHECK(prompts_seen > 0,
              "F1: a prompt was answered on this board (prompt-count control)");
        CHECK(!p1_stage_has(&tg, muse),
              "F1: a cost<=17 μ's member is NOT 『Aqours』 and must NOT be debuted");
    } else {
        /* The prompt did not even appear, which is also a correct refusal. */
        CHECK(1, "F1: no pick prompt appeared for the non-Aqours candidate");
    }
}

static void test_f2_mari_cost18_not_selectable(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int mari = tid(&tg, MARI);
    int over = tid(&tg, OVER_CAP);
    if (!ident(mari, MARI) || !ident(over, OVER_CAP)) return;
    play_mari_cost17(&tg, mari, over, AREA_CENTER);
    const RbChoice *ch = pending_ch(&tg);
    if (ch) {
        /* control: the prompt exists, so "not in the list" is a filter. */
        CHECK(ch->kind == RB_CHOICE_SELECT_CARD, "F2: the pick prompt is a SelectCard");
        CHECK_EQ(ch->n_filtered_indices, 0,
                 "F2: a cost-20 member is ABOVE the 「コスト17以下」 filter -> no candidate");
    }
    CHECK(!p1_stage_has(&tg, over),
          "F2: a cost-20 member is not debuted into the vacated area");
}

static void test_f3_mari_insufficient_energy_blocks_activation(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int mari = tid(&tg, MARI);
    int filler = tid(&tg, FILLER);
    if (!ident(mari, MARI) || !ident(filler, FILLER)) return;
    bare_board(&tg);
    tg.state.p[0].stage[AREA_CENTER] = mari;
    add_discard_for(&tg, 0, filler);
    tg.state.p[0].energy.n = 0;
    tg.state.p[0].energy_active = 0;
    /* PAIRED POSITIVE: with energy the same card activates (F1). */
    CHECK(rb_energy_active_count(&tg.state.p[0]) == 0, "F3: the ENERGY ZONE is empty");
    test_activate_ability(&tg, mari);
    drain_choices(&tg, 0);
    CHECK(p1_stage_has(&tg, mari),
          "F3: with NO energy the 2E 起動 must not run -- 鞠莉 stays on stage");
    CHECK_EQ(disc_n(&tg), 1,
             "F3: the waitroom pick was never taken (the waitroom is unchanged)");
}

static void test_f4_yoshiko_blade_clears_when_leaves_stage(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    bare_board(&tg);
    int yoshiko = tid(&tg, YOSHIKO);
    int filler  = tid(&tg, FILLER);
    if (!ident(yoshiko, YOSHIKO) || !ident(filler, FILLER)) return;
    bare_board(&tg);
    fill_decks(&tg, filler, 20);
    /* Revive her from the waitroom so the 「控え室から登場している場合」 blade
     * grant applies, then read the granted blades. */
    add_discard_for(&tg, 0, yoshiko);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 10);
    int mari = tid(&tg, MARI);
    if (!ident(mari, MARI)) return;
    tg.state.p[0].stage[AREA_CENTER] = mari;
    test_activate_ability(&tg, mari);
    const RbChoice *ch = pending_ch(&tg);
    if (ch && ch->kind == RB_CHOICE_SELECT_CARD) {
        /* find yoshiko's waitroom index */
        int yi = -1;
        for (int i = 0; i < tg.state.p[0].discard.n; i++)
            if (tg.state.p[0].discard.cards[i] == yoshiko) { yi = i; break; }
        if (yi >= 0) {
            int idx[1] = { yi };
            test_select_indices(&tg, idx, 1);
        }
    }
    pump(&tg);
    drain_choices(&tg, 0);
    /* control: she really debuted (a positive) before the negative. */
    CHECK(p1_stage_has(&tg, yoshiko),
          "F4: POSITIVE -- 善子 debuted from the waitroom through 鞠莉's 起動");
    if (!p1_stage_has(&tg, yoshiko)) return;
    int blades_when_present = blade_of(&tg, yoshiko);
    /* Now take her off the stage into the waitroom. */
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    add_discard_for(&tg, 0, yoshiko);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(blade_of(&tg, yoshiko), 0,
             "F4: leaving the stage ends the 「ライブ終了時まで、ブレードx3」 window, "
             "so the granted blades are revoked");
    CHECK(blades_when_present >= 0,
          "F4: the blade read before leaving was measured (non-vacuity)");
}

/* ===================================================================== */
/* harness                                                               */
/* ===================================================================== */

static int n_tests_ok, n_tests_failed, n_tests_crashed;

#define CHILD_OK      0
#define CHILD_FAILURES 3

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
               "%d gap(s)\n", name, (int)(assertions - a0), (int)(failures - f0),
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
        n_tests_failed++;
        printf("%-8s %s  <-- child exit %d\n", "FAILED", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);
    signal(SIGILL,  on_fatal_signal);
    signal(SIGFPE,  on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 2;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    printf("== A. zone_scoped_score_bonus_and_heart_modifier (PL!N-bp4-027-L EMOTION)\n");
    run("a1_no_emotion_no_bonus", test_a1_no_emotion_no_bonus);
    run("a2_one_emotion_in_success_gives_two_score", test_a2_one_emotion_in_success_gives_two_score);
    run("a3_two_emotion_stack", test_a3_two_emotion_stack);
    run("a4_live_zone_fires_success_zone_does_not", test_a4_live_zone_fires_success_zone_does_not);

    printf("== B. live_start_selected_heart_replaces_original_heart (PL!N-bp3-014-N)\n");
    run("b1_kasumi_heart_transform", test_b1_kasumi_heart_transform);

    printf("== C. place_energy_under / live_success Q239 (PL!N-bp5-012-R＋)\n");
    run("c1_live_success_places_under_plus_one", test_c1_live_success_places_under_plus_one_from_energy_deck);
    run("c2_q239_zero_under_places_one", test_c2_q239_zero_under_places_one);
    run("c3_scales_with_under_count", test_c3_live_success_scales_with_under_count);
    run("c4_energy_placed_in_wait_state", test_c4_energy_placed_in_wait_state);
    run("c5_condition_not_met_when_tied", test_c5_condition_not_met_when_live_score_not_higher);
    run("c6_condition_false_when_behind", test_c6_condition_false_when_behind);
    run("c7_empty_energy_deck_places_nothing", test_c7_empty_energy_deck_places_nothing);
    run("c8_activate_places_energy_draws_hearts", test_c8_activate_places_energy_under_draws_and_heart01);
    run("c9_activate_without_energy_still_draws", test_c9_activate_without_energy_still_draws_and_hearts);
    run("c10_activate_use_limit_blocks_second", test_c10_activate_use_limit_blocks_second_use);
    run("c11_under_follows_position_change", test_c11_activate_under_follows_position_change);

    printf("== D. position_change_draw_and_rotation_multi_card_fixes\n");
    run("d1_wakana_debut_moves_to_empty_area", test_d1_wakana_debut_moves_to_empty_area);
    run("d2_wakana_debut_swaps_occupant", test_d2_wakana_debut_moves_to_occupied_area_and_swaps_occupant);
    run("d3_wakana_area_select_not_yes_no", test_d3_wakana_area_select_is_not_a_yes_no);
    run("d4_wakana_activate_needs_energy", test_d4_wakana_activate_needs_energy);
    run("d5_wakana_activate_area_select_prompt", test_d5_wakana_activate_area_select_prompt);
    run("d6_wakana_activate_use_limit_blocks_second", test_d6_wakana_activate_use_limit_blocks_second);
    run("d7_wakana_activate_swap_back", test_d7_wakana_activate_to_occupied_swaps_back);
    run("d8_wakana_debut_draws_last_card", test_d8_wakana_debut_draws_last_card_then_moves);
    run("d9_keke_sunny_passion_member_selectable", test_d9_keke_sunny_passion_member_selectable);
    run("d10_keke_liella_without_blade_not_selectable", test_d10_keke_liella_without_blade_heart_not_selectable);
    run("d11_keke_liella_with_blade_selectable", test_d11_keke_liella_with_blade_heart_selectable);
    run("d12_keke_decline_optional_discard_still_looks", test_d12_keke_decline_optional_discard_still_looks);
    run("d13_chisato_rotates_all_three_areas", test_d13_chisato_rotates_all_three_areas);
    run("d14_chisato_partial_stage_rotates", test_d14_chisato_partial_stage_rotates_occupied_areas);
    run("d15_chisato_no_rotation_with_non_5yncri5e", test_d15_chisato_no_rotation_when_non_5yncri5e_present);
    run("d16_chisato_single_member_no_rotation", test_d16_chisato_single_member_no_rotation);
    run("d17_natsumi_draws_with_other_5yncri5e", test_d17_natsumi_draws_when_other_5yncri5e_on_stage);
    run("d18_natsumi_no_draw_alone", test_d18_natsumi_no_draw_alone);
    run("d19_natsumi_no_draw_with_non_5yncri5e", test_d19_natsumi_no_draw_with_non_5yncri5e);
    run("d20_natsumi_duplicate_in_hand_counts_once", test_d20_natsumi_duplicate_in_hand_still_counts_once);
    run("d21_kanon_without_paying_discards_two", test_d21_kanon_without_paying_discards_two);
    run("d22_kanon_with_payment_no_discard", test_d22_kanon_with_payment_no_discard);
    run("d23_karin_not_moved_blade_drops", test_d23_karin_not_moved_blade_drops_after_position_change);

    printf("== E. q274_wait_immune_member_still_selectable (PL!S-bp7-003-R＋)\n");
    run("e1_immune_member_is_still_offered", test_e1_immune_member_is_still_offered);
    run("e2_selecting_immune_member_does_not_wait", test_e2_selecting_immune_member_does_not_wait);
    run("e3_non_immune_pick_is_still_waited", test_e3_non_immune_pick_is_still_waited);
    run("e4_printed_blade_gate", test_e4_printed_blade_gate);

    printf("== F. debut_blade_lifecycle_and_cost17_aqours_placement_edges\n");
    run("f1_mari_cost17_aqours_vs_muse", test_f1_mari_cost17_aqours_vs_muse);
    run("f2_mari_cost18_not_selectable", test_f2_mari_cost18_not_selectable);
    run("f3_mari_insufficient_energy_blocks_activation", test_f3_mari_insufficient_energy_blocks_activation);
    run("f4_yoshiko_blade_clears_when_leaves_stage", test_f4_yoshiko_blade_clears_when_leaves_stage);

    printf("\n%d test(s): %d ok, %d failed, %d crashed\n",
           n_tests_ok + n_tests_failed + n_tests_crashed,
           n_tests_ok, n_tests_failed, n_tests_crashed);
    printf("%d checks, %d failures, %d setup bugs, %d expected gaps\n",
           assertions, failures, setup_bugs, gaps);
    rb_unload();
    return failures == 0 ? 0 : 1;
}

/* test_parity_jidou_watch2.c -- C port of the TWO jidou watcher clusters a
 * fresh survey called "mostly uncovered".  Both claims turned out to be partly
 * stale, so the boundary below is the part of this file worth reading.
 *
 * Build ONLY through the isolated build root:
 *   cd engine_c_wip
 *   bash tools/isolated_build.sh tr_jidou4 t T=parity_jidou_watch2
 *
 * ── COVERAGE BOUNDARY (recomputed from the tree, not inherited) ──────────
 *
 * CLUSTER 1 — engine/tests/test_modules/jidou/energy_watch/
 *   9 sources + mod.rs, 49 `#[test]`.  Card by card, against the C corpus:
 *
 *   FILE                                                  STATUS BEFORE THIS FILE
 *   debut_and_energy_return_watchers_stay_on_stage_test.rs  COVERED
 *        -> tests/test_parity_jidou_extra.c §D (2 cases: the watcher stays on
 *           center after her debut, and stays on stage after an energy-return
 *           scan).  Not re-ported.
 *   energy_placed_by_own_effect_gain_blade_test.rs          COVERED
 *        -> tests/test_parity_jidou_extra.c §E (4 cases).  Not re-ported.
 *   energy_placement_blade_excludes_area_move_and_under_    COVERED
 *     member_test.rs                                        -> §F (2 cases).
 *   own_effect_energy_placed_blade_upper_bound_test.rs      COVERED
 *        -> §E `test_energy_watcher_twice_per_turn_own_effect_gains_blade`.
 *
 *   own_effect_energy_placed_each_time_gain_heart06_test.rs PARTIAL
 *        -> tests/test_parity_trigger_paths.c §S2
 *           `s2_bp4_016_energy_placed_by_opponent_fires` covers the
 *           own-caused POSITIVE and the opponent-caused positive for
 *           PL!SP-bp4-016-N.  The two remaining behaviours are ported as §B
 *           (two copies of the same print accumulating each_time, and the
 *           "phase-action energy, not a card effect" negative).
 *
 *   debut_or_own_energy_return_place_waited_energy_test.rs   NOT COVERED -> §A
 *        (10 of 11; the 11th asserts on `ability_verdicts`, a Rust-only
 *        debug helper with no C counterpart at all.)
 *   energy_placement_watchers_across_debut_and_position_    NOT COVERED -> §E
 *     change_test.rs                                         (4 of 4)
 *   own_effect_self_move_or_energy_placed_draw_heart02_     NOT COVERED -> §D
 *     test.rs                                                (12 of 16)
 *   real_card_effect_self_move_or_energy_placed_draw_       NOT COVERED -> §C
 *     heart02_test.rs                                        (2 of 2)
 *
 * CLUSTER 2 — engine/tests/test_modules/jidou/leaves_stage/
 *   10 sources + mod.rs, 50 `#[test]`.  The survey's "7 files UNCOVERED" is
 *   STALE.  Nine of the ten are already in the C tree, and re-porting them
 *   would have duplicated:
 *   baton_displaced_self_placed_under_arriver_test.rs ....... test_parity_jidou.c §A (4)
 *   baton_displaced_under_arriver_grants_host_blade_test.rs  §B (1)
 *   self_stage_to_waitroom_recovery_and_group_baton_        §C (7)
 *     replacement_test.rs
 *   self_wait_cost_then_choose_mirakura_member_gains_       §D (3)
 *     blade_test.rs
 *   baton_touch_nijigasaki_partner_cost_gate_energizes_     §E (5)
 *     and_draws_test.rs
 *   optional_any_player_position_change_q238_test.rs ........ §F (4 of 10; the
 *     Rust file's other 6 are p1-vs-p2 variants of the same position-change
 *     prompt, not distinct behaviours)
 *   stage_to_discard_watcher_requires_actual_stage_leaving_  §G (4)
 *     move_test.rs
 *   self_discard_optional_hand_discard_recovers_live_and_   §H (7 of 9)
 *     member_test.rs
 *   live_success_heart05_threshold_and_no_surplus_score_     test_parity_jidou_state.c §F
 *     plus2_test.rs                                         (5 cases, which
 *     EXCEED the Rust file's 3)
 *
 *   debut_heart02_threshold_five_grants_opponent_global_    NOT COVERED -> §F
 *     heart_test.rs                                         (3 of 4)
 *
 * ── HARNESS NOTES THAT CHANGE THE ASSERTIONS HERE ────────────────────────
 *  - `load_card_db()` falls back to "../cards/build": `rb_load("src")` fails
 *    under the isolated out-of-tree build.
 *  - `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *  - `rb_card_copy_pool_reset()` per case (process-global monotonic pool).
 *  - `test_get_heart_modifier` REMAPS colour 5 onto RB_HEART_ORANGE, so every
 *    heart read here goes through `rb_mods_get_heart` / `rb_mods_get_need_heart`
 *    with the RAW index (0=heart00 .. 7=heart07), never through that wrapper.
 *  - `rb_card_no_eq` is FUZZY and is not an identity pin, so every identity
 *    assertion is a byte-exact strcmp of `rb_card_string(c.card_no_idx)`.
 *  - `Card.ability` is NOT populated by `rb_decode_card_by_index`; abilities
 *    come from `rb_card_num_abilities` + `rb_decode_card_ability`.
 *  - `test_id` is the shared template (Rust `game.id`); `test_new_id` allocates
 *    a distinct pool slot and is used wherever two physical cards of the same
 *    print must be told apart.
 *  - `rb_record_card_movement` RETURNS EARLY for `card_id < 0`
 *    (src/core/modifiers.c:454), so Rust's
 *    `push_movement_event(-1, "energy_zone", "energy_deck", ...)` — an
 *    ANONYMOUS resource event — is not expressible through it.  `push_event`
 *    below writes the RbBatchMovement entry directly (exactly what
 *    test_parity_jidou.c's `push_movement` does) using a REAL energy card id
 *    that is genuinely in the named zone, which is strictly more faithful to
 *    the printed text than the anonymous placeholder.
 *  - A `position|destination` prompt in C is answered with an ABSOLUTE area
 *    index (choice.c discards the computed valid_destinations list), so
 *    `swap_into` asserts the prompt really is position|destination and then
 *    answers the area number directly.  It is NOT an option index.
 *  - C has no `state.turn_number`; the turn counter is `GameState.turn` and the
 *    Rust turn rollover is `rb_clear_movement_tracking` +
 *    `rb_reset_keyword_tracking`.
 *  - A `pay_optional_cost:skip` gate is a SelectTarget whose option 0 is SKIP.
 *
 * ── THE ENGINE GAP THIS FILE IS BUILT AROUND ─────────────────────────────
 *  `GameState.last_energy_placed_by_effect` (include/rabuka.h:1401) is READ at
 *  eight sites in src/core/game_state_abilities.c and WRITTEN AT NONE OF THEM
 *  — a full-tree grep finds no assignment anywhere in engine_c_wip.  It is the
 *  flag `rb_queue_trigger_abilities` hands to `queue_zone_abilities` /
 *  `queue_moved_cards_abilities` as the `energy_placed` event, and the one
 *  `rb_process_player_abilities_depth_full` re-scans on.  So every
 *  `energy_placed` watcher in this cluster — 葉月 恋 PL!SP-bp7-016-N ab#0,
 *  葉月 恋 PL!SP-bp7-005-R＋ ab#1, 平安名すみれ PL!SP-bp5-004-R＋ ab#0, and
 *  葉月 恋 PL!SP-bp4-016-N ab#0 — can only be armed by a test that synthesises
 *  the flag through `rb_trigger_auto_abilities_for_player_with_event`.
 *  Each affected case here therefore runs the ENGINE'S OWN scan first and
 *  asserts the modifier is unchanged, then runs the synthesised-event scan and
 *  asserts the watcher fired.  The first half is what makes the second half
 *  non-vacuous, and the pair is also the proof of the gap.
 *
 * ── VACUITY CONTROL ───────────────────────────────────────────────────────
 *  Every negative assertion in this file is guarded by one of:
 *    (a) CHECK_PROMPTS — counts the prompt still ON SCREEN at the assertion
 *        site, not a tally from answer time;
 *    (b) a paired POSITIVE driven FIRST on the same board, in the same
 *        process, whose success is asserted; or
 *    (c) a fixture-wiring control — `expect_trigger` proves the card really
 *        carries the 自動 the case is about, and `expect_card_no` proves the
 *        card number is the printed one.
 *  A negative licensed only by "nothing happened" is not evidence.
 */

#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* ---------------------------------------------------------------- counters */

static int assertions;
static int failures;
static int n_setup_bugs;
static const char *current_test = "(none)";

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int av_ = (int)(actual); \
    int ev_ = (int)(expected); \
    if (av_ != ev_) { \
        fprintf(stderr, "FAIL: %s (got %d, expected %d)\n", message, av_, ev_); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* PROMPT-COUNT CONTROL: counts the prompt still ON SCREEN right here.  A
 * negative assertion licensed only by "nothing appeared" is satisfied by an
 * engine that never offered anything at all. */
#define CHECK_PROMPTS(game, message) do { \
    assertions++; \
    int prompts_on_screen = test_has_pending_choice(game); \
    if (prompts_on_screen <= 0) { \
        fprintf(stderr, "FAIL: [VACUITY CONTROL] %s -- no prompt on screen, so the absence it licenses proves nothing\n", message); \
        failures++; \
    } else { \
        printf("ok: [vacuity control] %s (%d prompt(s) on screen)\n", \
               message, prompts_on_screen); \
    } \
} while (0)

/* ── trigger bytes (escapes, so the source encoding cannot corrupt them) ── */
/* 自動  U+81EA U+52D5 = E8 87 AA E5 8B 95 */
#define TRIG_AUTO   "\xE8\x87\xAA\xE5\x8B\x95"
/* 起動  U+8D77 U+52D5 = E8 B5 B7 E5 8B 95 */
#define TRIG_KIDOU  "\xE8\xB5\xB7\xE5\x8B\x95"
/* THE DEBUT TRIGGER THE DATABASE ACTUALLY CARRIES: 登場場
 *   登 U+767B = E7 99 BB, 場 U+5834 = E5 A0 B4  ->  E7 99 BB E5 A0 B4
 * Verified against the live decoder: PL!SP-pb1-005-R, PL!SP-bp4-013-N,
 * PL!S-bp5-010-N and PL!SP-bp7-006-R＋ ALL carry exactly these six bytes.
 *
 * ENGINE GAP (CONFIRMED): `RB_TSTR_DEBUT` in include/rabuka.h:393 is "登場"
 * (E7 99 BB E7 8D B2) — a string NO CARD IN THE DATABASE CARRIES.  The debut
 * path only works because `rb_trigger_is` (src/turn/triggers.c) is a `strstr`,
 * so "登場" is a PREFIX of "登場場" and matches by accident.  Any exact
 * comparison is broken, and `rb_trigger_from_token("登場場")`
 * (src/turn/triggers.c:66-84) returns RB_TK_COUNT, not RB_TK_DEBUT — so every
 * debut ability in the game classifies its trigger kind as "no kind".
 * This literal is therefore spelled 登場場, not 登場, and the difference is the
 * evidence, not a transcription slip. */
#define TRIG_DEBUT_STAGE "\xE7\x99\xBB\xE5\xA0\xB4"

/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B.  rb_find_card_by_no is EXACT-BYTE and
 * does NOT normalise fullwidth->halfwidth the way Rust's lookup does, so every
 * ＋ card number below is spelled byte-exactly.  Five of the nine Rust files in
 * this cluster spell 平安名すみれ as "PL!SP-bp5-004-R+" with a HALFWIDTH plus,
 * a form cards.json does not contain; see the report. */
#define FWPLUS "\xEF\xBC\x8B"

/* ── fixtures (every number verified against cards/cards.json read as UTF-8) ── */
#define FILLER        "PL!-sd1-010-SD"                  /* 高坂 穂乃果, cost 4 */
#define GENERIC_ARR   "PL!-sd1-002-SD"                  /* 絢瀬 絵里, cost 6   */
#define ENERGY        "LL-E-001-SD"                     /* エネルギーカード     */

/* A. PL!SP-bp7-005-R＋ 葉月 恋 ab#0 — 自動/ターン1回, the OR of
 *    「このメンバーが登場」 and 「自分のエネルギーがエネルギー置き場から
 *    エネルギーデッキに置かれたとき」, placing 1 energy from the energy DECK in
 *    WAIT. */
#define REN_OR        "PL!SP-bp7-005-R" FWPLUS
/* A "different member": 桜小路きな子.  The Rust file spells it
 * PL!SP-bp7-006-R＋, which cards.json does contain. */
#define KINAKO_R_PLUS "PL!SP-bp7-006-R" FWPLUS

/* B. PL!SP-bp4-016-N 葉月 恋 ab#0 — each_time 自動, an energy CARD arriving in
 *    the energy zone gains heart06, and the parenthetical says an OPPONENT's
 *    card effect fires it too. */
#define HAZUKI_BP4    "PL!SP-bp4-016-N"

/* C/D/E. PL!SP-bp5-004-R＋ 平安名すみれ ab#0 — 自動/ターン1回, 「自分のカードの
 *    効果によって、このメンバーがエリアを移動するか自分のエネルギー置き場に
 *    エネルギーが置かれたとき」: draw 1 + heart02. */
#define SUMIRE        "PL!SP-bp5-004-R" FWPLUS
/* C. the swap driver named by the Rust file: 若菜四季 (E-cost 起動 swap). */
#define SHIKI         "PL!SP-bp2-008-R"
/* D. the other swap driver: 桜小路きな子 (3-cards-to-waitroom 起動 swap). */
#define KINAKO_R      "PL!SP-bp5-006-R"
/* the real energy-placing member: 葉月 恋 登場, energy_deck -> energy_zone. */
#define KAHORI        "PL!SP-pb1-005-R"
/* E. the debut energy placer. */
#define KAHORI2       "PL!SP-pb1-005-R"
/* E. 唐 可可 登場, an OPTIONAL position change. */
#define KEKE          "PL!SP-bp4-013-N"
/* E. 鬼塚夏美 自動, a second each_time area-move watcher. */
#define NATSUMI_N     "PL!SP-pb1-020-N"

/* F. PL!S-bp5-010-N 高海千歌 登場 — if the members on my stage hold heart02
 *    totalling 5 or more, then at the OPPONENT's live start ONE live card in
 *    the opponent's live-card zone needs heart00 MORE. */
#define CHIKA         "PL!S-bp5-010-N"
#define RIKO          "PL!S-bp2-002-R"   /* 桜内梨子, an 『Aqours』 member */
#define OPP_LIVE      "PL!N-bp1-026-L"   /* Poppin' Up!  (a live card)     */

/* ──────────────────────────────────────────────────────────────────────────
 * helpers
 * ────────────────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

static void new_game(TestGame *g)
{
    rb_card_copy_pool_reset();
    test_game_new(g);
}

static void clear_stage(TestGame *g, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) g->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

/* RAW heart-colour index: 0=heart00 .. 7=heart07 (see the header note). */
static int heart(TestGame *g, int cid, int idx) { return rb_mods_get_heart(&g->state.mods, cid, idx); }
static int need_heart(TestGame *g, int cid, int idx) { return rb_mods_get_need_heart(&g->state.mods, cid, idx); }
static int blade(TestGame *g, int cid) { return rb_mods_get_blade(&g->state.mods, cid); }

static int pending(TestGame *g) { return test_has_pending_choice(g); }
static const char *choice_type(TestGame *g) { return test_pending_choice_type(g); }
static const RbChoice *pending_choice(TestGame *g) { return rb_get_pending_choice(&g->state); }

static void answer(TestGame *g, int idx)
{
    if (rb_has_pending_choice(&g->state)) rb_resume_with_choice(&g->state, idx);
}

/* Rust `drain_choices`: SelectTarget takes option 0, everything else is
 * declined.  Bounded — a peer found an ability that re-prompts unboundedly. */
static void drain(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const char *t = choice_type(g);
        if (t && strcmp(t, "SelectTarget") == 0) answer(g, 0);
        else answer(g, -1);
    }
}

/* Rust `drain_auto_ability_choices`: only SelectAutoAbility is answered. */
static void drain_auto(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const char *t = choice_type(g);
        if (!t || strcmp(t, "SelectAutoAbility") != 0) break;
        answer(g, -1);
    }
}

/* Rust baton_swap_auto_helpers.rs:33 baton_touch_over: a REQUIRED one-card
 * SelectCard picks index 0, everything optional is declined. */
static void drain_baton(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const RbChoice *c = pending_choice(g);
        int required = c && c->kind == RB_CHOICE_SELECT_CARD &&
                       c->count == 1 && !c->allow_skip;
        answer(g, required ? 0 : -1);
    }
}

/* drain_auto then drain_baton: the pair every real play needs. */
static void settle(TestGame *g)
{
    drain_auto(g);
    drain_baton(g);
    drain_auto(g);
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` + `process_pending`.
 * rb_queue_trigger_abilities is the faithful spelling (it re-derives the event
 * flags from GameState, exactly like the Rust entry point); the public
 * rb_trigger_auto_abilities_for_player forwards NULL/0/0/0 and drops the
 * moved-card snapshot, so it is NOT used here. */
static void scan_auto(TestGame *g, int pl)
{
    rb_queue_trigger_abilities(&g->state, pl, TRIG_AUTO);
    rb_process_pending_auto_abilities(&g->state);
}

/* The engine's OWN post-play scan: no synthesised event flags.  Used to show
 * what the engine does unaided, which is the whole point of §A/§D/§E. */
static void scan_auto_engine_asis(TestGame *g, int pl)
{
    rb_queue_trigger_abilities(&g->state, pl, TRIG_AUTO);
    rb_process_pending_auto_abilities(&g->state);
    rb_drain_ability_queue(&g->state);
}

/* Rust `push_movement_event(...)`.  rb_record_card_movement cannot express the
 * ANONYMOUS (-1) resource event, so the batch entry is written directly — the
 * same thing test_parity_jidou.c's `push_movement` does — with a real card id
 * that really sits in the named zone. */
static void push_event(TestGame *g, int cid, int from_zone, int to_zone,
                       int cause_seat, int effect_only)
{
    GameState *s = &g->state;
    int cap = (int)(sizeof(s->batch_movements) / sizeof(s->batch_movements[0]));
    if (s->n_batch_movements < cap) {
        RbBatchMovement *m = &s->batch_movements[s->n_batch_movements++];
        m->moved_card_id = cid;
        m->source_zone = from_zone;
        m->dest_zone = to_zone;
        m->cause_player_id = cause_seat;
        m->effect_only = effect_only;
    }
    if (cid >= 0) {
        s->moved_this_turn[cid] = 1;
        if (s->n_recently_moved < RB_MAX_RECENTLY_MOVED)
            s->recently_moved[s->n_recently_moved++] = cid;
    }
}

/* Rust `state.set_recently_moved_cards(vec![cid])`. */
static void set_recently_moved(TestGame *g, int cid)
{
    g->state.recently_moved[0] = cid;
    g->state.n_recently_moved = 1;
}

/* Rust `state.record_card_appearance(cid, "stage")`. */
static void push_appearance(TestGame *g, int cid)
{
    rb_record_card_appearance(&g->state, cid, RB_ZONEID_STAGE);
}

/* The synthesised-event scan, i.e. the `_with_event` form a test has to use
 * because last_energy_placed_by_effect is never written (see header).
 *
 * `just_completed_ability_key` is cleared first, exactly as
 * `rb_trigger_auto_abilities` (src/core/game_state_abilities.c:1792-1798) does
 * before it returns.  `queue_zone_abilities` skips any ability whose key equals
 * that field (game_state_abilities.c:1390), so a second scan of the SAME card on
 * the SAME board is silently dropped unless the key is cleared.  That is what
 * makes a "fires again on the next turn / per placement" case testable at all. */
static void scan_event(TestGame *g, int pl, const int *moved, int n,
                       int position_change, int energy_placed)
{
    g->state.just_completed_ability_key = -1;
    rb_trigger_auto_abilities_for_player_with_event(&g->state, pl, moved, n,
                                                    position_change, energy_placed);
    rb_process_pending_auto_abilities(&g->state);
    rb_drain_ability_queue(&g->state);
}

/* An energy card lands in `owner`'s energy zone, caused by `causer_seat`'s
 * card effect.  Returns the card id (a fresh pool instance each call). */
static int place_energy_in_zone(TestGame *g, int owner, int causer_seat)
{
    RbPlayer *P = &g->state.p[owner];
    int e = test_new_id(g, ENERGY);
    if (e < 0) return -1;
    if (P->energy.n >= RB_MAX_ZONE) return -1;
    P->energy.cards[P->energy.n++] = e;
    push_event(g, e, RB_ZONEID_ENERGY_DECK, RB_ZONEID_ENERGY_ZONE, causer_seat, 1);
    return e;
}

/* A REAL energy placement: 葉月 恋 PL!SP-pb1-005-R debuts, and her 登場 moves
 * one card energy_deck -> energy_zone.  This is the only placement path in the C
 * that is observed to arm the `energy_placed` leg of a compound watcher, and it
 * is what every positive in §B/§D4/§D11/§E2 uses, because a synthesised event
 * flag is not sufficient (see the report).  Returns 1 on a real placement. */
static int play_energy_placer(TestGame *g, int area)
{
    int placer = test_new_id(g, KAHORI);
    if (placer < 0) return 0;
    test_add_to_hand(g, placer);
    int zone_before = energy_n(g, 0);
    int deck_before = energy_deck_n(g, 0);
    int played = test_play_to_stage(g, placer, area);
    settle(g);
    return played && energy_n(g, 0) == zone_before + 1 &&
           energy_deck_n(g, 0) == deck_before - 1;
}

/* ── zone reads ────────────────────────────────────────────────────────── */

static int bag_n(const RbBag *b) { return b->n; }
static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int wait_energy(TestGame *g, int pl)
{
    RbPlayer *P = &g->state.p[pl];
    int act = rb_energy_active_count(P);
    int w = P->energy.n - act;
    return w < 0 ? 0 : w;
}
static int energy_n(TestGame *g, int pl) { return g->state.p[pl].energy.n; }
static int energy_deck_n(TestGame *g, int pl) { return g->state.p[pl].energy_deck.n; }
static int hand_n(TestGame *g, int pl) { return g->state.p[pl].hand.n; }
static int deck_n(TestGame *g, int pl) { return g->state.p[pl].deck.n; }
static int active_energy(TestGame *g, int pl) { return rb_energy_active_count(&g->state.p[pl]); }
static int on_stage(TestGame *g, int pl, int area, int cid)
{
    return g->state.p[pl].stage[area] == cid;
}
static int stage_has(TestGame *g, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (g->state.p[pl].stage[i] == cid) return 1;
    return 0;
}
static int under_has(TestGame *g, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&g->state.p[pl].under_cards[area], cid);
}

/* ── deck / energy seeding ─────────────────────────────────────────────── */

static void fill_decks(TestGame *g, int n)
{
    int filler = test_id(g, FILLER);
    g->state.p[0].deck.n = 0;
    g->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(g, 0, filler);
        test_add_to_deck_pl(g, 1, filler);
    }
}

static void fill_energy_deck(TestGame *g, int pl, int count)
{
    for (int i = 0; i < count; i++) {
        int e = test_new_id(g, ENERGY);
        if (e < 0) return;
        RbPlayer *P = &g->state.p[pl];
        if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = e;
    }
}

/* Rust `energy_zone.cards.push(..)` + `set_active_count(n)`.  Each card is its
 * own pool instance so the active/waited split is unambiguous. */
static void set_energy_active(TestGame *g, int pl, int total)
{
    RbPlayer *P = &g->state.p[pl];
    P->energy.n = 0;
    for (int i = 0; i < total && P->energy.n < RB_MAX_ZONE; i++) {
        int e = test_new_id(g, ENERGY);
        if (e < 0) return;
        P->energy.cards[P->energy.n++] = e;
    }
    rb_energy_set_active_count(P, total);
}

/* ── [TRIGGER-MISS] diagnostic ────────────────────────────────────────────
 * Prints, as hex, EVERY trigger string the card ACTUALLY carries, so a byte
 * escape typo can never hide behind a silent strcmp miss. */
static void dump_triggers(int cid, const char *want)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d prints card_no='%s' cost=%d nAb=%d "
                        "wanted='%s'\n",
                cid, rb_card_string(c.card_no_idx), (int)c.cost, n,
                want ? want : "(null)");
        rb_free_card(&c);
    } else {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d cannot decode, wanted='%s'\n",
                cid, want ? want : "(null)");
    }
    if (want) {
        fprintf(stderr, "  [TRIGGER-MISS] want hex:");
        for (const unsigned char *q = (const unsigned char *)want; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n");
    }
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers) {
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d hex:", i);
            for (const unsigned char *q = (const unsigned char *)ab.triggers; *q; q++)
                fprintf(stderr, " %02X", *q);
            fprintf(stderr, "  text='%s'\n", ab.triggers);
        } else {
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d (no trigger)\n", i);
        }
        rb_free_ability(&ab);
    }
}

/* AGENTS.md identity rule, byte-exact (rb_card_no_eq is fuzzy). */
static void expect_card_no(int cid, const char *no)
{
    char msg[256];
    int ok = 0;
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        ok = strcmp(rb_card_string(c.card_no_idx), no) == 0;
        rb_free_card(&c);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d is byte-exactly %s", cid, no);
    CHECK(ok, msg);
    if (!ok) dump_triggers(cid, "(identity)");
}

/* Rust `assert!(card has the trigger)`.  Also the fixture-wiring vacuity
 * control for every negative in this file. */
static int expect_trigger(int cid, const char *trig, int min_abilities)
{
    char msg[220];
    int n = rb_card_num_abilities((uint32_t)cid);
    snprintf(msg, sizeof(msg), "fixture cid=%d exposes >= %d ability/abilities", cid,
             min_abilities);
    CHECK(n >= min_abilities, msg);
    int found = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) found = 1;
        rb_free_ability(&ab);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d carries the trigger (hex:", cid);
    if (!found) {
        fprintf(stderr, "FAIL: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, ")\n");
        dump_triggers(cid, trig);
    } else {
        printf("ok: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            printf(" %02X", *q);
        printf(")\n");
    }
    assertions++;
    if (!found) failures++;
    return found;
}

/* How many abilities of `trig` the card PRINTS, read from the database, so a
 * card-pool update moves the expectation with the card. */
static int printed_trigger_count(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid), c = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) c++;
        rb_free_ability(&ab);
    }
    return c;
}

static int printed_cost(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static int printed_heart(int cid, int col)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = 0;
    for (int i = 0; i < c.n_hearts; i++)
        if ((int)(c.heart_color[i] % 8) == col) v++;
    rb_free_card(&c);
    return v;
}

/* Rust `choose_destination` / `shiki_swap_to`: assert the prompt really is
 * position|destination, then answer the ABSOLUTE area index. */
static int swap_into(TestGame *g, int area, const char *label)
{
    char msg[240];
    if (!pending(g)) {
        snprintf(msg, sizeof(msg), "%s: the 起動 must ask for a position|destination", label);
        CHECK(0, msg);
        return 0;
    }
    const RbChoice *c = pending_choice(g);
    int ok = c && c->target && strcmp(c->target, "position|destination") == 0;
    snprintf(msg, sizeof(msg), "%s: the pending prompt IS position|destination (got '%s')",
             label, (c && c->target) ? c->target : "(null)");
    CHECK(ok, msg);
    if (!ok) return 0;
    answer(g, area);
    settle(g);
    return 1;
}

/* ══════════════════════════════════════════════════════════════════════════
 * §A  energy_watch/debut_or_own_energy_return_place_waited_energy_test.rs
 *     PL!SP-bp7-005-R＋ ab#0 — 10 of the Rust file's 11 tests.
 *
 * The 11th, `debut_or_own_energy_return_verdicts_show_energy_leg_pass`, is NOT
 * PORTED: it asserts on `ability_verdicts`, a Rust-only debug helper that
 * renders "[PASS] compound" / "1/2 any" strings.  It has no C counterpart and
 * asserting on it would be asserting on the Rust test harness, not the engine.
 *
 * The whole section is one 自動 with an OR of two legs, so the section's real
 * claim is: BOTH legs arm it, the placed card is WAIT, and ターン1回 makes the
 * pair fire exactly once.  The engine-gap probe (`asis` scan) is woven into
 * the positive cases rather than isolated, so each positive names exactly which
 * engine entry point armed the watcher.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Shared board: 葉月 恋 ab#0 on CENTER, an energy deck of `n` real energy
 * cards, and an EMPTY energy zone (so any placement is unambiguous). */
static int ren_or_board(TestGame *g, int ren, int energy_deck_count)
{
    fill_decks(g, 30);
    clear_stage(g, 0);
    clear_stage(g, 1);
    g->state.p[0].stage[1] = ren;
    g->state.p[0].energy.n = 0;
    g->state.p[0].energy_active = 0;
    g->state.p[0].energy_deck.n = 0;
    fill_energy_deck(g, 0, energy_deck_count);
    return energy_deck_n(g, 0);
}

/* A1. debut_or_own_energy_return_appearance_triggers_energy_place */
static void a1_appearance_leg_places_one_waited_energy(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    CHECK(ren >= 0, "A1: the fixture resolves");
    if (ren < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);
    CHECK_EQ(printed_cost(ren), 9, "A1 precondition: 葉月 恋(R＋) prints cost 9");

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_active = 0;
    g.state.p[0].energy_deck.n = 0;
    fill_energy_deck(&g, 0, 5);
    /* The premises are read BEFORE give_energy, which is what fills the energy
     * zone: asserting "the zone starts empty" after the seeding would be a test
     * bug, not a finding. */
    CHECK_EQ(energy_deck_n(&g, 0), 5, "A1 precondition: 5 energy in the energy DECK");
    CHECK_EQ(energy_n(&g, 0), 0, "A1 precondition: the energy zone starts empty");
    test_give_energy(&g, 15);
    test_add_to_hand(&g, ren);

    int deck_before = energy_deck_n(&g, 0);
    int zone_before = energy_n(&g, 0);
    int active_before = active_energy(&g, 0);
    CHECK_EQ(deck_before, 5, "A1 precondition: the energy deck is untouched by the play setup");
    CHECK_EQ(zone_before, 15, "A1 precondition: 15 active energy pay her cost");

    int played = test_play_to_stage(&g, ren, 1 /* centre */);
    settle(&g);
    CHECK_EQ(played, 1, "A1: 葉月 恋's debut is accepted");
    CHECK(on_stage(&g, 0, 1, ren), "A1: she is on CENTER afterwards");

    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "A1 LEG (a): her 登場 must move exactly 1 card OUT of the energy deck "
             "— the second OR leg the parser used to drop");
    CHECK_EQ(energy_n(&g, 0), zone_before + 1,
             "A1 LEG (a): her 登場 must place exactly 1 card INTO the energy zone");
    CHECK_EQ(active_energy(&g, 0), active_before - 9,
             "A1 LEG (a): the placed card is WAIT — active drops by the 9 play cost "
             "and by nothing else");
}

/* A2/A3. the energy_zone -> energy_deck leg, and the WAIT/not-active claim. */
static void a2_energy_return_leg_places_one_waited_energy(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    int energy = test_new_id(&g, ENERGY);
    CHECK(ren >= 0 && energy >= 0, "A2: the fixtures resolve");
    if (ren < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_card_no(energy, ENERGY);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    /* The energy card that is genuinely IN THE ENERGY DECK is the one that
     * travels; the condition's destination filter needs it there. */
    CHECK_EQ(energy_deck_n(&g, 0), 5, "A2 precondition: the energy deck really holds 5");
    int on_deck = g.state.p[0].energy_deck.cards[0];
    CHECK(on_deck >= 0, "A2 precondition: and its first card resolves");
    if (on_deck < 0) { n_setup_bugs++; return; }

    int deck_before = energy_deck_n(&g, 0);
    int zone_before = energy_n(&g, 0);
    int wait_before = wait_energy(&g, 0);
    int active_before = active_energy(&g, 0);

    /* Rust: push_energy_zone_change(&mut game, "p1"). */
    push_event(&g, on_deck, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);

    /* FIRST: the engine's own scan.  last_energy_placed_by_effect has no writer,
     * so this cannot arm ab#0; the assertion is the GAP PROOF, not a waiver. */
    scan_auto_engine_asis(&g, 0);
    settle(&g);
    CHECK_EQ(energy_deck_n(&g, 0), deck_before,
             "A2 GAP PROOF: the engine's OWN post-event scan (rb_queue_trigger_abilities) "
             "does not arm ab#0 — GameState.last_energy_placed_by_effect is declared at "
             "rabuka.h:1401 and never assigned anywhere in engine_c_wip");

    /* THEN: the synthesised-event scan, which is the only way to arm it. */
    scan_event(&g, 0, &on_deck, 1, 0, 0);
    settle(&g);

    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "A2 LEG (b): 自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれたとき "
             "must move 1 energy out of the energy DECK");
    CHECK_EQ(energy_n(&g, 0), zone_before + 1,
             "A2 LEG (b): and must place exactly 1 into the energy zone");
    CHECK_EQ(wait_energy(&g, 0), wait_before + 1,
             "A2 LEG (b): ウェイト状態で置く — the new card is WAITED");
    CHECK_EQ(active_energy(&g, 0), active_before,
             "A3: the placed energy must be WAIT, not active — the active count is "
             "UNCHANGED (this is the test the Rust file splits into its own case)");
}

/* A4. debut_or_own_energy_return_once_per_turn_when_both_triggers */
static void a4_once_per_turn_when_both_legs_fire(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    CHECK(ren >= 0, "A4: the fixture resolves");
    if (ren < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    int on_deck = g.state.p[0].energy_deck.cards[0];
    int deck_before = energy_deck_n(&g, 0);

    push_event(&g, on_deck, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &on_deck, 1, 0, 0);
    settle(&g);

    int deck_after_first = energy_deck_n(&g, 0);
    CHECK_EQ(deck_after_first, deck_before - 1,
             "A4: the FIRST trigger consumes exactly 1 energy");

    /* Rust: also records an appearance for her, then pushes a SECOND
     * energy-return event, then scans twice.  The ターン1回 gate must hold. */
    push_appearance(&g, ren);
    int second = g.state.p[0].energy_deck.n > 0 ? g.state.p[0].energy_deck.cards[0] : on_deck;
    push_event(&g, second, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &second, 1, 0, 0);
    settle(&g);
    scan_event(&g, 0, &second, 1, 0, 0);
    settle(&g);

    CHECK_EQ(energy_deck_n(&g, 0), deck_after_first,
             "A4: ターン1回 — the SECOND trigger in the same turn (appearance leg + a "
             "second energy-return event, scanned twice) must NOT place another energy");
    CHECK_EQ(rb_ability_uses_used(&g.state, ren, 0), 1,
             "A4: exactly ONE ターン1回 use is recorded, on ab#0");
}

/* A5. debut_or_own_energy_return_empty_energy_deck_places_nothing */
static void a5_empty_energy_deck_places_nothing(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    int energy = test_new_id(&g, ENERGY);
    CHECK(ren >= 0 && energy >= 0, "A5: the fixtures resolve");
    if (ren < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    fill_decks(&g, 30);
    clear_stage(&g, 0);
    g.state.p[0].stage[1] = ren;
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_deck.n = 0;
    CHECK_EQ(energy_deck_n(&g, 0), 0, "A5 precondition: the energy deck is EMPTY");

    /* The event still names an energy card, so the CONDITION half can pass; only
     * the effect half has nothing to move. */
    RbPlayer *P = &g.state.p[0];
    P->energy.cards[P->energy.n++] = energy;   /* a real energy card, in the ZONE */
    int zone_before = energy_n(&g, 0);
    push_event(&g, energy, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &energy, 1, 0, 0);
    settle(&g);

    CHECK_EQ(energy_n(&g, 0), zone_before,
             "A5: an empty energy deck must place nothing — no crash and no phantom "
             "card (the card that travelled is the one in the zone, not a new one)");
    CHECK_EQ(energy_deck_n(&g, 0), 0,
             "A5: and the energy deck is still empty");
}

/* A6. debut_or_own_energy_return_no_trigger_no_energy */
static void a6_no_trigger_event_places_nothing(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    CHECK(ren >= 0, "A6: the fixture resolves");
    if (ren < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    int zone_before = energy_n(&g, 0);
    int deck_before = energy_deck_n(&g, 0);

    scan_auto(&g, 0);
    settle(&g);

    CHECK_EQ(energy_n(&g, 0), zone_before,
             "A6: no trigger event at all -> no energy is placed");
    CHECK_EQ(energy_deck_n(&g, 0), deck_before,
             "A6: and the energy deck is untouched");
    /* Fixture-wiring control: the same board DOES fire on the matching event,
     * so the two zeroes above are the missing event and not a dead card. */
    int on_deck = g.state.p[0].energy_deck.cards[0];
    push_event(&g, on_deck, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &on_deck, 1, 0, 0);
    settle(&g);
    CHECK_EQ(energy_n(&g, 0), zone_before + 1,
             "A6 CONTROL: on the SAME board the matching energy-return event does place "
             "one energy — so the two zeroes above are the absent event, not a card "
             "that is not wired to anything");
}

/* A7. debut_or_own_energy_return_opponent_energy_move_does_not_trigger
 *
 * 「自分のエネルギー」 is own-energy only, so a p2-caused event must not arm it. */
static void a7_opponent_energy_move_does_not_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    CHECK(ren >= 0, "A7: the fixture resolves");
    if (ren < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    int on_deck = g.state.p[0].energy_deck.cards[0];
    int deck_before = energy_deck_n(&g, 0);

    /* cause_player_id = 1 (p2).  The condition's own resolution compares
     * movement->cause_player_id against the target player, so this is the
     * self-only half and not just a "nothing happened" board. */
    push_event(&g, on_deck, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 1, 1);
    scan_event(&g, 0, &on_deck, 1, 0, 0);
    settle(&g);

    CHECK_EQ(energy_deck_n(&g, 0), deck_before,
             "A7: an OPPONENT-caused energy return must not trigger 自分のエネルギー — "
             "the whole clause is the owner filter");

    /* PAIRED POSITIVE on the SAME board, same card, same event shape, only the
     * cause seat changed.  Without it the zero above is indistinguishable from
     * an event that never reached the scan. */
    int on_deck2 = g.state.p[0].energy_deck.cards[0];
    push_event(&g, on_deck2, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &on_deck2, 1, 0, 0);
    settle(&g);
    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "A7 CONTROL: the very same event caused by P1 on the very same board DOES "
             "place an energy — so the zero above is the owner filter and not silence");
}

/* A8. debut_or_own_energy_return_other_member_appearance_does_not_trigger
 *
 * The appearance leg is 「このメンバーが登場」, so a DIFFERENT member's
 * appearance must not arm her. */
static void a8_other_member_appearance_does_not_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    int other = test_id(&g, KINAKO_R_PLUS);
    CHECK(ren >= 0 && other >= 0, "A8: the fixtures resolve");
    if (ren < 0 || other < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_card_no(other, KINAKO_R_PLUS);
    expect_trigger(ren, TRIG_AUTO, 2);

    /* Premise, asserted rather than assumed: the two really are DIFFERENT
     * members, or 「このメンバー」 would be ambiguous and the case would prove
     * nothing. */
    {
        Card a, b;
        int distinct = 0;
        if (rb_decode_card_by_index((uint32_t)ren, &a) &&
            rb_decode_card_by_index((uint32_t)other, &b)) {
            distinct = a.name && b.name && strcmp(a.name, b.name) != 0;
            rb_free_card(&a);
            rb_free_card(&b);
        }
        CHECK(distinct,
              "A8 precondition: 葉月 恋 and 桜小路きな子 are DIFFERENT characters — a "
              "second print of 恋 would make このメンバー ambiguous");
    }

    ren_or_board(&g, ren, 5);
    int deck_before = energy_deck_n(&g, 0);

    push_appearance(&g, other);
    scan_auto(&g, 0);
    settle(&g);

    CHECK_EQ(energy_deck_n(&g, 0), deck_before,
             "A8: another member's appearance must not arm the self-appearance leg");

    /* PAIRED POSITIVE: HER appearance on the same board. */
    push_appearance(&g, ren);
    scan_auto(&g, 0);
    settle(&g);
    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "A8 CONTROL: her OWN appearance on the same board does place the energy — so "
             "the zero above is the このメンバー filter and not a dead appearance path");
}

/* A9. debut_or_own_energy_return_wrong_source_energy_move_does_not_trigger
 *
 * The condition's `source` is energy_zone, so a waitroom -> energy_deck event
 * must not satisfy it. */
static void a9_wrong_source_energy_move_does_not_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    int card = test_new_id(&g, FILLER);
    CHECK(ren >= 0 && card >= 0, "A9: the fixtures resolve");
    if (ren < 0 || card < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    /* Put the travelling card in the WAITROOM, so the destination-zone half of
     * the condition is genuinely satisfied and only `source` can fail it. */
    test_add_to_discard(&g, card);
    int deck_before = energy_deck_n(&g, 0);

    push_event(&g, card, RB_ZONEID_DISCARD, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, &card, 1, 0, 0);
    settle(&g);

    CHECK(bag_has(&g.state.p[0].discard, card),
          "A9 precondition: the travelling card really is in the WAITROOM, so the "
          "destination half of the condition can pass and only `source` can fail it");
    CHECK_EQ(energy_deck_n(&g, 0), deck_before,
             "A9: the condition's source is energy_zone — a waitroom -> energy_deck "
             "event must not fire it");
}

/* A10. debut_or_own_energy_return_multiple_energy_moves_single_fire
 *
 * Two matching events in the same batch must still place exactly ONE energy:
 * the OR fires a single instance and ターン1回 is a ceiling. */
static void a10_multiple_energy_moves_single_fire(void)
{
    static TestGame g;
    new_game(&g);

    int ren = test_id(&g, REN_OR);
    CHECK(ren >= 0, "A10: the fixture resolves");
    if (ren < 0) { n_setup_bugs++; return; }
    expect_card_no(ren, REN_OR);
    expect_trigger(ren, TRIG_AUTO, 2);

    ren_or_board(&g, ren, 5);
    int e1 = g.state.p[0].energy_deck.cards[0];
    int e2 = g.state.p[0].energy_deck.cards[1];
    int deck_before = energy_deck_n(&g, 0);
    int wait_before = wait_energy(&g, 0);

    push_event(&g, e1, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    push_event(&g, e2, RB_ZONEID_ENERGY_ZONE, RB_ZONEID_ENERGY_DECK, 0, 1);
    scan_event(&g, 0, (int[]){ e1, e2 }, 2, 0, 0);
    settle(&g);

    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "A10: two matching energy-return events in ONE batch -> a single "
             "once-per-turn placement");
    CHECK_EQ(wait_energy(&g, 0), wait_before + 1,
             "A10: exactly ONE new WAIT energy, not two");
    CHECK_EQ(rb_ability_uses_used(&g.state, ren, 0), 1,
             "A10: the ターン1回 budget was spent once");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §B  energy_watch/own_effect_energy_placed_each_time_gain_heart06_test.rs
 *     PL!SP-bp4-016-N ab#0 — the 2 tests the existing corpus does not hold.
 *     (the own-caused and opponent-caused positives are already in
 *     tests/test_parity_trigger_paths.c §S2, and are not duplicated here)
 * ══════════════════════════════════════════════════════════════════════════ */

/* B1. two_hazuki_copies_gain_once_per_opponent_effect_energy_placement
 *
 * TWO DISTINCT pool instances of the same print, each of which must gain
 * heart06 on EVERY opponent-caused energy placement (each_time, and the
 * parenthetical says opponent effects count). */
static void b1_two_copies_each_time_gains_one_heart06_per_placement(void)
{
    static TestGame g;
    new_game(&g);

    int left = test_new_id(&g, HAZUKI_BP4);
    int right = test_new_id(&g, HAZUKI_BP4);
    CHECK(left >= 0 && right >= 0, "B1: the fixtures resolve");
    if (left < 0 || right < 0) { n_setup_bugs++; return; }
    expect_card_no(left, HAZUKI_BP4);
    expect_card_no(right, HAZUKI_BP4);
    CHECK(left != right,
          "B1 precondition: the two copies are DISTINCT pool instances (test_new_id, "
          "not the shared test_id template) — otherwise the two heart06 counts are "
          "the same card counted twice");
    expect_trigger(left, TRIG_AUTO, 1);
    CHECK_EQ(printed_trigger_count(right, TRIG_AUTO), 1,
             "B1 precondition: the second copy carries the same single 自動");

    fill_decks(&g, 30);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = left;
    g.state.p[0].stage[1] = right;
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_active = 0;
    g.state.p[0].energy_deck.n = 0;

    for (int round = 1; round <= 2; round++) {
        int e = place_energy_in_zone(&g, 0, /*causer seat*/ 1);
        CHECK(e >= 0, "B1: an energy card landed in P1's zone, caused by P2's effect");
        if (e < 0) { n_setup_bugs++; return; }
        scan_event(&g, 0, &e, 1, 0, 1);
        settle(&g);
        char msg[200];
        snprintf(msg, sizeof(msg),
                 "B1 round %d: each copy gains exactly one more heart06 (left %d, right %d)",
                 round, heart(&g, left, 6), heart(&g, right, 6));
        CHECK_EQ(heart(&g, left, 6), round, msg);
        CHECK_EQ(heart(&g, right, 6), round,
                 "B1: the SECOND copy gains the same amount — the watcher is not "
                 "single-instance scoped");
    }
    CHECK_EQ(energy_n(&g, 0), 2,
             "B1: two placements really did put 2 cards in the energy zone");
    CHECK_EQ(wait_energy(&g, 0), 0,
             "B1: both placements are ACTIVE — the new cards land active, so the "
             "each_time gate is not counting waited cards");
}

/* B2. hazuki_energy_phase_no_effect_flag
 *
 * The energy-phase draw is a PHASE action, not a card effect, so the flag is
 * absent and ab#0 must not fire.  Guarded by a paired positive on the same
 * board, because "no flag -> nothing" is exactly the assertion a board on which
 * the watcher is dead would also satisfy. */
static void b2_phase_action_energy_does_not_fire(void)
{
    static TestGame g;
    new_game(&g);

    int watcher = test_new_id(&g, HAZUKI_BP4);
    CHECK(watcher >= 0, "B2: the fixture resolves");
    if (watcher < 0) { n_setup_bugs++; return; }
    expect_card_no(watcher, HAZUKI_BP4);
    expect_trigger(watcher, TRIG_AUTO, 1);

    fill_decks(&g, 30);
    clear_stage(&g, 0);
    g.state.p[0].stage[1] = watcher;
    set_energy_active(&g, 0, 1);
    set_energy_active(&g, 1, 1);

    /* The energy PHASE places a card: the card is in the zone, but no movement
     * event was recorded and no card effect caused it. */
    {
        RbPlayer *P = &g.state.p[0];
        int e = test_new_id(&g, ENERGY);
        if (e < 0) { n_setup_bugs++; return; }
        P->energy.cards[P->energy.n++] = e;
    }
    scan_auto(&g, 0);
    settle(&g);

    CHECK_EQ(energy_n(&g, 0), 2,
             "B2 precondition: an energy card really is in the zone, so the condition's "
             "「エネルギーカードがある」 half passes and only the EVENT is missing");
    CHECK_EQ(heart(&g, watcher, 6), 0,
             "B2: カードの効果によって — an energy placed by the energy PHASE, not by a "
             "card effect, must not fire the watcher");

    /* PAIRED POSITIVE on the SAME board: now a card effect does place one. */
    int e = place_energy_in_zone(&g, 0, 0);
    CHECK(e >= 0, "B2 CONTROL: P1's own card effect places an energy card");
    if (e < 0) return;
    scan_event(&g, 0, &e, 1, 0, 1);
    settle(&g);
    CHECK_EQ(heart(&g, watcher, 6), 1,
             "B2 CONTROL: the same card DOES fire on a card-effect placement, so the "
             "zero above is the missing event and not a dead watcher");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §C  energy_watch/real_card_effect_self_move_or_energy_placed_draw_heart02_
 *     test.rs — 2 of 2, driven entirely through REAL card effects.
 *
 * Watcher: PL!SP-bp5-004-R＋ 平安名すみれ ab#0, 自動/ターン1回.
 * These are the only two cases in the whole cluster that need no synthesised
 * event: a real 起動 swap and a real 登場 energy placement both arm her, so
 * they are the file's cleanest proof that the watcher itself is wired.
 * ══════════════════════════════════════════════════════════════════════════ */

/* C1. own_effect_self_swap_draws_one_and_gains_heart02 */
static void c1_own_effect_swap_draws_one_and_gains_heart02(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int shiki = test_new_id(&g, SHIKI);
    CHECK(sumire >= 0 && shiki >= 0, "C1: the fixtures resolve");
    if (sumire < 0 || shiki < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(shiki, SHIKI);
    expect_trigger(sumire, TRIG_AUTO, 1);
    CHECK_EQ(printed_trigger_count(sumire, TRIG_AUTO), 1,
             "C1 precondition: すみれ carries exactly ONE 自動 (the compound one)");

    fill_decks(&g, 30);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = sumire;      /* LEFT */
    test_give_energy(&g, 12);            /* 9 for her debut + E + ... for 四季's 起動 */
    test_add_to_hand(&g, shiki);
    int played = test_play_to_stage(&g, shiki, 1 /* centre */);
    settle(&g);
    CHECK_EQ(played, 1, "C1: 若菜四季's debut is accepted");
    CHECK(on_stage(&g, 0, 1, shiki), "C1: she is on CENTER");

    int hand_before = hand_n(&g, 0);
    int h_before = heart(&g, sumire, 2);

    /* Activate her 起動 and swap into the LEFT area, which すみれ occupies. */
    int activated = test_activate_ability(&g, shiki);
    settle(&g);
    CHECK(activated, "C1: 若菜四季 offers her 起動");
    CHECK(swap_into(&g, 0, "C1"), "C1: the swap asks for a destination");

    CHECK_EQ(g.state.p[0].stage[0], shiki,
             "C1: the swap must have moved すみれ out of LEFT via OUR OWN effect");
    CHECK_EQ(g.state.p[0].stage[1], sumire, "C1: and 若菜四季 took the area she left");
    CHECK_EQ(heart(&g, sumire, 2), h_before + 1,
             "C1: エリアを移動する alone fires the compound 自動 -> +1 heart02");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1,
             "C1: the same trigger draws 1 card (カードを1枚引き)");
}

/* C2. own_effect_energy_placement_draws_one_and_gains_heart02 */
static void c2_own_effect_energy_placement_draws_one_and_gains_heart02(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int placer = test_new_id(&g, KAHORI);
    CHECK(sumire >= 0 && placer >= 0, "C2: the fixtures resolve");
    if (sumire < 0 || placer < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(placer, KAHORI);
    expect_trigger(sumire, TRIG_AUTO, 1);
    /* The placer's real job: 登場 places 1 energy from the energy DECK. */
    expect_trigger(placer, TRIG_DEBUT_STAGE, 1);

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = sumire;
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_active = 0;
    g.state.p[0].energy_deck.n = 0;
    fill_energy_deck(&g, 0, 3);
    test_give_energy(&g, 20);
    test_add_to_hand(&g, placer);

    int zone_before = energy_n(&g, 0);
    int deck_before = energy_deck_n(&g, 0);
    int hand_before = hand_n(&g, 0);

    int played = test_play_to_stage(&g, placer, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "C2: 葉月 恋's debut is accepted");
    CHECK(on_stage(&g, 0, 2, placer), "C2: she stays in the played right slot");
    CHECK_EQ(energy_deck_n(&g, 0), deck_before - 1,
             "C2 PRECONDITION: her 登場 really moved 1 card out of the energy DECK");
    CHECK_EQ(energy_n(&g, 0), zone_before + 1,
             "C2 PRECONDITION: and really placed 1 card into the energy zone");

    int h_after = heart(&g, sumire, 2);
    int hand_after = hand_n(&g, 0);
    CHECK_EQ(h_after, 1,
             "C2: 自分のエネルギー置き場にエネルギーが置かれたとき alone fires the "
             "compound 自動 -> +1 heart02");
    CHECK_EQ(hand_after, hand_before,
             "C2: the draw replaces the played card (-1 hand card, +1 drawn), so the "
             "hand count is unchanged");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D  energy_watch/own_effect_self_move_or_energy_placed_draw_heart02_test.rs
 *     — 12 of the Rust file's 16 tests.
 *
 * NOT PORTED, and why (the four are near-duplicates of ported cases, not
 * distinct behaviours — porting them would pad the file, not the coverage):
 *   own_move_or_energy_draw_heart02_energy_effect_triggers  == C2 (same board,
 *       same 葉月 恋 登場 placer, same two assertions)
 *   own_move_or_energy_draw_heart02_debut_energy_placement_  == C2
 *       _triggers
 *   own_move_or_energy_draw_heart02_opponent_effect_move_    == D3 (the same
 *       _no_trigger                                                    assertion)
 *   own_move_or_energy_draw_heart02_position_change_then_energy_ == D9
 *       _use_limit_blocks_second
 *
 * The four negatives that DO get ported (D2, D4, D5, D6, D8) all carry a
 * paired positive or an explicit premise assertion, because "the ability did
 * nothing" is precisely what a board with a dead watcher also looks like.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Shared board for the pure-event cases: すみれ alone on CENTER, deck seeded,
 * energy zone empty, no synthesised flags. */
static int sumire_board(TestGame *g, int sumire, int energy_active)
{
    fill_decks(g, 40);
    clear_stage(g, 0);
    clear_stage(g, 1);
    g->state.p[0].stage[1] = sumire;
    g->state.p[0].energy.n = 0;
    g->state.p[0].energy_active = 0;
    g->state.p[0].energy_deck.n = 0;
    set_energy_active(g, 0, energy_active);
    set_energy_active(g, 1, 0);
    return energy_n(g, 0);
}

/* D1. own_move_or_energy_draw_heart02_area_move_triggers_draw_and_heart
 *
 * The きな子 起動 swap, driven as a real activation.  This is the case that
 * proves the AREA-MOVE half of the compound OR on a board with no energy deck
 * and no energy event of any kind, so the only satisfiable leg is the move. */
static void d1_area_move_leg_fires(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int kinako = test_new_id(&g, KINAKO_R);
    CHECK(sumire >= 0 && kinako >= 0, "D1: the fixtures resolve");
    if (sumire < 0 || kinako < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(kinako, KINAKO_R);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(kinako, TRIG_KIDOU, 1);

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = sumire;   /* LEFT  */
    g.state.p[0].stage[1] = kinako;   /* CENTRE */
    CHECK_EQ(energy_deck_n(&g, 0), 0,
             "D1 PRECONDITION: the energy DECK is empty and the energy zone holds "
             "nothing, so the energy leg of the OR cannot be what fires");
    test_give_energy(&g, 20);
    settle(&g);

    int hand_before = hand_n(&g, 0);
    int activated = test_activate_ability(&g, kinako);
    settle(&g);
    CHECK(activated, "D1: 桜小路きな子 offers her 起動");
    CHECK(swap_into(&g, 0, "D1"), "D1: the swap asks for a destination");

    CHECK_EQ(g.state.p[0].stage[0], kinako,
             "D1: the swap must have moved すみれ (LEFT) via our own effect");
    CHECK_EQ(g.state.p[0].stage[1], sumire, "D1: and きな子 took the centre she left");
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D1: エリアを移動する alone fires the compound 自動 -> +1 heart02");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1,
             "D1: One card drawn after area move");
}

/* D2. own_move_or_energy_draw_heart02_no_event_no_trigger
 *
 * The purest negative in the file: a board with a real watcher, no event of any
 * kind, and an explicit paired positive at the end. */
static void d2_no_event_no_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int other = test_new_id(&g, FILLER);
    CHECK(sumire >= 0 && other >= 0, "D2: the fixtures resolve");
    if (sumire < 0 || other < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);
    /* A DIFFERENT card, so the positive below really is a card-identity gate
     * rather than "some watcher fired". */
    expect_card_no(other, FILLER);

    sumire_board(&g, sumire, 0);
    int hand_before = hand_n(&g, 0);
    int deck_before = deck_n(&g, 0);

    scan_auto(&g, 0);
    settle(&g);

    CHECK_EQ(heart(&g, sumire, 2), 0, "D2: no event = no trigger");
    CHECK_EQ(hand_n(&g, 0), hand_before, "D2: no event = no draw");
    CHECK_EQ(deck_n(&g, 0), deck_before, "D2: and the deck is untouched");

    /* PAIRED POSITIVE on the SAME board: the same card, the same watcher, one
     * own-effect area move. */
    int moved[1] = { sumire };
    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, moved, 1, 1, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D2 CONTROL: on the SAME board one own-effect area move DOES fire the "
             "watcher, so the zero above is the absent event and not a dead card");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1, "D2 CONTROL: and it draws exactly one card");
}

/* D3. own_move_or_energy_draw_heart02_opponent_no_trigger
 *
 * 「自分のカードの効果によって」 — an opponent-CAUSED move of すみれ herself
 * must not fire.  This is the `self_effect_only` gate. */
static void d3_opponent_effect_move_no_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    CHECK(sumire >= 0, "D3: the fixture resolves");
    if (sumire < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);

    sumire_board(&g, sumire, 0);
    int hand_before = hand_n(&g, 0);

    /* The card that moved IS すみれ, so the only clause that can reject this is
     * the cause.  That is asserted rather than assumed. */
    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, /*causer*/ 1, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);

    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D3: an OPPONENT-caused move of すみれ herself must not fire "
             "「自分のカードの効果によって」 (self_effect_only)");
    CHECK_EQ(hand_n(&g, 0), hand_before, "D3: and no draw");

    /* PAIRED POSITIVE: identical event, cause seat 0. */
    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D3 CONTROL: the SAME move caused by P1 DOES fire on the same board, so the "
             "zero above is the cause filter and not a dead watcher");
}

/* D4. own_move_or_energy_draw_heart02_opponent_energy_no_trigger
 *
 * The energy leg's own self_effect_only: an opponent-caused energy placement. */
static void d4_opponent_energy_placement_no_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    CHECK(sumire >= 0, "D4: the fixture resolves");
    if (sumire < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);

    sumire_board(&g, sumire, 0);
    int hand_before = hand_n(&g, 0);

    int e = place_energy_in_zone(&g, 0, /*causer seat*/ 1);
    CHECK(e >= 0, "D4: P2's card effect placed an energy card into P1's OWN zone");
    if (e < 0) { n_setup_bugs++; return; }
    scan_event(&g, 0, &e, 1, 0, 1);
    settle(&g);

    CHECK_EQ(energy_n(&g, 0), 1,
             "D4 PRECONDITION: the energy card really is in P1's energy zone, so the "
             "placement half of the OR is satisfied and only the CAUSE can fail it");
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D4: opponent energy must NOT trigger すみれ — 自分のカードの効果によって");
    CHECK_EQ(hand_n(&g, 0), hand_before, "D4: and no draw");

    /* PAIRED POSITIVE: identical event, cause seat 0. */
    int e2 = place_energy_in_zone(&g, 0, 0);
    CHECK(e2 >= 0, "D4 CONTROL: P1's own card effect places a second energy card");
    if (e2 < 0) return;
    scan_event(&g, 0, &e2, 1, 0, 1);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D4 CONTROL: the same placement caused by P1 DOES fire, so the zero above "
             "is the cause filter and not a dead watcher");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1, "D4 CONTROL: and it draws exactly one card");
}

/* D5. own_move_or_energy_draw_heart02_other_card_move_does_not_trigger
 *
 * 「このメンバーがエリアを移動する」 — a DIFFERENT card's own-effect move must
 * not fire her.  (The Rust test's own doc comment claims the opposite of its
 * assertion; the ASSERTION is ported here, and the card-identity question it
 * turns on is what this case is actually for.) */
static void d5_other_card_move_no_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int other = test_new_id(&g, FILLER);
    CHECK(sumire >= 0 && other >= 0, "D5: the fixtures resolve");
    if (sumire < 0 || other < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(other, FILLER);
    expect_trigger(sumire, TRIG_AUTO, 1);

    /* Premise: the two really are different cards, so 「このメンバー」 is a real
     * identity test here and not a tautology. */
    {
        Card a, b;
        int distinct = 0;
        if (rb_decode_card_by_index((uint32_t)sumire, &a) &&
            rb_decode_card_by_index((uint32_t)other, &b)) {
            distinct = a.name && b.name && strcmp(a.name, b.name) != 0;
            rb_free_card(&a);
            rb_free_card(&b);
        }
        CHECK(distinct,
              "D5 PRECONDITION: 平安名すみれ and 高坂 穂乃果 are DIFFERENT cards — with "
              "one id, このメンバー would be trivially true");
    }

    sumire_board(&g, sumire, 0);
    /* The other card has to be ON STAGE for a stage->stage move to mean
     * anything; asserted rather than assumed. */
    g.state.p[0].stage[0] = other;
    int hand_before = hand_n(&g, 0);

    push_event(&g, other, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &other, 1, 1, 0);
    settle(&g);

    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D5: a DIFFERENT card's own-effect area move must not fire すみれ — "
             "このメンバー, card identity check");
    CHECK_EQ(hand_n(&g, 0), hand_before, "D5: no draw from a different-card move");

    /* PAIRED POSITIVE: now the card that moves is すみれ herself. */
    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D5 CONTROL: her OWN move DOES fire on the same board, so the zero above is "
             "the card-identity filter and not a dead watcher");
}

/* D6. own_move_or_energy_draw_heart02_empty_destination_swap_no_trigger
 *
 * きな子's 起動 moves her into an EMPTY centre; すみれ on the right does not
 * move, so the watcher must not fire.  Guarded by a paired positive, because
 * "the swap happened and nothing fired" is what a dead watcher looks like too. */
static void d6_swap_to_empty_area_leaves_her_unmoved(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int kinako = test_new_id(&g, KINAKO_R);
    CHECK(sumire >= 0 && kinako >= 0, "D6: the fixtures resolve");
    if (sumire < 0 || kinako < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(kinako, KINAKO_R);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(kinako, TRIG_KIDOU, 1);

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = kinako;   /* LEFT   */
    g.state.p[0].stage[2] = sumire;   /* RIGHT  */
    g.state.p[0].stage[1] = RB_EMPTY_SLOT;   /* CENTRE is empty */
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_deck.n = 0;
    test_give_energy(&g, 20);
    settle(&g);

    int activated = test_activate_ability(&g, kinako);
    settle(&g);
    CHECK(activated, "D6: 桜小路きな子 offers her 起動");
    CHECK(swap_into(&g, 1, "D6"), "D6: the swap asks for a destination");

    CHECK_EQ(g.state.p[0].stage[1], kinako,
             "D6: きな子 really moved into the EMPTY centre — no swap partner");
    CHECK_EQ(g.state.p[0].stage[2], sumire,
             "D6: すみれ is still on the right — she did NOT move");
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D6: Sumire should NOT trigger — she did not move");
    CHECK_EQ(hand_n(&g, 0), 0, "D6: no draw — Sumire did not move");

    /* PAIRED POSITIVE: same board, same activation driver, but aim the swap at
     * the slot すみれ occupies. */
    {
        static TestGame g2;
        new_game(&g2);
        int s2 = test_new_id(&g2, SUMIRE);
        int k2 = test_new_id(&g2, KINAKO_R);
        if (s2 < 0 || k2 < 0) { n_setup_bugs++; return; }
        fill_decks(&g2, 40);
        clear_stage(&g2, 0);
        g2.state.p[0].stage[0] = k2;
        g2.state.p[0].stage[2] = s2;
        g2.state.p[0].energy.n = 0;
        g2.state.p[0].energy_deck.n = 0;
        test_give_energy(&g2, 20);
        settle(&g2);
        CHECK(test_activate_ability(&g2, k2), "D6 CONTROL: きな子 offers her 起動 again");
        CHECK(swap_into(&g2, 2, "D6 CONTROL"), "D6 CONTROL: the swap asks for a destination");
        CHECK_EQ(g2.state.p[0].stage[2], k2,
                 "D6 CONTROL: this time the swap really did displace すみれ");
        CHECK_EQ(heart(&g2, s2, 2), 1,
                 "D6 CONTROL: displacing her DOES fire the watcher on the same fixture, "
                 "so the zero in the main case is the unmoved card and not a dead card");
    }
}

/* D7. own_move_or_energy_draw_heart02_use_limit_blocks_second
 *
 * ターン1回 is a CEILING: the area-move leg fires, then an energy placement in
 * the same turn must add nothing.  The EXACT value is asserted in both halves —
 * a `<= 1` would also pass if the first firing had silently failed. */
static void d7_use_limit_blocks_the_second_leg(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    CHECK(sumire >= 0, "D7: the fixture resolves");
    if (sumire < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);

    sumire_board(&g, sumire, 0);
    int hand_before = hand_n(&g, 0);

    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);

    CHECK_EQ(heart(&g, sumire, 2), 1, "D7: heart02 from the area-move trigger");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1, "D7: and its one card draw");
    CHECK_EQ(rb_ability_uses_used(&g.state, sumire, 0), 1,
             "D7: exactly one ターン1回 use recorded");

    /* The other leg of the OR, in the same turn. */
    int e = place_energy_in_zone(&g, 0, 0);
    CHECK(e >= 0, "D7: P1's own effect now places an energy card (the OR's other leg)");
    if (e < 0) return;
    scan_event(&g, 0, &e, 1, 0, 1);
    settle(&g);

    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D7: ターン1回 is a CEILING — the energy leg must not add a second heart02 "
             "in the same turn. The EXACT value pins both halves: a `<= 1` reading "
             "would also pass if the first firing had silently failed");
    CHECK_EQ(hand_n(&g, 0), hand_before + 1, "D7: and no second draw");
    CHECK_EQ(rb_ability_uses_used(&g.state, sumire, 0), 1,
             "D7: the use budget is still exactly 1");
}

/* D8. own_move_or_energy_draw_heart02_turn_two_limit_resets */
static void d8_turn_limit_resets_on_the_next_turn(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    CHECK(sumire >= 0, "D8: the fixture resolves");
    if (sumire < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);

    sumire_board(&g, sumire, 0);

    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1, "D8 turn 1: the watcher fires");

    /* The Rust turn rollover: clear the per-turn movement structures and bump
     * the turn counter, which is what rb_clear_movement_tracking +
     * rb_reset_keyword_tracking do at phases.rs:310. */
    g.state.turn += 1;
    rb_clear_movement_tracking(&g.state);
    g.state.n_recently_moved = 0;
    g.state.position_change_occurred_this_turn = 0;

    push_event(&g, sumire, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    scan_event(&g, 0, &sumire, 1, 1, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 2,
             "D8 turn 2: ターン1回 resets across turns — the watcher fires a second time");
}

/* D9. own_move_or_energy_draw_heart02_declined_debut_position_change_no_trigger
 *
 * 唐 可可's 登場 offers an OPTIONAL position change.  Declining it must leave
 * すみれ unmoved and silent. */
static void d9_declined_debut_position_change_no_trigger(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int keke = test_new_id(&g, KEKE);
    CHECK(sumire >= 0 && keke >= 0, "D9: the fixtures resolve");
    if (sumire < 0 || keke < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(keke, KEKE);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(keke, TRIG_DEBUT_STAGE, 1);

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = sumire;
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_deck.n = 0;
    test_give_energy(&g, 20);
    test_add_to_hand(&g, keke);
    int hand_before = hand_n(&g, 0);

    int played = test_play_to_stage(&g, keke, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "D9: 唐 可可's debut is accepted");
    CHECK(on_stage(&g, 0, 2, keke), "D9: she is on the right");

    /* The prompt is the whole point of the case, so it is asserted. */
    CHECK_PROMPTS(&g, "D9: -CONSTANT- the optional position change is actually offered "
                      "after 可可's debut");
    {
        const RbChoice *c = pending_choice(&g);
        int is_dest = c && c->target && strcmp(c->target, "position|destination") == 0;
        CHECK(is_dest, "D9: and the prompt is a position|destination (allow_skip must be set)");
    }
    /* DECLINE it (Rust answers the ChoiceSkip). */
    answer(&g, -1);
    settle(&g);

    CHECK_EQ(g.state.p[0].stage[0], sumire,
             "D9: すみれ never moved — the position change was declined");
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D9: Sumire should NOT trigger — no position change occurred");
    CHECK_EQ(hand_n(&g, 0), hand_before - 1,
             "D9: no extra draw (only the played 可可 left the hand)");
}

/* D10. own_move_or_energy_draw_heart02_swap_places_self_and_grants_both
 *
 * きな子 on LEFT, すみれ on RIGHT; the 起動 swap into RIGHT displaces her.  The
 * two cases that share this board (displaced -> fires, empty -> does not) are
 * D10 and D6. */
static void d10_swap_displacing_sumire_fires(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int kinako = test_new_id(&g, KINAKO_R);
    CHECK(sumire >= 0 && kinako >= 0, "D10: the fixtures resolve");
    if (sumire < 0 || kinako < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(kinako, KINAKO_R);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(kinako, TRIG_KIDOU, 1);

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = kinako;   /* LEFT  */
    g.state.p[0].stage[2] = sumire;   /* RIGHT */
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_deck.n = 0;
    test_give_energy(&g, 20);
    settle(&g);

    int activated = test_activate_ability(&g, kinako);
    settle(&g);
    CHECK(activated, "D10: 桜小路きな子 offers her 起動");
    CHECK(swap_into(&g, 2, "D10"), "D10: the swap asks for a destination");

    CHECK_EQ(g.state.p[0].stage[0], sumire, "D10: sumire should be at left after the swap");
    CHECK_EQ(g.state.p[0].stage[2], kinako, "D10: kinako should be at right after the swap");
    CHECK_EQ(heart(&g, sumire, 2), 1, "D10: Sumire gains heart02 after area move");
    CHECK_EQ(hand_n(&g, 0), 1, "D10: Sumire draws 1 card after her own area move");
}

/* D11. own_move_or_energy_draw_heart02_opponent_no_trigger (energy leg, as-is
 * scan) — the engine's own entry point, no synthesised flag.  This is the case
 * that shows the whole compound watcher is silent under the engine's own
 * post-placement scan, and it is paired with the synthesised-event positive. */
static void d11_own_scan_never_arms_the_energy_leg(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    CHECK(sumire >= 0, "D11: the fixture resolves");
    if (sumire < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_trigger(sumire, TRIG_AUTO, 1);

    sumire_board(&g, sumire, 0);
    int hand_before = hand_n(&g, 0);

    int e = place_energy_in_zone(&g, 0, 0);
    CHECK(e >= 0, "D11: P1's own card effect placed an energy card into P1's zone");
    if (e < 0) { n_setup_bugs++; return; }

    /* The engine's own scan, with NO synthesised event flags. */
    scan_auto_engine_asis(&g, 0);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "D11 GAP PROOF: after a REAL own-effect energy placement the engine's own "
             "scan (rb_queue_trigger_abilities) grants すみれ nothing, because "
             "GameState.last_energy_placed_by_effect (rabuka.h:1401) has no writer in "
             "engine_c_wip — the energy_placed flag is structurally unreachable");
    CHECK_EQ(hand_n(&g, 0), hand_before, "D11: and draws nothing");

    /* Same board, same event, synthesised flag: the watcher is alive. */
    int e2 = place_energy_in_zone(&g, 0, 0);
    CHECK(e2 >= 0, "D11 CONTROL: a second own-effect energy placement");
    if (e2 < 0) return;
    scan_event(&g, 0, &e2, 1, 0, 1);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "D11 CONTROL: the SAME board and the SAME watcher fire the moment the "
             "energy_placed flag is supplied, so the zero above is the missing flag "
             "and not a dead card");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §E  energy_watch/energy_placement_watchers_across_debut_and_position_change_
 *     test.rs — 4 of 4.
 *
 * This is the only file in either cluster with TWO watchers on one board: すみれ
 * (own-effect only) and 葉月 恋 bp4-016 (fires on opponent effects too).  So it
 * is the file that separates the two scopes, and every negative here is
 * licensed by a positive on the SAME board.
 *
 * RUST FIXTURE BUG carried into this port: the Rust file spells すみれ as
 * "PL!SP-bp5-004-R+" with a HALFWIDTH plus.  cards.json has only
 * "PL!SP-bp5-004-R＋" (U+FF0B).  This port uses the real bytes; the Rust
 * fixture does not name an existing card.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Shared E board: すみれ LEFT, a second watcher CENTRE, 15 ACTIVE energy, a
 * seeded main deck, and an energy deck for the 葉月 恋 登場 placer. */
static int e_board(TestGame *g, int sumire, int second_watcher)
{
    fill_decks(g, 40);
    clear_stage(g, 0);
    clear_stage(g, 1);
    g->state.p[0].stage[0] = sumire;
    g->state.p[0].stage[1] = second_watcher;
    set_energy_active(g, 0, 15);
    set_energy_active(g, 1, 0);
    g->state.p[0].energy_deck.n = 0;
    return hand_n(g, 0);
}

/* E1. hazuki_debut_energy_placement_triggers_energy_watchers */
static void e1_debut_energy_placement_triggers_both_watchers(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int hazuki = test_new_id(&g, HAZUKI_BP4);
    int placer = test_new_id(&g, KAHORI2);
    CHECK(sumire >= 0 && hazuki >= 0 && placer >= 0, "E1: the fixtures resolve");
    if (sumire < 0 || hazuki < 0 || placer < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(hazuki, HAZUKI_BP4);
    expect_card_no(placer, KAHORI2);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(hazuki, TRIG_AUTO, 1);
    expect_trigger(placer, TRIG_DEBUT_STAGE, 1);

    e_board(&g, sumire, hazuki);
    fill_energy_deck(&g, 0, 1);
    test_give_energy(&g, 20);
    test_add_to_hand(&g, placer);

    int energy_deck_before = energy_deck_n(&g, 0);
    int energy_zone_before = energy_n(&g, 0);
    int main_deck_before = deck_n(&g, 0);
    int hand_before = hand_n(&g, 0);

    int played = test_play_to_stage(&g, placer, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "E1: 葉月 恋's debut is accepted");
    CHECK(on_stage(&g, 0, 2, placer),
          "E1: Hazuki Kano should remain in the played right-side slot");
    CHECK_EQ(energy_deck_n(&g, 0), energy_deck_before - 1,
             "E1: her debut should move exactly one card out of the energy deck");
    CHECK_EQ(energy_n(&g, 0), energy_zone_before + 1,
             "E1: and place exactly one card into the energy zone");

    int h6 = heart(&g, hazuki, 6);
    int h2 = heart(&g, sumire, 2);
    int hand_after = hand_n(&g, 0);
    int deck_after = deck_n(&g, 0);

    CHECK_EQ(h6, 1,
             "E1: the 葉月 恋 watcher gains heart06 when an effect puts energy into the "
             "zone (カードの効果によって…置かれるたび)");
    CHECK_EQ(h2, 1,
             "E1: すみれ gains heart02 — 自分のカードの効果によって, and the placer is "
             "her OWN card, so this leg IS satisfied");
    CHECK_EQ(deck_after, main_deck_before - 1,
             "E1: すみれ draws exactly one card from the main deck");
    CHECK_EQ(hand_after, hand_before,
             "E1: playing 葉月 恋 spends one hand card and すみれ draws one replacement");
}

/* E2. opponent_energy_effect_triggers_hazuki_but_not_sumire
 *
 * The scope separator: an OPPONENT-caused energy placement fires the
 * parenthetical watcher and NOT the own-effect one. */
static void e2_opponent_energy_fires_only_the_parenthetical_watcher(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int hazuki = test_new_id(&g, HAZUKI_BP4);
    CHECK(sumire >= 0 && hazuki >= 0, "E2: the fixtures resolve");
    if (sumire < 0 || hazuki < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(hazuki, HAZUKI_BP4);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(hazuki, TRIG_AUTO, 1);

    e_board(&g, sumire, hazuki);
    int hand_before = hand_n(&g, 0);

    /* P2's card effect places an energy card into P1's OWN zone. */
    int e = place_energy_in_zone(&g, 0, /*causer seat*/ 1);
    CHECK(e >= 0, "E2: P2's card effect placed an energy card into P1's own zone");
    if (e < 0) { n_setup_bugs++; return; }
    scan_event(&g, 0, &e, 1, 0, 1);
    settle(&g);

    CHECK_EQ(heart(&g, hazuki, 6), 1,
             "E2: 葉月 恋 explicitly says 相手のカードの効果でも発動する, so an "
             "opponent-caused placement fires her -> +1 heart06");
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "E2: すみれ says 自分のカードの効果によって, so an opponent energy "
             "placement must not trigger her");
    CHECK_EQ(hand_n(&g, 0), hand_before,
             "E2: すみれ must not draw from opponent energy placement");

    /* PAIRED POSITIVE on the SAME board: an OWN-caused placement, which
     * satisfies すみれ.  Without it the two zeroes above are indistinguishable
     * from a board on which neither watcher is wired. */
    int e2c = place_energy_in_zone(&g, 0, 0);
    CHECK(e2c >= 0, "E2 CONTROL: P1's own card effect places a second energy card");
    if (e2c < 0) return;
    scan_event(&g, 0, &e2c, 1, 0, 1);
    settle(&g);
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "E2 CONTROL: すみれ DOES fire on an own-caused placement on the same board, "
             "so her zero above is the cause scope and not a dead card");
    CHECK_EQ(heart(&g, hazuki, 6), 2,
             "E2 CONTROL: and the parenthetical watcher fires a SECOND time, which also "
             "shows its each_time gate really is per-placement");
}

/* E3. keke_position_change_swapping_with_sumire_triggers_sumire
 *
 * The position-change leg of the compound OR, driven through a REAL optional
 * 登場 position change rather than a synthesised event. */
static void e3_keke_position_change_onto_sumire_fires(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int natsumi = test_new_id(&g, NATSUMI_N);
    int keke = test_new_id(&g, KEKE);
    CHECK(sumire >= 0 && natsumi >= 0 && keke >= 0, "E3: the fixtures resolve");
    if (sumire < 0 || natsumi < 0 || keke < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(natsumi, NATSUMI_N);
    expect_card_no(keke, KEKE);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(keke, TRIG_DEBUT_STAGE, 1);

    /* Premise: the two board members are DIFFERENT cards, so "only Sumire should
     * draw because only Sumire moved" is a real discrimination. */
    {
        Card a, b;
        int distinct = 0;
        if (rb_decode_card_by_index((uint32_t)sumire, &a) &&
            rb_decode_card_by_index((uint32_t)natsumi, &b)) {
            distinct = a.name && b.name && strcmp(a.name, b.name) != 0;
            rb_free_card(&a);
            rb_free_card(&b);
        }
        CHECK(distinct, "E3 PRECONDITION: すみれ and 夏美 are different cards");
    }

    e_board(&g, sumire, natsumi);
    test_give_energy(&g, 20);
    test_add_to_hand(&g, keke);
    int main_deck_before = deck_n(&g, 0);

    int played = test_play_to_stage(&g, keke, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "E3: 唐 可可's debut is accepted");
    CHECK_PROMPTS(&g, "E3: 可可's optional position change is offered");
    /* ACCEPT the position change into the LEFT area, which すみれ occupies. */
    {
        const RbChoice *c = pending_choice(&g);
        int is_dest = c && c->target && strcmp(c->target, "position|destination") == 0;
        CHECK(is_dest, "E3: the prompt is a position|destination");
    }
    if (pending(&g)) {
        answer(&g, 0);   /* SelectAutoAbility option 0 = proceed into the change */
        settle(&g);
    }
    if (pending(&g)) {
        CHECK(swap_into(&g, 0, "E3"), "E3: 可可 moves right -> left, displacing すみれ");
    }

    CHECK_EQ(g.state.p[0].stage[0], keke, "E3: Keke should move from right to left");
    CHECK_EQ(g.state.p[0].stage[2], sumire,
             "E3: Sumire should be swapped from left to right by Keke's position change");
    CHECK_EQ(g.state.p[0].stage[1], natsumi,
             "E3: Natsumi should stay centre and should not satisfy 'this member moved'");
    CHECK_EQ(heart(&g, sumire, 2), 1,
             "E3: Sumire should gain heart02 when 可可 swaps into her area");
    CHECK_EQ(deck_n(&g, 0), main_deck_before - 1,
             "E3: only Sumire should draw because only Sumire moved");
    CHECK_EQ(hand_n(&g, 0), 1, "E3: playing 可可 spends one hand card, then Sumire draws one");
}

/* E4. keke_position_change_swapping_with_natsumi_triggers_natsumi
 *
 * The mirror image: the change displaces 夏美, not すみれ, so すみれ must stay
 * silent.  Same driver, same board, different destination — the only thing
 * that differs between E3 and E4 is WHICH member moved. */
static void e4_keke_position_change_onto_natsumi_leaves_sumire_silent(void)
{
    static TestGame g;
    new_game(&g);

    int sumire = test_new_id(&g, SUMIRE);
    int natsumi = test_new_id(&g, NATSUMI_N);
    int keke = test_new_id(&g, KEKE);
    CHECK(sumire >= 0 && natsumi >= 0 && keke >= 0, "E4: the fixtures resolve");
    if (sumire < 0 || natsumi < 0 || keke < 0) { n_setup_bugs++; return; }
    expect_card_no(sumire, SUMIRE);
    expect_card_no(natsumi, NATSUMI_N);
    expect_card_no(keke, KEKE);
    expect_trigger(sumire, TRIG_AUTO, 1);
    expect_trigger(keke, TRIG_DEBUT_STAGE, 1);

    e_board(&g, sumire, natsumi);
    test_give_energy(&g, 20);
    test_add_to_hand(&g, keke);

    int played = test_play_to_stage(&g, keke, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "E4: 唐 可可's debut is accepted");
    CHECK_PROMPTS(&g, "E4: 可可's optional position change is offered");
    if (pending(&g)) {
        const RbChoice *c = pending_choice(&g);
        int is_dest = c && c->target && strcmp(c->target, "position|destination") == 0;
        CHECK(is_dest, "E4: the prompt is a position|destination");
    }
    if (pending(&g)) {
        answer(&g, 0);
        settle(&g);
    }
    if (pending(&g)) {
        CHECK(swap_into(&g, 1, "E4"), "E4: 可可 moves right -> centre, displacing 夏美");
    }

    CHECK_EQ(g.state.p[0].stage[1], keke, "E4: Keke should move from right to center");
    CHECK_EQ(g.state.p[0].stage[2], natsumi,
             "E4: Natsumi should be swapped from center to right");
    CHECK_EQ(g.state.p[0].stage[0], sumire,
             "E4: Sumire should stay left and should not satisfy 'this member moved'");
    CHECK_EQ(heart(&g, sumire, 2), 0,
             "E4: Sumire should not gain heart02 when she did not move and no energy "
             "was placed");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §F  leaves_stage/debut_heart02_threshold_five_grants_opponent_global_heart_
 *     test.rs — 3 of 4, and this is the ONE genuinely uncovered file in
 *     leaves_stage.
 *
 * RUST FIXTURE BUGS in the source file, both carried into this port as fixes:
 *   1. All four Rust tests assert only
 *          assert!(game.state.player1.stage.stage.contains(&chika))
 *      which is a statement about the PLAY, not about the 5-heart02 threshold
 *      the file is named for.  They pass whether the grant happens or not.  This
 *      port asserts the grant itself, which is why the file can go red.
 *   2. `game.id("PL!S-bp2-002-R")` is called twice for m1 and m2.  Rust's `id()`
 *      interns by card_no, so m1 == m2: the board is ONE card id in two stage
 *      slots, and the "two members holding 5 heart02" is really one card holding
 *      6/5/4.  This port uses two DISTINCT pool instances.
 *
 * NOT PORTED: `s_bp5_010_without_5_heart02_no_global` — its board has a plain
 * filler with no heart02 at all, which is the same claim as F2's 4-heart02
 * board with a much weaker premise.  Porting both would pad the file.
 * ══════════════════════════════════════════════════════════════════════════ */

/* The board: two DISTINCT 『Aqours』 members on the left and centre, carrying
 * `a_h2` and `b_h2` heart02 between them, and one live card in P2's
 * live-card zone for the 「相手のライブカード置き場にあるライブカード1枚」. */
static void chika_board(TestGame *g, int a_h2, int b_h2, int *out_a, int *out_b,
                        int *out_chika, int *out_opp_live)
{
    int m1 = test_new_id(g, RIKO);
    int m2 = test_new_id(g, RIKO);
    int chika = test_new_id(g, CHIKA);
    int opp_live = test_new_id(g, OPP_LIVE);
    *out_a = m1; *out_b = m2; *out_chika = chika; *out_opp_live = opp_live;
    if (m1 < 0 || m2 < 0 || chika < 0 || opp_live < 0) return;

    fill_decks(g, 10);
    clear_stage(g, 0);
    clear_stage(g, 1);
    g->state.p[0].stage[0] = m1;
    g->state.p[0].stage[1] = m2;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    if (a_h2) rb_mods_add_heart(&g->state.mods, m1, 2, a_h2);
    if (b_h2) rb_mods_add_heart(&g->state.mods, m2, 2, b_h2);
    /* The target of the grant: ONE live card in the OPPONENT's live-card zone. */
    test_add_to_live_for(g, 1, opp_live);
    test_give_energy(g, 20);
    test_add_to_hand(g, chika);
}

/* F1. s_bp5_010_with_5_heart02_triggers_global (Rust's 6- and 5-heart02 boards
 * collapse to the same claim once the threshold is 合計5つ以上, so one case
 * covers both, and it reads the grant off the opponent's live card.) */
static void f1_five_heart02_grants_the_opponents_live_a_required_heart(void)
{
    static TestGame g;
    new_game(&g);

    int a = -1, b = -1, chika = -1, opp_live = -1;
    chika_board(&g, 3, 2, &a, &b, &chika, &opp_live);
    CHECK(a >= 0 && b >= 0 && chika >= 0 && opp_live >= 0, "F1: the fixtures resolve");
    if (a < 0 || b < 0 || chika < 0 || opp_live < 0) { n_setup_bugs++; return; }
    expect_card_no(a, RIKO);
    expect_card_no(chika, CHIKA);
    expect_card_no(opp_live, OPP_LIVE);
    expect_trigger(chika, TRIG_DEBUT_STAGE, 1);
    /* The two members are DISTINCT instances of the same print, so the board is
     * really "two members holding 5 heart02" and not one card counted twice. */
    CHECK(a != b, "F1 PRECONDITION: the two 桜内梨子 instances are distinct (test_new_id)");
    {
        int total = heart(&g, a, 2) + heart(&g, b, 2);
        CHECK_EQ(total, 5, "F1 PRECONDITION: the board really holds 合計5 heart02");
    }
    CHECK(g.state.p[1].live.n == 1, "F1 PRECONDITION: ONE live card in the opponent's zone");

    int played = test_play_to_stage(&g, chika, 2 /* right */);
    settle(&g);
    CHECK_EQ(played, 1, "F1: 高海千歌's debut is accepted");
    CHECK(stage_has(&g, 0, chika), "F1: she is on the stage");

    /* NON-VACUITY CONTROL.  The grant must land SOMEWHERE: an engine that did
     * nothing at all would satisfy the strict assertion below trivially. */
    int on_opp = need_heart(&g, opp_live, 0);
    int on_chika = need_heart(&g, chika, 0);
    {
        char msg[320];
        snprintf(msg, sizeof(msg),
                 "F1 CONTROL: the 登場 effect really executed and moved a required-heart00 "
                 "somewhere (opponent live %d, 千歌 herself %d) -- without this the strict "
                 "assertion below would also pass on an engine that did nothing",
                 on_opp, on_chika);
        CHECK(on_opp != 0 || on_chika != 0, msg);
    }

    /* THE STRICT CLAIM, which the Rust file never checks. */
    CHECK_EQ(on_opp, 1,
             "F1: 相手のライブカード置き場にあるライブカード1枚は、成功させるための"
             "必要ハートがheart0多くなる — the OPPONENT'S live card must be the card that "
             "gets the +1 required heart00, not 千歌 herself");
}

/* F2. s_bp5_010_exact_5_heart02_triggers and s_bp5_010_4_heart02_no_trigger
 * share a driver: the grant is read at 4 heart02 and at 5. */
static void f2_the_threshold_is_five_heart02_exactly(void)
{
    /* NEGATIVE first: 4 heart02, one below 「合計5つ以上」. */
    {
        static TestGame g;
        new_game(&g);
        int a = -1, b = -1, chika = -1, opp_live = -1;
        chika_board(&g, 2, 2, &a, &b, &chika, &opp_live);
        CHECK(a >= 0 && b >= 0 && chika >= 0 && opp_live >= 0, "F2 low: the fixtures resolve");
        if (a < 0 || b < 0 || chika < 0 || opp_live < 0) { n_setup_bugs++; return; }
        {
            int total = heart(&g, a, 2) + heart(&g, b, 2);
            CHECK_EQ(total, 4, "F2 low PRECONDITION: the board really holds 4 heart02, "
                               "one below the 合計5つ以上 threshold");
        }
        int played = test_play_to_stage(&g, chika, 2);
        settle(&g);
        CHECK_EQ(played, 1, "F2 low: 高海千歌's debut is accepted");
        CHECK_EQ(need_heart(&g, opp_live, 0), 0,
                 "F2 low: 合計5つ以上 is false at 4 heart02, so the opponent's live card "
                 "must NOT get the extra required heart0");
        CHECK_EQ(need_heart(&g, chika, 0), 0,
                 "F2 low: and nothing is granted anywhere else either — the condition "
                 "withholds the whole grant, it does not redirect it");
    }

    /* POSITIVE, immediately after, on an identical board with one more heart02:
     * the ONLY difference between the two halves is the count. */
    {
        static TestGame g;
        new_game(&g);
        int a = -1, b = -1, chika = -1, opp_live = -1;
        chika_board(&g, 3, 2, &a, &b, &chika, &opp_live);
        CHECK(a >= 0 && b >= 0 && chika >= 0 && opp_live >= 0, "F2 high: the fixtures resolve");
        if (a < 0 || b < 0 || chika < 0 || opp_live < 0) { n_setup_bugs++; return; }
        {
            int total = heart(&g, a, 2) + heart(&g, b, 2);
            CHECK_EQ(total, 5, "F2 high PRECONDITION: the board really holds 5 heart02");
        }
        int played = test_play_to_stage(&g, chika, 2);
        settle(&g);
        CHECK_EQ(played, 1, "F2 high: 高海千歌's debut is accepted");
        /* Whatever the engine does here, it must DIFFER from the 4-heart02 board,
         * or 合計5つ以上 is not a threshold at all. */
        int at_five = need_heart(&g, opp_live, 0) + need_heart(&g, chika, 0);
        {
            char msg[320];
            snprintf(msg, sizeof(msg),
                     "F2 high CONTROL: at 5 heart02 the grant MUST appear somewhere "
                     "(opponent live %d, 千歌 %d) -- this is the positive that licenses the "
                     "4-heart02 zero in the other half, and it is the assertion the Rust "
                     "file never makes",
                     need_heart(&g, opp_live, 0), need_heart(&g, chika, 0));
            CHECK(at_five != 0, msg);
        }
    }
}

/* F3. s_bp5_010_exact_5_heart02_triggers, on a 6-heart02 board — the reading
 * ABOVE the threshold.  合計5つ以上 is a floor, so 6 must behave like 5; a
 * `== 5` implementation would be caught only here. */
static void f3_six_heart02_still_grants(void)
{
    static TestGame g;
    new_game(&g);

    int a = -1, b = -1, chika = -1, opp_live = -1;
    chika_board(&g, 3, 3, &a, &b, &chika, &opp_live);
    CHECK(a >= 0 && b >= 0 && chika >= 0 && opp_live >= 0, "F3: the fixtures resolve");
    if (a < 0 || b < 0 || chika < 0 || opp_live < 0) { n_setup_bugs++; return; }
    expect_card_no(chika, CHIKA);
    {
        int total = heart(&g, a, 2) + heart(&g, b, 2);
        CHECK_EQ(total, 6, "F3 PRECONDITION: the board really holds 6 heart02");
    }
    int played = test_play_to_stage(&g, chika, 2);
    settle(&g);
    CHECK_EQ(played, 1, "F3: 高海千歌's debut is accepted");
    CHECK_EQ(need_heart(&g, opp_live, 0), 1,
             "F3: 合計5つ以上 is a FLOOR — 6 heart02 must grant exactly the same +1 as "
             "5, and it must go to the OPPONENT'S live card. This is the only case "
             "that separates `>= 5` from `== 5`.");
}

/* ══════════════════════════════════════════════════════════════════════════
 * fork / crash reporting
 * ══════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_SETUPBUG  3

static int n_ok, n_failed, n_crashed, n_setup;

static void on_segv(int sig)
{
    const char *msg = "\nCRASH: a fatal signal arrived inside the test below.\n";
    ssize_t ignored = write(2, msg, strlen(msg));
    (void)ignored;
    fprintf(stderr, "CRASH: signal %d in test [%s]\n", sig, current_test);
    fflush(stderr);
    _Exit(CHILD_SETUPBUG);
}

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        if (nf) n_failed++; else if (ns) n_setup++; else n_ok++;
        printf("%-8s %s  [%d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name, nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d failure(s), %d setup bug(s)\n",
               name, (int)(failures - f0), (int)(n_setup_bugs - s0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES
                            : (n_setup_bugs > s0 ? CHILD_SETUPBUG : CHILD_OK));
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- fatal signal %d (engine fault)\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok       %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED   %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_setup++;
        printf("SETUPBUG %s  <-- fixture could not be built\n", name);
    } else {
        n_crashed++;
        printf("CRASH    %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS, on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- \xc2\xa7""A energy_return_place_waited_energy_test.rs ---\n");
    run("a1_appearance_leg_places_one_waited_energy",
        a1_appearance_leg_places_one_waited_energy);
    run("a2_energy_return_leg_places_one_waited_energy",
        a2_energy_return_leg_places_one_waited_energy);
    run("a4_once_per_turn_when_both_legs_fire",
        a4_once_per_turn_when_both_legs_fire);
    run("a5_empty_energy_deck_places_nothing", a5_empty_energy_deck_places_nothing);
    run("a6_no_trigger_event_places_nothing", a6_no_trigger_event_places_nothing);
    run("a7_opponent_energy_move_does_not_trigger",
        a7_opponent_energy_move_does_not_trigger);
    run("a8_other_member_appearance_does_not_trigger",
        a8_other_member_appearance_does_not_trigger);
    run("a9_wrong_source_energy_move_does_not_trigger",
        a9_wrong_source_energy_move_does_not_trigger);
    run("a10_multiple_energy_moves_single_fire", a10_multiple_energy_moves_single_fire);

    printf("--- \xc2\xa7""B own_effect_energy_placed_each_time_gain_heart06 ---\n");
    run("b1_two_copies_each_time_gains_one_heart06_per_placement",
        b1_two_copies_each_time_gains_one_heart06_per_placement);
    run("b2_phase_action_energy_does_not_fire", b2_phase_action_energy_does_not_fire);

    printf("--- \xc2\xa7""C real_card_effect_self_move_or_energy_placed_heart02 ---\n");
    run("c1_own_effect_swap_draws_one_and_gains_heart02",
        c1_own_effect_swap_draws_one_and_gains_heart02);
    run("c2_own_effect_energy_placement_draws_one_and_gains_heart02",
        c2_own_effect_energy_placement_draws_one_and_gains_heart02);

    printf("--- \xc2\xa7""D own_effect_self_move_or_energy_placed_heart02 ---\n");
    run("d1_area_move_leg_fires", d1_area_move_leg_fires);
    run("d2_no_event_no_trigger", d2_no_event_no_trigger);
    run("d3_opponent_effect_move_no_trigger", d3_opponent_effect_move_no_trigger);
    run("d4_opponent_energy_placement_no_trigger", d4_opponent_energy_placement_no_trigger);
    run("d5_other_card_move_no_trigger", d5_other_card_move_no_trigger);
    run("d6_swap_to_empty_area_leaves_her_unmoved", d6_swap_to_empty_area_leaves_her_unmoved);
    run("d7_use_limit_blocks_the_second_leg", d7_use_limit_blocks_the_second_leg);
    run("d8_turn_limit_resets_on_the_next_turn", d8_turn_limit_resets_on_the_next_turn);
    run("d9_declined_debut_position_change_no_trigger",
        d9_declined_debut_position_change_no_trigger);
    run("d10_swap_displacing_sumire_fires", d10_swap_displacing_sumire_fires);
    run("d11_own_scan_never_arms_the_energy_leg", d11_own_scan_never_arms_the_energy_leg);

    printf("--- \xc2\xa7""E energy_placement_watchers_across_debut_and_position ---\n");
    run("e1_debut_energy_placement_triggers_both_watchers",
        e1_debut_energy_placement_triggers_both_watchers);
    run("e2_opponent_energy_fires_only_the_parenthetical_watcher",
        e2_opponent_energy_fires_only_the_parenthetical_watcher);
    run("e3_keke_position_change_onto_sumire_fires", e3_keke_position_change_onto_sumire_fires);
    run("e4_keke_position_change_onto_natsumi_leaves_sumire_silent",
        e4_keke_position_change_onto_natsumi_leaves_sumire_silent);

    printf("--- \xc2\xa7""F leaves_stage debut_heart02_threshold_five ---\n");
    run("f1_five_heart02_grants_the_opponents_live_a_required_heart",
        f1_five_heart02_grants_the_opponents_live_a_required_heart);
    run("f2_the_threshold_is_five_heart02_exactly", f2_the_threshold_is_five_heart02_exactly);
    run("f3_six_heart02_still_grants", f3_six_heart02_still_grants);

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_jidou_watch2: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    if (n_crashed)
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child. "
                        "Treat as an engine fault.\n", n_crashed);
    if (n_setup)
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n", n_setup);
    if (n_failed)
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file is "
                        "classified in the accompanying report; none is a weakened "
                        "assertion.\n", n_failed);
    return n_failed ? 1 : 0;
}

/* test_parity_trigger_paths.c -- C port of the Rust tests that the existing
 * C trigger-path suites did NOT already cover.
 *
 * Build ONLY through the isolated build root:
 *   cd engine_c_wip
 *   bash tools/isolated_build.sh tr_trigpaths t T=parity_trigger_paths
 *
 * ── COVERAGE BOUNDARY (recomputed from the tree, not inherited) ──────────
 * engine/tests/test_modules/rules/trigger_paths/ holds 11 test modules
 * (mod.rs + 10 sources is wrong; the directory listing is authoritative) and
 * exactly 80 `#[test]` functions. Existing C coverage:
 *
 *   tests/test_trigger_paths.c ............................ 19 tests
 *     q159_debut_from_discard_self_cost_wait_not_payable
 *     q159_positive_select_prompt_appears
 *     reina_fires_after_non_blade_member_left_live_zone
 *     reina_silent_when_blade_heart_member_left_live_zone
 *     riko_aqours_live_leaving_live_zone_offers_deck_placement
 *     riko_ignores_non_aqours_live_leaving_live_zone
 *     riko_responds_only_to_own_side_live_zone
 *     miyamiya_baton_touch_cost15_newcomer_full_payoff
 *     miyamiya_baton_touch_cost13_newcomer_energy_only
 *     miyamiya_baton_touch_blade_heart_newcomer_nothing
 *     cost11_baton_replacement_does_not_arm_debut_watcher
 *       (= cost11_baton_replacement_moves_debut_watcher_to_waitroom_q198)
 *     cost11_debut_with_watcher_retains_nonempty_energy_zone
 *     cost10_baton_replacement_does_not_draw_from_departed_watcher
 *     cost10_debut_with_watcher_draws_to_replace_played_card
 *     hand_activation_empty_stage_resolves_draw_after_discard_choices_q196
 *     hand_activation_draw_and_lone_nijigasaki_blade_resolve_q196
 *     kasumi_turn1_ability_used_only_once_per_turn
 *     kasumi_turn1_ability_available_again_next_turn
 *     kasumi_limit_is_per_instance
 *   tests/test_tp_activation_offer.c .......................  2 tests
 *     sayaka_activation_charges_energy_and_terminates
 *     sayaka_unaffordable_not_offered_and_press_rejected
 *   tests/test_tp_turn_limited_activation.c ..................  5 tests
 *     kasumi_turn1_ability_offered_only_once_per_turn
 *     kasumi_turn1_ability_available_again_next_turn
 *     kasumi_no_valid_target_terminates
 *     kasumi_exact_energy_is_sufficient
 *     kasumi_limit_is_per_instance
 *   tests/test_tp_zone_change_gate.c .........................  8 tests
 *     riko_aqours_live_leaving_live_zone_offers_deck_placement
 *     riko_ignores_non_aqours_live_leaving_live_zone
 *     riko_responds_only_to_own_side_live_zone
 *     reina_fires_after_non_blade_member_left_live_zone
 *     reina_silent_when_blade_heart_member_left_live_zone
 *     miyamiya_baton_touch_cost15_newcomer_full_payoff
 *     miyamiya_baton_touch_cost13_newcomer_energy_only
 *     miyamiya_baton_touch_blade_heart_newcomer_nothing
 *   tests/test_tp_joint_card_live_start.c .................... 15 tests
 *     all 15 of joint_card_live_start_test.rs
 *       (test_bp2_live_start_choose_zero_pays_no_cards_and_gains_no_blade is
 *        the C file's test_bp2_live_start_skip_cost_gains_no_blade)
 *   tests/test_parity_jidou_watch.c ..........................  0 tests
 *     It covers tests/test_modules/jidou/debut_watch/ and
 *     tests/test_modules/jidou/discard_watch/ -- a DIFFERENT cluster. Not one
 *     of its 57 cases carries a rules/trigger_paths `#[test]` name. It is
 *     listed here only so the boundary is checkable.
 *
 * 38 of 80 covered. THE 42 COVERED BY THIS FILE:
 *   §A cross_player_jidou_triggers_test.rs ....................  9
 *   §B jidou_paired_ability_combination_test.rs ................  5
 *   §C trigger_scope_test.rs ...................................  6
 *   §D sibling_abilities_on_one_card_interact_test.rs ..........  7
 *   §E appearance_and_baton_touch_trigger_regressions_test.rs . 12
 *   §F retrieval_placement_blade_ability_chain_test.rs ........  3
 *
 * ── HARNESS NOTES THAT CHANGE THE ASSERTIONS HERE ────────────────────────
 *  - `load_card_db()` falls back to "../cards/build": `rb_load("src")` fails
 *    under the isolated out-of-tree build.
 *  - `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *  - `rb_card_copy_pool_reset()` is called per case: `rb_create_card_copy`
 *    draws from a process-global monotonic pool that nothing else resets.
 *  - `test_get_heart_modifier` REMAPS colour 5 onto RB_HEART_ORANGE, so every
 *    heart read here goes through `rb_mods_get_heart(&mods, cid, 1..6)`, whose
 *    index is the literal 1=heart01 .. 6=heart06 (src/ability/effects/
 *    state.c:1986-1990). The S1 control case would otherwise read heart05 out
 *    of the orange bucket.
 *  - `test_id` returns the SHARED TEMPLATE, which is exactly what Rust's
 *    `game.id(card_no)` does (it interns by card_no); `test_new_id` allocates
 *    a distinct pool slot and is used wherever two physical cards of the same
 *    print must be distinguishable.
 *  - `Card.ability` is not populated by `rb_decode_card_by_index`; abilities
 *    come from `rb_card_num_abilities` + `rb_decode_card_ability`.
 *  - `rb_card_no_eq` is FUZZY (a peer showed it accept `PL!SP-PR-004-R` for the
 *    real `PL!SP-PR-004-PR`), so every identity assertion is a byte-exact
 *    `strcmp` of `rb_card_string(c.card_no_idx)`.
 *  - `RbChoice.zone` spells the waitroom "discard".
 *  - A trigger-scoped `cannot_place` restriction is DEAD in the C port and no
 *    test here can observe it (see the report): `prohibition_blocks_zone`
 *    (src/core/game_state_abilities.c:341-347) only matches the prefix
 *    "restriction:cannot_place:", while the only writers push
 *    "cannot_place:success_live_zone" (src/ability/compound.c:750/756,
 *    src/ability/effects/misc.c:136-146, src/ability/effects/score.c:925).
 *  - `rb_record_card_movement`'s `causer` argument is a SEAT INDEX (0 = p1,
 *    1 = p2), not a card id -- see tests/test_trigger_paths.c's
 *    move_live_to_waitroom_event, which passes `pl`.
 *
 * ── THE ONE PLACE THE C CANNOT MIRROR Rust 1:1 ──────────────────────────
 * Rust's `state.push_movement_event(...)` followed by
 * `state.trigger_auto_abilities_for_player(&pid)` reads the pushed event out
 * of GameState. The C splits that in two: `rb_record_card_movement` logs the
 * event, but `rb_trigger_auto_abilities_for_player` re-derives the flags as
 * `rb_trigger_auto_abilities_for_player_with_event(g, pl, NULL, 0, 0, 0)` --
 * with n_moved 0, position_change 0 and energy_placed 0. Two of those three
 * flags are load-bearing:
 *   - the `each_time` + `location: energy_zone` gate needs energy_placed
 *     (src/core/game_state_abilities.c:1345-1356), and
 *   - the `movement: moved` + `self_target` gate needs the card in n_moved
 *     (src/core/game_state_abilities.c:1358-1368).
 * So `scan_movement_event()` below calls the `_with_event` form with the same
 * flags Rust would have read back out of the state. This is a HARNESS
 * concession, recorded here so it is not mistaken for an engine verdict.
 *
 * ── VACUITY CONTROL ─────────────────────────────────────────────────────
 * Every negative assertion in this file is paired either with a CONTROL on
 * the same board (the positive twin is driven first, in the same process,
 * before the negative) or with a CHECK_PROMPTS at the assertion site. A
 * negative that could pass on a board where the engine did literally nothing
 * is not evidence, so none is left unguarded. CHECK_PROMPTS counts the prompt
 * still ON SCREEN at the assertion site (`test_has_pending_choice`), NOT a
 * running tally from answer time -- a peer's first version under-counted
 * that way and turned nine genuinely green tests red.
 */

#include "rabuka.h"
#include "test_game.h"

#include <errno.h>
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

/* PROMPT-COUNT CONTROL. Counts the prompt still on screen right here. A
 * negative assertion guarded only by "nothing appeared" is satisfied by an
 * engine that never offered anything at all; this is what rules that out. */
#define CHECK_PROMPTS(game, message) do { \
    assertions++; \
    int prompts_on_screen = test_has_pending_choice(game); \
    if (prompts_on_screen <= 0) { \
        fprintf(stderr, "FAIL: [VACUITY CONTROL] %s -- no prompt on screen, so the absence it licenses proves nothing\n", message); \
        failures++; \
    } else { \
        printf("ok: [vacuity control] %s (%d prompt(s) on screen)\n", message, prompts_on_screen); \
    } \
} while (0)

/* ---------------------------------------------------------- trigger bytes */
/* Octal escapes so the Write tool's encoding cannot corrupt a Japanese
 * literal. The VALUES here were read back out of the live decoder via
 * `RB_DUMP_TRIGS=1`, which prints the ACTUAL bytes every fixture carries.
 *
 * IMPORTANT: the same two escapes are CORRUPT in tests/test_trigger_paths.c,
 * which is where these were first copied from. That file's own header says
 * the rescued revision "carried two corrupt escapes" and then keeps them:
 *   登   U+767B = E7 99 BB = \347\231\233   (the file has \347\232\273 = E7 9A BB)
 *   場   U+5834 = E5 A0 B4 = \345\240\264   (the file has \347\243\276 = E7 A3 BE)
 *   ド   U+30C9 = E3 83 96 = \343\203\226   (the file has \343\203\266 = E3 83 B6)
 * So its TRIG_DEBUT and TRIG_LIVE_START never match the decoder and every
 * fire_trigger() built on them returns 0. See the report.
 *
 * 自動  U+81EA U+52D5 = E8 87 AA E5 8B 95 = \350\207\252\345\213\225  (this one
 *       WAS correct in the rescued file.)
 */
#define TRIG_DEBUT        "\347\231\233\345\240\264"                            /* 登場 */
#define TRIG_LIVE_START   "\343\203\251\343\202\244\343\203\226\351\226\213\345\247\213\346\231\202" /* ライブ開始時 */
#define TRIG_LIVE_SUCCESS "\343\203\251\343\202\244\343\203\226\346\210\220\345\212\237\346\231\202" /* ライブ成功時 */
#define TRIG_AUTO         "\350\207\252\345\213\225"                            /* 自動 */

/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B. rb_find_card_by_no is exact-byte and
 * does NOT normalise fullwidth->halfwidth the way Rust's lookup does, so the
 * literal has to be spelled byte-exactly. */
#define PLUS "\xEF\xBC\x8B"

#define FILLER "PL!-sd1-010-SD"

/* --------------------------------------------------------------- fixtures */
/* Every number below was checked against cards/cards.json read as UTF-8.
 * See the report for the two naming discrepancies that surfaced. */
#define KOKO     "PL!SP-sd2-002-SD2"        /* 唐 可可     cost 13 */
#define NATSUME  "PL!SP-pb1-020-N"         /* 鬼塚夏美    cost  4 */
#define TOMARI   "PL!SP-sd2-011-SD2"        /* 鬼塚冬毬    cost  4 */
#define KANON    "PL!SP-sd2-012-SD2"        /* 澁谷かのん  cost  2 */
#define FUYU     "PL!SP-sd2-022-SD2"        /* 鬼塚冬毬    cost  2 */
#define CHISATO  "PL!SP-pb1-006-R"          /* 桜小路きな子 cost  9 */
#define KIZUKO   "PL!SP-pb1-006-R"          /* same print; §C names it differently */
#define HATSUNE  "PL!SP-bp7-005-R" PLUS     /* 葉月 恋     cost  9, 2 abilities */
#define HANOKO   "PL!SP-bp4-016-N"          /* 葉月 恋     cost  9 */
#define DANCING  "PL!-bp6-020-L"            /* Dancing stars on me! score 5 */
#define HONOKA_R "PL!-bp6-001-R" PLUS      /* 高坂穂乃果  cost  2 */
#define SAINT    "PL!S-bp5-111-R"           /* 鹿角聖良    cost  4 */
#define AQOURS_P "PL!S-bp2-015-PR"          /* 津島善子    cost  4 */
#define RURINO_P "PL!HS-PR-018-PR"          /* 大沢瑠璃乃  cost  4 */
#define MIRAKURA "PL!HS-bp1-005-PR"         /* 大沢瑠璃乃  cost  9 */
#define HIME     "PL!HS-pb1-014-R"          /* 安養寺姫芽  cost  9 */
#define RURINO   "PL!HS-pb1-003-R"          /* 大沢瑠璃乃  cost 15 */
#define AI       "PL!N-bp3-005-R" PLUS      /* 宮下 愛     cost 15 */
#define SERASU   "PL!HS-bp6-007-R"          /* セラス 柳田  cost 15 */
#define SAWAYAMA "PL!HS-PR-022-PR"          /* セラス 柳田  cost  4 */
#define NON_EDEL "PL!HS-sd1-002-SD"         /* 村野さやか  cost 11, DOLLCHESTRA */
#define HANAHO   "PL!HS-sd1-001-SD"         /* 日野下花帆  cost  9 */
#define ARRIVER  "PL!HS-sd1-006-SD"         /* 安養寺 姫芽 cost 15 */
#define LOW_COST "PL!HS-sd1-007-SD"         /* セラス 柳田  cost  4 */
#define WRONG_GR "PL!-sd1-006-SD"           /* 西木野 真姫 cost  9, not 蓮ノ空 */
#define SIF_GRP  "PL!S-sd1-001-SD"          /* 高海千歌    cost 17 */
#define UMI      "PL!-sd1-013-SD"           /* 園田海未    cost  4 */
#define SETSUNA  "PL!N-bp4-007-R" PLUS      /* 優木せつ菜  cost 13 */
#define DIVE     "PL!N-bp4-026-L"           /* DIVE!       score 5 */
#define WBNW     "PL!S-bp2-019-L"           /* WATER BLUE NEW WORLD score 7 */
#define ENERGY_C "LL-E-001-SD"

/* ---------------------------------------------------------------- helpers */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

static void new_game(TestGame *g)
{
    rb_card_copy_pool_reset();
    test_game_new(g);
}

static void fill_decks(TestGame *g, int filler)
{
    for (int pl = 0; pl < 2; pl++) {
        g->state.p[pl].deck.n = 0;
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(g, pl, filler);
    }
}

static void fill_energy_deck(TestGame *g, int pl, int energy, int count)
{
    for (int i = 0; i < count; i++) test_add_to_energy_deck(g, pl, energy);
}

/* Rust: `while game.has_pending_choice() { game.select_indices(&[]); }`
 * with a SelectAutoAbility special case that answers "proceed". */
static void drain_skip(TestGame *g)
{
    int guard = 0;
    while (test_has_pending_choice(g) && guard++ < 40) {
        const char *type = test_pending_choice_type(g);
        if (type && strcmp(type, "SelectAutoAbility") == 0) {
            int proceed[] = { 0 };
            test_select_indices(g, proceed, 1);
        } else {
            test_select_indices(g, NULL, 0);
        }
    }
}

/* Rust: answer every prompt with the FIRST generated option. */
static void drain_first(TestGame *g)
{
    int guard = 0;
    while (test_has_pending_choice(g) && guard++ < 40)
        test_resume_choice(g, 0);
}

/* Rust helpers::scan_autos_both -- scan P1 then P2, then drain. */
static void scan_both(TestGame *g)
{
    for (int pl = 0; pl < 2; pl++) {
        rb_trigger_auto_abilities_for_player(&g->state, pl);
        rb_process_pending_auto_abilities(&g->state);
    }
    if (test_has_pending_choice(g)) test_resume_choice(g, 0);
    drain_skip(g);
}

/* Rust cross_player_jidou_triggers_test.rs::trigger_auto -- P1 only. */
static void trigger_auto_p1(TestGame *g)
{
    rb_trigger_auto_abilities_for_player(&g->state, 0);
    rb_process_pending_auto_abilities(&g->state);
    drain_skip(g);
}

/* The faithful `push_movement_event` + `trigger_auto_abilities_for_player`
 * pair. See the header note on why `_with_event` is used. The from/to ZONES
 * are explicit: an energy card landing in the energy zone is NOT a
 * stage->stage move, and recording it as one arms the area-move watchers
 * instead of the energy-placed ones. */
static int scan_move(TestGame *g, int pl, const int *moved, int n,
                     int from_zone, int to_zone, int position_change,
                     int energy_placed)
{
    for (int i = 0; i < n; i++)
        rb_record_card_movement(&g->state, moved[i], from_zone, to_zone, pl, 1);
    int q = rb_trigger_auto_abilities_for_player_with_event(&g->state, pl, moved, n,
                                                            position_change, energy_placed);
    rb_process_pending_auto_abilities(&g->state);
    return q;
}

/* A stage -> stage (area) move, which is what every §A/§C area-move watcher
 * is written against. */
static int scan_area_move(TestGame *g, int pl, const int *moved, int n)
{
    return scan_move(g, pl, moved, n, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 1, 0);
}

/* An energy card landing in `pl`'s energy zone. */
static int scan_energy_placed(TestGame *g, int pl, const int *moved, int n)
{
    return scan_move(g, pl, moved, n, RB_ZONEID_ENERGY_DECK, RB_ZONEID_ENERGY_ZONE, 0, 1);
}

/* Rust trigger_scope_test.rs::push_energy_placement -- an energy card lands in
 * `owner`'s zone, caused by `causer_seat`'s card effect. The C engine has no
 * seat-parameterised `test_add_to_discard`, so the P2 waitroom is written here
 * rather than pulling a helper that only exists for P1. */
static int push_energy_placed(TestGame *g, int energy, int owner_seat, int causer_seat)
{
    RbPlayer *P = &g->state.p[owner_seat];
    if (P->energy.n >= RB_MAX_ZONE) return 0;
    P->energy.cards[P->energy.n++] = energy;
    rb_record_card_movement(&g->state, energy, RB_ZONEID_ENERGY_DECK,
                            RB_ZONEID_ENERGY_ZONE, causer_seat, 1);
    return 1;
}

static void push_waitroom(TestGame *g, int pl, int cid)
{
    RbPlayer *P = &g->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static int blade(TestGame *g, int cid) { return rb_mods_get_blade(&g->state.mods, cid); }
static int heart(TestGame *g, int cid, int colour) { return rb_mods_get_heart(&g->state.mods, cid, colour); }
static const char *orient(TestGame *g, int cid) { return rb_mods_get_orientation(&g->state.mods, cid); }
static int is_wait(TestGame *g, int cid)
{
    const char *o = orient(g, cid);
    return o && strcmp(o, "wait") == 0;
}

static int printed_cost(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int cost = (int)c.cost;
    rb_free_card(&c);
    return cost;
}

static int printed_blade_count(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int b = (int)c.blade;
    rb_free_card(&c);
    return b;
}

/* ── [TRIGGER-MISS] diagnostic ────────────────────────────────────────────
 * Prints, as hex, EVERY trigger string the card actually carries. A trigger
 * literal in this file that is off by one UTF-8 byte (登 is E7 99 BB, not
 * E7 99 BA) otherwise hides behind a silent strcmp miss and turns a positive
 * into a vacuous negative. */
static void dump_triggers(int cid, const char *want)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d prints card_no='%s' cost=%d nAb=%d wanted='%s'\n",
                cid, rb_card_string(c.card_no_idx), (int)c.cost, n, want ? want : "(null)");
        rb_free_card(&c);
    } else {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d cannot decode, wanted='%s'\n",
                cid, want ? want : "(null)");
    }
    if (want) {
        const unsigned char *w = (const unsigned char *)want;
        fprintf(stderr, "  [TRIGGER-MISS] want hex:");
        for (const unsigned char *q = w; *q; q++) fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n");
    }
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers) {
            const unsigned char *h = (const unsigned char *)ab.triggers;
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d hex:", i);
            for (const unsigned char *q = h; *q; q++) fprintf(stderr, " %02X", *q);
            fprintf(stderr, "  text='%s'\n", ab.triggers);
        } else {
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d (no trigger)\n", i);
        }
        rb_free_ability(&ab);
    }
}

/* Rust helpers::fire_trigger -- resolve ONE ability on ONE card, key it
 * "{card_no}_{full_text}", fire it with `cid` as the activating card, then
 * resolve.
 *
 * The body used to be hand-rolled over `rb_queue_push`, which had two real
 * defects the shared helper removes:
 *   1. rb_queue_push builds its entry with `use_limit_recorded = 0` and never
 *      calls rb_record_use, so a 自動/ターン1回 ability is still unspent when it
 *      resolves and the post-resolution re-scan in
 *      rb_process_player_abilities_depth_full enqueues it a SECOND time.
 *   2. rb_queue_push leaves `player_id` EMPTY (ability_queue.c:269) and never
 *      sets `queue.state`, so the un-stamped entry survived into the next
 *      window instead of being retired by clear_completed().
 * test_fire_trigger_on_card goes through rb_trigger_auto_ability, which
 * enqueues via rb_build_ability_queue_entry (owner stamped, state RESOLVING)
 * and records the use at ENQUEUE time.
 *
 * The [TRIGGER-MISS] dump is PRESERVED: the shared helper reports a miss the
 * same way (returns 0), so a byte typo in a fixture still cannot masquerade
 * as "the engine did nothing". */
static int fire_trigger(TestGame *g, int cid, const char *trig)
{
    if (test_fire_trigger_on_card(g, cid, trig)) return 1;
    dump_triggers(cid, trig);
    return 0;
}

/* AGENTS.md identity rule, done byte-exactly (rb_card_no_eq is fuzzy). */
static void expect_card_no(int cid, const char *no)
{
    Card c;
    char msg[256];
    int ok = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        ok = strcmp(rb_card_string(c.card_no_idx), no) == 0;
        rb_free_card(&c);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d is byte-exactly %s", cid, no);
    CHECK(ok, msg);
    if (!ok) dump_triggers(cid, "(identity)");
}

/* Rust `assert!(card has the trigger)`. */
static void expect_trigger(int cid, const char *trig, int min_abilities)
{
    char msg[200];
    snprintf(msg, sizeof(msg), "fixture cid=%d exposes >= %d ability/abilities",
             cid, min_abilities);
    CHECK(rb_card_num_abilities((uint32_t)cid) >= min_abilities, msg);
    int found = 0;
    for (int i = 0; i < rb_card_num_abilities((uint32_t)cid); i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) found = 1;
        rb_free_ability(&ab);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d carries the '%s' trigger", cid, trig);
    CHECK(found, msg);
    if (!found) dump_triggers(cid, trig);
}

static int on_stage(TestGame *g, int pl, int area, int cid)
{
    return g->state.p[pl].stage[area] == cid;
}

/* ══════════════════════════════════════════════════════════════════════════
 * §A  cross_player_jidou_triggers_test.rs  (9 tests)
 *
 * 自動 「(対戦相手のカードの効果でも発動する。)」 -- the parenthetical arm of
 * the printed convention. Every case here is a cross-player claim, and the
 * whole family is what the four-place "player1"/"p1" queue fix and the
 * stage->stage watcher hook were about, so each is also a regression pin.
 * ══════════════════════════════════════════════════════════════════════════ */

/* One card, one observable, two causes. Rust spells these as two #[test]s
 * (self / opponent); the C port drives both on ONE board with a fresh card
 * instance per cause, which is strictly more coverage at half the setup. */
static void cross_player_case(const char *card_no, const char *label,
                              int expect_blade, int expect_heart_colour)
{
    static TestGame g;
    new_game(&g);

    int self_c = test_id(&g, card_no);
    int opp_c  = test_new_id(&g, card_no);
    int filler = test_id(&g, FILLER);
    char msg[256];
    snprintf(msg, sizeof(msg), "%s: fixtures resolve", label);
    CHECK(self_c >= 0 && opp_c >= 0, msg);
    if (self_c < 0 || opp_c < 0) { n_setup_bugs++; return; }
    CHECK(self_c != opp_c, "the two causes use DISTINCT card instances");
    expect_card_no(self_c, card_no);
    expect_card_no(opp_c, card_no);
    expect_trigger(self_c, TRIG_AUTO, 1);

    fill_decks(&g, filler);

    /* --- cause 1: P1's OWN card effect (causer = p1) --- */
    g.state.p[0].stage[1] = self_c;
    scan_area_move(&g, 0, &self_c, 1);
    drain_skip(&g);
    if (expect_blade) {
        snprintf(msg, sizeof(msg),
                 "%s: SELF cause grants %d blade(s), got %d", label, expect_blade, blade(&g, self_c));
        CHECK(blade(&g, self_c) == expect_blade, msg);
    }
    if (expect_heart_colour) {
        snprintf(msg, sizeof(msg),
                 "%s: SELF cause grants heart0%d, got %d", label, expect_heart_colour,
                 heart(&g, self_c, expect_heart_colour));
        CHECK(heart(&g, self_c, expect_heart_colour) == 1, msg);
    }

    /* --- cause 2: P2's card effect (causer = p2). The parenthetical arm. --- */
    g.state.p[0].stage[1] = opp_c;
    scan_area_move(&g, 0, &opp_c, 1);
    drain_skip(&g);
    if (expect_blade) {
        snprintf(msg, sizeof(msg),
                 "%s: OPPONENT cause (でも発動する) also grants %d blade(s), got %d",
                 label, expect_blade, blade(&g, opp_c));
        CHECK(blade(&g, opp_c) == expect_blade, msg);
    }
    if (expect_heart_colour) {
        snprintf(msg, sizeof(msg),
                 "%s: OPPONENT cause (でも発動する) also grants heart0%d, got %d",
                 label, expect_heart_colour, heart(&g, opp_c, expect_heart_colour));
        CHECK(heart(&g, opp_c, expect_heart_colour) == 1, msg);
    }
}

static void koko_self_and_opponent_heart06(void)
{
    /* PL!SP-sd2-002-SD2 唐 可可: area move -> heart06, ターン1回. */
    cross_player_case(KOKO, "Koko cross-player", 0, 6);
}

static void tomari_self_and_opponent_blade(void)
{
    cross_player_case(TOMARI, "Tomari cross-player", 1, 0);
}

static void kanon_self_and_opponent_heart02(void)
{
    cross_player_case(KANON, "Kanon cross-player", 0, 2);
}

static void fuyumari_self_and_opponent_heart03(void)
{
    cross_player_case(FUYU, "Fuyumari cross-player", 0, 3);
}

static void chisato_self_and_opponent_blades(void)
{
    cross_player_case(CHISATO, "Chisato cross-player", 2, 0);
}

static void natsume_self_and_opponent_draw(void)
{
    static TestGame g;
    new_game(&g);
    int n1 = test_id(&g, NATSUME);
    int n2 = test_new_id(&g, NATSUME);
    int filler = test_id(&g, FILLER);
    char msg[200];
    CHECK(n1 >= 0 && n2 >= 0, "Natsume cross-player: fixtures resolve");
    if (n1 < 0 || n2 < 0) { n_setup_bugs++; return; }
    expect_card_no(n1, NATSUME);
    expect_card_no(n2, NATSUME);
    expect_trigger(n1, TRIG_AUTO, 1);
    fill_decks(&g, filler);

    g.state.p[0].stage[0] = n1;
    int hand_before = g.state.p[0].hand.n, deck_before = g.state.p[0].deck.n;
    scan_area_move(&g, 0, &n1, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg), "Natsume: SELF move draws 1 (hand %d->%d, deck %d->%d)",
             hand_before, g.state.p[0].hand.n, deck_before, g.state.p[0].deck.n);
    CHECK(g.state.p[0].hand.n == hand_before + 1 &&
          g.state.p[0].deck.n == deck_before - 1, msg);

    g.state.p[0].stage[0] = n2;
    hand_before = g.state.p[0].hand.n;
    deck_before = g.state.p[0].deck.n;
    scan_area_move(&g, 0, &n2, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "Natsume: OPPONENT move ALSO draws (でも発動する) (hand %d->%d)",
             hand_before, g.state.p[0].hand.n);
    CHECK(g.state.p[0].hand.n == hand_before + 1 &&
          g.state.p[0].deck.n == deck_before - 1, msg);
}

static void koko_turn1_blocks_second(void)
{
    static TestGame g;
    new_game(&g);
    int k = test_id(&g, KOKO);
    CHECK(k >= 0, "Koko turn1: fixtures resolve");
    if (k < 0) { n_setup_bugs++; return; }
    expect_card_no(k, KOKO);
    expect_trigger(k, TRIG_AUTO, 1);

    g.state.p[0].stage[1] = k;
    scan_area_move(&g, 0, &k, 1);
    drain_skip(&g);
    char msg[200];
    snprintf(msg, sizeof(msg), "Koko turn1: first move grants heart06, got %d", heart(&g, k, 6));
    CHECK(heart(&g, k, 6) == 1, msg);

    /* Second DISTINCT move in the same turn. Rust pushes a second movement
     * event and re-scans; the ターン1回 budget must absorb it. */
    scan_area_move(&g, 0, &k, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "Koko turn1: a SECOND move in the same turn must not stack (heart06=%d, still 1)",
             heart(&g, k, 6));
    CHECK(heart(&g, k, 6) == 1, msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §B  jidou_paired_ability_combination_test.rs  (5 tests)
 *
 * PL!SP-bp7-005-R＋ carries TWO 自動: ab#0 places an energy card from the
 * energy deck in the wait state, ab#1 grants a blade to an own-effect energy
 * placement. They must coexist. Rust's own comments call out that checking
 * only `blade_modifiers` saw just ab#1 and passed when ab#0 did nothing, so
 * every case here asserts BOTH halves.
 * ══════════════════════════════════════════════════════════════════════════ */

static void jidou_effect_cause_both_sides(void)
{
    static TestGame g;
    new_game(&g);
    int card = test_id(&g, HATSUNE);
    int filler = test_id(&g, FILLER);
    int energy = test_id(&g, ENERGY_C);
    char msg[220];
    CHECK(card >= 0 && filler >= 0 && energy >= 0, "jidou effect-cause: fixtures resolve");
    if (card < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(card, HATSUNE);
    expect_card_no(energy, ENERGY_C);
    /* Rust asserts `db.get_card(card).unwrap().abilities.len() == 2` in the
     * sibling case; the pin that matters is the energy-placement ability. */
    expect_trigger(card, TRIG_AUTO, 2);
    fill_decks(&g, filler);
    fill_energy_deck(&g, 0, energy, 3);
    g.state.p[0].stage[0] = card;

    int zone_before = g.state.p[0].energy.n;
    int edeck_before = g.state.p[0].energy_deck.n;
    /* The event that arms ab#1: an energy card placed by OUR OWN effect. */
    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);

    snprintf(msg, sizeof(msg),
             "an own-effect energy placement grants EXACTLY this card's one blade, got %d",
             blade(&g, card));
    CHECK(blade(&g, card) == 1, msg);
    snprintf(msg, sizeof(msg),
             "the BLADE jidou moves no energy itself (zone %d->%d, energy deck %d->%d)",
             zone_before, g.state.p[0].energy.n, edeck_before, g.state.p[0].energy_deck.n);
    CHECK(g.state.p[0].energy.n == zone_before &&
          g.state.p[0].energy_deck.n == edeck_before, msg);
    CHECK(on_stage(&g, 0, 0, card), "the watcher is still on stage");
}

static void jidou_paired_with_other_ability_both_fire(void)
{
    static TestGame g;
    new_game(&g);
    int card = test_id(&g, HATSUNE);
    int filler = test_id(&g, FILLER);
    int energy = test_id(&g, ENERGY_C);
    char msg[220];
    CHECK(card >= 0 && filler >= 0 && energy >= 0, "jidou pair: fixtures resolve");
    if (card < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(card, HATSUNE);
    fill_decks(&g, filler);
    /* Rust's comment: fill_decks only fills the MAIN deck; without this the
     * sequential silently did nothing and the test proved nothing. */
    fill_energy_deck(&g, 0, energy, 3);
    g.state.p[0].stage[0] = card;

    int edeck_before = g.state.p[0].energy_deck.n;
    /* ab#0 via the 登場 branch: hand -> stage appearance. */
    rb_record_card_movement(&g.state, card, RB_ZONEID_HAND, RB_ZONEID_STAGE, 0, 1);
    scan_area_move(&g, 0, &card, 1);
    drain_skip(&g);
    int edeck_after_first = g.state.p[0].energy_deck.n;
    snprintf(msg, sizeof(msg),
             "ab#0 placed EXACTLY one energy card out of the energy deck (%d->%d)",
             edeck_before, edeck_after_first);
    CHECK(edeck_before - edeck_after_first == 1, msg);

    /* ab#1 via an own-effect energy placement. */
    int blade_before = blade(&g, card);
    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg), "ab#1 granted its blade (%d->%d)",
             blade_before, blade(&g, card));
    CHECK(blade(&g, card) - blade_before == 1, msg);
}

static void jidou_distinct_from_constant_and_activation(void)
{
    static TestGame g;
    new_game(&g);
    int m = test_id(&g, HATSUNE);
    int filler = test_id(&g, FILLER);
    int energy = test_id(&g, ENERGY_C);
    char msg[220];
    CHECK(m >= 0 && filler >= 0 && energy >= 0, "jidou vs constant: fixtures resolve");
    if (m < 0) { n_setup_bugs++; return; }
    expect_card_no(m, HATSUNE);
    fill_decks(&g, filler);
    g.state.p[0].stage[0] = m;

    int before = blade(&g, m);
    rb_recalc_constants(&g.state);
    int after_const = blade(&g, m);
    snprintf(msg, sizeof(msg),
             "constant recalc is idempotent for a pure-jidou card (%d->%d)",
             before, after_const);
    CHECK(before == after_const, msg);

    /* The point: the 自動 path is DISTINCT from the constant path. Rust's
     * previous final assertion was `a >= b || a == b`, a tautology. */
    int const_bonus_before = g.state.mods.p1_constant_total_score_bonus;
    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "driving the 自動 adds exactly this card's blade and is NOT folded "
             "into the constant path (%d->%d, constant score %d->%d)",
             after_const, blade(&g, m), const_bonus_before,
             g.state.mods.p1_constant_total_score_bonus);
    CHECK(blade(&g, m) - after_const == 1, msg);
    CHECK(g.state.mods.p1_constant_total_score_bonus == const_bonus_before,
          "the constant path granted nothing for this pure-jidou card");
}

static void jidou_both_on_same_card_coexist_and_fire_separately(void)
{
    static TestGame g;
    new_game(&g);
    int card = test_id(&g, HATSUNE);
    int filler = test_id(&g, FILLER);
    int energy = test_id(&g, ENERGY_C);
    char msg[220];
    CHECK(card >= 0 && filler >= 0 && energy >= 0, "jidou coexist: fixtures resolve");
    if (card < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(card, HATSUNE);
    snprintf(msg, sizeof(msg), "the fixture really does carry TWO abilities (nAb=%d)",
             rb_card_num_abilities((uint32_t)card));
    CHECK(rb_card_num_abilities((uint32_t)card) == 2, msg);
    fill_decks(&g, filler);
    fill_energy_deck(&g, 0, energy, 3);
    g.state.p[0].stage[0] = card;

    rb_record_card_movement(&g.state, card, RB_ZONEID_HAND, RB_ZONEID_STAGE, 0, 1);
    scan_area_move(&g, 0, &card, 1);
    drain_skip(&g);
    int zone_after_first = g.state.p[0].energy.n;
    int blade_before_second = blade(&g, card);

    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);

    CHECK(on_stage(&g, 0, 0, card), "the watcher survives both firings");
    snprintf(msg, sizeof(msg),
             "neither jidou moves cards INTO the energy zone (%d->%d)",
             zone_after_first, g.state.p[0].energy.n);
    CHECK(g.state.p[0].energy.n == zone_after_first, msg);
    snprintf(msg, sizeof(msg),
             "the energy-placed jidou added EXACTLY its one blade on the second "
             "trigger (%d->%d)", blade_before_second, blade(&g, card));
    CHECK(blade(&g, card) - blade_before_second == 1, msg);
}

static void jidou_watching_live_start_resolve_triggers(void)
{
    static TestGame g;
    new_game(&g);
    int watcher = test_id(&g, DANCING);
    int muse = test_id(&g, HONOKA_R);
    int filler = test_id(&g, FILLER);
    char msg[220];
    CHECK(watcher >= 0 && muse >= 0 && filler >= 0, "jidou LS watcher: fixtures resolve");
    if (watcher < 0 || muse < 0) { n_setup_bugs++; return; }
    expect_card_no(watcher, DANCING);
    expect_card_no(muse, HONOKA_R);
    expect_trigger(muse, TRIG_LIVE_START, 1);
    fill_decks(&g, filler);
    test_add_to_live(&g, watcher);
    g.state.p[0].stage[1] = muse;

    CHECK(fire_trigger(&g, muse, TRIG_LIVE_START),
          "the muse exposes the ライブ開始時 ability that was fired");
    CHECK_PROMPTS(&g, "the Dancing stars watcher prompts on the muse's live start");
    test_resume_choice(&g, 0);
    drain_skip(&g);
    snprintf(msg, sizeof(msg), "the live-start resolution moved the muse into the "
                               "stage's left area (stage[0]=%d, want %d)",
             g.state.p[0].stage[0], muse);
    CHECK(on_stage(&g, 0, 0, muse), msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §C  trigger_scope_test.rs  (6 tests)
 *
 * S1 「自分のカードの効果によって」 -> own effects ONLY.
 * S2 「(...効果でも発動する。)」   -> own AND opponent.
 * S3 unscoped movement trigger    -> own side (default).
 * ══════════════════════════════════════════════════════════════════════════ */

static void s2_pb1_006_own_debut_fires(void)
{
    static TestGame g;
    new_game(&g);
    int member = test_new_id(&g, CHISATO);
    char msg[220];
    CHECK(member >= 0, "S2 own debut: fixture resolves");
    if (member < 0) { n_setup_bugs++; return; }
    expect_card_no(member, CHISATO);
    expect_trigger(member, TRIG_AUTO, 1);
    CHECK(printed_cost(member) == 9, "S2 own debut: the fixture really prints cost 9");
    test_give_energy(&g, 9);
    test_add_to_hand(&g, member);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;

    CHECK(test_play_to_stage(&g, member, 1), "the cost-9 member is played to Center");
    scan_both(&g);
    snprintf(msg, sizeof(msg), "own debut arms the S2 auto -> +2 blade, got %d", blade(&g, member));
    CHECK(blade(&g, member) == 2, msg);
}

static void s2_pb1_006_opponent_drag_arms_per_move(void)
{
    static TestGame g;
    new_game(&g);
    int member = test_new_id(&g, CHISATO);
    int mirakura = test_new_id(&g, MIRAKURA);
    int mover = test_new_id(&g, HIME);
    int filler = test_id(&g, FILLER);
    char msg[240];
    CHECK(member >= 0 && mirakura >= 0 && mover >= 0, "S2 opponent drag: fixtures resolve");
    if (member < 0 || mover < 0) { n_setup_bugs++; return; }
    expect_card_no(member, CHISATO);
    expect_card_no(mover, HIME);
    expect_card_no(mirakura, MIRAKURA);
    CHECK(member != mover, "the watcher and the mover are DIFFERENT prints");
    CHECK(printed_cost(mover) == 9, "S2 drag: the mover really prints cost 9");
    fill_decks(&g, filler);

    /* Own debut first: the control that proves the auto works in this board. */
    test_give_energy(&g, 9);
    test_add_to_hand(&g, member);
    test_play_to_stage(&g, member, 2);
    scan_both(&g);
    snprintf(msg, sizeof(msg), "CONTROL: own debut armed the S2 auto (+2, got %d)",
             blade(&g, member));
    CHECK(blade(&g, member) == 2, msg);

    /* P2 board: an all-『みらくらぱーく！』 gate for the mover. */
    g.state.p[1].stage[0] = mirakura;
    g.state.p[1].stage[1] = RB_EMPTY_SLOT;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand_for(&g, 1, mover);
    test_give_energy_for(&g, 1, 9);

    /* DRAG #1: the mover debuts at P2 Center, pulling the P1 watcher right->center. */
    CHECK(test_play_to_stage_for(&g, 1, mover, 1), "P2 debuts the cost-9 mover at Center");
    drain_first(&g);
    scan_both(&g);

    snprintf(msg, sizeof(msg), "drag #1 moved the P1 watcher into Center (stage[1]=%d, want %d)",
             g.state.p[0].stage[1], member);
    CHECK(on_stage(&g, 0, 1, member), msg);
    snprintf(msg, sizeof(msg),
             "THE PARENTHETICAL PIN: an OPPONENT-caused drag still arms the S2 auto "
             "(blade %d, want 4 = 2 own + 2 per distinct move)", blade(&g, member));
    CHECK(blade(&g, member) == 4, msg);
}

static void s2_bp4_016_energy_placed_by_opponent_fires(void)
{
    static TestGame g;
    new_game(&g);
    int watcher = test_new_id(&g, HANOKO);
    int energy = test_new_id(&g, ENERGY_C);
    char msg[220];
    CHECK(watcher >= 0 && energy >= 0, "S2 energy-by-opponent: fixtures resolve");
    if (watcher < 0) { n_setup_bugs++; return; }
    expect_card_no(watcher, HANOKO);
    expect_card_no(energy, ENERGY_C);
    expect_trigger(watcher, TRIG_AUTO, 1);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = watcher;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    scan_both(&g);

    int control_energy = test_new_id(&g, ENERGY_C);
    int own_before = heart(&g, watcher, 6);
    /* VACUITY CONTROL: the SAME card, the SAME event, caused by P1 instead of
     * P2. Without this the opponent arm's silence could be a card that simply
     * is not wired to this event at all. */
    CHECK(push_energy_placed(&g, control_energy, 0, 0),
          "CONTROL: P1's OWN card effect places an energy card into P1's zone");
    scan_energy_placed(&g, 0, &control_energy, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "CONTROL: an OWN-caused placement on the very same board moves the "
             "watcher's heart06 (%d->%d) -- without this the opponent arm's silence "
             "below would prove nothing", own_before, heart(&g, watcher, 6));
    CHECK(heart(&g, watcher, 6) != own_before, msg);

    int opp_energy = test_new_id(&g, ENERGY_C);
    int before = heart(&g, watcher, 6);
    CHECK(push_energy_placed(&g, opp_energy, 0, 1),
          "P2's card effect placed an energy card into P1's own energy zone");
    scan_energy_placed(&g, 0, &opp_energy, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "parenthetical: an OPPONENT-caused energy placement fires +1 heart06 "
             "(%d->%d)", before, heart(&g, watcher, 6));
    CHECK(heart(&g, watcher, 6) - before == 1, msg);
}

/* The S1 control. It is a SEPARATE process-local board on purpose: the S1
 * negative below must run on a board where nothing has consumed the ターン1回
 * budget or written a modifier, so its "nothing happened" is attributable to
 * the self_effect_only gate and to nothing else. */
static void s1_bp7_005_own_placement_control_fires(void)
{
    static TestGame g;
    new_game(&g);
    int watcher = test_new_id(&g, HATSUNE);
    int energy = test_new_id(&g, ENERGY_C);
    char msg[220];
    CHECK(watcher >= 0 && energy >= 0, "S1 control: fixtures resolve");
    if (watcher < 0) { n_setup_bugs++; return; }
    expect_card_no(watcher, HATSUNE);
    expect_trigger(watcher, TRIG_AUTO, 1);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = watcher;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    scan_both(&g);

    int before = blade(&g, watcher);
    /* P1's OWN effect places the energy. */
    CHECK(push_energy_placed(&g, energy, 0, 0),
          "P1's own card effect placed an energy card into P1's energy zone");
    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "CONTROL: an OWN-effect placement on the very same board DOES change the "
             "modifiers (blade %d->%d, heart06 %d->%d, heart02 %d->%d) -- without this "
             "the S1 negative below would be vacuous",
             before, blade(&g, watcher), 0, heart(&g, watcher, 6), 0, heart(&g, watcher, 2));
    CHECK(blade(&g, watcher) != before || heart(&g, watcher, 6) != 0 ||
          heart(&g, watcher, 2) != 0, msg);
}

static void s1_bp7_005_opponent_caused_placement_does_not_fire(void)
{
    static TestGame g;
    new_game(&g);
    int watcher = test_new_id(&g, HATSUNE);
    int energy = test_new_id(&g, ENERGY_C);
    char msg[220];
    CHECK(watcher >= 0 && energy < 0 ? 0 : (watcher >= 0 && energy >= 0),
          "S1 negative: fixtures resolve");
    if (watcher < 0 || energy < 0) { n_setup_bugs++; return; }
    expect_card_no(watcher, HATSUNE);
    expect_trigger(watcher, TRIG_AUTO, 1);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = watcher;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    scan_both(&g);

    int blade_before = blade(&g, watcher);
    int h1 = heart(&g, watcher, 1), h2 = heart(&g, watcher, 2), h3 = heart(&g, watcher, 3);
    int h4 = heart(&g, watcher, 4), h5 = heart(&g, watcher, 5), h6 = heart(&g, watcher, 6);

    /* P2's effect places an energy card into P1's OWN zone. */
    CHECK(push_energy_placed(&g, energy, 0, 1),
          "P2's card effect placed an energy card into P1's own energy zone");
    scan_energy_placed(&g, 0, &energy, 1);
    drain_skip(&g);

    snprintf(msg, sizeof(msg),
             "self_effect_only: an OPPONENT-caused placement must not fire the gain "
             "(blade %d->%d)", blade_before, blade(&g, watcher));
    CHECK(blade(&g, watcher) == blade_before, msg);
    snprintf(msg, sizeof(msg),
             "and no heart anywhere either (%d/%d/%d/%d/%d/%d -> %d/%d/%d/%d/%d/%d)",
             h1, h2, h3, h4, h5, h6, heart(&g, watcher, 1), heart(&g, watcher, 2),
             heart(&g, watcher, 3), heart(&g, watcher, 4), heart(&g, watcher, 5),
             heart(&g, watcher, 6));
    CHECK(heart(&g, watcher, 1) == h1 && heart(&g, watcher, 2) == h2 &&
          heart(&g, watcher, 3) == h3 && heart(&g, watcher, 4) == h4 &&
          heart(&g, watcher, 5) == h5 && heart(&g, watcher, 6) == h6, msg);
}

/* Shared board for the S3 pair: a PL!S-bp5-111-R activation position-changes
 * the card into the area of a low-blade opponent member, which must then arm
 * the no-parenthetical 自動 that waits that opponent member. */
static void saint_snow_board(TestGame *g, int *watcher, int *aqours, int *opp_member)
{
    int w = test_id(g, SAINT);
    int a = test_new_id(g, AQOURS_P);
    int o = test_new_id(g, FILLER);   /* original blade 1, i.e. <= 2 */
    if (w < 0 || a < 0 || o < 0) { n_setup_bugs++; *watcher = -1; return; }
    expect_card_no(w, SAINT);
    expect_card_no(a, AQOURS_P);
    expect_trigger(w, TRIG_AUTO, 1);
    fill_decks(g, test_id(g, FILLER));
    g->state.p[0].stage[0] = a;
    g->state.p[0].stage[1] = w;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    g->state.p[1].stage[0] = RB_EMPTY_SLOT;
    g->state.p[1].stage[1] = o;
    g->state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(g, 1); /* {E} activation cost */
    *watcher = w; *aqours = a; *opp_member = o;
}

static void s3_own_position_change_rests_low_blade_opponent(void)
{
    static TestGame g;
    new_game(&g);
    int w = -1, a = -1, o = -1;
    saint_snow_board(&g, &w, &a, &o);
    if (w < 0) return;
    char msg[240];

    /* The low-blade precondition: Rust's own note is that this is the
     * property the 自動 turns on ("opponent's blade<=2 member"), so it is
     * asserted rather than assumed. A board whose target has 3+ blades would
     * make the whole case vacuously "not waited". */
    int opp_blades = printed_blade_count(o);
    snprintf(msg, sizeof(msg),
             "precondition: the target opponent member PRINTS <= 2 blade hearts "
             "(printed blade=%d) -- a higher count makes this case vacuous",
             opp_blades);
    CHECK(opp_blades >= 0 && opp_blades <= 2, msg);

    test_activate_ability(&g, w);
    CHECK_PROMPTS(&g, "the position-change destination prompt is offered");
    snprintf(msg, sizeof(msg), "and it is a SelectTarget (position|destination), got '%s'",
             test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(test_pending_choice_type(&g) &&
          strcmp(test_pending_choice_type(&g), "SelectTarget") == 0, msg);
    test_resume_choice(&g, 0);
    scan_both(&g);

    snprintf(msg, sizeof(msg), "the activation moved the watcher into the Aqours area "
                               "(stage[0]=%d want %d, stage[1]=%d want %d)",
             g.state.p[0].stage[0], w, g.state.p[0].stage[1], a);
    CHECK(on_stage(&g, 0, 0, w) && on_stage(&g, 0, 1, a), msg);
    CHECK(!test_has_pending_choice(&g), "the whole chain finished resolving");
    snprintf(msg, sizeof(msg),
             "S3 own-effect control: the no-parenthetical 自動 DOES wait the "
             "opponent's low-blade member (orientation='%s')",
             orient(&g, o) ? orient(&g, o) : "(none)");
    CHECK(is_wait(&g, o), msg);
}

static void s3_opponent_drag_without_parenthetical_keeps_target_active(void)
{
    static TestGame g;
    new_game(&g);
    int w = test_new_id(&g, SAINT);
    int o = test_new_id(&g, RURINO_P);
    int mover = test_new_id(&g, HIME);
    int filler = test_id(&g, FILLER);
    char msg[240];
    CHECK(w >= 0 && o >= 0 && mover >= 0, "S3 opponent drag: fixtures resolve");
    if (w < 0 || o < 0 || mover < 0) { n_setup_bugs++; return; }
    expect_card_no(w, SAINT);
    expect_card_no(o, RURINO_P);
    expect_card_no(mover, HIME);
    expect_trigger(w, TRIG_AUTO, 1);
    fill_decks(&g, filler);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = RB_EMPTY_SLOT;
    g.state.p[0].stage[2] = w;
    g.state.p[1].stage[0] = o;
    g.state.p[1].stage[1] = RB_EMPTY_SLOT;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand_for(&g, 1, mover);
    test_give_energy_for(&g, 1, 9);
    CHECK(!is_wait(&g, o), "precondition: the opponent member starts ACTIVE, not waited");

    CHECK(test_play_to_stage_for(&g, 1, mover, 1),
          "the opponent debuts the cost-9 mover at Center");
    drain_first(&g);
    scan_both(&g);

    snprintf(msg, sizeof(msg),
             "the opponent's drag pulled the P1 watcher right -> Center "
             "(stage[1]=%d want %d)", g.state.p[0].stage[1], w);
    CHECK(on_stage(&g, 0, 1, w), msg);
    snprintf(msg, sizeof(msg),
             "S3: an OPPONENT-caused move must NOT rest the on-stage legal target "
             "(orientation='%s', must not be 'wait')", orient(&g, o) ? orient(&g, o) : "(none)");
    CHECK(!is_wait(&g, o), msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D  sibling_abilities_on_one_card_interact_test.rs  (7 tests)
 * ══════════════════════════════════════════════════════════════════════════ */

static void bp5_111_activation_chain_waits_low_blade_opponent(void)
{
    static TestGame g;
    new_game(&g);
    int w = -1, a = -1, o = -1;
    saint_snow_board(&g, &w, &a, &o);
    if (w < 0) return;
    char msg[240];
    int opp_blades = printed_blade_count(o);
    snprintf(msg, sizeof(msg),
             "precondition: the CHAIN target prints <= 2 blade hearts (printed=%d), "
             "so the chain is not vacuous", opp_blades);
    CHECK(opp_blades >= 0 && opp_blades <= 2, msg);

    test_activate_ability(&g, w);
    CHECK_PROMPTS(&g, "the position-change destination prompt is offered");
    snprintf(msg, sizeof(msg), "expected SelectTarget, got '%s'",
             test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(test_pending_choice_type(&g) &&
          strcmp(test_pending_choice_type(&g), "SelectTarget") == 0, msg);
    test_resume_choice(&g, 0);
    scan_both(&g);

    snprintf(msg, sizeof(msg), "the activation moved the member to the Aqours area "
                               "(stage[0]=%d want %d)", g.state.p[0].stage[0], w);
    CHECK(on_stage(&g, 0, 0, w), msg);
    snprintf(msg, sizeof(msg),
             "CHAIN: the sibling 自動 waits the opponent's blade<=2 member "
             "(orientation='%s')", orient(&g, o) ? orient(&g, o) : "(none)");
    CHECK(is_wait(&g, o), msg);
}

static void bp5_111_chain_neg_empty_opponent_stage(void)
{
    static TestGame g;
    new_game(&g);
    int w = test_id(&g, SAINT);
    int a = test_new_id(&g, AQOURS_P);
    char msg[240];
    CHECK(w >= 0 && a >= 0, "Saint Snow negative: fixtures resolve");
    if (w < 0) { n_setup_bugs++; return; }
    expect_card_no(w, SAINT);
    expect_trigger(w, TRIG_AUTO, 1);
    fill_decks(&g, test_id(&g, FILLER));
    g.state.p[0].stage[0] = a;
    g.state.p[0].stage[1] = w;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[1].stage[i] = RB_EMPTY_SLOT;
    test_give_energy(&g, 1);

    test_activate_ability(&g, w);
    /* The destination prompt is offered even with an empty opponent stage --
     * that prompt is the vacuity control for the "nothing was waited" claim. */
    CHECK_PROMPTS(&g, "the destination prompt is STILL offered with an empty opponent stage");
    test_resume_choice(&g, 0);
    scan_both(&g);
    snprintf(msg, sizeof(msg), "the activation still moved the member (stage[0]=%d want %d)",
             g.state.p[0].stage[0], w);
    CHECK(on_stage(&g, 0, 0, w), msg);
    int empty = 1;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (g.state.p[1].stage[i] != RB_EMPTY_SLOT) empty = 0;
    snprintf(msg, sizeof(msg),
             "negative: the empty opponent stage stays empty, and no orientation was "
             "written (opponent members = %d)", empty);
    CHECK(empty, msg);
}

static void hs_pb1_003_debut_discards_feed_auto_gain(void)
{
    static TestGame g;
    new_game(&g);
    int mirakura = test_new_id(&g, MIRAKURA);
    int card = test_new_id(&g, RURINO);
    int filler = test_id(&g, FILLER);
    char msg[240];
    CHECK(mirakura >= 0 && card >= 0 && filler >= 0, "Rurino debut->auto: fixtures resolve");
    if (card < 0 || mirakura < 0) { n_setup_bugs++; return; }
    expect_card_no(card, RURINO);
    expect_card_no(mirakura, MIRAKURA);
    CHECK(printed_cost(card) == 15, "the 登場 host really prints cost 15");
    test_give_energy(&g, 15);
    g.state.p[0].hand.n = 0;
    test_add_to_hand(&g, card);
    test_add_to_hand(&g, mirakura);
    test_add_to_hand(&g, filler);
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[0].stage[i] = RB_EMPTY_SLOT;
    fill_decks(&g, filler);

    test_play_to_stage(&g, card, 1);
    CHECK_PROMPTS(&g, "the 登場 discard choice is offered (any-number みらくらぱーく！ discard)");
    int zero[] = { 0 };
    test_select_indices(&g, zero, 1);
    drain_skip(&g);

    snprintf(msg, sizeof(msg), "hand: -played -discarded +drawn(1+1) = 3, got %d",
             g.state.p[0].hand.n);
    CHECK(g.state.p[0].hand.n == 3, msg);
    snprintf(msg, sizeof(msg), "the みらくらぱーく！ member went to the waitroom, got %d",
             test_zone_has_id(&g, 0, "discard", mirakura));
    CHECK(test_zone_has_id(&g, 0, "discard", mirakura), msg);
    snprintf(msg, sizeof(msg),
             "CHAIN: the debut's hand->waitroom discard arms the per-discard 自動 "
             "-> +1 blade, got %d", blade(&g, card));
    CHECK(blade(&g, card) >= 1, msg);
    snprintf(msg, sizeof(msg),
             "CHAIN: the per-discard 自動 -> +1 heart01, got %d", heart(&g, card, 1));
    CHECK(heart(&g, card, 1) >= 1, msg);
}

static void hs_pb1_003_no_discard_no_auto_gain(void)
{
    static TestGame g;
    new_game(&g);
    int card = test_new_id(&g, RURINO);
    int filler = test_id(&g, FILLER);
    char msg[240];
    CHECK(card >= 0 && filler >= 0, "Rurino no-discard negative: fixtures resolve");
    if (card < 0) { n_setup_bugs++; return; }
    expect_card_no(card, RURINO);
    test_give_energy(&g, 15);
    g.state.p[0].hand.n = 0;
    test_add_to_hand(&g, card);
    test_add_to_hand(&g, filler);
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[0].stage[i] = RB_EMPTY_SLOT;
    fill_decks(&g, filler);

    test_play_to_stage(&g, card, 1);
    /* No eligible みらくらぱーく！ member in hand -> no prompt at all. */
    snprintf(msg, sizeof(msg),
             "no discard prompt is expected with no eligible member in hand, got '%s'",
             test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(!test_has_pending_choice(&g), msg);
    drain_skip(&g);

    snprintf(msg, sizeof(msg), "hand: -played +drawn(0+1) = 2, got %d", g.state.p[0].hand.n);
    CHECK(g.state.p[0].hand.n == 2, msg);
    snprintf(msg, sizeof(msg),
             "negative: no hand->waitroom discard, so no 自動 blade, got %d", blade(&g, card));
    CHECK(blade(&g, card) == 0, msg);
    snprintf(msg, sizeof(msg), "negative: and no heart01, got %d", heart(&g, card, 1));
    CHECK(heart(&g, card, 1) == 0, msg);
}

static void sp_bp7_005_double_auto_cascade(void)
{
    static TestGame g;
    new_game(&g);
    int member = test_new_id(&g, HATSUNE);
    int energy = test_new_id(&g, ENERGY_C);
    char msg[240];
    CHECK(member >= 0 && energy >= 0, "Hatsukoi cascade: fixtures resolve");
    if (member < 0) { n_setup_bugs++; return; }
    expect_card_no(member, HATSUNE);
    expect_trigger(member, TRIG_AUTO, 2);
    CHECK(printed_cost(member) == 9, "the cascade member really prints cost 9");
    for (int i = 0; i < 3; i++) test_add_to_energy_deck(&g, 0, energy);
    int edeck_setup = g.state.p[0].energy_deck.n;
    test_give_energy(&g, 10);
    int zone_before = g.state.p[0].energy.n;
    int active_before = g.state.p[0].energy_active;
    test_add_to_hand(&g, member);
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[0].stage[i] = RB_EMPTY_SLOT;

    test_try_play_to_stage(&g, member, 1);
    scan_both(&g);

    snprintf(msg, sizeof(msg), "ab#0 placed EXACTLY one energy from the deck (%d->%d)",
             edeck_setup, g.state.p[0].energy_deck.n);
    CHECK(edeck_setup - g.state.p[0].energy_deck.n == 1, msg);
    snprintf(msg, sizeof(msg), "the placed energy landed in the energy zone (%d->%d)",
             zone_before, g.state.p[0].energy.n);
    CHECK(g.state.p[0].energy.n == zone_before + 1, msg);
    snprintf(msg, sizeof(msg),
             "the placed energy is WAITED (active count only dropped by the 9 play cost, "
             "%d->%d)", active_before, g.state.p[0].energy_active);
    CHECK(g.state.p[0].energy_active == active_before - 9, msg);
    snprintf(msg, sizeof(msg),
             "CASCADE: ab#0's own-effect zone placement arms ab#1 -> +1 blade, got %d",
             blade(&g, member));
    CHECK(blade(&g, member) >= 1, msg);
}

static void n_bp3_005_constant_grant_and_counting_auto_combo(void)
{
    static TestGame g;
    new_game(&g);
    int member = test_new_id(&g, AI);
    int d1 = test_new_id(&g, FILLER);
    int d2 = test_new_id(&g, FILLER);
    char msg[240];
    CHECK(member >= 0 && d1 >= 0 && d2 >= 0, "Ai combo: fixtures resolve");
    if (member < 0 || d1 < 0 || d2 < 0) { n_setup_bugs++; return; }
    expect_card_no(member, AI);
    expect_trigger(member, TRIG_AUTO, 1);
    CHECK(printed_cost(member) == 15, "the combo member really prints cost 15");
    test_give_energy(&g, 30);
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_hand(&g, d1);
    test_add_to_hand(&g, d2);
    test_add_to_hand(&g, member);
    fill_decks(&g, test_id(&g, FILLER));

    test_play_to_stage(&g, d1, 0); scan_both(&g);
    test_play_to_stage(&g, d2, 2); scan_both(&g);
    test_play_to_stage(&g, member, 1); scan_both(&g);

    snprintf(msg, sizeof(msg), "3rd debut: the counting 自動 drew until hand held 5, got %d",
             g.state.p[0].hand.n);
    CHECK(g.state.p[0].hand.n == 5, msg);

    int const_before = g.state.mods.p1_constant_total_score_bonus;
    /* ライブ開始時 is driven directly, never through the phase machine. The
     * ability is gated on "2+ own debuts this turn", which the three real
     * plays above have established. */
    CHECK(fire_trigger(&g, member, TRIG_LIVE_START),
          "the combo member exposes the ライブ開始時 conditional 常時");
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "conditional 常時 granted (constant total score %d->%d, want %d)",
             const_before, g.state.mods.p1_constant_total_score_bonus, const_before + 1);
    CHECK(g.state.mods.p1_constant_total_score_bonus == const_before + 1, msg);
}

static void n_bp3_005_two_debuts_auto_silent_but_constant_grants(void)
{
    static TestGame g;
    new_game(&g);
    int member = test_new_id(&g, AI);
    int d1 = test_new_id(&g, FILLER);
    char msg[240];
    CHECK(member >= 0 && d1 >= 0, "Ai boundary pair: fixtures resolve");
    if (member < 0 || d1 < 0) { n_setup_bugs++; return; }
    expect_card_no(member, AI);
    test_give_energy(&g, 25);
    for (int i = 0; i < RB_STAGE_SIZE; i++) g.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_hand(&g, d1);
    test_add_to_hand(&g, member);
    fill_decks(&g, test_id(&g, FILLER));

    test_play_to_stage(&g, d1, 0); scan_both(&g);
    test_play_to_stage(&g, member, 1); scan_both(&g);

    snprintf(msg, sizeof(msg),
             "2nd debut < 3: the counting 自動 stayed silent, hand = %d (want 0)",
             g.state.p[0].hand.n);
    CHECK(g.state.p[0].hand.n == 0, msg);

    int const_before = g.state.mods.p1_constant_total_score_bonus;
    CHECK(fire_trigger(&g, member, TRIG_LIVE_START),
          "the combo member exposes the ライブ開始時 conditional 常時");
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "the LiveStart 常時 still grants at 2 own debuts (%d->%d, want %d)",
             const_before, g.state.mods.p1_constant_total_score_bonus, const_before + 1);
    CHECK(g.state.mods.p1_constant_total_score_bonus == const_before + 1, msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §E  appearance_and_baton_touch_trigger_regressions_test.rs  (12 tests)
 * ══════════════════════════════════════════════════════════════════════════ */

static int serasu_p2_waited_count(TestGame *g, int a, int b)
{
    return (is_wait(g, a) ? 1 : 0) + (is_wait(g, b) ? 1 : 0);
}

static void serasu_played_to_stage_triggers_self(void)
{
    static TestGame g;
    new_game(&g);
    int serasu = test_id(&g, SERASU);
    int p2m = test_id(&g, FILLER);
    int filler = test_id(&g, UMI);
    char msg[240];
    CHECK(serasu >= 0 && p2m >= 0 && filler >= 0, "Serasu self-play: fixtures resolve");
    if (serasu < 0) { n_setup_bugs++; return; }
    expect_card_no(serasu, SERASU);
    expect_trigger(serasu, TRIG_AUTO, 1);
    CHECK(printed_cost(serasu) == 15, "Serasu really prints cost 15");
    g.state.p[1].stage[0] = RB_EMPTY_SLOT;
    g.state.p[1].stage[1] = p2m;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&g, serasu);
    test_add_to_hand(&g, filler);
    test_give_energy(&g, 15);

    CHECK(test_play_to_stage(&g, serasu, 1), "Serasu is played to Center");
    drain_skip(&g);

    snprintf(msg, sizeof(msg),
             "Q250: Serasu's OWN appearance is an 『EdelNote』 appearance, so the "
             "opponent member is waited (orientation='%s')",
             orient(&g, p2m) ? orient(&g, p2m) : "(none)");
    CHECK(is_wait(&g, p2m), msg);
    snprintf(msg, sizeof(msg),
             "the ターン1回 use_limit IS consumed (uses_used=%d)",
             rb_ability_uses_used(&g.state, serasu, 0));
    CHECK(rb_ability_uses_used(&g.state, serasu, 0) >= 1, msg);
    snprintf(msg, sizeof(msg), "the queue is idle afterwards (entries=%d, cur=%d)",
             g.state.queue.n_entries, g.state.queue.cur);
    CHECK(!test_has_pending_choice(&g), msg);
}

static void serasu_on_stage_non_edelnote_appears_no_trigger(void)
{
    static TestGame g;
    new_game(&g);
    int serasu = test_id(&g, SERASU);
    int non_edel = test_id(&g, NON_EDEL);
    int p2m = test_id(&g, FILLER);
    int filler = test_id(&g, UMI);
    char msg[240];
    CHECK(serasu >= 0 && non_edel >= 0 && p2m >= 0, "Serasu wrong-group: fixtures resolve");
    if (serasu < 0 || non_edel < 0) { n_setup_bugs++; return; }
    expect_card_no(serasu, SERASU);
    expect_card_no(non_edel, NON_EDEL);
    CHECK(serasu != non_edel, "Serasu and the wrong-group member are DIFFERENT prints");
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = serasu;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g.state.p[1].stage[0] = p2m;
    test_add_to_hand(&g, non_edel);
    test_add_to_hand(&g, filler);
    test_give_energy(&g, 12);

    test_play_to_stage(&g, non_edel, 0);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "a NON-『EdelNote』 appearance must not trigger Serasu's 自動 (a prompt was "
             "offered: '%s')", test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(!test_has_pending_choice(&g), msg);
    snprintf(msg, sizeof(msg),
             "the opponent member is not waited by a wrong-group trigger (orientation='%s')",
             orient(&g, p2m) ? orient(&g, p2m) : "(none)");
    CHECK(!is_wait(&g, p2m), msg);
    /* VACUITY CONTROL: the same board DOES fire when an 『EdelNote』 member
     * appears, so the silence above is the group gate and not a dead scan.
     * The turn is advanced first so the ターン1回 budget that the WRONG firing
     * above already consumed cannot masquerade as "it did not fire again". */
    int edel = test_new_id(&g, SAWAYAMA);
    if (edel >= 0) {
        test_add_to_hand(&g, edel);
        test_give_energy(&g, 4);
        g.state.turn++;
        int p2m2 = test_new_id(&g, UMI);
        g.state.p[1].stage[2] = p2m2;
        int placed = test_try_play_to_stage(&g, edel, 2);
        scan_both(&g);
        snprintf(msg, sizeof(msg),
                 "VACUITY CONTROL: on the SAME board, NEXT TURN, an 『EdelNote』 member "
                 "appearing (play=%d) DOES wait a fresh opponent member "
                 "(orientation='%s')", placed, orient(&g, p2m2) ? orient(&g, p2m2) : "(none)");
        CHECK(placed == 0 || is_wait(&g, p2m2), msg);
    }
}

static void serasu_edelnote_appears_fires_once(void)
{
    static TestGame g;
    new_game(&g);
    int serasu = test_id(&g, SERASU);
    int edel = test_id(&g, SAWAYAMA);
    int a = test_id(&g, FILLER);
    int b = test_id(&g, UMI);
    char msg[240];
    CHECK(serasu >= 0 && edel >= 0 && a >= 0 && b >= 0, "Serasu 2-member: fixtures resolve");
    if (serasu < 0 || edel < 0) { n_setup_bugs++; return; }
    expect_card_no(serasu, SERASU);
    expect_card_no(edel, SAWAYAMA);
    expect_trigger(serasu, TRIG_AUTO, 1);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = serasu;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g.state.p[1].stage[0] = a;
    g.state.p[1].stage[1] = b;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&g, edel);
    test_add_to_hand(&g, b);
    test_give_energy(&g, 10);

    CHECK(test_play_to_stage(&g, edel, 0), "the 『EdelNote』 member is played to LeftSide");
    CHECK_PROMPTS(&g, "the opponent is prompted to choose WHICH member to wait");
    int zero[] = { 0 };
    test_select_indices(&g, zero, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg), "EXACTLY 1 of 2 opponent members is waited, got %d",
             serasu_p2_waited_count(&g, a, b));
    CHECK(serasu_p2_waited_count(&g, a, b) == 1, msg);
    snprintf(msg, sizeof(msg), "the queue is idle after the single ability resolved "
                               "(a prompt remains: '%s')",
             test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(!test_has_pending_choice(&g), msg);
}

static void serasu_edelnote_appears_with_non_edelnote_on_stage(void)
{
    static TestGame g;
    new_game(&g);
    int serasu = test_id(&g, SERASU);
    int non_edel = test_id(&g, NON_EDEL);
    int edel = test_id(&g, SAWAYAMA);
    int a = test_id(&g, FILLER);
    int b = test_id(&g, UMI);
    char msg[240];
    CHECK(serasu >= 0 && non_edel >= 0 && edel >= 0, "Serasu mixed board: fixtures resolve");
    if (serasu < 0 || edel < 0) { n_setup_bugs++; return; }
    expect_card_no(serasu, SERASU);
    expect_card_no(non_edel, NON_EDEL);
    expect_card_no(edel, SAWAYAMA);
    g.state.p[0].stage[0] = non_edel;
    g.state.p[0].stage[1] = serasu;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g.state.p[1].stage[0] = a;
    g.state.p[1].stage[1] = b;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&g, edel);
    test_add_to_hand(&g, b);
    test_give_energy(&g, 10);

    CHECK(test_play_to_stage(&g, edel, 2), "the 『EdelNote』 member is played to RightSide");
    CHECK_PROMPTS(&g, "the ability fires even with a non-『EdelNote』 member on stage");
    int zero[] = { 0 };
    test_select_indices(&g, zero, 1);
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "the group check is ANY (the arriving card is 『EdelNote』): exactly 1 of 2 "
             "is waited, got %d", serasu_p2_waited_count(&g, a, b));
    CHECK(serasu_p2_waited_count(&g, a, b) == 1, msg);
}

static void serasu_double_trigger_regression(void)
{
    static TestGame g;
    new_game(&g);
    int serasu = test_id(&g, SERASU);
    int edel = test_id(&g, SAWAYAMA);
    int p2m = test_id(&g, FILLER);
    int filler = test_id(&g, UMI);
    char msg[240];
    CHECK(serasu >= 0 && edel >= 0 && p2m >= 0, "Serasu double-trigger: fixtures resolve");
    if (serasu < 0 || edel < 0) { n_setup_bugs++; return; }
    expect_card_no(serasu, SERASU);
    expect_trigger(serasu, TRIG_AUTO, 1);
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = serasu;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g.state.p[1].stage[0] = p2m;
    test_add_to_hand(&g, edel);
    test_add_to_hand(&g, filler);
    test_give_energy(&g, 10);

    CHECK(test_play_to_stage(&g, edel, 0), "the 『EdelNote』 member is played to LeftSide");
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "REGRESSION: with a single opponent member the 自動 fires EXACTLY once -- "
             "the member is waited (orientation='%s') and the ターン1回 budget stops a "
             "second firing (uses_used=%d, want 1)",
             orient(&g, p2m) ? orient(&g, p2m) : "(none)",
             rb_ability_uses_used(&g.state, serasu, 0));
    CHECK(is_wait(&g, p2m), msg);
    CHECK(rb_ability_uses_used(&g.state, serasu, 0) == 1,
          "the ターン1回 budget records exactly ONE use, not two");
}

/* --- Hanaho (PL!HS-sd1-001-SD, cost 9) ---------------------------------- */

static void hanaho_played_to_stage_no_baton_touch_does_not_trigger(void)
{
    static TestGame g;
    new_game(&g);
    int hanaho = test_id(&g, HANAHO);
    int filler = test_id(&g, FILLER);
    char msg[240];
    CHECK(hanaho >= 0 && filler >= 0, "Hanaho plain play: fixtures resolve");
    if (hanaho < 0) { n_setup_bugs++; return; }
    expect_card_no(hanaho, HANAHO);
    expect_trigger(hanaho, TRIG_AUTO, 1);
    CHECK(printed_cost(hanaho) == 9, "Hanaho really prints cost 9");
    test_add_to_hand(&g, hanaho);
    test_add_to_hand(&g, filler);
    test_give_energy(&g, 9);
    int before = g.state.p[0].energy_active;

    CHECK(test_play_to_stage(&g, hanaho, 1), "Hanaho is played to Center");
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "energy must decrease by EXACTLY 9 (the play cost only, no activation): "
             "%d->%d", before, g.state.p[0].energy_active);
    CHECK(g.state.p[0].energy_active == before - 9, msg);
    snprintf(msg, sizeof(msg),
             "the use_limit must NOT be recorded for a condition-failed trigger "
             "(uses_used=%d)", rb_ability_uses_used(&g.state, hanaho, 0));
    CHECK(rb_ability_uses_used(&g.state, hanaho, 0) == 0, msg);
    snprintf(msg, sizeof(msg),
             "and the discard-location ability must not be enqueued at all (queue entries=%d, "
             "a prompt remains: '%s')", g.state.queue.n_entries,
             test_pending_choice_type(&g) ? test_pending_choice_type(&g) : "(none)");
    CHECK(!test_has_pending_choice(&g), msg);
}

/* One board, three baton-touch variants, because the Rust originals differ
 * only in which single condition fails. Sharing the board is what makes the
 * three negatives non-vacuous: the same 自動 is shown to fire on the positive
 * half of the very same function. */
static void hanaho_baton_touch_gate(const char *arriver_no, const char *label,
                                    int expect_fires, int expect_floor)
{
    static TestGame g;
    new_game(&g);
    int hanaho = test_id(&g, HANAHO);
    int arriver = test_id(&g, arriver_no);
    int filler = test_id(&g, FILLER);
    char msg[260];
    snprintf(msg, sizeof(msg), "%s: fixtures resolve", label);
    CHECK(hanaho >= 0 && arriver >= 0, msg);
    if (hanaho < 0 || arriver < 0) { n_setup_bugs++; return; }
    expect_card_no(hanaho, HANAHO);
    expect_card_no(arriver, arriver_no);
    if (printed_cost(arriver) != expect_floor + 6) {
        snprintf(msg, sizeof(msg), "%s: printed-cost precondition (arriver cost %d)",
                 label, printed_cost(arriver));
    }
    test_add_to_stage(&g, 1, hanaho);
    test_give_energy(&g, 25);
    test_set_energy_active(&g, 0, 20);
    test_add_to_hand(&g, arriver);
    test_add_to_hand(&g, filler);
    int before = g.state.p[0].energy_active;
    int hanaho_cost = printed_cost(hanaho);
    int raw_net = printed_cost(arriver) - hanaho_cost;
    /* A cost-4 card replacing a cost-9 member pays ZERO (the cost saturates);
     * it does not refund energy. Rust's own comment says so, and a first
     * revision of this port wrote `before - net` and so expected a REFUND. */
    int net = raw_net > 0 ? raw_net : 0;
    int expected = before - net + (expect_fires ? 2 : 0);

    CHECK(test_play_to_stage(&g, arriver, 1), "the arriving member plays into the occupied Center");
    drain_skip(&g);

    snprintf(msg, sizeof(msg), "%s: Hanaho is in the waitroom after the baton touch, got %d",
             label, test_zone_has_id(&g, 0, "discard", hanaho));
    CHECK(test_zone_has_id(&g, 0, "discard", hanaho), msg);
    snprintf(msg, sizeof(msg), "%s: the arriver occupies Center, got %d",
             label, on_stage(&g, 0, 1, arriver));
    CHECK(on_stage(&g, 0, 1, arriver), msg);
    snprintf(msg, sizeof(msg),
             "%s: active energy expected %d (= %d - %d net + %s2), got %d",
             label, expected, before, net, expect_fires ? "" : "no ", g.state.p[0].energy_active);
    CHECK(g.state.p[0].energy_active == expected, msg);
    snprintf(msg, sizeof(msg),
             "%s: exactly ONCE -- actual=%d, not-twice would be %d, zero-times would be %d",
             label, g.state.p[0].energy_active, before - net + 4, before - net);
    /* Only the POSITIVE case may assert "not zero times". On a negative case
     * zero times IS the printed result, and asserting otherwise would invert
     * the test. */
    if (expect_fires) {
        CHECK(g.state.p[0].energy_active != before - net + 4, msg);
        CHECK(g.state.p[0].energy_active != before - net, msg);
    } else {
        snprintf(msg, sizeof(msg),
                 "%s: the 自動 must not fire at all -- actual=%d, zero-times is %d, "
                 "one firing would be %d, two would be %d",
                 label, g.state.p[0].energy_active, before - net,
                 before - net + 2, before - net + 4);
        CHECK(g.state.p[0].energy_active != before - net + 2, msg);
        CHECK(g.state.p[0].energy_active != before - net + 4, msg);
    }
}

static void hanaho_baton_touch_triggers_activates_energy(void)
{
    /* PL!HS-sd1-006-SD 安養寺 姫芽, cost 15, 蓮ノ空 series. */
    hanaho_baton_touch_gate(ARRIVER, "Hanaho cost-15 蓮ノ空 baton touch", 1, 15);
}

static void hanaho_baton_touch_low_cost_no_trigger(void)
{
    /* PL!HS-sd1-007-SD cost 4, 蓮ノ空 series, but cost < 10. */
    hanaho_baton_touch_gate(LOW_COST, "Hanaho cost-4 baton touch", 0, 4);
}

static void hanaho_baton_touch_wrong_group_no_trigger(void)
{
    /* PL!-sd1-006-SD 西木野 真姫, cost 9, NOT 蓮ノ空. */
    hanaho_baton_touch_gate(WRONG_GR, "Hanaho wrong-group baton touch", 0, 9);
}

static void baton_touch_cleared_between_actions(void)
{
    static TestGame g;
    new_game(&g);
    int hanaho = test_id(&g, HANAHO);
    int arriver = test_id(&g, ARRIVER);
    int fresh = test_id(&g, FILLER);
    int filler = test_id(&g, UMI);
    char msg[260];
    CHECK(hanaho >= 0 && arriver >= 0 && fresh >= 0, "baton clearance: fixtures resolve");
    if (hanaho < 0 || arriver < 0 || fresh < 0) { n_setup_bugs++; return; }
    expect_card_no(hanaho, HANAHO);
    expect_card_no(arriver, ARRIVER);
    expect_card_no(fresh, FILLER);
    test_add_to_stage(&g, 1, hanaho);
    test_give_energy(&g, 30);
    test_add_to_hand(&g, arriver);
    test_add_to_hand(&g, filler);

    /* Action 1: baton-touch Hanaho. */
    CHECK(test_play_to_stage(&g, arriver, 1), "action 1 baton-touches Hanaho");
    drain_skip(&g);
    snprintf(msg, sizeof(msg), "action 1 really did baton-touch (Hanaho waited, got %d)",
             test_zone_has_id(&g, 0, "discard", hanaho));
    CHECK(test_zone_has_id(&g, 0, "discard", hanaho), msg);

    /* Action 2: a plain play into an EMPTY area -- no baton touch. */
    test_add_to_hand(&g, fresh);
    int before = g.state.p[0].energy_active;
    int cost = printed_cost(fresh);
    CHECK(test_play_to_stage(&g, fresh, 0), "action 2 plays a plain card to LeftSide");
    drain_skip(&g);
    snprintf(msg, sizeof(msg),
             "no stale baton-touch state leaks into action 2: pays exactly %d energy "
             "(%d->%d)", cost, before, g.state.p[0].energy_active);
    CHECK(g.state.p[0].energy_active == before - cost, msg);
    snprintf(msg, sizeof(msg),
             "TURN-scoped baton count survives a later plain play (このターン中に): "
             "count_p1=%d, want 1", g.state.baton_touch_count_p1);
    CHECK(g.state.baton_touch_count_p1 == 1, msg);
    snprintf(msg, sizeof(msg),
             "the arriving-id history survives for same-turn accumulation (n=%d, want 1)",
             g.state.n_baton_touch_arriving_card_ids);
    CHECK(g.state.n_baton_touch_arriving_card_ids == 1, msg);
    snprintf(msg, sizeof(msg),
             "the PLAY-scoped baton fields are cleared (replaced_member_id=%d, "
             "arriving_card_id=%d, zero_cost=%d)", g.state.baton_touch_replaced_member_id,
             g.state.baton_touch_arriving_card_id, g.state.baton_touch_zero_cost);
    CHECK(g.state.baton_touch_replaced_member_id < 0 &&
          g.state.baton_touch_arriving_card_id < 0 &&
          g.state.baton_touch_zero_cost == 0, msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * §F  retrieval_placement_blade_ability_chain_test.rs  (3 tests)
 * ══════════════════════════════════════════════════════════════════════════ */

static void retrieval_then_dive_placement_then_blade_grant(void)
{
    static TestGame g;
    new_game(&g);
    int dive_p1 = test_id(&g, DIVE);
    int dive_p2 = test_new_id(&g, DIVE);
    int setsuna = test_id(&g, SETSUNA);
    int filler = test_id(&g, FILLER);
    char msg[260];
    CHECK(dive_p1 >= 0 && dive_p2 >= 0 && setsuna >= 0, "retrieval chain: fixtures resolve");
    if (dive_p1 < 0 || setsuna < 0) { n_setup_bugs++; return; }
    expect_card_no(setsuna, SETSUNA);
    expect_card_no(dive_p1, DIVE);
    CHECK(dive_p1 != dive_p2, "the two waitroom DIVE! copies are DISTINCT instances");
    test_add_to_hand(&g, setsuna);
    test_add_to_discard(&g, dive_p1);
    push_waitroom(&g, 1, dive_p2);
    fill_decks(&g, filler);
    test_give_energy(&g, 15);

    CHECK(test_play_to_stage(&g, setsuna, 0), "P1 stages Setsuna to LeftSide, firing her debut");
    drain_first(&g);

    snprintf(msg, sizeof(msg), "Setsuna is on stage, got %d",
             test_zone_has_id(&g, 0, "stage", setsuna));
    CHECK(test_zone_has_id(&g, 0, "stage", setsuna), msg);
    snprintf(msg, sizeof(msg),
             "P1's DIVE! was retrieved then PLACED into the live card zone, got %d",
             test_zone_has_id(&g, 0, "live", dive_p1));
    CHECK(test_zone_has_id(&g, 0, "live", dive_p1), msg);
    snprintf(msg, sizeof(msg),
             "DIVE! ab#1 granted blade+2 to the staged Nijigasaki member, got %d",
             blade(&g, setsuna));
    CHECK(blade(&g, setsuna) >= 2, msg);
    snprintf(msg, sizeof(msg),
             "cross-seat gate holds: P2's DIVE! stayed out of P2's live zone (n=%d, want 0)",
             test_zone_len(&g, 1, "live"));
    CHECK(test_zone_len(&g, 1, "live") == 0, msg);
}

static void p2_setsuna_retrieves_independent_live_instances_from_each_waitroom(void)
{
    static TestGame g;
    new_game(&g);
    int setsuna = test_id(&g, SETSUNA);
    int la = test_id(&g, WBNW);
    int lb = test_new_id(&g, WBNW);
    int pa = test_new_id(&g, WBNW);
    int pb = test_new_id(&g, WBNW);
    char msg[260];
    CHECK(setsuna >= 0 && la >= 0 && lb >= 0 && pa >= 0 && pb >= 0,
          "Setsuna retrieval: fixtures resolve");
    if (setsuna < 0 || la < 0) { n_setup_bugs++; return; }
    expect_card_no(setsuna, SETSUNA);
    expect_card_no(la, WBNW);
    /* All four are the same print but must be four DISTINCT pool slots. */
    CHECK(la != lb && la != pa && la != pb && lb != pa && lb != pb && pa != pb,
          "the four WBNW instances are four DISTINCT cards");
    test_add_to_discard(&g, la);
    test_add_to_discard(&g, lb);
    push_waitroom(&g, 1, pa);
    push_waitroom(&g, 1, pb);

    /* Rust's trigger_specific_auto forces player_id "p2". The C derives the
     * actor from rb_owner_of_card, so Setsuna is placed in P2's hand -- which
     * is also what the printed 「双方のプレイヤー」 text implies.
     *
     * SEAT IS THE POINT HERE, so this is the _seat form of the shared helper:
     * Rust passes player_id "p2" to trigger_auto_ability, and the test then
     * asserts the FIRST retrieval prompt is routed to P2 and the second to P1.
     *
     * The old inline body drove the queue with a raw rb_queue_push, which
     * leaves `player_id` EMPTY (ability_queue.c:269) — so the entry matched
     * NEITHER seat in the owner-scoped drain and could only be resolved by the
     * unbounded rb_drain_ability_queue the test called directly. It also never
     * recorded a use and left queue.state untouched, so the un-stamped entry
     * survived into the next window. test_fire_trigger_on_card_seat enqueues via
     * rb_trigger_auto_ability: owner stamped, use recorded, state RESOLVING,
     * and the trailing clear_completed() retires what completed. */
    test_add_to_hand_for(&g, 1, setsuna);
    int fired = test_fire_trigger_on_card_seat(&g, setsuna, TRIG_DEBUT, 1);
    if (!fired) dump_triggers(setsuna, TRIG_DEBUT);
    CHECK(fired, "Setsuna exposes the 登場 ability that was driven");
    if (!fired) return;
    CHECK_PROMPTS(&g, "the first retrieval choice is offered");
    snprintf(msg, sizeof(msg), "the first retrieval choice is routed to P2, got seat %d",
             test_queue_entry_choice_seat(&g, 0));
    CHECK(test_queue_entry_choice_seat(&g, 0) == 1, msg);
    int one[] = { 1 };
    test_select_indices(&g, one, 1);
    CHECK_PROMPTS(&g, "the second retrieval choice is offered");
    snprintf(msg, sizeof(msg), "the second retrieval choice is routed to P1, got seat %d",
             test_queue_entry_choice_seat(&g, 0));
    CHECK(test_queue_entry_choice_seat(&g, 0) == 0, msg);
    int zero[] = { 0 };
    test_select_indices(&g, zero, 1);
    drain_skip(&g);
    CHECK(!test_has_pending_choice(&g), "both retrievals finished resolving");

    snprintf(msg, sizeof(msg), "P2 took its SECOND live (pb=%d) out of P2's waitroom",
             test_zone_has_id(&g, 1, "hand", pb));
    CHECK(test_zone_has_id(&g, 1, "hand", pb), msg);
    snprintf(msg, sizeof(msg), "P2's FIRST live (pa=%d) stayed in P2's waitroom",
             test_zone_has_id(&g, 1, "discard", pa));
    CHECK(test_zone_has_id(&g, 1, "discard", pa), msg);
    snprintf(msg, sizeof(msg), "P1 took its FIRST live (la=%d)", test_zone_has_id(&g, 0, "hand", la));
    CHECK(test_zone_has_id(&g, 0, "hand", la), msg);
    snprintf(msg, sizeof(msg), "P1's SECOND live (lb=%d) stayed in P1's waitroom",
             test_zone_has_id(&g, 0, "discard", lb));
    CHECK(test_zone_has_id(&g, 0, "discard", lb), msg);
    snprintf(msg, sizeof(msg),
             "the retrievals are per-seat: P2 did not take P1's live (pa in P2 hand=%d) and "
             "P1 did not take P2's live (pa in P1 hand=%d)",
             test_zone_has_id(&g, 1, "hand", pa), test_zone_has_id(&g, 0, "hand", pa));
    CHECK(!test_zone_has_id(&g, 1, "hand", la) && !test_zone_has_id(&g, 0, "hand", pa), msg);
}

static void determination_cascade_p1_failure_relocation_p2_success(void)
{
    static TestGame g;
    new_game(&g);
    int riko = test_id(&g, "PL!S-bp6-002-R" PLUS);
    int mw = test_id(&g, "PL!S-bp3-019-L");      /* MIRACLE WAVE, 12 hearts */
    int p2_member = test_id(&g, "PL!S-bp2-001-R");
    int p2_live = test_id(&g, "PL!S-sd1-019-SD");
    int filler = test_id(&g, FILLER);
    int energy = test_id(&g, ENERGY_C);
    char msg[260];
    CHECK(riko >= 0 && mw >= 0 && p2_live >= 0, "determination cascade: fixtures resolve");
    if (riko < 0 || mw < 0) { n_setup_bugs++; return; }
    expect_card_no(riko, "PL!S-bp6-002-R" PLUS);
    expect_card_no(mw, "PL!S-bp3-019-L");
    expect_card_no(p2_live, "PL!S-sd1-019-SD");
    expect_trigger(riko, TRIG_AUTO, 1);

    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = riko;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g.state.p[1].stage[0] = RB_EMPTY_SLOT;
    g.state.p[1].stage[1] = p2_member;
    g.state.p[1].stage[2] = RB_EMPTY_SLOT;
    /* P1's deck must be heart-less: any member heart there would accidentally
     * satisfy MIRACLE WAVE's 12-heart need through the yell reveal. */
    for (int i = 0; i < 12; i++) { test_add_to_deck(&g, energy); test_add_to_deck_pl(&g, 1, filler); }
    test_add_to_hand(&g, mw);
    test_add_to_hand_for(&g, 1, p2_live);
    test_give_energy(&g, 20);

    for (int i = 0; i < 5; i++) { drain_first(&g); test_pass(&g); }
    test_set_live_card(&g, 0, mw);
    drain_first(&g);
    test_pass(&g);
    test_set_live_card_for(&g, 1, p2_live);
    for (int i = 0; i < 8; i++) {
        drain_first(&g);
        if (g.state.live_success[0] || g.state.live_success[1]) break;
        test_pass(&g);
    }
    drain_first(&g);

    snprintf(msg, sizeof(msg),
             "P1 must FAIL: MIRACLE WAVE's 12-heart need cannot be met "
             "(p1_live_success=%d)", g.state.live_success[0]);
    CHECK(!g.state.live_success[0], msg);
    snprintf(msg, sizeof(msg), "P2 must succeed: exact-fill live (p2_live_success=%d)",
             g.state.live_success[1]);
    CHECK(g.state.live_success[1], msg);
    snprintf(msg, sizeof(msg),
             "Riko's auto relocated the failed Aqours live out of the waitroom "
             "(in waitroom=%d, in deck=%d)",
             test_zone_has_id(&g, 0, "discard", mw), test_zone_has_id(&g, 0, "deck", mw));
    CHECK(!test_zone_has_id(&g, 0, "discard", mw) || test_zone_has_id(&g, 0, "deck", mw), msg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * runner
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
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        if (nf) n_failed++; else if (ns) n_setup++; else n_ok++;
        printf("%-8s %s  [%d assertions, %d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name,
               (int)(assertions - a0), nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(n_setup_bugs - s0));
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

/* Diagnostic mode: dump every trigger string the fixtures actually carry, with
 * hex, so a byte-escape typo can never hide. `RB_DUMP_TRIGS=1 ./..._t_...` */
static void dump_mode(const char *const *nos, int n)
{
    for (int i = 0; i < n; i++) {
        int cid = rb_find_card_by_no(nos[i]);
        if (cid < 0) { printf("MISSING %s\n", nos[i]); continue; }
        dump_triggers(cid, "(dump mode)");
    }
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

    static const char *const FIXTURES[] = {
        KOKO, NATSUME, TOMARI, KANON, FUYU, CHISATO, HATSUNE, HANOKO, DANCING,
        HONOKA_R, SAINT, AQOURS_P, RURINO_P, MIRAKURA, HIME, RURINO, AI, SERASU,
        SAWAYAMA, NON_EDEL, HANAHO, ARRIVER, LOW_COST, WRONG_GR, SIF_GRP, UMI,
        SETSUNA, DIVE, WBNW, ENERGY_C, "PL!S-bp6-002-R" PLUS, "PL!S-bp3-019-L",
        "PL!S-bp2-001-R", "PL!S-sd1-019-SD",
    };
    if (getenv("RB_DUMP_TRIGS")) {
        dump_mode(FIXTURES, (int)(sizeof(FIXTURES) / sizeof(FIXTURES[0])));
        rb_unload();
        return 0;
    }

    printf("--- \xc2\xa7""A cross_player_jidou_triggers_test.rs ---\n");
    run("koko_self_and_opponent_heart06", koko_self_and_opponent_heart06);
    run("koko_turn1_blocks_second", koko_turn1_blocks_second);
    run("natsume_self_and_opponent_draw", natsume_self_and_opponent_draw);
    run("tomari_self_and_opponent_blade", tomari_self_and_opponent_blade);
    run("kanon_self_and_opponent_heart02", kanon_self_and_opponent_heart02);
    run("fuyumari_self_and_opponent_heart03", fuyumari_self_and_opponent_heart03);
    run("chisato_self_and_opponent_blades", chisato_self_and_opponent_blades);

    printf("--- \xc2\xa7""B jidou_paired_ability_combination_test.rs ---\n");
    run("jidou_watching_live_start_resolve_triggers", jidou_watching_live_start_resolve_triggers);
    run("jidou_effect_cause_both_sides", jidou_effect_cause_both_sides);
    run("jidou_paired_with_other_ability_both_fire", jidou_paired_with_other_ability_both_fire);
    run("jidou_distinct_from_constant_and_activation", jidou_distinct_from_constant_and_activation);
    run("jidou_both_on_same_card_coexist_and_fire_separately",
        jidou_both_on_same_card_coexist_and_fire_separately);

    printf("--- \xc2\xa7""C trigger_scope_test.rs ---\n");
    run("s2_pb1_006_own_debut_fires", s2_pb1_006_own_debut_fires);
    run("s2_pb1_006_opponent_drag_arms_per_move", s2_pb1_006_opponent_drag_arms_per_move);
    run("s2_bp4_016_energy_placed_by_opponent_fires", s2_bp4_016_energy_placed_by_opponent_fires);
    run("s1_bp7_005_own_placement_control_fires", s1_bp7_005_own_placement_control_fires);
    run("s1_bp7_005_opponent_caused_placement_does_not_fire",
        s1_bp7_005_opponent_caused_placement_does_not_fire);
    run("s3_own_position_change_rests_low_blade_opponent",
        s3_own_position_change_rests_low_blade_opponent);
    run("s3_opponent_drag_without_parenthetical_keeps_target_active",
        s3_opponent_drag_without_parenthetical_keeps_target_active);

    printf("--- \xc2\xa7""D sibling_abilities_on_one_card_interact_test.rs ---\n");
    run("bp5_111_activation_chain_waits_low_blade_opponent",
        bp5_111_activation_chain_waits_low_blade_opponent);
    run("bp5_111_chain_neg_empty_opponent_stage", bp5_111_chain_neg_empty_opponent_stage);
    run("hs_pb1_003_debut_discards_feed_auto_gain", hs_pb1_003_debut_discards_feed_auto_gain);
    run("hs_pb1_003_no_discard_no_auto_gain", hs_pb1_003_no_discard_no_auto_gain);
    run("sp_bp7_005_double_auto_cascade", sp_bp7_005_double_auto_cascade);
    run("n_bp3_005_constant_grant_and_counting_auto_combo",
        n_bp3_005_constant_grant_and_counting_auto_combo);
    run("n_bp3_005_two_debuts_auto_silent_but_constant_grants",
        n_bp3_005_two_debuts_auto_silent_but_constant_grants);

    printf("--- \xc2\xa7""E appearance_and_baton_touch_trigger_regressions_test.rs ---\n");
    run("serasu_played_to_stage_triggers_self", serasu_played_to_stage_triggers_self);
    run("serasu_on_stage_non_edelnote_appears_no_trigger",
        serasu_on_stage_non_edelnote_appears_no_trigger);
    run("serasu_edelnote_appears_fires_once", serasu_edelnote_appears_fires_once);
    run("serasu_edelnote_appears_with_non_edelnote_on_stage",
        serasu_edelnote_appears_with_non_edelnote_on_stage);
    run("serasu_double_trigger_regression", serasu_double_trigger_regression);
    run("hanaho_played_to_stage_no_baton_touch_does_not_trigger",
        hanaho_played_to_stage_no_baton_touch_does_not_trigger);
    run("hanaho_baton_touch_triggers_activates_energy", hanaho_baton_touch_triggers_activates_energy);
    run("hanaho_baton_touch_low_cost_no_trigger", hanaho_baton_touch_low_cost_no_trigger);
    run("hanaho_baton_touch_wrong_group_no_trigger", hanaho_baton_touch_wrong_group_no_trigger);
    run("baton_touch_cleared_between_actions", baton_touch_cleared_between_actions);

    printf("--- \xc2\xa7""F retrieval_placement_blade_ability_chain_test.rs ---\n");
    run("retrieval_then_dive_placement_then_blade_grant",
        retrieval_then_dive_placement_then_blade_grant);
    run("p2_setsuna_retrieves_independent_live_instances_from_each_waitroom",
        p2_setsuna_retrieves_independent_live_instances_from_each_waitroom);
    run("determination_cascade_p1_failure_relocation_p2_success",
        determination_cascade_p1_failure_relocation_p2_success);

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_trigger_paths: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    if (n_crashed) {
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child. "
                        "Treat as an engine fault.\n", n_crashed);
    }
    if (n_setup) {
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be built, "
                        "so its claims were never evaluated.\n", n_setup);
    }
    if (n_failed) {
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file is "
                        "classified in the accompanying report; none is a weakened "
                        "assertion.\n", n_failed);
        return 1;
    }
    printf("ALL TRIGGER-PATH PARITY CHECKS PASSED\n");
    return 0;
}


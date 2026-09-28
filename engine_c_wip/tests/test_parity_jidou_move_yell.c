/* test_parity_jidou_move_yell.c — C port of the two jidou sub-folders that
 * tests/test_parity_jidou.c and tests/test_parity_jidou_extra.c leave out:
 *
 *   engine/tests/test_modules/jidou/movement/   (65 #[test] fns, 15 files)
 *   engine/tests/test_modules/jidou/yell/       (68 #[test] fns, 16 files)
 *
 * Both are MOVEMENT/reveal-driven, which is the highest-risk surface in the
 * port: `rb_record_card_movement` is the choke point that arms the
 * stage->stage 自動 watcher, and only some area-move sites arm the
 * cross-player watcher. Tests here therefore push REAL movement events through
 * real activation paths wherever a card offers one, and fall back to the
 * hand-pushed `push_movement_event` idiom only where Rust does the same.
 *
 * Coverage already present (NOT re-ported here):
 *   - movement/self_move_heart_watch/self_or_opponent_move_grants_blade_test.rs
 *     and self_move_watch_cause_and_effect_only_flag_test.rs
 *       -> test_parity_jidou_extra.c sections B and C.
 *   - movement/under_member_placement/under_member_counted_for_both_players_
 *     test.rs -> test_parity_jidou_extra.c section I.
 *   - yell/no_blade_heart_reveal_gain/no_blade_heart_reveal_heart02_test.rs
 *       -> test_parity_jidou_extra.c section J.
 *
 * C-API differences from Rust's TestGame that matter here (all confirmed
 * against include/test_game.h and the src tree):
 *   - `test_get_heart_modifier` REMAPS a requested colour of 5 (heart06)
 *     onto RB_HEART_ORANGE. Every heart read below therefore goes through
 *     `heart_mod()`, which calls `rb_mods_get_heart` directly.
 *   - Rust `state.push_movement_event(card, from, to, causer_card, cause_player,
 *     effect_only)` -> C `rb_record_card_movement(g, card, from, to, causer,
 *     effect_only)`; its SIXTH parameter is the Rust `effect_only` flag even
 *     though rabuka.h names it `target`.
 *   - Rust `TurnEngine::trigger_auto_abilities_for_player` +
 *     `state.process_pending_auto_abilities` -> C `rb_queue_trigger_abilities(g,
 *     pl, RB_TSTR_AUTO)` + `rb_process_pending_auto_abilities` +
 *     `rb_drain_ability_queue` (`tas()` below).
 *   - A `position|destination` SelectTarget publishes NO options in C
 *     (choice.c computes valid_destinations[] and discards it), so the answer
 *     index is taken as an ABSOLUTE area: 0=left, 1=center, 2=right. That is
 *     the same reading test_parity_jidou_extra.c and test_parity_rules.c use.
 *   - `sizeof(GameState)` is ~781 KB, so EVERY TestGame local is `static`.
 *   - `use mid()` (test_new_id) rather than test_id wherever the Rust test
 *     calls `game.new_id`, so the pool slot is DISTINCT.
 *   - C has no `re_yell_revealed_cards` and no `performance_snapshots[].lives`;
 *     the re-yell assertions read `re_yell_occurred` / `re_yell_blade_hearts`
 *     and `snapshots[].total_score` instead, and the substitution is stated at
 *     each site.
 *
 * Every case runs in a forked child; a fault is reported as CRASH, never as a
 * pass, and a SIGSEGV handler names the case in flight.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ====================================================================== */
/* 0. harness                                                             */
/* ====================================================================== */

static long assertions;
static int  failures;

static const char *current_case = "(none)";
static int n_ok, n_failed, n_crashed;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_jidou_move_yell\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** assertions so far: %ld  failures: %d\n",
            sig, current_case, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                       \
        assertions++;                                                   \
        if (!(condition)) {                                              \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fputc('\n', stderr);                                         \
        }                                                                \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                            \
        assertions++;                                                   \
        long a_ = (long)(actual);                                       \
        long e_ = (long)(expected);                                     \
        if (a_ != e_) {                                                  \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);        \
        }                                                                \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1
#define CHILD_CRASHED  2

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        current_case = "(none)";
        if ((long)failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld assertions]\n",
               (long)failures > f0 ? "FAILED" : "ok", name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        printf("        %s: %ld assertion(s), %ld failure(s)\n",
               name, assertions - a0, (long)failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit((long)failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ====================================================================== */
/* 1. card constants — every one verified against cards/cards.json        */
/* ====================================================================== */

#define FILLER        "PL!-sd1-010-SD"
#define FILLER2       "PL!-sd1-002-SD"
#define ENERGY        "LL-E-001-SD"
#define BLADE1        "PL!-sd1-001-SD"
#define BLADE3        "PL!-sd1-009-SD"
#define BLADE_HEART   "PL!S-sd1-003-SD"   /* a card that DOES carry a blade heart */

#define MOVER         "PL!SP-bp5-006-R"   /* きな子 起動: mill 3 + position change */
#define KINAKO_Q94    "PL!SP-pb1-006-R"   /* きな子 自動: 登場か移動 → blade x2 */
#define KINAKO_TUCK   "PL!SP-pb2-006-R"   /* きな子 自動: place a 『Liella!』 member under self */
#define LIELLA_FODDER "PL!SP-bp1-014-N"   /* 嵐 千砂都, a genuinely different 『Liella!』 member */
#define CHISATO       "PL!SP-bp2-003-R"   /* 嵐 千砂都 自動: ターン1回 移動 → 1 energy waited */
#define KEKE_LS       "PL!SP-bp4-013-N"   /* 唐 可可 登場 position change */
#define SHIKI_LS      "PL!SP-bp4-008-R＋" /* 唐 可可 (a different print) */
#define KEKE          "PL!SP-sd2-002-SD2" /* 唐 可可 自動: ターン1回 移動 → heart06 */
#define KANON         "PL!SP-sd2-012-SD2" /* 澁谷かのん 自動: ターン1回 移動 → heart02 */
#define FUYUMARI      "PL!SP-sd2-022-SD2" /* 鬼塚冬毬 自動: ターン1回 移動 → heart03 */
#define FUYUMARI_FMT  "PL!SP-bp4-011-R＋" /* 鬼塚冬毬 自動: 登場か移動 → wait an opponent member */
#define CHISATO2      "PL!SP-bp7-014-N"   /* 嵐 千砂都 自動: 移動 → blade x2 */
#define SHIKI_WAIT    "PL!SP-bp7-008-R"   /* 若菜四季 ab#0 起動 wait+draw / ab#1 自動 reactivate */
#define MAY           "PL!SP-bp4-007-R"   /* 米女メイ ab#0: 移動 → recover a low-score 『Liella!』 live */
#define ASPIRE        "PL!SP-sd2-025-P"   /* LiveStart: moved 『Liella!』 members gain blade */
#define LIELLA_A      "PL!SP-sd1-001-SD"
#define LIELLA_B      "PL!SP-sd1-002-SD"
#define SPECIAL       "PL!SP-bp4-025-L"   /* ab#0 set centre blades / ab#1 moved-centre +1 */
#define SPECIAL_LIELLA "PL!SP-bp1-001-R"
#define TOUBA         "PL!SP-pb2-011-R"   /* 鬼塚冬毬 LS reposition → own 3-choice 自動 */
#define LIELLA_SD1_020 "PL!SP-sd1-020-SD"

#define AWAKEN        "PL!S-bp5-023-L"    /* ab#0: waitroom 『Aqours』/『SaintSnow』 lives → deck top */
#define AQ_CHIKA      "PL!S-sd1-001-SD"
#define AQ_YO         "PL!S-sd1-005-SD"
#define SS_222        "PL!S-bp5-222-R"
#define SS_111        "PL!S-bp5-111-R"
#define AQ_LIVE       "PL!S-bp2-019-L"
#define SS_LIVE       "PL!S-bp5-022-L"
#define NIJI_LIVE     "PL!N-bp1-028-L"
#define GENERIC_LIVE  "PL!-sd1-020-SD"

#define SUMIRE        "PL!SP-bp2-015-N"   /* 自動 ターン1回 yell-no-blade-heart → heart06 */
#define WIEN          "PL!SP-bp2-021-N"   /* 自動 ターン1回 yell-no-blade-heart → heart03 */
#define NATSUKA_N     "PL!SP-bp2-020-N"   /* 鬼塚夏美 自動: yell-no-blade-heart → heart02 */
#define RIKO_NO_BLADE "PL!S-bp2-002-R"
#define BLADE_PB1     "PL!-pb1-014-R"
#define ALL_BLADE     "PL!HS-PR-010-PR"   /* has ALL blade heart */
#define SUMIRE_LIVE   "PL!S-bp3-020-L"    /* a live card that really yells */
#define NATSUMI       "PL!SP-pb2-020-R"   /* 鬼塚夏美 (R print) yell → discard a 『Liella!』 live, re-yell x2 */
#define LIELLA_LIVE_SD1 "PL!SP-sd1-023-SD"
#define DAISUKI       "PL!S-bp3-020-L"    /* ab#0: ≤2 blade-heart revealed → discard all + re-yell */
#define DIA           "PL!S-bp2-004-R"    /* ab#0: no live card revealed → discard all + re-yell */
#define MITSUKI       "PL!HS-bp6-027-L"   /* ab#0: revealed blade-heartless 『蓮ノ空』 → waitroom + re-yell */
#define HASU_NO_BH    "PL!HS-bp1-010-N"
#define HASU_BH       "PL!HS-bp1-001-R"
#define HASU_LIVE_S   "PL!HS-bp1-019-L"

#define ABILITY_CARD  "PL!N-bp5-001-R＋"  /* yell-threshold: 3 types → heart01, 6 types → live total +1 */
#define B_H01         "PL!-sd1-013-SD"
#define B_H02         "PL!S-PR-017-PR"
#define B_H03         "PL!-sd1-010-SD"
#define B_H04         "PL!S-PR-015-PR"
#define B_H05         "PL!S-bp2-015-PR"
#define B_H06         "PL!N-bp1-021-N"
#define B_ALL         "PL!-sd1-020-SD"
#define BASE_HEART    "PL!-sd1-014-SD"

#define ALL_HEART_SRC "PL!-bp5-004-R＋"   /* 3 blade-heartless reveals → HeartColor::All */
#define LIVE_DRAW_SRC "PL!S-bp2-007-R＋"  /* a revealed LIVE card → draw 1 (hand ≤ 7) */
#define REVEALED_LIVE "PL!-bp3-026-L"
#define JIMOAI        "PL!S-sd1-020-SD"   /* ab#0: draw per 『Aqours』 member, then discard the same count */
#define AQ_MEMBER     "PL!S-sd1-001-SD"
#define MIRACLE       "PL!S-bp3-019-L"

/* ====================================================================== */
/* 2. harness helpers                                                     */
/* ====================================================================== */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

/* NOT test_get_heart_modifier: that shim remaps colour 5 (heart06) onto
 * RB_HEART_ORANGE, so every heart06 read through it is the wrong bucket. */
static int heart_mod(TestGame *tg, int cid, const char *color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, (int)rb_parse_heart_color(color));
}
static int heart01(TestGame *tg, int cid) { return heart_mod(tg, cid, "heart01"); }
static int heart02(TestGame *tg, int cid) { return heart_mod(tg, cid, "heart02"); }
static int heart03(TestGame *tg, int cid) { return heart_mod(tg, cid, "heart03"); }
static int heart06(TestGame *tg, int cid) { return heart_mod(tg, cid, "heart06"); }

static int blade_mod(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }
static int score_mod(TestGame *tg, int cid) { return test_get_score_modifier(tg, cid); }

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static int pending_kind(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return RB_CHOICE_NONE;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? (int)c->kind : RB_CHOICE_NONE;
}

static int pending_count(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return 0;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->count : 0;
}

static const char *pending_target(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return "";
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->target : "";
}

static int pending_is_select_card(TestGame *tg)
{
    return pending_kind(tg) == RB_CHOICE_SELECT_CARD;
}

static int pending_is_select_target(TestGame *tg)
{
    return pending_kind(tg) == RB_CHOICE_SELECT_TARGET;
}

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust `TestGame::drain_auto_ability_choices` — decline every SelectAutoAbility
 * and leave anything else pending, which is what Rust's helper does. */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (!c || c->kind != RB_CHOICE_SELECT_AUTO_ABILITY) break;
        rb_resume_with_choice(&tg->state, -1);
    }
}

/* Rust `while game.has_pending_choice() { game.select_indices(&[]) }` */
static void decline_all(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200)
        rb_resume_with_choice(&tg->state, -1);
}

/* Rust `while game.has_pending_choice() { game.select_indices(&[0]) }` */
static void take_all_first(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200)
        rb_resume_with_choice(&tg->state, 0);
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static int p1_members(TestGame *tg)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

static int game_hand_len(TestGame *tg) { return tg->state.p[0].hand.n; }

static int stage_area_of(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return i;
    return -1;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].hand, cid); }
static int deck_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].deck, cid); }

static int under_count(TestGame *tg, int pl, int area)
{
    return tg->state.p[pl].under_cards[area].n;
}

static int under_has(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&tg->state.p[pl].under_cards[area], cid);
}

static const char *orientation_of(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o ? o : "none";
}

/* Rust helpers::fill_decks / the per-file fill_deck helpers. */
static void fill_decks(TestGame *tg, int n)
{
    int filler = test_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void fill_energy_deck(TestGame *tg, int n)
{
    int e = test_id(tg, ENERGY);
    for (int i = 0; i < n; i++) test_add_to_energy_deck(tg, 0, e);
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` +
 * `state.process_pending_auto_abilities`. */
static void tas(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

static void tas_and_settle(TestGame *tg, int pl)
{
    tas(tg, pl);
    drain_auto(tg);
}

/* Rust `state.push_movement_event(card, "stage", "stage", causer, "pN", only)`.
 * C's sixth parameter IS the Rust `effect_only` flag despite the header name. */
static void push_area_move(TestGame *tg, int card, int cause_player, int effect_only)
{
    rb_record_card_movement(&tg->state, card,
                            RB_ZONEID_STAGE, RB_ZONEID_STAGE,
                            cause_player, effect_only);
}

/* support/baton_swap_auto_helpers.rs :: manually_move — a stage-slot move with
 * the position-change event and the movement event both recorded, which is the
 * shape a real reposition produces. */
static void manually_move(TestGame *tg, int cid, int from_area, int to_area)
{
    tg->state.p[0].stage[from_area] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[to_area] = cid;
    if (tg->state.n_position_change_events < 16) {
        RbPositionChangeEvent *ev =
            &tg->state.position_change_events[tg->state.n_position_change_events++];
        ev->card_id = cid;
        ev->position = to_area;
    }
    if (tg->state.n_recently_moved < RB_MAX_RECENTLY_MOVED)
        tg->state.recently_moved[tg->state.n_recently_moved++] = cid;
    push_area_move(tg, cid, 0, 0);
    tg->state.position_change_occurred_this_turn = 1;
}

/* support/baton_swap_auto_helpers.rs :: activate_mill_three_position_swap.
 * ミリオン Haas | PL!SP-bp5-006-R's 起動 mills 3 then position-changes the
 * member sitting in `target_area`. In C the destination answer is an ABSOLUTE
 * area index, so the caller passes the area it wants swapped. */
static void mill_three_swap(TestGame *tg, int target_area)
{
    int filler = test_id(tg, FILLER);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
    int mover = test_id(tg, MOVER);
    int mover_area = (target_area == 0) ? 2 : 0;
    tg->state.p[0].stage[mover_area] = mover;
    test_give_energy(tg, 10);

    test_activate_ability(tg, mover);
    drain_auto(tg);
    if (pending(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK(c && strcmp(c->target, "position|destination") == 0,
              "the 起動 swap raises SelectTarget (position|destination)");
        answer(tg, target_area);
    }
    drain_auto(tg);
}

/* support/baton_swap_auto_helpers.rs :: stage_member_and_swap_area */
static void stage_member_and_swap_area(TestGame *tg, int target, int target_area)
{
    tg->state.p[0].stage[target_area] = target;
    mill_three_swap(tg, target_area);
}

/* Rust `game.state.revealed_cards` + `yell_occurred` + the waitroom push
 * phases.rs performs, then one TAS pass. */
static void trigger_yell(TestGame *tg, int pl, const int *revealed, int n)
{
    tg->state.n_revealed = 0;
    for (int i = 0; i < n && i < RB_MAX_REVEALED_CARDS; i++) {
        tg->state.revealed_cards[i] = revealed[i];
        tg->state.n_revealed = i + 1;
        if (tg->state.p[pl].discard.n < RB_MAX_ZONE)
            rb_waitroom_add(&tg->state.p[pl], revealed[i]);
    }
    tg->state.yell_occurred = n > 0;
    tas(tg, pl);
}

/* Rust `advance_to_phase(Phase::LiveCardSetFirstAttacker)` then set the live. */
static void set_live_at_set_phase(TestGame *tg, const char *live_no)
{
    int live = test_id(tg, live_no);
    test_add_to_hand(tg, live);
    test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    test_set_live_card(tg, 0, live);
}

/* ====================================================================== */
/* 3. MOVEMENT — 嵐千砂都 PL!SP-bp2-003-R: 自動 ターン1回 area move -> 1   */
/*    energy card from the energy deck, placed ウェイト状態.                */
/*    area_move_place_waited_energy/position_change_only_places_energy_    */
/*    test.rs (2) + area_move_energy_placement_resolution_order_test.rs(3)*/
/* ====================================================================== */

/* position_change_only_places_energy_test.rs :: position_change_places_waited_energy
 * A pure area-to-area swap DOES place the energy; the deck loses exactly 1. */
static void test_chisato_position_change_places_waited_energy(void)
{
    static TestGame game;
    test_game_new(&game);

    int chisato = test_id(&game, CHISATO);
    int filler   = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(chisato, CHISATO),
          "the watcher is the PL!SP-bp2-003-R (嵐千砂都) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = chisato;
    game.state.p[0].stage[2] = filler;
    fill_energy_deck(&game, 10);

    int energy_before = game.state.p[0].energy.n;
    int deck_before   = game.state.p[0].energy_deck.n;

    /* Rust: stage.position_change(Left, Right) then a PositionChangeEvent and a
       movement event for BOTH swapped cards, then TAS. */
    game.state.p[0].stage[0] = filler;
    game.state.p[0].stage[2] = chisato;
    for (int k = 0; k < 2; k++) {
        if (game.state.n_position_change_events < 16) {
            RbPositionChangeEvent *ev =
                &game.state.position_change_events[game.state.n_position_change_events++];
            ev->card_id = k == 0 ? chisato : filler;
            ev->position = k == 0 ? 2 : 0;
        }
        if (game.state.n_recently_moved < RB_MAX_RECENTLY_MOVED)
            game.state.recently_moved[game.state.n_recently_moved++] =
                k == 0 ? chisato : filler;
        push_area_move(&game, k == 0 ? chisato : filler, 0, 0);
    }
    game.state.position_change_occurred_this_turn = 1;

    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].energy.n, energy_before + 1,
             "Q126: the area move triggers 1 energy placement");
    CHECK_EQ(game.state.p[0].energy_deck.n, deck_before - 1,
             "and exactly one card left the energy deck");
}

/* position_change_only_places_energy_test.rs :: leaving_stage_does_not_place_
 * waited_energy — Q126's negative: a stage->waitroom ZONE change is not an
 * area move. */
static void test_chisato_leaving_stage_does_not_place_energy(void)
{
    static TestGame game;
    test_game_new(&game);

    int chisato = test_id(&game, CHISATO);
    clear_stage(&game, 0);
    game.state.p[0].stage[0] = chisato;
    fill_energy_deck(&game, 10);

    int zone_before = game.state.p[0].energy.n;
    int deck_before = game.state.p[0].energy_deck.n;

    /* Remove her from the stage, then push the stage->waitroom movement event. */
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    rb_waitroom_add(&game.state.p[0], chisato);
    rb_record_card_movement(&game.state, chisato,
                            RB_ZONEID_STAGE, RB_ZONEID_DISCARD, 0, 0);

    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].energy.n, zone_before,
             "Q126: a stage-to-waitroom zone change must not place energy");
    CHECK_EQ(game.state.p[0].energy_deck.n, deck_before,
             "Q126: and must not consume an energy card");
}

/* area_move_energy_placement_resolution_order_test.rs :: chisato_auto_first_
 * no_energy — a debut that is NOT hers never arms her 自動. */
static void test_chisato_no_energy_when_another_card_debuts(void)
{
    static TestGame game;
    test_game_new(&game);

    int chisato = test_id(&game, CHISATO);
    int shiki    = test_new_id(&game, SHIKI_LS);
    int filler   = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(shiki, SHIKI_LS),
          "the debuter is the PL!SP-bp4-008-R＋ (唐 可可) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = chisato;
    fill_decks(&game, 20);
    fill_energy_deck(&game, 10);
    test_add_to_hand(&game, shiki);
    test_give_energy(&game, 20);

    int zone_before = game.state.p[0].energy.n;
    test_play_to_stage(&game, shiki, 2);
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].energy.n, zone_before,
             "千砂都 never moved, so her 自動 must not place energy");
}

/* area_move_energy_placement_resolution_order_test.rs :: third_member_
 * position_change_no_energy_for_chisato — a swap that does NOT involve her. */
static void test_chisato_no_energy_for_a_third_member_swap(void)
{
    static TestGame game;
    test_game_new(&game);

    int chisato = test_id(&game, CHISATO);
    int kinako  = test_new_id(&game, MOVER);
    int filler  = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(kinako, MOVER),
          "the swapper is the PL!SP-bp5-006-R (桜小路きな子) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[1] = chisato;
    game.state.p[0].stage[2] = filler;
    fill_decks(&game, 20);
    fill_energy_deck(&game, 10);
    test_add_to_hand(&game, kinako);
    test_give_energy(&game, 20);

    int zone_before = game.state.p[0].energy.n;
    int played = test_play_to_stage(&game, kinako, 0);
    drain_auto(&game);
    CHECK_EQ(played, 1, "きな子's debut is accepted");
    CHECK(!pending(&game),
          "no prompt is expected right after the debut (the 自動 resolves at activation)");

    test_activate_ability(&game, kinako);
    CHECK(pending(&game), "the position destination choice is raised");
    if (pending(&game)) answer(&game, 2);   /* swap into Right */
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].stage[2], kinako, "きな子 now sits at Right");
    CHECK_EQ(game.state.p[0].stage[0], filler, "and the filler was swapped to Left");
    CHECK_EQ(game.state.p[0].stage[1], chisato, "千砂都 is untouched at Center");
    CHECK_EQ(game.state.p[0].energy.n, zone_before,
             "千砂都 did not move, so no energy may be placed for her");
}

/* area_move_energy_placement_resolution_order_test.rs :: position_change_
 * involving_chisato_grants_energy — the positive of the same pair. */
static void test_chisato_gains_energy_when_she_is_swapped(void)
{
    static TestGame game;
    test_game_new(&game);

    int chisato = test_id(&game, CHISATO);
    int kinako  = test_new_id(&game, MOVER);
    CHECK(rb_card_no_eq(kinako, MOVER), "the swapper is the PL!SP-bp5-006-R print");

    clear_stage(&game, 0);
    fill_decks(&game, 20);
    fill_energy_deck(&game, 10);
    test_add_to_hand(&game, chisato);
    test_add_to_hand(&game, kinako);
    test_give_energy(&game, 20);

    int played_c = test_play_to_stage(&game, chisato, 0);
    int played_k = test_play_to_stage(&game, kinako, 2);
    drain_auto(&game);
    CHECK_EQ(played_c, 1, "千砂都's debut is accepted");
    CHECK_EQ(played_k, 1, "きな子's debut is accepted");
    CHECK_EQ(game.state.p[0].stage[0], chisato, "precondition: 千砂都 at Left");
    CHECK_EQ(game.state.p[0].stage[2], kinako,  "precondition: きな子 at Right");

    int zone_before = game.state.p[0].energy.n;
    test_activate_ability(&game, kinako);
    CHECK(pending(&game), "the position destination choice is raised");
    if (pending(&game)) answer(&game, 0);   /* swap into Left, i.e. 千砂都 */
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].stage[0], kinako,  "the swap really happened at Left");
    CHECK_EQ(game.state.p[0].stage[2], chisato, "千砂都 moved to Right");
    CHECK_EQ(game.state.p[0].energy.n, zone_before + 1,
             "千砂都 moved, so her 自動 places one energy");
}

/* ====================================================================== */
/* 3. MOVEMENT — 唐 可可 PL!SP-sd2-002-SD2 ab#1: 自動 ターン1回 area move   */
/*    -> heart06 until live end.                                          */
/*    self_move_grants_heart06_until_live_end_test.rs (2) +               */
/*    self_move_watch_natural_move_and_turn_limit_edges_test.rs (2)      */
/* ====================================================================== */

static void test_keke_kidou_position_change_grants_heart06(void)
{
    static TestGame game;
    test_game_new(&game);

    int keke   = test_id(&game, KEKE);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(keke, KEKE), "the watcher is the PL!SP-sd2-002-SD2 (唐 可可) print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = keke;
    game.state.p[0].stage[1] = filler;
    test_give_energy(&game, 4);

    test_activate_ability(&game, keke);
    CHECK(pending(&game), "the 起動 raises the position|destination prompt");
    if (pending(&game)) {
        CHECK(pending_is_select_target(&game), "it is a SelectTarget");
        CHECK(strcmp(pending_target(&game), "position|destination") == 0,
              "the target is position|destination");
        answer(&game, 0);   /* absolute area 0 == left */
    }
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].stage[0], keke,   "可可 moved to Left");
    CHECK_EQ(game.state.p[0].stage[1], filler, "the filler moved to Center");
    CHECK_EQ(heart06(&game, keke), 1,
             "the real 起動 position change grants heart06 x1 via ab#1");
}

static void test_keke_opponent_effect_area_move_triggers_heart06(void)
{
    static TestGame game;
    test_game_new(&game);

    int keke = test_id(&game, KEKE);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = keke;

    /* Rust: push_movement_event(keke, "stage","stage", Some(keke), "p2", true) */
    push_area_move(&game, keke, 1, 1);
    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(heart06(&game, keke), 1,
             "「(対戦相手のカードの効果でも発動する。)」 — an opponent-caused move grants heart06");
}

static void test_keke_natural_move_grants_nothing(void)
{
    static TestGame game;
    test_game_new(&game);

    int keke = test_id(&game, KEKE);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = keke;

    /* Rust: push_movement_event(keke, "stage","stage", None, "p1", false) —
       the C shim requires a non-negative card id, so the None causer becomes 0
       (an unspecified causer) while effect_only stays false. */
    rb_record_card_movement(&game.state, keke,
                            RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 0);
    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(heart06(&game, keke), 0,
             "a NATURAL move (effect_only false) must not fire the 自動");
}

static void test_keke_turn1_blocks_a_second_area_move(void)
{
    static TestGame game;
    test_game_new(&game);

    int keke   = test_id(&game, KEKE);
    int filler = test_id(&game, FILLER);

    clear_stage(&game, 0);
    game.state.p[0].stage[1] = keke;
    test_give_energy(&game, 4);

    test_activate_ability(&game, keke);
    if (pending(&game)) answer(&game, 0);
    drain_auto(&game);
    int first = heart06(&game, keke);
    CHECK_EQ(first, 1, "precondition: the first move granted heart06 x1");

    /* Give her a swap partner and drive a SECOND real 起動. */
    game.state.p[0].stage[2] = filler;
    test_give_energy(&game, 4);
    test_activate_ability(&game, keke);
    if (pending(&game)) answer(&game, 0);
    drain_auto(&game);

    CHECK_EQ(heart06(&game, keke), 1,
             "「{{ターン1回}}」 — a second area move in the same turn adds nothing");
}

/* ====================================================================== */
/* 4. MOVEMENT — 澁谷かのん / 鬼塚冬毬: the ターン1回 cap and the           */
/*    「ライブ終了時まで」 expiry.                                          */
/*    area_move_grant_turn_limit_and_live_end_expiry_test.rs (4)          */
/* ====================================================================== */

static void kanon_or_fuyumari_turn1_blocks_second_move(const char *card_no,
                                                      const char *heart_color)
{
    static TestGame game;
    test_game_new(&game);

    int card = test_id(&game, card_no);
    CHECK(rb_card_no_eq(card, card_no), "the watcher fixture is the named print");
    fill_decks(&game, 40);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = card;

    for (int i = 0; i < 3; i++) {
        push_area_move(&game, card, 0, 1);
        tas(&game, 0);
        drain_auto(&game);
    }
    CHECK_EQ(heart_mod(&game, card, heart_color), 1,
             "three area moves in one turn grant the heart exactly once "
             "(「{{ターン1回}}」 is per TURN, not per move)");
}

static void test_kanon_turn1_blocks_a_second_area_move(void)
{
    kanon_or_fuyumari_turn1_blocks_second_move(KANON, "heart02");
}

static void test_fuyumari_turn1_blocks_a_second_area_move(void)
{
    kanon_or_fuyumari_turn1_blocks_second_move(FUYUMARI, "heart03");
}

/* 「ライブ終了時まで」 — the grant must not outlive the live. The phase walk
 * crosses the victory determination, which is what runs the expiry. */
static void kanon_or_fuyumari_grant_expires_at_live_end(const char *card_no,
                                                        const char *heart_color)
{
    static TestGame game;
    test_game_new(&game);

    int card = test_id(&game, card_no);
    int live = test_id(&game, GENERIC_LIVE);
    fill_decks(&game, 40);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = card;
    test_add_to_hand(&game, live);
    test_give_energy(&game, 10);

    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, live);

    push_area_move(&game, card, 0, 1);
    tas(&game, 0);
    drain_auto(&game);
    CHECK_EQ(heart_mod(&game, card, heart_color), 1,
             "precondition: the grant is live BEFORE the rollover, so the "
             "reading below is about expiry and not an ability that never fired");

    int guard = 0;
    while (game.state.phase != RB_PHASE_VICTORY && guard++ < 20) {
        test_pass(&game);
        drain_auto(&game);
        if (pending(&game)) decline_all(&game);
        if (game.state.phase == RB_PHASE_VICTORY) break;
    }
    CHECK_EQ(game.state.phase, RB_PHASE_VICTORY,
             "the walk reached the victory determination");
    /* One more step runs execute_live_victory_determination / the expiry. */
    test_pass(&game);
    drain_auto(&game);
    decline_all(&game);
    rb_recalc_constants(&game.state);

    CHECK_EQ(heart_mod(&game, card, heart_color), 0,
             "「ライブ終了時まで」 — the grant does not survive the live");
}

static void test_kanon_heart02_does_not_survive_the_live(void)
{
    kanon_or_fuyumari_grant_expires_at_live_end(KANON, "heart02");
}

static void test_fuyumari_heart03_does_not_survive_the_live(void)
{
    kanon_or_fuyumari_grant_expires_at_live_end(FUYUMARI, "heart03");
}

/* ====================================================================== */
/* 5. MOVEMENT — 桜小路きな子 PL!SP-pb1-006-R ab#0: 自動 登場か、エリアを移動 */
/*    するたび、ライブ終了時まで、ブレードブレード.                           */
/*    debut_or_self_area_move_gain_two_blades_q94_q171_test.rs (3)        */
/* ====================================================================== */

static void test_kinako_q94_debut_grants_two_blades(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO_Q94);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(kinako, KINAKO_Q94),
          "the watcher is the PL!SP-pb1-006-R (桜小路きな子) print — NOT the "
          "PL!SP-bp5-006-R / PL!SP-bp2-006-R transpositions of the same character");
    CHECK_EQ(test_get_cost_modifier(&game, kinako), 0, "precondition: no cost modifier yet");

    clear_stage(&game, 0);
    test_add_to_hand(&game, kinako);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 9);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;

    int played = test_play_to_stage(&game, kinako, 0);
    drain_auto(&game);
    CHECK_EQ(played, 1, "きな子's debut is accepted");
    CHECK_EQ(blade_mod(&game, kinako), 2, "Q94: the 登場 leg grants blade x2");
}

static void test_kinako_q94_debut_blades_expire_at_live_victory(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO_Q94);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(kinako, KINAKO_Q94), "the watcher is the PL!SP-pb1-006-R print");

    fill_decks(&game, 20);
    clear_stage(&game, 0);
    test_add_to_hand(&game, kinako);
    test_give_energy(&game, 9);
    test_play_to_stage(&game, kinako, 0);
    drain_auto(&game);
    CHECK_EQ(blade_mod(&game, kinako), 2, "precondition: the debut granted blade x2");

    int guard = 0;
    while (game.state.phase != RB_PHASE_VICTORY && guard++ < 20) {
        test_pass(&game);
        drain_auto(&game);
        if (pending(&game)) decline_all(&game);
        if (game.state.phase == RB_PHASE_VICTORY) break;
    }
    CHECK_EQ(game.state.phase, RB_PHASE_VICTORY,
             "Q171: the walk reached the victory determination");
    CHECK_EQ(blade_mod(&game, kinako), 2,
             "the grant survives UNTIL LiveVictoryDetermination ends");

    test_pass(&game);
    drain_auto(&game);
    decline_all(&game);
    rb_recalc_constants(&game.state);
    CHECK_EQ(game.state.p[0].stage[0], kinako, "she is still on the stage");
    CHECK_EQ(blade_mod(&game, kinako), 0,
             "Q171: 「ライブ終了時まで」 — the grant is gone after the rollover");
}

static void test_kinako_q94_debut_then_swap_stacks_four_blades(void)
{
    static TestGame game;
    test_game_new(&game);

    int watcher = test_id(&game, KINAKO_Q94);
    int swapper = test_new_id(&game, MOVER);
    CHECK(rb_card_no_eq(watcher, KINAKO_Q94), "the watcher is the PL!SP-pb1-006-R print");
    CHECK(rb_card_no_eq(swapper, MOVER),      "the swapper is the PL!SP-bp5-006-R print");
    CHECK(watcher != swapper, "the two きな子 printings are two separate instances");

    clear_stage(&game, 0);
    fill_decks(&game, 20);
    test_add_to_hand(&game, watcher);
    test_add_to_hand(&game, swapper);
    test_give_energy(&game, 40);

    test_play_to_stage(&game, watcher, 0);
    drain_auto(&game);
    CHECK_EQ(blade_mod(&game, watcher), 2, "the debut leg: +2");

    test_play_to_stage(&game, swapper, 2);
    drain_auto(&game);
    CHECK_EQ(blade_mod(&game, watcher), 2,
             "the OTHER member's debut must not touch the watcher");

    test_activate_ability(&game, swapper);
    CHECK(pending(&game), "the swap raises a destination choice");
    if (pending(&game)) answer(&game, 0);   /* absolute area 0 == left */
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].stage[2], watcher, "the watcher moved to Right");
    CHECK_EQ(game.state.p[0].stage[0], swapper, "the swapper took Left");
    CHECK_EQ(blade_mod(&game, watcher), 4,
             "Q94 CORE: debut (+2) + area move (+2) = exactly +4");
}

/* ====================================================================== */
/* 6. MOVEMENT — area_swap_grants_blade_heart_or_recovers_group_live_test   */
/*    (5): three cards whose 自動 all key on an AREA MOVE driven by a real  */
/*    ミリオン Haas 起動 swap.                                             */
/* ====================================================================== */

static void test_area_swap_grants_self_two_blades(void)
{
    static TestGame game;
    test_game_new(&game);

    int target = test_id(&game, CHISATO2);
    CHECK(rb_card_no_eq(target, CHISATO2),
          "the watcher is the PL!SP-bp7-014-N (嵐千砂都) print");
    stage_member_and_swap_area(&game, target, 0);
    CHECK(stage_area_of(&game, 0, target) >= 0,
          "precondition: the swap really moved her out of Left");
    CHECK_EQ(blade_mod(&game, target), 2, "an area move grants this card blade x2");
}

static void test_area_swap_grants_self_heart02(void)
{
    static TestGame game;
    test_game_new(&game);

    int target = test_id(&game, KANON);
    CHECK(rb_card_no_eq(target, KANON), "the watcher is the PL!SP-sd2-012-SD2 print");
    stage_member_and_swap_area(&game, target, 1);
    CHECK(stage_area_of(&game, 0, target) != 1,
          "precondition: the swap really moved her out of Center");
    CHECK_EQ(heart02(&game, target), 1, "an area move grants this card heart02 x1");
}

static void test_area_swap_grants_self_heart03(void)
{
    static TestGame game;
    test_game_new(&game);

    int target = test_id(&game, FUYUMARI);
    CHECK(rb_card_no_eq(target, FUYUMARI), "the watcher is the PL!SP-sd2-022-SD2 print");
    stage_member_and_swap_area(&game, target, 1);
    CHECK(stage_area_of(&game, 0, target) != 1,
          "precondition: the swap really moved her out of Center");
    CHECK_EQ(heart03(&game, target), 1, "an area move grants this card heart03 x1");
}

static void test_area_swap_recovers_low_score_group_live(void)
{
    static TestGame game;
    test_game_new(&game);

    int may   = test_id(&game, MAY);
    int liella = test_new_id(&game, LIELLA_LIVE_SD1);
    CHECK(rb_card_no_eq(may, MAY), "the recoverer is the PL!SP-bp4-007-R (米女メイ) print");

    clear_stage(&game, 0);
    game.state.p[0].discard.n = 0;
    rb_waitroom_add(&game.state.p[0], liella);

    stage_member_and_swap_area(&game, may, 1);

    CHECK(hand_has(&game, liella),
          "米女メイ ab#0 recovers a score≤3 『Liella!』 live card from the waitroom");
}

static void test_shiki_wait_activation_then_area_swap_clears_wait(void)
{
    static TestGame game;
    test_game_new(&game);

    int shiki = test_id(&game, SHIKI_WAIT);
    CHECK(rb_card_no_eq(shiki, SHIKI_WAIT),
          "the card is the PL!SP-bp7-008-R (若菜四季) print");
    clear_stage(&game, 0);
    fill_decks(&game, 20);
    game.state.p[0].stage[1] = shiki;
    test_give_energy(&game, 10);

    test_activate_ability(&game, shiki);
    drain_auto(&game);
    CHECK(strcmp(orientation_of(&game, shiki), "wait") == 0,
          "若菜四季 ab#0 起動 leaves her ウェイト状態");

    mill_three_swap(&game, 1);
    CHECK(strcmp(orientation_of(&game, shiki), "wait") != 0,
          "若菜四季 ab#1 自動 reactivates a ウェイト状態 member that area-moved");
}

/* ====================================================================== */
/* 7. MOVEMENT — 若菜四季's self-watching pair: the GATE, the CYCLE and the */
/*    DRAW.  shiki_wait_gate_arms_only_after_kidou_test.rs (3)            */
/* ====================================================================== */

static void shiki_stage(TestGame *tg, int shiki)
{
    fill_decks(tg, 20);
    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = shiki;
    test_give_energy(tg, 10);
}

/* Swap 若菜四季 out of whichever area she is actually in, and prove the move
 * really happened — a no-op swap would make every ab#1 assertion vacuous. */
static void shiki_move(TestGame *tg, int shiki, const char *why)
{
    int from = stage_area_of(tg, 0, shiki);
    CHECK(from >= 0, "%s: 若菜四季 must be on the stage before the swap", why);
    if (from < 0) return;
    mill_three_swap(tg, from);
    int to = stage_area_of(tg, 0, shiki);
    CHECK(to >= 0 && to != from,
          "%s: the swap must actually move her out of %d (she is now at %d)",
          why, from, to);
}

static void test_shiki_kidou_waits_her_and_draws_exactly_one(void)
{
    static TestGame game;
    test_game_new(&game);

    int shiki = test_id(&game, SHIKI_WAIT);
    CHECK(rb_card_no_eq(shiki, SHIKI_WAIT), "the card is the PL!SP-bp7-008-R print");
    shiki_stage(&game, shiki);

    int hand_before  = test_hand_len(&game);
    int deck_before  = test_deck_len(&game);

    test_activate_ability(&game, shiki);
    drain_auto(&game);

    CHECK(strcmp(orientation_of(&game, shiki), "wait") == 0,
          "ab#0 起動's cost leaves 若菜四季 ウェイト状態");
    CHECK_EQ(test_hand_len(&game), hand_before + 1,
             "ab#0 起動 draws exactly one card");
    CHECK_EQ(test_deck_len(&game), deck_before - 1,
             "the card came from the deck, so the hand delta names the draw");
}

static void test_shiki_auto_does_not_fire_while_she_is_active(void)
{
    static TestGame game;
    test_game_new(&game);

    int shiki = test_id(&game, SHIKI_WAIT);
    shiki_stage(&game, shiki);
    CHECK(strcmp(orientation_of(&game, shiki), "none") == 0,
          "precondition: a freshly staged member carries no orientation, i.e. アクティブ");

    int hand_before = test_hand_len(&game);
    shiki_move(&game, shiki, "the control move");

    CHECK(strcmp(orientation_of(&game, shiki), "none") == 0,
          "GAP 1 (THE GATE): ab#1 must NOT fire for a member that was already "
          "アクティブ — 「ウェイト状態のこのメンバー」 is the gate");
    CHECK_EQ(test_hand_len(&game), hand_before,
             "a silent 自動 draws nothing — ab#0 起動 is the only thing that draws here");
}

static void test_shiki_auto_is_one_shot_per_waited_state(void)
{
    static TestGame game;
    test_game_new(&game);

    int shiki = test_id(&game, SHIKI_WAIT);
    shiki_stage(&game, shiki);

    int hand_before = test_hand_len(&game);
    test_activate_ability(&game, shiki);
    drain_auto(&game);
    CHECK(strcmp(orientation_of(&game, shiki), "wait") == 0, "armed by the 起動");
    CHECK_EQ(test_hand_len(&game), hand_before + 1, "precondition: the arming draw landed once");

    /* First move: the 自動 consumes the waited state. */
    shiki_move(&game, shiki, "the first armed move");
    CHECK(strcmp(orientation_of(&game, shiki), "active") == 0,
          "ab#1 自動 reactivates a ウェイト状態 member that area-moved");
    CHECK_EQ(test_hand_len(&game), hand_before + 1,
             "GAP 3 (THE DRAW): the 自動 does not re-run ab#0's カードを1枚引く");

    /* Second move with the gate shut. Orientation alone cannot separate "the
     * 自動 re-ran and re-set Active" from "it never ran", so the DRAW decides. */
    int hand_after = test_hand_len(&game);
    shiki_move(&game, shiki, "the second, unarmed move");
    CHECK(strcmp(orientation_of(&game, shiki), "active") == 0,
          "with the waited state already consumed, a second move leaves her as the 自動 left her");
    CHECK_EQ(test_hand_len(&game), hand_after,
             "GAP 2 (THE CYCLE): the second, unarmed move must draw nothing");

    /* Re-arm through the 起動 and the cycle closes. */
    test_activate_ability(&game, shiki);
    drain_auto(&game);
    CHECK(strcmp(orientation_of(&game, shiki), "wait") == 0,
          "ab#0 起動 carries no printed use_limit, so a second activation waits her again");
    CHECK_EQ(test_hand_len(&game), hand_after + 1,
             "each 起動 activation draws exactly one card");

    shiki_move(&game, shiki, "the re-armed move");
    CHECK(strcmp(orientation_of(&game, shiki), "active") == 0,
          "the re-armed 自動 must fire on the next self-move — a gate that latches "
          "shut would leave her ウェイト状態 forever");
}

/* ====================================================================== */
/* 7b. ROOT-CAUSE probe: answering a `position|destination` SelectTarget   */
/*     with an absolute area index.  EVERY "real 起動 swap" test above     */
/*     depends on this, so the gap is isolated here once instead of        */
/*     surfacing as a dozen confusing reds.                                */
/* ====================================================================== */

static void test_position_destination_answer_performs_the_swap(void)
{
    static TestGame game;
    test_game_new(&game);

    int keke   = test_id(&game, KEKE);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(keke, KEKE), "the mover is the PL!SP-sd2-002-SD2 print");

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = keke;
    game.state.p[0].stage[1] = filler;
    test_give_energy(&game, 4);

    test_activate_ability(&game, keke);
    CHECK(pending(&game), "the 起動 raises a position|destination prompt");
    if (pending(&game)) {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        CHECK(c && strcmp(c->target, "position|destination") == 0,
              "the pending choice IS position|destination");
        answer(&game, 0);   /* absolute area 0 == left */
    }
    drain_auto(&game);

    CHECK_EQ(game.state.p[0].stage[0], keke,
             "answering position|destination with area 0 must move her to Left "
             "(rb_resume_position_change, state.c:1489-1504, reads selected_idx "
             "as an absolute area)");
    CHECK_EQ(game.state.p[0].stage[1], filler,
             "and the occupant of Left must be swapped to Center");
}

/* ====================================================================== */
/* 8. MOVEMENT — Aspire PL!SP-sd2-025-P: a TIMING CONDITION on the blade   */
/*    grant, not a 自動.  6 tests from                                    */
/*    area_move_timing_and_movement_state_conditions_test.rs PATH 1.       */
/* ====================================================================== */

static void aspire_scenario(TestGame *tg, int in_l1, int in_c, int in_c_is_liella,
                            int in_p2l, int moved_a, int moved_b, int moved_p2, int *out)
{
    int aspire  = test_id(tg, ASPIRE);
    int liella  = test_new_id(tg, LIELLA_A);
    int other   = test_new_id(tg, LIELLA_B);
    int nonlia  = test_new_id(tg, FILLER);
    int centre  = test_new_id(tg, FILLER);
    CHECK(rb_card_no_eq(aspire, ASPIRE), "the live card is the PL!SP-sd2-025-P print");

    fill_decks(tg, 20);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[1] = in_l1 ? liella : RB_EMPTY_SLOT;
    tg->state.p[0].stage[2] = in_c  ? (in_c_is_liella ? other : centre) : RB_EMPTY_SLOT;
    tg->state.p[1].stage[0] = in_p2l ? nonlia : RB_EMPTY_SLOT;

    /* Rust `state.cards_moved_this_turn` + `position_change_occurred_this_turn` */
    tg->state.n_recently_moved = 0;
    if (moved_a)   tg->state.recently_moved[tg->state.n_recently_moved++] = liella;
    if (moved_b)   tg->state.recently_moved[tg->state.n_recently_moved++] =
                       (in_c_is_liella ? other : centre);
    if (moved_p2)  tg->state.recently_moved[tg->state.n_recently_moved++] = nonlia;
    tg->state.position_change_occurred_this_turn = 1;

    set_live_at_set_phase(tg, ASPIRE);
    test_advance_to_phase(tg, RB_PHASE_PERFORMANCE);
    drain_auto(tg);

    *out = blade_mod(tg, liella) * 1000 + blade_mod(tg, other) * 100
         + blade_mod(tg, nonlia) * 10 + blade_mod(tg, centre);
}

static void test_aspire_moved_liella_gains_blade_only(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    /* left = moved 『Liella!』, centre = unmoved 『Liella!』, p2 left = moved non-『Liella!』 */
    aspire_scenario(&game, 1, 1, 1, 1, 1, 0, 1, &v);
    CHECK_EQ(v / 1000, 1, "a moved 『Liella!』 member gains blade");
    CHECK_EQ((v / 100) % 10, 0, "an UNMOVED 『Liella!』 member gains nothing");
    CHECK_EQ(v % 10, 0, "a moved NON-『Liella!』 on the OPPONENT's stage gains nothing");
}

static void test_aspire_only_non_liella_moved_no_blade(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    aspire_scenario(&game, 0, 0, 0, 0, 0, 0, 1, &v);
    CHECK_EQ(v % 10, 0, "only a non-『Liella!』 member moved -> 0 blade");
}

static void test_aspire_liella_and_non_liella_moved_only_liella_gains(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    /* left = moved 『Liella!』, centre = moved NON-『Liella!』 (both on MY stage) */
    aspire_scenario(&game, 1, 1, 0, 0, 1, 1, 0, &v);
    CHECK_EQ(v / 1000, 1, "the moved 『Liella!』 member gains blade");
    CHECK_EQ(v % 10, 0, "the moved NON-『Liella!』 member on MY stage gains nothing");
}

static void test_aspire_no_movement_no_blade(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    aspire_scenario(&game, 1, 0, 0, 0, 0, 0, 0, &v);
    CHECK_EQ(v / 1000, 0, "「このターン中にエリアを移動した」 is false -> no blade");
}

/* ====================================================================== */
/* 9. MOVEMENT — 鬼塚冬毬 PL!SP-bp4-011-R＋: 自動 このメンバーが登場か、     */
/*    エリアを移動したとき、相手のステージのブレード3以下のメンバー1人を   */
/*    ウェイトに.  6 tests (PATH 3).                                      */
/* ====================================================================== */

static int has_all_blade(TestGame *tg, int cid)
{
    Card c;
    int r = 0;
    if (rb_card_get_card_by_id(cid, &c)) {
        r = rb_card_has_blade_heart(&c);
        rb_free_card(&c);
    }
    return r;
}

static void fuyumari_appear_scenario(TestGame *tg, int use_area_move, int *out_wait)
{
    int fuyu   = test_id(tg, FUYUMARI_FMT);
    int target = test_new_id(tg, FILLER);
    int other  = test_new_id(tg, FILLER2);
    CHECK(rb_card_no_eq(fuyu, FUYUMARI_FMT),
          "the 自動 host is the PL!SP-bp4-011-R＋ (鬼塚冬毬) print");
    CHECK(has_all_blade(tg, target), "precondition: the low-blade target has a blade heart");

    fill_decks(tg, 20);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[0] = fuyu;
    tg->state.p[1].stage[0] = target;
    tg->state.p[1].stage[1] = other;

    rb_record_card_appearance(&tg->state, fuyu, RB_ZONEID_HAND);
    tg->state.n_recently_moved = 0;
    tg->state.recently_moved[tg->state.n_recently_moved++] = fuyu;
    if (use_area_move) {
        push_area_move(tg, fuyu, 0, 1);
        tg->state.position_change_occurred_this_turn = 1;
    }

    tas(tg, 0);
    drain_auto(tg);
    *out_wait = (strcmp(orientation_of(tg, target), "wait") == 0) * 100
              + (strcmp(orientation_of(tg, other), "wait") == 0);
}

static void test_fuyumari_appear_triggers_opponent_wait(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    fuyumari_appear_scenario(&game, 0, &v);
    CHECK_EQ(v / 100, 1,
             "the 登場 leg waits exactly one opponent member whose blade is ≤ 3");
    CHECK_EQ(v % 100, 0, "and only ONE member is waited (count=1)");
}

static void test_fuyumari_area_move_triggers_opponent_wait(void)
{
    static TestGame game;
    test_game_new(&game);
    int v = 0;
    fuyumari_appear_scenario(&game, 1, &v);
    CHECK_EQ(v / 100, 1, "the area-move leg waits the same single opponent member");
    CHECK_EQ(v % 100, 0, "and only ONE member is waited");
}

static void test_fuyumari_blade_limit_excludes_high_blade(void)
{
    static TestGame game;
    test_game_new(&game);

    int fuyu  = test_id(&game, FUYUMARI_FMT);
    int high  = test_new_id(&game, BLADE3);
    fill_decks(&game, 20);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = fuyu;
    game.state.p[1].stage[0] = high;
    CHECK(rb_card_no_eq(high, BLADE3),
          "the high-blade fixture is the PL!-sd1-009-SD (blade 5 > 3) print");

    rb_record_card_appearance(&game.state, fuyu, RB_ZONEID_HAND);
    game.state.n_recently_moved = 0;
    game.state.recently_moved[game.state.n_recently_moved++] = fuyu;

    tas(&game, 0);
    drain_auto(&game);

    CHECK(strcmp(orientation_of(&game, high), "wait") != 0,
          "a member whose blade exceeds 3 is excluded from the candidate set");
}

static void test_fuyumari_already_wait_excluded_from_candidates(void)
{
    static TestGame game;
    test_game_new(&game);

    int fuyu   = test_id(&game, FUYUMARI_FMT);
    int waited = test_new_id(&game, FILLER);
    int other  = test_new_id(&game, FILLER2);
    fill_decks(&game, 20);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = fuyu;
    game.state.p[1].stage[0] = waited;
    game.state.p[1].stage[1] = other;

    rb_mods_set_orientation(&game.state.mods, waited, "wait");
    rb_record_card_appearance(&game.state, fuyu, RB_ZONEID_HAND);
    game.state.n_recently_moved = 0;
    game.state.recently_moved[game.state.n_recently_moved++] = fuyu;

    tas(&game, 0);
    drain_auto(&game);

    CHECK(strcmp(orientation_of(&game, other), "wait") == 0,
          "an already-ウェイト member is filtered out, so the other candidate is taken");
    CHECK(strcmp(orientation_of(&game, waited), "wait") == 0,
          "the already-waited member is left unchanged");
}

/* ====================================================================== */
/* 10. MOVEMENT — ability_chain_combo: one ability's resolution FEEDS the  */
/*     other.  3 of the 5 Rust tests (pb2006 x2, special color x2).       */
/* ====================================================================== */

static void test_pb2006_jidou_placement_feeds_constant_cost_up(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO_TUCK);
    int fodder = test_new_id(&game, LIELLA_FODDER);
    CHECK(rb_card_no_eq(kinako, KINAKO_TUCK),
          "the subject is the PL!SP-pb2-006-R (桜小路きな子) print");
    CHECK(rb_card_no_eq(fodder, LIELLA_FODDER),
          "the fodder is the PL!SP-bp1-014-N (嵐 千砂都) print — a DIFFERENT "
          "member, not a second copy of the subject");

    fill_decks(&game, 20);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = kinako;
    game.state.p[0].discard.n = 0;
    rb_waitroom_add(&game.state.p[0], fodder);

    rb_recalc_constants(&game.state);
    CHECK_EQ(test_get_cost_modifier(&game, kinako), 0,
             "control: nothing under きな子 -> her 常時 grants no cost");

    manually_move(&game, kinako, 1, 0);
    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(under_count(&game, 0, 0), 1,
             "the 自動 places 1 『Liella!』 member from the waitroom under her");
    CHECK(under_has(&game, 0, 0, fodder), "and it is the card from the waitroom");
    CHECK(!waitroom_has(&game, fodder), "the placed card left the waitroom");
    CHECK_EQ(test_get_cost_modifier(&game, kinako), 1,
             "CHAIN PAYOFF: 常時 (+1 cost per 『Liella!』 under) now shows +1");
}

static void test_pb2006_jidou_once_per_turn_no_second_placement(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO_TUCK);
    int a = test_new_id(&game, LIELLA_FODDER);
    int b = test_new_id(&game, LIELLA_FODDER);
    CHECK(a != b, "the two fodder copies are distinct pool slots");

    fill_decks(&game, 20);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = kinako;
    game.state.p[0].discard.n = 0;
    rb_waitroom_add(&game.state.p[0], a);
    rb_waitroom_add(&game.state.p[0], b);

    manually_move(&game, kinako, 1, 0);
    tas(&game, 0);
    drain_auto(&game);

    manually_move(&game, kinako, 0, 2);
    tas(&game, 0);
    drain_auto(&game);

    CHECK_EQ(under_count(&game, 0, 2), 0,
             "「{{ターン1回}}」: a second area move in the same turn must NOT "
             "place a second card");
}

static void special_color_scenario(TestGame *tg, int move_back, int *out_blade,
                                   int *out_score, int *out_bonus)
{
    int special = test_id(tg, SPECIAL);
    int liella  = test_id(tg, SPECIAL_LIELLA);
    int filler  = test_id(tg, FILLER);
    CHECK(rb_card_no_eq(special, SPECIAL), "the live card is the PL!SP-bp4-025-L print");
    CHECK(rb_card_no_eq(liella, SPECIAL_LIELLA),
          "the member is the PL!SP-bp1-001-R (blade 3) print");

    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = liella;
    fill_decks(tg, 10);
    test_add_to_hand(tg, special);

    test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    test_set_live_card(tg, 0, special);
    test_advance_to_phase(tg, RB_PHASE_PERFORMANCE);
    drain_auto(tg);
    *out_blade = blade_mod(tg, liella);

    /* Rust: leave center and (optionally) return to it. */
    manually_move(tg, liella, 1, 0);
    if (move_back) manually_move(tg, liella, 0, 1);

    /* The ライブ成功時 must be raised while the host is still on the board. The
     * live card has already left the live zone by this point, so it is put back
     * there — the Rust helper addresses the ability by id, the C shim
     * (`rb_trigger_live_success`) scans the zones. */
    test_add_to_live(tg, special);
    tg->state.live_success[0] = 1;
    rb_trigger_live_success(&tg->state, 0);
    drain_auto(tg);

    *out_score = score_mod(tg, special);
    *out_bonus = tg->state.mods.p1_constant_total_score_bonus;
    *out_blade = blade_mod(tg, liella);
}

static void test_special_color_set_blades_and_score_twin_in_one_live(void)
{
    static TestGame game;
    test_game_new(&game);
    int blade = 0, score = 0, bonus = 0;
    special_color_scenario(&game, 1, &blade, &score, &bonus);
    CHECK_EQ(blade, 3, "ab#0: the centre 『Liella!』 member has her blades set to 3");
    CHECK_EQ(score, 1, "ab#1: the centre 『Liella!』 member moved this turn -> +1 score");
    CHECK_EQ(blade, 3, "ab#0's blade set persists alongside ab#1's score");
}

static void test_special_color_no_score_when_moved_liella_left_center(void)
{
    static TestGame game;
    test_game_new(&game);
    int blade = 0, score = 0, bonus = 0;
    special_color_scenario(&game, 0, &blade, &score, &bonus);
    CHECK_EQ(score, 0,
             "ab#1: the moved 『Liella!』 is no longer in centre -> no +1 score "
             "(the printed condition requires センターエリアにいる)");
    (void)blade; (void)bonus;
}

/* ====================================================================== */
/* 11. MOVEMENT — under_member_placement: discard 『Liella!』 member under   */
/*     self on ライブ成功時 and on her own area move.  (2)                  */
/* ====================================================================== */

static void test_kinako_tucks_group_member_under_self_on_area_move(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO_TUCK);
    int swapper = test_new_id(&game, MOVER);
    int liella  = test_new_id(&game, LIELLA_SD1_020);
    CHECK(rb_card_no_eq(kinako, KINAKO_TUCK),   "the subject is the PL!SP-pb2-006-R print");
    CHECK(rb_card_no_eq(swapper, MOVER),        "the swapper is the PL!SP-bp5-006-R print");
    CHECK(kinako != swapper, "the two きな子 printings are two separate instances");
    CHECK(rb_card_no_eq(liella, LIELLA_SD1_020),
          "the tucked card is the PL!SP-sd1-020-SD print");

    clear_stage(&game, 0);
    fill_decks(&game, 20);
    test_add_to_hand(&game, kinako);
    test_add_to_hand(&game, swapper);
    game.state.p[0].discard.n = 0;
    rb_waitroom_add(&game.state.p[0], liella);
    test_give_energy(&game, 20);

    CHECK_EQ(test_play_to_stage(&game, kinako, 0), 1, "きな子 (subject) debuts at Left");
    CHECK_EQ(test_play_to_stage(&game, swapper, 1), 1, "きな子 (swapper) debuts at Center");
    drain_auto(&game);

    /* The 起動 position change is the movement that arms the subject's 自動. */
    test_activate_ability(&game, swapper);
    drain_auto(&game);
    if (pending(&game)) answer(&game, 0);
    drain_auto(&game);

    CHECK(!waitroom_has(&game, liella),
          "the 『Liella!』 card must have left the waitroom via the 自動");
    int idx = stage_area_of(&game, 0, kinako);
    CHECK(idx >= 0, "precondition: the subject is still on the stage");
    if (idx >= 0) {
        CHECK(under_has(&game, 0, idx, liella),
              "the 『Liella!』 card must be tucked under the subject after the move");
    }
}

/* ====================================================================== */
/* 12. MOVEMENT — PL!S-bp5-023-L ab#0: ライブ開始時, 『Aqours』 +          */
/*     『SaintSnow』 members totalling cost ≥ 20 -> put up to 4 matching    */
/*     waitroom LIVE cards on the deck top.  7 of the 10 Rust tests.       */
/* ====================================================================== */

typedef struct {
    const char *aq_cost_card, *ss_cost_card;
    const char *live1, *live2, *live3, *live4, *other_live;
} AwakenFixture;

static void awaken_setup(TestGame *tg, AwakenFixture *fx, int n_waitroom_lives)
{
    fill_decks(tg, 30);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    test_add_to_hand(tg, test_id(tg, fx->aq_cost_card));
    test_add_to_hand(tg, test_id(tg, fx->ss_cost_card));
    test_give_energy(tg, 30);
    test_play_to_stage(tg, test_id(tg, fx->aq_cost_card), 0);
    test_play_to_stage(tg, test_id(tg, fx->ss_cost_card), 1);
    drain_auto(tg);
    test_recalc(tg);
    if (n_waitroom_lives >= 1) test_add_to_discard(tg, test_new_id(tg, fx->live1));
    if (n_waitroom_lives >= 2) test_add_to_discard(tg, test_new_id(tg, fx->live2));
    if (n_waitroom_lives >= 3) test_add_to_discard(tg, test_new_id(tg, fx->live3));
    if (n_waitroom_lives >= 4) test_add_to_discard(tg, test_new_id(tg, fx->live4));
}

static void test_awaken_happy_path_prompts_for_four_cards(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, SS_222, AQ_LIVE, AQ_LIVE, AQ_LIVE, SS_LIVE };
    awaken_setup(&game, &fx, 4);
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    CHECK(pending(&game), "『Aqours』 + 『SaintSnow』 with total cost ≥ 20 -> prompt");
    if (pending(&game)) {
        CHECK(pending_is_select_card(&game), "the prompt is a SelectCard");
        CHECK_EQ(pending_count(&game), 4, "「4枚まで好きな順番で」 -> count 4");
    }
}

static void test_awaken_no_saintsnow_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, AQ_CHIKA, AQ_LIVE, AQ_LIVE, AQ_LIVE, AQ_LIVE };
    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    test_add_to_hand(&game, test_id(&game, AQ_CHIKA));
    test_give_energy(&game, 20);
    test_play_to_stage(&game, test_id(&game, AQ_CHIKA), 1);
    drain_auto(&game);
    test_recalc(&game);
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    CHECK(!pending(&game), "only 『Aqours』 on stage -> no prompt");
}

static void test_awaken_cost_below_20_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_YO, SS_111, AQ_LIVE, AQ_LIVE, AQ_LIVE, AQ_LIVE };
    CHECK(rb_card_no_eq(test_id(&game, AQ_YO), AQ_YO),
          "the low-cost 『Aqours』 fixture is the PL!S-sd1-005-SD print");
    CHECK(rb_card_no_eq(test_id(&game, SS_111), SS_111),
          "the low-cost 『SaintSnow』 fixture is the PL!S-bp5-111-R print");
    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    test_add_to_hand(&game, test_id(&game, AQ_YO));
    test_add_to_hand(&game, test_id(&game, SS_111));
    test_give_energy(&game, 20);
    test_play_to_stage(&game, test_id(&game, AQ_YO), 0);
    test_play_to_stage(&game, test_id(&game, SS_111), 1);
    drain_auto(&game);
    test_recalc(&game);
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    CHECK(!pending(&game), "both groups present but total cost < 20 -> no prompt");
}

static void test_awaken_cost_at_least_20_fires(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, SS_111, AQ_LIVE, AQ_LIVE, AQ_LIVE, AQ_LIVE };
    awaken_setup(&game, &fx, 1);
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    CHECK(pending(&game), "17 + 4 = 21 ≥ 20 -> the ability fires");
}

static void test_awaken_can_skip(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, SS_222, AQ_LIVE, AQ_LIVE, AQ_LIVE, AQ_LIVE };
    awaken_setup(&game, &fx, 1);
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    CHECK(pending(&game), "the ability offers the optional placement");
    decline_all(&game);
    CHECK_EQ(game.state.p[0].discard.n, 1,
             "「…置いてもよい」 — declining leaves the card in the waitroom");
}

static void test_awaken_no_matching_live_in_waitroom_skips(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, SS_222, NIJI_LIVE, NIJI_LIVE, NIJI_LIVE, NIJI_LIVE };
    CHECK(rb_card_no_eq(test_id(&game, NIJI_LIVE), NIJI_LIVE),
          "the non-matching live is the PL!N-bp1-028-L (虹ヶ咲) print");
    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    test_add_to_hand(&game, test_id(&game, AQ_CHIKA));
    test_add_to_hand(&game, test_id(&game, SS_222));
    test_give_energy(&game, 30);
    test_play_to_stage(&game, test_id(&game, AQ_CHIKA), 0);
    test_play_to_stage(&game, test_id(&game, SS_222), 1);
    drain_auto(&game);
    test_recalc(&game);
    test_add_to_discard(&game, test_new_id(&game, NIJI_LIVE));
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    CHECK(!pending(&game), "no 『Aqours』/『SaintSnow』 live in the waitroom -> no prompt");
}

static void test_awaken_only_matching_lives_are_filtered(void)
{
    static TestGame game;
    test_game_new(&game);
    AwakenFixture fx = { AQ_CHIKA, SS_222, AQ_LIVE, SS_LIVE, AQ_LIVE, SS_LIVE };
    awaken_setup(&game, &fx, 3);
    game.state.p[0].discard.n = 0;
    int niji = test_new_id(&game, NIJI_LIVE);
    rb_waitroom_add(&game.state.p[0], niji);   /* index 3, must be excluded */
    set_live_at_set_phase(&game, AWAKEN);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    CHECK(pending(&game), "the prompt is raised");
    if (pending(&game)) {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        CHECK(c && strcmp(c->zone, "discard") == 0,
              "the waitroom is named \"discard\" in C");
        CHECK_EQ(c->n_filtered_indices, 3,
                 "only the 3 『Aqours』/『SaintSnow』 live cards are filterable");
        int contains_niji = 0;
        for (int i = 0; i < c->n_filtered_indices; i++)
            if (c->filtered_indices[i] == 3) contains_niji = 1;
        CHECK(!contains_niji, "the 虹ヶ咲 live card at waitroom index 3 is NOT selectable");
    }
}

/* ====================================================================== */
/* 13. YELL — no_blade_heart_reveal_gain: 自動 ターン1回 「エールにより公開 */
/*     された自分のカードの中にブレードハートを持つカードがないとき、       */
/*     ライブ終了時まで、heart を得る」.  鬼塚夏美 (heart02) is already in   */
/*     test_parity_jidou_extra.c section J; 住藤(heart06) / ウィーン        */
/*     (heart03) are the two siblings.  (6 + 5 tests)                      */
//* ====================================================================== */

static void sumire_wien_scenario(TestGame *tg, int with_sumire, int with_wien,
                                 const int *revealed, int n, int *out_h6, int *out_h3)
{
    int bladed = test_new_id(tg, BLADE_HEART);
    int sumire = test_id(tg, SUMIRE);
    int wien   = test_id(tg, WIEN);
    CHECK(rb_card_no_eq(sumire, SUMIRE), "the heart06 watcher is the PL!SP-bp2-015-N print");
    CHECK(rb_card_no_eq(wien, WIEN),     "the heart03 watcher is the PL!SP-bp2-021-N print");

    clear_stage(tg, 0);
    tg->state.p[0].stage[0] = bladed;
    tg->state.p[0].stage[1] = with_sumire ? sumire : RB_EMPTY_SLOT;
    tg->state.p[0].stage[2] = with_wien   ? wien   : RB_EMPTY_SLOT;
    trigger_yell(tg, 0, revealed, n);
    drain_auto(tg);
    *out_h6 = heart06(tg, sumire);
    *out_h3 = heart03(tg, wien);
}

static void test_yell_no_blade_heart_grants_heart06_and_heart03(void)
{
    static TestGame game;
    test_game_new(&game);
    int r1 = test_new_id(&game, RIKO_NO_BLADE);
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, &r1, 1, &h6, &h3);
    CHECK_EQ(h6, 1, "「ブレードハートを持つカードがないとき」 -> 住藤 gains heart06");
    CHECK_EQ(h3, 1, "the SAME condition, ウィーン's colour: heart03");
}

static void test_yell_with_blade_heart_blocks_both_watchers(void)
{
    static TestGame game;
    test_game_new(&game);
    int bladed = test_new_id(&game, BLADE_PB1);
    CHECK(has_all_blade(&game, bladed),
          "precondition: PL!-pb1-014-R really carries a blade heart");
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, &bladed, 1, &h6, &h3);
    CHECK_EQ(h6, 0, "a yelLED blade heart makes the negation false -> no heart06");
    CHECK_EQ(h3, 0, "…and no heart03 either");
}

static void test_yell_all_blade_blocks_both_watchers(void)
{
    static TestGame game;
    test_game_new(&game);
    int allb = test_new_id(&game, ALL_BLADE);
    CHECK(has_all_blade(&game, allb),
          "precondition: PL!HS-PR-010-PR carries ALL blade heart "
          "(has_blade_heart includes ALL)");
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, &allb, 1, &h6, &h3);
    CHECK_EQ(h6, 0, "ALL blade heart counts as a blade heart -> heart06 blocked");
    CHECK_EQ(h3, 0, "…and heart03 blocked");
}

static void test_yell_mixed_blade_and_no_blade_still_blocks(void)
{
    static TestGame game;
    test_game_new(&game);
    int no_blade = test_new_id(&game, RIKO_NO_BLADE);
    int bladed   = test_new_id(&game, BLADE_PB1);
    int rev[2] = { no_blade, bladed };
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, rev, 2, &h6, &h3);
    CHECK_EQ(h6, 0, "one blade heart among the revealed cards blocks the whole ability");
    CHECK_EQ(h3, 0, "…for both watchers");
}

static void test_yell_two_no_blade_cards_still_grants(void)
{
    static TestGame game;
    test_game_new(&game);
    int a = test_new_id(&game, RIKO_NO_BLADE);
    int b = test_new_id(&game, RIKO_NO_BLADE);
    int rev[2] = { a, b };
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, rev, 2, &h6, &h3);
    CHECK_EQ(h6, 1, "two blade-heartless cards still satisfy 「ないとき」");
    CHECK_EQ(h3, 1, "…for both watchers");
}

static void test_yell_empty_revealed_no_gain(void)
{
    static TestGame game;
    test_game_new(&game);
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, NULL, 0, &h6, &h3);
    CHECK_EQ(h6, 0, "an empty yell (yell_occurred false) must not trigger");
    CHECK_EQ(h3, 0, "…for either watcher");
}

static void test_yell_turn1_blocks_a_second_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    int r1 = test_new_id(&game, RIKO_NO_BLADE);
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, &r1, 1, &h6, &h3);
    CHECK_EQ(h6, 1, "precondition: the first yell granted heart06");
    int r2 = test_new_id(&game, RIKO_NO_BLADE);
    int h6b = 0, h3b = 0;
    sumire_wien_scenario(&game, 1, 1, &r2, 1, &h6b, &h3b);
    CHECK_EQ(h6b, 1, "「{{ターン1回}}」 — a second yell in the same turn adds nothing");
    CHECK_EQ(h3b, 1, "…and the same for the heart03 watcher");
}

static void test_yell_turn_limit_resets_next_turn(void)
{
    /* The engine keys use_limit by turn_number, so a game whose turn_number is
     * already 2 must be able to fire the same 自動. */
    static TestGame game;
    test_game_new(&game);
    game.state.turn = 2;
    int r1 = test_new_id(&game, RIKO_NO_BLADE);
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 1, 1, &r1, 1, &h6, &h3);
    CHECK_EQ(h6, 1, "on turn 2 the yell-triggered 自動 fires for the first time");
    CHECK_EQ(h3, 1, "…for both watchers");
}

static void test_yell_only_one_present_triggers_self_only(void)
{
    static TestGame game;
    test_game_new(&game);
    int r1 = test_new_id(&game, RIKO_NO_BLADE);
    int h6 = 0, h3 = 0;
    sumire_wien_scenario(&game, 0, 1, &r1, 1, &h6, &h3);
    CHECK_EQ(h3, 1, "with only ウィーン on stage, only her heart03 fires");
    CHECK_EQ(h6, 0, "住藤 is absent, so no heart06 anywhere");
}

/* ====================================================================== */
/* 14. YELL — wien_yell_no_blade_heart_gain_heart03_until_live_end_test.rs  */
/*     (3): the honest negatives around a REAL yell.                      */
/* ====================================================================== */

static void wien_real_live_scenario(TestGame *tg, const char *deck_card,
                                    int expect_heart03)
{
    int wien   = test_id(tg, WIEN);
    int bladed = test_new_id(tg, BLADE_HEART);
    int filler = test_id(tg, FILLER);
    CHECK(rb_card_no_eq(wien, WIEN), "the watcher is the PL!SP-bp2-021-N print");

    fill_decks(tg, 10);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[0] = bladed;
    tg->state.p[0].stage[1] = wien;

    /* The deck decides what the yell reveals. */
    int d = test_id(tg, deck_card);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, 0, d);

    set_live_at_set_phase(tg, SUMIRE_LIVE);
    test_advance_to_phase(tg, RB_PHASE_PERFORMANCE);
    drain_auto(tg);

    CHECK_EQ(heart03(tg, wien), expect_heart03,
             expect_heart03
             ? "a real yell revealing no blade heart grants heart03 DURING the live"
             : "a yelLED blade heart makes the negation false -> no heart03");
}

static void test_wien_q112_real_yell_no_blade_heart_grants_heart03(void)
{
    static TestGame game;
    test_game_new(&game);
    wien_real_live_scenario(&game, ENERGY, 1);
}

static void test_wien_q112_real_yell_with_blade_heart_no_gain(void)
{
    static TestGame game;
    test_game_new(&game);
    wien_real_live_scenario(&game, BLADE_HEART, 0);
}

static void test_wien_q113_zero_score_live_yields_no_yell(void)
{
    static TestGame game;
    test_game_new(&game);
    int wien   = test_id(&game, WIEN);
    int bladed = test_new_id(&game, BLADE_HEART);
    fill_decks(&game, 10);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = bladed;
    game.state.p[0].stage[1] = wien;

    /* A 0-score live card: no yell happens at all. */
    set_live_at_set_phase(&game, FILLER);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    CHECK_EQ(game.state.yell_occurred, 0,
             "Q113 premise: a 0-score live card produces NO yell");
    CHECK_EQ(heart03(&game, wien), 0,
             "Q113: no yell -> the yell-triggered 自動 must not fire");
}

/* ====================================================================== */
/* 15. YELL — 黒澤ダイヤ PL!S-bp2-004-R ab#0 and ダイスキだったらダイジョ */
/*     ウブ！PL!S-bp3-020-L ab#0: the ≤2-blade-heart / no-live-card        */
/*     re-yell pair.  (blade_heart_two_or_fewer_..., 6)                    */
/* ====================================================================== */

/* Rust asserts on `state.re_yell_revealed_cards`; C has no such field, so
 * the observable is `re_yell_occurred` (live.c sets it when a re-yell effect
 * fires) together with the re-yell blade harvest. */
static int re_yell_happened(TestGame *tg)
{
    return tg->state.re_yell_occurred != 0;
}

static void daisuki_scenario(TestGame *tg, int stage_members, int blade_on_one,
                             int energy_deck, int *out_pending, int *out_reveal)
{
    int daisuki = test_id(tg, DAISUKI);
    int filler  = test_new_id(tg, FILLER);
    int bladed  = test_new_id(tg, FILLER);
    CHECK(rb_card_no_eq(daisuki, DAISUKI), "the live card is the PL!S-bp3-020-L print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    if (energy_deck) {
        int e = test_id(tg, ENERGY);
        for (int i = 0; i < 60; i++) { test_add_to_deck_pl(tg, 0, e); test_add_to_deck_pl(tg, 1, e); }
    } else {
        int f = test_id(tg, FILLER);
        for (int i = 0; i < 60; i++) { test_add_to_deck_pl(tg, 0, f); test_add_to_deck_pl(tg, 1, f); }
    }
    test_give_energy(tg, 15);

    for (int i = 0; i < stage_members && i < 3; i++)
        tg->state.p[0].stage[i] = (i == 2) ? bladed : test_new_id(tg, FILLER);
    if (blade_on_one) rb_mods_add_blade(&tg->state.mods, bladed, 2);

    set_live_at_set_phase(tg, DAISUKI);
    test_advance_to_phase(tg, RB_PHASE_PERFORMANCE_SECOND);
    drain_auto(tg);
    *out_pending = pending(tg);
    *out_reveal  = tg->state.n_revealed;
}

static void test_daisuki_zero_stage_no_reveal_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 0, 0, 0, &p, &r);
    CHECK_EQ(r, 0, "0 stage members -> 0 revealed cards");
    CHECK_EQ(p, 0, "「1枚以上公開したとき」 — 0 revealed -> no trigger");
}

static void test_daisuki_two_blade_hearts_prompts(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 2, 0, 0, &p, &r);
    CHECK_EQ(r, 2, "2 stage members -> 2 revealed cards");
    CHECK_EQ(p, 1, "「1枚以上公開」 + 「2枚以下」 -> the discard prompt appears");
}

static void test_daisuki_three_blade_hearts_blocks(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 3, 1, 0, &p, &r);
    CHECK_EQ(r, 3, "3 stage members -> 3 revealed cards");
    CHECK_EQ(p, 0, "3 blade hearts > 2 -> the whole ability is blocked");
    CHECK_EQ(re_yell_happened(&game), 0, "and no re-yell");
}

static void test_daisuki_zero_blade_hearts_prompts(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 2, 0, 1, &p, &r);
    CHECK(p, "0 blade hearts ≤ 2 -> the discard prompt appears");
}

static void test_daisuki_accept_discard_then_re_yells(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 2, 0, 0, &p, &r);
    CHECK(p, "the discard prompt appears");
    if (p) {
        int count = pending_count(&game);
        CHECK_EQ(count, 2, "「ブレードハートの数以下のカードなら」 -> count 2");
        int idx[RB_MAX_ZONE];
        for (int i = 0; i < count && i < RB_MAX_ZONE; i++) idx[i] = i;
        test_select_indices(&game, idx, count);
        drain_auto(&game);
    }
    CHECK_EQ(re_yell_happened(&game), 1,
             "accepting the discard discards the revealed cards and re-yells");
}

static void test_daisuki_skip_discard_no_re_yell(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, r = 0;
    daisuki_scenario(&game, 2, 0, 0, &p, &r);
    CHECK(p, "the discard prompt appears");
    decline_all(&game);
    CHECK_EQ(re_yell_happened(&game), 0,
             "declining the optional discard means no re-yell");
}

/* ====================================================================== */
/* 16. YELL — 鬼塚夏美 PL!SP-pb2-020-R: on yell you may discard a            */
/*     『Liella!』 LIVE card from HAND, and if you do, yell 2 more.  (4)    */
/* ====================================================================== */

static void natsumi_yell_setup(TestGame *tg)
{
    int filler = test_new_id(tg, FILLER);
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    int natsumi = test_id(tg, NATSUMI);
    CHECK(rb_card_no_eq(natsumi, NATSUMI),
          "the staged print is PL!SP-pb2-020-R (鬼塚夏美) — NOT the "
          "PL!SP-bp2-020-N that the heart02 test uses");
    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = natsumi;
}

static void natsumi_scenario(TestGame *tg, int n_lives_in_hand, int *out_pending,
                             int *out_live)
{
    natsumi_yell_setup(tg);
    *out_live = -1;
    for (int i = 0; i < n_lives_in_hand; i++) {
        int live = test_new_id(tg, LIELLA_LIVE_SD1);
        CHECK(rb_card_no_eq(live, LIELLA_LIVE_SD1),
              "the offered live is the PL!SP-sd1-023-SD (『Liella!』) print");
        test_add_to_hand(tg, live);
        *out_live = live;
    }
    int r1 = test_new_id(tg, FILLER);
    int r2 = test_new_id(tg, FILLER);
    int rev[2] = { r1, r2 };
    trigger_yell(tg, 0, rev, 2);
    *out_pending = pending(tg);
}

static void test_natsumi_discards_liella_live_for_two_extra_yells(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, live = -1;
    natsumi_scenario(&game, 1, &p, &live);
    CHECK(p, "the optional 『Liella!』 live discard is prompted");
    CHECK(live >= 0 && hand_has(&game, live), "precondition: the live is in hand");
    if (p) {
        answer(&game, 0);
        drain_auto(&game);
    }
    CHECK(live >= 0 && waitroom_has(&game, live),
          "the paid cost really moved the 『Liella!』 live card to the waitroom");
    CHECK(live >= 0 && !hand_has(&game, live), "and it left the hand");
    CHECK_EQ(re_yell_happened(&game), 1,
             "paying the optional discard triggers the additional yells");
}

static void test_natsumi_decline_discard_no_extra_yells(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, live = -1;
    natsumi_scenario(&game, 1, &p, &live);
    CHECK(p, "the optional discard is prompted");
    decline_all(&game);
    CHECK(live >= 0 && hand_has(&game, live), "declined: the live card stays in hand");
    CHECK_EQ(re_yell_happened(&game), 0, "declined: no additional yells");
}

static void test_natsumi_no_liella_live_in_hand_no_prompt(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, live = -1;
    natsumi_scenario(&game, 0, &p, &live);
    CHECK_EQ(p, 0, "no eligible 『Liella!』 live in hand -> auto-skip, no prompt");
    CHECK_EQ(re_yell_happened(&game), 0, "…and no additional yells");
}

static void test_natsumi_use_limit_blocks_a_second_yell(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0, live = -1;
    natsumi_scenario(&game, 2, &p, &live);
    CHECK(p, "the first yell prompts");
    if (p) { answer(&game, 0); drain_auto(&game); }
    int revealed_after = game.state.n_revealed;
    int hand_after = game.state.p[0].hand.n;

    /* Second yell in the same turn. */
    tas(&game, 0);
    drain_auto(&game);
    CHECK_EQ(pending(&game), 0, "「{{ターン1回}}」 — a second yell does not prompt");
    CHECK_EQ(game.state.n_revealed, revealed_after,
             "the blocked firing performs no further yells");
    CHECK_EQ(game.state.p[0].hand.n, hand_after, "the second live is never touched");
}

/* ====================================================================== */
/* 17. YELL — PL!N-bp5-001-R＋: DISTINCT blade-heart TYPES in the reveal.  */
/*     0 -> nothing, 3 -> heart01, 6 -> heart01 + live total +1.  (9)      */
/* ====================================================================== */

static void blade_heart_threshold_scenario(TestGame *tg, int n_types, int add_b_all,
                                           int n_base_heart_only, int n_duplicate_h01,
                                           int *out_h01, int *out_bonus)
{
    int ability_card = test_id(tg, ABILITY_CARD);
    CHECK(rb_card_no_eq(ability_card, ABILITY_CARD),
          "the ability card is the PL!N-bp5-001-R＋ print");

    const char *refs[6] = { B_H01, B_H02, B_H03, B_H04, B_H05, B_H06 };
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[1] = ability_card;

    int revealed[RB_MAX_REVEALED_CARDS];
    int n = 0;
    for (int i = 0; i < n_types && i < 6; i++) revealed[n++] = test_new_id(tg, refs[i]);
    for (int i = 0; i < n_duplicate_h01; i++) revealed[n++] = test_new_id(tg, B_H01);
    for (int i = 0; i < n_base_heart_only; i++) revealed[n++] = test_new_id(tg, BASE_HEART);
    if (add_b_all) revealed[n++] = test_new_id(tg, B_ALL);

    tg->state.n_revealed = 0;
    for (int i = 0; i < n; i++) {
        tg->state.revealed_cards[i] = revealed[i];
        tg->state.n_revealed = i + 1;
        rb_waitroom_add(&tg->state.p[0], revealed[i]);
    }
    tg->state.yell_occurred = n > 0;
    tas(tg, 0);
    drain_auto(tg);

    *out_h01   = heart01(tg, ability_card);
    *out_bonus = tg->state.mods.p1_constant_total_score_bonus;
}

static void test_threshold_zero_blade_heart_types_no_effect(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 0, 0, 1, 0, &h1, &bonus);
    CHECK_EQ(h1, 0, "0 blade-heart types -> no heart01");
    CHECK_EQ(bonus, 0, "0 blade-heart types -> no live-total bonus");
}

static void test_threshold_two_blade_heart_types_no_effect(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 2, 0, 0, 0, &h1, &bonus);
    CHECK_EQ(h1, 0, "2 types is below the threshold of 3 -> no heart01");
    CHECK_EQ(bonus, 0, "2 types is below the threshold of 6 -> no live-total bonus");
}

static void test_threshold_three_blade_heart_types_grants_heart01(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 3, 0, 0, 0, &h1, &bonus);
    CHECK_EQ(h1, 1, "3 distinct blade-heart types -> heart01 x1");
    CHECK_EQ(bonus, 0, "3 types is still below the threshold of 6");
}

static void test_threshold_five_blade_heart_types_heart01_only(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 5, 0, 0, 0, &h1, &bonus);
    CHECK_EQ(h1, 1, "5 distinct types -> heart01 x1");
    CHECK_EQ(bonus, 0, "5 types is still below the threshold of 6");
}

static void test_threshold_six_blade_heart_types_grants_both(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 6, 0, 0, 0, &h1, &bonus);
    CHECK_EQ(h1, 1, "6 distinct types -> heart01 x1");
    CHECK_EQ(bonus, 1, "6 distinct types -> 「常時：ライブの合計スコア＋１する」");
}

static void test_threshold_all_blade_heart_types_with_b_all_grants_both(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 6, 1, 0, 0, &h1, &bonus);
    CHECK_EQ(h1, 1, "6 types + b_all -> heart01 x1");
    CHECK_EQ(bonus, 1, "b_all contributing all 6+ types -> live total +1");
}

static void test_threshold_base_heart_only_does_not_count(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 0, 0, 3, 0, &h1, &bonus);
    CHECK_EQ(h1, 0,
             "cards with base_heart but NO blade_heart must NOT count toward "
             "the blade-heart-type condition");
}

static void test_threshold_blade_hearts_count_base_hearts_ignored(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    blade_heart_threshold_scenario(&game, 3, 0, 2, 0, &h1, &bonus);
    CHECK_EQ(h1, 1,
             "3 blade-heart types grant heart01 even with base-heart-only cards present");
}

static void test_threshold_duplicate_blade_heart_colors_do_not_stack(void)
{
    static TestGame game;
    test_game_new(&game);
    int h1 = 0, bonus = 0;
    /* 2 distinct colours (heart01 twice + heart02) */
    blade_heart_threshold_scenario(&game, 2, 0, 0, 1, &h1, &bonus);
    CHECK_EQ(h1, 0,
             "3 cards but only 2 DISTINCT blade-heart colours -> the threshold "
             "of 3 is not met");
}

/* ====================================================================== */
/* 18. YELL — PL!-bp5-004-R＋: 3 blade-heartless reveals set HeartColor::All*/
/*     (3)                                                                */
/* ====================================================================== */

static int has_all_heart_modifier(TestGame *tg)
{
    /* Rust scans every card's heart map for HeartColor::All. Probing the
     * ability card itself is equivalent here because the grant targets the
     * 自動's own card. */
    int src = test_id(tg, ALL_HEART_SRC);
    return rb_mods_get_heart(&tg->state.mods, src, RB_HEART_ALL) > 0;
}

static void all_heart_scenario(TestGame *tg, const char *const *cards, int n,
                               int *out_before, int *out_after)
{
    int src = test_id(tg, ALL_HEART_SRC);
    CHECK(rb_card_no_eq(src, ALL_HEART_SRC), "the source is the PL!-bp5-004-R＋ print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[1] = src;
    tg->state.n_revealed = 0;
    for (int i = 0; i < n; i++) {
        int id = test_new_id(tg, cards[i]);
        tg->state.revealed_cards[i] = id;
        tg->state.n_revealed = i + 1;
        rb_waitroom_add(&tg->state.p[0], id);
    }
    tg->state.yell_occurred = n > 0;

    *out_before = has_all_heart_modifier(tg);
    tas(tg, 0);
    drain_auto(tg);
    *out_after = has_all_heart_modifier(tg);
}

static void test_yell_three_blade_heartless_members_set_all_heart(void)
{
    static TestGame game;
    test_game_new(&game);
    const char *three[3] = { RIKO_NO_BLADE, "PL!S-PR-013-PR", "PL!S-sd1-006-SD" };
    CHECK(rb_card_no_eq(test_id(&game, three[1]), "PL!S-PR-013-PR"),
          "the second reference is the PL!S-PR-013-PR print");
    CHECK(rb_card_no_eq(test_id(&game, three[2]), "PL!S-sd1-006-SD"),
          "the third reference is the PL!S-sd1-006-SD print");
    int b = 0, a = 0;
    all_heart_scenario(&game, three, 3, &b, &a);
    CHECK_EQ(b, 0, "HeartColor::All must not be present before");
    CHECK_EQ(a, 1, "three qualifying members in the yell must set HeartColor::All");
}

static void test_yell_three_blade_heart_members_do_not_set_all_heart(void)
{
    static TestGame game;
    test_game_new(&game);
    const char *three[3] = { BLADE_PB1, BLADE_PB1, BLADE_PB1 };
    int b = 0, a = 0;
    all_heart_scenario(&game, three, 3, &b, &a);
    CHECK_EQ(a, b, "three members WITH blade hearts must not set HeartColor::All");
}

static void test_yell_two_blade_heartless_members_do_not_set_all_heart(void)
{
    static TestGame game;
    test_game_new(&game);
    const char *two[2] = { RIKO_NO_BLADE, "PL!S-PR-013-PR" };
    int b = 0, a = 0;
    all_heart_scenario(&game, two, 2, &b, &a);
    CHECK_EQ(a, b, "only two qualifying members must not set HeartColor::All");
}

/* ====================================================================== */
/* 19. YELL — PL!S-bp2-007-R＋: a revealed LIVE card draws 1, but only     */
/*     while the hand holds at most 7.  (3)                               */
/* ====================================================================== */

static void live_reveal_draw_scenario(TestGame *tg, const char *revealed_no,
                                      int n_hand, int *out_delta)
{
    int src = test_id(tg, LIVE_DRAW_SRC);
    CHECK(rb_card_no_eq(src, LIVE_DRAW_SRC), "the source is the PL!S-bp2-007-R＋ print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[1] = src;
    int filler = test_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < n_hand; i++) test_add_to_hand(tg, test_new_id(tg, FILLER));

    int id = test_new_id(tg, revealed_no);
    tg->state.n_revealed = 0;
    tg->state.revealed_cards[0] = id;
    tg->state.n_revealed = 1;
    rb_waitroom_add(&tg->state.p[0], id);
    tg->state.yell_occurred = 1;

    int before = game_hand_len(tg);
    tas(tg, 0);
    drain_auto(tg);
    *out_delta = game_hand_len(tg) - before;
}

static void test_live_revealed_draws_one_with_empty_hand(void)
{
    static TestGame game;
    test_game_new(&game);
    int d = 0;
    live_reveal_draw_scenario(&game, REVEALED_LIVE, 0, &d);
    CHECK(rb_card_no_eq(test_id(&game, REVEALED_LIVE), REVEALED_LIVE),
          "the revealed card is the PL!-bp3-026-L print");
    CHECK_EQ(d, 1, "a revealed LIVE card draws 1 with the hand at most 7");
}

static void test_live_revealed_with_eight_hand_cards_does_not_draw(void)
{
    static TestGame game;
    test_game_new(&game);
    int d = 0;
    live_reveal_draw_scenario(&game, REVEALED_LIVE, 8, &d);
    CHECK_EQ(d, 0, "the reveal must not draw when the hand already holds 8");
}

static void test_yell_without_live_card_does_not_draw(void)
{
    static TestGame game;
    test_game_new(&game);
    int d = 0;
    live_reveal_draw_scenario(&game, FILLER, 0, &d);
    CHECK_EQ(d, 0, "no LIVE card in the yell -> no draw");
}

/* ====================================================================== */
/* 20. YELL — JIMO-AI Dash! PL!S-sd1-020-SD ab#0: ライブ成功時, draw 1     */
/*     per 『Aqours』 member, then discard the same count.  (4)             */
/* ====================================================================== */

static void jimoai_scenario(TestGame *tg, int extra_hand, int n_aqours,
                            int *out_deck_delta, int *out_hand)
{
    int sd20 = test_id(tg, JIMOAI);
    int filler = test_new_id(tg, FILLER);
    CHECK(rb_card_no_eq(sd20, JIMOAI), "the host is the PL!S-sd1-020-SD print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < n_aqours && i < 3; i++)
        tg->state.p[0].stage[i] = test_new_id(tg, AQ_MEMBER);
    for (int i = 0; i < extra_hand; i++) test_add_to_hand(tg, test_new_id(tg, FILLER));
    /* The Rust helper addresses the ライブ成功時 by ability id; the C shim
     * (rb_trigger_live_success) scans the board's zones, so the member must
     * actually be on the stage. */
    tg->state.p[0].stage[2] = sd20;

    int deck_before = test_deck_len(tg);
    int hand_before = test_hand_len(tg);

    tg->state.live_success[0] = 1;

    rb_trigger_live_success(&tg->state, 0);
    /* The second action is a fixed-count hand discard. */
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 64) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD && c->count > 0) {
            int idx[RB_MAX_ZONE];
            int n = c->count < RB_MAX_ZONE ? c->count : RB_MAX_ZONE;
            for (int i = 0; i < n; i++) idx[i] = i;
            test_select_indices(tg, idx, n);
        } else {
            rb_resume_with_choice(&tg->state, 0);
        }
    }
    /* Rust notes the hand-selection cost path records the picks without moving
     * them (a known C gap in rb_resolver_handle_hand_selection), so the hand
     * delta is reported for both the engine's and the helper's reading. */
    *out_deck_delta = deck_before - test_deck_len(tg);
    *out_hand = test_hand_len(tg);
}

static void test_jimoai_draw_2_discard_2(void)
{
    static TestGame game;
    test_game_new(&game);
    int dd = 0, h = 0;
    jimoai_scenario(&game, 5, 2, &dd, &h);
    CHECK_EQ(dd, 2, "2 『Aqours』 members -> draw 2");
    CHECK_EQ(h, 5, "hand back to 5 after discarding the same count");
}

static void test_jimoai_draw_1_discard_1(void)
{
    static TestGame game;
    test_game_new(&game);
    int dd = 0, h = 0;
    jimoai_scenario(&game, 5, 1, &dd, &h);
    CHECK_EQ(dd, 1, "1 『Aqours』 member -> draw 1");
    CHECK_EQ(h, 5, "hand back to 5 after discarding the same count");
}

static void test_jimoai_draw_0_no_discard(void)
{
    static TestGame game;
    test_game_new(&game);
    int dd = 0, h = 0;
    jimoai_scenario(&game, 5, 0, &dd, &h);
    CHECK_EQ(dd, 0, "0 『Aqours』 members -> draw 0");
    CHECK_EQ(h, 5, "and therefore discard 0: the hand is unchanged");
}

/* ====================================================================== */
/* 21. YELL — MIRACLE WAVE PL!S-bp3-019-L: at ライブ成功時, score = 4     */
/*     when the yell revealed no blade heart OR surplus ≥ 2.  (1)          */
/* ====================================================================== */

static void test_miracle_wave_excess_heart_scores_four(void)
{
    static TestGame game;
    test_game_new(&game);

    int wave = test_id(&game, MIRACLE);
    int chika1 = test_id(&game, AQ_CHIKA);
    int chika2 = test_new_id(&game, AQ_CHIKA);
    CHECK(rb_card_no_eq(wave, MIRACLE), "the live card is the PL!S-bp3-019-L print");
    CHECK(chika1 != chika2, "the two 千歌 instances are distinct pool slots");

    fill_decks(&game, 20);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = chika1;
    game.state.p[0].stage[1] = chika2;
    test_add_to_hand(&game, wave);

    set_live_at_set_phase(&game, MIRACLE);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE_SECOND);
    drain_auto(&game);
    test_advance_to_phase(&game, RB_PHASE_VICTORY);
    drain_auto(&game);
    test_pass(&game);
    drain_auto(&game);
    decline_all(&game);

    CHECK_EQ(score_mod(&game, wave), 0,
             "the ライブ成功時 set_score grant is reverted after the live");
    int found = 0, score = -1;
    for (int i = 0; i < game.state.n_snapshots; i++) {
        RbLiveSnapshot *s = &game.state.snapshots[i];
        for (int k = 0; k < s->n_lives; k++) {
            if (s->lives[k] == wave) { found = 1; score = s->total_score; }
        }
    }
    CHECK(found, "a performance snapshot for the live card exists");
    CHECK_EQ(score, 4, "Q182: excess heart ≥ 2 -> the live scores exactly 4");
}

/* ====================================================================== */
/* 22. YELL — 黒澤ダイヤ PL!S-bp2-004-R ab#0: no LIVE card among the      */
/*     revealed -> discard them all and re-yell.  (2)                      */
/* ====================================================================== */

static void dia_scenario(TestGame *tg, int *out_pending)
{
    int dia = test_id(tg, DIA);
    int filler = test_new_id(tg, FILLER);
    CHECK(rb_card_no_eq(dia, DIA), "the 自動 host is the PL!S-bp2-004-R print");

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.p[0].stage[0] = test_new_id(tg, FILLER);
    tg->state.p[0].stage[1] = test_new_id(tg, FILLER);
    tg->state.p[0].stage[2] = dia;
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 60; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    test_give_energy(tg, 15);

    set_live_at_set_phase(tg, GENERIC_LIVE);
    test_advance_to_phase(tg, RB_PHASE_PERFORMANCE_SECOND);
    drain_auto(tg);
    *out_pending = pending(tg);
}

static void test_dia_re_yell_works(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0;
    dia_scenario(&game, &p);
    CHECK(p, "no LIVE card among the revealed -> the optional discard prompt");
    if (p) {
        int count = pending_count(&game);
        int idx[RB_MAX_ZONE];
        int n = count < RB_MAX_ZONE ? count : RB_MAX_ZONE;
        for (int i = 0; i < n; i++) idx[i] = i;
        test_select_indices(&game, idx, n);
        drain_auto(&game);
    }
    CHECK(game.state.n_revealed > 0, "the original yell really revealed cards");
    CHECK_EQ(re_yell_happened(&game), 1,
             "Q107: accepting the discard makes the re-yell follow");
}

static void test_dia_skip_discard_no_followup(void)
{
    static TestGame game;
    test_game_new(&game);
    int p = 0;
    dia_scenario(&game, &p);
    CHECK(p, "the optional discard prompt is raised");
    decline_all(&game);
    CHECK_EQ(re_yell_happened(&game), 0,
             "Q107: skipping the optional discard means no re-yell");
}


int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- M1: 嵐千砂都 PL!SP-bp2-003-R area move -> waited energy ---\n");
    run("chisato_position_change_places_waited_energy",
        test_chisato_position_change_places_waited_energy);
    run("chisato_leaving_stage_does_not_place_energy",
        test_chisato_leaving_stage_does_not_place_energy);
    run("chisato_no_energy_when_another_card_debuts",
        test_chisato_no_energy_when_another_card_debuts);
    run("chisato_no_energy_for_a_third_member_swap",
        test_chisato_no_energy_for_a_third_member_swap);
    run("chisato_gains_energy_when_she_is_swapped",
        test_chisato_gains_energy_when_she_is_swapped);

    printf("--- M2: 唐 可可 heart06 自動 ---\n");
    run("keke_kidou_position_change_grants_heart06",
        test_keke_kidou_position_change_grants_heart06);
    run("keke_opponent_effect_area_move_triggers_heart06",
        test_keke_opponent_effect_area_move_triggers_heart06);
    run("keke_natural_move_grants_nothing", test_keke_natural_move_grants_nothing);
    run("keke_turn1_blocks_a_second_area_move", test_keke_turn1_blocks_a_second_area_move);

    printf("--- M3: ターン1回 cap and ライブ終了時まで expiry ---\n");
    run("kanon_turn1_blocks_a_second_area_move", test_kanon_turn1_blocks_a_second_area_move);
    run("fuyumari_turn1_blocks_a_second_area_move", test_fuyumari_turn1_blocks_a_second_area_move);
    run("kanon_heart02_does_not_survive_the_live", test_kanon_heart02_does_not_survive_the_live);
    run("fuyumari_heart03_does_not_survive_the_live", test_fuyumari_heart03_does_not_survive_the_live);

    printf("--- M4: 桜小路きな子 PL!SP-pb1-006-R debut/move +2 blade ---\n");
    run("kinako_q94_debut_grants_two_blades", test_kinako_q94_debut_grants_two_blades);
    run("kinako_q94_debut_blades_expire_at_live_victory",
        test_kinako_q94_debut_blades_expire_at_live_victory);
    run("kinako_q94_debut_then_swap_stacks_four_blades",
        test_kinako_q94_debut_then_swap_stacks_four_blades);

    printf("--- M5: real 起動 swaps arm three area-move 自動 ---\n");
    run("area_swap_grants_self_two_blades", test_area_swap_grants_self_two_blades);
    run("area_swap_grants_self_heart02", test_area_swap_grants_self_heart02);
    run("area_swap_grants_self_heart03", test_area_swap_grants_self_heart03);
    run("area_swap_recovers_low_score_group_live", test_area_swap_recovers_low_score_group_live);
    run("shiki_wait_activation_then_area_swap_clears_wait",
        test_shiki_wait_activation_then_area_swap_clears_wait);

    printf("--- M6: 若菜四季's wait gate / cycle / draw ---\n");
    run("shiki_kidou_waits_her_and_draws_exactly_one",
        test_shiki_kidou_waits_her_and_draws_exactly_one);
    run("shiki_auto_does_not_fire_while_she_is_active",
        test_shiki_auto_does_not_fire_while_she_is_active);
    run("shiki_auto_is_one_shot_per_waited_state", test_shiki_auto_is_one_shot_per_waited_state);

    printf("--- M7: root cause probe for the position|destination answer ---\n");
    run("position_destination_answer_performs_the_swap",
        test_position_destination_answer_performs_the_swap);

    printf("--- M8: Aspire PL!SP-sd2-025-P timing condition on the blade ---\n");
    run("aspire_moved_liella_gains_blade_only",
        test_aspire_moved_liella_gains_blade_only);
    run("aspire_only_non_liella_moved_no_blade",
        test_aspire_only_non_liella_moved_no_blade);
    run("aspire_liella_and_non_liella_moved_only_liella_gains",
        test_aspire_liella_and_non_liella_moved_only_liella_gains);
    run("aspire_no_movement_no_blade", test_aspire_no_movement_no_blade);

    printf("--- M9: 鬼塚冬毬 PL!SP-bp4-011-R＋ 登場/area-move opponent wait ---\n");
    run("fuyumari_appear_triggers_opponent_wait",
        test_fuyumari_appear_triggers_opponent_wait);
    run("fuyumari_area_move_triggers_opponent_wait",
        test_fuyumari_area_move_triggers_opponent_wait);
    run("fuyumari_blade_limit_excludes_high_blade",
        test_fuyumari_blade_limit_excludes_high_blade);
    run("fuyumari_already_wait_excluded_from_candidates",
        test_fuyumari_already_wait_excluded_from_candidates);

    printf("--- M10: ability chains (jidou feeds 常時) ---\n");
    run("pb2006_jidou_placement_feeds_constant_cost_up",
        test_pb2006_jidou_placement_feeds_constant_cost_up);
    run("pb2006_jidou_once_per_turn_no_second_placement",
        test_pb2006_jidou_once_per_turn_no_second_placement);
    run("special_color_set_blades_and_score_twin_in_one_live",
        test_special_color_set_blades_and_score_twin_in_one_live);
    run("special_color_no_score_when_moved_liella_left_center",
        test_special_color_no_score_when_moved_liella_left_center);

    printf("--- M11: under-member placement on her own area move ---\n");
    run("kinako_tucks_group_member_under_self_on_area_move",
        test_kinako_tucks_group_member_under_self_on_area_move);

    printf("--- M12: PL!S-bp5-023-L ab#0 waitroom lives -> deck top ---\n");
    run("awaken_happy_path_prompts_for_four_cards",
        test_awaken_happy_path_prompts_for_four_cards);
    run("awaken_no_saintsnow_does_not_fire", test_awaken_no_saintsnow_does_not_fire);
    run("awaken_cost_below_20_does_not_fire", test_awaken_cost_below_20_does_not_fire);
    run("awaken_cost_at_least_20_fires", test_awaken_cost_at_least_20_fires);
    run("awaken_can_skip", test_awaken_can_skip);
    run("awaken_no_matching_live_in_waitroom_skips",
        test_awaken_no_matching_live_in_waitroom_skips);
    run("awaken_only_matching_lives_are_filtered",
        test_awaken_only_matching_lives_are_filtered);

    printf("--- Y1: yell-reveal blade-heart absence (heart06 / heart03) ---\n");
    run("yell_no_blade_heart_grants_heart06_and_heart03",
        test_yell_no_blade_heart_grants_heart06_and_heart03);
    run("yell_with_blade_heart_blocks_both_watchers",
        test_yell_with_blade_heart_blocks_both_watchers);
    run("yell_all_blade_blocks_both_watchers", test_yell_all_blade_blocks_both_watchers);
    run("yell_mixed_blade_and_no_blade_still_blocks",
        test_yell_mixed_blade_and_no_blade_still_blocks);
    run("yell_two_no_blade_cards_still_grants", test_yell_two_no_blade_cards_still_grants);
    run("yell_empty_revealed_no_gain", test_yell_empty_revealed_no_gain);
    run("yell_turn1_blocks_a_second_trigger", test_yell_turn1_blocks_a_second_trigger);
    run("yell_turn_limit_resets_next_turn", test_yell_turn_limit_resets_next_turn);
    run("yell_only_one_present_triggers_self_only",
        test_yell_only_one_present_triggers_self_only);

    printf("--- Y2: ウィーン's heart03 through a REAL yell ---\n");
    run("wien_q112_real_yell_no_blade_heart_grants_heart03",
        test_wien_q112_real_yell_no_blade_heart_grants_heart03);
    run("wien_q112_real_yell_with_blade_heart_no_gain",
        test_wien_q112_real_yell_with_blade_heart_no_gain);
    run("wien_q113_zero_score_live_yields_no_yell",
        test_wien_q113_zero_score_live_yields_no_yell);

    printf("--- Y3: ダイスキだったらダイジョウブ！ re-yell (≤2 blade hearts) ---\n");
    run("daisuki_zero_stage_no_reveal_no_trigger",
        test_daisuki_zero_stage_no_reveal_no_trigger);
    run("daisuki_two_blade_hearts_prompts", test_daisuki_two_blade_hearts_prompts);
    run("daisuki_three_blade_hearts_blocks", test_daisuki_three_blade_hearts_blocks);
    run("daisuki_zero_blade_hearts_prompts", test_daisuki_zero_blade_hearts_prompts);
    run("daisuki_accept_discard_then_re_yells", test_daisuki_accept_discard_then_re_yells);
    run("daisuki_skip_discard_no_re_yell", test_daisuki_skip_discard_no_re_yell);

    printf("--- Y4: 鬼塚夏美 yell -> discard a 『Liella!』 live, re-yell x2 ---\n");
    run("natsumi_discards_liella_live_for_two_extra_yells",
        test_natsumi_discards_liella_live_for_two_extra_yells);
    run("natsumi_decline_discard_no_extra_yells",
        test_natsumi_decline_discard_no_extra_yells);
    run("natsumi_no_liella_live_in_hand_no_prompt",
        test_natsumi_no_liella_live_in_hand_no_prompt);
    run("natsumi_use_limit_blocks_a_second_yell",
        test_natsumi_use_limit_blocks_a_second_yell);

    printf("--- Y5: distinct blade-heart type thresholds (3 and 6) ---\n");
    run("threshold_zero_blade_heart_types_no_effect",
        test_threshold_zero_blade_heart_types_no_effect);
    run("threshold_two_blade_heart_types_no_effect",
        test_threshold_two_blade_heart_types_no_effect);
    run("threshold_three_blade_heart_types_grants_heart01",
        test_threshold_three_blade_heart_types_grants_heart01);
    run("threshold_five_blade_heart_types_heart01_only",
        test_threshold_five_blade_heart_types_heart01_only);
    run("threshold_six_blade_heart_types_grants_both",
        test_threshold_six_blade_heart_types_grants_both);
    run("threshold_all_blade_heart_types_with_b_all_grants_both",
        test_threshold_all_blade_heart_types_with_b_all_grants_both);
    run("threshold_base_heart_only_does_not_count",
        test_threshold_base_heart_only_does_not_count);
    run("threshold_blade_hearts_count_base_hearts_ignored",
        test_threshold_blade_hearts_count_base_hearts_ignored);
    run("threshold_duplicate_blade_heart_colors_do_not_stack",
        test_threshold_duplicate_blade_heart_colors_do_not_stack);

    printf("--- Y6: three blade-heartless reveals set HeartColor::All ---\n");
    run("yell_three_blade_heartless_members_set_all_heart",
        test_yell_three_blade_heartless_members_set_all_heart);
    run("yell_three_blade_heart_members_do_not_set_all_heart",
        test_yell_three_blade_heart_members_do_not_set_all_heart);
    run("yell_two_blade_heartless_members_do_not_set_all_heart",
        test_yell_two_blade_heartless_members_do_not_set_all_heart);

    printf("--- Y7: a revealed LIVE card draws 1 (hand <= 7) ---\n");
    run("live_revealed_draws_one_with_empty_hand",
        test_live_revealed_draws_one_with_empty_hand);
    run("live_revealed_with_eight_hand_cards_does_not_draw",
        test_live_revealed_with_eight_hand_cards_does_not_draw);
    run("yell_without_live_card_does_not_draw", test_yell_without_live_card_does_not_draw);

    printf("--- Y8: JIMO-AI Dash! draw N then discard N ---\n");
    run("jimoai_draw_2_discard_2", test_jimoai_draw_2_discard_2);
    run("jimoai_draw_1_discard_1", test_jimoai_draw_1_discard_1);
    run("jimoai_draw_0_no_discard", test_jimoai_draw_0_no_discard);

    printf("--- Y9: MIRACLE WAVE scores 4 on surplus heart >= 2 ---\n");
    run("miracle_wave_excess_heart_scores_four", test_miracle_wave_excess_heart_scores_four);

    printf("--- Y10: 黒澤ダイヤ no-live-card re-yell ---\n");
    run("dia_re_yell_works", test_dia_re_yell_works);
    run("dia_skip_discard_no_followup", test_dia_skip_discard_no_followup);

    printf("\n== parity_jidou_move_yell: %d ok, %d failed, %d crashed "
           "(case in flight: %s)\n", n_ok, n_failed, n_crashed, current_case);
    printf("== %ld assertions total, %d assertion failures\n", assertions, failures);
    return (n_failed || n_crashed) ? 1 : 0;
}

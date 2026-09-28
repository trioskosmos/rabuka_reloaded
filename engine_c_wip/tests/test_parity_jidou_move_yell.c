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
/* 1. harness helpers                                                     */
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

/* ── card constants (every one verified against cards/cards.json) ──────────── */
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
/* 2. MOVEMENT — 嵐千砂都 PL!SP-bp2-003-R: 自動 ターン1回 area move -> 1   */
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
    while (game.state.current_phase != RB_PHASE_VICTORY && guard++ < 20) {
        test_pass(&game);
        drain_auto(&game);
        if (pending(&game)) decline_all(&game);
        if (game.state.current_phase == RB_PHASE_VICTORY) break;
    }
    CHECK_EQ(game.state.current_phase, RB_PHASE_VICTORY,
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
    while (game.state.current_phase != RB_PHASE_VICTORY && guard++ < 20) {
        test_pass(&game);
        drain_auto(&game);
        if (pending(&game)) decline_all(&game);
        if (game.state.current_phase == RB_PHASE_VICTORY) break;
    }
    CHECK_EQ(game.state.current_phase, RB_PHASE_VICTORY,
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
/* main                                                                    */
/* ====================================================================== */

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

    printf("\n== parity_jidou_move_yell: %d ok, %d failed, %d crashed "
           "(case in flight: %s)\n", n_ok, n_failed, n_crashed, current_case);
    printf("== %ld assertions total, %d assertion failures\n", assertions, failures);
    return (n_failed || n_crashed) ? 1 : 0;
}

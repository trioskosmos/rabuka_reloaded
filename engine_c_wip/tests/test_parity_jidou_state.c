/* test_parity_jidou_state.c ? C port of the jidou sub-folders that
 * tests/test_parity_jidou.c and tests/test_parity_jidou_extra.c left open.
 *
 * Sources, with the real size of each cluster (files / #[test] fns):
 *
 *   A. jidou/state_watch/live_phase_group_wait_discard_reactivate_test.rs
 *      1 file / 7 #[test] ? 三船栞子 PL!N-bp7-022-N ab#0
 *         「ライブフェイズの間、自分のステージにいる『虹ヶ咲』のメンバー1人が
 *           ウェイト状態になったとき、手札を1枚控え室に置いてもよい。
 *           そうしたとき、そのメンバーをアクティブにする。」
 *   B. jidou/state_watch/own_effect_wait_cheap_opponent_draw_one_q177_test.rs
 *      1 file / 7 #[test] ? 西木野真姫 PL!-pb1-015-R ab#1 (own-effect wait of a
 *      cost<=4 OPPONENT member -> draw 1) plus the ab#0 optional-cost gate.
 *   C. jidou/title_once/dive_ab0_does_not_arm_without_own_main_phase_discard_
 *      to_hand_test.rs ? 1 file / 6 #[test] ? DIVE! PL!N-bp4-026-L ab#0's
 *      「自分のメインフェイズにこのカードが控え室から手札に加えられたとき」 gate.
 *   D. jidou/title_once/dive_ab0_phase_gate_and_multi_copy_live_zone_limit_
 *      edges_test.rs ? 1 file / 10 #[test] ? DIVE! ab#0 multi-copy + ab#1 +
 *      live_card_set_limit_reduction.
 *   E. jidou/title_once/dive_retrieved_to_hand_ab0_places_live_zone_ab1_
 *      grants_blade_test.rs ? 1 file / 8 #[test].
 *   F. jidou/leaves_stage/live_success_heart05_threshold_and_no_surplus_score_
 *      plus2_test.rs ? 1 file / 3 #[test] ? Strawberry Trapper
 *      PL!S-pb1-021-L 「『Aqours』のメンバーが持つハートにheart05が合計4個以上
 *      あり、このターン、相手が余剰のハートを持たずにライブを成功させていた場合、
 *      このカードのスコアを＋２する。」
 *
 * NOT ported here (already covered):
 *   jidou/title_once/dive_in_live_zone_only_ab1_grants_blade_test.rs
 *     (1 file / 3 #[test]) ? test_parity_jidou_extra.c section G already ports
 *     all three tests verbatim (dive_live_zone_only_ab1_triggers,
 *     dive_not_in_live_zone_no_trigger, dive_no_niji_no_target).
 *
 * ── C API notes that matter here ────────────────────────────────────────
 *   - sizeof(GameState) is ~781 KB, so every multi-fixture TestGame local in
 *     this file is `static`. (A stack TestGame segfaults.)
 *   - `rb_load("src")` fails under the isolated out-of-tree build, so
 *     load_card_db() falls back to "../cards/build".
 *   - Rust `game.state.set_recently_moved_cards(v)` -> C
 *     `state.recently_moved[] / n_recently_moved`.
 *   - Rust `game.state.recently_state_changed.push((card, from, to, cause))`
 *     -> C `state.recently_state_changed[]` (card ids) PLUS the per-card
 *     `state_change_from[card] / state_change_to[card]` pair, which is what
 *     condition.c:eval_state_change actually reads. The C struct has NO cause
 *     column at all ? see the B_opponent_* tests.
 *   - `test_get_heart_modifier` REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE (heart05).
 *   - `rb_record_card_movement`'s SIXTH parameter is the Rust `effect_only`
 *     flag even though the header names it `target`.
 *   - The C phase discriminator is `g->active` vs `g->first_attacker` (Rust's
 *     `current_turn_phase` SecondAttackerNormal), so "P2's own main phase" is
 *     `g->active = 1` with the default first_attacker=0/second_attacker=1.
 *
 * ── the choice-drain obligation ─────────────────────────────────────────
 * rb_resume_with_choice does NOT continue the C resolver: the rest of a
 * `sequential` effect is parked as a NEW queue entry (choice.c logs
 * "CHOICE_EPILOGUE ... queue_actions=1"). Rust's `select_indices` continues
 * in-process, so a test that only loops on `has_pending_choice` silently stops
 * halfway through every multi-step effect. drain_accept_optionals() therefore
 * alternates between answering prompts and rb_drain_ability_queue(). This is
 * load-bearing for 三船栞子 ab#0 (move_cards then change_state) and DIVE! ab#0
 * (move_cards then reduce_live_card_set_limit).
 *
 * ── fork isolation ──────────────────────────────────────────────────────
 * The engine has confirmed process-killing faults in the ability effects, so
 * each test runs in a forked child; a child that dies on a signal is reported
 * as CRASH, never as a pass, and the SIGSEGV/SIGBUS/SIGABRT handler names the
 * test in flight. No watchdog: this toolchain cannot deliver one reliably.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN ? PL!N-bp4-007-R＋ is a DIFFERENT card number
 * from PL!N-bp4-007-R; cards.json only has the fullwidth form. */
#define PLUS "\xef\xbc\x8b"

static int failures;
static int assertions;
static int harness_gaps;
static const char *current_test = "(none)";

/* A real assertion. Failing it fails the test, and the test fails the run. */
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
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A C-vs-Rust HARNESS gap: the Rust assertion is not expressible through the
 * C shim at all (no API surface, not a behaviour). Counted separately from a
 * parity failure. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

#define DIAG(...) do { fprintf(stderr, "        (diagnostic: " __VA_ARGS__); \
                       fprintf(stderr, ")\n"); } while (0)

/* ???????????????????????????????????????????????????????????????????????
 * card constants ? every one verified against cards/cards.json
 * ??????????????????????????????????????????????????????????????????????? */
#define FILLER        "PL!-sd1-010-SD"   /* 高坂 穂乃果   cost 4  Printemps */
#define SHIORIKO      "PL!N-bp7-022-N"   /* 三船栞子       cost 4  R3BIRTH   */
#define NIJI_MEMBER   "PL!N-PR-003-PR"   /* 上原歩夢       cost 9  A・ZU・NA  */
#define NIJI_MEMBER2  "PL!N-sd1-001-SD"   /* 上原歩夢       cost 13 A・ZU・NA  */
#define MAKI          "PL!-pb1-015-R"    /* 西木野真姫     cost 11 BiBi      */
#define CHEAP_OPP     "PL!SP-sd1-019-SD" /* 若菜四季       cost 2  5yncri5e!  */
#define CHEAP_OPP_B   "PL!-sd1-011-SD"   /* 絢瀬 絵里     cost 4  BiBi      */
#define EXPENSIVE_OPP "PL!-sd1-014-SD"   /* 星空 凛       cost 9  lilywhite */
#define TOUBATSU      "PL!SP-pb2-011-R"  /* 鬼塚冬毬       cost 13 5yncri5e!  */
#define SHIKI         "PL!SP-bp2-008-R"  /* 若菜四季       cost 9  5yncri5e!  */
#define CHEAP_NIJI    "PL!N-PR-009-PR"   /* 優木せつ菜     cost 2  A・ZU・NA  */
#define AYUMU         "PL!N-bp3-006-R"   /* 近江彼方       cost 9  QU4RTZ    */
#define DIVE          "PL!N-bp4-026-L"   /* DIVE!          score 5           */
#define SETSUNA_BOTH  "PL!N-bp4-007-R" PLUS /* 優木せつ菜 (cross-line retrieval) */
#define SETSUNA_ONE   "PL!N-bp5-019-N"   /* 優木せつ菜 (own-waitroom retrieval) */
#define OTHER_NIJI_LIVE "PL!N-bp4-025-L" /* VIVID WORLD    score 6           */
#define OTHER_LIVE    "PL!-sd1-020-SD"   /* きっと青春が聞こえる score 2      */
#define TRAPPER       "PL!S-pb1-021-L"   /* Strawberry Trapper score 1 GuiltyKiss */
#define RIKO_A        "PL!S-bp2-002-R"   /* 桜内梨子 cost 4 GuiltyKiss, NO printed heart */
#define RIKO_B        "PL!S-sd1-011-SD"  /* 桜内梨子 cost 4 GuiltyKiss, b_heart05 x1   */
#define AQ_H5_B       "PL!S-sd1-017-SD"  /* 小原鞠莉 GuiltyKiss, b_heart05 x1         */
#define AQ_H4_A       "PL!S-sd1-008-SD"  /* 小原鞠莉 GuiltyKiss, b_heart04 x1         */
#define AQ_H4_B       "PL!S-sd1-002-SD"  /* 桜内梨子 GuiltyKiss, b_heart04 x1         */

/* ???????????????????????????????????????????????????????????????????????
 * harness
 * ??????????????????????????????????????????????????????????????????????? */

/* The card blobs live in src/ in the in-tree build, but the isolated build
 * root only copies sources/headers, so fall back to the canonical directory. */
static int load_card_db(void)
{
    static const char *const dirs[] = {"src", "../cards/build", "cards/build",
                                       "../../cards/build"};
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++)
        if (rb_load(dirs[i]) == 0) return 0;
    return 1;
}

static int blade_mod(TestGame *tg, int cid)     { return test_get_blade_modifier(tg, cid); }
static int score_mod(TestGame *tg, int cid)     { return test_get_score_modifier(tg, cid); }

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}
static void answer_first(TestGame *tg) { answer(tg, 0); }
static void answer_skip(TestGame *tg)  { answer(tg, -1); }

/* Rust helpers/choices.rs `drain_auto_ability_choices`: only
 * SelectAutoAbility prompts are answered, everything else breaks the loop. */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        if (strcmp(test_pending_choice_type(tg), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&tg->state, -1);
    }
}

/* The ability queue is left in RB_QUEUE_RESOLVING by rb_resume_with_choice
 * (choice.c logs "CHOICE_RESUME_END ... state=1"), and
 * rb_drain_ability_queue bails out immediately on that state. A test has to
 * drop it back to RB_QUEUE_IDLE before asking the engine to continue, or every
 * effect parked by the choice epilogue ("CHOICE_EPILOGUE ... queue_actions=1")
 * is silently dropped. */
/* The choice epilogue parks the rest of a `sequential` effect in
 * queue.pending_actions[] (not as a new entry), and rb_drain_ability_queue
 * only walks n_entries. rb_queue_resume_pending_actions is public API and is
 * what actually runs the parked tail. */
static void pump_queue(TestGame *tg)
{
    if (rb_has_pending_choice(&tg->state)) return;
    rb_queue_set_state(&tg->state.queue, RB_QUEUE_IDLE);
    rb_queue_resume_pending_actions(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

/* Rust baton_swap_auto_helpers.rs `drain_auto_choices` /
 * `resolve_auto_choices_accepting_optionals`: required 1-card SelectCard
 * prompts are answered with index 0, everything else declines ? and the
 * ability queue is pumped after every round (see the file header). */
static void drain_all_with(TestGame *tg, int idx)
{
    int guard = 0;
    while (guard++ < 400) {
        if (!rb_has_pending_choice(&tg->state)) {
            int before = tg->state.queue.n_entries;
            pump_queue(tg);
            if (!rb_has_pending_choice(&tg->state) &&
                tg->state.queue.n_entries == before) break;
            continue;
        }
        {
            const RbChoice *c = rb_get_pending_choice(&tg->state);
            if (!c) break;
            answer(tg, (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) ? -1 : idx);
        }
    }
    pump_queue(tg);
}
static void drain_accept_optionals(TestGame *tg) { drain_all_with(tg, 0); }
static void drain_decline(TestGame *tg)          { drain_all_with(tg, -1); }

/* Rust TurnEngine::trigger_auto_abilities_for_player + process_pending.
 *
 * NOTE: rb_process_pending_auto_abilities() is NOT usable here. It runs a pass
 * for BOTH seats, and each pass ends with
 *     if (!g->queue.has_pending) { ...; g->queue.n_entries = 0; }
 * (src/core/game_state_abilities.c:2114-2118). The p1 pass therefore wipes a
 * p2-owned entry before the p2 pass can ever see it. Calling the single-seat
 * rb_process_player_abilities() directly is the faithful equivalent of Rust's
 * per-player drain and keeps both seats' autos reachable. */
static void tas_full(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_player_abilities(&tg->state, pl);
    rb_drain_ability_queue(&tg->state);
}
/* Rust helpers::scan_autos_both */
static void scan_autos_both(TestGame *tg)
{
    tas_full(tg, 0);
    tas_full(tg, 1);
    drain_accept_optionals(tg);
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int live_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].live, cid); }
static int hand_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].hand, cid); }
static int wait_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].discard, cid); }
static int stage_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return 1;
    return 0;
}
static const char *orientation_of(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o ? o : "";
}
static int is_waited(TestGame *tg, int cid) { return strcmp(orientation_of(tg, cid), "wait") == 0; }

/* test_add_to_discard is P1-only; this is the p2 waitroom equivalent (the C
 * engine names the waitroom `discard`). */
static void add_to_waitroom_for(TestGame *tg, int pl, int cid)
{
    rb_waitroom_add(&tg->state.p[pl], cid);
}

/* Rust `state.set_recently_moved_cards(v)` */
static void set_recently_moved_n(TestGame *tg, const int *ids, int n)
{
    tg->state.n_recently_moved = 0;
    for (int i = 0; i < n && i < RB_MAX_RECENTLY_MOVED; i++)
        tg->state.recently_moved[tg->state.n_recently_moved++] = ids[i];
}
static void set_recently_moved(TestGame *tg, int cid) { set_recently_moved_n(tg, &cid, 1); }

/* Rust `state.push_movement_event(card, from, to, causer, cause_player,
 * effect_only)`. The C shim's sixth parameter IS the Rust effect_only flag. */
static void push_movement(TestGame *tg, int card, int from, int to,
                          int cause_player, int effect_only)
{
    rb_record_card_movement(&tg->state, card, from, to, cause_player, effect_only);
}

/* Rust `state.recently_state_changed.push((card, from, to, cause))`.
 *
 * The C GameState keeps the card ids in recently_state_changed[] and the
 * from/to pair per card in state_change_from[]/state_change_to[]; it has no
 * cause-player column at all. condition.c:eval_state_change (the
 * state_change_condition evaluator) reads the per-card pair, so that is what
 * this helper writes. */
static void record_state_change(TestGame *tg, int card, int from_wait, int to_wait)
{
    if (tg->state.n_recently_state_changed < RB_MAX_RECENTLY_MOVED)
        tg->state.recently_state_changed[tg->state.n_recently_state_changed++] = card;
    if (card >= 0 && card < RB_MAX_CARD_IDS) {
        tg->state.state_change_from[card] = (int8_t)from_wait;
        tg->state.state_change_to[card]   = (int8_t)to_wait;
    }
    if (tg->state.n_turn_state_changes < 64) {
        int row = tg->state.n_turn_state_changes++;
        tg->state.turn_state_changes[row][0] = tg->state.activating_card;
        tg->state.turn_state_changes[row][1] = card;
        tg->state.turn_state_changes[row][2] = from_wait ? 'w' : 'a';
        tg->state.turn_state_changes[row][3] = to_wait ? 'w' : 'a';
    }
}

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

/* Printed heart count of a colour, exactly as condition.c:eval_group_aggregate
 * -> get_card_total_hearts would read the Card record. */
static int printed_heart(int cid, int color)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int n = 0;
    for (int h = 0; h < c.n_hearts; h++)
        if ((c.heart_color[h] % 8) == color) n += c.heart_count[h];
    rb_free_card(&c);
    return n;
}

/* Raw Card heart record, so a test can quote the engine's own numbers. */
static void dump_card_hearts(int cid, const char *no)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) {
        DIAG("%s: undecodable", no);
        return;
    }
    fprintf(stderr, "        (diagnostic: %s n_hearts=%d heart_color=[", no, c.n_hearts);
    for (int h = 0; h < c.n_hearts; h++) fprintf(stderr, "%d ", c.heart_color[h]);
    fprintf(stderr, "] heart_count=[");
    for (int h = 0; h < c.n_hearts; h++) fprintf(stderr, "%d ", c.heart_count[h]);
    fprintf(stderr, "])\n");
    rb_free_card(&c);
}

/* ???????????????????????????????????????????????????????????????????????
 * A. jidou/state_watch/live_phase_group_wait_discard_reactivate_test.rs
 *
 * 三船栞子 PL!N-bp7-022-N ab#0 (自動/ターン1回). Its condition is a compound
 * AND of
 *   (1) temporal_condition phase="live_phase"
 *   (2) state_change_condition group 虹ヶ咲, location=stage,
 *       from_state=active, to_state=wait
 * so every negative test below isolates one clause.
 * ??????????????????????????????????????????????????????????????????????? */

/* Rust shioriko_wait_setup(game, waited). */
static void shioriko_wait_setup(TestGame *tg, int waited)
{
    tg->state.p[0].stage[1] = waited;                        /* Center   */
    tg->state.p[0].stage[0] = test_id(tg, SHIORIKO);         /* LeftSide */
    tg->state.phase = RB_PHASE_PERFORMANCE;                   /* FirstAttackerPerformance */
    record_state_change(tg, waited, 0, 1);                   /* active -> wait */
    rb_mods_set_orientation(&tg->state.mods, waited, "wait");
    tas_full(tg, 0);
}

static void test_shioriko_fixtures_are_the_printed_ones(void)
{
    static TestGame game;
    test_game_new(&game);
    int shioriko = test_id(&game, SHIORIKO);
    int walked   = test_id(&game, NIJI_MEMBER);
    CHECK(rb_card_no_eq(shioriko, SHIORIKO),
          "the watcher is the PL!N-bp7-022-N (三船栞子) print");
    CHECK(rb_card_no_eq(walked, NIJI_MEMBER),
          "the waited member is the PL!N-PR-003-PR (上原歩夢) print");
    CHECK(rb_card_matches_group_str(walked, "虹ヶ咲"),
          "precondition: 上原歩夢 really is a 『虹ヶ咲』 member (unit A・ZU・NA)");
    CHECK(rb_card_matches_group_str(shioriko, "虹ヶ咲"),
          "precondition: 三船栞子 herself is a 『虹ヶ咲』 member (unit R3BIRTH)");
}

static void test_live_phase_group_member_wait_optional_discard_removes_wait(void)
{
    static TestGame game;
    test_game_new(&game);

    int shioriko = test_id(&game, SHIORIKO);
    int waited   = test_id(&game, NIJI_MEMBER);
    int filler_id = test_id(&game, FILLER);
    (void)shioriko;
    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, filler_id);        /* the optional cost */
    test_add_to_hand(&game, filler_id);
    game.state.p[0].stage[1] = waited;
    game.state.p[0].stage[0] = shioriko;
    game.state.phase = RB_PHASE_PERFORMANCE;
    record_state_change(&game, waited, 0, 1);
    rb_mods_set_orientation(&game.state.mods, waited, "wait");

    CHECK(is_waited(&game, waited), "precondition: the member really is waited");
    CHECK_EQ(game.state.p[0].hand.n, 1, "precondition: one card in hand to pay the cost");

    tas_full(&game, 0);
    CHECK(pending(&game), "三船栞子 ab#0 must offer the optional discard during the live phase");
    drain_accept_optionals(&game);

    /* The first sequential step is move_cards hand -> 控え室 (discard). The C
       hand-selection resolver (choice.c:1649-1699) accumulates the picked card
       into selected_cards, clears the accumulator at :1698 and NEVER calls
       rb_choice_send_to_dst, so a fixed-count `source:"hand"` move_cards is a
       no-op. Kept strict and red. */
    CHECK(wait_has(&game, 0, filler),
          "『手札を1枚控え室に置いてもよい』 — the optional cost must actually move the card");
    CHECK(!hand_has(&game, 0, filler), "…and it must leave the hand");
    CHECK(!is_waited(&game, waited),
          "accepting the cost reactivates the waited 虹ヶ咲 member (ab#0's second step)");
}

static void test_live_phase_group_wait_decline_discard_stays_wait(void)
{
    static TestGame game;
    test_game_new(&game);

    int waited = test_id(&game, NIJI_MEMBER);
    test_add_to_hand(&game, test_id(&game, FILLER));
    shioriko_wait_setup(&game, waited);

    CHECK(pending(&game), "the optional discard is prompted");
    drain_decline(&game);

    CHECK(is_waited(&game, waited), "declining the cost leaves the member waited");
}

static void test_non_live_phase_wait_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);

    test_add_to_hand(&game, test_id(&game, FILLER));
    game.state.p[0].stage[1] = test_id(&game, NIJI_MEMBER);
    game.state.p[0].stage[0] = test_id(&game, SHIORIKO);
    game.state.phase = RB_PHASE_MAIN;              /* NOT the live phase */
    record_state_change(&game, game.state.p[0].stage[1], 0, 1);
    rb_mods_set_orientation(&game.state.mods, game.state.p[0].stage[1], "wait");

    tas_full(&game, 0);
    drain_auto(&game);

    CHECK(!pending(&game),
          "『ライブフェイズの間』 ? outside the live phase the watcher must not prompt");
    CHECK(is_waited(&game, game.state.p[0].stage[1]), "and the member stays waited");
}

static void test_non_nijigasaki_wait_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);

    /* FILLER is 高坂 穂乃果 (Printemps), NOT 虹ヶ咲. */
    int waited = test_id(&game, FILLER);
    CHECK(!rb_card_matches_group_str(waited, "虹ヶ咲"),
          "precondition: the waited member is NOT a 『虹ヶ咲』 member");
    test_add_to_hand(&game, test_id(&game, FILLER));
    shioriko_wait_setup(&game, waited);
    drain_accept_optionals(&game);

    CHECK(!pending(&game),
          "『自分のステージにいる『虹ヶ咲』のメンバー1人が…』 ? a non-虹ヶ咲 wait must not prompt");
}

static void test_empty_hand_auto_skips_no_reactivate(void)
{
    static TestGame game;
    test_game_new(&game);

    int waited = test_id(&game, NIJI_MEMBER);
    game.state.p[0].hand.n = 0;
    CHECK_EQ(game.state.p[0].hand.n, 0, "precondition: hand starts empty");
    shioriko_wait_setup(&game, waited);
    drain_accept_optionals(&game);

    CHECK(!pending(&game), "empty hand: the optional 1-card cost auto-skips, no prompt");
    CHECK(is_waited(&game, waited), "empty hand: the member stays waited");
}

static void test_second_wait_same_live_phase_does_not_refire(void)
{
    static TestGame game;
    test_game_new(&game);

    int waited_a = test_id(&game, NIJI_MEMBER);
    int waited_b = test_new_id(&game, NIJI_MEMBER);
    CHECK(rb_card_no_eq(waited_a, NIJI_MEMBER) && rb_card_no_eq(waited_b, NIJI_MEMBER),
          "both fixtures are the PL!N-PR-003-PR print");
    CHECK(waited_a != waited_b,
          "test_new_id must allocate a DISTINCT pool slot (Rust game.new_id)");

    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, test_id(&game, FILLER));
    shioriko_wait_setup(&game, waited_a);
    drain_accept_optionals(&game);
    CHECK(!is_waited(&game, waited_a), "the FIRST 虹ヶ咲 wait reactivates");

    /* Second 虹ヶ咲 member waits in the same live phase. */
    game.state.p[0].stage[2] = waited_b;
    record_state_change(&game, waited_b, 0, 1);
    rb_mods_set_orientation(&game.state.mods, waited_b, "wait");
    tas_full(&game, 0);
    drain_auto(&game);

    CHECK(!pending(&game), "ターン1回: the budget is consumed, so no second prompt");
    CHECK(is_waited(&game, waited_b), "ターン1回: the second member stays waited");
}

static void test_self_wait_fires_for_shioriko(void)
{
    static TestGame game;
    test_game_new(&game);

    int shioriko = test_id(&game, SHIORIKO);
    CHECK(rb_card_matches_group_str(shioriko, "虹ヶ咲"),
          "precondition: 三船栞子 herself is a 『虹ヶ咲』 member (unit R3BIRTH)");
    test_add_to_hand(&game, test_id(&game, FILLER));

    game.state.p[0].stage[1] = shioriko;         /* only her on stage */
    game.state.phase = RB_PHASE_PERFORMANCE;
    record_state_change(&game, shioriko, 0, 1);
    rb_mods_set_orientation(&game.state.mods, shioriko, "wait");

    tas_full(&game, 0);
    CHECK(pending(&game), "waiting HERSELF also fires the watcher");
    drain_accept_optionals(&game);
    CHECK(!is_waited(&game, shioriko), "her own wait reactivates her");
}

/* ???????????????????????????????????????????????????????????????????????
 * B. jidou/state_watch/own_effect_wait_cheap_opponent_draw_one_q177_test.rs
 *
 * 西木野真姫 PL!-pb1-015-R:
 *   ab#1 (自動/ターン1回) 「自分のカードの効果によって、相手のステージにいる
 *     アクティブ状態のコスト4以下のメンバーがウェイト状態になったとき、
 *     カードを1枚引く。」 (Q177: the draw is mandatory, it cannot be skipped)
 *   ab#0 (登場/ライブ開始時, センター) 「『BiBi』のメンバー1人をウェイトにして
 *     もよい：相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイト
 *     にする。」 ? ab#0's opponent-wait is what trips ab#1.
 * ??????????????????????????????????????????????????????????????????????? */

static void test_maki_fixtures_are_the_printed_ones(void)
{
    static TestGame game;
    test_game_new(&game);
    int maki  = test_id(&game, MAKI);
    int cheap = test_id(&game, CHEAP_OPP);
    int dear  = test_id(&game, CHEAP_OPP_B);
    int rich  = test_id(&game, EXPENSIVE_OPP);
    CHECK(rb_card_no_eq(maki, MAKI), "the watcher is the PL!-pb1-015-R (西木野真姫) print");
    CHECK(rb_card_no_eq(cheap, CHEAP_OPP), "PL!SP-sd1-019-SD is the cost-2 opponent member");
    CHECK(rb_card_no_eq(dear, CHEAP_OPP_B), "PL!-sd1-011-SD is the cost-4 BiBi opponent member");
    CHECK(rb_card_no_eq(rich, EXPENSIVE_OPP), "PL!-sd1-014-SD is the cost-9 (> 4) opponent member");
    CHECK(rb_card_matches_group_str(maki, "BiBi"),
          "precondition: 西木野真姫 is a 『BiBi』 member, so ab#0's own cost is payable");
    int costs[3] = {0, 0, 0};
    const char *nos[3] = {CHEAP_OPP, CHEAP_OPP_B, EXPENSIVE_OPP};
    for (int i = 0; i < 3; i++) {
        int id = test_id(&game, nos[i]);
        Card c;
        if (id >= 0 && rb_decode_card_by_index((uint32_t)id, &c)) { costs[i] = (int)c.cost; rb_free_card(&c); }
    }
    CHECK(costs[0] <= 4 && costs[1] <= 4,
          "precondition: the two 「cheap」 opponent members really cost <= 4");
    CHECK(costs[2] > 4,
          "precondition: PL!-sd1-014-SD really costs > 4, so it is outside ab#1's gate");
    DIAG("costs = %d / %d / %d", costs[0], costs[1], costs[2]);
}

static void test_own_effect_wait_of_cheap_opponent_after_debut_draws_one_q177(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki   = test_id(&game, MAKI);
    int cheap  = test_id(&game, CHEAP_OPP);
    int cheap2 = test_id(&game, CHEAP_OPP_B);

    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, maki);
    test_add_to_hand(&game, test_id(&game, FILLER));
    game.state.p[1].stage[0] = cheap;
    game.state.p[1].stage[1] = cheap2;
    test_give_energy(&game, 11);
    fill_decks(&game, 10);

    test_play_to_stage(&game, maki, 1);
    drain_auto(&game);

    int hand_after_play = game.state.p[0].hand.n;

    /* Pay the optional cost: wait a 『BiBi』 member (真姫 herself at Center). */
    CHECK(pending(&game), "ab#0's optional 「『BiBi』のメンバー1人をウェイトにしてもよい」 cost must be offered");
    CHECK(pending(&game) && strcmp(pending_kind(&game), "SelectTarget") == 0,
          "expected a SelectTarget optional-cost gate");
    answer_first(&game);
    /* Opponent picks the member to wait. */
    CHECK(pending(&game), "the opponent should be asked which member to wait");
    if (pending(&game)) {
        int cur = game.state.queue.cur;
        DIAG("wait-member prompt is queue entry %d of %d, choice owner seat = %d",
             cur, game.state.queue.n_entries, test_queue_entry_choice_seat(&game, cur));
        CHECK(test_queue_entry_choice_seat(&game, cur) == 1,
              "the wait-member choice is routed to the OPPONENT (p2)");
        answer_first(&game);
    }
    /* NOTE (engine observation, not a behavioural divergence):
       rb_effect_change_state's own re-trigger scan (effects/state.c:629-634)
       calls rb_trigger_auto_abilities_for_player, which only ENQUEUES. Nothing
       then calls rb_process_pending_auto_abilities / rb_drain_ability_queue,
       so in C a state_change_condition 自動 only resolves once the surrounding
       turn loop drains the queue (Rust continues in-process). */
    drain_accept_optionals(&game);
    DIAG("hand after the wait = %d, expected %d", game.state.p[0].hand.n, hand_after_play + 1);

    CHECK_EQ(game.state.p[0].hand.n, hand_after_play + 1,
             "Q177: a cost<=4 opponent member waited by an OWN effect draws 1");
    CHECK(stage_has(&game, 1, cheap), "the opponent member is still on stage, just waited");
    CHECK(is_waited(&game, cheap), "and it is in the wait orientation");
}

static void test_declined_unit_wait_cost_leaves_opponent_active_and_no_draw(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki   = test_id(&game, MAKI);
    int expopp = test_id(&game, EXPENSIVE_OPP);

    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, maki);
    test_add_to_hand(&game, test_id(&game, FILLER));
    game.state.p[1].stage[0] = expopp;
    test_give_energy(&game, 11);
    fill_decks(&game, 10);

    test_play_to_stage(&game, maki, 1);

    CHECK(pending(&game), "the optional wait cost is offered even for a cost-9 target");
    CHECK(pending(&game) && strcmp(pending_kind(&game), "SelectTarget") == 0,
          "expected a SelectTarget optional-cost gate");
    answer_skip(&game);                      /* option index 0 == Skip */
    drain_decline(&game);

    CHECK(!is_waited(&game, expopp),
          "the opponent member is NOT waited when the optional cost was skipped");
    CHECK_EQ(game.state.p[0].hand.n, 1, "and ab#1 draws nothing");
}

static void test_actual_cost_nine_wait_does_not_trigger_maki_draw(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki      = test_id(&game, MAKI);
    int expensive = test_id(&game, EXPENSIVE_OPP);

    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, maki);
    test_add_to_hand(&game, test_id(&game, FILLER));
    game.state.p[1].stage[0] = expensive;
    test_give_energy(&game, 11);
    fill_decks(&game, 10);

    test_play_to_stage(&game, maki, 1);
    CHECK(pending(&game) && strcmp(pending_kind(&game), "SelectTarget") == 0,
          "expected a SelectTarget optional-cost gate");
    answer_first(&game);                     /* option index 1 == Pay */
    drain_accept_optionals(&game);

    CHECK(is_waited(&game, expensive), "the cost-9 opponent member really was waited");
    CHECK_EQ(game.state.p[0].hand.n, 1,
             "『コスト4以下』 ? a cost-9 wait must NOT draw (the Rust test's own "
             "assertion; note the Rust doc-comment claims the evaluator skips "
             "cost_limit, which the C engine does honour)");
}

static void test_declined_cost_with_empty_opponent_stage_draws_nothing(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki = test_id(&game, MAKI);
    clear_stage(&game, 1);
    game.state.p[0].hand.n = 0;
    test_add_to_hand(&game, maki);
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_give_energy(&game, 11);
    fill_decks(&game, 10);

    test_play_to_stage(&game, maki, 1);

    CHECK(pending(&game), "the optional wait cost is offered even with no opponent member");
    int hand_after_play = game.state.p[0].hand.n;
    answer_skip(&game);
    drain_decline(&game);

    CHECK_EQ(game.state.p[0].hand.n, hand_after_play,
             "no opponent member on stage -> ab#0 does nothing and ab#1 does not draw");
}

/* The positive control for the next test: identical fixture, identical
 * recorded transition, scanned identically ? and the draw DOES happen, so the
 * negative below is not vacuously green. */
static void test_own_effect_wait_of_cheap_opponent_draws(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki  = test_id(&game, MAKI);
    int cheap = test_id(&game, CHEAP_NIJI);
    CHECK(rb_card_no_eq(cheap, CHEAP_NIJI),
          "the opponent member is the PL!N-PR-009-PR (優木せつ菜, cost 2) print");

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = maki;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = cheap;
    game.state.p[1].stage[1] = RB_EMPTY_SLOT;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    fill_decks(&game, 10);

    CHECK(!is_waited(&game, cheap), "precondition: the opponent member starts active");

    /* An OWN-card-effect wait of a cost<=4 opponent member. */
    record_state_change(&game, cheap, 0, 1);
    rb_mods_set_orientation(&game.state.mods, cheap, "wait");
    scan_autos_both(&game);

    CHECK(is_waited(&game, cheap), "precondition: the opponent member really was waited");
    CHECK_EQ(game.state.p[0].hand.n, 1,
             "ab#1: an own-effect wait of a cost<=4 OPPONENT member draws exactly 1");
}

static void test_opponent_effect_wait_of_cheap_member_does_not_draw(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki  = test_id(&game, MAKI);
    int cheap = test_id(&game, CHEAP_NIJI);

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = maki;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = cheap;
    game.state.p[1].stage[1] = RB_EMPTY_SLOT;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    fill_decks(&game, 10);

    /* The OPPONENT waits their own member: the same recorded transition, but
       the causer is p2. Rust pushes the cause player as the 4th tuple element;
       the C GameState has no cause column for a state change at all
       (recently_state_changed is a bare i16 list and eval_state_change keys
       only off state_change_from/state_change_to). Both seats are scanned, as
       Rust's scan_autos_both does, so 真姫's ab#1 is genuinely reachable here ?
       B_own_effect_wait_of_cheap_opponent_draws is the matched positive. */
    record_state_change(&game, cheap, 0, 1);
    rb_mods_set_orientation(&game.state.mods, cheap, "wait");
    scan_autos_both(&game);

    CHECK(is_waited(&game, cheap), "precondition: the opponent member really was waited");
    DIAG("P1 hand after the p2-caused wait = %d, expected 0", game.state.p[0].hand.n);
    CHECK_EQ(game.state.p[0].hand.n, 0,
             "『自分のカードの効果によって』 ? a p2-caused wait must NOT draw for 真姫");
}

static void test_opponent_debut_waited_member_does_not_draw(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki  = test_id(&game, MAKI);
    int ayumu = test_id(&game, AYUMU);
    CHECK(rb_card_no_eq(ayumu, AYUMU),
          "the opposing debuter is the PL!N-bp3-006-R (近江彼方) print");

    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = maki;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    clear_stage(&game, 1);
    game.state.p[0].hand.n = 0;
    game.state.p[1].hand.n = 0;
    fill_decks(&game, 10);
    test_add_to_hand_for(&game, 1, ayumu);
    test_give_energy_for(&game, 1, 9);

    int hand_before = game.state.p[0].hand.n;
    int played = test_play_to_stage_for(&game, 1, ayumu, 1);
    drain_accept_optionals(&game);
    scan_autos_both(&game);

    CHECK_EQ(played, 1, "P2's debut of 近江彼方 is accepted");
    CHECK(is_waited(&game, ayumu), "近江彼方 really waited itself via P2's debut effect");
    DIAG("P1 hand after the p2-caused wait = %d, expected %d",
         game.state.p[0].hand.n, hand_before);
    CHECK_EQ(game.state.p[0].hand.n, hand_before,
             "an opponent-caused wait must not draw for 西木野真姫");
}

/* The Rust test that drives the same chain through a real 若菜四季 起動 swap
 * (its cheap_opponent_wait_draw_own_effect_wait_draws). Kept as a scenario
 * probe: it needs 鬼塚冬毬's 3-option choice, which the C shim exposes only as
 * a raw index. */
static void test_own_effect_wait_chain_through_a_real_swap(void)
{
    static TestGame game;
    test_game_new(&game);

    int maki     = test_id(&game, MAKI);
    int toubatsu = test_id(&game, TOUBATSU);
    int shiki    = test_id(&game, SHIKI);
    int cheapopp = test_id(&game, CHEAP_NIJI);
    CHECK(rb_card_no_eq(toubatsu, TOUBATSU),
          "the moving member is the PL!SP-pb2-011-R (鬼塚冬毬) print");
    CHECK(rb_card_no_eq(shiki, SHIKI),
          "the swapper is the PL!SP-bp2-008-R (若菜四季) print");

    game.state.p[0].stage[0] = maki;
    game.state.p[0].stage[1] = toubatsu;
    game.state.p[0].stage[2] = shiki;
    game.state.p[1].stage[0] = cheapopp;
    game.state.p[1].stage[1] = RB_EMPTY_SLOT;
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    fill_decks(&game, 10);
    test_give_energy(&game, 10);
    int hand_before = game.state.p[0].hand.n;

    /* 若菜四季's 起動 swaps the two members: pick the CENTER area for the
       moving card, then the RIGHT slot for the other one (Rust picks the
       "center" generated action, which is the same final layout). */
    test_activate_ability(&game, shiki);
    drain_auto(&game);
    CHECK(pending(&game), "the swap's area choice is offered");
    if (pending(&game)) answer(&game, 1);       /* Center */
    drain_auto(&game);
    CHECK(pending(&game), "the swap's position choice is offered");
    if (pending(&game)) answer(&game, 2);       /* RightSide */
    drain_accept_optionals(&game);

    /* 鬼塚冬毬's 自動 offers three bullets; the Rust test picks index 1
       (the opponent-wait bullet). The C 自動-ability prompt is a single
       SelectAutoAbility, so the option list is not reachable. */
    EXPECTED_HARNESS_GAP(0,
        "鬼塚冬毬 ab#0's 3-option 「以下から1つを選ぶ」 list is not selectable in C: the "
        "C shim raises one SelectAutoAbility prompt with no per-option payload, so the "
        "『相手のステージにいる…メンバーをウェイトにする』 bullet cannot be chosen");
    CHECK_EQ(game.state.p[0].stage[1], shiki,
             "the swap really moved 若菜四季 to Center and 鬼塚冬毬 to Right");
    DIAG("opponent wait orientation after the swap = '%s', hand = %d (was %d)",
         orientation_of(&game, cheapopp), game.state.p[0].hand.n, hand_before);
    CHECK(is_waited(&game, cheapopp),
          "『相手のステージにいる…メンバーをウェイトにする』 ? the cheap opponent member waits");
    CHECK_EQ(game.state.p[0].hand.n, hand_before + 1,
             "真姫 ab#1: own-effect wait of the cheap opponent member draws 1");
}

/* ???????????????????????????????????????????????????????????????????????
 * C/D/E. jidou/title_once/ ? DIVE! PL!N-bp4-026-L
 *
 *   ab#0 (自動) 「自分のメインフェイズにこのカードが控え室から手札に加えられた
 *          とき、自分の手札からカード名が「DIVE!」のライブカード1枚を表向きで
 *          ライブカード置き場に置いてもよい。そうした場合、次のライブカード
 *          セットフェイズで…上限が1枚減る。」
 *   ab#1 (自動) 「このカードが表向きでライブカード置き場に置かれたとき、
 *          ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、
 *          ブレード2つを得る。」
 * ??????????????????????????????????????????????????????????????????????? */

/* Counts the SelectCard prompts ab#0 raises (its 「DIVE!」 pick) while
 * answering everything else with index 0. */
static int drain_counting_card_choices(TestGame *tg, int *extra_selects)
{
    int placements = 0;
    int guard = 0;
    while (pending(tg) && guard++ < 24) {
        if (strcmp(pending_kind(tg), "SelectCard") == 0) placements++;
        else if (extra_selects) (*extra_selects)++;
        answer_first(tg);
    }
    drain_accept_optionals(tg);
    return placements;
}

static void test_dive_ab0_placement_reduces_live_card_set_limit(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive       = test_id(&game, DIVE);
    int niji       = test_id(&game, NIJI_MEMBER);
    int other_live = test_id(&game, OTHER_LIVE);
    CHECK(rb_card_no_eq(dive, DIVE), "the live card is the PL!N-bp4-026-L (DIVE!) print");
    CHECK(rb_card_no_eq(other_live, OTHER_LIVE),
          "PL!-sd1-020-SD is a DIFFERENT live card (a decoy in hand)");
    CHECK(rb_card_no_eq(niji, NIJI_MEMBER), "the 虹ヶ咲 member is the PL!N-PR-003-PR print");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = niji;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    /* 3 decoy live cards + DIVE! already retrieved to hand. */
    test_add_to_hand(&game, other_live);
    test_add_to_hand(&game, other_live);
    test_add_to_hand(&game, test_new_id(&game, OTHER_LIVE));
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    CHECK_EQ(game.state.live_set_limit_reduction[0], 0, "precondition: no limit reduction yet");
    tas_full(&game, 0);
    CHECK(pending(&game), "ab#0 must prompt to place DIVE! in the live card zone");
    drain_accept_optionals(&game);

    CHECK(live_has(&game, 0, dive), "DIVE! ends up in the live card zone");
    CHECK_EQ(game.state.live_set_limit_reduction[0], 1,
             "『上限が1枚減る』 ? the live_card_set_limit_reduction must be 1");
    CHECK_EQ(3 - game.state.live_set_limit_reduction[0], 2,
             "so the LiveCardSet phase limit computes to 3 - 1 = 2");
}

static void test_dive_ab0_does_not_fire_outside_main_phase_phase_gate(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = test_id(&game, NIJI_MEMBER);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_ACTIVE;   /* NOT the main phase */
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    drain_decline(&game);

    CHECK(!live_has(&game, 0, dive),
          "『自分のメインフェイズに』 ? ab#0 must not place DIVE! outside the main phase");
}

static void test_dive_only_moved_copy_triggers_static_copy_does_not(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_moved  = test_id(&game, DIVE);
    int dive_static = test_new_id(&game, DIVE);
    CHECK(dive_moved != dive_static, "the two DIVE! copies are distinct pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = test_id(&game, NIJI_MEMBER);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, dive_static);      /* already in hand, never moved */
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, dive_moved);       /* the moved copy */
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    int only[1] = { dive_moved };
    set_recently_moved_n(&game, only, 1);

    int placements = drain_counting_card_choices(&game, NULL);

    CHECK_EQ(placements, 1,
             "only the MOVED DIVE! may trigger ab#0's placement prompt; the static copy must not");
    CHECK(live_has(&game, 0, dive_moved), "the moved copy is the one placed");
    CHECK(!live_has(&game, 0, dive_static), "the copy that never moved must stay in hand");
    CHECK(hand_has(&game, 0, dive_static), "…i.e. it is still in hand");
}

static void test_dive_two_static_one_moved_only_one_trigger(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_moved    = test_id(&game, DIVE);
    int dive_static_a = test_new_id(&game, DIVE);
    int dive_static_b = test_new_id(&game, DIVE);
    CHECK(dive_moved != dive_static_a && dive_moved != dive_static_b &&
          dive_static_a != dive_static_b, "three DISTINCT DIVE! pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = test_id(&game, NIJI_MEMBER);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, dive_static_a);
    test_add_to_hand(&game, dive_static_b);
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, dive_moved);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    int only[1] = { dive_moved };
    set_recently_moved_n(&game, only, 1);

    int placements = drain_counting_card_choices(&game, NULL);

    CHECK_EQ(placements, 1,
             "2 static DIVE! in hand + 1 moved: ab#0 must fire exactly once");
    CHECK(!live_has(&game, 0, dive_static_a) && !live_has(&game, 0, dive_static_b),
          "neither static copy may be placed");
}

static void test_dive_two_copies_real_movement_event_only_moved_places(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_moved  = test_id(&game, DIVE);
    int dive_static = test_new_id(&game, DIVE);
    CHECK(rb_card_no_eq(dive_moved, DIVE) && rb_card_no_eq(dive_static, DIVE),
          "both fixtures are the PL!N-bp4-026-L print");
    CHECK(dive_moved != dive_static, "and they are DISTINCT pool slots");

    clear_stage(&game, 0);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, dive_moved);
    test_add_to_hand(&game, dive_static);
    test_add_to_discard(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    /* Real movement event: ONLY dive_moved comes discard -> hand this batch. */
    push_movement(&game, dive_moved, RB_ZONEID_WAITROOM, RB_ZONEID_HAND, 0, 1);
    set_recently_moved(&game, dive_moved);

    int placements = drain_counting_card_choices(&game, NULL);

    CHECK_EQ(placements, 1, "exactly one placement selection (the moved copy)");
    CHECK(live_has(&game, 0, dive_moved) != live_has(&game, 0, dive_static),
          "exactly one copy may end up in the live card zone");
    CHECK(!live_has(&game, 0, dive_static), "the copy that never moved stays in hand");
}

static void test_dive_natural_draw_from_deck_does_not_arm_ab0(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_p2 = test_new_id(&game, DIVE);
    CHECK(rb_card_no_eq(dive_p2, DIVE), "P2's DIVE! fixture is the PL!N-bp4-026-L print");

    fill_decks(&game, 10);
    test_insert_deck_top(&game, 1, dive_p2);
    game.state.p[1].hand.n = 0;
    game.state.p[1].live.n = 0;
    /* test_game_new already sits at P1's Main with first_attacker=0 /
       second_attacker=1, so four passes walk Main -> Active -> Energy ->
       Draw -> Main and the Draw->Main transition performs the real deck draw
       for the ACTIVE player, which is P2 by then. */
    game.state.phase = RB_PHASE_MAIN;

    for (int i = 0; i < 4; i++) {
        DIAG("pass %d: phase=%s active=%d p2hand=%d", i, rb_phase_name(game.state.phase),
             game.state.active, game.state.p[1].hand.n);
        test_pass(&game);
        assertions++;
        if (pending(&game)) {
            fprintf(stderr, "FAIL: a prompt appeared during phase progression ? "
                            "DIVE! must not arm off a deck draw\n");
            failures++;
        }
    }

    CHECK(hand_has(&game, 1, dive_p2), "P2 drew DIVE! from the deck");
    CHECK(!live_has(&game, 1, dive_p2),
          "『控え室から手札に加えられたとき』 ? a deck draw is not that change, so ab#0 must not arm");
}

static void test_dive_ab1_no_blade_when_statically_in_live_zone(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    int niji = test_id(&game, NIJI_MEMBER);

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = niji;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive);           /* static presence, no movement */
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.n_recently_moved = 0;          /* clear_recently_moved_batch */
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    CHECK_EQ(blade_mod(&game, niji), 0,
             "the location condition carries movement:\"moved\" ? a static live-zone "
             "presence must not grant blade");
}

static void test_dive_ab1_rescan_after_flags_cleared_does_not_double_grant(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    int niji = test_new_id(&game, NIJI_MEMBER);

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = niji;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive);
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    drain_accept_optionals(&game);
    CHECK_EQ(blade_mod(&game, niji), 2, "the first grant gives exactly blade+2");

    /* Movement flags consumed/cleared ? the rescan must be silent. */
    game.state.n_recently_moved = 0;
    game.state.n_batch_movements = 0;
    tas_full(&game, 0);
    drain_accept_optionals(&game);

    CHECK_EQ(blade_mod(&game, niji), 2,
             "a rescan after the movement flags are cleared must NOT double the grant");
}

static void test_dive_both_retrieval_arms_own_copy_not_opponents(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_p1 = test_id(&game, DIVE);
    int dive_p2 = test_new_id(&game, DIVE);
    int setsuna = test_id(&game, SETSUNA_BOTH);
    CHECK(rb_card_no_eq(setsuna, SETSUNA_BOTH),
          "the retriever is the PL!N-bp4-007-R＋ print (FULLWIDTH plus, a different card no)");
    CHECK(dive_p1 != dive_p2, "P1's and P2's DIVE! are distinct pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].hand.n = 0;
    game.state.p[1].hand.n = 0;
    game.state.p[0].live.n = 0;
    game.state.p[1].live.n = 0;
    test_add_to_hand(&game, setsuna);
    test_add_to_discard(&game, dive_p1);
    add_to_waitroom_for(&game, 1, dive_p2);
    fill_decks(&game, 10);
    test_give_energy(&game, 15);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    test_play_to_stage(&game, setsuna, 1);
    drain_accept_optionals(&game);

    DIAG("p1 hand=%d live=%d, p2 hand=%d live=%d",
         game.state.p[0].hand.n, game.state.p[0].live.n,
         game.state.p[1].hand.n, game.state.p[1].live.n);
    CHECK(live_has(&game, 0, dive_p1),
          "P1's DIVE!: added to hand by P1's OWN effect during P1's main phase -> ab#0 arms");
    CHECK(hand_has(&game, 1, dive_p2),
          "P2's DIVE!: retrieved into P2's hand by the cross-line retrieval "
          "「自分と相手はそれぞれ…」");
    CHECK(!live_has(&game, 1, dive_p2),
          "P2's DIVE! must NOT auto-place ? moved by the OPPONENT's effect during P1's "
          "main phase, so ab#0's phase_target=self gate refuses it");
}

static void test_dive_wrong_target_retrieval_does_not_arm_ab0(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive       = test_id(&game, DIVE);
    int other_live = test_new_id(&game, OTHER_NIJI_LIVE);
    int setsuna    = test_id(&game, SETSUNA_ONE);
    CHECK(rb_card_no_eq(other_live, OTHER_NIJI_LIVE),
          "the decoy is the PL!N-bp4-025-L (VIVID WORLD) print, a DIFFERENT card no");
    CHECK(dive != other_live, "and a distinct pool slot");

    clear_stage(&game, 0);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, setsuna);
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_discard(&game, dive);
    test_add_to_discard(&game, other_live);
    fill_decks(&game, 10);
    test_give_energy(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    test_play_to_stage(&game, setsuna, 1);
    /* Optional 1-card discard cost. */
    CHECK(pending(&game), "the optional discard cost is offered");
    if (pending(&game)) {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        CHECK(c && strstr(c->zone, "hand") != NULL, "the cost is paid from hand");
        answer_first(&game);
    }
    /* Retrieval: two candidates in the waitroom. Pick the OTHER card. */
    CHECK(pending(&game), "the retrieval selection is offered");
    {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        CHECK(c && c->kind == RB_CHOICE_SELECT_CARD, "expected a SelectCard for the retrieval");
        DIAG("retrieval candidates = %d", c ? c->n_filtered_indices : -1);
        CHECK(c && c->n_filtered_indices == 2, "two 虹ヶ咲 live cards in the waitroom");
    }
    answer(&game, 1);
    drain_accept_optionals(&game);

    CHECK(!pending(&game), "nothing else may be pending after retrieving the non-DIVE card");
    CHECK(!live_has(&game, 0, dive),
          "DIVE! stayed in the waitroom ? ab#0 only arms for the card THAT was retrieved");
    CHECK(wait_has(&game, 0, dive), "DIVE! must still be in the waitroom");
}

static void test_dive_skip_placement_then_new_retrieval_still_triggers(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_a = test_id(&game, DIVE);
    int dive_b = test_new_id(&game, DIVE);
    CHECK(dive_a != dive_b, "two DISTINCT DIVE! pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = test_id(&game, NIJI_MEMBER);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    /* First retrieval: ab#0 fires, placement declined. */
    test_add_to_hand(&game, dive_a);
    set_recently_moved(&game, dive_a);
    tas_full(&game, 0);
    CHECK(pending(&game), "ab#0 fires for the first retrieval");
    drain_decline(&game);
    CHECK(!live_has(&game, 0, dive_a), "declining leaves DIVE! out of the live zone");

    /* Second retrieval, next turn. */
    game.state.turn += 1;
    test_add_to_hand(&game, dive_b);
    set_recently_moved(&game, dive_b);
    tas_full(&game, 0);
    CHECK(pending(&game), "ab#0 is not permanently blocked: it fires for the new retrieval");
    {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        DIAG("ab#0 pick candidates = %d, name filter chars = %d",
             c ? c->n_filtered_indices : -1, c ? c->n_characters : -1);
        CHECK(c && c->n_filtered_indices == 2, "both DIVE! copies are offered; pick the new one");
    }
    answer(&game, 1);
    drain_accept_optionals(&game);

    CHECK(live_has(&game, 0, dive_b), "only the CHOSEN copy is placed");
    CHECK(!live_has(&game, 0, dive_a), "the declined copy stays out of the live zone");
}

static void test_dive_ab1_fires_on_direct_placement(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    int niji = test_id(&game, NIJI_MEMBER);

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = niji;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive);
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    CHECK_EQ(blade_mod(&game, niji), 2, "ab#1 alone: DIVE! in the live zone grants blade+2");
}

static void test_dive_ab1_two_niji_members_only_one_gets_blade(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive   = test_id(&game, DIVE);
    int niji_a = test_id(&game, NIJI_MEMBER);
    int niji_b = test_id(&game, NIJI_MEMBER2);
    CHECK(rb_card_no_eq(niji_b, NIJI_MEMBER2),
          "the second 虹ヶ咲 member is the PL!N-sd1-001-SD print");
    CHECK(niji_a != niji_b, "and a distinct pool slot from PL!N-PR-003-PR");
    CHECK(rb_card_matches_group_str(niji_b, "虹ヶ咲"),
          "precondition: 上原歩夢 (other print) is 『虹ヶ咲』");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = niji_a;
    game.state.p[0].stage[2] = niji_b;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive);
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    int mod_a = blade_mod(&game, niji_a);
    int mod_b = blade_mod(&game, niji_b);
    DIAG("blade a=%d b=%d", mod_a, mod_b);
    CHECK(mod_a >= 2 || mod_b >= 2, "at least one 虹ヶ咲 member gains blade+2");
    CHECK(!(mod_a > 0 && mod_b > 0), "『メンバー1人』 ? only ONE member may be buffed");
}

static void test_dive_two_in_live_zone_two_blade_grants(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_a = test_id(&game, DIVE);
    int dive_b = test_new_id(&game, DIVE);
    int niji_a = test_id(&game, NIJI_MEMBER);
    int niji_b = test_id(&game, NIJI_MEMBER2);
    CHECK(dive_a != dive_b, "two DISTINCT DIVE! pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = niji_a;
    game.state.p[0].stage[2] = niji_b;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive_a);
    test_add_to_live_for(&game, 0, dive_b);
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    int both[2] = { dive_a, dive_b };
    set_recently_moved_n(&game, both, 2);

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    int total = blade_mod(&game, niji_a) + blade_mod(&game, niji_b);
    DIAG("total blade across both members = %d", total);
    CHECK(total >= 2, "two DIVE! in the live zone grant at least blade+2 in total");
}

static void test_dive_ab1_no_niji_target_no_crash(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive   = test_id(&game, DIVE);
    int filler = test_id(&game, FILLER);
    CHECK(!rb_card_matches_group_str(filler, "虹ヶ咲"),
          "precondition: the staged filler is NOT a 『虹ヶ咲』 member");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[0].stage[1] = filler;
    game.state.p[0].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_live(&game, dive);
    test_add_to_hand(&game, filler);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    CHECK(!pending(&game), "no 虹ヶ咲 target means no selection prompt");
    CHECK(hand_has(&game, 0, filler), "nothing may be added to hand without a valid target");
}

static void test_dive_ab0_declined_ab1_not_fired(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    int niji = test_id(&game, NIJI_MEMBER);

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = niji;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    set_recently_moved(&game, dive);

    tas_full(&game, 0);
    CHECK(pending(&game), "ab#0's optional placement is offered");
    drain_decline(&game);

    CHECK(!live_has(&game, 0, dive), "DIVE! must NOT be in the live card zone when declined");
    CHECK(!pending(&game), "ab#1 must not fire when ab#0's placement was declined");
    CHECK_EQ(blade_mod(&game, niji), 0, "and no blade may be granted");
}

static void test_dive_ab0_triggers_for_p2_during_p2_main_phase(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = test_id(&game, NIJI_MEMBER);
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[1].hand.n = 0;
    game.state.p[0].live.n = 0;
    game.state.p[1].live.n = 0;
    test_add_to_hand_for(&game, 1, test_id(&game, FILLER));
    test_add_to_hand_for(&game, 1, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 1;    /* Rust SecondAttackerNormal == P2 is the active player */

    set_recently_moved(&game, dive);
    tas_full(&game, 1);
    drain_accept_optionals(&game);

    CHECK(live_has(&game, 1, dive),
          "ab#0's gate is phase_target=self, so P2's own copy DOES arm in P2's main phase");
}

static void test_dive_ab0_no_trigger_for_p2_during_p1_main_phase(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[0] = RB_EMPTY_SLOT;
    game.state.p[1].stage[1] = test_id(&game, NIJI_MEMBER);
    game.state.p[1].stage[2] = RB_EMPTY_SLOT;
    game.state.p[0].hand.n = 0;
    game.state.p[1].hand.n = 0;
    game.state.p[0].live.n = 0;
    game.state.p[1].live.n = 0;
    test_add_to_hand_for(&game, 1, test_id(&game, FILLER));
    test_add_to_hand_for(&game, 1, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;    /* P1 is the active player */

    set_recently_moved(&game, dive);
    tas_full(&game, 1);
    drain_decline(&game);

    CHECK(!live_has(&game, 1, dive),
          "『自分のメインフェイズに』 ? P2's DIVE! must not fire during P1's main phase");
}

static void test_dive_ab0_no_trigger_from_static_hand(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive = test_id(&game, DIVE);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, dive);
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    game.state.n_recently_moved = 0;   /* nothing moved */

    tas_full(&game, 0);
    drain_decline(&game);

    CHECK(!live_has(&game, 0, dive),
          "『控え室から手札に加えられたとき』 ? a static hand card does not arm ab#0");
}

static void test_dive_ab0_places_dive_ab1_grants_blade_setsuna_chain(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive    = test_id(&game, DIVE);
    int setsuna = test_id(&game, SETSUNA_ONE);
    int niji    = test_id(&game, NIJI_MEMBER);
    CHECK(rb_card_no_eq(setsuna, SETSUNA_ONE),
          "the retriever is the PL!N-bp5-019-N (優木せつ菜) print");
    CHECK(!wait_has(&game, 0, dive), "precondition: DIVE! starts in exactly ONE zone (not hand)");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = niji;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_discard(&game, dive);
    test_add_to_hand(&game, setsuna);
    test_add_to_hand(&game, test_id(&game, FILLER));
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    test_give_energy(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    test_play_to_stage(&game, setsuna, 1);
    CHECK(pending(&game), "the optional 1-card discard cost is offered");
    if (pending(&game)) answer_first(&game);
    drain_accept_optionals(&game);

    DIAG("after the retrieval: dive in hand=%d, live=%d, queue entries=%d",
         hand_has(&game, 0, dive), live_has(&game, 0, dive), game.state.queue.n_entries);
    /* ENGINE-GAP EVIDENCE: nothing re-scanned auto abilities after the
       move_cards completed, so DIVE! ab#0 never saw the discard->hand
       transition. Scanning by hand right here proves the entry was there all
       along. */
    {
        int was_in_hand = hand_has(&game, 0, dive);
        set_recently_moved(&game, dive);
        tas_full(&game, 0);
        drain_accept_optionals(&game);
        DIAG("after an explicit rescan: dive in hand=%d live=%d (was in hand=%d)",
             hand_has(&game, 0, dive), live_has(&game, 0, dive), was_in_hand);
    }
    CHECK(live_has(&game, 0, dive), "ab#0 placed DIVE! in the live card zone");
    CHECK(!hand_has(&game, 0, dive), "and it is no longer in hand");
    DIAG("blade on the 虹ヶ咲 member = %d", blade_mod(&game, niji));
    CHECK(blade_mod(&game, niji) >= 2, "ab#1 granted blade+2 to the 虹ヶ咲 member");
}

static void test_dive_two_retrieved_chain_still_works(void)
{
    static TestGame game;
    test_game_new(&game);

    int dive_a = test_id(&game, DIVE);
    int dive_b = test_new_id(&game, DIVE);
    int niji   = test_id(&game, NIJI_MEMBER);
    CHECK(dive_a != dive_b, "the two DIVE! copies are DISTINCT pool slots");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    game.state.p[0].stage[0] = niji;
    game.state.p[0].hand.n = 0;
    game.state.p[0].live.n = 0;
    test_add_to_hand(&game, dive_a);
    test_add_to_hand(&game, dive_b);
    test_add_to_hand(&game, test_id(&game, FILLER));
    fill_decks(&game, 10);
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    int both[2] = { dive_a, dive_b };
    set_recently_moved_n(&game, both, 2);

    tas_full(&game, 0);
    drain_accept_optionals(&game);

    CHECK(live_has(&game, 0, dive_a) || live_has(&game, 0, dive_b),
          "at least one DIVE! should be in the live card zone after ab#0");
    DIAG("blade on the 虹ヶ咲 member = %d", blade_mod(&game, niji));
    CHECK(blade_mod(&game, niji) >= 2, "ab#1 grants blade+2 even when both copies move in one batch");
}

/* ???????????????????????????????????????????????????????????????????????
 * F. jidou/leaves_stage/live_success_heart05_threshold_and_no_surplus_score_
 *    plus2_test.rs ? Strawberry Trapper PL!S-pb1-021-L
 *
 * 「自分のステージにいる『Aqours』のメンバーが持つハートに、heart05が合計4個以上
 *   あり、このターン、相手が余剰のハートを持たずにライブを成功させていた場合、
 *   このカードのスコアを＋２する。」
 * The +2 is a live-scoped score modifier, so the Rust tests read it out of
 * performance_snapshots[0] (the mods are cleared by live end).
 *
 * RUST FIXTURE BUG carried into this port (evidence in cards/cards.json):
 * the Rust test comments claim 桜内梨子 prints "heart05=2" and that two of them
 * therefore total the 合計4個以上 threshold. Neither is true:
 *   PL!S-bp2-002-R  (RIKO_A)  has NO printed heart at all
 *                       (no need_heart / special_heart / blade_heart key);
 *   PL!S-sd1-011-SD (RIKO_B)  has blade_heart {"b_heart05": 1} ? ONE icon.
 * Every 『Aqours』 member in the database prints AT MOST ONE heart, so
 * 合計4個以上 is unreachable from printed hearts alone and the threshold can
 * only be met through heart modifiers. F1/F2 therefore grant explicit heart05
 * modifiers (documented below) so the test actually distinguishes gated from
 * ungated; F4/F5 are added probes for what the C engine's aggregate really
 * measures.
 * ??????????????????????????????????????????????????????????????????????? */

typedef struct { int member; int heart_color; int modifier; } AqSpec;

static int snapshot_score_detail(const TestGame *tg, int card, int *out_base)
{
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &tg->state.snapshots[i];
        for (int j = 0; j < s->n_lives; j++) {
            if (s->lives[j] == card) {
                if (out_base) *out_base = -1;
                return s->live_score_detail[j];
            }
        }
    }
    return -1;
}

static int trapper_base_score(void)
{
    static TestGame probe;
    static int initialised;
    int base = -1;
    if (!initialised) { test_game_new(&probe); initialised = 1; }
    int trapper = test_id(&probe, TRAPPER);
    Card c;
    if (trapper >= 0 && rb_decode_card_by_index((uint32_t)trapper, &c)) {
        base = (int)c.score;
        rb_free_card(&c);
    }
    return base;
}

/* Board + live-card + phase progression, shared by every F case. */
static void strawberry_board(TestGame *tg, const AqSpec *specs, int n_specs,
                             int seed_p2_no_excess_success)
{
    int trapper = test_id(tg, TRAPPER);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    for (int i = 0; i < n_specs; i++) {
        tg->state.p[0].stage[i] = specs[i].member;
        if (specs[i].modifier)
            rb_mods_add_heart(&tg->state.mods, specs[i].member,
                             specs[i].heart_color, specs[i].modifier);
    }
    for (int i = n_specs; i < RB_STAGE_SIZE; i++) tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
    tg->state.p[0].hand.n = 0;
    tg->state.p[1].hand.n = 0;
    tg->state.p[0].live.n = 0;
    test_add_to_hand(tg, trapper);
    fill_decks(tg, 20);
    tg->state.phase = RB_PHASE_MAIN;
    tg->state.active = 0;
    (void)seed_p2_no_excess_success;
}

/* advance_to_live_card_set_p1 + set_live_card + advance_to_live_start + the
 * live itself. `seed_p2` re-seeds the opponent's no-excess success right
 * before each pass, which is the C mirror of the Rust test's pre-seeded
 * `p2_live_success_this_turn` (the C engine publishes those flags only from
 * its own per-seat live evaluation, and P2 has not performed one yet). */
static void run_trapper_live(TestGame *tg, int seed_p2)
{
    for (int i = 0; i < 5; i++) {          /* Main -> Active -> Energy -> Draw -> Main -> LiveSet */
        if (pending(tg)) drain_decline(tg);
        test_pass(tg);
    }
    if (seed_p2) {
        tg->state.live_success[1] = 1;
        tg->state.p2_live_success_no_excess = 1;
    }
    test_set_live_card(tg, 0, test_id(tg, TRAPPER));
    int guard = 0;
    while (guard++ < 30) {
        if (pending(tg)) { drain_decline(tg); continue; }
        if (tg->state.n_snapshots > 0) break;
        if (seed_p2) {
            tg->state.live_success[1] = 1;
            tg->state.p2_live_success_no_excess = 1;
        }
        test_pass(tg);
    }
    for (int i = 0; i < 3; i++) {
        if (pending(tg)) { drain_decline(tg); continue; }
        test_pass(tg);
    }
    drain_decline(tg);
}

static void check_trapper_score(TestGame *tg, int expected_bonus, const char *what)
{
    int trapper = test_id(tg, TRAPPER);
    int base = trapper_base_score();
    int detail = snapshot_score_detail(tg, trapper, NULL);
    DIAG("snapshot score detail = %d, base = %d, bonus = %d",
         detail, base, detail >= 0 ? detail - base : -999);
    CHECK(detail >= 0, what);
    CHECK_EQ(detail - base, expected_bonus, what);
}

static void test_strawberry_trapper_fixtures_are_the_printed_ones(void)
{
    static TestGame game;
    test_game_new(&game);
    int trapper = test_id(&game, TRAPPER);
    int a = test_id(&game, RIKO_A);
    int b = test_id(&game, RIKO_B);
    int h4a = test_id(&game, AQ_H4_A);
    int h4b = test_id(&game, AQ_H4_B);
    CHECK(rb_card_no_eq(trapper, TRAPPER),
          "the live card is the PL!S-pb1-021-L (Strawberry Trapper) print");
    CHECK(rb_card_no_eq(a, RIKO_A) && rb_card_no_eq(b, RIKO_B),
          "the Rust fixtures are the 桜内梨子 print (PL!S-bp2-002-R / PL!S-sd1-011-SD)");
    CHECK(rb_card_matches_group_str(a, "Aqours"),
          "precondition: 桜内梨子 really is an 『Aqours』 member (unit GuiltyKiss)");
    CHECK(rb_card_matches_group_str(h4a, "Aqours") && rb_card_matches_group_str(h4b, "Aqours"),
          "precondition: the heart04 probe members are 『Aqours』 too");
    dump_card_hearts(a, RIKO_A);
    dump_card_hearts(b, RIKO_B);
    dump_card_hearts(h4a, AQ_H4_A);
    dump_card_hearts(h4b, AQ_H4_B);
    DIAG("engine-visible heart05: RIKO_A=%d RIKO_B=%d | heart04: H4_A=%d H4_B=%d",
         printed_heart(a, 5), printed_heart(b, 5), printed_heart(h4a, 4), printed_heart(h4b, 4));
    /* The Rust fixture bug, measured rather than assumed. The Rust comments
       claim 桜内梨子 prints heart05=2 apiece, so two of them total the
       合計4個以上 threshold; cards/cards.json has neither. The numbers the C
       Card record actually carries are printed above. */
    CHECK(printed_heart(a, 5) + printed_heart(b, 5) != 4,
          "the Rust pair does NOT total exactly 4 heart05, contradicting the test's "
          "own 「合計4個以上」 precondition comment");
}

/* F1 ? the Rust positive case, with the heart05 threshold actually reachable. */
static void test_strawberry_trapper_conditions_met_score_plus_2(void)
{
    static TestGame game;
    test_game_new(&game);

    int h5a = test_id(&game, RIKO_B);     /* 桜内梨子, b_heart05 x1 */
    int h5b = test_new_id(&game, AQ_H5_B); /* 小原鞠莉, b_heart05 x1 */
    AqSpec spec[2] = { {h5a, 5, 2}, {h5b, 5, 1} };   /* (1+2)+(1+1) = 5 >= 4 */
    strawberry_board(&game, spec, 2, 1);

    run_trapper_live(&game, 1);
    CHECK_EQ(score_mod(&game, test_id(&game, TRAPPER)), 0,
             "the +2 is a live-scoped modifier and is cleared once the live ends");
    check_trapper_score(&game, 2,
                        "『heart05が合計4個以上あり、相手が余剰のハートを持たずに成功』 -> +2");
}

/* F2 ? the Rust negative case: below the heart05 threshold. */
static void test_strawberry_trapper_insufficient_heart05_no_score(void)
{
    static TestGame game;
    test_game_new(&game);

    int h5a = test_id(&game, RIKO_B);
    AqSpec spec[1] = { {h5a, 5, 0} };    /* 1 heart05, below 合計4個以上 */
    strawberry_board(&game, spec, 1, 1);
    /* A single unmodified member must stay below the threshold. The engine's
       own aggregate is what matters here, so the number is quoted as a
       diagnostic and the board is only asserted to be under 4 heart05. */
    DIAG("single unmodified Aqours member: engine-visible heart05 = %d",
         printed_heart(h5a, 5));
    CHECK(printed_heart(h5a, 5) < 4,
          "precondition: one unmodified 桜内梨子 is below the 合計4個以上 threshold");

    run_trapper_live(&game, 1);
    CHECK_EQ(score_mod(&game, test_id(&game, TRAPPER)), 0,
             "『heart05が合計4個以上』 ? below the threshold no score bonus is granted");
    check_trapper_score(&game, 0, "…and the snapshot score carries no bonus");
}

/* F3 ? the Rust negative case: the opponent did not succeed. */
static void test_strawberry_trapper_no_opponent_success_no_score(void)
{
    static TestGame game;
    test_game_new(&game);

    int h5a = test_id(&game, RIKO_B);
    int h5b = test_new_id(&game, AQ_H5_B);
    AqSpec spec[2] = { {h5a, 5, 2}, {h5b, 5, 1} };
    strawberry_board(&game, spec, 2, 0);
    CHECK_EQ(game.state.live_success[1], 0,
             "precondition: the opponent did NOT succeed without excess heart");

    run_trapper_live(&game, 0);
    CHECK_EQ(score_mod(&game, test_id(&game, TRAPPER)), 0,
             "『相手が余剰のハートを持たずにライブを成功させていた場合』 ? no opponent "
             "success means no score bonus");
    check_trapper_score(&game, 0, "…and the snapshot score carries no bonus");
}

/* F4 ? ENGINE PROBE (not a Rust test). The printed text names heart05 only.
 * Board: two 『Aqours』 members with 5 heart04 between them and ZERO heart05.
 * If the +2 is still granted, the aggregate group condition ignores the
 * `heart_colors: ["heart05"]` field. */
static void test_strawberry_trapper_heart04_board_must_not_score(void)
{
    static TestGame game;
    test_game_new(&game);

    int h4a = test_id(&game, AQ_H4_A);
    int h4b = test_new_id(&game, AQ_H4_B);
    AqSpec spec[2] = { {h4a, 4, 2}, {h4b, 4, 1} };   /* heart04 only */
    strawberry_board(&game, spec, 2, 1);
    DIAG("heart05 probes on the heart04 board: %d + %d",
         printed_heart(h4a, 5), printed_heart(h4b, 5));

    run_trapper_live(&game, 1);
    check_trapper_score(&game, 0,
                        "『ハートにheart05が合計4個以上』 — a heart04-only board must NOT score");
}

/* F5 ? ENGINE PROBE (not a Rust test). A member with NO printed heart
 * (PL!S-bp2-002-R) stands next to a member holding exactly 4 heart05. The
 * board really has 合計4個以上 heart05, so the +2 must be granted. If it is
 * not, a heartless member DECREMENTS the aggregate instead of contributing 0. */
static void test_strawberry_trapper_heartless_member_must_not_reduce_total(void)
{
    static TestGame game;
    test_game_new(&game);

    int heartless = test_id(&game, RIKO_A);
    int h5        = test_id(&game, RIKO_B);
    AqSpec spec[2] = { {heartless, 5, 0}, {h5, 5, 3} };
    strawberry_board(&game, spec, 2, 1);
    DIAG("heartless member heart05 = %d, boosted member heart05 = %d (+3 modifier)",
         printed_heart(heartless, 5), printed_heart(h5, 5));

    run_trapper_live(&game, 1);
    check_trapper_score(&game, 2,
                        "a heartless 『Aqours』 member must contribute 0, not subtract, "
                        "from the 合計4個以上 total");
}

/* ??????????????????????????????????????????????????????????????????????? */

static void on_fault(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        /* No fork available: run in-process so the suite still reports. */
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) { n_failed++; printf("FAILED   %s\n", name); }
        else { n_ok++; printf("ok       %s  (%d assertions)\n", name, assertions - a0); }
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, g0 = harness_gaps;
        current_test = name;
        fn();
        printf("        %-64s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
               name, (int)assertions - a0, failures - f0, harness_gaps - g0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- engine fault, signal %d\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
    } else {
        n_failed++;
        printf("FAILED   %s\n", name);
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fault);
    signal(SIGBUS,  on_fault);
    signal(SIGABRT, on_fault);
    if (getenv("RB_ABILITY_DEBUG")) rb_ability_debug_set(1);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- A. state_watch / live_phase_group_wait_discard_reactivate ---\n");
    run("A_fixture_identity",                  test_shioriko_fixtures_are_the_printed_ones);
    run("A_live_phase_wait_accept_reactivates",test_live_phase_group_member_wait_optional_discard_removes_wait);
    run("A_live_phase_wait_decline_stays",     test_live_phase_group_wait_decline_discard_stays_wait);
    run("A_non_live_phase_no_fire",            test_non_live_phase_wait_does_not_fire);
    run("A_non_niji_wait_no_fire",             test_non_nijigasaki_wait_does_not_fire);
    run("A_empty_hand_auto_skips",             test_empty_hand_auto_skips_no_reactivate);
    run("A_second_wait_no_refire",             test_second_wait_same_live_phase_does_not_refire);
    run("A_self_wait_fires",                   test_self_wait_fires_for_shioriko);

    printf("--- B. state_watch / own_effect_wait_cheap_opponent_draw_one_q177 ---\n");
    run("B_fixture_identity",                  test_maki_fixtures_are_the_printed_ones);
    run("B_debut_cheap_opp_wait_draws_1",      test_own_effect_wait_of_cheap_opponent_after_debut_draws_one_q177);
    run("B_declined_cost_no_wait_no_draw",     test_declined_unit_wait_cost_leaves_opponent_active_and_no_draw);
    run("B_cost9_wait_does_not_draw",          test_actual_cost_nine_wait_does_not_trigger_maki_draw);
    run("B_empty_opp_stage_no_draw",           test_declined_cost_with_empty_opponent_stage_draws_nothing);
    run("B_own_effect_cheap_wait_draws",       test_own_effect_wait_of_cheap_opponent_draws);
    run("B_opponent_effect_wait_no_draw",      test_opponent_effect_wait_of_cheap_member_does_not_draw);
    run("B_opponent_debut_wait_no_draw",       test_opponent_debut_waited_member_does_not_draw);
    run("B_real_swap_own_effect_wait_draws",   test_own_effect_wait_chain_through_a_real_swap);

    printf("--- C/D/E. title_once / DIVE! ab#0 arming + ab#1 ---\n");
    run("CE_limit_reduction",                  test_dive_ab0_placement_reduces_live_card_set_limit);
    run("CE_no_fire_outside_main_phase",       test_dive_ab0_does_not_fire_outside_main_phase_phase_gate);
    run("CE_only_moved_copy_triggers",         test_dive_only_moved_copy_triggers_static_copy_does_not);
    run("CE_two_static_one_moved",             test_dive_two_static_one_moved_only_one_trigger);
    run("CE_real_movement_only_moved",         test_dive_two_copies_real_movement_event_only_moved_places);
    run("CE_natural_draw_does_not_arm",        test_dive_natural_draw_from_deck_does_not_arm_ab0);
    run("CE_ab1_static_live_no_blade",         test_dive_ab1_no_blade_when_statically_in_live_zone);
    run("CE_ab1_rescan_no_double",             test_dive_ab1_rescan_after_flags_cleared_does_not_double_grant);
    run("CE_both_retrieval_own_only",          test_dive_both_retrieval_arms_own_copy_not_opponents);
    run("CE_wrong_target_no_arm",              test_dive_wrong_target_retrieval_does_not_arm_ab0);
    run("CE_skip_then_new_retrieval",          test_dive_skip_placement_then_new_retrieval_still_triggers);
    run("CE_ab1_direct_placement",             test_dive_ab1_fires_on_direct_placement);
    run("CE_ab1_two_niji_one_blade",           test_dive_ab1_two_niji_members_only_one_gets_blade);
    run("CE_two_live_two_blade",               test_dive_two_in_live_zone_two_blade_grants);
    run("CE_ab1_no_niji_target",               test_dive_ab1_no_niji_target_no_crash);
    run("CE_ab0_declined_ab1_not_fired",       test_dive_ab0_declined_ab1_not_fired);
    run("CE_ab0_p2_own_main_phase",            test_dive_ab0_triggers_for_p2_during_p2_main_phase);
    run("CE_ab0_p2_during_p1_main_phase",      test_dive_ab0_no_trigger_for_p2_during_p1_main_phase);
    run("CE_ab0_no_trigger_static_hand",       test_dive_ab0_no_trigger_from_static_hand);
    run("CE_setsuna_debut_chain",              test_dive_ab0_places_dive_ab1_grants_blade_setsuna_chain);
    run("CE_two_retrieved_chain",              test_dive_two_retrieved_chain_still_works);

    printf("--- F. leaves_stage / live_success_heart05_threshold_and_no_surplus ---\n");
    run("F_fixture_identity",                  test_strawberry_trapper_fixtures_are_the_printed_ones);
    run("F_conditions_met_score_plus_2",       test_strawberry_trapper_conditions_met_score_plus_2);
    run("F_insufficient_heart05_no_score",     test_strawberry_trapper_insufficient_heart05_no_score);
    run("F_no_opponent_success_no_score",      test_strawberry_trapper_no_opponent_success_no_score);
    run("F_heart04_board_must_not_score",      test_strawberry_trapper_heart04_board_must_not_score);
    run("F_heartless_member_reduces_total",    test_strawberry_trapper_heartless_member_must_not_reduce_total);

    rb_unload();

    printf("\ntests passed          : %d\n", n_ok);
    printf("tests failed (parity) : %d\n", n_failed);
    printf("tests crashed (engine): %d\n", n_crashed);
    printf("see the per-test tally lines above for assertion / failure / harness-gap counts\n");
    if (n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL JIDOU-STATE PARITY CHECKS PASSED\n");
    return 0;
}

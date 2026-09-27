#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define HIMEKO "PL!HS-pb1-014-R"
#define MIRAKURA_RURINO "PL!HS-PR-029-PR"
#define NON_MIRAKURA_MEMBER "PL!-sd1-010-SD"
#define KOKO "PL!SP-sd2-002-P"

/* Push-movement-event choke point. Rust reaches it through
   GameState::push_movement_event (modifiers.rs:1481-1555), which records the
   event, flags the area move and ARMS the opponent-cause watchers in ONE call;
   since commit 219d1522 the C fold lives entirely inside
   rb_record_card_movement (src/core/modifiers.c:388-421), so move_and_trigger
   below drives the whole of it with a single call and must NOT repeat either
   half by hand. */
#define KOKO_SD2 "PL!SP-sd2-002-SD2"
#define NATSUME "PL!SP-pb1-020-N"
#define TOMARI "PL!SP-sd2-011-SD2"
#define KANON "PL!SP-sd2-012-SD2"
#define FUYUMARI "PL!SP-sd2-022-SD2"
#define CHISATO "PL!SP-pb1-006-R"

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_NE(actual, unexpected, message) do { \
    int actual_value = (actual); \
    int unexpected_value = (unexpected); \
    if (actual_value == unexpected_value) { \
        fprintf(stderr, "FAIL: %s (got %d)\n", message, actual_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ_STR(actual, expected, message) do { \
    const char *actual_value = (actual); \
    const char *expected_value = (expected); \
    if (strcmp(actual_value, expected_value) != 0) { \
        fprintf(stderr, "FAIL: %s (got %s expected %s)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static int heart06(TestGame *game, int card_id)
{
    return test_get_heart_modifier(game, card_id, RB_HEART_ORANGE);
}

static void fill_decks(TestGame *game, int filler)
{
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(game, 0, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static int stage_contains(const int stage[3], int card_id)
{
    return stage[0] == card_id || stage[1] == card_id || stage[2] == card_id;
}

static void himeko_debut_repositions_opponent_member_and_koko_responds(void)
{
    TestGame game;
    test_game_new(&game);
    int rurino = test_id(&game, MIRAKURA_RURINO);
    int himeko = test_id(&game, HIMEKO);
    int koko = test_id(&game, KOKO);
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = rurino;
    game.state.p[0].stage[1] = -1;
    game.state.p[0].stage[2] = -1;
    game.state.p[1].stage[0] = -1;
    game.state.p[1].stage[1] = -1;
    game.state.p[1].stage[2] = koko;
    test_add_to_hand(&game, himeko);
    fill_decks(&game, filler);
    test_give_energy(&game, 15);

    test_play_to_stage(&game, himeko, 1);

    CHECK(test_has_pending_choice(&game), "expected the opponent-member selection prompt");
    CHECK_EQ_STR(test_pending_choice_type(&game), "SelectTarget", "reposition target choice");
    int selected[] = {0};
    test_select_indices(&game, selected, 1);

    while (test_has_pending_choice(&game)) {
        test_select_indices(&game, NULL, 0);
    }

    CHECK_NE(game.state.p[1].stage[2], koko, "可可 must be repositioned away from P2-right");
    CHECK_EQ(game.state.p[1].stage[1], koko, "可可 should land on P2-center (姫芽's front)");
    CHECK(stage_contains(game.state.p[1].stage, koko), "position change keeps the member on the stage");
    CHECK_EQ(heart06(&game, koko), 1,
             "可可's area-move watcher must fire even though P1's effect moved her");
}

static void himeko_gate_blocked_no_reposition_no_koko_response(void)
{
    TestGame game;
    test_game_new(&game);
    int himeko = test_id(&game, HIMEKO);
    int koko = test_id(&game, KOKO);
    int outsider = test_id(&game, NON_MIRAKURA_MEMBER);
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = outsider;
    game.state.p[0].stage[1] = -1;
    game.state.p[0].stage[2] = -1;
    game.state.p[1].stage[0] = -1;
    game.state.p[1].stage[1] = -1;
    game.state.p[1].stage[2] = koko;
    test_add_to_hand(&game, himeko);
    fill_decks(&game, filler);
    test_give_energy(&game, 15);

    test_play_to_stage(&game, himeko, 1);
    while (test_has_pending_choice(&game)) {
        test_select_indices(&game, NULL, 0);
    }

    CHECK_EQ(game.state.p[1].stage[2], koko, "gate failed: 可可 must stay at P2-right");
    CHECK_EQ(heart06(&game, koko), 0,
             "no movement happened, so 可可's watcher must be silent");
}

/* ══════════════════════════════════════════════════════════════════════
   movement-event watcher arm — port of
   engine/tests/test_modules/rules/trigger_paths/cross_player_jidou_triggers_test.rs
   and the two lifecycle clauses in
   engine/tests/test_modules/jidou/movement/self_area_move_watch/
       area_move_grant_turn_limit_and_live_end_expiry_test.rs

   These drive the C analogue of Rust's `push_movement_event(card, "stage",
   "stage", Some(card), cause_player, true)` — the SINGLE choke point every
   movement funnels through — and then run p1's 自動 to completion. The 姫芽
   case above only proves the hook is wired at one hand-picked call site; these
   prove it is wired at the choke point itself.
   ══════════════════════════════════════════════════════════════════════ */

/* Mirror cross_player_jidou_triggers_test.rs::move_and_trigger. The final
   argument of Rust's push_movement_event is `effect_only` (true in BOTH the
   self-caused and the opponent-caused case) — the CAUSE PLAYER is what
   separates the two arms, not that flag.

   One rb_record_card_movement call is the whole event: RB_ZONEID_STAGE is 0,
   so STAGE->STAGE is what puts the move on the area-move arm
   (modifiers.c:417-420), and that arm is what sets
   position_change_occurred_this_turn and arms the opponent-cause watchers
   (modifiers.rs:1526-1552). Doing either of those by hand here — as this
   helper used to — double-fires every opponent-caused watcher. */
static void move_and_trigger(TestGame *game, int moved, int cause_player)
{
    rb_record_card_movement(&game->state, moved,
                            RB_ZONEID_STAGE, RB_ZONEID_STAGE,
                            cause_player, 1);
    rb_trigger_auto_abilities_for_player(&game->state, 0);
    rb_process_pending_auto_abilities(&game->state);
    test_drain_auto_choices(game);
    while (test_has_pending_choice(game)) {
        test_select_indices(game, NULL, 0);
    }
    /* 「ライブ終了時まで」 grants are materialised into the modifier tables by
       the constant recalculation, exactly as the real arming path does
       (src/ability/choice.c:2204-2206 calls rb_recalc_constants right after
       the movement event). */
    rb_recalc_constants(&game->state);
}

static int heart_col(TestGame *game, int card_id, int color)
{
    return test_get_heart_modifier(game, card_id, color);
}

/* Heart indices mirror Rust's HeartColor ordinal: Heart06 = 6 = RB_HEART_ORANGE,
   Heart02 = 2 = RB_HEART_YELLOW, Heart03 = 3 = RB_HEART_GREEN. */
static void koko_self_effect_triggers(void)
{
    TestGame game;
    test_game_new(&game);
    int koko = test_id(&game, KOKO_SD2);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    CHECK(koko >= 0, "koko sd2 card id resolves");
    fill_decks(&game, filler);
    game.state.p[0].stage[1] = koko;

    move_and_trigger(&game, koko, 0);

    CHECK_EQ(heart_col(&game, koko, RB_HEART_ORANGE),
             1, "self-caused area move grants heart06");
}

static void koko_opponent_effect_triggers(void)
{
    TestGame game;
    test_game_new(&game);
    int koko = test_id(&game, KOKO_SD2);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[1] = koko;

    /* Same event, caused by the OPPOSING player. The watcher carries
       「(対戦相手のカードの効果でも発動する。)」 so it must still arm. */
    move_and_trigger(&game, koko, 1);

    CHECK_EQ(heart_col(&game, koko, RB_HEART_ORANGE), 1,
             "opponent-caused area move also grants heart06 (でも発動する)");
}

static void koko_turn1_blocks_second(void)
{
    TestGame game;
    test_game_new(&game);
    int koko = test_id(&game, KOKO_SD2);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[1] = koko;

    move_and_trigger(&game, koko, 0);
    CHECK_EQ(heart_col(&game, koko, RB_HEART_ORANGE), 1, "first move grants heart06");
    move_and_trigger(&game, koko, 0);
    CHECK_EQ(heart_col(&game, koko, RB_HEART_ORANGE), 1,
             "「{{ターン1回}}」 — a second area move in the same turn adds nothing");
}

static void natsume_opponent_draw(void)
{
    TestGame game;
    test_game_new(&game);
    int natsume = test_id(&game, NATSUME);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, filler);
    game.state.p[0].stage[0] = natsume;
    int before = game.state.p[0].hand.n;

    move_and_trigger(&game, natsume, 1);

    CHECK_EQ(game.state.p[0].hand.n, before + 1,
             "opponent-caused area move also draws 1 for 夏美");
}

/* The dedupe identity of an opponent-cause arm is (ability, moved card,
   movement sequence number) — GameState::opp_cause_key, abilities.rs:1037-1041
   — so the watcher arms ONCE PER MOVE: two distinct opponent-caused moves in
   one turn each arm it, while a rescan of the SAME move must not. 夏美 has no
   use limit, so her hand size reads the arm count out directly. */
static void opp_cause_arms_once_per_move(void)
{
    TestGame game;
    test_game_new(&game);
    int natsume = test_id(&game, NATSUME);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, filler);
    game.state.p[0].stage[0] = natsume;
    int before = game.state.p[0].hand.n;

    move_and_trigger(&game, natsume, 1);
    CHECK_EQ(game.state.p[0].hand.n, before + 1,
             "first opponent-caused area move arms 夏美 once");

    /* Rescan the SAME movement event. The hook owns foreign-cause firings, so
       the generic stage scan must skip the watcher (hooked_by_foreign_cause,
       abilities.rs:506-545) and the per-move key must block a re-arm. */
    rb_trigger_auto_abilities_for_player(&game.state, 0);
    rb_process_pending_auto_abilities(&game.state);
    test_drain_auto_choices(&game);
    CHECK_EQ(game.state.p[0].hand.n, before + 1,
             "a rescan of the same opponent-caused move must not re-arm");

    move_and_trigger(&game, natsume, 1);
    CHECK_EQ(game.state.p[0].hand.n, before + 2,
             "a second DISTINCT opponent-caused area move arms 夏美 again");
}

static void tomari_opponent_blade(void)
{
    TestGame game;
    test_game_new(&game);
    int tomari = test_id(&game, TOMARI);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[0] = tomari;

    move_and_trigger(&game, tomari, 1);

    CHECK_EQ(test_get_blade_modifier(&game, tomari), 1,
             "opponent-caused area move also grants a blade for 冬毬");
}

static void kanon_opponent_heart02(void)
{
    TestGame game;
    test_game_new(&game);
    int kanon = test_id(&game, KANON);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[0] = kanon;

    move_and_trigger(&game, kanon, 1);

    CHECK_EQ(heart_col(&game, kanon, RB_HEART_YELLOW), 1,
             "opponent-caused area move also grants heart02 for かのん");
}

static void fuyumari_opponent_heart03(void)
{
    TestGame game;
    test_game_new(&game);
    int fuyu = test_id(&game, FUYUMARI);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[0] = fuyu;

    move_and_trigger(&game, fuyu, 1);

    CHECK_EQ(heart_col(&game, fuyu, RB_HEART_GREEN), 1,
             "opponent-caused area move also grants heart03 for 冬毬");
}

static void chisato_opponent_blades(void)
{
    TestGame game;
    test_game_new(&game);
    int chisato = test_id(&game, CHISATO);
    int filler = test_id(&game, NON_MIRAKURA_MEMBER);
    fill_decks(&game, filler);
    game.state.p[0].stage[0] = chisato;

    move_and_trigger(&game, chisato, 1);

    CHECK_EQ(test_get_blade_modifier(&game, chisato), 2,
             "opponent-caused area move also grants 2 blades for 千砂都");
}

int main(void)
{
    if (getenv("XP_DBG")) rb_ability_debug_set(1);
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    himeko_debut_repositions_opponent_member_and_koko_responds();
    himeko_gate_blocked_no_reposition_no_koko_response();
    koko_self_effect_triggers();
    koko_opponent_effect_triggers();
    koko_turn1_blocks_second();
    natsume_opponent_draw();
    opp_cause_arms_once_per_move();
    tomari_opponent_blade();
    kanon_opponent_heart02();
    fuyumari_opponent_heart03();
    chisato_opponent_blades();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL CROSS PLAYER INTERACTION CHECKS PASSED\n");
    return 0;
}

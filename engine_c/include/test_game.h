#ifndef TEST_GAME_H
#define TEST_GAME_H
#include "rabuka.h"

/* Minimal TestGame shim mirroring engine/tests/helpers/mod.rs:361 TestGame
   for mass-porting Rust tests to C. Provides the same vocabulary:
   test_game_new, test_id, add_to_hand/stage/discard, give_energy,
   play_to_stage, activate_ability, recalc, board introspection.
   Uses the real card database (cards.bin) via rb_find_card_by_no.
   Pool semantics: test_id() returns the same template index for each call
   (no per-copy pool). Tests needing two distinct copies of same card_no
   should use distinct card_nos in C or check single-copy behaviour.
   Mirrors Rust helpers/mod.rs:407 TestGame::new in Main phase. */

typedef struct {
    GameState state;
} TestGame;

void test_game_new(TestGame *tg);
int  test_id(TestGame *tg, const char *card_no); /* card index or -1, like Rust i16 */
void test_add_to_hand(TestGame *tg, int card_id);
void test_add_to_discard(TestGame *tg, int card_id);
void test_add_to_stage(TestGame *tg, int area, int card_id); /* area 0=left 1=center 2=right */
void test_add_to_success(TestGame *tg, int card_id);
void test_add_to_live(TestGame *tg, int card_id);
void test_add_to_deck(TestGame *tg, int card_id);
void test_give_energy(TestGame *tg, int count);
int  test_play_to_stage(TestGame *tg, int card_id, int area);
int  test_try_play_to_stage(TestGame *tg, int card_id, int area); /* returns 1 on success */
void test_recalc(TestGame *tg);
void test_clear_mods_for_card(TestGame *tg, int card_id);
/* opponent-side helpers (mirror player2.* in Rust TestGame) */
void test_give_opp_energy(TestGame *tg, int count);
void test_set_opp_stage(TestGame *tg, int area, int card_id);
void test_add_to_opp_live(TestGame *tg, int card_id);
void test_add_to_opp_success(TestGame *tg, int card_id);
/* trigger / temporary-effect helpers */
void test_fire_debut(TestGame *tg, int card_id);
void test_expire_effects(TestGame *tg);
const char *test_card_name(int card_id);

/* board helpers for assertions */
int  test_stage_has(TestGame *tg, int area, int card_id);
int  test_hand_has(TestGame *tg, int card_id);
int  test_success_count(TestGame *tg);
void test_print_board(TestGame *tg);

#endif

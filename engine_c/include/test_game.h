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
void test_place_under(TestGame *tg, int pl, int area, int card_id); /* tuck card under member at area */
void test_add_to_success(TestGame *tg, int card_id);
void test_add_to_live(TestGame *tg, int card_id);
void test_add_to_deck(TestGame *tg, int card_id);
void test_add_to_deck_pl(TestGame *tg, int pl, int card_id);
/* Prepend card to the top of player pl's deck (Rust main_deck.cards.insert(0, x)). */
void test_insert_deck_top(TestGame *tg, int pl, int card_id);
void test_add_to_energy(TestGame *tg, int pl, int card_id);
void test_add_to_energy_deck(TestGame *tg, int pl, int card_id);
void test_set_energy_active(TestGame *tg, int pl, int n);
void test_add_to_revealed(TestGame *tg, int card_id);
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
/* activate an ability by card id (finds it in hand) — mirrors
   ActionType::ActivateAbility via TurnEngine::execute_main_phase_action. */
int  test_activate_ability(TestGame *tg, int card_id);
/* Drain auto-ability (SelectAutoAbility) pending choices — mirrors
   TestGame::drain_auto_ability_choices (answers with proceed / empty). */
void test_drain_auto_choices(TestGame *tg);
/* Spend N active energy (energy_zone.sub_active) — saturating at 0. */
void test_spend_energy(TestGame *tg, int n);
/* choice / pending-choice shims (mirror helpers/mod.rs has_pending_choice /
   select_indices / select_option / pending_choice_count) */
int  test_has_pending_choice(TestGame *tg);
int  test_pending_choice_count(TestGame *tg);
/* live shim (mirror helpers/mod.rs set_live_card) */
void test_set_live_card(TestGame *tg, int zone, int card_id);
const char *test_card_name(int card_id);

/* board helpers for assertions */
int  test_stage_has(TestGame *tg, int area, int card_id);
int  test_hand_has(TestGame *tg, int card_id);
int  test_success_count(TestGame *tg);
void test_print_board(TestGame *tg);

/* phase / choice introspection + modifier getters (mirror TestGame helpers) */
void test_pass(TestGame *tg);
const char *test_pending_choice_type(TestGame *tg);
int  test_get_blade_modifier(TestGame *tg, int cid);
int  test_get_score_modifier(TestGame *tg, int cid);
int  test_get_cost_modifier(TestGame *tg, int cid);
int  test_get_heart_modifier(TestGame *tg, int cid, int color);
int  test_filler_hand(TestGame *tg);
int  rb_card_no_eq(int card_id, const char *no);
int  test_zone_has_card_no(TestGame *tg, int pl, const char *zone, const char *no);
int  test_zone_has_id(TestGame *tg, int pl, const char *zone, int id);

/* Answer a play paused by the play-time alternative-cost hook (Rust answer_play_choice
    for target "play_time_cost_reduction"). Completes the play at the chosen cost. If no
    such play is pending, falls back to the generic choice resume. */
void test_answer_play_cost_choice(TestGame *tg, int accept);

#endif

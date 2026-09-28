#ifndef TEST_GAME_H
#define TEST_GAME_H
#include "rabuka.h"

/* Minimal TestGame shim mirroring engine/tests/helpers/mod.rs:361 TestGame
   for mass-porting Rust tests to C. Provides the same vocabulary:
   test_game_new, test_id, add_to_hand/stage/discard, give_energy,
   play_to_stage, activate_ability, recalc, board introspection.
   Uses the real card database (cards.bin) via rb_find_card_by_no.
   test_id() returns the shared template index; test_new_id() allocates a
   distinct instance backed by that same card record, matching Rust's pool.
   Mirrors Rust helpers/mod.rs:407 TestGame::new in Main phase. */

typedef struct {
    GameState state;
} TestGame;

void test_game_new(TestGame *tg);
int  test_id(TestGame *tg, const char *card_no); /* card index or -1, like Rust i16 */
int  test_new_id(TestGame *tg, const char *card_no); /* distinct copy or -1 */
void test_add_to_hand(TestGame *tg, int card_id);
void test_add_to_discard(TestGame *tg, int card_id);
void test_add_to_stage(TestGame *tg, int area, int card_id); /* area 0=left 1=center 2=right */
void test_place_under(TestGame *tg, int pl, int area, int card_id); /* tuck card under member at area */
void test_add_to_success(TestGame *tg, int card_id);
void test_add_to_live(TestGame *tg, int card_id);
/* Seat-aware raw live placement (append, no hand removal). test_add_to_live is
   test_add_to_live_for(tg, 0, ...). */
void test_add_to_live_for(TestGame *tg, int pl, int card_id);
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
int  test_pending_choice_count(TestGame *tg);/* live shim (mirror helpers/mod.rs set_live_card).
   Rust's set_live_card(card_id) takes ONE argument and the engine APPENDS the
   card to the live card zone. `slot` exists only because tools/port_one.py
   emits test_set_live_card(&game, 0, id); it is deliberately IGNORED.
   A previous revision wrote live.cards[slot] at a fixed index, so three
   consecutive calls (the transpiler always passes 0) left ONE live card in
   the zone and silently invalidated every transpiled live test. Do not
   reintroduce a positional write here -- use test_insert_live_card_at. */
void test_set_live_card(TestGame *tg, int slot, int card_id);
/* Seat-aware live placement (Rust set_live_card_for(Side, card)). Writes `pl`'s
   OWN live zone and still APPENDS -- a P2-owned live card must land in
   p2.live_card_zone, or a P2-triggered ability never sees it while the
   fixture still looks plausible. P1 is the legacy default of
   test_set_live_card; use this whenever the card belongs to a named seat. */
void test_set_live_card_for(TestGame *tg, int pl, int card_id);
/* Explicit positional live-zone placement, for the rare test that needs a
   card at a specific live slot. test_set_live_card is the append path. */
void test_insert_live_card_at(TestGame *tg, int slot, int card_id);
/* Seat-aware positional live placement (P1 default above). */
void test_insert_live_card_at_for(TestGame *tg, int pl, int slot, int card_id);
const char *test_card_name(int card_id);
int  test_find_live_by_score(TestGame *tg, int score);

/* board helpers for assertions */
int  test_stage_has(TestGame *tg, int area, int card_id);
int  test_hand_has(TestGame *tg, int card_id);
int  test_success_count(TestGame *tg);
void test_print_board(TestGame *tg);

/* ── side-parameterised setup (mirror the Rust `*_for(Side, ..)` family) ── */
void test_add_to_hand_for(TestGame *tg, int pl, int card_id);
void test_give_energy_for(TestGame *tg, int pl, int count);
int  test_play_to_stage_for(TestGame *tg, int pl, int card_id, int area);
int  test_activate_ability_for(TestGame *tg, int pl, int card_id);

/* ── phase advance (mirror TestGame::advance_to_phase) ──
   Steps the turn until `target` (an RbPhase) is current, handing any prompt
   raised by the arriving step back to the caller. Returns 1 on arrival, 0 if
   the phase was never reached within 16 passes. */
int  test_advance_to_phase(TestGame *tg, int target);

/* ── zone-content assertions (beyond test_zone_has_id) ──
   `zone` accepts the Rust field names and the short C aliases:
   "hand", "deck"/"main_deck", "discard"/"waitroom", "live"/"live_card_zone",
   "success"/"success_live_card_zone", "energy"/"energy_zone",
   "energy_deck", "stage", "under<0..2>" (e.g. "under0"). */
int  test_zone_len(TestGame *tg, int pl, const char *zone);
int  test_zone_count_of_id(TestGame *tg, int pl, const char *zone, int id);
/* Dump a zone's contents in order for order-sensitive assertions. Returns the
   number of cards written (capped at `max`). */
int  test_zone_ids(TestGame *tg, int pl, const char *zone, int *out, int max);
/* Zone-independent card census: every card slot of BOTH seats across every
   zone (deck, hand, stage, under-cards, energy, energy-deck, live, success,
   waitroom). A card that moves between zones must leave this total unchanged. */
int  test_total_card_count(TestGame *tg);

/* ── cost/selection zone moves ──
   TEST-SIDE stand-in for a known engine gap: the fixed-count hand-selection
   cost path (rb_resolver_handle_hand_selection, choice.c) records the picks
   in resolver state and never moves them, so the cost cards stay in hand.
   These helpers let a test perform -- and therefore assert -- the move the
   engine omits. They do NOT emulate any engine decision; do not use them to
   paper over engine behaviour a test is meant to exercise. */
int  test_move_hand_to_waitroom(TestGame *tg, int pl, int n);
int  test_move_ids_to_waitroom(TestGame *tg, int pl, const int *ids, int n);

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

/* Choice-resume + introspection helpers used by the scenario replay runner
 (tests/replay.c scenario mode). Defined in src/test_game.c. */
void test_resume_choice(TestGame *tg, int idx);
/* Select indices from a pending choice — mirrors game.select_indices(&[..]).
   n == 0 DECLINES the prompt (Rust's select_indices(&[])), which is
   rb_resume_with_choice(g, -1); n > 1 passes the whole multi-index answer to
   rb_resume_with_choice_indices. The previous revision silently used only
   indices[0] and treated n == 0 as a total no-op, so a Rust "decline" left
   the prompt pending and a multi-pick could never complete. */
void test_select_indices(TestGame *tg, const int *indices, int n);
int  test_deck_len(TestGame *tg);
int  test_hand_len(TestGame *tg);

/* ── queue-entry seat introspection ──────────────────────────────────────
   One typed spelling of what two suites were hand-rolling as a raw
   strcmp against RbQueueEntry.player_id / .choice_player_id
   (test_parity_queue_resume.c, and test_p1_helpers.c's own queue block).
   The seat accessors return 0 (p1), 1 (p2) or -1 for an EMPTY or
   unrecognised token -- -1 is deliberately distinct from 0, so "the entry was
   never stamped with an owner" is assertable rather than invisible. The long
   form is normalised ("player1" -> "p1") as Rust's build_ability_queue_entry
   does (abilities.rs:240-246). RbQueueEntry's layout is unchanged. */
#define TEST_SEAT_ID_LEN 16
const char *test_seat_id(int pl);              /* "p1" / "p2" / ""          */
int  test_queue_n_entries(TestGame *tg);
int  test_queue_entry_seat(TestGame *tg, int idx);          /* 0 | 1 | -1     */
int  test_queue_entry_choice_seat(TestGame *tg, int idx);   /* 0 | 1 | -1     */
int  test_queue_entry_owned_by_seat(TestGame *tg, int idx, int pl);
int  test_queue_entry_choice_owned_by_seat(TestGame *tg, int idx, int pl);
/* Raw readers. Return 1 when `idx` addresses a real entry, 0 otherwise; `buf`
   is always cleared and NUL-terminated first, so a bad read yields "". */
int  test_queue_entry_player_id(TestGame *tg, int idx, char *buf, size_t buf_len);
int  test_queue_entry_choice_player_id(TestGame *tg, int idx, char *buf, size_t buf_len);

/* Distinct-name count. rb_max_distinct_names is defined in
   src/ability/util.c but not declared in include/rabuka.h (not this file's to
   edit), so it is declared locally in src/test_game.c and wrapped here;
   without it a test had to use the declared sibling
   rb_count_distinct_member_name_units and measure something else. */
int  test_max_distinct_names(const int *cards, int n);

#endif

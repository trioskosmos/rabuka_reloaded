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
/* EXACT card_no lookup, or -1. rb_find_card_by_no (and therefore test_id)
   is deliberately forgiving -- base+rarity rebuild, then lowest-prefix, then
   a SUBSTRING search (data.c:446) -- so a truncated or mistyped print
   ("PL!SP-PR-004-R" vs the real "PL!SP-PR-004-PR") resolves to some other
   real card instead of failing. Use test_id_exact whenever the print itself
   is under test; test_id stays forgiving because most of the corpus needs it. */
int  test_id_exact(TestGame *tg, const char *card_no);
int  test_new_id(TestGame *tg, const char *card_no); /* distinct copy or -1 */
/* PEEK, not a consume — mirror Rust TestGame::id_ref (helpers/mod.rs:245-253).
   Answers "what would the next id() hand out for this card_no?" WITHOUT
   advancing anything. DIVERGENCE, stated: in the C this returns the shared
   TEMPLATE id, which is also exactly what C's test_id() returns on every call,
   because C's test_id has no per-card counter at all. So in the C the peek and
   the consume always agree; in Rust they differ by one pool position. */
int  test_id_ref(TestGame *tg, const char *card_no);
/* Release every card-copy id back to the pool (data.c:279). The pool is
   process-global and MONOTONIC and nothing in the corpus resets it, so it
   runs dry after 4096-2639 = 1457 allocations -- and
   rb_build_deck_from_card_numbers eats from the same pool. Once dry,
   test_new_id returns -1 and every caller that ignores it gets two different
   prints resolving to the same id, which makes every identity assertion on
   them vacuously true. Suites that lean on test_new_id should call this
   between cases. Only safe once no copy handed out before the reset is still
   in play. */
void test_card_copy_pool_reset(void);
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
/* Mirror Rust TestGame::play_to_stage_baton (helpers/mod.rs:404): play into an
   area that MAY be occupied — the opt-in twin of test_play_to_stage, which
   warns when the swap was not asked for. */
int  test_play_to_stage_baton(TestGame *tg, int card_id, int area);
/* Mirror Rust TestGame::play_to_stage_double_baton (helpers/mod.rs:418): replace
   TWO members and land in a third. The only path that records two touches. */
int  test_play_to_stage_double_baton(TestGame *tg, int card_id, int area_a, int area_b, int land_in);
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
/* Mirror Rust pending_choice_count (helpers/choices.rs:421-428): HOW MANY CARDS
   THE PROMPT ASKS FOR, not "is one standing". A SelectCard answers its own
   `count`; every other kind answers 0 (SelectHeartColor / SelectAutoAbility /
   SelectLiveSuccess ask for no cards). It used to answer a constant 1 whenever
   any prompt existed, which was right only for a `SelectCard { count: 1 }`. */
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

/* ── phase advance (mirror TestGame::advance_to_phase,
       engine/tests/helpers/mod.rs:538-561) ──────────────────────────────────
   The turn is stepped ONE PHASE AT A TIME. A choice raised by the step is
   NEVER answered: it is handed back to the caller, because the phase being
   stepped into is usually the window whose effects raise the prompt, and
   draining it would consume the choice the test came to inspect.

   Contract of test_advance_to_phase_strict():
     * returns 1 only if `target` (an RbPhase, or a Rust Phase name via
       test_advance_to_phase_named) is the CURRENT phase on return;
     * otherwise returns 0 and changes nothing else -- check
       test_has_pending_choice() to find out whether it stopped on a prompt;
     * gives up after 16 passes (Rust's own bound, mod.rs:539);
     * rb_advance_phase is a no-op while a choice is pending, so the walk
       cannot spin on a frozen machine: it stops and reports instead.

   KNOWN ENGINE BEHAVIOUR the helper exposes but does NOT hide:
     * ENERGY -> DRAW sets the phase WITHOUT drawing (phase.c:52-56); the
       rb_draw that serves the seat runs on the NEXT pass, from the DRAW arm.
       "At Draw phase" and "the seat has its card" are one pass apart.
     * MAIN flips `active` to second_attacker and returns to ACTIVE
       (phase.c:71-73), so the Active/Energy/Draw triple that follows -- the
       round's FIRST draw -- is served to the OTHER seat. Read
       TestPhaseAdvance.in_swap_window, or check `tg.state.active` yourself;
       the helper will not paper over this.
     * RB_PHASE_PERFORMANCE and RB_PHASE_PERFORMANCE_SECOND (and their
       LIVE_SET pair) are ONE stored value each in the C engine, told apart
       by which seat is active (phase.c:87-90, :1070). The *_SECOND values
       are never stored, so a literal compare against them can never fire --
       which is why the legacy helper below cannot reach them at all. */
typedef enum {
    TEST_PHASE_STOPPED_REACHED   = 1, /* `target` is current on return         */
    TEST_PHASE_STOPPED_PROMPT    = 2, /* a choice is standing: answer it yourself */
    TEST_PHASE_STOPPED_TIMEOUT   = 0, /* not reached within max_passes           */
    TEST_PHASE_STOPPED_GAME_OVER = 3, /* the game is decided; it will not advance */
    TEST_PHASE_STOPPED_BAD_TARGET= 4  /* tg was NULL / target unrecognised       */
} TestPhaseStop;

typedef struct {
    int reached;        /* 1 iff `target` was current on return                */
    int prompt;         /* 1 iff a choice is standing (this helper never answers) */
    int passes;         /* rb_advance_phase calls issued                        */
    int noops;          /* passes that moved neither phase nor active seat      */
    int active;         /* tg->state.active on return                           */
    int first_attacker; /* tg->state.first_attacker on return                  */
    int second_attacker;/* tg->state.second_attacker on return                 */
    int in_swap_window; /* 1 iff active != first_attacker -- see the note above */
    int winner;         /* tg->state.winner on return (-1 = undecided)          */
} TestPhaseAdvance;

int  test_advance_to_phase_strict_ex(TestGame *tg, int target, int max_passes,
                                     TestPhaseAdvance *out); /* full detail */
int  test_advance_to_phase_strict(TestGame *tg, int target);   /* 1 == reached */
/* Same, addressed by Rust's Phase name ("Active", "Draw", "Main",
   "LiveCardSetFirstAttacker", "LiveCardSetSecondAttacker",
   "FirstAttackerPerformance", "SecondAttackerPerformance",
   "LiveVictoryDetermination", ...). Returns 1 on arrival, 0 otherwise. */
int  test_advance_to_phase_named(TestGame *tg, const char *phase_name);
int  test_phase_from_name(const char *name);   /* name -> RbPhase, -1 unknown  */
const char *test_phase_to_name(int phase);     /* RbPhase -> Rust Phase name  */
const char *test_phase_name_now(TestGame *tg); /* phase the game is in, now  */
/* Prints where an advance stopped, which phase it wanted, the seats and the
   swap-window flag. Safe to call unconditionally. */
void test_advance_report(TestGame *tg, const char *what, int target, int stop);
/* Faithful port of Rust advance_to_phase INCLUDING its auto-drain: answers
   every prompt raised mid-walk with option 0. Inside the live window that
   eats the YELL (rb_performance runs yell, 自動, allocation, score,
   live-success and victory inside one pass), so it is destructive. Use only
   when transcribing a Rust test whose assertions were written against the
   drained walk; otherwise use test_advance_to_phase_strict. */
int  test_advance_to_phase_rust(TestGame *tg, int target);
/* LEGACY and DANGEROUS -- behaviour unchanged so no shipped suite moves, but
   DO NOT BUILD ON IT. It auto-answers every prompt with index 0 and
   compares the phase LITERALLY, so RB_PHASE_PERFORMANCE_SECOND /
   RB_PHASE_LIVE_SET_SECOND can never match (they are never stored) and the
   16-pass walk runs straight through the yell window picking option 0.
   That is what made shipped tests falsely red. Prefer
   test_advance_to_phase_strict. */
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
/* Colour index goes STRAIGHT into rb_mods_get_heart (modifiers.c:64), which
   uses it as the raw slot 0..7. Engine colour map (state.c:2002):
     0=heart00 1=heart01 2=heart02 3=heart03 4=heart04 5=heart05 6=heart06 7=all
   so RB_HEART_PURPLE == 5 is heart05 and RB_HEART_ORANGE == 6 is heart06.
   This used to remap a requested 5 onto RB_HEART_ORANGE, which made every
   heart05 read return the heart06 slot -- a false green whenever heart06
   happened to hold the value. It is now a plain passthrough. */
int  test_get_heart_modifier(TestGame *tg, int cid, int color);
int  test_filler_hand(TestGame *tg);
/* DO NOT confuse with test_filler_hand, which RETURNS a filler card id and
   mutates nothing. These two are the Rust `fill_decks` / `fill_energy_deck`
   twins (helpers/mod.rs:564-583) and they MUTATE: fill_decks clears both main
   decks and leaves 30 copies of `filler` in each (the clear is load-bearing —
   a deck the fixture meant to be empty otherwise keeps the setup's cards);
   fill_energy_deck pushes `count` copies of "LL-E-001-SD" into seat `pl`'s
   ENERGY DECK (the draw pile) and does NOT raise energy_active. */
void test_fill_decks(TestGame *tg, int filler);
void test_fill_energy_deck(TestGame *tg, int pl, int count);
/* Mirror Rust stage_occupant (mod.rs:440-444): the card in `area` of P1's stage
   that is NOT `card_id`, or -1. This is the assertion that catches the
   baton-touch-into-your-own-fixture trap (docs/PBLL02_ABILITY_REVIEW.md §8.3,
   137 affected tests) — before this existed a test could not check its own
   setup even when it wanted to. */
int  test_stage_occupant(TestGame *tg, int card_id, int area);
/* EXACT card-number compare (never the forgiving rb_find_card_by_no chain).
   An empty `no` is refused: rb_card_string returns the shared "" literal for
   an out-of-range index, so "" used to match EVERY card. */
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

/* ═══════════════════════════════════════════════════════════════════════════
   FIRE ONE ABILITY ON ONE CARD  (Rust helpers/mod.rs:114 `fire_trigger`)
   ═══════════════════════════════════════════════════════════════════════════
   Rust resolves ONE ability_id for ONE card and calls trigger_auto_ability
   with an explicit card id. The C corpus used to spell this as
   `rb_queue_trigger_abilities(&tg->state, 0, trig)`, which is a WHOLE-BOARD
   TAS scan: `cid` only ever set `activating_card`, and every card in P1's
   stage / success / live / hand / energy zone that printed the same trigger
   fired too, doubling every discard / draw / score effect in the fixture.

   These target exactly one ability of exactly one card, through the existing
   engine entry point rb_trigger_auto_ability (the port of Rust
   GameState::trigger_auto_ability). No new engine API was needed.

   Return 1 when exactly one queue entry was produced, 0 when the card prints no
   such ability at that index or the enqueue was refused -- so a suite's
   `if (!fire_trigger(...)) return 0;` premise check keeps its meaning.

   No movement batch is snapshotted (Rust passes None). A suite that needs an
   appearance / movement event must publish it itself. */

/* Absolute ability index of the nth PRINTED ability of `cid` carrying `trig`,
   or -1. "Carrying" is the engine's own predicate (rb_trigger_is), so a card
   the TAS scan would refuse is never reported as printable. */
int  test_ability_index_for_trigger(int cid, const char *trig, int nth);
/* nth == 0 (Rust `fire_trigger`). */
int  test_fire_trigger_on_card(TestGame *tg, int cid, const char *trig);
/* pl == 1 for a suite whose Rust twin fires the trigger on the OPPONENT
   (test_parity_reveal_extra's fire_trigger_p2 and its neighbours). */
int  test_fire_trigger_on_card_seat(TestGame *tg, int cid, const char *trig, int pl);
/* Rust `fire_trigger_nth`: the Nth printed ability carrying the trigger. */
int  test_fire_trigger_on_card_nth(TestGame *tg, int cid, const char *trig, int nth);
/* ABSOLUTE ability index, for a suite isolating one leg of a multi-leg watcher.
    Not the same as _nth. Refuses an ability that does not print a trigger. */
int  test_fire_trigger_at_index(TestGame *tg, int cid, int ability_idx);

/* ── FLAT-suite per-test verdict layer ─────────────────────────────────────
    A FLAT suite runs every test body in ONE process out of ONE main(), so its
    `FAIL:` lines name an ASSERTION and not a test. A suite that adopts this
    puts ONE line in the failure branch of EVERY assertion macro it defines:

        #define CHECK(cond, msg) do {                        \\
            if (!(cond)) {                                  \\
                fprintf(stderr, "FAIL: %s\n", (msg));       \\
                failures++; FLAT_VERDICT();                  \\
            }                                               \\
        } while (0)

    `FLAT_VERDICT()` prints `FAILED: <enclosing function>` exactly once per
    function, so the failing tests in a FLAT suite become countable. Put it in
    `CHECK` AND in `CHECK_EQ` -- patching only `CHECK` silently loses the
    majority of a suite's failures, because most FLAT suites fail through
    CHECK_EQ. For a suite that forks per case, use the `FAILED:`/`ok:` verdict
    the FORK harness already prints instead; this is for the FLAT shape only.
    Defined once, in src/test_game.c, so the dedupe cannot drift per suite. */
void test_flat_verdict_failed(const char *fn);
int  test_flat_failed_count(void);
#define FLAT_VERDICT() test_flat_verdict_failed(__func__)

#endif

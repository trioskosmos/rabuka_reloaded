/* ============================================================================
 * game_setup.c — C port of engine/src/game/game_setup.rs (+ deck_ordering.rs),
 * plus the opening-sequence / mulligan rules whose Rust twins live in
 * engine/src/turn/phases.rs and engine/src/turn/actions/mod.rs and which the C
 * port has as STUBS in src/turn/phase.c.
 *
 * SCOPE (see the porting brief + PROGRESS.md "THE RULE"):
 *   game_setup.rs is ~75% presentation. Only the GAME-RULES half is ported:
 *     - the Pre-Game Procedure            (setup_game)
 *     - automatic-phase classification     (is_automatic_phase,
 *                                          is_live_card_set_phase)
 *     - the auto-advance / settle loops    (auto_advance_one,
 *                                          settle_single_player_state,
 *                                          settle_auto)
 *     - action dispatch                   (stage_area_from_parameters,
 *                                          execute_action,
 *                                          execute_action_for_player)
 *     - the DECISION content of the main-phase action generators
 *       (stage_play_cost, is_baton_touchable, double_baton_options,
 *        has_double_baton, area_candidates_for) exposed headlessly as
 *        rb_setup_*_candidates(), plus the live-card-set and mulligan
 *        selection limits.
 *   Deliberately NOT ported (presentation / transport, see the handover
 *   report): Action.description / description_ja, Action::with_ja /
 *   display_desc_for / display_desc, area_label_inner / area_label_en /
 *   area_label_ja, ActionType's Display + FromStr, ActionParameters'
 *   display fields (card_name / card_no / source_ability / disabled),
 *   AreaInfo, DoubleBatonPair-as-menu-rows, ActionType::to_tag / from_tag
 *   (3DS multiplayer wire protocol = link/transport), and every *_ja string.
 *   The generate_*_actions functions themselves are MENU BUILDERS; their rules
 *   content is what the rb_setup_*_candidates entry points below reproduce.
 *
 * EVERY exported symbol is prefixed `rb_setup_` so it cannot collide with an
 * existing rb_* (PROGRESS.md anti-pattern #3/#4: no new parallel architecture,
 * translate onto the existing GameState model).
 *
 * ──────────────────────────── REACHABILITY MAP (2026-10-02) ────────────────
 * This port's dominant defect is CORRECT CODE WITH ZERO CALLERS, so the call
 * graph is recorded here rather than left to be re-derived. External callers
 * (src/ and tests/ counted separately; no test/ file references rb_setup_* at
 * all, so every count below is engine-side):
 *
 *   WIRED (external src caller)
 *     rb_setup_game                        engine.c:1246 (decl), :1324 (call)
 *     rb_setup_choose_attacker             phase.c:22 (decl), :103
 *     rb_setup_handle_mulligan_skip        phase.c:23 (decl), :111
 *     rb_setup_for_each_ability_candidate  phase.c:33 (decl), :1459
 *
 *   Everything else is reachable only from inside this file. Two blockers,
 *   both outside this file's ownership:
 *
 *   (1) rb_setup_execute_action / _for_player take `RbSetupAction`, and
 *       `RbSetupAction` / `RbSetupActionType` are declared HERE, not in
 *       include/rabuka.h. No other translation unit can construct the argument,
 *       so the entire 29-variant action dispatch (the C twin of Rust's 81-caller
 *       game_setup::execute_action) is uncallable by construction. That is why
 *       rb_setup_mulligan_candidates, rb_setup_play_candidates,
 *       rb_setup_live_card_candidates, rb_setup_concede,
 *       rb_setup_stage_area_from_parameters and the RPS / SetLiveCard /
 *       PlayMemberToStage arms have no caller: they are all downstream of it.
 *       tests/test_parity_rules.c:1363 and tests/test_parity_rules_extra.c:229,
 *       :275, :307 assert that concede gap as an EXPECTED_GAP, so whoever adds
 *       the header field must revisit those pins with their owner.
 *
 *   (2) rb_generate_action_candidates (src/turn/phase.c:1353) returns an empty
 *       list unless phase == RB_PHASE_MAIN, so it never reaches the mulligan /
 *       live-set / RPS row builders. That is why the *_candidates row functions
 *       and rb_setup_is_live_card_set_phase have no consumer.
 *       rb_generate_action_candidates also carries an INLINE re-implementation
 *       of the play-member scan that rb_setup_play_candidates ports faithfully,
 *       and the two disagree twice:
 *         phase.c:1384  effective_cost = card.cost + cost_modifier - reduction
 *         Rust  :1769/1783  effective_cost = card_cost.saturating_sub(reduction)
 *                           (card_cost = card.cost.unwrap_or(0); the constant
 *                           cost modifier is NOT folded in here)
 *         phase.c:1405  existing_cost = existing.cost + existing_modifier
 *         Rust  :1674  floored_cost(existing.cost, mods.get_cost_modifier(..))
 *                      = saturate_u8(base + modifier).max(1)
 *         so a 0-cost baton-touched member is priced at 0 by the live path and
 *         at 1 by Rust.
 *       Neither drift is fixable from here; both disappear the moment that loop
 *       calls rb_setup_play_candidates.
 *
 *   NOT A DEFECT — presentation. The eight deck_ordering.rs functions
 *   (rb_setup_series_priority / _rarity_priority / _extract_series /
 *   _extract_rarity / _extract_base / _gba_sort_key / _sort_deck_for_gba /
 *   _flatten_and_sort_for_gba) are GBA-deck-builder ordering, pure string and
 *   sort logic. Nothing in engine/src/game/ calls them: series_priority and
 *   rarity_priority are called only by gba_sort_key, gba_sort_key only by
 *   sort_deck_for_gba / flatten_and_sort_for_gba, and extract_base, both sorts
 *   and the whole module have ZERO callers outside deck_ordering.rs's own
 *   #[cfg(test)] mod. platforms/gba/src/deck_builder.rs:889 carries its own
 *   copy (extract_base_card_no) instead of importing them. So they are dead in
 *   Rust too, they are GBA-UI presentation rather than a rule, and engine_c_wip
 *   has no GBA front end to consume them. Recorded so the next reader does not
 *   spend a session on it as a false lead.
 *
 * CROSS-FILE REQUESTS are marked `XREQ` below and repeated in the handover
 * report: everything in this file is verified with
 *   gcc -fsyntax-only -std=c11 -Iinclude -Isrc -Isrc/core/generated \
 *       src/game/game_setup.c
 * ==========================================================================*/

#include "rabuka.h"
#include "deck_builder.h"
#include "deck_parser.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── Local externs: real functions that exist in the C port but are not yet
 *    declared in include/rabuka.h. XREQ: declare these in rabuka.h. ──────── */
extern int rb_handle_rps_choice_p1(GameState *g, int choice);      /* phase.c */
extern int rb_handle_rps_choice_p2(GameState *g, int choice);      /* phase.c */
extern int rb_handle_set_live_card(GameState *g, int card_id);     /* phase.c */
extern int rb_handle_live_card_selection(GameState *g, int card_id,
                                         const int *indices, int n_indices);
extern int rb_handle_live_card_confirmation(GameState *g, const int *indices,
                                            int n_indices);
extern int rb_handle_live_card_skip(GameState *g);
extern int rb_handle_play_member_to_stage(GameState *g, int card_id,
                                          const int *indices, int n_indices,
                                          int stage_area, int use_baton_touch);
extern int rb_check_trigger_position(const char *triggers, int card_position);
extern int rb_check_effect_position(const char *effect_pos, int card_position);
extern void rb_queue_reset(GameState *g);   /* ability_queue.c:2554 */

/* Defined at the bottom of this file (deck_builder.h has no status to_str). */
const char *rb_setup_deck_build_status_str(int status);

/* Upper bound for one rb_setup_ability_candidates() result. */
#define RB_SETUP_MAX_ABILITIES 256
/* A hand member can replace at most (3 choose 2) pairs, each with two
 * placements, so the double-baton table for ONE card is at most 6 rows
 * (game_setup.rs:1569 double_baton_options). */
#define RB_SETUP_MAX_DOUBLE_BATON 6

/* ==========================================================================
 * Action vocabulary — game_setup.rs:51-80 (ActionType)
 *
 * The VARIANT LIST is the engine's action vocabulary and is required by the
 * dispatch below, so it is ported verbatim and in Rust declaration order. The
 * Display impl (wire/UI strings) and to_tag/from_tag (3DS link protocol) are
 * not ported — see the scope note at the top of the file.
 * ========================================================================*/
typedef enum {
    RB_SETUP_ACT_PASS = 0,
    RB_SETUP_ACT_ROCK_CHOICE,
    RB_SETUP_ACT_PAPER_CHOICE,
    RB_SETUP_ACT_SCISSORS_CHOICE,
    RB_SETUP_ACT_CHOOSE_FIRST_ATTACKER,
    RB_SETUP_ACT_CHOOSE_SECOND_ATTACKER,
    RB_SETUP_ACT_MULLIGAN_HEADER,
    RB_SETUP_ACT_SELECT_MULLIGAN,
    RB_SETUP_ACT_CONFIRM_MULLIGAN,
    RB_SETUP_ACT_SKIP_MULLIGAN,
    RB_SETUP_ACT_LIVE_CARD_HEADER,
    RB_SETUP_ACT_SELECT_LIVE_CARD,
    RB_SETUP_ACT_CONFIRM_LIVE_CARD_SET,
    RB_SETUP_ACT_SKIP_LIVE_CARD_SET,
    RB_SETUP_ACT_PLAY_MEMBER_TO_STAGE,
    RB_SETUP_ACT_USE_ABILITY,
    RB_SETUP_ACT_SET_LIVE_CARD,
    RB_SETUP_ACT_FINISH_LIVE_CARD_SET,
    RB_SETUP_ACT_DRAW_CARD,
    RB_SETUP_ACT_CHOICE_DECISION,
    RB_SETUP_ACT_CHOICE_SELECT,
    RB_SETUP_ACT_CHOICE_SKIP,
    RB_SETUP_ACT_CHOICE_OPTION,
    RB_SETUP_ACT_CHOICE_POSITION,
    RB_SETUP_ACT_ENERGY_CHARGE,
    RB_SETUP_ACT_PASS_REMAINING,
    RB_SETUP_ACT_CONCEDE,
    RB_SETUP_ACT_COUNT
} RbSetupActionType;

/* Headless Action: only the fields the ENGINE consumes. The display fields
 * (description, card_name, card_no, source_ability, base_cost, final_cost,
 * available_areas, double_baton_pairs, disabled, selected) are menu data and
 * are not reproduced; the decision-relevant costs ARE reported by the
 * rb_setup_*_candidates() entry points instead. */
typedef struct {
    RbSetupActionType type;
    int has_card_id;
    int card_id;
    int card_indices[RB_MAX_HAND];
    int n_card_indices;
    const char *stage_area;      /* NULL = unset */
    int has_stage_area_index;
    int stage_area_index;
    int use_baton_touch;
    int has_use_baton_touch;
    int has_ability_index;
    int ability_index;
} RbSetupAction;

/* game_setup.rs:1617-1626 AreaCandidate, minus the display-only
 * `existing_member_name`. */
typedef struct {
    int slot;                  /* 0=left 1=center 2=right (AREA_NAMES order) */
    int available;
    int cost;
    int is_baton_touch;
    int existing_member_id;    /* RB_EMPTY_SLOT when the area is empty */
} RbSetupAreaCandidate;

/* game_setup.rs:320-324 DoubleBatonOption */
typedef struct {
    int area_index[2];
    int placement_area;
    int cost;
} RbSetupDoubleBatonOption;

/* Decision table for one hand member — the rules content of the per-hand-card
 * body of game_setup.rs:1690 generate_main_phase_actions. */
typedef struct {
    int card_id;
    int hand_index;
    int card_cost;             /* card.cost.unwrap_or(0) */
    int reduction;             /* calculate_play_cost_reduction */
    int effective_cost;        /* card_cost.saturating_sub(reduction) */
    RbSetupAreaCandidate areas[RB_STAGE_SIZE];
    int any_area_available;
    RbSetupDoubleBatonOption pairs[RB_SETUP_MAX_DOUBLE_BATON];
    int n_pairs;
} RbSetupPlayCandidate;

/* One offerable activation — the rules content of the stage + discard ability
 * scans in game_setup.rs:2078-2348. */
typedef struct {
    int card_id;
    int ability_index;         /* printed index, or RB_GAINED_ABILITY_INDEX_BASE+n */
    int from_stage;            /* 1 = stage scan, 0 = discard (控え室) scan */
    int stage_slot;            /* 0..2 when from_stage, else RB_EMPTY_SLOT */
    int base_cost;             /* AbilityEffect::energy_cost_total */
    int effective_cost;        /* effective_activation_cost_for(.., groups) */
    int groups_on_stage;
    int has_optional_payment;
} RbSetupAbilityCandidate;

/* Mulligan / live-card selection rows (the non-label part of
 * game_setup.rs:1467 generate_mulligan_actions and :2353
 * generate_live_card_set_actions). */
typedef struct {
    int hand_index;
    int card_id;
    int selected;
    int selectable;            /* live-card rows only: false at the limit */
} RbSetupHandCandidate;

/* ─────────────────────────── small helpers ─────────────────────────── */

/* SELECTION STORAGE (flagged 2026-10-02 — HEADER REQUEST, do not "fix" here).
 *
 * These three helpers read and write g->selected_cards / g->n_selected_cards,
 * which is the SINGLE selection list include/rabuka.h:1379-1380 declares. Rust
 * has THREE separate ones and this file currently borrows one array for two of
 * them:
 *   core/game_state/mod.rs:151  mulligan_selected_indices: SmallVec<[u8; 6]>
 *   core/game_state/mod.rs:152  live_card_selected_indices: SmallVec<[u8; 3]>
 *   the pending-choice selection that include/rabuka.h:1379 already carries
 *   (chosen.c reads/writes it in ~164 places).
 * phases.rs:743/770/796/802 use the FIRST for every mulligan row and
 * phases.rs:856/863/867/880/903/914 use the SECOND for every live-set row.
 *
 * So a mulligan selection and a live-card selection are NOT independently
 * representable here, and neither is either of them versus a pending choice.
 * It is correct today for two reasons and no more: the mulligan phases and the
 * live-set phases are disjoint, and rb_advance_phase refuses to move while a
 * choice is pending (phase.c:68), so the headless auto-pass can never clear a
 * live choice list through setup_clear_selection(). Neither property is
 * enforced — nothing stops a host from driving rb_setup_handle_mulligan_skip
 * while a choice is open, and setup_clear_selection would silently drop the
 * choice's picks.
 *
 * Rust lines for the header field that makes this representable:
 *   core/game_state/mod.rs:151  mulligan_selected_indices: SmallVec<[u8; 6]>
 *   core/game_state/mod.rs:152  live_card_selected_indices: SmallVec<[u8; 3]>
 * i.e. two more fixed int[N] + int count pairs on GameState. RB_MAX_RECENTLY_MOVED
 * (8) is a safe N for the mulligan list (a hand is at most RB_MAX_HAND, but a
 * mulligan selection is a subset of the 6 dealt cards) and RB_MAX_LIVE_CARDS (3)
 * is exactly right for the live-set list, which is already hard-capped by
 * rb_setup_live_card_set_max.

/* AbilityEffect extra-string getter (card.rs filter_str_getter!). */
static const char *setup_effect_extra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

/* Is the game still running? Rust tests `game_result != GameResult::Ongoing`
 * (game_setup.rs:384, :436). The C port spells "ongoing" as winner == -1
 * (engine.c:1596 rb_game_init seeds winner = -1 and every terminal path sets
 * it). */
static int setup_game_ongoing(const GameState *g) {
    return g && g->winner == -1 && g->phase != RB_PHASE_DONE;
}

/* Does `selected` contain `idx`? (phases.rs:681 deselect_index) */
static int setup_index_selected(const GameState *g, int idx) {
    for (int i = 0; i < g->n_selected_cards; i++)
        if (g->selected_cards[i] == idx) return 1;
    return 0;
}

/* Remove `idx` from the selection list; returns 1 when something was removed.
 * phases.rs:681-692 deselect_index. */
static int setup_deselect_index(GameState *g, int idx) {
    for (int i = 0; i < g->n_selected_cards; i++) {
        if (g->selected_cards[i] != idx) continue;
        for (int j = i; j < g->n_selected_cards - 1; j++)
            g->selected_cards[j] = g->selected_cards[j + 1];
        g->n_selected_cards--;
        return 1;
    }
    return 0;
}

static void setup_clear_selection(GameState *g) { g->n_selected_cards = 0; }

static void setup_select_index(GameState *g, int idx) {
    if (g->n_selected_cards >= RB_MAX_RECENTLY_MOVED) return;
    g->selected_cards[g->n_selected_cards++] = idx;
}

/* ==========================================================================
 * core/constants.rs:116 floored_cost — saturate_u8(base + modifier).max(1).
 * ==========================================================================*/
static int setup_floored_cost(int base, int modifier) {
    int v = rb_saturate_u8(base + modifier);
    return v < 1 ? 1 : v;
}

/* ==========================================================================
 * Pre-Game Procedure — game_setup.rs:358 setup_game
 *
 *   Rule 6.2      : Pre-Game Procedure
 *   Rule 6.2.1.7  : each player moves the top 3 cards of the energy deck to
 *                   the energy zone
 *   then the game starts at RockPaperScissors.
 *
 * ORDERING (XREQ, engine.c owner): the faithful C opening sequence is
 *   rb_game_init (decks + shuffle, no draw)
 *   → rb_setup_game            (initial energy, phase = RPS)
 *   → RPS choice                (rb_setup_execute_action(Rock/Paper/Scissors))
 *   → rb_setup_choose_attacker  (seats, opening hands, MULLIGAN_FIRST)
 *   → two mulligans             (rb_setup_handle_mulligan_*)
 *   → Active.
 * rb_game_init (engine.c:1611-1623) currently draws the two 6-card hands,
 * decides RPS from the seeded RNG and jumps straight to RB_PHASE_ACTIVE, which
 * collapses the whole ceremony. Calling rb_setup_choose_attacker on top of the
 * current rb_game_init double-draws the opening hands until that body is
 * trimmed — hence XREQ.
 * ==========================================================================*/
int rb_setup_game(GameState *g) {
    if (!g) return 0;
    rb_setup_initial_energy(g);
    g->phase = RB_PHASE_RPS;
    return 1;
}

/* game_setup.rs:367 is_automatic_phase — phases the engine advances with no
 * user input.
 *
 * Phase mapping onto the C RbPhase enum (include/rabuka.h:766):
 *   Rust Phase::Active|Energy|Draw                  → RB_PHASE_ACTIVE/ENERGY/DRAW
 *   Rust Phase::FirstAttackerPerformance
 *        |SecondAttackerPerformance                → RB_PHASE_PERFORMANCE
 *                                                     (C runs both windows
 *                                                      out of RB_PHASE_PERFORMANCE,
 *                                                      live_batch_mode)
 *   Rust Phase::LiveVictoryDetermination            → RB_PHASE_VICTORY
 * NOT automatic: Main, Mulligan*, LiveCardSet*, RockPaperScissors,
 * ChooseFirstAttacker — exactly the Rust `matches!` list. */
int rb_setup_is_automatic_phase(const GameState *g) {
    if (!g) return 0;
    switch (g->phase) {
        case RB_PHASE_ACTIVE:
        case RB_PHASE_ENERGY:
        case RB_PHASE_DRAW:
        case RB_PHASE_PERFORMANCE:
        case RB_PHASE_PERFORMANCE_SECOND:
        case RB_PHASE_VICTORY:
            return 1;
        default:
            return 0;
    }
}

/* game_setup.rs:398 is_live_card_set_phase — the user must act, but it is a
 * distinct kind of "must stop" from a normal decision phase. */
int rb_setup_is_live_card_set_phase(const GameState *g) {
    if (!g) return 0;
    return g->phase == RB_PHASE_LIVE_SET || g->phase == RB_PHASE_LIVE_SET_SECOND;
}

/* game_setup.rs:382 auto_advance_one — advance exactly one automatic phase
 * (no pending choice), returning 1 if it did. */
int rb_setup_auto_advance_one(GameState *g) {
    if (!g) return 0;
    if (rb_has_pending_choice(g) || !setup_game_ongoing(g)) return 0;
    if (!rb_setup_is_automatic_phase(g)) return 0;
    rb_advance_phase(g);
    return 1;
}

/* game_setup.rs:410 settle_single_player_state — drive forward through all
 * automatic phases until a phase that requires user input or a pending ability
 * choice appears. The 500-iteration guard is load-bearing (Rust breaks out of
 * the loop on trip); C has no log channel for the guard message. */
void rb_setup_settle_single_player_state(GameState *g) {
    if (!g) return;
    for (unsigned iters = 0; iters < 500u; iters++) {
        if (rb_has_pending_choice(g)) break;
        if (!rb_setup_is_automatic_phase(g)) break;
        rb_advance_phase(g);
    }
}

/* game_setup.rs:434 settle_auto — used by platform main loops after executing
 * an action. Same loop as settle_single_player_state PLUS the two pre-game
 * phases (RockPaperScissors / ChooseFirstAttacker).
 *
 * RB_PHASE_OPENING IS the C spelling of Rust's Phase::ChooseFirstAttacker:
 * src/core/data.c:650 names it "Choose 1st", src/turn/phase.c:458 says so
 * explicitly, and phase.c:102-105 dispatches it into rb_setup_choose_attacker.
 * Omitting it (as this body used to) means the loop breaks with the game
 * parked on Choose-1st, which is exactly the stall rb_advance_phase's own
 * RPS arm walks out of two lines earlier — the C would leave the opening and
 * then stop dead on the phase the RPS arm hands off to.
 *
 * The two MULLIGAN phases are deliberately NOT auto-advanced here, matching
 * Rust's `matches!` list: a mulligan is a player decision, so settle_auto must
 * stop and let generate_possible_actions offer the rows. */
void rb_setup_settle_auto(GameState *g) {
    if (!g) return;
    for (unsigned i = 0; i < 500u; i++) {
        if (rb_has_pending_choice(g) || !setup_game_ongoing(g)) break;
        if (rb_setup_is_automatic_phase(g) ||
            g->phase == RB_PHASE_RPS || g->phase == RB_PHASE_OPENING) {
            rb_advance_phase(g);
        } else {
            break;
        }
    }
}

/* ==========================================================================
 * Opening sequence — turn/actions/mod.rs:230 handle_choose_attacker
 * ("set flags, draw 6 each, enter mulligan").
 *
 * THIS FUNCTION HAS NO C COUNTERPART, and the Rust twin is the ONLY place the
 * opening hands are dealt (GameState::new draws nothing — it just sets
 * current_phase = Phase::Active, and setup_game moves it to RockPaperScissors).
 * It is ported here because it is the opening-sequence step this file owns;
 * wiring it up is a CROSS-FILE REQUEST (see the handover report and the
 * ordering note on rb_setup_game above).
 * ==========================================================================*/
int rb_setup_choose_attacker(GameState *g, int p1_first) {
    if (!g) return 0;
    g->first_attacker = p1_first ? 0 : 1;
    g->second_attacker = p1_first ? 1 : 0;
    g->active = g->first_attacker;
    for (int i = 0; i < 6; i++) {
        rb_draw(g, 0);
        rb_draw(g, 1);
    }
    g->phase = RB_PHASE_MULLIGAN_FIRST;
    setup_clear_selection(g);   /* mulligan_selected_indices.clear() */
    g->mulligan_selecting[0] = g->mulligan_selecting[1] = 0;
    g->mulligan_done[0] = g->mulligan_done[1] = 0;
    return 1;
}

/* ==========================================================================
 * Mulligan — turn/phases.rs + core/game_state/mod.rs:788
 *
 * PHASE OWNERSHIP (verified 2026-10-02). The stubs this note used to describe
 * — phase.c's rb_handle_mulligan_selection / _confirmation / _skip, which only
 * flipped mulligan_selecting[] / mulligan_done[] and never touched a hand —
 * ARE DELETED from src/turn/phase.c. The rb_setup_* functions below are the
 * single live implementation of turn/phases.rs:661-805.
 *
 *   - SKIP is wired: phase.c:110-113 drives RB_PHASE_MULLIGAN_FIRST/SECOND into
 *     rb_setup_handle_mulligan_skip, which is how a headless game walks the
 *     whole ceremony (rb_game_init parks on RPS → rb_advance_phase resolves RPS
 *     → RB_PHASE_OPENING → rb_setup_choose_attacker draws 6 each and enters
 *     MULLIGAN_FIRST → both mulligans skip → ACTIVE). Proven by rb_engine's
 *     own trace: both hands read 7 with deck 33 on the first printed state,
 *     i.e. 6 dealt + 1 draw phase.
 *   - SELECTION and CONFIRMATION are NOT reachable. They are reached only from
 *     rb_setup_execute_action, and rb_setup_execute_action has no caller
 *     because its parameter type RbSetupAction / RbSetupActionType is declared
 *     at the top of THIS FILE and not in include/rabuka.h — no other
 *     translation unit can name it. That is the whole blocker, and it is a
 *     header request, not a missing call.
 * ==========================================================================*/

/* game_state/mod.rs:788 mulligan_owner_index — the seat whose mulligan decision
 * the current mulligan phase belongs to; -1 outside a mulligan phase. */
int rb_setup_mulligan_owner_index(const GameState *g) {
    if (!g) return -1;
    if (g->phase == RB_PHASE_MULLIGAN_FIRST) return g->first_attacker;
    if (g->phase == RB_PHASE_MULLIGAN_SECOND) return g->second_attacker;
    return -1;
}

/* phases.rs:698 advance_mulligan_phase — MulliganFirst → MulliganSecond →
 * Active. Returns 0 (and changes nothing) when the current phase is not a
 * mulligan phase. */
int rb_setup_advance_mulligan_phase(GameState *g) {
    if (!g) return 0;
    int is_second = (g->phase == RB_PHASE_MULLIGAN_SECOND);
    if (g->phase == RB_PHASE_MULLIGAN_FIRST) {
        g->phase = RB_PHASE_MULLIGAN_SECOND;
        return 1;
    }
    if (is_second) {
        g->phase = RB_PHASE_ACTIVE;
        return 1;
    }
    return 0;
}

/* phases.rs:661 resolve_hand_answer_index — explicit indices win, else look the
 * card id up in the OWNER's hand, else 0. */
int rb_setup_resolve_hand_answer_index(const GameState *g, int owner,
                                       int has_card_id, int card_id,
                                       const int *indices, int n_indices) {
    if (indices && n_indices > 0) return indices[0];
    if (has_card_id) {
        const RbPlayer *P = (owner == 1) ? &g->p[1] : &g->p[0];
        for (int i = 0; i < P->hand.n; i++)
            if (P->hand.cards[i] == card_id) return i;
        return 0;
    }
    return 0;
}

/* phases.rs:722 handle_mulligan_selection — toggle a hand index in the
 * mulligan selection. Resolution is against the mulligan OWNER, not
 * active_player(), which is what made the original regression invisible. */
int rb_setup_handle_mulligan_selection(GameState *g, int has_card_id, int card_id,
                                        const int *indices, int n_indices) {
    if (!g) return 0;
    int owner = rb_setup_mulligan_owner_index(g);
    int idx = rb_setup_resolve_hand_answer_index(g, owner, has_card_id, card_id,
                                                 indices, n_indices);
    if (setup_deselect_index(g, idx)) return 1;
    setup_select_index(g, idx);
    return 1;
}

/* phases.rs:751 handle_mulligan_confirmation — replace the selected cards with
 * the same number of fresh cards FROM THE OWNER'S OWN DECK.
 *
 * Branch by branch:
 *   1. capture the owner BEFORE advancing the phase (the whole point of the
 *      Rust regression suite);
 *   2. advance the phase; a non-mulligan phase is a no-op success;
 *   3. an explicitly empty index list means "not specified" — fall back to the
 *      server-side selection rather than silently discarding it;
 *   4. dedup + descending order so earlier removals do not shift later ones;
 *   5. each removed card goes to the BOTTOM of the owner's main deck, the deck
 *      is then shuffled once, and exactly `removed_count` cards are drawn;
 *   6. clear the selection.
 * Returns the number of cards actually replaced. */
int rb_setup_handle_mulligan_confirmation(GameState *g, const int *indices, int n_indices) {
    if (!g) return 0;
    int owner = rb_setup_mulligan_owner_index(g);
    if (!rb_setup_advance_mulligan_phase(g)) return 0;

    int chosen[RB_MAX_HAND];
    int n_chosen = 0;
    if (indices && n_indices > 0) {
        for (int i = 0; i < n_indices && i < RB_MAX_HAND; i++) chosen[n_chosen++] = indices[i];
    } else {
        for (int i = 0; i < g->n_selected_cards && i < RB_MAX_HAND; i++)
            chosen[n_chosen++] = g->selected_cards[i];
    }
    /* deduped_desc(indices): sort, dedup, reverse. */
    for (int i = 0; i < n_chosen - 1; i++)
        for (int j = i + 1; j < n_chosen; j++)
            if (chosen[i] > chosen[j]) { int t = chosen[i]; chosen[i] = chosen[j]; chosen[j] = t; }
    int n_uniq = 0;
    for (int i = 0; i < n_chosen; i++) {
        int dup = 0;
        for (int j = 0; j < n_uniq; j++)
            if (chosen[j] == chosen[i]) { dup = 1; break; }
        if (!dup) chosen[n_uniq++] = chosen[i];
    }
    for (int i = 0, j = n_uniq - 1; i < j; i++, j--) {
        int t = chosen[i]; chosen[i] = chosen[j]; chosen[j] = t;
    }

    if (owner < 0) owner = 0;
    RbPlayer *P = &g->p[owner];
    int removed = 0;
    for (int i = 0; i < n_uniq; i++) {
        int idx = chosen[i];
        if (idx < 0 || idx >= P->hand.n) continue;
        int card = rb_hand_remove_card(P, idx);
        if (card < 0) continue;
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = card;
        removed++;
    }
    rb_shuffle(P->deck.cards, P->deck.n);
    for (int i = 0; i < removed; i++) rb_draw(g, owner);
    setup_clear_selection(g);
    return removed;
}

/* phases.rs:801 handle_mulligan_skip — clear the selection, advance the phase. */
int rb_setup_handle_mulligan_skip(GameState *g) {
    if (!g) return 0;
    setup_clear_selection(g);
    rb_setup_advance_mulligan_phase(g);
    return 1;
}

/* The non-label content of game_setup.rs:1467 generate_mulligan_actions: one
 * row per hand card of the mulligan OWNER with its current selection flag. */
int rb_setup_mulligan_candidates(const GameState *g, RbSetupHandCandidate *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int owner = rb_setup_mulligan_owner_index(g);
    if (owner < 0) return 0;
    const RbPlayer *P = &g->p[owner];
    int n = 0;
    for (int i = 0; i < P->hand.n && n < max; i++) {
        out[n].hand_index = i;
        out[n].card_id = P->hand.cards[i];
        out[n].selected = setup_index_selected(g, i);
        out[n].selectable = 1;
        n++;
    }
    return n;
}

/* ==========================================================================
 * Action dispatch — game_setup.rs:465 stage_area_from_parameters
 * index wins, else the area name is parsed; None when neither is present.
 * ==========================================================================*/
int rb_setup_stage_area_from_parameters(const RbSetupAction *a) {
    if (!a) return -1;
    if (a->has_stage_area_index) {
        int idx = a->stage_area_index;
        if (idx >= 0 && idx < RB_STAGE_SIZE) return idx;
        return -1;
    }
    if (a->stage_area && a->stage_area[0]) return rb_member_area_to_index(a->stage_area);
    return -1;
}

/* core/game_state/mod.rs:416 concede. C spells "the game ended" as winner != -1;
 * the opponent is awarded the win, keyed on the seat order (Rust derives it
 * from the first-attacker flags, which are the same two seats here). */
int rb_setup_concede(GameState *g, int player_id) {
    if (!g) return 0;
    if (player_id < 0 || player_id > 1) return 0;   /* Err("Invalid player seat") */
    if (!setup_game_ongoing(g)) return 0;            /* reject_if_ended */
    g->winner = player_id ^ 1;
    g->phase = RB_PHASE_DONE;
    rb_queue_reset(g);          /* ability_queue.clear() */
    return 1;
}

/* core/game_state/abilities.rs:3419 reset_loop_detection.
 * XREQ: the Rust struct also clears `pending_loop_protocol` and
 * `loop_last_action`; GameState has neither field yet (see handover report). */
void rb_setup_reset_loop_detection(GameState *g) {
    if (!g) return;
    g->n_game_state_history = 0;
    g->loop_detected = 0;
}

/* actions/mod.rs:195 rps_choice_value */
static int setup_rps_choice_value(RbSetupActionType t) {
    switch (t) {
        case RB_SETUP_ACT_ROCK_CHOICE: return 0;
        case RB_SETUP_ACT_PAPER_CHOICE: return 1;
        case RB_SETUP_ACT_SCISSORS_CHOICE: return 2;
        default: return -1;
    }
}

/* actions/mod.rs:206 handle_rps_choice — PVP routes by the session player id,
 * the sandbox path plays P1 then P2 sequentially. C has no pending_rps_player_id
 * field, so the sandbox branch is the faithful one to reproduce. */
static int setup_handle_rps_choice(GameState *g, int choice_value) {
    if (choice_value < 0) return 0;
    if (g->player1_rps_choice < 0) return rb_handle_rps_choice_p1(g, choice_value);
    return rb_handle_rps_choice_p2(g, choice_value);
}

/* game_setup.rs:474 execute_action → actions/mod.rs:242
 * execute_main_phase_action_with_ability_index.
 *
 * Returns 1 on success, 0 on failure (Rust Result<(), String>; the C port has no
 * error string channel).
 *
 * NOT PORTED: the trailing `gs.record_action_boundary(action.action_type)`
 * (game_setup.rs:502-510). That needs the Rule 12-1 loop protocol state
 * (`loop_last_action`, `pending_loop_protocol`, an exported state hash) which
 * GameState does not carry — XREQ, see the handover report. It is NOT faked
 * with a no-op call. */
int rb_setup_execute_action(GameState *g, const RbSetupAction *a) {
    if (!g || !a) return 0;
    if (a->type == RB_SETUP_ACT_CONCEDE) {
        int ok = rb_setup_concede(g, g->active);
        rb_setup_reset_loop_detection(g);
        return ok;
    }

    /* actions/mod.rs:256-265 — UseAbility is checked INDEPENDENTLY and must
     * never route through the pending-choice resume. */
    if (a->type == RB_SETUP_ACT_USE_ABILITY) {
        if (rb_has_pending_choice(g)) return 0;
        /* XREQ: rb_execute_main_phase_action (engine.c:1542) has no
         * ability_index / card_indices / stage_area / use_baton_touch parameter,
         * so a gained-ability activation (RB_GAINED_ABILITY_INDEX_BASE + n)
         * cannot be routed yet. card_id alone reaches handle_use_ability's
         * find_gained_activation fallback. */
        return rb_execute_main_phase_action(g, 0,
                                            a->has_card_id ? a->card_id : -1,
                                            g->active, -1, -1) == 0;
    }

    /* actions/mod.rs:267-271 — any other action while a choice is pending is
     * the choice's answer. */
    if (rb_has_pending_choice(g)) {
        int r;
        if (a->n_card_indices > 0) r = rb_resume_with_choice_indices(g, a->card_indices, a->n_card_indices);
        else r = rb_resume_with_choice(g, a->has_card_id ? a->card_id : 0);
        return r >= 0;
    }

    switch (a->type) {
        case RB_SETUP_ACT_PASS:
            /* Rust: LiveCardSetFirst/Second refill the hand from the live zone
             * before moving on; the C rb_advance_phase LIVE_SET arm performs
             * exactly that refill, and the second window is folded into the same
             * phase, so one advance is the faithful C equivalent. */
            rb_advance_phase(g);
            return 1;

        case RB_SETUP_ACT_MULLIGAN_HEADER:
        case RB_SETUP_ACT_LIVE_CARD_HEADER:
            return 1;   /* Ok(()) */

        case RB_SETUP_ACT_ROCK_CHOICE:
        case RB_SETUP_ACT_PAPER_CHOICE:
        case RB_SETUP_ACT_SCISSORS_CHOICE:
            return setup_handle_rps_choice(g, setup_rps_choice_value(a->type));

        case RB_SETUP_ACT_CHOOSE_FIRST_ATTACKER:
            /* Rust: rps_winner != Some(2). C: rps_winner == 2 means "P2 won",
             * 0 means no winner yet (Rust's None) — so both the tie and the
             * P1 win elect P1. */
            return rb_setup_choose_attacker(g, g->rps_winner != 2);

        case RB_SETUP_ACT_CHOOSE_SECOND_ATTACKER:
            return rb_setup_choose_attacker(g, g->rps_winner == 2);

        case RB_SETUP_ACT_SELECT_MULLIGAN:
            return rb_setup_handle_mulligan_selection(g, a->has_card_id, a->card_id,
                                                      a->card_indices, a->n_card_indices);

        case RB_SETUP_ACT_CONFIRM_MULLIGAN:
            return rb_setup_handle_mulligan_confirmation(g, a->card_indices,
                                                         a->n_card_indices) >= 0;

        case RB_SETUP_ACT_SKIP_MULLIGAN:
            return rb_setup_handle_mulligan_skip(g);

        case RB_SETUP_ACT_PLAY_MEMBER_TO_STAGE:
            return rb_handle_play_member_to_stage(g,
                                                  a->has_card_id ? a->card_id : -1,
                                                  a->card_indices, a->n_card_indices,
                                                  rb_setup_stage_area_from_parameters(a),
                                                  a->use_baton_touch) != 0;

        case RB_SETUP_ACT_SET_LIVE_CARD:
            return rb_handle_set_live_card(g, a->has_card_id ? a->card_id : -1) != 0;

        case RB_SETUP_ACT_SELECT_LIVE_CARD:
            return rb_handle_live_card_selection(g, a->has_card_id ? a->card_id : -1,
                                                 a->card_indices, a->n_card_indices) != 0;

        case RB_SETUP_ACT_CONFIRM_LIVE_CARD_SET:
            return rb_handle_live_card_confirmation(g, a->card_indices,
                                                    a->n_card_indices) != 0;

        case RB_SETUP_ACT_SKIP_LIVE_CARD_SET:
            return rb_handle_live_card_skip(g) != 0;

        case RB_SETUP_ACT_FINISH_LIVE_CARD_SET:
            /* Err("FinishLiveCardSet action is obsolete - use Pass instead") */
            return 0;

        default:
            /* actions/mod.rs:339 `_ => Ok(())` — DrawCard, EnergyCharge,
             * PassRemaining and the Choice* variants all land here. */
            return 1;
    }
}

/* game_setup.rs:514 execute_action_for_player — same, except a concede is
 * charged to the named seat instead of the active one. */
int rb_setup_execute_action_for_player(GameState *g, const RbSetupAction *a, int player_id) {
    if (!g || !a) return 0;
    if (a->type == RB_SETUP_ACT_CONCEDE) {
        int ok = rb_setup_concede(g, player_id);
        rb_setup_reset_loop_detection(g);
        return ok;
    }
    return rb_setup_execute_action(g, a);
}

/* ==========================================================================
 * Main-phase play decision — the rules content of game_setup.rs:1690
 * generate_main_phase_actions.
 * ==========================================================================*/

/* game_setup.rs:1540 stage_play_cost — the printed cost of playing `card_id`
 * including its constant cost modifiers (floored_cost), so the baton-touch
 * price agrees with core/player.rs. Returns 0 for a card with no cost.
 *
 * FIDELITY NOTE: Rust's `cost: Option<u8>` cannot distinguish "no cost field"
 * from "cost 0"; the C Card stores a bare uint8_t, so cost == 0 is read as
 * "no printed cost" (the same convention phase.c:1208 uses). */
int rb_setup_stage_play_cost(const GameState *g, int card_id) {
    if (!g || card_id < 0) return 0;
    Card card;
    memset(&card, 0, sizeof(card));
    if (!rb_card_get_card_by_id(card_id, &card)) return 0;
    int base = card.cost;
    int modifier = rb_mods_get_cost((RbMods *)&g->mods, card_id);
    rb_free_card(&card);
    if (base == 0) return 0;
    return setup_floored_cost(base, modifier);
}

/* game_setup.rs:1553 is_baton_touchable — the card currently occupies the area
 * and may legally be replaced this turn.
 * Rule 9.6.2.1.2.1: the check follows the MEMBER, not the area — a member
 * deployed this turn cannot be baton-touched again. */
int rb_setup_is_baton_touchable(const GameState *g, int pl, int card_id,
                                const int protection[RB_STAGE_SIZE], int slot) {
    if (!g || card_id == RB_EMPTY_SLOT) return 0;
    if (rb_card_arrived_this_turn(g, pl, card_id)) return 0;
    if (protection && slot >= 0 && slot < RB_STAGE_SIZE && protection[slot]) return 0;
    return 1;
}

/* game_setup.rs:1605 has_double_baton — does the card carry a play_baton_touch
 * effect that replaces MORE THAN ONE member at a time? */
int rb_setup_has_double_baton(int card_id) {
    if (card_id < 0) return 0;
    int n_abilities = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int hit = 0;
        if (ab.effect && ab.effect->action &&
            !strcmp(ab.effect->action, "play_baton_touch")) {
            /* `count.unwrap_or(1) > 1`; C spells "absent" as count < 0. */
            int count = ab.effect->count < 0 ? 1 : ab.effect->count;
            hit = count > 1;
        }
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* game_setup.rs:1569 double_baton_options — every ordered pair of
 * baton-touchable areas, one option per placement area. Empty when the card
 * cannot double-baton or no pair is affordable. Returns the row count. */
int rb_setup_double_baton_options(const GameState *g, int pl,
                                  const int stage_card_ids[RB_STAGE_SIZE],
                                  const int baton_protected[RB_STAGE_SIZE],
                                  int effective_cost, int active_energy_count,
                                  RbSetupDoubleBatonOption *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int occupied_slot[RB_STAGE_SIZE];
    int occupied_card[RB_STAGE_SIZE];
    int n_occupied = 0;
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        int card_id = stage_card_ids ? stage_card_ids[slot] : RB_EMPTY_SLOT;
        if (!rb_setup_is_baton_touchable(g, pl, card_id, baton_protected, slot)) continue;
        if (n_occupied >= RB_STAGE_SIZE) break;
        occupied_slot[n_occupied] = slot;
        occupied_card[n_occupied] = card_id;
        n_occupied++;
    }
    int n = 0;
    for (int i = 0; i < n_occupied; i++) {
        for (int j = i + 1; j < n_occupied; j++) {
            int replaced = rb_setup_stage_play_cost(g, occupied_card[i]) +
                           rb_setup_stage_play_cost(g, occupied_card[j]);
            int cost = effective_cost - replaced;
            if (cost < 0) cost = 0;                       /* saturating_sub */
            if (active_energy_count < cost) continue;     /* not affordable */
            for (int which = 0; which < 2 && n < max; which++) {
                out[n].area_index[0] = occupied_slot[i];
                out[n].area_index[1] = occupied_slot[j];
                out[n].placement_area = which == 0 ? occupied_slot[i] : occupied_slot[j];
                out[n].cost = cost;
                n++;
            }
        }
    }
    return n;
}

/* game_setup.rs:1635 area_candidates_for — for each stage area, what placing
 * `card` there would cost and whether it is legal. An empty area costs the
 * card's full reduced cost; an occupied one is a baton touch priced at the
 * card's cost minus the member it replaces. Returns 1 when any area is
 * available. */
int rb_setup_area_candidates_for(const GameState *g, int pl,
                                 const int stage_card_ids[RB_STAGE_SIZE],
                                 const int baton_protected[RB_STAGE_SIZE],
                                 int card_cost, int effective_cost,
                                 int active_energy_count,
                                 RbSetupAreaCandidate out[RB_STAGE_SIZE]) {
    if (!g || !out) return 0;
    int any_available = 0;
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        out[slot].slot = slot;
        out[slot].available = 0;
        out[slot].cost = card_cost;              /* initial value, per Rust */
        out[slot].is_baton_touch = 0;
        out[slot].existing_member_id = RB_EMPTY_SLOT;

        int existing_id = stage_card_ids ? stage_card_ids[slot] : RB_EMPTY_SLOT;
        if (existing_id == RB_EMPTY_SLOT) {
            /* Empty area: a normal debut, priced at the reduced cost. */
            if (active_energy_count >= effective_cost) {
                out[slot].available = 1;
                out[slot].cost = effective_cost;
                any_available = 1;
            }
            continue;
        }
        if (!rb_setup_is_baton_touchable(g, pl, existing_id, baton_protected, slot))
            continue;
        /* Rust: `let Some(existing_member) = stage_cards[slot] else { continue }`
         * — the member must exist in the card database. */
        Card existing;
        memset(&existing, 0, sizeof(existing));
        if (!rb_card_get_card_by_id(existing_id, &existing)) continue;
        rb_free_card(&existing);

        int cost = effective_cost - rb_setup_stage_play_cost(g, existing_id);
        if (cost < 0) cost = 0;
        if (active_energy_count < cost) continue;
        out[slot].available = 1;
        out[slot].cost = cost;
        out[slot].is_baton_touch = 1;
        out[slot].existing_member_id = existing_id;
        any_available = 1;
    }
    return any_available;
}

/* The whole per-hand-card loop of game_setup.rs:1766-2075, rules only: for
 * every member card in the active player's hand, the play cost after
 * reductions, the three area candidates and the double-baton table. Returns
 * the number of rows written (cards with no available area are still reported
 * with any_area_available == 0 so a caller can tell "unplayable" from
 * "absent", matching the Rust `has_any_available` flag). */
int rb_setup_play_candidates(const GameState *g, RbSetupPlayCandidate *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int pl = g->active;
    const RbPlayer *P = &g->p[pl];
    int hand_count = P->hand.n;
    int active_energy_count = P->energy_active;

    /* Stage snapshot, hoisted once per call exactly as Rust hoists it (the
     * restriction walk is the expensive part and cannot change mid-loop). */
    int stage_card_ids[RB_STAGE_SIZE];
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) stage_card_ids[slot] = P->stage[slot];

    /* Rule 7.7.2.2 — a 「プレイ Pamplona」 prohibition withholds every
     * member-to-stage play. */
    if (rb_is_action_prohibited(g, "play_member")) return 0;

    int n = 0;
    for (int hand_index = 0; hand_index < P->hand.n && n < max; hand_index++) {
        int card_id = P->hand.cards[hand_index];
        if (!rb_card_is_member(card_id) || rb_card_is_live(card_id)) continue;

        Card card;
        memset(&card, 0, sizeof(card));
        if (!rb_card_get_card_by_id(card_id, &card)) continue;

        RbSetupPlayCandidate *row = &out[n];
        memset(row, 0, sizeof(*row));
        row->card_id = card_id;
        row->hand_index = hand_index;
        row->card_cost = card.cost;
        row->reduction = rb_calculate_play_cost_reduction(g, pl, hand_count, card_id);
        row->effective_cost = row->card_cost - row->reduction;
        if (row->effective_cost < 0) row->effective_cost = 0;  /* saturating_sub */

        /* Per-area baton-touch protection. Only the group test genuinely
         * depends on the incoming card, which is why Rust hoists the ability
         * walk to the stage. */
        int baton_protected[RB_STAGE_SIZE];
        for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
            int existing = stage_card_ids[slot];
            baton_protected[slot] = (existing != RB_EMPTY_SLOT) &&
                !rb_card_arrived_this_turn(g, pl, existing) &&
                rb_card_has_restriction(g, card_id, existing, "cannot_baton_touch");
        }

        row->any_area_available = rb_setup_area_candidates_for(
            g, pl, stage_card_ids, baton_protected,
            row->card_cost, row->effective_cost, active_energy_count, row->areas);

        if (rb_setup_has_double_baton(card_id))
            row->n_pairs = rb_setup_double_baton_options(
                g, pl, stage_card_ids, baton_protected,
                row->effective_cost, active_energy_count,
                row->pairs, RB_SETUP_MAX_DOUBLE_BATON);

        rb_free_card(&card);
        n++;
    }
    return n;
}

/* ==========================================================================
 * Main-phase ability offer — the rules content of game_setup.rs:2078-2348
 * (the stage scan and the discard / 控え室 scan).
 * ==========================================================================*/

/* card.rs:1432 energy_cost_total — printed energy total of a cost block, with
 * NO per-group reduction (that is effective_energy_cost_total / the C
 * rb_effective_activation_cost_for). Sums the sequential/compound children. */
static int setup_cost_energy_total(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) {
        int printed = e->count < 0 ? 0 : rb_saturate_u8(e->count);
        const char *energy = setup_effect_extra(e, "energy_count");
        if (!energy || !*energy) energy = setup_effect_extra(e, "energy");
        if (energy && *energy) {
            char *end = NULL;
            long value = strtol(energy, &end, 10);
            if (*energy && !*end && value >= 0 && value <= 255) printed = (int)value;
        }
        return printed;
    }
    int total = 0;
    for (int i = 0; i < e->n_child; i++)
        total = rb_saturate_u8(total + setup_cost_energy_total(e->child[i]));
    return total;
}

/* card.rs:1447 has_optional_payment — ANY pay_energy component is optional, so
 * the ability stays offered even when the energy is short (rule 9.6.2.3 /
 * Q228: the player may skip just that part). */
static int setup_cost_has_optional_payment(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy") && e->is_optional) return 1;
    for (int i = 0; i < e->n_child; i++)
        if (setup_cost_has_optional_payment(e->child[i])) return 1;
    return 0;
}

/* ability/util.rs:112 ability_trigger_can_activate — Main / Activation /
 * (Auto WITH a cost) / BatonTouch make an ability activatable in the main
 * phase. */
static int setup_ability_trigger_can_activate(const Ability *ab) {
    if (!ab) return 0;
    return rb_ability_has_trigger(ab, RB_TK_MAIN)
        || rb_ability_has_trigger(ab, RB_TK_ACTIVATION)
        || (rb_ability_has_trigger(ab, RB_TK_AUTO) && ab->cost != NULL)
        || rb_ability_has_trigger(ab, RB_TK_BATON_TOUCH);
}

/* ability/util.rs:83 ability_activation_gate_holds — 「…場合のみ起動できる」.
 * C's rb_can_activate_effect (resolver.c:644) is the faithful fold of this
 * (activation_condition gate + the position overlay the offer path shares with
 * the resolve path). `card_id` is the activating card, which is what the
 * condition is about (resolver.c passes the same host through
 * rb_eval_condition_for_host). */
static int setup_ability_activation_gate_holds(const GameState *g, int actor,
                                               const Ability *ab, int card_id) {
    if (!g || !ab) return 1;
    if (!ab->effect) return 1;
    return rb_can_activate_effect(g, actor, ab->effect, card_id);
}

/* ability/util.rs:136 activates_from_discard — activation_condition_parsed
 * with location = discard. The stage scan SKIPS these; the discard scan
 * REQUIRES them. */
static int setup_ability_activates_from_discard(const Ability *ab) {
    if (!ab || !ab->effect || !ab->effect->activation_condition) return 0;
    const char *loc = rb_cond_get_str(ab->effect->activation_condition, "location");
    if (!loc) return 0;
    return !strcmp(loc, "discard") || !strcmp(loc, "waitroom");
}

/* The cost gates shared by both scans (game_setup.rs:2164-2197 / 2277-2304):
 *   - a MANDATORY cost the player cannot pay withholds the offer (rule
 *     9.6.2.3); an optional-payment cost keeps it offered;
 *   - mandatory non-energy costs (discard / wait / ...) must also validate.
 * Returns 1 when the ability may be offered. */
static int setup_ability_cost_offered(const GameState *g, int actor,
                                      const Ability *ab, int stage_groups,
                                      int active_energy_count,
                                      int *out_base, int *out_effective) {
    int base = 0, effective = 0;
    if (ab->cost) {
        base = setup_cost_energy_total(ab->cost);
        effective = rb_effective_activation_cost_for(g, actor, ab->cost, stage_groups);
    }
    if (out_base) *out_base = base;
    if (out_effective) *out_effective = effective;
    if (!ab->cost) return 1;
    if (!setup_cost_has_optional_payment(ab->cost) && effective > active_energy_count)
        return 0;
    if (!rb_validate_cost(g, actor, ab->cost)) return 0;
    return 1;
}

/* Both scans at once (stage slots 0..2, then the 控え室). Printed abilities AND
 * runtime-gained ones on the stage (a gained 起動 is a real activation and must
 * be offered on the same terms); printed abilities only in the discard.
 * Returns the number of offers written. */
int rb_setup_ability_candidates(const GameState *g, RbSetupAbilityCandidate *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int pl = g->active;
    const RbPlayer *P = &g->p[pl];
    int active_energy_count = P->energy_active;
    int stage_groups = rb_distinct_stage_groups(g, pl);
    int n = 0;

    /* ── stage scan (game_setup.rs:2079-2250) ── */
    for (int slot = 0; slot < RB_STAGE_SIZE && n < max; slot++) {
        int card_id = P->stage[slot];
        if (card_id == RB_EMPTY_SLOT) continue;

        /* Printed abilities PLUS runtime-gained ones. A gained 起動 is a real
         * activation (handle_use_ability's find_gained_activation resolves it),
         * so it must be OFFERED on the same terms; the shared
         * RB_GAINED_ABILITY_INDEX_BASE + gained_idx encoding is what the
         * resolve path decodes back. rb_resolve_card_ability_at is the ONE
         * entry point that understands both index spaces. */
        int n_printed = rb_card_num_abilities((uint32_t)card_id);
        int n_gained = rb_card_num_gained_abilities(g, card_id);
        for (int i = 0; i < n_printed + n_gained && n < max; i++) {
            int ability_index = (i < n_printed)
                ? i
                : RB_GAINED_ABILITY_INDEX_BASE + (i - n_printed);
            Ability ab;
            memset(&ab, 0, sizeof(ab));
            if (!rb_resolve_card_ability_at(g, card_id, ability_index, &ab)) continue;

            int offer = 1;
            int base = 0, effective = 0;
            if (!setup_ability_trigger_can_activate(&ab)) offer = 0;
            else if (!setup_ability_activation_gate_holds(g, pl, &ab, card_id)) offer = 0;
            else if (!rb_check_trigger_position(ab.triggers, slot)) offer = 0;
            else if (!rb_check_effect_position(
                         setup_effect_extra(ab.effect, "activation_position"), slot)) offer = 0;
            else if (setup_ability_activates_from_discard(&ab)) offer = 0;
            else if (ab.use_limit >= 0 &&
                     rb_resolver_use_limit_reached(g, card_id, ability_index, ab.use_limit))
                offer = 0;
            else if (!setup_ability_cost_offered(g, pl, &ab, stage_groups,
                                                 active_energy_count, &base, &effective))
                offer = 0;

            if (offer) {
                out[n].card_id = card_id;
                out[n].ability_index = ability_index;
                out[n].from_stage = 1;
                out[n].stage_slot = slot;
                out[n].base_cost = base;
                out[n].effective_cost = effective;
                out[n].groups_on_stage = stage_groups;
                out[n].has_optional_payment = ab.cost ? setup_cost_has_optional_payment(ab.cost) : 0;
                n++;
            }
            rb_free_ability(&ab);
        }
    }

    /* ── discard (控え室) scan — game_setup.rs:2254-2348 ── */
    for (int i = 0; i < P->discard.n && n < max; i++) {
        int card_id = P->discard.cards[i];
        int n_printed = rb_card_num_abilities((uint32_t)card_id);
        for (int a = 0; a < n_printed && n < max; a++) {
            Ability ab;
            memset(&ab, 0, sizeof(ab));
            if (!rb_resolve_card_ability_at(g, card_id, a, &ab)) continue;

            int offer = 1;
            int base = 0, effective = 0;
            if (!setup_ability_activates_from_discard(&ab)) offer = 0;
            else if (!setup_ability_trigger_can_activate(&ab)) offer = 0;
            else if (!setup_ability_activation_gate_holds(g, pl, &ab, card_id)) offer = 0;
            else if (ab.use_limit >= 0 &&
                     rb_resolver_use_limit_reached(g, card_id, a, ab.use_limit))
                offer = 0;
            else if (!setup_ability_cost_offered(g, pl, &ab, stage_groups,
                                                 active_energy_count, &base, &effective))
                offer = 0;

            if (offer) {
                out[n].card_id = card_id;
                out[n].ability_index = a;
                out[n].from_stage = 0;
                out[n].stage_slot = RB_EMPTY_SLOT;
                out[n].base_cost = base;
                out[n].effective_cost = effective;
                out[n].groups_on_stage = stage_groups;
                out[n].has_optional_payment = ab.cost ? setup_cost_has_optional_payment(ab.cost) : 0;
                n++;
            }
            rb_free_ability(&ab);
        }
    }
    return n;
}

/* ── phase.c bridge ──────────────────────────────────────────────────────
 * rb_generate_action_candidates (src/turn/phase.c) is this file's Main-phase
 * action generator — the C twin of Rust's generate_main_phase_actions — and
 * its ability half IS the scan above; before this bridge existed it had no
 * caller in the tree, so an activated ability could never be offered.
 *
 * RbSetupAbilityCandidate is private to this file and include/rabuka.h is not
 * this file's to edit, so the offer table crosses the boundary through a sink
 * the caller supplies. The rules stay here, the action list stays in phase.c,
 * and the row layout is NOT duplicated across the two.
 *
 * ORDER (game_setup.rs:1694-2350): Pass, then the play-member offers inside
 * the `!is_action_prohibited("play_member")` guard, then these stage offers,
 * then the discard offers. The caller therefore invokes this AFTER its
 * play-member loop and OUTSIDE that guard — Rust closes the guard at :2076,
 * before the first activation scan at :2078.
 *
 * Returns the number of offers seen; a non-zero `sink` return stops the scan
 * early (the caller uses that to abort on allocation failure). */
int rb_setup_for_each_ability_candidate(
    const GameState *g,
    int (*sink)(void *ctx, int card_id, int ability_index, int from_stage,
                int stage_slot, int base_cost, int effective_cost),
    void *ctx) {
    if (!g || !sink) return 0;
    RbSetupAbilityCandidate offers[RB_SETUP_MAX_ABILITIES];
    int n = rb_setup_ability_candidates(g, offers, RB_SETUP_MAX_ABILITIES);
    if (n > RB_SETUP_MAX_ABILITIES) n = RB_SETUP_MAX_ABILITIES;
    for (int i = 0; i < n; i++)
        if (sink(ctx, offers[i].card_id, offers[i].ability_index,
                 offers[i].from_stage, offers[i].stage_slot,
                 offers[i].base_cost, offers[i].effective_cost))
            return i + 1;
    return n;
}

/* ==========================================================================
 * Live-card-set selection — the limit rule from game_setup.rs:2378-2388 and
 * :850-870 (phases.rs::handle_live_card_selection).
 * max(0, MAX_LIVE_CARDS − live_card_set_limit_reduction).
 * ==========================================================================*/
int rb_setup_live_card_set_max(const GameState *g, int pl) {
    int max = RB_MAX_LIVE_CARDS - (g ? g->live_set_limit_reduction[pl] : 0);
    return max < 0 ? 0 : max;
}

/* One row per hand card: `selectable` is false once the limit is reached and
 * the card is not already selected (those rows are withheld entirely by the
 * Rust generator). */
int rb_setup_live_card_candidates(const GameState *g, RbSetupHandCandidate *out, int max) {
    if (!g || !out || max <= 0) return 0;
    const RbPlayer *P = &g->p[g->active];
    int already_selected = g->n_selected_cards;
    int max_allowed = rb_setup_live_card_set_max(g, g->active);
    int at_limit = already_selected >= max_allowed;
    int n = 0;
    for (int hand_index = 0; hand_index < P->hand.n && n < max; hand_index++) {
        int card_id = P->hand.cards[hand_index];
        int is_selected = setup_index_selected(g, hand_index);
        if (at_limit && !is_selected) continue;      /* withheld entirely */
        out[n].hand_index = hand_index;
        out[n].card_id = card_id;
        out[n].selected = is_selected;
        out[n].selectable = 1;
        n++;
    }
    return n;
}

/* ==========================================================================
 * Deck construction — game_setup.rs:338 build_two_decks.
 *
 * Rust: two DeckBuilder::build_deck_from_database passes, each followed by
 * add_default_energy_cards_from_database, with the per-side error prefixed
 * "P1 deck: " / "P2 deck: ". The C deck builder (src/game/deck_builder.c,
 * owned by another agent) already folds the default-energy step into
 * rb_build_deck_from_card_numbers, so this is the two-call wrapper with the
 * Rust error prefixes — no card-database work is duplicated here.
 * Returns 0 on success; on failure writes "P1 deck: <status>" / "P2 deck: ..."
 * into `err` and returns the build status.
 * ==========================================================================*/
int rb_setup_build_two_decks(const char *const *n1, size_t n1_count,
                             const char *const *n2, size_t n2_count,
                             RbBuiltDeck *deck1, RbBuiltDeck *deck2,
                             char *err, size_t err_size) {
    int rc = rb_build_deck_from_card_numbers(n1, n1_count, NULL, 0, deck1);
    if (rc != RB_DECK_BUILD_OK) {
        if (err && err_size)
            snprintf(err, err_size, "P1 deck: %s", rb_setup_deck_build_status_str(rc));
        return rc;
    }
    rc = rb_build_deck_from_card_numbers(n2, n2_count, NULL, 0, deck2);
    if (rc != RB_DECK_BUILD_OK) {
        if (err && err_size)
            snprintf(err, err_size, "P2 deck: %s", rb_setup_deck_build_status_str(rc));
        return rc;
    }
    return RB_DECK_BUILD_OK;
}

/* deck_builder.h's status codes have no to_str in the C port; the Rust side
 * formats the underlying error, so a short tag is the faithful minimum here.
 * XREQ: prefer rb_deck_build_status_str() in deck_builder.c if one is added. */
const char *rb_setup_deck_build_status_str(int status) {
    switch (status) {
        case RB_DECK_BUILD_OK:        return "ok";
        case RB_DECK_BUILD_EINVAL:    return "invalid argument";
        case RB_DECK_BUILD_EFORMAT:   return "bad deck format";
        case RB_DECK_BUILD_EOVERFLOW: return "deck overflow";
        case RB_DECK_BUILD_ENOMEM:    return "out of memory";
        default:                      return "unknown error";
    }
}

/* ==========================================================================
 * deck_ordering.rs — Series/Rarity ordering + optimal card sort.
 *
 * FAITHFULNESS NOTE: this is deck ORDERING for the GBA deck builder / web card
 * browser ("minimises D-pad presses"), not a rule: nothing here touches game
 * state and no engine path calls it. It is ported because it is part of this
 * file's assignment and is pure, self-contained string/sort logic — but it is
 * presentation-support code, not gameplay.
 * ==========================================================================*/

/* deck_ordering.rs:16 SERIES_ORDER (order by popularity to minimise presses) */
static const char *const RB_SETUP_SERIES_ORDER[] = {
    "PL!",     /* μ's (most common) */
    "PL!SP",   /* μ's Special */
    "PL!S",    /* Aqours */
    "PL!N",    /* 虹ヶ咲 */
    "PL!HS",   /* Liella! */
    "LL",      /* 蓮ノ空 */
    "PR",      /* Promo */
    "Other"    /* everything else */
};
/* deck_ordering.rs:29 RARITY_ORDER (order by frequency in competitive decks) */
static const char *const RB_SETUP_RARITY_ORDER[] = {
    "N", "N＋",
    "R", "R＋",
    "SR", "SR＋",
    "SEC",
    "P", "P＋",
    "PR", "PR＋",
    "L",
    "PE＋",
    "SECL",
    "SRE",
    "SD", "SD2",
    "L+", "LLE",
    "AR", "RM",
    "SECE", "SECL"
};

size_t rb_setup_series_priority(const char *series) {
    size_t n = sizeof(RB_SETUP_SERIES_ORDER) / sizeof(RB_SETUP_SERIES_ORDER[0]);
    if (series)
        for (size_t i = 0; i < n; i++)
            if (!strcmp(RB_SETUP_SERIES_ORDER[i], series)) return i;
    return n;   /* unwrap_or(SERIES_ORDER.len()) */
}

size_t rb_setup_rarity_priority(const char *rarity) {
    size_t n = sizeof(RB_SETUP_RARITY_ORDER) / sizeof(RB_SETUP_RARITY_ORDER[0]);
    if (rarity)
        for (size_t i = 0; i < n; i++)
            if (!strcmp(RB_SETUP_RARITY_ORDER[i], rarity)) return i;
    return n;
}

/* Copy the first `len` bytes of `src` into a small rotating buffer. C has no
 * borrowed &str, so the two Rust slice-returning extractors are served from
 * this ring (documented lifetime: valid until RB_SETUP_STR_RING further calls). */
#define RB_SETUP_STR_RING 8
#define RB_SETUP_STR_MAX   64
static char rb_setup_str_ring[RB_SETUP_STR_RING][RB_SETUP_STR_MAX];
static int rb_setup_str_next;

static const char *rb_setup_str_slice(const char *src, size_t len) {
    char *buf = rb_setup_str_ring[rb_setup_str_next];
    rb_setup_str_next = (rb_setup_str_next + 1) % RB_SETUP_STR_RING;
    if (len >= RB_SETUP_STR_MAX) len = RB_SETUP_STR_MAX - 1;
    memcpy(buf, src, len);
    buf[len] = '\0';
    return buf;
}

/* deck_ordering.rs:57 extract_series — "PL!-BP1-001-R" -> "PL!" */
const char *rb_setup_extract_series(const char *card_no) {
    if (!card_no) return "";
    const char *dash = strchr(card_no, '-');
    if (!dash) return card_no;                 /* split('-').next() == whole */
    return rb_setup_str_slice(card_no, (size_t)(dash - card_no));
}

/* deck_ordering.rs:62 extract_rarity — "PL!-BP1-001-R" -> "R" */
const char *rb_setup_extract_rarity(const char *card_no) {
    if (!card_no || !*card_no) return "";
    const char *last = strrchr(card_no, '-');
    return last ? last + 1 : card_no;
}

/* deck_ordering.rs:67 extract_base — drop the rarity suffix:
 * "PL!-BP1-001-R" -> "PL!-BP1-001" (the whole string when there is no '-'). */
char *rb_setup_extract_base(const char *card_no) {
    if (!card_no) return NULL;
    const char *last = strrchr(card_no, '-');
    if (!last) return rb_strdup2(card_no);
    size_t len = (size_t)(last - card_no);
    char *out = (char *)rb_malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, card_no, len);
    out[len] = '\0';
    return out;
}

/* deck_ordering.rs:78 gba_sort_key — the key is
 * (series_priority, rarity_priority, card_no); the first two components are
 * reported through the out-params (the third is the card number itself, which
 * the comparator uses directly). */
void rb_setup_gba_sort_key(const char *card_no, size_t *series_out, size_t *rarity_out) {
    size_t sp = rb_setup_series_priority(rb_setup_extract_series(card_no));
    size_t rp = rb_setup_rarity_priority(rb_setup_extract_rarity(card_no));
    if (series_out) *series_out = sp;
    if (rarity_out) *rarity_out = rp;
}

/* deck_ordering.rs:85 sort_deck_for_gba. qsort is not stable, but the key's
 * third component is the card number itself, so equal keys are equal rows. */
static int setup_gba_cmp(const void *pa, const void *pb) {
    const char *const *a = (const char *const *)pa;
    const char *const *b = (const char *const *)pb;
    size_t as = 0, ar = 0, bs = 0, br = 0;
    rb_setup_gba_sort_key(*a, &as, &ar);
    rb_setup_gba_sort_key(*b, &bs, &br);
    if (as != bs) return as < bs ? -1 : 1;
    if (ar != br) return ar < br ? -1 : 1;
    return strcmp(*a, *b);
}

void rb_setup_sort_deck_for_gba(char **card_nos, size_t n) {
    if (!card_nos || n < 2) return;
    qsort(card_nos, n, sizeof(char *), setup_gba_cmp);
}

/* deck_ordering.rs:90 flatten_and_sort_for_gba — expand by quantity, then sort
 * for GBA navigation. The returned array borrows the input strings (like the
 * Rust Vec<String> does per clone), so release it with rb_free only. */
char **rb_setup_flatten_and_sort_for_gba(const char *const *card_nos,
                                         const uint8_t *qty, size_t n,
                                         size_t *out_count) {
    size_t total = 0;
    for (size_t i = 0; i < n; i++) total += qty ? (size_t)qty[i] : 1u;
    char **flat = (char **)rb_malloc(total ? total * sizeof(char *) : sizeof(char *));
    if (!flat) { if (out_count) *out_count = 0; return NULL; }
    size_t k = 0;
    for (size_t i = 0; i < n; i++) {
        int copies = qty ? (int)qty[i] : 1;
        for (int c = 0; c < copies && k < total; c++)
            flat[k++] = (char *)card_nos[i];
    }
    rb_setup_sort_deck_for_gba(flat, k);
    if (out_count) *out_count = k;
    return flat;
}
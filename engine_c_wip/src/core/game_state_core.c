/* ════════════════════════════════════════════════════════════════════════
   game_state_core.c — port of engine/src/core/game_state/mod.rs
   (the `impl GameState` block, mod.rs:391-1713)

   Scope: the GameState BEHAVIOUR that has no home elsewhere in the C port —
   player/seat/attacker queries, phase + turn counters, cost-paid recording
   hooks, effect-tracking clears, the tracking accessors (recently-moved,
   last-area-move, last-cost-wait, last-energy-placed), the revealed-card
   pushers, the choice label/count formatters, the zone privacy + index
   resolvers, and the rule-log / snapshot / ability-application writers.

   NOT ported here (deliberately, reported instead):
     · structured_log / debug_trace rings   — no GameState field
     · last_ability_trace                  — no GameState field
     · current_turn_phase                  — no GameState field (derived here)

   ⛔ CORRECTION 2026-10-04 (field census). This list USED to also claim, in these
   exact words, that three more items had "no GameState field":
     · cards_paid_cost_this_turn            — no GameState field (see report)
     · player1/2_cheer_revealed_cards       — no GameState field
     · revealed_cost_cards / *_card_meta    — no GameState field
   All of that was FALSE and has been deleted. Every one of those members EXISTS
   and is declared in include/rabuka.h: `cards_paid_cost_this_turn[]` +
   `n_cards_paid_cost_this_turn` (the tracking / play-time-alt-cost block),
   `player1_cheer_revealed_cards[]` + `player2_cheer_revealed_cards[]` and their two
   counters, and `revealed_cost_cards[]` + `n_revealed_cost_cards`. What was
   actually true is narrower and is now the whole statement: they are DECLARED BUT
   UNWIRED — 0 writers and 0 readers. Each one now carries its own measured
   BLOCKED-ON in the header, naming the exact file:line of the missing producer or
   consumer; `*_card_meta` is the one genuine absence (Rust's
   `revealed_cost_card_meta` is display-only). A comment that says "no field exists"
   when the field exists and is simply not wired sends the next reader looking for a
   declaration instead of at the call site, which is where the work is.

   ⛔ rule_log IS WIRED (2026-10-03). `GameState.rule_log[500][256]` +
   `rule_log_len[]` + `n_rule_log` landed in include/rabuka.h, and the four
   logging entry points below now append to it through gsc_rule_log_push().
   They used to route into rb_log_push_verdict(text,"rule_log",1), which is the
   CONDITION-VERDICT ring (ability/log.c, 32 slots, drop-newest): the two logs
   destroyed each other's entries. That routing is GONE.

   REACHABILITY, MEASURED (2026-10-02). 50 rb_gsc_* definitions; exactly ONE
   has an external caller in the whole tree:

       rb_gsc_concede          <- game_setup.c:764 (rb_setup_concede delegates)
       └─ rb_gsc_reject_if_ended  <- rb_gsc_concede + rb_gsc_concede_both
          └─ rb_gsc_game_has_ended

   The other 47 are externally unreachable (47 = 50 definitions - the 3
   reachable ones above). That is NOT evidence they are
   wrong: for most of them the Rust twin IS live and the matching C call site
   lives in a translation unit this file's owner does not write (resolver.c,
   phase.c, live.c, game_state_abilities.c, web_server.c, ...), so the gap is
   a wiring request, not a porting defect. A handful are genuine duplicates of
   a live twin in another file and are pure drift hazard. The full
   WIRE/DELETE/REPORT table for every one is in the handover report.

   ⛔ THE LOGGING GROUP MUST NOT REGRESS INTO THE VERDICT RING. Rust's
   push_rule_log / push_rule_log_fmt feed `rule_log: Vec<String>` ONLY
   (mod.rs:1526-1540). rb_log_push_verdict is the CONDITION-VERDICT buffer
   (ability/log.c: 32 slots, drop-newest). The one remaining offender is
   rb_gsc_push_rule_log_fmt, whose DECLARED signature takes no GameState* and so
   structurally cannot reach g->rule_log — see its body for the exact
   include/rabuka.h line that closes it. Every other entry point here goes
   through gsc_rule_log_push. current_turn_phase must NOT be added (it is
   derivable from g->phase + first/second_attacker, and adding it makes a second
   source of truth).

   Every exported symbol is prefixed rb_gsc_ so this file can be added to the
   build without colliding with any existing symbol.

   Helpers reused rather than reimplemented (do NOT re-derive these here):
     rb_zone_cards                (ability/util.c)   — zone id enumeration
     rb_card_get_card_names       (core/card.c)      — card name lookup
rb_find_card_stage_position  (ability/resolver.c)
      rb_get_pending_choice        (ability/choice.c)
      rb_get_pending_choice_player_id (ability/choice.c)
      rb_mods_trace_push           (core/modifiers.c) — ability-application ring
      rb_ability_debug_enabled     (ability/debug.c) — the rule-log gate (below)
      rb_opponent_id               (core/…)           — seat arithmetic
   ════════════════════════════════════════════════════════════════════════ */

#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdarg.h>

/* ── CROSS-FILE REQUEST: new public types/functions this file needs ────────

   RbGscZoneSnapshot  (below) is the C form of Rust's
   `GameState::zone_snapshot() -> HashMap<String, usize>`. The C port's
   existing RbZoneSnapshot is a DIFFERENT shape (types.rs ZoneSnapshot:
   6 aggregate counters for the WHOLE state), so it cannot carry the 16
   per-player zone keys the Rust map holds. Please declare:

       typedef struct {
           const char *key;   // one of "P1.hand" .. "P2.success_live"
           int         count;
       } RbGscZoneSnapshot;
       #define RB_GSC_ZONE_SNAPSHOT_KEYS 16
       int rb_gsc_zone_snapshot(const GameState *g, RbGscZoneSnapshot *out);

   plus declarations for every rb_gsc_* symbol below.

   ── Helpers this file declares locally (already in the tree, NOT in
      rabuka.h — wire them in or keep the local externs):
       int rb_phase_is_live_phase(int phase);   (core/data.c)
   ───────────────────────────────────────────────────────────────────────── */

/* ── Rust: Phase (engine/src/core/types.rs::Phase) ──────────────────────────
 *   The C RbPhase enum collapses Rust's 13 variants into 12 (there is no
 *   separate ChooseFirstAttacker; RB_PHASE_RPS covers the RPS->first-attacker
 *   step, with g->rps_winner carrying the outcome). The mapping is total:
 *   every C phase has a Rust counterpart.
 *
 *   RbGscTurnPhase MOVED to include/rabuka.h on 2026-10-03. It used to be a local
 *   typedef here, which meant the only way to declare
 *   rb_gsc_current_turn_phase() in the header was not to - the file compiled by
 *   accident rather than by contract. */

/* RbGscZoneSnapshot and RB_GSC_ZONE_SNAPSHOT_KEYS now live in include/rabuka.h;
   the local copy that used to sit here was removed once the header landed. */

int rb_phase_is_live_phase(int phase); /* core/data.c — defined there, not in rabuka.h */

/* Mirrors Phase::is_live_phase (types.rs:1310-1314). Kept as a thin wrapper so
   the mapping is stated once; rb_phase_is_live_phase already encodes exactly
   this set, so this is a translation of the match, not a second source. */
static int gsc_phase_is_live(int phase) {
    switch (phase) {
    case RB_PHASE_LIVE_SET:
    case RB_PHASE_LIVE_SET_SECOND:
    case RB_PHASE_PERFORMANCE:
    case RB_PHASE_PERFORMANCE_SECOND:
    case RB_PHASE_VICTORY:
        return 1;
    default:
        return rb_phase_is_live_phase(phase) ? 1 : 0;
    }
}

/* ── player ids ────────────────────────────────────────────────────────────
   Rust stores `Player.id` ("p1"/"p2") on the player and every
   by-id accessor compares against it. The C RbPlayer has no id field; the
   whole C port addresses players by seat index and spells the id "p1"/"p2"
   wherever a string id is needed (ability/cost.c:1631 uses the same
   convention). One definition here. */
static const char *gsc_player_id(int seat) { return seat == 0 ? "p1" : "p2"; }

/* Rust normalises "player1"/"player2" down to "p1"/"p2" before every id
   comparison (can_player_act:506-508, 533-536) and leaves any other spelling
   untouched. */
static int gsc_id_is(int seat, const char *id) {
    if (!id) return 0;
    if (!strcmp(id, "player1")) id = "p1";
    else if (!strcmp(id, "player2")) id = "p2";
    return strcmp(id, gsc_player_id(seat)) == 0;
}

/* ── first / second attacker ───────────────────────────────────────────────
   Rust: `if player1.is_first_attacker { player1 } else { player2 }`.
   The C GameState stores the resolved seats directly in
   g->first_attacker / g->second_attacker, so these are the faithful form of
   the Rust guard (rb_game_init keeps the two in sync with the RPS winner). */
int rb_gsc_first_attacker_index(const GameState *g);
int rb_gsc_second_attacker_index(const GameState *g);
int rb_gsc_active_player_index(const GameState *g);

int rb_gsc_first_attacker_index(const GameState *g) { return g ? g->first_attacker : 0; }
int rb_gsc_second_attacker_index(const GameState *g) { return g ? g->second_attacker : 1; }

/* ── active_player ──────────────────────────────────────────────────────────
   Rust: mod.rs:737-753. Phase first (the Mulligan / LiveCardSet / Performance
   phases pin the seat explicitly), falling back to current_turn_phase. The C
   port has no stored current_turn_phase field, so the fallback arm derives it
   from the live-phase set (a normal turn phase means g->phase is
   Active/Energy/Draw/Main, none of which appear in the explicit arms above). */
int rb_gsc_active_player_index(const GameState *g) {
    if (!g) return 0;
    switch (g->phase) {
    case RB_PHASE_MULLIGAN_FIRST:
    case RB_PHASE_LIVE_SET:
    case RB_PHASE_PERFORMANCE:
    case RB_PHASE_VICTORY:
        return g->first_attacker;   /* MulliganFirstAttacker | LiveCardSetFirst | FirstPerf | LiveVictory */
    case RB_PHASE_MULLIGAN_SECOND:
    case RB_PHASE_LIVE_SET_SECOND:
    case RB_PHASE_PERFORMANCE_SECOND:
        return g->second_attacker;  /* MulliganSecondAttacker | LiveCardSetSecond | SecondPerf */
    default:
        /* TurnPhase::FirstAttackerNormal => first, SecondAttackerNormal =>
         * second, Live => first. The Live case is already covered by the
         * explicit arms above (every live phase is listed), so a non-live
         * phase here is a NORMAL phase; which attacker owns it is carried by
         * g->first_attacker / g->second_attacker. */
        return g->first_attacker;
    }
}

RbPlayer *rb_gsc_active_player(GameState *g) { return g ? &g->p[rb_gsc_active_player_index(g)] : NULL; }

/* ── is_first_turn ──────────────────────────────────────────────────────────
   Rust: mod.rs:491-493 — DERIVED from turn_number, never a stored bool
   (it used to be a field initialised true and never written, so it read true
   for the whole game). */
int rb_gsc_is_first_turn(const GameState *g) { return g && g->turn == 1; }

/* ── phase_invariant ────────────────────────────────────────────────────────
   Rust: mod.rs:575-594. The C GameState has no `current_turn_phase` field,
   so it is derived (RB_PHASE_ACTIVE/ENERGY/DRAW/MAIN => normal; the live set
   => Live; everything else => the pre-game phases, which short-circuit to
   true in the first arm anyway). */
RbGscTurnPhase rb_gsc_current_turn_phase(const GameState *g) {
    switch (g ? g->phase : RB_PHASE_ACTIVE) {
    case RB_PHASE_LIVE_SET:
    case RB_PHASE_LIVE_SET_SECOND:
    case RB_PHASE_PERFORMANCE:
    case RB_PHASE_PERFORMANCE_SECOND:
    case RB_PHASE_VICTORY:
        return RB_GSC_TP_LIVE;
    case RB_PHASE_ACTIVE:
    case RB_PHASE_ENERGY:
    case RB_PHASE_DRAW:
    case RB_PHASE_MAIN:
        /* Which attacker owns a normal turn phase is the seat the engine
         * rolled over to; g->active is that seat. */
        return (g && g->active == g->second_attacker) ? RB_GSC_TP_SECOND_ATTACKER_NORMAL
                                                      : RB_GSC_TP_FIRST_ATTACKER_NORMAL;
    default:
        return RB_GSC_TP_FIRST_ATTACKER_NORMAL;
    }
}

int rb_gsc_phase_invariant(const GameState *g) {
    if (!g) return 0;
    switch (g->phase) {
    case RB_PHASE_RPS:                 /* RockPaperScissors */
    case RB_PHASE_OPENING:             /* ChooseFirstAttacker — a distinct C phase; Rust mod.rs:577-582 lists it in this same early-true group */
    case RB_PHASE_MULLIGAN_FIRST:      /* MulliganFirstAttacker */
    case RB_PHASE_MULLIGAN_SECOND:     /* MulliganSecondAttacker */
        return 1;
    default:
        break;
    }
    switch (rb_gsc_current_turn_phase(g)) {
    case RB_GSC_TP_FIRST_ATTACKER_NORMAL:
    case RB_GSC_TP_SECOND_ATTACKER_NORMAL:
        return g->phase == RB_PHASE_ACTIVE || g->phase == RB_PHASE_ENERGY ||
               g->phase == RB_PHASE_DRAW  || g->phase == RB_PHASE_MAIN;
    case RB_GSC_TP_LIVE:
    default:
        return gsc_phase_is_live(g->phase);
    }
}

/* ── can_player_act ────────────────────────────────────────────────────────
   Rust: mod.rs:495-573. Pending choices are authoritative (the queue stamps
   who must answer); the phase match is the fallback. The C queue stamps the
   seat in g->queue.actor (rb_get_pending_choice_player_id), and the flat
   RbChoice carries target_player_id for the SelectCard fallback. */
int rb_gsc_can_player_act(const GameState *g, int player_id) {
    if (!g) return 0;
    if (rb_has_pending_choice(g)) {
        int actor = rb_get_pending_choice_player_id(g);
        if (actor >= 0) return actor == player_id;

        const RbChoice *ch = rb_get_pending_choice(g);
        if (ch) {
            /* Rust: SelectAutoAbility | SelectLiveSuccess => compare the
             * embedded player_id directly; SelectCard => compare
             * target_player_id (picker is the same field in C). */
            if (ch->kind == RB_CHOICE_SELECT_AUTO_ABILITY ||
                ch->kind == RB_CHOICE_SELECT_NUMBER) {
                const char *cpid = ch->target_player_id[0] ? ch->target_player_id : NULL;
                if (cpid) return gsc_id_is(player_id, cpid);
            } else if (ch->kind == RB_CHOICE_SELECT_CARD ||
                       ch->kind == RB_CHOICE_SELECT_TARGET ||
                       ch->kind == RB_CHOICE_SELECT_POSITION ||
                       ch->kind == RB_CHOICE_SELECT_HEART_COLOR) {
                const char *cpid = ch->target_player_id[0] ? ch->target_player_id : NULL;
                if (cpid) return gsc_id_is(player_id, cpid);
            }
        }
    }
    switch (g->phase) {
    case RB_PHASE_RPS:
        /* Rust: RockPaperScissors — has THIS player submitted a choice yet?
         * C stores -1 for "no choice" (Rust Option<u8>::is_none). */
        return player_id == 0 ? g->player1_rps_choice < 0
                              : g->player2_rps_choice < 0;
    case RB_PHASE_OPENING:
        /* Rust: ChooseFirstAttacker — winner_idx == Some(1 | 2), i.e. only the
         * RPS winner may pick the first attacker. C collapses this phase into
         * RB_PHASE_RPS (g->rps_winner: -1 none, 0 p1, 1 p2, 2 tie), so the
         * same test is expressed against rps_winner. */
        return g->rps_winner == player_id;
    case RB_PHASE_MULLIGAN_FIRST:
    case RB_PHASE_LIVE_SET:
    case RB_PHASE_PERFORMANCE:
        return player_id == g->first_attacker;
    case RB_PHASE_MULLIGAN_SECOND:
    case RB_PHASE_LIVE_SET_SECOND:
    case RB_PHASE_PERFORMANCE_SECOND:
        return player_id == g->second_attacker;
    default: {
        int active = rb_gsc_active_player_index(g);
        return active == player_id;
    }
    }
}

/* ── opponent_has_performed ────────────────────────────────────────────────
   Rust: mod.rs:459-470. True once the OPPONENT's live is done, so their live
   zone / need-hearts may be revealed. */
int rb_gsc_opponent_has_performed(const GameState *g, int my_player_idx) {
    if (!g || (my_player_idx != 0 && my_player_idx != 1)) return 0;
    int opp_idx  = 1 - my_player_idx;
    int opp_first = (opp_idx == 0) ? (g->first_attacker == 0) : (g->first_attacker == 1);
    if (g->phase == RB_PHASE_PERFORMANCE_SECOND || g->phase == RB_PHASE_VICTORY) return 1;
    if (g->phase == RB_PHASE_PERFORMANCE && opp_first) return 1;
    return 0;
}

/* ── mulligan_owner_index ───────────────────────────────────────────────────
   Rust: mod.rs:788-794. NOTE the phase is advanced BEFORE the mulligan is
   applied, so active_player() is already the other seat — this is the
   separate owner query that avoids the observed "mulliganed the opponent's
   hand" bug.

   THE INVERSION THAT WAS HERE (fixed 2026-10-02). The previous body was

       int p1_is_first = (g->first_attacker == 0);
       MULLIGAN_FIRST  -> p1_is_first ? 1 : 0;
       MULLIGAN_SECOND -> p1_is_first ? 0 : 1;

   which returns 1 when player 1 IS the first attacker. Rust says

       MulliganFirstAttacker  => Some(usize::from(!player1.is_first_attacker))
       MulliganSecondAttacker => Some(usize::from( player1.is_first_attacker))

   and `usize::from(true) == 1`, so the two Rust arms expand to exactly the
   same table this function now implements:

       p1 is first | MulliganFirst | MulliganSecond
       ------------+---------------+----------------
       true        | 0             | 1
       false       | 1             | 0

   which is the four regression vectors at engine/src/turn/mulligan_tests.rs
   :137-140. The old body reproduced the DIAGONAL of that table instead
   (true -> 1 / 0), failing vectors :137 and :139.

   `g->first_attacker` / `g->second_attacker` ARE the seat indices
   (game_setup.c:504-505 `= p1_first ? 0 : 1` / `? 1 : 0`), so returning
   them directly is the faithful translation — not "the opposite seats".

   This was inert (zero callers tree-wide) so it could never have failed a
   suite; it was a landmine for whoever wired the shorter name. It remains a
   DUPLICATE of the live, correct twin rb_setup_mulligan_owner_index
   (game_setup.c:583-588), which is what the mulligan path actually calls.
   Do not create a third copy. */
int rb_gsc_mulligan_owner_index(const GameState *g) {
    if (!g) return -1;
    if (g->phase == RB_PHASE_MULLIGAN_FIRST)  return g->first_attacker;
    if (g->phase == RB_PHASE_MULLIGAN_SECOND) return g->second_attacker;
    return -1;
}

/* ── concede / concede_both ────────────────────────────────────────────────
   Rust: mod.rs:416-454. The C GameState has no `game_result` /
   `game_ended` pair; it has `winner` (-1 = ongoing, 0/1 = that seat won,
   2 = draw), which the engine already treats as "game over" when != -1.
   FirstAttackerWins therefore means winner = the first attacker's seat. */
int rb_gsc_game_has_ended(const GameState *g) { return g && g->winner != -1; }

int rb_gsc_reject_if_ended(const GameState *g) {
    return rb_gsc_game_has_ended(g) ? 0 : 1;
}

int rb_gsc_concede(GameState *g, int player_id) {
    if (!g) return 0;
    if (player_id < 0 || player_id > 1) return 0;      /* "Invalid player seat" */
    if (!rb_gsc_reject_if_ended(g)) return 0;            /* "Game has already ended" */
    int opponent_is_first = (player_id == 0) ? (g->first_attacker != 0)
                                             : (g->first_attacker == 0);
    g->winner = opponent_is_first ? g->first_attacker : g->second_attacker;
    rb_queue_clear(&g->queue);
    return 1;
}

int rb_gsc_concede_both(GameState *g) {
    if (!g) return 0;
    if (!rb_gsc_reject_if_ended(g)) return 0;
    g->winner = 2;                                       /* GameResult::Draw */
    rb_queue_clear(&g->queue);
    return 1;
}

/* ── seat / id accessors ────────────────────────────────────────────────────
   Rust: mod.rs:832-959. All of these are the "one definition replaces the
   per-file `if me == 0` chains" family. */
RbPlayer       *rb_gsc_seat_player_mut(GameState *g, int seat) { return g ? &g->p[seat ? 1 : 0] : NULL; }
const RbPlayer *rb_gsc_seat_player(const GameState *g, int seat) { return g ? &g->p[seat ? 1 : 0] : NULL; }
const RbPlayer *rb_gsc_const_active_player(const GameState *g) {
    return g ? &g->p[rb_gsc_active_player_index(g)] : NULL;
}

/* seated_pair(me) -> (me, opp). Returns `me` through *out_me. */
const RbPlayer *rb_gsc_seated_pair(const GameState *g, int me, const RbPlayer **out_opp) {
    if (!g) { if (out_opp) *out_opp = NULL; return NULL; }
    int a = me ? 1 : 0, b = me ? 0 : 1;
    if (out_opp) *out_opp = &g->p[b];
    return &g->p[a];
}

/* Rust: try_player_by_id — "p1"/"p2", plus the "player1"/"player2" spelling
   Rust's can_player_act normalises. Returns NULL for an unknown id. */
static RbPlayer *gsc_player_by_id(GameState *g, const char *id) {
    if (!g || !id) return NULL;
    if (gsc_id_is(0, id)) return &g->p[0];
    if (gsc_id_is(1, id)) return &g->p[1];
    return NULL;
}
RbPlayer       *rb_gsc_try_player_by_id_mut(GameState *g, const char *id) { return gsc_player_by_id(g, id); }
const RbPlayer *rb_gsc_try_player_by_id(const GameState *g, const char *id) {
    return gsc_player_by_id((GameState *)g, id);
}

/* seat_index_by_id: 0 for p1, else 1 (the legacy `else` arm, deliberately). */
int rb_gsc_seat_index_by_id(const char *player_id) { return gsc_id_is(0, player_id) ? 0 : 1; }

/* cheer_count_by_id / set — Rust mod.rs:895-909, 944-947. Unknown ids read
   / write player2, matching the legacy else arms. */
int rb_gsc_cheer_count_by_id(const GameState *g, const char *player_id) {
    return gsc_id_is(0, player_id) ? g->player1_cheer_blade_heart_count
                                   : g->player2_cheer_blade_heart_count;
}
void rb_gsc_set_cheer_count_by_id(GameState *g, const char *player_id, int count) {
    if (gsc_id_is(0, player_id)) g->player1_cheer_blade_heart_count = count;
    else                        g->player2_cheer_blade_heart_count = count;
}
int rb_gsc_seat_cheer_count(const GameState *g, int seat) {
    return seat == 0 ? g->player1_cheer_blade_heart_count
                     : g->player2_cheer_blade_heart_count;
}

/* score_bonus_by_id / seat_score_bonus — Rust mod.rs:912-918, 953-958. */
int rb_gsc_score_bonus_by_id(const GameState *g, const char *player_id) {
    return gsc_id_is(0, player_id) ? g->mods.p1_constant_total_score_bonus
                                   : g->mods.p2_constant_total_score_bonus;
}
int rb_gsc_seat_score_bonus(const GameState *g, int seat) {
    return seat == 0 ? g->mods.p1_constant_total_score_bonus
                     : g->mods.p2_constant_total_score_bonus;
}

/* ── clear_effect_tracking ──────────────────────────────────────────────────
   Rust: mod.rs:978-984. Must run whenever an ability completes (success or
   failure) and when a post-choice state machine exits.
   mods.last_cost_energy_count has no C field (see report). */
void rb_gsc_clear_effect_tracking(GameState *g) {
    if (!g) return;
    g->n_batch_movements = 0;
    g->mods.last_cost_discard_count = 0;
    g->mods.n_last_cost_moved_card_ids = 0;
    g->mods.n_last_under_move_host_ids = 0;
}

/* ── tracking accessors ─────────────────────────────────────────────────────
   Rust: mod.rs:987-1035. */
/* Rust: mod.rs:987-991 — `Option<i16>`, and -1 is the C field's own "no area
   move yet" sentinel (game_state_abilities.c:1544 reads it the same way).
   Returning 0 here would hand a caller card id 0, which is a real card. */
int rb_gsc_last_area_move_card_id(const GameState *g) {
    return g ? g->last_area_move_card_id : -1;
}
int rb_gsc_last_area_move_by_player(const GameState *g) {
    return g ? g->last_area_move_by_player : -1;
}
/* last_cost_wait_member: the FINAL member waited by the current cost
   (Q266 dynamic blade-limit reference member). Rust mod.rs:993-995 returns
   Option<i16>, so "nobody was waited on" is -1, not 0. */
int rb_gsc_last_cost_wait_member(const GameState *g) {
    if (!g || g->n_last_cost_waited_members <= 0) return -1;
    return g->last_cost_waited_members[g->n_last_cost_waited_members - 1];
}

/* recently_moved_cards() -> Option<&SmallVec>: n == 0 is the None case. */
int rb_gsc_recently_moved_cards(const GameState *g, int *out, int max) {
    if (!g || !out) return 0;
    int n = g->n_recently_moved < max ? g->n_recently_moved : max;
    memcpy(out, g->recently_moved, (size_t)n * sizeof(int));
    return n;
}
void rb_gsc_set_recently_moved_cards(GameState *g, const int *cards, int n) {
    if (!g) return;
    if (n < 0) n = 0;
    if (n > RB_MAX_RECENTLY_MOVED) n = RB_MAX_RECENTLY_MOVED;
    memcpy(g->recently_moved, cards, (size_t)n * sizeof(int));
    g->n_recently_moved = n;
}

int rb_gsc_last_energy_placed_by_effect(const GameState *g) {
    return g ? (g->last_energy_placed_by_effect ? 1 : 0) : 0;
}
int rb_gsc_last_energy_placed_by_player(const GameState *g) {
    return g ? g->last_energy_placed_by_player : -1;
}

/* ── current_ability_source_card_id ─────────────────────────────────────────
   Rust: mod.rs:1112-1114. */
int rb_gsc_current_ability_source_card_id(const GameState *g) {
    if (!g) return -1;
    int cur = rb_queue_current_entry(g);
    if (cur < 0 || cur >= g->queue.n_entries) return -1;
    return g->queue.entries[cur].card_id;
}

/* ── push_revealed_card ─────────────────────────────────────────────────────
   Rust: mod.rs:1070-1088 (+ private push_revealed_internal:1046-1067).
   The C GameState has `revealed_cards` but no `revealed_card_meta`, so the
   side-by-side meta push is reported rather than faked. NOTE: Rust's push
   does NOT clear first — callers that need a fresh list clear it themselves
   (rb_clear_revealed_cards, core/modifiers.c:591). */
void rb_gsc_push_revealed_card(GameState *g, int card_id) {
    if (!g) return;
    if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++] = card_id;
}

/* ── player_prefix / player_prefix_id ───────────────────────────────────────
   Rust: mod.rs:1164-1195. Owning card on p1's stage (or under it) -> p1;
   same for p2; otherwise the ACTIVE player's id. */
const char *rb_gsc_player_prefix_id(const GameState *g) {
    if (!g) return "??";
    int cid = g->activating_card;
    if (cid >= 0) {
        for (int pl = 0; pl < 2; pl++)
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (g->p[pl].stage[s] == cid) return gsc_player_id(pl);
        for (int pl = 0; pl < 2; pl++)
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                for (int i = 0; i < g->p[pl].under_cards[s].n; i++)
                    if (g->p[pl].under_cards[s].cards[i] == cid) return gsc_player_id(pl);
    }
    return gsc_player_id(rb_gsc_active_player_index(g));
}

/* ── zone_snapshot ──────────────────────────────────────────────────────────
   Rust: mod.rs:1198-1213 — 7 zones x 2 players, keyed "P1.hand" etc. */
int rb_gsc_zone_snapshot(const GameState *g, RbGscZoneSnapshot *out) {
    if (!g || !out) return 0;
    int k = 0;
    /* Key storage owned by this function's static table, so the returned
     * `key` pointers stay valid for the caller's lifetime (Rust returns owned
     * Strings from a HashMap; the C port has no allocator contract here, and
     * every consumer only reads the key). */
    static char key_store[RB_GSC_ZONE_SNAPSHOT_KEYS][24];
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        const char *pfx = pl == 0 ? "P1." : "P2.";
        static const char *sfx[7] = { "hand", "deck", "energy_deck", "energy_zone",
                                      "waitroom", "live_zone", "success_live" };
        int vals[7];
        vals[0] = P->hand.n;
        vals[1] = P->deck.n;
        vals[2] = P->energy_deck.n;
        vals[3] = P->energy.n;
        vals[4] = P->discard.n;
        vals[5] = P->live.n;
        vals[6] = P->success.n;
        for (int i = 0; i < 7; i++) {
            snprintf(key_store[k], sizeof(key_store[k]), "%s%s", pfx, sfx[i]);
            out[k].key   = key_store[k];
            out[k].count = vals[i];
            k++;
        }
    }
    return k;
}

/* ── zone_is_private ────────────────────────────────────────────────────────
   Rust: mod.rs:1275-1280 — per rules 4.8 (main deck), 4.9 (energy deck) and
   4.11 (hand): card IDENTITIES must never appear in a shared log. */
int rb_gsc_zone_is_private(const char *zone) {
    if (!zone) return 0;
    return !strcmp(zone, "hand") || !strcmp(zone, "deck") ||
           !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom") ||
           !strcmp(zone, "main_deck") || !strcmp(zone, "energy_deck");
}

/* ── resolve_card / resolve_index_any_player ────────────────────────────────
   Rust: mod.rs:1285-1303. The card NAME is returned only for public zones;
   a private zone yields the neutral "#<idx>" placeholder. */
static void gsc_resolve_card(const GameState *g, int pl, const char *zone, int idx, char *out, size_t out_sz) {
    char placeholder[32];
    snprintf(placeholder, sizeof(placeholder), "#%d", idx);
    if (!g || !zone || !out || !out_sz) { if (out && out_sz) out[0] = '\0'; return; }
    out[0] = '\0';

    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, ids, RB_MAX_ZONE);
    if (idx < 0 || idx >= n) { snprintf(out, out_sz, "%s", placeholder); return; }

    char names[256];
    int got = rb_card_get_card_names(ids[idx], names, sizeof(names));
    if (got > 0 && names[0]) {
        size_t w = strlen(names);
        if (w >= out_sz) w = out_sz - 1;
        memcpy(out, names, w);
        out[w] = '\0';
        return;
    }
    snprintf(out, out_sz, "%s", placeholder);
}

int rb_gsc_resolve_index_any_player(const GameState *g, const char *zone, int idx, char *out, size_t out_sz) {
    char placeholder[32];
    snprintf(placeholder, sizeof(placeholder), "#%d", idx);
    if (!out || !out_sz) return 0;
    if (rb_gsc_zone_is_private(zone)) { snprintf(out, out_sz, "%s", placeholder); return 1; }

    char p1[192];
    gsc_resolve_card(g, 0, zone, idx, p1, sizeof(p1));
    if (strcmp(p1, placeholder) != 0) { snprintf(out, out_sz, "%s", p1); return 1; }
    char p2[192];
    gsc_resolve_card(g, 1, zone, idx, p2, sizeof(p2));
    snprintf(out, out_sz, "%s", p2);
    return 1;
}

/* ── choice_offered_labels ──────────────────────────────────────────────────
   Rust: mod.rs:1307-1382. Returns newline-joined labels (the C port has no
   Vec<String>; callers use the count + a single buffer).
   RbChoice has no SelectLiveSuccess kind; RB_CHOICE_SELECT_AUTO_ABILITY
   covers both auto-ability and live-success option lists, and
   RB_CHOICE_SELECT_NUMBER is the count-choice prompt, which the C port routes
   through the same options list. */
int rb_gsc_choice_offered_labels(const GameState *g, const RbChoice *ch, char *out, size_t out_sz) {
    if (!ch || !out || !out_sz) return 0;
    out[0] = '\0';
    size_t used = 0;

#define GSC_EMIT(...)                                                        \
    do {                                                                     \
        if (used + 1 < out_sz) {                                             \
            int _w = snprintf(out + used, out_sz - used, __VA_ARGS__);       \
            if (_w > 0) used += (size_t)_w < (out_sz - used) ? (size_t)_w : (out_sz - used - 1); \
        }                                                                    \
    } while (0)

    switch (ch->kind) {
    case RB_CHOICE_SELECT_CARD:
        GSC_EMIT("[%s] select %d card(s): %s", ch->zone, ch->count, ch->description);
        if (ch->card_type[0]) GSC_EMIT("  type: %s", ch->card_type);
        if (ch->n_filtered_indices > 0) {
            for (int i = 0; i < ch->n_filtered_indices; i++) {
                char nm[192];
                rb_gsc_resolve_index_any_player(g, ch->zone, ch->filtered_indices[i], nm, sizeof(nm));
                GSC_EMIT("  - %s", nm);
            }
        } else {
            GSC_EMIT("  - candidates from %s", ch->zone);
        }
        break;

    case RB_CHOICE_SELECT_TARGET:
    case RB_CHOICE_SELECT_NUMBER:
        GSC_EMIT("%s", ch->description);
        for (int i = 0; i < ch->n_characters; i++) GSC_EMIT("  - %s", ch->characters[i]);
        GSC_EMIT("  (skip_allowed=%d)", ch->allow_skip ? 1 : 0);
        break;

    case RB_CHOICE_SELECT_POSITION:
        GSC_EMIT("%s (left/center/right)", ch->description);
        GSC_EMIT("  (skip allowed=%d)", ch->allow_skip ? 1 : 0);
        break;

    case RB_CHOICE_SELECT_HEART_COLOR:
        GSC_EMIT("select %d heart color(s)", ch->count);
        for (int i = 0; i < ch->n_heart_options; i++) GSC_EMIT("  - %s", ch->heart_options[i]);
        break;

    case RB_CHOICE_SELECT_AUTO_ABILITY:
        GSC_EMIT("%s", ch->description);
        for (int i = 0; i < ch->n_characters; i++) GSC_EMIT("  - %s", ch->characters[i]);
        break;

    case RB_CHOICE_NONE:
    default:
        break;
    }
#undef GSC_EMIT
    return (int)used;
}

/* ── choice_option_count ────────────────────────────────────────────────────
   Rust: mod.rs:1389-1410 — the number of REAL options (what the player picks
   from), not the label line count, so a 2-option prompt never reports 4. */
int rb_gsc_choice_option_count(const RbChoice *ch) {
    if (!ch) return 0;
    switch (ch->kind) {
    case RB_CHOICE_SELECT_CARD:
        if (ch->n_filtered_indices > 0) return ch->n_filtered_indices;
        return ch->count > 1 ? ch->count : 1;               /* (*count).max(1) */
    case RB_CHOICE_SELECT_TARGET:
    case RB_CHOICE_SELECT_NUMBER: {
        int n = ch->n_characters > 0 ? ch->n_characters : 1;
        return n > 1 ? n : 1;                              /* options.len().max(1) */
    }
    case RB_CHOICE_SELECT_POSITION:
        return 3;                                          /* left / center / right */
    case RB_CHOICE_SELECT_HEART_COLOR: {
        int n = ch->n_heart_options > 0 ? ch->n_heart_options : 1;
        return n > 1 ? n : 1;
    }
    case RB_CHOICE_SELECT_AUTO_ABILITY: {
        int n = ch->n_characters > 0 ? ch->n_characters : 1;
        return n > 1 ? n : 1;
    }
    case RB_CHOICE_NONE:
    default:
        return 0;
    }
}

/* ── rule-log writers ───────────────────────────────────────────────────────
   Rust: mod.rs:1526-1539 (push_rule_log / push_rule_log_fmt) landing in
   `rule_log: Vec<String>` through push_rule_log_to / truncate_rule_log
   (mod.rs:1584-1606).

   WAS WRONG AND IS NOW RIGHT. This used to read:

       The C GameState has no `rule_log: Vec<String>` ring; the port's rule-log
       sink is the global verdict buffer (ability/log.c, rb_log_push_verdict)

   and did exactly that — rb_gsc_push_rule_log called
   rb_log_push_verdict(text,"rule_log",1), which is the CONDITION-VERDICT ring
   (ability/log.c: RB_LOG_BUF_MAX == 32, drop-newest). Both
   include/rabuka.h:1786-1796 and the file banner above FORBID that routing,
   because the two logs then evict each other's entries: a rule-log line pushed
   past 32 total lines starts overwriting condition verdicts, and a condition
   verdict pushed past 32 loses the rule-log history. `GameState.rule_log[500]`
   / `rule_log_len[]` / `n_rule_log` now exist, so the ring is out of the path.

   gsc_rule_log_push is the faithful port of `push_rule_log_to`:
       log.push(text); truncate_rule_log(log);        // mod.rs:1584-1587
       if log.len() > LOG_BOUND_RULE { log.drain(0..log.len() - LOG_BOUND_RULE); }
   i.e. the NEWEST RB_GSC_RULE_LOG_MAX lines are kept and the oldest are dropped
   off the front. A C `Vec` has no drain, so the drain is the memmove below and
   the parallel `rule_log_len[]` array is shifted with it — otherwise entry i and
   its length would describe different lines. Rust's Vec is unbounded, so a line
   LONGER than RB_GSC_RULE_LOG_LINE_MAX is truncated here; the corpus's longest
   formatted rule-log line is far shorter, and dropping the tail of one line is
   the only bounded-array behaviour available.

   THE GATE. Rust gates push_rule_log / push_rule_log_fmt on
   `runtime_flags::logging_enabled()` (mod.rs:1527/:1536). The C has no exported
   equivalent of log.c's `rb_log_recording()` (= g_log_enabled ||
   rb_ability_debug_enabled(), ability/log.c:47-49, which is static), so this
   uses rb_ability_debug_enabled() — the ABILITY_DEBUG atomic the C does export
   and the gate live.c:1298 already applies to its own rule-log writer. Same
   observable behaviour as before this change (rb_log_push_verdict was gated by
   the same pair), so no suite moves. The C-only rb_log_set_enabled() hook does
   NOT reach here; that is reported, not silently accepted. */
static void gsc_rule_log_push(const GameState *g, const char *text) {
    if (!g || !text) return;
    if (!rb_ability_debug_enabled()) return;
    /* Rust takes `&mut self` because Vec::push needs it; the C takes a const
       GameState* because two of the four entry points are declared that way
       (include/rabuka.h:3613-3615) and re-casting them at each call site would
       be noisier than one documented cast here. `rule_log` is append-only
       display state — no reader in engine/src touches anything else on `g`. */
    GameState *m = (GameState *)g;
    if (m->n_rule_log >= RB_GSC_RULE_LOG_MAX) {
        memmove(m->rule_log[0], m->rule_log[1],
                (size_t)(RB_GSC_RULE_LOG_MAX - 1) * sizeof(m->rule_log[0]));
        memmove(m->rule_log_len, m->rule_log_len + 1,
                (size_t)(RB_GSC_RULE_LOG_MAX - 1) * sizeof(m->rule_log_len[0]));
        m->n_rule_log = RB_GSC_RULE_LOG_MAX - 1;
    }
    size_t len = strlen(text);
    if (len > (size_t)RB_GSC_RULE_LOG_LINE_MAX - 1)
        len = (size_t)RB_GSC_RULE_LOG_LINE_MAX - 1;
    memcpy(m->rule_log[m->n_rule_log], text, len);
    m->rule_log[m->n_rule_log][len] = '\0';
    m->rule_log_len[m->n_rule_log] = (uint16_t)len;
    m->n_rule_log++;
}

/* rb_gsc_push_rule_log_fmt — BLOCKED ON include/rabuka.h:3609.

   THE DECLARED SIGNATURE IS THE BLOCKER, and it is the one signature the header
   re-extraction got "right" and therefore hid. Rust's push_rule_log_fmt is a
   `&mut self` method (mod.rs:1535) — it takes the GameState by definition, and
   all 133 of its engine/src call sites (choice.rs:292/:1656/:1911, log.rs:157,
   resolver.rs:1033, phases.rs:1394/:1417/:1762) have one in hand. The C
   declaration

       void rb_gsc_push_rule_log_fmt(const char *fmt, ...);

   has no GameState*, so this function STRUCTURALLY CANNOT reach g->rule_log.
   Its previous body therefore called rb_gsc_push_rule_log, which wrote into the
   32-slot condition-verdict ring — the exact thing the header forbids.

   EXACT PATCH for the header owner (one line, include/rabuka.h:3609):
       - void rb_gsc_push_rule_log_fmt(const char *fmt, ...);
       + void rb_gsc_push_rule_log_fmt(GameState *g, const char *fmt, ...);

   That one change is what unblocks the whole group: the ~25 C sites that
   hand-roll the Rust call by writing rb_log_push_verdict(line,"rule_log",1)
   become one-token swaps to rb_gsc_push_rule_log_fmt(g, "%s", line). Until it
   lands this function CANNOT log, so rather than silently discard the line (or
   put it back in the ring) it says so once on stderr and returns. It has ZERO
   callers tree-wide today, so nothing is lost and no suite moves; the point of
   the one-shot warning is that the first future caller hears about it instead
   of shipping a rule log that vanishes. */
void rb_gsc_push_rule_log_fmt(GameState *g, const char *fmt, ...) {
    if (!g || !fmt) return;
    char line[RB_GSC_RULE_LOG_LINE_MAX];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(line, sizeof(line), fmt, ap);
    va_end(ap);
    gsc_rule_log_push(g, line);
}

/* ── log_entry ─────────────────────────────────────────────────────────────
    Rust: mod.rs:1117-1161. Gated on logging_enabled (gsc_rule_log_push's gate);
    `player_label` / `source_card_name` feed the structured entry, which has no
    C field, so only the rule-log text half lands (reported).

    Rust's SIBLING `log_ability` (mod.rs:1154-1162) is deliberately NOT ported:
    it has ZERO callers in engine/src and ZERO in engine/tests, so it is not a
    port of anything and was dead weight here. It was the only reason
    rb_gsc_player_prefix_id had a (discarded) caller. */
void rb_gsc_log_entry(GameState *g, const char *text, const char *player_label, int source_card_id,
                      const char *source_card_name, const char *category) {
    if (!g || !text) return;
    char line[768];
    snprintf(line, sizeof(line), "[T%d %s] %s | event=%s source=%d card=%s",
             g->turn, rb_phase_name(g->phase), text,
             category ? category : "", source_card_id,
             source_card_name ? source_card_name : "None");
    gsc_rule_log_push(g, line);
    (void)player_label;   /* only feeds the structured entry, which has no C field */
}

/* ── push_choice_offered / push_choice_resolved ─────────────────────────────
   Rust: mod.rs:1415-1522. Both push a structured entry AND (for resolved) a
   plain-text rule-log line; only the text half is expressible without the
   structured_log field, and that is what the 3DS game-log history reads. */
void rb_gsc_push_choice_offered(const GameState *g, const RbChoice *ch) {
    if (!g || !ch) return;
    int opts = rb_gsc_choice_option_count(ch);
    char labels[1024];
    rb_gsc_choice_offered_labels(g, ch, labels, sizeof(labels));
    /* Sized to the worst case the format can produce, so this intermediate is never
   the bottleneck: "[T" + int (11) + " " + phase_name + "] Choice offered: " +
   description (char[512], so 511) + " | options=" + int (11) + " skip_allowed="
   + int (11) + " source=" + int (11) + "\n" + labels (1023) + NUL = 1665 worst
   case. `line[1400]` was short of that, which is the -Wformat-truncation at
   :844. It was harmless -- gsc_rule_log_push copies into
   `rule_log[...][RB_GSC_RULE_LOG_LINE_MAX]`, so everything past that was dropped
   anyway -- but "harmless because a later buffer is smaller" is not a reason to
   leave a truncating intermediate in the middle of a formatter. */
char line[1792];
    snprintf(line, sizeof(line),
             "[T%d %s] Choice offered: %s | options=%d skip_allowed=%d source=%d\n%s",
             g->turn, rb_phase_name(g->phase),
             ch->description[0] ? ch->description : labels, opts,
             ch->allow_skip ? 1 : 0, g->activating_card, labels);
    gsc_rule_log_push(g, line);
}

void rb_gsc_push_choice_resolved(const GameState *g, const RbChoice *ch,
                                 const char *chosen, int skipped) {
    if (!g || !ch) return;
    int offered = rb_gsc_choice_option_count(ch);
    char line[768];
    snprintf(line, sizeof(line),
             "[choice] resolved: offered %d option(s), picked %s",
             offered, skipped ? "skip" : (chosen ? chosen : ""));
    gsc_rule_log_push(g, line);
}

/* ── push_performance_snapshot ──────────────────────────────────────────────
   Rust: mod.rs:1615-1617 — unconditional push (no logging gate). The C ring is
   RB_MAX_SNAPSHOTS; Rust's Vec is unbounded, so a full ring drops the push. */
void rb_gsc_push_performance_snapshot(GameState *g, const RbLiveSnapshot *snap) {
    if (!g || !snap) return;
    if (g->n_snapshots < RB_MAX_SNAPSHOTS) g->snapshots[g->n_snapshots++] = *snap;
}

/* ── record_ability_application ─────────────────────────────────────────────
   Rust: mod.rs:1621-1652. Source-tracking for the performance snapshot,
   pushed after a modifier is applied. The C equivalent ring is
   RbMods.trace (documented in rabuka.h as mirroring AbilityApplication). */
void rb_gsc_record_ability_application(GameState *g, int source_card_id,
                                       const char *ability_text, const char *effect_type,
                                       int target_card_id, int heart_color, int amount) {
    if (!g) return;
    int t = RB_EFFECT_HEART_BONUS;   /* Rust's `_ =>` fallback arm */
    if (effect_type) {
        if      (!strcmp(effect_type, "heart_bonus"))    t = RB_EFFECT_HEART_BONUS;
        else if (!strcmp(effect_type, "blade_bonus"))    t = RB_EFFECT_BLADE_BONUS;
        else if (!strcmp(effect_type, "score_bonus"))    t = RB_EFFECT_SCORE_BONUS;
        else if (!strcmp(effect_type, "score_set"))      t = RB_EFFECT_SCORE_SET;
        else if (!strcmp(effect_type, "transform"))      t = RB_EFFECT_TRANSFORM;
        else if (!strcmp(effect_type, "need_heart_mod") ||
                 !strcmp(effect_type, "need_heart_set")) t = RB_EFFECT_NEED_HEART_MOD;
        else if (!strcmp(effect_type, "heart_override")) t = RB_EFFECT_HEART_OVERRIDE;
        else if (!strcmp(effect_type, "cost_bonus"))     t = RB_EFFECT_COST_BONUS;
        else if (!strcmp(effect_type, "cost_set"))       t = RB_EFFECT_COST_SET;
        else if (!strcmp(effect_type, "blade_set"))      t = RB_EFFECT_BLADE_SET;
        else if (!strcmp(effect_type, "blade_type_set")) t = RB_EFFECT_BLADE_TYPE_SET;
    }
    rb_mods_trace_push(&g->mods, source_card_id,
                       ability_text ? ability_text : "", t,
                       target_card_id, heart_color, amount);
}

/* ── print_trace_node ───────────────────────────────────────────────────────
   Rust: mod.rs:1671-1712 (private). Prints the node, then the zone deltas
   between before/after, then recurses. `dump_last_trace` needs a
   `last_ability_trace` GameState field (reported); this node walker is
   usable by whoever adds it. */
void rb_gsc_print_trace_node(const RbAbilityTraceNode *node, int indent) {
    if (!node) return;
    for (int i = 0; i < indent; i++) fputs("  ", stdout);
    printf("- %s", node->label);
    if (node->has_card) printf(" [%s]", node->card);
    putchar('\n');

    if (node->has_before && node->has_after) {
        const RbZoneSnapshot *b = &node->before, *a = &node->after;
        char deltas[512]; size_t w = 0; deltas[0] = '\0';
#define DELTA(fmt_, ...) do { \
            int _n = snprintf(deltas + w, sizeof(deltas) - w, fmt_ "\n", __VA_ARGS__); \
            if (_n > 0) w += (size_t)_n < (sizeof(deltas) - w) ? (size_t)_n : (sizeof(deltas) - w - 1); \
        } while (0)
#define DELTA_IF(cond, fmt_, ...) do { if (cond) DELTA(fmt_, __VA_ARGS__); } while (0)
        DELTA_IF(b->hand_count != a->hand_count,   "Hand: %d -> %d", b->hand_count, a->hand_count);
        DELTA_IF(b->stage_count != a->stage_count, "Stage: %d -> %d", b->stage_count, a->stage_count);
        DELTA_IF(b->waitroom_count != a->waitroom_count, "Discard: %d -> %d", b->waitroom_count, a->waitroom_count);
        DELTA_IF(b->energy_count != a->energy_count, "Energy: %d -> %d", b->energy_count, a->energy_count);
        DELTA_IF(b->active_energy_count != a->active_energy_count, "Active Energy: %d -> %d", b->active_energy_count, a->active_energy_count);
        DELTA_IF(b->deck_count != a->deck_count,   "Deck: %d -> %d", b->deck_count, a->deck_count);
#undef DELTA_IF
#undef DELTA
        if (w) {
            for (int i = 0; i < indent; i++) fputs("  ", stdout);
            /* join with ", " the way Rust's changes.join(", ") does */
            for (size_t i = 0; i < w; i++) {
                if (deltas[i] == '\n') fputs(", ", stdout); else fputc(deltas[i], stdout);
            }
            putchar('\n');
        }
    }
    for (int i = 0; i < node->n_children; i++) rb_gsc_print_trace_node(node->children[i], indent + 1);
}
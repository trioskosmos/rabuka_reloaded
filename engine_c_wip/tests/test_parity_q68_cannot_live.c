/* ===========================================================================
 * test_parity_q68_cannot_live.c
 *
 * Q68 — 「ライブ開始時 IS UNUSABLE FOR A ライブ不能 PLAYER」.
 *
 * WHAT THE RULING SAYS
 * -------------------
 * cards/cards.json, FAQ Q68 (2025.02.07), stated on PL!HS-bp2-014-N,
 * PL!SP-bp1-001-P and PL!SP-bp1-001-R (identical text on all three):
 *
 *   『ライブできない』状態のプレイヤーは、ライブカードセットフェイズで
 *   ライブカード置き場に手札のカードを裏向きで置くことはできますが、
 *   パフォーマンスフェイズで表向きにしたカードの中にライブカードが
 *   あったとしても、そのライブカードを含めて控え室に置きます。その結果、
 *   ライブカード置き場にライブカードが置かれていないため、ライブは行われません。
 *   （ライブ開始時の能力は使えず、エールも行いません）
 *
 * Three requirements. This suite is (b), the ライブ開始時 one; (a) and (c) are
 * rb_perform_live's bail-out in src/turn/live.c, still BLOCKED-ON the same
 * per-turn fold this suite exercises here.
 *
 * WHAT RUST DOES (the oracle), cited by CONTENT
 * ---------------------------------------------
 * engine/src/turn/trigger_scan.rs:186-212 `trigger_live_start_abilities` gates
 * on exactly two things:
 *     195   if Self::is_trigger_suppressed(game_state, player_id, "live_start")
 *     207-212   if current_phase == FirstAttackerPerformance
 *               && !player.is_first_attacker
 *               && player.live_card_zone.cards.is_empty()
 * and on nothing else. The string `cannot_live_players` does not occur anywhere
 * in that file. Rust first reads it at engine/src/turn/phases.rs:351, i.e.
 * AFTER the two scan calls at :245-246, and consumes it inside the yell at
 * engine/src/turn/live.rs:2053-2055 (`discard_live_on_cannot_live`). Rust
 * therefore fires a cannot_live player's ライブ開始時 and then throws their live
 * cards away. That is the defect this suite's gate exists to NOT reproduce.
 *
 * The set is PER TURN in Rust: engine/src/core/game_state/mod.rs:145 declares
 * `cannot_live_players: SmallVec<[PlayerId; 2]>` and
 * engine/src/turn/phases.rs:280 empties it at the rollover, adjacent to
 * `cannot_activate_members.clear()` at :279. A second, redundant clear lives at
 * engine/src/core/game_state/abilities.rs:3207 (`clear_live_only_state`, reached
 * from `check_expired_effects` at :3241, which phases.rs:310 calls at the same
 * rollover). The writer is engine/src/ability/effects/restriction.rs:101-105.
 *
 * THE C, AND WHY THE GATE IS SAFE TO LAND
 * ---------------------------------------
 * The C's writer (src/ability/effects/rules.c:1922, the `cannot_live` arm of
 * rb_rules_execute_restriction) and its reader (rb_rules_cannot_live_player,
 * same file) both pre-date this suite, and BOTH pre-date any use: the reader had
 * ZERO callers, so `RbRulesState::cannot_live_players[2]` was set and never read
 * — and, worse, never CLEARED. Wiring the reader without a clear would have
 * turned a correct gate into "this seat can never perform a live again", which
 * is exactly the hazard PARITY_CHECKLIST.md item I2 warns about.
 *
 * The rollover clear is not landable here: the flag lives in rules.c's
 * file-static RbRulesState, whose accessor `rb_rules_state` is `static`, so
 * src/turn/phase.c (the RB_PHASE_VICTORY rollover, NOT this task's file) has no
 * way to reach it. What IS landable inside rules.c is the same idiom the C
 * already uses for Rust's per-turn `turn_limited_abilities` — the use-limit
 * table is keyed by turn (src/ability/ability_queue.c:1138) and
 * src/turn/phase.c:564-567 records that this "is the same guarantee Rust gets
 * from the explicit clear". So the flag now carries the turn it was set on and
 * the reader drops a stamp from an older turn. Section 2 below measures that
 * fold directly; it is the reason this gate is landed rather than BLOCKED-ON.
 * =========================================================================== */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* ⛔ BLOCKED-ON include/rabuka.h (main's file, not this task's). Both are
   defined in src/ability/effects/rules.c and are NOT declared in the header;
   the same file-scope-extern pattern is already used in src/turn/triggers.c:6-16
   for rb_check_effect_position. Add beside the other rb_rules_* accessors
   (rabuka.h, near the rules.c section):
       int rb_rules_cannot_live_player(GameState *g, int pl);
       int rb_rules_execute_restriction(GameState *g, int actor,
                                        const AbilityEffect *e);
   and then these two declarations can go. */
int rb_rules_cannot_live_player(GameState *g, int pl);
int rb_rules_execute_restriction(GameState *g, int actor, const AbilityEffect *e);

/* The two cards this suite names.
 *
 * `test_id_exact`, NOT `test_id`: rb_find_card_by_no has a substring fallback
 * that silently resolves a NEAR-MISS card (PARITY_CHECKLIST.md's CARD-NUMBER
 * TRAP — PL!SP-bp2-011-P and -R differ), and test_id_exact is the only helper
 * that strcmps the decoded card number with no fallback at all
 * (src/test_game.c, test_id_exact). Neither card needs an identity-tracked copy
 * (no test_new_id): nothing here counts occurrences of one card instance. */
#define Q68_RESTRICT_CARD "PL!SP-bp1-001-P" /* ab#0: 常時, restriction cannot_live, target self */
#define Q68_LIVESTART_CARD "PL!HS-bp5-019-L" /* ab#0: ライブ開始時 (ハナムスビ) */

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
        FLAT_VERDICT(); \
    } \
} while (0)

static void ensure_cards_loaded(void)
{
    static int tried;
    if (tried) return;
    tried = 1;
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: card database did not load\n");
        failures++;
        FLAT_VERDICT();
    }
}

/* ---------------------------------------------------------------------------
 * 1. THE WRITER, THROUGH THE REAL CARD.
 *
 * Nothing here hand-builds an AbilityEffect: the node is decoded out of the
 * committed bytecode blob by the engine's own decoder, so this section also
 * proves the `cannot_live` restriction survives the C's decode path at all.
 * ------------------------------------------------------------------------- */
static void t_q68_writer_from_the_real_card(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    int cid = test_id_exact(&tg, Q68_RESTRICT_CARD);
    CHECK(cid >= 0, "Q68_RESTRICT_CARD resolves by exact card-number match");
    if (cid < 0) return;
    CHECK(rb_card_num_abilities((uint32_t)cid) >= 1,
          "the ライブ不能 card carries at least one ability");

    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, 0, &ab)) {
        CHECK(0, "the ライブ不能 card's ab#0 decodes");
        return;
    }
    CHECK(ab.effect != NULL, "the ライブ不能 card's ab#0 has an effect");

    /* Premise: this really is the cannot_live node and not a near-miss card.
       restriction.rs:101-105 stamps the SEAT the restriction names, and the
       wire says target:"self", so the actor's own seat is the victim. */
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 0,
          "a fresh game has no ライブ不能 flag on p1");
    CHECK(rb_rules_cannot_live_player(&tg.state, 1) == 0,
          "a fresh game has no ライブ不能 flag on p2");

    rb_rules_execute_restriction(&tg.state, 0, ab.effect);
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 1,
          "restriction.rs:101-105 — the named seat is now ライブ不能");
    CHECK(rb_rules_cannot_live_player(&tg.state, 1) == 0,
          "and only that seat: the opponent is untouched");

    rb_free_ability(&ab);
}

/* ---------------------------------------------------------------------------
 * 2. THE LATCH — THE FOLD THAT MAKES THE GATE SAFE.
 *
 * Rust phases.rs:280 empties `cannot_live_players` at the rollover. The C cannot
 * be cleared from outside rules.c, so the turn stamp in rb_rules_cannot_live_player
 * carries the same guarantee. Both halves are measured here, because a gate that
 * only has the first half is the defect this section exists to prevent:
 *
 *   (a) SAME turn  -> the flag is still honoured (a naive "clear on read" would
 *       break the ability it is supposed to gate);
 *   (b) NEXT turn  -> the flag reads 0, which is what Rust's clear produces.
 *
 * (b) is the assertion that would FAIL against the pre-fix reader, which returned
 * `cannot_live_players[pl]` unconditionally.
 * ------------------------------------------------------------------------- */
static void t_q68_latch_is_per_turn(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    int cid = test_id_exact(&tg, Q68_RESTRICT_CARD);
    CHECK(cid >= 0, "Q68_RESTRICT_CARD resolves");
    if (cid < 0) return;

    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, 0, &ab)) {
        CHECK(0, "the ライブ不能 card's ab#0 decodes");
        return;
    }

    int turn0 = tg.state.turn;
    rb_rules_execute_restriction(&tg.state, 0, ab.effect);
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 1,
          "the flag is set for the turn it was set in");

    /* Re-reading inside the same turn must not consume the flag: several
     * ライブ開始時 scans and the cannot_live bail-out both read it. */
    for (int i = 0; i < 3; i++)
        CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 1,
              "reading the flag inside its own turn never clears it");

    /* phases.rs:280 — the rollover. The C bumps g->turn in the RB_PHASE_VICTORY
     * arm (src/turn/phase.c:382); this suite drives the same datum directly
     * rather than advancing the whole phase machine, so the assertion is about
     * the fold and not about how many phases a turn has. */
    tg.state.turn = turn0 + 1;
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 0,
          "phases.rs:280 — the flag does not outlive its turn "
          "(against the pre-fix reader this reads 1)");

    /* And it is not a one-shot: a SECOND ライブ不能 in the new turn arms again. */
    rb_rules_execute_restriction(&tg.state, 0, ab.effect);
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 1,
          "a ライブ不能 set in the new turn is honoured in the new turn");
    tg.state.turn = turn0 + 2;
    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 0,
          "and expires again at the next rollover");

    CHECK(rb_rules_cannot_live_player(&tg.state, 1) == 0,
          "the opponent is never armed by p1's own restriction");
    CHECK(rb_rules_cannot_live_player(NULL, 0) == 0,
          "a null game reads 0 rather than faulting");
    CHECK(rb_rules_cannot_live_player(&tg.state, 7) == 0,
          "an out-of-range seat reads 0 rather than faulting");

    rb_free_ability(&ab);
}

/* ---------------------------------------------------------------------------
 * 3. THE GATE — ライブ開始時 does not fire for a ライブ不能 seat.
 *
 * The scan returns the number of entries it ENQUEUED, and Rust's scan enqueues
 * before any condition is evaluated (trigger_scan.rs:238-259 collects then
 * :518-528 triggers), so the count is the right observable and does not depend
 * on the card's condition being satisfiable.
 *
 * The gate lives in rb_trigger_live_start (src/turn/triggers.c). Both seats are
 * scanned on one board with the SAME ライブ開始時 card staged for each, so the
 * only thing that differs is the flag — that is what makes this a control rather
 * than two unrelated measurements.
 * ------------------------------------------------------------------------- */
static void t_q68_live_start_is_gated(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    int live_cid = test_id_exact(&tg, Q68_LIVESTART_CARD);
    CHECK(live_cid >= 0, "Q68_LIVESTART_CARD resolves by exact card-number match");
    if (live_cid < 0) return;

    int restrict_cid = test_id_exact(&tg, Q68_RESTRICT_CARD);
    CHECK(restrict_cid >= 0, "Q68_RESTRICT_CARD resolves");
    if (restrict_cid < 0) return;

    /* Premises, so a card-number near-miss fails here with a clear message
     * instead of turning section 3 into a mystery. */
    {
        Ability probe;
        CHECK(rb_decode_card_ability((uint32_t)live_cid, 0, &probe),
              "the ライブ開始時 card's ab#0 decodes");
        CHECK(rb_ability_has_trigger(&probe, RB_TK_LIVE_START),
              "the staged card really carries ライブ開始時");
        rb_free_ability(&probe);
    }

    /* CONTROL, unrestricted: both seats fire. */
    test_add_to_stage(&tg, 0, live_cid);
    test_set_opp_stage(&tg, 0, live_cid);
    int p1_free = rb_trigger_live_start(&tg.state, 0);
    int p2_free = rb_trigger_live_start(&tg.state, 1);
    CHECK(p1_free > 0, "CONTROL: p1 fires its ライブ開始時 when unrestricted");
    CHECK(p2_free > 0, "CONTROL: p2 fires its ライブ開始時 when unrestricted");

    /* Now arm p1 only, through the real restriction node. */
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)restrict_cid, 0, &ab)) {
        CHECK(0, "the ライブ不能 card's ab#0 decodes");
        return;
    }
    rb_rules_execute_restriction(&tg.state, 0, ab.effect);
    rb_free_ability(&ab);

    CHECK(rb_rules_cannot_live_player(&tg.state, 0) == 1,
          "p1 is ライブ不能 going into the live");
    CHECK(rb_rules_cannot_live_player(&tg.state, 1) == 0,
          "p2 is not");

    int p1_gated = rb_trigger_live_start(&tg.state, 0);
    int p2_open  = rb_trigger_live_start(&tg.state, 1);
    CHECK(p1_gated == 0,
          "Q68: a ライブ不能 player's ライブ開始時 does not fire "
          "(against the pre-fix scan this is > 0)");
    CHECK(p2_open > 0,
          "Q68: the gate is per-seat — p2 still fires on the identical board");

    /* The gate must not leak into the next turn (the fold, observed through the
     * scan rather than through the reader). */
    tg.state.turn += 1;
    CHECK(rb_trigger_live_start(&tg.state, 0) > 0,
          "the gate expires with the turn: p1 fires again next turn");
}

/* ---------------------------------------------------------------------------
 * 4. THE GATE DOES NOT EAT RUST'S OWN GATES.
 *
 * rb_trigger_live_start had two early returns before this change
 * (src/turn/triggers.c:279 suppression, :301-302 the Q72 empty-live-zone guard).
 * The new one is a third; these two checks say the other two still work, and
 * that a restricted seat which is ALSO trigger-suppressed is still 0 rather than
 * something new.
 * ------------------------------------------------------------------------- */
static void t_q68_gate_composes_with_the_existing_guards(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    int live_cid = test_id_exact(&tg, Q68_LIVESTART_CARD);
    int restrict_cid = test_id_exact(&tg, Q68_RESTRICT_CARD);
    CHECK(live_cid >= 0 && restrict_cid >= 0, "both cards resolve");
    if (live_cid < 0 || restrict_cid < 0) return;

    /* Q72, trigger_scan.rs:207-212 — second attacker, empty live zone, no scan.
     * Modelled on the phase value Rust holds at the scan call site, which is
     * why this is driven through the direct call rather than the phase machine.
     * BOTH seats are staged: the controls below are about a seat that is NOT
     * ライブ不能, and a seat with nothing on stage has no ライブ開始時 to fire. */
    test_add_to_stage(&tg, 0, live_cid);
    test_set_opp_stage(&tg, 0, live_cid);
    tg.state.phase = RB_PHASE_PERFORMANCE;
    tg.state.first_attacker = 0;
    CHECK(rb_trigger_live_start(&tg.state, 1) == 0,
          "Q72: the empty-live-zone guard still fires for the second attacker");
    CHECK(rb_trigger_live_start(&tg.state, 0) > 0,
          "and the first attacker is unaffected by it");
    tg.state.phase = RB_PHASE_LIVE_SET_SECOND;

    /* A restricted seat that is also suppressed is 0 either way; the point is
     * that arming the restriction did not change the OTHER seat's behaviour. */
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)restrict_cid, 0, &ab)) {
        CHECK(0, "the ライブ不能 card's ab#0 decodes");
        return;
    }
    rb_rules_execute_restriction(&tg.state, 0, ab.effect);
    rb_free_ability(&ab);
    CHECK(rb_trigger_live_start(&tg.state, 0) == 0, "the new gate returns 0");
    CHECK(rb_trigger_live_start(&tg.state, 1) > 0,
          "the unrestricted seat is untouched by the gate");

    /* Bad arguments are refused before the flag is consulted. */
    CHECK(rb_trigger_live_start(NULL, 0) == 0, "a null game scans nothing");
    CHECK(rb_trigger_live_start(&tg.state, -1) == 0, "a bad seat scans nothing");
    CHECK(rb_trigger_live_start(&tg.state, 2) == 0, "an out-of-range seat scans nothing");
}

int main(void)
{
    printf("=== Q68: ライブ開始時 is unusable for a ライブ不能 player ===\n");
    t_q68_writer_from_the_real_card();
    t_q68_latch_is_per_turn();
    t_q68_live_start_is_gated();
    t_q68_gate_composes_with_the_existing_guards();
    printf("=== %d failed ===\n", failures);
    return failures ? 1 : 0;
}
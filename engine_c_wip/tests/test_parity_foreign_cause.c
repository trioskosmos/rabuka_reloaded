/* test_parity_foreign_cause.c
 *
 * Parity coverage for the OPPONENT-CAUSE area-movement guard:
 *   Rust twin  engine/src/core/game_state/abilities.rs  hooked_by_foreign_cause
 *   C port     engine_c_wip/src/core/game_state_abilities.c
 *                rb_hooked_by_foreign_cause
 *
 * WHY THIS FILE EXISTS. The C's scan of this arm used to read
 * `g->batch_movements` and reconstruct "was this an AREA move" with a
 * `source_zone==STAGE && dest_zone==STAGE` test. Rust reads
 * `gs.turn_area_movements` and performs no such reconstruction. Both halves of
 * the old body were wrong: `batch_movements` is the PER-BATCH set, emptied when
 * the batch closes, so a rescan pass after the batch found nothing and the guard
 * silently never fired. Correcting the vector moved ZERO failures across 33
 * suites, which is the measurement that proves no corpus ability exercises this
 * arm -- and an unpinned fix is one the next agent reverts. Hence this file.
 *
 * Rust reference, cited by CONTENT (that tree is being edited), the doc comment
 * above `hooked_by_foreign_cause`:
 *
 *   "A 「対戦相手のカードの効果でも発動する。」 watcher on an AREA MOVE is
 *    enqueued by push_movement_event, and only when the move was caused by the
 *    other player. Attribute via the watcher's own turn-scoped move record,
 *    which is stable across rescan passes unlike the batch sets."
 *
 *   if !ability.effect.is_some_and(|e| e.fires_on_opponent_effects()) { false }
 *   let Some(rec) = self.turn_area_movements.iter().rev()
 *                      .find(|m| m.moved_card_id == card_id) else { false };
 *   if rec.cause_player_id == player_id { false }
 *   true
 *
 * WHAT IS PINNED, and the half most likely to be wrong:
 *
 *  1. the `fires_on_opponent_effects` pre-gate, evaluated BEFORE anything else
 *     and with no movement lookup at all;
 *  2. the FOREIGN case -- an area movement whose `cause_player_id` differs from
 *     the watching seat is owned by the hook, so the generic TAS scan must not
 *     enqueue it a second time;
 *  3. **the NEGATIVE clause `cause_player_id == player_id` -> false.** This is
 *     the whole point of the guard: without it the C would suppress EVERY
 *     marker-carrying watcher on every area move, including the player's own.
 *  4. `.rev().find()`: the LAST matching movement is the one that answers, not
 *     the first. An own-caused move recorded AFTER a foreign one must suppress
 *     the guard. A forward scan gets this exactly backwards.
 *  5. the VECTOR: the answer must survive the batch being closed. `batch_movements`
 *     is emptied at the batch boundary (`g->n_batch_movements = 0` at
 *     game_state_abilities.c, game_state_core.c and rb_clear_card_movement_tracking
 *     -- modifiers.c); `turn_area_movements` survives until the turn rolls over
 *     (modifiers.rs `self.turn_area_movements.clear()`), and that difference is
 *     the bug being pinned.
 *  6. NON-area movements never reach the log, so they can never answer -- the
 *     property the old reader had to re-derive and could get wrong.
 *
 * WHAT IS DELIBERATELY NOT PINNED: a source-unconditional / target-guarded
 * split. There is none in this arm. The comment in modifiers.rs about the
 * source slot being pushed unconditionally while the target slot is guarded
 * belongs to `record_stage_position_swap`, a different function; here the match
 * is on `moved_card_id` alone, for any card in the area log.
 *
 * sizeof(GameState) is 1 195 656 B, so the TestGame is `static`.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Defined in src/core/game_state_abilities.c. It used to be `static`, which is
 * part of why the corrected vector was unpinnable: include/rabuka.h is owned by
 * another agent, so the prototype is repeated here and the definition was made
 * non-static. */
extern int rb_hooked_by_foreign_cause(const GameState *g, const AbilityEffect *e,
                                      int card_id, int player);
/* The TURN-ROLLOVER clear, `self.turn_area_movements.clear()` in modifiers.rs.
 * It is NOT rabuka.h's rb_clear_movement_tracking: that one
 * (game_state_abilities.c:581) is the post-drain clear and does not touch
 * n_turn_area_movements at all. The function wanted here is the one at
 * src/core/modifiers.c that ends with `g->n_turn_area_movements = 0`, and it has
 * no header declaration, hence the extern. */
extern void rb_clear_card_movement_tracking(GameState *g);

/* ── verdict layer (same shape as tests/test_parity_deck_order.c) ────────── */

static int failures;
static int assertions;
static const char *g_flat_failed[64];
static int g_flat_failed_n;

static void flat_verdict_failed(const char *fn)
{
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;
    if (g_flat_failed_n >= 64) return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}

#define FLAT_VERDICT() flat_verdict_failed(__func__)

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
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
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixtures ────────────────────────────────────────────────────────────── */

static TestGame tg;   /* static: 1 195 656 B */

/* The predicate the C reads is `rb_effect_fires_on_opponent_effects`, which is
 * `effect.extra["fires_on_opponent_effects"] == "true"` and nothing else (Rust:
 * `e.fires_on_opponent_effects()`, the same single wire key). So the fixture is
 * one extras entry, and `value` is what selects the three cases:
 *   "true"  -> the marker is present
 *   "false" -> the marker is explicitly absent
 *   NULL    -> the key is absent entirely
 * Literal strings only; this effect is never handed to rb_free_ability. */
static AbilityEffect marker_effect(const char *value)
{
    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"gain_resource";
    if (value) {
        e.extra_k[0] = (char *)"fires_on_opponent_effects";
        e.extra_v[0] = (char *)value;
        e.n_extra = 1;
    }
    return e;
}

/* Three DISTINCT card ids (test_new_id, not test_id: the latter aliases one
 * shared template, so the "a movement of some OTHER card must not answer" case
 * would be vacuous). */
static int three_cards(int *out)
{
    out[0] = test_new_id(&tg, "PL!-sd1-010-SD");
    out[1] = test_new_id(&tg, "PL!S-sd1-001-SD");
    out[2] = test_new_id(&tg, "PL!HS-sd1-008-SD");
    return (out[0] >= 0 && out[1] >= 0 && out[2] >= 0 &&
            out[0] != out[1] && out[1] != out[2] && out[0] != out[2]);
}

/* Stage one AREA movement (stage -> stage) through the engine's OWN writer, so
 * the test pins the writer and the reader together rather than trusting a
 * hand-built turn_area_movements entry. `causer` is the seat on
 * RbBatchMovement.cause_player_id, which is what Rust's `player_id` string is
 * compared against at the guard. */
static void area_move(GameState *g, int card_id, int causer)
{
    rb_record_card_movement(g, card_id, RB_ZONEID_STAGE, RB_ZONEID_STAGE, causer, 1);
}

static void reset(void)
{
    test_game_new(&tg);
    tg.state.turn = 1;
}

/* ── 1. the marker pre-gate ──────────────────────────────────────────────── */

static void test_marker_gate_precedes_everything(void)
{
    int c[3];
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }

    area_move(&tg.state, c[0], 1);
    CHECK_EQ(tg.state.n_turn_area_movements, 1,
             "the area movement reached turn_area_movements");

    AbilityEffect yes = marker_effect("true");
    AbilityEffect no  = marker_effect("false");
    AbilityEffect none = marker_effect(NULL);

    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "a watcher flagged fires_on_opponent_effects IS owned by the hook "
             "after a foreign-caused area move");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &no, c[0], 0), 0,
             "fires_on_opponent_effects=\"false\" is not the marker "
             "(abilities.rs: the predicate is an exact \"true\" test)");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &none, c[0], 0), 0,
             "an effect with no fires_on_opponent_effects key is not the marker");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, NULL, c[0], 0), 0,
             "a null effect is not the marker");
    /* The gate is evaluated BEFORE the movement lookup: an unmarked watcher on a
     * card that was never moved must also answer 0, and so must an unmarked one
     * for a card that WAS moved by the opponent (the case above). */
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &none, c[1], 0), 0,
             "the gate is unconditional and does not consult the area log");
}

/* ── 2 & 3. the FOREIGN case, and the `cause_player_id == player_id` negative ── */

/* This pair IS the guard. `1` means "the movement hook already owns this
 * firing, so queue_zone_abilities must skip it"; `0` means "the generic scan
 * owns it". Getting the second case wrong is the whole failure mode: a `!=`
 * that is really an `==` (or a dropped test) suppresses EVERY marker-carrying
 * watcher on EVERY area move, the player's own included. */
static void test_foreign_cause_fires_and_own_cause_does_not(void)
{
    int c[3];
    AbilityEffect yes = marker_effect("true");

    /* Seat 0 watches; seat 1 caused the move. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 1);
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "FOREIGN cause (cause_player_id=1, watcher seat=0): the hook owns "
             "the firing (abilities.rs: rec.cause_player_id != player_id)");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 1), 0,
             "OWN cause (cause_player_id=1, watcher seat=1): the guard must NOT "
             "fire, or the player's own area move loses its watcher entirely");

    /* The mirror: seat 1 watches, seat 0 caused the move. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 0);
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 1), 1,
             "the same holds mirrored: cause_player_id=0, watcher seat=1");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 0,
             "and the mirror negative: cause_player_id=0, watcher seat=0");

    /* A cause by NEITHER seat is still "not this player", i.e. foreign. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 7);
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "a cause attributed to a third value is != the watcher seat, so it "
             "is foreign (Rust compares ids, it does not range-check them)");
}

/* ── 4. `.rev().find()` — the LAST matching movement answers ──────────────── */

static void test_last_matching_area_movement_wins(void)
{
    int c[3];
    AbilityEffect yes = marker_effect("true");

    /* Foreign FIRST, own SECOND: the newest movement for this card is the
     * player's own, so the guard must NOT fire. A forward scan answers 1 here,
     * which is the single easiest way to get this predicate wrong. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 1);
    area_move(&tg.state, c[0], 0);
    CHECK_EQ(tg.state.n_turn_area_movements, 2, "both area movements are logged");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 0,
             "own SECOND: the LAST matching movement decides, so a foreign move "
             "older than it does not own the firing (`.rev().find()`)");

    /* Own FIRST, foreign SECOND: the guard fires. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 0);
    area_move(&tg.state, c[0], 1);
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "foreign SECOND: the newest movement is the foreign one, so the "
             "hook owns the firing");

    /* An intervening movement of a DIFFERENT card must not stop the scan. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 1);
    area_move(&tg.state, c[1], 0);
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "another card's own-caused area move does not answer for this card "
             "(the find() is on moved_card_id, not on the newest entry)");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[1], 0), 0,
             "and that other card's own-caused move does not fire the guard");
}

/* ── 5. the VECTOR: the answer survives the batch being closed ────────────── */

static void test_reads_the_turn_scoped_log_not_the_batch(void)
{
    int c[3];
    AbilityEffect yes = marker_effect("true");

    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    area_move(&tg.state, c[0], 1);

    /* Non-vacuity: the same event IS in the per-batch set at this point, so
     * clearing the batch below is a real change and not a no-op. */
    CHECK(tg.state.n_batch_movements >= 1,
          "the area movement is in batch_movements too, before the batch closes");
    CHECK(tg.state.batch_movements[0].source_zone == RB_ZONEID_STAGE &&
          tg.state.batch_movements[0].dest_zone == RB_ZONEID_STAGE,
          "and it really is the stage->stage shape the old reader filtered for");

    /* Closing the batch: this is literally `g->n_batch_movements = 0`, the reset
     * at game_state_abilities.c / game_state_core.c and inside
     * rb_clear_card_movement_tracking. The old reader found nothing after this
     * and the guard silently stopped firing; Rust's record is turn-scoped. */
    tg.state.n_batch_movements = 0;
    CHECK_EQ(tg.state.n_batch_movements, 0, "the batch is closed");
    CHECK_EQ(tg.state.n_turn_area_movements, 1,
             "but the turn-scoped area log still holds the movement");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 1,
             "the guard still answers after the batch closed -- this is the "
             "precise regression the batch_movements -> turn_area_movements "
             "correction fixed, and it moved zero suite failures when landed");

    /* The turn rollover does clear it (modifiers.rs:
     * self.turn_area_movements.clear()), so the record is not unbounded. */
    rb_clear_card_movement_tracking(&tg.state);
    CHECK_EQ(tg.state.n_turn_area_movements, 0,
             "the turn-scoped log IS cleared at the turn boundary");
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 0,
             "after the clear there is no record left to answer from");
}

/* ── 6. only AREA movements reach the log ────────────────────────────────── */

/* Rust never asks "was this an area move" -- `turn_area_movements` holds only
 * area moves by construction (modifiers.rs pushes the event clone inside the
 * stage->stage arm). The old C reader had to re-derive it from the zone pair,
 * which is the second half of the defect the correction removed. */
static void test_non_area_movements_never_answer(void)
{
    int c[3];
    AbilityEffect yes = marker_effect("true");

    static const int to_zones[] = {
        RB_ZONEID_HAND, RB_ZONEID_DISCARD, RB_ZONEID_ENERGY, RB_ZONEID_LIVE_CARD_ZONE
    };
    static const char *names[] = { "hand", "discard", "energy", "live_card_zone" };

    for (int i = 0; i < 4; i++) {
        reset();
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        rb_record_card_movement(&tg.state, c[0], RB_ZONEID_STAGE, to_zones[i], 1, 1);
        char msg[160];
        snprintf(msg, sizeof msg,
                 "a stage -> %s movement is logged in batch_movements but NOT in "
                 "turn_area_movements, so it cannot own an area-move firing",
                 names[i]);
        CHECK(tg.state.n_batch_movements >= 1, msg);
        CHECK_EQ(tg.state.n_turn_area_movements, 0, "  ... and the area log is empty");
        snprintf(msg, sizeof msg,
                 "a stage -> %s movement never fires the guard", names[i]);
        CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[0], 0), 0, msg);
    }

    /* A card that was never moved at all. */
    reset();
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    CHECK_EQ(rb_hooked_by_foreign_cause(&tg.state, &yes, c[2], 0), 0,
             "a card with no area movement this turn does not fire the guard "
             "(the `else { return false }` arm)");
}

/* ── 7. corpus reachability, so the "no corpus ability" claim stays checkable ── */

/* The 33-suite A/B that proved the vector correction moved zero failures was a
 * statement about the corpus, so it is re-derivable here from the shipped
 * bytecode with the engine's own decoder. The number is informational; the
 * assertion is that the marker EXISTS in the corpus at all, which is what makes
 * the arm worth pinning rather than deleting. */
static void test_corpus_reachability(void)
{
    long effects = 0;
    long marked_effects = 0;
    long marked_abilities = 0;
    long marked_cards = 0;
    for (unsigned i = 0; i < rb_num_abilities(); i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_ability(i, &ab)) continue;
        effects++;
        int marked = 0;
        /* The marker is an EFFECT key, so scan the effect tree the same way the
         * C predicate does: it is read off the effect, never off the card. */
        const AbilityEffect *stack[64];
        int sp = 0;
        stack[sp++] = ab.effect;
        while (sp > 0 && !marked) {
            const AbilityEffect *e = stack[--sp];
            if (!e) continue;
            for (int k = 0; k < e->n_extra; k++)
                if (e->extra_k[k] && !strcmp(e->extra_k[k], "fires_on_opponent_effects") &&
                    e->extra_v[k] && !strcmp(e->extra_v[k], "true")) { marked = 1; break; }
            if (marked) break;
            for (int q = 0; q < e->n_child && sp < 60; q++) stack[sp++] = e->child[q];
            const AbilityEffect *kids[] = { e->primary_effect, e->alternative_effect,
                                            e->look_action, e->select_action,
                                            e->followup_action, e->optional_action,
                                            e->conditional_action, e->gained_effect,
                                            e->resource_on_select, e->opponent_action };
            for (int q = 0; q < 10 && sp < 60; q++) stack[sp++] = kids[q];
        }
        if (marked) {
            marked_abilities++;
            for (int cid = 0; cid < rb_num_cards(); cid++) {
                int nab = rb_card_num_abilities((uint32_t)cid);
                for (int n = 0; n < nab; n++) {
                    uint32_t gi = 0;
                    if (rb_card_get_ability_idx((uint32_t)cid, n, &gi) && gi == i) {
                        marked_cards++;
                        break;
                    }
                }
            }
        }
        if (marked) marked_effects++;
        rb_free_ability(&ab);
    }
    printf("CENSUS abilities=%ld marked_abilities=%ld marked_cards=%ld\n",
           effects, marked_abilities, marked_cards);
    CHECK(marked_abilities > 0,
          "the corpus really does carry fires_on_opponent_effects markers, so "
          "this arm is reachable in principle and must stay pinned");
    CHECK(marked_abilities < effects,
          "and they are a strict minority, which is why the 33-suite A/B saw "
          "zero movement: no corpus AREA MOVE reaches the generic TAS scan with "
          "a marker-bearing watcher on the moved card");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_marker_gate_precedes_everything();
    test_foreign_cause_fires_and_own_cause_does_not();
    test_last_matching_area_movement_wins();
    test_reads_the_turn_scoped_log_not_the_batch();
    test_non_area_movements_never_answer();
    test_corpus_reachability();

    printf("\n%d assertions, %d failures\n", assertions, failures);
    return failures ? 1 : 0;
}
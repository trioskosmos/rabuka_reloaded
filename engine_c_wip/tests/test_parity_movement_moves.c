#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

/* ── per-test verdict layer (same shape as tests/test_surplus_heart.c:11-22):
 * a FLAT suite's FAIL lines name an assertion, never a test, so this suite
 * reports exactly one `FAILED: <test body>` per failing test body. No assertion
 * is added, removed or relaxed. */
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

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
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
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── condition builders ──────────────────────────────────────────────────
 * These assemble the two shapes the corpus actually uses for
 * `movement:"moves"`, measured over cards/abilities.json:
 *   - flags at the CONDITION's top level, and
 *   - flags inside the NESTED `trigger_event` object.
 * The second shape is the one the arm actually has to read: all 8 corpus
 * `movement:"moves"` nodes carry self_effect_only / energy_placed /
 * watches_area_move / destination inside `trigger_event`, never at the top
 * level (Rust reads them with `condition.get_X().or_else(|| te.and_then(..))`,
 * condition/state.rs:918-923 / :979-982 / :1082-1084). */

static void add_str(Condition *c, const char *key, const char *val) {
    CondField *f = &c->fields[c->n_fields++];
    f->key = (char *)key;
    f->v.tag = RB_TAG_STR;
    f->v.s = (char *)val;
}
static void add_bool(Condition *c, const char *key, int val) {
    CondField *f = &c->fields[c->n_fields++];
    f->key = (char *)key;
    f->v.tag = val ? RB_TAG_TRUE : RB_TAG_FALSE;
    f->v.b = val;
}
static void add_cond(Condition *c, const char *key, Condition *val) {
    CondField *f = &c->fields[c->n_fields++];
    f->key = (char *)key;
    f->v.tag = RB_TAG_OBJVAR;
    f->v.cond = val;
}

/* A bare `movement:"moves"` condition with no watch flags at all. */
static void bare_moves_condition(Condition *c) {
    memset(c, 0, sizeof(*c));
    c->variant = RB_COND_MOVEMENT;
    add_str(c, "movement", "moves");
}

/* `movement:"moves"` whose flags live in the nested trigger_event, which is how
 * the corpus carries them. */
static void moves_condition_with_trigger_event(Condition *c, Condition *te,
                                               int energy_placed, int has_energy,
                                               int watches_area, int self_only) {
    memset(c, 0, sizeof(*c));
    c->variant = RB_COND_MOVEMENT;
    add_str(c, "movement", "moves");
    memset(te, 0, sizeof(*te));
    te->variant = 0xFF;
    if (has_energy) add_bool(te, "energy_placed", energy_placed);
    if (watches_area) add_bool(te, "watches_area_move", 1);
    if (self_only) add_bool(te, "self_effect_only", 1);
    add_cond(c, "trigger_event", te);
}

static void record_move(GameState *g, int card_id, RbZoneId dest, int effect_only,
                        int cause_player) {
    RbBatchMovement *m = &g->batch_movements[g->n_batch_movements++];
    m->moved_card_id = card_id;
    m->source_zone = RB_ZONEID_HAND;
    m->dest_zone = (int)dest;
    m->cause_player_id = cause_player;
    m->effect_only = effect_only;
}

static void record_pce(GameState *g, int card_id) {
    RbPositionChangeEvent *e = &g->position_change_events[g->n_position_change_events++];
    e->card_id = card_id;
    e->position = 0;
}

/* ── tests ─────────────────────────────────────────────────────────────── */

/* THE DISAMBIGUATION PIN (condition/state.rs:1085-1089).
 *
 * A bare `movement:"moves"` node sets neither energy_placed nor
 * watches_area_move, so Rust's
 *     has_area_check = watches_area.unwrap_or(false)
 *                   || (energy_placed.is_none()
 *                       && (self_effect_only.is_some()
 *                           || condition.get_movement().unwrap_or("") == "moves"))
 * takes its SECOND disjunct. The `get_movement() == "moves"` half is
 * invariantly true -- the arm is only entered through
 * `match condition.get_movement() { "moves" => ... }` (state.rs:864-917) -- so
 * has_area_check IS true and the verdict is area_ok, NOT the vacuous `true`
 * from state.rs:1094-1095.
 *
 * This is the case that separates a correct port from one that "cleans up" the
 * `|| 1` tautology by dropping the disjunct it belongs to: with nothing moved,
 * area_ok is false (state.rs:962-967), so the answer is 0. A port that let
 * has_area_check fall to false here would return 1 and silently re-arm every
 * `movement:"moves"` watcher in the corpus. */
static void bare_moves_node_is_area_checked_not_vacuously_true(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c;
    bare_moves_condition(&c);

    tg.state.activating_card = 1001;
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 0,
             "bare movement:moves with NOTHING moved is 0: has_area_check is true "
             "(state.rs:1086-1088, movement==\"moves\" is invariant) so the verdict is "
             "area_ok, which is false (state.rs:962-967) -- not the vacuous true of "
             "state.rs:1094-1095");
}

/* 「このメンバーがエリアを移動する」 -- THIS card, not merely "a card"
 * (state.rs:962-967, the explicit `false` arm). A different card moving must
 * not satisfy it. */
static void a_different_card_moving_does_not_satisfy_moves(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c;
    bare_moves_condition(&c);

    tg.state.activating_card = 1001;
    record_pce(&tg.state, 2002);            /* NOT the activating card */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 0,
             "a DIFFERENT card's position change does not satisfy "
             "movement:moves (state.rs:935-961 in_pce tests moved_card_id == cid; "
             "area_ok is false at state.rs:962-967)");
}

/* The activating card's own position change satisfies it. */
static void this_card_position_change_satisfies_moves(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c;
    bare_moves_condition(&c);

    tg.state.activating_card = 1001;
    record_pce(&tg.state, 1001);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 1,
             "the activating card's own position change satisfies movement:moves "
             "(state.rs:936-940 in_pce, then area_ok at state.rs:962-976)");
}

/* energy_placed arriving via the NESTED trigger_event arms has_energy_check
 * (state.rs:1089), which switches the verdict from area_ok to energy_ok. */
static void nested_energy_placed_arms_the_energy_check(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 0, 0);

    tg.state.activating_card = -1;
    /* energy_placed present but nothing placed -> energy_val false. */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "energy_placed in the nested trigger_event with an empty batch is 0 "
             "(state.rs:977 energy_placed.is_none_or runs the body; energy_val is "
             "false at state.rs:1009, so the verdict at state.rs:1060 is false)");

    /* A watched placement -- energy destination AND effect_only -- satisfies it. */
    record_move(&tg.state, 3003, RB_ZONEID_ENERGY, 1, 0);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 1,
             "an effect_only arrival into energy satisfies the nested energy_placed "
             "(state.rs:988-995 is_watched_placement + state.rs:1060)");
}

/* is_watched_placement REQUIRES effect_only (state.rs:994). */
static void energy_placement_without_effect_only_is_not_watched(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 0, 0);

    tg.state.activating_card = -1;
    record_move(&tg.state, 3003, RB_ZONEID_ENERGY, 0, 0);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "an arrival into energy that is NOT effect_only is not watched "
             "(state.rs:994 `dz_ok && m.effect_only`)");
}

/* The destination constraint. Rust compares dest_zone as a STRING; the C stores
 * the same field as an RbZoneId, so a port that compared with strcmp would not
 * even compile, and one that compared the raw ints would mis-read the
 * energy/energy_zone alias. Only the named zone may arm the watcher. */
static void destination_constrains_the_watched_placement(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 0, 0);
    add_str(&te, "destination", "under_member");

    tg.state.activating_card = -1;
    record_move(&tg.state, 3003, RB_ZONEID_ENERGY, 1, 0);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "destination=\"under_member\" is not satisfied by an arrival into energy "
             "(state.rs:989-994: a non-empty want_dest compares against that zone only)");

    record_move(&tg.state, 3004, RB_ZONEID_UNDER_MEMBER, 1, 0);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 1,
             "destination=\"under_member\" IS satisfied by an effect_only arrival "
             "under a member (state.rs:989-994)");
}

/* watches_area_move + energy_placed present => BOTH checks, OR'd
 * (state.rs:1094-1101). Area true / energy false must still pass. */
static void both_checks_are_ored(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 1, 0);

    tg.state.activating_card = 1001;
    record_pce(&tg.state, 1001);            /* area_ok true */
    /* energy_ok false: nothing placed */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 1,
             "watches_area_move + energy_placed -> area_ok || energy_ok "
             "(state.rs:1096-1097); area_ok alone carries it");
}

/* Both checks present, both false -> 0. */
static void both_checks_false_is_false(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 1, 0);

    tg.state.activating_card = -1;          /* area_ok false */
    /* energy_ok false: nothing placed */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &c), 0,
             "watches_area_move + energy_placed with neither satisfied is 0 "
             "(state.rs:1096-1097 `area_ok || energy_ok`)");
}

/* watches_area_move present and energy_placed ABSENT => area check only
 * (state.rs:1085-1089, :1098-1099). */
static void watches_area_move_alone_selects_the_area_check(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 0, 0, 1, 0);

    tg.state.activating_card = 1001;
    record_pce(&tg.state, 1001);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 1,
             "watches_area_move with energy_placed ABSENT is area-checked only "
             "(state.rs:1085 has_area_check, :1098-1099 returns area_ok)");
}

/* The energy check is reached at all ONLY because the nested trigger_event is
 * consulted. If a port read the top level alone, energy_placed would look
 * absent, has_energy_check would be false, and every energy watcher would
 * silently answer area_ok instead. This pins the `te` fallback
 * (state.rs:921-923). */
static void energy_check_needs_the_trigger_event_fallback(void) {
    static TestGame tg;
    test_game_new(&tg);
    Condition c, te;
    moves_condition_with_trigger_event(&c, &te, 1, 1, 0, 0);

    tg.state.activating_card = 1001;
    record_pce(&tg.state, 1001);            /* area_ok would be TRUE */
    /* No placement at all -> energy_ok false. If the port ignored `te`,
     * energy_placed would read absent, has_energy_check false, and the verdict
     * would be area_ok = 1. */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, 1001, &c), 0,
             "the nested energy_placed is honoured, so area_ok alone does NOT carry "
             "the verdict (state.rs:921-923 te fallback; :1089 has_energy_check; "
             ":1100 returns energy_ok)");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    bare_moves_node_is_area_checked_not_vacuously_true();
    a_different_card_moving_does_not_satisfy_moves();
    this_card_position_change_satisfies_moves();
    nested_energy_placed_arms_the_energy_check();
    energy_placement_without_effect_only_is_not_watched();
    destination_constrains_the_watched_placement();
    both_checks_are_ored();
    both_checks_false_is_false();
    watches_area_move_alone_selects_the_area_check();
    energy_check_needs_the_trigger_event_fallback();
    rb_unload();
    if (failures) return 1;
    printf("ALL MOVEMENT:MOVES PARITY CHECKS PASSED\n");
    return 0;
}
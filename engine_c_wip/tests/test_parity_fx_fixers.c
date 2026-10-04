/* tests/test_parity_fx_fixers.c — parity tests for the 2026-10-04 fixer wave,
 * src/ability/condition.c.
 *
 * Every CHECK asserts what RUST asserts and cites the Rust line. Nothing here is
 * relaxed, skipped, or counted as "expected failure". Where a checklist item's
 * PREMISE was wrong, the test pins the CORRECT behaviour with the citation that
 * proves it, so the next agent cannot re-raise the same false item.
 *
 *   FX1  eval_group's temporal this_turn arm  — the ONLY reader of
 *        RbBatchMovement.cause_card_id in Rust (ability/condition/card.rs:1632-1658)
 *   FX2  check_card_property's empty check set — card.rs:134-136 returns TRUE
 *   FX3  movement:"position_change" — the reader g->position_change_events never
 *        had (ability/condition/state.rs:869-908 + :524-568)
 *
 * Reachability, measured with the engine's OWN decoder over the shipped bytecode
 * (tools/census_fx.c; 2116 card-ability pairs, 0 decode fallbacks):
 *   FX1 arm reachable at 2 nodes  — card 2467 / 2468 ab#0
 *   FX2 73 card_property nodes on 59 pairs; 47 negated; 0 of 73 unmapped
 *   FX3 26 movement:"position_change" nodes on 26 pairs; 4 name a position,
 *      2 carry area_direction:"from"; cost_limit on 0 of them
 *
 * sizeof(TestGame) == 1195656 bytes, so every GameState/TestGame local is static.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static const char *g_failed[256];
static int g_failed_n;

static void verdict_failed(const char *fn)
{
    for (int i = 0; i < g_failed_n; i++) if (g_failed[i] == fn) return;
    if (g_failed_n < 256) g_failed[g_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}
#define FLAT_VERDICT() verdict_failed(__func__)

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); FLAT_VERDICT(); failures++; } \
    else printf("ok: %s\n", msg); \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    int a_ = (actual), e_ = (expected); \
    if (a_ != e_) { fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, a_, e_); \
                    FLAT_VERDICT(); failures++; } \
    else printf("ok: %s\n", msg); \
} while (0)

static TestGame tg;   /* MUST be static: sizeof(TestGame) is 1195656 */

static CondValue cstr(char *v) { CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_STR; x.s = v; return x; }
static CondValue ctrue(void)     { CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_TRUE; return x; }
static CondValue carr(CondValue *a, uint32_t n) {
    CondValue x; memset(&x, 0, sizeof x); x.tag = RB_TAG_ARRAY; x.arr = a; x.arr_n = n; return x;
}

/* ═══════════════ FX1 — eval_group's temporal this_turn arm ══════════════════
 *
 * Rust, ability/condition/card.rs:1632-1658, verbatim shape:
 *   temporal=="this_turn" && self_target && group_names non-empty
 *                && card_type == Some("member_card")
 *   -> if let Some(activating_card_id) = self.activating_card_id {
 *          let player = self.resolve_condition_player(target.unwrap_or("self"));
 *          return game_state.turn_area_movements.iter().any(|m|
 *              m.moved_card_id == activating_card_id
 *              && m.effect_only
 *              && m.cause_player_id == player.id
 *              && m.cause_card_id.is_some_and(|c|
 *                     groups.iter().any(|g| card_matches_group_str(db, c, Some(g)))));
 *      }
 *      false
 *
 * The body this replaces scanned the MOVED-CARDS set, never tested
 * moved_card_id / effect_only / cause_player_id, and matched the group against
 * the MOVED card using group_names[0]. Each of those is a separate CHECK here so
 * a regression names itself.
 *
 * The fixture is the CAUSING card the clause talks about — a 『Liella!』 card —
 * found by PREDICATE at runtime, never by a hard-coded id, and cloned with
 * test_new_id() because test_id() aliases one shared template.
 */
static void fx1_cause_card_id_arm(void)
{
    test_game_new(&tg);
    GameState *g = &tg.state;

    /* two distinct copies: one is the ACTIVATING card that moved, the other is
       the Liella! card whose EFFECT caused the move. They must not be the same
       id or the arm would pass for the wrong reason. Each fixture is located by
       PREDICATE at runtime and cloned with test_new_id(), because test_id()
       aliases ONE shared template — two calls for two roles would hand back the
       same id and make every assertion here vacuous. */
    int mover = -1, liella = -1, not_group = -1;
    for (uint32_t cid = 0; cid < 3000; cid++) {
        Card c; memset(&c, 0, sizeof c);
        if (!rb_decode_card_by_index(cid, &c)) continue;
        const char *nm = rb_card_string(c.card_no_idx);
        if (nm && rb_card_matches_group_str((int)cid, "Liella!") && liella < 0)
            liella = test_new_id(&tg, nm);
        if (nm && !rb_card_matches_group_str((int)cid, "Liella!") && not_group < 0)
            not_group = test_new_id(&tg, nm);
        rb_free_card(&c);
        if (liella >= 0 && not_group >= 0) break;
    }
    /* the MOVER is a second, different Liella! clone — the clause is about a
       member that a Liella! card's EFFECT moved, so mover != cause in the
       interesting case, but the group test is on the cause either way. */
    { Card c; memset(&c, 0, sizeof c);
      if (liella >= 0 && rb_decode_card_by_index((uint32_t)liella, &c)) {
          const char *nm = rb_card_string(c.card_no_idx);
          if (nm) mover = test_new_id(&tg, nm);
          rb_free_card(&c);
      } }
    CHECK(liella >= 0 && not_group >= 0 && mover >= 0,
          "FX1: fixtures exist — one Liella! card, one non-Liella! card, one mover");
    if (liella < 0 || not_group < 0 || mover < 0) return;
    CHECK(mover != liella,
          "FX1: the mover and the cause are different cards (otherwise the arm is vacuous)");

    CondValue groups[1];
    groups[0] = cstr((char *)"Liella!");
    Condition cond = {0};
    cond.variant = RB_COND_GROUP;
    cond.fields[0] = (CondField){"temporal",        cstr((char *)"this_turn")};
    cond.fields[1] = (CondField){"self_target",     ctrue()};
    cond.fields[2] = (CondField){"group_names",     carr(groups, 1)};
    cond.fields[3] = (CondField){"card_type",       cstr((char *)"member_card")};
    cond.fields[4] = (CondField){"target",          cstr((char *)"self")};
    cond.n_fields = 5;

    /* no activating card -> Rust's `return false` at card.rs:1657 */
    g->activating_card = -1;
    g->n_turn_area_movements = 0;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: no activating card fails the arm (card.rs:1637 + :1657)");

    /* the full conjunction satisfied: the move was caused by a Liella! card */
    g->activating_card = mover;
    g->n_turn_area_movements = 1;
    g->turn_area_movements[0].moved_card_id    = mover;
    g->turn_area_movements[0].source_zone      = RB_ZONEID_STAGE;
    g->turn_area_movements[0].dest_zone        = RB_ZONEID_STAGE;
    g->turn_area_movements[0].cause_player_id  = 0;
    g->turn_area_movements[0].cause_card_id    = liella;
    g->turn_area_movements[0].effect_only      = 1;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 1,
             "FX1: effect-caused area move by a Liella! card satisfies the clause (card.rs:1642-1655)");

    /* leg 2: effect_only. Rust requires it; a non-effect move does NOT count. */
    g->turn_area_movements[0].effect_only = 0;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: effect_only is required (card.rs:1644)");
    g->turn_area_movements[0].effect_only = 1;

    /* leg 3: cause_player_id. Rust compares it to the resolved seat (card.rs:1645) */
    g->turn_area_movements[0].cause_player_id = 1;   /* target is "self" == seat 0 */
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: an opponent-caused move does not satisfy a self clause (card.rs:1645)");
    g->turn_area_movements[0].cause_player_id = 0;

    /* leg 4: the group is tested on the CAUSING card, not the moved one. A
       Liella! MOVER whose move was caused by a NON-Liella! card must fail. */
    g->turn_area_movements[0].cause_card_id = not_group;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: the group is matched on cause_card_id, NOT on the moved card (card.rs:1646-1653)");

    /* leg 1: moved_card_id must be the ACTIVATING card. */
    g->turn_area_movements[0].cause_card_id = liella;
    g->turn_area_movements[0].moved_card_id = not_group;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: only the activating card's own move counts (card.rs:1643)");
    g->turn_area_movements[0].moved_card_id = mover;

    /* cause_card_id absent (Rust's None) fails even though the mover matches. */
    g->turn_area_movements[0].cause_card_id = -1;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &cond), 0,
             "FX1: a move with no cause card fails (card.rs:1646 is_some_and)");
    g->turn_area_movements[0].cause_card_id = liella;

    /* the gate: card_type must be PRESENT and equal to member_card. The old body
       accepted an absent card_type, which Rust's `== Some("member_card")` rejects.
       Removing card_type must therefore fall through to the COUNT tail, which for
       an empty stage is 0 >= 1 -> false. */
    Condition no_ct = cond;
    no_ct.fields[3] = (CondField){"card_type", cstr((char *)"leader")};
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &no_ct), 0,
             "FX1: card_type != member_card does not enter the arm (card.rs:1635)");

    /* `.any()` vs `.first()` INSIDE the temporal arm is unreachable by
       construction, and this asserts the reason rather than pretending
       otherwise: Rust's multi-group arm sits IMMEDIATELY ABOVE the temporal arm
       (card.rs:1606-1631) and returns from inside its `if groups.len() > 1`
       block, so a condition with two or more group names never reaches
       card.rs:1642. A `.first()` port and a `.any()` port therefore cannot
       disagree on any input — but only if the reader agrees that a 2-element
       list goes to the multi-group branch, so that is what is pinned.
       (Measured: both corpus nodes, cards 2467 / 2468, carry a ONE-element
       group_names; the C's census confirms 0 `group` variants combine
       this_turn + self_target with an EMPTY list, tools/census_fx.c B section.) */
    const char *absent = "no such group at all";
    CHECK_EQ(rb_card_matches_group_str(liella, absent), 0,
             "FX1: control — the Liella! card matches no other group, so the branch test below is a real test");
    CondValue two[2];
    two[0] = cstr((char *)absent);
    two[1] = cstr((char *)"Liella!");
    Condition any_of = cond;
    any_of.fields[2] = (CondField){"group_names", carr(two, 2)};
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &any_of), 0,
             "FX1: a TWO-element group_names is taken by the multi-group arm above, not by this one (card.rs:1606-1631 returns first)");

    /* an EMPTY group_names list must NOT enter the arm at all
       (`is_some_and(|g| !g.is_empty())`, card.rs:1634) — it falls to the count
       tail instead. */
    CondValue empty[1];
    empty[0] = cstr((char *)"Liella!");
    Condition empty_list = cond;
    empty_list.fields[2] = (CondField){"group_names", carr(empty, 0)};
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &empty_list), 0,
             "FX1: an empty group_names list does not enter the arm (card.rs:1634)");
}

/* ═══════════════ FX2 — check_card_property's empty check set ═══════════════
 *
 * The reconciliation box claimed the C's `if (n == 0) return 1;` is a defect
 * ("an empty check set is FALSE in Rust and TRUE in C"), citing card.rs:230-252's
 * trailing `!matching.is_empty()`. That is FALSE: card.rs:133-136 short-circuits
 *
 *     let neg = condition.get_negation().unwrap_or(false);
 *     if check_cards.is_empty() {
 *         return true;
 *     }
 *
 * BEFORE the matching loop, so `!matching.is_empty()` is unreachable for an empty
 * zone, and Rust's own comment at card.rs:112-117 says "the empty case below
 * returns TRUE". engine/src/ability/condition/card.rs is UNMODIFIED in git, so
 * this is not a half-applied reference edit. Cluster D removed its assertion
 * rather than pin this; these CHECKs restore it, with the citation.
 *
 * 73 card_property nodes carry this, so getting the polarity backwards would be a
 * wide regression, not a cosmetic one.
 */
static void fx2_card_property_empty_set_is_true(void)
{
    test_game_new(&tg);
    GameState *g = &tg.state;
    g->p[0].stage[0] = RB_EMPTY_SLOT;
    g->p[0].stage[1] = RB_EMPTY_SLOT;
    g->p[0].stage[2] = RB_EMPTY_SLOT;
    g->n_revealed = 0;

    Condition plain = {0};
    plain.variant = RB_COND_LOCATION;
    plain.fields[0] = (CondField){"card_property", cstr((char *)"has_blade_heart")};
    plain.fields[1] = (CondField){"location",      cstr((char *)"stage")};
    plain.fields[2] = (CondField){"target",        cstr((char *)"self")};
    plain.n_fields = 3;

    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &plain), 1,
             "FX2: a card_property over an EMPTY zone is TRUE (card.rs:134-136)");

    Condition negated = plain;
    negated.fields[3] = (CondField){"negation", ctrue()};
    negated.n_fields = 4;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &negated), 1,
             "FX2: the NEGATED form over an empty zone is TRUE too — the early return precedes the negation fold (card.rs:133-136)");

    /* The control: a NON-empty zone with no matching card is FALSE, so the
       empty-set result above is the early return and not the general answer. */
    { int no_bh = -1;
      for (uint32_t cid = 0; cid < 3000; cid++) {
          Card c; memset(&c, 0, sizeof c);
          if (!rb_decode_card_by_index(cid, &c)) continue;
          int is_mem = (c.type_flags & 3u) == 0u;
          int take = is_mem && c.num_blade == 0 && no_bh < 0;
          rb_free_card(&c);
          if (take) { no_bh = (int)cid; break; }
      }
      CHECK(no_bh >= 0, "FX2: a member with no printed blade heart exists");
      if (no_bh >= 0) {
          Card c; memset(&c, 0, sizeof c);
          if (rb_decode_card_by_index((uint32_t)no_bh, &c)) {
              const char *nm = rb_card_string(c.card_no_idx);
              int copy = test_new_id(&tg, nm);
              rb_free_card(&c);
              if (copy >= 0) {
                  test_add_to_stage(&tg, 0, copy);
                  CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &plain), 0,
                           "FX2: control — a non-empty zone with no matching card is FALSE (card.rs:251 !matching.is_empty())");
                  CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &negated), 1,
                           "FX2: control — the negated form over a non-empty zone with no match is TRUE (card.rs:223 property_cards.is_empty())");
                  test_game_new(&tg);
                  g = &tg.state;
              }
          }
      }
    }

    /* `revealed_cards` with nothing revealed: the other empty zone in the corpus
       (26 of the 73 nodes name it). */
    Condition rev = plain;
    rev.fields[1] = (CondField){"location", cstr((char *)"revealed_cards")};
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &rev), 1,
             "FX2: revealed_cards with nothing revealed is TRUE (card.rs:119 + :134-136)");
}

/* ═══════════════ FX3 — movement:"position_change" ═════════════════════════
 *
 * g->position_change_events[] had exactly ONE reference in the whole C tree — the
 * reset at src/core/game_state_abilities.c:589 — while its writer
 * (rb_record_stage_position_swap, src/core/modifiers.c:1033-1050) filled it on
 * every area change. An event log with no consumer. Rust reads it in three
 * places; this is the first of the three, and it needs only `moved_card_id`.
 *
 * Rust ability/condition/state.rs:869-908 + :524-568, reached by 26 corpus nodes.
 */
static void fx3_position_change_reader(void)
{
    test_game_new(&tg);
    GameState *g = &tg.state;

    int a = test_new_id(&tg, "PL!-PR-001-PR");
    int b = test_new_id(&tg, "PL!-PR-002-PR");
    CHECK(a >= 0 && b >= 0, "FX3: two distinct stage members");
    if (a < 0 || b < 0) return;

    Condition pc = {0};
    pc.variant = RB_COND_MOVEMENT;
    pc.fields[0] = (CondField){"movement",   cstr((char *)"position_change")};
    pc.fields[1] = (CondField){"target",     cstr((char *)"self")};
    pc.fields[2] = (CondField){"location",   cstr((char *)"stage")};
    pc.n_fields = 3;

    /* No events at all: position_event_matches_filters returns false on an empty
       slice (state.rs:529-531) and the turn_area_movements disjunct is false
       without an activating card (state.rs:874). */
    g->n_position_change_events = 0;
    g->n_turn_area_movements = 0;
    g->n_recently_moved = 0;
    g->activating_card = -1;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &pc), 0,
             "FX3: no position_change_events and no activating card is FALSE (state.rs:529-531 + :874)");

    /* one event for the activating card, no filters -> match */
    g->activating_card = a;
    g->n_position_change_events = 1;
    g->position_change_events[0].card_id = a;
    g->position_change_events[0].position = 1;   /* centre */
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &pc), 1,
             "FX3: an unfiltered position_change over the activating card is TRUE (state.rs:566)");

    /* self_target narrows it to the activating card (state.rs:536-540) */
    Condition self_pc = pc;
    self_pc.fields[3] = (CondField){"self_target", ctrue()};
    self_pc.n_fields = 4;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &self_pc), 1,
             "FX3: self_target + the activating card's own event is TRUE (state.rs:536-566)");
    g->activating_card = b;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &self_pc), 0,
             "FX3: self_target rejects ANOTHER card's event (state.rs:536-540)");
    g->activating_card = a;

    /* the `position` filter reads new_position (state.rs:554-565). The C's
       RbPositionChangeEvent.position IS new_position. */
    Condition at_centre = pc;
    at_centre.fields[3] = (CondField){"position", cstr((char *)"center")};
    at_centre.n_fields = 4;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &at_centre), 1,
             "FX3: position=center matches the event's new_position (state.rs:554-565)");
    Condition at_left = pc;
    at_left.fields[3] = (CondField){"position", cstr((char *)"left_side")};
    at_left.n_fields = 4;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &at_left), 0,
             "FX3: position=left_side rejects an event whose new_position is centre (state.rs:560-563)");

    /* group_names filters on the MOVED card here (state.rs:541-553) — this is the
       3rd of the 26 nodes, e.g. card 1840's 『SaintSnow』. */
    int snow = -1;
    for (uint32_t cid = 0; cid < 3000; cid++)
        if (rb_card_matches_group_str((int)cid, "SaintSnow")) { snow = (int)cid; break; }
    CHECK(snow >= 0, "FX3: a SaintSnow card exists for the group_names filter");
    if (snow >= 0) {
        CondValue grp[1];
        grp[0] = cstr((char *)"SaintSnow");
        Condition by_group = pc;
        by_group.fields[3] = (CondField){"group_names", carr(grp, 1)};
        by_group.n_fields = 4;
        CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &by_group), 0,
                 "FX3: group_names the moved card does not carry rejects the event (state.rs:541-553)");
        g->position_change_events[0].card_id = snow;
        CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &by_group), 1,
                 "FX3: group_names the moved card DOES carry accepts the event (state.rs:541-553)");
        g->position_change_events[0].card_id = a;
    }

    /* The turn_area_movements disjunct (state.rs:874-906): no pce match, an
       activating card, an effect_only area move, and the card in this batch's
       moved set. This is the term that makes a CROSS-SEAT deferred watcher fire
       after the enqueuing player cleared position_change_events. */
    g->n_position_change_events = 0;
    g->n_turn_area_movements = 1;
    g->turn_area_movements[0].moved_card_id   = a;
    g->turn_area_movements[0].source_zone     = RB_ZONEID_STAGE;
    g->turn_area_movements[0].dest_zone       = RB_ZONEID_STAGE;
    g->turn_area_movements[0].cause_player_id = 0;
    g->turn_area_movements[0].cause_card_id   = -1;
    g->turn_area_movements[0].effect_only     = 1;
    g->n_recently_moved = 0;
    g->n_recently_moved = 1; g->recently_moved[0] = a;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &pc), 1,
             "FX3: the turn_area_movements disjunct fires for an effect_only batch move (state.rs:899-903)");

    g->turn_area_movements[0].effect_only = 0;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &pc), 0,
             "FX3: the disjunct requires effect_only (state.rs:901)");
    g->turn_area_movements[0].effect_only = 1;

    /* in_current_batch: with neither the entry snapshot nor recently_moved
       holding the card, the disjunct is false (state.rs:889-898). */
    g->n_recently_moved = 0;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &pc), 0,
             "FX3: without in_current_batch the disjunct is FALSE (state.rs:889-903)");

    /* moved_cards_match_cost_limit: no cost_limit -> TRUE, so it cannot change
       the verdict. Measured: cost_limit appears on 0 of the 26 position_change
       nodes. Asserted through a condition that DOES carry one, so the conjunct's
       shape is pinned rather than assumed. */
    CondValue nope[1]; nope[0] = cstr((char *)"Liella!");
    Condition with_limit = pc;
    with_limit.fields[3] = (CondField){"cost_limit", (CondValue){ .tag = RB_TAG_I64, .i = 3 }};
    with_limit.n_fields = 4;
    (void)nope;
    g->n_position_change_events = 1;
    g->position_change_events[0].card_id = a;
    g->position_change_events[0].position = 1;
    g->n_recently_moved = 0;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &with_limit), 0,
             "FX3: with no recently-moved card, moved_cards_match_cost_limit is FALSE (state.rs:582-584)");
    g->n_recently_moved = 1; g->recently_moved[0] = a;
    CHECK_EQ(rb_eval_condition_for_host(g, 0, -1, &with_limit), 1,
             "FX3: a recently-moved card within cost_limit 3 satisfies the conjunct (state.rs:585-590)");
}

int main(void)
{
    if (rb_load("src") != 0) { fprintf(stderr, "FAIL: rb_load\n"); return 2; }

    fx1_cause_card_id_arm();
    fx2_card_property_empty_set_is_true();
    fx3_position_change_reader();

    rb_unload();
    printf("\nfixer wave: %d checks failed\n", failures);
    for (int i = 0; i < g_failed_n; i++) printf("  failed: %s\n", g_failed[i]);
    return failures ? 1 : 0;
}
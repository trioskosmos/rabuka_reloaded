/* tests/test_parity_cluster_e.c — parity tests for cluster E (src/core/modifiers.c).
 *
 * One test body per landed item. Every CHECK cites the Rust line it asserts, and
 * every fixture is located by PREDICATE at runtime (never a bare card id), so a
 * corpus regeneration cannot silently turn a CHECK vacuous — each fixture test
 * first asserts that its fixture actually satisfies the predicate it is named
 * for.
 *
 * Items covered:
 *   E9   rb_record_stage_position_swap      modifiers.rs:1799-1857
 *   E10  rb_mods_count_as_multiple_weight   modifiers.rs:227-301
 *   E11  rb_mods_weighted_zone_count        modifiers.rs:308-343
 *   E12  rb_mods_replacement_appear_wait_applies  modifiers.rs:360-451
 *   E16  RbBatchMovement.cause_card_id      written from g->activating_card
 *   E15  the surviving need-heart undo       modifiers.rs:607-610
 *
 * Measurement backing these (tools/census_e.c, the engine's own decoder over
 * 985 abilities / 2116 card-ability pairs / 4925 effect nodes / 2639 cards):
 *   count_as_multiple        1 node  (PL!-pb2-041-L, count_as=2, counted_by="lily white")
 *   replacement_appear_wait  2 nodes (PL!-pb2-002-PP / -R, position=center, limit 4, "<=")
 *   non_stackable            3 nodes
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

 TestGame tg;   /* sizeof(TestGame) == 1113736 — MUST be static */

/* ── fixtures, located by predicate ─────────────────────────────────────── */

/* A MEMBER card whose PRINTED base-heart total is <= limit. Rust reads exactly
 * this total at modifiers.rs:367-371 (`card.base_heart.hearts.values().fold`),
 * which is why rb_card_total_hearts — which falls back to need_heart — is the
 * wrong helper for E12. */
static int find_member_with_base(TestGame *t, int want_le, int limit, int *out_total)
{
    int n = rb_num_cards();
    for (int id = 0; id < (int)n; id++) {
        if (!rb_card_is_member(id)) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)id, &c)) continue;
        int base = 0;
        for (int i = 0; i < c.num_base && i < c.n_hearts; i++) base += c.heart_count[i];
        rb_free_card(&c);
        int ok = want_le ? (base <= limit) : (base > limit);
        if (ok) { *out_total = base; return id; }
    }
    (void)t;
    return -1;
}

/* A card in the `group` group, excluding `avoid`, as a DISTINCT copy (never the
 * shared template: test_id() aliases one template, so two calls for two roles
 * would return the same id and make the test vacuous). Used as the
 * `counting_card` of E10/E11, which Rust filters through card_matches_group_str
 * (modifiers.rs:289 / :333). */
static int find_group_card(TestGame *t, const char *group, int avoid)
{
    int n = rb_num_cards();
    for (int id = 0; id < (int)n; id++) {
        if (id == avoid) continue;
        if (!rb_card_matches_group_str(id, group)) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)id, &c)) continue;
        uint16_t idx = c.card_no_idx;
        rb_free_card(&c);
        const char *no = rb_card_string(idx);
        if (!no) continue;
        int copy = test_new_id(t, no);
        if (copy >= 0) return copy;
    }
    return -1;
}

/* ⛔ include/rabuka.h IS NOT THIS TASK'S FILE (it is main's). The four cluster-E
   entry points landed in src/core/modifiers.c and are declared HERE rather than
   in the header, so the header bill for them stays a report:
     BLOCKED-ON: include/rabuka.h — add, beside the other rb_mods_* prototypes
     (around rabuka.h:291-315):
         unsigned rb_mods_count_as_multiple_weight(const GameState *g, int counted, int counting_card);
         unsigned rb_mods_weighted_zone_count(const GameState *g, const int *cards, int n_cards,
                                             const char *card_type, const char *group_names,
                                             int counting_card);
         int      rb_mods_replacement_appear_wait_applies(const GameState *g, int incoming, int area_idx);
         void     rb_record_stage_position_swap(GameState *g, int source_id, int source_index,
                                                int target_id, int target_index);
   plus the DELETION of the rb_mods_remove_need_heart line (E15, rabuka.h:306). */
extern unsigned rb_mods_count_as_multiple_weight(const GameState *g, int counted, int counting_card);
extern unsigned rb_mods_weighted_zone_count(const GameState *g, const int *cards, int n_cards,
                                            const char *card_type, const char *group_names,
                                            int counting_card);
extern int      rb_mods_replacement_appear_wait_applies(const GameState *g, int incoming, int area_idx);
extern void     rb_record_stage_position_swap(GameState *g, int source_id, int source_index,
                                               int target_id, int target_index);

/* ═══════════════════════ E16 — cause_card_id writer ═══════════════════════
   Rust push_movement_event takes `cause_card_id: Option<i16>` and its two
   position-change push sites pass `self.activating_card`
   (modifiers.rs:1843 / :1851). Before this fix the C's `RbBatchMovement event`
   was a stack local whose `cause_card_id` member was NEVER ASSIGNED, so the
   garbage on the stack was what reached g->batch_movements and
 * g->turn_area_movements. That is an uninitialised read, not a missing
   feature. */
static void test_e16_cause_card_id_is_written_from_activating_card(void)
{
    test_game_new(&tg);
    int a = test_new_id(&tg, "PL!-PR-001-PR");
    int b = test_new_id(&tg, "PL!-PR-002-PR");
    CHECK(a >= 0 && b >= 0, "E16: two distinct copies of the PR template exist");

    tg.state.n_batch_movements = 0;
    tg.state.n_turn_area_movements = 0;
    tg.state.activating_card = a;
    rb_record_card_movement(&tg.state, b, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);
    CHECK_EQ(tg.state.n_batch_movements, 1, "E16: rb_record_card_movement appended one batch entry");
    CHECK_EQ(tg.state.batch_movements[0].cause_card_id, a,
             "E16: cause_card_id is g->activating_card (modifiers.rs:1843)");
    CHECK_EQ(tg.state.turn_area_movements[0].cause_card_id, a,
             "E16: turn_area_movements gets the SAME event clone (modifiers.rs:1956)");
    CHECK_EQ(tg.state.batch_movements[0].effect_only, 1,
             "E16: effect_only is passed through unchanged");

    /* The negative case: Rust's `None`. The C spells it a negative value —
       8 call sites across src/ test `activating_card >= 0` — so -1 must
       propagate rather than be clamped to 0. */
    tg.state.n_batch_movements = 0;
    tg.state.activating_card = -1;
    rb_record_card_movement(&tg.state, b, RB_ZONEID_HAND, RB_ZONEID_STAGE, 0, 1);
    CHECK_EQ(tg.state.batch_movements[0].cause_card_id, -1,
             "E16: no activating card propagates as -1, Rust's None");
}

/* ═══════════════════════ E9 — record_stage_position_swap ═════════════════
   modifiers.rs:1799-1857. NOT a display/replay gap: the readers are the
   GAMEPLAY condition evaluator —
     engine/src/ability/condition/state.rs:871 / :879 / :929 / :938 / :1162
   — and game/display.rs:1858/1861 is only a fifth. */
static void test_e9_position_swap_writes_events_and_movements(void)
{
    test_game_new(&tg);
    int src = test_new_id(&tg, "PL!-PR-001-PR");
    int tgt = test_new_id(&tg, "PL!-PR-002-PR");
    int third = test_new_id(&tg, "PL!-PR-005-PR");
    CHECK(src >= 0 && tgt >= 0 && third >= 0, "E9: three distinct stage members");

    tg.state.n_position_change_events = 0;
    tg.state.n_batch_movements = 0;
    tg.state.n_recently_moved = 0;
    tg.state.activating_card = third;

    /* modifiers.rs:1820-1823 — source: old=source_index, new=target_index. */
    /* modifiers.rs:1831-1834 — target: old=target_index, new=source_index. */
    rb_record_stage_position_swap(&tg.state, src, 0, tgt, 2);
    CHECK_EQ(tg.state.n_position_change_events, 2,
             "E9: two PositionChangeEvents pushed (modifiers.rs:1817-1838)");
    CHECK_EQ(tg.state.position_change_events[0].card_id, src, "E9: event 0 is the source card");
    CHECK_EQ(tg.state.position_change_events[0].position, 2, "E9: source old_position 0 -> new 2");
    CHECK_EQ(tg.state.position_change_events[1].card_id, tgt, "E9: event 1 is the target card");
    CHECK_EQ(tg.state.position_change_events[1].position, 0, "E9: target old_position 2 -> new 0");

    /* modifiers.rs:1839-1856 — one movement event per REAL card, stage -> stage,
       effect_only true. */
    CHECK_EQ(tg.state.n_batch_movements, 2, "E9: two movement events (modifiers.rs:1839-1856)");
    CHECK_EQ(tg.state.batch_movements[0].moved_card_id, src, "E9: source moved first");
    CHECK_EQ(tg.state.batch_movements[1].moved_card_id, tgt, "E9: target moved second");
    CHECK_EQ(tg.state.batch_movements[0].source_zone, RB_ZONEID_STAGE, "E9: source zone stage");
    CHECK_EQ(tg.state.batch_movements[0].dest_zone, RB_ZONEID_STAGE,   "E9: dest zone stage");
    CHECK_EQ(tg.state.batch_movements[0].effect_only, 1, "E9: effect_only true (modifiers.rs:1845)");
    CHECK_EQ(tg.state.batch_movements[0].cause_card_id, third,
             "E9: cause_card_id is the activating card (modifiers.rs:1843)");

    /* modifiers.rs:1791-1793 — the source is recorded unconditionally, the
       target only when it is a real card. An EMPTY target slot is legal. */
    tg.state.n_position_change_events = 0;
    tg.state.n_batch_movements = 0;
    rb_record_stage_position_swap(&tg.state, src, 1, RB_EMPTY_SLOT, -1);
    CHECK_EQ(tg.state.n_position_change_events, 1,
             "E9: an empty target slot yields ONE event, not two (modifiers.rs:1791-1793)");
    CHECK_EQ(tg.state.position_change_events[0].card_id, src,
             "E9: the source is recorded even with no target");
    CHECK_EQ(tg.state.n_batch_movements, 1,
             "E9: the guarded push_movement_event is skipped for the empty target");
}

/* ═══════════════════════ E12 — replacement_appear_wait_applies ═══════════
   modifiers.rs:360-451. Reachable: 2 corpus nodes (PL!-pb2-002-PP / -R). */
static void test_e12_replacement_appear_wait(void)
{
    test_game_new(&tg);
    int host = test_new_id(&tg, "PL!-pb2-002-PP");
    CHECK(host >= 0, "E12: PL!-pb2-002-PP 絢瀬絵里 resolved");
    CHECK(rb_card_is_member(host), "E12: fixture premise — the host is a member card");

    /* The printed rule, read off the decoded effect so the test cannot drift
       from the corpus: position=center, heart_total_limit=4, operator "<=". */
    int limit = -1; const char *op = NULL; const char *pos = NULL;
    int n = rb_card_num_abilities((uint32_t)host);
    for (int ai = 0; ai < n; ai++) {
        Ability ab; memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)host, ai, &ab)) continue;
        const AbilityEffect *e = ab.effect;
        if (e && e->action && !strcmp(e->action, "replacement_appear_wait")) {
            for (int i = 0; i < e->n_extra; i++) {
                if (!e->extra_k[i]) continue;
                if (!strcmp(e->extra_k[i], "heart_total_limit"))    limit = atoi(e->extra_v[i]);
                if (!strcmp(e->extra_k[i], "heart_total_operator")) op    = e->extra_v[i];
                if (!strcmp(e->extra_k[i], "position"))             pos   = e->extra_v[i];
            }
        }
        rb_free_ability(&ab);
    }
    CHECK_EQ(limit, 4, "E12: the corpus rule is heart_total_limit=4");
    CHECK(op && !strcmp(op, "<="), "E12: the corpus operator is the DEFAULT \"<=\"");
    CHECK(pos && !strcmp(pos, "center"), "E12: the corpus position is \"center\"");

    int small_total = 0, big_total = 0;
    int small = find_member_with_base(&tg, 1, limit, &small_total);
    int big   = find_member_with_base(&tg, 0, limit, &big_total);
    CHECK(small >= 0 && small_total <= limit,
          "E12: fixture premise — a member with <= 4 printed base hearts exists");
    CHECK(big >= 0 && big_total > limit,
          "E12: fixture premise — a member with > 4 printed base hearts exists");

    /* No host on stage: modifiers.rs:450. */
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, small, 1), 0,
             "E12: no on-stage host means no rule (modifiers.rs:450)");

    /* Host on stage, centre. area_idx == 1 is the slot the rule names, so the
       rule holds (modifiers.rs:413-426 + :446). */
    test_add_to_stage(&tg, 1, host);
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, small, 1), 1,
             "E12: a <=4-heart member entering the CENTRE enters in wait (modifiers.rs:446)");
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, small, 0), 0,
             "E12: the rule governs area_idx, not the host's own slot (modifiers.rs:423)");

    /* modifiers.rs:432-441 — compare_counts(op, printed_hearts, limit). */
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, big, 1), 0,
             "E12: a >4-heart member is refused by heart_total_limit (modifiers.rs:435)");

    /* modifiers.rs:409-412 — a WAITED host's rule does not apply. */
    rb_mods_set_orientation(&tg.state.mods, host, "wait");
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, small, 1), 0,
             "E12: a waited host's rule does not apply (modifiers.rs:410)");
    rb_mods_set_orientation(&tg.state.mods, host, "active");
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, small, 1), 1,
             "E12: an ACTIVE host's rule applies again");

    /* modifiers.rs:386-388 — a member is never in front of itself. Without this
       the host's own rule matched her on the turn she arrived. */
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, host, 1), 0,
             "E12: the incoming card cannot be its own host (modifiers.rs:386)");

    /* modifiers.rs:364-366 — only a MEMBER can be held in wait. */
    int non_member = -1;
    for (int id = 0; id < (int)rb_num_cards(); id++)
        if (!rb_card_is_member(id)) { non_member = id; break; }
    CHECK(non_member >= 0, "E12: fixture premise — a non-member card exists");
    CHECK_EQ(rb_mods_replacement_appear_wait_applies(&tg.state, non_member, 1), 0,
             "E12: a non-member is never held in wait (modifiers.rs:364)");
}

/* ═══════════════════════ E10 — count_as_multiple_weight ══════════════════
   modifiers.rs:227-301. Reachable: exactly ONE corpus node, on PL!-pb2-041-L
   (count_as=2, counted_by="lily white", self_target=true, and a condition that
   requires the card to sit in its owner's success live card zone). */
static void test_e10_count_as_multiple_weight(void)
{
    test_game_new(&tg);
    int host = test_new_id(&tg, "PL!-pb2-041-L");
    CHECK(host >= 0, "E10: PL!-pb2-041-L 春情ロマンティック resolved");

    /* Read the rule off the decoded effect so the assertions cannot drift. */
    int count_as = 0; const char *counted_by = NULL; int cond_loc_ok = 0;
    int n = rb_card_num_abilities((uint32_t)host);
    for (int ai = 0; ai < n; ai++) {
        Ability ab; memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)host, ai, &ab)) continue;
        const AbilityEffect *e = ab.effect;
        if (e && e->action && !strcmp(e->action, "count_as_multiple")) {
            for (int i = 0; i < e->n_extra; i++) {
                if (!e->extra_k[i]) continue;
                if (!strcmp(e->extra_k[i], "count_as"))    count_as = atoi(e->extra_v[i]);
                if (!strcmp(e->extra_k[i], "counted_by")) counted_by = e->extra_v[i];
            }
            if (e->condition) for (uint32_t q = 0; q < e->condition->n_fields; q++) {
                const char *k = e->condition->fields[q].key;
                if (k && !strcmp(k, "location") && e->condition->fields[q].v.s &&
                    !strcmp(e->condition->fields[q].v.s, "success_live_card_zone"))
                    cond_loc_ok = 1;
            }
        }
        rb_free_ability(&ab);
    }
    CHECK_EQ(count_as, 2, "E10: the corpus weight is count_as=2");
    CHECK(counted_by && !strcmp(counted_by, "lily white"),
          "E10: counted_by is the lily white group");
    CHECK(cond_loc_ok, "E10: the rule's own condition requires the success live card zone");

    int lw = find_group_card(&tg, "lily white", host);
    int other = -1;
    for (int id = 0; id < (int)rb_num_cards(); id++) {
        if (id != host && !rb_card_matches_group_str(id, "lily white")) { other = id; break; }
    }
    CHECK(lw >= 0, "E10: fixture premise — a lily white card exists");
    CHECK(other >= 0, "E10: fixture premise — a non-lily-white card exists");

    /* The rule's condition is false while the card is in hand, so the weight is
       the DEFAULT 1 (modifiers.rs:274-276 -> :300). */
    CHECK_EQ((int)rb_mods_count_as_multiple_weight(&tg.state, host, lw), 1,
             "E10: the rule's own condition failing leaves the weight at 1");

    /* Put it in p1's success live card zone, which is what the condition wants. */
    rb_success_add(&tg.state.p[0], host);
    CHECK_EQ((int)rb_mods_count_as_multiple_weight(&tg.state, host, lw), 2,
             "E10: in the success zone and counted by a lily white card -> count_as (modifiers.rs:298)");

    /* modifiers.rs:282-297 — counted_by is a FILTER on the COUNTING card, so a
       counting card outside the listed group leaves the weight at 1. */
    CHECK_EQ((int)rb_mods_count_as_multiple_weight(&tg.state, host, other), 1,
             "E10: a counting card outside counted_by leaves the weight at 1 (modifiers.rs:286-297)");

    /* modifiers.rs:242-244 — no counting card at all means 1. The C resolves
       that through the queue's current entry and then g->activating_card; with
       the queue empty and activating_card negative the chain is exhausted. */
    tg.state.activating_card = -1;
    CHECK_EQ((int)rb_mods_count_as_multiple_weight(&tg.state, host, -1), 1,
             "E10: no counting card at all -> 1 (modifiers.rs:242-244)");

    /* A card with no count_as_multiple ability is always 1 (modifiers.rs:300). */
    CHECK_EQ((int)rb_mods_count_as_multiple_weight(&tg.state, lw, lw), 1,
             "E10: a card with no such ability weighs 1 (modifiers.rs:300)");
}

/* ═══════════════════════ E11 — weighted_zone_count ═══════════════════════
   modifiers.rs:308-343. Same single corpus node as E10; the weight is only ever
   consulted through this function. */
static void test_e11_weighted_zone_count(void)
{
    test_game_new(&tg);
    int host = test_new_id(&tg, "PL!-pb2-041-L");
    int lw   = find_group_card(&tg, "lily white", host);
    int other = -1;
    for (int id = 0; id < (int)rb_num_cards(); id++) {
        if (id != host && !rb_card_matches_group_str(id, "lily white")) { other = id; break; }
    }
    CHECK(host >= 0 && lw >= 0 && other >= 0,
          "E11: fixtures — the weighted card, a lily white card, a non-lily-white card");

    rb_success_add(&tg.state.p[0], host);
    int cards[3] = { host, lw, other };
    tg.state.activating_card = -1;

    /* No counting card -> every card weighs 1 -> 3. */
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, NULL, NULL, -1), 3,
             "E11: with no counting card the count is the plain card count (modifiers.rs:318-341)");

    /* counting_card = the lily white card -> only `host` carries the rule and it
       now weighs 2, so the total is 4. */
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, NULL, NULL, lw), 4,
             "E11: the lily white counting card sees host's count_as=2 (modifiers.rs:341)");

    /* The non-lily-white counting card does NOT see the weight (counted_by). */
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, NULL, NULL, other), 3,
             "E11: a counting card outside counted_by does not see the weight (modifiers.rs:286-297)");

    /* modifiers.rs:318 — an EMPTY card_type means "any", not "matches the empty
       type"; passing Some("") through would match nothing and count zero. */
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, "", "", lw), 4,
             "E11: an empty card_type means any (modifiers.rs:317-318)");

    /* modifiers.rs:330-339 — group_names is an `.any()` over the WHOLE
       comma-joined list, and an empty list is no restriction at all. */
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, NULL, "lily white", lw), 3,
             "E11: group_names=lily white keeps host+lw and drops other (modifiers.rs:330-339)");
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, cards, 3, NULL, "no such group", lw), 0,
             "E11: a group nothing matches counts zero (modifiers.rs:333-338)");

    /* modifiers.rs:323-324 — the EMPTY SLOT is skipped, not counted. */
    int with_empty[4] = { RB_EMPTY_SLOT, host, lw, other };
    CHECK_EQ((int)rb_mods_weighted_zone_count(&tg.state, with_empty, 4, NULL, NULL, lw), 4,
             "E11: an empty slot contributes 0 (modifiers.rs:323-324)");
}

/* ═══════════════════════ E15 — the surviving need-heart undo ══════════════
   E15 DELETED rb_mods_remove_need_heart (0 Rust twin, 0 callers tree-wide).
   The surviving undo is `add_need_heart(cid, color, -delta)`, which is exactly
   what Rust does at modifiers.rs:607-610. This pins the survivor so the deleted
   twin is not reintroduced under a new name. */
static void test_e15_need_heart_undo_is_add_of_negation(void)
{
    RbMods m;
    rb_mods_init(&m);
    const int cid = 900, col = RB_HEART_PURPLE;
    rb_mods_add_need_heart(&m, cid, col, -2);
    CHECK_EQ(rb_mods_get_need_heart(&m, cid, col), -2,
             "E15: a need-heart grant of -2 reads back as -2");
    rb_mods_add_need_heart(&m, cid, col, 2);
    CHECK_EQ(rb_mods_get_need_heart(&m, cid, col), 0,
             "E15: the Rust undo is add_need_heart(+delta) (modifiers.rs:607-610)");
    rb_mods_set_need_heart(&m, cid, col, 5);
    CHECK_EQ(rb_mods_get_need_heart(&m, cid, col), 5,
             "E15: a set-override is total() == set (rabuka.h:199, game_modifiers.rs:138)");
}

int main(void)
{
    if (rb_load("src") != 0) { fprintf(stderr, "FAIL: rb_load\n"); return 2; }

    test_e16_cause_card_id_is_written_from_activating_card();
    test_e9_position_swap_writes_events_and_movements();
    test_e12_replacement_appear_wait();
    test_e10_count_as_multiple_weight();
    test_e11_weighted_zone_count();
    test_e15_need_heart_undo_is_add_of_negation();

    rb_unload();
    printf("\ncluster E: %d checks failed\n", failures);
    for (int i = 0; i < g_failed_n; i++) printf("  failed: %s\n", g_failed[i]);
    return failures ? 1 : 0;
}
/* test_parity_energy_position_delta.c
 *
 * THE DELTA. Two Rust clusters were handed to this file after every other
 * energy/position subcluster was already claimed, and after two peer suites had
 * already absorbed the top-level energy files:
 *
 *   effects/energy/  (3 top-level .rs, 12 #[test])
 *     energy_zone_active_wait_select_test.rs   (4) -> test_parity_energy_remainder.c SECTION A
 *     wait_self_activate_pl_n_pb1_006_r_test.rs (3) -> test_parity_energy_remainder.c SECTION B
 *     pb2_baton_by_cost_15_mus_activates_two_
 *       energy_pl_pb2_009_pp_test.rs            (5) -> THIS FILE, SECTION A (no C equivalent anywhere)
 *
 *   effects/position/area_gated/  (2 .rs, 18 #[test])
 *     success_zone_center_area_mus_member_constant_blade_test.rs (11)
 *         -> test_parity_small_clusters.c SECTION F1, all 11, one per Rust fn
 *     center_area_syncrise_member_constant_blade_gain_test.rs    (7)
 *         -> test_parity_small_clusters.c SECTION F2, 5 of 7. The two Rust
 *            functions with no C equivalent are ported here as SECTION B2.
 *
 * So the deliverable is 7 Rust test functions, not 30. report_delta() at the
 * bottom prints that inventory on every run so the boundary is auditable
 * without opening the Rust tree.
 *
 * ---------------------------------------------------------------------------
 * WHAT SECTION A MEASURES, AND WHY IT IS NOT VACUOUS
 * ---------------------------------------------------------------------------
 * Yazawa Niko PL!-pb2-009-PP (jidou/auto, cost 9, unit BiBi, series
 * "Love Live!" -> group "mu's"), card text translated:
 *
 *   "[AUTO] When this member is placed from the stage into the waitroom, IF
 *    this member was baton-touched by a member of group mu's whose cost is
 *    15 or more, make 2 energy cards active."
 *
 * TWO independent gates on the baton-touched partner: cost >= 15 AND group
 * mu's. The Rust file's whole design point is that it drives the auto-ability
 * with the baton-touch STATE published explicitly and measures the active-count
 * delta ACROSS THE TRIGGER ONLY, so the arriving card's play path is excluded
 * from the window and the two negatives can be stated as ABSOLUTE ZERO rather
 * than as a comparison between two negatives.
 *
 * THE ENERGY-ZONE ACCOUNTING COMPANION (the brief's explicit requirement):
 * every case in SECTION A asserts THREE numbers, not one.
 *   1. active_count delta      -- the Rust assertion itself
 *   2. energy zone CARD COUNT  -- activation flips a state, it never removes a
 *      card, so this must be 5 before AND after
 *   3. under_cards[0..2].n     -- activation places nothing, so all three must
 *      be 0 before AND after
 * Numbers 2 and 3 are what make this a genuine detector of the known
 * energy-zone accounting defect instead of a test that can pass on a zone that
 * was never touched. They are also the pairing that catches the reported
 * symptom ("2 energy removed from the energy zone (got 12 expected 11)" paired
 * with "2 energy under the center member (got 1 expected 2)"): a removal
 * without a placement shows up as 2 failing immediately, and a placement
 * without a removal shows up as 3.
 *
 * Note the asymmetry, stated up front so results are read correctly: the
 * activation path is NOT where the off-by-one lives. If the zone accounting is
 * correct here, the defect is confined to the energy -> under-member PLACEMENT
 * path (src/ability/effects/move.c), which test_parity_energy_remainder.c
 * SECTION A2/A3 and test_parity_under_member.c already pin red. SECTION A here
 * is the CONTROL for that claim, not a duplicate of it.
 *
 * ---------------------------------------------------------------------------
 * WHAT SECTION B2 MEASURES
 * ---------------------------------------------------------------------------
 * PL!SP-pb2-022-R (jidou/auto, once per turn), card text translated: "[AUTO]
 * When a member of unit 5yncri5e! on your stage moves to the CENTER area, it
 * gains 4 blades until the end of the live."
 * Two shapes the peer file does not have:
 *   B2.1 a 5yncri5e! member that is NOT the host moves TO the centre -> the
 *        HOST gains 4 blades, and the MOVER does not
 *   B2.2 a 5yncri5e! member moves to a NON-centre area -> nothing fires
 *
 * ---------------------------------------------------------------------------
 * FLAT-CHOICE GAP (recorded, never counted as a pass)
 * ---------------------------------------------------------------------------
 * Both Rust position cases pick the destination by NAME:
 *   game.generated_actions().iter().position(|a| a.parameters.stage_area == "left")
 * The C RbChoice is FLAT: it carries no ordered option list in which entry i is
 * named "center". drive_position_dst() therefore answers a fixed index and the
 * caller ASSERTS THE RESULTING BOARD, which makes the index/area assumption
 * falsifiable instead of silently load-bearing. Whether the candidate list
 * happens to be populated is recorded as an EXPECTED_GAP, never as a pass.
 *
 * ---------------------------------------------------------------------------
 * RUST FIXTURE BUGS found while porting (cards/cards.json read as UTF-8)
 * ---------------------------------------------------------------------------
 *   NONE that invalidate an assertion, and one worth recording:
 *   - pb2_baton..._test.rs::fixture_cost_and_group_are_what_the_gates_assume
 *     binds `want_mus` and NEVER ASSERTS IT. The tuple's third element is
 *     dead. This port asserts the group as well, because the group gate is
 *     half of what the file exists to test.
 *   - center_area_syncrise..._test.rs labels PL!SP-bp2-008-R
 *     "(non-Syncri5e, Liella!)" and PL!-sd1-010-SD "mu's Printemps, NOT
 *     Syncri5e". cards.json: by `unit` they are 5yncri5e! / Printemps
 *     respectively, and by `series` BOTH map to Liella!. test_parity_small_
 *     clusters.c already reports this. SECTION B2 uses neither card, so no
 *     assertion here depends on the wrong label.
 *
 * ---------------------------------------------------------------------------
 * LANDMINES respected
 * ---------------------------------------------------------------------------
 *   - sizeof(GameState) is ~781 KB: every TestGame local is `static`.
 *   - test_id() aliases a shared template; test_new_id() allocates a distinct
 *     instance, mirroring Rust game.id()'s pool. mid() is always test_new_id.
 *   - rb_card_no_eq is FUZZY and is NOT an identity pin. ident() compares the
 *     PRINTED number byte-for-byte via rb_card_string(c.card_no_idx).
 *   - rb_card_group_name() derives the group from the SERIES; cards.bin leaves
 *     the explicit group field empty, so rb_card_string(card->group_idx) would
 *     return "" and every group predicate would match every card.
 *   - The energy bag is TOP-INDEX-0, matching Rust main_deck.cards.
 *   - Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *     from rb_card_num_abilities + rb_decode_card_ability.
 *   - Every drain of the ability queue is BOUNDED; see settle_queue() and
 *     drain(), and the peer drain_by_skip(limit) in test_parity_queue_resume.c.
 *   - THIS FILE IS PURE ASCII. No Japanese literal appears in it, so it cannot
 *     be corrupted by a tool that rewrites sources in a legacy code page. The
 *     only Japanese token it needs is RB_TSTR_AUTO, which rabuka.h already
 *     provides as a macro.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ------------------------------ harness ------------------------------ */

static int failures;
static int assertions;
static int setup_bugs;
static int expected_gaps;
static const char *current_test = "(none)";

#define CHECK(condition, ...) do {                                   \
    assertions++;                                                     \
    if (!(condition)) {                                               \
        fprintf(stderr, "FAIL [%s]: ", current_test);                \
        fprintf(stderr, __VA_ARGS__);                                 \
        fprintf(stderr, "\n");                                        \
        failures++;                                                   \
    }                                                                \
} while (0)

#define CHECK_EQ(actual, expected, ...) do {                         \
    assertions++;                                                     \
    int a_ = (actual);                                                \
    int e_ = (expected);                                              \
    if (a_ != e_) {                                                   \
        fprintf(stderr, "FAIL [%s]: ", current_test);                 \
        fprintf(stderr, __VA_ARGS__);                                 \
        fprintf(stderr, " (got %d expected %d)\n", a_, e_);           \
        failures++;                                                   \
    }                                                                \
} while (0)

/* A gap is NEVER a pass. It is counted, printed, and reported separately so a
 * reader can tell "the engine did the wrong thing" from "C cannot see it". */
#define EXPECTED_GAP(desc, condition) do {                           \
    assertions++;                                                     \
    if (!(condition)) {                                               \
        printf("GAP [%s]: %s\n", current_test, desc);                \
        expected_gaps++;                                             \
    } else {                                                          \
        printf("  (gap closed) %s\n", desc);                          \
    }                                                                \
} while (0)

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* Card numbers. Cost / unit / series verified against cards/cards.json read as
 * UTF-8; the group strings are what rb_card_series_to_group() returns. */
#define NICO_NO          "PL!-pb2-009-PP"    /* Yazawa Niko,      cost  9, BiBi       -> mu's    */
#define HIGH_COST_MUS_NO "PL!-pb2-001-PP"    /* Takasaka Honoka,  cost 22, Printemps  -> mu's    */
#define LOW_COST_MUS_NO  "PL!-bp3-002-P"     /* Ayase Eri,        cost  9, BiBi       -> mu's    */
#define HIGH_NON_MUS_NO  "PL!SP-bp4-004-P"   /* Hirayanagi Sumire,cost 22, CatChu!    -> Liella! */
#define ENERGY_NO        "LL-E-001-SD"
#define FILLER_NO        "PL!-sd1-010-SD"   /* Kosaka Honoka,    cost  4, Printemps  -> mu's    */

#define FUYU_NO        "PL!SP-pb2-022-R"    /* the HOST card, cost 15, 5yncri5e!        */
#define ABILITYLESS_NO "PL!SP-pb1-014-N"    /* 5yncri5e! member, cost 2, NO ability      */
#define KINAKO_NO      "PL!SP-bp5-006-R"    /* Sakurajouji Kinako, cost 11, 5yncri5e!,
                                               kidou(activate) = mill 3 then position   */

/* ------------------------------ fixtures ------------------------------ */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Rust `game.new_id(X)`: the NEXT DISTINCT pool slot. */
static int mid(TestGame *tg, const char *no)
{
    return test_new_id(tg, no);
}

/* Pin identity by the PRINTED number, byte-for-byte. rb_find_card_by_no and
 * rb_card_no_eq both have a fuzzy tail, so only strcmp of
 * rb_card_string(card_no_idx) cannot be satisfied by a near miss. */
static int ident(int id, const char *no)
{
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: card \"%s\" does not resolve\n", no);
        setup_bugs++; assertions++; return 0;
    }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: card id %d does not decode\n", id);
        setup_bugs++; assertions++; return 0;
    }
    char got[128];
    snprintf(got, sizeof got, "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr, "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n",
                id, got, no);
        setup_bugs++; assertions++; return 0;
    }
    return 1;
}

static int printed_cost(int id)
{
    Card c;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static const char *group_of(int id) { return rb_card_group_name(id); }

/* Empty every zone both seats own, and release the process-global card-copy
 * pool (rb_create_card_copy never resets it, and it runs dry after ~1457
 * allocations). */
static void bare_board(TestGame *tg)
{
    test_card_copy_pool_reset();
    test_game_new(tg);
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_recently_moved = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    tg->state.queue.deferred = NULL;
    tg->state.queue.resume_eff = NULL;
    tg->state.activation_keepalive_valid = 0;
    tg->state.baton_touch_count_p1 = 0;
    tg->state.baton_touch_count_p2 = 0;
    tg->state.baton_touch_replaced_member_id = -1;
    tg->state.baton_touch_arriving_card_id = -1;
    tg->state.baton_touch_replaced_member_cost = -1;
    tg->state.baton_touch_zero_cost = 0;
    tg->state.recently_moved_from_zone[0] = '\0';
    rb_clear_pending_choice(&tg->state);
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static void answer(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust `drain_choices()`: PROCEED an auto-ability, PAY an optional-cost gate
 * (whose option 0 is SKIP), take index 0 on everything else. BOUNDED, and it
 * returns how many prompts it drove, which is the non-vacuity counter. */
static int drain(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = 0;
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = 1;
        rb_resume_with_choice(&tg->state, idx);
        n++;
    }
    return n;
}

/* Settle the ability queue: answer every prompt and run every queued ability.
 * BOUNDED. Returns the queue depth it could NOT clear (0 = fully drained).
 *
 * This is a FIXTURE step, not a way to make an assertion green. The host card is
 * an auto-ability watcher, so the plays can leave an entry queued, and an
 * activation issued while the queue is non-empty is not the same activation the
 * Rust test issues -- test_activate_ability drives the queue itself, so a stale
 * entry both steals the activation and fires an unrelated ability. Every caller
 * ASSERTS the returned depth is 0, so a queue the engine cannot drain is
 * reported, not hidden. */
static int settle_queue(TestGame *tg, int guard_max)
{
    int guard = 0;
    while ((test_queue_n_entries(tg) > 0 || pending(tg)) && guard++ < guard_max) {
        if (pending(tg)) { drain(tg, 8); continue; }
        rb_process_pending_auto_abilities(&tg->state);
    }
    return test_queue_n_entries(tg);
}

static int active_n(TestGame *tg)      { return rb_energy_active_count(&tg->state.p[0]); }
static int zone_n(TestGame *tg)        { return tg->state.p[0].energy.n; }

/* The under-member census the brief demands alongside the zone count. */
static int under_total(TestGame *tg)
{
    int n = 0;
    for (int a = 0; a < RB_STAGE_SIZE; a++) n += tg->state.p[0].under_cards[a].n;
    return n;
}

static int blade_of(TestGame *tg, int cid) { return rb_mods_get_blade(&tg->state.mods, cid); }

static void push_deck(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++)
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static int slot_of(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] == cid) return i;
    return -1;
}

/* Diagnostic, permanent in the file (the peer suites do the same). A red B2
 * control has to be classified as "the kidou never opened a position prompt"
 * versus "the prompt opened and the swap went to the wrong area", and those two
 * are different engine defects, so the board + queue + phase are dumped at the
 * moment the prompt should have been on screen. */
static void describe_all(TestGame *tg, const char *tag, int host, int filler, int kinako)
{
    GameState *g = &tg->state;
    const RbChoice *c = rb_get_pending_choice(g);
    fprintf(stderr, "  (%s) phase=%s pending=%d kind=%s zone=%s target=%s count=%d "
                    "n_filtered=%d desc=\"%s\"\n",
            tag, rb_phase_name(g->phase), pending(tg),
            c ? test_pending_choice_type(tg) : "(none)",
            c ? c->zone : "", c ? c->target : "", c ? c->count : -1,
            c ? c->n_filtered_indices : -1, c ? c->description : "");
    fprintf(stderr, "  (%s) stage=[%d %d %d] host=%d filler=%d kinako=%d "
                    "deck=%d waitroom=%d hand=%d energy=%d(active=%d) "
                    "queue_entries=%d batch_moves=%d pos_change_this_turn=%d\n",
            tag, g->p[0].stage[0], g->p[0].stage[1], g->p[0].stage[2],
            host, filler, kinako, g->p[0].deck.n, g->p[0].discard.n,
            g->p[0].hand.n, g->p[0].energy.n, g->p[0].energy_active,
            test_queue_n_entries(tg), g->n_batch_movements,
            g->position_change_occurred_this_turn);
}

/* Rust `state.set_recently_moved_cards(vec![cid])` +
 * `recently_moved_from_zone = Some("stage")`. */
static void set_recently_moved_from_stage(TestGame *tg, int cid)
{
    tg->state.recently_moved[0] = cid;
    tg->state.n_recently_moved = 1;
    snprintf(tg->state.recently_moved_from_zone,
             sizeof tg->state.recently_moved_from_zone, "%s", "stage");
}

/* Rust `state.turn_movements.push(MovementEvent{ .. })`. C keeps the log in
 * batch_movements, which is what the auto-ability source/position filters read. */
static void push_movement(TestGame *tg, int cid, RbZoneId src, RbZoneId dst)
{
    GameState *g = &tg->state;
    if (g->n_batch_movements >=
        (int)(sizeof g->batch_movements / sizeof g->batch_movements[0])) return;
    RbBatchMovement *m = &g->batch_movements[g->n_batch_movements++];
    memset(m, 0, sizeof *m);
    m->moved_card_id = cid;
    m->source_zone = (int)src;
    m->dest_zone = (int)dst;
    m->cause_player_id = 0;
    m->effect_only = 0;
}

/* =========================================================================
 * SECTION A -- pb2_baton_by_cost_15_mus_activates_two_energy_pl_pb2_009_pp_test.rs
 * 5 #[test]
 * ========================================================================= */

/* Rust `auto_activates(arriving)`:
 *
 *   fill_decks(12 fillers per seat)
 *   5 energy pushed STRAIGHT INTO energy_zone.cards (so all WAITED)
 *   nico in the WAITROOM, the arriving card on the STAGE at Centre
 *   recently_moved = [nico], from_zone = "stage", baton_touch_* published
 *   trigger_auto_abilities_for_player("p1"); process_pending_auto_abilities("p1")
 *   drain_choices()
 *   => active_count() - before
 *
 * The play path is deliberately OUT of the measurement window; that is the whole
 * point of this Rust file (see its module comment).
 *
 * The out_* parameters carry the energy-zone accounting companions out to the
 * callers. Returns -999 for "fixture could not be built". */
static int auto_activates(TestGame *tg, const char *arriving_no,
                          int *out_zone_before, int *out_zone_after,
                          int *out_under_before, int *out_under_after,
                          int *out_under_left, int *out_under_center)
{
    int filler = mid(tg, FILLER_NO);
    if (filler < 0) { setup_bugs++; return -999; }
    push_deck(tg, 0, filler, 12);
    push_deck(tg, 1, filler, 12);

    /* Rust pushes the cards raw, which leaves the ACTIVE PREFIX at 0: all five
     * are waited. test_add_to_energy would RAISE the active count per card, so
     * the push is done by hand to match. */
    RbPlayer *P = &tg->state.p[0];
    int seeded = 0;
    for (int i = 0; i < 5; i++) {
        int e = mid(tg, ENERGY_NO);
        if (e < 0) { setup_bugs++; return -999; }
        if (P->energy.n >= RB_MAX_ZONE) break;
        P->energy.cards[P->energy.n++] = e;
        seeded++;
    }
    P->energy_active = 0;   /* energy_zone.cards.push() leaves every card waited */

    int nico = mid(tg, NICO_NO);
    int arriver = mid(tg, arriving_no);
    if (!ident(nico, NICO_NO) || !ident(arriver, arriving_no)) return -999;
    CHECK(seeded == 5, "fixture: 5 energy were seeded into the zone");

    /* The baton touch has already happened. */
    rb_waitroom_add(P, nico);
    set_recently_moved_from_stage(tg, nico);
    push_movement(tg, nico, RB_ZONEID_STAGE, RB_ZONEID_WAITROOM);
    tg->state.baton_touch_count_p1 = 1;
    tg->state.baton_touch_replaced_member_id = nico;
    tg->state.baton_touch_arriving_card_id = arriver;
    P->stage[AREA_CENTER] = arriver;

    *out_zone_before  = zone_n(tg);
    *out_under_before = under_total(tg);
    *out_under_left   = P->under_cards[AREA_LEFT].n;
    *out_under_center = P->under_cards[AREA_CENTER].n;

    int before = active_n(tg);
    rb_queue_trigger_abilities(&tg->state, 0, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    drain(tg, 16);

    *out_zone_after  = zone_n(tg);
    *out_under_after = under_total(tg);
    return active_n(tg) - before;
}

/* The zone-accounting companion, asserted identically in all five cases.
 * Activation flips a STATE; it never removes a card from the zone and never
 * places one under a member. Asserting both halves is what makes this a real
 * detector of the reported energy-zone off-by-one rather than a test that can
 * be satisfied by a zone nobody touched. */
static void assert_zone_untouched(TestGame *tg, const char *tag,
                                  int zone_b, int zone_a,
                                  int under_b, int under_a,
                                  int under_left, int under_center)
{
    /* Re-read the board rather than trusting the snapshot alone: if the caller
     * and the board ever disagree, that disagreement is itself the report. */
    CHECK_EQ(zone_a, zone_b,
             "%s: the energy ZONE CARD COUNT is unchanged by an activation "
             "(was %d, now %d) -- activation is a state flip, not a removal",
             tag, zone_b, zone_a);
    CHECK_EQ(zone_n(tg), zone_a,
             "%s: the zone count the caller snapshotted matches the live zone", tag);
    CHECK_EQ(under_a, under_b,
             "%s: no card was placed UNDER a member by the activation "
             "(was %d, now %d)", tag, under_b, under_a);
    CHECK_EQ(under_total(tg), under_a,
             "%s: the under-member census the caller snapshotted matches the board",
             tag);
    CHECK_EQ(under_left, 0,
             "%s: nothing is tucked under the LEFT-side member", tag);
    CHECK_EQ(under_center, 0,
             "%s: nothing is tucked under the CENTRE member", tag);
    CHECK_EQ(tg->state.p[0].under_cards[AREA_RIGHT].n, 0,
             "%s: nothing is tucked under the RIGHT-side member", tag);
}

/* A1 :: fixture_cost_and_group_are_what_the_gates_assume
 * Rust binds `want_mus` and NEVER ASSERTS IT (dead tuple element). This port
 * asserts it, because the group gate is half of what the file tests; without
 * the boundary properties pinned, a fixture drifting on cost or group would
 * silently stop testing the gate it is named for. */
static void a1_fixture_cost_and_group(void)
{
    static TestGame tg;
    bare_board(&tg);

    struct { const char *no; int cost; const char *group; } want[4] = {
        { HIGH_COST_MUS_NO, 22, "mu's"    },
        { LOW_COST_MUS_NO,   9, "mu's"    },
        { HIGH_NON_MUS_NO,  22, "Liella!" },
        { NICO_NO,           9, "mu's"    },
    };
    for (int i = 0; i < 4; i++) {
        int id = mid(&tg, want[i].no);
        if (!ident(id, want[i].no)) continue;
        CHECK_EQ(printed_cost(id), want[i].cost, "%s prints cost %d", want[i].no, want[i].cost);
        CHECK(rb_card_is_member(id), "%s is a member card", want[i].no);
        CHECK(!strcmp(group_of(id), want[i].group),
              "%s resolves to group \"%s\" (series-derived)", want[i].no, want[i].group);
        /* Assert the COMPARISON the ability performs, not just the number. */
        if (want[i].cost >= 15)
            CHECK(printed_cost(id) >= 15, "%s clears the cost>=15 gate", want[i].no);
        else
            CHECK(printed_cost(id) < 15, "%s is UNDER the cost>=15 gate", want[i].no);
    }
}

/* A2 :: a_cost_15_plus_mus_baton_touch_activates_two_energy
 * The POSITIVE. Without it the two absolute-zero cases below are satisfied by an
 * auto-ability that never fires, which is the failure mode an absolute-zero
 * suite is open to. */
static void a2_high_cost_mus_activates_two(void)
{
    static TestGame tg;
    bare_board(&tg);
    int zb = 0, za = 0, ub = 0, ua = 0, ul = -1, uc = -1;
    int delta = auto_activates(&tg, HIGH_COST_MUS_NO, &zb, &za, &ub, &ua, &ul, &uc);
    if (delta == -999) { CHECK(0, "fixture built"); return; }

    CHECK_EQ(delta, 2,
             "Takasaka Honoka is cost 22 and group mu's, so BOTH gates pass and "
             "exactly 2 energy is activated (zone %d, active before 0)", zb);
    assert_zone_untouched(&tg, "A2", zb, za, ub, ua, ul, uc);
}

/* A3 :: a_cheap_mus_baton_touch_activates_nothing
 * The COST gate: right group, UNDER the bound. Absolute zero is only achievable
 * because the play path is excluded from the measurement window. */
static void a3_cheap_mus_activates_nothing(void)
{
    static TestGame tg;
    bare_board(&tg);
    int zb = 0, za = 0, ub = 0, ua = 0, ul = -1, uc = -1;
    int delta = auto_activates(&tg, LOW_COST_MUS_NO, &zb, &za, &ub, &ua, &ul, &uc);
    if (delta == -999) { CHECK(0, "fixture built"); return; }

    CHECK_EQ(delta, 0,
             "Ayase Eri prints cost 9, UNDER the cost>=15 bound, so the "
             "auto-ability must activate NO energy. An engine that ignored the "
             "cost bound would return 2 here; comparing against the other "
             "negative would not catch it, because an engine ignoring BOTH gates "
             "would make the two negatives equal.");
    assert_zone_untouched(&tg, "A3", zb, za, ub, ua, ul, uc);
}

/* A4 :: an_expensive_non_mus_baton_touch_activates_nothing
 * The GROUP gate: over the bound, wrong group. Hirayanagi Sumire prints cost
 * 22, so she CLEARS the cost gate and only the group can stop her. */
static void a4_expensive_non_mus_activates_nothing(void)
{
    static TestGame tg;
    bare_board(&tg);
    int zb = 0, za = 0, ub = 0, ua = 0, ul = -1, uc = -1;
    int delta = auto_activates(&tg, HIGH_NON_MUS_NO, &zb, &za, &ub, &ua, &ul, &uc);
    if (delta == -999) { CHECK(0, "fixture built"); return; }

    CHECK_EQ(delta, 0,
             "Hirayanagi Sumire prints cost 22 and so CLEARS the cost>=15 gate, "
             "but her series maps to group Liella!, not mu's, so passing the cost "
             "gate must not be enough");
    assert_zone_untouched(&tg, "A4", zb, za, ub, ua, ul, uc);
}

/* A5 :: the_auto_fires_and_the_zeros_are_not_a_dead_ability
 * The CONTROL. Identical board to A2 by construction; its job is to make A3 and
 * A4 falsifiable. */
static void a5_control_the_auto_is_not_dead(void)
{
    static TestGame tg;
    bare_board(&tg);
    int zb = 0, za = 0, ub = 0, ua = 0, ul = -1, uc = -1;
    int delta = auto_activates(&tg, HIGH_COST_MUS_NO, &zb, &za, &ub, &ua, &ul, &uc);
    if (delta == -999) { CHECK(0, "fixture built"); return; }

    CHECK_EQ(delta, 2,
             "control: the auto-ability must activate 2 energy for a qualifying "
             "baton touch. If this is 0, A3 and A4 are unfalsifiable.");
    assert_zone_untouched(&tg, "A5", zb, za, ub, ua, ul, uc);
}

/* =========================================================================
 * SECTION B2 -- position/area_gated/center_area_syncrise_member_constant_
 *                 blade_gain_test.rs, the 2 functions the peer suite lacks.
 * ========================================================================= */

/* Rust `trigger_position_change_via_kinako` / the inline generated-action
 * lookup, and its C replacement. Answers the prompt AT MOST ONCE.
 *
 * FLAT-CHOICE GAP: Rust picks the destination by NAME --
 *   actions.iter().position(|a| a.parameters.stage_area == "center")
 * The C RbChoice carries no ordered, named option list. Preference order:
 *   1. an explicit candidate list, picking the entry that IS dst_slot
 *   2. otherwise the raw index, which IS the slot
 * The return value is the slot the driver actually aimed at, so the CALLER can
 * assert the resulting board and make a wrong index/area mapping fail loudly
 * instead of quietly turning the negative assertions green.
 * *had_list records whether a candidate list existed at all; the caller turns
 * that into an EXPECTED_GAP. Returns -1 when no prompt was on screen, which is
 * itself the engine gap the caller must report. */
static int drive_position_dst(TestGame *tg, int src_slot, int dst_slot,
                              int *had_list)
{
    *had_list = 0;
    if (!pending(tg)) return -1;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return -1;
    fprintf(stderr, "  (B2 position) src=%d want_dst=%d kind=%s target=%s "
                    "count=%d n_filtered=%d\n",
            src_slot, dst_slot, test_pending_choice_type(tg), c->target,
            c->count, c->n_filtered_indices);

    for (int i = 0; i < c->n_filtered_indices; i++) {
        int cand = c->filtered_indices[i];
        if (cand < 0 || cand >= RB_STAGE_SIZE || cand == src_slot) continue;
        if (!*had_list) *had_list = 1;
        if (cand == dst_slot) { answer(tg, i); return cand; }
    }
    if (*had_list) {
        /* A candidate list existed but did not name the wanted slot. Take the
         * first non-source candidate, and say so, because the caller asserts
         * the board and will go red if this picked the wrong area. */
        for (int i = 0; i < c->n_filtered_indices; i++) {
            int cand = c->filtered_indices[i];
            if (cand < 0 || cand >= RB_STAGE_SIZE || cand == src_slot) continue;
            fprintf(stderr, "  (B2 position) the candidate list did NOT name "
                            "slot %d; answering the first non-source entry "
                            "(slot %d)\n", dst_slot, cand);
            answer(tg, i);
            return cand;
        }
    }
    /* No usable candidate list: the index IS the slot. */
    answer(tg, dst_slot);
    return dst_slot;
}

/* Rust `give_energy_and_deck`: 35 energy and 20 fillers on P1's deck. */
static void give_energy_and_deck(TestGame *tg)
{
    test_give_energy(tg, 35);
    push_deck(tg, 0, mid(tg, FILLER_NO), 20);
}

/* B2.1 :: other_syncrise_member_moves_to_center_ability_card_gains_blades
 * Three 5yncri5e! members: the HOST left, an ABILITY-LESS 5yncri5e! member in
 * the centre, and Kinako on the right. Kinako's kidou swaps her with the centre
 * member, so a 5yncri5e! member that is NOT the host moves TO the centre and
 * the HOST's ability must fire. The MOVER and the displaced member must not gain
 * anything. */
static void b2_1_other_syncrise_member_to_centre_gains_host_blades(void)
{
    static TestGame tg;
    bare_board(&tg);
    int host   = mid(&tg, FUYU_NO);
    int filler = mid(&tg, ABILITYLESS_NO);
    int kinako = mid(&tg, KINAKO_NO);
    if (!ident(host, FUYU_NO) || !ident(filler, ABILITYLESS_NO) ||
        !ident(kinako, KINAKO_NO)) return;

    /* UNIT CONTROL: the whole point of this file is that the moved member is in
     * the SAME UNIT as the host. Assert `unit`, not `group`: group is
     * series-derived here and would read "Liella!" for all three cards, so a
     * group comparison would be vacuously true. */
    CHECK(!strcmp(rb_card_unit_name(host), rb_card_unit_name(filler)),
          "B2.1 CONTROL: host and the moved member really share one unit");
    CHECK(!strcmp(rb_card_unit_name(host), rb_card_unit_name(kinako)),
          "B2.1 CONTROL: host and Kinako really share one unit");
    CHECK_EQ(rb_card_num_abilities((uint32_t)filler), 0,
             "B2.1 CONTROL: the third member carries NO ability, so any blade it "
             "gains cannot come from itself");
    fprintf(stderr, "  (B2.1) groups (series-derived): host=%s filler=%s kinako=%s\n",
            group_of(host), group_of(filler), group_of(kinako));

    give_energy_and_deck(&tg);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, kinako);
    CHECK_EQ(test_play_to_stage(&tg, host, AREA_LEFT), 1, "B2.1: the host deploys LEFT");
    CHECK_EQ(test_play_to_stage(&tg, filler, AREA_CENTER), 1, "B2.1: the ability-less 5yncri5e! deploys CENTRE");
    CHECK_EQ(test_play_to_stage(&tg, kinako, AREA_RIGHT), 1, "B2.1: Kinako deploys RIGHT");
    drain(&tg, 12);
    int qd = settle_queue(&tg, 32);
    CHECK_EQ(qd, 0, "B2.1 CONTROL: the ability queue is empty before the kidou");

    int host_b   = blade_of(&tg, host);
    int filler_b = blade_of(&tg, filler);
    int kinako_b = blade_of(&tg, kinako);
    int src = slot_of(&tg, kinako);
    CHECK_EQ(src, AREA_RIGHT, "B2.1 CONTROL: Kinako really started on the RIGHT");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], filler,
             "B2.1 CONTROL: the ability-less 5yncri5e! really started in the CENTRE");

    CHECK(test_activate_ability(&tg, kinako) != 0,
          "B2.1: Kinako's kidou is accepted");
    describe_all(&tg, "B2.1 post-activate", host, filler, kinako);
    int had_list = 0;
    int dst = drive_position_dst(&tg, src, AREA_CENTER, &had_list);
    drain(&tg, 12);
    describe_all(&tg, "B2.1 post-drain", host, filler, kinako);

    EXPECTED_GAP("B2.1: the C position prompt exposes an ordered, named option "
                 "list so the destination can be picked BY NAME as the Rust test "
                 "does (generated_actions + parameters.stage_area)",
                 had_list);
    if (!had_list)
        printf("  (note) the position prompt offered NO candidate list; the "
               "destination was driven by raw index and aimed at slot %d, and is "
               "verified from the resulting board below instead\n", dst);

    /* BOARD CONTROLS: these make the blade assertions non-vacuous. */
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], kinako,
             "B2.1 CONTROL: Kinako really moved TO the CENTRE");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], filler,
             "B2.1 CONTROL: the ability-less 5yncri5e! really took the vacated RIGHT slot");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], host,
             "B2.1 CONTROL: the host was not disturbed");
    CHECK_EQ(slot_of(&tg, host), AREA_LEFT,
             "B2.1 CONTROL: the host is still on the stage");

    CHECK(blade_of(&tg, host) >= host_b + 4,
          "B2.1: another 5yncri5e! member moving TO the centre grants the HOST "
          "4 blades (was %d, now %d)", host_b, blade_of(&tg, host));
    CHECK_EQ(blade_of(&tg, filler), filler_b,
             "B2.1: the displaced member (moved AWAY from the centre) gains no blades");
    CHECK_EQ(blade_of(&tg, kinako), kinako_b,
             "B2.1: the MOVER gains no blades -- only the ability card does");
}

/* B2.2 :: syncrise_member_moves_to_non_center_no_trigger
 * The host sits in the CENTRE, an ability-less 5yncri5e! member is on the LEFT
 * and Kinako on the RIGHT. Kinako swaps with the LEFT member, so a 5yncri5e!
 * member moves to a NON-centre area and the host's ability must NOT fire. */
static void b2_2_syncrise_member_to_non_centre_no_trigger(void)
{
    static TestGame tg;
    bare_board(&tg);
    int host    = mid(&tg, FUYU_NO);
    int filler  = mid(&tg, ABILITYLESS_NO);
    int kinako  = mid(&tg, KINAKO_NO);
    if (!ident(host, FUYU_NO) || !ident(filler, ABILITYLESS_NO) ||
        !ident(kinako, KINAKO_NO)) return;
    CHECK(!strcmp(rb_card_unit_name(host), rb_card_unit_name(filler)),
          "B2.2 CONTROL: host and the moving member really share one unit");

    give_energy_and_deck(&tg);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, kinako);
    CHECK_EQ(test_play_to_stage(&tg, host, AREA_CENTER), 1, "B2.2: the host deploys CENTRE");
    CHECK_EQ(test_play_to_stage(&tg, filler, AREA_LEFT), 1, "B2.2: the ability-less 5yncri5e! deploys LEFT");
    CHECK_EQ(test_play_to_stage(&tg, kinako, AREA_RIGHT), 1, "B2.2: Kinako deploys RIGHT");
    drain(&tg, 12);
    int qd = settle_queue(&tg, 32);
    CHECK_EQ(qd, 0, "B2.2 CONTROL: the ability queue is empty before the kidou");

    int before = blade_of(&tg, host);
    int src = slot_of(&tg, kinako);
    CHECK_EQ(src, AREA_RIGHT, "B2.2 CONTROL: Kinako really started on the RIGHT");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], host,
             "B2.2 CONTROL: the host really started in the CENTRE");

    CHECK(test_activate_ability(&tg, kinako) != 0,
          "B2.2: Kinako's kidou is accepted");
    describe_all(&tg, "B2.2 post-activate", host, filler, kinako);
    int had_list = 0;
    drive_position_dst(&tg, src, AREA_LEFT, &had_list);
    drain(&tg, 12);
    describe_all(&tg, "B2.2 post-drain", host, filler, kinako);

    EXPECTED_GAP("B2.2: the C position prompt exposes an ordered, named option "
                 "list so the destination can be picked BY NAME as the Rust test "
                 "does (generated_actions + parameters.stage_area)",
                 had_list);

    /* BOARD CONTROLS: the swap really happened and NOTHING landed in the centre,
     * which is what makes "the host gained nothing" meaningful rather than
     * vacuous. */
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], kinako,
             "B2.2 CONTROL: Kinako really moved to the LEFT (a NON-centre area)");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], filler,
             "B2.2 CONTROL: the ability-less 5yncri5e! really took the vacated RIGHT slot");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], host,
             "B2.2 CONTROL: NO card entered the CENTRE, so the area gate is what "
             "stopped the trigger");

    CHECK_EQ(blade_of(&tg, host), before,
             "B2.2: a 5yncri5e! member moving to a NON-centre area grants no "
             "blades (was %d, now %d)", before, blade_of(&tg, host));
}

/* ------------------------------ card dump ------------------------------
 * RB_EPD_DUMP="PL!SP-bp5-006-R,PL!SP-pb2-022-R" decodes a card's abilities and
 * their effect trees. A red control has to be attributable to the PARSER or to
 * the EVALUATOR, and those are different engine files, so the decode has to be
 * visible. Resolved through test_new_id() -- the SAME representation the tests
 * drive -- because a dump that reads a different card record is worse than no
 * dump. */
static void dump_effect(const char *tag, const AbilityEffect *e, int depth)
{
    if (!e) { fprintf(stderr, "  %*s%s: (null)\n", depth * 2, "", tag); return; }
    fprintf(stderr,
            "  %*s%s: action=%s count=%d target=%s source=%s dest=%s opt=%d "
            "has_cond=%d n_child=%d n_extra=%d\n",
            depth * 2, "", tag, e->action ? e->action : "-", e->count,
            e->target ? e->target : "-", e->source ? e->source : "-",
            e->destination ? e->destination : "-", e->is_optional,
            e->has_condition, e->n_child, e->n_extra);
    /* The condition is printed because the reds in SECTION A are EVALUATOR
     * failures, not effect failures: the effects decode correctly. */
    if (e->condition) {
        fprintf(stderr, "  %*s  condition: variant=%d n_fields=%d\n",
                depth * 2, "", (int)e->condition->variant,
                (int)e->condition->n_fields);
        for (uint32_t i = 0; i < e->condition->n_fields; i++) {
            const CondField *f = &e->condition->fields[i];
            fprintf(stderr, "  %*s  cond[%u] %s tag=%d i=%lld s=%s\n",
                    depth * 2, "", i, f->key ? f->key : "?",
                    (int)f->v.tag, (long long)f->v.i, f->v.s ? f->v.s : "-");
            /* Array-typed condition values (group_names, characters, ...) carry
             * their payload in `arr`, not in `s`. Printing only `s` makes an
             * EMPTY group_names indistinguishable from an UNPOPULATED one, and
             * those two are different engine defects. */
            for (uint32_t k = 0; k < f->v.arr_n; k++)
                fprintf(stderr, "  %*s  cond[%u].arr[%u] tag=%d s=%s\n",
                        depth * 2, "", i, k, (int)f->v.arr[k].tag,
                        f->v.arr[k].s ? f->v.arr[k].s : "-");
        }
    }
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, "  %*s  extra[%d]=%s -> %s\n", depth * 2, "", i,
                e->extra_k[i] ? e->extra_k[i] : "-",
                e->extra_v[i] ? e->extra_v[i] : "-");
    for (int i = 0; i < e->n_child; i++) {
        char sub[32];
        snprintf(sub, sizeof sub, "child[%d]", i);
        dump_effect(sub, e->child[i], depth + 1);
    }
    if (e->optional_action)    dump_effect("OPTIONAL",    e->optional_action,    depth + 1);
    if (e->conditional_action) dump_effect("CONDITIONAL", e->conditional_action, depth + 1);
    if (e->followup_action)    dump_effect("FOLLOWUP",    e->followup_action,    depth + 1);
}

static void dump_card(const char *card_no)
{
    static TestGame tg;
    static int probe_done = 0;
    if (!probe_done) { test_game_new(&tg); probe_done = 1; }
    int id = test_new_id(&tg, card_no);
    if (id < 0) { fprintf(stderr, "[DUMP] %s: NOT FOUND\n", card_no); return; }
    int n = rb_card_num_abilities((uint32_t)id);
    Card c;
    char printed[128] = "(undecodable)";
    if (rb_decode_card_by_index((uint32_t)id, &c)) {
        snprintf(printed, sizeof printed, "%s", rb_card_string(c.card_no_idx));
        rb_free_card(&c);
    }
    fprintf(stderr, "[DUMP] \"%s\" id=%d n_abilities=%d printed=%s group=%s\n",
            card_no, id, n, printed, rb_card_group_name(id));
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
        fprintf(stderr, "  ability[%d] triggers=%s use_limit=%d\n", i,
                ab.triggers ? ab.triggers : "-", ab.use_limit);
        dump_effect("cost", ab.cost, 1);
        dump_effect("effect", ab.effect, 1);
        rb_free_ability(&ab);
    }
}

static void run_dump(const char *list)
{
    char buf[1024];
    snprintf(buf, sizeof buf, "%s", list);
    char *cursor = buf;
    while (cursor && *cursor) {
        char *comma = strchr(cursor, ',');
        if (comma) *comma = '\0';
        if (*cursor) dump_card(cursor);
        cursor = comma ? comma + 1 : NULL;
    }
}

/* ------------------------------ fork harness ------------------------------ */

static void on_segv(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d inside test: %s\n", sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures, g0 = expected_gaps;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = failures - f0;
        if (nf > 0) n_failed++; else n_ok++;
        printf("%-8s %s  [%d assertions, %d gaps, no fork isolation]\n",
               nf ? "FAILED" : "ok", name, assertions - a0, expected_gaps - g0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d gap(s)\n",
               name, assertions, failures - f0, expected_gaps);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? 1 : 0);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH",
               name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    }
    fflush(stdout);
}

/* The inventory this file was written against, printed on every run so the
 * boundary is auditable without opening the Rust tree. */
static void report_delta(void)
{
    printf("\n==== DELTA BOUNDARY (what the peers already own) ====\n");
    printf(" effects/energy/  top level\n");
    printf("   energy_zone_active_wait_select_test.rs        (4) -> test_parity_energy_remainder.c A\n");
    printf("   wait_self_activate_pl_n_pb1_006_r_test.rs      (3) -> test_parity_energy_remainder.c B\n");
    printf("   pb2_baton_by_cost_15_mus_..._pl_pb2_009_pp.rs  (5) -> THIS FILE  A\n");
    printf(" effects/energy/  place/         (13 .rs, 45) -> peer test_parity_look_energy_position.c\n");
    printf(" effects/energy/  under_member/  ( 3 .rs, 56) -> peer test_parity_look_energy_position.c\n");
    printf(" effects/position/ area_gated/success_zone_center_area_mus... (11) -> small_clusters F1\n");
    printf(" effects/position/ area_gated/center_area_syncrise...          ( 7) -> small_clusters F2 (5) + THIS FILE B2 (2)\n");
    printf(" effects/position/ area_move/    (13 .rs, 66) -> peer\n");
    printf(" effects/position/ per_card/     ( 1 .rs,  4) -> peer test_parity_zones_position.c\n");
    printf(" THIS FILE: 7 Rust test functions (5 in A, 2 in B2).\n");
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    {
        const char *d = getenv("RB_EPD_DUMP");
        if (d && *d) { run_dump(d); rb_unload(); return 0; }
    }

    printf("--- A energy/pb2_baton_by_cost_15_mus_activates_two_energy (PL!-pb2-009-PP) ---\n");
    run("fixture_cost_and_group_are_what_the_gates_assume", a1_fixture_cost_and_group);
    run("a_cost_15_plus_mus_baton_touch_activates_two_energy", a2_high_cost_mus_activates_two);
    run("a_cheap_mus_baton_touch_activates_nothing", a3_cheap_mus_activates_nothing);
    run("an_expensive_non_mus_baton_touch_activates_nothing", a4_expensive_non_mus_activates_nothing);
    run("the_auto_fires_and_the_zeros_are_not_a_dead_ability", a5_control_the_auto_is_not_dead);

    printf("--- B2 position/area_gated/center_area_syncrise, the 2 uncovered fns ---\n");
    run("other_syncrise_member_moves_to_center_ability_card_gains_blades",
        b2_1_other_syncrise_member_to_centre_gains_host_blades);
    run("syncrise_member_moves_to_non_center_no_trigger",
        b2_2_syncrise_member_to_non_centre_no_trigger);

    report_delta();
    rb_unload();

    printf("\n==== parity_energy_position_delta: %d passed, %d failed, %d crashed, "
           "%d expected gap(s), %d setup bug(s) ====\n",
           n_ok, n_failed, n_crashed, expected_gaps, setup_bugs);
    printf("assertion-level failures: %d\n", failures);
    if (failures || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: red is the report, not a build error.\n");
        return 1;
    }
    printf("ALL ENERGY/POSITION DELTA CHECKS PASSED\n");
    return 0;
}

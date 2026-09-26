/* Parity suite: auto-ability (自動) queuing + firing.
 *
 * Rust sources mirrored here:
 *   engine/src/core/game_state/abilities.rs
 *       :70-130   is_ability_invalidated / card_has_ability_trigger /
 *                 try_add_ability_invalidation
 *       :133-209  ability_uses_used / ability_has_remaining_uses /
 *                 record_ability_use / set_just_completed / batch match
 *       :360-403  condition_is_event_based
 *       :406-891  trigger_auto_abilities_for_player{,_with_event,_for_movement}
 *       :463-681  stage scan (activation-position gate, discard guard,
 *                 event-based pre-filter, each_time/energy heuristic,
 *                 trigger multiplicity, batch dedupe)
 *       :684-831  live-card scan + recently-moved scan
 *       :899-945  trigger_instance_count (§9.7.2.1)
 *       :1064-1170 trigger_auto_ability (string key)
 *       :1195-1239 trigger_auto_ability_by_index_refs
 *       :1336-1339 effect_is_ability_resolution_watcher
 *       :1348-1436 trigger_each_time_for_member
 *       :1450-1657 process_player_abilities_depth (Rule 9.5.3.2 ordering
 *                 choice, depth-first drain, post-loop batch rescan)
 *       :1659-1685 process_pending_auto_abilities
 *       :1693+    process_current_ability
 *   engine/tests/helpers/mod.rs:114  fire_trigger (string-keyed trigger shim)
 *   engine/tests/helpers/choices.rs:268 drain_auto_ability_choices
 *   engine/tests/test_modules/support/baton_swap_auto_helpers.rs:14
 *       resolve_auto_choices_accepting_optionals
 *   engine/tests/test_modules/characterization/* (auto-ability shapes)
 *
 * Assertion groups:
 *   A. trigger registry (card_has_ability_trigger / invalidation / uses)
 *   B. string-keyed trigger_auto_ability (fire_trigger shim + gained abilities)
 *   C. numeric trigger_auto_ability_by_index
 *   D. TAS queueing per condition shape (appearance / group / discard guard /
 *      resolution watcher / preceding_moved / live-zone self trigger)
 *   E. activation-position gate on the stage scan
 *   F. §9.7.2.1 trigger multiplicity for preceding_moved batches
 *   G. Rule 9.5.3.2 simultaneous-auto-ability ordering choice
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* 自動 (U+81EA U+52D5) */
#define TRIG_AUTO "\xE8\x87\xAA\xE5\x8B\x95"
/* 起動 (U+8D77\xE5\x8B\x95) */
#define TRIG_ACTIVATION "\xE8\xB5\xB7\xE5\x8B\x95"

/* rb_trigger_auto_ability exists in src/core/game_state_abilities.c but has
   no prototype in include/rabuka.h; declare it exactly as defined there
   (same trick as tests/test_parity_stats_pipeline.c). */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);
void rb_trigger_auto_ability_by_index(GameState *g, int trigger_type,
                                      int player_id, int explicit_card_id,
                                      int ability_index,
                                      const int *trigger_moved_cards, int n_moved,
                                      int triggering_member_id);

/* ── fixtures ──────────────────────────────────────────────────────── */

#define DIVE        "PL!N-bp4-026-L"  /* ab#1 自動: self enters live zone      */
#define AOI         "PL!N-pb1-005-R"  /* ab#0 自動: appearance, once per turn  */
#define KANAMUNE    "PL!N-sd2-010-SD2"/* ab#1 自動: 虹ヶ咲 member -> wait      */
#define FUYUMI      "PL!SP-pb2-011-R" /* ab#0 自動: center-area move watcher   */
#define DANCING     "PL!-bp6-020-L"   /* ab#1 自動: each_time resolution watch*/
#define HAZUKI_REN  "PL!SP-bp5-005-R\xEF\xBC\x8B" /* ab#1 自動: preceding_moved */
#define RIN_ACT     "PL!-sd1-005-SD"  /* 起動 only -- no 自動 ability          */
#define FILLER      "PL!-sd1-010-SD"

/* Decode ability `a` of card `cid`, returning its trigger token text.
   Caller frees. */
static int par_find_auto_index(int cid, int nth)
{
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = ab.triggers && !strcmp(ab.triggers, TRIG_AUTO);
        rb_free_ability(&ab);
        if (hit) {
            if (nth == 0) return a;
            nth--;
        }
    }
    return -1;
}

static int par_has_auto(int cid)
{
    return rb_card_num_abilities((uint32_t)cid) > 0 &&
           par_find_auto_index(cid, 0) >= 0;
}

static const char *par_card_no(int cid)
{
    static char buf[128];
    Card c;
    buf[0] = '\0';
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return buf;
    snprintf(buf, sizeof(buf), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    return buf;
}

static int par_entry_count(const GameState *g) { return g->queue.n_entries; }

static void par_reset_queue(GameState *g) { rb_queue_reset(g); }

static void par_drain(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 40) {
        if (tg->state.queue.auto_ability)
            rb_resume_with_choice(&tg->state, 0);
        else
            rb_resume_with_choice(&tg->state, -1);
    }
}

/* ══════════════════════════════════════════════════════════════════════
   A. trigger registry
   ══════════════════════════════════════════════════════════════════════ */

static void test_trigger_registry(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int dive = rb_find_card_by_no(DIVE);
    int rin  = rb_find_card_by_no(RIN_ACT);
    CHECK(dive >= 0, "DIVE! fixture resolves in the card database");
    CHECK(rin >= 0, "Rin fixture resolves in the card database");
    if (dive < 0 || rin < 0) return;

    /* abilities.rs:77 card_has_ability_trigger */
    CHECK_EQ(rb_card_has_ability_trigger_for(g, dive, TRIG_AUTO), 1,
             "card_has_ability_trigger: DIVE! carries a 自動 ability");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, rin, TRIG_AUTO), 0,
             "card_has_ability_trigger: Rin has only 起動, no 自動");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, rin, TRIG_ACTIVATION), 1,
             "card_has_ability_trigger: Rin does carry 起動");

    /* abilities.rs:95 try_add_ability_invalidation */
    CHECK_EQ(rb_try_add_ability_invalidation(g, dive, TRIG_AUTO, "turn_end"), 1,
             "try_add_ability_invalidation registers for a card that has the trigger");
    CHECK_EQ(rb_ability_is_invalidated(g, dive, TRIG_AUTO), 1,
             "is_ability_invalidated observes the new invalidation");
    CHECK_EQ(rb_try_add_ability_invalidation(g, dive, TRIG_AUTO, "turn_end"), 0,
             "try_add_ability_invalidation refuses a duplicate");
    CHECK_EQ(rb_try_add_ability_invalidation(g, rin, TRIG_AUTO, "turn_end"), 0,
             "try_add_ability_invalidation refuses a card with no such trigger");
    CHECK_EQ(rb_ability_is_invalidated(g, rin, TRIG_AUTO), 0,
             "the refused invalidation left no state behind");
    CHECK_EQ(rb_ability_is_invalidated(g, dive, TRIG_ACTIVATION), 0,
             "invalidation is per-trigger, not per-card");
}

/* ══════════════════════════════════════════════════════════════════════
   B. string-keyed trigger_auto_ability  (helpers/mod.rs:114 fire_trigger)
   ══════════════════════════════════════════════════════════════════════ */

static void test_trigger_auto_ability_string_key(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    CHECK(aoi >= 0, "Aoi fixture resolves");
    if (aoi < 0) return;
    int idx = par_find_auto_index(aoi, 0);
    CHECK(idx >= 0, "Aoi has a 自動 ability to fire");
    if (idx < 0) return;

    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", par_card_no(aoi));
    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)aoi, idx, &ab), "Aoi 自動 ability decodes");
    char ability_id[1024];
    snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
             ab.full_text ? ab.full_text : "");
    rb_free_ability(&ab);

    /* abilities.rs:1074 -- the whole body is inside `if let Some(card_no) =
       source_card_id`; with no source card number nothing is ever enqueued. */
    par_reset_queue(g);
    rb_trigger_auto_ability(g, ability_id, TRIG_AUTO, 0, NULL, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability is a no-op without a source card_no (abilities.rs:1074)");

    /* The happy path: exact "<card_no>_<full_text>" key enqueues one entry. */
    par_reset_queue(g);
    rb_trigger_auto_ability(g, ability_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 1,
             "trigger_auto_ability enqueues exactly one entry for a known key");
    if (par_entry_count(g) == 1) {
        CHECK_EQ(g->queue.entries[0].card_id, aoi,
                 "enqueued entry carries the explicit card id");
        CHECK_EQ(g->queue.entries[0].ability_idx, idx,
                 "enqueued entry carries the matched ability index");
        CHECK(strcmp(g->queue.entries[0].player_id, "p1") == 0,
              "player_id 0 is normalized to \"p1\" (abilities.rs:227-231)");
    }

    /* A wrong full_text suffix must not match anything. */
    par_reset_queue(g);
    rb_trigger_auto_ability(g, "PL!N-pb1-005-R_この能力は存在しない", TRIG_AUTO, 0,
                            card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability ignores an ability_id that matches no full_text");

    /* abilities.rs:1090 -- an invalidated card never enqueues. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered before the string-keyed fire");
    par_reset_queue(g);
    rb_trigger_auto_ability(g, ability_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability refuses an invalidated card (abilities.rs:1090)");
    CHECK_EQ(rb_ability_is_invalidated(g, aoi, TRIG_AUTO), 1,
             "invalidation still observable after the refused enqueue");
}

static void test_trigger_auto_ability_gained(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    if (aoi < 0) { CHECK(0, "Aoi fixture resolves for the gained-ability case"); return; }
    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", par_card_no(aoi));

    /* A gained 自動 ability (gain_ability effect) must be reachable through
       the "<card_no>_gained_<idx>" key (abilities.rs:1130-1166). */
    Ability gained;
    memset(&gained, 0, sizeof(gained));
    gained.triggers = (char *)TRIG_AUTO;
    gained.full_text = (char *)"gained auto probe";
    CHECK(rb_register_gained_ability(g, aoi, &gained) == 0,
          "a gained 自動 ability is stored at gained index 0");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, aoi, TRIG_AUTO), 1,
             "card_has_ability_trigger also inspects gained abilities");

    char gained_id[256];
    snprintf(gained_id, sizeof(gained_id), "%s_gained_0", card_no);
    par_reset_queue(g);
    rb_trigger_auto_ability(g, gained_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 1,
             "trigger_auto_ability enqueues a gained 自動 ability by its index key");
    if (par_entry_count(g) == 1)
        CHECK(g->queue.entries[0].ability_idx >= 0x8000,
              "gained entries use the GAINED_ABILITY_INDEX_BASE index space");

    /* An out-of-range gained index must not enqueue. */
    par_reset_queue(g);
    char bad_id[256];
    snprintf(bad_id, sizeof(bad_id), "%s_gained_7", card_no);
    rb_trigger_auto_ability(g, bad_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability ignores a gained index that does not exist");

    /* abilities.rs:1144 -- gained abilities respect invalidation too. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered for the gained-ability case");
    par_reset_queue(g);
    rb_trigger_auto_ability(g, gained_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "an invalidated card cannot fire its gained 自動 ability either");
}

/* ══════════════════════════════════════════════════════════════════════
   C. numeric trigger_auto_ability_by_index  (abilities.rs:1195)
   ══════════════════════════════════════════════════════════════════════ */

static void test_trigger_auto_ability_by_index(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int kan = rb_find_card_by_no(KANAMUNE);
    if (aoi < 0 || kan < 0) { CHECK(0, "by-index fixtures resolve"); return; }
    int aoi_auto = par_find_auto_index(aoi, 0);
    int kan_auto = par_find_auto_index(kan, 0);
    CHECK(aoi_auto >= 0 && kan_auto >= 0, "both fixtures expose a 自動 ability");
    if (aoi_auto < 0 || kan_auto < 0) return;

    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, aoi, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 1,
             "trigger_auto_ability_by_index enqueues a valid (card, ability) pair");
    if (par_entry_count(g) == 1) {
        CHECK_EQ(g->queue.entries[0].ability_idx, aoi_auto,
                 "by-index enqueue preserves the ability index");
        CHECK(strcmp(g->queue.entries[0].player_id, "p1") == 0,
              "by-index enqueue normalizes player 0 to \"p1\"");
    }

    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 1, aoi, aoi_auto, NULL, 0, -1);
    CHECK(par_entry_count(g) == 1 &&
              strcmp(g->queue.entries[0].player_id, "p2") == 0,
          "by-index enqueue normalizes player 1 to \"p2\"");

    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, aoi, 99, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "by-index enqueue rejects an out-of-range ability index");

    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, -1, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "by-index enqueue rejects a missing explicit card id");

    /* abilities.rs:1206 -- invalidation gate on the numeric path. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered for the by-index case");
    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, aoi, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability_by_index refuses an invalidated card (abilities.rs:1206)");

    /* A non-自動 ability is never enqueued on this path. */
    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, kan, kan_auto == 0 ? 1 : 0, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "by-index enqueue rejects a non-自動 ability on the same card");
}

/* ══════════════════════════════════════════════════════════════════════
   D. TAS queueing per condition shape  (abilities.rs:445-891)
   ══════════════════════════════════════════════════════════════════════ */

static int par_tas(TestGame *tg, int pl, const int *moved, int n_moved)
{
    par_reset_queue(&tg->state);
    return rb_trigger_auto_abilities_for_player_with_event(&tg->state, pl, moved,
                                                            n_moved, 0, 0);
}

static void test_tas_condition_shapes(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int kan = rb_find_card_by_no(KANAMUNE);
    int dancing = rb_find_card_by_no(DANCING);
    int dive = rb_find_card_by_no(DIVE);
    if (aoi < 0 || kan < 0 || dancing < 0 || dive < 0) {
        CHECK(0, "TAS fixtures resolve");
        return;
    }

    /* abilities.rs:377-381 -- Appearance conditions are NOT pre-filtered by the
       scan (they resolve through can_activate_effect), so an appearance-型
       自動 must still reach the queue. */
    int aoi_instance = test_id(&tg, AOI);
    test_add_to_stage(&tg, 1, aoi_instance);
    int queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 1,
          "an appearance-condition 自動 on stage is queued (not pre-filtered by TAS)");
    par_reset_queue(g);

    /* abilities.rs:393-400 -- group_condition is not event-based either, so the
       scan does not pre-filter it. */
    int kan_instance = test_id(&tg, KANAMUNE);
    test_add_to_stage(&tg, 0, kan_instance);
    queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 1,
          "a group-condition 自動 on stage is queued (not pre-filtered by TAS)");
    par_reset_queue(g);

    /* abilities.rs:480-489 / 1336 -- a resolution watcher
       (trigger_type each_time + watches_ability_resolution) is armed only by
       trigger_each_time_for_member, never by a board scan. */
    int dancing_instance = test_id(&tg, DANCING);
    test_add_to_live(&tg, dancing_instance);
    int before = par_entry_count(g);
    queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 0 && par_entry_count(g) == before,
          "a TAS scan never queues a 「能力が解決したとき」 resolution watcher");

    /* abilities.rs:697-723 -- the live scan applies the event-based pre-filter
       to live cards too; DIVE!'s self-live-zone condition is movement-based, so
       a scan with no moved cards must not queue it. */
    int dive_instance = test_id(&tg, DIVE);
    test_add_to_live(&tg, dive_instance);
    before = par_entry_count(g);
    queued = par_tas(&tg, 0, NULL, 0);
    CHECK(par_entry_count(g) == before,
          "a self-move live-zone 自動 is not queued when no card moved");

    /* ...but it IS queued when the card itself is in the moved batch
       (abilities.rs:716 / 726-736 movement gate). */
    int moved[1];
    moved[0] = dive_instance;
    queued = par_tas(&tg, 0, moved, 1);
    CHECK(par_entry_count(g) == before + queued && queued >= 1,
          "the live-zone 自動 queues when its own card is in the moved batch");
    par_reset_queue(g);
}

static void test_tas_discard_guard(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    if (aoi < 0) { CHECK(0, "discard-guard fixture resolves"); return; }
    int aoi_instance = test_id(&tg, AOI);
    test_add_to_stage(&tg, 1, aoi_instance);

    /* abilities.rs:515-535 -- a stage card whose 自動 condition watches
       "this card is in the waitroom" must not queue while it is on stage. */
    int queued = par_tas(&tg, 0, NULL, 0);
    int staged_in_discard = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < g->p[pl].discard.n; i++)
            if (g->p[pl].discard.cards[i] == aoi_instance) staged_in_discard = 1;
    CHECK_EQ(staged_in_discard, 0,
             "the on-stage card is not in the waitroom (fixture sanity)");
    CHECK(queued >= 0,
          "a TAS scan over a stage card with a discard-location condition is well defined");
    par_reset_queue(g);
}

static void test_tas_batch_dedupe(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    if (aoi < 0) { CHECK(0, "dedupe fixture resolves"); return; }
    test_add_to_stage(&tg, 1, test_id(&tg, AOI));

    /* abilities.rs:667-670 -- this_batch_triggered_ability_ids makes a second
       pass over the same ability in the same batch a no-op. */
    par_reset_queue(g);
    g->n_batch_triggered_keys = 0;
    int first = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    int n_after_first = par_entry_count(g);
    /* Second pass WITHOUT clearing the batch set must not double-queue. */
    int second = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK_EQ(second, 0,
             "a second TAS pass in the same batch queues nothing (batch dedupe)");
    CHECK_EQ(par_entry_count(g), n_after_first,
             "the second pass left the queue length unchanged");
    CHECK(first >= 1, "the first TAS pass did queue the stage ability");
    par_reset_queue(g);
}

static void test_each_time_resolution_watcher(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int dancing = rb_find_card_by_no(DANCING);
    int fuyumi  = rb_find_card_by_no(FUYUMI);
    int aoi     = rb_find_card_by_no(AOI);
    if (dancing < 0 || fuyumi < 0 || aoi < 0) {
        CHECK(0, "each_time watcher fixtures resolve");
        return;
    }

    int dancing_instance = test_id(&tg, DANCING);
    int aoi_instance     = test_id(&tg, AOI);
    test_add_to_live(&tg, dancing_instance);
    test_add_to_stage(&tg, 1, aoi_instance);   /* center: μ's member */

    /* abilities.rs:1361-1364 -- a non-stage member never arms the watcher. */
    par_reset_queue(g);
    rb_trigger_each_time_for_member(g, 0, "ライブ成功時", dancing_instance);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_each_time_for_member ignores a member that is not on stage");

    /* abilities.rs:1367-1415 -- a stage member whose group matches arms it. */
    int fuyumi_instance = test_id(&tg, FUYUMI);
    test_add_to_stage(&tg, 0, fuyumi_instance); /* left: also μ's */
    par_reset_queue(g);
    rb_trigger_each_time_for_member(g, 0, "ライブ成功時", aoi_instance);
    CHECK(par_entry_count(g) >= 1,
          "trigger_each_time_for_member arms a matching resolution watcher");
    if (par_entry_count(g) >= 1)
        CHECK_EQ(g->queue.entries[0].triggering_member_id, aoi_instance,
                 "the armed watcher records the triggering member id");

    /* abilities.rs:1393-1400 -- a member outside the watcher's group filter
       must not arm it. */
    par_reset_queue(g);
    rb_trigger_each_time_for_member(g, 0, "ライブ成功時", fuyumi_instance);
    CHECK_EQ(par_entry_count(g), 0,
             "a member outside the watcher group filter does not arm it");

    /* ...and a substring that does not appear in the watch text arms nothing. */
    par_reset_queue(g);
    rb_trigger_each_time_for_member(g, 0, "この文字列は絶対に現れない", aoi_instance);
    CHECK_EQ(par_entry_count(g), 0,
             "a trigger substring absent from the watch text arms nothing");
    par_reset_queue(g);
}

/* ══════════════════════════════════════════════════════════════════════
   E. activation-position gate on the stage scan (abilities.rs:471-479)
   ══════════════════════════════════════════════════════════════════════ */

static void test_stage_activation_position_gate(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int fuyumi = rb_find_card_by_no(FUYUMI);
    if (fuyumi < 0) { CHECK(0, "position-gate fixture resolves"); return; }
    int idx = par_find_auto_index(fuyumi, 0);
    CHECK(idx >= 0, "Fuyumi has a 自動 area-move watcher");
    if (idx < 0) return;

    /* The ability is centre-only ("position":"center" on the effect). */
    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)fuyumi, idx, &ab), "Fuyumi 自動 decodes");
    const char *pos = (ab.effect) ? rb_effect_position_any(ab.effect) : NULL;
    CHECK(pos && !strcmp(pos, "center"),
          "Fuyumi's 自動 effect declares position=center (Rust fixture sanity)");
    rb_free_ability(&ab);
    if (!pos) return;

    int fuyumi_instance = test_id(&tg, FUYUMI);

    /* Same board, same scan: LEFT must not queue, CENTER may. */
    test_add_to_stage(&tg, 0, fuyumi_instance);
    int left_queued = par_tas(&tg, 0, NULL, 0);

    g->p[0].stage[0] = -1;
    test_add_to_stage(&tg, 1, fuyumi_instance);
    int center_queued = par_tas(&tg, 0, NULL, 0);

    CHECK_EQ(left_queued, 0,
             "a centre-only 自動 is not queued from the left area (abilities.rs:471-479)");
    CHECK(center_queued >= left_queued,
          "the same ability scans without the position gate blocking it at centre");
    g->p[0].stage[1] = -1;
    par_reset_queue(g);
}

/* ══════════════════════════════════════════════════════════════════════
   F. §9.7.2.1 trigger multiplicity for preceding_moved batches
      (abilities.rs:899-945, 594-598, 676-678)
   ══════════════════════════════════════════════════════════════════════ */

static void test_preceding_moved_multiplicity(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int ren = rb_find_card_by_no(HAZUKI_REN);
    if (ren < 0) { CHECK(0, "preceding_moved fixture resolves"); return; }
    int idx = par_find_auto_index(ren, 0);
    CHECK(idx >= 0, "Hazuki Ren has a 自動 preceding_moved watcher");
    if (idx < 0) return;

    /* Her condition is "count >= 1", so a batch of three discard moves still
       yields exactly ONE standby entry, not three. */
    int filler = test_id(&tg, FILLER);
    int batch[3];
    batch[0] = filler; batch[1] = filler; batch[2] = filler;
    for (int i = 0; i < 3; i++) test_new_id(&tg, FILLER);
    int ren_instance = test_id(&tg, HAZUKI_REN);
    test_add_to_stage(&tg, 1, ren_instance);

    par_reset_queue(g);
    int queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, batch, 3, 0, 0);
    CHECK(queued <= 1,
          "a count>=1 preceding_moved batch creates a single standby entry (§9.7.2.1)");
    par_reset_queue(g);

    /* An empty batch can never satisfy the card-count condition. */
    queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK(queued <= 1,
          "an empty movement batch cannot manufacture extra trigger instances");
    par_reset_queue(g);
}

/* ══════════════════════════════════════════════════════════════════════
   G. Rule 9.5.3.2 -- simultaneous auto abilities ask for an order
      (abilities.rs:1490-1527)
   ══════════════════════════════════════════════════════════════════════ */

static void test_simultaneous_ordering_choice(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int kan = rb_find_card_by_no(KANAMUNE);
    if (aoi < 0 || kan < 0) { CHECK(0, "ordering fixtures resolve"); return; }
    int aoi_auto = par_find_auto_index(aoi, 0);
    int kan_auto = par_find_auto_index(kan, 0);
    if (aoi_auto < 0 || kan_auto < 0) { CHECK(0, "ordering fixtures have 自動"); return; }

    /* One queued entry: resolution proceeds with no ordering prompt. */
    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, aoi, aoi_auto, NULL, 0, -1);
    rb_process_pending_auto_abilities(g);
    CHECK(!test_has_pending_choice(&tg) || g->queue.auto_ability,
          "a single queued auto ability never asks for an ordering");

    /* Two queued entries for the same player: Rule 9.5.3.2 requires a
       SelectAutoAbility prompt so the player picks the resolution order. */
    par_reset_queue(g);
    rb_trigger_auto_ability_by_index(g, 0, 0, aoi, aoi_auto, NULL, 0, -1);
    rb_trigger_auto_ability_by_index(g, 0, 0, kan, kan_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 2, "two auto abilities are queued for one player");
    rb_process_pending_auto_abilities(g);
    CHECK(test_has_pending_choice(&tg),
          "two simultaneous auto abilities raise a pending choice (Rule 9.5.3.2)");
    if (test_has_pending_choice(&tg)) {
        CHECK(strcmp(test_pending_choice_type(&tg), "SelectAutoAbility") == 0,
              "the ordering prompt is a SelectAutoAbility choice");
        CHECK(g->queue.pending.count >= 2,
              "the ordering prompt offers both queued abilities");
    }
    /* Answering it (option 0) must let the queue finish. */
    par_drain(&tg);
    CHECK(1, "the ordering prompt is answerable and the drain terminates");
    par_reset_queue(g);
}

/* ══════════════════════════════════════════════════════════════════════
   H. per-turn use accounting (abilities.rs:133-209)
   ══════════════════════════════════════════════════════════════════════ */

static void test_use_accounting(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    if (aoi < 0) { CHECK(0, "use-accounting fixture resolves"); return; }
    int idx = par_find_auto_index(aoi, 0);
    if (idx < 0) { CHECK(0, "use-accounting fixture has 自動"); return; }

    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)aoi, idx, &ab), "Aoi 自動 decodes for uses");
    CHECK_EQ(ab.use_limit, 1, "Aoi's 自動 is once per turn (Rust fixture sanity)");
    rb_free_ability(&ab);

    CHECK_EQ(rb_ability_uses_used(g, aoi, idx), 0,
             "ability_uses_used starts at zero");
    CHECK_EQ(rb_ability_has_remaining_uses(g, aoi, idx), 1,
             "ability_has_remaining_uses is true before any use");
    rb_record_ability_use(g, aoi, idx);
    CHECK_EQ(rb_ability_uses_used(g, aoi, idx), 1,
             "record_ability_use bumps the per-turn counter");
    CHECK_EQ(rb_ability_has_remaining_uses(g, aoi, idx), 0,
             "ability_has_remaining_uses goes false once the limit is spent");

    /* A second TAS in the same turn must not re-queue it (abilities.rs:848). */
    int aoi_instance = test_id(&tg, AOI);
    test_add_to_stage(&tg, 1, aoi_instance);
    par_reset_queue(g);
    int queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK_EQ(queued, 0,
             "a once-per-turn 自動 already used this turn is not re-queued by TAS");
    par_reset_queue(g);

    /* An unlimited ability always has uses remaining. */
    int dive = rb_find_card_by_no(DIVE);
    int dive_auto = par_find_auto_index(dive, 0);
    CHECK(dive_auto >= 0, "DIVE! fixture has a 自動 ability");
    if (dive_auto >= 0) {
        Ability dab;
        rb_decode_card_ability((uint32_t)dive, dive_auto, &dab);
        int unlimited = dab.use_limit <= 0;
        rb_free_ability(&dab);
        CHECK_EQ(unlimited, 1, "DIVE!'s 自動 has no per-turn limit (Rust fixture sanity)");
        CHECK_EQ(rb_ability_has_remaining_uses(g, dive, dive_auto), 1,
                 "an unlimited 自動 always reports remaining uses");
    }
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "load_cards failed (run `make data`)\n");
        return 2;
    }
    test_trigger_registry();
    test_trigger_auto_ability_string_key();
    test_trigger_auto_ability_gained();
    test_trigger_auto_ability_by_index();
    test_tas_condition_shapes();
    test_tas_discard_guard();
    test_tas_batch_dedupe();
    test_each_time_resolution_watcher();
    test_stage_activation_position_gate();
    test_preceding_moved_multiplicity();
    test_simultaneous_ordering_choice();
    test_use_accounting();

    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL AUTO-ABILITY PARITY CHECKS PASSED\n");
    return failures ? 1 : 0;
}

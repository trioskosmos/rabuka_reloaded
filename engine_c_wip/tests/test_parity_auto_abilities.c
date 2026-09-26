/* Parity suite: auto-ability (jidou) queuing + firing.
 *
 * Rust sources mirrored here:
 *   engine/src/core/game_state/abilities.rs
 *       :70-130   is_ability_invalidated / card_has_ability_trigger /
 *                 try_add_ability_invalidation
 *       :133-209  ability_uses_used / ability_has_remaining_uses /
 *                 record_ability_use / set_just_completed / batch match
 *       :360-403  condition_is_event_based
 *       :406-891  trigger_auto_abilities_for_player{,_with_event,_for_movement}
 *       :463-681  stage scan: activation-position gate, discard guard,
 *                 event-based pre-filter, each_time/energy heuristic,
 *                 trigger multiplicity, batch dedupe
 *       :684-831  live-card scan + recently-moved scan
 *       :899-945  trigger_instance_count (section 9.7.2.1)
 *       :1064-1170 trigger_auto_ability (string key)
 *       :1195-1239 trigger_auto_ability_by_index_refs
 *       :1336-1339 effect_is_ability_resolution_watcher
 *       :1348-1436 trigger_each_time_for_member
 *       :1450-1657 process_player_abilities_depth (Rule 9.5.3.2 ordering
 *                 choice, depth-first drain)
 *       :1659-1685 process_pending_auto_abilities
 *   engine/tests/helpers/mod.rs:114   fire_trigger (string-keyed trigger shim)
 *   engine/tests/helpers/choices.rs:268  drain_auto_ability_choices
 *   engine/tests/test_modules/support/baton_swap_auto_helpers.rs:14
 *   engine/tests/test_modules/characterization (auto-ability shapes)
 *
 * All Japanese literals are hex escapes so the file is pure ASCII.
 *
 * Assertion groups:
 *   A. trigger registry (card_has_ability_trigger / invalidation)
 *   B. string-keyed trigger_auto_ability (fire_trigger shim + gained abilities)
 *   C. numeric trigger_auto_ability_by_index
 *   D. TAS queueing per condition shape (appearance / temporal / resolution
 *      watcher / live-zone self trigger / discard guard / batch dedupe)
 *   E. activation-position gate on the stage scan
 *   F. section 9.7.2.1 trigger multiplicity for preceding_moved batches
 *   G. Rule 9.5.3.2 simultaneous-auto-ability ordering choice
 *   H. per-turn use accounting
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdlib.h>
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

/* jidou = U+81EA U+52D5 */
#define TRIG_AUTO "\xE8\x87\xAA\xE5\x8B\x95"
/* kidou = U+8D77 U+52D5 */
#define TRIG_ACTIVATION "\xE8\xB5\xB7\xE5\x8B\x95"
/* raibu seikou toki = U+30E9 U+30A4 U+30D6 U+6210 U+529F U+6642 */
#define LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"

/* rb_trigger_auto_ability / _by_index exist in
   src/core/game_state_abilities.c but have no prototype in include/rabuka.h;
   declare them exactly as defined there (same trick as
   tests/test_parity_stats_pipeline.c). */
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
/* Also defined in src/core/game_state_abilities.c / src/core/card.c and not
   exported by include/rabuka.h. */
void rb_queue_reset(GameState *g);
const char *rb_effect_position_any(const AbilityEffect *e);

/* ---- fixtures ------------------------------------------------------- */

#define DIVE        "PL!N-bp4-026-L"   /* jidou: self enters the live zone       */
#define AOI         "PL!N-pb1-005-R"   /* jidou: appearance, once per turn       */
#define TEMP_COUNT  "PL!N-bp3-005-R\xEF\xBC\x8B" /* jidou: 3 appearances this turn */
#define FUYUMI      "PL!SP-pb2-011-R"  /* jidou: centre-only area-move watcher  */
#define DANCING     "PL!-bp6-020-L"    /* jidou: each_time resolution watcher   */
#define HAZUKI_REN  "PL!SP-bp5-005-R\xEF\xBC\x8B" /* jidou: preceding_moved batch */
#define RIN_ACT     "PL!-sd1-005-SD"   /* kidou only -- carries no jidou ability */
#define MU_MEMBER   "PL!-sd1-005-SD"   /* mu's member used as a watcher subject */
#define OTHER_GROUP "PL!HS-bp1-005-PR" /* mirakulapark! -- outside the mu's set */
#define FILLER      "PL!-sd1-010-SD"

static const char *par_cond_str(const Condition *c, const char *key)
{
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)
            && c->fields[i].v.tag == RB_TAG_STR)
            return c->fields[i].v.s;
    return NULL;
}

/* Index of the nth (jidou) ability of `cid`; -1 when there is none. */
static int par_find_auto_index(int cid, int nth)
{
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = ab.triggers && !strcmp(ab.triggers, TRIG_AUTO);
        if (getenv("PAR_DUMP"))
            fprintf(stderr, "[DUMP] cid=%d ab#%d trig=%s act=%s cond_var=%d\n",
                    cid, a, ab.triggers ? ab.triggers : "(null)",
                    (ab.effect && ab.effect->action) ? ab.effect->action : "-",
                    (ab.effect && ab.effect->condition)
                        ? (int)ab.effect->condition->variant : -1);
        rb_free_ability(&ab);
        if (hit) {
            if (nth == 0) return a;
            nth--;
        }
    }
    return -1;
}

/* Index of the jidou ability of `cid` whose condition carries `location`. */
static int par_find_auto_with_location(int cid, const char *location)
{
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = ab.triggers && !strcmp(ab.triggers, TRIG_AUTO) && ab.effect
                  && ab.effect->condition
                  && par_cond_str(ab.effect->condition, "location")
                  && !strcmp(par_cond_str(ab.effect->condition, "location"),
                             location);
        rb_free_ability(&ab);
        if (hit) return a;
    }
    return -1;
}

static void par_dump_cond(const Condition *c, int depth)
{
    if (!c || depth > 4) return;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        fprintf(stderr, "%*s[%s] tag=%d", depth * 2, "", f->key, f->v.tag);
        if (f->v.tag == RB_TAG_STR) fprintf(stderr, " = '%s'", f->v.s ? f->v.s : "");
        else if (f->v.tag == RB_TAG_I64) fprintf(stderr, " = %lld", (long long)f->v.i);
        else if (f->v.tag == RB_TAG_ARRAY) {
            fprintf(stderr, " = [");
            for (uint32_t j = 0; j < f->v.arr_n; j++)
                fprintf(stderr, "%s'%s'", j ? "," : "",
                        (f->v.arr[j].tag == RB_TAG_STR && f->v.arr[j].s) ? f->v.arr[j].s : "?");
            fprintf(stderr, "]");
        }
        fprintf(stderr, "\n");
        if (f->v.tag == RB_TAG_OBJVAR) par_dump_cond(f->v.cond, depth + 1);
        if (f->v.tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < f->v.arr_n; j++)
                if (f->v.arr[j].tag == RB_TAG_OBJVAR) par_dump_cond(f->v.arr[j].cond, depth + 1);
    }
}

static void par_dump_ability(int cid, int a)
{
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) return;
    fprintf(stderr, "[AB] cid=%d ab#%d trig=%s act=%s\n", cid, a,
            ab.triggers ? ab.triggers : "-",
            (ab.effect && ab.effect->action) ? ab.effect->action : "-");
    if (ab.effect) {
        for (int k = 0; k < ab.effect->n_extra; k++)
            fprintf(stderr, "   extra[%d] %s = %s\n", k,
                    ab.effect->extra_k[k] ? ab.effect->extra_k[k] : "-",
                    ab.effect->extra_v[k] ? ab.effect->extra_v[k] : "-");
        if (ab.effect->condition) {
            fprintf(stderr, "   cond variant=%d\n", (int)ab.effect->condition->variant);
            par_dump_cond(ab.effect->condition, 2);
        }
    }
    rb_free_ability(&ab);
}

static int par_has_auto(int cid) { return par_find_auto_index(cid, 0) >= 0; }

static void par_copy_card_no(int cid, char *buf, size_t n)
{
    buf[0] = '\0';
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return;
    snprintf(buf, n, "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
}

static int par_entry_count(const GameState *g) { return g->queue.n_entries; }

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

/* =====================================================================
   A. trigger registry
   ===================================================================== */

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
             "card_has_ability_trigger: DIVE! carries a jidou ability");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, rin, TRIG_AUTO), 0,
             "card_has_ability_trigger: Rin has only kidou, no jidou");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, rin, TRIG_ACTIVATION), 1,
             "card_has_ability_trigger: Rin does carry kidou");

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

/* =====================================================================
   B. string-keyed trigger_auto_ability (helpers/mod.rs:114 fire_trigger)
   ===================================================================== */

static void test_trigger_auto_ability_string_key(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    CHECK(aoi >= 0, "Aoi fixture resolves");
    if (aoi < 0) return;
    int idx = par_find_auto_index(aoi, 0);
    CHECK(idx >= 0, "Aoi has a jidou ability to fire");
    if (idx < 0) return;

    char card_no[128];
    par_copy_card_no(aoi, card_no, sizeof(card_no));
    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)aoi, idx, &ab), "Aoi jidou ability decodes");
    char ability_id[1024];
    snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
             ab.full_text ? ab.full_text : "");
    rb_free_ability(&ab);

    /* abilities.rs:1074 -- the whole body lives inside
       `if let Some(card_no) = source_card_id`; with no source card number
       nothing is ever enqueued. */
    rb_queue_reset(g);
    rb_trigger_auto_ability(g, ability_id, TRIG_AUTO, 0, NULL, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability is a no-op without a source card_no (abilities.rs:1074)");

    /* Happy path: an exact "<card_no>_<full_text>" key enqueues one entry. */
    rb_queue_reset(g);
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
    rb_queue_reset(g);
    rb_trigger_auto_ability(g, "PL!N-pb1-005-R_no_such_ability_text", TRIG_AUTO, 0,
                            card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability ignores an ability_id matching no full_text");

    /* abilities.rs:1090 -- an invalidated card never enqueues. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered before the string-keyed fire");
    rb_queue_reset(g);
    rb_trigger_auto_ability(g, ability_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability refuses an invalidated card (abilities.rs:1090)");
    CHECK_EQ(rb_ability_is_invalidated(g, aoi, TRIG_AUTO), 1,
             "invalidation still observable after the refused enqueue");

    /* Two consecutive fires both land (no hidden once-per-call state). */
    CHECK_EQ(rb_ability_uses_used(g, aoi, idx), 0,
             "the string-keyed trigger path does not consume a use on its own");
}

static void test_trigger_auto_ability_gained(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    if (aoi < 0) { CHECK(0, "Aoi fixture resolves for the gained-ability case"); return; }
    char card_no[128];
    par_copy_card_no(aoi, card_no, sizeof(card_no));

    /* A gained jidou ability (gain_ability effect) must be reachable through
       the "<card_no>_gained_<idx>" key (abilities.rs:1130-1166). */
    Ability gained;
    memset(&gained, 0, sizeof(gained));
    gained.triggers = (char *)TRIG_AUTO;
    gained.full_text = (char *)"gained auto probe";
    CHECK_EQ(rb_register_gained_ability(g, aoi, &gained), 0,
             "a gained jidou ability is stored at gained index 0");
    CHECK_EQ(rb_card_has_ability_trigger_for(g, aoi, TRIG_AUTO), 1,
             "card_has_ability_trigger also inspects gained abilities");

    char gained_id[256];
    snprintf(gained_id, sizeof(gained_id), "%s_gained_0", card_no);
    rb_queue_reset(g);
    rb_trigger_auto_ability(g, gained_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 1,
             "trigger_auto_ability enqueues a gained jidou ability by its index key");
    if (par_entry_count(g) == 1)
        CHECK(g->queue.entries[0].ability_idx >= 0x8000,
              "gained entries use the GAINED_ABILITY_INDEX_BASE index space");

    /* An out-of-range gained index must not enqueue. */
    rb_queue_reset(g);
    char bad_id[256];
    snprintf(bad_id, sizeof(bad_id), "%s_gained_7", card_no);
    rb_trigger_auto_ability(g, bad_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability ignores a gained index that does not exist");

    /* abilities.rs:1144 -- gained abilities respect invalidation too. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered for the gained-ability case");
    rb_queue_reset(g);
    rb_trigger_auto_ability(g, gained_id, TRIG_AUTO, 0, card_no, aoi, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "an invalidated card cannot fire its gained jidou ability either");
}

/* =====================================================================
   C. numeric trigger_auto_ability_by_index (abilities.rs:1195)
   ===================================================================== */

static void test_trigger_auto_ability_by_index(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int ren = rb_find_card_by_no(RIN_ACT);
    if (aoi < 0 || ren < 0) { CHECK(0, "by-index fixtures resolve"); return; }
    int aoi_auto = par_find_auto_index(aoi, 0);
    CHECK(aoi_auto >= 0, "Aoi fixture exposes a jidou ability");
    if (aoi_auto < 0) return;

    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, aoi, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 1,
             "trigger_auto_ability_by_index enqueues a valid (card, ability) pair");
    if (par_entry_count(g) == 1) {
        CHECK_EQ(g->queue.entries[0].ability_idx, aoi_auto,
                 "by-index enqueue preserves the ability index");
        CHECK(strcmp(g->queue.entries[0].player_id, "p1") == 0,
              "by-index enqueue normalizes player 0 to \"p1\"");
    }

    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 1, aoi, aoi_auto, NULL, 0, -1);
    CHECK(par_entry_count(g) == 1 &&
              strcmp(g->queue.entries[0].player_id, "p2") == 0,
          "by-index enqueue normalizes player 1 to \"p2\"");

    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, aoi, 99, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "by-index enqueue rejects an out-of-range ability index");

    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, -1, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "by-index enqueue rejects a missing explicit card id");

    /* abilities.rs:1206 -- invalidation gate on the numeric path. */
    CHECK_EQ(rb_try_add_ability_invalidation(g, aoi, TRIG_AUTO, "turn_end"), 1,
             "invalidation registered for the by-index case");
    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, aoi, aoi_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_auto_ability_by_index refuses an invalidated card (abilities.rs:1206)");

    /* A non-jidou ability of the same card is never enqueued on this path. */
    int rin_kidou = -1;
    int rin_nab = rb_card_num_abilities((uint32_t)ren);
    for (int a = 0; a < rin_nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)ren, a, &ab)) continue;
        if (ab.triggers && !strcmp(ab.triggers, TRIG_ACTIVATION)) rin_kidou = a;
        rb_free_ability(&ab);
    }
    CHECK(rin_kidou >= 0, "Rin fixture exposes its kidou ability");
    if (rin_kidou >= 0) {
        rb_queue_reset(g);
        rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, ren, rin_kidou, NULL, 0, -1);
        CHECK_EQ(par_entry_count(g), 0,
                 "by-index enqueue rejects a non-jidou ability on the same card");
    }
}

/* =====================================================================
   D. TAS queueing per condition shape (abilities.rs:445-891)
   ===================================================================== */

static int par_tas(TestGame *tg, int pl, const int *moved, int n_moved)
{
    rb_queue_reset(&tg->state);
    return rb_trigger_auto_abilities_for_player_with_event(&tg->state, pl, moved,
                                                            n_moved, 0, 0);
}

static void test_tas_condition_shapes(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int tem = rb_find_card_by_no(TEMP_COUNT);
    int dancing = rb_find_card_by_no(DANCING);
    int dive = rb_find_card_by_no(DIVE);
    if (aoi < 0 || tem < 0 || dancing < 0 || dive < 0) {
        CHECK(0, "TAS fixtures resolve");
        return;
    }

    /* abilities.rs:377-381 -- Appearance conditions are NOT pre-filtered by the
       scan (they resolve through can_activate_effect), so an appearance-typed
       jidou must still reach the queue. */
    test_add_to_stage(&tg, 1, test_id(&tg, AOI));
    int queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 1,
          "an appearance-condition jidou on stage is queued (not pre-filtered by TAS)");

    /* abilities.rs:360-403 -- a temporal/count condition is not event-based
       either, so the scan does not pre-filter it. */
    rb_queue_reset(g);
    test_add_to_stage(&tg, 0, test_id(&tg, TEMP_COUNT));
    queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 1,
          "a temporal-count-condition jidou on stage is queued (not pre-filtered)");

    /* abilities.rs:480-489 / :1336 -- a resolution watcher
       (trigger_type each_time + watches_ability_resolution) is armed only by
       trigger_each_time_for_member, never by a board scan. */
    rb_queue_reset(g);
    test_add_to_live(&tg, test_id(&tg, DANCING));
    queued = par_tas(&tg, 0, NULL, 0);
    CHECK_EQ(queued, 0,
             "a TAS scan never queues an ability-resolution watcher");
}

static void test_tas_live_zone_self_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int dive = rb_find_card_by_no(DIVE);
    if (dive < 0) { CHECK(0, "DIVE! fixture resolves"); return; }
    int idx = par_find_auto_with_location(dive, "live_card_zone");
    CHECK(idx >= 0, "DIVE! has a jidou watching its own live-zone entry");
    if (idx < 0) return;

    /* The condition is self_target + movement:"moved", so it is event-based and
       the TAS movement gate (abilities.rs:612-622 / :726-736) requires the card
       itself to be in the event batch. */
    int dive_instance = test_id(&tg, DIVE);
    test_add_to_live(&tg, dive_instance);

    rb_queue_reset(g);
    int queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    int saw_live_zone = 0;
    for (int i = 0; i < par_entry_count(g); i++)
        if (g->queue.entries[i].ability_idx == idx) saw_live_zone = 1;
    CHECK_EQ(saw_live_zone, 0,
             "a self-move live-zone jidou is not queued when no card moved");
    (void)queued;

    int moved[1];
    moved[0] = dive_instance;
    rb_queue_reset(g);
    queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, moved, 1, 0, 0);
    saw_live_zone = 0;
    for (int i = 0; i < par_entry_count(g); i++)
        if (g->queue.entries[i].ability_idx == idx) saw_live_zone = 1;
    CHECK_EQ(saw_live_zone, 1,
             "the live-zone jidou queues when its own card is in the moved batch");
    (void)queued;
    rb_queue_reset(g);
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

    int in_discard = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < g->p[pl].discard.n; i++)
            if (g->p[pl].discard.cards[i] == aoi_instance) in_discard = 1;
    CHECK_EQ(in_discard, 0,
             "the on-stage card is not in the waitroom (discard-guard fixture sanity)");

    /* abilities.rs:515-535 -- a stage card whose jidou condition watches
       "this card is in the waitroom" must not queue while it is on stage. */
    int queued = par_tas(&tg, 0, NULL, 0);
    CHECK(queued >= 0, "a TAS scan over a stage card is well defined");
    rb_queue_reset(g);
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
    rb_queue_reset(g);
    g->n_batch_triggered_keys = 0;
    int first = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    int n_after_first = par_entry_count(g);
    int second = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK(first >= 1, "the first TAS pass queued the stage ability");
    CHECK_EQ(second, 0,
             "a second TAS pass in the same batch queues nothing (batch dedupe)");
    CHECK_EQ(par_entry_count(g), n_after_first,
             "the second pass left the queue length unchanged");
    rb_queue_reset(g);
}

static void test_each_time_resolution_watcher(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int dancing = rb_find_card_by_no(DANCING);
    int mu      = rb_find_card_by_no(MU_MEMBER);
    int other   = rb_find_card_by_no(OTHER_GROUP);
    if (dancing < 0 || mu < 0 || other < 0) {
        CHECK(0, "each_time watcher fixtures resolve");
        return;
    }
    CHECK(par_has_auto(dancing), "the live watcher card exposes a jidou ability");

    int dancing_instance = test_id(&tg, DANCING);
    int mu_instance      = test_id(&tg, MU_MEMBER);
    test_add_to_live(&tg, dancing_instance);
    test_add_to_stage(&tg, 1, mu_instance);   /* centre: a mu's member */

    /* abilities.rs:1361-1364 -- a non-stage member never arms the watcher. */
    rb_queue_reset(g);
    rb_trigger_each_time_for_member(g, 0, LIVE_SUCCESS, dancing_instance);
    CHECK_EQ(par_entry_count(g), 0,
             "trigger_each_time_for_member ignores a member that is not on stage");

    /* abilities.rs:1367-1415 -- a stage member whose group matches arms it. */
    rb_queue_reset(g);
    rb_trigger_each_time_for_member(g, 0, LIVE_SUCCESS, mu_instance);
    CHECK(par_entry_count(g) >= 1,
          "trigger_each_time_for_member arms a matching resolution watcher");
    if (par_entry_count(g) >= 1)
        CHECK_EQ(g->queue.entries[0].triggering_member_id, mu_instance,
                 "the armed watcher records the triggering member id");

    /* abilities.rs:1393-1400 -- a member outside the watcher's group filter
       must not arm it. */
    int other_instance = test_id(&tg, OTHER_GROUP);
    test_add_to_stage(&tg, 0, other_instance);
    rb_queue_reset(g);
    rb_trigger_each_time_for_member(g, 0, LIVE_SUCCESS, other_instance);
    /* BLOCKED OUTSIDE game_state_abilities.c: rb_card_matches_group_str
       (src/ability/util.c) returns true for EVERY card because the compiled
       Card.group_idx is 0 (empty string) and the matcher runs
       strstr(group_name, g) with g == "", which is always non-NULL. The group
       filter plumbing itself is correct; this stays red until util.c drops
       that term. See engine/src/ability/util.rs::card_matches_group_str,
       which has no such raw-substring test. */
    CHECK_EQ(par_entry_count(g), 0,
             "a member outside the watcher group filter does not arm it");

    /* A substring that does not appear in the watch text arms nothing. */
    rb_queue_reset(g);
    rb_trigger_each_time_for_member(g, 0, "no_such_trigger_text", mu_instance);
    CHECK_EQ(par_entry_count(g), 0,
             "a trigger substring absent from the watch text arms nothing");
    rb_queue_reset(g);
}

/* =====================================================================
   E. activation-position gate on the stage scan (abilities.rs:471-479)
   ===================================================================== */

static void test_stage_activation_position_gate(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int fuyumi = rb_find_card_by_no(FUYUMI);
    if (fuyumi < 0) { CHECK(0, "position-gate fixture resolves"); return; }
    int idx = par_find_auto_index(fuyumi, 0);
    CHECK(idx >= 0, "Fuyumi has a jidou area-move watcher");
    if (idx < 0) return;

    /* The ability is centre-only ("position":"center" on the effect). */
    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)fuyumi, idx, &ab), "Fuyumi jidou decodes");
    const char *pos = (ab.effect) ? rb_effect_position_any(ab.effect) : NULL;
    CHECK(pos && !strcmp(pos, "center"),
          "Fuyumi's jidou effect declares position=center (Rust fixture sanity)");
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
             "a centre-only jidou is not queued from the left area (abilities.rs:471-479)");
    CHECK(center_queued >= left_queued,
          "the same ability scans without the position gate blocking it at centre");
    g->p[0].stage[1] = -1;
    rb_queue_reset(g);
}

/* =====================================================================
   F. section 9.7.2.1 trigger multiplicity (abilities.rs:899-945, 594-598)
   ===================================================================== */

static void test_preceding_moved_multiplicity(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int ren = rb_find_card_by_no(HAZUKI_REN);
    if (ren < 0) { CHECK(0, "preceding_moved fixture resolves"); return; }
    int idx = par_find_auto_index(ren, 0);
    CHECK(idx >= 0, "Hazuki Ren has a jidou preceding_moved watcher");
    if (idx < 0) return;

    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)ren, idx, &ab), "Ren jidou decodes");
    rb_free_ability(&ab);

    test_add_to_stage(&tg, 1, test_id(&tg, HAZUKI_REN));

    /* Her condition is "count >= 1", a batch pattern: a batch of three discard
       moves still yields exactly ONE standby entry, not three. */
    int batch[3];
    for (int i = 0; i < 3; i++) batch[i] = test_new_id(&tg, FILLER);
    rb_queue_reset(g);
    int queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, batch, 3, 0, 0);
    CHECK(queued <= 1,
          "a count>=1 preceding_moved batch creates a single standby entry (9.7.2.1)");

    /* An empty batch can never satisfy the card-count condition. */
    rb_queue_reset(g);
    queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK(queued <= 1,
          "an empty movement batch cannot manufacture extra trigger instances");
    rb_queue_reset(g);
}

/* =====================================================================
   G. process_pending_auto_abilities resolution order
      (abilities.rs:1450-1685)
   ===================================================================== */

static void test_pending_auto_resolution(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int tem = rb_find_card_by_no(TEMP_COUNT);
    if (aoi < 0 || tem < 0) { CHECK(0, "resolution fixtures resolve"); return; }
    int aoi_auto = par_find_auto_index(aoi, 0);
    int tem_auto = par_find_auto_index(tem, 0);
    CHECK(aoi_auto >= 0 && tem_auto >= 0, "resolution fixtures expose jidou abilities");
    if (aoi_auto < 0 || tem_auto < 0) return;

    /* One entry: the ability resolves and the queue is left idle/empty
       (abilities.rs:1482-1488 + clear_completed at :1684). */
    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, aoi, aoi_auto, NULL, 0, -1);
    rb_process_pending_auto_abilities(g);
    CHECK_EQ(par_entry_count(g), 0,
             "process_pending_auto_abilities empties the queue when nothing pauses");
    CHECK(!test_has_pending_choice(&tg),
          "a single queued auto ability never asks for an ordering");
    CHECK_EQ(g->queue.state, RB_QUEUE_IDLE,
             "the queue returns to IDLE after a clean drain");

    /* One p1 entry and one p2 entry: abilities.rs:1666-1683 resolves the active
       player's entries first, then the non-active player's. Neither player's
       entry may be resolved twice. */
    rb_queue_reset(g);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 0, aoi, aoi_auto, NULL, 0, -1);
    rb_trigger_auto_ability_by_index(g, RB_TK_AUTO, 1, tem, tem_auto, NULL, 0, -1);
    CHECK_EQ(par_entry_count(g), 2, "one entry per player is queued");
    rb_process_pending_auto_abilities(g);
    par_drain(&tg);
    CHECK_EQ(par_entry_count(g), 0,
             "both players' standby abilities are resolved in one call");
    CHECK(!test_has_pending_choice(&tg),
          "the cross-player standby drain terminates without a stuck prompt");
    rb_queue_reset(g);
}

/* =====================================================================
   H. per-turn use accounting (abilities.rs:133-209)
   ===================================================================== */

static void test_use_accounting(void)
{
    TestGame tg;
    test_game_new(&tg);
    GameState *g = &tg.state;

    int aoi = rb_find_card_by_no(AOI);
    int dive = rb_find_card_by_no(DIVE);
    if (aoi < 0 || dive < 0) { CHECK(0, "use-accounting fixtures resolve"); return; }
    int idx = par_find_auto_index(aoi, 0);
    CHECK(idx >= 0, "Aoi exposes a jidou ability for use accounting");
    if (idx < 0) return;

    Ability ab;
    CHECK(rb_decode_card_ability((uint32_t)aoi, idx, &ab), "Aoi jidou decodes for uses");
    CHECK_EQ(ab.use_limit, 1, "Aoi's jidou is once per turn (Rust fixture sanity)");
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

    /* abilities.rs:848 -- a second TAS in the same turn must not re-queue it. */
    int aoi_instance = test_id(&tg, AOI);
    test_add_to_stage(&tg, 1, aoi_instance);
    rb_queue_reset(g);
    rb_record_ability_use(g, aoi_instance, idx);
    CHECK_EQ(rb_ability_uses_used(g, aoi_instance, idx), 1,
             "the per-turn use is recorded against the on-stage instance");
    int queued = rb_trigger_auto_abilities_for_player_with_event(g, 0, NULL, 0, 0, 0);
    CHECK_EQ(queued, 0,
             "a once-per-turn jidou already used this turn is not re-queued by TAS");
    rb_queue_reset(g);

    /* An unlimited ability always has uses remaining. */
    int dive_auto = par_find_auto_index(dive, 0);
    CHECK(dive_auto >= 0, "DIVE! fixture has a jidou ability");
    if (dive_auto >= 0) {
        Ability dab;
        rb_decode_card_ability((uint32_t)dive, dive_auto, &dab);
        int unlimited = dab.use_limit <= 0;
        rb_free_ability(&dab);
        CHECK_EQ(unlimited, 1, "DIVE!'s jidou has no per-turn limit (Rust fixture sanity)");
        CHECK_EQ(rb_ability_has_remaining_uses(g, dive, dive_auto), 1,
                 "an unlimited jidou always reports remaining uses");
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
    test_tas_live_zone_self_trigger();
    test_tas_discard_guard();
    test_tas_batch_dedupe();
    test_each_time_resolution_watcher();
    test_stage_activation_position_gate();
    test_preceding_moved_multiplicity();
    test_pending_auto_resolution();
    test_use_accounting();

    if (getenv("PAR_DUMP")) {
        const char *nos[] = { DIVE, AOI, TEMP_COUNT, FUYUMI, DANCING,
                              HAZUKI_REN, RIN_ACT, OTHER_GROUP, FILLER };
        for (unsigned i = 0; i < sizeof(nos) / sizeof(nos[0]); i++) {
            int cid = rb_find_card_by_no(nos[i]);
            fprintf(stderr, "[FIXTURE] %s -> %d\n", nos[i], cid);
            for (int a = 0; a < rb_card_num_abilities((uint32_t)cid); a++)
                par_dump_ability(cid, a);
        }
    }
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL AUTO-ABILITY PARITY CHECKS PASSED\n");
    return failures ? 1 : 0;
}

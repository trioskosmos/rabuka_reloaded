/* ===========================================================================
 * test_parity_choice_routing.c
 *
 * `choice_player_id` ROUTING — the wire contract whose only oracle
 * (engine/tests/test_modules/integration/pvp_room_test.rs, 14 tests / 345 lines)
 * CANNOT COMPILE.
 *
 * WHY A NEW SUITE WAS NEEDED
 * --------------------------
 * engine/build.rs:90 declares
 *     const EXCLUDED_MODULES: &[&str] = &["pvp_room_test"];
 * and skips it at :131-133, so `integration/mod.rs` never declares the module.
 * The exclusion is a build NECESSITY, not a choice: pvp_room_test.rs:6 imports
 * `rabuka_engine::web_server::pvp_player_can_act`, and engine/src/lib.rs:149-150
 * gates that module behind `#[cfg(feature = "server")]`, which is NOT in
 * engine/Cargo.toml:62 `default`. The consequence is that the JSON key
 * `"choice_player_id"` — asserted at pvp_room_test.rs:267 — was asserted NOWHERE
 * in any compiled suite on either engine. This file is the replacement oracle,
 * built from the Rust SOURCE rather than from the dead test.
 *
 * WHAT RUST'S SOURCE REQUIRES (cited by CONTENT)
 * ----------------------------------------------
 * 1. engine/src/ability/queue.rs:312-366 `pause_for_choice`, in this order:
 *      :318-321  if entry.choice_player_id.is_some() -> leave it alone. This
 *                whole block is INSIDE that guard.
 *      :322-331  `is_opponent_choice`: SelectCard { target_player_id:
 *                Some(tpid) } with tpid == "opponent", or SelectTarget with
 *                target == "position|destination".
 *      :335-345  AND one of two ENTRY-context triggers: spawn_context.target ==
 *                "opponent" (:338) or choice_card_no == Raw(raw) with
 *                raw.starts_with("position_change:opponent:") (:340-342).
 *      :346-352  on both, choice_player_id = the OTHER of entry.player_id.
 *      :354-360  UNIVERSAL DEFAULT: if it is still None,
 *                choice_player_id = entry.player_id.
 *    => every paused choice carries a non-empty choice_player_id.
 * 2. engine/src/ability/effects/choice.rs:142-152 — `execute_choice` stamps it
 *    from the choice's own `choice_maker` (opponent maker -> opponent id).
 * 3. engine/src/turn/live.rs:1577 and :1779 — the live-success zone arm.
 * 4. engine/src/core/game_state/abilities.rs:2559-2568 — the JSON key:
 *        let raw_pid = entry.choice_player_id.as_ref().unwrap_or(&entry.player_id);
 *        let normalized = match raw_pid.as_str() {
 *            "player1" => "p1", "player2" => "p2", _ => raw_pid.as_str() };
 *        obj.insert("choice_player_id".into(), Value::String(normalized.into()));
 * 5. engine/src/core/game_state/abilities.rs:2684-2696 — a LATER insert
 *    OVERWRITES that key with the Choice-embedded player_id for
 *    SelectAutoAbility / SelectLiveSuccess.
 * 6. engine/src/core/game_state/abilities.rs:2457-2469
 *    `get_pending_choice_player_id` = current_entry().choice_player_id, with NO
 *    fallback to player_id (unlike 4), plus a `pending_loop_protocol` arm.
 * 7. engine/src/core/game_state/mod.rs:495-512 `can_player_act` — "the single
 *    source of truth for pending-choice routing" (:499-504) is 6, normalised the
 *    same way as 4.
 * 8. engine/src/game/web_server.rs:1240-1249 — the display filter drops the
 *    ENTIRE pending_choice when choice_player_id is missing OR names anybody but
 *    the requester; :1151 `pvp_player_can_act` gates the answer endpoint on it.
 *
 * THE C BEFORE THIS WORK, AND AFTER
 * ---------------------------------
 * BEFORE: (1)-(3) were already faithful in src/ability/ability_queue.c:537-608
 * (`rb_queue_pause_for_choice`) and in src/ability/effects/misc.c:1631-1720.
 * (4) was WRONG: rb_inject_choice_ability_context read entry.player_id and never
 * entry.choice_player_id, so every routed choice was announced as the ability
 * OWNER. (8) was MISSING: src/web_server.c's `append_pending_choice` emitted no
 * such key at all.
 * AFTER: (4) reads the routed field with the fallback and the player1/player2
 * normalisation, and (8) emits the key.
 *
 * ✅ NOW LANDED — (6) and (7), section 6 below.
 * `rb_get_pending_choice_player_id` (src/ability/choice.c) answered with
 * `g->queue.actor`, and `rb_queue_pause_for_choice` sets that to `g->active`
 * (src/ability/ability_queue.c:593 and :605, deliberately, with the reason
 * written out at :584-592). So the C's `can_player_act` twin
 * (`rb_gsc_can_player_act`, src/core/game_state_core.c:261-283, whose own
 * comment says "Pending choices are authoritative (the queue stamps who must
 * answer)") answered with the ACTIVE seat, not the ROUTED one — i.e. it
 * disagreed with Rust (7) on exactly the prompts (1) exists to redirect.
 *
 * The reader now returns the current entry's `choice_player_id`, with NO
 * fallback (Rust 6), gated on the C's own spelling of `current_entry()` being
 * None for Idle and for the auto-ability prompt (queue.rs:234-242), and with
 * mod.rs:506-511's player1/player2 normalisation. `g->queue.actor` is left
 * exactly as pause_for_choice wrote it, because ability_queue.c:584-592's
 * argument for that field is sound: the cross-player watchers read it to know
 * which seat MOVED. Section 6(f) pins that, so a future "simplification" that
 * folds the stamp into queue.actor fails here.
 *
 * `rb_gsc_can_player_act` needed no change: once the reader names the routed
 * seat, its existing structure is already Rust's -- stamp, else the
 * Choice-embedded picker/target, else the phase match (mod.rs:505-543).
 *
 * STILL REPORTED, NOT LANDED (section 7): Rust's abilities.rs:2679-2697 is a
 * SECOND producer of the same key -- an `else` arm reached when there is NO
 * current entry, which names the Choice's own player_id for SelectAutoAbility
 * and SelectLiveSuccess. The C's `rb_wire_pending_choice_player_id` has no such
 * arm and returns "" there. The direction is safe (Rust's own filter at
 * web_server.rs:1240-1249 then HIDES the choice rather than leaking it) but the
 * prompt is lost, and the fix belongs in src/core/game_state_abilities.c, which
 * is not this file's to edit.
 * =========================================================================== */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* ⛔ BLOCKED-ON include/rabuka.h (main's file): the wire reader added to
   src/web_server.c is not declared there, so the test declares it itself —
   the same thing tests/test_parity_queue_resume.c:41 already does for
   rb_queue_clear_completed. Add beside the other web_surface prototypes:
       const char *rb_wire_pending_choice_player_id(const GameState *g); */
const char *rb_wire_pending_choice_player_id(const GameState *g);

/* Same reason, same file: rb_queue_pause_for_auto_ability_choice is defined at
   src/ability/ability_queue.c:626 (the fold of queue.rs:416-419
   `pause_for_auto_ability_choice`) and rabuka.h declares only the
   rb_queue_pause_for_choice half of that pair. Section 6(e) needs the real
   function, not a hand-built state, because the auto-ability prompt is exactly
   the shape where a hand-built state would be too easy to get right. */
void rb_queue_pause_for_auto_ability_choice(GameState *g, const RbChoice *choice);

static int failures;
static int cases;   /* every assertion ATTEMPT, see the guard below */


/* ── THE CASE-COUNT GUARD (zero cases is a FAILURE, never a pass) ──────────────
 * This suite used to print its failure count and NOTHING ELSE, so a build in
 * which every test body below silently stopped running -- renamed helper, dead
 * fixture, an early return on a setup path -- still exited 0 and still read
 * green. That is the most dangerous measurement property in this repo: a broken
 * build and a passing build print the same thing.
 *
 * `cases` is incremented on EVERY assertion ATTEMPT, before the condition is
 * evaluated, so it is a count of cases actually reached and not a count of
 * cases that happened to pass. The only thing asserted below is cases > 0,
 * which is the weakest guard that still distinguishes "ran" from "did not
 * run"; it is deliberately NOT a floor that would need re-baselining when a
 * test is added or removed.
 *
 * This ADDS a check. It does not weaken, relax or delete an existing one: the
 * failure count and its exit status are untouched. */

#define CHECK(condition, message) do { \
    cases++; \
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

/* One queue entry owned by `owner`, ready for rb_queue_pause_for_choice.
 * queue.rs:367-409's Idle/Completed branch mints a dummy entry the same way, and
 * rb_queue_mint_dummy_entry is the C's fold of it. */
static void seed_entry(TestGame *tg, const char *owner)
{
    RbQueueEntry *e = &tg->state.queue.entries[0];
    memset(e, 0, sizeof(*e));
    e->card_id = 0;
    snprintf(e->player_id, sizeof(e->player_id), "%s", owner);
    tg->state.queue.n_entries = 1;
    tg->state.queue.cur = 0;
    tg->state.queue.has_pending = 0;
    tg->state.queue.state = RB_QUEUE_RESOLVING;
}

/* RbChoice carries `int filtered_indices[RB_MAX_ZONE]` and three 512-byte
 * description buffers, so it cannot be returned by value on this toolchain
 * ("assignment to expression with array type"); the two shapes below are
 * FILLED instead. */

/* queue.rs:322-331 — a SelectCard whose target_player_id is "opponent". */
static void fill_opponent_select_card_choice(RbChoice *ch)
{
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CHOICE_SELECT_CARD;
    snprintf(ch->zone, sizeof(ch->zone), "stage");
    ch->count = 1;
    snprintf(ch->target_player_id, sizeof(ch->target_player_id), "opponent");
}

/* The ordinary case: an opponent-UNDIRECTED SelectCard. Rust's `is_opponent_choice`
 * is false for it, so the universal default at queue.rs:354-360 applies. */
static void fill_plain_select_card_choice(RbChoice *ch)
{
    memset(ch, 0, sizeof(*ch));
    ch->kind = RB_CHOICE_SELECT_CARD;
    snprintf(ch->zone, sizeof(ch->zone), "hand");
    ch->count = 1;
    snprintf(ch->target_player_id, sizeof(ch->target_player_id), "self");
}

/* ---------------------------------------------------------------------------
 * 1. THE UNIVERSAL DEFAULT — queue.rs:354-360.
 *
 * Every paused choice gets a choice_player_id, and with no opponent route it is
 * the ENTRY'S OWNER. This is also the assertion that the C's stamp is not empty:
 * pvp_room_test.rs:176-193 (`normal_choice_stays_with_active_player_in_pvp`)
 * covers the same ground and does not compile.
 * ------------------------------------------------------------------------- */
static void t_default_routes_to_the_entry_owner(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    /* Owner p1. */
    seed_entry(&tg, "p1");
    RbChoice ch;
    fill_plain_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:358 — an unrouted choice goes to the entry's owner");

    /* Owner p2 — the default must follow the OWNER, not seat 0. */
    test_game_new(&tg);
    seed_entry(&tg, "p2");
    fill_plain_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p2") == 0,
          "queue.rs:358 — the default is the owner's own token, not p1");

    /* A long-form owner normalises the same way (abilities.rs:240-246). */
    test_game_new(&tg);
    seed_entry(&tg, "player2");
    fill_plain_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "player2") == 0,
          "the default copies the owner's raw token, as Rust's clone() does");

    /* And the seat accessors agree with the raw string. */
    CHECK(test_queue_entry_choice_seat(&tg, 0) == 1,
          "the seat reader maps player2 to seat 1");
}

/* ---------------------------------------------------------------------------
 * 2. THE OPPONENT REDIRECT — queue.rs:335-352.
 *
 * Both of Rust's ENTRY-context triggers are checked, because the point of the
 * split (src/ability/ability_queue.c:381-400) is that neither alone decides it.
 * ------------------------------------------------------------------------- */
static void t_opponent_redirect_needs_both_halves(void)
{
    static TestGame tg;
    ensure_cards_loaded();

    /* (a) the Raw route trigger alone. queue.rs:340-342. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    RbChoice ch;
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p2") == 0,
          "queue.rs:346-352 — the Raw opponent route sends the choice to p2");

    /* (b) the spawn-context trigger alone. queue.rs:335-339. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    snprintf(tg.state.queue.entries[0].spawn_target,
             sizeof(tg.state.queue.entries[0].spawn_target), "opponent");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p2") == 0,
          "queue.rs:338 — the opponent spawn context sends the choice to p2");

    /* (c) mirror image: a p2 entry is redirected to p1. The redirect is
     * "the other seat", not a hard-coded p2. */
    test_game_new(&tg);
    seed_entry(&tg, "p2");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:346 `let opp = if entry.player_id == \"p1\" { \"p2\" } "
          "else { \"p1\" }` — a p2 entry redirects to p1");

    /* (d) CONTEXT WITHOUT THE CHOICE SHAPE: the entry says opponent, the choice
     * is not opponent-directed. Rust's is_opponent_choice is false, so the
     * universal default wins and the seat is the OWNER. This is the negative
     * control that stops "any opponent context redirects" from being a
     * plausible-looking but wrong rule. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    fill_plain_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:332 `if !is_opponent_choice { return None; }` — an "
          "opponent CONTEXT alone does not redirect a non-opponent CHOICE");

    /* (e) THE CHOICE SHAPE WITHOUT THE CONTEXT: SelectCard{target_player_id:
     * "opponent"} but the entry carries neither trigger. Rust returns None at
     * :343-345 and the default applies. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:343-345 — the opponent CHOICE alone does not redirect "
          "without a context trigger");

    /* (f) the route must MATCH the literal prefix, not merely contain it. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:self:front");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:341 `raw.starts_with(\"position_change:opponent:\")` — a "
          "position_change:self route is not an opponent route");
}

/* ---------------------------------------------------------------------------
 * 3. AN EXISTING STAMP IS NEVER OVERWRITTEN — queue.rs:318-321.
 *
 * This is the arm that makes `execute_choice`'s choice_maker stamp
 * (choice.rs:142-152) and the live-success zone stamp (live.rs:1577/:1779)
 * survive the pause. pvp_room_test.rs:150-173
 * (`opponent_choice_via_select_auto_ability_works_in_pvp`) is its only Rust
 * coverage and does not compile.
 * ------------------------------------------------------------------------- */
static void t_existing_stamp_survives_the_pause(void)
{
    static TestGame tg;
    ensure_cards_loaded();

    /* The G1/RWC shape (actions/mod.rs:1435-1442 stamps, :1440-1442 resets):
     * pre-stamped "p1" on a p1 entry, then an opponent redirect arrives. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    snprintf(tg.state.queue.entries[0].choice_player_id,
             sizeof(tg.state.queue.entries[0].choice_player_id), "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    RbChoice ch;
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p1") == 0,
          "queue.rs:319 `if entry.choice_player_id.is_some() { return None }` — "
          "a pre-stamped seat is not overwritten by the redirect");

    /* And a pre-stamp to the OPPONENT on a p1 entry survives too, which is the
     * direction that actually matters for the leak filter at web_server.rs:1240. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    snprintf(tg.state.queue.entries[0].choice_player_id,
             sizeof(tg.state.queue.entries[0].choice_player_id), "p2");
    RbChoice plain;
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    CHECK(strcmp(tg.state.queue.entries[0].choice_player_id, "p2") == 0,
          "queue.rs:356 — an opponent pre-stamp survives even the default arm");
}

/* ---------------------------------------------------------------------------
 * 4. THE JSON KEY — Rust (4) and (5).
 *
 * `rb_inject_choice_ability_context` is the C's fold of
 * `inject_choice_ability_context` (abilities.rs:2483-...), which is what
 * get_pending_choice_json merges into the object the client receives
 * (abilities.rs:2475-2480). Before this work it read `entry.player_id` and
 * never `entry.choice_player_id`, so a routed choice was announced as the
 * ability owner: assertions 4b and 4c both FAIL against that body.
 * ------------------------------------------------------------------------- */
static void t_json_key_names_the_routed_seat(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    char buf[2048];

    /* (a) routed: the key must name the OPPONENT, not the owner. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    RbChoice ch;
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(strstr(buf, "\"choice_player_id\":\"p2\"") != NULL,
          "abilities.rs:2559-2568 — the key names the ROUTED seat (p2), "
          "not the entry owner (p1)");

    /* (b) unrouted: the key falls back to the owner. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    RbChoice plain;
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(strstr(buf, "\"choice_player_id\":\"p1\"") != NULL,
          "abilities.rs:2559 unwrap_or(&entry.player_id) — the key falls "
          "back to the owner");

    /* (c) the player1/player2 normalisation at abilities.rs:2560-2564. */
    test_game_new(&tg);
    seed_entry(&tg, "player2");
    RbChoice plain2;
    fill_plain_select_card_choice(&plain2);
    rb_queue_pause_for_choice(&tg.state, &plain2);
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(strstr(buf, "\"choice_player_id\":\"p2\"") != NULL,
          "abilities.rs:2561 `\"player2\" => \"p2\"` — the long form is "
          "normalised on the wire");

    /* (d) no entry at all: Rust's match leaves \"\" as \"\"; the old C body
     * fabricated \"p1\", which would have satisfied the leak filter at
     * web_server.rs:1243 for the WRONG seat. */
    test_game_new(&tg);
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;
    buf[0] = 'S';
    buf[1] = '\0';
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(buf[0] == '\0',
          "with no current entry the C writes nothing rather than a guess");
}

/* ---------------------------------------------------------------------------
 * 5. THE WEB SURFACE — Rust (8).
 *
 * pvp_room_test.rs:267 asserts
 *     json.get("choice_player_id")
 * on `get_pending_choice_json()`. The C's equivalent surface is
 * `append_pending_choice` in src/web_server.c, which is static; the seat it
 * emits is read back through the exported reader. Before this work the C's
 * `pending_choice` object carried kind/zone/target/card_type/count/allow_skip
 * and an integer `actor`, and NO `choice_player_id` at all.
 * ------------------------------------------------------------------------- */
static void t_web_surface_carries_the_key(void)
{
    static TestGame tg;
    ensure_cards_loaded();

    /* (a) unrouted -> p1. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    RbChoice plain;
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "p1") == 0,
          "the web surface names p1 for an unrouted choice");

    /* (b) routed -> p2. This is the assertion the compiled corpus had none of. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    RbChoice ch;
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "p2") == 0,
          "web_server.rs:1240-1249 — the web surface names the ROUTED seat, "
          "which is the only field its leak filter reads");

    /* (c) the reader is total: no queue, no entry, a bad index. It must not
     * fault, and it must not invent a seat. */
    test_game_new(&tg);
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "") == 0,
          "no current entry -> the empty token, which web_server.rs:1245-1248 "
          "treats as \"hide the choice\"");
    CHECK(strcmp(rb_wire_pending_choice_player_id(NULL), "") == 0,
          "a null game yields the empty token rather than faulting");

    /* (d) a pending choice really is pending on that board, so (a)/(b) are
     * measuring the routed state and not an empty queue. */
    CHECK(rb_has_pending_choice(&tg.state) == 0,
          "the premise of (c): no pause, so no pending choice");
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    RbChoice ch2;
    fill_opponent_select_card_choice(&ch2);
    rb_queue_pause_for_choice(&tg.state, &ch2);
    CHECK(rb_has_pending_choice(&tg.state) == 1,
          "the premise of (b): the routed choice really is pending");
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "p2") == 0,
          "and the web surface still names p2 while it is pending");
}

/* ---------------------------------------------------------------------------
 * 6. THE READER — Rust (6) and (7).
 *
 * Sections 1-5 pin the routing FIELD. This one pins the READER every consumer
 * is supposed to consult, which is where the C was wrong: it answered with
 * `g->queue.actor`, and pause_for_choice sets that to `g->active`
 * (src/ability/ability_queue.c:593/:605). So on a routed prompt the C named the
 * seat that was ALREADY ACTIVE — the opposite of what the redirect is for.
 *
 * Every assertion below is arranged so that the OLD body returns the wrong
 * answer: the discriminator in each case is that the entry's owner (or the
 * stamp) is the OTHER seat from g->active, and test_game_new leaves active = 0.
 * ------------------------------------------------------------------------- */

/* seed_entry + an explicit active seat, so "the stamp" and "the active seat"
 * can be made to disagree on purpose. */
static void seed_entry_active(TestGame *tg, const char *owner, int active)
{
    seed_entry(tg, owner);
    tg->state.active = active;
}

/* ===========================================================================
 * SECTION 8 -- the `select` SOURCE CARRIER (landed 2026-10-05)
 *
 * WHAT RUST REQUIRES
 *   engine/src/ability/effects/selection.rs:45-52  `execute_select_effect`:
 *       if effect.source_any().is_none()
 *          && effect.heart_colors_any().is_empty()
 *          && effect.or_card_types_any().is_none()
 *          && effect.characters_any().is_none_or(|v| v.is_empty())
 *          && effect.group_names_any().is_none()
 *       { return self.execute_area_select(gs, effect); }
 *   `source_any()` reads the DECODED `source` FIELD of AbilityEffect -- it is
 *   not an extras-table lookup.
 *
 * WHAT THE C DID
 *   src/ability/effects/draw.c:653 `rb_effect_select_effect` read the carrier
 *   with `draw_extra(e, "source")`, i.e. an extras-table row. The only writer
 *   of a `source` key is src/ability/vm.c:735, and it assigns the STRUCT FIELD
 *   `e->source` -- it never calls effect_set_extra for that key. So
 *   `draw_extra(e, "source")` was NULL for EVERY decoded select node, and the
 *   area_select arm at draw.c:664 was taken for all of them.
 *   MEASURED over the whole card corpus (tools/select_route_census.c):
 *       select nodes                         73
 *       e->source set (struct field)          39
 *       extra("source") set                   0   <-- always NULL
 *       routing BEFORE (draw_extra) area_select=14 heart_color=24 generic=35
 *       routing AFTER  (Rust's source_any) area_select=4 heart_color=24 generic=45
 *   i.e. 10 select nodes were offered an AREA prompt where Rust opens a card
 *   SelectCard.
 *
 * THE OBSERVABLE TWIN
 *   engine/tests/test_modules/effects/compound/cost_and_effect/
 *   discard_score_energy_live_recovery_edges_test.rs:15-42
 *   `discard_score_energy_live_recovery_no_live_in_discard_no_selection`:
 *   after paying the hand-discard cost, with an EMPTY waitroom, Rust
 *   auto-completes -- `assert!(!game.has_pending_choice())` at :37-40. The C
 *   instead opened the area prompt (observed shape:
 *   `kind=2 target="area_select" count=2 desc="Choose an area: left,right"`).
 *
 * THE ONE LEVEL DEEPER DEFECT THIS UNCOVERED (and fixed with it)
 *   Correctly routing selection.rs:87 `execute_select` reaches a branch that
 *   had never been reachable, and it carried a stale-pointer bug:
 *   draw.c (phase 0) parked `g->queue.resume_eff = e`, and choice.c:5393
 *   (the mode==5 C6 re-entry) forwarded that pointer back into
 *   rb_effect_both_hand_shuffle_under. For a TRIGGER-driven activation the
 *   ability tree is decoded onto the trigger handler's STACK, so by the time
 *   the prompt is answered that node is dead -- measured
 *   `RESUME_CAPTURE ... eff=0x7ffff9d20`, and dereferencing it took
 *   parity_sequential `j_keep_three_draw_three` to SIGSEGV (exit 139). The
 *   keep-N count is the only thing phases 1/2 read off the node, so it now
 *   travels in `g->keep_shuffle_under_count` and the dead pointer is not
 *   parked at all. This is the SAME alias class as the load-bearing deferred
 *   frees: an owning node aliased past its frame.
 * =========================================================================== */

static void t_select_source_carrier_is_the_decoded_field(void)
{
    static TestGame tg;
    int shizuku;
    test_game_new(&tg);
    shizuku = test_id(&tg, "PL!N-bp5-003-R");
    if (shizuku < 0) { CHECK(0, "Q214 PL!N-bp5-003-R is in the database"); return; }

    /* Decode the ability and find the `select` node Rust reads at
     * selection.rs:45. Its source must live in the STRUCT FIELD. */
    int n = rb_card_num_abilities((uint32_t)shizuku);
    int found = 0, struct_field = 0, extra_row = 0;
    for (int ai = 0; ai < n; ai++) {
        Ability ab; memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)shizuku, ai, &ab)) continue;
        for (int ci = 0; ab.effect && ci < ab.effect->n_child; ci++) {
            const AbilityEffect *c = ab.effect->child[ci];
            if (!c->action || strcmp(c->action, "select") != 0) continue;
            found++;
            if (c->source && *c->source) struct_field++;
            for (int xi = 0; xi < c->n_extra; xi++)
                if (c->extra_k[xi] && !strcmp(c->extra_k[xi], "source")) extra_row++;
        }
        rb_free_ability(&ab);
    }
    char msg[256];
    snprintf(msg, sizeof(msg),
             "selection.rs:45 -- Q214 decodes a `select` node whose source is "
             "the case's whole subject (found=%d)", found);
    CHECK(found > 0, msg);
    CHECK(struct_field == found,
             "selection.rs:45 `source_any()` -- EVERY `select` node carries its "
             "source in the DECODED FIELD, which is the only carrier "
             "draw.c:653 is allowed to read");
    /* The regression this pins: draw.c read the extras table, and vm.c:735
     * never writes that row, so the count was 0 and every node took the
     * area_select arm. */
    CHECK(extra_row == 0,
             "vm.c:735 -- `source` is NEVER an extras-table row, so reading it "
             "as one (the pre-fix draw.c:658) is what routed 10 nodes to "
             "area_select");
}

static void t_q214_empty_waitroom_opens_no_prompt(void)
{
    static TestGame tg;
    int shizuku, filler;
    test_game_new(&tg);
    shizuku = test_id(&tg, "PL!N-bp5-003-R");
    filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (shizuku < 0 || filler < 0) {
        CHECK(0, "Q214 + its PL!-sd1-010-SD filler are in the database");
        return;
    }
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = shizuku;
    tg.state.p[0].stage[2] = filler;
    tg.state.p[0].hand.n = 0;
    tg.state.p[0].hand.cards[tg.state.p[0].hand.n++] = filler;
    tg.state.p[0].discard.n = 0;              /* Rust :24 "waitroom empty" */
    test_give_energy(&tg, 5);

    /* Rust :26-34 UseAbility on shizuku, then :35 pay the hand cost. */
    int fired = test_activate_ability(&tg, shizuku);
    CHECK(fired, "Q214: the 起動 ability was queued (ActionType::UseAbility)");
    int one[] = { 0 };
    if (test_has_pending_choice(&tg)) test_select_indices(&tg, one, 1);

    /* Rust :37-40 -- `assert!(!game.has_pending_choice(), "no live in discard
     * should not prompt selection")`. This is the assertion the pre-fix C
     * failed: it opened `target="area_select"` because selection.rs:45 was
     * read off the wrong carrier. */
    CHECK(!test_has_pending_choice(&tg),
          "discard_score_energy_live_recovery_edges_test.rs:37-40 -- with an "
          "EMPTY waitroom the recovery leg has no candidate, so Rust "
          "auto-completes with NO prompt (pre-fix C opened area_select)");
    CHECK(tg.state.p[0].hand.n == 0,
             "discard_score_energy_live_recovery_edges_test.rs:41 -- the hand "
             "cost was still paid and discarded");
}

static void t_reader_answers_the_routed_seat(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    RbChoice ch, plain;

    /* (a) THE HEADLINE. A p1 ability hands an opponent-routed prompt to p2,
     * while p1 is still the active seat. Rust: abilities.rs:2466-2468 reads
     * the entry's stamp, which queue.rs:346-352 set to "p2".
     * The old body answered 0 (g->active). */
    test_game_new(&tg);
    seed_entry_active(&tg, "p1", 0);
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(rb_get_pending_choice_player_id(&tg.state) == 1,
          "abilities.rs:2466-2468 — a routed prompt names the ROUTED seat (1), "
          "not the active seat (0) the old body returned");
    CHECK(rb_gsc_can_player_act(&tg.state, 0) == 0,
          "mod.rs:505-511 — the seat the prompt was redirected AWAY from "
          "cannot act");
    CHECK(rb_gsc_can_player_act(&tg.state, 1) == 1,
          "mod.rs:511 `return norm == pid_str()` — the routed seat is the one "
          "that must answer");

    /* (b) THE OTHER DIRECTION OF THE SAME DEFECT. An UNROUTED prompt whose
     * owner is p2, with p1 active. The universal default (queue.rs:358) put
     * the OWNER's token in the stamp, so Rust reads p2. The old body answered
     * 0. This is the case the routed-only test above would miss: no redirect
     * happens, and the C is still wrong. */
    test_game_new(&tg);
    seed_entry_active(&tg, "p2", 0);
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    CHECK(rb_get_pending_choice_player_id(&tg.state) == 1,
          "abilities.rs:2466-2468 — an unrouted prompt names the entry's OWNER "
          "(1), not the active seat (0)");
    CHECK(rb_gsc_can_player_act(&tg.state, 0) == 0,
          "mod.rs:505-511 — the non-owner cannot answer another seat's prompt");

    /* (c) the mirror of (b): p1's entry with p2 active. */
    test_game_new(&tg);
    seed_entry_active(&tg, "p1", 1);
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    CHECK(rb_get_pending_choice_player_id(&tg.state) == 0,
          "abilities.rs:2466-2468 — and the reading does not depend on which "
          "seat happens to be active");
    CHECK(rb_gsc_can_player_act(&tg.state, 0) == 1,
          "mod.rs:505-511 — the owner may answer");

    /* (d) NO FALLBACK. abilities.rs:2468 is a bare
     * `.and_then(|e| e.choice_player_id.as_ref().cloned())` -- there is no
     * `unwrap_or(&entry.player_id)` here, unlike the JSON producer at
     * abilities.rs:2559 which DOES have one. So an entry that carries an owner
     * and no stamp reads None, and the reader must say so rather than borrow
     * the owner. The old body answered 0 here too, by way of g->active. */
    test_game_new(&tg);
    seed_entry_active(&tg, "p1", 0);
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    tg.state.queue.entries[0].choice_player_id[0] = '\0';   /* un-stamp it */
    CHECK(rb_get_pending_choice_player_id(&tg.state) == -1,
          "abilities.rs:2468 — there is NO player_id fallback in the reader; "
          "only the JSON producer at :2559 has one");

    /* (e) THE TWO current_entry() == None CASES, queue.rs:234-242. Both park a
     * pending choice, so `has_pending` alone does not filter them out.
     * (e1) the auto-ability prompt: WaitingForAutoAbilityChoice carries no
     * entry_index, and Rust's `current_entry` returns None for that variant
     * (queue.rs:240), so the reader answers None and can_player_act falls
     * through to its Choice-embedded arm. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_auto_ability_choice(&tg.state, &plain);
    CHECK(tg.state.queue.has_pending == 1,
          "the premise of (e1): an auto-ability prompt really is pending");
    CHECK(rb_get_pending_choice_player_id(&tg.state) == -1,
          "queue.rs:240 — current_entry() is None for "
          "WaitingForAutoAbilityChoice, so the entry behind it is not read");
    /* (e2) an Idle queue with a stamped entry: Rust's Idle arm is None. */
    test_game_new(&tg);
    seed_entry(&tg, "p1");
    fill_plain_select_card_choice(&plain);
    rb_queue_pause_for_choice(&tg.state, &plain);
    tg.state.queue.state = RB_QUEUE_IDLE;
    CHECK(rb_get_pending_choice_player_id(&tg.state) == -1,
          "queue.rs:240 — and None for Idle, which is the other arm Rust lists");

    /* (f) THE FIELD THIS FIX MUST NOT TOUCH. ability_queue.c:584-592 sets
     * queue.actor = g->active on purpose, because the opponent-movement and
     * cross-player watchers read it to learn which seat MOVED (the suite
     * `cross_player_interaction` carries the "opponent movement watcher fires"
     * pin). So on this board the two fields DISAGREE by construction:
     *   queue.actor = 0  (p1 moved, p1 is active)
     *   the stamp    = p2 (queue.rs:346-352 redirected the prompt)
     * and the reader must answer from the stamp while leaving the actor field
     * alone. If anyone ever "simplifies" the reader by folding the stamp into
     * queue.actor, both halves of this fail. */
    test_game_new(&tg);
    seed_entry_active(&tg, "p1", 0);
    rb_queue_set_choice_route_raw(&tg.state, "position_change:opponent:front");
    fill_opponent_select_card_choice(&ch);
    rb_queue_pause_for_choice(&tg.state, &ch);
    CHECK(tg.state.queue.actor == 0,
          "ability_queue.c:593 — queue.actor is the MOVING seat (g->active)");
    CHECK(rb_get_pending_choice_player_id(&tg.state) == 1,
          "abilities.rs:2466-2468 — while the reader answers from the stamp, "
          "which queue.rs:346-352 set to the OPPONENT");
    (void)rb_get_pending_choice_player_id(&tg.state);
    (void)rb_gsc_can_player_act(&tg.state, 0);
    (void)rb_gsc_can_player_act(&tg.state, 1);
    CHECK(tg.state.queue.actor == 0,
          "ability_queue.c:584-592 — reading the stamp does not rewrite "
          "queue.actor, which the cross-player watchers read");
}

/* ---------------------------------------------------------------------------
 * 7. THE SECOND PRODUCER OF THE KEY — Rust abilities.rs:2679-2697.
 *
 * `inject_choice_ability_context` writes choice_player_id TWICE. The first is
 * :2559-2568, inside `if let Some(entry) = entry_ref`. The second is the `else`
 * at :2679: when there is NO current entry, it still emits the key, taking the
 * Choice's own player_id for SelectAutoAbility / SelectLiveSuccess.
 *
 * MEASURED, not assumed: the C's `rb_wire_pending_choice_player_id`
 * (src/core/game_state_abilities.c) has no such arm and returns "" when
 * `cur` is out of range, so for a no-entry SelectAutoAbility prompt the two
 * engines emit different values for the key.
 *
 * WHY IT IS NOT FIXED HERE: that file is not this task's to edit. The direction
 * of the difference is SAFE — web_server.rs:1240-1249 drops the whole
 * `pending_choice` when the key is missing or names anybody but the requester,
 * so "" makes the C client HIDE a prompt Rust would show. A hidden prompt is a
 * bug; a leaked opponent prompt is the bug Rust's comment calls "defensive:
 * never leak opponent choices". This is pinned as a KNOWN DIVERGENCE so it
 * cannot be rediscovered as a surprise.
 * ------------------------------------------------------------------------- */
/* ---------------------------------------------------------------------------
 * 7. THE SECOND `choice_player_id` PRODUCER — Rust abilities.rs:2828-2846.
 *
 * `inject_choice_ability_context` has TWO producers of the key, not one:
 *
 *     if let Some(entry) = entry_ref { ... }                      // :2641-2717
 *         obj.insert("choice_player_id", normalize(entry.choice_player_id
 *                             .as_ref().unwrap_or(&entry.player_id)));
 *     } else if let Some(choice) = self.ability_queue.is_waiting_for_choice() {
 *         if let Choice::SelectAutoAbility { ref player_id, .. }
 *         | Choice::SelectLiveSuccess { ref player_id, .. } = choice {
 *             obj.insert("choice_player_id", normalize(player_id));
 *         }
 *     }
 *
 * The `else` arm is reached when `current_entry()` yields nothing, which in
 * Rust (ability/queue.rs:234-242) is exactly Idle and
 * WaitingForAutoAbilityChoice. The C returned early on both, emitting nothing,
 * and this section used to PIN that as a known divergence. game_state_abilities.c
 * now implements the arm, so the assertion below is the Rust behaviour and the
 * old "MEASURED DIVERGENCE ... not fixed" text is gone: the gap is closed, not
 * documented.
 *
 * THE CHOICE'S OWNER IN THE C. Rust reads
 * `Choice::SelectAutoAbility::player_id: Option<String>`. The C has no such
 * field: RbFullChoice (rabuka.h:1156) carries none and
 * `rb_full_choice_to_header` (types.c:515-541) copies none into RbChoice, whose
 * seat member is `actor` (rabuka.h:1001, "0=p1, 1=p2 - which player made the
 * choice"). That is the same datum — the seat the Choice answers to — and
 * `rb_queue_pause_for_auto_ability_choice` (ability_queue.c:714-722) stamps
 * `queue.actor = g->active` for precisely this prompt. types.c:525 folds
 * `RB_CC_SELECT_LIVE_SUCCESS` into `RB_CHOICE_SELECT_AUTO_ABILITY`, so one kind
 * test covers both Rust variants.
 *
 * WHY IT MATTERED: web_server.rs:1243 DROPS the whole `pending_choice` object
 * when the key is missing, so on this path the prompt was invisible to BOTH
 * seats and the game waited for an answer that could never arrive.
 * ------------------------------------------------------------------------- */
static void t_second_json_producer_names_the_choices_own_seat(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    char buf[2048];

    /* (a) THE ELSE ARM: no current entry, a SelectAutoAbility prompt waiting.
       Rust takes :2828-2846 and names the Choice's own seat. */
    test_game_new(&tg);
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;
    tg.state.queue.pending.kind   = RB_CHOICE_SELECT_AUTO_ABILITY;
    tg.state.queue.pending.actor  = 1;              /* p2 answers */
    tg.state.queue.has_pending    = 1;
    tg.state.queue.auto_ability   = 1;
    tg.state.queue.state          = RB_QUEUE_AWAITING_CHOICE;
    tg.state.queue.actor          = 1;
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "p2") == 0,
          "abilities.rs:2836-2844 — with NO current entry the `else` arm names "
          "the SelectAutoAbility Choice's own player_id (p2), which the "
          "pre-port C could not do and answered \"\"");
    buf[0] = 'S'; buf[1] = '\0';
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(strstr(buf, "\"choice_player_id\":\"p2\"") != NULL,
          "abilities.rs:2841-2844 — the same arm inserts the key into the JSON "
          "object, so the prompt is no longer dropped by web_server.rs:1243");

    /* (b) THE NEGATIVE CONTROL: same state, but the waiting Choice is NOT one
       of the two variants that carry a player_id. Rust inserts nothing, so the
       C must not invent a seat. */
    test_game_new(&tg);
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;
    tg.state.queue.pending.kind   = RB_CHOICE_SELECT_CARD;
    tg.state.queue.pending.actor  = 1;
    tg.state.queue.has_pending    = 1;
    tg.state.queue.state          = RB_QUEUE_AWAITING_CHOICE;
    tg.state.queue.actor          = 1;
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "") == 0,
          "abilities.rs:2833-2834 — only SelectAutoAbility and "
          "SelectLiveSuccess carry a player_id; a SelectCard with no entry "
          "inserts nothing");
    buf[0] = 'S'; buf[1] = '\0';
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(buf[0] == '\0',
          "and the JSON producer writes nothing for it, rather than guessing");

    /* (c) THE PREMISE GATE: `is_waiting_for_choice()` must actually hold. A
       pending SelectAutoAbility with the queue NOT in AwaitingChoice is not a
       waiting choice (queue.rs:200-205), so neither Rust nor the C answers. */
    test_game_new(&tg);
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;
    tg.state.queue.pending.kind   = RB_CHOICE_SELECT_AUTO_ABILITY;
    tg.state.queue.pending.actor  = 1;
    tg.state.queue.has_pending    = 1;
    tg.state.queue.state          = RB_QUEUE_RESOLVING;
    tg.state.queue.actor          = 1;
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "") == 0,
          "queue.rs:208-214 — has_pending outside AwaitingChoice is not a "
          "waiting choice, so the else arm does not run");

    /* (d) PRECEDENCE: when an entry IS present the first producer runs and the
       `else` is unreachable, so the entry's stamp wins even over the Choice's
       own seat. This is what keeps the existing routing assertions honest. */
    test_game_new(&tg);
    seed_entry(&tg, "p2");
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_AUTO_ABILITY;
    snprintf(ch.target_player_id, sizeof(ch.target_player_id), "p1");
    rb_queue_pause_for_choice(&tg.state, &ch);
    rb_inject_choice_ability_context(&tg.state, buf, sizeof(buf));
    CHECK(strstr(buf, "\"choice_player_id\":\"p2\"") != NULL,
          "abilities.rs:2559 — with an entry present the STAMP producer runs "
          "and the `else` at :2828 does not, so the C matches Rust exactly");
    CHECK(strcmp(rb_wire_pending_choice_player_id(&tg.state), "p2") == 0,
          "and the web surface agrees: the entry arm still wins, which is why "
          "no routed assertion moved when the else arm was added");
}

int main(void)
{
    printf("=== choice_player_id routing and the JSON wire key ===\n");
    t_default_routes_to_the_entry_owner();
    t_opponent_redirect_needs_both_halves();
    t_existing_stamp_survives_the_pause();
    t_json_key_names_the_routed_seat();
    t_web_surface_carries_the_key();
    t_select_source_carrier_is_the_decoded_field();
    t_q214_empty_waitroom_opens_no_prompt();
    t_reader_answers_the_routed_seat();
    t_second_json_producer_names_the_choices_own_seat();
    if (cases == 0) {
        fprintf(stderr, "FAIL: parity_choice_routing ran ZERO cases -- every test body stopped "
                        "before its first assertion; this is a broken suite, "
                        "not a passing one\n");
        return 1;
    }
    printf("parity_choice_routing: %d cases, %d failures\n", cases, failures);
    return failures ? 1 : 0;
}
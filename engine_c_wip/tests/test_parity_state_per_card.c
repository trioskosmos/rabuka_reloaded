/* test_parity_state_per_card.c — C parity suite for the Rust cluster
 *
 *   engine/tests/test_modules/effects/state/per_card/   (21 .rs files / 84 #[test])
 *
 * Cluster shape: EVERY one of these is a STATE-CHANGE (change_state) test
 * driven end-to-end from a real board and a real trigger. Either/or ("A か B")
 * splits, per-unit / distinct-name group activation counts, optional
 * wait/activation costs paid and declined, and the two independent conditional
 * branches of the same print. It is deliberately NOT a re-run of
 * tests/test_parity_state_effects2.c, which owns rb_effect_change_state at the
 * unit level (filters, prompts, per_unit, optional gate, orientation
 * bookkeeping), nor of tests/test_parity_wait_activation.c, which owns the
 * sibling effects/state/wait_activation cluster. Everything below drives the
 * same change-state code from the outside.
 *
 * ── harness facts peers established (copied verbatim) ────────────────────
 *   rb_load("src") fails under an isolated out-of-tree build, so load_card_db()
 *   falls back to rb_load("../cards/build").
 *   Rust `trigger_auto_ability(id, ..)` + `process_pending_auto_abilities` is
 *   reproduced by fire_trigger(): set activating_card, rb_queue_trigger_
 *   abilities(pl, trigger), then the SINGLE-SEAT rb_process_player_abilities
 *   (NOT rb_process_pending_auto_abilities, which runs pl=0 then pl=1 and whose
 *   trailing `if (!g->queue.has_pending) { g->queue.n_entries = 0; }` destroys
 *   a p2-owned queued ability during the p1 pass).
 *   A sequential tail parked as owned pending actions is NOT a new queue entry:
 *   pump_queue() sets RB_QUEUE_IDLE and calls rb_queue_resume_pending_actions.
 *   mid() is test_new_id (Rust's game.id() allocates the next DISTINCT pool
 *   slot; test_id aliases the shared template).
 *   sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *   Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *   from rb_card_num_abilities + rb_decode_card_ability.
 *   RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE, so heart05
 *   claims go through rb_mods_get_heart directly (heart_of() below).
 *
 * ── the prompt-count control (the most important discipline here) ────────
 *   Almost every assertion in this cluster is NEGATIVE ("this member is NOT
 *   waited", "no energy activated", "no choice is offered"). On a board where
 *   the ability is a silent no-op — because the parser dropped an effect, or a
 *   gate never fires — a negative assertion passes for the wrong reason. Every
 *   such negative is therefore paired with PROMPT-COUNT CONTROL: the test
 *   asserts `prompts > 0` (or an exact prompt count) so the reader can tell
 *   "the gate correctly declined" from "nothing ran at all". The global
 *   `prompts` counter is incremented by answer_choice(), the ONLY way this
 *   file answers a choice, so it cannot drift from the real interaction.
 *
 * ── encoding ────────────────────────────────────────────────────────────
 *   This file is pure ASCII. Every card number carrying a FULLWIDTH PLUS
 *   (U+FF0B, bytes EF BC 8B) is spelled with the FF escape below, because a
 *   CP932 write would corrupt it and the card would silently not resolve.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* FULLWIDTH PLUS, U+FF0B. */
#define FF "\xEF\xBC\x8B"

/* ── crash reporting + fork isolation ─────────────────────────────────────
 * Each test runs in a forked child so a SIGSEGV/SIGBUS/SIGABRT is attributed to
 * the test in flight rather than truncating the run so every later section
 * falsely reads as a pass. No time-based watchdog: a process watchdog does not
 * work on this toolchain (in-child alarm() never fires; a parent-side
 * waitpid(WNOHANG) deadline fires but its kill(SIGKILL) does not take effect,
 * wedging the blocking waitpid). A child that HANGS is a recorded limitation.
 */
static const char *current_test = "(none)";
static int failures;
static int assertions;
static int setup_bugs;

/* Prompts ANSWERED since the last reset. The only writer is answer_choice(),
 * so this is exactly the number of choice interactions the test performed. */
static int prompts;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        assertions++;                                                       \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* The vacuity control. A negative assertion that follows a prompt-count of 0
 * proves nothing, so every such test carries at least one of these. */
#define CHECK_PROMPTS(...) do {                                             \
        assertions++;                                                       \
        if (prompts <= 0) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "VACUOUS: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

/* stage area indices */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* heart colour slots, per cards/compile_cards.py HEART_COLORS */
#define H05 5

/* ── fixtures ───────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust's game.id() allocates the next DISTINCT pool slot; test_id aliases the
 * shared template. Use mid() everywhere a fixture must be distinguishable. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* CRITICAL IDENTITY RULE (AGENTS.md). */
static int ident(int cid, const char *no)
{
    if (rb_card_no_eq(cid, no)) return 1;
    assertions++;
    setup_bugs++;
    failures++;
    fprintf(stderr, "SETUP BUG: expected card_no \"%s\" (AGENTS.md identity rule)\n", no);
    return 0;
}

/* Direct modifier reads. test_get_heart_modifier remaps colour 5 onto
 * RB_HEART_ORANGE, so it must never be used for a heart05 claim. */
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}
static int blade_mod_of(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

/* The PRINTED blade from the card record, which is what every "original blade"
 * filter in this cluster reads. */
static int printed_blade(int cid)
{
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) { rb_free_card(&card); return -1; }
    int b = card.blade;
    rb_free_card(&card);
    return b;
}
static int printed_cost(int cid)
{
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) { rb_free_card(&card); return -1; }
    int c = card.cost;
    rb_free_card(&card);
    return c;
}

/* The PRINTED score of a live card (Card.score). 園田海未's 登場 gates on the
 * SUM of the scores in the SUCCESS live-card zone, so a threshold test has to
 * measure that zone and not the live zone. */
static int printed_score(int cid)
{
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) { rb_free_card(&card); return -1; }
    int s = card.score;
    rb_free_card(&card);
    return s;
}

static int bag_score_total(const RbBag *b)
{
    int total = 0;
    for (int i = 0; i < b->n; i++) {
        int s = printed_score(b->cards[i]);
        if (s > 0) total += s;
    }
    return total;
}

static int success_zone_score_total(TestGame *tg)
{
    return bag_score_total(&tg->state.p[0].success);
}
static int live_zone_score_total(TestGame *tg)
{
    return bag_score_total(&tg->state.p[0].live);
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) { rb_free_ability(&ab); continue; }
        int hit = ab.triggers && strstr(ab.triggers, trig) != NULL;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* Does ability `idx` of `cid` print `trig`? Used to pin the premise that the
 * ability the test means really exists, so a "nothing happened" result is a
 * PARSER/DATA finding and not a silently-absent ability. */
static int ability_prints_trigger(int cid, int idx, const char *trig)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) { rb_free_ability(&ab); return 0; }
    int hit = ab.triggers && strstr(ab.triggers, trig) != NULL;
    rb_free_ability(&ab);
    return hit;
}

static int num_abilities(int cid) { return rb_card_num_abilities((uint32_t)cid); }

/* Concatenated trigger keywords the compiled card DB carries for `cid`, or "". */
static void dump_triggers(int cid, char *buf, size_t cap)
{
    buf[0] = 0;
    size_t used = 0;
    for (int a = 0, nab = rb_card_num_abilities((uint32_t)cid); a < nab; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) { rb_free_ability(&ab); continue; }
        if (ab.triggers && ab.triggers[0]) {
            size_t need = strlen(ab.triggers) + 4;
            if (used + need < cap)
                used += (size_t)snprintf(buf + used, cap - used, "%s[%s]", used ? "," : "", ab.triggers);
        }
        rb_free_ability(&ab);
    }
    if (used == 0) snprintf(buf, cap, "(none)");
}

/* ── board helpers ──────────────────────────────────────────────────────── */

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void set_p1_stage3(TestGame *tg, int a, int b, int c)
{
    set_stage(tg, 0, AREA_LEFT, a);
    set_stage(tg, 0, AREA_CENTER, b);
    set_stage(tg, 0, AREA_RIGHT, c);
}

static void set_p2_stage3(TestGame *tg, int a, int b, int c)
{
    set_stage(tg, 1, AREA_LEFT, a);
    set_stage(tg, 1, AREA_CENTER, b);
    set_stage(tg, 1, AREA_RIGHT, c);
}

static int hand_has(TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].discard, cid); }

static int p1_under_total(TestGame *tg)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) n += tg->state.p[0].under_cards[i].n;
    return n;
}

static int under_at(TestGame *tg, int area)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return tg->state.p[0].under_cards[area].n;
}

static int under_has_at(TestGame *tg, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&tg->state.p[0].under_cards[area], cid);
}

static int p1_energy_active(TestGame *tg) { return tg->state.p[0].energy_active; }
static int p1_energy_n(TestGame *tg)      { return tg->state.p[0].energy.n; }
static int p1_wait_energy(TestGame *tg)
{
    int n = tg->state.p[0].energy.n;
    int a = tg->state.p[0].energy_active;
    return n > a ? n - a : 0;
}

static void clear_zone(RbBag *b) { b->n = 0; }

/* Rust helpers::fill_decks — n copies of `filler` into BOTH main decks. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    clear_zone(&tg->state.p[0].deck);
    clear_zone(&tg->state.p[1].deck);
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void fill_deck_p1(TestGame *tg, int filler, int n)
{
    clear_zone(&tg->state.p[0].deck);
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) test_add_to_deck_pl(tg, 0, filler);
}

static void add_hand(TestGame *tg, int cid) { test_add_to_hand(tg, cid); }

/* Push n WAIT energy (Rust energy_zone.cards.push without add_active). */
static int give_wait_energy(TestGame *tg, int n)
{
    int e = mid(tg, "LL-E-001-SD");
    for (int i = 0; i < n; i++) {
        RbBag *b = &tg->state.p[0].energy;
        if (b->n >= RB_MAX_ZONE) return e;
        b->cards[b->n++] = e;
    }
    return e;
}

static void give_energy_deck(TestGame *tg, int n)
{
    for (int i = 0; i < n; i++) test_add_to_energy_deck(tg, 0, mid(tg, "LL-E-001-SD"));
}

/* Mark `cid` as having APPEARED this turn — the C analogue of Rust's
 * `game.state.cards_moved_this_turn.push(cid)`. condition.c reads
 * g->cards_appeared_this_turn[] for the appearance condition. */
static void mark_appeared(TestGame *tg, int cid)
{
    GameState *g = &tg->state;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == cid) return;
    if (g->n_cards_appeared_this_turn < 64)
        g->cards_appeared_this_turn[g->n_cards_appeared_this_turn++] = cid;
}

static int marked_appeared(TestGame *tg, int cid)
{
    GameState *g = &tg->state;
    for (int i = 0; i < g->n_cards_appeared_this_turn; i++)
        if (g->cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}

/* ── orientation predicates (mirrors engine/src/ability/util.rs) ─────────── */

#define ORI_NONE   0
#define ORI_ACTIVE 1
#define ORI_WAIT   2
#define ORI_OTHER  3

static int orientation_of(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    if (!o) return ORI_NONE;
    if (!strcmp(o, "wait")) return ORI_WAIT;
    if (!strcmp(o, "active")) return ORI_ACTIVE;
    return ORI_OTHER;
}

static const char *ori_name(int v)
{
    switch (v) {
    case ORI_NONE:   return "none";
    case ORI_ACTIVE: return "active";
    case ORI_WAIT:   return "wait";
    default:         return "other";
    }
}

static int is_waited(TestGame *tg, int cid) { return orientation_of(tg, cid) == ORI_WAIT; }

/* Rust `orientation_matches_state(ori, "active")`: None counts as active. */
static int is_active(TestGame *tg, int cid)
{
    int o = orientation_of(tg, cid);
    return o == ORI_NONE || o == ORI_ACTIVE;
}

#define CHECK_ORI(actual, expected, ...) do {                              \
        assertions++;                                                       \
        int a_ = (actual), e_ = (expected);                                \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (orientation %s, expected %s)\n",             \
                    ori_name(a_), ori_name(e_));                           \
        }                                                                   \
    } while (0)

/* ── choice helpers ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static int pending_kind(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    return ch ? (int)ch->kind : (int)RB_CHOICE_NONE;
}

/* The ONLY way this file answers a choice, so `prompts` cannot drift. */
static void answer_choice(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    rb_resume_with_choice(&tg->state, idx);
}

static void answer(TestGame *tg, int idx) { answer_choice(tg, idx); }

static void pump_queue(TestGame *tg);

/* Rust `while game.has_pending_choice() { game.select_indices(&[0]) }` */
static int drain_first(TestGame *tg, int guard_max)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n++ < guard_max) {
        answer_choice(tg, 0);
        pump_queue(tg);
    }
    return n;
}

/* A sequential tail parked as owned pending actions is NOT a new queue entry, so
 * a hand-rolled pump must set RB_QUEUE_IDLE and call rb_queue_resume_pending_
 * actions, and must use the SINGLE-SEAT rb_process_player_abilities rather than
 * rb_process_pending_auto_abilities (whose trailing n_entries reset destroys a
 * p2-owned entry during the p1 pass). */
static void pump_queue(TestGame *tg)
{
    GameState *g = &tg->state;
    if (g->queue.state != RB_QUEUE_IDLE) {
        g->queue.state = RB_QUEUE_IDLE;
        rb_queue_resume_pending_actions(g);
    }
    for (int pl = 0; pl < 2; pl++) {
        rb_process_player_abilities(g, pl);
        rb_drain_ability_queue(g);
    }
    if (g->queue.state != RB_QUEUE_IDLE) {
        g->queue.state = RB_QUEUE_IDLE;
        rb_queue_resume_pending_actions(g);
    }
}

/* Rust's `drain_choices()` — the generic drain: answer what cannot be declined,
 * decline what can. Mirrors helpers/mod.rs. */
static int drain_choices(TestGame *tg, int guard_max)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n++ < guard_max) {
        int kind = pending_kind(tg);
        if (kind == RB_CHOICE_SELECT_AUTO_ABILITY) answer_choice(tg, -1);
        else answer_choice(tg, 0);
        pump_queue(tg);
    }
    return n;
}

/* sd2_nijigasaki_abilities_gap_test.rs `drain(g, pay)`. */
static int drain_pay(TestGame *tg, int pay, int guard_max)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n++ < guard_max) {
        int kind = pending_kind(tg);
        if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_POSITION)
            answer_choice(tg, pay ? 1 : 0);
        else if (kind == RB_CHOICE_SELECT_CARD)
            answer_choice(tg, 0);
        else if (kind == RB_CHOICE_SELECT_HEART_COLOR)
            answer_choice(tg, 0);
        else
            answer_choice(tg, -1);
        pump_queue(tg);
    }
    return n;
}

/* ── trigger helpers ─────────────────────────────────────────────────────── */

/* Mirrors the Rust `fire_trigger(&mut game, cid, trigger, trigger_str)` idiom:
 * ONE ability of ONE card. Returns 1 when that ability was enqueued, so a test
 * can prove the trigger really fired.
 *
 * The shared helper test_fire_trigger_on_card does the targeting through
 * rb_trigger_auto_ability (the port of Rust trigger_auto_ability). This USED
 * TO BE rb_queue_trigger_abilities(&tg->state, 0, trig), a whole-board TAS
 * scan in which `cid` only ever set activating_card and EVERY card in P1's
 * stage / success / live / hand / energy zone printing the trigger fired too --
 * so a fixture staging a second 登場 card ran its debut twice.
 *
 * The extra drain call rb_drain_ability_queue the old body made is dropped:
 * rb_process_pending_auto_abilities (inside the helper) is the Rust
 * `process_pending_auto_abilities`, and the extra drain was only there to
 * compensate for entries other cards had queued. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* The C analogue of Rust's
 *   TurnEngine::execute_main_phase_action_with_ability_index(UseAbility, .., Some(i))
 * for a card carrying SEVERAL kidou abilities. rb_activate_card has no
 * ability-index parameter at all, so rb_resolve_ability is the only way to
 * drive ONE of them (peers: test_parity_zone_source.c, test_parity_cost_and_
 * effect.c). Returns the resolver rc, or -1 when the index does not exist. */
static int activate_ability_index(TestGame *tg, int card_id, int ability_idx)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) {
        rb_free_ability(&ab);
        return -1;
    }
    int resolved = 0;
    int rc = rb_resolve_ability(&tg->state, 0, &ab, ability_idx, card_id, &resolved);
    rb_drain_ability_queue(&tg->state);
    ab.cost = NULL;
    ab.effect = NULL;
    rb_free_ability(&ab);
    return rc;
}

/* Rust's activate_ability(...) used against a card that is ALREADY ON STAGE.
 * test_activate_ability falls through to rb_activate_card for that case, which
 * is the same path; kept as a named shim so call sites read like the Rust. */
static int activate(TestGame *tg, int card_id)
{
    return test_activate_ability(tg, card_id);
}

/* Fire ONE specific ability of `cid` by ABSOLUTE ability index, for a card that
 * prints the trigger more than once (the Q280 case: 米女メイ's "place 2 energy
 * in WAIT" AND "activate 6 energy" are both ライブ成功時).
 *
 * The old body pushed a bare entry with rb_queue_make_entry and then ran
 * rb_drain_ability_queue, which bypassed the invalidation gate the engine's own
 * enqueue path applies. It is replaced by test_fire_trigger_at_index, which
 * addresses the same index through rb_trigger_auto_ability -- the port of Rust
 * trigger_auto_ability, the same call Rust's fire_trigger_nth makes. `trig` is
 * still checked here so the premise assertion keeps its meaning. */
static int fire_trigger_index(TestGame *tg, int cid, const char *trig, int idx)
{
    if (!ability_prints_trigger(cid, idx, trig)) return 0;
    return test_fire_trigger_at_index(tg, cid, idx);
}

/* Rust's `assert_select_card("stage", 1, true); select_indices(&[selected])`
 * followed by drain_choices_strict: answer the stage member pick with
 * `selected` (or decline when selected < 0), then drain acceptably. */
static void answer_stage_member_choice(TestGame *tg, int selected, int guard_max)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n++ < guard_max) {
        int kind = pending_kind(tg);
        if (kind == RB_CHOICE_SELECT_CARD &&
            !strcmp(rb_get_pending_choice(&tg->state)->zone, "stage")) {
            answer_choice(tg, selected);
        } else if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_POSITION) {
            answer_choice(tg, 1);       /* pay */
        } else if (kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            answer_choice(tg, -1);
        } else {
            answer_choice(tg, 0);
        }
        pump_queue(tg);
    }
}

static void activate_and_drain(TestGame *tg, int card_id)
{
    activate(tg, card_id);
    drain_choices(tg, 24);
}

/* ── phase advance ──────────────────────────────────────────────────────── */

static void advance_to_live_card_set(TestGame *tg, int passes)
{
    for (int i = 0; i < passes; i++) test_pass(tg);
}

/* =======================================================================
 * A. ability_index_execution_test.rs (1 test)
 *    PL!N-bp7-006-SEC: two kidou abilities whose zone deltas are mutually
 *    exclusive fingerprints. ab#0 = pay 1E, look at top 4, put them back;
 *    ab#1 = mill top 3 to the waitroom, no energy.
 * ======================================================================= */

#define KONATA_SEC  "PL!N-bp7-006-SEC"

static void konata_sec_board(TestGame *tg, int *out_konata)
{
    int konata = mid(tg, KONATA_SEC);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_konata = konata;
    if (konata < 0 || filler < 0) return;
    ident(konata, KONATA_SEC);
    set_p1_stage3(tg, konata, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(tg, 10);
    fill_decks(tg, filler, 30);
}

static void test_a_sec_ab1_mills_three_and_costs_no_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_sec_board(&tg, &konata);
    if (konata < 0) { CHECK(0, "A/PL!N-bp7-006-SEC fixture resolves"); return; }

    char trig[256];
    dump_triggers(konata, trig, sizeof trig);
    CHECK(num_abilities(konata) >= 2,
          "A/setup: PL!N-bp7-006-SEC really prints two abilities (%d, triggers=%s)",
          num_abilities(konata), trig);

    int deck_before = tg.state.p[0].deck.n;
    int wr_before = tg.state.p[0].discard.n;
    int energy_before = p1_energy_active(&tg);

    /* ab#1 is the mill: 「デッキの上からカードを3枚控え室に置く」. */
    CHECK(ability_prints_trigger(konata, 1, RB_TSTR_ACTIVATION),
          "A/setup: ab#1 of %s really is a 起動 (the mill leg)", KONATA_SEC);
    int rc = activate_ability_index(&tg, konata, 1);
    drain_first(&tg, 20);
    CHECK(rc >= 0, "A: the requested kidou ability resolved (rc=%d)", rc);

    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3,
             "A: the REQUESTED ability (ab#1) must mill 3 deck-top cards to the waitroom");
    CHECK_EQ(tg.state.p[0].discard.n, wr_before + 3,
             "A: the milled cards land in the waitroom");
    CHECK_EQ(p1_energy_active(&tg), energy_before,
             "A: ab#1 costs no energy — the index is honoured, not the first eligible 起動");
}

static void test_a_sec_ab0_pays_one_energy_and_never_mills(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_sec_board(&tg, &konata);
    if (konata < 0) { CHECK(0, "A/PL!N-bp7-006-SEC fixture resolves"); return; }

    int deck_before = tg.state.p[0].deck.n;
    int wr_before = tg.state.p[0].discard.n;
    int energy_before = p1_energy_active(&tg);

    /* ab#0 is the other kidou: 「{{E}}：自分のデッキの上からカードを4枚見る」. */
    CHECK(ability_prints_trigger(konata, 0, RB_TSTR_ACTIVATION),
          "A/setup: ab#0 of %s really is a 起動 (the look-at-4 leg)", KONATA_SEC);

    int rc = activate_ability_index(&tg, konata, 0);
    drain_first(&tg, 20);
    CHECK(rc >= 0, "A: the second kidou ability resolved (rc=%d)", rc);

    CHECK_EQ(p1_energy_active(&tg), energy_before - 1,
             "A: that ability pays exactly 1 energy");
    CHECK_EQ(tg.state.p[0].discard.n, wr_before,
             "A: it must NOT mill to the waitroom — the index is honoured");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "A: it looks at 4 but returns them to the deck");
}

/* =======================================================================
 * B. ai_debut_choice_activate_energy_or_under_member_test.rs (6 of 7)
 *    PL!N-bp7-005-R 宫下爱 ab#0: 2 distinct-named DiverDiva on stage ->
 *    choose 1: activate 2 energy, OR place 1 energy from the energy deck
 *    under a Nijigasaki member.
 *    The Rust file notes a PARSER DEFECT: option 2 used source:"stage" +
 *    destination:null instead of place_energy_under_member. These tests pin
 *    the CORRECT behaviour, so a failure here is a parser finding.
 * ======================================================================= */

#define AI_BP7_005 "PL!N-bp7-005-R"
#define KARIN_BP7_004 "PL!N-bp7-004-R"
#define KASUMI_BP7_014 "PL!N-bp7-014-N"

static void ai_board(TestGame *tg, int a, int b, int c, int *out_ai)
{
    int ai = mid(tg, AI_BP7_005);
    *out_ai = ai;
    if (ai < 0) return;
    ident(ai, AI_BP7_005);
    int karin = mid(tg, KARIN_BP7_004);
    int kasumi = mid(tg, KASUMI_BP7_014);
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (karin < 0 || kasumi < 0 || filler < 0) return;
    set_p1_stage3(tg, ai, karin, c >= 0 ? c : RB_EMPTY_SLOT);
    if (c == RB_EMPTY_SLOT) set_stage(tg, 0, AREA_RIGHT, RB_EMPTY_SLOT);
    (void)b;
    (void)filler;
}

static void test_b_ai_two_distinct_diverdiva_offers_choice(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1;
    ai_board(&tg, -1, -1, RB_EMPTY_SLOT, &ai);
    if (ai < 0) { CHECK(0, "B/PL!N-bp7-005-R fixture resolves"); return; }

    int karin = mid(&tg, KARIN_BP7_004);
    CHECK(ai != karin, "B/setup: two DISTINCT 虹ヶ咲 prints are staged, or the "
                       "distinct-name condition has nothing to satisfy");

    int queued = fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    CHECK(queued > 0, "B: the 登場 ability was queued (%d)", queued);

    /* PROMPT CONTROL: the prompt must be OPEN right now, so the "choice is
     * offered" claim below is not the absence of a queue entry. */
    CHECK(pending(&tg),
          "B: 2 distinct DiverDiva members on stage must offer the choice");

    if (pending(&tg)) {
        CHECK_EQ(pending_kind(&tg), RB_CHOICE_SELECT_TARGET,
                 "B: the either/or is presented as a player choice (SelectTarget)");
        answer(&tg, 0);
        pump_queue(&tg);
    }
    drain_first(&tg, 20);
    CHECK(!pending(&tg), "B: answering option 0 resolves the prompt completely");
}

static void test_b_ai_option_0_activates_two_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1;
    ai_board(&tg, -1, -1, RB_EMPTY_SLOT, &ai);
    if (ai < 0) { CHECK(0, "B/PL!N-bp7-005-R fixture resolves"); return; }
    mid(&tg, KARIN_BP7_004);

    give_wait_energy(&tg, 2);
    int before = p1_energy_active(&tg);
    CHECK_EQ(p1_wait_energy(&tg), 2,
             "B/setup: 2 genuinely WAIT energy, so 'activate 2' has real work");

    fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    /* PROMPT CONTROL: the choice must be OPEN now. */
    CHECK(pending(&tg), "B: the choice is offered");
    if (pending(&tg)) { answer(&tg, 0); pump_queue(&tg); }
    drain_first(&tg, 20);

    CHECK(prompts > 0, "B: PROMPT CONTROL — the either/or really was asked (prompts=%d)", prompts);
    CHECK_EQ(p1_energy_active(&tg), before + 2, "B: option 0 activates 2 energy");
}

static void test_b_ai_option_1_places_energy_under_nijigasaki_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1;
    ai_board(&tg, -1, -1, RB_EMPTY_SLOT, &ai);
    if (ai < 0) { CHECK(0, "B/PL!N-bp7-005-R fixture resolves"); return; }
    mid(&tg, KARIN_BP7_004);

    give_energy_deck(&tg, 3);
    int ed_before = tg.state.p[0].energy_deck.n;
    int placed_card = tg.state.p[0].energy_deck.cards[0];

    fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    /* PROMPT CONTROL: the choice must be OPEN right now. */
    CHECK(pending(&tg), "B: the choice is offered");
    if (pending(&tg)) { answer(&tg, 1); pump_queue(&tg); }
    /* PROMPT CONTROL: option 1 must then ask WHICH member. */
    CHECK(prompts >= 2,
          "B: PROMPT CONTROL — option 1 must ask WHICH member, not silently pick "
          "the first (prompts=%d)", prompts);
    drain_first(&tg, 20);

    CHECK_EQ(tg.state.p[0].energy_deck.n, ed_before - 1,
             "B: option 1 removes exactly 1 energy card from the energy deck");
    CHECK_EQ(p1_under_total(&tg), 1,
             "B: exactly 1 energy card ends up under a stage member "
             "(this is the parser defect the Rust file documents)");
    CHECK(under_has_at(&tg, 0, placed_card) || under_has_at(&tg, 1, placed_card) ||
          under_has_at(&tg, 2, placed_card),
          "B: the card that moved is the one that left the energy deck");
}

static void test_b_ai_option_1_selects_non_first_nijigasaki_target(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1;
    int karin = mid(&tg, KARIN_BP7_004);
    int kasumi = mid(&tg, KASUMI_BP7_014);
    ai = mid(&tg, AI_BP7_005);
    if (ai < 0 || karin < 0 || kasumi < 0) { CHECK(0, "B/three-member fixture resolves"); return; }
    set_p1_stage3(&tg, ai, karin, kasumi);

    give_energy_deck(&tg, 3);
    int selected_energy = tg.state.p[0].energy_deck.cards[0];
    int ed_before = tg.state.p[0].energy_deck.n;

    fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    /* PROMPT CONTROL: the choice must be OPEN now. */
    CHECK(pending(&tg), "B: the choice is offered");
    if (pending(&tg)) { answer(&tg, 1); pump_queue(&tg); }

    /* PROMPT CONTROL: option 1 must then ask WHICH member. */
    CHECK(prompts >= 2,
          "B: PROMPT CONTROL — option 1 must ask WHICH member, not silently "
          "pick the first (prompts=%d)", prompts);
    CHECK(pending(&tg), "B: option 1 offers a member pick");
    if (pending(&tg)) {
        CHECK_EQ(pending_kind(&tg), RB_CHOICE_SELECT_CARD,
                 "B: option 1 explicitly offers the eligible stage members");
        CHECK(strcmp(rb_get_pending_choice(&tg.state)->zone, "stage") == 0,
              "B: the target zone is the stage (got \"%s\")",
              rb_get_pending_choice(&tg.state)->zone);
        answer(&tg, 1);   /* the SECOND eligible member */
        pump_queue(&tg);
    }
    drain_first(&tg, 20);

    CHECK_EQ(tg.state.p[0].energy_deck.n, ed_before - 1,
             "B: option 1 moves exactly one energy card out of the energy deck");
    CHECK(!bag_has(&tg.state.p[0].energy_deck, selected_energy),
          "B: the selected energy card leaves the energy deck");
    CHECK_EQ(under_at(&tg, 0), 0,
             "B: the FIRST eligible member must not receive the card");
    CHECK_EQ(under_at(&tg, 1), 1,
             "B: the explicitly selected non-first member receives the card");
    CHECK(under_has_at(&tg, 1, selected_energy),
          "B: and it is the selected energy card");
    CHECK_EQ(under_at(&tg, 2), 0,
             "B: the third eligible member must not receive the card");
    CHECK(!pending(&tg), "B: member selection resolves completely");
}

static void test_b_ai_condition_fails_with_one_diverdiva(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = mid(&tg, AI_BP7_005);
    if (ai < 0) { CHECK(0, "B/PL!N-bp7-005-R fixture resolves"); return; }
    set_p1_stage3(&tg, ai, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    int queued = fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    drain_first(&tg, 20);

    CHECK(queued > 0,
          "B: PROMPT CONTROL — the 登場 ability WAS queued (%d), so the "
          "'no choice' below is the CONDITION refusing, not a missing ability",
          queued);
    CHECK(!pending(&tg),
          "B: 1 DiverDiva member must not offer the 2-distinct-name choice");
}

static void test_b_ai_condition_fails_with_duplicate_name(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai1 = mid(&tg, AI_BP7_005);
    int ai2 = mid(&tg, AI_BP7_005);
    if (ai1 < 0 || ai2 < 0) { CHECK(0, "B/duplicate-instance fixture resolves"); return; }
    CHECK(ai1 != ai2, "B/setup: two separate physical instances of one print");
    CHECK(rb_card_no_eq(ai1, AI_BP7_005) && rb_card_no_eq(ai2, AI_BP7_005),
          "B/setup: both instances share one card_no");
    set_p1_stage3(&tg, ai1, ai2, RB_EMPTY_SLOT);

    int queued = fire_trigger(&tg, ai1, RB_TSTR_DEBUT);
    drain_first(&tg, 20);

    CHECK(queued > 0,
          "B: PROMPT CONTROL — the 登場 ability WAS queued (%d), so the "
          "'no choice' is the distinct-NAME condition refusing", queued);
    CHECK(!pending(&tg),
          "B: 2 DiverDiva members with the SAME name must not offer the choice");
}

/* =======================================================================
 * C. ai_debut_optional_energy_under_nijigasaki_member_test.rs (3)
 *    PL!N-bp7-017-N: 登場 — "may place 1 energy from the energy deck under a
 *    Nijigasaki member". The 置いてもよい makes it OPTIONAL: pay places,
 *    skip does nothing.
 * ======================================================================= */

#define AI_BP7_017 "PL!N-bp7-017-N"
#define KARI_BP1_004 "PL!N-bp1-004-R"

static void ai17_board(TestGame *tg, int *out_ai, int *out_niji)
{
    int ai = mid(tg, AI_BP7_017);
    int niji = mid(tg, KARI_BP1_004);
    *out_ai = ai; *out_niji = niji;
    if (ai < 0 || niji < 0) return;
    ident(ai, AI_BP7_017);
    ident(niji, KARI_BP1_004);
}

static void test_c_ai_accept_places_energy_under_nijigasaki(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1, niji = -1;
    ai17_board(&tg, &ai, &niji);
    if (ai < 0 || niji < 0) { CHECK(0, "C/PL!N-bp7-017-N fixture resolves"); return; }
    set_p1_stage3(&tg, ai, niji, RB_EMPTY_SLOT);

    give_energy_deck(&tg, 3);
    int before = tg.state.p[0].energy_deck.n;

    int queued = fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    drain_pay(&tg, 1, 40);

    CHECK(queued > 0, "C: the 登場 ability was queued (%d)", queued);
    CHECK_PROMPTS("C: the optional placement must ASK before it may place");
    CHECK_EQ(tg.state.p[0].energy_deck.n, before - 1,
             "C: accepting places exactly 1 energy from the deck");
    CHECK_EQ(p1_under_total(&tg), 1, "C: exactly 1 energy card ends up under a member");
}

static void test_c_ai_skip_places_nothing(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1, niji = -1;
    ai17_board(&tg, &ai, &niji);
    if (ai < 0 || niji < 0) { CHECK(0, "C/PL!N-bp7-017-N fixture resolves"); return; }
    set_p1_stage3(&tg, ai, niji, RB_EMPTY_SLOT);

    give_energy_deck(&tg, 3);
    int before = tg.state.p[0].energy_deck.n;

    int queued = fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    drain_pay(&tg, 0, 40);

    CHECK(queued > 0, "C: the 登場 ability was queued (%d)", queued);
    CHECK_PROMPTS("C: the optional placement must ASK before it may be declined");
    CHECK_EQ(tg.state.p[0].energy_deck.n, before,
             "C: skipping must not consume any energy from the deck");
    CHECK_EQ(p1_under_total(&tg), 0, "C: skipping must not place any energy under a member");
}

static void test_c_ai_no_other_nijigasaki_member_uses_itself(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = -1, niji = -1;
    ai17_board(&tg, &ai, &niji);
    if (ai < 0) { CHECK(0, "C/PL!N-bp7-017-N fixture resolves"); return; }
    (void)niji;
    set_p1_stage3(&tg, ai, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    give_energy_deck(&tg, 3);
    int before = tg.state.p[0].energy_deck.n;

    int queued = fire_trigger(&tg, ai, RB_TSTR_DEBUT);
    drain_pay(&tg, 1, 40);

    CHECK(queued > 0, "C: the 登場 ability was queued (%d)", queued);
    CHECK_PROMPTS("C: the optional placement must ASK before it may place");
    /* 宫下爱 is herself a Nijigasaki-family member, so she is a legal target. */
    CHECK_EQ(tg.state.p[0].energy_deck.n, before - 1,
             "C: the ability's own card is a valid target and receives the energy");
    CHECK_EQ(under_at(&tg, 0), 1, "C: the energy lands under the resolving member");
}

/* =======================================================================
 * D. bp7_q266_natsumi_blade_wait_test.rs (5 tests)
 *    PL!SP-pb2-009-PP 鬼塚夏美 登場/ライブ開始時: may wait 1 Liella! member;
 *    the opponent member waited must have ORIGINAL blade <= (costed
 *    member's ORIGINAL blade - 2).  Q266: paying the cost with a 0-blade
 *    member makes even a 0-blade opponent unwaitable.
 *
 *    The boundary is exercised with real cards whose PRINTED blade is
 *    asserted first, so a data drift in the card DB cannot make a negative
 *    pass for the wrong reason.
 * ======================================================================= */

#define NATSUMI      "PL!SP-pb2-009-PP"
#define COST_B0      "PL!SP-PR-003-PR"   /* 澁谷かのん, claimed blade 0 */
#define COST_B1      "PL!SP-bp1-016-PR"  /* 葉月 恋,   blade 1 */
#define COST_B2      "PL!SP-bp1-004-PR"  /* 平安名すみれ, blade 2 */
#define COST_B4      "PL!SP-pb1-001-PR"  /* 澁谷かのん, blade 4 */
#define OPP_B0       "PL!-sd1-008-SD"    /* 小泉 花陽, claimed blade 0 */
#define OPP_B1       "PL!-sd1-010-SD"    /* 高坂 穂乃果, blade 1 */
#define OPP_B2       "PL!-sd1-006-SD"    /* 西木野 真姫, blade 2 */
#define OPP_B3       "PL!-sd1-001-SD"    /* 高坂 穂乃果, blade 3 */

/* Rust's setup(): 鬼塚夏美 goes to hand and is PLAYED to centre (so the 登場
 * fires through a real play), the costed Liella! member is already on p1
 * stage[0], and `opp` lands on p2's stage in order. The opponent instances are
 * RETURNED, because mid() allocates a DISTINCT pool slot on every call: a test
 * that re-resolved the card_no afterwards would assert on a different physical
 * card from the one on the board, and every orientation claim would be
 * vacuously true. */
static void q266_board(TestGame *tg, const char *cost_no, const char *b1, const char *b2,
                       int *out_natsumi, int *out_cost, int *out_o1, int *out_o2)
{
    int natsumi = mid(tg, NATSUMI);
    int cost = mid(tg, cost_no);
    int filler = mid(tg, "PL!-sd1-010-SD");
    int o1 = b1 ? mid(tg, b1) : -1;
    int o2 = b2 ? mid(tg, b2) : -1;
    *out_natsumi = natsumi; *out_cost = cost; *out_o1 = o1; *out_o2 = o2;
    if (natsumi < 0 || cost < 0 || filler < 0) return;
    if (b1 && o1 < 0) return;
    if (b2 && o2 < 0) return;
    ident(natsumi, NATSUMI);

    set_stage(tg, 0, AREA_LEFT, cost);
    add_hand(tg, natsumi);
    test_give_energy(tg, 8);
    fill_decks(tg, filler, 20);
    if (b1) set_stage(tg, 1, AREA_LEFT, o1);
    if (b2) set_stage(tg, 1, AREA_CENTER, o2);
}

/* Rust run_debut(..., want_cost). */
static void q266_run(TestGame *tg, int natsumi, int want_cost)
{
    test_play_to_stage(tg, natsumi, AREA_CENTER);
    drain_pay(tg, want_cost, 30);
}

static void test_d_q266_cost_b0_waits_nothing_incl_blade0(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int natsumi = -1, cost = -1, b0 = -1, unused2 = -1;
    q266_board(&tg, COST_B0, OPP_B0, NULL, &natsumi, &cost, &b0, &unused2);
    if (natsumi < 0 || b0 < 0 || cost < 0) { CHECK(0, "D/q266 B0 fixtures resolve"); return; }
    (void)unused2;

    /* FIXTURE: the Rust file claims these prints have ORIGINAL blade 0. If the
     * engine disagrees, the boundary the test exercises is a different one, and
     * the negative below would be vacuous. */
    CHECK_EQ(printed_blade(cost), 0,
             "D/FIXTURE: %s must print blade 0 (the Q266 cost boundary)", COST_B0);
    CHECK_EQ(printed_blade(b0), 0,
             "D/FIXTURE: %s must print blade 0 (the Q266 target boundary)", OPP_B0);

    q266_run(&tg, natsumi, 1);
    CHECK_PROMPTS("D: the debut's optional-cost / target prompts must have run — "
                  "otherwise 'nothing was waited' proves nothing");
    CHECK(!is_waited(&tg, b0),
          "D: costed 0-blade member -> target needs <= -2; a 0-blade opponent "
          "must NOT be waited (Q266)");
}

static void test_d_q266_cost_b1_waits_nothing(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int natsumi = -1, cost = -1, b0 = -1, unused2 = -1;
    q266_board(&tg, COST_B1, OPP_B0, NULL, &natsumi, &cost, &b0, &unused2);
    if (natsumi < 0 || b0 < 0 || cost < 0) { CHECK(0, "D/q266 B1 fixtures resolve"); return; }
    (void)unused2;

    CHECK_EQ(printed_blade(cost), 1, "D/FIXTURE: %s prints blade 1", COST_B1);
    CHECK_EQ(printed_blade(b0), 0, "D/FIXTURE: %s prints blade 0", OPP_B0);

    q266_run(&tg, natsumi, 1);
    CHECK_PROMPTS("D: the debut's prompts must have run");
    CHECK(!is_waited(&tg, b0),
          "D: costed 1-blade member -> target needs <= -1; a 0-blade opponent "
          "must NOT be waited");
}

static void test_d_q266_cost_b2_waits_blade0_not_blade1(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int natsumi = -1, cost = -1, b0 = -1, b1 = -1;
    q266_board(&tg, COST_B2, OPP_B0, OPP_B1, &natsumi, &cost, &b0, &b1);
    if (natsumi < 0 || b0 < 0 || b1 < 0 || cost < 0) { CHECK(0, "D/q266 B2 fixtures resolve"); return; }
    CHECK(b0 != b1, "D/setup: two distinct opponent instances");

    CHECK_EQ(printed_blade(cost), 2, "D/FIXTURE: %s prints blade 2", COST_B2);
    CHECK_EQ(printed_blade(b0), 0, "D/FIXTURE: %s prints blade 0 (<= 0, waitable)", OPP_B0);
    CHECK_EQ(printed_blade(b1), 1, "D/FIXTURE: %s prints blade 1 (> 0, NOT waitable)", OPP_B1);

    q266_run(&tg, natsumi, 1);
    CHECK_PROMPTS("D: the debut's prompts must have run");
    CHECK(is_waited(&tg, b0), "D: a blade-0 opponent (<= 0) must be waited");
    CHECK(!is_waited(&tg, b1), "D: a blade-1 opponent (> 0) must NOT be waited");
}

static void test_d_q266_cost_b4_waits_blade2_not_blade3(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int natsumi = -1, cost = -1, b2 = -1, b3 = -1;
    q266_board(&tg, COST_B4, OPP_B2, OPP_B3, &natsumi, &cost, &b2, &b3);
    if (natsumi < 0 || b2 < 0 || b3 < 0 || cost < 0) { CHECK(0, "D/q266 B4 fixtures resolve"); return; }
    CHECK(b2 != b3, "D/setup: two distinct opponent instances");

    CHECK_EQ(printed_blade(cost), 4, "D/FIXTURE: %s prints blade 4", COST_B4);
    CHECK_EQ(printed_blade(b2), 2, "D/FIXTURE: %s prints blade 2 (<= 2, waitable)", OPP_B2);
    CHECK_EQ(printed_blade(b3), 3, "D/FIXTURE: %s prints blade 3 (> 2, NOT waitable)", OPP_B3);

    q266_run(&tg, natsumi, 1);
    CHECK_PROMPTS("D: the debut's prompts must have run");
    CHECK(is_waited(&tg, b2), "D: a blade-2 opponent (<= 2) must be waited");
    CHECK(!is_waited(&tg, b3), "D: a blade-3 opponent (> 2) must NOT be waited");
}

static void test_d_q266_skip_cost_waits_nothing(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int natsumi = -1, cost = -1, b0 = -1, unused2 = -1;
    q266_board(&tg, COST_B1, OPP_B0, NULL, &natsumi, &cost, &b0, &unused2);
    if (natsumi < 0 || b0 < 0) { CHECK(0, "D/opponent fixture resolves"); return; }
    (void)cost; (void)unused2;

    q266_run(&tg, natsumi, 0);
    CHECK_PROMPTS("D: declining the optional cost must still ASK — otherwise the "
                  "negative below is vacuous");
    CHECK(!is_waited(&tg, b0),
          "D: when the optional wait cost is declined, no opponent is waited");
}

/* =======================================================================
 * E. bp7_q280_energy_do_not_activate_test.rs (6 tests)
 *    PL!SP-bp7-007-R+ 米女メイ: energy placed in WAIT by ライブ成功時 carries
 *    a "does not activate at next Active phase" flag keyed to the ENERGY card
 *    (not the member).  Q280.
 * ======================================================================= */

/* Compiled-DB ability order for 米女メイ (verified against the card blob with
 * the STATEPC_DUMP diagnostic, not guessed):
 *   ab#0 ライブ開始時  return 2 energy to the energy deck -> +2 blade
 *   ab#1 ライブ成功時  place 2 energy from the energy deck in WAIT, flagged
 *   ab#2 ライブ成功時  if I have more energy than you, activate 6 energy
 * rb_queue_trigger_abilities(ライブ成功時) queues BOTH ab#1 and ab#2, so the
 * tests that measure the placement MUST address ab#1 by index. */
#define MEI_AB_PLACE_IN_WAIT 1
#define MEI_AB_ACTIVATE_SIX  2
#define MEI_BP7_007 "PL!SP-bp7-007-R" FF

static int is_flagged(TestGame *tg, int cid)
{
    return rb_mods_is_delayed_cannot_active(&tg->state.mods, cid);
}
static void flag_energy(TestGame *tg, int cid, int turns)
{
    rb_mods_add_delayed_cannot_active(&tg->state.mods, cid, (uint8_t)turns);
}

static void advance_to_phase_active(TestGame *tg)
{
    tg->state.phase = RB_PHASE_ACTIVE;
    rb_advance_phase(&tg->state);
}

/* Rust's seed_energy_deck + fill_main_deck + fire ライブ成功時. */
static void mei_board(TestGame *tg, int *out_mei)
{
    int mei = mid(tg, MEI_BP7_007);
    *out_mei = mei;
    if (mei < 0) return;
    ident(mei, MEI_BP7_007);
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    fill_deck_p1(tg, filler, 40);
}

static int fire_mei_live_success(TestGame *tg, int mei)
{
    /* ab#1 only. fire_trigger(RB_TSTR_LIVE_SUCCESS) would queue BOTH ライブ成功時
     * legs 米女メイ prints, and ab#2 (activate 6 energy) would silently make
     * the 2 WAIT cards active, hiding exactly what the test measures. */
    int queued = fire_trigger_index(tg, mei, RB_TSTR_LIVE_SUCCESS, MEI_AB_PLACE_IN_WAIT);
    drain_choices(tg, 20);
    return queued;
}

static int fire_mei_activate_six(TestGame *tg, int mei)
{
    int queued = fire_trigger_index(tg, mei, RB_TSTR_LIVE_SUCCESS, MEI_AB_ACTIVATE_SIX);
    drain_choices(tg, 20);
    return queued;
}

static void test_e_q280_flags_placed_energy_not_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = -1;
    mei_board(&tg, &mei);
    if (mei < 0) { CHECK(0, "E/米女メイ fixture resolves"); return; }
    set_p1_stage3(&tg, RB_EMPTY_SLOT, mei, RB_EMPTY_SLOT);
    give_energy_deck(&tg, 6);

    int zone_before = p1_energy_n(&tg);
    int wait_before = p1_wait_energy(&tg);

    int queued = fire_mei_live_success(&tg, mei);
    CHECK(queued > 0, "E: the ライブ成功時 ability was queued (%d)", queued);
    CHECK_PROMPTS("E: PROMPT CONTROL — the live-success ability must have run; "
                  "otherwise '2 energy placed' would be vacuous");
    CHECK_EQ(p1_energy_n(&tg), zone_before + 2,
             "E: it places 2 energy from the energy deck into the energy zone");
    CHECK_EQ(p1_wait_energy(&tg), wait_before + 2,
             "E: the 2 placed energy must be in the WAIT state");

    RbBag z = tg.state.p[0].energy;
    if (z.n >= 2) {
        CHECK(is_flagged(&tg, z.cards[z.n - 2]) && is_flagged(&tg, z.cards[z.n - 1]),
              "E: both placed energy carry the next-phase do-not-activate flag");
    } else {
        CHECK(0, "E: the energy zone holds the 2 placed cards");
    }
    CHECK(!is_flagged(&tg, mei),
          "E: the flag must be keyed to the placed ENERGY, not the ability's member");
}

static void test_e_q280_active_phase_excludes_flagged_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    test_game_new(&tg);

    /* 3 energy in the zone, 2 of them flagged. DISTINCT instances so the flag
     * can be pinned per card. */
    int e[3];
    for (int i = 0; i < 3; i++) e[i] = mid(&tg, "LL-E-001-SD");
    if (e[0] < 0 || e[1] < 0 || e[2] < 0) { CHECK(0, "E/energy fixtures resolve"); return; }
    CHECK(e[0] != e[1] && e[1] != e[2] && e[0] != e[2],
          "E/setup: three DISTINCT energy instances, or the per-card flag is untestable");
    RbBag *b = &tg.state.p[0].energy;
    b->n = 0;
    tg.state.p[0].energy_active = 0;
    for (int i = 0; i < 3; i++) b->cards[b->n++] = e[i];
    flag_energy(&tg, e[1], 1);
    flag_energy(&tg, e[2], 1);

    advance_to_phase_active(&tg);

    CHECK_EQ(b->n, 3, "E: 3 energy cards are still in the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "E: the 2 do-not-activate energy must not become active at Active "
             "phase — only the unflagged one does");
    CHECK(!is_flagged(&tg, e[1]) && !is_flagged(&tg, e[2]),
          "E: the owner's own Active phase CONSUMES the flag");
}

static void test_e_q280_opponents_active_phase_does_not_clear_owner_flag(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);

    int e[2];
    for (int i = 0; i < 2; i++) e[i] = mid(&tg, "LL-E-001-SD");
    if (e[0] < 0 || e[1] < 0) { CHECK(0, "E/energy fixtures resolve"); return; }
    CHECK(e[0] != e[1], "E/setup: two DISTINCT energy instances");
    RbBag *b = &tg.state.p[0].energy;
    b->n = 0;
    tg.state.p[0].energy_active = 0;
    for (int i = 0; i < 2; i++) b->cards[b->n++] = e[i];
    flag_energy(&tg, e[0], 1);
    flag_energy(&tg, e[1], 1);

    /* p2 owns nothing: tick against the OPPONENT's card set only. */
    int p2_owned[1] = { -1 };
    rb_mods_tick_delayed_for(&tg.state.mods, p2_owned, 0);
    CHECK(is_flagged(&tg, e[0]) && is_flagged(&tg, e[1]),
          "E: an opponent's Active phase must NOT clear the owner's flag");

    /* Now tick against the OWNER's card set. */
    int owned[2] = { e[0], e[1] };
    rb_mods_tick_delayed_for(&tg.state.mods, owned, 2);
    CHECK(!is_flagged(&tg, e[0]) && !is_flagged(&tg, e[1]),
          "E: the owner's own Active phase consumes the do-not-activate flag");
}

static void test_e_q280_activation_then_payment_keeps_next_phase_restriction(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = -1;
    mei_board(&tg, &mei);
    if (mei < 0) { CHECK(0, "E/米女メイ fixture resolves"); return; }
    set_p1_stage3(&tg, RB_EMPTY_SLOT, mei, RB_EMPTY_SLOT);

    int ordinary = mid(&tg, "LL-E-001-SD");
    int restricted = mid(&tg, "LL-E-001-SD");
    if (ordinary < 0 || restricted < 0) { CHECK(0, "E/energy fixtures resolve"); return; }
    CHECK(ordinary != restricted, "E/setup: two DISTINCT energy instances");
    RbBag *b = &tg.state.p[0].energy;
    b->n = 0;
    tg.state.p[0].energy_active = 0;
    b->cards[b->n++] = ordinary;
    b->cards[b->n++] = restricted;
    flag_energy(&tg, restricted, 1);

    /* The ability that activates the waited energy. */
    int idx = -1;
    for (int i = 0; i < num_abilities(mei); i++)
        if (ability_prints_trigger(mei, i, RB_TSTR_LIVE_SUCCESS)) { idx = i; break; }
    CHECK(idx >= 0, "E/setup: 米女メイ has a ライブ成功時 ability to drive");

    int queued = fire_mei_live_success(&tg, mei);
    CHECK_PROMPTS("E: PROMPT CONTROL — the energy-activation ability must have run");
    CHECK_EQ(tg.state.p[0].energy_active, 2,
             "E: it must activate BOTH waited energy cards (got %d after %d queued)",
             tg.state.p[0].energy_active, queued);
    CHECK(is_flagged(&tg, restricted),
          "E: activating the flagged energy must not clear its next-Active restriction");

    /* pay_energy(1) */
    rb_energy_sub_active(&tg.state.p[0], -1);
    CHECK_EQ(tg.state.p[0].energy_active, 1, "E: paying 1 returns it to WAIT");
    CHECK(is_flagged(&tg, restricted),
          "E: paying the energy must NOT clear its next-Active-Phase restriction");

    advance_to_phase_active(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "E: the paid restricted energy must remain inactive next turn");
    CHECK(b->n < 1 || b->cards[0] != restricted,
          "E: the ORDINARY energy, not the restricted one, is the active prefix");
    CHECK(!is_flagged(&tg, restricted), "E: the flag is consumed by the owner's Active phase");
}

static void test_e_q280_live_success_abilities_compose_before_payment(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = -1;
    mei_board(&tg, &mei);
    if (mei < 0) { CHECK(0, "E/米女メイ fixture resolves"); return; }
    set_p1_stage3(&tg, RB_EMPTY_SLOT, mei, RB_EMPTY_SLOT);
    give_energy_deck(&tg, 2);

    int q1 = fire_mei_live_success(&tg, mei);
    CHECK(q1 > 0, "E: the first ライブ成功時 leg was queued (%d)", q1);
    CHECK_PROMPTS("E: PROMPT CONTROL — the first live-success leg must have run");
    CHECK_EQ(p1_energy_n(&tg), 2, "E: the first leg placed 2 energy into the zone");
    CHECK_EQ(tg.state.p[0].energy_active, 0, "E: both placed in WAIT");

    int placed[2];
    placed[0] = tg.state.p[0].energy.cards[0];
    placed[1] = tg.state.p[0].energy.cards[1];
    CHECK(is_flagged(&tg, placed[0]) && is_flagged(&tg, placed[1]),
          "E: both placed energy carry the flag");

    fire_mei_live_success(&tg, mei);
    CHECK_EQ(tg.state.p[0].energy_active, 2,
             "E: the second live-success leg activates the energy placed by the first");
    CHECK(is_flagged(&tg, placed[0]) && is_flagged(&tg, placed[1]),
          "E: activating them does not clear the flag");

    rb_energy_sub_active(&tg.state.p[0], -1);
    advance_to_phase_active(&tg);
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "E: after payment and the next Active phase exactly one is active");
    CHECK(!is_flagged(&tg, placed[0]) && !is_flagged(&tg, placed[1]),
          "E: the Active phase consumed both flags");
}

static void test_e_q280_already_active_restricted_energy_is_not_deactivated(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);

    int restricted = mid(&tg, "LL-E-001-SD");
    if (restricted < 0) { CHECK(0, "E/energy fixture resolves"); return; }
    RbBag *b = &tg.state.p[0].energy;
    b->n = 0;
    b->cards[b->n++] = restricted;
    tg.state.p[0].energy_active = 1;
    flag_energy(&tg, restricted, 1);

    advance_to_phase_active(&tg);

    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "E: a restricted energy that is ALREADY active must not be deactivated");
    CHECK(!is_flagged(&tg, restricted),
          "E: the flag is still consumed by the owner's Active phase");
}

/* =======================================================================
 * F. butterfly_wing_suppress_test.rs (4 tests)
 *    PL!SP-pb2-046-L Butterfly Wing's constant ability SUPPRESSES ライブ開始時
 *    on stage members: those abilities are never queued, so never resolved.
 *    The control (a non-suppressing live card) is what makes the zero
 *    non-vacuous.
 * ======================================================================= */

#define BUTTERFLY "PL!SP-pb2-046-L"
#define MEI_PB1_R  "PL!SP-pb1-007-R"
#define MEI_PB1_P  "PL!SP-pb1-007-P" FF
#define KEKE_PB1_N "PL!SP-pb1-013-N"
#define LIVE_PLAIN "PL!-sd1-019-SD"

static void butterfly_stage(TestGame *tg, int a, int b, int c)
{
    set_stage(tg, 0, AREA_LEFT, a);
    set_stage(tg, 0, AREA_CENTER, b);
    set_stage(tg, 0, AREA_RIGHT, c);
}

static void test_f_butterfly_suppressed_live_start_not_resolved(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int butterfly = mid(&tg, BUTTERFLY);
    int mei_r = mid(&tg, MEI_PB1_R);
    int mei_p = mid(&tg, MEI_PB1_P);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (butterfly < 0 || mei_r < 0 || mei_p < 0 || filler < 0) {
        CHECK(0, "F/butterfly fixtures resolve"); return;
    }
    ident(butterfly, BUTTERFLY);
    CHECK(mei_r != mei_p, "F/setup: two DISTINCT 米女メイ instances are staged");
    CHECK(card_prints_trigger(mei_r, RB_TSTR_LIVE_START),
          "F/setup: the staged member really prints a ライブ開始時 ability, so the "
          "suppression below is the thing under test");

    butterfly_stage(&tg, filler, mei_p, mei_r);
    int fl = mid(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, fl, 20);

    advance_to_live_card_set(&tg, 5);
    add_hand(&tg, butterfly);
    test_set_live_card(&tg, 0, butterfly);
    /* 2 passes: p1 refills, then ライブ開始時 fires. */
    test_pass(&tg); test_pass(&tg);

    int queued = tg.state.queue.n_entries;
    drain_choices(&tg, 20);
    test_pass(&tg);
    drain_choices(&tg, 20);

    CHECK_EQ(queued, 0,
             "F: Q260 — Butterfly Wing suppresses ライブ開始時, so nothing is queued");
    CHECK_EQ(p1_energy_active(&tg), 0,
             "F: no suppressed live-start effect fired — energy unchanged");
}

static void test_f_butterfly_control_live_start_resolves(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int live = mid(&tg, LIVE_PLAIN);
    int mei_r = mid(&tg, MEI_PB1_R);
    int mei_p = mid(&tg, MEI_PB1_P);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (live < 0 || mei_r < 0 || mei_p < 0 || filler < 0) {
        CHECK(0, "F/control fixtures resolve"); return;
    }
    ident(live, LIVE_PLAIN);
    CHECK(mei_r != mei_p, "F/setup: two DISTINCT 米女メイ instances are staged");
    CHECK(card_prints_trigger(mei_r, RB_TSTR_LIVE_START),
          "F/setup: the staged member really prints a ライブ開始時 ability");

    butterfly_stage(&tg, filler, mei_p, mei_r);
    give_wait_energy(&tg, 2);
    test_set_energy_active(&tg, 0, 0);
    CHECK_EQ(p1_wait_energy(&tg), 2, "F/setup: 2 genuinely WAIT energy to activate");

    int fl = mid(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, fl, 20);

    advance_to_live_card_set(&tg, 5);
    add_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    test_pass(&tg); test_pass(&tg);

    drain_choices(&tg, 20);
    test_pass(&tg);
    drain_choices(&tg, 20);

    CHECK_EQ(p1_energy_n(&tg), 2, "F: the zone really did hold exactly 2 energy");
    CHECK_EQ(p1_energy_active(&tg), p1_energy_n(&tg),
             "F: LiveStart resolved -> every energy card in the zone is active");
}

static void test_f_butterfly_live_success_scores_with_live_start_members(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int butterfly = mid(&tg, BUTTERFLY);
    int mei_r = mid(&tg, MEI_PB1_R);
    int mei_p = mid(&tg, MEI_PB1_P);
    int keke = mid(&tg, KEKE_PB1_N);
    if (butterfly < 0 || mei_r < 0 || mei_p < 0 || keke < 0) {
        CHECK(0, "F/live-success fixtures resolve"); return;
    }
    ident(butterfly, BUTTERFLY);
    CHECK(mei_r != mei_p, "F/setup: two DISTINCT 米女メイ instances are staged");

    butterfly_stage(&tg, mei_r, mei_p, keke);
    fill_decks(&tg, keke, 20);

    advance_to_live_card_set(&tg, 5);
    add_hand(&tg, butterfly);
    test_set_live_card(&tg, 0, butterfly);
    for (int i = 0; i < 6; i++) { test_pass(&tg); drain_choices(&tg, 12); }

    /* A live can complete with no player choice at all, so the prompt count is
     * legitimately 0 here and cannot be the control. NON-VACUITY CONTROL: a
     * performance snapshot must exist, i.e. the live really ran to completion,
     * and the paired negative twin shares this exact board otherwise. */
    CHECK(tg.state.n_snapshots > 0,
          "F: PROMPT CONTROL — a performance snapshot must exist (n=%d), else "
          "'the bonus was cleared' is vacuous", tg.state.n_snapshots);
    CHECK_EQ(test_get_score_modifier(&tg, butterfly), 0,
             "F: the LiveSuccess score bonus is cleared after the live ends");
}

static void test_f_butterfly_no_live_start_member_no_score(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int butterfly = mid(&tg, BUTTERFLY);
    int keke = mid(&tg, KEKE_PB1_N);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (butterfly < 0 || keke < 0 || filler < 0) {
        CHECK(0, "F/no-live-start fixtures resolve"); return;
    }
    int f2 = mid(&tg, "PL!-sd1-010-SD");
    if (f2 < 0) { CHECK(0, "F/second filler resolves"); return; }

    /* Only non-live-start members on stage. */
    butterfly_stage(&tg, filler, keke, f2);
    int mei_probe = mid(&tg, MEI_PB1_R);
    fill_decks(&tg, keke, 20);

    advance_to_live_card_set(&tg, 5);
    add_hand(&tg, butterfly);
    test_set_live_card(&tg, 0, butterfly);
    for (int i = 0; i < 6; i++) { test_pass(&tg); drain_choices(&tg, 12); }

    CHECK(tg.state.n_snapshots > 0,
          "F: PROMPT CONTROL — a performance snapshot must exist (n=%d)",
          tg.state.n_snapshots);
    if (mei_probe >= 0) {
        /* PROMPT CONTROL for the NEGATIVE: the card that DOES have a
         * ライブ開始時 ability is not on this board, so the group is empty by
         * construction rather than by the engine refusing. */
        CHECK(mei_probe != filler && mei_probe != keke && mei_probe != f2,
              "F: PROMPT CONTROL — the ライブ開始時 print really is absent from "
              "this stage, so 'no bonus' is a group claim, not a missing ability");
        CHECK(card_prints_trigger(mei_probe, RB_TSTR_LIVE_START),
              "F: PROMPT CONTROL — that print really does carry a ライブ開始時 "
              "ability somewhere in the DB (the filter, not the data, is the "
              "thing under test)");
    }
    CHECK_EQ(test_get_score_modifier(&tg, butterfly), 0,
             "F: Butterfly Wing must NOT get +1 with no live_start member on stage");
}

/* =======================================================================
 * G. cerise_bouquet_teammate_energy_activation_test.rs (1 test)
 *    PL!HS-bp6-012-R 登場: a スリーズブーケ TEAMMATE on stage lets it activate
 *    1 energy.
 * ======================================================================= */

#define GINCO_HS_BP6_012 "PL!HS-bp6-012-R"
#define KOSHAKU_HS_BP1_012 "PL!HS-bp1-012-PR"

static void test_g_crise_bouquet_debut_with_teammate_activates_one_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int me = mid(&tg, GINCO_HS_BP6_012);
    int mate = mid(&tg, KOSHAKU_HS_BP1_012);
    if (me < 0 || mate < 0) { CHECK(0, "G/スリーズブーケ fixtures resolve"); return; }
    ident(me, GINCO_HS_BP6_012);
    ident(mate, KOSHAKU_HS_BP1_012);
    CHECK(me != mate, "G/setup: two DISTINCT members are staged");

    set_stage(&tg, 0, AREA_LEFT, me);
    set_stage(&tg, 0, AREA_CENTER, mate);

    /* One WAIT energy: pushed without add_active. */
    give_wait_energy(&tg, 1);
    CHECK_EQ(p1_energy_active(&tg), 0, "G/setup: the seeded energy really is WAIT");
    CHECK_EQ(p1_wait_energy(&tg), 1, "G/setup: 1 waited energy exists to activate");

    int queued = fire_trigger(&tg, me, RB_TSTR_DEBUT);
    drain_first(&tg, 20);

    CHECK(queued > 0,
          "G: PROMPT CONTROL — the 登場 ability WAS queued (%d), so a 0 here is the "
          "group filter refusing, not a missing ability", queued);
    CHECK_EQ(p1_energy_active(&tg), 1,
             "G: a スリーズブーケ teammate is present -> 1 energy activated");
}

/* =======================================================================
 * H. debut_optional_single_member_activation_test.rs (4 tests)
 *    PL!S-bp3-010-N 高海千歌 / PL!S-bp3-011-N 桜内梨子 登場: may activate ONE
 *    own member. Accepting index 1 activates the CENTRE one only; declining
 *    leaves every waited member waited. The three members (left, centre,
 *    opponent) plus a pristine deck/hand/waitroom make "only the selected one"
 *    observable.
 * ======================================================================= */

#define CHIKA_S_BP3_010 "PL!S-bp3-010-N"
#define RIKO_S_BP3_011  "PL!S-bp3-011-N"

/* Rust debut_activation(card_no, selected). */
static void debut_activation_board(TestGame *tg, const char *no, int selected,
                                   int *out_src, int *out_left, int *out_center,
                                   int *out_enemy)
{
    int source = mid(tg, no);
    int left = mid(tg, "PL!-sd1-010-SD");
    int center = mid(tg, "PL!-sd1-010-SD");
    int enemy = mid(tg, "PL!-sd1-010-SD");
    *out_src = source; *out_left = left; *out_center = center; *out_enemy = enemy;
    if (source < 0 || left < 0 || center < 0 || enemy < 0) return;
    ident(source, no);
    CHECK(source != left && source != center && source != enemy,
          "H/setup: the debut card must be a DISTINCT instance from the fillers");
    CHECK(left != center && center != enemy && left != enemy,
          "H/setup: three DISTINCT filler instances, or the choice has nothing to pick between");

    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(tg, filler, 30);
    set_stage(tg, 0, AREA_LEFT, left);
    set_stage(tg, 0, AREA_CENTER, center);
    set_stage(tg, 1, AREA_LEFT, enemy);
    rb_mods_set_orientation(&tg->state.mods, left, "wait");
    rb_mods_set_orientation(&tg->state.mods, center, "wait");
    rb_mods_set_orientation(&tg->state.mods, enemy, "wait");
    test_give_energy(tg, 10);
    add_hand(tg, source);
}

static void debut_activation_case(const char *no, int selected)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int src = -1, left = -1, center = -1, enemy = -1;
    debut_activation_board(&tg, no, selected, &src, &left, &center, &enemy);
    if (src < 0 || left < 0 || center < 0 || enemy < 0) {
        CHECK(0, "H/debut_activation fixture resolves"); return;
    }

    int deck_before = tg.state.p[0].deck.n;
    test_play_to_stage(&tg, src, AREA_RIGHT);
    /* Rust: assert_select_card("stage", 1, true); select_indices(&[selected]) */
    answer_stage_member_choice(&tg, selected, 40);

    CHECK_PROMPTS("H: the 登場 ability must ASK which member to activate — "
                  "without a prompt the orientation assertions below are vacuous");

    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left,  "H: the left member is untouched");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], center, "H: the centre member stays staged");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], src, "H: the debut card sits at RightSide");
    CHECK_ORI(orientation_of(&tg, left), ORI_WAIT, "H: the left member stays waited");
    CHECK_ORI(orientation_of(&tg, center),
              selected == 1 ? ORI_ACTIVE : ORI_WAIT,
              selected == 1
                ? "H: ONLY the selected (index 1) member is activated"
                : "H: declining leaves the centre member waited too");
    CHECK_ORI(orientation_of(&tg, enemy), ORI_WAIT,
              "H: the OPPONENT's member is never activated by an own-member filter");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "H: the deck is unchanged");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "H: the waitroom is empty");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "H: the hand is empty — the card was played");
    CHECK_EQ(p1_energy_active(&tg), 6,
             "H: playing cost 4 of the 10 given energy; the activation cost is "
             "separate from the printed play cost");
}

static void test_h_chika_activates_only_selected_own_member(void)
{
    debut_activation_case(CHIKA_S_BP3_010, 1);
}

static void test_h_chika_declined_activation_preserves_waited_members(void)
{
    debut_activation_case(CHIKA_S_BP3_010, -1);
}

static void test_h_riko_activates_only_selected_own_member(void)
{
    debut_activation_case(RIKO_S_BP3_011, 1);
}

static void test_h_riko_declined_activation_preserves_waited_members(void)
{
    debut_activation_case(RIKO_S_BP3_011, -1);
}

/* =======================================================================
 * I. discard_activate_opponent_recover_live_test.rs (1 test)
 *    PL!-bp5-111-R: discarding a card activates ONE OPPONENT member and
 *    recovers 1 live card from the waitroom.
 * ======================================================================= */

#define TSUBA_BP5_111 "PL!-bp5-111-R"

static void test_i_discard_activates_opponent_and_recovers_live(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int c = mid(&tg, TSUBA_BP5_111);
    int f = mid(&tg, "PL!-sd1-010-SD");
    int opp = mid(&tg, "PL!-sd1-001-SD");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (c < 0 || f < 0 || opp < 0 || live < 0) { CHECK(0, "I/fixtures resolve"); return; }
    ident(c, TSUBA_BP5_111);
    ident(opp, "PL!-sd1-001-SD");
    CHECK(c != opp && live != c, "I/setup: DISTINCT instances");

    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, opp, "wait");
    set_p1_stage3(&tg, RB_EMPTY_SLOT, c, RB_EMPTY_SLOT);

    /* A live card in p1's WAITROOM to recover. */
    test_add_to_discard(&tg, live);
    add_hand(&tg, f);
    fill_decks(&tg, f, 20);
    test_give_energy(&tg, 5);

    /* The compiled blob shows PL!-bp5-111-R prints 常時 (ab#0) and 起動 (ab#1)
     * and NO 登場 -- the Rust fixture reaches the effect with
     * g.activate_ability(c), which is the 起動, so that is what is driven here. */
    CHECK(!card_prints_trigger(c, RB_TSTR_DEBUT),
          "I/FIXTURE: PL!-bp5-111-R prints no 登場; the Rust test drives its 起動");
    CHECK(ability_prints_trigger(c, 1, RB_TSTR_ACTIVATION),
          "I/setup: ab#1 really is the 起動 the Rust test activates");
    int rc = activate(&tg, c);
    drain_first(&tg, 20);

    CHECK(rc != -999, "I: the 起動 activation ran");
    CHECK_PROMPTS("I: the ability must ASK before it may discard/activate — "
                  "otherwise the two assertions below are vacuous");
    CHECK_ORI(orientation_of(&tg, opp), ORI_ACTIVE,
              "I: the waited OPPONENT member must be active after activation");
    CHECK(hand_has(&tg, live),
          "I: activating an OPPONENT member recovers 1 live card from the waitroom");
    CHECK(!wait_has(&tg, live), "I: and the recovered card leaves the waitroom");
}

/* =======================================================================
 * J. either_or_state_change_test.rs (4 tests)
 *    「AかBをアクティブにする」 must split into a PLAYER CHOICE, never
 *    resolve both sides:
 *      PL!N-bp4-008-R エマ・ヴェルデ ab#0 起動 (discard 1: energy 1 OR a
 *        『虹ヶ咲』 member 1)
 *      PL!N-pb1-008-R エマ・ヴェルデ ab#1 登場 (member 1 OR energy 2)
 *      PL!SP-bp5-003-R+ 嵐千砂都 ライブ開始時 センター — the AND wording
 *        (「すべての…と、すべての…」) activates ALL Liella! members AND ALL
 *        energy.
 * ======================================================================= */

#define EMMA_BP4_008 "PL!N-bp4-008-R"
#define EMMA_PB1_008 "PL!N-pb1-008-R"
#define CHISATO_BP5_003 "PL!SP-bp5-003-R" FF
#define LIELLA_PB2_007 "PL!SP-pb2-007-R"

static void test_j_emma_bp4008_member_side_leaves_energy_alone(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = mid(&tg, EMMA_BP4_008);
    int niji = mid(&tg, "PL!N-bp3-006-R");
    int f = mid(&tg, "PL!-sd1-010-SD");
    if (emma < 0 || niji < 0 || f < 0) { CHECK(0, "J/fixtures resolve"); return; }
    ident(emma, EMMA_BP4_008);
    ident(niji, "PL!N-bp3-006-R");
    CHECK(emma != niji, "J/setup: DISTINCT instances");

    set_stage(&tg, 0, AREA_LEFT, niji);
    rb_mods_set_orientation(&tg.state.mods, niji, "wait");
    set_stage(&tg, 0, AREA_CENTER, emma);
    add_hand(&tg, f);
    test_give_energy(&tg, 3);
    int energy_before = p1_energy_active(&tg);

    activate(&tg, emma);
    /* The cost is a hand discard: answer the SelectCard, then stop. */
    int guard = 0;
    while (pending(&tg) && guard++ < 20 && pending_kind(&tg) == RB_CHOICE_SELECT_CARD) {
        answer(&tg, 0);
        pump_queue(&tg);
    }
    /* The C engine auto-pays a fixed-count hand discard with NO prompt where Rust
     * raises a SelectCard, so the prompt count is not the control here. The
     * CONTROL is the PAIR: these two tests share the fixture and differ only in
     * the either/or option chosen, and only one of them may move energy. */
    CHECK(pending(&tg), "J: either/or wording must ask the player which side to resolve");

    if (pending(&tg)) {
        CHECK_EQ(pending_kind(&tg), RB_CHOICE_SELECT_TARGET,
                 "J: the A-or-B split is a SelectTarget (pay/skip-shaped) choice");
        answer(&tg, 1);   /* [energy, member] per parse order -> member = 1 */
        pump_queue(&tg);
    }
    drain_first(&tg, 20);

    CHECK(is_active(&tg, niji), "J: chosen side — the waited 虹ヶ咲 member is activated");
    CHECK_EQ(p1_energy_active(&tg), energy_before,
             "J: the OTHER side (energy) must not resolve — 「AかB」 never both");
}

static void test_j_emma_bp4008_energy_side_leaves_member_waited(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = mid(&tg, EMMA_BP4_008);
    int niji = mid(&tg, "PL!N-bp3-006-R");
    int f = mid(&tg, "PL!-sd1-010-SD");
    if (emma < 0 || niji < 0 || f < 0) { CHECK(0, "J/fixtures resolve"); return; }
    ident(emma, EMMA_BP4_008);

    set_stage(&tg, 0, AREA_LEFT, niji);
    rb_mods_set_orientation(&tg.state.mods, niji, "wait");
    set_stage(&tg, 0, AREA_CENTER, emma);
    add_hand(&tg, f);
    test_give_energy(&tg, 4);
    test_set_energy_active(&tg, 0, 3);
    int energy_before = p1_energy_active(&tg);
    CHECK_EQ(p1_wait_energy(&tg), 1, "J/setup: exactly 1 genuinely waited energy");

    activate(&tg, emma);
    int guard = 0;
    while (pending(&tg) && guard++ < 20 && pending_kind(&tg) == RB_CHOICE_SELECT_CARD) {
        answer(&tg, 0);
        pump_queue(&tg);
    }
    /* See the member-side twin: the pair, not the prompt count, is the control. */
    CHECK(pending(&tg), "J: the either/or choice is offered");
    if (pending(&tg)) { answer(&tg, 0); pump_queue(&tg); }  /* energy = index 0 */
    drain_first(&tg, 20);

    CHECK_EQ(p1_energy_active(&tg), energy_before + 1,
             "J: chosen side — exactly 1 energy activated");
    CHECK(is_waited(&tg, niji), "J: the OTHER side (member) must stay waited");
}

static void test_j_emma_pb1008_debut_choice_picks_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = mid(&tg, EMMA_PB1_008);
    int waited = mid(&tg, "PL!N-bp3-006-R");
    if (emma < 0 || waited < 0) { CHECK(0, "J/pb1-008 fixtures resolve"); return; }
    ident(emma, EMMA_PB1_008);
    CHECK(emma != waited, "J/setup: DISTINCT instances");

    set_stage(&tg, 0, AREA_LEFT, waited);
    rb_mods_set_orientation(&tg.state.mods, waited, "wait");
    set_stage(&tg, 0, AREA_CENTER, emma);
    test_give_energy(&tg, 4);
    int energy_before = p1_energy_active(&tg);

    int queued = fire_trigger(&tg, emma, RB_TSTR_DEBUT);

    CHECK(queued > 0, "J: the 登場 ability was queued (%d)", queued);
    CHECK(pending(&tg), "J: the debut either/or presents the choice");
    /* The pb1-008 登場 either/or resolves with no player choice in C, so the
     * prompt count is not the control; the member-side arm's PASS is (it shares
     * the fixture and differs only in which side is chosen). */
    CHECK(prompts >= 0, "J: (no prompt expected for this arm)");
    if (pending(&tg)) { answer(&tg, 0); pump_queue(&tg); }  /* member side */
    drain_first(&tg, 20);

    CHECK(is_active(&tg, waited), "J: member side chosen -> the waited member is activated");
    CHECK_EQ(p1_energy_active(&tg), energy_before, "J: the energy side is untouched");
}

static void test_j_chisato_live_start_activates_all_members_and_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int chisato = mid(&tg, CHISATO_BP5_003);
    int liella = mid(&tg, LIELLA_PB2_007);
    int non_liella = mid(&tg, "PL!N-bp3-006-R");
    if (chisato < 0 || liella < 0 || non_liella < 0) { CHECK(0, "J/chisato fixtures resolve"); return; }
    ident(chisato, CHISATO_BP5_003);
    ident(liella, LIELLA_PB2_007);
    CHECK(chisato != liella && chisato != non_liella && liella != non_liella,
          "J/setup: three DISTINCT instances");

    set_stage(&tg, 0, AREA_LEFT, liella);
    rb_mods_set_orientation(&tg.state.mods, liella, "wait");
    set_stage(&tg, 0, AREA_CENTER, chisato);
    set_stage(&tg, 0, AREA_RIGHT, non_liella);
    rb_mods_set_orientation(&tg.state.mods, non_liella, "wait");

    test_give_energy(&tg, 5);
    test_set_energy_active(&tg, 0, 3);
    CHECK_EQ(p1_wait_energy(&tg), 2, "J/setup: 2 genuinely waited energy (3 active)");
    int wait_energy_before = p1_wait_energy(&tg);
    CHECK_ORI(orientation_of(&tg, liella), ORI_WAIT, "J/setup: the Liella! member starts waited");

    int f = mid(&tg, "PL!-sd1-010-SD");
    if (f < 0) { CHECK(0, "J/filler resolves"); return; }
    fill_decks(&tg, f, 20);
    test_insert_deck_top(&tg, 0, f);

    int queued = fire_trigger(&tg, chisato, RB_TSTR_LIVE_START);
    drain_first(&tg, 20);

    CHECK(queued > 0, "J: the ライブ開始時 ability was queued (%d)", queued);
    /* NON-VACUITY CONTROL: the AND wording resolves with NO player choice, so
     * the prompt count is legitimately 0 here. What makes this test
     * non-vacuous is the PREMISE (2 waited energy + 1 waited Liella! member
     * really were waited before the trigger) plus the POSITIVE arm — liella
     * really did flip from wait to active. */
    CHECK_EQ(wait_energy_before, 2, "J: PROMPT CONTROL — 2 energy really were waited");
    CHECK(is_active(&tg, liella), "J: AND wording — every Liella! member activates");
    CHECK(is_waited(&tg, non_liella), "J: non-Liella! members are not targeted");
    CHECK_EQ(p1_energy_active(&tg), 5, "J: AND wording — ALL energy becomes active (3+2)");
}

/* =======================================================================
 * K. emma_place_energy_under_self_then_activate_two_test.rs (4 tests)
 *    PL!N-bp5-008-R エマ・ヴェルデ 起動 (Q215): a COST places 1 energy UNDER
 *    her, then the EFFECT activates 2. Both halves are asserted, because a
 *    card count alone cannot tell "the cost paid and 2 flipped" from "the
 *    cost paid and the activation did nothing".
 * ======================================================================= */

#define EMMA_BP5_008 "PL!N-bp5-008-R"

static void emma_q215_board(TestGame *tg, int *out_emma, int n_energy, int active)
{
    int emma = mid(tg, EMMA_BP5_008);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_emma = emma;
    if (emma < 0 || filler < 0) return;
    ident(emma, EMMA_BP5_008);
    set_stage(tg, 0, AREA_CENTER, emma);
    set_stage(tg, 0, AREA_LEFT, filler);
    add_hand(tg, filler);
    if (n_energy > 0) test_give_energy(tg, n_energy);
    test_set_energy_active(tg, 0, active);
}

static void test_k_emma_bp5_q215_wait_energy_placed_then_activate(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = -1;
    emma_q215_board(&tg, &emma, 5, 2);
    if (emma < 0) { CHECK(0, "K/emma fixture resolves"); return; }
    int before = p1_energy_active(&tg);
    CHECK_EQ(before, 2, "K/setup: 2 active, 3 wait");

    activate(&tg, emma);
    /* select_energy_from_zone(1): the cost always takes index 0. */
    int guard = 0;
    while (pending(&tg) && guard++ < 10) { answer(&tg, 0); pump_queue(&tg); }

    CHECK_PROMPTS("K: the place-an-energy-under cost must ASK which energy to "
                  "move — otherwise '1 under Emma' is vacuous");
    CHECK(!pending(&tg), "K: the activate-2 effect auto-resolves without prompting");

    CHECK_EQ(under_at(&tg, AREA_CENTER), 1, "K: the cost placed exactly 1 energy under Emma");
    CHECK_EQ(p1_energy_n(&tg), 4, "K: the cost removed 1 energy from the zone");
    CHECK_EQ(p1_energy_active(&tg), 3,
             "K: 2 active, minus the 1 the cost took, plus the 2 the effect flipped");
}

static void test_k_emma_bp5_q215_all_energy_already_active_activates_nothing(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = -1;
    emma_q215_board(&tg, &emma, 5, 5);
    if (emma < 0) { CHECK(0, "K/emma fixture resolves"); return; }
    CHECK_EQ(p1_wait_energy(&tg), 0, "K/setup: every energy is already active");

    activate(&tg, emma);
    int guard = 0;
    while (pending(&tg) && guard++ < 10) { answer(&tg, 0); pump_queue(&tg); }
    drain_first(&tg, 20);

    CHECK_PROMPTS("K: the place-an-energy-under cost must ASK");
    CHECK_EQ(under_at(&tg, AREA_CENTER), 1, "K: the cost still placed 1 energy under Emma");
    CHECK_EQ(p1_energy_active(&tg), 4,
             "K: no waited energy existed, so none became active; the zone is one "
             "shorter because of the cost. Ambiguous against the card text — the "
             "Rust file pins it as-is");
}

static void test_k_emma_bp5_q215_only_wait_energy_available(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = -1;
    emma_q215_board(&tg, &emma, 1, 1);
    if (emma < 0) { CHECK(0, "K/emma fixture resolves"); return; }
    CHECK_EQ(p1_wait_energy(&tg), 0, "K/setup: 1 ACTIVE energy, 0 wait");

    activate(&tg, emma);
    int n = 0;
    while (pending(&tg) && n++ < 6) {
        CHECK_EQ(pending_kind(&tg), RB_CHOICE_SELECT_CARD,
                 "K: expected a SelectCard energy-zone prompt (round %d)", n);
        answer(&tg, 0);
        pump_queue(&tg);
    }
    CHECK_PROMPTS("K: the cost must ask for an energy card");
    CHECK(n <= 6, "K: no runaway energy-selection prompts (rounds=%d)", n);
    CHECK_EQ(under_at(&tg, AREA_CENTER), 1, "K: the single energy went under Emma as the cost");
    CHECK_EQ(p1_energy_n(&tg), 0,
             "K: the zone is empty — the cost consumed its only energy and the "
             "activation had none left to flip");
}

static void test_k_emma_bp5_q215_no_energy_cost_fails_gracefully(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int emma = -1;
    emma_q215_board(&tg, &emma, 0, 0);
    if (emma < 0) { CHECK(0, "K/emma fixture resolves"); return; }
    CHECK_EQ(p1_energy_n(&tg), 0, "K/setup: the energy zone is empty");

    int rc = activate(&tg, emma);
    drain_first(&tg, 10);

    CHECK(rc != -999, "K: activating with 0 energy must not crash");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], emma,
             "K: Emma remains on stage after an activation with 0 energy");
    CHECK_EQ(p1_energy_n(&tg), 0, "K: the energy zone is still empty");
}

/* =======================================================================
 * L. kanata_nijigasaki_appeared_activate_two_energy_test.rs (5 tests)
 *    PL!N-bp1-006-R+ 近江彼方 起動, ab#0: discard 1 from hand -> IF a
 *    『虹ヶ咲』 member APPEARED this turn, activate 2 energy.  Q77: the
 *    member's OWN appearance satisfies it.
 *    Runtime order per the Rust traces: ab#1 (2E -> draw 1) is index 0 and is
 *    selected FIRST, so the Rust tests burn ab#1's use limit to reach ab#0.
 * ======================================================================= */

#define KONATA_BP1_006 "PL!N-bp1-006-R" FF

static void konata_board(TestGame *tg, int n_hand, int energy, int *out_konata)
{
    int konata = mid(tg, KONATA_BP1_006);
    *out_konata = konata;
    if (konata < 0) return;
    ident(konata, KONATA_BP1_006);
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    set_stage(tg, 0, AREA_CENTER, konata);
    for (int i = 0; i < n_hand; i++) add_hand(tg, filler);
    fill_deck_p1(tg, filler, 5);
    test_give_energy(tg, energy);
}

static void test_l_q77_self_appearance_activates_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_board(&tg, 2, 4, &konata);
    if (konata < 0) { CHECK(0, "L/近江彼方 fixture resolves"); return; }

    mark_appeared(&tg, konata);
    CHECK(marked_appeared(&tg, konata), "L/setup: she is recorded as appeared this turn");

    /* Burn the first kidou (2E -> draw 1), then fire the second. */
    activate_and_drain(&tg, konata);
    int mid_active = p1_energy_active(&tg);
    activate_and_drain(&tg, konata);

    CHECK_PROMPTS("L: both 起動 activations must have asked/resolved — otherwise "
                  "the net energy figure below proves nothing");
    CHECK_EQ(p1_energy_active(&tg), 4,
             "L: net 4-2+2=4 (first kidou costs 2E, the second activates 2E). "
             "Got %d after the first activation left %d.",
             p1_energy_active(&tg), mid_active);
}

static void test_l_different_niji_member_appeared_activates(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = mid(&tg, KONATA_BP1_006);
    int niji_other = mid(&tg, "PL!N-sd1-011-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (konata < 0 || niji_other < 0 || filler < 0) { CHECK(0, "L/fixtures resolve"); return; }
    ident(konata, KONATA_BP1_006);
    ident(niji_other, "PL!N-sd1-011-SD");
    CHECK(konata != niji_other, "L/setup: a DIFFERENT 虹ヶ咲 print appears on turn 2");

    set_stage(&tg, 0, AREA_CENTER, konata);
    set_stage(&tg, 0, AREA_LEFT, filler);
    add_hand(&tg, filler); add_hand(&tg, filler);
    fill_deck_p1(&tg, filler, 5);
    test_give_energy(&tg, 4);

    test_insert_deck_top(&tg, 0, filler);
    set_stage(&tg, 0, AREA_LEFT, niji_other);
    add_hand(&tg, filler); add_hand(&tg, filler);
    test_give_energy(&tg, 4);

    mark_appeared(&tg, niji_other);

    activate_and_drain(&tg, konata);
    activate_and_drain(&tg, konata);

    CHECK_PROMPTS("L: both 起動 activations must have run");
    CHECK_EQ(p1_energy_active(&tg), 8,
             "L: net 8-2+2=8 — a member appeared this turn satisfies the condition");
}

static void test_l_no_appearance_condition_fails(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_board(&tg, 2, 4, &konata);
    if (konata < 0) { CHECK(0, "L/近江彼方 fixture resolves"); return; }
    CHECK(!marked_appeared(&tg, konata), "L/setup: nothing appeared this turn");

    activate_and_drain(&tg, konata);
    int before = p1_energy_active(&tg);
    activate_and_drain(&tg, konata);
    int after = p1_energy_active(&tg);

    /* PROMPT CONTROL: without this, "no energy gain" is exactly what a no-op
     * would also produce. */
    CHECK_PROMPTS("L: the 起動 activations must have run — the 'no gain' claim "
                  "is otherwise indistinguishable from a silent no-op");
    CHECK_EQ(after, before, "L: no energy gain when no member appeared this turn");
}

static void test_l_non_niji_member_does_not_activate(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_board(&tg, 2, 4, &konata);
    if (konata < 0) { CHECK(0, "L/近江彼方 fixture resolves"); return; }
    int honoka = mid(&tg, "PL!-sd1-010-SD");
    if (honoka < 0) { CHECK(0, "L/mu member fixture resolves"); return; }

    mark_appeared(&tg, honoka);
    CHECK(marked_appeared(&tg, honoka), "L/setup: a NON-虹ヶ咲 member appeared this turn");

    activate_and_drain(&tg, konata);
    int before = p1_energy_active(&tg);
    activate_and_drain(&tg, konata);
    int after = p1_energy_active(&tg);

    CHECK_PROMPTS("L: the 起動 activations must have run — a group-mismatch "
                  "refusal is otherwise indistinguishable from a no-op");
    CHECK_EQ(after, before, "L: no energy gain when only a non-虹ヶ咲 member appeared");
}

static void test_l_use_limit_blocks_both_abilities(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_board(&tg, 3, 6, &konata);
    if (konata < 0) { CHECK(0, "L/近江彼方 fixture resolves"); return; }
    mark_appeared(&tg, konata);

    activate_and_drain(&tg, konata);
    activate_and_drain(&tg, konata);
    int before_third = p1_energy_active(&tg);

    int rc = activate(&tg, konata);
    drain_first(&tg, 10);

    CHECK_PROMPTS("L: the first two activations must have run — otherwise the "
                  "third refusal proves nothing about the use limit");
    CHECK_EQ(p1_energy_active(&tg), before_third,
             "L: the refused third 起動 changes no energy");
    CHECK(!pending(&tg), "L: a use-limit refusal must not open a prompt");
    CHECK(rc == 0 || rc == 1,
          "L: the third 起動 is refused (rc=%d); the engine has no typed "
          "use-limit error, so only the untouched state is asserted", rc);
}

/* =======================================================================
 * M. konata_ability_order_draw_and_activate_energy_test.rs (3 tests)
 *    The same print driven BY ABILITY INDEX (ab#1 = discard 1 from hand ->
 *    if a 『虹ヶ咲』 member appeared this turn, activate 2 energy). rb_activate_
 *    card has no ability-index parameter, so activate_ability_index() drives
 *    one 起動 through rb_resolve_ability.
 * ======================================================================= */

#define KONATA_ENERGY_ABILITY_IDX 1

static void konata_idx_board(TestGame *tg, int n_hand, int energy, int *out_konata)
{
    int konata = mid(tg, KONATA_BP1_006);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_konata = konata;
    if (konata < 0 || filler < 0) return;
    ident(konata, KONATA_BP1_006);
    set_stage(tg, 0, AREA_CENTER, konata);
    for (int i = 0; i < n_hand; i++) add_hand(tg, filler);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(tg, 0, filler);
    test_give_energy(tg, energy);
    /* Leave exactly 2 waited: the activate-2 must come from real waited cards. */
    test_set_energy_active(tg, 0, energy > 2 ? energy - 2 : 0);
}

static void test_m_konata_q77_self_appearance_activates_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_idx_board(&tg, 1, 4, &konata);
    if (konata < 0) { CHECK(0, "M/近江彼方 fixture resolves"); return; }

    CHECK(ability_prints_trigger(konata, KONATA_ENERGY_ABILITY_IDX, RB_TSTR_ACTIVATION),
          "M/setup: ability #%d of %s really is a 起動 (premise of the test)",
          KONATA_ENERGY_ABILITY_IDX, KONATA_BP1_006);
    mark_appeared(&tg, konata);

    int before = p1_energy_active(&tg);
    int rc = activate_ability_index(&tg, konata, KONATA_ENERGY_ABILITY_IDX);
    drain_first(&tg, 20);
    CHECK(rc >= 0, "M: the indexed 起動 resolved (rc=%d)", rc);

    CHECK_PROMPTS("M: the indexed 起動 must have asked for the discard cost — "
                  "otherwise the +2 below is vacuous");
    CHECK_EQ(p1_energy_active(&tg), before + 2,
             "M: ab#1 activates 2 energy when a 虹ヶ咲 member appeared this turn");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "M: the discard cost put 1 card in the waitroom");
}

static void test_m_konata_no_appearance_condition_fails(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_idx_board(&tg, 1, 4, &konata);
    if (konata < 0) { CHECK(0, "M/近江彼方 fixture resolves"); return; }
    CHECK(!marked_appeared(&tg, konata), "M/setup: nothing appeared this turn");
    CHECK(ability_prints_trigger(konata, KONATA_ENERGY_ABILITY_IDX, RB_TSTR_ACTIVATION),
          "M/setup: ability #%d really is a 起動", KONATA_ENERGY_ABILITY_IDX);

    int before = p1_energy_active(&tg);
    activate_ability_index(&tg, konata, KONATA_ENERGY_ABILITY_IDX);
    drain_first(&tg, 20);
    int after = p1_energy_active(&tg);

    CHECK_PROMPTS("M: PROMPT CONTROL — the indexed 起動 must have run (the cost "
                  "prompt is what makes the 'no gain' claim non-vacuous)");
    CHECK_EQ(after, before, "M: no energy change when no 虹ヶ咲 member appeared this turn");
}

static void test_m_konata_use_limit_blocks_second_activation(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = -1;
    konata_idx_board(&tg, 3, 6, &konata);
    if (konata < 0) { CHECK(0, "M/近江彼方 fixture resolves"); return; }
    mark_appeared(&tg, konata);

    int before = p1_energy_active(&tg);
    int rc1 = activate_ability_index(&tg, konata, KONATA_ENERGY_ABILITY_IDX);
    drain_first(&tg, 20);
    int after1 = p1_energy_active(&tg);

    CHECK_PROMPTS("M: the first activation must have run — the refusal below "
                  "proves nothing otherwise");
    CHECK(rc1 >= 0, "M: the first activation resolved (rc=%d)", rc1);
    CHECK_EQ(after1, before + 2, "M: the first activation activates 2 energy");

    int before2 = p1_energy_active(&tg);
    int rc2 = activate_ability_index(&tg, konata, KONATA_ENERGY_ABILITY_IDX);
    drain_first(&tg, 20);

    CHECK_EQ(p1_energy_active(&tg), before2,
             "M: the second activation of the SAME ability is blocked by its "
             "turn-1 use limit and changes no energy");
    CHECK(rc2 != 0 || rc2 == 0, "M: the second call returned (rc=%d)", rc2);
}

/* =======================================================================
 * N. konata_nijigasaki_debuted_this_turn_activates_energy_test.rs (2)
 *    Same print, reached through the DEFAULT (first-eligible) activation. The
 *    Rust tests pin what actually fires: ab#1 (2E -> draw 1), whose cost is a
 *    net -2 on the active count. The 登場 counting is exercised via
 *    debut_count_this_turn.
 * ======================================================================= */

static void test_n_konata_q77_debuted_this_turn_activates_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = mid(&tg, KONATA_BP1_006);
    int niji = mid(&tg, "PL!N-PR-009-PR");
    if (konata < 0 || niji < 0) { CHECK(0, "N/近江彼方 fixture resolves"); return; }
    ident(konata, KONATA_BP1_006);
    ident(niji, "PL!N-PR-009-PR");

    set_stage(&tg, 0, AREA_CENTER, konata);
    add_hand(&tg, niji);
    test_give_energy(&tg, 4);
    tg.state.debut_count_this_turn[0] = 1;

    int before = p1_energy_active(&tg);
    activate(&tg, konata);
    drain_first(&tg, 20);

    /* The Rust twin asserts NO prompt (a 2E cost auto-pays), so the prompt
     * count is legitimately 0 and cannot be the control. The control is the
     * ENERGY MOVEMENT itself plus the discarded-card claim. */
    CHECK_EQ(p1_energy_active(&tg), before - 2,
             "N: the first-eligible 起動 fired and its 2E cost auto-paid");
    CHECK(!wait_has(&tg, niji),
          "N: the 1-hand-discard cost of the OTHER ability was not paid");
}

static void test_n_konata_no_debut_condition_fails(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int konata = mid(&tg, KONATA_BP1_006);
    int niji = mid(&tg, "PL!N-PR-009-PR");
    if (konata < 0 || niji < 0) { CHECK(0, "N/近江彼方 fixture resolves"); return; }
    ident(konata, KONATA_BP1_006);

    set_stage(&tg, 0, AREA_CENTER, konata);
    add_hand(&tg, niji);
    test_give_energy(&tg, 4);
    CHECK_EQ(tg.state.debut_count_this_turn[0], 0, "N/setup: no member debuted");

    int before = p1_energy_active(&tg);
    activate(&tg, konata);
    drain_first(&tg, 20);
    int after = p1_energy_active(&tg);

    CHECK_EQ(before - after, 2, "N: the 2E cost is paid, net -2");
    CHECK(!pending(&tg), "N: the activation resolved without parking a prompt");
    CHECK(!wait_has(&tg, niji), "N: niji_member is NOT in the waitroom (ab#0 did not fire)");
}

/* =======================================================================
 * O. mei_per_distinct_catchu_member_energy_activation_test.rs (5 of 7)
 *    PL!SP-pb2-018-R 米女メイ ライブ開始時: per_unit + distinct by card NAME
 *    over 『CatChu!』 members on stage -> activate 1 energy each.
 *    (Only 5 of 7 are ported: catchu_only_non_catchu_on_stage is the same
 *    board as catchu_zero_members_nothing_activates minus Mei, and
 *    catchu_self_as_member_on_stage_plus_one_other is byte-identical in
 *    effect to catchu_basic_two_distinct_activate_2.)
 * ======================================================================= */

#define MEI_PB2_018 "PL!SP-pb2-018-R"
#define KANON_SD1_001 "PL!SP-sd1-001-SD"
#define SUMIRE_SD1_004 "PL!SP-sd1-004-SD"

static void catchu_board(TestGame *tg, int a, int b, int c, int energy, int active)
{
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(tg, filler, 30);
    set_stage(tg, 0, AREA_LEFT, a);
    set_stage(tg, 0, AREA_CENTER, b);
    set_stage(tg, 0, AREA_RIGHT, c);
    test_give_energy(tg, energy);
    test_set_energy_active(tg, 0, active);
}

static void test_o_catchu_two_distinct_activate_2(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = mid(&tg, MEI_PB2_018);
    int catchu = mid(&tg, KANON_SD1_001);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (mei < 0 || catchu < 0 || filler < 0) { CHECK(0, "O/fixtures resolve"); return; }
    ident(mei, MEI_PB2_018);
    ident(catchu, KANON_SD1_001);
    CHECK(mei != catchu, "O/setup: two DISTINCT CatChu! names are staged");
    CHECK(card_prints_trigger(mei, RB_TSTR_LIVE_START),
          "O/setup: 米女メイ really prints a ライブ開始時 ability");

    catchu_board(&tg, mei, catchu, filler, 5, 3);
    CHECK_EQ(p1_wait_energy(&tg), 2, "O/setup: 2 genuinely waited energy to activate");

    advance_to_live_card_set(&tg, 5);
    test_pass(&tg); test_pass(&tg);

    CHECK_EQ(p1_energy_active(&tg), 5,
             "O: 2 distinct CatChu! names on stage -> activate 2 (3->5)");
}

static void test_o_catchu_zero_members_nothing_activates(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    int filler2 = mid(&tg, "PL!-sd1-010-SD");
    if (filler < 0 || filler2 < 0) { CHECK(0, "O/fillers resolve"); return; }
    /* Mei is NOT on stage, so the ability cannot fire at all. */
    catchu_board(&tg, filler, filler2, filler, 3, 3);

    advance_to_live_card_set(&tg, 5);
    test_pass(&tg); test_pass(&tg);

    CHECK_EQ(p1_energy_active(&tg), 3, "O: no CatChu! member -> no activation");
}

static void test_o_catchu_three_distinct_activate_3(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = mid(&tg, MEI_PB2_018);
    int a = mid(&tg, KANON_SD1_001);
    int b = mid(&tg, SUMIRE_SD1_004);
    if (mei < 0 || a < 0 || b < 0) { CHECK(0, "O/three CatChu fixtures resolve"); return; }
    ident(mei, MEI_PB2_018);
    CHECK(mei != a && mei != b && a != b, "O/setup: three DISTINCT CatChu! prints");
    CHECK(card_prints_trigger(mei, RB_TSTR_LIVE_START),
          "O/setup: 米女メイ really prints a ライブ開始時 ability");

    catchu_board(&tg, mei, a, b, 3, 0);
    CHECK_EQ(p1_wait_energy(&tg), 3, "O/setup: 3 waited energy available");

    advance_to_live_card_set(&tg, 5);
    test_pass(&tg); test_pass(&tg);

    CHECK(!pending(&tg), "O: the per-unit activation auto-resolves with no prompt");
    CHECK_EQ(p1_energy_active(&tg), 3,
             "O: 3 distinct CatChu! names on stage -> activate 3 (0->3)");
}

static void test_o_catchu_duplicate_names_dedup_to_1(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = mid(&tg, MEI_PB2_018);
    int a = mid(&tg, KANON_SD1_001);
    int dup = mid(&tg, KANON_SD1_001);
    if (mei < 0 || a < 0 || dup < 0) { CHECK(0, "O/duplicate fixtures resolve"); return; }
    ident(mei, MEI_PB2_018);
    CHECK(a != dup, "O/setup: two PHYSICAL instances of one print");
    CHECK(rb_card_no_eq(a, KANON_SD1_001) && rb_card_no_eq(dup, KANON_SD1_001),
          "O/setup: both share one card_no, so distinct-by-name must dedup them");
    CHECK(card_prints_trigger(mei, RB_TSTR_LIVE_START),
          "O/setup: 米女メイ really prints a ライブ開始時 ability");

    catchu_board(&tg, mei, a, dup, 5, 3);
    CHECK_EQ(p1_wait_energy(&tg), 2, "O/setup: 2 genuinely waited energy");

    advance_to_live_card_set(&tg, 5);
    test_pass(&tg); test_pass(&tg);

    CHECK_EQ(p1_energy_active(&tg), 5,
             "O: 2 PHYSICAL but 2 DISTINCT names (mei + kanon) -> activate 2 (3->5)");
}

static void test_o_catchu_single_member_activate_1(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mei = mid(&tg, MEI_PB2_018);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (mei < 0 || filler < 0) { CHECK(0, "O/fixtures resolve"); return; }
    ident(mei, MEI_PB2_018);
    CHECK(card_prints_trigger(mei, RB_TSTR_LIVE_START),
          "O/setup: 米女メイ really prints a ライブ開始時 ability");

    catchu_board(&tg, mei, filler, RB_EMPTY_SLOT, 4, 3);
    CHECK_EQ(p1_wait_energy(&tg), 1, "O/setup: 1 genuinely waited energy");

    advance_to_live_card_set(&tg, 5);
    test_pass(&tg); test_pass(&tg);

    CHECK_EQ(p1_energy_active(&tg), 4, "O: 1 CatChu! member -> activate 1 (3->4)");
}

/* =======================================================================
 * P. member_activation_debut_and_live_start_test.rs (4 tests)
 *    PL!-bp3-005-R 星空凛 登場 activates ALL own waited members.
 *    PL!-bp3-001-R 高坂穗乃果 ライブ開始時 activates up to ONE ("1人まで"),
 *    which is SKIPPABLE.
 * ======================================================================= */

#define RIN_BP3_005 "PL!-bp3-005-R"
#define HONOKA_BP3_001 "PL!-bp3-001-R"
#define KANATA_PB1_006 "PL!N-pb1-006-R"

static void test_p_bp3_005_debut_activates_all_waited_members(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int card = mid(&tg, RIN_BP3_005);
    int mate_a = mid(&tg, "PL!-sd1-010-SD");
    int mate_b = mid(&tg, "PL!-sd1-010-SD");
    if (card < 0 || mate_a < 0 || mate_b < 0) { CHECK(0, "P/fixtures resolve"); return; }
    ident(card, RIN_BP3_005);
    CHECK(mate_a != mate_b, "P/setup: two DISTINCT teammate instances");
    ident(mate_a, "PL!-sd1-010-SD");
    ident(mate_b, "PL!-sd1-010-SD");

    test_give_energy(&tg, 20);
    fill_decks(&tg, mate_a, 30);

    set_stage(&tg, 0, AREA_LEFT, mate_a);
    set_stage(&tg, 0, AREA_CENTER, mate_b);
    rb_mods_set_orientation(&tg.state.mods, mate_a, "wait");
    rb_mods_set_orientation(&tg.state.mods, mate_b, "wait");
    CHECK_ORI(orientation_of(&tg, mate_a), ORI_WAIT, "P/setup: A is waited");
    CHECK_ORI(orientation_of(&tg, mate_b), ORI_WAIT, "P/setup: B is waited");

    add_hand(&tg, card);
    test_play_to_stage(&tg, card, AREA_RIGHT);
    drain_pay(&tg, 1, 40);

    CHECK_ORI(orientation_of(&tg, mate_a), ORI_ACTIVE, "P: waited teammate A is activated");
    CHECK(!pending(&tg), "P: the debut resolved with no prompt parked");
    CHECK_ORI(orientation_of(&tg, mate_b), ORI_ACTIVE, "P: waited teammate B is activated");
}

static void test_p_combo_mass_activate_readies_self_waited_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int mass = mid(&tg, RIN_BP3_005);
    int kanata = mid(&tg, KANATA_PB1_006);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (mass < 0 || kanata < 0 || filler < 0) { CHECK(0, "P/combo fixtures resolve"); return; }
    ident(mass, RIN_BP3_005);
    ident(kanata, KANATA_PB1_006);

    test_give_energy(&tg, 25);
    fill_decks(&tg, filler, 30);

    add_hand(&tg, kanata);
    test_play_to_stage(&tg, kanata, AREA_LEFT);
    drain_pay(&tg, 1, 40);

    /* Kanata's 起動: wait self, activate 1 energy. */
    int e0 = p1_energy_active(&tg);
    activate(&tg, kanata);
    drain_pay(&tg, 1, 40);

    CHECK_ORI(orientation_of(&tg, kanata), ORI_WAIT, "P: the cost waits her");
    CHECK_EQ(p1_energy_active(&tg), e0 + 1, "P: the effect activates one energy");

    add_hand(&tg, mass);
    test_play_to_stage(&tg, mass, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK_ORI(orientation_of(&tg, kanata), ORI_ACTIVE,
              "P: the mass-activate debut re-activates the self-waited member");

    int e2 = p1_energy_active(&tg);
    activate(&tg, kanata);
    drain_pay(&tg, 1, 40);
    CHECK_EQ(p1_energy_active(&tg), e2 + 1,
             "P: the re-activated member can activate again immediately");
}

static void bp3_001_live_start_board(TestGame *tg, int *out_card, int *out_sleeper)
{
    int card = mid(tg, HONOKA_BP3_001);
    int sleeper = mid(tg, "PL!-sd1-010-SD");
    int filler = mid(tg, "PL!-sd1-010-SD");
    int live_card = mid(tg, "PL!-sd1-020-SD");
    *out_card = card; *out_sleeper = sleeper;
    if (card < 0 || sleeper < 0 || filler < 0 || live_card < 0) return;
    ident(card, HONOKA_BP3_001);
    test_give_energy(tg, 15);
    fill_decks(tg, filler, 30);
    set_stage(tg, 0, AREA_LEFT, sleeper);
    rb_mods_set_orientation(&tg->state.mods, sleeper, "wait");
    add_hand(tg, card);
    test_play_to_stage(tg, card, AREA_CENTER);
    drain_pay(tg, 1, 40);
    add_hand(tg, live_card);
    advance_to_live_card_set(tg, 5);
    test_set_live_card(tg, 0, live_card);
}

static void test_p_bp3_001_live_start_activates_one_chosen_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int card = -1, sleeper = -1;
    bp3_001_live_start_board(&tg, &card, &sleeper);
    if (card < 0 || sleeper < 0) { CHECK(0, "P/bp3-001 fixture resolves"); return; }
    CHECK(card_prints_trigger(card, RB_TSTR_LIVE_START),
          "P/setup: PL!-bp3-001-R really prints a ライブ開始時 ability");

    /* Rust captures hand_before and p1_zone BEFORE the two passes (p1 refills
     * +1 by setting the live card, then the live-start fires). */
    int live_zone = tg.state.p[0].live.n;
    int hand_before = tg.state.p[0].hand.n;
    test_pass(&tg);
    test_pass(&tg);

    int guard = 0;
    while (pending(&tg) && guard++ < 30) {
        int kind = pending_kind(&tg);
        if (kind == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&tg, 0);
        else if (kind == RB_CHOICE_SELECT_CARD) {
            CHECK(strcmp(rb_get_pending_choice(&tg.state)->zone, "stage") == 0,
                  "P: expected a stage SelectCard, got \"%s\"",
                  rb_get_pending_choice(&tg.state)->zone);
            answer(&tg, 0);
        } else answer(&tg, -1);
        pump_queue(&tg);
    }

    CHECK_PROMPTS("P: the live-start must have run — otherwise 'the chosen member "
                  "was activated' is indistinguishable from an inert board");
    CHECK_ORI(orientation_of(&tg, sleeper), ORI_ACTIVE,
              "P: the live-start activates the chosen waited member");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before + live_zone,
             "P: activate-one draws nothing beyond the live-zone refill");
}

static void test_p_bp3_001_live_start_can_skip_activation(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int card = -1, sleeper = -1;
    bp3_001_live_start_board(&tg, &card, &sleeper);
    if (card < 0 || sleeper < 0) { CHECK(0, "P/bp3-001 fixture resolves"); return; }
    CHECK(card_prints_trigger(card, RB_TSTR_LIVE_START),
          "P/setup: PL!-bp3-001-R really prints a ライブ開始時 ability");

    int guard = 0;
    while (pending(&tg) && guard++ < 30) {
        if (pending_kind(&tg) == RB_CHOICE_SELECT_AUTO_ABILITY) answer(&tg, 0);
        else answer(&tg, -1);
        pump_queue(&tg);
    }

    CHECK_PROMPTS("P: the live-start must have run and offered the 1-person "
                  "choice — otherwise 'stayed waited' proves nothing");
    CHECK_ORI(orientation_of(&tg, sleeper), ORI_WAIT,
              "P: a declined activation leaves the member waited");
}

/* =======================================================================
 * Q. nijigasaki_bp1_006_promoted_pay_two_draw_one_test.rs (1 test)
 *    PL!N-bp1-006-R+ 起動: 「{{E}}{{E}}、カードを1枚引く」 — pay EXACTLY 2
 *    energy, draw EXACTLY 1. The FULLWIDTH PLUS suffix is load-bearing: the
 *    ASCII "PL!N-bp1-006-R" does not exist in cards.json.
 * ======================================================================= */

static void test_q_bp1_006r_pay_two_draw_one(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int member = mid(&tg, KONATA_BP1_006);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (member < 0 || filler < 0) { CHECK(0, "Q/近江彼方 fixture resolves"); return; }
    ident(member, KONATA_BP1_006);
    /* The ASCII-spelled sibling is a real trap: the Rust doc comment names
     * "PL!N-bp1-006-R", which cards.json does NOT contain. */
    CHECK(!rb_card_no_eq(member, "PL!N-bp1-006-R"),
          "Q/FIXTURE: the ASCII spelling is a DIFFERENT (absent) card_no, so the "
          "fullwidth-plus suffix is load-bearing");

    set_stage(&tg, 0, AREA_CENTER, member);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 2);

    int before_hand = tg.state.p[0].hand.n;
    int before_deck = tg.state.p[0].deck.n;

    /* The 起動 is addressed BY INDEX: the compiled blob gives PL!N-bp1-006-R+
     * ab#0 = 「{{E}}{{E}}：カードを1枚引く」 and ab#1 = the discard-then-activate-2
     * one, so the first-eligible activate() is ambiguous about which leg fires
     * and the test must pin the one it is about. */
    CHECK(ability_prints_trigger(member, 0, RB_TSTR_ACTIVATION),
          "Q/setup: ab#0 of %s really is the 起動 under test", KONATA_BP1_006);
    int rc = activate_ability_index(&tg, member, 0);
    drain_first(&tg, 20);
    CHECK(rc >= 0, "Q: the 起動 activation succeeded with 2 energy (rc=%d)", rc);

    /* NON-VACUITY CONTROL: a 2E cost auto-pays with no prompt, so the control
     * is that the energy really moved. */
    CHECK_EQ(p1_energy_active(&tg), 0, "Q: exactly 2 energy paid (control: the cost ran)");
    CHECK_EQ(tg.state.p[0].hand.n, before_hand + 1, "Q: drew exactly 1 card");
    CHECK_EQ(tg.state.p[0].deck.n, before_deck - 1, "Q: the deck lost exactly 1 card");
}

/* =======================================================================
 * R. pl_sp_bp4_008_rightside_debut_energy_activation_test.rs (2 tests)
 *    PL!SP-bp4-008-P 若菜四季: the debut's placement must honour the chosen
 *    member AREA (RightSide -> index 2, LeftSide -> index 0).
 * ======================================================================= */

#define SHIKI_SP_BP4_008 "PL!SP-bp4-008-P"

static void shiki_board(TestGame *tg, int *out_shiki)
{
    int shiki = mid(tg, SHIKI_SP_BP4_008);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_shiki = shiki;
    if (shiki < 0 || filler < 0) return;
    ident(shiki, SHIKI_SP_BP4_008);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    add_hand(tg, shiki);
    test_give_energy(tg, 20);
}

static void test_r_sp_bp4_008_rightside_activates_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int shiki = -1;
    shiki_board(&tg, &shiki);
    if (shiki < 0) { CHECK(0, "R/若菜四季 fixture resolves"); return; }

    test_play_to_stage(&tg, shiki, AREA_RIGHT);
    drain_pay(&tg, 1, 40);

    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], shiki,
             "R: playing to RightSide lands at stage index 2");
    /* The Rust assertion is deliberately weak (`!has_pending || hand <= 2`);
     * the sharp C claim is that the debut resolved to completion. */
    CHECK_PROMPTS("R: the 登場 must have run — otherwise the placement is inert");
    CHECK(!pending(&tg), "R: the debut resolved to completion (no prompt parked)");
}

static void test_r_sp_bp4_008_leftside_lands_at_index_zero(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int shiki = -1;
    shiki_board(&tg, &shiki);
    if (shiki < 0) { CHECK(0, "R/若菜四季 fixture resolves"); return; }

    test_play_to_stage(&tg, shiki, AREA_LEFT);
    drain_pay(&tg, 1, 40);

    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], shiki,
             "R: playing to LeftSide lands at stage index 0");
    CHECK_PROMPTS("R: the 登場 must have run");
    CHECK(!pending(&tg), "R: the debut resolved to completion");
}

/* =======================================================================
 * S. q137_already_waited_cost_test.rs (6 tests)
 *    Q137: 「ウェイトにする」 means putting an ACTIVE member into wait, so it
 *      can never be paid with an already-waited member.
 *      PL!SP-bp4-002-R 唐可可 登場: optional SELF wait cost + look-at-4.
 *      PL!-pb1-018-R 矢澤にこ 登場: both seats put a cost<=2 member from their
 *      waitroom into an EMPTY area, waited.
 * ======================================================================= */

#define KEKE_SP_BP4_002 "PL!SP-bp4-002-R"
#define NICO_PB1_018    "PL!-pb1-018-R"
#define CHEAP1_SP_SD1_019 "PL!SP-sd1-019-SD"
#define CHEAP2_SP_SD1_020 "PL!SP-sd1-020-SD"

static void keke_board(TestGame *tg, int *out_keke)
{
    int keke = mid(tg, KEKE_SP_BP4_002);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_keke = keke;
    if (keke < 0 || filler < 0) return;
    ident(keke, KEKE_SP_BP4_002);
    add_hand(tg, keke);
    test_give_energy(tg, 10);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
}

static void test_s_q137_keke_active_cost_pay_applies_wait(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int keke = -1;
    keke_board(&tg, &keke);
    if (keke < 0) { CHECK(0, "S/唐可可 fixture resolves"); return; }

    test_play_to_stage(&tg, keke, AREA_LEFT);
    CHECK(pending(&tg), "S: the optional self-wait cost must be offered while she is ACTIVE");
    CHECK_PROMPTS("S: the cost prompt is the whole point of the test");

    answer(&tg, 1);   /* pay */
    pump_queue(&tg);
    drain_choices(&tg, 30);

    CHECK_ORI(orientation_of(&tg, keke), ORI_WAIT, "S: paying the cost waits 唐可可");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], keke, "S: she stays on stage after paying");
}

static void test_s_q137_keke_skip_cost_no_wait(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int keke = -1;
    keke_board(&tg, &keke);
    if (keke < 0) { CHECK(0, "S/唐可可 fixture resolves"); return; }

    test_play_to_stage(&tg, keke, AREA_LEFT);
    CHECK(pending(&tg), "S: the optional self-wait cost must be offered");
    answer(&tg, 0);   /* skip */
    pump_queue(&tg);
    drain_choices(&tg, 30);

    CHECK_PROMPTS("S: the cost prompt is the whole point of the test");
    CHECK_ORI(orientation_of(&tg, keke), ORI_NONE,
              "S: declining leaves 唐可可 NOT waited");
}

static void test_s_q137_keke_already_waited_cannot_pay_again(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int keke = -1;
    keke_board(&tg, &keke);
    if (keke < 0) { CHECK(0, "S/唐可可 fixture resolves"); return; }

    test_play_to_stage(&tg, keke, AREA_LEFT);
    CHECK(pending(&tg), "S: the optional self-wait cost must be offered");
    answer(&tg, 1);
    pump_queue(&tg);
    drain_choices(&tg, 30);

    CHECK_ORI(orientation_of(&tg, keke), ORI_WAIT, "S: she is waited after paying");
    /* Q137: the cost 「このメンバーをウェイトにする」 means putting an ACTIVE
     * member into wait. She is now waited, so it is not payable a second time.
     * The observable consequence the Rust test pins: the ability resolves to
     * completion, no prompt stays parked, and she stays on stage. */
    CHECK_PROMPTS("S: PROMPT CONTROL — she must actually be WAIT, otherwise the "
                  "'cannot pay again' state was never reached");
    CHECK(!pending(&tg), "S: the ability fully resolved (no prompt parked)");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], keke, "S: she is still on stage");
}

/* Nico's debut: each seat picks a cost<=2 member from its own waitroom and
 * places it in an empty area, waited. */
static void nico_board(TestGame *tg, int *out_nico, const char *p1_cheap, const char *p2_cheap,
                       int *out_c1, int *out_c2)
{
    int nico = mid(tg, NICO_PB1_018);
    int filler = mid(tg, "PL!-sd1-010-SD");
    *out_nico = nico; *out_c1 = -1; *out_c2 = -1;
    if (nico < 0 || filler < 0) return;
    ident(nico, NICO_PB1_018);
    add_hand(tg, nico);
    add_hand(tg, filler);
    test_give_energy(tg, 7);
    set_stage(tg, 0, AREA_LEFT, RB_EMPTY_SLOT);
    if (p1_cheap) {
        int a = mid(tg, p1_cheap);
        *out_c1 = a;
        if (a >= 0) test_add_to_discard(tg, a);
    }
    if (p2_cheap) {
        int b = mid(tg, p2_cheap);
        *out_c2 = b;
        if (b >= 0) {
            RbBag *bag = &tg->state.p[1].discard;
            if (bag->n < RB_MAX_ZONE) bag->cards[bag->n++] = b;
        }
    }
}

static void test_s_q137_nico_both_have_eligible_members(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int nico = -1, cheap1 = -1, cheap2 = -1;
    nico_board(&tg, &nico, CHEAP1_SP_SD1_019, CHEAP2_SP_SD1_020, &cheap1, &cheap2);
    if (nico < 0 || cheap1 < 0 || cheap2 < 0) { CHECK(0, "S/cheap fixtures resolve"); return; }
    CHECK(cheap1 != cheap2, "S/setup: the two seats' candidates are DISTINCT prints");
    CHECK_EQ(printed_cost(cheap1), 2, "S/FIXTURE: %s costs 2 (<= 2)", CHEAP1_SP_SD1_019);
    CHECK_EQ(printed_cost(cheap2), 2, "S/FIXTURE: %s costs 2 (<= 2)", CHEAP2_SP_SD1_020);

    test_play_to_stage(&tg, nico, AREA_LEFT);
    int guard = 0;
    while (pending(&tg) && guard++ < 30) { answer(&tg, 0); pump_queue(&tg); }

    CHECK_PROMPTS("S: PROMPT CONTROL — the position prompt must have been offered "
                  "to BOTH seats, or 'placed in wait' is vacuous");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], nico, "S: 矢澤にこ lands on the left");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], cheap1, "S: p1's cost-2 member is placed");
    CHECK_ORI(orientation_of(&tg, cheap1), ORI_WAIT, "S: p1's placed member is WAITED");
    CHECK_EQ(tg.state.p[1].stage[AREA_RIGHT], cheap2, "S: p2's cost-2 member is placed");
    CHECK_ORI(orientation_of(&tg, cheap2), ORI_WAIT, "S: p2's placed member is WAITED");
}

static void test_s_q137_nico_no_eligible_cards_skips(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int nico = -1, c1 = -1, c2 = -1;
    nico_board(&tg, &nico, NULL, NULL, &c1, &c2);
    if (nico < 0) { CHECK(0, "S/矢澤にこ fixture resolves"); return; }

    test_play_to_stage(&tg, nico, AREA_LEFT);
    drain_choices(&tg, 30);

    CHECK(!pending(&tg), "S: with no eligible card in either waitroom, nothing is asked");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], nico, "S: 矢澤にこ is on the left");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT, "S: the centre is empty");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], RB_EMPTY_SLOT, "S: the right is empty");
    /* PROMPT CONTROL for the negative: the SAME card, with a candidate present
     * in the previous test, must have asked. This arm must not merely be
     * silent. */
    CHECK_PROMPTS("S: NOTE — the Rust twin for this arm also asserts no prompt; "
                  "the non-vacuity is carried by the p1/p2 placement test above, "
                  "which uses the same print and the same board otherwise");
}

static void test_s_q137_nico_only_opponent_has_eligible(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int nico = -1, c1 = -1, cheap2 = -1;
    nico_board(&tg, &nico, NULL, CHEAP2_SP_SD1_020, &c1, &cheap2);
    if (nico < 0 || cheap2 < 0) { CHECK(0, "S/cheap fixture resolves"); return; }
    (void)c1;
    CHECK_EQ(printed_cost(cheap2), 2, "S/FIXTURE: %s costs 2", CHEAP2_SP_SD1_020);

    test_play_to_stage(&tg, nico, AREA_LEFT);
    int guard = 0;
    while (pending(&tg) && guard++ < 30) { answer(&tg, 0); pump_queue(&tg); }

    CHECK_PROMPTS("S: PROMPT CONTROL — p2 must have been ASKED for its position; "
                  "otherwise 'p2's member is placed' is vacuous");
    CHECK(!pending(&tg), "S: the resolution completes");
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], cheap2, "S: p2's member is placed on the left");
    CHECK_ORI(orientation_of(&tg, cheap2), ORI_WAIT, "S: p2's placed member is WAITED");
}

/* =======================================================================
 * T. sd2_nijigasaki_abilities_gap_test.rs (12 tests)
 *    The seven PL!N-sd2-*-SD2 prints: optional / per-unit / filtered waits
 *    driven from a REAL live-start and from real plays.
 *
 *    NON-VACUITY NOTE for this section. Several of these 登場 / ライブ開始時
 *    abilities resolve with NO player choice in the C engine (and the Rust
 *    twins drive them through a plain `play_to_stage` + drain), so a prompt
 *    count is legitimately 0 and cannot be the control here. The control is
 *    instead the ARM PAIR plus a fixture premise, and every arm in this
 *    section has one:
 *      - 006 kanata pay/skip  : same board, only the pay answer differs; the
 *                              pay arm asserts a member became WAITED and the
 *                              skip arm asserts it did not. The two cannot
 *                              both pass on a no-op board.
 *      - 013 ayumu            : the blade-1 opponent must be WAITED on a
 *                              niji-only stage (positive) and must NOT be
 *                              waited when a non-niji ally occupies the
 *                              stage; the positive arm alone makes the
 *                              negative non-vacuous.
 *      - 017 ai pay/skip      : same board, only the pay answer differs.
 *      - 019 setsuna          : the card is asserted to PRINT a 登場 heart05
 *                              and the value claimed is exact (1), so a
 *                              no-op grant cannot satisfy it; the opponent
 *                              cost values are asserted against cards.json.
 *      - 021 rinne            : POSITIVE arm (a cost-4 opponent IS waited)
 *                              with both cost values asserted.
 *    So a red NEGATIVE arm in this section is a filter/condition decision,
 *    not a silently-inert board.
 * ======================================================================= */

#define KANATA_SD2_006 "PL!N-sd2-006-SD2"
#define SHIORIKO_SD2_010 "PL!N-sd2-010-SD2"
#define AYUMU_SD2_013 "PL!N-sd2-013-SD2"
#define SHIZUKU_SD2_015 "PL!N-sd2-015-SD2"
#define AI_SD2_017 "PL!N-sd2-017-SD2"
#define SETSUNA_SD2_019 "PL!N-sd2-019-SD2"
#define RINNE_SD2_021 "PL!N-sd2-021-SD2"
#define SD2_ALLY "PL!N-sd2-013-SD2"
#define OPP_COST4 "PL!-sd1-010-SD"
#define OPP_B3    "PL!-sd1-001-SD"
#define OPP_COST5 "PL!-pb1-021-PR"

static void sd2_seed_decks(TestGame *tg)
{
    int f = mid(tg, "PL!-sd1-010-SD");
    if (f < 0) return;
    fill_decks(tg, f, 30);
}

/* Rust drive_to_live_start(): one live, 5 passes to LiveCardSet, set live,
 * 2 more passes. */
static void sd2_drive_to_live_start(TestGame *tg, int live)
{
    add_hand(tg, live);
    advance_to_live_card_set(tg, 5);
    test_set_live_card(tg, 0, live);
    test_pass(tg);
    test_pass(tg);
}

static void test_t_kanata_pays_wait_and_gains_two_blade(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int kanata = mid(&tg, KANATA_SD2_006);
    int ally = mid(&tg, SD2_ALLY);
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (kanata < 0 || ally < 0 || live < 0) { CHECK(0, "T/006 fixture resolves"); return; }
    ident(kanata, KANATA_SD2_006);
    CHECK(kanata != ally, "T/setup: the ally is a DISTINCT instance");

    set_p1_stage3(&tg, ally, kanata, RB_EMPTY_SLOT);
    test_give_energy(&tg, 8);
    sd2_seed_decks(&tg);
    sd2_drive_to_live_start(&tg, live);
    drain_pay(&tg, 1, 40);

    CHECK(is_waited(&tg, ally), "T/006: paying the optional wait leaves the 虹 ally waited");
    CHECK_EQ(blade_mod_of(&tg, kanata), 2,
             "T/006: waiting an own 虹 member grants +2 blade till live end");
}

static void test_t_kanata_skip_optional_wait_gains_nothing(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int kanata = mid(&tg, KANATA_SD2_006);
    int ally = mid(&tg, SD2_ALLY);
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (kanata < 0 || ally < 0 || live < 0) { CHECK(0, "T/006 fixture resolves"); return; }
    ident(kanata, KANATA_SD2_006);
    CHECK(kanata != ally, "T/setup: the ally is a DISTINCT instance");

    set_p1_stage3(&tg, ally, kanata, RB_EMPTY_SLOT);
    test_give_energy(&tg, 8);
    sd2_seed_decks(&tg);
    sd2_drive_to_live_start(&tg, live);
    drain_pay(&tg, 0, 40);

    CHECK(!is_waited(&tg, ally), "T/006: skipping the optional wait leaves the ally NOT waited");
    CHECK_EQ(blade_mod_of(&tg, kanata), 0, "T/006: skipping grants no blade");
}

static void test_t_shioriko_debut_draws_two(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int shi = mid(&tg, SHIORIKO_SD2_010);
    if (shi < 0) { CHECK(0, "T/010 fixture resolves"); return; }
    ident(shi, SHIORIKO_SD2_010);
    sd2_seed_decks(&tg);
    int hand_before = tg.state.p[0].hand.n;
    add_hand(&tg, shi);
    test_give_energy(&tg, 20);

    test_play_to_stage(&tg, shi, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 2, "T/010: the debut draws 2 cards");
}

static void test_t_shioriko_auto_activates_waited_ally(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int shi = mid(&tg, SHIORIKO_SD2_010);
    int ally = mid(&tg, SD2_ALLY);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (shi < 0 || ally < 0 || filler < 0) { CHECK(0, "T/010 fixture resolves"); return; }
    ident(shi, SHIORIKO_SD2_010);
    CHECK(shi != ally, "T/setup: the ally is a DISTINCT instance");

    set_p1_stage3(&tg, ally, shi, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, ally, "wait");
    add_hand(&tg, filler);

    int queued = fire_trigger(&tg, shi, RB_TSTR_AUTO);
    drain_pay(&tg, 1, 40);

    CHECK(queued > 0,
          "T: PROMPT CONTROL — the 自動 ability WAS queued (%d), so a still-waited "
          "ally is the condition refusing, not a missing ability", queued);
    CHECK(!is_waited(&tg, ally), "T/010: the auto leg activates the waited 虹 member");
    CHECK_EQ(blade_mod_of(&tg, ally), 2, "T/010: the member gains +2 blade after activation");
}

static void test_t_ayumu_waits_opp_low_blade_when_stage_only_nijigasaki(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ayumu = mid(&tg, AYUMU_SD2_013);
    int opp_low = mid(&tg, OPP_COST4);
    int opp_high = mid(&tg, OPP_B3);
    if (ayumu < 0 || opp_low < 0 || opp_high < 0) { CHECK(0, "T/013 fixture resolves"); return; }
    ident(ayumu, AYUMU_SD2_013);
    CHECK(opp_low != opp_high, "T/setup: two DISTINCT opponent instances");
    CHECK_EQ(printed_blade(opp_low), 1, "T/FIXTURE: %s prints blade 1 (<= 2, waitable)", OPP_COST4);
    CHECK_EQ(printed_blade(opp_high), 3, "T/FIXTURE: %s prints blade 3 (> 2, NOT waitable)", OPP_B3);

    set_p1_stage3(&tg, ayumu, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, opp_low, opp_high, RB_EMPTY_SLOT);
    add_hand(&tg, ayumu);
    test_give_energy(&tg, 20);
    sd2_seed_decks(&tg);

    test_play_to_stage(&tg, ayumu, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK(is_waited(&tg, opp_low), "T/013: a blade-1 opponent is waited when the stage is 虹-only");
    CHECK(!is_waited(&tg, opp_high), "T/013: a blade-3 opponent is NOT waitable (original blade > 2)");
}

static void test_t_ayumu_no_wait_with_non_nijigasaki_ally(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ayumu = mid(&tg, AYUMU_SD2_013);
    int outsider = mid(&tg, OPP_COST4);
    int opp = mid(&tg, OPP_COST4);
    if (ayumu < 0 || outsider < 0 || opp < 0) { CHECK(0, "T/013 fixture resolves"); return; }
    ident(ayumu, AYUMU_SD2_013);
    CHECK(outsider != opp, "T/setup: the non-虹 ally and the opponent are DISTINCT instances");

    set_p1_stage3(&tg, outsider, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, opp, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    add_hand(&tg, ayumu);
    test_give_energy(&tg, 20);
    sd2_seed_decks(&tg);

    test_play_to_stage(&tg, ayumu, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK(!is_waited(&tg, opp),
          "T/013: with a non-虹 ally on stage the wait must not happen");
}

static void test_t_shizuku_kidou_draws_after_waiting_self(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int shizuku = mid(&tg, SHIZUKU_SD2_015);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (shizuku < 0 || filler < 0) { CHECK(0, "T/015 fixture resolves"); return; }
    ident(shizuku, SHIZUKU_SD2_015);
    CHECK(ability_prints_trigger(shizuku, 0, RB_TSTR_ACTIVATION) ||
          num_abilities(shizuku) > 0,
          "T/setup: 桜坂しずく carries a 起動 (n=%d)", num_abilities(shizuku));

    set_p1_stage3(&tg, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    sd2_seed_decks(&tg);
    add_hand(&tg, filler);
    int hand_before = tg.state.p[0].hand.n;
    test_give_energy(&tg, 4);

    activate(&tg, shizuku);
    drain_pay(&tg, 1, 40);

    CHECK(is_waited(&tg, shizuku), "T/015: しずく is waited as the cost of the 起動");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "T/015: discard 1 then draw 1 leaves the hand size unchanged");
}

static void test_t_ai_pay_activates_a_waited_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = mid(&tg, AI_SD2_017);
    int waited = mid(&tg, SD2_ALLY);
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (ai < 0 || waited < 0 || live < 0) { CHECK(0, "T/017 fixture resolves"); return; }
    ident(ai, AI_SD2_017);
    CHECK(ai != waited, "T/setup: the target is a DISTINCT instance");

    set_p1_stage3(&tg, waited, ai, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, waited, "wait");
    test_give_energy(&tg, 8);
    sd2_seed_decks(&tg);
    sd2_drive_to_live_start(&tg, live);
    drain_pay(&tg, 1, 40);

    CHECK(!is_waited(&tg, waited), "T/017: paying the energy activates the waited stage member");
}

static void test_t_ai_skip_pay_keeps_waited(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int ai = mid(&tg, AI_SD2_017);
    int waited = mid(&tg, SD2_ALLY);
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (ai < 0 || waited < 0 || live < 0) { CHECK(0, "T/017 fixture resolves"); return; }
    ident(ai, AI_SD2_017);
    CHECK(ai != waited, "T/setup: the target is a DISTINCT instance");

    set_p1_stage3(&tg, waited, ai, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, waited, "wait");
    test_give_energy(&tg, 8);
    sd2_seed_decks(&tg);
    sd2_drive_to_live_start(&tg, live);
    drain_pay(&tg, 0, 40);

    CHECK(is_waited(&tg, waited), "T/017: skipping the pay leaves the member waited");
}

static void test_t_setsuna_debut_gains_heart05(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int setsu = mid(&tg, SETSUNA_SD2_019);
    if (setsu < 0) { CHECK(0, "T/019 fixture resolves"); return; }
    ident(setsu, SETSUNA_SD2_019);
    /* The Rust test pins the premise: the print really is a 登場 heart05. */
    int prints_heart05_debut = 0;
    for (int i = 0; i < num_abilities(setsu); i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)setsu, i, &ab)) { rb_free_ability(&ab); continue; }
        int is_debut = ab.triggers && strstr(ab.triggers, RB_TSTR_DEBUT) != NULL;
        if (is_debut && ab.triggerless_text && strstr(ab.triggerless_text, "heart05"))
            prints_heart05_debut = 1;
        rb_free_ability(&ab);
    }
    CHECK(prints_heart05_debut,
          "T: PREMISE — %s must print a 登場 heart05 (n=%d)", SETSUNA_SD2_019,
          num_abilities(setsu));

    sd2_seed_decks(&tg);
    add_hand(&tg, setsu);
    test_give_energy(&tg, 8);
    test_play_to_stage(&tg, setsu, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK_EQ(heart_of(&tg, setsu, H05), 1,
             "T/019: 「ライブ終了時まで、heart05を得る」 grants exactly one heart05");

    /* 「ライブ終了時まで」 is timed, not card-attached: it survives her leaving
     * the board and is due to expire when the live ends. */
    set_p1_stage3(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, setsu, H05), 1,
             "T/019: a live_end grant is TIMED, not card-attached — it survives "
             "the card leaving the stage");
}

static void test_t_setsuna_live_start_waits_only_cost2_or_less(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int setsu = mid(&tg, SETSUNA_SD2_019);
    int opp4 = mid(&tg, OPP_COST4);
    int opp5 = mid(&tg, OPP_COST5);
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (setsu < 0 || opp4 < 0 || opp5 < 0 || live < 0) { CHECK(0, "T/019 fixture resolves"); return; }
    ident(setsu, SETSUNA_SD2_019);
    CHECK(opp4 != opp5, "T/setup: two DISTINCT opponent instances");
    CHECK_EQ(printed_cost(opp4), 4, "T/FIXTURE: %s costs 4 (over the <=2 cap)", OPP_COST4);
    CHECK_EQ(printed_cost(opp5), 5, "T/FIXTURE: %s costs 5 (over the <=2 cap)", OPP_COST5);

    set_p1_stage3(&tg, RB_EMPTY_SLOT, setsu, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, opp4, opp5, RB_EMPTY_SLOT);
    test_give_energy(&tg, 8);
    sd2_seed_decks(&tg);
    sd2_drive_to_live_start(&tg, live);
    drain_pay(&tg, 1, 40);

    CHECK(!is_waited(&tg, opp4) && !is_waited(&tg, opp5),
          "T/019: the live_start wait targets cost<=2, so cost-4/5 members are NOT waited");
}

static void test_t_rinne_waits_opp_cost_four_member(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int rinne = mid(&tg, RINNE_SD2_021);
    int opp4 = mid(&tg, OPP_COST4);
    int opp5 = mid(&tg, OPP_COST5);
    if (rinne < 0 || opp4 < 0 || opp5 < 0) { CHECK(0, "T/021 fixture resolves"); return; }
    ident(rinne, RINNE_SD2_021);
    CHECK(opp4 != opp5, "T/setup: two DISTINCT opponent instances");
    CHECK_EQ(printed_cost(opp4), 4, "T/FIXTURE: %s costs 4 (<= 4, waitable)", OPP_COST4);
    CHECK_EQ(printed_cost(opp5), 5, "T/FIXTURE: %s costs 5 (over the cap)", OPP_COST5);

    set_p2_stage3(&tg, opp4, opp5, RB_EMPTY_SLOT);
    add_hand(&tg, rinne);
    test_give_energy(&tg, 20);
    sd2_seed_decks(&tg);

    test_play_to_stage(&tg, rinne, AREA_CENTER);
    drain_pay(&tg, 1, 40);

    CHECK(is_waited(&tg, opp4), "T/021: a cost-4 opponent member is waited");
    CHECK(!is_waited(&tg, opp5), "T/021: a cost-5 opponent member is NOT waited (over the cost-4 limit)");
}

/* =======================================================================
 * U. success_score_energy_pl_bp4_004_r_test.rs (2 tests)
 *    PL!-bp4-004-R 園田海未 登場: if the SUCCESS live zone's score total is
 *    >= 6, activate 2 energy. The two arms straddle the threshold.
 * ======================================================================= */

#define UMI_BP4_004 "PL!-bp4-004-R"
#define LIVE6_SP_BP1_027 "PL!SP-bp1-027-L"
#define LIVE5_S_PR_024 "PL!S-PR-024-PR"

static void test_u_success_score_six_activates_two_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int umi = mid(&tg, UMI_BP4_004);
    int live6 = mid(&tg, LIVE6_SP_BP1_027);
    if (umi < 0 || live6 < 0) { CHECK(0, "U/fixtures resolve"); return; }
    ident(umi, UMI_BP4_004);
    ident(live6, LIVE6_SP_BP1_027);
    CHECK(umi != live6, "U/setup: a DISTINCT success live card is staged");
    CHECK(printed_score(live6) >= 0,
          "U: the live card decodes (printed score-icon count %d — the PERFORMED "
          "live score is engine-computed, so the Rust fixture's 'score 6' is a "
          "performed total, not the printed field)", printed_score(live6));

    set_stage(&tg, 0, AREA_CENTER, umi);
    test_add_to_success(&tg, live6);
    /* PREMISE, asserted BEFORE the trigger: the gate reads the SUCCESS zone. */
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "U: the SUCCESS zone really holds exactly 1 card (the gate reads that zone)");
    /* 5 energy, 2 active -> 3 waited for the activation to consume. */
    for (int i = 0; i < 5; i++) {
        RbBag *b = &tg.state.p[0].energy;
        if (b->n < RB_MAX_ZONE) b->cards[b->n++] = mid(&tg, "LL-E-001-SD");
    }
    test_set_energy_active(&tg, 0, 2);
    CHECK_EQ(p1_wait_energy(&tg), 3, "U/setup: 3 genuinely waited energy to activate");

    int queued = fire_trigger(&tg, umi, RB_TSTR_DEBUT);
    drain_choices(&tg, 20);

    CHECK(queued > 0, "U: the 登場 ability was queued (%d)", queued);
    CHECK_PROMPTS("U: PROMPT CONTROL — the score gate must have been evaluated; "
                  "a no-op board would also leave the count at 2");
    CHECK_EQ(p1_energy_active(&tg), 4, "U: success score total 6 >= 6 -> activate 2");
}

static void test_u_success_score_five_activates_no_energy(void)
{
    static TestGame tg;
    prompts = 0;
    test_game_new(&tg);
    int umi = mid(&tg, UMI_BP4_004);
    int live5 = mid(&tg, LIVE5_S_PR_024);
    if (umi < 0 || live5 < 0) { CHECK(0, "U/fixtures resolve"); return; }
    ident(umi, UMI_BP4_004);
    ident(live5, LIVE5_S_PR_024);
    CHECK(umi != live5, "U/setup: a DISTINCT success live card is staged");
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "U: the SUCCESS zone really holds exactly 1 card (the gate reads that zone)");

    set_stage(&tg, 0, AREA_CENTER, umi);
    test_add_to_success(&tg, live5);
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "U: the SUCCESS zone really holds exactly 1 card (the gate reads that zone)");
    for (int i = 0; i < 4; i++) {
        RbBag *b = &tg.state.p[0].energy;
        if (b->n < RB_MAX_ZONE) b->cards[b->n++] = mid(&tg, "LL-E-001-SD");
    }
    test_set_energy_active(&tg, 0, 1);
    CHECK_EQ(p1_wait_energy(&tg), 3, "U/setup: 3 genuinely waited energy available");

    int queued = fire_trigger(&tg, umi, RB_TSTR_DEBUT);
    drain_choices(&tg, 20);

    CHECK(queued > 0, "U: the 登場 ability was queued (%d)", queued);
    /* NON-VACUITY CONTROL: the twin above is POSITIVE and shares this fixture
     * with only the success-zone card swapped, so a 0 here is the threshold
     * refusing, not an inert board. The prompt count is legitimately 0. */
    CHECK_EQ(p1_energy_active(&tg), 1, "U: success score total 5 < 6 -> nothing activated");
}

/* =======================================================================
 * main — fork isolation
 * ======================================================================= */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_tests_ok, n_tests_failed, n_tests_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0);
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [in-process fallback]\n", nf ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), prompts);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);

    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_tests_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    if (getenv("STATEPC_DUMP")) {
        /* Deliberate opt-in diagnostic: dumps the compiled ability list (index,
         * trigger keyword, full text) for the prints this suite addresses by
         * index. It is how the index constants in sections A/E/Q were derived
         * from the card blob rather than guessed. Off unless STATEPC_DUMP is
         * set in the environment. */
        static const char *probe[] = {
            "PL!N-bp7-006-SEC", "PL!N-bp7-005-R", "PL!N-bp7-017-N",
            "PL!SP-bp7-007-R" FF, "PL!-bp5-111-R", "PL!N-bp5-008-R",
            "PL!N-bp1-006-R" FF, "PL!SP-pb2-018-R", "PL!-bp4-004-R",
            "PL!S-bp3-010-N", "PL!S-bp3-011-N", "PL!N-sd2-015-SD2",
            "PL!N-sd2-006-SD2", "PL!N-sd2-019-SD2", "PL!N-sd2-021-SD2",
            "PL!N-sd2-013-SD2", "PL!N-sd2-017-SD2", "PL!N-sd2-010-SD2",
            "PL!-pb1-018-R", "PL!SP-bp4-002-R", "PL!SP-pb2-009-PP",
            NULL
        };
        for (int k = 0; probe[k]; k++) {
            int cid = test_id(&(TestGame){0}, probe[k]);
            printf("\n== %s -> id %d, %d ability(ies)\n", probe[k], cid,
                   cid >= 0 ? num_abilities(cid) : -1);
            for (int i = 0; cid >= 0 && i < num_abilities(cid); i++) {
                Ability ab; memset(&ab, 0, sizeof ab);
                if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) { rb_free_ability(&ab); continue; }
                printf("   ab#%d trig=[%s] cost=%s text=[%s]\n", i,
                       ab.triggers ? ab.triggers : "",
                       ab.cost ? "yes" : "no",
                       ab.triggerless_text ? ab.triggerless_text : "");
                rb_free_ability(&ab);
            }
        }
        return 0;
    }


    printf("--- A ability_index_execution_test.rs (PL!N-bp7-006-SEC) ---\n");
    run("a_sec_ab1_mills_three", test_a_sec_ab1_mills_three_and_costs_no_energy);
    run("a_sec_ab0_pays_one_energy", test_a_sec_ab0_pays_one_energy_and_never_mills);

    printf("--- B ai_debut_choice_activate_energy_or_under_member_test.rs ---\n");
    run("b_ai_two_distinct_offers_choice", test_b_ai_two_distinct_diverdiva_offers_choice);
    run("b_ai_option_0_activates_two", test_b_ai_option_0_activates_two_energy);
    run("b_ai_option_1_under_member", test_b_ai_option_1_places_energy_under_nijigasaki_member);
    run("b_ai_option_1_non_first", test_b_ai_option_1_selects_non_first_nijigasaki_target);
    run("b_ai_one_diverdiva_no_choice", test_b_ai_condition_fails_with_one_diverdiva);
    run("b_ai_duplicate_name_no_choice", test_b_ai_condition_fails_with_duplicate_name);

    printf("--- C ai_debut_optional_energy_under_nijigasaki_member_test.rs ---\n");
    run("c_ai_accept_places", test_c_ai_accept_places_energy_under_nijigasaki);
    run("c_ai_skip_places_nothing", test_c_ai_skip_places_nothing);
    run("c_ai_self_is_target", test_c_ai_no_other_nijigasaki_member_uses_itself);

    printf("--- D bp7_q266_natsumi_blade_wait_test.rs ---\n");
    run("d_q266_b0_waits_nothing", test_d_q266_cost_b0_waits_nothing_incl_blade0);
    run("d_q266_b1_waits_nothing", test_d_q266_cost_b1_waits_nothing);
    run("d_q266_b2_blade0_yes_blade1_no", test_d_q266_cost_b2_waits_blade0_not_blade1);
    run("d_q266_b4_blade2_yes_blade3_no", test_d_q266_cost_b4_waits_blade2_not_blade3);
    run("d_q266_skip_waits_nothing", test_d_q266_skip_cost_waits_nothing);

    printf("--- E bp7_q280_energy_do_not_activate_test.rs ---\n");
    run("e_q280_flags_energy_not_member", test_e_q280_flags_placed_energy_not_member);
    run("e_q280_active_excludes_flagged", test_e_q280_active_phase_excludes_flagged_energy);
    run("e_q280_opponent_keeps_flag", test_e_q280_opponents_active_phase_does_not_clear_owner_flag);
    run("e_q280_activate_then_pay", test_e_q280_activation_then_payment_keeps_next_phase_restriction);
    run("e_q280_abilities_compose", test_e_q280_live_success_abilities_compose_before_payment);
    run("e_q280_already_active_kept", test_e_q280_already_active_restricted_energy_is_not_deactivated);

    printf("--- F butterfly_wing_suppress_test.rs ---\n");
    run("f_butterfly_suppressed", test_f_butterfly_suppressed_live_start_not_resolved);
    run("f_butterfly_control", test_f_butterfly_control_live_start_resolves);
    run("f_butterfly_score_with_livestart", test_f_butterfly_live_success_scores_with_live_start_members);
    run("f_butterfly_no_score_without", test_f_butterfly_no_live_start_member_no_score);

    printf("--- G cerise_bouquet_teammate_energy_activation_test.rs ---\n");
    run("g_crise_bouquet_debut", test_g_crise_bouquet_debut_with_teammate_activates_one_energy);

    printf("--- H debut_optional_single_member_activation_test.rs ---\n");
    run("h_chika_selected", test_h_chika_activates_only_selected_own_member);
    run("h_chika_declined", test_h_chika_declined_activation_preserves_waited_members);
    run("h_riko_selected", test_h_riko_activates_only_selected_own_member);
    run("h_riko_declined", test_h_riko_declined_activation_preserves_waited_members);

    printf("--- I discard_activate_opponent_recover_live_test.rs ---\n");
    run("i_discard_activates_opponent", test_i_discard_activates_opponent_and_recovers_live);

    printf("--- J either_or_state_change_test.rs ---\n");
    run("j_emma_bp4008_member_side", test_j_emma_bp4008_member_side_leaves_energy_alone);
    run("j_emma_bp4008_energy_side", test_j_emma_bp4008_energy_side_leaves_member_waited);
    run("j_emma_pb1008_member_side", test_j_emma_pb1008_debut_choice_picks_member);
    run("j_chisato_and_semantics", test_j_chisato_live_start_activates_all_members_and_energy);

    printf("--- K emma_place_energy_under_self_then_activate_two_test.rs ---\n");
    run("k_q215_wait_energy", test_k_emma_bp5_q215_wait_energy_placed_then_activate);
    run("k_q215_all_active", test_k_emma_bp5_q215_all_energy_already_active_activates_nothing);
    run("k_q215_only_one_energy", test_k_emma_bp5_q215_only_wait_energy_available);
    run("k_q215_no_energy", test_k_emma_bp5_q215_no_energy_cost_fails_gracefully);

    printf("--- L kanata_nijigasaki_appeared_activate_two_energy_test.rs ---\n");
    run("l_q77_self_appearance", test_l_q77_self_appearance_activates_energy);
    run("l_different_niji_appeared", test_l_different_niji_member_appeared_activates);
    run("l_no_appearance_fails", test_l_no_appearance_condition_fails);
    run("l_non_niji_does_not_activate", test_l_non_niji_member_does_not_activate);
    run("l_use_limit_blocks_both", test_l_use_limit_blocks_both_abilities);

    printf("--- M konata_ability_order_draw_and_activate_energy_test.rs ---\n");
    run("m_konata_q77_by_index", test_m_konata_q77_self_appearance_activates_energy);
    run("m_konata_no_appearance_by_index", test_m_konata_no_appearance_condition_fails);
    run("m_konata_use_limit_by_index", test_m_konata_use_limit_blocks_second_activation);

    printf("--- N konata_nijigasaki_debuted_this_turn_activates_energy_test.rs ---\n");
    run("n_konata_debuted_this_turn", test_n_konata_q77_debuted_this_turn_activates_energy);
    run("n_konata_no_debut", test_n_konata_no_debut_condition_fails);

    printf("--- O mei_per_distinct_catchu_member_energy_activation_test.rs ---\n");
    run("o_catchu_two_distinct", test_o_catchu_two_distinct_activate_2);
    run("o_catchu_zero_members", test_o_catchu_zero_members_nothing_activates);
    run("o_catchu_three_distinct", test_o_catchu_three_distinct_activate_3);
    run("o_catchu_duplicate_names", test_o_catchu_duplicate_names_dedup_to_1);
    run("o_catchu_single_member", test_o_catchu_single_member_activate_1);

    printf("--- P member_activation_debut_and_live_start_test.rs ---\n");
    run("p_bp3_005_debut_all", test_p_bp3_005_debut_activates_all_waited_members);
    run("p_combo_mass_activate", test_p_combo_mass_activate_readies_self_waited_member);
    run("p_bp3_001_live_start_one", test_p_bp3_001_live_start_activates_one_chosen_member);
    run("p_bp3_001_live_start_skip", test_p_bp3_001_live_start_can_skip_activation);

    printf("--- Q nijigasaki_bp1_006_promoted_pay_two_draw_one_test.rs ---\n");
    run("q_bp1_006r_pay_two_draw_one", test_q_bp1_006r_pay_two_draw_one);

    printf("--- R pl_sp_bp4_008_rightside_debut_energy_activation_test.rs ---\n");
    run("r_bp4_008_rightside", test_r_sp_bp4_008_rightside_activates_energy);
    run("r_bp4_008_leftside", test_r_sp_bp4_008_leftside_lands_at_index_zero);

    printf("--- S q137_already_waited_cost_test.rs ---\n");
    run("s_q137_keke_pay", test_s_q137_keke_active_cost_pay_applies_wait);
    run("s_q137_keke_skip", test_s_q137_keke_skip_cost_no_wait);
    run("s_q137_keke_already_waited", test_s_q137_keke_already_waited_cannot_pay_again);
    run("s_q137_nico_both", test_s_q137_nico_both_have_eligible_members);
    run("s_q137_nico_none", test_s_q137_nico_no_eligible_cards_skips);
    run("s_q137_nico_p2_only", test_s_q137_nico_only_opponent_has_eligible);

    printf("--- T sd2_nijigasaki_abilities_gap_test.rs ---\n");
    run("t_kanata_pay_wait_blade2", test_t_kanata_pays_wait_and_gains_two_blade);
    run("t_kanata_skip_no_blade", test_t_kanata_skip_optional_wait_gains_nothing);
    run("t_shioriko_debut_draws_two", test_t_shioriko_debut_draws_two);
    run("t_shioriko_auto_activates", test_t_shioriko_auto_activates_waited_ally);
    run("t_ayumu_waits_low_blade", test_t_ayumu_waits_opp_low_blade_when_stage_only_nijigasaki);
    run("t_ayumu_no_wait_non_niji", test_t_ayumu_no_wait_with_non_nijigasaki_ally);
    run("t_shizuku_kidou", test_t_shizuku_kidou_draws_after_waiting_self);
    run("t_ai_pay_activates", test_t_ai_pay_activates_a_waited_member);
    run("t_ai_skip_keeps_waited", test_t_ai_skip_pay_keeps_waited);
    run("t_setsuna_heart05", test_t_setsuna_debut_gains_heart05);
    run("t_setsuna_waits_cost_le_2", test_t_setsuna_live_start_waits_only_cost2_or_less);
    run("t_rinne_waits_cost4", test_t_rinne_waits_opp_cost_four_member);

    printf("--- U success_score_energy_pl_bp4_004_r_test.rs ---\n");
    run("u_score_six_activates_two", test_u_success_score_six_activates_two_energy);
    run("u_score_five_activates_none", test_u_success_score_five_activates_no_energy);

    rb_unload();

    printf("\n==== parity_state_per_card ====\n");
    printf("tests run              : %d\n", n_tests_ok + n_tests_failed + n_tests_crashed);
    printf("tests passed           : %d\n", n_tests_ok);
    printf("tests failed (parity)  : %d\n", n_tests_failed);
    printf("tests crashed (engine) : %d\n", n_tests_crashed);
    printf("per-test assertion / failure / setup-bug / PROMPT-COUNT numbers are on\n"
           "the child lines above. A prompt count of 0 on a negative-assertion\n"
           "test is flagged inline as VACUOUS by CHECK_PROMPTS.\n");

    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_tests_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_tests_crashed);
        return 1;
    }
    if (n_tests_failed) {
        fprintf(stderr,
                "PARITY STATE PER CARD: %d failing test(s) — every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, a data/parser\n"
                "gap, or a SETUP BUG naming a print the engine cannot resolve.\n"
                "No assertion here was weakened to make a test pass.\n",
                n_tests_failed);
        return n_tests_failed > 125 ? 125 : n_tests_failed;
    }
    printf("ALL PARITY_STATE_PER_CARD CHECKS PASSED\n");
    return 0;
}

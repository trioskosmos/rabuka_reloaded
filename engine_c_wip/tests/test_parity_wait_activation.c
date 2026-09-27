/* test_parity_wait_activation.c — C parity suite for the Rust cluster
 *
 *   engine/tests/test_modules/effects/state/wait_activation/   (21 files)
 *
 * plus (same file, same behaviour family, taken after the primary cluster was
 * finished):
 *
 *   engine/tests/test_modules/characterization/                (see section Z)
 *
 * The cluster is the state-change + activation corner: who gets put to WAIT,
 * who gets ACTIVATED, what the optional activation cost does when it is paid
 * and when it is declined, how activation conditions gate an ability, and how
 * the transitions are recorded. It is deliberately NOT a re-run of
 * tests/test_parity_state_effects2.c, which owns the rb_effect_change_state
 * primitive surface (filters, prompts, per-unit, optional gate, orientation
 * bookkeeping) at the unit level. Everything below drives the same code
 * end-to-end from a real board and a real trigger.
 *
 * ── harness facts peers established (copied verbatim) ────────────────────
 *   `rb_load("src")` fails under an isolated out-of-tree build, so every test
 *   binary needs load_card_db() with the `../cards/build` fallback.
 *
 *   Rust `trigger_auto_ability(id, ..)` + `process_pending_auto_abilities` is
 *   reproduced by `fire_trigger()` below: set activating_card, queue the
 *   trigger, process. The public C header exposes the per-trigger scan
 *   (rb_queue_trigger_abilities) rather than a string-keyed single-ability
 *   entry point, so the queue is built from the trigger token; every fixture
 *   board in this file carries exactly one card printing that token, and
 *   card_prints_trigger() asserts that premise before the trigger is fired.
 *
 *   Rust `select_indices(&[i])` -> rb_resume_with_choice(g, i);
 *   Rust `select_indices(&[])` (decline) -> rb_resume_with_choice(g, -1);
 *   Rust `select_option(1)` (pay)   -> rb_resume_with_choice(g, 1);
 *   Rust `select_option(0)` (skip)  -> rb_resume_with_choice(g, 0).
 *
 * ── the activation_condition caveat ─────────────────────────────────────
 *   `vm.c` does not decode an effect-level `activation_condition`, which makes
 *   the activation-condition branch of rb_can_activate_effect and the
 *   activation-condition term of rb_resolver_needs_gate dead code. Every test
 *   in section C below is a GATED activation case: the ability must NOT run
 *   when the printed condition is unmet, and MUST run when it is met. They are
 *   written so the gated and ungated arms differ in OBSERVABLE STATE (a member
 *   that is waited vs not, an energy count that moved vs not), never in a way
 *   that a dead gate would also satisfy — a negative case that passes only
 *   because nothing happens is worthless, so each negative is paired with a
 *   positive that shares the fixture and differs only in the condition.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

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

/* ── card DB ───────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* ── orientation predicates (mirrors engine/src/ability/util.rs) ─────────── */

/* Rust `is_waited`: mods.get_orientation_modifier(cid) == Some("wait"). */
static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && !strcmp(o, "wait");
}

/* Rust `orientation_matches_state(ori, "active")`: None counts as active. */
static int is_active(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return !o || !strcmp(o, "active");
}

/* Tri-state so a test can pin "no modifier at all" separately from "not wait". */
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

#define CHECK_ORI(actual, expected, message) do { \
    checks++; \
    int a = (actual), e = (expected); \
    if (a != e) { \
        fprintf(stderr, "FAIL: %s (orientation %s, expected %s)\n", message, \
                ori_name(a), ori_name(e)); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── card data helpers ──────────────────────────────────────────────────── */

static int card_cost(int cid)
{
    Card c;
    int cost = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        cost = (int)c.cost;
        rb_free_card(&c);
    }
    return cost;
}

static int card_printed_blade(int cid)
{
    Card c;
    int blade = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        blade = (int)c.blade;
        rb_free_card(&c);
    }
    return blade;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* CRITICAL IDENTITY RULE (AGENTS.md): pin the print before staging it. */
static int check_identity(TestGame *tg, int cid, const char *no, const char *what)
{
    char msg[160];
    snprintf(msg, sizeof msg, "setup: %s resolves to %s", what, no);
    CHECK(rb_card_no_eq(cid, no), msg);
    return rb_card_no_eq(cid, no);
}

/* ── board / deck helpers ───────────────────────────────────────────────── */

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && i < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void fill_deck_p1(TestGame *tg, int filler, int n)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void set_p1_stage3(TestGame *tg, int a, int b, int c)
{
    set_stage(tg, 0, 0, a);
    set_stage(tg, 0, 1, b);
    set_stage(tg, 0, 2, c);
}

static void set_p2_stage3(TestGame *tg, int a, int b, int c)
{
    set_stage(tg, 1, 0, a);
    set_stage(tg, 1, 1, b);
    set_stage(tg, 1, 2, c);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int deck_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].deck, cid); }
static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }

static int p2_stage_has(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[1].stage[i] == cid) return 1;
    return 0;
}

static int under_n(TestGame *tg, int pl, int area)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return tg->state.p[pl].under_cards[area].n;
}

static int under_has(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&tg->state.p[pl].under_cards[area], cid);
}

/* ── choice helpers ────────────────────────────────────────────────────── */

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static const RbChoice *pending_choice(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return NULL;
    return rb_get_pending_choice(&tg->state);
}

/* Rust `while game.has_pending_choice() { game.select_indices(&[0]) }` */
static void drain_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) answer(tg, 0);
}

/* Rust `while game.has_pending_choice() { game.select_indices(&[]) }` —
 * decline/skips everything. */
static void drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) answer(tg, -1);
}

/* bp7_wait_immunity_helpers.rs:46-53 — SelectCard takes index 0, everything
 * else is answered with the "proceed" option. */
static void drain_card_then_option(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) {
        if (!strcmp(test_pending_choice_type(tg), "SelectCard")) answer(tg, 0);
        else answer(tg, 1);
    }
}

/* sp_bp5_choice_energy_test.rs: pay_optional_cost:skip_optional_cost -> pay;
 * any other SelectTarget -> pay too. */
static void drain_pay_then_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) {
        if (!strcmp(test_pending_choice_type(tg), "SelectTarget")) answer(tg, 1);
        else answer(tg, 0);
    }
}

/* ── trigger helpers ───────────────────────────────────────────────────── */

/* Mirrors helpers/mod.rs `trigger_auto(&mut game, cid, trigger, trigger_str)`. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* ── phase helpers (live-start / live-success flows) ───────────────────── */

static void advance_to_live_card_set(TestGame *tg, int passes)
{
    for (int i = 0; i < passes; i++) test_pass(tg);
}

static int advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
    /* live_start_wait_filters... drain_auto: only SelectAutoAbility is
     * answered, everything else stops the loop. */
    int guard = 0;
    while (pending(tg) && guard++ < 30) {
        if (strcmp(pending_kind(tg), "SelectAutoAbility") != 0) break;
        answer(tg, -1);
    }
    return guard;
}

/* ══════════════════════════════════════════════════════════════════════
 * A. self_wait_and_nijigasaki_activation_test.rs
 *    PL!N-bp3-006-R 彼方 登場 self-wait; ワンダーメイツ 虹ヶ咲 activation
 * ══════════════════════════════════════════════════════════════════════ */

static void test_kanata_n_bp3_debut_active_self_becomes_waited(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanata = test_id(&tg, "PL!N-bp3-006-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (!check_identity(&tg, kanata, "PL!N-bp3-006-R", "the 彼方 debut print")) return;
    fill_decks(&tg, filler, 20);

    set_p1_stage3(&tg, RB_EMPTY_SLOT, kanata, RB_EMPTY_SLOT);
    CHECK_ORI(orientation_of(&tg, kanata), ORI_NONE,
              "setup: a freshly staged member starts active (no modifier)");

    CHECK(fire_trigger(&tg, kanata, RB_TSTR_DEBUT),
          "PL!N-bp3-006-R really prints a 登場 ability and the trigger is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, kanata), ORI_WAIT,
              "『このメンバーをウェイトにする』 — the debut resolves with a self-wait");
}

static void test_wondermates_live_start_activates_waited_nijigasaki(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanata   = test_id(&tg, "PL!N-bp1-006-R＋");
    int wonder   = test_id(&tg, "PL!N-sd2-025-P");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler   = test_id(&tg, "PL!-sd1-010-SD");
    if (kanata < 0 || wonder < 0 || outsider < 0 || filler < 0) {
        CHECK(0, "wondermates fixtures resolve");
        return;
    }
    check_identity(&tg, kanata, "PL!N-bp1-006-R＋", "the 彼方 起動 print (NOT pb1-006-R)");
    check_identity(&tg, wonder, "PL!N-sd2-025-P", "ワンダーメイツ");

    fill_decks(&tg, filler, 30);
    /* Fund her 起動 so the "still usable" assertion below means something. */
    test_give_energy(&tg, 4);
    test_add_to_hand(&tg, filler);

    set_p1_stage3(&tg, kanata, wonder, outsider);
    rb_mods_set_orientation(&tg.state.mods, kanata, "wait");
    rb_mods_set_orientation(&tg.state.mods, outsider, "wait");

    CHECK(fire_trigger(&tg, wonder, RB_TSTR_LIVE_START),
          "ワンダーメイツ prints a ライブ開始時 and the trigger is queued");
    drain_first(&tg, 12);

    CHECK(is_active(&tg, kanata),
          "『虹ヶ咲のメンバー1人をアクティブにする』 — the waited 虹ヶ咲 member ends active");
    CHECK(is_waited(&tg, outsider),
          "the μ's member is not touched by a 虹ヶ咲-group filter");

    /* The 起動 offer must survive: activating her costs 2E and draws 2, so a
     * successful activation is the observable proof she is still usable. */
    int deck_before = tg.state.p[0].deck.n;
    int rc = test_activate_ability(&tg, kanata);
    drain_first(&tg, 8);
    CHECK(rc != 0, "the re-activated 彼方 can still activate her 起動 this turn");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 2,
             "her 起動 (2E draw-2) actually resolved, so the offer was live");
}

/* wondermates_group_filter_negatives_test.rs — the three group-filter negatives
 * the sibling positive test cannot reach. */

static void test_wondermates_activates_a_nijigasaki_never_the_outsider(void)
{
    TestGame tg;
    test_game_new(&tg);
    int wonder   = test_id(&tg, "PL!N-sd2-025-P");
    int first    = test_id(&tg, "PL!N-bp1-006-R＋");
    int second   = test_new_id(&tg, "PL!N-bp1-013-PR"); /* 虹ヶ咲, distinct from 近江彼方 */
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler   = test_id(&tg, "PL!-sd1-010-SD");
    if (wonder < 0 || first < 0 || second < 0 || outsider < 0) {
        CHECK(0, "wondermates group-filter fixtures resolve");
        return;
    }
    CHECK(first != second,
          "setup: two DISTINCT 虹ヶ咲 members, or there is no choice to get wrong");
    fill_decks(&tg, filler, 30);

    set_p1_stage3(&tg, first, wonder, outsider);
    rb_mods_set_orientation(&tg.state.mods, first, "wait");
    rb_mods_set_orientation(&tg.state.mods, second, "wait");
    rb_mods_set_orientation(&tg.state.mods, outsider, "wait");

    CHECK(fire_trigger(&tg, wonder, RB_TSTR_LIVE_START),
          "the ワンダーメイツ live-start is queued");
    drain_first(&tg, 12);

    int a = is_active(&tg, first);
    int b = is_active(&tg, second);
    CHECK(a || b, "『『虹ヶ咲』のメンバー1人をアクティブにする』 — a 虹ヶ咲 member ends active");
    CHECK(!is_active(&tg, outsider),
          "the μ's member is NEVER activated, whatever the prompt offered");
    CHECK_EQ(a + b, 1, "「1人を」 — exactly ONE member is activated, not the whole group");
}

static void test_wondermates_activates_nothing_without_a_nijigasaki_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int wonder = test_id(&tg, "PL!N-sd2-025-P");
    int out_a  = test_id(&tg, "PL!-sd1-010-SD");
    int out_b  = test_new_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (wonder < 0 || out_a < 0 || out_b < 0) {
        CHECK(0, "wondermates no-candidate fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 30);
    set_p1_stage3(&tg, out_a, wonder, out_b);
    rb_mods_set_orientation(&tg.state.mods, out_a, "wait");
    rb_mods_set_orientation(&tg.state.mods, out_b, "wait");

    CHECK(fire_trigger(&tg, wonder, RB_TSTR_LIVE_START),
          "the ワンダーメイツ live-start is queued with no 虹ヶ咲 member staged");
    drain_first(&tg, 12);

    CHECK(!is_active(&tg, out_a) && !is_active(&tg, out_b),
          "with no 『虹ヶ咲』 member staged, nothing may be activated — a group-blind "
          "reading would activate one of these");
}

static void test_wondermates_leaves_an_already_active_nijigasaki_alone(void)
{
    TestGame tg;
    test_game_new(&tg);
    int wonder   = test_id(&tg, "PL!N-sd2-025-P");
    int niji     = test_id(&tg, "PL!N-bp1-006-R＋");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler   = test_id(&tg, "PL!-sd1-010-SD");
    if (wonder < 0 || niji < 0 || outsider < 0) {
        CHECK(0, "wondermates already-active fixtures resolve");
        return;
    }
    fill_decks(&tg, filler, 30);
    set_p1_stage3(&tg, niji, wonder, outsider);
    rb_mods_set_orientation(&tg.state.mods, niji, "active");
    rb_mods_set_orientation(&tg.state.mods, outsider, "wait");

    CHECK(fire_trigger(&tg, wonder, RB_TSTR_LIVE_START),
          "the ワンダーメイツ live-start is queued against an already-active member");
    drain_first(&tg, 12);

    CHECK(!is_active(&tg, outsider),
          "the μ's member is not touched even when the only 『虹ヶ咲』 member is active");
    CHECK(!pending(&tg),
          "with nothing left to resolve the prompt queue is empty, so the ability is "
          "not stuck on a choice it could not offer");
    CHECK_EQ(tg.state.activating_card, -1,
             "the ability runs to completion: activating_card is cleared on finish");
}

/* ══════════════════════════════════════════════════════════════════════
 * B. self_wait_printemps_energy_pl_pb1_003_r_test.rs
 *    PL!-pb1-003-R ことり 登場 — optional self-wait cost
 * ══════════════════════════════════════════════════════════════════════ */

static void pb1_003_board(TestGame *tg, int *kotori, int *p1m, int *p2m)
{
    int k = test_id(tg, "PL!-pb1-003-R");
    int a = test_id(tg, "PL!-sd1-010-SD");
    int b = test_id(tg, "PL!-pb1-021-PR");
    *kotori = k; *p1m = a; *p2m = b;
    if (k < 0 || a < 0 || b < 0) return;
    set_p1_stage3(tg, a, k, b);
    test_give_energy(tg, 5);
    test_set_energy_active(tg, 0, 2);
}

static void test_pb1_003_declined_self_wait_preserves_active_energy(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kotori, p1m, p2m;
    pb1_003_board(&tg, &kotori, &p1m, &p2m);
    if (kotori < 0) { CHECK(0, "PL!-pb1-003-R fixtures resolve"); return; }
    check_identity(&tg, kotori, "PL!-pb1-003-R", "the 南ことり debut print");

    CHECK(fire_trigger(&tg, kotori, RB_TSTR_DEBUT), "the ことり 登場 is queued");
    CHECK(pending(&tg), "optional self-wait must prompt");
    answer(&tg, 0); /* Rust select_option(0) = decline */

    CHECK_EQ(tg.state.p[0].energy_active, 2,
             "declined cost -> no energy activated");
    CHECK_ORI(orientation_of(&tg, kotori), ORI_NONE,
              "declining the optional cost leaves ことり active, not waited");
    CHECK(!pending(&tg), "nothing further is asked after declining");
}

static void test_pb1_003_paid_self_wait_activates_energy_per_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kotori, p1m, p2m;
    pb1_003_board(&tg, &kotori, &p1m, &p2m);
    if (kotori < 0) { CHECK(0, "PL!-pb1-003-R fixtures resolve"); return; }

    CHECK(fire_trigger(&tg, kotori, RB_TSTR_DEBUT), "the ことり 登場 is queued");
    answer(&tg, 1); /* Rust select_option(1) = pay */

    CHECK_ORI(orientation_of(&tg, kotori), ORI_WAIT,
              "accepted cost waits ことり");
    CHECK_EQ(tg.state.p[0].energy_active, 5,
             "per-Printemps count includes every stage member (3) -> 2+3=5 active");
}

static void test_pb1_003_counts_two_physical_instances_separately(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kotori  = test_id(&tg, "PL!-pb1-003-R");
    int kotori2 = test_new_id(&tg, "PL!-pb1-003-R");
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (kotori < 0 || kotori2 < 0) { CHECK(0, "PL!-pb1-003-R copies resolve"); return; }
    CHECK(kotori != kotori2, "two separate physical instances of the same print");
    CHECK(rb_card_no_eq(kotori, "PL!-pb1-003-R") && rb_card_no_eq(kotori2, "PL!-pb1-003-R"),
          "both instances share one card_no");

    set_p1_stage3(&tg, kotori, kotori2, filler);
    test_give_energy(&tg, 5);
    test_set_energy_active(&tg, 0, 2);

    CHECK(fire_trigger(&tg, kotori, RB_TSTR_DEBUT), "the ことり 登場 is queued");
    answer(&tg, 1);

    CHECK_ORI(orientation_of(&tg, kotori), ORI_WAIT, "accepted cost waits the resolving ことり");
    CHECK_EQ(tg.state.p[0].energy_active, 4,
             "only the two physical Printemps instances count -> 2+2=4 active");
}

/* ══════════════════════════════════════════════════════════════════════
 * C. cost_and_blade_filtered_opponent_wait_test.rs
 *    PL!HS-pb1-010-R さやか 登場 (cost>=10 gate, cost<=4 opponent wait)
 *    PL!SP-bp7-009-R 夏美 ライブ開始時 (center gate, original blade <= 2)
 *
 * Every case here is an ACTIVATION-CONDITION gate. The positive and the
 * negative differ ONLY in the fixture that the printed condition reads, and
 * the assertion is an exact orientation, so a dead gate cannot pass both.
 * ══════════════════════════════════════════════════════════════════════ */

#define SAYAKA   "PL!HS-pb1-010-R"
#define RUBY     "PL!S-bp5-009-R"   /* cost 15, blade 5 */
#define FILLER4  "PL!-sd1-010-SD"   /* cost 4,  blade 1 */
#define KOTORI5  "PL!-pb1-021-PR"   /* cost 5,  blade 1 */

static void test_sayaka_debut_with_cost10_member_waits_cost4_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int sayaka = test_id(&tg, SAYAKA);
    int big    = test_id(&tg, RUBY);
    int cheap  = test_id(&tg, FILLER4);
    if (sayaka < 0 || big < 0 || cheap < 0) { CHECK(0, "sayaka fixtures resolve"); return; }
    check_identity(&tg, sayaka, SAYAKA, "the 村野さやか debut print");
    CHECK_EQ(card_cost(big), 15, "setup: my cost-15 member really costs >= 10");
    CHECK_EQ(card_cost(cheap), 4, "setup: the opponent's member really costs <= 4");
    CHECK_EQ(card_cost(sayaka), 2, "setup: さやか herself costs 2, so she is not the >= 10 member");

    set_p1_stage3(&tg, big, sayaka, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, cheap, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, sayaka, RB_TSTR_DEBUT), "the さやか 登場 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, cheap), ORI_WAIT,
              "my stage has a cost-15 member -> the opponent's cost-4 member is waited");
}

static void test_sayaka_debut_without_cost10_member_leaves_opponent_active(void)
{
    TestGame tg;
    test_game_new(&tg);
    int sayaka = test_id(&tg, SAYAKA);
    int cheap  = test_id(&tg, FILLER4);
    if (sayaka < 0 || cheap < 0) { CHECK(0, "sayaka negative fixtures resolve"); return; }
    CHECK_EQ(card_cost(sayaka), 2, "setup: the only member on my stage costs 2");

    set_p1_stage3(&tg, RB_EMPTY_SLOT, sayaka, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, cheap, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, sayaka, RB_TSTR_DEBUT), "the さやか 登場 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, cheap), ORI_NONE,
              "no cost>=10 member on MY stage -> the activation condition is unmet and "
              "the opponent's member stays pristine (this arm distinguishes a LIVE gate "
              "from a dead one: the positive above waits the same fixture)");
}

static void test_sayaka_debut_excludes_cost5_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int sayaka = test_id(&tg, SAYAKA);
    int big    = test_id(&tg, RUBY);
    int pricey  = test_id(&tg, KOTORI5);
    if (sayaka < 0 || big < 0 || pricey < 0) { CHECK(0, "sayaka cost-5 fixtures resolve"); return; }
    CHECK_EQ(card_cost(pricey), 5, "setup: the cost-5 member is over the コスト4以下 filter");

    set_p1_stage3(&tg, big, sayaka, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, pricey, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, sayaka, RB_TSTR_DEBUT), "the さやか 登場 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, pricey), ORI_NONE,
              "the activation condition is met but the opponent's cheapest member costs "
              "5 > 4 -> nobody is waited");
}

static void test_natsumi_live_start_waits_only_low_original_blade_in_center(void)
{
    TestGame tg;
    test_game_new(&tg);
    int natsumi = test_id(&tg, "PL!SP-bp7-009-R");
    int low     = test_id(&tg, FILLER4);
    int high    = test_id(&tg, RUBY);
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (natsumi < 0 || low < 0 || high < 0) { CHECK(0, "natsumi fixtures resolve"); return; }
    check_identity(&tg, natsumi, "PL!SP-bp7-009-R", "the 鬼塚夏美 live-start print");
    CHECK_EQ(card_printed_blade(low), 1, "setup: the low member's ORIGINAL blade is 1");
    CHECK_EQ(card_printed_blade(high), 5, "setup: the high member's ORIGINAL blade is 5");
    fill_decks(&tg, filler, 20);

    set_p1_stage3(&tg, RB_EMPTY_SLOT, natsumi, RB_EMPTY_SLOT); /* CENTER required */
    set_p2_stage3(&tg, low, high, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, natsumi, RB_TSTR_LIVE_START), "the 夏美 ライブ開始時 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, low), ORI_WAIT,
              "opponent member with 元々ブレード1 <= 2 is waited");
    CHECK_ORI(orientation_of(&tg, high), ORI_NONE,
              "original blade 5 exceeds the limit -> untouched");
}

static void test_natsumi_from_a_side_does_nothing(void)
{
    TestGame tg;
    test_game_new(&tg);
    int natsumi = test_id(&tg, "PL!SP-bp7-009-R");
    int low     = test_id(&tg, FILLER4);
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (natsumi < 0 || low < 0) { CHECK(0, "natsumi side-gate fixtures resolve"); return; }
    fill_decks(&tg, filler, 20);

    /* same eligible target, but the source is on a SIDE this time */
    set_p1_stage3(&tg, natsumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, low, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, natsumi, RB_TSTR_LIVE_START), "the 夏美 ライブ開始時 is queued");
    drain_skip(&tg, 12);

    CHECK_ORI(orientation_of(&tg, low), ORI_NONE,
              "（センター限定）: not in center -> no effect even with eligible targets "
              "(the centre arm above waits this same fixture)");
}

/* ══════════════════════════════════════════════════════════════════════
 * D. pl_s_bp6_015_n_debut_opponent_cost2_wait_test.rs
 *    PL!S-bp6-015-N 善子 登場 — コスト2以下 opponent wait
 * ══════════════════════════════════════════════════════════════════════ */

#define YOSHIKO  "PL!S-bp6-015-N"
#define OPP_C2   "PL!N-bp7-017-N"   /* cost 2  */
#define OPP_C13  "PL!-sd1-003-SD"   /* cost 13 */

static void yoshiko_setup(TestGame *tg, const char *opp_no, int *yoshiko, int *opp)
{
    int y = test_id(tg, YOSHIKO);
    int o = test_id(tg, opp_no);
    *yoshiko = y; *opp = o;
    if (y < 0 || o < 0) return;
    int filler = test_id(tg, "PL!-sd1-010-SD");
    set_p2_stage3(tg, o, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(tg, y);
    test_give_energy(tg, 15);
    fill_deck_p1(tg, filler, 5);
}

static void test_yoshiko_waits_opponent_cost2(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko, opp;
    yoshiko_setup(&tg, OPP_C2, &yoshiko, &opp);
    if (yoshiko < 0 || opp < 0) { CHECK(0, "PL!S-bp6-015-N fixtures resolve"); return; }
    check_identity(&tg, yoshiko, YOSHIKO, "the 津島善子 debut print");
    CHECK_EQ(card_cost(opp), 2, "setup: the opponent's member really costs 2");

    CHECK(test_play_to_stage(&tg, yoshiko, 1), "善子 is played to centre");
    drain_skip(&tg, 12);

    CHECK_ORI(orientation_of(&tg, opp), ORI_WAIT,
              "コスト2以下のメンバーの1人をウェイト状態にする — the cost-2 opponent qualifies");
}

static void test_yoshiko_leaves_cost13_opponent_pristine(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko, opp;
    yoshiko_setup(&tg, OPP_C13, &yoshiko, &opp);
    if (yoshiko < 0 || opp < 0) { CHECK(0, "PL!S-bp6-015-N negative fixtures resolve"); return; }
    CHECK_EQ(card_cost(opp), 13, "setup: the opponent's member really costs 13");

    CHECK(test_play_to_stage(&tg, yoshiko, 1), "善子 is played to centre");
    drain_skip(&tg, 12);

    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "コスト2以下 only — a cost-13 opponent member gets no orientation modifier "
              "at all (the cost-2 arm above waits the same fixture)");
    CHECK(p2_stage_has(&tg, opp), "the opponent's member must still be on their stage");
}

static void test_yoshiko_empty_opponent_stage_offers_no_choice(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, YOSHIKO);
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (yoshiko < 0) { CHECK(0, "PL!S-bp6-015-N empty-opponent fixtures resolve"); return; }
    set_p2_stage3(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, yoshiko);
    test_give_energy(&tg, 15);
    fill_deck_p1(&tg, filler, 5);

    CHECK(test_play_to_stage(&tg, yoshiko, 1), "善子 is played to centre");
    drain_skip(&tg, 12);
    CHECK(!pending(&tg),
          "an empty opponent stage has no candidate, so no choice may be offered");
}

static void test_yoshiko_two_cost2_choose_exactly_one(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko = test_id(&tg, YOSHIKO);
    int opp1    = test_id(&tg, OPP_C2);
    int opp2    = test_new_id(&tg, OPP_C2);
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    if (yoshiko < 0 || opp1 < 0 || opp2 < 0) { CHECK(0, "PL!S-bp6-015-N two-copies fixtures resolve"); return; }
    CHECK(opp1 != opp2, "two separate card instances");
    CHECK(rb_card_no_eq(opp1, OPP_C2) && rb_card_no_eq(opp2, OPP_C2),
          "both instances are the same cost-2 print");
    CHECK_EQ(card_cost(opp1), 2, "setup: the shared print really costs 2");

    set_p2_stage3(&tg, opp1, opp2, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, yoshiko);
    test_give_energy(&tg, 15);
    fill_deck_p1(&tg, filler, 5);

    CHECK(test_play_to_stage(&tg, yoshiko, 1), "善子 is played to centre");
    CHECK(pending(&tg), "expected a SelectCard for multiple cost-2 targets");
    CHECK_EQ(strcmp(pending_kind(&tg), "SelectCard"), 0,
             "with two qualifying members the 1人 effect offers a card selection");
    answer(&tg, 0);
    drain_skip(&tg, 12);

    CHECK_EQ(orientation_of(&tg, opp1), ORI_WAIT,
             "1人 — exactly the chosen member (index 0) is waited");
    CHECK_ORI(orientation_of(&tg, opp2), ORI_NONE,
              "1人 — the unchosen member is not waited");
}

/* ══════════════════════════════════════════════════════════════════════
 * E. debut_low_cost_opponent_wait_test.rs
 *    PL!SP-pb2-029-N メイ 登場/ライブ開始時 — cost<=2 opponent wait
 * ══════════════════════════════════════════════════════════════════════ */

static void test_mai_debut_waits_opponent_cost2_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int mai   = test_id(&tg, "PL!SP-pb2-029-N");
    int cheap = test_id(&tg, "PL!SP-PR-007-PR");
    if (mai < 0 || cheap < 0) { CHECK(0, "PL!SP-pb2-029-N fixtures resolve"); return; }
    check_identity(&tg, mai, "PL!SP-pb2-029-N", "the 米女メイ debut print");
    check_identity(&tg, cheap, "PL!SP-PR-007-PR", "the KALEIDOSCORE cost-2 print");
    CHECK_EQ(card_cost(cheap), 2, "setup: the opponent's member really costs 2");

    set_p1_stage3(&tg, mai, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, cheap, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(card_prints_trigger(mai, RB_TSTR_DEBUT),
          "PL!SP-pb2-029-N really prints a 登場-triggered ability");
    CHECK(fire_trigger(&tg, mai, RB_TSTR_DEBUT), "the メイ 登場 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, cheap), ORI_WAIT,
              "the opponent's cost-2 member should be rested");
}

static void test_mai_debut_ignores_opponent_over_cost2(void)
{
    TestGame tg;
    test_game_new(&tg);
    int mai    = test_id(&tg, "PL!SP-pb2-029-N");
    int pricey = test_id(&tg, OPP_C13);
    if (mai < 0 || pricey < 0) { CHECK(0, "PL!SP-pb2-029-N negative fixtures resolve"); return; }
    CHECK_EQ(card_cost(pricey), 13, "setup: the opponent's member really costs 13");

    set_p1_stage3(&tg, mai, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, pricey, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, mai, RB_TSTR_DEBUT), "the メイ 登場 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, pricey), ORI_NONE,
              "コスト2以下 only — a cost-13 opponent member is not a candidate (the "
              "cost-2 arm above waits the same fixture)");
}

/* ══════════════════════════════════════════════════════════════════════
 * F. low_blade_opponent_wait_pl_hs_bp6_013_r_test.rs
 *    PL!HS-bp6-013-R こ鈴 ライブ開始時 — low original blade opponent wait
 * ══════════════════════════════════════════════════════════════════════ */

static void test_kozuru_live_start_waits_low_blade_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int me     = test_id(&tg, "PL!HS-bp6-013-R");
    int victim = test_id(&tg, "PL!N-sd1-010-SD");
    int filler = test_id(&tg, "PL!N-sd1-010-SD");
    if (me < 0 || victim < 0) { CHECK(0, "PL!HS-bp6-013-R fixtures resolve"); return; }
    check_identity(&tg, me, "PL!HS-bp6-013-R", "the 徒町小鈴 live-start print");
    CHECK(card_printed_blade(victim) >= 0, "setup: the victim prints a blade count");
    fill_decks(&tg, filler, 20);

    set_p1_stage3(&tg, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, victim, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, me, RB_TSTR_LIVE_START), "the 小鈴 ライブ開始時 is queued");
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, victim), ORI_WAIT, "opponent low-blade member waited");
}

static void test_kozuru_empty_opponent_stage_is_a_noop(void)
{
    TestGame tg;
    test_game_new(&tg);
    int me     = test_id(&tg, "PL!HS-bp6-013-R");
    int filler = test_id(&tg, "PL!N-sd1-010-SD");
    if (me < 0) { CHECK(0, "PL!HS-bp6-013-R noop fixtures resolve"); return; }
    fill_decks(&tg, filler, 20);
    set_p1_stage3(&tg, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_trigger(&tg, me, RB_TSTR_LIVE_START), "the 小鈴 ライブ開始時 is queued");
    drain_skip(&tg, 12);

    CHECK(!pending(&tg), "an empty opponent stage offers nothing to choose");
    CHECK_EQ(tg.state.p[1].stage[0], RB_EMPTY_SLOT, "p2 LeftSide stays empty");
    CHECK_EQ(tg.state.p[1].stage[1], RB_EMPTY_SLOT, "p2 Center stays empty");
    CHECK_EQ(tg.state.p[1].stage[2], RB_EMPTY_SLOT, "p2 RightSide stays empty");
}

/* ══════════════════════════════════════════════════════════════════════
 * G. hanayo_self_wait_cost_activates_other_member_test.rs
 *    PL!-bp6-008-R 花陽 起動 — self_cost wait, effect activates another member
 *    Q248: the ability is usable even with no waited members on stage.
 * ══════════════════════════════════════════════════════════════════════ */

#define HANAYO "PL!-bp6-008-R"

static void hanayo_deploy(TestGame *tg, int *hanayo, int *friend)
{
    int h = test_id(tg, HANAYO);
    int f = test_id(tg, "PL!-sd1-010-SD");
    *hanayo = h; *friend = f;
    if (h < 0 || f < 0) return;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, f);
    test_add_to_hand(tg, h);
    test_give_energy(tg, 8);
}

static void test_hanayo_activate_with_no_other_members(void)
{
    TestGame tg;
    test_game_new(&tg);
    int hanayo, friend;
    hanayo_deploy(&tg, &hanayo, &friend);
    if (hanayo < 0) { CHECK(0, "PL!-bp6-008-R fixtures resolve"); return; }
    check_identity(&tg, hanayo, HANAYO, "the 小泉花陽 起動 print");
    CHECK_EQ(card_cost(hanayo), 7, "setup: 花陽 costs 7, so 8 energy covers cost+1");

    CHECK(test_play_to_stage(&tg, hanayo, 1), "花陽 is played to centre");
    drain_skip(&tg, 8);
    CHECK(!pending(&tg), "花陽 has no debut ability");

    CHECK(test_activate_ability(&tg, hanayo) != 0,
          "Q248: the 起動 succeeds even with no other members on stage");
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, hanayo), ORI_WAIT,
              "the activation cost is paid: 花陽 is now wait");
    CHECK(!pending(&tg), "no choice needed — no other members to activate");
}

static void test_hanayo_activate_others_all_active_is_a_noop(void)
{
    TestGame tg;
    test_game_new(&tg);
    int hanayo, friend;
    hanayo_deploy(&tg, &hanayo, &friend);
    if (hanayo < 0) { CHECK(0, "PL!-bp6-008-R all-active fixtures resolve"); return; }

    set_p1_stage3(&tg, friend, hanayo, RB_EMPTY_SLOT);
    CHECK(test_activate_ability(&tg, hanayo) != 0, "the 起動 succeeds with an active-only board");
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, hanayo), ORI_WAIT, "the activation cost is paid");
    CHECK_ORI(orientation_of(&tg, friend), ORI_NONE,
              "the friend stays active — the effect only targets waited members");
    CHECK(!pending(&tg), "no choice — no wait members to activate");
}

static void test_hanayo_activate_with_wait_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int hanayo, friend;
    hanayo_deploy(&tg, &hanayo, &friend);
    if (hanayo < 0) { CHECK(0, "PL!-bp6-008-R wait-member fixtures resolve"); return; }

    set_p1_stage3(&tg, friend, hanayo, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, friend, "wait");

    CHECK(test_activate_ability(&tg, hanayo) != 0, "the 起動 finds the waited friend");
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, hanayo), ORI_WAIT, "the activation cost is paid");
    CHECK_ORI(orientation_of(&tg, friend), ORI_ACTIVE,
              "『自分のステージにいるほかのメンバー1人をアクティブにする』 — the waited "
              "friend is activated (this arm distinguishes the effect from a dead one: the "
              "all-active arm above leaves the same friend pristine)");
    CHECK(!pending(&tg), "no remaining choices");
}

static void test_hanayo_use_limit_blocks_second_activation(void)
{
    TestGame tg;
    test_game_new(&tg);
    int hanayo, friend;
    hanayo_deploy(&tg, &hanayo, &friend);
    if (hanayo < 0) { CHECK(0, "PL!-bp6-008-R use-limit fixtures resolve"); return; }

    CHECK(test_play_to_stage(&tg, hanayo, 1), "花陽 is played to centre");
    drain_skip(&tg, 8);
    CHECK(test_activate_ability(&tg, hanayo) != 0, "the first activation succeeds");
    drain_skip(&tg, 8);

    int rc = test_activate_ability(&tg, hanayo);
    CHECK(rc == 0, "ターン1回 — the second activation in the same turn is refused");
    /* The refusal must be the use limit, not a silent success. */
    CHECK_ORI(orientation_of(&tg, hanayo), ORI_WAIT,
              "the refused second activation does not change the board");
}

/* ══════════════════════════════════════════════════════════════════════
 * H. kanata_active_phase_restriction_and_live_success_self_wait_test.rs
 *    PL!N-bp5-006-R 彼方 — 常時 active-phase restriction + ライブ成功時 self-wait
 * ══════════════════════════════════════════════════════════════════════ */

#define KANATA_R "PL!N-bp5-006-R"

static void test_kanata_live_success_with_others_waits_self(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanata = test_id(&tg, KANATA_R);
    int other  = test_id(&tg, "PL!-sd1-001-SD");
    int live   = test_id(&tg, "PL!-sd1-019-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (kanata < 0 || other < 0 || live < 0) { CHECK(0, "PL!N-bp5-006-R fixtures resolve"); return; }
    check_identity(&tg, kanata, KANATA_R, "the 近江彼方 live-success print");
    CHECK(card_prints_trigger(kanata, RB_TSTR_LIVE_SUCCESS),
          "PL!N-bp5-006-R really prints a ライブ成功時 self-wait");

    fill_deck_p1(&tg, filler, 40);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 1, filler);
    set_p1_stage3(&tg, kanata, other, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, live);

    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live);
    test_pass(&tg);
    test_pass(&tg);
    drain_skip(&tg, 12);
    for (int i = 0; i < 3; i++) test_pass(&tg);
    drain_skip(&tg, 12);
    test_pass(&tg);

    CHECK_ORI(orientation_of(&tg, kanata), ORI_WAIT,
              "『自分のステージにこのメンバー以外のメンバーがいる場合、このメンバーを"
              "ウェイトにする』 — 彼方 is waited by her own LiveSuccess ability");
}

static void test_kanata_live_success_no_others_does_not_wait(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanata = test_id(&tg, KANATA_R);
    int live   = test_id(&tg, "PL!-sd1-019-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (kanata < 0 || live < 0) { CHECK(0, "PL!N-bp5-006-R solo fixtures resolve"); return; }

    fill_deck_p1(&tg, filler, 40);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 1, filler);
    set_p1_stage3(&tg, kanata, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, live);

    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, live);
    test_pass(&tg);
    test_pass(&tg);
    drain_skip(&tg, 12);
    for (int i = 0; i < 3; i++) test_pass(&tg);
    drain_skip(&tg, 12);
    test_pass(&tg);

    CHECK(!is_waited(&tg, kanata),
          "no other members on stage -> 彼方 must NOT be waited (the paired positive "
          "above waits the same card with a second member staged)");
}

static void test_kanata_not_auto_activated_in_active_phase_but_others_are(void)
{
    TestGame tg;
    test_game_new(&tg);
    int kanata = test_id(&tg, KANATA_R);
    int other  = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-013-SD");
    if (kanata < 0 || other < 0) { CHECK(0, "PL!N-bp5-006-R active-phase fixtures resolve"); return; }
    fill_decks(&tg, filler, 30);
    set_p1_stage3(&tg, kanata, other, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, kanata, "wait");
    rb_mods_set_orientation(&tg.state.mods, other, "wait");

    test_recalc(&tg);
    /* Mirror the Rust constant_cannot_activate_members check through the C
     * surface that owns the same rule. */
    CHECK(card_prints_trigger(kanata, RB_TSTR_CONSTANT),
          "setup: 彼方 prints the 常時 「アクティブにしない」 restriction");
    CHECK(!card_prints_trigger(other, RB_TSTR_CONSTANT),
          "setup: the partner member prints no such restriction");

    tg.state.phase = RB_PHASE_ACTIVE;
    test_pass(&tg);

    CHECK_ORI(orientation_of(&tg, kanata), ORI_WAIT,
              "『自分のアクティブフェイズにアクティブにしない』 — 彼方 is not auto-activated");
    CHECK(!is_waited(&tg, other),
          "the partner member with no restriction IS auto-activated in the active phase "
          "(this arm distinguishes the restriction from a dead gate)");
}

/* ══════════════════════════════════════════════════════════════════════
 * I. bp7_karin_wait_blade_limit_test.rs
 *    PL!N-bp7-004-R 果林 起動 — DYNAMIC blade limit from the energy under her
 * ══════════════════════════════════════════════════════════════════════ */

#define KARIN     "PL!N-bp7-004-R"
#define ENERGY    "LL-E-001-SD"
#define ENEMY_B1  "PL!-sd1-010-SD"   /* original blade 1 */
#define ENEMY_B4  "PL!N-bp1-001-R"   /* original blade 4 */

/* Put 果林 at centre with `pre_under` energy already under her, plus one energy
 * card in the zone so the activation cost is payable. */
static int karin_setup(TestGame *tg, int pre_under)
{
    int karin = test_id(tg, KARIN);
    int e     = test_id(tg, ENERGY);
    if (karin < 0 || e < 0) return -1;
    set_stage(tg, 0, 1, karin);
    for (int i = 0; i < pre_under; i++) test_place_under(tg, 0, 1, e);
    RbPlayer *P = &tg->state.p[0];
    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = e;
    return karin;
}

static void test_karin_cost_places_energy_and_waits_below_limit(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 0);
    int enemy = test_id(&tg, ENEMY_B1);
    if (karin < 0 || enemy < 0) { CHECK(0, "PL!N-bp7-004-R fixtures resolve"); return; }
    check_identity(&tg, karin, KARIN, "the 朝香果林 起動 print");
    CHECK_EQ(card_printed_blade(enemy), 1, "setup: the enemy member has original blade 1");
    set_p2_stage3(&tg, enemy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");
    drain_card_then_option(&tg, 20);

    CHECK_EQ(under_n(&tg, 0, 1), 1, "activation cost must place exactly 1 energy under 果林");
    CHECK_ORI(orientation_of(&tg, enemy), ORI_WAIT,
              "blade-1 opponent (<= limit 2) should be waited");
}

static void test_karin_does_not_wait_above_limit(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 0);
    int enemy = test_id(&tg, ENEMY_B4);
    if (karin < 0 || enemy < 0) { CHECK(0, "PL!N-bp7-004-R above-limit fixtures resolve"); return; }
    CHECK_EQ(card_printed_blade(enemy), 4, "setup: the enemy member has original blade 4");
    set_p2_stage3(&tg, enemy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");
    drain_card_then_option(&tg, 20);

    CHECK_EQ(under_n(&tg, 0, 1), 1, "1 energy under 果林 -> limit 2");
    CHECK_ORI(orientation_of(&tg, enemy), ORI_NONE,
              "blade-4 opponent (> limit 2) must NOT be waited");
}

static void test_karin_waits_only_in_limit_when_both_present(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 0);
    int low   = test_id(&tg, ENEMY_B1);
    int high  = test_id(&tg, ENEMY_B4);
    if (karin < 0 || low < 0 || high < 0) { CHECK(0, "PL!N-bp7-004-R both fixtures resolve"); return; }
    set_p2_stage3(&tg, low, high, RB_EMPTY_SLOT);

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");
    drain_card_then_option(&tg, 20);

    CHECK_ORI(orientation_of(&tg, low), ORI_WAIT, "blade-1 member should be waited");
    CHECK_ORI(orientation_of(&tg, high), ORI_NONE, "blade-4 member must not be waited");
}

static void test_karin_dynamic_limit_scales_with_energy_under(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 3);
    int enemy = test_id(&tg, ENEMY_B4);
    if (karin < 0 || enemy < 0) { CHECK(0, "PL!N-bp7-004-R dynamic fixtures resolve"); return; }
    CHECK_EQ(under_n(&tg, 0, 1), 3, "setup: 3 energy pre-seeded under 果林");
    set_p2_stage3(&tg, enemy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");
    drain_card_then_option(&tg, 20);

    CHECK_EQ(under_n(&tg, 0, 1), 4, "3 pre-seeded + 1 cost = 4 energy under -> limit 5");
    CHECK_ORI(orientation_of(&tg, enemy), ORI_WAIT,
              "DYNAMIC limit: the SAME blade-4 member that failed at limit 2 is now "
              "waited when energy_under = 4");
}

static void test_karin_two_eligible_members_offers_selection(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 0);
    int low_a = test_id(&tg, ENEMY_B1);
    int low_b = test_new_id(&tg, ENEMY_B1);
    if (karin < 0 || low_a < 0 || low_b < 0) { CHECK(0, "PL!N-bp7-004-R two-target fixtures resolve"); return; }
    set_p2_stage3(&tg, low_a, low_b, RB_EMPTY_SLOT);

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");

    int stage_choice_filtered = -1;
    int guard = 0;
    while (pending(&tg) && guard++ < 20) {
        const RbChoice *c = pending_choice(&tg);
        if (c && c->kind == RB_CHOICE_SELECT_CARD &&
            (!strcmp(c->zone, "energy_zone") || !strcmp(c->zone, "energy"))) {
            answer(&tg, 0); /* pay the cost with the first energy card */
        } else if (c && c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "stage")) {
            stage_choice_filtered = c->n_filtered_indices;
            answer(&tg, 1);   /* pick the SECOND eligible member */
        } else {
            answer(&tg, 1);   /* SelectTarget -> Pay */
        }
    }

    CHECK_EQ(stage_choice_filtered, 2, "both eligible members must be selectable");
    CHECK_ORI(orientation_of(&tg, low_b), ORI_WAIT,
              "the SECOND member (index 1) chosen by the player is waited");
    CHECK_ORI(orientation_of(&tg, low_a), ORI_NONE,
              "the FIRST member must NOT be waited when the second was chosen");
}

static void test_karin_cost_energy_choice_selects_chosen_card(void)
{
    TestGame tg;
    test_game_new(&tg);
    int karin = karin_setup(&tg, 0);
    int enemy = test_id(&tg, ENEMY_B1);
    int e2    = test_id(&tg, ENERGY);
    if (karin < 0 || enemy < 0 || e2 < 0) { CHECK(0, "PL!N-bp7-004-R energy-choice fixtures resolve"); return; }
    set_p2_stage3(&tg, enemy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    /* Second energy card so the cost really is a choice. */
    RbPlayer *P = &tg.state.p[0];
    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = e2;

    CHECK(test_activate_ability(&tg, karin) != 0, "the 起動 is activated");

    int saw_energy_choice = 0;
    int energy_count_ok = 0;
    int guard = 0;
    while (pending(&tg) && guard++ < 20) {
        const RbChoice *c = pending_choice(&tg);
        if (c && c->kind == RB_CHOICE_SELECT_CARD &&
            (!strcmp(c->zone, "energy_zone") || !strcmp(c->zone, "energy"))) {
            saw_energy_choice = 1;
            energy_count_ok = (c->count == 1);
            answer(&tg, 1); /* pick the SECOND energy card */
        } else if (c && c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "stage")) {
            answer(&tg, 0);
        } else {
            answer(&tg, 1);
        }
    }

    CHECK(saw_energy_choice, "an energy-card selection choice must be offered");
    CHECK(energy_count_ok, "must place exactly 1 energy under");
    CHECK_EQ(under_n(&tg, 0, 1), 1, "one energy placed under 果林");
    CHECK(under_has(&tg, 0, 1, e2),
          "the SECOND (chosen) energy card is the one placed — the cost offers the "
          "actual zone cards rather than auto-picking the first");
}

/* ══════════════════════════════════════════════════════════════════════
 * J. sp_bp5_choice_energy_test.rs
 *    PL!SP-bp5-001-R＋ かのん — choice behind an optional pay_energy cost
 * ══════════════════════════════════════════════════════════════════════ */

#define KANON "PL!SP-bp5-001-R＋"

static int kanon_deploy(TestGame *tg)
{
    int card = test_id(tg, KANON);
    if (card < 0) return -1;
    test_add_to_hand(tg, card);
    test_give_energy(tg, 15);
    if (!test_play_to_stage(tg, card, 1)) return -1;
    return card;
}

static void test_kanon_pay_and_draw(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card = test_id(&tg, KANON);
    int deck_card = test_id(&tg, "PL!-sd1-010-SD");
    if (card < 0 || deck_card < 0) { CHECK(0, "PL!SP-bp5-001-R+ fixtures resolve"); return; }
    check_identity(&tg, card, KANON, "the 澁谷かのん debut print");

    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = deck_card;
    int deck_before = P->deck.n;

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }

    /* prompt 1 of 2: the pay-or-skip cost gate */
    CHECK(pending(&tg), "prompt 1 of 2: the optional cost is offered first");
    const RbChoice *c = pending_choice(&tg);
    CHECK(c && c->kind == RB_CHOICE_SELECT_TARGET, "prompt 1 is a SelectTarget");
    CHECK(c && !strncmp(c->target, "pay_optional_cost", 18),
          "prompt 1 is the pay-or-skip cost gate");
    CHECK(c && c->allow_skip, "the optional cost must allow skipping");
    answer(&tg, 1); /* pay */

    /* prompt 2 of 2: the two-option choice */
    c = pending_choice(&tg);
    CHECK(c && c->kind == RB_CHOICE_SELECT_TARGET,
          "prompt 2 of 2: the two-option choice comes after the cost is settled");
    CHECK(c && !strcmp(c->target, "choice"), "prompt 2 is the two-option choice");
    answer(&tg, 1); /* the draw line */

    CHECK(hand_has(&tg, deck_card), "the draw line must bring the deck card into hand");
    CHECK(!deck_has(&tg, deck_card), "the drawn card must have LEFT the deck");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 1, "the draw line is exactly one card");
    CHECK(!pending(&tg), "the draw resolves on its own; nothing is left to answer");
}

static void test_kanon_pay_and_wait_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, KANON);
    int cheap  = test_id(&tg, "PL!-sd1-010-SD");    /* cost 4 — the filter boundary */
    int pricey = test_id(&tg, "PL!-pb1-021-PR");    /* cost 5 — over the cap */
    int d1     = test_id(&tg, "PL!-sd1-010-SD");
    if (card < 0 || cheap < 0 || pricey < 0) { CHECK(0, "PL!SP-bp5-001-R+ wait fixtures resolve"); return; }
    CHECK_EQ(card_cost(cheap), 4, "setup: the cost-4 member sits on the filter boundary");
    CHECK_EQ(card_cost(pricey), 5, "setup: the cost-5 member is over the cap");
    set_p2_stage3(&tg, cheap, pricey, RB_EMPTY_SLOT);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = d1;

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }
    answer(&tg, 1); /* pay */
    answer(&tg, 0); /* the wait line */

    CHECK(!pending(&tg), "one eligible member and a count-1 effect resolve without a prompt");
    CHECK_ORI(orientation_of(&tg, cheap), ORI_WAIT, "the cost-4 opponent member must be waited");
    CHECK_ORI(orientation_of(&tg, pricey), ORI_NONE,
              "the cost-5 member is out of the filter's range and must be untouched");
    CHECK(!hand_has(&tg, d1), "the wait line was chosen, so no card was drawn");
    CHECK_EQ(tg.state.p[1].stage[0], cheap, "a wait never moves a member between areas");
    CHECK_EQ(tg.state.p[1].stage[1], pricey, "a wait never moves a member between areas");
    CHECK_EQ(tg.state.p[1].stage[2], RB_EMPTY_SLOT, "the opponent's empty slot stays empty");
}

static void test_kanon_waits_only_members_at_or_below_cost_four(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card   = test_id(&tg, KANON);
    int cheap4 = test_id(&tg, "PL!-sd1-010-SD");    /* cost 4 — inside */
    int cheap2 = test_id(&tg, "PL!-sd1-002-SD");    /* cost 2 — inside */
    int pricey5 = test_id(&tg, "PL!-pb1-021-PR");    /* cost 5 — outside */
    if (card < 0 || cheap4 < 0 || cheap2 < 0 || pricey5 < 0) {
        CHECK(0, "PL!SP-bp5-001-R+ two-target fixtures resolve");
        return;
    }
    CHECK_EQ(card_cost(cheap4), 4, "setup: the boundary member costs 4");
    CHECK_EQ(card_cost(cheap2), 2, "setup: the second eligible member costs 2");
    CHECK_EQ(card_cost(pricey5), 5, "setup: the excluded member costs 5");
    set_p2_stage3(&tg, cheap4, cheap2, pricey5);

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }
    answer(&tg, 1); /* pay */
    answer(&tg, 0); /* the wait line */

    CHECK(pending(&tg), "with two members inside the cost filter, the wait must ask which one");
    CHECK_EQ(strcmp(pending_kind(&tg), "SelectCard"), 0,
             "more than one candidate -> a CARD selection, not a target list");
    answer(&tg, 0);
    drain_skip(&tg, 8);

    CHECK_EQ((orientation_of(&tg, cheap4) == ORI_WAIT) +
             (orientation_of(&tg, cheap2) == ORI_WAIT), 1,
             "the wait line is exactly one member, not all eligible ones");
    CHECK_ORI(orientation_of(&tg, pricey5), ORI_NONE,
              "the cost-5 member must never be waited, whatever the player picked");
}

static void test_kanon_decline_cost_no_effect(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card     = test_id(&tg, KANON);
    int opponent = test_id(&tg, "PL!-sd1-010-SD");
    int deck_card = test_id(&tg, "PL!-sd1-010-SD");
    if (card < 0 || opponent < 0) { CHECK(0, "PL!SP-bp5-001-R+ decline fixtures resolve"); return; }
    CHECK_EQ(card_cost(opponent), 4, "setup: a cost-4 target IS available");
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opponent, RB_EMPTY_SLOT);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = deck_card;

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }
    int energy_after_play = tg.state.p[0].energy_active;

    CHECK(pending(&tg), "even with the energy available the cost is optional, so it is offered");
    answer(&tg, 0); /* skip */
    drain_first(&tg, 12);

    CHECK_EQ(tg.state.p[0].energy_active, energy_after_play,
             "declining the optional cost must not spend the E");
    CHECK(!hand_has(&tg, deck_card), "declining the cost resolves the whole ability: no draw");
    CHECK_ORI(orientation_of(&tg, opponent), ORI_NONE,
              "declining the cost resolves the whole ability: no wait");
    CHECK(!waitroom_has(&tg, deck_card),
          "declining the cost must not discard anything either");
}

static void test_kanon_pay_and_choose_draw_is_exclusive(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card      = test_id(&tg, KANON);
    int opponent  = test_id(&tg, "PL!-sd1-010-SD");
    int deck_card = test_id(&tg, "PL!-sd1-010-SD");
    if (card < 0 || opponent < 0) { CHECK(0, "PL!SP-bp5-001-R+ exclusivity fixtures resolve"); return; }
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opponent, RB_EMPTY_SLOT);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = deck_card;

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }
    answer(&tg, 1); /* pay */
    answer(&tg, 1); /* the draw line */

    CHECK(hand_has(&tg, deck_card), "the draw line must bring the deck card to hand");
    CHECK(!deck_has(&tg, deck_card), "the drawn card left the deck");
    CHECK_ORI(orientation_of(&tg, opponent), ORI_NONE,
              "the two lines are exclusive: taking the draw must not also wait the member, "
              "even though a cost-4 target was available");
    CHECK(!pending(&tg), "the draw line opens no further prompt");
}

static void test_kanon_decline_payment_has_no_follow_up(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card      = test_id(&tg, KANON);
    int opponent  = test_id(&tg, "PL!-sd1-010-SD");
    int deck_card = test_id(&tg, "PL!-sd1-010-SD");
    if (card < 0 || opponent < 0) { CHECK(0, "PL!SP-bp5-001-R+ decline follow-up fixtures resolve"); return; }
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opponent, RB_EMPTY_SLOT);
    RbPlayer *P = &tg.state.p[0];
    P->deck.n = 0;
    P->deck.cards[P->deck.n++] = deck_card;

    if (kanon_deploy(&tg) < 0) { CHECK(0, "かのん is played to centre"); return; }
    int energy_after_play = tg.state.p[0].energy_active;
    int hand_before = tg.state.p[0].hand.n;

    const RbChoice *c = pending_choice(&tg);
    CHECK(c && !strncmp(c->target, "pay_optional_cost", 18),
          "the pay-or-skip gate comes first");
    answer(&tg, 0); /* skip the cost */
    drain_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, opponent), ORI_NONE,
              "declining the cost must not wait the opponent member");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "declining the cost must not draw");
    CHECK_EQ(tg.state.p[0].energy_active, energy_after_play,
             "declining the optional cost must not spend the E");
    CHECK(deck_has(&tg, deck_card), "the deck card is still on the deck, not consumed by the skip");
    CHECK(!hand_has(&tg, deck_card), "the deck card never reached the hand");
}

/* ══════════════════════════════════════════════════════════════════════
 * K. all_liella_gated_opponent_wait_test.rs
 *    PL!SP-pb2-047-L — optional discard-1 cost, then an all-Liella-gated wait
 * ══════════════════════════════════════════════════════════════════════ */

#define WELCOME  "PL!SP-pb2-047-L"
#define LIELLA_A "PL!SP-pb2-036-N"   /* cost 5, Superstar */
#define LIELLA_B "PL!SP-pb2-037-N"   /* cost 5, Superstar */
#define ENEMY_C2 "PL!SP-PR-010-PR"   /* cost 2 */

static void welcome_board(TestGame *tg, const char *second_member, int *enemy)
{
    int filler = test_id(tg, "PL!-sd1-010-SD");
    int live   = test_id(tg, WELCOME);
    int l1     = test_id(tg, LIELLA_A);
    int l2     = test_id(tg, second_member);
    int foe    = test_id(tg, ENEMY_C2);
    *enemy = foe;
    if (live < 0 || l1 < 0 || l2 < 0 || foe < 0) return;
    fill_decks(tg, filler, 20);
    test_add_to_live(tg, live);
    set_p1_stage3(tg, l1, l2, RB_EMPTY_SLOT);
    set_p2_stage3(tg, foe, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK(card_prints_trigger(live, RB_TSTR_LIVE_START),
          "setup: PL!SP-pb2-047-L really prints a ライブ開始時");
}

static void test_welcome_all_liella_waits_cost2_enemy(void)
{
    TestGame tg;
    test_game_new(&tg);
    int enemy;
    welcome_board(&tg, LIELLA_B, &enemy);
    if (enemy < 0) { CHECK(0, "PL!SP-pb2-047-L fixtures resolve"); return; }
    check_identity(&tg, enemy, ENEMY_C2, "the cost-2 KALEIDOSCORE enemy");
    CHECK_EQ(card_cost(enemy), 2, "setup: the enemy really costs 2");
    int fodder = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, fodder);

    CHECK(fire_trigger(&tg, test_id(&tg, WELCOME), RB_TSTR_LIVE_START),
          "the ライブ開始時 is queued");
    CHECK(pending(&tg), "optional discard cost prompt expected");
    CHECK_EQ(strcmp(pending_kind(&tg), "SelectCard"), 0,
             "expected SelectCard for the discard cost");
    answer(&tg, 0); /* accept the discard */
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, enemy), ORI_WAIT,
              "all-Liella stage -> the enemy cost<=2 member is waited");
}

static void test_welcome_non_liella_stage_does_not_wait_enemy(void)
{
    TestGame tg;
    test_game_new(&tg);
    int enemy;
    welcome_board(&tg, "PL!-sd1-010-SD", &enemy); /* a μ's member breaks "all" */
    if (enemy < 0) { CHECK(0, "PL!SP-pb2-047-L negative fixtures resolve"); return; }
    int fodder = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, fodder);

    CHECK(fire_trigger(&tg, test_id(&tg, WELCOME), RB_TSTR_LIVE_START),
          "the ライブ開始時 is queued with a non-Liella member staged");
    CHECK(pending(&tg),
          "the optional discard cost is offered even when the wait condition fails");
    CHECK_EQ(strcmp(pending_kind(&tg), "SelectCard"), 0,
             "expected SelectCard for the discard cost");
    answer(&tg, 0);
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, enemy), ORI_NONE,
              "non-Liella present -> the enemy is not waited (the all-Liella arm above "
              "waits the same fixture)");
}

/* ══════════════════════════════════════════════════════════════════════
 * L. maki_optional_self_wait_cost_opponent_own_wait_test.rs
 *    PL!-pb1-015-R 真姫 — optional BiBi-wait cost, opponent waits 1 of their own
 * ══════════════════════════════════════════════════════════════════════ */

#define MAKI "PL!-pb1-015-R"
#define MAKI_P2_C2  "PL!-pb1-011-R"  /* cost 2 */
#define MAKI_P2_C4  "PL!-pb1-009-R"  /* cost 4 */
#define MAKI_P2_C13 "PL!-pb1-002-R"  /* cost 13 */

static void test_maki_debut_pay_cost_waits_exactly_one_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int maki = test_id(&tg, MAKI);
    int p2_a = test_id(&tg, MAKI_P2_C2);
    int p2_b = test_id(&tg, MAKI_P2_C4);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (maki < 0 || p2_a < 0 || p2_b < 0) { CHECK(0, "PL!-pb1-015-R fixtures resolve"); return; }
    check_identity(&tg, maki, MAKI, "the 西木野真姫 debut print");
    CHECK_EQ(card_cost(p2_a), 2, "setup: opponent member A really costs 2");
    CHECK_EQ(card_cost(p2_b), 4, "setup: opponent member B really costs 4");
    set_p2_stage3(&tg, p2_a, p2_b, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, filler);

    CHECK(test_play_to_stage(&tg, maki, 1), "真姫 is played to centre");
    test_give_energy(&tg, 11);
    drain_card_then_option(&tg, 20);

    CHECK_ORI(orientation_of(&tg, maki), ORI_WAIT, "真姫 should be in wait state (cost paid)");
    int waited = (orientation_of(&tg, p2_a) == ORI_WAIT) + (orientation_of(&tg, p2_b) == ORI_WAIT);
    CHECK_EQ(waited, 1, "exactly 1 opponent member should be waited by 真姫's effect");
}

static void test_maki_debut_skip_cost_no_effect(void)
{
    TestGame tg;
    test_game_new(&tg);
    int maki = test_id(&tg, MAKI);
    int p2m  = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (maki < 0 || p2m < 0) { CHECK(0, "PL!-pb1-015-R skip-cost fixtures resolve"); return; }
    set_p2_stage3(&tg, p2m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 11);
    test_add_to_hand(&tg, filler);

    CHECK(test_play_to_stage(&tg, maki, 1), "真姫 is played to centre");
    int energy_after_play = tg.state.p[0].energy_active;

    /* Skip the optional cost: the FIRST prompt is the gate, answered with 0. */
    if (pending(&tg)) answer(&tg, 0);
    drain_card_then_option(&tg, 20);

    CHECK(orientation_of(&tg, maki) == ORI_NONE || orientation_of(&tg, maki) == ORI_ACTIVE,
          "真姫 should remain active when the cost is skipped");
    CHECK_EQ(tg.state.p[1].stage[0], p2m, "the opponent's member must still be on their stage");
    CHECK_ORI(orientation_of(&tg, p2m), ORI_NONE,
              "declining the optional cost must wait NO opponent member — ab#0's whole "
              "effect IS the cost (the paid arm above waits one of the same two)");
    CHECK(tg.state.p[0].energy_active >= energy_after_play,
          "declining the optional cost spends nothing beyond the play cost");
}

static void test_maki_ab1_draws_on_cost4_opponent_waited(void)
{
    TestGame tg;
    test_game_new(&tg);
    int maki   = test_id(&tg, MAKI);
    int p2m    = test_id(&tg, MAKI_P2_C4);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int extra  = test_id(&tg, "PL!-sd1-013-SD");
    if (maki < 0 || p2m < 0 || extra < 0) { CHECK(0, "PL!-pb1-015-R ab#1 draw fixtures resolve"); return; }
    CHECK_EQ(card_cost(p2m), 4, "setup: the waited opponent member really costs 4 (<=4)");
    set_p2_stage3(&tg, p2m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 11);
    test_add_to_hand(&tg, filler);
    RbPlayer *P = &tg.state.p[0];
    if (P->deck.n < RB_MAX_DECK) P->deck.cards[P->deck.n++] = extra;

    CHECK(test_play_to_stage(&tg, maki, 1), "真姫 is played to centre");
    drain_card_then_option(&tg, 20);

    CHECK_ORI(orientation_of(&tg, p2m), ORI_WAIT,
              "the opponent's cost-4 member is waited by ab#0");
    CHECK(hand_has(&tg, extra),
          "ab#1 should draw 1 when a cost<=4 opponent member is waited");
}

static void test_maki_ab1_no_draw_on_cost_over4(void)
{
    TestGame tg;
    test_game_new(&tg);
    int maki = test_id(&tg, MAKI);
    int p2h  = test_id(&tg, MAKI_P2_C13);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    if (maki < 0 || p2h < 0) { CHECK(0, "PL!-pb1-015-R ab#1 negative fixtures resolve"); return; }
    CHECK_EQ(card_cost(p2h), 13, "setup: the opponent's member really costs 13 (>4)");
    set_p2_stage3(&tg, p2h, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 11);
    test_add_to_hand(&tg, filler);

    CHECK(test_play_to_stage(&tg, maki, 1), "真姫 is played to centre");
    int hand_before = tg.state.p[0].hand.n;
    drain_card_then_option(&tg, 20);

    CHECK(orientation_of(&tg, p2h) == ORI_WAIT || orientation_of(&tg, p2h) == ORI_NONE,
          "the opponent's cost-13 member is over ab#0's own printed range, so this arm "
          "only pins the DRAW");
    CHECK(tg.state.p[0].hand.n <= hand_before,
          "ab#1 must NOT draw when the waited opponent member costs > 4");
}

/* ══════════════════════════════════════════════════════════════════════
 * M. optional_self_wait_target_pl_pr_007_pr_test.rs
 *    PL!-PR-007-PR 希 — optional self-wait, then wait ONE chosen opponent member
 * ══════════════════════════════════════════════════════════════════════ */

static void test_nozomi_optional_self_wait_keeps_selected_opponent_on_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    int nozomi = test_id(&tg, "PL!-PR-007-PR");
    int t1     = test_id(&tg, "PL!-sd1-010-SD");
    int t2     = test_id(&tg, "PL!-sd1-011-SD");
    if (nozomi < 0 || t1 < 0 || t2 < 0) { CHECK(0, "PL!-PR-007-PR fixtures resolve"); return; }
    check_identity(&tg, nozomi, "PL!-PR-007-PR", "the 東條希 debut print");

    set_p2_stage3(&tg, RB_EMPTY_SLOT, t1, t2); /* Center = t1, LeftSide = t2 */
    test_add_to_hand(&tg, nozomi);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, nozomi, 2), "希 is played to RightSide");
    CHECK(pending(&tg), "希 should wait for the optional cost choice");
    answer(&tg, 1); /* pay */

    CHECK_ORI(orientation_of(&tg, nozomi), ORI_WAIT, "希 should be in wait state");
    CHECK(pending(&tg), "希 should wait for the effect target choice");
    answer(&tg, 1);
    drain_skip(&tg, 8);

    CHECK_EQ(tg.state.p[1].stage[1], t1, "the target member should still be on stage at Center");
    CHECK_ORI(orientation_of(&tg, t1), ORI_WAIT, "the chosen target should be in wait state");
    CHECK_ORI(orientation_of(&tg, t2), ORI_NONE, "the unchosen member is untouched");
}

/* ══════════════════════════════════════════════════════════════════════
 * N. self_discard_target_wait_pl_bp6_010_n_test.rs
 *    PL!-bp6-010-N 穂乃果 起動 — self_cost discard + cost-limit opponent wait
 * ══════════════════════════════════════════════════════════════════════ */

static void test_honoka_self_discard_waits_selected_target_keeps_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    int honoka = test_id(&tg, "PL!-bp6-010-N");
    int t1     = test_id(&tg, "PL!-sd1-010-SD");
    int t2     = test_id(&tg, "PL!-sd1-013-SD");
    if (honoka < 0 || t1 < 0 || t2 < 0) { CHECK(0, "PL!-bp6-010-N fixtures resolve"); return; }
    check_identity(&tg, honoka, "PL!-bp6-010-N", "the 高坂穂乃果 起動 print");

    set_p2_stage3(&tg, RB_EMPTY_SLOT, t1, t2); /* Center = t1, LeftSide = t2 */
    test_add_to_hand(&tg, honoka);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, honoka, 2), "穂乃果 is played to RightSide");
    CHECK(test_activate_ability(&tg, honoka) != 0, "the 起動 is activated");
    drain_card_then_option(&tg, 20);

    CHECK_ORI(orientation_of(&tg, honoka), ORI_WAIT,
              "the self_cost discard leaves 穂乃果 in wait, not in the waitroom");
    CHECK_EQ(tg.state.p[1].stage[1], t1, "the target should remain on stage at Center");
    CHECK(orientation_of(&tg, t1) == ORI_WAIT || orientation_of(&tg, t2) == ORI_WAIT,
          "exactly one of the two opponent members is waited");
}

/* ══════════════════════════════════════════════════════════════════════
 * O. live_start_wait_filters_pl_n_bp5_004_r_pl_sp_pr_021_pr_test.rs
 *    PL!N-bp5-004-R 果林 ライブ開始時 — 元々持つブレード (ORIGINAL blade)
 *    PL!SP-PR-021-PR かのん ライブ開始時 — hearts below 5 gate
 * ══════════════════════════════════════════════════════════════════════ */

static void test_niji_live_start_waits_original_four_blades_only(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member        = test_id(&tg, "PL!N-bp5-004-R");
    int opponent      = test_id(&tg, "PL!-PR-003-PR");
    int modified      = test_id(&tg, "PL!-sd1-010-SD");
    int filler_live   = test_id(&tg, "PL!-sd1-019-SD");
    if (member < 0 || opponent < 0 || modified < 0 || filler_live < 0) {
        CHECK(0, "PL!N-bp5-004-R fixtures resolve");
        return;
    }
    check_identity(&tg, member, "PL!N-bp5-004-R", "the 朝香果林 live-start print");
    CHECK_EQ(card_printed_blade(opponent), 4, "setup: the natural-blade-4 member really prints 4");
    CHECK(card_printed_blade(modified) != 4,
          "setup: the modified member's PRINTED blade is not 4, so only a modifier makes it 4");

    set_p1_stage3(&tg, RB_EMPTY_SLOT, member, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, opponent, modified, RB_EMPTY_SLOT);
    rb_mods_add_blade(&tg.state.mods, modified, 3); /* printed 1 + 3 == 4 */
    CHECK_EQ(card_printed_blade(modified) + test_get_blade_modifier(&tg, modified), 4,
             "setup: the second member only REACHES 4 through a blade modifier");

    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 1, test_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, filler_live);
    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, filler_live);
    advance_to_live_start(&tg);
    drain_pay_then_first(&tg, 12);

    CHECK_ORI(orientation_of(&tg, opponent), ORI_WAIT,
              "a member with natural 4 blades must be waited");
    CHECK_ORI(orientation_of(&tg, modified), ORI_NONE,
              "modified-to-4 blades does not satisfy 元々持つブレード4: not a legal target");
}

static void test_sp_pr_021_below_five_hearts_no_opponent_wait(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member      = test_id(&tg, "PL!SP-PR-021-PR");
    int opponent    = test_id(&tg, "PL!-sd1-002-SD");
    int filler_live = test_id(&tg, "PL!-sd1-019-SD");
    if (member < 0 || opponent < 0 || filler_live < 0) {
        CHECK(0, "PL!SP-PR-021-PR fixtures resolve");
        return;
    }
    check_identity(&tg, member, "PL!SP-PR-021-PR", "the 澁谷かのん live-start print");
    set_p1_stage3(&tg, RB_EMPTY_SLOT, member, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opponent, RB_EMPTY_SLOT);

    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 1, test_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, filler_live);
    advance_to_live_card_set(&tg, 5);
    test_set_live_card(&tg, 0, filler_live);
    advance_to_live_start(&tg);
    drain_skip(&tg, 12);

    CHECK(!is_waited(&tg, opponent),
          "hearts below 5 must not trigger the opponent wait");
}

/* ══════════════════════════════════════════════════════════════════════
 * P. debut_opponent_chooses_active_member_wait_q189_test.rs
 *    PL!-bp4-009-R にこ 登場 (Q189) — the OPPONENT chooses which of their own
 *    active members is waited
 * ══════════════════════════════════════════════════════════════════════ */

#define NICO "PL!-bp4-009-R"

static void test_nico_debut_waits_only_active_opponent_member(void)
{
    TestGame tg;
    test_game_new(&tg);
    int nico = test_id(&tg, NICO);
    int p2m  = test_id(&tg, "PL!-sd1-010-SD");
    if (nico < 0 || p2m < 0) { CHECK(0, "PL!-bp4-009-R fixtures resolve"); return; }
    check_identity(&tg, nico, NICO, "the 矢澤にこ debut print");

    set_p2_stage3(&tg, p2m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, nico);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, nico, 1), "にこ is played to centre");
    drain_first(&tg, 12);

    CHECK(p2_stage_has(&tg, p2m), "the opponent member should still be on stage");
    CHECK_ORI(orientation_of(&tg, p2m), ORI_WAIT,
              "Q189: the opponent's active member is put to wait and stays on stage");
}

static void test_nico_debut_choice_is_routed_to_the_opponent(void)
{
    TestGame tg;
    test_game_new(&tg);
    int nico = test_id(&tg, NICO);
    int p2_a = test_id(&tg, "PL!-sd1-010-SD");
    int p2_b = test_id(&tg, "PL!-sd1-013-SD");
    if (nico < 0 || p2_a < 0 || p2_b < 0) { CHECK(0, "PL!-bp4-009-R choice fixtures resolve"); return; }
    set_p2_stage3(&tg, p2_a, p2_b, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, nico);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, nico, 1), "にこ is played to centre");
    CHECK(pending(&tg), "the opponent must choose which member to wait with 2+ eligible");

    const RbChoice *c = pending_choice(&tg);
    CHECK(c && !strcmp(c->target_player_id, "p2"),
          "the wait-member choice must be routed to the opponent (p2)");
    CHECK(c && c->n_filtered_indices == 2,
          "both opponent stage members are offered to the chooser");

    answer(&tg, 1); /* the opponent picks index 1 */
    drain_skip(&tg, 8);

    CHECK_ORI(orientation_of(&tg, p2_b), ORI_WAIT, "the chosen member (index 1) is waited");
    CHECK(!is_waited(&tg, p2_a),
          "the unchosen member stays active (not waited) — exactly one of the two");
}

/* ══════════════════════════════════════════════════════════════════════
 * Q. hand_debut_no_wait_pl_s_bp6_001_r_test.rs
 *    PL!S-bp6-001-R 千歌 登場 — appearing from the WAITROOM (not the hand) is
 *    what arms the wait; a hand debut must not
 * ══════════════════════════════════════════════════════════════════════ */

static void test_shion_hand_debut_leaves_opponent_unwaited(void)
{
    TestGame tg;
    test_game_new(&tg);
    int shion = test_id(&tg, "PL!S-bp6-001-R");
    int opp   = test_id(&tg, "PL!-sd1-010-SD");
    if (shion < 0 || opp < 0) { CHECK(0, "PL!S-bp6-001-R fixtures resolve"); return; }
    check_identity(&tg, shion, "PL!S-bp6-001-R", "the 高海千歌 debut print");

    set_p2_stage3(&tg, opp, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, shion);
    test_give_energy(&tg, 4);

    CHECK(test_play_to_stage(&tg, shion, 1), "千歌 debuted from hand");
    drain_skip(&tg, 12);
    CHECK(tg.state.p[0].stage[1] == shion, "千歌 occupies the centre area");
    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "appearing from the HAND (not the waitroom) must not trigger the wait");
}

/* ══════════════════════════════════════════════════════════════════════
 * R. debut_low_blade_wait_immunity_test.rs + center_optional_unit_wait_…_test
 *    松浦果南 PL!S-bp7-003-R＋ cannot_wait_by_effect — an opponent's wait
 *    effect must not touch a protected member.
 * ══════════════════════════════════════════════════════════════════════ */

#define KANAN "PL!S-bp7-003-R＋"

/* Establish the immunity on p2's own 果南, mirroring
 * bp7_wait_immunity_helpers::p2_establish_wait_immunity. The C engine has no
 * wait_immune_members list, so the immunity is established by actually
 * PLAYING 果南 for player 2 and answering its debut choices — if the C engine
 * never records the immunity, the assertions below go red, which is the
 * correct signal. */
static int p2_establish_wait_immunity(TestGame *tg)
{
    int kanan = test_id(tg, KANAN);
    int filler = test_id(tg, "PL!-sd1-010-SD");
    int energy = test_id(tg, ENERGY);
    if (kanan < 0 || filler < 0) return -1;

    /* Flip the seat: p2 plays first. */
    tg->state.first_attacker = 1;
    tg->state.second_attacker = 0;

    RbPlayer *P2 = &tg->state.p[1];
    P2->deck.n = 0;
    for (int i = 0; i < 20; i++) P2->deck.cards[P2->deck.n++] = filler;
    if (P2->hand.n < RB_MAX_ZONE) P2->hand.cards[P2->hand.n++] = kanan;
    for (int i = 0; i < 10; i++) {
        if (P2->energy.n < RB_MAX_ZONE) P2->energy.cards[P2->energy.n++] = energy;
        if (P2->energy_active < RB_MAX_ENERGY_CARDS) P2->energy_active++;
    }

    int played = rb_play_member(&tg->state, 1, P2->hand.n - 1, 1);
    if (!played) {
        tg->state.first_attacker = 0;
        tg->state.second_attacker = 1;
        return -1;
    }
    /* bp7_wait_immunity_helpers.rs:51-64 — SelectCard takes 0, else option 0. */
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) {
        if (!strcmp(test_pending_choice_type(tg), "SelectCard")) answer(tg, 0);
        else answer(tg, 0);
    }
    tg->state.first_attacker = 0;
    tg->state.second_attacker = 1;
    return kanan;
}

static void test_katsuraki_all_members_wait_blocked_by_immunity(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p2_kanan = p2_establish_wait_immunity(&tg);
    if (p2_kanan < 0) { CHECK(0, "p2 can establish the wait immunity"); return; }
    check_identity(&tg, p2_kanan, KANAN, "p2's 果南 print");
    CHECK(p2_stage_has(&tg, p2_kanan), "player2's 果南 is on their stage");
    CHECK_EQ(card_printed_blade(p2_kanan), 2,
             "setup: 果南 prints blade 2, inside her own blade<=3 protection window");

    int katsuraki = test_id(&tg, "PL!HS-pb1-008-R");
    if (katsuraki < 0) { CHECK(0, "PL!HS-pb1-008-R resolves"); return; }
    test_add_to_hand(&tg, katsuraki);
    test_give_energy(&tg, 20);

    CHECK(test_play_to_stage(&tg, katsuraki, 1), "桂城 is played to centre");
    drain_first(&tg, 16);

    CHECK(!is_waited(&tg, p2_kanan),
          "cannot_wait_by_effect: 桂城's all-members wait must be blocked for the "
          "protected member");
}

static void test_nico_opponent_wait_blocked_by_immunity(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p2_kanan = p2_establish_wait_immunity(&tg);
    if (p2_kanan < 0) { CHECK(0, "p2 can establish the wait immunity"); return; }

    int nico = test_id(&tg, NICO);
    if (nico < 0) { CHECK(0, "PL!-bp4-009-R resolves"); return; }
    test_add_to_hand(&tg, nico);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, nico, 1), "にこ is played to centre");
    drain_first(&tg, 16);

    CHECK(!is_waited(&tg, p2_kanan),
          "にこ's opponent wait must be blocked by the wait-immunity");
}

static void test_honoka_cost_limit_wait_blocked_by_immunity(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p2_kanan = p2_establish_wait_immunity(&tg);
    if (p2_kanan < 0) { CHECK(0, "p2 can establish the wait immunity"); return; }

    int honoka = test_id(&tg, "PL!-bp6-010-N");
    if (honoka < 0) { CHECK(0, "PL!-bp6-010-N resolves"); return; }
    test_add_to_hand(&tg, honoka);
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, honoka, 2), "穂乃果 is played to RightSide");
    CHECK(test_activate_ability(&tg, honoka) != 0, "穂乃果's 起動 is activated");
    drain_first(&tg, 16);

    CHECK(!is_waited(&tg, p2_kanan),
          "穂乃果's cost-limit wait must be blocked by the wait-immunity");
}

static void test_maki_opponent_own_wait_blocked_by_immunity(void)
{
    TestGame tg;
    test_game_new(&tg);
    int p2_kanan = p2_establish_wait_immunity(&tg);
    if (p2_kanan < 0) { CHECK(0, "p2 can establish the wait immunity"); return; }

    /* 真姫 ab#0: 『BiBi』1人をウェイトにしてもよい → the OPPONENT waits 1 of their
     * own active members. The only p2 member is the protected 果南. */
    int maki   = test_id(&tg, MAKI);
    int bibi   = test_id(&tg, "PL!-sd1-011-SD");
    if (maki < 0 || bibi < 0) { CHECK(0, "PL!-pb1-015-R immunity fixtures resolve"); return; }
    set_p1_stage3(&tg, bibi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, maki);
    test_give_energy(&tg, 11);

    CHECK(test_play_to_stage(&tg, maki, 1), "真姫 is played to centre");
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) {
        if (!strcmp(test_pending_choice_type(tg), "SelectCard")) answer(tg, 0);
        else answer(tg, 1);
    }
    drain_skip(&tg, 8);

    CHECK(!is_waited(&tg, p2_kanan),
          "真姫's ab#0 opponent-own-wait must be blocked by the wait-immunity");
}

/* ══════════════════════════════════════════════════════════════════════
 * S. riko_center_gated_live_start_low_blade_opponent_wait_test.rs
 *    PL!S-bp5-002-R＋ 梨子 ライブ開始時/センター —
 *    location_condition: left cost == right cost, require_position_cards
 * ══════════════════════════════════════════════════════════════════════ */

#define RIKO       "PL!S-bp5-002-R＋"
#define AQOURS_C4  "PL!S-bp2-002-R"   /* cost 4 */
#define AQOURS_C2  "PL!S-PR-025-PR"   /* cost 2 */

static void riko_setup(TestGame *tg, int *riko, int *opp)
{
    int r = test_id(tg, RIKO);
    int f = test_id(tg, "PL!-sd1-010-SD");
    *riko = r; *opp = f;
    if (r < 0 || f < 0) return;
    fill_decks(tg, f, 15);
    test_give_energy(tg, 15);
    set_p1_stage3(tg, RB_EMPTY_SLOT, r, RB_EMPTY_SLOT);
}

static void riko_run_live_start(TestGame *tg)
{
    int live = test_id(tg, "PL!-sd1-020-SD");
    if (live < 0) return;
    test_add_to_hand(tg, live);
    advance_to_live_card_set(tg, 5);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg);
    drain_skip(tg, 12);
}

static void test_riko_equal_cost_both_occupied_triggers(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ fixtures resolve"); return; }
    check_identity(&tg, riko, RIKO, "the 桜内梨子 live-start print");
    int c4 = test_id(&tg, AQOURS_C4);
    if (c4 < 0) { CHECK(0, "the cost-4 side member resolves"); return; }
    CHECK_EQ(card_cost(c4), 4, "setup: the side member really costs 4");

    set_stage(&tg, 0, 0, c4);
    set_stage(&tg, 0, 2, c4);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_WAIT,
              "low-blade opponent member should be wait when both side costs are equal");
}

static void test_riko_different_costs_no_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ negative fixtures resolve"); return; }
    int c4 = test_id(&tg, AQOURS_C4);
    int c2 = test_id(&tg, AQOURS_C2);
    if (c4 < 0 || c2 < 0) { CHECK(0, "the side members resolve"); return; }
    CHECK_EQ(card_cost(c4), 4, "setup: left really costs 4");
    CHECK_EQ(card_cost(c2), 2, "setup: right really costs 2");

    set_stage(&tg, 0, 0, c4);
    set_stage(&tg, 0, 2, c2);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "different side costs -> the activation condition is unmet (the equal-cost "
              "arm above waits this same opponent member)");
}

static void test_riko_left_empty_no_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ left-empty fixtures resolve"); return; }
    int c4 = test_id(&tg, AQOURS_C4);
    if (c4 < 0) { CHECK(0, "the cost-4 side member resolves"); return; }
    set_stage(&tg, 0, 0, RB_EMPTY_SLOT);
    set_stage(&tg, 0, 2, c4);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "require_position_cards: left empty -> no trigger");
}

static void test_riko_right_empty_no_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ right-empty fixtures resolve"); return; }
    int c4 = test_id(&tg, AQOURS_C4);
    if (c4 < 0) { CHECK(0, "the cost-4 side member resolves"); return; }
    set_stage(&tg, 0, 0, c4);
    set_stage(&tg, 0, 2, RB_EMPTY_SLOT);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "require_position_cards: right empty -> no trigger");
}

static void test_riko_both_empty_no_trigger(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ both-empty fixtures resolve"); return; }
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_NONE,
              "require_position_cards: both sides empty -> no trigger");
}

static void test_riko_same_cost_two_triggers(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ same-cost fixtures resolve"); return; }
    int c2 = test_id(&tg, AQOURS_C2);
    if (c2 < 0) { CHECK(0, "the cost-2 side member resolves"); return; }
    CHECK_EQ(card_cost(c2), 2, "setup: both side members really cost 2");
    set_stage(&tg, 0, 0, c2);
    set_stage(&tg, 0, 2, c2);
    set_p2_stage3(&tg, RB_EMPTY_SLOT, opp, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp), ORI_WAIT,
              "two distinct cost-2 side members still satisfy left == right");
}

static void test_riko_waits_all_repeated_opponent_instances(void)
{
    TestGame tg;
    test_game_new(&tg);
    int riko, opp;
    riko_setup(&tg, &riko, &opp);
    if (riko < 0) { CHECK(0, "PL!S-bp5-002-R+ multi-target fixtures resolve"); return; }
    int c4 = test_id(&tg, AQOURS_C4);
    int opp_a = test_new_id(&tg, "PL!S-bp2-002-R");
    int opp_b = test_new_id(&tg, "PL!S-bp2-002-R");
    if (c4 < 0 || opp_a < 0 || opp_b < 0) { CHECK(0, "the repeated instances resolve"); return; }
    CHECK(opp_a != opp_b, "two DISTINCT physical opponent members");
    set_stage(&tg, 0, 0, c4);
    set_stage(&tg, 0, 2, c4);
    set_p2_stage3(&tg, opp_a, opp_b, RB_EMPTY_SLOT);

    riko_run_live_start(&tg);
    CHECK_ORI(orientation_of(&tg, opp_a), ORI_WAIT,
              "『すべてのメンバー』 — the first low-blade opponent instance is waited");
    CHECK_ORI(orientation_of(&tg, opp_b), ORI_WAIT,
              "『すべてのメンバー』 — the second low-blade opponent instance is waited too");
}

/* ══════════════════════════════════════════════════════════════════════
 * T. state-change bookkeeping at activation time
 *
 *    None of the Rust files in this cluster assert the engine's own
 *    recently_state_changed / turn_state_changes bookkeeping for a WAIT or an
 *    ACTIVATION reached through a real board, so this section pins it from the
 *    cluster's own end-to-end flows. It is the "state change applied at
 *    activation" behaviour the cluster is named for.
 * ══════════════════════════════════════════════════════════════════════ */

static void test_yoshiko_wait_is_recorded_in_state_change_log(void)
{
    TestGame tg;
    test_game_new(&tg);
    int yoshiko, opp;
    yoshiko_setup(&tg, OPP_C2, &yoshiko, &opp);
    if (yoshiko < 0 || opp < 0) { CHECK(0, "PL!S-bp6-015-N bookkeeping fixtures resolve"); return; }
    CHECK(test_play_to_stage(&tg, yoshiko, 1), "善子 is played to centre");
    drain_skip(&tg, 12);
    CHECK_ORI(orientation_of(&tg, opp), ORI_WAIT, "setup: the cost-2 opponent is waited");

    int found = 0;
    for (int i = 0; i < tg.state.n_recently_state_changed; i++)
        if (tg.state.recently_state_changed[i] == opp) found = 1;
    CHECK(found, "the waited member is recorded in recently_state_changed");
    CHECK(tg.state.state_change_from[opp] == 0 && tg.state.state_change_to[opp] == 1,
          "active -> wait records state_change_from=0 / state_change_to=1");
    CHECK(tg.state.n_turn_state_changes >= 1,
          "the flip is recorded in turn_state_changes");
}

static void test_hanayo_activation_records_both_flips_in_one_turn(void)
{
    TestGame tg;
    test_game_new(&tg);
    int hanayo, friend;
    hanayo_deploy(&tg, &hanayo, &friend);
    if (hanayo < 0) { CHECK(0, "PL!-bp6-008-R bookkeeping fixtures resolve"); return; }

    set_p1_stage3(&tg, friend, hanayo, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, friend, "wait");
    int n_before = tg.state.n_turn_state_changes;

    CHECK(test_activate_ability(&tg, hanayo) != 0, "the 起動 is activated");
    drain_skip(&tg, 12);

    CHECK_ORI(orientation_of(&tg, hanayo), ORI_WAIT, "the self_cost wait landed");
    CHECK_ORI(orientation_of(&tg, friend), ORI_ACTIVE, "the friend's activation landed");
    CHECK(tg.state.n_turn_state_changes >= n_before + 2,
          "one 起動 produces BOTH flips (self->wait, friend->active) in turn_state_changes");
    CHECK(tg.state.last_wait_to_active_count >= 1,
          "the wait->active flip is counted in last_wait_to_active_count");
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: card database load\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    /* A. self-wait at debut, and activation of a waited member by group. */
    test_kanata_n_bp3_debut_active_self_becomes_waited();
    test_wondermates_live_start_activates_waited_nijigasaki();
    test_wondermates_activates_a_nijigasaki_never_the_outsider();
    test_wondermates_activates_nothing_without_a_nijigasaki_member();
    test_wondermates_leaves_an_already_active_nijigasaki_alone();

    /* B. optional self-wait activation cost, paid and declined. */
    test_pb1_003_declined_self_wait_preserves_active_energy();
    test_pb1_003_paid_self_wait_activates_energy_per_member();
    test_pb1_003_counts_two_physical_instances_separately();

    /* C. activation-condition gates (cost, blade, centre). */
    test_sayaka_debut_with_cost10_member_waits_cost4_opponent();
    test_sayaka_debut_without_cost10_member_leaves_opponent_active();
    test_sayaka_debut_excludes_cost5_opponent();
    test_natsumi_live_start_waits_only_low_original_blade_in_center();
    test_natsumi_from_a_side_does_nothing();

    /* D/E. cost-filtered opponent waits. */
    test_yoshiko_waits_opponent_cost2();
    test_yoshiko_leaves_cost13_opponent_pristine();
    test_yoshiko_empty_opponent_stage_offers_no_choice();
    test_yoshiko_two_cost2_choose_exactly_one();
    test_mai_debut_waits_opponent_cost2_member();
    test_mai_debut_ignores_opponent_over_cost2();

    /* F. low-blade live-start wait + the empty-board no-op. */
    test_kozuru_live_start_waits_low_blade_opponent();
    test_kozuru_empty_opponent_stage_is_a_noop();

    /* G. Q248 — activation is legal with no waited members on stage. */
    test_hanayo_activate_with_no_other_members();
    test_hanayo_activate_others_all_active_is_a_noop();
    test_hanayo_activate_with_wait_member();
    test_hanayo_use_limit_blocks_second_activation();

    /* H. 常時 phase restriction + LiveSuccess self-wait. */
    test_kanata_live_success_with_others_waits_self();
    test_kanata_live_success_no_others_does_not_wait();
    test_kanata_not_auto_activated_in_active_phase_but_others_are();

    /* I. DYNAMIC blade limit driven by the energy under the activating member. */
    test_karin_cost_places_energy_and_waits_below_limit();
    test_karin_does_not_wait_above_limit();
    test_karin_waits_only_in_limit_when_both_present();
    test_karin_dynamic_limit_scales_with_energy_under();
    test_karin_two_eligible_members_offers_selection();
    test_karin_cost_energy_choice_selects_chosen_card();

    /* J. choice behind an optional energy cost, both lines, both pay/skip. */
    test_kanon_pay_and_draw();
    test_kanon_pay_and_wait_opponent();
    test_kanon_waits_only_members_at_or_below_cost_four();
    test_kanon_decline_cost_no_effect();
    test_kanon_pay_and_choose_draw_is_exclusive();
    test_kanon_decline_payment_has_no_follow_up();

    /* K. all-Liella gated opponent wait behind a discard cost. */
    test_welcome_all_liella_waits_cost2_enemy();
    test_welcome_non_liella_stage_does_not_wait_enemy();

    /* L. optional self-wait cost feeding an opponent-own wait. */
    test_maki_debut_pay_cost_waits_exactly_one_opponent();
    test_maki_debut_skip_cost_no_effect();
    test_maki_ab1_draws_on_cost4_opponent_waited();
    test_maki_ab1_no_draw_on_cost_over4();

    /* M/N. optional self-wait + target selection, self_cost discard. */
    test_nozomi_optional_self_wait_keeps_selected_opponent_on_stage();
    test_honoka_self_discard_waits_selected_target_keeps_stage();

    /* O. ORIGINAL-blade (not current blade) live-start filter + a heart gate. */
    test_niji_live_start_waits_original_four_blades_only();
    test_sp_pr_021_below_five_hearts_no_opponent_wait();

    /* P. Q189 — the opponent chooses which of their own members is waited. */
    test_nico_debut_waits_only_active_opponent_member();
    test_nico_debut_choice_is_routed_to_the_opponent();

    /* Q. appearing from the hand does not arm the wait. */
    test_shion_hand_debut_leaves_opponent_unwaited();

    /* R. wait-immunity (cannot_wait_by_effect) blocks opponent waits. */
    test_katsuraki_all_members_wait_blocked_by_immunity();
    test_nico_opponent_wait_blocked_by_immunity();
    test_honoka_cost_limit_wait_blocked_by_immunity();
    test_maki_opponent_own_wait_blocked_by_immunity();

    /* S. location_condition (left cost == right cost) live start. */
    test_riko_equal_cost_both_occupied_triggers();
    test_riko_different_costs_no_trigger();
    test_riko_left_empty_no_trigger();
    test_riko_right_empty_no_trigger();
    test_riko_both_empty_no_trigger();
    test_riko_same_cost_two_triggers();
    test_riko_waits_all_repeated_opponent_instances();

    /* T. state-change bookkeeping reached through real activations. */
    test_yoshiko_wait_is_recorded_in_state_change_log();
    test_hanayo_activation_records_both_flips_in_one_turn();

    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("ALL PARITY WAIT ACTIVATION CHECKS PASSED\n");
    return 0;
}


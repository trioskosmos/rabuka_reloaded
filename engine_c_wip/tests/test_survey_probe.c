/* tests/test_survey_probe.c
 *
 * PROOF-OF-SURVEY probe. Written by the `tr_survey` research agent to
 * demonstrate the coverage-inventory method end to end: it ports the cheapest
 * genuinely-UNCOVERED cluster found by the survey and nothing else.
 *
 * CLUSTER:  engine/tests/test_modules/effects/look_select/recruit_stage/
 *           cost_tier_remainder_to_deck_top_routing_test.rs   (4 tests)
 *
 * WHY THIS ONE: the survey matched zero of those four Rust tests against any C
 * test function name, and the card the whole cluster exists for,
 * PL!HS-bp6-029-L ("Proof"), appears ZERO times anywhere in engine_c_wip/tests
 * (grep for "bp6-029" returns no hit) and nowhere in include/. It is a
 * self-contained single-live-card fixture: one LiveStart ability gated on a
 * stage-cost threshold, one look_and_select with a split destination (hand) +
 * remainder_destination (deck_top), and one need_heart delta. No engine change
 * is needed to reach it, so it is the cheapest real gap.
 *
 * THE CARD (cards/cards.json, PL!HS-bp6-029-L, name "Proof", type live,
 * unit DOLLCHESTRA), printed ability:
 *
 *   [live_start] If the total cost of the Hasunosora members on MY stage is 20
 *   or more, look at the top 2 cards of the deck. Add 1 of them to my hand and
 *   return the rest to the TOP of the deck. If 30 or more, ADDITIONALLY reduce
 *   this card's required hearts by 2 of heart00.
 *
 * That is a three-way threshold: <20 nothing, >=20 look2/take1/remainder-to-top,
 * >=30 additionally need_heart heart00 -2. All four Rust tests probe a
 * different rung of it, and they are ported verbatim in intent.
 *
 * ── ASCII-ONLY FILE, ON PURPOSE ───────────────────────────────────────────
 * Every literal here is ASCII. The card numbers needing a fullwidth plus are
 * spelled "\xEF\xBC\x8B" (U+FF0B), and the Japanese group/card names appear
 * only as ASCII transliterations in comments. A PowerShell
 * `-Encoding utf8` round trip mangles non-ASCII source in this repo (AGENTS.md
 * environment notes), so the source stays clean rather than being fragile.
 *
 * ── harness landmines other agents confirmed, respected here ───────────────
 *   - rb_load("src") fails under an isolated out-of-tree build, so load_card_db()
 *     MUST fall back to "../cards/build". tools/isolated_build.sh only plants
 *     $ROOT/cards/build beside the copied tree.
 *   - sizeof(GameState) is ~781 KB, so every TestGame here is `static`.
 *   - test_get_heart_modifier() REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE, which silently corrupts a heart05 read. Every heart
 *     read here goes through rb_mods_get_need_heart directly.
 *   - Rust `game.id("X")` is a SHARED template slot; C's twin is test_id().
 *     Rust `game.new_id("X")` allocates a distinct pool slot; C's twin is
 *     test_new_id(). The mapping is per-call-site, not blanket.
 *   - C's deck bag is TOP-INDEX-0, matching Rust's main_deck push order used
 *     here, so `test_add_to_deck` reproduces `cards.push(...)` exactly.
 *   - test_set_live_card is the P1 path; this cluster is P1-only.
 *   - Group identity keys off the `unit` field, and the C group string is
 *     non-ASCII, so the reference group is captured at RUNTIME from Proof
 *     itself rather than written as a literal. See pin_reference_group().
 *   - There is deliberately NO watchdog. A process watchdog is not achievable
 *     on this toolchain (documented in test_parity_characterization_extra.c):
 *     an in-child alarm() never fired and a parent-side kill(SIGKILL) does not
 *     take effect. run() is crash ISOLATION, not a hang detector.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, the suffix on the "R+" / "P+" print. */
#define PLUS "\xEF\xBC\x8B"

/* ── card DB ────────────────────────────────────────────────────────────── */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* ── crash reporting ────────────────────────────────────────────────────── */
static const char *current_test = "(none)";
static int failures;
static long assertions;
static int n_setup_bugs;

static void on_segv(int sig)
{
    fprintf(stderr,
            "\n*** signal %d inside test: %s\n"
            "*** The engine, not this test, is at fault. Assertions evaluated\n"
            "*** before the fault: %ld, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

/* ── assertions ─────────────────────────────────────────────────────────── */
#define CHECK(condition, ...) do {                                          \
        assertions++;                                                      \
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

/* ── fixtures ───────────────────────────────────────────────────────────── */

/* Rust game.id() -> shared template slot. C twin: test_id. */
static int sid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        assertions++;
        n_setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* Rust game.new_id() -> distinct pool slot. C twin: test_new_id. */
static int nid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        n_setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* AGENTS.md identity rule: pin the print before staging it. */
static int check_identity(int cid, const char *no, const char *what)
{
    char msg[160];
    snprintf(msg, sizeof msg, "setup: %s resolves to %s", what, no);
    CHECK(rb_card_no_eq(cid, no), "%s", msg);
    return rb_card_no_eq(cid, no);
}

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

static int check_cost(int cid, int expected, const char *what)
{
    char msg[160];
    snprintf(msg, sizeof msg, "setup: %s prints cost %d", what, expected);
    CHECK_EQ(card_cost(cid), expected, "%s", msg);
    return card_cost(cid) == expected;
}

/* The Hasunosora group name is a non-ASCII literal, so it is not written
 * here. rb_card_group_name() resolves a card's group from its series exactly as
 * Card::deserialize does (src/core/card.c:251), and PL!HS-bp6-029-L is itself
 * a Hasunosora live card, so "same group as Proof" IS "is Hasunosora". */
static const char *ref_group;

static void pin_reference_group(TestGame *tg)
{
    int proof = test_id(tg, "PL!HS-bp6-029-L");
    ref_group = (proof >= 0) ? rb_card_group_name(proof) : "";
    CHECK(ref_group && *ref_group,
          "setup: Proof resolves to a non-empty group (the Hasunosora identity "
          "this cluster's threshold is written against)");
}

static int same_group_as_reference(int cid)
{
    const char *g = rb_card_group_name(cid);
    return g && ref_group && *g && !strcmp(g, ref_group);
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust:
 *     while game.has_pending_choice() {
 *         match game.pending_choice_type() {
 *             Some("SelectAutoAbility") => game.select_indices(&[]),
 *             _                          => game.select_indices(&[0]),
 *         }
 *     }
 * `select_indices(&[])` is a DECLINE (rb_resume_with_choice(g, -1));
 * `select_indices(&[0])` is the first pick.
 *
 * SURVEY_DEBUG=1 names each prompt, because when a port disagrees with Rust
 * the question is always "which prompt did the engine actually raise", not a
 * guess (AGENTS.md: establish the failing baseline before theorising).
 */
static int drain_traced(TestGame *tg, int guard_max)
{
    int trace = survey_trace_on();
    int guard = 0;
    while (pending(tg) && guard < guard_max) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        const char *kind = test_pending_choice_type(tg);
        if (trace) {
            fprintf(stderr,
                    "  [drain %d] kind=%s zone=%s count=%d skip=%d "
                    "group=%s card_type=%s n_filtered=%d hand=%d deck=%d\n",
                    guard, kind ? kind : "(null)", ch ? ch->zone : "?",
                    ch ? ch->count : -1, ch ? ch->allow_skip : -1,
                    (ch && ch->filter_group[0]) ? ch->filter_group : "-",
                    (ch && ch->card_type[0]) ? ch->card_type : "-",
                    ch ? ch->n_filtered_indices : -1,
                    tg->state.p[0].hand.n, tg->state.p[0].deck.n);
        }
        if (kind && !strcmp(kind, "SelectAutoAbility")) answer(tg, -1);
        else answer(tg, 0);
        guard++;
    }
    return guard;
}

static int survey_trace_on(void)
{
    const char *dbg = getenv("SURVEY_DEBUG");
    return dbg && *dbg && strcmp(dbg, "0") != 0;
}

/* Rust `advance_to_live_card_set_p1` = 5 passes. */
static void advance_to_live_card_set_p1(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust `advance_to_live_start` = 2 passes. */
static void advance_to_live_start(TestGame *tg)
{
    for (int i = 0; i < 2; i++) {
        test_pass(tg);
        if (survey_trace_on()) {
            const char *k = pending(tg) ? test_pending_choice_type(tg) : NULL;
            fprintf(stderr, "  [pass %d] phase=%s pending=%s hand=%d deck=%d "
                            "live=%d wait=%d\n",
                    i, rb_phase_name(tg->state.phase), k ? k : "no",
                    tg->state.p[0].hand.n, tg->state.p[0].deck.n,
                    tg->state.p[0].live.n, tg->state.p[0].discard.n);
        }
    }
}

static int need_heart_h00(TestGame *tg, int cid)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, 0 /* Heart00 */);
}

/* Every TestGame is static: sizeof(GameState) is ~781 KB. */
static TestGame tg_a, tg_b, tg_c, tg_d;

/* ═════════════════════════════════════════════════════════════════════════
 * common fixture: one Proof, a cost-thresholded Hasunosora stage, a 2-card
 * top and a filler tail. The threshold counts ONLY the Hasunosora members on
 * stage, so the third area is a plain filler and is excluded from the sum --
 * the Rust originals stage [hs1, hs2, filler] and assert against 13+15 /
 * 15+15 / 9+9+4, i.e. the filler never counts.
 * ═════════════════════════════════════════════════════════════════════════ */
struct proof_board {
    int proof, hs0, hs1, top1, top2, filler;
    int p1_stage_cost_total;
};

static void build_proof_board(TestGame *tg, struct proof_board *b,
                              const char *h0, const char *h1,
                              int distinct_tops)
{
    b->proof  = sid(tg, "PL!HS-bp6-029-L");
    b->filler = sid(tg, "PL!-sd1-010-SD");
    b->hs0    = sid(tg, h0);
    b->hs1    = sid(tg, h1);

    check_identity(b->proof, "PL!HS-bp6-029-L", "the live card under test");
    pin_reference_group(tg);
    CHECK(same_group_as_reference(b->hs0), "setup: %s is in Proof's own group", h0);
    CHECK(same_group_as_reference(b->hs1), "setup: %s is in Proof's own group", h1);
    /* Negative control on the group check itself: the filler is NOT
     * Hasunosora, so a group predicate that matched everything is caught. */
    CHECK(!same_group_as_reference(b->filler),
          "setup: the filler PL!-sd1-010-SD is NOT in Proof's group");

    b->p1_stage_cost_total = card_cost(b->hs0) + card_cost(b->hs1);

    /* Rust: main_deck.cards.clear(); push(top1); push(top2); then 10 fillers.
     * C's deck bag is TOP-INDEX-0, so appending reproduces that order. */
    tg->state.p[0].deck.n = 0;
    if (distinct_tops) {
        b->top1 = nid(tg, "PL!-sd1-010-SD");
        b->top2 = nid(tg, "PL!-sd1-005-SD");
    } else {
        b->top1 = sid(tg, "PL!HS-bp1-012-PR");   /* Hasunosora, cost 4, abilityless */
        b->top2 = sid(tg, "PL!HS-bp1-012-N");    /* same slot family */
    }
    test_add_to_deck(tg, b->top1);
    test_add_to_deck(tg, b->top2);
    for (int i = 0; i < 10; i++) test_add_to_deck(tg, b->filler);

    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 1, b->filler);

    set_stage(tg, 0, 0, b->hs0);
    set_stage(tg, 0, 1, b->hs1);
    set_stage(tg, 0, 2, b->filler);

    test_add_to_hand(tg, b->proof);
}

/* ═════════════════════════════════════════════════════════════════════════
 * 1. issue12_proof_look_and_select_cost_20plus
 *    Stage 13 + 15 = 28 >= 20 but < 30 -> look2/take1 fires, no heart delta.
 * ═════════════════════════════════════════════════════════════════════════ */
static void test_issue12_proof_cost_20plus(void)
{
    struct proof_board b;
    test_game_new(&tg_a);
    build_proof_board(&tg_a, &b,
                      "PL!HS-sd1-008-SD",          /* Izumi, cost 13 */
                      "PL!HS-bp1-004-R" PLUS,     /* Tsuregi, cost 15 */
                      1);
    check_cost(b.hs0, 13, "the 13-cost Hasunosora member");
    check_cost(b.hs1, 15, "the 15-cost Hasunosora member");
    CHECK_EQ(b.p1_stage_cost_total, 28,
             "setup: Hasunosora stage cost is 28 (>=20, <30)");

    advance_to_live_card_set_p1(&tg_a);
    test_set_live_card(&tg_a, 0, b.proof);
    advance_to_live_start(&tg_a);
    drain_traced(&tg_a, 64);

    /* 12a: cost < 30 -> no heart reduction. */
    CHECK_EQ(need_heart_h00(&tg_a, b.proof), 0,
             "12a: stage cost 28 < 30 -> Proof keeps its printed need_heart heart00");
}

/* ═════════════════════════════════════════════════════════════════════════
 * 2. issue12_proof_look_and_select_cost_30plus
 *    Stage 15 + 15 = 30 -> the >=30 rung: heart00 need -2 as well.
 * ═════════════════════════════════════════════════════════════════════════ */
static void test_issue12_proof_cost_30plus(void)
{
    struct proof_board b;
    test_game_new(&tg_b);
    build_proof_board(&tg_b, &b,
                      "PL!HS-bp1-004-R" PLUS,     /* Tsuregi, cost 15 */
                      "PL!HS-bp5-002-R" PLUS,     /* Murano Sayaka, cost 15 */
                      1);
    check_cost(b.hs0, 15, "the first 15-cost Hasunosora member");
    check_cost(b.hs1, 15, "the second 15-cost Hasunosora member");
    CHECK_EQ(b.p1_stage_cost_total, 30,
             "setup: Hasunosora stage cost is 30 (>=30)");

    advance_to_live_card_set_p1(&tg_b);
    test_set_live_card(&tg_b, 0, b.proof);
    advance_to_live_start(&tg_b);
    drain_traced(&tg_b, 64);

    /* 12b: cost >= 30 -> heart00 -2 ("reduce required hearts by 2x heart00"). */
    CHECK_EQ(need_heart_h00(&tg_b, b.proof), -2,
             "12b: stage cost 30 >= 30 -> Proof's need_heart heart00 is reduced by 2");
}

/* ═════════════════════════════════════════════════════════════════════════
 * 3. proof_cost_below_20_no_effect
 *    Stage 9 < 20 -> the whole ability does nothing: nothing drawn, no delta.
 *    This is the negative control for the threshold; without it the other two
 *    could pass on a gate that is simply absent.
 * ═════════════════════════════════════════════════════════════════════════ */
static void test_proof_cost_below_20_no_effect(void)
{
    test_game_new(&tg_c);
    int proof  = sid(&tg_c, "PL!HS-bp6-029-L");
    int hs_low = sid(&tg_c, "PL!HS-bp1-005-PR");   /* Osawa Rurino, cost 9 */
    int filler = sid(&tg_c, "PL!-sd1-010-SD");
    int filler_copy2 = nid(&tg_c, "PL!-sd1-010-SD");

    check_identity(proof, "PL!HS-bp6-029-L", "the live card under test");
    pin_reference_group(&tg_c);
    CHECK(same_group_as_reference(hs_low),
          "setup: PL!HS-bp1-005-PR is in Proof's own group");
    check_cost(hs_low, 9, "the below-threshold Hasunosora member");

    tg_c.state.p[0].deck.n = 0;
    for (int i = 0; i < 12; i++) test_add_to_deck(&tg_c, filler);
    tg_c.state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg_c, 1, filler);

    set_stage(&tg_c, 0, 0, hs_low);
    set_stage(&tg_c, 0, 1, filler);
    set_stage(&tg_c, 0, 2, filler_copy2);
    test_add_to_hand(&tg_c, proof);

    advance_to_live_card_set_p1(&tg_c);
    test_set_live_card(&tg_c, 0, proof);
    advance_to_live_start(&tg_c);
    drain_traced(&tg_c, 64);

    /* Rust asserts hand.cards.len() == 1, i.e. only Proof itself. The C hand
     * holds no other card by construction, so the same claim is hand.n == 1. */
    CHECK_EQ(tg_c.state.p[0].hand.n, 1,
             "Proof: Hasunosora stage cost 9 < 20 -> the ability draws nothing");
    CHECK_EQ(need_heart_h00(&tg_c, proof), 0,
             "Proof: Hasunosora stage cost 9 < 20 -> no need_heart reduction");
}

/* ═════════════════════════════════════════════════════════════════════════
 * 4. proof_cost_20plus_draws_card
 *    Stage 9 + 9 + 4 = 22 >= 20 on ABILITYLESS Hasunosora members (so Proof's
 *    LiveStart is the only thing that can prompt), and the two cards on top of
 *    the deck are also Hasunosora, so the look-and-select has a legal pick ->
 *    the hand grows by exactly 1.
 * ═════════════════════════════════════════════════════════════════════════ */
static void test_proof_cost_20plus_draws_card(void)
{
    test_game_new(&tg_d);
    int proof  = sid(&tg_d, "PL!HS-bp6-029-L");
    int hs1    = sid(&tg_d, "PL!HS-bp1-016-PR");  /* Ginko,   cost 9, no ability */
    int hs2    = sid(&tg_d, "PL!HS-bp1-016-N");   /* Ginko,   cost 9, no ability */
    int hs3    = sid(&tg_d, "PL!HS-bp1-012-PR");  /* Kozuru,  cost 4, no ability */
    int filler = sid(&tg_d, "PL!-sd1-010-SD");
    int top1   = sid(&tg_d, "PL!HS-bp1-012-PR");  /* abilityless Hasunosora, top */
    int top2   = sid(&tg_d, "PL!HS-bp1-012-N");   /* abilityless Hasunosora, 2nd */

    check_identity(proof, "PL!HS-bp6-029-L", "the live card under test");
    pin_reference_group(&tg_d);
    CHECK(same_group_as_reference(hs1), "setup: PL!HS-bp1-016-PR is in Proof's own group");
    CHECK(same_group_as_reference(hs2), "setup: PL!HS-bp1-016-N is in Proof's own group");
    CHECK(same_group_as_reference(hs3), "setup: PL!HS-bp1-012-PR is in Proof's own group");
    CHECK(same_group_as_reference(top1), "setup: top card PL!HS-bp1-012-PR is in Proof's group");
    CHECK(same_group_as_reference(top2), "setup: 2nd card PL!HS-bp1-012-N is in Proof's group");
    check_cost(hs1, 9, "hs1");
    check_cost(hs2, 9, "hs2");
    check_cost(hs3, 4, "hs3");
    CHECK_EQ(card_cost(hs1) + card_cost(hs2) + card_cost(hs3), 22,
             "setup: Hasunosora stage cost is 22 (>=20, <30)");

    tg_d.state.p[0].deck.n = 0;
    test_add_to_deck(&tg_d, top1);
    test_add_to_deck(&tg_d, top2);
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg_d, filler);
    tg_d.state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg_d, 1, filler);

    set_stage(&tg_d, 0, 0, hs1);
    set_stage(&tg_d, 0, 1, hs2);
    set_stage(&tg_d, 0, 2, hs3);
    test_add_to_hand(&tg_d, proof);

    advance_to_live_card_set_p1(&tg_d);
    test_set_live_card(&tg_d, 0, proof);
    advance_to_live_start(&tg_d);

    /* Rust: hand_after_setup = hand.len() BEFORE draining, then take [0] until
     * no choice remains. Every arm here is abilityless, so Proof's
     * look-and-select is the only prompt. */
    int hand_after_setup = tg_d.state.p[0].hand.n;
    int prompts = drain_traced(&tg_d, 64);

    CHECK(prompts > 0,
          "Proof: the >=20 rung raises a look-and-select prompt (found none, so "
          "the hand cannot grow)");
    CHECK_EQ(tg_d.state.p[0].hand.n, hand_after_setup + 1,
             "Proof: Hasunosora stage cost 22 >= 20 -> look 2 / take 1 grows the hand by 1");
    CHECK_EQ(need_heart_h00(&tg_d, proof), 0,
             "Proof: Hasunosora stage cost 22 < 30 -> no need_heart reduction");
}

/* ── per-test isolation ──────────────────────────────────────────────────
 * In this toolchain `make t` reports SUCCESS for a killed process, so a suite
 * that aborts mid-run looks green. Each test therefore runs in a forked child
 * whose exit status carries its own failure count, and a signal-killed child
 * is reported as CRASH -- never as a pass. There is deliberately no watchdog.
 */
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
        /* No fork available: fall back to in-process, accepting that a crash
         * truncates the rest of the run. */
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0);
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions]\n", nf ? "FAILED" : "ok", name,
               (int)(assertions - a0));
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(n_setup_bugs - s0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
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
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    /* Line-buffer stdout: a crash mid-run must not swallow the results. */
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- effects/look_select/recruit_stage/"
           "cost_tier_remainder_to_deck_top_routing_test.rs ---\n");
    printf("card under test: PL!HS-bp6-029-L (Proof, live card, Hasunosora)\n");

    run("issue12_proof_look_and_select_cost_20plus",
        test_issue12_proof_cost_20plus);
    run("issue12_proof_look_and_select_cost_30plus",
        test_issue12_proof_cost_30plus);
    run("proof_cost_below_20_no_effect",
        test_proof_cost_below_20_no_effect);
    run("proof_cost_20plus_draws_card",
        test_proof_cost_20plus_draws_card);

    rb_unload();

    printf("\n=== survey_probe ===\n");
    printf("tests passed          : %d\n", n_tests_ok);
    printf("tests failed (parity) : %d\n", n_tests_failed);
    printf("tests crashed (engine): %d\n", n_tests_crashed);
    if (n_setup_bugs)
        printf("fixture print bugs    : %d\n", n_setup_bugs);

    if (n_tests_failed || n_tests_crashed) {
        fprintf(stderr,
                "SURVEY PROBE: %d failing test(s), %d crash(es)\n",
                n_tests_failed, n_tests_crashed);
        int code = n_tests_failed + n_tests_crashed;
        return code > 125 ? 125 : code;
    }
    printf("SURVEY PROBE PASSED\n");
    return 0;
}

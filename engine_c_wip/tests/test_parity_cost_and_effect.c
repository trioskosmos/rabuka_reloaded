/* test_parity_cost_and_effect.c — C parity suite for the Rust cluster
 *   engine/tests/test_modules/effects/compound/cost_and_effect/   (31 files)
 *
 * The cluster is about the COMPOUND shape of an ability: a cost that must be
 * resolved (or declined) BEFORE the effect can run, and the ordering between
 * the two. Every expectation below is taken from the Rust source of truth; the
 * governing Rust file is cited above each block.
 *
 * Behaviours prioritised (one block per distinct behaviour, not per Rust test):
 *   A. mandatory energy + self-discard cost, then the effect        (Q196)
 *   B. optional energy cost: pay / pay-0 / unaffordable / DECLINE   (Q214)
 *   C. multi-card cost: sequential re-prompt, count scaled by state (bp4-002 / pb1-007)
 *   D. count-2 hand cost must MOVE two cards, then the stage select  (G19)
 *   E. reveal cost filtered by cost_values, places the card under    (bp7-003)
 *   F. optional SelectCard recovery: accept / decline / no candidate (bp3-021)
 *   G. optional energy placement: accept / decline                  (bp5-027)
 *   H. optional member cost gate: accept / decline, 1 vs 2 candidates (bp6-021)
 *   I. opponent-forced discard: comply / decline                    (pb1-006)
 *   J. optional debut cost: mill + live recovery, pay / decline     (HS-pb1-004)
 *   K. wait-another-group-member cost: unpayable / opponent / paid  (bp3-008)
 *   L. optional EE debut cost: pay / skip / multiple candidates    (HS-bp2-018)
 *   M. optional 2-card cost -> heart colour -> stage target, and skip (HS-sd1-008)
 *   N. compound cost (wait + discard) then sacrifice/summon        (bp3-006)
 *   O. cost discard + reveal-5 + all-match blade                   (bp6-006)
 *
 * Owners: src/ability/cost.c, src/ability/choice.c, src/ability/compound.c.
 * This file is a TEST file; it never edits engine code.
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

/* ══════════════════════════════════════════════════════════════════════════
 * Local helpers
 * ══════════════════════════════════════════════════════════════════════════ */

/* Empty every zone both players own, so each card-count assertion below is
 * unambiguous. Mirrors the Rust tests, which clear main_deck / hand / waitroom
 * explicitly in their setup helpers. */
static void bare_board(TestGame *tg)
{
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
    tg->state.n_selected_cards = 0;
    tg->state.live_set_limit_reduction[0] = 0;
    tg->state.live_set_limit_reduction[1] = 0;
    tg->state.queue.deferred = NULL;
    tg->state.queue.resume_eff = NULL;
    tg->state.activation_keepalive_valid = 0;
    rb_clear_pending_choice(&tg->state);
}

/* Resolve a card number through the C lookup AND pin its identity.
 *
 * rb_find_card_by_no() has a fuzzy tail (data.c card_no_contains_lowest), so a
 * typo'd card number silently resolves to a DIFFERENT print and the test would
 * prove nothing. rb_card_no_eq() is an exact comparison against the shipped
 * card_no, so every fixture here is identity-checked (AGENTS.md identity rule). */
static int cid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        fprintf(stderr, "FIXTURE: %s does not resolve in the card DB\n", no);
        return -1;
    }
    if (!rb_card_no_eq(id, no)) {
        Card c;
        char got[64] = "?";
        if (rb_decode_card_by_index((uint32_t)id, &c)) {
            snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
            rb_free_card(&c);
        }
        fprintf(stderr,
                "FIXTURE: wanted print %s but the lookup fuzzy-matched %s (idx %d)\n",
                no, got, id);
        return -1;
    }
    return id;
}

/* Resolve a fixture pair, returning 0 (and logging) when either is missing. */
static int need(TestGame *tg, int *out, const char *no, int n)
{
    int ok = 1;
    for (int i = 0; i < n; i++) {
        out[i] = cid(tg, no);
        if (out[i] < 0) ok = 0;
    }
    return ok;
}

/* Resolve ability `idx` of `card` in the shape BOTH engine paths use for
 * "cost first, then effect":
 *
 *   Rust  resolver::resolve_ability — validate + pay the printed COST, and
 *         only then run the EFFECT. An unpayable cost aborts the resolution.
 *   C     engine.c:1089-1102 (the 登場 path) — rb_pay_cost(), and when the
 *         cost opens a choice park the effect in g->queue.deferred so that the
 *         cost's ANSWER is what runs it.
 *
 * Driving the decoded cost as a child of the effect executor instead (what
 * rb_activate_card's own bridge at engine.c:1150-1159 does) is a DIFFERENT
 * code path with different semantics — the effect executor auto-resolves an
 * exact-count pick where the cost path prompts for it. The bridge's own
 * divergence is pinned separately, in
 * test_gap_sequential_cost_root_dropped_by_bridge(). */
static int activate_ability_index(TestGame *tg, int card, int idx)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)card, (uint32_t)idx, &ab)) {
        rb_free_ability(&ab);
        return 0;
    }
    if (!ab.cost && !ab.effect) { rb_free_ability(&ab); return 0; }

    /* A 「このカードを手札から控え室に置く」 cost resolves against the card
     * being activated, so the activating card has to be published first
     * (rb_activate_card sets it from the action; engine.c's 登場 loop gets it
     * from the play). */
    tg->state.activating_card = card;
    tg->state.n_recently_moved = 0;

    int paid = 1;
    if (ab.cost) {
        paid = rb_pay_cost(&tg->state, 0, ab.cost) != 0;
        if (test_has_pending_choice(tg) && ab.effect) {
            /* engine.c:1091-1098 — the cost opened a choice, so the effect
             * becomes the captured continuation of that choice. */
            tg->state.queue.deferred = ab.effect;
            tg->state.queue.resume_host = card;
            tg->state.queue.resume_eff = ab.effect;
            /* The cost prompt's resume may dereference the tree, so hand the
             * whole ability to the keepalive and detach it from `ab`. */
            tg->state.activation_keepalive = ab;
            tg->state.activation_keepalive_valid = 1;
            ab.cost = NULL;
            ab.effect = NULL;
        }
    }
    if (ab.effect && paid && !test_has_pending_choice(tg))
        rb_execute_effect_ex(&tg->state, 0, ab.effect, card);
    rb_drain_ability_queue(&tg->state);
    rb_free_ability(&ab);
    return 1;
}

/* Release the parked ability once nothing is waiting on it (engine.c:1161). */
static void release_activation_keepalive(TestGame *tg)
{
    if (tg->state.queue.has_pending || tg->state.queue.deferred) return;
    if (!tg->state.activation_keepalive_valid) return;
    rb_free_ability(&tg->state.activation_keepalive);
    tg->state.activation_keepalive_valid = 0;
}

/* The resolver entry point, WITHOUT the rb_activate_card cost+effect bridge.
 * rb_resolve_ability runs the real cost path (dynamic counts, sequential
 * costs, filters) and is therefore the faithful way to observe the printed
 * COST; it does not carry the effect across a cost prompt, so it is used only
 * for cost-shape assertions. */
static int resolve_ability_index(TestGame *tg, int card, int idx)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)card, idx, &ab)) {
        rb_free_ability(&ab);
        return 0;
    }
    int resolved = 0;
    int r = rb_resolve_ability(&tg->state, 0, &ab, idx, card, &resolved);
    rb_free_ability(&ab);
    return r;
}

/* Answer every pending choice the way the Rust twins' generic
 *   while game.has_pending_choice() { game.select_indices(&[0]) }
 * drain does: option 1 for a SelectTarget / SelectHeartColor (the Rust
 * select_option(1) idiom), index 0 for a card pick. */
static void drain_accept(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 64) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        int kind = ch ? (int)ch->kind : (int)RB_CHOICE_NONE;
        if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_HEART_COLOR)
            rb_resume_with_choice(&tg->state, 1);
        else
            rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
        release_activation_keepalive(tg);
    }
}

/* Answer every pending choice by DECLINING it: option 0 for a pay/skip
 * SelectTarget, an empty selection (idx -1) for a SelectCard. */
static void drain_decline(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 64) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        int kind = ch ? (int)ch->kind : (int)RB_CHOICE_NONE;
        if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_HEART_COLOR)
            rb_resume_with_choice(&tg->state, 0);
        else
            rb_resume_with_choice(&tg->state, -1);
        rb_process_pending_auto_abilities(&tg->state);
        release_activation_keepalive(tg);
    }
}

/* Fire ライブ開始時 for every card in player 1's live zone. Mirrors the Rust
 * fire_live_start helper: register the auto ability, then process it. */
static void fire_live_start(TestGame *tg)
{
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Fire ライブ成功時 for player 1. The Rust twins call fire_trigger(.., LiveSuccess,
 * ..), which registers the auto ability directly. The C port gates the scan on
 * rb_should_trigger_live_success, i.e. g->live_success[pl] (triggers.c:180-185),
 * so the flag has to be raised first — a harness requirement, not a behaviour
 * under test. */
static void fire_live_success(TestGame *tg)
{
    tg->state.live_success[0] = 1;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Fill a zone with `n` copies of `card` (Rust `for _ in 0..n { push(filler) }`). */
static void fill_zone(RbBag *bag, int card, int n)
{
    bag->n = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) bag->cards[bag->n++] = card;
}

static int bag_has(const RbBag *b, int card)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == card) return 1;
    return 0;
}

static int under_count(TestGame *tg, int area)
{
    return tg->state.p[0].under_cards[area].n;
}

static int is_waited(TestGame *tg, int card)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, card);
    return o != NULL && strcmp(o, "wait") == 0;
}

/* The Rust q214 tests pick "a live card whose score is N" out of the whole
 * database; mirror that search rather than hard-coding a print. */
static int find_live_with_score(int score)
{
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        if (!rb_card_is_live((int)i)) continue;
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        int match = (int)c.score == score;
        rb_free_card(&c);
        if (match) return (int)i;
    }
    return -1;
}

/* Opt-in decoder dump: set RB_COSTEFF_DUMP=1 to print the cost/effect tree of
 * every fixture this file touches. Diagnostic only — no assertion reads it. */
static void dump_tree(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stderr);
    fprintf(stderr, "%s src=%s dst=%s tgt=%s n=%d opt=%d",
            e->action ? e->action : "(null)",
            e->source ? e->source : "-", e->destination ? e->destination : "-",
            e->target ? e->target : "-", e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, " [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?",
                e->extra_v[i] ? e->extra_v[i] : "?");
    fputc('\n', stderr);
    for (int i = 0; i < e->n_child; i++) dump_tree(e->child[i], depth + 1);
    if (e->optional_action) { for (int i=0;i<=depth;i++) fputs("  ", stderr);
        fprintf(stderr, "OPTIONAL->\n"); dump_tree(e->optional_action, depth+1); }
    if (e->conditional_action) { for (int i=0;i<=depth;i++) fputs("  ", stderr);
        fprintf(stderr, "CONDITIONAL->\n"); dump_tree(e->conditional_action, depth+1); }
}

static void dump_fixture(const char *no, int idx)
{
    int id = rb_find_card_by_no(no);
    if (id < 0) { fprintf(stderr, "DUMP %s ab#%d: NO SUCH CARD\n", no, idx); return; }
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)id, (uint32_t)idx, &ab)) {
        fprintf(stderr, "DUMP %s ab#%d: DECODE FAILED\n", no, idx);
        rb_free_ability(&ab);
        return;
    }
    fprintf(stderr, "DUMP %s (idx %d) ab#%d:\n  COST:\n", no, id, idx);
    dump_tree(ab.cost, 2);
    fprintf(stderr, "  EFFECT:\n");
    dump_tree(ab.effect, 2);
    rb_free_ability(&ab);
}

static void dump_all_fixtures(void)
{
    if (!getenv("RB_COSTEFF_DUMP")) return;
    static const char *const cards[] = {
        "PL!N-pb1-003-R", "PL!N-bp5-003-R", "PL!-bp4-002-R＋", "PL!-pb1-007-R",
        "PL!S-bp7-005-R＋", "PL!SP-bp7-003-R＋", "PL!S-bp3-021-L", "PL!SP-bp5-027-L",
        "PL!-bp6-021-L", "PL!S-pb1-006-R", "PL!HS-pb1-004-R", "PL!N-bp3-008-R＋",
        "PL!HS-bp2-018-N", "PL!HS-sd1-008-SD", "PL!S-bp3-006-R＋", "PL!-bp6-006-R＋",
    };
    for (unsigned i = 0; i < sizeof(cards) / sizeof(cards[0]); i++)
        for (int idx = 0; idx < 4; idx++) dump_fixture(cards[i], idx);
}

static const char *pending_zone(TestGame *tg)
{
    const RbChoice *ch = test_has_pending_choice(tg) ? rb_get_pending_choice(&tg->state) : NULL;
    return (ch && ch->zone[0]) ? ch->zone : "";
}

/* ══════════════════════════════════════════════════════════════════════════
 * A. Mandatory 2E + self-discard cost, THEN the effect.
 *    Rust: hand_only_self_discard_draw_group_blade_q196_test.rs
 *      桜坂しずく PL!N-pb1-003-R — 起動/2E: このカードを手札から控え室に置く：
 *      カードを1枚引き、ライブ終了時まで、自分のステージにいる『虹ヶ咲』の
 *      メンバー1人はブレードを得る。この能力は、このカードが手札にある場合のみ。
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_q196_mandatory_cost_then_draw(void)
{
    TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);

    int shizuku, filler;
    CHECK(need(&tg, &shizuku, "PL!N-pb1-003-R", 1) &&
              need(&tg, &filler, "PL!-sd1-010-SD", 1),
          "A: the Q196 fixtures resolve with the exact printed card numbers");
    if (shizuku < 0 || filler < 0) return;

    test_add_to_stage(&tg, 0, filler);
    test_add_to_stage(&tg, 1, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, shizuku);
    test_add_to_hand(&tg, filler);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    int deck_before = tg.state.p[0].deck.n;
    int hand_before = tg.state.p[0].hand.n;

    activate_ability_index(&tg, shizuku, 0);
    drain_accept(&tg);

    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 13,
             "A: the mandatory 2E leg is charged (15-2=13)");
    CHECK(!test_hand_has(&tg, shizuku),
          "A: the self-discard cost moved the activating card out of hand");
    CHECK(test_zone_has_id(&tg, 0, "discard", shizuku),
          "A: the self-discarded card lands in the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 1,
             "A: after the cost, the effect draws 1 card from the deck");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "A: net hand change is -1 cost card +1 drawn card");
}

/* A2. The hand-only gate: this 起動 must not run from the stage.
 *     Rust: hand_only_self_discard_draw_q196_needs_hand_activation
 *     The Rust twin asserts UseAbility from the stage returns Err. The C
 *     harness (test_activate_ability) falls back to rb_activate_card for a card
 *     that is not in hand, so instead of pinning the Err we pin the OBSERVABLE
 *     the Rust test also pins: no energy spent, no card moved, no draw, no
 *     prompt. */
static void test_q196_hand_only_gate_from_stage(void)
{
    TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);

    int shizuku, filler;
    if (!need(&tg, &shizuku, "PL!N-pb1-003-R", 1) ||
        !need(&tg, &filler, "PL!-sd1-010-SD", 1)) {
        CHECK(0, "A2: the Q196 hand-only fixtures resolve");
        return;
    }
    test_add_to_stage(&tg, 1, shizuku);
    test_give_energy(&tg, 15);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);
    int deck_before = tg.state.p[0].deck.n;

    activate_ability_index(&tg, shizuku, 0);
    drain_accept(&tg);

    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 15,
             "A2: a hand-only 起動 refuses to spend energy when fired from the stage");
    CHECK_EQ(tg.state.p[0].stage[1], shizuku,
             "A2: a refused 起動 leaves her on stage");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "A2: 「カードを1枚引く must not run when the hand-only gate refuses");
}

/* ══════════════════════════════════════════════════════════════════════════
 * B. Optional energy cost that scales with the recovered live card's SCORE.
 *    Rust: discard_live_recovery_score_energy_cost_q214_test.rs
 *    (桜坂しずく PL!N-bp5-003-R: 手札を1枚控え室に置く → 自分の控え室から
 *     ライブカードを1枚選ぶ → そうした場合、選んだカードのスコア分の
 *     エネルギーを支払ってもよい：支払った場合、そのカードを自分の手札に加え、
 *     自身のライブ終了時まで、ブレード×3を得る。)
 *    Four cases in the Rust file: pay (score 2), pay (score 0), unaffordable,
 *    and DECLINE.
 * ══════════════════════════════════════════════════════════════════════════ */
static void q214_setup(TestGame *tg, int shizuku, int filler, int live, int energy)
{
    bare_board(tg);
    test_add_to_stage(tg, 0, filler);
    test_add_to_stage(tg, 1, shizuku);
    test_add_to_stage(tg, 2, test_new_id(tg, "PL!-sd1-010-SD"));
    test_add_to_hand(tg, filler);                    /* the cost card */
    test_add_to_discard(tg, live);                   /* the recovery source */
    test_give_energy(tg, energy);
    tg->state.activating_card = shizuku;
}

static void test_q214_optional_score_energy_pay_and_decline(void)
{
    int shizuku_no = rb_find_card_by_no("PL!N-bp5-003-R");
    int live2 = find_live_with_score(2);
    int live0 = find_live_with_score(0);
    CHECK(shizuku_no >= 0 && live2 >= 0 && live0 >= 0,
          "B: the Q214 fixtures (桜坂しずく + a score-2 and a score-0 live) resolve");
    if (shizuku_no < 0 || live2 < 0 || live0 < 0) return;
    CHECK(rb_card_no_eq(shizuku_no, "PL!N-bp5-003-R"),
          "B: the Q214 member is the printed 桜坂しずく print");

    /* ── case 1: score-2 live, 5 energy, ACCEPT the payment ─────────────── */
    {
        TestGame tg; test_game_new(&tg);
        int shizuku, filler, filler2;
        if (!need(&tg, &shizuku, "PL!N-bp5-003-R", 1) ||
            !need(&tg, &filler, "PL!-sd1-010-SD", 1)) { CHECK(0, "B1 fixtures"); return; }
        filler2 = test_new_id(&tg, "PL!-sd1-010-SD");
        q214_setup(&tg, shizuku, filler, live2, 5);

        activate_ability_index(&tg, shizuku, 0);
        CHECK(test_has_pending_choice(&tg), "B1: the hand-discard cost prompt appears");
        CHECK(!strcmp(pending_zone(&tg), "hand"), "B1: the cost selects out of the hand");
        rb_resume_with_choice(&tg.state, 0);

        CHECK(test_has_pending_choice(&tg), "B1: the live-card pick follows the paid cost");
        rb_resume_with_choice(&tg.state, 0);

        const RbChoice *gate = test_has_pending_choice(&tg) ? rb_get_pending_choice(&tg.state) : NULL;
        CHECK(gate != NULL && gate->kind == RB_CHOICE_SELECT_TARGET,
              "B1: the optional score-energy payment is a SelectTarget gate");
        if (gate && gate->kind == RB_CHOICE_SELECT_TARGET)
            rb_resume_with_choice(&tg.state, 1);   /* accept */
        /* Answer the gate exactly once: if it re-offers, the payment never
         * happened and the ability is stuck. */
        {
            const RbChoice *again = test_has_pending_choice(&tg) ? rb_get_pending_choice(&tg.state) : NULL;
            CHECK(again == NULL || strstr(again->target, "pay_optional_cost") == NULL,
                  "B1: the optional payment gate does not loop after an accepted answer");
        }
        drain_accept(&tg);

        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 3,
                 "B1: accepting pays the live card's score in energy (5 -> 3)");
        CHECK(test_hand_has(&tg, live2),
              "B1: after paying, the recovered live card is MOVED into the hand");
        CHECK(!test_zone_has_id(&tg, 0, "discard", live2),
              "B1: the paid live card is no longer in the waitroom");
        (void)filler2;
    }

    /* ── case 2: score-0 live, 1 energy, ACCEPT (pays zero) ─────────────── */
    {
        TestGame tg; test_game_new(&tg);
        int shizuku, filler;
        if (!need(&tg, &shizuku, "PL!N-bp5-003-R", 1) ||
            !need(&tg, &filler, "PL!-sd1-010-SD", 1)) { CHECK(0, "B2 fixtures"); return; }
        q214_setup(&tg, shizuku, filler, live0, 1);

        activate_ability_index(&tg, shizuku, 0);
        drain_accept(&tg);

        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 1,
                 "B2: a score-0 live costs 0 energy, so 1 energy is untouched");
        CHECK(test_hand_has(&tg, live0),
              "B2: a score-0 live is still recovered to the hand when the cost is accepted");
        CHECK(!test_zone_has_id(&tg, 0, "discard", live0),
              "B2: the score-0 live leaves the waitroom");
    }

    /* ── case 3: score-2 live but only 1 energy → the gate auto-skips ──── */
    {
        TestGame tg; test_game_new(&tg);
        int shizuku, filler;
        if (!need(&tg, &shizuku, "PL!N-bp5-003-R", 1) ||
            !need(&tg, &filler, "PL!-sd1-010-SD", 1)) { CHECK(0, "B3 fixtures"); return; }
        q214_setup(&tg, shizuku, filler, live2, 1);

        activate_ability_index(&tg, shizuku, 0);
        drain_accept(&tg);

        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 1,
                 "B3: an unaffordable optional energy cost charges nothing");
        CHECK(!test_hand_has(&tg, live2),
              "B3: an unaffordable optional cost must NOT recover the live card");
        CHECK(test_zone_has_id(&tg, 0, "discard", live2),
              "B3: the live card stays in the waitroom when the cost is skipped");
    }

    /* ── case 4: score-2 live, 5 energy, DECLINE ────────────────────────── */
    {
        TestGame tg; test_game_new(&tg);
        int shizuku, filler;
        if (!need(&tg, &shizuku, "PL!N-bp5-003-R", 1) ||
            !need(&tg, &filler, "PL!-sd1-010-SD", 1)) { CHECK(0, "B4 fixtures"); return; }
        q214_setup(&tg, shizuku, filler, live2, 5);

        activate_ability_index(&tg, shizuku, 0);
        drain_accept(&tg);   /* pay the mandatory hand-discard cost, pick the live */

        const RbChoice *gate = test_has_pending_choice(&tg) ? rb_get_pending_choice(&tg.state) : NULL;
        CHECK(gate != NULL && gate->kind == RB_CHOICE_SELECT_TARGET,
              "B4: declining still goes through the optional payment gate");
        if (gate && gate->kind == RB_CHOICE_SELECT_TARGET)
            rb_resume_with_choice(&tg.state, 0);   /* DECLINE (option 0) */
        drain_accept(&tg);

        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 5,
                 "B4: declining the optional cost leaves the energy untouched");
        CHECK(!test_hand_has(&tg, live2),
              "B4: declining the cost must NOT move the live card to the hand");
        CHECK(test_zone_has_id(&tg, 0, "discard", live2),
              "B4: the live card stays in the waitroom when the cost is declined");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * C. Multi-card cost: the prompt count is scaled by the state, and the picks
 *    arrive SEQUENTIALLY (one re-prompt per card).
 *    Rust: success_score_six_discard_two_live_recovery_test.rs
 *    (絢瀬絵里 PL!-bp4-002-R＋: 自分の成功数が6以上なら、…手札を2枚控え室に
 *     置く：自分の控え室からライブカードを1枚手札に加える。)
 *    Rust: live_success_success_count_scales_cost_and_lilywhite_live_retrieval_
 *          pl_pb1_007_test.rs (東條 希 PL!-pb1-007-R: 手札を3枚控え室に置く,
 *          コストは成功ライブ1枚につき1減る)
 * ══════════════════════════════════════════════════════════════════════════ */

/* A live card that really is a live card, used as the recovery source. */
#define MUS_LIVE  "PL!-sd1-020-SD"
#define FILLER_M  "PL!-sd1-010-SD"

static void test_success_score_gate_offers_no_cost(void)
{
    TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);

    int eli, filler, live_a, live_b;
    if (!need(&tg, &eli, "PL!-bp4-002-R＋", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &live_a, "PL!-bp3-025-L", 1) ||
        !need(&tg, &live_b, "PL!-bp3-026-L", 1)) {
        CHECK(0, "C0: the 絢瀬絵里 fixtures resolve");
        return;
    }
    test_add_to_hand(&tg, eli);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_discard(&tg, live_a);
    test_add_to_discard(&tg, live_b);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    /* 成功数が6未満 → the whole ability is gated off and nothing prompts. */
    CHECK(test_play_to_stage(&tg, eli, 1), "C0: 絢瀬絵里 deploys to the centre stage");
    CHECK(!test_has_pending_choice(&tg),
          "C0: below the 成功数6 gate the recovery offers no cost prompt");
    /* The 起動 is ability #1 (ability #0 is her 常時 heart06 clause) — the
     * 成功数6 parenthetical must gate the activation itself. */
    activate_ability_index(&tg, eli, 1);
    drain_accept(&tg);
    CHECK(!test_has_pending_choice(&tg),
          "C0: the 起動 is refused outright below the 成功数6 gate");
    CHECK_EQ(tg.state.p[0].hand.n, 3,
             "C0: the refused 起動 discards no cost card");
}

static void test_success_score_sequential_discard_two(void)
{
    TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);

    int eli, filler, live_a, live_b;
    if (!need(&tg, &eli, "PL!-bp4-002-R＋", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &live_a, "PL!-bp3-025-L", 1) ||
        !need(&tg, &live_b, "PL!-bp3-026-L", 1)) {
        CHECK(0, "C1: the 絢瀬絵里 fixtures resolve");
        return;
    }
    test_add_to_hand(&tg, eli);
    for (int i = 0; i < 3; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_M));
    test_add_to_discard(&tg, live_a);
    test_add_to_discard(&tg, live_b);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    CHECK(test_play_to_stage(&tg, eli, 1), "C1: 絢瀬絵里 deploys to the centre stage");
    /* ab#1 is the 起動; ab#0 is her 常時 clause. */
    activate_ability_index(&tg, eli, 1);

    CHECK(test_has_pending_choice(&tg), "C1: the hand-discard cost prompts (1/2)");
    CHECK_EQ(test_pending_choice_count(&tg), 1, "C1: a pending choice is outstanding");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(test_has_pending_choice(&tg), "C1: the cost RE-PROMPTS for the second discard");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(test_has_pending_choice(&tg), "C1: after both cost cards, the live-card pick appears");
    rb_resume_with_choice(&tg.state, 0);
    drain_accept(&tg);

    CHECK(test_hand_has(&tg, live_a) || test_hand_has(&tg, live_b),
          "C1: the chosen live card is MOVED from the waitroom into the hand");
}

static void test_pb1_007_cost_scales_with_success_count(void)
{
    static const struct { int success; int cost; } rows[] = { {0, 3}, {1, 2}, {2, 1} };
    for (unsigned r = 0; r < sizeof(rows) / sizeof(rows[0]); r++) {
        /* ── the COST shape, read through the resolver's real cost path ───── */
        {
            TestGame tg;
            test_game_new(&tg);
            bare_board(&tg);
            int me, lily, filler, mus_live;
            if (!need(&tg, &me, "PL!-pb1-007-R", 1) ||
                !need(&tg, &lily, "PL!-bp3-014-N", 1) ||
                !need(&tg, &filler, FILLER_M, 1) ||
                !need(&tg, &mus_live, MUS_LIVE, 1)) {
                CHECK(0, "C2a: the 東條 希 fixtures resolve");
                return;
            }
            test_add_to_stage(&tg, 0, me);
            test_add_to_stage(&tg, 1, lily);
            test_give_energy(&tg, 10);
            for (int i = 0; i < rows[r].success; i++)
                test_add_to_success(&tg, test_new_id(&tg, "PL!N-bp1-025-L"));
            for (int i = 0; i < 5; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_M));
            test_add_to_discard(&tg, mus_live);

            resolve_ability_index(&tg, me, 0);
            CHECK(test_has_pending_choice(&tg), "C2a: the hand-discard cost prompts");
            const RbChoice *ch = rb_get_pending_choice(&tg.state);
            char msg[128];
            snprintf(msg, sizeof(msg),
                     "C2a: %d success live card(s) scale 手札を3枚 to a cost of %d (got %d)",
                     rows[r].success, rows[r].cost, ch ? ch->count : -1);
            CHECK(ch != NULL && ch->count == rows[r].cost, msg);
        }

        /* ── the EFFECT, after the scaled cost has been paid ──────────────── */
        {
            TestGame tg;
            test_game_new(&tg);
            bare_board(&tg);
            int me, lily, filler, mus_live;
            if (!need(&tg, &me, "PL!-pb1-007-R", 1) ||
                !need(&tg, &lily, "PL!-bp3-014-N", 1) ||
                !need(&tg, &filler, FILLER_M, 1) ||
                !need(&tg, &mus_live, MUS_LIVE, 1)) { CHECK(0, "C2b fixtures"); return; }
            test_add_to_stage(&tg, 0, me);
            test_add_to_stage(&tg, 1, lily);
            test_give_energy(&tg, 10);
            for (int i = 0; i < rows[r].success; i++)
                test_add_to_success(&tg, test_new_id(&tg, "PL!N-bp1-025-L"));
            for (int i = 0; i < 5; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_M));
            test_add_to_discard(&tg, mus_live);

            activate_ability_index(&tg, me, 0);
            for (int i = 0; i < 8 && test_has_pending_choice(&tg); i++)
                rb_resume_with_choice(&tg.state, 0);
            drain_accept(&tg);
            CHECK(test_hand_has(&tg, mus_live),
                  "C2b: after the scaled cost is paid, the μ's live card reaches the hand");
        }
    }
}

static void test_pb1_007_insufficient_hand_and_group_filter(void)
{
    /* Insufficient hand: a cost of 3 with 2 cards cannot be paid. */
    {
        TestGame tg;
        test_game_new(&tg);
        bare_board(&tg);
        int me, lily, filler;
        if (!need(&tg, &me, "PL!-pb1-007-R", 1) ||
            !need(&tg, &lily, "PL!-bp3-014-N", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "C3 fixtures"); return; }
        test_add_to_stage(&tg, 0, me);
        test_add_to_stage(&tg, 1, lily);
        test_give_energy(&tg, 10);
        for (int i = 0; i < 2; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_M));

        activate_ability_index(&tg, me, 0);
        drain_accept(&tg);
        int mus_live = cid(&tg, MUS_LIVE);
        if (mus_live >= 0) {
            test_add_to_discard(&tg, mus_live);
            CHECK(!test_hand_has(&tg, mus_live),
                  "C3: with 2 hand cards against a cost of 3 the live card is not retrieved");
        }
    }
    /* A non-μ's live card is not retrieved even when the cost is paid in full. */
    {
        TestGame tg;
        test_game_new(&tg);
        bare_board(&tg);
        int me, lily, filler, liella_live;
        if (!need(&tg, &me, "PL!-pb1-007-R", 1) ||
            !need(&tg, &lily, "PL!-bp3-014-N", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &liella_live, "PL!S-bp2-024-L", 1)) { CHECK(0, "C4 fixtures"); return; }
        test_add_to_stage(&tg, 0, me);
        test_add_to_stage(&tg, 1, lily);
        test_give_energy(&tg, 10);
        test_add_to_success(&tg, cid(&tg, "PL!N-bp1-025-L"));   /* 1 success -> cost 2 */
        for (int i = 0; i < 5; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_M));
        test_add_to_discard(&tg, liella_live);

        activate_ability_index(&tg, me, 0);
        for (int i = 0; i < 2 && test_has_pending_choice(&tg); i++)
            rb_resume_with_choice(&tg.state, 0);
        drain_accept(&tg);
        CHECK(!test_hand_has(&tg, liella_live),
              "C4: a Liella! live card is not retrieved — the recovery is filtered to μ's");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * D. A count-2 hand cost must MOVE two cards into the waitroom, and only then
 *    does the follow-on stage select arrive.
 *    Rust: discard_two_self_and_other_group_debut_retrigger_test.rs
 *    (渡辺 曜 PL!S-bp7-005-R＋ ab#2 — 起動: 手札を2枚控え室に置く：
 *     このメンバーと自分のステージにいるほかの『Aqours』のメンバー1人を
 *     選ぶ。それらが持つ登場能力それぞれ1つを発動させる。)
 * ══════════════════════════════════════════════════════════════════════════ */
static void discard_two_setup(TestGame *tg, int watanabe, int hanamaru, int filler, int wait_target)
{
    bare_board(tg);
    test_add_to_stage(tg, 0, hanamaru);
    test_add_to_stage(tg, 1, watanabe);
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, filler);
    test_add_to_discard(tg, wait_target);
    test_give_energy(tg, 20);
}

static void test_discard_two_cost_moves_two_cards(void)
{
    TestGame tg;
    test_game_new(&tg);

    int watanabe, hanamaru, filler, target;
    if (!need(&tg, &watanabe, "PL!S-bp7-005-R＋", 1) ||
        !need(&tg, &hanamaru, "PL!S-bp7-007-R＋", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &target, "PL!-sd1-001-SD", 1)) {
        CHECK(0, "D1: the 渡辺 曜 fixtures resolve");
        return;
    }
    discard_two_setup(&tg, watanabe, hanamaru, filler, target);

    int hand_before = tg.state.p[0].hand.n;
    int wr_before = tg.state.p[0].discard.n;
    CHECK(hand_before >= 2, "D1: setup gave the cost 2 hand cards");

    activate_ability_index(&tg, watanabe, 2);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(test_has_pending_choice(&tg), "D1: the printed cost prompt appears");
    CHECK(ch != NULL && ch->kind == RB_CHOICE_SELECT_CARD &&
              !strcmp(ch->zone, "hand"),
          "D1: 手札を2枚控え室に置く — the cost is a SelectCard over the HAND");
    CHECK_EQ(ch ? ch->count : -1, 2,
             "D1: …2枚 — the printed cost count is exactly 2");

    /* Answer one index per resume (how every other C suite eats a SelectCard). */
    for (int i = 0; i < 4; i++) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        if (!test_has_pending_choice(&tg) || !c || c->kind != RB_CHOICE_SELECT_CARD ||
            strcmp(c->zone, "hand"))
            break;
        rb_resume_with_choice(&tg.state, 0);
    }

    CHECK_EQ(tg.state.p[0].discard.n, wr_before + 2,
             "D1: the two cost cards must LAND in the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2,
             "D1: the two cost cards must actually LEAVE the hand");

    ch = rb_get_pending_choice(&tg.state);
    CHECK(test_has_pending_choice(&tg) && ch != NULL &&
              ch->kind == RB_CHOICE_SELECT_CARD && !strcmp(ch->zone, "stage"),
          "D1: the このメンバーと…を選ぶ stage select follows the PAID cost");
    CHECK_EQ(ch ? ch->count : -1, 2,
             "D1: the stage select targets 2 members (this member + 1 other)");
    CHECK(ch == NULL || ch->n_filtered_indices == 0 ||
              (ch->n_filtered_indices > 1),
          "D1: 渡辺 曜 herself (このメンバー) must stay selectable — G19 excluded her");
}

static void test_discard_two_multi_index_answer_in_one_call(void)
{
    /* The Rust twin pays the two-card cost with ONE select_indices(&[0, 1]).
     * choice.c:2438 collapses a SelectCard answer to selected_indices[0], so
     * this is the direct pin for that defect: keep it strict. */
    TestGame tg;
    test_game_new(&tg);

    int watanabe, hanamaru, filler, target;
    if (!need(&tg, &watanabe, "PL!S-bp7-005-R＋", 1) ||
        !need(&tg, &hanamaru, "PL!S-bp7-007-R＋", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &target, "PL!-sd1-001-SD", 1)) {
        CHECK(0, "D2: the 渡辺 曜 fixtures resolve");
        return;
    }
    discard_two_setup(&tg, watanabe, hanamaru, filler, target);

    int hand_before = tg.state.p[0].hand.n;
    int wr_before = tg.state.p[0].discard.n;

    activate_ability_index(&tg, watanabe, 2);
    CHECK(test_has_pending_choice(&tg), "D2: the two-card cost prompts");
    {
        const int picks[2] = { 0, 1 };
        rb_resume_with_choice_indices(&tg.state, picks, 2);
    }
    CHECK_EQ(tg.state.p[0].discard.n, wr_before + 2,
             "D2: a single select_indices(&[0,1]) answer must pay the WHOLE two-card cost");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2,
             "D2: a single two-index answer must remove both cards from the hand");
}

/* ══════════════════════════════════════════════════════════════════════════
 * E. Reveal cost: the eligible set is FILTERED by cost_values (10 or 20), the
 *    revealed card goes under the activating member, and the effect draws 2.
 *    Rust: reveal_cost_ten_or_twenty_under_member_draw_two_test.rs
 *    (嵐 千砂都 PL!SP-bp7-003-R＋ ab#2)
 * ══════════════════════════════════════════════════════════════════════════ */
static void chika_setup(TestGame *tg, int chika, int filler)
{
    bare_board(tg);
    test_add_to_stage(tg, 1, chika);
    test_give_energy(tg, 20);
    (void)filler;
}

static void test_reveal_cost_cost20_places_under_and_draws(void)
{
    TestGame tg;
    test_game_new(&tg);

    int chika, cost20, cost20b, low;
    if (!need(&tg, &chika, "PL!SP-bp7-003-R＋", 1) ||
        !need(&tg, &cost20, "PL!SP-pb2-005-R", 1) ||
        !need(&tg, &cost20b, "PL!SP-pb2-005-R", 1) ||
        !need(&tg, &low, FILLER_M, 1)) {
        CHECK(0, "E1: the 嵐 千砂都 fixtures resolve");
        return;
    }
    chika_setup(&tg, chika, low);
    test_add_to_hand(&tg, cost20);
    test_add_to_hand(&tg, cost20b);
    test_add_to_hand(&tg, low);
    test_add_to_deck_pl(&tg, 0, low);
    test_add_to_deck_pl(&tg, 0, low);

    int hand_before = tg.state.p[0].hand.n;
    activate_ability_index(&tg, chika, 2);
    drain_accept(&tg);

    CHECK_EQ(under_count(&tg, 1), 1, "E1: exactly one member is placed under 嵐 千砂都");
    CHECK_EQ(tg.state.p[0].under_cards[1].cards[0], cost20,
             "E1: the revealed cost-20 member is the one placed under");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 2 - 1,
             "E1: reveal 1 from hand, draw 2");
}

static void test_reveal_cost_low_cost_not_selectable(void)
{
    TestGame tg;
    test_game_new(&tg);

    int chika, low;
    if (!need(&tg, &chika, "PL!SP-bp7-003-R＋", 1) ||
        !need(&tg, &low, FILLER_M, 1)) { CHECK(0, "E2 fixtures"); return; }
    chika_setup(&tg, chika, low);
    test_add_to_hand(&tg, low);
    test_add_to_deck_pl(&tg, 0, low);
    test_add_to_deck_pl(&tg, 0, low);

    int hand_before = tg.state.p[0].hand.n;
    int deck_before = tg.state.p[0].deck.n;

    activate_ability_index(&tg, chika, 2);
    drain_accept(&tg);

    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "E2: no valid reveal → no draw");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "E2: no valid reveal → hand unchanged");
    CHECK_EQ(under_count(&tg, 1), 0, "E2: nothing is placed under 嵐 千砂都");
}

static void test_reveal_cost_cost10_placed_under(void)
{
    TestGame tg;
    test_game_new(&tg);

    int chika, cost10, low;
    if (!need(&tg, &chika, "PL!SP-bp7-003-R＋", 1) ||
        !need(&tg, &cost10, "PL!N-bp1-003-R＋", 1) ||
        !need(&tg, &low, FILLER_M, 1)) { CHECK(0, "E3 fixtures"); return; }
    chika_setup(&tg, chika, low);
    test_add_to_hand(&tg, cost10);
    test_add_to_deck_pl(&tg, 0, low);
    test_add_to_deck_pl(&tg, 0, low);

    activate_ability_index(&tg, chika, 2);
    drain_accept(&tg);

    CHECK_EQ(under_count(&tg, 1), 1, "E3: a cost-10 member is placed under 嵐 千砂都");
    CHECK_EQ(tg.state.p[0].under_cards[1].cards[0], cost10,
             "E3: the cost-10 member is the card placed under");
}

/* ══════════════════════════════════════════════════════════════════════════
 * F. Optional SelectCard recovery from the waitroom: accept, decline, and the
 *    no-candidate case.
 *    Rust: optional_member_topdeck_blade_pl_s_bp3_021_l_test.rs
 *    (想いよひとつになれ PL!S-bp3-021-L — ライブ開始時: 自分の控え室から
 *     『虹ヶ咲』のメンバー1人を選んでGrab.hand: そうした場合、選んだカードを
 *     自分のデッキの一番上に置き、ライブ終了時まで、ステージのメンバー1人に
 *     ブレード×1を得る。)
 * ══════════════════════════════════════════════════════════════════════════ */
static void bp3_021_setup(TestGame *tg, int live, int mate, int filler, int wait_member)
{
    bare_board(tg);
    fill_zone(&tg->state.p[0].deck, filler, 10);
    fill_zone(&tg->state.p[1].deck, filler, 10);
    test_add_to_live(tg, live);
    test_add_to_stage(tg, 1, mate);
    if (wait_member >= 0) test_add_to_discard(tg, wait_member);
}

static void test_optional_recovery_accepted_reaches_deck_top(void)
{
    TestGame tg;
    test_game_new(&tg);

    int live, mate, filler, wait_member;
    if (!need(&tg, &live, "PL!S-bp3-021-L", 1) ||
        !need(&tg, &mate, "PL!S-sd1-001-SD", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &wait_member, "PL!N-bp3-006-R", 1)) { CHECK(0, "F1 fixtures"); return; }
    bp3_021_setup(&tg, live, mate, filler, wait_member);

    fire_live_start(&tg);
    CHECK(test_has_pending_choice(&tg), "F1: the waitroom member is offered");
    CHECK(!strcmp(pending_zone(&tg), "discard") || !strcmp(pending_zone(&tg), "waitroom"),
          "F1: the recovery select is sourced from the waitroom");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL && ch->allow_skip, "F1: the recovery is optional — skip must be offered");
    rb_resume_with_choice(&tg.state, 0);
    drain_accept(&tg);

    CHECK_EQ(tg.state.p[0].deck.cards[0], wait_member,
             "F1: the recovered member is MOVED to the deck TOP");
    CHECK(!bag_has(&tg.state.p[0].discard, wait_member),
          "F1: the recovered member is no longer in the waitroom");
    CHECK_EQ(test_get_blade_modifier(&tg, mate), 1,
             "F1: そうした場合 — the staged member gains 1 blade");
}

static void test_optional_recovery_declined_preserves_waitroom(void)
{
    TestGame tg;
    test_game_new(&tg);

    int live, mate, filler, wait_member;
    if (!need(&tg, &live, "PL!S-bp3-021-L", 1) ||
        !need(&tg, &mate, "PL!S-sd1-001-SD", 1) ||
        !need(&tg, &filler, FILLER_M, 1) ||
        !need(&tg, &wait_member, "PL!N-bp3-006-R", 1)) { CHECK(0, "F2 fixtures"); return; }
    bp3_021_setup(&tg, live, mate, filler, wait_member);

    fire_live_start(&tg);
    CHECK(test_has_pending_choice(&tg),
          "F2: the optional recovery prompt is expected (a waitroom member is present)");
    CHECK(!strcmp(test_pending_choice_type(&tg), "SelectCard"),
          "F2: the optional recovery prompt is a SelectCard");
    rb_resume_with_choice(&tg.state, -1);          /* select_indices(&[]) */
    drain_accept(&tg);

    CHECK(test_zone_has_id(&tg, 0, "discard", wait_member),
          "F2: declined — the member stays in the waitroom");
    CHECK(tg.state.p[0].deck.n == 0 || tg.state.p[0].deck.cards[0] != wait_member,
          "F2: declined — the member is not placed on the deck");
    CHECK_EQ(test_get_blade_modifier(&tg, mate), 0, "F2: declined — no blade is granted");
}

static void test_optional_recovery_no_candidate_no_prompt(void)
{
    TestGame tg;
    test_game_new(&tg);

    int live, mate, filler;
    if (!need(&tg, &live, "PL!S-bp3-021-L", 1) ||
        !need(&tg, &mate, "PL!S-sd1-001-SD", 1) ||
        !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "F3 fixtures"); return; }
    bp3_021_setup(&tg, live, mate, filler, -1);

    fire_live_start(&tg);
    drain_accept(&tg);

    CHECK(!test_has_pending_choice(&tg), "F3: no member in the waitroom → no prompt");
    CHECK_EQ(test_get_blade_modifier(&tg, mate), 0, "F3: no prompt → no blade");
}

/* ══════════════════════════════════════════════════════════════════════════
 * G. Optional energy placement: accepting places the energy WAITED and makes
 *    the opponent draw; declining does neither.
 *    Rust: optional_wait_energy_opponent_draw_pl_sp_bp5_027_l_test.rs
 *    (HOT PASSION!! PL!SP-bp5-027-L)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_optional_energy_placement_accept_and_decline(void)
{
    int live_no = rb_find_card_by_no("PL!SP-bp5-027-L");
    CHECK(live_no >= 0 && rb_card_no_eq(live_no, "PL!SP-bp5-027-L"),
          "G: HOT PASSION!! resolves to the printed card");
    if (live_no < 0) return;

    /* accept */
    {
        TestGame tg; test_game_new(&tg);
        int live, filler, energy_card;
        if (!need(&tg, &live, "PL!SP-bp5-027-L", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &energy_card, "LL-E-001-SD", 1)) { CHECK(0, "G1 fixtures"); return; }
        bare_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 10);
        fill_zone(&tg.state.p[1].deck, filler, 10);
        test_add_to_live(&tg, live);
        test_add_to_energy_deck(&tg, 0, energy_card);
        test_add_to_energy_deck(&tg, 0, energy_card);
        int p2_hand_before = tg.state.p[1].hand.n;

        fire_live_success(&tg);
        CHECK(test_has_pending_choice(&tg), "G1: the optional energy placement prompts");
        rb_resume_with_choice(&tg.state, 1);   /* accept */
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].energy.n, 1, "G1: one energy card is placed into the energy zone");
        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 0,
                 "G1: the placed energy arrives WAITED");
        CHECK_EQ(tg.state.p[1].hand.n, p2_hand_before + 1,
                 "G1: placement accepted → the opponent draws 1");
    }

    /* decline */
    {
        TestGame tg; test_game_new(&tg);
        int live, filler, energy_card;
        if (!need(&tg, &live, "PL!SP-bp5-027-L", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &energy_card, "LL-E-001-SD", 1)) { CHECK(0, "G2 fixtures"); return; }
        bare_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 10);
        fill_zone(&tg.state.p[1].deck, filler, 10);
        test_add_to_live(&tg, live);
        test_add_to_energy_deck(&tg, 0, energy_card);
        test_add_to_energy_deck(&tg, 0, energy_card);
        int p2_hand_before = tg.state.p[1].hand.n;

        fire_live_success(&tg);
        CHECK(test_has_pending_choice(&tg), "G2: the optional energy placement prompts");
        rb_resume_with_choice(&tg.state, -1);   /* select_indices(&[]) */
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].energy.n, 0, "G2: declined → no energy placed");
        CHECK_EQ(tg.state.p[1].hand.n, p2_hand_before,
                 "G2: declined → the opponent draws nothing");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * H. Optional member cost gate on a ライブ成功時: accept with ONE candidate
 *    and decline with TWO candidates.
 *    Rust: optional_member_cost_score_and_live_recovery_test.rs
 *    (Wonderful Rush PL!-bp6-021-L)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_optional_member_cost_accept_and_decline(void)
{
    /* accept, single candidate */
    {
        TestGame tg; test_game_new(&tg);
        int live, mus_member, mus_live, filler;
        if (!need(&tg, &live, "PL!-bp6-021-L", 1) ||
            !need(&tg, &mus_member, FILLER_M, 1) ||
            !need(&tg, &mus_live, MUS_LIVE, 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "H1 fixtures"); return; }
        bare_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 10);
        fill_zone(&tg.state.p[1].deck, filler, 10);
        test_add_to_live(&tg, live);
        test_add_to_stage(&tg, 0, mus_member);
        test_add_to_discard(&tg, mus_live);

        fire_live_success(&tg);
        CHECK(test_has_pending_choice(&tg),
              "H1: the optional μ's-member cost gate is offered even with one candidate");
        CHECK(!strcmp(test_pending_choice_type(&tg), "SelectTarget"),
              "H1: the optional cost gate is a SelectTarget");
        rb_resume_with_choice(&tg.state, 1);   /* accept */
        drain_accept(&tg);

        CHECK_EQ(test_get_score_modifier(&tg, live), 1, "H1: cost accepted → score +1");
        CHECK(test_hand_has(&tg, mus_live), "H1: cost accepted → the μ's live is retrieved to hand");
        CHECK(!test_zone_has_id(&tg, 0, "discard", mus_live),
              "H1: the retrieved live card left the waitroom");
        CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT,
                 "H1: cost accepted → the sacrificed μ's member left the stage");
        CHECK(test_zone_has_id(&tg, 0, "discard", mus_member),
              "H1: cost accepted → the sacrificed member went to the waitroom");
    }

    /* decline, two candidates */
    {
        TestGame tg; test_game_new(&tg);
        int live, m1, m2, mus_live, filler;
        if (!need(&tg, &live, "PL!-bp6-021-L", 1) ||
            !need(&tg, &m1, FILLER_M, 1) ||
            !need(&tg, &m2, FILLER_M, 1) ||
            !need(&tg, &mus_live, MUS_LIVE, 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "H2 fixtures"); return; }
        bare_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 10);
        fill_zone(&tg.state.p[1].deck, filler, 10);
        test_add_to_live(&tg, live);
        test_add_to_stage(&tg, 0, m1);
        test_add_to_stage(&tg, 2, m2);
        test_add_to_discard(&tg, mus_live);

        fire_live_success(&tg);
        CHECK(test_has_pending_choice(&tg), "H2: two candidates → a prompt is expected");
        rb_resume_with_choice(&tg.state, 0);   /* decline (option 0) */
        drain_accept(&tg);

        CHECK_EQ(test_get_score_modifier(&tg, live), 0, "H2: cost declined → no score");
        CHECK(!test_hand_has(&tg, mus_live), "H2: cost declined → no retrieval");
        CHECK_EQ(tg.state.p[0].stage[0], m1, "H2: cost declined → member 1 stays on stage");
        CHECK_EQ(tg.state.p[0].stage[2], m2, "H2: cost declined → member 2 stays on stage");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * I. An opponent-forced discard: complying gives no blade, declining gives +4.
 *    Rust: reveal_live_opponent_discard_or_four_blades_test.rs
 *    (津島善子 PL!S-pb1-006-R)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_opponent_forced_discard_comply_and_decline(void)
{
    /* comply */
    {
        TestGame tg; test_game_new(&tg);
        int yohane, live_hand, filler;
        if (!need(&tg, &yohane, "PL!S-pb1-006-R", 1) ||
            !need(&tg, &live_hand, "PL!-sd1-019-SD", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "I1 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, yohane);
        test_add_to_hand(&tg, live_hand);
        test_add_to_deck_pl(&tg, 1, filler);
        test_add_to_deck_pl(&tg, 1, filler);
        test_give_energy(&tg, 15);

        activate_ability_index(&tg, yohane, 0);
        CHECK(test_has_pending_choice(&tg), "I1: the opponent is prompted to discard");
        rb_resume_with_choice(&tg.state, 0);
        drain_accept(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, yohane), 0,
                 "I1: opponent discarded → そうでない場合は no blade");
    }

    /* decline */
    {
        TestGame tg; test_game_new(&tg);
        int yohane, live_hand, filler;
        if (!need(&tg, &yohane, "PL!S-pb1-006-R", 1) ||
            !need(&tg, &live_hand, "PL!-sd1-019-SD", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "I2 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, yohane);
        test_add_to_hand(&tg, live_hand);
        test_add_to_deck_pl(&tg, 1, filler);
        test_give_energy(&tg, 15);

        activate_ability_index(&tg, yohane, 0);
        CHECK(test_has_pending_choice(&tg), "I2: the opponent is prompted");
        rb_resume_with_choice(&tg.state, -1);   /* select_indices(&[]) = decline */
        drain_accept(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, yohane), 4,
                 "I2: opponent declined → ブレード×4");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * J. Optional hand-discard 登場 cost: paid → mill 3 + recover the スリーズブeke
 *    live; declined → no mill, live stays in the waitroom.
 *    Rust: discard_mill_live_recovery_pl_hs_pb1_004_r_test.rs
 *    (百生吟子 PL!HS-pb1-004-R)
 * ══════════════════════════════════════════════════════════════════════════ */
static void meko_setup(TestGame *tg, int meko, int filler, int sblive, int filler2)
{
    bare_board(tg);
    fill_zone(&tg->state.p[0].deck, filler, 10);
    fill_zone(&tg->state.p[1].deck, filler2, 10);
    test_add_to_hand(tg, meko);
    test_give_energy(tg, 5);
    test_add_to_hand(tg, filler2);          /* the optional cost card */
    test_add_to_discard(tg, sblive);
}

static void test_optional_debut_cost_mill_paid_and_declined(void)
{
    /* paid */
    {
        TestGame tg; test_game_new(&tg);
        int meko, filler, filler2, sblive;
        if (!need(&tg, &meko, "PL!HS-pb1-004-R", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &sblive, "PL!HS-PR-010-PR", 1)) { CHECK(0, "J1 fixtures"); return; }
        filler2 = test_new_id(&tg, FILLER_M);
        meko_setup(&tg, meko, filler, sblive, filler2);

        CHECK(test_play_to_stage(&tg, meko, 1), "J1: 百生吟子 deploys to the centre stage");
        int deck_before = tg.state.p[0].deck.n;
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3, "J1: the top 3 deck cards are milled");
        CHECK(test_hand_has(&tg, sblive), "J1: the 『スリーズブーケ』 live is recovered to hand");
    }

    /* declined */
    {
        TestGame tg; test_game_new(&tg);
        int meko, filler, filler2, sblive;
        if (!need(&tg, &meko, "PL!HS-pb1-004-R", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &sblive, "PL!HS-PR-010-PR", 1)) { CHECK(0, "J2 fixtures"); return; }
        filler2 = test_new_id(&tg, FILLER_M);
        meko_setup(&tg, meko, filler, sblive, filler2);

        CHECK(test_play_to_stage(&tg, meko, 1), "J2: 百生吟子 deploys to the centre stage");
        CHECK(test_has_pending_choice(&tg), "J2: the optional hand-discard cost is offered");
        CHECK(!strcmp(test_pending_choice_type(&tg), "SelectCard"),
              "J2: the optional cost prompt is a skippable SelectCard");
        drain_decline(&tg);

        CHECK_EQ(tg.state.p[0].deck.n, 10, "J2: declined → no mill");
        CHECK(test_zone_has_id(&tg, 0, "discard", sblive),
              "J2: declined → the live card stays in the waitroom");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * K. 「自分のステージにいるほかの『虹ヶ咲』のメンバー1人をウェイトにする」 as a
 *    mandatory cost: unpayable → the whole activation is refused (Q56), an
 *    opponent's member is not a candidate (Q163), and a legal candidate is
 *    auto-waited (exclude_self keeps the activator out).
 *    Rust: activate_wait_other_group_member_then_draw_one_test.rs
 *    (Helen PL!N-bp3-008-R＋)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_wait_other_group_cost(void)
{
    /* unpayable: only 虹ヶ咲 on stage is Helen herself (exclude_self) */
    {
        TestGame tg; test_game_new(&tg);
        int emma, filler;
        if (!need(&tg, &emma, "PL!N-bp3-008-R＋", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "K1 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, filler);
        test_add_to_stage(&tg, 1, emma);
        test_add_to_stage(&tg, 2, test_new_id(&tg, FILLER_M));
        test_add_to_hand(&tg, filler);
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);

        activate_ability_index(&tg, emma, 0);
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].hand.n, 1,
                 "K1: no draw happens because the mandatory wait cost has no candidate");
        CHECK_EQ(tg.state.p[0].deck.n, 30, "K1: the deck is untouched by a refused cost");
    }

    /* Q163: the opponent's 虹ヶ咲 member is not a cost target */
    {
        TestGame tg; test_game_new(&tg);
        int emma, opp_niji, filler;
        if (!need(&tg, &emma, "PL!N-bp3-008-R＋", 1) ||
            !need(&tg, &opp_niji, "PL!N-sd1-001-SD", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "K2 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, emma);
        test_add_to_stage(&tg, 1, filler);
        test_set_opp_stage(&tg, 0, opp_niji);
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);

        activate_ability_index(&tg, emma, 0);
        drain_accept(&tg);

        CHECK(!is_waited(&tg, opp_niji),
              "K2: Q163 — an opponent's 虹ヶ咲 member must remain unchanged");
        CHECK_EQ(tg.state.p[0].hand.n, 0,
                 "K2: Q163 — no target prompt, so nothing is drawn");
    }

    /* a legal 虹ヶ咲 candidate is auto-waited and the draw follows */
    {
        TestGame tg; test_game_new(&tg);
        int emma, niji, filler;
        if (!need(&tg, &emma, "PL!N-bp3-008-R＋", 1) ||
            !need(&tg, &niji, "PL!N-sd1-001-SD", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "K3 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, filler);
        test_add_to_stage(&tg, 1, emma);
        test_add_to_stage(&tg, 2, niji);
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);

        activate_ability_index(&tg, emma, 0);
        drain_accept(&tg);

        CHECK(is_waited(&tg, niji), "K3: the only 虹ヶ咲 candidate is auto-waited as the cost");
        CHECK(!is_waited(&tg, emma), "K3: exclude_self — Helen herself is NOT waited");
        CHECK(!is_waited(&tg, filler), "K3: a non-虹ヶ咲 filler is NOT waited");
        CHECK(tg.state.p[0].hand.n > 0, "K3: the draw follows the paid cost");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * L. Optional EE 登場 cost: pay → the waitroom live card goes face-up and the
 *    live-card-set limit drops; skip → nothing happens; two candidates → exactly
 *    one is placed.
 *    Rust: hs_bp2_018_live_card_zone_test.rs (日下あすか PL!HS-bp2-018-N)
 * ══════════════════════════════════════════════════════════════════════════ */
static void asuka_setup(TestGame *tg, int asuka, int filler, int live, int n_live)
{
    bare_board(tg);
    test_add_to_hand(tg, asuka);
    for (int i = 0; i < n_live; i++) test_add_to_discard(tg, live);
    fill_zone(&tg->state.p[0].deck, filler, 10);
    fill_zone(&tg->state.p[1].deck, filler, 10);
    test_give_energy(tg, 10);
}

static void test_optional_debut_ee_pay_skip_and_multi(void)
{
    /* pay */
    {
        TestGame tg; test_game_new(&tg);
        int asuka, filler, live;
        if (!need(&tg, &asuka, "PL!HS-bp2-018-N", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &live, "PL!-sd1-019-SD", 1)) { CHECK(0, "L1 fixtures"); return; }
        asuka_setup(&tg, asuka, filler, live, 1);

        CHECK(test_play_to_stage(&tg, asuka, 1), "L1: 日下あすか deploys to the centre stage");
        CHECK(test_has_pending_choice(&tg), "L1: the optional EE cost is offered");
        rb_resume_with_choice(&tg.state, 1);       /* pay */
        drain_accept(&tg);

        CHECK(test_zone_has_id(&tg, 0, "live", live),
              "L1: after paying, the waitroom live card is MOVED to the live zone");
        CHECK(!test_zone_has_id(&tg, 0, "discard", live),
              "L1: the live card is removed from the waitroom");
        CHECK_EQ(tg.state.live_set_limit_reduction[0], 1,
                 "L1: the live-card-set limit is reduced by 1 after the ability resolves");
    }

    /* skip */
    {
        TestGame tg; test_game_new(&tg);
        int asuka, filler, live;
        if (!need(&tg, &asuka, "PL!HS-bp2-018-N", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &live, "PL!-sd1-019-SD", 1)) { CHECK(0, "L2 fixtures"); return; }
        asuka_setup(&tg, asuka, filler, live, 1);

        CHECK(test_play_to_stage(&tg, asuka, 1), "L2: 日下あすか deploys to the centre stage");
        CHECK(test_has_pending_choice(&tg), "L2: the optional EE cost is offered");
        rb_resume_with_choice(&tg.state, 0);       /* skip */
        drain_accept(&tg);

        CHECK(test_zone_has_id(&tg, 0, "discard", live),
              "L2: skipping the cost leaves the live card in the waitroom");
        CHECK(!test_zone_has_id(&tg, 0, "live", live),
              "L2: skipping the cost places nothing in the live zone");
        CHECK_EQ(tg.state.live_set_limit_reduction[0], 0,
                 "L2: skipping the cost must not reduce the live-card-set limit");
    }

    /* two candidates → exactly one is placed */
    {
        TestGame tg; test_game_new(&tg);
        int asuka, filler, live;
        if (!need(&tg, &asuka, "PL!HS-bp2-018-N", 1) ||
            !need(&tg, &filler, FILLER_M, 1) ||
            !need(&tg, &live, "PL!-sd1-019-SD", 1)) { CHECK(0, "L3 fixtures"); return; }
        asuka_setup(&tg, asuka, filler, live, 2);

        CHECK(test_play_to_stage(&tg, asuka, 1), "L3: 日下あすか deploys to the centre stage");
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].live.n, 1,
                 "L3: exactly 1 live card is in the live zone out of 2 candidates");
        CHECK_EQ(tg.state.p[0].discard.n, 1, "L3: the unchosen live card stays in the waitroom");
        CHECK_EQ(tg.state.live_set_limit_reduction[0], 1,
                 "L3: the limit reduction is exactly 1");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * M. An optional TWO-card cost, then a heart-colour pick, then a stage target;
 *    and the same ability with the cost declined.
 *    Rust: pl_hs_sd1_008_test.rs (桂城 泉 PL!HS-sd1-008-SD)
 * ══════════════════════════════════════════════════════════════════════════ */
static void izumi_setup(TestGame *tg, int izumi, int kozue, int tsuzuri, int filler,
                        int n_cost_cards, int with_live)
{
    bare_board(tg);
    test_add_to_stage(tg, 0, kozue);
    test_add_to_stage(tg, 1, izumi);
    test_add_to_stage(tg, 2, tsuzuri);
    for (int i = 0; i < n_cost_cards; i++)
        test_add_to_hand(tg, i % 2 ? test_new_id(tg, "PL!HS-PR-004-PR")
                                   : test_new_id(tg, "PL!HS-bp1-012-PR"));
    if (with_live) {
        int live = cid(tg, MUS_LIVE);
        if (live >= 0) test_add_to_hand(tg, live);
    }
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    (void)tg;
}

static void izumi_decks(TestGame *tg, int filler)
{
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 1, filler);
    test_add_to_deck_pl(tg, 1, test_new_id(tg, FILLER_M));
    test_give_energy(tg, 10);
}

static void test_optional_two_card_cost_pay_and_skip(void)
{
    int izumi_no = rb_find_card_by_no("PL!HS-sd1-008-SD");
    CHECK(izumi_no >= 0 && rb_card_no_eq(izumi_no, "PL!HS-sd1-008-SD"),
          "M: 桂城 泉 resolves to the printed card");
    if (izumi_no < 0) return;

    /* paid: 2 hand cards, then heart colour, then a stage target */
    {
        TestGame tg; test_game_new(&tg);
        int izumi, kozue, tsuzuri, filler;
        if (!need(&tg, &izumi, "PL!HS-sd1-008-SD", 1) ||
            !need(&tg, &kozue, "PL!HS-bp1-012-PR", 1) ||
            !need(&tg, &tsuzuri, "PL!HS-PR-004-PR", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "M1 fixtures"); return; }
        izumi_setup(&tg, izumi, kozue, tsuzuri, filler, 2, 1);
        izumi_decks(&tg, filler);

        int live = cid(&tg, MUS_LIVE);
        if (live >= 0) test_set_live_card(&tg, 0, live);
        /* ab#1 is 桂城 泉's ライブ開始時 (the optional 2-card 蓮ノ空 cost);
         * ab#0 is her 常時 clause, which carries no cost at all. */
        activate_ability_index(&tg, izumi, 1);

        CHECK(test_has_pending_choice(&tg), "M1: the optional cost prompts");
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        CHECK(ch != NULL && ch->count == 2, "M1: the printed cost is 2 hand cards");
        CHECK(ch != NULL && ch->allow_skip, "M1: the two-card cost is optional");

        /* The Rust twin pays it with ONE select_indices(&[0, 1]). */
        {
            const int picks[2] = { 0, 1 };
            rb_resume_with_choice_indices(&tg.state, picks, 2);
        }
        drain_accept(&tg);

        CHECK_EQ(test_get_heart_modifier(&tg, kozue, RB_HEART_PINK), 2,
                 "M1: the chosen ally gains +2 heart01");
        CHECK_EQ(test_get_heart_modifier(&tg, izumi, RB_HEART_PINK), 0,
                 "M1: 桂城 泉 (self) gains no heart01");
        CHECK_EQ(test_get_heart_modifier(&tg, tsuzuri, RB_HEART_PINK), 0,
                 "M1: the un-targeted ally gains no heart01");
    }

    /* declined */
    {
        TestGame tg; test_game_new(&tg);
        int izumi, kozue, tsuzuri, filler;
        if (!need(&tg, &izumi, "PL!HS-sd1-008-SD", 1) ||
            !need(&tg, &kozue, "PL!HS-bp1-012-PR", 1) ||
            !need(&tg, &tsuzuri, "PL!HS-PR-004-PR", 1) ||
            !need(&tg, &filler, FILLER_M, 1)) { CHECK(0, "M2 fixtures"); return; }
        izumi_setup(&tg, izumi, kozue, tsuzuri, filler, 0, 1);
        izumi_decks(&tg, filler);

        int live = cid(&tg, MUS_LIVE);
        if (live >= 0) test_set_live_card(&tg, 0, live);
        activate_ability_index(&tg, izumi, 1);
        drain_decline(&tg);

        int total = 0;
        const int colors[4] = { RB_HEART_PINK, RB_HEART_GREEN, RB_HEART_BLUE, RB_HEART_PURPLE };
        for (int i = 0; i < 4; i++)
            total += rb_mods_get_heart(&tg.state.mods, kozue, colors[i]) +
                     rb_mods_get_heart(&tg.state.mods, tsuzuri, colors[i]);
        CHECK_EQ(total, 0, "M2: declining the optional cost grants no hearts at all");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * N. A compound two-clause cost (wait THIS member + discard 1 from hand) is
 *    paid in full BEFORE the sacrifice/summon effect runs; the activation is
 *    refused when the position gate or the hand cost cannot be met.
 *    Rust: center_wait_discard_other_group_cost_plus_two_same_area_deploy_test.rs
 *    (津島善子 PL!S-bp3-006-R＋)
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_center_compound_cost_paid_before_effect(void)
{
    /* basic success */
    {
        TestGame tg; test_game_new(&tg);
        int yoshiko, chika, riko, hand_card, dia;
        if (!need(&tg, &yoshiko, "PL!S-bp3-006-R＋", 1) ||
            !need(&tg, &chika, "PL!S-bp2-001-R", 1) ||
            !need(&tg, &riko, "PL!S-bp2-002-R", 1) ||
            !need(&tg, &hand_card, FILLER_M, 1) ||
            !need(&tg, &dia, "PL!S-bp2-004-R", 1)) { CHECK(0, "N1 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 1, yoshiko);
        test_add_to_stage(&tg, 0, chika);
        test_add_to_stage(&tg, 2, riko);
        test_add_to_hand(&tg, hand_card);
        test_add_to_discard(&tg, dia);
        test_give_energy(&tg, 5);
        int stage_before = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (tg.state.p[0].stage[i] != RB_EMPTY_SLOT) stage_before++;

        activate_ability_index(&tg, yoshiko, 0);
        drain_accept(&tg);

        CHECK(is_waited(&tg, yoshiko),
              "N1: cost clause 1 — 津島善子 must be put to WAIT");
        CHECK_EQ(tg.state.p[0].stage[1], yoshiko,
                 "N1: 津島善子 stays on stage, in the wait state");
        CHECK(!test_hand_has(&tg, hand_card),
              "N1: cost clause 2 — the hand card left the hand");
        CHECK(test_zone_has_id(&tg, 0, "discard", hand_card),
              "N1: cost clause 2 — the hand card is in the waitroom as payment");
        CHECK(test_zone_has_id(&tg, 0, "discard", chika),
              "N1: the effect sacrificed the selected Aqours member");
        CHECK_EQ(tg.state.p[0].stage[0], dia,
                 "N1: ダイヤ (cost 11 = 9+2) is summoned to the vacated area");
        CHECK(!test_zone_has_id(&tg, 0, "discard", dia),
              "N1: the summoned member is no longer in the waitroom");
        int stage_after = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (tg.state.p[0].stage[i] != RB_EMPTY_SLOT) stage_after++;
        CHECK_EQ(stage_after, stage_before, "N1: the stage count is unchanged (1 replaced)");
    }

    /* not in the centre: the position gate blocks the activation entirely */
    {
        TestGame tg; test_game_new(&tg);
        int yoshiko, chika;
        if (!need(&tg, &yoshiko, "PL!S-bp3-006-R＋", 1) ||
            !need(&tg, &chika, "PL!-sd1-001-SD", 1)) { CHECK(0, "N2 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 0, yoshiko);
        test_add_to_stage(&tg, 1, chika);
        test_give_energy(&tg, 5);

        activate_ability_index(&tg, yoshiko, 0);
        drain_accept(&tg);

        CHECK(!test_has_pending_choice(&tg),
              "N2: activating off-centre opens no prompt");
        CHECK_EQ(tg.state.p[0].stage[0], yoshiko, "N2: the stage is unchanged off-centre");
    }

    /* empty hand: the mandatory discard clause is unpayable (Q56) */
    {
        TestGame tg; test_game_new(&tg);
        int yoshiko, chika;
        if (!need(&tg, &yoshiko, "PL!S-bp3-006-R＋", 1) ||
            !need(&tg, &chika, "PL!S-bp2-001-R", 1)) { CHECK(0, "N3 fixtures"); return; }
        bare_board(&tg);
        test_add_to_stage(&tg, 1, yoshiko);
        test_add_to_stage(&tg, 0, chika);
        test_give_energy(&tg, 5);

        activate_ability_index(&tg, yoshiko, 0);
        drain_accept(&tg);

        CHECK(!test_has_pending_choice(&tg),
              "N3: an unpayable hand cost is refused BEFORE any prompt is opened");
        CHECK_EQ(tg.state.p[0].hand.n, 0, "N3: the hand is still empty");
        CHECK_EQ(tg.state.p[0].discard.n, 0, "N3: nothing entered the waitroom");
        CHECK(!is_waited(&tg, yoshiko),
              "N3: no cost clause runs at all when the compound cost is refused");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * O. Cost discard + 「好きなハートの色を1つ指定する」 + reveal 5: the cost must
 *    move exactly one hand card, must not charge energy, and the all-match
 *    condition must grant blade +3 while a single mismatch grants nothing.
 *    Rust: discard_cost_reveal_five_conditional_mus_search_pl_bp6_006_test.rs
 *    (西木野真姫 PL!-bp6-006-R＋)
 * ══════════════════════════════════════════════════════════════════════════ */
static void maki_setup(TestGame *tg, int maki, int filler, const int *deck_top, int n_top)
{
    bare_board(tg);
    test_add_to_stage(tg, 1, maki);
    test_add_to_hand(tg, filler);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < n_top; i++) test_add_to_deck_pl(tg, 0, deck_top[i]);
    for (int i = 0; i < 30 - n_top; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, 1, filler);
    test_give_energy(tg, 10);
    tg->state.turn = 1;
}

static void test_maki_cost_discard_and_all_match_blade(void)
{
    /* Each case gets its OWN TestGame. bare_board() clears the zones but NOT
     * tg->state.mods, and ブレード granted by ① is a live-end modifier that
     * would still be counted in case ② — reusing one TestGame made the blade
     * read 3+3 and then 3+3+3, which looks exactly like an engine bug. */
    int maki, filler, ruby, mus_maki;
    {
        TestGame ids;
        test_game_new(&ids);
        if (!need(&ids, &maki, "PL!-bp6-006-R＋", 1) ||
            !need(&ids, &filler, FILLER_M, 1) ||
            !need(&ids, &ruby, "PL!S-bp2-009-R", 1) ||
            !need(&ids, &mus_maki, "PL!-bp5-015-N", 1)) { CHECK(0, "O fixtures"); return; }
    }

    /* the printed cost is a hand discard: one card out, no energy spent */
    {
        TestGame tg; test_game_new(&tg);
        int top[5];
        for (int i = 0; i < 5; i++) top[i] = filler;
        maki_setup(&tg, maki, filler, top, 5);
        int hand_before = tg.state.p[0].hand.n;
        int energy_before = rb_energy_active_count(&tg.state.p[0]);

        activate_ability_index(&tg, maki, 0);
        drain_accept(&tg);

        CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
                 "O1: the printed cost discards exactly 1 hand card");
        CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), energy_before,
                 "O1: the discard cost charges no energy");
    }

    /* all five revealed cards carry heart02 → blade +3 and a μ's pick */
    {
        TestGame tg; test_game_new(&tg);
        int top[5] = { ruby, ruby, ruby, ruby, mus_maki };
        maki_setup(&tg, maki, filler, top, 5);
        activate_ability_index(&tg, maki, 0);
        drain_accept(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, maki), 3,
                 "O2: all 5 revealed cards match heart02 → ブレード×3");
        CHECK(test_hand_has(&tg, mus_maki),
              "O2: the selected μ's card is MOVED from the revealed set into the hand");
    }

    /* 4 of 5 match → the condition fails and no blade is granted */
    {
        TestGame tg; test_game_new(&tg);
        int top[5] = { ruby, ruby, ruby, ruby, filler };
        maki_setup(&tg, maki, filler, top, 5);
        activate_ability_index(&tg, maki, 0);
        drain_accept(&tg);
        CHECK_EQ(test_get_blade_modifier(&tg, maki), 0,
                 "O3: 4 of 5 matching heart02 is not enough → blade 0");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * P. ENGINE GAP (red on purpose): rb_activate_card's cost+effect bridge drops a
 *    `sequential_cost` cost root entirely.
 *    engine.c:1150-1170 wraps ab.cost + ab.effect into ONE root and hands it to
 *    rb_execute_effect_ex, but executor.c:357-366 makes a `sequential_cost` node
 *    a deliberate NO-OP ("the sub-costs are already paid in order by the
 *    dedicated cost path"). No cost path runs inside that bridge, so a 起動
 *    whose printed cost root is sequential_cost pays NOTHING at all.
 *    Card: 桜坂しずく PL!N-pb1-003-R (Q196) — 起動/2E: このカードを手札から
 *    控え室に置く：カードを1枚引き、…
 *    Expected: 2 energy tapped and the activating card in the waitroom.
 *    Actual:   0 energy tapped, the card stays in hand, only the effect runs.
 *    The behavioural block A above routes around this by paying the root through
 *    rb_pay_cost first; this block keeps the bridge bug itself pinned.
 * ══════════════════════════════════════════════════════════════════════════ */
static void test_gap_sequential_cost_root_dropped_by_bridge(void)
{
    TestGame tg;
    test_game_new(&tg);
    bare_board(&tg);

    int shizuku, filler;
    if (!need(&tg, &shizuku, "PL!N-pb1-003-R", 1) ||
        !need(&tg, &filler, FILLER_M, 1)) {
        CHECK(0, "P: the Q196 fixtures resolve");
        return;
    }
    test_add_to_hand(&tg, shizuku);
    test_add_to_hand(&tg, filler);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    /* Drive the bridge by hand, WITHOUT the sequential_cost pre-pay that
     * activate_ability_index() performs: cost then effect, both through
     * rb_execute_effect_ex, exactly as engine.c:1159 does. */
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)shizuku, 0, &ab) || !ab.cost) {
        CHECK(0, "P: PL!N-pb1-003-R ab#0 decodes with a cost");
        rb_free_ability(&ab);
        return;
    }
    CHECK(ab.cost->action != NULL && !strcmp(ab.cost->action, "sequential_cost"),
          "P: the Q196 cost root really is a sequential_cost (the case the bridge drops)");
    AbilityEffect *act = (AbilityEffect *)calloc(1, sizeof(AbilityEffect));
    if (ab.cost)   act->child[act->n_child++] = ab.cost;
    if (ab.effect) act->child[act->n_child++] = ab.effect;
    tg.state.activation_act = act;
    tg.state.activation_keepalive = ab;
    tg.state.activation_keepalive_valid = 1;
    rb_execute_effect_ex(&tg.state, 0, act, shizuku);
    rb_drain_ability_queue(&tg.state);
    act->n_child = 0;
    tg.state.activation_keepalive.cost = NULL;
    tg.state.activation_keepalive.effect = NULL;
    rb_free_ability(&tg.state.activation_keepalive);
    tg.state.activation_keepalive_valid = 0;
    free(act);
    tg.state.activation_act = NULL;

    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 13,
             "P: the rb_activate_card bridge must charge the 2E leg of a sequential_cost cost");
    CHECK(test_zone_has_id(&tg, 0, "discard", shizuku),
          "P: the bridge must pay the このカードを手札から控え室に置く leg");
}

/* ══════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 2;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    dump_all_fixtures();

    test_q196_mandatory_cost_then_draw();
    test_q196_hand_only_gate_from_stage();

    test_q214_optional_score_energy_pay_and_decline();

    test_success_score_gate_offers_no_cost();
    test_success_score_sequential_discard_two();
    test_pb1_007_cost_scales_with_success_count();
    test_pb1_007_insufficient_hand_and_group_filter();

    test_discard_two_cost_moves_two_cards();
    test_discard_two_multi_index_answer_in_one_call();

    test_reveal_cost_cost20_places_under_and_draws();
    test_reveal_cost_low_cost_not_selectable();
    test_reveal_cost_cost10_placed_under();

    test_optional_recovery_accepted_reaches_deck_top();
    test_optional_recovery_declined_preserves_waitroom();
    test_optional_recovery_no_candidate_no_prompt();

    test_optional_energy_placement_accept_and_decline();
    test_optional_member_cost_accept_and_decline();
    test_opponent_forced_discard_comply_and_decline();
    test_optional_debut_cost_mill_paid_and_declined();
    test_wait_other_group_cost();
    test_optional_debut_ee_pay_skip_and_multi();
    test_optional_two_card_cost_pay_and_skip();
    test_center_compound_cost_paid_before_effect();
    test_maki_cost_discard_and_all_match_blade();
    test_gap_sequential_cost_root_dropped_by_bridge();

    rb_unload();
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL COST-AND-EFFECT PARITY CHECKS PASSED\n");
    return failures == 0 ? 0 : 1;
}

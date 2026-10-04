/* test_parity_deck_order.c
 *
 * Parity coverage for the deck-ORDER prompt:
 *   Rust twin  engine/src/ability/choice.rs   handle_order_selection
 *              engine/src/ability/effects/move/mod.rs  maybe_prompt_deck_order
 *                                                    + the Finalize `needs_order` arm
 *   C port     engine_c_wip/src/ability/choice.c  rb_order_prompt_begin
 *                                                rb_move_handle_order_selection
 *                                                rb_resolver_handle_order_selection
 *
 * WHY THIS FILE EXISTS. 「好きな順番でデッキの上に置く」 is the only rule in the
 * game whose answer is a SEQUENCE OF ANSWERS, not one answer. The C used to
 * parse a single comma-separated "2,0,1", reorder the looked_at pool and clear
 * the choice state — it never PLACED the cards, so they were left on the table
 * and the deck silently lost them. Rust (choice.rs:2984-3055) instead:
 *
 *   - parses ONE index per answer (choice.rs:2994-2998),
 *   - removes that one card from the pool and appends it to `ordered`,
 *   - while MORE THAN ONE card remains, RE-ARMS the same
 *     SelectTarget{target:"order"} over the shrunken pool (choice.rs:3004-3020),
 *   - on the last answer drains the remainder (choice.rs:3021) and inserts
 *     every card at deck position 0 in REVERSE, so the FIRST-picked card ends
 *     up on top (choice.rs:3042-3044).
 *
 * The two producers are the same three statements (set the pool, arm the
 * SelectTarget, park), so choice.c exposes ONE entry point, rb_order_prompt_begin,
 * for both. The producer call site itself is still missing in the C
 * (src/ability/effects/move_missing.c) — see test_corpus_reach below, which pins
 * the two corpus shapes that must reach it, and the BLOCKED-ON note in
 * engine_c_wip/NOZOMI_ROOT_CAUSE.md.
 *
 * sizeof(GameState) is 1 195 656 B, so the TestGame is `static`.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* 笏笏 Deck-order entry points. Both are defined in choice.c but rabuka.h is
 *     owned elsewhere, so they are declared here verbatim from
 *     src/ability/choice.c. This also pins the EXACT contract the missing
 *     move_missing.c producer must call. 笏笏 */

/* Fills the looked_at pool and arms Choice::SelectTarget{target:"order"}.
 * `reverse_pool` is mod.rs:1906-1908 (order_pool.reverse() for a Hand source).
 * Returns 0 when there is no order to ask about. */
extern int  rb_order_prompt_begin(GameState *g, int actor, const int *cards, int n,
                                  int reverse_pool);
/* Answers one step. 0 = order complete (cards placed on the deck), 1 = re-armed,
 * -1 = invalid answer (Rust's Err: the prompt stands). */
extern int  rb_move_handle_order_selection(GameState *g, const char *selected);
extern void rb_resolver_handle_order_selection(GameState *g, const char *selected);

/* ── verdict layer (same shape as tests/test_parity_queue_resume.c) ───────── */

static int failures;
static int assertions;
static const char *g_flat_failed[64];
static int g_flat_failed_n;

static void flat_verdict_failed(const char *fn)
{
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;
    if (g_flat_failed_n >= 64) return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}

#define FLAT_VERDICT() flat_verdict_failed(__func__)

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixtures ────────────────────────────────────────────────────────────── */

static TestGame tg;   /* static: 1 195 656 B */

static void clear_zones(GameState *g, int pl)
{
    RbPlayer *P = &g->p[pl];
    memset(&P->hand, 0, sizeof(P->hand));
    memset(&P->deck, 0, sizeof(P->deck));
    memset(&P->discard, 0, sizeof(P->discard));
    memset(&P->live, 0, sizeof(P->live));
    memset(&P->success, 0, sizeof(P->success));
}

/* Three DISTINCT card ids (test_new_id, not test_id: the latter aliases one
 * shared template, so all three would be the same card and every ordering
 * assertion would be vacuous). */
static int three_cards(int *out)
{
    out[0] = test_new_id(&tg, "PL!-sd1-010-SD");
    out[1] = test_new_id(&tg, "PL!-sd1-002-SD");
    out[2] = test_new_id(&tg, "PL!HS-sd1-008-SD");
    return (out[0] >= 0 && out[1] >= 0 && out[2] >= 0 &&
            out[0] != out[1] && out[1] != out[2] && out[0] != out[2]);
}

static int pool_of(int pl, int *out)
{
    return rb_looked_at_pool(pl, out, RB_MAX_ZONE);
}

/* ── 1. the prompt shape ─────────────────────────────────────────────────── */

/* mod.rs:1915-1931 / :3139-3149. Both Rust producers arm the SAME choice:
 * SelectTarget, target "order", allow_skip false, a "(N cards)" description,
 * and a pool of exactly N cards. The old C body keyed off pending.zone
 * containing "looked_at", which rb_emit_choice(g,...,NULL,...) never sets. */
static void test_prompt_arms_select_target_order(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);

    int started = rb_order_prompt_begin(&tg.state, 0, c, 3, 0);
    CHECK_EQ(started, 1, "a 3-card deck order prompt is issued");

    CHECK_EQ(tg.state.queue.pending.kind, RB_CHOICE_SELECT_TARGET,
             "the deck-order prompt is a SelectTarget (choice.rs:3008)");
    CHECK(tg.state.queue.pending.target[0] &&
          strcmp(tg.state.queue.pending.target, "order") == 0,
          "the prompt target is \"order\" so the SelectTarget dispatcher routes here");
    CHECK_EQ(tg.state.queue.pending.count, 3,
             "the prompt count is the pool size (choice.rs:3005)");
    CHECK_EQ(tg.state.queue.pending.allow_skip, 0,
             "allow_skip is false (choice.rs:3013)");
    CHECK(strstr(tg.state.queue.pending.description, "(3 cards)") != NULL,
          "the description carries the card count (choice.rs:3010)");
    CHECK_EQ(tg.state.queue.resume_look_owner, 0,
             "the look owner is restated after the emit, which clears it");
    CHECK_EQ(tg.state.queue.state, RB_QUEUE_AWAITING_CHOICE,
             "the queue parks in WaitingForChoice");
    CHECK(rb_has_pending_choice(&tg.state), "a pending choice is recorded");

    int pool[RB_MAX_ZONE];
    CHECK_EQ(pool_of(0, pool), 3, "all three cards are in the looked_at pool");
    CHECK_EQ(pool[0], c[0], "pool[0] is the first card offered");
    CHECK_EQ(pool[1], c[1], "pool[1] is the second card offered");
    CHECK_EQ(pool[2], c[2], "pool[2] is the third card offered");
    CHECK_EQ(tg.state.p[0].deck.n, 0,
             "nothing is on the deck yet — the cards wait for the order");
}

/* mod.rs:1906-1908 — a Hand source offers the pool REVERSED. */
static void test_hand_source_reverses_the_pool(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);

    CHECK_EQ(rb_order_prompt_begin(&tg.state, 0, c, 3, 1), 1,
             "the reversed pool still issues a prompt");
    int pool[RB_MAX_ZONE];
    CHECK_EQ(pool_of(0, pool), 3, "the reversed pool keeps all three cards");
    CHECK_EQ(pool[0], c[2], "a Hand source reverses the offered order (mod.rs:1907)");
    CHECK_EQ(pool[2], c[0], "the reversal is total, not a swap of the ends");
}

/* mod.rs:1893 `taken.len() > 1` — one card has no order to choose. */
static void test_single_card_issues_no_prompt(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);

    CHECK_EQ(rb_order_prompt_begin(&tg.state, 0, c, 1, 0), 0,
             "a single card asks for no order (mod.rs:1893)");
    CHECK(!rb_has_pending_choice(&tg.state), "no choice is left pending");
    CHECK_EQ(pool_of(0, (int[RB_MAX_ZONE]){0}), 0, "no pool is opened");
}

/* ── 2. one index per answer, and the re-arm ─────────────────────────────── */

/* choice.rs:2994-3003 — the answer is ONE index, and exactly that card leaves
 * the pool. The old C parsed the whole "2,0,1" in one answer. */
static void test_one_index_per_answer_shrinks_the_pool(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);

    int r = rb_move_handle_order_selection(&tg.state, "1");
    CHECK_EQ(r, 1, "answering index 1 of 3 re-arms (choice.rs:3019 returns Ok)");

    int pool[RB_MAX_ZONE];
    CHECK_EQ(pool_of(0, pool), 2, "the picked card left the pool (remove(idx))");
    CHECK_EQ(pool[0], c[0], "the card BEFORE the pick is still first");
    CHECK_EQ(pool[1], c[2], "the card AFTER the pick shifted down into its place");
    CHECK(strcmp(tg.state.queue.pending.target, "order") == 0,
          "the re-armed prompt is the SAME SelectTarget order choice");
    CHECK(strstr(tg.state.queue.pending.description, "(2 cards)") != NULL,
          "the re-armed description recounts the SHRUNKEN pool (choice.rs:3005)");
    CHECK_EQ(tg.state.queue.state, RB_QUEUE_AWAITING_CHOICE,
             "the queue is still parked on the re-armed prompt");
    CHECK_EQ(tg.state.p[0].deck.n, 0,
             "still nothing on the deck — the order is not finished");
}

/* choice.rs:2994-2998 — `parse::<usize>().filter(|&i| i < len).ok_or(..)`.
 * Every rejection is Rust's Err: the answer is dropped and the PROMPT STANDS. */
static void test_invalid_answer_leaves_the_prompt_standing(void)
{
    static const char *bad[] = { "9", "3", "-1", "x", "", "1,2", " 1", "1 " };
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);

    int pool[RB_MAX_ZONE];
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++) {
        int r = rb_move_handle_order_selection(&tg.state, bad[i]);
        CHECK_EQ(r, -1, "a malformed answer is Rust's Err, not a silent accept");
        CHECK_EQ(pool_of(0, pool), 3, "the pool is untouched by a rejected answer");
        CHECK(rb_has_pending_choice(&tg.state), "the prompt still stands");
        CHECK_EQ(tg.state.p[0].deck.n, 0, "a rejected answer places nothing");
    }
}

/* ── 3. the fix the whole cluster is about: the cards LAND on the deck ───── */

/* choice.rs:3021-3054. Pool [a,b,c]; answer "2" then "0" leaves one card, so
 * Rust drains it: ordered = [c, a, b]. Inserting each at deck position 0 in
 * reverse (b, a, c) leaves the deck as [c, a, b] — the FIRST-picked card on
 * top, in the order the player chose.
 *
 * The old C body left the deck at n=0 for this exact input. */
static void test_final_answer_places_every_card_on_the_deck(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    /* Pre-seed one unrelated card so the insert has to shift, not just append. */
    int pre = test_new_id(&tg, "PL!SP-bp2-011-R");
    if (pre < 0) { CHECK(0, "the pre-existing deck fixture resolves"); return; }
    test_add_to_deck(&tg, pre);
    CHECK_EQ(tg.state.p[0].deck.n, 1, "the deck starts with one card in it");

    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);
    CHECK_EQ(rb_move_handle_order_selection(&tg.state, "2"), 1, "the first pick re-arms");
    int r = rb_move_handle_order_selection(&tg.state, "0");
    CHECK_EQ(r, 0, "the last pick completes the order");

    const RbBag *D = &tg.state.p[0].deck;
    CHECK_EQ(D->n, 4, "all three cards plus the pre-existing one are on the deck");
    CHECK_EQ(D->cards[0], c[2], "the FIRST card picked is on TOP (choice.rs:3042)");
    CHECK_EQ(D->cards[1], c[0], "the second card picked is next");
    CHECK_EQ(D->cards[2], c[1], "the drained remainder is last of the three");
    CHECK_EQ(D->cards[3], pre, "the pre-existing deck card was shifted, not dropped");
    CHECK_EQ(pool_of(0, (int[RB_MAX_ZONE]){0}), 0,
             "the looked_at pool is emptied (Rust drains it)");
}

/* Two cards take ONE answer: after the pick the pool holds 1, so Rust's
 * `if len > 1` is false and the order completes immediately. */
static void test_two_cards_complete_in_one_answer(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);

    rb_order_prompt_begin(&tg.state, 0, c, 2, 0);
    int r = rb_move_handle_order_selection(&tg.state, "1");
    CHECK_EQ(r, 0, "a 2-card order completes on the first answer");
    const RbBag *D = &tg.state.p[0].deck;
    CHECK_EQ(D->n, 2, "both cards are on the deck");
    CHECK_EQ(D->cards[0], c[1], "the picked card is on top");
    CHECK_EQ(D->cards[1], c[0], "the unpicked card is beneath it");
}

/* A two-answer run must not carry accumulator state from the previous prompt:
 * Rust starts each order from an EMPTY `ordered` (choice.rs:2985-2992). */
static void test_accumulator_resets_between_prompts(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }

    clear_zones(&tg.state, 0);
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);
    rb_move_handle_order_selection(&tg.state, "0");
    rb_move_handle_order_selection(&tg.state, "0");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "the first order placed three cards");

    /* Second order, same three cards (they are back out of the deck in this
     * fixture; the engine does not revalidate membership at answer time). */
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);
    int r = rb_move_handle_order_selection(&tg.state, "0");
    CHECK_EQ(r, 1, "the second order re-arms on its first answer, not completes");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "no card is placed mid-order");
    r = rb_move_handle_order_selection(&tg.state, "0");
    CHECK_EQ(r, 0, "the second order completes on its second answer");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "the second order adds exactly three more");
    CHECK_EQ(tg.state.p[0].deck.cards[0], c[0], "the second order starts clean");
}

/* ── 4. the wrapper the SelectTarget dispatcher actually calls ───────────── */

/* choice.c: the `"order"` arm of rb_resolver_handle_select_target. On a
 * re-arm the wrapper must NOT clear the choice or drain — that is the other
 * half of the old bug (a drained prompt is invisible to the player).
 *
 * N cards take N-1 answers: after each pick the pool must hold MORE THAN ONE
 * for the prompt to re-arm (choice.rs:3004), and the single survivor is drained
 * rather than picked (choice.rs:3021). So 3 cards take 2 answers. The wrapper
 * must leave a pending choice after the first and none after the second. */
static void test_wrapper_re_arms_through_the_dispatcher(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);

    rb_resolver_handle_order_selection(&tg.state, "0");
    CHECK(rb_has_pending_choice(&tg.state),
          "after a re-arm the prompt is STILL pending (no clear, no drain)");
    CHECK_EQ(tg.state.queue.state, RB_QUEUE_AWAITING_CHOICE,
             "the queue is still parked");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "a re-arm places nothing");

    rb_resolver_handle_order_selection(&tg.state, "0");
    CHECK_EQ(pool_of(0, (int[RB_MAX_ZONE]){0}), 0,
             "the survivor is DRAINED into the order, leaving the pool empty "
             "(choice.rs:3021)");
    CHECK_EQ(tg.state.p[0].deck.n, 3,
             "the completing answer placed all three cards at once");

    CHECK(!rb_has_pending_choice(&tg.state),
          "the completing answer clears the choice state (choice.rs:3054)");
    CHECK_EQ(tg.state.p[0].deck.cards[0], c[0], "all three in the order picked");
    CHECK_EQ(tg.state.p[0].deck.cards[1], c[1], "second pick is second on the deck");
    CHECK_EQ(tg.state.p[0].deck.cards[2], c[2], "the drained survivor is last");
}

/* A rejected answer through the wrapper must also leave the prompt standing. */
static void test_wrapper_keeps_prompt_on_bad_answer(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    rb_order_prompt_begin(&tg.state, 0, c, 3, 0);

    rb_resolver_handle_order_selection(&tg.state, "7");
    CHECK(rb_has_pending_choice(&tg.state), "an out-of-range answer keeps the prompt");
    CHECK_EQ(pool_of(0, (int[RB_MAX_ZONE]){0}), 3, "and leaves the pool whole");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "and places nothing");

    rb_resolver_handle_order_selection(&tg.state, "2,0,1");
    CHECK_EQ(pool_of(0, (int[RB_MAX_ZONE]){0}), 3,
             "the old comma-separated answer form is now REJECTED, not honoured");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "and still places nothing");
}

/* ── 5. corpus reachability: what the missing producer must fire for ─────── */

/* There is no producer in the C yet (NOZOMI_ROOT_CAUSE.md BLOCKED-ON 2), so the
 * consumer above is reached by no ability. These two are the ONLY corpus shapes
 * that can reach it, and both are pinned here so the producer cannot be landed
 * against a stale or wrong shape:
 *
 *   PL!N-bp3-010-P ab#0 — choose_target_player -> effect_steps[0]:
 *       move_cards, source "discard", destination "deck_bottom", count 2,
 *       placement_order "any_order", action_by "self".
 *     mod.rs:1870 (discard is an eligible source) + :1869 (deck_bottom is a deck
 *     destination) + :1892 (any_order) + :1893 (taken.len() > 1) — the
 *     maybe_prompt_deck_order arm.
 *
 *   PL!-bp3-014-N ab#0 — look_and_select -> select_action:
 *       placement_order "any_order", any_number "true", destination "deck_top".
 *     mod.rs:3120-3125 needs_order, the Finalize arm.
 */
static const char *dex(const AbilityEffect *e, const char *k)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && strcmp(e->extra_k[i], k) == 0)
            return e->extra_v[i];
    return NULL;
}

static int ability_of(const char *card_no, int n, Ability *out)
{
    int c = rb_find_card_by_no(card_no);
    if (c < 0) return 0;
    if (n >= rb_card_num_abilities((uint32_t)c)) return 0;
    return rb_decode_card_ability((uint32_t)c, n, out);
}

static void test_corpus_reach(void)
{
    Ability a;

    if (ability_of("PL!N-bp3-010-P", 0, &a)) {
        const AbilityEffect *s = (a.effect && a.effect->n_child == 1)
                                 ? a.effect->child[0] : NULL;
        CHECK(s && s->action && strcmp(s->action, "move_cards") == 0,
              "PL!N-bp3-010-P ab#0 step 0 is the move_cards that wants an order");
        CHECK(s && s->source && strcmp(s->source, "discard") == 0,
              "its source is \"discard\" (an eligible source, mod.rs:1870)");
        CHECK(s && s->destination && strcmp(s->destination, "deck_bottom") == 0,
              "its destination is \"deck_bottom\" (a deck destination, mod.rs:1869)");
        CHECK_EQ(s ? s->count : -1, 2, "its count is 2, so taken.len() > 1 (mod.rs:1893)");
        const char *po = dex(s, "placement_order");
        CHECK(po && strcmp(po, "any_order") == 0,
              "its placement_order is \"any_order\" (mod.rs:1892)");
        rb_free_ability(&a);
    } else {
        CHECK(0, "PL!N-bp3-010-P ab#0 decodes");
    }

    if (ability_of("PL!-bp3-014-N", 0, &a)) {
        const AbilityEffect *sel = a.effect ? a.effect->select_action : NULL;
        const char *po = dex(sel, "placement_order");
        CHECK(po && strcmp(po, "any_order") == 0,
              "PL!-bp3-014-N ab#0 select_action placement_order is \"any_order\"");
        CHECK(sel && sel->destination && strcmp(sel->destination, "deck_top") == 0,
              "PL!-bp3-014-N ab#0 select_action destination is \"deck_top\" (mod.rs:3120)");
        rb_free_ability(&a);
    } else {
        CHECK(0, "PL!-bp3-014-N ab#0 decodes");
    }
}

/* ── 6. the PRODUCER: rb_move_maybe_prompt_deck_order ────────────────────── */

/* The producer is the twin of Rust's
 *   engine/src/ability/effects/move/mod.rs  maybe_prompt_deck_order
 * called from `execute_move_cards` at the position after the two
 * `taken.is_empty()` early-outs and BEFORE the distinct dedupe. Rust reference
 * cited by content (that file is being edited), doc comment:
 *   "Deck-order prompt for multi-card moves to the deck with AnyOrder.
 *    Returns true when the prompt was issued (caller must return early)."
 *
 * It is non-static in src/ability/effects/move_missing.c; include/rabuka.h is
 * owned elsewhere, so the prototype is repeated here. */
extern int rb_move_maybe_prompt_deck_order(GameState *g, int actor,
                                           const AbilityEffect *e,
                                           const int *taken, int n_taken);

/* A minimal move_cards effect carrying only the members the producer reads.
 * `placement_order` and `all` are both wire KEYS that vm.c stores in the extras
 * table (effect_decode_placement_order / the generic extras reader), never as
 * struct fields, so this is the shape a decoded node really has. */
static void order_effect(AbilityEffect *e, const char *source,
                         const char *destination, const char *placement_order,
                         int all, int count)
{
    memset(e, 0, sizeof *e);
    e->action = (char *)"move_cards";
    e->source = (char *)source;
    e->destination = (char *)destination;
    e->count = count;
    e->n_extra = 0;
    if (placement_order) {
        e->extra_k[e->n_extra] = (char *)"placement_order";
        e->extra_v[e->n_extra] = (char *)placement_order;
        e->n_extra++;
    }
    if (all) {
        e->extra_k[e->n_extra] = (char *)"all";
        e->extra_v[e->n_extra] = (char *)"true";
        e->n_extra++;
    }
}

/* mod.rs:2272 is INSIDE `execute_move_cards`, so the honest end-to-end
 * assertion drives the exported entry the effect dispatcher routes through
 * (move.c: `rb_effect_move_cards` -> `rb_move_execute_move_cards_ported`) with a
 * three-card discard, which is exactly the shape of PL!N-bp3-010-P ab#0. */
static void test_executor_raises_the_order_prompt(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, c[i]);

    AbilityEffect e;
    order_effect(&e, "discard", "deck_bottom", "any_order", 0, 3);
    rb_effect_move_cards(&tg.state, 0, &e);

    CHECK(rb_has_pending_choice(&tg.state),
          "a 3-card discard -> deck_bottom any_order move now REACHES the order "
          "prompt (it did not before the producer existed)");
    CHECK_EQ(tg.state.queue.pending.kind, RB_CHOICE_SELECT_TARGET,
             "what it raised is a SelectTarget (mod.rs:1915-1925)");
    CHECK(tg.state.queue.pending.target[0] &&
          strcmp(tg.state.queue.pending.target, "order") == 0,
          "with target \"order\", so the SelectTarget dispatcher routes to "
          "rb_resolver_handle_order_selection");
    CHECK(strstr(tg.state.queue.pending.description, "(3 cards)") != NULL,
          "the description counts the take (mod.rs:1917)");

    int pool[RB_MAX_ZONE];
    CHECK_EQ(pool_of(0, pool), 3, "all three taken cards are in the pool");
    CHECK_EQ(pool[0], c[0], "the offer follows the take's own order (discard)");
    CHECK_EQ(tg.state.p[0].deck.n, 0,
             "the take is NOT placed yet — the order prompt owns it now "
             "(mod.rs:2272 returns early, placement is skipped)");
}

/* The producer is only a predicate plus a forward: rb_order_prompt_begin owns
 * the prompt, the pool and the answer state. Proving the hand-off end to end is
 * what distinguishes "the producer fired" from "the producer fired AND the
 * landed consumer still works". */
static void test_executor_prompt_is_answered_onto_the_deck(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, c[i]);

    AbilityEffect e;
    order_effect(&e, "discard", "deck_bottom", "any_order", 0, 3);
    rb_effect_move_cards(&tg.state, 0, &e);
    if (!rb_has_pending_choice(&tg.state)) {
        CHECK(0, "SETUP: the order prompt is pending");
        return;
    }

    /* 3 cards take N-1 = 2 answers (choice.rs:3004 re-arms only while MORE THAN
     * ONE card remains; choice.rs:3021 drains the survivor). */
    rb_resume_with_choice(&tg.state, 0);
    CHECK(rb_has_pending_choice(&tg.state),
          "answering once out of three re-arms the same prompt (choice.rs:3019)");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "a re-arm places nothing yet");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(!rb_has_pending_choice(&tg.state), "the second answer completes it");

    const RbBag *D = &tg.state.p[0].deck;
    CHECK_EQ(D->n, 3, "the three taken cards landed on the deck");
    CHECK_EQ(D->cards[0], c[0], "the FIRST card picked is on TOP (choice.rs:3042)");
    CHECK_EQ(D->cards[1], c[1], "the second pick is next");
    CHECK_EQ(D->cards[2], c[2], "the drained survivor is last");
}

/* Each leg of Rust's guard, one at a time. `n > 1` and the four arms of
 * `is_deck_dest` and `is_eligible_source` are all `&&` conjuncts, so each is
 * asserted in isolation: a producer that quietly dropped one would still pass a
 * single end-to-end case. */
static void test_producer_guard_legs(void)
{
    int c[3];
    static const char *deck_dests[] = { "deck", "deck_top", "deck_bottom" };
    static const char *other_dests[] = {
        "deck_top_or_bottom", "discard", "hand", "stage", "same_area",
        "empty_area", "waitroom", "live_card_zone", "success_live_zone", ""
    };
    static const char *eligible_sources[] = { "discard", "selected_cards", "hand" };
    static const char *other_sources[] = {
        "looked_at", "recently_moved", "preceding_moved", "those_cards",
        "revealed_cards", "stage", "energy", "live_card_zone", "", "nonsense"
    };

    for (size_t i = 0; i < sizeof deck_dests / sizeof deck_dests[0]; i++) {
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        AbilityEffect e;
        order_effect(&e, "discard", deck_dests[i], "any_order", 0, 3);
        char msg[128];
        snprintf(msg, sizeof msg,
                 "destination \"%s\" is a deck destination (mod.rs:1864-1869)",
                 deck_dests[i]);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), 1, msg);
    }
    for (size_t i = 0; i < sizeof other_dests / sizeof other_dests[0]; i++) {
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        AbilityEffect e;
        order_effect(&e, "discard", other_dests[i], "any_order", 0, 3);
        char msg[128];
        snprintf(msg, sizeof msg,
                 "destination \"%s\" is NOT a deck destination, so no prompt",
                 other_dests[i]);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), 0, msg);
    }
    for (size_t i = 0; i < sizeof eligible_sources / sizeof eligible_sources[0]; i++) {
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        AbilityEffect e;
        order_effect(&e, eligible_sources[i], "deck_top", "any_order", 0, 3);
        char msg[128];
        snprintf(msg, sizeof msg,
                 "source \"%s\" is an eligible source (mod.rs:1870-1872)",
                 eligible_sources[i]);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), 1, msg);
    }
    for (size_t i = 0; i < sizeof other_sources / sizeof other_sources[0]; i++) {
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        AbilityEffect e;
        order_effect(&e, other_sources[i], "deck_top", "any_order", 0, 3);
        char msg[128];
        snprintf(msg, sizeof msg,
                 "source \"%s\" is not an eligible source without the "
                 "all-extras key", other_sources[i]);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), 0, msg);
    }

    /* mod.rs:1873-1874 — LookedAt is eligible ONLY with an explicit 「すべて」. */
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    AbilityEffect la;
    order_effect(&la, "looked_at", "deck_top", "any_order", 0, 3);
    CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &la, c, 3), 0,
             "a looked_at source with no all-extras key is not eligible (mod.rs:1874)");
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    AbilityEffect la2;
    order_effect(&la2, "looked_at", "deck_top", "any_order", 1, 3);
    CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &la2, c, 3), 1,
             "looked_at WITH 「すべて」 is eligible (mod.rs:1873)");

    /* mod.rs:1892 — only PlacementOrder::AnyOrder reaches the prompt, and
     * vm.c's decoder only ever writes that one literal, so this string compare
     * IS the enum test. */
    static const char *orders[] = { NULL, "", "fixed", "any_order_x", "AnyOrder" };
    for (size_t i = 0; i < sizeof orders / sizeof orders[0]; i++) {
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        AbilityEffect e;
        order_effect(&e, "discard", "deck_top", orders[i], 0, 3);
        int expect = (orders[i] && !strcmp(orders[i], "any_order")) ? 1 : 0;
        char msg[128];
        snprintf(msg, sizeof msg,
                 "placement_order \"%s\" is %sany_order, so the prompt is %s",
                 orders[i] ? orders[i] : "(absent)",
                 expect ? "" : "not ", expect ? "issued" : "withheld");
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), expect, msg);
    }

    /* mod.rs:1893 `taken.len() > 1`. */
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    AbilityEffect one;
    order_effect(&one, "discard", "deck_top", "any_order", 0, 1);
    CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &one, c, 1), 0,
             "one taken card asks for no order (mod.rs:1893)");
    CHECK(!rb_has_pending_choice(&tg.state), "and no choice is left behind");
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &one, c, 2), 1,
             "the very same effect with two taken cards DOES ask (mod.rs:1893)");
}

/* mod.rs:1906-1908 — `order_pool.reverse()` for a Hand source. The producer is
 * what decides this, so it is pinned HERE as well as on the consumer. */
static void test_producer_reverses_a_hand_source_pool(void)
{
    int c[3];
    test_game_new(&tg);
    if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
    clear_zones(&tg.state, 0);
    for (int i = 0; i < 3; i++) test_add_to_hand(&tg, c[i]);

    AbilityEffect e;
    order_effect(&e, "hand", "deck_top", "any_order", 0, 3);
    CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, &e, c, 3), 1,
             "a 3-card hand -> deck_top any_order move asks for an order");
    int pool[RB_MAX_ZONE];
    CHECK_EQ(pool_of(0, pool), 3, "all three hand cards are in the pool");
    CHECK_EQ(pool[0], c[2], "a Hand source offers the pool REVERSED (mod.rs:1907)");
    CHECK_EQ(pool[2], c[0], "the reversal is total");
}

/* The REAL decoded node, not a hand-built one: `PL!N-bp3-010-P` ab#0 step 0 is
 * the corpus shape the new suite's test_corpus_reach pins, and the producer has
 * to accept the node the engine's own decoder produces. */
static void test_producer_accepts_the_real_corpus_node(void)
{
    Ability a;
    if (!ability_of("PL!N-bp3-010-P", 0, &a)) {
        CHECK(0, "PL!N-bp3-010-P ab#0 decodes");
        return;
    }
    const AbilityEffect *s = (a.effect && a.effect->n_child == 1) ? a.effect->child[0] : NULL;
    CHECK(s != NULL, "its single effect step decodes");
    if (s) {
        int c[3];
        test_game_new(&tg);
        if (!three_cards(c)) { CHECK(0, "three distinct card fixtures resolve"); return; }
        clear_zones(&tg.state, 0);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, s, c, 2), 1,
                 "the REAL decoded PL!N-bp3-010-P step raises the order prompt");
        CHECK(tg.state.queue.pending.target[0] &&
              strcmp(tg.state.queue.pending.target, "order") == 0,
              "and it is the SelectTarget \"order\" choice");
        test_game_new(&tg);
        clear_zones(&tg.state, 0);
        CHECK_EQ(rb_move_maybe_prompt_deck_order(&tg.state, 0, s, c, 1), 0,
                 "the same node with a ONE-card take asks for nothing (mod.rs:1893)");
    }
    rb_free_ability(&a);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_prompt_arms_select_target_order();
    test_hand_source_reverses_the_pool();
    test_single_card_issues_no_prompt();

    test_one_index_per_answer_shrinks_the_pool();
    test_invalid_answer_leaves_the_prompt_standing();

    test_final_answer_places_every_card_on_the_deck();
    test_two_cards_complete_in_one_answer();
    test_accumulator_resets_between_prompts();

    test_wrapper_re_arms_through_the_dispatcher();
    test_wrapper_keeps_prompt_on_bad_answer();

    test_corpus_reach();

    test_executor_raises_the_order_prompt();
    test_executor_prompt_is_answered_onto_the_deck();
    test_producer_guard_legs();
    test_producer_reverses_a_hand_source_pool();
    test_producer_accepts_the_real_corpus_node();

    printf("\n%d assertions, %d failures\n", assertions, failures);
    return failures ? 1 : 0;
}

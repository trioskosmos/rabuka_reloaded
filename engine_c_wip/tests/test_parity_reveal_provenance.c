/* tests/test_parity_reveal_provenance.c
 *
 * E5 + E6 and the reveal-provenance call sites, translated from the ORACLE at
 * engine/src/core/game_state/modifiers.rs and
 * engine/src/ability/effects/move/mod.rs. The oracle is never edited.
 *
 *   E5  remove_revealed_cost_card        modifiers.rs:2121-2132
 *         if let Some(index) = self.revealed_cost_cards.iter()
 *                                  .position(|id| *id == card_id) {
 *             self.revealed_cost_cards.remove(index);
 *             if index < self.revealed_cost_card_meta.len() {
 *                 self.revealed_cost_card_meta.remove(index);
 *             }
 *         }
 *
 *   E6  remove_from_source_hands         modifiers.rs:2139-2158
 *         let mut seen = HashSet::<i16>::default();
 *         for &cid in card_ids {
 *             if !seen.insert(cid) { continue; }
 *             // Only remove from hand if the card was from a cost reveal
 *             // (tracked in revealed_cost_cards). Non-cost reveals (deck peek,
 *             // etc.) should NOT remove from hand.
 *             if !self.revealed_cost_cards.contains(&cid) { continue; }
 *             for player in [&mut self.player1, &mut self.player2] {
 *                 if let Some(pos) = player.hand.cards.iter()
 *                                             .position(|&c| c == cid) {
 *                     player.hand.remove_card(pos);
 *                     break;
 *                 }
 *             }
 *         }
 *
 *   and the two CALL SITES that were absent from the C:
 *     move/mod.rs:614    gs.remove_from_source_hands(&taken)     (E6)
 *     move/mod.rs:615-633  the deck sweep and the waitroom sweep
 *     move/mod.rs:1535   gs.remove_revealed_card(card_id)
 *     move/mod.rs:1536   gs.remove_revealed_cost_card(card_id)  (E5)
 *     move/mod.rs:1954-1960  the head of place_taken_cards:
 *         for &card_id in taken {
 *             let was_cost_revealed = gs.revealed_cost_cards.contains(&card_id);
 *             if was_cost_revealed { gs.remove_revealed_card(card_id); }
 *             gs.remove_revealed_cost_card(card_id);
 *         }
 *
 * WHY THIS FILE EXISTS. E5 and E6 had NO C twin at all, and nothing in the C
 * ever removed a card from `revealed_cards[]` or `revealed_cost_cards[]`
 * (rb_remove_revealed_card had zero call sites before this change), so a card
 * that left a zone kept its reveal provenance forever. include/rabuka.h:1591-1609
 * recorded E5/E6 as blocked on the producer `push_revealed_cost_card`; that
 * producer has since landed (src/ability/cost.c:1913-1914), so both readers are
 * reachable and are ported here.
 *
 * A REVEAL PEEKS, IT DOES NOT DRAIN. That single fact is what makes the three
 * post-drain sweeps at move/mod.rs:615-633 load-bearing: taking a revealed card
 * copies it, and nothing else removes the original, so without them the deck and
 * the waitroom each keep a card the engine has also handed to the player.
 *
 * WHAT IS ASSERTED IS WHAT RUST ASSERTS, AT RUST'S LINES:
 *   1  E5 removes the FIRST match only (modifiers.rs:2123 `.position()`), so two
 *      copies of one print as a cost keep one provenance entry
 *   2  E5 is a no-op for an id that is not in the list (the `if let Some`)
 *   3  E6's `revealed_cost_cards.contains` gate: a deck PEEK leaves the card in
 *      hand, a PAID cost does not (modifiers.rs:2148)
 *   4  E6's `seen` set: a duplicated id in `card_ids` clears ONE hand slot
 *   5  E6's `break`: at most one hand slot across BOTH seats, player1 first
 *   6  the revealed-cards take drops the hand slot, the deck copy and the
 *      waitroom copy (move/mod.rs:614-633)
 *
 * CARD NUMBER: PL!-sd1-010-SD (member), verified against cards/cards.json, as
 * in tests/test_parity_need_heart_live_flow.c:70. It is reached only through
 * test_new_id, whose -1 return is checked. No substring lookup is used, so the
 * rb_find_card_by_no near-miss trap does not apply.
 *
 * sizeof(GameState) is pinned at 1 090 264 B by tests/test_harness_self.c §A4,
 * nothing here adds a field, and every TestGame below is `static`.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* E5 + E6 are defined non-static in src/core/modifiers.c but are NOT surfaced in
 * include/rabuka.h, which the coordinator edits centrally. File-local prototypes
 * keep this suite self-contained -- the same precedent
 * tests/test_parity_need_heart_live_flow.c:99 sets for E3/E4. */
void rb_remove_revealed_cost_card(GameState *g, int card_id);
void rb_remove_from_source_hands(GameState *g, const int *card_ids, int n_ids);
/* src/ability/effects/move.c:1727 -- also unsurfaced. */
int rb_move_resolve_from_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int is_max,
                                         int *out_ids, int max);

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", (msg)); failures++; FLAT_VERDICT(); } \
    else printf("ok: %s\n", (msg)); \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", (msg), a_, e_); \
        failures++; FLAT_VERDICT(); \
    } else printf("ok: %s\n", (msg)); \
} while (0)

static const char *CARD_MEMBER = "PL!-sd1-010-SD";

static int has_id(const RbBag *b, int cid) {
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static void push_cost_reveal(GameState *g, int cid) {
    if (g->n_revealed_cost_cards < RB_MAX_ZONE)
        g->revealed_cost_cards[g->n_revealed_cost_cards++] = cid;
}

/* ── 1 + 2 ── E5 removes the FIRST match, and nothing else.
 *      modifiers.rs:2121-2132. */
static void e5_removes_first_match_only(void) {
    static TestGame tg;
    test_game_new(&tg);

    int one = test_new_id(&tg, CARD_MEMBER);
    int other = test_new_id(&tg, CARD_MEMBER);
    if (one < 0 || other < 0) {
        CHECK(0, "fixtures resolve (two distinct member instances)");
        return;
    }

    /* Two copies of ONE print paid as a cost is the shape that makes
     * `.position()` observable: two separate instances of the same card_no. */
    push_cost_reveal(&tg.state, one);
    push_cost_reveal(&tg.state, one);
    push_cost_reveal(&tg.state, other);
    CHECK_EQ(tg.state.n_revealed_cost_cards, 3, "CONTROL: three cost-reveal entries");

    rb_remove_revealed_cost_card(&tg.state, one);
    CHECK_EQ(tg.state.n_revealed_cost_cards, 2,
             "modifiers.rs:2123-2125 -- ONE entry removed, not both copies: "
             "`position()` finds the first match and Vec::remove takes one");
    {
        int still_one = 0;
        for (int i = 0; i < tg.state.n_revealed_cost_cards; i++)
            if (tg.state.revealed_cost_cards[i] == one) still_one++;
        CHECK_EQ(still_one, 1,
                 "the surviving copy is still in the list and the OTHER card's "
                 "entry is untouched -- this is a remove-one, not a filter");
    }
    CHECK_EQ(tg.state.revealed_cost_cards[0], one,
             "the FIRST entry was the one removed (index order is preserved, "
             "not reversed)");

    /* The `if let Some(...)` arm: an id that is not in the list changes nothing. */
    rb_remove_revealed_cost_card(&tg.state, -12345);
    CHECK_EQ(tg.state.n_revealed_cost_cards, 2,
             "modifiers.rs:2123 -- an id with no match is a no-op, not a shift");

    rb_remove_revealed_cost_card(&tg.state, one);
    CHECK_EQ(tg.state.n_revealed_cost_cards, 1,
             "the second removal takes the remaining copy");
    CHECK_EQ(tg.state.revealed_cost_cards[0], other,
             "and `other` is left, so the list really is per-id and not "
             "positional");
}

/* ── 3 ── E6's cost-reveal gate: a PEEK keeps the card in hand.
 *      modifiers.rs:2146-2149. */
static void e6_requires_cost_reveal_provenance(void) {
    static TestGame tg;
    test_game_new(&tg);

    int paid = test_new_id(&tg, CARD_MEMBER);
    int peeked = test_new_id(&tg, CARD_MEMBER);
    if (paid < 0 || peeked < 0) {
        CHECK(0, "fixtures resolve (two distinct member instances)");
        return;
    }

    test_add_to_hand_for(&tg, 0, paid);
    test_add_to_hand_for(&tg, 0, peeked);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "CONTROL: two cards in player1's hand");

    /* ONLY `paid` gets cost-reveal provenance. `peeked` is in revealed_cards
     * -- a deck peek / yell reveal -- which Rust deliberately does NOT treat as
     * a cost. The test asserts the gate, so `peeked` must be reachable state
     * that the engine itself would have produced. */
    test_add_to_revealed(&tg, paid);
    test_add_to_revealed(&tg, peeked);
    push_cost_reveal(&tg.state, paid);
    CHECK(tg.state.n_revealed >= 2 && tg.state.n_revealed_cost_cards == 1,
          "CONTROL: both are revealed, but only one is a COST reveal");

    int ids[2]; ids[0] = paid; ids[1] = peeked;
    rb_remove_from_source_hands(&tg.state, ids, 2);

    CHECK_EQ(has_id(&tg.state.p[0].hand, paid), 0,
             "modifiers.rs:2148+2151-2154 -- the COST-revealed card leaves the "
             "hand: paying with a card moves it, so the hand copy must go");
    CHECK_EQ(has_id(&tg.state.p[0].hand, peeked), 1,
             "modifiers.rs:2146-2147 -- the PEEKED card STAYS in hand: "
             "`revealed_cards` is not `revealed_cost_cards`, and a reveal only "
             "peeks. Clearing it here would delete a card the player still owns");
}

/* ── 4 ── E6's `seen` set: a duplicated id clears ONE hand slot.
 *      modifiers.rs:2141-2142. */
static void e6_dedupes_repeated_ids(void) {
    static TestGame tg;
    test_game_new(&tg);

    int cid = test_new_id(&tg, CARD_MEMBER);
    if (cid < 0) { CHECK(0, "fixtures resolve (one member instance)"); return; }

    test_add_to_hand_for(&tg, 0, cid);
    push_cost_reveal(&tg.state, cid);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "CONTROL: the card is in hand once");

    int ids[3]; ids[0] = cid; ids[1] = cid; ids[2] = cid;
    rb_remove_from_source_hands(&tg.state, ids, 3);
    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "modifiers.rs:2141-2142 -- `seen.insert` short-circuits the repeats, "
             "and since `remove_card` shifts the tail a second pass would strip a "
             "different slot. One id, one removal");

    /* The negative that makes the above non-vacuous in the other direction: an id
     * that is NOT a cost reveal is never removed even when it is repeated. */
    int peeked = test_new_id(&tg, CARD_MEMBER);
    if (peeked < 0) return;
    test_add_to_hand_for(&tg, 0, peeked);
    int ids2[2]; ids2[0] = peeked; ids2[1] = peeked;
    rb_remove_from_source_hands(&tg.state, ids2, 2);
    CHECK_EQ(has_id(&tg.state.p[0].hand, peeked), 1,
             "the `seen` dedupe cannot rescue a card that failed the cost-reveal "
             "gate -- the gate is checked once per DISTINCT id");
}

/* ── 5 ── E6's `break`: at most one seat loses the card, player1 first.
 *      modifiers.rs:2151-2157. */
static void e6_clears_only_the_first_seat(void) {
    static TestGame tg;
    test_game_new(&tg);

    int cid = test_new_id(&tg, CARD_MEMBER);
    if (cid < 0) { CHECK(0, "fixtures resolve (one member instance)"); return; }

    /* Deliberately the SAME id in both hands. It is not a legal board, and that
     * is the point: it is the only way to observe the `break`, because a legal
     * card is in exactly one hand and the assertion would otherwise be
     * unfalsifiable. */
    tg.state.p[0].hand.cards[tg.state.p[0].hand.n++] = cid;
    tg.state.p[1].hand.cards[tg.state.p[1].hand.n++] = cid;
    push_cost_reveal(&tg.state, cid);
    CHECK_EQ(tg.state.p[0].hand.n + tg.state.p[1].hand.n, 2,
             "CONTROL: the same id sits in both hands");

    int ids[1]; ids[0] = cid;
    rb_remove_from_source_hands(&tg.state, ids, 1);

    CHECK_EQ(has_id(&tg.state.p[0].hand, cid), 0,
             "modifiers.rs:2151-2157 -- player1 is iterated first and its hit "
             "removes the slot");
    CHECK_EQ(has_id(&tg.state.p[1].hand, cid), 1,
             "and the `break` stops there: player2 KEEPS its copy. A port that "
             "looped both seats without the break would clear both");
}

/* ── 6 ── the take itself: a reveal PEEKS, so the take must drain the three
 *      copies. move/mod.rs:604-633. */
static void revealed_cards_take_drains_every_copy(void) {
    static TestGame tg;
    test_game_new(&tg);

    int paid    = test_new_id(&tg, CARD_MEMBER);
    int peeked  = test_new_id(&tg, CARD_MEMBER);
    int from_dk = test_new_id(&tg, CARD_MEMBER);
    int from_wr = test_new_id(&tg, CARD_MEMBER);
    if (paid < 0 || peeked < 0 || from_dk < 0 || from_wr < 0) {
        CHECK(0, "fixtures resolve (four distinct member instances)");
        return;
    }

    /* paid:    revealed as a COST, and still in the hand -> hand slot drained.
     * peeked:  revealed, NOT a cost, and still in the hand -> hand slot KEPT.
     * from_dk: revealed, and physically still in player1's deck.
     * from_wr: revealed, and physically in player1's waitroom (Rule 8.4.7: yell
     *          cards go to the waitroom after check_live_success drains the
     *          resolution zone). */
    test_add_to_revealed(&tg, paid);
    test_add_to_revealed(&tg, peeked);
    test_add_to_revealed(&tg, from_dk);
    test_add_to_revealed(&tg, from_wr);
    push_cost_reveal(&tg.state, paid);

    test_add_to_hand_for(&tg, 0, paid);
    test_add_to_hand_for(&tg, 0, peeked);
    tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = from_dk;
    test_add_to_discard(&tg, from_wr);

    int n_rev = tg.state.n_revealed;
    CHECK_EQ(n_rev, 4, "CONTROL: four revealed cards");
    CHECK_EQ(has_id(&tg.state.p[0].deck, from_dk), 1, "CONTROL: deck copy is real");
    CHECK_EQ(has_id(&tg.state.p[0].discard, from_wr), 1,
             "CONTROL: waitroom copy is real");

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.count = n_rev;            /* take_count = min(count, n_revealed) */

    int out[8];
    int n = rb_move_resolve_from_revealed_cards(&tg.state, 0, &e,
                                                n_rev, 0, 0, out, 8);
    if (n != n_rev) {
        CHECK(0, "SETUP: the take consumed the whole revealed pool without prompting");
        return;
    }

    CHECK_EQ(tg.state.n_revealed, 0,
             "mod.rs:604-613 -- the pool itself is drained (this already worked "
             "before the change and is the control for the next three)");

    CHECK_EQ(has_id(&tg.state.p[0].hand, paid), 0,
             "mod.rs:614 -> modifiers.rs:2148 -- the COST-revealed card's hand "
             "copy is drained by E6");
    CHECK_EQ(has_id(&tg.state.p[0].hand, peeked), 1,
             "mod.rs:614 -> modifiers.rs:2146-2147 -- the PEEKED card's hand "
             "copy survives, because E6's gate is `revealed_cost_cards`, not "
             "`revealed_cards`");
    CHECK_EQ(has_id(&tg.state.p[0].deck, from_dk), 0,
             "mod.rs:615-623 -- \"the reveal only peeked, not drained ... it's "
             "still in the deck and must be removed here to avoid duplication\"");
    CHECK_EQ(has_id(&tg.state.p[0].discard, from_wr), 0,
             "mod.rs:625-632 -- the waitroom sweep, Rule 8.4.7: yell cards "
             "reach the waitroom and would otherwise be duplicated");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    e5_removes_first_match_only();
    e6_requires_cost_reveal_provenance();
    e6_dedupes_repeated_ids();
    e6_clears_only_the_first_seat();
    revealed_cards_take_drains_every_copy();

    if (failures) {
        fprintf(stderr, "FAILED: parity_reveal_provenance  [%d failure(s)]\n", failures);
        return 1;
    }
    printf("PASSED: parity_reveal_provenance\n");
    return 0;
}
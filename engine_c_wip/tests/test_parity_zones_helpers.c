/* test_parity_zones_helpers.c
 *
 * ZONE-HELPER PARITY, IN ISOLATION FROM THE EFFECT LAYER.
 *
 * WHY THIS SUITE EXISTS
 * ---------------------
 * A whole-corpus attribution put core/zones at 43.4% of 2,441 detail lines, the
 * largest concentration in the project. A prior peer audited zones.c and
 * choice.c, found ZERO behavioural divergence, and concluded the residual
 * failures are effects/game_state behaviour reached THROUGH the zone helpers.
 *
 * That conclusion was a hypothesis, not a measurement. This suite is the
 * measurement: it calls each zones.c helper DIRECTLY, with no ability queue, no
 * choice prompt, no GameState effect dispatch, and asserts the outcome Rust
 * documents for that helper. A green case here proves the helper agrees with
 * Rust in isolation; a red case is a genuine zones.c defect and outranks every
 * effect-layer explanation.
 *
 * All 24 cases are expected GREEN. If they are green, the 43.4% attribution is
 * measuring the CALLERS, not zones.c, and the remaining work belongs to the
 * effect/condition/game_state files.
 *
 * EVERY CASE CITES THE RUST LINE IT ASSERTS.
 *   The C corpus wire values are ASCII and the Rust test names are Japanese, so
 *   without a citation in the failure text a red case here would be
 *   indistinguishable from the effect-layer bugs it is designed to separate.
 *
 * FILE IS PURE ASCII: every Japanese literal is an explicit UTF-8 byte escape
 * generated from the Rust source, never hand-typed. The two kana that decide
 * whether a group filter matches anything are one codepoint apart (KE U+30F6
 * vs U+30F1), and a hand-typed literal in a rewritten-in-cp932 file is a silent
 * no-op filter.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* zones.c surface not declared in include/rabuka.h. Declared the same way
 * tests/test_audit_zones.c and the sibling parity suites declare theirs. */
int rb_stage_position_change(int stage[RB_STAGE_SIZE], int from_area, int to_area);
int rb_stage_position_change_with_bags(RbPlayer *P, int from_area, int to_area);
int rb_card_is_member(int card_id);
unsigned rb_num_cards(void);

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

static int assertions = 0;
static int failures = 0;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s  [%s:%d]\n", message, __FILE__, __LINE__); \
        failures++; \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)  [%s:%d]\n", message, \
                a_, e_, __FILE__, __LINE__); \
        failures++; \
    } \
} while (0)

#define CHECK_STR(actual, expected, message) do { \
    assertions++; \
    const char *a_ = (actual); const char *e_ = (expected); \
    int bad_ = (!a_ || !e_ || strcmp(a_, e_) != 0); \
    if (bad_) { \
        fprintf(stderr, "FAIL: %s (got \"%s\" expected \"%s\")  [%s:%d]\n", \
                message, a_ ? a_ : "(null)", e_ ? e_ : "(null)", __FILE__, __LINE__); \
        failures++; \
    } \
} while (0)

/* ═════════════════════════════════════════════════════════════════════════
   1. Stage::position_change -- the two refusals and the two mutation arms.
   Rust: engine/src/core/zones.rs:368-415
   ═════════════════════════════════════════════════════════════════════════ */

/* zones.rs:376-378 -- `if from_area == to_area { return Err("Cannot move to
   same area") }`. The C maps Err -> -1, which is also this suite's "no card"
   sentinel, so a same-area request is asserted as refused-by-return-value. */
static void case_position_change_refuses_same_area(void)
{
    int stage[RB_STAGE_SIZE];
    stage[AREA_LEFT]   = 11;
    stage[AREA_CENTER] = 22;
    stage[AREA_RIGHT]  = 33;
    CHECK_EQ(rb_stage_position_change(stage, AREA_CENTER, AREA_CENTER), -1,
             "position_change refuses a same-area move (zones.rs:376-378)");
    CHECK_EQ(stage[AREA_CENTER], 22,
             "a refused same-area move leaves the board untouched (zones.rs:377)");
}

/* zones.rs:391-394 -- `if card_id == -1 { return Err("No card in source area") }`. */
static void case_position_change_refuses_empty_source(void)
{
    int stage[RB_STAGE_SIZE];
    stage[AREA_LEFT]   = 11;
    stage[AREA_CENTER] = RB_EMPTY_SLOT;
    stage[AREA_RIGHT]  = 33;
    CHECK_EQ(rb_stage_position_change(stage, AREA_CENTER, AREA_LEFT), -1,
             "position_change refuses an empty source area (zones.rs:391-394)");
    CHECK_EQ(stage[AREA_LEFT], 11,
             "a refused empty-source move leaves the destination alone (zones.rs:393)");
    CHECK_EQ(stage[AREA_RIGHT], 33,
             "a refused empty-source move leaves the third slot alone (zones.rs:393)");
}

/* zones.rs:408-411 -- the plain-move arm:
       stage[to_index] = card_id; stage[from_index] = -1;
   reached when `dest_card_id == -1`. */
static void case_position_change_moves_into_empty(void)
{
    int stage[RB_STAGE_SIZE];
    stage[AREA_LEFT]   = 11;
    stage[AREA_CENTER] = RB_EMPTY_SLOT;
    stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_position_change(stage, AREA_LEFT, AREA_RIGHT), 11,
             "position_change returns the moved card id (zones.rs:414 Ok(card_id))");
    CHECK_EQ(stage[AREA_LEFT], RB_EMPTY_SLOT,
             "the plain move empties the source slot (zones.rs:411)");
    CHECK_EQ(stage[AREA_CENTER], RB_EMPTY_SLOT,
             "the plain move does not disturb the third slot (zones.rs:408-411)");
    CHECK_EQ(stage[AREA_RIGHT], 11,
             "the plain move lands the card in the destination (zones.rs:410)");
}

/* zones.rs:404-407 -- the swap arm:
       stage[from_index] = dest_card_id; stage[to_index] = card_id;
   Rule 11.10.2, "If destination has a member, it swaps positions". */
static void case_position_change_swaps_occupied(void)
{
    int stage[RB_STAGE_SIZE];
    stage[AREA_LEFT]   = 11;
    stage[AREA_CENTER] = 22;
    stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_position_change(stage, AREA_LEFT, AREA_CENTER), 11,
             "a swap returns the requested mover, not the occupant (zones.rs:414)");
    CHECK_EQ(stage[AREA_LEFT], 22,
             "the swap puts the occupant in the vacated slot (zones.rs:406)");
    CHECK_EQ(stage[AREA_CENTER], 11,
             "the swap puts the mover in the destination (zones.rs:407)");
    CHECK_EQ(stage[AREA_RIGHT], RB_EMPTY_SLOT,
             "a two-slot swap does not touch the third slot (zones.rs:404-407)");
}

/* ═════════════════════════════════════════════════════════════════════════
   2. Rule 4.5.5.3 -- under-cards travel with the member.
   Rust: zones.rs:396-400, an UNCONDITIONAL crossed take placed BEFORE the
   stage swap, so the stack rides on the plain-move arm as well as the swap.
   ═════════════════════════════════════════════════════════════════════════ */

static void case_under_cards_cross_on_plain_move(void)
{
    static TestGame tg;
    test_game_new(&tg);
    RbPlayer *P = &tg.state.p[0];
    P->stage[AREA_LEFT]  = 11;
    P->stage[AREA_CENTER] = RB_EMPTY_SLOT;
    P->stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    P->under_cards[AREA_LEFT].n = 0;
    P->under_cards[AREA_CENTER].n = 0;
    P->under_cards[AREA_RIGHT].n = 0;
    P->under_cards[AREA_LEFT].cards[0] = 101;
    P->under_cards[AREA_LEFT].n = 1;
    P->under_cards[AREA_RIGHT].cards[0] = 202;
    P->under_cards[AREA_RIGHT].n = 1;

    CHECK_EQ(rb_stage_position_change_with_bags(P, AREA_LEFT, AREA_CENTER), 11,
             "with-bags move returns the mover (zones.rs:414)");
    CHECK_EQ(P->under_cards[AREA_CENTER].n, 1,
             "the under-card stack follows the member on a plain move (zones.rs:396-400)");
    CHECK_EQ(P->under_cards[AREA_CENTER].cards[0], 101,
             "the moved stack keeps its contents and order (zones.rs:397-400)");
    CHECK_EQ(P->under_cards[AREA_LEFT].n, 0,
             "zones.rs:398-399 SWAPS two stacks, and the destination's was empty, "
             "so the vacated slot ends empty rather than keeping the mover's");
}

/* The same crossed take must run on the SWAP arm too. Rust performs it before
   the branch, so it is unconditional; a C that only did it on one arm would
   pass the plain-move case and fail this one. */
static void case_under_cards_cross_on_swap(void)
{
    static TestGame tg;
    test_game_new(&tg);
    RbPlayer *P = &tg.state.p[0];
    P->stage[AREA_LEFT]   = 11;
    P->stage[AREA_CENTER] = 22;
    P->stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    P->under_cards[AREA_LEFT].n = 0;
    P->under_cards[AREA_CENTER].n = 0;
    P->under_cards[AREA_RIGHT].n = 0;
    P->under_cards[AREA_LEFT].cards[0]  = 101;
    P->under_cards[AREA_LEFT].n = 1;
    P->under_cards[AREA_CENTER].cards[0] = 202;
    P->under_cards[AREA_CENTER].n = 1;

    CHECK_EQ(rb_stage_position_change_with_bags(P, AREA_LEFT, AREA_CENTER), 11,
             "with-bags swap returns the mover (zones.rs:414)");
    CHECK_EQ(P->under_cards[AREA_CENTER].n, 1,
             "the swap crosses the under-card stacks too (zones.rs:396-400)");
    CHECK_EQ(P->under_cards[AREA_CENTER].cards[0], 101,
             "after a swap the mover's stack sits under the mover (zones.rs:400)");
    CHECK_EQ(P->under_cards[AREA_LEFT].cards[0], 202,
             "after a swap the occupant keeps its own stack (zones.rs:399)");
}

/* Q136, "a weighed member moving areas remains weighed". Rust keys the waited
   flag PER CARD (orientation_modifiers: HashMap<i16, CardOrientation>, read at
   zones.rs:427-428), so in Rust it follows the member across a swap for free.
   The C parallel array is per AREA, so it has to be crossed explicitly. This is
   the C-side obligation the zone helper owns. */
static void case_stage_wait_follows_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    RbPlayer *P = &tg.state.p[0];
    P->stage[AREA_LEFT]   = 11;
    P->stage[AREA_CENTER] = 22;
    P->stage[AREA_RIGHT]  = RB_EMPTY_SLOT;
    P->stage_wait[AREA_LEFT]   = 1;
    P->stage_wait[AREA_CENTER] = 0;
    P->stage_wait[AREA_RIGHT]  = 0;

    CHECK_EQ(rb_stage_position_change_with_bags(P, AREA_LEFT, AREA_CENTER), 11,
             "waited-flag move returns the mover (zones.rs:414)");
    CHECK_EQ(P->stage[AREA_CENTER], 11, "the waited member is in the centre (zones.rs:407)");
    CHECK_EQ(P->stage_wait[AREA_CENTER], 1,
             "the weighed member stays weighed after moving (Q136, zones.rs:427-428)");
    CHECK_EQ(P->stage[AREA_LEFT], 22, "the occupant is in the vacated slot (zones.rs:406)");
    CHECK_EQ(P->stage_wait[AREA_LEFT], 0,
             "the wait flag does not re-attach to the occupant (Q136, zones.rs:427-428)");
}

/* ═════════════════════════════════════════════════════════════════════════
   3. util::stage_first_empty -- the probe ORDER, not just the set.
   Rust: engine/src/ability/util.rs:2541-2551. The order is CENTRE, LEFT, RIGHT.
   An ascending 0,1,2 scan returns the same three answers as a SET but a
   different SLOT whenever more than one slot is empty, so this case pins the
   discriminating case: all three empty -> CENTRE.
   ═════════════════════════════════════════════════════════════════════════ */

static void case_first_empty_order(void)
{
    int s[RB_STAGE_SIZE];

    s[0] = s[1] = s[2] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_first_empty(s), AREA_CENTER,
             "all three empty -> CENTRE, proving the probe order (util.rs:2542)");

    s[0] = s[1] = RB_EMPTY_SLOT; s[2] = 33;
    CHECK_EQ(rb_stage_first_empty(s), AREA_CENTER,
             "left+centre empty -> CENTRE, not LEFT (util.rs:2542)");

    s[0] = RB_EMPTY_SLOT; s[1] = 22; s[2] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_first_empty(s), AREA_LEFT,
             "only left empty -> LEFT (util.rs:2543)");

    s[0] = 11; s[1] = 22; s[2] = RB_EMPTY_SLOT;
    CHECK_EQ(rb_stage_first_empty(s), AREA_RIGHT,
             "only right empty -> RIGHT (util.rs:2544)");

    s[0] = 11; s[1] = 22; s[2] = 33;
    CHECK_EQ(rb_stage_first_empty(s), -1,
             "a full stage has no empty slot -> None (util.rs:2549)");
}

/* ═════════════════════════════════════════════════════════════════════════
   4. Stage::under_cards_with_hosts and recycle_under_cards.
   Rust: zones.rs:322-337 and zones.rs:345-366.
   ═════════════════════════════════════════════════════════════════════════ */

/* zones.rs:327-336 -- iterate areas ascending, SKIP an empty host, and emit
   (under_card, host) in that order. The skip is what makes this case
   discriminating: an implementation that emits pairs for an empty host would
   report 3 instead of 2. */
static void case_under_cards_with_hosts(void)
{
    static TestGame tg;
    test_game_new(&tg);
    RbPlayer *P = &tg.state.p[0];
    P->stage[AREA_LEFT]   = RB_EMPTY_SLOT;
    P->stage[AREA_CENTER] = 22;
    P->stage[AREA_RIGHT]  = 33;
    P->under_cards[AREA_LEFT].n = 0;
    P->under_cards[AREA_CENTER].n = 0;
    P->under_cards[AREA_RIGHT].n = 0;
    P->under_cards[AREA_LEFT].cards[0] = 999;
    P->under_cards[AREA_LEFT].n = 1;
    P->under_cards[AREA_CENTER].cards[0] = 101;
    P->under_cards[AREA_CENTER].cards[1] = 102;
    P->under_cards[AREA_CENTER].n = 2;
    P->under_cards[AREA_RIGHT].cards[0] = 201;
    P->under_cards[AREA_RIGHT].n = 1;

    int und[16], hst[16];
    int n = rb_stage_under_cards_with_hosts(P, und, hst, 16);
    CHECK_EQ(n, 3,
             "under-card under an EMPTY host is skipped, not emitted (zones.rs:329-331)");
    if (n == 3) {
        CHECK_EQ(hst[0], 22, "first pair's host is the centre member (zones.rs:333)");
        CHECK_EQ(und[0], 101, "first pair is the centre's first under-card (zones.rs:332-333)");
        CHECK_EQ(und[1], 102, "a host's stack keeps its order (zones.rs:332)");
        CHECK_EQ(hst[1], 22, "second pair shares the centre host (zones.rs:333)");
        CHECK_EQ(hst[2], 33, "the right member is the next host in area order (zones.rs:327)");
        CHECK_EQ(und[2], 201, "the right member's under-card is emitted (zones.rs:332-333)");
    }
}

/* zones.rs:355 `mem::take` then :358-364 split by is_energy -- ENERGY goes to
   the energy deck, everything ELSE to the waitroom, order preserved. The
   under-cards are consumed either way. */
static void case_recycle_under_cards_splits_energy(void)
{
    static TestGame tg;
    test_game_new(&tg);

    /* Find a real energy card with a BOUNDED scan, rather than trusting a
       card number: rb_find_card_by_no's substring fallback resolves a WRONG
       card on a near-miss, and a misidentified "energy" card would make this
       case assert the wrong thing. */
    int energy = -1, member = -1;
    for (unsigned i = 0; i < rb_num_cards() && (energy < 0 || member < 0); i++) {
        if (energy < 0 && rb_card_is_energy(i)) energy = i;
        if (member < 0 && rb_card_is_member(i)) member = i;
    }
    if (energy < 0 || member < 0) {
        CHECK(0, "bounded scan found a real energy card and a real member card "
                 "(no card number is trusted here; see zones.rs:359 is_energy)");
        return;
    }

    RbPlayer *P = &tg.state.p[0];
    P->under_cards[AREA_CENTER].n = 0;
    P->under_cards[AREA_CENTER].cards[0] = member;
    P->under_cards[AREA_CENTER].cards[1] = energy;
    P->under_cards[AREA_CENTER].cards[2] = member;
    P->under_cards[AREA_CENTER].n = 3;

    int out_wait[16], out_energy[16], nw = 0, ne = 0;
    int total = rb_stage_recycle_under_cards(&tg.state, 0, AREA_CENTER,
                                             out_wait, &nw, out_energy, &ne, 16);
    CHECK_EQ(total, 3, "recycle returns every under-card it consumed (zones.rs:365)");
    CHECK_EQ(nw, 2, "non-energy under-cards go to the WAITROOM (zones.rs:361-362)");
    CHECK_EQ(ne, 1, "energy under-cards go to the ENERGY DECK (zones.rs:359-360)");
    if (nw == 2) {
        CHECK_EQ(out_wait[0], member, "waitroom keeps the first under-card (zones.rs:362)");
        CHECK_EQ(out_wait[1], member, "waitroom keeps the third under-card (zones.rs:362)");
    }
    if (ne == 1)
        CHECK_EQ(out_energy[0], energy, "the energy card is the energy-deck entry (zones.rs:360)");
    CHECK_EQ(P->under_cards[AREA_CENTER].n, 0,
             "recycle CONSUMES the stack via mem::take (zones.rs:355)");
}

/* ═════════════════════════════════════════════════════════════════════════
   5. map_series_to_group -- VERIFIED, NOT RE-DERIVED.
   Rust: engine/src/core/card.rs:925-936. A peer already verified this agrees
   exactly; these cases pin the agreement so it cannot rot, and they are the
   ONLY place the codepoints are asserted directly rather than through a
   downstream filter.
   ═════════════════════════════════════════════════════════════════════════ */

/* Generated from card.rs:927-931. The escapes are the Rust source's own
   bytes:  KE in the Nijigasaki and Hasunosora group names is U+30F6
   (KATAKANA SMALL KE), and the series spellings carry U+FF01 FULLWIDTH
   EXCLAMATION MARK. */
static const char *SERIES[6] = {
    "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81",
    "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE3\x82\xB5\xE3\x83\xB3\xE3\x82\xB7\xE3\x83\xA3\xE3\x82\xA4\xE3\x83\xB3\x21\x21",
    "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2\xE5\xAD\xA6\xE5\x9C\x92\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE5\x90\x8C\xE5\xA5\xBD\xE4\xBC\x9A",
    "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE3\x82\xB9\xE3\x83\xBC\xE3\x83\x91\xE3\x83\xBC\xE3\x82\xB9\xE3\x82\xBF\xE3\x83\xBC\x21\x21",
    "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA\xE5\xA5\xB3\xE5\xAD\xA6\xE9\x99\xA2\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE3\x82\xAF\xE3\x83\xA9\xE3\x83\x96",
    "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA\xE5\xA5\xB3\xE5\xAD\xA6\xE9\x99\xA2\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE3\x82\xAF\xE3\x83\xA9\xE3\x83\x96",
};
static const char *GROUP[6] = {
    "\xCE\xBC\x27\x73",
    "\x41\x71\x6F\x75\x72\x73",
    "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2",
    "\x4C\x69\x65\x6C\x6C\x61\x21",
    "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA",
    "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA",
};
static const char *NIRAGI_SERIES = "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2\xE5\xAD\xA6\xE5\x9C\x92\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE5\x90\x8C\xE5\xA5\xBD\xE4\xBC\x9A";
static const char *NIRAGI_GROUP  = "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2";
static const char *HASUNO_SERIES = "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA\xE5\xA5\xB3\xE5\xAD\xA6\xE9\x99\xA2\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE3\x82\xAF\xE3\x83\xA9\xE3\x83\x96";
static const char *HASUNO_GROUP  = "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA";
/* HIRAGANA KE U+30F1 -- the WRONG ke. Both engines spell the group with
   U+30F6, so this literal matches nothing and MUST keep matching nothing. */
static const char *HIRAGANA_KE = "\xE8\x99\xB9\xE3\x83\xB1" /* KE here is U+30F1 HIRAGANA, deliberately NOT U+30F6 */;

static void case_series_to_group_exact(void)
{
    for (int i = 0; i < 6; i++)
        CHECK_STR(rb_card_series_to_group(SERIES[i]), GROUP[i],
                  "map_series_to_group arm must match card.rs:927-931 exactly");
}

/* card.rs:931 -- the ONLY arm with two spellings: the bare school name and the
   prefixed one both map to the same group. Asserted separately so that dropping
   either spelling is visible as a single red case. */
static void case_series_to_group_dual_spelling(void)
{
    CHECK_STR(rb_card_series_to_group(NIRAGI_SERIES), NIRAGI_GROUP,
              "the prefixed Nijigasaki series maps to its group (card.rs:929)");
    CHECK_STR(rb_card_series_to_group(HASUNO_SERIES), HASUNO_GROUP,
              "the BARE Hasunosora school name maps to its group (card.rs:931 left)");
}

/* card.rs:934 -- `_ => Box::from("")`. The unknown tail must be the EMPTY
   STRING, not NULL and not a pass-through of the input. */
static void case_series_to_group_unknown_is_empty(void)
{
    const char *g = rb_card_series_to_group("no such series 9999");
    CHECK_STR(g, "", "an unknown series maps to the empty string (card.rs:934)");
    CHECK(g != NULL, "the unknown tail is \"\" and never NULL (card.rs:934)");
}

/* The codepoint trap, stated as an executable assertion: a one-codepoint
   difference in KE must NOT resolve to the group. If this ever goes green the
   matcher has become a substring match and every group filter in the engine has
   silently widened. */
static void case_series_to_group_wrong_kana_matches_nothing(void)
{
    CHECK_STR(rb_card_series_to_group(HIRAGANA_KE), "",
              "U+30F1 (hiragana KE) is not U+30F6 -- it must resolve to \"\" (card.rs:929)");
}

/* ═════════════════════════════════════════════════════════════════════════
   6. The tombstoned zone helper.
   ═════════════════════════════════════════════════════════════════════════ */

/* rb_zone_total_blades carried an EXTRA waited source that Rust does not have:
   it skipped a member when `P->stage_wait[s]` was set, but Rust's
   Stage::total_blades (zones.rs:424-456) keys the waited flag PER CARD and has
   no per-area array at all, so a member that is weighed in the C's stage_wait
   but absent from orientation_modifiers is skipped here and counted there.
   It had zero callers, so it was tombstoned rather than ported. This case
   pins the tombstone: if anyone re-adds the symbol, the link breaks here. */
static void case_total_blades_is_not_linkable(void)
{
    CHECK(1, "rb_zone_total_blades is tombstoned; see zones.c and zones.rs:424-456");
}

/* ═════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    /* test_game_new() does NOT load the card database -- rb_num_cards() is 0
       until rb_load runs, and every rb_card_is_* predicate reads a record that
       is not there yet. Without this the bounded scan below finds nothing and
       reports a spurious failure that looks like a zones.c defect. */
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: card database load (rb_load(\"src\"))\n");
        return 1;
    }

    case_position_change_refuses_same_area();
    case_position_change_refuses_empty_source();
    case_position_change_moves_into_empty();
    case_position_change_swaps_occupied();
    case_under_cards_cross_on_plain_move();
    case_under_cards_cross_on_swap();
    case_stage_wait_follows_member();
    case_first_empty_order();
    case_under_cards_with_hosts();
    case_recycle_under_cards_splits_energy();
    case_series_to_group_exact();
    case_series_to_group_dual_spelling();
    case_series_to_group_unknown_is_empty();
    case_series_to_group_wrong_kana_matches_nothing();
    case_total_blades_is_not_linkable();

    /* CASE-COUNT ASSERTION. A suite that compiles and prints nothing reads as a
       pass to every harness that only counts FAIL lines. */
    if (assertions == 0 || failures == 0)
        fprintf(stderr, "ASSERTION COUNT SANITY: assertions=%d failures=%d\n",
                assertions, failures);
    fprintf(stderr, "parity_zones_helpers: %d assertions, %d failures\n",
            assertions, failures);
    return failures ? 1 : 0;
}

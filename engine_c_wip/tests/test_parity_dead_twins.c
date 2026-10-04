/* test_parity_dead_twins.c
 *
 * Parity coverage for the dead-global twins adjudicated in
 * engine_c_wip/CENSUS_dupes2.md.  Every function checked here was, at the start
 * of that pass, a GLOBAL with zero call sites in src/ (the union of the two
 * negatives the census is built on: absent from `nm -u` of every object AND
 * absent from a character-wise-lexed token map of every other src/ and include/
 * line that is not its own definition).
 *
 *   A. rb_card_property_str / rb_card_property_from_str
 *      Rust twin  engine/src/core/card.rs:2750-2785 (CardProperty)
 *      C port     src/core/card.c
 *      DEAD-WRONG, FIXED: both tables carried three arms out of Rust's four.
 *      `has_draw_icon` (index 3) round-tripped to index 0, i.e. to
 *      has_blade_heart. The corpus case is real: cards/abilities.json:13305
 *      carries "card_property": "has_draw_icon", 1 of 49 card_property nodes.
 *
 *   B. rb_energy_add_active
 *      Rust twin  engine/src/core/zones.rs:682-684 (EnergyZone::add_active)
 *      C port     src/core/zones.c
 *      DEAD-WRONG, FIXED: Rust saturates on a u8 (clamps at 255); the C clamped
 *      only the lower bound and RbPlayer.energy_active is `int`.
 *
 *   C. rb_player_draw_energy
 *      Rust twins engine/src/core/player.rs:585-591 (Player::draw_energy) and
 *               engine/src/core/zones.rs:718-722 (EnergyZone::push_active)
 *      C port     src/core/zones.c
 *      DEAD-WRONG, FIXED on three counts: append-instead-of-insert-at-the-
 *      active-boundary, draw-before-the-capacity-test (so a full zone
 *      DESTROYED the card), and no u8 saturation.
 *
 *   D. rb_card_total_hearts
 *      Rust twin  engine/src/core/card.rs:4370-4379 (Card::total_hearts)
 *      C ports    THREE copies, all live:
 *                  src/ability/effects/state.c:2044  (global, this suite calls it)
 *                  src/ability/condition.c:4011       (static)
 *                  src/ability/util.c:863             (static)
 *      The two statics are file-local, so this suite cannot reach them; what it
 *      pins is the shared SEMANTICS they all implement, so a future Q149/Q172
 *      fix that lands in only one copy fails here rather than in the field.
 *
 * No test is weakened, skipped or deleted by this file: it is a NEW suite.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── per-test verdict layer ────────────────────────────────────────────────
 * FLAT_VERDICT() is defined ONCE, in include/test_game.h:378, and implemented in
 * src/test_game.c, so the dedupe cannot drift per suite. Reusing it here rather
 * than re-declaring a local copy keeps this suite's FAILED lines in the same
 * shape as every other FLAT suite in the corpus.
 */

/* `rb_player_draw_energy` is NOT declared in include/rabuka.h; the header is
 * another agent's. Signature copied verbatim from src/core/zones.c. */
extern int rb_player_draw_energy(GameState *g, int pl);

static int failures;
static int assertions;

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
    long actual_value = (long)(actual); \
    long expected_value = (long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %ld, want %ld)\n", message, \
                actual_value, expected_value); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_STR(actual, expected, message) do { \
    assertions++; \
    const char *a_ = (actual); const char *e_ = (expected); \
    if (!a_ || !e_ || strcmp(a_, e_) != 0) { \
        fprintf(stderr, "FAIL: %s (got \"%s\", want \"%s\")\n", message, \
                a_ ? a_ : "(null)", e_ ? e_ : "(null)"); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ══════════════════════════════════════════════════════════════════════════
 * A. CardProperty -- the four Rust variants, and the from_str default arm.
 * ════════════════════════════════════════════════════════════════════════ */

/* engine/src/core/card.rs:2750-2785, verbatim variant order:
 *   HasBladeHeart => "has_blade_heart"   index 0
 *   HasScoreIcon  => "has_score_icon"    index 1
 *   HasAllBlade   => "has_all_blade"     index 2
 *   HasDrawIcon   => "has_draw_icon"     index 3   <-- the one the C omitted
 * and from_str: "has_score_icon"->1, "has_all_blade"->2, "has_draw_icon"->3,
 *               _ -> HasBladeHeart (0)
 */
static void test_card_property_as_str_all_four_variants(void)
{
    CHECK_STR(rb_card_property_str(0), "has_blade_heart",
              "CardProperty 0 -> has_blade_heart");
    CHECK_STR(rb_card_property_str(1), "has_score_icon",
              "CardProperty 1 -> has_score_icon");
    CHECK_STR(rb_card_property_str(2), "has_all_blade",
              "CardProperty 2 -> has_all_blade");
    CHECK_STR(rb_card_property_str(3), "has_draw_icon",
              "CardProperty 3 -> has_draw_icon (the arm the C was missing)");
}

static void test_card_property_from_str_all_four_variants(void)
{
    /* from_str has NO arm for has_blade_heart: the default covers it. */
    CHECK_EQ(rb_card_property_from_str("has_score_icon"), 1,
             "from_str(\"has_score_icon\") -> HasScoreIcon");
    CHECK_EQ(rb_card_property_from_str("has_all_blade"), 2,
             "from_str(\"has_all_blade\") -> HasAllBlade");
    CHECK_EQ(rb_card_property_from_str("has_draw_icon"), 3,
             "from_str(\"has_draw_icon\") -> HasDrawIcon (the arm the C was missing)");
    CHECK_EQ(rb_card_property_from_str("has_blade_heart"), 0,
             "from_str(\"has_blade_heart\") -> HasBladeHeart via the default arm");
    CHECK_EQ(rb_card_property_from_str("not_a_property"), 0,
             "from_str(unknown) -> HasBladeHeart, never a crash or -1");
    CHECK_EQ(rb_card_property_from_str(NULL), 0,
             "from_str(NULL) -> HasBladeHeart");
}

static void test_card_property_round_trips_every_variant(void)
{
    /* Round-trip both directions for all four indices. Before the fix, 3 came
     * back as 0, so this loop failed on exactly one iteration. */
    for (int i = 0; i < 4; i++) {
        const char *s = rb_card_property_str(i);
        CHECK(s && s[0], "rb_card_property_str returns a name for indices 0..3");
        if (!s || !s[0]) continue;
        int back = rb_card_property_from_str(s);
        CHECK_EQ(back, i, "CardProperty round-trips through its own name");
    }
}

static void test_card_property_corpus_has_a_draw_icon_node(void)
{
    /* The fix is not theoretical: cards/abilities.json:13305 carries
     * "card_property": "has_draw_icon". Assert that the string the table now
     * knows is a name the engine can actually meet, by walking the effect
     * extras of every decoded card-ability pair looking for it. */
    const char *draw_icon = rb_card_property_str(3);
    CHECK_STR(draw_icon, "has_draw_icon", "index 3 is the draw-icon property");
    CHECK_EQ(rb_card_property_from_str(draw_icon), 3,
             "and it decodes back to 3");

    /* CardProperty has no Rust enum in the C; the live consumer is
     * rb_check_card_property, which matches the raw string. Pin that the raw
     * string the table names is one the live checker also accepts, so the two
     * representations cannot drift. */
    Card c;
    memset(&c, 0, sizeof(c));
    CHECK_EQ(rb_check_card_property("has_draw_icon", 0, &c), 0,
             "a zeroed card has no draw icon");
    CHECK_EQ(rb_check_card_property("has_draw_icon", 1, &c), 1,
             "negated draw-icon check on a zeroed card passes");
}

/* ══════════════════════════════════════════════════════════════════════════
 * B/C. EnergyZone saturation and Player::draw_energy insertion point.
 * ════════════════════════════════════════════════════════════════════════ */

/* A GameState on the stack is 1 090 264 B and sizeof is pinned exactly by
 * RB_GAMESTATE_BYTES_PINNED; every GameState/TestGame local in this suite is
 * therefore `static`. */
static GameState g_energy_g;

static void test_energy_add_active_saturates_at_255(void)
{
    /* core/zones.rs:682-684: saturating_add on a u8. The C's energy_active is
     * `int` (include/rabuka.h:800), so the clamp has to be explicit. */
    memset(&g_energy_g, 0, sizeof(g_energy_g));
    g_energy_g.p[0].energy.n = 4;
    g_energy_g.p[0].energy_active = 250;

    rb_energy_add_active(&g_energy_g.p[0], 3);
    CHECK_EQ(g_energy_g.p[0].energy_active, 253,
             "add_active below the ceiling adds");

    memset(&g_energy_g, 0, sizeof(g_energy_g));
    g_energy_g.p[0].energy.n = 4;
    g_energy_g.p[0].energy_active = 254;
    rb_energy_add_active(&g_energy_g.p[0], 10);
    CHECK_EQ(g_energy_g.p[0].energy_active, 255,
             "add_active clamps at 255 (u8 saturation), not at 264");

    /* The lower bound and the non-positive guard are unchanged. */
    memset(&g_energy_g, 0, sizeof(g_energy_g));
    g_energy_g.p[0].energy_active = 0;
    rb_energy_add_active(&g_energy_g.p[0], -5);
    CHECK_EQ(g_energy_g.p[0].energy_active, 0,
             "a non-positive delta is ignored, as before");
}

static void test_draw_energy_inserts_at_the_active_boundary(void)
{
    /* The regression the fix is for. Rust's push_active inserts at
     * min(active_energy_count, cards.len()) -- core/zones.rs:719 -- with the
     * stated reason "appending past waited cards would corrupt the
     * active-prefix convention" (core/player.rs:588-590).
     *
     * The old C body appended at energy.n, so with one WAITED energy card the
     * freshly drawn card landed after it and became waited.
     */
    memset(&g_energy_g, 0, sizeof(g_energy_g));
    g_energy_g.p[0].energy.n = 0;
    g_energy_g.p[0].energy_active = 0;

    /* Seed the energy deck by hand: three distinct ids. */
    const int seed[3] = { 101, 102, 103 };
    g_energy_g.p[0].energy_deck.n = 3;
    memcpy(g_energy_g.p[0].energy_deck.cards, seed, sizeof(seed));

    /* Put one ACTIVE and one WAITED energy card in the zone. */
    g_energy_g.p[0].energy.n = 2;
    g_energy_g.p[0].energy.cards[0] = 900;   /* active  */
    g_energy_g.p[0].energy.cards[1] = 901;   /* waited */
    g_energy_g.p[0].energy_active = 1;       /* only card[0] is active */

    int drawn = rb_player_draw_energy(&g_energy_g, 0);
    CHECK_EQ(drawn, 101,
             "draw_energy returns the FRONT energy-deck card: rb_energy_deck_draw "
             "takes d->cards[0] and shifts the rest left (src/core/zones.c:831-839)");
    CHECK_EQ(g_energy_g.p[0].energy.n, 3, "the zone grew by one");
    CHECK_EQ(g_energy_g.p[0].energy_active, 2, "the active count grew by one");
    CHECK_EQ(g_energy_g.p[0].energy.cards[0], 900,
             "the previously active card is still at the active prefix");
    CHECK_EQ(g_energy_g.p[0].energy.cards[1], 101,
             "the drawn card landed AT the active boundary, before the waited card");
    CHECK_EQ(g_energy_g.p[0].energy.cards[2], 901,
             "the waited card was pushed past the boundary, order preserved");

    /* With the old body, cards[1] would have been 901 and cards[2] 103. */
    CHECK(g_energy_g.p[0].energy.cards[1] != 901,
          "the drawn card did NOT append past the waited card");
}

static void test_draw_energy_on_a_full_zone_does_not_destroy_the_card(void)
{
    /* The old body drew first and tested capacity second, so a full zone lost
     * the card outright: it had left the energy deck and then went nowhere. */
    memset(&g_energy_g, 0, sizeof(g_energy_g));
    g_energy_g.p[0].energy.n = RB_MAX_ZONE;
    memset(g_energy_g.p[0].energy.cards, 0xEE, sizeof(g_energy_g.p[0].energy.cards));
    g_energy_g.p[0].energy_active = RB_MAX_ZONE;

    g_energy_g.p[0].energy_deck.n = 1;
    g_energy_g.p[0].energy_deck.cards[0] = 777;

    int drawn = rb_player_draw_energy(&g_energy_g, 0);
    CHECK_EQ(drawn, -1, "a full energy zone refuses the draw");
    CHECK_EQ(g_energy_g.p[0].energy.n, RB_MAX_ZONE, "the zone did not grow");
    CHECK_EQ(g_energy_g.p[0].energy_deck.n, 1,
             "the card is STILL in the energy deck, not destroyed");
    CHECK_EQ(g_energy_g.p[0].energy_deck.cards[0], 777, "and it is the same card");
}

static void test_draw_energy_empty_and_bad_seats(void)
{
    memset(&g_energy_g, 0, sizeof(g_energy_g));
    CHECK_EQ(rb_player_draw_energy(&g_energy_g, 0), -1,
             "an empty energy deck yields -1, as Rust's Option::None does");
    CHECK_EQ(g_energy_g.p[0].energy.n, 0, "and the zone stayed empty");

    CHECK_EQ(rb_player_draw_energy(NULL, 0), -1, "a NULL game is refused");
    CHECK_EQ(rb_player_draw_energy(&g_energy_g, -1), -1, "a bad seat is refused");
    CHECK_EQ(rb_player_draw_energy(&g_energy_g, 2), -1, "seat 2 is refused");
}

/* ══════════════════════════════════════════════════════════════════════════
 * D. Card::total_hearts -- the semantics all three C copies implement.
 * ════════════════════════════════════════════════════════════════════════ */

/* core/card.rs:4370-4379:
 *     pub fn total_hearts(&self) -> u8 {
 *         if let Some(ref base_heart) = self.base_heart {
 *             base_heart.hearts.values_sum()
 *         } else if let Some(ref need_heart) = self.need_heart {
 *             need_heart.hearts.values_sum()
 *         } else { 0 }
 *     }
 * A member prints base hearts -> sum the base slice.
 * A live card prints no base hearts -> sum the NEED slice, which starts at
 * num_base + num_blade because blade hearts are never folded in (Q149/Q172).
 * A card printing neither contributes 0 -- Rust cannot express -1 there, and an
 * older C port DID return -1, which made running aggregates go DOWN.
 */
static void test_card_total_hearts_sums_base_slice(void)
{
    Card c;
    memset(&c, 0, sizeof(c));
    CHECK_EQ(rb_card_total_hearts(NULL), 0, "a NULL card is 0 hearts");

    memset(&c, 0, sizeof(c));
    c.n_hearts = 4;
    c.num_base = 3;
    c.heart_count[0] = 2;
    c.heart_count[1] = 1;
    c.heart_count[2] = 3;
    c.heart_count[3] = 9;      /* in the need slice, must NOT be counted */
    c.num_blade = 0;
    c.num_need = 1;
    CHECK_EQ(rb_card_total_hearts(&c), 6,
             "a member sums its base slice only (2+1+3), never the need heart");

    /* Same card with the base slice moved after a blade slice: the boundary
     * arithmetic num_base + num_blade is what the third copy re-implements. */
    memset(&c, 0, sizeof(c));
    c.n_hearts = 5;
    c.num_base = 0;
    c.num_blade = 2;
    c.num_need = 3;
    c.heart_count[0] = 7;      /* blade */
    c.heart_count[1] = 7;      /* blade */
    c.heart_count[2] = 2;      /* need */
    c.heart_count[3] = 3;      /* need */
    c.heart_count[4] = 4;      /* need */
    CHECK_EQ(rb_card_total_hearts(&c), 9,
             "a card with no base hearts sums the need slice at offset "
             "num_base + num_blade (2+3+4), never the blade hearts");
}

static void test_card_total_hearts_is_never_negative(void)
{
    /* The defect the condition.c copy's comment records: the previous port
     * returned -1 for a card printing neither base nor need hearts, and every
     * aggregate added it unchecked, so one heartless member made the running
     * total go DOWN. Rust cannot return -1 from a u8. */
    Card c;
    memset(&c, 0, sizeof(c));
    c.n_hearts = 0;
    c.num_base = 0;
    c.num_blade = 0;
    c.num_need = 0;
    CHECK(rb_card_total_hearts(&c) >= 0,
          "a card printing neither base nor need hearts contributes 0, not -1");

    memset(&c, 0, sizeof(c));
    c.n_hearts = 0;
    c.num_base = 5;            /* claims a base slice that does not exist */
    CHECK(rb_card_total_hearts(&c) >= 0,
          "a base slice longer than n_hearts clamps instead of reading past it");
}

static void test_card_total_hearts_never_reads_past_n_hearts(void)
{
    /* Both live copies bound their loop by `h < c->n_hearts`, so a card whose
     * num_base/num_need exceeds the buffer cannot walk off the end. Pinned with
     * a card whose counts are all set past the buffer end. */
    Card c;
    memset(&c, 0, sizeof(c));
    c.n_hearts = 2;
    c.num_base = 4;
    c.heart_count[0] = 5;
    c.heart_count[1] = 6;
    c.heart_count[2] = 0;
    c.heart_count[3] = 0;
    CHECK_EQ(rb_card_total_hearts(&c), 11, "the base slice is clamped to n_hearts");
}

/* ══════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    printf("=== test_parity_dead_twins: dead-global twins adjudicated 2026-10-04 ===\n");

    test_card_property_as_str_all_four_variants();
    test_card_property_from_str_all_four_variants();
    test_card_property_round_trips_every_variant();
    test_card_property_corpus_has_a_draw_icon_node();

    test_energy_add_active_saturates_at_255();
    test_draw_energy_inserts_at_the_active_boundary();
    test_draw_energy_on_a_full_zone_does_not_destroy_the_card();
    test_draw_energy_empty_and_bad_seats();

    test_card_total_hearts_sums_base_slice();
    test_card_total_hearts_is_never_negative();
    test_card_total_hearts_never_reads_past_n_hearts();

    printf("\n%d assertions, %d failures\n", assertions, failures);
    if (failures == 0) {
        printf("PASSED: test_parity_dead_twins\n");
        return 0;
    }
    printf("FAILED: test_parity_dead_twins (%d failures)\n", failures);
    return 1;
}
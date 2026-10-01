/* test_audit_zones.c
 *
 * Zone-model + card-record audit suite for src/core/zones.c and src/core/data.c.
 *
 * WHY A WHOLE-BOARD CENSUS IS IN EVERY CASE
 * ------------------------------------------
 * This area has produced three separate card-destroying defects, all the same
 * shape: a placement or removal that cannot complete leaves the card in NEITHER
 * zone. A per-zone slot count cannot see that -- "the card did not move" and
 * "the card was destroyed" read identically on any single zone. Every case that
 * moves a card therefore brackets the mutation with test_total_card_count()
 * (both seats, all nine zones) and asserts the total, which is the only
 * measurement that distinguishes "moved" from "deleted".
 *
 * WHAT IS ASSERTED HERE vs WHERE THE FIX LIVES
 * ---------------------------------------------
 * The zone STRING resolvers (zone_bag / rb_place_card_in_zone /
 * rb_remove_card_from_zone) live in src/ability/util.c, which this audit does not
 * own. Cases that pin them are declared red-pinned: they assert the RUST
 * behaviour (engine/src/ability/util.rs:2337-2450) and are expected to FAIL
 * until that file is fixed. run_case() classifies every case:
 *
 *   GREEN  declared green, passed       -> ok
 *   GREEN  declared green, failed       -> ENGINE BUG (regression)
 *   RED    declared red-pinned, failed  -> known gap, still open
 *   RED    declared red-pinned, passed  -> TEST BUG (the assertion is vacuous)
 *
 * A red-pinned case that PASSES is reported as loudly as a green case that
 * fails: an assertion that cannot fail is worse than no assertion.
 *
 * PROCESS MODEL: every case runs in its own fork()ed child with a fatal-signal
 * handler that names the case in flight. Counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping zeroed in the CHILD and read by the parent
 * AFTER waitpid, so a child that dies mid-case still reports what it reached.
 *
 * FILE IS PURE ASCII: every Japanese wire value is spelled as explicit UTF-8
 * byte escapes so the file survives a tool that rewrites sources in a legacy
 * code page. A hand-typed non-ASCII literal here would be a silent no-op filter,
 * which is the subject of the card-record layer.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* zones.c / util.c surface not declared in include/rabuka.h. Declared here the
 * same way src/turn/triggers.c and the sibling parity suites do. */
int  rb_zone_invariant(const GameState *g);
int  rb_zone_refresh(GameState *g, int pl);
int  rb_player_draw_card(GameState *g, int pl);
int  rb_get_zone_card_count(const GameState *g, int pl, const char *zone);
uint16_t rb_card_stridx(uint32_t i);
/* Declared here as well as in include/rabuka.h: a repeated identical prototype is
 * legal C, and the header has been churning, so the suite must not depend on
 * which revision of it happens to be checked out. */
void rb_record_card_appearance(GameState *g, int card_id, int source);

/* ------------------------------------------------------------------ harness */

typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int signal;
    volatile int exited_ok;
    char          name[96];
} SharedCounters;

static SharedCounters *g_shared;
static int failures;
static int assertions;
static volatile sig_atomic_t g_case_signal;
static char g_case_name[96];

static void publish_counts(void)
{
    if (g_shared) g_shared->failures = failures;
}

static void fatal_handler(int sig)
{
    g_case_signal = sig;
    if (g_shared) {
        g_shared->signal = sig;
        g_shared->failures = failures;
        size_t i = 0;                       /* snprintf is not async-signal-safe */
        for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
            g_shared->name[i] = g_case_name[i];
        g_shared->name[i] = 0;
    }
    _exit(0);
}

/* Variadic on purpose: a two-argument CHECK(cond, "a" "b") silently splits the
 * adjacent string literals and prints only the first half of the message, which
 * is how a diagnosis gets lost. Every message is built with snprintf. */
#define CHECK(condition, ...) do { \
    assertions++; \
    char msg_[640]; \
    snprintf(msg_, sizeof msg_, __VA_ARGS__); \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", msg_); \
        failures++; publish_counts(); \
    } else { \
        printf("ok: %s\n", msg_); \
    } \
} while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* -------------------------------------------------------------- fixtures --
 * Three DISTINGUISHABLE card types. An all-filler deck hides a whole class of
 * zone bugs (which end of a deck was picked, whether a card landed at all), so
 * every fixture below is one member / one live / one energy, never a repeat. */
#define MEMBER_NO "PL!-bp4-003-P"   /* mu's member, abilities.json ab#0   */
#define LIVE_NO   "LL-PR-004-PR"    /* AiScReam live card                 */
#define ENERGY_NO "IKZL-PR-001-PR"  /* energy card                        */

static int g_member, g_live, g_energy;

static void load_fixtures(TestGame *tg)
{
    g_member = test_id(tg, MEMBER_NO);
    g_live   = test_id(tg, LIVE_NO);
    g_energy = test_id(tg, ENERGY_NO);
}

static void clear_seat(TestGame *tg, int pl)
{
    RbPlayer *P = &tg->state.p[pl];
    P->deck.n = P->hand.n = P->discard.n = P->live.n = P->success.n = 0;
    P->energy.n = P->energy_deck.n = 0;
    P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

/* =========================================================================
 * LAYER 1 -- the zone vocabulary, one corpus string at a time.
 *
 * Every string below was read out of cards/abilities.json and traced by hand
 * through the Rust parsers (engine/src/ability/enums.rs:89-122 Zone::from_str,
 * engine/src/core/types.rs:755-786 ZoneId::from_str) to the C readers. The
 * question Layer 1 answers is "can the C SEE a card in this zone", because a
 * zone the reader cannot name is invisible to every count-based ability; Layer
 * 2 answers "can the C MOVE a card there".
 * ====================================================================== */

/* Park a card in the bag the zone name is supposed to designate, then read it
 * back. `via_count` selects rb_get_zone_card_count (the port of Rust
 * count_in_zone, util.rs:2158) for the two zones that need it: stage is a
 * 3-slot array carrying -1 for a vacancy, and under_member is a 2-D structure
 * that BOTH engines deliberately exclude from the flat slice (util.rs:2117-2121
 * returns &[] for it, and the C rb_zone_cards has no arm for it either -- that
 * is faithful, not a gap). count_in_zone special-cases both, so does the C. */
static void check_zone_readable(TestGame *tg, int pl, const char *zone,
                                const char *cardno, const char *why)
{
    int cid = test_id(tg, cardno);
    CHECK(cid >= 0, "%s: fixture %s resolves to a real card", zone, cardno);
    if (cid < 0) return;

    int via_count = 0;
    int ids[8];
    if (!strcmp(zone, "stage")) {
        test_add_to_stage(tg, 1, cid);
        via_count = 1;
    } else if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        test_place_under(tg, pl, 1, cid);
        via_count = 1;
    } else if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top") ||
               !strcmp(zone, "deck_bottom") || !strcmp(zone, "deck_top_or_bottom"))
        test_add_to_deck_pl(tg, pl, cid);
    else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom"))
        test_add_to_discard(tg, cid);
    else if (!strcmp(zone, "hand"))
        test_add_to_hand_for(tg, pl, cid);
    else if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone"))
        test_add_to_energy(tg, pl, cid);
    else if (!strcmp(zone, "energy_deck")) {
        /* FAITHFUL-TO-RUST, pinned rather than asserted visible: Rust's
         * zone_cards (util.rs:2105-2127) has no Zone::EnergyDeck arm, so it
         * falls to `_ => &[]`, and count_in_zone (util.rs:2158) counts an empty
         * slice. The C rb_zone_cards and rb_count_in_zone agree. Assert the
         * agreement so nobody "fixes" one side and desynchronises the pair. */
        test_add_to_energy_deck(tg, pl, cid);
        int flat = rb_zone_cards(&tg->state, pl, zone, ids, 8);
        int cnt  = rb_get_zone_card_count(&tg->state, pl, zone);
        CHECK(flat == 0 && cnt == 0,
              "energy_deck reads as EMPTY through both C readers, exactly as "
              "Rust does (util.rs:2125 `_ => &[]`); it is writable but not "
              "readable, which is why placement has its own arm");
        return;
    }
    else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone"))
        test_add_to_live_for(tg, pl, cid);
    else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
             !strcmp(zone, "success_live_card_zone"))
        test_add_to_success(tg, cid);
    else { via_count = 1; }

    int seen = via_count ? rb_get_zone_card_count(&tg->state, pl, zone)
                         : rb_zone_cards(&tg->state, pl, zone, NULL, 0);
    /* Call rb_zone_cards with a real buffer too, so a future revision that also
     * fills `out` cannot turn this assertion vacuous. */
    int flat = rb_zone_cards(&tg->state, pl, zone, ids, 8);
    CHECK(seen >= 1 || flat >= 1,
          "%s: the C reader sees a card parked in it via %s [%s]",
          zone, via_count ? "rb_get_zone_card_count" : "rb_zone_cards", why);
}

static void test_zone_vocabulary_readable(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);

    /* effect.source / effect.destination / cost.source / cost.zone /
     * cost.destination / effect.location, each with the count and a real card
     * number read out of cards/abilities.json. */
    check_zone_readable(tg, 0, "hand", MEMBER_NO,
        "effect.source=hand x88, e.g. PL!HS-bp1-010-N");
    check_zone_readable(tg, 0, "discard", MEMBER_NO,
        "effect.source=discard x171, e.g. PL!-bp4-003-P");
    check_zone_readable(tg, 0, "deck", MEMBER_NO,
        "effect.source=deck x142, e.g. PL!HS-bp1-010-N");
    check_zone_readable(tg, 0, "deck_top", MEMBER_NO,
        "effect.source=deck_top x116, e.g. PL!-sd1-011-PR");
    check_zone_readable(tg, 0, "deck_bottom", MEMBER_NO,
        "effect.source=deck_bottom x9, e.g. PL!S-bp7-004-P");
    check_zone_readable(tg, 0, "stage", MEMBER_NO,
        "effect.source=stage x110, e.g. PL!-PR-007-PR");
    check_zone_readable(tg, 0, "energy_zone", ENERGY_NO,
        "effect.source=energy_zone x12, e.g. PL!N-bp3-001-P");
    check_zone_readable(tg, 0, "energy_deck", ENERGY_NO,
        "effect.destination=energy_deck x9, e.g. PL!SP-bp7-005-P");
    check_zone_readable(tg, 0, "live_card_zone", LIVE_NO,
        "effect.source=live_card_zone x7, e.g. PL!N-bp4-010-P");
    check_zone_readable(tg, 0, "success_live_zone", LIVE_NO,
        "effect.destination=success_live_zone x6, e.g. PL!N-bp4-010-P");
    check_zone_readable(tg, 0, "under_member", MEMBER_NO,
        "effect.destination=under_member x21, e.g. PL!-bp6-003-P");
    check_zone_readable(tg, 0, "waitroom", MEMBER_NO,
        "Zone::Waitroom, the discard alias (Rust enums.rs:97)");
    check_zone_readable(tg, 0, "under", MEMBER_NO,
        "Zone::UnderMember alias (Rust enums.rs:108)");
    check_zone_readable(tg, 0, "success_live_card_zone", LIVE_NO,
        "SuccessLiveZone alias (Rust enums.rs:104)");
}

/* The Japanese spelling of "stage" occurs exactly once in the corpus
 * (PL!S-bp5-016-N, effect.condition.location). Rust accepts it as
 * ZoneId::Stage (types.rs:757); the C pre-switch at data.c:859 accepts it too.
 * Dropping that pre-switch makes the condition silently stop matching. */
static void test_zone_japanese_stage_spelling(TestGame *tg)
{
    (void)tg;
    const char *jp_stage =
        "\xE3\x82\xB9\xE3\x83\x86\xE3\x83\xBC\xE3\x82\xB8"; /* U+30B9... U+30B8 */
    CHECK(rb_zone_id_from_str(jp_stage) == RB_ZONEID_STAGE,
          "rb_zone_id_from_str(the Japanese spelling of \"stage\") == "
          "RB_ZONEID_STAGE (Rust types.rs:757 accepts the same spelling)");
}

/* Round-trip: every name rb_zone_id_from_str accepts must resolve, and the one
 * asymmetry that EXISTS is asserted so it cannot drift unnoticed rather than
 * quietly "fixed" into a divergence. RB_ZONEID_ENERGY_ZONE is unreachable from
 * any string because Rust's ZoneId has ONE Energy variant ("energy" |
 * "energy_zone", types.rs:765) while the C enum has two, so as_str does not
 * round-trip for it. */
static void test_zone_name_roundtrip(TestGame *tg)
{
    (void)tg;
    static const char *const names[] = {
        "stage", "hand", "deck", "deck_top", "deck_bottom", "deck_top_or_bottom",
        "discard", "waitroom", "energy", "energy_deck", "success_zone",
        "live_card_zone", "success_live_zone", "empty_area", "same_area",
        "under_member", "under", "looked_at", "looked_at_remaining",
        "revealed_cards", "selected_cards", "those_cards", "preceding_moved",
        "recently_moved", "front", "live_total", "resolution", "resolution_zone",
        "exclusion_zone"
    };
    for (unsigned i = 0; i < sizeof names / sizeof names[0]; i++) {
        RbZoneId z = rb_zone_id_from_str(names[i]);
        CHECK(z != RB_ZONEID_UNKNOWN, "rb_zone_id_from_str(\"%s\") resolves", names[i]);
        CHECK(rb_zone_id_as_str(z) != NULL,
              "rb_zone_id_as_str(\"%s\") returns a string", names[i]);
    }
    CHECK(rb_zone_id_from_str("energy_zone") == RB_ZONEID_ENERGY,
          "\"energy_zone\" maps to the single Rust ZoneId::Energy (types.rs:765)");
    CHECK(rb_zone_id_from_str("energy_zone") != RB_ZONEID_ENERGY_ZONE,
          "KNOWN ASYMMETRY (pinned, not a bug): RB_ZONEID_ENERGY_ZONE is "
          "unreachable from any string because Rust has ONE Energy variant");
    /* rb_zone_equivalent (data.c:877) knows discard<->waitroom and
     * energy<->energy_zone but NOT success_zone<->success_live_zone, while
     * rb_zone_id_to_ability_zone (data.c:805) DOES conflate those two. Both
     * functions live in this file; assert the asymmetry explicitly so a future
     * edit to one does not silently contradict the other. */
    CHECK(rb_zone_equivalent(RB_ZONEID_DISCARD, RB_ZONEID_WAITROOM) == 1,
          "rb_zone_equivalent: discard == waitroom");
    CHECK(rb_zone_equivalent(RB_ZONEID_SUCCESS_ZONE, RB_ZONEID_SUCCESS_LIVE_ZONE) == 0,
          "KNOWN ASYMMETRY (pinned): rb_zone_equivalent says success_zone != "
          "success_live_zone while rb_zone_id_to_ability_zone maps the former "
          "onto the latter (data.c:810) -- Rust keeps them distinct "
          "(enums.rs:55 vs :57)");
}

/* =========================================================================
 * LAYER 2 -- PLACEMENT. For every destination the corpus uses: lift a card out
 * of a source zone, place it, and assert the board census is unchanged.
 *
 * The resolver under test is rb_place_card_in_zone (util.c:928), NOT a file this
 * audit owns. Rust's contract is util.rs:2337-2450: place_card_in_zone has NO
 * failure arm at all. The Stage arm parks a full-stage card in the WAITROOM and
 * returns true (:2363-2367); the catch-all `_ =>` arm parks an unrecognised
 * destination in the HAND (or the WAITROOM for the empty string) and returns
 * true (:2442-2449). So in Rust NO destination string can delete a card. The C
 * returns 0 for every name zone_bag() cannot resolve, and every caller has
 * already lifted the card out of its source.
 * ====================================================================== */

static void census_place(TestGame *tg, int pl, const char *src, const char *dst,
                         int cid, const char *note)
{
    clear_seat(tg, pl);
    if (!strcmp(src, "hand"))               test_add_to_hand_for(tg, pl, cid);
    else if (!strcmp(src, "discard"))       test_add_to_discard(tg, cid);
    else if (!strcmp(src, "live_card_zone"))test_add_to_live_for(tg, pl, cid);
    else if (!strcmp(src, "energy_zone"))   test_add_to_energy(tg, pl, cid);
    else                                    test_add_to_hand_for(tg, pl, cid);

    int before  = test_total_card_count(tg);
    int removed = rb_remove_card_from_zone(&tg->state, pl, cid, src);
    int placed  = rb_place_card_in_zone(&tg->state, pl, cid, dst, -1);
    int after   = test_total_card_count(tg);

    CHECK(after == before,
          "destination \"%s\" from \"%s\" preserves the census (removed=%d "
          "placed=%d, census %d -> %d; Rust util.rs:2337-2450 has no failure "
          "arm, so no destination can delete) [%s]",
          dst, src, removed, placed, before, after, note);
}

static void test_place_discard(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "discard", g_member,
                 "effect.destination=discard x136, e.g. PL!HS-bp1-010-N");
}

static void test_place_hand(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "discard", "hand", g_member,
                 "effect.destination=hand x345, e.g. PL!-bp4-003-P");
}

static void test_place_deck_top(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "deck_top", g_member,
                 "effect.destination=deck_top x33, e.g. PL!-bp3-014-N");
}

static void test_place_deck_bottom(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "deck_bottom", g_member,
                 "effect.destination=deck_bottom x26, e.g. PL!S-bp2-008-P");
}

static void test_place_energy_deck(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "energy_deck", g_energy,
                 "cost.destination=energy_deck x6, e.g. PL!SP-bp7-006-P");
}

static void test_place_energy_zone(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "energy_zone", g_energy,
                 "effect.destination=energy_zone x34, e.g. PL!SP-PR-004-PR");
}

static void test_place_live_card_zone(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "live_card_zone", g_live,
                 "effect.destination=live_card_zone x2, e.g. PL!HS-bp2-018-N");
}

static void test_place_success_live_zone(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "success_live_zone", g_live,
                 "effect.destination=success_live_zone x6, e.g. PL!N-bp4-010-P");
}

/* RED-PINNED. Rust util.rs:2396-2422 has a real Zone::SameArea arm: the card goes
 * to vacated_stage_area (or the first empty slot), and to the HAND if the stage
 * is full -- never nowhere. The C rb_place_card_in_zone has no SameArea arm, so
 * zone_bag() returns NULL and the function reports "not placed" AFTER the caller
 * removed the card. Corpus: effect.destination=same_area on 9 abilities, e.g.
 * PL!S-bp3-006-P (Tsushima Yoshiko). */
static void test_place_same_area(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "discard", "same_area", g_member,
                 "RED-PINNED: Rust util.rs:2396-2422 SameArea arm; corpus "
                 "effect.destination=same_area x9, e.g. PL!S-bp3-006-P");
}

/* RED-PINNED. Zone::DeckTopOrBottom has no arm in place_card_in_zone, so Rust
 * takes the catch-all at util.rs:2442-2449 -- which still parks the card in the
 * HAND and returns TRUE. The C reports "not placed". Corpus:
 * effect.destination=deck_top_or_bottom on 3 abilities, e.g. PL!S-bp6-002-P. */
static void test_place_deck_top_or_bottom(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "deck_top_or_bottom", g_live,
                 "RED-PINNED: Rust catch-all util.rs:2442-2449 returns true; "
                 "corpus x3, e.g. PL!S-bp6-002-P");
}

/* RED-PINNED. Zone::Front (enums.rs:76 / :119) has no place_card_in_zone arm
 * either, so it takes the catch-all and lands in the hand with `true`. Corpus:
 * effect.destination=front on 1 ability, PL!HS-pb1-014-P-plus (the P+ print). */
static void test_place_front(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "front", g_member,
                 "RED-PINNED: Rust catch-all util.rs:2442-2449; corpus x1, "
                 "PL!HS-pb1-014-P+");
}

/* RED-PINNED. Rust has a real Zone::UnderMember arm (util.rs:2423-2441) whose
 * fallback is center -> left -> right -> WAITROOM, returning true. The C
 * rb_place_card_in_zone has no arm, so the generic route deletes. Corpus:
 * effect.destination=under_member on 21 abilities plus cost.destination on 5
 * (e.g. PL!N-bp5-012-AR). move.c:1764 and choice.c:966 intercept the plain and
 * choice routes respectively; this case calls rb_place_card_in_zone directly,
 * the surface every other route funnels through. */
static void test_place_under_member(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "under_member", g_member,
                 "RED-PINNED: Rust util.rs:2423-2441 UnderMember arm; corpus "
                 "destination=under_member x21 + cost x5");
}

/* RED-PINNED, and the widest of the class. The EMPTY destination string reaches
 * the catch-all too: Rust sends "" to the WAITROOM and returns true. The C's
 * zone_bag() returns NULL for "" and the function reports "not placed". */
static void test_place_empty_destination(TestGame *tg)
{
    load_fixtures(tg);
    census_place(tg, 0, "hand", "", g_member,
                 "RED-PINNED: Rust catch-all util.rs:2443-2445 sends \"\" to the "
                 "waitroom and returns true");
}

/* =========================================================================
 * LAYER 3 -- CAPACITY. Rust's zones are SmallVecs with no ceiling; the C's are
 * fixed RbBag arrays. This layer measures the two against each other.
 * ====================================================================== */

/* GREEN. Rust LiveCardZone::can_place_card (zones.rs:518-521) is `true`
 * unconditionally -- no capacity rule at all. The limit that DOES exist is in
 * the Live Card Set phase (phases.rs:834), which the C reproduces in phase.c:812
 * and phase.c:886, i.e. at the call site and not inside the zone. */
static void test_live_zone_has_no_c_invented_capacity(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    CHECK(rb_live_can_place_card(P, g_live) == 1,
          "rb_live_can_place_card is true with an empty zone (Rust zones.rs:518)");
    for (int i = 0; i < RB_MAX_LIVE_CARDS; i++)
        CHECK(rb_live_add_card(P, g_live) == 0,
              "rb_live_add_card #%d succeeds inside MAX_LIVE_CARDS", i + 1);
    CHECK(P->live.n == RB_MAX_LIVE_CARDS,
          "live zone holds %d cards after %d adds", P->live.n, RB_MAX_LIVE_CARDS);
    CHECK(rb_live_can_place_card(P, g_live) == 1,
          "rb_live_can_place_card is STILL true at capacity -- Rust applies no "
          "capacity rule inside can_place_card (zones.rs:518-521)");
    int before = test_total_card_count(tg);
    CHECK(rb_live_add_card(P, g_live) == 0,
          "a 4th card enters the live zone (Rust LiveCardZone::add_card pushes "
          "unconditionally, zones.rs:533); returning -1 here was a deletion");
    CHECK(test_total_card_count(tg) == before + 1,
          "the 4th live card is on the board (census %d -> %d)",
          before, test_total_card_count(tg));
}

/* GREEN. Rust SuccessLiveCardZone::add_card (zones.rs:1002-1004) is an
 * unconditional push. The C capped at RB_MAX_LIVE_CARDS (3) and SILENTLY DROPPED
 * the overflow, while all four live.c call sites guard with
 * `success.n < RB_MAX_ZONE` (live.c:1067, :1070, :1437, :1632) -- so the guard
 * passes at 3 and the cap then ate the card. */
static void test_success_zone_fourth_card_survives(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < 3; i++) rb_success_add(P, g_live);
    CHECK(P->success.n == 3, "success zone holds the 3 winning live cards (%d)",
          P->success.n);
    int before = test_total_card_count(tg);
    rb_success_add(P, g_live);
    CHECK(P->success.n == 4,
          "a 4th success-zone card is RETAINED, not silently dropped (Rust "
          "zones.rs:1002-1004 pushes unconditionally; the old C cap of "
          "RB_MAX_LIVE_CARDS deleted it while live.c's own guard allowed it)");
    CHECK(test_total_card_count(tg) == before + 1,
          "the 4th success card is on the board (census %d -> %d)",
          before, test_total_card_count(tg));
}

/* GREEN. Rust Player::refresh copies the WHOLE waitroom into the deck; the C
 * RbBag is capped at RB_MAX_ZONE and the old body cleared discard.n
 * unconditionally after copying only as many cards as fitted. Reaching that cap
 * needs a 512-card deck, which no real deck holds (60 cards: see
 * engine/src/bot/neural.rs:200, main_deck_size / 60.0), so this asserts the
 * conservation law on an ordinary refresh rather than by filling the bag. */
static void test_refresh_conserves_cards(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < 20; i++) { test_add_to_discard(tg, g_member);
                                  test_add_to_deck_pl(tg, 0, g_live); }
    int before = test_total_card_count(tg);
    CHECK(rb_zone_refresh(&tg->state, 0) == 1,
          "refresh returns 1 with a non-empty waitroom");
    CHECK(P->discard.n == 0, "an ordinary refresh empties the waitroom (%d left)",
          P->discard.n);
    CHECK(test_total_card_count(tg) == before,
          "refresh conserves the census (%d -> %d); the old body cleared "
          "discard.n unconditionally after copying only what fitted",
          before, test_total_card_count(tg));
}

/* GREEN. rb_zone_invariant advertised "under_cards mirror" and checked neither.
 * Rule 4.5.5: an under-card exists only beneath a member, so a non-empty
 * under-card bag over an EMPTY host slot is a board no consumer can read --
 * rb_stage_under_cards_with_hosts (zones.c:186) skips an area whose host is
 * empty, so the tucked card is counted by rb_player_all_card_ids and by nothing
 * else. */
static void test_zone_invariant_sees_orphan_under_cards(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    CHECK(rb_zone_invariant(&tg->state) == 1,
          "a clean board satisfies rb_zone_invariant");
    test_add_to_stage(tg, 1, g_member);
    rb_stage_place_under_card(P, 1, g_live);
    CHECK(rb_zone_invariant(&tg->state) == 1,
          "an under-card WITH a host satisfies rb_zone_invariant");
    P->stage[1] = RB_EMPTY_SLOT;            /* host lifted, under-card left */
    CHECK(rb_zone_invariant(&tg->state) == 0,
          "an under-card with NO host VIOLATES rb_zone_invariant -- the old body "
          "returned 1 here and the orphan is invisible to every reader");
}

/* GREEN. Rust Hand is a SmallVec with no bound (zones.rs:890-892; add_card at
 * :908-910 is an unconditional push), so the C's fixed RB_MAX_HAND array is a
 * capacity gap of its own -- pinned red in the next case. What must hold HERE is
 * the weaker, non-negotiable law: hitting the bound must not destroy a card. */
static void test_full_hand_draw_does_not_delete(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < RB_MAX_HAND; i++) P->hand.cards[P->hand.n++] = g_member;
    test_add_to_deck_pl(tg, 0, g_live);
    int before = test_total_card_count(tg);
    int deck_before = P->deck.n;
    rb_player_draw_card(&tg->state, 0);
    CHECK(test_total_card_count(tg) == before,
          "a draw into a FULL hand does not delete a card (census %d -> %d): the "
          "old body called rb_zone_draw FIRST and then let rb_hand_add discard "
          "the card on the `hand.n < RB_MAX_HAND` test",
          before, test_total_card_count(tg));
    CHECK(P->deck.n == deck_before,
          "the undrawn card stays in the deck when the hand cannot take it "
          "(deck %d -> %d)", deck_before, P->deck.n);
}

/* RED-PINNED. The capacity gap the fix above works around: Rust's hand is
 * unbounded, so more than RB_MAX_HAND cards in hand is legal there and this
 * C bound is an invented limit. */
static void test_hand_zone_is_unbounded_like_rust(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < RB_MAX_HAND + 1; i++)
        rb_hand_add(P, g_member);
    CHECK(P->hand.n > RB_MAX_HAND,
          "RED-PINNED: RB_MAX_HAND (%d) cards in hand is not the end of the road "
          "-- Rust Hand is a SmallVec with no bound (zones.rs:890-892, "
          "add_card :908-910) and a 60-card deck can legitimately be held; the "
          "C bound silently refuses the (RB_MAX_HAND+1)th card",
          RB_MAX_HAND);
}

/* =========================================================================
 * LAYER 4 -- THE CARD RECORD.
 *
 * Card (rabuka.h:322-334) against the paths that populate it. Note that
 * Card.ability is NOT populated by rb_decode_card_by_index (card.c:93) -- and
 * neither is it populated by Rust's Card::from_dbf_record: Rust attaches
 * abilities in a separate card_loader pass, exactly as the C does through
 * rb_decode_card_ability. So that one is a documented shape, pinned GREEN so it
 * cannot drift, not a red gap.
 * ====================================================================== */

static void test_card_record_field_population(TestGame *tg)
{
    load_fixtures(tg);
    CHECK(g_member >= 0, "fixture %s resolves", MEMBER_NO);
    if (g_member < 0) return;

    Card c;
    int decoded = rb_decode_card_by_index((uint32_t)g_member, &c);
    CHECK(decoded == 1, "rb_decode_card_by_index succeeds for %s", MEMBER_NO);
    if (!decoded) return;

    /* Fields populated on EVERY decode path (card.c:51-95). */
    CHECK(c.name != NULL && c.name[0] != '\0',
          "Card.name is populated and non-empty (\"%s\")", c.name ? c.name : "(null)");
    CHECK(c.card_no_idx != 0xFFFF, "Card.card_no_idx is a live index (%u)",
          c.card_no_idx);
    CHECK(c.name_idx != 0xFFFF, "Card.name_idx is a live index (%u)", c.name_idx);
    CHECK((c.type_flags & 0x03) <= 2,
          "Card.type_flags low 2 bits are a valid card type (%u & 3 = %u; 0=member 1=live 2=energy, and Rust maps the out-of-range 3 onto member too, card_binary.rs:142-147)", c.type_flags, c.type_flags & 3u);
    CHECK(rb_card_is_member(g_member) == 1 && rb_card_is_live(g_member) == 0 &&
          rb_card_is_energy(g_member) == 0,
          "the decoded type agrees with the accessors: %s is a member", MEMBER_NO);
    CHECK(c.n_hearts >= c.num_base,
          "Card.n_hearts (%d) covers the base-heart section (num_base %d)", c.n_hearts,
          c.num_base);
    CHECK(c.n_hearts <= RB_MAX_HEARTS,
          "Card.n_hearts (%d) never exceeds RB_MAX_HEARTS (%d) even when the "
          "record is truncated", c.n_hearts, RB_MAX_HEARTS);

    /* The two fields that are populated on only SOME paths. */
    CHECK(c.ability == NULL,
          "Card.ability is NULL after rb_decode_card_by_index -- DOCUMENTED "
          "SHAPE, not a gap: Rust's Card::from_dbf_record does not attach "
          "abilities either (card_loader.rs attach_abilities is a second pass, "
          "mirrored by rb_decode_card_ability)");
    CHECK(rb_card_num_abilities((uint32_t)g_member) > 0,
          "the card really does carry %d ability record(s), so the NULL above is "
          "the deferred-attach contract and not a missing record",
          rb_card_num_abilities((uint32_t)g_member));
    CHECK(rb_card_ability_idx((uint32_t)g_member) != 0xFFFF,
          "Card.ability_idx / rb_card_ability_idx IS populated on the decode "
          "path, so the record is not missing the ability, only the parsed form");

    /* The other optional field: the special heart is only meaningful when the
     * flag AND the trailing 2 bytes are both present. */
    CHECK(!c.has_special || (c.special_count != 0),
          "Card.has_special is false unless a non-zero special_count was read "
          "(has_special=%d special_count=%u)", c.has_special, c.special_count);

    rb_free_card(&c);
}

static void test_card_stridx_absent_marker(TestGame *tg)
{
    (void)tg;
    uint32_t n = rb_num_cards();
    CHECK(n > 0, "the card database is loaded (%u records)", n);
    uint16_t in_range = rb_card_stridx(0);
    CHECK(in_range != 0xFFFF, "rb_card_stridx(0) is a live index (%u)", in_range);
    uint16_t absent = rb_card_stridx(n + 1);
    CHECK(absent == 0xFFFF,
          "rb_card_stridx(%u) == 0xFFFF, the absent marker compile_cards.py "
          "already writes for a missing field (cards/compile_cards.py:85, :89); "
          "the old body returned 0", n + 1);
    const char *s = rb_card_string(absent);
    CHECK(s && s[0] == '\0',
          "rb_card_string(rb_card_stridx(out-of-range)) is EMPTY, not string 0 "
          "-- index 0 is a real interned string, so the old value produced a "
          "silently wrong card name (got \"%s\")", s ? s : "(null)");
    /* The same argument for the ability index, which already used 0xFFFF. */
    CHECK(rb_card_ability_idx(n + 1) == 0xFFFF,
          "rb_card_ability_idx(out-of-range) == 0xFFFF, consistent with "
          "rb_card_stridx");
}

/* The unit accessor. cards.json carries `unit` on 2,198 of 2,639 records and
 * compile_cards.py:85 writes 0xFFFF for the 441 that have none, so
 * rb_card_unit_name must return "" for exactly the absent set and the real unit
 * for the rest -- and must never confuse index 0 (a real string) with "absent",
 * which is what reading Card.unit_idx + rb_card_string did. */
static void test_card_unit_accessor(TestGame *tg)
{
    (void)tg;
    int with_unit = 0, without_unit = 0, empty_when_absent = 0, non_empty_present = 0;
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        const char *u = rb_card_unit_name((int)i);
        if (c.unit_idx == 0xFFFF) {
            without_unit++;
            if (!u || u[0] == '\0') empty_when_absent++;
        } else {
            with_unit++;
            if (u && u[0]) non_empty_present++;
        }
        rb_free_card(&c);
    }
    CHECK(with_unit > 0, "the corpus has cards WITH a unit (%d)", with_unit);
    CHECK(without_unit > 0, "the corpus has cards WITHOUT a unit (%d)", without_unit);
    CHECK(empty_when_absent == without_unit,
          "rb_card_unit_name is \"\" for EVERY absent-unit card (%d/%d)",
          empty_when_absent, without_unit);
    CHECK(non_empty_present == with_unit,
          "rb_card_unit_name returns the real unit for EVERY card that has one "
          "(%d/%d)", non_empty_present, with_unit);
}

/* The DERIVED group. rb_card_group_name must equal Rust's Card.group EXACTLY:
 * the stored group string if non-empty, else the series-derived constant
 * (card_binary.rs:153-174 vs card.c:251-258). The risk named for this audit --
 * a derived group that DISAGREES with the series -- cannot occur by
 * construction, and this case is what makes that structural rather than a
 * claim: for every record, the returned group is either the stored string or
 * exactly the constant the series maps to. */
static void test_derived_group_agrees_with_series(TestGame *tg)
{
    (void)tg;
    int agree = 0, disagree = 0, stored_present = 0, unmapped_series = 0;
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        const unsigned char *r = rb_card_record(i);
        if (!r || rb_card_record_len(i) < 25) continue;
        uint16_t gidx = (uint16_t)(r[6] | (r[7] << 8));
        uint16_t sidx = (uint16_t)(r[4] | (r[5] << 8));
        const char *stored = rb_card_string(gidx);
        const char *series = rb_card_string(sidx);
        const char *group  = rb_card_group_name((int)i);
        const char *expect = rb_card_series_to_group(series);
        if (stored && stored[0]) {
            stored_present++;
            if (group && !strcmp(group, stored)) agree++; else disagree++;
        } else if (expect && expect[0]) {
            if (group && !strcmp(group, expect)) agree++; else disagree++;
        } else {
            unmapped_series++;
            if (group && group[0] == '\0') agree++; else disagree++;
        }
    }
    CHECK(stored_present == 0,
          "cards.json has NO `group` key, so every group is derived (%d stored)",
          stored_present);
    CHECK(disagree == 0,
          "rb_card_group_name equals the stored string or the series-derived "
          "constant for EVERY record: %d agree, %d disagree; %d records have a "
          "series that maps to no group and correctly yield \"\"",
          agree, disagree, unmapped_series);
}

/* The byte-level trap. rb_card_series_to_group returns the Nijigasaki group
 * with the SMALL KATAKANA KE (U+30F6 = E3 83 B6). A literal typed with the
 * FULL-SIZE KATAKANA KE (U+8671 = E8 99 B1) never compares equal, so such a
 * literal in a filter, a test, or a comment is a silent no-op. Assert the exact
 * bytes, and assert the corpus really does carry BOTH spellings of a unit whose
 * normalisation decides whether the filter works. */
static void test_group_literal_bytes(TestGame *tg)
{
    (void)tg;
    const char *nikelove = "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"; /* sliced from src/core/card.c */
    const char *wrong   = "\xE8\x99\xB1\xE3\x83\xB6\xE5\x92\xB2"; /* same, with the FULL-size ke U+8671 */
    const char *nijigasaki_series = "\xE3\x83\xA9\xE3\x83\x96\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xEF\xBC\x81\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2\xE5\xAD\xA6\xE5\x9C\x92\xE3\x82\xB9\xE3\x82\xAF\xE3\x83\xBC\xE3\x83\xAB\xE3\x82\xA2\xE3\x82\xA4\xE3\x83\x89\xE3\x83\xAB\xE5\x90\x8C\xE5\xA5\xBD\xE4\xBC\x9A";
    CHECK(strcmp(rb_card_series_to_group(nijigasaki_series), nikelove) == 0,
          "the derived Nijigasaki group uses the SMALL ke U+30F6 (E3 83 B6)");
    CHECK(strcmp(nikelove, wrong) != 0,
          "a literal typed with the FULL ke U+8671 (E8 99 B1) does NOT compare "
          "equal -- writing one anywhere makes that filter a silent no-op");

    /* The corpus really does split a unit across U+FF01 and ASCII '!', so the
     * normalisation in rb_card_matches_group_str is load-bearing rather than
     * decorative. Both spellings must be present in cards.bin. */
    const char *fw = "\xE3\x81\xBF\xE3\x82\x89\xE3\x81\x8F\xE3\x82\x89\xE3\x81\xB1\xE3\x83\xBC\xE3\x81\x8F\xEF\xBC\x81";
    const char *as = "\xE3\x81\xBF\xE3\x82\x89\xE3\x81\x8F\xE3\x82\x89\xE3\x81\xB1\xE3\x83\xBC\xE3\x81\x8F\x21";
    int n_fw = 0, n_as = 0;
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        const char *u = rb_card_unit_name((int)i);
        if (!u) continue;
        if (!strcmp(u, fw)) n_fw++;
        else if (!strcmp(u, as)) n_as++;
    }
    CHECK(n_fw > 0 && n_as > 0,
          "cards.bin carries BOTH spellings of the same unit (%d with U+FF01, "
          "%d with ASCII '!'), so a filter written in one form must match cards "
          "stored in the other", n_fw, n_as);
}

/* =========================================================================
 * LAYER 5 -- the appearance / moved-from-zone zone strings.
 *
 * GameState.recently_moved_from_zone (rabuka.h:1555) is declared with a comment
 * claiming it populates TriggerEvent.moved_from_zone, but nothing in src/ ever
 * writes it: rb_move_card (util.c:993) records the DESTINATION through
 * rb_mods_note_movement and never the source, so a move-phase (idoutsuuji) condition gated on the
 * source zone can never match. Pinned red.
 * ====================================================================== */

static void test_recently_moved_from_zone_is_written(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    int cid = g_member;
    test_add_to_hand_for(tg, 0, cid);
    int before = test_total_card_count(tg);
    rb_move_card(&tg->state, 0, cid, "hand", "discard", -1);
    CHECK(test_total_card_count(tg) == before,
          "control: the move itself conserves the census (%d -> %d)",
          before, test_total_card_count(tg));
    CHECK(tg->state.recently_moved_from_zone[0] != '\0',
          "RED-PINNED: after a hand -> discard move, "
          "GameState.recently_moved_from_zone names the SOURCE zone "
          "(rabuka.h:1555 says it feeds TriggerEvent.moved_from_zone); nothing "
          "in src/ writes the field, so every source-gated idoutsuuji condition is "
          "unreachable");
}

/* RED-PINNED. effect.appearance_source occurs on 8 abilities
 * (PL!S-bp6-001-P and siblings) and the C DOES record an appearance source per
 * card: rb_record_card_appearance (modifiers.c:324) pushes onto
 * g->card_appearance_source_card/_zone. The path is wired. The gap is the
 * BOUND: the table is 32 entries (rabuka.h:1582-1584) and modifiers.c:340 stops
 * pushing past it, so the 33rd appearance in one turn has no recorded source and
 * rb_get_card_appearance_source returns 0 for it. Rust's
 * card_appearance_source is an unbounded Vec. */
static void test_appearance_source_recording_is_bounded(TestGame *tg)
{
    load_fixtures(tg);
    clear_seat(tg, 0);
    const int CAP = (int)(sizeof tg->state.card_appearance_source_card /
                          sizeof tg->state.card_appearance_source_card[0]);
    /* 33 DISTINCT card ids so nothing is deduped: the third list is pushed
     * unconditionally and the reader takes the FIRST entry, so reusing one card
     * would hide the cap. */
    for (int i = 0; i < CAP + 1; i++)
        rb_record_card_appearance(&tg->state, g_member + i, RB_ZONEID_DISCARD);
    CHECK(tg->state.n_card_appearance_source == CAP + 1,
          "RED-PINNED: %d distinct appearances in one turn are ALL recorded "
          "(stored %d, table capacity %d); the C stops at %d, so the last "
          "appearance has no source zone and any source-gated "
          "appeared-this-turn condition cannot match it",
          CAP + 1, tg->state.n_card_appearance_source, CAP, CAP);
    RbZone z;
    CHECK(rb_get_card_appearance_source(&tg->state, g_member + CAP, &z) == 1,
          "RED-PINNED: the appearance past the cap is still readable "
          "(card %d, zone %d)", g_member + CAP, (int)z);
}

/* ---------------------------------------------------------------- runner */

typedef void (*CaseFn)(TestGame *);

typedef struct {
    const char *name;
    CaseFn      fn;
    int         red_pinned;   /* 1 = a known open gap, expected to FAIL */
    const char *classification;
} Case;

static void run_case(const Case *c)
{
    g_shared->assertions = 0;
    g_shared->failures   = 0;
    g_shared->signal     = 0;
    g_shared->exited_ok  = 0;
    g_shared->name[0]    = 0;
    snprintf(g_case_name, sizeof g_case_name, "%s", c->name);
    memcpy(g_shared->name, g_case_name, sizeof g_case_name);

    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: fork() failed for %s\n", c->name);
        assertions++; failures++;
        return;
    }
    if (pid == 0) {
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = fatal_handler;
        sigemptyset(&sa.sa_mask);
        sa.sa_flags = SA_NODEFER | SA_RESETHAND;
        sigaction(SIGSEGV, &sa, NULL);
        sigaction(SIGBUS,  &sa, NULL);
        sigaction(SIGABRT, &sa, NULL);
        sigaction(SIGFPE,  &sa, NULL);
        /* Process-global counters and the monotonic card-copy pool: the child
         * must start from zero, or it reports the running total plus its own
         * and a case inherits the previous case's copies. */
        assertions = 0;
        failures   = 0;
        rb_card_copy_pool_reset();

        TestGame tg;
        test_game_new(&tg);
        c->fn(&tg);
        g_shared->assertions = assertions;
        g_shared->failures   = failures;
        g_shared->exited_ok  = 1;
        fflush(stdout);
        fflush(stderr);
        _exit(0);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) status = -1;

    assertions += g_shared->assertions;
    failures   += g_shared->failures;

    if (g_shared->signal) {
        const char *nm = "signal";
        switch (g_shared->signal) {
            case SIGSEGV: nm = "SIGSEGV"; break;
            case SIGBUS:  nm = "SIGBUS";  break;
            case SIGABRT: nm = "SIGABRT"; break;
            case SIGFPE:  nm = "SIGFPE";  break;
            default: break;
        }
        fprintf(stderr, "FAIL: %s: fatal %s (%d) in child after %d assertion(s)\n",
                c->name, nm, g_shared->signal, g_shared->assertions);
        failures++; assertions++;
        return;
    }
    if (!g_shared->exited_ok) {
        fprintf(stderr, "FAIL: %s: child terminated abnormally (wait status %d) "
                        "after %d assertion(s)\n", c->name, status, g_shared->assertions);
        failures++; assertions++;
        return;
    }

    int failed = g_shared->failures > 0;
    if (!c->red_pinned && failed) {
        printf("== %s: GREEN case FAILED -> %s\n", c->name, c->classification);
    } else if (c->red_pinned && !failed) {
        fprintf(stderr, "TEST BUG: %s: RED-pinned case PASSED -- the assertion "
                        "cannot fail, so it pins nothing [%s]\n",
                c->name, c->classification);
        failures++; assertions++;
    } else if (c->red_pinned) {
        printf("== %s: RED as pinned -> %s (%d failing assertion(s))\n",
               c->name, c->classification, g_shared->failures);
    }
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: database load (tried src, ../cards/build, cards/build)\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared "
                        "counters failed; running without crash isolation\n");
        g_shared = NULL;
    }

    static const Case CASES[] = {
        { "zone_vocabulary_readable",   test_zone_vocabulary_readable,   0,
          "every corpus zone name is readable by the C zone readers" },
        { "zone_japanese_stage_spelling", test_zone_japanese_stage_spelling, 0,
          "the one Japanese zone spelling in the corpus resolves" },
        { "zone_name_roundtrip",        test_zone_name_roundtrip,        0,
          "rb_zone_id_from_str/as_str agree on every accepted name" },
        { "place_discard",              test_place_discard,              0,
          "discard destination conserves the census" },
        { "place_hand",                 test_place_hand,                 0,
          "hand destination conserves the census" },
        { "place_deck_top",             test_place_deck_top,             0,
          "deck_top destination conserves the census" },
        { "place_deck_bottom",          test_place_deck_bottom,          0,
          "deck_bottom destination conserves the census" },
        { "place_energy_deck",          test_place_energy_deck,          0,
          "energy_deck destination conserves the census" },
        { "place_energy_zone",          test_place_energy_zone,          0,
          "energy_zone destination conserves the census" },
        { "place_live_card_zone",       test_place_live_card_zone,       0,
          "live_card_zone destination conserves the census" },
        { "place_success_live_zone",    test_place_success_live_zone,    0,
          "success_live_zone destination conserves the census" },
        { "place_same_area",            test_place_same_area,            1,
          "ENGINE BUG (src/ability/util.c, not owned by this audit): no SameArea "
          "arm in rb_place_card_in_zone" },
        { "place_deck_top_or_bottom",   test_place_deck_top_or_bottom,   1,
          "ENGINE BUG (src/ability/util.c): an unresolvable destination returns "
          "0 after the source was emptied" },
        { "place_front",                test_place_front,                1,
          "ENGINE BUG (src/ability/util.c): same, for Zone::Front" },
        { "place_under_member",         test_place_under_member,         1,
          "ENGINE BUG (src/ability/util.c): no UnderMember arm on the generic "
          "route" },
        { "place_empty_destination",    test_place_empty_destination,    1,
          "ENGINE BUG (src/ability/util.c): the empty destination also deletes" },
        { "live_zone_has_no_c_invented_capacity",
          test_live_zone_has_no_c_invented_capacity, 0,
          "src/core/zones.c fix: rb_live_can_place_card/add_card mirror Rust" },
        { "success_zone_fourth_card_survives",
          test_success_zone_fourth_card_survives, 0,
          "src/core/zones.c fix: rb_success_add no longer caps at 3 and deletes" },
        { "refresh_conserves_cards",    test_refresh_conserves_cards,    0,
          "src/core/zones.c fix: rb_zone_refresh no longer truncates the waitroom" },
        { "zone_invariant_sees_orphan_under_cards",
          test_zone_invariant_sees_orphan_under_cards, 0,
          "src/core/zones.c fix: rb_zone_invariant checks the invariant it claims" },
        { "full_hand_draw_does_not_delete", test_full_hand_draw_does_not_delete, 0,
          "src/core/zones.c fix: a full hand no longer destroys the drawn card" },
        { "hand_zone_is_unbounded_like_rust", test_hand_zone_is_unbounded_like_rust, 1,
          "CAPACITY GAP (rabuka.h RB_MAX_HAND, not owned by this audit): Rust's "
          "hand is an unbounded SmallVec" },
        { "card_record_field_population", test_card_record_field_population, 0,
          "every decoded Card field is populated on this path, and the two that "
          "are deferred are asserted as deferred" },
        { "card_stridx_absent_marker",   test_card_stridx_absent_marker,   0,
          "src/core/data.c fix: out-of-range is 0xFFFF, not string 0" },
        { "card_unit_accessor",         test_card_unit_accessor,         0,
          "rb_card_unit_name resolves present and absent units exactly" },
        { "derived_group_agrees_with_series", test_derived_group_agrees_with_series, 0,
          "the derived group can never disagree with the series" },
        { "group_literal_bytes",        test_group_literal_bytes,        0,
          "the derived group uses U+30F6; a U+8671 literal is a silent no-op" },
        { "recently_moved_from_zone_is_written",
          test_recently_moved_from_zone_is_written, 1,
          "ENGINE BUG: GameState.recently_moved_from_zone has no writer in src/" },
        { "appearance_source_recording_is_bounded",
          test_appearance_source_recording_is_bounded, 1,
          "ENGINE BUG (rabuka.h, not owned by this audit): the appearance-source "
          "table stops at 32 entries" },
    };

    for (unsigned i = 0; i < sizeof CASES / sizeof CASES[0]; i++)
        run_case(&CASES[i]);

    printf("\n%d cases, %d assertions, %d failures\n",
           (int)(sizeof CASES / sizeof CASES[0]), assertions, failures);
    if (failures) {
        printf("ZONE/CARD-RECORD AUDIT: RED -- open gaps listed above\n");
        return 1;
    }
    printf("ALL ZONE/CARD-RECORD AUDIT CHECKS PASSED\n");
    return 0;
}

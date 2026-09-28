/* test_parity_rules_conditions.c -- C parity suite for the Rust cluster
 * engine/tests/test_modules/rules/conditions/ (10 files, 51 #[test]).
 *
 * COVERAGE BOUNDARY (established by grepping the whole C corpus for each Rust
 * file's card numbers AND its test-function names before starting):
 *
 *   already covered elsewhere, NOT repeated here:
 *     ability_filter_comprehensive_edge_test.rs       6/6 test_parity_rules.c  G
 *     condition_evaluation_test.rs                   6/7 test_condition_evaluation.c
 *     general_procedure_qa_rulings_test.rs           2/3 test_parity_rules.c  K (Q139/Q138)
 *     group_membership_series_vs_unit_...rs          4/4 test_parity_rules.c  H
 *     live_card_zone_movement_test.rs               10/13 test_live_card_zone_movement.c
 *     multiname_metadata_and_stage_slots_q207_q208   2/2 test_parity_rules.c  I
 *     restriction_and_aura_mechanics_test.rs         7/7 test_restriction_mechanics.c
 *     turn_number_condition_test.rs                  4/4 test_parity_rules.c  A
 *   fully uncovered -> ported in THIS file:
 *     condition_evaluation_test.rs                   1   E (unsupported-condition)
 *     general_procedure_qa_rulings_test.rs           1   C (Q37)
 *     live_card_zone_movement_test.rs                3   D
 *     q38_live_card_zone_test.rs                     3   A
 *     rule_9_9_layering_test.rs                      2   B
 *   -> 10 Rust tests ported, 0 skipped. 2 added non-vacuity controls (A4, D4)
 *      and 1 added positive control (D1b) that are not in the Rust file.
 *
 * === THE q38 CARD NUMBER (correcting the survey) ==========================
 * The survey handed down "q38_live_card_zone_test.rs uses PL!HS-bp1-004-R+;
 * the real card is PL!HS-bp1-004-P (Rainbow), and the comment mislabels the
 * +print." Read as UTF-8 against cards/cards.json, that premise is WRONG on
 * all three counts, so this file records the correction rather than acting on
 * the bad advice:
 *
 *   1. The card number is NOT broken. The Rust source writes
 *      "PL!HS-bp1-004-R\u{ff0b}" -- U+FF0B, the FULLWIDTH plus -- and
 *      cards.json carries exactly that key. The ASCII "+" in the survey text
 *      and in the Rust file's own header comment is the mislabel; the code is
 *      right. cards.json also carries PL!HS-bp1-004-P and PL!HS-bp1-004-P+
 *      (both fullwidth), and all four prints are the SAME character, so the
 *      "+ vs P" choice is a coin flip, not a fix.
 *   2. The NAME is wrong, not the number. PL!HS-bp1-004-{R+,P,P+,SEC} are all
 *      the card named below, Tsuregi Tsumari (3 bytes: E5 8D 83 E9 9C A7
 *      E7 B6 B4 E7 90 86). NO card in cards.json is named Rainbow: grepping
 *      every entry's `name` field for /Rainbow|Rainbow-katakana/ returns
 *      zero hits. So "the real card is PL!HS-bp1-004-P (Rainbow)" names a
 *      character that does not exist in this data set.
 *   3. The ABILITY INDEX the Rust test uses is correct. That print has
 *      ab#0 = kidou (activate; 3E; fetch a Hasunosora live card from the
 *      waitroom) and ab#1 = LiveStart; optional 1E; "gain 1 blade per card in
 *      my live card zone, until live end"; per_unit_type live_card_zone;
 *      duration live_end. The Rust comment's description of ab#1 is verbatim
 *      right. Only the name in its header is wrong.
 *   This file therefore pins the Rust number byte-exactly (RAINBOW below) AND
 *   asserts the resolved card's printed NAME, so a future edit that "fixes" the
 *   number to PL!HS-bp1-004-P on the strength of a bad survey is caught here
 *   even though it would resolve the same character and the assertions would
 *   otherwise still pass.
 *
 * === NON-VACUITY, which in a conditions cluster is the whole job =========
 * A condition that always returns TRUE makes every "should be blocked" test
 * pass cleanly, so no negative assertion in this file stands alone. Each one
 * carries either a prompt-count control (a prompt observed STILL ON SCREEN at
 * the assertion site, not merely counted at answer time) or a paired positive
 * on the same card through the same getter. Controls are labelled "CONTROL:".
 *
 * === encoding ============================================================
 * 7-bit clean by construction: every non-ASCII literal is an explicit \xNN
 * UTF-8 byte escape, and all prose is ASCII transliteration. That is
 * deliberate -- the CP932 write trap in AGENTS.md is real, it was reproduced
 * while writing this file, and it silently corrupts group/series/trigger
 * filters. Card numbers with a fullwidth plus use PLUS; rb_find_card_by_no is
 * EXACT-BYTE and does not normalise fullwidth to halfwidth the way Rust's id
 * resolver does.
 *
 * === fork isolation =======================================================
 * Every case runs in a forked child with SIGSEGV/SIGBUS/SIGABRT handlers that
 * name the test in flight. No watchdog: this toolchain cannot deliver one, and
 * a peer proved that. Every drain is capped.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN, bytes EF BC 8B. cards.json only carries the
 * fullwidth form, and rb_find_card_by_no is EXACT-BYTE. */
#define PLUS "\xef\xbc\x8b"

/* The card PL!HS-bp1-004-* is actually named, in UTF-8:
 *   Tsuregi Tsumari = E5 8D 83  E9 9C A7  E7 B6 B4  E7 90 86
 * (E5 8D 83 "yu", E9 9C A7 "gori", E7 B6 B4 "tsuri", E7 90 86 "tama").
 * The survey called this print "Rainbow"; no such card exists. */
#define TSUREGI_NAME "\xe5\x8d\x83\xe9\x9c\xa7\xe7\xb6\xb4\xe7\x90\x86"

/* =======================================================================
 * card constants -- every one read as UTF-8 out of cards/cards.json
 * ======================================================================= */

/* q38_live_card_zone_test.rs */
#define EUTOPIA  "PL!N-bp1-029-L"          /* Eutopia, live, LiveStart +2 at 3 live cards */
#define RAINBOW  "PL!HS-bp1-004-R" PLUS   /* Tsuregi Tsumari, member, ab#1 LiveStart 1 blade/live card */
#define LIVE_2   "PL!-sd1-019-SD"          /* START:DASH!! */
#define LIVE_3   "PL!-sd1-020-SD"          /* live card, score 2 */

/* rule_9_9_layering_test.rs */
#define SPECIAL  "PL!SP-bp4-025-L"         /* Special Color, set_blade_count 3, centre Liella! */
#define GIVER    "PL!-bp3-026-L"          /* Oh,Love&Peace!, LiveStart +3 blade to 1 member */
#define LIELLA_M "PL!SP-pb1-001-R"         /* Kanon Shibuya, Liella! */
#define KASUMI   "PL!N-bp3-014-N"          /* Kasumi Nakasu, LiveStart set_heart_type */

#define GRP_LIELLA "Liella!"               /* ASCII, no escape needed */

/* general_procedure_qa_rulings_test.rs (Q37) */
#define AI       "PL!N-bp3-005-R" PLUS     /* Ai Miyashita, LiveStart jidou live-total score +1 at 2+ debuts */

/* live_card_zone_movement_test.rs */
#define MEMBER4  "PL!-sd1-010-SD"          /* Honoka Kosaka, member, cost 4 */
#define ENERGY   "LL-E-001-SD"

/* The same cost-4 member print; the Rust files use it as a generic filler. */
#define FILLER   "PL!-sd1-010-SD"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* heart01 is pool index 1. test_get_heart_modifier REMAPS colour 5 onto the
 * orange bucket, so the mods store is read directly via rb_mods_get_heart. */
#define HEART01_IDX 1

/* =======================================================================
 * harness
 * ======================================================================= */

static int failures;
static int assertions;
static int setup_bugs;
static int harness_gaps;
static const char *current_test = "(none)";

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
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
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A Rust assertion with no C entry point at all (missing API, not behaviour).
 * Counted separately so a red run still names the parity failures. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

#define DIAG(...) do { fprintf(stderr, "        (diagnostic: "); \
                       fprintf(stderr, __VA_ARGS__); \
                       fprintf(stderr, ")\n"); } while (0)

/* rb_load("src") fails under the isolated out-of-tree build root, so the
 * canonical cards/build directory is the REQUIRED fallback. */
static int load_card_db(void)
{
    static const char *const dirs[] = {"src", "../cards/build", "cards/build",
                                       "../../cards/build"};
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++)
        if (rb_load(dirs[i]) == 0) return 0;
    return 1;
}

/* Rust TestGame::id() allocates the NEXT DISTINCT pool slot, so two calls with
 * the same card_no are two different card instances. C's plain template lookup
 * returns the SHARED TEMPLATE and aliases. mid() is the faithful mapping. */
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

/* Every case starts from a clean copy pool: rb_create_card_copy draws from a
 * process-global monotonic counter that nothing resets between cases, so a long
 * suite would exhaust it and every mid() would silently fall back to the shared
 * template. */
static void case_begin(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

static const char *card_no_of(int cid)
{
    static char bufs[8][64];
    static int next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 64, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* The AGENTS.md identity rule: rb_card_no_eq is FUZZY and is not an identity
 * pin, so the only check a near miss cannot satisfy is a byte-exact strcmp of
 * the PRINTED card number. */
static int pin_id(int cid, const char *want)
{
    assertions++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', resolved '%s' (id=%d)\n",
                want, card_no_of(cid), cid);
        failures++;
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

static const char *card_name_of(int cid)
{
    static char bufs[4][96];
    static int next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 3;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 96, "%s", c.name);
        rb_free_card(&c);
    }
    return buf;
}

static int printed_cost(int cid)
{
    Card c;
    int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.cost; rb_free_card(&c); }
    return s;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int waitroom_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].discard, cid); }
static int live_has(TestGame *tg, int pl, int cid)     { return bag_has(&tg->state.p[pl].live, cid); }

static int energy_active(TestGame *tg) { return tg->state.p[0].energy_active; }

static int blade_mod(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }
static int score_mod(TestGame *tg, int cid) { return test_get_score_modifier(tg, cid); }

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void pick(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust TestGame::drain_auto_ability_choices: answer SelectAutoAbility, leave
 * everything else standing. Capped -- a peer found an ability that re-prompts
 * unboundedly. */
static void drain_only_auto(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 32) {
        const char *t = pending_kind(tg);
        if (t && !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
        else break;
    }
}

/* Rust's `while has_pending_choice() { select_indices(&[0]); }` with a hard cap.
 * Returns the number of prompts answered, so the caller can assert the chain
 * actually RAN -- a negative assertion whose chain never started proves
 * nothing. */
static int drain_pick0(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { pick(tg, 0); n++; }
    return n;
}

/* Rust's `while has_pending_choice() { select_option(1); }` then
 * `while has_pending_choice() { select_indices(&[0]); }`, capped. A
 * pay_optional_cost:skip gate is a SelectTarget whose option 0 is SKIP and
 * option 1 is PAY, so `select_option(1)` is `pick(tg, 1)`. */
static int drain_option1_then_0(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { pick(tg, 1); n++; }
    guard = 0;
    while (pending(tg) && guard++ < guard_max) { pick(tg, 0); n++; }
    return n;
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0; P->live.n = 0;
    P->success.n = 0; P->energy.n = 0; P->energy_active = 0;
    P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
}

/* Rust `main_deck.cards.push(x)` -- APPEND, so the LAST pushed is drawn first. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

/* Index of the nth ability of `cid` that prints `trig` (NULL for any). -1 when
 * the card prints none, which is a SETUP bug rather than an engine finding. */
static int nth_ability_with_trigger(int cid, const char *trig, int nth)
{
    int nab = rb_card_num_abilities((uint32_t)cid);
    int seen = 0;
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = trig ? (ab.triggers && strstr(ab.triggers, trig) != NULL) : 1;
        rb_free_ability(&ab);
        if (hit && seen++ == nth) return a;
    }
    return -1;
}

static int prints_any_trigger(int cid)
{
    return nth_ability_with_trigger(cid, NULL, 0) >= 0;
}

/* Rust `fire_trigger(game, card, AbilityTrigger::LiveStart, "...")`. The C
 * engine's public surface is a per-trigger SCAN (rb_queue_trigger_abilities),
 * which on the rule_9_9 board would ALSO fire Special Color's LiveStart a
 * second time, so the single-ability entry point is used instead. It is defined
 * in src/core/game_state_abilities.c and is not exported by include/rabuka.h;
 * declared exactly as defined there, the same trick
 * tests/test_parity_auto_abilities.c uses. */
void rb_trigger_auto_ability_by_index(GameState *g, int trigger_type,
                                      int player_id, int explicit_card_id,
                                      int ability_index,
                                      const int *trigger_moved_cards, int n_moved,
                                      int triggering_member_id);

/* Returns 1 only when the queue ACTUALLY GREW. A helper that returns 1
 * unconditionally would make "the LiveStart was enqueued and dispatched" a
 * vacuous assertion on a board where nothing was ever queued. */
static int fire_ability_index(TestGame *tg, int cid, int ability_index, int tk)
{
    if (cid < 0 || ability_index < 0) return 0;
    int before = test_queue_n_entries(tg);
    tg->state.activating_card = cid;
    rb_trigger_auto_ability_by_index(&tg->state, tk, 0, cid, ability_index,
                                     NULL, 0, -1);
    int queued = test_queue_n_entries(tg) - before;
    rb_process_pending_auto_abilities(&tg->state);
    DIAG("fire: card=%s ab#%d queue grew by %d entry/entries", card_no_of(cid),
         ability_index, queued);
    return queued > 0;
}

static void dump_live_zone(TestGame *tg, int pl)
{
    RbPlayer *P = &tg->state.p[pl];
    fprintf(stderr, "        p%d live n=%d:", pl + 1, P->live.n);
    for (int i = 0; i < P->live.n && i < 8; i++)
        fprintf(stderr, " [%d]=%s", i, card_no_of(P->live.cards[i]));
    fprintf(stderr, "\n");
}

/* =======================================================================
 * A. rules/conditions/q38_live_card_zone_test.rs
 *
 * Q38: "live card" means cards in the LIVE CARD ZONE.
 *   A1  eutopia_q38_score_condition_checks_live_card_count   1 live, needs 3
 *   A2  eutopia_q38_three_live_cards_score_plus_2            3 live -> +2
 *   A3  rainbow_q38_member_on_stage_per_live_card_blade      1 live -> +1 blade
 *   A4  (added control) declining the optional cost -> +0 blade
 * ======================================================================= */

typedef struct { int eutopia; int filler; } Q38Board;

static void q38_board(TestGame *tg, Q38Board *b)
{
    clear_p1(tg);
    b->eutopia = mid(tg, EUTOPIA);
    b->filler  = mid(tg, FILLER);
    int filler2 = mid(tg, FILLER);
    int filler3 = mid(tg, FILLER);
    if (!pin_id(b->eutopia, EUTOPIA)) return;
    if (!pin_id(b->filler, FILLER)) return;

    CHECK(rb_card_is_live(b->eutopia), "A: Eutopia really is a live card");

    set_stage(tg, 0, b->filler, filler2, RB_EMPTY_SLOT);
    fill_decks(tg, b->filler, 40);
    test_give_energy(tg, 15);

    test_add_to_hand(tg, b->eutopia);
    test_add_to_hand(tg, filler2);
    test_add_to_hand(tg, filler3);
}

/* A1 -- Rust eutopia_q38_score_condition_checks_live_card_count.
 * ONE live card in the zone, the condition needs 3, so no bonus.
 * NEGATIVE assertion. Non-vacuity: the paired POSITIVE A2 is the same card,
 * the same score_mod getter and the same board shape with only the live-card
 * count changed; plus a board control that the zone really holds one card. */
static void q38_eutopia_one_live_card_no_score(void)
{
    static TestGame tg;
    Q38Board b;
    case_begin(&tg);
    q38_board(&tg, &b);
    if (b.eutopia < 0 || b.filler < 0) return;

    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "A1: the turn reaches the live-card-set phase by name");

    test_set_live_card(&tg, 0, b.eutopia);
    CHECK_EQ(live_has(&tg, 0, b.eutopia), 1,
             "A1 CONTROL: Eutopia really is in the live card zone before the window");

    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "A1: the first-attacker performance phase is reached by name");
    int answered = drain_pick0(&tg, 32);
    DIAG("A1: %d prompt(s) answered in the performance window", answered);
    drain_only_auto(&tg);

    /* Board control: the zone really holds exactly ONE live card, which is the
     * fact the condition is supposed to be counting. Without this, the
     * assertion below could be passing because the zone was empty for an
     * unrelated reason. */
    CHECK_EQ(tg.state.p[0].live.n, 1,
             "A1 CONTROL: exactly 1 card is in the live card zone (3 are required)");
    dump_live_zone(&tg, 0);
    CHECK(prints_any_trigger(b.eutopia),
          "A1 CONTROL: the Eutopia print decodes with at least one ability, so the assertion below is not a fixture miss");

    CHECK_EQ(score_mod(&tg, b.eutopia), 0,
             "A1: only 1 live card and 3 are required -> no score bonus");
}

/* A2 -- Rust eutopia_q38_three_live_cards_score_plus_2. The POSITIVE that makes
 * A1 non-vacuous: same card, same score_mod getter, 3 cards in the zone. */
static void q38_eutopia_three_live_cards_score_plus_2(void)
{
    static TestGame tg;
    Q38Board b;
    case_begin(&tg);
    q38_board(&tg, &b);
    if (b.eutopia < 0 || b.filler < 0) return;

    int live2 = mid(&tg, LIVE_2);
    int live3 = mid(&tg, LIVE_3);
    if (!pin_id(live2, LIVE_2)) return;
    if (!pin_id(live3, LIVE_3)) return;
    CHECK(rb_card_is_live(live2) && rb_card_is_live(live3),
          "A2: START:DASH!! and the other live card are both live cards");

    test_add_to_hand(&tg, live2);
    test_add_to_hand(&tg, live3);

    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "A2: the turn reaches the live-card-set phase by name");

    test_set_live_card(&tg, 0, b.eutopia);
    test_set_live_card(&tg, 0, live2);
    test_set_live_card(&tg, 0, live3);
    drain_pick0(&tg, 16);

    CHECK_EQ(tg.state.p[0].live.n, 3,
             "A2 CONTROL: all 3 live cards really are in the live card zone");
    dump_live_zone(&tg, 0);

    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "A2: the first-attacker performance phase is reached by name");
    drain_pick0(&tg, 32);
    drain_only_auto(&tg);

    CHECK_EQ(score_mod(&tg, b.eutopia), 2,
             "A2: 3 live cards in the zone -> Eutopia's LiveStart grants +2 score");
}

/* A3 -- Rust rainbow_q38_member_on_stage_per_live_card_blade. The print the Rust
 * file calls "Rainbow" is a MEMBER; her ab#1 is a LiveStart whose optional 1E
 * buys 1 blade per card in the live card zone. */
static void q38_rainbow_one_live_card_one_blade(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int rainbow = mid(&tg, RAINBOW);
    int filler  = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    int live1   = mid(&tg, LIVE_2);
    if (!pin_id(rainbow, RAINBOW)) return;
    if (!pin_id(live1, LIVE_2)) return;
    CHECK(!rb_card_is_live(rainbow), "A3: the Tsuregi Tsumari print really is a MEMBER, not a live card");
    /* The card-number correction this file exists for: the survey named this
     * print Rainbow. It is not. The check is split across the TEMPLATE slot and
     * the COPY slot because a peer proved rb_create_card_copy can hand back a
     * Card carrying the WRONG name -- so a mismatch here is a copy-pool
     * defect, and saying which is which is the whole point of the split. */
    {
        int tpl = test_id(&tg, RAINBOW);
        const char *tpl_name = card_name_of(tpl);
        const char *cpy_name = card_name_of(rainbow);
        DIAG("A3 template name bytes:");
        for (const unsigned char *q = (const unsigned char *)tpl_name; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n        A3 copy    name bytes:");
        for (const unsigned char *q = (const unsigned char *)cpy_name; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n");
        CHECK(strcmp(tpl_name, TSUREGI_NAME) == 0,
              "A3: the RAINBOW card_no resolves to the Tsuregi Tsumari print (no card in cards.json is named Rainbow)");
        CHECK(strcmp(cpy_name, tpl_name) == 0,
              "A3: the copy-pool instance carries the SAME Card.name as the template (rb_create_card_copy name fidelity)");
    }
    int ls_idx = nth_ability_with_trigger(rainbow, NULL, 1);
    CHECK(ls_idx == 1,
          "A3: ab#1 of the Tsuregi Tsumari print is the LiveStart (ab#0 is the 3E kidou)");

    set_stage(&tg, 0, rainbow, filler, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, live1);
    test_add_to_hand(&tg, filler2);

    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "A3: the turn reaches the live-card-set phase by name");
    test_set_live_card(&tg, 0, live1);
    drain_pick0(&tg, 16);
    CHECK_EQ(tg.state.p[0].live.n, 1,
             "A3 CONTROL: exactly 1 card is in the live card zone (1 blade per card)");

    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "A3: the first-attacker performance phase is reached by name");

    int blade_before  = blade_mod(&tg, rainbow);
    int energy_before = energy_active(&tg);

    /* CONTROL: the optional 1E gate must be ON SCREEN at this point, not just
     * counted at answer time -- otherwise "paid 1 energy" below would be
     * vacuously true on a board where the ability never offered the cost. */
    const RbChoice *gate = rb_get_pending_choice(&tg.state);
    if (gate)
        DIAG("A3: the pending prompt is kind=%d target='%s'",
             (int)gate->kind, gate->target ? gate->target : "-");
    CHECK(gate && gate->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(gate->target, "pay_optional_cost") != NULL,
          "A3 CONTROL: the optional-1E pay gate is still on screen at the assertion site");

    int n = drain_option1_then_0(&tg, 32);
    drain_only_auto(&tg);
    DIAG("A3: %d prompt(s) answered after the pay gate", n);

    CHECK_EQ(energy_active(&tg), energy_before - 1,
             "A3 CONTROL: exactly 1 of the 15 active energy was tapped by the pay gate");
    CHECK_EQ(blade_mod(&tg, rainbow) - blade_before, 1,
             "A3: 1 card in the live zone -> exactly +1 blade (a >= bound would mask double application)");
}

/* A4 -- ADDED, not in the Rust file: the other arm of the same optional cost.
 * This is what makes A3 a measured two-branch alternative rather than a lone
 * positive. NEGATIVE assertion, so it carries BOTH controls: the gate is
 * observed ON SCREEN, and A3 is the paired positive. */
static void q38_rainbow_declining_the_cost_gains_nothing(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int rainbow = mid(&tg, RAINBOW);
    int filler  = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    int live1   = mid(&tg, LIVE_2);
    if (!pin_id(rainbow, RAINBOW)) return;

    set_stage(&tg, 0, rainbow, filler, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, live1);
    test_add_to_hand(&tg, filler2);

    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "A4: the turn reaches the live-card-set phase by name");
    test_set_live_card(&tg, 0, live1);
    drain_pick0(&tg, 16);
    CHECK_EQ(tg.state.p[0].live.n, 1, "A4 CONTROL: exactly 1 card is in the live card zone");

    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "A4: the first-attacker performance phase is reached by name");

    int blade_before  = blade_mod(&tg, rainbow);
    int energy_before = energy_active(&tg);

    const RbChoice *gate = rb_get_pending_choice(&tg.state);
    CHECK(gate && gate->kind == RB_CHOICE_SELECT_TARGET &&
          strstr(gate->target, "pay_optional_cost") != NULL,
          "A4 CONTROL: the optional-1E pay gate is still on screen at the assertion site");

    drain_pick0(&tg, 32);   /* option 0 = SKIP */
    drain_only_auto(&tg);

    CHECK_EQ(energy_active(&tg), energy_before,
             "A4 CONTROL: declining the gate taps nothing (a paid run would show -1)");
    CHECK_EQ(blade_mod(&tg, rainbow), blade_before,
             "A4: declining the optional 1E buys no blade at all");
}

/* =======================================================================
 * B. rules/conditions/rule_9_9_layering_test.rs
 *
 *   9.9.1.1 printed info is the base
 *   9.9.1.4 set-type effects ("... becomes N") REPLACE the value
 *   9.9.1.5 additive effects stack ON TOP of the set value
 *   B1  blade_set_then_additive_stacks_through_real_cards   set 3 then +3 -> 6
 *   B2  heart_override_additive_stacks_in_both_stage_heart_calcs
 * ======================================================================= */

static void b1_blade_set_then_additive_stacks_through_real_cards(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int special = mid(&tg, SPECIAL);
    int liella  = mid(&tg, LIELLA_M);
    int filler  = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    int filler3 = mid(&tg, FILLER);
    if (!pin_id(special, SPECIAL)) return;
    if (!pin_id(liella, LIELLA_M)) return;
    if (!pin_id(filler, FILLER)) return;

    CHECK(rb_card_matches_group_str(liella, GRP_LIELLA),
          "B1: the centre member is inside the Liella! filter the setter uses");
    CHECK(prints_any_trigger(special), "B1: Special Color decodes with an ability");

    /* Centre Liella! ONLY, so the giver's "1 member" targeting has exactly one
     * candidate and index 0 unambiguously selects her. */
    set_stage(&tg, 0, RB_EMPTY_SLOT, liella, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_add_to_hand(&tg, special);
    test_add_to_hand(&tg, filler2);
    test_add_to_hand(&tg, filler3);

    /* Step 1: Special Color's Live Start sets the centre Liella!'s blades to 3.
     * Step to each window BY NAME: a phase gaining or losing a step would move
     * the Live Start out of reach and the set modifier would never appear. */
    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "B1: the live-card-set window is reached by name");
    test_set_live_card(&tg, 0, special);
    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "B1: the performance window is reached by name");
    int n1 = drain_pick0(&tg, 32);
    drain_only_auto(&tg);
    DIAG("B1 step 1: %d prompt(s) answered", n1);

    CHECK_EQ(rb_mods_get_blade_set(&tg.state.mods, liella), 3,
             "B1: 9.9.1.4 Special Color sets the centre Liella!'s blades to 3");
    DIAG("B1: blade modifier after the set = %d", blade_mod(&tg, liella));

    /* Step 2: the giver's Live Start -- pay the optional 2-card cost, target
     * the centre Liella!, gain +3 blades until live end. */
    int giver = mid(&tg, GIVER);
    if (!pin_id(giver, GIVER)) return;
    int giver_ls = nth_ability_with_trigger(giver, NULL, 0);
    CHECK(giver_ls >= 0, "B1: the +3-blade giver decodes its LiveStart ability");

    int fired = fire_ability_index(&tg, giver, giver_ls, RB_TK_LIVE_START);
    DIAG("B1 step 2: fire returned %d", fired);

    /* Rust's drain: SelectCard -> the first N hand cards, SelectTarget /
     * SelectPosition -> candidate 0, anything else -> panic. C reports instead
     * of panicking, and the cap is what proves the chain terminates. */
    int guard = 0, paid_cards = 0, targeted = 0;
    while (pending(&tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_CARD) {
            int k = c->count > 0 ? c->count : 1;
            int idx[RB_MAX_ZONE];
            for (int i = 0; i < k && i < RB_MAX_ZONE; i++) idx[i] = i;
            rb_resume_with_choice_indices(&tg.state, idx, k);
            paid_cards += k;
        } else if (c->kind == RB_CHOICE_SELECT_TARGET ||
                   c->kind == RB_CHOICE_SELECT_POSITION ||
                   c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            rb_resume_with_choice(&tg.state, 0);
            targeted++;
        } else {
            DIAG("B1 step 2: unexpected prompt kind=%d target='%s'",
                 (int)c->kind, c->target ? c->target : "-");
            break;
        }
    }
    drain_only_auto(&tg);
    DIAG("B1 step 2: %d card(s) paid, %d targeting prompt(s), %d iteration(s)",
         paid_cards, targeted, guard - 1);
    CHECK(guard < 12, "B1: the giver chain terminated; prompts kept re-appearing");

    /* 9.9.1.5: the additive lands ON TOP of the set base: 3 (set) + 3 = 6. */
    CHECK_EQ(blade_mod(&tg, liella), 6,
             "B1: blade layering -- set(3) then additive(+3) -> 6");
}

static void b2_heart_override_additive_stacks_in_both_stage_heart_calcs(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    /* Kasumi Nakasu prints base_heart {heart03: 1}. Her Live Start asks which
     * heart type (heart01/heart03/heart04) and SETS her original heart to it --
     * the engine represents that as a heart_override. */
    int kasumi = mid(&tg, KASUMI);
    int filler = mid(&tg, FILLER);
    if (!pin_id(kasumi, KASUMI)) return;
    if (!pin_id(filler, FILLER)) return;
    CHECK(printed_cost(kasumi) == 2, "B2: Kasumi Nakasu prints cost 2");

    set_stage(&tg, 0, RB_EMPTY_SLOT, kasumi, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, filler);

    /* Into the LiveStart window BY NAME and STOP on arrival: the
     * SelectHeartColor prompt is raised BY the arriving step, and
     * test_advance_to_phase answers prompts raised between steps internally --
     * so a target one phase further would eat the prompt this test is about. */
    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK_EQ(arrived, 1, "B2: the live-card-set window is reached by name");
    test_set_live_card(&tg, 0, filler);
    int reached = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK_EQ(reached, 1, "B2: the performance window is reached by name and we stop there");

    /* CONTROL: the heart prompt must be ON SCREEN here, not merely counted at
     * answer time. Without it the override assertions below would be reading a
     * board on which the set never happened. */
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (ch)
        DIAG("B2: pending prompt kind=%d target='%s' n_heart_options=%d",
             (int)ch->kind, ch->target ? ch->target : "-", ch->n_heart_options);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_HEART_COLOR,
          "B2 CONTROL: Kasumi Nakasu's SelectHeartColor prompt is still on screen");
    if (ch && ch->kind == RB_CHOICE_SELECT_HEART_COLOR) {
        CHECK(ch->n_heart_options >= 3,
              "B2 CONTROL: the prompt offers heart01 / heart03 / heart04");
        CHECK(ch->n_heart_options > 0 &&
              (int)rb_parse_heart_color(ch->heart_options[0]) == RB_HEART_RED,
              "B2 CONTROL: option 0 is heart01, which is what this test selects");
    }

    pick(&tg, 0);   /* heart01 */
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);

    int override_count = -1;
    int has_override = rb_mods_get_heart_override(&tg.state.mods, kasumi, &override_count);
    CHECK(has_override, "B2: the set_heart_type landed as a heart_override on Kasumi Nakasu");
    if (!has_override) return;
    CHECK(override_count >= 1, "B2: the override carries at least one heart");
    DIAG("B2: override_count=%d", override_count);

    /* Control: with no additive, the override value stands alone.
     * rb_calc_stage_hearts is Player::calculate_stage_hearts. */
    int hearts_player[8];
    rb_calc_stage_hearts(&tg.state, 0, hearts_player);
    CHECK_EQ(hearts_player[HEART01_IDX], override_count,
             "B2 CONTROL: with no additive the override alone contributes exactly override_count heart01");

    /* 9.9.1.5: an additive +1 heart01 must stack ON TOP. */
    rb_mods_add_heart(&tg.state.mods, kasumi, HEART01_IDX, 1);

    rb_calc_stage_hearts(&tg.state, 0, hearts_player);
    CHECK_EQ(hearts_player[HEART01_IDX], override_count + 1,
             "B2: 9.9.1.5 calculate_stage_hearts applies additive modifiers ON TOP of the override");

    /* get_available_hearts -- the other stage-heart computation, reached
     * through the full stage-hearts pipeline. */
    int hearts_zone[8];
    rb_stage_hearts_pipeline(&tg.state, 0, hearts_zone);
    CHECK_EQ(hearts_zone[HEART01_IDX], override_count + 1,
             "B2: 9.9.1.5 get_available_hearts applies additive modifiers ON TOP of the override");
}

/* =======================================================================
 * C. rules/conditions/general_procedure_qa_rulings_test.rs -- Q37
 *
 * Q37: a LiveStart auto resolves ONCE per timing. Firing the trigger again in
 * the same window must not stack the jidou (always-on) gain.
 * ======================================================================= */

static void c_q37_live_start_grant_does_not_stack_on_refire(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, AI);
    int d1 = mid(&tg, FILLER);
    int d2 = mid(&tg, FILLER);
    if (!pin_id(member, AI)) return;
    if (!pin_id(d1, FILLER)) return;

    /* Ai Miyashita ab#1 is the LiveStart: "if 2 or more members have appeared on
     * my stage this turn, gain a jidou effect of +1 to the live total score,
     * until live end". Its gate is a temporal_condition carrying
     * trigger_event {type: temporal_count, count: 2}. */
    int ls_idx = nth_ability_with_trigger(member, NULL, 1);
    CHECK(ls_idx == 1, "C: ab#1 of Ai Miyashita is the LiveStart (ab#0 is the 3-debut draw)");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, d1, 30);
    test_give_energy(&tg, 30);
    test_add_to_hand(&tg, d1);
    test_add_to_hand(&tg, d2);
    test_add_to_hand(&tg, member);

    int played1 = test_play_to_stage(&tg, d1, AREA_LEFT);
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);
    CHECK_EQ(played1, 1, "C: the first member is deployed to the left area");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], d1, "C: the first member occupies the left area");

    int played2 = test_play_to_stage(&tg, d2, AREA_RIGHT);
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);
    CHECK_EQ(played2, 1, "C: the second member is deployed to the right area");

    int played3 = test_play_to_stage(&tg, member, AREA_CENTER);
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);
    CHECK_EQ(played3, 1, "C: Ai Miyashita is deployed to the centre area");

    /* CONTROL for the negative half: the gate is "2 or more appearances this
     * turn", so the board must really carry two debuts. Without this, a zero
     * score bonus would prove nothing. */
    CHECK_EQ(tg.state.debut_count_this_turn[0] >= 2 ? 1 : 0, 1,
             "C CONTROL: 2 members have debuted this turn, so the >=2 gate is satisfied");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "C CONTROL: no live-total jidou bonus exists before the LiveStart fires");

    /* Fire the LiveStart trigger ONCE per its natural timing (Q37). */
    int fired = fire_ability_index(&tg, member, ls_idx, RB_TK_LIVE_START);
    CHECK(fired, "C: the LiveStart ability was enqueued and dispatched");
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);

    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "C: one LiveStart timing -> exactly one jidou live-total score+1 grant");

    /* Second fire in the SAME window: the gained-ability registration is not
     * idempotent, so this is where Q37 could stack. */
    fire_ability_index(&tg, member, ls_idx, RB_TK_LIVE_START);
    drain_pick0(&tg, 16);
    drain_only_auto(&tg);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "C: re-firing the LiveStart in the same window does NOT stack the jidou grant");
}

/* =======================================================================
 * D. rules/conditions/live_card_zone_movement_test.rs -- the three tests with
 *    no C home. The other ten already live in test_live_card_zone_movement.c.
 *
 *   D1  face_down_non_live_card_survives_live_card_set_check
 *   D1b (added control) the removal path IS capable of running
 *   D2  invalid_resolution_card_returns_to_its_owner
 *   D4  (added control) the owner decides, not the active seat
 *   D3  rule_10_5_2_non_energy_card_in_energy_zone_goes_to_owners_waitroom
 * ======================================================================= */

static void d1_face_down_non_live_card_survives_live_card_set_check(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    int filler = mid(&tg, FILLER);
    if (!pin_id(member, MEMBER4)) return;
    if (!pin_id(filler, FILLER)) return;
    CHECK(!rb_card_is_live(member), "D1: the planted card is NOT a live card");

    test_add_to_live(&tg, member);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);

    /* Rust leaves live_card_zone.face_up at its default (false) and asserts the
     * card SURVIVES check_timing, because Rust's check_invalid_live_cards
     * returns early on a face-DOWN zone (engine/src/turn/actions/mod.rs:1702:
     * `if !...face_up { return }`). The C RbPlayer has NO face_up field at all,
     * so the Rust board is reproduced as faithfully as the struct allows:
     * there is nothing that could be set. */
    DIAG("D1: C GameState has no live_card_zone.face_up field, so the face-down "
         "state of the Rust fixture is not representable");

    CHECK_EQ(live_has(&tg, 0, member), 1,
             "D1 CONTROL: the non-live card is in the live zone before check_timing");

    rb_check_timing(&tg.state);

    CHECK_EQ(live_has(&tg, 0, member), 1,
             "D1: a face-DOWN non-live card survives the live-card-set timing check");
    CHECK_EQ(waitroom_has(&tg, 0, member), 0,
             "D1: the face-DOWN non-live card is not sent to the waitroom");
    dump_live_zone(&tg, 0);
    fprintf(stderr, "        p1 waitroom n=%d\n", tg.state.p[0].discard.n);
}

static void d1b_removal_path_control(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int a = mid(&tg, MEMBER4);
    int b = mid(&tg, MEMBER4);
    int filler = mid(&tg, FILLER);
    if (!pin_id(a, MEMBER4)) return;
    if (!pin_id(b, MEMBER4)) return;

    test_add_to_live(&tg, a);
    test_add_to_live(&tg, b);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);
    CHECK_EQ(tg.state.p[0].live.n, 2, "D1b CONTROL: two non-live cards are in the live zone");

    rb_check_timing(&tg.state);

    /* D1's survival assertion is only worth something if the removal path is
     * capable of running at all. This is its paired positive: the SAME
     * non-live card in the SAME zone, on a board where the only difference is
     * that C cannot represent face-down, IS removed. If D1 and D1b ever agree
     * on the outcome, D1 is vacuous. */
    CHECK_EQ(tg.state.p[0].live.n, 0,
             "D1b CONTROL: a non-live card IS removed from the live zone by check_timing");
    CHECK_EQ(waitroom_has(&tg, 0, a), 1, "D1b: the removed card lands in P1's waitroom");
    CHECK_EQ(waitroom_has(&tg, 0, b), 1, "D1b: both removed cards land in P1's waitroom");
}

/* Rust pushes the resolution card for OWNER 1 while P1 is the first attacker,
 * so the second attacker (P2) owns the resolution and receives the card. */
static void d2_invalid_resolution_card_returns_to_its_owner(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    if (!pin_id(member, MEMBER4)) return;

    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;
    tg.state.active = 0;    /* the live is still P1's; the resolution is P2's */
    fill_decks(&tg, member, 30);

    rb_resolution_add(&tg.state, member);
    CHECK_EQ(rb_resolution_len(&tg.state), 1, "D2 CONTROL: the card is in the resolution zone");

    rb_check_timing(&tg.state);

    CHECK_EQ(rb_resolution_len(&tg.state), 0, "D2: the resolution zone is emptied by check_timing");
    CHECK_EQ(waitroom_has(&tg, 0, member), 0, "D2: P1's waitroom must NOT receive P2's card");
    CHECK_EQ(waitroom_has(&tg, 1, member), 1,
             "D2: the resolution card returns to its OWNER's waitroom (P2, the second attacker)");
    fprintf(stderr, "        p1 waitroom n=%d, p2 waitroom n=%d, active=%d, "
                    "first_attacker=%d, second_attacker=%d\n",
            tg.state.p[0].discard.n, tg.state.p[1].discard.n,
            tg.state.active, tg.state.first_attacker, tg.state.second_attacker);
}

/* D4 -- ADDED control for D2. The C resolution zone is a plain RbBag with no
 * owner array, so the engine has nothing to route on except g->active. D2 and
 * D4 differ ONLY in the seat that owns the resolution, which is what makes
 * "the owner decides, not the active seat" a measurement rather than an
 * assumption. */
static void d4_resolution_owner_decides_not_the_active_seat(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member = mid(&tg, MEMBER4);
    if (!pin_id(member, MEMBER4)) return;

    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;
    tg.state.active = 1;    /* the ACTIVE seat is now P2, so a g->active-driven
                             * dump and an owner-driven dump coincide */
    fill_decks(&tg, member, 30);

    rb_resolution_add(&tg.state, member);
    rb_check_timing(&tg.state);

    CHECK_EQ(waitroom_has(&tg, 1, member), 1,
             "D4 CONTROL: when the owner and the active seat agree, the card reaches P2's waitroom");
    CHECK_EQ(waitroom_has(&tg, 0, member), 0,
             "D4 CONTROL: P1's waitroom stays empty on that board");
}

/* Rule 10.5.2: a non-energy card in the energy zone is moved to THAT PLAYER'S
 * waitroom. Owner-scoped: only the offending player's copy is touched. */
static void d3_rule_10_5_2_non_energy_card_in_energy_zone(void)
{
    static TestGame tg;
    case_begin(&tg);
    clear_p1(&tg);

    int member_p1 = mid(&tg, MEMBER4);
    int member_p2 = mid(&tg, MEMBER4);
    int energy    = mid(&tg, ENERGY);
    if (!pin_id(member_p1, MEMBER4)) return;
    if (!pin_id(energy, ENERGY)) return;
    if (member_p1 == member_p2) {
        CHECK(0, "D3 SETUP BUG: the two copies of the member must be DISTINCT pool slots");
        setup_bugs++;
        return;
    }
    CHECK(rb_card_is_energy(energy), "D3: LL-E-001-SD is really an energy card");
    CHECK(!rb_card_is_energy(member_p1), "D3: the member really is not an energy card");

    /* A legal energy card and an illegal member card share P1's energy zone. */
    test_add_to_energy(&tg, 0, energy);
    test_add_to_energy(&tg, 0, member_p1);
    test_add_to_energy(&tg, 1, member_p2);
    fill_decks(&tg, energy, 30);

    CHECK_EQ(tg.state.p[0].energy.n, 2,
             "D3 CONTROL: P1's energy zone holds one legal and one illegal card");
    CHECK_EQ(tg.state.p[1].energy.n, 1, "D3 CONTROL: P2's energy zone holds one illegal card");

    rb_check_timing(&tg.state);

    CHECK_EQ(bag_has(&tg.state.p[0].energy, member_p1), 0,
             "D3: the non-energy card must leave P1's energy zone");
    CHECK_EQ(waitroom_has(&tg, 0, member_p1), 1,
             "D3: it goes to the OWNER's waitroom (P1)");
    CHECK_EQ(bag_has(&tg.state.p[1].energy, member_p2), 0,
             "D3: check timing runs the rule procedure for both players, so P2's copy is cleaned too");
    CHECK_EQ(waitroom_has(&tg, 1, member_p2), 1,
             "D3: each player's own illegal card goes to that player's own waitroom");
    CHECK_EQ(waitroom_has(&tg, 0, member_p2), 0,
             "D3: P1's waitroom must not receive P2's card");
    CHECK_EQ(waitroom_has(&tg, 1, member_p1), 0,
             "D3: P2's waitroom must not receive P1's card");
    CHECK_EQ(bag_has(&tg.state.p[0].energy, energy), 1,
             "D3: a real energy card stays in the energy zone");
}

/* =======================================================================
 * E. rules/conditions/condition_evaluation_test.rs
 *     unsupported_condition_fails_closed_and_missing_condition_stays_unconditional
 *
 * Rust has a real `Condition::Unsupported` variant. It reports condition_type()
 * "custom" (condition.rs:324), is what {"type":"custom"} deserialises to
 * (core/card.rs:3773), and evaluate_condition returns FALSE for it
 * (condition.rs:554) -- that is the fail-closed rule.
 * ======================================================================= */

static void add_str_field(Condition *c, const char *key, const char *v)
{
    c->fields[c->n_fields].key = (char *)key;
    c->fields[c->n_fields].v.tag = RB_TAG_STR;
    c->fields[c->n_fields].v.s = (char *)v;
    c->n_fields++;
}

static void add_i64_field(Condition *c, const char *key, int64_t v)
{
    c->fields[c->n_fields].key = (char *)key;
    c->fields[c->n_fields].v.tag = RB_TAG_I64;
    c->fields[c->n_fields].v.i = v;
    c->n_fields++;
}

static void e_unsupported_condition_fails_closed(void)
{
    static TestGame tg;
    case_begin(&tg);

    /* The Rust Unsupported variant decodes from the token "custom". The C
     * condition vocabulary has no Unsupported member, so "custom" lands on
     * RB_COND_ALWAYS_TRUE, which condition.c answers with a bare 1. */
    int custom_variant = rb_condition_type_from_str("custom");
    CHECK_EQ(custom_variant, (int)RB_COND_ALWAYS_TRUE,
             "E: the token \"custom\" decodes to the C variant RB_COND_ALWAYS_TRUE");

    Condition unsupported;
    memset(&unsupported, 0, sizeof unsupported);
    unsupported.variant = custom_variant;

    /* The Rust assertion: `assert!(!context.evaluate_condition(&unsupported))`
     * -- an UNRECOGNISED condition must FAIL CLOSED. */
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &unsupported), 0,
             "E: an unsupported condition FAILS CLOSED (Rust Condition::Unsupported -> false)");

    /* CONTROL 1 -- the evaluator is not a constant 0: a location condition the
     * board satisfies must be true. */
    int filler = mid(&tg, FILLER);
    if (!pin_id(filler, FILLER)) return;
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    Condition satisfied;
    memset(&satisfied, 0, sizeof satisfied);
    satisfied.variant = RB_COND_LOCATION;
    add_i64_field(&satisfied, "count", 1);
    add_str_field(&satisfied, "operator", ">=");
    add_str_field(&satisfied, "card_type", "member_card");
    add_str_field(&satisfied, "location", "stage");
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &satisfied), 1,
             "E CONTROL: a satisfied location condition is TRUE");

    /* CONTROL 2 -- and not a constant 1 either. The same condition against an
     * empty board must be false, so the two controls are distinguishable and
     * the fail-closed assertion above is measuring the CUSTOM variant rather
     * than a dead evaluator. */
    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, -1, &satisfied), 0,
             "E CONTROL: the same location condition against an empty stage is FALSE");

    /* "a MISSING condition stays unconditional": an AbilityEffect with
     * condition == NULL is allowed. C has no ConditionContext::allows(), so
     * only the effect-level half is unobservable -- the condition-level half is
     * what the two controls above already pin. */
    EXPECTED_HARNESS_GAP(0,
                         "ConditionContext::allows(&AbilityEffect) has no C entry "
                         "point: no exported function takes an AbilityEffect and "
                         "reports whether its (absent) condition admits it, so the "
                         "'missing condition stays unconditional' assertion is not "
                         "expressible from C");
}

/* =======================================================================
 * fork harness
 * ======================================================================= */

static void on_fault(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) { n_failed++; printf("FAILED   %s\n", name); }
        else { n_ok++; printf("ok       %s  (%d assertions)\n", name, assertions - a0); }
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, g0 = harness_gaps;
        current_test = name;
        fn();
        printf("        %-40s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
               name, assertions - a0, failures - f0, harness_gaps - g0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- engine fault, signal %d\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
    } else {
        n_failed++;
        printf("FAILED   %s\n", name);
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fault);
    signal(SIGBUS,  on_fault);
    signal(SIGABRT, on_fault);
    if (getenv("RB_ABILITY_DEBUG")) rb_ability_debug_set(1);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- A. q38 live card zone (PL!N-bp1-029-L / PL!HS-bp1-004-R+) ---\n");
    run("A1_eutopia_one_live_no_score",  q38_eutopia_one_live_card_no_score);
    run("A2_eutopia_three_live_plus_2",  q38_eutopia_three_live_cards_score_plus_2);
    run("A3_rainbow_one_live_one_blade", q38_rainbow_one_live_card_one_blade);
    run("A4_rainbow_declined_no_blade",  q38_rainbow_declining_the_cost_gains_nothing);

    printf("--- B. rule 9.9 layering (PL!SP-bp4-025-L / PL!N-bp3-014-N) ---\n");
    run("B1_blade_set_then_additive",    b1_blade_set_then_additive_stacks_through_real_cards);
    run("B2_heart_override_additive",    b2_heart_override_additive_stacks_in_both_stage_heart_calcs);

    printf("--- C. Q37 live start does not stack on refire (PL!N-bp3-005-R+) ---\n");
    run("C_q37_no_stack_on_refire",      c_q37_live_start_grant_does_not_stack_on_refire);

    printf("--- D. live card zone movement (rule 10.5.2 / resolution owner) ---\n");
    run("D1_face_down_survives",         d1_face_down_non_live_card_survives_live_card_set_check);
    run("D1b_removal_path_control",      d1b_removal_path_control);
    run("D2_resolution_to_owner",        d2_invalid_resolution_card_returns_to_its_owner);
    run("D4_owner_control",              d4_resolution_owner_decides_not_the_active_seat);
    run("D3_rule_10_5_2_energy_zone",    d3_rule_10_5_2_non_energy_card_in_energy_zone);

    printf("--- E. unsupported condition fails closed ---\n");
    run("E_unsupported_fails_closed",    e_unsupported_condition_fails_closed);

    rb_unload();
    printf("\n==== rules/conditions parity: %d cases passed, %d cases failed, "
           "%d crashed (%d assertions, %d setup bugs, %d harness gaps) ====\n",
           n_ok, n_failed, n_crashed, assertions, setup_bugs, harness_gaps);
    return (failures || n_crashed) ? 1 : 0;
}

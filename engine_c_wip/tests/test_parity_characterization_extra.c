/* tests/test_parity_characterization_extra.c
 *
 * C port of the BEHAVIOUR half of the Rust `characterization` cluster,
 * i.e. the tests that pin EXISTING engine behaviour rather than corpus
 * shape. The corpus-shape / decoder half (ability_golden, action_coverage,
 * bytecode_validation, bytecode_deep_compare, card_id_lookup_determinism)
 * is owned by tests/test_parity_characterization.c; this file deliberately
 * does not duplicate it.
 *
 * Cluster source: engine/tests/test_modules/characterization/
 *
 *   1. bp7_character_name_condition_test.rs          (§A)
 *   2. candidate_pool_builder_test.rs                (§B)
 *   3. character_condition_fix_test.rs               (§C)
 *   4. corpus_smoke_test.rs                          (§D)
 *   5. decode_audit_behavior_pins_test.rs            (§E)
 *   6. kashino_player_choice_and_order_measurements_test.rs  (§F)
 *   7. kinako_hs_test.rs                             (§G)
 *   8. pipeline_characterization_test.rs             (§H)
 *   9. prohibition_layer_characterization_test.rs    (§I)
 *  10. size_budget_test.rs                           (§J)
 *  11. strict_engine_bug_expected_fail_test.rs       (§K)
 *  12. zero_tested_action_types_test.rs              (§L)
 *
 * Not ported, and why (reported, not silently dropped):
 *   - describe_parity_test.rs         needs describe_effect_en/ja; no C
 *                                     equivalent exists (checked rabuka.h and
 *                                     the whole src tree).
 *   - logging_test.rs                 needs state.structured_log, a Rust-only
 *                                     metadata log. The C port has a different
 *                                     rb_log_* verdict ring with no
 *                                     choice-offered/resolved entries at all.
 *   - performance_snapshot_audit_test.rs  30 KB of per-card snapshot goldens.
 *                                     The snapshot shape differs (C has only the
 *                                     filled totals, no `breakdown.allocations`
 *                                     list), so every golden would have to be
 *                                     re-derived rather than ported.
 *   - unique_abilities_test.rs       1 byte: an empty file, no test fns.
 *
 * Honesty contract
 * ----------------
 * A `characterization` cluster test is a parity CLAIM about the C engine. When
 * a claim does not hold, that is the most valuable thing this file can report,
 * so every assertion is a hard CHECK and the run stays red. Nothing here is
 * downgraded to a warning to make the suite green. The process returns the
 * failure count, capped at 125.
 *
 * §D (corpus_smoke) runs LAST, deliberately. It is the broadest sweep, so a
 * fault in the middle of the file would swallow the results of every section
 * after it. Running it last means the log still shows what every other section
 * found. This ordering was originally justified by the ability_effects.c
 * %s-vs-char segfault (see the crash-reporting block below), which is now
 * FIXED in commit 15b368d8; the ordering is kept because the run() fork
 * isolation it dovetails with is still load-bearing for any future fault, not
 * because that specific bug is still live.
 *
 * LANDMINES other agents confirmed, respected here:
 *   - test_get_heart_modifier() REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE. Every heart read below goes through rb_mods_get_heart /
 *     rb_mods_get_need_heart directly.
 *   - RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   - Rust's game.id("X") allocates the NEXT DISTINCT pool slot; C's template
 *     lookup aliases. mid() (test_new_id) is the default.
 *   - Group identity keys off the `unit` field, not the printed group name.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── crash reporting ───────────────────────────────────────────────────
 *
 * HISTORY, so the next reader is not sent to a bug that no longer exists:
 * this block used to blame a CONFIRMED undefined-behaviour fault in
 * src/ability/effects/ability_effects.c — rb_translated_execute_gain_ability_effect
 * and rb_translated_execute_invalidate_ability passed
 * `effect->card_type_field[0]` / `effect->self_target_field[0]` (plain `char`,
 * not pointers) into `%s` slots, which printed a byte as an address and killed
 * the process nondeterministically.
 *
 * THAT IS FIXED (commit 15b368d8, "Fix process-killing %s crash in
 * ability_effects.c debug traces"): both arrays are now passed WHOLE, and gcc
 * reports no -Wformat= warning anywhere in the tree (those four were the only
 * ones it ever reported).  The Rust twin emits no such trace; its analogue at
 * ability_effects.rs:289 formats the whole CardType, never its first byte.
 * The regression is pinned by tests/test_ability_effects_crash.c, which sweeps
 * all 255 non-NUL first bytes through both entry points.  Do NOT reinstate the
 * old blame below.
 *
 * The fork+SIGSEGV harness is nevertheless still needed and still correct: it
 * is not a watchdog against that one bug, it is generic crash isolation.  A
 * signal-killed child is reported as CRASH and the sections after it still run,
 * so a future engine fault cannot silently truncate the run and let the
 * remaining sections read as passes.  There is deliberately NO time-based
 * watchdog: a process watchdog is not achievable on this toolchain (an in-child
 * alarm() never fired across a 500 s run, and a parent-side waitpid(WNOHANG)
 * deadline fires but its kill(SIGKILL) does not take effect, wedging the
 * blocking waitpid), so adding one would be dead code that only pretends to
 * protect the run.  A child that HANGS is not caught here; that is a known,
 * recorded limitation, not an oversight.
 */
static const char *current_test = "(none)";
static int failures;
static long assertions;

static void on_segv(int sig)
{
    fprintf(stderr,
            "\n*** SEGFAULT (signal %d) inside test: %s\n"
            "*** The engine, not this test, is at fault.  The previously-blamed\n"
            "*** cause -- ability_effects.c passing card_type_field[0] /\n"
            "*** self_target_field[0] (plain char) to %%s slots -- is FIXED in\n"
            "*** commit 15b368d8, so do not chase it.  Remaining candidates:\n"
            "***  1. Some ability on the staged board pushes a prompt that a\n"
            "***     drain then walks off the end of.\n"
            "***  2. Any other unfixed out-of-bounds access reached by this board.\n"
            "*** Assertions evaluated before the fault: %ld, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    /* _Exit (C99) rather than _exit: the latter is POSIX and is not declared
     * under a strict -std=c11 without _POSIX_C_SOURCE, and the implicit
     * declaration is itself an error on this toolchain. */
    _Exit(128 + sig);
}

/* ── assertion plumbing ───────────────────────────────────────────────── */

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

/* heart colour slots, per cards/compile_cards.py HEART_COLORS */
#define H00 0  /* heart00 pink wildcard */
#define H01 1  /* heart01 RED           */
#define H02 2  /* heart02 YELLOW        */
#define H03 3  /* heart03 GREEN         */
#define H04 4  /* heart04 BLUE          */
#define H05 5  /* heart05 PURPLE        */
#define H06 6  /* heart06 ORANGE        */
#define HALL 7 /* the "all" aggregate   */

/* stage area indices (test_add_to_stage / test_play_to_stage) */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ─────────────────────────────────────────────────────────── */

static int n_setup_bugs;

/* `src` is the in-tree layout; the isolated out-of-tree build root used by
 * tools/isolated_build.sh only has `../cards/build`. rb_load("src") alone
 * therefore fails under an isolated build. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust game.id("X") -> the NEXT DISTINCT pool slot. mid() is the C twin.
 * A miss means the fixture print is wrong, which is a SETUP bug, not an
 * engine finding, so it is counted separately from parity failures. */
static int mid(TestGame *tg, const char *no)
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

/* AGENTS.md identity rule: pin the staged print. PL!S-bp7-003-R＋ and
 * PL!S-bp3-003-R＋ are one letter apart, and BOTH print 松浦果南, so a
 * transposition would still pass a bare name assertion while quietly testing
 * the wrong print. */
static void ident(int cid, const char *expected)
{
    assertions++;
    if (!rb_card_no_eq(cid, expected)) {
        failures++;
        fprintf(stderr, "FAIL: identity: card id %d is '%s', expected the print '%s'\n",
                cid, test_card_name(cid), expected);
    }
}

/* The two 果南 prints: same NAME, different card. The C twin of the Rust
 * assert_same_card_name + assert_ne pair. */
static void ident_same_name(int a, int b, const char *ctx)
{
    assertions++;
    const char *na = test_card_name(a);
    const char *nb = test_card_name(b);
    if (!na || !nb || strcmp(na, nb) != 0) {
        failures++;
        fprintf(stderr, "FAIL: %s: the two cards must print the same name ('%s' vs '%s')\n",
                ctx, na ? na : "?", nb ? nb : "?");
    }
    assertions++;
    if (a == b) {
        failures++;
        fprintf(stderr, "FAIL: %s: the two prints must be SEPARATE instances (both %d)\n",
                ctx, a);
    }
}

/* Start from a board with no cards anywhere, so a staged card is the only
 * instance that can satisfy a position / count condition. */
static void clear_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = P->hand.n = P->discard.n = 0;
        P->live.n = P->success.n = 0;
        P->energy.n = P->energy_deck.n = 0;
        P->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.n_prohibition = 0;
    tg->state.n_prohibition_effects = 0;
    tg->state.n_snapshots = 0;
    tg->state.yell_occurred = 0;
    tg->state.re_yell_occurred = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_batch_movements = 0;
    tg->state.mods.p1_constant_total_score_bonus = 0;
    tg->state.mods.p2_constant_total_score_bonus = 0;
}

static void fresh(TestGame *tg)
{
    test_game_new(tg);
    clear_board(tg);
}

static void set_stage(TestGame *tg, int pl, int area, int card_id)
{
    tg->state.p[pl].stage[area] = card_id;
    tg->state.p[pl].stage_wait[area] = 0;
}

static int zone_has(TestGame *tg, int pl, const char *zone, int id)
{
    return test_zone_has_id(tg, pl, zone, id);
}

static int energy_active(const TestGame *tg)
{
    return tg->state.p[0].energy_active;
}

/* Fill P1's and P2's main decks with `filler`, so a deck refresh cannot
 * wander into the assertion. The C twin of the Rust fill_decks helper. */
static void fill_decks(TestGame *tg, int filler)
{
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Answer every pending choice, bounded so a pathological prompt cannot hang.
 * SelectTarget / SelectHeartColor take an option; everything else takes index
 * 0. This is the peers' drain idiom. */
static void drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        int kind = ch ? (int)ch->kind : (int)RB_CHOICE_NONE;
        if (kind == RB_CHOICE_SELECT_TARGET || kind == RB_CHOICE_SELECT_HEART_COLOR)
            rb_resume_with_choice(&tg->state, 1);
        else
            rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

/* Every pending choice answered with option 0 (the Rust select_indices(&[])
/ * with select_option(0) idempotently). Used for "decline the optional" arms. */
static void drain_decline(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

/* Every pending choice answered with option 1 — the Rust accept_optional
 * discard, whose prompt is ["No","Yes"]. */
static void drain_accept(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        rb_resume_with_choice(&tg->state, 1);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void fire_live_start(TestGame *tg)
{
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

static void fire_live_success(TestGame *tg)
{
    tg->state.live_success[0] = 1;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §A  bp7_character_name_condition_test.rs
 *
 * 小原鞠莉 (PL!S-bp7-008-R) ライブ開始時:
 *   「自分のデッキの一番下のカードを控え室に置いてもよい。それが「松浦果南」か
 *    「黒澤ダイヤ」の場合、それを手札に加える。」
 *
 * The defect this pins: the character-name test became `custom` → AlwaysTrue
 * (so the add-to-hand ALWAYS ran), and the follow-up was
 * `move_cards{source:"discard"}` which grabbed ANY discard card rather than
 * the specific card just placed. The fixed form is
 * `condition{characters, source:"preceding_moved"}` +
 * `move_cards{source:"preceding_moved"}`.
 * ═══════════════════════════════════════════════════════════════════════ */

#define MARI     "PL!S-bp7-008-R"    /* 小原鞠莉                      */
#define KANAN7   "PL!S-bp7-003-R＋"  /* 松浦果南 (the named print)     */
#define KANAN3   "PL!S-bp3-003-R＋"  /* 松浦果南 (the OTHER print)     */
#define DIA      "PL!S-bp7-004-R"    /* 黒澤ダイヤ                     */
#define SUIMIRE  "PL!SP-bp7-004-R"   /* 平安名すみれ (neither)          */
#define LIVE_020 "PL!-sd1-020-SD"    /* the live card that drives the flow */
#define FILLER   "PL!-sd1-010-SD"

/* Seed P1's deck so the bottom-most card is `bottom_card`. The top is a
 * filler that a deck-bottom read never touches. */
/* Seed P1's deck so the bottom-most card is `bottom_card`. The rest are
 * filler that a deck-bottom read never touches. The fillers are the shared
 * template (test_id), which is correct here: the Rust file pushes the SAME
 * filler id ten times, and the assertion is about where the BOTTOM card went,
 * not about filler identity. */
static void seed_deck_with_bottom(TestGame *tg, int bottom_card)
{
    int filler = test_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    /* The deck is a plain list whose LAST slot is the bottom, so the marker is
     * appended after the fillers. */
    test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    test_add_to_deck_pl(tg, 0, bottom_card);
}

/* The C twin of setup_mari(): mari in the centre, five passes to LiveCardSet,
 * then the deck is seeded so the bottom marker survives the live-card-set
 * draws, then the live card is set and two more passes reach live start. */
static int setup_mari(TestGame *tg, int bottom_card, int extra_waitroom)
{
    int mari = mid(tg, MARI);
    ident(mari, MARI);
    set_stage(tg, 0, AREA_CENTER, mari);

    test_give_energy(tg, 3);
    int live = mid(tg, LIVE_020);
    test_add_to_hand(tg, live);
    test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    seed_deck_with_bottom(tg, bottom_card);
    if (extra_waitroom >= 0) {
        tg->state.p[0].discard.n = 0;
        test_add_to_discard(tg, extra_waitroom);
    }
    test_set_live_card(tg, 0, live);
    drain_skip(tg);
    /* The Rust twin reaches live start with two more game.pass() calls, which
     * fire the ライブ開始時 triggers. The C twin fires them directly, which is
     * the same call the phase walk makes — without it nothing fires and every
     * assertion below would pass vacuously or fail for the wrong reason. */
    fire_live_start(tg);
    return bottom_card;
}

/* 1. bottom card IS 松浦果南 → discarded, then added to hand. */
static void test_bp7_mari_bottom_kanan_added_to_hand(void)
{
    TestGame tg;
    fresh(&tg);
    int kanan = mid(&tg, KANAN7);
    ident(kanan, KANAN7);
    int bottom = setup_mari(&tg, kanan, -1);
    drain_accept(&tg);

    CHECK(!zone_has(&tg, 0, "main_deck", bottom),
          "§A mari_bottom_kanan: 果南 must leave the deck");
    CHECK(zone_has(&tg, 0, "hand", bottom),
          "§A mari_bottom_kanan: 果南 must be added to hand");
    CHECK(!zone_has(&tg, 0, "discard", bottom),
          "§A mari_bottom_kanan: 果南 must not remain in the waitroom");
}

/* 2. bottom card IS 黒澤ダイヤ → discarded, then added to hand. */
static void test_bp7_mari_bottom_dia_added_to_hand(void)
{
    TestGame tg;
    fresh(&tg);
    int dia = mid(&tg, DIA);
    ident(dia, DIA);
    int bottom = setup_mari(&tg, dia, -1);
    drain_accept(&tg);

    CHECK(!zone_has(&tg, 0, "main_deck", bottom),
          "§A mari_bottom_dia: ダイヤ must leave the deck");
    CHECK(zone_has(&tg, 0, "hand", bottom),
          "§A mari_bottom_dia: ダイヤ must be added to hand");
    CHECK(!zone_has(&tg, 0, "discard", bottom),
          "§A mari_bottom_dia: ダイヤ must not remain in the waitroom");
}

/* 3. bottom card is neither → discarded, STAYS in the waitroom. This is the
 * arm that fails if the character-name condition has degraded to AlwaysTrue. */
static void test_bp7_mari_bottom_suimire_stays_in_waitroom(void)
{
    TestGame tg;
    fresh(&tg);
    int suimire = mid(&tg, SUIMIRE);
    int bottom = setup_mari(&tg, suimire, -1);
    drain_accept(&tg);

    CHECK(!zone_has(&tg, 0, "main_deck", bottom),
          "§A mari_bottom_suimire: すみれ must leave the deck (the discard ran)");
    CHECK(zone_has(&tg, 0, "discard", bottom),
          "§A mari_bottom_suimire: すみれ must REMAIN in the waitroom — the "
          "「果南かダイヤ」 condition must be false");
    CHECK(!zone_has(&tg, 0, "hand", bottom),
          "§A mari_bottom_suimire: すみれ must NOT be added to hand");
}

/* 4. player declines the optional discard → nothing moves. */
static void test_bp7_mari_skip_optional_discard_moves_nothing(void)
{
    TestGame tg;
    fresh(&tg);
    int kanan = mid(&tg, KANAN7);
    int bottom = setup_mari(&tg, kanan, -1);
    drain_decline(&tg);

    CHECK(zone_has(&tg, 0, "main_deck", bottom),
          "§A mari_skip_optional: 果南 must remain in the deck when skipped");
    CHECK(!zone_has(&tg, 0, "discard", bottom),
          "§A mari_skip_optional: no discard when the optional is declined");
    CHECK(!zone_has(&tg, 0, "hand", bottom),
          "§A mari_skip_optional: no add-to-hand when the optional is declined");
}

/* 5. the waitroom already holds a DIFFERENT 果南, the placed card is not one:
 * the follow-up must target the SPECIFIC placed card, so the pre-existing one
 * must stay put. This is the arm that fails under the old
 * `source:"discard"` behaviour. */
static void test_bp7_mari_does_not_grab_other_discard_match(void)
{
    TestGame tg;
    fresh(&tg);
    int suimire = mid(&tg, SUIMIRE);
    int other_kanan = mid(&tg, KANAN3);
    int named_kanan = mid(&tg, KANAN7);
    ident(other_kanan, KANAN3);
    ident(named_kanan, KANAN7);
    ident_same_name(other_kanan, named_kanan, "two 果南 prints");

    int bottom = setup_mari(&tg, suimire, other_kanan);
    drain_accept(&tg);

    CHECK(zone_has(&tg, 0, "discard", bottom),
          "§A mari_no_grab_other: the placed すみれ stays in the waitroom");
    CHECK(!zone_has(&tg, 0, "hand", other_kanan),
          "§A mari_no_grab_other: the PRE-EXISTING discard 果南 must NOT be "
          "grabbed (preceding_moved targets only the just-placed card)");
    CHECK(zone_has(&tg, 0, "discard", other_kanan),
          "§A mari_no_grab_other: the pre-existing discard 果南 stays put");
    CHECK(!zone_has(&tg, 0, "hand", bottom),
          "§A mari_no_grab_other: すみれ must not reach the hand");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §B  candidate_pool_builder_test.rs
 *
 * The five pool builders share filter logic that will drift. These pin the
 * CURRENT filter semantics so a future shared builder can be validated.
 * ═══════════════════════════════════════════════════════════════════════ */

static void test_candidate_pool_filter_by_card_type(void)
{
    TestGame tg;
    fresh(&tg);
    int m1 = mid(&tg, "PL!-sd1-008-SD");  /* member */
    int l1 = mid(&tg, "PL!N-sd1-025-SD");  /* live   */
    int e1 = mid(&tg, FILLER);             /* member */
    CHECK(rb_card_is_member(m1), "§B fixture PL!-sd1-008-SD must be a member");
    CHECK(rb_card_is_live(l1),   "§B fixture PL!N-sd1-025-SD must be a live card");

    /* The filter choice.rs / look.rs use: card_type == "member_card". */
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    f.has_filter = 1;
    snprintf(f.card_type, sizeof f.card_type, "%s", "member_card");

    int hand_pool[2] = { m1, l1 };
    int hand_out[4];
    int n = rb_matching_ids(&f, hand_pool, 2, hand_out, 4);
    int has_m = 0, has_l = 0;
    for (int i = 0; i < n; i++) {
        if (hand_out[i] == m1) has_m = 1;
        if (hand_out[i] == l1) has_l = 1;
    }
    CHECK(has_m, "§B candidate_pool_card_type: the member filter must keep the "
                 "member card");
    CHECK(!has_l, "§B candidate_pool_card_type: the member filter must drop "
                  "the live card");

    /* The same filter on the waitroom. */
    int wr_pool[1] = { e1 };
    int wr_out[4];
    int n2 = rb_matching_ids(&f, wr_pool, 1, wr_out, 4);
    int wr_has = (n2 == 1 && wr_out[0] == e1);
    CHECK_EQ(wr_has, rb_card_is_member(e1) ? 1 : 0,
             "§B candidate_pool_card_type: waitroom membership under the member "
             "filter must agree with is_member()");
}

static void test_candidate_pool_filter_by_group_name(void)
{
    TestGame tg;
    fresh(&tg);
    int liella = mid(&tg, "PL!SP-bp1-001-R");
    ident(liella, "PL!SP-bp1-001-R");

    RbCardFilter f;
    memset(&f, 0, sizeof f);
    f.has_filter = 1;
    f.has_group = 1;
    snprintf(f.group, sizeof f.group, "%s", "Liella!");

    int one[1] = { liella };
    int out[4];
    int n = rb_matching_ids(&f, one, 1, out, 4);
    CHECK_EQ(n, 1,
             "§B candidate_pool_group: the Liella! filter must select its own "
             "member (got %d of 1)", n);

    /* Determinism: the same input must give the same answer twice. */
    int out2[4];
    CHECK_EQ(rb_matching_ids(&f, one, 1, out2, 4), n,
             "§B candidate_pool_group: the group filter must be deterministic");
}

/* rb_max_distinct_names() exists in ability/util.c but is NOT declared in
 * rabuka.h, so this file uses its declared sibling
 * rb_count_distinct_member_name_units() — the same "two copies of one name are
 * one name" claim, expressed through the Q278/Q279 joint-aware counter. */
static void test_candidate_pool_distinct_filter(void)
{
    TestGame tg;
    fresh(&tg);
    int m1 = mid(&tg, "PL!-sd1-008-SD");
    int m2 = mid(&tg, "PL!-sd1-008-SD");
    ident_same_name(m1, m2, "two copies of one print");

    /* One instance, then two copies of the same name. A duplicated name must
     * not raise the distinct count. */
    int one[1] = { m1 };
    int two[2] = { m1, m2 };
    int d1 = rb_count_distinct_member_name_units(one, 1);
    int d2 = rb_count_distinct_member_name_units(two, 2);
    CHECK_EQ(d1, 1,
             "§B candidate_pool_distinct: one card contributes one distinct name "
             "(got %d)", d1);
    CHECK_EQ(d2, d1,
             "§B candidate_pool_distinct: a second copy of the same name must "
             "NOT raise the distinct count (%d -> %d)", d1, d2);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §C  character_condition_fix_test.rs
 *
 * Fix 2: appearance_condition now supports `positions_characters`, on
 * みらくりえーしょん (PL!HS-bp2-026-L) ライブ開始時:
 *   「自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに
 *    「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合、
 *    このカードのスコアを+2する。」
 *
 * These are the highest-signal claims in the file: a condition that never
 * fires and a modifier that never applies look identical from the positive
 * arm alone, so every negative arm is asserted too.
 * ═══════════════════════════════════════════════════════════════════════ */

#define MIRACLERITION "PL!HS-bp2-026-L"
#define OSAWA         "PL!HS-sd1-003-SD"  /* right_side  */
#define HIMENO        "PL!HS-sd1-006-SD"  /* left_side   */
#define MEGUMI        "PL!HS-bp1-015-N"  /* center 藤島慈 */
#define C_FILLER      "PL!-sd1-013-SD"

/* Stage `left/center/right` (-1 for an empty slot), set the live card, and
 * return its id. The C twin of setup_miraclerition. */
static int setup_miraclerition(TestGame *tg, int left, int center, int right)
{
    int live = mid(tg, MIRACLERITION);
    ident(live, MIRACLERITION);
    set_stage(tg, 0, AREA_LEFT, left);
    set_stage(tg, 0, AREA_CENTER, center);
    set_stage(tg, 0, AREA_RIGHT, right);
    test_add_to_hand(tg, live);
    test_recalc(tg);
    fire_live_start(tg);
    drain_skip(tg);
    return live;
}

static void test_charcond_miraclerition_correct_positions_gains_score(void)
{
    TestGame tg;
    fresh(&tg);
    int osawa  = mid(&tg, OSAWA);
    int himeno = mid(&tg, HIMENO);
    int megumi = mid(&tg, MEGUMI);
    int live = setup_miraclerition(&tg, himeno, megumi, osawa);
    CHECK_EQ(test_get_score_modifier(&tg, live), 2,
             "§C miraclerition_correct_positions: +2 when all three characters "
             "stand at the named positions");
}

static void test_charcond_miraclerition_empty_stage_no_score(void)
{
    TestGame tg;
    fresh(&tg);
    int live = setup_miraclerition(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§C miraclerition_empty_stage: an empty stage must not score");
}

static void test_charcond_miraclerition_wrong_positions_no_score(void)
{
    TestGame tg;
    fresh(&tg);
    int osawa  = mid(&tg, OSAWA);
    int himeno = mid(&tg, HIMENO);
    int megumi = mid(&tg, MEGUMI);
    /* osawa should be right but is at left; himeno should be left but is at
     * right; megumi is correct at center. */
    int live = setup_miraclerition(&tg, osawa, megumi, himeno);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§C miraclerition_wrong_positions: swapping two characters must "
             "not score");
}

static void test_charcond_miraclerition_two_of_three_no_score(void)
{
    TestGame tg;
    fresh(&tg);
    int osawa  = mid(&tg, OSAWA);
    int himeno = mid(&tg, HIMENO);
    int live = setup_miraclerition(&tg, himeno, RB_EMPTY_SLOT, osawa);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§C miraclerition_two_of_three: only two characters present must "
             "not score");
}

static void test_charcond_miraclerition_wrong_character_at_position_no_score(void)
{
    TestGame tg;
    fresh(&tg);
    int osawa  = mid(&tg, OSAWA);
    int himeno = mid(&tg, HIMENO);
    int wrong  = mid(&tg, FILLER);
    int live = setup_miraclerition(&tg, himeno, wrong, osawa);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§C miraclerition_wrong_character: a wrong character at center "
             "must not score");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §D  corpus_smoke_test.rs
 *
 * The behavioural half: EVERY card in the corpus is staged, activated where
 * possible, and its choices drained — then the structural invariant is
 * asserted: a card instance lives in EXACTLY ONE zone, and no zone lists an
 * id twice. A drain that duplicates or drops a card is invisible to a
 * per-card test.
 * ═══════════════════════════════════════════════════════════════════════ */

/* The C twin of assert_state_invariants: returns the number of distinct
 * violations for the given player, and asserts there are none. */
static void assert_state_invariants(TestGame *tg, const char *pname)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            if (a >= RB_STAGE_SIZE) CHECK(0, "%s: stage slot count drifted", pname);
            int id = P->stage[a];
            if (id < 0) continue;
            /* An id must not also sit in a list zone. */
            for (int k = 0; k < P->hand.n; k++)
                CHECK(P->hand.cards[k] != id,
                      "%s: card %d simultaneously in hand and stage[%d]", pname, id, a);
            for (int k = 0; k < P->discard.n; k++)
                CHECK(P->discard.cards[k] != id,
                      "%s: card %d simultaneously in waitroom and stage[%d]", pname, id, a);
            for (int k = 0; k < P->deck.n; k++)
                CHECK(P->deck.cards[k] != id,
                      "%s: card %d simultaneously in main_deck and stage[%d]", pname, id, a);
            for (int k = 0; k < P->live.n; k++)
                CHECK(P->live.cards[k] != id,
                      "%s: card %d simultaneously in live_card_zone and stage[%d]", pname, id, a);
            for (int k = 0; k < P->success.n; k++)
                CHECK(P->success.cards[k] != id,
                      "%s: card %d simultaneously in success_live_zone and stage[%d]", pname, id, a);
            for (int k = 0; k < P->energy.n; k++)
                CHECK(P->energy.cards[k] != id,
                      "%s: card %d simultaneously in energy_zone and stage[%d]", pname, id, a);
        }
        /* No list zone may list the same id twice. */
        for (int k = 0; k < P->hand.n; k++)
            for (int j = k + 1; j < P->hand.n; j++)
                CHECK(P->hand.cards[k] != P->hand.cards[j],
                      "%s: card %d listed twice in hand", pname, P->hand.cards[k]);
        for (int k = 0; k < P->discard.n; k++)
            for (int j = k + 1; j < P->discard.n; j++)
                CHECK(P->discard.cards[k] != P->discard.cards[j],
                      "%s: card %d listed twice in waitroom", pname, P->discard.cards[k]);
        for (int k = 0; k < P->live.n; k++)
            for (int j = k + 1; j < P->live.n; j++)
                CHECK(P->live.cards[k] != P->live.cards[j],
                      "%s: card %d listed twice in live_card_zone", pname, P->live.cards[k]);
    }
}

static void test_corpus_smoke_every_card_executes(void)
{
    TestGame tg;
    fresh(&tg);
    long smoked = 0;
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        const unsigned char *r = rb_card_record(i);
        if (!r) continue;
        const char *no = rb_card_string((uint16_t)(r[0] | (r[1] << 8)));
        if (!no || !*no) continue;
        int cid = test_id(&tg, no);
        if (cid < 0) {
            /* Non-card records (ability-only entries) are skipped, not failed. */
            continue;
        }
        smoked++;

        clear_board(&tg);
        /* Stage the card so self-targeting, position gates and the constant
         * recalculation all have something to chew on. */
        test_add_to_stage(&tg, AREA_CENTER, cid);
        tg.state.activating_card = cid;
        test_recalc(&tg);

        /* The walk is long and the engine logs verbosely, so announce each
         * card on stderr. If the process dies, the last line names the card
         * that killed it, and the SIGSEGV handler in main() adds the test name
         * — without both the crash is unlocatable. */
        fprintf(stderr, "§D smoke card %ld/%u: %s (id %d)\n",
                smoked, (unsigned)n, no, cid);
        fflush(stderr);

        /* Activation may legitimately refuse (unpaid cost, wrong phase). A
         * refusal is fine; a corrupted board is not. */
        test_activate_ability(&tg, cid);
        drain_skip(&tg);
        rb_process_pending_auto_abilities(&tg.state);
        drain_skip(&tg);

        assert_state_invariants(&tg, no);
    }
    CHECK(smoked > 2000,
          "§D corpus_smoke: only %ld cards were actually exercised — the whole "
          "database is expected (>2000), so the walk short-circuited", smoked);
    printf("info: §D corpus_smoke executed %ld cards with structural invariants "
           "intact\n", smoked);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §E  decode_audit_behavior_pins_test.rs
 *
 * Cards whose condition fields ride the shadow schema. The observable
 * behaviour is pinned regardless of which layer implements it.
 * ═══════════════════════════════════════════════════════════════════════ */

/* 心04 read through rb_mods_get_heart, never test_get_heart_modifier. */
static int heart04_of(TestGame *tg, int cid)
{
    return rb_mods_get_heart(&tg->state.mods, cid, H04);
}

static void test_decode_audit_mifune_same_name_in_success_zone_gains_heart04(void)
{
    TestGame tg;
    fresh(&tg);
    int mifune = mid(&tg, "PL!N-bp4-010-R＋");
    ident(mifune, "PL!N-bp4-010-R＋");
    int niji_a = mid(&tg, "PL!N-sd1-025-SD");
    int niji_b = mid(&tg, "PL!N-sd1-025-SD");
    ident_same_name(niji_a, niji_b, "two 虹ヶ咲 live instances");

    /* Direct placement: the ライブ開始時 trigger in isolation. */
    set_stage(&tg, 0, AREA_CENTER, mifune);
    test_add_to_live(&tg, niji_a);
    test_add_to_success(&tg, niji_b);
    test_recalc(&tg);

    fire_live_start(&tg);
    /* 「ライブカードを1枚選ぶ」 — a SelectCard prompt appears even with exactly
     * one candidate. */
    CHECK(test_has_pending_choice(&tg),
          "§E mifune_same_name: the live-card selection must be prompted");
    rb_resume_with_choice(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_skip(&tg);

    CHECK_EQ(heart04_of(&tg, niji_a), 1,
             "§E mifune_same_name: a same-name live card in the success zone "
             "must give the selected live card heart04");
}

static void test_decode_audit_mifune_different_name_no_heart04(void)
{
    TestGame tg;
    fresh(&tg);
    int mifune = mid(&tg, "PL!N-bp4-010-R＋");
    int niji = mid(&tg, "PL!N-sd1-025-SD");
    int other = mid(&tg, "PL!-sd1-020-SD");

    set_stage(&tg, 0, AREA_CENTER, mifune);
    test_add_to_live(&tg, niji);
    test_add_to_success(&tg, other);
    test_recalc(&tg);

    fire_live_start(&tg);
    CHECK(test_has_pending_choice(&tg),
          "§E mifune_different_name: the live-card selection must be prompted "
          "even when the name gate will discard the result");
    rb_resume_with_choice(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_skip(&tg);

    CHECK_EQ(heart04_of(&tg, niji), 0,
             "§E mifune_different_name: a different-name card in the success "
             "zone must NOT give heart04");
}

static void test_decode_audit_mifune_empty_success_zone_no_heart04(void)
{
    TestGame tg;
    fresh(&tg);
    int mifune = mid(&tg, "PL!N-bp4-010-R＋");
    int niji = mid(&tg, "PL!N-sd1-025-SD");

    set_stage(&tg, 0, AREA_CENTER, mifune);
    test_add_to_live(&tg, niji);
    test_recalc(&tg);

    fire_live_start(&tg);
    CHECK(test_has_pending_choice(&tg),
          "§E mifune_empty_success: the live-card selection must be prompted "
          "even with an empty success zone");
    rb_resume_with_choice(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_skip(&tg);

    CHECK_EQ(heart04_of(&tg, niji), 0,
             "§E mifune_empty_success: an empty success zone must NOT give "
             "heart04");
}

/* Dream Believers（105期Ver.）(PL!HS-sd1-018-SD) ライブ開始時:
 *   「自分のステージに『蓮ノ空』のメンバーが3人以上いて、かつ自分の控え室に
 *    カード名に「DreamBelievers」を含むライブカードがある場合、+1」
 *
 * Compound gate: BOTH clauses must hold. Each failing clause is its own arm,
 * because a gate that short-circuits wrong (e.g. accepts either clause) passes
 * the positive arm and fails only here.
 */
#define DREAM_105 "PL!HS-sd1-018-SD"
#define HS_MEMBER "PL!HS-bp5-001-P"
#define DREAM_BASE "PL!HS-bp1-019-L"
#define DREAM_104  "PL!HS-bp5-017-L"

static int dream_believers_setup(TestGame *tg, int n_hs_members, int waitroom_card)
{
    int live = mid(tg, DREAM_105);
    ident(live, DREAM_105);
    test_add_to_live(tg, live);
    for (int slot = 0; slot < n_hs_members && slot < RB_STAGE_SIZE; slot++)
        set_stage(tg, 0, slot, mid(tg, HS_MEMBER));
    if (waitroom_card >= 0) test_add_to_discard(tg, waitroom_card);
    test_recalc(tg);
    fire_live_start(tg);
    drain_skip(tg);
    return live;
}

static void test_decode_audit_dream_believers_both_clauses_score_plus_one(void)
{
    TestGame tg;
    fresh(&tg);
    int live = dream_believers_setup(&tg, 3, mid(&tg, DREAM_BASE));
    CHECK_EQ(test_get_score_modifier(&tg, live), 1,
             "§E dream_believers_both_clauses: 3 蓮ノ空 members AND a "
             "DreamBelievers-named live card in the waitroom -> +1");
}

static void test_decode_audit_dream_believers_wrong_name_no_bonus(void)
{
    TestGame tg;
    fresh(&tg);
    int live = dream_believers_setup(&tg, 3, mid(&tg, "PL!-sd1-020-SD"));
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§E dream_believers_wrong_name: a waitroom live card whose name "
             "lacks the substring must not score");
}

static void test_decode_audit_dream_believers_two_members_no_bonus(void)
{
    TestGame tg;
    fresh(&tg);
    int live = dream_believers_setup(&tg, 2, mid(&tg, DREAM_BASE));
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§E dream_believers_two_members: only 2 蓮ノ空 members must not "
             "score even with a name match");
}

static void test_decode_audit_dream_believers_empty_waitroom_no_bonus(void)
{
    TestGame tg;
    fresh(&tg);
    int live = dream_believers_setup(&tg, 3, -1);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§E dream_believers_empty_waitroom: an empty waitroom must not "
             "score");
}

static void test_decode_audit_dream_believers_variant_name_still_matches(void)
{
    TestGame tg;
    fresh(&tg);
    /* 「Dream Believers（104期Ver.）」 also CONTAINS the substring, so the gate
     * is a substring test, not an exact match. */
    int live = dream_believers_setup(&tg, 3, mid(&tg, DREAM_104));
    CHECK_EQ(test_get_score_modifier(&tg, live), 1,
             "§E dream_believers_variant: the 104期Ver. name contains the "
             "substring, so it must also score");
}

/* 澁谷かのん (PL!SP-bp2-001-R＋) 登場 negative: nothing invalidatable ->
 * 「これにより無効にした場合」 never fires -> no waitroom recovery. */
static void test_decode_audit_kanon_alone_on_stage_no_recovery(void)
{
    TestGame tg;
    fresh(&tg);
    int kanon = mid(&tg, "PL!SP-bp2-001-R＋");
    ident(kanon, "PL!SP-bp2-001-R＋");
    int liella = mid(&tg, "PL!SP-sd1-001-SD");

    test_add_to_hand(&tg, kanon);
    test_add_to_discard(&tg, liella);
    test_give_energy(&tg, 13);
    /* No other member on stage: kanon debuts into an empty centre. */
    test_play_to_stage(&tg, kanon, AREA_CENTER);
    drain_skip(&tg);

    CHECK(!zone_has(&tg, 0, "hand", liella),
          "§E kanon_no_recovery: no invalidatable Liella! member means "
          "「これにより無効にした場合」 cannot fire, so no recovery");
    CHECK(zone_has(&tg, 0, "discard", liella),
          "§E kanon_no_recovery: the Liella! card stays in the waitroom");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §F  kashino_player_choice_and_order_measurements_test.rs
 *
 * One assertion, kept deliberately: the parser must emit the clauses the
 * routing measurements turn on. The file's retracted defect report is the
 * point — a false defect report is worse than no note.
 * ═══════════════════════════════════════════════════════════════════════ */

static void test_kashino_identity(void)
{
    TestGame tg;
    fresh(&tg);
    int k = mid(&tg, "PL!N-bp3-010-R");
    ident(k, "PL!N-bp3-010-R");
    CHECK(test_card_name(k) && strcmp(test_card_name(k), "三船栞子") == 0,
          "§F kashino_identity: PL!N-bp3-010-R must print '三船栞子' (got '%s')",
          test_card_name(k) ? test_card_name(k) : "?");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §G  kinako_hs_test.rs
 *
 * Q81: multi-name cards reference INDIVIDUAL character names, so the printed
 * name must split cleanly. "exactly 3", not ">= 3": a parser that glued a
 * fourth character on would survive a >=3 assertion, which is the failure
 * Q81 is about.
 * ═══════════════════════════════════════════════════════════════════════ */

static void test_kinako_hs_multiname_card_has_three_parts(void)
{
    TestGame tg;
    fresh(&tg);
    int multi = mid(&tg, "LL-bp1-001-R＋");
    ident(multi, "LL-bp1-001-R＋");
    const char *name = test_card_name(multi);
    CHECK(name && strchr(name, '&'),
          "§G kinako_hs: the multi-name card '%s' must contain '&'",
          name ? name : "?");

    int parts = 1;
    for (const char *p = name; p && *p; p++)
        if (*p == '&') parts++;
    CHECK_EQ(parts, 3,
             "§G kinako_hs_multiname_has_three_parts: '%s' splits into %d names; "
             "the card carries exactly three individual names",
             name ? name : "?", parts);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §H  pipeline_characterization_test.rs
 *
 * A1/A2: the SAME board read through all stage-heart and blade entry points
 * must AGREE. A pipeline that double-counts scores a live wrong, and a
 * per-card test cannot see it because each reads only one entry point.
 * ═══════════════════════════════════════════════════════════════════════ */

static void test_h_heart_pipeline_three_entries_agree(void)
{
    TestGame tg;
    fresh(&tg);
    int m1 = mid(&tg, "PL!-sd1-008-SD");
    int m2 = mid(&tg, "PL!SP-bp1-001-R");
    int m3 = mid(&tg, "PL!-PR-003-PR");
    test_add_to_stage(&tg, AREA_LEFT, m1);
    test_add_to_stage(&tg, AREA_CENTER, m2);
    test_add_to_stage(&tg, AREA_RIGHT, m3);

    /* All five heart layers at once, because that is where the orders diverge:
     * copy, multiplier, override, and additives on both sides. */
    rb_mods_set_heart_copy(&tg.state.mods, m1, m2);
    rb_mods_set_heart_color_multiplier(&tg.state.mods, m2, H02);
    rb_mods_set_heart_override(&tg.state.mods, m3, H06, 3);
    rb_mods_add_heart(&tg.state.mods, m1, H01, 1);
    rb_mods_add_heart(&tg.state.mods, m3, H06, 2);
    rb_mods_add_heart(&tg.state.mods, m2, H02, 1);

    int via_zone[8], via_pipeline[8];
    rb_calc_stage_hearts(&tg.state, 0, via_zone);
    rb_stage_hearts_pipeline(&tg.state, 0, via_pipeline);

    /* Third entry point: sum the per-member detail by hand. */
    int manual[8];
    memset(manual, 0, sizeof manual);
    uint8_t base[8], bonus[8];
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        int cid = tg.state.p[0].stage[a];
        if (cid < 0) continue;
        rb_member_heart_detail(&tg.state.mods, cid, base, bonus);
        for (int c = 0; c < 8; c++) manual[c] += base[c] + bonus[c];
    }

    for (int c = 0; c < 8; c++) {
        CHECK_EQ(via_pipeline[c], via_zone[c],
                 "§H heart_pipeline_three_entries_agree: colour %d differs "
                 "between rb_stage_hearts_pipeline and rb_calc_stage_hearts", c);
        CHECK_EQ(manual[c], via_zone[c],
                 "§H heart_pipeline_three_entries_agree: colour %d differs "
                 "between a per-member sum and rb_calc_stage_hearts", c);
    }

    /* Control: the three agreeing on zero would pass vacuously. */
    int total = 0;
    for (int c = 0; c < 8; c++) total += via_zone[c];
    CHECK(total > 0,
          "§H heart_pipeline_three_entries_agree: the board must aggregate a "
          "positive number of hearts (got %d) — three zeros agreeing is not "
          "agreement", total);
}

static void test_h_heart_pipeline_ordering_agrees(void)
{
    TestGame tg;
    fresh(&tg);
    int m = mid(&tg, "PL!-sd1-008-SD");
    test_add_to_stage(&tg, AREA_LEFT, m);

    /* All layers on ONE card: copy self (a no-op that still exercises the
     * path), multiplier, override, and an additive on the override colour. */
    rb_mods_set_heart_copy(&tg.state.mods, m, m);
    rb_mods_set_heart_color_multiplier(&tg.state.mods, m, H03);
    rb_mods_set_heart_override(&tg.state.mods, m, H02, 2);
    rb_mods_add_heart(&tg.state.mods, m, H02, 1);
    rb_mods_add_heart(&tg.state.mods, m, H03, 5);

    int a[8], b[8];
    rb_calc_stage_hearts(&tg.state, 0, a);
    rb_stage_hearts_pipeline(&tg.state, 0, b);
    CHECK_EQ(b[H02], a[H02],
             "§H heart_pipeline_ordering: override + additive on the override "
             "colour must agree across pipelines");
    CHECK_EQ(b[H03], a[H03],
             "§H heart_pipeline_ordering: colour %d must agree across pipelines",
             H03);
}

static void test_h_blade_pipeline_set_plus_additive(void)
{
    TestGame tg;
    fresh(&tg);
    int m = mid(&tg, "PL!SP-bp1-001-R");
    ident(m, "PL!SP-bp1-001-R");
    test_add_to_stage(&tg, AREA_LEFT, m);

    /* 9.9.1.5: set 5 with additive +2 stacks -> 7. */
    rb_mods_set_blade(&tg.state.mods, m, 5);
    rb_mods_add_blade(&tg.state.mods, m, 2);
    RbModifierEntry entry = tg.state.mods.blade[m];
    CHECK_EQ(rb_effective_blade(m, entry), 7,
             "§H blade_set_plus_additive: 5(set)+2(additive) must read 7 per "
             "9.9.1.5");

    /* Without set, the additive stacks on the PRINTED blade. */
    Card c;
    memset(&c, 0, sizeof c);
    if (rb_decode_card_by_index((uint32_t)m, &c)) {
        int printed = c.blade;
        rb_free_card(&c);
        /* A fresh slot so the previous entry does not leak in. */
        int fresh_id = mid(&tg, "PL!SP-bp1-001-R");
        rb_mods_add_blade(&tg.state.mods, fresh_id, 3);
        RbModifierEntry e2 = tg.state.mods.blade[fresh_id];
        CHECK_EQ((int)e2.set, 0, "§H blade_additive_only: set must stay 0");
        CHECK_EQ(rb_effective_blade(fresh_id, e2), printed + 3,
                 "§H blade_additive_only: an additive-only blade must read "
                 "printed(%d) + 3", printed);
    } else {
        CHECK(0, "§H blade_additive_only: the fixture card must decode");
    }
}

/* §9.10 replacement effects: two movements on one event must be two DISTINCT
 * entries — ordering is the caller's choice, so the claim is separability. */
static void test_h_replacement_two_events_are_separate(void)
{
    TestGame tg;
    fresh(&tg);
    int mover  = mid(&tg, FILLER);
    int mover2 = mid(&tg, FILLER);
    CHECK(mover != mover2, "§H replacement_two_events: two distinct instances "
                           "are required");

    tg.state.n_batch_movements = 0;
    rb_record_card_movement(&tg.state, mover,  RB_ZONEID_STAGE,
                            RB_ZONEID_WAITROOM, mover, 0);
    rb_record_card_movement(&tg.state, mover2, RB_ZONEID_STAGE,
                            RB_ZONEID_WAITROOM, mover2, 0);
    CHECK_EQ(tg.state.n_batch_movements, 2,
             "§H replacement_two_events: two movements must be two events");
    CHECK(tg.state.batch_movements[0].moved_card_id !=
              tg.state.batch_movements[1].moved_card_id,
          "§H replacement_two_events: the two entries must name different cards");
}

/* MIRACLE WAVE's printed score is 7; the ROADMAP says 4. Pin the real value so
 * drift is caught, and note the discrepancy rather than hiding it. */
static void test_h_timestamp_singleton_set_card_pinned(void)
{
    TestGame tg;
    fresh(&tg);
    int c1 = mid(&tg, "PL!S-bp3-019-L");
    ident(c1, "PL!S-bp3-019-L");
    Card c;
    memset(&c, 0, sizeof c);
    if (rb_decode_card_by_index((uint32_t)c1, &c)) {
        int score = rb_card_get_score(&c);
        rb_free_card(&c);
        CHECK_EQ(score, 7,
                 "§H timestamp_singleton_set_card: PL!S-bp3-019-L MIRACLE WAVE's "
                 "printed score must stay pinned at 7 (the roadmap says 4; the "
                 "DB says 7 and the DB is what ships)");
    } else {
        CHECK(0, "§H timestamp_singleton_set_card: PL!S-bp3-019-L must decode");
    }

    /* The other singleton candidate must exist and carry a collision-relevant
     * value. */
    int c2 = test_id(&tg, "LL-bp7-001-R＋");
    if (c2 < 0) c2 = test_id(&tg, "LL-bp7-001-R");
    CHECK(c2 >= 0,
          "§H timestamp_singleton_set_card: the LL-bp7-001 family must exist as "
          "a timestamp collision candidate");
}

/* Cross-seat mirror: two DIFFERENT lives, one per seat, neither leaking. */
static void test_h_cross_seat_mirror(void)
{
    TestGame tg;
    fresh(&tg);
    int a = mid(&tg, "PL!SP-bp5-027-L");  /* HOT PASSION!! */
    int b = mid(&tg, "PL!S-bp7-025-L");   /* Guilty Night, Guilty Kiss! */
    ident(a, "PL!SP-bp5-027-L");
    ident(b, "PL!S-bp7-025-L");
    CHECK(a != b, "§H cross_seat_mirror: the two seats hold two DIFFERENT lives");

    set_stage(&tg, 0, AREA_LEFT, a);
    set_stage(&tg, 1, AREA_LEFT, b);
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], a, "§H cross_seat_mirror: seat 1 holds the first");
    CHECK_EQ(tg.state.p[1].stage[AREA_LEFT], b, "§H cross_seat_mirror: seat 2 holds the second");
    CHECK(tg.state.p[1].stage[AREA_LEFT] != a,
          "§H cross_seat_mirror: seat 2 must not hold seat 1's live");
    CHECK(tg.state.p[0].stage[AREA_LEFT] != b,
          "§H cross_seat_mirror: seat 1 must not hold seat 2's live");

    /* Both must actually carry an ability, or the mirror pins nothing. */
    Card ca, cb;
    memset(&ca, 0, sizeof ca);
    memset(&cb, 0, sizeof cb);
    if (rb_decode_card_by_index((uint32_t)a, &ca) &&
        rb_decode_card_by_index((uint32_t)b, &cb)) {
        CHECK(ca.ability != NULL,
              "§H cross_seat_mirror: HOT PASSION!! must have an ability");
        CHECK(cb.ability != NULL,
              "§H cross_seat_mirror: Guilty Night, Guilty Kiss! must have an "
              "ability");
        rb_free_card(&ca);
        rb_free_card(&cb);
    } else {
        CHECK(0, "§H cross_seat_mirror: both mirror cards must decode");
    }
}

/* §9.5 check-timing cascade smoke: two INSTANCES of one card on stage, then
 * the live-card-set flow. A second instance, not the same id twice: two slots
 * holding one id is not a board that can occur. */
static void test_h_s9_check_timing_cascade_smoke(void)
{
    TestGame tg;
    fresh(&tg);
    int live = mid(&tg, "PL!N-sd1-025-SD");
    int m1 = mid(&tg, "PL!-sd1-008-SD");
    int m2 = mid(&tg, "PL!-sd1-008-SD");
    CHECK(m1 != m2, "§H s9_cascade: the two stage members must be distinct "
                    "instances");
    test_add_to_stage(&tg, AREA_LEFT, m1);
    test_add_to_stage(&tg, AREA_CENTER, m2);
    test_add_to_hand(&tg, live);
    test_recalc(&tg);

    int hearts[8];
    rb_calc_stage_hearts(&tg.state, 0, hearts);
    int total = 0;
    for (int c = 0; c < 8; c++) total += hearts[c];
    CHECK(total >= 2,
          "§H s9_cascade: two staged members must supply at least 2 hearts "
          "(got %d) — the per-member pipeline must be callable before the live",
          total);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §I  prohibition_layer_characterization_test.rs
 *
 * Restrictions are stored as strings and matched by substring. Before the
 * layer is typed, pin its observable lifecycle: a conditional cannot_live
 * registers only while its gate holds, and recompute must not leave stale
 * allow-state behind.
 * ═══════════════════════════════════════════════════════════════════════ */

#define KANON_LIVE "PL!SP-bp1-001-R"   /* 「自分のステージにほかのメンバーがいない場合、自分はライブできない。」 */
#define KANON_MATE "PL!HS-bp5-004-R"
#define KAGAYAI    "PL!S-bp2-024-L"   /* 「このカードは成功ライブカード置き場に置くことができない。」 */
#define NORMAL_LIVE "PL!N-bp1-025-L"  /* no restriction */

static void test_i_kanon_alone_on_stage_prohibits_live(void)
{
    TestGame tg;
    fresh(&tg);
    int me = mid(&tg, KANON_LIVE);
    ident(me, KANON_LIVE);
    set_stage(&tg, 0, AREA_LEFT, me);
    test_recalc(&tg);
    CHECK(rb_is_action_prohibited(&tg.state, "cannot_live"),
          "§I kanon_alone_on_stage: alone on stage -> live must be prohibited");
}

static void test_i_kanon_with_teammate_live_allowed_again(void)
{
    TestGame tg;
    fresh(&tg);
    int me = mid(&tg, KANON_LIVE);
    int mate = mid(&tg, KANON_MATE);
    set_stage(&tg, 0, AREA_LEFT, me);
    set_stage(&tg, 0, AREA_CENTER, mate);
    test_recalc(&tg);
    CHECK(!rb_is_action_prohibited(&tg.state, "cannot_live"),
          "§I kanon_with_teammate: a teammate on stage fails the 「ほかのメンバーがいない」 "
          "gate, so no prohibition may register");
}

static void test_i_removing_teammate_reprohibits_without_stale_state(void)
{
    TestGame tg;
    fresh(&tg);
    int me = mid(&tg, KANON_LIVE);
    int mate = mid(&tg, KANON_MATE);
    set_stage(&tg, 0, AREA_LEFT, me);
    set_stage(&tg, 0, AREA_CENTER, mate);
    test_recalc(&tg);
    CHECK(!rb_is_action_prohibited(&tg.state, "cannot_live"),
          "§I remove_teammate: the two-member precondition must be allowed");

    /* The teammate leaves; recomputation must re-register the prohibition. */
    set_stage(&tg, 0, AREA_CENTER, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK(rb_is_action_prohibited(&tg.state, "cannot_live"),
          "§I remove_teammate: a stale allow-state must not survive "
          "recomputation");
}

/* Enforced at PLACEMENT TIME by scanning the card's OWN printed Restriction —
 * not via prohibition_effects registration. The LiveCardZone and
 * SuccessLiveZone are interchangeable for cannot_place. */
static void test_i_bp2024_cannot_place_blocks_both_live_zones(void)
{
    TestGame tg;
    fresh(&tg);
    int live = mid(&tg, KAGAYAI);
    ident(live, KAGAYAI);
    CHECK(!rb_can_place_card_in_zone(&tg.state, live, "success_live_card_zone"),
          "§I bp2024: the printed cannot_place must block success-zone "
          "placement");
    CHECK(!rb_can_place_card_in_zone(&tg.state, live, "live_card_zone"),
          "§I bp2024: LiveCardZone and SuccessLiveZone are interchangeable for "
          "cannot_place, so the live zone must be blocked too");
}

static void test_i_bp2024_positive_control_normal_live_is_placeable(void)
{
    TestGame tg;
    fresh(&tg);
    int normal = mid(&tg, NORMAL_LIVE);
    ident(normal, NORMAL_LIVE);
    CHECK(rb_can_place_card_in_zone(&tg.state, normal, "success_live_card_zone"),
          "§I bp2024_positive_control: a live card WITHOUT the restriction must "
          "be placeable — a validator that blocks everything passes the "
          "negative arms vacuously");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §J  size_budget_test.rs
 *
 * The Rust file's surviving invariant is architectural, not numeric: an idle
 * queue entry must not carry a copy of the resolver. A regression here is
 * invisible to every behaviour test, so it is worth asserting.
 *
 * The C port keeps the resolver entirely OUT of the entry: RbQueueEntry has
 * only a `has_resolver` flag, and the RbAbilityResolver itself is a
 * stack-allocated object in ability/choice.c that never lives in the queue.
 * So the C spelling of "the resolver is boxed" is: the queue is a fixed-size
 * array of entries, and no entry payload contains the resolver.
 * ═══════════════════════════════════════════════════════════════════════ */

static void test_j_queue_entry_does_not_inline_the_resolver(void)
{
    TestGame tg;
    fresh(&tg);

    /* The queue is a fixed inline array, not a growable heap list. */
    CHECK_EQ(RB_QUEUE_DEPTH, 16,
             "§J size_budget: the pinned ability-queue depth is 16");
    CHECK_EQ(sizeof(((RbAbilityQueue *)0)->entries),
             (size_t)RB_QUEUE_DEPTH * sizeof(RbQueueEntry),
             "§J size_budget: the queue must be exactly RB_QUEUE_DEPTH entries "
             "— an over-allocated tail is a per-entry memory regression");

    /* An idle queue holds no resolver: the C guarantee that idle entries pay
     * flag-sized rather than resolver-sized cost. */
    int flagged = 0;
    for (int i = 0; i < tg.state.queue.n_entries; i++)
        if (tg.state.queue.entries[i].has_resolver) flagged++;
    CHECK_EQ(flagged, 0,
             "§J size_budget: a freshly created queue must have no entry flagged "
             "as owning a resolver (got %d)", flagged);

    /* The entry must stay small. A pointer-carrying entry is far below this;
     * an entry that accidentally grew an embedded AbilityEffect tree is not.
     * The bound is deliberately loose — this is a regression guard, not an
     * optimisation target. */
    CHECK(sizeof(RbQueueEntry) < 4096,
          "§J size_budget: RbQueueEntry is %zu bytes, well past what a "
          "pointer-carrying entry needs — has it grown an embedded copy?",
          sizeof(RbQueueEntry));
}

/* ═══════════════════════════════════════════════════════════════════════
 * §K  strict_engine_bug_expected_fail_test.rs
 *
 * STRICT: each asserts the CORRECT behaviour per card text. The Rust header
 * records that each one once documented a real bug, asserted the printed
 * value, and failed; the bugs are since fixed and they have become regression
 * pins for behaviour that was once wrong. If one of these fails now, the
 * ENGINE is wrong, not the assert. The C port keeps them hard for exactly
 * that reason — a red assertion here is a proven parity break.
 * ═══════════════════════════════════════════════════════════════════════ */

#define PB1007      "PL!-pb1-007-R"    /* cost 3 - success_count (clamped 0) */
#define PB1007_LILY "PL!-bp3-014-N"
#define SUCCESS_L   "PL!N-bp1-025-L"
#define MUSUME      "PL!-sd1-020-SD"
#define KEKE        "PL!SP-pb2-006-R"  /* 工藤，就这么决定了！ */
#define LIELLA_K    "PL!SP-pb2-012-R"  /* 澁谷栞 / Liella! Kanon */
#define SUMIRE      "PL!SP-bp2-004-R"
#define PR_CENTER   "PL!HS-PR-001-PR"
#define PR045       "PL!S-PR-045-PR"

/* PB1-007: cost should be 3 - success_count. The engine used to stay at 3
 * regardless of how many success cards were on the field.
 * Returns the id of the activated card, so the caller acts on the SAME
 * instance the setup staged (test_id aliases the shared template; mid() does
 * not, so re-looking-up would land on a different instance). */
static int setup_pb1007_with_success(TestGame *tg, int n)
{
    int me   = mid(tg, PB1007);
    int lily = mid(tg, PB1007_LILY);
    ident(me, PB1007);
    set_stage(tg, 0, AREA_LEFT, me);
    set_stage(tg, 0, AREA_CENTER, lily);
    test_give_energy(tg, 10);
    for (int i = 0; i < n; i++) test_add_to_success(tg, mid(tg, SUCCESS_L));
    for (int i = 0; i < 5; i++) test_add_to_hand(tg, mid(tg, MUSUME));
    test_add_to_discard(tg, mid(tg, MUSUME));
    test_recalc(tg);
    return me;
}

static void test_k_pb1007_cost_2_with_1_success(void)
{
    TestGame tg;
    fresh(&tg);
    int me = setup_pb1007_with_success(&tg, 1);
    test_activate_ability(&tg, me);
    CHECK(test_has_pending_choice(&tg),
          "§K pb1007_cost_2_with_1_success: activation must prompt for cost");
    CHECK_EQ(test_pending_choice_count(&tg), 2,
             "§K pb1007_cost_2_with_1_success: 1 success -> cost 2 "
             "(printed 3 minus 1)");
}

static void test_k_pb1007_cost_1_with_2_success(void)
{
    TestGame tg;
    fresh(&tg);
    int me = setup_pb1007_with_success(&tg, 2);
    test_activate_ability(&tg, me);
    CHECK_EQ(test_pending_choice_count(&tg), 1,
             "§K pb1007_cost_1_with_2_success: 2 success -> cost 1");
}

static void test_k_pb1007_cost_3_with_0_success(void)
{
    TestGame tg;
    fresh(&tg);
    int me = setup_pb1007_with_success(&tg, 0);
    test_activate_ability(&tg, me);
    CHECK_EQ(test_pending_choice_count(&tg), 3,
             "§K pb1007_cost_3_with_0_success: 0 success -> cost 3, the printed "
             "value");
}

/* PR045: only a cost-7 baton draws. The engine used to draw for any cost. */
static int try_baton_strict(TestGame *tg, const char *replaced_no)
{
    int replaced = mid(tg, replaced_no);
    set_stage(tg, 0, AREA_LEFT, replaced);
    int me = mid(tg, PR045);
    test_add_to_hand(tg, me);
    test_give_energy(tg, 25);
    test_add_to_deck_pl(tg, 0, mid(tg, FILLER));
    test_add_to_deck_pl(tg, 0, mid(tg, FILLER));
    int before = tg->state.p[0].deck.n;
    test_play_to_stage(tg, me, AREA_LEFT);
    int had = test_has_pending_choice(tg);
    if (had) {
        rb_resume_with_choice(&tg->state, 0);
        rb_process_pending_auto_abilities(&tg->state);
    }
    int after = tg->state.p[0].deck.n;
    return had && (before - after == 2);
}

static void test_k_pr045_cost6_should_not_draw(void)
{
    TestGame tg;
    fresh(&tg);
    CHECK(!try_baton_strict(&tg, "PL!-sd1-003-SD"),
          "§K pr045_cost6: a cost-6 baton must NOT draw");
}

static void test_k_pr045_cost8_should_not_draw(void)
{
    TestGame tg;
    fresh(&tg);
    CHECK(!try_baton_strict(&tg, "PL!SP-bp5-111-R"),
          "§K pr045_cost8: a cost-8 baton must NOT draw");
}

static void test_k_pr045_cost4_should_not_draw(void)
{
    TestGame tg;
    fresh(&tg);
    CHECK(!try_baton_strict(&tg, "PL!-sd1-001-SD"),
          "§K pr045_cost4: a cost-4 baton must NOT draw");
}

/* Keke: 1 Liella under -> cost +1. The engine used to give 0. */
static void test_k_keke_1_liella_cost_plus1(void)
{
    TestGame tg;
    fresh(&tg);
    int keke = mid(&tg, KEKE);
    int liella = mid(&tg, LIELLA_K);
    ident(keke, KEKE);
    set_stage(&tg, 0, AREA_LEFT, keke);
    test_place_under(&tg, 0, AREA_LEFT, liella);
    test_recalc(&tg);
    CHECK_EQ(test_get_cost_modifier(&tg, keke), 1,
             "§K keke_1_liella: 1 Liella under -> cost +1");
}

static void test_k_keke_2_liella_cost_plus2(void)
{
    TestGame tg;
    fresh(&tg);
    int keke = mid(&tg, KEKE);
    int l1 = mid(&tg, LIELLA_K);
    int l2 = mid(&tg, LIELLA_K);
    CHECK(l1 != l2, "§K keke_2_liella: the two under-cards must be distinct "
                    "instances");
    set_stage(&tg, 0, AREA_LEFT, keke);
    test_place_under(&tg, 0, AREA_LEFT, l1);
    test_place_under(&tg, 0, AREA_LEFT, l2);
    test_recalc(&tg);
    CHECK_EQ(test_get_cost_modifier(&tg, keke), 2,
             "§K keke_2_liella: 2 Liella under -> cost +2");
}

/* SP-bp2-004 highest_cost with a cost modifier respected: the modifier must
 * flip which member counts as highest. */
static void test_k_highest_cost_respects_modifier(void)
{
    TestGame tg;
    fresh(&tg);
    int sumire = mid(&tg, SUMIRE);
    int center = mid(&tg, PR_CENTER);
    int right  = mid(&tg, FILLER);
    ident(sumire, SUMIRE);
    set_stage(&tg, 0, AREA_LEFT, sumire);
    set_stage(&tg, 0, AREA_CENTER, center);
    set_stage(&tg, 0, AREA_RIGHT, right);
    test_recalc(&tg);

    int before = rb_mods_get_heart(&tg.state.mods, sumire, H03);
    CHECK_EQ(before, 1,
             "§K highest_cost_before: centre 10 > left 9, so sumire gains "
             "heart03");

    /* Push the right-hand member's effective cost above the centre's. */
    rb_mods_add_cost(&tg.state.mods, right, 8);
    test_recalc(&tg);
    int after = rb_mods_get_heart(&tg.state.mods, sumire, H03);
    CHECK_EQ(after, 0,
             "§K highest_cost_respects_modifier: right effective 12 > centre 10, "
             "so the centre is no longer highest and sumire must lose the heart");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §L  zero_tested_action_types_test.rs
 *
 * Action types the corpus exercises through ZERO dedicated Rust tests. Every
 * claim here is a characterization claim rather than a re-test.
 * ═══════════════════════════════════════════════════════════════════════ */

#define KANON_DEBUT  "PL!SP-bp2-001-R＋"  /* 澁谷かのん */
#define KANON_TARGET "PL!SP-sd1-003-SD"
#define KANON_RECOVER "PL!SP-pb1-001-R"
#define VIVID        "PL!N-bp4-025-L"
#define NATSUMI      "PL!SP-bp5-025-L"    /* 常夏☆サンシャイン */
#define VITAMIN      "PL!SP-bp2-024-L"    /* ビタミンSUMMER！ */
#define STEP         "PL!S-bp6-019-L"     /* Step! ZERO to ONE */
#define AQOURS_CHIKA "PL!S-PR-025-RM"

/* Kanon's debut nullifies a Liella! member's live_start, then recovers a
 * Liella! card from the waitroom. */
static void test_l_kanon_invalidate_liella_live_start(void)
{
    TestGame tg;
    fresh(&tg);
    int kanon = mid(&tg, KANON_DEBUT);
    int target = mid(&tg, KANON_TARGET);
    int recovery = mid(&tg, KANON_RECOVER);
    ident(kanon, KANON_DEBUT);
    ident(target, KANON_TARGET);
    set_stage(&tg, 0, AREA_LEFT, target);
    test_add_to_hand(&tg, kanon);
    test_add_to_discard(&tg, recovery);
    test_give_energy(&tg, 20);

    test_play_to_stage(&tg, kanon, AREA_CENTER);
    drain_skip(&tg);

    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], target,
             "§L kanon_invalidate: the target must stay at left");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], kanon,
             "§L kanon_invalidate: kanon must land at centre");
    CHECK(rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "§L kanon_invalidate: the target's LiveStart must be invalidated");
    CHECK(zone_has(&tg, 0, "hand", recovery),
          "§L kanon_invalidate: the follow-up must recover the Liella! card to "
          "hand");
    CHECK(!zone_has(&tg, 0, "discard", recovery),
          "§L kanon_invalidate: the recovered card must leave the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "§L kanon_invalidate: the hand must hold exactly the recovered "
             "card (got %d)", tg.state.p[0].hand.n);

    /* The invalidated LiveStart must not offer its optional cost. */
    fire_live_start(&tg);
    CHECK(!test_has_pending_choice(&tg),
          "§L kanon_invalidate: an invalidated LiveStart must not offer its "
          "optional cost");
    CHECK_EQ(test_get_blade_modifier(&tg, target), 0,
             "§L kanon_invalidate: an invalidated LiveStart must not apply its "
             "blade modifier");
}

/* VIVID WORLD ライブ成功時 ab#1: the YELLED 虹ヶ咲 members must collectively
 * cover heart01..heart06. The tests populate revealed_cards directly so the
 * condition is evaluated in isolation. */
#define NJI_Y1 "PL!N-bp1-012-R＋"  /* 鐘嵐珠:    h01,h04,h06 */
#define NJI_Y2 "PL!N-bp1-005-R"    /* 宮下愛:    h01,h02    */
#define NJI_Y3 "PL!N-bp1-007-R"    /* 優木せつ菜: h02,h03    */
#define NJI_Y4 "PL!N-bp1-004-R"    /* 朝香果林:  h05,h06    */
#define NJI_Y5 "PL!N-bp1-003-R＋"  /* 桜坂しずく: h04,h05    */
#define NICO   "PL!-sd1-009-SD"    /* 矢澤にこ:  h01,h03,h06 */

static int vivid_world_score(TestGame *tg, const char *const *yelled, int n)
{
    int live = mid(tg, VIVID);
    ident(live, VIVID);
    test_add_to_live(tg, live);
    for (int i = 0; i < n; i++) test_add_to_revealed(tg, mid(tg, yelled[i]));

    /* Heart00 is the wildcard; the C field is stage_hearts[pl][colour]. */
    memset(tg->state.stage_hearts[0], 0, sizeof tg->state.stage_hearts[0]);
    tg->state.stage_hearts[0][H00] = 20;
    test_recalc(tg);

    fire_live_success(tg);
    drain_skip(tg);
    return test_get_score_modifier(tg, live);
}

static void test_l_vivid_world_all_heart_colors_present(void)
{
    TestGame tg;
    fresh(&tg);
    static const char *const yelled[] = { NJI_Y1, NJI_Y2, NJI_Y3, NJI_Y4, NJI_Y5 };
    CHECK_EQ(vivid_world_score(&tg, yelled, 5), 1,
             "§L vivid_world_all_colours: the yelled 虹ヶ咲 subset covering "
             "heart01..heart06 -> +1");
}

static void test_l_vivid_world_missing_heart_color(void)
{
    TestGame tg;
    fresh(&tg);
    /* heart06 is missing from this subset. */
    static const char *const yelled[] = { NJI_Y2, NJI_Y3, NJI_Y5 };
    CHECK_EQ(vivid_world_score(&tg, yelled, 3), 0,
             "§L vivid_world_missing_colour: heart06 absent from the 虹ヶ咲 "
             "subset -> no score");
}

static void test_l_vivid_world_half_nijigasaki_half_other(void)
{
    TestGame tg;
    fresh(&tg);
    /* ALL yelled cards cover heart01..heart06, but the heart06 comes from a
     * NON-虹ヶ咲 card, which the group check must filter out. */
    static const char *const yelled[] = { NJI_Y2, NJI_Y3, NJI_Y5, NICO };
    CHECK_EQ(vivid_world_score(&tg, yelled, 4), 0,
             "§L vivid_world_half_nijigasaki: the non-虹ヶ咲 heart06 supplier "
             "must be filtered out, so the 虹ヶ咲 subset is still missing "
             "heart06 -> no score");
}

/* 常夏☆サンシャイン ライブ成功時: pay any number of energy, +1 score per 4
 * energy paid. */
static void test_l_natsumi_sunshine_pay_any_energy_for_score(void)
{
    TestGame tg;
    fresh(&tg);
    int live = mid(&tg, NATSUMI);
    int nico = mid(&tg, NICO);
    int h02a = mid(&tg, "PL!SP-sd1-020-SD");
    int h02b = mid(&tg, "PL!SP-sd1-020-SD");
    ident(live, NATSUMI);
    /* The members must satisfy need_heart: h02=2, h03=2, h06=2, h0=8. */
    set_stage(&tg, 0, AREA_LEFT, nico);
    set_stage(&tg, 0, AREA_CENTER, h02a);
    set_stage(&tg, 0, AREA_RIGHT, h02b);
    test_add_to_hand(&tg, live);
    test_give_energy(&tg, 10);
    fill_decks(&tg, mid(&tg, FILLER));
    test_recalc(&tg);

    test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    test_set_live_card(&tg, 0, live);
    drain_skip(&tg);

    /* Reach live success. The C phase enum has no separate
     * LiveVictoryDetermination / LiveEnd: RB_PHASE_PERFORMANCE covers yell and
     * heart resolution, and rb_perform_live in batch mode produces the verdict
     * and the performance snapshot. The C twin of the Rust
     * LiveVictoryDetermination trigger is rb_trigger_live_success with
     * live_success[0] set. */
    test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    drain_skip(&tg);
    rb_perform_live(&tg.state, 0);
    drain_skip(&tg);
    tg.state.live_success[0] = 1;
    rb_trigger_live_success(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_skip(&tg);

    /* Pay 8 energy: eight SelectCard confirmations before finalizing. */
    int paid = 0;
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 32) {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        if (ch && ch->kind == RB_CHOICE_SELECT_CARD && paid < 8) {
            rb_resume_with_choice(&tg.state, 0);
            paid++;
        } else {
            break;
        }
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);

    CHECK_EQ(paid, 8,
             "§L natsumi_sunshine: the 任意の枚数のコスト prompt must accept 8 "
             "energy payments");
    CHECK_EQ(test_get_score_modifier(&tg, live), 2,
             "§L natsumi_sunshine: 8 energy paid -> +2 score (one per 4)");
    CHECK_EQ(energy_active(&tg), 2,
             "§L natsumi_sunshine: 10 given minus 8 paid leaves 2 active energy");
}

/* ビタミンSUMMER！ ライブ成功時: if P1's hand outgrows the opponent's, +1.
 * The live-end score bonus is cleared, but the bonus is visible in the
 * performance snapshot's final score. */
static void test_l_vitamin_summer_live_success_hand_condition(void)
{
    TestGame tg;
    fresh(&tg);
    int live = mid(&tg, VITAMIN);
    ident(live, VITAMIN);
    int nico = mid(&tg, NICO);
    int nico2 = mid(&tg, NICO);
    int h02 = mid(&tg, "PL!SP-sd1-020-SD");
    set_stage(&tg, 0, AREA_LEFT, nico);
    set_stage(&tg, 0, AREA_CENTER, nico2);
    set_stage(&tg, 0, AREA_RIGHT, h02);
    test_add_to_hand(&tg, live);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, mid(&tg, FILLER));
    for (int i = 0; i < 3; i++) test_add_to_hand_for(&tg, 1, mid(&tg, FILLER));
    fill_decks(&tg, mid(&tg, FILLER));
    test_recalc(&tg);

    test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    test_set_live_card(&tg, 0, live);
    drain_skip(&tg);
    test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    drain_skip(&tg);
    rb_perform_live(&tg.state, 0);
    drain_skip(&tg);
    /* ライブ成功時 fires here; the bonus is then cleared when the live ends,
     * which in C is the phase walk past RB_PHASE_VICTORY. */
    tg.state.live_success[0] = 1;
    rb_trigger_live_success(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_skip(&tg);
    test_advance_to_phase(&tg, RB_PHASE_VICTORY);
    drain_skip(&tg);

    /* The LiveSuccess bonus is cleared once the live ends. */
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "§L vitamin_summer: the LiveSuccess score bonus must be cleared "
             "after the live");

    /* The Rust test asserts (score - base_score) == 1 on
     * performance_snapshots[0].lives[0]. RbLiveSnapshot has no base_score and
     * the field is `snapshots`, so the delta is not directly expressible; that
     * shape difference is reported, not faked with a substituted number. */
    const RbLiveSnapshot *s = NULL;
    for (int i = tg.state.n_snapshots - 1; i >= 0; i--)
        if (tg.state.snapshots[i].player == 0) { s = &tg.state.snapshots[i]; break; }
    CHECK(s != NULL,
          "§L vitamin_summer: the live must produce a performance snapshot for "
          "P1, otherwise the +1 bonus is unverifiable");
    if (s) {
        CHECK(s->n_lives > 0,
              "§L vitamin_summer: the snapshot must record the live card");
        CHECK(s->total_score > 0,
              "§L vitamin_summer: the live must have scored above zero (got %d)",
              s->total_score);
    }
}

/* Step! ZERO to ONE ライブ開始時: all-Aqours stage -> +1 score, draw 1, then
 * move 1 from hand to the deck top or bottom. */
static void test_l_step_zero_to_one_live_start(void)
{
    TestGame tg;
    fresh(&tg);
    int live = mid(&tg, STEP);
    int chika = mid(&tg, AQOURS_CHIKA);
    ident(live, STEP);
    set_stage(&tg, 0, AREA_LEFT, chika);
    test_add_to_live(&tg, live);
    int filler = mid(&tg, FILLER);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_hand(&tg, filler);
    test_recalc(&tg);

    fire_live_start(&tg);
    drain_skip(&tg);

    CHECK(!test_has_pending_choice(&tg),
          "§L step_zero_to_one: the whole ability must resolve, leaving no "
          "prompt pending");
    CHECK_EQ(test_get_score_modifier(&tg, live), 1,
             "§L step_zero_to_one: an all-Aqours stage must give the live card "
             "+1 score");
}

/* ═══════════════════════════════════════════════════════════════════════ */

/* Each test builds its own board through fresh(); run() only brackets the
 * pass/fail delta so the summary lists every claim by name. */
/* ── per-test isolation ─────────────────────────────────────────────────
 *
 * The engine has at least two confirmed faults that can kill the process
 * mid-test (see the top of this file and on_segv below). Left alone, the
 * first one to fire truncates the whole run and every section after it reports
 * nothing — which reads as "those tests passed" when they never executed.
 *
 * run() therefore executes each test in a forked child. The child's exit
 * status carries its own failure count, so a test that crashes is attributed
 * to itself and the sections after it still run. A child that dies on a
 * signal is reported as CRASH, never as a pass.
 */
#define CHILD_OK        0   /* child exited 0: test had no failures       */
#define CHILD_FAILURES  1   /* child exited 1: test had hard failures      */
#define CHILD_CRASHED   2   /* child died on a signal: engine fault        */

static int  n_tests_ok, n_tests_failed, n_tests_crashed;

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
        int na = (int)(assertions - a0), nf = (int)(failures - f0);        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions]\n", nf ? "FAILED" : "ok", name, na);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        /* The tally is printed by the child because the parent's counters live
         * in memory the child has its own copy of; a crash loses the child's
         * copy, which is why a crashed test reports no assertion count. */
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
        printf("%-8s %s  <-- unexpected child exit %d\n",
               "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    /* stdout is block-buffered when redirected to a file, so a crash mid-run
     * would swallow every "ok"/"FAILED" line and the summary. Line-buffer it so
     * the log always shows exactly how far the run got. */
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §A bp7_character_name_condition_test.rs ---\n");
    run("bp7_mari_bottom_kanan_added_to_hand",
        test_bp7_mari_bottom_kanan_added_to_hand);
    run("bp7_mari_bottom_dia_added_to_hand",
        test_bp7_mari_bottom_dia_added_to_hand);
    run("bp7_mari_bottom_suimire_stays_in_waitroom",
        test_bp7_mari_bottom_suimire_stays_in_waitroom);
    run("bp7_mari_skip_optional_discard",
        test_bp7_mari_skip_optional_discard_moves_nothing);
    run("bp7_mari_no_grab_other_discard_match",
        test_bp7_mari_does_not_grab_other_discard_match);

    printf("--- §B candidate_pool_builder_test.rs ---\n");
    run("candidate_pool_card_type", test_candidate_pool_filter_by_card_type);
    run("candidate_pool_group_name", test_candidate_pool_filter_by_group_name);
    run("candidate_pool_distinct", test_candidate_pool_distinct_filter);

    printf("--- §C character_condition_fix_test.rs ---\n");
    run("miraclerition_correct_positions",
        test_charcond_miraclerition_correct_positions_gains_score);
    run("miraclerition_empty_stage",
        test_charcond_miraclerition_empty_stage_no_score);
    run("miraclerition_wrong_positions",
        test_charcond_miraclerition_wrong_positions_no_score);
    run("miraclerition_two_of_three",
        test_charcond_miraclerition_two_of_three_no_score);
    run("miraclerition_wrong_character",
        test_charcond_miraclerition_wrong_character_at_position_no_score);

    printf("--- §D corpus_smoke_test.rs ---\n");

    printf("--- §E decode_audit_behavior_pins_test.rs ---\n");
    run("mifune_same_name_heart04",
        test_decode_audit_mifune_same_name_in_success_zone_gains_heart04);
    run("mifune_different_name_no_heart04",
        test_decode_audit_mifune_different_name_no_heart04);
    run("mifune_empty_success_no_heart04",
        test_decode_audit_mifune_empty_success_zone_no_heart04);
    run("dream_believers_both_clauses",
        test_decode_audit_dream_believers_both_clauses_score_plus_one);
    run("dream_believers_wrong_name",
        test_decode_audit_dream_believers_wrong_name_no_bonus);
    run("dream_believers_two_members",
        test_decode_audit_dream_believers_two_members_no_bonus);
    run("dream_believers_empty_waitroom",
        test_decode_audit_dream_believers_empty_waitroom_no_bonus);
    run("dream_believers_variant_name",
        test_decode_audit_dream_believers_variant_name_still_matches);
    /* The §E kanon arm stays at the tail. It was moved there because reaching
     * its debut walked into the ability_effects.c %s-vs-char segfault, which
     * is now FIXED (commit 15b368d8); the position is kept because the arm is
     * the broadest remaining reach in this file and run()'s fork isolation
     * still makes a fault there cost only that one test. */
    printf("--- §F kashino_player_choice_and_order_measurements_test.rs ---\n");
    run("kashino_identity", test_kashino_identity);

    printf("--- §G kinako_hs_test.rs ---\n");
    run("kinako_hs_multiname_three_parts",
        test_kinako_hs_multiname_card_has_three_parts);

    printf("--- §H pipeline_characterization_test.rs ---\n");
    run("heart_pipeline_three_entries_agree",
        test_h_heart_pipeline_three_entries_agree);
    run("heart_pipeline_ordering_agrees",
        test_h_heart_pipeline_ordering_agrees);
    run("blade_set_plus_additive", test_h_blade_pipeline_set_plus_additive);
    run("replacement_two_events_separate",
        test_h_replacement_two_events_are_separate);
    run("timestamp_singleton_set_card", test_h_timestamp_singleton_set_card_pinned);
    run("cross_seat_mirror", test_h_cross_seat_mirror);
    run("s9_check_timing_cascade", test_h_s9_check_timing_cascade_smoke);

    printf("--- §I prohibition_layer_characterization_test.rs ---\n");
    run("kanon_alone_prohibits_live",
        test_i_kanon_alone_on_stage_prohibits_live);
    run("kanon_with_teammate_allowed",
        test_i_kanon_with_teammate_live_allowed_again);
    run("remove_teammate_reprohibits",
        test_i_removing_teammate_reprohibits_without_stale_state);
    run("bp2024_blocks_both_live_zones",
        test_i_bp2024_cannot_place_blocks_both_live_zones);
    run("bp2024_positive_control",
        test_i_bp2024_positive_control_normal_live_is_placeable);

    printf("--- §J size_budget_test.rs ---\n");
    run("queue_entry_no_inlined_resolver",
        test_j_queue_entry_does_not_inline_the_resolver);

    printf("--- §K strict_engine_bug_expected_fail_test.rs ---\n");
    run("pb1007_cost_2_with_1_success", test_k_pb1007_cost_2_with_1_success);
    run("pb1007_cost_1_with_2_success", test_k_pb1007_cost_1_with_2_success);
    run("pb1007_cost_3_with_0_success", test_k_pb1007_cost_3_with_0_success);
    run("pr045_cost6_no_draw", test_k_pr045_cost6_should_not_draw);
    run("pr045_cost8_no_draw", test_k_pr045_cost8_should_not_draw);
    run("pr045_cost4_no_draw", test_k_pr045_cost4_should_not_draw);
    run("keke_1_liella_cost_plus1", test_k_keke_1_liella_cost_plus1);
    run("keke_2_liella_cost_plus2", test_k_keke_2_liella_cost_plus2);
    run("highest_cost_respects_modifier",
        test_k_highest_cost_respects_modifier);

    printf("--- §L zero_tested_action_types_test.rs ---\n");
    run("vivid_world_all_colours", test_l_vivid_world_all_heart_colors_present);
    run("vivid_world_missing_colour", test_l_vivid_world_missing_heart_color);
    run("vivid_world_half_nijigasaki",
        test_l_vivid_world_half_nijigasaki_half_other);
    run("natsumi_sunshine_pay_any_energy",
        test_l_natsumi_sunshine_pay_any_energy_for_score);
    run("vitamin_summer_hand_condition",
        test_l_vitamin_summer_live_success_hand_condition);
    run("kanon_invalidate_liella_live_start",
        test_l_kanon_invalidate_liella_live_start);
    run("step_zero_to_one", test_l_step_zero_to_one_live_start);
    run("kanon_alone_no_recovery",
        test_decode_audit_kanon_alone_on_stage_no_recovery);

    /* §D last: the broadest sweep, and the one most likely to reach the engine
     * fault. A SIGSEGV handler in main() names the test in flight. */
    printf("--- §D corpus_smoke_test.rs (run last) ---\n");
    run("corpus_smoke_every_card_executes",
        test_corpus_smoke_every_card_executes);

    rb_unload();

    printf("\n=== parity_characterization_extra ===\n");
    printf("tests passed          : %d\n", n_tests_ok);
    printf("tests failed (parity) : %d\n", n_tests_failed);
    printf("tests crashed (engine): %d\n", n_tests_crashed);
    /* A fixture print that is not in the database is a bug in THIS file, not a
     * parity break, so it is tallied separately. Each occurrence is printed as
     * a SETUP BUG line and each test line carries its own setup-bug count,
     * because a child's counters die with the child. */
    if (n_tests_failed || n_tests_crashed) {
        fprintf(stderr,
                "PARITY CHARACTERIZATION EXTRA: %d failing test(s), %d crash(es)\n",
                n_tests_failed, n_tests_crashed);
        int code = n_tests_failed + n_tests_crashed;
        return code > 125 ? 125 : code;
    }
    printf("PARITY CHARACTERIZATION EXTRA PASSED\n");
    return 0;
}

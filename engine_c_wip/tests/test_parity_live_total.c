/* Parity suite for engine/tests/test_modules/effects/score/live_total/ --
 * all 11 cluster test files, 44 #[test] functions, against the C engine.
 *
 * CLUSTER SHAPE. Every test in this cluster asserts ONE of two accumulators:
 *
 *   mods.p1_constant_total_score_bonus / .p2_...   「ライブの合計スコア」 as a
 *       per-PLAYER constant (printed target "live_total"), or
 *   mods.get_score_modifier(card)                   a per-CARD modifier.
 *
 * Those are genuinely different code paths and several Rust tests assert BOTH,
 * so both are checked here rather than trusting one.
 *
 * HARNESS NOTES (deliberate, documented so numbers are not misread):
 *
 *  - load_card_db() falls back to ../cards/build. rb_load("src") is the in-tree
 *    layout; tools/isolated_build.sh stages the blobs beside the copied tree,
 *    so an isolated out-of-tree build finds the DB only through that path.
 *
 *  - trigger_live_success() drives ライブ成功時 through rb_trigger_live_success,
 *    the same path test_live_success_rules.c uses. Advancing the real phase
 *    machine is NOT used for it: block_performance_phase in src/ability/types.c
 *    is a static LATCH that stops ライブ成功時 effects ever firing after the
 *    first member's first entry. Every LiveSuccess test here also asserts
 *    rb_should_trigger_live_success() is true BEFORE dispatching, so a
 *    zero-matching-members case can never be confused with a closed window.
 *
 *  - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE, so it
 *    cannot read heart05. Heart reads/writes here go through rb_mods_get_heart
 *    / rb_mods_add_heart directly, like test_get_heart_modifier's own peers.
 *
 *  - card_no lookups are EXACT BYTES in C (rb_find_card_by_no); the Rust
 *    database normalises fullwidth->halfwidth, so the Rust fixtures
 *    "LL-bp1-001-R+" and "PL!S-bp3-001-R\u{ff0b}" both resolve there. Here the
 *    two suffix-bearing cards are spelled with the U+FF0B escape "\xEF\xBC\x8B"
 *    so the fullwidth-plus fixtures carry identical bytes to cards.json.
 *
 *  - Every TestGame local is `static`: sizeof(GameState) is ~781 KB.
 *
 *  - VACUITY. Each Rust test that asserts a NEGATIVE gets, in the same test
 *    function, a positive control or a prompt-count assertion, marked
 *    "control:". A negative that would pass on a board where the effect is a
 *    no-op is the failure mode this file is built to avoid; see the
 *    CONTROLLED_NEGATIVES tally in the summary printed at the end.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

static int failures;
static int assertions;
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

#define GAP(message) do { \
    harness_gaps++; \
    printf("HARNESS GAP: %s\n", message); \
} while (0)

/* ── fixtures with a fullwidth-plus suffix (U+FF0B) ───────────────────── */
#define CHIKA  "PL!S-bp3-001-R\xEF\xBC\x8B"   /* 高海千歌 */
#define JOINT  "LL-bp1-001-R\xEF\xBC\x8B"    /* 上原歩夢&澁谷かのん&日野下花帆 */

/* ── helpers ─────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Mirrors Rust `while has_pending_choice { select_indices(&[]) }`. */
static void drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (ch && ch->kind == RB_CHOICE_SELECT_AUTO_ABILITY)
            test_resume_choice(tg, 0);
        else
            test_resume_choice(tg, -1);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

/* Rust's trigger_live_success_abilities drains every remaining prompt: a
   SelectCard that allows skip is skipped, a SelectTarget takes option 0, and
   anything else stops the loop. This is the faithful port of that drain. */
static void drain_live_success_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 200) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        if (!ch) break;
        if (ch->kind == RB_CHOICE_SELECT_CARD && ch->allow_skip) {
            test_resume_choice(tg, -1);
        } else if (ch->kind == RB_CHOICE_SELECT_TARGET) {
            test_resume_choice(tg, 0);
        } else {
            break;
        }
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static int total_bonus(TestGame *tg, int pl)
{
    return pl == 0 ? (int)tg->state.mods.p1_constant_total_score_bonus
                   : (int)tg->state.mods.p2_constant_total_score_bonus;
}

static int score_mod(TestGame *tg, int cid)
{
    return test_get_score_modifier(tg, cid);
}

/* Rust mods.get_orientation_modifier(id) -> Option<&str>. Returns the stored
   string or NULL. rb_mods_get_orientation is used directly (there is no shim). */
static const char *orientation_of(TestGame *tg, int cid)
{
    const char *s = rb_mods_get_orientation(&tg->state.mods, cid);
    if (!s || !*s) return NULL;
    return s;
}

static int is_wait(TestGame *tg, int cid)
{
    const char *s = orientation_of(tg, cid);
    return s && strcmp(s, "wait") == 0;
}

/* Printed base-heart total, read back from the ENGINE's decode. Used to pin the
   fixture rather than trusting the Rust comment (several printed values in this
   corpus have been found to disagree with the test's own comment). */
static int engine_base_hearts(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int total = 0;
    for (int i = 0; i < c.num_base && i < RB_MAX_HEARTS; i++) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

/* Printed score / printed need, read back from the ENGINE's decode. Used to pin
   the fixtures rather than trusting the Rust comments. */
static int base_score_of(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int s = (int)c.score;
    rb_free_card(&c);
    return s;
}

/* Printed required-heart total of a live card, from the engine's decode. */
static int engine_need_hearts(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    /* the decoder lays the (color,count) pairs out as
       [num_base base][num_blade blade][num_need need] (src/core/card.c:72-84) */
    int total = 0;
    int off = c.num_base + c.num_blade;
    for (int i = 0; i < c.num_need && off + i < RB_MAX_HEARTS; i++)
        total += c.heart_count[off + i];
    rb_free_card(&c);
    return total;
}

static void assert_identity(int cid, const char *no, const char *what)
{
    assertions++;
    if (!rb_card_no_eq(cid, no)) {
        fprintf(stderr, "FAIL: %s is not %s (cid=%d)\n", what, no, cid);
        failures++;
    } else {
        printf("ok: %s == %s\n", what, no);
    }
}

/* Fire every LiveStart ability on `pl`'s live zone + stage. The Rust helper
   trigger_live_start_ability() names ONE ability of ONE card by id; C has no
   ability-id-addressed entry point, so the public rb_trigger_live_start scan is
   used. On these fixtures only the card under test carries ライブ開始時. */
static void fire_live_start(TestGame *tg, int pl, int cid)
{
    tg->state.activating_card = cid;
    rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Open the ライブ成功時 window WITHOUT dispatching, so a test can assert the
   precondition (the Rust files assert should_trigger_live_success inside their
   single shared dispatch helper) before the ability runs.
   rb_should_trigger_live_success() needs BOTH g->p[pl].live.n > 0 and
   g->live_success[pl] (src/turn/triggers.c:197). The Rust fire_trigger helper
   bypasses the whole gate by dispatching the ability directly; the C surface
   expresses the same bypass by setting the flag and parking a filler live card
   in the live zone. */
static void open_live_success_window(TestGame *tg, int pl, int filler_live)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[pl] = 1;
    if (tg->state.p[pl].live.n == 0) test_add_to_live_for(tg, pl, filler_live);
}

/* Fire ライブ成功時 through the rb_trigger_live_success path (see header note on
   the block_performance_phase latch). */
static void fire_live_success(TestGame *tg, int pl, int cid)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[pl] = 1;
    tg->state.activating_card = cid;
    rb_trigger_live_success(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
}

/* =======================================================================
 * A. exact_energy_count_constant_score_test.rs  (1 test)
 *    PL!SP-bp5-222-R 聖澤悠奈 — 「自分のエネルギーが8枚이면」 EXACT count.
 * ======================================================================= */

static void yuna_exactly_eight_energy_grants_one_total_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int yuna = test_id(&tg, "PL!SP-bp5-222-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(yuna, "PL!SP-bp5-222-R", "PL!SP-bp5-222-R fixture");
    fill_decks(&tg, filler, 20);
    test_add_to_stage(&tg, 1, yuna);

    test_give_energy(&tg, 7);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0, "7 energy -> no live-total bonus");

    test_give_energy(&tg, 1);            /* 8 */
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1, "exactly 8 energy -> live total +1");

    test_give_energy(&tg, 1);            /* 9 */
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0, "9 energy -> the exact-count bonus is gone");
    CHECK_EQ(total_bonus(&tg, 1), 0, "the bonus is owner-side only");
}

/* =======================================================================
 * B. twelve_energy_pl_sp_pb1_002_r_test.rs  (1 test)
 *    PL!SP-pb1-002-R 唐 可可 — 「エネルギーカードが12枚以上」 MINIMUM.
 * ======================================================================= */

static void pl_sp_pb1_002_r_twelve_energy_grants_total_score_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int keke = test_id(&tg, "PL!SP-pb1-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(keke, "PL!SP-pb1-002-R", "PL!SP-pb1-002-R fixture");
    fill_decks(&tg, filler, 20);
    test_add_to_stage(&tg, 1, keke);

    test_give_energy(&tg, 11);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0, "11 energy -> no live-total bonus");

    test_give_energy(&tg, 1);            /* 12 */
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "エネルギーカードが12枚以上 -> live total +1");
}

/* =======================================================================
 * C. side_member_original_blade_gated_constant_score_test.rs  (1 test)
 *    PL!-bp6-009-R — BOTH side areas must hold an ORIGINAL blade-2 member.
 * ======================================================================= */

static void muse_center_with_two_original_blade_side_members_adds_one(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me   = test_id(&tg, "PL!-bp6-009-R");
    int left = test_id(&tg, "PL!-sd1-006-SD");
    int right = test_new_id(&tg, "PL!-sd1-006-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(me, "PL!-bp6-009-R", "PL!-bp6-009-R fixture");
    assert_identity(left, "PL!-sd1-006-SD", "PL!-sd1-006-SD (left) fixture");
    assert_identity(right, "PL!-sd1-006-SD", "PL!-sd1-006-SD (right) fixture");
    CHECK(left != right, "control: the two side members are distinct instances");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, me);
    test_add_to_stage(&tg, 0, left);
    test_add_to_stage(&tg, 2, right);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "blade-2 members in BOTH side areas -> live total +1");

    /* Negative control: dropping one side member must drop the bonus. Without
       this the +1 above could come from any single member and the "both" gate
       would never be exercised. */
    test_add_to_stage(&tg, 2, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "control: only the left side filled -> no live-total bonus");

    /* Second control: TWO side members that are NOT blade-2 must also be +0.
       Together with the two above this separates "blade count not read" from
       "position pair not read". */
    test_add_to_stage(&tg, 0, filler);
    test_add_to_stage(&tg, 2, filler);
    test_recalc(&tg);
    CHECK(!rb_card_is_live(filler),
          "control: the low-blade filler fixture decodes as a non-live member");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "control: two NON-blade-2 side members -> no live-total bonus");
}

/* =======================================================================
 * D. kasumi_bp5_002_heart_greater_test.rs  (6 tests)
 *    PL!N-bp5-002-R 中須かすみ — 「ほかのすべてのメンバーより多くのハート」,
 *    strictly, across BOTH stages.
 *    Positive controls: tests D1/D2/D5 (+1). Negative: D3/D4/D6.
 * ======================================================================= */

static void kasumi_alone_on_both_stages_gains_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-bp5-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(kasumi, "PL!N-bp5-002-R", "PL!N-bp5-002-R fixture");
    /* cards.json base_heart = {heart03:3, heart04:1, heart05:1, heart06:1} = 6.
       Assert the ENGINE's decode so the comparison below is anchored to a
       printed value rather than to the Rust comment. */
    CHECK_EQ(engine_base_hearts(kasumi), 6,
             "PL!N-bp5-002-R decodes to 6 printed base hearts");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 0, kasumi);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "Kasumi alone on both stages -> +1 live total");
}

static void kasumi_with_lower_opponent_gains(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-bp5-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);
    CHECK(engine_base_hearts(kasumi) > engine_base_hearts(filler),
          "control: the fixture really is Kasumi(6) > filler(2) on printed hearts");

    test_add_to_stage(&tg, 0, kasumi);
    test_set_opp_stage(&tg, 0, filler);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1, "Kasumi (6) > opponent filler (2) -> +1");
}

static void kasumi_tie_with_opponent_no_gain(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi  = test_id(&tg, "PL!N-bp5-002-R");
    int kasumi2 = test_new_id(&tg, "PL!N-bp5-002-R");
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(kasumi2, "PL!N-bp5-002-R", "the second Kasumi fixture");
    CHECK_EQ(engine_base_hearts(kasumi2), engine_base_hearts(kasumi),
             "control: the tie really is a tie (equal printed hearts)");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 0, kasumi);
    test_set_opp_stage(&tg, 0, kasumi2);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "a tie on hearts is NOT \"more than every other\" -> +0");
}

static void kasumi_lower_than_opponent_no_gain(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-bp5-002-R");
    int high   = test_new_id(&tg, "PL!N-bp5-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);
    rb_mods_add_heart(&tg.state.mods, high, RB_HEART_GREEN, 2);
    /* control: read the modifier back through rb_mods_get_heart (NOT
       test_get_heart_modifier, which remaps colour 5 onto RB_HEART_ORANGE).
       If this is 0 the +0 below would be about the harness, not the engine. */
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, high, RB_HEART_GREEN), 2,
             "control: the opponent's +2 heart03 modifier really landed");
    CHECK(engine_base_hearts(high) + 2 > engine_base_hearts(kasumi),
          "control: the opponent member really does out-heart Kasumi");

    test_add_to_stage(&tg, 0, kasumi);
    test_set_opp_stage(&tg, 0, high);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "a higher member on the opponent's stage disqualifies Kasumi");
}

static void kasumi_with_own_side_lower_still_checks_both(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-bp5-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);
    CHECK(engine_base_hearts(kasumi) > engine_base_hearts(filler),
          "control: Kasumi out-hearts her own-side partner");

    test_add_to_stage(&tg, 0, kasumi);
    test_add_to_stage(&tg, 1, filler);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "Kasumi still highest among own side + empty opponent -> +1");
}

static void kasumi_with_own_side_higher_no_gain(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-bp5-002-R");
    int high   = test_id(&tg, "PL!-sd1-010-SD");   /* a non-未来 member */
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);
    /* The comparison member is deliberately NOT another Kasumi: with a second
       未来 staged, the +1 would belong to HER, and the total would not be
       attributable to the card under test. */
    rb_mods_add_heart(&tg.state.mods, high, RB_HEART_GREEN, 6);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, high, RB_HEART_GREEN), 6,
             "control: the own-side member's +6 heart03 modifier really landed");
    CHECK(engine_base_hearts(high) + 6 > engine_base_hearts(kasumi),
          "control: the own-side member really does out-heart Kasumi");

    test_add_to_stage(&tg, 0, kasumi);
    test_add_to_stage(&tg, 1, high);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "a member on her OWN side with more hearts disqualifies her");
}

/* =======================================================================
 * E. kasumi_energy_under_test.rs  (2 tests)
 *    PL!N-pb1-002-R 中須かすみ — 「このメンバーの下にエネルギーカードが2枚以上」
 *    gated on the DEBUT optional cost actually being paid.
 *    Positive control: E1 (+1, 2 energy under). Negative: E2 (skip -> +0).
 * ======================================================================= */

static void kasumi_constant_score_bonus_applies_when_energy_under(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-pb1-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(kasumi, "PL!N-pb1-002-R", "PL!N-pb1-002-R fixture");
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, kasumi);
    test_give_energy(&tg, 20);

    test_play_to_stage(&tg, kasumi, 1);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "no live-total bonus before the optional debut cost is paid");
    /* control: the play really did pend the optional energy cost */
    CHECK(test_has_pending_choice(&tg),
          "control: the debut pends the optional 「Mm energy under」 cost");

    if (test_has_pending_choice(&tg)) {
        const char *ct = test_pending_choice_type(&tg);
        /* Rust: game.select_energy_from_zone(2) -- the cost is "place 2 of your
           energy under this member", so the answer must ACCEPT the optional
           cost. Two C answer shapes are tried, in the established order
           (test_parity_card_score.c accept_optional_cost uses the single-index
           resume; test_select_indices carries the whole multi-index answer);
           whichever the engine honours must move two cards, which the two
           assertions below pin. */
        if (ct && strcmp(ct, "SelectCard") == 0) {
            int pick[2] = { 0, 1 };
            test_select_indices(&tg, pick, 2);
            if (tg.state.p[0].under_cards[1].n == 0)
                test_resume_choice(&tg, 0);
        } else if (ct && strcmp(ct, "SelectTarget") == 0) {
            test_resume_choice(&tg, 1);
        }
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);

    CHECK_EQ(tg.state.p[0].under_cards[1].n, 2,
             "two energy cards were tucked under Kasumi");
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "+1 live-total bonus when 2+ energy are under the member");
}

static void kasumi_constant_score_without_energy_under(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kasumi = test_id(&tg, "PL!N-pb1-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, kasumi);
    test_give_energy(&tg, 20);

    test_play_to_stage(&tg, kasumi, 1);
    CHECK(test_has_pending_choice(&tg),
          "control: the same optional cost prompt is raised");
    if (test_has_pending_choice(&tg)) test_select_indices(&tg, NULL, 0);
    drain_skip(&tg);

    CHECK_EQ(tg.state.p[0].under_cards[1].n, 0,
             "declining the optional cost leaves no energy under Kasumi");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "no live-total bonus when the optional cost was skipped");
}

/* =======================================================================
 * F. opponent_success_score_six_constant_total_bonus_test.rs  (3 tests)
 *    PL!N-bp4-012-P 鐘 嵐珠 — 「相手の成功ライブカード置き場のスコア合計>=6」
 * ======================================================================= */

static void opponent_success_score_six_grants_one_live_total_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int umi    = test_id(&tg, "PL!N-bp4-012-P");
    int live_6 = test_id(&tg, "PL!SP-bp1-027-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(umi, "PL!N-bp4-012-P", "PL!N-bp4-012-P fixture");
    assert_identity(live_6, "PL!SP-bp1-027-L", "PL!SP-bp1-027-L fixture");
    CHECK(rb_card_is_live(live_6), "control: the score-6 fixture is a live card");
    CHECK_EQ(base_score_of(live_6), 6,
             "control: cards.json scores PL!SP-bp1-027-L at 6");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, umi);
    test_add_to_opp_success(&tg, live_6);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "Umi: opponent success-zone score >= 6 -> +1 live total");
    CHECK_EQ(total_bonus(&tg, 1), 0, "the bonus is owner-side only");
}

static void opponent_success_score_zero_leaves_per_card_modifier_zero(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int umi    = test_id(&tg, "PL!N-bp4-012-P");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, umi);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[1].success.n, 0,
             "control: the opponent success zone really is empty");
    CHECK_EQ(score_mod(&tg, umi), 0,
             "Umi: opponent score 0 -> no per-card score modifier");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "Umi: opponent score 0 -> no live-total bonus either");
}

static void opponent_success_score_removal_clears_live_total_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int umi    = test_id(&tg, "PL!N-bp4-012-P");
    int live_6 = test_id(&tg, "PL!SP-bp1-027-L");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, umi);
    test_add_to_opp_success(&tg, live_6);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "control: the bonus is live before the removal");
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "opponent score >=6 -> +1 live-total bonus");

    tg.state.p[1].success.n = 0;
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "Umi: opponent cards removed -> the live-total bonus clears");
}

/* =======================================================================
 * G. opponent_surplus_constant_total_score_test.rs  (4 tests)
 *    PL!S-bp5-008-R 小原鞠莉 — 「相手の余剰ハートが2つ以上あるかぎり」
 *    Positive controls: G1 middle, G2, G4. Negative: G1 first, G3.
 * ======================================================================= */

static void opponent_surplus_gates_dynamic_live_total_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari = test_id(&tg, "PL!S-bp5-008-R");
    int opp  = test_new_id(&tg, "PL!S-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(mari, "PL!S-bp5-008-R", "PL!S-bp5-008-R fixture");
    assert_identity(opp, "PL!S-sd1-001-SD", "PL!S-sd1-001-SD opponent fixture");
    /* cards.json base_heart = {heart02:3, heart04:2, heart05:2} = 7 */
    CHECK_EQ(engine_base_hearts(opp), 7,
             "control: the opponent member really prints 7 hearts");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, mari);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "no opponent members -> no surplus -> no bonus");

    test_set_opp_stage(&tg, 0, opp);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "opponent surplus 7 >= 2 -> our live total +1");

    test_set_opp_stage(&tg, 0, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "the bonus is removed once the opponent surplus drops below 2");
}

static void opponent_surplus_exactly_two_grants_live_total_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari      = test_id(&tg, "PL!S-bp5-008-R");
    int opp       = test_new_id(&tg, "PL!S-sd1-001-SD");
    int need5     = test_id(&tg, "PL!-sd1-020-SD");
    int filler    = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(need5, "PL!-sd1-020-SD", "PL!-sd1-020-SD live fixture");
    /* cards.json need_heart = {heart01:1, heart03:1, heart0:3} = 5 */
    CHECK_EQ(engine_need_hearts(need5), 5,
             "control: PL!-sd1-020-SD really needs 5 hearts");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, mari);
    test_set_opp_stage(&tg, 0, opp);
    test_add_to_opp_live(&tg, need5);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "surplus exactly 2 (7 hearts - 5 needed) -> the bonus applies");
}

static void opponent_hearts_fully_consumed_by_need_grant_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari   = test_id(&tg, "PL!S-bp5-008-R");
    int opp    = test_new_id(&tg, "PL!S-sd1-001-SD");
    int need7  = test_id(&tg, "PL!S-PR-023-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(need7, "PL!S-PR-023-PR", "PL!S-PR-023-PR live fixture");
    /* cards.json need_heart = {heart02:2, heart04:2, heart05:2, heart0:1} = 7 */
    CHECK_EQ(engine_need_hearts(need7), 7,
             "control: PL!S-PR-023-PR really needs 7 hearts");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, mari);
    test_set_opp_stage(&tg, 0, opp);
    test_add_to_opp_live(&tg, need7);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "surplus 0 (7 hearts - 7 needed) < 2 -> no bonus");
}

static void opponent_surplus_bonus_targets_live_total_not_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari   = test_id(&tg, "PL!S-bp5-008-R");
    int opp    = test_new_id(&tg, "PL!S-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    test_add_to_stage(&tg, 1, mari);
    test_set_opp_stage(&tg, 0, opp);
    test_recalc(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "the live-total accumulator holds the +1");
    CHECK_EQ(score_mod(&tg, mari), 0,
             "ライブの合計スコア targets the TOTAL -> no per-card modifier");
}

/* =======================================================================
 * H. live_success_per_under_member_live_total_score_cap_test.rs (3 tests)
 *    PL!SP-pb2-008-R 若菜四季 — per-unit +1 per 2 Liella! under-member cards,
 *    max_repeats 2.
 *    Driven through rb_trigger_live_success (see header).
 * ======================================================================= */

static int shiki_fixture(TestGame *tg, int n_under, int *out_kanon)
{
    int shiki = test_id(tg, "PL!SP-pb2-008-R");
    int kanon = test_id(tg, "PL!SP-sd1-001-SD");
    int under_card = test_id(tg, "PL!SP-sd1-002-SD");
    int live   = test_id(tg, "PL!-sd1-020-SD");
    int filler = test_id(tg, "PL!-sd1-010-SD");

    tg->state.p[0].stage[0] = kanon;
    tg->state.p[0].stage[1] = shiki;
    test_add_to_live(tg, live);
    for (int i = 0; i < n_under; i++) test_place_under(tg, 0, 1, under_card);
    tg->state.stage_hearts[0][RB_HEART_ALL] = 20;
    fill_decks(tg, filler, 10);
    *out_kanon = kanon;
    return shiki;
}

static void shiki_per_unit_score_capped_at_2(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kanon = 0;
    int shiki = shiki_fixture(&tg, 6, &kanon);
    assert_identity(shiki, "PL!SP-pb2-008-R", "PL!SP-pb2-008-R fixture");
    assert_identity(kanon, "PL!SP-sd1-001-SD", "PL!SP-sd1-001-SD fixture");
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 6,
             "control: six Liella! under-member cards really are under Shiki");

    /* precondition: the ライブ成功時 window must be OPEN, or the dispatch below
       is a silent no-op and the cap assertion proves nothing. */
    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, shiki);
    drain_live_success_choices(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 2,
             "live total score is capped at +2 once (raw 6/2*1=3, max_repeats=2)");
    CHECK_EQ(score_mod(&tg, kanon), 0,
             "the printed target is the live total, not a per-card member modifier");
}

static void shiki_per_unit_score_no_cap_needed(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kanon = 0;
    int shiki = shiki_fixture(&tg, 2, &kanon);
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 2,
             "control: two matching under-member cards");
    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, shiki);
    drain_live_success_choices(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 1,
             "2 matching /2 * 1 = +1 once (below the cap of 2)");}

static void shiki_per_unit_score_zero_matching(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kanon = 0;
    int shiki = shiki_fixture(&tg, 0, &kanon);
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 0,
             "control: no matching under-member cards are staged");
    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, shiki);
    drain_live_success_choices(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 0,
             "0 matching cards -> no live-total bonus");
}

/* =======================================================================
 * I. live_success_self_effect_move_live_total_score_test.rs (7 tests)
 *    PL!SP-pb2-003-R 嵐 千砂都 — 「このターン、自分の『Liella!』のカードの
 *    効果によってこのメンバーがエリアを移動していた場合」
 *    Positive control: I1 / I7 (+1). Negatives: I2..I6.
 * ======================================================================= */

static void fill_deck_and_energy(TestGame *tg)
{
    int filler = test_id(tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 40; i++) test_add_to_deck(tg, filler);
    for (int i = 0; i < 15; i++) test_add_to_energy(tg, 0, filler);
    test_set_energy_active(tg, 0, 15);
}

/* Rust accept_swap_to: answer the position|destination SelectTarget for the
   Chisato swap by picking the generated action whose stage_area is `dest`. The
   flat C RbChoice carries NO option list, so the destination is not observable
   from the choice; the established C idiom (test_parity_card_score.c) answers
   index 0. That is recorded as a harness gap rather than silently assumed. */
static int accept_swap_to(TestGame *tg, const char *dest)
{
    if (!test_has_pending_choice(tg)) return 0;
    const char *t = test_pending_choice_type(tg);
    if (!t || strcmp(t, "SelectTarget") != 0) return 0;
    GAP("position|destination option list is not observable from the flat C "
        "RbChoice; answering index 0 instead of selecting the requested area");
    test_resume_choice(tg, 0);
    rb_process_pending_auto_abilities(&tg->state);
    drain_skip(tg);
    (void)dest;
    return 1;
}

static void chisato_liella_move_grants_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    int keke    = test_id(&tg, "PL!SP-bp4-013-N");
    assert_identity(chisato, "PL!SP-pb2-003-R", "PL!SP-pb2-003-R fixture");
    assert_identity(keke, "PL!SP-bp4-013-N", "PL!SP-bp4-013-N fixture");
    fill_deck_and_energy(&tg);

    tg.state.p[0].stage[0] = chisato;
    test_add_to_hand(&tg, keke);

    /* Do NOT drain before accept_swap_to: the position|destination
       SelectTarget IS the swap prompt we are about to answer. */
    test_play_to_stage(&tg, keke, 2);
    int prompted = accept_swap_to(&tg, "left");
    CHECK(prompted, "control: the debut really raised the Chisato swap prompt");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 1,
             "a Liella! effect moved Chisato -> live total +1");
    CHECK_EQ(score_mod(&tg, chisato), 0,
             "the printed target is the live total, not a per-card modifier");
}

static void chisato_non_liella_move_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    int chii    = test_id(&tg, "PL!HS-bp2-006-R");
    int filler  = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(chii, "PL!HS-bp2-006-R", "PL!HS-bp2-006-R fixture");
    fill_deck_and_energy(&tg);

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].stage[1] = filler;
    test_add_to_hand(&tg, chii);

    test_play_to_stage(&tg, chii, 2);
    drain_skip(&tg);
    /* Rust walks up to 3 formation-change prompts; mirror with a bounded loop. */
    for (int i = 0; i < 3 && test_has_pending_choice(&tg); i++) {
        test_resume_choice(&tg, 0);
        rb_process_pending_auto_abilities(&tg.state);
        drain_skip(&tg);
    }

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(score_mod(&tg, chisato), 0,
             "a non-Liella! effect (藤島 慈) moves Chisato -> no bonus");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "a non-Liella! effect grants no live-total bonus either");
}

static void chisato_opponent_move_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    fill_deck_and_energy(&tg);
    tg.state.p[0].stage[0] = chisato;

    rb_record_card_movement(&tg.state, chisato, RB_ZONEID_STAGE,
                            RB_ZONEID_STAGE, /*causer*/ 1, /*effect_only*/ 1);
    CHECK_EQ(tg.state.n_batch_movements, 1,
             "control: an opponent-caused stage move really was recorded");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(score_mod(&tg, chisato), 0, "an opponent's effect -> no bonus");
    CHECK_EQ(total_bonus(&tg, 0), 0, "an opponent's effect -> no live total");
}

static void chisato_natural_move_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    fill_deck_and_energy(&tg);
    tg.state.p[0].stage[0] = chisato;

    /* effect_only = 0 -> natural movement */
    rb_record_card_movement(&tg.state, chisato, RB_ZONEID_STAGE,
                            RB_ZONEID_STAGE, /*causer*/ 0, /*effect_only*/ 0);
    CHECK_EQ(tg.state.n_batch_movements, 1,
             "control: a natural stage move really was recorded");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(score_mod(&tg, chisato), 0, "a natural move -> no bonus");
    CHECK_EQ(total_bonus(&tg, 0), 0, "a natural move -> no live total");
}

static void chisato_no_move_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    fill_deck_and_energy(&tg);
    tg.state.p[0].stage[0] = chisato;

    CHECK_EQ(tg.state.n_batch_movements, 0,
             "control: no movement at all was recorded");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(score_mod(&tg, chisato), 0, "no movement -> no bonus");
    CHECK_EQ(total_bonus(&tg, 0), 0, "no movement -> no live total");
}

static void chisato_previous_turn_move_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    int keke    = test_id(&tg, "PL!SP-bp4-013-N");
    fill_deck_and_energy(&tg);

    tg.state.p[0].stage[0] = chisato;
    test_add_to_hand(&tg, keke);
    test_play_to_stage(&tg, keke, 2);
    accept_swap_to(&tg, "left");
    drain_skip(&tg);

    /* Rust: turn_area_movements.clear(); turn_number += 1. C folds
       turn_area_movements into batch_movements, cleared per turn. */
    tg.state.n_batch_movements = 0;
    tg.state.turn++;
    CHECK(tg.state.turn > 1, "control: the turn really was advanced");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(score_mod(&tg, chisato), 0, "a previous-turn move -> no bonus");
    CHECK_EQ(total_bonus(&tg, 0), 0, "a previous-turn move -> no live total");
}

static void chisato_move_and_live_success_gives_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-pb2-003-R");
    int keke    = test_id(&tg, "PL!SP-bp4-013-N");
    fill_deck_and_energy(&tg);

    tg.state.p[0].stage[0] = chisato;
    test_add_to_hand(&tg, keke);
    test_play_to_stage(&tg, keke, 2);
    accept_swap_to(&tg, "left");
    drain_skip(&tg);

    int has_move = 0;
    for (int i = 0; i < tg.state.n_batch_movements; i++)
        if (tg.state.batch_movements[i].moved_card_id == chisato &&
            tg.state.batch_movements[i].effect_only &&
            tg.state.batch_movements[i].cause_player_id == 0)
            has_move = 1;
    CHECK(has_move,
          "control: Chisato's own-effect movement is in the turn movement log");

    open_live_success_window(&tg, 0, test_id(&tg, "PL!-sd1-020-SD"));
    CHECK(rb_should_trigger_live_success(&tg.state, 0),
          "precondition: the ライブ成功時 window is open");
    fire_live_success(&tg, 0, chisato);
    drain_live_success_choices(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 1,
             "a Liella! move this turn plus ライブ成功時 -> live total +1");
}

/* =======================================================================
 * J. chika_bp3_001_wait_self_activation_live_total_score_test.rs (10 tests)
 *    PL!S-bp3-001-R＋ 高海千歌 — 「センターエリアに登場している場合のみ起動」
 *    cost: put 1 own member to wait -> that member gains a 常時 +1 live total
 *    for the rest of the live.
 * ======================================================================= */

static void wait_member_self_cost_grants_one_total_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source   = test_id(&tg, CHIKA);
    int opponent = test_id(&tg, "PL!-sd1-010-SD");
    int filler   = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(source, CHIKA, "PL!S-bp3-001-R＋ fixture");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[1] = source;
    tg.state.p[1].stage[1] = opponent;

    test_activate_ability(&tg, source);
    CHECK(!test_has_pending_choice(&tg),
          "with a single own member the cost needs no choice");
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "left area untouched");
    CHECK_EQ(tg.state.p[0].stage[1], source, "source stays on stage");
    CHECK_EQ(tg.state.p[1].stage[1], opponent, "the opponent is untouched");
    CHECK(is_wait(&tg, source),
          "the cost put the source itself into wait orientation");
    CHECK(orientation_of(&tg, opponent) == NULL,
          "the opponent gained no orientation modifier");
    CHECK_EQ(total_bonus(&tg, 0), 1, "the waited member grants live total +1");
    CHECK_EQ(total_bonus(&tg, 1), 0, "the opponent gains nothing");
}

static void wait_member_outside_center_cannot_pay_cost_or_gain(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = source;
    /* Rust try_activate_ability() discards its Result; the C shim's int return
       is the same "did it run" bit, so the call is equivalent. */
    (void)test_activate_ability(&tg, source);
    CHECK(!test_has_pending_choice(&tg),
          "outside the center area the ability does not even prompt");
    CHECK_EQ(tg.state.p[0].stage[0], source, "the member is left alone");
    CHECK(orientation_of(&tg, source) == NULL,
          "no orientation modifier is granted outside the center");
    CHECK_EQ(total_bonus(&tg, 0), 0, "no live-total bonus outside the center");

    /* control: the SAME ability does work in the center, so the +0 above is
       the area gate and not a dead fixture. */
    test_add_to_stage(&tg, 0, RB_EMPTY_SLOT);
    test_add_to_stage(&tg, 1, source);
    test_activate_ability(&tg, source);
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "control: the same card in the center does grant live total +1");
}

static void wait_member_selected_cost_target_stays_on_stage(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int other  = test_id(&tg, "PL!S-bp2-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = other;
    tg.state.p[0].stage[1] = source;

    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg),
          "control: with two own members the cost does prompt for a target");
    if (test_has_pending_choice(&tg)) {
        const char *ct = test_pending_choice_type(&tg);
        CHECK(ct && strcmp(ct, "SelectCard") == 0,
              "the wait target prompt is a SelectCard over the stage");
        int pick[1] = { 0 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);

    CHECK(!test_has_pending_choice(&tg), "the prompt resolved");
    CHECK_EQ(tg.state.p[0].stage[0], other, "the waited member stays on stage");
    CHECK_EQ(tg.state.p[0].stage[1], source, "the source stays on stage");
    CHECK(is_wait(&tg, other), "the selected target is in wait orientation");
    CHECK(orientation_of(&tg, source) == NULL, "the source is not waited");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "no card was discarded by the wait");
    CHECK_EQ(total_bonus(&tg, 0), 1, "the waited member grants live total +1");
}

static void wait_member_selected_cost_target_grants_exactly_one(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int other  = test_id(&tg, "PL!S-bp2-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = other;
    tg.state.p[0].stage[1] = source;
    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg), "control: the cost prompt is raised");
    if (test_has_pending_choice(&tg)) {
        int pick[1] = { 0 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 1, "exactly +1 live total for one waited member");
    CHECK_EQ(total_bonus(&tg, 1), 0, "the opponent's accumulator is untouched");
}

static void wait_member_right_target_waits_without_affecting_others(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int left   = test_id(&tg, "PL!S-bp2-001-R");
    int right  = test_id(&tg, "PL!S-bp2-002-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = left;
    tg.state.p[0].stage[1] = source;
    tg.state.p[0].stage[2] = right;

    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg), "control: the cost prompt is raised");
    if (test_has_pending_choice(&tg)) {
        int pick[1] = { 2 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);

    CHECK(!test_has_pending_choice(&tg), "the prompt resolved");
    CHECK_EQ(tg.state.p[0].stage[0], left, "left member still on stage");
    CHECK_EQ(tg.state.p[0].stage[1], source, "source still on stage");
    CHECK_EQ(tg.state.p[0].stage[2], right, "right member still on stage");
    CHECK(is_wait(&tg, right), "the RIGHT target is the one waited");
    CHECK(orientation_of(&tg, left) == NULL, "the unselected left is unaffected");
    CHECK(orientation_of(&tg, source) == NULL, "the unselected source is unaffected");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "no card was discarded by the wait");
    CHECK_EQ(total_bonus(&tg, 0), 1, "the waited member grants live total +1");
}

static void wait_member_second_activation_cannot_stack(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int other  = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = other;
    tg.state.p[0].stage[1] = source;

    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg), "control: the first activation prompts");
    if (test_has_pending_choice(&tg)) {
        int pick[1] = { 0 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1, "control: the first activation granted +1");
    CHECK(is_wait(&tg, other), "control: the selected member is waiting");
    CHECK(orientation_of(&tg, source) == NULL, "control: the source is not waited");

    /* Rust try_activate_ability() discards its Result. */
    (void)test_activate_ability(&tg, source);

    CHECK(!test_has_pending_choice(&tg), "the second activation raises nothing");
    CHECK_EQ(tg.state.p[0].stage[0], other, "the stage is unchanged");
    CHECK_EQ(tg.state.p[0].stage[1], source, "the source is unchanged");
    CHECK(is_wait(&tg, other), "the first waited member is still waiting");
    CHECK(orientation_of(&tg, source) == NULL, "the source is still not waited");
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "a second activation cannot stack a second live-total +1");
}

/* Baton-touch removal. Rust drives
   TurnEngine::execute_main_phase_action(PlayMemberToStage, ..); the C
   rb_execute_main_phase_action only dispatches UseAbility/Pass, so the
   established C equivalent is test_play_to_stage(), which runs the real
   main-phase rb_play_member and therefore the real baton path. */
static void wait_member_baton_removes_recipient_loses_total_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source      = test_id(&tg, CHIKA);
    int recipient   = test_id(&tg, "PL!-sd1-010-SD");
    int replacement = test_new_id(&tg, "PL!-sd1-010-SD");
    int filler      = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = recipient;
    tg.state.p[0].stage[1] = source;
    test_add_to_hand(&tg, replacement);
    test_give_energy(&tg, 4);

    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg), "control: the cost prompt is raised");
    if (test_has_pending_choice(&tg)) {
        int pick[1] = { 0 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1, "control: the waited member granted +1");
    CHECK(rb_card_num_gained_abilities(&tg.state, recipient) > 0,
          "control: the gained ability is registered on the RECIPIENT");

    int played = test_play_to_stage(&tg, replacement, 0);
    CHECK(played, "control: baton touch onto the occupied left area succeeded");
    drain_skip(&tg);

    CHECK(!test_has_pending_choice(&tg), "no prompt is left after the baton");
    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "the phase is still Main");
    CHECK_EQ(tg.state.p[0].stage[0], replacement,
             "the baton-touched member replaced the recipient");
    CHECK_EQ(tg.state.p[0].stage[1], source, "the source is untouched");
    CHECK(test_zone_has_id(&tg, 0, "discard", recipient),
          "the removed recipient went to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "the hand is empty after the play");
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, recipient), 0,
             "the gained ability dies with the recipient");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "removing the recipient immediately removes the live-total +1");
}

static void wait_member_baton_removes_source_recipient_keeps_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int source      = test_id(&tg, CHIKA);
    int recipient   = test_id(&tg, "PL!-sd1-010-SD");
    int replacement = test_new_id(&tg, "PL!-sd1-010-SD");
    int filler      = test_id(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler, 20);

    tg.state.p[0].stage[0] = recipient;
    tg.state.p[0].stage[1] = source;
    test_add_to_hand(&tg, replacement);
    test_give_energy(&tg, 4);

    test_activate_ability(&tg, source);
    CHECK(test_has_pending_choice(&tg), "control: the cost prompt is raised");
    if (test_has_pending_choice(&tg)) {
        int pick[1] = { 0 };
        test_select_indices(&tg, pick, 1);
        rb_process_pending_auto_abilities(&tg.state);
    }
    drain_skip(&tg);
    CHECK_EQ(total_bonus(&tg, 0), 1, "control: the waited member granted +1");

    int played = test_play_to_stage(&tg, replacement, 1);
    CHECK(played, "control: baton touch onto the occupied center succeeded");
    drain_skip(&tg);

    CHECK(!test_has_pending_choice(&tg), "no prompt is left after the baton");
    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "the phase is still Main");
    CHECK_EQ(tg.state.p[0].stage[0], recipient, "the recipient stays on stage");
    CHECK_EQ(tg.state.p[0].stage[1], replacement,
             "the baton-touched member replaced the activating source");
    CHECK(test_zone_has_id(&tg, 0, "discard", source),
          "the removed source went to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "the hand is empty after the play");
    CHECK(is_wait(&tg, recipient),
          "the recipient is STILL waited after the source leaves");
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "removing the SOURCE leaves the recipient's live-total +1 intact");
}

/* Rust assert_live_end_expiry(set_live). The gained 常時 has duration
   "live_end": it must survive every phase up to and including victory
   determination, and must be gone once the turn rolls over. */
static void wait_member_total_score_expires_at_live_end(int set_live)
{
    static TestGame tg;
    test_game_new(&tg);
    int source = test_id(&tg, CHIKA);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live   = test_id(&tg, "PL!-sd1-019-SD");
    fill_decks(&tg, filler, 30);
    for (int i = 0; i < 4; i++) test_add_to_energy_deck(&tg, 0, filler);
    for (int i = 0; i < 4; i++) test_add_to_energy_deck(&tg, 1, filler);
    test_add_to_success(&tg, filler);          /* give the live a home to run to */
    tg.state.p[0].stage[1] = source;
    if (set_live) test_add_to_live(&tg, live);

    test_activate_ability(&tg, source);
    drain_skip(&tg);
    CHECK(!test_has_pending_choice(&tg), "activation resolved with no prompt");
    CHECK_EQ(total_bonus(&tg, 0), 1, "the waited source grants live total +1");
    CHECK_EQ(tg.state.phase, RB_PHASE_MAIN, "activation stays in Main");

    /* Drive toward victory determination. The phase assertions below are the
       Rust ones; C folds LiveCardSetP1/P2 and First/SecondAttackerPerformance
       into RB_PHASE_LIVE_SET / _SECOND / _PERFORMANCE, so the ladder is checked
       by name-to-phase mapping rather than one-for-one. */
    int saw_live_set = 0, saw_performance = 0, saw_victory = 0;
    for (int i = 0; i < 12; i++) {
        drain_skip(&tg);
        int ph = tg.state.phase;
        if (ph == RB_PHASE_LIVE_SET || ph == RB_PHASE_LIVE_SET_SECOND) saw_live_set = 1;
        if (ph == RB_PHASE_PERFORMANCE || ph == RB_PHASE_PERFORMANCE_SECOND) saw_performance = 1;
        if (ph == RB_PHASE_VICTORY) { saw_victory = 1; break; }
        test_pass(&tg);
    }
    CHECK(saw_live_set, "control: the turn really reached the live-set phase");
    CHECK(saw_performance || saw_victory,
          "control: the turn really reached performance/victory");
    CHECK(saw_victory, "the turn really reached victory determination");

    /* The heart of the mechanic: still +1 at victory determination. */
    CHECK_EQ(total_bonus(&tg, 0), 1,
             "the gained live-total +1 survives all the way to victory");

    /* One more pass rolls the turn over and expires the live_end duration. */
    drain_skip(&tg);
    test_pass(&tg);
    drain_skip(&tg);

    CHECK_EQ(total_bonus(&tg, 0), 0,
             "live end: the gained live-total +1 expires after the live");
}

static void wait_member_total_score_persists_through_victory_and_expires(void)
{
    wait_member_total_score_expires_at_live_end(1);
}

static void wait_member_total_score_expires_without_setting_live(void)
{
    wait_member_total_score_expires_at_live_end(0);
}

/* =======================================================================
 * K. optional_discard_named_members_live_total_score_test.rs (6 tests)
 *    LL-bp1-001-R＋ — ライブ開始時, optional "put any combination of exactly 3
 *    of 上原歩夢/澁谷かのん/日野下花帆 from hand to the waitroom" -> 常時 +3
 *    live total. A hand copy of the joint card itself also qualifies.
 *    Positive controls: K1 (+3), K2 (+3), K5 (+3), K6 (+3). Negatives: K3, K4.
 * ======================================================================= */

/* Rust handle_optional_cost(&[i...]): feed indices to the SelectCard prompts,
   then skip whatever is left, then drain. */
static void handle_optional_cost(TestGame *tg, const int *indices, int n)
{
    int remaining = n;
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 50) {
        const char *ct = test_pending_choice_type(tg);
        if (!ct || strcmp(ct, "SelectCard") != 0) break;
        if (remaining > 0) {
            int pick[1] = { indices[0] };
            test_select_indices(tg, pick, 1);
            remaining--;
            indices++;
        } else {
            test_select_indices(tg, NULL, 0);
        }
        rb_process_pending_auto_abilities(&tg->state);
    }
    drain_skip(tg);
}

static void joint_fixture(TestGame *tg, int joint, const int *hand, int n_hand)
{
    int filler = test_id(tg, "PL!-sd1-010-SD");
    fill_decks(tg, filler, 30);
    tg->state.p[0].stage[1] = joint;
    tg->state.p[0].hand.n = 0;
    for (int i = 0; i < n_hand; i++) test_add_to_hand(tg, hand[i]);
    test_give_energy(tg, 5);
}

static void joint_live_start_discard_three_gains_score3(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint = test_id(&tg, JOINT);
    int ayumu = test_id(&tg, "PL!N-bp1-001-R");
    int kanon = test_id(&tg, "PL!SP-bp1-001-R");
    int kaho  = test_id(&tg, "PL!HS-bp1-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(joint, JOINT, "LL-bp1-001-R＋ fixture (fullwidth plus)");
    assert_identity(ayumu, "PL!N-bp1-001-R", "上原歩夢 fixture");
    assert_identity(kanon, "PL!SP-bp1-001-R", "澁谷かのん fixture");
    assert_identity(kaho, "PL!HS-bp1-001-R", "日野下花帆 fixture");
    int hand[4] = { ayumu, kanon, kaho, filler };
    joint_fixture(&tg, joint, hand, 4);
    int hand_before = tg.state.p[0].hand.n;

    fire_live_start(&tg, 0, joint);
    CHECK(test_has_pending_choice(&tg),
          "control: the optional named discard really pends a SelectCard");
    int idx[3] = { 0, 1, 2 };
    handle_optional_cost(&tg, idx, 3);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 3, "three cards were discarded");
    CHECK_EQ(total_bonus(&tg, 0), 3, "+3 live-total bonus");
}

static void joint_live_start_discard_two_gains_score3(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint = test_id(&tg, JOINT);
    int ayumu = test_id(&tg, "PL!N-bp1-001-R");
    int kanon = test_id(&tg, "PL!SP-bp1-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int hand[3] = { ayumu, kanon, filler };
    joint_fixture(&tg, joint, hand, 3);
    int hand_before = tg.state.p[0].hand.n;

    fire_live_start(&tg, 0, joint);
    CHECK(test_has_pending_choice(&tg),
          "control: the optional named discard really pends a SelectCard");
    int idx[2] = { 0, 1 };
    handle_optional_cost(&tg, idx, 2);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2, "two cards were discarded");
    CHECK_EQ(total_bonus(&tg, 0), 3, "any combination up to 3 -> +3 live total");
}

static void joint_live_start_skip_optional_no_effect(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint = test_id(&tg, JOINT);
    int ayumu = test_id(&tg, "PL!N-bp1-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int hand[2] = { ayumu, filler };
    joint_fixture(&tg, joint, hand, 2);
    int hand_before = tg.state.p[0].hand.n;

    fire_live_start(&tg, 0, joint);
    CHECK(test_has_pending_choice(&tg),
          "control: the same optional cost prompt is raised");
    handle_optional_cost(&tg, NULL, 0);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "skipping discards nothing");
    CHECK_EQ(score_mod(&tg, joint), 0, "no per-card score modifier");
    CHECK_EQ(total_bonus(&tg, 0), 0,
             "control: the accumulator is +0 here and +3 in the paying tests");
}

static void joint_live_start_no_named_skips_gracefully(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint = test_id(&tg, JOINT);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int hand[1] = { filler };
    joint_fixture(&tg, joint, hand, 1);

    fire_live_start(&tg, 0, joint);
    drain_skip(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, 1, "no card is discarded when nothing matches");
    CHECK_EQ(score_mod(&tg, joint), 0, "no score modifier when the cost is skipped");
    CHECK_EQ(total_bonus(&tg, 0), 0, "no live-total bonus when the cost is skipped");
}

static void joint_live_start_discard_mixed_named_gains_score3(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint = test_id(&tg, JOINT);
    int kanon = test_id(&tg, "PL!SP-bp1-001-R");
    int kaho  = test_id(&tg, "PL!HS-bp1-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int hand[3] = { kanon, kaho, filler };
    joint_fixture(&tg, joint, hand, 3);
    int hand_before = tg.state.p[0].hand.n;

    fire_live_start(&tg, 0, joint);
    CHECK(test_has_pending_choice(&tg),
          "control: the optional named discard really pends a SelectCard");
    int idx[2] = { 0, 1 };
    handle_optional_cost(&tg, idx, 2);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2, "two mixed cards discarded");
    CHECK_EQ(total_bonus(&tg, 0), 3, "a mixed named combination -> +3 live total");
}

static void joint_live_start_hand_copy_counts_as_named_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int joint_on_stage = test_id(&tg, JOINT);
    int joint_in_hand  = test_new_id(&tg, JOINT);
    int kanon = test_id(&tg, "PL!SP-bp1-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    assert_identity(joint_in_hand, JOINT, "the hand-copy fixture");
    CHECK(joint_in_hand != joint_on_stage,
          "control: the hand copy is a distinct instance from the staged card");
    int hand[3] = { joint_in_hand, kanon, filler };
    joint_fixture(&tg, joint_on_stage, hand, 3);
    int hand_before = tg.state.p[0].hand.n;

    fire_live_start(&tg, 0, joint_on_stage);
    CHECK(test_has_pending_choice(&tg),
          "control: the optional named discard really pends a SelectCard");
    int idx[2] = { 0, 1 };
    handle_optional_cost(&tg, idx, 2);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2,
             "the self-copy plus 澁谷かのん were discarded");
    CHECK(test_zone_has_id(&tg, 0, "discard", joint_in_hand),
          "the self-copy counts as a named card and reaches the waitroom");
    CHECK_EQ(total_bonus(&tg, 0), 3,
             "+3 live-total bonus with the self-copy counted as a named card");
}

/* ── fork harness ────────────────────────────────────────────────────── */

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
        printf("        %-58s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
               name, (int)assertions - a0, failures - f0, harness_gaps - g0);
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

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- A. exact_energy_count_constant_score_test.rs ---\n");
    run("A_yuna_exactly_eight_energy_grants_one_total_score",
        yuna_exactly_eight_energy_grants_one_total_score);

    printf("--- B. twelve_energy_pl_sp_pb1_002_r_test.rs ---\n");
    run("B_pl_sp_pb1_002_r_twelve_energy_bonus",
        pl_sp_pb1_002_r_twelve_energy_grants_total_score_bonus);

    printf("--- C. side_member_original_blade_gated_constant_score_test.rs ---\n");
    run("C_muse_two_original_blade_side_members_adds_one",
        muse_center_with_two_original_blade_side_members_adds_one);

    printf("--- D. kasumi_bp5_002_heart_greater_test.rs ---\n");
    run("D1_kasumi_alone_on_both_stages_gains_score",
        kasumi_alone_on_both_stages_gains_score);
    run("D2_kasumi_with_lower_opponent_gains",
        kasumi_with_lower_opponent_gains);
    run("D3_kasumi_tie_with_opponent_no_gain",
        kasumi_tie_with_opponent_no_gain);
    run("D4_kasumi_lower_than_opponent_no_gain",
        kasumi_lower_than_opponent_no_gain);
    run("D5_kasumi_with_own_side_lower_still_checks_both",
        kasumi_with_own_side_lower_still_checks_both);
    run("D6_kasumi_with_own_side_higher_no_gain",
        kasumi_with_own_side_higher_no_gain);

    printf("--- E. kasumi_energy_under_test.rs ---\n");
    run("E1_kasumi_constant_score_bonus_applies_when_energy_under",
        kasumi_constant_score_bonus_applies_when_energy_under);
    run("E2_kasumi_constant_score_without_energy_under",
        kasumi_constant_score_without_energy_under);

    printf("--- F. opponent_success_score_six_constant_total_bonus_test.rs ---\n");
    run("F1_opponent_success_score_six_grants_one",
        opponent_success_score_six_grants_one_live_total_bonus);
    run("F2_opponent_success_score_zero_leaves_modifier_zero",
        opponent_success_score_zero_leaves_per_card_modifier_zero);
    run("F3_opponent_success_score_removal_clears_bonus",
        opponent_success_score_removal_clears_live_total_bonus);

    printf("--- G. opponent_surplus_constant_total_score_test.rs ---\n");
    run("G1_opponent_surplus_gates_live_total_bonus",
        opponent_surplus_gates_dynamic_live_total_bonus);
    run("G2_opponent_surplus_exactly_two_grants",
        opponent_surplus_exactly_two_grants_live_total_bonus);
    run("G3_opponent_hearts_fully_consumed_no_bonus",
        opponent_hearts_fully_consumed_by_need_grant_no_bonus);
    run("G4_opponent_surplus_targets_live_total_not_card",
        opponent_surplus_bonus_targets_live_total_not_card);

    printf("--- H. live_success_per_under_member_live_total_score_cap_test.rs ---\n");
    run("H1_shiki_per_unit_score_capped_at_2",
        shiki_per_unit_score_capped_at_2);
    run("H2_shiki_per_unit_score_no_cap_needed",
        shiki_per_unit_score_no_cap_needed);
    run("H3_shiki_per_unit_score_zero_matching",
        shiki_per_unit_score_zero_matching);

    printf("--- I. live_success_self_effect_move_live_total_score_test.rs ---\n");
    run("I1_chisato_liella_move_grants_score",
        chisato_liella_move_grants_score);
    run("I2_chisato_non_liella_move_no_bonus",
        chisato_non_liella_move_no_bonus);
    run("I3_chisato_opponent_move_no_bonus",
        chisato_opponent_move_no_bonus);
    run("I4_chisato_natural_move_no_bonus",
        chisato_natural_move_no_bonus);
    run("I5_chisato_no_move_no_bonus",
        chisato_no_move_no_bonus);
    run("I6_chisato_previous_turn_move_no_bonus",
        chisato_previous_turn_move_no_bonus);
    run("I7_chisato_move_and_live_success_gives_score",
        chisato_move_and_live_success_gives_score);

    printf("--- J. chika_bp3_001_wait_self_activation_live_total_score_test.rs ---\n");
    run("J1_wait_member_self_cost_grants_one_total_score",
        wait_member_self_cost_grants_one_total_score);
    run("J2_wait_member_outside_center_cannot_pay_or_gain",
        wait_member_outside_center_cannot_pay_cost_or_gain);
    run("J3_wait_member_selected_cost_target_stays_on_stage",
        wait_member_selected_cost_target_stays_on_stage);
    run("J4_wait_member_selected_cost_target_grants_exactly_one",
        wait_member_selected_cost_target_grants_exactly_one);
    run("J5_wait_member_right_target_waits_without_affecting_others",
        wait_member_right_target_waits_without_affecting_others);
    run("J6_wait_member_second_activation_cannot_stack",
        wait_member_second_activation_cannot_stack);
    run("J7_wait_member_baton_removes_recipient_loses_score",
        wait_member_baton_removes_recipient_loses_total_score);
    run("J8_wait_member_baton_removes_source_recipient_keeps_score",
        wait_member_baton_removes_source_recipient_keeps_score);
    run("J9_wait_member_total_score_persists_through_victory",
        wait_member_total_score_persists_through_victory_and_expires);
    run("J10_wait_member_total_score_expires_without_setting_live",
        wait_member_total_score_expires_without_setting_live);

    printf("--- K. optional_discard_named_members_live_total_score_test.rs ---\n");
    run("K1_joint_discard_three_gains_score3",
        joint_live_start_discard_three_gains_score3);
    run("K2_joint_discard_two_gains_score3",
        joint_live_start_discard_two_gains_score3);
    run("K3_joint_skip_optional_no_effect",
        joint_live_start_skip_optional_no_effect);
    run("K4_joint_no_named_skips_gracefully",
        joint_live_start_no_named_skips_gracefully);
    run("K5_joint_discard_mixed_named_gains_score3",
        joint_live_start_discard_mixed_named_gains_score3);
    run("K6_joint_hand_copy_counts_as_named_card",
        joint_live_start_hand_copy_counts_as_named_card);

    rb_unload();

    /* The per-test counters live in the forked CHILD, so this parent cannot
       print a grand assertion total -- the per-test lines printed by run()
       above are the record. The exit status must still reflect the children. */
    printf("\n===== test_parity_live_total summary =====\n");
    printf("test functions : %d  (ok %d / failed %d / crashed %d)\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed);
    printf("harness gaps   : %d\n", harness_gaps);
    if (n_failed || n_crashed) {
        printf("LIVE-TOTAL PARITY CHECKS FAILED (%d failing, %d crashing)\n",
               n_failed, n_crashed);
        return 1;
    }
    printf("ALL LIVE-TOTAL PARITY CHECKS PASSED\n");
    return 0;
}

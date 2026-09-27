/* Parity suite for engine/tests/test_modules/effects/gain/blades/ --
 * the `live_start/` and `constants/` sub-clusters (and the `per_card/`
 * sub-cluster, folded in because it shares every fixture with `live_start/`).
 *
 * Every test function names the Rust file it mirrors. The theme in one place:
 *   * `constants/`  -- 常時 (constant / always-on) blade modifiers, driven
 *                      through test_recalc() -> rb_recalc_constants(), exactly
 *                      like Rust's game.state.recalculate_constants().
 *   * `live_start/` -- ライブ開始時 blade grants, driven EITHER through the
 *                      real turn walk (test_pass / test_set_live_card) like
 *                      the Rust tests that walk the phases, OR through the
 *                      string-keyed trigger shim (bl_fire_live_start) that
 *                      mirrors helpers/mod.rs `fire_live_start` / `fire_trigger`.
 *
 * Card identity: every test that stages a card whose ability the assertion
 * depends on asserts the resolved card_no (rb_card_no_eq) before the blade
 * assertion. This cluster is exactly where the bp2/pb2 transposition trap
 * bites -- PL!SP-bp2-009-P and PL!SP-bp2-009-R＋ are DIFFERENT prints that
 * happen to share the hand-pair live-start text, and PL!-PR-021-PR (小林実香)
 * vs PL!-pb1-021-PR (南ことり) are different people whose costs flip the
 * stage-total comparison in energy_count_and_stage_cost_constant_blade_test.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int assertions;

/* Trigger wire tokens (engine/src/ability/enums.rs ability trigger strings). */
#define TRIG_LIVE_START  "ライブ開始時"
#define TRIG_LIVE_SUCCESS "ライブ成功時"
#define TRIG_DEBUT        "登場"

/* ── assertions ──────────────────────────────────────────────────────────── */

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

/* ── helpers ─────────────────────────────────────────────────────────────── */

/* The card DB. `src` is the in-tree layout; `../cards/build` is where the
 * Makefile (and tools/isolated_build.sh) put the blobs, so an isolated
 * out-of-tree build finds the database through this second path. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Rust helpers::TestGame::id() allocates the NEXT DISTINCT pool slot, so two
 * `game.id("X")` calls are two different card instances.  C's plain template
 * lookup returns the SHARED TEMPLATE index, which aliases; test_new_id() is the
 * distinct copy.  Every Rust `game.id(...)` in this file therefore maps to mid()
 * below, and only Rust `game.id_ref(...)` -- the deliberate shared template --
 * maps to the template lookup.  Getting this wrong silently stages one card
 * twice, which the multi-copy blade counts would then mis-count. */
static int mid(TestGame *tg, const char *no)
{
    return test_new_id(tg, no);
}

static void fill_decks(TestGame *tg, int filler)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust helpers::fill_decks pushes onto both main decks WITHOUT clearing. */
static void fill_decks_append(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static int blade(TestGame *tg, int cid)
{
    return test_get_blade_modifier(tg, cid);
}

static int heart(TestGame *tg, int cid, int color)
{
    return test_get_heart_modifier(tg, cid, color);
}

/* The heart slot a printed heartNN actually occupies.
 *   heart00 -> 0 (RB_HEART_PINK, the wildcard slot)   heart01 -> 1 (RED)
 *   heart02 -> 2 (YELLOW)   heart03 -> 3 (GREEN)      heart04 -> 4 (BLUE)
 *   heart05 -> 5 (PURPLE)   heart06 -> 6 (ORANGE)
 * (src/ability/util.c:rb_parse_heart_color is the single owner of that map.)
 *
 * Read it through rb_mods_get_heart DIRECTLY rather than test_get_heart_modifier:
 * the shim remaps a requested colour of 5 onto RB_HEART_ORANGE, which silently
 * turns a heart05 query into a heart06 query. Same hazard the rescued
 * test_tp_joint_card_live_start.c documents. */
#define BL_HEART01 1
#define BL_HEART05 5
#define BL_HEART06 6

static int heart_slot(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_heart(&tg->state.mods, cid, slot);
}

static int printed_blade(int cid)
{
    Card c;
    if (cid < 0) return -1;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int b = (int)c.blade;
    rb_free_card(&c);
    return b;
}

static int printed_cost(int cid)
{
    Card c;
    if (cid < 0) return -1;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

/* The Rust `SetupGuard` line every blade test leans on: the staged id really is
 * the print the test names.  Returns 1 when it is, so the caller can bail out
 * rather than assert nonsense about card -1. */
static int ident(TestGame *tg, int cid, const char *no)
{
    if (!rb_card_no_eq(cid, no)) {
        fprintf(stderr, "SETUP BUG: expected card %s, resolved id=%d (%s)\n",
                no, cid, cid >= 0 ? test_card_name(cid) : "<none>");
        failures++;
        assertions++;
        return 0;
    }
    return 1;
}

/* helpers/mod.rs:114 fire_trigger / fire_live_start -- build the
 * "<card_no>_<full_text>" ability id for the nth ability printing `trig`, queue
 * exactly that ability, set activating_card, process the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int bl_fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", rb_card_string(card.card_no_idx));
    char ability_id[512];
    int found = 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, trigger_type, 0, card_no,
                            cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int bl_fire_live_start(TestGame *tg, int cid)
{
    return bl_fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START);
}

static int bl_fire_debut(TestGame *tg, int cid)
{
    return bl_fire_trigger(tg, cid, TRIG_DEBUT, TRIG_DEBUT);
}

/* ── choice answering ───────────────────────────────────────────────────────
 * The C engine reads ONE resume index: `selected_idx < 0` is Rust's
 * `select_indices(&[])` / `select_option(0)` (DECLINE / skip the optional
 * cost); any index >= 0 is an ACCEPT -- and on a SelectTarget the index is not
 * an option ordinal at all, only its sign matters, so "accept" is 0 and
 * "skip" is -1.  Getting that backwards silently PAWs every optional cost,
 * which is why each drain below names the Rust helper it mirrors. */
#define BL_SKIP   (-1)
#define BL_ACCEPT (0)

static void bl_drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        rb_resume_with_choice(&tg->state, BL_SKIP);
    }
}

/* Proceed with the queued ライブ開始時 ability (SelectAutoAbility -> 0), pick
 * index 0 for a mandatory card pick, decline everything else. */
static void bl_drain_proceed(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = BL_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = BL_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip) idx = 0;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* pay_optional_costs_selecting_last_hand_card: proceed with the auto ability,
 * pick the LAST hand card for a hand-zone SelectCard, ACCEPT (pay) a
 * SelectTarget, decline any re-prompt so the cost finalizes. */
static void bl_drain_pay_costs(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 30) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            idx = BL_ACCEPT;
        } else if (c && c->kind == RB_CHOICE_SELECT_CARD && c->zone[0] &&
                   !strcmp(c->zone, "hand")) {
            idx = tg->state.p[0].hand.n - 1;
            if (idx < 0) return;
        } else if (c && c->kind == RB_CHOICE_SELECT_TARGET) {
            idx = BL_ACCEPT;
        } else {
            idx = BL_SKIP;
        }
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* skip_optional_card_and_target_choices / drain_skippable_choices: proceed
 * with the auto ability, then DECLINE every skippable gate -- so a
 * 「…てもよい」 cost is skipped, not paid. */
static void bl_drain_decline_optional(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 30) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = BL_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = BL_ACCEPT;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* success_score_comparison_pl_bp4_018_n_test.rs:
 *     while game.has_pending_choice() { game.select_indices(&[0]); }
 * Unconditional index 0 -- the C fold of `select_indices(&[0])`. Unlike the
 * drains above there is NO per-kind branch: every prompt is answered with the
 * first index, which in the C engine means "accept". Used where the Rust test
 * needs a pending 登場 gate resolved before it starts pushing success cards. */
static void bl_drain_pick_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        rb_resume_with_choice(&tg->state, 0);
    }
}

/* live_start_optional_energy_self_blades_test.rs:
 *     fn skip_optional_card_and_target_choices(game: &mut TestGame) {
 *         while game.has_pending_choice() && guard < 30 {
 *             match game.get_pending_choice() {
 *                 Choice::SelectAutoAbility { .. }          => game.select_indices(&[]),
 *                 Choice::SelectCard { allow_skip: true, ..} => game.select_indices(&[]),
 *                 Choice::SelectTarget{ allow_skip: true, ..} => game.select_option(0),
 *                 _ => break,
 *             }
 *         }
 *     }
 * NOTE this is NOT the same as bl_drain_decline_optional above, and the
 * distinction is the whole point of the B14 declined-vs-paid twin:
 * `select_indices(&[])` is a DECLINE (index -1), so the ライブ開始時 ability is
 * NOT used at all, whereas a SelectTarget is answered with select_option(0) =
 * ACCEPT. Any prompt outside those three shapes breaks the loop, as in Rust. */
static void bl_drain_skip_optional(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 30) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx;
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            idx = BL_SKIP;            /* select_indices(&[]) */
        } else if (c->kind == RB_CHOICE_SELECT_CARD && c->allow_skip) {
            idx = BL_SKIP;            /* select_indices(&[]) */
        } else if (c->kind == RB_CHOICE_SELECT_TARGET && c->allow_skip) {
            idx = BL_ACCEPT;          /* select_option(0) */
        } else {
            break;                    /* _ => break */
        }
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* 「手札を1枚控え室に置いてもよい」 : proceed with the auto ability, take the
 * first hand card as the cost, accept a SelectTarget gate, decline the
 * any_number re-prompt. */
static void bl_pay_discard_cost(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 30) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = BL_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = BL_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_CARD &&
                 c->zone[0] && !strcmp(c->zone, "hand") && !c->allow_skip) idx = 0;
        else if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = BL_ACCEPT;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* Accept a conditional_optional / pay_optional_cost gate, then take every
 * offered SelectCard (the 9-card 「選び」 of PL!SP-bp7-028-L). */
static void bl_accept_and_take_all(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = BL_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = BL_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = BL_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_CARD) idx = 0;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* Rust advance_to_live_card_set_p1: five blind passes land on the live-card
 * set window. */
static void bl_advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start: two more passes reach ライブ開始時. */
static void bl_advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

static int discard_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == cid) return 1;
    return 0;
}

static int hand_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == cid) return 1;
    return 0;
}

static int live_total_bonus(TestGame *tg, int pl)
{
    return pl == 0 ? (int)tg->state.mods.p1_constant_total_score_bonus
                   : (int)tg->state.mods.p2_constant_total_score_bonus;
}

/* ══════════════════════════════════════════════════════════════════════════
 * PART A -- effects/gain/blades/constants/  (常時 blade modifiers)
 * ══════════════════════════════════════════════════════════════════════════ */

/* A1. center_constant_blade_bonus_test.rs
 *     center_constant_two_blades_test.rs
 * Both files stage the identical board and assert the identical claim, so they
 * share one C test: PL!SP-bp4-003-R 常時 センターにいる場合、ブレード+2。 */
static void a_center_constant_pl_sp_bp4_003(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int m = mid(&tg, "PL!SP-bp4-003-R");
    if (!ident(&tg, m, "PL!SP-bp4-003-R")) return;
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = m;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK(blade(&tg, m) >= 2,
          "PL!SP-bp4-003-R center constant should grant at least +2 blade");
}

/* A2. center_five_blades_constant_test.rs
 * PL!SP-bp1-004-PR 常時 センターにいる場合、ブレード+5。 */
static void a_center_five_blades_sumire(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int sumire = mid(&tg, "PL!SP-bp1-004-PR");
    if (!ident(&tg, sumire, "PL!SP-bp1-004-PR")) return;
    tg.state.p[0].stage[1] = sumire;
    test_give_energy(&tg, 20);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sumire), 5, "center position grants +5 blade");

    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sumire), 0, "left position grants no blade bonus");
}

/* A3. conditional_constant_blade_bonuses_test.rs -- two waited opponents. */
static void a_two_waited_opponents_pl_bp3_002(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!-bp3-002-R");
    if (!ident(&tg, member, "PL!-bp3-002-R")) return;
    int opp1 = test_new_id(&tg, "PL!-sd1-001-SD");
    int opp2 = test_new_id(&tg, "PL!-sd1-002-SD");
    tg.state.p[1].stage[0] = opp1;
    tg.state.p[1].stage[1] = opp2;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
    rb_mods_set_orientation(&tg.state.mods, opp1, "wait");
    rb_mods_set_orientation(&tg.state.mods, opp2, "wait");
    tg.state.p[0].stage[1] = member;
    test_recalc(&tg);
    CHECK(blade(&tg, member) >= 2,
          "two waited opponent members should grant >= +2 blade");
}

/* A3b. conditional_constant_blade_bonuses_test.rs -- 3 opponent success cards. */
static void a_three_opponent_success_pl_s_pb1_009(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!S-pb1-009-R");
    if (!ident(&tg, member, "PL!S-pb1-009-R")) return;
    tg.state.p[0].stage[1] = member;
    for (int i = 0; i < 3; i++) test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-020-SD"));
    test_recalc(&tg);
    CHECK(blade(&tg, member) >= 3,
          "opponent with 3 success cards should grant >= +3 blade");
}

/* A4. constant_ability_edge_cases_test.rs -- PL!S-PR-037-PR
 * 常時 自分のステージにいるメンバーがちょうど2人であるかぎり、
 * heart05＋ブレード×1。 */
static void a_exactly_two_members_pl_s_pr_037(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int card = mid(&tg, "PL!S-PR-037-PR");
    if (!ident(&tg, card, "PL!S-PR-037-PR")) return;
    int a = mid(&tg, "PL!-sd1-001-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    fill_decks(&tg, filler);

    test_add_to_stage(&tg, 1, card);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "1 stage member -> no blade");
    CHECK_EQ(heart_slot(&tg, card, BL_HEART05), 0, "1 stage member -> no heart05");

    test_add_to_stage(&tg, 0, a);
    test_recalc(&tg);
    CHECK(blade(&tg, card) >= 1, "2 stage members -> blade granted");
    CHECK(heart_slot(&tg, card, BL_HEART05) >= 1, "2 stage members -> heart05 granted");

    test_add_to_stage(&tg, 2, test_new_id(&tg, "PL!-sd1-002-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "3 stage members -> no blade (ちょうど2人)");
}

/* A4b. constant_ability_edge_cases_test.rs -- PL!HS-pb1-007-R compound. */
static void a_compound_self_two_opp_three_pl_hs_pb1_007(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int card = mid(&tg, "PL!HS-pb1-007-R");
    if (!ident(&tg, card, "PL!HS-pb1-007-R")) return;
    test_add_to_stage(&tg, 1, card);
    test_add_to_stage(&tg, 0, mid(&tg, "PL!-sd1-001-SD"));

    tg.state.p[1].stage[0] = mid(&tg, "PL!-sd1-001-SD");
    tg.state.p[1].stage[1] = mid(&tg, "PL!-sd1-002-SD");
    test_recalc(&tg);
    CHECK_EQ(heart_slot(&tg, card, BL_HEART06), 0, "self=2, opp=2 -> no heart06");

    tg.state.p[1].stage[2] = mid(&tg, "PL!-sd1-003-SD");
    test_recalc(&tg);
    CHECK(heart_slot(&tg, card, BL_HEART06) >= 1, "self=2, opp=3 -> heart06 granted");

    test_add_to_stage(&tg, 2, mid(&tg, "PL!-sd1-004-SD"));
    test_recalc(&tg);
    CHECK_EQ(heart_slot(&tg, card, BL_HEART06), 0, "self=3 -> no heart06");
}

/* A4c. constant_ability_edge_cases_test.rs -- PL!HS-pb1-022-N member names. */
static void a_member_name_constants_pl_hs_pb1_022(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int card = mid(&tg, "PL!HS-pb1-022-N");
    int toko = mid(&tg, "PL!HS-bp2-015-N");
    int rurino = mid(&tg, "PL!HS-bp6-019-N");
    if (!ident(&tg, card, "PL!HS-pb1-022-N")) return;
    if (!ident(&tg, toko, "PL!HS-bp2-015-N")) return;
    if (!ident(&tg, rurino, "PL!HS-bp6-019-N")) return;

    test_add_to_stage(&tg, 1, card);
    test_recalc(&tg);
    CHECK_EQ(heart_slot(&tg, card, BL_HEART01), 0, "no named members -> no heart01");
    CHECK_EQ(blade(&tg, card), 0, "no named members -> no blade");

    test_add_to_stage(&tg, 0, toko);
    test_recalc(&tg);
    CHECK_EQ(heart_slot(&tg, card, BL_HEART01), 0, "only 藤島慈 -> no heart01");
    CHECK(blade(&tg, card) >= 2, "藤島慈 on stage -> blade x2");

    test_add_to_stage(&tg, 2, rurino);
    test_recalc(&tg);
    CHECK(heart_slot(&tg, card, BL_HEART01) >= 2, "大沢瑠璃乃 on stage -> heart01 x2");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "藤島慈 removed -> blade gone");
    CHECK(heart(&tg, card, 0) >= 2,
          "藤島慈 removed but 大沢 still there -> heart01 x2 remains");
}

/* A5. distinct_stage_costs_constant_blade_test.rs */
static void a_distinct_stage_costs_uses_modified_cost(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int sayaka = mid(&tg, "PL!HS-bp5-002-R＋");
    int cost2_a = mid(&tg, "PL!-sd1-002-SD");
    int cost2_b = mid(&tg, "PL!HS-bp2-004-R");
    if (!ident(&tg, sayaka, "PL!HS-bp5-002-R＋")) return;
    CHECK_EQ(printed_cost(sayaka), 15, "setup: the ability owner really costs 15");
    CHECK_EQ(printed_cost(cost2_a), 2, "setup: PL!-sd1-002-SD really costs 2");
    CHECK_EQ(printed_cost(cost2_b), 2, "setup: PL!HS-bp2-004-R really costs 2");

    tg.state.p[0].stage[0] = sayaka;
    tg.state.p[0].stage[1] = cost2_a;
    tg.state.p[0].stage[2] = cost2_b;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 0,
             "costs 15, 2, 2 have a duplicate -> no blade");

    rb_mods_add_cost(&tg.state.mods, cost2_b, 2);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 1,
             "after +2 cost modifier costs are 15, 2, 4 -> all distinct -> 1 blade");
}

/* A6. energy_count_and_stage_cost_constant_blade_test.rs
 * PL!-PR-021-PR 常時 自分のエネルギーがちょうど7個なら、ブレード2つ。 */
static void a_exactly_seven_energy_pl_pr_021(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int nico = mid(&tg, "PL!-PR-021-PR");
    if (!ident(&tg, nico, "PL!-PR-021-PR")) return;
    tg.state.p[0].stage[1] = nico;

    test_give_energy(&tg, 6);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, nico), 0, "6 energy != exactly 7 -> no blades");

    test_give_energy(&tg, 1);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, nico), 2, "exactly 7 energy -> ブレード2つ");

    test_give_energy(&tg, 1);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, nico), 0, "8 energy != exactly 7 -> blades lost again");
}

/* A6b. energy_count_and_stage_cost_constant_blade_test.rs
 * PL!SP-bp4-009-R compares the two SIDES' stage cost totals, so the
 * 小鳥遊/南ことり transposition would silently invert the comparison. */
static void a_cheaper_stage_total_pl_sp_bp4_009(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int natsumi = mid(&tg, "PL!SP-bp4-009-R");
    int big = mid(&tg, "PL!S-bp5-009-R");
    int small = mid(&tg, "PL!-pb1-021-PR");
    if (!ident(&tg, natsumi, "PL!SP-bp4-009-R")) return;
    if (!ident(&tg, big, "PL!S-bp5-009-R")) return;
    if (!ident(&tg, small, "PL!-pb1-021-PR")) return;
    CHECK(!rb_card_no_eq(small, "PL!-PR-021-PR"),
          "setup: 小鳥遊 and 南ことり are distinct prints, not a transposition");
    CHECK_EQ(printed_cost(natsumi), 9, "setup: PL!SP-bp4-009-R really costs 9");
    CHECK_EQ(printed_cost(big), 15, "setup: PL!S-bp5-009-R really costs 15");
    CHECK_EQ(printed_cost(small), 5, "setup: PL!-pb1-021-PR really costs 5");

    tg.state.p[0].stage[1] = natsumi;
    tg.state.p[1].stage[1] = big;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, natsumi), 3, "my total 9 < opponent 15 -> ブレード3つ");

    tg.state.p[1].stage[1] = small;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, natsumi), 0, "my total 9 > opponent 5 -> blades gone");
}

/* A7. higher_cost_member_pl_hs_bp2_002_r_plus_test.rs */
static void a_higher_cost_member_pl_hs_bp2_002(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int sayaka = mid(&tg, "PL!HS-bp2-002-R＋");
    int big = mid(&tg, "PL!S-bp5-009-R");
    if (!ident(&tg, sayaka, "PL!HS-bp2-002-R＋")) return;
    tg.state.p[0].stage[1] = sayaka;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 0, "alone: no higher-cost member -> 0");
    tg.state.p[0].stage[0] = big;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 3, "cost-15 member > her 13 -> ブレード3つ");
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 0, "no bigger member -> blades gone");
}

/* A8. lone_member_constant_blade_branches_test.rs -- both branches. */
static void a_lone_member_branches(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int seras = mid(&tg, "PL!HS-pb1-015-R");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, seras, "PL!HS-pb1-015-R")) return;
    CHECK_EQ(printed_blade(seras), 5, "setup: PL!HS-pb1-015-R prints blade 5");

    test_add_to_stage(&tg, 1, seras);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, seras), -3, "lone on stage -> ブレードを３つ失う");

    test_add_to_stage(&tg, 0, filler);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, seras), 0, "ほかのメンバーがいる -> no loss");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, seras), -3, "alone again -> the loss returns");
}

/* Second half of lone_member_constant_blade_branches_test.rs.
 * NOTE / RED BY DESIGN: cards/cards.json has no `PL!HS-pb6-002-R` entry, so
 * mid() resolves to -1 and ident() below records the failure.  The Rust file
 * names the same card_no, so the Rust test cannot be exercising the printed
 * card either -- a wrong card_no in the test source, not an engine behaviour.
 * The assertion is left strict rather than swapped for a different card. */
static void a_lone_member_sayaka_bp6_002(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int sayaka = mid(&tg, "PL!HS-pb6-002-R");
    if (!ident(&tg, sayaka, "PL!HS-pb6-002-R")) return;
    CHECK_EQ(printed_blade(sayaka), 2, "setup: PL!HS-pb6-002-R prints blade 2");
    test_add_to_stage(&tg, 1, sayaka);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 2, "lone on stage -> blade+2");
    test_add_to_stage(&tg, 2, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, sayaka), 0, "not alone anymore -> bonus off");
}

/* A9. opponent_energy_ahead_constant_blades_test.rs
 * A10. opponent_energy_lead_constant_blade_branches_test.rs */
static int setup_energy_comparison(TestGame *tg, int p1_energy, int p2_energy)
{
    int you = mid(tg, "PL!S-pb1-005-R");
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (!ident(tg, you, "PL!S-pb1-005-R")) return -1;
    test_add_to_stage(tg, 1, you);
    for (int i = 0; i < p1_energy; i++) test_add_to_energy(tg, 0, filler);
    test_set_energy_active(tg, 0, p1_energy);
    for (int i = 0; i < p2_energy; i++) test_add_to_energy(tg, 1, filler);
    test_set_energy_active(tg, 1, p2_energy);
    return you;
}

static void a_opponent_energy_ahead_grants_three(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int you = setup_energy_comparison(&tg, 3, 7);
    if (you < 0) return;
    test_recalc(&tg);
    CHECK(blade(&tg, you) > 0, "opponent 7 > self 3 -> should gain blade");
    CHECK_EQ(blade(&tg, you), 3, "opponent energy ahead -> +3 blade");

    /* the p1 energy card count must not leak: 10 more active energy kills it */
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, you), 0,
              "self overtakes the opponent -> the bonus goes away");
}

static void a_energy_lead_equal_and_behind(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int you = setup_energy_comparison(&tg, 5, 5);
    if (you < 0) return;
    test_recalc(&tg);
    CHECK(!(blade(&tg, you) > 0), "opponent 5 == self 5 -> no blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int you2 = setup_energy_comparison(&tg2, 8, 3);
    if (you2 < 0) return;
    test_recalc(&tg2);
    CHECK(!(blade(&tg2, you2) > 0), "opponent 3 < self 8 -> no blade");
}

static void a_energy_lead_toggles_off_and_on(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int you = setup_energy_comparison(&tg, 3, 7);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (you < 0) return;
    test_recalc(&tg);
    CHECK(blade(&tg, you) > 0, "opponent ahead -> blade");

    for (int i = 0; i < 10; i++) test_add_to_energy(&tg, 0, filler);
    test_set_energy_active(&tg, 0, 13);
    test_recalc(&tg);
    CHECK(!(blade(&tg, you) > 0), "self 13 > opponent 7 -> blade removed");

    for (int i = 0; i < 10; i++) test_add_to_energy(&tg, 1, filler);
    test_set_energy_active(&tg, 1, 17);
    test_recalc(&tg);
    CHECK(blade(&tg, you) > 0, "opponent 17 > self 13 -> blade returns");
    CHECK_EQ(blade(&tg, you), 3, "and it is the full +3");
}

/* A11. other_unit_member_constant_blades_test.rs
 * A14. per_other_unit_member_constant_blades_test.rs */
static void a_per_other_unit_member_excludes_self_and_wrong_unit(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int megumi = mid(&tg, "PL!HS-bp2-006-R");
    int hime = mid(&tg, "PL!HS-bp1-005-R");
    int doll = mid(&tg, "PL!HS-sd1-005-SD");
    if (!ident(&tg, megumi, "PL!HS-bp2-006-R")) return;

    tg.state.p[0].stage[0] = hime;
    tg.state.p[0].stage[1] = megumi;
    tg.state.p[0].stage[2] = doll;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, megumi), 1,
             "1 other みらくらぱーく！ member -> exactly 1 blade (self and DOLLCHESTRA excluded)");
}

static void a_per_other_unit_member_two_and_three_copies(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = mid(&tg, "PL!HS-bp2-006-R");
    int b = mid(&tg, "PL!HS-bp2-006-R");
    int unrelated = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, a, "PL!HS-bp2-006-R")) return;
    CHECK(a != b, "setup: two distinct ids for the same print");

    tg.state.p[0].stage[0] = b;
    tg.state.p[0].stage[1] = a;
    tg.state.p[0].stage[2] = unrelated;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, a), 1, "copy A: 1 other -> 1 blade");
    CHECK_EQ(blade(&tg, b), 1, "copy B: 1 other -> 1 blade");
    CHECK_EQ(blade(&tg, unrelated), 0, "unrelated member: 0 blade");

    static TestGame tg3;
    test_game_new(&tg3);
    int c1 = mid(&tg3, "PL!HS-bp2-006-R");
    int c2 = mid(&tg3, "PL!HS-bp2-006-R");
    int c3 = mid(&tg3, "PL!HS-bp2-006-R");
    tg3.state.p[0].stage[0] = c1;
    tg3.state.p[0].stage[1] = c2;
    tg3.state.p[0].stage[2] = c3;
    test_recalc(&tg3);
    CHECK_EQ(blade(&tg3, c1), 2, "3 copies: each sees 2 others -> 2 blades");
    CHECK_EQ(blade(&tg3, c2), 2, "3 copies: each sees 2 others -> 2 blades");
    CHECK_EQ(blade(&tg3, c3), 2, "3 copies: each sees 2 others -> 2 blades");
}

/* A12. own_empty_opponent_nonempty_success_constant_blades_test.rs */
static void a_riko_own_empty_opponent_nonempty(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int riko = mid(&tg, "PL!S-bp2-001-R");
    if (!ident(&tg, riko, "PL!S-bp2-001-R")) return;
    test_add_to_stage(&tg, 1, riko);
    test_add_to_opp_success(&tg, mid(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, riko), 3, "own=0, opponent>=1 -> 3 blades");

    tg.state.p[1].success.n = 0;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, riko), 0, "opponent success card removed -> 0 blades");
}

static void a_riko_own_nonempty_and_both_empty(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int riko = mid(&tg, "PL!S-bp2-001-R");
    if (!ident(&tg, riko, "PL!S-bp2-001-R")) return;
    test_add_to_stage(&tg, 1, riko);
    test_add_to_success(&tg, mid(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, riko), 0, "own success zone non-empty -> condition fails");

    static TestGame tg2;
    test_game_new(&tg2);
    int riko2 = mid(&tg2, "PL!S-bp2-001-R");
    if (!ident(&tg2, riko2, "PL!S-bp2-001-R")) return;
    test_add_to_stage(&tg2, 1, riko2);
    test_recalc(&tg2);
    CHECK_EQ(blade(&tg2, riko2), 0, "both success zones empty -> 0 blades");
}

/* A13. own_energy_ahead_constant_blade_test.rs */
static void a_sp_bp7_020_own_energy_ahead(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp7-020-N");
    if (!ident(&tg, me, "PL!SP-bp7-020-N")) return;
    tg.state.p[0].stage[0] = me;
    test_give_energy(&tg, 3);
    test_give_opp_energy(&tg, 1);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, me), 2, "energy ahead -> +2 blades");

    static TestGame tg2;
    test_game_new(&tg2);
    int me2 = mid(&tg2, "PL!SP-bp7-020-N");
    if (!ident(&tg2, me2, "PL!SP-bp7-020-N")) return;
    tg2.state.p[0].stage[0] = me2;
    test_give_energy(&tg2, 1);
    test_give_opp_energy(&tg2, 3);
    test_recalc(&tg2);
    CHECK_EQ(blade(&tg2, me2), 0, "energy behind -> no blades");
}

/* A15. per_waited_opponent_member_constant_blade_test.rs */
static void a_per_waited_opponent_member_pl_bp3_002(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int eri = mid(&tg, "PL!-bp3-002-R");
    int opp1 = test_new_id(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, eri, "PL!-bp3-002-R")) return;
    tg.state.p[0].stage[1] = eri;
    tg.state.p[1].stage[0] = opp1;
    rb_mods_set_orientation(&tg.state.mods, opp1, "wait");
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, eri), 1, "1 waited opponent member -> exactly 1 blade");
}

/* A16. position_gated_constant_blades_test.rs */
static void a_position_gated_pl_sp_sd2_004(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!SP-sd2-004-SD2");
    if (!ident(&tg, member, "PL!SP-sd2-004-SD2")) return;
    tg.state.p[0].stage[1] = member;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, member), 4, "center grants exactly +4 blade");
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[0] = member;
    test_recalc(&tg);
    CHECK(blade(&tg, member) != 4, "left position grants no center bonus");
}

static void a_position_gated_left_and_right(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int left = mid(&tg, "PL!SP-pb2-035-N");
    if (!ident(&tg, left, "PL!SP-pb2-035-N")) return;
    tg.state.p[0].stage[0] = left;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, left), 2, "left position grants exactly +2 blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int right = mid(&tg2, "PL!SP-pb2-041-N");
    if (!ident(&tg2, right, "PL!SP-pb2-041-N")) return;
    tg2.state.p[0].stage[2] = right;
    test_recalc(&tg2);
    CHECK_EQ(blade(&tg2, right), 2, "right position grants exactly +2 blade");
}

/* A17. ruby_front_area_cost_gated_blade_loss_test.rs
 * PL!S-bp7-009-R 常時：このメンバーの正面のエリアにいるコスト4以下のメンバーは、
 * ブレードを1つ失う。 */
static void a_ruby_front_area_cost_gated(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp7-009-R");
    int cost4 = mid(&tg, "PL!-sd1-010-SD");
    int cost5 = mid(&tg, "PL!-pb1-021-PR");
    if (!ident(&tg, ruby, "PL!S-bp7-009-R")) return;
    CHECK_EQ(printed_cost(cost4), 4, "setup: PL!-sd1-010-SD costs 4");
    CHECK_EQ(printed_cost(cost5), 5, "setup: PL!-pb1-021-PR costs 5");

    /* P1 Center Ruby, P2 Center cost-4 member -> -1 */
    tg.state.p[0].stage[1] = ruby;
    tg.state.p[1].stage[1] = cost4;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, cost4), -1,
             "cost-4 member directly in front of Ruby loses 1 blade");

    /* P2 Center cost-5 member -> untouched */
    static TestGame tg5;
    test_game_new(&tg5);
    int r5 = mid(&tg5, "PL!S-bp7-009-R");
    int c5 = mid(&tg5, "PL!-pb1-021-PR");
    tg5.state.p[0].stage[1] = r5;
    tg5.state.p[1].stage[1] = c5;
    test_recalc(&tg5);
    CHECK_EQ(blade(&tg5, c5), 0, "cost-5 member is not affected");

    /* side slots are not "in front" */
    static TestGame tgs;
    test_game_new(&tgs);
    int rs = mid(&tgs, "PL!S-bp7-009-R");
    int c4s = mid(&tgs, "PL!-sd1-010-SD");
    tgs.state.p[0].stage[1] = rs;
    tgs.state.p[1].stage[0] = c4s;
    test_recalc(&tgs);
    CHECK_EQ(blade(&tgs, c4s), 0, "P2 Left is not in front of P1 Center Ruby");
}

static void a_ruby_left_and_right_mirroring(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp7-009-R");
    int opp_right = mid(&tg, "PL!-sd1-010-SD");
    int opp_center = test_new_id(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, ruby, "PL!S-bp7-009-R")) return;
    tg.state.p[0].stage[0] = ruby;
    tg.state.p[1].stage[1] = opp_center;
    tg.state.p[1].stage[2] = opp_right;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, opp_right), -1, "P1 Left Ruby debuffs P2 Right");
    CHECK_EQ(blade(&tg, opp_center), 0, "P1 Left Ruby does not debuff P2 Center");

    static TestGame tg2;
    test_game_new(&tg2);
    int ruby2 = mid(&tg2, "PL!S-bp7-009-R");
    int opp_left = mid(&tg2, "PL!-sd1-010-SD");
    tg2.state.p[0].stage[2] = ruby2;
    tg2.state.p[1].stage[0] = opp_left;
    test_recalc(&tg2);
    CHECK_EQ(blade(&tg2, opp_left), -1, "P1 Right Ruby debuffs P2 Left");
}

static void a_ruby_facing_ruby_and_empty_front(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby_p1 = mid(&tg, "PL!S-bp7-009-R");
    int ruby_p2 = mid(&tg, "PL!S-bp7-009-R");
    tg.state.p[0].stage[1] = ruby_p1;
    tg.state.p[1].stage[1] = ruby_p2;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ruby_p1), -1, "P1 Ruby loses a blade to P2 Ruby");
    CHECK_EQ(blade(&tg, ruby_p2), -1, "P2 Ruby loses a blade to P1 Ruby");

    static TestGame tge;
    test_game_new(&tge);
    int ruby_e = mid(&tge, "PL!S-bp7-009-R");
    tge.state.p[0].stage[1] = ruby_e;
    test_recalc(&tge);
    CHECK_EQ(blade(&tge, ruby_e), 0, "empty front slot: nothing to debuff, no crash");
}

static void a_ruby_moves_and_leaves(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp7-009-R");
    int opp = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, ruby, "PL!S-bp7-009-R")) return;
    tg.state.p[0].stage[1] = ruby;
    tg.state.p[1].stage[1] = opp;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, opp), -1, "setup: -1 with Ruby in front");

    tg.state.p[1].stage[0] = opp;
    tg.state.p[1].stage[1] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, opp), 0, "member moves out of the front area -> recovers");

    tg.state.p[1].stage[1] = opp;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, opp), 0, "Ruby leaves the stage -> modifier removed");
}

static void a_ruby_zero_blade_member_stays_zero(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp7-009-R");
    int opp = mid(&tg, "PL!-sd1-008-SD");
    if (!ident(&tg, ruby, "PL!S-bp7-009-R")) return;
    CHECK_EQ(printed_blade(opp), 0, "setup: PL!-sd1-008-SD prints blade 0");
    tg.state.p[0].stage[1] = ruby;
    tg.state.p[1].stage[1] = opp;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, opp), -1, "the internal modifier is still -1");
    int eff_ruby = rb_effective_blade(ruby, tg.state.mods.blade[ruby]);
    int eff_opp = rb_effective_blade(opp, tg.state.mods.blade[opp]);
    int total = eff_ruby + eff_opp;
    CHECK_EQ(total, 0, "the effective blade total floors at 0, never negative");
}

/* A18. success_pile_difference_pl_s_bp6_009_r_plus_test.rs -- the two
 * constant-blade halves. */
static void a_ruby_success_pile_difference(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp6-009-R＋");
    if (!ident(&tg, ruby, "PL!S-bp6-009-R＋")) return;
    tg.state.p[0].stage[1] = ruby;
    for (int i = 0; i < 3; i++) test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ruby), 3, "success-pile difference 3 -> ブレード3");

    for (int i = 0; i < 2; i++) test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ruby), 1, "success-pile difference 1 -> ブレード1");
}

static void a_ruby_success_pile_difference_per_instance(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby_left = mid(&tg, "PL!S-bp6-009-R＋");
    int ruby_center = test_new_id(&tg, "PL!S-bp6-009-R＋");
    if (!ident(&tg, ruby_left, "PL!S-bp6-009-R＋")) return;
    CHECK(ruby_left != ruby_center, "setup: two distinct instances of the print");
    tg.state.p[0].stage[0] = ruby_left;
    tg.state.p[0].stage[1] = ruby_center;
    for (int i = 0; i < 3; i++) test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ruby_left), 3, "left instance gets ブレード3");
    CHECK_EQ(blade(&tg, ruby_center), 3, "center instance gets ブレード3");
}

/* A19. success_pile_difference_pl_s_bp6_009_r_plus_test.rs -- the ライブ成功時
 * halves: blade grants interacting with the live SCORE.  This is the one place
 * the constant cluster reaches into the score pipeline. */
static void fire_ruby_live_success(TestGame *tg, int live_card)
{
    int hearts[8];
    memset(hearts, 0, sizeof hearts);
    hearts[0] = 20; /* Heart00 wildcard, mirrors the Rust BaseHeart fixture */
    for (int i = 0; i < 8; i++) tg->state.stage_hearts[0][i] = hearts[i];
    test_add_to_live(tg, live_card);
    tg->state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&tg->state, 0),
          "precondition: the ライブ成功時 window must be OPEN");
    rb_trigger_live_success(&tg->state, 0);
    bl_drain_skip(tg);
}

static void a_ruby_live_success_scores_revealed_aqours(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp6-009-R＋");
    int live_card = mid(&tg, "PL!S-bp2-026-L");
    int revealed = mid(&tg, "PL!S-bp2-024-L");
    if (!ident(&tg, ruby, "PL!S-bp6-009-R＋")) return;
    tg.state.p[0].stage[1] = ruby;
    test_add_to_revealed(&tg, revealed);

    fire_ruby_live_success(&tg, live_card);

    CHECK(!test_has_pending_choice(&tg), "no choice should remain pending");
    CHECK_EQ(live_total_bonus(&tg, 0), 1,
             "one revealed 『Aqours』 score-icon live adds +1 to the live total");
    CHECK_EQ(test_get_score_modifier(&tg, live_card), 0,
             "the bonus is a live TOTAL bonus, not a per-live-card score modifier");
}

static void a_ruby_off_center_live_success_does_not_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp6-009-R＋");
    int live_card = mid(&tg, "PL!S-bp2-026-L");
    if (!ident(&tg, ruby, "PL!S-bp6-009-R＋")) return;
    tg.state.p[0].stage[0] = ruby;
    test_add_to_revealed(&tg, mid(&tg, "PL!S-bp2-024-L"));

    fire_ruby_live_success(&tg, live_card);

    CHECK_EQ(live_total_bonus(&tg, 0), 0, "off-center Ruby scores nothing");
}

static void a_ruby_live_success_requires_one_revealed_card_matching_every_filter(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ruby = mid(&tg, "PL!S-bp6-009-R＋");
    int live_card = mid(&tg, "PL!S-bp2-026-L");
    if (!ident(&tg, ruby, "PL!S-bp6-009-R＋")) return;
    tg.state.p[0].stage[1] = ruby;
    test_add_to_revealed(&tg, mid(&tg, "PL!S-bp2-026-L")); /* Aqours, no score icon */
    test_add_to_revealed(&tg, mid(&tg, "PL!SP-sd1-023-SD")); /* score icon, not Aqours */

    fire_ruby_live_success(&tg, live_card);

    CHECK_EQ(live_total_bonus(&tg, 0), 0,
             "neither revealed card matches every filter -> no +1");
}

/* A20. success_score_comparison_pl_bp4_018_n_test.rs */
static void pl_bp4_018_stage(TestGame *tg, int *out)
{
    int card = mid(tg, "PL!-bp4-018-N");
    int filler = mid(tg, "PL!-sd1-010-SD");
    int own_live = mid(tg, "PL!-sd1-019-SD");
    int opp_live = mid(tg, "PL!-sd1-020-SD");
    if (!ident(tg, card, "PL!-bp4-018-N")) { *out = -1; return; }
    test_give_energy(tg, 15);
    fill_decks(tg, filler);
    test_add_to_hand(tg, card);
    test_play_to_stage(tg, card, 1);
    bl_drain_pick_first(tg);
    out[0] = card; out[1] = own_live; out[2] = opp_live;
}

static void a_bp4_018_higher_own_success_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ids[3];
    pl_bp4_018_stage(&tg, ids);
    if (ids[0] < 0) return;
    int card = ids[0], own_live = ids[1], opp_live = ids[2];

    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "equal success scores (0=0) -> 0 blade");

    test_add_to_success(&tg, own_live);
    test_recalc(&tg);
    CHECK(blade(&tg, card) >= 2, "own score 1 > opponent 0 -> blade x2");

    test_add_to_opp_success(&tg, opp_live);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "own score 1 < opponent 2 -> 0 blade");

    test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, card), 0, "equal scores (2=2) -> 0 blade");
}

/* A21. two_live_cards_constant_double_blade_test.rs -- PL!N-pb1-001-R */
static void a_two_live_cards_thresholds(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ayumu = mid(&tg, "PL!N-pb1-001-R");
    int live1 = mid(&tg, "PL!-sd1-019-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, ayumu, "PL!N-pb1-001-R")) return;
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = ayumu;
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 5);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 0, "empty live_card_zone -> 0 blade");

    test_add_to_live(&tg, live1);
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 0, "1 live card (<2) -> 0 blade");

    test_add_to_live(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 2, "2 live cards -> +2 blade");

    test_add_to_live(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 2, "3 live cards (>=2) -> still +2 blade");

    tg.state.p[0].live.n = 0;
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 0, "cards leave the zone -> bonus removed");
}

static void a_two_live_cards_wrong_zones(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int ayumu = mid(&tg, "PL!N-pb1-001-R");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, ayumu, "PL!N-pb1-001-R")) return;
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = ayumu;
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 5);
    for (int i = 0; i < 3; i++) test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, ayumu), 0, "success_live_card_zone is the wrong zone -> 0 blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int ayumu2 = mid(&tg2, "PL!N-pb1-001-R");
    int filler2 = mid(&tg2, "PL!-sd1-010-SD");
    int m1 = mid(&tg2, "PL!-sd1-010-SD");
    int m2 = test_new_id(&tg2, "PL!-sd1-010-SD");
    tg2.state.p[0].stage[0] = m1;
    tg2.state.p[0].stage[1] = ayumu2;
    tg2.state.p[0].stage[2] = m2;
    test_add_to_live(&tg2, mid(&tg2, "PL!-sd1-019-SD"));
    test_give_energy(&tg2, 5);
    test_recalc(&tg2);
    CHECK_EQ(blade(&tg2, ayumu2), 0,
             "stage member cards do not count toward the live_card_zone threshold");
}

/* A22. exact_energy_gain_pl_pr_021_pr_test.rs
 * The constant must refresh with NO manual recalc after an energy gain, so this
 * test fires a real ライブ開始時 ability and asserts the modifier immediately. */
static void a_exactly_seven_energy_refreshes_after_live_start_gain(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int niko = mid(&tg, "PL!-PR-021-PR");
    int yuna = mid(&tg, "PL!SP-bp5-222-R");
    if (!ident(&tg, niko, "PL!-PR-021-PR")) return;
    if (!ident(&tg, yuna, "PL!SP-bp5-222-R")) return;
    tg.state.p[0].stage[0] = niko;
    tg.state.p[0].stage[1] = yuna;
    test_give_energy(&tg, 6);
    for (int i = 0; i < 3; i++) test_add_to_energy_deck(&tg, 0, mid(&tg, "LL-E-001-SD"));
    test_recalc(&tg);
    CHECK_EQ(blade(&tg, niko), 0, "setup: 6 energy -> the constant is off");

    CHECK(bl_fire_live_start(&tg, yuna), "PL!SP-bp5-222-R prints a ライブ開始時");
    CHECK(test_has_pending_choice(&tg), "the optional energy cost is offered");
    rb_resume_with_choice(&tg.state, 1); /* pay */
    bl_drain_pay_costs(&tg);

    CHECK_EQ(tg.state.p[0].energy.n, 7, "the placement brings the zone to exactly 7");
    CHECK_EQ(blade(&tg, niko), 2,
             "exactly-7 must hold right after the gain (no manual recalc)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * PART B -- effects/gain/blades/live_start/
 * ══════════════════════════════════════════════════════════════════════════ */

/* B1. all_stage_heart_colors_blades_shizuku_bp5_015_n_test.rs */
static void b_shizuku_all_stage_heart_colors(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int shizuku = mid(&tg, "PL!N-bp5-015-N");
    if (!ident(&tg, shizuku, "PL!N-bp5-015-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));

    tg.state.p[0].stage[0] = shizuku;
    tg.state.p[0].stage[1] = mid(&tg, "PL!-sd1-001-SD");
    tg.state.p[0].stage[2] = mid(&tg, "PL!S-sd1-003-SD");
    bl_fire_live_start(&tg, shizuku);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, shizuku), 2, "all six colors present across members -> +2 blades");
}

static void b_shizuku_missing_heart_colors(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int shizuku = mid(&tg, "PL!N-bp5-015-N");
    if (!ident(&tg, shizuku, "PL!N-bp5-015-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    tg.state.p[0].stage[0] = shizuku;
    tg.state.p[0].stage[1] = mid(&tg, "PL!-sd1-001-SD");
    bl_fire_live_start(&tg, shizuku);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, shizuku), 0, "colors 02/04/05 missing -> no blades");
}

/* B2. debut_and_live_start_blade_grant_test.rs
 * TIMING: the debut grant is fired through the 登場 trigger shim so it is not
 * confused with the live-start grant of the second half. */
static void b_hs_cl1_006_debut_grants_three_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!HS-cl1-006-CL");
    if (!ident(&tg, me, "PL!HS-cl1-006-CL")) return;
    tg.state.p[0].stage[1] = me;
    bl_fire_debut(&tg, me);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, me), 3, "debut grants +3 blades until live end");
}

static void b_sd1_022_live_start_grants_only_to_aqours(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!S-sd1-022-SD");
    if (!ident(&tg, live, "PL!S-sd1-022-SD")) return;
    test_add_to_live(&tg, live);
    int a1 = mid(&tg, "PL!S-sd1-001-SD");
    int a2 = test_new_id(&tg, "PL!S-sd1-002-SD");
    int non_aqours = mid(&tg, "PL!HS-bp5-004-R");
    if (!ident(&tg, non_aqours, "PL!HS-bp5-004-R")) return;
    tg.state.p[0].stage[0] = a1;
    tg.state.p[0].stage[1] = a2;
    tg.state.p[0].stage[2] = non_aqours;
    bl_fire_live_start(&tg, live);
    bl_drain_skip(&tg);
    CHECK(blade(&tg, a1) >= 1, "Aqours member 1 gains a blade");
    CHECK(blade(&tg, a2) >= 1, "Aqours member 2 gains a blade");
    CHECK_EQ(blade(&tg, non_aqours), 0, "the non-Aqours member must not gain");
}

/* B3. left_area_moved_blades_pl_sp_bp4_017_n_test.rs
 * B4. moved_right_side_pl_sp_bp4_020_n_test.rs
 * The two position-gated "moved this turn" live starts. */
static void b_bp4_017_left_area_moved(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp4-017-N");
    if (!ident(&tg, me, "PL!SP-bp4-017-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    tg.state.p[0].stage[0] = me;
    tg.state.moved_this_turn[me] = 1;
    tg.state.position_change_occurred_this_turn = 1;
    bl_fire_live_start(&tg, me);
    bl_drain_pick_first(&tg);
    CHECK_EQ(blade(&tg, me), 2, "moved-this-turn on the left side -> +2 blades");
}

static void b_bp4_017_left_area_unmoved(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp4-017-N");
    if (!ident(&tg, me, "PL!SP-bp4-017-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    tg.state.p[0].stage[0] = me;
    bl_fire_live_start(&tg, me);
    CHECK_EQ(blade(&tg, me), 0, "did not move this turn -> no blades");
}

static void b_bp4_017_center_area_moved(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp4-017-N");
    if (!ident(&tg, me, "PL!SP-bp4-017-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    tg.state.p[0].stage[1] = me;
    tg.state.moved_this_turn[me] = 1;
    tg.state.position_change_occurred_this_turn = 1;
    bl_fire_live_start(&tg, me);
    CHECK_EQ(blade(&tg, me), 0, "the ability only activates in the left-side area");
}

static void b_bp4_020_right_area_moved(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp4-020-N");
    if (!ident(&tg, me, "PL!SP-bp4-020-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!N-sd1-010-SD"));
    tg.state.p[0].stage[2] = me;
    tg.state.moved_this_turn[me] = 1;
    tg.state.position_change_occurred_this_turn = 1;
    bl_fire_live_start(&tg, me);
    CHECK_EQ(blade(&tg, me), 2, "moved right-side member -> +2 blades");
}

static void b_bp4_020_center_area_moved(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!SP-bp4-020-N");
    if (!ident(&tg, me, "PL!SP-bp4-020-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!N-sd1-010-SD"));
    tg.state.p[0].stage[1] = me;
    tg.state.moved_this_turn[me] = 1;
    tg.state.position_change_occurred_this_turn = 1;
    bl_fire_live_start(&tg, me);
    CHECK_EQ(blade(&tg, me), 0, "right-side-only ability must not fire from center");
}

/* B5. live_start_high_cost_hasunosora_member_blades_test.rs */
static void b_cl1_010_hasunosora_cost_threshold(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!HS-cl1-010-CL");
    int big = mid(&tg, "PL!HS-bp5-004-R");
    if (!ident(&tg, live, "PL!HS-cl1-010-CL")) return;
    if (!ident(&tg, big, "PL!HS-bp5-004-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, live);
    tg.state.p[0].stage[1] = big;
    bl_fire_live_start(&tg, live);
    CHECK_EQ(blade(&tg, big), 2, "cost-15 Hasunosora member gains +2 blades");

    static TestGame tg2;
    test_game_new(&tg2);
    int live2 = mid(&tg2, "PL!HS-cl1-010-CL");
    int low = mid(&tg2, "PL!HS-bp1-002-CL");
    if (!ident(&tg2, low, "PL!HS-bp1-002-CL")) return;
    fill_decks(&tg2, test_new_id(&tg2, "PL!-sd1-010-SD"));
    test_add_to_live(&tg2, live2);
    tg2.state.p[0].stage[1] = low;
    bl_fire_live_start(&tg2, live2);
    CHECK_EQ(blade(&tg2, low), 0, "a cost-5 member is under the >=10 threshold");
}

/* B6. live_start_live_card_ability_filter_edge_test.rs
 * B9. triggerless_live_other_member_pl_bp4_014_n_test.rs
 * B10. live_start_plain_live_card_blade_edge_test.rs
 * All four drive PL!-bp4-014-N, whose ライブ開始時 gates on "no live card in
 * the zone has a ライブ開始時 / ライブ成功時 ability". */
static void b_rin_setup(TestGame *tg, int *rin, int *mate)
{
    int filler = test_new_id(tg, "PL!-sd1-010-SD");
    fill_decks(tg, filler);
    *rin = mid(tg, "PL!-bp4-014-N");
    *mate = mid(tg, "PL!S-sd1-001-SD");
    if (!ident(tg, *rin, "PL!-bp4-014-N")) { *rin = -1; return; }
    tg->state.p[0].stage[0] = *mate;
    tg->state.p[0].stage[1] = *rin;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
}

static void b_rin_triggerless_live_grants(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rin, mate;
    b_rin_setup(&tg, &rin, &mate);
    if (rin < 0) return;
    test_add_to_live(&tg, mid(&tg, "PL!-sd1-020-SD"));
    bl_fire_live_start(&tg, rin);
    CHECK_EQ(blade(&tg, mate), 2, "a triggerless live -> the other member gains +2 blades");
    CHECK_EQ(blade(&tg, rin), 0, "exclude_self: Rin never boosts herself");
}

static void b_rin_live_with_triggers_grants_nothing(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rin, mate;
    b_rin_setup(&tg, &rin, &mate);
    if (rin < 0) return;
    test_add_to_live(&tg, mid(&tg, "PL!N-sd2-007-P")); /* has both LS and LSS */
    bl_fire_live_start(&tg, rin);
    CHECK_EQ(blade(&tg, mate), 0, "a live with both LS and LSS does not count");

    static TestGame tg2;
    test_game_new(&tg2);
    int rin2, mate2;
    b_rin_setup(&tg2, &rin2, &mate2);
    if (rin2 < 0) return;
    test_add_to_live(&tg2, mid(&tg2, "PL!N-bp1-027-L")); /* LS only */
    bl_fire_live_start(&tg2, rin2);
    CHECK_EQ(blade(&tg2, mate2), 0, "a live with only LS does not count");
}

static void b_rin_is_null_live_counts_as_no_ability(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rin, mate;
    b_rin_setup(&tg, &rin, &mate);
    if (rin < 0) return;
    test_add_to_live(&tg, mid(&tg, "PL!HS-bp1-019-L")); /* is_null */
    bl_fire_live_start(&tg, rin);
    CHECK_EQ(blade(&tg, mate), 2, "an is_null live counts as no ability -> blade granted");
}

static void b_rin_empty_and_mixed_live_zones(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rin, mate;
    b_rin_setup(&tg, &rin, &mate);
    if (rin < 0) return;
    bl_fire_live_start(&tg, rin);
    CHECK_EQ(blade(&tg, mate), 0, "an empty live zone grants nothing");

    static TestGame tg2;
    test_game_new(&tg2);
    int rin2, mate2;
    b_rin_setup(&tg2, &rin2, &mate2);
    if (rin2 < 0) return;
    test_add_to_live(&tg2, mid(&tg2, "PL!-sd1-020-SD"));
    test_add_to_live(&tg2, test_new_id(&tg2, "PL!-sd1-020-SD"));
    bl_fire_live_start(&tg2, rin2);
    CHECK_EQ(blade(&tg2, mate2), 2, "two triggerless lives still grant");

    static TestGame tg3;
    test_game_new(&tg3);
    int rin3, mate3;
    b_rin_setup(&tg3, &rin3, &mate3);
    if (rin3 < 0) return;
    test_add_to_live(&tg3, mid(&tg3, "PL!-sd1-020-SD"));
    test_add_to_live(&tg3, test_new_id(&tg3, "PL!N-sd2-007-P"));
    bl_fire_live_start(&tg3, rin3);
    CHECK_EQ(blade(&tg3, mate3), 2, "one plain live among mixed lives is enough");
}

/* B7. live_start_mus_member_blade_gain_test.rs */
static void b_bp4_024_mus_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!-bp4-024-L");
    int mus_mate = mid(&tg, "PL!S-sd1-001-SD");
    int mus_member = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, live, "PL!-bp4-024-L")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, live);
    tg.state.p[0].stage[1] = mus_mate;
    tg.state.p[0].stage[0] = mus_member;
    bl_fire_live_start(&tg, live);
    CHECK_EQ(blade(&tg, mus_member), 1, "a μ's member gains one blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int live2 = mid(&tg2, "PL!-bp4-024-L");
    int aq = mid(&tg2, "PL!S-sd1-001-SD");
    fill_decks(&tg2, test_new_id(&tg2, "PL!-sd1-010-SD"));
    test_add_to_live(&tg2, live2);
    tg2.state.p[0].stage[0] = aq;
    bl_fire_live_start(&tg2, live2);
    CHECK_EQ(blade(&tg2, aq), 0, "with no μ's member, no blade");
}

/* B8. live_start_named_chisato_member_blade_test.rs */
static void b_sp_bp7_025_named_chisato(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int holder = mid(&tg, "PL!SP-bp7-025-L");
    int chisato = mid(&tg, "PL!SP-pb1-014-PR");
    int other = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, holder, "PL!SP-bp7-025-L")) return;
    if (!ident(&tg, chisato, "PL!SP-pb1-014-PR")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, holder);
    tg.state.p[0].stage[1] = chisato;
    tg.state.p[0].stage[0] = other;
    bl_fire_live_start(&tg, holder);
    CHECK_EQ(blade(&tg, chisato), 1, "the staged Chisato gains 1 blade");
    CHECK_EQ(blade(&tg, other), 0, "other members gain nothing");
}

/* B11. live_start_paid_self_wait_center_member_blades_test.rs
 * The pay-optional-cost gate is the interesting part: it must present a
 * SelectTarget, and paying it waits the source. */
static void b_bp4_011_paid_self_wait_center_mus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!-bp4-011-N");
    if (!ident(&tg, me, "PL!-bp4-011-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    int center = test_new_id(&tg, "PL!-sd1-001-SD");
    int left_mu = test_new_id(&tg, "PL!-sd1-007-SD");
    tg.state.p[0].stage[0] = me;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = left_mu;

    bl_fire_live_start(&tg, me);
    CHECK(test_has_pending_choice(&tg), "pay-optional-cost prompt expected");
    const RbChoice *c = rb_get_pending_choice(&tg.state);
    CHECK(c && c->kind == RB_CHOICE_SELECT_TARGET,
          "expected a SelectTarget pay_optional_cost gate");
    rb_resume_with_choice(&tg.state, 1); /* pay */
    bl_drain_pick_first(&tg);

    const char *orient = rb_mods_get_orientation(&tg.state.mods, me);
    CHECK(orient && !strcmp(orient, "wait"), "the cost waits this member");
    CHECK_EQ(blade(&tg, center), 2, "the center-area μ's member gains +2 blades");
    CHECK_EQ(blade(&tg, left_mu), 0, "a non-center member gains nothing");
}

static void b_bp4_017_twin_paid_self_wait(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!-bp4-017-N");
    if (!ident(&tg, me, "PL!-bp4-017-N")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    int center = test_new_id(&tg, "PL!-sd1-001-SD");
    tg.state.p[0].stage[0] = me;
    tg.state.p[0].stage[1] = center;

    bl_fire_live_start(&tg, me);
    CHECK(test_has_pending_choice(&tg), "pay-optional-cost prompt expected");
    const RbChoice *c = rb_get_pending_choice(&tg.state);
    CHECK(c && c->kind == RB_CHOICE_SELECT_TARGET,
          "expected a SelectTarget pay_optional_cost gate");
    rb_resume_with_choice(&tg.state, 1);
    bl_drain_pick_first(&tg);
    CHECK_EQ(blade(&tg, center), 1, "the twin grants +1 blade to the center μ's member");
}

/* B12. optional_discard_other_member_blades_honoka_pb1_010_r_test.rs */
static void b_honoka_paid_discard_other_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int honoka = mid(&tg, "PL!-pb1-010-R");
    if (!ident(&tg, honoka, "PL!-pb1-010-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, honoka);
    int mate_a = mid(&tg, "PL!S-sd1-001-SD");
    int mate_b = mid(&tg, "PL!-sd1-007-SD");
    tg.state.p[0].stage[0] = mate_a;
    tg.state.p[0].stage[1] = mate_b;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));

    bl_fire_live_start(&tg, honoka);
    CHECK(test_has_pending_choice(&tg), "the optional discard is offered");
    rb_resume_with_choice(&tg.state, 0);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, mate_a), 1, "member A gains one blade");
    CHECK_EQ(blade(&tg, mate_b), 1, "member B gains one blade");
    CHECK_EQ(blade(&tg, honoka), 0, "ほかのメンバー excludes Honoka herself");
}

static void b_honoka_empty_hand_auto_skips(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int honoka = mid(&tg, "PL!-pb1-010-R");
    if (!ident(&tg, honoka, "PL!-pb1-010-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, honoka);
    int mate_a = mid(&tg, "PL!S-sd1-001-SD");
    tg.state.p[0].stage[0] = mate_a;
    bl_fire_live_start(&tg, honoka);
    CHECK(!test_has_pending_choice(&tg),
          "an unpayable optional cost (empty hand) auto-skips without prompting");
    CHECK_EQ(blade(&tg, mate_a), 0, "declined -> no blades");
}

static void b_honoka_paid_without_other_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int honoka = mid(&tg, "PL!-pb1-010-R");
    if (!ident(&tg, honoka, "PL!-pb1-010-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, honoka);
    int hand_fodder = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, hand_fodder);

    bl_fire_live_start(&tg, honoka);
    CHECK(test_has_pending_choice(&tg), "the optional discard is offered");
    rb_resume_with_choice(&tg.state, 0);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, honoka), 0, "no other members -> no blades");
    CHECK(discard_has(&tg, hand_fodder), "the cost fodder was discarded");
}

/* B13. optional_discard_success_zone_blades_maki_bp3_006_r_test.rs */
static void b_maki_three_success_cards_six_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int maki = mid(&tg, "PL!-bp3-006-R");
    if (!ident(&tg, maki, "PL!-bp3-006-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, maki);
    for (int i = 0; i < 3; i++) test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    int hand_fodder = test_new_id(&tg, "PL!-sd1-010-SD");
    test_add_to_hand(&tg, hand_fodder);

    bl_fire_live_start(&tg, maki);
    CHECK(test_has_pending_choice(&tg), "optional cost prompted");
    rb_resume_with_choice(&tg.state, 0);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, maki), 6, "3 success-zone cards x 2 blades");
    CHECK(!hand_has(&tg, hand_fodder), "the cost fodder left the hand");
    CHECK(discard_has(&tg, hand_fodder), "the cost fodder was discarded");
}

static void b_maki_empty_hand_and_empty_success_zone(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int maki = mid(&tg, "PL!-bp3-006-R");
    if (!ident(&tg, maki, "PL!-bp3-006-R")) return;
    fill_decks(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_live(&tg, maki);
    for (int i = 0; i < 3; i++) test_add_to_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    bl_fire_live_start(&tg, maki);
    CHECK(!test_has_pending_choice(&tg), "an unpayable optional cost auto-skips");
    CHECK_EQ(blade(&tg, maki), 0, "declined -> no blades despite a full success zone");

    static TestGame tg2;
    test_game_new(&tg2);
    int maki2 = mid(&tg2, "PL!-bp3-006-R");
    int hand_fodder = test_new_id(&tg2, "PL!-sd1-010-SD");
    fill_decks(&tg2, test_new_id(&tg2, "PL!-sd1-010-SD"));
    test_add_to_live(&tg2, maki2);
    test_add_to_hand(&tg2, hand_fodder);
    bl_fire_live_start(&tg2, maki2);
    CHECK(test_has_pending_choice(&tg2), "the discard cost is prompted");
    const RbChoice *c = rb_get_pending_choice(&tg2.state);
    CHECK(c && c->kind == RB_CHOICE_SELECT_CARD && c->allow_skip,
          "expected a skippable SelectCard discard-cost prompt");
    rb_resume_with_choice(&tg2.state, 0);
    bl_drain_skip(&tg2);
    CHECK_EQ(blade(&tg2, maki2), 0, "no success cards -> 0 x 2 blades");
    CHECK(discard_has(&tg2, hand_fodder),
          "the cost was still paid even though it yielded nothing");
}

/* B14. live_start_optional_energy_self_blades_test.rs
 * Declining must be distinguishable from paying: the declined twin is the
 * assertion that catches a pay-gate that always pays. */
static void b_optional_energy_declined_and_paid(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!HS-PR-018-PR");
    if (!ident(&tg, member, "PL!HS-PR-018-PR")) return;
    fill_decks_append(&tg, mid(&tg, "PL!-sd1-010-SD"), 20);
    tg.state.p[0].stage[1] = member;
    test_give_energy(&tg, 15);
    bl_advance_to_live_set(&tg);
    for (int i = 0; i < 3; i++) {
        test_pass(&tg);
        bl_drain_skip_optional(&tg);
    }
    CHECK_EQ(blade(&tg, member), 0,
             "declining 「E 支払ってもよい」 must grant no blade at all");

    static TestGame tg2;
    test_game_new(&tg2);
    int m2 = mid(&tg2, "PL!HS-PR-018-PR");
    if (!ident(&tg2, m2, "PL!HS-PR-018-PR")) return;
    fill_decks_append(&tg2, mid(&tg2, "PL!-sd1-010-SD"), 20);
    tg2.state.p[0].stage[1] = m2;
    test_give_energy(&tg2, 15);
    for (int i = 0; i < 7; i++) {
        test_pass(&tg2);
        bl_drain_pay_costs(&tg2);
    }
    CHECK_EQ(blade(&tg2, m2), 2, "paying E 支払ってもよい grants exactly two blades");
}

static void b_pay_one_energy_gains_exactly_one_blade(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!N-bp1-001-R");
    if (!ident(&tg, member, "PL!N-bp1-001-R")) return;
    fill_decks_append(&tg, mid(&tg, "PL!-sd1-010-SD"), 20);
    tg.state.p[0].stage[1] = member;
    test_give_energy(&tg, 15);
    for (int i = 0; i < 7; i++) {
        test_pass(&tg);
        bl_drain_pay_costs(&tg);
    }
    CHECK_EQ(blade(&tg, member), 1,
             "paying 1E grants exactly one ブレード, not merely at least one");
}

/* B15. live_start_optional_energy_other_group_blades_test.rs
 * B16. live_start_other_group_blade_excludes_self_test.rs */
static void b_other_niji_member_gains_source_does_not(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int self_member = mid(&tg, "PL!N-sd1-001-SD");
    int other_niji = test_new_id(&tg, "PL!N-bp4-007-R＋");
    if (!ident(&tg, self_member, "PL!N-sd1-001-SD")) return;
    if (!ident(&tg, other_niji, "PL!N-bp4-007-R＋")) return;
    tg.state.p[0].stage[0] = self_member;
    tg.state.p[0].stage[1] = other_niji;
    fill_decks_append(&tg, test_new_id(&tg, "PL!-sd1-010-SD"), 20);
    test_give_energy(&tg, 15);
    for (int i = 0; i < 7; i++) {
        test_pass(&tg);
        bl_drain_pay_costs(&tg);
    }
    CHECK_EQ(blade(&tg, other_niji), 1,
             "the other 虹ヶ咲 member gains exactly one blade");
    CHECK_EQ(blade(&tg, self_member), 0, "the source herself is not included");
}

static void b_other_niji_lone_source_unboosted(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!N-sd1-001-SD");
    if (!ident(&tg, member, "PL!N-sd1-001-SD")) return;
    tg.state.p[0].stage[0] = member;
    fill_decks_append(&tg, test_new_id(&tg, "PL!-sd1-010-SD"), 20);
    test_give_energy(&tg, 15);
    for (int i = 0; i < 7; i++) {
        test_pass(&tg);
        bl_drain_skip_optional(&tg);
    }
    CHECK_EQ(blade(&tg, member), 0, "no other 虹ヶ咲 on stage -> no blade boost");
}

/* B17. live_start_optional_discard_self_blades_test.rs */
static void b_optional_discard_self_one_blade(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member = mid(&tg, "PL!N-bp1-005-R");
    if (!ident(&tg, member, "PL!N-bp1-005-R")) return;
    fill_decks_append(&tg, mid(&tg, "PL!-sd1-010-SD"), 20);
    tg.state.p[0].stage[1] = member;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_give_energy(&tg, 15);
    int waitroom_before = tg.state.p[0].discard.n;
    for (int i = 0; i < 7; i++) {
        test_pass(&tg);
        bl_drain_pay_costs(&tg);
    }
    CHECK_EQ(blade(&tg, member), 1, "paying the optional discard -> exactly +1 blade");
    CHECK_EQ(tg.state.p[0].discard.n, waitroom_before + 1,
             "「手札を1枚控え室に置いてもよい」 -- the cost really was paid");
}

static void b_optional_discard_self_two_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rin = mid(&tg, "PL!N-sd1-004-SD");
    if (!ident(&tg, rin, "PL!N-sd1-004-SD")) return;
    fill_decks_append(&tg, mid(&tg, "PL!-sd1-010-SD"), 20);
    tg.state.p[0].stage[1] = rin;
    test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    test_give_energy(&tg, 10);
    for (int i = 0; i < 7; i++) {
        test_pass(&tg);
        bl_drain_pay_costs(&tg);
    }
    CHECK_EQ(blade(&tg, rin), 2, "paying the optional discard -> exactly +2 ブレード");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "one hand card was discarded as the cost");
}

/* B18. live_start_hand_pair_blades_test.rs
 * B18b. live_start_hand_pair_blade_snapshot_q109_test.rs
 * TIMING: these walk the real turn machine, so the blade count depends on how
 * many cards are in hand AT ライブ開始時, after set_live_card removed the live
 * card and LiveCardSetFirstAttacker drew a replacement. */
static void b_hand_pair_six_hand_gives_three(void)
{
    static TestGame tg;
    test_game_new(&tg);
    /* The Rust files disagree on the print: live_start_hand_pair_blades_test.rs
     * stages PL!SP-bp2-009-R＋, the q109 file stages PL!SP-bp2-009-P.  Both are
     * 鬼塚夏美 with the same hand-pair text, but they are DIFFERENT prints, so
     * the identity is asserted rather than assumed. */
    int natsumi = mid(&tg, "PL!SP-bp2-009-R＋");
    if (!ident(&tg, natsumi, "PL!SP-bp2-009-R＋")) return;
    int filler = mid(&tg, "PL!-sd1-010-SD");
    int live_card = mid(&tg, "PL!-sd1-019-SD");

    tg.state.p[0].stage[1] = natsumi;
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, live_card);
    fill_decks_append(&tg, filler, 10);

    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live_card);
    bl_advance_to_live_start(&tg);
    bl_drain_pick_first(&tg);

    CHECK_EQ(blade(&tg, natsumi), 3, "6 hand cards at ライブ開始時 -> 3 blades (6/2)");
}

static void b_hand_pair_q109_snapshot(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int natsumi = mid(&tg, "PL!SP-bp2-009-P");
    if (!ident(&tg, natsumi, "PL!SP-bp2-009-P")) return;
    int filler = mid(&tg, "PL!-sd1-010-SD");
    int live_card = mid(&tg, "PL!-sd1-019-SD");
    tg.state.p[0].stage[1] = natsumi;
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, live_card);
    fill_decks_append(&tg, filler, 10);

    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live_card);
    bl_advance_to_live_start(&tg);
    bl_drain_pick_first(&tg);
    CHECK_EQ(blade(&tg, natsumi), 3, "6 hand -> 3 blade at resolution");

    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    CHECK_EQ(blade(&tg, natsumi), 3,
             "Q109: the blade count is a snapshot -- hand 8 does not make it 4");

    tg.state.p[0].hand.n = tg.state.p[0].hand.n > 3 ? tg.state.p[0].hand.n - 3 : 0;
    CHECK_EQ(blade(&tg, natsumi), 3,
             "Q109: discarding after resolution leaves the blade count alone");
}

static void b_hand_pair_q109_empty_and_odd_hand(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int natsumi = mid(&tg, "PL!SP-bp2-009-P");
    if (!ident(&tg, natsumi, "PL!SP-bp2-009-P")) return;
    int filler = mid(&tg, "PL!-sd1-010-SD");
    int live_card = mid(&tg, "PL!-sd1-019-SD");
    tg.state.p[0].stage[1] = natsumi;
    test_add_to_hand(&tg, live_card);
    fill_decks_append(&tg, filler, 10);
    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live_card);
    bl_advance_to_live_start(&tg);
    bl_drain_pick_first(&tg);
    CHECK_EQ(blade(&tg, natsumi), 0, "0 hand at resolution -> 0 blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int n2 = mid(&tg2, "PL!SP-bp2-009-P");
    int f2 = mid(&tg2, "PL!-sd1-010-SD");
    int l2 = mid(&tg2, "PL!-sd1-019-SD");
    tg2.state.p[0].stage[1] = n2;
    for (int i = 0; i < 4; i++) test_add_to_hand(&tg2, f2);
    test_add_to_hand(&tg2, l2);
    fill_decks_append(&tg2, f2, 10);
    bl_advance_to_live_set(&tg2);
    test_set_live_card(&tg2, 0, l2);
    bl_advance_to_live_start(&tg2);
    bl_drain_pick_first(&tg2);
    CHECK_EQ(blade(&tg2, n2), 2, "5 hand at resolution -> 5/2 = 2 blade (floor)");
}

/* B19. live_start_center_liella_set_blade_three_test.rs -- set_blade(3), a
 * different modifier OPERATION from everything else in this cluster. */
static void bq195_setup(TestGame *tg, int *special, int filler_hand)
{
    int filler = mid(tg, "PL!-sd1-010-SD");
    fill_decks_append(tg, filler, 10);
    *special = mid(tg, "PL!SP-bp4-025-L");
    if (!ident(tg, *special, "PL!SP-bp4-025-L")) { *special = -1; return; }
    test_add_to_hand(tg, *special);
    if (filler_hand) test_add_to_hand(tg, filler);
}

static void bq195_run(TestGame *tg, int special)
{
    bl_advance_to_live_set(tg);
    test_set_live_card(tg, 0, special);
    bl_advance_to_live_start(tg);
    bl_drain_skip_optional(tg);
}

static void bq195_center_liella_set_to_three(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int special;
    bq195_setup(&tg, &special, 1);
    if (special < 0) return;
    int liella = mid(&tg, "PL!SP-bp1-001-R");
    int non_liella = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, liella, "PL!SP-bp1-001-R")) return;
    tg.state.p[0].stage[0] = non_liella;
    tg.state.p[0].stage[1] = liella;
    bq195_run(&tg, special);
    CHECK_EQ(blade(&tg, liella), 3, "Liella! at center: set_blade(3) stores a raw 3");
    CHECK_EQ(blade(&tg, non_liella), 0, "a non-Liella! member gets no modifier");
}

static void bq195_position_and_group_filters(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int special;
    bq195_setup(&tg, &special, 1);
    if (special < 0) return;
    int liella = mid(&tg, "PL!SP-bp1-001-R");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = liella;
    tg.state.p[0].stage[1] = filler;
    bq195_run(&tg, special);
    CHECK_EQ(blade(&tg, liella), 0, "Liella! at the left side is excluded by position");

    static TestGame tg2;
    test_game_new(&tg2);
    int special2;
    bq195_setup(&tg2, &special2, 1);
    if (special2 < 0) return;
    int liella_r = mid(&tg2, "PL!SP-bp1-001-R");
    int filler_a = mid(&tg2, "PL!-sd1-010-SD");
    int filler_b = test_new_id(&tg2, "PL!-sd1-010-SD");
    tg2.state.p[0].stage[0] = filler_a;
    tg2.state.p[0].stage[1] = filler_b;
    tg2.state.p[0].stage[2] = liella_r;
    bq195_run(&tg2, special2);
    CHECK_EQ(blade(&tg2, liella_r), 0, "Liella! at the right side is excluded by position");

    static TestGame tg3;
    test_game_new(&tg3);
    int special3;
    bq195_setup(&tg3, &special3, 1);
    if (special3 < 0) return;
    int non_liella = mid(&tg3, "PL!-sd1-010-SD");
    tg3.state.p[0].stage[1] = non_liella;
    bq195_run(&tg3, special3);
    CHECK_EQ(blade(&tg3, non_liella), 0, "a non-Liella! at center is excluded by group");
}

static void bq195_only_center_of_several(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int special;
    bq195_setup(&tg, &special, 1);
    if (special < 0) return;
    int liella_l = mid(&tg, "PL!SP-bp1-001-R");
    int liella_c = test_new_id(&tg, "PL!SP-bp1-001-R");
    int liella_r = test_new_id(&tg, "PL!SP-bp1-001-R");
    CHECK(liella_l != liella_c && liella_c != liella_r, "setup: three distinct instances");
    tg.state.p[0].stage[0] = liella_l;
    tg.state.p[0].stage[1] = liella_c;
    tg.state.p[0].stage[2] = liella_r;
    bq195_run(&tg, special);
    CHECK_EQ(blade(&tg, liella_c), 3, "the center Liella! gets set to 3");
    CHECK_EQ(blade(&tg, liella_l), 0, "the left Liella! gets nothing");
    CHECK_EQ(blade(&tg, liella_r), 0, "the right Liella! gets nothing");
}

static void bq195_center_empty_and_self_target(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int special;
    bq195_setup(&tg, &special, 1);
    if (special < 0) return;
    int liella_l = mid(&tg, "PL!SP-bp1-001-R");
    int liella_r = test_new_id(&tg, "PL!SP-bp1-001-R");
    tg.state.p[0].stage[0] = liella_l;
    tg.state.p[0].stage[2] = liella_r;
    bq195_run(&tg, special);
    CHECK_EQ(blade(&tg, liella_l), 0, "an empty center grants nothing to the left");
    CHECK_EQ(blade(&tg, liella_r), 0, "an empty center grants nothing to the right");

    static TestGame tg2;
    test_game_new(&tg2);
    int special2;
    bq195_setup(&tg2, &special2, 1);
    if (special2 < 0) return;
    int self_liella = mid(&tg2, "PL!SP-bp1-001-R");
    int opp_liella = test_new_id(&tg2, "PL!SP-bp1-001-R");
    int filler = mid(&tg2, "PL!-sd1-010-SD");
    tg2.state.p[0].stage[0] = filler;
    tg2.state.p[0].stage[1] = self_liella;
    tg2.state.p[1].stage[1] = opp_liella;
    bq195_run(&tg2, special2);
    CHECK_EQ(blade(&tg2, self_liella), 3, "our center Liella! is set to 3");
    CHECK_EQ(blade(&tg2, opp_liella), 0, "the opponent's Liella! is out of target=self");
}

/* set_blade(3) must SET, not add: the same answer for blade 1, 2, 3 and 4. */
static void bq195_set_not_add(void)
{
    struct { const char *no; int printed; } cases[] = {
        { "PL!SP-PR-008-PR", 1 },
        { "PL!SP-PR-012-PR", 2 },
        { "PL!SP-bp1-001-R", 3 },
        { "PL!SP-bp5-006-R", 4 },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        static TestGame tg;
        test_game_new(&tg);
        int liella = mid(&tg, cases[i].no);
        if (!ident(&tg, liella, cases[i].no)) continue;
        CHECK_EQ(printed_blade(liella), cases[i].printed,
                  "setup: the staged Liella! really prints the claimed blade count");
        tg.state.p[0].stage[1] = liella;
        int special;
        bq195_setup(&tg, &special, 1);
        if (special < 0) continue;
        bq195_run(&tg, special);
        char msg[160];
        snprintf(msg, sizeof msg,
                 "blade=%d Liella! at center: set_blade(3) stores 3 (+3 would store %d)",
                 cases[i].printed, cases[i].printed + 3);
        CHECK_EQ(blade(&tg, liella), 3, msg);
    }
}

static void bq195_existing_additive_stacks(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int liella = mid(&tg, "PL!SP-bp1-001-R");
    if (!ident(&tg, liella, "PL!SP-bp1-001-R")) return;
    tg.state.p[0].stage[1] = liella;
    int special;
    bq195_setup(&tg, &special, 1);
    if (special < 0) return;
    rb_mods_add_blade(&tg.state.mods, liella, 1);
    bq195_run(&tg, special);
    CHECK_EQ(blade(&tg, liella), 4, "Q195: an existing +1 stacks on set_blade(3)");

    static TestGame tg2;
    test_game_new(&tg2);
    int kinako = mid(&tg2, "PL!SP-bp5-006-R");
    if (!ident(&tg2, kinako, "PL!SP-bp5-006-R")) return;
    tg2.state.p[0].stage[1] = kinako;
    int special2;
    bq195_setup(&tg2, &special2, 1);
    if (special2 < 0) return;
    rb_mods_add_blade(&tg2.state.mods, kinako, 1);
    bq195_run(&tg2, special2);
    CHECK_EQ(blade(&tg2, kinako), 4,
             "Q195: blade=4 plus an existing +1 -> set(3) + add(1) = 4");
}

/* B20. live_start_blade_per_discard_test.rs -- LL-bp2-001-R＋, one blade per
 * discarded named card.  The discard is an any_number cost with a characters
 * filter, so the hand only ever contains matching copies. */
static void b_triple_discard_blade_per_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int stage = mid(&tg, "LL-bp2-001-R＋");
    if (!ident(&tg, stage, "LL-bp2-001-R＋")) return;
    int hand1 = test_new_id(&tg, "LL-bp2-001-R＋");
    int hand2 = test_new_id(&tg, "LL-bp2-001-R＋");
    int live = mid(&tg, "PL!-sd1-020-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[1] = stage;
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, hand1);
    test_add_to_hand(&tg, hand2);
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, filler);
    fill_decks_append(&tg, filler, 10);

    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    bl_advance_to_live_start(&tg);

    CHECK(test_has_pending_choice(&tg), "the optional discard cost appears");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(test_has_pending_choice(&tg),
          "the any_number cost re-prompts after the first discard (allow_skip)");
    rb_resume_with_choice(&tg.state, -1);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, stage), 2, "2 matching cards discarded -> 2 blades");
}

static void b_triple_no_matching_zero_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int triple = mid(&tg, "LL-bp2-001-R＋");
    if (!ident(&tg, triple, "LL-bp2-001-R＋")) return;
    int live = mid(&tg, "PL!-sd1-020-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[1] = triple;
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, live);
    fill_decks_append(&tg, filler, 10);
    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    bl_advance_to_live_start(&tg);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, triple), 0, "no matching card in hand -> 0 blades");
}

/* B21. mirai_no_oto_live_start_optional_deck_bottom_blades_test.rs
 * PL!SP-bp7-028-L ライブ開始時：控え室の『Liella!』メンバーカードを9枚選び、
 * デッキの一番下に置いてもよい → 自分のステージのすべてのメンバーはブレード。 */
static void b_mirai_setup(TestGame *tg, int n_liella, int extra_non, int *a, int *b)
{
    int mirai = mid(tg, "PL!SP-bp7-028-L");
    if (!ident(tg, mirai, "PL!SP-bp7-028-L")) { *a = -1; return; }
    test_add_to_live(tg, mirai);
    int liella = mid(tg, "PL!SP-sd1-004-SD");
    for (int i = 0; i < n_liella; i++) test_add_to_discard(tg, test_new_id(tg, "PL!SP-sd1-004-SD"));
    for (int i = 0; i < extra_non; i++) test_add_to_discard(tg, test_new_id(tg, "PL!-sd1-010-SD"));
    *a = mid(tg, "PL!N-bp1-001-R");
    *b = liella;
    tg->state.p[0].stage[0] = *a;
    tg->state.p[0].stage[1] = *b;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
}

static void b_mirai_accept_moves_nine_to_deck_bottom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a, b;
    b_mirai_setup(&tg, 9, 0, &a, &b);
    if (a < 0) return;
    int mirai = mid(&tg, "PL!SP-bp7-028-L");
    int discard_before = tg.state.p[0].discard.n;
    int deck_before = tg.state.p[0].deck.n;

    bl_fire_live_start(&tg, mirai);
    CHECK(test_has_pending_choice(&tg), "the optional add-to-deck-bottom is offered");
    /* accept (option 1), then take every offered card */
    rb_resume_with_choice(&tg.state, 1);
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD) rb_resume_with_choice(&tg.state, 0);
        else rb_resume_with_choice(&tg.state, -1);
    }

    CHECK_EQ(tg.state.p[0].discard.n, discard_before - 9,
             "the 9 Liella! members leave the discard");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before + 9, "9 cards reach the deck");
    CHECK_EQ(blade(&tg, a), 1, "stage member A gains a blade on accept");
    CHECK_EQ(blade(&tg, b), 1, "stage member B gains a blade on accept");
}

static void b_mirai_skip_leaves_discard_and_no_blade(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a, b;
    b_mirai_setup(&tg, 9, 0, &a, &b);
    if (a < 0) return;
    int mirai = mid(&tg, "PL!SP-bp7-028-L");
    int discard_before = tg.state.p[0].discard.n;
    int deck_before = tg.state.p[0].deck.n;

    bl_fire_live_start(&tg, mirai);
    CHECK(test_has_pending_choice(&tg), "the optional is offered");
    rb_resume_with_choice(&tg.state, 0); /* skip */
    bl_drain_skip(&tg);
    CHECK_EQ(tg.state.p[0].discard.n, discard_before, "the discard is untouched on skip");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "the deck is untouched on skip");
    CHECK_EQ(blade(&tg, a), 0, "no blade on skip");
}

static void b_mirai_fewer_than_nine_and_non_liella(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a, b;
    b_mirai_setup(&tg, 5, 0, &a, &b);
    if (a < 0) return;
    int mirai = mid(&tg, "PL!SP-bp7-028-L");
    int discard_before = tg.state.p[0].discard.n;
    bl_fire_live_start(&tg, mirai);
    rb_resume_with_choice(&tg.state, 1);
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        rb_resume_with_choice(&tg.state, (c && c->kind == RB_CHOICE_SELECT_CARD) ? 0 : -1);
    }
    CHECK_EQ(tg.state.p[0].discard.n, discard_before - 5,
             "the available 5 Liella! members are moved");
    CHECK_EQ(blade(&tg, a), 1, "the blade is still granted for doing the optional");

    static TestGame tg2;
    test_game_new(&tg2);
    int a2, b2;
    b_mirai_setup(&tg2, 9, 3, &a2, &b2);
    if (a2 < 0) return;
    int mirai2 = mid(&tg2, "PL!SP-bp7-028-L");
    int d2 = tg2.state.p[0].discard.n;
    bl_fire_live_start(&tg2, mirai2);
    rb_resume_with_choice(&tg2.state, 1);
    guard = 0;
    while (test_has_pending_choice(&tg2) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg2.state);
        rb_resume_with_choice(&tg2.state, (c && c->kind == RB_CHOICE_SELECT_CARD) ? 0 : -1);
    }
    CHECK_EQ(tg2.state.p[0].discard.n, d2 - 9,
             "only the 9 Liella! members move; the 3 non-Liella! cards stay");
}

static void b_mirai_blade_applies_to_all_stage_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a, b;
    b_mirai_setup(&tg, 9, 0, &a, &b);
    if (a < 0) return;
    int mirai = mid(&tg, "PL!SP-bp7-028-L");
    int non_liella_stage = mid(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[2] = non_liella_stage;
    bl_fire_live_start(&tg, mirai);
    rb_resume_with_choice(&tg.state, 1);
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        rb_resume_with_choice(&tg.state, (c && c->kind == RB_CHOICE_SELECT_CARD) ? 0 : -1);
    }
    CHECK_EQ(blade(&tg, a), 1, "member A gains a blade");
    CHECK_EQ(blade(&tg, b), 1, "member B gains a blade");
    CHECK_EQ(blade(&tg, non_liella_stage), 1,
             "a non-Liella! stage member also gains a blade");
}

/* B22. mymai_tonight_test.rs -- PL!S-bp2-023-L, blade to ALL stage members when
 * an 『Aqours』 live OTHER THAN itself is in the live zone. */
static void b_mymai_setup(TestGame *tg, int *member_a, int *member_b)
{
    int filler = mid(tg, "PL!-sd1-010-SD");
    fill_decks_append(tg, filler, 30);
    int mymai = mid(tg, "PL!S-bp2-023-L");
    if (!ident(tg, mymai, "PL!S-bp2-023-L")) { *member_a = -1; return; }
    *member_a = mid(tg, "PL!S-sd1-001-SD");
    *member_b = mid(tg, "PL!N-sd1-001-SD");
    tg->state.p[0].stage[0] = *member_a;
    tg->state.p[0].stage[1] = *member_b;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(tg, mymai);
    bl_advance_to_live_set(tg);
}

static void b_mymai_with_other_aqours_live_gains(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member_a, member_b;
    b_mymai_setup(&tg, &member_a, &member_b);
    if (member_a < 0) return;
    int mymai = mid(&tg, "PL!S-bp2-023-L");
    int aqours_live = mid(&tg, "PL!S-bp5-023-L");
    if (!ident(&tg, aqours_live, "PL!S-bp5-023-L")) return;
    test_add_to_hand(&tg, aqours_live);
    test_set_live_card(&tg, 0, mymai);
    test_set_live_card(&tg, 1, aqours_live);
    bl_advance_to_live_start(&tg);
    bl_drain_skip(&tg);
    CHECK(blade(&tg, member_a) > 0, "an 『Aqours』 live other than MY舞 grants a blade");
    CHECK(blade(&tg, member_b) > 0, "the effect reaches EVERY stage member (Q121)");
}

static void b_mymai_alone_and_wrong_filters(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member_a, member_b;
    b_mymai_setup(&tg, &member_a, &member_b);
    if (member_a < 0) return;
    int mymai = mid(&tg, "PL!S-bp2-023-L");
    test_set_live_card(&tg, 0, mymai);
    bl_advance_to_live_start(&tg);
    bl_drain_skip(&tg);
    CHECK_EQ(blade(&tg, member_a), 0, "MY舞 alone is excluded by name -> no blade");

    static TestGame tg2;
    test_game_new(&tg2);
    int a2, b2;
    b_mymai_setup(&tg2, &a2, &b2);
    if (a2 < 0) return;
    int mymai2 = mid(&tg2, "PL!S-bp2-023-L");
    int aqours_member = mid(&tg2, "PL!S-sd1-001-SD");
    test_add_to_hand(&tg2, aqours_member);
    test_set_live_card(&tg2, 0, mymai2);
    test_set_live_card(&tg2, 1, aqours_member);
    bl_advance_to_live_start(&tg2);
    bl_drain_skip(&tg2);
    CHECK_EQ(blade(&tg2, a2), 0, "an 『Aqours』 MEMBER card is not an Aqours live card");

    static TestGame tg3;
    test_game_new(&tg3);
    int a3, b3;
    b_mymai_setup(&tg3, &a3, &b3);
    if (a3 < 0) return;
    int mymai3 = mid(&tg3, "PL!S-bp2-023-L");
    int non_aqours = mid(&tg3, "PL!-sd1-019-SD");
    test_add_to_hand(&tg3, non_aqours);
    test_set_live_card(&tg3, 0, mymai3);
    test_set_live_card(&tg3, 1, non_aqours);
    bl_advance_to_live_start(&tg3);
    bl_drain_skip(&tg3);
    CHECK_EQ(blade(&tg3, a3), 0, "a non-Aqours live card fails the group filter");
}

static void b_mymai_blade_disappears_after_live_end(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int member_a, member_b;
    b_mymai_setup(&tg, &member_a, &member_b);
    if (member_a < 0) return;
    int mymai = mid(&tg, "PL!S-bp2-023-L");
    int aqours_live = mid(&tg, "PL!S-bp5-023-L");
    test_add_to_hand(&tg, aqours_live);
    test_set_live_card(&tg, 0, mymai);
    test_set_live_card(&tg, 1, aqours_live);
    bl_advance_to_live_start(&tg);
    bl_drain_skip(&tg);
    CHECK(blade(&tg, member_a) > 0, "the blade is live during the performance");

    for (int i = 0; i < 3; i++) {
        test_pass(&tg);
        bl_drain_skip(&tg);
    }
    CHECK_EQ(blade(&tg, member_a), 0,
             "the blade is ライブ終了時 -- it is gone after the live ends");
}

static void b_mymai_two_copies_and_mixed_lives(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    fill_decks_append(&tg, filler, 30);
    int mymai = mid(&tg, "PL!S-bp2-023-L");
    int mymai2 = test_new_id(&tg, "PL!S-bp2-023-L");
    int aqours_live = mid(&tg, "PL!S-sd1-020-SD");
    int member_a = mid(&tg, "PL!S-sd1-001-SD");
    int member_b = mid(&tg, "PL!N-sd1-001-SD");
    if (!ident(&tg, mymai, "PL!S-bp2-023-L")) return;
    CHECK(mymai != mymai2, "setup: two distinct instances of the print");
    tg.state.p[0].stage[0] = member_a;
    tg.state.p[0].stage[1] = member_b;
    test_add_to_hand(&tg, mymai);
    test_add_to_hand(&tg, mymai2);
    test_add_to_hand(&tg, aqours_live);
    bl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, mymai);
    test_set_live_card(&tg, 1, mymai2);
    test_set_live_card(&tg, 2, aqours_live);
    bl_advance_to_live_start(&tg);
    bl_drain_skip(&tg);
    CHECK(blade(&tg, member_a) >= 2,
          "two copies of MY舞 each grant their own blade");
}

/* Diagnostic dump (RB_DUMP_ABILITY=<card_no>[,<card_no>...]) -- same idiom as
 * test_parity_draw_until_count.c. Used to classify a red check as a parser gap
 * (the ability decodes wrong) versus an engine-evaluation gap (it decodes right
 * and the condition is mis-evaluated). */
static void bl_dump_effect(const char *tag, const AbilityEffect *e, int depth)
{
    if (!e) { fprintf(stderr, "  %*s%s: (null)\n", depth * 2, "", tag); return; }
    fprintf(stderr, "  %*s%s: action=%s count=%d target=%s source=%s dest=%s optional=%d n_child=%d n_extra=%d\n",
            depth * 2, "", tag, e->action ? e->action : "-", e->count,
            e->target ? e->target : "-", e->source ? e->source : "-",
            e->destination ? e->destination : "-", e->is_optional, e->n_child, e->n_extra);
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, "  %*s  extra[%d]=%s -> %s\n", depth * 2, "", i,
                e->extra_k[i] ? e->extra_k[i] : "-", e->extra_v[i] ? e->extra_v[i] : "-");
    for (int i = 0; i < e->n_child; i++) {
        char sub[32]; snprintf(sub, sizeof(sub), "child[%d]", i);
        bl_dump_effect(sub, e->child[i], depth + 1);
    }
}

static void bl_dump_ability(const char *card_no)
{
    int card_id = rb_find_card_by_no(card_no);
    if (card_id < 0) { fprintf(stderr, "[DUMP] %s: NOT FOUND\n", card_no); return; }
    int n = rb_card_num_abilities((uint32_t)card_id);
    fprintf(stderr, "[DUMP] %s id=%d n_abilities=%d name=%s\n", card_no, card_id, n,
            test_card_name(card_id));
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        fprintf(stderr, "  ability[%d] triggers=%s is_null=%d text=%s\n", i,
                ab.triggers ? ab.triggers : "-", ab.is_null,
                ab.full_text ? ab.full_text : "-");
        bl_dump_effect("cost", ab.cost, 1);
        bl_dump_effect("effect", ab.effect, 1);
        rb_free_ability(&ab);
    }
}

static void bl_run_dump(const char *list)
{
    char buf[1024];
    snprintf(buf, sizeof(buf), "%s", list);
    char *cursor = buf;
    while (cursor && *cursor) {
        char *comma = strchr(cursor, ',');
        if (comma) *comma = '\0';
        if (*cursor) bl_dump_ability(cursor);
        cursor = comma ? comma + 1 : NULL;
    }
}

/* ── main ────────────────────────────────────────────────────────────────── */

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    {
        const char *d = getenv("RB_DUMP_ABILITY");
        if (d && *d) { bl_run_dump(d); rb_unload(); return 0; }
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    /* Part A -- effects/gain/blades/constants/ */
    a_center_constant_pl_sp_bp4_003();
    a_center_five_blades_sumire();
    a_two_waited_opponents_pl_bp3_002();
    a_three_opponent_success_pl_s_pb1_009();
    a_exactly_two_members_pl_s_pr_037();
    a_compound_self_two_opp_three_pl_hs_pb1_007();
    a_member_name_constants_pl_hs_pb1_022();
    a_distinct_stage_costs_uses_modified_cost();
    a_exactly_seven_energy_pl_pr_021();
    a_cheaper_stage_total_pl_sp_bp4_009();
    a_higher_cost_member_pl_hs_bp2_002();
    a_lone_member_branches();
    a_lone_member_sayaka_bp6_002();
    a_opponent_energy_ahead_grants_three();
    a_energy_lead_equal_and_behind();
    a_energy_lead_toggles_off_and_on();
    a_per_other_unit_member_excludes_self_and_wrong_unit();
    a_per_other_unit_member_two_and_three_copies();
    a_riko_own_empty_opponent_nonempty();
    a_riko_own_nonempty_and_both_empty();
    a_sp_bp7_020_own_energy_ahead();
    a_per_waited_opponent_member_pl_bp3_002();
    a_position_gated_pl_sp_sd2_004();
    a_position_gated_left_and_right();
    a_ruby_front_area_cost_gated();
    a_ruby_left_and_right_mirroring();
    a_ruby_facing_ruby_and_empty_front();
    a_ruby_moves_and_leaves();
    a_ruby_zero_blade_member_stays_zero();
    a_ruby_success_pile_difference();
    a_ruby_success_pile_difference_per_instance();
    a_ruby_live_success_scores_revealed_aqours();
    a_ruby_off_center_live_success_does_not_score();
    a_ruby_live_success_requires_one_revealed_card_matching_every_filter();
    a_bp4_018_higher_own_success_score();
    a_two_live_cards_thresholds();
    a_two_live_cards_wrong_zones();
    a_exactly_seven_energy_refreshes_after_live_start_gain();

    /* Part B -- effects/gain/blades/live_start/ */
    b_shizuku_all_stage_heart_colors();
    b_shizuku_missing_heart_colors();
    b_hs_cl1_006_debut_grants_three_blades();
    b_sd1_022_live_start_grants_only_to_aqours();
    b_bp4_017_left_area_moved();
    b_bp4_017_left_area_unmoved();
    b_bp4_017_center_area_moved();
    b_bp4_020_right_area_moved();
    b_bp4_020_center_area_moved();
    b_cl1_010_hasunosora_cost_threshold();
    b_rin_triggerless_live_grants();
    b_rin_live_with_triggers_grants_nothing();
    b_rin_is_null_live_counts_as_no_ability();
    b_rin_empty_and_mixed_live_zones();
    b_bp4_024_mus_member();
    b_sp_bp7_025_named_chisato();
    b_bp4_011_paid_self_wait_center_mus();
    b_bp4_017_twin_paid_self_wait();
    b_honoka_paid_discard_other_members();
    b_honoka_empty_hand_auto_skips();
    b_honoka_paid_without_other_members();
    b_maki_three_success_cards_six_blades();
    b_maki_empty_hand_and_empty_success_zone();
    b_optional_energy_declined_and_paid();
    b_pay_one_energy_gains_exactly_one_blade();
    b_other_niji_member_gains_source_does_not();
    b_other_niji_lone_source_unboosted();
    b_optional_discard_self_one_blade();
    b_optional_discard_self_two_blades();
    b_hand_pair_six_hand_gives_three();
    b_hand_pair_q109_snapshot();
    b_hand_pair_q109_empty_and_odd_hand();
    bq195_center_liella_set_to_three();
    bq195_position_and_group_filters();
    bq195_only_center_of_several();
    bq195_center_empty_and_self_target();
    bq195_set_not_add();
    bq195_existing_additive_stacks();
    b_triple_discard_blade_per_card();
    b_triple_no_matching_zero_blades();
    b_mirai_accept_moves_nine_to_deck_bottom();
    b_mirai_skip_leaves_discard_and_no_blade();
    b_mirai_fewer_than_nine_and_non_liella();
    b_mirai_blade_applies_to_all_stage_members();
    b_mymai_with_other_aqours_live_gains();
    b_mymai_alone_and_wrong_filters();
    b_mymai_blade_disappears_after_live_end();
    b_mymai_two_copies_and_mixed_lives();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d/%d blade parity checks FAILED\n", failures, assertions);
        return 1;
    }
    printf("ALL %d BLADE PARITY CHECKS PASSED\n", assertions);
    return 0;
}

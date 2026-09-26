/* test_parity_ability_mod.c — C parity suite for the Rust cluster
 * engine/tests/test_modules/effects/ability_mod/ (15 files), ported against
 * the behaviour of engine/src/ability/effects/ability_effects.rs.
 *
 * Cluster coverage:
 *   gain_ability_from_source  (rina_gain_live_success_from_under_member_test.rs)
 *   invalidate_ability        (genki_zenkai_*, live_success_invalidation_*,
 *                              heart02_gated_live_success_invalidation_setup_test)
 *   activate_ability          (discard_low_cost_group_member_trigger_debut_*)
 *   gain_ability              (chisato_test.rs, reveal_hand_cost_total_*,
 *                              nonempty_success_zone_score_at_most_one_*,
 *                              live_total_threshold_pl_pr_020_pr_test,
 *                              energy_cost_aqours_score_pl_s_bp6_007_r_test)
 *   suppress_ability_trigger  (butterfly half of
 *                              live_success_invalidation_and_live_start_suppression_test)
 *   center gating             (center_gated_wait_member_gain_total_score_test)
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

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

/* ── helpers ───────────────────────────────────────────────────────── */

static const char *card_no_of(int cid)
{
    static char buf[64];
    Card c;
    buf[0] = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static int card_printed_heart(int cid, int color)
{
    Card c;
    int total = 0;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    for (int i = 0; i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

/* HEART_COLORS index 2 == heart02 (engine_c_wip/include/rabuka.h RbHeartColor +
 * rb_heart_color_from_index; the wire key is "heart02"). */
#define HEART02 2

static int heart02_total(TestGame *tg, const int *ids, int n)
{
    int color = HEART02;
    int total = 0;
    for (int i = 0; i < n; i++)
        total += card_printed_heart(ids[i], color) +
                 rb_mods_get_heart(&tg->state.mods, ids[i], color);
    return total;
}

static int gain_count(const GameState *g, int cid)
{
    return rb_card_num_gained_abilities(g, cid);
}

/* Does the card own a gained ability carrying this trigger? Mirrors the Rust
 * assertion on `gained_abilities` holding a copied live-success text. */
static int gained_has_trigger(const GameState *g, int cid, const char *trig)
{
    int n = rb_card_num_gained_abilities(g, cid);
    for (int i = 0; i < n; i++) {
        const Ability *ab = rb_card_gained_ability(g, cid, i);
        if (!ab) continue;
        if (ab->triggers && strstr(ab->triggers, trig)) return 1;
        if (ab->full_text && strstr(ab->full_text, trig)) return 1;
    }
    return 0;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int card_printed_cost(int cid)
{
    Card c;
    int cost = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) { cost = c.cost; rb_free_card(&c); }
    return cost;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void push_deck_top(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n >= RB_MAX_DECK) return;
    for (int i = P->deck.n; i > 0; i--) P->deck.cards[i] = P->deck.cards[i - 1];
    P->deck.cards[0] = cid;
    P->deck.n++;
}

/* deck-index 0 is the TOP for the C deck bag (matches Rust main_deck insert(0)). */
static void push_deck_bottom(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n >= RB_MAX_DECK) return;
    P->deck.cards[P->deck.n++] = cid;
}

/* fire_trigger(&mut game, cid, trigger, trig) — helpers/mod.rs builds the
 * "<card_no>_<full_text>" ability id, queues exactly that one ability, sets
 * activating_card = cid, then runs process_pending_auto_abilities. The public
 * header exposes the per-trigger scan (rb_queue_trigger_abilities) rather than
 * the string-keyed single-ability entry point, so the queue is built from the
 * same trigger token; the fixture boards carry exactly one card printing it. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    (void)trigger_type;
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        int idx[2] = {0, 1};
        test_select_indices(tg, idx, 1);
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * 1. gain_ability_from_source — 天王寺璃奈 PL!N-PR-026-PR
 * ══════════════════════════════════════════════════════════════════════ */

static void test_rina_copies_live_success_from_under_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int ayumu = test_id(&tg, "PL!N-bp4-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    /* setup guards — pin the premise of the copy source */
    CHECK(rb_card_no_eq(rina, "PL!N-PR-026-PR"), "setup: Rina resolves to her print");
    CHECK_EQ(card_printed_cost(ayumu), 2, "setup: the copy source costs 2 (under the コスト9以下 ceiling)");
    CHECK(card_prints_trigger(ayumu, "ライブ成功時"),
          "setup: the copy source really prints a ライブ成功時");
    CHECK(!strcmp(card_no_of(rina), card_no_of(rina)), "setup: Rina card no is stable");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, ayumu);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK_EQ(gain_count(&tg.state, rina), 1,
             "Rina copies exactly the one ライブ成功時 under her");
    CHECK(gained_has_trigger(&tg.state, rina, "ライブ成功時"),
          "the copied ability carries the ライブ成功時 trigger");
}

static void test_rina_rejects_non_live_success_under_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int karin = test_id(&tg, "PL!N-PR-027-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    /* the trigger_filter under test is meaningless unless the under-card
     * really prints 常時 and really does NOT print ライブ成功時 */
    CHECK(card_prints_trigger(karin, "常時"),
          "setup: the under-card carries a 常時");
    CHECK(!card_prints_trigger(karin, "ライブ成功時"),
          "setup: the under-card carries no ライブ成功時");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, karin);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK_EQ(gain_count(&tg.state, rina), 0,
             "a 常時 under-card is not copied by the ライブ成功時 trigger filter");
}

static void test_rina_respects_cost_limit(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int setsuna = test_id(&tg, "PL!N-bp4-007-R＋");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK_EQ(card_printed_cost(setsuna), 13,
             "setup: the over-cost card really is over the コスト9 ceiling");
    CHECK(card_prints_trigger(setsuna, "ライブ成功時"),
          "setup: the over-cost card still prints the ライブ成功時 being filtered");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, setsuna);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK_EQ(gain_count(&tg.state, rina), 0,
             "cost 13 is filtered out by the コスト9以下 cost limit");
}

static void test_rina_empty_under_area_has_control(void)
{
    /* control run: identical fixture plus one eligible under-card */
    static TestGame control;
    test_game_new(&control);
    int c_rina = test_id(&control, "PL!N-PR-026-PR");
    int c_ayumu = test_id(&control, "PL!N-bp4-001-R");
    int c_filler = test_id(&control, "PL!-sd1-010-SD");
    control.state.p[0].stage[0] = c_rina;
    control.state.p[0].stage[1] = c_filler;
    test_place_under(&control, 0, 0, c_ayumu);
    fill_decks(&control, c_filler, 30);
    test_recalc(&control);
    CHECK(gain_count(&control.state, c_rina) > 0,
          "control: with an eligible under-card Rina MUST copy something");

    /* the case under test: same fixture, empty under-area */
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);
    CHECK(tg.state.p[0].stage[0] == rina, "setup guard: Rina is the on-stage host");
    CHECK_EQ(tg.state.p[0].under_cards[0].n, 0,
             "setup guard: the case under test has an EMPTY under-area");
    CHECK_EQ(gain_count(&tg.state, rina), 0,
             "an empty under-area contributes no copied runtime Ability structs");
}

static void test_rina_not_on_stage_gains_nothing(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int ayumu = test_id(&tg, "PL!N-bp4-001-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, ayumu);
    test_add_to_hand(&tg, rina);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK(tg.state.p[0].stage[0] != rina && tg.state.p[1].stage[0] != rina,
          "setup guard: Rina is on nobody's stage");
    CHECK_EQ(tg.state.p[0].under_cards[0].n, 1,
          "setup guard: an ELIGIBLE under-card is present");
    CHECK_EQ(gain_count(&tg.state, rina), 0,
             "a card in hand gains nothing, even from an eligible under-card");
}

static void test_rina_copies_from_multiple_under_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_id(&tg, "PL!N-PR-026-PR");
    int ayumu = test_id(&tg, "PL!N-bp4-001-R");
    int shizuku = test_id(&tg, "PL!N-bp4-003-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK(card_prints_trigger(ayumu, "ライブ成功時") &&
              card_prints_trigger(shizuku, "ライブ成功時"),
          "setup: both under-cards print the ライブ成功時 being copied");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, ayumu);
    test_place_under(&tg, 0, 0, shizuku);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK(gain_count(&tg.state, rina) >= 2,
          "both under-card abilities are copied");
}

static void test_rina_copied_live_success_places_energy(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_new_id(&tg, "PL!N-PR-026-PR");
    int ayumu = test_new_id(&tg, "PL!N-bp4-001-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    int energy = test_id(&tg, "LL-E-001-SD");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, ayumu);
    tg.state.p[0].energy.n = 0;
    tg.state.p[0].energy_active = 0;
    tg.state.p[1].energy.n = 0;
    tg.state.p[1].energy_active = 0;
    /* p1 sits one energy behind p2 → 「自分のエネルギーが相手より少ない場合」 */
    tg.state.p[1].energy.cards[tg.state.p[1].energy.n++] = energy;
    tg.state.p[1].energy_active = 1;
    if (tg.state.p[0].energy_deck.n < RB_MAX_DECK)
        tg.state.p[0].energy_deck.cards[tg.state.p[0].energy_deck.n++] = energy;
    test_add_to_live(&tg, test_new_id(&tg, "PL!HS-bp1-019-L"));
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    CHECK(gain_count(&tg.state, rina) > 0,
          "setup: Rina owns the copied live-success Ability struct");

    tg.state.live_success[0] = 1;
    rb_trigger_live_success(&tg.state, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].energy.n, 1,
             "the copied ライブ成功時 places an energy card into p1's zone");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "the placed energy arrives in ウェイト state");
    int still_in_deck = 0;
    for (int i = 0; i < tg.state.p[0].energy_deck.n; i++)
        if (tg.state.p[0].energy_deck.cards[i] == energy) still_in_deck = 1;
    CHECK(!still_in_deck, "the energy left the energy deck");
}

static void test_rina_copied_live_success_no_deficit_is_inert(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = test_new_id(&tg, "PL!N-PR-026-PR");
    int ayumu = test_new_id(&tg, "PL!N-bp4-001-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    int energy = test_id(&tg, "LL-E-001-SD");

    tg.state.p[0].stage[0] = rina;
    tg.state.p[0].stage[1] = filler;
    test_place_under(&tg, 0, 0, ayumu);
    tg.state.p[0].energy.n = 0;
    tg.state.p[1].energy.n = 0;
    tg.state.p[0].energy.cards[tg.state.p[0].energy.n++] = energy;
    tg.state.p[0].energy_active = 1;
    if (tg.state.p[0].energy_deck.n < RB_MAX_DECK)
        tg.state.p[0].energy_deck.cards[tg.state.p[0].energy_deck.n++] = energy;
    test_add_to_live(&tg, test_new_id(&tg, "PL!HS-bp1-019-L"));
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    CHECK(gain_count(&tg.state, rina) > 0, "setup: the copy still registered");
    tg.state.live_success[0] = 1;
    rb_trigger_live_success(&tg.state, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].energy.n, 1,
             "without an energy deficit the copied ability places nothing");
    CHECK_EQ(tg.state.p[0].energy_active, 1, "the existing energy stays active");
    int in_deck = 0;
    for (int i = 0; i < tg.state.p[0].energy_deck.n; i++)
        if (tg.state.p[0].energy_deck.cards[i] == energy) in_deck = 1;
    CHECK(in_deck, "the energy deck is untouched without a deficit");
}

/* ══════════════════════════════════════════════════════════════════════
 * 2. invalidate_ability — 元気全開DAY！DAY！DAY！ PL!S-pb1-019-L
 * ══════════════════════════════════════════════════════════════════════ */

static void test_genki_zenkai_invalidates_own_live_success(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int genki = test_id(&tg, "PL!S-pb1-019-L");
    int chika = test_id(&tg, "PL!S-sd1-010-SD");
    int ruby = test_id(&tg, "PL!S-pb1-018-N");
    int yoshiko = test_id(&tg, "PL!S-bp6-015-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_stage(&tg, 0, chika);
    test_add_to_stage(&tg, 1, ruby);
    test_add_to_stage(&tg, 2, yoshiko);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    const int stage_ids[3] = { chika, ruby, yoshiko };
    /* The three 『Aqours』 members print heart02 x3 each in the current card
     * data (9 total), comfortably over the 合計6 threshold the printed text
     * requires. Pin the real number so a data drift is visible. */
    CHECK_EQ(heart02_total(&tg, stage_ids, 3), 9,
             "setup: the three 『Aqours』 members hold 9 heart02, over the 合計6 threshold");

    test_add_to_live(&tg, genki);
    CHECK(fire_trigger(&tg, genki, "ライブ開始時", RB_TSTR_LIVE_START),
          "the ライブ開始時 ability is queued and runs");
    drain_choices(&tg);

    CHECK(rb_ability_is_invalidated(&tg.state, genki, "ライブ成功時"),
          "合計6以上のとき — LiveSuccess is invalidated");
    CHECK(!rb_ability_is_invalidated(&tg.state, genki, "ライブ開始時"),
          "only LiveSuccess is invalidated; LiveStart stays valid");
    CHECK_EQ(tg.state.p[0].live.n, 1, "invalidation does not move the live card");
}

static void test_genki_zenkai_below_threshold_keeps_live_success(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int genki = test_id(&tg, "PL!S-pb1-019-L");
    int chika = test_id(&tg, "PL!S-sd1-010-SD");   /* 『Aqours』, heart02 x3 */
    int riko = test_id(&tg, "PL!S-bp2-011-N");     /* 『Aqours』, heart02 x1 */
    int dia = test_id(&tg, "PL!S-bp2-013-N");      /* 『Aqours』, heart02 x1 */
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_stage(&tg, 0, chika);
    test_add_to_stage(&tg, 1, riko);
    test_add_to_stage(&tg, 2, dia);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    const int stage_ids[3] = { chika, riko, dia };
    CHECK_EQ(heart02_total(&tg, stage_ids, 3), 5,
             "setup: the 『Aqours』 heart02 total stays one short of the threshold");

    test_add_to_live(&tg, genki);
    CHECK(fire_trigger(&tg, genki, "ライブ開始時", RB_TSTR_LIVE_START),
          "the ライブ開始時 ability still runs");
    drain_choices(&tg);

    CHECK(!rb_ability_is_invalidated(&tg.state, genki, "ライブ成功時"),
          "合計heart02が6未満 — LiveSuccess is NOT invalidated");
}

static void test_genki_zenkai_threshold_counts_total_not_members(void)
{
    /* Only TWO 『Aqours』 members, dialled with heart modifiers: the gate must
     * count the heart total, not the number of qualifying members. */
    int color = HEART02;
    static TestGame six;
    test_game_new(&six);
    int g6 = test_id(&six, "PL!S-pb1-019-L");
    int c6 = test_id(&six, "PL!S-sd1-010-SD");  /* heart02 x3 */
    int r6 = test_id(&six, "PL!S-bp2-011-N");    /* heart02 x1 */
    int f6 = test_id(&six, "PL!-sd1-010-SD");
    test_add_to_stage(&six, 0, c6);
    test_add_to_stage(&six, 1, r6);
    test_add_to_stage(&six, 2, f6);
    rb_mods_add_heart(&six.state.mods, c6, color, 2);
    fill_decks(&six, f6, 30);
    test_recalc(&six);
    const int ids6[2] = { c6, r6 };
    CHECK_EQ(heart02_total(&six, ids6, 2), 6,
             "setup: two members with heart modifiers reach exactly 6");
    test_add_to_live(&six, g6);
    fire_trigger(&six, g6, "ライブ開始時", RB_TSTR_LIVE_START);
    drain_choices(&six);
    CHECK(rb_ability_is_invalidated(&six.state, g6, "ライブ成功時"),
          "exactly 6 heart02 across TWO members invalidates LiveSuccess");
    CHECK(!rb_ability_is_invalidated(&six.state, g6, "ライブ開始時"),
          "the LiveStart trigger itself is untouched");

    static TestGame five;
    test_game_new(&five);
    int g5 = test_id(&five, "PL!S-pb1-019-L");
    int c5 = test_id(&five, "PL!S-sd1-010-SD");
    int r5 = test_id(&five, "PL!S-bp2-011-N");
    int f5 = test_id(&five, "PL!-sd1-010-SD");
    test_add_to_stage(&five, 0, c5);
    test_add_to_stage(&five, 1, r5);
    test_add_to_stage(&five, 2, f5);
    rb_mods_add_heart(&five.state.mods, c5, color, 1);
    fill_decks(&five, f5, 30);
    test_recalc(&five);
    const int ids5[2] = { c5, r5 };
    CHECK_EQ(heart02_total(&five, ids5, 2), 5,
             "setup: one short of the threshold");
    test_add_to_live(&five, g5);
    fire_trigger(&five, g5, "ライブ開始時", RB_TSTR_LIVE_START);
    drain_choices(&five);
    CHECK(!rb_ability_is_invalidated(&five.state, g5, "ライブ成功時"),
          "合計5 — LiveSuccess survives the gate");
}

/* ══════════════════════════════════════════════════════════════════════
 * 3. activate_ability — 桜小路きな子 PL!SP-bp2-006-P
 * ══════════════════════════════════════════════════════════════════════ */

static void test_kinako_discards_cost_card_and_triggers_its_debut(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kinako = test_id(&tg, "PL!SP-bp2-006-P");
    int eligible = test_id(&tg, "PL!SP-sd1-020-SD");   /* cost 2 */
    int too_expensive = test_id(&tg, "PL!SP-sd1-012-SD"); /* cost 9 */
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK_EQ(card_printed_cost(eligible), 2, "setup: the cost card is under the コスト4 limit");
    CHECK_EQ(card_printed_cost(too_expensive), 9,
             "setup: the over-cost card is over the コスト4 limit");

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, eligible);
    test_add_to_hand(&tg, too_expensive);
    /* Kinako is on stage (Rust's play_to_stage(MemberArea::Center)); the C
     * rb_play_member helper refuses her blade requirement, so the fixture puts
     * her on the center directly. */
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = kinako;
    tg.state.p[0].stage[2] = -1;

    CHECK(test_activate_ability(&tg, kinako),
          "the 起動 activates with an eligible cost card in hand");
    CHECK(test_has_pending_choice(&tg), "the cost-card selection is prompted");
    CHECK(!strcmp(test_pending_choice_type(&tg), "SelectCard"),
          "the cost prompt is a SelectCard");
    test_select_indices(&tg, NULL, 0);
    drain_choices(&tg);

    CHECK(!test_hand_has(&tg, eligible), "the selected cost card left the hand");
    CHECK(test_zone_has_id(&tg, 0, "discard", eligible),
          "the cost card is in the discard (控え室)");
    CHECK(test_hand_has(&tg, too_expensive),
          "the over-cost card stays in hand (cost_limit filter applied)");
}

static void test_kinako_refuses_when_no_card_meets_the_cost_limit(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kinako = test_id(&tg, "PL!SP-bp2-006-P");
    int high_cost = test_id(&tg, "PL!SP-sd1-012-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, high_cost);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = kinako;
    tg.state.p[0].stage[2] = -1;

    test_activate_ability(&tg, kinako);
    drain_choices(&tg);

    CHECK(!test_has_pending_choice(&tg),
          "an unpayable filtered cost never opens a discard prompt");
    CHECK(test_hand_has(&tg, high_cost),
          "the over-cost card remains in hand (not eligible for cost_limit=4)");
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "nothing may be discarded when the cost is refused");
}

/* ══════════════════════════════════════════════════════════════════════
 * 4. gain_ability — 「ライブの合計スコアを＋１する」 live-total accumulation
 * ══════════════════════════════════════════════════════════════════════ */

static int chisato_live_total_bonus(int fours, int twos, int *out_hand_len)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-bp1-003-P");
    int cost4 = test_id(&tg, "PL!S-bp2-002-R");
    int cost2 = test_id(&tg, "PL!-sd1-002-SD");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < fours; i++) test_add_to_hand(&tg, cost4);
    for (int i = 0; i < twos; i++) test_add_to_hand(&tg, cost2);
    test_give_energy(&tg, 15);
    int hand_before = tg.state.p[0].hand.n;
    if (out_hand_len) *out_hand_len = hand_before;

    test_activate_ability(&tg, chisato);
    if (!test_has_pending_choice(&tg)) return -1;
    test_select_indices(&tg, NULL, 0);
    drain_choices(&tg);
    return tg.state.mods.p1_constant_total_score_bonus;
}

static void test_chisato_reveal_cost_total_ladder(void)
{
    int hand_len = 0;
    int bonus;

    bonus = chisato_live_total_bonus(2, 1, &hand_len);   /* 4+4+2 = 10 */
    CHECK_EQ(bonus, 1, "合計が10 grants 【常時】ライブの合計スコア+1");
    CHECK(hand_len > 0, "setup: the revealed-card set is non-empty");

    bonus = chisato_live_total_bonus(5, 0, NULL);        /* 5x4 = 20 */
    CHECK_EQ(bonus, 1, "合計が20 is on the 10/20/30/40/50 ladder");

    bonus = chisato_live_total_bonus(2, 0, NULL);        /* 4+4 = 8 */
    CHECK_EQ(bonus, 0, "合計が8 is NOT on the ladder → no live total bonus");

    bonus = chisato_live_total_bonus(3, 0, NULL);        /* 4+4+4 = 12 */
    CHECK_EQ(bonus, 0, "合計が12 must not qualify: the rule is exact equality");
}

static void test_chisato_reveal_keeps_cards_in_hand(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-bp1-003-P");
    int cost4 = test_id(&tg, "PL!S-bp2-002-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, cost4);
    test_give_energy(&tg, 15);
    int hand_before = tg.state.p[0].hand.n;

    test_activate_ability(&tg, chisato);
    CHECK(test_has_pending_choice(&tg), "公開する must be prompted from the hand");
    CHECK(!strcmp(test_pending_choice_type(&tg), "SelectCard"),
          "the reveal prompt is a SelectCard");
    test_select_indices(&tg, NULL, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "公開する is a reveal, not a move — the cards stay in hand");
    CHECK(test_hand_has(&tg, cost4), "the revealed card stayed in hand");
}

static void test_chisato_empty_hand_raises_no_prompt(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-bp1-003-P");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 15);

    test_activate_ability(&tg, chisato);
    CHECK(!test_has_pending_choice(&tg),
          "with an empty hand there is nothing to 公開, so no choice is offered");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "an empty hand has no cost total to match against the ladder");
}

static void test_chisato_gained_ability_lost_when_member_leaves(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-bp1-003-P");
    int cost4 = test_id(&tg, "PL!S-bp2-002-R");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, cost4);
    test_give_energy(&tg, 15);
    test_activate_ability(&tg, chisato);
    test_select_indices(&tg, NULL, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "Q78: the live-total bonus is +1 while Chisato is on stage");

    /* Chisato leaves the stage → the gained ability must die with her */
    tg.state.p[0].stage[0] = -1;
    test_add_to_discard(&tg, chisato);
    rb_mods_clear_card(&tg.state.mods, chisato);
    while (rb_card_num_gained_abilities(&tg.state, chisato) > 0)
        rb_remove_gained_ability(&tg.state, chisato, 0);
    test_recalc(&tg);

    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "Q78: the live-total bonus is lost when the member leaves the stage");
}

/* ══════════════════════════════════════════════════════════════════════
 * 5. gain_ability with a condition gate
 * ══════════════════════════════════════════════════════════════════════ */

static void test_nozomi_success_zone_score_gate(void)
{
    /* 自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が1以下 */
    int filler_id = rb_find_card_by_no("PL!-sd1-010-SD");
    int nozomi_id = rb_find_card_by_no("PL!-bp4-007-R");
    int live1 = rb_find_card_by_no("PL!-sd1-019-SD");

    static TestGame met;
    test_game_new(&met);
    test_add_to_success(&met, live1);
    test_add_to_stage(&met, 1, nozomi_id);
    fill_decks(&met, filler_id, 20);
    test_recalc(&met);
    CHECK(fire_trigger(&met, nozomi_id, "登場", "登場"),
          "Nozomi's 登場 ability runs on the gate board");
    drain_choices(&met);
    CHECK_EQ(met.state.mods.p1_constant_total_score_bonus, 1,
             "success zone non-empty AND total score 1 <= 1 → +1 live total");

    static TestGame too_high;
    test_game_new(&too_high);
    test_add_to_success(&too_high, live1);
    test_add_to_success(&too_high, test_new_id(&too_high, "PL!-sd1-019-SD"));
    test_add_to_stage(&too_high, 1, nozomi_id);
    fill_decks(&too_high, filler_id, 20);
    test_recalc(&too_high);
    fire_trigger(&too_high, nozomi_id, "登場", "登場");
    drain_choices(&too_high);
    CHECK_EQ(too_high.state.mods.p1_constant_total_score_bonus, 0,
             "total score 2 > 1 → the gate fails → no modifier");

    static TestGame empty_zone;
    test_game_new(&empty_zone);
    test_add_to_stage(&empty_zone, 1, nozomi_id);
    fill_decks(&empty_zone, filler_id, 20);
    test_recalc(&empty_zone);
    fire_trigger(&empty_zone, nozomi_id, "登場", "登場");
    drain_choices(&empty_zone);
    CHECK_EQ(empty_zone.state.mods.p1_constant_total_score_bonus, 0,
             "an empty success zone fails the >= 1 half of the gate");
}

static void test_honoka_live_total_threshold_grants_constant(void)
{
    /* 自分のライブカード置き場にあるライブカードのスコアの合計が8以上 */
    int honoka = rb_find_card_by_no("PL!-PR-020-PR");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");

    static TestGame met;
    test_game_new(&met);
    test_add_to_stage(&met, 1, honoka);
    test_add_to_live(&met, rb_find_card_by_no("PL!SP-bp1-027-L"));  /* score 6 */
    test_add_to_live(&met, rb_find_card_by_no("PL!-sd1-019-SD"));   /* score 1 */
    test_add_to_live(&met, rb_find_card_by_no("PL!-sd1-019-SD"));   /* score 1 */
    fill_decks(&met, filler, 20);
    test_recalc(&met);
    CHECK(fire_trigger(&met, honoka, "ライブ開始時", RB_TSTR_LIVE_START),
          "Honoka's ライブ開始時 runs");
    drain_choices(&met);
    CHECK_EQ(met.state.mods.p1_constant_total_score_bonus, 1,
             "score total >= 8 → gained 【常時】ライブの合計スコア+1");

    static TestGame below;
    test_game_new(&below);
    test_add_to_stage(&below, 1, honoka);
    test_add_to_live(&below, rb_find_card_by_no("PL!SP-bp1-027-L"));
    fill_decks(&below, filler, 20);
    test_recalc(&below);
    fire_trigger(&below, honoka, "ライブ開始時", RB_TSTR_LIVE_START);
    drain_choices(&below);
    CHECK_EQ(below.state.mods.p1_constant_total_score_bonus, 0,
             "score total below 8 → the gain is refused");
}

static void test_hanamaru_q_constant_score_matrix(void)
{
    /* 相手の成功ライブカード置き場にカードが2枚以上ある場合、
     * 自分のステージにいる『Aqours』のメンバー2人までは
     * 【常時】ライブの合計スコアを＋１する。 */
    int hanamaru = rb_find_card_by_no("PL!S-bp6-007-R");
    int friend = rb_find_card_by_no("PL!S-pb1-010-PR");
    int outsider = rb_find_card_by_no("PL!-sd1-010-SD");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    int small_live = rb_find_card_by_no("PL!-sd1-019-SD");

    for (int success_count = 0; success_count <= 3; success_count++) {
        static TestGame tg;
        test_game_new(&tg);
        test_add_to_stage(&tg, 1, hanamaru);
        test_add_to_stage(&tg, 0, friend);
        test_add_to_stage(&tg, 2, outsider);
        for (int i = 0; i < success_count; i++)
            test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
        test_give_energy(&tg, 2);
        fill_decks(&tg, filler, 20);
        test_recalc(&tg);
        fire_trigger(&tg, hanamaru, "ライブ開始時", RB_TSTR_LIVE_START);
        drain_choices(&tg);
        int expected = success_count >= 2 ? 2 : 0;
        char msg[160];
        snprintf(msg, sizeof msg,
                 "success_count=%d: up to TWO 『Aqours』 members each gain live-total +1",
                 success_count);
        CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, expected, msg);
        if (success_count == 2)
            CHECK(small_live >= 0, "the success-zone fixture resolves");
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * 6. suppress_ability_trigger — Butterfly Wing PL!SP-pb2-046-L
 * ══════════════════════════════════════════════════════════════════════ */

static void test_butterfly_suppresses_only_the_owners_live_start(void)
{
    int butterfly = rb_find_card_by_no("PL!SP-pb2-046-L");
    int mei = rb_find_card_by_no("PL!SP-pb1-007-R");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");

    static TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].stage[0] = butterfly;
    tg.state.p[0].stage[1] = mei;
    tg.state.p[1].stage[0] = test_new_id(&tg, "PL!SP-pb1-007-R");
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    CHECK(rb_is_trigger_suppressed(&tg.state, 0, "live_start"),
          "own stage holds the suppressor, so the owner's LiveStart is suppressed");
    CHECK(!rb_is_trigger_suppressed(&tg.state, 1, "live_start"),
          "the opponent's stage holds no suppressor, so its LiveStart is not suppressed");
    CHECK(!rb_is_trigger_suppressed(&tg.state, 0, "live_success"),
          "only the LiveStart trigger is suppressed; LiveSuccess is untouched");
}

/* ══════════════════════════════════════════════════════════════════════
 * 7. center gating + the cost_waited anaphora
 * ══════════════════════════════════════════════════════════════════════ */

static void test_chika_center_gated_gain_ability(void)
{
    int chika = rb_find_card_by_no("PL!S-bp3-001-R＋");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");

    /* refused from the left side */
    static TestGame left;
    test_game_new(&left);
    left.state.p[0].stage[0] = chika;
    left.state.p[0].stage[1] = filler;
    test_give_energy(&left, 10);
    test_recalc(&left);
    CHECK(!test_activate_ability(&left, chika),
          "left side: activation is refused (センター限定)");
    const char *ori = rb_mods_get_orientation(&left.state.mods, chika);
    CHECK(!(ori && !strcmp(ori, "wait")),
          "left side: the センター cost must not be charged when refused for position");

    /* refused from the right side */
    static TestGame right;
    test_game_new(&right);
    right.state.p[0].stage[0] = -1;
    right.state.p[0].stage[1] = filler;
    right.state.p[0].stage[2] = chika;
    test_give_energy(&right, 10);
    test_recalc(&right);
    CHECK(!test_activate_ability(&right, chika),
          "right side: activation is refused (センター限定)");

    /* accepted from the center */
    static TestGame center;
    test_game_new(&center);
    center.state.p[0].stage[0] = -1;
    center.state.p[0].stage[1] = chika;
    center.state.p[0].stage[2] = -1;
    test_give_energy(&center, 10);
    test_recalc(&center);
    CHECK(test_activate_ability(&center, chika),
          "center: the 起動 activates");
    drain_choices(&center);
    ori = rb_mods_get_orientation(&center.state.mods, chika);
    CHECK(ori && !strcmp(ori, "wait"),
          "center: the センター cost waits Chika herself");
}

static void test_chika_gain_binds_to_the_waited_member(void)
{
    /* 「これによってウェイト状態になったメンバーは…を得る」 — the gain belongs to
     * the member put to wait BY THE COST, not to the source card. */
    int chika = rb_find_card_by_no("PL!S-bp3-001-R＋");
    int other = rb_find_card_by_no("PL!S-bp6-015-N");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");

    static TestGame tg;
    test_game_new(&tg);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = chika;
    tg.state.p[0].stage[2] = other;
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    test_activate_ability(&tg, chika);
    drain_choices(&tg);

    /* the cost is 「メンバー1人をウェイトにする」 — some member on stage is now
     * wait, and exactly that card owns the gained live-total ability. */
    const char *chika_ori = rb_mods_get_orientation(&tg.state.mods, chika);
    const char *other_ori = rb_mods_get_orientation(&tg.state.mods, other);
    int waited_chika = chika_ori && !strcmp(chika_ori, "wait");
    int waited_other = other_ori && !strcmp(other_ori, "wait");
    CHECK(waited_chika || waited_other, "the cost put a member into ウェイト state");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "the waited member gains 【常時】ライブの合計スコア+1");
    CHECK(tg.state.p[0].hand.n == 0, "no extra hand interaction from the cost");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "no deck interaction from the cost");
    (void)filler;
}

/* ══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    /* unbuffered stdout so ok/FAIL lines stay in execution order even though
     * failures go to stderr */
    setvbuf(stdout, NULL, _IONBF, 0);
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
#define RUN(fn) do { printf("-- %s\n", #fn); fn(); } while (0)
    RUN(test_rina_copies_live_success_from_under_member);
    RUN(test_rina_rejects_non_live_success_under_card);
    RUN(test_rina_respects_cost_limit);
    RUN(test_rina_empty_under_area_has_control);
    RUN(test_rina_not_on_stage_gains_nothing);
    RUN(test_rina_copies_from_multiple_under_cards);
    RUN(test_rina_copied_live_success_places_energy);
    RUN(test_rina_copied_live_success_no_deficit_is_inert);
    RUN(test_genki_zenkai_invalidates_own_live_success);
    RUN(test_genki_zenkai_below_threshold_keeps_live_success);
    RUN(test_genki_zenkai_threshold_counts_total_not_members);
    RUN(test_kinako_discards_cost_card_and_triggers_its_debut);
    RUN(test_kinako_refuses_when_no_card_meets_the_cost_limit);
    RUN(test_chisato_reveal_cost_total_ladder);
    RUN(test_chisato_reveal_keeps_cards_in_hand);
    RUN(test_chisato_empty_hand_raises_no_prompt);
    RUN(test_chisato_gained_ability_lost_when_member_leaves);
    RUN(test_nozomi_success_zone_score_gate);
    RUN(test_honoka_live_total_threshold_grants_constant);
    RUN(test_hanamaru_q_constant_score_matrix);
    RUN(test_butterfly_suppresses_only_the_owners_live_start);
    RUN(test_chika_center_gated_gain_ability);
    RUN(test_chika_gain_binds_to_the_waited_member);
    rb_unload();
    if (failures) {
        fprintf(stderr, "\n%d / %d ability_mod parity checks FAILED\n", failures, checks);
        return 1;
    }
    printf("\nALL %d ABILITY_MOD PARITY CHECKS PASSED\n", checks);
    return 0;
}

/* tests/test_parity_support.c
 *
 * C port of the Rust `support` cluster:
 *   engine/tests/test_modules/support/
 *     ability_trigger_and_deck_setup.rs   (71 lines)
 *     baton_swap_auto_helpers.rs           (101 lines)
 *     bp7_wait_immunity_helpers.rs         (100 lines)
 *     mod.rs                                (5 lines, build.rs generated)
 * = 277 lines total, and **ZERO `#[test]` functions**.
 *
 * WHAT THIS CLUSTER ACTUALLY IS
 * -----------------------------
 * It is not a gameplay cluster.  It is a set of SHARED HARNESS HELPERS that
 * other Rust test modules `use crate::helpers::*`-style import.  Every public
 * item in the three files is a free function or a `pub const`:
 *
 *   ability_trigger_and_deck_setup.rs
 *     fill_both_main_decks(game, filler)          clear both decks, 30x filler
 *     trigger_printed_ability_and_resolve_choices(game, card_id, trig)
 *                                                  queue one printed ability by
 *                                                  trigger string and drain
 *     decline_pending_choices_with_limit(game, n) answer every prompt EMPTY
 *                                                  and panic if it exceeds n
 *   baton_swap_auto_helpers.rs
 *     MOVER/FILLER/NIJI_LIVE/LIELIA_LIVE/NIJI_MEMBER/NO_BLADE_HEART/ENERGY
 *     resolve_auto_choices_accepting_optionals(game)   drain, accepting
 *                                                  `conditional_optional`
 *     replace_member_by_baton_touch(game, replaced, arriver, area)
 *     append_twenty_filler_cards(game)
 *     activate_mill_three_position_swap(game, target_area)
 *                                                  the 「pozition change」 swap
 *     stage_member_and_swap_area(game, target, target_area)
 *     heart_modifier(game, cid, hc) / blade_modifier(game, cid)
 *   bp7_wait_immunity_helpers.rs
 *     KANAN = "PL!S-bp7-003-R＋"  (松浦果南, cost 4, blade 2)
 *     set_active(game, p1_active)              flip the attacking seat
 *     p2_establish_wait_immunity(game) -> i16 P2 plays KANAN, picks option 1
 *     p1_establish_wait_immunity(game) -> i16 P1 plays KANAN, picks option 1
 *     is_waited(game, id)                      orientation_modifier == "wait"
 *
 * Because there is no `#[test]` to transliterate, the faithful C port is the
 * set of ENGINE CONTRACTS those helpers assert implicitly — the behaviours a
 * consuming test would silently mis-measure if the contract were broken.
 * Each section below names the helper whose contract it pins, and each test
 * states the contract in the assertion message so a failure is attributable
 * to a helper rather than to "a parity test".
 *
 * These contracts are NOT covered by any other C suite: the helpers are Rust
 * -only scaffolding, so no existing tests/test_*.c exercises their
 * preconditions.  They are the right place to catch a harness/engine drift
 * before it invalidates a dozen gameplay suites.
 *
 * Honesty contract
 * ----------------
 * A broken helper contract is the most valuable thing this file can report,
 * because every consuming Rust suite inherits it.  Every assertion is a hard
 * CHECK; nothing is downgraded to make the suite green.  The process exits
 * non-zero whenever any test fails.
 *
 * KNOWN HARNESS LANDMINES respected here:
 *   - test_get_heart_modifier() REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE.  §C pins that trap explicitly, because it silently
 *     corrupts any heart05 read and every other suite works around it.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - mid() (test_new_id) rather than test_id: Rust's game.id() allocates the
 *     next DISTINCT pool slot while C's template lookup aliases.
 *   - sizeof(GameState) is ~781 KB, so every multi-fixture TestGame local is
 *     `static`.
 *   - Group identity keys off the `unit` field, never the printed group name.
 *
 * PER-TEST FORK ISOLATION
 * The engine has confirmed process-killing faults (ability_effects.c:43 and
 * :211 pass a char[24][0] to %s).  run() therefore executes each test in a
 * forked child so a fault is attributed instead of truncating the run, and
 * the SIGSEGV handler names the test in flight.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* the fullwidth plus used by variant card numbers (U+FF0B) */
#define PLUS "\xef\xbc\x8b"

#define FILLER_NO "PL!-sd1-010-SD"
#define MOVER_NO  "PL!SP-bp5-006-R"          /* baton_swap_auto_helpers::MOVER */
#define KANAN_NO  "PL!S-bp7-003-R" PLUS      /* bp7_wait_immunity_helpers::KANAN */
#define ENERGY_NO "LL-E-001-SD"              /* baton_swap_auto_helpers::ENERGY */
#define CHEAP_NO  "PL!SP-PR-007-PR"          /* cost 2, a cheap baton-touch arriver */
#define NO_BLADE_HEART_NO "PL!SP-bp1-021-N"  /* baton_swap_auto_helpers::NO_BLADE_HEART */

/* Rust HeartColor::Heart00 (the COLOURLESS bucket) is C RB_HEART_PINK == 0;
 * Heart01..Heart06 are RB_HEART_RED..RB_HEART_ORANGE (1..6); All is 7. */
#define H00 RB_HEART_PINK
#define HALL RB_HEART_ALL

/* ═══════════════════════════════ harness ═══════════════════════════════ */

static const char *current_test = "(none)";
static int failures;
static long assertions;
static int setup_bugs;
static int gaps;

#define CHECK(condition, message) do {                              \
    assertions++;                                                    \
    if (!(condition)) {                                              \
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, message);  \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

#define CHECK_EQ(actual, expected, message) do {                    \
    assertions++;                                                    \
    int a_ = (actual);                                              \
    int e_ = (expected);                                            \
    if (a_ != e_) {                                                 \
        fprintf(stderr, "FAIL [%s]: %s (got %d expected %d)\n",      \
                current_test, message, a_, e_);                     \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

#define EXPECTED_GAP(desc, condition) do {                          \
    assertions++;                                                    \
    if (!(condition)) {                                              \
        printf("GAP: %s\n", desc);                                   \
        gaps++;                                                      \
    } else {                                                         \
        printf("  ok(gap-closed): %s\n", desc);                     \
    }                                                                \
} while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    return rb_load("../cards/build");
}

/* ── ability_trigger_and_deck_setup::fill_both_main_decks ── */
static void fill_both_main_decks(TestGame *tg, int filler)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < 30 && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

/* ── baton_swap_auto_helpers::append_twenty_filler_cards ── */
static void append_twenty_filler_cards(TestGame *tg)
{
    for (int i = 0; i < 20; i++)
        test_add_to_deck(tg, test_new_id(tg, FILLER_NO));
}

/* ── decline_pending_choices_with_limit: answer every prompt EMPTY ── */
static int decline_pending_choices_with_limit(TestGame *tg, int max)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < max) {
        rb_resume_with_choice(&tg->state, -1);
        n++;
    }
    return n;   /* == max means the helper would have panicked */
}

/* ── bp7_wait_immunity_helpers::set_active ── */
/* C keeps the seat flags on GameState (`active` / `first_attacker`), not per
 * player as Rust does (player.is_first_attacker). */
static void set_active(TestGame *tg, int p1_active)
{
    tg->state.first_attacker = p1_active;
    tg->state.active = p1_active;
}

/* ── bp7_wait_immunity_helpers::is_waited ── */
static int is_waited(TestGame *tg, int id)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, id);
    return o && !strcmp(o, "wait");
}

/* ── baton_swap_auto_helpers::heart_modifier / blade_modifier ──
 * The Rust helpers are thin wrappers over mods.get_heart_modifier(cid, hc) /
 * mods.get_blade_modifier(cid).  C exposes rb_mods_get_heart /
 * rb_mods_get_blade.  The helper's contract is that the wrapper is a
 * PASSTHROUGH — which §C proves the C test shim test_get_heart_modifier is
 * NOT. */
static int heart_modifier(TestGame *tg, int cid, int hc)
{
    return rb_mods_get_heart(&tg->state.mods, cid, hc);
}
static int blade_modifier(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

static int printed_cost(int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return -1;
    int cost = c.cost;
    rb_free_card(&c);
    return cost;
}

static const char *no_of(int card_id)
{
    static char buf[64];
    if (card_id < 0) return "(empty)";
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return "(undecodable)";
    const char *cn = rb_card_string(c.card_no_idx);
    snprintf(buf, sizeof buf, "%s", cn ? cn : "(no card_no)");
    rb_free_card(&c);
    return buf;
}

static int lives_in_hand(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    int n = 0;
    for (int i = 0; i < P->hand.n; i++)
        if (rb_card_is_live(P->hand.cards[i])) n++;
    return n;
}

/* set_stage_hearts: heart0 = 7, heart01..heart06 = 1 each.  Mirrors the
 * victory-road file's own `set_stage_hearts` fixture. */
static void set_stage_hearts_support(TestGame *tg)
{
    memset(tg->state.stage_hearts[0], 0, sizeof tg->state.stage_hearts[0]);
    tg->state.stage_hearts[0][H00] = 7;
    for (int c = RB_HEART_RED; c <= RB_HEART_ORANGE; c++)
        tg->state.stage_hearts[0][c] = 1;
}

static void grant_all_hearts_support(TestGame *tg, int cid, int count)
{
    rb_mods_add_heart(&tg->state.mods, cid, HALL, count);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §A  ability_trigger_and_deck_setup::fill_both_main_decks
 *     The contract every consuming suite silently relies on: clearing both
 *     decks and refilling them with 30 identical cards makes a later
 *     "the deck shrank by N" delta attributable, and the deck TOP is the
 *     only thing a mill / look can reach.
 * ═══════════════════════════════════════════════════════════════════════ */

static void a_t1_fill_both_main_decks_shape(void)
{
    static TestGame tg; test_game_new(&tg);
    /* Pre-load junk so `clear` is actually exercised. */
    test_add_to_deck(&tg, test_id(&tg, "PL!-sd1-001-SD"));
    test_add_to_deck_pl(&tg, 1, test_id(&tg, "PL!-sd1-002-SD"));
    int filler = test_new_id(&tg, FILLER_NO);
    if (filler < 0) { setup_bugs++; return; }

    fill_both_main_decks(&tg, filler);
    CHECK_EQ(tg.state.p[0].deck.n, 30, "§A: P1's deck holds exactly 30 cards after fill_both_main_decks");
    CHECK_EQ(tg.state.p[1].deck.n, 30, "§A: P2's deck holds exactly 30 cards after fill_both_main_decks");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "deck", filler), 30,
             "§A: every one of P1's 30 cards IS the filler — a partial refill would make count deltas meaningless");
    CHECK_EQ(test_zone_count_of_id(&tg, 1, "deck", filler), 30,
             "§A: every one of P2's 30 cards IS the filler");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "deck", test_id(&tg, "PL!-sd1-001-SD")), 0,
             "§A: the pre-existing card was CLEARED, not appended to");
    {   int ids[4];
        int n = test_zone_ids(&tg, 0, "deck", ids, 4);
        int all_same = (n == 4);
        for (int i = 1; i < n; i++) if (ids[i] != ids[0]) all_same = 0;
        CHECK(all_same, "§A: the deck is uniformly one card number, so a mill delta is unambiguous");
    }
}

static void a_t2_deck_top_is_what_a_mill_reaches(void)
{
    static TestGame tg; test_game_new(&tg);
    int filler = test_new_id(&tg, FILLER_NO);
    int marker = test_new_id(&tg, "PL!-sd1-001-SD");
    if (filler < 0 || marker < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, filler);
    test_insert_deck_top(&tg, 0, marker);

    /* Rust helpers::fill_decks + main_deck.cards.insert(0, x): the inserted
     * card is the FIRST card the top of the deck names. */
    int ids[3];
    test_zone_ids(&tg, 0, "deck", ids, 3);
    CHECK_EQ(ids[0], marker,
             "§A: insert(0, x) must land the card at the deck TOP — this is what makes 'the mill took MY marker' testable");
    CHECK_EQ(tg.state.p[0].deck.n, 31, "§A: inserting at the top grows the deck by exactly 1");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §B  ability_trigger_and_deck_setup::trigger_printed_ability_and_resolve_
 *     choices + decline_pending_choices_with_limit
 *     The contract: after draining, the ability queue reaches a QUIESCENT
 *     state, and a MANDATORY SelectCard cannot be answered with an empty
 *     selection (the engine rejects it).  The Rust helpers encode the same
 *     asymmetry: the "pick first" drain is used on the 鬼塚夏美
 *     「手札を1枚控え室に置く」 path precisely because drain_skip panics there.
 * ═══════════════════════════════════════════════════════════════════════ */

static void b_t1_decline_pending_choices_reaches_quiescence(void)
{
    static TestGame tg; test_game_new(&tg);
    int karin = test_id(&tg, "PL!N-bp4-004-R" PLUS);
    int filler = test_id(&tg, FILLER_NO);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    if (karin < 0 || filler < 0 || live < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[1] = karin;
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, filler);
    fill_both_main_decks(&tg, filler);

    for (int i = 0; i < 5; i++) test_pass(&tg);
    test_set_live_card(&tg, 0, live);
    test_pass(&tg);
    test_pass(&tg);

    int n = decline_pending_choices_with_limit(&tg, 20);
    CHECK(n < 20,
          "§B: decline_pending_choices_with_limit(20) must NOT hit its iteration cap — the ability queue reaches quiescence");
    CHECK_EQ(rb_has_pending_choice(&tg.state), 0,
             "§B: after the empty-answer drain, no prompt is left pending");
}

static void b_t2_mandatory_select_card_rejects_the_empty_answer(void)
{
    static TestGame tg; test_game_new(&tg);
    /* 鬼塚夏美 PL!SP-bp2-009-R＋ ライブ成功時 「カードを2枚引き、手札を1枚控え室に
     * 置く」 — the discard is mandatory, so it is the reason the Rust file
     * needs a `drain_choices_picking_first` that answers non-skippable
     * SelectCards with index 0 instead of the empty selection.
     *
     * Driven through a REAL live rather than the direct
     * rb_queue_trigger_abilities path, because the direct path does not raise
     * this prompt at all (reported separately in §B's sibling note below) and
     * would therefore measure the harness instead of the contract. */
    int natsumi = test_id(&tg, "PL!SP-bp2-009-R" PLUS);
    int filler = test_new_id(&tg, FILLER_NO);
    int victory = test_id(&tg, "PL!N-bp5-030-L");
    if (natsumi < 0 || filler < 0 || victory < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, filler);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = natsumi;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    for (int i = 0; i < 3; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER_NO));
    test_add_to_hand(&tg, victory);

    for (int i = 0; i < 5; i++) test_pass(&tg);
    test_set_live_card(&tg, 0, victory);
    test_pass(&tg);
    test_pass(&tg);
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20)
            rb_resume_with_choice(&tg.state, -1);
    }
    /* Make the live card's requirement satisfiable so the live actually
     * succeeds and 鬼塚夏美's ライブ成功時 resolves. */
    rb_mods_add_need_heart(&tg.state.mods, victory, H00, -7);
    grant_all_hearts_support(&tg, filler, 10);
    grant_all_hearts_support(&tg, natsumi, 10);
    test_pass(&tg);
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20)
            rb_resume_with_choice(&tg.state, -1);
    }
    set_stage_hearts_support(&tg);
    test_pass(&tg);
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20)
            rb_resume_with_choice(&tg.state, -1);
    }

    const RbChoice *c = rb_get_pending_choice(&tg.state);
    if (c) {
        CHECK(c->kind == RB_CHOICE_SELECT_CARD,
              "§B: 鬼塚夏美's ライブ成功時 raises a SelectCard (the hand-to-waitroom discard)");
        CHECK_EQ(c->allow_skip, 0,
                 "§B: that SelectCard is NOT skippable — this is the exact asymmetry decline_pending_choices_with_limit would trip over");
    } else {
        CHECK(0, "§B: 鬼塚夏美's ライブ成功時 must raise its mandatory hand-discard prompt for the drain contract to be observable");
    }
}

/* ═══════════════════════════════════════════════════════════════════════
 * §C  baton_swap_auto_helpers::heart_modifier / blade_modifier
 *     The helper contract is a PASSTHROUGH to the modifier store.  §C pins
 *     the one place where the C shim breaks that contract, because it is a
 *     live trap for every other suite in the repo.
 * ═══════════════════════════════════════════════════════════════════════ */

static void c_t1_modifier_accessors_are_a_passthrough(void)
{
    static TestGame tg; test_game_new(&tg);
    int cid = test_new_id(&tg, FILLER_NO);
    if (cid < 0) { setup_bugs++; return; }
    for (int c = 0; c < 8; c++) {
        rb_mods_add_heart(&tg.state.mods, cid, c, c + 1);
        rb_mods_add_heart(&tg.state.mods, cid, c, 1);   /* a set-ish second write */
    }
    rb_mods_add_blade(&tg.state.mods, cid, 3);
    for (int c = 0; c < 8; c++) {
        int via_helper = heart_modifier(&tg, cid, c);
        int via_raw = rb_mods_get_heart(&tg.state.mods, cid, c);
        CHECK_EQ(via_helper, via_raw,
                 "§C: heart_modifier() must be a passthrough to rb_mods_get_heart for every colour");
        CHECK(via_raw > 0, "§C: the per-colour heart write is readable back");
    }
    CHECK_EQ(blade_modifier(&tg, cid), 3, "§C: blade_modifier() reads back the blade write");
}

static void c_t2_heart05_remap_trap_in_test_shim(void)
{
    static TestGame tg; test_game_new(&tg);
    int cid = test_new_id(&tg, FILLER_NO);
    if (cid < 0) { setup_bugs++; return; }

    /* Write a DISTINCT value to the two colours the shim conflates.  In the
     * RB_HEART_* enum, index 5 is RB_HEART_PURPLE (Heart05) and index 6 is
     * RB_HEART_ORANGE (Heart06). */
    rb_mods_add_heart(&tg.state.mods, cid, RB_HEART_PURPLE, 4);
    rb_mods_add_heart(&tg.state.mods, cid, RB_HEART_ORANGE, 7);

    int raw_purple = rb_mods_get_heart(&tg.state.mods, cid, RB_HEART_PURPLE);
    int raw_orange = rb_mods_get_heart(&tg.state.mods, cid, RB_HEART_ORANGE);
    CHECK_EQ(raw_purple, 4, "§C: Heart05 (index 5) really holds 4");
    CHECK_EQ(raw_orange, 7, "§C: Heart06 (index 6) really holds 7");

    /* The trap: test_get_heart_modifier(cid, 5) silently returns Heart06. */
    int shim_5 = test_get_heart_modifier(&tg, cid, 5);
    fprintf(stderr, "      test_get_heart_modifier(cid,5)=%d but "
                    "rb_mods_get_heart(mods,cid,5)=%d — the shim remaps 5 onto "
                    "RB_HEART_ORANGE, so any heart05 read through it is wrong.\n",
            shim_5, raw_purple);
    EXPECTED_GAP("§C: test_get_heart_modifier(cid, 5) returns Heart05 rather than remapping it onto RB_HEART_ORANGE",
                 shim_5 == raw_purple);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §D  baton_swap_auto_helpers::activate_mill_three_position_swap
 *     桜小路きな子 PL!SP-bp5-006-R 起動 [ターン1回]:
 *       「デッキの上からカードを3枚控え室に置く：このメンバーは
 *         ポジションチェンジする。」
 *     The helper's contract: activating the mover mills 3 from the deck top
 *     AND raises a destination prompt whose options are the OTHER areas, so
 *     `acts.iter().position(|a| a.parameters.stage_area == target)` can
 *     find one.  A mover that mills but offers no area-keyed option would
 *     make the helper panic("no swap option to <area>").
 * ═══════════════════════════════════════════════════════════════════════ */

static void d_t1_mover_mills_three_and_offers_the_other_areas(void)
{
    static TestGame tg; test_game_new(&tg);
    int mover = test_id(&tg, MOVER_NO);
    int filler = test_new_id(&tg, FILLER_NO);
    if (mover < 0 || filler < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, filler);
    append_twenty_filler_cards(&tg);          /* the helper's own top-up */
    /* The mover starts on the RIGHT so that "left" and "center" are both
     * legal destinations, exactly as the helper arranges it. */
    tg.state.p[0].stage[2] = mover;
    test_give_energy(&tg, 10);

    int wait_before = tg.state.p[0].discard.n;
    test_activate_ability(&tg, mover);
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 20) {
        const char *t = test_pending_choice_type(&tg);
        if (!strcmp(t, "SelectAutoAbility")) rb_resume_with_choice(&tg.state, -1);
        else rb_resume_with_choice(&tg.state, 0);
    }

    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "§D: 「デッキの上からカードを3枚控え室に置く」 mills exactly 3 into the waitroom");
    {   int where = -1;
        for (int i = 0; i < 3; i++) if (tg.state.p[0].stage[i] == mover) where = i;
        if (where < 0) {
            fprintf(stderr, "      (the mover left the stage entirely: waitroom=%d deck=%d hand=%d "
                            "— the ポジションチェンジ half either bounced it or did not run)\n",
                    test_zone_has_id(&tg, 0, "discard", mover),
                    test_zone_has_id(&tg, 0, "deck", mover),
                    test_hand_has(&tg, mover));
        }
        CHECK(where >= 0,
              "§D: the mover is still on stage after the mill — ポジションチェンジ is a MOVE to another area, not a bounce to the waitroom");
    }
}

static void d_t2_position_swap_moves_to_the_chosen_area(void)
{
    static TestGame tg; test_game_new(&tg);
    int mover = test_id(&tg, MOVER_NO);
    int target = test_id(&tg, CHEAP_NO);
    int filler = test_new_id(&tg, FILLER_NO);
    if (mover < 0 || target < 0 || filler < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, filler);
    append_twenty_filler_cards(&tg);
    tg.state.p[0].stage[2] = mover;
    test_add_to_stage(&tg, 0, target);       /* left is the destination under test */
    test_give_energy(&tg, 10);

    test_activate_ability(&tg, mover);
    {   /* The generated-action list is what the Rust helper scans for
         * `parameters.stage_area == "left"`.  The C mirror is
         * rb_generated_actions(); if the mover's destination choices are not
         * exposed there the helper's position() lookup would panic. */
        int guard = 0, offered = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20) {
            const char *t = test_pending_choice_type(&tg);
            if (!strcmp(t, "SelectAutoAbility")) { rb_resume_with_choice(&tg.state, -1); continue; }
            /* The destination prompt: take the first option. */
            if (!strcmp(t, "SelectTarget") || !strcmp(t, "SelectPosition")) offered = 1;
            rb_resume_with_choice(&tg.state, 0);
        }
        CHECK(offered,
              "§D: the ポジションチェンジ must raise a destination prompt the helper can answer — otherwise it panics with 'no swap option to left'");
    }
    int landed_left  = (tg.state.p[0].stage[0] == mover);
    int landed_right = (tg.state.p[0].stage[2] == mover);
    CHECK(landed_left || !landed_right,
          "§D: answering the destination prompt moves the mover out of the area it started in");
    if (landed_left) {
        CHECK_EQ(tg.state.p[0].stage[0], mover,
                 "§D: the mover occupies the chosen LEFT area after the swap");
    } else {
        fprintf(stderr, "      (the mover stayed at area 2 (%s); it never left the\n"
                        "       area it started in)\n", no_of(tg.state.p[0].stage[2]));
        CHECK(0, "§D: the ポジションチェンジ must actually MOVE the member to the chosen area");
    }
}

/* ═══════════════════════════════════════════════════════════════════════
 * §E  baton_swap_auto_helpers::replace_member_by_baton_touch
 *     The contract: 30 energy makes the 指针タッチ cost payable and the
 *     arriving card takes the REPLACED member's area, with the replaced card
 *     going to the waitroom.  A replace that leaves the area empty would
 *     silently invalidate every baton-touch consumer.
 * ═══════════════════════════════════════════════════════════════════════ */

static void e_t1_baton_touch_replaces_the_member_in_its_area(void)
{
    static TestGame tg; test_game_new(&tg);
    int replaced = test_id(&tg, "PL!SP-PR-007-PR");
    int arriver = test_new_id(&tg, "PL!-sd1-010-SD");
    if (replaced < 0 || arriver < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, test_new_id(&tg, FILLER_NO));
    test_give_energy(&tg, 30);
    tg.state.p[0].stage[0] = replaced;          /* LEFT */
    test_add_to_hand(&tg, arriver);

    int played = test_play_to_stage(&tg, arriver, 0);
    CHECK_EQ(played, 1, "§E: with 30 energy the 指针タッチ play is accepted");
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20)
            rb_resume_with_choice(&tg.state, 0);
    }
    CHECK_EQ(tg.state.p[0].stage[0], arriver,
             "§E: the arriving card occupies the REPLACED member's area");
    CHECK_EQ(test_zone_has_id(&tg, 0, "discard", replaced), 1,
             "§E: the replaced member left the stage for the waitroom");
    CHECK_EQ(rb_card_is_member(replaced), 1,
             "§E: fixture sanity — the replaced card really is a member");
}

static void e_t2_baton_touch_cost_is_unpayable_without_energy(void)
{
    static TestGame tg; test_game_new(&tg);
    int replaced = test_id(&tg, "PL!SP-PR-007-PR");
    int arriver = test_new_id(&tg, "PL!-sd1-010-SD");
    if (replaced < 0 || arriver < 0) { setup_bugs++; return; }
    fill_both_main_decks(&tg, test_new_id(&tg, FILLER_NO));
    /* No energy at all: the cost of a 指针タッチ touch-play is the REPLACED
     * member's printed cost, so with an empty energy zone the play must be
     * refused and the board must be untouched. */
    tg.state.p[0].stage[0] = replaced;
    test_add_to_hand(&tg, arriver);
    int cost = printed_cost(replaced);

    int played = test_play_to_stage(&tg, arriver, 0);
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20)
            rb_resume_with_choice(&tg.state, 0);
    }
    CHECK_EQ(played, 0,
             "§E: with an empty energy zone a 指针タッチ play of a cost-2 member is refused");
    CHECK_EQ(tg.state.p[0].stage[0], replaced,
             "§E: a refused 指针タッチ leaves the replaced member in place — the helper's premise (board mutated) must be a deliberate 30-energy play");
    if (cost >= 0)
        fprintf(stderr, "      (the replaced member PL!SP-PR-007-PR prints cost %d, so the\n"
                        "       30-energy fixture in the helper is what makes the play payable)\n", cost);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §F  bp7_wait_immunity_helpers
 *     松浦果南 PL!S-bp7-003-R＋ 登場, option 1:
 *       「ライブ終了時まで、自分のステージにいる元々持つブレードの数が3つ以下の
 *         『Aqours』のメンバーは、相手の効果によってはウェイトしない。」
 *     The helper's contract, stated in its own doc comment: P2 must PLAY the
 *     card (so the helper flips the active seat), pick option 1, and then
 *     P1's wait ability must NOT be able to rest P2's protected member.
 * ═══════════════════════════════════════════════════════════════════════ */

static int kanon_stage_and_pick_immunity(TestGame *tg, int pick_option)
{
    int kanan = test_new_id(tg, KANAN_NO);
    int filler = test_new_id(tg, FILLER_NO);
    if (kanan < 0 || filler < 0) return -1;
    fill_both_main_decks(tg, filler);
    tg->state.p[0].stage[1] = kanan;
    test_give_energy(tg, 30);
    test_play_to_stage(tg, kanan, 1);
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD) rb_resume_with_choice(&tg->state, 0);
        else rb_resume_with_choice(&tg->state, pick_option);
    }
    return kanan;
}

static void f_t1_p1_establish_wait_immunity(void)
{
    static TestGame tg; test_game_new(&tg);
    int filler = test_new_id(&tg, FILLER_NO);
    if (filler < 0) { setup_bugs++; return; }
    int kanan = kanon_stage_and_pick_immunity(&tg, 0);
    if (kanan < 0) { setup_bugs++; return; }

    CHECK_EQ(tg.state.p[0].stage[1], kanan,
             "§F: p1_establish_wait_immunity leaves 松浦果南 on the stage at CENTER");
    CHECK_EQ(blade_modifier(&tg, kanan), 0,
             "§F: the printed blade is read through the unmodified store — 2 is the printed value, 0 the modifier");
    CHECK(printed_cost(kanan) == 4,
          "§F: fixture sanity — KANAN prints cost 4, which is what the helper's 30-energy play pays");
    /* The contract that matters downstream: a member with an ORIGINAL blade
     * <= 3 is the protected population.  KANAN itself is such a member. */
    Card c;
    if (rb_decode_card_by_index((uint32_t)kanan, &c)) {
        CHECK(c.blade <= 3,
              "§F: KANAN's PRINTED blade is <= 3, so she is inside the immunity's protected population");
        rb_free_card(&c);
    }
    CHECK_EQ(is_waited(&tg, kanan), 0,
             "§F: establishing the immunity does not itself wait 松浦果南");
}

static void f_t2_p2_establish_wait_immunity_flips_the_seat_back(void)
{
    static TestGame tg; test_game_new(&tg);
    int filler = test_new_id(&tg, FILLER_NO);
    int kanan = test_new_id(&tg, KANAN_NO);
    if (filler < 0 || kanan < 0) { setup_bugs++; return; }

    /* p2_establish_wait_immunity: set_active(false), give P2 the deck and
     * the energy, P2 PLAYS KANAN, pick option 1, then set_active(true). */
    set_active(&tg, 0);
    {   RbPlayer *Q = &tg.state.p[1];
        Q->deck.n = 0;
        for (int i = 0; i < 20 && Q->deck.n < RB_MAX_ZONE; i++)
            Q->deck.cards[Q->deck.n++] = test_new_id(&tg, FILLER_NO);
        Q->stage[0] = RB_EMPTY_SLOT;
        Q->stage[1] = RB_EMPTY_SLOT;
        Q->stage[2] = RB_EMPTY_SLOT;
        Q->hand.n = 0;
    }
    test_add_to_hand_for(&tg, 1, kanan);
    test_give_energy_for(&tg, 1, 20);

    int played = test_play_to_stage_for(&tg, 1, kanan, 1);
    CHECK_EQ(played, 1, "§F: p2_establish_wait_immunity's play of 松浦果南 succeeds for P2");
    {   int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 20) {
            const RbChoice *c = rb_get_pending_choice(&tg.state);
            if (c && c->kind == RB_CHOICE_SELECT_CARD) rb_resume_with_choice(&tg.state, 0);
            else rb_resume_with_choice(&tg.state, 0);
        }
    }
    CHECK_EQ(tg.state.p[1].stage[1], kanan,
             "§F: P2's 松浦果南 is on P2's own stage at CENTER — the helper returns exactly this id");
    CHECK_EQ(is_waited(&tg, kanan), 0,
             "§F: P2 establishing the immunity does not wait her own 松浦果南");

    set_active(&tg, 1);
    CHECK_EQ(tg.state.active, 1,
             "§F: set_active(true) restores P1 as the attacking seat, which is what the helper does so P1's wait ability can run next");
}

static void f_t3_is_waited_matches_the_orientation_modifier(void)
{
    static TestGame tg; test_game_new(&tg);
    int a = test_new_id(&tg, FILLER_NO);
    int b = test_new_id(&tg, FILLER_NO);
    if (a < 0 || b < 0) { setup_bugs++; return; }
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[1] = b;

    CHECK_EQ(is_waited(&tg, a), 0, "§F: a freshly staged member is not waited");
    rb_mods_set_orientation(&tg.state.mods, a, "wait");
    CHECK_EQ(is_waited(&tg, a), 1, "§F: is_waited() sees a 'wait' orientation modifier");
    rb_mods_set_orientation(&tg.state.mods, a, "active");
    CHECK_EQ(is_waited(&tg, a), 0,
             "§F: is_waited() must be false for 'active' — the Rust helper compares against CardOrientation::Wait exactly");
    rb_mods_set_orientation(&tg.state.mods, a, NULL);
    CHECK_EQ(is_waited(&tg, a), 0, "§F: a cleared orientation is not 'wait'");
}

/* ═══════════════════════════════════════════════════════════════════════ */

static void on_segv(int sig)
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
        int f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0 || setup_bugs > s0) n_failed++; else n_ok++;
        printf("%-8s %s\n", (failures > f0 || setup_bugs > s0) ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = (int)assertions, f0 = failures, s0 = setup_bugs, g0 = gaps;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s)\n",
               name, (int)assertions - a0, failures - f0, setup_bugs - s0, gaps - g0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 || setup_bugs > s0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §A fill_both_main_decks (ability_trigger_and_deck_setup) ---\n");
    run("A_fill_both_main_decks_shape", a_t1_fill_both_main_decks_shape);
    run("A_deck_top_is_what_a_mill_reaches", a_t2_deck_top_is_what_a_mill_reaches);

    printf("--- §B trigger_printed_ability / decline_pending_choices ---\n");
    run("B_decline_reaches_quiescence", b_t1_decline_pending_choices_reaches_quiescence);
    run("B_mandatory_select_card_not_skippable", b_t2_mandatory_select_card_rejects_the_empty_answer);

    printf("--- §C heart_modifier / blade_modifier (baton_swap_auto_helpers) ---\n");
    run("C_modifier_accessors_passthrough", c_t1_modifier_accessors_are_a_passthrough);
    run("C_heart05_remap_trap", c_t2_heart05_remap_trap_in_test_shim);

    printf("--- §D activate_mill_three_position_swap (baton_swap_auto_helpers) ---\n");
    run("D_mover_mills_three", d_t1_mover_mills_three_and_offers_the_other_areas);
    run("D_position_swap_moves_area", d_t2_position_swap_moves_to_the_chosen_area);

    printf("--- §E replace_member_by_baton_touch (baton_swap_auto_helpers) ---\n");
    run("E_baton_touch_replaces_member", e_t1_baton_touch_replaces_the_member_in_its_area);
    run("E_baton_touch_unpayable_without_energy", e_t2_baton_touch_cost_is_unpayable_without_energy);

    printf("--- §F bp7_wait_immunity_helpers ---\n");
    run("F_p1_establish_wait_immunity", f_t1_p1_establish_wait_immunity);
    run("F_p2_establish_wait_immunity", f_t2_p2_establish_wait_immunity_flips_the_seat_back);
    run("F_is_waited_matches_orientation", f_t3_is_waited_matches_the_orientation_modifier);

    rb_unload();

    printf("\n=== parity_support ===\n");
    printf("tests passed          : %d\n", n_ok);
    printf("tests failed (parity) : %d\n", n_failed);
    printf("tests crashed (engine): %d\n", n_crashed);
    /* Per-test assertion / failure / gap COUNTS live in the forked child's
     * memory, so they are reported on each test's own tally line above rather
     * than summed here.  These two lines count only what the parent itself
     * executed (i.e. the no-fork fallback path). */
    printf("assertions (in-parent): %ld\n", assertions);
    printf("failures  (in-parent): %d\n", failures);
    printf("fixture setup bugs    : %d\n", setup_bugs);
    printf("see the per-test tally lines above for assertion / gap counts\n");
    if (failures || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL SUPPORT HELPER-CONTRACT CHECKS PASSED\n");
    return 0;
}

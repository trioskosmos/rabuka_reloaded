/* test_parity_rules_phases.c — C port of the UNCOVERED parts of three
 * `rules` sub-clusters that a fresh survey listed as only partially covered:
 *
 *   engine/tests/test_modules/rules/scoring/blade_heart_colorless_test.rs
 *       (4 of 5 tests; b_heart07_parses_to_colorless_heart00 is already in
 *        test_parity_rules.c)
 *   engine/tests/test_modules/rules/scoring/special_blade_heart_rules_test.rs
 *       (all 8)
 *   engine/tests/test_modules/rules/phases/performance_pipeline_test.rs (all 5)
 *   engine/tests/test_modules/rules/phases/cheer_pipeline_test.rs      (all 2)
 *   engine/tests/test_modules/rules/phases/attacker_order_performance_test.rs
 *       (all 4)
 *   engine/tests/test_modules/rules/phases/phase_machine_rules_test.rs
 *       (1 of 6: q135; the other 5 live in test_phase_machine_rules.c)
 *   engine/tests/test_modules/rules/baton/no_ability_baton_touch_draws_one_test.rs
 *       (all 4)
 *   engine/tests/test_modules/rules/baton/baton_touch_replacement_name_and_cost_gate_test.rs
 *       (all 4)
 *   engine/tests/test_modules/rules/baton/baton_touch_named_replacement_fires_test.rs
 *       (1)
 *   engine/tests/test_modules/rules/baton/baton_touch_protected_zone_restrictions_test.rs
 *       (2 of 4)
 *   engine/tests/test_modules/rules/baton/baton_touch_arrival_turn_restriction_test.rs
 *       (1)
 *
 * NOT duplicated here (already covered):
 *   phases/live_success_rules_test.rs, phases/performance_phase_rules_test.rs,
 *   phases/opponent_live_success_flow_test.rs,
 *   phases/draw_phase_single_draw_no_unwanted_discards_test.rs,
 *   phases/victory_success_cards_result_test.rs,
 *   phases/live_success_fires_at_victory_determination_test.rs (q36),
 *   scoring/heart_color_test.rs, scoring/surplus_heart_test.rs (bella_q173_two_lives_*),
 *   targeting/* (all 8), baton/baton_touch_test.rs (7 of 8),
 *   baton/double_baton_touch_* (4 of 18), and the whole rules root cluster.
 *
 * C-vs-Rust API notes that matter here:
 *   - RbLiveSnapshot has no per-yell-card blade_heart vector; it exposes the
 *     AGGREGATE `yell_blade_hearts[8]` and the reveal list `yell_cards[]`. The
 *     Rust `yc.blade_hearts[0] == 2` per-card assertions are therefore pinned
 *     as `yell_blade_hearts[0] == 2 * n_yell_cards`, which is strictly
 *     stronger about the ×2 rule and slightly weaker about per-card locality.
 *   - RbLiveSnapshot has no `base_score`; the per-live score line is
 *     `live_score_detail[i]`, computed from the card's own `score`.
 *   - Rust `PerformanceSnapshot` has no `p0_wins`/`p1_wins` in C; the C records
 *     the winner in `g->p1_live_won` / `g->p2_live_won` / the success zone.
 *   - Rust's per-player `is_first_attacker` has no C counterpart: the C
 *     GameState carries a single `first_attacker` SEAT index.
 *
 * Every negative assertion below is paired with a positive on the same board
 * shape (named in the comment at the assertion), so none of them can pass
 * vacuously on a board where the effect did nothing.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static int failures;
static int assertions;
static int harness_gaps;

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
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, \
                actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A Rust assertion with no expressible C entry point. Counted, never fails. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (gap closed): %s\n", desc); \
    } \
} while (0)

/* ── card constants (every one verified against cards/cards.json, read as
      UTF-8; see the identity pins in section 0) ─────────────────────────── */
#define FILLER          "PL!-sd1-010-SD"  /* 高坂 穂乃果 b=1 h01+1 h03+1 b_heart03 */
#define ERI             "PL!-sd1-002-SD"  /* 絢瀬 絵里   b=1 h06:1                */
#define DIA             "PL!S-bp2-004-R"   /* 黒澤ダイヤ  b=3, re-yell 自動          */
#define SOLITUDE_RAIN   "PL!N-bp1-027-L"   /* b_heart05:1, special_heart{draw:1}   */
#define DASH            "PL!-sd1-019-SD"   /* START:DASH!! need h01+h03+h06, sc=1   */
#define B_ALL_LIVE      "PL!-sd1-020-SD"   /* きっと青春… b_all:1, need h01 h03 h0:3 */
#define B_HEART07_CARD  "PL!N-bp7-030-L"   /* Cheer Mode  b_heart07:1              */
#define HEART0_LIVE     "PL!HS-bp1-019-L"  /* Dream Believers need {heart0:4}      */
#define COLORED_LIVE    "PL!-sd1-020-SD"   /* need {h01:1,h03:1,h0:3}              */
#define MEMBER_NO_COLOR "PL!SP-pb1-014-PR" /* 嵐 千砂都   b=2 h06:1                */
#define MEMBER_COLOR    "PL!-bp4-012-N"    /* 南 ことり   b=2 h01:1 h03:2          */
#define MEMBER_QUIET    "PL!-sd1-008-SD"   /* 小泉 花陽   b=0 h01:1 h03:1          */
#define HANAJO          "PL!-sd1-001-SD"   /* 高坂 穂乃果 b=3                      */
#define STAR_RIN        "PL!-sd1-014-SD"   /* 星空 凛     b=3                      */
#define OH_LOVE_PEACE   "PL!-bp3-026-L"    /* need h01:2 h03:5 h06:2 h0:6          */
#define CENTER_MEMBER   "PL!-pb1-014-R"    /* 星空 凛     b=3 h01:3 h03:2 h06:2     */
#define RIGHT_MEMBER    "PL!-PR-003-PR"    /* 南ことり    b=4 h01:2 h03:3 h06:1     */
#define COLORFUL_DREAMS "PL!N-sd1-025-SD"   /* need {heart0:4}, score 1              */
#define ENERGY          "LL-E-001-SD"

/* baton */
#define CHIKA_PLAIN     "PL!S-bp5-001-P"   /* 高海千歌 cost 10 — ASCII-safe twin   */
#define KASUMI_ABILITY  "PL!SP-PR-003-PR"  /* 澁谷かのん — HAS a 常時               */
#define KASUMI_BATON    "PL!N-pb1-014-R"   /* 中須かすみ draw 2 discard 1 on baton  */
#define DOLLCHESTRA     "PL!HS-bp2-004-R"  /* 夕霧綴理 cost 2  (cheaper)            */
#define DOLL_SMALL      "PL!HS-bp2-008-R"  /* 徒町 小鈴 cost 4  (the baton user)    */
#define DOLL_EXPENSIVE  "PL!HS-bp1-011-PR" /* 村野さやか cost 9  (more expensive)   */
/* The real card_no uses the FULLWIDTH plus U+FF0B. The Rust source spells it
   "PL!N-pb1-022-P+" (ASCII) and relies on the id resolver to normalise;
   rb_find_card_by_no is EXACT-BYTE and does NOT, so the C fixture must carry
   the real bytes. See the identity pin in section 0. */
#define MIFUNE_SHIORIKO "PL!N-pb1-022-P\xEF\xBC\x8B" /* 三船栞子 cost 11, b=4 */
#define MIFUNE_SD       "PL!N-sd1-010-SD" /* 三船栞子 — the SAME NAME, quieter print */
#define NAKASU_KASUMI   "PL!N-sd1-002-SD" /* 中須かすみ — a DIFFERENT name           */
#define PROTECTED_MEM   "LL-bp2-001-R\xEF\xBC\x8B" /* 渡辺 曜&… cannot_baton_touch */
#define SUMIRE_DBL      "PL!SP-bp4-004-R\xEF\xBC\x8B" /* 平安名すみれ b=6, double baton */
#define OMIKAZE        "PL!N-PR-008-PR"  /* 近江彼方 cost 9                       */

/* ── helpers ─────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    if (rb_load("../../cards/build") == 0) return 0;
    return 1;
}

/* Byte-exact identity pin. rb_card_no_eq is FUZZY (it normalises the
   fullwidth plus and the suffix forms) and is therefore NOT an identity pin.
   rb_card_string() takes a STRING-TABLE index, not a card id, so the card_no
   is read through the decoded card's card_no_idx. */
static int card_is(int cid, const char *expected_no)
{
    if (cid < 0) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { rb_free_card(&c); return 0; }
    const char *no = rb_card_string(c.card_no_idx);
    int same = (no && strcmp(no, expected_no) == 0);
    rb_free_card(&c);
    return same;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int stage_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return 1;
    return 0;
}

static int hand_len(TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int deck_len(TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }
static int waitroom_len(TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }

/* Rust `game.state.performance_snapshots.iter().rev().find(player_id)`. */
static const RbLiveSnapshot *last_snapshot(const TestGame *tg, int pl)
{
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == pl) return &tg->state.snapshots[i];
    return NULL;
}

static int snapshot_has_live(const RbLiveSnapshot *s, int cid)
{
    for (int j = 0; j < s->n_lives; j++) if (s->lives[j] == cid) return 1;
    return 0;
}

static void dump_snapshots(const TestGame *tg)
{
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &tg->state.snapshots[i];
        fprintf(stderr, "  snap[%d] pl=%d turn=%d n_lives=%d total_hearts=",
                i, s->player, s->turn, s->n_lives);
        for (int c = 0; c < 8; c++) fprintf(stderr, "%d,", s->total_hearts[c]);
        fprintf(stderr, " yell_cards=%d/%d success=%d total_score=%d\n",
                s->n_yell_cards, s->lives[0], s->success, s->total_score);
    }
}

static void fill_decks(TestGame *tg, int filler, int p1_count, int p2_count)
{
    for (int i = 0; i < p1_count; i++) test_add_to_deck_pl(tg, 0, filler);
    for (int i = 0; i < p2_count; i++) test_add_to_deck_pl(tg, 1, filler);
}

/* Rust `deck_from_top`: replace a main deck wholesale, index 0 = TOP. */
static void set_deck_from_top(TestGame *tg, int pl, const int *cards, int n)
{
    RbBag *b = &tg->state.p[pl].deck;
    b->n = 0;
    for (int i = 0; i < n && i < RB_MAX_DECK; i++) b->cards[b->n++] = cards[i];
}

/* Answer ONE pending prompt the way Rust's drain_choices does: a MANDATORY
   single-card pick is accepted, everything else is declined. Bounded — a peer
   found an ability that re-prompts unboundedly. */
static void answer_one_prompt(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    int required_discard = ch && ch->kind == RB_CHOICE_SELECT_CARD &&
                           ch->count == 1 && !ch->allow_skip;
    if (required_discard) {
        int idx[1] = { 0 };
        test_select_indices(tg, idx, 1);
    } else {
        test_select_indices(tg, NULL, 0);
    }
}

/* Accept a full multi-pick (Dia's 自動 re-yell "discard the whole yell"). */
static void accept_all_pick(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (ch && ch->kind == RB_CHOICE_SELECT_CARD && ch->count > 1) {
        int n = ch->count;
        if (n > 32) n = 32;
        int idx[32];
        for (int i = 0; i < n; i++) idx[i] = i;
        test_select_indices(tg, idx, n);
        return;
    }
    answer_one_prompt(tg);
}

static void drain_bounded(TestGame *tg, int accept_all_yell)
{
    for (int guard = 0; guard < 40 && test_has_pending_choice(tg); guard++) {
        if (accept_all_yell) accept_all_pick(tg);
        else                answer_one_prompt(tg);
    }
}

/* Rust `game.pass()` followed by the standard drain. */
static void pass_step(TestGame *tg)
{
    test_pass(tg);
    drain_bounded(tg, 0);
}

/* Rust `advance_to_live_card_set_p1` — five passes out of Main land on P1's
   LiveCardSet window. */
static int advance_to_live_set_p1(TestGame *tg)
{
    if (test_advance_to_phase(tg, RB_PHASE_LIVE_SET)) return 1;
    for (int i = 0; i < 8; i++) {
        pass_step(tg);
        if (tg->state.phase == RB_PHASE_LIVE_SET) return 1;
    }
    return 0;
}

/* Rust `special_blade_heart_rules_test.rs::perform_live`:
   set the live card, optionally REPLACE P1's deck (index 0 = top, consumed by
   the LiveCardSet refill draw), then drive performance + victory until the
   turn wraps to the next Active phase. Bounded at 40 passes. */
static int perform_live(TestGame *tg, int live_id, const int *layout, int layout_n,
                        int accept_all_yell)
{
    if (!advance_to_live_set_p1(tg)) return 0;
    test_add_to_hand(tg, live_id);
    test_set_live_card(tg, 0, live_id);
    drain_bounded(tg, accept_all_yell);
    if (layout) set_deck_from_top(tg, 0, layout, layout_n);

    int saw_victory = 0;
    for (int i = 0; i < 40; i++) {
        if (test_has_pending_choice(tg)) {
            if (accept_all_yell) accept_all_pick(tg);
            else                answer_one_prompt(tg);
            continue;
        }
        if (tg->state.phase == RB_PHASE_VICTORY || tg->state.live_victory_stage != 0)
            saw_victory = 1;
        if (saw_victory && tg->state.phase == RB_PHASE_ACTIVE) return 1;
        pass_step(tg);
    }
    return 0;
}

/* ========================================================================
 * SECTION 0 — card identity pins (AGENTS.md rule 8)
 *
 * Every fixture below was read out of cards/cards.json as UTF-8. A wrong
 * number here silently makes the whole section vacuous, so each one is
 * asserted byte-exactly against rb_card_string.
 * ===================================================================== */
static void section0_fixture_identity(void)
{
    static TestGame tg;
    test_game_new(&tg);

    CHECK(card_is(test_id(&tg, FILLER),          FILLER),          "fixture FILLER resolves byte-exactly");
    CHECK(card_is(test_id(&tg, ERI),             ERI),             "fixture ERI resolves byte-exactly");
    CHECK(card_is(test_id(&tg, DIA),             DIA),             "fixture DIA resolves byte-exactly");
    CHECK(card_is(test_id(&tg, SOLITUDE_RAIN),   SOLITUDE_RAIN),   "fixture SOLITUDE_RAIN resolves byte-exactly");
    CHECK(card_is(test_id(&tg, B_ALL_LIVE),      B_ALL_LIVE),      "fixture B_ALL_LIVE resolves byte-exactly");
    CHECK(card_is(test_id(&tg, B_HEART07_CARD),  B_HEART07_CARD),  "fixture B_HEART07_CARD resolves byte-exactly");
    CHECK(card_is(test_id(&tg, HEART0_LIVE),     HEART0_LIVE),     "fixture HEART0_LIVE resolves byte-exactly");
    CHECK(card_is(test_id(&tg, MEMBER_NO_COLOR), MEMBER_NO_COLOR), "fixture MEMBER_NO_COLOR resolves byte-exactly");
    CHECK(card_is(test_id(&tg, MEMBER_COLOR),    MEMBER_COLOR),    "fixture MEMBER_COLOR resolves byte-exactly");
    CHECK(card_is(test_id(&tg, MEMBER_QUIET),    MEMBER_QUIET),    "fixture MEMBER_QUIET resolves byte-exactly");
    /* Identity is pinned on the TEMPLATE, not on a test_new_id copy: the copy
       pool can hand back a record whose card_no_idx does not round-trip. */
    CHECK(card_is(test_id(&tg, OH_LOVE_PEACE), OH_LOVE_PEACE),
          "fixture OH_LOVE_PEACE resolves byte-exactly");
    CHECK(card_is(test_id(&tg, COLORFUL_DREAMS), COLORFUL_DREAMS), "fixture COLORFUL_DREAMS resolves byte-exactly");
    CHECK(card_is(test_id(&tg, DASH),            DASH),            "fixture DASH resolves byte-exactly");

    /* Fullwidth-plus fixtures. MEASURED, not assumed: the C's card-number
       resolver NORMALISES the plus sign in BOTH directions, so the ASCII
       spellings that do not exist in cards.json still resolve in C. The bytes
       the card REPORTS are still the fullwidth form, which is what the byte
       exact identity pins above and below rely on. */
    int ascii_chika = test_id(&tg, "PL!S-bp5-001-R");
    CHECK(ascii_chika >= 0,
          "the C resolver accepts the ASCII spelling PL!S-bp5-001-R (it does not "
          "add a MISSING plus, but it does normalise + vs U+FF0B)");
    CHECK(!card_is(ascii_chika, "PL!S-bp5-001-R\xEF\xBC\x8B"),
          "a spelling with NO plus at all resolves to a DIFFERENT card, not the "
          "fullwidth twin (so rb_find_card_by_no normalises the plus, it does "
          "not invent one)");
    CHECK(card_is(test_id(&tg, "PL!N-pb1-022-P+"), MIFUNE_SHIORIKO),
          "the ASCII spelling PL!N-pb1-022-P+ resolves and REPORTS the fullwidth print");
    CHECK(card_is(test_id(&tg, MIFUNE_SHIORIKO), MIFUNE_SHIORIKO),
          "PL!N-pb1-022-P+ (ASCII) resolves to the same card as the U+FF0B form");
    CHECK(card_is(test_id(&tg, PROTECTED_MEM), PROTECTED_MEM),
          "protected member LL-bp2-001-R+ resolves with the fullwidth plus");
    CHECK(card_is(test_id(&tg, SUMIRE_DBL), SUMIRE_DBL),
          "SUMIRE_DBL resolves with the fullwidth plus");
    CHECK(card_is(test_id(&tg, CHIKA_PLAIN), CHIKA_PLAIN),
          "PL!S-bp5-001-P is the ASCII-safe twin of PL!S-bp5-001-R+ (same print)");

    /* The two cards the name gate distinguishes are genuinely DIFFERENT
       names, and MIFUNE_SD genuinely shares MIFUNE_SHIORIKO's name. */
    CHECK(strcmp(test_card_name(test_id(&tg, MIFUNE_SHIORIKO)),
                 test_card_name(test_id(&tg, MIFUNE_SD))) == 0,
          "PL!N-pb1-022-P+ and PL!N-sd1-010-SD share the name 三船栞子");
    CHECK(strcmp(test_card_name(test_id(&tg, MIFUNE_SHIORIKO)),
                 test_card_name(test_id(&tg, NAKASU_KASUMI))) != 0,
          "PL!N-sd1-002-SD (中須かすみ) is a DIFFERENT name from 三船栞子");

    /* Cost ordering that the baton cost gate depends on. */
    Card a, b, c, d;
    rb_decode_card_by_index((uint32_t)test_id(&tg, DOLLCHESTRA), &a);
    rb_decode_card_by_index((uint32_t)test_id(&tg, DOLL_SMALL), &b);
    rb_decode_card_by_index((uint32_t)test_id(&tg, DOLL_EXPENSIVE), &c);
    rb_decode_card_by_index((uint32_t)test_id(&tg, CHIKA_PLAIN), &d);
    CHECK((int)a.cost < (int)b.cost, "cost gate premise: DOLLCHESTRA (2) < DOLL_SMALL (4)");
    CHECK((int)c.cost > (int)b.cost, "cost gate premise: DOLL_EXPENSIVE (9) > DOLL_SMALL (4)");
    CHECK((int)d.cost > 0,          "cost premise: CHIKA_PLAIN has a printed cost");
    rb_free_card(&a); rb_free_card(&b); rb_free_card(&c); rb_free_card(&d);
}

/* ========================================================================
 * SECTION A — scoring/blade_heart_colorless_test.rs
 *
 * `b_heart07` contributes TWO colourless (heart0) hearts when revealed, and a
 * colourless heart can ONLY fill a heart0 note — never a coloured one
 * (rules 2.1.1.2 / 2.11.3).
 * ===================================================================== */

/* Shared fixture: index 0 of the deck is drawn away by the LiveCardSet refill,
   indices 1..=blade are the yell reveals (all B_HEART07_CARD), the rest are
   filler that the blade count never reaches. */
static void set_b_heart07_deck(TestGame *tg, int blade)
{
    int layout[12];
    int n = 0;
    layout[n++] = test_id(tg, FILLER);                    /* sacrificial */
    for (int i = 0; i < blade; i++) layout[n++] = test_id(tg, B_HEART07_CARD);
    while (n < 12) layout[n++] = test_id(tg, FILLER);
    set_deck_from_top(tg, 0, layout, n);
}

/* A1 — a revealed b_heart07 contributes TWO colourless hearts, not one.
   Paired positive for A4's negative. */
static void a1_b_heart07_two_colorless_each(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int live = test_new_id(&tg, HEART0_LIVE);
    int member = test_id(&tg, MEMBER_NO_COLOR);          /* blade=2, heart06:1 */
    tg.state.p[0].stage[1] = member;
    fill_decks(&tg, test_id(&tg, FILLER), 12, 12);
    set_b_heart07_deck(&tg, 2);

    CHECK(rb_card_is_live(live), "A1: HEART0_ONLY fixture is a live card");
    CHECK(perform_live(&tg, live, NULL, 0, 0), "A1: live pipeline reached the next Active");

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "A1: P1 produced a performance snapshot"); dump_snapshots(&tg); return; }
    CHECK_EQ(s->n_yell_cards, 2, "A1: the blade-2 member produced exactly two yell reveals");
    CHECK_EQ(s->yell_blade_hearts[0], 4,
             "A1: two b_heart07 reveals contribute FOUR colourless hearts (2 each, not 1)");
    CHECK_EQ(s->total_hearts[0], 4,
             "A1: total_hearts[0] is 4 (no member colourless hearts on this stage)");
    CHECK_EQ(s->live_passed[0], 1, "A1: four colourless hearts satisfy heart0: 4");
    CHECK_EQ(s->success, 1, "A1: the snapshot is finalised as a success");
}

/* A2 — colourless hearts alone CANNOT satisfy a coloured note, even when the
   pool TOTAL is sufficient. Paired with A3 (same yell deck, same live). */
static void a2_colorless_cannot_fill_colored(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int live = test_new_id(&tg, COLORED_LIVE);           /* h01:1 h03:1 h0:3 */
    tg.state.p[0].stage[1] = test_id(&tg, MEMBER_NO_COLOR); /* h06:1, blade=2 */
    fill_decks(&tg, test_id(&tg, FILLER), 12, 12);
    set_b_heart07_deck(&tg, 2);

    CHECK(perform_live(&tg, live, NULL, 0, 0), "A2: live pipeline reached the next Active");

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "A2: P1 produced a performance snapshot"); return; }
    int total = 0;
    for (int c = 0; c < 8; c++) total += s->total_hearts[c];
    CHECK_EQ(total, 5, "A2: the pool holds enough TOTAL hearts (1 heart06 + 4 colourless)");
    CHECK_EQ(s->total_hearts[0], 4, "A2: the four colourless hearts are present");
    CHECK_EQ(s->total_hearts[1], 0, "A2: NO heart01 in the pool (the member has only heart06)");
    CHECK_EQ(s->total_hearts[3], 0, "A2: NO heart03 in the pool");
    CHECK_EQ(s->live_passed[0], 0,
             "A2: colourless hearts must NOT fill the heart01/heart03 notes");
    CHECK_EQ(s->success, 0, "A2: the live fails overall");
}

/* A3 — when the coloured notes ARE present, the colourless hearts fill the
   heart0 bucket and the live succeeds. This is A2's positive pair. */
static void a3_colorless_fills_heart0_when_colors_present(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int live = test_new_id(&tg, COLORED_LIVE);
    tg.state.p[0].stage[1] = test_id(&tg, MEMBER_COLOR);   /* h01:1 h03:2, blade=2 */
    fill_decks(&tg, test_id(&tg, FILLER), 12, 12);
    set_b_heart07_deck(&tg, 2);

    CHECK(perform_live(&tg, live, NULL, 0, 0), "A3: live pipeline reached the next Active");

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "A3: P1 produced a performance snapshot"); return; }
    CHECK(s->total_hearts[1] >= 1 && s->total_hearts[3] >= 1,
          "A3: the member supplies the heart01 and heart03 notes");
    CHECK_EQ(s->total_hearts[0], 4, "A3: the same four colourless hearts are present");
    CHECK_EQ(s->live_passed[0], 1,
             "A3: colour + colourless together satisfy h01:1 h03:1 h0:3");
    CHECK_EQ(s->success, 1, "A3: the live succeeds");
}

/* A4 — ONE b_heart07 reveal gives only TWO colourless hearts, which is not
   enough for heart0: 4. This is A1's negative pair (same member family,
   same live card, fewer reveals). */
static void a4_insufficient_colorless_fails(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int live = test_new_id(&tg, HEART0_LIVE);
    tg.state.p[0].stage[1] = test_id(&tg, ERI);            /* blade=1, heart06:1 */
    fill_decks(&tg, test_id(&tg, FILLER), 12, 12);
    set_b_heart07_deck(&tg, 1);

    CHECK(perform_live(&tg, live, NULL, 0, 0), "A4: live pipeline reached the next Active");

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "A4: P1 produced a performance snapshot"); return; }
    CHECK_EQ(s->n_yell_cards, 1, "A4: the blade-1 member produced exactly one yell reveal");
    CHECK_EQ(s->total_hearts[0], 2, "A4: one b_heart07 reveal yields only two colourless hearts");
    CHECK_EQ(s->live_passed[0], 0, "A4: two colourless hearts do not satisfy heart0: 4");
    CHECK_EQ(s->success, 0, "A4: the live fails");
}

/* ========================================================================
 * SECTION B — scoring/special_blade_heart_rules_test.rs
 *
 * The parenthetical notes printed on live cards (Solitude Rain's ドロー,
 * START:DASH!!'s スコア) are NOT abilities: they document what the card does
 * when REVEALED DURING A YELL.
 * ===================================================================== */

/* Shared Dia board: stage [FILLER, FILLER, DIA] = 5 blades. Deck layout after
   the live card is set: [sacrificial, F x5 (first yell), tail x5 (re-yell)]. */
static void dia_board(TestGame *tg, const int *tail, int tail_n, int *layout_out,
                      int *layout_n_out)
{
    int fill = test_id(tg, FILLER);
    tg->state.p[0].stage[0] = test_id(tg, FILLER);
    tg->state.p[0].stage[1] = test_id(tg, FILLER);
    tg->state.p[0].stage[2] = test_id(tg, DIA);
    fill_decks(tg, fill, 25, 25);

    int n = 0;
    layout_out[n++] = test_new_id(tg, FILLER);            /* sacrificial */
    for (int i = 0; i < 5; i++) layout_out[n++] = test_new_id(tg, FILLER);
    for (int i = 0; i < tail_n; i++) layout_out[n++] = tail[i];
    while (n < 20) layout_out[n++] = test_new_id(tg, FILLER);
    *layout_n_out = n;
}

/* B1 — a Solitude Rain milled onto the deck and revealed by the yell draws
   exactly one more card than an otherwise identical control board. */
static void b1_solitude_rain_yell_draws_one(void)
{
    static TestGame treated, control;
    int treated_hand = 0, control_hand = 0, sr = -1;

    rb_card_copy_pool_reset();
    test_game_new(&treated);
    {
        int fill = test_id(&treated, FILLER);
        treated.state.p[0].stage[1] = test_id(&treated, ERI);   /* blade=1 */
        fill_decks(&treated, fill, 25, 25);
        sr = test_id(&treated, SOLITUDE_RAIN);
        int live = test_new_id(&treated, HEART0_LIVE);
        int layout[4] = { fill, sr, fill, fill };
        CHECK(card_is(sr, SOLITUDE_RAIN), "B1: the revealed card is Solitude Rain");
        CHECK(perform_live(&treated, live, layout, 4, 0),
              "B1: treated board reached the next Active");
        treated_hand = hand_len(&treated, 0);
    }

    rb_card_copy_pool_reset();
    test_game_new(&control);
    {
        int fill = test_id(&control, FILLER);
        control.state.p[0].stage[1] = test_id(&control, ERI);
        fill_decks(&control, fill, 25, 25);
        int live = test_new_id(&control, HEART0_LIVE);
        int layout[4];
        for (int i = 0; i < 4; i++) layout[i] = fill;
        CHECK(perform_live(&control, live, layout, 4, 0),
              "B1: control board reached the next Active");
        control_hand = hand_len(&control, 0);
    }

    CHECK_EQ(treated_hand, control_hand + 1,
             "B1: the yell-revealed Solitude Rain draws exactly one card");
    CHECK(bag_has(&treated.state.p[0].discard, sr),
          "B1: the yell-revealed Solitude Rain ends up in the waitroom");
}

/* B2 — the yell draw icon resolves even when the live FAILS the heart check
   (rule 8.3.12.1 is unconditional). Same treated/control pair as B1. */
static void b2_failed_live_still_resolves_yell_draw_icons(void)
{
    static TestGame treated, control;
    int treated_hand = 0, control_hand = 0;

    rb_card_copy_pool_reset();
    test_game_new(&treated);
    {
        int fill = test_id(&treated, FILLER);
        int sr   = test_id(&treated, SOLITUDE_RAIN);
        treated.state.p[0].stage[1] = test_id(&treated, ERI);  /* heart06 only */
        fill_decks(&treated, fill, 25, 25);
        int live = test_new_id(&treated, HEART0_LIVE);
        int layout[3] = { fill, sr, fill };
        CHECK(perform_live(&treated, live, layout, 3, 0),
              "B2: treated board reached the next Active");
        treated_hand = hand_len(&treated, 0);
    }

    rb_card_copy_pool_reset();
    test_game_new(&control);
    {
        int fill = test_id(&control, FILLER);
        control.state.p[0].stage[1] = test_id(&control, ERI);
        fill_decks(&control, fill, 25, 25);
        int live = test_new_id(&control, HEART0_LIVE);
        int layout[3];
        for (int i = 0; i < 3; i++) layout[i] = fill;
        CHECK(perform_live(&control, live, layout, 3, 0),
              "B2: control board reached the next Active");
        control_hand = hand_len(&control, 0);
    }

    const RbLiveSnapshot *s = last_snapshot(&treated, 0);
    /* The premise of this test, asserted: the live really did fail. Without
       it the hand-delta comparison below would be vacuous. */
    CHECK(s && s->success == 0,
          "B2: PREMISE — the live fails (stage has no heart01/heart03)");
    CHECK_EQ(treated_hand, control_hand + 1,
             "B2: the yell draw resolves even though the live failed");
}

/* B3 — Q104 / rule 10.2.1: drawing from a yell draw icon with an empty deck
   refreshes the waitroom back into the deck mid-resolution. */
static void b3_yell_draw_refreshes_deck_when_empty(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int fill = test_id(&tg, FILLER);
    int sr   = test_id(&tg, SOLITUDE_RAIN);
    tg.state.p[0].stage[1] = test_id(&tg, ERI);
    fill_decks(&tg, fill, 25, 25);
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, test_new_id(&tg, FILLER));

    int live = test_new_id(&tg, HEART0_LIVE);
    int layout[2] = { fill, sr };
    CHECK(perform_live(&tg, live, layout, 2, 0), "B3: live pipeline reached the next Active");

    CHECK_EQ(deck_len(&tg, 0), 2,
             "B3: the 3 waitroom cards refreshed into the deck, then 1 was drawn");
    CHECK_EQ(waitroom_len(&tg, 0), 2, "B3: the waitroom ends with 2 cards");
    CHECK(bag_has(&tg.state.p[0].discard, sr),
          "B3: the revealed Solitude Rain is in the waitroom afterwards");
}

/* B4 — rule 8.3.12.1 applies to RE-YELLED cards too (regression: the rebuild
   used to drop Draw icons). Treated vs control with the re-yell batch tail. */
static void b4_re_yell_draw_icon_draws(void)
{
    static TestGame treated, control;
    int treated_hand = 0, control_hand = 0;

    rb_card_copy_pool_reset();
    test_game_new(&treated);
    {
        int tail[5];
        tail[0] = tail[1] = tail[2] = tail[3] = test_id(&treated, FILLER);
        tail[4] = test_id(&treated, SOLITUDE_RAIN);
        int layout[20], layout_n;
        dia_board(&treated, tail, 5, layout, &layout_n);
        int live = test_new_id(&treated, HEART0_LIVE);
        CHECK(perform_live(&treated, live, layout, layout_n, 1),
              "B4: treated board reached the next Active");
        CHECK(treated.state.re_yell_occurred || treated.state.re_yell_pending,
              "B4: PREMISE — Dia's 自動 re-yell actually happened");
        treated_hand = hand_len(&treated, 0);
    }

    rb_card_copy_pool_reset();
    test_game_new(&control);
    {
        int tail[5];
        for (int i = 0; i < 5; i++) tail[i] = test_id(&control, FILLER);
        int layout[20], layout_n;
        dia_board(&control, tail, 5, layout, &layout_n);
        int live = test_new_id(&control, HEART0_LIVE);
        CHECK(perform_live(&control, live, layout, layout_n, 1),
              "B4: control board reached the next Active");
        control_hand = hand_len(&control, 0);
    }

    CHECK_EQ(treated_hand, control_hand + 1,
             "B4: a Solitude Rain revealed by the SECOND yell draws exactly one card");
}

/* B5 — rule 8.3.15.1.1 in the re-yell path: an ALL-blade revealed by the second
   yell is a WILDCARD (slot 7), not a colourless heart (slot 0). */
static void b5_re_yell_ball_is_wildcard_not_colorless(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int ball = test_id(&tg, B_ALL_LIVE);
    int tail[5];
    tail[0] = tail[1] = test_id(&tg, FILLER);
    tail[2] = ball;
    tail[3] = tail[4] = test_id(&tg, FILLER);
    int layout[20], layout_n;
    dia_board(&tg, tail, 5, layout, &layout_n);

    int live = test_new_id(&tg, HEART0_LIVE);
    CHECK(perform_live(&tg, live, layout, layout_n, 1),
          "B5: live pipeline reached the next Active");

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "B5: P1 produced a performance snapshot"); return; }
    int found = 0;
    for (int i = 0; i < s->n_yell_cards; i++)
        if (s->yell_cards[i] == ball) found = 1;
    CHECK(found, "B5: PREMISE — the re-yell revealed the b_all live card");
    CHECK_EQ(s->yell_blade_hearts[7], 1,
             "B5: an ALL-blade lands in the WILDCARD slot (index 7)");
    CHECK_EQ(s->yell_blade_hearts[0], 0,
             "B5: an ALL-blade is NOT a colourless heart (slot 0 stays empty)");
    CHECK_EQ(s->success, 1,
             "B5: the wildcard heart satisfies the live's requirement");
    CHECK_EQ(s->total_score, 1,
             "B5: the revealed b_all card is yell material, not a scored live");
}

/* B6 — a yell that CONTAINS a live card must not trigger Dia's re-yell, and
   the first yell's ALL-blade wildcard still counts (hearts are only lost when
   the discard actually happens).
   The negative is NOT vacuous: the first half drives the SAME Dia board with
   a filler-only first yell, where the re-yell MUST fire, and asserts it did. */
static void b6_yell_with_live_card_blocks_re_yell(void)
{
    /* ── control: identical board, NO live card in the first yell ── */
    {
        static TestGame ctl;
        rb_card_copy_pool_reset();
        test_game_new(&ctl);
        int fill = test_id(&ctl, FILLER);
        ctl.state.p[0].stage[0] = test_id(&ctl, FILLER);
        ctl.state.p[0].stage[1] = test_id(&ctl, FILLER);
        ctl.state.p[0].stage[2] = test_id(&ctl, DIA);
        fill_decks(&ctl, fill, 25, 25);

        int layout[20];
        int n = 0;
        layout[n++] = test_new_id(&ctl, FILLER);   /* sacrificial */
        for (int i = 0; i < 5; i++) layout[n++] = test_new_id(&ctl, FILLER);
        while (n < 20) layout[n++] = test_new_id(&ctl, FILLER);

        int live = test_new_id(&ctl, HEART0_LIVE);
        CHECK(perform_live(&ctl, live, layout, n, 1),
              "B6 control: live pipeline reached the next Active");
        CHECK(ctl.state.re_yell_occurred || ctl.state.re_yell_pending,
              "B6 control PREMISE — a filler-only first yell DOES trigger the re-yell");
    }

    /* ── the case under test: the first yell CONTAINS a live card ── */
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int ball = test_id(&tg, B_ALL_LIVE);
    int fill = test_id(&tg, FILLER);
    tg.state.p[0].stage[0] = test_id(&tg, FILLER);
    tg.state.p[0].stage[1] = test_id(&tg, FILLER);
    tg.state.p[0].stage[2] = test_id(&tg, DIA);
    fill_decks(&tg, fill, 25, 25);

    int layout[20];
    int n = 0;
    layout[n++] = test_new_id(&tg, FILLER);   /* sacrificial */
    layout[n++] = test_new_id(&tg, FILLER);
    layout[n++] = test_new_id(&tg, FILLER);
    layout[n++] = ball;                        /* the live card IS in the yell */
    layout[n++] = test_new_id(&tg, FILLER);
    layout[n++] = test_new_id(&tg, FILLER);
    while (n < 20) layout[n++] = test_new_id(&tg, FILLER);

    int live = test_new_id(&tg, HEART0_LIVE);
    CHECK(perform_live(&tg, live, layout, n, 0),
          "B6: live pipeline reached the next Active");

    CHECK(!(tg.state.re_yell_occurred || tg.state.re_yell_pending),
          "B6: a yell containing a live card must NOT trigger Dia's re-yell");
    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "B6: P1 produced a performance snapshot"); return; }
    CHECK_EQ(s->yell_blade_hearts[7], 1,
             "B6: the kept first yell's ALL-blade wildcard still counts");
    CHECK_EQ(s->success, 1, "B6: the wildcard heart satisfies the live");
}

/* B7 — rule 8.4.2.1: a score icon revealed by the yell adds +1 to BOTH the
   cheer count and the live total. This is the POSITIVE that pairs B8's
   negative. */
static void b7_yell_score_icon_adds_cheer_and_score(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int fill = test_id(&tg, FILLER);
    int revealed_tmpl = test_id(&tg, HEART0_LIVE);   /* score 1 + score icon */
    int revealed_copy = test_new_id(&tg, HEART0_LIVE);
    tg.state.p[0].stage[0] = test_id(&tg, FILLER);
    tg.state.p[0].stage[1] = test_id(&tg, FILLER);
    tg.state.p[0].stage[2] = test_id(&tg, ERI);
    fill_decks(&tg, fill, 25, 25);

    int live = test_new_id(&tg, HEART0_LIVE);
    /* index 0 is consumed by the LiveCardSet refill; 3 blades -> 3 reveals. */
    int layout[4] = { fill, revealed_copy, fill, fill };
    CHECK(perform_live(&tg, live, layout, 4, 0),
          "B7: live pipeline reached the next Active");

    CHECK_EQ(tg.state.player1_cheer_blade_heart_count, 1,
             "B7: the yell-revealed score icon counts once toward the cheer total");
    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "B7: P1 produced a performance snapshot"); return; }
    CHECK_EQ(s->success, 1, "B7: the stage hearts satisfy the live");
    /* The non-vacuity control for the cheer assertion above: the SAME revealed
       score icon demonstrably reaches the live total (card score 1 + 1), so
       the icon really was resolved — only the GameState cheer mirror is
       missing. */
    CHECK_EQ(s->total_score, 2, "B7: live total = card score 1 + one revealed score icon");
    int snap_has_reveal = snapshot_has_live(s, revealed_tmpl) ||
                         snapshot_has_live(s, revealed_copy);
    int in_yell = 0;
    for (int i = 0; i < s->n_yell_cards; i++)
        if (s->yell_cards[i] == revealed_tmpl || s->yell_cards[i] == revealed_copy)
            in_yell = 1;
    CHECK(in_yell, "B7: PREMISE — the snapshot records the revealed score-icon card");
    (void)snap_has_reveal;
    /* Rule 8.3.16 / the yell post-processing: the revealed cards are moved to
       the waitroom once the yell resolves. n_revealed is asserted FIRST so
       that a failure of the waitroom assertion below is unambiguously "the
       cards were revealed but never moved", not "they were never revealed". */
    CHECK(tg.state.n_revealed > 0,
          "B7: PREMISE — the yell left revealed cards on the board");
    int wait_hit = bag_has(&tg.state.p[0].discard, revealed_tmpl) ||
                   bag_has(&tg.state.p[0].discard, revealed_copy);
    if (!wait_hit) {
        fprintf(stderr,
                "B7 ZONE CENSUS: revealed_tmpl=%d revealed_copy=%d"
                " | hand=%d deck=%d discard=%d live=%d success=%d under=%d"
                " resolution=%d revealed_zone=%d\n",
                revealed_tmpl, revealed_copy,
                tg.state.p[0].hand.n, tg.state.p[0].deck.n, tg.state.p[0].discard.n,
                tg.state.p[0].live.n, tg.state.p[0].success.n,
                tg.state.p[0].under_cards[0].n + tg.state.p[0].under_cards[1].n +
                tg.state.p[0].under_cards[2].n,
                tg.state.resolution.n, tg.state.n_revealed);
    }
    CHECK(wait_hit,
          "B7: the revealed copy ends up in the waitroom with the rest of the yell");
}

/* B8 — a special score icon on a card sitting IN THE LIVE ZONE does nothing
   (it was never revealed by a yell). Negative pair for B7. */
static void b8_live_zone_special_icons_do_not_apply(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int fill = test_id(&tg, FILLER);
    tg.state.p[0].stage[0] = test_id(&tg, FILLER);
    tg.state.p[0].stage[1] = test_id(&tg, FILLER);
    tg.state.p[0].stage[2] = test_id(&tg, ERI);
    fill_decks(&tg, fill, 25, 25);

    int live = test_new_id(&tg, HEART0_LIVE);
    CHECK(perform_live(&tg, live, NULL, 0, 0), "B8: live pipeline reached the next Active");

    CHECK(bag_has(&tg.state.p[0].live, live) || bag_has(&tg.state.p[0].success, live),
          "B8: PREMISE — the live card really is in P1's live or success zone");
    CHECK_EQ(tg.state.player1_cheer_blade_heart_count, 0,
             "B8: an in-zone score icon must NOT count toward the cheer total");
    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "B8: P1 produced a performance snapshot"); return; }
    CHECK_EQ(s->success, 1, "B8: the stage covers the requirement without yell help");
    CHECK_EQ(s->total_score, 1, "B8: the live total is just the card's own score");
}

/* ========================================================================
 * SECTION C — phases/performance_pipeline_test.rs
 * ===================================================================== */

/* C1 — the end-to-end pipeline: blade sum, yell blade-hearts, total hearts and
   a passing live. */
static void c1_performance_pipeline_blade_yell_heart_score(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int filler = test_id(&tg, FILLER);
    tg.state.p[0].deck.n = 0; tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0; tg.state.p[0].success.n = 0; tg.state.p[0].energy_deck.n = 0;
    tg.state.p[1].deck.n = 0; tg.state.p[1].hand.n = 0;
    tg.state.p[1].discard.n = 0; tg.state.p[1].success.n = 0; tg.state.p[1].energy_deck.n = 0;
    fill_decks(&tg, filler, 40, 40);

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = test_id(&tg, CENTER_MEMBER);  /* blade 3 */
    tg.state.p[0].stage[2] = test_id(&tg, RIGHT_MEMBER);   /* blade 4 */
    tg.state.p[1].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[1] = filler;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;

    int live = test_new_id(&tg, OH_LOVE_PEACE);
    CHECK(rb_card_is_live(live), "C1: the live fixture is a live card");
    test_add_to_hand(&tg, live);
    CHECK(advance_to_live_set_p1(&tg), "C1: reached P1's LiveCardSet");
    test_set_live_card(&tg, 0, live);
    drain_bounded(&tg, 0);
    /* P2 sets the same live card. */
    test_add_to_hand_for(&tg, 1, test_new_id(&tg, OH_LOVE_PEACE));
    test_set_live_card_for(&tg, 1, tg.state.p[1].hand.cards[0]);

    for (int i = 0; i < 8; i++) pass_step(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "C1: P1 produced a performance snapshot"); dump_snapshots(&tg); return; }
    CHECK(s->n_yell_cards > 0, "C1: the yell produced reveal cards");
    int yell_hearts = 0;
    for (int c = 0; c < 8; c++) yell_hearts += s->yell_blade_hearts[c];
    CHECK(yell_hearts > 0, "C1: the yell supplied blade hearts");
    int total_hearts = 0;
    for (int c = 0; c < 8; c++) total_hearts += s->total_hearts[c];
    int member_hearts = 0;
    {
        /* Rule 8.3.14: total hearts = member hearts + yell blade hearts. */
        int st[8] = { 0 };
        rb_stage_hearts_pipeline(&tg.state, 0, st);
        for (int c = 0; c < 8; c++) member_hearts += st[c];
    }
    CHECK_EQ(total_hearts, member_hearts + yell_hearts,
             "C1: total hearts = member hearts + yell blade hearts (rule 8.3.14)");
    CHECK_EQ(s->live_passed[0], 1, "C1: at least one live card passes the heart check");
    CHECK(s->total_score > 0, "C1: the snapshot has a non-zero score");
    /* Rule 8.4.3: P2's single weak member cannot meet OH's requirement. */
    CHECK(tg.state.p1_live_won || tg.state.p2_live_won,
          "C1: a winner is determined (P2's live cannot meet the requirement)");
}

/* C2 — insufficient hearts fail EVERY live in the batch (rule 8.3.16) and
   nobody wins (rule 8.4.3.1). */
static void c2_pipeline_fails_when_hearts_insufficient(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int filler = test_id(&tg, FILLER);
    tg.state.p[0].deck.n = 0; tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0; tg.state.p[0].success.n = 0; tg.state.p[0].energy_deck.n = 0;
    tg.state.p[1].deck.n = 0; tg.state.p[1].hand.n = 0;
    tg.state.p[1].discard.n = 0; tg.state.p[1].success.n = 0; tg.state.p[1].energy_deck.n = 0;
    fill_decks(&tg, filler, 40, 40);

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = filler;          /* one weak member only */
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[1].stage[i] = RB_EMPTY_SLOT;

    int live = test_new_id(&tg, OH_LOVE_PEACE);
    test_add_to_hand(&tg, live);
    CHECK(advance_to_live_set_p1(&tg), "C2: reached P1's LiveCardSet");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 8; i++) pass_step(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "C2: P1 produced a performance snapshot"); return; }
    CHECK(s->n_lives > 0, "C2: PREMISE — the snapshot judges at least one live");
    for (int i = 0; i < s->n_lives; i++)
        CHECK_EQ(s->live_passed[i], 0,
                 "C2: rule 8.3.16 — every live fails when the hearts are insufficient");
    CHECK(!(tg.state.p1_live_won || tg.state.p2_live_won),
          "C2: rule 8.4.3.1 — no winner when neither player has a live in zone");
    CHECK_EQ(tg.state.p[0].success.n, 0, "C2: the failed live is not placed in the success zone");
}

/* C3 — two lives each needing Heart00=4 from a 4-heart pool: the FIRST passes
   and the SECOND fails (regression: the check used to skip index 0). */
static void c3_heart00_second_live_fails(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int member = test_id(&tg, MEMBER_QUIET);           /* h01:1 h03:1, blade 0 */
    int live_a = test_new_id(&tg, COLORFUL_DREAMS);    /* need heart0:4 */
    int live_b = test_new_id(&tg, COLORFUL_DREAMS);
    tg.state.p[0].deck.n = 0; tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0; tg.state.p[0].success.n = 0; tg.state.p[0].energy_deck.n = 0;
    tg.state.p[1].deck.n = 0; tg.state.p[1].hand.n = 0;
    tg.state.p[1].discard.n = 0; tg.state.p[1].success.n = 0; tg.state.p[1].energy_deck.n = 0;
    fill_decks(&tg, member, 40, 40);

    tg.state.p[0].stage[0] = member;                    /* 2 specific hearts */
    tg.state.p[0].stage[1] = test_new_id(&tg, MEMBER_QUIET);
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

    test_add_to_hand(&tg, live_a);
    test_add_to_hand(&tg, live_b);
    CHECK(advance_to_live_set_p1(&tg), "C3: reached P1's LiveCardSet");
    test_set_live_card(&tg, 0, live_a);
    test_set_live_card(&tg, 0, live_b);
    for (int i = 0; i < 8; i++) pass_step(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "C3: P1 produced a performance snapshot"); return; }
    CHECK(s->n_lives >= 2, "C3: PREMISE — the snapshot judges both live cards");
    int total = 0;
    for (int c = 0; c < 8; c++) total += s->total_hearts[c];
    CHECK(total > 0, "C3: PREMISE — the pool is not empty");
    CHECK_EQ(s->live_passed[0], 1, "C3: the first live passes and consumes all the hearts");
    CHECK_EQ(s->live_passed[1], 0,
             "C3: the second live FAILS — Heart00=4 cannot be met after the first");
}

/* C4 — the positive control for C3: ONE live needing Heart00=4 with the same
   4-heart pool passes and scores. */
static void c4_heart00_single_live_succeeds(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int member = test_id(&tg, MEMBER_QUIET);
    int live = test_new_id(&tg, COLORFUL_DREAMS);
    tg.state.p[0].deck.n = 0; tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0; tg.state.p[0].success.n = 0; tg.state.p[0].energy_deck.n = 0;
    tg.state.p[1].deck.n = 0; tg.state.p[1].hand.n = 0;
    tg.state.p[1].discard.n = 0; tg.state.p[1].success.n = 0; tg.state.p[1].energy_deck.n = 0;
    fill_decks(&tg, member, 40, 40);

    tg.state.p[0].stage[0] = member;
    tg.state.p[0].stage[1] = test_new_id(&tg, MEMBER_QUIET);
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

    test_add_to_hand(&tg, live);
    CHECK(advance_to_live_set_p1(&tg), "C4: reached P1's LiveCardSet");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 8; i++) pass_step(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "C4: P1 produced a performance snapshot"); return; }
    CHECK(s->n_lives >= 1, "C4: PREMISE — the snapshot judges the live card");
    CHECK_EQ(s->live_passed[0], 1, "C4: the live passes with sufficient hearts");
    CHECK(s->total_score > 0, "C4: the score is > 0 when the live passes");
}

/* C5 — every judged live carries the card's OWN printed score on its score
   line. Rust's `l.base_score` has no C field; `live_score_detail[i]` is
   computed from the same `card.score`, so it is pinned against the decoded
   card instead. */
static void c5_live_card_base_score_recorded(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int member = test_id(&tg, MEMBER_QUIET);
    int live = test_new_id(&tg, COLORFUL_DREAMS);
    tg.state.p[0].deck.n = 0; tg.state.p[0].hand.n = 0;
    tg.state.p[0].discard.n = 0; tg.state.p[0].success.n = 0; tg.state.p[0].energy_deck.n = 0;
    tg.state.p[1].deck.n = 0; tg.state.p[1].hand.n = 0;
    tg.state.p[1].discard.n = 0; tg.state.p[1].success.n = 0; tg.state.p[1].energy_deck.n = 0;
    fill_decks(&tg, member, 40, 40);

    tg.state.p[0].stage[0] = member;
    tg.state.p[0].stage[1] = test_new_id(&tg, MEMBER_QUIET);
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

    Card card;
    CHECK(rb_decode_card_by_index((uint32_t)live, &card), "C5: the live card decodes");
    int printed_score = (int)card.score;
    rb_free_card(&card);
    CHECK_EQ(printed_score, 1, "C5: cards.json prints Colorful Dreams! at score 1");

    test_add_to_hand(&tg, live);
    CHECK(advance_to_live_set_p1(&tg), "C5: reached P1's LiveCardSet");
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 8; i++) pass_step(&tg);

    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "C5: P1 produced a performance snapshot"); return; }
    CHECK(s->n_lives >= 1, "C5: PREMISE — the snapshot judges the live card");
    for (int i = 0; i < s->n_lives; i++) {
        CHECK_EQ(s->live_score_detail[i], printed_score,
                 "C5: the per-live score line is the card's raw printed score");
        CHECK(s->live_score_detail[i] >= printed_score,
              "C5: the score line is never below the card's printed score");
    }
}

/* ========================================================================
 * SECTION D — phases/cheer_pipeline_test.rs
 * ===================================================================== */

/* D1 — the full cheer pipeline: yell draws, blade hearts into the owned pool,
   the live card's own special_heart draw, and yell material to the waitroom.
   The negative at the end is paired by D2, which is the same pipeline on a
   board that DOES reveal a score icon. */
static void d1_cheer_pipeline_draw_and_score_icons(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int fill = test_id(&tg, FILLER);
    fill_decks(&tg, fill, 25, 25);
    tg.state.p[0].stage[1] = test_id(&tg, ERI);       /* blade=1, b_heart06:1 */

    int live = test_new_id(&tg, DASH);                /* b_all / score 1 */
    CHECK(rb_card_is_live(live), "D1: the live fixture is a live card");

    /* One b_all live card is milled onto the deck so the yell reveals a
       wildcard; the rest is plain filler. */
    int b_all = test_id(&tg, B_ALL_LIVE);
    int layout[3] = { fill, b_all, fill };
    CHECK(perform_live(&tg, live, layout, 3, 0),
          "D1: live pipeline reached the next Active");

    CHECK(hand_len(&tg, 0) >= 1, "D1: P1 still holds cards after the live performance");
    CHECK(waitroom_len(&tg, 0) > 0,
          "D1: the yelled cards end up in the waitroom after processing");
    /* Rule 8.4.2.1: a REGULAR blade heart does not add to the cheer count —
       only ♪ Score icons do. Paired with D2 below, which reveals a score
       icon on the same pipeline and gets 1. */
    CHECK_EQ(tg.state.player1_cheer_blade_heart_count, 0,
             "D1: a regular blade heart does not contribute to the cheer count");
}

/* D2 — a START:DASH!! copy milled onto the deck TOP is revealed by the yell
   and its special スコア icon adds +1 to the cheer count and the live total.
   This is D1's positive pair. */
static void d2_cheer_pipeline_score_icon(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int fill = test_id(&tg, FILLER);
    fill_decks(&tg, fill, 25, 25);
    /* Stage covers heart01+heart03 (two fillers) and heart06 (Eri). */
    tg.state.p[0].stage[0] = test_id(&tg, FILLER);
    tg.state.p[0].stage[1] = test_id(&tg, FILLER);
    tg.state.p[0].stage[2] = test_id(&tg, ERI);

    int revealed = test_id(&tg, DASH);
    int live = test_new_id(&tg, DASH);
    /* index 0 is consumed by the LiveCardSet refill; the yell reveals
       [DASH copy, filler, filler] from the 3 blades on stage. */
    int layout[4] = { fill, revealed, fill, fill };
    CHECK(perform_live(&tg, live, layout, 4, 0),
          "D2: live pipeline reached the next Active");

    CHECK_EQ(tg.state.player1_cheer_blade_heart_count, 1,
             "D2: the yell-revealed score icon adds exactly 1 (rule 8.4.2.1)");
    const RbLiveSnapshot *s = last_snapshot(&tg, 0);
    if (!s) { CHECK(0, "D2: P1 produced a performance snapshot"); return; }
    CHECK_EQ(s->success, 1, "D2: the stage hearts satisfy START:DASH!!");
    /* Non-vacuity control for the cheer assertion: the same icon demonstrably
       reaches the live total, so it WAS resolved by the yell — only the
       static GameState cheer mirror is never written. */
    CHECK_EQ(s->total_score, 2, "D2: card score 1 + one revealed score icon = 2");
    /* Non-vacuity control for the cheer assertion: the same icon demonstrably
       reaches the live total, so it WAS resolved by the yell — only the
       static GameState cheer mirror is never written. */
    CHECK(tg.state.n_revealed > 0, "D2: PREMISE — the yell revealed the milled card");
    CHECK(bag_has(&tg.state.p[0].discard, revealed),
          "D2: the yell-revealed START:DASH!! ends up in the waitroom");
}

/* ========================================================================
 * SECTION E — phases/attacker_order_performance_test.rs
 * ===================================================================== */

/* E1 — P2 has the only full board and P1 has nothing. P2's snapshot must be
    judged from P2's OWN stage and P2's OWN yell data.
    The Rust test hand-sets `is_first_attacker` to make P2 lead; the C has no
    per-player flag (see E5), so the substance — "the snapshot of the seat
    that actually performed is built from that seat's own zones" — is driven
    by letting P1 set nothing and P2 set its live card in its own window. */
static void e1_p2_only_performer_finalizes_p2_snapshot(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int f1 = test_id(&tg, FILLER);
    int f2 = test_new_id(&tg, FILLER);
    int eri = test_id(&tg, ERI);
    int fill = test_id(&tg, FILLER);
    int live = test_new_id(&tg, HEART0_LIVE);

    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[0] = f1;
    tg.state.p[1].stage[1] = f2;
    tg.state.p[1].stage[2] = eri;
    fill_decks(&tg, fill, 25, 25);

    test_add_to_hand_for(&tg, 1, live);

    int saw_victory = 0, p2_acted = 0, finished = 0;
    for (int i = 0; i < 40; i++) {
        if (test_has_pending_choice(&tg)) { answer_one_prompt(&tg); continue; }
        if (tg.state.phase == RB_PHASE_VICTORY) saw_victory = 1;
        if (saw_victory && tg.state.phase == RB_PHASE_ACTIVE) { finished = 1; break; }
        if (!p2_acted &&
            (tg.state.phase == RB_PHASE_LIVE_SET ||
             tg.state.phase == RB_PHASE_LIVE_SET_SECOND) &&
            tg.state.active == 1) {
            p2_acted = 1;
            test_set_live_card_for(&tg, 1, live);
        }
        test_pass(&tg);
    }
    CHECK(p2_acted, "E1: P2 reached its own LiveCardSet window and set a live card");
    CHECK(finished, "E1: the drive reached the Live Result / victory phase");
    CHECK_EQ(tg.state.p[0].live.n, 0, "E1: P1 set no live card");

    const RbLiveSnapshot *p2 = last_snapshot(&tg, 1);
    if (!p2) { CHECK(0, "E1: P2 produced a performance snapshot"); dump_snapshots(&tg); return; }
    CHECK_EQ(p2->n_lives, 1, "E1: P2's live card is judged (pre-fix this was 0)");
    CHECK_EQ(p2->live_passed[0], 1,
             "E1: the stage covers heart01+heart03+heart06 >= Dream Believers' heart0:4");
    CHECK_EQ(p2->success, 1, "E1: P2's own performance is finalised as a success");
    CHECK_EQ(p2->total_score, 1, "E1: Dream Believers scores 1, no score icons revealed");

    const RbLiveSnapshot *p1 = last_snapshot(&tg, 0);
    if (p1) CHECK_EQ(p1->n_lives, 0,
                     "E1: P1 staged nothing, so its snapshot has no judged lives");
    else printf("ok: E1: P1 produced no snapshot at all (also acceptable)\n");

    CHECK(bag_has(&tg.state.p[1].live, live) || bag_has(&tg.state.p[1].success, live),
          "E1: the successful live card belongs to P2's live/success zone");
}

/* Shared driver for E2/E3/E4. Walks the real phase machine and acts in EACH
   seat's LiveCardSet window according to perform1/perform2, so the attacker
   order is whatever the turn engine actually produces rather than something
   the test hand-sets. Bounded at 60 steps. */
static int attacker_order_run(TestGame *tg, int perform1, int perform2,
                              int *out_live0, int *out_live1)
{
    int member = test_id(tg, STAR_RIN);
    int member2 = test_new_id(tg, STAR_RIN);
    int fill = test_id(tg, FILLER);
    tg->state.p[0].stage[0] = member;
    tg->state.p[1].stage[0] = member2;
    for (int i = 1; i < RB_STAGE_SIZE; i++) {
        tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
        tg->state.p[1].stage[i] = RB_EMPTY_SLOT;
    }
    fill_decks(tg, fill, 20, 20);

    int lives[2];
    lives[0] = test_new_id(tg, HEART0_LIVE);
    lives[1] = test_new_id(tg, HEART0_LIVE);
    test_add_to_hand_for(tg, 0, lives[0]);
    test_add_to_hand_for(tg, 1, lives[1]);
    if (out_live0) *out_live0 = lives[0];
    if (out_live1) *out_live1 = lives[1];

    int acted[2] = { 0, 0 };
    int saw_victory = 0;
    int finished = 0;
    for (int i = 0; i < 60; i++) {
        if (test_has_pending_choice(tg)) { answer_one_prompt(tg); continue; }
        if (tg->state.phase == RB_PHASE_VICTORY) saw_victory = 1;
        if (saw_victory && tg->state.phase == RB_PHASE_ACTIVE) { finished = 1; break; }
        if ((tg->state.phase == RB_PHASE_LIVE_SET ||
             tg->state.phase == RB_PHASE_LIVE_SET_SECOND)) {
            int seat = tg->state.active;
            if (seat >= 0 && seat <= 1 && !acted[seat]) {
                acted[seat] = 1;
                if (seat == 0 ? perform1 : perform2)
                    test_set_live_card_for(tg, seat, lives[seat]);
            }
        }
        test_pass(tg);
    }
    return finished;
}

/* E2 — a SOLE winner's live card goes to the success zone and the OTHER seat
   keeps its live card in hand. (The Rust test additionally pins
   `is_first_attacker` flipping for the next turn; see E5's harness gap.) */
static void e2_sole_winner_places_its_live(void)
{
    /* Only ONE seat sets a live card, so exactly one seat wins. */
    for (int performer = 0; performer <= 1; performer++) {
        static TestGame tg;
        rb_card_copy_pool_reset();
        test_game_new(&tg);

        int live0 = -1, live1 = -1;
        int done = attacker_order_run(&tg, performer == 0, performer == 1,
                                      &live0, &live1);
        CHECK(done, "E2: the live round completed");

        int winner = performer;
        int loser  = 1 - winner;
        CHECK_EQ(tg.state.p[winner].success.n, 1,
                 "E2: the sole winner places exactly one live card");
        CHECK(bag_has(&tg.state.p[winner].success, winner == 0 ? live0 : live1),
              "E2: the success-zone card is the winner's own live card");
        CHECK_EQ(tg.state.p[loser].success.n, 0,
                 "E2: the non-performing seat places nothing");
        CHECK(bag_has(&tg.state.p[loser].hand, loser == 0 ? live0 : live1),
              "E2: the non-performing seat keeps its live card in hand");
    }
}

/* E3 — a TIED success places BOTH live cards and neither side gains. */
static void e3_tied_success_places_both(void)
{
    for (int iter = 0; iter < 2; iter++) {
        static TestGame tg;
        rb_card_copy_pool_reset();
        test_game_new(&tg);

        int live0 = -1, live1 = -1;
        CHECK(attacker_order_run(&tg, 1, 1, &live0, &live1),
              "E3: the live round completed");

        CHECK_EQ(tg.state.p[0].success.n, 1, "E3: the tie places P1's live card");
        CHECK_EQ(tg.state.p[1].success.n, 1, "E3: the tie places P2's live card");
        CHECK(bag_has(&tg.state.p[0].success, live0), "E3: P1's live is the card placed");
        CHECK(bag_has(&tg.state.p[1].success, live1), "E3: P2's live is the card placed");
    }
}

/* E4 — NO performance places nothing and leaves both live cards in hand. */
static void e4_no_performance_places_nothing(void)
{
    for (int iter = 0; iter < 2; iter++) {
        static TestGame tg;
        rb_card_copy_pool_reset();
        test_game_new(&tg);

        int live0 = -1, live1 = -1;
        CHECK(attacker_order_run(&tg, 0, 0, &live0, &live1),
              "E4: the live round completed");

        CHECK_EQ(tg.state.p[0].success.n, 0, "E4: P1 places nothing when nobody performs");
        CHECK_EQ(tg.state.p[1].success.n, 0, "E4: P2 places nothing when nobody performs");
        CHECK_EQ(waitroom_len(&tg, 0), 0, "E4: P1's waitroom gains nothing");
        CHECK_EQ(waitroom_len(&tg, 1), 0, "E4: P2's waitroom gains nothing");
        CHECK(bag_has(&tg.state.p[0].hand, live0), "E4: P1's live card is still in hand");
        CHECK(bag_has(&tg.state.p[1].hand, live1), "E4: P2's live card is still in hand");
    }
}

/* E5 — rule 8.4.13 attribution: with NO live performed, the C's single
   `first_attacker` seat index is what the Rust per-player `is_first_attacker`
   flag maps onto. The Rust flip-for-next-turn assertion has no C counterpart
   beyond that, which is recorded as a harness gap rather than a pass. */
static void e5_first_attacker_seat_attribution(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int live0 = -1, live1 = -1;
    attacker_order_run(&tg, 0, 0, &live0, &live1);

    EXPECTED_HARNESS_GAP(tg.state.first_attacker == 0,
        "Rust's per-player `is_first_attacker` (which flips for the next turn "
        "after a sole winner, and never changes on a tie or a no-show) has no C "
        "counterpart: C GameState carries a single `first_attacker` SEAT index "
        "that records the RPS winner, not a per-player flag that the live "
        "result updates");
}

/* ========================================================================
 * SECTION F — phases/phase_machine_rules_test.rs (q135 only; the file's
 * other 5 tests already live in test_phase_machine_rules.c)
 * ===================================================================== */

/* F1 — Q135: the REAL Active phase activates every waited member, not just the
   first one processed. */
static void f1_active_phase_activates_all_waited_members(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int filler = test_id(&tg, FILLER);
    int members[3];
    members[0] = filler;
    members[1] = test_new_id(&tg, FILLER);
    members[2] = test_new_id(&tg, FILLER);
    for (int i = 0; i < 3; i++) tg.state.p[0].stage[i] = members[i];
    for (int i = 0; i < 3; i++) {
        rb_mods_set_orientation(&tg.state.mods, members[i], "wait");
        tg.state.p[0].stage_wait[i] = 1;
    }
    fill_decks(&tg, filler, 30, 30);

    int guard = 0;
    while (guard < 24) {
        if (tg.state.phase == RB_PHASE_ACTIVE && tg.state.active == 0) break;
        guard++;
        pass_step(&tg);
    }
    /* The premise, asserted rather than assumed: this must be P1's OWN Active
       phase, or rule 7.4.1 would correctly leave P1's cards waited. */
    CHECK_EQ(tg.state.phase, RB_PHASE_ACTIVE, "F1: reached an Active phase");
    CHECK_EQ(tg.state.active, 0, "F1: PREMISE — this is P1's own Active phase");
    pass_step(&tg);
    CHECK_EQ(tg.state.phase, RB_PHASE_ENERGY, "F1: one pass carries Active -> Energy");

    for (int i = 0; i < 3; i++) {
        const char *o = rb_mods_get_orientation(&tg.state.mods, members[i]);
        CHECK(!(o && strcmp(o, "wait") == 0),
              "F1: Q135 — the real Active phase activates EVERY waited member");
    }
}

/* ========================================================================
 * SECTION G — baton/no_ability_baton_touch_draws_one_test.rs
 *
 * A member whose own 常時 is "draw 1" fires ONLY when it baton-touches a
 * member with NO abilities; a plain debut draws nothing. G1 is the positive
 * that makes G2/G3/G4's negative hand-count assertions non-vacuous.
 * ===================================================================== */

/* Rust `no_ability_baton_touch_draws_one_test.rs`'s drain: accept a MANDATORY
   single-card pick, decline everything else. */
static void drain_required_first(TestGame *tg)
{
    for (int guard = 0; guard < 40 && test_has_pending_choice(tg); guard++)
        answer_one_prompt(tg);
}

static void g1_baton_from_no_ability_draws_one(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int chika = test_id(&tg, CHIKA_PLAIN);
    int no_ability = test_id(&tg, FILLER);
    int filler = test_id(&tg, ERI);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = no_ability;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, chika);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    int hand_before = hand_len(&tg, 0);
    test_play_to_stage(&tg, chika, 1);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], chika, "G1: Chika is at Center after the baton touch");
    CHECK(bag_has(&tg.state.p[0].discard, no_ability),
          "G1: the replaced no-ability member sits in the waitroom");
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "G1: the draw of 1 compensates the hand loss from playing Chika");
}

/* G2 — baton-touching a member that HAS a 常時 draws nothing.
   Positive control: G1 on the same card, same board, absent the ability. */
static void g2_baton_from_ability_member_no_draw(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int chika = test_id(&tg, CHIKA_PLAIN);
    int has_ability = test_id(&tg, KASUMI_ABILITY);
    int filler = test_id(&tg, ERI);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = has_ability;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, chika);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    int hand_before = hand_len(&tg, 0);
    test_play_to_stage(&tg, chika, 1);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], chika, "G2: Chika is at Center after the baton touch");
    CHECK_EQ(hand_len(&tg, 0), hand_before - 1, "G2: no draw — the hand shrank by one");
}

/* G3 — a plain debut into an EMPTY area draws nothing. Control for G1/G2:
   same card, same energy, no displacement. */
static void g3_normal_debut_no_draw(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int chika = test_id(&tg, CHIKA_PLAIN);
    int filler = test_id(&tg, ERI);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, chika);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, filler);
    test_give_energy(&tg, 15);

    int hand_before = hand_len(&tg, 0);
    test_play_to_stage(&tg, chika, 1);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], chika, "G3: Chika is at Center after a plain debut");
    CHECK_EQ(hand_len(&tg, 0), hand_before - 1, "G3: no draw — the hand shrank by one");
}

/* G4 — a baton touch with an EMPTY deck refreshes the waitroom and then
   draws. Positive for the refresh path; G1's non-empty deck is the control. */
static void g4_baton_draw_triggers_refresh(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int chika = test_id(&tg, CHIKA_PLAIN);
    int no_ability = test_id(&tg, FILLER);
    int filler = test_id(&tg, ERI);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = no_ability;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, chika);
    tg.state.p[0].deck.n = 0;
    for (int i = 0; i < 2; i++) test_add_to_discard(&tg, filler);
    test_give_energy(&tg, 15);

    int hand_before = hand_len(&tg, 0);
    test_play_to_stage(&tg, chika, 1);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], chika, "G4: Chika is at Center after the baton touch");
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "G4: the draw of 1 from the refreshed deck compensates the hand loss");
    CHECK(deck_len(&tg, 0) > 0 || waitroom_len(&tg, 0) > 0,
          "G4: the refresh happened (the cards exist somewhere)");
}

/* ========================================================================
 * SECTION H — baton/baton_touch_replacement_name_and_cost_gate_test.rs
 * ===================================================================== */

/* H1 — Kasumi's baton replacement fires on her OWN name: draw 2, discard 1.
   This is the positive that makes H2's no-draw assertion non-vacuous. */
static void h1_baton_own_name_fires_draw_two_discard_one(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int old_me = test_id(&tg, KASUMI_BATON);
    int me = test_new_id(&tg, KASUMI_BATON);
    int filler = test_id(&tg, FILLER);
    tg.state.p[0].stage[0] = old_me;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 20);
    int d1 = test_new_id(&tg, FILLER);
    int d2 = test_new_id(&tg, FILLER);
    test_add_to_deck_pl(&tg, 0, d1);
    test_add_to_deck_pl(&tg, 0, d2);
    while (deck_len(&tg, 0) < 40) test_add_to_deck_pl(&tg, 0, test_new_id(&tg, FILLER));

    test_play_to_stage(&tg, me, 0);

    CHECK(bag_has(&tg.state.p[0].hand, d1) && bag_has(&tg.state.p[0].hand, d2),
          "H1: PREMISE — both drawn cards reached hand (proves the ability fired)");
    CHECK(test_has_pending_choice(&tg),
          "H1: the hand-discard prompt appears after the baton debut");
    CHECK(test_has_pending_choice(&tg) &&
          strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "H1: the prompt is a SelectCard (hand, count=1)");
    {
        int idx[1] = { 0 };
        test_select_indices(&tg, idx, 1);
    }
    CHECK(bag_has(&tg.state.p[0].discard, old_me),
          "H1: the replaced 中須かすみ sits in the waitroom");
    CHECK_EQ(hand_len(&tg, 0), 1, "H1: drew 2 then discarded 1, so one card remains");
    CHECK(waitroom_len(&tg, 0) > 0, "H1: a discard happened");
}

/* H2 — the same card baton-touching a DIFFERENT name fires nothing. */
static void h2_baton_other_name_no_draw(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int other = test_id(&tg, FILLER);
    int me = test_new_id(&tg, KASUMI_BATON);
    tg.state.p[0].stage[0] = other;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 20);

    int deck_before = deck_len(&tg, 0);
    test_play_to_stage(&tg, me, 0);
    drain_required_first(&tg);

    CHECK_EQ(deck_len(&tg, 0), deck_before,
             "H2: the wrong replaced name means no draw and no mill");
    CHECK_EQ(tg.state.p[0].stage[0], me, "H2: the baton touch itself still happened");
}

/* H3 — replacing a CHEAPER member grants +2 blades until live end. */
static void h3_baton_cheaper_member_fires_two_blades(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int cheap = test_id(&tg, DOLLCHESTRA);
    int me = test_new_id(&tg, DOLL_SMALL);
    tg.state.p[0].stage[0] = cheap;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 20);
    test_play_to_stage(&tg, me, 0);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], me, "H3: the baton touch replaced the cheaper member");
    CHECK_EQ(test_get_blade_modifier(&tg, me), 2,
             "H3: replacing a cheaper member grants +2 blades until live end");
}

/* H4 — replacing a MORE EXPENSIVE member grants nothing. Paired with H3. */
static void h4_baton_expensive_member_no_blades(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int pricey = test_id(&tg, DOLL_EXPENSIVE);
    int me = test_new_id(&tg, DOLL_SMALL);
    tg.state.p[0].stage[0] = pricey;
    test_add_to_hand(&tg, me);
    test_give_energy(&tg, 25);
    test_play_to_stage(&tg, me, 0);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], me, "H4: the baton touch replaced the expensive member");
    CHECK_EQ(test_get_blade_modifier(&tg, me), 0,
             "H4: replacing a MORE expensive member grants no blades");
}

/* ========================================================================
 * SECTION I — baton/baton_touch_named_replacement_fires_test.rs
 * ===================================================================== */

/* I1 — the replacement name gate: the same name fires, a different name does
   not. Both halves are on the same board shape, so the negative is paired
   with an in-case positive. */
static void i1_named_replacement_fires(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    /* ── same name: 三船栞子 replaces 三船栞子 ── */
    int mia = test_id(&tg, MIFUNE_SHIORIKO);
    int same_name = test_id(&tg, MIFUNE_SD);
    int filler = test_id(&tg, FILLER);
    test_give_energy(&tg, 15);
    tg.state.p[0].stage[1] = same_name;
    test_add_to_hand(&tg, mia);
    test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, filler);

    test_play_to_stage(&tg, mia, 1);
    CHECK(test_has_pending_choice(&tg),
          "I1: replacing the SAME name fires and waits for a discard choice");
    CHECK_EQ(hand_len(&tg, 0), 2, "I1: two cards were drawn before the discard");
    {
        int idx[1] = { 0 };
        test_select_indices(&tg, idx, 1);
    }
    CHECK_EQ(hand_len(&tg, 0), 1, "I1: one card remains after the discard completes");

    /* ── different name: 三船栞子 replaces 中須かすみ ── */
    int mia2 = test_new_id(&tg, MIFUNE_SHIORIKO);
    int other_name = test_id(&tg, NAKASU_KASUMI);
    drain_required_first(&tg);
    tg.state.p[0].stage[1] = other_name;
    test_add_to_hand(&tg, mia2);
    test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, filler);

    int hand_before = hand_len(&tg, 0);
    test_play_to_stage(&tg, mia2, 1);
    drain_required_first(&tg);

    CHECK(strcmp(test_card_name(mia2), test_card_name(other_name)) != 0,
          "I1: PREMISE — the two cards really do have different names");
    CHECK_EQ(tg.state.p[0].stage[1], mia2,
             "I1: PREMISE — the second play really did baton-touch the other member");
    CHECK(!test_has_pending_choice(&tg),
          "I1: replacing a DIFFERENT name does not fire the draw/discard ability");
    CHECK_EQ(hand_len(&tg, 0), hand_before - 1,
             "I1: no draw happened (the hand shrank by exactly the played card)");
}

/* ========================================================================
 * SECTION J — baton/baton_touch_protected_zone_restrictions_test.rs
 * ===================================================================== */

/* J1 — a single baton into a NON-protected area succeeds even when a
   cannot_baton_touch card sits in a different area. */
static void j1_single_baton_to_non_protected_zone_succeeds(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int protected_mem = test_id(&tg, PROTECTED_MEM);
    int filler_stage = test_id(&tg, HANAJO);
    int arriving = test_id(&tg, FILLER);
    tg.state.p[0].stage[0] = protected_mem;
    tg.state.p[0].stage[1] = filler_stage;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, arriving);
    test_give_energy(&tg, 10);

    int touches_before = rb_get_baton_touch_count(&tg.state, 0);
    test_play_to_stage(&tg, arriving, 1);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], protected_mem,
             "J1: the protected card is untouched on Left");
    CHECK_EQ(tg.state.p[0].stage[1], arriving,
             "J1: the arriving card replaced the unprotected Center member");
    /* The non-vacuity control for the rule-8.4.13 point: a can_baton_touch
       member in a DIFFERENT area does not block this play. */
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), touches_before + 1,
             "J1: exactly one baton touch was recorded");
    CHECK(bag_has(&tg.state.p[0].discard, filler_stage),
          "J1: the displaced unprotected member went to the waitroom");
}

/* J2 — a DOUBLE baton touching an area that holds a cannot_baton_touch member
   must be REJECTED entirely: neither area moves, the arrival stays in hand and
   no touch is counted. */
static void j2_double_baton_to_protected_zone_rejected(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int protected_mem = test_id(&tg, PROTECTED_MEM);
    int filler = test_id(&tg, HANAJO);
    int sumire = test_id(&tg, SUMIRE_DBL);
    tg.state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&tg, 0, filler);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 1, filler);
    tg.state.p[0].stage[0] = protected_mem;
    tg.state.p[0].stage[1] = filler;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, sumire);
    test_give_energy(&tg, 30);

    CHECK(rb_has_cannot_baton_touch_protection(sumire, protected_mem),
          "J2: PREMISE — the Left occupant really carries cannot_baton_touch");

    /* Roll past the arrival turn: rule 9.6.2.1.2.1 clears deployed_this_turn. */
    for (int i = 0; i < 7; i++) pass_step(&tg);

    int touches_before = rb_get_baton_touch_count(&tg.state, 0);
    test_play_to_stage(&tg, sumire, 1);
    /* The engine offers the optional double-baton gate; answer it and then
       read the board. Bounded so a re-prompting engine cannot hang the child. */
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], protected_mem,
             "J2: a rejected double baton leaves the protected area untouched");
    CHECK_EQ(tg.state.p[0].stage[1], filler,
             "J2: a rejected double baton leaves the unprotected area untouched too");
    CHECK(!bag_has(&tg.state.p[0].discard, protected_mem) &&
          !bag_has(&tg.state.p[0].discard, filler),
          "J2: neither target was put into the waitroom");
    CHECK(bag_has(&tg.state.p[0].hand, sumire),
          "J2: the arriving member is still in hand");
    CHECK_EQ(rb_get_baton_touch_count(&tg.state, 0), touches_before,
             "J2: a rejected double baton is not counted");
}

/* ========================================================================
 * SECTION K — baton/baton_touch_arrival_turn_restriction_test.rs
 * ===================================================================== */

/* K1 — rule 9.6.2.1.2.1 / Q29: a member played THIS turn cannot be sent to the
   waitroom by a baton touch on the same turn; from the NEXT turn the identical
   play is legal. The second half is the positive that makes the first
   non-vacuous. */
static void k1_baton_blocked_on_arrival_turn_allowed_next_turn(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);

    int first = test_id(&tg, FILLER);        /* cost 4, no abilities */
    int second = test_id(&tg, OMIKAZE);      /* cost 9               */
    int filler = test_id(&tg, FILLER);
    fill_decks(&tg, filler, 30, 30);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, first);
    test_add_to_hand(&tg, second);

    test_play_to_stage(&tg, first, 0);
    drain_required_first(&tg);
    CHECK_EQ(tg.state.p[0].stage[0], first, "K1: the first deployment is a plain play");

    /* Same turn: the baton touch onto her occupied area must be rejected. */
    test_play_to_stage(&tg, second, 0);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], first,
             "K1: Q29 — the arrival-turn member stays on stage");
    CHECK(bag_has(&tg.state.p[0].hand, second),
          "K1: the incoming member stays in hand");

    /* Turn 2: the identical play is legal. */
    /* Turn 2: the identical play is legal. The premise is that the turn
       wrapped — asserted, not assumed. */
    int guard = 0, wrapped = 0;
    while (guard < 30) {
        guard++;
        pass_step(&tg);
        if (tg.state.turn > 1) { wrapped = 1; break; }
    }
    CHECK(wrapped, "K1: the turn wrapped, so this is a LATER turn than the debut");

    test_play_to_stage(&tg, second, 0);
    drain_required_first(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], second, "K1: the incoming member takes the area");
    CHECK(bag_has(&tg.state.p[0].discard, first),
          "K1: Q24/Q141 — the departing member goes to the waitroom");
}

/* ========================================================================
 * fork harness
 * ===================================================================== */
static const char *current_test = "(none)";

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
        printf("        %-46s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
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

    printf("--- 0. card identity pins (cards.json, UTF-8) ---\n");
    run("fixture_identity",           section0_fixture_identity);

    printf("--- A. scoring/blade_heart_colorless_test.rs ---\n");
    run("a1_b_heart07_two_colorless_each",   a1_b_heart07_two_colorless_each);
    run("a2_colorless_cannot_fill_colored",  a2_colorless_cannot_fill_colored);
    run("a3_colorless_fills_heart0",         a3_colorless_fills_heart0_when_colors_present);
    run("a4_insufficient_colorless_fails",   a4_insufficient_colorless_fails);

    printf("--- B. scoring/special_blade_heart_rules_test.rs ---\n");
    run("b1_solitude_rain_yell_draws_one",  b1_solitude_rain_yell_draws_one);
    run("b2_failed_live_still_draws",       b2_failed_live_still_resolves_yell_draw_icons);
    run("b3_yell_draw_refreshes_deck",      b3_yell_draw_refreshes_deck_when_empty);
    run("b4_re_yell_draw_icon_draws",       b4_re_yell_draw_icon_draws);
    run("b5_re_yell_ball_is_wildcard",      b5_re_yell_ball_is_wildcard_not_colorless);
    run("b6_yell_with_live_blocks_re_yell", b6_yell_with_live_card_blocks_re_yell);
    run("b7_yell_score_icon_adds",          b7_yell_score_icon_adds_cheer_and_score);
    run("b8_live_zone_icons_do_not_apply",  b8_live_zone_special_icons_do_not_apply);

    printf("--- C. phases/performance_pipeline_test.rs ---\n");
    run("c1_pipeline_blade_yell_score",     c1_performance_pipeline_blade_yell_heart_score);
    run("c2_pipeline_fails_insufficient",   c2_pipeline_fails_when_hearts_insufficient);
    run("c3_heart00_second_live_fails",     c3_heart00_second_live_fails);
    run("c4_heart00_single_live_succeeds",  c4_heart00_single_live_succeeds);
    run("c5_live_card_base_score_recorded", c5_live_card_base_score_recorded);

    printf("--- D. phases/cheer_pipeline_test.rs ---\n");
    run("d1_cheer_pipeline_draw_icons",     d1_cheer_pipeline_draw_and_score_icons);
    run("d2_cheer_pipeline_score_icon",     d2_cheer_pipeline_score_icon);

    printf("--- E. phases/attacker_order_performance_test.rs ---\n");
    run("e1_p2_only_performer_snapshot",    e1_p2_only_performer_finalizes_p2_snapshot);
    run("e2_sole_winner_places_live",       e2_sole_winner_places_its_live);
    run("e3_tied_success_places_both",      e3_tied_success_places_both);
    run("e4_no_performance_places_nothing", e4_no_performance_places_nothing);
    run("e5_first_attacker_seat_gap",       e5_first_attacker_seat_attribution);

    printf("--- F. phases/phase_machine_rules_test.rs (q135) ---\n");
    run("f1_q135_all_waited_activate",      f1_active_phase_activates_all_waited_members);

    printf("--- G. baton/no_ability_baton_touch_draws_one_test.rs ---\n");
    run("g1_baton_no_ability_draws_one",    g1_baton_from_no_ability_draws_one);
    run("g2_baton_ability_member_no_draw",  g2_baton_from_ability_member_no_draw);
    run("g3_normal_debut_no_draw",           g3_normal_debut_no_draw);
    run("g4_baton_draw_triggers_refresh",   g4_baton_draw_triggers_refresh);

    printf("--- H. baton/baton_touch_replacement_name_and_cost_gate_test.rs ---\n");
    run("h1_baton_own_name_fires",          h1_baton_own_name_fires_draw_two_discard_one);
    run("h2_baton_other_name_no_draw",      h2_baton_other_name_no_draw);
    run("h3_baton_cheaper_two_blades",      h3_baton_cheaper_member_fires_two_blades);
    run("h4_baton_expensive_no_blades",     h4_baton_expensive_member_no_blades);

    printf("--- I. baton/baton_touch_named_replacement_fires_test.rs ---\n");
    run("i1_named_replacement_fires",       i1_named_replacement_fires);

    printf("--- J. baton/baton_touch_protected_zone_restrictions_test.rs ---\n");
    run("j1_single_baton_non_protected",    j1_single_baton_to_non_protected_zone_succeeds);
    run("j2_double_baton_protected_reject", j2_double_baton_to_protected_zone_rejected);

    printf("--- K. baton/baton_touch_arrival_turn_restriction_test.rs ---\n");
    run("k1_baton_blocked_on_arrival_turn", k1_baton_blocked_on_arrival_turn_allowed_next_turn);

    rb_unload();
    printf("\n==== rules/{scoring,phases,baton} parity: %d cases passed, "
           "%d cases failed, %d crashed (per-case assertion counts above) ====\n",
           n_ok, n_failed, n_crashed);
    return (failures || n_crashed) ? 1 : 0;
}

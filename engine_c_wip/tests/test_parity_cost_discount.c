/* test_parity_cost_discount.c
 *
 * C parity suite for the two remaining cost_mod clusters:
 *   engine/tests/test_modules/effects/cost_mod/activation_discount/  (10 .rs, 48 #[test])
 *   engine/tests/test_modules/effects/cost_mod/per_card/            ( 6 .rs, 38 #[test])
 * 86 #[test] functions in total. Every Rust test function name is reproduced
 * verbatim in the run(...) list in main() so a missing port is greppable.
 *
 * This file is PURE ASCII. Every Japanese literal is a \xNN escape, and every
 * card number carrying FULLWIDTH PLUS (U+FF0B, bytes ef bc 8b) is written
 * "....-R\xEF\xBC\x8B", never a raw byte and never an ASCII '+':
 * rb_find_card_by_no() is EXACT-BYTE and does NOT normalise fullwidth to
 * halfwidth the way Rust's CardDatabase::get_card_id does.
 *
 * ---------------------------------------------------------------------------
 * WHAT test_parity_cost_compound.c ALREADY COVERS (checked before writing
 * anything here, so nothing below duplicates it)
 * ---------------------------------------------------------------------------
 * Its header claims "effects/cost_mod (activation_discount, per_card)" but the
 * only overlap is inside two functions:
 *   test_modify_cost_operation_split (section J) covers
 *     - LL-bp2-001-R+ hand-count subtract via rb_compute_play_cost (the
 *       hand_card_count file's PLAY path, not its mods.constant_cost_bonuses
 *       read, and with the +1 hand_count convention, so its "3 in hand -> base-3"
 *       differs from the Rust constant-bonus expectation of base-2);
 *     - the 「on stage, hand-conditioned aura discounts nothing」 rule;
 *     - PL!S-bp3-016-N success-zone +1/card via rb_compute_play_cost, i.e. the
 *       single test of success_zone_stage_cost_deploy_energy_test.rs;
 *     - the set-override replacing the computed cost.
 *   test_constant_cost_bonus_is_additive (section K) covers 唐可可
 *   PL!SP-bp7-002-R contributing +2 with 8 energy vs 0 -- the ONE positive of
 *   the sixteen seven_energy_lead tests. None of the fifteen gates, the
 *   mirror cases, the recalc idempotency or any baton-pass arithmetic appears.
 * Everything in sections A-N below is therefore new.
 *
 * ---------------------------------------------------------------------------
 * NON-VACUITY DISCIPLINE
 * ---------------------------------------------------------------------------
 * Cost-mod clusters are dominated by NEGATIVE assertions: "no discount", "no
 * prompt", "the card is still in hand". In C each of those reads green just as
 * happily on a board where the effect never executed at all. Every negative
 * therefore carries a control, and the control is reported on stdout so a
 * reader can see WHICH control ran:
 *
 *   cd_wired(cid)  PARSER control: the card really decodes a 常時/constant
 *                  ability whose effect action is "modify_cost" (plus the
 *                  operation and value the test depends on). Proves the card
 *                  under test is the card the test believes it is.
 *   cd_note_yd(s)  PROMPT/VERDICT control: counts modify_yell_count EFFECT
 *                  verdicts the executor logged since the resolution started.
 *                  Only used where the effect runs through the ability log
 *                  (the ライブ開始時 yell reduction, section B).
 *   cd_paired(...) A PAIRED POSITIVE inside the same case where one is
 *                  possible. The plain energy-count gates (sections C, M, N)
 *                  have no logged verdict: the 常時 scan in
 *                  rb_recalc_constants applies the modifier without logging, so
 *                  the control is the paired positive on the SAME card in the
 *                  SAME file plus cd_wired(). Where a negative has no paired
 *                  positive, that is said out loud in a comment.
 *   cd_ptc()       The play-time alternative-cost control. The C engine does
 *                  not raise a SelectTarget for a play-time cost alternative;
 *                  it PAUSES the play and sets g->ptc_active. Every "the
 *                  choice is offered" assertion is ptc_active, and every "no
 *                  choice is offered" assertion is !ptc_active. Getting this
 *                  backwards is the biggest vacuity risk in this file, so the
 *                  two arms of each test assert OPPOSITE ptc states and the
 *                  energies differ, so neither can pass vacuously.
 *
 * ---------------------------------------------------------------------------
 * HARNESS LANDMINES HONOURED
 * ---------------------------------------------------------------------------
 *   - mid() == test_new_id() everywhere (Rust game.id() allocates the NEXT
 *     DISTINCT pool slot; C's template lookup aliases). test_id is used only
 *     where Rust itself deliberately shared one id.
 *   - rb_card_copy_pool_reset() per case. rb_create_card_copy draws from a
 *     process-global monotonic pool nothing else resets; without it a long
 *     suite exhausts the 4096 ids, mid() returns -1, and every identity
 *     assertion silently degrades to the shared template.
 *   - ident() is a BYTE-EXACT strcmp of rb_card_string(card_no_idx).
 *     rb_card_no_eq() is FUZZY and a peer showed it accepts PL!SP-PR-004-R
 *     against the real print PL!SP-PR-004-PR.
 *   - Every TestGame local is `static` (sizeof(GameState) is ~781 KB).
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE, so this
 *     file never calls it (no heart assertions at all in this cluster).
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *     from rb_card_num_abilities + rb_decode_card_ability.
 *   - Declining a SelectAutoAbility is NOT declining an optional cost: the C
 *     queue opens 起動 with a "shall this ability run?" step BEFORE the
 *     ability's own cost. cd_proceed_auto() answers THAT step, and only then
 *     does the cost prompt exist. No pay_optional_cost:skip gate is answered
 *     anywhere in this file (no card here has one on the paths exercised).
 *
 * ---------------------------------------------------------------------------
 * CONFIRMED RUST FIXTURE BUGS (evidence in cards/cards.json)
 * ---------------------------------------------------------------------------
 * 1. NO Rust fixture bug was found in this cluster. Every card number the 16
 *    .rs files name exists in cards/cards.json under the EXACT spelling the
 *    Rust source uses (the fullwidth-plus prints are written \u{ff0b} in Rust
 *    and are carried here as the \xEF\xBC\x8B escape), and every printed stat
 *    the tests reason about was re-read from cards.json and matches:
 *      PL!SP-pb1-010-R  cost 4   (the "spent 4 energy (base cost)" comment is
 *                                  CORRECT; an earlier revision of this file
 *                                  queried PL!SP-**bp1**-010-R instead and
 *                                  wrongly reported a contradiction)
 *      PL!SP-bp2-010-R＋ cost 15  (the -8 yell card, ab#1)
 *      PL!SP-bp7-002-R  cost 2   唐 可可; the "9 - 4 = 5" baton arithmetic
 *                                  depends on 2 and on PL!-sd1-006-SD = 9
 *      PL!-bp4-008-R    cost 4   小泉花陽; PL!-sd1-021-SD score 3;
 *                                  PL!-sd1-001-SD cost 11 (the "11 - 7 = 4" case)
 *      PL!S-bp3-016-N   cost 4   国木田花丸 (the +1 per success-zone card)
 *      PL!S-bp5-001-R＋  cost 10  高海千歌; PL!SP-PR-003-PR cost 2 (澁谷かのん,
 *                                  has a 登場 ability, so it is correctly the
 *                                  NO-reduction fixture); PL!-sd1-010-SD cost 4
 *      PL!-bp5-004-R＋  all four rarities (AR/P/R＋/SEC) exist; its 起動 cost
 *                                  is 4 energy, so the test's "EEEE(4)" is the
 *                                  ACTIVATION cost, not the printed 13;
 *                                  LL-bp1-001-R＋ carries three series, so
 *                                  umi + it really is 4 distinct groups
 *      LL-bp2-001-R＋   cost 20  (the "20 - 2 = 18" arithmetic depends on it)
 *      PL!HS-bp6-006-R＋/P cost 20 安養寺 姫芽; PL!HS-bp1-005-R cost 9 and
 *                                  PL!HS-PR-005-PR cost 10, both unit
 *                                  「みらくらぱーく!」 (ASCII !) while the
 *                                  ability spells the filter FULLWIDTH
 *                                  「みらくらぱーく！」 -- noted, not a bug
 *      LL-bp7-001-R＋   cost 15  and ab#0 names exactly 国木田花丸 /
 *                                  優木せつ菜 / 嵐千砂都
 *      PL!SP-pb1-038-SRE        really is the non-member 「澁谷かのん＆嵐 千砂都」
 *      PL!N-bp7-011-R＋  cost 13  ミア・テイラー
 *      PL!N-sd2-003-SD2 桜坂しずく; PL!N-sd1-025-SD is 虹ヶ咲 (score 1) and
 *                                  PL!-sd1-020-SD is ラブライブ！/mu's (score 2),
 *                                  so the group negative is a real group test
 *      PL!-bp6-019-L   Music S.T.A.R.T!!; PL!-PR-015-PR cost 17 mu's/BiBi;
 *                                  PL!-PR-001-PR cost 4 mu's; PL!S-bp2-008-P
 *                                  cost 17 Aqours/GuiltyKiss -- the
 *                                  "non-mu's" comment is CORRECT
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── counters ──────────────────────────────────────────────────────────── */

static int assertions;
static int failures;
static int n_setup_bugs;
static int n_vacuous;
static int n_tests_ok, n_tests_failed, n_tests_setup, n_tests_crashed;
static const char *current_test = "(none)";

#define CHILD_OK       0
#define CHILD_FAILURES 1
#define CHILD_SETUPBUG 3
#define CHILD_VACUOUS  4

/* Variadic so a red claim can print the numbers it is complaining about --
   the whole point of a red engine-bug report. */
#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "\n        [%s]\n", current_test); \
        failures++; \
    } \
} while (0)

#define CHECK_EQ(actual, expected, ...) do { \
    assertions++; \
    int cd_a = (int)(actual); \
    int cd_e = (int)(expected); \
    if (cd_a != cd_e) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  (got %d, expected %d)\n        [%s]\n", cd_a, cd_e, current_test); \
        failures++; \
    } \
} while (0)

/* A negative whose non-vacuity could not be established inside its own case.
   Recorded, never hidden: the assertion above it stays exactly as the Rust
   test wrote it, and the run summary counts these separately. */
#define VACUOUS(...) do { \
    assertions++; n_vacuous++; \
    fprintf(stderr, "VACUOUS: "); \
    fprintf(stderr, __VA_ARGS__); \
    fprintf(stderr, "\n        [%s]\n", current_test); \
} while (0)

/* ── card numbers ────────────────────────────────────────────────────────
   FP = FULLWIDTH PLUS U+FF0B (ef bc 8b). rb_find_card_by_no is exact-byte. */
/* group names, byte-exact with the literals in src/core/card.c's
   rb_card_series_to_group. 虹ヶ咲 = e8 99 b9 e3 83 b6 e5 92 b2; みらくらぱーく
   never appears here because no assertion needs it. */
#define GRP_NIJI  "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"  /* 虹ヶ咲 */
#define GRP_MUS   "\xCE\xBC's"                                /* mu's  */

#define FP "\xEF\xBC\x8B"

/* per_card/wien_cost_mod_test.rs */
#define WIEN_COST   "PL!SP-pb1-010-R"                /* printed cost 11, NOT 4 */
#define ENERGY_CARD "LL-E-001-SD"
/* per_card/wien_yell_count_test.rs */
#define WIEN_YELL   "PL!SP-bp2-010-R" FP
#define FILLER      "PL!-sd1-010-SD"                /* mu's member, cost 4 */
#define FILLER_LIVE "PL!-sd1-020-SD"                /* mu's live, score 2 */
#define FILLER_LIVE2 "PL!-sd1-019-SD"               /* mu's live, score 1 */
/* per_card/seven_energy_lead_stage_cost_two_bonus_test.rs */
#define TANG        "PL!SP-bp7-002-R"               /* base cost 2 */
#define COST9       "PL!-sd1-006-SD"                /* base cost 9 */
/* per_card/success_zone_score_six_stage_cost_three_bonus_test.rs */
#define HANAYO      "PL!-bp4-008-R"                 /* base cost 4 */
#define HANAYO_P    "PL!-bp4-008-P"
#define LIVE_SCORE3 "PL!-sd1-021-SD"                /* score 3 */
#define ARRIVER     "PL!-sd1-001-SD"                /* cost 11 */
/* per_card/success_zone_stage_cost_deploy_energy_test.rs */
#define HANAMARU_Q155 "PL!S-bp3-016-N"               /* base cost 4 */
/* activation_discount/abilityless_member_play_cost_reduction_test.rs */
#define CHIKA       "PL!S-bp5-001-R" FP             /* base cost 10 */
#define HAS_ABILITY "PL!SP-PR-003-PR"               /* 澁谷かのん, cost 2, has ab#0 */
/* activation_discount/distinct_group_activation_discount_q228_test.rs */
#define UMI_R       "PL!-bp5-004-R" FP
#define UMI_P       "PL!-bp5-004-P"
#define UMI_AR      "PL!-bp5-004-AR"
#define UMI_SEC     "PL!-bp5-004-SEC"
#define MULTI3      "LL-bp1-001-R" FP               /* 3 series -> 3 groups */
#define LIELLA      "PL!SP-bp1-007-R" FP            /* スーパースター!! */
#define SAYAKA      "PL!HS-pb1-002-R"               /* cost 2, 蓮ノ空 */
#define RUBY        "PL!S-bp5-009-R"                /* cost 15 (over the limit) */
/* activation_discount/hand_card_count_play_cost_reduction_test.rs */
#define KOTORI      "LL-bp2-001-R" FP               /* base cost 20 */
/* activation_discount/per_unit_member_cost_reduction_q249_test.rs */
#define HIMENO_R    "PL!HS-bp6-006-R" FP            /* base cost 20 */
#define HIMENO_P    "PL!HS-bp6-006-P"
#define MIRAKU_A    "PL!HS-bp1-005-R"               /* みらくらぱーく!, cost 9 */
#define MIRAKU_B    "PL!HS-PR-005-PR"               /* みらくらぱーく!, cost 10 */
/* activation_discount/play_time_three_named_member_discard_cost_ten_test.rs */
#define TRIPLE3     "LL-bp7-001-R" FP               /* base cost 15 -> 10 */
#define HANAMARU    "PL!S-bp2-016-N"
#define SETSUNA     "PL!N-PR-009-PR"
#define CHISATO     "PL!SP-pb1-014-PR"
#define CHISATO_ENERGY "PL!SP-pb1-038-SRE"
/* activation_discount/shuffle_waitroom_members_optional_play_discount_test.rs */
#define MIA         "PL!N-bp7-011-R" FP             /* base cost 13 */
/* activation_discount/success_zone_group_member_play_discount_test.rs */
#define SHIZUKU     "PL!N-sd2-003-SD2"
#define NIJI_LIVE   "PL!N-sd1-025-SD"               /* 虹ヶ咲 live, score 1 */
#define AQOURS_LIVE "PL!-sd1-020-SD"                /* mu's live -- NOT niji */
/* activation_discount/success_zone_high_cost_group_deploy_discount_test.rs */
#define MUSIC       "PL!-bp6-019-L"
#define MAKI        "PL!-PR-015-PR"                 /* mu's, cost 17 */
#define HONOKA      "PL!-PR-001-PR"                 /* mu's, cost 4 */
#define MARI        "PL!S-bp2-008-P"                /* Aqours, cost 17 */

/* ── harness ───────────────────────────────────────────────────────────── */

/* `src` is the in-tree layout; an isolated out-of-tree build finds the
   database only through the sibling path the Makefile writes. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static void on_segv(int sig)
{
    const char *msg = "\nCRASH: a fatal signal arrived inside the test below.\n";
    ssize_t ignored = write(2, msg, strlen(msg));
    (void)ignored;
    fprintf(stderr, "CRASH: signal %d in test [%s]\n", sig, current_test);
    fflush(stderr);
    _Exit(CHILD_SETUPBUG);
}

/* Every case starts from a clean id pool. */
static void case_begin(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

/* Rust TestGame::id() -> the NEXT DISTINCT pool slot. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* BYTE-EXACT identity pin. rb_card_no_eq() is fuzzy and is not an identity
   pin: a peer showed it accepting PL!SP-PR-004-R against the real print
   PL!SP-PR-004-PR. */
static int card_no_of(int cid, char *buf, size_t n)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { if (n) buf[0] = '\0'; return 0; }
    snprintf(buf, n, "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    return 1;
}

static int ident(int cid, const char *no)
{
    char buf[128];
    if (cid < 0 || !card_no_of(cid, buf, sizeof buf)) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: card id %d could not be decoded (expected %s)\n",
                cid, no);
        return 0;
    }
    if (strcmp(buf, no) != 0) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: card id %d is \"%s\", the test names \"%s\"\n",
                cid, buf, no);
        return 0;
    }
    return 1;
}

static int printed_cost(int cid)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = c.cost;
    rb_free_card(&c);
    return v;
}

static int printed_score(int cid)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = c.score;
    rb_free_card(&c);
    return v;
}

static int printed_blade(int cid)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = c.blade;
    rb_free_card(&c);
    return v;
}

/* mods.get_cost_modifier -- the additive `add` accumulator. */
static int cost_mod(TestGame *tg, int cid)
{
    return rb_mods_get_cost(&tg->state.mods, cid);
}

/* mods.get_cost_modifier_set -- the set-override slot. */
static int cost_set(TestGame *tg, int cid)
{
    return rb_mods_get_cost_set(&tg->state.mods, cid);
}

/* mods.constant_cost_bonuses[card] -- the map the hand-count tests read. */
static int constant_cost(TestGame *tg, int cid)
{
    if (cid < 0 || cid >= RB_MAX_CARD_IDS) return 0;
    return tg->state.mods.constant_cost[cid];
}

static void set_stage3(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static void set_stage1(TestGame *tg, int pl, int area, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[area] = cid;
    P->stage_wait[area] = 0;
}

static int active(TestGame *tg, int pl)
{
    return rb_energy_active_count(&tg->state.p[pl]);
}

static int hand_n(TestGame *tg, int pl)
{
    return tg->state.p[pl].hand.n;
}

static int hand_has(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) return 1;
    return 0;
}

static int waitroom_has(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) return 1;
    return 0;
}

static int waitroom_n(TestGame *tg, int pl)
{
    return tg->state.p[pl].discard.n;
}

static int stage_has(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == cid) return 1;
    return 0;
}

static int stage_empty(TestGame *tg, int pl)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) return 0;
    return 1;
}

static int success_n(TestGame *tg, int pl)
{
    return tg->state.p[pl].success.n;
}

/* Rust fill_decks / reset_both_main_decks_to_ten_fillers: CLEAR both decks,
   then push `n` fillers into each. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void bare_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
        P->live.n = 0; P->success.n = 0;
        P->energy.n = 0; P->energy_deck.n = 0; P->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
}

/* ── PARSER control ─────────────────────────────────────────────────────── */

/* Proves the card really decodes the 常時 cost-modifying ability the test is
 * about, and that its operation / value are the ones the arithmetic assumes.
 * Returns 1 when such an ability exists. `want_op` is "add" / "subtract" /
 * "set"; `want_val` is the expected `value` (pass 0 to skip the check). */
static int cd_wired_op(TestGame *tg, int cid, const char *want_op, int want_val)
{
    (void)tg;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = 0;
        if (ab.effect && ab.effect->action &&
            !strcmp(ab.effect->action, "modify_cost") &&
            rb_ability_has_trigger(&ab, RB_TK_CONSTANT)) {
            hit = 1;
            if (want_op) {
                const char *op = NULL;
                for (int k = 0; k < ab.effect->n_extra; k++)
                    if (ab.effect->extra_k[k] &&
                        !strcmp(ab.effect->extra_k[k], "operation"))
                        op = ab.effect->extra_v[k];
                if (!op || strcmp(op, want_op) != 0) hit = 0;
            }
            if (hit && want_val) {
                int v = 0;
                for (int k = 0; k < ab.effect->n_extra; k++)
                    if (ab.effect->extra_k[k] &&
                        !strcmp(ab.effect->extra_k[k], "value") &&
                        ab.effect->extra_v[k])
                        v = atoi(ab.effect->extra_v[k]);
                if (v == 0) v = ab.effect->count;
                if (v != want_val) hit = 0;
            }
        }
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    assertions++; n_setup_bugs++;
    fprintf(stderr, "SETUP BUG: card id=%d carries no 常時 modify_cost ability "
                    "with operation=%s value=%d\n", cid,
            want_op ? want_op : "(any)", want_val);
    return 0;
}

/* ── PROMPT / VERDICT control ───────────────────────────────────────────── */

/* Counts `action` EFFECT verdicts the executor logged since `since`
   (a rb_log_buffer_len() snapshot). > 0 proves the ability REACHED the
   executor, so a following zero is a real filter verdict and not a dead
   effect. */
static int cd_count_effects(int since, const char *action)
{
    RbAbilityLogItem items[256];
    int n = rb_log_drain_verdicts_since(since, items, 256);
    int c = 0;
    for (int i = 0; i < n; i++) {
        if (items[i].kind == RB_LOG_KIND_EFFECT) {
            const char *a = items[i].as.effect.action;
            if (a && !strcmp(a, action)) c++;
        }
        rb_log_free_item(&items[i]);
    }
    return c;
}

static void cd_note_effects(int since, const char *action, const char *what)
{
    int c = cd_count_effects(since, action);
    assertions++;
    if (c > 0)
        printf("ok: [%s] the ability DISPATCHED (%d %s verdict(s)); the zero "
               "below is a filter verdict, not a dead effect\n", what, c, action);
    else
        printf("ok: [%s] the ability did NOT dispatch (0 %s verdicts) -- its "
               "trigger/condition rejected this board. The non-vacuity of the "
               "zero below rests on the paired positive on this ability in "
               "this file, not on this count\n", what, action);
}

/* ── play-time alternative cost control ─────────────────────────────────── */

/* The C engine does NOT queue a SelectTarget for a play-time cost
   alternative: rb_play_member pauses the play and sets g->ptc_active, and
   rb_complete_play_with_cost finishes it. `ptc_active` is therefore the
   "the choice is offered" signal, and its ABSENCE is the "no choice" signal. */
static int cd_ptc(TestGame *tg) { return tg->state.ptc_active; }

/* Answer the paused play. `accept` selects the alternative cost (>=0) or the
   printed cost (-1). Mirrors Rust answer_play_choice(accept). */
static void cd_answer_ptc(TestGame *tg, int accept)
{
    test_answer_play_cost_choice(tg, accept);
}

/* ── choice draining ────────────────────────────────────────────────────── */

#define CD_ACCEPT 0
#define CD_SKIP  (-1)

/* Proceed the "shall this ability run?" step the C queue opens BEFORE an
   ability's own cost. Answering THAT with the skip index refuses the WHOLE
   ability, so it must never be used as "decline the cost". Mirrors Rust's
   TestGame::drain_auto_ability_choices, which the Rust play_to_stage /
   activate_ability helpers already performed. */
static int cd_proceed_auto(TestGame *tg)
{
    int n = 0;
    while (test_has_pending_choice(tg) && n < 8) {
        if (tg->state.queue.resume_mode == 3 || tg->state.queue.auto_ability) {
            rb_resume_with_choice(&tg->state, CD_ACCEPT);
            n++;
            continue;
        }
        break;
    }
    return n;
}

/* Rust drain_choices(): answer what cannot be declined, decline what can. */
static void cd_drain(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        if (tg->state.queue.resume_mode == 3 || tg->state.queue.auto_ability)
            rb_resume_with_choice(&tg->state, CD_ACCEPT);
        else
            rb_resume_with_choice(&tg->state, CD_SKIP);
    }
}

/* Rust `while has_pending_choice() { select_indices(&[0]) }` */
static void cd_drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, CD_ACCEPT);
}

/* Rust `while has_pending_choice() { select_indices(&[]) }` */
static void cd_drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, CD_SKIP);
}

/* ── phase walk (section B only) ────────────────────────────────────────── */

/* Five blind passes from the initial Main phase reach the live-card set
   window. A pending prompt is answered here rather than being allowed to
   absorb a pass. */
static int cd_advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 8; i++) {
        if (tg->state.phase == RB_PHASE_LIVE_SET ||
            tg->state.phase == RB_PHASE_PERFORMANCE) return 1;
        if (test_has_pending_choice(tg)) {
            rb_resume_with_choice(&tg->state, CD_ACCEPT);
            continue;
        }
        test_pass(tg);
    }
    return tg->state.phase == RB_PHASE_LIVE_SET ||
           tg->state.phase == RB_PHASE_PERFORMANCE;
}

/* Pass into the performance window (where ライブ開始時 fires) and drain. */
static int cd_finish_live_setup(TestGame *tg)
{
    for (int i = 0; i < 4; i++) {
        if (tg->state.phase == RB_PHASE_PERFORMANCE) break;
        if (test_has_pending_choice(tg)) {
            rb_resume_with_choice(&tg->state, CD_ACCEPT);
            continue;
        }
        test_pass(tg);
    }
    cd_proceed_auto(tg);
    return tg->state.phase == RB_PHASE_PERFORMANCE;
}

static int stage_blade_total(TestGame *tg, int pl)
{
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = tg->state.p[pl].stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        int printed = printed_blade(cid);
        if (printed < 0) printed = 0;
        total += printed + rb_mods_get_blade(&tg->state.mods, cid);
    }
    if (total < 0) total = 0;
    return total;
}

/* ======================================================================
 * A. per_card/wien_cost_mod_test.rs (4)
 *    PL!SP-pb1-010-R ウィーン・マルガレーテ
 *    常時: 自分のエネルギーが10枚以上ある場合、ステージにいるこのメンバーの
 *    コストを＋４する。
 *
 *    FIXTURE BUG (see header): cards.json gives this print cost 11, the Rust
 *    comments say 4. The two "play for base cost" half-assertions are
 *    therefore unsatisfiable, so each of A1/A2 states the fixture guard
 *    explicitly and then asserts the SUBJECT of the test (the modifier as a
 *    function of own energy) on a staged Wien, which is the only part of the
 *    behaviour that is well-defined on the real print.
 * ====================================================================== */

static void a_wien_low_energy_no_cost_modifier(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_COST);
    if (!ident(wien, WIEN_COST)) return;
    CHECK_EQ(printed_cost(wien), 4,
             "fixture guard: PL!SP-pb1-010-R ウィーン・マルガレーテ costs 4 "
             "(cards.json: cost 4, blade 1, series ラブライブ！スーパースター!!) -- "
             "the Rust test's \"spent 4 energy (base cost)\" is correct");
    /* PARSER control: the ability the test is about really is on the card. */
    CHECK(cd_wired_op(&tg, wien, "add", 4),
          "PARSER CONTROL: Wien carries a 常時 modify_cost add +4");

    set_stage1(&tg, 0, 1, wien);
    test_give_energy(&tg, 9);
    test_recalc(&tg);
    CHECK_EQ(active(&tg, 0), 9, "setup: 9 active energy");
    CHECK_EQ(cost_mod(&tg, wien), 0,
             "no cost modifier when own energy < 10 (9 < 10)");
}

static void a_wien_high_energy_cost_modifier_applied(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_COST);
    if (!ident(wien, WIEN_COST)) return;
    CHECK_EQ(printed_cost(wien), 4,
             "fixture guard: her printed cost is 4, so \"played for base cost 4\" "
             "holds; the +4 is on-stage only and the play pays 4");
    CHECK(cd_wired_op(&tg, wien, "add", 4),
          "PARSER CONTROL: Wien carries a 常時 modify_cost add +4");

    set_stage1(&tg, 0, 1, wien);
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    CHECK_EQ(active(&tg, 0), 10, "setup: 10 active energy");
    CHECK_EQ(cost_mod(&tg, wien), 4,
             "+4 cost modifier at exactly 10 energy (>= 10 boundary)");
}

static void a_wien_cost_modifier_dynamic(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_COST);
    int energy_id = mid(&tg, ENERGY_CARD);
    if (!ident(wien, WIEN_COST) || !ident(energy_id, ENERGY_CARD)) return;
    CHECK(cd_wired_op(&tg, wien, "add", 4),
          "PARSER CONTROL: Wien carries a 常時 modify_cost add +4");

    set_stage1(&tg, 0, 1, wien);
    test_give_energy(&tg, 9);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, wien), 0, "no modifier at 9 energy");

    test_add_to_energy(&tg, 0, energy_id);
    test_set_energy_active(&tg, 0, 10);
    test_recalc(&tg);
    CHECK_EQ(active(&tg, 0), 10, "setup: energy raised to 10");
    CHECK_EQ(cost_mod(&tg, wien), 4, "+4 modifier at 10 energy");

    /* Remove one: pop the energy card and drop the active count. */
    tg.state.p[0].energy.n--;
    test_set_energy_active(&tg, 0, 9);
    test_recalc(&tg);
    CHECK_EQ(active(&tg, 0), 9, "setup: energy back to 9");
    CHECK_EQ(cost_mod(&tg, wien), 0,
             "modifier removed when energy drops back to 9");
}

static void a_wien_cost_modifier_cleared_on_leave(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_COST);
    if (!ident(wien, WIEN_COST)) return;
    CHECK(cd_wired_op(&tg, wien, "add", 4),
          "PARSER CONTROL: Wien carries a 常時 modify_cost add +4");

    set_stage1(&tg, 0, 1, wien);
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, wien), 4, "+4 modifier while on stage with 10 energy");

    /* Remove from stage, push to the waitroom, clear the card's modifiers. */
    set_stage1(&tg, 0, 1, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, wien);
    test_clear_mods_for_card(&tg, wien);
    test_recalc(&tg);
    CHECK(!stage_has(&tg, 0, wien), "setup: Wien is off the stage");
    CHECK_EQ(cost_mod(&tg, wien), 0, "modifier cleared after card leaves stage");
}

/* ======================================================================
 * B. per_card/wien_yell_count_test.rs (5)
 *    PL!SP-bp2-010-R＋ ab#1 ライブ開始時 modify_yell_count(subtract 8),
 *    gated on card_count_condition(>= 1, exclude_self, stage).
 *
 *    NON-VACUITY: the ライブ開始時 walk is the only thing in this cluster that
 *    runs through the ability log, so every case here brackets it with a
 *    cd_note_effects() count. B1 and B2 are the paired positive/negative of
 *    the same card on the same walk; B1 alone carrying a > 0 count is what
 *    makes B2's zero a real gate verdict.
 * ====================================================================== */

static void b_wien_with_partner_reduces_cheer_checks_by_8(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_YELL);
    int partner = mid(&tg, FILLER);
    int live = mid(&tg, FILLER_LIVE);
    int filler = mid(&tg, FILLER);
    if (!ident(wien, WIEN_YELL) || !ident(partner, FILLER) ||
        !ident(live, FILLER_LIVE)) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, partner);
    test_add_to_hand(&tg, live);
    fill_decks(&tg, filler, 10);

    int since = rb_log_buffer_len();
    CHECK(cd_advance_to_live_set(&tg), "walk reaches the live-card set window");
    test_set_live_card(&tg, 0, live);
    CHECK(cd_finish_live_setup(&tg), "walk reaches the performance window");
    cd_drain_skip(&tg);
    cd_note_effects(since, "modify_yell_count",
                    "B1 partner board -- PROMPT CONTROL");

    rb_perform_cheer_check(&tg.state, "p1", 10);
    CHECK_EQ(tg.state.cheer_checks_required, 2,
             "Wien + partner must reduce the required cheer checks by exactly 8");
}

static void b_wien_alone_no_reduction(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_YELL);
    int live = mid(&tg, FILLER_LIVE);
    int filler = mid(&tg, FILLER);
    if (!ident(wien, WIEN_YELL) || !ident(live, FILLER_LIVE)) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, live);
    fill_decks(&tg, filler, 10);

    int since = rb_log_buffer_len();
    CHECK(cd_advance_to_live_set(&tg), "walk reaches the live-card set window");
    test_set_live_card(&tg, 0, live);
    CHECK(cd_finish_live_setup(&tg), "walk reaches the performance window");
    cd_drain_skip(&tg);
    /* A count of 0 here is EXPECTED and is itself the verdict: the
       card_count_condition(>= 1 other member) rejected the board. That is
       precisely the behaviour under test, and the paired positive is B1. */
    cd_note_effects(since, "modify_yell_count",
                    "B2 alone -- PROMPT CONTROL (0 is the condition verdict)");

    rb_perform_cheer_check(&tg.state, "p1", 10);
    CHECK_EQ(tg.state.cheer_checks_required, 10,
             "Wien alone must not reduce the required cheer checks");
}

static void b_opponent_cheer_checks_unaffected(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_YELL);
    int partner = mid(&tg, FILLER);
    int live = mid(&tg, FILLER_LIVE);
    int filler = mid(&tg, FILLER);
    if (!ident(wien, WIEN_YELL) || !ident(live, FILLER_LIVE)) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, partner);
    test_add_to_hand(&tg, live);
    fill_decks(&tg, filler, 10);

    int since = rb_log_buffer_len();
    CHECK(cd_advance_to_live_set(&tg), "walk reaches the live-card set window");
    test_set_live_card(&tg, 0, live);
    CHECK(cd_finish_live_setup(&tg), "walk reaches the performance window");
    cd_drain_skip(&tg);
    cd_note_effects(since, "modify_yell_count",
                    "B3 opponent check -- PROMPT CONTROL (same board as B1, so "
                    "the -8 really was applied to P1)");

    rb_perform_cheer_check(&tg.state, "p2", 10);
    CHECK_EQ(tg.state.cheer_checks_required, 10,
             "the opponent's required cheer checks are untouched by Wien");
}

static void b_q117_second_wien_copy_counts_as_other_member(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien1 = mid(&tg, WIEN_YELL);
    int wien2 = mid(&tg, WIEN_YELL);
    int live = mid(&tg, FILLER_LIVE);
    int filler = mid(&tg, FILLER);
    if (!ident(wien1, WIEN_YELL) || !ident(wien2, WIEN_YELL)) return;
    CHECK(wien1 != wien2,
          "fixture guard: two distinct pool slots (mid() never aliases)");
    set_stage3(&tg, 0, RB_EMPTY_SLOT, wien1, wien2);
    test_add_to_hand(&tg, live);
    fill_decks(&tg, filler, 10);

    int since = rb_log_buffer_len();
    CHECK(cd_advance_to_live_set(&tg), "walk reaches the live-card set window");
    test_set_live_card(&tg, 0, live);
    CHECK(cd_finish_live_setup(&tg), "walk reaches the performance window");
    cd_drain_skip(&tg);
    cd_note_effects(since, "modify_yell_count",
                    "B4 two Wien copies -- PROMPT CONTROL (2 is the saturation "
                    "case: -16 on a base of 10)");

    rb_perform_cheer_check(&tg.state, "p1", 10);
    CHECK_EQ(tg.state.cheer_checks_required, 0,
             "Q117: a second copy of Wien satisfies 「このメンバー以外」, so both "
             "copies fire and -16 saturates to 0");
}

static void b_blade_gained_after_live_start_does_not_reduce_yell_checks(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int wien = mid(&tg, WIEN_YELL);
    int partner = mid(&tg, FILLER);
    int live = mid(&tg, FILLER_LIVE);
    int filler = mid(&tg, FILLER);
    if (!ident(wien, WIEN_YELL) || !ident(partner, FILLER)) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, partner);
    rb_mods_add_blade(&tg.state.mods, partner, -3);
    test_add_to_hand(&tg, live);
    fill_decks(&tg, filler, 10);

    int since = rb_log_buffer_len();
    CHECK(cd_advance_to_live_set(&tg), "walk reaches the live-card set window");
    test_set_live_card(&tg, 0, live);
    CHECK(cd_finish_live_setup(&tg), "walk reaches the performance window");
    cd_drain_skip(&tg);
    cd_note_effects(since, "modify_yell_count",
                    "B5 post-LiveStart blade -- PROMPT CONTROL");

    /* Q111: a blade gained after LiveStart must not change the -8, which was
       fixed when ライブ開始時 resolved. */
    rb_mods_add_blade(&tg.state.mods, partner, 2);
    int blade = stage_blade_total(&tg, 0);
    CHECK(blade >= 0, "setup: the stage blade total is computable");
    rb_perform_cheer_check(&tg.state, "p1", (uint8_t)blade);
    int expected = blade - 8;
    if (expected < 0) expected = 0;
    CHECK_EQ(tg.state.cheer_checks_required, expected,
             "Q111: Wien's -8 is fixed at LiveStart and applies to the LATER "
             "blade total");
}

/* ======================================================================
 * C. per_card/seven_energy_lead_stage_cost_two_bonus_test.rs (16)
 *    PL!SP-bp7-002-R 唐 可可 (base cost 2)
 *    常時: 自分のエネルギーが7枚以上あり、かつ自分のエネルギーが相手より多い
 *    かぎり、ステージにいるこのメンバーのコストを＋２する。
 *
 *    NON-VACUITY: the 常時 scan applies without logging a verdict, so each
 *    negative carries cd_wired_op() (PARSER) plus a PAIRED POSITIVE on the same
 *    card in the same section (C1/C2 for every >=7-or-more gate, C2 for the
 *    energy-lead gate). A negative whose paired positive could not be
 *    constructed says so in its own comment.
 *
 *    STRUCTURAL GAP: RbGeneratedArea carries only {available, is_baton_touch}.
 *    The Rust action-gate tests assert `center.cost` and `final_cost`, neither
 *    of which the C generator records, so C13/C14/C16 assert the availability
 *    flags and the PRICE through rb_compute_play_cost (the same function the
 *    generator itself calls) instead, and say so.
 * ====================================================================== */

static int c_place_tang(TestGame *tg, int cid, int p1_energy, int p2_energy)
{
    int e = mid(tg, ENERGY_CARD);
    set_stage3(tg, 0, RB_EMPTY_SLOT, cid, RB_EMPTY_SLOT);
    for (int i = 0; i < p1_energy; i++) test_add_to_energy(tg, 0, e);
    test_set_energy_active(tg, 0, p1_energy);
    for (int i = 0; i < p2_energy; i++) test_add_to_energy(tg, 1, e);
    test_set_energy_active(tg, 1, p2_energy);
    test_recalc(tg);
    return cid;
}

static int c_price(TestGame *tg, int cid)
{
    return rb_compute_play_cost(&tg->state, 0, cid, 0);
}

static void c_condition_met_applies_plus2_once(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    CHECK_EQ(printed_cost(t), 2, "fixture guard: 唐可可 base cost 2 (cards.json)");
    CHECK(cd_wired_op(&tg, t, "add", 2),
          "PARSER CONTROL: 唐可可 carries a 常時 modify_cost add +2");
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(active(&tg, 0), 7, "setup: 7 active energy for P1, 0 for P2");
    CHECK_EQ(cost_mod(&tg, t), 2,
             "7 >= 7 and 7 > 0 -> cost +2 exactly once (PAIRED POSITIVE for C3/C4/C5)");
}

static void c_more_energy_high_margin_applies_once(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    CHECK(cd_wired_op(&tg, t, "add", 2),
          "PARSER CONTROL: 唐可可 carries a 常時 modify_cost add +2");
    c_place_tang(&tg, t, 10, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "10 >= 7 and 10 > 0 -> +2 exactly once");
}

static void c_equal_energy_denies_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    CHECK(cd_wired_op(&tg, t, "add", 2),
          "PARSER CONTROL: 唐可可 carries a 常時 modify_cost add +2 (the same "
          "ability C1 shows landing +2 on a 7-vs-0 board)");
    c_place_tang(&tg, t, 7, 7);
    CHECK_EQ(active(&tg, 0), 7, "setup: P1 has 7");
    CHECK_EQ(active(&tg, 1), 7, "setup: P2 has 7");
    CHECK_EQ(cost_mod(&tg, t), 0,
             "7 == 7 is not MORE than the opponent -> no +2 even at the >= 7 gate");
}

static void c_less_energy_denies_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    CHECK(cd_wired_op(&tg, t, "add", 2),
          "PARSER CONTROL: 唐可可 carries a 常時 modify_cost add +2 (paired "
          "positive is C1 on the same ability)");
    c_place_tang(&tg, t, 7, 9);
    CHECK_EQ(cost_mod(&tg, t), 0, "7 < 9 is not more than the opponent -> no +2");
}

static void c_below_seven_denies_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    CHECK(cd_wired_op(&tg, t, "add", 2),
          "PARSER CONTROL: 唐可可 carries a 常時 modify_cost add +2 (paired "
          "positive is C1 on the same ability)");
    c_place_tang(&tg, t, 6, 0);
    CHECK_EQ(cost_mod(&tg, t), 0, "6 < 7 -> no +2");
}

static void c_mirror_only_more_energy_side_gets_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t1 = mid(&tg, TANG);
    int t2 = mid(&tg, TANG);
    int e = mid(&tg, ENERGY_CARD);
    if (!ident(t1, TANG) || !ident(t2, TANG)) return;
    CHECK(t1 != t2, "fixture guard: two distinct pool slots");
    set_stage1(&tg, 0, 1, t1);
    set_stage1(&tg, 1, 1, t2);
    for (int i = 0; i < 8; i++) test_add_to_energy(&tg, 0, e);
    test_set_energy_active(&tg, 0, 8);
    for (int i = 0; i < 5; i++) test_add_to_energy(&tg, 1, e);
    test_set_energy_active(&tg, 1, 5);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, t1), 2, "P1 (8) has more energy -> +2");
    CHECK_EQ(cost_mod(&tg, t2), 0, "P2 (5) has less energy -> no +2");
    CHECK_EQ(cost_mod(&tg, t1), 2,
             "mirror control: the +2 on P1's copy is P1's own evaluation, not a "
             "shared scan (C6's second assertion cannot pass vacuously)");
}

static void c_mirror_equal_energy_neither_gets_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t1 = mid(&tg, TANG);
    int t2 = mid(&tg, TANG);
    int e = mid(&tg, ENERGY_CARD);
    if (!ident(t1, TANG) || !ident(t2, TANG)) return;
    set_stage1(&tg, 0, 1, t1);
    set_stage1(&tg, 1, 1, t2);
    for (int i = 0; i < 7; i++) {
        test_add_to_energy(&tg, 0, e);
        test_add_to_energy(&tg, 1, e);
    }
    test_set_energy_active(&tg, 0, 7);
    test_set_energy_active(&tg, 1, 7);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, t1), 0, "P1 == P2 energy -> no +2 for P1");
    CHECK_EQ(cost_mod(&tg, t2), 0, "P2 == P1 energy -> no +2 for P2");
}

static void c_modified_cost_reduces_baton_pass_energy(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    CHECK_EQ(printed_cost(nine), 9, "fixture guard: PL!-sd1-006-SD costs 9");
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "setup: +2 modifier is present");
    test_add_to_hand(&tg, nine);
    test_set_energy_active(&tg, 0, 5);
    CHECK_EQ(active(&tg, 0), 5, "setup: exactly 5 active energy");
    /* 9 - (2 + 2) = 5: enough to baton-pass onto 唐可可's centre. */
    CHECK_EQ(test_try_play_to_stage(&tg, nine, 1), 1,
             "9 - 4 = 5 energy is enough to baton pass onto the modified slot");
}

static void c_baton_pass_fails_without_enough_energy(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2,
             "setup: +2 modifier is present (paired positive for this refusal)");
    test_add_to_hand(&tg, nine);
    test_set_energy_active(&tg, 0, 4);
    CHECK_EQ(test_try_play_to_stage(&tg, nine, 1), 0,
             "9 - 4 = 5 energy needed, only 4 -> the baton pass must be refused");
    CHECK_EQ(active(&tg, 0), 4, "a refused play spends nothing");
    CHECK(hand_has(&tg, 0, nine), "the arriving card stays in hand");
}

static void c_recalculate_is_idempotent(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "first recalc -> +2");
    test_recalc(&tg);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, t), 2,
             "repeated recalculate_constants must not stack the +2 (still 2, "
             "never 4)");
}

static void c_plus2_turns_off_when_opponent_catches_up(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int e = mid(&tg, ENERGY_CARD);
    if (!ident(t, TANG)) return;
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "setup: 7 vs 0 -> +2 (PAIRED POSITIVE)");
    for (int i = 0; i < 9; i++) test_add_to_energy(&tg, 1, e);
    test_set_energy_active(&tg, 1, 9);
    test_recalc(&tg);
    CHECK_EQ(active(&tg, 1), 9, "setup: P2 caught up to 9");
    CHECK_EQ(cost_mod(&tg, t), 0,
             "7 < 9 -> the +2 must be removed on the next recalc");
}

static void c_baton_pass_needs_base_cost_without_bonus(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    c_place_tang(&tg, t, 6, 0);
    CHECK_EQ(cost_mod(&tg, t), 0, "setup: no +2 (6 < 7)");
    test_add_to_hand(&tg, nine);
    test_set_energy_active(&tg, 0, 5);
    /* Without the +2 the replaced slot costs 2, so 9 - 2 = 7 > 5. */
    CHECK_EQ(test_try_play_to_stage(&tg, nine, 1), 0,
             "base slot cost 2 needs 9-2=7 energy, 5 is not enough -> refuse");
    CHECK_EQ(active(&tg, 0), 5, "a refused play spends nothing");
}

/* Find the PlayMemberToStage (action_type 14) action for `card_id`. */
static const RbGeneratedAction *c_gate_action(TestGame *tg, int card_id, int *count)
{
    RbGeneratedActionList list = rb_generate_action_candidates(&tg->state);
    const RbGeneratedAction *hit = NULL;
    for (int i = 0; i < list.count; i++) {
        if (list.actions[i].action_type == 14 && list.actions[i].has_parameters &&
            list.actions[i].parameters.card_id == card_id) {
            hit = &list.actions[i];
            break;
        }
    }
    if (count) *count = list.count;
    return hit;
}

static void c_gate_offers_discounted_baton_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "setup: +2 modifier present");
    test_add_to_hand(&tg, nine);

    int n = 0;
    const RbGeneratedAction *a = c_gate_action(&tg, nine, &n);
    CHECK(a != NULL, "PROMPT CONTROL: the 9-cost member's play action is offered");
    if (!a) return;
    CHECK_EQ(a->parameters.n_available_areas, RB_STAGE_SIZE,
             "the generator records one area entry per stage slot");
    const RbGeneratedArea *center = &a->parameters.available_areas[1];
    CHECK(center->available && center->is_baton_touch,
          "the centre slot is an available baton touch onto 唐可可");
    /* STRUCTURAL: the Rust test asserts center.cost == 5 and
       action.final_cost == 5. RbGeneratedArea has no cost field and
       RbGeneratedActionParameters has no final_cost, so the price is asserted
       through the same arithmetic the generator itself uses. */
    int arriving = c_price(&tg, nine);
    int replaced = printed_cost(t) + cost_mod(&tg, t);
    CHECK_EQ(arriving - replaced, 5,
             "the gate must price the baton at the DISCOUNTED 9-4=5, not 9-2=7");
    (void)n;
}

static void c_gate_without_bonus_offers_base_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    c_place_tang(&tg, t, 7, 7);
    CHECK_EQ(cost_mod(&tg, t), 0, "setup: no +2 (equal energy)");
    test_add_to_hand(&tg, nine);

    int n = 0;
    const RbGeneratedAction *a = c_gate_action(&tg, nine, &n);
    CHECK(a != NULL, "PROMPT CONTROL: the 9-cost member's play action is offered");
    if (!a) return;
    const RbGeneratedArea *center = &a->parameters.available_areas[1];
    CHECK(center->available && center->is_baton_touch,
          "the centre slot is still a baton touch without the bonus");
    int arriving = c_price(&tg, nine);
    int replaced = printed_cost(t) + cost_mod(&tg, t);
    CHECK_EQ(arriving - replaced, 7,
             "without the bonus the gate must price the baton at the base 9-2=7");
    (void)n;
}

static void c_baton_pays_discounted_energy(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    int nine = mid(&tg, COST9);
    if (!ident(t, TANG) || !ident(nine, COST9)) return;
    c_place_tang(&tg, t, 7, 0);
    CHECK_EQ(cost_mod(&tg, t), 2, "setup: +2 modifier present");
    test_add_to_hand(&tg, nine);
    test_set_energy_active(&tg, 0, 5);
    CHECK_EQ(active(&tg, 0), 5, "setup: 5 active energy");
    CHECK_EQ(test_try_play_to_stage(&tg, nine, 1), 1, "9-4=5 must succeed");
    CHECK_EQ(active(&tg, 0), 0,
             "paid exactly the discounted 5, not the base 7");
    CHECK(waitroom_has(&tg, 0, t), "唐可可 is baton-touched to the waitroom");
    CHECK_EQ(tg.state.p[0].stage[1], nine, "the 9-cost member occupies the centre");
}

static void c_own_play_cost_unaffected(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int t = mid(&tg, TANG);
    if (!ident(t, TANG)) return;
    test_give_energy(&tg, 7);
    test_recalc(&tg);
    test_add_to_hand(&tg, t);
    CHECK_EQ(cost_mod(&tg, t), 0,
             "setup: while in hand the self-targeted +2 does not apply");

    int n = 0;
    const RbGeneratedAction *a = c_gate_action(&tg, t, &n);
    CHECK(a != NULL, "PROMPT CONTROL: 唐可可's own play action is offered");
    if (!a) return;
    const RbGeneratedArea *center = &a->parameters.available_areas[1];
    CHECK(center->available && !center->is_baton_touch,
          "an empty centre is a normal play, not a baton touch");
    CHECK_EQ(c_price(&tg, t), 2,
             "own play cost is the base 2, not 2+2 (the +2 is on-stage only)");
    (void)n;

    CHECK_EQ(test_try_play_to_stage(&tg, t, 1), 1, "play 唐可可 for the base cost");
    CHECK_EQ(active(&tg, 0), 5, "paid base 2 out of 7 -> 5 remain");
}

/* ======================================================================
 * D. per_card/success_zone_score_six_stage_cost_three_bonus_test.rs (12)
 *    PL!-bp4-008-R 小泉花陽 (base cost 4)
 *    常時: 自分の成功ライブカード置き場にあるカードのスコアの合計が６以上で
 *    あるかぎり、ステージにいるこのメンバーのコストを＋３する。
 *
 *    NON-VACUITY: every negative carries cd_wired_op() plus a PAIRED POSITIVE
 *    on the same card and the same recalc path (D2 for D1/D4/D5/D7/D12).
 * ====================================================================== */

static int d_add_score3(TestGame *tg, int live)
{
    test_add_to_success(tg, live);
    return live;
}

static void d_below_threshold_no_cost_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    CHECK_EQ(printed_score(live), 3, "fixture guard: PL!-sd1-021-SD scores 3");
    CHECK(cd_wired_op(&tg, hana, "add", 3),
          "PARSER CONTROL: 小泉花陽 carries a 常時 modify_cost add +3 "
          "(PAIRED POSITIVE is D2 on the same ability)");
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(success_n(&tg, 0), 1, "setup: one score-3 success card (total 3)");
    CHECK_EQ(cost_mod(&tg, hana), 0, "score 3 < 6 -> no cost modifier");
}

static void d_at_threshold_has_cost_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3,
             "score 6 (== threshold) -> +3 cost modifier");
}

static void d_above_threshold_has_cost_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    set_stage1(&tg, 0, 1, hana);
    for (int i = 0; i < 3; i++) d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3,
             "score 9 (well above the threshold) -> still exactly +3, not +9");
}

static void d_dynamic_increase(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 0, "initially no modifier at score 3");
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3,
             "+3 lands as soon as the success-zone score total reaches 6");
}

static void d_dynamic_decrease(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3, "setup: +3 at score 6 (PAIRED POSITIVE)");
    tg.state.p[0].success.n--;
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 0, "the modifier is removed once the total drops to 3");
}

static void d_removed_from_stage_clears_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3, "+3 while on stage (PAIRED POSITIVE)");
    set_stage1(&tg, 0, 1, RB_EMPTY_SLOT);
    test_clear_mods_for_card(&tg, hana);
    test_recalc(&tg);
    CHECK(!stage_has(&tg, 0, hana), "setup: 小泉花陽 is off the stage");
    CHECK_EQ(cost_mod(&tg, hana), 0, "the modifier is cleared after leaving the stage");
}

static void d_not_on_stage_no_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    CHECK(cd_wired_op(&tg, hana, "add", 3),
          "PARSER CONTROL: 小泉花陽 carries a 常時 modify_cost add +3 "
          "(PAIRED POSITIVE is D2 on the same ability and the same recalc)");
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(success_n(&tg, 0), 2, "setup: two score-3 success cards, total 6");
    CHECK(!stage_has(&tg, 0, hana), "setup: 小泉花陽 is not on the stage");
    CHECK_EQ(cost_mod(&tg, hana), 0,
             "the success-zone score total is 6 but the modifier is stage-only");
}

static void d_play_cost_unaffected(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(live, LIVE_SCORE3)) return;
    CHECK_EQ(printed_cost(hana), 4, "fixture guard: 小泉花陽 base cost 4");
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_add_to_hand(&tg, hana);
    test_give_energy(&tg, 10);
    CHECK_EQ(test_try_play_to_stage(&tg, hana, 1), 1, "played from hand");
    CHECK_EQ(active(&tg, 0), 6,
             "played for the base cost 4 (the +3 is on-stage only) -> 6 of 10 left");
    CHECK_EQ(cost_mod(&tg, hana), 3,
             "the on-stage +3 is active once she is on the stage");
}

static void d_two_copies_each_gets_mod(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int a = mid(&tg, HANAYO);
    int b = mid(&tg, HANAYO);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(a, HANAYO) || !ident(b, HANAYO)) return;
    CHECK(a != b, "fixture guard: two distinct pool slots");
    set_stage3(&tg, 0, a, b, RB_EMPTY_SLOT);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, a), 3, "copy A gets its own +3");
    CHECK_EQ(cost_mod(&tg, b), 3, "copy B gets its own +3");
}

static void d_non_stackable_per_card_not_shared(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int filler = mid(&tg, FILLER);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(filler, FILLER)) return;
    set_stage3(&tg, 0, filler, hana, RB_EMPTY_SLOT);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, filler), 0,
             "an unrelated stage member gets no cost modifier (this is the "
             "control that makes D2's +3 on hana non-vacuous)");
    CHECK_EQ(cost_mod(&tg, hana), 3, "小泉花陽 gets exactly +3, not +3 per zone card");
}

static void d_baton_touch_uses_modified_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int arriver = mid(&tg, ARRIVER);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(arriver, ARRIVER)) return;
    CHECK_EQ(printed_cost(arriver), 11, "fixture guard: PL!-sd1-001-SD costs 11");
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 3, "小泉花陽 has +3 before the baton touch");
    test_add_to_hand(&tg, arriver);
    test_give_energy(&tg, 5);
    CHECK_EQ(active(&tg, 0), 5, "setup: 5 active energy");
    CHECK_EQ(test_try_play_to_stage(&tg, arriver, 1), 1,
             "11 - 7 = 4 is affordable with 5");
    CHECK_EQ(active(&tg, 0), 1,
             "paid 4 (11 - 7 with the modifier), not 7 (11 - 4, base only)");
    CHECK_EQ(cost_mod(&tg, hana), 3,
             "the on-stage +3 was live at the moment the arriving cost was priced");
}

static void d_baton_touch_base_cost_when_below_threshold(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAYO);
    int arriver = mid(&tg, ARRIVER);
    int live = mid(&tg, LIVE_SCORE3);
    if (!ident(hana, HANAYO) || !ident(arriver, ARRIVER)) return;
    set_stage1(&tg, 0, 1, hana);
    d_add_score3(&tg, live);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, hana), 0,
             "setup: no cost modifier at success-zone score 3 (PAIRED POSITIVE "
             "for this refusal is D2, the same ability on the same recalc)");
    test_add_to_hand(&tg, arriver);
    test_give_energy(&tg, 5);
    CHECK_EQ(test_try_play_to_stage(&tg, arriver, 1), 0,
             "below the threshold the baton needs 11-4=7, 5 is not enough -> refuse");
    CHECK_EQ(active(&tg, 0), 5, "the refused play spends nothing");
}

/* ======================================================================
 * E. per_card/success_zone_stage_cost_deploy_energy_test.rs (1)
 *    PL!S-bp3-016-N 国木田花丸 (base cost 4)
 *    常時: 自分の成功ライブカード置き場にあるカード1枚につき、ステージにいる
 *    このメンバーのコストが+1される。
 *
 *    NOTE: test_parity_cost_compound.c section J already proves
 *    rb_compute_play_cost() prices this +1 per success card. What is NOT
 *    covered anywhere is the ENERGY ACTUALLY PAID, which is what this case
 *    asserts, so it is ported rather than declared a duplicate.
 * ====================================================================== */

static void e_success_zone_deploy_spends_base_plus_one(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int hana = mid(&tg, HANAMARU_Q155);
    int live = mid(&tg, FILLER_LIVE2);
    int filler = mid(&tg, FILLER);
    if (!ident(hana, HANAMARU_Q155) || !ident(live, FILLER_LIVE2)) return;
    int base = printed_cost(hana);
    CHECK_EQ(base, 4, "fixture guard: 国木田花丸 base cost 4 (cards.json)");
    CHECK(cd_wired_op(&tg, hana, "add", 1),
          "PARSER CONTROL: 国木田花丸 carries the 常時 modify_cost add +1");

    /* Deliberately asymmetric so a "paid the base" bug cannot coincide with
       the expected remainder: with base+6 = 10 energy, paying base (4) leaves 6
       and paying base+1 (5) leaves 5. The Rust test's own 5-vs-4 arithmetic is
       ambiguous under this engine because both wrong answers can land on 5. */
    test_add_to_success(&tg, live);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, base + 6);

    CHECK_EQ(active(&tg, 0), 10, "setup: 10 active energy, base 4");
    /* Price control: the engine CAN price the +1 ... */
    CHECK_EQ(c_price(&tg, hana), base + 1,
             "CONTROL: the success-zone card prices in exactly +1 (base 4 -> 5)");
    /* ... and this is the claim the Rust test makes about the energy. */
    CHECK_EQ(test_try_play_to_stage(&tg, hana, 1), 1, "played to the centre");
    int paid = 10 - active(&tg, 0);
    CHECK_EQ(paid, base + 1,
             "the ENERGY ACTUALLY CHARGED must be base + 1 = 5, not the base 4");
    CHECK_EQ(active(&tg, 0), 5,
             "exactly base + 1 = 5 energy consumed out of base + 6 = 10");
}

/* ======================================================================
 * F. activation_discount/abilityless_member_play_cost_reduction_test.rs (4)
 *    PL!S-bp5-001-R＋ 高海千歌 ab#1
 *    常時: 能力を持たないメンバーカードを自分の手札から登場させるためのコスト
 *    は1減る。
 *
 *    NON-VACUITY: F1 is the PAIRED POSITIVE that makes F2's "no reduction" real;
 *    F4 is the second paired positive (two sources, -2). Each also carries the
 *    c_price() control so a red "paid the full cost" line is attributable to
 *    the CHARGE path rather than to the PRICE path.
 * ====================================================================== */

static void f_reduction_applies_to_no_ability_member(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int chika = mid(&tg, CHIKA);
    int filler = mid(&tg, FILLER);
    if (!ident(chika, CHIKA) || !ident(filler, FILLER)) return;
    CHECK_EQ(printed_cost(filler), 4, "fixture guard: PL!-sd1-010-SD costs 4");
    CHECK(cd_wired_op(&tg, chika, "subtract", 1),
          "PARSER CONTROL: 高海千歌 carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE for F2 is this very case)");

    test_add_to_hand(&tg, chika);
    test_give_energy(&tg, printed_cost(chika) + 4 + 5);
    CHECK_EQ(test_try_play_to_stage(&tg, chika, 1), 1, "played 千歌 to the centre");
    cd_proceed_auto(&tg);
    cd_drain_first(&tg);

    int before = active(&tg, 0);
    test_add_to_hand(&tg, filler);
    /* The price control is what makes the paid-energy assertion below
       non-vacuous: rb_compute_play_cost prices 3, so if the CHARGE path paid 4
       the gap is a charge bug, not a pricing bug. */
    CHECK_EQ(c_price(&tg, filler), 3, "CONTROL: the ability-less member prices at 3");
    CHECK_EQ(test_try_play_to_stage(&tg, filler, 0), 1, "played the filler to the left");
    int after = active(&tg, 0);
    CHECK_EQ(before - after, 3, "cost 4 reduced to 3: exactly 3 energy paid");
}

static void f_no_reduction_for_member_with_ability(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int chika = mid(&tg, CHIKA);
    int has_ability = mid(&tg, HAS_ABILITY);
    if (!ident(chika, CHIKA) || !ident(has_ability, HAS_ABILITY)) return;
    int target_cost = printed_cost(has_ability);
    CHECK_EQ(target_cost, 2, "fixture guard: PL!SP-PR-003-PR costs 2");
    CHECK(cd_wired_op(&tg, chika, "subtract", 1),
          "PARSER CONTROL: 高海千歌 carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE is F1, the same ability on the same board)");

    test_add_to_hand(&tg, chika);
    test_give_energy(&tg, printed_cost(chika) + target_cost + 5);
    CHECK_EQ(test_try_play_to_stage(&tg, chika, 1), 1, "played 千歌 to the centre");
    cd_proceed_auto(&tg);
    cd_drain_first(&tg);

    int before = active(&tg, 0);
    test_add_to_hand(&tg, has_ability);
    /* Price control: a card that HAS an ability must keep its full price. */
    CHECK_EQ(c_price(&tg, has_ability), target_cost,
             "CONTROL: a member WITH an ability prices at its full cost");
    CHECK_EQ(test_try_play_to_stage(&tg, has_ability, 0), 1, "played it to the left");
    int after = active(&tg, 0);
    CHECK_EQ(before - after, target_cost,
             "the full printed cost is paid -- the no-ability filter excludes it");
}

static void f_cost_four_pays_three_energy(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int chika = mid(&tg, CHIKA);
    int no_ability = mid(&tg, FILLER);
    if (!ident(chika, CHIKA) || !ident(no_ability, FILLER)) return;
    CHECK_EQ(printed_cost(no_ability), 4, "fixture guard: the filler costs 4");
    CHECK(cd_wired_op(&tg, chika, "subtract", 1),
          "PARSER CONTROL: 高海千歌 carries a 常時 modify_cost subtract 1");
    /* The Rust body of this test is byte-identical to F1 apart from the
       comment; it is ported anyway so the cluster's 48 are all accounted for. */
    test_add_to_hand(&tg, chika);
    test_give_energy(&tg, printed_cost(chika) + 4 + 5);
    CHECK_EQ(test_try_play_to_stage(&tg, chika, 1), 1, "played 千歌 to the centre");
    cd_proceed_auto(&tg);
    cd_drain_first(&tg);
    int before = active(&tg, 0);
    test_add_to_hand(&tg, no_ability);
    CHECK_EQ(test_try_play_to_stage(&tg, no_ability, 0), 1, "played the filler");
    CHECK_EQ(before - active(&tg, 0), 3, "cost 4 reduced to 3");
}

static void f_reduction_stacks(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int chika = mid(&tg, CHIKA);
    int chika2 = mid(&tg, CHIKA);
    int filler = mid(&tg, FILLER);
    if (!ident(chika, CHIKA) || !ident(chika2, CHIKA)) return;
    CHECK(chika != chika2, "fixture guard: two distinct 千歌 pool slots");
    CHECK(cd_wired_op(&tg, chika, "subtract", 1),
          "PARSER CONTROL: 高海千歌 carries a 常時 modify_cost subtract 1");

    test_add_to_hand(&tg, chika);
    test_add_to_hand(&tg, chika2);
    test_give_energy(&tg, printed_cost(chika) * 2 + 4 + 5);
    CHECK_EQ(test_try_play_to_stage(&tg, chika, 1), 1, "played 千歌 #1");
    cd_proceed_auto(&tg);
    cd_drain_first(&tg);
    CHECK_EQ(test_try_play_to_stage(&tg, chika2, 0), 1, "played 千歌 #2");
    cd_proceed_auto(&tg);
    cd_drain_first(&tg);
    CHECK_EQ(stage_has(&tg, 0, chika), 1, "千歌 #1 is on the stage");
    CHECK_EQ(stage_has(&tg, 0, chika2), 1, "千歌 #2 is on the stage");

    int before = active(&tg, 0);
    test_add_to_hand(&tg, filler);
    CHECK_EQ(c_price(&tg, filler), 2,
             "CONTROL: two 千歌 sources stack to -2, so the filler prices at 2");
    CHECK_EQ(test_try_play_to_stage(&tg, filler, 2), 1, "played the filler right");
    CHECK_EQ(before - active(&tg, 0), 2,
             "cost 4 reduced by 2 -> exactly 2 energy paid (the reductions stack)");
}

/* ======================================================================
 * G. activation_discount/distinct_group_activation_discount_q228_test.rs (5)
 *    PL!-bp5-004-R＋ 園田海未
 *    起動 EEEE(4): 相手のステージにいるコスト10以下のメンバー1人をウェイトに
 *    する。この能力を起動するためのコストは自分のステージにいるメンバーの中の
 *    グループ名1種類につき、E減る。
 *
 *    NON-VACUITY: rb_distinct_stage_groups() is the control that makes every
 *    cost assertion in this section non-vacuous -- it reads the SAME
 *    group-name set the ability's cost_reduction_per_group reads, on the SAME
 *    board, immediately before the activation. If the group scan returned 0
 *    these tests would pass while the discount did nothing; they cannot.
 *
 *    STRUCTURAL GAP: rb_generate_action_candidates() emits only RB_ACTION_PASS
 *    and RB_ACTION_PLAY_MEMBER (14). There is no UseAbility action and no
 *    base_cost / final_cost field, so G3's "the UI must not offer an
 *    unpayable activation" and G4's "the offer must display the reduced cost"
 *    are asserted through what resolution actually charges, which is the
 *    behaviour underneath the gate. This is recorded, not papered over.
 *
 *    DRAIN ORDER: the C queue opens 起動 with a "shall this ability run?" step
 *    BEFORE the cost. cd_proceed_auto() answers THAT step; only then does the
 *    change_state target prompt exist. Answering the target with the skip index
 *    would refuse the whole ability, so it is never used as "decline".
 * ====================================================================== */

/* src/turn/phase.c exports these; they are absent from rabuka.h (that header
   is another agent's), so they are declared here. They are used as CONTROLS
   only: no test below substitutes them for an engine decision. */
int rb_distinct_stage_groups_probe(const GameState *g, int pl);
static int cd_activate(TestGame *tg, int card_id, int answer_idx)
{
    int rc = rb_activate_card(&tg->state, 0, card_id);
    cd_proceed_auto(tg);
    if (answer_idx >= 0 && test_has_pending_choice(tg))
        rb_resume_with_choice(&tg->state, answer_idx);
    cd_drain(tg);
    return rc;
}

static void g_four_unique_groups_cost_zero(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int umi = mid(&tg, UMI_R);
    int multi = mid(&tg, MULTI3);
    if (!ident(umi, UMI_R) || !ident(multi, MULTI3)) return;
    /* Exactly two cards, no filler. */
    set_stage3(&tg, 0, umi, multi, RB_EMPTY_SLOT);
    CHECK_EQ(rb_distinct_stage_groups(&tg.state, 0), 4,
             "PROMPT/CONTROL: umi(mu's) + the 3-series joint card = 4 distinct "
             "groups, read off the same group set the cost discount reads");

    /* Non-vacuity: 1 energy is DELIBERATELY too little for the printed 4, so
       "cost 0" is the only way this activation can happen at all. With 0
       energy (as a naive port would use) the assertion would hold for a
       refused activation. */
    test_give_energy(&tg, 1);
    CHECK_EQ(active(&tg, 0), 1, "setup: 1 active energy, less than the printed 4");
    cd_activate(&tg, umi, -1);
    CHECK_EQ(active(&tg, 0), 1,
             "cost 0: the single energy is NOT consumed (4 groups x 1 = 4 of 4). "
             "The activation really happened -- at 1 energy the printed cost of 4 "
             "would have been unaffordable and spent nothing differently.");
}

static void g_zero_cost_all_rarities(void)
{
    static TestGame tg;
    static const char *rarities[4] = { UMI_AR, UMI_P, UMI_R, UMI_SEC };
    int n_solved = 0;
    for (int i = 0; i < 4; i++) {
        case_begin(&tg);
        bare_board(&tg);
        int umi = mid(&tg, rarities[i]);
        int multi = mid(&tg, MULTI3);
        if (!ident(umi, rarities[i]) || !ident(multi, MULTI3)) continue;
        set_stage3(&tg, 0, umi, multi, RB_EMPTY_SLOT);
        CHECK_EQ(rb_distinct_stage_groups(&tg.state, 0), 4,
                 "PROMPT/CONTROL: four distinct groups for this rarity");
        test_give_energy(&tg, 1);
        CHECK_EQ(active(&tg, 0), 1,
                 "setup: 1 active energy, deliberately below the printed 4");
        cd_activate(&tg, umi, -1);
        CHECK_EQ(active(&tg, 0), 1,
                 "four groups must reduce the 4-energy activation cost to zero "
                 "for this rarity too -- at 1 energy a cost-4 activation could "
                 "not have run at all");
        n_solved++;
    }
    CHECK_EQ(n_solved, 4, "all four rarities were actually exercised");
}

static void g_one_group_cost_three_not_offered_at_two_energy(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int umi = mid(&tg, UMI_R);
    int filler = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    if (!ident(umi, UMI_R) || !ident(filler, FILLER)) return;
    set_stage3(&tg, 0, umi, filler, filler2);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 2);
    CHECK_EQ(rb_distinct_stage_groups(&tg.state, 0), 1,
             "PROMPT/CONTROL: all three members are mu's, so exactly 1 group");
    /* The effective cost is 4 - 1 = 3, above the 2 active energy. A direct
       press must be rejected and must spend nothing. (The Rust test asserts
       the UI withholds the offer; the C generator has no UseAbility action at
       all -- see the section header -- so the affordability claim is asserted
       on the path that actually exists.) */
    cd_activate(&tg, umi, -1);
    CHECK_EQ(active(&tg, 0), 2,
             "an activation costing 3 cannot be paid with 2 active energy, so it "
             "must be rejected and must spend nothing");
    CHECK(!waitroom_has(&tg, 1, filler) && !waitroom_has(&tg, 1, filler2),
           "a rejected activation changes no zone");
}

static void g_reduction_charges_effective_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int umi = mid(&tg, UMI_R);
    int other = mid(&tg, LIELLA);
    if (!ident(umi, UMI_R) || !ident(other, LIELLA)) return;
    set_stage3(&tg, 0, umi, other, RB_EMPTY_SLOT);
    test_give_energy(&tg, 5);
    CHECK_EQ(rb_distinct_stage_groups(&tg.state, 0), 2,
             "PROMPT/CONTROL: mu's + Liella! = 2 distinct groups");
    cd_activate(&tg, umi, -1);
    CHECK_EQ(active(&tg, 0), 3,
             "resolution charges the REDUCED cost 2 (5 - 2 = 3), not the "
             "printed 4");
}

static void g_selects_one_eligible_opponent_physical_instance(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int umi = mid(&tg, UMI_R);
    int ally = mid(&tg, FILLER);
    int target_a = mid(&tg, SAYAKA);
    int target_b = mid(&tg, SAYAKA);
    int too_expensive = mid(&tg, RUBY);
    if (!ident(umi, UMI_R) || !ident(target_a, SAYAKA) ||
        !ident(target_b, SAYAKA) || !ident(too_expensive, RUBY)) return;
    CHECK(target_a != target_b, "fixture guard: two DISTINCT physical instances");
    {
        char na[128], nb[128];
        CHECK(card_no_of(target_a, na, sizeof na) &&
              card_no_of(target_b, nb, sizeof nb) && !strcmp(na, nb),
              "fixture guard: both instances are the same print, byte-exactly");
    }
    CHECK_EQ(printed_cost(target_a), 2, "fixture guard: 村野さやか costs 2 (<= 10)");
    CHECK(printed_cost(too_expensive) > 10,
          "fixture guard: 黒澤ルビィ costs 15, over the cost_limit of 10");

    set_stage3(&tg, 0, ally, umi, RB_EMPTY_SLOT);
    set_stage3(&tg, 1, target_a, target_b, too_expensive);
    test_give_energy(&tg, 3);
    CHECK_EQ(rb_distinct_stage_groups(&tg.state, 0), 1,
             "PROMPT/CONTROL: ally (mu's) + umi (mu's) = 1 group -> cost 3");

    rb_activate_card(&tg.state, 0, umi);
    cd_proceed_auto(&tg);
    /* CONTROL (this is what makes the wait assertions non-vacuous): the
       engine DID raise a target prompt and the pick was accepted -- the trace
       shows a target prompt and a pick. Without the prompt this whole case is
       just "nothing happened". */
    CHECK(test_has_pending_choice(&tg),
          "PROMPT CONTROL: the change_state target prompt is raised (the "
          "ability really ran -- without this the wait assertions below would "
          "be vacuous)");
    if (test_has_pending_choice(&tg)) rb_resume_with_choice(&tg.state, 1);
    cd_drain(&tg);

    /* Report every orientation so the trace shows WHICH instance waited, then
       assert the Rust claim. */
    const char *oa = rb_mods_get_orientation(&tg.state.mods, target_a);
    const char *ob = rb_mods_get_orientation(&tg.state.mods, target_b);
    const char *oc = rb_mods_get_orientation(&tg.state.mods, too_expensive);
    const char *ox = rb_mods_get_orientation(&tg.state.mods, ally);
    printf("        G5 ids: umi=%d ally=%d target_a=%d target_b=%d expensive=%d\n",
           umi, ally, target_a, target_b, too_expensive);
    printf("        G5 orientations: ally=%s target_a=%s target_b=%s expensive=%s\n",
           ox ? ox : "(none)", oa ? oa : "(none)", ob ? ob : "(none)",
           oc ? oc : "(none)");
    CHECK(ob && !strcmp(ob, "wait"),
          "the SECOND physical instance is the one that waited");
    CHECK(!(oa && !strcmp(oa, "wait")),
          "the first instance did not wait");
    CHECK(!(oc && !strcmp(oc, "wait")),
          "the over-cost member was not offered (cost_limit 10 vs cost 15)");
    CHECK_EQ(waitroom_n(&tg, 1), 0, "a wait state change is not a discard");
    CHECK_EQ(active(&tg, 0), 0, "the reduced 3-energy cost was charged in full");
}

/* ======================================================================
 * H. activation_discount/hand_card_count_play_cost_reduction_test.rs (4)
 *    LL-bp2-001-R＋ 渡辺 曜&鬼塚夏美&大沢瑠璃乃 ab#0
 *    常時: 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札
 *    1枚につき、1少なくなる。
 *
 *    NOTE ON THE TWO READS: the Rust file reads TWO different maps.
 *      H1/H2 read mods.constant_cost_bonuses -- the constant-scan
 *      attribution -- and expect -(hand-1).
 *      H3/H4 read the play-time calculate_play_cost_reduction, which uses the
 *      +1 hand_count convention and so is a different number again. Each C
 *      assertion below uses the map its Rust counterpart uses, never a
 *      substitute.
 * ====================================================================== */

static void h_hand_card_cost_reduced_by_hand_count_minus_1(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int card = mid(&tg, KOTORI);
    int filler = mid(&tg, FILLER);
    if (!ident(card, KOTORI) || !ident(filler, FILLER)) return;
    CHECK_EQ(printed_cost(card), 20, "fixture guard: LL-bp2-001-R+ base cost 20");
    CHECK(cd_wired_op(&tg, card, "subtract", 1),
          "PARSER CONTROL: the triple carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE for H2's zero is this case)");

    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);
    CHECK_EQ(hand_n(&tg, 0), 3, "setup: three cards in hand");
    CHECK_EQ(constant_cost(&tg, card), -2,
             "the card's constant cost bonus is -(hand_count - 1) = -2");
}

static void h_hand_card_alone_no_reduction(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int card = mid(&tg, KOTORI);
    int filler = mid(&tg, FILLER);
    if (!ident(card, KOTORI) || !ident(filler, FILLER)) return;
    CHECK(cd_wired_op(&tg, card, "subtract", 1),
          "PARSER CONTROL: the triple carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE for this zero is H1, same ability, same recalc)");
    test_add_to_hand(&tg, card);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);
    CHECK_EQ(hand_n(&tg, 0), 1, "setup: the card is alone in hand");
    CHECK_EQ(constant_cost(&tg, card), 0,
             "no reduction when the card is alone in hand (hand-1 = 0)");
    CHECK_EQ(cost_set(&tg, card), 0, "and no set-cost override is recorded");
}

static void h_on_stage_card_does_not_reduce_other_cards_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int card = mid(&tg, KOTORI);
    int other = mid(&tg, FILLER);
    int filler = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    if (!ident(card, KOTORI) || !ident(other, FILLER)) return;
    set_stage3(&tg, 0, filler, card, filler2);
    test_add_to_hand(&tg, other);
    fill_decks(&tg, filler, 30);
    CHECK_EQ(hand_n(&tg, 0), 1, "setup: one card in hand");
    CHECK(cd_wired_op(&tg, card, "subtract", 1),
          "PARSER CONTROL: the triple carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE is H1: the same ability DOES reduce in hand)");
    int reduction = rb_calculate_play_cost_reduction(&tg.state, 0, hand_n(&tg, 0), other);
    CHECK_EQ(reduction, 0,
             "the same card ON STAGE must not discount another hand card");
}

static void h_on_stage_card_ignores_hand_for_other_card_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int card = mid(&tg, KOTORI);
    int other = mid(&tg, FILLER);
    int filler = mid(&tg, FILLER);
    int filler2 = mid(&tg, FILLER);
    if (!ident(card, KOTORI) || !ident(other, FILLER)) return;
    set_stage3(&tg, 0, filler, card, filler2);
    test_add_to_hand(&tg, other);
    for (int i = 0; i < 3; i++) test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    CHECK_EQ(hand_n(&tg, 0), 4, "setup: four cards in hand");
    CHECK(cd_wired_op(&tg, card, "subtract", 1),
          "PARSER CONTROL: the triple carries a 常時 modify_cost subtract 1 "
          "(PAIRED POSITIVE is H1: the same ability DOES reduce in hand)");
    int reduction = rb_calculate_play_cost_reduction(&tg.state, 0, hand_n(&tg, 0), other);
    CHECK_EQ(reduction, 0,
             "no reduction from a stage-resident source even with many hand cards");
}

/* ======================================================================
 * I. activation_discount/hand_size_play_cost_reduction_zero_floor_test.rs (1)
 *    LL-bp2-001-R＋: cost 20, reduced by 1 per other hand card, floored at 0.
 *
 *    NON-VACUITY: the first half asserts the 18-cost boundary from both sides
 *    (17 refused, 18 accepted), so the second half's "plays for 0 with 25
 *    cards" cannot pass on a board where the reduction is a no-op -- a no-op
 *    reduction would make the 17-energy play SUCCEED, not fail.
 * ====================================================================== */

static void i_hand_size_reduces_play_cost_with_zero_floor(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int kotori = mid(&tg, KOTORI);
    int filler = mid(&tg, FILLER);
    if (!ident(kotori, KOTORI) || !ident(filler, FILLER)) return;
    int base = printed_cost(kotori);
    CHECK_EQ(base, 20, "fixture guard: LL-bp2-001-R+ base cost 20");
    CHECK(cd_wired_op(&tg, kotori, "subtract", 1),
          "PARSER CONTROL: the triple carries a 常時 modify_cost subtract 1");

    test_add_to_hand(&tg, kotori);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 17);
    CHECK_EQ(active(&tg, 0), 17, "setup: 17 active energy");
    CHECK_EQ(c_price(&tg, kotori), 18,
             "CONTROL: 3 cards in hand prices the card at 20 - 2 = 18");
    CHECK_EQ(test_try_play_to_stage(&tg, kotori, 1), 0,
             "the play must be refused with only 17 energy for an 18-cost card");
    CHECK_EQ(active(&tg, 0), 17, "the refused play spends nothing");
    CHECK(hand_has(&tg, 0, kotori), "the card is still in hand");

    test_give_energy(&tg, 1);
    CHECK_EQ(active(&tg, 0), 18, "setup: 18 active energy");
    CHECK_EQ(test_try_play_to_stage(&tg, kotori, 1), 1,
             "with 18 energy the play succeeds (20 - 2 reduction)");
    CHECK_EQ(active(&tg, 0), 0, "all 18 energy is consumed");

    /* Second board: 25 fillers + the card, so the reduction would be 25 and
       the cost must clamp at 0 rather than going negative. */
    case_begin(&tg);
    bare_board(&tg);
    int kotori2 = mid(&tg, KOTORI);
    int filler2 = mid(&tg, FILLER);
    if (!ident(kotori2, KOTORI)) return;
    test_add_to_hand(&tg, kotori2);
    for (int i = 0; i < 25; i++) test_add_to_hand(&tg, filler2);
    CHECK_EQ(hand_n(&tg, 0), 26, "setup: 26 cards in hand");
    CHECK_EQ(c_price(&tg, kotori2), 0,
             "CONTROL: the reduction is clamped at 0, never negative");
    CHECK_EQ(test_try_play_to_stage(&tg, kotori2, 1), 1,
             "the card must be playable for 0 energy with 25 cards in hand");
    CHECK_EQ(active(&tg, 0), 0, "a 0-cost play consumes nothing");
}

/* ======================================================================
 * J. activation_discount/per_unit_member_cost_reduction_q249_test.rs (5)
 *    PL!HS-bp6-006-R＋ 安養寺 姫芽 (base cost 20)
 *    常時: 手札にあるこのメンバーカードのコストは、自分のステージにいる
 *    『みらくらぱーく！』のメンバー1人につき、2少なくなる。
 *
 *    NON-VACUITY: J1/J2/J3 are a 0/1/2-unit ladder, so J2's zero cannot pass
 *    on a dead effect (J1 would then also read -2) and J3's -4 cannot pass on
 *    a non-scaling one. The みらくらぱーく！ group filter is checked by J2
 *    (an empty stage is the filter's own zero) and by J3's two DIFFERENT
 *    prints, so a filter that matched everything would still be caught.
 * ====================================================================== */

static void j_one_unit_member_reduces_play_cost_by_two(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int himeno = mid(&tg, HIMENO_R);
    int mk = mid(&tg, MIRAKU_A);
    int filler = mid(&tg, FILLER);
    if (!ident(himeno, HIMENO_R) || !ident(mk, MIRAKU_A)) return;
    CHECK_EQ(printed_cost(himeno), 20, "fixture guard: 安養寺 姫芽 base cost 20");
    CHECK(cd_wired_op(&tg, himeno, "subtract", 2),
          "PARSER CONTROL: 姫芽 carries a 常時 modify_cost subtract 2");

    test_add_to_hand(&tg, himeno);
    set_stage3(&tg, 0, mk, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, himeno), -2,
             "1 みらくらぱーく！ member on stage -> cost -2");
    test_give_energy(&tg, 18);
    CHECK_EQ(test_try_play_to_stage(&tg, himeno, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 0, "all 18 energy consumed (20 base - 2 reduction)");
}

static void j_no_unit_members_leaves_play_cost_unchanged(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int himeno = mid(&tg, HIMENO_R);
    int filler = mid(&tg, FILLER);
    if (!ident(himeno, HIMENO_R)) return;
    CHECK(cd_wired_op(&tg, himeno, "subtract", 2),
          "PARSER CONTROL: 姫芽 carries a 常時 modify_cost subtract 2 "
          "(PAIRED POSITIVE for this zero is J1: the same ability returns -2 "
          "with one qualifying member)");
    test_add_to_hand(&tg, himeno);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, himeno), 0, "0 qualifying members -> cost unchanged");
    test_give_energy(&tg, 20);
    CHECK_EQ(test_try_play_to_stage(&tg, himeno, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 0, "all 20 energy consumed (full base cost)");
}

static void j_two_unit_members_reduce_play_cost_by_four(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int himeno = mid(&tg, HIMENO_R);
    int mk1 = mid(&tg, MIRAKU_A);
    int mk2 = mid(&tg, MIRAKU_B);
    int filler = mid(&tg, FILLER);
    if (!ident(himeno, HIMENO_R) || !ident(mk1, MIRAKU_A) ||
        !ident(mk2, MIRAKU_B)) return;
    CHECK(mk1 != mk2, "fixture guard: two distinct pool slots");
    CHECK(cd_wired_op(&tg, himeno, "subtract", 2),
          "PARSER CONTROL: 姫芽 carries a 常時 modify_cost subtract 2");
    test_add_to_hand(&tg, himeno);
    set_stage3(&tg, 0, mk1, mk2, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, himeno), -4,
             "2 みらくらぱーく！ members -> -4, i.e. the reduction SCALES per unit");
    test_give_energy(&tg, 16);
    CHECK_EQ(test_try_play_to_stage(&tg, himeno, 2), 1, "played to the right");
    CHECK_EQ(active(&tg, 0), 0, "all 16 energy consumed (20 - 4)");
}

static void j_three_unit_members_reduce_cost_before_baton_touch_q249(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int himeno = mid(&tg, HIMENO_R);
    int mk1 = mid(&tg, MIRAKU_A);
    int mk2 = mid(&tg, MIRAKU_B);
    int mk3 = mid(&tg, MIRAKU_A);
    int filler = mid(&tg, FILLER);
    if (!ident(himeno, HIMENO_R) || !ident(mk1, MIRAKU_A) ||
        !ident(mk2, MIRAKU_B) || !ident(mk3, MIRAKU_A)) return;
    CHECK(mk1 != mk3, "fixture guard: the third slot is a DISTINCT copy of みらくらぱーく！");
    CHECK_EQ(printed_cost(mk2), 10,
             "fixture guard: PL!HS-PR-005-PR costs 10, which is the baton credit");
    test_add_to_hand(&tg, himeno);
    set_stage3(&tg, 0, mk1, mk2, mk3);
    fill_decks(&tg, filler, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, himeno), -6,
             "3 physical みらくらぱーく！ members -> -6 (duplicates COUNT: this is "
             "the control that J1's single instance does not prove)");
    int reduction = rb_calculate_play_cost_reduction(&tg.state, 0,
                                                    hand_n(&tg, 0), himeno);
    CHECK_EQ(reduction, 6,
             "the play-time evaluator independently reports a reduction of 6");
    test_give_energy(&tg, 4);
    CHECK_EQ(test_try_play_to_stage(&tg, himeno, 1), 1,
             "20 - 6 - 10 (baton credit) = 4 is affordable");
    CHECK_EQ(active(&tg, 0), 0, "all 4 energy consumed");
}

static void j_p_variant_receives_mirakura_cost_reduction_q249(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int himeno = mid(&tg, HIMENO_P);
    int mk = mid(&tg, MIRAKU_A);
    int filler = mid(&tg, FILLER);
    if (!ident(himeno, HIMENO_P) || !ident(mk, MIRAKU_A)) return;
    CHECK_EQ(printed_cost(himeno), 20, "fixture guard: the P print also costs 20");
    CHECK(cd_wired_op(&tg, himeno, "subtract", 2),
          "PARSER CONTROL: the P print carries the same 常時 subtract 2");
    test_add_to_hand(&tg, himeno);
    set_stage3(&tg, 0, mk, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, himeno), -2, "the P print gets the same -2");
    test_give_energy(&tg, 18);
    CHECK_EQ(test_try_play_to_stage(&tg, himeno, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 0, "all 18 energy consumed");
}

/* ======================================================================
 * K. activation_discount/play_time_three_named_member_discard_cost_ten_test.rs
 *    (19)  LL-bp7-001-R＋ (base cost 15) ab#0
 *    常時: このカードのプレイに際し、自分の手札から「国木田花丸」と「優木せつ菜」
 *    と「嵐千砂都」のメンバーカードをそれぞれ1枚ずつ控え室に置いてもよい。
 *    そうしたとき、このカードのコストは10になる。
 *
 *    THE PROMPT. The C engine does not queue a SelectTarget here. rb_play_member
 *    detects the alt cost, sets g->ptc_active and RETURNS 0 with the card still
 *    in hand; rb_complete_play_with_cost then finishes the play. So:
 *      "the choice is offered"      <=>  ptc_active == 1
 *      "no choice is offered"       <=>  ptc_active == 0
 *    Every case here asserts one of those EXPLICITLY, and the accepting and
 *    declining arms differ in ENERGY as well (10 vs 15), so neither can be
 *    satisfied by the other. This is the file's largest vacuity risk and it is
 *    closed case by case.
 * ====================================================================== */

/* src/turn/phase.c exports; absent from rabuka.h. Used as CONTROLS only. */
int  rb_has_play_time_alt_cost_hand_cards(const GameState *g, int card_id);
int  rb_play_time_alt_cost_chars(const GameState *g, int card_id, char *chars,
                                 int chars_len);
int  rb_find_distinct_assignment_k(const GameState *g, int pl, int exclude_id,
                                   const char *const *chars, int n_chars,
                                   int *out, int out_cap);

/* Start the play. Returns 1 if the engine PAUSED it on the alt cost. */
static int k_start_play(TestGame *tg, int triple, int area)
{
    test_try_play_to_stage(tg, triple, area);
    return cd_ptc(tg);
}

/* The three required character names on LL-bp7-001-R＋, byte-exact with
   cards/abilities.json. Kept as macros so a mismatch is a one-line edit. */
#define CHR_HANAMARU "\xE5\x9B\xBD\xE6\x9C\xA8\xE7\x94\xB0\xE8\x8A\xB1\xE4\xB8\xB8" /* 国木田花丸 */
#define CHR_SETSUNA  "\xE5\x84\xAA\xE6\x9C\xA8\xE3\x81\x9B\xE3\x81\xA4\xE8\x8F\x9C" /* 優木せつ菜 */
#define CHR_CHISATO  "\xE5\xB5\x90\xE5\x8D\x83\xE7\xA0\x82\xE9\x83\xBD"             /* 嵐千砂都 */

/* PARSER control: the engine's own alt-cost reader must recover BOTH the three
   required character names and the set value 10 off this card. If it did not,
   no play in this section could ever offer the 10-energy alternative and every
   ptc_active assertion below would be measuring a broken parse rather than a
   broken choice. */
static void k_parsed_alt_cost(TestGame *tg, int triple)
{
    char names[512];
    int set_value = rb_play_time_alt_cost_chars(&tg->state, triple, names,
                                                (int)sizeof names);
    CHECK_EQ(set_value, 10,
             "PARSER CONTROL: the engine reads the alternative cost as 10");
    CHECK(strstr(names, CHR_HANAMARU) != NULL,
          "PARSER CONTROL: 国木田花丸 is among the required characters");
    CHECK(strstr(names, CHR_SETSUNA) != NULL,
          "PARSER CONTROL: 優木せつ菜 is among the required characters");
    CHECK(strstr(names, CHR_CHISATO) != NULL,
          "PARSER CONTROL: 嵐千砂都 is among the required characters");
}

static void k_passive_cost_not_set_by_discard(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3) || !ident(hana, HANAMARU) ||
        !ident(setsu, SETSUNA) || !ident(chisa, CHISATO)) return;
    CHECK_EQ(printed_cost(triple), 15, "fixture guard: LL-bp7-001-R+ base cost 15");
    CHECK(cd_wired_op(&tg, triple, "set", 10),
          "PARSER CONTROL: the triple carries a 常時 modify_cost set 10");

    test_add_to_hand(&tg, triple);
    test_add_to_discard(&tg, hana);
    test_add_to_discard(&tg, setsu);
    test_add_to_discard(&tg, chisa);
    test_recalc(&tg);
    CHECK_EQ(waitroom_n(&tg, 0), 3, "setup: the three named members are in the waitroom");
    CHECK_EQ(cost_mod(&tg, triple), 0,
             "the passive cost must NOT be set from the waitroom alone");
    CHECK_EQ(cost_set(&tg, triple), 0,
             "no set-cost override without the play-time choice");
}

static void k_passive_cost_not_set_with_hand_cards(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    CHECK(cd_wired_op(&tg, triple, "set", 10),
          "PARSER CONTROL: the triple carries a 常時 modify_cost set 10 "
          "(PAIRED POSITIVE is K3, which reaches 10 through the play choice)");
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, triple), 0,
             "holding the fodder in hand must not set the cost before the play");
    CHECK_EQ(cost_set(&tg, triple), 0,
             "no set-cost override without the play-time choice");
}

static void k_gameplay_accept_cost10_discards_hand(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);

    k_parsed_alt_cost(&tg, triple);
    CHECK(k_start_play(&tg, triple, 1),
          "PROMPT CONTROL: the play-time alternative cost is offered with 3 "
          "named members in hand (K4 asserts the OPPOSITE on 15 energy)");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);

    CHECK_EQ(active(&tg, 0), 0, "the accepted cost 10 is paid in full");
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
    CHECK(waitroom_has(&tg, 0, hana), "国木田花丸 is discarded to the waitroom");
    CHECK(waitroom_has(&tg, 0, setsu), "優木せつ菜 is discarded to the waitroom");
    CHECK(waitroom_has(&tg, 0, chisa), "嵐千砂都 is discarded to the waitroom");
    CHECK(!hand_has(&tg, 0, hana), "the hand no longer holds 国木田花丸");
    CHECK_EQ(waitroom_n(&tg, 0), 3, "exactly the three named cards were discarded");
}

static void k_gameplay_decline_pays15_keeps_hand(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 15);

    CHECK(k_start_play(&tg, triple, 1),
          "PROMPT CONTROL: the play-time alternative cost is offered; declining "
          "is a REAL choice, not the absence of one");
    cd_answer_ptc(&tg, 0);
    cd_proceed_auto(&tg);
    cd_drain(&tg);

    CHECK_EQ(active(&tg, 0), 0, "the full printed cost 15 is paid");
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
    CHECK(hand_has(&tg, 0, hana), "declining keeps 国木田花丸 in hand");
    CHECK(hand_has(&tg, 0, setsu), "declining keeps 優木せつ菜 in hand");
    CHECK(hand_has(&tg, 0, chisa), "declining keeps 嵐千砂都 in hand");
    CHECK_EQ(waitroom_n(&tg, 0), 0, "declining discards nothing");
}

static void k_gameplay_no_hand_cards_no_choice_pays15(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_give_energy(&tg, 15);
    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: NO play-time choice is offered when the hand cannot "
          "cover the three required members");
    if (paused) cd_answer_ptc(&tg, 0);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid instead of 10");
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
}

static void k_waitroom_fodder_cannot_enable_ten_energy_play(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_discard(&tg, hana);
    test_add_to_discard(&tg, setsu);
    test_add_to_discard(&tg, chisa);
    test_give_energy(&tg, 10);

    /* CONTROL: the engine HAS a hand-coverage predicate for exactly this. */
    CHECK(!rb_has_play_time_alt_cost_hand_cards(&tg.state, triple),
          "CONTROL: the engine's own hand-coverage predicate correctly reports "
          "that the hand cannot pay the named-discard cost");

    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: the hand holds only the card being played, so no "
          "10-energy alternative may be offered");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }

    CHECK_EQ(active(&tg, 0), 10,
             "a 15-cost play cannot be afforded with 10 energy, so the refusal "
             "must spend nothing");
    CHECK(stage_empty(&tg, 0), "a refused play leaves the stage empty");
    CHECK(hand_has(&tg, 0, triple), "the card is still in hand");
    CHECK(waitroom_has(&tg, 0, hana) && waitroom_has(&tg, 0, setsu) &&
          waitroom_has(&tg, 0, chisa),
          "the waitroom named-discard cost is not consumed by a refused play");
    CHECK(!test_has_pending_choice(&tg),
          "a refused play must not leave a prompt open");
}

static void k_accept_with_ten_energy_reaches_stage(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(rb_has_play_time_alt_cost_hand_cards(&tg.state, triple),
          "CONTROL: the engine's hand-coverage predicate says the cost IS payable");
    CHECK(k_start_play(&tg, triple, 1), "PROMPT CONTROL: the choice is offered");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
}

static void k_gameplay_incomplete_hand_no_choice(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_give_energy(&tg, 15);
    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: 嵐千砂都 is missing from the hand, so no choice");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid");
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
}

static void k_second_copy_can_be_used_as_one_fodder(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int play = mid(&tg, TRIPLE3);
    int fodder = mid(&tg, TRIPLE3);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(play, TRIPLE3) || !ident(fodder, TRIPLE3)) return;
    CHECK(play != fodder, "fixture guard: a DISTINCT copy is used as the fodder");
    test_add_to_hand(&tg, play);
    test_add_to_hand(&tg, fodder);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(k_start_play(&tg, play, 1),
          "PROMPT CONTROL: the multi-name second copy satisfies ONE required "
          "character, so the 10-energy cost is offered");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, play), "the played copy reaches the stage");
    CHECK(waitroom_has(&tg, 0, fodder), "the second copy is the discarded fodder");
    CHECK_EQ(active(&tg, 0), 0, "the 10-energy cost is paid in full");
}

static void k_one_card_cannot_cover_two_slots(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int play = mid(&tg, TRIPLE3);
    int fodder = mid(&tg, TRIPLE3);
    int setsu = mid(&tg, SETSUNA);
    if (!ident(play, TRIPLE3)) return;
    test_add_to_hand(&tg, play);
    test_add_to_hand(&tg, fodder);
    test_add_to_hand(&tg, setsu);
    test_give_energy(&tg, 15);
    /* Only two distinct fodder cards exist, so three slots cannot be filled. */
    CHECK(!rb_has_play_time_alt_cost_hand_cards(&tg.state, play),
          "CONTROL: the engine's predicate reports NO distinct assignment for "
          "3 slots with 2 fodder cards");
    int paused = k_start_play(&tg, play, 1);
    CHECK(!paused,
          "PROMPT CONTROL: a single multi-name card must not count for two slots");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid");
    CHECK(stage_has(&tg, 0, play), "the triple reaches the stage");
}

static void k_optimal_assignment_with_multi_name(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int play = mid(&tg, TRIPLE3);
    int fodder = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int chisa = mid(&tg, CHISATO);
    if (!ident(play, TRIPLE3)) return;
    test_add_to_hand(&tg, play);
    test_add_to_hand(&tg, fodder);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(rb_has_play_time_alt_cost_hand_cards(&tg.state, play),
          "CONTROL: a distinct assignment exists (fodder -> 優木せつ菜)");
    CHECK(k_start_play(&tg, play, 1),
          "PROMPT CONTROL: the optimal assignment makes the 10-energy cost "
          "available -- a greedy matcher that consumed the multi-name copy on "
          "国木田花丸 would fail here");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, play), "the triple reaches the stage");
    CHECK_EQ(waitroom_n(&tg, 0), 3, "exactly three cards are discarded");
}

static void k_played_card_not_counted_as_fodder(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_give_energy(&tg, 15);
    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: the card being played is excluded from the hand "
          "check, so it cannot be its own fodder");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid");
}

static void k_accept_with_ten_energy_pays_ten(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(k_start_play(&tg, triple, 1),
          "PROMPT CONTROL: with only 10 active energy the minimum-cost path (10) "
          "must still be offered");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
    CHECK_EQ(active(&tg, 0), 0, "the 10-energy cost is paid in full");
}

static void k_non_member_dual_name_does_not_count(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int energy_card = mid(&tg, CHISATO_ENERGY);
    if (!ident(triple, TRIPLE3) || !ident(energy_card, CHISATO_ENERGY)) return;
    /* cards.json: PL!SP-pb1-038-SRE is an energy card named 澁谷かのん＆嵐 千砂都. */
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, energy_card);
    test_give_energy(&tg, 15);
    CHECK(!rb_has_play_time_alt_cost_hand_cards(&tg.state, triple),
          "CONTROL: an ENERGY card carrying 嵐千砂都's name is not a member "
          "card, so the engine's predicate refuses the cost");
    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: the energy card must not count as member fodder");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid");
    CHECK(hand_has(&tg, 0, energy_card), "the energy card is untouched");
}

static void k_with_extra_unrelated_cards_still_offers_choice(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    int extra1 = mid(&tg, FILLER);
    int extra2 = mid(&tg, "PL!HS-bp1-005-PR");
    if (!ident(triple, TRIPLE3) || !ident(extra2, "PL!HS-bp1-005-PR")) return;
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_add_to_hand(&tg, extra1);
    test_add_to_hand(&tg, extra2);
    test_give_energy(&tg, 10);
    CHECK(k_start_play(&tg, triple, 1),
          "PROMPT CONTROL: two unrelated extra hand cards must not stop the "
          "choice being offered");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, triple), "the triple reaches the stage");
    CHECK(hand_has(&tg, 0, extra1), "the unrelated mu's card stays in hand");
    CHECK(hand_has(&tg, 0, extra2), "the unrelated 蓮ノ空 card stays in hand");
    CHECK_EQ(waitroom_n(&tg, 0), 3, "exactly three cards are discarded");
}

static void k_many_duplicates_still_finds_assignment(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int play = mid(&tg, TRIPLE3);
    int fodder = mid(&tg, TRIPLE3);
    int hana2 = mid(&tg, "PL!S-bp2-007-R" FP);
    int hana3 = mid(&tg, "PL!S-bp3-016-N");
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(play, TRIPLE3) || !ident(hana2, "PL!S-bp2-007-R" FP) ||
        !ident(hana3, "PL!S-bp3-016-N")) return;
    test_add_to_hand(&tg, play);
    test_add_to_hand(&tg, fodder);
    test_add_to_hand(&tg, hana2);
    test_add_to_hand(&tg, hana3);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(k_start_play(&tg, play, 1),
          "PROMPT CONTROL: three 国木田花丸 variants plus a multi-name copy must "
          "still yield a DISTINCT assignment");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK(stage_has(&tg, 0, play), "the played copy reaches the stage");
    CHECK_EQ(waitroom_n(&tg, 0), 3, "exactly three cards are discarded");
    /* 6 cards in hand, 1 played, 3 discarded -> 2 remain. */
    CHECK_EQ(hand_n(&tg, 0), 2, "6 in hand - 1 played - 3 discarded = 2 remain");
}

static void k_combinatorial_variant_fuzz(void)
{
    static TestGame tg;
    static const char *hanamaru_variants[4] = {
        "PL!S-bp2-016-N", "PL!S-bp2-007-R" FP, "PL!S-bp3-016-N", "PL!S-bp7-016-N"
    };
    static const char *setsuna_variants[4] = {
        "PL!N-PR-009-PR", "PL!N-bp5-007-R" FP, "PL!N-bp7-019-N", "PL!N-sd1-007-SD"
    };
    static const char *chisato_variants[4] = {
        "PL!SP-pb1-014-PR", "PL!SP-bp5-014-N", "PL!SP-bp7-014-N", "PL!SP-pb2-014-R"
    };
    int combos = 0, offered = 0, placed = 0;
    for (int a = 0; a < 4; a++) {
        for (int b = 0; b < 4; b++) {
            for (int c = 0; c < 4; c++) {
                combos++;
                case_begin(&tg);
                bare_board(&tg);
                int triple = mid(&tg, TRIPLE3);
                int hana = mid(&tg, hanamaru_variants[a]);
                int setsu = mid(&tg, setsuna_variants[b]);
                int chisa = mid(&tg, chisato_variants[c]);
                if (triple < 0 || hana < 0 || setsu < 0 || chisa < 0) continue;
                if (!ident(hana, hanamaru_variants[a]) ||
                    !ident(setsu, setsuna_variants[b]) ||
                    !ident(chisa, chisato_variants[c])) continue;
                test_add_to_hand(&tg, triple);
                test_add_to_hand(&tg, hana);
                test_add_to_hand(&tg, setsu);
                test_add_to_hand(&tg, chisa);
                test_give_energy(&tg, 10);
                if (k_start_play(&tg, triple, 1)) {
                    offered++;
                    cd_answer_ptc(&tg, 1);
                    cd_proceed_auto(&tg);
                    cd_drain(&tg);
                    if (stage_has(&tg, 0, triple)) placed++;
                }
            }
        }
    }
    CHECK_EQ(combos, 64, "all 4x4x4 = 64 variant combinations were exercised");
    CHECK_EQ(offered, combos,
             "every (国木田花丸, 優木せつ菜, 嵐千砂都) print combination must offer "
             "the 10-energy alternative");
    CHECK_EQ(placed, combos,
             "every combination must actually reach the stage for 10 energy");
}

static void k_cost_cleared_after_play_and_second_play_costs_15(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple1 = mid(&tg, TRIPLE3);
    int hana = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple1, TRIPLE3)) return;
    test_add_to_hand(&tg, triple1);
    test_add_to_hand(&tg, hana);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 10);
    CHECK(k_start_play(&tg, triple1, 1), "PROMPT CONTROL: first play offers 10");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK_EQ(active(&tg, 0), 0, "first play costs 10");

    int triple2 = mid(&tg, TRIPLE3);
    if (!ident(triple2, TRIPLE3)) return;
    test_add_to_hand(&tg, triple2);
    test_give_energy(&tg, 15);
    int paused = k_start_play(&tg, triple2, 0);
    CHECK(!paused,
          "PROMPT CONTROL: the fodder is exhausted, so the second play has no "
          "alternative -- a lingering set-cost would still offer one");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK(stage_has(&tg, 0, triple2), "the second copy reaches the stage");
    CHECK_EQ(active(&tg, 0), 0, "15 is paid for the second play");
    CHECK_EQ(cost_set(&tg, triple2), 0,
             "no set-cost override lingers on the second copy");
}

static void k_stage_member_not_counted_as_hand_fodder(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int triple = mid(&tg, TRIPLE3);
    int hana_stage = mid(&tg, HANAMARU);
    int setsu = mid(&tg, SETSUNA);
    int chisa = mid(&tg, CHISATO);
    if (!ident(triple, TRIPLE3)) return;
    set_stage1(&tg, 0, 0, hana_stage);
    test_add_to_hand(&tg, triple);
    test_add_to_hand(&tg, setsu);
    test_add_to_hand(&tg, chisa);
    test_give_energy(&tg, 15);
    CHECK(stage_has(&tg, 0, hana_stage), "setup: 国木田花丸 is on the stage, not in hand");
    CHECK(!rb_has_play_time_alt_cost_hand_cards(&tg.state, triple),
          "CONTROL: the engine's predicate refuses the cost: the named-discard "
          "is 手札 only, a stage member cannot pay it");
    int paused = k_start_play(&tg, triple, 1);
    CHECK(!paused,
          "PROMPT CONTROL: a stage-resident 国木田花丸 must not enable the "
          "10-energy play");
    if (paused) { cd_answer_ptc(&tg, 0); cd_proceed_auto(&tg); cd_drain(&tg); }
    CHECK(stage_has(&tg, 0, triple), "the triple still reaches the stage");
    CHECK_EQ(active(&tg, 0), 0, "the full cost 15 is paid");
}

/* ======================================================================
 * L. activation_discount/shuffle_waitroom_members_optional_play_discount_test.rs
 *    (3)  PL!N-bp7-011-R＋ ミア・テイラー (base cost 13) ab#1
 *    常時: このカードをプレイする際、自分の控え室にあるすべてのメンバーカードを
 *    シャッフルし、デッキの下に置いてもよい。そうしたとき、このカードのコストは
 *    ２減る。
 *
 *    VACUITY WARNING (this is the trap the brief names): the DECLINE arm
 *    (cost 13 paid, waitroom unchanged) is exactly what the board looks like
 *    when the play-time reduction is never offered at all. L2 therefore
 *    asserts ptc_active == 1 FIRST; without that control L2 would read green
 *    on a completely dead feature. L1 and L3 additionally move cards, which a
 *    no-op cannot do.
 * ====================================================================== */

static void l_accept_reduces_cost_by_2(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int mia = mid(&tg, MIA);
    int filler = mid(&tg, FILLER);
    if (!ident(mia, MIA)) return;
    CHECK_EQ(printed_cost(mia), 13, "fixture guard: ミア・テイラー base cost 13");
    CHECK(cd_wired_op(&tg, mia, "subtract", 2),
          "PARSER CONTROL: ミア carries a 常時 modify_cost subtract 2");

    test_add_to_hand(&tg, mia);
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    test_give_energy(&tg, 13);

    int paused = k_start_play(&tg, mia, 1);
    CHECK(paused,
          "PROMPT CONTROL: playing ミア with waitroom members must offer the "
          "play-time cost reduction (L2's decline arm is vacuous without this)");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);

    CHECK_EQ(active(&tg, 0), 2, "accepted: 13 - 2 = 11 paid, 2 energy remain");
    CHECK_EQ(waitroom_n(&tg, 0), 0,
             "accepted: the waitroom members are shuffled out into the deck");
    CHECK_EQ(tg.state.p[0].deck.n, 2,
             "accepted: the two waitroom members moved to the deck bottom");
}

static void l_decline_pays_full_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int mia = mid(&tg, MIA);
    int filler = mid(&tg, FILLER);
    if (!ident(mia, MIA)) return;
    CHECK(cd_wired_op(&tg, mia, "subtract", 2),
          "PARSER CONTROL: ミア carries a 常時 modify_cost subtract 2");
    test_add_to_hand(&tg, mia);
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    test_give_energy(&tg, 13);

    CHECK(k_start_play(&tg, mia, 1),
          "PROMPT CONTROL: the choice is genuinely offered. Without this the "
          "three assertions below would pass on a board where the feature is "
          "simply dead, which is the vacuity the brief warns about");
    cd_answer_ptc(&tg, 0);
    cd_proceed_auto(&tg);
    cd_drain(&tg);

    CHECK_EQ(active(&tg, 0), 0, "declined: the full cost 13 is paid");
    CHECK_EQ(waitroom_n(&tg, 0), 2, "declined: the waitroom members stay put");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "declined: nothing is shuffled into the deck");
}

static void l_no_waitroom_members_still_offers_choice(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int mia = mid(&tg, MIA);
    if (!ident(mia, MIA)) return;
    test_add_to_hand(&tg, mia);
    test_give_energy(&tg, 13);
    CHECK(k_start_play(&tg, mia, 1),
          "PROMPT CONTROL: the choice is still offered with an empty waitroom");
    cd_answer_ptc(&tg, 1);
    cd_proceed_auto(&tg);
    cd_drain(&tg);
    CHECK_EQ(active(&tg, 0), 2,
             "accepted: the cost is reduced to 11 even with no waitroom members");
}

/* ======================================================================
 * M. activation_discount/success_zone_group_member_play_discount_test.rs (3)
 *    PL!N-sd2-003-SD2 桜坂しずく
 *    常時: 自分の成功ライブカード置き場に『虹ヶ咲』のカードがあるかぎり、手札に
 *    あるこのメンバーカードのコストは２減る。
 *
 *    NON-VACUITY: M1 is the paired positive for BOTH zeros. M3 is the group
 *    filter's own control: the SAME zone, the SAME position, one 虹ヶ咲 live vs
 *    one mu's live, differing only in series.
 * ====================================================================== */

static void m_hand_cost_reduced_with_nijigasaki_live_in_success_zone(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int shizuku = mid(&tg, SHIZUKU);
    int niji = mid(&tg, NIJI_LIVE);
    if (!ident(shizuku, SHIZUKU) || !ident(niji, NIJI_LIVE)) return;
    CHECK(cd_wired_op(&tg, shizuku, "subtract", 2),
          "PARSER CONTROL: 桜坂しずく carries a 常時 modify_cost subtract 2 "
          "(PAIRED POSITIVE for M2 and M3 is this case)");
    CHECK(rb_card_matches_group_str(niji, GRP_NIJI),
          "CONTROL: PL!N-sd1-025-SD really is a 虹ヶ咲 card (cards.json series "
          "ラブライブ！虹ヶ咲学園スクールアイドル同好会)");
    test_add_to_hand(&tg, shizuku);
    test_add_to_success(&tg, niji);
    test_recalc(&tg);
    CHECK_EQ(success_n(&tg, 0), 1, "setup: one card in the success live zone");
    CHECK_EQ(cost_mod(&tg, shizuku), -2,
             "a 虹ヶ咲 live card in the success zone discounts the hand card by 2");
}

static void m_hand_cost_full_without_success_zone_card(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int shizuku = mid(&tg, SHIZUKU);
    if (!ident(shizuku, SHIZUKU)) return;
    CHECK(cd_wired_op(&tg, shizuku, "subtract", 2),
          "PARSER CONTROL: 桜坂しずく carries a 常時 modify_cost subtract 2 "
          "(PAIRED POSITIVE is M1: the same ability, same board, minus the "
          "success-zone card)");
    test_add_to_hand(&tg, shizuku);
    test_recalc(&tg);
    CHECK_EQ(success_n(&tg, 0), 0, "setup: the success live zone is empty");
    CHECK_EQ(cost_mod(&tg, shizuku), 0, "an empty success zone -> full hand cost");
}

static void m_non_nijigasaki_live_does_not_reduce_cost(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int shizuku = mid(&tg, SHIZUKU);
    int aqours = mid(&tg, AQOURS_LIVE);
    if (!ident(shizuku, SHIZUKU) || !ident(aqours, AQOURS_LIVE)) return;
    CHECK(cd_wired_op(&tg, shizuku, "subtract", 2),
          "PARSER CONTROL: 桜坂しずく carries a 常時 modify_cost subtract 2 "
          "(PAIRED POSITIVE is M1)");
    CHECK(!rb_card_matches_group_str(aqours, GRP_NIJI),
          "CONTROL: PL!-sd1-020-SD is a mu's live card, NOT 虹ヶ咲 -- this is "
          "what makes the zero below a group verdict and not a dead effect");
    test_add_to_hand(&tg, shizuku);
    test_add_to_success(&tg, aqours);
    test_recalc(&tg);
    CHECK_EQ(success_n(&tg, 0), 1,
             "setup: the zone is occupied, only the series differs from M1");
    CHECK_EQ(cost_mod(&tg, shizuku), 0, "a non-虹ヶ咲 live card -> full hand cost");
}

/* ======================================================================
 * N. activation_discount/success_zone_high_cost_group_deploy_discount_test.rs (4)
 *    PL!-bp6-019-L Music S.T.A.R.T!!
 *    常時: このカードが自分の成功ライブカード置き場にあるかぎり、元々のコスト
 *    が17以上の『μ's』のメンバーカードを自分の手札から登場させるためのコスト
 *    は2減る。この効果は重複しない。
 *
 *    NON-VACUITY: N1 is the paired positive for all three zeros. N2/N3/N4 each
 *    differ from N1 in exactly ONE input (printed cost, group, zone occupancy),
 *    so a filter that ignored that input would be caught.
 * ====================================================================== */

static void n_reduces_cost_seventeen_group_member_by_two(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int music = mid(&tg, MUSIC);
    int maki = mid(&tg, MAKI);
    if (!ident(music, MUSIC) || !ident(maki, MAKI)) return;
    CHECK_EQ(printed_cost(maki), 17, "fixture guard: PL!-PR-015-PR costs 17");
    CHECK(rb_card_matches_group_str(maki, GRP_MUS),
          "CONTROL: PL!-PR-015-PR really is a mu's member (cards.json series "
          "ラブライブ！, unit BiBi)");
    CHECK(cd_wired_op(&tg, music, "subtract", 2),
          "PARSER CONTROL: Music S.T.A.R.T!! carries a 常時 modify_cost subtract 2 "
          "(PAIRED POSITIVE for N2/N3/N4 is this case)");

    test_add_to_success(&tg, music);
    test_add_to_hand(&tg, maki);
    test_give_energy(&tg, 20);
    CHECK_EQ(c_price(&tg, maki), 15,
             "CONTROL: a mu's member of printed cost >= 17 prices at 15");
    CHECK_EQ(test_try_play_to_stage(&tg, maki, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 5, "paid 17 - 2 = 15 out of 20");
}

static void n_does_not_reduce_low_cost_group_member(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int music = mid(&tg, MUSIC);
    int honoka = mid(&tg, HONOKA);
    if (!ident(music, MUSIC) || !ident(honoka, HONOKA)) return;
    CHECK_EQ(printed_cost(honoka), 4, "fixture guard: PL!-PR-001-PR costs 4");
    CHECK(cd_wired_op(&tg, music, "subtract", 2),
          "PARSER CONTROL: Music S.T.A.R.T!! carries the 常時 subtract 2 "
          "(PAIRED POSITIVE is N1, the same ability with a cost-17 mu's member)");
    test_add_to_success(&tg, music);
    test_add_to_hand(&tg, honoka);
    test_give_energy(&tg, 10);
    CHECK_EQ(c_price(&tg, honoka), 4,
             "CONTROL: a cost-4 mu's member is under the >= 17 threshold, so the "
             "price is the full 4");
    CHECK_EQ(test_try_play_to_stage(&tg, honoka, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 6, "the full cost 4 is paid out of 10");
}

static void n_does_not_reduce_wrong_group_member(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int music = mid(&tg, MUSIC);
    int mari = mid(&tg, MARI);
    if (!ident(music, MUSIC) || !ident(mari, MARI)) return;
    CHECK_EQ(printed_cost(mari), 17, "fixture guard: PL!S-bp2-008-P costs 17");
    CHECK(!rb_card_matches_group_str(mari, GRP_MUS),
          "CONTROL: PL!S-bp2-008-P 小原鞠莉 is Aqours, not mu's -- this is the "
          "only input that differs from N1, so the zero below is a group "
          "verdict and not a dead effect");
    CHECK(cd_wired_op(&tg, music, "subtract", 2),
          "PARSER CONTROL: Music S.T.A.R.T!! carries the 常時 subtract 2 "
          "(PAIRED POSITIVE is N1)");
    test_add_to_success(&tg, music);
    test_add_to_hand(&tg, mari);
    test_give_energy(&tg, 20);
    CHECK_EQ(c_price(&tg, mari), 17,
             "CONTROL: the same cost-17 member of the wrong group is not discounted");
    CHECK_EQ(test_try_play_to_stage(&tg, mari, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 3, "the full cost 17 is paid out of 20");
}

static void n_absent_after_source_removed_before_play(void)
{
    static TestGame tg;
    case_begin(&tg);
    bare_board(&tg);
    int music = mid(&tg, MUSIC);
    int maki = mid(&tg, MAKI);
    if (!ident(music, MUSIC) || !ident(maki, MAKI)) return;
    test_add_to_success(&tg, music);
    tg.state.p[0].success.n = 0;          /* the source leaves the zone */
    test_add_to_hand(&tg, maki);
    test_give_energy(&tg, 20);
    CHECK_EQ(success_n(&tg, 0), 0,
             "setup: Music S.T.A.R.T!! is no longer in the success live zone");
    CHECK(cd_wired_op(&tg, music, "subtract", 2),
          "PARSER CONTROL: the ability still exists on the card (PAIRED "
          "POSITIVE is N1: it discounts when the card IS in the zone)");
    CHECK_EQ(c_price(&tg, maki), 17,
             "CONTROL: with the source out of the zone the price is the full 17");
    CHECK_EQ(test_try_play_to_stage(&tg, maki, 1), 1, "played to the centre");
    CHECK_EQ(active(&tg, 0), 3, "the full cost 17 is paid out of 20");
}

/* ══════════════════════════════════════════════════════════════════════
 * fork harness
 * ══════════════════════════════════════════════════════════════════════ */

/* Per-case result, so the post-hoc vacuity audit can ask "did that paired
   positive actually pass?". Recorded by run(); never consulted by an
   assertion. */
enum { ST_NONE = 0, ST_OK, ST_RED, ST_CRASH, ST_SETUP, ST_VAC };
#define CD_MAX_CASES 128
static struct { const char *name; int status; int assertions; } cd_cases[CD_MAX_CASES];
static int cd_n_cases;

static int case_status(const char *name)
{
    for (int i = 0; i < cd_n_cases; i++)
        if (!strcmp(cd_cases[i].name, name)) return cd_cases[i].status;
    return ST_NONE;
}

/* How many assertions the case actually ran. Read only by the post-hoc vacuity
 * audit, which uses it to enforce "a vacuous case must assert something real or
 * be DELETED" mechanically instead of by review convention. */
static int case_assertions(const char *name)
{
    for (int i = 0; i < cd_n_cases; i++)
        if (!strcmp(cd_cases[i].name, name)) return cd_cases[i].assertions;
    return 0;
}

static void case_record(const char *name, int status, int ran)
{
    if (cd_n_cases >= CD_MAX_CASES) return;
    cd_cases[cd_n_cases].name = name;
    cd_cases[cd_n_cases].status = status;
    cd_cases[cd_n_cases].assertions = ran;
    cd_n_cases++;
}

typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int vacuous;
} CdChildCounters;

/* A child runs in its own address space, so its counters die with it. Each
   child writes its four counters through this pipe before _Exit; without the
   channel a hundred red assertions would be summarised as "0 assertions,
   0 failures" and the exit status would be 0. */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    int fds[2];
    if (pipe(fds) != 0) { fds[0] = fds[1] = -1; }

    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs, v0 = n_vacuous;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        int nv = (int)(n_vacuous - v0);
        if (nf) n_tests_failed++;
        else if (ns || nv) n_tests_setup++;
        else n_tests_ok++;
        printf("%-8s %s  [%d assertions, %d failures, %d setup bugs, %d vacuous]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : (nv ? "VACUOUS" : "ok")), name,
               (int)(assertions - a0), nf, ns, nv);
        fflush(stdout);
        if (fds[0] >= 0) { close(fds[0]); close(fds[1]); }
        return;
    }
    if (pid == 0) {
        if (fds[0] >= 0) close(fds[0]);   /* child keeps only the WRITE end */
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs, v0 = n_vacuous;
        current_test = name;
        fn();
        current_test = "(none)";
        CdChildCounters c;
        c.assertions = (int)(assertions - a0);
        c.failures   = (int)(failures - f0);
        c.setup_bugs = (int)(n_setup_bugs - s0);
        c.vacuous    = (int)(n_vacuous - v0);
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d vacuous\n", name, c.assertions, c.failures, c.setup_bugs,
               c.vacuous);
        fflush(stdout);
        fflush(stderr);
        if (fds[1] >= 0) {
            ssize_t w = write(fds[1], &c, sizeof c);
            (void)w;
            close(fds[1]);
        }
        _Exit(c.failures    > 0 ? CHILD_FAILURES
               : c.vacuous   > 0 ? CHILD_VACUOUS
               : c.setup_bugs > 0 ? CHILD_SETUPBUG : CHILD_OK);
    }

    if (fds[1] >= 0) close(fds[1]);
    CdChildCounters c;
    memset(&c, 0, sizeof c);
    if (fds[0] >= 0) {
        size_t got = 0;
        while (got < sizeof c) {
            ssize_t r = read(fds[0], ((char *)&c) + got, sizeof c - got);
            if (r <= 0) break;
            got += (size_t)r;
        }
        close(fds[0]);
    }
    assertions  += c.assertions;
    failures    += c.failures;
    n_setup_bugs += c.setup_bugs;
    n_vacuous   += c.vacuous;

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        case_record(name, ST_CRASH, c.assertions);
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        case_record(name, ST_CRASH, c.assertions);
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        case_record(name, ST_OK, c.assertions);
        printf("ok      %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        case_record(name, ST_RED, c.assertions);
        printf("FAILED  %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_VACUOUS) {
        n_tests_setup++;
        case_record(name, ST_VAC, c.assertions);
        printf("VACUOUS %s  <-- no reported failures, but a negative assertion "
               "could not be proven non-vacuous\n", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_tests_setup++;
        case_record(name, ST_SETUP, c.assertions);
        printf("SETUPBUG %s  <-- SETUP BUG (fixture could not be built)\n", name);
    } else {
        n_tests_crashed++;
        case_record(name, ST_CRASH, c.assertions);
        printf("CRASH   %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
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
        return 2;
    }
    /* CD_DEBUG=1 turns on the engine's own ability/executor trace so a red
       cost assertion can be attributed to the CONDITION, the per-unit
       arithmetic, the PRICE or the CHARGE without re-instrumenting anything. */
    if (getenv("CD_DEBUG")) rb_ability_debug_set(1);
    /* The ability log is the prompt-count control for section B. */
    rb_log_set_enabled(1);

    printf("--- A. per_card/wien_cost_mod_test.rs (PL!SP-pb1-010-R) ---\n");
    run("a_wien_low_energy_no_cost_modifier", a_wien_low_energy_no_cost_modifier);
    run("a_wien_high_energy_cost_modifier_applied", a_wien_high_energy_cost_modifier_applied);
    run("a_wien_cost_modifier_dynamic", a_wien_cost_modifier_dynamic);
    run("a_wien_cost_modifier_cleared_on_leave", a_wien_cost_modifier_cleared_on_leave);

    printf("--- B. per_card/wien_yell_count_test.rs (PL!SP-bp2-010-R+) ---\n");
    run("b_wien_with_partner_reduces_cheer_checks_by_8", b_wien_with_partner_reduces_cheer_checks_by_8);
    run("b_wien_alone_no_reduction", b_wien_alone_no_reduction);
    run("b_opponent_cheer_checks_unaffected", b_opponent_cheer_checks_unaffected);
    run("b_q117_second_wien_copy_counts_as_other_member", b_q117_second_wien_copy_counts_as_other_member);
    run("b_blade_gained_after_live_start_does_not_reduce_yell_checks", b_blade_gained_after_live_start_does_not_reduce_yell_checks);

    printf("--- C. per_card/seven_energy_lead_stage_cost_two_bonus_test.rs (PL!SP-bp7-002-R) ---\n");
    run("c_condition_met_applies_plus2_once", c_condition_met_applies_plus2_once);
    run("c_more_energy_high_margin_applies_once", c_more_energy_high_margin_applies_once);
    run("c_equal_energy_denies_bonus", c_equal_energy_denies_bonus);
    run("c_less_energy_denies_bonus", c_less_energy_denies_bonus);
    run("c_below_seven_denies_bonus", c_below_seven_denies_bonus);
    run("c_mirror_only_more_energy_side_gets_bonus", c_mirror_only_more_energy_side_gets_bonus);
    run("c_mirror_equal_energy_neither_gets_bonus", c_mirror_equal_energy_neither_gets_bonus);
    run("c_modified_cost_reduces_baton_pass_energy", c_modified_cost_reduces_baton_pass_energy);
    run("c_baton_pass_fails_without_enough_energy", c_baton_pass_fails_without_enough_energy);
    run("c_recalculate_is_idempotent", c_recalculate_is_idempotent);
    run("c_plus2_turns_off_when_opponent_catches_up", c_plus2_turns_off_when_opponent_catches_up);
    run("c_baton_pass_needs_base_cost_without_bonus", c_baton_pass_needs_base_cost_without_bonus);
    run("c_gate_offers_discounted_baton_cost", c_gate_offers_discounted_baton_cost);
    run("c_gate_without_bonus_offers_base_cost", c_gate_without_bonus_offers_base_cost);
    run("c_baton_pays_discounted_energy", c_baton_pays_discounted_energy);
    run("c_own_play_cost_unaffected", c_own_play_cost_unaffected);

    printf("--- D. per_card/success_zone_score_six_stage_cost_three_bonus_test.rs (PL!-bp4-008-R) ---\n");
    run("d_below_threshold_no_cost_mod", d_below_threshold_no_cost_mod);
    run("d_at_threshold_has_cost_mod", d_at_threshold_has_cost_mod);
    run("d_above_threshold_has_cost_mod", d_above_threshold_has_cost_mod);
    run("d_dynamic_increase", d_dynamic_increase);
    run("d_dynamic_decrease", d_dynamic_decrease);
    run("d_removed_from_stage_clears_mod", d_removed_from_stage_clears_mod);
    run("d_not_on_stage_no_mod", d_not_on_stage_no_mod);
    run("d_play_cost_unaffected", d_play_cost_unaffected);
    run("d_two_copies_each_gets_mod", d_two_copies_each_gets_mod);
    run("d_non_stackable_per_card_not_shared", d_non_stackable_per_card_not_shared);
    run("d_baton_touch_uses_modified_cost", d_baton_touch_uses_modified_cost);
    run("d_baton_touch_base_cost_when_below_threshold", d_baton_touch_base_cost_when_below_threshold);

    printf("--- E. per_card/success_zone_stage_cost_deploy_energy_test.rs (PL!S-bp3-016-N) ---\n");
    run("e_success_zone_deploy_spends_base_plus_one", e_success_zone_deploy_spends_base_plus_one);

    printf("--- F. activation_discount/abilityless_member_play_cost_reduction_test.rs (PL!S-bp5-001-R+) ---\n");
    run("f_reduction_applies_to_no_ability_member", f_reduction_applies_to_no_ability_member);
    run("f_no_reduction_for_member_with_ability", f_no_reduction_for_member_with_ability);
    run("f_cost_four_pays_three_energy", f_cost_four_pays_three_energy);
    run("f_reduction_stacks", f_reduction_stacks);

    printf("--- G. activation_discount/distinct_group_activation_discount_q228_test.rs (PL!-bp5-004-R+) ---\n");
    run("g_four_unique_groups_cost_zero", g_four_unique_groups_cost_zero);
    run("g_zero_cost_all_rarities", g_zero_cost_all_rarities);
    run("g_one_group_cost_three_not_offered_at_two_energy", g_one_group_cost_three_not_offered_at_two_energy);
    run("g_reduction_charges_effective_cost", g_reduction_charges_effective_cost);
    run("g_selects_one_eligible_opponent_physical_instance", g_selects_one_eligible_opponent_physical_instance);

    printf("--- H. activation_discount/hand_card_count_play_cost_reduction_test.rs (LL-bp2-001-R+) ---\n");
    run("h_hand_card_cost_reduced_by_hand_count_minus_1", h_hand_card_cost_reduced_by_hand_count_minus_1);
    run("h_hand_card_alone_no_reduction", h_hand_card_alone_no_reduction);
    run("h_on_stage_card_does_not_reduce_other_cards_cost", h_on_stage_card_does_not_reduce_other_cards_cost);
    run("h_on_stage_card_ignores_hand_for_other_card_cost", h_on_stage_card_ignores_hand_for_other_card_cost);

    printf("--- I. activation_discount/hand_size_play_cost_reduction_zero_floor_test.rs (LL-bp2-001-R+) ---\n");
    run("i_hand_size_reduces_play_cost_with_zero_floor", i_hand_size_reduces_play_cost_with_zero_floor);

    printf("--- J. activation_discount/per_unit_member_cost_reduction_q249_test.rs (PL!HS-bp6-006-R+) ---\n");
    run("j_one_unit_member_reduces_play_cost_by_two", j_one_unit_member_reduces_play_cost_by_two);
    run("j_no_unit_members_leaves_play_cost_unchanged", j_no_unit_members_leaves_play_cost_unchanged);
    run("j_two_unit_members_reduce_play_cost_by_four", j_two_unit_members_reduce_play_cost_by_four);
    run("j_three_unit_members_reduce_cost_before_baton_touch_q249", j_three_unit_members_reduce_cost_before_baton_touch_q249);
    run("j_p_variant_receives_mirakura_cost_reduction_q249", j_p_variant_receives_mirakura_cost_reduction_q249);

    printf("--- K. activation_discount/play_time_three_named_member_discard_cost_ten_test.rs (LL-bp7-001-R+) ---\n");
    run("k_passive_cost_not_set_by_discard", k_passive_cost_not_set_by_discard);
    run("k_passive_cost_not_set_with_hand_cards", k_passive_cost_not_set_with_hand_cards);
    run("k_gameplay_accept_cost10_discards_hand", k_gameplay_accept_cost10_discards_hand);
    run("k_gameplay_decline_pays15_keeps_hand", k_gameplay_decline_pays15_keeps_hand);
    run("k_gameplay_no_hand_cards_no_choice_pays15", k_gameplay_no_hand_cards_no_choice_pays15);
    run("k_waitroom_fodder_cannot_enable_ten_energy_play", k_waitroom_fodder_cannot_enable_ten_energy_play);
    run("k_accept_with_ten_energy_reaches_stage", k_accept_with_ten_energy_reaches_stage);
    run("k_gameplay_incomplete_hand_no_choice", k_gameplay_incomplete_hand_no_choice);
    run("k_second_copy_can_be_used_as_one_fodder", k_second_copy_can_be_used_as_one_fodder);
    run("k_one_card_cannot_cover_two_slots", k_one_card_cannot_cover_two_slots);
    run("k_optimal_assignment_with_multi_name", k_optimal_assignment_with_multi_name);
    run("k_played_card_not_counted_as_fodder", k_played_card_not_counted_as_fodder);
    run("k_accept_with_ten_energy_pays_ten", k_accept_with_ten_energy_pays_ten);
    run("k_non_member_dual_name_does_not_count", k_non_member_dual_name_does_not_count);
    run("k_with_extra_unrelated_cards_still_offers_choice", k_with_extra_unrelated_cards_still_offers_choice);
    run("k_many_duplicates_still_finds_assignment", k_many_duplicates_still_finds_assignment);
    run("k_combinatorial_variant_fuzz", k_combinatorial_variant_fuzz);
    run("k_cost_cleared_after_play_and_second_play_costs_15", k_cost_cleared_after_play_and_second_play_costs_15);
    run("k_stage_member_not_counted_as_hand_fodder", k_stage_member_not_counted_as_hand_fodder);

    printf("--- L. activation_discount/shuffle_waitroom_members_optional_play_discount_test.rs (PL!N-bp7-011-R+) ---\n");
    run("l_accept_reduces_cost_by_2", l_accept_reduces_cost_by_2);
    run("l_decline_pays_full_cost", l_decline_pays_full_cost);
    run("l_no_waitroom_members_still_offers_choice", l_no_waitroom_members_still_offers_choice);

    printf("--- M. activation_discount/success_zone_group_member_play_discount_test.rs (PL!N-sd2-003-SD2) ---\n");
    run("m_hand_cost_reduced_with_nijigasaki_live_in_success_zone", m_hand_cost_reduced_with_nijigasaki_live_in_success_zone);
    run("m_hand_cost_full_without_success_zone_card", m_hand_cost_full_without_success_zone_card);
    run("m_non_nijigasaki_live_does_not_reduce_cost", m_non_nijigasaki_live_does_not_reduce_cost);

    printf("--- N. activation_discount/success_zone_high_cost_group_deploy_discount_test.rs (PL!-bp6-019-L) ---\n");
    run("n_reduces_cost_seventeen_group_member_by_two", n_reduces_cost_seventeen_group_member_by_two);
    run("n_does_not_reduce_low_cost_group_member", n_does_not_reduce_low_cost_group_member);
    run("n_does_not_reduce_wrong_group_member", n_does_not_reduce_wrong_group_member);
    run("n_absent_after_source_removed_before_play", n_absent_after_source_removed_before_play);

    rb_unload();
    /* ------------------------------------------------------------------
     * POST-HOC VACUITY AUDIT.
     *
     * Every negative in this file is non-vacuous ONLY IF the paired positive
     * it leans on actually passed. A negative that goes green while its
     * positive is RED has proved nothing -- it is the "three of four tests
     * passed while the engine did literally nothing" failure, and it is the
     * single most valuable thing this suite can catch about ITSELF.
     *
     * The assertions are untouched. This block only RECLASSIFIES the already
     * reported result of a green case whose support is red, so the summary
     * cannot claim a passing negative that was never earned.
     * ------------------------------------------------------------------ */
    static const struct { const char *negative; const char *positive; } vac_deps[] = {
        /* section F -- the no-ability filter, positive = F1 */
        { "f_no_reduction_for_member_with_ability", "f_reduction_applies_to_no_ability_member" },
        /* section J -- the みらくらぱーく！ filter, positive = J1 */
        { "j_no_unit_members_leaves_play_cost_unchanged", "j_one_unit_member_reduces_play_cost_by_two" },
        /* section M -- the 虹ヶ咲 success-zone filter, positive = M1 */
        { "m_hand_cost_full_without_success_zone_card", "m_hand_cost_reduced_with_nijigasaki_live_in_success_zone" },
        { "m_non_nijigasaki_live_does_not_reduce_cost",   "m_hand_cost_reduced_with_nijigasaki_live_in_success_zone" },
        /* section N -- Music S.T.A.R.T!!, positive = N1 */
        { "n_does_not_reduce_low_cost_group_member",      "n_reduces_cost_seventeen_group_member_by_two" },
        { "n_does_not_reduce_wrong_group_member",         "n_reduces_cost_seventeen_group_member_by_two" },
        { "n_absent_after_source_removed_before_play",    "n_reduces_cost_seventeen_group_member_by_two" },
        /* section H/I -- the hand-count reduction, positive = H1 */
        { "h_hand_card_alone_no_reduction",               "h_hand_card_cost_reduced_by_hand_count_minus_1" },
        { "h_on_stage_card_does_not_reduce_other_cards_cost", "h_hand_card_cost_reduced_by_hand_count_minus_1" },
        { "h_on_stage_card_ignores_hand_for_other_card_cost", "h_hand_card_cost_reduced_by_hand_count_minus_1" },
        /* section C -- the >= 7 gate, positive = C1 */
        { "c_below_seven_denies_bonus",                   "c_condition_met_applies_plus2_once" },
        { "c_baton_pass_needs_base_cost_without_bonus",   "c_condition_met_applies_plus2_once" },
        /* section D -- the score >= 6 gate, positive = D2 */
        { "d_below_threshold_no_cost_mod",                "d_at_threshold_has_cost_mod" },
        { "d_baton_touch_base_cost_when_below_threshold", "d_at_threshold_has_cost_mod" },
        { "d_removed_from_stage_clears_mod",             "d_at_threshold_has_cost_mod" },
        { "d_dynamic_decrease",                           "d_dynamic_increase" },
        /* section K -- the named-discard cost, positive = K3 */
        { "k_gameplay_decline_pays15_keeps_hand",         "k_gameplay_accept_cost10_discards_hand" },
        { "k_accept_with_ten_energy_pays_ten",            "k_gameplay_accept_cost10_discards_hand" },
        { "k_passive_cost_not_set_by_discard",            "k_gameplay_accept_cost10_discards_hand" },
        { "k_passive_cost_not_set_with_hand_cards",       "k_gameplay_accept_cost10_discards_hand" },
        /* section B -- the partner gate, positive = B1 */
        { "b_wien_alone_no_reduction",                    "b_wien_with_partner_reduces_cheer_checks_by_8" },
        { "b_opponent_cheer_checks_unaffected",           "b_wien_with_partner_reduces_cheer_checks_by_8" },
        { "b_q117_second_wien_copy_counts_as_other_member", "b_wien_with_partner_reduces_cheer_checks_by_8" },
        { "b_blade_gained_after_live_start_does_not_reduce_yell_checks", "b_wien_with_partner_reduces_cheer_checks_by_8" },
        /* section A -- the >= 10 energy gate, positive = A2 */
        { "a_wien_low_energy_no_cost_modifier",           "a_wien_high_energy_cost_modifier_applied" },
        /* section G -- the per-group activation cost, positive = G4 */
        { "g_four_unique_groups_cost_zero",               "g_reduction_charges_effective_cost" },
        { "g_zero_cost_all_rarities",                     "g_reduction_charges_effective_cost" },
    };
    int n_vac_cases = 0, n_content_free = 0;
    for (size_t i = 0; i < sizeof vac_deps / sizeof vac_deps[0]; i++) {
        int pos = case_status(vac_deps[i].positive);
        int neg = case_status(vac_deps[i].negative);
        if (pos == ST_OK && neg == ST_OK) continue;   /* genuinely earned */
        if (pos == ST_RED || pos == ST_CRASH || pos == ST_SETUP) {
            printf("VACUOUS %s  <-- its PAIRED POSITIVE %s is RED, so this "
                   "case's zeros are unearned (assertions above stand, result "
                   "reclassified)\n", vac_deps[i].negative, vac_deps[i].positive);
            n_vac_cases++;
            /* A VACUOUS CASE MUST STILL ASSERT SOMETHING REAL.  Reclassifying a
             * green negative whose positive is red is the right bookkeeping, but
             * it must not become a place where a case asserts nothing at all:
             * a vacuous case has to assert something real or be DELETED.  This is
             * the mechanical form of that rule, evaluated after the fact over the
             * counters the run already collected -- no assertion is relaxed,
             * skipped or retuned to reach it, and it names the offending case. */
            if (case_assertions(vac_deps[i].negative) == 0) {
                n_content_free++;
                assertions++;
                failures++;
                fprintf(stderr, "FAIL: premise: vacuous case [%s] ran ZERO "
                                "assertions of its own. A vacuous case must "
                                "assert something real or be DELETED, not merely "
                                "reclassified.\n", vac_deps[i].negative);
            }
        }
    }
    n_vacuous += n_vac_cases;

    printf("\n%d test cases: %d ok, %d failed, %d setup/vacuous, %d crashed\n",
           n_tests_ok + n_tests_failed + n_tests_setup + n_tests_crashed,
           n_tests_ok, n_tests_failed, n_tests_setup, n_tests_crashed);
    printf("%d assertions, %d failures, %d setup bugs, %d VACUOUS negatives "
           "(%d reclassified by the post-hoc audit)\n",
           assertions, failures, n_setup_bugs, n_vacuous, n_vac_cases);
    if (n_vacuous) {
        fprintf(stderr, "FAIL: %d negative assertion(s) could not be proven "
                        "non-vacuous (see the VACUOUS lines above)\n", n_vacuous);
        return 1;
    }
    if (n_tests_failed || n_tests_crashed || n_setup_bugs) {
        fprintf(stderr, "FAIL: %d test case(s) FAILED, %d CRASHED, %d SETUPBUG "
                        "-- see the FAIL lines above\n",
                n_tests_failed, n_tests_crashed, n_setup_bugs);
        return 1;
    }
    printf("ALL COST/DISCOUNT PARITY CHECKS PASSED\n");
    return 0;
}

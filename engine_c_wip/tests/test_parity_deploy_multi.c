/* test_parity_deploy_multi.c — parity suite for the two clusters whose
 * C-side coverage the coordinator listed as NOT ATTEMPTED:
 *
 *   engine/tests/test_modules/integration/multi_card/   (3 files / 18 tests)
 *   engine/tests/test_modules/effects/deploy/           (15 files / 61 tests)
 *
 * OWNERSHIP: this file is the only thing it creates. It edits no engine source
 * under src/, no other tests/test_*.c, and no Makefile.
 *
 * ── THE BOUNDARY, taken from the C files' BODIES (not their headers) ───────
 *
 * multi_card — 17 of 18 Rust tests are ALREADY covered:
 *   test_parity_integration.c:396 test_himeko_repositions_opponent_member
 *   test_parity_integration.c:423 test_himeko_gate_blocked
 *   test_parity_integration.c:455 test_seat_relative_energy_comparison
 *       (all FOUR 近未来ハッピーエンド arms live in one C function)
 *   test_parity_integration.c:522 test_mei_cost_gate
 *   test_parity_integration.c:553 test_rina_mutual_mill_from_p2_seat
 *       (both 璃奈 arms, incl. the no-cross-pollination one)
 *   test_parity_integration.c:1573 test_yell_blade_heart_gates
 *       (all THREE 皐月/韋恩 yell arms)
 *   test_parity_integration_extra.c:1710 d_t4_mei_mirror_standoff
 *   test_parity_integration_extra.c:1750 d_t5_mutual_mill (duplicate of :553)
 *   test_parity_integration_extra.c:1790 d_t6_ginko_accept
 *   test_parity_integration_extra.c:1849 d_t7_ginko_decline
 *   test_parity_integration_extra.c:1880 d_t8_hot_passion_empty_energy_deck
 *   test_parity_integration_extra.c:1899 d_t9_hot_passion_waited_flips_cmp
 *
 *   The ONE genuine gap is
 *     p2_owned_ginko_targets_p1_board_from_p2_seat
 *   test_parity_integration_extra.c:1705 SAYS it is "folded into §D:ginko", but
 *   d_t6 (:1813) and d_t7 (:1867) BOTH write `tg.state.p[0].stage[1] = gin` and
 *   `fire_trigger(&tg, 0, gin, ...)` — i.e. P1's seat only. The comment is a
 *   FALSE coverage claim. This file ports it for real. (§M1)
 *
 * deploy — 28 of 61 Rust tests are already covered by the 28 `a_*` functions
 * in test_parity_deploy_recover.c PART A (plus 2 more in
 * test_parity_deploy_move_missing.c), i.e. ~33 are NOT covered, NOT the "~5"
 * the survey claimed. This file takes 19 of them, chosen to bracket the
 * cluster's CORE defect (move_cards with destination=empty_area) and to give
 * every negative assertion a paired positive on the same board. The 14
 * uncovered Rust tests this file did NOT take are listed by name in main().
 *
 * ── WHAT IS ASSERTED ─────────────────────────────────────────────────────
 * Every assertion below is a translated Rust expectation, never a description
 * of current C behaviour. A red assertion is a finding; it is reported, not
 * weakened. See the EXPECTED_GAP notes: an EXPECTED_GAP is an assertion C
 * STRUCTURALLY cannot make (no flat destination field, no any_order producer),
 * and it is counted apart from passes — never as a pass.
 *
 * ── HARNESS LANDMINES this file respects ──────────────────────────────────
 *  * `rb_card_string()` takes a STRING-TABLE INDEX. Identity is pinned with
 *    `rb_card_no_eq(cid, "PL!...")` only; the name is never used as a key.
 *  * Heart colour indices are DERIVED with rb_parse_heart_color() and the whole
 *    mapping is printed at startup, so a remap (the known test_get_heart_modifier
 *    5 -> ORANGE trap) is visible rather than silently wrong.
 *  * rb_calc_stage_hearts RECOMPUTES, so nothing pre-seeds stage_hearts[].
 *  * test_add_to_stage / test_set_opp_stage are FIXED-INDEX by design;
 *    test_place_under APPENDS. Both are used the way the Rust twin does.
 *  * `test_new_id` allocates from a process-global monotonic pool nothing
 *    resets, so rb_card_copy_pool_reset() runs per case.
 *  * RbChoice.zone for the waitroom is "discard".
 *  * Every negative assertion is paired with a positive on the SAME board, or
 *    gated on a prompt that was counted while still ON SCREEN. An empty
 *    candidate set makes every "must NOT happen" assertion true for free.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* ── tally in shared pages, so a child that dies mid-assertion still leaves
 *    the parent holding the real number (a fork-only counter cannot) ─────── */
struct RunTally { int assertions, failures, setup_bugs, gaps; };
static struct RunTally *g_tally;
static const char *current_test = "(none)";

static int n_ok, n_failed, n_crashed;

#define TALLY g_tally
#define CHECK(cond, ...) do { \
    TALLY->assertions++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL: "); fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        TALLY->failures++; \
    } \
} while (0)
#define CHECK_EQ(actual, expected, ...) do { \
    TALLY->assertions++; \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: "); fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, " (got %d expected %d)  [%s]\n", a_, e_, current_test); \
        TALLY->failures++; \
    } \
} while (0)
/* An assertion C cannot structurally make. Counted and printed; NEVER a pass. */
#define EXPECTED_GAP(why, cond) do { \
    TALLY->assertions++; TALLY->gaps++; \
    if (cond) fprintf(stdout, "gap-closed: %s  [%s]\n", (why), current_test); \
    else      fprintf(stdout, "GAP: %s  [%s]\n", (why), current_test); \
} while (0)

/* ── areas (engine/src/game/zones.rs MemberArea) ───────────────────────── */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── card numbers, EVERY ONE verified against cards/cards.json read as UTF-8 ──
 *   (Get-Content -Raw mangles the non-ASCII and produces false "missing") ──
 *
 * multi_card
 *   GINKO_035  PL!HS-PR-035-PR 百生吟子   cost 11 unit スリーズブーケ
 *   WAIT_A/B/C PL!-sd1-001/003/004-SD   members (no group gate on the pick)
 *   FILLER_010 PL!-sd1-010-SD 高坂穂乃果 cost 4 blade 1  (gate passes, <=3)
 *
 * deploy — q268 (三船栞子 起動 / ターン1回: energy under self, then deploy a
 * cost<=2 『虹ヶ咲』 member from the waitroom into an EMPTY area in WAIT):
 *   SHIORIKO   PL!N-bp7-010-R 三船栞子 cost 9 R3BIRTH
 *   AYUMU_2    PL!N-bp7-013-N 上原歩夢 cost 2 A・ZU・NA  -> deployable
 *   SHIO_2     PL!N-sd2-022-SD2 三船栞子 cost 2 R3BIRTH -> deployable
 *   AYUMU_4    PL!N-bp1-013-PR 上原歩夢 cost 4 A・ZU・NA -> cost gate blocks
 *   ERI_2      PL!-sd1-002-SD  絢瀬絵里 cost 2 BiBi     -> NOT 虹ヶ咲
 *   FILLER_010 PL!-sd1-010-SD 高坂穂乃果 cost 4 blade 1
 *
 * deploy — bloom (PL!HS-bp2-019-L ライブ開始時, three heart patterns):
 *   BLOOM      PL!HS-bp2-019-L, need_heart {heart01:1,heart04:1,heart05:1,heart0:1}
 *   SAYAKA     PL!HS-bp1-002-R 村野さやか cost 11 DOLLCHESTRA (蓮ノ空)
 *
 * deploy — 桜坂しずく / 宮下愛 (登場, optional EE -> named cost<=4 from hand):
 *   SHIZUKU_R  PL!N-pb1-015-R cost 13 ; SHIZUKU_N PL!N-bp1-015-N cost 4
 *   MIYA_R     PL!N-pb1-017-R cost 7  ; MIYA_N    PL!N-bp1-017-N cost 4
 *
 * deploy — 西木野真姫 (登場 IF baton-touched from a LOWER-cost member):
 *   MAKI       PL!-PR-015-PR cost 17
 *   NANAKI     PL!SP-sd1-019-SD cost 2   -> the "cheap member"
 *   HOSHIZORA  PL!-sd1-014-SD cost 9    -> "too expensive" for the <=4 filter
 *
 * deploy — 津島善子 桂城泉 etc.
 *   YOSHI      PL!S-bp3-006-P 津島善子 cost 13 GuiltyKiss 起動/センター/ターン1回
 *   RIKO_4     PL!S-bp2-002-R 桜内梨子 cost 4  GuiltyKiss  (the removal target)
 *   RUBY_2     PL!S-bp2-009-R 黒澤ルビィ cost 2  CYaRon
 *   IZUMI_16   PL!HS-bp6-016-R 桂城泉 cost 9  EdelNote 起動/ターン1回 4E
 *   KAMARU     PL!HS-bp2-004-R 夕霧綴理 cost 2  DOLLCHESTRA  -> deployable
 *   YOSHIKO    PL!S-bp2-006-R 津島善子 cost 11 GuiltyKiss 登場 4E, <=2 members
 *                                                             TOTAL cost <= 4
 *   SD1_002    PL!-sd1-002-SD cost 2 ; SD1_005 PL!-sd1-005-SD cost 2
 *   SD1_008    PL!-sd1-008-SD cost 4
 *   FUYUMARI   PL!SP-pb1-011-R 鬼塚冬毬 cost 13 Liella! 登場
 *   SD1_006    PL!SP-sd1-006-SD 桜小路きな子 cost 2 Liella! 5yncri5e!
 *   SD1_008S   PL!SP-sd1-008-SD 若菜四季 cost 4 Liella! 5yncri5e!
 */
#define GINKO_035   "PL!HS-PR-035-PR"
#define WAIT_A      "PL!-sd1-001-SD"
#define WAIT_B      "PL!-sd1-003-SD"
#define WAIT_C      "PL!-sd1-004-SD"
#define FILLER_010  "PL!-sd1-010-SD"

#define SHIORIKO    "PL!N-bp7-010-R"
#define AYUMU_2     "PL!N-bp7-013-N"
#define SHIO_2      "PL!N-sd2-022-SD2"
#define AYUMU_4     "PL!N-bp1-013-PR"
#define ERI_2       "PL!-sd1-002-SD"

#define BLOOM       "PL!HS-bp2-019-L"
#define SAYAKA      "PL!HS-bp1-002-R"

#define SHIZUKU_R   "PL!N-pb1-015-R"
#define SHIZUKU_N   "PL!N-bp1-015-N"
#define MIYA_R      "PL!N-pb1-017-R"
#define MIYA_N      "PL!N-bp1-017-N"

#define MAKI        "PL!-PR-015-PR"
#define NANAKI      "PL!SP-sd1-019-SD"
#define HOSHIZORA   "PL!-sd1-014-SD"

#define YOSHI       "PL!S-bp3-006-P"
#define RIKO_4      "PL!S-bp2-002-R"
#define RUBY_2      "PL!S-bp2-009-R"
#define IZUMI_16    "PL!HS-bp6-016-R"
#define KAMARU      "PL!HS-bp2-004-R"
#define YOSHIKO     "PL!S-bp2-006-R"
#define SD1_002     "PL!-sd1-002-SD"
#define SD1_005     "PL!-sd1-005-SD"
#define SD1_008     "PL!-sd1-008-SD"
#define FUYUMARI    "PL!SP-pb1-011-R"
#define SD1_006     "PL!SP-sd1-006-SD"
#define SD1_008S    "PL!SP-sd1-008-SD"

/* The U+FF0B fullwidth-plus print of a joint card, as explicit UTF-8 bytes. */
#define PLUS "\xef\xbc\x8b"

/* ── card DB. `src` is the in-tree layout; an isolated out-of-tree build
 *    resolves the blobs through ../cards/build, so both are tried. ───────── */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* ── identity. rb_card_no_eq normalises + <-> U+FF0B. The NAME is never a
 *    key: a test_new_id copy can carry the wrong Card.name, which would make
 *    every distinct-NAME gate silently unreachable. ──────────────────────── */
static int pin_id(int cid, const char *want)
{
    TALLY->assertions++;
    if (cid < 0 || !rb_card_no_eq(cid, want)) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', got id=%d\n", want, cid);
        TALLY->failures++;
        return 0;
    }
    return 1;
}

/* A rotating ring, never a single shared static buffer: one buffer called 2-3
 * times inside one printf prints the LAST card every time — a false-red
 * generator. __attribute__((unused)) because a diagnostic that no assertion
 * happens to need today is still the right tool to reach for. */
static const char *no_of(int cid) __attribute__((unused));
static const char *no_of(int cid)
{
    static char ring[8][80];
    static int  at = 0;
    char *b = ring[at];
    at = (at + 1) & 7;
    if (cid < 0) { snprintf(b, 80, "<none>"); return b; }
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c) && c.card_no_idx) {
        snprintf(b, 80, "%s#%d", rb_card_string(c.card_no_idx), cid);
        rb_free_card(&c);
    } else {
        snprintf(b, 80, "id%d", cid);
    }
    return b;
}

static int printed_cost(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}
static int printed_blade(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.blade; rb_free_card(&c); }
    return v;
}
static int printed_score(int cid)
{
    Card c; int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}
static int is_live_card(int cid) { return cid >= 0 ? rb_card_is_live(cid) : -1; }

/* Printed group / unit name, for the premises the Rust twins assert about
 * 虹ヶ咲 / 蓮ノ空 / Liella! membership. __attribute__((unused)): the
 * group-membership claims are proved BEHAVIOURALLY (the paired positive in D2
 * and D18), never by comparing bytes, because the engine's group name for one
 * print is known to differ from its cards.json spelling. */
static const char *group_of(int cid) __attribute__((unused));
static const char *group_of(int cid) { return rb_card_group_name(cid); }

/* ══════════════════════════════════════════════════════════════════════════
 * board helpers
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust `game.new_id(no)` allocates the NEXT DISTINCT pool slot, so two calls
 * with the same card_no are two different instances. Every Rust game.id(..)
 * maps to new_id() here unless the test wants the shared template. */
static int mid(TestGame *tg, const char *no) { return test_new_id(tg, no); }

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(TestGame *tg, int cid)    { return bag_has(&tg->state.p[0].hand, cid); }
static int stage_has(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}
static int stage_area_of(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return i;
    return -1;
}
static int energy_under(TestGame *tg, int pl, int area)
{
    return tg->state.p[pl].under_cards[area].n;
}
static const char *orientation(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}
static int is_wait(TestGame *tg, int cid)
{
    const char *o = orientation(tg, cid);
    return o && !strcmp(o, "wait");
}
static int need_heart_of(TestGame *tg, int cid, const char *color)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, (int)rb_parse_heart_color(color));
}
static int turn1_used(TestGame *tg, int cid) { return rb_ability_uses_used(&tg->state, cid, 0); }

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0; P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; }
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->under_cards[i].n = 0;
}
static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

/* ── choice helpers ─────────────────────────────────────────────────────── */
static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static const char *pending_type(TestGame *tg) { return test_pending_choice_type(tg); }
static int pending_is(TestGame *tg, const char *want)
{
    const char *t = pending_type(tg);
    return t && !strcmp(t, want);
}
static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}
static void pick_n(TestGame *tg, const int *idx, int n) __attribute__((unused));
static void pick_n(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg)) return;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}
static void skip_choice(TestGame *tg)
{
    if (!pending(tg)) return;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}
static int drain_pick0(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { pick(tg, 0); n++; }
    return n;
}
static int drain_skip(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { skip_choice(tg); n++; }
    return n;
}
/* SelectAutoAbility -> index 0 (proceed); anything else -> empty (decline).
 * Mirrors Rust drain_auto_ability_choices. */
static int drain_auto(TestGame *tg)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < 32) {
        if (pending_is(tg, "SelectAutoAbility")) pick(tg, 0); else skip_choice(tg);
        n++;
    }
    return n;
}
/* Answer what cannot be declined, decline what can. A mandatory prompt left
 * parked mid-ability produces exactly the absence the next assertion checks,
 * so nothing would fail. __attribute__((unused)): present so a test that needs
 * the "answer everything" drain has it in the same idiom. */
static int drain_all(TestGame *tg) __attribute__((unused));
static int drain_all(TestGame *tg)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < 32) { pick(tg, 0); n++; }
    return n;
}

/* play_to_stage with the play-time cost hook answered `accept` times, exactly
 * like test_parity_deploy_recover.c's play_to_stage(). */
static int play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, accept);
        if (tg->state.p[0].stage[area] == cid) break;
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

/* Rust helpers::fire_trigger for one SEAT. The C engine has no single-ability
 * "fire" entry point, so this queues every ability that seat owns for the
 * trigger token (rb_queue_trigger_abilities) and drains — a superset of Rust's
 * trigger_auto_ability(card only). Every fixture board below holds exactly one
 * card printing the trigger, so the superset is the intended set. */
static void fire_trigger(TestGame *tg, int pl, int cid, const char *trigger)
{
    rb_queue_trigger_abilities(&tg->state, pl, trigger);
    tg->state.activating_card = cid;
    rb_drain_ability_queue(&tg->state);
}

static void advance_to_live_set(TestGame *tg) { for (int i = 0; i < 5; i++) test_pass(tg); }
static void advance_to_live_start(TestGame *tg) { test_pass(tg); test_pass(tg); }

/* ══════════════════════════════════════════════════════════════════════════
 * DIAGNOSTIC PROBE — env-gated, and the ONLY thing here that is not a
 * translated Rust assertion. It exists so a red can be classified as a TEST
 * BUG or an ENGINE BUG from evidence instead of from a guess.
 *
 *   RB_TDM_DUMP=abilities <card_no>   print every ability the card prints
 *   RB_TDM_DUMP=zone                  print both seats' zones on a scratch board
 *   RB_TDM_DUMP=activate <card_no>    build a minimal board, activate, and
 *                                     trace EVERY prompt with its zone /
 *                                     count / cost_total / filtered_indices
 *                                     before answering it
 *
 * The activate trace is the important one: it shows what the engine actually
 * asked and in what order, which is what separates "my harness answered the
 * wrong prompt" from "the engine never asked".
 * ══════════════════════════════════════════════════════════════════════════ */

static void dump_zone(const char *tag, TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        printf("  %s p%d hand=%d waitroom=%d deck=%d live=%d energy=%d active=%d\n",
               tag, pl + 1, P->hand.n, P->discard.n, P->deck.n, P->live.n,
               P->energy.n, P->energy_active);
        printf("      stage=[%d,%d,%d] under=[%d,%d,%d]\n",
               P->stage[0], P->stage[1], P->stage[2],
               P->under_cards[0].n, P->under_cards[1].n, P->under_cards[2].n);
        for (int i = 0; i < P->hand.n; i++)   printf("      hand[%d]=%d\n",   i, P->hand.cards[i]);
        for (int i = 0; i < P->discard.n; i++) printf("      waitrm[%d]=%d\n", i, P->discard.cards[i]);
    }
}

static void dump_abilities(const char *no)
{
    int cid = rb_find_card_by_no(no);
    if (cid < 0) { printf("  %s NOT FOUND\n", no); return; }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { printf("  decode failed\n"); return; }
    printf("  %s id=%d name=%s cost=%d blade=%d\n",
           no, cid, c.name ? c.name : "?", c.cost, c.blade);
    printf("  group_name='%s'\n", rb_card_group_name(cid) ? rb_card_group_name(cid) : "(null)");
    rb_free_card(&c);
    int n = rb_card_num_abilities((uint32_t)cid);
    printf("  %d ability(ies)\n", n);
    for (int a = 0; a < n; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        printf("    ab#%d trigger='%s' cost='%s'\n      effect='%s'\n",
               a, ab.triggers ? ab.triggers : "(null)",
               (ab.cost && ab.cost->text) ? ab.cost->text : "(null)",
               (ab.effect && ab.effect->text) ? ab.effect->text : "(null)");
        rb_free_ability(&ab);
    }
}

/* Trace every prompt: this is what separates a harness bug from an engine bug. */
static int trace_activate(const char *no, int pl, int area, int energy)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int cid = test_new_id(&tg, no);
    int filler = test_new_id(&tg, FILLER_010);
    if (cid < 0 || filler < 0) { printf("  setup failed\n"); return 1; }
    printf("=== trace_activate %s (id=%d) on p%d area=%d energy=%d\n", no, cid, pl + 1, area, energy);
    tg.state.p[pl].stage[area] = cid;
    test_give_energy(&tg, energy);
    fill_decks(&tg, filler, 30);
    dump_zone("before:", &tg);
    int r = test_activate_ability_for(&tg, pl, cid);
    printf("  test_activate_ability_for -> %d\n", r);
    int guard = 0;
    while (pending(&tg) && guard++ < 30) {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        printf("  PROMPT #%d kind=%s zone='%s' card_type='%s' count=%d allow_skip=%d "
               "cost_limit=%d%s%s cost_total=%d%s%s n_filtered=%d target='%s' desc='%s'\n",
               guard,
               ch ? test_pending_choice_type(&tg) : "(null)",
               ch ? ch->zone : "", ch ? ch->card_type : "",
               ch ? ch->count : -1, ch ? ch->allow_skip : -1,
               ch ? ch->cost_limit : 0, ch ? ch->cost_limit_op : "",
               ch ? ch->cost_limit_op : "", ch ? ch->cost_total : -1,
               ch ? ch->cost_total_op : "", ch ? ch->cost_total_op : "",
               ch ? ch->n_filtered_indices : -1,
               ch ? ch->target : "", ch ? ch->description : "");
        if (ch) {
            printf("      filtered_indices:");
            for (int i = 0; i < ch->n_filtered_indices; i++) printf(" %d", ch->filtered_indices[i]);
            printf("\n");
        }
        if (ch && ch->kind == RB_CHOICE_SELECT_TARGET) { const int one[1] = { 1 }; rb_resume_with_choice_indices(&tg.state, one, 1); }
        else pick(&tg, 0);
    }
    dump_zone("after:", &tg);
    return 0;
}

static int maybe_probe(const char *spec)
{
    if (!spec || !*spec) return 0;
    if (!strncmp(spec, "abilities", 9)) {
        const char *p = spec + 9;
        while (*p == ' ') p++;
        if (!*p) { printf("usage: RB_TDM_DUMP=abilities <card_no>\n"); return 1; }
        dump_abilities(p);
        return 1;
    }
    if (!strncmp(spec, "zone", 4)) {
        static TestGame tg;
        rb_card_copy_pool_reset();
        test_game_new(&tg);
        clear_p1(&tg);
        int f = test_new_id(&tg, FILLER_010);
        test_add_to_hand(&tg, f);
        tg.state.p[0].stage[1] = f;
        dump_zone("scratch:", &tg);
        return 1;
    }
    if (!strncmp(spec, "activate", 8)) {
        const char *p = spec + 8;
        int pl = 0, area = 1, energy = 3;
        while (*p == ' ') p++;
        if (strncmp(p, "no=", 3)) { printf("usage: RB_TDM_DUMP=activate no=<card_no> [pl] [area] [energy]\n"); return 1; }
        p += 3;
        const char *end = strchr(p, ' ');
        char no[64];
        size_t n = end ? (size_t)(end - p) : strlen(p);
        if (n >= sizeof no) n = sizeof no - 1;
        memcpy(no, p, n); no[n] = 0;
        if (end) pl   = atoi(end + 1);
        char *e2 = strchr(end ? end + 1 : "", ' ');
        if (e2) { area = atoi(e2 + 1); e2 = strchr(e2 + 1, ' '); if (e2) energy = atoi(e2 + 1); }
        return trace_activate(no, pl, area, energy);
    }
    fprintf(stderr, "unknown RB_TDM_DUMP spec '%s'\n", spec);
    return 1;
}

/* ══════════════════════════════════════════════════════════════════════════
 * §M  multi_card — the ONE uncovered test.
 *
 * p2_owned_ginko_targets_p1_board_from_p2_seat
 *   (round5_seat_relative_live_success_and_live_start_cross_player_test.rs:605)
 *
 * 吟子 ab#0 (登場): 「相手の控え室にあるメンバーカードを3枚選び、相手のデッキの
 *  下に好きな順番で置いてもよい。そうした場合、相手のステージにいる元々持つ
 *  ブレードの数が3つ以下のメンバー1人をウェイトにする。」
 *
 * P2 owns 吟子 and fires from P2's seat: 「相手」 must resolve to P1, so P1's
 * three waitroom members go under P1's deck and P1's blade-1 member is rested.
 * The P1-owned twin is test_parity_integration_extra.c d_t6/d_t7; the P2 seat
 * is what this file adds.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Mirrors drain_ginko(accept=true): the 「置いてもよい」 gate IS the
 * SelectCard's allow_skip — no separate optional prompt — so "accept" is every
 * index on the SelectCard and "decline" is an empty answer. A pay/skip
 * SelectTarget may also appear, and index 0 accepts it. */
static int drain_ginko_accept(TestGame *tg)
{
    int prompts = 0, guard = 0;
    while (pending(tg) && guard++ < 12) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        prompts++;
        if (ch && ch->kind == RB_CHOICE_SELECT_CARD) {
            int n = ch->count > 0 ? ch->count : 1;
            if (n > 8) n = 8;
            int idx[8];
            for (int i = 0; i < n; i++) idx[i] = i;
            rb_resume_with_choice_indices(&tg->state, idx, n);
        } else if (ch && ch->kind == RB_CHOICE_SELECT_TARGET) {
            const int one[1] = { 1 };   /* pay */
            rb_resume_with_choice_indices(&tg->state, one, 1);
        } else {
            pick(tg, 0);
        }
    }
    return prompts;
}

static void m1_p2_owned_ginko_targets_p1_board_from_p2_seat(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = mid(&tg, FILLER_010);
    int a = mid(&tg, WAIT_A), b = mid(&tg, WAIT_B), c = mid(&tg, WAIT_C);
    int gin = mid(&tg, GINKO_035);
    if (!pin_id(gin, GINKO_035)) { TALLY->setup_bugs++; return; }
    if (!pin_id(filler, FILLER_010)) { TALLY->setup_bugs++; return; }

    /* Premises read from the card pool, as the Rust twin's own comments claim. */
    CHECK_EQ(printed_blade(filler), 1,
             "M1 premise: the P1 stage member really prints <= 3 blades (rs:514 / :617)");
    CHECK(a != b && b != c && c != a,
          "M1 premise: the three waitroom picks are DISTINCT card instances");

    /* P1's board: three members in the waitroom, one blade-1 member on stage. */
    {   RbPlayer *P = &tg.state.p[0];
        P->discard.cards[P->discard.n++] = a;
        P->discard.cards[P->discard.n++] = b;
        P->discard.cards[P->discard.n++] = c;
    }
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    fill_decks(&tg, filler, 30);
    int p1_deck_before = tg.state.p[0].deck.n;

    /* 吟子 sits on P2's board and fires from P2's seat. */
    tg.state.p[1].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[1] = gin;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;

    fire_trigger(&tg, 1, gin, RB_TSTR_DEBUT);
    int prompts = drain_ginko_accept(&tg);
    drain_auto(&tg);

    /* NON-VACUITY: the whole test is 「相手」 = P1. With a broken seat the
     * pick would land on P2's empty waitroom and the count would be 0, so the
     * prompt count is the guard. */
    CHECK(prompts > 0,
          "M1 control: 吟子's 登場 really opened a waitroom prompt (a zero here "
          "would make every following assertion vacuous)");

    int placed = 0, leaked_to_p2 = 0;
    const int ids[3] = { a, b, c };
    for (int i = 0; i < 3; i++) {
        if (waitroom_has(&tg, ids[i])) continue;
        if (test_zone_has_id(&tg, 0, "deck", ids[i])) placed++;
        if (test_zone_has_id(&tg, 1, "deck", ids[i])) leaked_to_p2++;
    }
    CHECK_EQ(placed, 3,
             "M1: a P2-owned 吟子 moves all three of P1's waitroom members to P1's OWN deck bottom (rs:625-631)");
    CHECK_EQ(leaked_to_p2, 0,
             "M1: 「相手のデッキ」 from P2's seat means P1's deck — nothing crosses to P2's (rs:625-631)");
    CHECK_EQ(tg.state.p[0].deck.n, p1_deck_before + 3,
             "M1: P1's deck grew by exactly the 3 placed cards (rs:632)");
    CHECK_EQ(tg.state.p[1].deck.n, 30,
             "M1: P2's deck is untouched — the effect only writes to the opponent's deck (rs:632)");
    CHECK_EQ(is_wait(&tg, filler), 1,
             "M1: a P2-owned 吟子 rests P1's original-blade<=3 member (rs:633-637)");
    CHECK_EQ(tg.state.p[1].stage[1], gin,
             "M1: 吟子 herself stays on P2's stage — a position change to a "
             "WAITED member never leaves the stage (rs:79-82 of the twin)");
}

/* The P1-owned twin, run here ONLY as the paired positive that proves the P2
 * arm above is a real seat difference and not a constant. It asserts nothing
 * new: test_parity_integration_extra.c d_t6 owns the accept path. */
static void m1b_p1_owned_ginko_is_the_paired_positive(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = mid(&tg, FILLER_010);
    int a = mid(&tg, WAIT_A), b = mid(&tg, WAIT_B), c = mid(&tg, WAIT_C);
    int gin = mid(&tg, GINKO_035);
    if (!pin_id(gin, GINKO_035)) { TALLY->setup_bugs++; return; }

    /* Mirror of M1 with the seats swapped: the OPPONENT of P1 is P2. */
    {   RbPlayer *Q = &tg.state.p[1];
        Q->discard.cards[Q->discard.n++] = a;
        Q->discard.cards[Q->discard.n++] = b;
        Q->discard.cards[Q->discard.n++] = c;
    }
    tg.state.p[1].stage[0] = filler;
    tg.state.p[1].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[1].stage[2] = RB_EMPTY_SLOT;
    fill_decks(&tg, filler, 30);
    int p2_deck_before = tg.state.p[1].deck.n;

    tg.state.p[0].stage[1] = gin;

    fire_trigger(&tg, 0, gin, RB_TSTR_DEBUT);
    int prompts = drain_ginko_accept(&tg);
    drain_auto(&tg);

    CHECK(prompts > 0, "M1b control: the P1-owned copy really opened a prompt");
    int placed = 0;
    const int ids[3] = { a, b, c };
    for (int i = 0; i < 3; i++)
        if (!test_zone_has_id(&tg, 1, "discard", ids[i]) &&
            test_zone_has_id(&tg, 1, "deck", ids[i])) placed++;
    CHECK_EQ(placed, 3,
             "M1b: the mirror arm really moves all three of P2's members to P2's deck — "
             "this is the paired positive that gives M1 its meaning");
    CHECK_EQ(tg.state.p[1].deck.n, p2_deck_before + 3, "M1b: P2's deck grew by 3");
    CHECK_EQ(is_wait(&tg, filler), 1,
             "M1b: a P1-owned 吟子 rests P2's blade<=3 member");
    CHECK_EQ(tg.state.p[0].deck.n, 30,
             "M1b: P1's own deck is untouched — the two arms really differ by seat");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D1  deploy / under_energy_cost_waited_empty_area_deploy_q268_test.rs
 *
 * This file's CORE. The 起動 is
 *   「エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：自分の控え室から
 *     コスト2以下の『虹ヶ咲』のメンバーカードを1枚、メンバーのいないエリアに
 *     ウェイト状態で登場させる。」
 * The EFFECT half is a move_cards with destination = "empty_area", which is
 * the destination the survey reports as never landing. Tests 1 and 2 of the
 * file are already in test_parity_deploy_recover.c (a_q268_no_empty_area_... /
 * a_q268_deploy_to_empty_area_...); tests 3-7 are not, and they are the arms
 * that decide whether the destination filter itself is correct.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust run_activate: cost = the energy card placed under self (SelectCard over
 * the energy zone, index 0), the pay/skip gate is a SelectTarget answered with
 * 1 (pay), and every other SelectCard takes index 0. The Rust loop CAPS at 30
 * iterations; so does this one. Returns the number of prompts seen, which is
 * the non-vacuity control for every arm below. */
static int q268_run_activate(TestGame *tg, int shioriko)
{
    int prompts = 0, guard = 0;
    test_activate_ability(tg, shioriko);
    while (pending(tg) && guard++ < 30) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        prompts++;
        if (ch && ch->kind == RB_CHOICE_SELECT_CARD) {
            pick(tg, 0);
        } else if (ch && ch->kind == RB_CHOICE_SELECT_TARGET) {
            const int one[1] = { 1 };    /* pay */
            rb_resume_with_choice_indices(&tg->state, one, 1);
        } else {
            pick(tg, 0);
        }
    }
    return prompts;
}

/* Shioriko on Center with `energy` active, per the Rust `setup()`. */
static int q268_setup(TestGame *tg, int energy)
{
    int shioriko = mid(tg, SHIORIKO);
    if (!pin_id(shioriko, SHIORIKO)) return -1;
    tg->state.p[0].stage[AREA_CENTER] = shioriko;
    test_give_energy(tg, energy);
    return shioriko;
}

/* D1. rs:145 under_energy_..._too_expensive_target_no_deploy
 *
 * A cost-4 虹ヶ咲 member is NOT a valid target (rs:159-167), and the cost is
 * still paid. Taken with its PAIRED POSITIVE: an eligible cost-2 虹ヶ咲 member
 * sits in the same waitroom and MUST deploy, so the negative half cannot pass
 * on an empty candidate set. */
static void d1_q268_too_expensive_target_no_deploy_with_paired_positive(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shioriko = q268_setup(&tg, 3);
    if (shioriko < 0) { TALLY->setup_bugs++; return; }
    int tgt = mid(&tg, AYUMU_4);
    int good = mid(&tg, AYUMU_2);
    int left = mid(&tg, FILLER_010);
    if (!pin_id(tgt, AYUMU_4)) { TALLY->setup_bugs++; return; }
    if (!pin_id(good, AYUMU_2)) { TALLY->setup_bugs++; return; }
    CHECK_EQ(printed_cost(tgt), 4, "D1 premise: the blocked member really costs 4 (> 2)");
    CHECK_EQ(printed_cost(good), 2, "D1 premise: the eligible member really costs 2 (<= 2)");

    /* Only RIGHT is empty, so a deploy WOULD be possible if a target matched. */
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, tgt);
    test_add_to_discard(&tg, good);

    int prompts = q268_run_activate(&tg, shioriko);
    drain_auto(&tg);

    CHECK(prompts > 0,
          "D1 control: the 起動 really opened prompts; with none, both halves below "
          "would be vacuous");
    CHECK_EQ(is_wait(&tg, tgt), 0,
             "D1: the cost-4 虹ヶ咲 member is not a valid target and is never waited (rs:159-167)");
    CHECK(stage_area_of(&tg, tgt) < 0,
           "D1: no cost-4 member is deployed anywhere (rs:163-166)");
    CHECK(waitroom_has(&tg, tgt),
          "D1: the cost-4 member stays in the waitroom (rs:159-162)");
    /* PAIRED POSITIVE, same board, same prompt sequence. */
    CHECK(stage_area_of(&tg, good) >= 0,
          "D1 POSITIVE: the cost-2 虹ヶ咲 member on the SAME board IS deployed — the "
          "negative half above is a filter result, not an empty candidate set");
    CHECK_EQ(stage_area_of(&tg, good), AREA_RIGHT,
             "D1 POSITIVE: it lands in the only empty area (rs:120-123 of the twin)");
    CHECK(!waitroom_has(&tg, good), "D1 POSITIVE: the deployed member left the waitroom (rs:129-132)");
    CHECK_EQ(energy_under(&tg, 0, AREA_CENTER), 1,
             "D1: the cost (1 energy under 三船栞子) is still paid when no VALID target "
             "exists (rs:167-172). ROOT: the energy-zone resume arm "
             "src/ability/choice.c:1638-1671 has no target==\"under_member\" case, so "
             "line 1669 sends the selected energy to the WAITROOM instead of under the "
             "member — and the chained effect then never runs");
    CHECK(bag_has(&tg.state.p[0].discard, 1),
          "D1 EVIDENCE: the selected energy card really is in the WAITROOM, which is "
          "exactly what choice.c:1669 hard-codes for any non-energy_state_change pick");
}

/* D2. rs:179 under_energy_..._wrong_group_target_no_deploy
 *
 * A cost-2 member of the WRONG group is not a target. Paired positive: an
 * eligible 虹ヶ咲 member in the same waitroom must deploy.
 *
 * The group filter is the one the survey reports as fed a CSV `group_names`
 * copied verbatim into choice.filter_group and then matched as a single exact
 * token, which returns an EMPTY candidate set. An empty set makes the whole of
 * this test pass for free, so the positive half is the load-bearing part. */
static void d2_q268_wrong_group_target_no_deploy_with_paired_positive(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shioriko = q268_setup(&tg, 3);
    if (shioriko < 0) { TALLY->setup_bugs++; return; }
    int tgt = mid(&tg, ERI_2);
    int good = mid(&tg, AYUMU_2);
    int left = mid(&tg, FILLER_010);
    if (!pin_id(tgt, ERI_2)) { TALLY->setup_bugs++; return; }
    if (!pin_id(good, AYUMU_2)) { TALLY->setup_bugs++; return; }
    CHECK_EQ(printed_cost(tgt), 2,
             "D2 premise: the wrong-group member really costs 2, so ONLY the group "
             "filter can exclude it (rs:179-188)");
    CHECK_EQ(printed_cost(good), 2, "D2 premise: the eligible member really costs 2");

    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, tgt);
    test_add_to_discard(&tg, good);

    int prompts = q268_run_activate(&tg, shioriko);
    drain_auto(&tg);

    CHECK(prompts > 0, "D2 control: the 起動 really opened prompts");
    CHECK(waitroom_has(&tg, tgt),
          "D2: the cost-2 mu's member is not a 虹ヶ咲 target and stays in the waitroom (rs:191-195)");
    CHECK(stage_area_of(&tg, tgt) < 0,
           "D2: the non-虹ヶ咲 member is not deployed (rs:196-198)");
    CHECK(stage_area_of(&tg, good) >= 0,
          "D2 POSITIVE: the 虹ヶ咲 cost-2 member on the SAME board IS deployed — without "
          "this, an EMPTY candidate set would satisfy the whole test");
    CHECK_EQ(stage_area_of(&tg, good), AREA_RIGHT,
             "D2 POSITIVE: it lands in the only empty area");
    CHECK_EQ(energy_under(&tg, 0, AREA_CENTER), 1,
             "D2: the cost is still paid even though the effect filtered the card out "
             "(rs:199). Same root as D1: choice.c:1638-1671 sends the selected energy to "
             "the WAITROOM, not under the member, so the cost never lands as under_cards");
}

/* D3. rs:206 under_energy_..._two_valid_targets_deploy_one
 *
 * Two valid targets -> EXACTLY ONE deploys, the other stays in the waitroom
 * (rs:218-231). This is the "exact count must be chosen when possible" rule
 * (9.6.3.1) on a destination=empty_area deploy. */
static void d3_q268_two_valid_targets_deploy_one(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shioriko = q268_setup(&tg, 3);
    if (shioriko < 0) { TALLY->setup_bugs++; return; }
    int t1 = mid(&tg, AYUMU_2);
    int t2 = mid(&tg, SHIO_2);
    if (!pin_id(t1, AYUMU_2)) { TALLY->setup_bugs++; return; }
    if (!pin_id(t2, SHIO_2)) { TALLY->setup_bugs++; return; }
    CHECK(t1 != t2, "D3 premise: the two targets are DISTINCT card instances");
    CHECK_EQ(printed_cost(t1), 2, "D3 premise: target 1 really costs 2");
    CHECK_EQ(printed_cost(t2), 2, "D3 premise: target 2 really costs 2");

    /* Both LEFT and RIGHT are empty; only Center is occupied. */
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, t1);
    test_add_to_discard(&tg, t2);

    int prompts = q268_run_activate(&tg, shioriko);
    drain_auto(&tg);

    CHECK(prompts > 0, "D3 control: the 起動 really opened prompts");
    int a1 = stage_area_of(&tg, t1), a2 = stage_area_of(&tg, t2);
    int deployed = (a1 >= 0) + (a2 >= 0);
    CHECK_EQ(deployed, 1,
             "D3: EXACTLY ONE of two valid 虹ヶ咲 cost-2 members is deployed (rs:218-225). "
             "Got 0 — but NOT because the empty_area destination is broken: D18 in this "
             "file deploys into an empty area fine. ROOT: the 起動's under_member COST is "
             "mis-routed by choice.c:1638-1671, and the chained deploy effect then never "
             "runs, so the two waitroom candidates were never offered at all. 2 would "
             "mean the count gate is ignored");
    CHECK_EQ(tg.state.p[0].discard.n, 1,
             "D3: one target remains in the waitroom (rs:226-230)");
    CHECK_EQ(energy_under(&tg, 0, AREA_CENTER), 1, "D3: the cost is paid (rs:231)");
    /* The survivor is exactly the one that did not deploy. */
    CHECK(waitroom_has(&tg, a1 >= 0 ? t2 : t1),
          "D3: the un-deployed target is the one left in the waitroom");
}

/* D4. rs:238 under_energy_..._turn_limit_blocks_second_activation
 *
 * The ターン1回 gate. The Rust twin is careful to prove the FIRST activation
 * worked (the use is recorded) so the second refusal is not passing for the
 * wrong reason, and that a refusal costs nothing. Both halves are ported. */
static void d4_q268_turn_limit_blocks_second_activation(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shioriko = q268_setup(&tg, 3);
    if (shioriko < 0) { TALLY->setup_bugs++; return; }
    int t1 = mid(&tg, AYUMU_2);
    int t2 = mid(&tg, SHIO_2);
    int left = mid(&tg, FILLER_010);
    if (!pin_id(t1, AYUMU_2)) { TALLY->setup_bugs++; return; }
    if (!pin_id(t2, SHIO_2)) { TALLY->setup_bugs++; return; }
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, t1);
    test_add_to_discard(&tg, t2);

    int prompts = q268_run_activate(&tg, shioriko);
    drain_auto(&tg);

    /* rs:253-260 — the first activation must have recorded its use, else this
     * test would pass for the wrong reason. */
    CHECK(prompts > 0, "D4 control: the first activation really opened prompts");
    CHECK_EQ(turn1_used(&tg, shioriko), 1,
             "D4: the FIRST activation records its ターン1回 use (rs:253-260) — without "
             "this the second refusal would prove nothing");
    int energy_after_first = tg.state.p[0].energy_active;
    int under_after_first = energy_under(&tg, 0, AREA_CENTER);
    CHECK_EQ(under_after_first, 1,
             "D4: the first activation really placed its cost energy under 三船栞子 (rs:261-262)");

    /* rs:265 — the second activation in the same turn must be REFUSED. */
    int second = test_activate_ability(&tg, shioriko);
    CHECK_EQ(second, 0,
             "D4: ターン1回 — a second activation in the same turn must be refused (rs:264-270)");
    /* A refusal must cost nothing. */
    CHECK_EQ(tg.state.p[0].energy_active, energy_after_first,
             "D4: a ターン1回 refusal must not spend a second energy (rs:273)");
    CHECK_EQ(energy_under(&tg, 0, AREA_CENTER), under_after_first,
             "D4: a ターン1回 refusal must not place a second energy under her (rs:274-278)");
    CHECK_EQ(pending(&tg), 0,
             "D4: a use-limit refusal must not open a prompt (rs:279-282)");
    CHECK_EQ(turn1_used(&tg, shioriko), 1,
             "D4: the refused attempt must not add a second use record (rs:284-292)");
}

/* D5. rs:299 under_energy_..._no_target_no_empty_area_cost_still_paid
 *
 * Neither a target nor an empty area -> the cost is STILL paid and both stage
 * slots are unchanged (rs:315-329). This is the QA point of Q268: the cost is
 * payable independently of whether the effect can resolve. */
static void d5_q268_no_target_no_empty_area_cost_still_paid(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shioriko = q268_setup(&tg, 3);
    if (shioriko < 0) { TALLY->setup_bugs++; return; }
    int tgt = mid(&tg, ERI_2);
    int left = mid(&tg, FILLER_010);
    int right = mid(&tg, FILLER_010);
    if (!pin_id(tgt, ERI_2)) { TALLY->setup_bugs++; return; }
    CHECK(left != right, "D5 premise: the two fillers are DISTINCT instances");
    tg.state.p[0].stage[AREA_LEFT] = left;
    tg.state.p[0].stage[AREA_RIGHT] = right;
    test_add_to_discard(&tg, tgt);

    int prompts = q268_run_activate(&tg, shioriko);
    drain_auto(&tg);

    CHECK(prompts > 0, "D5 control: the 起動 really opened prompts");
    /* ROOT CAUSE EVIDENCE, and the reason the cost assertions fail in D1..D5. The
     * 起動's cost is 「エネルギー置き場にあるエネルギー1枚をこのメンバーの下に
     * 置く」, which the engine emits as a SelectCard with zone="energy",
     * target="under_member". src/ability/choice.c:1638-1671 handles zone=="energy"
     * in exactly two ways: target=="energy_state_change", or an unconditional
     * rb_choice_send_to_dst(..., "waitroom"). There is no under_member arm, so
     * the cost card is sent to the WAITROOM, no under_cards entry is created,
     * and the chained deploy effect never runs at all. That is why the "1 target
     * remains in the waitroom" assertion below sees 3, not 1: the two candidates
     * were never offered.
     *
     * NOTE this REFUTES the widely-repeated claim that
     * move_cards(destination="empty_area") never lands: D18 in this same file
     * deploys into an empty area successfully when the cost is a plain energy
     * spend. The broken piece is the under_member DESTINATION OF THE COST, and it
     * aborts the whole chained effect. test_parity_deploy_recover.c cases
     * q268_no_area / q268_wait are red on this same root today. */
    CHECK_EQ(energy_under(&tg, 0, AREA_CENTER), 1,
             "D5/Q268: the cost is paid even when there is NEITHER a target nor an empty "
             "area (rs:315-319). ROOT: choice.c:1638-1671, zone==\"energy\" has no "
             "target==\"under_member\" arm and sends the energy to the waitroom");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], left,  "D5: the left area is unchanged (rs:320-324)");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], right, "D5: the right area is unchanged (rs:325-329)");
    CHECK(waitroom_has(&tg, tgt), "D5: the non-虹ヶ咲 waitroom card is untouched");
    /* The waitroom holds the mis-routed energy card as well as the original
     * member; that is the direct observable of the root cause. */
    CHECK_EQ(tg.state.p[0].discard.n, 2,
             "D5 EVIDENCE: the waitroom holds the mu's member PLUS the energy card the "
             "cost was supposed to put under 三船栞子 (choice.c:1669 routes it here)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D2  deploy / bloom_hs_test.rs — the two heart-pattern options the C suite
 * never picks. test_parity_deploy_recover.c a_bloom_live_start_heart_choice
 * pins option 0 ONLY; options 1 and 2 select the heart04 and heart05 patterns,
 * so a SelectTarget whose option ORDER is transposed would pass today.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust setup for all four bloom tests: 40 P1 deck fillers, 10 P2, a filler and
 * Bloom in hand, 蓮ノ空 at Center, 3 energy. */
static int bloom_setup(TestGame *tg, int *hasu_out)
{
    int bloom = mid(tg, BLOOM);
    int hasu = mid(tg, SAYAKA);
    int filler = mid(tg, FILLER_010);
    if (!pin_id(bloom, BLOOM)) return -1;
    if (!pin_id(hasu, SAYAKA)) return -1;
    /* Premises, read from the pool: a LIVE card with a score, and a 蓮ノ空
     * (DOLLCHESTRA) member on stage. Verified against cards.json. */
    if (!is_live_card(bloom)) {
        fprintf(stderr, "FAIL: premise: %s is not a LIVE card\n", BLOOM);
        TALLY->failures++;
    }
    if (printed_score(bloom) <= 0) {
        fprintf(stderr, "FAIL: premise: %s has no printed score\n", BLOOM);
        TALLY->failures++;
    }
    if (!hasu_out) return -1;
    *hasu_out = hasu;

    fill_decks(tg, filler, 40);
    {   RbPlayer *Q = &tg->state.p[1];
        Q->deck.n = 0;
        for (int i = 0; i < 10 && Q->deck.n < RB_MAX_ZONE; i++)
            Q->deck.cards[Q->deck.n++] = filler;
    }
    test_add_to_hand(tg, filler);
    test_add_to_hand(tg, bloom);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[AREA_CENTER] = hasu;
    tg->state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_give_energy(tg, 3);
    return bloom;
}

/* D6. rs:124 bloom_live_start_choice_second_option_heart04 */
static void d6_bloom_live_start_choice_second_option_heart04(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    int hasu = 0;
    int bloom = bloom_setup(&tg, &hasu);
    if (bloom < 0) { TALLY->setup_bugs++; return; }

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, bloom);
    advance_to_live_start(&tg);

    /* rs:148 — the choice must be ON SCREEN before the option is chosen. This
     * is the non-vacuity control: with no prompt, choosing "option 1" proves
     * nothing about the option order. */
    int on_screen = pending(&tg);
    CHECK(on_screen, "D6 control: the heart-pattern choice is presented at ライブ開始時 (rs:148)");
    if (on_screen) pick(&tg, 1);        /* the heart04 pattern */
    drain_skip(&tg, 8);

    CHECK_EQ(need_heart_of(&tg, bloom, "heart04"), 2,
             "D6: option 1 (2xheart04 + heart0) sets the heart04 requirement to 2 (rs:153-157)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart0"), 1,
             "D6: option 1 also sets the heart0 requirement to 1 (rs:159-163)");
}

/* D7. rs:167 bloom_live_start_choice_third_option_heart05 */
static void d7_bloom_live_start_choice_third_option_heart05(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    int hasu = 0;
    int bloom = bloom_setup(&tg, &hasu);
    if (bloom < 0) { TALLY->setup_bugs++; return; }

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, bloom);
    advance_to_live_start(&tg);

    int on_screen = pending(&tg);
    CHECK(on_screen, "D7 control: the heart-pattern choice is presented at ライブ開始時 (rs:191)");
    if (on_screen) pick(&tg, 2);        /* the heart05 pattern */
    drain_skip(&tg, 8);

    CHECK_EQ(need_heart_of(&tg, bloom, "heart05"), 2,
             "D7: option 2 (2xheart05 + heart0) sets the heart05 requirement to 2 (rs:195-199)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart0"), 1,
             "D7: option 2 also sets the heart0 requirement to 1 (rs:201-205)");
}

/* D8. rs:210 bloom_choice_is_exclusive_other_hearts_zero
 *
 * Choosing heart01 must leave heart04 AND heart05 at 0 — the 「のうち、選んだ
 * 1つ」 is exclusive (rs:229-230). test_parity_deploy_recover.c asserts this
 * too (a_bloom, :1221-1224); it is repeated here because it is the ONLY
 * cross-option invariant and the arm that catches a leaked "set all three".
 * PAIRED POSITIVE: option 0 really did write heart01, so a board where the
 * whole effect no-opped cannot satisfy it. */
static void d8_bloom_choice_is_exclusive_other_hearts_zero(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    int hasu = 0;
    int bloom = bloom_setup(&tg, &hasu);
    if (bloom < 0) { TALLY->setup_bugs++; return; }

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, bloom);
    advance_to_live_start(&tg);

    int on_screen = pending(&tg);
    CHECK(on_screen, "D8 control: the heart-pattern choice is presented (rs:226)");
    if (on_screen) pick(&tg, 0);        /* the heart01 pattern */
    drain_skip(&tg, 8);

    /* PAIRED POSITIVE: the chosen pattern really was written. */
    CHECK_EQ(need_heart_of(&tg, bloom, "heart01"), 2,
             "D8 POSITIVE: choosing the heart01 pattern really sets heart01 to 2 (rs:228)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart0"), 1,
             "D8 POSITIVE: the wildcard slot is really written too (rs:231)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart04"), 0,
             "D8: heart04 stays 0 when the heart01 pattern was chosen (rs:229)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart05"), 0,
             "D8: heart05 stays 0 when the heart01 pattern was chosen (rs:230)");
}

/* D9. rs:81 bloom_no_hasunosuka_member_no_choice — the gate negative.
 *
 * No 蓮ノ空 member on stage -> NO prompt and NO modification. This is the
 * negative half of D6/D7/D8, and it is the arm that proves the prompt in those
 * three was produced by the CONDITION, not by the live-start window. It needs
 * a prompt-count control of its own: the paired positive is the same live-start
 * window WITH a 蓮ノ空 member, which is D6. Both are stated in the message. */
static void d9_bloom_no_hasunosuka_member_no_choice(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    int bloom = mid(&tg, BLOOM);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(bloom, BLOOM)) { TALLY->setup_bugs++; return; }
    if (!pin_id(filler, FILLER_010)) { TALLY->setup_bugs++; return; }

    fill_decks(&tg, filler, 40);
    {   RbPlayer *Q = &tg.state.p[1];
        Q->deck.n = 0;
        for (int i = 0; i < 10 && Q->deck.n < RB_MAX_ZONE; i++)
            Q->deck.cards[Q->deck.n++] = filler;
    }
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, bloom);
    /* NO 蓮ノ空 member on stage: the mu's filler is deliberately NOT staged. */
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 3);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, bloom);
    advance_to_live_start(&tg);

    int prompts = drain_skip(&tg, 8);
    /* The prompt count is measured AT the assertion site, not after the drain,
     * and the paired positive is D6 (same window, 蓮ノ空 present -> a prompt
     * IS raised). Without D6 this assertion is vacuous; that is why D6 exists. */
    CHECK_EQ(prompts, 0,
             "D9: with no 蓮ノ空 member on stage NO choice appears (rs:105-108). The "
             "paired positive is D6: the identical window with 蓮ノ空 at Center raises one");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart01"), 0,
             "D9: the heart01 requirement is NOT modified (rs:110-115)");
    CHECK_EQ(need_heart_of(&tg, bloom, "heart0"), 0,
             "D9: the heart0 requirement is NOT modified (rs:116-120)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D3  deploy / n_pb1_015_shizuku_test.rs and n_pb1_017_miyashita_test.rs
 *
 * 桜坂しずく and 宮下愛 each print
 *   「登場 EE 支払ってもよい：手札からコスト4以下の「<same name>」のメンバー
 *     ーカードを1枚ステージに登場させる。」
 * The pay and skip arms of both are already in test_parity_deploy_recover.c
 * (a_shizuku_pay_2e / a_shizuku_skip / a_miyashita_pay_2e). The three arms
 * below are not: "no eligible named member", "insufficient energy", and (for
 * 宮下愛) the skip arm.
 *
 * RUST FIXTURE NOTE — the Rust twin uses "PL!N-bp1-015-N" and "PL!N-bp1-017-N"
 * for the hand card. Both are MISSING from cards/cards.json; the real prints
 * are "PL!N-bp1-015-PRproteinbar" (cost 4) and "PL!N-bp1-017-N" is present
 * while the -R activator the twin uses is "PL!N-pb1-017-R". The C twin in
 * test_parity_deploy_recover.c already uses PL!N-bp1-015-N / PL!N-bp1-017-N,
 * which DO exist (cost 4 each, per cards.json), so the Rust file's spelling is
 * the buggy one and the C card numbers are correct. This file follows cards.json.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust shizuku_no_eligible_shizuku_no_deploy / miyashita_no_eligible_no_deploy:
 * pay the optional cost but the hand holds no eligible NAMED member, so
 * nothing deploys and the non-matching hand card is untouched. */
static void d10_shizuku_no_eligible_shizuku_no_deploy(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shizuku = mid(&tg, SHIZUKU_R);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(shizuku, SHIZUKU_R)) { TALLY->setup_bugs++; return; }
    if (!pin_id(filler, FILLER_010)) { TALLY->setup_bugs++; return; }

    test_add_to_hand(&tg, shizuku);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 15);
    for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    play_to_stage(&tg, shizuku, AREA_CENTER, 0);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], shizuku, "D10 setup: しずく is on Center");
    /* rs:58-59 — the pay/skip gate must be ON SCREEN; the answer is PAY. */
    int gate = pending(&tg);
    CHECK(gate, "D10 control: the optional pay-2E gate is presented (rs:58)");
    int prompts = 0;
    if (gate) { pick(&tg, 1); prompts++; }     /* pay */
    prompts += drain_pick0(&tg, 8);
    drain_auto(&tg);

    /* PAIRED POSITIVE: a prompt WAS answered above. Without it, "nothing
     * happened" would be indistinguishable from "the effect never ran". */
    CHECK(prompts > 0,
          "D10 POSITIVE: at least one prompt was answered, so the negative below "
          "measures the FILTER and not a dead board");
    CHECK(hand_has(&tg, filler),
          "D10: the only other hand card is not a 桜坂しずく and stays in hand (rs:63). "
          "ENGINE BUG: the 「桜坂しずく」 name filter is NOT applied — the mu's filler "
          "PL!-sd1-010-SD was deployed instead. test_parity_deploy_recover.c "
          "a_shizuku_pay_2e_deploys cannot see this because it only checks that the "
          "shizuku_hand card is gone from hand, which is true whether the RIGHT or the "
          "WRONG card was taken");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "D10: exactly the one non-matching card remains in hand (rs:63)");
    /* A new card must never appear out of nowhere. */
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], shizuku,
             "D10: しずく is the only member on stage — no phantom deploy");
}

/* D11. rs:66 shizuku_insufficient_energy_no_pay
 *
 * Only 1 energy for a cost-13 member: the play must be REFUSED, the energy
 * untouched, and NEITHER hand card moved. The Rust twin is careful to assert
 * the target card too (rs:86-89), because a refusal that half-happened would
 * otherwise pass. */
static void d11_shizuku_insufficient_energy_no_pay(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int shizuku = mid(&tg, SHIZUKU_R);
    int hand_named = mid(&tg, SHIZUKU_N);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(shizuku, SHIZUKU_R)) { TALLY->setup_bugs++; return; }
    if (!pin_id(hand_named, SHIZUKU_N)) { TALLY->setup_bugs++; return; }
    CHECK(printed_cost(shizuku) > 1,
          "D11 premise: the activator really costs more than the 1 energy given (rs:77-78)");

    test_add_to_hand(&tg, shizuku);
    test_add_to_hand(&tg, hand_named);
    test_give_energy(&tg, 1);
    for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    int r = test_try_play_to_stage(&tg, shizuku, AREA_CENTER);
    CHECK_EQ(r, 0, "D11: a cost-13 member cannot be played with 1 energy (rs:77-78)");
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "D11: a refused play leaves the 1 energy UNSPENT (rs:81)");
    CHECK(hand_has(&tg, shizuku), "D11: a refused play leaves the activator in hand (rs:82-85)");
    CHECK(hand_has(&tg, hand_named),
          "D11: a refused play must not move the named target out of hand either (rs:86-89)");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "D11: a refused play must leave the stage empty (rs:90-94)");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], RB_EMPTY_SLOT,  "D11: left stays empty");
    CHECK_EQ(tg.state.p[0].stage[AREA_RIGHT], RB_EMPTY_SLOT, "D11: right stays empty");
    CHECK_EQ(pending(&tg), 0, "D11: a refused play opens no prompt");
}

/* D12. rs:24 miyashita_skip_no_deploy — the SKIP arm of 宮下愛's 登場, which
 * test_parity_deploy_recover.c never ports (it ports the PAY arm only,
 * a_miyashita_pay_2e_deploys). */
static void d12_miyashita_skip_no_deploy(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int miya = mid(&tg, MIYA_R);
    int miya_hand = mid(&tg, MIYA_N);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(miya, MIYA_R)) { TALLY->setup_bugs++; return; }
    if (!pin_id(miya_hand, MIYA_N)) { TALLY->setup_bugs++; return; }

    test_add_to_hand(&tg, miya);
    test_add_to_hand(&tg, miya_hand);
    test_give_energy(&tg, 15);
    for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    play_to_stage(&tg, miya, AREA_CENTER, 0);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], miya, "D12 setup: 宮下愛 is on Center");
    int gate = pending(&tg);
    CHECK(gate, "D12 control: the optional pay-2E gate is presented (rs:36-37)");
    int prompts = 0;
    if (gate) { pick(&tg, 0); prompts++; }     /* SKIP */
    prompts += drain_skip(&tg, 8);
    drain_auto(&tg);

    CHECK(prompts > 0, "D12 POSITIVE: a prompt was really answered before the negative");
    CHECK(hand_has(&tg, miya_hand),
          "D12: declining the optional cost keeps the named member in hand (rs:39). "
          "ENGINE BUG: the pay_optional_cost:skip gate was answered with index 0 (SKIP) "
          "and the deploy STILL happened — with the WRONG card, because the 「宮下愛」 "
          "name filter is not applied. The engine's [DEBUT_COST_PENDING] resume log "
          "shows the skip being taken and the debut continuing anyway");
    CHECK(stage_area_of(&tg, miya_hand) < 0,
          "D12: the NAMED card was not deployed on the skip path (rs:40)");
    /* PAIRED POSITIVE, and a warning about the peer: D12's positive counterpart is
     * test_parity_deploy_recover.c a_miyashita_pay_2e_deploys (:395), but that arm
     * is BLIND to the wrong-card bug — it only asserts that miya_hand is no longer
     * in hand, which holds whether the right or the wrong card was deployed. So
     * that peer arm being green does NOT establish this negative. D10 is the arm
     * that pins card identity, and it is red. */
    EXPECTED_GAP("D12 the paired PAY positive (test_parity_deploy_recover.c "
                 "a_miyashita_pay_2e_deploys :395) is card-identity-BLIND, so it cannot "
                 "establish this negative; D10 is the identity-pinning arm and is red",
                 hand_has(&tg, miya_hand));
}

/* D13. rs:59 miyashita_insufficient_energy_cannot_play */
static void d13_miyashita_insufficient_energy_cannot_play(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int miya = mid(&tg, MIYA_R);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(miya, MIYA_R)) { TALLY->setup_bugs++; return; }
    CHECK(printed_cost(miya) > 1,
          "D13 premise: 宮下愛 really costs more than the 1 energy given (rs:69)");

    test_add_to_hand(&tg, miya);
    test_give_energy(&tg, 1);
    for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    int r = test_try_play_to_stage(&tg, miya, AREA_CENTER);
    CHECK_EQ(r, 0, "D13: 宮下愛 cannot be afforded with 1 energy (rs:69)");
    CHECK_EQ(tg.state.p[0].energy_active, 1, "D13: the 1 energy is UNSPENT (rs:70)");
    CHECK(hand_has(&tg, miya), "D13: a refused play leaves 宮下愛 in hand (rs:71-74)");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "D13: a refused play leaves the stage empty (rs:75-79)");
    CHECK_EQ(pending(&tg), 0, "D13: a refused play opens no prompt");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D4  deploy / lower_cost_baton_optional_low_cost_hand_deploy_test.rs
 *
 * 西木野真姫 「このメンバーよりコストが低いメンバーからバトンタッチして登場し
 *   た場合、自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させ
 *   てもよい。」 The baton-touch positive and the no-baton-touch negative are
 *   in test_parity_deploy_recover.c (a_lower_cost_baton_deploys_cheap_member /
 *   a_no_baton_touch_no_appear). The two arms below are not.
 * ══════════════════════════════════════════════════════════════════════════ */

/* D14. rs:69 lower_cost_baton_hand_deploy_equal_cost_no_appear
 *
 * Baton touch at EQUAL cost (17 vs 17) fails the 「コストが低い」 gate: the new
 * copy enters play, the old one leaves, and NO appear effect fires. The
 * negative ("exactly 1 member on stage") is safe on its own because the play
 * itself is a strong positive: the new Maki really IS on stage, so a total
 * engine failure could not produce it. */
static void d14_lower_cost_baton_equal_cost_no_appear(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int maki_hand = mid(&tg, MAKI);
    int maki_stage = mid(&tg, MAKI);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(maki_hand, MAKI)) { TALLY->setup_bugs++; return; }
    if (!pin_id(maki_stage, MAKI)) { TALLY->setup_bugs++; return; }
    CHECK(maki_hand != maki_stage,
          "D14 premise: the incoming and the replaced Maki are DISTINCT instances");
    CHECK_EQ(printed_cost(maki_hand), printed_cost(maki_stage),
             "D14 premise: the baton touch really is at EQUAL cost (17 == 17, rs:67)");

    /* The equal-cost Maki sits at Center; the incoming copy is played there. */
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = maki_stage;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, maki_hand);
    test_give_energy(&tg, 17);
    fill_decks(&tg, filler, 30);

    int r = test_play_to_stage(&tg, maki_hand, AREA_CENTER);
    CHECK_EQ(r, 1, "D14: the equal-cost baton touch itself SUCCEEDS (rs:80-88) — the "
                   "paired positive that keeps the negative below honest");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], maki_hand,
             "D14: the new Maki is on stage after the baton touch (rs:90-93)");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER] != maki_stage, 1,
             "D14: the old Maki really was displaced (the two are distinct ids)");
    int occupied = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg.state.p[0].stage[i] != RB_EMPTY_SLOT) occupied++;
    CHECK_EQ(occupied, 1,
             "D14: exactly 1 member is on stage — the old Maki was replaced and NO "
             "member appeared from hand (rs:94-104)");
    /* No hand card was pulled onto the stage. */
    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "D14: nothing was taken from hand — 「このメンバーよりコストが低い」 failed");
}

/* D15. rs:131 lower_cost_baton_hand_deploy_no_eligible_hand_skips
 *
 * A LOWER-cost baton touch, but the only hand card costs 9 (> 4), so the
 * optional appear has no legal target and skips gracefully. Paired positive: a
 * LOWER-cost baton touch really did happen (the new Maki is on stage), and the
 * sibling with an eligible card is a_lower_cost_baton_deploys_cheap_member. */
static void d15_lower_cost_baton_no_eligible_hand_skips(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int maki = mid(&tg, MAKI);
    int too_dear = mid(&tg, HOSHIZORA);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(maki, MAKI)) { TALLY->setup_bugs++; return; }
    if (!pin_id(too_dear, HOSHIZORA)) { TALLY->setup_bugs++; return; }
    CHECK(printed_cost(too_dear) > 4,
          "D15 premise: the hand card really costs more than the cost-4 filter (rs:156-157)");
    CHECK(printed_cost(filler) < printed_cost(maki),
          "D15 premise: the displaced member really costs LESS than 17, so the "
          "baton-touch gate PASSES (rs:21-22)");

    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = filler;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, maki);
    test_add_to_hand(&tg, too_dear);
    test_give_energy(&tg, 17);
    fill_decks(&tg, filler, 30);

    int r = test_play_to_stage(&tg, maki, AREA_CENTER);
    CHECK_EQ(r, 1, "D15: the lower-cost baton touch itself SUCCEEDS (rs:144-152) — the "
                   "paired positive for the negative below");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], maki,
             "D15: Maki is on stage after the baton touch");
    /* The Rust twin asserts the END STATE only (rs:154-162). Whether the engine
     * raises a hand SelectCard and then finds it empty, or declines to raise one
     * at all, is not observable in Rust — so this file asserts the end state
     * strictly and records the prompt-count question as a gap rather than
     * inventing an expectation Rust never made. */
    int prompts = drain_skip(&tg, 10);
    drain_auto(&tg);
    EXPECTED_GAP("D15 Rust asserts only that nothing was deployed and the cost-9 card "
                 "stayed in hand; it cannot observe whether C raises an empty hand "
                 "SelectCard first. C DOES raise one (RB_CHOICE_SELECT_CARD zone=hand "
                 "target=self) — reported, not asserted either way",
                 prompts == 0);
    CHECK(hand_has(&tg, too_dear),
          "D15: the cost-9 card stays in hand — the cost<=4 filter excluded it (rs:154-157)");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "D15: the hand is otherwise untouched");
    int occupied = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg.state.p[0].stage[i] != RB_EMPTY_SLOT) occupied++;
    CHECK_EQ(occupied, 1, "D15: no extra member appeared on the stage (rs:158-162)");
    CHECK(stage_area_of(&tg, too_dear) < 0,
          "D15: the cost-9 card was never placed on the stage");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D5  deploy / s_bp3_006_cost_ref_test.rs — 津島善子 津島善子
 *
 * 「このメンバーをウェイトにし、手札を1枚控え室に置く：このメンバー以外の『Aqours』
 *   のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室から、
 *   そのメンバーのコストに2を足した数に等しいコストの『Aqours』のメンバーカードを1枚、
 *   そのメンバーがいたエリアに登場させる。」（センターのみ起動可能）
 *
 * RUST FIXTURE BUG, with cards.json evidence — the Rust twin's positive
 * (`yoshi_center_cost4_to_cost6_succeeds`, rs:7-47) stages
 * "PL!S-bp2-008-P" as the "cost 4 + 2 = 6" candidate. cards.json reads:
 *     "PL!S-bp2-008-P": name 小原鞠莉, cost 17, unit GuiltyKiss
 * and NO cost-6 Aqours member exists in the entire database (the cost ladder
 * jumps 5 -> 7 -> 9). The Rust assertion at rs:46 is a disjunction
 * `stage[0]==candidate || waitroom.contains(target)` which is satisfied by the
 * FIRST branch alone, so the test passes for the wrong reason even in Rust.
 * The negative arm the twin really pins — `yoshi_cost4_with_no_cost6_in_discard_no_place`
 * (rs:50-88) — is portable and is D16; the two are split here so the bug is
 * visible instead of silently inherited.
 * ══════════════════════════════════════════════════════════════════════════ */

/* D16. rs:50 yoshi_cost4_with_no_cost6_in_discard_no_place
 *
 * The removal half WORKS (the cost-4 桜内梨子 goes to the waitroom) but the
 * follow-up has no cost-6 『Aqours』 card, so nothing is deployed. The
 * "removal happened" assertion is the PAIRED POSITIVE that gives the
 * "nothing placed" half its meaning: without it the board never moved and the
 * negative would be free. */
static void d16_yoshi_cost4_no_cost6_in_discard_nothing_placed(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int yoshi = mid(&tg, YOSHI);
    int target = mid(&tg, RIKO_4);
    int wrong_cost = mid(&tg, RUBY_2);
    if (!pin_id(yoshi, YOSHI)) { TALLY->setup_bugs++; return; }
    if (!pin_id(target, RIKO_4)) { TALLY->setup_bugs++; return; }
    if (!pin_id(wrong_cost, RUBY_2)) { TALLY->setup_bugs++; return; }
    CHECK_EQ(printed_cost(target), 4,
             "D16 premise: the removal target really costs 4, so cost+2 would be 6 (rs:55)");
    CHECK(printed_cost(wrong_cost) != printed_cost(target) + 2,
          "D16 premise: the waitroom card is NOT cost 6, so no legal candidate exists (rs:55, rs:87)");

    /* 善子 must be at CENTER — the ability is centre-only (rs:117-125). */
    tg.state.p[0].stage[AREA_LEFT] = target;
    tg.state.p[0].stage[AREA_CENTER] = yoshi;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, wrong_cost);
    test_add_to_hand(&tg, mid(&tg, FILLER_010));
    test_give_energy(&tg, 20);
    fill_decks(&tg, mid(&tg, FILLER_010), 30);

    int act = test_activate_ability(&tg, yoshi);
    CHECK_EQ(act, 1, "D16: a CENTRE-positioned 善子 is activatable (rs:20, the twin)");
    int prompts = drain_pick0(&tg, 12);
    drain_auto(&tg);

    /* PAIRED POSITIVE: the activation really ran and really moved the target. */
    CHECK(prompts > 0, "D16 control: the activation really opened prompts (rs:63-83)");
    CHECK(waitroom_has(&tg, target),
          "D16 POSITIVE: the cost-4 『Aqours』 member really was moved to the waitroom "
          "(rs:85-86) — this is what makes the 'nothing placed' half non-vacuous");
    CHECK(stage_area_of(&tg, wrong_cost) < 0,
          "D16: the non-cost-6 waitroom card is not placed on the stage (rs:87)");
    CHECK_EQ(tg.state.p[0].stage[AREA_LEFT], RB_EMPTY_SLOT,
             "D16: the vacated area is left empty — no cost-6 card existed to fill it (rs:87)");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], yoshi,
             "D16: 善子 herself is still at Center");
    /* 「このメンバーをウェイトにし」 is the printed first half of the cost. */
    CHECK_EQ(is_wait(&tg, yoshi), 1,
             "D16: the cost waits 善子 herself (「このメンバーをウェイトにし」). ROOT: the "
             "two-part cost 「このメンバーをウェイトにし、手札を1枚控え室に置く」 is applied "
             "as a single DISCARD_MOVE destination=same_area; the ウェイト (orientation) "
             "half is never applied at all");
}

/* D17. rs:91 yoshi_no_other_aqours_no_target
 *
 * 善子 is the ONLY member on stage, so after the hand-discard cost there is no
 * 『Aqours』 member to remove and the effect ends (9.6.3.1.3: zero eligible
 * targets -> the selection is ignored entirely, no prompt left stuck). */
static void d17_yoshi_no_other_aqours_no_target(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int yoshi = mid(&tg, YOSHI);
    int filler = mid(&tg, FILLER_010);
    if (!pin_id(yoshi, YOSHI)) { TALLY->setup_bugs++; return; }
    if (!pin_id(filler, FILLER_010)) { TALLY->setup_bugs++; return; }

    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = yoshi;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 20);
    fill_decks(&tg, filler, 30);

    int prompts_total = 0;
    test_activate_ability(&tg, yoshi);
    prompts_total += drain_pick0(&tg, 12);
    drain_auto(&tg);
    printf("  D17 raised %d prompt(s) before the state-change cost fired\n", prompts_total);
    EXPECTED_GAP("D17 is a pure negative (no eligible target -> the selection is ignored). "
                 "Rust asserts the end state, which needs no prompt to have been raised; "
                 "C raised one first. Recorded, not asserted either way",
                 prompts_total == 0);

    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], yoshi,
             "D17: 善子 is on Center — she is never removed by her own ability (rs:108). "
             "ENGINE BUG: she was REMOVED FROM THE STAGE. The cost "
             "「このメンバーをウェイトにし、手札を1枚控え室に置く」 is a STATE CHANGE, but "
             "the C engine runs it as a card move, so the activating member leaves the "
             "stage instead of being waited");
    CHECK_EQ(pending(&tg), 0,
             "D17: with no other 『Aqours』 member the selection is ignored and NO prompt "
             "is left stuck (9.6.3.1.3). The paired positive is D16, where an eligible "
             "target really IS selected");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "D17: the hand-discard half of the cost did NOT happen either — the hand is "
             "untouched, so the whole two-part cost is mis-handled");
    CHECK(stage_area_of(&tg, yoshi) < 0 && !waitroom_has(&tg, yoshi) && !hand_has(&tg, yoshi),
          "D17 EVIDENCE: 善子 is in NO zone at all — the state-change cost DESTROYED her, "
          "which is the same class as the known rb_place_card_in_zone/zone_bag NULL "
          "card-vanish defect");
}

/* D18. rs:118 pl_hs_bp6_016_r_non_hasunosora_ignored
 *
 * 桂城泉 「起動 ターン1回 4E：自分の控え室からコスト4以下の『蓮ノ空』のメンバーカードを
 *   1枚、メンバーのいないエリアに登場させる。」 An eligible 蓮ノ空 member and an
 *   ineligible mu's member sit in the waitroom TOGETHER: only the 蓮ノ空 one
 *   deploys. The eligible one IS the paired positive — the negative half is the
 *   outsider staying put, and it is worthless without it. */
static void d18_izumi_bp6_non_hasunosora_ignored(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    int me = mid(&tg, IZUMI_16);
    int kamaru = mid(&tg, KAMARU);
    int outsider = mid(&tg, FILLER_010);
    if (!pin_id(me, IZUMI_16)) { TALLY->setup_bugs++; return; }
    if (!pin_id(kamaru, KAMARU)) { TALLY->setup_bugs++; return; }
    if (!pin_id(outsider, FILLER_010)) { TALLY->setup_bugs++; return; }
    CHECK_EQ(printed_cost(kamaru), 2,
             "D18 premise: 夕霧綴理 really costs 2, inside the cost-<=4 filter (rs:122-123)");
    CHECK(printed_cost(outsider) <= 4,
          "D18 premise: the mu's member also costs <= 4, so ONLY the group filter can "
          "exclude it (rs:123-124)");

    fill_decks(&tg, filler, 30);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = me;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, kamaru);
    test_add_to_discard(&tg, outsider);
    test_give_energy(&tg, 6);

    int act = test_activate_ability(&tg, me);
    CHECK_EQ(act, 1, "D18: 桂城泉 is activatable at Center with 6 energy (rs:11-22)");
    int prompts = drain_pick0(&tg, 10);
    drain_auto(&tg);

    CHECK(prompts > 0, "D18 control: the activation really opened prompts");
    /* PAIRED POSITIVE. */
    CHECK(stage_has(&tg, kamaru),
          "D18 POSITIVE: the eligible 蓮ノ空 member really deploys (rs:130-133). Without "
          "this the 'ignored' half below is free");
    /* NEGATIVE half, now meaningful. */
    CHECK(waitroom_has(&tg, outsider),
          "D18: the non-蓮ノ空 member is ignored and stays in the waitroom (rs:134-137)");
    CHECK(stage_area_of(&tg, outsider) < 0,
          "D18: the non-蓮ノ空 member is never placed on the stage");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "D18: exactly the outsider remains in the waitroom");
    /* 桂城泉 is 蓮ノ空 herself but she is a STAGE member, never a waitroom
     * candidate, and her cost is 9 so the filter excludes her anyway. */
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], me, "D18: 桂城泉 stays at Center");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D6  deploy / optional_energy_deploy_two_total_cost_four_test.rs
 *
 * 津島善子 「登場 4E 支払ってもよい：自分の控え室から、コストの合計が4以下になる
 *   ようにメンバーカードを2枚までステージに登場させる。」
 * The over-cost filter and the full-stage arm are in test_parity_deploy_recover.c
 * (a_yoshiko_over_cost_filtered_and_budget_tracked / a_yoshiko_full_stage_graceful).
 * The budget REDUCTION arm is not, and it is the one that can be vacuous: if
 * the re-prompt never appears, "the second card was not deployed" is free.
 * ══════════════════════════════════════════════════════════════════════════ */

/* D19. rs:301 deploy_cost2_then_cost4_exceeds
 *
 * Deploy a cost-2 member first; the re-prompt must carry the REMAINING budget of
 * 2, so the cost-4 card no longer fits and only the second cost-2 card deploys.
 * Two prompt-count controls: the initial pick must be ON SCREEN, and the
 * re-prompt must be ON SCREEN with cost_total == 2 before anything is answered. */
static void d19_yoshiko_cost2_then_cost4_exceeds_budget(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int yoshiko = mid(&tg, YOSHIKO);
    int cost2a = mid(&tg, SD1_002);
    int cost2b = mid(&tg, SD1_005);
    int cost4 = mid(&tg, SD1_008);
    if (!pin_id(yoshiko, YOSHIKO)) { TALLY->setup_bugs++; return; }
    if (!pin_id(cost2a, SD1_002)) { TALLY->setup_bugs++; return; }
    if (!pin_id(cost2b, SD1_005)) { TALLY->setup_bugs++; return; }
    if (!pin_id(cost4, SD1_008)) { TALLY->setup_bugs++; return; }
    CHECK_EQ(printed_cost(cost2a), 2, "D19 premise: the first pick really costs 2");
    CHECK_EQ(printed_cost(cost2b), 2, "D19 premise: the second cost-2 card really costs 2");
    CHECK_EQ(printed_cost(cost4), 4, "D19 premise: the over-budget card really costs 4");

    test_add_to_discard(&tg, cost2a);   /* waitroom index 0 */
    test_add_to_discard(&tg, cost4);    /* index 1 */
    test_add_to_discard(&tg, cost2b);   /* index 2 */
    test_add_to_hand(&tg, yoshiko);
    test_give_energy(&tg, 16);
    tg.state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;

    play_to_stage(&tg, yoshiko, AREA_CENTER, 0);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], yoshiko, "D19 setup: 善子 is on Center");
    CHECK(pending(&tg), "D19 control: the optional 4E gate is ON SCREEN (rs:90-97)");

    /* Pay. */
    if (pending_is(&tg, "SelectTarget")) { const int one[1] = { 1 }; rb_resume_with_choice_indices(&tg.state, one, 1); }
    CHECK(pending_is(&tg, "SelectCard"),
          "D19 control: the waitroom pick is a SelectCard, ON SCREEN (rs:98-100)");
    {
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        if (ch) CHECK_EQ(ch->cost_total, 4,
                         "D19: the INITIAL prompt carries the full budget cost_total 4 (rs:117-120)");
    }
    pick(&tg, 0);                                  /* the first cost-2 card */
    if (pending_is(&tg, "SelectPosition")) pick(&tg, 0);
    /* NOT drain_pick0 here: that would auto-answer the budget re-prompt before it
     * can be inspected, which is exactly the prompt the whole test is about. */

    /* The RE-prompt must be on screen, carrying the REDUCED budget. */
    const RbChoice *re = rb_get_pending_choice(&tg.state);
    int re_prompt = (re && re->kind == RB_CHOICE_SELECT_CARD);
    CHECK(re_prompt,
          "D19 control: a SECOND SelectCard is ON SCREEN after the first deploy. If it "
          "never appears, the 'the cost-4 card did not deploy' half below is vacuous");
    if (re_prompt) {
        CHECK_EQ(re->cost_total, 2,
                 "D19: the re-prompt's cost_total is the REMAINING budget 2, not the "
                 "original 4 (rs:301-330)");
        int offers_cost4 = 0, offers_cost2b = 0, n = 0;
        for (int i = 0; i < re->n_filtered_indices; i++) {
            int wi = re->filtered_indices[i];
            if (wi < 0 || wi >= tg.state.p[0].discard.n) continue;
            n++;
            int c = tg.state.p[0].discard.cards[wi];
            if (c == cost4) offers_cost4 = 1;
            if (c == cost2b) offers_cost2b = 1;
        }
        CHECK_EQ(offers_cost4, 0,
                 "D19: the cost-4 card is NOT offered once only 2 budget remains (rs:301-330)");
        CHECK_EQ(offers_cost2b, 1, "D19: the other cost-2 card IS still offered");
        CHECK_EQ(n, 1, "D19: exactly one card fits the remaining budget of 2. Got 0 — the "
                       "re-prompt carries an EMPTY filtered set, so the cost<=2 filter is "
                       "not applied to the re-prompt at all");
        /* ANSWER THE RE-PROMPT, then check the end state. Rust takes both (rs:337). */
        pick(&tg, 0);
        if (pending_is(&tg, "SelectPosition")) pick(&tg, 0);
    }
    drain_skip(&tg, 6);

    CHECK(stage_has(&tg, cost2a),
          "D19 POSITIVE: the first selected cost-2 member really is on stage (rs:162-209)");
    CHECK(stage_area_of(&tg, cost4) < 0,
          "D19: the cost-4 card is never deployed — the total would exceed 4 (rs:301-330). "
          "The re-prompt DID appear, but with cost_total 0 and an empty filtered set, so "
          "the running budget is lost and the cost-4 card is accepted");
    CHECK(waitroom_has(&tg, cost4), "D19: the cost-4 card stays in the waitroom");
    CHECK(waitroom_has(&tg, cost2b),
          "D19: this arm DECLINES the second card, so the other cost-2 card stays too");
    CHECK_EQ(tg.state.p[0].discard.n, 2, "D19: exactly two waitroom cards remain");
}

/* D20. rs:128 fuyumari_q95_player_chooses_card_from_discard
 *
 * 鬼塚冬毬 「登場『鬼塚冬毬』以外の『Liella!』のメンバー1人をステージから控え室に
 *   置いてもよい：自分の控え室から、これにより控え室に置いたメンバーカードを1枚、
 *   そのメンバーがいたエリアに登場させる。」
 * The PLAYER chooses WHICH waitroom card re-deploys, and it re-deploys into the
 * VICTIM's area — not into a fixed slot. a_fuyumari_exclude_self_from_cost
 * (test_parity_deploy_recover.c:1107) uses ONE candidate so the identity of the
 * redeployed card is not pinned; this arm has TWO waitroom members and asserts
 * that the one the player picked is the one that appears, at Left. */
static void d20_fuyumari_q95_player_chooses_redeployed_card(void)
{
    static TestGame tg;
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    clear_p1(&tg);
    int fuyumari = mid(&tg, FUYUMARI);
    int victim = mid(&tg, SD1_006);       /* 桜小路きな子, cost 2, Liella! 5yncri5e! */
    int chosen = mid(&tg, SD1_008S);      /* 若菜四季, cost 4, Liella! 5yncri5e!    */
    if (!pin_id(fuyumari, FUYUMARI)) { TALLY->setup_bugs++; return; }
    if (!pin_id(victim, SD1_006)) { TALLY->setup_bugs++; return; }
    if (!pin_id(chosen, SD1_008S)) { TALLY->setup_bugs++; return; }
    /* Identity trap: the DEBUT print is PL!SP-pb1-011-R, not the Liella! live
     * PL!SP-pb2-011-R. */
    CHECK(!rb_card_no_eq(fuyumari, "PL!SP-pb2-011-R"),
          "D20 premise: the staged card is the debut print, not PL!SP-pb2-011-R");

    test_add_to_hand(&tg, fuyumari);
    /* The victim is at LEFT; Center is where 冬毬 will debut. */
    tg.state.p[0].stage[AREA_LEFT] = victim;
    tg.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_discard(&tg, chosen);     /* waitroom index 0 -> the one we pick */
    test_add_to_discard(&tg, victim);
    test_give_energy(&tg, 13);
    fill_decks(&tg, mid(&tg, FILLER_010), 30);

    play_to_stage(&tg, fuyumari, AREA_CENTER, 0);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], fuyumari, "D20 setup: 冬毬 is on Center");
    /* Pay the optional stage cost. */
    int gate = pending(&tg);
    CHECK(gate, "D20 control: the optional stage cost presents a pay/skip gate (rs:149-151)");
    if (gate) pick(&tg, 1);
    /* The victim's removal is a single candidate and auto-resolves; the NEXT
     * ask is the re-deploy pick. It must be ON SCREEN before it is answered. */
    int pick_prompt = pending_is(&tg, "SelectCard");
    CHECK(pick_prompt,
          "D20 control: the re-deploy SelectCard is ON SCREEN (rs:156-164). If it never "
          "appears, the 'chosen card is the one that appears' assertion is vacuous");
    if (pick_prompt) pick(&tg, 0);       /* choose waitroom index 0 == `chosen` */
    int leftover = drain_pick0(&tg, 6);
    drain_auto(&tg);
    if (leftover < 0) CHECK(0, "unreachable");

    /* PAIRED POSITIVE + the identity claim. */
    CHECK(stage_area_of(&tg, chosen) >= 0,
          "D20: the card the PLAYER chose (waitroom index 0) is the one that appears, in "
          "the victim's own area at Left (rs:172-174). ENGINE BUG: the engine put the "
          "VICTIM back, i.e. it re-deployed the cost target rather than the card THIS WAY "
          "sent to the waitroom. test_parity_deploy_recover.c "
          "a_fuyumari_exclude_self_from_cost cannot see this: it seeds only ONE waitroom "
          "card, which is the victim, so identity is indistinguishable there");
    CHECK(stage_area_of(&tg, chosen) >= 0,
          "D20 POSITIVE: a card other than the victim really is the redeployed one, so the "
          "identity claim above is not a no-op");
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], fuyumari, "D20: 冬毬 on Center (rs:176-178)");
    CHECK(stage_area_of(&tg, victim) < 0,
          "D20: the VICTIM itself does not re-deploy — the re-deployed card is the one "
          "THIS WAY sent to the waitroom (rs:140 / rs:165)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * driver
 * ══════════════════════════════════════════════════════════════════════════ */

static void on_crash(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) { current_test = name; fn(); current_test = "(none)"; return; }
    if (pid == 0) {
        int a0 = TALLY->assertions, f0 = TALLY->failures,
            s0 = TALLY->setup_bugs, g0 = TALLY->gaps;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s)\n",
               name, TALLY->assertions - a0, TALLY->failures - f0,
               TALLY->setup_bugs - s0, TALLY->gaps - g0);
        fflush(stdout); fflush(stderr);
        _Exit(TALLY->failures > f0 || TALLY->setup_bugs > s0 ? 1 : 0);
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
    signal(SIGSEGV, on_crash);
    signal(SIGBUS,  on_crash);
    signal(SIGABRT, on_crash);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    void *shared = mmap(NULL, 4096, PROT_READ | PROT_WRITE,
                        MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: could not map the shared tally page\n");
        return 1;
    }
    memset(shared, 0, 4096);
    g_tally = (struct RunTally *)shared;

    if (maybe_probe(getenv("RB_TDM_DUMP"))) { rb_unload(); return 0; }

    /* Print the heart-colour mapping so a remap is visible rather than a
     * silently-wrong index. The known trap is test_get_heart_modifier, which
     * remaps 5 onto RB_HEART_ORANGE; this file never calls it. */
    printf("heart colour mapping (rb_parse_heart_color -> enum -> %s):\n",
           rb_heart_color_as_str(RB_HEART_PINK));
    {
        static const char *names[] = { "heart00", "heart01", "heart02", "heart03",
                                       "heart04", "heart05", "heart06" };
        for (int i = 0; i < 7; i++)
            printf("  %s -> %d (%s)\n", names[i],
                   (int)rb_parse_heart_color(names[i]),
                   rb_heart_color_as_str(rb_parse_heart_color(names[i])));
    }

    printf("--- M multi_card: the one test the C suite never actually ran ---\n");
    run("M1_p2_owned_ginko_targets_p1_board", m1_p2_owned_ginko_targets_p1_board_from_p2_seat);
    run("M1b_p1_owned_ginko_paired_positive", m1b_p1_owned_ginko_is_the_paired_positive);

    printf("--- D1 deploy q268 empty_area (the core destination defect) ---\n");
    run("D1_q268_too_expensive_no_deploy", d1_q268_too_expensive_target_no_deploy_with_paired_positive);
    run("D2_q268_wrong_group_no_deploy",  d2_q268_wrong_group_target_no_deploy_with_paired_positive);
    run("D3_q268_two_valid_deploy_one",  d3_q268_two_valid_targets_deploy_one);
    run("D4_q268_turn1_blocks_second",   d4_q268_turn_limit_blocks_second_activation);
    run("D5_q268_no_target_no_area_paid", d5_q268_no_target_no_empty_area_cost_still_paid);

    printf("--- D2 deploy bloom heart-pattern options 1 and 2 ---\n");
    run("D6_bloom_option1_heart04", d6_bloom_live_start_choice_second_option_heart04);
    run("D7_bloom_option2_heart05", d7_bloom_live_start_choice_third_option_heart05);
    run("D8_bloom_choice_exclusive", d8_bloom_choice_is_exclusive_other_hearts_zero);
    run("D9_bloom_no_hasunosuka_no_choice", d9_bloom_no_hasunosuka_member_no_choice);

    printf("--- D3 deploy shizuku / miyashita 登場 optional-cost arms ---\n");
    run("D10_shizuku_no_eligible_no_deploy", d10_shizuku_no_eligible_shizuku_no_deploy);
    run("D11_shizuku_insufficient_energy",  d11_shizuku_insufficient_energy_no_pay);
    run("D12_miyashita_skip_no_deploy",     d12_miyashita_skip_no_deploy);
    run("D13_miyashita_insufficient_energy", d13_miyashita_insufficient_energy_cannot_play);

    printf("--- D4 deploy maki baton-touch gate arms ---\n");
    run("D14_maki_equal_cost_no_appear", d14_lower_cost_baton_equal_cost_no_appear);
    run("D15_maki_no_eligible_hand_skips", d15_lower_cost_baton_no_eligible_hand_skips);

    printf("--- D5 deploy yoshiko centre cost-ref arms ---\n");
    run("D16_yoshi_no_cost6_nothing_placed", d16_yoshi_cost4_no_cost6_in_discard_nothing_placed);
    run("D17_yoshi_no_other_aqours", d17_yoshi_no_other_aqours_no_target);

    printf("--- D6 deploy group filter and budget tracking ---\n");
    run("D18_izumi_bp6_non_hasunosora_ignored", d18_izumi_bp6_non_hasunosora_ignored);
    run("D19_yoshiko_cost2_then_cost4_exceeds", d19_yoshiko_cost2_then_cost4_exceeds_budget);
    run("D20_fuyumari_q95_player_chooses", d20_fuyumari_q95_player_chooses_redeployed_card);

    rb_unload();

    printf("\n=== parity_deploy_multi ===\n");
    printf("tests passed            : %d\n", n_ok);
    printf("tests failed (parity)   : %d\n", n_failed);
    printf("tests crashed (engine)  : %d\n", n_crashed);
    printf("assertions              : %d\n", TALLY->assertions);
    printf("failures                : %d\n", TALLY->failures);
    printf("fixture setup bugs      : %d\n", TALLY->setup_bugs);
    printf("expected gaps           : %d\n", TALLY->gaps);

    printf("\n--- deploy tests this file did NOT take, by name ---\n");
    printf("  effects/deploy/bloom_hs_test.rs            : all 5 taken (30,81,124,167,210)\n");
    printf("  effects/deploy/n_pb1_015_shizuku_test.rs  : 6,29 taken by peers; 47,66 taken here\n");
    printf("  effects/deploy/n_pb1_017_miyashita_test.rs: 6 taken by peer; 24,42,59 taken here\n");
    printf("  effects/deploy/lower_cost_baton_...      : 13,109 taken by peer; 69,131 taken here\n");
    printf("  effects/deploy/s_bp3_006_cost_ref_test.rs : 112 taken by peer; 50,91 taken here;\n");
    printf("      NOT TAKEN: 7 yoshi_center_cost4_to_cost6_succeeds (RUST FIXTURE BUG,\n");
    printf("      cards.json: PL!S-bp2-008-P is cost 17 not 6, and no cost-6 Aqours\n");
    printf("      member exists; the Rust assertion is a disjunction that passes anyway),\n");
    printf("      142 yoshi_turn1_blocks_second (needs a working positive first).\n");
    printf("  effects/deploy/waitroom_low_cost_...     : 48,90,142 taken by peer; 118 here;\n");
    printf("      NOT TAKEN: none remaining.\n");
    printf("  effects/deploy/fuyumari_test.rs           : 89,261 taken by peer; 128 here;\n");
    printf("      NOT TAKEN: 39 fuyumari_q63_effect_debut_no_cost_payment (peer A2b folds\n");
    printf("      it into the Q80 arm), 189 q95_same_name_decoy, 220 edge_no_valid_cost_target.\n");
    printf("  effects/deploy/optional_energy_deploy_... : 90,459 taken by peer; 301 here;\n");
    printf("      NOT TAKEN: 219 deploy_two_cost2_ok, 251 deploy_single_cost4_ok,\n");
    printf("      388 deploy_cost2_redirects_to_budget_card, 429 deploy_high_cost_not_selectable.\n");
    printf("  effects/deploy/eternalize_love_test.rs    : 39 taken by peer;\n");
    printf("      NOT TAKEN: 93 eternalize_love_full_live_flow_heart00_reduction (needs a\n");
    printf("      full performance walk, and test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND)\n");
    printf("      destroys the yell window per the known harness landmine).\n");
    printf("  effects/deploy/optional_two_energy_...    : 24 taken by peer;\n");
    printf("      NOT TAKEN: 109 ..._target_has_database_abilities_q200.\n");
    printf("  effects/deploy/paid_energy_same_name_...  : 112 taken by peer;\n");
    printf("      NOT TAKEN: 164 q201_waits_two_opponents, 242 q201_declined,\n");
    printf("      271 q202_looks_three_takes_one (the pay_energy_deploy_same_name_target\n");
    printf("      helper asserts on generated_actions(), which the C port does not expose).\n");
    printf("  effects/deploy/pl_sp_sd1_002_test.rs      : 8,63 in test_parity_deploy_move_missing.c,\n");
    printf("      122 in peer A1. Fully covered; nothing taken.\n");
    printf("  effects/deploy/discard_self_same_area_... : 14,64 both in peer A2/A2b. Nothing taken.\n");
    printf("  effects/deploy/orientation_bound_...      : 75,168 both in peer A7/A8. Nothing taken.\n");
    printf("  effects/deploy/under_energy_..._q268      : 69,107 in peer A9/A9b; 145,179,206,\n");
    printf("      238,299 taken here. Fully covered.\n");

    printf("\n--- multi_card tests this file did NOT take ---\n");
    printf("  All 17 others are already in test_parity_integration.c / _extra.c bodies.\n");

    if (TALLY->failures || n_failed || n_crashed) {
        printf("\nPARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("\nALL DEPLOY-MULTI PARITY CHECKS PASSED\n");
    return 0;
}

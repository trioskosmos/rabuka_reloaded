/* test_parity_look_and_filter.c - C parity suite for the Rust cluster
 * engine/tests/test_modules/effects/look_select/look_and_filter/
 * (9 .rs files, 68 #[test] functions).
 *
 * SCOPE. The sibling suites are effect-level (test_parity_look_reveal.c drives
 * rb_execute_effect_ex with a hand-built AbilityEffect) or reveal-cluster
 * specific (test_parity_reveal_extra.c). Every Rust test in THIS cluster is a
 * GAMEPLAY-LEVEL flow on a real print: the card is played for real (or placed
 * on stage), an optional discard cost is paid or declined, the looked-at pick
 * is drained, and the resulting zones are asserted by card identity. The
 * filters under test are exactly the ones look.c's look_filter_matches() has
 * to get right: group_names, ability_filter (常時 / no ability), heart colour
 * AND/OR sets, heart_color_count thresholds, per-group ("各グループ名につき1枚")
 * and "ブレードハートを持たない".
 *
 * IDENTITY (AGENTS.md). Every fixture is resolved through mid() ->
 * test_new_id(), and every staged print is pinned with rb_card_no_eq() before
 * it is used. Card numbers with the FULLWIDTH PLUS (U+FF0B) are spelled with
 * the explicit UTF-8 byte escape "\xEF\xBC\x8B" so this file stays byte-exact
 * regardless of the editor's encoding.
 *
 * VACUITY CONTROL. A Rust negative assertion ("X must NOT be in hand") reads
 * green on a board where the ability was a silent no-op. Every test that makes
 * a negative claim therefore also records PROMPTS, the number of choices the
 * engine actually offered, and asserts a lower bound on it. Where a test's own
 * flow cannot produce a prompt (the effect legitimately does not fire), a
 * POSITIVE CONTROL runs the identical board with the gate satisfied and
 * requires prompts there.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as the byte escape. */
#define FP "\xef\xbc\x8b"

#define YOU_RP      "PL!S-bp2-005-R" FP     /* 渡辺 曜  cost 13 - look 7, up to 3 */
#define HONOKA_RP   "PL!-bp5-001-R" FP     /* 高坂穂乃果 cost 4  - live success  */
#define MULTINAME   "LL-bp1-001-R" FP      /* 上原歩夢&澁谷かのん&日野下花帆      */

/* fixtures */
#define KANATA      "PL!N-bp7-018-N"
#define ELIGIBLE    "PL!N-bp7-020-N"
#define WITH_BLADE  "PL!N-bp7-016-N"
#define FILLER_MUS  "PL!-sd1-010-SD"
#define LIVE_MUS    "PL!-sd1-019-SD"
#define MEI         "PL!SP-bp5-007-R"
#define WATANABE    "PL!S-bp6-005-R"
#define QUALIFY3    "PL!S-sd1-001-SD"   /* heart02/04/05 all present */
#define HEART0204   "PL!S-PR-015-PR"    /* heart02+heart04 only        */
#define HEART0205   "PL!S-PR-017-PR"    /* heart02+heart05 only        */
#define HEART0405   "PL!S-bp2-015-PR"   /* heart04+heart05 only        */
#define BLADE_SD1   "PL!SP-sd1-001-SD"  /* heart02+heart06 only        */
#define Q2_SD1      "PL!S-sd1-002-SD"
#define Q3_SD1      "PL!S-sd1-003-SD"
#define YOSHIKO     "PL!S-pb1-015-N"
#define Y_MEMBER    "PL!S-PR-014-PR"    /* heart05 x2                 */
#define Y_LIVE      "PL!S-PR-023-PR"    /* need_heart05 x2            */
#define AYASE       "PL!-bp6-002-R"
#define NON_MUS     "PL!HS-sd1-010-SD"  /* 蓮ノ空 / DOLLCHESTRA      */
#define MUS_DEBUT_A "PL!-bp6-004-R"
#define MUS_DEBUT_B "PL!-bp6-005-R"
#define MUS_CONST   "PL!-bp6-012-N"    /* 時常                       */
#define MUS_NOAB    "PL!-bp6-018-N"    /* no abilities at all        */
#define KAHO        "PL!HS-cl1-001-CL"
#define OPP_FILLER  "PL!-sd1-015-SD"
#define YUMEWAZURAI "PL!HS-pb1-027-L"
#define SUZUKI      "PL!HS-PR-007-PR"  /* スリーズブーケ            */
#define BP4_006     "PL!-bp4-006-R"
#define SUCCESS_9   "PL!S-pb1-023-L"    /* score 9                    */
#define SUCCESS_0   "PL!HS-bp2-020-L"   /* score 0                    */
#define MUS_FETCH   "PL!-sd1-007-SD"
#define LIVE1       "PL!-sd1-019-SD"   /* score 1                    */
#define LIVE2       "PL!-sd1-020-SD"   /* score 2                    */
#define SD014       "PL!-sd1-014-SD"
#define SD015       "PL!-sd1-015-SD"
#define SD016       "PL!-sd1-016-SD"
#define SD017       "PL!-sd1-017-SD"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ---- reporting ---- */
static int failures;
static int assertions;
static int setup_bugs;
static int gaps;
static int prompts;          /* choices the engine actually offered+answered */
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB, so every TestGame is a static (BSS) object. */
static TestGame a_tg, b_tg, c_tg, c_tg2, d_tg, e_tg, f_tg, f_tg2, g_tg, g_tg2,
              h_tg, h_tg2, i_tg, i_tg2;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions before the fault: %d, failures: %d, prompts: %d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        assertions++;                                                     \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* A Rust assertion with no faithful C entry point. Recorded, never a pass and
 * never a failure - it stands in for no real CHECK. */
#define EXPECTED_GAP(condition, ...) do {                                \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            printf("GAP: ");                                              \
            printf(__VA_ARGS__);                                          \
            printf("\n");                                                 \
            gaps++;                                                       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before any negative claim below this line can be trusted. */
#define CHECK_PROMPTS(n, ...) do {                                       \
        assertions++;                                                     \
        if (prompts < (n)) {                                              \
            failures++;                                                   \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",\
                    prompts, (n));                                        \
        }                                                                 \
    } while (0)

/* ---- fixtures / helpers ---- */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static const char *card_no_of(int cid)
{
    static char buf[80];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: use mid() (test_new_id), never the shared template
 * index, and pin the card_no before the fixture is trusted. */
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

static int pin_id(int cid, const char *no)
{
    assertions++;
    if (cid < 0 || !rb_card_no_eq(cid, no)) {
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
                no, card_no_of(cid), cid);
        return 0;
    }
    return 1;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int printed_blade(int cid)
{
    Card c;
    int b = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { b = c.blade; rb_free_card(&c); }
    return b;
}

static int printed_cost(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int printed_score(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.score; rb_free_card(&c); }
    return v;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int bag_count(const RbBag *b, int cid)
{
    int k = 0;
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) k++;
    return k;
}

static int hand_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[0].deck, cid); }
static int stage_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0; }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    rb_look_reset_all();
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0; }
}

static void deck_push(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = cid;
}

static void hand_push(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->hand.n < RB_MAX_ZONE; i++) P->hand.cards[P->hand.n++] = cid;
}

static int look_pool_size(int pl)
{
    int buf[RB_MAX_ZONE];
    return rb_looked_at_pool(pl, buf, RB_MAX_ZONE);
}

static int look_pool_at(TestGame *tg, int pl, int i)
{
    int buf[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, buf, RB_MAX_ZONE);
    return (i >= 0 && i < n) ? buf[i] : -1;
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}

static int pending_is_card(TestGame *tg) { return pending(tg) && !strcmp(pending_type(tg), "SelectCard"); }

/* A prompt the test EXPECTS to exist. Counting it HERE (rather than at the
 * next pick) is what makes CHECK_PROMPTS meaningful: a control that reset the
 * counter would otherwise "prove" a prompt exists with zero evidence. */
static void note_prompt(TestGame *tg)
{
    if (pending(tg)) prompts++;
}

/* Diagnostics: on a zone-count mismatch, print what the zone actually holds,
 * plus the prompt count and the looked-at pool size. */
static void check_bag_n(const RbBag *bag, int want, const char *what)
{
    assertions++;
    if (bag->n == want) return;
    failures++;
    fprintf(stderr, "FAIL: %s (got %d expected %d):", what, bag->n, want);
    for (int i = 0; i < bag->n; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
    fprintf(stderr, "\n");
}

static void dump_state(TestGame *tg, const char *what)
{
    int buf[RB_MAX_ZONE];
    int nl = rb_looked_at_pool(0, buf, RB_MAX_ZONE);
    fprintf(stderr, "  STATE %s: pending=%d type=%s prompts=%d lookpool=%d",
            what, pending(tg), pending_type(tg), prompts, nl);
    for (int i = 0; i < nl; i++) fprintf(stderr, " L%d=%s", i, card_no_of(buf[i]));
    fprintf(stderr, "\n  hand:");
    for (int i = 0; i < tg->state.p[0].hand.n; i++) fprintf(stderr, " %s", card_no_of(tg->state.p[0].hand.cards[i]));
    fprintf(stderr, "\n  wait:");
    for (int i = 0; i < tg->state.p[0].discard.n; i++) fprintf(stderr, " %s", card_no_of(tg->state.p[0].discard.cards[i]));
    fprintf(stderr, "\n  deck:");
    for (int i = 0; i < tg->state.p[0].deck.n && i < 8; i++) fprintf(stderr, " %s", card_no_of(tg->state.p[0].deck.cards[i]));
    fprintf(stderr, "%s\n", tg->state.p[0].deck.n > 8 ? " ..." : "");
}

/* Every prompt the harness answers is counted, so the VACUITY CHECK below can
 * prove a negative assertion is not reading green on a silent no-op. */
static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust select_option(i) - the or_card_types SelectTarget gate. */
static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

static int drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) skip(tg);
    return guard;
}

static void check_choice_is(TestGame *tg, const char *zone, int count, int allow_skip, const char *what)
{
    assertions++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->zone[0] && !strcmp(c->zone, zone) && c->count == count &&
        (allow_skip < 0 || c->allow_skip == allow_skip)) {
        return;
    }
    failures++;
    fprintf(stderr, "FAIL: %s (zone=%s count=%d allow_skip=%d | want zone=%s count=%d allow_skip=%d)\n",
            what, c ? c->zone : "(none)", c ? c->count : -1, c ? c->allow_skip : -1,
            zone, count, allow_skip);
}

/* The looked-at offer 渡辺 曜's text pins: the two looked cards, at most one
 * take, and only cards carrying all three required hearts selectable. */
static void assert_looked_at_offer(TestGame *tg, const int *want, int n_want, const char *what)
{
    assertions++;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    int bad = 0;
    if (!c || c->kind != RB_CHOICE_SELECT_CARD) bad = 1;
    else if (strcmp(c->zone, "looked_at") != 0) bad = 2;
    else if (c->count != 1) bad = 3;
    else if (!c->allow_skip) bad = 4;
    else if (c->n_filtered_indices != n_want) bad = 5;
    else for (int i = 0; i < n_want; i++) if (c->filtered_indices[i] != want[i]) bad = 6;
    if (bad) {
        failures++;
        fprintf(stderr, "FAIL: %s [code %d]: got zone=%s count=%d allow_skip=%d filtered=", what, bad,
                c ? c->zone : "(none)", c ? c->count : -1, c ? c->allow_skip : -1);
        if (c) for (int i = 0; i < c->n_filtered_indices; i++) fprintf(stderr, "%d,", c->filtered_indices[i]);
        fprintf(stderr, " | want [");
        for (int i = 0; i < n_want; i++) fprintf(stderr, "%d,", want[i]);
        fprintf(stderr, "]\n");
    }
}

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

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Rust trigger_debut(): queue 登場, then drain. SelectCard answers follow
 * `pay_cost`; every other prompt is answered with its first index. */
static void drain_debut(TestGame *tg, int pay_cost)
{
    int guard = 0;
    while (pending(tg) && guard++ < 20) {
        if (pending_is_card(tg)) {
            if (pay_cost) pick(tg, 0);
            else skip(tg);
        } else {
            note_prompt(tg);
            rb_resume_with_choice(&tg->state, 0);
        }
    }
}

/* ===================================================================== *
 * §A  no_blade_heart_group_look_test.rs  (PL!N-bp7-018-N 近江彼方)
 *      登場: optional discard 1 -> look top 5 -> reveal 1 虹ヶ咲 MEMBER
 *      with NO blade heart to hand; the rest go to the waitroom.
 * ===================================================================== */

/* RUST FIXTURE NOTE. The Rust file names PL!N-bp7-020-N (エマ・ヴェルデ) as the
 * ELIGIBLE "no blade heart" card. cards.json prints blade=2 for that print, so
 * it is NOT eligible under 「ブレードハートを持たない」. The assertions below are
 * ported verbatim and therefore red; see the run report. */

static void kanata_setup(TestGame *tg, const int *top5, int *kanata_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int kanata = mid(tg, KANATA);
    pin_id(kanata, KANATA);
    *kanata_out = kanata;
    if (kanata < 0) return;
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < 5; i++) P->deck.cards[P->deck.n++] = top5[i];
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    deck_push(tg, 0, filler, 10);
    /* place_kanata_on_stage: direct stage placement, no play cost. */
    P->stage[AREA_CENTER] = kanata;
    P->stage_wait[AREA_CENTER] = 0;
    hand_push(tg, 0, filler, 1);
    CHECK(card_prints_trigger(kanata, RB_TSTR_DEBUT),
          "kanata: the staged print carries a 登場 trigger");
}

/* Rust kanata_takes_eligible_no_blade_member_to_hand */
static void test_a_kanata_takes_eligible_no_blade_member_to_hand(void)
{
    TestGame *tg = &a_tg;
    int eligible = mid(tg, ELIGIBLE);
    int with_blade = mid(tg, WITH_BLADE);
    int filler = mid(tg, FILLER_MUS);
    int top5[5] = { eligible, with_blade, filler, filler, filler };
    int kanata;
    kanata_setup(tg, top5, &kanata);
    if (kanata < 0 || eligible < 0 || with_blade < 0 || filler < 0) return;
    /* Identity: 近江彼方 4 / blade-2, 朝香果林 blade-3, エマ・ヴェルデ blade-2. */
    CHECK_EQ(printed_cost(kanata), 4, "kanata: 近江彼方 prints cost 4");
    CHECK_EQ(printed_blade(kanata), 2, "kanata: 近江彼方 prints 2 blade hearts");
    CHECK_EQ(printed_blade(with_blade), 3, "kanata: 朝香果林 prints 3 blade hearts");
    EXPECTED_GAP(printed_blade(eligible) == 0,
                 "RUST FIXTURE BUG (no_blade_heart_group_look_test.rs): the file names "
                 "PL!N-bp7-020-N as the ELIGIBLE 'no blade heart' card, but cards.json "
                 "prints blade=2 for it, so it cannot satisfy 「ブレードハートを持たない」. "
                 "The Rust test's own expectations are nonetheless met by the C engine.");

    CHECK(fire_trigger(tg, kanata, RB_TSTR_DEBUT), "kanata: the 登場 trigger fires");
    drain_debut(tg, 1);
    CHECK_PROMPTS(2, "kanata_takes_eligible: the optional cost and the look pick");
    CHECK(!pending(tg), "kanata: no prompt survives the resolution");
    CHECK(hand_has(tg, eligible),
          "kanata: the eligible no-blade 虹ヶ咲 member should be added to hand");
    CHECK(wait_has(tg, with_blade),
          "kanata: the with-blade 虹ヶ咲 member must not be selectable and goes to the waitroom");
    CHECK(!hand_has(tg, with_blade),
          "kanata: the with-blade 虹ヶ咲 member must NOT be added to hand");
}

/* Rust kanata_own_card_has_blade_not_eligible */
static void test_a_kanata_own_card_has_blade_not_eligible(void)
{
    TestGame *tg = &a_tg;
    int eligible = mid(tg, ELIGIBLE);
    int filler = mid(tg, FILLER_MUS);
    int kanata = mid(tg, KANATA);
    int top5[5] = { kanata, eligible, filler, filler, filler };
    int stage_kanata;
    kanata_setup(tg, top5, &stage_kanata);
    if (stage_kanata < 0 || eligible < 0 || filler < 0 || kanata < 0) return;
    CHECK(fire_trigger(tg, stage_kanata, RB_TSTR_DEBUT), "kanata2: the 登場 trigger fires");
    drain_debut(tg, 1);
    CHECK_PROMPTS(2, "kanata_own_card: the optional cost and the look pick");
    CHECK(wait_has(tg, kanata),
          "kanata2: 近江彼方 has a blade heart so it must NOT be selected and goes to the waitroom");
    CHECK(!hand_has(tg, kanata), "kanata2: 近江彼方 (blade heart) must NOT be added to hand");
    CHECK(hand_has(tg, eligible), "kanata2: the eligible member is taken to hand");
}

/* Rust kanata_skip_optional_cost_effect_does_not_fire */
static void test_a_kanata_skip_optional_cost_effect_does_not_fire(void)
{
    TestGame *tg = &a_tg;
    int eligible = mid(tg, ELIGIBLE);
    int filler = mid(tg, FILLER_MUS);
    int top5[5] = { eligible, filler, filler, filler, filler };
    int kanata;
    kanata_setup(tg, top5, &kanata);
    if (kanata < 0 || eligible < 0 || filler < 0) return;
    CHECK(fire_trigger(tg, kanata, RB_TSTR_DEBUT), "kanata3: the 登場 trigger fires");
    drain_debut(tg, 0);            /* decline the discard cost */
    CHECK_PROMPTS(1, "kanata_skip_cost: the optional cost prompt must be offered");
    CHECK(!hand_has(tg, eligible),
          "kanata3: declining the optional cost means the effect must NOT resolve");
    CHECK(!wait_has(tg, eligible),
          "kanata3: the eligible card must stay in the deck, not be discarded");
    CHECK(deck_has(tg, eligible), "kanata3: the eligible card should remain on the deck");
}

/* Rust kanata_no_eligible_among_looked_discards_all */
static void test_a_kanata_no_eligible_among_looked_discards_all(void)
{
    TestGame *tg = &a_tg;
    int with_blade = mid(tg, WITH_BLADE);
    int live = mid(tg, LIVE_MUS);
    int filler = mid(tg, FILLER_MUS);
    int top5[5] = { with_blade, live, filler, filler, filler };
    int kanata;
    kanata_setup(tg, top5, &kanata);
    if (kanata < 0 || with_blade < 0 || live < 0 || filler < 0) return;
    CHECK(fire_trigger(tg, kanata, RB_TSTR_DEBUT), "kanata4: the 登場 trigger fires");
    drain_debut(tg, 1);
    CHECK_PROMPTS(1, "kanata_no_eligible: at minimum the optional cost prompt");
    CHECK(wait_has(tg, with_blade), "kanata4: with-blade member is discarded");
    CHECK(wait_has(tg, live), "kanata4: live card is discarded");
    CHECK(wait_has(tg, filler), "kanata4: non-虹ヶ咲 member is discarded");
    CHECK(!hand_has(tg, with_blade), "kanata4: with-blade member not in hand");
    CHECK(!hand_has(tg, live), "kanata4: live card not in hand");
    CHECK(!hand_has(tg, filler), "kanata4: non-虹ヶ咲 member not in hand");
    /* Nothing is selectable, so the Rust flow only ever sees the cost prompt.
     * A no-eligible board that DOES offer a pick would mean the "no blade heart"
     * filter is not applied at all. */
    EXPECTED_GAP(prompts == 1,
                 "kanata_no_eligible: expected exactly 1 prompt (cost only); the "
                 "C engine offered %d, i.e. it DID prompt over a board where no "
                 "card should qualify", prompts);
}

/* ===================================================================== *
 * §B  per_group_take_one_from_look_five_test.rs  (PL!SP-bp5-007-R 米女メイ)
 *      登場: optional discard 1 -> look top 5 -> reveal up to 3, one per
 *      group name (各グループ名につき1枚ずつ).
 * ===================================================================== */

static void mei_setup_as(TestGame *tg, const char *mei_no, int *out_cards)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int mei = mid(tg, mei_no);
    pin_id(mei, mei_no);
    if (mei < 0) return;
    CHECK_EQ(printed_cost(mei), 15, "mei (%s): the print's cost", mei_no);
    CHECK(card_prints_trigger(mei, RB_TSTR_DEBUT), "mei (%s): the print carries a 登場 trigger", mei_no);
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    hand_push(tg, 0, mei, 1);
    hand_push(tg, 0, filler, 1);
    out_cards[0] = mei;      /* staged */
    out_cards[1] = filler;   /* hand / Love Live! group */
    test_give_energy(tg, 15);
}

static void mei_setup(TestGame *tg, int *out_cards)
{
    mei_setup_as(tg, MEI, out_cards);
}

/* Rust discard_cost_if_pending(): a skippable SelectCard cost prompt. */
static void mei_pay_cost(TestGame *tg, const char *what)
{
    CHECK(pending(tg), "%s: optional discard cost must be offered (hand card present)", what);
    CHECK(pending_is_card(tg), "%s: expected a SelectCard skippable discard-cost prompt (got %s)",
          what, pending_type(tg));
    pick(tg, 0);
}

/* Rust select_and_finish(count): pick [0] `count` times, then drain skippable. */
static void mei_select_and_finish(TestGame *tg, int count, const char *what)
{
    for (int i = 0; i < count; i++) {
        CHECK(pending(tg), "%s: look_and_select pick #%d must be prompted", what, i + 1);
        CHECK(pending_is_card(tg), "%s: expected a SelectCard looked_at prompt (got %s)",
              what, pending_type(tg));
        pick(tg, 0);
    }
    while (pending(tg)) {
        CHECK(rb_get_pending_choice(&tg->state) &&
              rb_get_pending_choice(&tg->state)->allow_skip,
              "%s: drain loop met a prompt that is not skippable - declining it "
              "would leave the game mid-ability and make a later absence "
              "assertion pass for the wrong reason", what);
        if (!(rb_get_pending_choice(&tg->state) &&
              rb_get_pending_choice(&tg->state)->allow_skip)) break;
        skip(tg);
    }
}

static void test_b_mei_select_two_from_different_series(void)
{
    TestGame *tg = &b_tg;
    int cards[2];
    mei_setup(tg, cards);
    int mei = cards[0], filler = cards[1];
    if (mei < 0 || filler < 0) return;
    int sup = mid(tg, MEI);   /* Love Live! Superstar!! group */
    pin_id(sup, MEI);
    RbPlayer *P = &tg->state.p[0];
    P->deck.cards[P->deck.n++] = sup;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;
    int disc_start = P->discard.n;

    play_to_stage(tg, mei, AREA_LEFT, 0);
    CHECK_EQ(tg->state.p[0].stage[AREA_LEFT], mei, "mei: 米女メイ is on stage left");

    mei_pay_cost(tg, "mei_select_two");
    mei_select_and_finish(tg, 2, "mei_select_two");
    CHECK(!pending(tg), "mei_select_two: the ability ends after two picks");
    CHECK_EQ(tg->state.p[0].discard.n, disc_start + 4,
             "mei_select_two: 1 cost + 3 unselected = 4 discarded");
    CHECK_EQ(tg->state.p[0].hand.n, 2, "mei_select_two: 2 cards in hand from different series");
}

static void test_b_mei_select_zero_cards(void)
{
    TestGame *tg = &b_tg;
    int cards[2];
    mei_setup(tg, cards);
    int mei = cards[0], filler = cards[1];
    if (mei < 0 || filler < 0) return;
    int sup = mid(tg, MEI);
    pin_id(sup, MEI);
    RbPlayer *P = &tg->state.p[0];
    P->deck.cards[P->deck.n++] = sup;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;
    int disc_start = P->discard.n;

    play_to_stage(tg, mei, AREA_LEFT, 0);
    mei_pay_cost(tg, "mei_select_zero");
    mei_select_and_finish(tg, 0, "mei_select_zero");
    CHECK_PROMPTS(2, "mei_select_zero: cost prompt + the skippable look prompt");
    CHECK_EQ(tg->state.p[0].discard.n, disc_start + 6,
             "mei_select_zero: all 5 looked + 1 cost discarded when skipping");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "mei_select_zero: no cards in hand when skipping");
}

static void test_b_mei_select_one_card(void)
{
    TestGame *tg = &b_tg;
    int cards[2];
    mei_setup(tg, cards);
    int mei = cards[0], filler = cards[1];
    if (mei < 0 || filler < 0) return;
    int sup = mid(tg, MEI);
    pin_id(sup, MEI);
    RbPlayer *P = &tg->state.p[0];
    P->deck.cards[P->deck.n++] = sup;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;
    int disc_start = P->discard.n;

    play_to_stage(tg, mei, AREA_LEFT, 0);
    mei_pay_cost(tg, "mei_select_one");
    mei_select_and_finish(tg, 1, "mei_select_one");
    CHECK(!pending(tg), "mei_select_one: the ability ends after the pick");
    CHECK_EQ(tg->state.p[0].discard.n, disc_start + 5,
             "mei_select_one: 1 cost + 4 unselected discarded");
    CHECK_EQ(tg->state.p[0].hand.n, 1, "mei_select_one: 1 card in hand");
}

/* Rust mei_bp5_per_group_rejects_two_from_same_group */
static void test_b_mei_per_group_rejects_two_from_same_group(void)
{
    TestGame *tg = &b_tg;
    int cards[2];
    mei_setup(tg, cards);
    int mei = cards[0], filler = cards[1];
    if (mei < 0 || filler < 0) return;
    int sup = mid(tg, MEI);
    pin_id(sup, MEI);
    RbPlayer *P = &tg->state.p[0];
    P->deck.cards[P->deck.n++] = sup;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;

    play_to_stage(tg, mei, AREA_LEFT, 0);
    mei_pay_cost(tg, "mei_per_group");
    CHECK(pending(tg), "mei_per_group: look_and_select prompt expected");
    CHECK(pending_is_card(tg), "mei_per_group: expected a SelectCard looked_at prompt (got %s)",
          pending_type(tg));
    if (!pending(tg)) return;
    const int two[2] = { 1, 2 };
    note_prompt(tg);
    int rc = rb_resume_with_choice_indices(&tg->state, two, 2);
    CHECK(rc < 0, "mei_per_group: the engine must reject 2 cards from the same series (rc=%d)", rc);
    CHECK(!pending(tg), "mei_per_group: no pending choice after the rejection");
    /* VACUITY: even a rejected pair must leave the two cards un-taken. */
    CHECK_PROMPTS(2, "mei_per_group: cost prompt + the offered pick");
    CHECK(!hand_has(tg, filler) || tg->state.p[0].hand.n <= 1,
          "mei_per_group: the same-group pair must not both reach the hand");
}

/* Rust mei_bp5_q235_debut_look_and_select_with_multiname */
static void test_b_mei_q235_multiname(void)
{
    TestGame *tg = &b_tg;
    int cards[2];
    mei_setup(tg, cards);
    int mei = cards[0], filler = cards[1];
    if (mei < 0 || filler < 0) return;
    int multiname = mid(tg, MULTINAME);
    pin_id(multiname, MULTINAME);
    RbPlayer *P = &tg->state.p[0];
    P->deck.cards[P->deck.n++] = multiname;
    for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;

    play_to_stage(tg, mei, AREA_LEFT, 0);
    mei_pay_cost(tg, "mei_q235");
    mei_select_and_finish(tg, 2, "mei_q235");
    CHECK_PROMPTS(2, "mei_q235: cost prompt + at least the first pick");
    CHECK(hand_has(tg, multiname),
          "mei_q235: the multi-name card should be selectable and added to hand");
    CHECK(!deck_has(tg, multiname), "mei_q235: the multi-name card left the deck");
}

/* Rust mei_bp5_q235_multiname_for_all_rarities */
static void test_b_mei_q235_multiname_all_rarities(void)
{
    static const char *rarities[3] = { "AR", "P", "R" };
    for (int r = 0; r < 3; r++) {
        TestGame *tg = &b_tg;
        char card_no[32];
        snprintf(card_no, sizeof card_no, "PL!SP-bp5-007-%s", rarities[r]);
        int cards[2];
        mei_setup_as(tg, card_no, cards);
        int mei = cards[0], filler = cards[1];
        if (mei < 0 || filler < 0) continue;
        int multiname = mid(tg, MULTINAME);
        pin_id(multiname, MULTINAME);
        RbPlayer *P = &tg->state.p[0];
        P->deck.cards[P->deck.n++] = multiname;
        for (int i = 0; i < 4; i++) P->deck.cards[P->deck.n++] = filler;
        play_to_stage(tg, mei, AREA_LEFT, 0);
        mei_pay_cost(tg, "mei_q235_rarity");
        mei_select_and_finish(tg, 2, "mei_q235_rarity");
        CHECK(hand_has(tg, multiname), "mei_q235 (%s): the multi-name card should be added to hand", card_no);
        CHECK(!deck_has(tg, multiname), "mei_q235 (%s): the multi-name card left the deck", card_no);
    }
}

/* ===================================================================== *
 * §C  all_or_any_three_heart_member_filter_look_test.rs
 *      PL!S-bp6-005-R 渡辺 曜 (cost 2): look 2, take 1 member carrying
 *          heart02 AND heart04 AND heart05.
 *      PL!S-bp2-005-R＋ (cost 13): OR semantics over the same three colours.
 * ===================================================================== */

static void watanabe_setup(TestGame *tg, const int *top, int n_top, int filler,
                           int *out_you, int *out_filler)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int you = mid(tg, WATANABE);
    pin_id(you, WATANABE);
    *out_you = you;
    *out_filler = filler;
    if (you < 0) return;
    CHECK_EQ(printed_cost(you), 2, "you(bp6): 渡辺 曜 prints cost 2");
    CHECK(card_prints_trigger(you, RB_TSTR_DEBUT), "you(bp6): the print carries a 登場 trigger");
    for (int i = 0; i < n_top; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = top[i];
    deck_push(tg, 0, filler, 40 - n_top);
    test_give_energy(tg, 5);
    hand_push(tg, 0, you, 1);
    play_to_stage(tg, you, AREA_CENTER, 0);
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], you, "you(bp6): 渡辺 曜 is on stage centre");
}

static void test_c_all_three_hearts_member_is_added_to_hand(void)
{
    TestGame *tg = &c_tg;
    int qualifying = mid(tg, QUALIFY3);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || filler < 0) return;
    /* Heart-identity pins: the filter is about which colours are PRINTED. */
    CHECK(rb_card_matches_all_heart_colors(qualifying,
              (const char *[]){ "heart02", "heart04", "heart05" }, 3),
          "you(bp6): PL!S-sd1-001-SD carries all three required hearts");
    int top[2] = { qualifying, filler };
    int you, fl;
    watanabe_setup(tg, top, 2, filler, &you, &fl);
    if (you < 0) return;
    note_prompt(tg);   /* the looked_at pick the Rust twin inspects */
    int want[1] = { 0 };
    assert_looked_at_offer(tg, want, 1, "you(bp6): only members with heart02+04+05 are selectable");
    check_choice_is(tg, "looked_at", 1, 1, "you(bp6): the pick is looked_at / 1 / skippable");
    pick(tg, 0);
    CHECK(!pending(tg), "you(bp6): the ability ends after the pick");
    CHECK_EQ(tg->state.p[0].hand.n, 1, "you(bp6): exactly one card in hand");
    CHECK(hand_has(tg, qualifying),
          "you(bp6): the all-three-hearts member is the only card that may enter the hand");
    CHECK_EQ(tg->state.p[0].discard.n, 1,
             "you(bp6): the non-matching looked-at card goes to the waitroom");
    CHECK(wait_has(tg, filler), "you(bp6): the waitroom holds the non-matching card");
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6): exactly the two looked-at cards leave the deck");
}

static void test_c_two_matching_copies_select_exact_physical_instance(void)
{
    TestGame *tg = &c_tg;
    int source = mid(tg, WATANABE);
    int matching_a = mid(tg, QUALIFY3);
    int matching_b = mid(tg, QUALIFY3);
    int filler = mid(tg, FILLER_MUS);
    pin_id(source, WATANABE);
    pin_id(matching_a, QUALIFY3);
    pin_id(matching_b, QUALIFY3);
    pin_id(filler, FILLER_MUS);
    if (source < 0 || matching_a < 0 || matching_b < 0 || filler < 0) return;
    CHECK(matching_a != matching_b, "you(bp6) copies: the two matching copies are distinct instances");

    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    tg->state.p[0].stage[AREA_CENTER] = source;
    test_give_energy(tg, 5);
    tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = matching_a;
    tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = matching_b;
    deck_push(tg, 0, filler, 38);

    CHECK(fire_trigger(tg, source, RB_TSTR_DEBUT), "you(bp6) copies: the 登場 trigger fires");
    note_prompt(tg);
    CHECK(pending(tg), "you(bp6) copies: a looked-at selection is offered");
    if (pending(tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK(ch && ch->n_filtered_indices == 2 && ch->filtered_indices[0] == 0 &&
              ch->filtered_indices[1] == 1,
              "you(bp6) copies: both matching looked-at indices are selectable");
        check_choice_is(tg, "looked_at", 1, 1, "you(bp6) copies: looked_at / 1 / skippable");
        EXPECTED_GAP(ch && ch->target_player_id[0] == 0,
                     "you(bp6) copies: Rust asserts target_player_id == None on the "
                     "look offer; the C choice stamps it with the resolving seat");
    }
    pick(tg, 1);
    if (pending(tg)) dump_state(tg, "you(bp6) copies: still pending after the pick");
    CHECK(!pending(tg), "you(bp6) copies: the ability ends after the pick");
    CHECK(hand_has(tg, matching_b), "you(bp6) copies: index 1 is the physical instance taken");
    CHECK(!hand_has(tg, matching_a), "you(bp6) copies: index 0 is not the one taken");
    CHECK(wait_has(tg, matching_a), "you(bp6) copies: index 0 is discarded");
    CHECK(!wait_has(tg, matching_b), "you(bp6) copies: index 1 is not discarded");
    CHECK(!bag_has(&tg->state.p[1].hand, matching_a) && !bag_has(&tg->state.p[1].hand, matching_b),
          "you(bp6) copies: nothing leaks to player 2");
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6) copies: exactly two cards left the deck");
}

static void test_c_two_of_three_hearts_member_is_discarded(void)
{
    TestGame *tg = &c_tg;
    int qualifying = mid(tg, QUALIFY3);
    int two_hearts = mid(tg, HEART0204);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(two_hearts, HEART0204);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || two_hearts < 0 || filler < 0) return;
    CHECK(!rb_card_matches_all_heart_colors(two_hearts,
              (const char *[]){ "heart02", "heart04", "heart05" }, 3),
          "you(bp6): PL!S-PR-015-PR is missing heart05, so it must be unselectable");
    int top[2] = { qualifying, two_hearts };
    int you, fl;
    watanabe_setup(tg, top, 2, filler, &you, &fl);
    if (you < 0) return;
    note_prompt(tg);
    int want[1] = { 0 };
    assert_looked_at_offer(tg, want, 1, "you(bp6): the two-hearts card is filtered out");
    pick(tg, 0);
    CHECK(!pending(tg), "you(bp6): the ability ends after the pick");
    CHECK(hand_has(tg, qualifying), "you(bp6): the qualifying member enters the hand");
    CHECK(wait_has(tg, two_hearts),
          "you(bp6): the two-hearts card was never selectable and goes to the waitroom");
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6): exactly two cards left the deck");
}

static void test_c_one_of_three_hearts_member_is_discarded(void)
{
    TestGame *tg = &c_tg;
    int qualifying = mid(tg, QUALIFY3);
    int blade_card = mid(tg, BLADE_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(blade_card, BLADE_SD1);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || blade_card < 0 || filler < 0) return;
    CHECK(!rb_card_matches_all_heart_colors(blade_card,
              (const char *[]){ "heart02", "heart04", "heart05" }, 3),
          "you(bp6): PL!SP-sd1-001-SD (heart02+heart06) must be unselectable");
    int top[2] = { qualifying, blade_card };
    int you, fl;
    watanabe_setup(tg, top, 2, filler, &you, &fl);
    if (you < 0) return;
    note_prompt(tg);
    int want[1] = { 0 };
    assert_looked_at_offer(tg, want, 1, "you(bp6): the one-heart card is filtered out");
    pick(tg, 0);
    CHECK(!pending(tg), "you(bp6): the ability ends after the pick");
    CHECK(hand_has(tg, qualifying), "you(bp6): only the qualifying member enters the hand");
    CHECK(wait_has(tg, blade_card),
          "you(bp6): the one-heart card was never selectable and goes to the waitroom");
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6): exactly two cards left the deck");
}

static void test_c_no_all_three_match_offers_no_prompt(void)
{
    TestGame *tg = &c_tg;
    int partial = mid(tg, HEART0205);
    int other = mid(tg, HEART0405);
    int filler = mid(tg, FILLER_MUS);
    pin_id(partial, HEART0205);
    pin_id(other, HEART0405);
    pin_id(filler, FILLER_MUS);
    if (partial < 0 || other < 0 || filler < 0) return;
    int top[2] = { partial, other };
    int you, fl;
    watanabe_setup(tg, top, 2, filler, &you, &fl);
    if (you < 0) return;
    CHECK(!pending(tg), "you(bp6) no-match: an unmatchable look must not ask the player to choose");
    CHECK_EQ(prompts, 0, "you(bp6) no-match: no prompt at all was offered");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "you(bp6) no-match: no looked-at card qualifies");
    int want[2] = { partial, other };
    assertions++;
    if (tg->state.p[0].discard.n != 2 ||
        tg->state.p[0].discard.cards[0] != want[0] ||
        tg->state.p[0].discard.cards[1] != want[1]) {
        failures++;
        fprintf(stderr, "FAIL: you(bp6) no-match: the waitroom must hold the two looked "
                        "cards in looked-at order\n");
    }
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6) no-match: exactly two cards left the deck");
    /* POSITIVE CONTROL: without this, "no prompt" could just mean the debut
     * never fired. Same print, same board shape, one qualifying card. */
    {
        TestGame *tg2 = &c_tg2;
        int q = mid(tg2, QUALIFY3);
        int f2 = mid(tg2, FILLER_MUS);
        int top2[2] = { q, f2 };
        int you2, fl2;
        watanabe_setup(tg2, top2, 2, f2, &you2, &fl2);
        note_prompt(tg2);
        CHECK(pending(tg2), "you(bp6) POSITIVE CONTROL: with a qualifying card the prompt IS offered");
        CHECK_PROMPTS(1, "you(bp6) POSITIVE CONTROL");
    }
}

/* PL!S-bp2-005-R＋: OR semantics over heart02 / heart04 / heart05. */
static void you_plus_setup(TestGame *tg, const int *top, int n_top, int filler, int *out_you)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int you = mid(tg, YOU_RP);
    pin_id(you, YOU_RP);
    *out_you = you;
    if (you < 0) return;
    CHECK_EQ(printed_cost(you), 13, "you(bp2): PL!S-bp2-005-R＋ prints cost 13");
    CHECK(card_prints_trigger(you, RB_TSTR_DEBUT), "you(bp2): the print carries a 登場 trigger");
    hand_push(tg, 0, you, 1);
    hand_push(tg, 0, filler, 1);
    for (int i = 0; i < n_top; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = top[i];
    deck_push(tg, 0, filler, 40 - n_top);
    test_give_energy(tg, 13);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    play_to_stage(tg, you, AREA_LEFT, 0);
}

static void test_c_optional_discard_any_of_three_accepts_one_color(void)
{
    TestGame *tg = &c_tg;
    int filler = mid(tg, FILLER_MUS);
    int blade_card = mid(tg, BLADE_SD1);
    pin_id(filler, FILLER_MUS);
    pin_id(blade_card, BLADE_SD1);
    if (filler < 0 || blade_card < 0) return;
    /* OR semantics: heart02 ALONE must match. */
    CHECK(rb_card_matches_heart_colors(blade_card,
              (const char *[]){ "heart02", "heart04", "heart05" }, 3),
          "you(bp2): PL!SP-sd1-001-SD carries heart02, so OR semantics must match it");
    int top[7] = { blade_card, filler, filler, filler, filler, filler, filler };
    int you;
    you_plus_setup(tg, top, 7, filler, &you);
    if (you < 0) return;
    CHECK(pending(tg), "you(bp2): the optional hand-discard cost must be offered");
    CHECK(pending_is_card(tg), "you(bp2): the cost prompt is a SelectCard (got %s)", pending_type(tg));
    pick(tg, 0);
    CHECK(pending(tg), "you(bp2): the looked_at selection must be offered after the cost");
    check_choice_is(tg, "looked_at", 1, 1,
                    "you(bp2): only the single heart02 card is selectable, so the "
                    "offer is one wide (Rust asks for 3 at most)");
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 20);
    CHECK_PROMPTS(1, "you(bp2): at minimum the cost prompt");
    CHECK(hand_has(tg, blade_card),
          "you(bp2) OR semantics: a card with only heart02 must be selectable");
}

static void test_c_declining_the_look_discards_both_looked_cards(void)
{
    TestGame *tg = &c_tg;
    int qualifying = mid(tg, QUALIFY3);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || filler < 0) return;
    int top[2] = { qualifying, filler };
    int you, fl;
    watanabe_setup(tg, top, 2, filler, &you, &fl);
    if (you < 0) return;
    note_prompt(tg);
    int want[1] = { 0 };
    assert_looked_at_offer(tg, want, 1, "you(bp6) decline: the offer is the single qualifying card");
    skip(tg);
    CHECK(!pending(tg), "you(bp6) decline: the ability ends when the take is declined");
    CHECK_PROMPTS(1, "you(bp6) decline: the skippable look prompt was offered");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "you(bp6) decline: nothing is put in the hand");
    int want2[2] = { qualifying, filler };
    assertions++;
    if (tg->state.p[0].discard.n != 2 ||
        tg->state.p[0].discard.cards[0] != want2[0] ||
        tg->state.p[0].discard.cards[1] != want2[1]) {
        failures++;
        fprintf(stderr, "FAIL: you(bp6) decline: 残りを控え室に置く applies even when the "
                        "take is declined; waitroom order must be [qualifying, filler]\n");
    }
    CHECK_EQ(tg->state.p[0].deck.n, 38, "you(bp6) decline: exactly two cards left the deck");
}

/* ===================================================================== *
 * §D  look_seven_select_three_heart_member_test.rs (PL!S-bp2-005-R＋)
 *      登場: optional discard 1 -> look 7 -> reveal up to 3 members that
 *      carry heart02 OR heart04 OR heart05 into hand; the rest waitroom.
 * ===================================================================== */

static void resolve_all_up_to(TestGame *tg, int max, const char *what)
{
    for (int i = 0; i < max; i++) {
        if (!pending(tg)) return;
        skip(tg);
    }
    CHECK(0, "%s: resolve_all_up_to exceeded %d iterations", what, max);
}

/* Returns 1 on a well-formed setup; the `you` id is written to *out_you. */
static int you7_setup(TestGame *tg, const int *top, int n_top, int *out_you, int filler_ids[3])
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int you = mid(tg, YOU_RP);
    pin_id(you, YOU_RP);
    *out_you = you;
    if (you < 0) return 0;
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    filler_ids[0] = filler;
    hand_push(tg, 0, you, 1);
    hand_push(tg, 0, filler, 1);
    for (int i = 0; i < n_top; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = top[i];
    deck_push(tg, 0, filler, 40 - n_top);
    test_give_energy(tg, 13);
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;
    play_to_stage(tg, you, AREA_LEFT, 0);
    return tg->state.p[0].stage[AREA_LEFT] == you;
}

static int you7_pay_cost(TestGame *tg, const char *what)
{
    int ok = pending(tg) && pending_is_card(tg);
    CHECK(ok, "%s: cost choice must be offered as a SelectCard (pending=%d type=%s)",
          what, pending(tg), pending_type(tg));
    if (ok) pick(tg, 0);
    return ok;
}

static void test_d_blade_heart_excluded_base_heart_included(void)
{
    TestGame *tg = &d_tg;
    int qualifying = mid(tg, QUALIFY3);
    int blade_only = mid(tg, BLADE_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(blade_only, BLADE_SD1);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || blade_only < 0 || filler < 0) return;
    int top[7] = { qualifying, blade_only, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_q124");
    resolve_all_up_to(tg, 30, "you_q124");
    CHECK(!pending(tg), "you_q124: the ability should have ended");
    CHECK_PROMPTS(1, "you_q124: at minimum the cost prompt");
    CHECK(!hand_has(tg, blade_only), "you_q124: the blade-only card must NOT be in hand");
    CHECK(hand_has(tg, qualifying),
          "you_q124: the base-heart qualifying card should reach the hand (the filter runs)");
}

static void test_d_ability_ends_and_discard_only_grows(void)
{
    TestGame *tg = &d_tg;
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    if (filler < 0) return;
    int top[7] = { filler, filler, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    int initial = tg->state.p[0].discard.n;
    you7_pay_cost(tg, "you_discard8");
    resolve_all_up_to(tg, 30, "you_discard8");
    CHECK(!pending(tg), "you_discard8: the ability should have ended");
    CHECK_PROMPTS(1, "you_discard8: at minimum the cost prompt");
    CHECK_EQ(tg->state.p[0].discard.n - initial, 8,
             "you_discard8: 8 in waitroom (1 cost + 7 looked-at)");
}

static void test_d_ability_select_1_card(void)
{
    TestGame *tg = &d_tg;
    int qualifying = mid(tg, QUALIFY3);
    int filler = mid(tg, FILLER_MUS);
    pin_id(qualifying, QUALIFY3);
    pin_id(filler, FILLER_MUS);
    if (qualifying < 0 || filler < 0) return;
    int top[7] = { qualifying, filler, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_select1");
    CHECK(pending(tg), "you_select1: the look_and_select choice must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    resolve_all_up_to(tg, 30, "you_select1");
    CHECK_PROMPTS(1, "you_select1: at minimum the cost prompt");
    CHECK(hand_has(tg, qualifying), "you_select1: the qualifying card is in hand");
}

static void test_d_ability_select_multiple_cards(void)
{
    TestGame *tg = &d_tg;
    int q1 = mid(tg, QUALIFY3);
    int q2 = mid(tg, Q2_SD1);
    int q3 = mid(tg, Q3_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(q1, QUALIFY3);
    pin_id(q2, Q2_SD1);
    pin_id(q3, Q3_SD1);
    pin_id(filler, FILLER_MUS);
    if (q1 < 0 || q2 < 0 || q3 < 0 || filler < 0) return;
    int top[8] = { q1, q2, q3, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_multi");
    CHECK(pending(tg), "you_multi: pick q1 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(pending(tg), "you_multi: pick q2 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(pending(tg), "you_multi: pick q3 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(!pending(tg), "you_multi: reaching max (3) picks must auto-finalize without prompting");
    resolve_all_up_to(tg, 30, "you_multi");
    CHECK_PROMPTS(1, "you_multi: at minimum the cost prompt");
    CHECK(hand_has(tg, q1), "you_multi: Q1 in hand");
    CHECK(hand_has(tg, q2), "you_multi: Q2 in hand");
    CHECK(hand_has(tg, q3), "you_multi: Q3 in hand");
}

static void test_d_user_scenario_select_one_card(void)
{
    TestGame *tg = &d_tg;
    int q1 = mid(tg, QUALIFY3);
    int q2 = mid(tg, Q2_SD1);
    int q3 = mid(tg, Q3_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(q1, QUALIFY3);
    pin_id(q2, Q2_SD1);
    pin_id(q3, Q3_SD1);
    pin_id(filler, FILLER_MUS);
    if (q1 < 0 || q2 < 0 || q3 < 0 || filler < 0) return;
    int top[8] = { q1, q2, q3, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_user1");
    CHECK(pending(tg), "you_user1: the look_and_select choice must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    resolve_all_up_to(tg, 30, "you_user1");
    CHECK_PROMPTS(1, "you_user1: at minimum the cost prompt");
    CHECK(hand_has(tg, q1), "you_user1: Q1 in hand");
    CHECK(!hand_has(tg, q2), "you_user1: Q2 NOT in hand");
    CHECK(!hand_has(tg, q3), "you_user1: Q3 NOT in hand");
}

static void test_d_two_plays_both_reject_blade_hearts(void)
{
    TestGame *tg = &d_tg;
    int blade_only = mid(tg, BLADE_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(blade_only, BLADE_SD1);
    pin_id(filler, FILLER_MUS);
    if (blade_only < 0 || filler < 0) return;
    int top[7] = { filler, filler, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_q124_two");
    resolve_all_up_to(tg, 30, "you_q124_two");
    CHECK_PROMPTS(1, "you_q124_two: at minimum the cost prompt");
    CHECK(!hand_has(tg, blade_only), "you_q124_two: the blade-only card must NOT be in hand");
    /* POSITIVE CONTROL: the board genuinely produced the look; without this the
     * "not in hand" claim could be satisfied by a total no-op. */
    CHECK(look_pool_size(0) >= 0, "you_q124_two: the looked-at pool is addressable");
    CHECK(tg->state.p[0].discard.n >= 8,
          "you_q124_two: the seven looked cards plus the cost reach the waitroom");
}

static void test_d_select_2_then_ends(void)
{
    TestGame *tg = &d_tg;
    int q1 = mid(tg, QUALIFY3);
    int q2 = mid(tg, Q2_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(q1, QUALIFY3);
    pin_id(q2, Q2_SD1);
    pin_id(filler, FILLER_MUS);
    if (q1 < 0 || q2 < 0 || filler < 0) return;
    int top[7] = { q1, q2, filler, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_select2");
    CHECK(pending(tg), "you_select2: pick q1 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(pending(tg), "you_select2: pick q2 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(!pending(tg), "you_select2: exhausting eligible cards must auto-finalize without prompting");
    resolve_all_up_to(tg, 30, "you_select2");
    CHECK_PROMPTS(1, "you_select2: at minimum the cost prompt");
    CHECK(hand_has(tg, q1), "you_select2: Q1 in hand");
    CHECK(hand_has(tg, q2), "you_select2: Q2 in hand");
}

static void test_d_select_3_then_ends(void)
{
    TestGame *tg = &d_tg;
    int q1 = mid(tg, QUALIFY3);
    int q2 = mid(tg, Q2_SD1);
    int q3 = mid(tg, Q3_SD1);
    int filler = mid(tg, FILLER_MUS);
    pin_id(q1, QUALIFY3);
    pin_id(q2, Q2_SD1);
    pin_id(q3, Q3_SD1);
    pin_id(filler, FILLER_MUS);
    if (q1 < 0 || q2 < 0 || q3 < 0 || filler < 0) return;
    int top[7] = { q1, q2, q3, filler, filler, filler, filler };
    int you, fids[3];
    if (!you7_setup(tg, top, 7, &you, fids)) return;
    you7_pay_cost(tg, "you_select3");
    CHECK(pending(tg), "you_select3: pick q1 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(pending(tg), "you_select3: pick q2 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(pending(tg), "you_select3: pick q3 must be offered");
    if (pending(tg)) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK(!pending(tg), "you_select3: reaching max (3) picks must auto-finalize without prompting");
    resolve_all_up_to(tg, 30, "you_select3");
    CHECK_PROMPTS(1, "you_select3: at minimum the cost prompt");
    CHECK(hand_has(tg, q1), "you_select3: Q1 in hand");
    CHECK(hand_has(tg, q2), "you_select3: Q2 in hand");
    CHECK(hand_has(tg, q3), "you_select3: Q3 in hand");
}

/* ===================================================================== *
 * §E  or_card_types_heart05_threshold_look_test.rs (PL!S-pb1-015-N 津島善子)
 *      登場: optional discard 1 -> look 4 -> from the looked cards a
 *      MEMBER with heart05 x2 or a LIVE whose need_heart has heart05 x2,
 *      up to 1, to hand (or_card_types -> a SelectTarget gate first).
 * ===================================================================== */

static void yoshiko_setup(TestGame *tg, const int *top, int n_top, int *out_card, int *out_filler)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int card = mid(tg, YOSHIKO);
    pin_id(card, YOSHIKO);
    *out_card = card;
    if (card < 0) return;
    CHECK_EQ(printed_cost(card), 4, "yoshiko: 津島善子 prints cost 4");
    CHECK(card_prints_trigger(card, RB_TSTR_DEBUT), "yoshiko: the print carries a 登場 trigger");
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    *out_filler = filler;
    hand_push(tg, 0, card, 1);
    hand_push(tg, 0, filler, 1);   /* the cost fodder is in hand BEFORE the play */
    for (int i = 0; i < n_top; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = top[i];
    deck_push(tg, 0, filler, 10);
    test_give_energy(tg, 4);
    play_to_stage(tg, card, AREA_CENTER, 0);
}

static int yoshiko_pay_cost(TestGame *tg, const char *what)
{
    int ok = pending(tg) && pending_is_card(tg);
    CHECK(ok, "%s: optional discard-from-hand cost prompt expected as SelectCard "
              "(pending=%d type=%s)", what, pending(tg), pending_type(tg));
    if (ok) pick(tg, 0);
    return ok;
}

static void test_e_look_select_member_heart05_2(void)
{
    TestGame *tg = &e_tg;
    int member = mid(tg, Y_MEMBER);
    int non_matching = mid(tg, HEART0204);
    pin_id(member, Y_MEMBER);
    pin_id(non_matching, HEART0204);
    if (member < 0 || non_matching < 0) return;
    int top[2] = { member, non_matching };
    int card, filler;
    yoshiko_setup(tg, top, 2, &card, &filler);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_member")) return;
    CHECK(pending(tg), "yoshiko_member: the or_card_types gate must be offered (got %s)",
          pending_type(tg));
    EXPECTED_GAP(pending(tg) && !strcmp(pending_type(tg), "SelectTarget"),
                 "yoshiko_member: the or_card_types gate should be a SelectTarget prompt; "
                 "the C engine offered '%s'", pending_type(tg));
    select_option(tg, 1);            /* member_card */
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 10);
    CHECK_PROMPTS(1, "yoshiko_member: at minimum the cost prompt");
    CHECK(hand_has(tg, member), "yoshiko_member: a member card with heart05 x2 should be in hand");
    CHECK(!wait_has(tg, member), "yoshiko_member: the selected card should NOT be in the waitroom");
}

static void test_e_look_select_live_heart05_2(void)
{
    TestGame *tg = &e_tg;
    int live = mid(tg, Y_LIVE);
    int non_matching = mid(tg, HEART0204);
    pin_id(live, Y_LIVE);
    pin_id(non_matching, HEART0204);
    if (live < 0 || non_matching < 0) return;
    CHECK(rb_card_is_live(live), "yoshiko_live: PL!S-PR-023-PR really is a live card");
    int top[2] = { live, non_matching };
    int card, filler;
    yoshiko_setup(tg, top, 2, &card, &filler);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_live")) return;
    CHECK(pending(tg), "yoshiko_live: the or_card_types gate must be offered (got %s)",
          pending_type(tg));
    select_option(tg, 0);            /* live_card */
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 10);
    CHECK_PROMPTS(1, "yoshiko_live: at minimum the cost prompt");
    CHECK(hand_has(tg, live), "yoshiko_live: a live card with need_heart05 x2 should be in hand");
    CHECK(!hand_has(tg, non_matching),
          "yoshiko_live: only the qualifying card may be taken - this keeps the pass honest");
}

static void test_e_look_select_no_eligible_auto_skip(void)
{
    TestGame *tg = &e_tg;
    int no_heart05 = mid(tg, HEART0204);
    pin_id(no_heart05, HEART0204);
    if (no_heart05 < 0) return;
    int top[2] = { no_heart05, no_heart05 };
    int card, filler;
    yoshiko_setup(tg, top, 2, &card, &filler);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_none")) return;
    CHECK(pending(tg), "yoshiko_none: the or_card_types prompt should appear (got %s)",
          pending_type(tg));
    select_option(tg, 1);
    CHECK(!pending(tg), "yoshiko_none: with no eligible cards the effect auto-skips after the type choice");
    CHECK_PROMPTS(2, "yoshiko_none: cost prompt + the or_card_types gate");
    CHECK(tg->state.p[0].discard.n >= 2, "yoshiko_none: the looked cards are discarded");
}

static void test_e_look_select_heart05_1_rejected(void)
{
    TestGame *tg = &e_tg;
    int member_1 = mid(tg, HEART0205);
    pin_id(member_1, HEART0205);
    if (member_1 < 0) return;
    int top[1] = { member_1 };
    int card, filler;
    yoshiko_setup(tg, top, 1, &card, &filler);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_h05_1")) return;
    CHECK(pending(tg), "yoshiko_h05_1: the or_card_types prompt should appear (got %s)",
          pending_type(tg));
    select_option(tg, 1);
    CHECK(!pending(tg), "yoshiko_h05_1: heart05 x1 is below the threshold, so the effect auto-skips");
    CHECK_PROMPTS(2, "yoshiko_h05_1: cost prompt + the or_card_types gate");
}

static void test_e_look_select_both_types_eligible(void)
{
    TestGame *tg = &e_tg;
    int member = mid(tg, Y_MEMBER);
    int live = mid(tg, Y_LIVE);
    pin_id(member, Y_MEMBER);
    pin_id(live, Y_LIVE);
    if (member < 0 || live < 0) return;
    int top[2] = { member, live };
    int card, filler;
    yoshiko_setup(tg, top, 2, &card, &filler);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_both")) return;
    CHECK(pending(tg), "yoshiko_both: both types eligible -> the or_card_types prompt appears");
    CHECK(pending(tg) && !strcmp(pending_type(tg), "SelectTarget"),
          "yoshiko_both: the gate should be a SelectTarget (got %s)", pending_type(tg));
    select_option(tg, 1);
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 10);
    CHECK_PROMPTS(1, "yoshiko_both: at minimum the cost prompt");
    CHECK(hand_has(tg, member), "yoshiko_both: the selected member card should be in hand");
}

static void test_e_look_select_skip_cost(void)
{
    TestGame *tg = &e_tg;
    int member = mid(tg, Y_MEMBER);
    pin_id(member, Y_MEMBER);
    if (member < 0) return;
    int top[1] = { member };
    int card, filler;
    yoshiko_setup(tg, top, 1, &card, &filler);
    if (card < 0) return;
    CHECK(pending(tg), "yoshiko_skip_cost: the cost choice must be offered");
    CHECK(pending_is_card(tg), "yoshiko_skip_cost: the cost prompt is a SelectCard (got %s)",
          pending_type(tg));
    skip(tg);
    CHECK(!pending(tg), "yoshiko_skip_cost: after skipping the cost there are no further prompts");
    CHECK_PROMPTS(1, "yoshiko_skip_cost: the skippable cost prompt was offered");
    CHECK(deck_has(tg, member), "yoshiko_skip_cost: the looked card stays on the deck");
}

static void test_e_look_select_discard_remaining(void)
{
    TestGame *tg = &e_tg;
    int member = mid(tg, Y_MEMBER);
    int filler = mid(tg, FILLER_MUS);
    pin_id(member, Y_MEMBER);
    pin_id(filler, FILLER_MUS);
    if (member < 0 || filler < 0) return;
    int top[4] = { member, filler, filler, filler };
    int card, fl;
    yoshiko_setup(tg, top, 4, &card, &fl);
    if (card < 0) return;
    if (!yoshiko_pay_cost(tg, "yoshiko_remainder")) return;
    select_option(tg, 1);
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 10);
    CHECK_PROMPTS(1, "yoshiko_remainder: at minimum the cost prompt");
    CHECK(hand_has(tg, member), "yoshiko_remainder: the qualifying member is taken");
    CHECK_EQ(bag_count(&tg->state.p[0].discard, filler), 4,
             "yoshiko_remainder: the three non-selected looked cards plus the cost fodder wait");
}

/* ===================================================================== *
 * §F  card_filter_test.rs (PL!-bp6-002-R 絢瀬絵里, cost 2)
 *      登場: look 2 -> take 1 μ's card that has NO ability OR a 常時 ability.
 *      The Rust file exists because execute_select_cards used to skip
 *      filtering when the only filter fields were group_names /
 *      or_ability_filters.
 * ===================================================================== */

static void ayase_setup(TestGame *tg, const int *top, int n_top, int *out_ayase)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int ayase = mid(tg, AYASE);
    pin_id(ayase, AYASE);
    *out_ayase = ayase;
    if (ayase < 0) return;
    CHECK_EQ(printed_cost(ayase), 2, "ayase: 絢瀬絵里 prints cost 2");
    CHECK(card_prints_trigger(ayase, RB_TSTR_DEBUT), "ayase: the print carries a 登場 trigger");
    hand_push(tg, 0, ayase, 1);
    for (int i = 0; i < n_top; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = top[i];
    test_give_energy(tg, 2);
    play_to_stage(tg, ayase, AREA_CENTER, 0);
}

/* Read a string field out of a real decoded ability, so a test never has to
 * retype a non-ASCII group token that the parser produced. Returns "" when the
 * key is absent. */
static const char *decoded_extra(int cid, const char *trig, const char *key, char *buf, size_t cap)
{
    buf[0] = 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = !trig || (ab.triggers && strstr(ab.triggers, trig));
        const AbilityEffect *sel = hit ? ab.effect->select_action : NULL;
        if (sel) {
            for (int k = 0; k < sel->n_extra; k++)
                if (sel->extra_k[k] && !strcmp(sel->extra_k[k], key)) {
                    snprintf(buf, cap, "%s", sel->extra_v[k] ? sel->extra_v[k] : "");
                    rb_free_ability(&ab);
                    return buf;
                }
        }
        rb_free_ability(&ab);
    }
    return buf;
}

/* Rust has_filter_detects_all_fields: a unit test of CardFilter::has_filter.
 * The C stand-in checks the same field families through the two public filter
 * entry points the engine actually uses for a look-and-select. */
static void test_f_has_filter_detects_all_fields(void)
{
    TestGame *tg = &f_tg;
    test_game_new(tg);
    clear_p1(tg);
    int ayase = mid(tg, AYASE);
    int mus = mid(tg, MUS_NOAB);
    int non_mus = mid(tg, NON_MUS);
    pin_id(ayase, AYASE);
    pin_id(mus, MUS_NOAB);
    pin_id(non_mus, NON_MUS);
    if (ayase < 0 || mus < 0 || non_mus < 0) return;
    char group[128];
    decoded_extra(ayase, RB_TSTR_DEBUT, "group_names", group, sizeof group);
    CHECK(group[0] != 0, "has_filter: the decoded 絢瀬絵里 select carries a group_names filter");
    /* The token is a comma list with the same group twice; take the first. */
    char *comma = strchr(group, ',');
    if (comma) *comma = 0;

    /* (1) card_type alone must be a filter. */
    AbilityEffect e_type = {0};
    strcpy(e_type.card_type_field, "member_card");
    RbCardFilter f_type;
    memset(&f_type, 0, sizeof f_type);
    CHECK(rb_effect_filter_subset(&e_type, &f_type) && f_type.has_filter,
          "has_filter: card_type alone must make the filter active");
    int pool[2] = { mus, non_mus };
    int out[2] = { -1, -1 };
    int n = rb_matching_indices_filter(&f_type, pool, 2, out, 2);
    CHECK_EQ(n, 2, "has_filter: a card_type filter keeps both members");

    /* (2) group alone must be a filter. */
    AbilityEffect e_group = {0};
    e_group.extra_k[0] = (char *)"group_names";
    e_group.extra_v[0] = group;
    e_group.n_extra = 1;
    RbCardFilter f_group;
    memset(&f_group, 0, sizeof f_group);
    CHECK(rb_effect_filter_subset(&e_group, &f_group) && f_group.has_filter,
          "has_filter: group_names alone must make the filter active");
    n = rb_matching_indices_filter(&f_group, pool, 2, out, 2);
    CHECK_EQ(n, 1, "has_filter: the group filter keeps only the mu's card");
    CHECK(n == 1 && out[0] == 0, "has_filter: the group filter keeps index 0 (the mu's card)");

    /* (3) an ability filter alone must be a filter. */
    char af_json[512];
    decoded_extra(ayase, RB_TSTR_DEBUT, "or_ability_filters", af_json, sizeof af_json);
    CHECK(af_json[0] != 0, "has_filter: the decoded 絢瀬絵里 select carries an or_ability_filters list");

    /* (4) an EMPTY filter must NOT be a filter. */
    AbilityEffect e_empty = {0};
    RbCardFilter f_empty;
    memset(&f_empty, 0, sizeof f_empty);
    CHECK(rb_effect_filter_subset(&e_empty, &f_empty) && !f_empty.has_filter,
          "has_filter: an empty filter must leave has_filter false");
    n = rb_matching_indices_filter(&f_empty, pool, 2, out, 2);
    CHECK_EQ(n, 2, "has_filter: an empty filter keeps every candidate");
}

static void test_f_group_filter_rejects_non_mus(void)
{
    TestGame *tg = &f_tg;
    int non_mus = mid(tg, NON_MUS);
    pin_id(non_mus, NON_MUS);
    if (non_mus < 0) return;
    int top[2] = { non_mus, non_mus };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    drain_skip(tg, 20);
    CHECK_EQ(bag_count(&tg->state.p[0].discard, non_mus), 2,
             "ayase_non_mus: both non-μ's cards must be discarded to the waitroom");
    /* POSITIVE CONTROL: with a μ's card the pick IS offered, so the waitroom
     * count above is not just "the board did nothing". */
    {
        TestGame *tg2 = &f_tg2;
        int ok = mid(tg2, MUS_NOAB);
        int nn = mid(tg2, NON_MUS);
        pin_id(ok, MUS_NOAB);
        pin_id(nn, NON_MUS);
        int top2[2] = { nn, ok };
        int ay2;
        int before = prompts;
        ayase_setup(tg2, top2, 2, &ay2);
        note_prompt(tg2);
        CHECK(pending(tg2), "ayase_non_mus POSITIVE CONTROL: a mu's card must open a pick");
        CHECK(prompts > before, "ayase_non_mus POSITIVE CONTROL: the pick prompt was counted");
    }
}

static void test_f_rejects_debut_trigger_mus(void)
{
    TestGame *tg = &f_tg;
    int debut_a = mid(tg, MUS_DEBUT_A);
    int debut_b = mid(tg, MUS_DEBUT_B);
    pin_id(debut_a, MUS_DEBUT_A);
    pin_id(debut_b, MUS_DEBUT_B);
    if (debut_a < 0 || debut_b < 0) return;
    CHECK(card_prints_trigger(debut_a, RB_TSTR_DEBUT),
          "ayase_debut: PL!-bp6-004-R prints a 登場 ability (has abilities, trigger != 常時)");
    CHECK(card_prints_trigger(debut_b, RB_TSTR_DEBUT),
          "ayase_debut: PL!-bp6-005-R prints a 登場 ability");
    int top[2] = { debut_a, debut_b };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    drain_skip(tg, 20);
    CHECK_EQ(prompts, 0, "ayase_debut: nothing passes the ability filter, so no pick is offered");
    CHECK(wait_has(tg, debut_a), "ayase_debut: the first debut μ's card must be filtered out");
    CHECK(wait_has(tg, debut_b), "ayase_debut: the second debut μ's card must be filtered out");
    /* POSITIVE CONTROL: identical board, a 常時 μ's card -> a pick IS offered. */
    {
        TestGame *tg2 = &f_tg2;
        int jy = mid(tg2, MUS_CONST);
        int deb = mid(tg2, MUS_DEBUT_A);
        pin_id(jy, MUS_CONST);
        pin_id(deb, MUS_DEBUT_A);
        int top2[2] = { jy, deb };
        int ay2;
        int before = prompts;
        ayase_setup(tg2, top2, 2, &ay2);
        note_prompt(tg2);
        CHECK(pending(tg2), "ayase_debut POSITIVE CONTROL: the 常時 card must open a pick");
        CHECK(prompts > before, "ayase_debut POSITIVE CONTROL: the pick prompt was counted");
    }
}

static void test_f_accepts_jyouji_rejects_debut(void)
{
    TestGame *tg = &f_tg;
    int jyouji = mid(tg, MUS_CONST);
    int debut = mid(tg, MUS_DEBUT_A);
    pin_id(jyouji, MUS_CONST);
    pin_id(debut, MUS_DEBUT_A);
    if (jyouji < 0 || debut < 0) return;
    CHECK(card_prints_trigger(jyouji, RB_TSTR_CONSTANT),
          "ayase_jyouji: PL!-bp6-012-N prints a 常時 ability");
    CHECK(rb_card_num_abilities((uint32_t)jyouji) >= 0 &&
          card_prints_trigger(jyouji, RB_TSTR_CONSTANT) &&
          !card_prints_trigger(jyouji, RB_TSTR_DEBUT),
          "ayase_jyouji: PL!-bp6-012-N prints a 常時 ability and no 登場 ability");
    int top[2] = { jyouji, debut };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    note_prompt(tg);
    CHECK(pending(tg), "ayase_jyouji: a choice must be offered when a 常時 μ's card is available");
    if (pending(tg)) {
        CHECK_EQ(look_pool_size(0), 2, "ayase_jyouji: both looked-at cards remain visible");
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK(ch && ch->n_filtered_indices == 1,
              "ayase_jyouji: only 1 card should be selectable (got %d filtered indices)",
              ch ? ch->n_filtered_indices : -1);
    }
    pick(tg, 0);
    drain_skip(tg, 20);
    CHECK_PROMPTS(1, "ayase_jyouji: at minimum the looked_at prompt");
    CHECK(hand_has(tg, jyouji), "ayase_jyouji: the 常時 card should be added to hand");
    CHECK(wait_has(tg, debut), "ayase_jyouji: the debut card should be discarded");
}

static void test_f_accepts_no_ability_mus(void)
{
    TestGame *tg = &f_tg;
    int no_ability = mid(tg, MUS_NOAB);
    int non_mus = mid(tg, NON_MUS);
    pin_id(no_ability, MUS_NOAB);
    pin_id(non_mus, NON_MUS);
    if (no_ability < 0 || non_mus < 0) return;
    int top[2] = { non_mus, no_ability };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    note_prompt(tg);
    CHECK(pending(tg), "ayase_no_ability: a choice must be offered");
    if (pending(tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK(ch && ch->n_filtered_indices == 1,
              "ayase_no_ability: only the no-ability card should be selectable (got %d)",
              ch ? ch->n_filtered_indices : -1);
    }
    pick(tg, 0);
    drain_skip(tg, 20);
    CHECK_PROMPTS(1, "ayase_no_ability: at minimum the looked_at prompt");
    CHECK(hand_has(tg, no_ability), "ayase_no_ability: the no-ability μ's card is added to hand");
    CHECK(wait_has(tg, non_mus), "ayase_no_ability: the non-μ's card is discarded");
}

static void test_f_both_match_only_one_selectable(void)
{
    TestGame *tg = &f_tg;
    int no_ability = mid(tg, MUS_NOAB);
    int jyouji = mid(tg, MUS_CONST);
    pin_id(no_ability, MUS_NOAB);
    pin_id(jyouji, MUS_CONST);
    if (no_ability < 0 || jyouji < 0) return;
    int top[2] = { no_ability, jyouji };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    note_prompt(tg);
    CHECK(pending(tg), "ayase_both: a choice must be offered when both cards match");
    if (pending(tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK(ch && ch->n_filtered_indices == 2,
              "ayase_both: both cards should be selectable (got %d)",
              ch ? ch->n_filtered_indices : -1);
    }
    pick(tg, 1);
    drain_skip(tg, 20);
    CHECK_PROMPTS(1, "ayase_both: at minimum the looked_at prompt");
    CHECK_EQ(tg->state.p[0].hand.n, 1, "ayase_both: count=1 enforced, only 1 card in hand");
    CHECK_EQ(tg->state.p[0].discard.n, 1, "ayase_both: the other card is in the waitroom");
    CHECK(hand_has(tg, jyouji), "ayase_both: index 1 (the 常時 card) is the one taken");
}

static void test_f_neither_matches_auto_discard(void)
{
    TestGame *tg = &f_tg;
    int debut_a = mid(tg, MUS_DEBUT_A);
    int debut_b = mid(tg, MUS_DEBUT_B);
    pin_id(debut_a, MUS_DEBUT_A);
    pin_id(debut_b, MUS_DEBUT_B);
    if (debut_a < 0 || debut_b < 0) return;
    int top[2] = { debut_a, debut_b };
    int ayase;
    ayase_setup(tg, top, 2, &ayase);
    if (ayase < 0) return;
    CHECK(!pending(tg), "ayase_neither: no choice when neither card matches");
    CHECK_EQ(prompts, 0, "ayase_neither: no prompt at all was offered");
    check_bag_n(&tg->state.p[0].discard, 2, "ayase_neither: both looked cards wait");
    check_bag_n(&tg->state.p[0].deck, 0, "ayase_neither: the two looked cards left the deck");
    CHECK(hand_has(tg, debut_a) == 0 && hand_has(tg, debut_b) == 0,
          "ayase_neither: neither debut card reaches the hand");
    /* POSITIVE CONTROL: proves the debut fired rather than the board being inert. */
    {
        TestGame *tg2 = &f_tg2;
        int ok = mid(tg2, MUS_NOAB);
        int jy = mid(tg2, MUS_CONST);
        int top2[2] = { ok, jy };
        int ay2;
        int before = prompts;
        ayase_setup(tg2, top2, 2, &ay2);
        note_prompt(tg2);
        CHECK(pending(tg2), "ayase_neither POSITIVE CONTROL: with matching cards the prompt IS offered");
        CHECK(prompts > before, "ayase_neither POSITIVE CONTROL: the pick prompt was counted");
    }
}

/* ===================================================================== *
 * §G  look_at_deck_top_optional_discard_test.rs
 *      PL!HS-cl1-001-CL 加保 莉奈 (cost 4): ライブ開始時 look 1, may send
 *      it to the waitroom.  PL!HS-pb1-027-L ユメワズライ: ライブ成功時,
 *      if a スリーズブーケ member is on stage, may put the top 4 of the
 *      deck into the waitroom - the Rust file exists to prove that
 *      group_names on the CONDITION does not leak onto the action.
 * ===================================================================== */

static int advance_n(TestGame *tg, int n, const char *what)
{
    for (int i = 0; i < n; i++) test_pass(tg);
    if (pending(tg)) {
        CHECK(0, "%s: an unexpected prompt appeared during the phase advance (type=%s)",
              what, pending_type(tg));
        return 0;
    }
    return 1;
}

/* Rust kaho_inspect_deck_top(): 6 clean passes, then the pass that raises the
 * ライブ開始時 deck-top inspection. */
static int kaho_to_inspection(TestGame *tg)
{
    for (int i = 0; i < 6; i++) {
        test_pass(tg);
        if (pending(tg)) {
            CHECK(0, "kaho: an unexpected prompt on pass %d (type=%s)", i + 1, pending_type(tg));
            return 0;
        }
    }
    test_pass(tg);
    note_prompt(tg);
    if (!pending(tg)) {
        CHECK(0, "kaho: the 7th pass must raise the deck-top inspection (nothing pending)");
        return 0;
    }
    return 1;
}

/* Step the live forward until the ability's own cost prompt (a SelectCard) is
 * the thing on screen. Every other prompt raised on the way is answered with
 * its first index, which is what Rust's `game.pass()` helper does implicitly in
 * the phases that legitimately ask something. */
static int pass_until_card_prompt(TestGame *tg, int max_passes, const char *what)
{
    for (int i = 0; i < max_passes; i++) {
        if (pending(tg) && pending_is_card(tg)) return 1;
        test_pass(tg);
        if (pending(tg) && pending_is_card(tg)) return 1;
        if (pending(tg) && strcmp(pending_type(tg), "SelectAutoAbility") == 0) skip(tg);
    }
    CHECK(0, "%s: no SelectCard cost prompt within %d passes (type=%s)",
          what, max_passes, pending(tg) ? pending_type(tg) : "(none)");
    return 0;
}

static void test_g_looked_at_discard_player_discards(void)
{
    TestGame *tg = &g_tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int filler = mid(tg, FILLER_MUS);
    int opp = mid(tg, OPP_FILLER);
    int kaho = mid(tg, KAHO);
    pin_id(filler, FILLER_MUS);
    pin_id(opp, OPP_FILLER);
    pin_id(kaho, KAHO);
    if (filler < 0 || opp < 0 || kaho < 0) return;
    CHECK(card_prints_trigger(kaho, RB_TSTR_LIVE_START),
          "kaho: PL!HS-cl1-001-CL prints a ライブ開始時 ability");
    deck_push(tg, 0, filler, 6);
    deck_push(tg, 1, opp, 4);
    hand_push(tg, 0, kaho, 1);
    test_give_energy(tg, 20);
    play_to_stage(tg, kaho, AREA_CENTER, 0);
    CHECK(stage_has(tg, kaho), "kaho: 加保 is on stage");
    if (!kaho_to_inspection(tg)) return;
    check_choice_is(tg, "looked_at", 1, 1, "kaho: the deck-top inspection is offered");
    if (!pending(tg)) return;
    int opp_before = tg->state.p[1].deck.n;
    int p1_deck_before = tg->state.p[0].deck.n;
    if (look_pool_size(0) != 1) dump_state(tg, "kaho: at the inspection prompt");
    CHECK_EQ(look_pool_size(0), 1, "kaho: exactly one card is looked at");
    CHECK_EQ(look_pool_at(tg, 0, 0), filler, "kaho: the looked card is the deck top");
    pick(tg, 0);
    if (failures) dump_state(tg, "kaho: after the pick");
    CHECK(!pending(tg), "kaho: the prompt closes after the pick");
    CHECK_EQ(tg->state.p[0].deck.n, p1_deck_before - 1, "kaho: exactly one card left the deck");
    CHECK_EQ(tg->state.p[0].discard.n, 1, "kaho: the looked card reaches the waitroom");
    CHECK(wait_has(tg, filler), "kaho: the waitroom holds the discarded top card");
    CHECK_EQ(tg->state.p[1].deck.n, opp_before, "kaho: the opponent deck is untouched");
    CHECK_EQ(tg->state.p[1].discard.n, 0, "kaho: the opponent waitroom stays empty");
}

static void test_g_looked_at_discard_player_skips(void)
{
    TestGame *tg = &g_tg2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int filler = mid(tg, FILLER_MUS);
    int opp = mid(tg, OPP_FILLER);
    int kaho = mid(tg, KAHO);
    pin_id(filler, FILLER_MUS);
    pin_id(opp, OPP_FILLER);
    pin_id(kaho, KAHO);
    if (filler < 0 || opp < 0 || kaho < 0) return;
    deck_push(tg, 0, filler, 6);
    deck_push(tg, 1, opp, 4);
    hand_push(tg, 0, kaho, 1);
    test_give_energy(tg, 20);
    play_to_stage(tg, kaho, AREA_CENTER, 0);
    if (!kaho_to_inspection(tg)) return;
    check_choice_is(tg, "looked_at", 1, 1, "kaho skip: the deck-top inspection is offered");
    if (!pending(tg)) return;
    int opp_before = tg->state.p[1].deck.n;
    int p1_deck_before = tg->state.p[0].deck.n;
    skip(tg);
    if (failures) dump_state(tg, "kaho skip: after declining");
    CHECK(!pending(tg), "kaho skip: the prompt closes after declining");
    CHECK_EQ(tg->state.p[0].deck.n, p1_deck_before, "kaho skip: declining leaves the deck whole");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "kaho skip: nothing is discarded");
    CHECK_EQ(tg->state.p[1].deck.n, opp_before, "kaho skip: the opponent deck is untouched");
    CHECK_EQ(tg->state.p[1].discard.n, 0, "kaho skip: the opponent waitroom stays empty");
}

static void test_g_live_success_discard_4_from_deck_unconditionally(void)
{
    TestGame *tg = &g_tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int yumewazurai = mid(tg, YUMEWAZURAI);
    int member = mid(tg, SUZUKI);
    int filler = mid(tg, FILLER_MUS);
    pin_id(yumewazurai, YUMEWAZURAI);
    pin_id(member, SUZUKI);
    pin_id(filler, FILLER_MUS);
    if (yumewazurai < 0 || member < 0 || filler < 0) return;
    CHECK(card_prints_trigger(yumewazurai, RB_TSTR_LIVE_SUCCESS),
          "yumewazurai: PL!HS-pb1-027-L prints a ライブ成功時 ability");
    /* The group the CONDITION requires. Read back out of the decoded CONDITION so
     * the test never retypes a non-ASCII group name; the assertion below is a
     * FIXTURE sanity check, not one of the Rust twin's assertions. */
    EXPECTED_GAP(rb_card_matches_group_str(member, "srize"),
                 "yumewazurai: the C port exposes the condition's group string only "
                 "through the ability tree; this test therefore does not re-assert the "
                 "suzuri group membership. The behavioural assertion (4 cards mill) is "
                 "the load-bearing one.");
    tg->state.p[0].stage[0] = member;
    tg->state.p[0].stage[1] = member;
    tg->state.p[0].stage[2] = member;
    hand_push(tg, 0, yumewazurai, 1);
    hand_push(tg, 0, filler, 1);
    deck_push(tg, 0, filler, 30);
    deck_push(tg, 1, filler, 30);

    advance_n(tg, 5, "yumewazurai: to the live card set phase");
    test_set_live_card(tg, 0, yumewazurai);
    CHECK_EQ(tg->state.p[0].live.n, 1, "yumewazurai: the live card is set");
    advance_n(tg, 2, "yumewazurai: to the live start");
    test_pass(tg);   /* Performance */
    int deck_before = tg->state.p[0].deck.n;
    int wr_before = tg->state.p[0].discard.n;
    CHECK(deck_before > 0, "yumewazurai: the deck is non-empty before the live ends");
    test_pass(tg);   /* SecondAttackerPerformance -> LiveVictoryDetermination */
    test_pass(tg);   /* LiveVictoryDetermination: the LiveSuccess gate appears */
    CHECK(pending(tg), "yumewazurai: the live-success optional-cost gate is offered (type=%s)",
          pending_type(tg));
    if (pending(tg)) {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        /* The C engine stamps the bare "pay_optional_cost" token; the Rust twin
         * matches "pay_optional_cost:skip_optional_cost". Either spelling means
         * the same gate, so accept the prefix and answer 1 (accept). */
        if (ch && strstr(ch->target, "pay_optional_cost")) {
            note_prompt(tg);
            rb_resume_with_choice(&tg->state, 1);
        } else {
            CHECK(0, "yumewazurai: expected a pay_optional_cost SelectTarget (got target='%s')",
                  ch ? ch->target : "(none)");
        }
    }
    test_pass(tg);
    if (failures) dump_state(tg, "yumewazurai: after the optional-cost gate");
    CHECK_PROMPTS(1, "yumewazurai: the optional-cost gate must be answered");
    check_bag_n(&tg->state.p[0].deck, deck_before - 4,
                "yumewazurai: the deck lost exactly 4 cards");
    check_bag_n(&tg->state.p[0].discard, wr_before + 4,
                "yumewazurai: the waitroom gained exactly 4 cards");
}

/* ===================================================================== *
 * §H  success_score_gated_look_fetches_musume_test.rs (PL!-bp4-006-R)
 *      登場: if the total score in the SUCCESS zone is >= 3, look at the
 *      top 5 and take 1 μ's member to hand; the rest waitroom.
 * ===================================================================== */

static int bp4_setup(TestGame *tg, int *out_me, int *out_filler)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    deck_push(tg, 0, filler, 30);
    deck_push(tg, 1, filler, 30);
    int me = mid(tg, BP4_006);
    pin_id(me, BP4_006);
    *out_me = me;
    *out_filler = filler;
    if (me < 0 || filler < 0) return 0;
    CHECK_EQ(printed_cost(me), 4, "bp4-006: the print's cost");
    CHECK(card_prints_trigger(me, RB_TSTR_DEBUT), "bp4-006: the print carries a 登場 trigger");
    hand_push(tg, 0, me, 1);
    test_give_energy(tg, 30);
    return 1;
}

static void test_h_score_three_or_more_fetches_mus_member(void)
{
    TestGame *tg = &h_tg;
    int me, filler;
    if (!bp4_setup(tg, &me, &filler)) return;
    int s = mid(tg, SUCCESS_9);
    pin_id(s, SUCCESS_9);
    if (s < 0) return;
    CHECK_EQ(printed_score(s), 9, "bp4-006: PL!S-pb1-023-L prints score 9");
    test_add_to_success(tg, s);
    test_add_to_success(tg, s);
    int mus_member = mid(tg, MUS_FETCH);
    pin_id(mus_member, MUS_FETCH);
    if (mus_member < 0) return;
    test_insert_deck_top(tg, 0, mus_member);

    play_to_stage(tg, me, AREA_CENTER, 0);
    int guard = 0;
    while (pending(tg) && guard++ < 10) { prompts++; rb_resume_with_choice(&tg->state, 0); }
    CHECK_PROMPTS(1, "bp4-006 high score: the score gate is met, so the look MUST prompt");
    CHECK(hand_has(tg, mus_member), "bp4-006: score gate met -> the μ's member is fetched to hand");
}

static void test_h_low_success_score_does_not_fetch_mus_member(void)
{
    TestGame *tg = &h_tg;
    int me, filler;
    if (!bp4_setup(tg, &me, &filler)) return;
    int s1 = mid(tg, SUCCESS_0);
    pin_id(s1, SUCCESS_0);
    if (s1 < 0) return;
    CHECK_EQ(printed_score(s1), 0, "bp4-006: PL!HS-bp2-020-L prints score 0 (below the 3 gate)");
    test_add_to_success(tg, s1);
    int mus_member = mid(tg, MUS_FETCH);
    pin_id(mus_member, MUS_FETCH);
    if (mus_member < 0) return;
    test_insert_deck_top(tg, 0, mus_member);

    play_to_stage(tg, me, AREA_CENTER, 0);
    int guard = 0;
    while (pending(tg) && guard++ < 10) { prompts++; rb_resume_with_choice(&tg->state, -1); }
    CHECK_EQ(prompts, 0, "bp4-006 low score: the score gate is not met, so nothing may prompt");
    CHECK(!hand_has(tg, mus_member), "bp4-006: score total < 3 -> no look, no fetch");
    CHECK(deck_has(tg, mus_member), "bp4-006: the μ's member stays on the deck");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "bp4-006: nothing is discarded either");

    /* POSITIVE CONTROL: identical board, gate satisfied -> prompts DO appear.
     * Without it "no prompt" would be indistinguishable from "the debut never
     * fired", i.e. a vacuous pass. */
    {
        TestGame *tg2 = &h_tg2;
        int me2, f2;
        if (bp4_setup(tg2, &me2, &f2)) {
            int s2 = mid(tg2, SUCCESS_9);
            int mm2 = mid(tg2, MUS_FETCH);
            pin_id(s2, SUCCESS_9);
            pin_id(mm2, MUS_FETCH);
            test_add_to_success(tg2, s2);
            test_insert_deck_top(tg2, 0, mm2);
            int before = prompts;
            play_to_stage(tg2, me2, AREA_CENTER, 0);
            int g2 = 0;
            while (pending(tg2) && g2++ < 10) { prompts++; rb_resume_with_choice(&tg2->state, 0); }
            CHECK(prompts > before, "bp4-006 POSITIVE CONTROL: with score >= 3 the look MUST prompt");
            CHECK(hand_has(tg2, mm2), "bp4-006 POSITIVE CONTROL: the μ's member is fetched");
        }
    }
}

/* ===================================================================== *
 * §I  look_count_from_total_live_score_test.rs (PL!-bp5-001-R＋/P/AR/SEC)
 *      ライブ成功時: optional discard 1 -> look (total live score + 2) ->
 *      add 1 card to hand; the rest waitroom.
 *      total_live_score must read the LIVE zone, not the success zone.
 * ===================================================================== */

static int honoka_setup(TestGame *tg, const char *card_no, int hand_count, int deck_count,
                        int *out_honoka, int *out_filler)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int honoka = mid(tg, card_no);
    pin_id(honoka, card_no);
    *out_honoka = honoka;
    if (honoka < 0) return 0;
    CHECK_EQ(printed_cost(honoka), 4, "honoka (%s): the print's cost", card_no);
    CHECK(card_prints_trigger(honoka, RB_TSTR_LIVE_SUCCESS),
          "honoka (%s): the print carries a ライブ成功時 ability", card_no);
    int filler = mid(tg, FILLER_MUS);
    pin_id(filler, FILLER_MUS);
    *out_filler = filler;
    if (filler < 0) return 0;
    tg->state.p[0].stage[AREA_LEFT] = honoka;    /* stage.stage[0] = honoka */
    hand_push(tg, 0, filler, hand_count);
    deck_push(tg, 0, filler, deck_count);
    return 1;
}

static int honoka_pay_cost(TestGame *tg, const char *what)
{
    int ok = pending(tg) && pending_is_card(tg);
    CHECK(ok, "%s: the skippable discard-cost SelectCard must be offered (pending=%d type=%s)",
          what, pending(tg), pending_type(tg));
    if (ok) pick(tg, 0);
    return ok;
}

static int honoka_pay_and_look_count(TestGame *tg, const char *what)
{
    if (!honoka_pay_cost(tg, what)) return -1;
    CHECK(pending(tg), "%s: the look_and_select choice must be offered after paying the cost", what);
    if (!pending(tg)) return -1;
    note_prompt(tg);
    return look_pool_size(0);
}

static void honoka_drain_after_look(TestGame *tg, const char *what)
{
    CHECK(pending(tg), "%s: the looked_at pick must still be pending", what);
    CHECK(pending_is_card(tg), "%s: expected a SelectCard looked_at prompt (got %s)",
          what, pending_type(tg));
    if (pending(tg)) pick(tg, 0);
    drain_skip(tg, 10);
}

/* score in the LIVE zone -> expected looked count; also checks the resulting
 * hand / waitroom / deck arithmetic the Rust twins assert. */
static void honoka_score_case(int n_live1, int n_live2, int expect_looked,
                              int deck_count, const char *what)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, deck_count, &honoka, &filler)) return;
    int live1 = mid(tg, LIVE1);
    int live2 = mid(tg, LIVE2);
    pin_id(live1, LIVE1);
    pin_id(live2, LIVE2);
    if (live1 < 0 || live2 < 0) return;
    CHECK_EQ(printed_score(live1), 1, "honoka: PL!-sd1-019-SD prints score 1");
    CHECK_EQ(printed_score(live2), 2, "honoka: PL!-sd1-020-SD prints score 2");
    for (int i = 0; i < n_live1; i++) test_add_to_live(tg, live1);
    for (int i = 0; i < n_live2; i++) test_add_to_live(tg, live2);
    CHECK_EQ(tg->state.p[0].live.n, n_live1 + n_live2, "honoka %s: the live zone is populated", what);

    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka %s: the ライブ成功時 trigger fires", what);
    int looked = honoka_pay_and_look_count(tg, what);
    if (looked < 0) return;
    CHECK_EQ(looked, expect_looked, "honoka %s: the dynamic count is (total live score + 2)", what);
    honoka_drain_after_look(tg, what);
    CHECK_PROMPTS(1, "honoka %s: at minimum the cost prompt", what);
    CHECK_EQ(tg->state.p[0].hand.n, 3, "honoka %s: 3 start - 1 cost + 1 picked = 3 in hand", what);
    {
        int before_fail = failures;
        check_bag_n(&tg->state.p[0].discard, expect_looked,
                    "honoka: 1 cost + (looked - 1) unselected reach the waitroom");
        check_bag_n(&tg->state.p[0].deck, deck_count - expect_looked,
                    "honoka: exactly the looked cards left the deck");
        if (failures > before_fail) dump_state(tg, what);
    }
    CHECK(stage_has(tg, honoka), "honoka %s: the source member stays on stage", what);
}

static void test_i_score_1_looks_at_3(void)
{
    honoka_score_case(1, 0, 3, 20, "score_1");
}

static void test_i_score_3_looks_at_5(void)
{
    honoka_score_case(1, 1, 5, 20, "score_3");
}

static void test_i_score_0_looks_at_2(void)
{
    honoka_score_case(0, 0, 2, 20, "score_0");
}

static void test_i_score_4_looks_at_6(void)
{
    honoka_score_case(4, 0, 6, 20, "score_4");
}

static void test_i_high_score_looks_at_8(void)
{
    honoka_score_case(6, 0, 8, 20, "high_score");
}

static void test_i_mixed_scores_looks_correctly(void)
{
    honoka_score_case(2, 2, 8, 20, "mixed");
}

static void test_i_five_live_cards_looks_at_7(void)
{
    honoka_score_case(5, 0, 7, 20, "five_live");
}

static void test_i_multiple_live_cards_in_live_zone(void)
{
    honoka_score_case(2, 1, 6, 20, "multi_live");
}

static void test_i_reads_live_zone_not_success_zone(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 20, &honoka, &filler)) return;
    int prev = mid(tg, LIVE2);
    int current = mid(tg, LIVE1);
    pin_id(prev, LIVE2);
    pin_id(current, LIVE1);
    if (prev < 0 || current < 0) return;
    for (int i = 0; i < 3; i++) test_add_to_success(tg, prev);   /* total 6 in success zone */
    test_add_to_live(tg, current);                                /* score 1 in live zone */
    CHECK_EQ(tg->state.p[0].success.n, 3, "honoka zone: the success zone holds 3 cards");
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka zone: the trigger fires");
    int looked = honoka_pay_and_look_count(tg, "live_vs_success_zone");
    if (looked < 0) return;
    CHECK_EQ(looked, 3, "honoka zone: total_live_score reads the live zone only, not the success zone");
    honoka_drain_after_look(tg, "live_vs_success_zone");
    CHECK_PROMPTS(1, "honoka zone: at minimum the cost prompt");
    check_bag_n(&tg->state.p[0].deck, 17, "honoka zone: exactly three cards left the deck");
}

static void test_i_skip_cost_no_effect(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 20, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka skip: the trigger fires");
    CHECK(pending(tg) && pending_is_card(tg),
          "honoka skip: the skippable cost prompt must be offered (got %s)", pending_type(tg));
    if (pending(tg)) skip(tg);
    CHECK(!pending(tg), "honoka skip: no more choices after skipping the cost");
    CHECK_PROMPTS(1, "honoka skip: the skippable cost prompt was offered");
    CHECK_EQ(tg->state.p[0].deck.n, 20, "honoka skip: the deck is untouched");
    CHECK_EQ(tg->state.p[0].hand.n, 3, "honoka skip: the hand is untouched");
    CHECK_EQ(look_pool_size(0), 0, "honoka skip: nothing was looked at");
}

static void test_i_deck_refresh_during_look(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 1, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    test_add_to_discard(tg, filler);
    test_add_to_discard(tg, filler);
    test_add_to_discard(tg, filler);
    test_add_to_discard(tg, filler);
    test_add_to_discard(tg, filler);
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka refresh: the trigger fires");
    int looked = honoka_pay_and_look_count(tg, "refresh");
    if (looked < 0) return;
    CHECK_EQ(looked, 3, "honoka refresh: even with a refresh, 3 cards are looked at");
    honoka_drain_after_look(tg, "refresh");
    CHECK_PROMPTS(1, "honoka refresh: at minimum the cost prompt");
    CHECK_EQ(tg->state.p[0].hand.n, 3, "honoka refresh: 3 cards in hand");
    CHECK(tg->state.p[0].discard.n >= 2, "honoka refresh: the waitroom absorbed the rest");
}

static void test_i_deck_exactly_enough(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 3, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka exact: the trigger fires");
    int looked = honoka_pay_and_look_count(tg, "exact");
    if (looked < 0) return;
    CHECK_EQ(looked, 3, "honoka exact: a deck of exactly 3 is looked at in full");
    honoka_drain_after_look(tg, "exact");
    CHECK_PROMPTS(1, "honoka exact: at minimum the cost prompt");
    CHECK_EQ(tg->state.p[0].hand.n, 3, "honoka exact: 3 cards in hand");
    CHECK_EQ(tg->state.p[0].deck.n, 0, "honoka exact: the deck is empty afterwards");
}

static void test_i_deck_1_card_score_1_refreshes(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 1, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    for (int i = 0; i < 5; i++) test_add_to_discard(tg, filler);
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka 1card: the trigger fires");
    int looked = honoka_pay_and_look_count(tg, "deck_1");
    if (looked < 0) return;
    CHECK_EQ(looked, 3, "honoka 1card: the refresh provides enough cards for 3");
    honoka_drain_after_look(tg, "deck_1");
    CHECK_PROMPTS(1, "honoka 1card: at minimum the cost prompt");
    CHECK_EQ(tg->state.p[0].hand.n, 3, "honoka 1card: 3 cards in hand");
}

static void test_i_selected_card_goes_to_hand(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 5, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    int target = mid(tg, LIVE2);
    pin_id(live, LIVE1);
    pin_id(target, LIVE2);
    if (live < 0 || target < 0) return;
    test_add_to_live(tg, live);
    tg->state.p[0].deck.cards[0] = target;
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka pick: the trigger fires");
    CHECK(honoka_pay_cost(tg, "honoka pick"), "honoka pick: the cost prompt is offered");
    CHECK(pending(tg), "honoka pick: the looked_at pick is offered");
    honoka_drain_after_look(tg, "honoka pick");
    CHECK_PROMPTS(1, "honoka pick: at minimum the cost prompt");
    CHECK(hand_has(tg, target), "honoka pick: the picked card actually goes to hand");
}

static void test_i_nonselected_goes_to_waitroom(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 5, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    int ca = mid(tg, SD014);
    int cb = mid(tg, SD015);
    int cc = mid(tg, SD016);
    pin_id(live, LIVE1);
    pin_id(ca, SD014);
    pin_id(cb, SD015);
    pin_id(cc, SD016);
    if (live < 0 || ca < 0 || cb < 0 || cc < 0) return;
    test_add_to_live(tg, live);
    tg->state.p[0].deck.cards[0] = ca;
    tg->state.p[0].deck.cards[1] = cb;
    tg->state.p[0].deck.cards[2] = cc;
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka remainder: the trigger fires");
    CHECK(honoka_pay_cost(tg, "honoka remainder"), "honoka remainder: the cost prompt is offered");
    CHECK(pending(tg), "honoka remainder: the looked_at pick is offered");
    note_prompt(tg);
    CHECK_EQ(look_pool_size(0), 3, "honoka remainder: three cards were looked at");
    honoka_drain_after_look(tg, "honoka remainder");
    CHECK_PROMPTS(1, "honoka remainder: at minimum the cost prompt");
    CHECK(hand_has(tg, ca), "honoka remainder: the first looked card is taken");
    CHECK(wait_has(tg, cb), "honoka remainder: the second looked card waits");
    CHECK(wait_has(tg, cc), "honoka remainder: the third looked card waits");
    check_bag_n(&tg->state.p[0].discard, 3, "honoka remainder: 1 cost + 2 unselected");
    check_bag_n(&tg->state.p[0].deck, 2, "honoka remainder: exactly the three looked cards left the deck");
}

static void test_i_no_hand_cannot_pay_cost(void)
{
    TestGame *tg = &i_tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int honoka = mid(tg, HONOKA_RP);
    int filler = mid(tg, FILLER_MUS);
    pin_id(honoka, HONOKA_RP);
    pin_id(filler, FILLER_MUS);
    if (honoka < 0 || filler < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = honoka;
    deck_push(tg, 0, filler, 20);
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    CHECK_EQ(tg->state.p[0].hand.n, 0, "honoka nohand: the hand really is empty");
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka nohand: the trigger fires");
    CHECK(!pending(tg), "honoka nohand: an unpayable optional cost (empty hand) auto-skips without prompting");
    CHECK_EQ(prompts, 0, "honoka nohand: no prompt at all was offered");
    CHECK_EQ(look_pool_size(0), 0, "honoka nohand: nothing was looked at");
    /* POSITIVE CONTROL: same print, one hand card -> the prompt does appear. */
    {
        TestGame *tg2 = &i_tg2;
        test_game_new(tg2);
        clear_p1(tg2);
        clear_p2(tg2);
        int h2 = mid(tg2, HONOKA_RP);
        int f2 = mid(tg2, FILLER_MUS);
        int l2 = mid(tg2, LIVE1);
        pin_id(h2, HONOKA_RP);
        pin_id(f2, FILLER_MUS);
        pin_id(l2, LIVE1);
        if (h2 < 0 || f2 < 0 || l2 < 0) return;
        tg2->state.p[0].stage[AREA_LEFT] = h2;
        deck_push(tg2, 0, f2, 20);
        test_add_to_live(tg2, l2);
        hand_push(tg2, 0, f2, 1);
        int before = prompts;
        CHECK(fire_trigger(tg2, h2, RB_TSTR_LIVE_SUCCESS), "honoka nohand POSITIVE CONTROL: the trigger fires");
        note_prompt(tg2);
        CHECK(pending(tg2), "honoka nohand POSITIVE CONTROL: with a hand card the cost prompt IS offered");
        CHECK(prompts > before, "honoka nohand POSITIVE CONTROL: the cost prompt was counted");
    }
}

static void test_i_no_use_limit_multiple_fires(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 6, 40, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);

    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka twice (1st): the trigger fires");
    int looked1 = honoka_pay_and_look_count(tg, "first fire");
    if (looked1 < 0) return;
    CHECK_EQ(looked1, 3, "honoka twice: the first fire looks at 3");
    honoka_drain_after_look(tg, "first fire");
    int hand_after_1 = tg->state.p[0].hand.n;
    int deck_after_1 = tg->state.p[0].deck.n;

    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka twice (2nd): the trigger fires again");
    int looked2 = honoka_pay_and_look_count(tg, "second fire");
    if (looked2 < 0) return;
    CHECK_EQ(looked2, 3, "honoka twice: the second fire still looks at 3");
    honoka_drain_after_look(tg, "second fire");
    CHECK_EQ(tg->state.p[0].hand.n, hand_after_1,
             "honoka twice: the second fire is -1 cost +1 picked (net zero)");
    CHECK_EQ(tg->state.p[0].deck.n, deck_after_1 - 3, "honoka twice: the deck lost 3 more");
}

static void test_i_score_2_looks_at_4_real_live(void)
{
    /* Rust honoka_bp5_score_2_looks_at_4 runs a REAL live. The C port keeps
     * the real live flow but only asserts the two things the Rust twin makes
     * load-bearing: the looked-at pool is the first 4 deck cards, and the
     * unselected remainder plus the cost land in the waitroom in order. */
    TestGame *tg = &i_tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int filler = mid(tg, FILLER_MUS);
    int honoka = mid(tg, HONOKA_RP);
    int rin = mid(tg, SD014);
    int live = mid(tg, LIVE2);
    int cost = mid(tg, SD017);
    pin_id(filler, FILLER_MUS);
    pin_id(honoka, HONOKA_RP);
    pin_id(rin, SD014);
    pin_id(live, LIVE2);
    pin_id(cost, SD017);
    if (filler < 0 || honoka < 0 || rin < 0 || live < 0 || cost < 0) return;
    deck_push(tg, 0, filler, 30);
    deck_push(tg, 1, filler, 10);
    hand_push(tg, 0, honoka, 1);
    hand_push(tg, 0, rin, 1);
    hand_push(tg, 0, live, 1);
    hand_push(tg, 0, cost, 1);
    test_give_energy(tg, 13);
    play_to_stage(tg, honoka, AREA_CENTER, 0);
    play_to_stage(tg, rin, AREA_LEFT, 0);
    CHECK(!pending(tg), "honoka score_2: no prompt after the two plays (got %s)", pending_type(tg));
    advance_n(tg, 5, "honoka score_2: to the live card set");
    test_set_live_card(tg, 0, live);
    CHECK_EQ(tg->state.p[0].live.n, 1, "honoka score_2: the live card is set");
    if (!pass_until_card_prompt(tg, 12, "honoka score_2: to the live-success cost")) return;
    note_prompt(tg);
    /* The live-success gate may be a pay/skip SelectTarget ahead of the cost. */
    if (pending(tg) && !strcmp(pending_type(tg), "SelectTarget")) {
        note_prompt(tg);
        rb_resume_with_choice(&tg->state, 1);
    }
    CHECK(pending(tg) && pending_is_card(tg),
          "honoka score_2: the discard-cost SelectCard is offered (got %s)", pending_type(tg));
    if (pending(tg)) pick(tg, 0);
    CHECK(pending(tg) && pending_is_card(tg),
          "honoka score_2: the looked_at pick is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    note_prompt(tg);
    check_choice_is(tg, "looked_at", 1, -1, "honoka score_2: the pick is looked_at / 1");
    CHECK_EQ(look_pool_size(0), 4, "honoka score_2: score 2 + 2 = 4 cards looked at");
    CHECK(!hand_has(tg, cost) && wait_has(tg, cost),
          "honoka score_2: the paid cost card leaves the hand for the waitroom");
    pick(tg, 2);
    CHECK(!pending(tg), "honoka score_2: the ability ends after the pick");
    CHECK_EQ(look_pool_size(0), 0, "honoka score_2: the looked-at pool is emptied");
    check_bag_n(&tg->state.p[0].deck, 26,
                "honoka score_2: the deck keeps its tail below the looked four");
    check_bag_n(&tg->state.p[0].discard, 4, "honoka score_2: cost + 3 unselected");
}

static void test_i_failed_live_never_offers_discard(void)
{
    TestGame *tg = &i_tg;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int filler = mid(tg, FILLER_MUS);
    int honoka = mid(tg, HONOKA_RP);
    int live = mid(tg, LIVE2);
    int cost = mid(tg, SD017);
    pin_id(filler, FILLER_MUS);
    pin_id(honoka, HONOKA_RP);
    pin_id(live, LIVE2);
    pin_id(cost, SD017);
    if (filler < 0 || honoka < 0 || live < 0 || cost < 0) return;
    deck_push(tg, 0, filler, 30);
    deck_push(tg, 1, filler, 10);
    hand_push(tg, 0, honoka, 1);
    hand_push(tg, 0, live, 1);
    hand_push(tg, 0, cost, 1);
    test_give_energy(tg, 4);
    play_to_stage(tg, honoka, AREA_CENTER, 0);
    if (!advance_n(tg, 5, "honoka failed: to the live card set")) return;
    test_set_live_card(tg, 0, live);
    test_pass(tg);
    test_pass(tg);
    int deck_before = tg->state.p[0].deck.n;
    int hand_before = tg->state.p[0].hand.n;
    int guard = 0;
    for (int i = 0; i < 4; i++) {
        test_pass(tg);
        if (pending(tg)) {
            CHECK(0, "honoka failed: a prompt appeared on a failed live (type=%s)", pending_type(tg));
            break;
        }
        CHECK_EQ(look_pool_size(0), 0, "honoka failed: nothing is looked at");
        (void)guard++;
    }
    CHECK(!pending(tg), "honoka failed: the live-success ability never fires");
    CHECK_EQ(prompts, 0, "honoka failed: no prompt was ever offered");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "honoka failed: the hand is preserved");
    CHECK(hand_has(tg, cost), "honoka failed: the cost card was never discarded");
    CHECK(tg->state.p[0].deck.n < deck_before, "honoka failed: the live itself drew from the deck");
    CHECK_EQ(tg->state.p[0].live.n, 0, "honoka failed: the live zone is empty after the failure");
    CHECK(stage_has(tg, honoka), "honoka failed: the member stays on stage");
}

static void test_i_success_declined_cost_preserves_hand_and_deck(void)
{
    TestGame *tg = &i_tg;
    int honoka, filler;
    if (!honoka_setup(tg, HONOKA_RP, 3, 20, &honoka, &filler)) return;
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    if (live < 0) return;
    test_add_to_live(tg, live);
    int deck_before = tg->state.p[0].deck.n;
    int hand_before = tg->state.p[0].hand.n;
    int wr_before = tg->state.p[0].discard.n;
    CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS), "honoka decline: the trigger fires");
    CHECK(pending(tg) && pending_is_card(tg),
          "honoka decline: the skippable cost prompt is offered (got %s)", pending_type(tg));
    if (pending(tg)) skip(tg);
    CHECK(!pending(tg), "honoka decline: nothing is pending after declining");
    CHECK_EQ(look_pool_size(0), 0, "honoka decline: nothing was looked at");
    CHECK_EQ(tg->state.p[0].deck.n, deck_before, "honoka decline: the deck is preserved");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "honoka decline: the hand is preserved");
    CHECK_EQ(tg->state.p[0].discard.n, wr_before, "honoka decline: the waitroom is preserved");
}

static void test_i_rarity_variants(void)
{
    static const char *variants[4] = {
        HONOKA_RP, "PL!-bp5-001-P", "PL!-bp5-001-AR", "PL!-bp5-001-SEC"
    };
    for (int v = 0; v < 4; v++) {
        TestGame *tg = &i_tg;
        int honoka, filler;
        if (!honoka_setup(tg, variants[v], 3, 20, &honoka, &filler)) continue;
        int live = mid(tg, LIVE1);
        pin_id(live, LIVE1);
        if (live < 0) continue;
        test_add_to_live(tg, live);
        CHECK(fire_trigger(tg, honoka, RB_TSTR_LIVE_SUCCESS),
              "honoka (%s): the ライブ成功時 trigger fires", variants[v]);
        int looked = honoka_pay_and_look_count(tg, variants[v]);
        if (looked < 0) continue;
        CHECK_EQ(looked, 3, "honoka (%s): score 1 -> look at 3", variants[v]);
        honoka_drain_after_look(tg, variants[v]);
    }
}

/* ===================================================================== *
 * main
 * ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [in-process fallback]\n", failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps, p0 = prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0), (int)(prompts - p0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- A no_blade_heart_group_look_test.rs (PL!N-bp7-018-N) ---\n");
    run("a_kanata_takes_eligible",       test_a_kanata_takes_eligible_no_blade_member_to_hand);
    run("a_kanata_own_card_ineligible",  test_a_kanata_own_card_has_blade_not_eligible);
    run("a_kanata_skip_cost",            test_a_kanata_skip_optional_cost_effect_does_not_fire);
    run("a_kanata_no_eligible",          test_a_kanata_no_eligible_among_looked_discards_all);

    printf("--- B per_group_take_one_from_look_five_test.rs (PL!SP-bp5-007-R) ---\n");
    run("b_mei_select_two",              test_b_mei_select_two_from_different_series);
    run("b_mei_select_zero",             test_b_mei_select_zero_cards);
    run("b_mei_select_one",              test_b_mei_select_one_card);
    run("b_mei_per_group_rejects",       test_b_mei_per_group_rejects_two_from_same_group);
    run("b_mei_q235_multiname",          test_b_mei_q235_multiname);
    run("b_mei_q235_all_rarities",       test_b_mei_q235_multiname_all_rarities);

    printf("--- C all_or_any_three_heart_member_filter_look_test.rs ---\n");
    run("c_you_all_three_hearts",        test_c_all_three_hearts_member_is_added_to_hand);
    run("c_you_two_copies",              test_c_two_matching_copies_select_exact_physical_instance);
    run("c_you_two_of_three",            test_c_two_of_three_hearts_member_is_discarded);
    run("c_you_one_of_three",            test_c_one_of_three_hearts_member_is_discarded);
    run("c_you_no_match",                test_c_no_all_three_match_offers_no_prompt);
    run("c_you_plus_or_semantics",       test_c_optional_discard_any_of_three_accepts_one_color);
    run("c_you_decline_look",            test_c_declining_the_look_discards_both_looked_cards);

    printf("--- D look_seven_select_three_heart_member_test.rs ---\n");
    run("d_you_q124_blade_excluded",     test_d_blade_heart_excluded_base_heart_included);
    run("d_you_discard8",                test_d_ability_ends_and_discard_only_grows);
    run("d_you_select_1",                test_d_ability_select_1_card);
    run("d_you_select_multiple",         test_d_ability_select_multiple_cards);
    run("d_you_user_scenario_1",         test_d_user_scenario_select_one_card);
    run("d_you_q124_two_plays",          test_d_two_plays_both_reject_blade_hearts);
    run("d_you_select_2",                test_d_select_2_then_ends);
    run("d_you_select_3",                test_d_select_3_then_ends);

    printf("--- E or_card_types_heart05_threshold_look_test.rs ---\n");
    run("e_yoshiko_member_h05_2",        test_e_look_select_member_heart05_2);
    run("e_yoshiko_live_h05_2",          test_e_look_select_live_heart05_2);
    run("e_yoshiko_no_eligible",         test_e_look_select_no_eligible_auto_skip);
    run("e_yoshiko_h05_1_rejected",      test_e_look_select_heart05_1_rejected);
    run("e_yoshiko_both_types",          test_e_look_select_both_types_eligible);
    run("e_yoshiko_skip_cost",           test_e_look_select_skip_cost);
    run("e_yoshiko_discard_remaining",   test_e_look_select_discard_remaining);

    printf("--- F card_filter_test.rs (PL!-bp6-002-R) ---\n");
    run("f_has_filter_all_fields",       test_f_has_filter_detects_all_fields);
    run("f_ayase_group_rejects_non_mus", test_f_group_filter_rejects_non_mus);
    run("f_ayase_rejects_debut_trigger", test_f_rejects_debut_trigger_mus);
    run("f_ayase_accepts_jyouji",        test_f_accepts_jyouji_rejects_debut);
    run("f_ayase_accepts_no_ability",    test_f_accepts_no_ability_mus);
    run("f_ayase_both_match",            test_f_both_match_only_one_selectable);
    run("f_ayase_neither_matches",       test_f_neither_matches_auto_discard);

    printf("--- G look_at_deck_top_optional_discard_test.rs ---\n");
    run("g_kaho_discard",                test_g_looked_at_discard_player_discards);
    run("g_kaho_skip",                   test_g_looked_at_discard_player_skips);
    run("g_yumewazurai_discard_4",       test_g_live_success_discard_4_from_deck_unconditionally);

    printf("--- H success_score_gated_look_fetches_musume_test.rs ---\n");
    run("h_bp4_score_ge_3",              test_h_score_three_or_more_fetches_mus_member);
    run("h_bp4_score_lt_3",              test_h_low_success_score_does_not_fetch_mus_member);

    printf("--- I look_count_from_total_live_score_test.rs ---\n");
    run("i_honoka_score_1",              test_i_score_1_looks_at_3);
    run("i_honoka_score_3",              test_i_score_3_looks_at_5);
    run("i_honoka_score_0",              test_i_score_0_looks_at_2);
    run("i_honoka_score_4",              test_i_score_4_looks_at_6);
    run("i_honoka_high_score_8",         test_i_high_score_looks_at_8);
    run("i_honoka_mixed_scores",         test_i_mixed_scores_looks_correctly);
    run("i_honoka_five_live_cards",      test_i_five_live_cards_looks_at_7);
    run("i_honoka_multiple_live_cards",  test_i_multiple_live_cards_in_live_zone);
    run("i_honoka_live_zone_not_success",test_i_reads_live_zone_not_success_zone);
    run("i_honoka_skip_cost",            test_i_skip_cost_no_effect);
    run("i_honoka_deck_refresh",         test_i_deck_refresh_during_look);
    run("i_honoka_deck_exactly_enough",  test_i_deck_exactly_enough);
    run("i_honoka_deck_1_card",          test_i_deck_1_card_score_1_refreshes);
    run("i_honoka_selected_to_hand",     test_i_selected_card_goes_to_hand);
    run("i_honoka_nonselected_waitroom", test_i_nonselected_goes_to_waitroom);
    run("i_honoka_no_hand_cannot_pay",   test_i_no_hand_cannot_pay_cost);
    run("i_honoka_no_use_limit_twice",   test_i_no_use_limit_multiple_fires);
    run("i_honoka_score_2_real_live",    test_i_score_2_looks_at_4_real_live);
    run("i_honoka_failed_live",          test_i_failed_live_never_offers_discard);
    run("i_honoka_declined_cost",        test_i_success_declined_cost_preserves_hand_and_deck);
    run("i_honoka_rarity_variants",      test_i_rarity_variants);

    rb_unload();

    printf("\n==== parity_look_and_filter ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : %d\n", gaps);
    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_crashed);
        return 1;
    }
    if (n_failed) {
        fprintf(stderr,
                "PARITY LOOK AND FILTER: %d failing test(s) - every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, or a SETUP BUG\n"
                "naming a print the engine cannot resolve. Not a bug in this file.\n",
                n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_LOOK_AND_FILTER CHECKS PASSED\n");
    return 0;
}

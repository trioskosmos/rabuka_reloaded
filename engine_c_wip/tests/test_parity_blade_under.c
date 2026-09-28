/* test_parity_blade_under.c
 *
 * C parity suite for the cluster
 *   engine/tests/test_modules/effects/gain/blades/under_member/
 *     (8 .rs + mod.rs, 44 #[test])
 *
 * The single theme: a card physically placed UNDER a member on the stage, and
 * what the under-card stack buys the member hosting it (or a member hosting
 * some other card).
 *
 *   A under_liella_member_constant_host_blade_test.rs        (10 tests)
 *     PL!SP-bp7-001-R 澁谷かのん 常時: while THIS card is under a 『Liella!』
 *     member, THAT member gains a blade. The grant flows BACKWARDS -- from the
 *     under-card to its host -- which is the opposite direction from every
 *     other test in this file and is the reason the C constant scan matters.
 *
 *   B bp7_watanabe_under_card_blade_test.rs                   (9 tests)
 *     PL!S-bp7-005-R＋ 渡辺 曜: 登場 parks a waitroom member under a CHOSEN
 *     stage member, and ab#1 常時 grants blade to every 『Aqours』 member on
 *     your stage THAT HAS A MEMBER CARD UNDER IT. The grant is a FILTER over
 *     the stage, not a grant to the host of the ability.
 *
 *   C member_under_aqours_constant_blade_test.rs              (1 test)
 *     the minimal shape of B's constant, without the 登場 half.
 *
 *   D bp7_under_member_per_unit_blade_test.rs                 (7 tests)
 *     PL!SP-bp7-003-R＋ 嵐千砂都 常時 per_unit (no distinct) and
 *     PL!N-bp7-003-R＋ 桜坂しずく ライブ開始時 per_unit + distinct:card_name.
 *     The defect these pin is the `location: "under_member"` scope.
 *
 *   E per_under_member_energy_constant_blades_test.rs         (2 tests)
 *     PL!N-pb1-011-R ミア・テイラー 常時: blade per ENERGY under her.
 *
 *   F q278_q279_joint_card_under_member_blade_test.rs          (10 tests)
 *     Q278/Q279: how a 多種統合 JOINT "A&B&C" member counts toward
 *     「名前の異なるメンバーカード1枚につき」 -- one unit, and only when it
 *     brings a name not already present.
 *
 *   G shizuku_under_member_heart_copy_test.rs                 (4 tests)
 *     PL!N-bp7-003-R＋ 起動: move a cost<=17 『虹ヶ咲』 member from the waitroom
 *     under herself, and her ORIGINAL hearts become the placed card's hearts.
 *
 *   H live_start_distinct_under_member_blades_test.rs         (1 test)
 *     the two-distinct-under-members lower bound, driven through a real turn.
 *
 * Every Rust #[test] function name is reproduced verbatim in the `run(...)`
 * list in main() so a missing port is greppable.
 *
 * ---------------------------------------------------------------------------
 * WHAT THE PARSED WIRE ACTUALLY SAYS (cards/abilities.json, the same file the
 * bytecode blob is generated from -- every claim below is read off it, not off
 * the printed card text, so a parser regression is visible here too)
 * ---------------------------------------------------------------------------
 *  PL!SP-bp7-003-R＋ ab#0 常時
 *    gain_resource{blade, count:1, per_unit:true, per_unit_count:1,
 *                  per_unit_type:"枚", card_type:"member_card",
 *                  location:"under_member"}
 *  PL!N-bp7-003-R＋ ab#1 ライブ開始時
 *    gain_resource{blade, count:1, per_unit:true, per_unit_count:1,
 *                  per_unit_type:"枚", card_type:"member_card",
 *                  distinct:"card_name", location:"under_member",
 *                  duration:"live_end"}
 *  PL!N-pb1-011-R ab#0 常時
 *    gain_resource{blade, count:1, per_unit:true, card_type:"energy_card",
 *                  location:"under_member"}
 *  PL!S-bp7-005-R＋ ab#1 常時
 *    gain_resource{blade, count:1, all:true, requires_under_card:true,
 *                  group_names:["Aqours"], card_type:"member_card",
 *                  target:"self"}
 *  PL!SP-bp7-001-R ab#0 常時
 *    gain_resource{blade, count:1, conditional, condition{group_condition,
 *                  group_names:["Liella!"], location:"under_member",
 *                  self_target:true, card_type:"member_card"}}
 *  PL!N-bp7-003-R＋ ab#0 起動
 *    sequential[ move_cards{discard -> under_member, cost_limit:17 "<=",
 *                  group_names:["虹ヶ咲"], card_type:"member_card",
 *                  under_self:true},
 *                set_heart_type{ref_value:"placed_under",
 *                  original_value:true, duration:"live_end"} ]
 *
 * So the cluster needs FOUR distinct engine capabilities, in two different
 * evaluation paths:
 *   1. per_unit scaling on a 常時 gain_resource            (sections C, D1-D3, E)
 *   2. a 常時 ability that lives on an UNDER-card          (section A)
 *   3. a 常時 gain_resource that is a FILTER over the
 *      stage rather than a grant to the host               (sections B, C)
 *   4. per_unit + distinct:card_name on a ライブ開始時     (sections D4-D7,
 *                                                            F, H)
 *
 * ---------------------------------------------------------------------------
 * ANTI-VACUITY DISCIPLINE
 * ---------------------------------------------------------------------------
 * Most of this cluster's assertions are NEGATIVE ("no blade is granted"), and
 * a negative passes for free on a board where the engine did nothing at all.
 * Every negative test here therefore runs a POSITIVE CONTROL arm first --
 * the same fixture with the one variable that the negative is about changed
 * back -- and asserts the control DOES grant. `CHECK(control == expected, ...)`
 * with the message "ANTI-VACUITY CONTROL" is that assertion. Where the engine
 * is broken the control goes red too, which is the honest reading: a negative
 * in a family whose positive arm is dead proves nothing.
 *
 * The trigger shim prints a [TRIGGER-MISS] line dumping every trigger string
 * the card ACTUALLY carries, so a mistyped trigger keyword can never hide
 * behind a passing negative.
 *
 * ---------------------------------------------------------------------------
 * LANDMINES respected (confirmed by other agents)
 * ---------------------------------------------------------------------------
 *   - test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE, so every
 *     heart read here goes through rb_mods_get_heart directly.
 *   - RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   - Rust game.id("X") allocates the NEXT DISTINCT pool slot; C's template
 *     lookup aliases. mid() (test_new_id) is used everywhere.
 *   - rb_create_card_copy draws from a PROCESS-GLOBAL monotonic pool that
 *     nothing resets between cases; new_board() calls rb_card_copy_pool_reset()
 *     per test so a long suite cannot exhaust the 4096 ids and silently fall
 *     back to the shared template.
 *   - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *   - rb_card_no_eq is FUZZY and is NOT an identity pin; ident() compares the
 *     PRINTED number byte-for-byte via rb_card_string().
 *   - Card numbers carrying a FULLWIDTH PLUS (U+FF0B, bytes ef bc 8b) are
 *     written as "\xEF\xBC\x8B": rb_find_card_by_no is EXACT-BYTE and does not
 *     normalise fullwidth->halfwidth the way Rust's CardDatabase::get_card_id
 *     does, so a hand-typed '+' would resolve to a different (or no) print.
 *
 * This file is pure ASCII: every Japanese string literal is an explicit UTF-8
 * byte escape so the file survives any tool that rewrites sources in a legacy
 * code page.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

static int failures;
static int assertions;
static int setup_bugs;
static int n_tests_ok, n_tests_failed, n_tests_crashed;
static const char *current_test = "(none)";

/* ── assertions ──────────────────────────────────────────────────────────── */

#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        failures++; \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)  [%s]\n", message, \
                actual_value, expected_value, current_test); \
        failures++; \
    } \
} while (0)

/* ── trigger keywords (engine/src/ability/enums.rs) ─────────────────────────
 * Byte-checked against the strings the decoder actually prints; the
 * [TRIGGER-MISS] diagnostic in bu_fire_trigger dumps what a card really
 * carries when the match fails, so a typo here can never hide. */
#define TRIG_DEBUT        "\xE7\x99\xBB\xE5\xA0\xB4"                            /* 登場        */
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82" /* ライブ開始時 */
#define TRIG_CONSTANT     "\xE6\x99\x82\xE6\x97\xB6"                            /* 常時        */

/* 起動 = the activation trigger string, used as a trigger_type token. */
#define TRIG_ACTIVATE     "\xE8\xB5\xB7\xE5\x8B\x95"                            /* 起動        */

/* Heart colour slots (src/ability/util.c:rb_parse_heart_color):
 * heart01 -> 1, heart02 -> 2, heart03 -> 3, heart04 -> 4, heart06 -> 6. */
#define H01 1
#define H02 2
#define H03 3
#define H04 4

/* area indices (engine/src/game/zones.rs MemberArea) */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── card numbers ──────────────────────────────────────────────────────────
 * Every one of these was verified against cards/cards.json (read as UTF-8).
 * The four FULLWIDTH-PLUS numbers carry "\xEF\xBC\x8B" and are commented with
 * the name cards.json prints. */
#define KANON_BP7   "PL!SP-bp7-001-R"             /* 澁谷かのん   */
#define WATANABE    "PL!S-bp7-005-R\xEF\xBC\x8B"  /* 渡辺 曜      */
#define CHIKA       "PL!SP-bp7-003-R\xEF\xBC\x8B"  /* 嵐 千砂都    */
#define SHIZUKU     "PL!N-bp7-003-R\xEF\xBC\x8B"  /* 桜坂しずく   */
#define JOINT       "LL-bp1-001-R\xEF\xBC\x8B"    /* 上原歩夢&澁谷かのん&日野下花帆 */
#define HOST_AQOURS "PL!S-bp7-006-R"              /* 津島善子 (Aqours)     */
#define HOST_AQOURS2 "PL!S-bp7-004-R"             /* 黒澤ダイヤ (Aqours)   */
#define NON_AQOURS  "PL!SP-sd1-001-SD"            /* 澁谷かのん (Liella!)  */
#define AQOURS_SD   "PL!S-sd1-015-SD"             /* 津島善子 (Aqours)     */
#define LIELLA_HOST "PL!SP-sd1-004-SD"            /* 平安名すみれ (Liella!)*/
#define LIELLA_HOST2 "PL!SP-sd1-001-SD"           /* 澁谷かのん (Liella!)   */
#define MEMBER_UNDER "PL!-sd1-001-SD"             /* 高坂 穂乃果 (μ's)     */
#define UNDER_AQOURS1 "PL!S-bp2-001-R"            /* 高海千歌 (Aqours)     */
#define UNDER_AQOURS2 "PL!S-bp2-002-R"            /* 桜内梨子 (Aqours)     */
#define MIA         "PL!N-pb1-011-R"              /* ミア・テイラー        */
#define AYUMU       "PL!N-bp1-001-R"              /* 上原歩夢              */
#define KANON_BP1   "PL!SP-bp1-001-R"             /* 澁谷かのん            */
#define HANABO      "PL!HS-bp1-001-R"             /* 日野下花帆            */
#define AYUMU_SD    "PL!N-sd1-001-SD"             /* 上原歩夢              */
#define KASUMI_SD   "PL!N-sd1-002-SD"             /* 中須かすみ            */
#define HONOKA_SD   "PL!-sd1-010-SD"              /* 高坂 穂乃果 (μ's)     */
#define ENERGY_NO   "LL-E-001-SD"                 /* エネルギーカード      */
#define FILLER_NO   "PL!-sd1-010-SD"              /* 高坂 穂乃果 (μ's)     */
#define LIVE_NO     "PL!-sd1-020-SD"              /* きっと青春が聞こえる  */

/* ── card DB ─────────────────────────────────────────────────────────────── */

/* `src` is the in-tree layout; an isolated out-of-tree build resolves the
 * blobs through ../cards/build, so both paths are tried. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Rust TestGame::id() allocates the NEXT DISTINCT pool slot, so two
 * game.id("X") calls are two different card instances. C's plain template
 * lookup aliases the shared template; test_new_id() is the distinct copy. */
static int mid(TestGame *tg, const char *no)
{
    return test_new_id(tg, no);
}

/* Pin identity EXACTLY.
 *
 * rb_find_card_by_no() has a fuzzy tail and rb_card_no_eq() is fuzzy too --
 * a peer found rb_card_no_eq("PL!SP-PR-004-R", ...) happily match the real
 * print "PL!SP-PR-004-PR". The guard below therefore compares the PRINTED
 * number byte-for-byte via rb_card_string(), the only check a near miss
 * cannot satisfy. A mistyped card number here is a SETUP BUG, not an engine
 * verdict, and is reported as such rather than absorbed into the verdict. */
static int ident(int id, const char *no)
{
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: card \"%s\" did not resolve in the card DB\n", no);
        setup_bugs++;
        assertions++;
        return 0;
    }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: card %s (id %d) failed to decode\n", no, id);
        setup_bugs++;
        assertions++;
        return 0;
    }
    char got[128];
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr,
                "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n", id, got, no);
        setup_bugs++;
        assertions++;
        return 0;
    }
    return 1;
}

/* Resolve + pin in one call: the common case. Returns the id, or -1 after
 * reporting a setup bug (the caller then returns early so no board assertion
 * is reported against the wrong card). */
static int card(TestGame *tg, const char *no)
{
    int id = mid(tg, no);
    if (!ident(id, no)) return -1;
    return id;
}

static int printed_cost(int id)
{
    Card c;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

/* ── board helpers ───────────────────────────────────────────────────────── */

/* Empty every zone both seats own so each card-count assertion is unambiguous. */
static void bare_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            p->stage[i] = RB_EMPTY_SLOT;
            p->stage_wait[i] = 0;
            p->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_recently_moved = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    tg->state.queue.deferred = NULL;
    tg->state.queue.resume_eff = NULL;
    tg->state.activation_keepalive_valid = 0;
    rb_clear_pending_choice(&tg->state);
}

/* Every test starts here. The pool reset is REQUIRED, not cosmetic: the copy
 * pool is process-global and monotonic, and a 44-case suite that lets it run
 * dry gets -1 from test_new_id, after which an id()-style wrapper silently
 * falls back to the shared template and every identity assertion goes
 * vacuous. */
static void new_board(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    bare_board(tg);
}

/* Rust `game.state.player1.stage.stage = [-1, x, -1]` etc. */
static void set_stage(TestGame *tg, int pl, int left, int center, int right)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[AREA_LEFT] = left;
    P->stage[AREA_CENTER] = center;
    P->stage[AREA_RIGHT] = right;
}

static int under_n(TestGame *tg, int pl, int area)
{
    return tg->state.p[pl].under_cards[area].n;
}

static int under_has(TestGame *tg, int pl, int area, int card_id)
{
    const RbBag *b = &tg->state.p[pl].under_cards[area];
    for (int i = 0; i < b->n; i++) if (b->cards[i] == card_id) return 1;
    return 0;
}

static int bag_has(const RbBag *b, int card_id)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == card_id) return 1;
    return 0;
}

static int waitroom_has(TestGame *tg, int card_id)
{
    return bag_has(&tg->state.p[0].discard, card_id);
}

static int hand_has(TestGame *tg, int card_id)
{
    return bag_has(&tg->state.p[0].hand, card_id);
}

static int blade_of(TestGame *tg, int card_id)
{
    return rb_mods_get_blade(&tg->state.mods, card_id);
}

static int heart_of(TestGame *tg, int card_id, int slot)
{
    return rb_mods_get_heart(&tg->state.mods, card_id, slot);
}

static int heart_copy_of(TestGame *tg, int card_id)
{
    return rb_mods_get_heart_copy(&tg->state.mods, card_id);
}

/* Rust `fn seed_deck(game)`: 10 fillers in BOTH main decks. */
static void seed_deck(TestGame *tg, int filler)
{
    for (int i = 0; i < 10; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* live_start_distinct_under_member_blades_test::fill_both_main_decks: 20. */
static void fill_both_main_decks(TestGame *tg, int filler)
{
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* ── phase walking ───────────────────────────────────────────────────────── */

static void pass_n(TestGame *tg, int n)
{
    for (int i = 0; i < n; i++) test_pass(tg);
}

/* Rust advance_to_live_card_set_p1: five passes. */
static void advance_to_live_set(TestGame *tg) { pass_n(tg, 5); }
/* Rust advance_to_live_start: two more passes. */
static void advance_to_live_start(TestGame *tg) { pass_n(tg, 2); }

/* ── choice answering ────────────────────────────────────────────────────── */
#define BU_SKIP   (-1)
#define BU_ACCEPT (0)

static int bu_prompt(TestGame *tg) { return test_has_pending_choice(tg); }
static int bu_prompt_kind(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? (int)c->kind : (int)RB_CHOICE_NONE;
}
static int bu_prompt_count(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->count : -1;
}
static const char *bu_prompt_zone(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return (c && c->zone[0]) ? c->zone : "";
}

/* `while has_pending_choice() { select_indices(&[0]) }` -- the Rust
 * activate_and_resolve / q278 drain shape: answer ACCEPT on every prompt. */
static int bu_drain_pick0(TestGame *tg, int rounds)
{
    int guard = 0;
    while (bu_prompt(tg) && guard++ < rounds) {
        rb_resume_with_choice(&tg->state, BU_ACCEPT);
        rb_process_pending_auto_abilities(&tg->state);
    }
    return guard;
}

/* PROCEED the auto-ability step, DECLINE everything else. See
 * test_parity_under_member.c for why the skip index must never be used on a
 * SelectAutoAbility: it refuses the WHOLE ability and the negative
 * assertions then pass on a board where nothing ran. */
static int bu_drain_skip(TestGame *tg, int rounds)
{
    int guard = 0;
    while (bu_prompt(tg) && guard++ < rounds) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) ? BU_ACCEPT : BU_SKIP;
        rb_resume_with_choice(&tg->state, idx);
        rb_process_pending_auto_abilities(&tg->state);
    }
    return guard;
}

/* Answer with one explicit index (Rust's `select_indices(&[i])`), then let the
 * auto-ability queue settle. Used where the CHOICE of index is the claim
 * (which stage member the 渡辺 曜 登場 parks the card under). */
static int bu_pick_index(TestGame *tg, int idx, int rounds)
{
    int guard = 0;
    while (bu_prompt(tg) && guard++ < rounds) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int use = (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) ? BU_ACCEPT : idx;
        rb_resume_with_choice(&tg->state, use);
        rb_process_pending_auto_abilities(&tg->state);
    }
    return guard;
}

/* ── trigger shim (Rust helpers::fire_trigger) ─────────────────────────────
 * Build the "<card_no>_<full_text>" ability id for the nth ability printing
 * `trig`, queue exactly that ability, set activating_card, process the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int bu_fire_trigger(TestGame *tg, int card_id, const char *trig,
                           const char *type)
{
    Card c;
    if (card_id < 0 || !rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", rb_card_string(c.card_no_idx));
    char ability_id[768];
    int found = 0;
    int nab = rb_card_num_abilities((uint32_t)card_id);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&c);
    if (!found) {
        /* Diagnostic: which trigger strings does this card actually carry?
         * A mismatch here is a HARNESS bug, not an engine bug, so it must be
         * loud rather than silently turning into a vacuous negative. */
        fprintf(stderr, "[TRIGGER-MISS] card %s: no ability with triggers \"%s\"; has:",
                card_no, trig);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)card_id, a, &ab)) continue;
            fprintf(stderr, " [%d]=\"%s\"", a, ab.triggers ? ab.triggers : "(null)");
            rb_free_ability(&ab);
        }
        fprintf(stderr, "\n");
        return 0;
    }
    rb_trigger_auto_ability(&tg->state, ability_id, type, 0, card_no, card_id,
                            NULL, 0, -1);
    tg->state.activating_card = card_id;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int bu_fire_live_start(TestGame *tg, int card_id)
{
    return bu_fire_trigger(tg, card_id, TRIG_LIVE_START, TRIG_LIVE_START);
}

/* ── diagnostic dump (RB_BLADE_UNDER_DUMP="PL!N-bp7-003-R＋,PL!SP-bp7-001-R") ── */
static void bu_dump_effect(const AbilityEffect *e, const char *pfx)
{
    if (!e) return;
    fprintf(stderr, "%s action=%s target=%s source=%s dest=%s count=%d cond=%d\n",
            pfx, e->action ? e->action : "-", e->target ? e->target : "-",
            e->source ? e->source : "-", e->destination ? e->destination : "-",
            e->count, e->has_condition ? 1 : 0);
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, "%s   extra[%d]=%s -> %s\n", pfx, i,
                e->extra_k[i] ? e->extra_k[i] : "-",
                e->extra_v[i] ? e->extra_v[i] : "-");
    for (int i = 0; i < e->n_child; i++)
        bu_dump_effect(e->child[i], "      child");
}

static void bu_dump_card(const char *no)
{
    int id = rb_find_card_by_no(no);
    if (id < 0) { fprintf(stderr, "[DUMP] %s: NOT FOUND\n", no); return; }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) { fprintf(stderr, "[DUMP] %s: decode failed\n", no); return; }
    char got[128];
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    fprintf(stderr, "[DUMP] %s -> id %d, printed \"%s\" cost=%d blade=%d\n",
            no, id, got, (int)c.cost, (int)c.blade);
    rb_free_card(&c);
    int nab = rb_card_num_abilities((uint32_t)id);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)id, a, &ab)) continue;
        fprintf(stderr, "  [DUMP %s ab#%d] triggers=\"%s\" cost=%s effect=%s\n",
                no, a, ab.triggers ? ab.triggers : "(null)",
                ab.cost ? "yes" : "no", ab.effect ? "yes" : "no");
        bu_dump_effect(ab.effect, "   ");
        rb_free_ability(&ab);
    }
}

/* ========================================================================
 * SECTION A
 * under_liella_member_constant_host_blade_test.rs
 * PL!SP-bp7-001-R 澁谷かのん 常時: the grant flows from the UNDER-CARD to its
 * Liella! host, so this section is the only one in the file where the card
 * carrying the 常時 ability is not on the stage.
 * ===================================================================== */

/* Run the canonical POSITIVE board: 澁谷かのん under a Liella! host at Center.
 * Every negative in this section calls it first as its anti-vacuity control,
 * so a control value of 0 means "the engine granted nothing here at all" and
 * the negative that follows proves nothing. Returns -1 on a setup bug. */
static int a_control_liella_under_center(void)
{
    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return -1;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    test_recalc(&tg);
    return blade_of(&tg, host);
}

/* A1 :: under_liella_host_blade_under_liella_host_gains_blade */
static void A_under_liella_host_gains_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: 澁谷かのん is under Center");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 1,
             "Liella! host with 澁谷かのん underneath -> 1 blade");
}

/* A2 :: under_liella_host_blade_on_stage_grants_no_blade */
static void A_on_stage_grants_no_blade(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1,
             "ANTI-VACUITY CONTROL: the same fixture WITH 澁谷かのん under the "
             "host must grant 1 blade -- otherwise the zeros below are free");

    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return;
    set_stage(&tg, 0, kanon, host, RB_EMPTY_SLOT);   /* both ON the stage */
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "setup: nothing is under anybody");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0,
             "澁谷かのん beside the host, not under it -> host gains 0");
    CHECK_EQ(blade_of(&tg, kanon), 0,
             "澁谷かのん on the stage (not under a member) -> 0 blade");
}

/* A3 :: under_liella_host_blade_under_non_liella_host_grants_no_blade */
static void A_under_non_liella_host_grants_no_blade(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1,
             "ANTI-VACUITY CONTROL: the Liella! host arm must grant 1 blade");

    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, AQOURS_SD);     /* 津島善子, Aqours */
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: 澁谷かのん is under Center");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0,
             "Aqours host -> the 『Liella!』 group scope refuses the grant");
}

/* A4 :: under_liella_host_blade_absent_grants_no_blade */
static void A_absent_grants_no_blade(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1,
             "ANTI-VACUITY CONTROL: with 澁谷かのん under the host the grant must "
             "be 1 -- so the 0 below is the missing under-card, not a dead engine");

    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    if (host < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "setup: nothing under the host");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0, "no under-card -> host gains 0");
}

/* A5 :: two_under_member_sources_grant_two_blades_to_one_liella_host */
static void A_two_sources_two_blades_to_one_host(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: one 澁谷かのん under -> 1 blade");

    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int k1 = card(&tg, KANON_BP7);
    int k2 = card(&tg, KANON_BP7);
    if (host < 0 || k1 < 0 || k2 < 0) return;
    CHECK(k1 != k2, "setup: two DISTINCT instances of the print");
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, k1);
    test_place_under(&tg, 0, AREA_CENTER, k2);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2, "setup: 2 copies under Center");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 2,
             "2 copies of 澁谷かのん under one Liella! host -> 2 blade "
             "(each copy's 常時 is independent)");
}

/* A6 :: under_liella_host_blade_under_center_does_not_leak_to_other_slots */
static void A_under_center_does_not_leak(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the Center arm must grant 1 blade");

    static TestGame tg;
    new_board(&tg);
    int center_host = card(&tg, LIELLA_HOST);
    int right_host = card(&tg, LIELLA_HOST2);
    int kanon = card(&tg, KANON_BP7);
    if (center_host < 0 || right_host < 0 || kanon < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, center_host, right_host);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, center_host), 1, "Center host with 澁谷かのん under -> 1");
    CHECK_EQ(blade_of(&tg, right_host), 0,
             "right host (Liella! but nothing under) -> 0 blade");
}

/* A7 :: under_liella_host_blade_in_hand_contributes_nothing */
static void A_in_hand_contributes_nothing(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the under-card arm must grant 1");

    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, kanon);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "setup: 澁谷かのん is in hand");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0, "澁谷かのん only in hand -> host gains 0");
}

/* A8 :: under_liella_host_blade_copies_under_two_liella_hosts */
static void A_copies_under_two_hosts(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the Center arm must grant 1");

    static TestGame tg;
    new_board(&tg);
    int center_host = card(&tg, LIELLA_HOST);
    int right_host = card(&tg, LIELLA_HOST2);
    int ka = card(&tg, KANON_BP7);
    int kb = card(&tg, KANON_BP7);
    if (center_host < 0 || right_host < 0 || ka < 0 || kb < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, center_host, right_host);
    test_place_under(&tg, 0, AREA_CENTER, ka);
    test_place_under(&tg, 0, AREA_RIGHT, kb);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, center_host), 1, "Center host with 澁谷かのん under -> 1");
    CHECK_EQ(blade_of(&tg, right_host), 1, "Right host with 澁谷かのん under -> 1");
}

/* A9 :: under_liella_host_blade_under_opponent_liella_host_gains_blade */
static void A_under_opponent_liella_host_gains_blade(void)
{
    int control = a_control_liella_under_center();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the P1 arm must grant 1");

    static TestGame tg;
    new_board(&tg);
    int p2_host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (p2_host < 0 || kanon < 0) return;
    set_stage(&tg, 1, RB_EMPTY_SLOT, p2_host, RB_EMPTY_SLOT);
    test_place_under(&tg, 1, AREA_CENTER, kanon);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, p2_host), 1,
             "the constant is not player-scoped: an OPPONENT Liella! host under "
             "which 澁谷かのん is stacked also gains 1 blade");
}

/* A10 :: under_liella_host_blade_leaving_removes_blade
 * Dynamic removal: 澁谷かのん leaves the under-stack -> recalc must drop the
 * grant, with no stale modifier. */
static void A_leaving_removes_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int host = card(&tg, LIELLA_HOST);
    int kanon = card(&tg, KANON_BP7);
    if (host < 0 || kanon < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, host), 1, "with 澁谷かのん under -> 1 blade");

    int out_wait[RB_STAGE_SIZE * 16], n_wait = 0;
    int out_energy[RB_STAGE_SIZE * 16], n_energy = 0;
    rb_stage_recycle_under_cards(&tg.state, 0, AREA_CENTER,
                                 out_wait, &n_wait, out_energy, &n_energy,
                                 RB_STAGE_SIZE * 16);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "the under stack is emptied");
    CHECK_EQ(n_wait + n_energy, 1, "the recycled card is accounted for");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, host), 0,
             "after 澁谷かのん leaves -> 0 blade (no stale modifier)");
}

/* ========================================================================
 * SECTION B
 * bp7_watanabe_under_card_blade_test.rs
 * PL!S-bp7-005-R＋ 渡辺 曜: the 常時 grant is a FILTER over your own stage
 * (every 『Aqours』 member that HAS a member card under it), not a grant to
 * the member carrying the ability. The 登場 half parks a waitroom member
 * under a CHOSEN stage member.
 * ===================================================================== */

/* The canonical POSITIVE board for this section: 渡辺 曜 on Right, an Aqours
 * host on Center with a member card under it. Anti-vacuity control for every
 * negative below. */
static int b_control_aqours_host_under_member(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return -1;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, watanabe);
    test_place_under(&tg, 0, AREA_CENTER, under);
    test_recalc(&tg);
    return blade_of(&tg, host);
}

/* B1 :: watanabe_aqours_host_with_member_under_gains_blade */
static void B_aqours_host_with_member_under_gains_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, watanabe);
    test_place_under(&tg, 0, AREA_CENTER, under);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: a member card is under the host");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 1,
             "『Aqours』 member with a member card under it -> 1 blade");
    CHECK_EQ(blade_of(&tg, watanabe), 0,
             "the grant goes to the FILTERED member, not to 渡辺 曜 herself");
}

/* B2 :: watanabe_aqours_host_without_under_gains_nothing */
static void B_aqours_host_without_under_gains_nothing(void)
{
    int control = b_control_aqours_host_under_member();
    CHECK_EQ(control, 1,
             "ANTI-VACUITY CONTROL: the same host WITH a member card under it "
             "must gain 1 blade");

    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    if (watanabe < 0 || host < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, watanabe);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "setup: nothing under the host");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0, "Aqours host with nothing underneath -> 0");
}

/* B3 :: watanabe_aqours_host_with_energy_under_gains_nothing */
static void B_aqours_host_with_energy_under_gains_nothing(void)
{
    int control = b_control_aqours_host_under_member();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: a MEMBER under the host grants 1");

    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int energy = card(&tg, ENERGY_NO);
    if (watanabe < 0 || host < 0 || energy < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, watanabe);
    test_place_under(&tg, 0, AREA_CENTER, energy);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: the ENERGY card is under the host");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0,
             "only an ENERGY card underneath -> 0 (the text needs a member card)");
}

/* B4 :: watanabe_non_aqours_host_with_member_under_gains_nothing */
static void B_non_aqours_host_with_member_under_gains_nothing(void)
{
    int control = b_control_aqours_host_under_member();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the 『Aqours』 host must gain 1");

    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, NON_AQOURS);      /* 澁谷かのん, Liella! */
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, host, watanabe);
    test_place_under(&tg, 0, AREA_CENTER, under);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, host), 0,
             "non-『Aqours』 host with a member card under it -> 0 blade (group scope)");
}

/* B5 :: watanabe_self_no_under_gains_nothing */
static void B_self_no_under_gains_nothing(void)
{
    int control = b_control_aqours_host_under_member();
    CHECK_EQ(control, 1, "ANTI-VACUITY CONTROL: the filtered host must gain 1");

    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    if (watanabe < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, watanabe, RB_EMPTY_SLOT);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "setup: nothing under 渡辺 曜");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, watanabe), 0,
             "渡辺 曜 alone on stage with no card under her -> 0 blade (no self-grant)");
}

/* B6 :: watanabe_two_hosts_with_members_under_both_gain */
static void B_two_hosts_both_gain(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int a = card(&tg, HOST_AQOURS);
    int b = card(&tg, HOST_AQOURS2);
    int u1 = card(&tg, MEMBER_UNDER);
    int u2 = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || a < 0 || b < 0 || u1 < 0 || u2 < 0) return;
    set_stage(&tg, 0, watanabe, a, b);
    test_place_under(&tg, 0, AREA_CENTER, u1);
    test_place_under(&tg, 0, AREA_RIGHT, u2);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: a member under Center");
    CHECK_EQ(under_n(&tg, 0, AREA_RIGHT), 1, "setup: a member under Right");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, a), 1, "center 『Aqours』 host -> 1 blade");
    CHECK_EQ(blade_of(&tg, b), 1, "right 『Aqours』 host -> 1 blade");
    CHECK_EQ(blade_of(&tg, watanabe), 0, "渡辺 曜 herself -> 0 blade");
}

/* B7 :: watanabe_debut_offers_which_member_to_place_under
 * ab#0 must raise a SelectCard over the STAGE asking which member receives
 * the waitroom card. */
static void B_debut_offers_which_member_to_place_under(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return;
    CHECK_EQ(printed_cost(watanabe), 15,
             "fixture guard: 渡辺 曜 costs 15 (cards.json: cost 15)");
    test_add_to_hand(&tg, watanabe);
    test_add_to_discard(&tg, under);
    set_stage(&tg, 0, host, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 15);

    CHECK_EQ(test_play_to_stage(&tg, watanabe, AREA_CENTER), 1,
             "play_to_stage: 渡辺 曜 reaches Center");
    CHECK(bu_prompt(&tg),
          "PROMPT CONTROL: ab#0 must prompt which member to place the card under");
    CHECK_EQ((int)bu_prompt_kind(&tg), (int)RB_CHOICE_SELECT_CARD,
             "the prompt is a SelectCard");
    CHECK_EQ(bu_prompt_count(&tg), 1, "exactly 1 stage member is requested");
    CHECK(!strcmp(bu_prompt_zone(&tg), "stage"),
          "the pick comes from the STAGE (the card to place is already chosen)");
}

/* B8 :: watanabe_debut_places_under_chosen_host_and_grants_blade */
static void B_debut_places_under_chosen_host_and_grants_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return;
    test_add_to_hand(&tg, watanabe);
    test_add_to_discard(&tg, under);
    set_stage(&tg, 0, host, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 15);

    CHECK_EQ(test_play_to_stage(&tg, watanabe, AREA_CENTER), 1, "play_to_stage ok");
    CHECK(bu_prompt(&tg), "PROMPT CONTROL: ab#0 prompts for the host");
    CHECK_EQ(bu_prompt_count(&tg), 1, "exactly 1 member is requested");
    bu_pick_index(&tg, 0, 10);      /* choose the LEFT slot (index 0) */

    CHECK_EQ(under_n(&tg, 0, AREA_LEFT), 1, "the card is under the chosen host");
    CHECK(under_has(&tg, 0, AREA_LEFT, under), "and it is the waitroom card");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0,
             "and NOT under 渡辺 曜 (the center slot)");
    CHECK(!waitroom_has(&tg, under), "the card left the waitroom (moved, not copied)");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, host), 1,
             "『Aqours』 host with a member card under it (via ab#0) -> 1 blade");
}

/* B9 :: watanabe_debut_places_under_self_grants_blade */
static void B_debut_places_under_self_grants_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int watanabe = card(&tg, WATANABE);
    int host = card(&tg, HOST_AQOURS);
    int under = card(&tg, MEMBER_UNDER);
    if (watanabe < 0 || host < 0 || under < 0) return;
    test_add_to_hand(&tg, watanabe);
    test_add_to_discard(&tg, under);
    set_stage(&tg, 0, host, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_give_energy(&tg, 15);

    CHECK_EQ(test_play_to_stage(&tg, watanabe, AREA_CENTER), 1, "play_to_stage ok");
    CHECK(bu_prompt(&tg), "PROMPT CONTROL: ab#0 prompts for the host");
    bu_pick_index(&tg, 1, 10);      /* choose the CENTER slot (index 1) = 渡辺 曜 */

    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "the card is under 渡辺 曜 (center)");
    CHECK(under_has(&tg, 0, AREA_CENTER, under), "and it is the waitroom card");
    CHECK_EQ(under_n(&tg, 0, AREA_LEFT), 0, "and not under the Aqours host");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, watanabe), 1,
             "『Aqours』 member with a member card under it -> 1 blade");
}

/* ========================================================================
 * SECTION C
 * member_under_aqours_constant_blade_test.rs
 * The minimal shape of B's constant, without the 登場 half.
 * ===================================================================== */

/* C1 :: aqours_member_with_member_underneath_gains_one_blade */
static void C_aqours_member_with_member_under_gains_one_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    int member = card(&tg, WATANABE);
    int under_card = card(&tg, UNDER_AQOURS1);
    if (member < 0 || under_card < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, member, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, under_card);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: an 『Aqours』 member under her");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, member), 1,
             "under-card 『Aqours』 member should gain exactly +1 blade");
}

/* ========================================================================
 * SECTION D
 * bp7_under_member_per_unit_blade_test.rs
 * The `location: "under_member"` scope. D1-D3 drive the 常時 per_unit path
 * (PL!SP-bp7-003-R＋); D4-D7 drive the ライブ開始時 per_unit + distinct path
 * (PL!N-bp7-003-R＋) through a real turn walk.
 * ===================================================================== */

/* Run しずく's ライブ開始時 over the given under-cards and return the blade she
 * ended with. This is q278_q279_joint_card_under_member_blade_test's
 * `run_and_get_blade` and bp7_under_member_per_unit_blade_test's body, both
 * of which drive 5 passes -> LiveCardSet, set live, 2 passes -> LiveStart. */
static int d_run_shizuku_live_start(TestGame *tg, const char *const *under_nos,
                                    int n_under)
{
    int shizuku = card(tg, SHIZUKU);
    if (shizuku < 0) return -1;
    set_stage(tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    for (int i = 0; i < n_under; i++) {
        int u = mid(tg, under_nos[i]);
        if (!ident(u, under_nos[i])) return -1;
        test_place_under(tg, 0, AREA_CENTER, u);
    }
    CHECK_EQ(under_n(tg, 0, AREA_CENTER), n_under, "setup: the under-card stack");
    int filler = card(tg, FILLER_NO);
    int live = card(tg, LIVE_NO);
    if (filler < 0 || live < 0) return -1;
    test_give_energy(tg, 3);
    seed_deck(tg, filler);
    test_add_to_hand(tg, live);

    advance_to_live_set(tg);
    test_set_live_card(tg, 0, live);
    advance_to_live_start(tg);
    bu_drain_skip(tg, 12);
    return blade_of(tg, shizuku);
}

/* D1 :: chika_constant_blade_per_member_under */
static void D_chika_constant_blade_per_member_under(void)
{
    static TestGame tg;
    new_board(&tg);
    int chika = card(&tg, CHIKA);
    if (chika < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, chika, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, card(&tg, "PL!SP-sd1-001-SD"));
    test_place_under(&tg, 0, AREA_CENTER, card(&tg, "PL!SP-sd1-001-SD"));
    test_place_under(&tg, 0, AREA_CENTER, card(&tg, "PL!SP-sd1-004-SD"));
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 3, "setup: 3 member cards under Center");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, chika), 3,
             "3 member cards under -> 3 blade (per_unit location=under_member)");
}

/* D2 :: chika_constant_blade_zero_under */
static void D_chika_constant_blade_zero_under(void)
{
    /* Anti-vacuity control: the 3-under arm must grant 3. */
    static TestGame ctl;
    new_board(&ctl);
    int c_chika = card(&ctl, CHIKA);
    if (c_chika < 0) return;
    set_stage(&ctl, 0, RB_EMPTY_SLOT, c_chika, RB_EMPTY_SLOT);
    for (int i = 0; i < 3; i++) test_place_under(&ctl, 0, AREA_CENTER, card(&ctl, "PL!SP-sd1-004-SD"));
    test_recalc(&ctl);
    CHECK_EQ(blade_of(&ctl, c_chika), 3,
             "ANTI-VACUITY CONTROL: 3 member cards under 嵐 千砂都 must give 3 "
             "blade -- otherwise the 0 below is a dead per_unit, not a zero count");

    static TestGame tg;
    new_board(&tg);
    int chika = card(&tg, CHIKA);
    if (chika < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, chika, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, chika), 0, "0 member cards under -> 0 blade");
}

/* D3 :: chika_constant_blade_counts_only_under */
static void D_chika_constant_blade_counts_only_under(void)
{
    static TestGame tg;
    new_board(&tg);
    int chika = card(&tg, CHIKA);
    if (chika < 0) return;
    int kanon = card(&tg, "PL!SP-sd1-001-SD");
    int sumire = card(&tg, "PL!SP-sd1-004-SD");
    int filler = card(&tg, FILLER_NO);
    if (kanon < 0 || sumire < 0 || filler < 0) return;
    set_stage(&tg, 0, kanon, chika, sumire);
    test_add_to_hand(&tg, kanon);
    test_add_to_discard(&tg, sumire);
    test_add_to_deck_pl(&tg, 0, filler);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: exactly 1 member under Center");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, chika), 1,
             "1 member under + 4 elsewhere -> 1 blade (elsewhere NOT counted)");
}

/* D4 :: shizuku_live_start_blade_per_distinct_under */
static void D_shizuku_live_start_blade_per_distinct_under(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { "PL!SP-sd1-001-SD", "PL!SP-sd1-004-SD", "PL!SP-sd1-003-SD" };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 3, "3 distinct-named member cards under -> 3 blade");
}

/* D5 :: shizuku_live_start_blade_dedups_same_name */
static void D_shizuku_live_start_blade_dedups_same_name(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { "PL!SP-sd1-001-SD", "PL!SP-sd1-001-SD", "PL!SP-sd1-004-SD" };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 2,
             "2 same-name + 1 different-name member under -> 2 blade (distinct names)");
}

/* D6 :: shizuku_live_start_blade_zero_under */
static void D_shizuku_live_start_blade_zero_under(void)
{
    static TestGame ctl;
    new_board(&ctl);
    const char *cu[3] = { "PL!SP-sd1-001-SD", "PL!SP-sd1-004-SD", "PL!SP-sd1-003-SD" };
    int control = d_run_shizuku_live_start(&ctl, cu, 3);
    CHECK_EQ(control, 3,
             "ANTI-VACUITY CONTROL: the 3-distinct-under arm must grant 3 blade -- "
             "otherwise the 0 below cannot distinguish 'no under-cards' from "
             "'the ライブ開始時 never fired'");

    static TestGame tg;
    new_board(&tg);
    int blade = d_run_shizuku_live_start(&tg, NULL, 0);
    CHECK_EQ(blade, 0, "0 member cards under -> 0 blade");
}

/* D7 :: shizuku_live_start_blade_counts_only_under */
static void D_shizuku_live_start_blade_counts_only_under(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    if (shizuku < 0) return;
    int kanon = card(&tg, "PL!SP-sd1-001-SD");
    int sumire = card(&tg, "PL!SP-sd1-004-SD");
    int filler = card(&tg, FILLER_NO);
    int live = card(&tg, LIVE_NO);
    if (kanon < 0 || sumire < 0 || filler < 0 || live < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, kanon);
    test_add_to_hand(&tg, sumire);
    test_add_to_discard(&tg, kanon);
    test_place_under(&tg, 0, AREA_CENTER, kanon);
    test_give_energy(&tg, 3);
    seed_deck(&tg, filler);
    test_add_to_hand(&tg, live);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg);
    bu_drain_skip(&tg, 12);

    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 1, "setup: 1 member under Center");
    CHECK_EQ(blade_of(&tg, shizuku), 1,
             "1 member under + 3 elsewhere -> 1 blade (elsewhere NOT counted)");
}

/* EXTRA (not a Rust test) :: two しずく instances must count only their OWN
 * under-cards. This is the two-instance scoping probe for
 * `per_unit location:under_member`: the C arm in rb_effect_count flattens
 * EVERY stage area's under_cards for the actor, so it has no way to tell the
 * two hosts apart. Reported as a distinct finding; it is not part of the 44. */
static void D_extra_two_shizuku_instances_are_scoped_to_their_own_under_cards(void)
{
    static TestGame tg;
    new_board(&tg);
    int left = card(&tg, SHIZUKU);
    int center = card(&tg, SHIZUKU);
    if (left < 0 || center < 0) return;
    CHECK(left != center, "setup: two DISTINCT instances of 桜坂しずく");
    set_stage(&tg, 0, left, center, RB_EMPTY_SLOT);
    for (int i = 0; i < 3; i++) test_place_under(&tg, 0, AREA_LEFT,  card(&tg, "PL!SP-sd1-001-SD"));
    for (int i = 0; i < 3; i++) test_place_under(&tg, 0, AREA_CENTER, card(&tg, "PL!SP-sd1-004-SD"));
    CHECK_EQ(under_n(&tg, 0, AREA_LEFT), 3, "setup: 3 under LeftSide");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 3, "setup: 3 under Center");

    int filler = card(&tg, FILLER_NO);
    int live = card(&tg, LIVE_NO);
    if (filler < 0 || live < 0) return;
    test_give_energy(&tg, 3);
    seed_deck(&tg, filler);
    test_add_to_hand(&tg, live);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg);
    bu_drain_skip(&tg, 16);

    CHECK_EQ(blade_of(&tg, left), 3,
             "the LeftSide しずく counts only the 3 cards under HER (not 6)");
    CHECK_EQ(blade_of(&tg, center), 3,
             "the Center しずく counts only the 3 cards under HER (not 6)");
    CHECK_EQ(under_n(&tg, 0, AREA_LEFT), 3, "LeftSide under-cards untouched");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 3, "Center under-cards untouched");
}

/* ========================================================================
 * SECTION E
 * per_under_member_energy_constant_blades_test.rs
 * PL!N-pb1-011-R ミア・テイラー 常時: blade per ENERGY card under her. The
 * same per_unit constant path as D1-D3, but the per-unit FILTER is
 * card_type:"energy_card", so an ordinary member under her must not count.
 * ===================================================================== */

/* E1 :: constant_blades_equal_under_member_energy_count */
static void E_constant_blades_equal_under_energy_count(void)
{
    static TestGame tg;
    new_board(&tg);
    int mia = card(&tg, MIA);
    int energy = card(&tg, ENERGY_NO);
    if (mia < 0 || energy < 0) return;
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT, "setup: stage starts empty");
    set_stage(&tg, 0, RB_EMPTY_SLOT, mia, RB_EMPTY_SLOT);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], mia, "ミア・テイラー at Center");
    test_place_under(&tg, 0, AREA_CENTER, energy);
    test_place_under(&tg, 0, AREA_CENTER, energy);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2, "setup: 2 energy under Center");

    /* The Rust twin guards "the card has at least one decodable ability" so a
     * fixture that never resolved cannot make the blade assertion vacuous. */
    int nab = rb_card_num_abilities((uint32_t)mia);
    CHECK(nab > 0,
          "ANTI-VACUITY CONTROL: ミア・テイラー must decode at least one ability "
          "for the per-under-energy bonus to be reachable at all");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2, "the 2 under-cards survive setup");

    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, mia), 2, "2 energy under -> 2 blade");
}

/* E2 :: constant_blades_zero_without_under_member_energy */
static void E_constant_blades_zero_without_under_energy(void)
{
    static TestGame ctl;
    new_board(&ctl);
    int c_mia = card(&ctl, MIA);
    int c_e = card(&ctl, ENERGY_NO);
    if (c_mia < 0 || c_e < 0) return;
    set_stage(&ctl, 0, RB_EMPTY_SLOT, c_mia, RB_EMPTY_SLOT);
    test_place_under(&ctl, 0, AREA_CENTER, c_e);
    test_place_under(&ctl, 0, AREA_CENTER, c_e);
    test_recalc(&ctl);
    CHECK_EQ(blade_of(&ctl, c_mia), 2,
             "ANTI-VACUITY CONTROL: 2 energy under ミア・テイラー must give 2 "
             "blade -- otherwise the 0 below is free");

    static TestGame tg;
    new_board(&tg);
    int mia = card(&tg, MIA);
    if (mia < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, mia, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, mia), 0, "0 energy under -> 0 blade");
}

/* EXTRA (not a Rust test) :: a MEMBER under her must not count toward the
 * energy per-unit count. This is the card_type:"energy_card" filter half of
 * the same constant, which no Rust test in this cluster covers. */
static void E_extra_member_under_does_not_count_as_energy(void)
{
    static TestGame tg;
    new_board(&tg);
    int mia = card(&tg, MIA);
    if (mia < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, mia, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, card(&tg, MEMBER_UNDER));
    test_place_under(&tg, 0, AREA_CENTER, card(&tg, ENERGY_NO));
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2, "setup: 1 member + 1 energy under");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, mia), 1,
             "only the ENERGY card counts: 1 member + 1 energy under -> 1 blade, "
             "not 2");
}

/* ========================================================================
 * SECTION F
 * q278_q279_joint_card_under_member_blade_test.rs
 * How a JOINT (多種統合 "A&B&C") member counts toward 「名前の異なるメンバーカード
 * 1枚につき」: as ONE unit, and only when it brings a name not already present.
 * ===================================================================== */

/* F1 :: q278_ayumu_plus_joint_is_two_blades */
static void F_q278_ayumu_plus_joint_is_two_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[2] = { AYUMU, JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 2);
    CHECK_EQ(blade, 2, "Q278: ayumu + joint{ayumu,kanon,hanaba} -> 2 blades");
}

/* F2 :: q279_all_singles_plus_joint_is_three_blades */
static void F_q279_all_singles_plus_joint_is_three_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[4] = { AYUMU, KANON_BP1, HANABO, JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 4);
    CHECK_EQ(blade, 3,
             "Q279: 3 singles + joint (constituents already present) -> 3 blades, "
             "the joint adds NO 4th slot");
}

/* F3 :: q279_three_singles_no_joint_is_three_blades */
static void F_q279_three_singles_no_joint_is_three_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { AYUMU, KANON_BP1, HANABO };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 3, "3 distinct single-name cards -> 3 blades");
}

/* F4 :: qpartial_singles_plus_joint_counts_new_mitigating_name */
static void F_qpartial_singles_plus_joint_is_three_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { AYUMU, KANON_BP1, JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 3,
             "歩下+かのん present, joint introduces 花帆 -> 3 blades");
}

/* F5 :: qjoint_alone_is_one_blade */
static void F_qjoint_alone_is_one_blade(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[1] = { JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 1);
    CHECK_EQ(blade, 1, "a lone joint card -> 1 blade");
}

/* F6 :: qduplicates_dedupe */
static void F_qduplicates_dedupe(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { AYUMU, AYUMU, KANON_BP1 };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 2, "2歩下 + かのか -> 2 blades");
}

/* F7 :: qzero_under_is_zero_blades */
static void F_qzero_under_is_zero_blades(void)
{
    static TestGame ctl;
    new_board(&ctl);
    const char *cu[3] = { AYUMU, KANON_BP1, HANABO };
    int control = d_run_shizuku_live_start(&ctl, cu, 3);
    CHECK_EQ(control, 3,
             "ANTI-VACUITY CONTROL: 3 singles under 桜坂しずく must give 3 blades -- "
             "otherwise the 0 below could be a ライブ開始時 that never fired");

    static TestGame tg;
    new_board(&tg);
    int blade = d_run_shizuku_live_start(&tg, NULL, 0);
    CHECK_EQ(blade, 0, "no under-cards -> 0 blades");
}

/* F8 :: q279_duplicate_joint_cards_count_as_one_name */
static void F_q279_duplicate_joint_cards_count_as_one_name(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[2] = { JOINT, JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 2);
    CHECK_EQ(blade, 1, "two copies of the same joint -> 1 blade");
}

/* F9 :: q279_single_plus_duplicate_joint_cards_count_each_name_once */
static void F_q279_single_plus_duplicate_joint_is_two_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    const char *u[3] = { AYUMU, JOINT, JOINT };
    int blade = d_run_shizuku_live_start(&tg, u, 3);
    CHECK_EQ(blade, 2, "one single plus duplicate joint names -> 2 blades");
}

/* F10 :: q278_ability_placed_card_and_joint_are_counted_at_live_start
 * The end-to-end: the 起動 parks 歩夢 under her (and sets the heart copy),
 * a joint is added by hand, and at ライブ開始時 BOTH count. */
static void F_q278_ability_placed_and_joint_counted_at_live_start(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    int ayumu = card(&tg, AYUMU);
    int filler = card(&tg, FILLER_NO);
    if (shizuku < 0 || ayumu < 0 || filler < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, ayumu);
    seed_deck(&tg, filler);

    CHECK(printed_cost(ayumu) <= 17,
          "fixture guard: the waitroom target must cost <= 17 (cards.json: "
          "PL!N-bp1-001-R cost 9)");
    test_activate_ability(&tg, shizuku);
    bu_drain_pick0(&tg, 12);

    /* PROMPT CONTROL: the 起動 must really have run; an unrun ability would
     * leave an empty under-stack and the "0" blade reading below would be
     * vacuously true. */
    CHECK(under_has(&tg, 0, AREA_CENTER, ayumu),
          "PROMPT CONTROL: the 起動 must park 上原歩夢 from the waitroom under "
          "しずく -- otherwise the blade assertion below is vacuous");

    int joint = card(&tg, JOINT);
    int live = card(&tg, LIVE_NO);
    if (joint < 0 || live < 0) return;
    test_place_under(&tg, 0, AREA_CENTER, joint);
    test_give_energy(&tg, 3);
    test_add_to_hand(&tg, live);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg);
    bu_drain_skip(&tg, 12);

    CHECK_EQ(heart_copy_of(&tg, shizuku), ayumu,
             "heart_copy maps しずく -> the member the 起動 placed under her");
    CHECK_EQ(blade_of(&tg, shizuku), 2,
             "the ability-placed member and the joint must both count at ライブ開始時");
}

/* ========================================================================
 * SECTION G
 * shizuku_under_member_heart_copy_test.rs
 * The 起動 half: move a cost<=17 『虹ヶ咲』 member from the waitroom under
 * herself, and until live end her ORIGINAL hearts become that card's hearts.
 * ===================================================================== */

/* shizuku_under_member_heart_copy_test::activate_and_resolve */
static int g_activate_and_resolve(TestGame *tg, int shizuku)
{
    test_activate_ability(tg, shizuku);
    /* The move_cards step creates a card-selection choice for the discard
     * card; some engines auto-select a single eligible card. Either path. */
    bu_drain_pick0(tg, 8);
    const RbBag *u = &tg->state.p[0].under_cards[AREA_CENTER];
    return u->n > 0 ? u->cards[u->n - 1] : RB_EMPTY_SLOT;
}

/* G1 :: shizuku_heart_copy_matches_placed_card */
static void G_heart_copy_matches_placed_card(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    int ayumu = card(&tg, AYUMU_SD);
    int filler = card(&tg, FILLER_NO);
    if (shizuku < 0 || ayumu < 0 || filler < 0) return;
    CHECK_EQ(printed_cost(ayumu), 13,
             "fixture guard: 上原歩夢 PL!N-sd1-001-SD costs 13 (cards.json: 13), "
             "which is <= 17 so the 起動's filter admits her");
    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, ayumu);
    seed_deck(&tg, filler);

    int placed = g_activate_and_resolve(&tg, shizuku);
    CHECK_EQ(placed, ayumu, "上原歩夢 is placed under しずく (Center)");
    CHECK(under_has(&tg, 0, AREA_CENTER, ayumu), "and it is under Center");
    CHECK(!waitroom_has(&tg, ayumu), "上原歩夢 leaves the waitroom (moved)");

    CHECK_EQ(heart_copy_of(&tg, shizuku), ayumu,
             "heart_copy should map しずく -> the placed card");

    int hearts[8];
    rb_calc_stage_hearts(&tg.state, 0, hearts);
    /* cards.json: PL!N-sd1-001-SD base_heart = {heart01:3, heart02:1, heart04:1} */
    CHECK_EQ(hearts[H01], 3, "しずく's heart01 copies 歩夢's 3");
    CHECK_EQ(hearts[H02], 1, "しずく's heart02 copies 歩夢's 1");
    CHECK_EQ(hearts[H04], 1, "しずく's heart04 copies 歩夢's 1");
}

/* G2 :: shizuku_heart_copy_uses_placed_cards_hearts */
static void G_heart_copy_uses_placed_cards_hearts(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    int kasumi = card(&tg, KASUMI_SD);
    int filler = card(&tg, FILLER_NO);
    if (shizuku < 0 || kasumi < 0 || filler < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, kasumi);
    seed_deck(&tg, filler);

    int placed = g_activate_and_resolve(&tg, shizuku);
    CHECK_EQ(placed, kasumi, "中須かすみ is placed under しずく");
    CHECK_EQ(heart_copy_of(&tg, shizuku), kasumi,
             "heart_copy should map しずく -> かすみ");

    int hearts[8];
    rb_calc_stage_hearts(&tg.state, 0, hearts);
    /* cards.json: PL!N-sd1-002-SD base_heart = {heart03:2, heart04:1} */
    CHECK_EQ(hearts[H03], 2, "しずく's heart03 copies かすみ's 2");
    CHECK_EQ(hearts[H04], 1, "しずく's heart04 copies かすみ's 1");
    CHECK_EQ(hearts[H01], 0,
             "no heart01 leaks from しずく's OWN hearts (かすみ has none)");
}

/* G3 :: shizuku_heart_copy_no_discard_card */
static void G_heart_copy_no_discard_card(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    int filler = card(&tg, FILLER_NO);
    if (shizuku < 0 || filler < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, card(&tg, HONOKA_SD));   /* μ's, not 虹ヶ咲 */
    seed_deck(&tg, filler);

    test_activate_ability(&tg, shizuku);
    /* Anti-vacuity, same reasoning as G4: an unanswered prompt would leave the
     * effect unrun and BOTH absence assertions below would hold without the
     * group filter ever being consulted. */
    CHECK(!bu_prompt(&tg),
          "ANTI-VACUITY CONTROL: with no eligible discard card the ability must "
          "decline outright rather than park a selection prompt nobody answers");

    CHECK_EQ(heart_copy_of(&tg, shizuku), -1,
             "no heart_copy without an eligible discard card");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0,
             "no card should be placed under しずく");
}

/* G4 :: shizuku_heart_copy_rejects_non_nijigasaki_group */
static void G_heart_copy_rejects_non_nijigasaki_group(void)
{
    static TestGame tg;
    new_board(&tg);
    int shizuku = card(&tg, SHIZUKU);
    int honoka = card(&tg, HONOKA_SD);          /* 高坂 穂乃果, μ's */
    int filler = card(&tg, FILLER_NO);
    if (shizuku < 0 || honoka < 0 || filler < 0) return;
    CHECK(printed_cost(honoka) <= 17,
          "fixture guard: the decoy must be within the cost half of the filter "
          "(cards.json: PL!-sd1-010-SD cost 4) so ONLY the group half can reject it");
    set_stage(&tg, 0, RB_EMPTY_SLOT, shizuku, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, honoka);
    seed_deck(&tg, filler);

    test_activate_ability(&tg, shizuku);
    CHECK(!bu_prompt(&tg),
          "ANTI-VACUITY CONTROL: the ability must decline outright on a non-虹ヶ咲 "
          "discard card, not park an unanswered selection prompt -- otherwise the "
          "absence assertions below pass without the filter ever being evaluated");
    CHECK_EQ(heart_copy_of(&tg, shizuku), -1, "non-虹ヶ咲 discard card must be rejected");
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 0, "no card placed under しずく");
}

/* ========================================================================
 * SECTION H
 * live_start_distinct_under_member_blades_test.rs
 * ===================================================================== */

/* H1 :: live_start_two_distinct_under_members_grant_at_least_two_blades */
static void H_two_distinct_under_members_grant_at_least_two_blades(void)
{
    static TestGame tg;
    new_board(&tg);
    int member = card(&tg, SHIZUKU);
    int under_a = card(&tg, UNDER_AQOURS1);
    int under_b = card(&tg, UNDER_AQOURS2);
    int filler = card(&tg, FILLER_NO);
    if (member < 0 || under_a < 0 || under_b < 0 || filler < 0) return;
    set_stage(&tg, 0, RB_EMPTY_SLOT, member, RB_EMPTY_SLOT);
    test_place_under(&tg, 0, AREA_CENTER, under_a);
    test_place_under(&tg, 0, AREA_CENTER, under_b);
    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2, "setup: 2 distinct members under");
    fill_both_main_decks(&tg, filler);
    test_give_energy(&tg, 15);

    /* Rust advance_live_skipping_optional_cards: 7 passes, declining the
     * skippable card prompts between them. */
    for (int i = 0; i < 7; i++) { test_pass(&tg); bu_drain_skip(&tg, 6); }

    CHECK_EQ(under_n(&tg, 0, AREA_CENTER), 2,
             "the under-cards are still there after the walk");
    CHECK(blade_of(&tg, member) >= 2,
          "2 distinct under-members -> >= +2 blade (got %d)",
          blade_of(&tg, member));
}

/* ========================================================================
 * harness
 * ===================================================================== */

static void on_segv(int sig)
{
    fprintf(stderr,
            "\n*** SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(3);
}

#define CHILD_OK        0
#define CHILD_FAILURES  1

/* Each test builds its own board; run() only brackets the pass/fail delta.
 * The test still runs in a forked child so an engine fault is attributed to
 * ONE test and the sections after it still execute. */
static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = failures - f0;
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions]\n", nf ? "FAILED" : "ok", name,
               assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s)\n", name,
               assertions, failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH",
               name, WTERMSIG(status));
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
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
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
    {
        const char *d = getenv("RB_BLADE_UNDER_DUMP");
        if (d && *d) {
            char buf[512];
            snprintf(buf, sizeof(buf), "%s", d);
            char *cursor = buf;
            while (cursor && *cursor) {
                char *comma = strchr(cursor, ',');
                if (comma) *comma = '\0';
                if (*cursor) bu_dump_card(cursor);
                cursor = comma ? comma + 1 : NULL;
            }
            rb_unload();
            return 0;
        }
    }

    printf("--- A under_liella_member_constant_host_blade_test.rs ---\n");
    run("under_liella_host_blade_under_liella_host_gains_blade",
        A_under_liella_host_gains_blade);
    run("under_liella_host_blade_on_stage_grants_no_blade",
        A_on_stage_grants_no_blade);
    run("under_liella_host_blade_under_non_liella_host_grants_no_blade",
        A_under_non_liella_host_grants_no_blade);
    run("under_liella_host_blade_absent_grants_no_blade",
        A_absent_grants_no_blade);
    run("two_under_member_sources_grant_two_blades_to_one_liella_host",
        A_two_sources_two_blades_to_one_host);
    run("under_liella_host_blade_under_center_does_not_leak_to_other_slots",
        A_under_center_does_not_leak);
    run("under_liella_host_blade_in_hand_contributes_nothing",
        A_in_hand_contributes_nothing);
    run("under_liella_host_blade_copies_under_two_liella_hosts",
        A_copies_under_two_hosts);
    run("under_liella_host_blade_under_opponent_liella_host_gains_blade",
        A_under_opponent_liella_host_gains_blade);
    run("under_liella_host_blade_leaving_removes_blade",
        A_leaving_removes_blade);

    printf("--- B bp7_watanabe_under_card_blade_test.rs ---\n");
    run("watanabe_aqours_host_with_member_under_gains_blade",
        B_aqours_host_with_member_under_gains_blade);
    run("watanabe_aqours_host_without_under_gains_nothing",
        B_aqours_host_without_under_gains_nothing);
    run("watanabe_aqours_host_with_energy_under_gains_nothing",
        B_aqours_host_with_energy_under_gains_nothing);
    run("watanabe_non_aqours_host_with_member_under_gains_nothing",
        B_non_aqours_host_with_member_under_gains_nothing);
    run("watanabe_self_no_under_gains_nothing", B_self_no_under_gains_nothing);
    run("watanabe_two_hosts_with_members_under_both_gain", B_two_hosts_both_gain);
    run("watanabe_debut_offers_which_member_to_place_under",
        B_debut_offers_which_member_to_place_under);
    run("watanabe_debut_places_under_chosen_host_and_grants_blade",
        B_debut_places_under_chosen_host_and_grants_blade);
    run("watanabe_debut_places_under_self_grants_blade",
        B_debut_places_under_self_grants_blade);

    printf("--- C member_under_aqours_constant_blade_test.rs ---\n");
    run("aqours_member_with_member_underneath_gains_one_blade",
        C_aqours_member_with_member_under_gains_one_blade);

    printf("--- D bp7_under_member_per_unit_blade_test.rs ---\n");
    run("chika_constant_blade_per_member_under",
        D_chika_constant_blade_per_member_under);
    run("chika_constant_blade_zero_under", D_chika_constant_blade_zero_under);
    run("chika_constant_blade_counts_only_under",
        D_chika_constant_blade_counts_only_under);
    run("shizuku_live_start_blade_per_distinct_under",
        D_shizuku_live_start_blade_per_distinct_under);
    run("shizuku_live_start_blade_dedups_same_name",
        D_shizuku_live_start_blade_dedups_same_name);
    run("shizuku_live_start_blade_zero_under",
        D_shizuku_live_start_blade_zero_under);
    run("shizuku_live_start_blade_counts_only_under",
        D_shizuku_live_start_blade_counts_only_under);
    run("EXTRA_two_shizuku_instances_scoped_to_their_own_under_cards",
        D_extra_two_shizuku_instances_are_scoped_to_their_own_under_cards);

    printf("--- E per_under_member_energy_constant_blades_test.rs ---\n");
    run("constant_blades_equal_under_member_energy_count",
        E_constant_blades_equal_under_energy_count);
    run("constant_blades_zero_without_under_member_energy",
        E_constant_blades_zero_without_under_energy);
    run("EXTRA_member_under_does_not_count_as_energy",
        E_extra_member_under_does_not_count_as_energy);

    printf("--- F q278_q279_joint_card_under_member_blade_test.rs ---\n");
    run("q278_ayumu_plus_joint_is_two_blades", F_q278_ayumu_plus_joint_is_two_blades);
    run("q279_all_singles_plus_joint_is_three_blades",
        F_q279_all_singles_plus_joint_is_three_blades);
    run("q279_three_singles_no_joint_is_three_blades",
        F_q279_three_singles_no_joint_is_three_blades);
    run("qpartial_singles_plus_joint_counts_new_mitigating_name",
        F_qpartial_singles_plus_joint_is_three_blades);
    run("qjoint_alone_is_one_blade", F_qjoint_alone_is_one_blade);
    run("qduplicates_dedupe", F_qduplicates_dedupe);
    run("qzero_under_is_zero_blades", F_qzero_under_is_zero_blades);
    run("q279_duplicate_joint_cards_count_as_one_name",
        F_q279_duplicate_joint_cards_count_as_one_name);
    run("q279_single_plus_duplicate_joint_cards_count_each_name_once",
        F_q279_single_plus_duplicate_joint_is_two_blades);
    run("q278_ability_placed_card_and_joint_are_counted_at_live_start",
        F_q278_ability_placed_and_joint_counted_at_live_start);

    printf("--- G shizuku_under_member_heart_copy_test.rs ---\n");
    run("shizuku_heart_copy_matches_placed_card",
        G_heart_copy_matches_placed_card);
    run("shizuku_heart_copy_uses_placed_cards_hearts",
        G_heart_copy_uses_placed_cards_hearts);
    run("shizuku_heart_copy_no_discard_card", G_heart_copy_no_discard_card);
    run("shizuku_heart_copy_rejects_non_nijigasaki_group",
        G_heart_copy_rejects_non_nijigasaki_group);

    printf("--- H live_start_distinct_under_member_blades_test.rs ---\n");
    run("live_start_two_distinct_under_members_grant_at_least_two_blades",
        H_two_distinct_under_members_grant_at_least_two_blades);

    printf("\n==== parity_blade_under: %d tests ok, %d failed, %d crashed "
           "(%d setup bugs) ====\n",
           n_tests_ok, n_tests_failed, n_tests_crashed, setup_bugs);
    return failures ? 1 : 0;
}

/* Parity suite for engine/tests/test_modules/effects/gain/hearts/live_start/
 *
 * 16 Rust test files, all entirely unported. This file covers the whole
 * cluster. Every C test names the Rust file (and where relevant the Rust test
 * function) it mirrors.
 *
 * THEME. A ライブ開始時 (live-start) heart gain is the only heart source in the
 * game that is not 常時 (constant) -- it is a one-shot modifier attached to a
 * card during the live and reverted at ライブ終了時. That makes this cluster the
 * adversarial check on four separate mechanisms that a 常時-only cluster never
 * touches:
 *
 *   1. TIMING.   The grant must land at the live-start point of the phase
 *                 machine, not at 常時 recalc time. Tests that drive the real
 *                 turn walk (test_pass / test_set_live_card) are separated from
 *                 tests that fire the trigger directly through the
 *                 rb_trigger_auto_ability shim, because those two paths fail
 *                 for different reasons.
 *   2. GATING.   heart gained at live start is almost always conditional:
 *                 required-heart aggregates over the live-card zone, movement
 *                 events, group filters, a discard-pile AND, a paid optional
 *                 cost. A gate that never fires is indistinguishable from a
 *                 grant that never happened, so every test here asserts the
 *                 NEGATIVE case too.
 *   3. SCOPE.    「1人」 (one member) vs 「すべての」 (all members) vs the
 *                 ability holder itself, and 自分の / 相手の ownership.
 *   4. AGGREGATION.  A heart modifier on a card is a MODIFIER; the live score
 *                 only sees it through stage-heart aggregation. Where the Rust
 *                 test reads calculate_stage_hearts(), this file reads
 *                 rb_calc_stage_hearts() and checks the same colour counts.
 *
 * Heart colour indices follow cards/compile_cards.py HEART_COLORS and the C
 * RB_HEART_* enum:
 *   0 = heart00 (PINK wildcard)  1 = heart01 RED   2 = heart02 YELLOW
 *   3 = heart03 GREEN            4 = heart04 BLUE  5 = heart05 PURPLE
 *   6 = heart06 ORANGE           7 = ALL
 * Every read goes through rb_mods_get_heart directly: test_get_heart_modifier
 * remaps a requested colour of 5 onto RB_HEART_ORANGE, which would silently
 * turn a heart05 assertion into a heart06 one.
 *
 * Card identity: test_id() (mid() below) is used for every Rust `game.id(...)`
 * because Rust's id() allocates the NEXT DISTINCT pool slot -- two game.id("X")
 * calls are two instances. C's plain template lookup aliases, so test_id() is
 * only correct when the card is genuinely unique in its board; mid() is the
 * safe default and the one used everywhere except where a shared template is
 * deliberate. ident() asserts the resolved card_no before any ability claim,
 * because this cluster is adjacent to the bp2/pb2 transposition trap
 * (PL!SP-bp2-011-R is debut-only, PL!SP-pb2-011-R has auto + live-start).
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int assertions;

#define TRIG_LIVE_START "ライブ開始時"

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
    long long actual_value = (long long)(actual); \
    long long expected_value = (long long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %lld expected %lld)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── heart colour slots ───────────────────────────────────────────────────── */

#define H00 0   /* heart00 -- wildcard / pink */
#define H01 1   /* heart01 RED */
#define H02 2   /* heart02 YELLOW */
#define H03 3   /* heart03 GREEN */
#define H04 4   /* heart04 BLUE */
#define H05 5   /* heart05 PURPLE */
#define H06 6   /* heart06 ORANGE */
#define HALL 7  /* "All" -- the icon_all aggregate slot */

/* ── helpers ─────────────────────────────────────────────────────────────── */

/* `src` is the in-tree layout; `../cards/build` is where the Makefile (and
 * tools/isolated_build.sh) put the blobs, so an isolated out-of-tree build
 * finds the database only through this second path. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust helpers::TestGame::id() allocates the NEXT DISTINCT pool slot, so two
 * game.id("X") calls are two different card instances. C's plain template
 * lookup returns the SHARED TEMPLATE index, which aliases; test_new_id() is
 * the distinct copy. Every Rust game.id(...) maps to mid() below. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
        failures++;
    }
    return id;
}

/* The Rust SetupGuard: the staged id really is the print the test names.
 * Returns 1 when it is, so a caller can bail rather than assert about -1. */
static int ident(TestGame *tg, int cid, const char *no)
{
    if (!rb_card_no_eq(cid, no)) {
        assertions++;
        fprintf(stderr, "SETUP BUG: expected card %s, got id=%d (%s)\n",
                no, cid, cid >= 0 ? test_card_name(cid) : "<none>");
        failures++;
        return 0;
    }
    return 1;
}

static int heart_of(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_heart(&tg->state.mods, cid, slot);
}

static int heart01_of(TestGame *tg, int cid) { return heart_of(tg, cid, H01); }

/* heart01 + heart02 + heart06 -- the sum the area-moved-member test reads. */
static int heart_1_2_6(TestGame *tg, int cid)
{
    return heart_of(tg, cid, H01) + heart_of(tg, cid, H02) + heart_of(tg, cid, H06);
}

/* Rust fill_decks: push `n` filler cards onto BOTH main decks without
 * clearing. Several cluster tests then clear a deck by hand, so both the
 * clearing and the non-clearing form are needed. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void clear_decks(TestGame *tg)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
}

static void set_hand(TestGame *tg, const int *ids, int n)
{
    RbPlayer *P = &tg->state.p[0];
    P->hand.n = 0;
    for (int i = 0; i < n; i++) P->hand.cards[P->hand.n++] = ids[i];
}

static void set_stage3(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static int live_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].live.n; i++)
        if (tg->state.p[0].live.cards[i] == cid) return 1;
    return 0;
}

static int hand_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == cid) return 1;
    return 0;
}

static int discard_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == cid) return 1;
    return 0;
}

static int deck_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].deck.n; i++)
        if (tg->state.p[0].deck.cards[i] == cid) return 1;
    return 0;
}

/* ── live-start trigger firing ────────────────────────────────────────────── */

/* helpers/mod.rs:114 fire_trigger -- build the "<card_no>_<full_text>" ability
 * id for the first ability printing `trig`, queue exactly that ability, set
 * activating_card, process the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int hl_fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
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

static int hl_fire_live_start(TestGame *tg, int cid)
{
    return hl_fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START);
}

/* ── choice answering ───────────────────────────────────────────────────────
 * The C engine reads ONE resume index: -1 is Rust's select_indices(&[]) /
 * select_option(0) (DECLINE, and on a SelectTarget also the only "skip"),
 * >= 0 is ACCEPT and on a SelectTarget the sign is all that matters so
 * "accept" is 0. Getting that backwards silently PAWs every optional cost. */
#define HL_SKIP   (-1)
#define HL_ACCEPT (0)

/* while has_pending_choice() { select_indices(&[]) } */
static void hl_drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, HL_SKIP);
}

/* while has_pending_choice() { select_indices(&[0]) } -- unconditional first
 * index, i.e. "accept" for every prompt shape. */
static void hl_drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, 0);
}

/* Proceed with a queued auto ability, take index 0 for a mandatory card pick,
 * decline everything else. */
static void hl_drain_proceed(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = HL_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = HL_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip) idx = 0;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* ── phase walk ───────────────────────────────────────────────────────────── */

/* Rust advance_to_live_card_set / advance_to_live_card_set_p1: five blind
 * passes reach the live-card set window. */
static void hl_advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start / finish_live_setup: two more passes reach the
 * ライブ開始時 point where the ability queue is drained. */
static void hl_advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* ══════════════════════════════════════════════════════════════════════════
 * Diagnostic dump: HSLS_DUMP=<card_no>[,<card_no>...] prints every ability the
 * named card carries, so a red check can be classified as a PARSER gap (the
 * ability decodes wrong) versus an ENGINE gap (it decodes right and the
 * evaluation is wrong) without guessing. Same idiom as HC_DUMP in
 * test_parity_hearts_constants.c and RB_DUMP_ABILITY in
 * test_parity_draw_until_count.c. The suite is unchanged when it is unset.
 * ══════════════════════════════════════════════════════════════════════════ */

static void dump_val(const CondValue *v)
{
    if (!v) return;
    switch (v->tag) {
    case RB_TAG_STR: printf("\"%s\"", v->s ? v->s : ""); break;
    case RB_TAG_I64: printf("%lld", (long long)v->i); break;
    case RB_TAG_TRUE: printf("true"); break;
    case RB_TAG_FALSE: printf("false"); break;
    case RB_TAG_ARRAY:
        printf("arr[");
        for (uint32_t i = 0; i < v->arr_n; i++) { if (i) printf(","); dump_val(&v->arr[i]); }
        printf("]");
        break;
    case RB_TAG_OBJVAR: printf("objvar(%lld)", (long long)v->i); break;
    default: printf("tag=0x%02x", v->tag); break;
    }
}

static void dump_cond(const Condition *c, int depth)
{
    if (!c) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("cond variant=%d nfields=%u\n", c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        for (int k = 0; k < depth + 1; k++) fputs("  ", stdout);
        printf("%s: ", c->fields[i].key ? c->fields[i].key : "?");
        dump_val(&c->fields[i].v);
        printf("\n");
        if (c->fields[i].v.cond) dump_cond(c->fields[i].v.cond, depth + 2);
        /* A nested Condition can hang off an ARRAY ELEMENT, not just the
         * array value itself: an `and` / `or` node stores its operands that
         * way, and printing only v.cond makes the whole conjunction look
         * empty. */
        for (uint32_t j = 0; c->fields[i].v.arr && j < c->fields[i].v.arr_n; j++)
            if (c->fields[i].v.arr[j].cond)
                dump_cond(c->fields[i].v.arr[j].cond, depth + 2);
    }
}

static void dump_effect(const char *tag, const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("%s action=%s target=%s count=%d per_unit=%d per_unit_count=%d "
           "distinct=%d repeat_limit=%d nchild=%d has_cond=%d\n",
           tag, e->action ? e->action : "(null)", e->target ? e->target : "(null)",
           e->count, e->per_unit, e->per_unit_count, e->distinct_flag,
           e->repeat_limit, e->n_child, e->has_condition);
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            printf("      extra %s = %s\n", e->extra_k[i],
                   e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition) dump_cond(e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect("child", e->child[i], depth + 1);
    if (e->primary_effect) dump_effect("primary", e->primary_effect, depth + 1);
    if (e->alternative_effect) dump_effect("alt", e->alternative_effect, depth + 1);
    if (e->followup_action) dump_effect("followup", e->followup_action, depth + 1);
    if (e->conditional_action) dump_effect("cond", e->conditional_action, depth + 1);
}

static void dump_card(const char *no)
{
    int c = rb_find_card_by_no(no);
    if (c < 0) { printf("=== %s : NOT IN DB\n", no); return; }
    Card card;
    printf("=== %s (id=%d)\n", no, c);
    if (!rb_decode_card_by_index((uint32_t)c, &card)) { printf("  decode failed\n"); return; }
    printf("  resolved_card_no=%s name=%s cost=%d blade=%d base:",
           rb_card_string(card.card_no_idx), card.name ? card.name : "(null)",
           card.cost, (int)card.blade);
    for (int i = 0; i < card.num_base; i++)
        printf(" %d:%d", card.heart_color[i], card.heart_count[i]);
    printf("  need:");
    for (int i = 0; i < card.num_need; i++) {
        int k = card.num_base + i;
        if (k >= RB_MAX_HEARTS) break;
        printf(" %d:%d", card.heart_color[k], card.heart_count[k]);
    }
    printf("\n");
    int n = rb_card_num_abilities((uint32_t)c);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)c, i, &ab)) continue;
        printf("  ab#%d triggers=%s\n", i, ab.triggers ? ab.triggers : "(null)");
        dump_effect("cost", ab.cost, 2);
        dump_effect("effect", ab.effect, 2);
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
}

static int run_dump(const char *list)
{
    char buf[1024];
    snprintf(buf, sizeof buf, "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ","))
        dump_card(tok);
    return 0;
}

/* ── tests ────────────────────────────────────────────────────────────────── */

/* ══════════════════════════════════════════════════════════════════════════
 * A. TRIGGER-FIRED tests -- the ability is queued directly through
 *    hl_fire_live_start, exactly like the Rust `fire_trigger` /
 *    `trigger_ability` helpers these files define. Nothing here depends on the
 *    phase machine, so a red check in section A is a pure engine/parser
 *    verdict, never a mistimed setup.
 * ══════════════════════════════════════════════════════════════════════════ */

/* live_start_chosen_heart_area_moved_member_test.rs
 *   pl_sp_bp5_024_l_live_start_chosen_heart_granted_only_to_area_moved_member
 *   pl_sp_bp5_024_l_opponent_movement_does_not_qualify
 * PL!SP-bp5-024-L (MIRACLE NEW STORY): at live start pick one of
 * heart01/heart02/heart06; a member that MOVED AREAS this turn gains 1 of the
 * chosen heart until live end. The Rust test sums the three colours because
 * the chosen one is whichever option index 0 lands on. */
static void a_area_moved_member_pl_sp_bp5_024_l(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!SP-bp5-024-L");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, live, "PL!SP-bp5-024-L")) return;
    fill_decks(&tg, filler, 10);

    /* Rust: game.add_to_hand(live); game.set_live_card(live). */
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    CHECK(live_has(&tg, live), "MIRACLE NEW STORY occupies the live-card zone");

    int mover = mid(&tg, "PL!S-bp5-001-R＋");
    int stationary = mid(&tg, "PL!HS-bp5-001-R＋");
    if (!ident(&tg, mover, "PL!S-bp5-001-R＋")) return;
    set_stage3(&tg, 0, mover, RB_EMPTY_SLOT, stationary);
    /* Rust push_movement_event(mover, "stage", "stage", None, "p1", true) --
     * the C fold is rb_record_card_movement. Without the EVENT the
     * moved_this_turn condition reads "nothing moved" and the test passes or
     * fails for the wrong reason. */
    rb_record_card_movement(&tg.state, mover, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 1);

    CHECK(hl_fire_live_start(&tg, live),
          "MIRACLE NEW STORY carries a ライブ開始時 ability");

    /* Rust answer_all(&mut game, 0): SelectHeartColor takes option 0, every
     * other prompt takes index 0. */
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 12)
        rb_resume_with_choice(&tg.state, 0);

    CHECK_EQ(heart_1_2_6(&tg, mover), 1,
             "area-moved member gains the chosen heart until live end");
    CHECK_EQ(heart_1_2_6(&tg, stationary), 0,
             "member that did not move areas gains nothing");
}

/* Same card, negative ownership case: an OPPONENT's area move must not
 * qualify a member of yours. */
static void a_area_moved_opponent_does_not_qualify(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!SP-bp5-024-L");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, live, "PL!SP-bp5-024-L")) return;
    fill_decks(&tg, filler, 10);
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);

    int opp_mover = mid(&tg, "PL!S-bp5-001-R＋");
    if (!ident(&tg, opp_mover, "PL!S-bp5-001-R＋")) return;
    set_stage3(&tg, 1, opp_mover, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_record_card_movement(&tg.state, opp_mover, RB_ZONEID_STAGE, RB_ZONEID_STAGE, 1, 1);

    hl_fire_live_start(&tg, live);
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 12)
        rb_resume_with_choice(&tg.state, 0);

    CHECK_EQ(heart_1_2_6(&tg, opp_mover), 0,
             "opponent movement does not qualify the opponent's own member");
}

/* live_start_four_heart02_required_gain_heart02_test.rs
 *   pl_s_bp6_010_n_live_requires_four_heart02_grants_one_heart02
 *   pl_s_bp6_010_n_live_requires_no_heart02_grants_none
 * PL!S-bp6-010-N: aggregate heart02 across the live cards' REQUIREMENTS >= 4
 * -> gain one heart02. This is the purest "live-start heart gain fed by
 * stage/zone aggregation" case in the cluster. */
static void a_bp6_010_n_required_heart02(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int me = mid(&tg, "PL!S-bp6-010-N");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, me, "PL!S-bp6-010-N")) return;
    fill_decks(&tg, filler, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);

    int live = mid(&tg, "PL!SP-pb1-023-L");
    if (!ident(&tg, live, "PL!SP-pb1-023-L")) return;
    test_add_to_hand(&tg, live);
    test_set_live_card(&tg, 0, live);
    /* Rust also pins the phase to FirstAttackerPerformance; the C condition
     * is `temporal: during_live` on the ability, not on g->phase, so the
     * direct fire below must not depend on it -- assert that. */
    tg.state.phase = RB_PHASE_PERFORMANCE;

    hl_fire_live_start(&tg, me);
    hl_drain_skip(&tg);
    CHECK_EQ(heart_of(&tg, me, H02), 1,
             "required-heart02 total 4 >= 4 -> gain one heart02 until live end");

    /* Negative: a live card whose requirement totals 0 heart02. */
    static TestGame tg2;
    test_game_new(&tg2);
    int me2 = mid(&tg2, "PL!S-bp6-010-N");
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    if (!ident(&tg2, me2, "PL!S-bp6-010-N")) return;
    fill_decks(&tg2, filler2, 10);
    set_stage3(&tg2, 0, RB_EMPTY_SLOT, me2, RB_EMPTY_SLOT);
    int live2 = mid(&tg2, "PL!HS-bp2-020-L");
    if (!ident(&tg2, live2, "PL!HS-bp2-020-L")) return;
    test_add_to_hand(&tg2, live2);
    test_set_live_card(&tg2, 0, live2);
    tg2.state.phase = RB_PHASE_PERFORMANCE;

    hl_fire_live_start(&tg2, me2);
    hl_drain_skip(&tg2);
    CHECK_EQ(heart_of(&tg2, me2, H02), 0,
             "aggregate heart02 total 0 < 4 -> no gain");
}

/* live_start_live_zone_heart04_threshold_gain_test.rs
 *   dia_pl_s_bp5_013_n_heart04_total_exactly_four_grants_heart04
 *   dia_pl_s_bp5_013_n_heart04_total_three_no_grant
 *   dia_pl_s_bp5_013_n_empty_live_zone_no_grant
 * PL!S-bp5-013-N (黒澤ダイヤ): live-card heart04 requirement total >= 4. */
static void a_dia_bp5_013_n_heart04_threshold(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    int dia = mid(&tg, "PL!S-bp5-013-N");
    if (!ident(&tg, dia, "PL!S-bp5-013-N")) return;
    fill_decks(&tg, filler, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, dia, RB_EMPTY_SLOT);

    /* empty live zone first */
    hl_fire_live_start(&tg, dia);
    hl_drain_skip(&tg);
    CHECK_EQ(heart_of(&tg, dia, H04), 0, "empty live zone -> no heart04");

    /* aggregate == 4: two PL!S-bp2-020-L (heart04 x2 each) */
    static TestGame tg2;
    test_game_new(&tg2);
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    int dia2 = mid(&tg2, "PL!S-bp5-013-N");
    if (!ident(&tg2, dia2, "PL!S-bp5-013-N")) return;
    fill_decks(&tg2, filler2, 10);
    set_stage3(&tg2, 0, RB_EMPTY_SLOT, dia2, RB_EMPTY_SLOT);
    int l1 = mid(&tg2, "PL!S-bp2-020-L");
    int l2 = mid(&tg2, "PL!S-bp2-020-L");
    if (!ident(&tg2, l1, "PL!S-bp2-020-L")) return;
    test_add_to_live(&tg2, l1);
    test_add_to_live(&tg2, l2);
    hl_fire_live_start(&tg2, dia2);
    hl_drain_skip(&tg2);
    CHECK_EQ(heart_of(&tg2, dia2, H04), 1,
             "aggregate == 4 satisfies >=4 -> heart04 granted");

    /* aggregate 3: 2 + 1 */
    static TestGame tg3;
    test_game_new(&tg3);
    int filler3 = mid(&tg3, "PL!N-sd1-010-SD");
    int dia3 = mid(&tg3, "PL!S-bp5-013-N");
    if (!ident(&tg3, dia3, "PL!S-bp5-013-N")) return;
    fill_decks(&tg3, filler3, 10);
    set_stage3(&tg3, 0, RB_EMPTY_SLOT, dia3, RB_EMPTY_SLOT);
    int m1 = mid(&tg3, "PL!S-bp2-020-L");
    int m2 = mid(&tg3, "PL!S-bp3-020-L");
    if (!ident(&tg3, m1, "PL!S-bp2-020-L")) return;
    if (!ident(&tg3, m2, "PL!S-bp3-020-L")) return;
    test_add_to_live(&tg3, m1);
    test_add_to_live(&tg3, m2);
    hl_fire_live_start(&tg3, dia3);
    hl_drain_skip(&tg3);
    CHECK_EQ(heart_of(&tg3, dia3, H04), 0, "aggregate 3 < 4 -> no grant");
}

/* live_start_live_zone_heart05_threshold_gain_test.rs
 *   pl_s_bp5_017_n_live_start_heart05_total_four_grants_heart05
 *   pl_s_bp5_017_n_live_start_heart05_total_two_grants_no_heart05
 *   pl_s_bp5_017_n_live_start_empty_live_zone_grants_no_heart05
 * PL!S-bp5-017-N (小原鞠莉): the heart05 twin of Dia. Kept as a separate test
 * because heart05 is the slot test_get_heart_modifier silently remaps -- a
 * heart05 regression would be invisible through that shim. */
static void a_mari_bp5_017_n_heart05_threshold(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    int mari = mid(&tg, "PL!S-bp5-017-N");
    if (!ident(&tg, mari, "PL!S-bp5-017-N")) return;
    fill_decks(&tg, filler, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, mari, RB_EMPTY_SLOT);
    hl_fire_live_start(&tg, mari);
    hl_drain_skip(&tg);
    CHECK_EQ(heart_of(&tg, mari, H05), 0, "empty live zone -> no heart05");

    static TestGame tg2;
    test_game_new(&tg2);
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    int mari2 = mid(&tg2, "PL!S-bp5-017-N");
    if (!ident(&tg2, mari2, "PL!S-bp5-017-N")) return;
    fill_decks(&tg2, filler2, 10);
    set_stage3(&tg2, 0, RB_EMPTY_SLOT, mari2, RB_EMPTY_SLOT);
    int l = mid(&tg2, "PL!HS-PR-011-PR");
    if (!ident(&tg2, l, "PL!HS-PR-011-PR")) return;
    test_add_to_live(&tg2, l);
    hl_fire_live_start(&tg2, mari2);
    hl_drain_skip(&tg2);
    CHECK_EQ(heart_of(&tg2, mari2, H05), 1, "aggregate == 4 satisfies >=4");

    static TestGame tg3;
    test_game_new(&tg3);
    int filler3 = mid(&tg3, "PL!N-sd1-010-SD");
    int mari3 = mid(&tg3, "PL!S-bp5-017-N");
    if (!ident(&tg3, mari3, "PL!S-bp5-017-N")) return;
    fill_decks(&tg3, filler3, 10);
    set_stage3(&tg3, 0, RB_EMPTY_SLOT, mari3, RB_EMPTY_SLOT);
    int l3 = mid(&tg3, "PL!S-PR-023-PR");
    if (!ident(&tg3, l3, "PL!S-PR-023-PR")) return;
    test_add_to_live(&tg3, l3);
    hl_fire_live_start(&tg3, mari3);
    hl_drain_skip(&tg3);
    CHECK_EQ(heart_of(&tg3, mari3, H05), 0, "aggregate 2 < 4 -> no grant");
}

/* live_start_group_only_required_hearts_twelve_gain_two_all_hearts_test.rs
 *   ..._single_live_exact_modifier
 *   ..._total_twenty_four_exact_modifier
 *   ..._non_aq_present_all_members_fails
 *   ..._aggregate_below_threshold_no_hearts
 *   ..._mix_fills_3_slots_passes
 *   ..._empty_zone_no_hearts
 *   ..._three_slots_all_aqours_passes
 *   required_hearts_exact_twelve_gain_two_all_hearts
 *   required_hearts_nine_grants_no_all_hearts
 * PL!S-bp6-002-SEC (桜内梨子): EVERY card in your live-card zone must be
 * 『Aqours』 AND their heart02+heart04+heart05 requirements must total >= 12;
 * then gain 「all」 x2. The gain lands in the icon_all slot (HALL), not in any
 * single colour -- an easy thing to assert in the wrong place. */
static void a_riko_bp6_002_sec_all_hearts_twelve(void)
{
    struct { const char *lives[3]; int n; int expect; const char *msg; } cases[] = {
        { { "PL!S-bp3-019-L", 0, 0 }, 1, 2,
          "single MIRACLE WAVE (12) -> exactly +2 all-heart" },
        { { "PL!S-bp3-019-L", "PL!S-bp3-019-L", 0 }, 2, 2,
          "2x MIRACLE WAVE (24) -> +2 all-heart" },
        { { "PL!-bp5-019-L", "PL!S-sd1-019-SD", 0 }, 2, 0,
          "a non-Aqours live card in the zone -> all_members check fails" },
        { { "PL!S-sd1-019-SD", "PL!S-sd1-019-SD", "PL!S-sd1-019-SD" }, 3, 0,
          "3x SD live (3 each = 9 < 12) -> condition fails, no hearts" },
        { { "PL!S-bp3-019-L", "PL!S-sd1-019-SD", "PL!S-sd1-019-SD" }, 3, 2,
          "MIRACLE WAVE + 2x SD (18 >= 12) -> +2 all-heart" },
        { { 0, 0, 0 }, 0, 0, "empty live zone -> no hearts" },
        { { "PL!S-bp3-019-L", "PL!S-bp3-019-L", "PL!S-sd1-019-SD" }, 3, 2,
          "2x MIRACLE WAVE + SD (27 >= 12) -> +2 all-heart" },
        { { "PL!S-sd1-019-SD", "PL!S-sd1-019-SD", "PL!S-sd1-019-SD" }, 3, 0,
          "3x SD (9) stays below the 12 threshold" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        static TestGame tg;
        test_game_new(&tg);
        int riko = mid(&tg, "PL!S-bp6-002-SEC");
        int filler = mid(&tg, "PL!N-sd1-010-SD");
        if (!ident(&tg, riko, "PL!S-bp6-002-SEC")) return;
        fill_decks(&tg, filler, 10);
        set_stage3(&tg, 0, RB_EMPTY_SLOT, riko, RB_EMPTY_SLOT);
        int bad = 0;
        for (int k = 0; k < cases[i].n; k++) {
            int live = mid(&tg, cases[i].lives[k]);
            if (live < 0 || !ident(&tg, live, cases[i].lives[k])) { bad = 1; break; }
            test_add_to_live(&tg, live);
        }
        if (bad) return;
        hl_fire_live_start(&tg, riko);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, riko, HALL), cases[i].expect, cases[i].msg);
        /* The all-heart grant must NOT also land in the individual colour
         * slots: the Rust test pins heart02/04/05 at 0. */
        CHECK_EQ(heart_of(&tg, riko, H02), 0, "Riko: heart02 slot stays 0");
        CHECK_EQ(heart_of(&tg, riko, H04), 0, "Riko: heart04 slot stays 0");
        CHECK_EQ(heart_of(&tg, riko, H05), 0, "Riko: heart05 slot stays 0");
    }
}

/* required_hearts_exact_twelve_gain_two_all_hearts -- the exact-boundary case
 * the Rust file states separately: 4 x SD live card = exactly 12. */
static void a_riko_exact_twelve(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int riko = mid(&tg, "PL!S-bp6-002-SEC");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, riko, "PL!S-bp6-002-SEC")) return;
    fill_decks(&tg, filler, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, riko, RB_EMPTY_SLOT);
    int sd = mid(&tg, "PL!S-sd1-019-SD");
    if (!ident(&tg, sd, "PL!S-sd1-019-SD")) return;
    for (int i = 0; i < 4; i++) test_add_to_live(&tg, mid(&tg, "PL!S-sd1-019-SD"));
    hl_fire_live_start(&tg, riko);
    hl_drain_skip(&tg);
    CHECK_EQ(heart_of(&tg, riko, HALL), 2, "4x SD (exactly 12) -> +2 all-heart");
}

/* live_start_lone_group_member_hearts_become_heart04_test.rs
 *   pl_s_bp7_024_l_lone_aqours_member_hearts_become_heart04
 *   pl_s_bp7_024_l_no_aqours_member_no_transform
 * PL!S-bp7-024-L (ときめき分類学): ONE 『Aqours』 member on your stage has its
 * hearts become heart04. This is a heart-COLOUR REWRITE, not a heart gain, so
 * the assertion is on rb_mods_get_heart_color_multiplier. */
static void a_bp7_024_l_lone_aqours_hearts_become_heart04(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int live = mid(&tg, "PL!S-bp7-024-L");
    if (!ident(&tg, live, "PL!S-bp7-024-L")) return;
    test_add_to_live(&tg, live);
    int aqours = mid(&tg, "PL!S-bp5-007-R");
    int other = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, aqours, "PL!S-bp5-007-R")) return;
    set_stage3(&tg, 0, aqours, other, RB_EMPTY_SLOT);

    hl_fire_live_start(&tg, live);
    hl_drain_first(&tg);

    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg.state.mods, aqours), RB_HEART_BLUE,
             "the lone Aqours member's hearts become heart04");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg.state.mods, other), -1,
             "non-Aqours member untouched");

    static TestGame tg2;
    test_game_new(&tg2);
    int live2 = mid(&tg2, "PL!S-bp7-024-L");
    if (!ident(&tg2, live2, "PL!S-bp7-024-L")) return;
    test_add_to_live(&tg2, live2);
    int other2 = mid(&tg2, "PL!N-sd1-010-SD");
    set_stage3(&tg2, 0, other2, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    hl_fire_live_start(&tg2, live2);
    hl_drain_first(&tg2);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg2.state.mods, other2), -1,
             "no Aqours member -> nothing transformed");
}

/* live_start_paid_discard_other_member_heart01_test.rs
 *   pl_bp4_013_n_paid_discard_grants_other_member_heart01
 *   pl_bp4_013_n_declined_discard_grants_no_heart01
 * PL!-bp4-013-N: optional 「手札のカードを1枚控え室に置いてもよい」 -> ANOTHER
 * stage member gains heart01. exclude_self must keep the holder out. */
static void a_bp4_013_n_paid_discard_other_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);

    int me = mid(&tg, "PL!-bp4-013-N");
    if (!ident(&tg, me, "PL!-bp4-013-N")) return;
    int mate = mid(&tg, "PL!N-sd1-010-SD");
    set_stage3(&tg, 0, mate, me, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, mid(&tg, "PL!N-sd1-010-SD"));

    hl_fire_live_start(&tg, me);
    CHECK(test_has_pending_choice(&tg), "optional discard gate must be offered");
    rb_resume_with_choice(&tg.state, 0);   /* select_option(0) = accept */
    int guard = 0;
    while (test_has_pending_choice(&tg) && guard++ < 10)
        rb_resume_with_choice(&tg.state, 0);

    CHECK_EQ(heart01_of(&tg, mate), 1,
             "the OTHER stage member gains heart01 until live end");
    CHECK_EQ(heart01_of(&tg, me), 0,
             "exclude_self: the ability holder gains nothing");

    /* Declined twin. */
    static TestGame tg2;
    test_game_new(&tg2);
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    if (filler2 < 0) return;
    fill_decks(&tg2, filler2, 10);
    int me2 = mid(&tg2, "PL!-bp4-013-N");
    if (!ident(&tg2, me2, "PL!-bp4-013-N")) return;
    int mate2 = mid(&tg2, "PL!N-sd1-010-SD");
    set_stage3(&tg2, 0, mate2, me2, RB_EMPTY_SLOT);
    test_add_to_hand(&tg2, mid(&tg2, "PL!N-sd1-010-SD"));
    hl_fire_live_start(&tg2, me2);
    if (test_has_pending_choice(&tg2)) rb_resume_with_choice(&tg2.state, 1); /* decline */
    CHECK_EQ(heart01_of(&tg2, mate2), 0, "declined -> no heart gain");
}

/* live_start_paid_energy_heart01_gain_test.rs
 *   pl_hs_pr_029_pr_pay_energy_grants_heart01
 *   pl_hs_pr_029_pr_decline_energy_no_heart01
 * PL!HS-PR-029-PR: optional 「エネルギー1つocas」 paid -> heart01 until live end.
 * The energy CONSUMPTION is half the claim, so it is asserted too. */
static void a_hs_pr_029_pr_paid_energy(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    int me = mid(&tg, "PL!HS-PR-029-PR");
    if (!ident(&tg, me, "PL!HS-PR-029-PR")) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, me, RB_EMPTY_SLOT);
    test_give_energy(&tg, 3);
    int active_before = rb_energy_active_count(&tg.state.p[0]);

    hl_fire_live_start(&tg, me);
    CHECK(test_has_pending_choice(&tg), "optional energy cost prompted");
    rb_resume_with_choice(&tg.state, 1);   /* select_option(1) = Pay */
    hl_drain_skip(&tg);
    CHECK_EQ(heart01_of(&tg, me), 1, "paid -> heart01 until live end");
    CHECK(rb_energy_active_count(&tg.state.p[0]) < active_before,
          "energy was consumed");

    static TestGame tg2;
    test_game_new(&tg2);
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    if (filler2 < 0) return;
    fill_decks(&tg2, filler2, 10);
    int me2 = mid(&tg2, "PL!HS-PR-029-PR");
    if (!ident(&tg2, me2, "PL!HS-PR-029-PR")) return;
    set_stage3(&tg2, 0, RB_EMPTY_SLOT, me2, RB_EMPTY_SLOT);
    test_give_energy(&tg2, 3);
    hl_fire_live_start(&tg2, me2);
    rb_resume_with_choice(&tg2.state, -1);  /* Rust select_indices(&[]) = decline */
    CHECK_EQ(heart01_of(&tg2, me2), 0, "declined energy -> no heart01");
}

/* live_start_discard_shuffle_group_members_heart01_test.rs
 * PL!N-bp7-028-L (Cooking with Love): if your DISCARD holds a 『虹ヶ咲』 live
 * card AND a 『虹ヶ咲』 member card with no blade heart, you may shuffle the
 * whole discard onto the deck bottom; doing so gives every 『虹ヶ咲』 member on
 * your stage heart01 until live end.
 *
 * The Rust file's own header records the two parser defects these tests pin:
 * the condition used location:"stage" instead of the discard, and the AND
 * branch lost the live-card prong. The condition is a conjunction of two
 * objvar()-carried sub-conditions, so every negative case below is a distinct
 * branch of the AND, not a near-duplicate. */
static void a_cooking_bp7_028_l_condition_branches(void)
{
    /* helper: build a fresh board, put the named prints in P1's discard and
     * the standard three-member stage, fire, and report whether the optional
     * offer appeared. */
    struct { const char *discard[3]; int n; int expect_offer; const char *msg; } cases[] = {
        { { "PL!N-bp1-026-L", "PL!N-bp1-001-R", 0 }, 2, 1,
          "LIVE card + blade-less member in discard -> the may-shuffle offer appears" },
        { { "PL!N-bp1-026-L", "PL!N-bp7-007-R＋", 0 }, 2, 0,
          "a member WITH a blade heart does not satisfy the no-blade-heart prong" },
        { { "PL!N-bp1-026-L", 0, 0 }, 1, 0,
          "a LIVE card alone must not satisfy the AND condition" },
        { { "PL!N-bp1-001-R", 0, 0 }, 1, 0,
          "a blade-less member alone must not satisfy the AND condition" },
        { { "PL!N-bp7-007-R＋", 0, 0 }, 1, 0,
          "a member with a blade heart alone must not satisfy the condition" },
        { { "PL!SP-sd1-001-SD", "PL!SP-sd1-001-SD", 0 }, 2, 0,
          "non-虹ヶ咲 discard cards must not satisfy the condition" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        static TestGame tg;
        test_game_new(&tg);
        int bad = 0;
        for (int k = 0; k < cases[i].n; k++) {
            int c = mid(&tg, cases[i].discard[k]);
            if (c < 0 || !ident(&tg, c, cases[i].discard[k])) { bad = 1; break; }
            test_add_to_discard(&tg, c);
        }
        if (bad) return;
        set_stage3(&tg, 0,
                  mid(&tg, "PL!N-bp1-001-R"),
                  mid(&tg, "PL!N-bp7-007-R＋"),
                  mid(&tg, "PL!SP-sd1-001-SD"));
        int cooking = mid(&tg, "PL!N-bp7-028-L");
        if (!ident(&tg, cooking, "PL!N-bp7-028-L")) return;
        hl_fire_live_start(&tg, cooking);
        CHECK_EQ(test_has_pending_choice(&tg), cases[i].expect_offer, cases[i].msg);
        hl_drain_skip(&tg);
    }
}

/* cooking_condition_checks_discard_not_stage: the qualifying cards are on
 * STAGE, the discard is empty -> no offer. This is the test that pins the
 * parser's wrong location:"stage". */
static void a_cooking_bp7_028_l_condition_scopes_to_discard(void)
{
    static TestGame tg;
    test_game_new(&tg);
    /* Two DISTINCT instances: the same id in both slots is not a board that
     * can occur, and a "count the qualifying members" condition would
     * collapse to one. */
    set_stage3(&tg, 0,
               mid(&tg, "PL!N-bp1-001-R"),
               mid(&tg, "PL!SP-sd1-001-SD"),
               mid(&tg, "PL!SP-sd1-001-SD"));
    CHECK_EQ(tg.state.p[0].discard.n, 0, "setup: the discard starts empty");
    int cooking = mid(&tg, "PL!N-bp7-028-L");
    if (!ident(&tg, cooking, "PL!N-bp7-028-L")) return;
    hl_fire_live_start(&tg, cooking);
    CHECK(!test_has_pending_choice(&tg),
          "cards on stage (not in discard) must not satisfy the discard condition");
    hl_drain_skip(&tg);
}

/* cooking_accept_shuffles_discard_to_deck_bottom / cooking_decline_does_nothing
 * / cooking_accept_gives_heart01_to_all_niji_members -- the accept / decline
 * twins, and the actual heart grant. */
static void a_cooking_bp7_028_l_accept_and_decline(void)
{
    /* accept */
    static TestGame tg;
    test_game_new(&tg);
    int live_in_discard = mid(&tg, "PL!N-bp1-026-L");
    int member_in_discard = mid(&tg, "PL!N-bp1-001-R");
    int extra_in_discard = mid(&tg, "PL!SP-sd1-001-SD");
    if (!ident(&tg, live_in_discard, "PL!N-bp1-026-L")) return;
    test_add_to_discard(&tg, live_in_discard);
    test_add_to_discard(&tg, member_in_discard);
    test_add_to_discard(&tg, extra_in_discard);
    int member_a = mid(&tg, "PL!N-bp1-001-R");
    int member_b = mid(&tg, "PL!N-bp7-007-R＋");
    int non_niji = mid(&tg, "PL!SP-sd1-001-SD");
    set_stage3(&tg, 0, member_a, member_b, non_niji);
    int deck_before = tg.state.p[0].deck.n;

    int cooking = mid(&tg, "PL!N-bp7-028-L");
    if (!ident(&tg, cooking, "PL!N-bp7-028-L")) return;
    hl_fire_live_start(&tg, cooking);
    int offered = test_has_pending_choice(&tg);
    CHECK(offered, "the may-shuffle option is offered");
    if (offered) rb_resume_with_choice(&tg.state, 1);  /* select_option(1) = Pay */
    hl_drain_first(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "all discard cards should be shuffled to the deck bottom");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before + 3, "3 discard cards go back to deck");
    CHECK(deck_has(&tg, live_in_discard) && deck_has(&tg, member_in_discard) &&
              deck_has(&tg, extra_in_discard),
          "every shuffled card landed in the deck");
    CHECK_EQ(heart_of(&tg, member_a, H01), 1, "虹ヶ咲 member A gains heart01");
    CHECK_EQ(heart_of(&tg, member_b, H01), 1, "虹ヶ咲 member B gains heart01");
    CHECK_EQ(heart_of(&tg, non_niji, H01), 0,
             "a non-虹ヶ咲 member must NOT gain heart01");

    /* decline */
    static TestGame tg2;
    test_game_new(&tg2);
    int m1 = mid(&tg2, "PL!N-bp1-026-L");
    int m2 = mid(&tg2, "PL!N-bp1-001-R");
    if (!ident(&tg2, m1, "PL!N-bp1-026-L")) return;
    test_add_to_discard(&tg2, m1);
    test_add_to_discard(&tg2, m2);
    int stage_a = mid(&tg2, "PL!N-bp1-001-R");
    set_stage3(&tg2, 0, stage_a, mid(&tg2, "PL!N-bp7-007-R＋"),
               mid(&tg2, "PL!SP-sd1-001-SD"));
    int discard_before = tg2.state.p[0].discard.n;
    int deck2_before = tg2.state.p[0].deck.n;
    int cooking2 = mid(&tg2, "PL!N-bp7-028-L");
    if (!ident(&tg2, cooking2, "PL!N-bp7-028-L")) return;
    hl_fire_live_start(&tg2, cooking2);
    int offered2 = test_has_pending_choice(&tg2);
    CHECK(offered2, "the may-shuffle option is offered");
    if (offered2) rb_resume_with_choice(&tg2.state, 0);  /* select_option(0) = Skip */
    hl_drain_skip(&tg2);
    CHECK_EQ(tg2.state.p[0].discard.n, discard_before,
             "declining must leave the discard untouched");
    CHECK_EQ(tg2.state.p[0].deck.n, deck2_before,
             "declining must not touch the deck");
    CHECK_EQ(heart_of(&tg2, stage_a, H01), 0, "declining must not grant heart01");
    CHECK(discard_has(&tg2, m2),
          "the discarded member should remain in the discard after declining");
}

/* live_start_four_blade_group_member_gain_two_heart02_test.rs
 * PL!N-sd2-026-P (Fire Bird): ONE 『虹ヶ咲』 member on your stage with 4 or
 * more CURRENT blades gains 2x heart02 until live end. The Rust header
 * records defect D19: the parser emitted a "concurrent heart+blade grant", so
 * the card granted +4 BLADE and +4 heart02 to an unfiltered member. Every
 * assertion below is chosen to catch exactly that: the blade modifier is
 * pinned at 0 in every case, and the eligibility boundary is 4 (not 5). */
static void a_fire_bird_four_blade_group_heart02(void)
{
    /* one eligible member, blade 5 -> auto-targeted */
    {
        static TestGame tg;
        test_game_new(&tg);
        int miyashita = mid(&tg, "PL!N-PR-028-PR");
        if (!ident(&tg, miyashita, "PL!N-PR-028-PR")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, miyashita, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, miyashita, H02), 2,
                 "a 5-blade 虹ヶ咲 member gains exactly +2 heart02");
        CHECK_EQ(heart_of(&tg, miyashita, H01), 0,
                 "no other heart colour should be gained");
        CHECK_EQ(test_get_blade_modifier(&tg, miyashita), 0,
                 "Fire Bird must NOT grant blades (defect D19 granted +4 blade)");
    }
    /* boundary: EXACTLY 4 blades is eligible */
    {
        static TestGame tg;
        test_game_new(&tg);
        int ayumu = mid(&tg, "PL!N-bp5-001-R＋");
        if (!ident(&tg, ayumu, "PL!N-bp5-001-R＋")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, ayumu, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, ayumu, H02), 2, "exactly-4-blade member is eligible");
        CHECK_EQ(test_get_blade_modifier(&tg, ayumu), 0, "and still gains no blade");
    }
    /* below the threshold: 3 blades -> ineligible */
    {
        static TestGame tg;
        test_game_new(&tg);
        int shiori = mid(&tg, "PL!N-sd1-012-SD");
        if (!ident(&tg, shiori, "PL!N-sd1-012-SD")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, shiori, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, shiori, H02), 0, "blade-3 member must NOT receive heart02");
        CHECK_EQ(test_get_blade_modifier(&tg, shiori), 0, "and gains no blade");
    }
    /* 2 blades -> ineligible */
    {
        static TestGame tg;
        test_game_new(&tg);
        int kasumi = mid(&tg, "PL!N-bp1-002-R＋");
        if (!ident(&tg, kasumi, "PL!N-bp1-002-R＋")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, kasumi, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, kasumi, H02), 0, "no gain with a 2-blade member");
    }
    /* a 4-blade member of ANOTHER group is excluded by the 虹ヶ咲 filter */
    {
        static TestGame tg;
        test_game_new(&tg);
        int chika = mid(&tg, "PL!S-bp2-001-R");
        if (!ident(&tg, chika, "PL!S-bp2-001-R")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, chika, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, chika, H02), 0,
                 "a non-虹ヶ咲 4-blade member must be excluded by the group filter");
    }
    /* the filter reads the CURRENT blade total (base + modifiers) */
    {
        static TestGame tg;
        test_game_new(&tg);
        int shiori = mid(&tg, "PL!N-sd1-012-SD");
        if (!ident(&tg, shiori, "PL!N-sd1-012-SD")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, shiori, RB_EMPTY_SLOT);
        rb_mods_add_blade(&tg.state.mods, shiori, 1);   /* printed 3 + 1 = 4 */
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, shiori, H02), 2,
                 "current blade total (3 printed + 1 modifier = 4) satisfies >= 4");
    }
    /* a WAITED member is still 「ステージにいる」 and stays eligible */
    {
        static TestGame tg;
        test_game_new(&tg);
        int miyashita = mid(&tg, "PL!N-PR-028-PR");
        if (!ident(&tg, miyashita, "PL!N-PR-028-PR")) return;
        set_stage3(&tg, 0, RB_EMPTY_SLOT, miyashita, RB_EMPTY_SLOT);
        rb_mods_set_orientation(&tg.state.mods, miyashita, "wait");
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        hl_drain_skip(&tg);
        CHECK_EQ(heart_of(&tg, miyashita, H02), 2,
                 "waited members are still on the stage and eligible");
    }
    /* multiple eligible members: exactly one is chosen, the other gets nothing */
    {
        static TestGame tg;
        test_game_new(&tg);
        int ayumu = mid(&tg, "PL!N-bp5-001-R＋");
        int miyashita = mid(&tg, "PL!N-PR-028-PR");
        if (!ident(&tg, ayumu, "PL!N-bp5-001-R＋")) return;
        if (!ident(&tg, miyashita, "PL!N-PR-028-PR")) return;
        set_stage3(&tg, 0, ayumu, miyashita, RB_EMPTY_SLOT);
        int bird = mid(&tg, "PL!N-sd2-026-P");
        if (!ident(&tg, bird, "PL!N-sd2-026-P")) return;
        test_add_to_live(&tg, bird);
        hl_fire_live_start(&tg, bird);
        CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
              "multiple eligible members should require a selection prompt");
        rb_resume_with_choice(&tg.state, 0);
        hl_drain_proceed(&tg);
        int a = heart_of(&tg, ayumu, H02);
        int m = heart_of(&tg, miyashita, H02);
        CHECK((a == 2 && m == 0) || (a == 0 && m == 2),
              "exactly one of the two eligible members gains +2 heart02");
        CHECK_EQ(test_get_blade_modifier(&tg, ayumu) +
                     test_get_blade_modifier(&tg, miyashita), 0,
                 "no blade gain regardless of selection");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * B. TURN-WALK tests -- the ability is reached by driving the REAL phase
 *    machine (test_pass / test_set_live_card), like the Rust tests that call
 *    advance_to_live_set / finish_live_setup. A red check in section B can be
 *    a mistimed setup OR an engine gap, so each of these names the phase it
 *    expects to be standing on.
 * ══════════════════════════════════════════════════════════════════════════ */

/* The common Rust preamble for the walk tests:
 *   advance_to_live_set(game);
 *   game.state.player1.hand.cards.clear();
 *   game.state.player1.hand.cards.push(<cost cards>);
 *   game.state.player1.hand.cards.push(live);
 *   game.set_live_card(live);
 *   game.state.player1.main_deck.cards.clear();   (in the hand-cost tests)
 *   finish_live_setup(game);
 * `hand` must already contain `live` as its LAST entry; set_live_card lifts it
 * out of hand into the live zone. */
static void hl_walk_to_live_start(TestGame *tg, const int *hand, int hand_n,
                                   int live, int clear_decks_after)
{
    hl_advance_to_live_set(tg);
    set_hand(tg, hand, hand_n);
    test_set_live_card(tg, 0, live);
    if (clear_decks_after) clear_decks(tg);
    hl_advance_to_live_start(tg);
}

/* live_start_same_group_pair_cost_heart01_test.rs
 * PL!HS-bp5-006-R (安養寺姫芽): optional 「手札の同じグループ名を持つカード2枚を
 * 控え室に置いてもよい」 -> heart01 x2 until live end. "同じグループ名" is a
 * relation BETWEEN the two discarded cards, not a match against the holder.
 * The auto-skip negatives are the load-bearing part: an unpayable optional
 * cost must vanish WITHOUT a prompt, and if it prompts instead the "no
 * heart01" assertion below would pass for the wrong reason. */
static void b_himeno_bp5_006_same_group_pair(void)
{
    struct { const char *hand[3]; int n; int expect_prompt; int expect_h01;
             const char *msg_prompt; } cases[] = {
        { { "PL!HS-bp6-011-R", "PL!HS-bp6-011-R", 0 }, 2, 1, 2,
          "a 蓮ノ空 pair in hand triggers the cost prompt" },
        { { "PL!N-sd1-010-SD", "PL!N-sd1-010-SD", 0 }, 2, 1, 2,
          "a pair whose group differs from the holder's still qualifies" },
        { { "PL!HS-bp6-011-R", "PL!N-sd1-010-SD", 0 }, 2, 0, 0,
          "1 蓮ノ空 + 1 Printemps = no matching pair -> auto-skip" },
        { { "PL!HS-bp6-011-R", "PL!N-sd1-010-SD", 0 }, 2, 0, 0,
          "only 1 same-group card in hand -> need 2 -> auto-skip" },
        { { "PL!-bp5-111-R", "PL!-bp5-111-R", 0 }, 2, 0, 0,
          "two no-group cards share no group name -> auto-skip" },
        { { "PL!HS-bp6-011-R", "PL!HS-bp1-012-PR", 0 }, 2, 1, 2,
          "cross-unit same-group pair (みらくらぱーく! + スリーズブーケ)" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        static TestGame tg;
        test_game_new(&tg);
        int himeno = mid(&tg, "PL!HS-bp5-006-R");
        int filler = mid(&tg, "PL!N-sd1-010-SD");
        if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
        if (filler < 0) return;
        fill_decks(&tg, filler, 10);
        test_give_energy(&tg, 10);
        set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);

        int hand[3];
        int bad = 0;
        for (int k = 0; k < cases[i].n; k++) {
            int c = cases[i].hand[k][0] == 'P' && cases[i].hand[k][1] == 'L' &&
                            !strncmp(cases[i].hand[k], "PL!HS", 5)
                        ? mid(&tg, cases[i].hand[k])
                        : mid(&tg, cases[i].hand[k]);
            if (c < 0 || !ident(&tg, c, cases[i].hand[k])) { bad = 1; break; }
            hand[k] = c;
        }
        if (bad) return;
        int live = mid(&tg, "PL!-sd1-020-SD");
        if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
        hand[cases[i].n] = live;
        hl_walk_to_live_start(&tg, hand, cases[i].n + 1, live, 1);

        CHECK_EQ(test_has_pending_choice(&tg), cases[i].expect_prompt, cases[i].msg_prompt);
        if (cases[i].expect_prompt) {
            rb_resume_with_choice(&tg.state, 0);
            CHECK(test_has_pending_choice(&tg), "second selection prompt should appear");
            rb_resume_with_choice(&tg.state, 0);
        }
        CHECK_EQ(heart01_of(&tg, himeno), cases[i].expect_h01,
                 "himeno heart01 after the cost is paid or skipped");
    }
}

/* himeno_bp5_skip_cost_via_choice: the explicit DECLINE of a payable cost.
 * The cards must stay in hand and no heart is granted. */
static void b_himeno_bp5_006_skip_cost(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int himeno = mid(&tg, "PL!HS-bp5-006-R");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
    int s1 = mid(&tg, "PL!HS-bp6-011-R");
    int s2 = mid(&tg, "PL!HS-bp6-011-R");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, s1, "PL!HS-bp6-011-R")) return;
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[3] = { s1, s2, live };
    hl_walk_to_live_start(&tg, hand, 3, live, 1);

    CHECK(test_has_pending_choice(&tg), "the payable cost prompts");
    rb_resume_with_choice(&tg.state, -1);   /* Rust try_select_indices(&[]) */
    CHECK_EQ(heart01_of(&tg, himeno), 0, "declining the cost grants no heart01");
    CHECK(hand_has(&tg, s1) && hand_has(&tg, s2),
          "both cost cards stay in hand after declining");
}

/* himeno_bp5_discarded_go_to_waitroom: paying the cost must MOVE the two
 * cards to the waitroom. This is the assertion that separates "the cost was
 * answered" from "the cost was paid". */
static void b_himeno_bp5_006_cost_moves_cards_to_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int himeno = mid(&tg, "PL!HS-bp5-006-R");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
    int s1 = mid(&tg, "PL!HS-bp6-011-R");
    int s2 = mid(&tg, "PL!HS-bp6-011-R");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, s1, "PL!HS-bp6-011-R")) return;
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[3] = { s1, s2, live };
    hl_walk_to_live_start(&tg, hand, 3, live, 1);

    CHECK(test_has_pending_choice(&tg), "the cost prompt appears");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(test_has_pending_choice(&tg), "the second selection prompt appears");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(discard_has(&tg, s1) && discard_has(&tg, s2),
          "both paid cost cards land in the waitroom");
    CHECK_EQ(heart01_of(&tg, himeno), 2, "paying the pair costs 2 heart01");
}

/* himeno_bp5_left_position / himeno_bp5_right_position: the holder's stage
 * AREA must not matter for this ability (it reads hand, not the stage). */
static void b_himeno_bp5_006_stage_area_irrelevant(void)
{
    for (int area = 0; area < RB_STAGE_SIZE; area++) {
        static TestGame tg;
        test_game_new(&tg);
        int himeno = mid(&tg, "PL!HS-bp5-006-R");
        int filler = mid(&tg, "PL!N-sd1-010-SD");
        if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
        if (filler < 0) return;
        fill_decks(&tg, filler, 10);
        test_give_energy(&tg, 10);
        set_stage3(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        tg.state.p[0].stage[area] = himeno;
        int s1 = mid(&tg, "PL!HS-bp6-011-R");
        int s2 = mid(&tg, "PL!HS-bp6-011-R");
        int live = mid(&tg, "PL!-sd1-020-SD");
        if (!ident(&tg, s1, "PL!HS-bp6-011-R")) return;
        if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
        int hand[3] = { s1, s2, live };
        hl_walk_to_live_start(&tg, hand, 3, live, 1);
        rb_resume_with_choice(&tg.state, 0);
        if (test_has_pending_choice(&tg)) rb_resume_with_choice(&tg.state, 0);
        const char *where = area == 0 ? "left" : area == 1 ? "center" : "right";
        char msg[96];
        snprintf(msg, sizeof msg, "himeno in the %s area still pays the pair cost", where);
        CHECK_EQ(heart01_of(&tg, himeno), 2, msg);
    }
}

/* live_start_discarded_group_member_heart01_test.rs
 * PL!HS-bp5-003-R＋ (大沢瑠璃乃): optional discard 1 from hand; then ONE stage
 * member whose GROUP matches the DISCARDED card gains heart01. The Rust file's
 * point is that same_group_name resolves to the card's group (position ②) and
 * NOT its unit (position ③), so the candidate pool is asserted directly from
 * the pending choice's filtered_indices. */
static void b_rurino_bp5_003_discard_group_filter(void)
{
    /* Only the みらくらぱーく! members (hs@0, rurino@1) are selectable; the
     * μ's Printemps member at 2 is not in the pool at all. */
    static TestGame tg;
    test_game_new(&tg);
    int rurino = mid(&tg, "PL!HS-bp5-003-R＋");
    int hs = mid(&tg, "PL!HS-bp6-011-R");
    int muse = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, rurino, "PL!HS-bp5-003-R＋")) return;
    if (!ident(&tg, hs, "PL!HS-bp6-011-R")) return;
    set_stage3(&tg, 0, hs, rurino, muse);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);

    int cost_card = mid(&tg, "PL!HS-bp6-011-R");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, cost_card, "PL!HS-bp6-011-R")) return;
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[2] = { cost_card, live };
    hl_walk_to_live_start(&tg, hand, 2, live, 0);

    /* The Rust test's first two lines are try_select_indices(&[0]) and
     * try_select_indices(&[1]): answer the hand-discard cost, then pick the
     * stage member. The "cost prompts" CLAIM is asserted strictly on its own;
     * the navigation is guarded only so that an absent cost prompt is
     * reported once (as the engine gap it is) instead of also producing a
     * meaningless "index 1 out of range" style failure. */
    const RbChoice *c0 = rb_get_pending_choice(&tg.state);
    int cost_is_hand = c0 && c0->kind == RB_CHOICE_SELECT_CARD && c0->zone[0] &&
                       !strcmp(c0->zone, "hand");
    CHECK(cost_is_hand,
          "the optional hand-discard cost prompts for a card");
    if (cost_is_hand) rb_resume_with_choice(&tg.state, 0);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_CARD && ch->zone[0] &&
              !strcmp(ch->zone, "stage"),
          "the target prompt is a stage SelectCard");
    CHECK(ch && ch->n_filtered_indices == 2 && ch->filtered_indices[0] == 0 &&
              ch->filtered_indices[1] == 1,
          "only みらくらぱーく! members (hs@0, rurino@1) are selectable, not Printemps@2");
    rb_resume_with_choice(&tg.state, 1);   /* pick rurino */
    hl_drain_skip(&tg);

    CHECK_EQ(heart01_of(&tg, rurino), 1, "rurino (みらくらぱーく!) gets +1 heart01");
    CHECK_EQ(heart01_of(&tg, muse), 0, "μ's Printemps member gets no heart01");
    CHECK_EQ(heart01_of(&tg, hs), 0,
             "the OTHER matching member is unselected and gets nothing");
}

/* rurino_bp5_cross_unit_same_group_all_selectable: all three staged members
 * share group=蓮ノ空 across DIFFERENT units, so all three must be selectable.
 * This is the assertion that distinguishes group from unit. */
static void b_rurino_bp5_003_cross_unit_same_group(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rurino = mid(&tg, "PL!HS-bp5-003-R＋");
    int mirakura = mid(&tg, "PL!HS-bp6-011-R");
    int trois = mid(&tg, "PL!HS-bp1-012-PR");
    if (!ident(&tg, rurino, "PL!HS-bp5-003-R＋")) return;
    if (!ident(&tg, trois, "PL!HS-bp1-012-PR")) return;
    set_stage3(&tg, 0, mirakura, rurino, trois);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);

    int cost_card = mid(&tg, "PL!HS-bp6-011-R");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, cost_card, "PL!HS-bp6-011-R")) return;
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[2] = { cost_card, live };
    hl_walk_to_live_start(&tg, hand, 2, live, 0);

    const RbChoice *c0 = rb_get_pending_choice(&tg.state);
    int cost_is_hand = c0 && c0->kind == RB_CHOICE_SELECT_CARD && c0->zone[0] &&
                       !strcmp(c0->zone, "hand");
    CHECK(cost_is_hand, "the cost prompt appears");
    if (cost_is_hand) rb_resume_with_choice(&tg.state, 0);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->n_filtered_indices == 3 && ch->filtered_indices[0] == 0 &&
              ch->filtered_indices[1] == 1 && ch->filtered_indices[2] == 2,
          "all three 蓮ノ空 members are selectable across different units");
    rb_resume_with_choice(&tg.state, 1);  /* pick rurino */
    hl_drain_skip(&tg);
    CHECK_EQ(heart01_of(&tg, rurino), 1, "rurino gets +1 heart01");
    CHECK_EQ(heart01_of(&tg, trois), 0,
             "trois (スリーズブーケ) was not selected and gets nothing");
    CHECK(!test_has_pending_choice(&tg), "no pending choices after setup");
}

/* himeno_bp5_same_group_cost_filters_hand_cards (from the same Rust file):
 * the PAIR relation is computed over the HAND, so a card from a group with
 * only one hand representative is excluded from the candidate pool. */
static void b_himeno_bp5_006_hand_pair_filter(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int himeno = mid(&tg, "PL!HS-bp5-006-R");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
    int same1 = mid(&tg, "PL!HS-bp6-011-R");
    int same2 = mid(&tg, "PL!HS-bp6-011-R");
    int wrong = mid(&tg, "PL!N-sd1-010-SD");
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, same1, "PL!HS-bp6-011-R")) return;
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[4] = { same1, same2, wrong, live };
    hl_walk_to_live_start(&tg, hand, 4, live, 1);

    const RbChoice *c0 = rb_get_pending_choice(&tg.state);
    int cost_is_hand = c0 && c0->kind == RB_CHOICE_SELECT_CARD && c0->zone[0] &&
                       !strcmp(c0->zone, "hand");
    CHECK(cost_is_hand, "the cost prompt should appear as a hand SelectCard");
    if (cost_is_hand) rb_resume_with_choice(&tg.state, 0);
    /* Rust: the second prompt is the SECOND hand pick of the same-group pair.
     * Asserting the pair filter on whatever prompt is actually on screen keeps
     * the claim meaningful whether the engine is one step ahead or behind. */
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_CARD,
          "the same-group pair cost is a card selection prompt");
    CHECK(ch && ch->n_filtered_indices == 2 && ch->filtered_indices[0] == 0 &&
              ch->filtered_indices[1] == 1,
          "only the 蓮ノ空 pair (indices 0,1) is selectable, not the lone Printemps card");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(test_has_pending_choice(&tg), "second selection prompt should appear");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(discard_has(&tg, same1) && discard_has(&tg, same2),
          "both same-group cards go to the waitroom");
    CHECK(hand_has(&tg, wrong), "the lone-group card stays in hand");
    CHECK_EQ(heart01_of(&tg, himeno), 2, "the activating card gains 2 heart01");
}

/* himeno_bp5_same_group_cost_auto_skips_when_no_match /
 * ..._auto_skips_when_only_no_group_cards: the optional cost must be
 * auto-skipped (no prompt at all) when no pair exists. A prompt here would
 * make the following heart01==0 check pass for the wrong reason, so the
 * prompt check is deliberately the FIRST assertion. */
static void b_himeno_bp5_006_cost_auto_skips(void)
{
    struct { const char *hand[3]; int n; const char *msg; } cases[] = {
        { { "PL!HS-bp6-011-R", "PL!N-sd1-010-SD", 0 }, 2,
          "1 蓮ノ空 + 1 Printemps = no matching pair -> no cost prompt" },
        { { "PL!-bp5-111-R", "PL!-bp5-111-R", 0 }, 2,
          "only no-group cards in hand -> no cost prompt" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        static TestGame tg;
        test_game_new(&tg);
        int himeno = mid(&tg, "PL!HS-bp5-006-R");
        int filler = mid(&tg, "PL!N-sd1-010-SD");
        if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
        if (filler < 0) return;
        fill_decks(&tg, filler, 10);
        test_give_energy(&tg, 10);
        set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
        int hand[3];
        for (int k = 0; k < cases[i].n; k++) {
            int c = mid(&tg, cases[i].hand[k]);
            if (c < 0 || !ident(&tg, c, cases[i].hand[k])) return;
            hand[k] = c;
        }
        int live = mid(&tg, "PL!-sd1-020-SD");
        if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
        hand[cases[i].n] = live;
        hl_walk_to_live_start(&tg, hand, cases[i].n + 1, live, 1);
        CHECK(!test_has_pending_choice(&tg), cases[i].msg);
        CHECK_EQ(heart01_of(&tg, himeno), 0,
                 "no heart01 should be granted when the cost is auto-skipped");
    }
}

/* himeno_bp5_empty_hand_skips: an empty hand cannot pay a 2-card cost. */
static void b_himeno_bp5_006_empty_hand_skips(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int himeno = mid(&tg, "PL!HS-bp5-006-R");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, himeno, "PL!HS-bp5-006-R")) return;
    if (filler < 0) return;
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
    int live = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
    int hand[1] = { live };
    hl_walk_to_live_start(&tg, hand, 1, live, 1);
    CHECK(!test_has_pending_choice(&tg), "empty hand -> no cost prompt");
    CHECK_EQ(heart01_of(&tg, himeno), 0, "no heart01 without the cost");
}

/* himeno_bp5_p_variant / himeno_bp5_ar_variant: the same ability on the other
 * two rarity prints. A parser that only emitted the R ability would pass every
 * case above and fail here, so the variants are worth their own assertions. */
static void b_himeno_bp5_006_rarity_variants(void)
{
    const char *variants[2] = { "PL!HS-bp5-006-P", "PL!HS-bp5-006-AR" };
    for (int v = 0; v < 2; v++) {
        static TestGame tg;
        test_game_new(&tg);
        int himeno = mid(&tg, variants[v]);
        int filler = mid(&tg, "PL!N-sd1-010-SD");
        if (!ident(&tg, himeno, variants[v])) return;
        if (filler < 0) return;
        fill_decks(&tg, filler, 10);
        test_give_energy(&tg, 10);
        set_stage3(&tg, 0, RB_EMPTY_SLOT, himeno, RB_EMPTY_SLOT);
        int s1 = mid(&tg, "PL!HS-bp6-011-R");
        int s2 = mid(&tg, "PL!HS-bp6-011-R");
        int live = mid(&tg, "PL!-sd1-020-SD");
        if (!ident(&tg, s1, "PL!HS-bp6-011-R")) return;
        if (!ident(&tg, live, "PL!-sd1-020-SD")) return;
        int hand[3] = { s1, s2, live };
        hl_walk_to_live_start(&tg, hand, 3, live, 1);
        CHECK(test_has_pending_choice(&tg), "the variant's cost prompt appears");
        rb_resume_with_choice(&tg.state, 0);
        if (test_has_pending_choice(&tg)) rb_resume_with_choice(&tg.state, 0);
        char msg[96];
        snprintf(msg, sizeof msg, "%s: paying the pair grants heart01 x2", variants[v]);
        CHECK_EQ(heart01_of(&tg, himeno), 2, msg);
    }
}

/* live_start_discard_distinct_heart_colors_q246_test.rs
 * LL-bp6-001-R＋: you may put any number of 南ことり/黒澤ダイヤ/徒町小鈴 from
 * hand into the waitroom; for each DISTINCT heart colour those cards carry you
 * gain 1 heart of that colour until live end. The load-bearing behaviour is the
 * DEDUPLICATION across overlapping colours and the EXCLUSION of colours on
 * cards you did not choose. */
static void b_bp6_001_r_discard_distinct_heart_colors(void)
{
    /* Q246 main: kosuzu {H04,H05,H06} + dia {H02,H05} -> H05 shared, counted
     * once: four distinct colours, each exactly one. */
    static TestGame tg;
    test_game_new(&tg);
    int joint = mid(&tg, "LL-bp6-001-R＋");
    int kosuzu = mid(&tg, "PL!HS-pb1-005-R");
    int dia = mid(&tg, "PL!S-bp3-004-R");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, joint, "LL-bp6-001-R＋")) return;
    if (!ident(&tg, kosuzu, "PL!HS-pb1-005-R")) return;
    if (!ident(&tg, dia, "PL!S-bp3-004-R")) return;
    if (filler < 0) return;
    set_stage3(&tg, 0, RB_EMPTY_SLOT, joint, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);

    int hand[3] = { kosuzu, dia, filler };
    hl_walk_to_live_start(&tg, hand, 3, filler, 0);

    const RbChoice *c0 = rb_get_pending_choice(&tg.state);
    int cost_is_hand = c0 && c0->kind == RB_CHOICE_SELECT_CARD && c0->zone[0] &&
                       !strcmp(c0->zone, "hand");
    CHECK(cost_is_hand, "bp6 should prompt for an any-number discard");
    if (cost_is_hand) {
        const int pick[2] = { 0, 1 };
        rb_resume_with_choice_indices(&tg.state, pick, 2);
        if (test_has_pending_choice(&tg)) rb_resume_with_choice(&tg.state, -1);
    }
    hl_drain_skip(&tg);

    CHECK_EQ(heart_of(&tg, joint, H02), 1, "H02 from dia");
    CHECK_EQ(heart_of(&tg, joint, H04), 1, "H04 from kosuzu");
    CHECK_EQ(heart_of(&tg, joint, H05), 1, "H05 is shared and is counted ONCE");
    CHECK_EQ(heart_of(&tg, joint, H06), 1, "H06 from kosuzu");
    CHECK_EQ(heart_of(&tg, joint, H01), 0, "H01 was not present on either card");
    CHECK_EQ(heart_of(&tg, joint, H03), 0, "H03 was not present on either card");

    /* Subset selection: a THIRD eligible card in hand that is not chosen must
     * contribute none of its colours. */
    static TestGame tg2;
    test_game_new(&tg2);
    int joint2 = mid(&tg2, "LL-bp6-001-R＋");
    int kosuzu2 = mid(&tg2, "PL!HS-pb1-005-R");
    int dia2 = mid(&tg2, "PL!S-bp3-004-R");
    int kotori2 = mid(&tg2, "PL!-bp3-003-R");
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    if (!ident(&tg2, joint2, "LL-bp6-001-R＋")) return;
    if (!ident(&tg2, kotori2, "PL!-bp3-003-R")) return;
    if (filler2 < 0) return;
    set_stage3(&tg2, 0, RB_EMPTY_SLOT, joint2, RB_EMPTY_SLOT);
    fill_decks(&tg2, filler2, 10);
    test_give_energy(&tg2, 10);
    int hand2[4] = { kosuzu2, dia2, kotori2, filler2 };
    hl_walk_to_live_start(&tg2, hand2, 4, filler2, 0);
    const RbChoice *c1 = rb_get_pending_choice(&tg2.state);
    int cost2 = c1 && c1->kind == RB_CHOICE_SELECT_CARD && c1->zone[0] &&
                !strcmp(c1->zone, "hand");
    CHECK(cost2, "bp6 prompts for the any-number discard");
    if (cost2) {
        const int pick2[2] = { 0, 1 };
        rb_resume_with_choice_indices(&tg2.state, pick2, 2);
        if (test_has_pending_choice(&tg2)) rb_resume_with_choice(&tg2.state, -1);
    }
    hl_drain_skip(&tg2);
    CHECK_EQ(heart_of(&tg2, joint2, H02), 1, "H02 from dia (subset case)");
    CHECK_EQ(heart_of(&tg2, joint2, H04), 1, "H04 from kosuzu (subset case)");
    CHECK_EQ(heart_of(&tg2, joint2, H05), 1, "H05 shared (subset case)");
    CHECK_EQ(heart_of(&tg2, joint2, H06), 1, "H06 from kosuzu (subset case)");
    CHECK_EQ(heart_of(&tg2, joint2, H01), 0,
             "H01 must NOT come from the unselected kotori");
    CHECK_EQ(heart_of(&tg2, joint2, H03), 0,
             "H03 must NOT come from the unselected kotori");

    /* Single eligible card: kosuzu alone -> exactly its three colours. */
    static TestGame tg3;
    test_game_new(&tg3);
    int joint3 = mid(&tg3, "LL-bp6-001-R＋");
    int kosuzu3 = mid(&tg3, "PL!HS-pb1-005-R");
    int filler3 = mid(&tg3, "PL!N-sd1-010-SD");
    if (!ident(&tg3, joint3, "LL-bp6-001-R＋")) return;
    if (!ident(&tg3, kosuzu3, "PL!HS-pb1-005-R")) return;
    if (filler3 < 0) return;
    set_stage3(&tg3, 0, RB_EMPTY_SLOT, joint3, RB_EMPTY_SLOT);
    fill_decks(&tg3, filler3, 10);
    test_give_energy(&tg3, 10);
    int hand3[2] = { kosuzu3, filler3 };
    hl_walk_to_live_start(&tg3, hand3, 2, filler3, 0);
    const RbChoice *c3 = rb_get_pending_choice(&tg3.state);
    int cost3 = c3 && c3->kind == RB_CHOICE_SELECT_CARD && c3->zone[0] &&
                !strcmp(c3->zone, "hand");
    CHECK(cost3, "bp6 prompts even with one eligible card");
    if (cost3) {
        rb_resume_with_choice(&tg3.state, 0);
        if (test_has_pending_choice(&tg3)) rb_resume_with_choice(&tg3.state, -1);
    }
    hl_drain_skip(&tg3);
    int total = 0;
    for (int z = H01; z <= H06; z++) total += heart_of(&tg3, joint3, z);
    CHECK_EQ(heart_of(&tg3, joint3, H04), 1, "H04 from the single kosuzu");
    CHECK_EQ(heart_of(&tg3, joint3, H05), 1, "H05 from the single kosuzu");
    CHECK_EQ(heart_of(&tg3, joint3, H06), 1, "H06 from the single kosuzu");
    CHECK_EQ(total, 3, "3 distinct hearts from the single kosuzu card");
}

/* live_start_chosen_heart_per_success_card_test.rs
 *   live_start_chosen_heart_scales_with_success_zone_card_count
 * PL!-bp3-013-N (園田海未): pick one of heart01/heart03/heart06 at live start,
 * then gain 1 of the chosen heart PER CARD in your success live-card zone.
 * This is the cluster's clearest "live-start gain feeds a live-dynamic count"
 * case: the modifier's SIZE is read from a zone the engine keeps mutating. */
static void b_umi_bp3_013_n_per_success_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int umi = mid(&tg, "PL!-bp3-013-N");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, umi, "PL!-bp3-013-N")) return;
    if (filler < 0) return;

    clear_decks(&tg);
    fill_decks(&tg, filler, 40);
    tg.state.p[1].deck.n = 10;
    for (int i = 0; i < 10; i++) tg.state.p[1].deck.cards[i] = filler;
    test_add_to_hand(&tg, filler);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, umi, RB_EMPTY_SLOT);
    for (int i = 0; i < 3; i++) test_add_to_success(&tg, filler);
    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;
    test_give_energy(&tg, 20);

    /* Rust: while !game.has_pending_choice() && passes < 20 { game.pass() } --
     * stop the instant the choice appears, because the next pass would
     * auto-resolve it. */
    int passes = 0;
    while (!test_has_pending_choice(&tg) && passes < 20) { passes++; test_pass(&tg); }
    CHECK(test_has_pending_choice(&tg),
          "Umi's ability should create a heart-colour choice at live start");

    rb_resume_with_choice(&tg.state, 0);  /* select_option(0): first colour */
    CHECK(!test_has_pending_choice(&tg),
          "the per-unit heart gain must resolve without a further prompt");
    CHECK_EQ(heart01_of(&tg, umi), 3,
             "gained 3 heart01 (1 per 3 success-zone cards)");

    /* The modifier is a per-CARD value, so it must reach the stage heart
     * aggregate -- that is the only path by which a live-start heart gain
     * affects the live score. Rust reads calculate_stage_hearts(); the C
     * equivalent is rb_calc_stage_hearts. */
    int agg[8];
    rb_calc_stage_hearts(&tg.state, 0, agg);
    CHECK(agg[H01] >= 3,
          "the live-start heart01 modifier reaches the stage-heart aggregate");
}

/* live_start_equal_success_counts_heart02_test.rs
 * PL!N-bp5-007-R＋ (優木せつ菜): if your success-zone card count EQUALS your
 * opponent's, gain heart02 x2 until live end. Both the 0-vs-0 and the
 * 2-vs-1 negatives are covered, plus the other rarity prints. */
static void b_setsuna_bp5_007_equal_success_counts(void)
{
    /* both zero -> equal -> granted */
    static TestGame tg;
    test_game_new(&tg);
    int setsuna = mid(&tg, "PL!N-bp5-007-R＋");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    int live = mid(&tg, "PL!-sd1-019-SD");
    if (!ident(&tg, setsuna, "PL!N-bp5-007-R＋")) return;
    if (!ident(&tg, live, "PL!-sd1-019-SD")) return;
    if (filler < 0) return;
    clear_decks(&tg);
    fill_decks(&tg, filler, 40);
    set_stage3(&tg, 0, setsuna, filler, RB_EMPTY_SLOT);
    int hand[2] = { live, filler };
    hl_walk_to_live_start(&tg, hand, 2, live, 0);
    test_give_energy(&tg, 15);
    CHECK_EQ(tg.state.p[0].success.n, 0, "setup: P1 success zone starts empty");
    CHECK_EQ(tg.state.p[1].success.n, 0, "setup: P2 success zone starts empty");
    hl_drain_first(&tg);
    CHECK_EQ(heart_of(&tg, setsuna, H02), 2,
             "Q230: both success counts are 0 -> heart02 x2 gained");

    /* 2 vs 1 -> not equal -> nothing */
    static TestGame tg2;
    test_game_new(&tg2);
    int setsuna2 = mid(&tg2, "PL!N-bp5-007-R＋");
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    int live2 = mid(&tg2, "PL!-sd1-019-SD");
    if (!ident(&tg2, setsuna2, "PL!N-bp5-007-R＋")) return;
    if (!ident(&tg2, live2, "PL!-sd1-019-SD")) return;
    if (filler2 < 0) return;
    clear_decks(&tg2);
    fill_decks(&tg2, filler2, 40);
    set_stage3(&tg2, 0, setsuna2, filler2, RB_EMPTY_SLOT);
    test_add_to_success(&tg2, live2);
    test_add_to_success(&tg2, live2);
    test_add_to_opp_success(&tg2, live2);
    int hand2[2] = { live2, filler2 };
    hl_walk_to_live_start(&tg2, hand2, 2, live2, 0);
    test_give_energy(&tg2, 15);
    hl_drain_first(&tg2);
    CHECK_EQ(heart_of(&tg2, setsuna2, H02), 0,
             "unequal success counts (P1=2, P2=1) -> condition fails -> no heart02");

    /* the other three rarity prints of the same ability */
    const char *variants[3] = { "PL!N-bp5-007-AR", "PL!N-bp5-007-P", "PL!N-bp5-007-SEC" };
    for (int v = 0; v < 3; v++) {
        static TestGame tgv;
        test_game_new(&tgv);
        int setsuna = mid(&tgv, variants[v]);
        int filler = mid(&tgv, "PL!N-sd1-010-SD");
        int live = mid(&tgv, "PL!-sd1-019-SD");
        if (!ident(&tgv, setsuna, variants[v])) return;
        if (!ident(&tgv, live, "PL!-sd1-019-SD")) return;
        if (filler < 0) return;
        clear_decks(&tgv);
        fill_decks(&tgv, filler, 40);
        set_stage3(&tgv, 0, setsuna, filler, RB_EMPTY_SLOT);
        int hand[2] = { live, filler };
        hl_walk_to_live_start(&tgv, hand, 2, live, 0);
        test_give_energy(&tgv, 15);
        hl_drain_first(&tgv);
        char msg[96];
        snprintf(msg, sizeof msg,
                 "%s: equal zero success counts should grant heart02 twice", variants[v]);
        CHECK_EQ(heart_of(&tgv, setsuna, H02), 2, msg);
    }
}

/* live_start_convert_selected_member_heart_color_test.rs
 * PL!HS-bp5-021-L (ジョーショーキリュー): at live start one 『蓮ノ空』 member
 * on your stage has its hearts converted to heart01 until live end. Like
 * PL!S-bp7-024-L this is a COLOUR REWRITE, asserted through
 * rb_mods_get_heart_color_multiplier, and like it the count of converted
 * members is 「1人」 -- not 「すべて」. */
static void b_hs_bp5_021_l_convert_selected_member(void)
{
    /* a single eligible member: converted, and the stage heart total collapses
     * onto heart01. */
    static TestGame tg;
    test_game_new(&tg);
    int live_card = mid(&tg, "PL!HS-bp5-021-L");
    int member = mid(&tg, "PL!HS-sd1-003-SD");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    if (!ident(&tg, live_card, "PL!HS-bp5-021-L")) return;
    if (!ident(&tg, member, "PL!HS-sd1-003-SD")) return;
    if (filler < 0) return;
    test_give_energy(&tg, 10);
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, member);
    test_play_to_stage(&tg, member, 1);
    hl_drain_skip(&tg);
    test_add_to_hand(&tg, live_card);
    hl_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live_card);
    hl_advance_to_live_start(&tg);
    hl_drain_first(&tg);

    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg.state.mods, member), RB_HEART_RED,
             "the member's hearts are converted to heart01");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg.state.mods, live_card), -1,
             "the live card must NOT be given a heart colour multiplier");
    int agg[8];
    rb_calc_stage_hearts(&tg.state, 0, agg);
    int total = 0;
    for (int z = 0; z < 8; z++) total += agg[z];
    CHECK(agg[RB_HEART_RED] >= 2, "at least 2 heart01 after the conversion");
    /* PL!HS-sd1-003-SD prints 1+1 = 2 hearts, so the Rust file's "total 3"
     * case belongs to PL!HS-sd1-001-SD (3 printed heart04) and is asserted
     * separately below. */
    CHECK_EQ(total, 2, "the conversion preserves the total heart count at 2");

    /* heart01_conversion_preserves_exact_total: the 3-heart print, whose
     * colours must all collapse onto heart01 with the total unchanged. */
    static TestGame tg5;
    test_game_new(&tg5);
    int live5 = mid(&tg5, "PL!HS-bp5-021-L");
    int member5 = mid(&tg5, "PL!HS-sd1-001-SD");
    int filler5 = mid(&tg5, "PL!N-sd1-010-SD");
    if (!ident(&tg5, live5, "PL!HS-bp5-021-L")) return;
    if (!ident(&tg5, member5, "PL!HS-sd1-001-SD")) return;
    if (filler5 < 0) return;
    test_give_energy(&tg5, 10);
    fill_decks(&tg5, filler5, 30);
    test_add_to_hand(&tg5, member5);
    test_play_to_stage(&tg5, member5, 1);
    hl_drain_skip(&tg5);
    test_add_to_hand(&tg5, live5);
    hl_advance_to_live_set(&tg5);
    test_set_live_card(&tg5, 0, live5);
    hl_advance_to_live_start(&tg5);
    hl_drain_first(&tg5);
    int agg5[8];
    rb_calc_stage_hearts(&tg5.state, 0, agg5);
    int total5 = 0;
    for (int z = 0; z < 8; z++) total5 += agg5[z];
    CHECK_EQ(agg5[RB_HEART_RED], 3, "all 3 hearts become heart01 after the transform");
    CHECK_EQ(agg5[RB_HEART_BLUE], 0, "the original heart04 slot is 0 after the transform");
    CHECK_EQ(total5, 3, "the total heart count is unchanged at 3");

    /* two eligible members -> exactly ONE is converted (「1人」)
     *
     * The board is built with set_stage3 rather than two test_play_to_stage
     * calls. The Rust test plays a 9-cost member to LeftSide and a 7-cost
     * member to Centre; the C placement rule bounces the 9-cost member out
     * entirely, so the left slot ends up empty and only one member survives --
     * that is a PLACEMENT question, not a heart-conversion one, and letting
     * it leak in would make this test silently measure nothing. */
    static TestGame tg2;
    test_game_new(&tg2);
    int live2 = mid(&tg2, "PL!HS-bp5-021-L");
    int a = mid(&tg2, "PL!HS-sd1-001-SD");
    int b = mid(&tg2, "PL!HS-sd1-003-SD");
    int filler2 = mid(&tg2, "PL!N-sd1-010-SD");
    if (!ident(&tg2, live2, "PL!HS-bp5-021-L")) return;
    if (!ident(&tg2, a, "PL!HS-sd1-001-SD")) return;
    if (!ident(&tg2, b, "PL!HS-sd1-003-SD")) return;
    if (filler2 < 0) return;
    test_give_energy(&tg2, 20);
    fill_decks(&tg2, filler2, 30);
    set_stage3(&tg2, 0, a, b, RB_EMPTY_SLOT);
    test_add_to_hand(&tg2, live2);
    hl_advance_to_live_set(&tg2);
    test_set_live_card(&tg2, 0, live2);
    hl_advance_to_live_start(&tg2);
    /* Rust `drain_all` answers select_indices(&[0]) = the FIRST selectable
     * candidate. The claim is therefore "the member that was converted is the
     * first filtered index of the stage prompt", which pins the mapping
     * instead of only pinning that somebody was converted. */
    const RbChoice *cs = rb_get_pending_choice(&tg2.state);
    int stage_prompt = cs && cs->kind == RB_CHOICE_SELECT_CARD && cs->zone[0] &&
                       !strcmp(cs->zone, "stage");
    int first_slot = -1;
    if (stage_prompt && cs->n_filtered_indices > 0) first_slot = cs->filtered_indices[0];
    CHECK(stage_prompt, "two eligible members must raise a stage selection prompt");
    hl_drain_first(&tg2);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg2.state.mods, a), RB_HEART_RED,
             stage_prompt
                 ? "the FIRST selectable member is the one converted"
                 : "exactly one member is converted");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg2.state.mods, b), -1,
             "the unselected member is NOT converted");
    if (stage_prompt) {
        int converted = (rb_mods_get_heart_color_multiplier(&tg2.state.mods, a) != -1)
                            ? a : b;
        CHECK_EQ(converted == tg2.state.p[0].stage[first_slot] ? 1 : 0, 1,
                 "the converted member is the first filtered index offered");
    }
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg2.state.mods, live2), -1,
             "the live card is NOT converted");

    /* three eligible members, pick index 1 -> the CENTRE member is converted
     * (Rust three_eligible_members_only_picked_center_converted). Built with
     * set_stage3 for the same placement reason as above. */
    static TestGame tg6;
    test_game_new(&tg6);
    int live6 = mid(&tg6, "PL!HS-bp5-021-L");
    int left6 = mid(&tg6, "PL!HS-sd1-003-SD");
    int centre6 = mid(&tg6, "PL!HS-sd1-003-SD");
    int right6 = mid(&tg6, "PL!HS-sd1-003-SD");
    int filler6 = mid(&tg6, "PL!N-sd1-010-SD");
    if (!ident(&tg6, live6, "PL!HS-bp5-021-L")) return;
    if (filler6 < 0) return;
    test_give_energy(&tg6, 30);
    fill_decks(&tg6, filler6, 30);
    set_stage3(&tg6, 0, left6, centre6, right6);
    test_add_to_hand(&tg6, live6);
    hl_advance_to_live_set(&tg6);
    test_set_live_card(&tg6, 0, live6);
    hl_advance_to_live_start(&tg6);
    /* Rust: drain until a stage SelectCard appears, then select_indices(&[1])
     * and break. */
    int picked = 0;
    int guard6 = 0;
    while (test_has_pending_choice(&tg6) && guard6++ < 24) {
        const RbChoice *c = rb_get_pending_choice(&tg6.state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD && c->zone[0] &&
            !strcmp(c->zone, "stage")) {
            rb_resume_with_choice(&tg6.state, 1);   /* filtered index 1 */
            picked = 1;
            break;
        }
        rb_resume_with_choice(&tg6.state, -1);
    }
    CHECK(picked, "three eligible members raise a stage SelectCard");
    hl_drain_skip(&tg6);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg6.state.mods, centre6), RB_HEART_RED,
             "the CENTRE member (picked index 1) is converted");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg6.state.mods, left6), -1,
             "the left member is NOT converted");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg6.state.mods, right6), -1,
             "the right member is NOT converted");

    /* no 蓮ノ空 member at all -> nothing is converted */
    static TestGame tg3;
    test_game_new(&tg3);
    int live3 = mid(&tg3, "PL!HS-bp5-021-L");
    int filler_member = mid(&tg3, "PL!-sd1-007-SD");
    int filler3 = mid(&tg3, "PL!N-sd1-010-SD");
    if (!ident(&tg3, live3, "PL!HS-bp5-021-L")) return;
    if (!ident(&tg3, filler_member, "PL!-sd1-007-SD")) return;
    if (filler3 < 0) return;
    test_give_energy(&tg3, 10);
    fill_decks(&tg3, filler3, 30);
    test_add_to_hand(&tg3, filler_member);
    test_play_to_stage(&tg3, filler_member, 1);
    hl_drain_skip(&tg3);
    test_add_to_hand(&tg3, live3);
    hl_advance_to_live_set(&tg3);
    test_set_live_card(&tg3, 0, live3);
    hl_advance_to_live_start(&tg3);
    hl_drain_first(&tg3);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg3.state.mods, filler_member), -1,
             "no 蓮ノ空 member on stage -> no conversion at all");

    /* one eligible + one ineligible member: only 1 eligible, so it is
     * auto-selected and no stage prompt is raised. */
    static TestGame tg4;
    test_game_new(&tg4);
    int live4 = mid(&tg4, "PL!HS-bp5-021-L");
    int hasuno = mid(&tg4, "PL!HS-sd1-003-SD");
    int non_hasuno = mid(&tg4, "PL!N-sd1-010-SD");
    int filler4 = mid(&tg4, "PL!N-sd1-010-SD");
    if (!ident(&tg4, live4, "PL!HS-bp5-021-L")) return;
    if (!ident(&tg4, hasuno, "PL!HS-sd1-003-SD")) return;
    if (filler4 < 0) return;
    test_give_energy(&tg4, 20);
    fill_decks(&tg4, filler4, 30);
    test_add_to_hand(&tg4, hasuno);
    test_add_to_hand(&tg4, non_hasuno);
    test_play_to_stage(&tg4, hasuno, 1);
    hl_drain_skip(&tg4);
    test_play_to_stage(&tg4, non_hasuno, 0);
    hl_drain_skip(&tg4);
    test_add_to_hand(&tg4, live4);
    hl_advance_to_live_set(&tg4);
    test_set_live_card(&tg4, 0, live4);
    hl_advance_to_live_start(&tg4);
    int saw_stage_choice = 0;
    int guard = 0;
    while (test_has_pending_choice(&tg4) && guard++ < 24) {
        const RbChoice *c = rb_get_pending_choice(&tg4.state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD && c->zone[0] &&
            !strcmp(c->zone, "stage")) {
            saw_stage_choice = 1;
            rb_resume_with_choice(&tg4.state, 0);
        } else if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            rb_resume_with_choice(&tg4.state, 0);
        } else {
            rb_resume_with_choice(&tg4.state, 0);
        }
    }
    CHECK(!saw_stage_choice,
          "with only 1 eligible member the target is auto-selected (no stage prompt)");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg4.state.mods, hasuno), RB_HEART_RED,
             "the 蓮ノ空 member is converted");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg4.state.mods, non_hasuno), -1,
             "the non-蓮ノ空 member is NOT converted");
}

/* required_hearts_twelve_gain_two_all_hearts_expires_at_live_end
 * (from the Riko Rust file): the 「all」 heart modifier is ライブ終了時, so
 * the turn walk must CLEAR it after the live ends. This is the only test in
 * the cluster that pins the expiry, and it is the reason a live-start heart
 * gain must not be implemented as a constant. */
static void b_riko_bp6_002_sec_all_hearts_expires_at_live_end(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int riko = mid(&tg, "PL!S-bp6-002-SEC");
    int filler = mid(&tg, "PL!N-sd1-010-SD");
    int mw = mid(&tg, "PL!S-bp3-019-L");
    if (!ident(&tg, riko, "PL!S-bp6-002-SEC")) return;
    if (!ident(&tg, mw, "PL!S-bp3-019-L")) return;
    if (filler < 0) return;
    clear_decks(&tg);
    fill_decks(&tg, filler, 20);
    set_stage3(&tg, 0, RB_EMPTY_SLOT, riko, RB_EMPTY_SLOT);
    test_give_energy(&tg, 20);
    int hand[2] = { mw, filler };
    hl_walk_to_live_start(&tg, hand, 2, mw, 0);
    hl_drain_skip(&tg);

    CHECK_EQ(heart_of(&tg, riko, HALL), 2,
             "during the live: +2 all-heart (MIRACLE WAVE total 12 >= 12)");

    /* Rust: performance P1, performance P2, LiveVictoryDetermination, then one
     * more pass into the next phase where check_expired_effects runs. */
    for (int i = 0; i < 3; i++) { test_pass(&tg); hl_drain_skip(&tg); }
    test_pass(&tg);
    hl_drain_skip(&tg);

    CHECK_EQ(heart_of(&tg, riko, HALL), 0,
             "the all-heart must be cleared after LiveVictoryDetermination "
             "(duration = live_end)");
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    if (getenv("HSLS_DEBUG")) rb_ability_debug_set(1);

    if (getenv("HSLS_DUMP")) {
        int rc = run_dump(getenv("HSLS_DUMP"));
        rb_unload();
        return rc;
    }

    /* HSLS_ONLY=<name> runs a single group, so a red check can be isolated
     * without the other 25 tests' output interleaving with the engine's
     * verdict log. Diagnostic only; unset means the whole suite. */
    const char *only = getenv("HSLS_ONLY");
    struct { const char *name; void (*fn)(void); } suite[] = {
        { "area_moved",            a_area_moved_member_pl_sp_bp5_024_l },
        { "area_moved_opp",        a_area_moved_opponent_does_not_qualify },
        { "bp6_010",               a_bp6_010_n_required_heart02 },
        { "dia_heart04",           a_dia_bp5_013_n_heart04_threshold },
        { "mari_heart05",          a_mari_bp5_017_n_heart05_threshold },
        { "riko_all",              a_riko_bp6_002_sec_all_hearts_twelve },
        { "riko_twelve",           a_riko_exact_twelve },
        { "bp7_024",               a_bp7_024_l_lone_aqours_hearts_become_heart04 },
        { "bp4_013",               a_bp4_013_n_paid_discard_other_member },
        { "hs_pr_029",             a_hs_pr_029_pr_paid_energy },
        { "cooking_cond",          a_cooking_bp7_028_l_condition_branches },
        { "cooking_discard_scope", a_cooking_bp7_028_l_condition_scopes_to_discard },
        { "cooking_accept",        a_cooking_bp7_028_l_accept_and_decline },
        { "fire_bird",             a_fire_bird_four_blade_group_heart02 },
        { "himeno_pair",           b_himeno_bp5_006_same_group_pair },
        { "himeno_skip",           b_himeno_bp5_006_skip_cost },
        { "himeno_waitroom",       b_himeno_bp5_006_cost_moves_cards_to_waitroom },
        { "himeno_area",           b_himeno_bp5_006_stage_area_irrelevant },
        { "himeno_autoskip",       b_himeno_bp5_006_cost_auto_skips },
        { "himeno_empty",          b_himeno_bp5_006_empty_hand_skips },
        { "himeno_variants",       b_himeno_bp5_006_rarity_variants },
        { "rurino_group",          b_rurino_bp5_003_discard_group_filter },
        { "rurino_crossunit",      b_rurino_bp5_003_cross_unit_same_group },
        { "himeno_handpair",       b_himeno_bp5_006_hand_pair_filter },
        { "bp6_001",               b_bp6_001_r_discard_distinct_heart_colors },
        { "umi",                   b_umi_bp3_013_n_per_success_card },
        { "setsuna",               b_setsuna_bp5_007_equal_success_counts },
        { "hs_bp5_021",            b_hs_bp5_021_l_convert_selected_member },
        { "riko_expire",           b_riko_bp6_002_sec_all_hearts_expires_at_live_end },
    };
    for (unsigned i = 0; i < sizeof suite / sizeof suite[0]; i++) {
        if (only && strcmp(only, suite[i].name) != 0) continue;
        suite[i].fn();
    }

    rb_unload();
    printf("\n%d assertions, %d failures\n", assertions, failures);
    if (failures) return 1;
    printf("ALL HEARTS LIVE-START PARITY CHECKS PASSED\n");
    return 0;
}

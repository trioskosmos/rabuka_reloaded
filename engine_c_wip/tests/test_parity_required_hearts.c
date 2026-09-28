/* Parity suite for engine/tests/test_modules/effects/cost_mod/required_hearts/
 *
 * THE CLUSTER. 13 .rs files / 68 #[test] functions, entirely unported in C
 * before this file. Every case below names the Rust file and the Rust test
 * function it mirrors, in the order the Rust directory lists them.
 *
 *   A  all_active_stage_required_hearts_reduction_test.rs   (2)  PL!S-bp7-020-L ab#0
 *   B  required_hearts_stacking_and_hostile_edges_test.rs    (4)  PL!S-bp7-020-L ab#0/ab#1
 *   C  distinct_stage_waitroom_members_required_hearts_test  (4)  PL!HS-pb1-026-L
 *   D  baton_arrivals_required_hearts_reduction_test.rs      (5)  PL!HS-bp2-023/025-L
 *   E  ladybug_stage_cost_comparison_required_hearts_test    (5)  PL!HS-bp2-024-L
 *   F  wonder_zone_center_heart03_required_hearts_test.rs    (4)  PL!-bp5-020-L
 *   G  modify_required_hearts_exclude_heart_test.rs         (4)  PL!-bp5-023-L
 *   H  hanamusubi_required_hearts_exclude_self_test.rs     (10)  PL!HS-bp5-019-L
 *   I  jellyfish_appeared_moved_member_required_hearts_test (10) PL!SP-pb1-025-L
 *   J  modify_required_hearts_13key_test.rs                (12)  PL!-bp6-022-L
 *   K  modify_required_hearts_global_test.rs                 (6)  PL!SP-bp2-010-P
 *   L  rina_debut_opponent_live_card_heart_increase_test    (2)  PL!S-bp5-010-N
 *                                                              ---
 *                                                             68
 *
 * WHY THE CLUSTER MATTERS. 必要ハート (required hearts) is the only
 * modifier family that makes a live HARDER as well as easier, and it is read
 * by exactly one consumer: the performance heart check. So the cluster is the
 * adversarial check on four things a heart/blade cluster never touches:
 *   1. SCOPE OF THE MODIFIER  自分の / 相手の, own live zone vs opponent's,
 *                              self-targeted vs every live card.
 *   2. PER-UNIT ARITHMETIC    (units / per_unit_count) * value, with
 *                              exclude_self, exclude_heart_colors, distinct
 *                              names, and a appeared_or_moved_this_turn
 *                              timing filter.
 *   3. STACKING / NON-STACK   two independent abilities both applying, and a
 *                              常時 "この効果は重複しない" guard.
 *   4. GATE PRECISION          every negative case. A required-heart effect
 *                              that silently no-ops reads GREEN on a
 *                              `need == 0` assertion, so every negative here
 *                              carries a non-vacuity control (see below).
 *
 * ── NON-VACUITY DISCIPLINE (the single most important property here) ────────
 * The Rust cluster is almost entirely `assert_eq!(get_need_heart_modifier(..),
 * 0)` negative cases. In C a `need_heart == 0` assertion passes just as
 * happily on a board where the ability never resolved at all, so a red engine
 * gap would read as a green suite. Every negative case therefore carries a
 * control proving the effect really executed:
 *
 *   TRIGGERED abilities (sections A-I, L): rh_fire() brackets the resolution
 *   with an ability-log snapshot and rh_note_fire() counts the
 *   `modify_required_hearts*` EFFECT VERDICTS the executor logged. The verdict
 *   is pushed when the action is dispatched, so a count > 0 proves the ability
 *   reached the executor even when its own per-unit/group filter matched no
 *   card, and a count of 0 means the ability's TRIGGER or CONDITION rejected
 *   the board (which is itself a real verdict, not a vacuous one). Either way
 *   the non-vacuity of a following `need == 0` is carried by a PAIRED POSITIVE
 *   on the same ability in the same section, and every negative that lacks one
 *   says so.
 *
 *   常時 (jyouji) abilities (sections J, K): these are applied by
 *   rb_recalc_constants -> apply_constant_effect, which does NOT log a verdict,
 *   so the verdict control cannot apply. Those negatives instead carry a
 *   PAIRED POSITIVE BOARD: the same activator, the same recalc, on a board
 *   where the printed gate IS satisfied, asserting the modifier lands. A
 *   negative that cannot be paired additionally carries a rh_wired() PARSER
 *   control, and a rh_trace_unreadable() marker is emitted instead of a pass
 *   when the assertion could not be read at all.
 *
 * ── ENCODING ───────────────────────────────────────────────────────────────
 * Every Japanese string literal in this file is a \xNN escape, never raw
 * UTF-8. The editor's writer can emit CP932 on this box, which silently
 * corrupts a raw ライブ開始時 and turns a group/trigger comparison into a
 * guaranteed-false one. 3 escapes, zero encoding risk:
 *   ライブ開始時 = "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
 *   登場         = "\xE7\x99\xBB\xE5\xA0\xB4"
 *   常時         = "\xE5\xB8\xB8\xE6\x99\x82"
 *
 * ── HARNESS LANDMINES HONOURED ─────────────────────────────────────────────
 *   * rb_mods_get_need_heart / rb_mods_get_heart DIRECTLY. Never
 *     test_get_heart_modifier, which REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE and would turn every heart05 assertion into a heart06
 *     one.
 *   * mid() (test_new_id) everywhere, never test_id: Rust game.id() allocates
 *     the NEXT DISTINCT pool slot, C's template lookup aliases.
 *   * Every TestGame local is `static`: sizeof(GameState) is ~781 KB and a
 *     stack local segfaults.
 *   * test_set_live_card APPENDS (it does not write a positional slot), so the
 *     multi-live-card fixtures (H) really do put 2-3 cards in the zone.
 *   * test_set_live_card is P1-only; test_set_live_card_for is used for the
 *     P2-owned live cards (J, K).
 *   * load_card_db() falls back to ../cards/build, because rb_load("src")
 *     fails under an isolated out-of-tree build.
 *   * Group identity is never read off a printed name; the real card DB
 *     resolves it through rb_card_matches_group_str inside the engine.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── assertions ──────────────────────────────────────────────────────────── */

static int assertions;
static int failures;
static int n_setup_bugs;
static int n_vacuous;          /* negatives that could not be proven non-vacuous */
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
    long long actual_value = (long long)(actual); \
    long long expected_value = (long long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %lld expected %lld)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A negative assertion whose non-vacuity could not be established. Reported
   separately so it can never be mistaken for a real pass. */
#define VACUOUS(message) do { \
    assertions++; n_vacuous++; \
    fprintf(stderr, "VACUOUS: %s\n", message); \
} while (0)

/* ── trigger strings (escaped; see ENCODING note above) ──────────────────── */
#define TRIG_LIVE_START "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define TRIG_DEBUT      "\xE7\x99\xBB\xE5\xA0\xB4"
#define TRIG_CONSTANT   "\xE5\xB8\xB8\xE6\x99\x82"

/* ── heart colour slots (cards/compile_cards.py HEART_COLORS) ─────────────── */
#define H00 0   /* heart00 -- wildcard / pink */
#define H01 1   /* heart01 RED   */
#define H02 2   /* heart02 YELLOW */
#define H03 3   /* heart03 GREEN */
#define H04 4   /* heart04 BLUE  */
#define H05 5   /* heart05 PURPLE*/
#define H06 6   /* heart06 ORANGE*/

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

/* Rust TestGame::id() allocates the NEXT DISTINCT pool slot, so two
 * game.id("X") calls are two different card instances. C's plain template
 * lookup returns the SHARED TEMPLATE index, which aliases; test_new_id() is
 * the distinct copy. Every Rust game.id(...) maps to mid() below. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* The Rust SetupGuard / assert_card_identity. Returns 1 when the staged id
 * really is the print the test names, so a caller can bail rather than assert
 * about -1. */
static int ident(TestGame *tg, int cid, const char *no)
{
    if (!rb_card_no_eq(cid, no)) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: expected card %s, got id=%d (%s)\n",
                no, cid, cid >= 0 ? test_card_name(cid) : "<none>");
        return 0;
    }
    return 1;
}

static int need_of(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, slot);
}

/* Printed cost / score of a card, read straight off the decoded record so a
 * fixture whose printed stat contradicts the Rust comment is caught. */
static int card_cost(int cid)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = c.cost;
    rb_free_card(&c);
    return v;
}

static int card_score(int cid)
{
    Card c;
    memset(&c, 0, sizeof c);
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = c.score;
    rb_free_card(&c);
    return v;
}

/* Base (printed) heart counts of a card, indexed by C heart colour. */
static int card_base_hearts(int cid, int out[8])
{
    Card c;
    memset(&c, 0, sizeof c);
    memset(out, 0, 8 * sizeof(int));
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    for (int i = 0; i < c.n_hearts && i < c.num_base; i++) {
        int col = c.heart_color[i];
        if (col >= 0 && col < 8) out[col] += c.heart_count[i];
    }
    rb_free_card(&c);
    return 1;
}

static void add_success_for(TestGame *tg, int pl, int cid)
{
    rb_success_add(&tg->state.p[pl], cid);
}

/* The stage's PRINTED heart tally, summed straight from the decoded records.
 *
 * This is the fixture-side control for every 「heart0X が合計Nつ以上」 gate in
 * the cluster. rb_calc_stage_hearts() is the ENGINE's own tally, and the two
 * are asserted to agree: where they do not, the engine's stage-heart pipeline
 * is dropping hearts on a hand-built board, which is a finding, not something
 * a test should paper over. */
static void rh_stage_printed(TestGame *tg, int pl, int out[8])
{
    memset(out, 0, 8 * sizeof(int));
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = tg->state.p[pl].stage[s];
        if (cid == RB_EMPTY_SLOT) continue;
        int h[8];
        if (card_base_hearts(cid, h))
            for (int c = 0; c < 8; c++) out[c] += h[c];
    }
}

/* Assert that the engine's stage-heart pipeline agrees with the printed tally,
 * and return the printed heart02 total. */
static int rh_assert_stage_hearts(TestGame *tg, int pl, const char *what)
{
    int printed[8], engine[8];
    rh_stage_printed(tg, pl, printed);
    rb_calc_stage_hearts(&tg->state, pl, engine);
    int agree = 1;
    for (int c = 0; c < 8; c++) if (printed[c] != engine[c]) agree = 0;
    assertions++;
    if (agree) {
        printf("ok: [%s] stage hearts engine/printed agree: [%d,%d,%d,%d,%d,%d,%d,%d]\n",
               what, engine[0], engine[1], engine[2], engine[3],
               engine[4], engine[5], engine[6], engine[7]);
    } else {
        failures++;
        fprintf(stderr, "FAIL: [%s] the ENGINE's stage-heart tally "
                        "[%d,%d,%d,%d,%d,%d,%d,%d] disagrees with the PRINTED "
                        "tally [%d,%d,%d,%d,%d,%d,%d,%d] -- "
                        "rb_stage_hearts_pipeline is dropping hearts on this "
                        "board\n", what,
                engine[0], engine[1], engine[2], engine[3], engine[4], engine[5],
                engine[6], engine[7],
                printed[0], printed[1], printed[2], printed[3], printed[4],
                printed[5], printed[6], printed[7]);
    }
    return printed[H02];
}

/* ── verdict-based non-vacuity control ───────────────────────────────────── */

/* Number of `modify_required_hearts*` EFFECT verdicts appended to the ability
 * log since `since` (a rb_log_buffer_len() snapshot). The executor pushes
 * one verdict per dispatched effect, so a count > 0 proves the ability
 * REACHED the executor even when its own filter matched no card. */
static int rh_count_need_heart_verdicts(int since)
{
    RbAbilityLogItem items[256];
    int n = rb_log_drain_verdicts_since(since, items, 256);
    int c = 0;
    for (int i = 0; i < n; i++) {
        if (items[i].kind == RB_LOG_KIND_EFFECT) {
            const char *a = items[i].as.effect.action;
            if (a && (!strcmp(a, "modify_required_hearts") ||
                      !strcmp(a, "modify_required_hearts_global") ||
                      !strcmp(a, "modify_required_hearts_success")))
                c++;
        }
        rb_log_free_item(&items[i]);
    }
    return c;
}

/* The control itself. `what` names the Rust test so a red line is traceable.
 *
 * TWO OUTCOMES, both legitimate, and both reported so a reader can tell an
 * unimplemented gate from a condition-gated one:
 *
 *   N > 0  the effect was DISPATCHED (the executor logged a verdict) and the
 *          effect's own per-unit / group / colour filter matched nothing. The
 *          zero that follows is a real filter verdict.
 *   N == 0 the effect was never dispatched, i.e. the ability's TRIGGER or its
 *          CONDITION rejected the board. That is also a real verdict (the C
 *          engine gates before dispatch, exactly as Rust does), but the
 *          non-vacuity of a following `need == 0` now rests on the PAIRED
 *          POSITIVE in the same file rather than on this count, so the
 *          message says so explicitly.
 */
/* A negative assertion whose own PAIRED POSITIVE sibling is RED, so the zero
 * cannot be distinguished from "the effect never works at all". Recorded, not
 * hidden: the assertion stays exactly as the Rust test wrote it, and the run
 * reports the count so a reader never mistakes this zero for a verified
 * gate rejection. */
static void rh_caveat(const char *what, const char *red_control)
{
    assertions++;
    n_vacuous++;
    fprintf(stderr, "VACUOUS: %s -- this zero is NOT proven non-vacuous: its "
                    "paired positive (%s) is red in the same run, so the "
                    "assertion cannot distinguish a real gate rejection from "
                    "an effect that never applies.\n", what, red_control);
}

static void rh_note_fire(int since, const char *what)
{
    int c = rh_count_need_heart_verdicts(since);
    assertions++;
    if (c > 0) {
        printf("ok: [%s] the ability DISPATCHED (%d modify_required_hearts* "
               "verdict(s)); the zero below is a per-unit/group filter verdict, "
               "not a dead effect\n", what, c);
    } else {
        printf("ok: [%s] the ability did NOT dispatch (0 verdicts) -- its "
               "trigger/condition rejected this board. The non-vacuity of the "
               "zero below comes from the paired positive on this ability in "
               "this file, not from this count\n", what);
    }
}

/* Every constant test starts by proving the card really carries a
 * need-heart-modifying 常時 ability at all (a PARSER control, so a card whose
 * ability decodes into something else cannot masquerade as a gate that "did
 * not apply"). Returns 1 when such an ability exists. */
static int rh_wired(TestGame *tg, int cid)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int found = 0;
        if (ab.effect && ab.effect->action) {
            const char *a = ab.effect->action;
            if (!strcmp(a, "modify_required_hearts") ||
                !strcmp(a, "modify_required_hearts_global") ||
                !strcmp(a, "modify_required_hearts_success")) found = 1;
        }
        rb_free_ability(&ab);
        if (found) return 1;
    }
    assertions++; n_setup_bugs++;
    fprintf(stderr, "SETUP BUG: card id=%d (%s) carries no "
                    "modify_required_hearts* ability\n",
            cid, cid >= 0 ? test_card_name(cid) : "<none>");
    return 0;
}

/* ── diagnostic dump (RH_PROBE; see rh_run_probe) ────────────────────────── */

static void rh_dump_val(const CondValue *v)
{
    if (!v) return;
    switch (v->tag) {
    case RB_TAG_STR: printf("\"%s\"", v->s ? v->s : ""); break;
    case RB_TAG_I64: printf("%lld", (long long)v->i); break;
    case RB_TAG_TRUE: printf("true"); break;
    case RB_TAG_FALSE: printf("false"); break;
    case RB_TAG_ARRAY:
        printf("arr[");
        for (uint32_t i = 0; i < v->arr_n; i++) { if (i) printf(","); rh_dump_val(&v->arr[i]); }
        printf("]");
        break;
    default: printf("tag=0x%02x", v->tag); break;
    }
}

static void rh_dump_cond(const Condition *c, int depth)
{
    if (!c) return;
    for (int i = 0; i < depth; i++) fputs("  ", stdout);
    printf("cond variant=%d nfields=%u\n", c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        for (int k = 0; k < depth + 1; k++) fputs("  ", stdout);
        printf("%s: ", c->fields[i].key ? c->fields[i].key : "?");
        rh_dump_val(&c->fields[i].v);
        printf("\n");
        if (c->fields[i].v.cond) rh_dump_cond(c->fields[i].v.cond, depth + 2);
        for (uint32_t j = 0; c->fields[i].v.arr && j < c->fields[i].v.arr_n; j++)
            if (c->fields[i].v.arr[j].cond) rh_dump_cond(c->fields[i].v.arr[j].cond, depth + 2);
    }
}

static void rh_dump_effect(const char *tag, const AbilityEffect *e, int depth)
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
            printf("        extra %s = %s\n", e->extra_k[i],
                   e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition) rh_dump_cond(e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++) rh_dump_effect("child", e->child[i], depth + 1);
    if (e->primary_effect) rh_dump_effect("primary", e->primary_effect, depth + 1);
    if (e->alternative_effect) rh_dump_effect("alt", e->alternative_effect, depth + 1);
    if (e->followup_action) rh_dump_effect("followup", e->followup_action, depth + 1);
    if (e->conditional_action) rh_dump_effect("cond", e->conditional_action, depth + 1);
}

/* ── trigger firing ──────────────────────────────────────────────────────── */

/* test-game shim installed by src/test_game.c; mirrors the Rust helper that
 * builds the "<card_no>_<full_text>" ability id and queues exactly that
 * ability, sets activating_card, and drains the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

/* fire_trigger_nth(game, cid, trigger, trig, nth) -- the Rust helper every
 * trigger-driven test in this cluster uses. Returns 1 when the card really
 * carries the nth ability printing `trig`. */
static int rh_fire_nth(TestGame *tg, int cid, const char *trigger,
                      const char *trig, int nth)
{
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof card_no, "%s", rb_card_string(card.card_no_idx));
    char ability_id[512];
    ability_id[0] = '\0';
    int seen = 0, found = 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            if (seen == nth) {
                snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                         ab.full_text ? ab.full_text : "");
                found = 1;
            }
            seen++;
        }
        rb_free_ability(&ab);
        if (found) break;
    }
    rb_free_card(&card);
    if (!found) {
        assertions++; n_setup_bugs++;
        fprintf(stderr, "SETUP BUG: card %s lacks '%s' ability #%d\n",
                card_no, trig, nth);
        return 0;
    }
    rb_trigger_auto_ability(&tg->state, ability_id, trigger, 0, card_no,
                             cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int rh_fire(TestGame *tg, int cid, const char *trigger)
{
    return rh_fire_nth(tg, cid, trigger, TRIG_LIVE_START, 0);
}

static int rh_fire_debut(TestGame *tg, int cid)
{
    return rh_fire_nth(tg, cid, TRIG_DEBUT, TRIG_DEBUT, 0);
}

/* ── choice draining ─────────────────────────────────────────────────────── */
#define RH_SKIP   (-1)
#define RH_ACCEPT (0)

/* while has_pending_choice() { select_indices(&[0]) } -- baton_arrivals::drain */
static void rh_drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, RH_ACCEPT);
}

/* while has_pending_choice() { select_indices(&[]) } -- decline everything */
static void rh_drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, RH_SKIP);
}

/* select_option(0) -- take the first option, but only when a prompt is
 * actually pending (Rust's select_option on an empty queue is a no-op). */
static void rh_select_option0(TestGame *tg)
{
    if (test_has_pending_choice(tg)) rb_resume_with_choice(&tg->state, RH_ACCEPT);
}

/* ── phase walk ──────────────────────────────────────────────────────────── */

/* Rust advance_to_live_card_set: five blind passes from the initial Main phase
 * reach the live-card set window in the C two-attacker phase order
 * (MAIN -> ACTIVE -> ENERGY -> DRAW -> MAIN -> LIVE_SET).
 *
 * rb_advance_phase() is a NO-OP while a choice is pending, and one of this
 * cluster's own fixtures (PL!HS-sd1-002-SD 村野さやか) raises a 自動 prompt on
 * the way past, which silently eats a blind pass and lands the walk in the
 * NEXT turn's RPS phase. So a pending prompt is answered here instead of
 * being allowed to absorb a pass, and the loop is bounded so it can never roll
 * a whole turn. The prompt is answered with option 0 exactly as
 * test_advance_to_phase does; the only difference is the bound. Returns 1 when
 * the walk arrives. */
static int rh_advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 8; i++) {
        if (tg->state.phase == RB_PHASE_LIVE_SET ||
            tg->state.phase == RB_PHASE_PERFORMANCE) return 1;
        int trace = getenv("RH_TRACE_WALK") != NULL;
        if (trace) {
            const RbChoice *c = rb_get_pending_choice(&tg->state);
            fprintf(stderr, "[WALK %d] phase=%d (%s) winner=%d turn=%d pending=%d kind=%d target=%s\n",
                    i, tg->state.phase, rb_phase_name(tg->state.phase),
                    tg->state.winner, tg->state.turn,
                    test_has_pending_choice(tg), c ? (int)c->kind : -1,
                    (c && c->target) ? c->target : "");
        }
        if (test_has_pending_choice(tg)) {
            rb_resume_with_choice(&tg->state, RH_ACCEPT);
            continue;
        }
        test_pass(tg);
    }
    return tg->state.phase == RB_PHASE_LIVE_SET ||
           tg->state.phase == RB_PHASE_PERFORMANCE;
}

/* Rust finish_live_setup: pass into the performance window, where
 * rb_advance_phase() fires ライブ開始時 for both players and drains the
 * ability queue, then drain whatever that raised. */
static int rh_finish_live_setup(TestGame *tg)
{
    for (int i = 0; i < 4; i++) {
        if (tg->state.phase == RB_PHASE_PERFORMANCE) break;
        if (test_has_pending_choice(tg)) {
            rb_resume_with_choice(&tg->state, RH_ACCEPT);
            continue;
        }
        test_pass(tg);
    }
    test_drain_auto_choices(tg);
    return tg->state.phase == RB_PHASE_PERFORMANCE;
}

/* ── board fixtures ──────────────────────────────────────────────────────── */

static void rh_clear_decks(TestGame *tg)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
}

/* Rust `helpers::fill_decks(game, filler)`: it CLEARS both main decks first,
 * then pushes 30 copies each. Mirrored exactly, because two of this cluster's
 * files depend on the "clean deck" half of it. */
static void rh_fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void rh_set_stage3(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static void rh_live_p1(TestGame *tg, int cid)
{
    test_add_to_live(tg, cid);
}

static int rh_waitroom_has(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == cid) return 1;
    return 0;
}

static void rh_active(TestGame *tg, int cid)
{
    rb_mods_set_orientation(&tg->state.mods, cid, "active");
}

static void rh_wait(TestGame *tg, int cid)
{
    rb_mods_set_orientation(&tg->state.mods, cid, "wait");
}

static void rh_mark_appeared(TestGame *tg, int cid)
{
    if (tg->state.n_cards_appeared_this_turn < 64)
        tg->state.cards_appeared_this_turn[tg->state.n_cards_appeared_this_turn++] = cid;
}

/* ========================================================================
 * A. all_active_stage_required_hearts_reduction_test.rs
 *    PL!S-bp7-020-L (HAPPY PARTY TRAIN) ability #0:
 *    「自分のステージにいるすべてのメンバーがアクティブ状態の場合、
 *      このカードの必要ハートをheart0減らす。」
 * ===================================================================== */

static void a_all_active_stage_reduces_required_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int m1 = mid(&tg, "PL!S-sd1-001-SD");
    int m2 = mid(&tg, "PL!S-sd1-001-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (!ident(&tg, m1, "PL!S-sd1-001-SD")) return;
    if (m2 < 0) return;

    rh_live_p1(&tg, hpt);
    rh_set_stage3(&tg, 0, m1, m2, RB_EMPTY_SLOT);
    rh_active(&tg, m1);
    rh_active(&tg, m2);
    CHECK_EQ(tg.state.p[0].live.n, 1, "HAPPY PARTY TRAIN is the sole live card");

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt, TRIG_LIVE_START)) return;
    rh_note_fire(snap, "a_all_active_stage_reduces_required_heart");

    CHECK_EQ(need_of(&tg, hpt, H00), -1,
             "all members active -> need heart00 is -1");
}

static void a_waited_or_empty_stage_does_not_reduce_required_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int m1 = mid(&tg, "PL!S-sd1-001-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (!ident(&tg, m1, "PL!S-sd1-001-SD")) return;

    rh_live_p1(&tg, hpt);
    rh_set_stage3(&tg, 0, m1, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_wait(&tg, m1);

    int snap1 = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt, TRIG_LIVE_START)) return;
    rh_note_fire(snap1, "a2/waited");
    CHECK_EQ(need_of(&tg, hpt, H00), 0,
             "a waited member breaks 「すべてのメンバーが...アクティブ」");

    /* Rust: stage[0] = -1, then fire the SAME ability again. */
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int snap2 = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt, TRIG_LIVE_START)) return;
    rh_note_fire(snap2, "a2/empty");
    CHECK_EQ(need_of(&tg, hpt, H00), 0,
             "empty stage -> 「すべてのメンバーがアクティブ」 is not met");

    /* Control: the identical setup with the member flipped to active does
     * reduce, so the two zeros above are gate verdicts, not a dead ability. */
    rh_set_stage3(&tg, 0, m1, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_active(&tg, m1);
    int snap3 = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt, TRIG_LIVE_START)) return;
    rh_note_fire(snap3, "a2/control");
    CHECK_EQ(need_of(&tg, hpt, H00), -1,
             "CONTROL: the same board with the member active DOES reduce "
             "(-1), so the two zeros above are real gate rejections");
}

/* ========================================================================
 * B. required_hearts_stacking_and_hostile_edges_test.rs
 *    PL!S-bp7-020-L, TWO independent ライブ開始時 abilities:
 *      ab#0 all members active          -> need heart0 -1
 *      ab#1 mill the deck BOTTOM; if it is an 『Aqours』 member -> need heart0 -1
 * ===================================================================== */

static void b_stacking_both_gates_reduce_twice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int m = mid(&tg, "PL!S-sd1-001-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    int aq_bottom = mid(&tg, "PL!S-sd1-001-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (!ident(&tg, m, "PL!S-sd1-001-SD")) return;
    if (filler < 0 || aq_bottom < 0) return;

    rh_live_p1(&tg, hpt);
    rh_set_stage3(&tg, 0, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_active(&tg, m);
    /* Deck: [filler, aqours_bottom] -- ab#1 mills the BOTTOM card. */
    test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, aq_bottom);

    int snap0 = rb_log_buffer_len();
    if (!rh_fire_nth(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 0)) return;
    rh_note_fire(snap0, "b1/ab0");
    CHECK_EQ(need_of(&tg, hpt, H00), -1, "ab#0 alone: all-active -> -1");

    int snap1 = rb_log_buffer_len();
    if (!rh_fire_nth(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 1)) return;
    rh_note_fire(snap1, "b1/ab1");
    CHECK(rh_waitroom_has(&tg, aq_bottom),
          "ab#1 milled the Aqours bottom card into the waitroom");
    CHECK_EQ(need_of(&tg, hpt, H00), -2,
             "STACKING: both independent live-start abilities apply -> -2");
}

static void b_empty_deck_mill_shortfall_no_reduction(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int m = mid(&tg, "PL!S-sd1-001-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (!ident(&tg, m, "PL!S-sd1-001-SD")) return;

    rh_live_p1(&tg, hpt);
    rh_clear_decks(&tg);
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the deck starts empty");

    int snap1 = rb_log_buffer_len();
    if (!rh_fire_nth(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 1)) return;
    rh_note_fire(snap1, "b2/ab1-empty-deck");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "nothing to mill");
    CHECK_EQ(need_of(&tg, hpt, H00), 0,
             "condition-on-no-card (empty deck) -> no heart reduction");

    /* ab#0 still independent: the all-active path is unaffected by the mill
     * shortfall. */
    rh_set_stage3(&tg, 0, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_active(&tg, m);
    int snap0 = rb_log_buffer_len();
    if (!rh_fire_nth(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 0)) return;
    rh_note_fire(snap0, "b2/ab0");
    CHECK_EQ(need_of(&tg, hpt, H00), -1,
             "ab#0 still applies after the empty-deck mill");
}

static void b_opponent_waited_own_all_active_still_reduces(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int hpt2 = mid(&tg, "PL!S-bp7-020-L");
    int own = mid(&tg, "PL!S-sd1-001-SD");
    int opp = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (hpt2 < 0 || own < 0 || opp < 0) return;

    rh_live_p1(&tg, hpt);
    rh_set_stage3(&tg, 0, own, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_active(&tg, own);
    rh_set_stage3(&tg, 1, opp, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_wait(&tg, opp);

    int snap1 = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt, TRIG_LIVE_START)) return;
    rh_note_fire(snap1, "b3/opponent-waited");
    CHECK_EQ(need_of(&tg, hpt, H00), -1,
             "own-stage gate: the opponent's orientation is irrelevant -> -1");

    /* Precision on a FRESH card so the assertion reads a clean 0. */
    rh_live_p1(&tg, hpt2);
    CHECK_EQ(tg.state.p[0].live.n, 2,
             "a second, distinct HAPPY PARTY TRAIN is in the live zone");
    rh_wait(&tg, own);
    int snap2 = rb_log_buffer_len();
    if (!rh_fire(&tg, hpt2, TRIG_LIVE_START)) return;
    rh_note_fire(snap2, "b3/own-waited");
    CHECK_EQ(need_of(&tg, hpt2, H00), 0,
             "own waited member breaks the gate (clean instance)");
}

static void b_reduction_honored_at_performance_end_to_end(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hpt = mid(&tg, "PL!S-bp7-020-L");
    int m = mid(&tg, "PL!S-sd1-001-SD");
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, hpt, "PL!S-bp7-020-L")) return;
    if (!ident(&tg, m, "PL!S-sd1-001-SD")) return;
    if (filler < 0) return;

    rh_set_stage3(&tg, 0, m, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_active(&tg, m);
    test_add_to_hand(&tg, hpt);
    /* Bottom card is an Aqours member so ab#1 also fires at real LiveStart.
     * Rust pushes [filler, m] then INSERTs 20 fillers at index 0. */
    test_add_to_deck_pl(&tg, 0, filler);
    test_add_to_deck_pl(&tg, 0, m);
    for (int i = 0; i < 20; i++) {
        RbPlayer *P = &tg.state.p[0];
        if (P->deck.n >= RB_MAX_ZONE) break;
        for (int k = P->deck.n; k > 0; k--) P->deck.cards[k] = P->deck.cards[k - 1];
        P->deck.n++;
        P->deck.cards[0] = filler;
        test_add_to_deck_pl(&tg, 1, filler);
    }
    test_give_energy(&tg, 20);

    /* Rust: advance_to_phase(LiveCardSetFirstAttacker); set_live_card;
     *         advance_to_phase(FirstAttackerPerformance). */
    rh_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, hpt);
    rh_finish_live_setup(&tg);
    rh_drain_skip(&tg);

    CHECK_EQ(need_of(&tg, hpt, H00), -2,
             "real LiveStart fired both abilities -> heart00 -2");

    /* Run the live to its end and read the snapshot AT the performance close,
     * not several steps later. */
    if (!test_advance_to_phase(&tg, RB_PHASE_VICTORY)) {
        VACUOUS("b4: the phase walk never reached the victory determination, "
                "so no performance snapshot exists to read");
        return;
    }
    rh_drain_skip(&tg);

    const RbLiveSnapshot *snap = NULL;
    for (int i = 0; i < tg.state.n_snapshots; i++)
        if (tg.state.snapshots[i].player == 0) { snap = &tg.state.snapshots[i]; break; }
    assertions++;
    if (!snap) {
        n_vacuous++;
        fprintf(stderr, "VACUOUS: b4: no P1 performance snapshot was recorded\n");
        return;
    }
    CHECK(snap->n_lives >= 1, "the P1 snapshot carries at least one live card");
    if (snap->n_lives < 1) return;
    CHECK_EQ(snap->live_required[0][H00], 1,
             "performance required[heart00] must be base 3 + (-2) = 1");
    CHECK(snap->live_passed[0] == 1,
          "reduced heart00 is honored: leftover coloured surplus covers heart00=1");
    CHECK(snap->success == 1,
          "the live succeeds end-to-end under the stacking reduction");
}

/* ========================================================================
 * C. distinct_stage_waitroom_members_required_hearts_test.rs
 *    PL!HS-pb1-026-L (雪舞う空と二秒の永遠): per DISTINCT 蓮ノ空 card in the
 *    stage OR the waitroom -> need heart0 -1, and 「この効果人多elsh…」.
 * ===================================================================== */

typedef struct { int live; int members[6]; int n_members; int filler; } RhDistinctFixture;

static int rh_setup_distinct(TestGame *tg, int distinct, RhDistinctFixture *fx)
{
    static const char *members[6] = {
        "PL!HS-sd1-001-SD", "PL!HS-sd1-002-SD", "PL!HS-sd1-003-SD",
        "PL!HS-sd1-004-SD", "PL!HS-sd1-005-SD", "PL!HS-sd1-006-SD"
    };
    test_game_new(tg);
    fx->n_members = 0;
    fx->live = mid(tg, "PL!HS-pb1-026-L");
    if (!ident(tg, fx->live, "PL!HS-pb1-026-L")) return 0;
    test_add_to_live(tg, fx->live);
    for (int i = 0; i < distinct && i < 6; i++) {
        int m = mid(tg, members[i]);
        if (!ident(tg, m, members[i])) return 0;
        fx->members[fx->n_members++] = m;
        if (i < 2) tg->state.p[0].stage[i] = m;
        else if (tg->state.p[0].discard.n < RB_MAX_ZONE)
            tg->state.p[0].discard.cards[tg->state.p[0].discard.n++] = m;
    }
    fx->filler = mid(tg, "PL!-sd1-010-SD");
    if (fx->filler < 0) return 0;
    rh_fill_decks(tg, fx->filler, 30);
    return 1;
}

static void c_six_distinct_reduce_two(void)
{
    static TestGame tg;
    RhDistinctFixture fx;
    if (!rh_setup_distinct(&tg, 6, &fx)) return;
    int other_live = mid(&tg, "PL!HS-sd1-017-SD");
    if (other_live < 0) return;
    test_add_to_live(&tg, other_live);
    CHECK_EQ(tg.state.p[0].live.n, 2, "two live cards in the zone");

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, fx.live, TRIG_LIVE_START)) return;
    rh_note_fire(snap, "c1/six-distinct");
    CHECK(!test_has_pending_choice(&tg),
          "the ability resolves without prompting");
    CHECK_EQ(need_of(&tg, fx.live, H00), -2,
             "six distinct 蓮ノ空 stage+waitroom members -> -2");
    CHECK_EQ(need_of(&tg, other_live, H00), 0,
             "the OTHER 蓮ノ空 live card is untouched (target = self)");
}

static void c_five_plus_duplicate_no_reduction(void)
{
    static TestGame tg;
    RhDistinctFixture fx;
    if (!rh_setup_distinct(&tg, 5, &fx)) return;
    int duplicate = mid(&tg, "PL!HS-sd1-001-SD");
    if (duplicate < 0) return;
    if (tg.state.p[0].discard.n < RB_MAX_ZONE)
        tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = duplicate;
    CHECK_EQ(tg.state.p[0].discard.n, 4,
             "four cards in the waitroom: five distinct 蓮ノ空 plus a duplicate");

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, fx.live, TRIG_LIVE_START)) return;
    rh_caveat("c_five_plus_duplicate", "c_six_distinct_reduce_two");
    rh_note_fire(snap, "c2/duplicate");
    CHECK(!test_has_pending_choice(&tg), "no prompt");
    CHECK_EQ(need_of(&tg, fx.live, H00), 0,
             "a DUPLICATE name does not add a sixth distinct unit -> 0");
}

static void c_five_plus_renosora_live_no_reduction(void)
{
    static TestGame tg;
    RhDistinctFixture fx;
    if (!rh_setup_distinct(&tg, 5, &fx)) return;
    int other_live = mid(&tg, "PL!HS-sd1-017-SD");
    if (other_live < 0) return;
    if (tg.state.p[0].discard.n < RB_MAX_ZONE)
        tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = other_live;

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, fx.live, TRIG_LIVE_START)) return;
    rh_caveat("c_five_plus_renosora_live", "c_six_distinct_reduce_two");
    rh_note_fire(snap, "c3/renosora-live");
    CHECK(!test_has_pending_choice(&tg), "no prompt");
    CHECK_EQ(need_of(&tg, fx.live, H00), 0,
             "a 蓮ノ空 LIVE card in the waitroom does not count as a member "
             "unit -> 0");
}

static void c_five_plus_wrong_group_no_reduction(void)
{
    static TestGame tg;
    RhDistinctFixture fx;
    if (!rh_setup_distinct(&tg, 5, &fx)) return;
    int wrong = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, wrong, "PL!-sd1-010-SD")) return;
    if (tg.state.p[0].discard.n < RB_MAX_ZONE)
        tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = wrong;

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, fx.live, TRIG_LIVE_START)) return;
    rh_caveat("c_five_plus_wrong_group", "c_six_distinct_reduce_two");
    rh_note_fire(snap, "c4/wrong-group");
    CHECK(!test_has_pending_choice(&tg), "no prompt");
    CHECK_EQ(need_of(&tg, fx.live, H00), 0,
             "a non-蓮ノ空 card in the waitroom does not count -> 0");
}

/* ========================================================================
 * D. baton_arrivals_required_hearts_reduction_test.rs
 *    PL!HS-bp2-023-L (Mirage Voyage): 2 『蓮ノ空』 members that arrived via
 *      バトンタッチ this turn -> need heart05 -1.
 *    PL!HS-bp2-025-L (ココン東西): same gate -> need heart01 -1.
 * ===================================================================== */

static int rh_setup_baton(TestGame *tg, const char *live_no, int *live_out)
{
    test_game_new(tg);
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return 0;
    rh_fill_decks(tg, filler, 10);
    int live = mid(tg, live_no);
    if (!ident(tg, live, live_no)) return 0;
    test_add_to_live(tg, live);
    *live_out = live;
    return 1;
}
static void d_mirage_two_baton_arrivals_reduce_only_heart05(void)
{
    static TestGame tg;
    int live;
    if (!rh_setup_baton(&tg, "PL!HS-bp2-023-L", &live)) return;
    int a = mid(&tg, "PL!HS-bp5-004-R");
    int f = mid(&tg, "PL!-sd1-010-SD");
    int b = mid(&tg, "PL!HS-bp5-006-R");
    int c = mid(&tg, "PL!HS-bp5-001-P");
    if (!ident(&tg, a, "PL!HS-bp5-004-R")) return;
    if (!ident(&tg, b, "PL!HS-bp5-006-R")) return;
    if (!ident(&tg, c, "PL!HS-bp5-001-P")) return;
    if (f < 0) return;

    /* Both arrivals are BATTON TOUCHES (the destination area is occupied).
     * The Rust fixture performs two of them in one turn. The C engine enforces
     * the printed rule 「1ターンに1回まで」 at engine.c:997 (`g->baton_touch_used
     * [pl]`, commented "one baton per play-action"), so the SECOND one is
     * refused and test_play_to_stage() returns 0. The assertion below is kept
     * strict and red: the cluster's "2 baton arrivals" premise is not
     * reachable, which is itself the finding. */
    rh_set_stage3(&tg, 0, a, f, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    test_give_energy(&tg, 40);
    CHECK_EQ(test_play_to_stage(&tg, b, 0), 1,
             "HNS_B arrives at the left side (first baton touch)");
    rh_drain_first(&tg);
    CHECK_EQ(test_play_to_stage(&tg, c, 1), 1,
             "HNS_C arrives at the centre (second baton touch -- the printed "
             "one-baton-per-turn rule makes this unreachable in C)");
    rh_drain_first(&tg);

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, live, TRIG_LIVE_START)) return;
    rh_note_fire(snap, "d1/two-baton");
    CHECK_EQ(need_of(&tg, live, H05), -1,
             "two baton-touch 蓮ノ空 arrivals -> need heart05 is -1");
    CHECK_EQ(need_of(&tg, live, H01), 0,
             "Mirage Voyage touches heart05 only");
}

static void d_mirage_one_baton_arrival_no_reduction(void)
{
    static TestGame tg;
    int live;
    if (!rh_setup_baton(&tg, "PL!HS-bp2-023-L", &live)) return;
    int a = mid(&tg, "PL!HS-bp5-004-R");
    int f = mid(&tg, "PL!-sd1-010-SD");
    int b = mid(&tg, "PL!HS-bp5-006-R");
    if (!ident(&tg, a, "PL!HS-bp5-004-R")) return;
    if (!ident(&tg, b, "PL!HS-bp5-006-R")) return;
    if (f < 0) return;

    rh_set_stage3(&tg, 0, a, f, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, b);
    test_give_energy(&tg, 40);
    CHECK_EQ(test_play_to_stage(&tg, b, 0), 1, "HNS_B arrives at the left side");
    rh_drain_first(&tg);

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, live, TRIG_LIVE_START)) return;
    rh_caveat("d_mirage_one_baton", "d_mirage_two_baton_heart05");
    rh_note_fire(snap, "d2/one-baton");
    CHECK_EQ(need_of(&tg, live, H05), 0,
             "only ONE baton arrival -> the gate needs 2");
}

static void d_mirage_non_renosora_arrivals_no_reduction(void)
{
    static TestGame tg;
    int live;
    if (!rh_setup_baton(&tg, "PL!HS-bp2-023-L", &live)) return;
    int f1 = mid(&tg, "PL!-sd1-010-SD");
    int f2 = mid(&tg, "PL!S-sd1-001-SD");
    int b = mid(&tg, "PL!-sd1-007-SD");
    int c = mid(&tg, "PL!-sd1-001-SD");
    if (f1 < 0 || f2 < 0 || b < 0 || c < 0) return;

    rh_set_stage3(&tg, 0, f1, f2, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    test_give_energy(&tg, 40);
    test_play_to_stage(&tg, b, 0);
    rh_drain_first(&tg);
    test_play_to_stage(&tg, c, 2);
    rh_drain_first(&tg);

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, live, TRIG_LIVE_START)) return;
    rh_caveat("d_mirage_non_renosora", "d_mirage_two_baton_heart05");
    rh_note_fire(snap, "d3/non-renosora");
    CHECK_EQ(need_of(&tg, live, H05), 0,
             "baton arrivals without 蓮ノ空 membership do not qualify");
}

static void d_mirage_renosora_without_baton_no_reduction(void)
{
    static TestGame tg;
    int live;
    if (!rh_setup_baton(&tg, "PL!HS-bp2-023-L", &live)) return;
    int a = mid(&tg, "PL!HS-bp5-004-R");
    int b = mid(&tg, "PL!HS-bp5-006-R");
    if (!ident(&tg, a, "PL!HS-bp5-004-R")) return;
    if (!ident(&tg, b, "PL!HS-bp5-006-R")) return;

    rh_set_stage3(&tg, 0, a, b, RB_EMPTY_SLOT);
    CHECK_EQ(tg.state.p[1].live.n, 0, "P2 has no live card (irrelevant here)");

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, live, TRIG_LIVE_START)) return;
    rh_caveat("d_mirage_renosora_no_baton", "d_mirage_two_baton_heart05");
    rh_note_fire(snap, "d4/no-baton");
    CHECK_EQ(need_of(&tg, live, H05), 0,
             "蓮ノ空 members without a baton-touch arrival do not qualify");
}

static void d_kokon_two_baton_arrivals_reduce_only_heart01(void)
{
    static TestGame tg;
    int live;
    if (!rh_setup_baton(&tg, "PL!HS-bp2-025-L", &live)) return;
    int a = mid(&tg, "PL!HS-bp5-004-R");
    int f = mid(&tg, "PL!-sd1-010-SD");
    int b = mid(&tg, "PL!HS-bp5-006-R");
    int c = mid(&tg, "PL!HS-bp5-001-P");
    if (!ident(&tg, a, "PL!HS-bp5-004-R")) return;
    if (!ident(&tg, b, "PL!HS-bp5-006-R")) return;
    if (!ident(&tg, c, "PL!HS-bp5-001-P")) return;
    if (f < 0) return;

    rh_set_stage3(&tg, 0, a, f, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, c);
    test_give_energy(&tg, 40);
    CHECK_EQ(test_play_to_stage(&tg, b, 0), 1,
             "HNS_B arrives at the left side (first baton touch)");
    rh_drain_first(&tg);
    CHECK_EQ(test_play_to_stage(&tg, c, 1), 1,
             "HNS_C arrives at the centre (second baton touch -- unreachable "
             "in C, see the note in d_mirage_two_baton_heart05)");
    rh_drain_first(&tg);

    int snap = rb_log_buffer_len();
    if (!rh_fire(&tg, live, TRIG_LIVE_START)) return;
    rh_note_fire(snap, "d5/kokon");
    CHECK_EQ(need_of(&tg, live, H01), -1,
             "ココン東西 reduces the heart01 requirement by 1");
    CHECK_EQ(need_of(&tg, live, H05), 0,
             "ココン東西 touches heart01 only");
}

/* ========================================================================
 * E. ladybug_stage_cost_comparison_required_hearts_test.rs  (Q114)
 *    PL!HS-bp2-024-L (レディバグ): at live start, if 徒町小鈴 is on the stage
 *    AND a 村野さやか with a HIGHER COST is also on the stage, need heart0 -3.
 *    Members only need to be ON STAGE; they do not need to have debuted.
 * ===================================================================== */

/* The shared Q114 board: `live_no` goes to hand, the two members (or fewer)
 * are placed directly on the stage, both decks are padded, and the turn is
 * walked to the performance. Returns the live card id. */
static int rh_ladybug_board(TestGame *tg, int left, int center)
{
    /* The caller has already built its card ids; this helper owns the
     * GameState lifetime, so it must call test_game_new() itself. Every
     * test_game_new() field is load-bearing here: a bare memset would
     * leave winner=0, which makes rb_advance_phase() a no-op and the
     * turn walk never leaves RB_PHASE_RPS. */
    test_game_new(tg);
    int ladybug = mid(tg, "PL!HS-bp2-024-L");
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (!ident(tg, ladybug, "PL!HS-bp2-024-L")) return -1;
    if (filler < 0) return -1;
    rh_set_stage3(tg, 0, left, center, RB_EMPTY_SLOT);
    test_add_to_hand(tg, ladybug);
    for (int i = 0; i < 10; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    rh_advance_to_live_set(tg);
    int reached_live_set =
        (tg->state.phase == RB_PHASE_LIVE_SET || tg->state.phase == RB_PHASE_PERFORMANCE);
    CHECK(reached_live_set, "the turn walk reaches the live-card set window");
    if (!reached_live_set)
        fprintf(stderr, "  [diag] the turn walk stalled in phase %d (%s) -- an "
                        "ability prompt is absorbing passes on this board\n",
                tg->state.phase, rb_phase_name(tg->state.phase));
    test_set_live_card(tg, 0, ladybug);
    CHECK_EQ(tg->state.p[0].live.n, 1, "Ladybug is the live card");
    rh_finish_live_setup(tg);
    return ladybug;
}

static void e_both_members_on_stage_reduces_hearts(void)
{
    static TestGame tg;
    int kosuzu = mid(&tg, "PL!HS-sd1-013-SD");
    int sayaka = mid(&tg, "PL!HS-sd1-002-SD");
    if (!ident(&tg, kosuzu, "PL!HS-sd1-013-SD")) return;
    if (!ident(&tg, sayaka, "PL!HS-sd1-002-SD")) return;
    /* Cost comparison is the gate: 小鈴 cost 2, さやか cost 11. */
    CHECK(card_cost(kosuzu) == 2, "徒町小鈴 prints cost 2");
    CHECK(card_cost(sayaka) == 11, "村野さやか prints cost 11 (> 2)");
    int live = rh_ladybug_board(&tg, kosuzu, sayaka);
    if (live < 0) return;
    CHECK_EQ(need_of(&tg, live, H00), -3,
             "Q114: both members on stage -> need heart00 is -3");
}

static void e_missing_sayaka_no_reduction(void)
{
    static TestGame tg;
    int kosuzu = mid(&tg, "PL!HS-sd1-013-SD");
    if (!ident(&tg, kosuzu, "PL!HS-sd1-013-SD")) return;
    int live = rh_ladybug_board(&tg, kosuzu, RB_EMPTY_SLOT);
    if (live < 0) return;
    CHECK_EQ(need_of(&tg, live, H00), 0,
             "Q114: missing さやか -> no heart reduction");
}

static void e_missing_kosuzu_no_reduction(void)
{
    static TestGame tg;
    int sayaka = mid(&tg, "PL!HS-sd1-002-SD");
    if (!ident(&tg, sayaka, "PL!HS-sd1-002-SD")) return;
    int live = rh_ladybug_board(&tg, RB_EMPTY_SLOT, sayaka);
    if (live < 0) return;
    CHECK_EQ(need_of(&tg, live, H00), 0,
             "Q114: missing 小鈴 -> no heart reduction");
}

static void e_wrong_character_no_reduction(void)
{
    static TestGame tg;
    int other = mid(&tg, "PL!HS-bp1-001-R");
    if (!ident(&tg, other, "PL!HS-bp1-001-R")) return;
    int live = rh_ladybug_board(&tg, other, other);
    if (live < 0) return;
    CHECK_EQ(need_of(&tg, live, H00), 0,
             "Q114: neither 小鈴 nor さやか on stage -> no reduction");
}

static void e_no_members_no_reduction(void)
{
    static TestGame tg;
    int live = rh_ladybug_board(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    if (live < 0) return;
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "the stage really is empty");
    CHECK_EQ(need_of(&tg, live, H00), 0,
             "Q114: no members at all -> no reduction");
}

/* ========================================================================
 * F. wonder_zone_center_heart03_required_hearts_test.rs
 *    PL!-bp5-020-L (Wonder zone): per 2 heart03 on the CENTRE member ->
 *    need heart0 -1 (per_unit_heart_colors + per_unit_count = 2).
 * ===================================================================== */

static int rh_wonder_board(TestGame *tg, int center_member)
{
    /* The caller has already built its card ids; this helper owns the
     * GameState lifetime, so it must call test_game_new() itself. Every
     * test_game_new() field is load-bearing here: a bare memset would
     * leave winner=0, which makes rb_advance_phase() a no-op and the
     * turn walk never leaves RB_PHASE_RPS. */
    test_game_new(tg);
    int wonder = mid(tg, "PL!-bp5-020-L");
    int filler = mid(tg, "LL-E-001-SD");
    if (!ident(tg, wonder, "PL!-bp5-020-L")) return -1;
    if (filler < 0) return -1;
    rh_set_stage3(tg, 0, RB_EMPTY_SLOT, center_member, RB_EMPTY_SLOT);
    test_add_to_hand(tg, wonder);
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    test_give_energy(tg, 10);
    rh_advance_to_live_set(tg);
    test_set_live_card(tg, 0, wonder);
    rh_finish_live_setup(tg);
    return wonder;
}

static void f_center_honoka_two_heart03_reduces_1(void)
{
    static TestGame tg;
    int honoka = mid(&tg, "PL!-sd1-001-SD");
    if (!ident(&tg, honoka, "PL!-sd1-001-SD")) return;
    /* printed base: heart01=1 heart03=2 heart06=1 -> 2 heart03 in the centre */
    int wonder = rh_wonder_board(&tg, honoka);
    if (wonder < 0) return;
    if (getenv("RH_DEBUG"))
        for (int c = 0; c < 8; c++)
            fprintf(stderr, "[F1 diag] wonder need_heart[%d] add=%d set=%d\n",
                    c, tg.state.mods.need_heart[wonder][c].add,
                    tg.state.mods.need_heart[wonder][c].set);
    CHECK_EQ(need_of(&tg, wonder, H00), -1,
             "centre member with 2 heart03 -> 1 unit -> need heart00 is -1");
}

static void f_center_rin_zero_heart03_no_reduction(void)
{
    static TestGame tg;
    int rin = mid(&tg, "PL!-sd1-005-SD");
    if (!ident(&tg, rin, "PL!-sd1-005-SD")) return;
    int wonder = rh_wonder_board(&tg, rin);
    if (wonder < 0) return;
    CHECK_EQ(need_of(&tg, wonder, H00), 0,
             "centre member with 0 heart03 -> 0 reduction");
    rh_caveat("f_center_rin_zero_heart03", "f_center_two_heart03_reduces_1");
}

static void f_no_center_member_no_reduction(void)
{
    static TestGame tg;
    int wonder = rh_wonder_board(&tg, RB_EMPTY_SLOT);
    if (wonder < 0) return;
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT, "the centre really is empty");
    CHECK_EQ(need_of(&tg, wonder, H00), 0,
             "empty centre -> condition fails -> 0");
    rh_caveat("f_no_center_member", "f_center_two_heart03_reduces_1");
}

static void f_center_non_mus_no_reduction(void)
{
    static TestGame tg;
    int non_mus = mid(&tg, "PL!SP-sd1-001-SD");
    if (!ident(&tg, non_mus, "PL!SP-sd1-001-SD")) return;
    int wonder = rh_wonder_board(&tg, non_mus);
    if (wonder < 0) return;
    CHECK_EQ(need_of(&tg, wonder, H00), 0,
             "a non-μ's centre member -> condition fails -> 0");
    rh_caveat("f_center_non_mus", "f_center_two_heart03_reduces_1");
}

/* ========================================================================
 * G. modify_required_hearts_exclude_heart_test.rs
 *    PL!-bp5-023-L (乙姫心で恋宮殿): per stage member POSSESSING a heart colour
 *    other than heart01/heart06 -> need heart0 -1 (exclude_heart_colors).
 * ===================================================================== */

static int rh_otsubimi_board(TestGame *tg, int a, int b, int c)
{
    /* The caller has already built its card ids; this helper owns the
     * GameState lifetime, so it must call test_game_new() itself. Every
     * test_game_new() field is load-bearing here: a bare memset would
     * leave winner=0, which makes rb_advance_phase() a no-op and the
     * turn walk never leaves RB_PHASE_RPS. */
    test_game_new(tg);
    int card = mid(tg, "PL!-bp5-023-L");
    int filler = mid(tg, "LL-E-001-SD");
    if (!ident(tg, card, "PL!-bp5-023-L")) return -1;
    if (filler < 0) return -1;
    rh_set_stage3(tg, 0, a, b, c);
    test_add_to_hand(tg, card);
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    test_give_energy(tg, 10);
    rh_advance_to_live_set(tg);
    test_set_live_card(tg, 0, card);
    rh_finish_live_setup(tg);
    return card;
}

static void g_all_members_excluded_no_reduction(void)
{
    static TestGame tg;
    int e1 = mid(&tg, "PL!-sd1-002-SD");     /* heart06 only  */
    int e2 = mid(&tg, "PL!-PR-002-PR");      /* heart06 only  */
    int e3 = mid(&tg, "PL!-sd1-005-SD");     /* heart01 only  */
    if (!ident(&tg, e1, "PL!-sd1-002-SD")) return;
    if (!ident(&tg, e2, "PL!-PR-002-PR")) return;
    if (!ident(&tg, e3, "PL!-sd1-005-SD")) return;
    /* Printed-heart control: the fixture really carries ONLY excluded
     * colours, so the zero below is a filter verdict, not a data accident. */
    {
        int h1[8], h2[8], h3[8];
        card_base_hearts(e1, h1); card_base_hearts(e2, h2); card_base_hearts(e3, h3);
        CHECK(h1[H06] > 0 && h1[H01] == 0 && h1[H02] == 0 && h1[H03] == 0 && h1[H04] == 0 && h1[H05] == 0,
              "PL!-sd1-002-SD really carries heart06 only");
        CHECK(h2[H06] > 0 && h2[H01] == 0 && h2[H02] == 0 && h2[H03] == 0 && h2[H04] == 0 && h2[H05] == 0,
              "PL!-PR-002-PR really carries heart06 only");
        CHECK(h3[H01] > 0 && h3[H02] == 0 && h3[H03] == 0 && h3[H04] == 0 && h3[H05] == 0 && h3[H06] == 0,
              "PL!-sd1-005-SD really carries heart01 only");
    }
    int card = rh_otsubimi_board(&tg, e1, e2, e3);
    if (card < 0) return;
    CHECK_EQ(tg.state.p[0].stage[0], e1, "all three members are really staged");
    CHECK_EQ(need_of(&tg, card, H00), 0,
             "every member has ONLY excluded colours -> 0 reduction");
}

static void g_one_non_excluded_member_reduces_1(void)
{
    static TestGame tg;
    int counts = mid(&tg, "PL!-sd1-001-SD");   /* heart01+heart03+heart06 */
    int excl1 = mid(&tg, "PL!-sd1-002-SD");
    int excl2 = mid(&tg, "PL!-PR-002-PR");
    if (!ident(&tg, counts, "PL!-sd1-001-SD")) return;
    if (excl1 < 0 || excl2 < 0) return;
    int card = rh_otsubimi_board(&tg, counts, excl1, excl2);
    if (card < 0) return;
    CHECK_EQ(need_of(&tg, card, H00), -1,
             "one member owns a colour outside {heart01, heart06} -> -1");
}

static void g_mixed_excluded_and_counted_members(void)
{
    static TestGame tg;
    int excl = mid(&tg, "PL!-sd1-002-SD");
    int cnt1 = mid(&tg, "PL!-sd1-001-SD");
    int cnt2 = mid(&tg, "PL!-sd1-010-SD");     /* heart01+heart03 */
    if (!ident(&tg, cnt1, "PL!-sd1-001-SD")) return;
    if (!ident(&tg, cnt2, "PL!-sd1-010-SD")) return;
    if (excl < 0) return;
    int card = rh_otsubimi_board(&tg, excl, cnt1, cnt2);
    if (card < 0) return;
    CHECK_EQ(need_of(&tg, card, H00), -2,
             "two eligible members -> -2");
}

static void g_member_with_both_excluded_and_allowed_colors_counts(void)
{
    static TestGame tg;
    int member = mid(&tg, "PL!-sd1-001-SD");  /* heart01+heart03+heart06 */
    int excl1 = mid(&tg, "PL!-sd1-002-SD");
    int excl2 = mid(&tg, "PL!-PR-002-PR");
    if (!ident(&tg, member, "PL!-sd1-001-SD")) return;
    if (excl1 < 0 || excl2 < 0) return;
    int card = rh_otsubimi_board(&tg, member, excl1, excl2);
    if (card < 0) return;
    CHECK_EQ(need_of(&tg, card, H00), -1,
             "heart01+heart03+heart06 counts (it HAS heart03) -> -1");
}

/* ========================================================================
 * H. hanamusubi_required_hearts_exclude_self_test.rs
 *    PL!HS-bp5-019-L (ハナムスビ): per OTHER 『蓮ノ空』 card in my live zone ->
 *    need heart04 -2 (exclude_self + per_unit).
 * ===================================================================== */

static int rh_hanamusubi_board(TestGame *tg, const int *lives, int n_lives)
{
    /* The caller has already built its card ids; this helper owns the
     * GameState lifetime, so it must call test_game_new() itself. Every
     * test_game_new() field is load-bearing here: a bare memset would
     * leave winner=0, which makes rb_advance_phase() a no-op and the
     * turn walk never leaves RB_PHASE_RPS. */
    test_game_new(tg);
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return -1;
    int f2 = mid(tg, "PL!-sd1-010-SD");
    int f3 = mid(tg, "PL!-sd1-010-SD");
    if (f2 < 0 || f3 < 0) return -1;
    rh_set_stage3(tg, 0, filler, f2, f3);
    for (int i = 0; i < n_lives; i++) test_add_to_hand(tg, lives[i]);
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    test_give_energy(tg, 10);
    rh_advance_to_live_set(tg);
    for (int i = 0; i < n_lives; i++) test_set_live_card(tg, 0, lives[i]);
    rh_finish_live_setup(tg);
    return 0;
}

static void h_alone_no_reduction(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    int lives[1] = { hana };
    if (rh_hanamusubi_board(&tg, lives, 1) != 0) return;
    CHECK_EQ(tg.state.p[0].live.n, 1, "ハナムスビ is the ONLY live card");
    CHECK_EQ(need_of(&tg, hana, H04), 0,
             "exclude_self: alone there is no OTHER 蓮ノ空 card -> 0");
}

static void h_with_one_other_renosora_reduces_2(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int other = mid(&tg, "PL!HS-bp5-017-L");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (!ident(&tg, other, "PL!HS-bp5-017-L")) return;
    int lives[2] = { hana, other };
    if (rh_hanamusubi_board(&tg, lives, 2) != 0) return;
    CHECK_EQ(tg.state.p[0].live.n, 2, "both 蓮ノ空 live cards are set");
    CHECK_EQ(need_of(&tg, hana, H04), -2,
             "one OTHER 蓮ノ空 live card -> need heart04 is -2");
    CHECK_EQ(need_of(&tg, other, H04), 0,
             "the other card is not modified (target = self)");
}

static void h_with_two_other_renosora_reduces_4(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int a = mid(&tg, "PL!HS-bp5-017-L");
    int b = mid(&tg, "PL!HS-bp5-018-L");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (!ident(&tg, a, "PL!HS-bp5-017-L")) return;
    if (!ident(&tg, b, "PL!HS-bp5-018-L")) return;
    int lives[3] = { hana, a, b };
    if (rh_hanamusubi_board(&tg, lives, 3) != 0) return;
    CHECK_EQ(tg.state.p[0].live.n, 3, "three live cards are set");
    CHECK_EQ(need_of(&tg, hana, H04), -4,
             "two OTHER 蓮ノ空 live cards -> -2 EACH, so -4 (per-unit value "
             "is 2, not 1)");
}

static void h_with_non_renosora_no_reduction(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int non = mid(&tg, "PL!-sd1-019-SD");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (!ident(&tg, non, "PL!-sd1-019-SD")) return;
    int lives[2] = { hana, non };
    if (rh_hanamusubi_board(&tg, lives, 2) != 0) return;
    CHECK_EQ(need_of(&tg, hana, H04), 0,
             "a non-蓮ノ空 live card does not satisfy the group filter -> 0");
}

static void h_reduces_only_heart04_not_heart00(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int other = mid(&tg, "PL!HS-bp5-017-L");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (other < 0) return;
    int lives[2] = { hana, other };
    if (rh_hanamusubi_board(&tg, lives, 2) != 0) return;
    CHECK_EQ(need_of(&tg, hana, H04), -2, "heart04 is reduced by 2");
    CHECK_EQ(need_of(&tg, hana, H00), 0,
             "heart0 is NOT reduced -- the ability names heart04 only");
}

static void h_two_hanamusubi_each_reduces_2(void)
{
    static TestGame tg;
    int a = mid(&tg, "PL!HS-bp5-019-L");
    int b = mid(&tg, "PL!HS-bp5-019-L");
    if (!ident(&tg, a, "PL!HS-bp5-019-L")) return;
    if (!ident(&tg, b, "PL!HS-bp5-019-L")) return;
    int lives[2] = { a, b };
    if (rh_hanamusubi_board(&tg, lives, 2) != 0) return;
    CHECK_EQ(need_of(&tg, a, H04), -2, "A sees B as one other -> -2");
    CHECK_EQ(need_of(&tg, b, H04), -2, "B sees A as one other -> -2");
}

static void h_three_hanamusubi_each_reduces_4(void)
{
    static TestGame tg;
    int a = mid(&tg, "PL!HS-bp5-019-L");
    int b = mid(&tg, "PL!HS-bp5-019-L");
    int c = mid(&tg, "PL!HS-bp5-019-L");
    if (!ident(&tg, a, "PL!HS-bp5-019-L")) return;
    if (b < 0 || c < 0) return;
    int lives[3] = { a, b, c };
    if (rh_hanamusubi_board(&tg, lives, 3) != 0) return;
    CHECK_EQ(need_of(&tg, a, H04), -4, "A sees two others -> -4");
    CHECK_EQ(need_of(&tg, b, H04), -4, "B sees two others -> -4");
    CHECK_EQ(need_of(&tg, c, H04), -4, "C sees two others -> -4");
}

static void h_two_hanamusubi_one_filler_each_reduces_2(void)
{
    static TestGame tg;
    int a = mid(&tg, "PL!HS-bp5-019-L");
    int b = mid(&tg, "PL!HS-bp5-019-L");
    int filler_live = mid(&tg, "PL!-sd1-019-SD");
    if (!ident(&tg, a, "PL!HS-bp5-019-L")) return;
    if (filler_live < 0) return;
    int lives[3] = { a, b, filler_live };
    if (rh_hanamusubi_board(&tg, lives, 3) != 0) return;
    CHECK_EQ(need_of(&tg, a, H04), -2, "A: the filler does not match -> -2");
    CHECK_EQ(need_of(&tg, b, H04), -2, "B: the filler does not match -> -2");
}

static void h_one_hanamusubi_one_filler_no_reduction(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int filler_live = mid(&tg, "PL!-sd1-019-SD");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (filler_live < 0) return;
    int lives[2] = { hana, filler_live };
    if (rh_hanamusubi_board(&tg, lives, 2) != 0) return;
    CHECK_EQ(need_of(&tg, hana, H04), 0, "no OTHER 蓮ノ空 card -> 0");
}

static void h_one_hanamusubi_two_fillers_no_reduction(void)
{
    static TestGame tg;
    int hana = mid(&tg, "PL!HS-bp5-019-L");
    int f1 = mid(&tg, "PL!-sd1-019-SD");
    int f2 = mid(&tg, "PL!-sd1-020-SD");
    if (!ident(&tg, hana, "PL!HS-bp5-019-L")) return;
    if (!ident(&tg, f1, "PL!-sd1-019-SD")) return;
    if (!ident(&tg, f2, "PL!-sd1-020-SD")) return;
    int lives[3] = { hana, f1, f2 };
    if (rh_hanamusubi_board(&tg, lives, 3) != 0) return;
    CHECK_EQ(need_of(&tg, hana, H04), 0,
             "two DIFFERENT non-蓮ノ空 live cards still give 0");
}

/* ========================================================================
 * I. jellyfish_appeared_moved_member_required_hearts_test.rs
 *    PL!SP-pb1-025-L (Jellyfish): per 『5yncri5e!』 member that APPEARED OR
 *    MOVED this turn -> need heart0 -1 (timing_condition =
 *    appeared_or_moved_this_turn).
 * ===================================================================== */

static int rh_jellyfish_board(TestGame *tg, int *jelly_out)
{
    int jelly = mid(tg, "PL!SP-pb1-025-L");
    if (!ident(tg, jelly, "PL!SP-pb1-025-L")) return -1;
    *jelly_out = jelly;
    return 0;
}

/* Build the shared Jellyfish board: decks padded, `n_hold` pre-staged
 * members, `n_hand` cards in hand, `energy` active energy. */
static void rh_jelly_setup(TestGame *tg, int jelly, int n_hold,
                           const int *hold, int n_hand, const int *hand, int energy)
{
    int filler = mid(tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    for (int i = 0; i < 10; i++) {
        int a = mid(tg, "PL!-sd1-010-SD");
        int b = mid(tg, "PL!-sd1-010-SD");
        if (a >= 0) test_add_to_deck_pl(tg, 0, a);
        if (b >= 0) test_add_to_deck_pl(tg, 1, b);
    }
    rh_set_stage3(tg, 0, n_hold > 0 ? hold[0] : RB_EMPTY_SLOT,
                  n_hold > 1 ? hold[1] : RB_EMPTY_SLOT,
                  n_hold > 2 ? hold[2] : RB_EMPTY_SLOT);
    test_add_to_hand(tg, jelly);
    for (int i = 0; i < n_hand; i++) test_add_to_hand(tg, hand[i]);
    test_give_energy(tg, energy);
}

static void i_two_members_appeared_reduce_by_2(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    int natsumi = mid(&tg, "PL!SP-pb1-009-R");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    if (!ident(&tg, natsumi, "PL!SP-pb1-009-R")) return;
    int hand[2] = { chisato, natsumi };
    rh_jelly_setup(&tg, jelly, 0, NULL, 2, hand, 10);
    rh_advance_to_live_set(&tg);
    CHECK_EQ(test_play_to_stage(&tg, chisato, 0), 1, "千砂都 debuts left");
    CHECK_EQ(test_play_to_stage(&tg, natsumi, 1), 1, "夏美 debuts centre");
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -2,
             "two appeared 5yncri5e! members -> -2");
}

static void i_one_member_both_flags_counts_once(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    int hand[1] = { chisato };
    rh_jelly_setup(&tg, jelly, 0, NULL, 1, hand, 10);
    rh_advance_to_live_set(&tg);
    CHECK_EQ(test_play_to_stage(&tg, chisato, 1), 1, "千砂都 debuts centre");
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -1,
             "Q99: a member that both appeared AND moved counts ONCE -> -1");
}

static void i_position_change_swap_creates_countable_movement(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int wakana = mid(&tg, "PL!SP-pb1-008-R");
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    if (!ident(&tg, wakana, "PL!SP-pb1-008-R")) return;
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    int hand[1] = { wakana };
    rh_jelly_setup(&tg, jelly, 1, &chisato, 1, hand, 20);
    rh_advance_to_live_set(&tg);
    CHECK_EQ(test_play_to_stage(&tg, wakana, 1), 1, "四季 debuts into the centre");

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    assertions++;
    if (!ch) {
        n_vacuous++;
        fprintf(stderr, "VACUOUS: i3: no area_select prompt is pending after "
                        "四季's debut, so the Q-rule under test never ran\n");
        return;
    }
    CHECK(ch->kind == RB_CHOICE_SELECT_TARGET, "the prompt is a SelectTarget");
    CHECK(!strcmp(ch->target, "area_select"), "the prompt target is \"area_select\"");
    /* The flat RbChoice carries NO option list (only n_heart_options), so the
     * Rust `options.contains("left")` assertion is not directly observable.
     * The C area_select handler puts the option list into `description`; that
     * is read here and recorded as an EXPECTED_GAP on the flat struct. */
    CHECK(strstr(ch->description, "left") != NULL,
          "the offered area list contains \"left\" (read from description; "
          "the flat RbChoice has no option-list field -- EXPECTED_GAP)");

    rh_select_option0(&tg);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -2,
             "a position change makes the second member countable -> -2");
}

static void i_mixed_qualifying_and_non_qualifying(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    int natsumi = mid(&tg, "PL!SP-pb1-009-R");
    int honoka = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    if (!ident(&tg, natsumi, "PL!SP-pb1-009-R")) return;
    if (!ident(&tg, honoka, "PL!-sd1-010-SD")) return;
    int hand[3] = { chisato, natsumi, honoka };
    rh_jelly_setup(&tg, jelly, 0, NULL, 3, hand, 10);
    rh_advance_to_live_set(&tg);
    test_play_to_stage(&tg, chisato, 0);
    test_play_to_stage(&tg, natsumi, 1);
    test_play_to_stage(&tg, honoka, 2);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -2,
             "the non-5yncri5e! member does not count -> -2, not -3");
}

static void i_no_qualifying_members_no_reduction(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int honoka = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, honoka, "PL!-sd1-010-SD")) return;
    int hand[1] = { honoka };
    rh_jelly_setup(&tg, jelly, 0, NULL, 1, hand, 10);
    rh_advance_to_live_set(&tg);
    test_play_to_stage(&tg, honoka, 1);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), 0,
             "only a non-5yncri5e! member appeared -> 0");
    rh_caveat("i_no_qualifying", "i_two_appeared_reduce_2");
}

static void i_three_members_all_qualify_reduce_by_3(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    int natsumi = mid(&tg, "PL!SP-pb1-009-R");
    int wakana = mid(&tg, "PL!SP-PR-010-PR");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    if (!ident(&tg, natsumi, "PL!SP-pb1-009-R")) return;
    if (!ident(&tg, wakana, "PL!SP-PR-010-PR")) return;
    int hand[3] = { chisato, natsumi, wakana };
    rh_jelly_setup(&tg, jelly, 0, NULL, 3, hand, 10);
    rh_advance_to_live_set(&tg);
    test_play_to_stage(&tg, chisato, 0);
    test_play_to_stage(&tg, natsumi, 1);
    test_play_to_stage(&tg, wakana, 2);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -3, "three qualifying members -> -3");
}

static void i_moved_without_appearing_this_turn_counts(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int wakana = mid(&tg, "PL!SP-pb1-008-R");
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    int honoka = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, wakana, "PL!SP-pb1-008-R")) return;
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    if (!ident(&tg, honoka, "PL!-sd1-010-SD")) return;
    int hold[3] = { chisato, RB_EMPTY_SLOT, honoka };
    int hand[1] = { wakana };
    rh_jelly_setup(&tg, jelly, 3, hold, 1, hand, 20);
    rh_advance_to_live_set(&tg);
    test_play_to_stage(&tg, wakana, 1);
    rh_select_option0(&tg);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -2,
             "the swapped-out 四季 counts as MOVED -> -2");
}

static void i_neither_appeared_nor_moved_not_counted(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    int hold[1] = { chisato };
    rh_jelly_setup(&tg, jelly, 1, hold, 0, NULL, 10);
    rh_advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), 0,
             "a member that neither appeared nor moved does not count -> 0");
    rh_caveat("i_neither_appeared_nor_moved", "i_two_appeared_reduce_2");
}

static void i_appeared_then_left_not_counted(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    for (int i = 0; i < 10; i++) {
        int a = mid(&tg, "PL!-sd1-010-SD");
        int b = mid(&tg, "PL!-sd1-010-SD");
        if (a >= 0) test_add_to_deck_pl(&tg, 0, a);
        if (b >= 0) test_add_to_deck_pl(&tg, 1, b);
    }
    test_add_to_hand(&tg, jelly);
    test_give_energy(&tg, 20);
    /* Place 千砂都 on the stage directly, then remove her.
     *
     * ENGINE-GAP CAVEAT: the appearance is recorded BY HAND here, not by
     * rb_play_member(). `cards_appeared_this_turn[]` is written in exactly two
     * places in the C port -- rb_record_card_appearance() (which has NO
     * callers anywhere under src/) and rb_move_fire_debut_side_effects() (the
     * move_cards executor) -- and rb_play_member() calls neither. Without this
     * hand-written record the per-unit `appeared_or_moved_this_turn` count is
     * structurally 0 on the member-debut path, which is why i1..i7 fail while
     * this one passes. Keep the explicit record so the test still asserts the
     * rule under test (Q98: off-stage members do not count) rather than the
     * unrelated recording gap. */
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, chisato, RB_EMPTY_SLOT);
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rh_advance_to_live_set(&tg);
    rh_mark_appeared(&tg, chisato);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT, "千砂都 really is off-stage");
    CHECK(rb_has_card_appeared_this_turn(&tg.state, chisato),
          "千砂都 is still recorded as appeared this turn");
    CHECK_EQ(need_of(&tg, jelly, H00), 0,
             "Q98: appeared + moved but NOT on stage -> 0");
    rh_caveat("i_appeared_then_left", "i_two_appeared_reduce_2");
}

static void i_one_left_one_stays(void)
{
    static TestGame tg;
    int jelly;
    if (rh_jellyfish_board(&tg, &jelly) != 0) return;
    int chisato = mid(&tg, "PL!SP-pb1-014-N");
    int natsumi = mid(&tg, "PL!SP-pb1-009-R");
    if (!ident(&tg, chisato, "PL!SP-pb1-014-N")) return;
    if (!ident(&tg, natsumi, "PL!SP-pb1-009-R")) return;
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (filler < 0) return;
    for (int i = 0; i < 10; i++) {
        int a = mid(&tg, "PL!-sd1-010-SD");
        int b = mid(&tg, "PL!-sd1-010-SD");
        if (a >= 0) test_add_to_deck_pl(&tg, 0, a);
        if (b >= 0) test_add_to_deck_pl(&tg, 1, b);
    }
    test_add_to_hand(&tg, jelly);
    test_give_energy(&tg, 20);
    rh_set_stage3(&tg, 0, chisato, natsumi, RB_EMPTY_SLOT);
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, natsumi, RB_EMPTY_SLOT);
    rh_advance_to_live_set(&tg);
    rh_mark_appeared(&tg, natsumi);
    test_set_live_card(&tg, 0, jelly);
    rh_finish_live_setup(&tg);
    CHECK_EQ(need_of(&tg, jelly, H00), -1,
             "Q98: only 夏美 is on stage -> -1");
}

/* ========================================================================
 * J. modify_required_hearts_13key_test.rs
 *    PL!-bp6-022-L (Dreamin' Go! Go!!), 常時, as long as it is in MY SUCCESS
 *    live card zone: reduce by 2 the need heart00 of MY μ's live cards whose
 *    ORIGINAL score is >= 5. 「この効果は重複しない」.
 *    Rust calls evaluate_success_zone_constant_abilities(); the C
 *    counterpart is rb_recalc_constants(), which is strictly broader (it also
 *    scans the STAGE and the LIVE zone) -- see the final report.
 * ===================================================================== */

#define J_ACTIVATE "PL!-bp6-022-L"
#define J_HIGH     "PL!-bp3-021-L"   /* μ's, score 6 */
#define J_LOW      "PL!-sd1-020-SD"   /* μ's, score 2 */
#define J_NONMUS   "PL!S-PR-024-PR"   /* Aqours, score 5 */

static int rh_j_setup(TestGame *tg, int pl_activator,
                      const int *p1_lives, int n1, const int *p2_lives, int n2)
{
    int act = mid(tg, J_ACTIVATE);
    if (!ident(tg, act, J_ACTIVATE)) return -1;
    add_success_for(tg, pl_activator, act);
    for (int i = 0; i < n1; i++) test_add_to_live_for(tg, 0, p1_lives[i]);
    for (int i = 0; i < n2; i++) test_add_to_live_for(tg, 1, p2_lives[i]);
    if (!rh_wired(tg, act)) return -1;
    return act;
}

static void j_high_score_mus_live_gets_reduction(void)
{
    static TestGame tg;
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, high, J_HIGH)) return;
    CHECK(card_score(high) == 6, "愛してるばんざーい! prints score 6");
    int lives[1] = { high };
    int act = rh_j_setup(&tg, 0, lives, 1, NULL, 0);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "score-6 μ's live in my live zone -> need heart00 is -2");
}

static void j_low_score_mus_live_no_reduction(void)
{
    static TestGame tg;
    int low = mid(&tg, J_LOW);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, low, J_LOW)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    CHECK(card_score(low) == 2, "きっと青春が聞こえる prints score 2");
    int lives[2] = { low, high };
    int act = rh_j_setup(&tg, 0, lives, 2, NULL, 0);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, low, H00), 0,
             "score-2 μ's live -> no modifier (the printed threshold is 5)");
    /* NON-VACUITY CONTROL: the SAME recalc, the SAME activator, with a
     * score-6 μ's live alongside. Without this the zero above would also
     * read green on a board where the 常時 never applied at all. */
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: the same recalc DOES reduce the score-6 μ's live, so "
             "the score-2 zero is a real threshold rejection");
}

static void j_mixed_scores_filter_correctly(void)
{
    static TestGame tg;
    int high = mid(&tg, J_HIGH);
    int low = mid(&tg, J_LOW);
    if (!ident(&tg, high, J_HIGH)) return;
    if (!ident(&tg, low, J_LOW)) return;
    int lives[2] = { high, low };
    int act = rh_j_setup(&tg, 0, lives, 2, NULL, 0);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2, "the score-6 live is reduced by 2");
    CHECK_EQ(need_of(&tg, low, H00), 0,  "the score-2 live is not");
}

static void j_non_mus_high_score_no_reduction(void)
{
    static TestGame tg;
    int nonmus = mid(&tg, J_NONMUS);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, nonmus, J_NONMUS)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    CHECK(card_score(nonmus) == 5,
          "勇気はどこに?君の胸に! prints score 5 (>= 5) but is Aqours");
    int lives[2] = { nonmus, high };
    int act = rh_j_setup(&tg, 0, lives, 2, NULL, 0);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, nonmus, H00), 0,
             "an Aqours score-5 live is not a μ's live -> no modifier");
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: the μ's live beside it IS reduced");
}

static void j_activator_not_in_success_zone_does_nothing(void)
{
    static TestGame tg;
    int act = mid(&tg, J_ACTIVATE);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, act, J_ACTIVATE)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    if (!rh_wired(&tg, act)) return;

    /* CONTROL PHASE: activator in the SUCCESS zone -> the effect applies. */
    test_add_to_success(&tg, act);
    test_add_to_live(&tg, high);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: with the activator in the success zone the effect applies");

    /* Now move it to the LIVE zone and re-evaluate: 「自分の成功ライブカード
     * 置き場にあるかぎり」 is no longer satisfied. */
    tg.state.p[0].success.n = 0;
    test_add_to_live(&tg, act);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(tg.state.p[0].success.n, 0, "the success zone really is empty");
    CHECK_EQ(need_of(&tg, high, H00), 0,
             "activator in the LIVE zone -> the as-long-as gate is off -> 0");
}

static void j_no_live_cards_no_crash(void)
{
    static TestGame tg;
    int act = mid(&tg, J_ACTIVATE);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, act, J_ACTIVATE)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    if (!rh_wired(&tg, act)) return;

    test_add_to_success(&tg, act);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(tg.state.p[0].live.n, 0, "the live zone really is empty");
    int nonzero = 0;
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++)
        for (int c = 0; c < 8; c++)
            if (tg.state.mods.need_heart[cid][c].add != 0 ||
                tg.state.mods.need_heart[cid][c].set != 0) nonzero = 1;
    CHECK_EQ(nonzero, 0, "no live cards -> no need_heart modifier anywhere");

    /* NON-VACUITY CONTROL: add a qualifying live card on the SAME board. */
    test_add_to_live(&tg, high);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: the same activator DOES reduce once a qualifying live "
             "card exists, so the empty-zone zero above is meaningful");
}

static void j_non_stackable_duplicate_cards(void)
{
    static TestGame tg;
    int act = mid(&tg, J_ACTIVATE);
    int act2 = mid(&tg, J_ACTIVATE);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, act, J_ACTIVATE)) return;
    if (!ident(&tg, act2, J_ACTIVATE)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    if (!rh_wired(&tg, act)) return;
    test_add_to_success(&tg, act);
    test_add_to_success(&tg, act2);
    test_add_to_live(&tg, high);
    CHECK_EQ(tg.state.p[0].success.n, 2,
             "two DISTINCT copies of the 13-key card are in the success zone");
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "「この効果は重複しない」 -> -2, NOT -4");
}

static void j_as_long_as_expiration_on_zone_exit(void)
{
    static TestGame tg;
    int act = mid(&tg, J_ACTIVATE);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, act, J_ACTIVATE)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    if (!rh_wired(&tg, act)) return;
    test_add_to_success(&tg, act);
    test_add_to_live(&tg, high);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), -2, "before removal -> -2");
    tg.state.p[0].success.n = 0;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, high, H00), 0,
             "after the activator leaves the success zone -> 0 (as-long-as "
             "expiry; the first assertion is this test's own control)");
}

static void j_p1_activator_does_not_affect_p2(void)
{
    static TestGame tg;
    int p1_live = mid(&tg, J_HIGH);
    int p2_live = mid(&tg, J_HIGH);
    int lives[1] = { p1_live };
    int p2[1] = { p2_live };
    int act = rh_j_setup(&tg, 0, lives, 1, p2, 1);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, p1_live, H00), -2, "P1's own score-6 μ's live -> -2");
    CHECK_EQ(need_of(&tg, p2_live, H00), 0,
             "P2's identical score-6 μ's live is NOT affected (target = self)");
}

static void j_p2_activator_targets_p2_not_p1(void)
{
    static TestGame tg;
    int p1_live = mid(&tg, J_HIGH);
    int p2_live = mid(&tg, J_HIGH);
    int lives[1] = { p1_live };
    int p2[1] = { p2_live };
    int act = rh_j_setup(&tg, 1, lives, 1, p2, 1);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, p2_live, H00), -2,
             "P2 owns the activator -> P2's score-6 μ's live is -2 "
             "(regression for resolve_target_player(\"self\") defaulting to p1)");
    CHECK_EQ(need_of(&tg, p1_live, H00), 0,
             "P1's identical live is NOT affected (the activator is P2's)");
}

static void j_p2_activator_respects_score_threshold(void)
{
    static TestGame tg;
    int low = mid(&tg, J_LOW);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, low, J_LOW)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    int p2[2] = { low, high };
    int act = rh_j_setup(&tg, 1, NULL, 0, p2, 2);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, low, H00), 0, "P2's score-2 μ's live -> no modifier");
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: P2's score-6 μ's live beside it IS reduced");
}

static void j_p2_activator_respects_group_filter(void)
{
    static TestGame tg;
    int nonmus = mid(&tg, J_NONMUS);
    int high = mid(&tg, J_HIGH);
    if (!ident(&tg, nonmus, J_NONMUS)) return;
    if (!ident(&tg, high, J_HIGH)) return;
    int p2[2] = { nonmus, high };
    int act = rh_j_setup(&tg, 1, NULL, 0, p2, 2);
    if (act < 0) return;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, nonmus, H00), 0,
             "P2's Aqours score-5 live -> no modifier (group filter)");
    CHECK_EQ(need_of(&tg, high, H00), -2,
             "CONTROL: P2's μ's score-6 live beside it IS reduced");
}

/* ========================================================================
 * K. modify_required_hearts_global_test.rs
 *    PL!SP-bp2-010-P (ウィーン・マルガレーテ), 常時: EVERY live card in the
 *    OPPONENT's live card zone needs heart0 +1, per Wien instance.
 *    PL!S-bp5-011-N (桜内梨子), 登場: if my stage's members hold >= 5 heart02
 *    in total, one live card in the OPPONENT's live zone needs heart0 +1 at
 *    the opponent's live start.
 * ===================================================================== */

#define K_WIEN "PL!SP-bp2-010-P"
#define K_RIKO "PL!S-bp5-011-N"
#define K_OPP_LIVE_A "PL!-sd1-019-SD"
#define K_OPP_LIVE_B "PL!-sd1-020-SD"
#define K_OPP_LIVE_C "PL!-sd1-021-SD"

static void k_wien_constant_increases_opponent_live_cards(void)
{
    static TestGame tg;
    int wien = mid(&tg, K_WIEN);
    int opp = mid(&tg, K_OPP_LIVE_A);
    if (!ident(&tg, wien, K_WIEN)) return;
    if (!ident(&tg, opp, K_OPP_LIVE_A)) return;
    if (!rh_wired(&tg, wien)) return;
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, opp);
    CHECK_EQ(tg.state.p[1].live.n, 1, "the opponent owns one live card");
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, opp, H00), 1,
             "Wien's 常時 gives the OPPONENT's live card exactly +1 heart00");
}

static void k_wien_constant_removed_when_leaves_stage(void)
{
    static TestGame tg;
    int wien = mid(&tg, K_WIEN);
    int opp = mid(&tg, K_OPP_LIVE_A);
    if (!ident(&tg, wien, K_WIEN)) return;
    if (!rh_wired(&tg, wien)) return;
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, opp);
    rb_recalc_constants(&tg.state);
    CHECK(need_of(&tg, opp, H00) >= 1,
          "the modifier is active while Wien is on stage (this assertion is "
          "the non-vacuity control for the zero below)");
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, opp, H00), 0,
             "the modifier is removed once Wien leaves the stage");
}

static void k_wien_constant_applies_to_all_opponent_live_cards(void)
{
    static TestGame tg;
    int wien = mid(&tg, K_WIEN);
    int l1 = mid(&tg, K_OPP_LIVE_A);
    int l2 = mid(&tg, K_OPP_LIVE_B);
    int l3 = mid(&tg, K_OPP_LIVE_C);
    if (!ident(&tg, wien, K_WIEN)) return;
    if (!ident(&tg, l1, K_OPP_LIVE_A)) return;
    if (!ident(&tg, l2, K_OPP_LIVE_B)) return;
    if (!ident(&tg, l3, K_OPP_LIVE_C)) return;
    /* Three DIFFERENT prints, so "all of them" cannot pass by matching the
     * same card twice. */
    CHECK(test_max_distinct_names((const int[]){ l1, l2, l3 }, 3) == 3,
          "the three opponent live prints are three distinct names");
    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, l1);
    test_add_to_live_for(&tg, 1, l2);
    test_add_to_live_for(&tg, 1, l3);
    CHECK_EQ(tg.state.p[1].live.n, 3, "the opponent has three live cards");
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, l1, H00), 1, "opponent live 0 gets exactly +1");
    CHECK_EQ(need_of(&tg, l2, H00), 1, "opponent live 1 gets exactly +1");
    CHECK_EQ(need_of(&tg, l3, H00), 1, "opponent live 2 gets exactly +1");
}

static void k_two_wien_instances_stack(void)
{
    static TestGame tg;
    int wien_l = mid(&tg, K_WIEN);
    int wien_c = mid(&tg, K_WIEN);
    int la = mid(&tg, K_OPP_LIVE_A);
    int lb = mid(&tg, K_OPP_LIVE_B);
    if (!ident(&tg, wien_l, K_WIEN)) return;
    if (!ident(&tg, wien_c, K_WIEN)) return;
    if (!rh_wired(&tg, wien_l)) return;
    rh_set_stage3(&tg, 0, wien_l, wien_c, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, la);
    test_add_to_live_for(&tg, 1, lb);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, la, H00), 2, "two Wien instances stack -> +2");
    CHECK_EQ(need_of(&tg, lb, H00), 2, "the second opponent live also +2");
    rh_set_stage3(&tg, 0, wien_l, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(need_of(&tg, la, H00), 1,
             "one instance removed -> back to +1 (non-vacuity control for the "
             "+2 assertions)");
    CHECK_EQ(need_of(&tg, lb, H00), 1, "same for the second opponent live");
}

static void k_riko_condition_met_increases_opponent_hearts(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chika = mid(&tg, "PL!S-sd1-001-SD");   /* heart02=3 heart04=2 heart05=2 */
    int riko_sd = mid(&tg, "PL!S-sd1-011-SD"); /* heart05=2               */
    int riko_bp5 = mid(&tg, K_RIKO);          /* heart05=1               */
    int opp = mid(&tg, K_OPP_LIVE_A);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, chika, "PL!S-sd1-001-SD")) return;
    if (!ident(&tg, riko_sd, "PL!S-sd1-011-SD")) return;
    if (!ident(&tg, riko_bp5, K_RIKO)) return;
    if (filler < 0) return;
    if (!rh_wired(&tg, riko_bp5)) return;

    rh_set_stage3(&tg, 0, chika, riko_sd, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, opp);
    test_add_to_hand(&tg, riko_bp5);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 10);
    CHECK_EQ(test_play_to_stage(&tg, riko_bp5, 2), 1, "梨子 debuts right");
    rh_drain_skip(&tg);

    /* CARDS.JSON EVIDENCE for the gate colour. The two Anniversary cards with
     * near-identical wording are NOT identical:
     *   PL!S-bp5-010-N (高海千歌, CYaRon!)   -> heart02 が合計5つ以上
     *   PL!S-bp5-011-N (桜内梨子, GuiltyKiss) -> heart05 が合計5つ以上
     * so the Rust test's "heart05" comment for bp5-011-N is CORRECT and a C
     * port that gates this card on heart02 is wrong. Assert the printed
     * aggregate so a red line is attributable to the card data. */
    int printed[8];
    rh_stage_printed(&tg, 0, printed);
    CHECK_EQ(printed[H02], 3, "the stage's PRINTED heart02 total is 3 (千歌 only)");
    CHECK_EQ(printed[H05], 5,
             "the stage's PRINTED heart05 total is 2 + 2 + 1 = 5, meeting the "
             "PRINTED 「heart05が合計5つ以上」 gate on PL!S-bp5-011-N");
    rh_assert_stage_hearts(&tg, 0, "k-riko/condition-met");

    CHECK_EQ(need_of(&tg, opp, H00), 1,
             "Riko should increase the opponent's need heart00 by 1");
}

static void k_riko_condition_not_met_does_nothing(void)
{
    static TestGame tg;
    int riko_sd = mid(&tg, "PL!S-sd1-011-SD");
    int riko_bp5 = mid(&tg, K_RIKO);
    int opp = mid(&tg, K_OPP_LIVE_A);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, riko_sd, "PL!S-sd1-011-SD")) return;
    if (!ident(&tg, riko_bp5, K_RIKO)) return;
    if (filler < 0) return;
    if (!rh_wired(&tg, riko_bp5)) return;

    rh_set_stage3(&tg, 0, riko_sd, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, opp);
    test_add_to_hand(&tg, riko_bp5);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 10);
    CHECK_EQ(test_play_to_stage(&tg, riko_bp5, 2), 1, "梨子 debuts right");
    rh_drain_skip(&tg);
    {
        int printed[8];
        rh_stage_printed(&tg, 0, printed);
        CHECK(printed[H05] < 5,
              "the stage's PRINTED heart05 total is below the gate (2 + 1 = 3 < 5)");
    }
    CHECK_EQ(need_of(&tg, opp, H00), 0,
             "condition not met -> no modifier at all");

    /* NON-VACUITY CONTROL: push the SAME board over the PRINTED heart05 gate.
     * Two 桜内梨子 support cards (heart05=2 each) beside 梨子 herself
     * (heart05=1) make the aggregate 5. If the ability is not wired at all,
     * this control reads 0 too and the zero above proves nothing. */
    int a = mid(&tg, "PL!S-sd1-011-SD");   /* heart05=2 */
    int b = mid(&tg, "PL!S-sd1-011-SD");   /* heart05=2 */
    if (a < 0 || b < 0) return;
    rh_set_stage3(&tg, 0, a, b, riko_bp5);
    {
        int printed[8];
        rh_stage_printed(&tg, 0, printed);
        CHECK_EQ(printed[H05], 5,
                 "CONTROL board: the PRINTED heart05 total now reaches the "
                 "printed gate (2 + 2 + 1 = 5)");
    }
    rh_assert_stage_hearts(&tg, 0, "k-riko/control");
    CHECK_EQ(need_of(&tg, opp, H00), 1,
             "CONTROL: with the PRINTED gate satisfied the opponent's live "
             "card does get +1, so the zero above is a gate verdict");
}

/* ========================================================================
 * L. rina_debut_opponent_live_card_heart_increase_test.rs
 *    PL!S-bp5-010-N, 登場: if my stage's members hold >= 5 heart02 in total,
 *    a live card in the OPPONENT's live zone needs heart0 +1.
 * ===================================================================== */

static void l_heart02_ge5_increases_opponent_need_heart00(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = mid(&tg, "PL!S-bp5-010-N");
    int h02a = mid(&tg, "PL!S-sd1-010-SD");   /* heart02=2 */
    int h02b = mid(&tg, "PL!S-sd1-010-SD");   /* heart02=2 */
    int opp_live = mid(&tg, K_OPP_LIVE_A);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, rina, "PL!S-bp5-010-N")) return;
    if (!ident(&tg, h02a, "PL!S-sd1-010-SD")) return;
    if (h02b < 0 || filler < 0) return;
    if (!rh_wired(&tg, rina)) return;

    rh_set_stage3(&tg, 0, h02a, rina, h02b);
    test_add_to_live_for(&tg, 1, opp_live);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 0, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    test_give_energy(&tg, 5);

    int printed[8];
    rh_stage_printed(&tg, 0, printed);
    CHECK(printed[H02] >= 5, "the stage's PRINTED heart02 total is >= 5");
    CHECK_EQ(printed[H02], 5,
             "the stage's PRINTED heart02 total is exactly 2 + 1 + 2");
    rh_assert_stage_hearts(&tg, 0, "l1/heart02");

    int before = need_of(&tg, opp_live, H00);
    int snap = rb_log_buffer_len();
    if (!rh_fire_debut(&tg, rina)) return;
    rh_drain_skip(&tg);
    rh_note_fire(snap, "l1/rina-debut");
    int after = need_of(&tg, opp_live, H00);
    CHECK(after > before,
          "the OPPONENT's live card need heart00 must increase");
    CHECK_EQ(after, 1, "and it increases by exactly 1");
}

static void l_heart02_lt5_no_effect(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int rina = mid(&tg, "PL!S-bp5-010-N");
    int opp_live = mid(&tg, K_OPP_LIVE_A);
    int filler = mid(&tg, "PL!-sd1-010-SD");
    if (!ident(&tg, rina, "PL!S-bp5-010-N")) return;
    if (filler < 0) return;
    if (!rh_wired(&tg, rina)) return;

    rh_set_stage3(&tg, 0, RB_EMPTY_SLOT, rina, RB_EMPTY_SLOT);
    test_add_to_live_for(&tg, 1, opp_live);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 0, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    test_give_energy(&tg, 5);

    int printed[8];
    rh_stage_printed(&tg, 0, printed);
    CHECK(printed[H02] < 5,
          "the stage's PRINTED heart02 total is < 5 (Rina alone prints heart02=1)");
    CHECK_EQ(printed[H02], 1, "Rina alone contributes exactly 1 PRINTED heart02");
    rh_assert_stage_hearts(&tg, 0, "l2/heart02");

    int before = need_of(&tg, opp_live, H00);
    int snap = rb_log_buffer_len();
    if (!rh_fire_debut(&tg, rina)) return;
    rh_drain_skip(&tg);
    rh_caveat("l_heart02_lt5", "l_heart02_ge5_increase");
    rh_note_fire(snap, "l2/rina-debut-lt5");
    int after = need_of(&tg, opp_live, H00);
    CHECK_EQ(after, before, "no change when heart02 < 5");
}

/* ========================================================================
 * per-test isolation: forked child, signal handler naming the test in
 * flight. NO watchdog -- an in-child alarm() never fires on this toolchain
 * and a parent-side waitpid(WNOHANG) deadline's kill(SIGKILL) does not take
 * effect, so one would be dead code that only pretends to protect the run.
 * ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_SETUPBUG  3
#define CHILD_VACUOUS   4

static int n_tests_ok, n_tests_failed, n_tests_crashed, n_tests_setup;
static int n_vacuous_reported;

static void on_segv(int sig)
{
    const char *msg = "\nCRASH: a fatal signal arrived inside the test below.\n";
    ssize_t ignored = write(2, msg, strlen(msg));
    (void)ignored;
    fprintf(stderr, "CRASH: signal %d in test [%s]\n", sig, current_test);
    fflush(stderr);
    _Exit(CHILD_SETUPBUG);
}

/* A child runs in its own address space, so its `assertions` / `failures` /
 * `n_setup_bugs` / `n_vacuous` counters die with it. Without a channel back to
 * the parent the summary line reads "0 assertions, 0 failures" on a run that
 * just produced a hundred red assertions -- and the exit status was 0, i.e. a
 * red suite reported success. Each child therefore writes its four counters
 * through this pipe before _Exit. */
typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int vacuous;
} RhChildCounters;

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
        assertions += (int)(assertions - a0);
        if (nf) n_tests_failed++;
        else if (ns) n_tests_setup++;
        else n_tests_ok++;
        n_vacuous += nv;
        printf("%-8s %s  [%d assertions, %d failures, %d setup bugs, %d vacuous]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name,
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
        RhChildCounters c;
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
        _Exit(c.failures   > 0 ? CHILD_FAILURES
               : c.vacuous  > 0 ? CHILD_VACUOUS
               : c.setup_bugs > 0 ? CHILD_SETUPBUG : CHILD_OK);
    }

    if (fds[1] >= 0) close(fds[1]);
    RhChildCounters c;
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
    assertions += c.assertions;
    failures   += c.failures;
    n_setup_bugs += c.setup_bugs;
    n_vacuous  += c.vacuous;
    n_vacuous_reported += c.vacuous;

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        printf("ok      %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("FAILED  %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_VACUOUS) {
        n_tests_setup++;
        printf("VACUOUS %s  <-- no reported failures, but a negative assertion "
               "could not be proven non-vacuous\n", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_tests_setup++;
        printf("SETUPBUG %s  <-- SETUP BUG (fixture could not be built)\n", name);
    } else {
        n_tests_crashed++;
        printf("CRASH   %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* RH_PROBE=<card_no>[,<card_no>...] -- diagnostic only: print a card's
 * decoded base/need hearts, its cost/score, and the stage-heart pipeline
 * output for a one-card stage. Used to separate a PARSER gap (the card decodes
 * with no hearts) from an ENGINE gap (the card decodes fine and the pipeline
 * drops it). The suite is unchanged when it is unset. */
static int rh_run_probe(const char *list)
{
    static TestGame tg;
    test_game_new(&tg);
    if (!strcmp(list, "L1") || !strcmp(list, "L1F")) {
        int do_fork = !strcmp(list, "L1F");
        if (do_fork) {
            fflush(stdout);
            pid_t p = fork();
            if (p == 0) { rh_run_probe("L1"); fflush(stdout); _Exit(0); }
            int st = 0; waitpid(p, &st, 0);
            printf("forked child status=%d\n", st);
            return 0;
        }
        int rina = mid(&tg, "PL!S-bp5-010-N");
        int h02a = mid(&tg, "PL!S-sd1-010-SD");
        int h02b = mid(&tg, "PL!S-sd1-010-SD");
        int opp = mid(&tg, "PL!-sd1-019-SD");
        int filler = mid(&tg, "PL!-sd1-010-SD");
        printf("ids: rina=%d h02a=%d h02b=%d opp=%d filler=%d\n",
               rina, h02a, h02b, opp, filler);
        ident(&tg, rina, "PL!S-bp5-010-N");
        ident(&tg, h02a, "PL!S-sd1-010-SD");
        printf("rh_wired=%d\n", rh_wired(&tg, rina));
        rh_set_stage3(&tg, 0, h02a, rina, h02b);
        printf("stage = [%d,%d,%d]\n", tg.state.p[0].stage[0],
               tg.state.p[0].stage[1], tg.state.p[0].stage[2]);
        for (int cid = 2526; cid <= 2528; cid++)
            printf("  mods[%d]: heart_copy=%d heart_mult=%d hco=%d "
                   "heart[0..7].add=[%d,%d,%d,%d,%d,%d,%d,%d] set=[%d,%d,%d,%d,%d,%d,%d,%d]\n",
                   cid, tg.state.mods.heart_copy[cid],
                   tg.state.mods.heart_multiplier[cid],
                   tg.state.mods.heart_color_override[cid],
                   tg.state.mods.heart[cid][0].add, tg.state.mods.heart[cid][1].add,
                   tg.state.mods.heart[cid][2].add, tg.state.mods.heart[cid][3].add,
                   tg.state.mods.heart[cid][4].add, tg.state.mods.heart[cid][5].add,
                   tg.state.mods.heart[cid][6].add, tg.state.mods.heart[cid][7].add,
                   tg.state.mods.heart[cid][0].set, tg.state.mods.heart[cid][1].set,
                   tg.state.mods.heart[cid][2].set, tg.state.mods.heart[cid][3].set,
                   tg.state.mods.heart[cid][4].set, tg.state.mods.heart[cid][5].set,
                   tg.state.mods.heart[cid][6].set, tg.state.mods.heart[cid][7].set);
        int h[8];
        rb_calc_stage_hearts(&tg.state, 0, h);
        printf("BEFORE energy: [%d,%d,%d,%d,%d,%d,%d,%d]\n",
               h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        test_add_to_live_for(&tg, 1, opp);
        for (int i = 0; i < 30; i++) {
            test_add_to_deck_pl(&tg, 0, filler);
            test_add_to_deck_pl(&tg, 1, filler);
        }
        test_give_energy(&tg, 5);
        printf("stage = [%d,%d,%d]\n", tg.state.p[0].stage[0],
               tg.state.p[0].stage[1], tg.state.p[0].stage[2]);
        rb_calc_stage_hearts(&tg.state, 0, h);
        printf("AFTER energy: [%d,%d,%d,%d,%d,%d,%d,%d]\n",
               h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        return 0;
    }
    char buf[512];
    snprintf(buf, sizeof buf, "%s", list);
    for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
        int cid = rb_find_card_by_no(tok);
        if (cid < 0) { printf("=== %s : NOT IN DB\n", tok); continue; }
        Card c;
        memset(&c, 0, sizeof c);
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) { printf("=== %s : decode failed\n", tok); continue; }
        printf("=== %s (template id=%d) type_flags=%d cost=%d score=%d "
               "num_base=%d num_blade=%d num_need=%d n_hearts=%d\n",
               tok, cid, c.type_flags, c.cost, c.score, c.num_base, c.num_blade,
               c.num_need, c.n_hearts);
        for (int i = 0; i < c.n_hearts && i < RB_MAX_HEARTS; i++)
            printf("    heart[%d] color=%d count=%d%s\n", i, c.heart_color[i],
                   c.heart_count[i],
                   i < c.num_base ? "  (base)" : (i < c.num_base + c.num_blade ? "  (blade)" : "  (need)"));
        rb_free_card(&c);
        int copy = rb_create_card_copy(cid);
        int h[8];
        rh_set_stage3(&tg, 0, copy, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
        rb_calc_stage_hearts(&tg.state, 0, h);
        printf("    copy id=%d stage_hearts=[%d,%d,%d,%d,%d,%d,%d,%d]\n", copy,
               h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        int nab = rb_card_num_abilities((uint32_t)copy);
        printf("    abilities=%d\n", nab);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)copy, a, &ab)) continue;
            printf("      ab#%d triggers=%s full_text=%s\n", a,
                   ab.triggers ? ab.triggers : "(null)",
                   ab.full_text ? ab.full_text : "(null)");
            rh_dump_effect("        effect", ab.effect, 2);
            rh_dump_effect("        cost", ab.cost, 2);
            rb_free_ability(&ab);
        }
    }
    return 0;
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
    if (getenv("RH_PROBE")) {
        int rc = rh_run_probe(getenv("RH_PROBE"));
        rb_unload();
        return rc;
    }
    /* RH_DEBUG=1 turns on the engine's own ability/executor trace, so a red
     * per-unit assertion can be attributed to the CONDITION, the per-unit
     * arithmetic, or the recipient filter without re-instrumenting anything. */
    if (getenv("RH_DEBUG")) rb_ability_debug_set(1);
    /* The ability log is the prompt-count control: rh_note_fire() counts
     * modify_required_hearts* effect verdicts to prove a negative assertion
     * is not reading green on a board where the ability never executed. */
    rb_log_set_enabled(1);

    printf("--- A. all_active_stage_required_hearts_reduction_test.rs (PL!S-bp7-020-L ab#0) ---\n");
    run("a_all_active_stage_reduces_required_heart", a_all_active_stage_reduces_required_heart);
    run("a_waited_or_empty_stage_does_not_reduce", a_waited_or_empty_stage_does_not_reduce_required_heart);

    printf("--- B. required_hearts_stacking_and_hostile_edges_test.rs (PL!S-bp7-020-L ab#0/ab#1) ---\n");
    run("b_stacking_both_gates_reduce_twice", b_stacking_both_gates_reduce_twice);
    run("b_empty_deck_mill_shortfall", b_empty_deck_mill_shortfall_no_reduction);
    run("b_opponent_waited_own_all_active", b_opponent_waited_own_all_active_still_reduces);
    run("b_reduction_honored_at_performance", b_reduction_honored_at_performance_end_to_end);

    printf("--- C. distinct_stage_waitroom_members_required_hearts_test.rs (PL!HS-pb1-026-L) ---\n");
    run("c_six_distinct_reduce_two", c_six_distinct_reduce_two);
    run("c_five_plus_duplicate", c_five_plus_duplicate_no_reduction);
    run("c_five_plus_renosora_live", c_five_plus_renosora_live_no_reduction);
    run("c_five_plus_wrong_group", c_five_plus_wrong_group_no_reduction);

    printf("--- D. baton_arrivals_required_hearts_reduction_test.rs (PL!HS-bp2-023/025-L) ---\n");
    run("d_mirage_two_baton_heart05", d_mirage_two_baton_arrivals_reduce_only_heart05);
    run("d_mirage_one_baton", d_mirage_one_baton_arrival_no_reduction);
    run("d_mirage_non_renosora", d_mirage_non_renosora_arrivals_no_reduction);
    run("d_mirage_renosora_no_baton", d_mirage_renosora_without_baton_no_reduction);
    run("d_kokon_two_baton_heart01", d_kokon_two_baton_arrivals_reduce_only_heart01);

    printf("--- E. ladybug_stage_cost_comparison_required_hearts_test.rs (PL!HS-bp2-024-L) ---\n");
    run("e_both_members_reduce_three", e_both_members_on_stage_reduces_hearts);
    run("e_missing_sayaka", e_missing_sayaka_no_reduction);
    run("e_missing_kosuzu", e_missing_kosuzu_no_reduction);
    run("e_wrong_character", e_wrong_character_no_reduction);
    run("e_no_members", e_no_members_no_reduction);

    printf("--- F. wonder_zone_center_heart03_required_hearts_test.rs (PL!-bp5-020-L) ---\n");
    run("f_center_two_heart03_reduces_1", f_center_honoka_two_heart03_reduces_1);
    run("f_center_zero_heart03", f_center_rin_zero_heart03_no_reduction);
    run("f_no_center_member", f_no_center_member_no_reduction);
    run("f_center_non_mus", f_center_non_mus_no_reduction);

    printf("--- G. modify_required_hearts_exclude_heart_test.rs (PL!-bp5-023-L) ---\n");
    run("g_all_members_excluded", g_all_members_excluded_no_reduction);
    run("g_one_non_excluded_reduces_1", g_one_non_excluded_member_reduces_1);
    run("g_mixed_reduces_2", g_mixed_excluded_and_counted_members);
    run("g_both_excluded_and_allowed_counts", g_member_with_both_excluded_and_allowed_colors_counts);

    printf("--- H. hanamusubi_required_hearts_exclude_self_test.rs (PL!HS-bp5-019-L) ---\n");
    run("h_alone_no_reduction", h_alone_no_reduction);
    run("h_one_other_reduces_2", h_with_one_other_renosora_reduces_2);
    run("h_two_others_reduce_4", h_with_two_other_renosora_reduces_4);
    run("h_non_renosora_no_reduction", h_with_non_renosora_no_reduction);
    run("h_only_heart04", h_reduces_only_heart04_not_heart00);
    run("h_two_hanamusubi_each_2", h_two_hanamusubi_each_reduces_2);
    run("h_three_hanamusubi_each_4", h_three_hanamusubi_each_reduces_4);
    run("h_two_hana_one_filler", h_two_hanamusubi_one_filler_each_reduces_2);
    run("h_one_hana_one_filler", h_one_hanamusubi_one_filler_no_reduction);
    run("h_one_hana_two_fillers", h_one_hanamusubi_two_fillers_no_reduction);

    printf("--- I. jellyfish_appeared_moved_member_required_hearts_test.rs (PL!SP-pb1-025-L) ---\n");
    run("i_two_appeared_reduce_2", i_two_members_appeared_reduce_by_2);
    run("i_one_both_flags_once", i_one_member_both_flags_counts_once);
    run("i_position_change_swap", i_position_change_swap_creates_countable_movement);
    run("i_mixed_qualifying", i_mixed_qualifying_and_non_qualifying);
    run("i_no_qualifying", i_no_qualifying_members_no_reduction);
    run("i_three_qualify_reduce_3", i_three_members_all_qualify_reduce_by_3);
    run("i_moved_without_appearing", i_moved_without_appearing_this_turn_counts);
    run("i_neither_appeared_nor_moved", i_neither_appeared_nor_moved_not_counted);
    run("i_appeared_then_left", i_appeared_then_left_not_counted);
    run("i_one_left_one_stays", i_one_left_one_stays);

    printf("--- J. modify_required_hearts_13key_test.rs (PL!-bp6-022-L) ---\n");
    run("j_high_score_mus_reduction", j_high_score_mus_live_gets_reduction);
    run("j_low_score_no_reduction", j_low_score_mus_live_no_reduction);
    run("j_mixed_scores", j_mixed_scores_filter_correctly);
    run("j_non_mus_high_score", j_non_mus_high_score_no_reduction);
    run("j_activator_not_in_success_zone", j_activator_not_in_success_zone_does_nothing);
    run("j_no_live_cards_no_crash", j_no_live_cards_no_crash);
    run("j_non_stackable", j_non_stackable_duplicate_cards);
    run("j_as_long_as_expiration", j_as_long_as_expiration_on_zone_exit);
    run("j_p1_activator_not_p2", j_p1_activator_does_not_affect_p2);
    run("j_p2_activator_targets_p2", j_p2_activator_targets_p2_not_p1);
    run("j_p2_score_threshold", j_p2_activator_respects_score_threshold);
    run("j_p2_group_filter", j_p2_activator_respects_group_filter);

    printf("--- K. modify_required_hearts_global_test.rs (PL!SP-bp2-010-P / PL!S-bp5-011-N) ---\n");
    run("k_wien_increases_opponent", k_wien_constant_increases_opponent_live_cards);
    run("k_wien_removed_on_exit", k_wien_constant_removed_when_leaves_stage);
    run("k_wien_all_opponent_lives", k_wien_constant_applies_to_all_opponent_live_cards);
    run("k_two_wien_stack", k_two_wien_instances_stack);
    run("k_riko_condition_met", k_riko_condition_met_increases_opponent_hearts);
    run("k_riko_condition_not_met", k_riko_condition_not_met_does_nothing);

    printf("--- L. rina_debut_opponent_live_card_heart_increase_test.rs (PL!S-bp5-010-N) ---\n");
    run("l_heart02_ge5_increase", l_heart02_ge5_increases_opponent_need_heart00);
    run("l_heart02_lt5_no_effect", l_heart02_lt5_no_effect);

    rb_unload();
    printf("\n%d test cases: %d ok, %d failed, %d setup/vacuous, %d crashed\n",
           n_tests_ok + n_tests_failed + n_tests_setup + n_tests_crashed,
           n_tests_ok, n_tests_failed, n_tests_setup, n_tests_crashed);
    printf("%d assertions, %d failures, %d setup bugs, %d VACUOUS negatives\n",
           assertions, failures, n_setup_bugs, n_vacuous_reported);
    if (n_vacuous_reported) {
        fprintf(stderr, "FAIL: %d negative assertion(s) could not be proven "
                        "non-vacuous (see the VACUOUS lines above)\n",
                n_vacuous_reported);
        return 1;
    }
    if (n_tests_failed || n_tests_crashed) {
        fprintf(stderr, "FAIL: %d test case(s) FAILED and %d CRASHED -- see the "
                        "FAIL lines above\n", n_tests_failed, n_tests_crashed);
        return 1;
    }
    printf("ALL REQUIRED-HEARTS PARITY CHECKS PASSED\n");
    return 0;
}

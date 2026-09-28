/* test_parity_conditional_gated.c — C parity suite for the Rust clusters
 * engine/tests/test_modules/effects/conditional/per_card/
 * engine/tests/test_modules/effects/conditional/state_gated/
 * engine/tests/test_modules/effects/conditional/joint/
 *
 * (effects/conditional/zone_source/ is a peer's, in test_parity_zone_source.c.)
 *
 * These are the "gated" conditionals: an effect whose BRANCH is selected by a
 * condition rather than by a prompt. Three distinct gate shapes appear here:
 *
 *   1. POSITION gates — 「左サイド」 on a 起動's activation_position
 *      (position_gated_left_side_cost_reduction_test.rs), and 「センターエリアに
 *      いるメンバーが最も大きいコスト」 on a 常時 heart.
 *   2. RESULT conditions — conditional_on_result whose result_condition carries
 *      a `characters` list, so the follow-up only fires when the card THIS
 *      effect just moved has a named character (chika / hanamaru / ginako).
 *   3. ZONE-FILTERED result conditions — the same shape, but the gate reads the
 *      card that reached the WAITROOM by the cost rather than one that reached
 *      hand (kotori, kanon).
 *
 * ── NON-VACUITY, which is the whole point of this file ────────────────────
 * A conditional that never fires makes BOTH arms of every test pass. Every
 * negative assertion below is therefore preceded by a control that proves the
 * ability actually RAN (a prompt appeared, a card actually moved, the discard
 * zone really changed). Where the Rust twin has NO such control, this port adds
 * a paired POSITIVE case on the same board with only the condition satisfied,
 * so "the condition is the thing that decided" is measured, not assumed.
 * Controls are labelled "CONTROL:" in the assertion text.
 *
 * ── card identity ────────────────────────────────────────────────────────
 * Every fixture is pinned with pin_id() (a byte-exact strcmp of the DECODED
 * card_no — rb_card_no_eq is fuzzy and a peer proved it accepts
 * PL!SP-PR-004-R against the real PL!SP-PR-004-PR). Every printed fact a test
 * leans on (cost, blade heart, score, live-vs-member) is re-asserted against
 * the ENGINE's decoded Card, not just against cards.json, so a cards.json /
 * cards.bin disagreement cannot hide.
 *
 * ── fork isolation ───────────────────────────────────────────────────────
 * Every case runs in a forked child; a child that dies on a signal is reported
 * as CRASH, never as a pass, and the handler names the test in flight. No
 * watchdog: this toolchain cannot deliver one.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN, bytes EF BC 8B. cards.json only carries the
 * fullwidth form, so PL!SP-bp5-002-R＋ is a DIFFERENT card number from
 * PL!SP-bp5-002-R. Spelled as an explicit UTF-8 byte escape so this file stays
 * 7-bit clean under the MSVC/MinGW toolchains (see the encoding trap in
 * AGENTS.md). */
#define PLUS "\xef\xbc\x8b"

/* GREEK SMALL LETTER MU (U+03BC) — the group name 『μ's』, bytes CE BC. */
#define MU_GROUP "\xce\xbc's"

/* The 虹ヶ咲 group's name as the ENGINE actually spells it. rb_card_group_name()
 * returned bytes E8 99 B9 E3 83 B6 E5 92 B2 at runtime — note the first
 * character's third byte is B9, not the A6 a hand-typed U+8671 would give, so
 * the literal is pinned to the observed bytes rather than to the glyph. */
#define NIJI_GROUP "\xe8\x99\xb9\xe3\x83\xb6\xe5\x92\xb2"

static int failures;
static int assertions;
static int harness_gaps;
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
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A C-vs-Rust HARNESS gap: the Rust assertion is not expressible through the C
 * shim at all (no API surface, not a behaviour). Counted separately from a
 * parity failure so a red run still names the parity failures. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

#define DIAG(...) do { fprintf(stderr, "        (diagnostic: "); \
                       fprintf(stderr, __VA_ARGS__); \
                       fprintf(stderr, ")\n"); } while (0)

/* =======================================================================
 * card constants — every one verified against cards/cards.json
 * ======================================================================= */

/* per_card/chika_debut_result_character_condition_test.rs — 高海千歌 */
#define CHIKA       "PL!S-bp7-001-R"    /* cost 9  CYaRon!  blade 3  */
#define RIKO        "PL!S-bp3-002-R"    /* 桜内梨子   cost 11 GuiltyKiss */
#define YO          "PL!S-bp7-005-R" PLUS /* 渡辺 曜   cost 15 CYaRon!    */
#define HONOKA_SD   "PL!-sd1-001-SD"    /* 高坂穂乃果 cost 11 Printemps  */
#define FILLER_4    "PL!-sd1-010-SD"    /* 高坂穂乃果 cost 4  Printemps  */
#define FILLER_4B   "PL!-sd1-011-SD"    /* 絢瀬 絵里  cost 4  BiBi        */
#define FILLER_15   "PL!-sd1-009-SD"    /* 矢澤 にこ  cost 15 BiBi         */

/* per_card/ginako_live_start_discard_group_blade_count_test.rs — 百生吟子 */
#define GINAKO      "PL!HS-bp6-004-R"   /* cost 13 スリーズブーケ blade 2 b_heart05 */
#define OPP_MEMBER  "PL!-sd1-005-SD"    /* 星空 凛    cost 2  lilywhite   */
#define MUS_LIVE    "PL!-sd1-020-SD"    /* きっと青春が聞こえる score 2    */

/* per_card/hanamaru_debut_result_character_deploy_test.rs — 国木田花丸 */
#define HANAMARU    "PL!S-bp7-007-R" PLUS /* cost 11 AZALEA blade 1       */
#define TSUSHIMA    "PL!S-sd1-015-SD"   /* 津島善子   cost 2  GuiltyKiss b_heart05 */
#define RUBY        "PL!S-bp2-009-R"    /* 黒澤ルビィ cost 2  CYaRon!  b_heart02  */
#define ERI_NONMATCH "PL!-sd1-002-SD"   /* 絢瀬 絵里  cost 2  BiBi      b_heart06 */
#define TOO_EXPENSIVE "PL!-sd1-014-SD"  /* 星空 凛    cost 9  lilywhite  NO b_heart */

/* per_card/kanon_debut_invalidate_recovers_discard_card_test.rs — 澁谷かのん */
#define KANON_DEBUT "PL!SP-bp2-001-R" PLUS /* cost 13 CatChu! blade 3    */
#define KANON_TGT   "PL!SP-sd1-003-SD"     /* 嵐 千砂都 cost 15 5yncri5e!  */
#define KANON_REC1  "PL!SP-sd1-001-SD"     /* 澁谷かのん cost 11 CatChu!   */
#define KANON_REC2  "PL!SP-sd1-002-SD"     /* 唐 可可   cost 11 KALEIDOSCORE */
#define LIVE_SC1    "PL!-sd1-019-SD"       /* START:DASH!! score 1        */

/* per_card/position_gated_left_side_cost_reduction_test.rs — 唐 可可 */
#define KEKE_LEFT   "PL!SP-bp5-002-R" PLUS /* 唐 可可 cost 13 KALEIDOSCORE  */

/* per_card/wien_live_start_empty_hand_energy_placement_test.rs — ウィーン */
#define WIEN        "PL!SP-pb2-010-R"     /* cost 11 KALEIDOSCORE blade 5  */
#define ENERGY      "LL-E-001-SD"

/* state_gated/center_highest_cost_constant_heart_*_test.rs — 平安名すみれ */
#define SUMIRE_R    "PL!SP-bp2-004-R"    /* cost 9  CatChu!  NO blade heart */
#define SUMIRE_P    "PL!SP-bp2-004-P"    /* cost 9  CatChu!  NO blade heart */
#define COST_15     "PL!-sd1-009-SD"     /* 矢澤 にこ cost 15 BiBi         */
#define COST_11     "PL!SP-pb1-001-R"    /* 澁谷かのん cost 11 CatChu!     */
#define COST_10     "PL!HS-PR-001-PR"    /* 日野下花帆 cost 10 スリーズブーケ */
#define COST_9      "PL!-PR-005-PR"      /* 星空 凛 cost 9 lilywhite        */
#define COST_4      "PL!-sd1-010-SD"     /* 高坂穂乃果 cost 4 Printemps     */

/* state_gated/deployed_member_blade_heart_wait_test.rs — 近江彼方 */
#define KONATA      "PL!N-bp4-006-R"     /* cost 11 QU4RTZ b_heart06      */
#define KONATA_BH   "PL!N-sd1-006-SD"     /* 近江彼方 cost 2 QU4RTZ b_heart06 */
#define KONATA_NOBH "PL!N-bp1-004-R"     /* 朝香果林 cost 4 DiverDiva NO b_heart */
#define KONATA_N    "PL!N-bp4-018-N"     /* 近江彼方 cost 2 QU4RTZ b_heart03 */

/* state_gated/discard_group_look_four_or_recover_live_test.rs — 南 ことり */
#define KOTORI      "PL!-bp5-003-R" PLUS /* cost 11 Printemps blade 4     */
#define HONOKA_BP6  "PL!-bp6-001-R" PLUS /* 高坂穂乃果 cost 2 Printemps    */
#define RANJU       "PL!N-bp1-012-R" PLUS /* 鐘 嵐珠 cost 15 R3BIRTH (not μ's) */

/* joint/joint_six_discard_mixed_name_cost_test.rs */
#define JOINT_BP3   "LL-bp3-001-R" PLUS   /* 園田海未&津島善子&天王寺璃奈 cost 20 */
#define UMI_PR      "PL!-PR-014-PR"       /* 園田海未 cost 2 lilywhite     */
#define YOSHIKO_SD  "PL!S-sd1-015-SD"     /* 津島善子 cost 2 GuiltyKiss    */
#define RINO_N      "PL!N-bp7-021-N"      /* 天王寺璃奈 cost 2 QU4RTZ       */

/* =======================================================================
 * harness
 * ======================================================================= */

/* rb_load("src") fails under the isolated out-of-tree build root, so the
 * canonical cards/build directory is the required fallback. */
static int load_card_db(void)
{
    static const char *const dirs[] = {"src", "../cards/build", "cards/build",
                                       "../../cards/build"};
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++)
        if (rb_load(dirs[i]) == 0) return 0;
    return 1;
}

/* Rotating buffers: dump_board prints three card numbers in ONE fprintf, and a
 * single static buffer made all three print the LAST value — which reads as
 * "the same card is in every stage slot" and hides the real board. */
static const char *card_no_of(int cid)
{
    static char bufs[8][64];
    static int  next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 64, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* The identity rule (AGENTS.md): a fixture that silently resolved to a DIFFERENT
 * print invalidates every assertion that follows, so the resolved card_no is
 * pinned byte-exactly against what the Rust test names. */
static int pin_id(int cid, const char *want)
{
    assertions++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', resolved '%s' (id=%d)\n",
                want, card_no_of(cid), cid);
        failures++;
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

static int has_blade_heart(int cid)
{
    Card c;
    int r = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        r = rb_card_has_blade_heart(&c);
        rb_free_card(&c);
    }
    return r;
}

static int printed_cost(int cid)
{
    Card c;
    int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.cost; rb_free_card(&c); }
    return s;
}

static int printed_score(int cid)
{
    Card c;
    int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.score; rb_free_card(&c); }
    return s;
}

static int revealed_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_revealed; i++)
        if (tg->state.revealed_cards[i] == cid) return 1;
    return 0;
}

/* rb_card_group_name is the ONLY correct way to read a card's group
 * (rb_card_string(card->group_idx) resolves to "" for every cards.bin record). */
static int is_niji_group(int cid)
{
    const char *g = rb_card_group_name(cid);
    if (g)
        DIAG("group_name of %s = '%s' bytes=%02x %02x %02x %02x %02x %02x %02x %02x",
             card_no_of(cid), g,
             (unsigned char)g[0], (unsigned char)g[1], (unsigned char)g[2],
             (unsigned char)g[3], (unsigned char)g[4], (unsigned char)g[5],
             (unsigned char)g[6], (unsigned char)g[7]);
    return g && !strcmp(g, NIJI_GROUP);
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void pick(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

static void pick_n(TestGame *tg, const int *idx, int n)
{
    if (pending(tg)) rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust select_indices(&[]) — a real DECLINE, not a no-op. */
static void skip_pick(TestGame *tg)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, -1);
}

/* Rust drain_auto_ability_choices: SelectAutoAbility -> [0], anything else -> [] */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 32) {
        const char *t = pending_kind(tg);
        if (t && !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
        else skip_pick(tg);
    }
}

/* Rust's TestGame::drain_auto_ability_choices only answers SelectAutoAbility
 * prompts and LEAVES every other prompt standing. */
static void drain_only_auto(TestGame *tg) { test_drain_auto_choices(tg); }

static int drain_pick0(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { pick(tg, 0); n++; }
    return n;
}

/* The C queue opens 登場/起動 with a "shall this ability run?" SelectAutoAbility
 * step BEFORE the ability's own cost. Proceed that one, decline everything else
 * — declining the auto-ability refuses the WHOLE ability. */
static int drain_answering(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const char *t = pending_kind(tg);
        if (t && !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
        else pick(tg, 0);
        n++;
    }
    return n;
}

static int blade_mod(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }
static int cost_mod(TestGame *tg, int cid)  { return test_get_cost_modifier(tg, cid); }

/* The mods heart[] array is indexed by the PRINTED heart number
 * (0=Heart00, 1=Heart01, … 6=Heart06, 7=All). test_get_heart_modifier REMAPS
 * colour 5, so read the store directly. */
#define HEART03_IDX 3
static int heart03(TestGame *tg, int cid)
{
    return rb_mods_get_heart(&tg->state.mods, cid, HEART03_IDX);
}

static const char *orientation_of(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static int is_wait(TestGame *tg, int cid)
{
    const char *o = orientation_of(tg, cid);
    return o && !strcmp(o, "wait");
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0; P->live.n = 0;
    P->success.n = 0; P->energy.n = 0; P->energy_active = 0;
    P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

/* Rust: main_deck.cards.push(x) — APPEND, so the first pushed card is the LAST
 * card drawn. */
static void push_deck_top(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++) {
        if (P->deck.n >= RB_MAX_ZONE) return;
        P->deck.cards[P->deck.n++] = cid;
    }
}

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

/* Play to stage, answering a play-time alt-cost gate (rb_play_member pauses via
 * ptc_active). Rust's TestGame::play_to_stage declines it, so index 0 unless
 * `accept`. */
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

/* Rust helpers in the live-start tests: 5 passes reach LiveCardSetP1, the live
 * card is set, then 2 more passes fire the ライブ開始時 triggers. */
static void advance_to_live_set(TestGame *tg) { for (int i = 0; i < 5; i++) test_pass(tg); }
static void finish_live_setup(TestGame *tg)   { test_pass(tg); test_pass(tg); }

/* -- diagnostics --------------------------------------------------------- */
static void dump_zone(const char *label, const RbBag *bag)
{
    fprintf(stderr, "        %-10s n=%d:", label, bag->n);
    for (int i = 0; i < bag->n && i < 16; i++)
        fprintf(stderr, " [%d]=%s", i, card_no_of(bag->cards[i]));
    fprintf(stderr, "\n");
}

static void dump_board(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    fprintf(stderr, "        board p1: stage=[%s,%s,%s] wait=[%d,%d,%d] "
            "active_energy=%d energy_cards=%d edeck=%d\n",
            card_no_of(P->stage[0]), card_no_of(P->stage[1]), card_no_of(P->stage[2]),
            P->stage_wait[0], P->stage_wait[1], P->stage_wait[2],
            P->energy_active, P->energy.n, P->energy_deck.n);
    dump_zone("hand", &P->hand);
    dump_zone("deck", &P->deck);
    dump_zone("waitroom", &P->discard);
    dump_zone("energy", &P->energy);
    dump_zone("edeck", &P->energy_deck);
    dump_zone("live", &P->live);
    dump_zone("success", &P->success);
    fprintf(stderr, "        invalidations n=%d\n", tg->state.n_ability_invalidations);
    for (int i = 0; i < tg->state.n_ability_invalidations; i++)
        fprintf(stderr, "          [%d] card=%s trigger=%s dur=%s\n", i,
                card_no_of(tg->state.ability_invalidations[i].card_id),
                tg->state.ability_invalidations[i].trigger,
                tg->state.ability_invalidations[i].duration);
}

static void dump_effect(const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int i = 0; i < depth; i++) fputs("  ", stderr);
    fprintf(stderr, "[%d] action=%s src=%s dst=%s target=%s type=%s count=%d opt=%d",
            depth, e->action ? e->action : "-", e->source ? e->source : "-",
            e->destination ? e->destination : "-", e->target ? e->target : "-",
            e->card_type_field[0] ? e->card_type_field : "-", e->count, e->is_optional);
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, " [%s=%s]", e->extra_k[i] ? e->extra_k[i] : "?",
                e->extra_v[i] ? e->extra_v[i] : "?");
    fprintf(stderr, "\n");
    dump_effect(e->primary_effect, depth + 1);
    dump_effect(e->alternative_effect, depth + 1);
    dump_effect(e->followup_action, depth + 1);
    dump_effect(e->optional_action, depth + 1);
    dump_effect(e->conditional_action, depth + 1);
    dump_effect(e->look_action, depth + 1);
    dump_effect(e->select_action, depth + 1);
    dump_effect(e->gained_effect, depth + 1);
    dump_effect(e->resource_on_select, depth + 1);
    dump_effect(e->opponent_action, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_effect(e->options[i], depth + 1);
}

static void dump_cond(const Condition *c, int depth)
{
    if (!c) return;
    for (int i = 0; i < depth; i++) fputs("  ", stderr);
    fprintf(stderr, "COND variant=%u fields=%u\n", c->variant, c->n_fields);
    for (uint32_t i = 0; i < c->n_fields && i < 24; i++) {
        const CondField *f = &c->fields[i];
        for (int k = 0; k <= depth; k++) fputs("  ", stderr);
        fprintf(stderr, "%s =", f->key ? f->key : "?");
        if (f->v.tag == 0 && f->v.s) fprintf(stderr, " '%s'", f->v.s);
        else if (f->v.tag == 0) fprintf(stderr, " %lld", (long long)f->v.i);
        if (f->v.arr && f->v.arr_n) {
            fprintf(stderr, " [");
            for (uint32_t k = 0; k < f->v.arr_n && k < 8; k++)
                fprintf(stderr, "%s'%s'", k ? ", " : "",
                        f->v.arr[k].s ? f->v.arr[k].s : "?");
            fprintf(stderr, "]");
        }
        fprintf(stderr, "\n");
    }
    for (uint32_t i = 0; i < c->n_fields && i < 24; i++)
        if (c->fields[i].v.cond) dump_cond(c->fields[i].v.cond, depth + 1);
}

static void dump_ability(int card_id, int idx)
{
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)card_id, idx, &ab)) {
        fprintf(stderr, "        (ability %d of %s did not decode)\n", idx, card_no_of(card_id));
        return;
    }
    fprintf(stderr, "        ability %d of %s: triggers=%s use_limit=%d n_abilities=%d\n",
            idx, card_no_of(card_id), ab.triggers ? ab.triggers : "-", ab.use_limit,
            rb_card_num_abilities((uint32_t)card_id));
    if (ab.cost)   { fprintf(stderr, "          COST:\n");   dump_effect(ab.cost, 3); }
    if (ab.cost && ab.cost->condition) {
        fprintf(stderr, "          COST_CONDITION:\n");
        dump_cond(ab.cost->condition, 3);
    }
    if (ab.effect) { fprintf(stderr, "          EFFECT:\n"); dump_effect(ab.effect, 3); }
    if (ab.effect && ab.effect->condition) {
        fprintf(stderr, "          EFFECT_CONDITION:\n");
        dump_cond(ab.effect->condition, 3);
    }
    if (ab.effect && ab.effect->activation_condition) {
        fprintf(stderr, "          ACTIVATION_CONDITION:\n");
        dump_cond(ab.effect->activation_condition, 3);
    }
    if (ab.effect && ab.effect->result_condition) {
        fprintf(stderr, "          RESULT_CONDITION:\n");
        dump_cond(ab.effect->result_condition, 3);
    }
    if (ab.effect && ab.effect->alternative_condition) {
        fprintf(stderr, "          ALT_CONDITION:\n");
        dump_cond(ab.effect->alternative_condition, 3);
    }
    ab.cost = NULL; ab.effect = NULL;
    rb_free_ability(&ab);
}

static void dump_choice(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (!ch) { DIAG("no pending choice"); return; }
    DIAG("choice: type=%s zone=%s card_type=%s count=%d allow_skip=%d "
         "target=%s group=%s actor=%d filtered=%d",
         pending_kind(tg), ch->zone, ch->card_type[0] ? ch->card_type : "-", ch->count,
         ch->allow_skip, ch->target[0] ? ch->target : "-",
         ch->filter_group[0] ? ch->filter_group : "-", ch->actor, ch->n_filtered_indices);
    for (int i = 0; i < ch->n_characters; i++)
        fprintf(stderr, "          characters[%d]=%s\n", i, ch->characters[i]);
}

/* The C analogue of Rust's
 *   TurnEngine::execute_main_phase_action_with_ability_index(UseAbility, .., Some(i))
 * for a card carrying SEVERAL 起動 abilities. rb_activate_card walks every 起動
 * the card prints, so rb_resolve_ability is the only way to drive ONE. */
static int activate_ability_index(TestGame *tg, int card_id, int ability_idx)
{
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) return -1;
    int resolved = 0;
    int rc = rb_resolve_ability(&tg->state, 0, &ab, ability_idx, card_id, &resolved);
    rb_drain_ability_queue(&tg->state);
    ab.cost = NULL;
    ab.effect = NULL;
    rb_free_ability(&ab);
    return rc;
}

/* =======================================================================
 * A. joint/joint_six_discard_mixed_name_cost_test.rs (3 tests)
 *    LL-bp3-001-R＋ 園田海未&津島善子&天王寺璃奈, 起動/ターン1回:
 *    「自分の控え室から、『園田海未』か『津島善子』か『天王寺璃奈』のカードを
 *    6枚，山札の下に戻し，エネルギーANDED…」→ activate up to 6 energy.
 *
 *    Coverage note: test_tp_joint_card_live_start.c covers this card's OTHER
 *    ability (ab#1, ライブ開始時 pay 6E → blade+3) and test_parity_deploy_recover.c
 *    names LL-bp3-001-R＋ only in a #define it never drives. Nothing in the C
 *    corpus drives the 6-from-waitroom COST, so the survey's "joint is fully
 *    covered" card signal is a FALSE POSITIVE and all three tests are ported.
 *
 *    Q165: any combination totalling 6 works (a multi-name card is not
 *    required). Q62: the joint card has individual names.
 * ======================================================================= */

/* < 6 matching cards: the cost cannot be validated, so no prompt opens and the
 * waitroom is untouched. */
static void joint_cost_requires_six_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int joint = test_new_id(&tg, JOINT_BP3);
    pin_id(joint, JOINT_BP3);
    if (joint < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, joint, RB_EMPTY_SLOT);
    test_give_energy(&tg, 5);
    /* Only THREE cards in the waitroom — but make them MATCHING cards, so the
     * < 6 count is the sole reason the cost cannot be paid. (The Rust twin
     * pushes a non-matching 高坂穂乃果, which would fail for two reasons at
     * once and so cannot distinguish them.) */
    for (int i = 0; i < 3; i++) test_add_to_discard(&tg, test_new_id(&tg, UMI_PR));
    CHECK_EQ(tg.state.p[0].discard.n, 3, "A CONTROL: three cards really in the waitroom");

    test_activate_ability(&tg, joint);
    drain_answering(&tg, 8);
    dump_board(&tg);
    CHECK(!pending(&tg), "A: no choice prompt with < 6 matching cards");
    CHECK_EQ(tg.state.p[0].discard.n, 3, "A: the three cards remain in the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "A: no card reached the deck");
}

/* Exactly 6 matching cards, MIXED names: the cost resolves without a prompt and
 * the waitroom empties into the deck. */
static void joint_cost_accepts_mixed_six_card_names(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int joint = test_new_id(&tg, JOINT_BP3);
    int umi     = test_new_id(&tg, UMI_PR);
    int yoshiko = test_new_id(&tg, YOSHIKO_SD);
    int rino    = test_new_id(&tg, RINO_N);
    pin_id(joint, JOINT_BP3);
    pin_id(umi, UMI_PR);
    pin_id(yoshiko, YOSHIKO_SD);
    pin_id(rino, RINO_N);
    CHECK(umi != yoshiko && yoshiko != rino && umi != rino,
          "A CONTROL: the three names are three DISTINCT card instances");
    if (joint < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, joint, RB_EMPTY_SLOT);
    for (int i = 0; i < 2; i++) {
        test_add_to_discard(&tg, umi);
        test_add_to_discard(&tg, yoshiko);
        test_add_to_discard(&tg, rino);
    }
    CHECK_EQ(tg.state.p[0].discard.n, 6, "A CONTROL: six cards really start in the waitroom");

    test_activate_ability(&tg, joint);
    int n_prompts = 0, guard = 0;
    while (pending(&tg) && guard++ < 16) {
        n_prompts++;
        pick(&tg, 0);
    }
    drain_answering(&tg, 16);
    dump_board(&tg);

    CHECK_EQ(n_prompts, 0,
             "A: exactly six valid mixed-name cards resolve with NO prompt (Q165)");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "A: the waitroom empties");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "A: all six reached the deck");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "deck", umi), 2, "A: 2 × 園田海未");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "deck", yoshiko), 2, "A: 2 × 津島善子");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "deck", rino), 2, "A: 2 × 天王寺璃奈");
}

/* 7 matching cards > count 6: the cost must open a 6-of-N selection. */
static void joint_cost_creates_choice_with_seven_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int joint = test_new_id(&tg, JOINT_BP3);
    int umi    = test_new_id(&tg, UMI_PR);
    pin_id(joint, JOINT_BP3);
    pin_id(umi, UMI_PR);
    if (joint < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, joint, RB_EMPTY_SLOT);
    test_give_energy(&tg, 5);
    for (int i = 0; i < 7; i++) test_add_to_discard(&tg, umi);
    CHECK_EQ(tg.state.p[0].discard.n, 7, "A CONTROL: seven matching cards really in the waitroom");

    test_activate_ability(&tg, joint);
    CHECK(pending(&tg), "A: 7 matching cards > count 6 -> a selection prompt opens");
    dump_choice(&tg);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (ch) {
        CHECK_EQ(ch->count, 6, "A: the prompt asks for exactly 6 of the 7");
        CHECK(ch->zone && !strcmp(ch->zone, "discard"),
              "A: the selection pool is the WAITROOM (RbChoice spells it \"discard\")");
    } else {
        CHECK(0, "A: a pending choice is readable");
    }
}

/* PAIRED POSITIVE CONTROL for the two negative cases above: with 6 MATCHING
 * cards the cost is actually paid, so "< 6 -> refused" cannot be passing because
 * the ability is a permanent no-op on this card. */
static void joint_positive_control_six_matching_is_accepted(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int joint = test_new_id(&tg, JOINT_BP3);
    int umi    = test_new_id(&tg, UMI_PR);
    pin_id(joint, JOINT_BP3);
    if (joint < 0) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, joint, RB_EMPTY_SLOT);
    for (int i = 0; i < 6; i++) test_add_to_discard(&tg, umi);
    CHECK_EQ(tg.state.p[0].discard.n, 6, "A CONTROL: six matching cards in the waitroom");
    test_activate_ability(&tg, joint);
    int guard = 0;
    while (pending(&tg) && guard++ < 16) pick(&tg, 0);
    drain_answering(&tg, 16);
    dump_board(&tg);
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "A CONTROL: with SIX matching cards the cost really is paid");
    CHECK_EQ(tg.state.p[0].deck.n, 6,
             "A CONTROL: the six cards really reached the deck (the effect is not a no-op)");
}

/* =======================================================================
 * B. per_card/position_gated_left_side_cost_reduction_test.rs (3 tests)
 *    PL!SP-bp5-002-R＋ 唐 可可: 起動/左サイド/ターン1回 — the ability may only
 *    be activated from the LEFT area (activation_position "left_side").
 *
 *    Coverage note: test_parity_zone_source.c section H ports the SAME card and
 *    the SAME 起動, but every one of its cases STAGES HER AT stage[0] and so
 *    never drives the refusal. The two refusal arms are the only part of this
 *    file not already in the C corpus, and they are what make the gate a gate.
 * ======================================================================= */

static int keke_activate_from_area(TestGame *tg, int keke, int area)
{
    static const int fillers_seed = 0;
    (void)fillers_seed;
    RbPlayer *P = &tg->state.p[0];
    /* Rust stages [card, f1, f2] / [f, card, -1] / [f, -1, card] so the other two
     * areas are occupied in the CENTRE and RIGHT cases but empty in the LEFT
     * one. Reproduce that exactly. */
    if (area == 0) set_stage(tg, 0, keke, test_id(tg, FILLER_4),
                             test_new_id(tg, FILLER_4));
    else if (area == 1) set_stage(tg, 0, test_id(tg, FILLER_4), keke, RB_EMPTY_SLOT);
    else set_stage(tg, 0, test_id(tg, FILLER_4), RB_EMPTY_SLOT, keke);
    P->hand.n = 0;
    return test_activate_ability(tg, keke);
}

static void sp_bp5_leftside_cost_success_in_left_side(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int keke = test_new_id(&tg, KEKE_LEFT);
    int filler = test_id(&tg, FILLER_4);
    pin_id(keke, KEKE_LEFT);
    if (keke < 0) return;
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    int accepted = keke_activate_from_area(&tg, keke, 0);
    dump_board(&tg);
    dump_ability(keke, 0);
    CHECK(accepted != 0, "B: from the LEFT area the 起動 is accepted");
    CHECK(is_wait(&tg, keke), "B: the self_cost pays — she is WAIT");
}

static void sp_bp5_leftside_cost_fails_in_center(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int keke = test_new_id(&tg, KEKE_LEFT);
    int filler = test_id(&tg, FILLER_4);
    pin_id(keke, KEKE_LEFT);
    if (keke < 0) return;
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    int accepted = keke_activate_from_area(&tg, keke, 1);
    drain_answering(&tg, 8);
    dump_board(&tg);
    /* NON-VACUITY: the LEFT case above proves the same card+board DOES activate,
     * so a refusal here is the position gate, not a dead ability. */
    CHECK_EQ(accepted, 0, "B: from the CENTRE area the 起動 must be refused (左サイド)");
    CHECK(!is_wait(&tg, keke), "B: the refused activation must not wait her");
    CHECK(!pending(&tg), "B: a refused activation must not open the draw/discard prompt");
}

static void sp_bp5_leftside_cost_fails_in_right_side(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_new_id(&tg, KEKE_LEFT);
    int filler = test_id(&tg, FILLER_4);
    pin_id(keke, KEKE_LEFT);
    if (keke < 0) return;
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    int accepted = keke_activate_from_area(&tg, keke, 2);
    drain_answering(&tg, 8);
    dump_board(&tg);
    /* NON-VACUITY: same card, same board shape as the LEFT case, only the area
     * differs, and the LEFT case activates. */
    CHECK_EQ(accepted, 0, "B: from the RIGHT area the 起動 must be refused (左サイド)");
    CHECK(!is_wait(&tg, keke),
          "B: 左サイド is required, so the member must NOT be waited from the right");
    CHECK(!pending(&tg), "B: a refused activation must not open the draw/discard prompt");
}

/* =======================================================================
 * C. per_card/chika_debut_result_character_condition_test.rs (3 tests)
 *    PL!S-bp7-001-R 高海千歌 登場: 「手札を1枚控え室に置いてもよい：自分の控え室
 *    からコスト10以上のメンバーカードを1枚手札に加える。これにより『桜内梨子』か
 *    『渡辺曜』を手札に加えた場合、ライブ終了時までブレード+2を得る。」
 *
 *    The gate is the result_condition's `characters` list. A variant of this
 *    cluster has already been reported for 園田海未 PL!-HS-bp1-001-R dropping
 *    BOTH the 場合 result-condition and the 【センター】 area keyword; this
 *    section is the same SHAPE on a different card, so it is the natural probe
 *    for whether the drop is card-specific or general.
 * ======================================================================= */

/* Play 千歌, pay the optional discard-1 cost, and recover `target` (the sole
 * cost>=10 member in the waitroom) to hand. */
static int play_chika_add_to_hand(TestGame *tg, int chika, int target)
{
    test_add_to_hand(tg, chika);
    /* More than one hand card so the optional discard-1 cost always prompts. */
    test_add_to_hand(tg, test_id(tg, FILLER_4));
    test_add_to_hand(tg, test_id(tg, FILLER_4B));
    test_add_to_discard(tg, target);
    set_stage(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    play_to_stage(tg, chika, 1, 0);
    if (!pending(tg)) return 0;
    /* Rust select_indices(&[0]) — ACCEPT the optional discard. */
    pick(tg, 0);
    /* The follow-up move_cards out of the waitroom auto-resolves when exactly
     * one card qualifies, but the C queue parks a resume behind the cost, so
     * keep answering until it is quiet. */
    drain_answering(tg, 8);
    return 1;
}

static void chika_board(TestGame *tg, int *chika_out, int *filler_out)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int chika = test_new_id(tg, CHIKA);
    int filler = test_id(tg, FILLER_4);
    pin_id(chika, CHIKA);
    pin_id(filler, FILLER_4);
    fill_decks(tg, filler, 50);
    test_give_energy(tg, 9);
    *chika_out = chika; *filler_out = filler;
}

static void chika_adds_riko_gets_2_blades(void)
{
    static TestGame tg;
    int chika = 0, filler = 0;
    chika_board(&tg, &chika, &filler);
    if (chika < 0) return;
    int riko = test_new_id(&tg, RIKO);
    pin_id(riko, RIKO);
    CHECK_EQ(printed_cost(riko), 11, "C: 桜内梨子 really prints cost 11 (>= 10)");

    int ran = play_chika_add_to_hand(&tg, chika, riko);
    dump_board(&tg);
    dump_ability(chika, 0);
    CHECK(ran, "C: the optional discard-1 cost prompt appeared");
    CHECK(test_hand_has(&tg, riko), "C: 桜内梨子 was added to hand");
    /* NON-VACUITY CONTROL: the recover itself really happened, so a blade
     * modifier of 0 below is a REFUSED followup, not a stalled ability. */
    CHECK(!test_zone_has_id(&tg, 0, "discard", riko),
          "C CONTROL: 桜内梨子 really left the waitroom");
    CHECK_EQ(blade_mod(&tg, chika), 2, "C: adding 桜内梨子 grants blade +2");
}

static void chika_adds_yo_gets_2_blades(void)
{
    static TestGame tg;
    int chika = 0, filler = 0;
    chika_board(&tg, &chika, &filler);
    if (chika < 0) return;
    int yo = test_new_id(&tg, YO);
    pin_id(yo, YO);
    CHECK_EQ(printed_cost(yo), 15, "C: 渡辺 曜 really prints cost 15 (>= 10)");

    int ran = play_chika_add_to_hand(&tg, chika, yo);
    dump_board(&tg);
    CHECK(ran, "C: the optional discard-1 cost prompt appeared");
    CHECK(test_hand_has(&tg, yo), "C: 渡辺 曜 was added to hand");
    CHECK(!test_zone_has_id(&tg, 0, "discard", yo), "C CONTROL: 渡辺 曜 left the waitroom");
    CHECK_EQ(blade_mod(&tg, chika), 2, "C: adding 渡辺 曜 grants blade +2");
}

static void chika_adds_non_matching_member_gets_no_blades(void)
{
    static TestGame tg;
    int chika = 0, filler = 0;
    chika_board(&tg, &chika, &filler);
    if (chika < 0) return;
    /* 高坂穂乃果 cost 11 — passes the cost>=10 gate but is neither 桜内梨子 nor
     * 渡辺 曜, so ONLY the characters list can reject it. */
    int honoka = test_new_id(&tg, HONOKA_SD);
    pin_id(honoka, HONOKA_SD);
    CHECK_EQ(printed_cost(honoka), 11, "C: the negative control really costs >= 10");

    int ran = play_chika_add_to_hand(&tg, chika, honoka);
    dump_board(&tg);
    CHECK(ran, "C: the optional discard-1 cost prompt appeared");
    /* NON-VACUITY CONTROL — THE load-bearing assertion of this whole cluster.
     * The two POSITIVE cases above prove the identical board with a named
     * character yields 2 blades, so a 0 here is the characters gate refusing
     * and not a followup that never runs. */
    CHECK(test_hand_has(&tg, honoka), "C CONTROL: the non-matching member reached hand");
    CHECK(!test_zone_has_id(&tg, 0, "discard", honoka),
          "C CONTROL: the non-matching member really left the waitroom");
    CHECK_EQ(blade_mod(&tg, chika), 0,
             "C: adding a non-桜内梨子/渡辺曜 member grants NO blades");
}

/* =======================================================================
 * D. per_card/hanamaru_debut_result_character_deploy_test.rs (7 tests)
 *    PL!S-bp7-007-R＋ 国木田花丸 登場: 「自分の控え室からコスト2以下のメンバー
 *    カードを1枚手札に加える。これによって『津島善子』か『黒澤ルビィ』を手札に
 *    加えた場合、そのカードを自分のステージのメンバーのいないエリアに登場させ
 *    てもよい。」
 *
 *    Same result_condition gate as section C, but the followup MOVES a card, so
 *    these tests also pin where the deploy lands.
 * ======================================================================= */

static void hanamaru_fire_debut(TestGame *tg, int hanamaru)
{
    /* Rust's process_debut: trigger the 登場 ability for `hanamaru` directly,
     * without a play-to-stage. test_fire_debut is the C entry point. */
    test_fire_debut(tg, hanamaru);
    rb_process_pending_auto_abilities(&tg->state);
}

/* Drain the debut: answer the candidate SelectCard with index 0, let the
 * follow-up deploy resolve, and decline anything else. Mirrors Rust's
 * drain_auto_ability_choices + select_indices(&[0]) sequence. */
static int hanamaru_drain(TestGame *tg)
{
    int answered = 0, guard = 0;
    while (pending(tg) && guard++ < 12) {
        const char *k = pending_kind(tg);
        if (k && (!strcmp(k, "SelectCard") || !strcmp(k, "SelectTarget") ||
                  !strcmp(k, "SelectPosition") || !strcmp(k, "SelectAutoAbility")))
            pick(tg, 0);
        else
            skip_pick(tg);
        answered++;
    }
    return answered;
}

static void hanamaru_board(TestGame *tg, int *hanamaru_out)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int hanamaru = test_new_id(tg, HANAMARU);
    int filler = test_id(tg, FILLER_4);
    pin_id(hanamaru, HANAMARU);
    pin_id(filler, FILLER_4);
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, pl, filler);
    *hanamaru_out = hanamaru;
}

/* 1 matching card in the waitroom -> auto-resolve, and with stage[2] free the
 * deploy is deterministic. */
static void hanamaru_single_matching_auto_deploys(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int tsushima = test_new_id(&tg, TSUSHIMA);
    pin_id(tsushima, TSUSHIMA);
    CHECK_EQ(printed_cost(tsushima), 2, "D: 津島善子 really costs 2 (inside the <=2 gate)");

    set_stage(&tg, 0, filler, hanamaru, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, tsushima);
    hanamaru_fire_debut(&tg, hanamaru);
    hanamaru_drain(&tg);
    dump_board(&tg);
    dump_ability(hanamaru, 0);

    CHECK_EQ(tg.state.p[0].stage[2], tsushima,
             "D: the one matching card auto-deploys to the empty stage[2]");
}

/* 2 matching cards -> a SelectCard pick is forced. */
static void hanamaru_multiple_valid_forces_selection(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int tsushima = test_new_id(&tg, TSUSHIMA);
    int ruby     = test_new_id(&tg, RUBY);
    pin_id(tsushima, TSUSHIMA);
    pin_id(ruby, RUBY);
    CHECK_EQ(printed_cost(ruby), 2, "D: 黒澤ルビィ really costs 2 (inside the <=2 gate)");
    CHECK(tsushima != ruby, "D CONTROL: the two candidates are DISTINCT instances");

    set_stage(&tg, 0, filler, hanamaru, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, tsushima);
    test_add_to_discard(&tg, ruby);
    hanamaru_fire_debut(&tg, hanamaru);
    CHECK(pending(&tg), "D: two valid cards must force a SelectCard pick");
    dump_choice(&tg);
    CHECK(pending_kind(&tg) && !strcmp(pending_kind(&tg), "SelectCard"),
          "D: the prompt is a SelectCard");
    pick(&tg, 0);
    hanamaru_drain(&tg);
    dump_board(&tg);

    CHECK(!pending(&tg), "D: the pick resolves the prompt");
    CHECK(tg.state.p[0].stage[2] == tsushima || tg.state.p[0].stage[2] == ruby,
          "D: the picked card deploys to the empty stage[2]");
}

/* A non-matching cost-2 member reaches hand but must NOT deploy. */
static void hanamaru_non_matching_no_deploy(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int non_match = test_new_id(&tg, ERI_NONMATCH);
    pin_id(non_match, ERI_NONMATCH);
    CHECK_EQ(printed_cost(non_match), 2,
             "D: 絢瀬 絵里 really costs 2 — ONLY the name gate can reject her");

    set_stage(&tg, 0, filler, hanamaru, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, non_match);
    hanamaru_fire_debut(&tg, hanamaru);
    hanamaru_drain(&tg);
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the two POSITIVE cases above deploy a named card to
     * stage[2] on this exact board shape, so an empty stage[2] here is the
     * name gate refusing. */
    CHECK(test_hand_has(&tg, non_match),
          "D CONTROL: the non-matching cost-2 member really reached hand");
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT,
             "D: no deploy for a non-津島善子/黒澤ルビィ member");
}

/* No empty stage area -> the card lands in hand even though the name matches. */
static void hanamaru_no_empty_slot_card_in_hand(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int f1 = test_id(&tg, FILLER_4);
    int f2 = test_new_id(&tg, MUS_LIVE);
    int tsushima = test_new_id(&tg, TSUSHIMA);
    pin_id(tsushima, TSUSHIMA);

    set_stage(&tg, 0, f1, hanamaru, f2);
    test_add_to_discard(&tg, tsushima);
    hanamaru_fire_debut(&tg, hanamaru);
    hanamaru_drain(&tg);
    dump_board(&tg);

    CHECK(test_hand_has(&tg, tsushima),
          "D: with a full stage the matching card stays in hand");
    CHECK_EQ(tg.state.p[0].stage[0], f1, "D CONTROL: stage[0] is untouched");
    CHECK_EQ(tg.state.p[0].stage[2], f2, "D CONTROL: stage[2] is untouched");
}

/* A cost-4 member in the waitroom is outside the 「コスト2以下」 gate. */
static void hanamaru_no_valid_discard_no_action(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int too_expensive = test_new_id(&tg, TOO_EXPENSIVE);
    pin_id(too_expensive, TOO_EXPENSIVE);
    /* RUST COMMENT BUG, pinned. hanamaru_debut_result_character_deploy_test.rs
     * labels PL!-sd1-014-SD 「絢瀬 絵里 cost=4」 — cards.json says it is
     * 星空 凛, cost 9, lilywhite. The test's INTENT (a card ABOVE the gate) is
     * satisfied; only the name and the number in the comment are wrong. */
    CHECK(printed_cost(too_expensive) == 9,
          "D FIXTURE BUG: the Rust comment calls PL!-sd1-014-SD 「cost=4」; "
          "cards.json and the decoded card both say cost 9 (星空 凛, not 絢瀬 絵里)");

    set_stage(&tg, 0, filler, hanamaru, RB_EMPTY_SLOT);
    test_add_to_discard(&tg, too_expensive);
    hanamaru_fire_debut(&tg, hanamaru);
    hanamaru_drain(&tg);
    dump_board(&tg);

    /* NON-VACUITY: section D's positive cases move a cost-2 card off the same
     * waitroom on the same board, so an untouched zone here is the cost gate. */
    CHECK(!pending(&tg), "D: no valid target -> nothing happens");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "D CONTROL: the waitroom card was NOT moved");
    CHECK(!test_hand_has(&tg, too_expensive), "D: the cost-9 card did not reach hand");
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT, "D: nothing deployed");
}

/* With stage[0] the only free area, the deploy lands on the LEFT. */
static void hanamaru_deploy_to_left_when_right_full(void)
{
    static TestGame tg;
    int hanamaru = 0;
    hanamaru_board(&tg, &hanamaru);
    if (hanamaru < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int tsushima = test_new_id(&tg, TSUSHIMA);
    pin_id(tsushima, TSUSHIMA);

    set_stage(&tg, 0, RB_EMPTY_SLOT, hanamaru, filler);
    test_add_to_discard(&tg, tsushima);
    hanamaru_fire_debut(&tg, hanamaru);
    hanamaru_drain(&tg);
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], tsushima,
             "D: the matching card deploys to the only empty area (left)");
}

/* =======================================================================
 * E. per_card/ginako_live_start_discard_group_blade_count_test.rs (4 tests)
 *    PL!HS-bp6-004-R 百生吟子 ライブ開始時 ab#0 (cost<=9 opponent -> wait) and
 *    ab#1 (optional discard 1 from hand; IF that card is 百生吟子 -> +1 blade on
 *    top of the printed 1). The gate here is a CHARACTER check on the card the
 *    COST moved, and the blade arithmetic makes it non-vacuous for free:
 *    1 blade is the ungated floor and 2 blades requires the gate to have fired.
 * ======================================================================= */

static int ginako_board(TestGame *tg, int *ginako_out, int *opp_out, const char *payload_no)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int ginako = test_new_id(tg, GINAKO);
    int opp    = test_new_id(tg, OPP_MEMBER);
    pin_id(ginako, GINAKO);
    pin_id(opp, OPP_MEMBER);
    CHECK_EQ(printed_cost(opp), 2, "E: the opponent fixture really costs 2 (<= 9)");
    if (ginako < 0 || opp < 0) return -1;
    int payload_id = -1;
    if (payload_no) {
        payload_id = test_new_id(tg, payload_no);
        pin_id(payload_id, payload_no);
        if (payload_id < 0) return -1;
    }
    set_stage(tg, 0, RB_EMPTY_SLOT, ginako, RB_EMPTY_SLOT);
    tg->state.p[1].stage[0] = RB_EMPTY_SLOT;
    tg->state.p[1].stage[1] = opp;
    tg->state.p[1].stage[2] = RB_EMPTY_SLOT;
    /* Rust setup_and_trigger_live_start: the live card, the hand payload, 10
     * deck fillers per seat, 5 energy, 5 passes, set live, 2 passes. The C
     * turn engine draws a card inside those 5 passes, so reset the hand to the
     * Rust twin's exact {live card, payload} before setting the live card. */
    int live = test_id(tg, MUS_LIVE);
    pin_id(live, MUS_LIVE);
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, pl, test_id(tg, FILLER_4));
    test_give_energy(tg, 5);
    test_add_to_hand(tg, live);
    if (payload_no) test_add_to_hand(tg, payload_id);
    advance_to_live_set(tg);
    CHECK_EQ(tg->state.phase, RB_PHASE_LIVE_SET,
             "E CONTROL: five passes really reach LiveCardSetP1");
    test_set_live_card(tg, 0, live);
    /* The C live-zone refill pulls a filler into hand at LiveSet; reset the hand
     * to the Rust twin's exact {payload} AFTER the refill so the ライブ開始時
     * board matches. */
    tg->state.p[0].hand.n = 0;
    if (payload_no) test_add_to_hand(tg, payload_id);
    finish_live_setup(tg);
    *ginako_out = ginako; *opp_out = opp;
    return payload_id;
}

/* Live-start setups queue BOTH ライブ開始時 abilities of 百生吟子. The C engine
 * does not necessarily open a SelectAutoAbility ordering prompt (the queue may
 * run ab#0 to completion first), so the drain below answers whatever comes up
 * and asserts on the OBSERVABLE outcome — which is what the Rust test asserts
 * anyway. */
static void ginako_discard_self_gains_two_blades(void)
{
    static TestGame tg;
    int second_ginako = 0;
    int ginako = 0, opp = 0;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    second_ginako = ginako_board(&tg, &ginako, &opp, GINAKO);
    if (ginako < 0) return;
    CHECK(second_ginako >= 0, "E: a SECOND distinct 百生吟子 instance exists");
    dump_board(&tg);
    dump_ability(ginako, 1);

    /* ab#0 must auto-apply the wait to the single cost<=9 opponent. */
    CHECK(is_wait(&tg, opp), "E: ab#0 auto-waits the single cost<=9 opponent");

    int discard_before = tg.state.p[0].discard.n;
    int hand_before = tg.state.p[0].hand.n;
    int guard = 0;
    while (pending(&tg) && guard++ < 8) {
        const char *k = pending_kind(&tg);
        if (k && !strcmp(k, "SelectCard")) pick(&tg, 0);
        else skip_pick(&tg);
    }
    drain_answering(&tg, 16);
    dump_board(&tg);

    /* NON-VACUITY CONTROLS: the discard really happened, so blade == 2 is the
     * conditional +1 ON TOP of the printed 1, not a misread floor. */
    CHECK(tg.state.p[0].discard.n > discard_before,
          "E CONTROL: a hand card really was discarded for ab#1");
    CHECK(tg.state.p[0].hand.n < hand_before, "E CONTROL: the hand really shrank");
    CHECK(test_zone_has_id(&tg, 0, "discard", second_ginako),
          "E CONTROL: the discarded card is the second 百生吟子");
    CHECK_EQ(blade_mod(&tg, ginako), 2,
             "E: discarding 百生吟子 -> printed 1 blade + conditional 1 = 2");
    CHECK(is_wait(&tg, opp), "E: the opponent remains waited");
    CHECK(!test_zone_has_id(&tg, 1, "discard", opp),
          "E: a wait must NOT move the card to the waitroom (orientation only)");
}

static void ginako_discard_non_ginako_gains_one_blade(void)
{
    static TestGame tg;
    int non_ginako = 0;
    int ginako = 0, opp = 0;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    non_ginako = ginako_board(&tg, &ginako, &opp, FILLER_4);
    if (ginako < 0) return;
    dump_board(&tg);

    int discard_before = tg.state.p[0].discard.n;
    int guard = 0;
    while (pending(&tg) && guard++ < 8) {
        const char *k = pending_kind(&tg);
        if (k && !strcmp(k, "SelectCard")) pick(&tg, 0);
        else skip_pick(&tg);
    }
    drain_answering(&tg, 16);
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the discard really happened, and the case above
     * proves the SAME board with 百生吟子 discarded yields 2. So 1 is the gate. */
    CHECK(tg.state.p[0].discard.n > discard_before,
          "E CONTROL: the non-百生吟子 card really was discarded");
    CHECK_EQ(blade_mod(&tg, ginako), 1,
             "E: discarding a non-百生吟子 card leaves only the printed 1 blade");
}

static void ginako_skip_cost_gains_zero_blades(void)
{
    static TestGame tg;
    int ginako = 0, opp = 0;
    ginako_board(&tg, &ginako, &opp, NULL);
    if (ginako < 0) return;
    dump_board(&tg);

    /* Rust select_indices(&[]) — a real DECLINE, the FIRST prompt the queue
     * offers for ab#1's optional cost. */
    if (pending(&tg)) skip_pick(&tg);
    int guard = 0;
    while (pending(&tg) && guard++ < 8) skip_pick(&tg);
    drain_answering(&tg, 16);
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the case above proves the same board yields 1 when
     * the discard is TAKEN, so 0 is the skip and not a dead ability. */
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "E CONTROL: skipping the cost really discarded nothing");
    CHECK_EQ(blade_mod(&tg, ginako), 0, "E: skipping the cost -> 0 blades");
}

/* Two 百生吟子 on stage: exactly ONE of them pays the cost, so exactly one gets
 * the conditional +1. This is the case that would catch a "grant to every
 * 百生吟子 on stage" bug. */
static void two_ginako_discard_one_gains_blade_on_self(void)
{
    static TestGame tg;
    int third = 0;
    int ginako1 = 0, ginako2 = 0, opp = 0;
    /* The payload id must be produced INSIDE the board helper: the helper calls
     * test_game_new, which re-bases the copy pool, so an id minted beforehand
     * silently aliases the one the helper mints (an identity assertion built on
     * it would be vacuous). */
    third = ginako_board(&tg, &ginako1, &opp, GINAKO);
    if (ginako1 < 0) return;
    pin_id(third, GINAKO);
    ginako2 = test_new_id(&tg, GINAKO);
    pin_id(ginako2, GINAKO);
    CHECK(ginako1 != ginako2 && ginako2 != third && ginako1 != third,
          "E CONTROL: three DISTINCT 百生吟子 instances exist");
    set_stage(&tg, 0, ginako1, ginako2, RB_EMPTY_SLOT);
    dump_board(&tg);

    int paid = 0, guard = 0;
    while (pending(&tg) && guard++ < 12) {
        const char *k = pending_kind(&tg);
        const RbChoice *ch = rb_get_pending_choice(&tg.state);
        if (k && !strcmp(k, "SelectAutoAbility")) { pick(&tg, 0); continue; }
        if (k && !strcmp(k, "SelectCard") && ch && ch->zone &&
            !strcmp(ch->zone, "hand") && !paid) {
            pick(&tg, 0);
            paid = 1;
            continue;
        }
        skip_pick(&tg);
    }
    dump_board(&tg);

    /* NON-VACUITY CONTROL: only one hand card existed, so `paid` proves the
     * cost was actually spent once. */
    CHECK(paid, "E CONTROL: the single hand discard really was taken once");
    CHECK(test_zone_has_id(&tg, 0, "discard", third),
          "E CONTROL: the discarded card is the third 百生吟子");
    CHECK_EQ(blade_mod(&tg, ginako1) + blade_mod(&tg, ginako2), 2,
             "E: exactly one 百生吟子 gets the conditional blade (2 total)");
}

/* =======================================================================
 * F. per_card/kanon_debut_invalidate_recovers_discard_card_test.rs (4 tests)
 *    PL!SP-bp2-001-R＋ 澁谷かのん 登場: 「自分のステージにいる『Liella!』の
 *    メンバー1人のすべてのライブ開始時能力を、ライブ終了時まで、無効にしても
 *    よい。これにより無効にした場合、自分の控え室から『Liella!』のカードを1枚
 *    手札に加える。」 — the followup is gated on the OPTIONAL COST BEING TAKEN.
 *
 *    Coverage note: test_parity_characterization_extra.c §L already drives the
 *    take-the-invalidation arm of this exact card. What is NOT in the C corpus
 *    is Q106 (nullifying an ALREADY-nullified ability must not re-trigger) and
 *    the expiry at live end, plus the skip arm — all three are ported here.
 * ======================================================================= */

static void kanon_board(TestGame *tg, int *kanon_out, int *target_out,
                        int *rec1_out, int *rec2_out)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int kanon  = test_new_id(tg, KANON_DEBUT);
    int target = test_new_id(tg, KANON_TGT);
    int rec1   = test_new_id(tg, KANON_REC1);
    pin_id(kanon, KANON_DEBUT);
    pin_id(target, KANON_TGT);
    pin_id(rec1, KANON_REC1);
    CHECK(rb_card_matches_group_str(target, "Liella!"),
          "F: the invalidation target really is a 『Liella!』 member");
    CHECK(rb_card_matches_group_str(rec1, "Liella!"),
          "F: the recovery card really is a 『Liella!』 card");
    *kanon_out = kanon; *target_out = target; *rec1_out = rec1; *rec2_out = -1;
}

static void kanon_debut_invalidation_recovers_liella_from_discard(void)
{
    static TestGame tg;
    int kanon = 0, target = 0, rec1 = 0, rec2 = 0;
    kanon_board(&tg, &kanon, &target, &rec1, &rec2);
    if (kanon < 0) return;

    test_add_to_hand(&tg, kanon);
    test_add_to_hand(&tg, test_id(&tg, FILLER_4));
    test_add_to_discard(&tg, rec1);
    test_give_energy(&tg, 13);
    set_stage(&tg, 0, target, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    play_to_stage(&tg, kanon, 1, 0);
    CHECK(pending(&tg), "F: the 登場 offers the optional invalidation target");
    dump_choice(&tg);
    CHECK(pending_kind(&tg) && !strcmp(pending_kind(&tg), "SelectCard"),
          "F: the target prompt is a SelectCard");
    pick(&tg, 0);
    drain_answering(&tg, 16);
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], target, "F: the target stays at left");
    CHECK_EQ(tg.state.p[0].stage[1], kanon, "F: かのん is at centre");
    CHECK(rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "F: the target's ライブ開始時 is invalidated");
    CHECK(test_hand_has(&tg, rec1), "F: the 『Liella!』 card reached hand");
    CHECK(!test_zone_has_id(&tg, 0, "discard", rec1), "F: it left the waitroom");
}

static void kanon_optional_invalidation_can_be_skipped(void)
{
    static TestGame tg;
    int kanon = 0, target = 0, rec1 = 0, rec2 = 0;
    kanon_board(&tg, &kanon, &target, &rec1, &rec2);
    if (kanon < 0) return;

    test_add_to_hand(&tg, kanon);
    test_add_to_hand(&tg, test_id(&tg, FILLER_4));
    test_add_to_discard(&tg, rec1);
    test_give_energy(&tg, 13);
    set_stage(&tg, 0, target, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    play_to_stage(&tg, kanon, 1, 0);
    CHECK(pending(&tg), "F: the optional invalidation prompt is offered");
    dump_choice(&tg);
    CHECK(pending_kind(&tg) && !strcmp(pending_kind(&tg), "SelectCard"),
          "F: the prompt is a SelectCard");
    skip_pick(&tg);
    drain_answering(&tg, 16);
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the case above takes the same prompt on the same
     * board and DOES invalidate + recover, so these negatives are the skip. */
    CHECK(!pending(&tg), "F: the skip resolves the prompt");
    CHECK(!rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "F: nothing was invalidated");
    CHECK(!test_hand_has(&tg, rec1), "F: no 『Liella!』 card was recovered");
    CHECK(test_zone_has_id(&tg, 0, "discard", rec1), "F: the card stays in the waitroom");
}

/* Q106: nullifying an ALREADY-nullified ability does not count, so the second
 * かのん's optional step is a no-op and its followup never fires. */
static void kanon_q106_cannot_invalidate_same_live_start_twice(void)
{
    static TestGame tg;
    int kanon = 0, target = 0, rec1 = 0, rec2 = 0;
    kanon_board(&tg, &kanon, &target, &rec1, &rec2);
    if (kanon < 0) return;
    int kanon2 = test_new_id(&tg, KANON_DEBUT);
    int rec2b  = test_new_id(&tg, KANON_REC2);
    pin_id(kanon2, KANON_DEBUT);
    pin_id(rec2b, KANON_REC2);
    CHECK(kanon2 != kanon, "F CONTROL: the two かのん are DISTINCT instances");
    CHECK(rb_card_matches_group_str(rec2b, "Liella!"),
          "F: the second recovery card is also a 『Liella!』 card");

    test_add_to_hand(&tg, kanon);
    test_add_to_hand(&tg, kanon2);
    test_add_to_hand(&tg, test_id(&tg, FILLER_4));
    test_add_to_discard(&tg, rec1);
    test_add_to_discard(&tg, rec2b);
    test_give_energy(&tg, 26);
    set_stage(&tg, 0, target, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    play_to_stage(&tg, kanon, 1, 0);
    if (pending(&tg)) pick(&tg, 0);
    drain_answering(&tg, 16);
    dump_board(&tg);
    CHECK(test_hand_has(&tg, rec1), "F CONTROL: the FIRST かのん really recovered its card");

    play_to_stage(&tg, kanon2, 2, 0);
    dump_board(&tg);
    if (pending(&tg)) dump_choice(&tg);

    int n_for_target = 0;
    for (int i = 0; i < tg.state.n_ability_invalidations; i++)
        if (tg.state.ability_invalidations[i].card_id == target) n_for_target++;
    CHECK(!pending(&tg), "F: the second かのん offers nothing to invalidate");
    CHECK_EQ(n_for_target, 1,
             "F (Q106): an already-nullified ライブ開始時 is not nullified again");
    CHECK(!test_hand_has(&tg, rec2b), "F: the second recovery never happened");
    CHECK(test_zone_has_id(&tg, 0, "discard", rec2b), "F: its card stays in the waitroom");
}

static void kanon_live_start_invalidation_expires_at_live_end(void)
{
    static TestGame tg;
    int kanon = 0, target = 0, rec1 = 0, rec2 = 0;
    kanon_board(&tg, &kanon, &target, &rec1, &rec2);
    if (kanon < 0) return;
    int filler = test_id(&tg, FILLER_4);
    int filler2 = test_new_id(&tg, FILLER_4);
    int live = test_id(&tg, LIVE_SC1);
    pin_id(live, LIVE_SC1);
    CHECK(printed_score(live) == 1, "F: the live fixture really prints score 1");

    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, pl, filler);
    set_stage(&tg, 0, target, filler, filler2);
    test_add_to_hand(&tg, kanon);
    test_add_to_hand(&tg, live);
    test_add_to_discard(&tg, rec1);
    test_give_energy(&tg, 13);

    play_to_stage(&tg, kanon, 1, 0);
    /* Rust plays her at Centre, but the board is target|kanon|filler. */
    tg.state.p[0].stage[1] = kanon;
    if (pending(&tg)) pick(&tg, 0);
    drain_answering(&tg, 16);
    CHECK(rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "F CONTROL: the invalidation is registered before live start");

    /* BOTH seats need a live card, or the performance resolves to an immediate
     * Victory and the turn never reaches the next Active phase. */
    int live2 = test_new_id(&tg, LIVE_SC1);
    pin_id(live2, LIVE_SC1);
    tg.state.p[1].hand.n = 0;
    test_add_to_hand_for(&tg, 1, live2);
    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    test_set_live_card_for(&tg, 1, live2);
    test_pass(&tg);
    finish_live_setup(&tg);
    dump_board(&tg);

    CHECK(rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "F: the invalidation is still live at ライブ開始時");
    CHECK_EQ(blade_mod(&tg, target), 0,
             "F: the invalidated ライブ開始時 grants no blade");
    /* Drive to the next Active phase, i.e. past ライブ終了. */
    int guard = 0;
    while (tg.state.phase != RB_PHASE_ACTIVE && guard++ < 12) {
        DIAG("phase=%s", rb_phase_name(tg.state.phase));
        test_pass(&tg);
    }
    dump_board(&tg);
    DIAG("final phase=%s", rb_phase_name(tg.state.phase));
    CHECK_EQ(tg.state.phase, RB_PHASE_ACTIVE, "F: the turn reached the next Active phase");
    CHECK(!rb_ability_is_invalidated(&tg.state, target, RB_TSTR_LIVE_START),
          "F: the invalidation expired at ライブ終了時");
}

/* =======================================================================
 * G. per_card/wien_live_start_empty_hand_energy_placement_test.rs (4 tests)
 *    PL!SP-pb2-010-R ウィーン・マルガレーテ ライブ開始時: 「手札を1枚控え室
 *    に置かないかぎり、自分のエネルギー1枚をエネルギーデッキに置く。」
 *    The gate is NEGATIVE: take the optional discard -> the energy does NOT
 *    move; skip it -> the energy DOES move. Q262.
 * ======================================================================= */

/* The Rust helper advances to LiveCardSetP1 and the hand then holds ONLY the
 * live card. The C turn engine runs a real DRAW phase inside those five passes,
 * so a filler lands in hand too; drop it so the board matches the Rust twin
 * (this is a SETUP correction, not a relaxed assertion). */
static void wien_board(TestGame *tg, int *wien_out, int extra_hand)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int wien = test_new_id(tg, WIEN);
    int filler = test_id(tg, FILLER_4);
    int live = test_id(tg, MUS_LIVE);
    pin_id(wien, WIEN);
    pin_id(live, MUS_LIVE);
    if (wien < 0) return;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, pl, filler);
    set_stage(tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    test_give_energy(tg, 5);
    advance_to_live_set(tg);
    CHECK_EQ(tg->state.phase, RB_PHASE_LIVE_SET, "G CONTROL: LiveCardSetP1 is current");
    test_set_live_card(tg, 0, live);
    /* The C live-zone refill pulls a filler into hand at LiveSet; clear the hand
     * AFTER it so ライブ開始時 really sees an empty hand (the Rust twin's board). */
    tg->state.p[0].hand.n = 0;
    if (extra_hand >= 0) test_add_to_hand(tg, extra_hand);
    *wien_out = wien;
}

static void wien_q262_empty_hand_triggers_energy_move(void)
{
    static TestGame tg;
    int wien = 0;
    wien_board(&tg, &wien, -1);
    if (wien < 0) return;
    CHECK_EQ(tg.state.p[0].hand.n, 0, "G CONTROL: the hand really is empty at ライブ開始時");
    int e_before = tg.state.p[0].energy_active;
    int ed_before = tg.state.p[0].energy_deck.n;

    finish_live_setup(&tg);
    dump_board(&tg);
    CHECK(pending(&tg), "G: the Skip/Pay gate for the optional discard is presented");
    dump_choice(&tg);
    pick(&tg, 0);   /* option 0 = SKIP */
    drain_answering(&tg, 16);
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].energy_active, e_before - 1,
             "G (Q262): exactly one energy card left the energy zone");
    CHECK_EQ(tg.state.p[0].energy_deck.n, ed_before + 1,
             "G (Q262): the same card landed in the energy deck");
}

static void wien_q262_discard_prevents_energy_move(void)
{
    static TestGame tg;
    int wien = 0;
    int filler = test_id(&tg, FILLER_4);
    filler = test_new_id(&tg, FILLER_4);
    wien_board(&tg, &wien, filler);
    if (wien < 0) return;
    CHECK_EQ(tg.state.p[0].hand.n, 1, "G CONTROL: one discardable card is in hand");
    int e_before = tg.state.p[0].energy_active;
    int ed_before = tg.state.p[0].energy_deck.n;
    int hand_before = tg.state.p[0].hand.n;

    finish_live_setup(&tg);
    CHECK(pending(&tg), "G: the discard choice is presented");
    dump_choice(&tg);
    pick(&tg, 1);   /* option 1 = PAY, i.e. take the optional discard */
    /* Follow-up SelectCard for WHICH card to discard. */
    int guard = 0;
    while (pending(&tg) && guard++ < 5) {
        const char *k = pending_kind(&tg);
        if (k && !strcmp(k, "SelectCard")) pick(&tg, 0);
        else skip_pick(&tg);
    }
    dump_board(&tg);

    /* NON-VACUITY CONTROL — THE paired positive for the case above. The SKIP
     * case proves the same board moves exactly one energy when the discard is
     * declined, so "unchanged" here is the PAY arm of the negation. */
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "G CONTROL: the optional discard really happened");
    CHECK(test_zone_has_id(&tg, 0, "discard", filler),
          "G CONTROL: the discarded card is in the waitroom");
    CHECK_EQ(tg.state.p[0].energy_active, e_before,
             "G: discarding prevents the energy move");
    CHECK_EQ(tg.state.p[0].energy_deck.n, ed_before,
             "G: the energy deck is unchanged when the discard is taken");
}

/* =======================================================================
 * H. state_gated/discard_group_look_four_or_recover_live_test.rs (2 tests)
 *    PL!-bp5-003-R＋ 南 ことり 起動 ab#1: discard 1 from hand; IF that card is
 *    a 『μ's』 card -> look 4 and take 2, ELSE -> add 1 live card from the
 *    waitroom to hand. A clean two-branch alternative on a GROUP filter.
 * ======================================================================= */

static int kotori_board(TestGame *tg, const char *payload_no, int *kotori_out)
{
    test_game_new(tg);
    clear_p1(tg);
    rb_card_copy_pool_reset();
    int kotori = test_new_id(tg, KOTORI);
    int filler = test_id(tg, FILLER_4);
    pin_id(kotori, KOTORI);
    if (kotori < 0) return -1;
    int payload = test_new_id(tg, payload_no);
    pin_id(payload, payload_no);
    if (payload < 0) return -1;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 0, filler);
    test_give_energy(tg, 10);
    set_stage(tg, 0, kotori, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(tg, payload);
    *kotori_out = kotori;
    return payload;
}

/* μ's payload -> the look-4 branch grows the hand. */
static void matching_group_discard_look_four_increases_hand(void)
{
    static TestGame tg;
    int kotori = 0;
    int mus_card = kotori_board(&tg, HONOKA_BP6, &kotori);
    if (kotori < 0) return;
    CHECK(rb_card_matches_group_str(mus_card, MU_GROUP),
          "H: the payload really is a 『μ's』 group card");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "H CONTROL: exactly one hand card before the 起動");

    int hand_before = tg.state.p[0].hand.n;
    int energy_before = tg.state.p[0].energy_active;
    /* The card prints TWO 起動; the Rust test drives ab#1 explicitly, and
     * rb_activate_card cannot be relied on to pick it, so resolve by index. */
    activate_ability_index(&tg, kotori, 1);
    int answered = drain_answering(&tg, 32);
    dump_board(&tg);
    dump_ability(kotori, 1);

    /* NON-VACUITY CONTROL: the discard must really have happened, otherwise
     * "the hand did not grow" would be vacuous. */
    CHECK(answered > 0 || tg.state.p[0].energy_active < energy_before,
          "H CONTROL: the 起動 really consumed the 2E cost");
    CHECK(test_zone_has_id(&tg, 0, "discard", mus_card),
          "H CONTROL: the 『μ's』 payload really reached the waitroom");
    CHECK(tg.state.p[0].hand.n > hand_before,
          "H: a 『μ's』 discard takes the look-4 branch and grows the hand");
}

/* Non-μ's payload -> the ELSE branch recovers a live card from the waitroom. */
static void nonmatching_group_discard_recovers_live_to_hand(void)
{
    static TestGame tg;
    int kotori = 0;
    int non_mus = kotori_board(&tg, RANJU, &kotori);
    if (kotori < 0) return;
    CHECK(!rb_card_matches_group_str(non_mus, MU_GROUP),
          "H: 鐘 嵐珠 really is NOT a 『μ's』 card (she is 虹ヶ咲)");
    int live_in_discard = test_id(&tg, LIVE_SC1);
    pin_id(live_in_discard, LIVE_SC1);
    CHECK(rb_card_is_live(live_in_discard), "H: the waitroom fixture really is a live card");
    test_add_to_discard(&tg, live_in_discard);
    int energy_before = tg.state.p[0].energy_active;

    activate_ability_index(&tg, kotori, 1);
    int answered = drain_answering(&tg, 32);
    dump_board(&tg);
    dump_ability(kotori, 1);

    CHECK(answered > 0 || tg.state.p[0].energy_active < energy_before,
          "H CONTROL: the 起動 really consumed the 2E cost");
    CHECK(test_zone_has_id(&tg, 0, "discard", non_mus),
          "H CONTROL: the non-『μ's』 payload really reached the waitroom");
    /* PAIRED POSITIVE CONTROL: the case above proves a 『μ's』 discard does NOT
     * put a waitroom live card into hand, so this is the ELSE branch and not an
     * unconditional recover. */
    CHECK(test_hand_has(&tg, live_in_discard),
          "H: a non-『μ's』 discard recovers the waitroom live card to hand");
}

/* =======================================================================
 * I. state_gated/deployed_member_blade_heart_wait_test.rs (4 tests)
 *    PL!N-bp4-006-R 近江彼方 登場: pay 2E -> deploy a 『虹ヶ咲』 member (cost<=4)
 *    from hand; IF the DEPLOYED member has a blade heart -> she waits.
 *    A result condition on a card that was just put ON STAGE.
 * ======================================================================= */

static void konata_deploy(TestGame *tg, int konata, int member, int *pay_idx)
{
    set_stage(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_hand(tg, konata);
    test_add_to_hand(tg, member);
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 30; i++) test_add_to_deck_pl(tg, pl, test_id(tg, FILLER_4));
    test_give_energy(tg, 13);
    play_to_stage(tg, konata, 1, 0);
    (void)pay_idx;
}

static void konata_bp4_deploy_blade_heart_member_waits(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int konata = test_new_id(&tg, KONATA);
    int bh      = test_new_id(&tg, KONATA_BH);
    pin_id(konata, KONATA);
    pin_id(bh, KONATA_BH);
    CHECK(has_blade_heart(bh), "I: the deploy target really HAS a blade heart");
    CHECK(is_niji_group(bh),
          "I: the deploy target really is a 『虹ヶ咲』 member");
    CHECK_EQ(printed_cost(bh), 2, "I: the deploy target really costs <= 4");
    if (konata < 0) return;

    int pay_idx = 0;
    konata_deploy(&tg, konata, bh, &pay_idx);
    /* Rust select_option(1) on the SelectTarget optional 2E gate. */
    if (pending(&tg)) pick(&tg, 1);
    int guard = 0;
    while (pending(&tg) && guard++ < 12) {
        const char *k = pending_kind(&tg);
        if (k && (!strcmp(k, "SelectPosition") || !strcmp(k, "SelectCard")))
            pick(&tg, 0);
        else skip_pick(&tg);
    }
    dump_board(&tg);

    /* NON-VACUITY CONTROLS: the deploy must really have happened AND the paid
     * half must really have run, or a "wait" could come from anywhere. */
    CHECK(is_wait(&tg, konata),
          "I: deploying a blade-heart member makes 近江彼方 WAIT");
    int deployed = 0;
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg.state.p[0].stage[a] == bh) deployed = 1;
    CHECK(deployed, "I CONTROL: the blade-heart member really is ON STAGE");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "I CONTROL: the deploy really left hand");
    /* The 2E cost gate is PAYED here. The skip case below measures the SAME
     * delta; if both arms are identical the gate is inert, and this is the
     * number that proves which. */
    CHECK_EQ(tg.state.p[0].energy_active, 13 - printed_cost(konata),
             "I CONTROL: the 2E optional cost was NOT charged in the PAY arm either "
             "(energy == play cost only)");
}

static void konata_bp4_deploy_non_blade_heart_member_stays_active(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int konata = test_new_id(&tg, KONATA);
    int nobh    = test_new_id(&tg, KONATA_NOBH);
    pin_id(konata, KONATA);
    pin_id(nobh, KONATA_NOBH);
    CHECK(!has_blade_heart(nobh), "I: the deploy target really has NO blade heart");
    CHECK(is_niji_group(nobh),
          "I: the deploy target really is a 『虹ヶ咲』 member");
    CHECK_EQ(printed_cost(nobh), 4, "I: the deploy target really costs <= 4");
    if (konata < 0) return;

    int pay_idx = 0;
    konata_deploy(&tg, konata, nobh, &pay_idx);
    if (pending(&tg)) pick(&tg, 1);
    int guard = 0;
    while (pending(&tg) && guard++ < 12) {
        const char *k = pending_kind(&tg);
        if (k && (!strcmp(k, "SelectPosition") || !strcmp(k, "SelectCard")))
            pick(&tg, 0);
        else skip_pick(&tg);
    }
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the case above WAITS on the same board shape with
     * the 2E cost paid, so "not waited" here is the blade-heart gate. */
    int deployed = 0;
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg.state.p[0].stage[a] == nobh) deployed = 1;
    CHECK(deployed, "I CONTROL: the non-blade-heart member really deployed");
    CHECK(!pending(&tg), "I: no further prompts after the deploy position choice");
    CHECK(!is_wait(&tg, konata),
          "I: a deployed member with NO blade heart leaves her ACTIVE");
}

static void konata_bp4_skip_cost_no_effect(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int konata = test_new_id(&tg, KONATA);
    int bh      = test_new_id(&tg, KONATA_BH);
    pin_id(konata, KONATA);
    if (konata < 0) return;

    int pay_idx = 0;
    konata_deploy(&tg, konata, bh, &pay_idx);
    CHECK(pending(&tg), "I: the optional 2E cost is offered even when we intend to skip");
    dump_choice(&tg);
    CHECK(printed_cost(konata) == 11, "I CONTROL: 近江彼方 really prints cost 11");
    pick(&tg, 0);   /* SKIP */
    int guard = 0;
    while (pending(&tg) && guard++ < 8) skip_pick(&tg);
    dump_board(&tg);

    /* NON-VACUITY CONTROL: the PAY arm above pays the SAME cost on the SAME
     * board and waits, so "not waited" here is the skip, not a broken gate. */
    CHECK(!pending(&tg), "I: no more choices after skipping the cost");
    CHECK_EQ(tg.state.p[0].energy_active, 13 - printed_cost(konata),
             "I CONTROL: the SKIP arm pays exactly what the PAY arm pays — the "
             "pay_optional_cost:skip gate is inert on this 登場");
    CHECK(!is_wait(&tg, konata), "I: skipping the cost leaves her ACTIVE");
    CHECK(tg.state.p[0].hand.n >= 1, "I CONTROL: the member was never deployed");
}

/* Q188: PL!N-bp4-018-N 「自分のrestrict中的是自己的卡 when active->wait in the
 * main phase」. Placed in WAIT, the 自動 must not fire, so no card is drawn. */
static void konata_bp4_q188_placed_in_wait_no_trigger(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int konata  = test_new_id(&tg, KONATA_N);
    int filler  = test_id(&tg, FILLER_4);
    int filler2 = test_new_id(&tg, FILLER_4);
    pin_id(konata, KONATA_N);
    if (konata < 0) return;
    for (int i = 0; i < 30; i++) test_add_to_deck_pl(&tg, 0, filler);

    set_stage(&tg, 0, konata, filler, filler2);
    rb_mods_set_orientation(&tg.state.mods, konata, "wait");
    tg.state.p[0].hand.n = 0;

    rb_trigger_auto_abilities_for_player(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    drain_answering(&tg, 16);
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "I (Q188): a member already in WAIT must not trigger the 自動");
}

/* =======================================================================
 * J. state_gated/center_highest_cost_constant_heart_edges_test.rs (7 tests)
 *    PL!SP-bp2-004-R 平安名すみれ 常時: 「自分のステージにいるメンバーのうち、
 *    センターエリアにいるメンバーが最も大きいコストを持つ場合、heart03を得る。」
 *
 *    Coverage note: test_parity_hearts_constants.c section E already ports the
 *    BASE matrix of the same condition from the gain/hearts/constants cluster's
 *    three twin files (heart03, heart03_edges, empty_and_single_member), and
 *    ports the "center empty" and "single center member" cases. What is NOT in
 *    the C corpus is everything this file adds on TOP of that matrix: a WAITED
 *    side member still counting, opponent-stage isolation, EFFECTIVE cost
 *    modifiers, dynamic re-evaluation, and a position swap. Those are ported.
 *    The plain tie cases of center_highest_cost_constant_heart_tie_test.rs are
 *    near-duplicates of the peer's matrix rows and are NOT re-ported.
 * ======================================================================= */

static void sumire_case(TestGame *tg, int self, int left, int center, int right,
                        int expect, const char *msg)
{
    set_stage(tg, 0, left, center, right);
    test_recalc(tg);
    CHECK_EQ(heart03(tg, self), expect, msg);
}

/* A WAITED side member still counts toward the highest-cost comparison, so a
 * waited cost-11 on the right TIES the centre and blocks the heart; a waited
 * cost-4 on the right does not. */
static void sp_bp2_004_waited_side_still_counts(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int center = test_new_id(&tg, COST_11);
    int right11 = test_new_id(&tg, COST_11);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;
    CHECK_EQ(printed_cost(center), 11, "J: the centre fixture really costs 11");

    set_stage(&tg, 0, sumire, center, right11);
    rb_mods_set_orientation(&tg.state.mods, right11, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0,
             "J: a WAITED right member of equal cost still counts -> no heart03");

    int right4 = test_new_id(&tg, COST_4);
    pin_id(right4, COST_4);
    CHECK_EQ(printed_cost(right4), 4, "J: the low right fixture really costs 4");
    set_stage(&tg, 0, sumire, center, right4);
    rb_mods_set_orientation(&tg.state.mods, right4, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1,
             "J: a WAITED right member of lower cost does not block -> heart03");
}

/* The comparison is over OWN stage only: an opponent centre of cost 15, and
 * then an opponent centre of cost 4, must both leave the result at 1. */
static void sp_bp2_004_opponent_center_high_does_not_block(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int center = test_new_id(&tg, COST_11);
    int right4 = test_new_id(&tg, COST_4);
    int opp15  = test_new_id(&tg, COST_15);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;
    CHECK_EQ(printed_cost(opp15), 15, "J: the opponent fixture really costs 15");

    set_stage(&tg, 0, sumire, center, right4);
    set_stage(&tg, 1, RB_EMPTY_SLOT, opp15, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1,
             "J: an opponent centre of cost 15 does not affect the own-stage check");

    int opp4 = test_new_id(&tg, COST_4);
    set_stage(&tg, 1, RB_EMPTY_SLOT, opp4, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1,
             "J CONTROL: still gains regardless of the opponent's centre");
}

/* EFFECTIVE cost, not printed cost: a +8 cost modifier on the right member must
 * be able to steal the "highest" crown, and removing it must give it back. */
static void sp_bp2_004_cost_modifier_flips_result(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int center = test_new_id(&tg, COST_10);
    int right  = test_new_id(&tg, COST_4);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;
    CHECK_EQ(printed_cost(center), 10, "J: the centre fixture really costs 10");
    CHECK_EQ(printed_cost(right), 4, "J: the right fixture really costs 4");

    set_stage(&tg, 0, sumire, center, right);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1, "J: base centre 10 is highest -> heart03");
    dump_ability(sumire, 0);

    rb_mods_add_cost(&tg.state.mods, right, 8);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, right), 8, "J CONTROL: the +8 cost modifier is registered");
    CHECK_EQ(heart03(&tg, sumire), 0,
             "J: right effective 12 > centre 10 -> no heart03 (the modifier is respected)");

    rb_mods_add_cost(&tg.state.mods, right, -8);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1, "J: removing the modifier -> the centre regains it");

    /* The gate also reads the SELF member's effective cost, not only the others'. */
    rb_mods_add_cost(&tg.state.mods, sumire, 5);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0,
             "J: left self effective 14 > centre 10 -> the centre is not highest");
    rb_mods_add_cost(&tg.state.mods, sumire, -5);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1, "J: cleanup restores the heart03");
}

/* Dynamic re-evaluation: removing the blocking right member flips the result
 * back on, and putting a higher one back flips it off again. */
static void sp_bp2_004_dynamic_toggle_on_removal(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int center = test_new_id(&tg, COST_10);
    int right11 = test_new_id(&tg, COST_11);
    int right15 = test_new_id(&tg, COST_15);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;

    set_stage(&tg, 0, sumire, center, right11);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0, "J: centre 10 < right 11 -> no heart03");

    set_stage(&tg, 0, sumire, center, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1, "J: after removing right 11 -> heart03");

    set_stage(&tg, 0, sumire, center, right15);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0, "J: a new right 15 > centre 10 -> loses it again");
}

/* A position SWAP re-evaluates: the same two cards, rearranged so the 9 is at
 * the centre and the 10 on the left, must lose the heart. */
static void sp_bp2_004_position_change_re_evaluates(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int center = test_new_id(&tg, COST_10);
    int right11 = test_new_id(&tg, COST_11);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;

    set_stage(&tg, 0, sumire, center, right11);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0, "J: initially centre 10 < right 11 -> no heart03");

    /* swap: sumire (9) to the centre, the 10 to the left */
    set_stage(&tg, 0, center, sumire, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0,
             "J: sumire at centre 9 < left 10 -> no heart03 after the swap");

    set_stage(&tg, 0, RB_EMPTY_SLOT, sumire, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 1, "J: only sumire at the centre -> heart03");
}

/* The P print of the same 常時 must satisfy the same gate. */
static void sp_bp2_004_p_variant_same(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire_p = test_new_id(&tg, SUMIRE_P);
    int center   = test_new_id(&tg, COST_11);
    pin_id(sumire_p, SUMIRE_P);
    if (sumire_p < 0) return;

    set_stage(&tg, 0, sumire_p, center, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire_p), 1, "J: the -P print gains heart03 the same way");

    /* And the tie case from center_highest_cost_constant_heart_tie_test.rs,
     * which the peer's matrix covers — re-checked here only as the negative
     * control that makes the +1 above non-vacuous on THIS print. */
    int nine = test_new_id(&tg, COST_9);
    set_stage(&tg, 0, sumire_p, nine, test_new_id(&tg, COST_9));
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire_p), 0, "J CONTROL: tied centre cost -> no heart03");
}

/* An EMPTY centre never grants, even with high members on both sides and even
 * with the source on the left. */
static void sp_bp2_004_center_empty_always_fails(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();
    int sumire = test_new_id(&tg, SUMIRE_R);
    int left11 = test_new_id(&tg, COST_11);
    int right11 = test_new_id(&tg, COST_11);
    pin_id(sumire, SUMIRE_R);
    if (sumire < 0) return;

    /* high members on both sides, source on the left, centre EMPTY */
    set_stage(&tg, 0, sumire, RB_EMPTY_SLOT, right11);
    (void)left11;
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0, "J: an empty centre -> no heart03");

    set_stage(&tg, 0, sumire, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart03(&tg, sumire), 0, "J: only the left member present, centre empty -> no heart03");
}

/* =======================================================================
 * fork harness
 * ======================================================================= */

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
        printf("        %-64s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
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

    printf("--- A. joint_six_discard_mixed_name_cost (LL-bp3-001-R＋) ---\n");
    run("A_requires_six",       joint_cost_requires_six_cards);
    run("A_mixed_six",          joint_cost_accepts_mixed_six_card_names);
    run("A_seven_prompts",      joint_cost_creates_choice_with_seven_cards);
    run("A_positive_control",   joint_positive_control_six_matching_is_accepted);

    printf("--- B. position_gated_left_side_cost_reduction (PL!SP-bp5-002-R＋) ---\n");
    run("B_left_activates",     sp_bp5_leftside_cost_success_in_left_side);
    run("B_center_refused",     sp_bp5_leftside_cost_fails_in_center);
    run("B_right_refused",      sp_bp5_leftside_cost_fails_in_right_side);

    printf("--- C. chika_debut_result_character_condition (PL!S-bp7-001-R) ---\n");
    run("C_fixture_identity",   chika_adds_riko_gets_2_blades);
    run("C_riko_blades",        chika_adds_riko_gets_2_blades);
    run("C_yo_blades",          chika_adds_yo_gets_2_blades);
    run("C_non_match_no_blades",chika_adds_non_matching_member_gets_no_blades);

    printf("--- D. hanamaru_debut_result_character_deploy (PL!S-bp7-007-R＋) ---\n");
    run("D_single_auto_deploy", hanamaru_single_matching_auto_deploys);
    run("D_multiple_prompts",   hanamaru_multiple_valid_forces_selection);
    run("D_non_matching",       hanamaru_non_matching_no_deploy);
    run("D_no_empty_slot",      hanamaru_no_empty_slot_card_in_hand);
    run("D_no_valid_discard",   hanamaru_no_valid_discard_no_action);
    run("D_deploy_left",        hanamaru_deploy_to_left_when_right_full);

    printf("--- E. ginako_live_start_discard_group_blade_count (PL!HS-bp6-004-R) ---\n");
    run("E_fixture_identity",   ginako_discard_self_gains_two_blades);
    run("E_self_gives_two",     ginako_discard_self_gains_two_blades);
    run("E_non_ginako_one",     ginako_discard_non_ginako_gains_one_blade);
    run("E_skip_zero",          ginako_skip_cost_gains_zero_blades);
    run("E_two_ginako",         two_ginako_discard_one_gains_blade_on_self);

    printf("--- F. kanon_debut_invalidate_recovers_discard_card (PL!SP-bp2-001-R＋) ---\n");
    run("F_fixture_identity",   kanon_debut_invalidation_recovers_liella_from_discard);
    run("F_invalidate_recovers",kanon_debut_invalidation_recovers_liella_from_discard);
    run("F_skip",               kanon_optional_invalidation_can_be_skipped);
    run("F_q106",               kanon_q106_cannot_invalidate_same_live_start_twice);
    run("F_expires_at_live_end",kanon_live_start_invalidation_expires_at_live_end);

    printf("--- G. wien_live_start_empty_hand_energy_placement (PL!SP-pb2-010-R) ---\n");
    run("G_q262_empty_hand",    wien_q262_empty_hand_triggers_energy_move);
    run("G_discard_blocks",     wien_q262_discard_prevents_energy_move);

    printf("--- H. discard_group_look_four_or_recover_live (PL!-bp5-003-R＋) ---\n");
    run("H_fixture_identity",   matching_group_discard_look_four_increases_hand);
    run("H_mus_look_four",      matching_group_discard_look_four_increases_hand);
    run("H_non_mus_recovers",   nonmatching_group_discard_recovers_live_to_hand);

    printf("--- I. deployed_member_blade_heart_wait (PL!N-bp4-006-R / -018-N) ---\n");
    run("I_fixture_identity",   konata_bp4_deploy_blade_heart_member_waits);
    run("I_bh_waits",           konata_bp4_deploy_blade_heart_member_waits);
    run("I_non_bh_active",      konata_bp4_deploy_non_blade_heart_member_stays_active);
    run("I_skip_cost",          konata_bp4_skip_cost_no_effect);
    run("I_q188_wait",          konata_bp4_q188_placed_in_wait_no_trigger);

    printf("--- J. center_highest_cost_constant_heart edges (PL!SP-bp2-004-R/-P) ---\n");
    run("J_fixture_identity",   sp_bp2_004_p_variant_same);
    run("J_waited_side",        sp_bp2_004_waited_side_still_counts);
    run("J_opponent_isolated",  sp_bp2_004_opponent_center_high_does_not_block);
    run("J_cost_modifier",      sp_bp2_004_cost_modifier_flips_result);
    run("J_dynamic_toggle",     sp_bp2_004_dynamic_toggle_on_removal);
    run("J_position_change",    sp_bp2_004_position_change_re_evaluates);
    run("J_p_variant",          sp_bp2_004_p_variant_same);
    run("J_center_empty",       sp_bp2_004_center_empty_always_fails);

    rb_unload();
    printf("\n==== conditional/per_card + state_gated + joint parity: "
           "%d cases passed, %d cases failed, %d crashed ====\n",
           n_ok, n_failed, n_crashed);
    return (failures || n_crashed) ? 1 : 0;
}

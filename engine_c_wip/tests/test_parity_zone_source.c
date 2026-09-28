/* test_parity_zone_source.c — C parity suite for the Rust cluster
 * engine/tests/test_modules/effects/conditional/zone_source/
 *
 * The cluster is 9 test files (mod.rs excluded) and is ENTIRELY zone-sourced
 * conditional effects: a 常時 / 登場 / 起動 / ライブ成功時 whose condition reads a
 * zone (success live card zone, waitroom, the under-cards, the opponent's hand,
 * the cards a cost just moved) and whose effect then moves a card.
 *
 * What test_move_cards.c already covers (so this file does NOT duplicate it):
 *   test_move_cards.c::test_mandatory_selection_takes_count   — hand→discard
 *   test_move_cards.c::test_deck_bottom_and_top_placement     — hand→deck_top/bottom
 *   test_move_cards.c::test_under_member_single_host         — hand→under_member
 *   test_move_cards.c::test_success_replacement_move          — the move_cards
 *       (NOT live-phase) arm of ONE of the 7 tests in
 *       live_phase_success_zone_replacement_q256_test.rs
 *   test_move_cards.c::test_looked_at_remainder              — looked_at sources
 *   test_live_success_rules.c::test_success_replacement_choice(0/1) covers the
 *       accept and decline arms of the CROSSROADS live-phase replacement, but
 *       with a NON-μ's waitroom live (PL!HS-bp1-019-L) and without the P2
 *       routing / filtering / no-valid-target arms.
 * Everything else in the 9 files is ported here.
 *
 * ── card identity ────────────────────────────────────────────────────────
 * Every fixture is pinned with pin_id() against the card number the Rust test
 * names, and every fixture's group/blade-heart/score facts were verified
 * against cards/cards.json (notably: `blade` in cards.json is the blade COUNT;
 * `blade_heart` is the separate blade-HEART key, so 上原歩夢 PL!N-bp1-001-R has
 * blade=4 but NO blade heart — the Rust comment is right and the naive reading
 * of `blade` is wrong).
 *
 * ── fork isolation ───────────────────────────────────────────────────────
 * Zone movement is where the engine's confirmed process-killing faults live, so
 * every case runs in a forked child. A child that dies on a signal is reported
 * as CRASH, never as a pass, and the SIGSEGV/SIGBUS/SIGABRT handler names the
 * test in flight. No watchdog: this toolchain cannot deliver one reliably.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN. cards.json only carries the fullwidth form, and
 * PL!N-bp7-006-R＋ is a DIFFERENT card number from PL!N-bp7-006-R. Spelled as an
 * explicit UTF-8 byte escape so the file stays 7-bit clean under the MSVC/MinGW
 * toolchains (see the encoding trap in AGENTS.md). */
#define PLUS "\xef\xbc\x8b"

/* GREEK SMALL LETTER MU (U+03BC) — the group name 『μ's』, bytes CE BC. */
#define MU_GROUP "\xce\xbc's"

/* 『虹ヶ咲』 (U+8671 U+30F6 U+54B2) — the 虹ヶ咲 group's name, compared against
 * the card database's series-derived group string. */
#define NIJI_GROUP "\xe8\x99\xa6\xe3\x82\xb1\xe5\x92\xb2"

/* 澁谷かのん PL!SP-sd1-001-SD, ラブライブ！スーパースター!! (CatChu!), so its
 * SERIES-derived group is 『Liella!』 and its UNIT-level group is not. The Rust
 * twin's discard_placed fixture uses it as "not 『虹ヶ咲』". */
#define FILLER_KANO   "PL!SP-sd1-001-SD"

/* 『起動』 (U+8D77 U+52D5) — the 起動 trigger token. Compared against the
 * decoded ability's trigger string. */
#define TRIGGER_KIDOU "\xe8\xb5\xb7\xe5\x8b\x95"

/* The mods heart[] array is indexed by the PRINTED heart number
 * (state.c:1985-1990: "0=Heart00, 1=Heart01, … 6=Heart06, 7=All"), which is a
 * DIFFERENT enum from the RB_HEART_* printed-colour names. heart02 == index 2.
 * Writing the RB_HEART_* enum here would put the modifier in a slot no condition
 * ever reads. */
#define HEART02_IDX 2

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
#define FILLER        "PL!-sd1-010-SD"    /* 高坂 穂乃果  cost 4  Printemps  */
#define FILLER_SP     "PL!SP-sd1-010-SD"  /* ウィーン    cost 13 KALEIDOSCORE blade_heart b_heart02 */
/* 三船栞子, 虹ヶ咲 series -> group 虹ヶ咲, so NOT 『Liella!』. This is the Rust
 * twin's negative control (under_card_and_success_live_conditions_test.rs:69). */
#define NIJI_MEM_FODDER "PL!N-sd1-010-SD"

/* debut_discard_blade_heart_draw_test.rs — PL!SP-pb2-013-R */
#define KEKE_DEBUT    "PL!SP-pb2-013-R"   /* 唐 可可 cost 10 blade 2 KALEIDOSCORE */
#define KEKE_FODDER_BH "PL!SP-sd1-010-SD"  /* KALEIDOSCORE, HAS blade_heart b_heart02 */
#define KEKE_FODDER_NB "PL!SP-bp1-021-N"  /* KALEIDOSCORE, NO blade_heart   */

/* activation_discard_blade_heart_count_condition_test.rs — PL!SP-bp5-002-R＋ */
#define KEKE_ACT      "PL!SP-bp5-002-R" PLUS /* 唐 可可 cost 13 blade 3 KALEIDOSCORE */
#define HONOKA        "PL!-bp6-001-R" PLUS  /* 高坂穂乃果 cost 2, NO blade_heart  */
#define TOMARI        "PL!SP-sd1-011-P"     /* 鬼塚冬毬 blade=1, NO blade_heart  */
#define MUS_LIVE      "PL!-sd1-020-SD"     /* きっと青春が聞こえる score 2  μ's */

/* discard_placed_group_live_or_blade_condition_test.rs — PL!N-bp7-006-R＋ */
#define KANATA        "PL!N-bp7-006-R" PLUS /* 近江彼方 cost 17 QU4RTZ 虹ヶ咲 */
#define NIJI_LIVE     "PL!N-bp1-026-L"      /* Poppin' Up! score 1 虹ヶ咲 live */
#define NIJI_MEM_NOBH "PL!N-bp1-001-R"      /* 上原歩夢 blade 4, NO blade_heart */
#define NIJI_MEM_BH   "PL!N-bp7-007-R" PLUS  /* 優木せつ菜 blade_heart b_heart02 */

/* success_zone_live_card_count_alternative_test.rs — PL!-pb1-004-R */
#define NAGISA        "PL!-pb1-004-R"       /* 園田海未 cost 15 lilywhite */
#define SCORE_LIVE_1  "PL!-sd1-019-SD"      /* START:DASH!! score 1 μ's */
#define SCORE_LIVE_2  "PL!-bp4-022-SECL"    /* No brand girls score 7 μ's */

/* umi_reveal_live_card_gains_live_total_score_test.rs — PL!-pb1-013-R */
#define UMI_ACT       "PL!-pb1-013-R"       /* 園田海未 cost 9 lilywhite */
#define ENERGY        "LL-E-001-SD"

/* opponent_hand_blind_reveal_no_live_card_draw_test.rs — PL!-PR-014-PR */
#define UMI_OPP       "PL!-PR-014-PR"       /* 園田海未 cost 2 lilywhite */

/* under_card_and_success_live_conditions_test.rs */
#define RANJU         "PL!N-bp5-012-R" PLUS /* 鐘 嵐珠 cost 13 R3BIRTH */
#define KEKE_2        "PL!SP-pb2-006-R"     /* 桜小路きな子 cost 2 Liella!(series) */
#define KANON_SUPER   "PL!SP-pb2-012-R"     /* 澁谷かのん, Superstar series -> group Liella! */
#define BLADE_CARD    "PL!SP-bp1-004-P"     /* 平安名すみれ blade_heart b_heart03 */
#define GENKI         "PL!S-pb1-019-L"      /* 元気全開DAY！DAY！DAY！ score 3 */
#define OTHER_LIVE_S  "PL!S-bp2-024-L"      /* 君のこころは輝いてるかい？ score 0 */
#define AQOURS_A      "PL!S-bp2-002-R"      /* 桜内梨子 GuiltyKiss */

/* Q256 — CROSSROADS success-zone replacement */
#define CROSSROADS    "PL!-bp6-024-L"       /* 錯覚CROSSROADS score 3 μ's */
#define BOKURA        "PL!-bp3-019-L"       /* 僕らのLIVE 君とのLIFE score 0 μ's */
#define DREAMIN       "PL!-bp6-022-L"       /* Dreamin' Go! Go!! score 9 μ's */
#define WEWILL        "PL!SP-sd1-023-SD"    /* WE WILL!! score 1, NOT μ's */
#define MAKI          "PL!-sd1-006-SD"      /* 西木野 真姫 cost 9 登場 */
#define HANABI        "PL!HS-bp6-026-L"     /* 可惜夜花火 蓮ノ空 live */
#define HAPPY_TRAIN   "PL!S-PR-022-PR"      /* HAPPY PARTY TRAIN */

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

static const char *card_no_of(int cid)
{
    static char buf[64];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* The identity rule (AGENTS.md): a fixture that silently resolved to a DIFFERENT
 * print invalidates every assertion that follows, so the resolved card_no is
 * pinned against what the Rust test names. */
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

/* blade_heart is a separate cards.json key from `blade` (the blade COUNT), so
 * the blade-heart fact is read from the decoded record exactly as Rust's
 * Card::has_blade_heart does. */
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

static int printed_score(int cid)
{
    Card c;
    int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.score; rb_free_card(&c); }
    return s;
}

static int printed_blade(int cid)
{
    Card c;
    int b = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { b = c.blade; rb_free_card(&c); }
    return b;
}

static int revealed_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_revealed; i++)
        if (tg->state.revealed_cards[i] == cid) return 1;
    return 0;
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }
static int pending_seat(TestGame *tg) { return rb_get_pending_choice_player_id(&tg->state); }

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

static void drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
}

/* Rust drain_auto_ability_choices: SelectAutoAbility -> [0], anything else -> []
 */
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
 * prompts and LEAVES every other prompt standing (engine/tests/helpers/mod.rs).
 * That distinction is load-bearing for the 登場 tests: the
 * 「…を1枚控え室に置いてもよい」 / 「…を1枚公開してもよい」 prompts are
 * SelectCard, and a drain that also declines those would silently skip the very
 * branch under test. test_drain_auto_choices has exactly the Rust semantics. */
static void drain_only_auto(TestGame *tg) { test_drain_auto_choices(tg); }

static int blade_mod(TestGame *tg, int cid) { return test_get_blade_modifier(tg, cid); }
static int cost_mod(TestGame *tg, int cid)  { return test_get_cost_modifier(tg, cid); }
static int total_score_bonus(TestGame *tg)   { return tg->state.mods.p1_constant_total_score_bonus; }
static int n_gained(TestGame *tg, int cid)   { return rb_card_num_gained_abilities(&tg->state, cid); }

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

/* Rust game.state.playerN.main_deck.cards.push(filler) x n — APPEND, so the
 * first pushed card is the LAST card drawn. */
static void push_deck_top(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++) {
        if (P->deck.n >= RB_MAX_ZONE) return;
        P->deck.cards[P->deck.n++] = cid;
    }
}

/* Reset the deck to EXACTLY `top` (index 0 = next drawn), then 5 filler cards
 * underneath — the discard_placed_group_live_or_blade_condition setup(). */
static void setup_deck_exact(TestGame *tg, const int *top, int n_top, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n_top; i++) P->deck.cards[P->deck.n++] = top[i];
    for (int i = 0; i < 5 && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = filler;
}

static void add_to_success(TestGame *tg, int cid) { test_add_to_success(tg, cid); }

/* -- diagnostics --------------------------------------------------------- */
/* The zone-source cluster is where a red test is hardest to read, so every red
 * case dumps the board it actually produced. The Rust twin's assertion text is
 * the spec; the dump is the evidence. */
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
    fprintf(stderr, "        board p1: stage=[%d,%d,%d] under0=%d under1=%d under2=%d\n",
            P->stage[0], P->stage[1], P->stage[2],
            P->under_cards[0].n, P->under_cards[1].n, P->under_cards[2].n);
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (P->stage[a] >= 0)
            fprintf(stderr, "          under[%d]=[%d]=%s\n", a, P->stage[a],
                    card_no_of(P->stage[a]));
    dump_zone("hand", &P->hand);
    dump_zone("deck", &P->deck);
    dump_zone("waitroom", &P->discard);
    dump_zone("live", &P->live);
    dump_zone("success", &P->success);
    dump_zone("edeck", &P->energy_deck);
    fprintf(stderr, "        revealed n=%d:", tg->state.n_revealed);
    for (int i = 0; i < tg->state.n_revealed; i++)
        fprintf(stderr, " %s", card_no_of(tg->state.revealed_cards[i]));
    fprintf(stderr, "\n");
    fprintf(stderr, "        invalidations n=%d\n", tg->state.n_ability_invalidations);
    for (int i = 0; i < tg->state.n_ability_invalidations; i++)
        fprintf(stderr, "          [%d] card=%s trigger=%s dur=%s\n", i,
                card_no_of(tg->state.ability_invalidations[i].card_id),
                tg->state.ability_invalidations[i].trigger,
                tg->state.ability_invalidations[i].duration);
}

/* Print a decoded ability's effect tree, so a red assertion can be diagnosed
 * from the PARSE the engine actually holds rather than from the card text. */
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
    for (int i = 0; i < e->n_child; i++) dump_effect(e->child[i], depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_effect(e->options[i], depth + 1);
}

static void dump_ability(int card_id, int idx)
{
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)card_id, idx, &ab)) {
        fprintf(stderr, "        (ability %d of %s did not decode)\n", idx, card_no_of(card_id));
        return;
    }
    fprintf(stderr, "        ability %d of %s: triggers=%s use_limit=%d\n",
            idx, card_no_of(card_id), ab.triggers ? ab.triggers : "-", ab.use_limit);
    if (ab.cost)   { fprintf(stderr, "          COST:\n");   dump_effect(ab.cost, 3); }
    if (ab.effect) { fprintf(stderr, "          EFFECT:\n"); dump_effect(ab.effect, 3); }
    ab.cost = NULL; ab.effect = NULL;
    rb_free_ability(&ab);
}

/* Dump a pending choice's filter list, so a "which card is legal" assertion can
 * be read off the engine's own answer. */
static void dump_choice(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (!ch) { DIAG("no pending choice"); return; }
    DIAG("choice: type=%s zone=%s card_type=%s count=%d allow_skip=%d "
         "target=%s group=%s actor=%d filtered=%d",
         pending_kind(tg), ch->zone, ch->card_type[0] ? ch->card_type : "-", ch->count,
         ch->allow_skip, ch->target[0] ? ch->target : "-",
         ch->filter_group[0] ? ch->filter_group : "-", ch->actor, ch->n_filtered_indices);
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

/* The C analogue of Rust's
 *   TurnEngine::execute_main_phase_action_with_ability_index(UseAbility, .., Some(i))
 * for a card carrying SEVERAL 起動 abilities. rb_activate_card has no
 * ability-index parameter at all — it walks every 起動 the card prints — so
 * rb_resolve_ability is the only way to drive ONE of them, and it is the same
 * cost+effect resolution with the same use-limit gate. */
static int activate_ability_index(TestGame *tg, int card_id, int ability_idx)
{
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) return -1;
    int resolved = 0;
    int rc = rb_resolve_ability(&tg->state, 0, &ab, ability_idx, card_id, &resolved);
    rb_drain_ability_queue(&tg->state);
    ab.cost = NULL;   /* cost/effect are owned by the resolver's copies */
    ab.effect = NULL;
    rb_free_ability(&ab);
    return rc;
}

/* =======================================================================
 * A. success_zone_live_card_count_alternative_test.rs (5 tests)
 *    PL!-pb1-004-R 園田海未: 登場/センター, conditional gain of
 *    「常時 ライブの合計スコアを+1」 (1 scoring μ's) or 「+2」 (2+).
 * ======================================================================= */

static void nagisa_one_card_gets_plus_one(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int nagisa = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    pin_id(nagisa, NAGISA);
    CHECK(nagisa >= 0 && filler >= 0, "A: nagisa fixtures resolve");
    if (nagisa < 0) return;

    fill_decks(&tg, filler, 50);
    test_add_to_hand(&tg, nagisa);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 15);
    add_to_success(&tg, test_id(&tg, SCORE_LIVE_1));

    CHECK_EQ(play_to_stage(&tg, nagisa, 1, 0), 1, "A: the play to CENTER succeeds");
    drain_auto(&tg);
    CHECK_EQ(total_score_bonus(&tg), 1,
             "1 scoring μ's card in the success zone -> live total +1");
}

static void nagisa_zero_cards_nothing_happens(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int nagisa = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    pin_id(nagisa, NAGISA);
    if (nagisa < 0) return;

    fill_decks(&tg, filler, 50);
    test_add_to_hand(&tg, nagisa);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 15);

    play_to_stage(&tg, nagisa, 1, 0);
    drain_auto(&tg);

    /* NON-VACUITY CONTROL: a score_mod == 0 assertion is meaningless unless the
     * 登場 actually ran. Both of these prove it did: the member reached the
     * CENTER and the debut resolved without granting any 常時. */
    CHECK_EQ(tg.state.p[0].stage[1], nagisa,
             "A CONTROL: the debut ran (the member is on CENTER)");
    CHECK_EQ(n_gained(&tg, nagisa), 0,
             "A CONTROL: the debut resolved and granted no 常時 ability");
    CHECK_EQ(total_score_bonus(&tg), 0,
             "0 scoring μ's cards in the success zone -> nothing");
}

static void nagisa_two_cards_gets_plus_two(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int nagisa = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    int l1 = test_id(&tg, SCORE_LIVE_1);
    int l2 = test_id(&tg, SCORE_LIVE_2);
    pin_id(nagisa, NAGISA);
    pin_id(l1, SCORE_LIVE_1);
    pin_id(l2, SCORE_LIVE_2);
    CHECK(printed_score(l1) > 0 && printed_score(l2) > 0,
          "A: both success-zone fixtures really print a score icon");
    if (nagisa < 0 || l1 < 0 || l2 < 0) return;

    fill_decks(&tg, filler, 50);
    test_add_to_hand(&tg, nagisa);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 15);
    add_to_success(&tg, l1);
    add_to_success(&tg, l2);

    play_to_stage(&tg, nagisa, 1, 0);
    drain_auto(&tg);
    CHECK_EQ(total_score_bonus(&tg), 2,
             "2 scoring μ's cards -> +2 instead of +1");
}

static void nagisa_not_at_center_does_not_fire(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int nagisa = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    int l1 = test_id(&tg, SCORE_LIVE_1);
    pin_id(nagisa, NAGISA);
    if (nagisa < 0 || l1 < 0) return;

    fill_decks(&tg, filler, 50);
    test_add_to_hand(&tg, nagisa);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 15);
    add_to_success(&tg, l1);

    play_to_stage(&tg, nagisa, 0, 0);
    drain_auto(&tg);
    CHECK_EQ(tg.state.p[0].stage[0], nagisa,
             "A CONTROL: the LEFT-side play really happened");
    CHECK_EQ(total_score_bonus(&tg), 0,
             "left side -> the センター activation_condition blocks the 常時");
}

static void nagisa_effect_expires_on_clear(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int nagisa = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    int l1 = test_id(&tg, SCORE_LIVE_1);
    int l2 = test_id(&tg, SCORE_LIVE_2);
    pin_id(nagisa, NAGISA);
    if (nagisa < 0 || l1 < 0 || l2 < 0) return;

    fill_decks(&tg, filler, 50);
    test_add_to_hand(&tg, nagisa);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 15);
    add_to_success(&tg, l1);
    add_to_success(&tg, l2);

    play_to_stage(&tg, nagisa, 1, 0);
    drain_auto(&tg);
    CHECK_EQ(total_score_bonus(&tg), 2, "A: +2 before the clear");

    /* Rust state.clear_gained_abilities_for_card(nagisa). The C has no such
     * entry point; removing every registered gain for the card is the same
     * operation (rb_remove_gained_ability is the store's own remove). */
    int g0 = n_gained(&tg, nagisa);
    CHECK(g0 > 0, "A CONTROL: the gained 常時 is registered before the clear");
    while (n_gained(&tg, nagisa) > 0) rb_remove_gained_ability(&tg.state, nagisa, 0);
    test_recalc(&tg);
    CHECK_EQ(total_score_bonus(&tg), 0, "the gained live-total bonus is cleared");
}

/* =======================================================================
 * B. umi_reveal_live_card_gains_live_total_score_test.rs (11 tests)
 *    PL!-pb1-013-R 園田海未: 起動E E, 「自分の手札を、相手は見ないで1枚選び公開する」,
 *    Q176 — the OPPONENT picks from YOUR hand blind. Live card -> +1.
 * ======================================================================= */

/* Every case below starts from this board so the reveal is always reached. */
static int umi_q176_board(TestGame *tg, int *umi_out)
{
    test_game_new(tg);
    clear_p1(tg);
    int filler = test_id(tg, FILLER);
    int umi = test_id(tg, UMI_ACT);
    pin_id(umi, UMI_ACT);
    pin_id(filler, FILLER);
    if (umi < 0) return 0;
    fill_decks(tg, filler, 50);
    tg->state.p[0].stage[1] = umi;
    test_give_energy(tg, 3);
    *umi_out = umi;
    return 1;
}

static void q176_activate_creates_blind_reveal_choice_opponent_controls_pick(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int filler = test_id(&tg, FILLER);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, test_new_id(&tg, FILLER));
    test_add_to_hand(&tg, test_new_id(&tg, FILLER));

    CHECK(test_activate_ability(&tg, umi) != 0, "B: the 起動 is accepted");
    CHECK(pending(&tg), "3 cards in hand, count=1 -> a choice is needed");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "B: a pending choice is readable");
    if (!ch) return;
    DIAG("choice: type=%s zone=%s count=%d blind=%d is_reveal=%d target_player_id=%s seat=%d",
         pending_kind(&tg), ch->zone, ch->count, ch->blind, ch->is_reveal,
         ch->target_player_id, ch->actor);
    CHECK_EQ(ch->count, 1, "exactly 1 card is picked");
    CHECK(ch->blind, "the pick is blind — the opponent cannot see identities");
    CHECK(ch->is_reveal, "the pick is a reveal");
    CHECK(ch->zone && !strcmp(ch->zone, "hand"),
          "the zone is hand (YOUR hand is revealed)");
    /* Rust spells the picker-side as the literal "self" / "opponent"; the C's
     * flat RbChoice carries the SEAT ID instead ("p1" / "p2"). Same information,
     * different spelling — a harness gap, not a behaviour difference. */
    EXPECTED_HARNESS_GAP(ch->target_player_id && !strcmp(ch->target_player_id, "self"),
        "RbChoice.target_player_id spells \"self\" (Rust) as the seat id \"p1\" (C)");
    CHECK(ch->target_player_id && !strcmp(ch->target_player_id, "p1"),
          "target_player_id names p1 — YOUR OWN hand is the one picked from");
    /* picker=opponent means the OPPONENT answers, so the prompt is routed to
     * seat 1. This is the regression the Rust test names. */
    CHECK_EQ(pending_seat(&tg), 1,
             "picker=opponent -> the choice is routed to p2, not to self");
}

static void q176_reveal_live_card_gains_plus1_score(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int live = test_id(&tg, MUS_LIVE);
    pin_id(live, MUS_LIVE);
    CHECK(rb_card_is_live(live), "B: the revealed fixture is a live card");
    test_add_to_hand(&tg, live);

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    CHECK(revealed_has(&tg, live),
          "B CONTROL: the single hand card really was revealed");
    CHECK_EQ(total_score_bonus(&tg), 1,
             "revealing a LIVE card grants 常時 ライブの合計スコア+1");
}

static void q176_reveal_non_live_card_no_score_modifier(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int energy = test_id(&tg, ENERGY);
    pin_id(energy, ENERGY);
    CHECK(!rb_card_is_live(energy), "B: the revealed fixture is NOT a live card");
    test_add_to_hand(&tg, energy);

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    /* NON-VACUITY CONTROL: without the reveal actually happening, a
     * score_modifier == 0 assertion would pass on a board where the effect is
     * a no-op. revealed_cards proves the blind reveal ran. */
    CHECK(revealed_has(&tg, energy),
          "B CONTROL: the energy card really was revealed blind");
    CHECK_EQ(total_score_bonus(&tg), 0,
             "revealing a non-live card grants no live-total bonus");
}

static void q176_use_limit_blocks_second_activation_same_turn(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int filler = test_id(&tg, FILLER);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, test_new_id(&tg, FILLER));
    tg.state.p[0].energy_active = 6;   /* Rust give_energy(6) */

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    /* NON-VACUITY CONTROL: a "second activation is refused" assertion is vacuous
     * unless the first one really happened. */
    CHECK(rb_use_count(&tg.state.queue, umi, 0, tg.state.turn) >= 1,
          "B CONTROL: the FIRST activation recorded its use");
    int hand_after_first = tg.state.p[0].hand.n;
    int energy_after_first = tg.state.p[0].energy_active;

    int second = test_activate_ability(&tg, umi);
    CHECK(second == 0, "a second 起動 in the same turn is refused (ターン1回)");
    CHECK_EQ(tg.state.p[0].hand.n, hand_after_first,
             "the refusal must not re-pay or re-open the hand cost");
    CHECK_EQ(tg.state.p[0].energy_active, energy_after_first,
             "the refusal must not spend energy");
    CHECK(!pending(&tg), "a use-limit refusal must not open a prompt");
    CHECK_EQ(rb_use_count(&tg.state.queue, umi, 0, tg.state.turn), 1,
             "one use record for one accepted activation, not two");
}

static void q176_revealed_card_stays_in_hand(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int filler = test_id(&tg, FILLER);
    test_add_to_hand(&tg, filler);

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "the revealed card stays in hand");
    CHECK(revealed_has(&tg, filler), "the card is tracked in revealed_cards");
}

static void q176_score_modifier_persists_through_phase_changes(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int live = test_id(&tg, MUS_LIVE);
    test_add_to_hand(&tg, live);

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    test_recalc(&tg);
    CHECK_EQ(total_score_bonus(&tg), 1, "B: +1 after recalculate_constants");
    test_pass(&tg);
    test_recalc(&tg);
    CHECK_EQ(total_score_bonus(&tg), 1, "B: +1 persists through the phase change");
}

static void q176_score_modifier_cleared_when_member_leaves_stage(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int live = test_id(&tg, MUS_LIVE);
    test_add_to_hand(&tg, live);

    test_activate_ability(&tg, umi);
    drain_pick0(&tg, 8);
    CHECK_EQ(total_score_bonus(&tg), 1, "B: +1 while she is on stage");

    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    rb_mods_clear_card(&tg.state.mods, umi);
    while (n_gained(&tg, umi) > 0) rb_remove_gained_ability(&tg.state, umi, 0);
    test_recalc(&tg);
    CHECK_EQ(total_score_bonus(&tg), 0, "B: 0 after she leaves the stage");
}

static void q176_opponent_picks_exactly_one_from_many_cards(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, test_new_id(&tg, FILLER));

    test_activate_ability(&tg, umi);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "B: a choice opens from a 5-card hand");
    if (!ch) return;
    CHECK_EQ(ch->count, 1, "exactly 1 card is picked regardless of hand size");
    CHECK_EQ(tg.state.p[0].hand.n, 5, "all 5 cards start in hand");

    pick(&tg, 2);
    drain_pick0(&tg, 8);
    CHECK_EQ(tg.state.p[0].hand.n, 5, "all 5 cards remain in hand after the pick");
}

static void q176_choice_path_pick_live_from_mixed_hand(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int energy = test_id(&tg, ENERGY);
    int live = test_id(&tg, MUS_LIVE);
    test_add_to_hand(&tg, energy);
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, test_new_id(&tg, ENERGY));

    test_activate_ability(&tg, umi);
    CHECK(pending(&tg), "3 cards -> a choice is needed");
    dump_choice(&tg);
    pick(&tg, 1);
    drain_pick0(&tg, 8);
    dump_board(&tg);
    CHECK_EQ(total_score_bonus(&tg), 1, "picking the LIVE card -> live total +1");
    CHECK(revealed_has(&tg, live), "the live card is the one revealed");
    CHECK_EQ(tg.state.p[0].hand.n, 3, "all 3 cards remain in hand");
}

static void q176_choice_path_pick_non_live_from_mixed_hand(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int energy = test_id(&tg, ENERGY);
    int live = test_id(&tg, MUS_LIVE);
    test_add_to_hand(&tg, energy);
    test_add_to_hand(&tg, live);
    test_add_to_hand(&tg, test_new_id(&tg, ENERGY));

    test_activate_ability(&tg, umi);
    pick(&tg, 0);
    drain_pick0(&tg, 8);
    /* CONTROL first: the reveal must really have happened. */
    CHECK(revealed_has(&tg, energy), "B CONTROL: the energy card really was revealed");
    CHECK_EQ(total_score_bonus(&tg), 0, "picking a non-live card -> no bonus");
    CHECK(revealed_has(&tg, energy), "the non-live card is the one revealed");
}

static void q176_picked_card_tracked_in_revealed_cards(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_q176_board(&tg, &umi)) return;
    int card_a = test_id(&tg, MUS_LIVE);
    int card_b = test_id(&tg, ENERGY);
    test_add_to_hand(&tg, card_a);
    test_add_to_hand(&tg, card_b);

    test_activate_ability(&tg, umi);
    pick(&tg, 1);
    drain_pick0(&tg, 8);
    CHECK(revealed_has(&tg, card_b), "the picked card_b is in revealed_cards");
    CHECK(!revealed_has(&tg, card_a),
          "the unpicked card_a is NOT in revealed_cards");
}

/* =======================================================================
 * C. opponent_hand_blind_reveal_no_live_card_draw_test.rs (5 tests)
 *    PL!-PR-014-PR 園田海未: 登場, 「相手の手札を、自分は見ないで3枚選び公開する」.
 *    YOU pick from the OPPONENT's hand; no live card among the 3 -> draw 1.
 * ======================================================================= */

static int umi_opp_board(TestGame *tg, int n_opp, int *umi_out, int live_in_opp_hand)
{
    test_game_new(tg);
    clear_p1(tg);
    int filler = test_id(tg, FILLER);
    int umi = test_id(tg, UMI_OPP);
    pin_id(umi, UMI_OPP);
    if (umi < 0) return 0;
    test_give_energy(tg, 3);
    tg->state.p[0].hand.n = 0;
    test_add_to_hand(tg, umi);
    RbPlayer *P1 = &tg->state.p[0];
    P1->deck.n = 0;
    for (int i = 0; i < 10 && P1->deck.n < RB_MAX_ZONE; i++)
        P1->deck.cards[P1->deck.n++] = filler;
    RbPlayer *P2 = &tg->state.p[1];
    P2->hand.n = 0;
    for (int i = 0; i < n_opp; i++)
        test_add_to_hand_for(tg, 1, (i == live_in_opp_hand) ? test_id(tg, MUS_LIVE)
                                                            : test_new_id(tg, FILLER));
    P1->stage[0] = RB_EMPTY_SLOT; P1->stage[1] = RB_EMPTY_SLOT; P1->stage[2] = RB_EMPTY_SLOT;
    *umi_out = umi;
    return 1;
}

static void umi_pr014_appear_creates_blind_reveal_choice(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 5, &umi, -1)) return;
    play_to_stage(&tg, umi, 0, 0);
    CHECK(pending(&tg), "the 登場 creates a reveal choice");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (!ch) { CHECK(0, "C: a pending choice is readable"); return; }
    DIAG("choice: type=%s zone=%s count=%d blind=%d is_reveal=%d target_player_id=%s seat=%d",
         pending_kind(&tg), ch->zone, ch->count, ch->blind, ch->is_reveal,
         ch->target_player_id, ch->actor);
    CHECK_EQ(ch->count, 3, "3 cards are picked");
    CHECK(ch->blind, "the pick is blind — you cannot see the opponent's cards");
    CHECK(ch->is_reveal, "the pick is a reveal");
    CHECK(ch->zone && !strcmp(ch->zone, "hand"),
          "the zone is hand (the OPPONENT's hand)");
    EXPECTED_HARNESS_GAP(ch->target_player_id && !strcmp(ch->target_player_id, "opponent"),
        "RbChoice.target_player_id spells \"opponent\" (Rust) as the seat id \"p2\" (C)");
    CHECK(ch->target_player_id && !strcmp(ch->target_player_id, "p2"),
          "target_player_id names p2 — the OPPONENT's hand is the one revealed");
    CHECK_EQ(pending_seat(&tg), 0,
             "picker=self -> the choice is routed to p1 (the reveal is YOURS)");
}

static void umi_pr014_no_live_card_draws_one(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 3, &umi, -1)) return;
    play_to_stage(&tg, umi, 0, 0);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "1 card is drawn (the played member is gone)");
    CHECK_EQ(tg.state.n_revealed, 3, "3 cards are revealed");
    CHECK(!pending(&tg), "a 3-of-3 pick needs no further prompt");
}

static void umi_pr014_live_card_present_no_draw(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 3, &umi, 1)) return;
    int live = test_id(&tg, MUS_LIVE);
    play_to_stage(&tg, umi, 0, 0);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "no card is drawn when a live card is revealed");
    /* NON-VACUITY CONTROL: hand == 0 is also what a no-op 登場 produces, so the
     * reveal itself must be shown to have happened. */
    CHECK_EQ(tg.state.n_revealed, 3, "C CONTROL: 3 cards really were revealed");
    CHECK(revealed_has(&tg, live), "C CONTROL: the live card is among them");
}

static void umi_pr014_sequential_reprompt_preserves_blind_and_target(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 5, &umi, -1)) return;
    play_to_stage(&tg, umi, 0, 0);
    CHECK(pending(&tg), "C: a 5-card opponent hand forces a choice");

    pick(&tg, 0);
    CHECK(pending(&tg), "the prompt re-opens after the first pick");
    const RbChoice *r1 = rb_get_pending_choice(&tg.state);
    if (!r1) { CHECK(0, "C: the re-prompt is readable"); return; }
    DIAG("re-prompt: type=%s count=%d blind=%d is_reveal=%d target_player_id=%s",
         pending_kind(&tg), r1->count, r1->blind, r1->is_reveal, r1->target_player_id);
    CHECK_EQ(r1->count, 2, "the re-prompt asks for 2 more");
    CHECK(r1->blind, "the re-prompt preserves blind=true");
    CHECK(r1->is_reveal, "the re-prompt preserves is_reveal=true");
    CHECK(r1->target_player_id && !strcmp(r1->target_player_id, "opponent"),
          "the re-prompt preserves target=opponent");

    pick(&tg, 1);
    CHECK(pending(&tg), "the prompt re-opens for the last card");
    pick(&tg, 2);
    CHECK(!pending(&tg), "the reveal resolves after 3 picks");
}

static void umi_pr014_opponent_hand_not_consumed(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 3, &umi, -1)) return;
    play_to_stage(&tg, umi, 0, 0);
    CHECK_EQ(tg.state.p[1].hand.n, 3,
             "the OPPONENT's hand is not consumed by the blind reveal");
}

static void umi_pr014_you_control_the_pick(void)
{
    static TestGame tg;
    int umi = 0;
    if (!umi_opp_board(&tg, 5, &umi, -1)) return;
    play_to_stage(&tg, umi, 0, 0);
    pick(&tg, 0); pick(&tg, 1); pick(&tg, 2);
    CHECK_EQ(tg.state.n_revealed, 3, "3 cards are revealed by P1's own choices");
    CHECK(!pending(&tg), "the selection completes — YOU chose all 3");
}

/* =======================================================================
 * D. under_card_and_success_live_conditions_test.rs (5 tests)
 * ======================================================================= */

static void ranju_under_plus_one_places_correctly(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int ranju = test_id(&tg, RANJU);
    int energy = test_id(&tg, ENERGY);
    int filler = test_id(&tg, FILLER);
    pin_id(ranju, RANJU);
    if (ranju < 0) return;

    tg.state.p[0].stage[1] = ranju;
    tg.state.p[0].under_cards[1].cards[0] = energy;
    tg.state.p[0].under_cards[1].cards[1] = energy;
    tg.state.p[0].under_cards[1].n = 2;
    for (int i = 0; i < 11; i++) test_add_to_energy_deck(&tg, 0, energy);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 5);

    /* The Rust twin asserts ONLY its own setup and says so; the live-success
     * placement itself is not driven. Both asserts are ported verbatim, and the
     * printed rule is recorded as a diagnostic so the day the placement is
     * driven this test is visibly not checking it yet. */
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 2, "D: two energy cards sit under her");
    CHECK(tg.state.p[0].energy_deck.n >= 1, "D: the energy deck is stocked");
    DIAG("ranju live-success would place underCount+1 = 3 energy; not driven here "
         "(the Rust twin explicitly does not drive it either)");
}

static void keke_under_cost_per_unit(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_2);
    int liella_under = test_id(&tg, KANON_SUPER);
    pin_id(keke, KEKE_2);
    pin_id(liella_under, KANON_SUPER);
    /* cards.json: 『Liella!』 is the SERIES-level group for
     * ラブライブ！スーパースター!!, so a CatChu! print still satisfies it. */
    CHECK(rb_card_matches_group_str(liella_under, "Liella!"),
          "D: the under-card really is a 『Liella!』 group card");
    if (keke < 0 || liella_under < 0) return;

    tg.state.p[0].stage[0] = keke;
    test_place_under(&tg, 0, 0, liella_under);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, keke), 1, "1 『Liella!』 member under her -> cost +1");

    int second = test_new_id(&tg, KANON_SUPER);
    pin_id(second, KANON_SUPER);
    CHECK(rb_card_matches_group_str(second, "Liella!"),
          "D CONTROL: the SECOND 『Liella!』 copy also resolves its group");
    test_place_under(&tg, 0, 0, second);
    test_recalc(&tg);
    DIAG("under[0] n=%d cost_mod=%d", tg.state.p[0].under_cards[0].n, cost_mod(&tg, keke));
    CHECK_EQ(cost_mod(&tg, keke), 2, "2 under her -> cost +2 (per card, not per area)");

    int non_liella = test_id(&tg, NIJI_MEM_FODDER);
    pin_id(non_liella, NIJI_MEM_FODDER);
    CHECK(!rb_card_matches_group_str(non_liella, "Liella!"),
          "D: the control card is NOT a 『Liella!』 group card");
    test_place_under(&tg, 0, 0, non_liella);
    test_recalc(&tg);
    CHECK_EQ(cost_mod(&tg, keke), 2, "a non-『Liella!』 card under her adds nothing");
    CHECK_EQ(tg.state.p[0].under_cards[0].n, 3,
             "setup guard: three cards really are under her");
}

static void umi_conditional_alternative_0_1_2(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int umi = test_id(&tg, NAGISA);
    int filler = test_id(&tg, FILLER);
    pin_id(umi, NAGISA);
    if (umi < 0) return;
    fill_decks(&tg, filler, 30);
    tg.state.p[0].success.n = 0;
    tg.state.p[0].stage[1] = umi;
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].stage[1], umi, "D: she is on stage with an empty success zone");

    int scoring = test_id(&tg, "PL!N-bp1-025-L");
    pin_id(scoring, "PL!N-bp1-025-L");
    CHECK(printed_score(scoring) > 0, "D: the +1 fixture really prints a score icon");
    add_to_success(&tg, scoring);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].success.n, 1, "D: one scoring card in the success zone");

    int scoring2 = test_new_id(&tg, "PL!N-bp1-025-L");
    add_to_success(&tg, scoring2);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].success.n, 2, "D: two scoring cards in the success zone");
    /* The Rust twin never asserts the +0/+1/+2 bonus it names; this port keeps
     * the same tautology visible rather than silently replacing it. */
    DIAG("Rust umi_conditional_alternative_0_1_2 asserts only its own setup; the "
         "+0/+1/+2 live-total branch is pinned by nagisa_*_cards_gets_plus_* above");
}

static void kaleidoscore_optional_branches(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_DEBUT);
    int blade_card = test_id(&tg, BLADE_CARD);
    int energy = test_id(&tg, ENERGY);
    int filler = test_id(&tg, FILLER);
    pin_id(keke, KEKE_DEBUT);
    pin_id(blade_card, BLADE_CARD);
    CHECK(has_blade_heart(blade_card),
          "D: the blade-bearing branch needs a card that HAS a blade heart");
    /* RUST FIXTURE BUG, pinned. The prompt is
     * 「手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい」 — a GROUP-filtered
     * pool. cards.json gives PL!SP-bp1-004-P (平安名すみれ) unit = "CatChu!",
     * NOT "KALEIDOSCORE", so the Rust twin's own discard target can never be
     * picked and the "KNOWN GAP" it documents is really an illegal fixture. The
     * same step works end to end with a legal KALEIDOSCORE print
     * (PL!SP-sd1-010-SD) in section I. */
    CHECK(!rb_card_matches_group_str(blade_card, "KALEIDOSCORE"),
          "D FIXTURE BUG: the Rust twin's discard target is NOT a 『KALEIDOSCORE』 card");
    CHECK(rb_card_matches_group_str(test_id(&tg, KEKE_FODDER_BH), "KALEIDOSCORE"),
          "D: PL!SP-sd1-010-SD (unit KALEIDOSCORE) IS a legal target for that step");
    if (keke < 0 || blade_card < 0) return;
    test_add_to_energy_deck(&tg, 0, energy);
    fill_decks(&tg, filler, 30);
    test_add_to_hand(&tg, keke);
    test_add_to_hand(&tg, blade_card);
    test_give_energy(&tg, 10);

    int energy_zone_before = tg.state.p[0].energy.n;
    int energy_deck_before = tg.state.p[0].energy_deck.n;

    play_to_stage(&tg, keke, 0, 0);
    /* Rust: g.drain_choices_strict(&["SelectTarget", "SelectCard",
     * "SelectAutoAbility"], &[0]) — it ANSWERS the SelectCard prompt with
     * index 0, i.e. it ACCEPTS the optional discard. The Rust twin's own drain
     * is therefore inconsistent with the assertions that follow it (which
     * require the discard NOT to have happened). The drain is ported faithfully
     * and the assertions are left strict, so the disagreement is visible. */
    int guard = 0;
    while (pending(&tg) && guard++ < 8) {
        const char *t = pending_kind(&tg);
        if (t && (!strcmp(t, "SelectAutoAbility") || !strcmp(t, "SelectTarget") ||
                  !strcmp(t, "SelectCard")))
            pick(&tg, 0);
        else
            skip_pick(&tg);
    }
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].stage[0], keke, "D: 唐 可可 is on the stage");
    /* The Rust twin's KNOWN GAP pins, asserted strictly. If the prompt really
     * never appeared these would pass — the dump above is what shows whether it
     * did. */
    CHECK(test_hand_has(&tg, blade_card),
          "KNOWN GAP: 手札の『KALEIDOSCORE』のカードを1枚控え室に置いてもよい does not "
          "offer its prompt, so the card is still in hand");
    CHECK_EQ(tg.state.p[0].energy.n, energy_zone_before,
             "KNOWN GAP: the エネルギーカードを1枚ウェイト状態で置く step is gated on "
             "that discard, so no energy joined the zone");
    CHECK(tg.state.p[0].energy_deck.n <= energy_deck_before,
          "KNOWN GAP: nothing left the energy deck either, so the whole step is skipped");
}

static void genki_invalidate_isolated_to_self(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int genki = test_id(&tg, GENKI);
    int other_live = test_id(&tg, OTHER_LIVE_S);
    int aqours_a = test_id(&tg, AQOURS_A);
    int filler = test_id(&tg, FILLER);
    pin_id(genki, GENKI);
    pin_id(other_live, OTHER_LIVE_S);
    pin_id(aqours_a, AQOURS_A);
    if (genki < 0 || other_live < 0 || aqours_a < 0) return;
    int aqours_b = test_new_id(&tg, AQOURS_A);
    CHECK(aqours_a != aqours_b, "D: two separate 『Aqours』 instances");

    tg.state.p[0].stage[0] = aqours_a;
    tg.state.p[0].stage[1] = aqours_b;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    for (int cid = 0; cid < 2; cid++) {
        int c = cid ? aqours_b : aqours_a;
        /* heart02 is colour index 1 in the C heart enum; test_get_heart_modifier
         * remaps colour 5 onto RB_HEART_ORANGE, so the mod store is written
         * directly to avoid the remap. */
        rb_mods_add_heart(&tg.state.mods, c, HEART02_IDX, 3);
    }
    test_add_to_hand(&tg, genki);
    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 10);

    int arrived = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK(arrived, "D: the turn reaches the live-card-set phase");
    test_set_live_card(&tg, 0, genki);
    arrived = test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE);
    CHECK(arrived, "D: the turn reaches the performance phase");

    /* The invalidation must be created BY THE ENGINE, so the live performance
     * has to run. rb_perform_live is what fires ライブ開始時. */
    rb_perform_live(&tg.state, 0);
    drain_auto(&tg);

    /* NON-VACUITY CONTROL: the printed gate is 「自分のステージにいる『Aqours』の
     * メンバーが持つハートに、heart02が合計6個以上ある場合」, so prove the gate's
     * own input really reached 6 before reading the invalidation. */
    CHECK(tg.state.p[0].stage[0] == aqours_a && tg.state.p[0].stage[1] == aqours_b,
          "D CONTROL: both 『Aqours』 members really reached the stage");
    DIAG("heart02 on a=%d on b=%d (gate needs a total of 6)",
         rb_mods_get_heart(&tg.state.mods, aqours_a, HEART02_IDX),
         rb_mods_get_heart(&tg.state.mods, aqours_b, HEART02_IDX));
    CHECK(rb_mods_get_heart(&tg.state.mods, aqours_a, HEART02_IDX) >= 3 &&
          rb_mods_get_heart(&tg.state.mods, aqours_b, HEART02_IDX) >= 3,
          "D CONTROL: the 合計6 heart02 the printed gate needs is really present");
    DIAG("invalidation store: n=%d", tg.state.n_ability_invalidations);
    CHECK(rb_ability_is_invalidated(&tg.state, genki, RB_TSTR_LIVE_SUCCESS),
          "元気全開's ライブ開始時 invalidates its OWN ライブ成功時");
    CHECK(!rb_ability_is_invalidated(&tg.state, other_live, RB_TSTR_LIVE_SUCCESS),
          "君のこころは輝いてるかい？ has its own ライブ成功時 and must not be invalidated");
    CHECK(!rb_ability_is_invalidated(&tg.state, genki, RB_TSTR_LIVE_START),
          "only ライブ成功時 was invalidated — ライブ開始時 is a separate trigger");
    CHECK(!rb_ability_is_invalidated(&tg.state, other_live, RB_TSTR_LIVE_START),
          "the other live's ライブ開始時 is untouched too");
}

/* =======================================================================
 * E. live_phase_success_zone_replacement_q256_test.rs (7 tests)
 *    CROSSROADS 「このカードを成功ライブカード置き場に置く場合、代わりに自分の
 *    控え室にある『μ's』のライブカードを1枚置いてもよい。」 driven by the LIVE
 *    PHASE (TurnEngine::move_live_to_success_and_handle_wins).
 *    test_move_cards.c ports the move_cards (debut) arm only; test_live_success_rules.c
 *    ports the p1 accept/decline arms with a NON-μ's candidate. The p2 routing,
 *    the multi-target filter, the Dreamin' identity and the no-valid-target arm
 *    are ported here.
 * ======================================================================= */

static void live_phase_success_zone_replacement_routes_choice_to_player_one(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int muse = test_id(&tg, BOKURA);
    pin_id(crossroads, CROSSROADS);
    pin_id(muse, BOKURA);
    CHECK(rb_card_matches_group_str(muse, MU_GROUP),
          "E: the replacement really is a 『μ's』 live card");
    if (crossroads < 0 || muse < 0) return;

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, muse);
    tg.state.p1_live_won = 1;   /* Rust: move_live_to_success_and_handle_wins(true, false) */

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "the live phase opens a replacement prompt");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    DIAG("choice: type=%s zone=%s count=%d allow_skip=%d nfiltered=%d actor=%d",
         pending_kind(&tg), ch ? ch->zone : "-", ch ? ch->count : -1,
         ch ? ch->allow_skip : -1, ch ? ch->n_filtered_indices : -1,
         pending_seat(&tg));
    CHECK_EQ(pending_seat(&tg), 0, "the p1 winner is the one prompted");
    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 0, "discard", crossroads),
          "CROSSROADS is in the waitroom after the replacement");
    CHECK(test_zone_has_id(&tg, 0, "success", muse),
          "the μ's live card is in the success zone");
    CHECK_EQ(tg.state.p[0].live.n, 0, "the live zone is empty after processing");
}

static void live_phase_declined_replacement_places_original_in_success_zone(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int muse = test_id(&tg, BOKURA);
    pin_id(crossroads, CROSSROADS);
    if (crossroads < 0 || muse < 0) return;

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, muse);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "E: the replacement prompt is open");
    skip_pick(&tg);
    CHECK(test_zone_has_id(&tg, 0, "success", crossroads),
          "declining places CROSSROADS itself in the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", muse),
          "the μ's live card stays in the waitroom");
    CHECK_EQ(tg.state.p[0].live.n, 0, "E: the live zone is empty");
}

static void live_phase_success_zone_replacement_routes_choice_to_player_two(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int muse = test_id(&tg, BOKURA);
    pin_id(crossroads, CROSSROADS);
    if (crossroads < 0 || muse < 0) return;

    test_add_to_live_for(&tg, 1, crossroads);
    rb_waitroom_add(&tg.state.p[1], muse);
    tg.state.p2_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "E: the p2 winner is prompted too");
    CHECK_EQ(pending_seat(&tg), 1, "the p2 winner is the one prompted");
    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 1, "discard", crossroads),
          "p2's CROSSROADS is in p2's waitroom");
    CHECK(test_zone_has_id(&tg, 1, "success", muse),
          "p2's μ's live card is in p2's success zone");
}

static void live_phase_replacement_preserves_target_identity_dreamin(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int dreamin = test_id(&tg, DREAMIN);
    pin_id(crossroads, CROSSROADS);
    pin_id(dreamin, DREAMIN);
    if (crossroads < 0 || dreamin < 0) return;

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, dreamin);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "E: the replacement prompt is open");
    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 0, "discard", crossroads),
          "CROSSROADS is in the waitroom");
    CHECK(test_zone_has_id(&tg, 0, "success", dreamin),
          "Dreamin' Go! Go!! is in the success zone");
}

static void live_phase_replacement_filters_multiple_waitroom_targets(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int dreamin = test_id(&tg, DREAMIN);
    int bokura = test_id(&tg, BOKURA);
    int filler = test_id(&tg, WEWILL);
    pin_id(crossroads, CROSSROADS);
    pin_id(dreamin, DREAMIN);
    pin_id(bokura, BOKURA);
    pin_id(filler, WEWILL);
    CHECK(!rb_card_matches_group_str(filler, MU_GROUP),
          "E: WE WILL!! is a live card that is NOT μ's");
    if (crossroads < 0 || dreamin < 0 || bokura < 0 || filler < 0) return;

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, dreamin);
    test_add_to_discard(&tg, bokura);
    test_add_to_discard(&tg, filler);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(pending(&tg), "E: the replacement prompt is open");
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (!ch) { CHECK(0, "E: the prompt is readable"); return; }
    dump_choice(&tg);
    for (int i = 0; i < ch->n_filtered_indices; i++) {
        int widx = ch->filtered_indices[i];
        const char *no = widx >= 0 && widx < tg.state.p[0].discard.n
                       ? card_no_of(tg.state.p[0].discard.cards[widx]) : "?";
        DIAG("  [%d] waitroom idx %d -> %s", i, widx, no);
    }
    CHECK_EQ(ch->n_filtered_indices, 2, "only the TWO μ's waitroom lives are offered");
    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 0, "success", dreamin),
          "the first μ's target reaches the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", filler),
          "the non-μ's live card was never offered and stays in the waitroom");
    CHECK_EQ(tg.state.p[0].live.n, 0, "E: the live zone is empty");
}

static void debut_reveal_success_zone_replacement_preserves_target_identity_dreamin(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int maki = test_id(&tg, MAKI);
    int crossroads = test_id(&tg, CROSSROADS);
    int dreamin = test_id(&tg, DREAMIN);
    int filler_live = test_new_id(&tg, WEWILL);
    pin_id(maki, MAKI);
    pin_id(crossroads, CROSSROADS);
    pin_id(dreamin, DREAMIN);
    if (maki < 0 || crossroads < 0 || dreamin < 0) return;

    test_add_to_hand(&tg, maki);
    test_add_to_hand(&tg, crossroads);
    test_add_to_discard(&tg, dreamin);
    add_to_success(&tg, filler_live);
    test_add_to_hand(&tg, filler_live);
    fill_decks(&tg, test_id(&tg, FILLER), 30);
    test_give_energy(&tg, 9);

    play_to_stage(&tg, maki, 1, 0);

    int cost_handled = 0, replacement_handled = 0, extra = 0;
    int guard = 0;
    while (pending(&tg) && guard++ < 8) {
        dump_choice(&tg);
        if (!cost_handled) { pick(&tg, 0); cost_handled = 1; }
        else if (!replacement_handled) { pick(&tg, 0); replacement_handled = 1; }
        else { extra++; break; }
    }
    dump_board(&tg);
    CHECK(cost_handled, "E: the optional reveal-cost prompt appeared");
    CHECK(replacement_handled, "E: the CROSSROADS replacement prompt appeared");
    CHECK_EQ(extra, 0, "E: no unexpected extra prompt");
    CHECK(test_zone_has_id(&tg, 0, "discard", crossroads),
          "CROSSROADS is in the waitroom (replaced)");
    CHECK(test_zone_has_id(&tg, 0, "success", dreamin),
          "Dreamin' Go! Go!! is in the success zone");
    CHECK(test_hand_has(&tg, filler_live),
          "the original success-zone card is returned to hand");
}

static void live_phase_no_valid_replacement_places_original_in_success_zone(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int crossroads = test_id(&tg, CROSSROADS);
    int filler = test_new_id(&tg, WEWILL);
    pin_id(crossroads, CROSSROADS);
    CHECK(!rb_card_matches_group_str(filler, MU_GROUP),
          "E: the only waitroom live card is NOT μ's, so no target is legal");
    if (crossroads < 0) return;

    test_add_to_live(&tg, crossroads);
    test_add_to_discard(&tg, filler);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    CHECK(!pending(&tg), "no prompt when the waitroom holds no μ's live");
    CHECK(test_zone_has_id(&tg, 0, "success", crossroads),
          "CROSSROADS is placed in the success zone directly");
}

/* =======================================================================
 * F. debut_reveal_success_zone_replacement_q256_test.rs (10 tests)
 *    PL!-sd1-006-SD 西木野真姫 登場: 「手札のライブカードを1枚公開してもよい：
 *    自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、
 *    これにより公開したカードを自分の成功ライブカード置き場に置く。」
 * ======================================================================= */

typedef struct { int maki, crossroads; } MakiBoard;

static int setup_maki_reveal(TestGame *tg, const int *waitroom, int n_waitroom,
                             const int *success, int n_success, MakiBoard *out)
{
    test_game_new(tg);
    clear_p1(tg);
    int maki = test_id(tg, MAKI);
    int crossroads = test_id(tg, CROSSROADS);
    pin_id(maki, MAKI);
    pin_id(crossroads, CROSSROADS);
    if (maki < 0 || crossroads < 0) return 0;
    test_add_to_hand(tg, maki);
    test_add_to_hand(tg, crossroads);
    for (int i = 0; i < n_waitroom; i++) rb_waitroom_add(&tg->state.p[0], waitroom[i]);
    for (int i = 0; i < n_success; i++) add_to_success(tg, success[i]);
    fill_decks(tg, test_id(tg, FILLER), 30);
    test_give_energy(tg, 9);
    out->maki = maki;
    out->crossroads = crossroads;
    return 1;
}

/* The optional 「手札のライブカードを1枚公開してもよい」 prompt, with the field
 * assertions the Rust twin makes before it answers. */
static int select_maki_reveal(TestGame *tg, int card_id)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (!ch) {
        CHECK(0, "F: the optional reveal prompt is open");
        return 0;
    }
    dump_choice(tg);
    CHECK(ch->zone && !strcmp(ch->zone, "hand"), "F: the reveal pool is hand");
    /* The printed text is 「手札のライブカードを1枚公開してもよい」 — the pool is
     * restricted to LIVE cards. Rust asserts card_type == Some("live_card"). */
    CHECK(ch->card_type && !strcmp(ch->card_type, "live_card"),
          "F: the reveal is filtered to live cards");
    if (!ch->card_type[0]) {
        /* Evidence for the red above. The engine's PARSE does carry the filter
         * (reveal src=hand type=live_card), so the prompt's pool really is
         * unfiltered and every hand card is offered, live or not. */
        for (int i = 0; i < tg->state.p[0].hand.n; i++) {
            int c = tg->state.p[0].hand.cards[i];
            DIAG("UNFILTERED: hand[%d]=%-20s %s (n_filtered=%d)", i, card_no_of(c),
                 rb_card_is_live(c) ? "LIVE   " : "NON-LIVE", ch->n_filtered_indices);
        }
        dump_ability(386, 0);   /* the Maki host, id pinned by pin_id above */
    }
    CHECK_EQ(ch->count, 1, "F: exactly 1 card is revealed");
    CHECK(ch->allow_skip, "F: the reveal is optional (allow_skip)");
    /* Answer with the PHYSICAL hand index of the requested card. */
    int idx = -1;
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == card_id) { idx = i; break; }
    CHECK(idx >= 0, "F: the requested live card is in hand when the prompt is answered");
    if (idx < 0) return 0;
    pick(tg, idx);
    return 1;
}

static void assert_crossroads_replacement_choice(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (!ch) { CHECK(0, "F: the CROSSROADS replacement prompt is open"); return; }
    dump_choice(tg);
    CHECK(ch->zone && !strcmp(ch->zone, "discard"),
          "F: the replacement pool is the WAITROOM (RbChoice.zone spells it \"discard\")");
    CHECK(ch->card_type && !strcmp(ch->card_type, "live_card"),
          "F: the replacement is filtered to live cards");
    CHECK_EQ(ch->count, 1, "F: exactly 1 replacement");
    CHECK(ch->allow_skip, "F: the replacement is optional");
    CHECK(ch->filter_group && !strcmp(ch->filter_group, MU_GROUP),
          "F: the replacement is group-filtered to 『μ's』");
}

static int zone_occurrences(const TestGame *tg, int card_id)
{
    const GameState *g = &tg->state;
    int n = 0;
    for (int i = 0; i < g->p[0].hand.n; i++)     if (g->p[0].hand.cards[i] == card_id) n++;
    for (int i = 0; i < g->p[0].discard.n; i++)   if (g->p[0].discard.cards[i] == card_id) n++;
    for (int i = 0; i < g->p[0].live.n; i++)      if (g->p[0].live.cards[i] == card_id) n++;
    for (int i = 0; i < g->p[0].success.n; i++)   if (g->p[0].success.cards[i] == card_id) n++;
    for (int i = 0; i < RB_STAGE_SIZE; i++)      if (g->p[0].stage[i] == card_id) n++;
    for (int i = 0; i < g->n_revealed; i++)       if (g->revealed_cards[i] == card_id) n++;
    return n;
}

static void maki_q256_accepts_crossroads_replacement(void)
{
    static TestGame tg;
    MakiBoard b;
    int muse = 0, existing = 0;
    test_game_new(&tg);           /* resolve fixtures before the real setup */
    muse = test_id(&tg, BOKURA);
    existing = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(muse, BOKURA);

    rb_waitroom_add(&tg.state.p[0], muse);
    add_to_success(&tg, existing);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;
    assert_crossroads_replacement_choice(&tg);
    pick(&tg, 0);
    dump_board(&tg);

    CHECK(test_hand_has(&tg, existing),
          "F: the previous success-zone card is returned to hand");
    CHECK(!test_zone_has_id(&tg, 0, "success", existing),
          "F: the previous success-zone card left the success zone");
    CHECK(test_zone_has_id(&tg, 0, "success", muse), "F: μ's live is in the success zone");
    dump_board(&tg);
    CHECK(test_zone_has_id(&tg, 0, "discard", b.crossroads), "F: CROSSROADS is in the waitroom");
    CHECK_EQ(zone_occurrences(&tg, b.maki), 1, "F: 真姫 exists exactly once");
    CHECK_EQ(zone_occurrences(&tg, muse), 1, "F: the μ's live exists exactly once");
    CHECK_EQ(zone_occurrences(&tg, b.crossroads), 1, "F: CROSSROADS exists exactly once");
    CHECK_EQ(zone_occurrences(&tg, existing), 1, "F: the previous live exists exactly once");
}

static void maki_q256_declines_crossroads_replacement(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int muse = test_id(&tg, BOKURA);
    int existing = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(muse, BOKURA);
    rb_waitroom_add(&tg.state.p[0], muse);
    add_to_success(&tg, existing);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;
    CHECK(!test_hand_has(&tg, b.crossroads),
          "F: the revealed CROSSROADS already left hand");
    assert_crossroads_replacement_choice(&tg);
    skip_pick(&tg);

    CHECK(test_hand_has(&tg, existing), "F: the previous success card is in hand");
    CHECK(test_zone_has_id(&tg, 0, "success", b.crossroads),
          "F: declining puts CROSSROADS itself in the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", muse),
          "F: the μ's live card stays in the waitroom");
    CHECK(!pending(&tg), "F: the replacement prompt is resolved");
}

static void maki_q256_can_decline_reveal(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int muse = test_id(&tg, BOKURA);
    int existing = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    rb_waitroom_add(&tg.state.p[0], muse);
    add_to_success(&tg, existing);

    play_to_stage(&tg, b.maki, 1, 0);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "F: the optional reveal prompt is open");
    if (ch) {
        CHECK(ch->zone && !strcmp(ch->zone, "hand"), "F: the reveal pool is hand");
        CHECK(ch->allow_skip, "F: the reveal may be declined");
    }
    skip_pick(&tg);

    CHECK(test_hand_has(&tg, b.crossroads), "F: CROSSROADS stays in hand");
    CHECK(test_zone_has_id(&tg, 0, "success", existing),
          "F: the success zone is untouched when the reveal is declined");
    CHECK(test_zone_has_id(&tg, 0, "discard", muse), "F: the μ's live stays in the waitroom");
    CHECK(!pending(&tg), "F: no prompt remains");
}

static void maki_q256_without_live_card_in_hand_does_nothing(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int existing = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    /* Retain-everything-but-CROSSROADS, mirroring the Rust retain(). */
    RbHand *H = &tg.state.p[0].hand;
    int w = 0;
    for (int i = 0; i < H->n; i++) if (H->cards[i] != b.crossroads) H->cards[w++] = H->cards[i];
    H->n = w;
    add_to_success(&tg, existing);

    play_to_stage(&tg, b.maki, 1, 0);
    CHECK(!pending(&tg), "F: no live card in hand -> the 登場 does nothing");
    CHECK(test_zone_has_id(&tg, 0, "success", existing),
          "F: the success zone is untouched");
}

static void maki_q256_empty_success_still_attempts_placement(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int muse = test_id(&tg, BOKURA);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(muse, BOKURA);
    rb_waitroom_add(&tg.state.p[0], muse);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;
    CHECK(!test_hand_has(&tg, b.crossroads), "F: the revealed card left hand");
    assert_crossroads_replacement_choice(&tg);
    pick(&tg, 0);
    CHECK(test_zone_has_id(&tg, 0, "success", muse), "F: μ's live is in the success zone");
    dump_board(&tg);
    CHECK(test_zone_has_id(&tg, 0, "discard", b.crossroads), "F: CROSSROADS is in the waitroom");
}

static void maki_q256_selects_exact_success_card(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int selected = test_new_id(&tg, WEWILL);
    int remaining = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    add_to_success(&tg, selected);
    add_to_success(&tg, remaining);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (!ch) { CHECK(0, "F: the success-zone selection prompt is open"); return; }
    dump_choice(&tg);
    CHECK(ch->zone && !strcmp(ch->zone, "success_live_zone"),
          "F: the pick pool is the success live card zone");
    CHECK_EQ(ch->count, 1, "F: exactly 1 success-zone card is taken");
    CHECK(!ch->allow_skip, "F: taking a success-zone card is mandatory");
    CHECK_EQ(ch->n_filtered_indices, 2, "F: both success-zone cards are offered");
    pick(&tg, 0);
    dump_board(&tg);
    CHECK(test_hand_has(&tg, selected), "F: the selected card is in hand");
    CHECK(test_zone_has_id(&tg, 0, "success", remaining),
          "F: the unselected card stays in the success zone");
    CHECK(test_zone_has_id(&tg, 0, "success", b.crossroads),
          "F: CROSSROADS joins the success zone");
    CHECK(!pending(&tg), "F: no prompt remains");
}

static void maki_q256_non_crossroads_reveal_bypasses_replacement(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int ordinary = test_id(&tg, HANABI);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(ordinary, HANABI);
    RbHand *H = &tg.state.p[0].hand;
    int w = 0;
    for (int i = 0; i < H->n; i++) if (H->cards[i] != b.crossroads) H->cards[w++] = H->cards[i];
    H->n = w;
    test_add_to_hand(&tg, ordinary);
    /* A NON-live card in hand too, so the reveal prompt's pool can be shown to
     * admit a card the printed 「手札のライブカードを1枚公開」 forbids. This is
     * the behaviour the empty RbChoice.card_type above costs. */
    int not_a_live = test_id(&tg, "PL!-sd1-001-SD");
    pin_id(not_a_live, "PL!-sd1-001-SD");
    CHECK(!rb_card_is_live(not_a_live), "F: the extra hand card is NOT a live card");
    test_add_to_hand(&tg, not_a_live);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, ordinary)) return;
    CHECK(test_zone_has_id(&tg, 0, "success", ordinary),
          "F: a NON-μ's live bypasses the replacement entirely");
    CHECK(!pending(&tg), "F: no replacement prompt for a non-μ's live");
}

static void maki_q256_non_mus_waitroom_does_not_prompt(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int non_mus = test_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(non_mus, WEWILL);
    rb_waitroom_add(&tg.state.p[0], non_mus);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;
    CHECK(!pending(&tg), "F: no μ's waitroom target -> no replacement prompt");
    CHECK(test_zone_has_id(&tg, 0, "success", b.crossroads),
          "F: CROSSROADS itself lands in the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", non_mus), "F: the non-μ's live is untouched");
}

static void maki_q256_filters_multiple_mus_targets(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int non_mus = test_new_id(&tg, WEWILL);
    int first_mus = test_id(&tg, BOKURA);
    int second_mus = test_id(&tg, DREAMIN);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(first_mus, BOKURA);
    pin_id(second_mus, DREAMIN);
    rb_waitroom_add(&tg.state.p[0], non_mus);
    rb_waitroom_add(&tg.state.p[0], first_mus);
    rb_waitroom_add(&tg.state.p[0], second_mus);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (!ch) { CHECK(0, "F: the filtered replacement prompt is open"); return; }
    DIAG("filtered waitroom: n=%d ->", ch->n_filtered_indices);
    for (int i = 0; i < ch->n_filtered_indices; i++) {
        int widx = ch->filtered_indices[i];
        DIAG("  [%d] waitroom idx %d -> %s", i, widx,
             card_no_of(tg.state.p[0].discard.cards[widx]));
    }
    CHECK_EQ(ch->n_filtered_indices, 2,
             "F: the non-μ's waitroom card is filtered out (physical indices 1,2)");
    /* Answer by the FILTERED index of the second μ's target, not its physical
     * waitroom index (Rust select_waitroom_card_filtered). */
    int fidx = -1;
    for (int i = 0; i < ch->n_filtered_indices; i++)
        if (tg.state.p[0].discard.cards[ch->filtered_indices[i]] == second_mus) fidx = i;
    CHECK(fidx >= 0, "F: the second μ's target is in the filtered list");
    if (fidx >= 0) pick(&tg, fidx);

    CHECK(test_zone_has_id(&tg, 0, "success", second_mus),
          "F: the chosen μ's live is in the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", first_mus),
          "F: the other μ's live stays in the waitroom");
    CHECK(test_zone_has_id(&tg, 0, "discard", non_mus), "F: the non-μ's live stays put");
    dump_board(&tg);
    CHECK(test_zone_has_id(&tg, 0, "discard", b.crossroads), "F: CROSSROADS is in the waitroom");
}

static void maki_q256_uses_only_the_cost_revealed_card(void)
{
    static TestGame tg;
    MakiBoard b;
    test_game_new(&tg);
    int muse = test_id(&tg, BOKURA);
    int unrelated = test_id(&tg, HAPPY_TRAIN);
    int existing = test_new_id(&tg, WEWILL);
    if (!setup_maki_reveal(&tg, NULL, 0, NULL, 0, &b)) return;
    pin_id(muse, BOKURA);
    pin_id(unrelated, HAPPY_TRAIN);
    rb_waitroom_add(&tg.state.p[0], muse);
    add_to_success(&tg, existing);
    test_add_to_revealed(&tg, unrelated);

    play_to_stage(&tg, b.maki, 1, 0);
    if (!select_maki_reveal(&tg, b.crossroads)) return;
    assert_crossroads_replacement_choice(&tg);
    pick(&tg, 0);

    CHECK(test_zone_has_id(&tg, 0, "success", muse), "F: μ's live is in the success zone");
    dump_board(&tg);
    CHECK(test_zone_has_id(&tg, 0, "discard", b.crossroads), "F: CROSSROADS is in the waitroom");
    CHECK(revealed_has(&tg, unrelated), "F: the unrelated reveal is still tracked");
    CHECK(!test_zone_has_id(&tg, 0, "success", unrelated),
          "F: the unrelated reveal was NOT used as the placement");
    CHECK(test_hand_has(&tg, existing), "F: the previous success card is in hand");
}

/* =======================================================================
 * G. discard_placed_group_live_or_blade_condition_test.rs (9 tests)
 *    PL!N-bp7-006-R＋ 近江彼方 ab#1 (the SECOND 起動): mill the top 3, then if
 *    the milled 3 hold a 『虹ヶ咲』 live OR a 『虹ヶ咲』 member without a blade
 *    heart, choose 1 of: activate 2 energy / gain blade x2.
 * ======================================================================= */

static int kanata_activate_ab1(TestGame *tg, int kanata)
{
    int rc = activate_ability_index(tg, kanata, 1);
    if (rc >= 0) rb_drain_ability_queue(&tg->state);
    return rc;
}

/* A: the Rust twin drives ONE specific 起動. rb_activate_card has no
 * ability-index parameter, so a plain activation would ALSO run ab#0 (E: look
 * at the top 4 and reorder them 「好きな順番」). */
static void kanata_setup_uses_the_ability_index_entry_point(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    int n = rb_card_num_abilities((uint32_t)kanata);
    CHECK(n >= 2, "G: 近江彼方 really prints two abilities (the target is ab#1)");
    int acts = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (rb_decode_card_ability((uint32_t)kanata, i, &ab)) {
            int is_act = ab.triggers && strstr(ab.triggers, TRIGGER_KIDOU);
            if (is_act) acts++;
            DIAG("ability %d triggers=%s use_limit=%d text=%s", i,
                 ab.triggers ? ab.triggers : "-", ab.use_limit,
                 ab.triggerless_text ? ab.triggerless_text : "-");
            ab.cost = NULL; ab.effect = NULL;
            rb_free_ability(&ab);
        }
    }
    CHECK_EQ(acts, 2, "G: both of them are 起動 abilities");
    /* The parse is printed so a red section-G assertion can be read against the
     * effect tree the engine actually holds. */
    dump_ability(kanata, 0);
    dump_ability(kanata, 1);
    EXPECTED_HARNESS_GAP(0,
        "rb_activate_card()/test_activate_ability() take no ability index, so "
        "近江彼方's ab#1 cannot be driven through the C activation path; "
        "rb_resolve_ability(g, actor, ab, ability_idx=1, host, &resolved) is the "
        "substitute used throughout section G");
}

static void kanata_cost_mills_top_three(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_id(&tg, NIJI_LIVE);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { live, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    int deck_before = tg.state.p[0].deck.n;
    int wait_before = tg.state.p[0].discard.n;
    dump_zone("deck BEFORE", &tg.state.p[0].deck);
    int rc = kanata_activate_ab1(&tg, kanata);
    CHECK(rc != 0, "G: ab#1 activates");

    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3,
             "the cost mills exactly the top 3 deck cards");
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "the 3 milled cards are in the waitroom");
    dump_zone("deck AFTER", &tg.state.p[0].deck);
    dump_zone("waitroom AFTER", &tg.state.p[0].discard);
    CHECK(test_zone_has_id(&tg, 0, "discard", live), "the 虹ヶ咲 live was discarded");
    CHECK(test_zone_has_id(&tg, 0, "discard", fill), "the non-虹ヶ咲 filler was discarded");
}

static void kanata_live_card_in_discard_offers_choice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_id(&tg, NIJI_LIVE);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { live, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    kanata_activate_ab1(&tg, kanata);
    /* NON-VACUITY CONTROL: "a choice is offered" is the whole assertion, so the
     * negative twin (kanata_no_matching_card_does_not_offer_choice) is the
     * control — and this card's cost really ran. */
    CHECK_EQ(tg.state.p[0].discard.n, 3, "G CONTROL: the cost really milled 3 cards");
    CHECK(pending(&tg), "a 虹ヶ咲 live card among the discarded 3 offers the choice");
}

static void kanata_member_without_blade_heart_offers_choice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int member = test_id(&tg, NIJI_MEM_NOBH);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    pin_id(member, NIJI_MEM_NOBH);
    /* cards.json: 上原歩夢 PL!N-bp1-001-R has blade=4 (blade COUNT) but NO
     * blade_heart key, so has_blade_heart() is false. */
    CHECK(!has_blade_heart(member),
          "G: 上原歩夢 really has NO blade heart (blade 4 != blade heart)");
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { member, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    kanata_activate_ab1(&tg, kanata);
    CHECK_EQ(tg.state.p[0].discard.n, 3, "G CONTROL: the cost really milled 3 cards");
    CHECK(pending(&tg), "a 虹ヶ咲 member without a blade heart offers the choice");
}

static void kanata_member_with_blade_heart_does_not_offer_choice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int member = test_id(&tg, NIJI_MEM_BH);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    pin_id(member, NIJI_MEM_BH);
    CHECK(has_blade_heart(member),
          "G: 優木せつ菜 really HAS a blade heart (blade_heart b_heart02)");
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { member, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    kanata_activate_ab1(&tg, kanata);
    CHECK_EQ(tg.state.p[0].discard.n, 3, "G CONTROL: the cost really milled 3 cards");
    CHECK(!pending(&tg),
          "a 虹ヶ咲 member WITH a blade heart must not satisfy the no-blade branch");
}

static void kanata_no_matching_card_does_not_offer_choice(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int f = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    CHECK(!rb_card_matches_group_str(f, NIJI_GROUP),
          "G: the filler is a live-less CatChu! member, not 『虹ヶ咲』");
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { f, f, f };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    kanata_activate_ab1(&tg, kanata);
    CHECK_EQ(tg.state.p[0].discard.n, 3, "G CONTROL: the cost really milled 3 cards");
    CHECK(!pending(&tg),
          "no 『虹ヶ咲』 live / no-blade-heart member among the discarded 3 -> no choice");
}

static void kanata_option_activate_two_energy(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_id(&tg, NIJI_LIVE);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    int energy = test_id(&tg, ENERGY);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { live, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));
    /* Two energy cards in the zone in WAIT state. */
    tg.state.p[0].energy.cards[0] = energy;
    tg.state.p[0].energy.cards[1] = energy;
    tg.state.p[0].energy.n = 2;
    tg.state.p[0].energy_active = 0;
    int active_before = tg.state.p[0].energy_active;

    kanata_activate_ab1(&tg, kanata);
    CHECK(pending(&tg), "G: the choice is offered");
    DIAG("choice: type=%s count=%d options=%s", pending_kind(&tg),
         rb_get_pending_choice(&tg.state) ? rb_get_pending_choice(&tg.state)->count : -1,
         pending_kind(&tg));
    pick(&tg, 0);
    CHECK_EQ(tg.state.p[0].energy_active, active_before + 2,
             "option 0 activates 2 energy");
}

static void kanata_option_gain_blade_two(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int member = test_id(&tg, NIJI_MEM_NOBH);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { member, fill, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    int blade_before = blade_mod(&tg, kanata);
    kanata_activate_ab1(&tg, kanata);
    CHECK(pending(&tg), "G: the choice is offered");
    pick(&tg, 1);
    CHECK_EQ(blade_mod(&tg, kanata), blade_before + 2, "option 1 grants blade x2");
}

static void kanata_preexisting_live_in_discard_does_not_count(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int kanata = test_id(&tg, KANATA);
    int f = test_id(&tg, FILLER_KANO);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    /* A 虹ヶ咲 live card already in the waitroom BEFORE activating. */
    rb_waitroom_add(&tg.state.p[0], test_id(&tg, NIJI_LIVE));
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { f, f, f };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    kanata_activate_ab1(&tg, kanata);
    CHECK(!pending(&tg),
          "the condition only looks at the 3 cards THIS cost placed, not the "
          "whole waitroom");
}

static void kanata_use_limit_twice_per_turn(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int live = test_id(&tg, NIJI_LIVE);
    int live2 = test_new_id(&tg, NIJI_LIVE);
    int fill = test_id(&tg, FILLER_KANO);
    int kanata = test_id(&tg, KANATA);
    pin_id(kanata, KANATA);
    if (kanata < 0) return;
    tg.state.p[0].stage[1] = kanata;
    int top3[3] = { live, live2, fill };
    setup_deck_exact(&tg, top3, 3, test_id(&tg, FILLER));

    int first = kanata_activate_ab1(&tg, kanata);
    CHECK(first != 0, "G: the first use activates");
    CHECK(pending(&tg), "G CONTROL: the first use really offered a choice");
    dump_choice(&tg);
    pick(&tg, 1);

    DIAG("use_count ab#1 after the first use = %d (limit 2, turn %d)",
         rb_ability_uses_used(&tg.state, kanata, 1), tg.state.turn);
    int second = kanata_activate_ab1(&tg, kanata);
    DIAG("second activation returned %d; waitroom n=%d deck n=%d", second,
         tg.state.p[0].discard.n, tg.state.p[0].deck.n);
    dump_choice(&tg);
    /* The deck is now fillers, so the condition fails and no choice appears; the
     * point of the second use is only that ターン2回 still permits it. */
    CHECK(second != 0, "G: the second use is still permitted (ターン2回)");

    int third = kanata_activate_ab1(&tg, kanata);
    DIAG("third activation returned %d", third);
    CHECK_EQ(third, 0, "G: a third activation in the same turn is blocked by ターン2回");
}

/* =======================================================================
 * H. activation_discard_blade_heart_count_condition_test.rs (4 tests)
 *    PL!SP-bp5-002-R＋ 唐 可可: 起動/左サイド/ターン1回, 「このメンバーを
 *    ウェイトにする：カードを3枚引き、手札を2枚控え室に置く。これにより控え室に
 *    置いたカードの中にブレードハートを持たないメンバーカードが1枚以上ある場合、
 *    このメンバーをアクティブにする。2枚ある場合、さらにブレード×2を得る。」
 * ======================================================================= */

static int keke_act_activate(TestGame *tg, int keke, const int *discard_idx, int n)
{
    int rc = test_activate_ability(tg, keke);
    (void)discard_idx; (void)n;
    return rc;
}

static void keke_act_cost_waits_and_prompts_two_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_ACT);
    int live1 = test_id(&tg, MUS_LIVE);
    int live2 = test_new_id(&tg, MUS_LIVE);
    int filler = test_new_id(&tg, FILLER);
    pin_id(keke, KEKE_ACT);
    CHECK(rb_card_is_live(live1) && rb_card_is_live(live2),
          "H: the two discard fixtures are live cards (not members)");
    if (keke < 0) return;

    tg.state.p[0].stage[0] = keke;
    test_add_to_hand(&tg, live1);
    test_add_to_hand(&tg, live2);
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    keke_act_activate(&tg, keke, NULL, 0);
    CHECK(is_wait(&tg, keke), "H: the self_cost leaves her WAIT");
    CHECK(pending(&tg), "H: the 2-card hand discard is prompted");
    dump_choice(&tg);
    const int both[2] = { 0, 1 };
    pick_n(&tg, both, 2);

    /* NON-VACUITY CONTROL. "0 blades / stays WAIT" is exactly what a board on
     * which the 2-card discard cost never MOVED anything also looks like, so
     * the discard itself is asserted first. */
    CHECK(test_zone_has_id(&tg, 0, "discard", live1) &&
          test_zone_has_id(&tg, 0, "discard", live2),
          "H CONTROL: both picked LIVE cards really reached the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "H CONTROL: the 2-card hand cost really emptied the hand");
    /* Two LIVE cards were discarded: neither is a member card, so the
     * no-blade-heart MEMBER count is 0 and she must stay WAIT with 0 blades. */
    CHECK(is_wait(&tg, keke),
          "H: 0 no-blade-heart MEMBER cards discarded -> she stays WAIT");
    CHECK_EQ(blade_mod(&tg, keke), 0, "H: 0 blades");
}

static void keke_act_discard_1_no_blade_heart_member(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_ACT);
    int live1 = test_id(&tg, MUS_LIVE);
    int no_bh = test_id(&tg, HONOKA);
    int filler = test_new_id(&tg, FILLER);
    pin_id(keke, KEKE_ACT);
    pin_id(no_bh, HONOKA);
    CHECK(!has_blade_heart(no_bh), "H: 高坂穂乃果 really has no blade heart");
    if (keke < 0) return;

    tg.state.p[0].stage[0] = keke;
    test_add_to_hand(&tg, live1);
    test_add_to_hand(&tg, no_bh);
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    keke_act_activate(&tg, keke, NULL, 0);
    CHECK(is_wait(&tg, keke), "H: the self_cost leaves her WAIT");
    CHECK(pending(&tg), "H: the 2-card hand discard is prompted");
    const int both[2] = { 0, 1 };
    pick_n(&tg, both, 2);

    CHECK(!is_wait(&tg, keke),
          "H: 1 no-blade-heart member discarded -> she becomes ACTIVE");
    CHECK_EQ(blade_mod(&tg, keke), 0, "H: 0 blades (only the 2-card branch grants them)");
}

static void keke_act_discard_2_no_blade_heart_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_ACT);
    int m1 = test_id(&tg, HONOKA);
    int m2 = test_new_id(&tg, HONOKA);
    int filler = test_new_id(&tg, FILLER);
    pin_id(keke, KEKE_ACT);
    if (keke < 0) return;

    tg.state.p[0].stage[0] = keke;
    test_add_to_hand(&tg, m1);
    test_add_to_hand(&tg, m2);
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    keke_act_activate(&tg, keke, NULL, 0);
    CHECK(is_wait(&tg, keke), "H: the self_cost leaves her WAIT");
    CHECK(pending(&tg), "H: the 2-card hand discard is prompted");
    const int both[2] = { 0, 1 };
    pick_n(&tg, both, 2);

    CHECK(!is_wait(&tg, keke),
          "H: 2 no-blade-heart members discarded -> she becomes ACTIVE");
    CHECK_EQ(blade_mod(&tg, keke), 2, "H: 2 blades");
}

static void keke_act_discard_2_tomari_no_blade_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_ACT);
    int t1 = test_id(&tg, TOMARI);
    int t2 = test_new_id(&tg, TOMARI);
    int filler = test_new_id(&tg, FILLER);
    pin_id(keke, KEKE_ACT);
    pin_id(t1, TOMARI);
    /* cards.json: 鬼塚冬毬 PL!SP-sd1-011-P has blade=1 (blade COUNT) and NO
     * blade_heart key, so has_blade_heart() is false. */
    CHECK(printed_blade(t1) == 1, "H: 鬼塚冬毬 really prints blade 1");
    CHECK(!has_blade_heart(t1), "H: 鬼塚冬毬 really has NO blade heart");
    if (keke < 0) return;

    tg.state.p[0].stage[0] = keke;
    test_add_to_hand(&tg, t1);
    test_add_to_hand(&tg, t2);
    push_deck_top(&tg, 0, filler, 10);
    test_give_energy(&tg, 10);

    keke_act_activate(&tg, keke, NULL, 0);
    CHECK(is_wait(&tg, keke), "H: the self_cost leaves her WAIT");
    CHECK(pending(&tg), "H: the 2-card hand discard is prompted");
    const int both[2] = { 0, 1 };
    pick_n(&tg, both, 2);

    CHECK(!is_wait(&tg, keke),
          "H: 鬼塚冬毬 has no blade_heart -> both count -> she becomes ACTIVE");
    CHECK_EQ(blade_mod(&tg, keke), 2,
             "H: 鬼塚冬毬 has no blade_heart -> both count -> blade +2");
}

/* =======================================================================
 * I. debut_discard_blade_heart_draw_test.rs (2 tests)
 *    PL!SP-pb2-013-R 唐 可可 登場: 「手札の『KALEIDOSCORE』のカードを1枚控え室に
 *    置いてもよい：自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態
 *    で置く。これにより控え室に置いたカードがブレードハートを持たない場合、
 *    カードを1枚引く。」
 * ======================================================================= */

static void keke_debut_discard_blade_heart_no_draw(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_DEBUT);
    int fodder = test_id(&tg, KEKE_FODDER_BH);
    int filler = test_id(&tg, FILLER);
    int energy = test_id(&tg, ENERGY);
    pin_id(keke, KEKE_DEBUT);
    pin_id(fodder, KEKE_FODDER_BH);
    CHECK(has_blade_heart(fodder),
          "I: the discarded card really HAS a blade heart");
    if (keke < 0) return;

    test_add_to_hand(&tg, keke);
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 10);
    push_deck_top(&tg, 0, filler, 10);
    test_add_to_energy_deck(&tg, 0, energy);

    int energy_before = tg.state.p[0].energy.n;
    int hand_before = tg.state.p[0].hand.n;
    int energy_deck_before = tg.state.p[0].energy_deck.n;

    play_to_stage(&tg, keke, 1, 0);
    drain_only_auto(&tg);
    CHECK(pending(&tg), "I: the optional 『KALEIDOSCORE』 discard prompt appears");
    dump_choice(&tg);
    dump_ability(keke, 0);
    pick(&tg, 0);
    CHECK(!pending(&tg), "I: the prompt is resolved");
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], keke, "I: she is on CENTER");
    /* NON-VACUITY CONTROL: the hand-size and energy assertions below are only
     * meaningful once the discard itself has actually happened. */
    CHECK(!test_hand_has(&tg, fodder), "I: the 『KALEIDOSCORE』 card left hand");
    CHECK(test_zone_has_id(&tg, 0, "discard", fodder), "I: it is in the waitroom");
    CHECK_EQ(tg.state.p[0].energy.n, energy_before + 1,
             "I: 1 energy joined the zone in WAIT state");
    CHECK_EQ(energy_deck_before - tg.state.p[0].energy_deck.n, 1,
             "I: 1 energy left the energy deck");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 2,
             "I: hand = (played member) - (discarded) and NO card was drawn");
}

static void keke_debut_discard_no_blade_heart_draws(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    int keke = test_id(&tg, KEKE_DEBUT);
    int fodder = test_id(&tg, KEKE_FODDER_NB);
    int filler = test_id(&tg, FILLER);
    int energy = test_id(&tg, ENERGY);
    pin_id(keke, KEKE_DEBUT);
    pin_id(fodder, KEKE_FODDER_NB);
    CHECK(!has_blade_heart(fodder),
          "I: the discarded card really has NO blade heart");
    if (keke < 0) return;

    test_add_to_hand(&tg, keke);
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 10);
    push_deck_top(&tg, 0, filler, 10);
    test_add_to_energy_deck(&tg, 0, energy);

    int energy_before = tg.state.p[0].energy.n;
    int hand_before = tg.state.p[0].hand.n;
    int energy_deck_before = tg.state.p[0].energy_deck.n;

    play_to_stage(&tg, keke, 1, 0);
    drain_only_auto(&tg);
    CHECK(pending(&tg), "I: the optional 『KALEIDOSCORE』 discard prompt appears");
    dump_choice(&tg);
    pick(&tg, 0);
    CHECK(!pending(&tg), "I: the prompt is resolved");
    dump_board(&tg);

    CHECK_EQ(tg.state.p[0].stage[1], keke, "I: she is on CENTER");
    /* NON-VACUITY CONTROL: "a card WAS drawn" is indistinguishable from "the
     * whole optional step never ran" unless the discard is shown to have
     * happened, so the discard is asserted first. */
    CHECK(!test_hand_has(&tg, fodder), "I: the 『KALEIDOSCORE』 card left hand");
    CHECK(test_zone_has_id(&tg, 0, "discard", fodder), "I: it is in the waitroom");
    CHECK_EQ(tg.state.p[0].energy.n, energy_before + 1,
             "I: 1 energy joined the zone in WAIT state");
    CHECK_EQ(energy_deck_before - tg.state.p[0].energy_deck.n, 1,
             "I: 1 energy left the energy deck");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "I: a card WAS drawn (the discarded card had no blade heart)");
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

    printf("--- A. success_zone_live_card_count_alternative (PL!-pb1-004-R) ---\n");
    run("A_fixture_identity",   nagisa_two_cards_gets_plus_two);
    run("A_one_card_plus_one",   nagisa_one_card_gets_plus_one);
    run("A_zero_cards_nothing",  nagisa_zero_cards_nothing_happens);
    run("A_two_cards_plus_two",  nagisa_two_cards_gets_plus_two);
    run("A_not_at_center",       nagisa_not_at_center_does_not_fire);
    run("A_expires_on_clear",    nagisa_effect_expires_on_clear);

    printf("--- B. umi_reveal_live_card_gains_live_total_score (PL!-pb1-013-R) ---\n");
    run("B_fixture_identity",    q176_picked_card_tracked_in_revealed_cards);
    run("B_blind_reveal_choice", q176_activate_creates_blind_reveal_choice_opponent_controls_pick);
    run("B_live_plus_one",       q176_reveal_live_card_gains_plus1_score);
    run("B_non_live_no_bonus",   q176_reveal_non_live_card_no_score_modifier);
    run("B_use_limit",           q176_use_limit_blocks_second_activation_same_turn);
    run("B_stays_in_hand",       q176_revealed_card_stays_in_hand);
    run("B_persists",            q176_score_modifier_persists_through_phase_changes);
    run("B_cleared_off_stage",   q176_score_modifier_cleared_when_member_leaves_stage);
    run("B_one_of_many",         q176_opponent_picks_exactly_one_from_many_cards);
    run("B_mixed_pick_live",     q176_choice_path_pick_live_from_mixed_hand);
    run("B_mixed_pick_non_live", q176_choice_path_pick_non_live_from_mixed_hand);
    run("B_tracked",             q176_picked_card_tracked_in_revealed_cards);

    printf("--- C. opponent_hand_blind_reveal_no_live_card_draw (PL!-PR-014-PR) ---\n");
    run("C_appear_choice",       umi_pr014_appear_creates_blind_reveal_choice);
    run("C_no_live_draws_one",   umi_pr014_no_live_card_draws_one);
    run("C_live_present",        umi_pr014_live_card_present_no_draw);
    run("C_reprompt",            umi_pr014_sequential_reprompt_preserves_blind_and_target);
    run("C_hand_not_consumed",   umi_pr014_opponent_hand_not_consumed);
    run("C_you_control",         umi_pr014_you_control_the_pick);

    printf("--- D. under_card_and_success_live_conditions ---\n");
    run("D_ranju_smoke",         ranju_under_plus_one_places_correctly);
    run("D_keke_under_cost",     keke_under_cost_per_unit);
    run("D_umi_alt_smoke",       umi_conditional_alternative_0_1_2);
    run("D_kaleidoscore_gap",    kaleidoscore_optional_branches);
    run("D_genki_invalidate",    genki_invalidate_isolated_to_self);

    printf("--- E. live_phase_success_zone_replacement_q256 ---\n");
    run("E_fixture_identity",     live_phase_success_zone_replacement_routes_choice_to_player_one);
    run("E_p1_accept",           live_phase_success_zone_replacement_routes_choice_to_player_one);
    run("E_declined",            live_phase_declined_replacement_places_original_in_success_zone);
    run("E_p2_routing",          live_phase_success_zone_replacement_routes_choice_to_player_two);
    run("E_dreamin_identity",    live_phase_replacement_preserves_target_identity_dreamin);
    run("E_multi_filter",        live_phase_replacement_filters_multiple_waitroom_targets);
    run("E_debut_dreamin",       debut_reveal_success_zone_replacement_preserves_target_identity_dreamin);
    run("E_no_valid_target",     live_phase_no_valid_replacement_places_original_in_success_zone);

    printf("--- F. debut_reveal_success_zone_replacement_q256 (PL!-sd1-006-SD) ---\n");
    run("F_fixture_identity",     maki_q256_uses_only_the_cost_revealed_card);
    run("F_accepts_replacement", maki_q256_accepts_crossroads_replacement);
    run("F_declines_replacement",maki_q256_declines_crossroads_replacement);
    run("F_can_decline_reveal",  maki_q256_can_decline_reveal);
    run("F_no_live_in_hand",     maki_q256_without_live_card_in_hand_does_nothing);
    run("F_empty_success",       maki_q256_empty_success_still_attempts_placement);
    run("F_selects_exact",       maki_q256_selects_exact_success_card);
    run("F_non_crossroads",      maki_q256_non_crossroads_reveal_bypasses_replacement);
    run("F_non_mus_waitroom",    maki_q256_non_mus_waitroom_does_not_prompt);
    run("F_filters_multi_mus",   maki_q256_filters_multiple_mus_targets);
    run("F_only_cost_revealed",  maki_q256_uses_only_the_cost_revealed_card);

    printf("--- G. discard_placed_group_live_or_blade_condition (PL!N-bp7-006-R＋) ---\n");
    run("G_ability_index_gap",   kanata_setup_uses_the_ability_index_entry_point);
    run("G_cost_mills_three",    kanata_cost_mills_top_three);
    run("G_live_offers_choice",  kanata_live_card_in_discard_offers_choice);
    run("G_member_nobh_choice",  kanata_member_without_blade_heart_offers_choice);
    run("G_member_bh_no_choice", kanata_member_with_blade_heart_does_not_offer_choice);
    run("G_no_match_no_choice",  kanata_no_matching_card_does_not_offer_choice);
    run("G_option_energy",       kanata_option_activate_two_energy);
    run("G_option_blade",        kanata_option_gain_blade_two);
    run("G_preexisting_ignored", kanata_preexisting_live_in_discard_does_not_count);
    run("G_use_limit_twice",     kanata_use_limit_twice_per_turn);

    printf("--- H. activation_discard_blade_heart_count_condition (PL!SP-bp5-002-R＋) ---\n");
    run("H_fixture_identity",    keke_act_discard_2_tomari_no_blade_heart);
    run("H_zero_members",        keke_act_cost_waits_and_prompts_two_cards);
    run("H_one_member",          keke_act_discard_1_no_blade_heart_member);
    run("H_two_members",         keke_act_discard_2_no_blade_heart_members);
    run("H_tomari_two",          keke_act_discard_2_tomari_no_blade_heart);

    printf("--- I. debut_discard_blade_heart_draw (PL!SP-pb2-013-R) ---\n");
    run("I_fixture_identity",    keke_debut_discard_no_blade_heart_draws);
    run("I_blade_heart_no_draw", keke_debut_discard_blade_heart_no_draw);
    run("I_no_blade_heart_draw",keke_debut_discard_no_blade_heart_draws);

    rb_unload();
    /* The assertion / harness-gap counters live in the forked children, so they
     * are reported per case on the "N assertion(s), M failure(s), K harness
     * gap(s)" line each child prints; this summary reports the case counts. */
    printf("\n==== zone_source parity: %d cases passed, %d cases failed, "
           "%d crashed ====\n", n_ok, n_failed, n_crashed);
    return (failures || n_crashed) ? 1 : 0;
}

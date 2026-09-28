/* test_parity_ability_watch.c — C port of engine/tests/test_modules/jidou/
 * ability_watch/ (11 .rs files, 59 `#[test]` functions).
 *
 * Build ONLY through the isolated build root:
 *   cd engine_c_wip
 *   bash tools/isolated_build.sh tr_abilitywatch t T=parity_ability_watch
 *
 * ── COVERAGE BOUNDARY ─────────────────────────────────────────────────────
 * ALREADY COVERED by tests/test_parity_jidou_extra.c §A, and therefore NOT
 * re-ported here (listed so the accounting is checkable):
 *   constant_conditions_exact_counts_and_names_test.rs ................... 4
 *     test_kanan_grants_only_at_exactly_two_members
 *     test_komari_grants_only_when_both_stages_together_hold_six
 *     test_wakana_same_condition_different_colours
 *     test_hime_name_keyed_constants_fire_independently
 *   (that file's other sections are other jidou sub-folders, not this cluster.)
 *
 * NOT PORTED, and why (see the accompanying report for the full accounting):
 *   constant_grant_lifecycle_test.rs ............. 6  (needs a live
 *       set_live_card / live-end rollover walk this harness cannot do
 *       without a phase machine)
 *   lifecycle_tail_idempotence_test.rs ........... 7  (same)
 *   invalidated_live_start_restores_next_live.rs . 3  (needs a real walk to
 *       LiveCardSetSecondAttacker)
 *   copied_ability_registration_lifecycle.rs .... 5  of 12 (the two
 *       filter-discrimination cases and the two phase-walk cases)
 *
 * ── HARNESS NOTES THAT CHANGE THE ASSERTIONS HERE ────────────────────────
 *  - `load_card_db()` falls back to "../cards/build": `rb_load("src")` fails
 *    under the isolated out-of-tree build.
 *  - `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *  - `rb_card_copy_pool_reset()` per case (process-global monotonic pool).
 *  - `test_get_heart_modifier` REMAPS colour 5 onto RB_HEART_ORANGE, so every
 *    heart read here goes through `rb_mods_get_heart` with the RAW enum index
 *    (0=heart00 .. 6=heart06), never through that wrapper.
 *  - A `position|destination` prompt in C is answered with an ABSOLUTE area
 *    index (rb_resume_position_change reads selected_idx as an area), and the
 *    RbChoice carries no option list — so `choose_destination` asserts the
 *    prompt really is position|destination and then answers the area directly.
 *  - C has no `state.turn_number`: the turn counter is `g->queue.use_turn`, and
 *    the Rust turn rollover is `rb_reset_keyword_tracking`.
 *  - `Card.ability` is not populated by `rb_decode_card_by_index`; abilities
 *    come from `rb_card_num_abilities` + `rb_decode_card_ability`.
 *  - `rb_card_no_eq` is FUZZY, so every identity assertion is a byte-exact
 *    `strcmp` of `rb_card_string(c.card_no_idx)`.
 *  - `rb_on_cards_left_zones` exists in modifiers.c but is NOT declared in
 *    include/rabuka.h (not this file's to edit), so it is declared here.
 *  - Rust `game.state.gained_abilities` (the triggerless TEXT registration) has
 *    no C field at all — see src/core/modifiers.c:730-734. The C analogue is
 *    `gained_card_abilities`, read through rb_card_num_gained_abilities, which
 *    is the OTHER half of the same registration. Every "granted" count in §E
 *    therefore names the half it actually read.
 *  - A `pay_optional_cost:skip` gate is a SelectTarget whose option 0 is SKIP
 *    and 1 is PAY. §C's non-μ's case pays (option 1) so the half that resolves
 *    really resolves.
 *
 * ── TRIGGER-FIRING HARNESS ────────────────────────────────────────────────
 * `fire_trigger` + `dump_triggers` are the pair that makes a byte-escape typo
 * impossible to mistake for "the engine did nothing": every failure prints the
 * ACTUAL trigger bytes, in hex, that the card carries. The trigger literals
 * are written as \xNN escapes precisely so the Write tool's encoding cannot
 * corrupt them:
 *   登   U+767B = E7 99 BB   (NOT E7 9A BB)
 *   場   U+5834 = E5 A0 B4
 *   開   U+958B = E9 96 8B   (NOT E5 90 8F)
 *   始   U+59CB = E5 A7 8B   (NOT E6 98 8E)
 *   ラ   U+30E9 = E3 83 A9   (NOT E3 83 96, which is ヴ)
 *   常時 U+5E38 U+6642 = E5 B8 B8 E6 99 82
 *   自動 U+81EA U+52D5 = E8 87 AA E5 8B 95
 *   起動 U+8D77 U+52D5 = E8 B5 B7 E5 8B 95
 * The [TRIGGER-MISS] dump caught three of those wrong escapes the moment this
 * file was first run; the escapes quoted in the task card were wrong too.
 *
 * ── VACUITY CONTROL ───────────────────────────────────────────────────────
 * This cluster is nothing but trigger firing, so a negative assertion on a
 * board where the ability NEVER fired passes trivially. Every negative here is
 * guarded either by CHECK_PROMPTS (counting the prompt still ON SCREEN at the
 * assertion site, not a tally from answer time) or by a paired POSITIVE on the
 * same board driven first in the same process.
 */

#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

/* modifiers.c owns this; include/rabuka.h does not declare it. */
extern void rb_on_cards_left_zones(GameState *g, int card_id);

/* ---------------------------------------------------------------- counters */

static int assertions;
static int failures;
static int n_setup_bugs;
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
    int av_ = (int)(actual); \
    int ev_ = (int)(expected); \
    if (av_ != ev_) { \
        fprintf(stderr, "FAIL: %s (got %d, expected %d)\n", message, av_, ev_); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* PROMPT-COUNT CONTROL: counts the prompt still ON SCREEN right here. A
 * negative assertion licensed only by "nothing appeared" is satisfied by an
 * engine that never offered anything at all. */
#define CHECK_PROMPTS(game, message) do { \
    assertions++; \
    int prompts_on_screen = test_has_pending_choice(game); \
    if (prompts_on_screen <= 0) { \
        fprintf(stderr, "FAIL: [VACUITY CONTROL] %s -- no prompt on screen, so the absence it licenses proves nothing\n", message); \
        failures++; \
    } else { \
        printf("ok: [vacuity control] %s (%d prompt(s) on screen)\n", \
               message, prompts_on_screen); \
    } \
} while (0)

/* ── trigger bytes (escapes, so the source encoding cannot corrupt them) ── */
/* 登場         U+767B U+767A = E7 99 BB E7 8D B2 */
#define TRIG_DEBUT        "\xE7\x99\xBB\xE7\x8D\xB2"
/* ライブ開始時  U+30E9 U+30A4 U+30D6 U+958B U+59CB U+6642
 *                   = E3 83 A9 E3 82 A4 E3 83 96 E9 96 8B E5 A7 8B E6 99 82 */
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
/* ライブ成功時  U+30E9 U+30A4 U+30D6 U+6210 U+529F U+6642
 *                   = E3 83 A9 E3 82 A4 E3 83 96 E6 88 90 E5 8A 9F E6 99 82 */
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"
/* 自動         U+81EA U+52D5 = E8 87 AA E5 8B 95 */
#define TRIG_AUTO         "\xE8\x87\xAA\xE5\x8B\x95"
/* 起動         U+8D77 U+52D5 = E8 B5 B7 E5 8B 95 */
#define TRIG_KIDOU        "\xE8\xB5\xB7\xE5\x8B\x95"
/* 常時         U+5E38 U+6642 = E5 B8 B8 E6 99 82 */
#define TRIG_CONSTANT     "\xE5\xB8\xB8\xE6\x99\x82"

/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B. rb_find_card_by_no is exact-byte and
 * does NOT normalise fullwidth->halfwidth the way Rust's lookup does. */
#define PLUS "\xEF\xBC\x8B"

/* ── fixtures (every card_no verified against cards/cards.json read as UTF-8) */
#define FILLER        "PL!-sd1-010-SD"    /* cost 4, blade 1 */
#define ENERGY        "LL-E-001-SD"
#define CHEAP         "PL!-sd1-001-SD"    /* prints no score */
#define PLAIN_LIVE    "PL!-sd1-019-SD"    /* START:DASH!! score 1 */

/* A. success_zone_conditions_drive_effects_test.rs */
#define RIN           "PL!-bp6-014-N"     /* 星空 凛        lilywhite, cost 4 */
#define MIKI          "PL!-bp6-015-N"     /* 西木野真姫     BiBi,       cost 4 */
#define NIKO_ALT      "PL!-bp6-009-R"     /* 矢澤にこ       BiBi,       cost 9 */
#define SONG          "PL!-bp5-022-L"     /* A song for You! You? You!!   */
#define HEARTBEAT     "PL!-bp4-021-L"     /* ? HEARTBEAT    score 6 */
#define LILYWHITE_LIVE "PL!-bp4-025-L"    /* iMystery       unit lilywhite */
#define BIBI_LIVE     "PL!-bp4-026-L"     /* succesfully... unit BiBi       */
#define LILYWHITE_MEM "PL!-sd1-004-SD"    /* 園田海未       unit lilywhite */
#define EXACT2_A      "PL!-bp3-014-PR"    /* 星空 凛        blade 2 */
#define THREE_BLADE   "PL!-sd1-004-SD"    /* 園田海未       blade 3 */
#define EXACT2_B      "PL!-sd1-006-SD"    /* 西木野真姫     blade 2 */
#define MID_LIVE      "PL!SP-bp1-027-L"   /* score 6 */
#define DEAR_LIVE     "PL!N-bp1-028-L"    /* score 5 */

/* B. constant_energy_and_slot_conditions_test.rs */
#define NIKO_7        "PL!-PR-021-PR"     /* 矢澤にこ       BiBi, cost 7 */
#define NATSUMI       "PL!SP-bp4-009-R"   /* 鬼塚夏美       cost 9 */
#define KAKKO         "PL!SP-bp7-002-R"   /* 唐 可可        cost 2 */
#define FUYURI        "PL!SP-bp5-011-R"   /* 鬼塚冬毬       cost 9 */
#define SAYAKA        "PL!HS-bp6-012-R"   /* 百生吟子       cost 2 */
#define OTHER_SUR     "PL!HS-bp1-012-PR"  /* a different スリーズブeke member */

/* C. Q255 watcher pair: live_start_resolved_own_center_group_reposition +
 *    live_success_resolved_moved_own_center_group_gain_live_score */
#define DANCING       "PL!-bp6-020-L"     /* Dancing stars on me!  score 5 */
#define HONOKA_MUS    "PL!-bp6-001-R" PLUS /* 高坂穂乃果  μ's, LS+LSS */
#define KOTORI_MUS    "PL!-bp6-003-R" PLUS /* 小林愛香    μ's, LS+LSS */
#define CHIKA         "PL!S-PR-013-PR"     /* 高海千歌    Aqours, LS w/ optional {E}{E} */
#define KARIN_NIJI    "PL!N-bp5-016-N"     /* 虹ヶ咲 member, LSS draw+discard */

/* D. live_start_success_resolved_watcher_arms_only_after_resolution_test.rs */
#define VICTORY       "PL!N-bp5-030-L"     /* 繚乱！ビクトリーロード score 5 */
#define NIJISAKU_MEM  "PL!SP-bp2-009-R" PLUS /* a member with a ライブ成功時 ability */

/* E. copied_ability_registration_lifecycle_test.rs */
#define RINA          "PL!N-PR-026-PR"    /* 天王寺璃奈 常時 copy a ライブ成功時 */
#define AYUMU         "PL!N-bp4-001-R"    /* 上原歩夢 虹ヶ咲 cost 2, LSS place energy waited */
#define HAZUKI        "PL!SP-pb2-005-R"   /* 葉月 恋   常時 copy a 起動 */
#define KINOKO        "PL!SP-sd1-006-SD"  /* 桜小路きな子 『Liella!』 起動 */
#define SHIZUKU       "PL!N-bp4-003-R"    /* 渋谷希, a second 虹ヶ咲 source, cost 4 */
#define DECK_OVER     "PL!N-bp4-007-R" PLUS /* over 天王寺璃奈's コスト9以下 ceiling */
#define DREAM         "PL!HS-bp1-019-L"   /* Dream Believers, prints NO LSS */

/* ── helpers ─────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

static void new_game(TestGame *g)
{
    rb_card_copy_pool_reset();
    test_game_new(g);
}

static void fill_decks(TestGame *g, int n)
{
    int filler = test_id(g, FILLER);
    g->state.p[0].deck.n = 0;
    g->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(g, 0, filler);
        test_add_to_deck_pl(g, 1, filler);
    }
}

static int blade(TestGame *g, int cid) { return rb_mods_get_blade(&g->state.mods, cid); }
static int score_mod(TestGame *g, int cid) { return rb_mods_get_score(&g->state.mods, cid); }
static int need_heart(TestGame *g, int cid, int idx) { return rb_mods_get_need_heart(&g->state.mods, cid, idx); }
/* RAW heart-colour index: 0=heart00 .. 6=heart06 (see the header note). */
static int heart(TestGame *g, int cid, int idx) { return rb_mods_get_heart(&g->state.mods, cid, idx); }
static int bonus(TestGame *g) { return (int)g->state.mods.p1_constant_total_score_bonus; }
static int cost_mod(TestGame *g, int cid) { return rb_mods_get_cost(&g->state.mods, cid); }

static int pending(TestGame *g) { return test_has_pending_choice(g); }
static const char *choice_type(TestGame *g) { return test_pending_choice_type(g); }
static const RbChoice *pending_choice(TestGame *g) { return rb_get_pending_choice(&g->state); }

static void clear_stage(TestGame *g, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) g->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static int active_energy(TestGame *g, int pl) { return rb_energy_active_count(&g->state.p[pl]); }
static int stage_cost_total(TestGame *g, int pl)
{
    int total = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = g->state.p[pl].stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        total += (int)c.cost;
        rb_free_card(&c);
    }
    return total;
}
static int printed_cost(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}
static int printed_blade(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.blade;
    rb_free_card(&c);
    return v;
}
static int printed_score(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
    rb_free_card(&c);
    return v;
}

/* Rust `gained_card_abilities.len()` for `cid`. This is the RESOLVED-ABILITY
 * half of the copy registration; the triggerless-text half has no C field. */
static int granted_abilities(TestGame *g, int cid)
{
    return rb_card_num_gained_abilities(&g->state, cid);
}

/* Put exactly `total` energy cards in p's zone, all ACTIVE. Each card is its
 * own pool instance, because a zone's active/waited split is tracked per zone
 * and one id sitting in two zones makes the split ambiguous. */
static void set_energy(TestGame *g, int pl, int total)
{
    RbPlayer *P = &g->state.p[pl];
    P->energy.n = 0;
    P->energy_active = 0;
    for (int i = 0; i < total && P->energy.n < RB_MAX_ZONE; i++) {
        int e = test_new_id(g, ENERGY);
        if (e < 0) return;
        P->energy.cards[P->energy.n++] = e;
    }
    rb_energy_set_active_count(P, total);
}

static void push_success(TestGame *g, int pl, int cid)
{
    RbPlayer *P = &g->state.p[pl];
    if (P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++] = cid;
}
static void drop_success(TestGame *g, int pl, int cid)
{
    RbPlayer *P = &g->state.p[pl];
    for (int i = 0; i < P->success.n; i++)
        if (P->success.cards[i] == cid) P->success.cards[i] = P->success.cards[--P->success.n];
}
static void push_live(TestGame *g, int pl, int cid)
{
    RbPlayer *P = &g->state.p[pl];
    if (P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++] = cid;
}

static void under_push(TestGame *g, int area, int cid)
{
    RbBag *u = &g->state.p[0].under_cards[area];
    if (u->n >= RB_MAX_ZONE) return;   /* RbBag.cards is a fixed array */
    u->cards[u->n++] = cid;
}
static void under_retain(TestGame *g, int area, int drop)
{
    RbBag *u = &g->state.p[0].under_cards[area];
    int k = 0;
    for (int i = 0; i < u->n; i++)
        if (u->cards[i] != drop) u->cards[k++] = u->cards[i];
    u->n = k;
}

/* Rust `game.drain_choices`: SelectTarget takes option 0, everything else is
 * declined. Bounded, because a peer found an ability that re-prompts. */
static void drain(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const char *t = choice_type(g);
        if (t && strcmp(t, "SelectTarget") == 0) rb_resume_with_choice(&g->state, 0);
        else rb_resume_with_choice(&g->state, -1);
    }
}

/* Rust `drain_auto_ability_choices`: accept every SelectAutoAbility ordering
 * prompt, decline everything else. */
static void drain_auto(TestGame *g)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const char *t = choice_type(g);
        if (t && strcmp(t, "SelectAutoAbility") == 0) rb_resume_with_choice(&g->state, 0);
        else rb_resume_with_choice(&g->state, -1);
    }
}

/* Rust `drain_choices_strict(&["SelectCard"], &[0])`: answer only the named
 * prompt kind with the given index, decline anything else, bounded. */
static void drain_strict(TestGame *g, const char *kind, int idx)
{
    int guard = 0;
    while (rb_has_pending_choice(&g->state) && guard++ < 60) {
        const char *t = choice_type(g);
        if (kind && t && strcmp(t, kind) == 0) rb_resume_with_choice(&g->state, idx);
        else rb_resume_with_choice(&g->state, -1);
    }
}

/* ── [TRIGGER-MISS] diagnostic ─────────────────────────────────────────────
 * Prints, as hex, EVERY trigger string the card ACTUALLY carries, so a byte
 * escape typo can never hide behind a silent strcmp miss. */
static void dump_triggers(int cid, const char *want)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d prints card_no='%s' cost=%d blade=%d "
                        "nAb=%d wanted='%s'\n",
                cid, rb_card_string(c.card_no_idx), (int)c.cost, (int)c.blade, n,
                want ? want : "(null)");
        rb_free_card(&c);
    } else {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d cannot decode, wanted='%s'\n",
                cid, want ? want : "(null)");
    }
    if (want) {
        fprintf(stderr, "  [TRIGGER-MISS] want hex:");
        for (const unsigned char *q = (const unsigned char *)want; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, "\n");
    }
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers) {
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d hex:", i);
            for (const unsigned char *q = (const unsigned char *)ab.triggers; *q; q++)
                fprintf(stderr, " %02X", *q);
            fprintf(stderr, "  text='%s'\n", ab.triggers);
        } else {
            fprintf(stderr, "  [TRIGGER-MISS] got  #%d (no trigger)\n", i);
        }
        rb_free_ability(&ab);
    }
}

/* Rust helpers::fire_trigger — find the ability whose printed `triggers` string
 * is exactly `trig`, push it with `cid` as the activating card, drain, and run
 * the pending-auto pass. Returns 0 (after dumping) when the card carries no
 * such ability, so a fixture mistake can never masquerade as engine silence. */
static int fire_trigger(TestGame *g, int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int match = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (!match) continue;
        rb_queue_push(&g->state.queue, cid, i);
        g->state.activating_card = cid;
        rb_drain_ability_queue(&g->state);
        rb_process_pending_auto_abilities(&g->state);
        return 1;
    }
    dump_triggers(cid, trig);
    return 0;
}

/* AGENTS.md identity rule, byte-exact (rb_card_no_eq is fuzzy). */
static void expect_card_no(int cid, const char *no)
{
    char msg[256];
    int ok = 0;
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        ok = strcmp(rb_card_string(c.card_no_idx), no) == 0;
        rb_free_card(&c);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d is byte-exactly %s", cid, no);
    CHECK(ok, msg);
    if (!ok) dump_triggers(cid, "(identity)");
}

/* Rust `assert!(card has the trigger)`. */
static int expect_trigger(int cid, const char *trig, int min_abilities)
{
    char msg[220];
    int n = rb_card_num_abilities((uint32_t)cid);
    snprintf(msg, sizeof(msg), "fixture cid=%d exposes >= %d ability/abilities", cid,
             min_abilities);
    CHECK(n >= min_abilities, msg);
    int found = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) found = 1;
        rb_free_ability(&ab);
    }
    snprintf(msg, sizeof(msg), "fixture cid=%d carries the trigger (hex:", cid);
    if (!found) {
        fprintf(stderr, "FAIL: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, ")\n");
        dump_triggers(cid, trig);
    } else {
        printf("ok: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            printf(" %02X", *q);
        printf(")\n");
    }
    assertions++;
    if (!found) failures++;
    return found;
}

/* How many 起動 / ライブ成功時 abilities `cid` PRINTS, read from the database so
 * a card-pool update moves the expectation with the card. */
static int printed_trigger_count(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid), c = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) c++;
        rb_free_ability(&ab);
    }
    return c;
}

static int on_stage(TestGame *g, int pl, int area, int cid)
{
    return g->state.p[pl].stage[area] == cid;
}
static int stage_is_empty(TestGame *g, int pl, int area)
{
    return g->state.p[pl].stage[area] == RB_EMPTY_SLOT;
}
static int p1_stage_members_probe(TestGame *g)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (g->state.p[0].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

/* action-generator probe: how many 発動 (UseAbility) offers does the engine
 * make for `cid`? The C equivalent of Rust's generated_actions filter. */
static int count_use_offers(TestGame *g, int cid)
{
    RbGeneratedActionList list = rb_generate_action_candidates(&g->state);
    int n = 0;
    for (int i = 0; i < list.count; i++) {
        if (list.actions[i].action_type == RB_ACTION_ACTIVATE_ABILITY &&
            list.actions[i].has_parameters &&
            list.actions[i].parameters.card_id == cid)
            n++;
    }
    if (list.actions) free(list.actions);
    return n;
}

static int has_moved_this_turn(TestGame *g, int cid)
{
    if (cid < 0) return 0;
    if (cid >= (int)(sizeof(g->state.moved_this_turn) / sizeof(g->state.moved_this_turn[0])))
        return 0;
    return g->state.moved_this_turn[cid] != 0;
}

/* Rust `manually_move`: relocate `cid` WITHOUT an ability and record it as a
 * move this turn, which is the precondition half of both Q255 tests. */
static void manually_move(TestGame *g, int pl, int cid, int from, int to)
{
    g->state.p[pl].stage[from] = RB_EMPTY_SLOT;
    g->state.p[pl].stage[to] = cid;
    rb_record_card_movement(&g->state, cid, RB_ZONEID_STAGE, RB_ZONEID_STAGE,
                            pl, /*causer seat*/ 0 /*effect_only*/);
    g->state.position_change_occurred_this_turn = 1;
}

/* Rust `choose_destination`: assert the watcher asked for a
 * `position|destination`, then answer the ABSOLUTE area index. */
static int choose_destination(TestGame *g, int area, const char *label)
{
    char msg[220];
    if (!pending(g)) {
        snprintf(msg, sizeof(msg),
                 "%s: the watcher must ask for a position|destination", label);
        CHECK(0, msg);
        return 0;
    }
    const RbChoice *c = pending_choice(g);
    snprintf(msg, sizeof(msg), "%s: the pending prompt IS position|destination (got '%s')",
             label, c ? (c->target ? c->target : "(null)") : "(null)");
    int ok = c && c->target && strcmp(c->target, "position|destination") == 0;
    CHECK(ok, msg);
    if (!ok) return 0;
    rb_resume_with_choice(&g->state, area);
    drain_auto(g);
    return 1;
}

/* ══════════════════════════════════════════════════════════════════════════
 * §A  success_zone_conditions_drive_effects_test.rs (7 tests)
 *
 * Five abilities whose outcome is decided by what sits in the
 * 成功ライブカード置き場. Presence, per-card SCALING, and TWO thresholds on one
 * quantity — every assertion is an exact number, because each of those has
 * several distinguishable wrong answers.
 * ══════════════════════════════════════════════════════════════════════════ */

/* A1. rin_grants_heart01_while_a_lilywhite_live_is_there_and_not_for_a_member */
static void rin_success_zone_presence_grants_and_withdraws(void)
{
    static TestGame g;
    new_game(&g);

    int rin = test_id(&g, RIN);
    int lily_live = test_id(&g, LILYWHITE_LIVE);
    int lily_mem = test_id(&g, LILYWHITE_MEM);
    CHECK(rin >= 0 && lily_live >= 0 && lily_mem >= 0, "A1: fixtures resolve");
    if (rin < 0 || lily_live < 0 || lily_mem < 0) { n_setup_bugs++; return; }
    expect_card_no(rin, RIN);
    expect_card_no(lily_live, LILYWHITE_LIVE);
    expect_card_no(lily_mem, LILYWHITE_MEM);
    /* Premise read from the database, not typed: the unit name is Japanese and
     * a hand-written near-miss is exactly what makes a group premise vacuous. */
    CHECK(rb_card_matches_group_str(lily_live, "lilywhite"),
          "A1 precondition: the qualifying card really carries a lilywhite unit");
    CHECK(rb_card_matches_group_str(lily_mem, "lilywhite"),
          "A1 precondition: the lilywhite MEMBER also matches the group — only the "
          "condition's own card_type clause can exclude it");
    CHECK(!rb_card_matches_type(lily_mem, "live_card"),
          "A1 precondition: and the member is genuinely not a ライブカード");

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = rin;

    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, rin, 1), 0,
             "A1: an EMPTY success zone must not grant — 「かぎり」 is a condition, "
             "not a flat bonus");

    push_success(&g, 0, lily_mem);
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, rin, 1), 0,
             "A1: a lilywhite MEMBER in the success zone does not satisfy a "
             "condition that requires a ライブカード there (card_type: live_card). "
             "The positive below on the SAME board is the vacuity control: the group "
             "filter is demonstrably live, so a zero here is the type clause, not a "
             "dead condition.");

    push_success(&g, 0, lily_live);
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, rin, 1), 1,
             "A1: a 『lilywhite』 ライブカード in the success zone grants heart01");

    /* The transition the presence grant exists for. Removing only the
     * QUALIFYING card leaves the zone NON-EMPTY, so a grant that only checked
     * emptiness would survive. */
    drop_success(&g, 0, lily_live);
    rb_recalc_constants(&g.state);
    CHECK(g.state.p[0].success.n > 0,
          "A1 precondition: the zone is still holding the non-qualifying member, so "
          "this is a MEMBERSHIP withdrawal and not an empty-zone case");
    CHECK_EQ(heart(&g, rin, 1), 0,
             "A1: the qualifying card left, so the grant must be WITHDRAWN even "
             "though the zone is still non-empty");
}

/* A2. miki_success_zone_presence_is_group_specific */
static void miki_success_zone_presence_is_group_specific(void)
{
    static TestGame g;
    new_game(&g);

    int miki = test_id(&g, MIKI);
    int bibi = test_id(&g, BIBI_LIVE);
    int lily = test_id(&g, LILYWHITE_LIVE);
    CHECK(miki >= 0 && bibi >= 0 && lily >= 0, "A2: fixtures resolve");
    if (miki < 0 || bibi < 0 || lily < 0) { n_setup_bugs++; return; }
    expect_card_no(miki, MIKI);
    expect_card_no(bibi, BIBI_LIVE);
    expect_card_no(lily, LILYWHITE_LIVE);
    CHECK(rb_card_matches_group_str(bibi, "BiBi"),
          "A2 precondition: the 『BiBi』 live matches BiBi");
    CHECK(!rb_card_matches_group_str(bibi, "lilywhite"),
          "A2 precondition: and does NOT match lilywhite, or the two conditions are "
          "not separable and this test proves nothing about which group is checked");
    CHECK(rb_card_matches_group_str(lily, "lilywhite"),
          "A2 precondition: the lilywhite live matches lilywhite");
    CHECK(!rb_card_matches_group_str(lily, "BiBi"),
          "A2 precondition: and does NOT match BiBi");

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = miki;

    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, miki, 6), 0, "A2: an empty success zone grants nothing");

    push_success(&g, 0, lily);
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, miki, 6), 0,
             "A2: a 『lilywhite』 ライブカード does not satisfy 『『BiBi』のカードが"
             "あるかぎり』 — the unit filter is half the condition");

    push_success(&g, 0, bibi);
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, miki, 6), 1,
             "A2: adding the 『BiBi』 ライブカード satisfies the condition and grants "
             "heart06");
    CHECK_EQ(heart(&g, miki, 1), 0,
             "A2: and ONLY heart06 — heart01 belongs to 星空 凛, not to 真姫");
}

/* A3. niko_alt_no_score_when_a_side_slot_member_does_not_print_exactly_two_blades */
static void niko_alt_blade_limit_is_an_equality_not_a_floor(void)
{
    static TestGame g;
    new_game(&g);

    int niko = test_id(&g, NIKO_ALT);
    int exact = test_id(&g, EXACT2_A);
    int three = test_id(&g, THREE_BLADE);
    CHECK(niko >= 0 && exact >= 0 && three >= 0, "A3: fixtures resolve");
    if (niko < 0 || exact < 0 || three < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO_ALT);
    expect_card_no(exact, EXACT2_A);
    expect_card_no(three, THREE_BLADE);
    fill_decks(&g, 40);
    CHECK_EQ(printed_blade(exact), 2, "A3 precondition: the qualifying member prints exactly 2");
    CHECK_EQ(printed_blade(three), 3,
             "A3 precondition: the non-qualifying member prints 3 — one blade MORE, "
             "so a `>=` reading would accept it and this is the only test that can "
             "tell the two readings apart");

    clear_stage(&g, 0);
    g.state.p[0].stage[0] = exact;   /* left  side slot */
    g.state.p[0].stage[1] = niko;    /* centre: it is a センター ability */
    g.state.p[0].stage[2] = three;   /* right side slot */
    rb_recalc_constants(&g.state);
    CHECK_EQ(bonus(&g), 0,
             "A3: a side-slot member printing 3 blades does NOT satisfy 『元々持つ"
             "ブレードの数が2つ』 — the operator is an equality, so more is not better");

    /* Swapping the 3-blade member out for a 2-blade one flips it, which is what
     * makes this a condition test rather than a permanently-false one. */
    g.state.p[0].stage[2] = test_new_id(&g, EXACT2_B);
    rb_recalc_constants(&g.state);
    CHECK_EQ(bonus(&g), 1,
             "A3: replacing it with a 2-blade member grants the +1, so the zero above "
             "was the blade figure and not some unrelated reason the grant never "
             "applies");
}

/* A4. niko_alt_grants_score_only_while_both_side_slots_hold_two_blade_members */
static void niko_alt_needs_both_side_slots(void)
{
    static TestGame g;
    new_game(&g);

    int niko = test_id(&g, NIKO_ALT);
    int left = test_id(&g, EXACT2_A);
    int right = test_id(&g, EXACT2_B);
    CHECK(niko >= 0 && left >= 0 && right >= 0, "A4: fixtures resolve");
    if (niko < 0 || left < 0 || right < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO_ALT);
    expect_card_no(left, EXACT2_A);
    expect_card_no(right, EXACT2_B);
    fill_decks(&g, 40);
    CHECK_EQ(printed_blade(left), 2, "A4 precondition: left member prints exactly 2");
    CHECK_EQ(printed_blade(right), 2, "A4 precondition: right member prints exactly 2");

    clear_stage(&g, 0);
    g.state.p[0].stage[0] = left;
    g.state.p[0].stage[1] = niko;
    g.state.p[0].stage[2] = right;
    rb_recalc_constants(&g.state);
    CHECK_EQ(bonus(&g), 1, "A4: both side slots hold 2-blade members, so the +1 applies");

    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    rb_on_cards_left_zones(&g.state, left);
    rb_recalc_constants(&g.state);
    CHECK_EQ(bonus(&g), 0,
             "A4: one side slot empty is not 『右サイドエリアと左サイドエリアに…"
             "いるかぎり』 — the condition is over BOTH slots, so the +1 is withdrawn");
}

/* A5. song_boost_scales_with_the_number_of_cards_in_the_success_zone */
static void song_boost_scales_per_card(void)
{
    for (int cards = 0; cards <= 3; cards++) {
        static TestGame g;
        new_game(&g);
        int song = test_id(&g, SONG);
        int filler = test_id(&g, FILLER);
        char msg[240];
        snprintf(msg, sizeof(msg), "A5: fixtures resolve (cards=%d)", cards);
        CHECK(song >= 0 && filler >= 0, msg);
        if (song < 0) { n_setup_bugs++; return; }
        if (cards == 0) {
            expect_card_no(song, SONG);
            CHECK_EQ(printed_score(song), 5, "A5: A song for You! prints score 5");
        }
        fill_decks(&g, filler);
        clear_stage(&g, 0);
        push_live(&g, 0, song);
        for (int i = 0; i < cards; i++) {
            int c = test_new_id(&g, CHEAP);
            if (c >= 0) push_success(&g, 0, c);
        }

        if (!fire_trigger(&g, song, TRIG_LIVE_START)) { n_setup_bugs++; continue; }

        /* 0 cards -> +0; n cards -> +2n. A flat +2, an off-by-one in the count,
         * and a cap at 3 are the three distinguishable wrong answers. */
        snprintf(msg, sizeof(msg),
                 "A5: %d card(s) in the success zone -> +2 score each = %d (got %d)",
                 cards, cards * 2, score_mod(&g, song));
        CHECK_EQ(score_mod(&g, song), cards * 2, msg);

        /* heart01/heart03/heart06/heart0 all scale with the same count. The
         * second clause applied without the first (or not at all) is the other
         * distinguishable wrong answer, so all four are read. */
        static const int cols[4] = { 0, 1, 3, 6 };
        for (int k = 0; k < 4; k++) {
            snprintf(msg, sizeof(msg),
                     "A5: %d card(s) -> +%d to need-heart colour index %d (got %d); the "
                     "second clause scales with the first",
                     cards, cards, cols[k], need_heart(&g, song, cols[k]));
            CHECK_EQ(need_heart(&g, song, cols[k]), cards, msg);
        }
    }
}

/* A6. song_boost_does_not_count_opponent_success_zone_cards */
static void song_boost_ignores_the_opponent_success_zone(void)
{
    static TestGame g;
    new_game(&g);

    int song = test_id(&g, SONG);
    int filler = test_id(&g, FILLER);
    CHECK(song >= 0 && filler >= 0, "A6: fixtures resolve");
    if (song < 0) { n_setup_bugs++; return; }
    expect_card_no(song, SONG);
    fill_decks(&g, filler);
    clear_stage(&g, 0);
    push_live(&g, 0, song);
    for (int i = 0; i < 3; i++) {
        int c = test_new_id(&g, CHEAP);
        if (c >= 0) push_success(&g, 1, c);
    }
    CHECK_EQ(g.state.p[1].success.n, 3, "A6 precondition: p2's success zone holds 3");
    CHECK_EQ(g.state.p[0].success.n, 0, "A6 precondition: p1's success zone is empty");

    CHECK(fire_trigger(&g, song, TRIG_LIVE_START),
          "A6: the ability fires (a no-trigger fixture would make the negatives below "
          "vacuous)");
    CHECK_EQ(score_mod(&g, song), 0,
             "A6: p2's success zone is not 『自分の』 — three cards there must not give +6");
    CHECK_EQ(need_heart(&g, song, 0), 0,
             "A6: and no requirement relaxation from the opponent's cards either");
}

/* A7. heartbeat_two_thresholds_partition_the_success_zone_score_total */
static void heartbeat_two_thresholds_partition_the_total(void)
{
    /* Region 1: total below 6 — nothing at all. */
    {
        static TestGame g;
        new_game(&g);
        int hb = test_id(&g, HEARTBEAT);
        int cheap = test_id(&g, CHEAP);
        CHECK(hb >= 0 && cheap >= 0, "A7 region 1: fixtures resolve");
        if (hb < 0 || cheap < 0) { n_setup_bugs++; return; }
        CHECK_EQ(printed_score(cheap), 0,
                 "A7 region 1 precondition: the cheap fixture really carries no "
                 "printed score, so the total really is below 6");
        push_live(&g, 0, hb);
        push_success(&g, 0, cheap);
        CHECK(fire_trigger(&g, hb, TRIG_LIVE_START), "A7 region 1: the ability fires");
        CHECK_EQ(score_mod(&g, hb), 0,
                 "A7 region 1: success-zone total below 6 -> no score");
        CHECK_EQ(need_heart(&g, hb, 0), 0, "A7 region 1: below the 6 threshold nothing is relaxed");
    }

    /* Region 2: total of exactly 6 — relaxation but NOT the extra score. */
    {
        static TestGame g;
        new_game(&g);
        int hb = test_id(&g, HEARTBEAT);
        int mid = test_id(&g, MID_LIVE);
        CHECK(hb >= 0 && mid >= 0, "A7 region 2: fixtures resolve");
        if (hb < 0 || mid < 0) { n_setup_bugs++; return; }
        expect_card_no(mid, MID_LIVE);
        CHECK_EQ(printed_score(mid), 6, "A7 region 2 precondition: the mid live prints score 6");
        push_live(&g, 0, hb);
        push_success(&g, 0, mid);
        CHECK(fire_trigger(&g, hb, TRIG_LIVE_START), "A7 region 2: the ability fires");
        CHECK_EQ(need_heart(&g, hb, 0), -1,
                 "A7 region 2: a total of exactly 6 satisfies 『６以上』 and REDUCES "
                 "heart0 by 1 — 6 is the first value that does. The sign is negative "
                 "because the card says 減らす; reading +1 here would mean the effect "
                 "is applied backwards");
        CHECK_EQ(score_mod(&g, hb), 0,
                 "A7 region 2: but 6 is BELOW 『９以上』, so the additional +1 score must "
                 "NOT apply — the two thresholds are independent");
    }

    /* Region 3: total at or above 9 — both clauses. The total is COMPUTED from
     * the seeded cards, not assumed, so a fixture that quietly failed to reach
     * 9 would otherwise read as a failed threshold. */
    {
        static TestGame g;
        new_game(&g);
        int hb = test_id(&g, HEARTBEAT);
        int mid = test_id(&g, MID_LIVE);
        int dear = test_id(&g, DEAR_LIVE);
        CHECK(hb >= 0 && mid >= 0 && dear >= 0, "A7 region 3: fixtures resolve");
        if (hb < 0 || mid < 0 || dear < 0) { n_setup_bugs++; return; }
        expect_card_no(dear, DEAR_LIVE);
        CHECK_EQ(printed_score(mid) + printed_score(dear), 11,
                 "A7 region 3 precondition: 6 + 5 really clears the 『９以上』 threshold "
                 "(total computed from the fixtures, not assumed)");
        push_live(&g, 0, hb);
        push_success(&g, 0, mid);
        push_success(&g, 0, dear);
        CHECK(fire_trigger(&g, hb, TRIG_LIVE_START), "A7 region 3: the ability fires");
        CHECK_EQ(score_mod(&g, hb), 1,
                 "A7 region 3: a total of 9+ satisfies 『９以上』 and adds +1 score");
        CHECK_EQ(need_heart(&g, hb, 0), -1,
                 "A7 region 3: and the 6-clause reduction still applies — the "
                 "thresholds stack rather than being exclusive");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * §B  constant_energy_and_slot_conditions_test.rs (5 tests)
 *
 * Five 常時 whose grant is set by state this deck can drive precisely: an exact
 * energy count, a stage-cost total compared against the OPPONENT, an AND of two
 * energy clauses, an explicit 「ほかの」 self-exclusion, and a set of three grants
 * keyed to the holder's own SLOT. Each is read on BOTH sides of its boundary.
 * ══════════════════════════════════════════════════════════════════════════ */

/* B1. niko_two_blades_only_at_exactly_seven_energy_cards */
static void niko_two_blades_only_at_exactly_seven_energy(void)
{
    static TestGame g;
    new_game(&g);

    int niko = test_id(&g, NIKO_7);
    CHECK(niko >= 0, "B1: fixture resolves");
    if (niko < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO_7);
    CHECK_EQ(printed_cost(niko), 7, "B1: 矢澤にこ prints cost 7");
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = niko;

    set_energy(&g, 0, 6);
    rb_recalc_constants(&g.state);
    CHECK_EQ(blade(&g, niko), 0,
             "B1: 『ちょうど7枚』 is false at 6 — a 「7枚以上」 reading would grant here");

    set_energy(&g, 0, 7);
    rb_recalc_constants(&g.state);
    CHECK_EQ(blade(&g, niko), 2,
             "B1: exactly 7 energy grants 2 blades — the printed amount, not one");

    set_energy(&g, 0, 8);
    rb_recalc_constants(&g.state);
    CHECK_EQ(blade(&g, niko), 0,
             "B1: 『ちょうど7枚』 is ALSO false at 8 — a 7枚以上 reading grants here, "
             "and this is the assertion that separates the two");

    set_energy(&g, 0, 7);
    rb_recalc_constants(&g.state);
    CHECK_EQ(blade(&g, niko), 2,
             "B1: returning to 7 re-grants, so the zero above was the count and not a "
             "one-shot latch");
}

/* B2. natsumi_three_blades_while_own_stage_cost_total_is_below_the_opponents */
static void natsumi_three_blades_while_below_the_opponent(void)
{
    static TestGame g;
    new_game(&g);

    int natsumi = test_id(&g, NATSUMI);
    int filler = test_id(&g, FILLER);
    CHECK(natsumi >= 0 && filler >= 0, "B2: fixtures resolve");
    if (natsumi < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(natsumi, NATSUMI);
    CHECK_EQ(printed_cost(natsumi), 9, "B2: 鬼塚夏美 prints cost 9");
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    clear_stage(&g, 1);

    g.state.p[0].stage[0] = natsumi;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        g.state.p[1].stage[i] = test_new_id(&g, FILLER);
    rb_recalc_constants(&g.state);
    CHECK(stage_cost_total(&g, 0) < stage_cost_total(&g, 1),
          "B2 precondition: p1's stage cost total must be BELOW p2's for the positive");
    CHECK_EQ(blade(&g, natsumi), 3,
             "B2: own stage total below the opponent's grants 3 blades");

    /* p2 empties: 0 is no longer "higher than p1", so the grant goes. */
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        int leaving = g.state.p[1].stage[slot];
        g.state.p[1].stage[slot] = RB_EMPTY_SLOT;
        if (leaving != RB_EMPTY_SLOT) rb_on_cards_left_zones(&g.state, leaving);
    }
    rb_recalc_constants(&g.state);
    CHECK(stage_cost_total(&g, 0) > 0 && stage_cost_total(&g, 1) == 0,
          "B2 precondition: p1's total is still positive and p2's is now 0");
    CHECK_EQ(blade(&g, natsumi), 0,
             "B2: 『相手より低い』 is false once the opponent's stage is empty — the "
             "comparison really is against the OPPONENT's board, and an "
             "implementation comparing p1 against itself would grant here");
}

/* B3. kakko_costs_two_more_only_when_both_energy_clauses_hold */
static void kakko_costs_two_more_only_when_both_clauses_hold(void)
{
    static TestGame g;
    new_game(&g);

    int kakko = test_id(&g, KAKKO);
    int filler = test_id(&g, FILLER);
    CHECK(kakko >= 0 && filler >= 0, "B3: fixtures resolve");
    if (kakko < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(kakko, KAKKO);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = kakko;

    /* Clause 1 fails (6 < 7) even though p1 leads. */
    set_energy(&g, 0, 6);
    set_energy(&g, 1, 3);
    rb_recalc_constants(&g.state);
    CHECK(active_energy(&g, 1) < active_energy(&g, 0),
          "B3 precondition: p1 leads, so ONLY the 7枚以上 clause is failing");
    CHECK_EQ(cost_mod(&g, kakko), 0,
             "B3: 6 energy is below 『7枚以上』, and the condition is an AND — one "
             "failing clause withholds the whole +2");

    /* Both clauses hold. */
    set_energy(&g, 0, 7);
    rb_recalc_constants(&g.state);
    CHECK_EQ(cost_mod(&g, kakko), 2,
             "B3: 7 energy AND p1 ahead grants the +2 to her own cost");

    /* Clause 2 fails (p2 now leads) while clause 1 still holds. */
    set_energy(&g, 1, 9);
    rb_recalc_constants(&g.state);
    CHECK_EQ(active_energy(&g, 0), 7,
             "B3 precondition: p1 still has 7 energy, so 『7枚以上』 holds");
    CHECK(active_energy(&g, 1) > active_energy(&g, 0),
          "B3 precondition: p2 now leads, so ONLY the 相手より多い clause is failing");
    CHECK_EQ(cost_mod(&g, kakko), 0,
             "B3: 『自分のエネルギーが相手より多いかぎり』 is false, and an AND withholds "
             "the grant — an OR reading would still grant here");
}

/* B4. sayaka_no_energy_from_her_own_presence_but_yes_from_a_partner */
static void sayaka_other_group_clause_excludes_herself(void)
{
    /* NEGATIVE first: she is her own only same-unit member, and 「ほかの」
     * excludes her, so the counter must stop at the cost. */
    {
        static TestGame g;
        new_game(&g);
        int sayaka = test_id(&g, SAYAKA);
        int filler = test_id(&g, FILLER);
        CHECK(sayaka >= 0 && filler >= 0, "B4 negative: fixtures resolve");
        if (sayaka < 0 || filler < 0) { n_setup_bugs++; return; }
        expect_card_no(sayaka, SAYAKA);
        expect_card_no(test_id(&g, OTHER_SUR), OTHER_SUR);
        fill_decks(&g, 40);
        clear_stage(&g, 0);
        /* 6 energy with 5 ACTIVE and 1 WAITED: playing 吟子 costs 2 (5-2=3) and
         * her 登場 activates the one waited card, so the reading is 4 WITH the
         * effect and 3 WITHOUT it — a one-card difference no cost accounting
         * can absorb. Leaving only 1 active would make the play unpayable. */
        set_energy(&g, 0, 6);
        rb_energy_set_active_count(&g.state.p[0], 5);
        CHECK_EQ(active_energy(&g, 0), 5, "B4 negative precondition: 5 active");
        CHECK_EQ(g.state.p[0].energy.n - active_energy(&g, 0), 1,
                 "B4 negative precondition: exactly one waited energy for her 登場 to "
                 "activate");

        test_add_to_hand(&g, sayaka);
        test_play_to_stage(&g, sayaka, 1 /* centre */);
        drain(&g);
        CHECK_EQ(active_energy(&g, 0), 3,
                 "B4 negative: playing 吟子 with NO other same-unit member on stage "
                 "activates nothing: 5 - 2 (cost) = 3 and the waited card stays "
                 "waited. 『ほかの』 is the whole clause and her own presence must not "
                 "satisfy it. The positive below on the SAME deck is the control.");
    }

    /* POSITIVE: a DIFFERENT member of the same unit is already on stage. */
    {
        static TestGame g;
        new_game(&g);
        int sayaka = test_id(&g, SAYAKA);
        int other = test_id(&g, OTHER_SUR);
        int filler = test_id(&g, FILLER);
        CHECK(sayaka >= 0 && other >= 0 && filler >= 0, "B4 positive: fixtures resolve");
        if (sayaka < 0 || other < 0) { n_setup_bugs++; return; }
        expect_card_no(other, OTHER_SUR);
        /* The partner must be a DIFFERENT character, or 「ほかの」 is ambiguous. */
        {
            Card a, b;
            int distinct = 0;
            if (rb_decode_card_by_index((uint32_t)sayaka, &a) &&
                rb_decode_card_by_index((uint32_t)other, &b)) {
                distinct = (a.name && b.name && strcmp(a.name, b.name) != 0);
                rb_free_card(&a);
                rb_free_card(&b);
            }
            CHECK(distinct,
                  "B4 positive precondition: the partner is a DIFFERENT character — a "
                  "second print of 吟子 would make ほかの ambiguous");
        }
        fill_decks(&g, 40);
        clear_stage(&g, 0);
        set_energy(&g, 0, 6);
        rb_energy_set_active_count(&g.state.p[0], 5);
        g.state.p[0].stage[0] = other;
        test_add_to_hand(&g, sayaka);
        test_play_to_stage(&g, sayaka, 1 /* centre */);
        drain(&g);
        CHECK_EQ(active_energy(&g, 0), 4,
                 "B4 positive: with another same-unit member already staged the waited "
                 "card is activated too: 5 - 2 (cost) + 1 (effect) = 4. The one-card "
                 "difference from the negative case is the whole claim, and it is what "
                 "a missing ほかの exclusion would erase.");
    }
}

/* B5. fuyuri_which_three_hearts_she_grants_depends_entirely_on_her_slot */
static void fuyuri_slot_picks_which_hearts_she_grants(void)
{
    static TestGame g;
    new_game(&g);

    int fuyuri = test_id(&g, FUYURI);
    int filler = test_id(&g, FILLER);
    CHECK(fuyuri >= 0 && filler >= 0, "B5: fixtures resolve");
    if (fuyuri < 0) { n_setup_bugs++; return; }
    expect_card_no(fuyuri, FUYURI);
    /* The three 常時 are SEPARATE abilities keyed to her slot, so the fixture
     * must really carry three. */
    CHECK_EQ(rb_card_num_abilities((uint32_t)fuyuri), 3,
             "B5 precondition: 冬毬 carries three 常時, one per slot");
    fill_decks(&g, 40);

    /* LEFT: heart02 only. */
    clear_stage(&g, 0);
    g.state.p[0].stage[0] = fuyuri;
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, fuyuri, 2), 3, "B5 LEFT: 3x heart02");
    CHECK_EQ(heart(&g, fuyuri, 3), 0, "B5 LEFT: and no heart03");
    CHECK_EQ(heart(&g, fuyuri, 5), 0, "B5 LEFT: and no heart05");

    /* CENTRE: heart03 only — the heart02 grant must be GONE, not accumulated. */
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].stage[1] = fuyuri;
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, fuyuri, 2), 0,
             "B5 CENTRE: heart02 is withdrawn — an accumulating implementation would "
             "still read 3");
    CHECK_EQ(heart(&g, fuyuri, 3), 3, "B5 CENTRE: 3x heart03");
    CHECK_EQ(heart(&g, fuyuri, 5), 0, "B5 CENTRE: and no heart05");

    /* RIGHT: heart05 only. */
    g.state.p[0].stage[1] = RB_EMPTY_SLOT;
    g.state.p[0].stage[2] = fuyuri;
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, fuyuri, 2), 0, "B5 RIGHT: no heart02");
    CHECK_EQ(heart(&g, fuyuri, 3), 0, "B5 RIGHT: the centre's heart03 is withdrawn");
    CHECK_EQ(heart(&g, fuyuri, 5), 3, "B5 RIGHT: the RIGHT slot grants 3x heart05");

    /* Off the board entirely: all three withdrawn. */
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    rb_on_cards_left_zones(&g.state, fuyuri);
    rb_recalc_constants(&g.state);
    CHECK_EQ(heart(&g, fuyuri, 2) + heart(&g, fuyuri, 3) + heart(&g, fuyuri, 5), 0,
             "B5 OFF-STAGE: with her off stage no slot gate is satisfied, so all three "
             "grants are withdrawn — rule 4.1.4 makes the leaving card a new card");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §C  live_start_resolved_own_center_group_reposition_q255_test.rs (5)
 *  +  live_success_resolved_moved_own_center_group_gain_live_score_q255 (6)
 *  +  resolved_center_member_ability_movement_score_q255_test.rs (1)
 *
 * 12 tests over one card, PL!-bp6-020-L "Dancing stars on me!":
 *   ab#0 自動 ターン1回 — a 『μ's』 member in OWN centre resolves a ライブ開始時
 *         ability -> position-change THAT member.
 *   ab#1 自動 ターン1回 — a 『μ's』 member in OWN centre resolves a ライブ成功時
 *         ability -> if that member moved this turn -> +1 score to THIS live.
 * Q255 (2026.06.02): the watcher STILL fires when the member has already moved
 * off centre by resolution time; the triggering member's GROUP identity is what
 * matters, not its position at resolution time.
 *
 * This whole section exercises `rb_trigger_each_time_for_member`, which has
 * ZERO callers in the engine (the task card's report, re-checked here: the only
 * occurrence of that name anywhere under src/ is its own definition at
 * src/core/game_state_abilities.c:1705). Every assertion below is the strict
 * Rust assertion; none is relaxed to accommodate that.
 * ══════════════════════════════════════════════════════════════════════════ */

/* C1. live_start_resolution_reposition_resolving_center_group_member */
static void q255_live_start_repositions_the_resolving_member(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(filler >= 0 && ds >= 0 && honoka >= 0, "C1: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(ds, DANCING);
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"),
          "C1 precondition: 高坂穂乃果 really is a 『μ's』 member — the watcher keys "
          "on that group, and a typo here would make every assertion vacuous");
    expect_trigger(honoka, TRIG_LIVE_START, 1);
    CHECK_EQ(printed_score(ds), 5, "C1 precondition: Dancing stars on me! prints 5");
    CHECK_EQ(score_mod(&g, ds), 0, "C1 precondition: nothing modifies the live yet");

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_START), "C1: 穂乃果's ライブ開始時 resolves");

    CHECK(choose_destination(&g, 0 /* left */, "C1"),
          "C1: the watcher asks for a destination");
    CHECK(on_stage(&g, 0, 0, honoka),
          "C1: the RESOLVING member (Honoka) is the one repositioned");
    CHECK(stage_is_empty(&g, 0, 1), "C1: center now empty");
    CHECK(has_moved_this_turn(&g, honoka), "C1: reposition counts as a move this turn");
    CHECK_EQ(score_mod(&g, ds), 0, "C1: ab#0 alone grants no score");
    CHECK_EQ(pending(&g), 0, "C1: no residual prompts after the reposition");
}

/* C2. live_start_resolution_reposition_q255_member_already_off_center_still_triggers */
static void q255_live_start_fires_even_when_already_off_center(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C2: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"), "C2 precondition: 穂乃果 is μ's");
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    manually_move(&g, 0, honoka, 1, 0); /* centre -> left */
    CHECK(stage_is_empty(&g, 0, 1), "C2 precondition: she really is OFF centre now");

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_START), "C2: 穂乃果's ライブ開始時 resolves");

    CHECK(choose_destination(&g, 1 /* centre */, "C2"),
          "C2: the watcher still asks for a destination");
    CHECK(on_stage(&g, 0, 1, honoka),
          "C2: Q255 — the watcher fired despite the resolver being off centre at "
          "resolution time");
}

/* C3. live_start_resolution_reposition_non_mus_resolver_does_not_reposition_anything */
static void q255_live_start_non_mus_resolver_does_not_reposition(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int chika = test_id(&g, CHIKA);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && chika >= 0 && honoka >= 0, "C3: fixtures resolve");
    if (ds < 0 || chika < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(chika, CHIKA);
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"),
          "C3 precondition: the μ's BAIT really is μ's — a naive board-count reading "
          "of the group condition is exactly what this case exists to catch");
    CHECK(rb_card_matches_group_str(chika, "Aqours"),
          "C3 precondition: and the resolver is Aqours, so it is genuinely non-μ's");
    expect_trigger(chika, TRIG_LIVE_START, 1);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = chika;
    g.state.p[0].stage[0] = honoka;
    set_energy(&g, 0, 4);

    CHECK(fire_trigger(&g, chika, TRIG_LIVE_START), "C3: 千歌's ライブ開始時 resolves");

    /* Chika's own LS carries an OPTIONAL ability-level cost (`cost.optional:
     * true`, `pay_energy` 2) — NOT a `pay_optional_cost:skip` SelectTarget, so
     * the C auto-skips it and raises NO prompt. That is the correct reading of
     * the data (verified in cards/abilities.json), so the Rust test's "answer
     * the optional-cost gate with PAY" has no C counterpart to answer. What
     * matters for the negative below is only that her ability RESOLVED, which
     * fire_trigger already asserted; her auto-declined optional cost does not
     * stop the resolution.
     *
     * The vacuity control for this negative is C1, which is the SAME board
     * except the resolver is a μ's member: C1 asserts the destination prompt
     * DOES appear there. So a zero here is the group filter. */
    CHECK_EQ(pending(&g), 0,
             "C3: her optional cost is auto-declined without a prompt, and the "
             "resolution is complete — this is the control point for the negatives "
             "below, which C1 (same board, μ's resolver) makes non-vacuous");
    CHECK_EQ(rb_queue_has_pending_actions(&g.state), 0,
             "C3: and the ability queue is fully drained — her resolution RAN to "
             "completion rather than parking mid-ability, so the negatives below "
             "are not licensed by 'she never finished'");
    drain_auto(&g);

    CHECK_EQ(pending(&g), 0,
             "C3: a NON-μ's resolver must NOT arm the watcher — no destination prompt "
             "allowed");
    CHECK(on_stage(&g, 0, 1, chika), "C3: Chika stays in center");
    CHECK(on_stage(&g, 0, 0, honoka), "C3: Honoka was never moved");
    CHECK_EQ(score_mod(&g, ds), 0, "C3: and no score either");
}

/* C4. live_start_resolution_reposition_opponent_member_resolution_does_not_arm_own_watcher */
static void q255_live_start_opponent_resolution_does_not_arm_our_watcher(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int p2_honoka = test_new_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && p2_honoka >= 0, "C4: fixtures resolve");
    if (ds < 0 || p2_honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(p2_honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(p2_honoka, "μ's"),
          "C4 precondition: P2's 穂乃果 really is μ's, so the ONLY thing separating "
          "this from the positive is 自分のステージ");
    expect_trigger(p2_honoka, TRIG_LIVE_START, 1);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    clear_stage(&g, 1);
    push_live(&g, 0, ds);
    g.state.p[1].stage[1] = p2_honoka;

    /* Fire P2's Honoka LS through P2's own queue (the mirror of fire_trigger). */
    CHECK(fire_trigger(&g, p2_honoka, TRIG_LIVE_START), "C4: P2's ライブ開始時 resolves");
    drain_auto(&g);

    CHECK_EQ(pending(&g), 0,
             "C4: an opponent-stage resolution must not trigger OUR watcher");
    CHECK(on_stage(&g, 1, 1, p2_honoka), "C4: P2's member not repositioned");
    CHECK_EQ(p1_stage_members_probe(&g), 0, "C4: our stage untouched");
    CHECK_EQ(score_mod(&g, ds), 0, "C4: and our live card gains nothing");
}

/* C5. live_start_resolution_reposition_once_per_turn_blocks_second_resolution */
static void q255_live_start_is_once_per_turn(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C5: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(honoka, HONOKA_MUS);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_START), "C5: first ライブ開始時 resolves");
    CHECK(choose_destination(&g, 0 /* left */, "C5"),
          "C5: the first resolution opens the destination prompt");

    /* Deliberate re-fire in the same turn (the fire_trigger contract). */
    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_START), "C5: second ライブ開始時 resolves");
    CHECK_EQ(pending(&g), 0,
             "C5: ターン1回 — a second LS resolution in the SAME turn must not "
             "re-open the destination prompt");
    CHECK(on_stage(&g, 0, 0, honoka),
          "C5: Honoka stayed at left — no second reposition");
}

/* C6. live_success_resolution_score_unmoved_resolver_scores_nothing */
static void q255_live_success_unmoved_resolver_scores_nothing(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C6: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(honoka, HONOKA_MUS);
    expect_trigger(honoka, TRIG_LIVE_SUCCESS, 1);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    CHECK(!has_moved_this_turn(&g, honoka),
          "C6 precondition: she has NOT moved, so the watcher's has_moved gate is the "
          "only thing that can withhold the score");
    /* PAIRED POSITIVE pointer: C7 is the same board with exactly one difference
     * (the move), and C7 asserts +1. That pairing is what makes this zero
     * non-vacuous, and it is stated here so the two cannot be read apart. */
    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C6: 穂乃果's ライブ成功時 resolves");

    CHECK_EQ(pending(&g), 0,
             "C6: nothing to prompt — her own LSS condition misses and the watcher's "
             "has_moved gate fails");
    CHECK_EQ(score_mod(&g, ds), 0,
             "C6: has_moved gate — an unmoved resolver scores nothing");
}

/* C7. live_success_resolution_score_q255_moved_before_resolution_scores */
static void q255_live_success_moved_before_resolution_scores(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C7: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"), "C7 precondition: 穂乃果 is μ's");
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    manually_move(&g, 0, honoka, 1, 0); /* centre -> left */
    CHECK(has_moved_this_turn(&g, honoka),
          "C7 precondition: the move really was recorded — this is the only difference "
          "from C6, so C7 is C6's vacuity control");
    /* C6's paired negative — it must hold for the reason C7 names. */
    CHECK_EQ(bonus(&g), 0,
             "C7: before the resolution the live card carries no 常時 total-score "
             "bonus, so the +1 below cannot be a leftover constant");

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C7: 穂乃果's ライブ成功時 resolves");

    CHECK_EQ(pending(&g), 0, "C7: no prompts expected");
    CHECK_EQ(score_mod(&g, ds), 1,
             "C7: Q255 — a moved-this-turn μ's resolver whose LSS resolved gives "
             "exactly +1 on the live card");
}

/* C8. live_success_resolution_score_non_mus_resolver_no_score_even_with_mus_staged_and_moved */
static void q255_live_success_non_mus_resolver_no_score(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int karin = test_id(&g, KARIN_NIJI);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && karin >= 0 && honoka >= 0, "C8: fixtures resolve");
    if (ds < 0 || karin < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(karin, KARIN_NIJI);
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"),
          "C8 precondition: the μ's BAIT is really μ's");
    CHECK(!rb_card_matches_group_str(karin, "μ's"),
          "C8 precondition: the resolver is genuinely NOT μ's — otherwise this case "
          "cannot tell a group filter from a has_moved filter");
    CHECK(rb_card_matches_group_str(karin, "虹ヶ咲"),
          "C8 precondition: she is 虹ヶ咲, the unit the task card pins as "
          "E8 99 B9 E3 83 B6 E5 92 B2");
    expect_trigger(karin, TRIG_LIVE_SUCCESS, 1);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = karin;
    g.state.p[0].stage[0] = honoka;
    manually_move(&g, 0, karin, 1, 0);
    CHECK(has_moved_this_turn(&g, karin), "C8 precondition: the resolver DID move");
    /* C7's +1 is the vacuity control for this zero: on the SAME watcher and the
     * same live card, a μ's moved resolver scored +1. */
    CHECK_EQ(bonus(&g), 0,
             "C8: no 常時 total-score bonus on the board either, so a zero below is "
             "the watcher's group filter and not a missing modifier");

    CHECK(fire_trigger(&g, karin, TRIG_LIVE_SUCCESS), "C8:  Develop's ライブ成功時 resolves");
    drain_strict(&g, "SelectCard", 0);   /* answer her own draw+discard prompt only */

    CHECK_EQ(pending(&g), 0, "C8: no watcher prompts expected");
    CHECK_EQ(score_mod(&g, ds), 0,
             "C8: the resolver is 虹ヶ咲, not μ's — the watcher must not score even "
             "though she moved and a μ's member is staged");
}

/* C9. live_success_resolution_score_once_per_turn_single_score_for_two_resolvers */
static void q255_live_success_is_once_per_turn_for_two_resolvers(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    int kotori = test_id(&g, KOTORI_MUS);
    CHECK(ds >= 0 && honoka >= 0 && kotori >= 0, "C9: fixtures resolve");
    if (ds < 0 || honoka < 0 || kotori < 0) { n_setup_bugs++; return; }
    expect_card_no(kotori, KOTORI_MUS);
    CHECK(rb_card_matches_group_str(kotori, "μ's"), "C9 precondition: 小林愛香 is μ's");
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    g.state.p[0].stage[2] = kotori;
    manually_move(&g, 0, honoka, 1, 0); /* honoka: centre -> left */

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C9: 穂乃果's LSS resolves");
    CHECK_EQ(score_mod(&g, ds), 1, "C9: the first resolution scores");

    /* Kotori moves right -> centre, then HER LSS resolves the same turn. */
    manually_move(&g, 0, kotori, 2, 1);
    CHECK(fire_trigger(&g, kotori, TRIG_LIVE_SUCCESS), "C9: 愛香's LSS resolves");
    drain_auto(&g);
    CHECK_EQ(pending(&g), 0,
             "C9: 愛香's optional LSS should auto-skip with nothing under her — a "
             "prompt here means her ability did not auto-decline, not a watcher fire");
    CHECK_EQ(score_mod(&g, ds), 1,
             "C9: ターン1回 — a second μ's LSS resolution in the same turn adds nothing");
}

/* C10. live_success_q255_unrelated_member_movement_does_not_score */
static void q255_live_success_unrelated_movement_does_not_score(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    int ally = test_id(&g, KOTORI_MUS);
    CHECK(ds >= 0 && honoka >= 0 && ally >= 0, "C10: fixtures resolve");
    if (ds < 0 || honoka < 0 || ally < 0) { n_setup_bugs++; return; }
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    g.state.p[0].stage[2] = ally;
    manually_move(&g, 0, ally, 2, 0);   /* the ALLY moves, not the resolver */
    CHECK(has_moved_this_turn(&g, ally), "C10 precondition: the ALLY moved");
    CHECK(!has_moved_this_turn(&g, honoka),
          "C10 precondition: the RESOLVER did not move — only the bystander did");

    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C10: 穂乃果's LSS resolves");
    drain_auto(&g);
    CHECK_EQ(score_mod(&g, ds), 0,
             "C10: the has_moved gate is about the RESOLVING member, not anyone on "
             "stage — a board-level movement flag would score here");
}

/* C11. live_success_q255_turn_one_limit_resets_on_next_turn */
static void q255_live_success_turn_limit_resets_on_next_turn(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C11: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;
    manually_move(&g, 0, honoka, 1, 0);
    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C11: turn 1 resolution");
    CHECK_EQ(score_mod(&g, ds), 1, "C11: turn 1 scores");

    g.state.queue.use_turn += 1;
    rb_reset_keyword_tracking(&g.state);
    manually_move(&g, 0, honoka, 0, 1);
    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C11: turn 2 resolution");
    drain_auto(&g);
    CHECK_EQ(score_mod(&g, ds), 2,
             "C11: ターン1回 resets — the second turn's resolution scores again, so a "
             "latch that never clears would read 1");
}

/* C12. resolved_live_success_after_center_movement_gains_score_q255 */
static void q255_live_success_after_center_movement_gains_score(void)
{
    static TestGame g;
    new_game(&g);

    int filler = test_new_id(&g, FILLER);
    int ds = test_id(&g, DANCING);
    int honoka = test_id(&g, HONOKA_MUS);
    CHECK(ds >= 0 && honoka >= 0, "C12: fixtures resolve");
    if (ds < 0 || honoka < 0) { n_setup_bugs++; return; }
    expect_card_no(ds, DANCING);
    expect_card_no(honoka, HONOKA_MUS);
    CHECK(rb_card_matches_group_str(honoka, "μ's"), "C12 precondition: 穂乃果 is μ's");
    expect_trigger(honoka, TRIG_LIVE_SUCCESS, 1);
    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, ds);
    g.state.p[0].stage[1] = honoka;

    CHECK_EQ(score_mod(&g, ds), 0, "C12: before any resolution the live is unmodified");

    /* Centre -> left, then HER LSS resolves on this same board. */
    manually_move(&g, 0, honoka, 1, 0);
    CHECK(fire_trigger(&g, honoka, TRIG_LIVE_SUCCESS), "C12: 穂乃果's LSS resolves");
    drain_auto(&g);

    CHECK_EQ(score_mod(&g, ds), 1,
             "C12: the full chain — a μ's centre member moves this turn, then her "
             "ライブ成功時 resolves, and the watcher gives +1 to the live card");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §D  live_start_success_resolved_watcher_arms_only_after_resolution_test.rs
 *     (1 test)
 *
 * The DORMANT case: a 「〜のライブ成功時能力が解決するたび」 watcher must not
 * fire from a bare TAS scan. The card is PL!N-bp5-030-L 繚乱！ビクトリーロード.
 * ══════════════════════════════════════════════════════════════════════════ */

static void resolution_watcher_dormant_without_a_resolution_event(void)
{
    static TestGame g;
    new_game(&g);

    int victory = test_id(&g, VICTORY);
    int member = test_id(&g, NIJISAKU_MEM);
    int filler = test_id(&g, FILLER);
    CHECK(victory >= 0 && member >= 0 && filler >= 0, "D: fixtures resolve");
    if (victory < 0 || member < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(victory, VICTORY);
    expect_card_no(member, NIJISAKU_MEM);
    /* The watcher only means anything if the card really carries the each_time
     * 自動 and the member really carries a ライブ成功時. Pinned here, because a
     * card with neither would make the negatives vacuous. */
    expect_trigger(victory, TRIG_AUTO, 1);
    expect_trigger(member, TRIG_LIVE_SUCCESS, 1);
    CHECK_EQ(printed_score(victory), 5, "D: 繚乱！ビクトリーロード prints score 5");

    fill_decks(&g, 40);
    clear_stage(&g, 0);
    push_live(&g, 0, victory);
    g.state.p[0].stage[1] = member;
    test_add_to_hand(&g, filler);

    int deck_before = g.state.p[0].deck.n;

    /* Rust: trigger_auto_abilities_for_player + process_pending_auto_abilities
     * + drain_auto_ability_choices, with NO ability resolution anywhere. */
    rb_trigger_auto_abilities_for_player(&g.state, 0);
    rb_process_pending_auto_abilities(&g.state);
    drain_auto(&g);

    /* PACED POSITIVE: the very same board, one ライブ成功時 RESOLUTION later, in a
     * second game so the negative above is not polluted. It is NOT a claim that
     * the deck moves by exactly 1 — the delta on that board is the member's OWN
     * LSS draw, which is precisely why the negative above must not be read as a
     * positive. It is only the control that the deck counter is a live
     * observable at all. */
    {
        static TestGame h;
        new_game(&h);
        int victory2 = test_id(&h, VICTORY);
        int member2 = test_id(&h, NIJISAKU_MEM);
        int filler2 = test_id(&h, FILLER);
        fill_decks(&h, 40);
        clear_stage(&h, 0);
        push_live(&h, 0, victory2);
        h.state.p[0].stage[1] = member2;
        test_add_to_hand(&h, filler2);
        int h_before = h.state.p[0].deck.n;
        CHECK(fire_trigger(&h, member2, TRIG_LIVE_SUCCESS),
              "D positive premise: the member's ライブ成功時 really resolves");
        fprintf(stderr, "[D-VICTORY-PROBE] bare-scan deck=%d; after-resolution "
                        "deck %d->%d hand=%d prompts=%d\n",
                deck_before, h_before, h.state.p[0].deck.n,
                h.state.p[0].hand.n, pending(&h));
        CHECK(h.state.p[0].deck.n < h_before,
              "D positive control: a real ライブ成功時 resolution DOES change the deck, "
              "so the deck counter is a live observable and the zero below is a "
              "withheld effect rather than a dead measurement");
    }

    CHECK_EQ(pending(&g), 0,
             "D: the watcher must not prompt without a resolution event");
    CHECK_EQ(g.state.p[0].deck.n, deck_before,
             "D: the each_time ライブ成功時 watcher must NOT draw from a bare TAS scan");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §E  copied_ability_registration_lifecycle_test.rs (7 of 12 tests)
 *
 * 「○○の能力を1枚コピー」, the 常時 form. The copy is registered on the COPIER
 * and driven by `rb_recalc_constants` (Rust `recalculate_constants`) with the
 * eligible source sitting in the HOST'S OWN UNDER-AREA — a different driving
 * idiom from §A/§B, which use `fire_trigger`.
 *
 * THE C DEVIATION, stated up front: Rust tracks this in BOTH
 * `state.gained_abilities` (the triggerless TEXT registration) and
 * `state.gained_card_abilities` (the resolved-Ability table), and two of the
 * Rust tests assert the two halves AGREE. The C has ONLY the second — see the
 * comment at src/core/modifiers.c:730-734. So every count below reads
 * `rb_card_num_gained_abilities` (the resolved half) and says so in its
 * message, and no assertion here can speak to the text half at all. That is a
 * PORT gap, not something this file can assert on.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust `stage_host_with_source_under`: host in the left area, the eligible
 * source DIRECTLY UNDER IT. */
static void stage_host_with_source_under(TestGame *g, const char *host_no,
                                         const char *source_no, int *host, int *source)
{
    int filler = test_id(g, FILLER);
    *host = test_id(g, host_no);
    *source = test_id(g, source_no);
    clear_stage(g, 0);
    g->state.p[0].stage[0] = *host;
    g->state.p[0].stage[1] = filler;
    g->state.p[0].stage[2] = RB_EMPTY_SLOT;
    g->state.p[0].under_cards[0].n = 0;
    if (*source >= 0) under_push(g, 0, *source);
    fill_decks(g, 40);
}

/* E1. rina_registers_one_copied_live_success_per_recalculation */
static void rina_registers_one_copy_per_recalculation(void)
{
    static TestGame g;
    new_game(&g);

    int rina, ayumu;
    stage_host_with_source_under(&g, RINA, AYUMU, &rina, &ayumu);
    if (rina < 0 || ayumu < 0) { n_setup_bugs++; return; }
    expect_card_no(rina, RINA);
    expect_card_no(ayumu, AYUMU);
    /* Premise: the source really is eligible (虹ヶ咲, cost 2 under the コスト9以下
     * ceiling) and really does print the ライブ成功時 being copied. Without this,
     * "one copy" is also what an INELIGIBLE source would produce. */
    CHECK(rb_card_matches_group_str(ayumu, "A・ZU・NA"),
          "E1 precondition: the copy source is 虹ヶ咲");
    CHECK_EQ(printed_cost(ayumu), 2, "E1 precondition: 上原歩夢 costs 2");
    CHECK_EQ(printed_trigger_count(ayumu, TRIG_LIVE_SUCCESS), 1,
             "E1 precondition: the copy source must print exactly one ライブ成功時, "
             "otherwise the count below does not name a duplication");

    /* One scan first, so a later failure cannot be "the scan never ran". */
    rb_recalc_constants(&g.state);

    /* ROOT-CAUSE PROBE: the 璃奈 copy ability really is a 常時 on this card
     * whose top-level action is `gain_ability_from_source` over the host's own
     * under-area. If that is true, a zero registration means the constant
     * dispatcher did not run this action — not that the fixture was ineligible
     * (every eligibility premise above already passed). */
    {
        int n = rb_card_num_abilities((uint32_t)rina), found = 0;
        for (int i = 0; i < n; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)rina, i, &ab)) continue;
            if (ab.triggers && strcmp(ab.triggers, TRIG_CONSTANT) == 0 &&
                ab.effect && ab.effect->action &&
                strcmp(ab.effect->action, "gain_ability_from_source") == 0)
                found = 1;
            rb_free_ability(&ab);
        }
        CHECK(found,
              "E1 ROOT CAUSE: 璃奈 really does print a 常時 whose action is "
              "gain_ability_from_source over her own under-area, so a zero "
              "registration below is the constant dispatcher skipping that action");
    }

    CHECK_EQ(granted_abilities(&g, rina), 1,
             "E1: after ONE recalculation 璃奈 must hold exactly one copied ライブ成功時");

    for (int round = 1; round <= 5; round++) {
        char msg[220];
        snprintf(msg, sizeof(msg),
                 "E1 round %d: the 常時 re-ran and duplicated the copied ability — "
                 "recalculate_constants runs on EVERY state change, so a "
                 "registration that APPENDS rather than REPLACES shows up here",
                 round);
        rb_recalc_constants(&g.state);
        CHECK_EQ(granted_abilities(&g, rina), 1, msg);
    }
}

/* E2. rina_copied_live_success_places_one_energy_after_repeated_scans */
static void rina_copied_live_success_places_one_energy(void)
{
    static TestGame g;
    new_game(&g);

    int rina, ayumu;
    stage_host_with_source_under(&g, RINA, AYUMU, &rina, &ayumu);
    if (rina < 0 || ayumu < 0) { n_setup_bugs++; return; }
    expect_card_no(rina, RINA);
    expect_card_no(ayumu, AYUMU);
    fill_decks(&g, 40);

    /* p1 behind on energy so 上原歩夢's copied gate 「自分のエネルギーが相手より
     * 少ない場合」 is true, and 4 energy in the energy DECK so a second copy
     * would be visible rather than starved. */
    g.state.p[0].energy.n = 0;
    g.state.p[0].energy_active = 0;
    g.state.p[1].energy.n = 0;
    g.state.p[1].energy_active = 0;
    for (int i = 0; i < 4; i++) {
        int e = test_new_id(&g, ENERGY);
        if (e >= 0) test_add_to_energy_deck(&g, 0, e);
    }
    {
        int e = test_new_id(&g, ENERGY);
        if (e >= 0) {
            g.state.p[1].energy.cards[g.state.p[1].energy.n++] = e;
            rb_energy_set_active_count(&g.state.p[1], 1);
        }
    }
    CHECK(g.state.p[0].energy_deck.n >= 2, "E2: the energy deck must hold a spare card");

    /* Many recalculations: every real phase boundary in the live window runs
     * one, so this is the state the live-success dispatch actually sees. */
    for (int i = 0; i < 5; i++) rb_recalc_constants(&g.state);
    CHECK_EQ(granted_abilities(&g, rina), 1,
             "E2 precondition: exactly one copy must be registered before dispatch");

    int deck_before = g.state.p[0].energy_deck.n;
    int zone_before = g.state.p[0].energy.n;

    /* Arm the window trigger_live_success_abilities dispatches in. Rust's
     * `arm_live_success_window` sets stage_hearts from the live's own printed
     * heart0 requirement (read from the card, not hardcoded, so swapping the
     * live cannot leave the fixture under-arming) and the phase. An
     * UNSATISFIED live FAILED, and then NO ライブ成功時 fires at all (rules
     * 8.3.15-8.3.16), which would make every assertion below read as "the copy
     * did not fire" for a reason unrelated to the copy. */
    {
        int live = test_new_id(&g, DREAM);
        int required = need_heart(&g, live, 0);
        CHECK(required >= 0,
              "E2 precondition: Dream Believers' printed heart0 must be readable");
        g.state.phase = RB_PHASE_VICTORY;
        for (int c = 0; c < 8; c++) g.state.stage_hearts[0][c] = 0;
        g.state.stage_hearts[0][0] = required;   /* index 0 == RB_HEART_ALL */
        g.state.p[0].live.n = 0;
        push_live(&g, 0, live);
    }
    CHECK(rb_should_trigger_live_success(&g.state, 0),
          "E2 precondition: should_trigger_live_success must be true, or no "
          "ライブ成功時 fires at all and 'no energy moved' would be a failed live "
          "rather than a duplicated copy");
    rb_trigger_live_success(&g.state, 0);
    rb_process_pending_auto_abilities(&g.state);
    drain(&g);

    CHECK_EQ(g.state.p[0].energy_deck.n, deck_before - 1,
             "E2: the copied ライブ成功時 places exactly ONE energy card — one "
             "duplicated registration would move two");
    CHECK_EQ(g.state.p[0].energy.n, zone_before + 1,
             "E2: exactly one energy card reached the energy zone");
    CHECK_EQ(active_energy(&g, 0), 0,
             "E2: the placed energy is ウェイト状態, so the active count must not move");
}

/* E3. hazuki_copy_is_withdrawn_when_the_under_area_empties */
static void hazuki_copy_withdrawn_when_under_area_empties(void)
{
    static TestGame g;
    new_game(&g);

    int hazuki, kinoko;
    stage_host_with_source_under(&g, HAZUKI, KINOKO, &hazuki, &kinoko);
    if (hazuki < 0 || kinoko < 0) { n_setup_bugs++; return; }
    expect_card_no(hazuki, HAZUKI);
    expect_card_no(kinoko, KINOKO);
    CHECK(rb_card_matches_group_str(kinoko, "Liella!"),
          "E3 precondition: the copy source is 『Liella!』");
    CHECK_EQ(printed_trigger_count(kinoko, TRIG_KIDOU), 1,
             "E3 precondition: the copy source must print exactly one 起動");

    rb_recalc_constants(&g.state);
    CHECK_EQ(granted_abilities(&g, hazuki), 1,
             "E3 precondition: the 起動 must be copied while the source is under 葉月");

    /* The source leaves the under-area. Route it through the engine's own
     * zone-exit choke point, then run the 常時 again as any state change would. */
    under_retain(&g, 0, kinoko);
    rb_on_cards_left_zones(&g.state, kinoko);
    rb_recalc_constants(&g.state);

    CHECK_EQ(granted_abilities(&g, hazuki), 0,
             "E3: with no source under her the resolved Ability structs must be "
             "gone too — a stale entry lets 葉月 恋 activate a 起動 that is no "
             "longer printed to her");
}

/* E4. hazuki_offers_no_activation_once_the_source_is_gone */
static void hazuki_offers_no_activation_once_the_source_is_gone(void)
{
    static TestGame g;
    new_game(&g);

    int hazuki, kinoko;
    stage_host_with_source_under(&g, HAZUKI, KINOKO, &hazuki, &kinoko);
    if (hazuki < 0 || kinoko < 0) { n_setup_bugs++; return; }
    set_energy(&g, 0, 5);

    rb_recalc_constants(&g.state);
    CHECK(count_use_offers(&g, hazuki) >= 1,
          "E4 CONTROL: with 桜小路きな子 under her, 葉月 MUST be offered the copied "
          "起動, otherwise the zero below proves nothing — this assertion is what "
          "makes the negative non-vacuous");

    under_retain(&g, 0, kinoko);
    rb_on_cards_left_zones(&g.state, kinoko);
    rb_recalc_constants(&g.state);

    CHECK_EQ(count_use_offers(&g, hazuki), 0,
             "E4: with no 『Liella!』 source under her, 葉月 恋 has no abilities to "
             "activate — read through generate_possible_actions, so this is what a "
             "player would actually be offered");
}

/* E5. two_eligible_sources_register_one_copy_each_and_no_more */
static void two_eligible_sources_register_one_copy_each(void)
{
    static TestGame g;
    new_game(&g);

    int rina = test_id(&g, RINA);
    int ayumu = test_id(&g, AYUMU);
    int shizuku = test_id(&g, SHIZUKU);
    int filler = test_id(&g, FILLER);
    CHECK(rina >= 0 && ayumu >= 0 && shizuku >= 0, "E5: fixtures resolve");
    if (rina < 0 || ayumu < 0 || shizuku < 0) { n_setup_bugs++; return; }
    expect_card_no(ayumu, AYUMU);
    expect_card_no(shizuku, SHIZUKU);
    /* Two DIFFERENT 虹ヶ咲 members under the ceiling, each printing its own
     * ライブ成功時, so the expected count of 2 holds for two reasons. */
    CHECK(rb_card_matches_group_str(ayumu, "A・ZU・NA"), "E5: source 1 is 虹ヶ咲");
    CHECK(rb_card_matches_group_str(shizuku, "A・ZU・NA"), "E5: source 2 is 虹ヶ咲");
    {
        Card a, b;
        int distinct = 0;
        if (rb_decode_card_by_index((uint32_t)ayumu, &a) &&
            rb_decode_card_by_index((uint32_t)shizuku, &b)) {
            distinct = (a.name && b.name && strcmp(a.name, b.name) != 0);
            rb_free_card(&a);
            rb_free_card(&b);
        }
        CHECK(distinct, "E5 precondition: the two sources are DIFFERENT characters");
    }
    CHECK_EQ(printed_cost(shizuku), 4, "E5 precondition: 渋谷希 costs 4");
    CHECK_EQ(printed_trigger_count(shizuku, TRIG_LIVE_SUCCESS), 1,
             "E5 precondition: the second source must print exactly one ライブ成功時");

    clear_stage(&g, 0);
    g.state.p[0].stage[0] = rina;
    g.state.p[0].stage[1] = filler;
    g.state.p[0].stage[2] = RB_EMPTY_SLOT;
    under_push(&g, 0, ayumu);
    under_push(&g, 0, shizuku);
    fill_decks(&g, 40);

    for (int round = 0; round < 4; round++) {
        char msg[240];
        snprintf(msg, sizeof(msg),
                 "E5 round %d: one copy per eligible SOURCE, independent of how many "
                 "times the 常時 has run (resolved-Ability half only)", round);
        rb_recalc_constants(&g.state);
        CHECK_EQ(granted_abilities(&g, rina), 2, msg);
    }
}

/* E6. host_zone_exit_clears_the_copy_so_a_returned_host_starts_clean */
static void host_zone_exit_clears_the_copy(void)
{
    static TestGame g;
    new_game(&g);

    int rina, ayumu;
    stage_host_with_source_under(&g, RINA, AYUMU, &rina, &ayumu);
    if (rina < 0 || ayumu < 0) { n_setup_bugs++; return; }
    for (int i = 0; i < 3; i++) rb_recalc_constants(&g.state);
    CHECK_EQ(granted_abilities(&g, rina), 1,
             "E6 precondition: the copy is registered while 璃奈 is on stage");

    /* 璃奈 (and the source travelling with her, per Q139) leave the stage. */
    g.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g.state.p[0].under_cards[0].n = 0;
    rb_on_cards_left_zones(&g.state, rina);

    CHECK_EQ(granted_abilities(&g, rina), 0,
             "E6: a host off stage must hold no resolved Ability structs — a copy "
             "table keyed by card id but never cleaned would read 1 here");

    /* Back on stage with a source under her: exactly one copy again, not one per
     * lifetime pass through the board. */
    int returned = test_new_id(&g, RINA);
    int source_again = test_new_id(&g, AYUMU);
    g.state.p[0].stage[0] = returned;
    under_push(&g, 0, source_again);
    for (int i = 0; i < 3; i++) rb_recalc_constants(&g.state);
    CHECK_EQ(granted_abilities(&g, returned), 1,
             "E6: the returned host must hold exactly one copy, not one per board "
             "it saw");
}

/* E7. a_source_over_the_cost_ceiling_copies_nothing_even_in_the_right_group */
static void a_source_over_the_cost_ceiling_copies_nothing(void)
{
    {
        static TestGame g;
        new_game(&g);
        int rina, over;
        stage_host_with_source_under(&g, RINA, DECK_OVER, &rina, &over);
        if (rina < 0 || over < 0) { n_setup_bugs++; return; }
        expect_card_no(rina, RINA);
        expect_card_no(over, DECK_OVER);
        CHECK(rb_card_matches_group_str(over, "A・ZU・NA"),
              "E7: the source IS in the right group — eligibility by group is not "
              "what excludes it, so the zero below names the ceiling and nothing else");
        CHECK_EQ(printed_cost(over), 13,
                 "E7: the source costs 13, over 天王寺璃奈's コスト9以下 ceiling");

        rb_recalc_constants(&g.state);

        CHECK_EQ(granted_abilities(&g, rina), 0,
                 "E7: a cost-13 『虹ヶ咲』 member copies nothing — an empty list here is "
                 "what distinguishes a working ceiling from a group check that is "
                 "simply too permissive");
    }

    /* PAIRED POSITIVE: an 虹ヶ咲 member INSIDE the ceiling, same copier, same
     * ability, one different card, must copy. Without this the zero above would
     * also read for a copier whose copy never registers anything. */
    {
        static TestGame h;
        new_game(&h);
        int rina2, under_ceiling;
        stage_host_with_source_under(&h, RINA, AYUMU, &rina2, &under_ceiling);
        if (rina2 < 0 || under_ceiling < 0) { n_setup_bugs++; return; }
        expect_card_no(under_ceiling, AYUMU);
        CHECK(printed_cost(under_ceiling) <= 9,
              "E7 positive precondition: 上原歩夢 costs 2, inside the コスト9以下 ceiling");
        rb_recalc_constants(&h.state);
        CHECK_EQ(granted_abilities(&h, rina2), 1,
                 "E7 positive: an UNDER-ceiling 虹ヶ咲 source DOES copy one ライブ成功時, "
                 "so the zero above was the ceiling and not a copier that never "
                 "registers anything");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * runner
 * ══════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_SETUPBUG  3

static int n_ok, n_failed, n_crashed, n_setup;

static void on_segv(int sig)
{
    const char *msg = "\nCRASH: a fatal signal arrived inside the test below.\n";
    ssize_t ignored = write(2, msg, strlen(msg));
    (void)ignored;
    fprintf(stderr, "CRASH: signal %d in test [%s]\n", sig, current_test);
    fflush(stderr);
    _Exit(CHILD_SETUPBUG);
}

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        if (nf) n_failed++; else if (ns) n_setup++; else n_ok++;
        printf("%-8s %s  [%d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name, nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d failure(s), %d setup bug(s)\n",
               name, (int)(failures - f0), (int)(n_setup_bugs - s0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES
                            : (n_setup_bugs > s0 ? CHILD_SETUPBUG : CHILD_OK));
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- fatal signal %d (engine fault)\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok       %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED   %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_setup++;
        printf("SETUPBUG %s  <-- fixture could not be built\n", name);
    } else {
        n_crashed++;
        printf("CRASH    %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS, on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }

    printf("--- \xc2\xa7""A success_zone_conditions_drive_effects_test.rs ---\n");
    run("rin_success_zone_presence_grants_and_withdraws",
        rin_success_zone_presence_grants_and_withdraws);
    run("miki_success_zone_presence_is_group_specific",
        miki_success_zone_presence_is_group_specific);
    run("niko_alt_blade_limit_is_an_equality_not_a_floor",
        niko_alt_blade_limit_is_an_equality_not_a_floor);
    run("niko_alt_needs_both_side_slots", niko_alt_needs_both_side_slots);
    run("song_boost_scales_per_card", song_boost_scales_per_card);
    run("song_boost_ignores_the_opponent_success_zone",
        song_boost_ignores_the_opponent_success_zone);
    run("heartbeat_two_thresholds_partition_the_total",
        heartbeat_two_thresholds_partition_the_total);

    printf("--- \xc2\xa7""B constant_energy_and_slot_conditions_test.rs ---\n");
    run("niko_two_blades_only_at_exactly_seven_energy",
        niko_two_blades_only_at_exactly_seven_energy);
    run("natsumi_three_blades_while_below_the_opponent",
        natsumi_three_blades_while_below_the_opponent);
    run("kakko_costs_two_more_only_when_both_clauses_hold",
        kakko_costs_two_more_only_when_both_clauses_hold);
    run("sayaka_other_group_clause_excludes_herself",
        sayaka_other_group_clause_excludes_herself);
    run("fuyuri_slot_picks_which_hearts_she_grants",
        fuyuri_slot_picks_which_hearts_she_grants);

    printf("--- \xc2\xa7""C Q255 ability-watcher pair (Dancing stars on me!) ---\n");
    run("q255_live_start_repositions_the_resolving_member",
        q255_live_start_repositions_the_resolving_member);
    run("q255_live_start_fires_even_when_already_off_center",
        q255_live_start_fires_even_when_already_off_center);
    run("q255_live_start_non_mus_resolver_does_not_reposition",
        q255_live_start_non_mus_resolver_does_not_reposition);
    run("q255_live_start_opponent_resolution_does_not_arm_our_watcher",
        q255_live_start_opponent_resolution_does_not_arm_our_watcher);
    run("q255_live_start_is_once_per_turn", q255_live_start_is_once_per_turn);
    run("q255_live_success_unmoved_resolver_scores_nothing",
        q255_live_success_unmoved_resolver_scores_nothing);
    run("q255_live_success_moved_before_resolution_scores",
        q255_live_success_moved_before_resolution_scores);
    run("q255_live_success_non_mus_resolver_no_score",
        q255_live_success_non_mus_resolver_no_score);
    run("q255_live_success_is_once_per_turn_for_two_resolvers",
        q255_live_success_is_once_per_turn_for_two_resolvers);
    run("q255_live_success_unrelated_movement_does_not_score",
        q255_live_success_unrelated_movement_does_not_score);
    run("q255_live_success_turn_limit_resets_on_next_turn",
        q255_live_success_turn_limit_resets_on_next_turn);
    run("q255_live_success_after_center_movement_gains_score",
        q255_live_success_after_center_movement_gains_score);

    printf("--- \xc2\xa7""D live_start_success_resolved_watcher_arms_only_after_resolution ---\n");
    run("resolution_watcher_dormant_without_a_resolution_event",
        resolution_watcher_dormant_without_a_resolution_event);

    printf("--- \xc2\xa7""E copied_ability_registration_lifecycle_test.rs ---\n");
    run("rina_registers_one_copy_per_recalculation",
        rina_registers_one_copy_per_recalculation);
    run("rina_copied_live_success_places_one_energy",
        rina_copied_live_success_places_one_energy);
    run("hazuki_copy_withdrawn_when_under_area_empties",
        hazuki_copy_withdrawn_when_under_area_empties);
    run("hazuki_offers_no_activation_once_the_source_is_gone",
        hazuki_offers_no_activation_once_the_source_is_gone);
    run("two_eligible_sources_register_one_copy_each",
        two_eligible_sources_register_one_copy_each);
    run("host_zone_exit_clears_the_copy", host_zone_exit_clears_the_copy);
    run("a_source_over_the_cost_ceiling_copies_nothing",
        a_source_over_the_cost_ceiling_copies_nothing);

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_ability_watch: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    if (n_crashed)
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child. "
                        "Treat as an engine fault.\n", n_crashed);
    if (n_setup)
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n", n_setup);
    if (n_failed)
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file is "
                        "classified in the accompanying report; none is a weakened "
                        "assertion.\n", n_failed);
    return n_failed ? 1 : 0;
}

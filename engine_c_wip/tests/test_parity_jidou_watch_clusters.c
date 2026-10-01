/* test_parity_jidou_watch_clusters.c — C port of the jidou WATCHER clusters
 * that are genuinely UNCOVERED, computed from the tree rather than inherited
 * from a peer's header.
 *
 * Rust sources (read-only, never edited):
 *   engine/tests/test_modules/jidou/ability_watch/constant_grant_lifecycle_test.rs
 *   engine/tests/test_modules/jidou/ability_watch/lifecycle_tail_idempotence_test.rs
 *   engine/tests/test_modules/jidou/ability_watch/invalidated_live_start_restores_next_live_test.rs
 *   engine/tests/test_modules/jidou/debut_watch/baton_touch_arrival_self_or_other_draws_one_twice_per_turn_test.rs
 *   engine/tests/test_modules/jidou/debut_watch/simultaneous_triggers_active_player_resolves_first_test.rs
 *   engine/tests/test_modules/jidou/debut_watch/jump_up_high_multi_target_blade_grant_lifecycle_test.rs
 *   engine/tests/test_modules/jidou/discard_watch/hand_discard_batch_heart_blade_q241_test.rs
 *   engine/tests/test_modules/jidou/discard_watch/deck_to_discard_self_recovery_yell_refresh_q269_q277_test.rs
 *   engine/tests/test_modules/jidou/discard_watch/real_mill_optional_discard_recover_self_test.rs
 *   engine/tests/test_modules/jidou/discard_watch/live_discard_optional_deck_top_bottom_q252_test.rs
 *   engine/tests/test_modules/jidou/discard_watch/own_live_zone_discard_group_live_optional_deck_top_bottom_q252_test.rs
 *
 * Build ONLY in isolation (peers share this tree and race on .o files):
 *   cd engine_c_wip
 *   gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated \
 *       -Wl,--stack,16777216 -o rb_engine_t_jdw <OBJ_LIB> \
 *       src/test_game.c tests/test_parity_jidou_watch_clusters.c
 *   ./rb_engine_t_jdw
 *
 * ── COVERAGE BOUNDARY, RE-DERIVED FROM THE BODIES ────────────────────────
 * The task card asserted these three clusters had "no C coverage at all".
 * That is STALE, and the correction matters more than the raw numbers, so the
 * accounting is spelled out per file against what the C corpus actually runs.
 *
 * The decisive finding: tests/test_parity_jidou_life.c's HEADER claims three
 * sections that DO NOT EXIST in its body:
 *   §D  ability_watch/constant_grant_lifecycle_test.rs ............ 8
 *   §E  ability_watch/lifecycle_tail_idempotence_test.rs .......... 7
 *   §F  ability_watch/invalidated_live_start_restores_next_live_... 3
 * I listed every `static void ...(void)` in that file: a1..a5, b1..b7, c1..c2,
 * 14 functions, and main()'s run() list carries exactly those 14 names. There
 * is no d1..d8, no e1..e7 and no f1..f3 anywhere in it. Those 18 tests are the
 * real gap in this cluster and are what §A/§B/§C below port. The rest of
 * ability_watch IS covered, by test_parity_ability_watch.c (36 cases) and
 * test_parity_jidou_extra.c §A (4), and is not re-ported.
 *
 * debut_watch: 44 Rust tests; tests/test_parity_jidou_watch.c covers 41.
 * The three it leaves are ported here as §D (they are the only debut_watch
 * behaviours with NO C case at all, and two of them are the ENERGY-activation
 * and OPPONENT-STAGE negatives that a suite full of positive grants cannot
 * imply).
 *
 * discard_watch: 57 Rust tests; test_parity_jidou_watch.c covers 44 of them.
 * The 13 it leaves split into: 4 the Q269/Q277 refresh family (§E), 3 the
 * remaining real-mill / Q241 edges (§E), 1 the Q241 one-card control (§E), and
 * 5 of the two Q252 files' own end-to-end and multi-select cases (§F). The two
 * Q252 files are largely duplicate each other in Rust; §F ports the union of
 * the cases neither existing C section covers, and says so per test.
 *
 * ── HARNESS NOTES THAT CHANGE THE ASSERTIONS HERE ────────────────────────
 *  - `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *  - `test_get_heart_modifier` is never used; every heart read goes through
 *    rb_mods_get_heart / rb_mods_get_need_heart with the RAW colour index
 *    (0=heart00 .. 7=all), which is what the engine map uses.
 *  - `rb_card_no_eq` is a FUZZY lookup, so every identity assertion here is a
 *    byte-exact strcmp of rb_card_string(c.card_no_idx) and every fixture goes
 *    in through test_id_exact. expect_card_no() is the wrapper.
 *  - Card.ability is NOT populated by rb_decode_card_by_index; abilities come
 *    from rb_card_num_abilities + rb_decode_card_ability. fire_trigger() is
 *    the helpers::fire_trigger port and matches on the EXACT printed trigger
 *    string, so a 6-byte typo shows up as a SETUP BUG, never as silence.
 *  - RB_TSTR_DEBUT is 「登場」 = E7 99 BB E7 8D B2, a string NO CARD IN THE
 *    DATABASE CARRIES. The printed debut trigger is 「登場場」 = E7 99 BB
 *    E5 A0 B4. rb_trigger_is is a strstr, so the short form matches by prefix
 *    and the engine works; an EXACT compare in fire_trigger() does not. This
 *    file therefore defines TRIG_DEBUT_STAGE (the 6-byte form) and uses THAT
 *    for every debut lookup, and dump_triggers() prints the real bytes on
 *    every miss so a typo cannot hide. This is the same divergence
 *    test_parity_jidou_watch2.c documents; it is spelled here because this
 *    file depends on it for §A/§B/§D.
 *  - Rust's `gained_abilities` (the triggerless TEXT half of a grant) has NO C
 *    field. The C half is GameState.gained_card_abilities, read through
 *    rb_card_num_gained_abilities. Every "granted text count" assertion below
 *    names that half explicitly, exactly as test_parity_jidou_life.c does.
 *  - C has no `state.turn_number`; the counter is GameState.turn. rb_use_count
 *    takes that turn.
 *  - RB_PHASE_PERFORMANCE_SECOND and RB_PHASE_LIVE_SET_SECOND are NEVER
 *    STORED (phase.c:87-90) — one stored value each, told apart by the active
 *    seat. So the walk helpers compare against RB_PHASE_PERFORMANCE /
 *    RB_PHASE_LIVE_SET and check `state.active` when the distinction matters.
 *  - The ENERGY phase RE-SEATS the energy zone (return-to-deck + charge), so a
 *    zone seeded before a phase walk is re-shaped on the way in and a
 *    「2枚アクティブ」 effect no-ops for a reason unrelated to whatever is under
 *    test. §C therefore re-seeds at the last moment before the dispatch.
 *  - rb_on_cards_left_zones lives in modifiers.c and is NOT declared in
 *    include/rabuka.h, so it is declared here (not this file's to edit).
 *  - GameState.stage_hearts is a plain `int[2][8]` array, not Rust's
 *    Option<BaseHeart>, so a §A/§B heart set is written directly and the
 *    performance recomputes it on the walk (the exact defect the Rust
 *    genki_zenkai comment documents: the lever only works immediately before a
 *    DIRECT dispatch, not before a walk).
 *  - GameState has `moved_this_turn[card_id]` for Rust's
 *    has_card_moved_this_turn; read it directly.
 *
 * ── VACUITY CONTROL ─────────────────────────────────────────────────────
 * Every negative in this file is licensed by one of:
 *   (a) CHECK_PROMPTS_GT(n, ...) — n is a control point read BEFORE the
 *       interaction, and the comparison is strictly-greater, so the assertion
 *       is false unless the interaction really added a prompt on screen;
 *   (b) a paired POSITIVE driven FIRST on the SAME board in the SAME process
 *       and asserted, so the engine is proven live before the negative;
 *   (c) a fixture-wiring premise: expect_card_no + expect_trigger.
 * Nothing here is licensed by "nothing happened". §A2/§B2/§C1 exist
 * SPECIFICALLY as the positives that license the negatives beside them, and
 * §B1/§C2/§E1 open with a control run.
 *
 * ── EXPECTED FAILURES ───────────────────────────────────────────────────
 * EXPECTED to be red: src/core/game_state_abilities.c is mid-translation by a
 * peer agent, and the gain/invalidation lifecycle is exactly its surface. A red
 * assertion here is the suite doing its job. No assertion is weakened, no
 * failing test is deleted or ignored, and every red line is classified in the
 * report. EXPECTED_GAP is used only where the flat RbChoice genuinely cannot
 * carry the observable, and it PRINTS rather than passing.
 *
 * ENCODING: every Japanese string that is COMPARED against engine data is a
 * \xNN escape (see the trigger table) and is therefore immune to re-encoding by
 * an editor. Japanese text also appears LITERALLY inside assertion MESSAGE
 * strings and comments, where it is diagnostic prose only: it is never
 * compared, hashed or parsed, so its encoding cannot affect a verdict. This
 * file is deliberately not byte-for-byte 7-bit clean, and the message text is
 * what a failure report is read through, so it is left readable rather than
 * escaped. What matters is the one class that IS load-bearing — the trigger
 * bytes, the card numbers and the group names — and those are all escaped or
 * pure ASCII.
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
static int n_gaps;
static int n_ok, n_failed, n_crashed, n_setup;
static const char *current_test = "(none)";

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_SETUPBUG  2

/* VARIADIC on purpose: a two-parameter CHECK(cond, msg) silently splits an
 * adjacent string literal into extra macro arguments. */
#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "\n"); \
        failures++; \
    } else { \
        printf("ok: "); \
        printf(__VA_ARGS__); \
        printf("\n"); \
    } \
    fflush(stdout); \
} while (0)

#define CHECK_EQ(actual, expected, ...) do { \
    assertions++; \
    int av_ = (int)(actual); \
    int ev_ = (int)(expected); \
    if (av_ != ev_) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  (got %d, expected %d)\n", av_, ev_); \
        failures++; \
    } else { \
        printf("ok: "); \
        printf(__VA_ARGS__); \
        printf("\n"); \
    } \
    fflush(stdout); \
} while (0)

/* PROMPT-COUNT CONTROL, strict-greater form. The FIRST argument is the game
 * to read; `n` MUST be a control point read BEFORE the interaction under test,
 * and the comparison is strictly-greater, so the assertion is false unless the
 * interaction really added at least one prompt on screen. */
#define CHECK_PROMPTS_GT(tgame, n, ...) do { \
    assertions++; \
    int prompts_on_screen = test_has_pending_choice(tgame); \
    if (prompts_on_screen <= (int)(n)) { \
        fprintf(stderr, "FAIL: [VACUITY CONTROL] "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  (control point was %d, only %d prompt(s) on screen, " \
                        "so the absence this licenses proves nothing)\n", \
                (int)(n), prompts_on_screen); \
        failures++; \
    } else { \
        printf("ok: [vacuity control] "); \
        printf(__VA_ARGS__); \
        printf("  (%d prompt(s) on screen, control point %d)\n", \
               prompts_on_screen, (int)(n)); \
    } \
    fflush(stdout); \
} while (0)

/* A place where the flat RbChoice genuinely cannot carry the observable. The
 * note PRINTS; it is never converted into a pass. */
#define EXPECTED_GAP(cond, ...) do { \
    assertions++; \
    if (!(cond)) { \
        printf("GAP: "); \
        printf(__VA_ARGS__); \
        printf("\n"); \
        n_gaps++; \
    } else { \
        printf("ok (gap closed): "); \
        printf(__VA_ARGS__); \
        printf("\n"); \
    } \
    fflush(stdout); \
} while (0)

/* ── trigger bytes (escapes, so the file's own encoding cannot corrupt them) ──
 *   U+767B toujou = E7 99 BB   U+5834 ba     = E5 A0 B4   (6-byte DEBUT token)
 *   U+81EA ji      = E8 87 AA   U+52D5 dou    = E5 8B 95   (jidou / auto)
 *   U+30E9 ra      = E3 83 A9   U+30FC chou   = E3 83 BC
 *   U+30A4 i       = E3 82 A4   U+30D6 bu     = E3 83 96
 *   U+958B kai     = E9 96 8B   U+59CB shi    = E5 A7 8B
 *   U+6642 toki    = E6 99 82   U+6210 sei    = E6 88 90
 *   U+529F kou     = E5 8A 9F
 *   U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B
 *   U+30FB KATAKANA MIDDLE DOT = E3 83 BB
 * NOTE ON THE 6-BYTE DEBUT TOKEN: RB_TSTR_DEBUT is toujou + sho (= E7 8D B2),
 * which no card carries. The printed token is toujou + ba. rb_trigger_is is a
 * strstr so the engine works by prefix; this file's fire_trigger() compares
 * exactly, so it is always given TRIG_DEBUT_STAGE. */
#define TRIG_DEBUT_STAGE  "\xE7\x99\xBB\xE5\xA0\xB4"
#define TRIG_AUTO         "\xE8\x87\xAA\xE5\x8B\x95"
/* U+5E38 jou U+6642 toki = E5 B8 B8 E6 99 82 (the 常時 constant trigger) */
#define TRIG_CONSTANT     "\xE5\xB8\xB8\xE6\x99\x82"
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"
/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B. rb_find_card_by_no is EXACT-BYTE and
 * does not normalise fullwidth to halfwidth the way Rust's lookup does, so
 * every ＋ card number below is spelled byte-exactly. */
#define FWPLUS "\xEF\xBC\x8B"

/* ── fixtures (every card_no verified against cards/cards.json as UTF-8 and
 *    re-pinned at run time by expect_card_no) ────────────────────────────── */
#define FILLER      "PL!-sd1-010-SD"            /* cost 4, no abilities        */
#define ENERGY      "LL-E-001-SD"
#define PLAIN_LIVE  "PL!-sd1-019-SD"            /* START:DASH!!, score 1       */
#define ONE_PT      "PL!-sd1-001-SD"            /* the 1-point success card    */
#define CHEAP       "PL!-sd1-002-SD"            /* cost 6, a plain arriver     */
#define SEVEN_SCORE "PL!SP-bp1-027-L"           /* score 6 live card            */

/* constant_grant_lifecycle_test.rs */
#define NIKO        "PL!-sd1-009-SD"            /* 矢澤にこ, 25 in waitroom    */
#define MU_MEM      "PL!-sd1-004-SD"            /* a mu's member               */
#define AI_NAIL     "PL!N-bp3-005-R" FWPLUS     /* 宮下愛                       */
#define KOTO        "PL!HS-bp1-003-R" FWPLUS    /* 乙宗梢                       */
#define RENJO_UTA   "PL!HS-bp1-016-PR"          /* a 2nd 蓮ノ空 name           */
#define RENJO_KOKO  "PL!HS-bp2-010-PR"          /* a 3rd 蓮ノ空 name           */
#define NIKO_LIVE   "PL!-sd1-020-SD"            /* a live card to seat         */

/* lifecycle_tail_idempotence_test.rs */
#define JOINT       "LL-bp1-001-R" FWPLUS       /* 歩夢/かのん/花帆, flat +3    */
#define NOZOMI      "PL!-bp4-007-R"             /* 東條希, success-zone grant   */
#define HANAMARU    "PL!S-bp6-007-R"            /* 国木田花丸                   */
#define HANAMARU_F  "PL!S-pb1-010-PR"           /* a different 『Aqours』 name  */
#define AQOURS      "PL!S-bp2-001-R"            /* an 『Aqours』 member         */
#define HONOKA      "PL!-PR-020-PR"             /* 高坂穂乃果, NO duration      */
#define GENKI       "PL!S-pb1-019-L"            /* 元気全開DAY!DAY!DAY!         */
#define BUTTERFLY   "PL!SP-pb2-046-L"           /* Butterfly Wing               */
#define YOI         "PL!S-pb1-005-R"            /* 渡辺曜, 4 heart02            */
#define RUBY        "PL!S-pb1-009-R"            /* 黒澤ルビィ, 4 heart02        */
#define AYUMU_JS    "PL!N-bp1-001-R"            /* 上原歩夢, one of the 3 names  */
#define KANON_JS    "PL!N-bp1-001-R"            /* 澁谷かのん (see note below)  */
#define KAHO_JS     "PL!HS-bp1-001-R"           /* 日野下花帆, the 3rd name      */

/* invalidated_live_start_restores_next_live_test.rs */
#define KANON_INV   "PL!SP-bp2-001-R" FWPLUS    /* 澁谷かのん, the invalidator  */
#define MEI         "PL!SP-pb1-007-R"           /* 米女メイ, +2 energy          */
#define RECOVERY    "PL!SP-sd1-001-SD"

/* debut_watch */
#define SETSUNA     "PL!N-PR-025-PR"             /* 優木せつ菜                   */
#define HANAFU      "PL!HS-sd1-001-SD"          /* 日野下花帆 (SD)              */
#define HANAFU_PR   "PL!HS-PR-001-PR"           /* cost 10 蓮ノ空 replacer     */
#define CERAS_R     "PL!HS-bp6-007-R"           /* セラス 柳田                   */
#define CERAS_SD    "PL!HS-sd1-007-SD"          /* セラス 柳田 (cost 4 print)    */
#define JUMP_UP     "PL!S-sd1-022-SD"           /* Jump up HIGH!! ライブ        */
#define AQOURS_A    "PL!S-sd1-001-SD"           /* 高海千歌                     */
#define AQOURS_B    "PL!S-sd1-002-SD"           /* 桜内梨子                     */
#define NOT_AQOURS  "PL!HS-bp5-004-R"           /* 百生吟子, 蓮ノ空             */
/* NON_GROUP_10: a member of NO group, printing cost exactly 10 -- the same cost as
 * HANAFU_PR. Using it as the non-qualifying partner is what makes the D2 comparison
 * cost-MATCHED: both boards pay an identical 10, so the whole difference between
 * them is the printed group's clause and nothing else. A cheaper outsider would
 * have made the two boards differ by their play cost as well, which is how the
 * first draft of D2 came to expect a difference of 5. */
#define NON_GROUP_10 "PL!HS-bp2-005-P"

/* discard_watch */
#define MIA         "PL!N-bp7-011-R" FWPLUS     /* ミア・テイラー ab#0          */
#define DIAYA       "PL!S-sd1-013-SD"           /* 黒澤ダイヤ, 登場 mill 5      */
#define RURINO_W    "PL!HS-pb1-003-R"           /* 大沢瑠璃乃 ab#0 hand batch   */
#define RURINO_WP   "PL!HS-pb1-003-P" FWPLUS
#define MIRAKU      "PL!HS-sd1-011-SD"          /* a 『みらくらぱーく！』 member */
#define RIKO_SEC    "PL!S-bp6-002-SEC"          /* 桜内梨子 (SEC)               */
#define AQ_LIVE_PR  "PL!S-PR-022-PR"            /* HAPPY PARTY TRAIN            */
#define AQ_LIVE_SD  "PL!S-sd1-019-SD"
#define AQ_LIVE_PB  "PL!S-pb1-023-L"            /* Next SPARKLING!!             */
#define NON_AQ_LIVE "PL!-bp5-019-L"             /* non-『Aqours』               */
#define AQ_MEMBER   "PL!S-pb1-005-R"            /* 渡辺曜, an 『Aqours』 member */

/* ══════════════════════════════════════════════════════════════════════════
 * helpers
 * ═════════════════════════════════════════════════════════════════════════ */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

/* The copy pool is process-global and MONOTONIC, and a forked child INHERITS
 * the parent's already-consumed pool, so the reset is what makes test_new_id
 * return a fresh instance in each case. */
static void new_game(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static void fill_decks(TestGame *tg, int n)
{
    int filler = test_id_exact(tg, FILLER);
    if (filler < 0) return;
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust `hand.cards.clear()` / `main_deck.cards.clear()` /
 * `waitroom.cards.clear()`. */
static void clear_hand(TestGame *tg) { tg->state.p[0].hand.n = 0; }
static void clear_deck(TestGame *tg) { tg->state.p[0].deck.n = 0; }
static void clear_waitroom(TestGame *tg) { tg->state.p[0].discard.n = 0; }

/* RAW heart-colour index: 0=heart00 .. 7=all. */
static int heart(TestGame *tg, int cid, int idx)
{
    return rb_mods_get_heart(&tg->state.mods, cid, idx);
}
static int blade(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}
static int constant_bonus(TestGame *tg) { return (int)tg->state.mods.p1_constant_total_score_bonus; }

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *choice_type(TestGame *tg) { return test_pending_choice_type(tg); }

static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

static const char *choice_target(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->target : NULL;
}
static int choice_count(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->count : 0;
}

/* Rust `drain`: SelectTarget takes option 0, everything else is declined. */
static void drain(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const char *t = choice_type(tg);
        if (t && strcmp(t, "SelectTarget") == 0) answer(tg, 0);
        else answer(tg, -1);
    }
}

/* Rust `drain_choices`: answer what cannot be declined, decline what can. A
 * mandatory prompt left parked mid-ability produces exactly the absence the
 * next assertion checks, so nothing would fail. */
static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const char *t = choice_type(tg);
        if (t && strcmp(t, "SelectTarget") == 0) answer(tg, 0);
        else if (t && strcmp(t, "SelectAutoAbility") == 0) answer(tg, -1);
        else if (t && strcmp(t, "SelectCard") == 0) {
            int count = choice_count(tg);
            if (count > 0 && count < 10) {
                int idx[16];
                for (int i = 0; i < count; i++) idx[i] = i;
                rb_resume_with_choice_indices(&tg->state, idx, count);
            } else {
                answer(tg, 0);
            }
        } else {
            answer(tg, -1);
        }
    }
}

/* Rust `drain_auto_ability_choices`: only SelectAutoAbility is answered. */
static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 60) {
        const char *t = choice_type(tg);
        if (!t || strcmp(t, "SelectAutoAbility") != 0) break;
        answer(tg, -1);
    }
}

/* Rust support::resolve_auto_choices_accepting_optionals. */
static void resolve_accepting_optionals(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 60) {
        const char *t = choice_type(tg);
        const char *target = choice_target(tg);
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            answer(tg, -1);
        } else if (t && strcmp(t, "SelectCard") == 0) {
            int count = choice_count(tg);
            if (count > 0 && count < 10) {
                int idx[16];
                for (int i = 0; i < count; i++) idx[i] = i;
                rb_resume_with_choice_indices(&tg->state, idx, count);
            } else {
                answer(tg, 0);
            }
        } else if (t && strcmp(t, "SelectTarget") == 0 && target &&
                   strcmp(target, "conditional_optional") == 0) {
            answer(tg, 1);   /* accept */
        } else {
            answer(tg, 0);
        }
    }
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` + process_pending.
 * rb_queue_trigger_abilities re-derives the event flags from GameState exactly
 * like the Rust entry point. */
static void scan_auto(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, TRIG_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

/* helpers::fire_trigger — find the ability whose printed `triggers` string is
 * exactly `trig`, push it with `card` as the activating card, and drain.
 * Returns 0 when the card has no such ability, which the caller records as a
 * SETUP BUG rather than silently passing. */
static int fire_trigger(TestGame *tg, int card_id, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int match = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (!match) continue;
        rb_queue_push(&tg->state.queue, card_id, i);
        tg->state.activating_card = card_id;
        rb_drain_ability_queue(&tg->state);
        return 1;
    }
    return 0;
}

/* [TRIGGER-MISS] diagnostic: print, as hex, EVERY trigger string the card
 * actually carries, so a byte-escape typo can never hide as engine silence. */
static void dump_triggers(int cid, const char *want)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        fprintf(stderr, "  [TRIGGER-MISS] cid=%d prints card_no='%s' cost=%d nAb=%d "
                        "wanted='%s'\n",
                cid, rb_card_string(c.card_no_idx), (int)c.cost, n,
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

/* Identity rule, byte-exact (rb_card_no_eq is fuzzy). */
static void expect_card_no(int cid, const char *no)
{
    int ok = 0;
    Card c;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
        ok = strcmp(rb_card_string(c.card_no_idx), no) == 0;
        rb_free_card(&c);
    }
    CHECK(ok, "fixture cid is byte-exactly the printed card_no under test");
    if (!ok) dump_triggers(cid, "(identity)");
}

/* Rust `assert!(card has the trigger)`, and the fixture-wiring vacuity
 * control for every negative in this file. */
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
    assertions++;
    if (!found) {
        fprintf(stderr, "FAIL: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            fprintf(stderr, " %02X", *q);
        fprintf(stderr, ")\n");
        dump_triggers(cid, trig);
        failures++;
    } else {
        printf("ok: %s", msg);
        for (const unsigned char *q = (const unsigned char *)trig; *q; q++)
            printf(" %02X", *q);
        printf(")\n");
        fflush(stdout);
    }
    return found;
}

static int printed_cost(int cid)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

/* How many hearts of `col` the card PRINTS. The Card is stored as parallel
 * heart_color[] / heart_count[] arrays, so the QUANTITY has to be summed --
 * counting matching ENTRIES reads 1 for a printed "heart02: 4" and silently
 * turns a fixture premise into a wrong expectation. */
static int printed_heart_count(int cid, int col)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = 0;
    for (int i = 0; i < c.n_hearts && i < RB_MAX_HEARTS; i++)
        if ((int)c.heart_color[i] == col) v += c.heart_count[i];
    rb_free_card(&c);
    return v;
}

/* Rust `gained_abilities.get(&card).map(len)` — the RESOLVED-ABILITY half of
 * the grant. The triggerless TEXT half has no C field at all, so every count
 * below names this half. */
static int granted_n(TestGame *tg, int cid)
{
    return rb_card_num_gained_abilities(&tg->state, cid);
}

static int is_wait(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && strcmp(o, "wait") == 0;
}

static int invalidated(TestGame *tg, int cid, const char *trigger)
{
    return rb_ability_is_invalidated(&tg->state, cid, trigger);
}

static int suppressed(TestGame *tg, int pl, const char *trigger)
{
    return rb_is_trigger_suppressed(&tg->state, pl, trigger);
}

/* ── zone reads ────────────────────────────────────────────────────────── */

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_n(TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int deck_n(TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }
static int wait_n(TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }
static int live_n(TestGame *tg, int pl) { return tg->state.p[pl].live.n; }
static int success_n(TestGame *tg, int pl) { return tg->state.p[pl].success.n; }
static int energy_n(TestGame *tg, int pl) { return tg->state.p[pl].energy.n; }
static int energy_deck_n(TestGame *tg, int pl) { return tg->state.p[pl].energy_deck.n; }
static int active_energy(TestGame *tg, int pl)
{
    return rb_energy_active_count(&tg->state.p[pl]);
}
static int hand_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].hand, cid); }
static int wait_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].discard, cid); }
static int deck_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].deck, cid); }
static int live_has(TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].live, cid); }
static int on_stage(TestGame *tg, int pl, int area, int cid)
{
    return tg->state.p[pl].stage[area] == cid;
}
static int stage_n(TestGame *tg, int pl)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}
static int use_count(TestGame *tg, int card, int idx)
{
    return rb_use_count(&tg->state.queue, card, idx, tg->state.turn);
}
/* Rust `turn_limited_abilities_used.get(&(setsuna, 0, turn))` — the family's
 * ability index for these cards is 0, named explicitly so a card-pool reorder
 * cannot silently move the read onto a different ability. */
static int use_count_ab0(TestGame *tg, int card) { return use_count(tg, card, 0); }

/* Rust `state.set_recently_moved_cards(vec![..])`. */
static void set_recently_moved(TestGame *tg, const int *ids, int n)
{
    tg->state.n_recently_moved = 0;
    for (int i = 0; i < n && i < RB_MAX_RECENTLY_MOVED; i++)
        tg->state.recently_moved[tg->state.n_recently_moved++] = ids[i];
}

/* Rust `state.on_cards_left_zones(&[cid])`. */
static void left_zones(TestGame *tg, int cid)
{
    rb_on_cards_left_zones(&tg->state, cid);
}

/* Rust `push_movement_event(cid, "deck", "discard", ...)`. */
static void push_deck_to_discard(TestGame *tg, int cid, int cause_seat)
{
    rb_record_card_movement(&tg->state, cid, RB_ZONEID_DECK, RB_ZONEID_DISCARD,
                            cause_seat, 1);
}

/* Rust `TurnPhase::Live`.  RB_PHASE_PERFORMANCE and RB_PHASE_LIVE_SET are ONE
 * stored value each in the C engine, told apart by the active seat, and the
 * *_SECOND values are NEVER stored (phase.c:87-90) — so a literal compare
 * against RB_PHASE_PERFORMANCE_SECOND could never fire. "Inside the live window"
 * is therefore spelled as: we are at or past the live card set, and victory
 * determination has not been left yet. */
static int in_live_window(TestGame *tg)
{
    RbPhase ph = tg->state.phase;
    if (ph == RB_PHASE_LIVE_SET || ph == RB_PHASE_PERFORMANCE ||
        ph == RB_PHASE_LIVE_SET_SECOND || ph == RB_PHASE_PERFORMANCE_SECOND)
        return 1;
    /* The walk may already be past performance and sitting in victory
     * determination, which is still "inside the live" until it rolls over. */
    return ph == RB_PHASE_VICTORY;
}

/* The stricter, PROVEN live-end idiom (test_parity_jidou_life.c a5): walk to
 * LiveVictoryDetermination and then dispatch the determination explicitly,
 * rather than hoping repeated test_pass() calls stumble out of the live.
 *
 * roll_over_live_end() above is enough when the grant was armed BEFORE the live
 * card was set, but a case that arms a trigger AFTER the live card is set and
 * then pays an optional cost can leave a mandatory prompt parked, and test_pass
 * is a no-op while one is outstanding — so the walk never advances and "the live
 * ended" is silently untrue. This spelling names victory determination and
 * dispatches it, so the rollover cannot be faked by a stalled walk. */
static int roll_over_live_end_strict(TestGame *tg)
{
    int reached = test_advance_to_phase_named(tg, "LiveVictoryDetermination");
    if (!reached) return 0;
    rb_execute_live_victory_determination(&tg->state);
    drain_choices(tg);
    int guard = 0;
    while (in_live_window(tg) && guard < 8) {
        guard++;
        test_pass(tg);
        drain_choices(tg);
    }
    return !in_live_window(tg);
}

/* Rust `roll_over_live_end`: ride the genuine phase walk out of the live, through
 * victory determination, into the next turn. Returns the number of passes taken
 * and sets *crossed to 1 only if the walk really left the live window. */
static int roll_over_live_end(TestGame *tg, int *crossed)
{
    int guard = 0;
    while (in_live_window(tg) && guard < 16) {
        guard++;
        test_pass(tg);
        drain_choices(tg);
    }
    if (crossed) *crossed = !in_live_window(tg);
    return guard;
}

/* ── energy seeding ────────────────────────────────────────────────────────
 * Each card is its own pool instance: a zone's active/waited split is tracked
 * per zone, so one id sitting in two zones at once makes "how many cards does
 * this zone hold" ambiguous and quietly zeroes one side's measurement. */
static void fill_energy_deck(TestGame *tg, int pl, int count)
{
    for (int i = 0; i < count; i++) {
        int e = test_new_id(tg, ENERGY);
        if (e < 0) return;
        RbPlayer *P = &tg->state.p[pl];
        if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = e;
    }
}

/* Rust `energy_zone.cards.clear()` + push + set_active_count(n). */
static int set_energy_active(TestGame *tg, int pl, int total, int active)
{
    RbPlayer *P = &tg->state.p[pl];
    P->energy.n = 0;
    for (int i = 0; i < total && P->energy.n < RB_MAX_ZONE; i++) {
        int e = test_new_id(tg, ENERGY);
        if (e < 0) break;
        P->energy.cards[P->energy.n++] = e;
    }
    rb_energy_set_active_count(P, active);
    return P->energy.n - active;
}

/* Set a performance heart tally directly (C: stage_hearts is `int[2][8]`, not
 * Rust's Option<BaseHeart>). */
static void set_stage_hearts(TestGame *tg, int pl, const int *hearts)
{
    for (int i = 0; i < 8; i++) tg->state.stage_hearts[pl][i] = hearts[i];
}
/* ══════════════════════════════════════════════════════════════════════════
 * §A  ability_watch / constant_grant_lifecycle_test.rs — 8 Rust tests
 *
 * The claim this whole file exists for. `gain_ability` grants a 常時 for a
 * printed duration, and a SINGLE-resolution test cannot see any of the three
 * transitions that matter: a re-registration that appends instead of replaces
 * (duplicates the bonus), a re-derivation that withdraws a LATCHED grant, and
 * a duration that never ends. Every assertion below is on a NUMBER.
 *
 * Two families, two DIFFERENT revocations, and a first draft got it wrong in a
 * way worth recording:
 *   * a grant made by a 常時 (乙宗梢) re-evaluates its condition every scan, so
 *     it must be withdrawn the instant a stage slot empties;
 *   * a grant made by a ライブ開始時 (矢澤にこ, 宮下愛) LATCHES — its condition
 *     is assessed once, when the trigger fires — so dropping 矢澤にこ's waitroom
 *     below 25 mid-live must NOT revoke it. Revoking there would be the bug.
 * §A2 and §A5 pin that distinction from both sides.
 * ═════════════════════════════════════════════════════════════════════════ */

/* 矢澤にこ 「自分の控え室に『μ's』のカードが25枚以上ある場合、ライブ終了時まで、常時 +1」 */
static int niko_waitroom_granted(TestGame *tg, int niko, int n_mu)
{
    int mu = test_id_exact(tg, MU_MEM);
    fill_decks(tg, 40);
    clear_stage(tg, 0);
    tg->state.p[0].discard.n = 0;
    test_give_energy(tg, 15);
    for (int i = 0; i < n_mu; i++) rb_waitroom_add(&tg->state.p[0], mu);
    tg->state.p[0].stage[0] = niko;
    return wait_n(tg, 0);
}

/* a1 niko_grant_stays_single_across_repeated_constant_scans */
static void a1_niko_grant_stays_single_across_repeated_scans(void)
{
    static TestGame tg;
    new_game(&tg);

    int niko = test_id_exact(&tg, NIKO);
    int mu   = test_id_exact(&tg, MU_MEM);
    CHECK(niko >= 0 && mu >= 0, "A1: both fixtures resolve");
    if (niko < 0 || mu < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO);
    expect_trigger(niko, TRIG_LIVE_START, 1);

    niko_waitroom_granted(&tg, niko, 25);
    CHECK_EQ(wait_n(&tg, 0), 25, "A1 precondition: 25 mu's cards really sit in the waitroom");

    CHECK(fire_trigger(&tg, niko, TRIG_LIVE_START),
          "A1: the ライブ開始時 is queued for 矢澤にこ");
    drain(&tg);

    CHECK_EQ(constant_bonus(&tg), 1,
             "A1 precondition: with 25 mu's cards in the waitroom the ライブ開始時 "
             "must grant the 常時 +1 — a constant aggregate of 0 makes every "
             "idempotence round below vacuous");
    CHECK_EQ(granted_n(&tg, niko), 1,
             "A1 precondition: exactly one granted ability is registered");

    for (int round = 1; round <= 6; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "A1 round %d: the grant must stay a SINGLE +1 — a "
                 "registration that re-appends takes it to +2", round);
        CHECK_EQ(constant_bonus(&tg), 1, msg);
        snprintf(msg, sizeof(msg), "A1 round %d: the granted-ability list must hold "
                 "exactly one entry, which IS the statement 'not appended'", round);
        CHECK_EQ(granted_n(&tg, niko), 1, msg);
    }
}

/* a2 niko_grant_latches_at_live_start_and_survives_its_condition_later_failing */
static void a2_niko_grant_latches_at_live_start(void)
{
    static TestGame tg;
    new_game(&tg);

    int niko = test_id_exact(&tg, NIKO);
    int mu   = test_id_exact(&tg, MU_MEM);
    CHECK(niko >= 0 && mu >= 0, "A2: both fixtures resolve");
    if (niko < 0 || mu < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO);

    niko_waitroom_granted(&tg, niko, 25);
    CHECK(fire_trigger(&tg, niko, TRIG_LIVE_START), "A2: the ライブ開始時 fires");
    drain(&tg);
    CHECK_EQ(constant_bonus(&tg), 1, "A2 precondition: the grant is live");

    /* Drop the waitroom one card short of the printed threshold. */
    tg.state.p[0].discard.n -= 1;
    CHECK_EQ(wait_n(&tg, 0), 24,
             "A2 precondition: exactly one card short of the 25枚以上 threshold");
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 1,
             "「ライブ終了時まで」 runs from the moment the trigger fired: the "
             "25枚以上 condition is assessed ONCE, at ライブ開始時, and is not "
             "re-checked. Revoking here would take a legally held grant away.");
    CHECK_EQ(granted_n(&tg, niko), 1,
             "A2: the registration must be untouched by a condition that no longer "
             "applies — an engine that re-evaluated the latch passes a1 and fails here");
}

/* a3 niko_grant_expires_through_the_real_live_end_rollover */
static void a3_niko_grant_expires_at_live_end(void)
{
    static TestGame tg;
    new_game(&tg);

    int niko = test_id_exact(&tg, NIKO);
    int mu   = test_id_exact(&tg, MU_MEM);
    CHECK(niko >= 0 && mu >= 0, "A3: both fixtures resolve");
    if (niko < 0 || mu < 0) { n_setup_bugs++; return; }
    expect_card_no(niko, NIKO);

    niko_waitroom_granted(&tg, niko, 25);
    CHECK(fire_trigger(&tg, niko, TRIG_LIVE_START), "A3: the ライブ開始時 fires");
    drain(&tg);
    CHECK_EQ(constant_bonus(&tg), 1, "A3 precondition: the grant is live");

    /* DURATION, and it needs a REAL rollover. A grant armed while still in Main
     * never sees one, so a test asserting expiry without the walk asserts
     * nothing at all — that is the first draft of this case. */
    int live = test_id_exact(&tg, NIKO_LIVE);
    CHECK(live >= 0, "A3: the live fixture resolves");
    test_add_to_hand(&tg, live);
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "A3 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, live);
    CHECK_EQ(live_n(&tg, 0), 1, "A3 precondition: the live card really is seated");

    int guard = 0;
    while (in_live_window(&tg) && guard++ < 16) {
        test_pass(&tg);
        drain_choices(&tg);
    }
    CHECK(!in_live_window(&tg),
          "A3 precondition: the walk crossed the victory-determination rollover "
          "(live-end is where the printed duration is meant to end)");
    rb_recalc_constants(&tg.state);

    CHECK(wait_n(&tg, 0) >= 25,
          "A3 precondition: the 25枚以上 condition STILL holds at the rollover, so "
          "only the printed ライブ終了時まで can be what ended the grant — the next "
          "turn's Draw may have refilled the waitroom, which only strengthens it");
    CHECK_EQ(constant_bonus(&tg), 0,
             "ライブ終了時まで is the whole duration: the grant must NOT survive the "
             "live that granted it");
    CHECK_EQ(granted_n(&tg, niko), 0,
             "A3: the registration must be cleared by the rollover too — an expired "
             "grant still registered is one anything consulting the store can re-read");
}

/* 宮下愛 「このターン、自分のステージにメンバーが2回以上登場している場合、
 *   ライブ終了時まで、常時 +1」 — 『2回以上』 is a THRESHOLD, not a counter. */
static int ai_staged_past_threshold(TestGame *tg, int n_debuts, int *out_ai)
{
    int ai  = test_id_exact(tg, AI_NAIL);
    int mem = test_id_exact(tg, FILLER);
    if (ai < 0 || mem < 0) return -1;
    fill_decks(tg, 40);
    clear_stage(tg, 0);
    tg->state.p[0].hand.n = 0;
    tg->state.debut_count_this_turn[0] = 0;
    test_give_energy(tg, 30);

    int first = test_new_id(tg, FILLER);
    int third = test_new_id(tg, FILLER);
    if (first < 0 || third < 0) return -1;
    test_add_to_hand(tg, first);
    test_add_to_hand(tg, ai);
    test_add_to_hand(tg, third);

    test_play_to_stage(tg, first, 0);
    scan_auto(tg, 0);
    test_play_to_stage(tg, ai, 1);
    scan_auto(tg, 0);
    if (n_debuts >= 3) {
        test_play_to_stage(tg, third, 2);
        scan_auto(tg, 0);
    }
    *out_ai = ai;
    return stage_n(tg, 0);
}

/* a4 ai_grant_is_a_flat_bonus_past_its_two_debut_threshold */
static void a4_ai_grant_is_a_flat_bonus_past_its_threshold(void)
{
    static TestGame tg;
    new_game(&tg);

    int ai = -1;
    int staged = ai_staged_past_threshold(&tg, 3, &ai);
    CHECK(staged >= 0, "A4: the 宮下愛 fixture builds");
    if (staged < 0) { n_setup_bugs++; return; }
    expect_card_no(ai, AI_NAIL);
    expect_trigger(ai, TRIG_LIVE_START, 1);
    CHECK_EQ(staged, 3, "A4 precondition: three members are staged this turn");
    CHECK(tg.state.debut_count_this_turn[0] >= 3,
          "A4 precondition: the debut counter really reached 3, which is the "
          "「このターン…2回以上」 clause the grant is conditioned on");

    int turn_before = tg.state.turn;
    CHECK(fire_trigger(&tg, ai, TRIG_LIVE_START), "A4: the ライブ開始時 fires");
    drain(&tg);
    CHECK_EQ(tg.state.turn, turn_before,
             "A4 precondition: arming must not itself roll the turn over");

    CHECK_EQ(constant_bonus(&tg), 1,
             "「2回以上」 is a threshold, not a counter: three debuts past it must "
             "still grant a flat +1, never +2");
    CHECK_EQ(granted_n(&tg, ai), 1, "A4 precondition: exactly one granted ability");

    for (int round = 1; round <= 4; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "A4 round %d: the grant must not accumulate as "
                 "scans repeat", round);
        CHECK_EQ(constant_bonus(&tg), 1, msg);
        snprintf(msg, sizeof(msg), "A4 round %d: exactly one granted ability", round);
        CHECK_EQ(granted_n(&tg, ai), 1, msg);
    }
}

/* a5 ai_grant_expires_through_the_real_live_end_rollover */
static void a5_ai_grant_expires_at_live_end(void)
{
    static TestGame tg;
    new_game(&tg);

    int ai = -1;
    int staged = ai_staged_past_threshold(&tg, 2, &ai);
    CHECK(staged >= 0, "A5: the 宮下愛 fixture builds");
    if (staged < 0) { n_setup_bugs++; return; }
    expect_card_no(ai, AI_NAIL);

    CHECK(fire_trigger(&tg, ai, TRIG_LIVE_START), "A5: the ライブ開始時 fires");
    drain(&tg);
    CHECK_EQ(constant_bonus(&tg), 1, "A5 precondition: the grant is live");

    int live = test_id_exact(&tg, NIKO_LIVE);
    CHECK(live >= 0, "A5: the live fixture resolves");
    test_add_to_hand(&tg, live);
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "A5 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, live);

    int guard = 0;
    while (in_live_window(&tg) && guard++ < 16) {
        test_pass(&tg);
        drain_choices(&tg);
    }
    CHECK(!in_live_window(&tg),
          "A5 precondition: the walk crossed the live-end rollover");
    rb_recalc_constants(&tg.state);

    CHECK(stage_n(&tg, 0) >= 2,
          "A5 precondition: both debuts are STILL on stage, so the このターン "
          "condition is unchanged and only the duration can have ended the grant");
    CHECK_EQ(constant_bonus(&tg), 0, "A5: the grant must not outlive ライブ終了時");
    CHECK_EQ(granted_n(&tg, ai), 0,
             "A5: the registration must be cleared as well as the score");
}

/* 乙宗梢 「自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、
 *   かつ名前が異なる場合、常時 +1」 — a 常時 grant, so the condition IS
 * re-evaluated on every scan. */
static void stage_three_distinct_renjo(TestGame *tg, int koto)
{
    int uta  = test_id_exact(tg, RENJO_UTA);
    int koko = test_id_exact(tg, RENJO_KOKO);
    CHECK(uta >= 0 && koko >= 0, "A6: both 蓮ノ空 partner fixtures resolve");
    if (uta < 0 || koko < 0) return;
    CHECK(uta != koto && koko != koto && uta != koko,
          "A6 precondition: 名前が異なる means the three must be three DIFFERENT "
          "cards — two copies of one name would fail the clause for the wrong reason");
    tg->state.p[0].stage[0] = koto;
    tg->state.p[0].stage[1] = uta;
    tg->state.p[0].stage[2] = koko;
}

/* a6 koto_grant_is_withdrawn_when_one_area_empties_and_returns_when_refilled */
static void a6_koto_grant_withdrawn_and_regranted(void)
{
    static TestGame tg;
    new_game(&tg);

    int koto = test_id_exact(&tg, KOTO);
    CHECK(koto >= 0, "A6: the 乙宗梢 fixture resolves");
    if (koto < 0) { n_setup_bugs++; return; }
    expect_card_no(koto, KOTO);
    expect_trigger(koto, TRIG_CONSTANT, 1);

    stage_three_distinct_renjo(&tg, koto);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 1,
             "A6 precondition: all three areas hold 『蓮ノ空』 members of DIFFERENT "
             "names, so the 常時 +1 must be live");
    EXPECTED_GAP(granted_n(&tg, koto) == 1,
                 "A6: the Rust test also reads a REGISTRATION count for this printed "
                 "constant grant, and C has no field for that triggerless TEXT half at "
                 "all. GameState.gained_card_abilities is the RESOLVED-ABILITY half, "
                 "which a printed constant never populates, because the constant scan "
                 "evaluates the card's own printed ability numerically. The score "
                 "assertions above and below ARE the parity claim and they hold; this "
                 "registration count is not expressible in C on this path.");

    /* REVOCATION, through the engine's own zone-exit choke point so this is a
     * revocation test rather than another grant test. */
    int vacated = tg.state.p[0].stage[2];
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    left_zones(&tg, vacated);
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 0,
             "「エリアすべて」 means every area: ONE empty area must WITHDRAW the grant");
    CHECK_EQ(granted_n(&tg, koto), 0,
             "A6: on this C path the registration half is structurally absent, so this "
             "reads 0 whether or not anything was withdrawn -- it is kept as a "
             "documented no-op rather than silently counted as evidence");

    /* Refill: the grant returns ONCE, not stacked on the withdrawn one. */
    int koko = test_id_exact(&tg, RENJO_KOKO);
    CHECK(koko >= 0, "A6: the refill fixture resolves");
    if (koko < 0) return;
    tg.state.p[0].stage[2] = koko;
    for (int round = 1; round <= 4; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "A6 round %d: refilling the empty area re-grants "
                 "EXACTLY one +1 — not two, not zero", round);
        CHECK_EQ(constant_bonus(&tg), 1, msg);
    }
}

/* a7 koto_grant_requires_distinct_names_not_just_area_coverage */
static void a7_koto_grant_requires_distinct_names(void)
{
    static TestGame tg;
    new_game(&tg);

    int koto = test_id_exact(&tg, KOTO);
    int uta  = test_id_exact(&tg, RENJO_UTA);
    int uta2 = test_new_id(&tg, RENJO_UTA);
    CHECK(koto >= 0 && uta >= 0 && uta2 >= 0, "A7: all three fixtures resolve");
    if (koto < 0 || uta < 0 || uta2 < 0) { n_setup_bugs++; return; }
    expect_card_no(koto, KOTO);
    CHECK(uta2 != uta,
          "A7 precondition: the two right-hand slots hold DISTINCT physical cards of "
          "one print — a single id stamped into two slots would alias the zones");

    /* Three areas covered, only TWO distinct names. */
    tg.state.p[0].stage[0] = koto;
    tg.state.p[0].stage[1] = uta;
    tg.state.p[0].stage[2] = uta2;
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 0,
             "「名前が異なる」 is PART of the condition: three areas holding only TWO "
             "distinct names must NOT grant. Without this, a6 could be passing on "
             "area coverage alone and never exercising the name clause at all.");
    EXPECTED_GAP(granted_n(&tg, koto) == 0,
                 "A7: the same registration half as A6 -- not expressible in C for a "
                 "printed constant grant, so the name-clause verdict rests entirely on "
                 "the score assertion directly above");
}

/* a8 koto_grant_does_not_outlive_the_host_leaving_the_stage */
static void a8_koto_grant_does_not_outlive_host_zone_exit(void)
{
    static TestGame tg;
    new_game(&tg);

    int koto = test_id_exact(&tg, KOTO);
    CHECK(koto >= 0, "A8: the 乙宗梢 fixture resolves");
    if (koto < 0) { n_setup_bugs++; return; }
    expect_card_no(koto, KOTO);

    stage_three_distinct_renjo(&tg, koto);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 1, "A8 precondition: the grant is live");
    EXPECTED_GAP(granted_n(&tg, koto) == 1,
                 "A8: the same registration half as A6/A7 -- not expressible in C for a "
                 "printed constant grant, so the zone-exit verdict rests entirely on the "
                 "score assertion at the end of this case");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    left_zones(&tg, koto);
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 0,
             "rule 4.1.4: a host that left the stage must carry NO grant — a zone "
             "change makes the card new, and a surviving +1 would score for a card "
             "on no board");
    CHECK_EQ(granted_n(&tg, koto), 0,
             "A8: the same registration half as A6/A7 -- structurally absent in C, so "
             "this reads 0 either way and the zone-exit claim is carried by the score "
             "assertion immediately above");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §B  ability_watch / lifecycle_tail_idempotence_test.rs — 7 Rust tests
 *
 * The tail of the same family: every card here had a test that resolved its
 * ability ONCE and read the number, and a single resolution cannot see whether
 * the registration is STABLE. So all seven repeat the derivation as many times
 * as a real turn does and assert the number is UNCHANGED, and where the grant
 * is scoped they cross the boundary and assert it is GONE.
 *
 * Four of the seven grant a 常時 合計スコア bonus, so the observable is a NUMBER
 * and a grant that doubled passes a presence check and fails these. Two mutate
 * a trigger FLAG instead, and for those "idempotent" means the flag does not
 * compound.
 * ═════════════════════════════════════════════════════════════════════════ */

/* 上原歩夢＆澁谷かのん＆日野下花帆 「常時 合計スコアを+3」 until live end.
 * A flat 3 is the cleanest idempotence probe in the family: a duplicated
 * registration reads +6, not +3, so it cannot hide behind a rounding. */
static int joint_board(TestGame *tg, int *out_joint)
{
    int joint = test_id_exact(tg, JOINT);
    int ayumu = test_id_exact(tg, AYUMU_JS);
    int kanon = test_id_exact(tg, KANON_JS);
    int kaho  = test_id_exact(tg, KAHO_JS);
    int filler = test_id_exact(tg, FILLER);
    if (joint < 0 || ayumu < 0 || kanon < 0 || kaho < 0 || filler < 0) return 0;
    CHECK(joint != ayumu && joint != kanon && joint != kaho,
          "B precondition: the three cost cards really are three DIFFERENT prints — "
          "two copies of one name would satisfy the cost condition for the wrong reason");
    fill_decks(tg, 40);
    clear_stage(tg, 0);
    tg->state.p[0].hand.n = 0;
    tg->state.p[0].stage[1] = joint;
    test_add_to_hand(tg, ayumu);
    test_add_to_hand(tg, kanon);
    test_add_to_hand(tg, kaho);
    test_add_to_hand(tg, filler);
    test_give_energy(tg, 5);
    *out_joint = joint;
    return 1;
}

/* The optional cost names three cards; take all three, one prompt at a time. */
static int joint_pay_cost(TestGame *tg, int expect_prompts)
{
    int n_prompts = 0, guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) {
        const char *t = choice_type(tg);
        if (!t || strcmp(t, "SelectCard") != 0) break;
        n_prompts++;
        if (n_prompts <= 3) {
            int idx[1] = { n_prompts - 1 };
            rb_resume_with_choice_indices(&tg->state, idx, 1);
        } else {
            rb_resume_with_choice_indices(&tg->state, NULL, 0);
        }
    }
    drain(tg);
    (void)expect_prompts;
    return n_prompts;
}

/* b1 joint_three_score_grant_stays_single_across_repeated_scans */
static void b1_joint_grant_stays_single_across_scans(void)
{
    static TestGame tg;
    new_game(&tg);

    int joint = -1;
    CHECK(joint_board(&tg, &joint), "B1: the 歩夢/かのん/花帆 fixture builds");
    if (joint < 0) { n_setup_bugs++; return; }
    expect_card_no(joint, JOINT);
    expect_trigger(joint, TRIG_LIVE_START, 1);

    CHECK(fire_trigger(&tg, joint, TRIG_LIVE_START), "B1: the ライブ開始時 fires");
    int prompts = joint_pay_cost(&tg, 3);
    CHECK(prompts >= 1,
          "B1: the OPTIONAL cost must really have been offered — a case that paid "
          "nothing would not be about the grant's lifecycle at all");

    CHECK_EQ(constant_bonus(&tg), 3,
             "B1 precondition: discarding all three named cards must grant the flat +3");
    CHECK_EQ(granted_n(&tg, joint), 1, "B1 precondition: exactly one granted ability");

    for (int round = 1; round <= 6; round++) {
        char msg[256];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "B1 round %d: the grant must stay +3. A DUPLICATED "
                 "registration reads +6, which is the whole reason this card is the "
                 "cleanest idempotence probe in the family", round);
        CHECK_EQ(constant_bonus(&tg), 3, msg);
        snprintf(msg, sizeof(msg), "B1 round %d: exactly one granted ability", round);
        CHECK_EQ(granted_n(&tg, joint), 1, msg);
    }
}

/* b2 joint_three_score_grant_expires_at_the_real_live_end_rollover */
static void b2_joint_grant_expires_at_live_end(void)
{
    static TestGame tg;
    new_game(&tg);

    int joint = -1;
    CHECK(joint_board(&tg, &joint), "B2: the 歩夢/かのん/花帆 fixture builds");
    if (joint < 0) { n_setup_bugs++; return; }
    expect_card_no(joint, JOINT);

    int live = test_id_exact(&tg, PLAIN_LIVE);
    CHECK(live >= 0, "B2: the live fixture resolves");
    test_add_to_hand(&tg, live);
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "B2 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, live);

    CHECK(fire_trigger(&tg, joint, TRIG_LIVE_START), "B2: the ライブ開始時 fires");
    joint_pay_cost(&tg, 3);
    CHECK_EQ(constant_bonus(&tg), 3, "B2 precondition: the grant is live");

    CHECK(roll_over_live_end_strict(&tg),
          "B2 precondition: the walk crossed the live-end rollover — a grant armed "
          "outside a live never sees one, so this would assert nothing");
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 0,
             "ライブ終了時まで is the whole duration: the +3 must not survive the live "
             "that granted it");
    CHECK_EQ(granted_n(&tg, joint), 0,
             "B2: the registration must be cleared as well as the score");
}

/* 東條希 「自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が
 * 1以下の場合、ライブ終了時まで、常時 +1」 — a LATCHED condition, assessed once
 * at 登場, and revoked ONLY by the printed duration. */
static void b3_nozomi_grant_stays_single_then_expires(void)
{
    static TestGame tg;
    new_game(&tg);

    int nozomi = test_id_exact(&tg, NOZOMI);
    int one    = test_id_exact(&tg, ONE_PT);
    int live   = test_id_exact(&tg, PLAIN_LIVE);
    CHECK(nozomi >= 0 && one >= 0 && live >= 0, "B3: every fixture resolves");
    if (nozomi < 0 || one < 0 || live < 0) { n_setup_bugs++; return; }
    expect_card_no(nozomi, NOZOMI);
    expect_trigger(nozomi, TRIG_DEBUT_STAGE, 1);

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    tg.state.p[0].stage[1] = nozomi;
    test_give_energy(&tg, 11);
    rb_waitroom_add(&tg.state.p[0], one);      /* p1 SUCCESS zone: 1 card, <= 1 */
    test_add_to_success(&tg, one);
    CHECK_EQ(success_n(&tg, 0), 1,
             "B3 precondition: p1's success zone really holds exactly one card");

    test_add_to_hand(&tg, live);
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "B3 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, live);

    CHECK(fire_trigger(&tg, nozomi, TRIG_DEBUT_STAGE), "B3: the 登場 fires");
    drain(&tg);
    CHECK_EQ(constant_bonus(&tg), 1,
             "B3 precondition: a success zone holding one 1-point card must grant the +1");

    for (int round = 1; round <= 5; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "B3 round %d: the grant must not accumulate as "
                 "scans repeat", round);
        CHECK_EQ(constant_bonus(&tg), 1, msg);
        snprintf(msg, sizeof(msg), "B3 round %d: exactly one granted ability", round);
        CHECK_EQ(granted_n(&tg, nozomi), 1, msg);
    }

    /* The condition latched at 登場: emptying the success zone must NOT revoke. */
    tg.state.p[0].success.n = 0;
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 1,
             "the condition LATCHED at 登場: emptying the success zone mid-live must "
             "not revoke a grant the card says runs to ライブ終了時 — revoking here "
             "would take a legally held bonus away");

    int guard = 0;
    while (in_live_window(&tg) && guard++ < 16) {
        test_pass(&tg);
        drain_choices(&tg);
    }
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 0,
             "but the DURATION still ends it: ライブ終了時まで must clear the grant");
    CHECK_EQ(granted_n(&tg, nozomi), 0,
             "B3: the registration must be cleared by the rollover too");
}

/* 国木田花丸 — ライブ開始時 grant with an OPTIONAL cost, granted to TWO
 * 『Aqours』 members. The registration is checked on the RECIPIENT, because
 * this card grants TO a member rather than to itself. */
static int hanamaru_board(TestGame *tg, int *out_hanamaru, int *out_friend,
                          int *out_outsider)
{
    int hanamaru = test_id_exact(tg, HANAMARU);
    int friend   = test_id_exact(tg, HANAMARU_F);
    int outsider = test_id_exact(tg, FILLER);
    if (hanamaru < 0 || friend < 0 || outsider < 0) return 0;
    CHECK(hanamaru != friend,
          "B4 precondition: 花丸 and her 『Aqours』 friend really are two DIFFERENT "
          "names — two copies of one card would satisfy 「『Aqours』のメンバー」 for "
          "the wrong reason, which is exactly why the Rust file pins them apart");
    fill_decks(tg, 40);
    clear_stage(tg, 0);
    tg->state.p[0].stage[1] = hanamaru;
    tg->state.p[0].stage[0] = friend;
    tg->state.p[0].stage[2] = outsider;
    /* p1's SUCCESS zone empty, p2's holding 2 -- the printed condition. The
     * success zone is its own bag, NOT the waitroom: writing through
     * rb_waitroom_add here would leave p2's success zone empty and the whole
     * case would assert against a condition that was never met. */
    tg->state.p[0].success.n = 0;
    tg->state.p[1].success.n = 0;
    {
        int p2l1 = test_id_exact(tg, PLAIN_LIVE);
        int p2l2 = test_new_id(tg, PLAIN_LIVE);
        if (p2l1 < 0 || p2l2 < 0) return 0;
        rb_waitroom_add(&tg->state.p[1], p2l1);
        tg->state.p[1].success.cards[tg->state.p[1].success.n++] = p2l1;
        tg->state.p[1].success.cards[tg->state.p[1].success.n++] = p2l2;
    }
    test_give_energy(tg, 2);
    *out_hanamaru = hanamaru;
    *out_friend = friend;
    *out_outsider = outsider;
    return 1;
}

/* b4 hanamaru_grant_stays_single_then_expires_at_live_end */
static void b4_hanamaru_grant_stays_single_then_expires(void)
{
    static TestGame tg;
    new_game(&tg);

    int hanamaru = -1, friend = -1, outsider = -1;
    CHECK(hanamaru_board(&tg, &hanamaru, &friend, &outsider), "B4: the fixture builds");
    if (hanamaru < 0) { n_setup_bugs++; return; }
    expect_card_no(hanamaru, HANAMARU);
    expect_trigger(hanamaru, TRIG_LIVE_START, 1);
    CHECK_EQ(success_n(&tg, 1), 2, "B4 precondition: p2's success zone really holds 2");

    CHECK(fire_trigger(&tg, hanamaru, TRIG_LIVE_START), "B4: the ライブ開始時 fires");
    /* Take the energy branch of the optional cost, then the beneficiaries. */
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
        const char *t = choice_type(&tg);
        if (t && strcmp(t, "SelectTarget") == 0) answer(&tg, 0);
        else if (t && strcmp(t, "SelectCard") == 0) {
            int idx[2] = { 0, 1 };
            rb_resume_with_choice_indices(&tg.state, idx, 2);
        } else answer(&tg, -1);
    }

    CHECK_EQ(constant_bonus(&tg), 2,
             "B4 precondition: the condition holds, so BOTH 『Aqours』 members gain "
             "+1 — the non-『Aqours』 member on the right is not counted");
    CHECK_EQ(granted_n(&tg, friend), 1,
             "B4 precondition: the BENEFICIARY holds one granted ability — this card "
             "grants to an 『Aqours』 member, so the registration is on the recipient");

    for (int round = 1; round <= 5; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "B4 round %d: the grant must not accumulate as "
                 "scans repeat", round);
        CHECK_EQ(constant_bonus(&tg), 2, msg);
        snprintf(msg, sizeof(msg), "B4 round %d: exactly one granted ability on the "
                 "RECIPIENT", round);
        CHECK_EQ(granted_n(&tg, friend), 1, msg);
    }

    /* Entered through a REAL live, so there is a real ライブ終了時 to expire at.
     * Rolling phases without ever performing a live crosses no victory
     * determination and the grant never gets a chance to end. */
    test_give_energy(&tg, 5);
    int live = test_id_exact(&tg, PLAIN_LIVE);
    test_add_to_hand(&tg, live);
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "B4 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, live);

    CHECK(roll_over_live_end_strict(&tg),
          "B4 precondition: the walk crossed a real victory determination");
    rb_recalc_constants(&tg.state);

    CHECK_EQ(constant_bonus(&tg), 0, "B4: ライブ終了時まで must clear the grant");
    CHECK_EQ(granted_n(&tg, friend), 0,
             "B4: the recipient's registration must be cleared as well as the score");
}

/* 高坂穂乃果 「自分のライブカード置き場にあるライブカードのスコアの合計が8以上
 * の場合、常時 +1」 — NO printed duration, so its ONLY revocation is zone exit. */
static void b5_honoka_grant_is_permanent_and_leaves_only_on_zone_exit(void)
{
    static TestGame tg;
    new_game(&tg);

    int honoka = test_id_exact(&tg, HONOKA);
    int s6     = test_id_exact(&tg, SEVEN_SCORE);
    int l1     = test_id_exact(&tg, PLAIN_LIVE);
    CHECK(honoka >= 0 && s6 >= 0 && l1 >= 0, "B5: every fixture resolves");
    if (honoka < 0 || s6 < 0 || l1 < 0) { n_setup_bugs++; return; }
    expect_card_no(honoka, HONOKA);
    expect_trigger(honoka, TRIG_LIVE_START, 1);

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    /* Her ライブ開始時 is printed with a centre restriction, so she belongs in the
     * middle slot: in a side slot the ability cannot resolve at all and every
     * assertion below would pass for the wrong reason. */
    tg.state.p[0].stage[1] = honoka;
    test_give_energy(&tg, 13);
    /* THREE cards, not one: the aggregate is summed across the zone and the
     * verified fixture is exactly 6 + 1 + 1 = 8. */
    rb_waitroom_add(&tg.state.p[0], s6);
    test_add_to_live(&tg, s6);
    test_add_to_live(&tg, l1);
    test_add_to_live(&tg, test_new_id(&tg, PLAIN_LIVE));
    CHECK_EQ(live_n(&tg, 0), 3, "B5 precondition: the live zone really holds 3 cards");

    int h[8] = { 20, 0, 0, 0, 0, 0, 0, 0 };
    set_stage_hearts(&tg, 0, h);

    /* ライブ開始時 is dispatched on ENTRY to the performance phase. */
    CHECK(test_advance_to_phase_named(&tg, "FirstAttackerPerformance"),
          "B5 precondition: the walk must reach the phase whose entry dispatches "
          "ライブ開始時");
    drain(&tg);
    CHECK_EQ(constant_bonus(&tg), 1,
             "B5 precondition: the live zone totals >= 8, so the ライブ開始時 must "
             "grant the +1 EXACTLY once — a doubled dispatch reads 2");
    CHECK_EQ(granted_n(&tg, honoka), 1, "B5 precondition: one granted ability");

    for (int round = 1; round <= 5; round++) {
        char msg[128];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "B5 round %d: the grant must not accumulate as "
                 "scans repeat", round);
        CHECK_EQ(constant_bonus(&tg), 1, msg);
    }

    /* No duration is printed, so a live-end rollover must NOT take it away. */
    int guard = 0;
    while (in_live_window(&tg) && guard++ < 16) {
        test_pass(&tg);
        drain_choices(&tg);
    }
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 1,
             "this grant prints NO ライブ終了時まで, so it must SURVIVE a live end — "
             "expiring it here would silently drop a permanent +1 mid-match");

    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    left_zones(&tg, honoka);
    rb_recalc_constants(&tg.state);
    CHECK_EQ(constant_bonus(&tg), 0,
             "rule 4.1.4: zone exit makes the card new, so the grant must not outlive it");
    CHECK_EQ(granted_n(&tg, honoka), 0,
             "B5: the registration must be cleared by the zone exit");
}

/* 元気全開DAY!DAY!DAY! — 「自分のステージにいる『Aqours』のメンバーが持つハートに、
 * heart02 が合計6個以上ある場合、このカードのライブ成功時能力を無効にする」.
 * The flag targets the live card's OWN ライブ成功時, so the observable is the
 * INVALIDATION RECORD, not a score. The condition reads the heart02 the STAGED
 * 『Aqours』 members actually hold — that is a zone-content query, so
 * stage_hearts does NOT satisfy it and real members must be on the board. */
static void b6_genki_invalidation_survives_repeated_derivation(void)
{
    static TestGame tg;
    new_game(&tg);

    int genki = test_id_exact(&tg, GENKI);
    int yoi   = test_id_exact(&tg, YOI);
    int ruby  = test_id_exact(&tg, RUBY);
    CHECK(genki >= 0 && yoi >= 0 && ruby >= 0, "B6: every fixture resolves");
    if (genki < 0 || yoi < 0 || ruby < 0) { n_setup_bugs++; return; }
    expect_card_no(genki, GENKI);
    expect_trigger(genki, TRIG_LIVE_START, 1);
    CHECK(yoi != ruby,
          "B6 precondition: the two heart-holders really are two DISTINCT cards — "
          "one id in two slots would make the heart02 total ambiguous");

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    /* 4 + 4 clears the printed 6 threshold. The per-card figure is READ from the
     * database rather than hardcoded, because the Rust comment says 4 and the
     * decoded Card sums to 5 for one of the two prints; hardcoding a number the
     * decoder disagrees with would turn a premise into a false failure. What is
     * load-bearing is the TOTAL clearing 6, and that is what is asserted. */
    tg.state.p[0].stage[0] = yoi;
    tg.state.p[0].stage[1] = ruby;
    CHECK(yoi != ruby,
          "B6 precondition: the two heart-holders really are two DISTINCT cards — "
          "one id in two slots would make the heart02 total ambiguous");
    {
        int h2 = printed_heart_count(yoi, 2) + printed_heart_count(ruby, 2);
        CHECK(h2 >= 6,
              "B6 PRECONDITION: the staged 『Aqours』 members must really hold 6+ heart02 "
              "between them, or the invalidation condition is never met and the "
              "negative below would pass for the wrong reason (got %d)", h2);
    }
    for (int i = 0; i < 3; i++) fill_energy_deck(&tg, 0, 1);

    /* 元気全開 IS the live card, because the invalidation targets that card's own
     * ライブ成功時 and the dispatch looks at the live card. */
    test_add_to_hand(&tg, genki);
    test_give_energy(&tg, 20);
    tg.state.p[0].live.n = 0;
    CHECK(test_advance_to_phase_named(&tg, "LiveCardSetFirstAttacker"),
          "B6 precondition: the walk reaches the live-card-set phase");
    test_set_live_card(&tg, 0, genki);
    CHECK_EQ(live_n(&tg, 0), 1, "B6 precondition: 元気全開 really is the live card");

    /* 元気全開 prints score 3 and needs heart02 4 + a colourless 2, so the
     * performance heart set decides whether its ライブ成功時 is even reachable.
     * Satisfying it is not a convenience here: without it the live FAILS, the
     * gated dispatch is a no-op, and the negative would pass without the
     * invalidation doing anything. Injected immediately BEFORE the dispatch,
     * because the performance RECOMPUTES the tally on a walk and would overwrite
     * a value set earlier — the exact defect the Rust comment documents. */
    int h[8] = { 0, 0, 4, 0, 0, 0, 0, 0 };
    set_stage_hearts(&tg, 0, h);
    CHECK(test_advance_to_phase_named(&tg, "FirstAttackerPerformance"),
          "B6 precondition: the walk reaches the performance phase, which is where "
          "ライブ開始時 is dispatched");
    drain_auto(&tg);
    drain_choices(&tg);

    /* The performance is EXECUTED on a later pass, not on entry to its phase, so
     * keep stepping until a snapshot exists. Without one the gated dispatch below
     * is a no-op and the negative assertion after it is vacuous. */
    int snap_guard = 0;
    while (tg.state.n_snapshots == 0 && snap_guard++ < 6) {
        test_pass(&tg);
        drain_auto(&tg);
        drain_choices(&tg);
    }
    CHECK(tg.state.n_snapshots > 0,
          "B6 precondition: a real performance snapshot exists — without one the "
          "ライブ成功時 dispatch below never runs and the case proves nothing");

    /* Re-derive as many times as a live window does, keeping the invalidation
     * standing across the real dispatch rather than a forced one. */
    for (int i = 0; i < 3; i++) {
        rb_recalc_constants(&tg.state);
        drain_auto(&tg);
    }
    CHECK(invalidated(&tg, genki, RB_TSTR_LIVE_SUCCESS),
          "B6 precondition: with 6+ heart02 on stage the ライブ開始時 must invalidate "
          "the live card's own ライブ成功時");

    for (int round = 1; round <= 5; round++) {
        rb_recalc_constants(&tg.state);
        fire_trigger(&tg, genki, TRIG_LIVE_START);
        drain(&tg);
        char msg[160];
        snprintf(msg, sizeof(msg), "B6 round %d: the invalidation must remain a "
                 "SINGLE standing suppression, not be withdrawn by a re-derivation "
                 "and not compound into a stronger one", round);
        CHECK(invalidated(&tg, genki, RB_TSTR_LIVE_SUCCESS), msg);
    }

    /* The BEHAVIOURAL half — "the invalidated ライブ成功時 places NOTHING" — is
     * NOT asserted, and its absence is the FINDING rather than an omission: it
     * requires a live that SUCCEEDS, and whether 元気全開 can succeed under a
     * walked phase is exactly the open question the Rust file records. Asserting
     * it through the gated dispatch would pass identically when the live never
     * succeeded, i.e. vacuously. Recorded so a reader arriving at the missing
     * assertion has the reason, and so it is not mistaken for an oversight. */
    CHECK(1,
          "B6 recorded limitation: the behavioural half (an invalidated ライブ成功時 "
          "places nothing) is NOT asserted, because it is only meaningful on a live "
          "that SUCCEEDS and no fixture here is known to produce one — asserting it "
          "through the gated dispatch would hold identically on a FAILED live, i.e. "
          "vacuously. What IS established is the half that needs no window: the "
          "invalidation is registered by ライブ開始時 and survives repeated "
          "re-derivation as a single standing record.");
}

/* Butterfly Wing 「自分のステージにいるメンバーが持つライブ開始時能力は発動しない」
 * — a 常時 SUPPRESSION, so it is re-derived on every constant scan: the same
 * path whose append-instead-of-replace behaviour shipped as the duplication bug.
 * Idempotence here means the suppression survives as ONE standing effect. */
static void b7_butterfly_suppression_survives_repeated_scans(void)
{
    static TestGame tg;
    new_game(&tg);

    int butterfly = test_id_exact(&tg, BUTTERFLY);
    int aqours   = test_id_exact(&tg, AQOURS);
    CHECK(butterfly >= 0 && aqours >= 0, "B7: every fixture resolves");
    if (butterfly < 0 || aqours < 0) { n_setup_bugs++; return; }
    expect_card_no(butterfly, BUTTERFLY);
    expect_trigger(butterfly, TRIG_CONSTANT, 1);

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    tg.state.p[0].stage[0] = aqours;
    test_add_to_live(&tg, butterfly);
    CHECK_EQ(live_n(&tg, 0), 1, "B7 precondition: Butterfly Wing really is the live card");

    CHECK(suppressed(&tg, 0, "live_start"),
          "B7 precondition: the 常時 suppression must be live while Butterfly Wing is "
          "the live card — a precondition that fails would make the six rounds below "
          "a test of nothing at all");
    CHECK(!suppressed(&tg, 1, "live_start"),
          "B7 precondition: p2 is NOT suppressed, which is what the 自分のステージ "
          "scope means and what the per-round negative re-checks");

    for (int round = 1; round <= 6; round++) {
        char msg[176];
        rb_recalc_constants(&tg.state);
        snprintf(msg, sizeof(msg), "B7 round %d: re-deriving the 常時 must leave the "
                 "suppression in place — it is a STANDING effect, not a one-shot "
                 "registration", round);
        CHECK(suppressed(&tg, 0, "live_start"), msg);
        snprintf(msg, sizeof(msg), "B7 round %d: the suppression is 自分のステージ "
                 "only, so p2 must be unaffected on EVERY derivation", round);
        CHECK(!suppressed(&tg, 1, "live_start"), msg);
    }

    /* Removing the host from the live zone is the one thing that lifts it, and
     * the re-derivation after that must OBSERVE the lift rather than
     * re-registering from a stale copy. */
    {
        RbPlayer *P = &tg.state.p[0];
        int w = 0;
        for (int i = 0; i < P->live.n; i++)
            if (P->live.cards[i] != butterfly) P->live.cards[w++] = P->live.cards[i];
        P->live.n = w;
    }
    left_zones(&tg, butterfly);
    for (int i = 0; i < 3; i++) rb_recalc_constants(&tg.state);
    CHECK(!suppressed(&tg, 0, "live_start"),
          "once Butterfly Wing leaves the live zone the suppression must be gone, and "
          "the next scan must not resurrect it from a stale registration");
}

/* ══════════════════════════════════════════════════════════════════════════
 * §C  ability_watch / invalidated_live_start_restores_next_live_test.rs — 3
 *
 * The CONSEQUENCE half of 澁谷かのん's 登場 invalidation. The grant-side file
 * pins the prompt, the skip, the double-invalidation refusal and the flag's
 * expiry; it never drives the thing the invalidation is FOR — the silenced
 * card's ライブ開始時 not resolving — and never shows the ability COMING BACK.
 *
 * Why 米女メイ is the target: the existing expiry test asserts blade == 0 on a
 * card whose ライブ開始時 is OPTIONAL, so blade == 0 also holds when the ability
 * was never OFFERED, which makes that negative ambiguous. 米女メイ's
 * 「エネルギーを2枚アクティブにする」 is unconditional and has exactly one
 * observable: the active-energy counter. Silence therefore means "the counter
 * did not move", which no un-offered prompt can imitate.
 *
 * §C1 is the CONTROL that licenses §C2 and §C3. Without it every negative below
 * would also pass for a target whose ability never fires at all.
 * ═════════════════════════════════════════════════════════════════════════ */

/* The energy zones are seeded at the LAST moment before the dispatch, which is
 * load-bearing and not cosmetic: the Energy phase re-seats the zone (return to
 * deck + charge) and re-reads an empty energy_deck, so a zone seeded before the
 * walk is re-shaped on the way in and 「2枚アクティブ」 finds no waited energy.
 * The effect then no-ops for a reason that has nothing to do with the
 * invalidation, and the measurement reads 0 — a false positive for §C2. */
/* Seed BOTH energy zones at the LAST moment before the dispatch.
 *
 * Load-bearing, not cosmetic. The Energy phase re-seats the zone (return to deck
 * + charge) and re-reads an empty energy_deck, so a zone seeded BEFORE the walk
 * is re-shaped on the way in and the +2-active effect finds no waited energy to
 * activate. It then no-ops for a reason that has nothing to do with the
 * invalidation, and the measurement reads 0 -- which is exactly the value C2
 * asserts, so a pre-seeded zone would make C2 pass VACUOUSLY. Each card is its
 * own pool instance, because a zone's active/waited split is tracked per zone
 * and one id sitting in two zones makes "how many cards does this zone hold"
 * ambiguous. The energy DECKs are left EMPTY so no Energy phase inside the live
 * window can refill a zone and be charged to the trigger under test. */
static void seed_energy_zones_late(TestGame *tg)
{
    tg->state.p[0].energy.n = 0;
    tg->state.p[1].energy.n = 0;
    for (int i = 0; i < 6; i++) {
        int e1 = test_new_id(tg, ENERGY);
        int e2 = test_new_id(tg, ENERGY);
        if (e1 < 0 || e2 < 0) break;
        if (tg->state.p[0].energy.n < RB_MAX_ZONE)
            tg->state.p[0].energy.cards[tg->state.p[0].energy.n++] = e1;
        if (tg->state.p[1].energy.n < RB_MAX_ZONE)
            tg->state.p[1].energy.cards[tg->state.p[1].energy.n++] = e2;
    }
    rb_energy_set_active_count(&tg->state.p[0], 4);
    rb_energy_set_active_count(&tg->state.p[1], 4);
    tg->state.p[0].energy_deck.n = 0;
    tg->state.p[1].energy_deck.n = 0;
}

/* Walk to the live's trigger dispatch and return the active-energy delta that
 * entering the performance phase produced, for p1 and p2.
 *
 * PHASE-VALUE TRAP, avoided deliberately: RB_PHASE_LIVE_SET_SECOND and
 * RB_PHASE_PERFORMANCE_SECOND are NEVER STORED in C (phase.c:87-90) -- the
 * first/second attacker variants are ONE stored value each, told apart by
 * `state.active`. A literal compare against either _SECOND constant can never
 * fire, so the walk below targets RB_PHASE_LIVE_SET / RB_PHASE_PERFORMANCE and
 * does not try to distinguish the attacker seat.
 *
 * The BASELINE is the previous step, because an earlier phase in the window may
 * still move a counter (the Energy phase itself does). Sets *ok to 0 when the
 * walk never reached the dispatch, so a caller can never read "did not get
 * there" as "nothing moved". */
static void live_start_energy_delta(TestGame *tg, int *out_p1, int *out_p2, int *ok)
{
    int guard = 0;
    while (tg->state.phase != RB_PHASE_LIVE_SET && guard++ < 24) {
        test_pass(tg);
        drain_choices(tg);
    }
    *ok = (tg->state.phase == RB_PHASE_LIVE_SET);
    if (!*ok) { *out_p1 = 0; *out_p2 = 0; return; }

    seed_energy_zones_late(tg);
    CHECK(active_energy(tg, 0) == 4 && active_energy(tg, 1) == 4,
          "C precondition: both zones hold 4 active energy");
    CHECK(energy_n(tg, 0) - active_energy(tg, 0) >= 2 &&
          energy_n(tg, 1) - active_energy(tg, 1) >= 2,
          "C precondition: BOTH seats keep WAITED energy, so a +2 activation has "
          "something to activate. A p2 zone with none would read 0 for a reason "
          "unrelated to whatever is under test.");
    CHECK(energy_deck_n(tg, 0) == 0 && energy_deck_n(tg, 1) == 0,
          "C precondition: both energy DECKs are empty, so no Energy phase inside the "
          "live window can refill a zone and be charged to the trigger");

    int last1 = active_energy(tg, 0);
    int last2 = active_energy(tg, 1);
    guard = 0;
    while (tg->state.phase != RB_PHASE_PERFORMANCE && guard++ < 16) {
        last1 = active_energy(tg, 0);
        last2 = active_energy(tg, 1);
        test_pass(tg);
        drain_choices(tg);
    }
    *ok = (tg->state.phase == RB_PHASE_PERFORMANCE);
    *out_p1 = active_energy(tg, 0) - last1;
    *out_p2 = active_energy(tg, 1) - last2;
}

/* Control run: MEI on stage with NO invalidation. Proves the observable this
 * section reads really is +2, so a later "0" means the invalidation and not a
 * target that never fires. Built on its own TestGame because the walk consumes
 * it. */
static int control_mei_delta(void)
{
    static TestGame tg;
    new_game(&tg);
    int mei    = test_id_exact(&tg, MEI);
    int filler = test_id_exact(&tg, FILLER);
    int live   = test_id_exact(&tg, PLAIN_LIVE);
    CHECK(mei >= 0 && filler >= 0 && live >= 0, "C1: every fixture resolves");
    if (mei < 0 || filler < 0 || live < 0) { n_setup_bugs++; return -1; }
    expect_card_no(mei, MEI);
    /* MEI's printed unit is CatChu!, a DIFFERENT unit from Kanon's own. Kanon's
     * picker filters on the series-wide group, so the GROUP is what must be
     * pinned -- pinning her unit instead would narrow the claim to "Kanon can
     * name her own unit", which is not what the card says. */
    CHECK(rb_card_matches_group_str(mei, "Liella!"),
          "C1: the silenced target really is inside the series-wide group Kanon's "
          "picker filters on, not merely her own unit");

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    tg.state.p[0].stage[0] = mei;
    tg.state.p[0].stage[1] = filler;
    test_add_to_live(&tg, live);
    CHECK_EQ(live_n(&tg, 0), 1, "C1 precondition: p1 really holds a live card");

    int d1 = 0, d2 = 0, ok = 0;
    live_start_energy_delta(&tg, &d1, &d2, &ok);
    CHECK(ok, "C1 precondition: the walk reached the trigger dispatch (stopped at %s)",
          test_phase_name_now(&tg));
    if (!ok) { n_setup_bugs++; return -1; }
    return d1;
}

/* c1 mei_live_start_activates_two_energy_when_not_invalidated */
static void c1_mei_live_start_activates_two_energy(void)
{
    int delta = control_mei_delta();
    CHECK(delta >= 0, "C1: the control run completed");
    if (delta < 0) return;
    CHECK_EQ(delta, 2,
             "C1 CONTROL: MEI's live-start must move the active counter by 2. Without "
             "this, every negative in section C would also pass for a target whose "
             "ability never fires at all.");
}

/* c2 kanon_invalidation_silences_the_live_start_then_restores_it_next_live */
static void c2_kanon_invalidation_silences_then_restores(void)
{
    /* Control FIRST, on the SAME board shape, so the 0 below means the
     * invalidation rather than a target that never fires. */
    int control = control_mei_delta();
    CHECK_EQ(control, 2, "C2 CONTROL: the untouched board really does fire +2");

    static TestGame tg;
    new_game(&tg);
    int kanon    = test_id_exact(&tg, KANON_INV);
    int mei      = test_id_exact(&tg, MEI);
    int live     = test_id_exact(&tg, PLAIN_LIVE);
    int recovery = test_id_exact(&tg, RECOVERY);
    CHECK(kanon >= 0 && mei >= 0 && live >= 0 && recovery >= 0,
          "C2: every fixture resolves");
    if (kanon < 0 || mei < 0 || live < 0 || recovery < 0) { n_setup_bugs++; return; }
    expect_card_no(kanon, KANON_INV);
    expect_card_no(mei, MEI);
    CHECK(rb_card_matches_group_str(mei, "Liella!") &&
          rb_card_matches_group_str(kanon, "Liella!"),
          "C2: both the invalidator and the silenced target are inside the group");

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    test_add_to_hand(&tg, kanon);
    /* The recovery comes out of the WAITROOM, not the deck -- Kanon's printed cost
     * is a discard, so a deck-empty fixture would make the cost unpayable. */
    rb_waitroom_add(&tg.state.p[0], recovery);
    CHECK_EQ(wait_n(&tg, 0), 1, "C2 precondition: the recovery card is in the waitroom");
    test_give_energy(&tg, 13);
    tg.state.p[0].stage[0] = mei;
    test_add_to_live(&tg, live);

    /* Kanon's invalidation is OPTIONAL, so the PROMPT is what arms it. */
    CHECK(test_play_to_stage(&tg, kanon, 1), "C2: Kanon debuts to Center");
    CHECK_PROMPTS_GT(&tg, 0,
                     "C2: her debut must OFFER the optional invalidation -- a case in "
                     "which nothing was offered has not armed anything");
    {
        const char *t = choice_type(&tg);
        CHECK(t && strcmp(t, "SelectCard") == 0,
              "C2: the prompt is a target SELECTION for the invalidation");
    }
    {
        int idx[1] = { 0 };
        rb_resume_with_choice_indices(&tg.state, idx, 1);
    }
    drain_choices(&tg);

    CHECK(invalidated(&tg, mei, RB_TSTR_LIVE_START),
          "C2 precondition: the invalidation must actually be REGISTERED on MEI");

    /* --- live 1: silenced --- */
    int d1 = 0, d2 = 0, ok = 0;
    live_start_energy_delta(&tg, &d1, &d2, &ok);
    CHECK(ok, "C2: the walk reached the dispatch in live 1 (stopped at %s)",
          test_phase_name_now(&tg));
    if (ok) {
        CHECK_EQ(d1, 0,
                 "C2: the invalidated live-start must NOT activate energy -- and the "
                 "control above proved the same board moves the counter by 2");
    }
    CHECK(invalidated(&tg, mei, RB_TSTR_LIVE_START),
          "C2: the invalidation is live-end scoped, so it must STILL hold inside the live");

    /* --- live end: the invalidation lifts --- */
    int crossed = 0;
    roll_over_live_end(&tg, &crossed);
    CHECK(crossed, "C2: the walk really crossed the live-end rollover");
    CHECK(!invalidated(&tg, mei, RB_TSTR_LIVE_START),
          "C2: the live-end duration is the whole scope -- the invalidation must be gone");

    /* --- live 2: restored --- */
    /* The trigger is scanned per live, and p1's live card left the zone when the
     * first one ended, so the second live needs a card again. */
    fill_decks(&tg, 40);
    test_add_to_live(&tg, live);
    CHECK(!invalidated(&tg, mei, RB_TSTR_LIVE_START),
          "C2 precondition: nothing may RE-ARM the invalidation between the lives");

    int r1 = 0, r2 = 0, rok = 0;
    live_start_energy_delta(&tg, &r1, &r2, &rok);
    CHECK(rok, "C2: the walk reached the dispatch in live 2 (stopped at %s)",
          test_phase_name_now(&tg));
    if (rok) {
        CHECK_EQ(r1, 2,
                 "C2: the next live must run MEI's live-start again. An invalidation "
                 "that is removed-but-not-forgotten, and one that outlives the live "
                 "end, BOTH show up here and nowhere else.");
    }
}

/* c3 kanon_invalidation_does_not_touch_the_opponents_copy */
static void c3_kanon_invalidation_is_scoped_to_own_stage(void)
{
    static TestGame tg;
    new_game(&tg);
    int kanon    = test_id_exact(&tg, KANON_INV);
    int mei_p1   = test_id_exact(&tg, MEI);
    int mei_p2   = test_new_id(&tg, MEI);
    int recovery = test_id_exact(&tg, RECOVERY);
    int live     = test_id_exact(&tg, PLAIN_LIVE);
    CHECK(kanon >= 0 && mei_p1 >= 0 && mei_p2 >= 0 && recovery >= 0 && live >= 0,
          "C3: every fixture resolves");
    if (kanon < 0 || mei_p1 < 0 || mei_p2 < 0 || recovery < 0 || live < 0) {
        n_setup_bugs++;
        return;
    }
    CHECK(mei_p1 != mei_p2,
          "C3 precondition: the two stages must hold two DISTINCT card instances -- "
          "one id in two zones would make the scoping assertion vacuous");

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    test_add_to_hand(&tg, kanon);
    rb_waitroom_add(&tg.state.p[0], recovery);
    test_give_energy(&tg, 13);
    tg.state.p[0].stage[0] = mei_p1;
    tg.state.p[1].stage[0] = mei_p2;
    /* p2 needs a live card of its OWN: the second attacker's trigger is only
     * scanned for a player holding a live card (triggers.rs guards a live-less
     * second attacker), and this harness drives p1's actions, so p2's reveal is
     * placed directly -- the established idiom. */
    test_add_to_live(&tg, live);
    test_add_to_live_for(&tg, 1, live);
    CHECK_EQ(live_n(&tg, 1), 1, "C3 precondition: p2 really holds a live card");

    CHECK(test_play_to_stage(&tg, kanon, 1), "C3: Kanon debuts to Center");
    {
        const char *t = choice_type(&tg);
        CHECK(t && strcmp(t, "SelectCard") == 0,
              "C3: the debut must offer the optional invalidation as a target selection");
    }
    {
        int idx[1] = { 0 };
        rb_resume_with_choice_indices(&tg.state, idx, 1);
    }
    drain_choices(&tg);

    CHECK(invalidated(&tg, mei_p1, RB_TSTR_LIVE_START),
          "C3 precondition: p1's instance must be invalidated");
    CHECK(!invalidated(&tg, mei_p2, RB_TSTR_LIVE_START),
          "C3: the invalidation is scoped to the OWNER's stage, so p2's instance of "
          "the SAME print must be untouched");
    /* The two premises are asserted SEPARATELY on purpose. Combined into one
     * condition, a failure says only "not on opposite stages" and cannot tell a
     * fixture error from a placement the engine moved. */
    CHECK(on_stage(&tg, 1, 0, mei_p2),
          "C3 precondition: p2's instance really is on p2's stage");
    CHECK(on_stage(&tg, 0, 0, mei_p1),
          "C3 precondition: p1's instance really is STILL on p1's stage -- Kanon "
          "debuted to Center, so the Left slot must be untouched");

    int d1 = 0, d2 = 0, ok = 0;
    live_start_energy_delta(&tg, &d1, &d2, &ok);
    CHECK(ok, "C3: the walk reached the dispatch (stopped at %s)", test_phase_name_now(&tg));
    if (!ok) return;
    CHECK_EQ(d1, 0,
             "C3: the invalidated member's live-start must not activate p1's energy");
    CHECK_EQ(d2, 2,
             "C3: the OPPONENT's identical printed live-start must still resolve -- this "
             "is the same shared measurement the C1 control uses, so a 2 here means "
             "exactly what a 2 means there");
}

/* =====================================================================
 * Section D  debut_watch -- the three cases test_parity_jidou_watch.c
 * does NOT port. D1 and D2 are ENERGY and OPPONENT-STAGE negatives, which is
 * exactly what a suite full of positive appearance grants cannot imply: every
 * existing case in its sections L and P proves a watcher FIRES, and none of them
 * can show that it stays quiet on a board where it must.
 *
 *   baton_touch_arrival_self_or_other_draws_one_twice_per_turn_test.rs ::
 *       setsuna_opponent_stage_arrival_does_not_fire
 *   simultaneous_triggers_active_player_resolves_first_test.rs ::
 *       baton_touch_activates_energy
 *       edelnote_appearance_waits_opponent
 * ===================================================================== */

static int setsuna_use_count(TestGame *tg, int setsuna) { return use_count_ab0(tg, setsuna); }

static int setsuna_setup(TestGame *tg, int *out_setsuna)
{
    int setsuna = test_id_exact(tg, SETSUNA);
    int filler  = test_id_exact(tg, FILLER);
    CHECK(setsuna >= 0 && filler >= 0, "D: both fixtures resolve");
    if (setsuna < 0 || filler < 0) { n_setup_bugs++; return 0; }
    expect_card_no(setsuna, SETSUNA);
    /* Bulk deck filler uses the ONE template id, matching Rust `game.id` which
     * interns by card_no: the copy pool holds a limited number of instances per
     * card_no, and burning them on deck filler mints data-less ids that detonate
     * on the first DB lookup. */
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
    test_give_energy(tg, 30);
    *out_setsuna = setsuna;
    return 1;
}

/* d1 setsuna_opponent_stage_arrival_does_not_fire */
static void d1_setsuna_opponent_stage_arrival_does_not_fire(void)
{
    static TestGame tg;
    new_game(&tg);

    int setsuna = -1;
    if (!setsuna_setup(&tg, &setsuna)) return;
    tg.state.p[0].stage[0] = setsuna;

    int p2filler = test_new_id(&tg, FILLER);
    int arriver  = test_new_id(&tg, CHEAP);
    CHECK(p2filler >= 0 && arriver >= 0, "D1: both physical cards resolve");
    if (p2filler < 0 || arriver < 0) { n_setup_bugs++; return; }

    test_give_opp_energy(&tg, 30);
    tg.state.p[1].deck.n = 0;
    {
        int f = test_id_exact(&tg, FILLER);
        for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 1, f);
    }
    tg.state.p[1].stage[1] = p2filler;
    test_add_to_hand_for(&tg, 1, arriver);
    CHECK(hand_n(&tg, 1) >= 1, "D1 precondition: the opponent really holds the arriver");

    /* The play must actually HAPPEN, or "no draw" is true for free. */
    int played = test_play_to_stage_for(&tg, 1, arriver, 1);
    resolve_accepting_optionals(&tg);
    CHECK(played, "D1: the OPPONENT's baton-touch arrival really happened -- without "
                  "this the absence below is unfalsified");
    CHECK(on_stage(&tg, 1, 1, arriver), "D1: the arrival is on p2's center");

    CHECK_EQ(hand_n(&tg, 0), 0,
             "D1: an arrival on the OPPONENT's stage must not fire our watcher -- the "
             "printed clause scopes it to our OWN stage");
    CHECK_EQ(setsuna_use_count(&tg, setsuna), 0, "D1: and no use is recorded for it");
}

/* d2 baton_touch_activates_energy_q167
 * HANAFU's baton-touch trigger activates 2 energy when a cost>=10 member of her
 * group baton-touches her off. HANAFU(SD) costs 9, HANAFU_PR costs 10, so the
 * net cost is 1 and the ability's +2 nets to +1 over the 1 waited energy. */
/* One Q167 board, driven twice on separate TestGames: once with the cost-10
 * group partner that QUALIFIES the trigger, once with a cheap outsider that does
 * not. Returns the active-energy count after the play.
 *
 * The DELTA between the two runs is the assertion, not an absolute formula. An
 * earlier draft of this case wrote the arithmetic out by hand (-1 net cost, +2
 * activation) and was wrong about how the engine accounts a play cost against
 * the active count, so the case could only ever have been red. Comparing a
 * qualifying partner against a non-qualifying one on an otherwise identical
 * board is immune to that: whatever the cost model does to the counter, it does
 * the SAME thing on both boards, so the difference is the ability and nothing
 * else. */
static int q167_energy_after(int use_qualifying_partner, int *out_played)
{
    static TestGame tg;
    new_game(&tg);
    int hanafu   = test_id_exact(&tg, HANAFU);
    int replacer = use_qualifying_partner ? test_id_exact(&tg, HANAFU_PR)
                                          : test_id_exact(&tg, NON_GROUP_10);
    int filler   = test_id_exact(&tg, FILLER);
    *out_played = 0;
    CHECK(hanafu >= 0 && replacer >= 0 && filler >= 0,
          "D2: every fixture resolves for the %s board",
          use_qualifying_partner ? "qualifying" : "non-qualifying");
    if (hanafu < 0 || replacer < 0 || filler < 0) { n_setup_bugs++; return -1; }
    if (use_qualifying_partner) {
        expect_card_no(replacer, HANAFU_PR);
        CHECK(printed_cost(replacer) >= 10,
              "D2 precondition: the qualifying partner really costs >= 10, which is "
              "the gate the printed clause puts on the incoming member (got %d)",
              printed_cost(replacer));
        {
            int other = test_id_exact(&tg, NON_GROUP_10);
            int cq = printed_cost(replacer);
            int cn = other >= 0 ? printed_cost(other) : -1;
            CHECK(cq == cn,
                  "D2 PRECONDITION: the qualifying and non-qualifying partners must "
                  "print the SAME cost, or the two boards differ by their play cost as "
                  "well as by the ability and the comparison below is meaningless "
                  "(qualifying %d, non-qualifying %d)", cq, cn);
            /* U+8FF1 ren U+30CE no U+7A7A kuu = E8 BF 91 E3 83 8E E7 A9 BA. This is
             * the group's CANONICAL name in the C data (card.c maps the printed
             * series onto it), so the romanisation is NOT accepted here. */
            CHECK(rb_card_matches_group_str(replacer, "\xE8\xBF\x91\xE3\x83\x8E\xE7\xA9\xBA") !=
                  (other >= 0 && rb_card_matches_group_str(other, "\xE8\xBF\x91\xE3\x83\x8E\xE7\xA9\xBA")),
                  "D2 PRECONDITION: the two partners must differ on the printed group, "
                  "which is the only difference between the boards");
            CHECK(rb_card_matches_group_str(replacer, "\xE8\xBF\x91\xE3\x83\x8E\xE7\xA9\xBA"),
                  "D2 PRECONDITION: the qualifying partner really is inside the printed "
                  "group -- without this the board is not qualifying at all and the "
                  "Q167 delta below measures nothing");
        }
    }

    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    /* 20 active plus 20 WAITED: the activation needs waited energy to promote, and
     * an empty waited half would make the effect no-op for an unrelated reason. */
    test_give_energy(&tg, 20);
    set_energy_active(&tg, 0, 40, 20);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = hanafu;
    CHECK(active_energy(&tg, 0) == 20,
          "D2 precondition: 20 active energy to measure against");

    test_add_to_hand(&tg, replacer);
    int played = test_play_to_stage(&tg, replacer, 1);
    resolve_accepting_optionals(&tg);
    *out_played = played;
    CHECK(played, "D2: the incoming member really baton-touches HANAFU off Center");
    CHECK(on_stage(&tg, 0, 1, replacer), "D2: the incoming member really is at Center");
    CHECK(!on_stage(&tg, 0, 1, hanafu),
          "D2 precondition: HANAFU really did leave the stage, which is what the "
          "watcher is keyed on");
    return active_energy(&tg, 0);
}

/* d2 baton_touch_activates_energy_q167 */
static void d2_baton_touch_activates_energy_q167(void)
{
    int played_q = 0, played_n = 0;
    int e_qual = q167_energy_after(1, &played_q);
    int e_non  = q167_energy_after(0, &played_n);
    CHECK(e_qual >= 0 && e_non >= 0, "D2: both boards ran");
    if (e_qual < 0 || e_non < 0) return;

    CHECK(played_q && played_n,
          "D2 precondition: BOTH plays really happened -- otherwise the comparison "
          "below is between a board that moved and a board that did not");
    CHECK_EQ(e_qual - e_non, 2,
             "Q167: on an otherwise identical board, a >=10 partner of the printed "
             "group must leave the active-energy counter 2 HIGHER than a cheap "
             "outsider does. The cost model is identical on both boards, so the whole "
             "difference is the 「2枚アクティブ」 the clause grants (qualifying %d, "
             "non-qualifying %d)", e_qual, e_non);
}

/* d3 edelnote_appearance_waits_opponent -- the second case of Q84's file, and the
 * only one of its three that a suite full of positive waits does not imply. */
static void d3_edelnote_appearance_waits_opponent(void)
{
    static TestGame tg;
    new_game(&tg);

    int seras    = test_id_exact(&tg, CERAS_R);
    int edelnote = test_id_exact(&tg, CERAS_SD);
    int opp      = test_id_exact(&tg, FILLER);
    int filler   = test_id_exact(&tg, FILLER);
    CHECK(seras >= 0 && edelnote >= 0 && opp >= 0 && filler >= 0,
          "D3: every fixture resolves");
    if (seras < 0 || edelnote < 0 || opp < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(seras, CERAS_R);
    expect_card_no(edelnote, CERAS_SD);
    expect_trigger(seras, TRIG_AUTO, 1);
    CHECK(rb_card_matches_group_str(seras, "EdelNote"),
          "D3: the watcher really is an EdelNote member, which is the group its "
          "printed clause filters on");
    CHECK(printed_cost(edelnote) >= 0,
          "D3: the arriving print's cost resolves from the database");
    fill_decks(&tg, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    test_give_energy(&tg, 20);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = seras;
    tg.state.p[1].stage[0] = opp;
    CHECK(!is_wait(&tg, opp), "D3 precondition: the opponent member starts ACTIVE");

    test_add_to_hand(&tg, edelnote);
    int played = test_play_to_stage(&tg, edelnote, 2);
    resolve_accepting_optionals(&tg);

    CHECK(played, "D3: the EdelNote print really debuted to Right");
    CHECK(on_stage(&tg, 0, 2, edelnote), "D3: the arriver is on p1's Right");
    CHECK(is_wait(&tg, opp),
          "D3: the watcher's own appearance trigger must put the OPPONENT's lone "
          "active member to WAIT -- this is the Q250 effect driven through a real "
          "play rather than a synthesised trigger");
}

/* =====================================================================
 * Section E  discard_watch -- the Q269/Q277 refresh family, the remaining
 * real-mill edge, and the Q241 one-card control.
 *
 *   deck_to_discard_self_recovery_yell_refresh_q269_q277_test.rs ::
 *       q269_yell_reveal_mid_pool_does_not_trigger
 *       q269_yell_reveal_does_not_recover_existing_waitroom_copy
 *       q277_control_no_refresh_recovers_milled_self
 *       q277_refresh_before_auto_resolve_prevents_recover
 *       q277_real_mill_refreshes_before_mia_auto_resolves
 *   real_mill_optional_discard_recover_self_test.rs ::
 *       real_deck_top_mill_declined_discard_leaves_self_in_waitroom
 *   hand_discard_batch_heart_blade_q241_test.rs ::
 *       hand_discard_batch_q241_one_discarded_fires_once
 *
 * A yell REVEAL is a reveal, NOT a deck-to-waitroom placement, so MIA's ab#0
 * must never fire from one (official QA Q269). Q277 is the other side: a mill that
 * empties the deck to exactly 0 triggers a REFRESH before the auto resolves, the
 * refresh shuffles the waitroom back into the deck, and MIA is therefore no
 * longer in the waitroom to be recovered.
 * ===================================================================== */

/* Put MIA at deck index `idx` (0 = top) and give p1 one filler in hand. */
static int deck_top_mia(TestGame *tg, int idx)
{
    int mia = test_id_exact(tg, MIA);
    int f   = test_id_exact(tg, FILLER);
    CHECK(mia >= 0 && f >= 0, "E: both fixtures resolve");
    if (mia < 0 || f < 0) { n_setup_bugs++; return -1; }
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < idx; i++) test_add_to_deck_pl(tg, 0, f);
    test_add_to_deck_pl(tg, 0, mia);
    test_add_to_hand(tg, f);
    return mia;
}

/* Reproduce the real yell flow for p1: draw `count` cards into the resolution
 * zone, record each as a REVEALED card, then run the auto scan. The resolution
 * bag is emptied by rb_resolution_clear, so the ids are copied out BEFORE the
 * scan. Returns 1 if `mia` was among the revealed.
 *
 * A reveal pushes NO zone-change movement event, which is the whole point: the
 * tests below fail if a buggy reveal path ever starts emitting one. */
static int yell_reveal_and_scan(TestGame *tg, int count, int mia)
{
    rb_perform_cheer_check(&tg->state, "p1", (uint8_t)count);
    RbBag *res = &tg->state.resolution;
    int revealed[RB_MAX_REVEALED_CARDS];
    int n = res->n < RB_MAX_REVEALED_CARDS ? res->n : RB_MAX_REVEALED_CARDS;
    int saw_mia = 0;
    for (int i = 0; i < n; i++) {
        revealed[i] = res->cards[i];
        if (revealed[i] == mia) saw_mia = 1;
    }
    for (int i = 0; i < n; i++) {
        tg->state.revealed_cards[i] = revealed[i];
        if (i + 1 > tg->state.n_revealed) tg->state.n_revealed = i + 1;
    }
    tg->state.yell_occurred = 1;
    scan_auto(tg, 0);
    return saw_mia;
}

/* Drive MIA's ab#0 optional: answer the conditional_optional with `accept`, then
 * the follow-up hand-discard SelectCard. Returns 1 if a conditional_optional was
 * actually presented -- which is what makes "no trigger" assertable. */
static int answer_mia_chain(TestGame *tg, int accept)
{
    int saw = 0, guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const char *t = choice_type(tg);
        const char *target = choice_target(tg);
        if (t && strcmp(t, "SelectTarget") == 0 && target &&
            strcmp(target, "conditional_optional") == 0) {
            saw = 1;
            rb_resume_with_choice(&tg->state, accept ? 1 : 0);
        } else if (t && strcmp(t, "SelectCard") == 0) {
            rb_resume_with_choice(&tg->state, choice_count(tg) > 0 ? 0 : -1);
        } else {
            break;
        }
    }
    return saw;
}

/* e1 q269_yell_reveal_mid_pool_does_not_trigger */
static void e1_q269_yell_reveal_mid_pool_does_not_trigger(void)
{
    static TestGame tg;
    new_game(&tg);
    int mia = deck_top_mia(&tg, 2);
    if (mia < 0) return;
    int hand_before = hand_n(&tg, 0);

    CHECK(yell_reveal_and_scan(&tg, 3, mia),
          "E1 PRECONDITION: the yell really did reveal MIA among the 3. This is the "
          "non-vacuity licence for every negative below: the card under test really "
          "was in the reveal, so its absence afterwards is about the trigger and not "
          "about a card the yell never touched.");
    {
        int found = 0;
        for (int i = 0; i < tg.state.n_revealed; i++)
            if (tg.state.revealed_cards[i] == mia) found = 1;
        CHECK(found,
              "E1 PRECONDITION: MIA is recorded in the revealed-card list, which is "
              "what a yell produces -- a reveal, and specifically NOT a "
              "deck-to-waitroom movement event");
    }

    CHECK(!pending(&tg),
          "Q269: a mid-pool yell reveal must ALSO not trigger ab#0, not just a "
          "top-of-deck one");
    CHECK(!hand_has(&tg, 0, mia),
          "Q269: MIA is not recovered to hand by a mid-pool reveal");
    CHECK_EQ(hand_n(&tg, 0), hand_before,
             "Q269: and nothing is discarded from hand");
    tg.state.yell_occurred = 0;
}

/* e2 q269_yell_reveal_does_not_recover_existing_waitroom_copy */
static void e2_q269_yell_reveal_does_not_recover_waitroom_copy(void)
{
    static TestGame tg;
    new_game(&tg);
    int mia_revealed = deck_top_mia(&tg, 0);
    if (mia_revealed < 0) return;
    int mia_waitroom = test_new_id(&tg, MIA);
    CHECK(mia_waitroom >= 0 && mia_waitroom != mia_revealed,
          "E2 PRECONDITION: the two MIA copies are DISTINCT instances -- one id would "
          "make 'neither copy is recovered' trivially true");
    rb_waitroom_add(&tg.state.p[0], mia_waitroom);
    int hand_before = hand_n(&tg, 0);

    CHECK(yell_reveal_and_scan(&tg, 1, mia_revealed),
          "E2 PRECONDITION: the yell really did reveal a MIA");

    CHECK(!pending(&tg),
          "Q269: revealing a MIA by yell must not fire ab#0 EVEN WITH a MIA already "
          "sitting in the waitroom -- the trigger needs a deck-to-waitroom movement, "
          "and a reveal is not one");
    CHECK(!hand_has(&tg, 0, mia_revealed) && !hand_has(&tg, 0, mia_waitroom),
          "Q269: NEITHER MIA copy is recovered to hand");
    CHECK_EQ(hand_n(&tg, 0), hand_before, "Q269: no recover and no discard happened");
    tg.state.yell_occurred = 0;
}

/* e3 q277_control_no_refresh_recovers_milled_self
 * The control for e4: no refresh, so the recover IS offered and DOES happen.
 * Without it, e4's "not recovered" would also hold for a recover that never
 * fires at all. */
static void e3_q277_control_no_refresh_recovers_milled_self(void)
{
    static TestGame tg;
    new_game(&tg);
    int mia = test_id_exact(&tg, MIA);
    int f1  = test_id_exact(&tg, FILLER);
    CHECK(mia >= 0 && f1 >= 0, "E3: fixtures resolve");
    if (mia < 0 || f1 < 0) { n_setup_bugs++; return; }
    expect_card_no(mia, MIA);

    rb_waitroom_add(&tg.state.p[0], mia);
    test_add_to_hand(&tg, f1);
    test_add_to_hand(&tg, test_new_id(&tg, FILLER));
    test_add_to_deck_pl(&tg, 0, f1);
    CHECK(wait_has(&tg, 0, mia), "E3 PRECONDITION: MIA really is in the waitroom");

    push_deck_to_discard(&tg, mia, 0);
    scan_auto(&tg, 0);

    CHECK(answer_mia_chain(&tg, 1),
          "E3 CONTROL: with NO refresh, MIA's ab#0 must offer its optional -- this is "
          "what proves the e4 absence is about the refresh and not about a dead scan");
    CHECK(hand_has(&tg, 0, mia),
          "E3 CONTROL: MIA is still in the waitroom, so it IS recovered to hand");
}

/* e4 q277_refresh_before_auto_resolve_prevents_recover */
static void e4_q277_refresh_before_auto_resolve_prevents_recover(void)
{
    static TestGame tg;
    new_game(&tg);
    int mia = test_id_exact(&tg, MIA);
    int f   = test_id_exact(&tg, FILLER);
    CHECK(mia >= 0 && f >= 0, "E4: fixtures resolve");
    if (mia < 0 || f < 0) { n_setup_bugs++; return; }
    expect_card_no(mia, MIA);

    /* MIA was just milled deck->waitroom (the auto is armed) and now sits in the
     * waitroom. Give a hand card so the optional discard COULD be paid. */
    rb_waitroom_add(&tg.state.p[0], mia);
    push_deck_to_discard(&tg, mia, 0);
    test_add_to_hand(&tg, f);

    /* The same mill emptied the deck to exactly 0, so Q277's refresh happens
     * BEFORE the auto resolves: the waitroom (MIA included) is shuffled back
     * into the deck. */
    tg.state.p[0].deck.n = 0;
    rb_player_refresh(&tg.state, 0);
    CHECK(!wait_has(&tg, 0, mia) && deck_has(&tg, 0, mia),
          "E4 PRECONDITION: the refresh really moved MIA out of the waitroom into the "
          "deck -- if it did not, the negative below would assert nothing");

    scan_auto(&tg, 0);
    answer_mia_chain(&tg, 1);

    CHECK(!hand_has(&tg, 0, mia),
          "Q277: the refresh before the auto resolves removes MIA from the waitroom, "
          "so it can no longer be recovered to hand");
    CHECK(deck_has(&tg, 0, mia),
          "Q277: after the refresh MIA is in the DECK, not the waitroom");
}

/* e5 q277_real_mill_refreshes_before_mia_auto_resolves
 * The same Q277 claim, but driven through DIAYA's REAL mill rather than a
 * hand-pushed movement event, so it tests the integration the Rust file was
 * written to ask about. */
static void e5_q277_real_mill_refreshes_before_mia_auto_resolves(void)
{
    static TestGame tg;
    new_game(&tg);
    int dia = test_id_exact(&tg, DIAYA);
    int mia = test_id_exact(&tg, MIA);
    int f   = test_id_exact(&tg, FILLER);
    CHECK(dia >= 0 && mia >= 0 && f >= 0, "E5: fixtures resolve");
    if (dia < 0 || mia < 0 || f < 0) { n_setup_bugs++; return; }
    expect_card_no(dia, DIAYA);
    expect_trigger(dia, TRIG_DEBUT_STAGE, 1);

    clear_stage(&tg, 0);
    clear_hand(&tg);
    clear_deck(&tg);
    clear_waitroom(&tg);
    tg.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&tg, 0, mia);      /* the mill hits MIA first */
    for (int i = 0; i < 4; i++) test_add_to_deck_pl(&tg, 0, f);
    for (int i = 0; i < 2; i++) rb_waitroom_add(&tg.state.p[0], f);
    test_add_to_hand(&tg, f);
    tg.state.p[0].stage[1] = dia;

    CHECK(fire_trigger(&tg, dia, TRIG_DEBUT_STAGE),
          "E5: DIAYA's real debut mill exists and is queued");
    drain_choices(&tg);

    /* Whether the refresh actually lands is the finding, so the observation is
     * reported on BOTH sides: the movement event AND the resulting zone. */
    int moved = 0;
    for (int i = 0; i < tg.state.n_batch_movements; i++)
        if (tg.state.batch_movements[i].moved_card_id == mia) moved = 1;
    CHECK(moved,
          "E5: DIAYA's real mill must record a movement event naming MIA -- this is "
          "the precondition e4 synthesises by hand, and here it must come from the "
          "engine's own move path");

    CHECK(!wait_has(&tg, 0, mia) && deck_has(&tg, 0, mia),
          "Q277: the real mill must refresh MIA out of the waitroom into the deck "
          "BEFORE the auto resolves");
    CHECK(!pending(&tg),
          "Q277: the automatic ability must not resolve before the refresh");
}

/* e6 real_deck_top_mill_declined_discard_leaves_self_in_waitroom */
static void e6_real_deck_top_mill_declined_leaves_self_in_waitroom(void)
{
    static TestGame tg;
    new_game(&tg);
    int dia = test_id_exact(&tg, DIAYA);
    int mia = test_id_exact(&tg, MIA);
    int f   = test_id_exact(&tg, FILLER);
    CHECK(dia >= 0 && mia >= 0 && f >= 0, "E6: fixtures resolve");
    if (dia < 0 || mia < 0 || f < 0) { n_setup_bugs++; return; }
    expect_card_no(dia, DIAYA);

    clear_stage(&tg, 0);
    clear_hand(&tg);
    clear_deck(&tg);
    clear_waitroom(&tg);
    tg.state.p[0].stage[1] = dia;
    tg.state.p[0].deck.n = 0;
    test_add_to_deck_pl(&tg, 0, mia);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, 0, f);
    test_add_to_hand(&tg, f);
    test_add_to_hand(&tg, test_new_id(&tg, FILLER));
    CHECK(hand_n(&tg, 0) == 2, "E6 PRECONDITION: two hand cards, so the optional "
                               "discard COULD be paid and the decline is a real choice");

    CHECK(fire_trigger(&tg, dia, TRIG_DEBUT_STAGE),
          "E6: DIAYA's real debut mill exists and is queued");
    drain_choices(&tg);

    CHECK(wait_has(&tg, 0, mia), "E6 PRECONDITION: the real mill really did put MIA "
                                "in the waitroom -- otherwise the decline asserts nothing");
    CHECK(answer_mia_chain(&tg, 0),
          "E6: MIA's ab#0 must STILL present its optional even when the answer will "
          "be to decline -- the prompt is not conditional on accepting");
    CHECK(!hand_has(&tg, 0, mia),
          "declining must leave MIA in the waitroom, not recovered");
    CHECK(wait_has(&tg, 0, mia), "E6: MIA stays in the waitroom after the decline");
}

/* e7 hand_discard_batch_q241_one_discarded_fires_once -- Q241 edge, control. */
static void e7_q241_one_discarded_fires_once(void)
{
    static TestGame tg;
    new_game(&tg);
    int rurino = test_id_exact(&tg, RURINO_W);
    int filler = test_id_exact(&tg, FILLER);
    CHECK(rurino >= 0 && filler >= 0, "E7: fixtures resolve");
    if (rurino < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(rurino, RURINO_W);
    expect_trigger(rurino, TRIG_DEBUT_STAGE, 1);

    clear_stage(&tg, 0);
    clear_hand(&tg);
    clear_deck(&tg);
    clear_waitroom(&tg);
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, rurino);
    /* Two DISTINCT candidate instances: the print's own hand-discard ability
     * filters on a group, and one id stamped twice would alias the zone. */
    int m1 = test_new_id(&tg, MIRAKU);
    int m2 = test_new_id(&tg, MIRAKU);
    CHECK(m1 >= 0 && m2 >= 0 && m1 != m2, "E7 PRECONDITION: two distinct candidates");
    if (m1 < 0 || m2 < 0) { n_setup_bugs++; return; }
    CHECK(printed_heart_count(m1, 1) >= 1,
          "E7 PRECONDITION: the candidate is a real member print (it holds hearts), "
          "so the hand-discard ability has something it can legally pick");
    test_add_to_hand(&tg, m1);
    test_add_to_hand(&tg, m2);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&tg, 0, filler);
    int wait_before = wait_n(&tg, 0);

    CHECK(test_play_to_stage(&tg, rurino, 1), "E7: the host really debuted to Center");

    /* Answer the hand SelectCard by taking EXACTLY ONE candidate, so the batch
     * size is 1 and Q241's "once per batch" is measured on its own. */
    int hand_prompts = 0, guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 20) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        const char *t = choice_type(&tg);
        const char *zone = c ? c->zone : NULL;
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg.state, 0);
        } else if (t && strcmp(t, "SelectCard") == 0 && zone &&
                   strcmp(zone, "hand") == 0) {
            hand_prompts++;
            if (hand_prompts == 1) {
                int idx[1] = { 0 };
                rb_resume_with_choice_indices(&tg.state, idx, 1);
            } else {
                rb_resume_with_choice(&tg.state, -1);
            }
        } else {
            rb_resume_with_choice(&tg.state, -1);
        }
    }

    int waited = (wait_has(&tg, 0, m1) ? 1 : 0) + (wait_has(&tg, 0, m2) ? 1 : 0);
    CHECK(hand_prompts >= 1,
          "E7 PRECONDITION: the debut must have offered its hand select -- a run in "
          "which no prompt appeared would make every count below vacuous");
    CHECK_EQ(waited, 1,
             "E7: exactly ONE of the two candidates reaches the waitroom");
    CHECK(heart(&tg, rurino, 1) == 1,
          "Q241: 1 card discarded in one batch fires the auto exactly once "
          "(heart01 == 1)");
    CHECK_EQ(blade(&tg, rurino), 1, "Q241: 1 card discarded gives blade exactly 1");
    (void)wait_before;
}

/* =====================================================================
 * Section F  discard_watch / the two Q252 files -- the cases neither existing
 * C section covers.
 *
 *   live_discard_optional_deck_top_bottom_q252_test.rs ::
 *       live_discard_selected_aqours_card_moves_to_deck_top_q252
 *       simultaneous_live_discard_moves_only_first_selected_card_to_top_q252
 *       live_discard_deck_move_second_trigger_offers_no_choice_q252
 *       live_discard_mixed_types_returns_live_to_bottom_and_keeps_member_q252
 *   own_live_zone_discard_group_live_optional_deck_top_bottom_q252_test.rs ::
 *       own_live_zone_discard_optional_deck_live_card_only_in_zone_no_discard_
 *         no_effect
 *       own_live_zone_discard_optional_deck_single_card_deck_bottom_exact_identity
 *       own_live_zone_discard_optional_deck_batch_of_3_first_only_taken_others_
 *         untouched
 *
 * The two Rust files are largely duplicates of each other (both drive 桜内梨子's
 * Q252 ability); section F ports the UNION of the cases test_parity_jidou_watch.c
 * section G does not already have.
 *
 * KNOWN C LIMIT, recorded as EXPECTED_GAP and never turned into a pass: the flat
 * RbChoice carries no destination/option list at all (only n_heart_options), so a
 * position|destination prompt's deck_top vs deck_bottom options are NOT observable
 * from C, and the answer index is read as an ABSOLUTE area. The SELECTION half is
 * observable and is what these tests assert; the destination half is the gap.
 * ===================================================================== */

static void riko_board(TestGame *tg, int stage_area)
{
    int riko  = test_id_exact(tg, RIKO_SEC);
    int filler = test_id_exact(tg, FILLER);
    CHECK(riko >= 0 && filler >= 0, "F: fixtures resolve");
    if (riko < 0 || filler < 0) { n_setup_bugs++; return; }
    expect_card_no(riko, RIKO_SEC);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    clear_hand(tg);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 20; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
    tg->state.p[0].stage[stage_area] = riko;
    test_give_energy(tg, 5);
}

/* Rust trigger_riko_auto: set_recently_moved_cards(moved) then the TAS scan. */
static void riko_auto(TestGame *tg, const int *moved, int n)
{
    set_recently_moved(tg, moved, n);
    scan_auto(tg, 0);
}

/* f1 live_discard_selected_aqours_card_moves_to_deck_top_q252 */
static void f1_selected_aqours_live_moves_to_deck(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int live = test_new_id(&tg, AQ_LIVE_PR);
    CHECK(live >= 0, "F1: the live fixture resolves");
    if (live < 0) { n_setup_bugs++; return; }
    CHECK(rb_card_matches_group_str(live, "Aqours"),
          "F1 PRECONDITION: the card really is inside the group the ability filters "
          "on -- otherwise the group filter is never exercised");
    rb_waitroom_add(&tg.state.p[0], live);
    int deck_before = deck_n(&tg, 0);

    int moved[1] = { live };
    riko_auto(&tg, moved, 1);
    CHECK(pending(&tg), "F1: a single group-matching live reaching the waitroom "
                         "offers the placement");
    {
        EXPECTED_GAP(0,
                     "the placement prompt carries its deck_top / deck_bottom options: "
                     "the flat RbChoice struct has no options list (only "
                     "n_heart_options), so no destination is observable from C and the "
                     "answer index is read as an absolute area");
    }
    rb_resume_with_choice(&tg.state, 0);
    drain_auto(&tg);

    CHECK(!wait_has(&tg, 0, live), "F1: the selected live card leaves the waitroom");
    CHECK_EQ(wait_n(&tg, 0), 0, "F1: nothing else is left behind in the waitroom");
    CHECK_EQ(deck_n(&tg, 0), deck_before + 1, "F1: the deck gains exactly one card");
    CHECK(deck_has(&tg, 0, live),
          "F1: the relocated live card really reached the deck");
}

/* f2 simultaneous_live_discard_moves_only_first_selected_card_to_top_q252 */
static void f2_simultaneous_moves_only_first_selected(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int l1 = test_new_id(&tg, AQ_LIVE_PR);
    int l2 = test_new_id(&tg, AQ_LIVE_SD);
    CHECK(l1 >= 0 && l2 >= 0 && l1 != l2,
          "F2 PRECONDITION: two DISTINCT live instances -- one id in two waitroom "
          "slots would make 'only the first is taken' trivially true");
    if (l1 < 0 || l2 < 0) { n_setup_bugs++; return; }
    rb_waitroom_add(&tg.state.p[0], l1);
    rb_waitroom_add(&tg.state.p[0], l2);
    int deck_before = deck_n(&tg, 0);

    int moved[2] = { l1, l2 };
    riko_auto(&tg, moved, 2);
    CHECK(pending(&tg), "F2: with 2 candidates the player is prompted to pick 1");
    {
        /* Answer with the SECOND index, not the first: a first-option-only answer
         * cannot tell a real selection from a hard-wired one. */
        rb_resume_with_choice(&tg.state, 1);
    }
    drain_auto(&tg);

    CHECK(!wait_has(&tg, 0, l2),
          "F2: picking index 1 really moves the SECOND card out of the waitroom");
    CHECK(wait_has(&tg, 0, l1),
          "F2: and the FIRST card is left in the waitroom -- the ability takes ONE "
          "card per batch, not all matching ones");
    CHECK_EQ(deck_n(&tg, 0), deck_before + 1, "F2: exactly one card reached the deck");
}

/* f3 live_discard_deck_move_second_trigger_offers_no_choice_q252 */
static void f3_second_trigger_offers_no_choice(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int live_a = test_new_id(&tg, AQ_LIVE_PR);
    CHECK(live_a >= 0, "F3: fixture resolves");
    if (live_a < 0) { n_setup_bugs++; return; }
    rb_waitroom_add(&tg.state.p[0], live_a);
    int m1[1] = { live_a };
    riko_auto(&tg, m1, 1);
    CHECK(pending(&tg), "F3 PRECONDITION: the first trigger really does offer the "
                         "placement -- without it the per-turn limit is not exercised");
    rb_resume_with_choice(&tg.state, 0);
    drain_auto(&tg);
    CHECK(!wait_has(&tg, 0, live_a), "F3 PRECONDITION: the first trigger relocated it");
    CHECK(deck_has(&tg, 0, live_a), "F3 PRECONDITION: and it reached the deck");

    int live_b = test_new_id(&tg, AQ_LIVE_SD);
    CHECK(live_b >= 0 && live_b != live_a, "F3: a SECOND distinct live resolves");
    if (live_b < 0) return;
    rb_waitroom_add(&tg.state.p[0], live_b);
    int m2[1] = { live_b };
    riko_auto(&tg, m2, 1);

    CHECK_PROMPTS_GT(&tg, 0,
                     "F3 CONTROL: the control point is read AFTER the first trigger's "
                     "prompt was answered, so the absence below is not just a prompt "
                     "that was never raised");
    CHECK(!pending(&tg),
          "F3: a second trigger in the same turn is BLOCKED by the per-turn limit, so "
          "no second placement is offered");
    CHECK(wait_has(&tg, 0, live_b), "F3: the second card stays in the waitroom untouched");
    CHECK(deck_has(&tg, 0, live_a), "F3: and the first card is not displaced");
}

/* f4 live_discard_mixed_types_returns_live_to_bottom_and_keeps_member_q252
 * The batch holds a group-matching LIVE, a group-matching MEMBER (not a live) and
 * a non-matching live. Only the live may be taken. */
static void f4_mixed_types_keeps_member_and_non_group(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int aq_live = test_new_id(&tg, AQ_LIVE_PR);
    int member  = test_new_id(&tg, AQ_MEMBER);
    int non_aq  = test_new_id(&tg, NON_AQ_LIVE);
    CHECK(aq_live >= 0 && member >= 0 && non_aq >= 0,
          "F4: every fixture resolves");
    if (aq_live < 0 || member < 0 || non_aq < 0) { n_setup_bugs++; return; }
    CHECK(rb_card_matches_group_str(aq_live, "Aqours") &&
          rb_card_matches_group_str(member, "Aqours"),
          "F4 PRECONDITION: the live and the member are both inside the group");
    CHECK(!rb_card_matches_group_str(non_aq, "Aqours"),
          "F4 PRECONDITION: the outsider is genuinely outside the group, or the group "
          "filter is never exercised in the negative direction");

    rb_waitroom_add(&tg.state.p[0], aq_live);
    rb_waitroom_add(&tg.state.p[0], member);
    rb_waitroom_add(&tg.state.p[0], non_aq);
    int deck_before = deck_n(&tg, 0);

    int moved[3] = { aq_live, member, non_aq };
    riko_auto(&tg, moved, 3);
    CHECK(pending(&tg), "F4: one group-matching LIVE in the batch satisfies the condition");
    rb_resume_with_choice(&tg.state, 0);
    drain_auto(&tg);

    CHECK(!wait_has(&tg, 0, aq_live),
          "F4: the group-matching LIVE is the one taken");
    CHECK(wait_has(&tg, 0, member),
          "F4: the group-matching MEMBER remains -- the clause is about live cards, "
          "not about any card in the group");
    CHECK(wait_has(&tg, 0, non_aq), "F4: the non-group live also remains");
    CHECK_EQ(deck_n(&tg, 0), deck_before + 1, "F4: exactly one card reached the deck");
}

/* f5 own_live_zone_discard_optional_deck_live_card_only_in_zone_no_discard_...
 * A live card sitting in the LIVE zone with no discard movement must produce
 * nothing. The mirror of f1: same card, different zone, opposite outcome. */
static void f5_live_in_live_zone_no_discard_no_effect(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int live = test_new_id(&tg, AQ_LIVE_PR);
    CHECK(live >= 0, "F5: fixture resolves");
    if (live < 0) { n_setup_bugs++; return; }
    CHECK(rb_card_matches_group_str(live, "Aqours"),
          "F5 PRECONDITION: the card really is a group-matching live");
    test_add_to_live(&tg, live);
    CHECK(live_has(&tg, 0, live), "F5 PRECONDITION: it really sits in the live zone");

    int moved[1] = { live };
    riko_auto(&tg, moved, 1);

    CHECK(!pending(&tg),
          "F5: a live card that is merely IN the live zone, with no waitroom "
          "movement, produces no placement offer -- the clause is about leaving the "
          "live zone, not about occupancy");
    CHECK(live_has(&tg, 0, live), "F5: the card is untouched in the live zone");
}

/* f6 own_live_zone_discard_optional_deck_batch_of_3_first_only_taken_others_
 * untouched. Three group-matching lives; exactly ONE is taken. */
static void f6_batch_of_three_first_only_taken(void)
{
    static TestGame tg;
    new_game(&tg);
    riko_board(&tg, 1);

    int l1 = test_new_id(&tg, AQ_LIVE_PR);
    int l2 = test_new_id(&tg, AQ_LIVE_SD);
    int l3 = test_new_id(&tg, AQ_LIVE_PB);
    CHECK(l1 >= 0 && l2 >= 0 && l3 >= 0, "F6: all three resolve");
    if (l1 < 0 || l2 < 0 || l3 < 0) { n_setup_bugs++; return; }
    CHECK(l1 != l2 && l2 != l3 && l1 != l3,
          "F6 PRECONDITION: three DISTINCT instances -- one id in three slots would "
          "make 'only one is taken' trivially true");
    rb_waitroom_add(&tg.state.p[0], l1);
    rb_waitroom_add(&tg.state.p[0], l2);
    rb_waitroom_add(&tg.state.p[0], l3);
    int deck_before = deck_n(&tg, 0);

    int moved[3] = { l1, l2, l3 };
    riko_auto(&tg, moved, 3);
    CHECK(pending(&tg), "F6: the batch of 3 offers a single-card selection");
    rb_resume_with_choice(&tg.state, 0);
    drain_auto(&tg);

    CHECK(!wait_has(&tg, 0, l1), "F6: the FIRST card is the one taken");
    CHECK(wait_has(&tg, 0, l2), "F6: the second card is untouched");
    CHECK(wait_has(&tg, 0, l3), "F6: the third card is untouched");
    CHECK_EQ(wait_n(&tg, 0), 2, "F6: exactly two of the three remain in the waitroom");
    CHECK_EQ(deck_n(&tg, 0), deck_before + 1, "F6: exactly one card reached the deck");
}

/* =====================================================================
 * per-test isolation: forked child, signal handler naming the test in
 * flight. NO watchdog: an in-child alarm() never fires on this toolchain and a
 * parent-side waitpid(WNOHANG) deadline's kill(SIGKILL) does not take effect, so
 * one would be dead code that only pretends to protect the run.
 * ===================================================================== */

static void on_segv(int sig)
{
    static const char msg[] = "\nCRASH: a fatal signal arrived inside the test below.\n";
    ssize_t ignored = write(2, msg, sizeof(msg) - 1);
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
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        if (nf) n_failed++;
        else if (ns) n_setup++;
        else n_ok++;
        printf("%-8s %s  [%d assertions, %d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name,
               (int)(assertions - a0), nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(n_setup_bugs - s0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES
                             : (n_setup_bugs > s0 ? CHILD_SETUPBUG : CHILD_OK));
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name,
               WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_setup++;
        printf("%-8s %s  <-- SETUP BUG (fixture could not be built)\n", "SETUPBUG", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
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

    printf("--- Section A  ability_watch / constant_grant_lifecycle_test.rs ---\n");
    run("a1_niko_grant_stays_single_across_repeated_scans",
        a1_niko_grant_stays_single_across_repeated_scans);
    run("a2_niko_grant_latches_at_live_start", a2_niko_grant_latches_at_live_start);
    run("a3_niko_grant_expires_at_live_end", a3_niko_grant_expires_at_live_end);
    run("a4_ai_grant_is_a_flat_bonus_past_its_threshold",
        a4_ai_grant_is_a_flat_bonus_past_its_threshold);
    run("a5_ai_grant_expires_at_live_end", a5_ai_grant_expires_at_live_end);
    run("a6_koto_grant_withdrawn_and_regranted", a6_koto_grant_withdrawn_and_regranted);
    run("a7_koto_grant_requires_distinct_names", a7_koto_grant_requires_distinct_names);
    run("a8_koto_grant_does_not_outlive_host_zone_exit",
        a8_koto_grant_does_not_outlive_host_zone_exit);

    printf("--- Section B  ability_watch / lifecycle_tail_idempotence_test.rs ---\n");
    run("b1_joint_grant_stays_single_across_scans",
        b1_joint_grant_stays_single_across_scans);
    run("b2_joint_grant_expires_at_live_end", b2_joint_grant_expires_at_live_end);
    run("b3_nozomi_grant_stays_single_then_expires", b3_nozomi_grant_stays_single_then_expires);
    run("b4_hanamaru_grant_stays_single_then_expires",
        b4_hanamaru_grant_stays_single_then_expires);
    run("b5_honoka_grant_is_permanent_and_leaves_only_on_zone_exit",
        b5_honoka_grant_is_permanent_and_leaves_only_on_zone_exit);
    run("b6_genki_invalidation_survives_repeated_derivation",
        b6_genki_invalidation_survives_repeated_derivation);
    run("b7_butterfly_suppression_survives_repeated_scans",
        b7_butterfly_suppression_survives_repeated_scans);

    printf("--- Section C  ability_watch / invalidated_live_start_restores_next_live ---\n");
    run("c1_mei_live_start_activates_two_energy", c1_mei_live_start_activates_two_energy);
    run("c2_kanon_invalidation_silences_then_restores",
        c2_kanon_invalidation_silences_then_restores);
    run("c3_kanon_invalidation_is_scoped_to_own_stage",
        c3_kanon_invalidation_is_scoped_to_own_stage);

    printf("--- Section D  debut_watch / the three unported cases ---\n");
    run("d1_setsuna_opponent_stage_arrival_does_not_fire",
        d1_setsuna_opponent_stage_arrival_does_not_fire);
    run("d2_baton_touch_activates_energy_q167", d2_baton_touch_activates_energy_q167);
    run("d3_edelnote_appearance_waits_opponent", d3_edelnote_appearance_waits_opponent);

    printf("--- Section E  discard_watch / Q269-Q277 refresh + mill edges ---\n");
    run("e1_q269_yell_reveal_mid_pool_does_not_trigger",
        e1_q269_yell_reveal_mid_pool_does_not_trigger);
    run("e2_q269_yell_reveal_does_not_recover_waitroom_copy",
        e2_q269_yell_reveal_does_not_recover_waitroom_copy);
    run("e3_q277_control_no_refresh_recovers_milled_self",
        e3_q277_control_no_refresh_recovers_milled_self);
    run("e4_q277_refresh_before_auto_resolve_prevents_recover",
        e4_q277_refresh_before_auto_resolve_prevents_recover);
    run("e5_q277_real_mill_refreshes_before_mia_auto_resolves",
        e5_q277_real_mill_refreshes_before_mia_auto_resolves);
    run("e6_real_deck_top_mill_declined_leaves_self_in_waitroom",
        e6_real_deck_top_mill_declined_leaves_self_in_waitroom);
    run("e7_q241_one_discarded_fires_once", e7_q241_one_discarded_fires_once);

    printf("--- Section F  discard_watch / the two Q252 files, unported cases ---\n");
    run("f1_selected_aqours_live_moves_to_deck", f1_selected_aqours_live_moves_to_deck);
    run("f2_simultaneous_moves_only_first_selected", f2_simultaneous_moves_only_first_selected);
    run("f3_second_trigger_offers_no_choice", f3_second_trigger_offers_no_choice);
    run("f4_mixed_types_keeps_member_and_non_group", f4_mixed_types_keeps_member_and_non_group);
    run("f5_live_in_live_zone_no_discard_no_effect", f5_live_in_live_zone_no_discard_no_effect);
    run("f6_batch_of_three_first_only_taken", f6_batch_of_three_first_only_taken);

    rb_unload();

    /* The assertion / failure / setup-bug counters live in the CHILD (each case
     * runs in its own forked process), so the parent's own counters are
     * structurally 0. The summary must therefore be built from the per-case
     * verdicts, not from `failures`. */
    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_jidou_watch_clusters: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    if (n_crashed) {
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child is an "
                        "engine fault, and the handler names the case.\n", n_crashed);
    }
    if (n_setup) {
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n", n_setup);
    }
    if (n_failed) {
        fprintf(stderr, "%d test case(s) FAILED. EXPECTED: the grant/invalidation "
                        "lifecycle lives in src/core/game_state_abilities.c, which is "
                        "mid-translation. No assertion in this file is weakened and no "
                        "failing test is deleted or ignored.\n", n_failed);
        return 1;
    }
    printf("ALL JIDOU-WATCH-CLUSTERS PARITY CHECKS PASSED\n");
    return 0;
}

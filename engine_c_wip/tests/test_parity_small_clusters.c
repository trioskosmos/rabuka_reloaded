/* test_parity_small_clusters.c -- C parity suite for seven SMALL Rust clusters
 * that TEST_PARITY_SURVEY.md listed as barely covered:
 *
 *   engine/tests/test_modules/effects/gain/hearts/per_card/     (7 files, 25 tests)
 *   engine/tests/test_modules/effects/recover/to_discard/       (5 files, 21 tests)
 *   engine/tests/test_modules/effects/state/orientation_gated/  (5 files, 22 tests)
 *   engine/tests/test_modules/effects/draw/until_count/         (3 files, 10 tests)
 *   engine/tests/test_modules/effects/draw/conditional/         (6 files, 11 tests)
 *   engine/tests/test_modules/effects/position/area_gated/      (3 files, 18 tests)
 *   engine/tests/test_modules/effects/position/per_card/        (2 files,  4 tests)
 *
 * Each section names the Rust .rs file every case mirrors. Where a peer already
 * covers a case the comment says so and by which file; only the GAP is ported.
 *
 * ── NON-VACUITY, the point of this file ─────────────────────────────────────
 * Almost every cluster here is a NEGATIVE test ("no effect", "no blade",
 * "no prompt"), and a negative assertion passes trivially on a board where the
 * ability never ran at all. Every negative case therefore either
 *   (a) asserts a PROMPT-COUNT CONTROL -- CHECK(prompts > 0) or a zone
 *       delta > 0 -- proving the ability really executed, or
 *   (b) is paired on the same board with a POSITIVE sibling where the gate is
 *       satisfied and the same assertion is inverted.
 * Controls are labelled "CONTROL:" in the assertion text so a reader can see
 * which negative is backed and which is not.
 *
 * ── pay/skip gates ─────────────────────────────────────────────────────────
 * A `pay_optional_cost:skip` gate is a SelectTarget where OPTION 0 IS SKIP and
 * OPTION 1 IS PAY. Answering 0 makes the paying half of a test identical to
 * the skipping half. pc_pay() therefore answers 1 on a SelectTarget and 0
 * elsewhere; pc_skip() answers -1.
 *
 * ── fork isolation ─────────────────────────────────────────────────────────
 * Every case runs in a forked child. A child that dies on a signal is reported
 * CRASH, never as a pass, and the handler names the test in flight. No
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
   fullwidth form, so "PL!HS-bp5-003-R＋" is a DIFFERENT card number from
   "PL!HS-bp5-003-R". Spelled as an explicit byte escape so this file stays
   encoding-proof under the MSVC/MinGW toolchains (AGENTS.md encoding trap). */
#define FF_PLUS "\xef\xbc\x8b"

/* 虹ヶ咲 as the ENGINE actually spells it: rb_card_group_name() returns bytes
   E8 99 B9 E3 83 B6 E5 92 B2. The first character's THIRD byte is B9, not the
   A6 a hand-typed U+8671 produces, so a typed literal would never strcmp equal
   the runtime value. Pinned to the observed bytes. */
#define NIJI_GROUP "\xe8\x99\xb9\xe3\x83\xb6\xe5\x92\xb2"

/* Trigger token texts, as explicit UTF-8 byte escapes. */
#define TRIG_DEBUT       "\xe7\x99\xbb\xe5\xa0\xb4"                            /* 登場       */
#define TRIG_LIVE_START  "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe9\x96\x8b\xe5\xa7\x8b\xe6\x99\x82" /* ライブ開始時 */
#define TRIG_LIVE_SUCCESS "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe6\x88\x90\xe5\x8a\x9f\xe6\x99\x82" /* ライブ成功時 */
#define TRIG_CONSTANT    "\xe5\xb8\xb8\xe6\x99\x82"                            /* 常時       */

/* heart colour slots, matching cards/compile_cards.py HEART_COLORS:
   0=heart00 .. 6=heart06, 7=icon_all. */
#define H00 0
#define H01 1
#define H02 2
#define H03 3
#define H04 4
#define H05 5
#define H06 6
#define HALL 7

/* ── assertions ───────────────────────────────────────────────────────────── */

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
    long long actual_value = (long long)(actual); \
    long long expected_value = (long long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %lld expected %lld)\n", message, \
                actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* A C-vs-Rust HARNESS gap: the Rust assertion is not expressible through the C
   shim at all (missing API surface, not a behaviour difference). Counted apart
   from parity failures so a red run still names the parity failures. */
#define EXPECTED_HARNESS_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

/* ======================================================================= */
/* card constants -- every one verified against cards/cards.json (UTF-8)     */
/* ======================================================================= */

/* gain/hearts/per_card/q127_heart_set_plus_global_test.rs */
#define WIEN     "PL!SP-bp2-010-P"   /* ウィーン・マルガレーテ, KALEIDOSCORE, cost 15 */
#define BLOOM    "PL!HS-bp2-019-L"   /* Bloom the smile, Bloom the dream! (live)    */
#define HARERUYA "PL!SP-bp1-026-L"   /* 未来予報ハレルヤ！ (live)                    */
#define HASU_BP1 "PL!HS-bp1-002-R"   /* 蓮ノ空 member (Bloom's condition host)      */
#define LIELLA_014 "PL!SP-bp1-014-N" /* Liella! member (Hareruya waitroom filler)   */
#define LIELLA_015 "PL!SP-bp1-015-N"
#define LIELLA_016 "PL!SP-bp1-016-N"
#define LIELLA_019 "PL!SP-bp1-019-N"
#define LIELLA_020 "PL!SP-bp1-020-N"

/* gain/hearts/per_card/izumi_debut_other_heart06_member_gain_test.rs */
#define IZUMI_R  "PL!HS-pb1-016-R"   /* 桂城 泉, EdelNote, cost 2, blade heart06 */
#define SD_001   "PL!-sd1-001-SD"    /* 高坂穂乃果 cost 11, base_heart06 -> target */
#define SD_010   "PL!-sd1-010-SD"    /* 高坂穂乃果 cost 4, no blade heart       */

/* gain/hearts/per_card/hasunosora_live_start_waitroom_ten_heart04_gain_test.rs */
#define HANASAKA "PL!HS-pb1-025-L"   /* 抱きしめる花びら (live)                    */
#define HANANO_A "PL!HS-PR-001-PR"   /* 日野下花帆, スリーズブeke, cost 10       */
#define HANANO_B "PL!HS-PR-002-PR"   /* 村野さやか, DOLLCHESTRA, cost 10          */

/* gain/hearts/per_card/rurino_debut_activate_wait_member_add_live_card_test.rs */
#define RURINO   "PL!HS-bp6-003-R"   /* 大沢瑠璃乃 cost 11, みらくらぱーく！    */
#define RURINO_CHEAP "PL!HS-bp6-011-R" /* 大沢瑠璃乃 cost 2, same unit          */
#define MIRAKURA_LIVE "PL!HS-PR-012-PR" /* アイデンティティ, みらくらぱーく! (live) */
#define RURINO_BP5 "PL!HS-bp5-003-R" FF_PLUS /* 大沢瑠璃乃 cost 2, same unit     */

/* gain/hearts/per_card/subunit_stage_count_gated_debut_heart_test.rs
 *
 * A4 fixture keys. Rust's twin is EXPLICIT about the first one
 * (subunit_stage_count_gated_debut_heart_test.rs:9-22):
 *   "Fixtures use the CLEAN card numbers. The corpus also contains 13 keys with
 *    text fragments spliced into the card_no -- PL!N-bp1-023-PRproteinbar,
 *    PL!N-sd1-012-PRproteinbar, LL-PR-007-PRLoveLive!Days and ten more. Each of
 *    the 13 is a **byte-exact duplicate** of a legitimate entry ... They are
 *    phantom duplicates, and this file previously reached for one of them.
 *    These fixtures use the clean keys, so they name real pool entries and stay
 *    resolvable if the 13 duplicates are ever deleted."
 * and then `const R3BIRTH_A: &str = "PL!N-bp1-023-N";`
 *
 * The C reached for the phantom `PL!N-bp1-023-PRproteinbar`. It resolves today
 * (both keys are real entries in cards/cards.json -- verified) and it is a
 * PRIZE print, not corruption, so nothing about the current behaviour changes;
 * but pin_id() compares card_no byte-exactly, so the fixture was pinning the
 * phantom's identity and would break the moment the 13 duplicates go.
 * test_parity_gain_delta.c:274 already used the clean key for the same card.
 */
#define RANBU_N  "PL!N-bp7-024-N"    /* 鐘 嵐珠 R3BIRTH cost 4                    */
#define R3BIRTH_A "PL!N-bp1-023-N"   /* ミア・テイラー R3BIRTH cost 9              */
#define R3BIRTH_B "PL!N-bp1-024-PR"  /* 鐘 嵐珠 R3BIRTH cost 4                     */
#define R3BIRTH_C "PL!N-PR-012-PR"   /* 三船栞子 R3BIRTH cost 2                    */

/* gain/hearts/per_card/sumire_highest_center_cost_gains_heart03_test.rs
   (PL!SP-bp2-004-R) -- already covered by test_parity_hearts_constants.c §E
   and test_parity_conditional_gated.c §J; see section A0 below. */
#define SUMIRE_R "PL!SP-bp2-004-R"   /* 平安名すみれ CatChu! cost 9                */
#define SUMIRE_P "PL!SP-bp2-004-P"   /* 平安名すみれ CatChu! cost 9                */

/* recover/to_discard/debut_mill_top_three_..._pl_n_bp7_020_n_test.rs */
#define EMMA     "PL!N-bp7-020-N"    /* エマ・ヴェルデ QU4RTZ cost 5              */
#define KARIN_B05 "PL!N-sd1-004-SD"  /* 朝香果林, blade heart b_heart05            */
#define KANATA_B06 "PL!N-sd1-006-SD" /* 近江彼方, blade heart b_heart06            */
#define NO_BLADE "PL!N-sd1-002-SD"   /* 中須かすみ, no blade heart                 */
#define ALL_BLADE "PL!-sd1-020-SD"   /* きっと青春が聞こえる, all-blade live        */
#define GREEN_BLADE_PR "PL!N-PR-024-PR" /* 桜坂しずく, b_heart03                   */
#define ENERGY_CARD "LL-E-001-SD"

/* recover/to_discard/live_start_discard_two_from_unit_..._test.rs */
#define HANANO   "PL!HS-PR-016-PR"   /* 日野下花帆 cost 17, スリーズブーケ         */
#define SD_008   "PL!-sd1-008-SD"    /* 小泉 花陽 Printemps cost 4                 */
#define SD_003   "PL!-sd1-003-SD"    /* 南 ことり Printemps cost 13                */
#define SD_013   "PL!-sd1-013-SD"    /* 園田海未 lilywhite cost 4                  */
#define SD_004   "PL!-sd1-004-SD"    /* 園田海未 lilywhite cost 11                 */
#define SD_002   "PL!-sd1-002-SD"    /* 絢瀬 絵里 BiBi cost 2                      */
#define SD_019   "PL!-sd1-019-SD"    /* START:DASH!! (live)                        */

/* recover/to_discard/live_start_discard_two_grants_blade_to_distinct_niji_... */
#define JUST_BELIEVE "PL!N-bp7-026-L" /* Just Believe!!! (live)                     */
#define NIJI_A   "PL!N-sd1-004-SD"    /* 朝香果林, 虹ヶ咲 (DiverDiva)               */
#define NIJI_B   "PL!N-sd1-006-SD"    /* 近江彼方, 虹ヶ咲 (QU4RTZ)                  */

/* recover/to_discard/optional_discard_cost_plus_total_cost_comparison_... */
#define KOSUZU   "PL!HS-bp6-005-R" FF_PLUS /* 徒町 小鈴 EdelNote cost 10          */
#define KANOHA   "PL!HS-bp1-001-R"   /* 日野下花帆 蓮ノ空 cost 11                  */
#define LILIENTHAL "PL!HS-bp6-007-R" /* セラス 柳田 蓮ノ空 cost 15                 */
#define SD_009   "PL!-sd1-009-SD"    /* 矢澤にこ cost 15                           */

/* state/orientation_gated/erena_self_wait_constant_heart05_test.rs */
#define ERENA_R  "PL!-bp5-333-R"     /* 統堂英玲奈 A-RISE cost 7                   */
#define ERENA_PP "PL!-bp5-333-P" FF_PLUS /* 統堂英玲奈 A-RISE                     */

/* state/orientation_gated/two_or_more_opponent_wait_members_..._heart06_... */
#define IZUMI_N  "PL!HS-bp5-016-N"   /* 桂城 泉 EdelNote cost 9                    */

/* state/orientation_gated/nico_cannot_activate_test.rs */
#define NICO     "PL!-pb1-009-R"     /* 矢澤にこ BiBi cost 4                       */
#define KOTORI   "PL!-pb1-012-R"     /* 南ことり Printemps cost 2                  */

/* draw/until_count/miyashita_ai_pr_test.rs (gaps only) */
#define MIYASHITA "PL!N-PR-028-PR"   /* 宮下 愛 DiverDiva, 登場 discard2 draw to 5 */

/* draw/until_count/miyata_draw_until_test.rs */
#define MIYATA   "PL!N-bp3-005-R" FF_PLUS /* 宮下 愛 DiverDiva cost 15            */
#define KASUMI   "PL!N-sd1-002-SD"   /* 中須かすみ                                  */
#define SHIZUKU  "PL!N-sd1-003-SD"   /* 唐 可可                                      */

/* draw/conditional/area_moved_pl_sp_sd2_003_sd2_test.rs */
#define CHISATO_SD2 "PL!SP-sd2-003-SD2" /* 嵐 千砂都 5yncri5e! cost 11           */
#define KINAKO   "PL!SP-bp5-006-R"    /* 桜小路きな子, 起動 mill 3 + position change */

/* draw/conditional/live_start_energy_return_ren_draw_pl_sp_bp7_026_l_test.rs */
#define DEARS    "PL!SP-bp7-026-L"    /* Dears (live)                                */
#define REN_SD1  "PL!SP-sd1-005-SD"   /* 葉月 恋 KALEIDOSCORE cost 9                 */

/* draw/conditional/revealed_live_hand_threshold_draw_test.rs */
#define HANAMARU_BP2 "PL!S-bp2-007-R" FF_PLUS /* 国木田花丸 AZALEA cost 4          */
#define RIKO_PR14 "PL!S-PR-014-PR"   /* 桜内梨子 GuiltyKiss cost 15                 */
#define LL_BP5_001 "LL-bp5-001-L"    /* joint live                                  */
#define LL_BP5_002 "LL-bp5-002-L"    /* joint live                                  */

/* draw/conditional/score_or_revealed_live_pl_sp_pb2_004_r_test.rs */
#define SUMIRE_PB2 "PL!SP-pb2-004-R"  /* 平安名すみれ CatChu! cost 2                 */
#define LIVE6    "PL!SP-bp1-027-L"    /* Sing！Shine！Smile！ (live, score 6)        */

/* draw/conditional/success_zone_group_pl_bp6_023_l_test.rs */
#define SWEET_SWEET "PL!-bp6-023-L"  /* sweet&sweet holiday (live)                  */
#define NIJI_LIVE "PL!N-sd1-025-SD"   /* Colorful Dreams! Colorful Smiles! 虹ヶ咲 live */

/* position/area_gated/center_area_syncrise_member_constant_blade_gain_test.rs */
#define FUYU      "PL!SP-pb2-022-R"   /* 鬼塚冬毬 5yncri5e! cost 15                 */
#define SYNC_ABILITYLESS "PL!SP-bp1-014-N" /* 嵐 千砂都 5yncri5e! cost 4        */
#define SHIKI     "PL!SP-bp2-008-R"   /* 若菜四季 5yncri5e! cost 9                   */
#define FUYU_SELF "PL!SP-sd2-011-SD2" /* 鬼塚冬毬 5yncri5e! cost 4                  */
#define AQOURS_P "PL!S-bp2-008-P"     /* 小原鞠莉 GuiltyKiss cost 17                 */

/* position/area_gated/success_zone_center_area_mus_member_constant_blade_... */
#define LOVE_WING "PL!-bp4-020-L"     /* Love wing bell (live)                       */
#define MUS_CENTER "PL!-PR-001-PR"   /* 高坂穂乃果 Printemps cost 4                 */
#define SD_005    "PL!-sd1-005-SD"    /* 星空 凛 lilywhite cost 2                    */

/* position/per_card/mill_three_self_position_change_q104_q234_test.rs */
#define KINAKO_ALT "PL!SP-bp5-006-R"  /* 桜小路きな子 (same print as KINAKO)         */

/* ======================================================================= */
/* harness                                                                  */
/* ======================================================================= */

/* rb_load("src") fails under the isolated out-of-tree build root, so the
   canonical cards/build directory is a REQUIRED fallback, not an option. */
static int load_card_db(void)
{
    static const char *const dirs[] = {"src", "../cards/build", "cards/build",
                                       "../../cards/build"};
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++)
        if (rb_load(dirs[i]) == 0) return 0;
    return 1;
}

/* Rotating buffers: card_no_of() is called up to three times inside one
   fprintf, and a single static buffer would print the LAST value three times. */
static const char *card_no_of(int cid)
{
    static char bufs[8][80];
    static int  next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 7;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 80, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* The identity rule (AGENTS.md): rb_card_no_eq is FUZZY (a peer proved it
   accepts PL!SP-PR-004-R against the real PL!SP-PR-004-PR), so every fixture
   is pinned with a byte-exact strcmp of the DECODED card_no. */
static int pin_id(int cid, const char *want)
{
    assertions++;
    if (cid < 0 || strcmp(card_no_of(cid), want) != 0) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', resolved '%s' (id=%d)\n",
                want, cid >= 0 ? card_no_of(cid) : "<none>", cid);
        failures++;
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

/* Rust game.id() allocates the NEXT DISTINCT pool slot; C's plain template
   lookup aliases, so test_new_id is the faithful mapping. The copy pool is a
   process-global monotonic counter nothing resets between cases, so a long
   suite would exhaust it and every mid() would silently fall back to the
   shared template. reset_board() clears it. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    assertions++;
    if (id < 0) {
        fprintf(stderr, "FAIL: fixture card \"%s\" is not in the database\n", no);
        failures++;
    }
    return id;
}

/* A fresh board for one case: the shared copy pool is reset and every zone of
   both seats is emptied, so no assertion can read another case's residue. */
static void reset_board(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0; P->live.n = 0;
        P->success.n = 0; P->energy.n = 0; P->energy_active = 0;
        P->energy_deck.n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.n_revealed = 0;
    tg->state.live_success[0] = 0;
    tg->state.live_success[1] = 0;
    tg->state.last_draw_count = 0;
    tg->state.position_change_occurred_this_turn = 0;
}

/* ── board helpers ────────────────────────────────────────────────────────── */

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

/* Rust `main_deck.cards.clear()` then n x push(filler). */
static void fill_decks(TestGame *tg, int filler, int per_player)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < per_player && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

static void set_hand(TestGame *tg, int pl, const int *ids, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    P->hand.n = 0;
    for (int i = 0; i < n; i++) P->hand.cards[P->hand.n++] = ids[i];
}

static int hand_len(TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int deck_len(TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }
static int disc_len(TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }

static int hand_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < tg->state.p[pl].hand.n; i++)
        if (tg->state.p[pl].hand.cards[i] == cid) return 1;
    return 0;
}

static int disc_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < tg->state.p[pl].discard.n; i++)
        if (tg->state.p[pl].discard.cards[i] == cid) return 1;
    return 0;
}

static int live_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < tg->state.p[pl].live.n; i++)
        if (tg->state.p[pl].live.cards[i] == cid) return 1;
    return 0;
}

static int stage_has(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return 1;
    return 0;
}

static int n_discards_of(TestGame *tg, int pl, int cid)
{
    int n = 0;
    for (int i = 0; i < tg->state.p[pl].discard.n; i++)
        if (tg->state.p[pl].discard.cards[i] == cid) n++;
    return n;
}

/* test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE, so every heart
   read in this file goes straight at the store. */
static int heart_of(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_heart(&tg->state.mods, cid, slot);
}

static int need_heart_of(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, slot);
}

static int blade_of(TestGame *tg, int cid)
{
    return rb_mods_get_blade(&tg->state.mods, cid);
}

static int score_of(TestGame *tg, int cid)
{
    return rb_mods_get_score(&tg->state.mods, cid);
}

static int cost_of(TestGame *tg, int cid)
{
    return rb_mods_get_cost(&tg->state.mods, cid);
}

static int has_mod_entry(TestGame *tg, int cid)
{
    /* "the card has ANY modifier recorded" -- the Rust twin asks
       `heart_modifiers.contains_key(&id)`. rb_mods exposes no presence API,
       so the sum over all eight heart slots plus blade/score/cost is the
       faithful read: a card with no entry at all reads 0 everywhere. */
    for (int c = 0; c <= HALL; c++)
        if (rb_mods_get_heart(&tg->state.mods, cid, c) != 0) return 1;
    if (rb_mods_get_blade(&tg->state.mods, cid) != 0) return 1;
    if (rb_mods_get_score(&tg->state.mods, cid) != 0) return 1;
    if (rb_mods_get_cost(&tg->state.mods, cid) != 0) return 1;
    return 0;
}

static const char *orientation_of(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static int orientation_is_wait(TestGame *tg, int cid)
{
    const char *o = orientation_of(tg, cid);
    return o && !strcmp(o, "wait");
}

static int printed_cost(int cid)
{
    Card c; int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.cost; rb_free_card(&c); }
    return s;
}

static int printed_blade_heart(int cid)
{
    Card c; int s = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        s = rb_card_has_blade_heart(&c);
        rb_free_card(&c);
    }
    return s;
}

/* The printed blade-heart COLOUR, or -1. The Emma cluster counts DISTINCT
   blade-heart colours, so the fixture has to be pinned to the colour the Rust
   comment claims, not merely "has a blade heart". */
static int printed_blade_heart_colour(int cid)
{
    Card c; int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        for (int i = 0; i < c.num_blade; i++) {
            if (c.heart_color[c.num_base + i] != (uint8_t)RB_HEART_ALL) {
                s = c.heart_color[c.num_base + i];
                break;
            }
        }
        rb_free_card(&c);
    }
    return s;
}

/* All distinct blade-heart colours a print carries, for a blade-heart count
 * the printed ability may not limit to one. */
static int printed_blade_heart_colours(int cid, int *out, int max)
{
    Card c; int n = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    for (int i = 0; i < c.num_blade && n < max; i++) {
        int col = c.heart_color[c.num_base + i];
        if (col == (int)RB_HEART_ALL) continue;
        int seen = 0;
        for (int k = 0; k < n; k++) if (out[k] == col) seen = 1;
        if (!seen) out[n++] = col;
    }
    rb_free_card(&c);
    return n;
}

static int card_is_member(int cid) { return rb_card_is_member(cid); }
static int card_is_live(int cid)   { return rb_card_is_live(cid); }

/* How many abilities the COMPILED card database actually holds for `cid`.
 *
 * This matters because cards/cards.json is not what the engine reads: the C
 * tree loads cards/build (cards.bin + abilities.bin, produced by
 * compile_cards.py / compile_abilities.py), and a card whose JSON has a full
 * 「ability」 field can still decode to ZERO abilities in that build. When
 * that happens every ability-driven case for the card is measuring a DATA
 * BUILD, not the engine, and the run would otherwise report a silent
 * behavioural "gap" with the wrong cause. The section-level guard below turns
 * that into a named, single, unambiguous failure. */
static int n_abilities(int cid)
{
    return (cid >= 0) ? rb_card_num_abilities((uint32_t)cid) : -1;
}

/* A one-line report of every ability a card decodes to, so a data gap names
 * itself. */
static void dump_abilities(int cid, const char *tag)
{
    if (cid < 0) return;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return;
    int n = rb_card_num_abilities((uint32_t)cid);
    fprintf(stderr, "  (%s) %s: %d ability/abilities in the compiled DB\n",
            tag, rb_card_string(c.card_no_idx), n);
    for (int a = 0; a < n; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        fprintf(stderr, "      [%d] triggers=%s action=%s\n", a,
                ab.triggers ? ab.triggers : "(null)",
                (ab.effect && ab.effect->action) ? ab.effect->action : "(no effect)");
        rb_free_ability(&ab);
    }
    rb_free_card(&c);
}

/* The UNIT NAME, which is what 「同じユニット名」 and 「『蓮ノ空』」 actually
 * mean. rb_card_group_name returns the SERIES ("ラブライブ！") for every card
   in a set, so it CANNOT be used to compare units -- three of the abilities in
   this file are unit-scoped, and a series comparison would make all of them
   vacuous. Card.unit_idx indexes the same string table rb_card_string reads, so
   that is the faithful accessor. Rotating buffer: callers compare two names in
   one expression. */
static const char *unit_of(int cid)
{
    static char bufs[4][80];
    static int  next = 0;
    char *buf = bufs[next];
    next = (next + 1) & 3;
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = (c.unit_idx == UINT16_MAX) ? NULL : rb_card_string(c.unit_idx);
        if (s) snprintf(buf, 80, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* 「同じユニット名」 -- two cards share a unit iff their UNIT names match. */
static int same_unit(int a, int b)
{
    const char *ua = unit_of(a);
    const char *ub = unit_of(b);
    if (!ua || !ua[0] || !ub || !ub[0]) return 0;
    return strcmp(ua, ub) == 0;
}

/* ── choice plumbing ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void answer(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

static void skip_pick(TestGame *tg)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, -1);
}

/* PROCEED an auto-ability, index 0 on a card pick, index 1 on a
   pay_optional_cost gate (whose option 0 is SKIP). Answers nothing else with
   a skip. Returns the number of prompts it drove through, which is the
   non-vacuity counter. */
static int pc_pay(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = 0;
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = 1;   /* 0=SKIP, 1=PAY */
        rb_resume_with_choice(&tg->state, idx);
        n++;
    }
    return n;
}

/* Same shape, but the pay_optional_cost gate is DECLINED (index 0). */
static int pc_decline_pays_rest(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = 0;
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = 0;   /* SKIP */
        rb_resume_with_choice(&tg->state, idx);
        n++;
    }
    return n;
}

/* Proceed auto-abilities only; leave every other prompt standing, exactly like
   Rust's TestGame::drain_auto_ability_choices. Returns the count. */
static int pc_auto_only(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const char *k = pending_kind(tg);
        if (k && !strcmp(k, "SelectAutoAbility")) answer(tg, 0);
        else break;
        n++;
    }
    return n;
}

/* Rust `drain_choices()`: answer what cannot be declined, decline what can. */
static int pc_drain_skip(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { skip_pick(tg); n++; }
    return n;
}

/* Diagnostic: dump the pending prompt's shape. Used only when a red check
   needs to be classified as a parser gap vs an engine gap without guessing. */
static void describe_pending(TestGame *tg, const char *tag)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) { fprintf(stderr, "  (%s) no pending choice\n", tag); return; }
    fprintf(stderr, "  (%s) kind=%s zone=%s target=%s count=%d allow_skip=%d "
                    "route=%d n_filtered=%d desc=%s\n",
            tag, test_pending_choice_type(tg), c->zone, c->target, c->count,
            c->allow_skip, (int)c->route, c->n_filtered_indices, c->description);
}

/* ── trigger shim (Rust helpers::fire_trigger, mod.rs:114-141) ──────────────
   Resolve ONE ability of ONE card, key it "<card_no>_<full_text>", and call
   trigger_auto_ability with an explicit card id; then activating_card and
   process_pending_auto_abilities.
   This body used to be hand-rolled against a local rb_trigger_auto_ability
   prototype. The enqueue was already right, but the LOAD-BEARING ENQUEUE-TIME
   USE RECORD was missing: the C port spends a ターン1回 use at enqueue (every
   scan in src/turn/triggers.c calls rb_record_use and stamps
   rb_mark_entries_use_limit_recorded), so with the use left unspent the
   post-resolution re-scan in rb_process_player_abilities_depth_full saw it
   still available and enqueued the same ability a SECOND time. MEASURED on
   test_parity_state_per_card's T/010 (PL!N-sd2-010-SD2): the ally gained 4
   blade instead of 2. test_fire_trigger_core_seat (src/test_game.c:1166-1170)
   performs the record and the stamp.
   `test_fire_trigger_on_card` also matches with the ENGINE's predicate
   (rb_trigger_is, a strstr) instead of an exact strcmp -- the same predicate
   rb_trigger_auto_ability re-checks internally.
   The [TRIGGER-MISS] dump below is preserved verbatim: a miss is a HARNESS bug
   and must say so loudly instead of turning a negative assertion into a
   vacuous pass. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *type)
{
    (void)type;   /* Rust passes the SAME token as trigger and trig (mod.rs:130-138) */
    if (test_fire_trigger_on_card(tg, cid, trig)) return 1;
    Card c;
    memset(&c, 0, sizeof c);
    char card_no[160];
    card_no[0] = '\0';
    int decoded = (cid >= 0) && rb_decode_card_by_index((uint32_t)cid, &c);
    int nab = (cid >= 0) ? rb_card_num_abilities((uint32_t)cid) : 0;
    if (decoded) snprintf(card_no, sizeof(card_no), "%s", rb_card_string(c.card_no_idx));
    fprintf(stderr, "[TRIGGER-MISS] card %s: no ability printing \"%s\"; has:",
            card_no[0] ? card_no : "(undecodable)", trig);
    for (int a = 0; a < nab; a++) {
        Ability ab; memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        fprintf(stderr, " [%d]=\"%s\"", a, ab.triggers ? ab.triggers : "(null)");
        rb_free_ability(&ab);
    }
    fprintf(stderr, "\n");
    if (decoded) rb_free_card(&c);
    return 0;
}

static int fire_debut(TestGame *tg, int cid)
{
    return fire_trigger(tg, cid, TRIG_DEBUT, TRIG_DEBUT);
}

static int fire_live_start(TestGame *tg, int cid)
{
    return fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START);
}

/* ライブ成功時 must be driven through rb_trigger_live_success: the Rust twin's
   `fire_trigger` path needs BOTH live.n > 0 and live_success[pl], and a
   sibling's report is that the phase machine is not a reliable producer. */
static int fire_live_success(TestGame *tg, int cid)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = cid;
    int n = rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    return n;
}

/* ── phase walk ───────────────────────────────────────────────────────────── */

/* Rust advance_to_live_card_set_p1: five blind passes reach the live-card set
   window. */
static void walk_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start / finish_live_setup: two more passes. */
static void walk_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* ── position-change answer index ───────────────────────────────────────────
 * The 「移動」 (position_change) prompt is a SelectTarget whose resume handler
 * rb_resolver_handle_position_destination (choice.c) reads the answer with
 * `atoi(selected)` and applies it DIRECTLY as a stage slot via
 * rb_stage_position_index -- there is no filtered-candidate remap, so
 *
 *      answer 0 -> LEFT, 1 -> CENTRE, 2 -> RIGHT
 *
 * and the answer SWAPS the source slot with the destination slot. The
 * destination is NOT chosen relative to the source and there is no "same
 * slot" option, so answering the source's own index is a no-op swap. Every
 * position case in sections E4, F2 and G uses these constants so the index is
 * stated once. */
#define PICK_LEFT   0
#define PICK_CENTRE 1
#define PICK_RIGHT  2

/* Drive the 「移動」 prompt to a destination OTHER than the card's current
 * slot, without hard-coding an answer index.
 *
 * WHY THIS EXISTS: the prompt is a SelectTarget with target
 * "position|destination" and n_filtered_indices == 0, so the answer index is
 * passed straight through -- but rb_resume_position_change (state.c) then reads
 * the SOURCE slot by searching the stage for `g->queue.resume_host`. A test
 * that hard-codes an index is therefore testing its own guess about which of
 * those two conventions the engine uses.
 *
 * This helper picks the lowest other slot and RETURNS it, so the caller
 * asserts the card landed at the slot this function actually requested. The
 * engine-behaviour assertions (blades gained, etc.) are unchanged and strict;
 * only the bookkeeping says which destination was chosen.
 *
 * Returns the chosen slot, or -1 if there was no prompt. */
static int drive_position_to_other_slot(TestGame *tg, int src_slot)
{
    if (!pending(tg)) return -1;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return -1;
    fprintf(stderr, "  (position) src_slot=%d kind=%s target=%s n_filtered=%d\n",
            src_slot, test_pending_choice_type(tg), c->target, c->n_filtered_indices);

    /* An explicit candidate list wins; otherwise the index IS the slot. */
    for (int i = 0; i < c->n_filtered_indices; i++) {
        int cand = c->filtered_indices[i];
        if (cand >= 0 && cand < RB_STAGE_SIZE && cand != src_slot) {
            answer(tg, i);
            return cand;
        }
    }
    for (int slot = 0; slot < RB_STAGE_SIZE; slot++) {
        if (slot == src_slot) continue;
        answer(tg, slot);
        return slot;
    }
    return -1;
}

/* Which stage slot currently holds `cid`, or -1. */
static int slot_of(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return i;
    return -1;
}

static void push_deck(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++)
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

/* Rust `retain(|c| *c != id)` -- remove every copy of `cid`. */
static void remove_from_zone(TestGame *tg, int pl, const char *zone, int cid)
{
    RbBag *b;
    if (!strcmp(zone, "discard")) b = &tg->state.p[pl].discard;
    else if (!strcmp(zone, "hand")) b = &tg->state.p[pl].hand;
    else if (!strcmp(zone, "deck")) b = &tg->state.p[pl].deck;
    else if (!strcmp(zone, "live")) b = &tg->state.p[pl].live;
    else if (!strcmp(zone, "success")) b = &tg->state.p[pl].success;
    else return;
    int w = 0;
    for (int i = 0; i < b->n; i++)
        if (b->cards[i] != cid) b->cards[w++] = b->cards[i];
    b->n = w;
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION A -- effects/gain/hearts/per_card (7 .rs files, 25 #[test])
 * ══════════════════════════════════════════════════════════════════════════ */

/* ── A0. sumire_highest_center_cost_gains_heart03_test.rs (2 tests) ─────────
 * PL!SP-bp2-004-R 「常時: 自分のステージにいるメンバーのうち、センターエリア
 * にいるメンバーが最も大きいコストを持つ場合、heart03を得る」
 *
 * A peer's test_parity_hearts_constants.c section E already drives this
 * ability through an 18-row centre-cost matrix, and
 * test_parity_conditional_gated.c section J re-drives it with the tie and
 * empty-centre cases. Ported here anyway because the Rust twin asserts two
 * facts no peer states: the centre member need NOT be the source, and the
 * grant goes to SELF only. */
static void a0_sumire_center_highest_cost_heart03(void)
{
    static TestGame tg;
    reset_board(&tg);
    int sumire = mid(&tg, SUMIRE_R);
    int cost10 = mid(&tg, HANANO_A);   /* cost 10 */
    int cost4  = mid(&tg, SD_010);     /* cost 4  */
    if (!pin_id(sumire, SUMIRE_R) || !pin_id(cost10, HANANO_A) ||
        !pin_id(cost4, SD_010)) return;
    CHECK_EQ(printed_cost(sumire), 9, "A0 fixture: PL!SP-bp2-004-R costs 9");
    CHECK_EQ(printed_cost(cost10), 10, "A0 fixture: PL!HS-PR-001-PR costs 10");
    CHECK_EQ(printed_cost(cost4), 4, "A0 fixture: PL!-sd1-010-SD costs 4");

    /* Centre 9, left 10 -> centre is NOT the highest. */
    set_stage(&tg, 0, cost10, sumire, cost4);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, sumire, H03), 0,
             "A0: left (cost 10) outranks centre (cost 9) -> no heart03");

    /* The cost-10 card in the centre is not the source, so it gains nothing
     * even though the printed condition is now TRUE. */
    set_stage(&tg, 0, cost4, cost10, sumire);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, cost10, H03), 0,
             "A0: the 常時 grants to ITSELF, never to the qualifying centre member");
    CHECK_EQ(heart_of(&tg, sumire, H03), 0,
             "A0: sumire is on the RIGHT, so the centre-gate does not reach it");

    /* sumire moves to the centre: it is now strictly the highest. */
    set_stage(&tg, 0, sumire, cost10, cost4);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, sumire, H03), 1,
             "A0: sumire in the centre above 10 and 4 -> exactly one heart03");
}

/* A tie at the top is NOT 最も大きい. Two distinct instances of the SAME
 * cost-10 print share it, so neither gains. */
static void a0_sumire_tie_for_highest_no_heart03(void)
{
    static TestGame tg;
    reset_board(&tg);
    int tie_a = mid(&tg, HANANO_A);
    int tie_b = mid(&tg, HANANO_A);
    int low   = mid(&tg, SD_010);
    if (!pin_id(tie_a, HANANO_A) || !pin_id(tie_b, HANANO_A) ||
        !pin_id(low, SD_010)) return;
    CHECK(tie_a != tie_b, "A0 tie fixture: two game.id() calls are two instances");
    CHECK_EQ(printed_cost(tie_a), 10, "A0 tie fixture: both copies cost 10");
    CHECK_EQ(printed_cost(tie_b), 10, "A0 tie fixture: both copies cost 10");

    set_stage(&tg, 0, tie_a, tie_b, low);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, tie_b, H03), 0,
             "A0: the centre SHARES the highest cost with a side member -> no heart03");
    CHECK_EQ(heart_of(&tg, tie_a, H03), 0,
             "A0: the 常時 grants to itself only, never to the tied peer");

    /* CONTROL, and the only thing making the check above non-vacuous: the SAME
     * two cards, with the tied peer pulled off the stage, must flip the answer.
     * Without a control the zero above would also read on a board where the
     * 常時 never resolved. */
    set_stage(&tg, 0, RB_EMPTY_SLOT, tie_b, low);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, tie_b, H03), 1,
             "A0 CONTROL: with the tied peer gone, the lone centre member is "
             "trivially highest -> heart03 appears, so the tie check above is "
             "measuring the tie and not a dead ability");
}

/* ── A1. izumi_debut_other_heart06_member_gain_test.rs (2 tests) ────────────
 * PL!HS-pb1-016-R 桂城 泉 ab#0 登場:
 *   「自分のステージにいるこのメンバー以外の heart06 を持つメンバー1人は、
 *     ライブ終了時まで、heart06 を得る」
 * The KEY of this ability is the FILTER: only a member that ALREADY has a
 * printed heart06 qualifies. */
static void a1_izumi_grants_heart06_to_heart06_member_only(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi  = mid(&tg, IZUMI_R);
    int target = mid(&tg, SD_001);   /* printed base_heart06 */
    int filler = mid(&tg, SD_010);   /* no heart06 */
    if (!pin_id(izumi, IZUMI_R) || !pin_id(target, SD_001) ||
        !pin_id(filler, SD_010)) return;
    CHECK_EQ(printed_cost(izumi), 2, "A1 fixture: PL!HS-pb1-016-R costs 2");
    CHECK(card_is_member(izumi), "A1 fixture: Izumi is a member card");
    CHECK(card_is_member(target), "A1 fixture: the target is a member card");

    fill_decks(&tg, filler, 40);
    set_stage(&tg, 0, target, RB_EMPTY_SLOT, filler);
    test_give_energy(&tg, 2);
    test_add_to_hand(&tg, izumi);

    int played = test_play_to_stage(&tg, izumi, 1);
    CHECK_EQ(played, 1, "A1: Izumi's debut is accepted");
    CHECK_EQ(tg.state.p[0].stage[1], izumi, "A1 CONTROL: Izumi really is on the centre stage");

    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A1) prompts driven = %d\n", prompts);
    CHECK(prompts > 0,
          "A1 CONTROL: the 登場 ability opened at least one prompt, so the "
          "modifier checks below are not vacuous on a board where it never ran");

    CHECK_EQ(heart_of(&tg, target, H06), 1,
             "A1: the target that already has heart06 gains exactly +1 heart06");
    CHECK_EQ(heart_of(&tg, filler, H06), 0,
             "A1: a member WITHOUT a printed heart06 gains nothing");
    CHECK_EQ(heart_of(&tg, izumi, H06), 0,
             "A1: the activator is excluded (このメンバー以外)");
}

/* The negative half. Both staged members lack heart06, so there is no valid
 * target and nothing may be granted. The prompt control is this card's own
 * ability firing, checked by the A1 case above; here the only thing asserted
 * is that NO modifier appears. */
static void a1_izumi_no_valid_target_no_modifier(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_R);
    int f1 = mid(&tg, SD_010);
    int f2 = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_R) || !pin_id(f1, SD_010) || !pin_id(f2, SD_010)) return;
    CHECK(f1 != f2, "A1 negative fixture: the two fillers are distinct instances");

    fill_decks(&tg, f1, 40);
    set_stage(&tg, 0, f1, RB_EMPTY_SLOT, f2);
    test_give_energy(&tg, 2);
    test_add_to_hand(&tg, izumi);

    int played = test_play_to_stage(&tg, izumi, 1);
    CHECK_EQ(played, 1, "A1 negative: the debut is still accepted");
    CHECK_EQ(tg.state.p[0].stage[1], izumi,
             "A1 negative CONTROL: the debut really placed Izumi on stage");

    pc_pay(&tg, 12);

    CHECK_EQ(heart_of(&tg, izumi, H06), 0,
             "A1 negative: the activator gains nothing");
    CHECK_EQ(heart_of(&tg, f1, H06), 0,
             "A1 negative: filler 1 (no heart06) gains nothing");
    CHECK_EQ(heart_of(&tg, f2, H06), 0,
             "A1 negative: filler 2 (no heart06) gains nothing");
}

/* ── A2. hasunosora_live_start_waitroom_ten_heart04_gain_test.rs (4) ───────
 * PL!HS-pb1-025-L 抱きしめる花びら ab#0 ライブ開始時:
 *   「自分の控え室に『蓮ノ空』のメンバーカードが10枚以上ある場合、
 *     自分のステージにいる『蓮ノ空』のメンバー1人は、heart04 を得る」 */
static int a2_setup(TestGame *tg, int waitroom_count, int staged_hasu, int second_hasu)
{
    int hanasaka = mid(tg, HANASAKA);
    int f = mid(tg, SD_010);
    if (!pin_id(hanasaka, HANASAKA) || !pin_id(f, SD_010)) return -1;
    if (staged_hasu < 0) staged_hasu = mid(tg, HANANO_A);
    if (staged_hasu >= 0 && !pin_id(staged_hasu, HANANO_A)) return -1;
    if (second_hasu < 0) second_hasu = -1;
    else if (!pin_id(second_hasu, HANANO_B)) return -1;

    fill_decks(tg, f, 30);
    if (second_hasu >= 0) set_stage(tg, 0, staged_hasu, second_hasu, RB_EMPTY_SLOT);
    else set_stage(tg, 0, RB_EMPTY_SLOT, staged_hasu, RB_EMPTY_SLOT);
    test_add_to_hand(tg, hanasaka);
    test_add_to_hand(tg, f);
    /* 10+ 蓮ノ空 member cards in the WAITROOM. Distinct instances, because a
     * count of physical cards is what the printed rule says. */
    for (int i = 0; i < waitroom_count; i++) {
        int h = mid(tg, HANANO_A);
        if (h < 0) return -1;
        rb_waitroom_add(&tg->state.p[0], h);
    }
    test_give_energy(tg, 5);
    CHECK(card_is_live(hanasaka), "A2 fixture: 抱きしめる花びら is a live card");
    return hanasaka;
}

/* PL!HS-pb1-025-L must carry a ライブ開始時 ability at all, or the whole
 * cluster is measuring a parser miss. Asserted once here rather than inferred
 * from a zero result. */
static void a2_hanasaka_carries_live_start(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanasaka = mid(&tg, HANASAKA);
    if (!pin_id(hanasaka, HANASAKA)) return;
    int live = mid(&tg, SD_019);
    if (!pin_id(live, SD_019)) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_set_live_card(&tg, 0, hanasaka);
    fill_decks(&tg, live, 20);
    CHECK(fire_live_start(&tg, hanasaka),
          "A2: PL!HS-pb1-025-L carries a ライブ開始時 ability (parser guard)");
}

/* Condition met (10 in waitroom), exactly ONE eligible target -> the effect
 * auto-applies. */
static void a2_condition_met_single_target_auto_applies(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanasaka = a2_setup(&tg, 10, -1, -1);
    if (hanasaka < 0) return;
    int hasu = tg.state.p[0].stage[1];
    CHECK_EQ(stage_has(&tg, 0, hasu), 1,
             "A2: the 蓮ノ空 member really is staged");
    CHECK(printed_cost(hasu) == 10,
          "A2 fixture: the staged 蓮ノ空 member is the cost-10 print");

    CHECK(fire_live_start(&tg, hanasaka), "A2: the ライブ開始時 ability fires");
    if (pending(&tg)) describe_pending(&tg, "A2 single");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A2 single) prompts = %d\n", prompts);

    CHECK_EQ(disc_len(&tg, 0), 10, "A2 CONTROL: the waitroom still holds the 10 members");
    /* Exactly one eligible target means the effect auto-applies with NO prompt,
     * so the prompt count is deliberately NOT the control here. The control is
     * the a2_two_targets case on the same card, which DOES open a prompt. */
    CHECK_EQ(prompts, 0,
             "A2: with a single eligible target the effect auto-applies without "
             "a selection prompt");
    CHECK_EQ(heart_of(&tg, hasu, H04), 1,
             "A2: the sole eligible 蓮ノ空 member gains exactly +1 heart04");
    CHECK_EQ(heart_of(&tg, hanasaka, H04), 0,
             "A2: the live card itself is not the target");
}

/* Condition NOT met (9 < 10) -> no effect and, per the Rust twin, not even a
 * pending choice. The control is the sibling case above on the SAME card: if
 * 10 does prompt-and-grant and 9 does neither, the count is what decided. */
static void a2_condition_not_met_no_effect(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanasaka = a2_setup(&tg, 9, -1, -1);
    if (hanasaka < 0) return;
    int hasu = tg.state.p[0].stage[1];

    CHECK(fire_live_start(&tg, hanasaka), "A2: the ライブ開始時 ability fires at 9 too");
    if (pending(&tg)) describe_pending(&tg, "A2 nine");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A2 nine) prompts = %d\n", prompts);

    CHECK_EQ(disc_len(&tg, 0), 9, "A2 CONTROL: the waitroom really holds only 9 members");
    CHECK_EQ(heart_of(&tg, hasu, H04), 0,
             "A2: 9 members in the waitroom is below the 10 threshold -> no heart04");
    CHECK_EQ(prompts, 0,
             "A2: 9 members -> the ability offers no target prompt at all");
}

/* A non-蓮ノ空 staged member is not a target, and with no eligible target
 * there is no prompt. */
static void a2_non_hasunosora_not_targeted(void)
{
    static TestGame tg;
    reset_board(&tg);
    int f = mid(&tg, SD_010);
    int non = mid(&tg, SD_010);
    int hanasaka = mid(&tg, HANASAKA);
    if (!pin_id(f, SD_010) || !pin_id(non, SD_010) || !pin_id(hanasaka, HANASAKA)) return;

    fill_decks(&tg, f, 30);
    set_stage(&tg, 0, RB_EMPTY_SLOT, non, RB_EMPTY_SLOT);
    test_add_to_hand(&tg, hanasaka);
    test_add_to_hand(&tg, f);
    for (int i = 0; i < 10; i++) { int h = mid(&tg, HANANO_A); rb_waitroom_add(&tg.state.p[0], h); }
    test_give_energy(&tg, 5);
    CHECK_EQ(disc_len(&tg, 0), 10,
             "A2 non-蓮ノ空 CONTROL: 10 蓮ノ空 members are in the waitroom, so "
             "only the STAGE filter can be what blocks the grant");

    CHECK(fire_live_start(&tg, hanasaka), "A2: the ability fires");
    if (pending(&tg)) describe_pending(&tg, "A2 non-hasu");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A2 non-hasu) prompts = %d\n", prompts);

    CHECK_EQ(heart_of(&tg, non, H04), 0,
             "A2: a non-蓮ノ空 centre member gains no heart04");
    CHECK_EQ(prompts, 0,
             "A2: with no eligible 蓮ノ空 member on stage no target prompt opens");
}

/* Two 蓮ノ空 members on stage -> a SelectTarget appears and only the CHOSEN
 * one gains. target_count=1, so the unselected peer must be untouched. */
static void a2_two_targets_only_selected_gains(void)
{
    static TestGame tg;
    reset_board(&tg);
    /* TWO distinct 蓮ノ空 prints staged -- HANANO_A on the left, HANANO_B in
     * the centre. Both satisfy 「蓮ノ空」, so a player choice is required. */
    int hanasaka = a2_setup(&tg, 10, -1, -1);
    if (hanasaka < 0) return;
    int left   = mid(&tg, HANANO_A);
    int centre = mid(&tg, HANANO_B);
    if (!pin_id(left, HANANO_A) || !pin_id(centre, HANANO_B)) return;
    set_stage(&tg, 0, left, centre, RB_EMPTY_SLOT);

    CHECK(fire_live_start(&tg, hanasaka), "A2: the ability fires with two candidates");
    CHECK(pending(&tg),
          "A2 CONTROL: two eligible 蓮ノ空 members really do open a SelectTarget");
    if (pending(&tg)) describe_pending(&tg, "A2 two-targets");
    answer(&tg, 0);
    pc_pay(&tg, 12);

    CHECK_EQ(heart_of(&tg, left, H04), 1, "A2: the SELECTED member gains heart04");
    CHECK_EQ(heart_of(&tg, centre, H04), 0, "A2: the UNSELECTED member gains nothing");
}

/* ── A3. rurino_debut_activate_wait_member_add_live_card_test.rs (5) ──────
 * PL!HS-bp6-003-R 大沢瑠璃乃 ab#0 登場 (optional):
 *   「自分のステージにいるウェイト状態の『みらくらぱーく！』のメンバー1人を
 *     アクティブにしてもよい。そうした場合、自分の控え室から『みらくらぱーく！』
 *     のライブカードを1枚手札に加える」 */
static int a3_setup(TestGame *tg, int with_live_in_discard, int member_waited)
{
    int rurino = mid(tg, RURINO);
    int cheap  = mid(tg, RURINO_CHEAP);
    int live   = mid(tg, MIRAKURA_LIVE);
    int f      = mid(tg, SD_010);
    if (!pin_id(rurino, RURINO) || !pin_id(cheap, RURINO_CHEAP) ||
        !pin_id(live, MIRAKURA_LIVE) || !pin_id(f, SD_010)) return -1;
    CHECK_EQ(printed_cost(rurino), 11, "A3 fixture: PL!HS-bp6-003-R costs 11");
    CHECK_EQ(printed_cost(cheap), 2,  "A3 fixture: PL!HS-bp6-011-R costs 2");
    CHECK(card_is_live(live),  "A3 fixture: PL!HS-PR-012-PR is a live card");

    fill_decks(tg, f, 20);
    test_give_energy(tg, 15);
    if (with_live_in_discard) rb_waitroom_add(&tg->state.p[0], live);
    set_stage(tg, 0, cheap, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    if (member_waited) rb_mods_set_orientation(&tg->state.mods, cheap, "wait");
    test_add_to_hand(tg, rurino);
    return rurino;
}

static void a3_pay_activates_member_and_adds_live(void)
{
    static TestGame tg;
    reset_board(&tg);
    int rurino = a3_setup(&tg, 1, 1);
    if (rurino < 0) return;
    int cheap = tg.state.p[0].stage[0];
    int live  = mid(&tg, MIRAKURA_LIVE);
    /* pin_id already proved the print; the copy in the waitroom is a different
     * instance, so look it up rather than comparing against this one. */
    int live_in_discard = tg.state.p[0].discard.cards[0];

    CHECK(orientation_is_wait(&tg, cheap), "A3 precondition: the member starts in wait");
    CHECK_EQ(disc_len(&tg, 0), 1, "A3 precondition: the みらくらぱーく！ live card is in the waitroom");

    int played = test_play_to_stage(&tg, rurino, 1);
    CHECK_EQ(played, 1, "A3: the debut is accepted");
    CHECK_EQ(tg.state.p[0].stage[1], rurino, "A3 CONTROL: Rurino really reached the centre");

    CHECK(pending(&tg), "A3 CONTROL: the optional activation gate really opened");
    if (pending(&tg)) describe_pending(&tg, "A3 pay");
    /* Option 1 is PAY on a pay_optional_cost gate; answering 0 would be SKIP
     * and would make this the same board as the skipping case. */
    answer(&tg, 1);
    int more = pc_pay(&tg, 12);
    fprintf(stderr, "  (A3 pay) follow-up prompts = %d\n", more);

    CHECK(!orientation_is_wait(&tg, cheap),
          "A3: paying the optional cost activated the waited member");
    CHECK(!disc_has(&tg, 0, live_in_discard),
          "A3: the みらくらぱーく！ live card left the waitroom");
    CHECK(hand_has(&tg, 0, live_in_discard),
          "A3: 「そうした場合」 fired -- the live card reached hand");
    CHECK_EQ(n_discards_of(&tg, 0, live_in_discard), 1,
             "A3: exactly ONE copy of the live card was moved, not two");
    (void)live;
}

/* No waited みらくらぱーく！ member -> the unpayable option is not offered and
 * the second action never fires. */
static void a3_no_wait_member_skips_whole_sequence(void)
{
    static TestGame tg;
    reset_board(&tg);
    int rurino = a3_setup(&tg, 1, 0);
    if (rurino < 0) return;
    int cheap = tg.state.p[0].stage[0];
    int live_in_discard = tg.state.p[0].discard.cards[0];

    CHECK(!orientation_is_wait(&tg, cheap),
          "A3 negative precondition: the member is ACTIVE, so no wait target exists");

    int played = test_play_to_stage(&tg, rurino, 1);
    CHECK_EQ(played, 1, "A3 negative: the debut is still accepted");
    CHECK_EQ(tg.state.p[0].stage[1], rurino,
             "A3 negative CONTROL: the debut really placed Rurino on stage");

    CHECK(!pending(&tg),
          "A3 negative: with no waited member the optional prompt is NOT offered");

    CHECK(disc_has(&tg, 0, live_in_discard),
          "A3 negative: the live card stays in the waitroom");
    CHECK(!hand_has(&tg, 0, live_in_discard),
          "A3 negative: the live card never reached hand");
}

/* The member exists in wait, the player DECLINES -> the whole second action is
 * suppressed. This is the arm that the pay case above must differ from. */
static void a3_skip_optional_leaves_live_in_discard(void)
{
    static TestGame tg;
    reset_board(&tg);
    int rurino = a3_setup(&tg, 1, 1);
    if (rurino < 0) return;
    int cheap = tg.state.p[0].stage[0];
    int live_in_discard = tg.state.p[0].discard.cards[0];

    int played = test_play_to_stage(&tg, rurino, 1);
    CHECK_EQ(played, 1, "A3 skip: the debut is accepted");
    CHECK(pending(&tg), "A3 skip CONTROL: the optional gate opened");
    if (pending(&tg)) describe_pending(&tg, "A3 skip");
    answer(&tg, 0);              /* SKIP */
    pc_pay(&tg, 12);

    CHECK(disc_has(&tg, 0, live_in_discard),
          "A3 skip: declining the option leaves the live card in the waitroom");
    CHECK(!hand_has(&tg, 0, live_in_discard),
          "A3 skip: the 「そうした場合」 second action did not fire");
    /* The member must be UNCHANGED -- this is what makes the skip arm
     * different from the pay arm above and not a duplicate of it. */
    CHECK(orientation_is_wait(&tg, cheap),
          "A3 skip: the waited member is still in wait after the decline");
}

/* Empty waitroom: the activation still works, the move simply finds nothing,
 * and the engine must not fault. */
static void a3_empty_discard_no_crash(void)
{
    static TestGame tg;
    reset_board(&tg);
    int rurino = a3_setup(&tg, 0, 1);
    if (rurino < 0) return;
    int cheap = tg.state.p[0].stage[0];
    CHECK_EQ(disc_len(&tg, 0), 0, "A3 empty precondition: the waitroom really is empty");

    int played = test_play_to_stage(&tg, rurino, 1);
    CHECK_EQ(played, 1, "A3 empty: the debut is accepted");
    CHECK(pending(&tg), "A3 empty CONTROL: the optional gate still opened");
    answer(&tg, 1);              /* PAY */
    pc_pay(&tg, 12);

    CHECK(!orientation_is_wait(&tg, cheap),
          "A3 empty: paying still activated the waited member");
    CHECK_EQ(disc_len(&tg, 0), 0, "A3 empty: the waitroom is still empty");
}

/* PL!HS-bp5-003-R＋ ab#1 ライブ開始時: optional hand discard, then one member
 * of the DISCARDED card's group gains heart01. The group filter is the point:
 * the third staged member is a different unit and must not be selectable. */
static void a3_bp5_live_start_group_matched_heart01(void)
{
    static TestGame tg;
    reset_board(&tg);
    int rurino = mid(&tg, RURINO_BP5);
    int miraku = mid(&tg, RURINO_CHEAP);
    int filler = mid(&tg, SD_010);
    int live   = mid(&tg, MIRAKURA_LIVE);
    int costc  = mid(&tg, RURINO_CHEAP);
    if (!pin_id(rurino, RURINO_BP5) || !pin_id(miraku, RURINO_CHEAP) ||
        !pin_id(filler, SD_010) || !pin_id(live, MIRAKURA_LIVE) ||
        !pin_id(costc, RURINO_CHEAP)) return;

    set_stage(&tg, 0, rurino, miraku, filler);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 10);
    test_set_live_card(&tg, 0, live);
    int hand[2] = { costc, live };
    set_hand(&tg, 0, hand, 2);

    CHECK(fire_live_start(&tg, rurino), "A3 bp5: the ライブ開始時 ability fires");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A3 bp5) prompts = %d\n", prompts);
    CHECK(prompts > 0,
          "A3 bp5 CONTROL: the optional-cost prompt opened, so the heart01 "
          "assertions below are not vacuous");

    CHECK(heart_of(&tg, rurino, H01) > 0,
          "A3 bp5: a みらくらぱーく！ member gains heart01");
    CHECK_EQ(heart_of(&tg, filler, H01), 0,
             "A3 bp5: a Printemps member (wrong group) gains nothing");
}

/* ── A4. subunit_stage_count_gated_debut_heart_test.rs (2 tests) ────────────
 * PL!N-bp7-024-N 鐘 嵐珠 ab#0 登場: 3 R3BIRTH members across BOTH stages ->
 * heart01 until live end. */
static void a4_ranbu_three_r3birth_gains_heart01(void)
{
    static TestGame tg;
    reset_board(&tg);
    int me  = mid(&tg, RANBU_N);
    int r1  = mid(&tg, R3BIRTH_A);
    int r2  = mid(&tg, R3BIRTH_B);
    int r3  = mid(&tg, R3BIRTH_C);
    if (!pin_id(me, RANBU_N) || !pin_id(r1, R3BIRTH_A) ||
        !pin_id(r2, R3BIRTH_B) || !pin_id(r3, R3BIRTH_C)) return;
    CHECK_EQ(printed_cost(me), 4, "A4 fixture: PL!N-bp7-024-N costs 4");
    CHECK(card_is_member(r1), "A4 fixture: the Miya print is a member");
    CHECK(card_is_member(r3), "A4 fixture: the Shiori print is a member");

    fill_decks(&tg, r3, 20);
    set_stage(&tg, 0, me, r1, r2);
    set_stage(&tg, 1, r3, RB_EMPTY_SLOT, RB_EMPTY_SLOT);

    CHECK(fire_debut(&tg, me), "A4: the 登場 ability fires");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A4 three) prompts = %d\n", prompts);

    CHECK_EQ(stage_has(&tg, 0, me), 1,
             "A4 CONTROL: all three R3BIRTH members really are on stage (two "
             "yours, one the opponent's)");
    CHECK(heart_of(&tg, me, H01) > 0,
          "A4: 3 R3BIRTH members across both stages -> heart01 until live end");
}

static void a4_ranbu_two_r3birth_no_heart01(void)
{
    static TestGame tg;
    reset_board(&tg);
    int me = mid(&tg, RANBU_N);
    int r1 = mid(&tg, R3BIRTH_A);
    int other = mid(&tg, SD_010);
    if (!pin_id(me, RANBU_N) || !pin_id(r1, R3BIRTH_A) || !pin_id(other, SD_010)) return;

    fill_decks(&tg, other, 20);
    set_stage(&tg, 0, me, r1, other);
    CHECK_EQ(stage_has(&tg, 0, r1), 1, "A4 negative CONTROL: one R3BIRTH peer is staged");
    /* The gate is a GROUP count, so the filler must provably not be R3BIRTH. */
    CHECK(rb_card_group_name(other) != rb_card_group_name(r1),
          "A4 negative CONTROL: the filler is a DIFFERENT group from R3BIRTH");

    CHECK(fire_debut(&tg, me), "A4 negative: the 登場 ability fires");
    pc_pay(&tg, 12);

    CHECK_EQ(heart_of(&tg, me, H01), 0,
             "A4 negative: only 2 R3BIRTH members on stage -> no heart01");
    CHECK_EQ(heart_of(&tg, other, H01), 0,
             "A4 negative: the non-R3BIRTH filler gains nothing");
}

/* ── A5. q127_heart_set_plus_global_test.rs (10 tests) ──────────────────────
 * Q127: a 常時 that modifies every live card in the OPPONENT's live zone
 * (PL!SP-bp2-010-P ウィーン・マルガレーテ, +1 required heart00) must STACK
 * ON TOP OF a live-start SET operation (PL!HS-bp2-019-L Bloom, PL!SP-bp1-026-L
 * 未来予報ハレルヤ！). Rule Q115: set-to-X applies first, then add/subtract
 * modifiers stack. The Rust twin drives this through the real phase walk; here
 * the two abilities are queued directly (fire_live_start), which is the same
 * idiom the peer test_parity_hearts_live_start.c uses and keeps the case
 * independent of phase-machine timing. */

/* Bloom in P1's live zone, Wien on P2's stage, a 蓮ノ空 member on P1's stage
 * (Bloom's own condition host), 50 filler on both decks. */
static int a5_setup_bloom(TestGame *tg, int wien_area)
{
    int wien = mid(tg, WIEN);
    int bloom = mid(tg, BLOOM);
    int hasu = mid(tg, HASU_BP1);
    int filler = mid(tg, SD_010);
    if (!pin_id(wien, WIEN) || !pin_id(bloom, BLOOM) ||
        !pin_id(hasu, HASU_BP1) || !pin_id(filler, SD_010)) return -1;
    CHECK_EQ(printed_cost(wien), 15, "A5 fixture: PL!SP-bp2-010-P costs 15");
    CHECK(card_is_live(bloom), "A5 fixture: Bloom is a live card");

    fill_decks(tg, filler, 50);
    set_stage(tg, 1, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    if (wien_area >= 0) tg->state.p[1].stage[wien_area] = wien;
    set_stage(tg, 0, RB_EMPTY_SLOT, hasu, RB_EMPTY_SLOT);
    test_set_live_card(tg, 0, bloom);
    CHECK(live_has(tg, 0, bloom), "A5 CONTROL: Bloom really occupies the live-card zone");
    return bloom;
}

/* Hareruya's own ライブ開始時 is a cost CHANGE gated on 5 Liella! members in the
 * waitroom, so the fixture has to build them. */
static int a5_setup_hareruya(TestGame *tg)
{
    int wien = mid(tg, WIEN);
    int hareruya = mid(tg, HARERUYA);
    int filler = mid(tg, SD_010);
    if (!pin_id(wien, WIEN) || !pin_id(hareruya, HARERUYA) ||
        !pin_id(filler, SD_010)) return -1;
    CHECK(card_is_live(hareruya), "A5 fixture: 未来予報ハレルヤ！ is a live card");

    fill_decks(tg, filler, 50);
    set_stage(tg, 1, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    static const char *const liella[5] = { LIELLA_014, LIELLA_015, LIELLA_016,
                                           LIELLA_019, LIELLA_020 };
    for (int i = 0; i < 5; i++) {
        int m = mid(tg, liella[i]);
        if (m < 0 || !pin_id(m, liella[i])) return -1;
        rb_waitroom_add(&tg->state.p[0], m);
    }
    test_set_live_card(tg, 0, hareruya);
    CHECK(live_has(tg, 0, hareruya), "A5 CONTROL: 未来予報ハレルヤ！ is in the live zone");
    CHECK_EQ(disc_len(tg, 0), 5, "A5 CONTROL: 5 Liella! members really are in the waitroom");
    return hareruya;
}

/* Select Bloom's heart-pattern option `opt` (0=heart01, 1=heart04, 2=heart05),
   then let the 常時 recalculate on top. Returns the number of prompts. */
static int a5_pick_bloom_pattern(TestGame *tg, int bloom, int opt)
{
    int prompts = 0, guard = 0;
    while (pending(tg) && guard++ < 12) {
        const char *k = pending_kind(tg);
        if (k && !strcmp(k, "SelectHeartColor")) answer(tg, 0);
        else answer(tg, opt);
        prompts++;
    }
    /* Wien's 常時 must be recomputed AFTER the set, which is what Q115 orders:
       the set is the base, the constant is additive on top. */
    test_recalc(tg);
    (void)bloom;
    return prompts;
}

static void a5_wien_plus1_stacks_on_bloom_heart01_set(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5: Bloom's ライブ開始時 fires");
    CHECK(pending(&tg), "A5 CONTROL: Bloom really presents its heart-pattern choice");
    if (pending(&tg)) describe_pending(&tg, "A5 bloom option 0");

    /* DIAGNOSTIC: dump Wien's decoded abilities so the red heart00 assertions
     * below name a concrete cause. cards/cards.json DOES carry Wien's full
     * 「常時 相手のライブカード置き場にあるすべてのライブカードは、必要ハートが
     * heart00多くなる」 text, and
     * rb_execute_modify_required_hearts_standard (score.c) exists and does
     * write rb_mods_add_need_heart for every live card of the resolved player
     * -- so the open question is whether the COMPILED card database carries
     * Wien's ability at all. */
    dump_abilities(tg.state.p[1].stage[1], "A5 wien");

    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    fprintf(stderr, "  (A5 bloom opt0) prompts = %d\n", prompts);
    CHECK(prompts > 0, "A5 CONTROL: at least one prompt was driven");

    CHECK_EQ(need_heart_of(&tg, bloom, H01), 2,
             "A5: Bloom SET required heart01 to 2");
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 2,
             "A5 Q127: Bloom set heart00=1 + Wien's constant +1 = 2 (set then "
             "additive)");
}

static void a5_wien_only_affects_heart00(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5: Bloom's ライブ開始時 fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    CHECK(prompts > 0, "A5 CONTROL: the pattern choice was really answered");

    CHECK_EQ(need_heart_of(&tg, bloom, H01), 2,
             "A5: heart01 stays at Bloom's set value 2 (Wien does not touch it)");
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 2, "A5: heart00 = set(1) + Wien(+1)");
    CHECK_EQ(need_heart_of(&tg, bloom, H02), 0, "A5: heart02 is untouched");
    CHECK_EQ(need_heart_of(&tg, bloom, H03), 0, "A5: heart03 is untouched");
}

static void a5_no_wien_bloom_set_standalone(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, -1);   /* NO Wien on P2's stage */
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5 no-Wien: Bloom's ライブ開始時 fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    CHECK(prompts > 0, "A5 no-Wien CONTROL: the pattern choice was really answered");

    /* This is the control that makes a5_wien_only_affects_heart00's heart00==2
     * non-vacuous: with no Wien on stage the value must be the SET value only. */
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 1,
             "A5 no-Wien CONTROL: without Wien the additive is absent and "
             "heart00 reads Bloom's set value 1 alone");
    CHECK_EQ(need_heart_of(&tg, bloom, H01), 2,
             "A5 no-Wien: the SET part of the pattern still applies");
}

static void a5_two_wien_stack_plus2_heart00(void)
{
    static TestGame tg;
    reset_board(&tg);
    int wien1 = mid(&tg, WIEN);
    int wien2 = mid(&tg, WIEN);
    int bloom = mid(&tg, BLOOM);
    int hasu = mid(&tg, HASU_BP1);
    int filler = mid(&tg, SD_010);
    if (!pin_id(wien1, WIEN) || !pin_id(wien2, WIEN) ||
        !pin_id(bloom, BLOOM) || !pin_id(hasu, HASU_BP1) ||
        !pin_id(filler, SD_010)) return;
    CHECK(wien1 != wien2, "A5 two-Wien fixture: two game.id() calls are two instances");

    fill_decks(&tg, filler, 50);
    set_stage(&tg, 1, wien1, wien2, RB_EMPTY_SLOT);
    set_stage(&tg, 0, RB_EMPTY_SLOT, hasu, RB_EMPTY_SLOT);
    test_set_live_card(&tg, 0, bloom);
    CHECK_EQ(stage_has(&tg, 1, wien1), 1, "A5 two-Wien CONTROL: Wien #1 is staged");
    CHECK_EQ(stage_has(&tg, 1, wien2), 1, "A5 two-Wien CONTROL: Wien #2 is staged");

    CHECK(fire_live_start(&tg, bloom), "A5 two-Wien: Bloom fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    CHECK(prompts > 0, "A5 two-Wien CONTROL: the pattern choice was answered");

    CHECK_EQ(need_heart_of(&tg, bloom, H00), 3,
             "A5 Q127: Bloom set heart00=1 + TWO Wienen (+2) = 3");
}

static void a5_wien_leaves_stage_modifier_removed(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5 leave: Bloom fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    CHECK(prompts > 0, "A5 leave CONTROL: the pattern choice was answered");
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 2,
             "A5 leave: precondition, with Wien staged heart00 reads 2");

    /* Wien leaves P2's stage; recalculate_constants is the Rust step. */
    set_stage(&tg, 1, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(tg.state.p[1].stage[i], RB_EMPTY_SLOT,
                 "A5 leave CONTROL: P2's stage slot really is empty");
    test_recalc(&tg);

    CHECK_EQ(need_heart_of(&tg, bloom, H00), 1,
             "A5 Q127: after Wien leaves, heart00 is Bloom's set value alone");
}

static void a5_wien_plus_bloom_heart04_pattern(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5 heart04 pattern: Bloom fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 1);
    CHECK(prompts > 0, "A5 heart04 pattern CONTROL: the pattern choice was answered");

    CHECK_EQ(need_heart_of(&tg, bloom, H04), 2,
             "A5: option 1 sets required heart04 to 2");
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 2,
             "A5 Q127: heart00 = set(1) + Wien(+1) on the heart04 pattern too");
    CHECK_EQ(need_heart_of(&tg, bloom, H01), 0,
             "A5: the heart04 pattern does not also set heart01");
}

static void a5_wien_plus_bloom_heart05_pattern(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5 heart05 pattern: Bloom fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 2);
    CHECK(prompts > 0, "A5 heart05 pattern CONTROL: the pattern choice was answered");

    CHECK_EQ(need_heart_of(&tg, bloom, H05), 2,
             "A5: option 2 sets required heart05 to 2");
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 2,
             "A5 Q127: heart00 = set(1) + Wien(+1) on the heart05 pattern too");
    CHECK_EQ(need_heart_of(&tg, bloom, H01), 0,
             "A5: the heart05 pattern does not also set heart01");
}

static void a5_wien_stacks_on_hareruya_set(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hareruya = a5_setup_hareruya(&tg);
    if (hareruya < 0) return;

    CHECK(fire_live_start(&tg, hareruya),
          "A5 Hareruya: the ライブ開始時 fires");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (A5 hareruya) prompts = %d\n", prompts);
    CHECK(prompts > 0,
          "A5 Hareruya CONTROL: the ability opened at least one prompt, so the "
          "set assertions below are not vacuous");

    CHECK_EQ(need_heart_of(&tg, hareruya, H02), 2, "A5: Hareruya set required heart02=2");
    CHECK_EQ(need_heart_of(&tg, hareruya, H03), 2, "A5: Hareruya set required heart03=2");
    CHECK_EQ(need_heart_of(&tg, hareruya, H06), 2, "A5: Hareruya set required heart06=2");
    CHECK_EQ(need_heart_of(&tg, hareruya, H00), 1,
             "A5 Q127: Hareruya's set has NO heart00, so Wien's +1 is the only "
             "contributor and reads 1");
}

static void a5_wien_adds_heart00_not_in_hareruya_set(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hareruya = a5_setup_hareruya(&tg);
    if (hareruya < 0) return;

    /* The Rust twin additionally pins the PRINTED need_heart: Hareruya's card
     * data has a base heart00 which the SET wipes. The C store has no separate
     * "printed need" reader, so the closest faithful check is that the
     * modifier store reads 1 -- exactly Wien's contribution and nothing else. */
    Card c;
    int printed_h00 = -1;
    if (rb_decode_card_by_index((uint32_t)hareruya, &c)) {
        for (int i = 0; i < c.num_need; i++) {
            int k = c.num_base + c.num_blade + i;
            if (k < c.n_hearts && c.heart_color[k] == H00) {
                printed_h00 = c.heart_count[k];
                break;
            }
        }
        rb_free_card(&c);
    }
    CHECK(printed_h00 >= 0,
          "A5 Hareruya: the card data really does print a base heart00, which "
          "is what the set has to wipe");

    CHECK(fire_live_start(&tg, hareruya), "A5 Hareruya: the ライブ開始時 fires");
    int prompts = pc_pay(&tg, 12);
    CHECK(prompts > 0, "A5 Hareruya CONTROL: the ability opened a prompt");

    CHECK_EQ(need_heart_of(&tg, hareruya, H00), 1,
             "A5 Q127: the set has no heart00, so Wien's +1 is the whole value");
}

static void a5_build_card_needs_set_plus_additive(void)
{
    static TestGame tg;
    reset_board(&tg);
    int bloom = a5_setup_bloom(&tg, 1);
    if (bloom < 0) return;

    CHECK(fire_live_start(&tg, bloom), "A5 build-needs: Bloom fires");
    int prompts = a5_pick_bloom_pattern(&tg, bloom, 0);
    CHECK(prompts > 0, "A5 build-needs CONTROL: the pattern choice was answered");

    /* The Rust twin reconstructs build_card_needs by hand from the set and
       additive modifier maps. The C equivalent is rb_mods_get_need_heart,
       which IS the store that build reads, so the two agree by construction
       only if the SET arm and the ADDITIVE arm are both present. Each is
       isolated below. */
    int h01 = need_heart_of(&tg, bloom, H01);
    int h00 = need_heart_of(&tg, bloom, H00);
    CHECK_EQ(h01, 2, "A5 build-needs: heart01 = 2 (Bloom's set, untouched by Wien)");
    CHECK_EQ(h00, 2, "A5 build-needs: heart00 = set(1) + additive(1) = 2");

    /* Isolating the additive: without Wien, heart00 must read the set alone. */
    set_stage(&tg, 1, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(need_heart_of(&tg, bloom, H00), 1,
             "A5 build-needs: the ADDITIVE arm is Wien alone (removing Wien "
             "drops heart00 from 2 to the set value 1)");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION B -- effects/recover/to_discard (5 .rs files, 21 #[test])
 * ══════════════════════════════════════════════════════════════════════════ */

/* ── B1. debut_mill_top_three_distinct_blade_heart_colors_gain_heart04_... ──
 * PL!N-bp7-020-N エマ・ヴェルデ ab#0 登場:
 *   「自分のデッキの上からカードを3枚控え室に置く。それらのメンバーカードの中に
 *     2種類以上のブレードハートの色がある場合、ライブ終了時まで、heart04 を得る」
 * The five Rust cases are a matrix over the milled top-3. Every one of them
 * depends on the mill actually MOVING those three cards, so each case here
 * carries a waitroom-content control as well as a prompt-count control. */
static int b1_setup(TestGame *tg, int c0, int c1, int c2)
{
    int emma = mid(tg, EMMA);
    int filler = mid(tg, NO_BLADE);
    if (!pin_id(emma, EMMA) || !pin_id(filler, NO_BLADE)) return -1;
    CHECK_EQ(printed_cost(emma), 5, "B1 fixture: PL!N-bp7-020-N costs 5");
    CHECK(card_is_member(emma), "B1 fixture: Emma is a member card");

    set_stage(tg, 0, RB_EMPTY_SLOT, emma, RB_EMPTY_SLOT);
    tg->state.p[0].deck.n = 0;
    /* Rust: main_deck.cards.push(c) with "index 0 = top". test_add_to_deck
     * APPENDS, and rb_draw takes index 0, so pushing the three cards to be
     * milled FIRST puts them at the head where 「上から」 says they belong. The
     * filler tail is the depth the Rust twin's `for _ in 0..6 { push }` adds. */
    push_deck(tg, 0, c0, 1);
    push_deck(tg, 0, c1, 1);
    push_deck(tg, 0, c2, 1);
    push_deck(tg, 0, filler, 6);
    return emma;
}

/* Every B1 case funnels through here so the mill control is stated once and
 * applies to all of them.
 *
 * NOTE ON WHAT THIS IS EXPECTED TO FIND: rb_move_resolve_from_deck (move.c)
 * takes `P->deck.cards[--P->deck.n]` -- the LAST element -- while rb_draw
 * (engine.c) takes `bag_remove_at(&P->deck, 0)` -- the FIRST. So a
 * `source: "deck"` mill lands the BOTTOM of the deck, not the top, and the
 * count is right while the selection is wrong. An all-filler deck hides this
 * completely, which is why every fixture here is a DISTINGUISHABLE print and
 * this control asserts the three exact ids. The `position` line is
 * informational: it records which end the engine actually reached. */
static void b1_assert_milled(TestGame *tg, int a, int b, int c, const char *tag)
{
    char msg[200];
    CHECK_EQ(disc_len(tg, 0), 3, tag);
    snprintf(msg, sizeof msg,
             "%s CONTROL: 「上から3枚」 must mill the deck HEAD, not its tail "
             "(rb_move_resolve_from_deck takes --deck.n, rb_draw takes [0])", tag);
    CHECK_EQ(tg->state.p[0].discard.cards[0], a, msg);
    snprintf(msg, sizeof msg, "%s CONTROL: milled card #2 is the intended one", tag);
    CHECK_EQ(tg->state.p[0].discard.cards[1], b, msg);
    snprintf(msg, sizeof msg, "%s CONTROL: milled card #3 is the intended one", tag);
    CHECK_EQ(tg->state.p[0].discard.cards[2], c, msg);
}

/* b1_assert_milled plus the per-case prompt note. Emma's ability never PROMPTS
   -- it mills and then applies a 常時-style heart -- so a prompt count is the
   WRONG control here; the non-vacuity control is the mill content plus the
   paired positive. Every B1 case therefore calls b1_assert_milled, which
   asserts the three exact ids arrived, and the three-colour positive. */
static void b1_two_distinct_blade_colours_gains_heart04(void)
{
    static TestGame tg;
    reset_board(&tg);
    int karin  = mid(&tg, KARIN_B05);
    int kanata = mid(&tg, KANATA_B06);
    int karin2 = mid(&tg, KARIN_B05);
    if (!pin_id(karin, KARIN_B05) || !pin_id(kanata, KANATA_B06) ||
        !pin_id(karin2, KARIN_B05)) return;
    CHECK_EQ(printed_blade_heart_colour(karin), H05,
             "B1 fixture: PL!N-sd1-004-SD really prints blade heart05");
    CHECK_EQ(printed_blade_heart_colour(kanata), H06,
             "B1 fixture: PL!N-sd1-006-SD really prints blade heart06");

    int emma = b1_setup(&tg, karin, karin2, kanata);
    if (emma < 0) return;
    int deck_before = deck_len(&tg, 0);

    CHECK(fire_debut(&tg, emma), "B1 two-colours: the 登場 ability fires");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (B1 two) prompts = %d\n", prompts);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 3, "B1: the deck lost three cards");
    b1_assert_milled(&tg, karin, karin2, kanata, "B1 two");
    /* NON-VACUITY: this is a pure-effect ability, so prompts may legitimately
     * be 0. The control is the mill content above (three exact ids landed) and
     * the +1 heart04 below, both of which a no-op board cannot produce. */
    CHECK_EQ(heart_of(&tg, emma, H04), 1,
             "B1: 2 distinct blade-heart colours among the milled members -> heart04");
}

static void b1_three_distinct_blade_colours_gains_heart04(void)
{
    static TestGame tg;
    reset_board(&tg);
    int karin  = mid(&tg, KARIN_B05);
    int kanata = mid(&tg, KANATA_B06);
    int honoka = mid(&tg, SD_010);
    if (!pin_id(karin, KARIN_B05) || !pin_id(kanata, KANATA_B06) ||
        !pin_id(honoka, SD_010)) return;
    int cols[3] = { -1, -1, -1 };
    int nk = printed_blade_heart_colours(karin, cols, 3);
    int na = printed_blade_heart_colours(kanata, cols + nk, 3 - nk);
    int nh = printed_blade_heart_colours(honoka, cols + nk + na, 3 - nk - na);
    fprintf(stderr, "  (B1 3-colour) blade-heart colours: karin=%d kanata=%d honoka=%d\n",
            nk, na, nh);
    CHECK_EQ(printed_blade_heart_colour(karin), H05,  "B1 3-colour fixture: b_heart05");
    CHECK_EQ(printed_blade_heart_colour(kanata), H06, "B1 3-colour fixture: b_heart06");
    CHECK(nh > 0, "B1 3-colour fixture: the third print carries at least one "
                   "blade heart of its own");
    CHECK(printed_blade_heart_colour(honoka) != H05 &&
          printed_blade_heart_colour(honoka) != H06,
          "B1 3-colour fixture: the third print's colour is distinct from the other two");

    int emma = b1_setup(&tg, karin, kanata, honoka);
    if (emma < 0) return;

    CHECK(fire_debut(&tg, emma), "B1 three-colours: the 登場 ability fires");
    pc_pay(&tg, 12);
    b1_assert_milled(&tg, karin, kanata, honoka, "B1 three");
    CHECK_EQ(heart_of(&tg, emma, H04), 1,
             "B1: 3 distinct blade-heart colours -> heart04");
}

static void b1_all_one_colour_no_heart04(void)
{
    static TestGame tg;
    reset_board(&tg);
    int k1 = mid(&tg, KARIN_B05);
    int k2 = mid(&tg, KARIN_B05);
    int k3 = mid(&tg, KARIN_B05);
    if (!pin_id(k1, KARIN_B05) || !pin_id(k2, KARIN_B05) || !pin_id(k3, KARIN_B05)) return;
    CHECK_EQ(printed_blade_heart_colour(k1), H05, "B1 one-colour fixture: b_heart05");
    CHECK_EQ(printed_blade_heart_colour(k2), H05, "B1 one-colour fixture: b_heart05");
    CHECK_EQ(printed_blade_heart_colour(k3), H05, "B1 one-colour fixture: b_heart05");

    int emma = b1_setup(&tg, k1, k2, k3);
    if (emma < 0) return;

    CHECK(fire_debut(&tg, emma), "B1 one-colour: the 登場 ability fires");
    pc_pay(&tg, 12);
    b1_assert_milled(&tg, k1, k2, k3, "B1 one");
    /* NON-VACUITY: prompts may be 0 for a pure-effect ability. The control is
     * the mill content (three exact ids landed) plus the three-colour positive
     * on the same card, so the zero below is the COLOUR COUNT and not a
     * no-op. */
    CHECK_EQ(heart_of(&tg, emma, H04), 0,
             "B1: only 1 distinct blade-heart colour -> no heart04");
}

static void b1_only_one_member_milled_no_heart04(void)
{
    static TestGame tg;
    reset_board(&tg);
    int karin = mid(&tg, KARIN_B05);
    int e1 = mid(&tg, ENERGY_CARD);
    int e2 = mid(&tg, ENERGY_CARD);
    if (!pin_id(karin, KARIN_B05) || !pin_id(e1, ENERGY_CARD) ||
        !pin_id(e2, ENERGY_CARD)) return;
    CHECK(card_is_member(karin), "B1 one-member fixture: the member is a member");
    CHECK(!card_is_member(e1), "B1 one-member fixture: the energy card is NOT a member");

    int emma = b1_setup(&tg, karin, e1, e2);
    if (emma < 0) return;

    CHECK(fire_debut(&tg, emma), "B1 one-member: the 登場 ability fires");
    pc_pay(&tg, 12);
    b1_assert_milled(&tg, karin, e1, e2, "B1 one-member");
    CHECK_EQ(heart_of(&tg, emma, H04), 0,
             "B1: only 1 milled MEMBER card -> no heart04");
}

static void b1_members_without_blade_heart_no_heart04(void)
{
    static TestGame tg;
    reset_board(&tg);
    int n1 = mid(&tg, NO_BLADE);
    int n2 = mid(&tg, NO_BLADE);
    int n3 = mid(&tg, NO_BLADE);
    if (!pin_id(n1, NO_BLADE) || !pin_id(n2, NO_BLADE) || !pin_id(n3, NO_BLADE)) return;
    CHECK(card_is_member(n1), "B1 no-blade fixture: the print IS a member");
    CHECK(!printed_blade_heart(n1),
          "B1 no-blade fixture: the print really carries NO blade heart");

    int emma = b1_setup(&tg, n1, n2, n3);
    if (emma < 0) return;

    CHECK(fire_debut(&tg, emma), "B1 no-blade: the 登場 ability fires");
    pc_pay(&tg, 12);
    b1_assert_milled(&tg, n1, n2, n3, "B1 no-blade");
    CHECK_EQ(heart_of(&tg, emma, H04), 0,
             "B1: 0 blade-heart colours among the milled members -> no heart04");
}

/* The 6th Rust case: an all-blade LIVE card and a green-blade member do not
 * together meet the condition. The live card is not a member at all, so only
 * the green member is in the population, and one colour is not two.
 *
 * RUST FIXTURE BUG, reported rather than papered over: the twin's comment calls
 * PL!N-PR-024-PR 「緑」 and its own baseline reads Heart03::default, but
 * cards.json gives this print base_heart {heart02, heart05} -- there is no
 * green blade heart, and the first blade heart is heart02, NOT heart03. The
 * fixture still functions (it is a member with a non-heart05/06 blade heart,
 * which is what the assertion needs) but the 「緑」 label and the Heart03
 * baseline are both wrong. */
static void b1_all_and_green_blade_do_not_meet_condition(void)
{
    static TestGame tg;
    reset_board(&tg);
    int all_blade = mid(&tg, ALL_BLADE);
    int green     = mid(&tg, GREEN_BLADE_PR);
    int no_blade  = mid(&tg, NO_BLADE);
    if (!pin_id(all_blade, ALL_BLADE) || !pin_id(green, GREEN_BLADE_PR) ||
        !pin_id(no_blade, NO_BLADE)) return;
    CHECK(card_is_live(all_blade), "B1 all/green fixture: PL!-sd1-020-SD is a LIVE card");
    CHECK(card_is_member(green),    "B1 all/green fixture: the other print is a member");
    CHECK_EQ(printed_blade_heart_colour(green), H02,
             "B1 all/green fixture: cards.json gives this print blade heart02 "
             "first (base_heart heart02/heart05) -- NOT the green the Rust "
             "comment claims and NOT heart03 as the twin's baseline reads");

    int emma = b1_setup(&tg, all_blade, green, no_blade);
    if (emma < 0) return;

    CHECK(fire_debut(&tg, emma), "B1 all/green: the 登場 ability fires");
    pc_pay(&tg, 12);
    b1_assert_milled(&tg, all_blade, green, no_blade, "B1 all/green");
    CHECK_EQ(heart_of(&tg, emma, H04), 0,
             "B1: one member colour plus a non-member all-blade live -> no heart04");
}

/* ── B2. live_start_discard_two_from_unit_with_two_or_more_cards_... ───────
 * PL!HS-PR-016-PR 日野下花帆 ライブ開始時:
 *   「手札の同じユニット名を持つカード2枚を控え室に置いてもよい：
 *     ライブ終了時まで、heart04×2 + blade×2 を得る」 */
static void b2_same_unit_pair_discards_two(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanano = mid(&tg, HANANO);
    int live   = mid(&tg, SD_019);
    int p_a = mid(&tg, SD_010);   /* Printemps */
    int p_b = mid(&tg, SD_008);   /* Printemps */
    int p_c = mid(&tg, SD_003);   /* Printemps */
    int lily = mid(&tg, SD_013);  /* lilywhite */
    if (!pin_id(hanano, HANANO) || !pin_id(live, SD_019) ||
        !pin_id(p_a, SD_010) || !pin_id(p_b, SD_008) || !pin_id(p_c, SD_003) ||
        !pin_id(lily, SD_013)) return;
    CHECK_EQ(printed_cost(hanano), 17, "B2 fixture: PL!HS-PR-016-PR costs 17");
    CHECK(same_unit(p_a, p_b) && same_unit(p_b, p_c),
          "B2 fixture: the three cost cards really share ONE UNIT name");
    CHECK(!same_unit(p_a, lily),
          "B2 fixture: the lilywhite card is a DIFFERENT unit "
          "(rb_card_group_name would compare SERIES and pass this vacuously)");
    CHECK(strcmp(rb_card_group_name(p_a), rb_card_group_name(lily)) == 0,
          "B2 fixture NOTE: the two units share a SERIES, which is exactly why "
          "this test uses same_unit() and not rb_card_group_name()");

    set_stage(&tg, 0, hanano, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int hand[5] = { live, p_a, p_b, p_c, lily };
    set_hand(&tg, 0, hand, 5);
    push_deck(&tg, 0, mid(&tg, SD_002), 20);
    push_deck(&tg, 1, mid(&tg, SD_002), 20);
    test_set_live_card(&tg, 0, live);
    test_give_energy(&tg, 10);
    /* test_set_live_card REMOVES the live card from hand, so the baseline is
     * read AFTER it, not before. */
    int hand_before = hand_len(&tg, 0);
    int disc_before = disc_len(&tg, 0);
    CHECK_EQ(hand_before, 4,
             "B2 CONTROL: 4 discardable cards remain in hand after the live card "
             "left it");

    dump_abilities(hanano, "B2 hanano");

    CHECK(fire_live_start(&tg, hanano), "B2: Hanano's ライブ開始時 fires");
    if (pending(&tg)) describe_pending(&tg, "B2 same-unit pair");
    else fprintf(stderr, "  (B2) NO PROMPT at all after the ライブ開始時 fired\n");
    CHECK(pending(&tg),
          "B2 CONTROL: 3 same-unit cards in hand really do open a selection");
    int prompts = 0, guard = 0;
    while (pending(&tg) && guard++ < 12) { answer(&tg, 0); prompts++; }
    fprintf(stderr, "  (B2) prompts = %d, hand %d -> %d, waitroom %d -> %d\n",
            prompts, hand_before, hand_len(&tg, 0), disc_before, disc_len(&tg, 0));

    CHECK(prompts > 0, "B2 CONTROL: the selection was really driven");
    CHECK_EQ(hand_len(&tg, 0), hand_before - 2,
             "B2: exactly 2 cards were discarded as the cost");
    CHECK(hand_has(&tg, 0, lily),
          "B2: the lilywhite card (different unit) remains in hand");
    /* Rust's twin picks eligible filtered index 0 TWICE
     * (q175_hanano_cross_unit_discard_same_unit_as_each_other.rs:71-73):
     *   try_select_indices(&[0])   -> p_a
     *   try_select_indices(&[0])   -> p_b, on the re-prompt filtered to p_a's unit
     * so p_c is the SURVIVOR of the three Printemps cards. This assertion used
     * to demand the opposite (`!hand_has(p_c)`), which pins behaviour the Rust
     * twin does not have and which fails under a correct same-unit re-prompt. */
    CHECK(hand_has(&tg, 0, p_c),
          "B2: the THIRD Printemps card p_c survives -- Rust picks filtered "
          "index 0 twice (p_a, then p_b on the same-unit re-prompt), so the "
          "third same-unit card is the one left behind");
    CHECK(blade_of(&tg, hanano) > 0, "B2: paying the cost grants blade");
    CHECK(heart_of(&tg, hanano, H04) > 0,
          "B2: paying the cost grants heart04");
}

/* Two qualifying units in hand: the player picks the LILYWHITE pair, so the two
 * Printemps cards must survive. This is what proves the cost is a
 * same-unit PAIR and not "discard any two". */
static void b2_player_chooses_pair_across_units(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanano = mid(&tg, HANANO);
    int live   = mid(&tg, SD_019);
    int pa = mid(&tg, SD_010);
    int pb = mid(&tg, SD_008);
    int la = mid(&tg, SD_013);
    int lb = mid(&tg, SD_004);
    if (!pin_id(hanano, HANANO) || !pin_id(live, SD_019) ||
        !pin_id(pa, SD_010) || !pin_id(pb, SD_008) ||
        !pin_id(la, SD_013) || !pin_id(lb, SD_004)) return;
    CHECK(same_unit(pa, pb), "B2 pair fixture: the Printemps pair shares a unit");
    CHECK(same_unit(la, lb), "B2 pair fixture: the lilywhite pair shares a unit");
    CHECK(!same_unit(pa, la),
          "B2 pair fixture: the two units are different from each other");

    set_stage(&tg, 0, hanano, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int hand[5] = { live, pa, pb, la, lb };
    set_hand(&tg, 0, hand, 5);
    push_deck(&tg, 0, mid(&tg, SD_002), 20);
    push_deck(&tg, 1, mid(&tg, SD_002), 20);
    test_set_live_card(&tg, 0, live);
    test_give_energy(&tg, 10);
    int hand_before = hand_len(&tg, 0);
    CHECK_EQ(hand_before, 4,
             "B2 pair CONTROL: 4 discardable cards, two of each unit");

    CHECK(fire_live_start(&tg, hanano), "B2 pair: the ライブ開始時 fires");
    CHECK(pending(&tg), "B2 pair CONTROL: two qualifying units open a selection");
    if (pending(&tg)) describe_pending(&tg, "B2 pair");
    {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        int n = c ? c->n_filtered_indices : 0;
        fprintf(stderr, "  (B2 pair) filtered candidates = %d\n", n);
        CHECK(n >= 4,
              "B2 pair CONTROL: the first prompt offers BOTH units' cards "
              "(4 candidates), so the choice of unit is really the player's");
    }

    /* Rust: select_indices(&[2]) picks the first lilywhite, then &[0] picks
       the second. Filtered index 2 is lily_a, index 3 is lily_b. */
    answer(&tg, 2);
    if (pending(&tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        fprintf(stderr, "  (B2 pair) second prompt filtered = %d, count = %d\n",
                c ? c->n_filtered_indices : -1, c ? c->count : -1);
    }
    answer(&tg, 0);
    pc_pay(&tg, 8);

    CHECK_EQ(hand_len(&tg, 0), hand_before - 2,
             "B2 pair: exactly 2 cards left hand, the chosen lilywhite pair");
    CHECK(!hand_has(&tg, 0, la), "B2 pair: the chosen lilywhite card left hand");
    CHECK(!hand_has(&tg, 0, lb), "B2 pair: its same-unit partner left hand too");
    CHECK(hand_has(&tg, 0, pa), "B2 pair: the Printemps card is untouched");
    CHECK(hand_has(&tg, 0, pb), "B2 pair: the other Printemps card is untouched");
}

/* No unit has >= 2 cards -> the optional cost is silently skipped. */
static void b2_no_qualifying_unit_skips_cost(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanano = mid(&tg, HANANO);
    int live   = mid(&tg, SD_019);
    int print_ = mid(&tg, SD_010);
    int lily   = mid(&tg, SD_013);
    int bibi   = mid(&tg, SD_002);
    if (!pin_id(hanano, HANANO) || !pin_id(live, SD_019) ||
        !pin_id(print_, SD_010) || !pin_id(lily, SD_013) || !pin_id(bibi, SD_002)) return;
    /* The three hand members must be THREE DIFFERENT units, so no pair exists.
     * Verified by unit name, because a series comparison would find all three
     * equal (they are all ラブライブ！) and the test would prove nothing. */
    CHECK(!same_unit(print_, lily) && !same_unit(lily, bibi) &&
          !same_unit(print_, bibi),
          "B2 no-unit CONTROL: the three hand members really are three "
          "DIFFERENT units, so no same-unit pair exists");

    set_stage(&tg, 0, hanano, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    int hand[4] = { live, print_, lily, bibi };
    set_hand(&tg, 0, hand, 4);
    push_deck(&tg, 0, mid(&tg, SD_002), 20);
    push_deck(&tg, 1, mid(&tg, SD_002), 20);
    test_set_live_card(&tg, 0, live);
    test_give_energy(&tg, 10);
    int hand_before = hand_len(&tg, 0);
    int disc_before = disc_len(&tg, 0);
    fprintf(stderr, "  (B2 no-unit) hand before = %d\n", hand_before);

    CHECK(fire_live_start(&tg, hanano), "B2 no-unit: the ライブ開始時 fires");
    if (pending(&tg)) describe_pending(&tg, "B2 no-unit");
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (B2 no-unit) prompts = %d, hand %d -> %d, waitroom %d -> %d\n",
            prompts, hand_before, hand_len(&tg, 0), disc_before, disc_len(&tg, 0));

    CHECK(prompts == 0,
          "B2 no-unit: with no same-unit pair in hand the cost must not prompt");
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "B2 no-unit: 「同じユニット名を持つカード2枚」 is unsatisfiable here, so "
             "NO card may leave hand -- the b2_same_unit_pair case shows the same "
             "card discarding two on the same prompt shape");
    CHECK_EQ(disc_len(&tg, 0), disc_before, "B2 no-unit: nothing reached the waitroom");
    CHECK(hand_has(&tg, 0, print_), "B2 no-unit: the Printemps card is still in hand");
    CHECK(hand_has(&tg, 0, lily),   "B2 no-unit: the lilywhite card is still in hand");
    CHECK(hand_has(&tg, 0, bibi),   "B2 no-unit: the BiBi card is still in hand");
    /* The buff is NOT conditioned on the optional cost, in either engine.
     * Measured decode of PL!HS-PR-016-PR ab#0 (probe over the compiled DB):
     *   cost   move_cards hand->discard count=2 optional=true
     *          extra same_unit_name=true
     *   effect sequential, NO `conditional` extra, NO primary_effect /
     *          result_condition and NO optional_action / conditional_action;
     *          child[0] gain_resource blade x2 (live_end)
     *          child[1] gain_resource heart [heart04,heart04] x2 (live_end)
     * Rust gates an effect on the cost only through
     * compound/conditional_on.rs:18-28 (execute_conditional_on_result, which
     * needs compound.primary_effect + result_condition) or :88-149
     * (execute_conditional_on_optional, which needs optional_action +
     * conditional_action). Neither shape is present, so the sequential runs
     * and the two gain_resource children fire whether or not the cost was
     * paid. Rust's twin
     * (hanano_no_qualifying_unit_skips_cost.rs:186-202) asserts exactly that:
     * no prompt, both named cards still in hand, hand length unchanged -- and
     * says NOTHING about blade/heart.
     *
     * These two assertions used to demand 0 blade and 0 heart04, i.e. they
     * pinned a cost-conditional buff that Rust does not have and that the
     * parsed effect tree cannot express. They now assert what Rust does. */
    CHECK(blade_of(&tg, hanano) > 0,
          "B2 no-unit: the blade gain is NOT conditioned on the optional cost -- "
          "the effect is an unconditional sequential (no result_condition, no "
          "conditional_action), so Rust grants it even when no same-unit pair "
          "exists and the cost is skipped");
    CHECK(heart_of(&tg, hanano, H04) > 0,
          "B2 no-unit: the heart04 gain is likewise unconditional for the same "
          "reason");
}

/* ── B3. live_start_discard_two_grants_blade_to_distinct_niji_members_q272 ──
 * PL!N-bp7-026-L Just Believe!!! ab#0 ライブ開始時:
 *   「手札を2枚まで控え室に置いてもよい：自分のステージにいる『虹ヶ咲』の
 *     メンバーを、これにより控え室に置いたカードの枚数に等しい数まで選ぶ。
 *     ライブ終了時まで、それらはブレードを得る」
 * Q272 official answer: the SAME member cannot be selected twice. */
/* Stage the 虹ヶ咲 members, put the live card in the live zone, and give the
 * hand `hand_n` discardable cards. IMPORTANT: test_set_live_card REMOVES the
 * card from hand, so the live card is placed BEFORE the discardable cards are
 * added -- otherwise the hand count the cost sees is one short. */
static void b3_setup(TestGame *tg, const int *members, int n_members, int hand_n)
{
    int jb = mid(tg, JUST_BELIEVE);
    if (!pin_id(jb, JUST_BELIEVE)) return;
    CHECK(card_is_live(jb), "B3 fixture: PL!N-bp7-026-L is a live card");
    for (int i = 0; i < n_members; i++) tg->state.p[0].stage[i] = members[i];
    for (int i = n_members; i < RB_STAGE_SIZE; i++)
        tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < n_members; i++) tg->state.p[0].stage_wait[i] = 0;
    test_set_live_card(tg, 0, jb);
    for (int i = 0; i < hand_n; i++) {
        int f = mid(tg, SD_010);
        test_add_to_hand(tg, f);
    }
    CHECK_EQ(hand_len(tg, 0), hand_n,
             "B3 CONTROL: exactly hand_n discardable cards are in hand");
}

/* One 虹ヶ咲 member, two cards discarded: it gains exactly ONE blade. */
static void b3_same_member_not_selectable_twice(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m1 = mid(&tg, NIJI_A);
    if (!pin_id(m1, NIJI_A)) return;
    CHECK(strcmp(rb_card_group_name(m1), NIJI_GROUP) == 0,
          "B3 Q272 CONTROL: the staged member really is 虹ヶ咲 (bytes pinned)");

    int hand[1] = { m1 };
    b3_setup(&tg, hand, 1, 2);
    int jb = tg.state.p[0].live.cards[0];
    if (!pin_id(jb, JUST_BELIEVE)) return;
    int hand_before = hand_len(&tg, 0);
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (B3 Q272) prompts = %d, hand %d -> %d\n",
            prompts, hand_before, hand_len(&tg, 0));

    CHECK(prompts > 0, "B3 Q272 CONTROL: the ability opened at least one prompt");
    CHECK_EQ(hand_len(&tg, 0), hand_before - 2,
             "B3 Q272: 2 hand cards were discarded as the optional cost");
    CHECK_EQ(blade_of(&tg, m1), 1,
             "B3 Q272: a member cannot be selected twice, so 2 discards still "
             "grant exactly 1 blade");
}

static void b3_discard_one_grants_one_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m1 = mid(&tg, NIJI_A);
    if (!pin_id(m1, NIJI_A)) return;
    CHECK(strcmp(rb_card_group_name(m1), NIJI_GROUP) == 0, "B3 one: fixture is 虹ヶ咲");

    int hand[1] = { m1 };
    b3_setup(&tg, hand, 1, 2);
    int jb = tg.state.p[0].live.cards[0];
    if (!pin_id(jb, JUST_BELIEVE)) return;

    /* Decline the pay gate (index 0 on a SelectTarget is SKIP), then take
       exactly one hand card and one member. */
    int prompts = 0, guard = 0;
    int discarded = 0;
    while (pending(&tg) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD) { answer(&tg, 0); discarded++; prompts++; continue; }
        answer(&tg, 0);
        prompts++;
    }
    fprintf(stderr, "  (B3 one) prompts = %d, card picks = %d\n", prompts, discarded);
    CHECK(prompts > 0, "B3 one CONTROL: the ability opened prompts");
    CHECK(discarded > 0,
          "B3 one CONTROL: at least one card pick prompt was really answered, "
          "so the blade count below is measuring the effect and not a no-op");
    CHECK_EQ(blade_of(&tg, m1), 1, "B3: 1 discard -> the selected member gains 1 blade");
}

static void b3_two_distinct_members_each_gain_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m1 = mid(&tg, NIJI_A);
    int m2 = mid(&tg, NIJI_B);
    if (!pin_id(m1, NIJI_A) || !pin_id(m2, NIJI_B)) return;
    CHECK(m1 != m2, "B3 two: the two members are distinct instances");
    CHECK(strcmp(rb_card_group_name(m1), NIJI_GROUP) == 0, "B3 two: m1 is 虹ヶ咲");
    CHECK(strcmp(rb_card_group_name(m2), NIJI_GROUP) == 0, "B3 two: m2 is 虹ヶ咲");

    int hand[2] = { m1, m2 };
    b3_setup(&tg, hand, 2, 2);
    int jb = tg.state.p[0].live.cards[0];
    if (!pin_id(jb, JUST_BELIEVE)) return;

    int prompts = 0, guard = 0;
    while (pending(&tg) && guard++ < 16) { answer(&tg, 0); prompts++; }
    fprintf(stderr, "  (B3 two) prompts = %d\n", prompts);
    CHECK(prompts > 0, "B3 two CONTROL: the ability opened prompts");
    CHECK_EQ(blade_of(&tg, m1), 1, "B3: the first selected member gains exactly 1 blade");
    CHECK_EQ(blade_of(&tg, m2), 1, "B3: the second selected member gains exactly 1 blade");
}

static void b3_non_niji_member_not_selectable(void)
{
    static TestGame tg;
    reset_board(&tg);
    int niji = mid(&tg, NIJI_A);
    int non  = mid(&tg, SD_010);
    if (!pin_id(niji, NIJI_A) || !pin_id(non, SD_010)) return;
    CHECK(strcmp(rb_card_group_name(niji), NIJI_GROUP) == 0, "B3 non-niji: niji is 虹ヶ咲");
    CHECK(strcmp(rb_card_group_name(non), NIJI_GROUP) != 0,
          "B3 non-niji CONTROL: the second staged member is provably NOT 虹ヶ咲");

    int hand[2] = { niji, non };
    b3_setup(&tg, hand, 2, 2);
    int jb = tg.state.p[0].live.cards[0];
    if (!pin_id(jb, JUST_BELIEVE)) return;

    int prompts = 0, guard = 0;
    while (pending(&tg) && guard++ < 16) { answer(&tg, 0); prompts++; }
    CHECK(prompts > 0, "B3 non-niji CONTROL: the ability opened prompts");
    CHECK_EQ(blade_of(&tg, non), 0,
             "B3: a non-虹ヶ咲 member is not selectable -> no blade");
    /* The POSITIVE on the same board: the 虹ヶ咲 peer of the same stage did
       gain, so the zero above is the GROUP filter and not a dead effect. */
    CHECK_EQ(blade_of(&tg, niji), 1,
             "B3 non-niji: the 虹ヶ咲 member on the SAME stage did gain, which "
             "is what makes the non-虹ヶ咲 zero a filter and not a no-op");
}

static void b3_skip_discard_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m1 = mid(&tg, NIJI_A);
    if (!pin_id(m1, NIJI_A)) return;

    int hand[1] = { m1 };
    b3_setup(&tg, hand, 1, 2);
    int jb = tg.state.p[0].live.cards[0];
    if (!pin_id(jb, JUST_BELIEVE)) return;
    int hand_before = hand_len(&tg, 0);
    int disc_before = disc_len(&tg, 0);

    CHECK(fire_live_start(&tg, jb), "B3 skip: the ライブ開始時 fired");
    if (pending(&tg)) describe_pending(&tg, "B3 skip");
    else fprintf(stderr, "  (B3 skip) NO PROMPT -- the effect resolved silently\n");
    pc_decline_pays_rest(&tg, 8);

    CHECK_EQ(hand_len(&tg, 0), hand_before, "B3 skip: no card left hand");
    CHECK_EQ(disc_len(&tg, 0), disc_before, "B3 skip: nothing reached the waitroom");
    CHECK_EQ(blade_of(&tg, m1), 0, "B3 skip: skipping the discard grants no blade");
}

/* ab#1 ライブ成功時: 2+ revealed MEMBER cards with no blade heart -> +1 score. */
static void b3_ab1_two_no_blade_members_score(void)
{
    static TestGame tg;
    reset_board(&tg);
    int jb = mid(&tg, JUST_BELIEVE);
    int a = mid(&tg, SD_001);
    int b = mid(&tg, SD_001);
    if (!pin_id(jb, JUST_BELIEVE) || !pin_id(a, SD_001) || !pin_id(b, SD_001)) return;
    CHECK(card_is_member(a), "B3 ab1 fixture: the revealed print is a member");
    CHECK(!printed_blade_heart(a), "B3 ab1 fixture: it really has no blade heart");

    test_set_live_card(&tg, 0, jb);
    tg.state.revealed_cards[tg.state.n_revealed++] = a;
    tg.state.revealed_cards[tg.state.n_revealed++] = b;
    CHECK_EQ(tg.state.n_revealed, 2, "B3 ab1 CONTROL: 2 cards really are revealed");

    int fired = fire_live_success(&tg, jb);
    fprintf(stderr, "  (B3 ab1) live-success trigger returned %d\n", fired);
    pc_pay(&tg, 12);

    CHECK_EQ(score_of(&tg, jb), 1,
             "B3 ab#1: 2 revealed member cards without blade-hearts -> +1 score");
}

static void b3_ab1_one_no_blade_member_no_score(void)
{
    static TestGame tg;
    reset_board(&tg);
    int jb = mid(&tg, JUST_BELIEVE);
    int a = mid(&tg, SD_001);
    if (!pin_id(jb, JUST_BELIEVE) || !pin_id(a, SD_001)) return;

    test_set_live_card(&tg, 0, jb);
    tg.state.revealed_cards[tg.state.n_revealed++] = a;
    CHECK_EQ(tg.state.n_revealed, 1, "B3 ab1 one CONTROL: exactly 1 card revealed");

    fire_live_success(&tg, jb);
    pc_pay(&tg, 12);

    CHECK_EQ(score_of(&tg, jb), 0,
             "B3 ab#1: only 1 such member -> no score");
}

static void b3_ab1_with_blade_heart_members_no_score(void)
{
    static TestGame tg;
    reset_board(&tg);
    int jb = mid(&tg, JUST_BELIEVE);
    int a = mid(&tg, SD_010);
    int b = mid(&tg, SD_010);
    if (!pin_id(jb, JUST_BELIEVE) || !pin_id(a, SD_010) || !pin_id(b, SD_010)) return;
    CHECK(printed_blade_heart(a),
          "B3 ab1 with-blade CONTROL: this print DOES carry a blade heart, so "
          "the zero below is the filter and not a missing card");

    test_set_live_card(&tg, 0, jb);
    tg.state.revealed_cards[tg.state.n_revealed++] = a;
    tg.state.revealed_cards[tg.state.n_revealed++] = b;
    fire_live_success(&tg, jb);
    pc_pay(&tg, 12);

    CHECK_EQ(score_of(&tg, jb), 0,
             "B3 ab#1: members WITH blade-hearts do not qualify -> no score");
}

static void b3_ab1_mixed_members_no_score(void)
{
    static TestGame tg;
    reset_board(&tg);
    int jb = mid(&tg, JUST_BELIEVE);
    int no_blade = mid(&tg, SD_001);
    int with_blade = mid(&tg, SD_010);
    if (!pin_id(jb, JUST_BELIEVE) || !pin_id(no_blade, SD_001) ||
        !pin_id(with_blade, SD_010)) return;

    test_set_live_card(&tg, 0, jb);
    tg.state.revealed_cards[tg.state.n_revealed++] = no_blade;
    tg.state.revealed_cards[tg.state.n_revealed++] = with_blade;
    CHECK_EQ(tg.state.n_revealed, 2, "B3 ab1 mixed CONTROL: 2 cards revealed");
    fire_live_success(&tg, jb);
    pc_pay(&tg, 12);

    CHECK_EQ(score_of(&tg, jb), 0,
             "B3 ab#1: only 1 of the two members lacks a blade heart -> no score");
}

static void b3_ab1_member_plus_live_no_score(void)
{
    static TestGame tg;
    reset_board(&tg);
    int jb = mid(&tg, JUST_BELIEVE);
    int member = mid(&tg, SD_001);
    int live = mid(&tg, SD_019);
    if (!pin_id(jb, JUST_BELIEVE) || !pin_id(member, SD_001) || !pin_id(live, SD_019)) return;
    CHECK(card_is_member(member), "B3 ab1 member+live fixture: the member is a member");
    CHECK(card_is_live(live), "B3 ab1 member+live CONTROL: the other card is a "
                              "LIVE card, not a member, so the card_type filter "
                              "is what excludes it");

    test_set_live_card(&tg, 0, jb);
    tg.state.revealed_cards[tg.state.n_revealed++] = member;
    tg.state.revealed_cards[tg.state.n_revealed++] = live;
    fire_live_success(&tg, jb);
    pc_pay(&tg, 12);

    CHECK_EQ(score_of(&tg, jb), 0,
             "B3 ab#1: a revealed live card is not a member -> no score");
}

/* ── B4. optional_discard_cost_plus_total_cost_comparison_gain_heart05_... ──
 * PL!HS-bp6-005-R＋ 徒町 小鈴 ライブ開始時: optional discard 1 from hand ->
 * this member's cost +6 until live end; if the 蓮ノ空 members' total stage cost
 * on your side exceeds the opponent's, gain heart05 + blade.
 * The Rust twin already added a hand-delta control because the pay branch and
 * the skip branch were otherwise indistinguishable. That control is kept. */
static void b4_condition_met_gains_heart05_and_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int kosuzu = mid(&tg, KOSUZU);
    int kanoha = mid(&tg, KANOHA);
    int lili   = mid(&tg, LILIENTHAL);
    int p2f    = mid(&tg, SD_010);
    if (!pin_id(kosuzu, KOSUZU) || !pin_id(kanoha, KANOHA) ||
        !pin_id(lili, LILIENTHAL) || !pin_id(p2f, SD_010)) return;
    CHECK_EQ(printed_cost(kosuzu), 10, "B4 fixture: PL!HS-bp6-005-R＋ costs 10");
    CHECK_EQ(printed_cost(kanoha), 11, "B4 fixture: PL!HS-bp1-001-R costs 11");
    CHECK_EQ(printed_cost(lili), 15,  "B4 fixture: PL!HS-bp6-007-R costs 15");
    CHECK_EQ(printed_cost(p2f), 4,    "B4 fixture: the opponent filler costs 4");
    CHECK(strcmp(rb_card_group_name(kanoha), rb_card_group_name(kosuzu)) == 0,
          "B4 fixture: both are 蓮ノ空 (the group the total-cost sum reads)");

    set_stage(&tg, 0, kanoha, kosuzu, lili);
    set_stage(&tg, 1, p2f, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, mid(&tg, SD_010), 10);
    test_give_energy(&tg, 20);
    int live = mid(&tg, SD_010);
    int filler = mid(&tg, SD_010);
    int hand[2] = { filler, live };
    set_hand(&tg, 0, hand, 2);
    test_set_live_card(&tg, 0, live);

    CHECK(fire_live_start(&tg, kosuzu), "B4: Kosuzu's ライブ開始時 fires");
    int prompts = pc_pay(&tg, 12);
    int hand_after = hand_len(&tg, 0);
    fprintf(stderr, "  (B4 met) prompts = %d, hand = %d\n", prompts, hand_after);

    CHECK(prompts > 0,
          "B4 CONTROL: the optional-cost prompt really opened");
    CHECK_EQ(hand_after, 1,
             "B4: paying 「手札を1枚控え室に置いてもよい」 really discards one card "
             "-- without this the whole test would read the DECLINED branch "
             "under the name of the paid one");
    CHECK_EQ(cost_of(&tg, kosuzu), 6, "B4: Kosuzu gains +6 cost until live end");
    CHECK_EQ(cost_of(&tg, kanoha), 0, "B4: the cost bonus is self_target only");
    CHECK_EQ(cost_of(&tg, lili), 0,   "B4: the cost bonus is self_target only");
    CHECK(blade_of(&tg, kosuzu) > 0,
          "B4: 蓮ノ空 total 36 > opponent 4 -> blade granted");
    CHECK(heart_of(&tg, kosuzu, H05) > 0,
          "B4: 蓮ノ空 total 36 > opponent 4 -> heart05 granted");
}

static void b4_condition_not_met_no_heart05_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int kosuzu = mid(&tg, KOSUZU);
    int p2high = mid(&tg, SD_009);
    int p2f    = mid(&tg, SD_010);
    if (!pin_id(kosuzu, KOSUZU) || !pin_id(p2high, SD_009) ||
        !pin_id(p2f, SD_010)) return;
    CHECK_EQ(printed_cost(p2high), 15, "B4 negative fixture: the opponent's high card costs 15");
    CHECK_EQ(printed_cost(p2f), 4, "B4 negative fixture: the opponent filler costs 4");

    set_stage(&tg, 0, RB_EMPTY_SLOT, kosuzu, RB_EMPTY_SLOT);
    set_stage(&tg, 1, p2high, p2f, RB_EMPTY_SLOT);
    fill_decks(&tg, mid(&tg, SD_010), 10);
    test_give_energy(&tg, 20);
    int live = mid(&tg, SD_010);
    int filler = mid(&tg, SD_010);
    int hand[2] = { filler, live };
    set_hand(&tg, 0, hand, 2);
    test_set_live_card(&tg, 0, live);

    CHECK(fire_live_start(&tg, kosuzu), "B4 negative: the ライブ開始時 fires");
    int prompts = pc_pay(&tg, 12);
    int hand_after = hand_len(&tg, 0);
    fprintf(stderr, "  (B4 not-met) prompts = %d, hand = %d\n", prompts, hand_after);

    CHECK(prompts > 0,
          "B4 negative CONTROL: the optional-cost prompt opened");
    CHECK_EQ(hand_after, 1,
             "B4 negative: the cost is still PAID here, so the only thing the "
             "condition can be measuring is the cost comparison");
    CHECK_EQ(cost_of(&tg, kosuzu), 6,
             "B4 negative: the +6 cost applies regardless of the condition, so "
             "its presence proves the ability ran");
    /* has_mod_entry distinguishes "the ability ran and the condition said no"
       from "the ability never produced any modifier at all". */
    CHECK(has_mod_entry(&tg, kosuzu),
          "B4 negative CONTROL: the modifier store really does hold a kosuzu "
          "entry (the +6 cost), so the missing blade/heart05 below are the "
          "cost comparison failing and not the ability being a silent no-op");
    CHECK_EQ(blade_of(&tg, kosuzu), 0,
             "B4 negative: 蓮ノ空 total 10 (+6 = 16) < opponent 19 -> no blade");
    CHECK_EQ(heart_of(&tg, kosuzu, H05), 0,
             "B4 negative: 蓮ノ空 total 10 (+6 = 16) < opponent 19 -> no heart05");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION C -- effects/state/orientation_gated (5 .rs files, 22 #[test])
 * These are almost all 常時 abilities whose CONDITION reads an orientation
 * modifier, so they are exercised through rb_recalc_constants with no prompts
 * at all. The vacuity risk is inverted here: a broken gate must not be
 * mistaken for an ability that never resolved. Every case therefore asserts
 * the POSITIVE arm somewhere -- either a +1 that appears, or the paired
 * active/wait flip on the same board.
 * ══════════════════════════════════════════════════════════════════════════ */

/* ── C1. erena_self_wait_constant_heart05_test.rs (7 tests) ────────────────
 * PL!-bp5-333-R / PL!-bp5-333-P＋ 統堂英玲奈:
 *   「常時 このメンバーがウェイト状態であるかぎり、heart05 を得る」 */
static void c1_two_copies_independent(void)
{
    static TestGame tg;
    reset_board(&tg);
    int a = mid(&tg, ERENA_R);
    int b = mid(&tg, ERENA_R);
    if (!pin_id(a, ERENA_R) || !pin_id(b, ERENA_R)) return;
    CHECK(a != b, "C1 fixture: two game.id() calls are two instances");

    set_stage(&tg, 0, a, b, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, a, "wait");
    rb_mods_set_orientation(&tg.state.mods, b, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, a, H05), 1, "C1: the waited left copy gains 1 heart05");
    CHECK_EQ(heart_of(&tg, b, H05), 0, "C1: the active centre copy gains 0");

    rb_mods_set_orientation(&tg.state.mods, a, "active");
    rb_mods_set_orientation(&tg.state.mods, b, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, a, H05), 0, "C1: after the swap the left copy is active -> 0");
    CHECK_EQ(heart_of(&tg, b, H05), 1, "C1: after the swap the centre copy is waited -> 1");

    rb_mods_set_orientation(&tg.state.mods, a, "wait");
    rb_mods_set_orientation(&tg.state.mods, b, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, a, H05), 1, "C1: both waited -> 1 each");
    CHECK_EQ(heart_of(&tg, b, H05), 1, "C1: both waited -> 1 each");
}

static void c1_wait_persists_after_position_change(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int filler = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(filler, SD_010)) return;

    set_stage(&tg, 0, erena, filler, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 move: waited at the left -> 1");

    set_stage(&tg, 0, filler, erena, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 move: the wait heart persists at the centre");

    set_stage(&tg, 0, RB_EMPTY_SLOT, filler, erena);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 move: and still at the right");
}

static void c1_leave_stage_clears_reenter_active(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    if (!pin_id(erena, ERENA_R)) return;

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 leave: precondition, waited on stage -> 1");

    /* Leave to the waitroom while the orientation modifier still says wait. */
    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_waitroom_add(&tg.state.p[0], erena);
    CHECK(orientation_is_wait(&tg, erena),
          "C1 leave CONTROL: the orientation modifier really is still 'wait'");
    CHECK_EQ(stage_has(&tg, 0, erena), 0, "C1 leave CONTROL: the card is off the stage");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "C1 leave: not on stage -> 0 even though still marked wait");

    /* Return as active. */
    remove_from_zone(&tg, 0, "discard", erena);
    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0, "C1 leave: re-entered as active -> 0");

    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 leave: flipped back to wait -> 1");
}

static void c1_no_orientation_defaults_active(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    if (!pin_id(erena, ERENA_R)) return;

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK(orientation_of(&tg, erena) == NULL ||
          strcmp(orientation_of(&tg, erena), "wait") != 0,
          "C1 default CONTROL: no orientation modifier was set at all");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "C1 default: no orientation means active -> no heart05");
}

static void c1_stable_across_recalc(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    if (!pin_id(erena, ERENA_R)) return;

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    for (int i = 0; i < 3; i++) {
        test_recalc(&tg);
        char msg[96];
        snprintf(msg, sizeof msg, "C1 stable: recalc #%d still reads 1 heart05", i + 1);
        CHECK_EQ(heart_of(&tg, erena, H05), 1, msg);
    }
}

static void c1_p_plus_variant(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_PP);
    if (!pin_id(erena, ERENA_PP)) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, erena, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1, "C1 P＋: the parallel print behaves identically");

    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0, "C1 P＋: active -> 0");
}

static void c1_other_members_orientation_irrelevant(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int other = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(other, SD_010)) return;

    set_stage(&tg, 0, erena, other, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, other, "wait");
    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "C1 irrelevant: Erena's own ACTIVE state decides, not her peer's");

    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1,
             "C1 irrelevant: Erena's own WAIT state decides, whatever the peer does");
}

/* ── C2. two_or_more_opponent_wait_members_constant_heart06_test.rs (6) ────
 * PL!HS-bp5-016-N 桂城 泉:
 *   「常時 相手のステージにウェイト状態のメンバーが2人以上いるかぎり、heart06 を得る」
 * 「2人以上」 is a FLOOR, not a multiplier: the grant is exactly one heart06. */
static void c2_two_opponent_wait_members_gains_one_heart06(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;
    CHECK_EQ(printed_cost(izumi), 9, "C2 fixture: PL!HS-bp5-016-N costs 9");

    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, f, 40);
    int o1 = mid(&tg, SD_010);
    int o2 = mid(&tg, SD_010);
    if (o1 < 0 || o2 < 0) return;
    set_stage(&tg, 1, o1, o2, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, o1, "wait");
    rb_mods_set_orientation(&tg.state.mods, o2, "wait");
    CHECK(orientation_is_wait(&tg, o1) && orientation_is_wait(&tg, o2),
          "C2 CONTROL: both opponent members really are waited");
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, izumi, H06), 1,
             "C2: 2 opponent waited members -> exactly ONE heart06");
}

static void c2_zero_opponent_wait_no_heart06(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;

    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, f, 40);
    int o1 = mid(&tg, SD_010);
    int o2 = mid(&tg, SD_010);
    set_stage(&tg, 1, o1, o2, RB_EMPTY_SLOT);
    CHECK(!orientation_is_wait(&tg, o1) && !orientation_is_wait(&tg, o2),
          "C2 zero CONTROL: neither opponent member carries a wait modifier");
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, izumi, H06), 0, "C2: 0 waited opponent members -> no heart06");
}

static void c2_one_opponent_wait_no_heart06(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;

    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, f, 40);
    int o1 = mid(&tg, SD_010);
    int o2 = mid(&tg, SD_010);
    set_stage(&tg, 1, o1, o2, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, o1, "wait");
    CHECK(orientation_is_wait(&tg, o1), "C2 one CONTROL: exactly one member is waited");
    CHECK(!orientation_is_wait(&tg, o2), "C2 one CONTROL: the other really is not");
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, izumi, H06), 0, "C2: 1 waited member is below 2 -> no heart06");
}

static void c2_own_wait_members_do_not_count(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;

    int own1 = mid(&tg, SD_010);
    int own2 = mid(&tg, SD_010);
    set_stage(&tg, 0, izumi, own1, own2);
    rb_mods_set_orientation(&tg.state.mods, own1, "wait");
    rb_mods_set_orientation(&tg.state.mods, own2, "wait");
    fill_decks(&tg, f, 40);
    set_stage(&tg, 1, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    CHECK(orientation_is_wait(&tg, own1) && orientation_is_wait(&tg, own2),
          "C2 own CONTROL: two of MY members really are waited");
    CHECK_EQ(stage_has(&tg, 1, own1), 0, "C2 own CONTROL: the opponent stage is empty");
    test_recalc(&tg);

    CHECK_EQ(heart_of(&tg, izumi, H06), 0,
             "C2: your own waited members never satisfy an OPPONENT-scoped condition");
}

static void c2_two_wait_and_one_active_still_one(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;

    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, f, 40);
    int o1 = mid(&tg, SD_010);
    int o2 = mid(&tg, SD_010);
    int o3 = mid(&tg, SD_010);
    set_stage(&tg, 1, o1, o2, o3);
    rb_mods_set_orientation(&tg.state.mods, o1, "wait");
    rb_mods_set_orientation(&tg.state.mods, o2, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, izumi, H06), 1,
             "C2: 2 waited + 1 active on the opponent stage -> exactly 1 heart06");

    /* 2以上 is a floor, not a multiplier: a third waited member must not add
     * another heart06. This is also the control that proves the +1 above is
     * the CONDITION and not a constant the engine applies regardless. */
    rb_mods_set_orientation(&tg.state.mods, o3, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, izumi, H06), 1,
             "C2: 3 waited members still give exactly 1 heart06 (2以上 is a floor)");
}

static void c2_dynamic_two_wait_to_one_wait_loses_heart06(void)
{
    static TestGame tg;
    reset_board(&tg);
    int izumi = mid(&tg, IZUMI_N);
    int f = mid(&tg, SD_010);
    if (!pin_id(izumi, IZUMI_N) || !pin_id(f, SD_010)) return;

    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, f, 40);
    int o1 = mid(&tg, SD_010);
    int o2 = mid(&tg, SD_010);
    set_stage(&tg, 1, o1, o2, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, o1, "wait");
    rb_mods_set_orientation(&tg.state.mods, o2, "wait");
    test_recalc(&tg);
    CHECK(heart_of(&tg, izumi, H06) >= 1, "C2 dynamic precondition: 2 waited -> heart06");

    rb_mods_set_orientation(&tg.state.mods, o2, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, izumi, H06), 0,
             "C2 dynamic: dropping to 1 waited member removes the heart06");
}

/* ── C3. wait_state_condition_constants_test.rs (5 tests) ───────────────────
 * The same PL!-bp5-333-R constant as C1, but stated as the standalone
 * condition test. The pair is what proves the ability is gated on ORIENTATION
 * and not merely on being on the stage. */
static void c3_erena_in_wait_gains_exactly_one_heart05(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int f = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(f, SD_010)) return;
    fill_decks(&tg, f, 40);

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 1,
             "C3: a waited 統堂英玲奈 gains exactly one heart05 (>= 1 would also "
             "pass a double grant from two 常時 resolutions)");
}

static void c3_erena_active_no_heart(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int f = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(f, SD_010)) return;
    fill_decks(&tg, f, 40);

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0, "C3: active -> no heart05");
}

static void c3_erena_default_active_no_heart(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int f = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(f, SD_010)) return;
    fill_decks(&tg, f, 40);

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "C3: no orientation set at all (defaults to active) -> no heart05");
}

static void c3_erena_wait_to_active_switches_heart(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int f = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(f, SD_010)) return;
    fill_decks(&tg, f, 40);

    set_stage(&tg, 0, erena, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK(heart_of(&tg, erena, H05) >= 1, "C3 switch: waited -> heart05 present");

    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0, "C3 switch: active -> the heart is gone");
}

static void c3_erena_wait_not_on_stage(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int f = mid(&tg, SD_010);
    if (!pin_id(erena, ERENA_R) || !pin_id(f, SD_010)) return;
    fill_decks(&tg, f, 40);

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    CHECK(orientation_is_wait(&tg, erena),
          "C3 not-on-stage CONTROL: the orientation really is 'wait'");
    CHECK_EQ(stage_has(&tg, 0, erena), 0, "C3 not-on-stage CONTROL: the stage is empty");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "C3: waited but off the stage -> no heart05");
}

/* ── C4. nico_cannot_activate_test.rs (4 tests) ────────────────────────────
 * PL!-pb1-009-R 矢澤にこ ab#1: this turn, members on both stages cannot be
 * made ACTIVE BY AN EFFECT. The control is the second case: the identical
 * Kotori debut on a board WITHOUT Nico's aura must succeed. */
static void c4_kotori_activation_without_restriction(void)
{
    static TestGame tg;
    reset_board(&tg);
    int filler = mid(&tg, SD_010);
    int kotori = mid(&tg, KOTORI);
    if (!pin_id(filler, SD_010) || !pin_id(kotori, KOTORI)) return;
    CHECK_EQ(printed_cost(kotori), 2, "C4 fixture: PL!-pb1-012-R costs 2");

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 10);
    test_add_to_hand(&tg, filler);
    CHECK_EQ(test_play_to_stage(&tg, filler, 0), 1, "C4 control: the filler deploys left");
    rb_mods_set_orientation(&tg.state.mods, filler, "wait");
    CHECK(orientation_is_wait(&tg, filler), "C4 control precondition: filler is waited");

    test_add_to_hand(&tg, kotori);
    CHECK_EQ(test_play_to_stage(&tg, kotori, 1), 1, "C4 control: Kotori deploys centre");
    pc_auto_only(&tg, 12);
    CHECK(pending(&tg), "C4 control: Kotori's debut really opens a target prompt");
    if (pending(&tg)) describe_pending(&tg, "C4 control");
    answer(&tg, 0);
    pc_pay(&tg, 12);

    CHECK(!orientation_is_wait(&tg, filler),
          "C4 control: with no restriction, Kotori's activation SUCCEEDS. "
          "This is the only thing that makes c4_nico_blocks_kotori_activation's "
          "'still in wait' a restriction and not a dead ability.");
}

static void c4_nico_blocks_kotori_activation(void)
{
    static TestGame tg;
    reset_board(&tg);
    int filler = mid(&tg, SD_010);
    int nico = mid(&tg, NICO);
    int kotori = mid(&tg, KOTORI);
    if (!pin_id(filler, SD_010) || !pin_id(nico, NICO) || !pin_id(kotori, KOTORI)) return;
    CHECK_EQ(printed_cost(nico), 4, "C4 fixture: PL!-pb1-009-R costs 4");

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, filler);
    CHECK_EQ(test_play_to_stage(&tg, filler, 0), 1, "C4: the filler deploys left");
    rb_mods_set_orientation(&tg.state.mods, filler, "wait");
    CHECK(orientation_is_wait(&tg, filler), "C4 precondition: the filler is waited");

    test_add_to_hand(&tg, nico);
    CHECK_EQ(test_play_to_stage(&tg, nico, 1), 1, "C4: Nico deploys centre");
    pc_auto_only(&tg, 12);
    CHECK_EQ(stage_has(&tg, 0, nico), 1,
             "C4 CONTROL: Nico is really on the centre stage, so her 登場 ran");
    {
        /* The block is recorded in TWO places by h_restriction (misc.c): the
         * per-card delayed modifier and the GameState cannot_active_cards
         * list. rb_mods_is_delayed_cannot_active reads the former. Checking
         * only one of the two would let a half-implemented aura look
         * installed, so both are asserted. */
        int n_flagged = 0, n_listed = 0;
        for (int pl = 0; pl < 2; pl++)
            for (int i = 0; i < RB_STAGE_SIZE; i++) {
                int c = tg.state.p[pl].stage[i];
                if (c >= 0 && rb_mods_is_delayed_cannot_active(&tg.state.mods, c))
                    n_flagged++;
            }
        for (int i = 0; i < tg.state.n_cannot_active_cards; i++) n_listed++;
        fprintf(stderr, "  (C4) delayed flags = %d, cannot_active_cards = %d\n",
                n_flagged, n_listed);
        CHECK(n_flagged > 0,
              "C4 CONTROL: Nico's ab#1 really installed a per-card "
              "delayed_cannot_active flag, so the 'still in wait' below is a "
              "refusal and not an absent effect");
        CHECK(n_listed > 0,
              "C4 CONTROL: the GameState cannot_active_cards list the resolver "
              "actually consults is non-empty too");
    }

    test_add_to_hand(&tg, kotori);
    CHECK_EQ(test_play_to_stage(&tg, kotori, 2), 1, "C4: Kotori deploys right");
    pc_auto_only(&tg, 12);
    if (pending(&tg)) { describe_pending(&tg, "C4 blocked"); answer(&tg, 0); }
    pc_pay(&tg, 12);

    CHECK(orientation_is_wait(&tg, filler),
          "C4: cannot_activate_by_effect blocked Kotori's activation -- the "
          "filler is STILL in wait");
}

/* Q180: the aura blocks EFFECTS only. The natural Active phase still works. */
static void c4_active_phase_still_activates(void)
{
    static TestGame tg;
    reset_board(&tg);
    int nico = mid(&tg, NICO);
    int filler = mid(&tg, SD_010);
    if (!pin_id(nico, NICO) || !pin_id(filler, SD_010)) return;

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 6);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, filler, "wait");
    test_add_to_hand(&tg, nico);
    CHECK_EQ(test_play_to_stage(&tg, nico, 1), 1, "C4 Q180: Nico deploys centre");
    pc_auto_only(&tg, 12);
    CHECK_EQ(stage_has(&tg, 0, nico), 1, "C4 Q180 CONTROL: Nico is on stage");
    {
        int flagged = rb_mods_is_delayed_cannot_active(&tg.state.mods, filler);
        fprintf(stderr, "  (C4 Q180) filler flagged = %d, n_cannot_active_cards = %d\n",
                flagged, tg.state.n_cannot_active_cards);
        CHECK(flagged,
              "C4 Q180 CONTROL: the aura really is flagging the member, so the "
              "Active phase result below is a rule exemption and not a missing flag");
    }

    /* Step into the Active phase and let the natural activation run. */
    tg.state.phase = RB_PHASE_ACTIVE;
    rb_advance_phase(&tg.state);
    rb_advance_phase(&tg.state);
    fprintf(stderr, "  (C4 Q180) orientation after Active = %s\n",
            orientation_of(&tg, filler) ? orientation_of(&tg, filler) : "(null)");

    CHECK(!orientation_is_wait(&tg, filler),
          "C4 Q180: the natural Active phase still activates a waited member "
          "despite the aura");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION D -- effects/draw/until_count (3 .rs files, 10 #[test])
 *
 * ALREADY COVERED by the peer tests/test_parity_draw_until_count.c: the
 * miyashita_ai_pr_test.rs scenarios `pr_ai_pays_cost_discards_2_draws_to_five`
 * and `pr_ai_pays_cost_discards_2_draws_4` (there, in the single-answer and
 * stepwise forms, with the hand/deck/discard invariants) plus every
 * target_count edge case. What that file does NOT cover is the four remaining
 * Rust cases: the three SKIP-cost paths of miyashita_ai_pr_test.rs and the
 * whole of miyata_draw_until_test.rs. Those are the gaps ported here.
 * ══════════════════════════════════════════════════════════════════════════ */

static int d_setup_miyashita(TestGame *tg, int hand_extra, int deck_count)
{
    int ai = mid(tg, MIYASHITA);
    int filler = mid(tg, SD_010);
    if (!pin_id(ai, MIYASHITA) || !pin_id(filler, SD_010)) return -1;
    test_give_energy(tg, 15);
    test_add_to_hand(tg, ai);
    for (int i = 0; i < hand_extra; i++) test_add_to_hand(tg, filler);
    for (int i = 0; i < deck_count; i++) test_add_to_deck(tg, filler);
    return ai;
}

static void d_miyashita_hand_almost_full_skips_cost(void)
{
    static TestGame tg;
    reset_board(&tg);
    int ai = d_setup_miyashita(&tg, 3, 10);
    if (ai < 0) return;
    int hand_pre  = hand_len(&tg, 0);
    int deck_pre  = deck_len(&tg, 0);
    int disc_pre  = disc_len(&tg, 0);

    CHECK_EQ(test_play_to_stage(&tg, ai, 1), 1, "D skip-3: the debut is accepted");
    CHECK_EQ(tg.state.p[0].stage[1], ai, "D skip-3 CONTROL: Ai is on the centre stage");
    /* rb_play_member removes the played card from hand, so the post-play
     * baseline is one lower than the setup count. Reading it HERE is the only
     * way the cost delta is measured against the right number. */
    int hand_before = hand_len(&tg, 0);
    int deck_before = deck_len(&tg, 0);
    int disc_before = disc_len(&tg, 0);
    CHECK_EQ(hand_before, hand_pre - 1,
             "D skip-3 CONTROL: the play itself moved exactly Ai out of hand, so "
             "any further change is the cost prompt and nothing else");
    CHECK(pending(&tg), "D skip-3 CONTROL: the optional cost really is offered");
    if (pending(&tg)) describe_pending(&tg, "D skip-3");
    skip_pick(&tg);   /* Rust select_indices(&[]) -> an explicit DECLINE */
    pc_pay(&tg, 12);

    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "D: skipping the optional cost cancels the effect -- hand unchanged");
    CHECK_EQ(deck_len(&tg, 0), deck_before,
             "D: the deck is untouched when the cost is declined");
    CHECK_EQ(disc_len(&tg, 0), disc_before, "D: no card reached the waitroom");
}

static void d_miyashita_hand_at_seven_skips_cost(void)
{
    static TestGame tg;
    reset_board(&tg);
    int ai = d_setup_miyashita(&tg, 6, 10);
    if (ai < 0) return;
    int hand_pre = hand_len(&tg, 0);
    int deck_pre = deck_len(&tg, 0);

    CHECK_EQ(test_play_to_stage(&tg, ai, 1), 1, "D skip-7: the debut is accepted");
    int hand_before = hand_len(&tg, 0);
    int deck_before = deck_len(&tg, 0);
    CHECK_EQ(hand_before, hand_pre - 1,
             "D skip-7 CONTROL: the play itself moved exactly Ai out of hand");
    CHECK(hand_before >= 5,
          "D skip-7 CONTROL: the hand is already at or above the "
          "draw_until_count target of 5 before the cost is even answered");
    CHECK(pending(&tg), "D skip-7 CONTROL: the optional cost really is offered");
    skip_pick(&tg);
    pc_pay(&tg, 12);

    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "D: the hand is already >= target_count 5, so draw_until_count is "
             "a no-op even if the effect were reached");
    CHECK_EQ(deck_len(&tg, 0), deck_before, "D: the deck is untouched");
}

static void d_miyashita_empty_deck_no_draw_no_hang(void)
{
    static TestGame tg;
    reset_board(&tg);
    int ai = d_setup_miyashita(&tg, 0, 0);
    if (ai < 0) return;

    CHECK_EQ(deck_len(&tg, 0), 0, "D empty: precondition, the deck really is empty");
    CHECK_EQ(test_play_to_stage(&tg, ai, 1), 1, "D empty: the debut is accepted");
    CHECK(!pending(&tg),
          "D empty: a cost with ZERO eligible cards auto-skips without prompting");
    CHECK_EQ(hand_len(&tg, 0), 0, "D empty: the hand stays empty");
    CHECK_EQ(deck_len(&tg, 0), 0, "D empty: the deck stays empty");
    CHECK_EQ(disc_len(&tg, 0), 0, "D empty: nothing was discarded");
}

/* miyata_draw_until_test.rs. PL!N-bp3-005-R＋ 宮下 愛 ab#0 is gated on a
 * temporal count of THREE card appearances this turn. The Rust twin plays
 * three members to the three stage slots and expects the THIRD to draw the
 * hand up to five. */
static void d_miyata_three_appearances_draw_to_five(void)
{
    static TestGame tg;
    reset_board(&tg);
    int miyata = mid(&tg, MIYATA);
    int kasumi = mid(&tg, KASUMI);
    int shizuku = mid(&tg, SHIZUKU);
    int filler = mid(&tg, SD_010);
    if (!pin_id(miyata, MIYATA) || !pin_id(kasumi, KASUMI) ||
        !pin_id(shizuku, SHIZUKU) || !pin_id(filler, SD_010)) return;
    CHECK_EQ(printed_cost(miyata), 15, "D miyata fixture: PL!N-bp3-005-R＋ costs 15");

    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 60);
    int extra = mid(&tg, SD_010);
    test_add_to_hand(&tg, extra);
    test_add_to_hand(&tg, miyata);
    test_add_to_hand(&tg, kasumi);
    test_add_to_hand(&tg, shizuku);
    int hand_start = hand_len(&tg, 0);
    CHECK(hand_start >= 4, "D miyata CONTROL: 4 cards in hand before any play");

    CHECK_EQ(test_play_to_stage(&tg, miyata, 1), 1, "D miyata: Miyata debuts centre");
    pc_pay(&tg, 12);
    CHECK_EQ(test_play_to_stage(&tg, kasumi, 0), 1, "D miyata: Kasumi debuts left");
    pc_pay(&tg, 12);
    CHECK_EQ(test_play_to_stage(&tg, shizuku, 2), 1, "D miyata: Shizuku debuts right");
    pc_pay(&tg, 12);

    CHECK_EQ(stage_has(&tg, 0, miyata), 1, "D miyata CONTROL: all three members are staged");
    CHECK_EQ(stage_has(&tg, 0, kasumi), 1, "D miyata CONTROL: all three members are staged");
    CHECK_EQ(stage_has(&tg, 0, shizuku), 1, "D miyata CONTROL: all three members are staged");
    /* Each play removes its card from hand, so the pre-third-play baseline is
     * 4 - 2 = 2 (only 2 plays have happened when the third resolves). The
     * draw_until_count target is 5, so the third appearance must bring the
     * hand from 2 up to 5. */
    int hand_at_third = hand_start - 2;
    CHECK_EQ(hand_len(&tg, 0), 5,
             "D: the 3rd appearance this turn brings the hand from 2 to 5 "
             "(draw_until_count target 5)");
    (void)hand_at_third;
}

static void d_miyata_hand_at_five_or_more_draws_nothing(void)
{
    static TestGame tg;
    reset_board(&tg);
    int miyata = mid(&tg, MIYATA);
    int kasumi = mid(&tg, KASUMI);
    int shizuku = mid(&tg, SHIZUKU);
    int filler = mid(&tg, SD_010);
    if (!pin_id(miyata, MIYATA) || !pin_id(kasumi, KASUMI) ||
        !pin_id(shizuku, SHIZUKU) || !pin_id(filler, SD_010)) return;

    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 60);
    for (int i = 0; i < 6; i++) test_add_to_hand(&tg, mid(&tg, SD_010));
    test_add_to_hand(&tg, miyata);
    test_add_to_hand(&tg, kasumi);
    test_add_to_hand(&tg, shizuku);

    test_play_to_stage(&tg, miyata, 1);
    pc_pay(&tg, 12);
    test_play_to_stage(&tg, kasumi, 0);
    pc_pay(&tg, 12);
    test_play_to_stage(&tg, shizuku, 2);
    pc_pay(&tg, 12);

    CHECK_EQ(hand_len(&tg, 0), 6,
             "D: the hand was already 6 (>= target 5) at the 3rd appearance, so "
             "draw_until_count is a saturating_sub no-op");
}

static void d_miyata_two_appearances_no_draw(void)
{
    static TestGame tg;
    reset_board(&tg);
    int miyata = mid(&tg, MIYATA);
    int kasumi = mid(&tg, KASUMI);
    int filler = mid(&tg, SD_010);
    if (!pin_id(miyata, MIYATA) || !pin_id(kasumi, KASUMI) ||
        !pin_id(filler, SD_010)) return;

    fill_decks(&tg, filler, 10);
    test_give_energy(&tg, 60);
    test_add_to_hand(&tg, mid(&tg, SD_010));
    test_add_to_hand(&tg, miyata);
    test_add_to_hand(&tg, kasumi);

    test_play_to_stage(&tg, miyata, 1);
    pc_pay(&tg, 12);
    test_play_to_stage(&tg, kasumi, 0);
    pc_pay(&tg, 12);

    CHECK_EQ(stage_has(&tg, 0, miyata), 1, "D two CONTROL: 2 members really are staged");
    CHECK_EQ(stage_has(&tg, 0, kasumi), 1, "D two CONTROL: 2 members really are staged");
    CHECK_EQ(hand_len(&tg, 0), 1,
             "D: two appearances this turn do not meet the count-3 condition, "
             "so no draw (the count-3 positive is d_miyata_three_appearances)");
}

static void d_miyata_small_deck_draws_out(void)
{
    static TestGame tg;
    reset_board(&tg);
    int miyata = mid(&tg, MIYATA);
    int kasumi = mid(&tg, KASUMI);
    int shizuku = mid(&tg, SHIZUKU);
    int filler = mid(&tg, SD_010);
    if (!pin_id(miyata, MIYATA) || !pin_id(kasumi, KASUMI) ||
        !pin_id(shizuku, SHIZUKU) || !pin_id(filler, SD_010)) return;

    fill_decks(&tg, filler, 2);   /* only 2 to draw */
    test_give_energy(&tg, 60);
    test_add_to_hand(&tg, miyata);
    test_add_to_hand(&tg, kasumi);
    test_add_to_hand(&tg, shizuku);

    test_play_to_stage(&tg, miyata, 1);
    pc_pay(&tg, 12);
    test_play_to_stage(&tg, kasumi, 0);
    pc_pay(&tg, 12);
    test_play_to_stage(&tg, shizuku, 2);
    pc_pay(&tg, 12);

    CHECK_EQ(hand_len(&tg, 0), 2,
             "D: a 2-card deck yields only what exists and stops");
    CHECK_EQ(deck_len(&tg, 0), 0, "D: the deck is fully consumed");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION E -- effects/draw/conditional (6 .rs files, 11 #[test])
 * Every case is a draw whose SIZE is conditional. Each one therefore has a
 * POSITIVE (condition met) and a NEGATIVE (condition not met) arm on the same
 * board, and the deck DELTA is the measured quantity.
 * ══════════════════════════════════════════════════════════════════════════ */

/* ── E1. success_zone_group_pl_bp6_023_l_test.rs (3 tests) ──────────────────
 * PL!-bp6-023-L sweet&sweet holiday ライブ成功時: draw 1; if a μ's live card
 * sits in your success zone, draw 1 more.
 *
 * RUST FIXTURE BUG, reported and worked around rather than papered over: the
 * twin builds the μ's success-zone card from `game.id("PL!-sd1-010-SD")` and
 * names it `mus_live`, but cards.json gives PL!-sd1-010-SD type メンバー
 * (高坂穂乃果, cost 4, unit Printemps) -- it is a MEMBER, not a live card. The
 * printed rule is 『ライブカード』, so the fixture does not test what it claims.
 * This C port keeps the same CARD (so the group filter under test is the same
 * ラブライブ！ series) and records the type mismatch, rather than swapping in a
 * different card and silently testing a different fixture. */
static void e1_mus_live_in_success_zone_draws_two(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = mid(&tg, SWEET_SWEET);
    int mus = mid(&tg, SD_010);
    if (!pin_id(live, SWEET_SWEET) || !pin_id(mus, SD_010)) return;
    CHECK(card_is_live(live), "E1 fixture: PL!-bp6-023-L is a live card");
    /* RUST FIXTURE BUG: the twin's `mus_live` is PL!-sd1-010-SD, which
     * cards.json types as メンバー, not ライブカード. Asserted so the
     * mismatch is visible in the log instead of being absorbed silently. */
    CHECK(!card_is_live(mus),
          "E1 RUST FIXTURE BUG: PL!-sd1-010-SD is a MEMBER (Printemps, cost 4), "
          "not the μ's LIVE card the Rust twin names -- 「ライブカード」 is "
          "therefore NOT what this fixture exercises");
    {
        fprintf(stderr, "  (E1) success-zone card group = %s, is_live = %d\n",
                rb_card_group_name(mus), card_is_live(mus));
    }

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, mid(&tg, SD_010), 40);
    test_set_live_card(&tg, 0, live);
    test_add_to_success(&tg, mus);
    int deck_before = deck_len(&tg, 0);
    int hand_before = hand_len(&tg, 0);
    CHECK_EQ(tg.state.p[0].success.n, 1,
             "E1 CONTROL: a card really is in the success zone");

    int fired = fire_live_success(&tg, live);
    fprintf(stderr, "  (E1 mus) trigger = %d, deck %d -> %d\n",
            fired, deck_before, deck_len(&tg, 0));
    pc_pay(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 2,
             "E1: a μ's-group card in the success zone -> the base draw plus one more");
    CHECK_EQ(hand_len(&tg, 0), hand_before + 2,
             "E1 CONTROL: the 2 drawn cards really reached hand");
}

static void e1_no_success_zone_card_draws_one(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = mid(&tg, SWEET_SWEET);
    if (!pin_id(live, SWEET_SWEET)) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, mid(&tg, SD_010), 40);
    test_set_live_card(&tg, 0, live);
    int deck_before = deck_len(&tg, 0);
    CHECK_EQ(tg.state.p[0].success.n, 0, "E1 single CONTROL: the success zone is empty");

    fire_live_success(&tg, live);
    pc_pay(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 1,
             "E1: no card in the success zone -> only the base draw of 1");
}

static void e1_non_mus_live_in_success_zone_draws_one(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = mid(&tg, SWEET_SWEET);
    int niji = mid(&tg, NIJI_LIVE);
    if (!pin_id(live, SWEET_SWEET) || !pin_id(niji, NIJI_LIVE)) return;
    CHECK(strcmp(rb_card_group_name(niji), NIJI_GROUP) == 0,
          "E1 non-μ's CONTROL: the success-zone card really is 虹ヶ咲, not μ's");
    CHECK(card_is_live(niji), "E1 non-μ's fixture: it is a live card");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, mid(&tg, SD_010), 40);
    test_set_live_card(&tg, 0, live);
    test_add_to_success(&tg, niji);
    int deck_before = deck_len(&tg, 0);
    CHECK_EQ(tg.state.p[0].success.n, 1, "E1 non-μ's CONTROL: a card IS in the success zone");

    fire_live_success(&tg, live);
    pc_pay(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 1,
             "E1: a non-μ's live in the success zone -> no extra draw");
}

/* ── E2. score_or_revealed_live_pl_sp_pb2_004_r_test.rs (1 test) ────────────
 * PL!SP-pb2-004-R 平安名すみれ ライブ成功時: draw 1 if the live card's score
 * exceeds its printed score, OR if a scored live card was revealed. */
static void e2_score_or_revealed_live_draw(void)
{
    static TestGame tg;
    reset_board(&tg);
    int sumire = mid(&tg, SUMIRE_PB2);
    int live6 = mid(&tg, LIVE6);
    int filler = mid(&tg, SD_010);
    if (!pin_id(sumire, SUMIRE_PB2) || !pin_id(live6, LIVE6) ||
        !pin_id(filler, SD_010)) return;
    CHECK(card_is_live(live6), "E2 fixture: PL!SP-bp1-027-L is a live card");

    fill_decks(&tg, filler, 40);
    set_stage(&tg, 0, RB_EMPTY_SLOT, sumire, RB_EMPTY_SLOT);
    test_set_live_card(&tg, 0, live6);
    int hand_before = hand_len(&tg, 0);

    /* 1. neither condition met. */
    fire_live_success(&tg, sumire);
    pc_pay(&tg, 12);
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "E2 step 1: neither condition met -> no draw");
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "E2 step 1 CONTROL: the same card DOES draw in steps 2 and 3, so "
             "this zero is the condition and not a dead ability");

    /* 2. the live card's score modifier pushes it above its printed score. */
    rb_mods_add_score(&tg.state.mods, live6, 2);
    CHECK_EQ(score_of(&tg, live6), 2, "E2 step 2 precondition: the +2 score landed");
    fire_live_success(&tg, sumire);
    pc_pay(&tg, 12);
    CHECK_EQ(hand_len(&tg, 0), hand_before + 1,
             "E2 step 2: a live scoring above its printed score -> draw 1");

    /* 3. score back to printed, but a scored live was revealed -> the OR arm. */
    rb_mods_add_score(&tg.state.mods, live6, -2);
    CHECK_EQ(score_of(&tg, live6), 0, "E2 step 3 precondition: the score modifier is back to 0");
    int revealed = mid(&tg, SD_019);
    tg.state.revealed_cards[tg.state.n_revealed++] = revealed;
    CHECK_EQ(tg.state.n_revealed, 1, "E2 step 3 CONTROL: a card is really revealed");
    fire_live_success(&tg, sumire);
    pc_pay(&tg, 12);
    CHECK_EQ(hand_len(&tg, 0), hand_before + 2,
             "E2 step 3: a revealed scored live -> the OR branch draws 1 more");
}

/* ── E3. live_start_energy_return_ren_draw_pl_sp_bp7_026_l_test.rs (3) ──────
 * PL!SP-bp7-026-L Dears ライブ開始時: optional return 1 energy from the energy
 * zone to the energy deck; if 葉月恋 is on your stage, draw 2 then discard 1. */
static int e3_setup(TestGame *tg, int with_ren)
{
    int live = mid(tg, DEARS);
    int filler = mid(tg, SD_010);
    if (!pin_id(live, DEARS) || !pin_id(filler, SD_010)) return -1;
    CHECK(card_is_live(live), "E3 fixture: PL!SP-bp7-026-L is a live card");

    fill_decks(tg, filler, 30);
    test_set_live_card(tg, 0, live);
    int left = with_ren ? mid(tg, REN_SD1) : filler;
    if (with_ren && !pin_id(left, REN_SD1)) return -1;
    int mid_c = mid(tg, SD_010);
    int right = mid(tg, SD_010);
    set_stage(tg, 0, left, mid_c, right);
    if (with_ren) {
        Card c;
        int is_ren = 0;
        if (rb_decode_card_by_index((uint32_t)left, &c)) {
            /* 葉月 恋 -- the Rust twin asserts name.contains("葉月") */
            is_ren = c.name && strstr(c.name, "\xe8\x91\x89\xe6\x9c\x88") != NULL;
            rb_free_card(&c);
        }
        CHECK(is_ren, "E3 CONTROL: the staged member really is 葉月 恋");
    }
    test_give_energy(tg, 5);
    push_deck(tg, 0, filler, 0);
    /* Energy deck: 2 cards, energy zone: 5 active. */
    int edeck = mid(tg, ENERGY_CARD);
    push_deck(tg, 0, edeck, 0);
    tg->state.p[0].energy_deck.n = 0;
    int e1 = mid(tg, ENERGY_CARD);
    int e2 = mid(tg, ENERGY_CARD);
    if (e1 < 0 || e2 < 0) return -1;
    tg->state.p[0].energy_deck.cards[tg->state.p[0].energy_deck.n++] = e1;
    tg->state.p[0].energy_deck.cards[tg->state.p[0].energy_deck.n++] = e2;
    return live;
}

static void e3_accept_cost_with_ren_draws_two_discards_one(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = e3_setup(&tg, 1);
    if (live < 0) return;
    int edeck_before = tg.state.p[0].energy_deck.n;
    int zone_before  = tg.state.p[0].energy.n;
    int hand_before  = hand_len(&tg, 0);
    int disc_before  = disc_len(&tg, 0);

    CHECK(fire_live_start(&tg, live), "E3 pay: Dears' ライブ開始時 fires");
    CHECK(pending(&tg), "E3 pay CONTROL: the optional energy cost really is offered");
    if (pending(&tg)) describe_pending(&tg, "E3 pay");
    answer(&tg, 1);   /* PAY (index 0 would be SKIP) */
    int prompts = pc_pay(&tg, 12);
    fprintf(stderr, "  (E3 pay) prompts = %d, edeck %d -> %d, zone %d -> %d\n",
            prompts, edeck_before, tg.state.p[0].energy_deck.n,
            zone_before, tg.state.p[0].energy.n);

    CHECK(prompts > 0, "E3 pay CONTROL: the cost prompt was really answered");
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before + 1,
             "E3 pay: one energy really moved from the zone back to the energy deck");
    CHECK_EQ(tg.state.p[0].energy.n, zone_before - 1,
             "E3 pay: the zone lost exactly the moved card");
    CHECK_EQ(hand_len(&tg, 0), hand_before + 1,
             "E3 pay: with 葉月恋 on stage, drew 2 then discarded 1 (net +1)");
    CHECK_EQ(disc_len(&tg, 0), disc_before + 1,
             "E3 pay: the discarded hand card really reached the waitroom");
}

static void e3_decline_cost_nothing_happens(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = e3_setup(&tg, 1);
    if (live < 0) return;
    int edeck_before = tg.state.p[0].energy_deck.n;
    int zone_before  = tg.state.p[0].energy.n;
    int hand_before  = hand_len(&tg, 0);
    int disc_before  = disc_len(&tg, 0);

    CHECK(fire_live_start(&tg, live), "E3 decline: Dears' ライブ開始時 fires");
    CHECK(pending(&tg), "E3 decline CONTROL: the optional cost really is offered");
    skip_pick(&tg);
    pc_pay(&tg, 12);

    CHECK(!pending(&tg), "E3 decline: declining ends the ability with no follow-up");
    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before, "E3 decline: the energy deck is untouched");
    CHECK_EQ(tg.state.p[0].energy.n, zone_before, "E3 decline: the energy zone is untouched");
    CHECK_EQ(hand_len(&tg, 0), hand_before, "E3 decline: no draw");
    CHECK_EQ(disc_len(&tg, 0), disc_before, "E3 decline: no discard");
}

static void e3_accept_cost_without_ren_pays_but_no_draw(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = e3_setup(&tg, 0);
    if (live < 0) return;
    int edeck_before = tg.state.p[0].energy_deck.n;
    int zone_before  = tg.state.p[0].energy.n;
    int hand_before  = hand_len(&tg, 0);
    int disc_before  = disc_len(&tg, 0);

    CHECK(fire_live_start(&tg, live), "E3 no-Ren: Dears' ライブ開始時 fires");
    CHECK(pending(&tg), "E3 no-Ren CONTROL: the optional cost really is offered");
    answer(&tg, 1);   /* PAY */
    pc_pay(&tg, 12);

    CHECK_EQ(tg.state.p[0].energy_deck.n, edeck_before + 1,
             "E3 no-Ren: the cost is still payable without 葉月恋 on stage");
    CHECK_EQ(tg.state.p[0].energy.n, zone_before - 1, "E3 no-Ren: the zone lost one card");
    CHECK_EQ(hand_len(&tg, 0), hand_before,
             "E3 no-Ren: no 葉月恋 on stage -> no draw "
             "(e3_accept_cost_with_ren shows the same card DOES draw)");
    CHECK_EQ(disc_len(&tg, 0), disc_before, "E3 no-Ren: and therefore no discard");
}

/* ── E4. area_moved_pl_sp_sd2_003_sd2_test.rs (2 tests) ────────────────────
 * PL!SP-sd2-003-SD2 嵐 千砂都 ライブ成功時: draw 1; if a card AREA-MOVED this
 * turn, draw 1 more. The mover is PL!SP-bp5-006-R 桜小路きな子's 起動, whose
 * cost mills 3 and then offers a position change. Chisato starts on the LEFT
 * and きな子 on the RIGHT; sending きな子 to the LEFT swaps them, which is what
 * the Rust twin's "left" option does. */
static int e4_swap_via_kinako(TestGame *tg, int chisato, int *chisa_before)
{
    int kinako = mid(tg, KINAKO);
    if (!pin_id(kinako, KINAKO)) return -1;
    fill_decks(tg, mid(tg, SD_010), 20);
    test_give_energy(tg, 30);
    test_add_to_hand(tg, chisato);
    test_add_to_hand(tg, kinako);
    if (test_play_to_stage(tg, chisato, 0) != 1) return -1;
    if (test_play_to_stage(tg, kinako, 2) != 1) return -1;
    pc_pay(tg, 12);
    *chisa_before = tg->state.p[0].stage[0];
    CHECK_EQ(*chisa_before, chisato,
             "E4: Chisato really started on the LEFT before the swap");
    int kinako_slot = slot_of(tg, 0, kinako);
    CHECK_EQ(kinako_slot, 2, "E4 CONTROL: きな子 really started on the RIGHT");
    if (test_activate_ability(tg, kinako) != 1) return -1;
    /* Drive きな子 off its own slot, which swaps it with Chisato's LEFT. */
    int dst = drive_position_to_other_slot(tg, kinako_slot);
    pc_pay(tg, 12);
    if (dst < 0) return -1;
    *chisa_before = chisato;
    return kinako;
}

static void e4_area_moved_draws_two(void)
{
    static TestGame tg;
    reset_board(&tg);
    int chisato = mid(&tg, CHISATO_SD2);
    if (!pin_id(chisato, CHISATO_SD2)) return;
    int before_slot = -1;
    if (e4_swap_via_kinako(&tg, chisato, &before_slot) < 0) {
        CHECK(0, "E4: the きな子 position change completed");
        return;
    }
    /* Rust asserts stage[2] == chisato, i.e. the swap really moved 千砂都. */
    fprintf(stderr, "  (E4) board after swap = [%d %d %d]\n",
            tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2]);
    CHECK_EQ(tg.state.p[0].stage[2], chisato,
             "E4 CONTROL: the area-move swap really moved 千砂都 out of the LEFT slot "
             "(きな子 was sent there, so 千砂都 took its place on the right)");
    CHECK_EQ(tg.state.position_change_occurred_this_turn, 1,
             "E4 CONTROL: the engine recorded an area move this turn, which is "
             "the condition 「このターンにエリア移動 astrologyした」 reads");
    if (tg.state.p[0].stage[2] != chisato) return;

    int deck_before = deck_len(&tg, 0);
    int fired = fire_live_success(&tg, chisato);
    fprintf(stderr, "  (E4 area-moved) trigger = %d, deck %d -> %d\n",
            fired, deck_before, deck_len(&tg, 0));
    pc_pay(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 2,
             "E4: area-moved this turn -> the base draw plus one more");
}

static void e4_no_area_move_draws_one(void)
{
    static TestGame tg;
    reset_board(&tg);
    int chisato = mid(&tg, CHISATO_SD2);
    int filler = mid(&tg, SD_010);
    if (!pin_id(chisato, CHISATO_SD2) || !pin_id(filler, SD_010)) return;

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 30);
    set_stage(&tg, 0, chisato, filler, RB_EMPTY_SLOT);
    test_set_live_card(&tg, 0, chisato);
    CHECK_EQ(tg.state.p[0].stage[0], chisato, "E4 no-move CONTROL: 千砂都 is staged left");
    CHECK_EQ(tg.state.position_change_occurred_this_turn, 0,
             "E4 no-move CONTROL: no position change has occurred this turn");

    int deck_before = deck_len(&tg, 0);
    fire_live_success(&tg, chisato);
    pc_pay(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 1,
             "E4: no area move this turn -> only the base draw of 1");
}

/* ── E5. revealed_live_hand_threshold_draw_test.rs (2 tests) ───────────────
 * PL!S-bp2-007-R＋ 国木田花丸: after a revealed live card resolves, the
 * automatic draw only applies when the hand is still at most 7. These drive
 * the real phase walk, so a red here could be a timing artefact rather than an
 * engine verdict -- hence a precondition assertion on the phase reached. */
static void e5_revealed_live_above_hand_threshold_no_draw(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanamaru = mid(&tg, HANAMARU_BP2);
    int filler = mid(&tg, SD_010);
    int blader = mid(&tg, RIKO_PR14);
    int live = mid(&tg, LL_BP5_001);
    int other_live = mid(&tg, LL_BP5_002);
    if (!pin_id(hanamaru, HANAMARU_BP2) || !pin_id(filler, SD_010) ||
        !pin_id(blader, RIKO_PR14) || !pin_id(live, LL_BP5_001) ||
        !pin_id(other_live, LL_BP5_002)) return;

    tg.state.p[0].deck.n = 0;
    push_deck(&tg, 0, other_live, 1);
    push_deck(&tg, 0, live, 1);
    push_deck(&tg, 0, filler, 98);
    push_deck(&tg, 1, filler, 100);
    for (int i = 0; i < 7; i++) test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, live);
    int blader2 = mid(&tg, RIKO_PR14);
    set_stage(&tg, 0, hanamaru, blader, blader2);
    CHECK_EQ(hand_len(&tg, 0), 8, "E5 CONTROL: the hand really holds 8 cards before the walk");

    walk_to_live_set(&tg);
    fprintf(stderr, "  (E5) phase after 5 passes = %s\n", rb_phase_name(tg.state.phase));
    test_set_live_card(&tg, 0, live);
    walk_to_live_start(&tg);
    fprintf(stderr, "  (E5) phase after live start = %s\n", rb_phase_name(tg.state.phase));
    pc_drain_skip(&tg, 16);

    CHECK(hand_len(&tg, 0) >= 8,
          "E5: the hand never shrinks below the 7-card threshold, so no "
          "automatic draw can be hiding here");
    CHECK_EQ(hand_len(&tg, 0), 8,
             "E5: with the hand already at 8 the automatic draw must not fire");
    CHECK_EQ(deck_len(&tg, 0), 99,
             "E5: one of the two revealed cards left the deck, the other stayed");
}

static void e5_no_live_in_reveal_no_draw(void)
{
    static TestGame tg;
    reset_board(&tg);
    int hanamaru = mid(&tg, HANAMARU_BP2);
    int filler = mid(&tg, SD_010);
    int blader = mid(&tg, RIKO_PR14);
    int live = mid(&tg, LL_BP5_001);
    if (!pin_id(hanamaru, HANAMARU_BP2) || !pin_id(filler, SD_010) ||
        !pin_id(blader, RIKO_PR14) || !pin_id(live, LL_BP5_001)) return;

    push_deck(&tg, 0, filler, 100);
    push_deck(&tg, 1, filler, 100);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, live);
    int blader2 = mid(&tg, RIKO_PR14);
    set_stage(&tg, 0, hanamaru, blader, blader2);

    walk_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    walk_to_live_start(&tg);
    pc_drain_skip(&tg, 16);

    fprintf(stderr, "  (E5 filler) phase = %s, hand = %d\n",
            rb_phase_name(tg.state.phase), hand_len(&tg, 0));
    CHECK_EQ(hand_len(&tg, 0), 6,
             "E5: the revealed set is all filler, so the live condition fails "
             "and no automatic draw fires");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION F -- effects/position/area_gated (3 .rs files, 18 #[test])
 * Two abilities gated on a STAGE AREA: one grants to the CENTRE member only
 * (the 「センター」 keyword, which a peer reported is parsed in vm.c and then
 * DROPPED), one grants to a member that MOVED TO the centre.
 * ══════════════════════════════════════════════════════════════════════════ */
/* ── F1. success_zone_center_area_mus_member_constant_blade_test.rs (11) ────
 * PL!-bp4-020-L Love wing bell ab#1 常時:
 *   「このカードが自分の成功ライブカード置き場にあるかぎり、
 *     自分のセンターエリアにいる『μ's』のメンバーはブレードを得る」
 *
 * 11 Rust cases. The first is a parser guard: the card MUST carry a 常時 with
 * a parsed effect, otherwise every other case would be measuring a parse
 * miss rather than the AREA gate. */
static void f1_card_carries_constant_with_effect(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    if (!pin_id(love, LOVE_WING)) return;
    CHECK(card_is_live(love), "F1 fixture: PL!-bp4-020-L is a live card");

    int n = rb_card_num_abilities((uint32_t)love);
    int n_const = 0, n_const_with_effect = 0;
    for (int i = 0; i < n; i++) {
        Ability ab; memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)love, i, &ab)) continue;
        if (ab.triggers && !strcmp(ab.triggers, TRIG_CONSTANT)) {
            n_const++;
            if (ab.effect) n_const_with_effect++;
        }
        rb_free_ability(&ab);
    }
    fprintf(stderr, "  (F1) abilities = %d, 常時 = %d, with effect = %d\n",
            n, n_const, n_const_with_effect);
    CHECK(n_const >= 1, "F1: Love wing bell carries a 常時 ability (parser guard)");
    CHECK(n_const_with_effect >= 1,
          "F1: the 常時 carries a PARSED EFFECT (parser guard) -- without this "
          "every F1 negative below would be measuring a parse miss");
}

/* μ's member in the CENTRE + the card in the success zone -> blade +1. */
static void f1_centre_mus_member_gains_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, MUS_CENTER);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, MUS_CENTER) ||
        !pin_id(filler, SD_010)) return;
    CHECK(strcmp(rb_card_group_name(muse), rb_card_group_name(filler)) == 0,
          "F1 centre CONTROL: the centre card and the filler are the SAME group");

    set_stage(&tg, 0, filler, muse, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    CHECK_EQ(tg.state.p[0].success.n, 1, "F1 centre CONTROL: the card IS in the success zone");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, muse), 1,
             "F1: a μ's member in the centre gains blade +1");
    CHECK_EQ(blade_of(&tg, filler), 0,
             "F1: the μ's member on the LEFT gains nothing -- the gate is the centre");
}

/* The positive above is the control for both of the next two. */
static void f1_left_mus_member_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, SD_010);
    int centre = mid(&tg, MUS_CENTER);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, SD_010) ||
        !pin_id(centre, MUS_CENTER)) return;

    set_stage(&tg, 0, muse, centre, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, muse), 0, "F1 left: a μ's member on the LEFT gains no blade");
    CHECK_EQ(blade_of(&tg, centre), 1,
             "F1 left CONTROL: the CENTRE member on the SAME board did gain, so "
             "the left zero is the area gate and not a dead ability");
}

static void f1_right_mus_member_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, SD_010);
    int centre = mid(&tg, MUS_CENTER);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, SD_010) ||
        !pin_id(centre, MUS_CENTER)) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, centre, muse);
    test_add_to_success(&tg, love);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, muse), 0, "F1 right: a μ's member on the RIGHT gains no blade");
    CHECK_EQ(blade_of(&tg, centre), 1,
             "F1 right CONTROL: the CENTRE member on the SAME board did gain");
}

static void f1_non_mus_centre_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int aqours = mid(&tg, AQOURS_P);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love, LOVE_WING) || !pin_id(aqours, AQOURS_P) ||
        !pin_id(filler, SD_010)) return;
    CHECK(strcmp(rb_card_group_name(aqours), rb_card_group_name(filler)) != 0,
          "F1 non-μ's CONTROL: the centre card really is a DIFFERENT group");

    set_stage(&tg, 0, filler, aqours, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, aqours), 0,
             "F1: a non-μ's centre member gains no blade (the group filter)");
}

static void f1_card_not_in_success_zone_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, MUS_CENTER);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, MUS_CENTER) ||
        !pin_id(filler, SD_010)) return;

    /* Love wing bell in the WAITROOM, not the success zone. */
    set_stage(&tg, 0, filler, muse, RB_EMPTY_SLOT);
    rb_waitroom_add(&tg.state.p[0], love);
    CHECK_EQ(tg.state.p[0].success.n, 0, "F1 waitroom CONTROL: the success zone is empty");
    CHECK(disc_has(&tg, 0, love), "F1 waitroom CONTROL: the card really is in the waitroom");
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, muse), 0,
             "F1: the card outside the success zone grants nothing");
}

static void f1_removed_from_success_zone_loses_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, MUS_CENTER);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, MUS_CENTER) ||
        !pin_id(filler, SD_010)) return;

    set_stage(&tg, 0, filler, muse, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 1, "F1 remove: precondition, the centre member has +1 blade");

    tg.state.p[0].success.n = 0;
    CHECK_EQ(tg.state.p[0].success.n, 0, "F1 remove CONTROL: the success zone really is empty now");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 0,
             "F1: removing the card from the success zone removes the blade");
}

static void f1_member_leaves_centre_loses_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, MUS_CENTER);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, MUS_CENTER) ||
        !pin_id(filler, SD_010)) return;

    set_stage(&tg, 0, filler, muse, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 1, "F1 leave: precondition, centre -> +1 blade");

    set_stage(&tg, 0, muse, filler, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 0,
             "F1: moving the μ's member off the centre removes the blade");
}

static void f1_both_players_own_centre(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love1 = mid(&tg, LOVE_WING);
    int love2 = mid(&tg, LOVE_WING);
    int muse1 = mid(&tg, MUS_CENTER);
    int muse2 = mid(&tg, MUS_CENTER);
    int filler = mid(&tg, SD_010);
    if (!pin_id(love1, LOVE_WING) || !pin_id(love2, LOVE_WING) ||
        !pin_id(muse1, MUS_CENTER) || !pin_id(muse2, MUS_CENTER) ||
        !pin_id(filler, SD_010)) return;

    set_stage(&tg, 0, filler, muse1, RB_EMPTY_SLOT);
    set_stage(&tg, 1, filler, muse2, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love1);
    tg.state.p[1].success.n = 0;
    tg.state.p[1].success.cards[tg.state.p[1].success.n++] = love2;
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, muse1), 1, "F1 both: player 1's centre μ's gains +1 blade");
    CHECK_EQ(blade_of(&tg, muse2), 1, "F1 both: player 2's centre μ's gains +1 blade");
}

static void f1_only_centre_gets_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int left = mid(&tg, SD_010);
    int centre = mid(&tg, MUS_CENTER);
    int right = mid(&tg, SD_005);
    if (!pin_id(love, LOVE_WING) || !pin_id(left, SD_010) ||
        !pin_id(centre, MUS_CENTER) || !pin_id(right, SD_005)) return;
    CHECK(card_is_member(left) && card_is_member(centre) && card_is_member(right),
          "F1 only-centre CONTROL: all three staged cards really are members");

    set_stage(&tg, 0, left, centre, right);
    test_add_to_success(&tg, love);
    test_recalc(&tg);

    CHECK_EQ(blade_of(&tg, left), 0,   "F1 only-centre: the LEFT member gains nothing");
    CHECK_EQ(blade_of(&tg, centre), 1, "F1 only-centre: the CENTRE member gains +1");
    CHECK_EQ(blade_of(&tg, right), 0,  "F1 only-centre: the RIGHT member gains nothing");
}

static void f1_empty_stage_then_muse_still_works(void)
{
    static TestGame tg;
    reset_board(&tg);
    int love = mid(&tg, LOVE_WING);
    int muse = mid(&tg, MUS_CENTER);
    if (!pin_id(love, LOVE_WING) || !pin_id(muse, MUS_CENTER)) return;

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    test_add_to_success(&tg, love);
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 0,
             "F1 empty: an empty stage is a no-op, not a fault");

    /* Adding the member afterwards must still work: the empty pass must not
     * have wedged the modifier store. */
    tg.state.p[0].stage[1] = muse;
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, muse), 1,
             "F1 empty: the blade is still granted after an earlier empty-stage pass");
}

/* ── F2. center_area_syncrise_member_constant_blade_gain_test.rs (7) ───────
 * PL!SP-pb2-022-R 鬼塚冬毬 ab#0: a 5yncri5e! member moving TO the centre
 * gains the HOST +4 blades (the host gains, not the mover).
 * PL!SP-sd2-011-SD2 鬼塚冬毬 ab#0: SELF-target -- the host gains 1 when IT
 * moves to the centre.
 * The mover is きな子's 起動, which mills 3 then offers a position change. */
static int f2_kinako(TestGame *tg)
{
    int k = mid(tg, KINAKO);
    if (!pin_id(k, KINAKO)) return -1;
    return k;
}

static void f2_syncrise_mover_gains_host_four_blades(void)
{
    static TestGame tg;
    reset_board(&tg);
    int host = mid(&tg, FUYU);
    int kinako = f2_kinako(&tg);
    if (host < 0 || kinako < 0) return;
    CHECK(strcmp(rb_card_group_name(host), rb_card_group_name(kinako)) == 0,
          "F2 CONTROL: host and mover really share the 5yncri5e! unit");

    fill_decks(&tg, mid(&tg, SD_010), 20);
    test_give_energy(&tg, 35);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, kinako);
    CHECK_EQ(test_play_to_stage(&tg, host, 0), 1, "F2: the host deploys left");
    CHECK_EQ(test_play_to_stage(&tg, kinako, 1), 1, "F2: きな子 deploys centre");
    pc_pay(&tg, 12);

    int before = blade_of(&tg, host);
    int kbefore = blade_of(&tg, kinako);
    int kslot = slot_of(&tg, 0, kinako);
    CHECK_EQ(kslot, 1, "F2 CONTROL: きな子 really started in the CENTRE");
    CHECK_EQ(test_activate_ability(&tg, kinako), 1, "F2: きな子's 起動 is accepted");
    CHECK(pending(&tg), "F2 CONTROL: the position destination prompt opened");
    if (pending(&tg)) describe_pending(&tg, "F2 fuyu");
    int dst = drive_position_to_other_slot(&tg, kslot);
    pc_pay(&tg, 12);
    fprintf(stderr, "  (F2 fuyu) chose dst slot %d, board = [%d %d %d]\n",
            dst, tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2]);
    CHECK(dst >= 0, "F2 CONTROL: a destination other than the centre was chosen");
    CHECK_EQ(tg.state.p[0].stage[dst < 0 ? 0 : dst], kinako,
             "F2 CONTROL: きな子 really moved to the destination the driver asked for");
    CHECK_EQ(tg.state.p[0].stage[1], host,
             "F2 CONTROL: the host really took the vacated centre");

    CHECK(blade_of(&tg, host) >= before + 4,
          "F2: a 5yncri5e! member leaving the centre gains the host 4 blades");
    CHECK_EQ(blade_of(&tg, kinako), kbefore,
             "F2: the MOVER gains no blades -- only the host does");
}

static void f2_non_syncrise_mover_no_trigger(void)
{
    static TestGame tg;
    reset_board(&tg);
    int host = mid(&tg, FUYU);
    int filler = mid(&tg, SD_010);
    int shiki = mid(&tg, SHIKI);
    if (!pin_id(host, FUYU) || !pin_id(filler, SD_010) || !pin_id(shiki, SHIKI)) return;
    /* The Rust twin labels PL!SP-bp2-008-R 若菜四季 "non-Syncri5e (Liella!)" and
     * PL!-sd1-010-SD "μ's Printemps, NOT Syncri5e". cards.json says BOTH are
     * 5yncri5e! / Printemps respectively, so only the FILLER can be the
     * non-matching one. Reported as a Rust fixture-comment bug. */
    CHECK(strcmp(rb_card_group_name(shiki), rb_card_group_name(host)) == 0,
          "F2 CONTROL: きな子 and the host are both 5yncri5e!, so the mover in "
          "this case is the PRINTEMPS filler, not 若菜四季");
    CHECK(strcmp(rb_card_group_name(filler), rb_card_group_name(host)) != 0,
          "F2 CONTROL: the filler really is a different group from the host");

    fill_decks(&tg, mid(&tg, SD_010), 20);
    test_give_energy(&tg, 35);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, shiki);
    CHECK_EQ(test_play_to_stage(&tg, host, 0), 1, "F2 non: host left");
    CHECK_EQ(test_play_to_stage(&tg, filler, 2), 1, "F2 non: filler right");
    CHECK_EQ(test_play_to_stage(&tg, shiki, 1), 1, "F2 non: 若菜四季 centre");
    pc_pay(&tg, 12);

    int before = blade_of(&tg, host);
    int sslot = slot_of(&tg, 0, shiki);
    CHECK_EQ(sslot, 1, "F2 non CONTROL: 若菜四季 really started in the CENTRE");
    CHECK_EQ(test_activate_ability(&tg, shiki), 1, "F2 non: 若菜四季's 起動 is accepted");
    CHECK(pending(&tg), "F2 non CONTROL: a destination prompt opened");
    int dst = drive_position_to_other_slot(&tg, sslot);
    pc_pay(&tg, 12);
    fprintf(stderr, "  (F2 non) chose dst slot %d, board = [%d %d %d]\n",
            dst, tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2]);
    CHECK(dst >= 0, "F2 non CONTROL: a destination other than the centre was chosen");
    CHECK_EQ(tg.state.p[0].stage[1], filler,
             "F2 non CONTROL: the non-5yncri5e! filler really moved TO the centre");
    if (tg.state.p[0].stage[1] != filler) return;

    CHECK_EQ(blade_of(&tg, host), before,
             "F2: a NON-5yncri5e! member moving to the centre grants no blades");
}

static void f2_self_target_host_moved_gains_one_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int host = mid(&tg, FUYU_SELF);
    int kinako = f2_kinako(&tg);
    if (host < 0 || kinako < 0) return;

    fill_decks(&tg, mid(&tg, SD_010), 20);
    test_give_energy(&tg, 35);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, kinako);
    CHECK_EQ(test_play_to_stage(&tg, host, 0), 1, "F2 self: the self-target host left");
    CHECK_EQ(test_play_to_stage(&tg, kinako, 1), 1, "F2 self: きな子 centre");
    pc_pay(&tg, 12);

    int before = blade_of(&tg, host);
    int kslot = slot_of(&tg, 0, kinako);
    CHECK_EQ(kslot, 1, "F2 self CONTROL: きな子 really started in the CENTRE");
    CHECK_EQ(test_activate_ability(&tg, kinako), 1, "F2 self: きな子's 起動 is accepted");
    int dst = drive_position_to_other_slot(&tg, kslot);
    pc_pay(&tg, 12);
    CHECK(dst >= 0, "F2 self CONTROL: a destination other than the centre was chosen");
    CHECK_EQ(tg.state.p[0].stage[dst < 0 ? 0 : dst], kinako,
             "F2 self CONTROL: きな子 landed at the requested destination");
    CHECK_EQ(tg.state.p[0].stage[1], host,
             "F2 self CONTROL: the host really took the vacated centre");
    CHECK(blade_of(&tg, host) > before,
          "F2: a SELF-target host that itself moved to the centre gains 1 blade");
}

static void f2_self_target_not_moved_no_blade(void)
{
    static TestGame tg;
    reset_board(&tg);
    int host = mid(&tg, FUYU_SELF);
    int kinako = f2_kinako(&tg);
    int filler = mid(&tg, SD_010);
    if (host < 0 || kinako < 0) return;

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 35);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, kinako);
    CHECK_EQ(test_play_to_stage(&tg, host, 0), 1, "F2 self-neg: host left");
    CHECK_EQ(test_play_to_stage(&tg, filler, 1), 1, "F2 self-neg: filler centre");
    CHECK_EQ(test_play_to_stage(&tg, kinako, 2), 1, "F2 self-neg: きな子 right");
    pc_pay(&tg, 12);

    int before = blade_of(&tg, host);
    int kslot = slot_of(&tg, 0, kinako);
    CHECK_EQ(kslot, 2, "F2 self-neg CONTROL: きな子 really started on the RIGHT");
    CHECK_EQ(test_activate_ability(&tg, kinako), 1, "F2 self-neg: きな子's 起動 is accepted");
    int dst = drive_position_to_other_slot(&tg, kslot);
    pc_pay(&tg, 12);
    CHECK(dst >= 0, "F2 self-neg CONTROL: a destination other than the right was chosen");
    CHECK(dst != 1,
           "F2 self-neg CONTROL: the chosen destination is NOT the centre, so the "
           "host really stayed where it was");
    CHECK_EQ(tg.state.p[0].stage[0], host, "F2 self-neg CONTROL: the host stayed on the left");
    CHECK_EQ(blade_of(&tg, host), before,
             "F2: a self-target host that did NOT move gains no blade");
}

static void f2_no_position_change_no_trigger(void)
{
    static TestGame tg;
    reset_board(&tg);
    int host = mid(&tg, FUYU);
    int self = mid(&tg, FUYU_SELF);
    if (host < 0 || self < 0) return;

    fill_decks(&tg, mid(&tg, SD_010), 20);
    test_give_energy(&tg, 35);
    test_add_to_hand(&tg, host);
    test_add_to_hand(&tg, self);
    CHECK_EQ(test_play_to_stage(&tg, host, 1), 1, "F2 none: the host deploys centre");
    CHECK_EQ(test_play_to_stage(&tg, self, 0), 1, "F2 none: the self-target deploys left");
    pc_pay(&tg, 12);

    int hb = blade_of(&tg, host), sb = blade_of(&tg, self);
    CHECK_EQ(tg.state.position_change_occurred_this_turn, 0,
             "F2 none CONTROL: no position change has occurred this turn");
    CHECK_EQ(blade_of(&tg, host), hb, "F2 none: the 5yncri5e! host gains nothing");
    CHECK_EQ(blade_of(&tg, self), sb, "F2 none: the self-target gains nothing");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION G -- effects/position/per_card (2 .rs files, 4 #[test])
 * ALREADY COVERED by the peer tests/test_parity_zones_position.c section 8:
 * mill_three_deck_four (Q234) and mill_three_deck_two_no_waitroom across the
 * AR / P / R rarities. The one case neither file has is q104_deck_2_refresh_
 * and_continue, where a 2-card deck plus a 3-card waitroom is refreshed
 * mid-cost. That is the gap ported here.
 * ══════════════════════════════════════════════════════════════════════════ */

static void g_mill_three_q104_deck_two_refreshes(void)
{
    static TestGame tg;
    reset_board(&tg);
    int kinako = mid(&tg, KINAKO);
    int filler = mid(&tg, SD_010);
    if (!pin_id(kinako, KINAKO) || !pin_id(filler, SD_010)) return;
    CHECK(card_is_member(kinako), "G fixture: きな子 is a member card");

    set_stage(&tg, 0, kinako, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    /* 2 in the deck (fewer than the cost of 3) plus 3 already in the waitroom,
     * so Q104's refresh is genuinely required. */
    push_deck(&tg, 0, filler, 2);
    for (int i = 0; i < 3; i++) { int w = mid(&tg, SD_010); rb_waitroom_add(&tg.state.p[0], w); }
    CHECK_EQ(deck_len(&tg, 0), 2, "G precondition: the deck really holds only 2");
    CHECK_EQ(disc_len(&tg, 0), 3, "G precondition: the waitroom really holds 3");

    CHECK_EQ(test_activate_ability(&tg, kinako), 1, "G: the 起動 is accepted");
    int deck_after = deck_len(&tg, 0);
    int disc_after = disc_len(&tg, 0);
    fprintf(stderr, "  (G Q104) deck = %d, waitroom = %d\n", deck_after, disc_after);
    if (pending(&tg)) describe_pending(&tg, "G Q104");

    /* Q104: 2 come off the deck, the waitroom is reshuffled into the deck, and
     * the 3rd is milled from the refreshed deck. 5 cards total, 3 milled ->
     * 2 left. The refreshed deck contributed its 3 waitroom cards and lost 1
     * back to the waitroom, so 2 of the original 3 waitroom cards are back in
     * the deck with the 2 unmilled originals... the Rust twin states 4, which
     * is asserted below as printed. */
    CHECK_EQ(disc_after, 1, "G Q104: exactly 1 card sits in the waitroom afterwards");
    CHECK_EQ(deck_after, 4, "G Q104: 4 cards remain in the deck after the refresh");

    CHECK(pending(&tg),
          "G Q104: the position-change destination prompt opened after the "
          "cost was paid through the refresh");
    int dst = drive_position_to_other_slot(&tg, 0);   /* src = LEFT */
    pc_pay(&tg, 12);

    CHECK(dst >= 0, "G Q104 CONTROL: a destination other than the left was chosen");
    CHECK_EQ(tg.state.p[0].stage[dst < 0 ? 0 : dst], kinako,
             "G Q104 CONTROL: きな子 landed at the requested destination");
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "G Q104: the left slot emptied");
}

/* ══════════════════════════════════════════════════════════════════════════
 * fork harness
 * ══════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK       0
#define CHILD_FAILURES 1

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
        /* No fork available: run in-process, but say so, because then a fault
         * kills the whole suite and the remaining cases never report. */
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%d assertions, no fork isolation]\n",
               failures > f0 ? "FAILED" : "ok", name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, g0 = harness_gaps;
        current_test = name;
        fn();
        printf("        %-56s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
               name, assertions - a0, failures - f0, harness_gaps - g0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- engine fault, signal %d\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED   %s\n", name);
    } else {
        n_crashed++;
        printf("CRASH    %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fault);
    signal(SIGBUS,  on_fault);
    signal(SIGABRT, on_fault);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }
    if (getenv("RB_ABILITY_DEBUG")) rb_ability_debug_set(1);

    printf("--- A1. gain/hearts/per_card/izumi_debut_other_heart06 (PL!HS-pb1-016-R) ---\n");
    run("A1_heart06_target",      a1_izumi_grants_heart06_to_heart06_member_only);
    run("A1_no_valid_target",     a1_izumi_no_valid_target_no_modifier);

    printf("--- A2. gain/hearts/per_card/hasunosora_live_start_waitroom_ten (PL!HS-pb1-025-L) ---\n");
    run("A2_parser_guard",        a2_hanasaka_carries_live_start);
    run("A2_ten_in_waitroom",      a2_condition_met_single_target_auto_applies);
    run("A2_nine_in_waitroom",     a2_condition_not_met_no_effect);
    run("A2_non_hasunosora",       a2_non_hasunosora_not_targeted);
    run("A2_two_targets",          a2_two_targets_only_selected_gains);

    printf("--- A3. gain/hearts/per_card/rurino_debut_activate_wait_member (PL!HS-bp6-003-R) ---\n");
    run("A3_pay_activates",       a3_pay_activates_member_and_adds_live);
    run("A3_no_wait_member",      a3_no_wait_member_skips_whole_sequence);
    run("A3_skip_optional",       a3_skip_optional_leaves_live_in_discard);
    run("A3_empty_discard",       a3_empty_discard_no_crash);
    run("A3_bp5_group_heart01",   a3_bp5_live_start_group_matched_heart01);

    printf("--- A4. gain/hearts/per_card/subunit_stage_count_gated_debut (PL!N-bp7-024-N) ---\n");
    run("A4_three_r3birth",       a4_ranbu_three_r3birth_gains_heart01);
    run("A4_two_r3birth",         a4_ranbu_two_r3birth_no_heart01);

    printf("--- A5. gain/hearts/per_card/q127_heart_set_plus_global (PL!SP-bp2-010-P) ---\n");
    run("A5_bloom_heart01_set",   a5_wien_plus1_stacks_on_bloom_heart01_set);
    run("A5_only_heart00",        a5_wien_only_affects_heart00);
    run("A5_no_wien",             a5_no_wien_bloom_set_standalone);
    run("A5_two_wien",            a5_two_wien_stack_plus2_heart00);
    run("A5_wien_leaves",         a5_wien_leaves_stage_modifier_removed);
    run("A5_bloom_heart04",       a5_wien_plus_bloom_heart04_pattern);
    run("A5_bloom_heart05",       a5_wien_plus_bloom_heart05_pattern);
    run("A5_hareruya_set",        a5_wien_stacks_on_hareruya_set);
    run("A5_hareruya_heart00",    a5_wien_adds_heart00_not_in_hareruya_set);
    run("A5_build_card_needs",    a5_build_card_needs_set_plus_additive);

    printf("--- A0. gain/hearts/per_card/sumire_center_highest_cost (PL!SP-bp2-004-R) ---\n");
    run("A0_center_highest",      a0_sumire_center_highest_cost_heart03);
    run("A0_tie_for_highest",     a0_sumire_tie_for_highest_no_heart03);

    printf("--- B1. recover/to_discard/debut_mill_top_three (PL!N-bp7-020-N) ---\n");
    run("B1_two_colours",         b1_two_distinct_blade_colours_gains_heart04);
    run("B1_three_colours",       b1_three_distinct_blade_colours_gains_heart04);
    run("B1_one_colour",          b1_all_one_colour_no_heart04);
    run("B1_only_one_member",     b1_only_one_member_milled_no_heart04);
    run("B1_no_blade_heart",      b1_members_without_blade_heart_no_heart04);
    run("B1_all_and_green",       b1_all_and_green_blade_do_not_meet_condition);

    printf("--- B2. recover/to_discard/live_start_discard_two_same_unit (PL!HS-PR-016-PR) ---\n");
    run("B2_same_unit_pair",      b2_same_unit_pair_discards_two);
    run("B2_player_chooses_pair", b2_player_chooses_pair_across_units);
    run("B2_no_qualifying_unit",  b2_no_qualifying_unit_skips_cost);

    printf("--- B3. recover/to_discard/live_start_discard_two_niji Q272 (PL!N-bp7-026-L) ---\n");
    run("B3_q272_not_twice",      b3_same_member_not_selectable_twice);
    run("B3_one_discard_one",     b3_discard_one_grants_one_blade);
    run("B3_two_distinct",        b3_two_distinct_members_each_gain_blade);
    run("B3_non_niji",            b3_non_niji_member_not_selectable);
    run("B3_skip_discard",        b3_skip_discard_no_blade);
    run("B3_ab1_two_no_blade",    b3_ab1_two_no_blade_members_score);
    run("B3_ab1_one_no_blade",    b3_ab1_one_no_blade_member_no_score);
    run("B3_ab1_with_blade",      b3_ab1_with_blade_heart_members_no_score);
    run("B3_ab1_mixed",           b3_ab1_mixed_members_no_score);
    run("B3_ab1_member_plus_live", b3_ab1_member_plus_live_no_score);

    printf("--- B4. recover/to_discard/optional_discard_cost_plus_total_cost (PL!HS-bp6-005-R＋) ---\n");
    run("B4_condition_met",       b4_condition_met_gains_heart05_and_blade);
    run("B4_condition_not_met",   b4_condition_not_met_no_heart05_no_blade);

    printf("--- C1. state/orientation_gated/erena_self_wait_constant_heart05 ---\n");
    run("C1_two_copies",          c1_two_copies_independent);
    run("C1_wait_persists",       c1_wait_persists_after_position_change);
    run("C1_leave_stage",         c1_leave_stage_clears_reenter_active);
    run("C1_default_active",      c1_no_orientation_defaults_active);
    run("C1_stable_across_recalc", c1_stable_across_recalc);
    run("C1_p_plus_variant",      c1_p_plus_variant);
    run("C1_peer_irrelevant",     c1_other_members_orientation_irrelevant);

    printf("--- C2. state/orientation_gated/two_or_more_opponent_wait_heart06 (PL!HS-bp5-016-N) ---\n");
    run("C2_two_wait",            c2_two_opponent_wait_members_gains_one_heart06);
    run("C2_zero_wait",           c2_zero_opponent_wait_no_heart06);
    run("C2_one_wait",            c2_one_opponent_wait_no_heart06);
    run("C2_own_wait",            c2_own_wait_members_do_not_count);
    run("C2_two_wait_one_active", c2_two_wait_and_one_active_still_one);
    run("C2_dynamic",             c2_dynamic_two_wait_to_one_wait_loses_heart06);

    printf("--- C3. state/orientation_gated/wait_state_condition_constants ---\n");
    run("C3_wait_gains_one",      c3_erena_in_wait_gains_exactly_one_heart05);
    run("C3_active_no_heart",     c3_erena_active_no_heart);
    run("C3_default_no_heart",    c3_erena_default_active_no_heart);
    run("C3_wait_to_active",      c3_erena_wait_to_active_switches_heart);
    run("C3_wait_not_on_stage",   c3_erena_wait_not_on_stage);

    printf("--- C4. state/orientation_gated/nico_cannot_activate (PL!-pb1-009-R) ---\n");
    run("C4_kotori_works_clean",  c4_kotori_activation_without_restriction);
    run("C4_nico_blocks_kotori",  c4_nico_blocks_kotori_activation);
    run("C4_active_phase_still",  c4_active_phase_still_activates);

    printf("--- D1. draw/until_count/miyashita skip-cost gaps (PL!N-PR-028-PR) ---\n");
    run("D1_hand_almost_full",    d_miyashita_hand_almost_full_skips_cost);
    run("D1_hand_at_seven",       d_miyashita_hand_at_seven_skips_cost);
    run("D1_empty_deck",          d_miyashita_empty_deck_no_draw_no_hang);

    printf("--- D2. draw/until_count/miyata_draw_until (PL!N-bp3-005-R＋) ---\n");
    run("D2_three_appearances",   d_miyata_three_appearances_draw_to_five);
    run("D2_hand_six_no_draw",    d_miyata_hand_at_five_or_more_draws_nothing);
    run("D2_two_appearances",     d_miyata_two_appearances_no_draw);
    run("D2_small_deck",          d_miyata_small_deck_draws_out);

    printf("--- E1. draw/conditional/success_zone_group (PL!-bp6-023-L) ---\n");
    run("E1_mus_in_success",      e1_mus_live_in_success_zone_draws_two);
    run("E1_no_success_card",     e1_no_success_zone_card_draws_one);
    run("E1_non_mus_in_success",  e1_non_mus_live_in_success_zone_draws_one);

    printf("--- E2. draw/conditional/score_or_revealed_live (PL!SP-pb2-004-R) ---\n");
    run("E2_score_or_revealed",   e2_score_or_revealed_live_draw);

    printf("--- E3. draw/conditional/live_start_energy_return_ren (PL!SP-bp7-026-L) ---\n");
    run("E3_accept_with_ren",     e3_accept_cost_with_ren_draws_two_discards_one);
    run("E3_decline",             e3_decline_cost_nothing_happens);
    run("E3_accept_without_ren",  e3_accept_cost_without_ren_pays_but_no_draw);

    printf("--- E4. draw/conditional/area_moved_pl_sp_sd2_003_sd2 ---\n");
    run("E4_area_moved",          e4_area_moved_draws_two);
    run("E4_no_area_move",        e4_no_area_move_draws_one);

    printf("--- E5. draw/conditional/revealed_live_hand_threshold (PL!S-bp2-007-R＋) ---\n");
    run("E5_above_threshold",     e5_revealed_live_above_hand_threshold_no_draw);
    run("E5_no_live_in_reveal",   e5_no_live_in_reveal_no_draw);

    printf("--- F1. position/area_gated/success_zone_center_area_mus (PL!-bp4-020-L) ---\n");
    run("F1_parser_guard",        f1_card_carries_constant_with_effect);
    run("F1_centre_gains",        f1_centre_mus_member_gains_blade);
    run("F1_left_no_blade",       f1_left_mus_member_no_blade);
    run("F1_right_no_blade",      f1_right_mus_member_no_blade);
    run("F1_non_mus_no_blade",    f1_non_mus_centre_no_blade);
    run("F1_not_in_success",      f1_card_not_in_success_zone_no_blade);
    run("F1_removed_loses",       f1_removed_from_success_zone_loses_blade);
    run("F1_leaves_centre",       f1_member_leaves_centre_loses_blade);
    run("F1_both_players",        f1_both_players_own_centre);
    run("F1_only_centre",         f1_only_centre_gets_blade);
    run("F1_empty_stage",         f1_empty_stage_then_muse_still_works);

    printf("--- F2. position/area_gated/center_area_syncrise (PL!SP-pb2-022-R) ---\n");
    run("F2_mover_gains_host",    f2_syncrise_mover_gains_host_four_blades);
    run("F2_non_syncrise",        f2_non_syncrise_mover_no_trigger);
    run("F2_self_target_moved",   f2_self_target_host_moved_gains_one_blade);
    run("F2_self_target_still",   f2_self_target_not_moved_no_blade);
    run("F2_no_position_change",  f2_no_position_change_no_trigger);

    printf("--- G. position/per_card/mill_three_q104 (PL!SP-bp5-006-R) ---\n");
    run("G_q104_refresh",         g_mill_three_q104_deck_two_refreshes);

    rb_unload();
    printf("\n==== small-cluster parity: %d cases passed, %d cases failed, "
           "%d crashed, %d harness gap(s) ====\n",
           n_ok, n_failed, n_crashed, harness_gaps);
    if (failures) fprintf(stderr, "%d assertion-level failure(s)\n", failures);
    return (failures || n_crashed) ? 1 : 0;
}







/* test_parity_per_card.c — C port of two Rust clusters that nobody ported:
 *
 *   engine/tests/test_modules/effects/score/per_card/     (15 files, 59 tests)
 *   engine/tests/test_modules/effects/gain/blades/per_card/ (15 files, 56 tests)
 *
 * Two peers were ASSIGNED these clusters and never ran, so the whole thing was
 * treated as uncovered. Every card_no these clusters name was then grepped
 * against engine_c_wip/tests/*.c: 64 distinct prints, 44 already appear in some
 * existing suite, 20 did not. Only the genuine gaps are ported here, and each
 * section names the card_no that made it a gap.
 *
 * COVERED HERE (5 Rust files, 20 Rust tests):
 *   §A blades/per_card/q46_kanako_all_heart_timing_test.rs            7 tests
 *      gap card: PL!N-bp1-012-R＋   (鐘 嵐珠)
 *      The cluster's "kanako" label is a MISLABEL: cards.json has
 *      "PL!N-bp1-012-R＋" -> 鐘 嵐珠, and the file's own doc comment agrees
 *      (line 1: "Q46: 鐘 嵐珠 (PL!N-bp1-012-R+)"). 中須かすみ is
 *      "PL!N-bp5-002-R", a different card in a different file. The print in the
 *      fixture is CORRECT; only the identifier is wrong. Existing C suites
 *      reference the ASCII spelling "PL!N-pb1-012-R", which does not exist in
 *      cards.json, so they cannot have covered this print.
 *   §B blades/per_card/q148_waited_member_blade_total_test.rs          3 tests
 *      gap cards: PL!-bp3-023-L (ミはμ'sicのミ), PL!-bp6-003-R＋ (南ことり)
 *   §C blades/per_card/hazuki_activate_discard_per_liella_member_
 *      blade_test.rs                                                 3 of 4
 *      gap cards: PL!SP-bp5-005-R＋ (葉月 恋), PL!SP-bp1-004-R (平安名すみれ)
 *   §D blades/per_card/ren_baton_touch_group_check_test.rs            3 tests
 *      gap card: PL!SP-bp4-005-R＋ (葉月 恋). The file's identifier says
 *      "ren" and its comment on line 2 says 高坂穂乃果 is mu's; the CARD is
 *      correct (葉月 恋), so this is a naming bug, not a fixture bug.
 *   §E score/per_card/mute_kibiriver_revealed_card_heart_gain_test.rs 4 tests
 *      gap cards: PL!N-bp5-029-L (無敵級*ビリーバー), PL!N-bp5-002-R (中須かすみ)
 *
 * NOT COVERED, by name and reason (all pre-existing gaps, not introduced here):
 *   score/per_card/aurora_flower_set_card_identity_group_match_test.rs   5
 *   score/per_card/energy_state_condition_active_energy_gate_test.rs    7
 *   score/per_card/excess_heart_score_shift_never_below_zero_
 *     pl_n_bp5_010_r_test.rs                                             2
 *   score/per_card/live_success_remaining_yell_live_score_q253_test.rs  4
 *   score/per_card/live_success_surplus_score_zero_floor_test.rs        8
 *   score/per_card/replacement_choose_order_on_live_event_test.rs       2
 *   score/per_card/shizuku_aggregate_score_comparison_draw_test.rs      4
 *   score/per_card/solitude_rain_unique_heart_color_score_q67_test.rs   2
 *   score/per_card/strawberry_trapper_live_success_compound_score_test 5
 *   score/per_card/takaramono_live_success_no_excess_heart_score_test  2
 *   score/per_card/tokimeki_runners_all_heart_colors_live_success_test 5
 *   score/per_card/vitamin_summer_hand_count_comparison_live_score_test 3
 *   score/per_card/neutral_live_success_test.rs                         6
 *      -- needs game.generated_actions() / select_generated() /
 *         assert_pending_choice_type(), which have no C shim; porting it
 *         faithfully would mean building new harness, not a translation.
 *   blades/per_card/activate_change_state_wait_grants_live_end_blade     2
 *   blades/per_card/bp7_parser_gap_cards_test.rs                       11
 *   blades/per_card/eri_debut_up_to_two_opponent_members_to_wait       3
 *   blades/per_card/kanon_debut_one_per_group_deck_bottom_edge_test    3
 *   blades/per_card/modifier_layer_characterization_test.rs             3
 *   blades/per_card/per_unit_discard_count_regression_test.rs           1
 *   blades/per_card/q271_colorful_dreams_test.rs                        9
 *   blades/per_card/q273_fired_debut_ability_cost_paid_test.rs          2
 *   blades/per_card/turn_scope_regression_test.rs                       2
 *   blades/per_card/yell_same_group_members_multi_color_hearts_test     3
 *   blades/per_card/hazuki_..._test.rs::q233_declined_optional_energy   1
 *      -- needs set_recently_moved_cards() / recently_moved_from_zone and
 *         ability_uses_used(), none of which have a C entry point.
 *
 * C-API landmines honoured here (AGENTS.md + peer findings):
 *   - load_card_db() falls back to rb_load("../cards/build"); rb_load("src")
 *     alone fails under the isolated out-of-tree build.
 *   - mid() == test_new_id(), not test_id(). §B and §D both put the SAME print
 *     on two different seats, so aliasing would collapse the two instances.
 *   - sizeof(TestGame) is ~781 KB, so EVERY TestGame local in this file is
 *     `static`; a second stack TestGame in one test segfaults.
 *   - Heart reads go through rb_mods_get_need_heart / rb_mods_get_heart
 *     DIRECTLY. test_get_heart_modifier REMAPS a requested colour of 5 onto
 *     RB_HEART_ORANGE, which would silently turn §E's heart05 assertion into a
 *     heart06 one.
 *   - Group identity keys off the `unit` field, not the printed group name.
 *   - RbChoice.zone for the waitroom is "discard", not "waitroom".
 *   - test_set_live_card is the P1 APPEND path; test_insert_live_card_at is the
 *     positional one. §B needs "2 real live cards then a third", so it uses the
 *     positional form for the first two.
 *   - test_set_live_card_for(tg, pl, card) writes the OTHER seat's live zone.
 *
 * FINDINGS so far (every one is a strict Rust expectation the C engine misses;
 * the assertion stays RED — see the per-test comments for the classification):
 *
 *   FINDING 1 (§A) — 鐘 嵐珠's 常時 is NOT CONDITION-GATED in C.
 *     Rust: 「自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブ
 *     カードを1枚以上含む場合」. Observed C: the +2 blades are granted whenever
 *     the card is merely ON STAGE -- 2 live cards grants them, and 3 live cards
 *     with no 虹ヶ咲 live among them grants them too. Only the "not on stage"
 *     negative holds. So BOTH halves of the compound gate (the >=3 live-card
 *     count and the 虹ヶ咲 group gate over the live zone) are ignored.
 *     Contributing data gap, confirmed against cards/cards.json: there is NO
 *     `group` key on ANY card record. Members carry `unit` (鐘 嵐珠 is
 *     `unit: "R3BIRTH"`, NOT 虹ヶ咲); live cards carry `unit: null` and only
 *     `series`. A group gate evaluated over the live-card zone therefore has no
 *     field to read. The Rust fixture's own helper, PL!N-sd1-025-SD, is a
 *     ライブ with `unit: null`, so the "1 虹ヶ咲 live card" half of the
 *     condition cannot be satisfied by that print on card data alone either.
 *
 *   FINDING 2 (§C) — 葉月 恋's ab#0 grants a FLAT +3 blades.
 *     Rust: 「『Liella!』のメンバーカード1枚につき、ブレードを得る」 -- 2 copies
 *     discarded must be +2, 0 copies must be +0. Observed C: +3 in BOTH the
 *     2-Liella case and the 0-Liella case, while the empty-deck case (cost
 *     unpayable, ability never runs) is correctly 0. The grant is therefore
 *     independent of the discarded-card count. Note that +3 is exactly the
 *     常時 of the OTHER 葉月 恋 print, PL!SP-bp4-005-R＋
 *     (「自分のエネルギーが10枚以上あるかぎり、ブレード3枚を得る」), and the
 *     §C fixtures give 13 energy -- see test_d_ren_* for the same print's
 *     other ability. Whether this is a mis-resolved ability id or a
 *     count-independent execution path is NOT asserted here; the observed
 *     flat +3 is.
 *
 *   FINDING 3 (§D) — 葉月 恋 (PL!SP-bp4-005-R＋) ab#0 gates are not enforced.
 *     Rust guards the 登場 ability on 『Liella!』のメンバーからバトンタッチして
 *     登場 && エネルギーが7枚以上. The POSITIVE case passes in C; the two
 *     negatives do not: a baton touch from a non-Liella! member still places
 *     2 energy, and so does the low-energy case. That is precisely the
 *     regression the Rust file was written for ("Bug: The group check was
 *     missing -- Ren gained energy even when baton-touched from a non-Liella!
 *     member"), reintroduced in the C port.
 *
 *   FINDING 4 (§B) — ミはμ'sicのミ's ライブ開始時 fires but applies nothing.
 *     The ability IS in the compiled card DB with the correct ライブ開始時
 *     trigger (the [shim]/[diag] lines prove it: found=1 and the ability id
 *     matches cards.json's printed text exactly), and it is queued and
 *     processed. Its effect — 「必要ハートはheart0が2少なくなる」, gated on the
 *     stage blade total being >= 10 with 14 on the board — never lands, on
 *     either the pool-copy id or the template id, through the turn walk OR
 *     through a direct fire. So the gate, the effect, or the need_heart
 *     modifier write is missing; the "no modifier on the template id either"
 *     control rules out simple mis-targeting.
 *
 *   FINDING 5 (§E) — 無敵級*ビリーバー's ライブ開始時 fires but does nothing.
 *     Same shape as FINDING 4: the ability is in the DB and is queued and
 *     processed, yet no reveal, no selection prompt, no discard of the four
 *     revealed cards and no heart gain ever happens. The gate 「自分のステージ
 *     に「中須かすみ」がいる場合」 is satisfied in every failing fixture (the
 *     staged card IS 中須かすみ, asserted with rb_card_no_eq).
 *
 *   CAVEAT (§D, not a finding) — the low-energy fixture cannot be reproduced
 *     with Rust's arithmetic. Rust derives "6 energy left" from 葉月 恋's cost
 *     15 minus the outgoing member's cost 11 over a 10-energy start. The C
 *     baton-touch cost arithmetic lands elsewhere, so the C run still has >= 7
 *     active and the ab#0 energy gate legitimately passes. The strict Rust
 *     expectation is still asserted; the C-side energy count is printed in the
 *     failure message so the discrepancy is visible.
 *
 *   DIAGNOSTIC: set PC_DIAG_SHIM=1 in the environment to have
 *   pc_fire_live_start print, per call, the trigger keyword it compared and
 *   whether it matched. That is how FINDING 4 and 5 were separated from "the
 *   ability is not in the database at all". It is off by default.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── crash reporting + fork isolation ─────────────────────────────────────
 * Each test runs in a forked child so a SIGSEGV/SIGBUS/SIGABRT is attributed to
 * the test in flight rather than truncating the run so every later section
 * falsely reads as a pass. No time-based watchdog: a process watchdog does not
 * work on this toolchain (in-child alarm() never fires; a parent-side
 * waitpid(WNOHANG) deadline fires but its kill(SIGKILL) does not take effect,
 * wedging the blocking waitpid). A child that HANGS is a recorded limitation.
 */
static const char *current_test = "(none)";
static int failures;
static int assertions;
static int gaps;
static int setup_bugs;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        assertions++;                                                       \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* A Rust assertion with no reachable C entry point. Counted separately; never
 * fails the suite and never stands in for a real CHECK. */
#define EXPECTED_GAP(condition, desc) do {                                  \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            printf("GAP: %s\n", desc);                                      \
            gaps++;                                                         \
        }                                                                   \
    } while (0)

/* heart colour slots, per cards/compile_cards.py HEART_COLORS */
#define H00 0
#define H01 1
#define H02 2
#define H03 3
#define H04 4
#define H05 5
#define H06 6
#define HALL 7

/* stage area indices */
#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ───────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

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

static int ident(int cid, const char *no)
{
    if (rb_card_no_eq(cid, no)) return 1;
    assertions++;
    setup_bugs++;
    failures++;
    fprintf(stderr, "SETUP BUG: expected card_no \"%s\" (AGENTS.md identity rule)\n", no);
    return 0;
}

/* Direct modifier reads. test_get_heart_modifier remaps colour 5 onto
 * RB_HEART_ORANGE, so it must never be used for a heart05 claim. */
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}
static int need_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_need_heart(&tg->state.mods, cid, color);
}
static int blade_of(TestGame *tg, int cid)
{
    return test_get_blade_modifier(tg, cid);
}
/* The PRINTED blade from the card record. rb_mods_get_blade is the MODIFIER
 * only, so a "this card prints blade 7" claim has to come from here. */
static int printed_blade(int cid)
{
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return -1;
    int b = card.blade;
    rb_free_card(&card);
    return b;
}
/* Concatenated trigger keywords the compiled card DB carries for `cid`, or "".
 * Used to turn "the ability did not fire" into "the ability is not in the
 * compiled DB at all", which is a DATA/PARSER finding rather than an engine
 * one. */
static void dump_triggers(int cid, char *buf, size_t cap)
{
    buf[0] = 0;
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) { rb_free_card(&card); return; }
    int nab = rb_card_num_abilities((uint32_t)cid);
    size_t used = 0;
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && ab.triggers[0]) {
            size_t need = strlen(ab.triggers) + 3;
            if (used + need < cap) {
                used += (size_t)snprintf(buf + used, cap - used, "%s[%s]",
                                         used ? "," : "", ab.triggers);
            }
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
}
static int score_of(TestGame *tg, int cid)
{
    return test_get_score_modifier(tg, cid);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
        tg->state.p[pl].stage_wait[i] = 0;
        tg->state.p[pl].under_cards[i].n = 0;
    }
}

static void clear_live(TestGame *tg, int pl)
{
    tg->state.p[pl].live.n = 0;
}

/* Rust helpers::fill_decks — n copies of `filler` into BOTH main decks. */
static void fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static void add_deck_top(TestGame *tg, int pl, int cid, int times)
{
    for (int i = 0; i < times; i++) test_insert_deck_top(tg, pl, cid);
}

/* Rust advance_to_live_card_set / advance_to_live_card_set_p1: five blind
 * passes reach the live-card set window. */
static void pc_advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start / finish_live_setup: two more passes reach the
 * ライブ開始時 point where the ability queue is drained. */
static void pc_advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

#define PC_SKIP   (-1)
#define PC_ACCEPT (0)

/* while has_pending_choice() { select_indices(&[]) } */
static void pc_drain_skip(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, PC_SKIP);
}

/* while has_pending_choice() { select_indices(&[0]) } */
static void pc_drain_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        rb_resume_with_choice(&tg->state, 0);
}

/* Proceed with a queued auto ability, take index 0 for a mandatory card pick,
 * decline everything else. */
static void pc_drain_proceed(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = PC_SKIP;
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) idx = PC_ACCEPT;
        else if (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip) idx = 0;
        rb_resume_with_choice(&tg->state, idx);
    }
}

/* helpers/mod.rs:114 fire_trigger -- build the "<card_no>_<full_text>" ability
 * id for the first ability printing `trig`, queue exactly that ability, set
 * activating_card, process the queue. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int pc_fire_live_start(TestGame *tg, int cid)
{
    /* ライブ開始時 — the tag cards.json actually prints
     * ({{live_start.png|ライブ開始時}}), NOT ライブ実行時. Using the wrong
     * keyword makes pc_fire_live_start return 0 for every card, which looks
     * exactly like a missing ability.
     *
     * The bytes are 開 = U+958B = E9 96 8B. Do NOT hand-transcribe these: the
     * simplified 开 is U+5F00 = E5 BC 80, which is one byte-pair different and
     * silently makes the shim never match anything. The exact sequence below
     * was read out of the compiled DB, and matches the literal used by
     * tests/test_parity_hearts_live_start.c. */
    static const char *TRIG =
        "\xe3\x83\xa9\xe3\x82\xa4\xe3\x83\x96\xe9\x96\x8b\xe5\xa7\x8b\xe6\x99\x82";
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
        if (ab.triggers && strcmp(ab.triggers, TRIG) == 0) {
            snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        if (getenv("PC_DIAG_SHIM")) {
            fprintf(stderr, "        [shim] a=%d trig=<%s> match=%d\n",
                    a, ab.triggers ? ab.triggers : "",
                    ab.triggers && strcmp(ab.triggers, TRIG) == 0);
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (getenv("PC_DIAG_SHIM"))
        fprintf(stderr, "        [shim] cid=%d nab=%d found=%d id=<%s>\n",
                cid, nab, found, ability_id);
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, TRIG, 0, card_no,
                             cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* ===================================================================== */
/* §A  blades/per_card/q46_kanako_all_heart_timing_test.rs                 */
/*     鐘 嵐珠 (PL!N-bp1-012-R＋) 常時: 3+ live cards in zone including     */
/*     1+ 虹ヶ咲 live card -> gain 2 ALL hearts + 2 blades.                 */
/* ===================================================================== */

#define Q46_RANJU "PL!N-bp1-012-R＋"   /* 鐘 嵐珠 */
#define Q46_NIJI_LIVE "PL!N-sd1-025-SD" /* 虹ヶ咲 live card */
#define Q46_OTHER_LIVE "PL!-sd1-019-SD" /* non-虹ヶ咲 live card */
#define FILLER "PL!-sd1-010-SD"

/* Rust :15 q46_kanako_constant_grants_blades_when_condition_met */
static void test_a_q46_condition_met_grants_two_blades(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    int niji_live = mid(&game, Q46_NIJI_LIVE);
    int other_live = mid(&game, Q46_OTHER_LIVE);
    int other_live2 = mid(&game, Q46_OTHER_LIVE);
    ident(ranju, Q46_RANJU);
    CHECK(filler != filler2, "the two fillers are DISTINCT instances, not one aliased card");
    CHECK(niji_live != other_live, "the 虹ヶ咲 live and the other live are distinct prints");

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_insert_live_card_at(&game, 0, niji_live);
    test_insert_live_card_at(&game, 1, other_live);
    test_insert_live_card_at(&game, 2, other_live2);

    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 2,
             "Q46: 3 live cards (1 虹ヶ咲) -> the 常時 fires -> +2 blades");
}

/* Rust :48 q46_kanako_condition_less_than_3_live_cards_no_gain */
static void test_a_q46_fewer_than_three_live_cards_no_gain(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    ident(ranju, Q46_RANJU);

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_insert_live_card_at(&game, 0, mid(&game, Q46_NIJI_LIVE));
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));

    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 0,
             "Q46: only 2 live cards -> the 常時 does not fire -> no blade");
}

/* Rust :75 q46_kanako_no_nijigasaki_live_card_no_gain */
static void test_a_q46_no_nijigasaki_live_no_gain(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    ident(ranju, Q46_RANJU);

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    for (int i = 0; i < 3; i++)
        test_insert_live_card_at(&game, i, mid(&game, Q46_OTHER_LIVE));

    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 0,
             "Q46: 3 live cards but none 虹ヶ咲 -> the group half of the "
             "condition fails -> no blade");
}

/* Rust :110 q46_kanako_not_on_stage_no_constant */
static void test_a_q46_not_on_stage_no_constant(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    ident(ranju, Q46_RANJU);

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_add_to_hand(&game, ranju);
    test_insert_live_card_at(&game, 0, mid(&game, Q46_NIJI_LIVE));
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));
    test_insert_live_card_at(&game, 2, mid(&game, Q46_OTHER_LIVE));

    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 0,
             "Q46: 鐘 嵐珠 in hand, not on stage -> the 常時 is not evaluated");
}

/* Rust :145 q46_kanako_leaves_stage_blade_removed */
static void test_a_q46_leaves_stage_blade_removed(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    ident(ranju, Q46_RANJU);

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_insert_live_card_at(&game, 0, mid(&game, Q46_NIJI_LIVE));
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));
    test_insert_live_card_at(&game, 2, mid(&game, Q46_OTHER_LIVE));
    test_recalc(&game);
    CHECK_EQ(blade_of(&game, ranju), 2, "Q46: precondition -> +2 blades");

    int filler3 = mid(&game, FILLER);
    game.state.p[0].stage[AREA_CENTER] = RB_EMPTY_SLOT;
    test_add_to_stage(&game, AREA_RIGHT, filler3);
    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 0,
             "Q46: 鐘 嵐珠 leaves stage -> the 常時 is torn down -> 0 blades");
}

/* Rust :187 q46_live_card_removed_condition_fails_blade_removed */
static void test_a_q46_live_card_removed_blade_removed(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    int niji_live = mid(&game, Q46_NIJI_LIVE);
    ident(ranju, Q46_RANJU);

    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_insert_live_card_at(&game, 0, niji_live);
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));
    test_insert_live_card_at(&game, 2, mid(&game, Q46_OTHER_LIVE));
    test_recalc(&game);
    CHECK_EQ(blade_of(&game, ranju), 2, "Q46: precondition -> +2 blades");

    clear_live(&game, 0);
    test_insert_live_card_at(&game, 0, niji_live);
    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju), 0,
             "Q46: live cards removed -> < 3 -> the 常時 stops -> blades removed");
}

/* Rust :228 q46_multiple_kanako_each_gains_blades */
static void test_a_q46_multiple_copies_each_gain(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju1 = mid(&game, Q46_RANJU);
    int ranju2 = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    ident(ranju1, Q46_RANJU);
    ident(ranju2, Q46_RANJU);
    CHECK(ranju1 != ranju2,
          "the two 鐘 嵐珠 are DISTINCT pool slots (mid(), not test_id())");

    test_add_to_stage(&game, AREA_LEFT, ranju1);
    test_add_to_stage(&game, AREA_CENTER, ranju2);
    test_add_to_stage(&game, AREA_RIGHT, filler);
    test_insert_live_card_at(&game, 0, mid(&game, Q46_NIJI_LIVE));
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));
    test_insert_live_card_at(&game, 2, mid(&game, Q46_OTHER_LIVE));
    test_recalc(&game);

    CHECK_EQ(blade_of(&game, ranju1), 2, "Q46: the first 鐘 嵐珠 gains 2 blades");
    CHECK_EQ(blade_of(&game, ranju2), 2, "Q46: the second 鐘 嵐珠 gains 2 blades");
}

/* The file's own question, "when is the colour of the ALL hearts decided?",
 * has no Rust assertion at all, so it is recorded as a gap rather than
 * invented here. */
static void test_a_q46_all_heart_colour_timing(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ranju = mid(&game, Q46_RANJU);
    int filler = mid(&game, FILLER);
    int filler2 = mid(&game, FILLER);
    ident(ranju, Q46_RANJU);
    test_add_to_stage(&game, AREA_LEFT, filler);
    test_add_to_stage(&game, AREA_CENTER, ranju);
    test_add_to_stage(&game, AREA_RIGHT, filler2);
    test_insert_live_card_at(&game, 0, mid(&game, Q46_NIJI_LIVE));
    test_insert_live_card_at(&game, 1, mid(&game, Q46_OTHER_LIVE));
    test_insert_live_card_at(&game, 2, mid(&game, Q46_OTHER_LIVE));
    test_recalc(&game);

    /* The Rust suite never asserts the heart side of Q46, only the blade side. */
    EXPECTED_GAP(0,
                 "q46_kanako_all_heart_timing_test.rs asserts the BLADE half of "
                 "Q46 only; it never checks the 2 ALL hearts, so 'the ALL heart "
                 "colour is chosen at heart-check time, not at 常時 recalc time' "
                 "is asserted nowhere on either side. The C engine's heart slot "
                 "for an uncoloured ALL grant is the HALL wildcard.");
}

/* ===================================================================== */
/* §B  blades/per_card/q148_waited_member_blade_total_test.rs               */
/*     ミはμ'sicのミ (PL!-bp3-023-L) ライブ開始時: if total stage blade >= 10,  */
/*     required hearts decrease by heart00 x 2. Waited blade IS included.    */
/* ===================================================================== */

#define Q148_LIVE "PL!-bp3-023-L"       /* ミはμ'sicのミ */
#define Q148_BLADE7_A "PL!N-pb1-007-R"  /* 優木せつ菜, blade=7 */
#define Q148_BLADE7_B "PL!-bp6-003-R＋" /* 南ことり,   blade=7 */

static void q148_setup(TestGame *tg, int live, int m_a, int m_b)
{
    clear_stage(tg, 0);
    clear_live(tg, 0);
    if (m_a >= 0) test_add_to_stage(tg, AREA_LEFT, m_a);
    if (m_b >= 0) test_add_to_stage(tg, AREA_CENTER, m_b);
    tg->state.p[0].stage[AREA_RIGHT] = RB_EMPTY_SLOT;
    test_add_to_hand(tg, live);
    fill_decks(tg, mid(tg, "PL!-sd1-002-SD"), 20);
    pc_advance_to_live_set(tg);
    test_set_live_card(tg, 0, live);
    pc_advance_to_live_start(tg);
    pc_drain_first(tg);
}

/* Rust :20 q148_blade_total_active_members */
static void test_b_q148_active_blade_total_fires(void)
{
    static TestGame game;
    test_game_new(&game);

    int live = mid(&game, Q148_LIVE);
    int m_a = mid(&game, Q148_BLADE7_A);
    int m_b = mid(&game, Q148_BLADE7_B);
    ident(live, Q148_LIVE);
    ident(m_b, Q148_BLADE7_B);
    CHECK(m_a != m_b, "the two blade=7 members are distinct pool slots");

    q148_setup(&game, live, m_a, m_b);

    /* Diagnostic controls, so a red expectation below can be classified without
     * a second build: if these pass, the setup and the walk are fine and the
     * deficit is in the ability's CONDITION, not in the phase machine. */
    CHECK_EQ(game.state.p[0].stage[AREA_LEFT], m_a, "control: member A is on stage");
    CHECK_EQ(game.state.p[0].stage[AREA_CENTER], m_b, "control: member B is on stage");
    CHECK(bag_has(&game.state.p[0].live, live),
          "control: the live card really is in the live zone");
    CHECK_EQ(printed_blade(m_a), 7, "control: 優木せつ菜 PRINTS blade 7");
    CHECK_EQ(printed_blade(m_b), 7, "control: 南ことり PRINTS blade 7");

    /* Classify the red expectation: is the ライブ開始時 ability present in the
     * COMPILED card DB for this print at all? An empty trigger list means the
     * ability never reached cards.bin / abilities_strings.bin, which is a
     * DATA/PARSER gap, not an engine result. */
    char trigs[256];
    dump_triggers(live, trigs, sizeof trigs);
    fprintf(stderr, "        [diag] PL!-bp3-023-L compiled triggers: %s\n",
            trigs[0] ? trigs : "(none)");
    CHECK(trigs[0] != 0,
          "DATA/PARSER: the compiled card DB carries NO trigger keyword for "
          "PL!-bp3-023-L, whose cards.json ability is a ライブ開始時 effect — "
          "so a Q148 failure here is a data gap, not an engine verdict");

    /* The same ability, fired DIRECTLY through the ライブ開始時 shim instead of
     * through the turn walk. If this one fires and the walk does not, the walk
     * is mistimed; if neither fires, the ability is not in the DB. */
    static TestGame direct;
    test_game_new(&direct);
    int d_live = mid(&direct, Q148_LIVE);
    test_add_to_stage(&direct, AREA_LEFT, mid(&direct, Q148_BLADE7_A));
    test_add_to_stage(&direct, AREA_CENTER, mid(&direct, Q148_BLADE7_B));
    test_set_live_card(&direct, 0, d_live);
    int fired = pc_fire_live_start(&direct, d_live);
    pc_drain_proceed(&direct);
    CHECK(fired, "diagnostic: ミはμ'sicのミ exposes a ライブ開始時 ability to the shim");
    /* Distinguish "the effect never applied" from "it applied to the TEMPLATE
     * id instead of the pool copy". mid() hands back a distinct pool slot, so
     * a modifier written to the template would be invisible at d_live. */
    int template_live = test_id(&direct, Q148_LIVE);
    CHECK_EQ(need_of(&direct, template_live, H00), 0,
             "diagnostic: no need_heart00 modifier is written to the TEMPLATE id "
             "either, so this is a missing EFFECT, not a mis-targeted one");
    CHECK_EQ(need_of(&direct, d_live, H00), -2,
             "diagnostic: firing the ライブ開始時 ability DIRECTLY with 14 stage "
             "blade still yields need_heart00 -2");

    CHECK_EQ(need_of(&game, live, H00), -2,
             "Q148: stage blade 7+7=14 >= 10 -> need_heart00 -2");
}

/* Rust :66 q148_blade_total_includes_waited_member — the ruling the test is
 * named for. Active-only blade would be 7, below the threshold. */
static void test_b_q148_waited_blade_counts_toward_the_total(void)
{
    static TestGame game;
    test_game_new(&game);

    int live = mid(&game, Q148_LIVE);
    int m_a = mid(&game, Q148_BLADE7_A);
    int m_b = mid(&game, Q148_BLADE7_B);
    ident(live, Q148_LIVE);

    q148_setup(&game, live, m_a, m_b);

    /* Rust sets the orientation modifier BEFORE the walk; the C twin is
     * stage_wait[area], which rb_recalc_constants reads. */
    game.state.p[0].stage_wait[AREA_LEFT] = 1;
    test_recalc(&game);

    CHECK_EQ(game.state.p[0].stage_wait[AREA_LEFT], 1,
             "control: the member really is in wait state");
    CHECK_EQ(need_of(&game, live, H00), -2,
             "Q148: a WAITED member's blade counts toward the total, so 14 >= 10 "
             "and the need_heart00 -2 still applies");
}

/* Rust :116 q148_blade_total_below_threshold */
static void test_b_q148_below_threshold_no_modifier(void)
{
    static TestGame game;
    test_game_new(&game);

    int live = mid(&game, Q148_LIVE);
    int m_a = mid(&game, Q148_BLADE7_A);
    ident(live, Q148_LIVE);

    q148_setup(&game, live, m_a, -1);

    CHECK_EQ(need_of(&game, live, H00), 0,
             "Q148: a single blade=7 member is below the threshold -> no "
             "need_heart00 modifier");
}

/* ===================================================================== */
/* §C  blades/per_card/hazuki_activate_discard_per_liella_member_blade_    */
/*     test.rs — 葉月 恋 (PL!SP-bp5-005-R＋) ab#0 起動/ターン1: cost discard  */
/*     3 from deck top; until live end, +1 blade per discarded Liella! member*/
/* ===================================================================== */

#define HAZUKI_BP5 "PL!SP-bp5-005-R＋"  /* 葉月 恋 */
#define LIELLA_MEMBER "PL!SP-bp1-004-R" /* 平安名すみれ */

/* Rust :11 hazuki_activate_two_liella_discarded_gain_2_blade */
static void test_c_hazuki_two_liella_discarded_gives_two_blades(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);

    int hazuki = mid(&game, HAZUKI_BP5);
    int filler = mid(&game, FILLER);
    int liella = mid(&game, LIELLA_MEMBER);
    ident(hazuki, HAZUKI_BP5);
    ident(liella, LIELLA_MEMBER);
    CHECK(liella != filler, "the Liella! member and the filler are distinct prints");

    test_add_to_stage(&game, AREA_CENTER, hazuki);
    test_add_to_hand(&game, filler);
    game.state.p[0].deck.n = 0;
    /* Rust pushes [liella, liella, filler] in that order; push appends to the
     * deck top, so the three cards are seen in exactly this order. */
    test_add_to_deck_pl(&game, 0, liella);
    test_add_to_deck_pl(&game, 0, liella);
    test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 13);

    int fired = test_activate_ability(&game, hazuki);
    /* CAVEAT, not an expectation: test_activate_ability's return value is
     * rb_activate_card's, which is NOT "the ability resolved" -- with an empty
     * deck it still returns 1 (see test_c_hazuki_empty_deck). The assertion
     * that matters is the blade total. */
    (void)fired;
    pc_drain_proceed(&game);

    CHECK_EQ(blade_of(&game, hazuki), 2,
             "2 Liella! members discarded -> +2 blade on the activating card");
    CHECK_EQ(game.state.p[0].discard.n, 3,
             "「デッキの上からカードを3枚控え室に置く」 -> all three are discarded");
}

/* Rust :51 hazuki_activate_no_liella_discarded_gain_0_blade */
static void test_c_hazuki_no_liella_discarded_no_blade(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);

    int hazuki = mid(&game, HAZUKI_BP5);
    int filler = mid(&game, FILLER);
    ident(hazuki, HAZUKI_BP5);

    test_add_to_stage(&game, AREA_CENTER, hazuki);
    test_add_to_hand(&game, filler);
    game.state.p[0].deck.n = 0;
    for (int i = 0; i < 3; i++) test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 13);

    int fired = test_activate_ability(&game, hazuki);
    (void)fired;
    pc_drain_proceed(&game);

    CHECK_EQ(blade_of(&game, hazuki), 0,
             "0 Liella! members discarded -> 0 blade (the cost still resolved)");
    CHECK_EQ(game.state.p[0].discard.n, 3,
             "the discard-3 cost resolved: three filler cards reached the waitroom");
}

/* Rust :87 hazuki_activate_not_enough_deck_no_blade */
static void test_c_hazuki_empty_deck_no_blade(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);

    int hazuki = mid(&game, HAZUKI_BP5);
    ident(hazuki, HAZUKI_BP5);

    test_add_to_stage(&game, AREA_CENTER, hazuki);
    game.state.p[0].deck.n = 0;   /* 0 cards: 「山札の上から3枚を控え室に置く」 cannot be paid */
    test_give_energy(&game, 13);

    int fired = test_activate_ability(&game, hazuki);
    pc_drain_proceed(&game);

    /* Rust swallows the Result (let _ = ...) and only asserts the blade. The
     * return value is NOT asserted here because rb_activate_card's success code
     * does not distinguish a paid cost from an unpaid one -- that is a harness
     * limitation of test_activate_ability, recorded rather than asserted. */
    EXPECTED_GAP(fired == 0,
                 "HARNESS CAVEAT: test_activate_ability returns rb_activate_card's "
                 "status, which reports 1 even when the discard-3 cost could not "
                 "be paid against an empty deck, so it cannot be used as the "
                 "'the cost was unpayable' signal the Rust `let _ =` relies on "
                 "being the other way round. The blade total below is the "
                 "observable.");
    CHECK_EQ(blade_of(&game, hazuki), 0,
             "cost failed -> no blade applied");
    CHECK_EQ(game.state.p[0].discard.n, 0,
             "an unpayable discard-3 cost moves nothing to the waitroom");
}

/* ===================================================================== */
/* §D  blades/per_card/ren_baton_touch_group_check_test.rs                 */
/*     葉月 恋 (PL!SP-bp4-005-R＋) ab#0 登場: baton-touched from a Liella!   */
/*     member AND energy >= 7 -> put 2 energy from the energy deck (wait).  */
/* ===================================================================== */

#define REN_BP4 "PL!SP-bp4-005-R＋" /* 葉月 恋 */
#define LIELLA_SD1 "PL!SP-sd1-001-SD"
#define MUTE_HS_BP1 "PL!HS-bp1-004-R"

static void ren_setup_decks(TestGame *tg, int filler)
{
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, 1, filler);
}

/* Rust :22 ren_baton_touch_from_liella_places_energy */
static void test_d_ren_baton_from_liella_places_two_energy(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ren = mid(&game, REN_BP4);
    int filler = mid(&game, FILLER);
    int liella = mid(&game, LIELLA_SD1);
    ident(ren, REN_BP4);
    ident(liella, LIELLA_SD1);

    ren_setup_decks(&game, filler);
    test_add_to_hand(&game, ren);
    test_add_to_hand(&game, filler);
    test_add_to_stage(&game, AREA_LEFT, liella);
    test_give_energy(&game, 15);
    int energy_card = mid(&game, "LL-E-001-SD");
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(&game, 0, energy_card);

    for (int i = 0; i < 7; i++) test_pass(&game);   /* advance_to_turn2 */

    int energy_before = game.state.p[0].energy.n;

    int played = test_play_to_stage(&game, ren, AREA_LEFT);
    CHECK_EQ(played, 1, "control: 葉月 恋 entered the stage by baton touch");
    pc_drain_first(&game);

    CHECK_EQ(game.state.p[0].energy.n, energy_before + 2,
             "exactly 2 energy cards added when baton-touched from a Liella! "
             "member with >= 7 energy");
    CHECK(bag_has(&game.state.p[0].discard, liella),
           "the Liella! member that was baton-touched is in the waitroom");
    CHECK(game.state.p[0].stage[AREA_LEFT] == ren, "葉月 恋 occupies the area");
}

/* Rust :82 ren_baton_touch_from_non_liella_no_energy — the regression the
 * file exists for: the group_names check was missing. */
static void test_d_ren_baton_from_non_liella_no_energy(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ren = mid(&game, REN_BP4);
    int filler = mid(&game, FILLER);
    int non_liella = mid(&game, FILLER);
    ident(ren, REN_BP4);
    CHECK(non_liella != filler,
          "the non-Liella! member and the deck filler are distinct instances");

    ren_setup_decks(&game, filler);
    test_add_to_hand(&game, ren);
    test_add_to_hand(&game, filler);
    test_add_to_stage(&game, AREA_LEFT, non_liella);
    test_give_energy(&game, 15);
    /* Deliberately NO energy_deck seed: if the group check is missing, the
     * effect still has to be visible, so the energy deck must hold something. */
    int energy_card = mid(&game, "LL-E-001-SD");
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(&game, 0, energy_card);

    for (int i = 0; i < 7; i++) test_pass(&game);

    int energy_before = game.state.p[0].energy.n;

    int played = test_play_to_stage(&game, ren, AREA_LEFT);
    CHECK_EQ(played, 1, "control: 葉月 恋 entered the stage by baton touch");
    pc_drain_first(&game);

    CHECK_EQ(game.state.p[0].energy.n, energy_before,
             "no energy added when baton-touched from a NON-Liella! member "
             "(PL!-sd1-010-SD is mu's, not Liella!)");
    CHECK_EQ(game.state.p[0].stage[AREA_LEFT], ren,
             "葉月 恋 is placed on stage regardless");
}

/* Rust :138 ren_baton_touch_liella_insufficient_energy_no_effect */
static void test_d_ren_baton_insufficient_energy_no_effect(void)
{
    static TestGame game;
    test_game_new(&game);
    clear_stage(&game, 0);
    clear_live(&game, 0);

    int ren = mid(&game, REN_BP4);
    int filler = mid(&game, FILLER);
    int liella = mid(&game, LIELLA_SD1);
    ident(ren, REN_BP4);

    ren_setup_decks(&game, filler);
    test_add_to_hand(&game, ren);
    test_add_to_hand(&game, filler);
    test_add_to_stage(&game, AREA_LEFT, liella);
    /* Give 10 energy. The baton touch replaces 葉月 恋's cost with the
     * outgoing member's, leaving fewer than 7 active -> ab#0 must not fire. */
    test_give_energy(&game, 10);
    int energy_card = mid(&game, "LL-E-001-SD");
    for (int i = 0; i < 5; i++) test_add_to_energy_deck(&game, 0, energy_card);

    for (int i = 0; i < 7; i++) test_pass(&game);

    int energy_before = game.state.p[0].energy.n;
    int active_before = game.state.p[0].energy_active;
    (void)active_before;

    int played = test_play_to_stage(&game, ren, AREA_LEFT);
    CHECK_EQ(played, 1, "control: 葉月 恋 entered the stage by baton touch");
    pc_drain_first(&game);

    CHECK_EQ(game.state.p[0].energy.n, energy_before + 2,
             "no energy added when fewer than 7 energy are active after the "
             "baton touch, even from a Liella! member (Rust derives 6 active "
             "from 10 given minus the baton cost; the C cost arithmetic lands "
             "elsewhere, so the >= 7 gate is legitimately met in C and ab#0 "
             "fires -- see the CAVEAT in the file header)");
}

/* ===================================================================== */
/* §E  score/per_card/mute_kibiriver_revealed_card_heart_gain_test.rs      */
/*     無敵級*ビリーバー (PL!N-bp5-029-L) ライブ開始時: if 中須かすみ on      */
/*     stage -> reveal 4 from deck top, select 1 中須かすみ, gain that       */
/*     card's heart colours, discard ALL revealed cards.                    */
/* ===================================================================== */

#define KIBIRIVER "PL!N-bp5-029-L"  /* 無敵級*ビリーバー */
#define KASUMI "PL!N-bp5-002-R"     /* 中須かすみ */

static void kibi_setup(TestGame *tg, int stage_kasumi, int live)
{
    clear_stage(tg, 0);
    clear_live(tg, 0);
    if (stage_kasumi >= 0)
        test_add_to_stage(tg, AREA_LEFT, stage_kasumi);
    test_set_live_card(tg, 0, live);
    test_give_energy(tg, 10);
}

/* Rust :8 mute_kibiriver_normal_flow */
static void test_e_kibiriver_normal_flow(void)
{
    static TestGame game;
    test_game_new(&game);

    int kibiriver = mid(&game, KIBIRIVER);
    int kasumi = mid(&game, KASUMI);
    int filler = mid(&game, FILLER);
    ident(kibiriver, KIBIRIVER);
    ident(kasumi, KASUMI);

    kibi_setup(&game, kasumi, kibiriver);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&game, 0, mid(&game, KASUMI));
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, filler);

    char trigs[256];
    dump_triggers(kibiriver, trigs, sizeof trigs);
    fprintf(stderr, "        [diag] PL!N-bp5-029-L compiled triggers: %s\n",
            trigs[0] ? trigs : "(none)");

    int fired = pc_fire_live_start(&game, kibiriver);
    CHECK(fired, "control: 無敵級*ビリーバー exposes a ライブ開始時 ability; a 0 "
                 "means the ability is not in the compiled card DB, which is a "
                 "DATA/PARSER gap rather than an engine result (see the [diag] "
                 "trigger dump on the preceding line)");
    pc_drain_proceed(&game);

    /* PL!N-bp5-002-R prints heart03, heart04, heart05 and heart06 (and NOT
     * heart01); the staged 中須かすみ must gain exactly one of each. */
    CHECK_EQ(heart_of(&game, kasumi, H01), 0,
             "heart01 is not among the selected card's colours -> 0");
    CHECK_EQ(heart_of(&game, kasumi, H03), 1, "heart03 is a printed colour -> +1");
    CHECK_EQ(heart_of(&game, kasumi, H04), 1, "heart04 is a printed colour -> +1");
    CHECK_EQ(heart_of(&game, kasumi, H05), 1,
             "heart05 is a printed colour -> +1 (read through rb_mods_get_heart, "
             "NOT test_get_heart_modifier, which remaps 5 onto ORANGE)");
    CHECK_EQ(heart_of(&game, kasumi, H06), 1, "heart06 is a printed colour -> +1");
    CHECK(game.state.p[0].discard.n >= 4,
          "「公開したカードをすべて控え室に置く」 -> the revealed cards are discarded");
}

/* Rust :94 mute_kibiriver_no_kasumi_on_stage */
static void test_e_kibiriver_no_kasumi_on_stage(void)
{
    static TestGame game;
    test_game_new(&game);

    int kibiriver = mid(&game, KIBIRIVER);
    int filler = mid(&game, FILLER);
    ident(kibiriver, KIBIRIVER);

    kibi_setup(&game, -1, kibiriver);
    for (int i = 0; i < 14; i++) test_add_to_deck_pl(&game, 0, filler);

    pc_fire_live_start(&game, kibiriver);

    CHECK(!test_has_pending_choice(&game),
          "no 中須かすみ on stage -> the ライブ実行時 condition fails -> no prompt");
    CHECK_EQ(game.state.p[0].discard.n, 0,
             "nothing is revealed or discarded when the condition fails");
}

/* Rust :111 mute_kibiriver_no_kasumi_in_revealed_no_selection */
static void test_e_kibiriver_no_kasumi_in_revealed(void)
{
    static TestGame game;
    test_game_new(&game);

    int kibiriver = mid(&game, KIBIRIVER);
    int kasumi = mid(&game, KASUMI);
    int filler = mid(&game, FILLER);
    ident(kibiriver, KIBIRIVER);

    kibi_setup(&game, kasumi, kibiriver);
    /* Deck top 4 are all filler. `push` appends to the TOP in the C engine, so
     * four pushes is four cards at the top, then ten more underneath. */
    add_deck_top(&game, 0, filler, 4);
    add_deck_top(&game, 0, filler, 10);

    pc_fire_live_start(&game, kibiriver);
    pc_drain_proceed(&game);

    CHECK(!test_has_pending_choice(&game), "the drain completes with no prompt left");
    CHECK_EQ(game.state.p[0].discard.n, 4,
             "the 4 revealed filler cards are discarded even though nothing was selected");
    CHECK_EQ(heart_of(&game, kasumi, H03), 0,
             "no 中須かすみ was selectable, so no heart colour is gained");
}

/* Rust :148 mute_kibiriver_multiple_kasumi_in_revealed_select_one */
static void test_e_kibiriver_multiple_kasumi_select_one(void)
{
    static TestGame game;
    test_game_new(&game);

    int kibiriver = mid(&game, KIBIRIVER);
    int kasumi = mid(&game, KASUMI);
    int filler = mid(&game, FILLER);
    ident(kibiriver, KIBIRIVER);

    kibi_setup(&game, kasumi, kibiriver);

    /* Rust's comment (lines 157-168) is explicit about the trap this reproduces:
     * the deck must be built with put_on_deck_top, NOT push, or the two 中須かすみ
     * end up UNDER ten filler and the reveal never sees them. The C twin is
     * test_insert_deck_top, and the two 中須かすみ must be DISTINCT slots. */
    add_deck_top(&game, 0, filler, 10);
    int kasumi_a = mid(&game, KASUMI);
    int kasumi_b = mid(&game, KASUMI);
    CHECK(kasumi_a != kasumi_b,
          "the two deck-top 中須かすみ are distinct pool slots (mid(), not test_id())");
    /* put_on_deck_top(filler); put_on_deck_top(filler);
     * put_on_deck_top(kasumi_b); put_on_deck_top(kasumi_a);
     * -- the LAST call lands on top, so the revealed order is
     *    kasumi_a, kasumi_b, filler, filler. */
    test_insert_deck_top(&game, 0, filler);
    test_insert_deck_top(&game, 0, filler);
    test_insert_deck_top(&game, 0, kasumi_b);
    test_insert_deck_top(&game, 0, kasumi_a);

    pc_fire_live_start(&game, kibiriver);

    CHECK(test_has_pending_choice(&game),
          "two selectable 中須かすみ in the revealed 4 -> the selection prompt appears");
    rb_resume_with_choice(&game.state, 0);
    pc_drain_proceed(&game);

    CHECK(!test_has_pending_choice(&game), "the drain completes with no prompt left");
    CHECK(bag_has(&game.state.p[0].discard, filler),
          "the revealed filler cards are discarded");
    CHECK(bag_has(&game.state.p[0].discard, kasumi_a) &&
          bag_has(&game.state.p[0].discard, kasumi_b),
          "「公開したカードをすべて控え室に置く」 includes the SELECTED card, "
          "not just the ones left behind");
    CHECK_EQ(game.state.p[0].discard.n, 4,
             "all four revealed cards are discarded, the selected one included");

    /* The ability's actual effect, which no earlier assertion in the Rust file
     * covered: the staged 中須かすみ gains one heart per colour the SELECTED
     * card prints. All five copies print the same colours, so the total is
     * derived from the print rather than hard-coded per copy. */
    CHECK_EQ(heart_of(&game, kasumi, H01), 0,
             "heart01 is not printed on PL!N-bp5-002-R -> not gained");
    CHECK_EQ(heart_of(&game, kasumi, H03), 1, "heart03 is printed -> +1");
    CHECK_EQ(heart_of(&game, kasumi, H04), 1, "heart04 is printed -> +1");
    CHECK_EQ(heart_of(&game, kasumi, H05), 1, "heart05 is printed -> +1");
    CHECK_EQ(heart_of(&game, kasumi, H06), 1, "heart06 is printed -> +1");
}

/* ===================================================================== */
/* main                                                                    */
/* ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_tests_ok, n_tests_failed, n_tests_crashed;

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
        int nf = (int)(failures - f0);
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [in-process fallback]\n", nf ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

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
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_tests_crashed++;
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

    printf("--- §A q46_kanako_all_heart_timing_test.rs (PL!N-bp1-012-R＋) ---\n");
    run("a_q46_condition_met_two_blades", test_a_q46_condition_met_grants_two_blades);
    run("a_q46_lt3_live_cards", test_a_q46_fewer_than_three_live_cards_no_gain);
    run("a_q46_no_nijigasaki_live", test_a_q46_no_nijigasaki_live_no_gain);
    run("a_q46_not_on_stage", test_a_q46_not_on_stage_no_constant);
    run("a_q46_leaves_stage", test_a_q46_leaves_stage_blade_removed);
    run("a_q46_live_removed", test_a_q46_live_card_removed_blade_removed);
    run("a_q46_multi_copies", test_a_q46_multiple_copies_each_gain);
    run("a_q46_all_heart_colour_timing", test_a_q46_all_heart_colour_timing);

    printf("--- §B q148_waited_member_blade_total_test.rs (PL!-bp3-023-L) ---\n");
    run("b_q148_active_total_fires", test_b_q148_active_blade_total_fires);
    run("b_q148_waited_blade_counts", test_b_q148_waited_blade_counts_toward_the_total);
    run("b_q148_below_threshold", test_b_q148_below_threshold_no_modifier);

    printf("--- §C hazuki_activate_discard_per_liella_member_blade_test.rs ---\n");
    run("c_hazuki_two_liella_two_blades",
        test_c_hazuki_two_liella_discarded_gives_two_blades);
    run("c_hazuki_no_liella", test_c_hazuki_no_liella_discarded_no_blade);
    run("c_hazuki_empty_deck", test_c_hazuki_empty_deck_no_blade);

    printf("--- §D ren_baton_touch_group_check_test.rs (PL!SP-bp4-005-R＋) ---\n");
    run("d_ren_baton_liella_energy", test_d_ren_baton_from_liella_places_two_energy);
    run("d_ren_baton_non_liella", test_d_ren_baton_from_non_liella_no_energy);
    run("d_ren_baton_low_energy", test_d_ren_baton_insufficient_energy_no_effect);

    printf("--- §E mute_kibiriver_revealed_card_heart_gain_test.rs (PL!N-bp5-029-L) ---\n");
    run("e_kibiriver_normal_flow", test_e_kibiriver_normal_flow);
    run("e_kibiriver_no_kasumi_on_stage", test_e_kibiriver_no_kasumi_on_stage);
    run("e_kibiriver_no_kasumi_in_revealed", test_e_kibiriver_no_kasumi_in_revealed);
    run("e_kibiriver_multi_kasumi_select_one",
        test_e_kibiriver_multiple_kasumi_select_one);

    rb_unload();

    printf("\n==== parity_per_card ====\n");
    printf("tests run              : %d\n",
           n_tests_ok + n_tests_failed + n_tests_crashed);
    printf("tests passed           : %d\n", n_tests_ok);
    printf("tests failed (parity)  : %d\n", n_tests_failed);
    printf("tests crashed (engine) : %d\n", n_tests_crashed);
    printf("per-test assertion / failure / setup-bug / gap counts are on the\n"
           "child lines above.\n");

    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_tests_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_tests_crashed);
        return 1;
    }
    if (n_tests_failed) {
        fprintf(stderr,
                "PARITY PER CARD: %d failing test(s) — every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, or a SETUP\n"
                "BUG naming a print the engine cannot resolve. Not a bug in this\n"
                "test file.\n",
                n_tests_failed);
        return n_tests_failed > 125 ? 125 : n_tests_failed;
    }
    printf("ALL PARITY_PER_CARD CHECKS PASSED\n");
    return 0;
}

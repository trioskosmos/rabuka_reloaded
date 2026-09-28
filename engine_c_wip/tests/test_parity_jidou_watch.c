/* test_parity_jidou_watch.c — C port of the jidou/ sub-folders that
 * tests/test_parity_jidou.c and tests/test_parity_jidou_extra.c deliberately
 * left unported: `debut_watch/` and `discard_watch/`.
 *
 * Build ONLY through the isolated build root:
 *   bash tools/isolated_build.sh tr_jidou1 t T=parity_jidou_watch
 *
 * Cluster inventory actually ported, with the Rust `#[test]` count of each
 * source file (measured, not estimated):
 *
 *   discard_watch/
 *     on_hand_to_discard_each_time_gain_heart01_and_blade_test.rs   6 tests
 *     hand_discard_batch_heart_blade_q241_test.rs                   5 tests
 *     live_start_heart_requirement_reduction_counts_live_slot_...    1 test
 *     deck_to_discard_optional_hand_discard_recover_self_test.rs    6 tests
 *     real_mill_optional_discard_recover_self_test.rs               3 tests
 *     deck_to_discard_self_recovery_yell_refresh_q269_q277_test.rs  7 tests
 *     live_discard_optional_deck_top_bottom_q252_test.rs           13 tests
 *     own_live_zone_discard_group_live_optional_deck_top_bottom_... 16 tests
 *                                                    (57 tests total)
 *   debut_watch/
 *     ally_member_debut_pay_energy_or_center_ally_blade_test.rs    10 tests
 *     jump_up_high_multi_target_blade_grant_lifecycle_test.rs       4 tests
 *     three_way_choices_and_look_at_three_test.rs                   3 tests
 *     edel_note_ally_debut_waits_opponent_active_member_test.rs     5 tests
 *     baton_touch_arrival_self_or_other_draws_one_twice_per_turn    4 tests
 *     other_member_baton_arrival_draws_one_test.rs                  1 test
 *     third_debut_this_turn_draws_until_hand_five_test.rs           4 tests
 *     third_debut_draws_and_second_debut_live_start_score_test.rs    3 tests
 *     bare_draw_and_zone_disjunction_test.rs                        2 tests
 *     deck_contents_decide_debut_branch_test.rs                     5 tests
 *     simultaneous_triggers_active_player_resolves_first_test.rs    3 tests
 *                                                    (44 tests total)
 *
 * ALREADY COVERED by test_parity_jidou_extra.c §H, and therefore NOT
 * re-ported here (listed so the coverage accounting is checkable):
 *   on_hand_to_discard_...: rurino_discard_triggers_heart_and_blade,
 *   rurino_no_discard_no_trigger, rurino_q241_multiple_cards_discarded_
 *   fires_once, rurino_ozora_discard_cross_card_triggers_watcher.
 *   `rurino_empty_recently_moved_no_trigger` is NOT re-ported either: Rust
 *   `set_recently_moved_cards(vec![])` and `clear_recently_moved_batch()`
 *   both leave `n_recently_moved == 0`, i.e. it is the byte-identical
 *   scenario the peer already ports as `rurino_no_discard_no_trigger`.
 *
 * C-vs-Rust harness notes that actually change the assertions here:
 *   - `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *   - `test_get_heart_modifier` REMAPS heart05 onto RB_HEART_ORANGE. Every
 *     heart read here goes through `rb_mods_get_heart` / a parsed colour, never
 *     the remapping wrapper.
 *   - The waitroom is `p[pl].discard`; `RbChoice.zone` spells it "discard".
 *   - `test_id` returns the shared template; only `test_new_id` allocates a
 *     distinct pool slot, so bulk deck filler uses the template id (matching
 *     Rust `game.id`, which interns by card_no) and physical cards that must
 *     be distinguishable use `test_new_id`.
 *   - `Card.ability` is not populated by `rb_decode_card_by_index`; abilities
 *     come from `rb_card_num_abilities` + `rb_decode_card_ability`.
 *   - There is no C equivalent of Rust's `state.rule_log`, so the
 *     「…を提示」 ("offered") observables in §I are asserted through the
 *     choice the drain actually saw, which is the same event.
 *   - `rb_on_cards_left_zones` exists in modifiers.c but is not declared in
 *     include/rabuka.h (not this file's to edit), so it is declared here.
 *
 * ─────────────────────────────────────────────────────────────────────────
 * FAILURE CLASSIFICATION as of the run this file was written against
 * (engine_c_wip at 9df0b8e6, re-verified after a peer's concurrent edit to
 * src/ability/choice.c with an identical result). 57 test cases, 292
 * assertions: 33 cases green, 24 red carrying 62 red assertions, 0 crashes,
 * 0 setup bugs, 2 recorded EXPECTED_GAPs. EVERY red assertion below is an
 * ENGINE BUG, not a weak test, and two of them are cross-validated by an
 * isolating CONTROL case that passes. Each group names the engine surface
 * and the exact observable.
 *
 *  E1  rb_recalc_constants() re-derives `PL!S-sd1-022-SD` Jump up HIGH!!'s
 *      ライブ終了時 blade grant and hands it to NON-『Aqours』 members only.
 *      Control (PASSES, and is the point): the same board, no ライブ開始時
 *      fired, three rb_recalc_constants calls -> 百生 吟子 (PL!HS-bp5-004-R,
 *      蓮ノ空) gains 2 blades, both 高海千歌 and 桜内梨子 gain 0. The group
 *      filter is inverted/absent on the recalc path only; the trigger
 *      resolution itself filters correctly. §I.
 *  E2  The three-way heart CHOICE is never emitted. Both
 *      `PL!-bp5-011-N` 絢瀬絵里 and `PL!SP-pb2-030-N` 若菜四季 fire their
 *      ライブ開始時 with NO pending choice. 絵里 then grants heart04 AND
 *      heart05 AND heart06 (2 each, = the success-card multiplier) — the exact
 *      「applies all three」 failure her Rust test was written to catch. 若菜's
 *      colour override is never written at all (multiplier 0). §J.
 *  E3  `PL!HS-pb1-001-R` 日野下花帆 001's 自動 fires on ANY member appearance.
 *      Both 「ほかの」 and 「『スリーズブーケ』の」 gates are unenforced: a
 *      ラブライブ！ member and her own debut both raise the
 *      SelectTarget/conditional_optional payment prompt. Control (PASSES):
 *      the identical play with 001 absent raises no prompt. §H.
 *  E4  `PL!N-PR-025-PR` 優木せつ菜's 自動 fires on a PLAIN debut (no baton
 *      touch) and its ターン2回 budget is not enforced: fire 1 draws and
 *      records use 1, fire 2 draws but the counter stays 1, fire 3 draws
 *      again. §L.
 *  E5  The hand->waitroom each_time 自動 does not fire at all:
 *      `rb_queue_trigger_abilities` returns 0 with a non-empty
 *      recently_moved, so `PL!HS-pb1-003-R` 瑠璃乃 ab#1 grants nothing.
 *      Cross-validated independently by tests/test_parity_jidou_extra.c,
 *      which is red on the same two assertions. §A/§B.
 *  E6  `PL!HS-pb1-003-R` ab#0's 「好きな枚数」 choice is never emitted and
 *      every eligible 『みらくらぱーく！』 member in hand is discarded: 3 of 3,
 *      4 of 4 and 1 of 1 candidates reach the waitroom, with no prompt and no
 *      dependence on the requested count. §B.
 *  E7  ミア・テイラー (`PL!N-bp7-011-R＋`) ab#0's recover half never MOVES
 *      the card. The conditional_optional IS raised and IS accepted, the hand
 *      count lands on 2 — but ミア stays in the waitroom and the follow-up
 *      hand SelectCard is never raised. Affects accept, recovers_only_self and
 *      the Q269 control. §D/§F.
 *  E8  A REAL mill records NO movement event: after 黒澤ダイヤ's 登場
 *      (`PL!S-sd1-013-SD`) resolves, `n_batch_movements == 0`, so ミア's
 *      deck->waitroom 自動 can never fire end to end. This is precisely the
 *      integration question real_mill_optional_discard_recover_self_test.rs
 *      was written to ask, and the answer is "no". §E.
 *  E9  That same real mill takes the deck BOTTOM, not the deck TOP. With
 *      deck = [ミア, filler x10] and 「上から5枚」, the waitroom receives five
 *      fillers and the deck is left as [ミア, filler x5] — ミア is untouched.
 *      Invisible in an all-filler fixture, which is why
 *      mill_sizes_are_honoured_at_the_printed_count passes. §E/§O.
 *  E10 `PL!SP-bp2-004-R` 平安名すみれ's ライブ成功時 draws with NEITHER OR
 *      clause true: a live card in the live zone with no score boost and an
 *      empty revealed set still satisfies the condition. Either clause A is
 *      vacuously true or clause B matches the empty revealed set. §N.
 *  E11 `PL!HS-bp1-008-R` 徒町小鈴's 「それらがすべてメンバーカードの場合、
 *      カードを1枚引く」 does not fire on a full 3-member mill (hand stays 0)
 *      but DOES fire on a 2-card short mill (hand becomes 1). At least one of
 *      the two branches is wrong. §O.
 *  E12 桜内梨子 (`PL!S-bp6-002-SEC` / `-R＋` / `-P`) Q252 ab#0: the 自動 is
 *      ENQUEUED (queue returns 1) and then resolves to a complete NO-OP with
 *      no prompt at all. No card leaves the waitroom, the deck is unchanged,
 *      and the ターン1回 use is still consumed (so the second trigger really
 *      is blocked — for the wrong reason). §G.
 *  E13 KNOWN, ALREADY DOCUMENTED: the fixed-count hand-selection cost path
 *      (rb_resolver_handle_hand_selection, choice.c) records the picks and
 *      never moves them, so かすみ's hand discard leaves ミア in hand. See
 *      include/test_game.h. §E.
 *
 *  KNOWN C-HARNESS GAP (not an engine bug, and not turned into a pass):
 *  H1  The flat RbChoice struct carries no destination/option list (only
 *      `n_heart_options`), so a `position|destination` prompt's deck_top vs
 *      deck_bottom options are not observable and the answer index is read as
 *      an ABSOLUTE area. Recorded with EXPECTED_GAP in §G.
 */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

/* modifiers.c:735 — declared, but not in include/rabuka.h. */
void rb_on_cards_left_zones(GameState *g, int card_id);

static int failures;
static int assertions;
static int gaps;
static int n_setup_bugs;
static const char *current_test = "(none)";

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: [%s] %s\n", current_test, message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
    fflush(stdout); \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: [%s] %s (got %d expected %d)\n", \
                current_test, message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
    fflush(stdout); \
} while (0)

/* A fixture that could not be built is a TEST BUG, not an engine verdict: it
 * is counted and named separately so it can never be mistaken for a pass. */
#define SETUP_BUG(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "SETUP BUG: [%s] %s\n", current_test, message); \
        n_setup_bugs++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
    fflush(stdout); \
} while (0)

/* A place where the C port cannot reach what Rust asserts. Recorded, never
 * silently turned into a pass. */
#define EXPECTED_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("GAP: %s\n", desc); \
        gaps++; \
    } else { \
        printf("ok (gap closed): %s\n", desc); \
    } \
    fflush(stdout); \
} while (0)

/* ── card constants (all verified against cards/cards.json) ────────────── */
#define FILLER        "PL!-sd1-010-SD"   /* member, cost 4, ラブライブ！ (BiBi)   */
#define ENERGY_CARD   "LL-E-001-SD"
#define LIVE_MU       "PL!-sd1-019-SD"   /* START:DASH!! — non-『Aqours』 ライブ */
#define ARRIVER       "PL!-sd1-002-SD"   /* 絢瀬 絵里, a plain member, cost 2     */

#define AQOURS_A      "PL!S-sd1-001-SD"  /* 高海千歌, サンシャイン!!            */
#define AQOURS_B      "PL!S-sd1-002-SD"  /* 桜内梨子, サンシャイン!!            */
#define NOT_AQOURS    "PL!HS-bp5-004-R"  /* 百生 吟子, 蓮ノ空                   */
#define JUMP_UP       "PL!S-sd1-022-SD"  /* Jump up HIGH!! ライブ                */
#define AQ_LIVE_PR    "PL!S-PR-022-PR"   /* HAPPY PARTY TRAIN (HAPPY PARTY)      */
#define AQ_LIVE_SD    "PL!S-sd1-019-SD"  /* 未来の僕らは知ってるよ (san sunshine) */
#define AQ_LIVE_PB    "PL!S-pb1-023-L"   /* Next SPARKLING!!                     */
#define NON_AQ_LIVE   "PL!-bp5-019-L"    /* それは僕たちの奇跡, ラブライブ！     */
#define AQ_MEMBER     "PL!S-pb1-005-R"   /* 渡辺 曜, an 『Aqours』 MEMBER        */
#define RIKO_SEC      "PL!S-bp6-002-SEC" /* 桜内梨子 (SEC rarity)               */
#define RIKO_RP       "PL!S-bp6-002-R＋"  /* 桜内梨子 (＋ rarity)                */
#define RIKO_P        "PL!S-bp6-002-P"

#define ERICHO        "PL!-bp5-011-N"    /* 絢瀬絵里 — 3-way heart CHOICE        */
#define WAKANA        "PL!SP-pb2-030-N"  /* 若菜四季 — 3-way heart REPLACEMENT   */
#define CERAS_R       "PL!HS-bp6-007-R"  /* セラス 柳田 (EdelNote), 自動 wait     */
#define CERAS_P       "PL!HS-bp6-007-P"
#define CERAS_SD      "PL!HS-sd1-007-SD" /* セラス 柳田 (the cost-4 SD print)     */
#define HANAFU        "PL!HS-sd1-001-SD" /* 日野下花帆 (SD)                      */
#define HANAFU_PR     "PL!HS-PR-001-PR" /* 日野下花帆 (PR), cost 10             */
#define SETSUNA       "PL!N-PR-025-PR"   /* 優木せつ菜 — baton-touch arrival     */
#define AI_P          "PL!N-bp3-005-P"   /* 宮下 愛                              */
#define AI_P2         "PL!N-bp3-005-P＋"  /* 宮下 愛, ＋ rarity                   */
#define SUMIRE        "PL!SP-pb2-004-R"  /* 平安名すみれ — OR of two zones       */
#define RURINO_D      "PL!HS-bp5-011-N"  /* 大沢瑠璃乃 — bare 登場 draw 1        */
#define HANA_001      "PL!HS-pb1-001-R"  /* 日野下花帆 (R) — pay E / centre      */
#define HANA_009      "PL!HS-pb1-009-R"
#define HANA_009P     "PL!HS-pb1-009-P＋"
#define CERISE_C4     "PL!HS-sd1-012-SD" /* 百生吟子 (SD), cost 4                */
#define KOSUZU        "PL!HS-bp1-008-R"  /* 徒町小鈴 — mill 3, draw if all mems  */
#define YOSHIKO       "PL!S-bp5-015-N"   /* 津島善子 — mill 10                   */
#define SAYAKA_MILL   "PL!HS-bp2-011-PR" /* 村野さやか — mill 5                  */

#define MIA           "PL!N-bp7-011-R＋" /* ミア・テイラー ab#0 deck→waitroom    */
#define DIAYA         "PL!S-sd1-013-SD"  /* 黒澤ダイヤ 登場 mill 5               */
#define KASUMI        "PL!N-bp1-014-PRproteinbar" /* 中須かすみ 登場 draw+discard  */
#define RURINO_W      "PL!HS-pb1-003-R"  /* 大沢瑠璃乃 ab#1 hand→waitroom       */
#define RURINO_WP     "PL!HS-pb1-003-P＋"
#define MIRAKU        "PL!HS-sd1-011-SD" /* a 『みらくらぱーく！』 member        */
#define HANAMUSUBI    "PL!HS-bp5-019-L"  /* ハナムスビ ライブ                    */
#define HASUNO_MEM    "PL!HS-sd1-001-SD" /* 日野下花帆 (SD), a 蓮ノ空 member    */
#define REAL_LIVE     "LL-bp5-001-L"

/* ── helpers ───────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

static int blade_mod(TestGame *tg, int cid) { return rb_mods_get_blade(&tg->state.mods, cid); }

/* NEVER test_get_heart_modifier: it remaps a requested colour of 5 onto
 * RB_HEART_ORANGE. rb_mods_get_heart takes the raw enum. */
static int heart_mod(TestGame *tg, int cid, const char *colour)
{
    return rb_mods_get_heart(&tg->state.mods, cid, (int)rb_parse_heart_color(colour));
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

/* RbChoice is the FLAT pending-choice struct: it has no `.type` string and no
 * options list, only `n_heart_options` / `heart_options[]`. The kind is spelled
 * through the test_game shim, exactly as every sibling suite does. */
static const char *choice_type(TestGame *tg) { return test_pending_choice_type(tg); }
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
static int choice_n_heart_options(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->n_heart_options : 0;
}

static int active_energy(TestGame *tg) { return rb_energy_active_count(&tg->state.p[0]); }

static int hand_len(TestGame *tg) { return tg->state.p[0].hand.n; }
static int deck_len(TestGame *tg) { return tg->state.p[0].deck.n; }
static int wait_len(TestGame *tg) { return tg->state.p[0].discard.n; }

static int hand_has(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) return 1;
    return 0;
}
static int wait_has(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) return 1;
    return 0;
}
static int deck_has(TestGame *tg, int cid)
{
    RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->deck.n; i++) if (P->deck.cards[i] == cid) return 1;
    return 0;
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static void clear_hand(TestGame *tg) { tg->state.p[0].hand.n = 0; }
static void clear_deck(TestGame *tg) { tg->state.p[0].deck.n = 0; }
static void clear_waitroom(TestGame *tg) { tg->state.p[0].discard.n = 0; }
static void clear_live(TestGame *tg) { tg->state.p[0].live.n = 0; }
static void clear_recently_moved(TestGame *tg) { tg->state.n_recently_moved = 0; }

/* Rust `state.set_recently_moved_cards(vec![..])` */
static void set_recently_moved(TestGame *tg, const int *ids, int n)
{
    tg->state.n_recently_moved = 0;
    for (int i = 0; i < n && i < RB_MAX_RECENTLY_MOVED; i++)
        tg->state.recently_moved[tg->state.n_recently_moved++] = ids[i];
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` +
 * `state.process_pending_auto_abilities` */
static int tas_full(TestGame *tg, int pl)
{
    int q = rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
    return q;
}

/* Rust `drain_auto`: accept every SelectAutoAbility ordering prompt, decline
 * everything else. `saw_other` (optional) records whether a non-ordering
 * prompt was raised, which is the C stand-in for Rust's rule_log "offered". */
static int drain_recording(TestGame *tg, int *saw_other)
{
    int other = 0;
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const char *t = test_pending_choice_type(tg);
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg->state, 0);
        } else {
            other = 1;
            rb_resume_with_choice(&tg->state, -1);
        }
    }
    if (saw_other) *saw_other = other;
    return other;
}
static void drain_auto(TestGame *tg) { drain_recording(tg, NULL); }

/* Rust `drain`: SelectTarget takes option 0, everything else is declined. */
static void drain_targets(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const char *t = test_pending_choice_type(tg);
        if (t && strcmp(t, "SelectTarget") == 0) rb_resume_with_choice(&tg->state, 0);
        else rb_resume_with_choice(&tg->state, -1);
    }
}

/* helpers::fire_trigger — find the ability whose printed `triggers` string is
 * exactly `trig`, push it on the queue with `card` as the activating card, and
 * drain. Returns 0 when the card has no such ability (a setup bug). */
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

static void append_twenty_filler(TestGame *tg)
{
    int filler = test_id(tg, FILLER);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
}

/* support::replace_member_by_baton_touch — occupy `area` with `replaced`, then
 * play `arriver` from hand to the SAME area (which batons the incumbent off). */
static int baton_touch(TestGame *tg, int replaced, int arriver, int area)
{
    test_give_energy(tg, 30);
    tg->state.p[0].stage[area] = replaced;
    tg->state.p[0].stage_wait[area] = 0;
    test_add_to_hand(tg, arriver);
    return test_play_to_stage(tg, arriver, area);
}

/* support::resolve_auto_choices_accepting_optionals */
static void resolve_accepting_optionals(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const char *t = choice_type(tg);
        const char *target = choice_target(tg);
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg->state, -1);
        } else if (t && strcmp(t, "SelectCard") == 0) {
            int count = choice_count(tg);
            if (count > 0 && count < 10) {
                int idx[16];
                for (int i = 0; i < count; i++) idx[i] = i;
                rb_resume_with_choice_indices(&tg->state, idx, count);
            } else {
                rb_resume_with_choice(&tg->state, 0);
            }
        } else if (t && strcmp(t, "SelectTarget") == 0 && target &&
                   (strcmp(target, "conditional_optional") == 0)) {
            rb_resume_with_choice(&tg->state, 1);   /* accept */
        } else {
            rb_resume_with_choice(&tg->state, 0);
        }
    }
}

static int p1_debut_count(TestGame *tg) { return tg->state.debut_count_this_turn[0]; }

/* ===================================================================== */
/* A. discard_watch / on_hand_to_discard_each_time_gain_heart01_and_blade  */
/*     PL!HS-pb1-003-R ab#1 「自分の手札からカードが1枚以上控え室に置かれ */
/*     るたび、ライブ終了時まで、heart01+ブレードを得る」 (ターン2回)       */
/*     GAP ONLY: the ターン2回 second-firing budget. The four sibling tests */
/*     are already ported by test_parity_jidou_extra.c §H.                */
/* ===================================================================== */

static void test_rurino_use_limit_blocks_second_same_turn(void)
{
    static TestGame game;
    test_game_new(&game);

    int rurino = test_id(&game, RURINO_W);
    SETUP_BUG(rb_card_no_eq(rurino, RURINO_W),
              "the watcher is the PL!HS-pb1-003-R (大沢瑠璃乃) print");
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = rurino;
    clear_hand(&game);
    clear_recently_moved(&game);

    int f1 = test_id(&game, FILLER);
    int ids[1] = { f1 };
    set_recently_moved(&game, ids, 1);
    tas_full(&game, 0);
    drain_auto(&game);
    CHECK_EQ(heart_mod(&game, rurino, "heart01"), 1, "first: heart01=1");

    int f2 = test_new_id(&game, FILLER);
    int ids2[1] = { f2 };
    set_recently_moved(&game, ids2, 1);
    tas_full(&game, 0);
    drain_auto(&game);
    CHECK_EQ(heart_mod(&game, rurino, "heart01"), 2,
             "the second qualifying event in the same turn grants again (ターン2回)");
    CHECK_EQ(blade_mod(&game, rurino), 2,
             "and blade 2: the re-scan guard must not veto the second 自動");
}

/* ===================================================================== */
/* B. discard_watch / hand_discard_batch_heart_blade_q241_test.rs          */
/*     PL!HS-pb1-003-R ab#0 「手札の『みらくらぱーく！』のメンバーカードを  */
/*     好きな枚数控え室に置き、その後、その枚数に1を足した枚数のカードを   */
/*     引く」 + Q241 「1回の 배치につき発動」                             */
/* ===================================================================== */

static void rurino_hand_batch(TestGame *tg, const char *rurino_no,
                              int miraku_copies, int batch,
                              int *out_waited, int *out_heart, int *out_blade,
                              int *out_hand_prompts, int *out_hand_after)
{
    int rurino = test_id(tg, rurino_no);
    int filler = test_id(tg, FILLER);
    CHECK(rb_card_no_eq(rurino, rurino_no), "the debut host is the printed rarity under test");

    clear_stage(tg, 0);
    clear_hand(tg);
    clear_recently_moved(tg);
    test_add_to_hand(tg, rurino);
    int miraku[8];
    for (int i = 0; i < miraku_copies; i++) {
        miraku[i] = test_new_id(tg, MIRAKU);
        test_add_to_hand(tg, miraku[i]);
    }
    clear_deck(tg);
    for (int i = 0; i < 20; i++) { test_add_to_deck_pl(tg, 0, filler); test_add_to_deck_pl(tg, 1, filler); }
    test_give_energy(tg, 20);

    int wait_before = wait_len(tg);

    test_play_to_stage(tg, rurino, 1);

    /* Answer prompts BY SHAPE (never by ordinal): the first hand SelectCard
     * takes `batch` picks, later ones decline. */
    int hand_prompts = 0;
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        const char *t = choice_type(tg);
        const char *zone = c ? c->zone : NULL;
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&tg->state, 0);
        } else if (t && strcmp(t, "SelectCard") == 0 && zone &&
                   strcmp(zone, "hand") == 0) {
            hand_prompts++;
            if (hand_prompts == 1 && batch > 0) {
                int idx[8];
                for (int i = 0; i < batch; i++) idx[i] = i;
                rb_resume_with_choice_indices(&tg->state, idx, batch);
            } else {
                rb_resume_with_choice(&tg->state, -1);
            }
        } else {
            rb_resume_with_choice(&tg->state, -1);
        }
    }

    int waited = 0;
    for (int i = 0; i < wait_len(tg); i++) {
        int c = tg->state.p[0].discard.cards[i];
        for (int k = 0; k < miraku_copies; k++) if (c == miraku[k]) waited++;
    }
    (void)wait_before;
    *out_waited = waited;
    *out_heart = heart_mod(tg, rurino, "heart01");
    *out_blade = blade_mod(tg, rurino);
    *out_hand_prompts = hand_prompts;
    *out_hand_after = hand_len(tg);
}

static void test_hand_discard_batch_q241_two_discarded_auto_fires_once(void)
{
    static TestGame g1;
    test_game_new(&g1);
    int waited = 0, heart = 0, blade = 0, prompts = 0, hand_after = 0;
    rurino_hand_batch(&g1, RURINO_W, 3, 2, &waited, &heart, &blade, &prompts, &hand_after);
    CHECK(prompts >= 1, "the 登場 must have offered its 「好きな枚数」 hand select");
    CHECK_EQ(waited, 2,
             "好きな枚数 = 2: exactly 2 of the candidates reach the waitroom");
    CHECK_EQ(heart, 1, "Q241: 2 cards discarded in one batch → the 自動 fires once");
    CHECK_EQ(blade, 1, "Q241: 2 cards in one batch → blade exactly 1");

    /* The same batch on the ＋ printing, so rarity cannot be the reason. */
    static TestGame g2;
    test_game_new(&g2);
    rurino_hand_batch(&g2, RURINO_WP, 2, 2, &waited, &heart, &blade, &prompts, &hand_after);
    CHECK_EQ(waited, 2, "＋ printing: exactly 2 candidates reach the waitroom");
    CHECK_EQ(heart, 1, "＋ printing: Q241 still fires once");
    CHECK_EQ(blade, 1, "＋ printing: blade exactly 1");
}

static void test_hand_discard_batch_q241_three_discarded_fires_once(void)
{
    static TestGame game;
    test_game_new(&game);
    int waited = 0, heart = 0, blade = 0, prompts = 0, hand_after = 0;
    rurino_hand_batch(&game, RURINO_W, 4, 3, &waited, &heart, &blade, &prompts, &hand_after);
    CHECK_EQ(waited, 3,
             "好きな枚数 = 3: exactly 3 candidates reach the waitroom — pinned by "
             "IDENTITY, because 1 card would also fire the 自動 once");
    CHECK_EQ(heart, 1,
             "Q241: 3 cards discarded in one batch still fires the 自動 once");
    CHECK_EQ(blade, 1, "Q241: 3 cards in one batch → blade exactly 1");
}

static void test_hand_discard_batch_q241_zero_discarded_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    int waited = 0, heart = 0, blade = 0, prompts = 0, hand_after = 0;
    rurino_hand_batch(&game, RURINO_W, 1, 0, &waited, &heart, &blade, &prompts, &hand_after);
    CHECK_EQ(waited, 0, "0 chosen → no candidate reaches the waitroom");
    CHECK_EQ(heart, 0, "Q241: 0 cards discarded → no trigger at all");
    CHECK_EQ(blade, 0, "Q241: 0 cards discarded → no blade either");
}

/* ===================================================================== */
/* C. discard_watch / live_start_heart_requirement_reduction_counts_       */
/*     live_slot_cards_only_test.rs (Q213 ハナムスビ)                     */
/* ===================================================================== */

static void test_hanamusubi_q213_member_card_moved_before_live_start(void)
{
    static TestGame game;
    test_game_new(&game);

    int hanamusubi = test_id(&game, HANAMUSUBI);
    int filler     = test_id(&game, FILLER);
    int hasetsu    = test_id(&game, HASUNO_MEM);
    int filler2    = test_new_id(&game, FILLER);
    int live       = test_id(&game, REAL_LIVE);
    SETUP_BUG(rb_card_no_eq(hanamusubi, HANAMUSUBI),
              "the Q213 host is the PL!HS-bp5-019-L (ハナムスビ) print");
    SETUP_BUG(rb_card_no_eq(live, REAL_LIVE),
              "the real live card fixture is the LL-bp5-001-L print");
    /* Rust names this card 「蓮ノ空 member」; cards.json agrees on the SERIES
     * (ラブライブ！蓮ノ空女学院スクールアイドルクラブ) though the NAME is
     * 日野下花帆, not 蓮乃. The branch under test is 「not a ライブ」, so the
     * fixture is sound; only the Rust comment mislabels it. */
    SETUP_BUG(rb_card_matches_group_str(hasetsu, "スリーズブーケ") ||
              rb_card_matches_group_str(hasetsu, "蓮ノ空"),
              "the 「non-live member」 fixture really is a 蓮ノ空 non-ライブ card");

    clear_deck(&game);
    clear_hand(&game);
    game.state.p[1].deck.n = 0;
    for (int i = 0; i < 40; i++) {
        test_add_to_deck_pl(&game, 0, filler);
        test_add_to_deck_pl(&game, 1, filler);
    }

    clear_stage(&game, 0);
    game.state.p[0].stage[0] = hanamusubi;
    game.state.p[0].stage[1] = filler;
    game.state.p[0].stage[2] = filler2;
    test_add_to_hand(&game, hasetsu);
    test_add_to_hand(&game, live);

    for (int i = 0; i < 5; i++) test_pass(&game);

    test_set_live_card(&game, 0, hasetsu);   /* a MEMBER, forced into the live slot */
    test_set_live_card(&game, 0, live);      /* and a real ライブ alongside it     */

    int before = wait_len(&game);
    test_pass(&game);   /* LiveCardSetFirstAttacker → P2Turn  */
    test_pass(&game);   /* P2Turn → FirstAttackerPerformance → LiveStart */

    int after = wait_len(&game);
    CHECK(after >= before,
          "a non-ライブ card forced into the live slot is removed before "
          "ライブ開始時, so the Q213 heart-requirement reduction cannot count it");
    /* The Q213 reduction itself counts live-slot 蓮ノ空 cards only: the real
     * ライブ is the one that counts, and the member is gone. */
    CHECK(!test_zone_has_id(&game, 0, "live", hasetsu),
          "the member card is no longer in the live zone at ライブ開始時");
}

/* ===================================================================== */
/* D. discard_watch / deck_to_discard_optional_hand_discard_recover_self  */
/*     PL!N-bp7-011-R＋ ミア・テイラー ab#0                              */
/*     「このカードがデッキから控え室に置かれたとき、手札を1枚控え室に置い */
/*      てもよい。そうしたとき、控え室からこのカードを手札に加える。」   */
/* ===================================================================== */

/* Rust trigger_mia_from: push a deck(or hand)→discard movement event for every
 * moved card, run the TAS scan, then answer conditional_optional with
 * `accept` and the follow-up hand SelectCard. Returns whether a
 * conditional_optional was actually presented. */
static int trigger_mia_from(TestGame *tg, int mia, const int *moved, int n,
                            int from_zone, int accept)
{
    for (int i = 0; i < n; i++)
        rb_record_card_movement(&tg->state, moved[i], from_zone,
                                RB_ZONEID_DISCARD, 0, 1);
    tas_full(tg, 0);

    int saw_optional = 0;
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const char *t = choice_type(tg);
        const char *target = choice_target(tg);
        if (t && strcmp(t, "SelectTarget") == 0 && target &&
            strcmp(target, "conditional_optional") == 0) {
            saw_optional = 1;
            rb_resume_with_choice(&tg->state, accept ? 1 : 0);
        } else if (t && strcmp(t, "SelectCard") == 0) {
            rb_resume_with_choice(&tg->state, choice_count(tg) > 0 ? 0 : -1);
        } else {
            break;
        }
    }
    return saw_optional;
}

static void test_deck_to_discard_self_recovery_accept_discards_one_and_recovers_self(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia = test_id(&game, MIA);
    int f1  = test_id(&game, FILLER);
    int f2  = test_new_id(&game, FILLER);
    SETUP_BUG(rb_card_no_eq(mia, MIA),
              "the watcher is the PL!N-bp7-011-R＋ (ミア・テイラー) print");

    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    test_add_to_hand(&game, f1);
    test_add_to_hand(&game, f2);

    int moved[1] = { mia };
    int saw = trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 1);
    CHECK(saw, "accepting: ab#0 presents its conditional_optional recover");

    CHECK(hand_has(&game, mia), "ミア is recovered to hand on accept");
    CHECK_EQ(hand_len(&game), 2,
             "discard 1 + recover 1 → the hand is back to 2 cards");
    CHECK(wait_has(&game, f1) && !wait_has(&game, f2),
          "exactly the FIRST hand card (f1) is the one discarded");
}

static void test_deck_to_discard_self_recovery_skip_no_discard_no_recover(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia = test_id(&game, MIA);
    int f1  = test_id(&game, FILLER);
    int f2  = test_new_id(&game, FILLER);
    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    test_add_to_hand(&game, f1);
    test_add_to_hand(&game, f2);
    int before = hand_len(&game);

    int moved[1] = { mia };
    trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 0);

    CHECK(wait_has(&game, mia), "skipping leaves ミア in the waitroom");
    CHECK_EQ(hand_len(&game), before, "skipping discards nothing from hand");
    CHECK(!hand_has(&game, mia), "and ミア is NOT recovered");
}

static void test_deck_to_discard_self_recovery_empty_hand_no_recover(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia = test_id(&game, MIA);
    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);

    int moved[1] = { mia };
    trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 1);

    CHECK(wait_has(&game, mia),
          "empty hand → the optional discard cannot happen, so ミア stays put");
    CHECK(!hand_has(&game, mia),
          "そうしたとき: the recover is contingent on the discard actually "
          "happening, so an empty hand recovers nothing");
}

static void test_deck_to_discard_self_recovery_recovers_only_self(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia   = test_id(&game, MIA);
    int other = test_id(&game, LIVE_MU);   /* START:DASH!! — a DIFFERENT card */
    int f1    = test_id(&game, FILLER);
    CHECK(mia != other, "ミア and the other waitroom card are distinct ids");
    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    rb_waitroom_add(&game.state.p[0], other);
    test_add_to_hand(&game, f1);

    int moved[1] = { mia };
    trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 1);

    CHECK(hand_has(&game, mia), "ミア is recovered to hand");
    CHECK(wait_has(&game, other),
          "the non-ミア waitroom card must NOT be recovered — the trigger is "
          "self-targeted");
}

static void test_deck_to_discard_self_recovery_does_not_fire_for_other_card(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia = test_id(&game, MIA);
    int f1  = test_id(&game, FILLER);
    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    test_add_to_hand(&game, f1);

    /* A DIFFERENT card went deck→discard. */
    int moved[1] = { f1 };
    trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 1);

    CHECK(!hand_has(&game, mia),
          "ab#0 must not fire when a different card reaches the waitroom");
    CHECK_EQ(hand_len(&game), 1, "the hand is unchanged");
}

static void test_deck_to_discard_self_recovery_does_not_fire_on_hand_to_discard(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia = test_id(&game, MIA);
    int f1  = test_id(&game, FILLER);
    int f2  = test_new_id(&game, FILLER);
    clear_hand(&game);
    clear_waitroom(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    test_add_to_hand(&game, f1);
    test_add_to_hand(&game, f2);
    int hand_before = hand_len(&game);
    int wait_before = wait_len(&game);

    /* ミア went hand→discard, NOT deck→discard. */
    int moved[1] = { mia };
    trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_HAND, 1);

    CHECK(wait_has(&game, mia),
          "手札→控え室 is not the デッキ→控え室 trigger, so ミア stays put");
    CHECK(!hand_has(&game, mia), "and is not recovered to hand");
    CHECK_EQ(hand_len(&game), hand_before,
             "hand→discard must not discard anything from hand");
    CHECK_EQ(wait_len(&game), wait_before, "the waitroom is unchanged");
}

/* ===================================================================== */
/* E. discard_watch / real_mill_optional_discard_recover_self_test.rs     */
/*     The REAL 黒澤ダイヤ mill (deck_top→waitroom) must satisfy ミア's     */
/*     source="deck" condition — the integration question.                 */
/* ===================================================================== */

static void test_real_deck_top_mill_offers_discard_and_recovers_self(void)
{
    static TestGame game;
    test_game_new(&game);

    int dia = test_id(&game, DIAYA);
    int mia = test_id(&game, MIA);
    int filler = test_id(&game, FILLER);
    SETUP_BUG(rb_card_no_eq(dia, DIAYA), "the miller is the PL!S-sd1-013-SD (黒澤ダイヤ) print");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    clear_waitroom(&game);
    clear_recently_moved(&game);
    game.state.p[0].stage[1] = dia;
    test_insert_deck_top(&game, 0, mia);       /* the mill hits ミア first */
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, filler);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, test_new_id(&game, FILLER));

    SETUP_BUG(fire_trigger(&game, dia, RB_TSTR_DEBUT),
              "黒澤ダイヤ's real 登場 (mill 5) exists and is queued");
    drain_targets(&game);

    /* The mill has moved ミア: the real move must have landed her in the
     * waitroom AND the movement event must name her. Either alone would let
     * the test pass for the wrong reason. */
    int moved = 0;
    for (int i = 0; i < game.state.n_batch_movements; i++)
        if (game.state.batch_movements[i].moved_card_id == mia) moved = 1;
    CHECK(moved, "ダイヤ's real mill records a movement for ミア");
    CHECK(wait_has(&game, mia), "and ミア really is in the waitroom after the mill");

    int saw = 0, guard = 0;
    while (rb_has_pending_choice(&game.state) && guard++ < 40) {
        const char *t = choice_type(&game);
        const char *target = choice_target(&game);
        if (t && strcmp(t, "SelectAutoAbility") == 0) rb_resume_with_choice(&game.state, -1);
        else if (t && strcmp(t, "SelectTarget") == 0 && target &&
                 strcmp(target, "conditional_optional") == 0) { saw = 1; rb_resume_with_choice(&game.state, 1); }
        else if (t && strcmp(t, "SelectCard") == 0)
            rb_resume_with_choice(&game.state, choice_count(&game) > 0 ? 0 : -1);
        else break;
    }
    CHECK(saw, "ミア ab#0 must fire when a REAL deck_top→waitroom mill puts her in the waitroom");
    CHECK(hand_has(&game, mia), "ミア is recovered to hand after accepting the optional");
}

static void test_real_hand_discard_does_not_trigger_deck_to_discard_self_recovery(void)
{
    static TestGame game;
    test_game_new(&game);

    int kasumi = test_id(&game, KASUMI);
    int mia    = test_id(&game, MIA);
    int filler = test_id(&game, FILLER);
    SETUP_BUG(rb_card_no_eq(kasumi, KASUMI),
              "the activator is the PL!N-bp1-014-PRproteinbar (中須かすみ) print");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    clear_waitroom(&game);
    clear_recently_moved(&game);
    game.state.p[0].stage[1] = kasumi;
    test_add_to_deck_pl(&game, 0, filler);   /* the draw target */
    test_add_to_hand(&game, mia);            /* ミア herself is the discard */

    SETUP_BUG(fire_trigger(&game, kasumi, RB_TSTR_DEBUT),
              "中須かすみ's real 登場 (draw 1, discard 1 from hand) exists");

    /* Drive the prompts, picking ミア by identity for the hand discard. */
    int mia_discarded = 0, guard = 0;
    while (rb_has_pending_choice(&game.state) && guard++ < 40) {
        const char *t = choice_type(&game);
        if (t && strcmp(t, "SelectAutoAbility") == 0) {
            rb_resume_with_choice(&game.state, -1);
        } else if (t && strcmp(t, "SelectCard") == 0) {
            RbPlayer *P = &game.state.p[0];
            int idx = -1;
            for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == mia) { idx = i; break; }
            if (idx >= 0) { rb_resume_with_choice_indices(&game.state, &idx, 1); mia_discarded = 1; }
            else rb_resume_with_choice(&game.state, choice_count(&game) > 0 ? 0 : -1);
        } else if (t && strcmp(t, "SelectTarget") == 0) {
            rb_resume_with_choice(&game.state, 0);
        } else {
            break;
        }
    }
    CHECK(mia_discarded, "かすみ's discard selected ミア (hand→waitroom)");
    CHECK(wait_has(&game, mia), "ミア really sits in the waitroom after the hand discard");

    /* Now the auto scan: the source was HAND, so ab#0 must stay silent. */
    tas_full(&game, 0);
    int saw = 0, g2 = 0;
    while (rb_has_pending_choice(&game.state) && g2++ < 20) {
        const char *t = choice_type(&game);
        const char *target = choice_target(&game);
        if (t && strcmp(t, "SelectTarget") == 0 && target &&
            strcmp(target, "conditional_optional") == 0) { saw = 1; break; }
        rb_resume_with_choice(&game.state, 0);
    }
    CHECK(!saw, "discarding ミア from HAND must NOT trigger her deck→waitroom ab#0");
    CHECK(!hand_has(&game, mia), "ミア stays discarded, not recovered");
}

/* ===================================================================== */
/* F. discard_watch / deck_to_discard_self_recovery_yell_refresh_          */
/*     q269_q277_test.rs — a yell REVEAL is a 公開, not a deck→waitroom  */
/*     placement, so ab#0 must not fire.                                    */
/* ===================================================================== */

static void test_q269_yell_reveal_does_not_trigger_deck_to_discard_recovery(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia    = test_id(&game, MIA);
    int filler = test_id(&game, FILLER);
    clear_hand(&game);
    clear_deck(&game);
    clear_recently_moved(&game);
    test_add_to_hand(&game, filler);
    test_insert_deck_top(&game, 0, mia);
    for (int i = 0; i < 4; i++) test_add_to_deck_pl(&game, 0, filler);

    int hand_before = hand_len(&game);

    /* Reproduce the real yell flow: the cheer check draws into the RESOLUTION
     * zone, each card is recorded as a revealed card, and only THEN does the
     * auto-ability scan run. The resolution bag is emptied by
     * rb_resolution_clear, so the ids are copied out BEFORE the scan. */
    rb_perform_cheer_check(&game.state, "p1", 1);
    RbBag *res = &game.state.resolution;
    int revealed[RB_MAX_REVEALED_CARDS];
    int n_revealed = res->n < RB_MAX_REVEALED_CARDS ? res->n : RB_MAX_REVEALED_CARDS;
    int saw_mia = 0;
    for (int i = 0; i < n_revealed; i++) {
        revealed[i] = res->cards[i];
        if (revealed[i] == mia) saw_mia = 1;
    }
    CHECK(saw_mia, "the yell really did reveal ミア (the test must be meaningful)");

    for (int i = 0; i < n_revealed; i++) {
        game.state.revealed_cards[i] = revealed[i];
        if (i + 1 > game.state.n_revealed) game.state.n_revealed = i + 1;
    }
    game.state.yell_occurred = 1;

    tas_full(&game, 0);

    CHECK(!pending(&game), "Q269: a yell reveal must not raise ab#0's optional");
    CHECK(!hand_has(&game, mia), "Q269: ミア is not recovered to hand by a yell reveal");
    CHECK_EQ(hand_len(&game), hand_before, "Q269: the yell discards nothing from hand");
    CHECK(!wait_has(&game, mia), "Q269: ミア does not move to the waitroom from a yell reveal");
    game.state.yell_occurred = 0;
}

static void test_q269_control_deck_to_discard_triggers(void)
{
    static TestGame game;
    test_game_new(&game);

    int mia    = test_id(&game, MIA);
    int filler = test_id(&game, FILLER);
    clear_hand(&game);
    clear_deck(&game);
    clear_waitroom(&game);
    clear_recently_moved(&game);
    rb_waitroom_add(&game.state.p[0], mia);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, test_new_id(&game, FILLER));
    test_add_to_deck_pl(&game, 0, filler);

    int moved[1] = { mia };
    int saw = trigger_mia_from(&game, mia, moved, 1, RB_ZONEID_DECK, 1);

    CHECK(saw, "control: a genuine deck→waitroom movement DOES offer the recover");
    CHECK(hand_has(&game, mia),
          "control: the scan is live, so the Q269 negatives are about yell ≠ "
          "deck→waitroom and not about a dead scan");
}

/* ===================================================================== */
/* G. discard_watch / live_discard_optional_deck_top_bottom_q252 +        */
/*     own_live_zone_discard_group_live_optional_deck_top_bottom_q252      */
/*     桜内梨子 ab#0 「『Aqours』のライブカードが自分のライブカード置き場から */
/*     控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に     */
/*     置いてもよい」 (ターン1回)                                          */
/* ===================================================================== */

/* Rust trigger_riko_auto: set_recently_moved_cards(moved) then the TAS scan. */
static void riko_auto(TestGame *tg, const int *moved, int n)
{
    set_recently_moved(tg, moved, n);
    tas_full(tg, 0);
}

static void riko_board(TestGame *tg, const char *riko_no, int stage_area)
{
    int riko = test_id(tg, riko_no);
    int filler = test_id(tg, FILLER);
    clear_stage(tg, 0);
    clear_hand(tg);
    clear_deck(tg);
    clear_waitroom(tg);
    clear_live(tg);
    clear_recently_moved(tg);
    tg->state.p[0].stage[stage_area] = riko;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < 20; i++) { test_add_to_deck_pl(tg, 0, filler); test_add_to_deck_pl(tg, 1, filler); }
    test_give_energy(tg, 5);
}

static void test_riko_q252_no_movement_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    int live = test_new_id(&game, AQ_LIVE_PR);
    int filler = test_id(&game, FILLER);
    /* Directly placed in the waitroom with NO live_card_zone movement. */
    rb_waitroom_add(&game.state.p[0], live);
    (void)filler;
    tas_full(&game, 0);
    CHECK(!pending(&game),
          "no live_card_zone movement → the 自動 must not fire, even with an "
          "『Aqours』 live card sitting in the waitroom");
}

static void test_riko_q252_no_live_cards_anywhere_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    tas_full(&game, 0);
    CHECK(!pending(&game), "no live cards at all → the condition fails, no trigger");
}

static void test_riko_q252_non_aqours_live_alone_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    int non_aq = test_id(&game, NON_AQ_LIVE);
    SETUP_BUG(!rb_card_matches_group_str(non_aq, "Aqours"),
              "the negative fixture is genuinely outside 『Aqours』");
    rb_waitroom_add(&game.state.p[0], non_aq);
    int moved[1] = { non_aq };
    riko_auto(&game, moved, 1);
    CHECK(!pending(&game), "a non-『Aqours』 ライブ → the group filter blocks the trigger");
    CHECK(wait_has(&game, non_aq), "and the card stays in the waitroom");
}

static void test_riko_q252_empty_and_null_moved_cards_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    int live = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live);

    riko_auto(&game, NULL, 0);
    CHECK(!pending(&game), "an empty recently-moved set → the condition counts 0");

    clear_recently_moved(&game);
    tas_full(&game, 0);
    CHECK(!pending(&game), "no recently-moved record at all → the condition counts 0");
}

static void test_riko_q252_watcher_off_stage_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    clear_stage(&game, 0);                 /* the watcher leaves the stage */
    int live = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live);
    int moved[1] = { live };
    riko_auto(&game, moved, 1);
    CHECK(!pending(&game), "the TAS scan only sees stage cards, so an off-stage "
                            "桜内梨子 is never enqueued");
    CHECK(wait_has(&game, live), "and the live card stays in the waitroom");
}

static void test_riko_q252_mixed_batch_filters_correctly(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);

    int aq_live   = test_new_id(&game, AQ_LIVE_PR);
    int non_aq    = test_new_id(&game, NON_AQ_LIVE);
    int member    = test_new_id(&game, AQ_MEMBER);
    int energy    = test_new_id(&game, ENERGY_CARD);
    SETUP_BUG(rb_card_matches_group_str(aq_live, "Aqours"),
              "the positive fixture is genuinely inside 『Aqours』");
    SETUP_BUG(rb_card_matches_group_str(member, "Aqours"),
              "the 『Aqours』 MEMBER fixture is inside the group but is not a ライブ");
    rb_waitroom_add(&game.state.p[0], aq_live);
    rb_waitroom_add(&game.state.p[0], non_aq);
    rb_waitroom_add(&game.state.p[0], member);
    rb_waitroom_add(&game.state.p[0], energy);

    int moved[4] = { aq_live, non_aq, member, energy };
    riko_auto(&game, moved, 4);

    CHECK(pending(&game), "one 『Aqours』 ライブ in the batch satisfies the condition");
    /* The effect's "put it on the deck" step is a generated position|
     * destination prompt, which the C shim cannot enumerate (see
     * test_riko_q252_single_card_deck_top_exact_identity). Accept the FIRST
     * option and assert the SELECTION half, which is what the filter governs. */
    {
        const char *t = choice_type(&game);
        const char *target = choice_target(&game);
        CHECK(target && strcmp(target, "position|destination") == 0,
              "the prompt after the filter is the deck placement step");
        (void)t;
    }
    rb_resume_with_choice(&game.state, 0);
    drain_auto(&game);
    CHECK(!wait_has(&game, aq_live),
          "the single 『Aqours』 ライブ is the one taken — the non-『Aqours』 "
          "ライブ, the 『Aqours』 member and the energy are all filtered out");
    CHECK(wait_has(&game, non_aq),  "the non-『Aqours』 ライブ remains");
    CHECK(wait_has(&game, member), "the 『Aqours』 member remains (not a ライブ)");
    CHECK(wait_has(&game, energy), "the energy card remains");
}

static void test_riko_q252_single_card_deck_top_exact_identity(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);
    int live = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live);
    int deck_before = deck_len(&game);

    int moved[1] = { live };
    riko_auto(&game, moved, 1);
    CHECK(pending(&game), "a single 『Aqours』 ライブ reaching the waitroom offers the placement");
    {
        /* KNOWN C LIMITATION: the FLAT RbChoice carries no destination list at
         * all — choice.c computes valid_destinations[] and then discards it —
         * so a deck_top / deck_bottom option is not observable from C and the
         * answer index is taken as an ABSOLUTE area. Recorded, never turned
         * into a pass. */
        EXPECTED_GAP(0,
                     "the position|destination prompt carries its deck_top / "
                     "deck_bottom options — the flat RbChoice struct has no "
                     "options list (only n_heart_options), so no destination is "
                     "observable from C and the answer index is read as an area");
    }
    rb_resume_with_choice(&game.state, 0);
    drain_auto(&game);

    CHECK(!wait_has(&game, live), "the targeted live card leaves the waitroom");
    CHECK_EQ(wait_len(&game), 0, "nothing else is left in the waitroom");
    CHECK_EQ(deck_len(&game), deck_before + 1, "the deck gains exactly one card");
    CHECK(deck_has(&game, live),
          "the relocated live card really reached the deck (deck_top when the "
          "destination prompt resolves to index 0, deck_bottom otherwise)");
}

static void test_riko_q252_turn_limit_blocks_second_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);

    int live_a = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live_a);
    int m1[1] = { live_a };
    riko_auto(&game, m1, 1);
    CHECK(pending(&game), "first trigger: the placement prompt appears");
    rb_resume_with_choice(&game.state, 0);
    drain_auto(&game);
    CHECK(!wait_has(&game, live_a), "first trigger: live_a is relocated");
    CHECK(deck_has(&game, live_a), "first trigger: live_a reached the deck");

    int live_b = test_new_id(&game, AQ_LIVE_SD);
    rb_waitroom_add(&game.state.p[0], live_b);
    int m2[1] = { live_b };
    riko_auto(&game, m2, 1);
    CHECK(!pending(&game),
          "second trigger in the same turn: ターン1回 is spent, so no prompt");
    CHECK(wait_has(&game, live_b), "and live_b stays in the waitroom untouched");
    CHECK(deck_has(&game, live_a), "live_a is still on the deck, not displaced");
}

static void test_riko_q252_decline_keeps_cards_in_waitroom(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_SEC, 1);

    int live1 = test_new_id(&game, AQ_LIVE_PB);
    int live2 = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live1);
    rb_waitroom_add(&game.state.p[0], live2);
    int m[2] = { live1, live2 };
    riko_auto(&game, m, 2);
    CHECK(pending(&game), "Q252: with 2 candidates the player is prompted to pick 1 (or skip)");

    rb_resume_with_choice(&game.state, -1);   /* skip */
    drain_auto(&game);

    CHECK(wait_has(&game, live1), "declining keeps live1 in the waitroom");
    CHECK(wait_has(&game, live2), "declining keeps live2 in the waitroom");
    CHECK(!deck_has(&game, live1), "and neither card reaches the deck");
    CHECK(!deck_has(&game, live2), "and neither card reaches the deck");
}

static void test_riko_q252_r_plus_printing_selects_the_second_card(void)
{
    static TestGame game;
    test_game_new(&game);
    riko_board(&game, RIKO_RP, 1);

    int live1 = test_new_id(&game, AQ_LIVE_PB);
    int live2 = test_new_id(&game, AQ_LIVE_PR);
    rb_waitroom_add(&game.state.p[0], live1);
    rb_waitroom_add(&game.state.p[0], live2);
    int m[2] = { live1, live2 };
    riko_auto(&game, m, 2);
    CHECK(pending(&game), "the ＋ printing raises the same selection prompt");

    {
        const char *t = choice_type(&game);
        const char *target = choice_target(&game);
        CHECK(t && (strcmp(t, "SelectCard") == 0 || strcmp(t, "SelectTarget") == 0),
              "the prompt is the Q252 card selection");
        (void)target;
    }
    /* Answer with the SECOND index, not the first: a first-option-only answer
     * cannot tell a real selection from a hard-wired one. */
    rb_resume_with_choice(&game.state, 1);
    drain_auto(&game);
    CHECK(!wait_has(&game, live2),
          "picking index 1 really moves live2 out of the waitroom");
    CHECK(wait_has(&game, live1), "and leaves live1 in the waitroom");
}

/* ===================================================================== */
/* H. debut_watch / ally_member_debut_pay_energy_or_center_ally_blade      */
/*     PL!HS-pb1-001-R (optional pay E) and PL!HS-pb1-009-R (blade+2 at    */
/*     center) — both keyed on 『スリーズブーケ』 members appearing.        */
/* ===================================================================== */

static int play_and_drain(TestGame *tg, int card, int area, int *saw_other)
{
    int played = test_play_to_stage(tg, card, area);
    drain_recording(tg, saw_other);
    return played;
}

static void test_hana_001_ally_appears_offers_payment(void)
{
    static TestGame game;
    test_game_new(&game);

    int hana  = test_id(&game, HANA_001);
    int ally  = test_id(&game, CERISE_C4);   /* 百生吟子, cost 4 */
    int filler= test_id(&game, FILLER);
    int e     = test_id(&game, ENERGY_CARD);
    SETUP_BUG(rb_card_no_eq(hana, HANA_001), "the watcher is the PL!HS-pb1-001-R print");
    SETUP_BUG(rb_card_matches_group_str(ally, "スリーズブーケ"),
              "the arriving ally really is a 『スリーズブーケ』 member");
    SETUP_BUG(rb_card_matches_group_str(hana, "スリーズブーケ"),
              "the watcher is a 『スリーズブーケ』 member herself");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].stage[1] = hana;
    test_add_to_hand(&game, ally);
    game.state.p[0].energy.n = 0;
    for (int i = 0; i < 10; i++) {
        game.state.p[0].energy.cards[game.state.p[0].energy.n++] = e;
    }
    rb_energy_set_active_count(&game.state.p[0], 10);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&game, 0, filler);

    int before = active_energy(&game);
    int saw_other = 0;
    int played = play_and_drain(&game, ally, 0, &saw_other);
    CHECK(played, "the 『スリーズブーケ』 ally is played");
    CHECK_EQ(game.state.p[0].stage[0], ally, "and lands at Left");
    CHECK(saw_other,
          "the each_time fired: an optional payment prompt was raised (Rust "
          "asserts the rule_log contains \"offered\")");
    CHECK_EQ(active_energy(&game), before - 4,
             "declining the payment leaves energy at the play cost only (4)");
}

static void test_hana_001_non_matching_ally_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);

    int hana   = test_id(&game, HANA_001);
    int non_ally = test_id(&game, FILLER);   /* ラブライブ！, not スリーズブーケ */
    int filler = test_id(&game, FILLER);
    int e      = test_id(&game, ENERGY_CARD);
    SETUP_BUG(!rb_card_matches_group_str(non_ally, "スリーズブーケ"),
              "the negative fixture really is OUTSIDE 『スリーズブーケ』");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].stage[1] = hana;
    test_add_to_hand(&game, non_ally);
    game.state.p[0].energy.n = 0;
    for (int i = 0; i < 10; i++) game.state.p[0].energy.cards[game.state.p[0].energy.n++] = e;
    rb_energy_set_active_count(&game.state.p[0], 10);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&game, 0, filler);

    int before = active_energy(&game);
    int saw_other = 0;
    play_and_drain(&game, non_ally, 0, &saw_other);
    CHECK(!saw_other, "a non-『スリーズブeke』 ally must not raise the payment prompt");
    CHECK_EQ(active_energy(&game), before - 4,
             "no trigger means no energy activation, only the play cost (4)");

    /* CONTROL: the identical play on a board with 花帆 001 ABSENT. If a prompt
     * appears here too, the prompt belongs to the PLAY rather than to 001's
     * 自動, and the assertion above is about the wrong thing; if none appears,
     * the prompt above really is 001's 自動 firing without its group gate. */
    static TestGame control;
    test_game_new(&control);
    {
        int non_ally2 = test_id(&control, FILLER);
        int filler2   = test_id(&control, FILLER);
        int e2        = test_id(&control, ENERGY_CARD);
        clear_stage(&control, 0);
        clear_hand(&control);
        clear_deck(&control);
        test_add_to_hand(&control, non_ally2);
        control.state.p[0].energy.n = 0;
        for (int i = 0; i < 10; i++)
            control.state.p[0].energy.cards[control.state.p[0].energy.n++] = e2;
        rb_energy_set_active_count(&control.state.p[0], 10);
        for (int i = 0; i < 40; i++) test_add_to_deck_pl(&control, 0, filler2);
        int c_before = active_energy(&control);
        int c_saw = 0;
        play_and_drain(&control, non_ally2, 0, &c_saw);
        CHECK(!c_saw,
              "CONTROL: with no 花帆 001 on the stage, the identical play raises "
              "no prompt at all — so the prompt in the case above IS 001's 自動");
        CHECK_EQ(active_energy(&control), c_before - 4, "CONTROL: only the play cost");
    }
}

static void test_hana_001_self_play_no_trigger(void)
{
    static TestGame game;
    test_game_new(&game);

    int hana    = test_id(&game, HANA_001);
    int filler  = test_id(&game, FILLER);
    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].stage[0] = filler;
    test_add_to_hand(&game, hana);
    test_give_energy(&game, 15);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&game, 0, filler);

    int before = active_energy(&game);
    int saw_other = 0;
    play_and_drain(&game, hana, 1, &saw_other);
    CHECK(!saw_other,
          "「ほかの」成员的登場 excludes her own debut, so no payment prompt");
    CHECK_EQ(active_energy(&game), before - 11,
             "self-appearance costs her printed 11 and activates no energy");
}

static void hana_009_case(TestGame *tg, const char *hana_no, int hana_area,
                          int ally_area, int expect_blade)
{
    int hana   = test_id(tg, hana_no);
    int ally   = test_id(tg, CERISE_C4);
    int filler = test_id(tg, FILLER);
    int stabilize = test_new_id(tg, FILLER);
    CHECK(rb_card_no_eq(hana, hana_no), "the watcher is the printed 009 rarity");
    CHECK(rb_card_matches_group_str(ally, "スリーズブーケ"),
          "the arriving ally is a 『スリーズブーケ』 member");

    clear_stage(tg, 0);
    clear_hand(tg);
    clear_deck(tg);
    tg->state.p[0].stage[hana_area] = hana;
    if (hana_area != 1 && ally_area == 1) tg->state.p[0].stage[1] = stabilize;
    test_add_to_hand(tg, ally);
    test_give_energy(tg, 15);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(tg, 0, filler);

    int before = blade_mod(tg, hana);
    play_and_drain(tg, ally, ally_area, NULL);
    int after = blade_mod(tg, hana);
    CHECK_EQ(after, before + expect_blade,
             expect_blade
             ? "009 at CENTER gains blade+2 when a 『スリーズブeke』 member appears"
             : "009 is NOT at center, so 「自分のステージのセンター」 is false and it "
               "gains nothing");
}

static void test_hana_009_center_ally_appears_gains_blade(void)
{
    static TestGame g1;
    test_game_new(&g1);
    hana_009_case(&g1, HANA_009, 1, 0, 2);
}
static void test_hana_009_p_plus_center_ally_appears_gains_blade(void)
{
    static TestGame g2;
    test_game_new(&g2);
    hana_009_case(&g2, HANA_009P, 1, 0, 2);
}
static void test_hana_009_not_center_no_trigger(void)
{
    static TestGame g3;
    test_game_new(&g3);
    hana_009_case(&g3, HANA_009, 0, 1, 0);
}
static void test_hana_009_q245_self_deploy_to_center_triggers(void)
{
    static TestGame g4;
    test_game_new(&g4);
    /* Q245: 009 deployed to CENTER counts as her own appearance. */
    int hana   = test_id(&g4, HANA_009);
    int filler = test_id(&g4, FILLER);
    clear_stage(&g4, 0);
    clear_hand(&g4);
    clear_deck(&g4);
    g4.state.p[0].stage[0] = test_new_id(&g4, FILLER);
    test_add_to_hand(&g4, hana);
    test_give_energy(&g4, 15);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&g4, 0, filler);
    CHECK_EQ(blade_mod(&g4, hana), 0, "no blade before the self-deploy");
    play_and_drain(&g4, hana, 1, NULL);
    CHECK_EQ(g4.state.p[0].stage[1], hana, "009 really is at Center");
    CHECK(blade_mod(&g4, hana) >= 2,
          "Q245: 009 deployed to center triggers her own 自動 (blade+2)");
}
static void test_hana_009_q245_self_deploy_non_center_no_trigger(void)
{
    static TestGame g5;
    test_game_new(&g5);
    int hana   = test_id(&g5, HANA_009);
    int filler = test_id(&g5, FILLER);
    clear_stage(&g5, 0);
    clear_hand(&g5);
    clear_deck(&g5);
    g5.state.p[0].stage[1] = test_new_id(&g5, FILLER);   /* Center occupied */
    test_add_to_hand(&g5, hana);
    test_give_energy(&g5, 15);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&g5, 0, filler);
    CHECK_EQ(blade_mod(&g5, hana), 0, "no blade before the non-center deploy");
    play_and_drain(&g5, hana, 0, NULL);
    CHECK_EQ(blade_mod(&g5, hana), 0,
             "Q245 edge: 009 deployed to NON-center must not trigger — the "
             "condition requires the Center area");
}

/* ===================================================================== */
/* I. debut_watch / jump_up_high_multi_target_blade_grant_lifecycle        */
/*     PL!S-sd1-022-SD 「ライブ終了時まで、自分のステージにいる『Aqours』の */
/*     メンバーはブレードを得る」 — the MULTI-TARGET fan-out.              */
/* ===================================================================== */

static void jump_up_board(TestGame *tg, int *out_a, int *out_b, int *out_outsider)
{
    int live     = test_id(tg, JUMP_UP);
    int a        = test_id(tg, AQOURS_A);
    int b        = test_id(tg, AQOURS_B);
    int outsider = test_id(tg, NOT_AQOURS);
    int filler   = test_id(tg, FILLER);

    CHECK(rb_card_no_eq(live, JUMP_UP), "the grant host is the PL!S-sd1-022-SD print");
    CHECK(rb_card_matches_group_str(a, "Aqours"),
          "beneficiary A really is inside 『Aqours』 (by series, not by unit)");
    CHECK(rb_card_matches_group_str(b, "Aqours"), "beneficiary B really is inside 『Aqours』");
    CHECK(!rb_card_matches_group_str(outsider, "Aqours"),
          "the non-beneficiary must be OUTSIDE 『Aqours』, or the group filter is "
          "never exercised in the negative direction");

    clear_stage(tg, 0);
    clear_hand(tg);
    clear_live(tg);
    clear_recently_moved(tg);
    tg->state.p[0].stage[0] = a;
    tg->state.p[0].stage[1] = b;
    tg->state.p[0].stage[2] = outsider;
    test_add_to_live(tg, live);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
    *out_a = a; *out_b = b; *out_outsider = outsider;
}

static void test_jump_up_high_gives_each_aqours_member_exactly_one_blade(void)
{
    /* CONTROL FIRST: the same board with NO ライブ開始時 fired, put through
     * rb_recalc_constants. This isolates 「the recalc re-derives the grant
     * without its group filter」 from 「the outsider card grants blades on her
     * own 常時」 — the two produce an identical symptom at the assertion below
     * and only this control tells them apart. */
    static TestGame control;
    test_game_new(&control);
    {
        int ca, cb, co;
        jump_up_board(&control, &ca, &cb, &co);
        for (int i = 0; i < 3; i++) rb_recalc_constants(&control.state);
        CHECK_EQ(blade_mod(&control, co), 0,
                 "CONTROL: on this same board the non-『Aqours』 member gains no "
                 "blades from recalc alone — so any blade appearing below comes "
                 "from the ライブ開始時 grant being re-derived, not from the card's "
                 "own 常時");
        CHECK_EQ(blade_mod(&control, ca), 0,
                 "CONTROL: and neither does an 『Aqours』 member");
    }

    static TestGame game;
    test_game_new(&game);
    int a, b, outsider;
    jump_up_board(&game, &a, &b, &outsider);

    int live = test_id(&game, JUMP_UP);
    CHECK(fire_trigger(&game, live, RB_TSTR_LIVE_START),
          "the ライブ開始時 trigger is queued for Jump up HIGH!!");
    drain_targets(&game);

    CHECK_EQ(blade_mod(&game, a), 1,
             "an 『Aqours』 member gains EXACTLY ONE blade — 2 would mean the "
             "fan-out double-counted, and a `>= 1` check cannot see that");
    CHECK_EQ(blade_mod(&game, b), 1, "the second 『Aqours』 member also gains exactly one");
    CHECK_EQ(blade_mod(&game, outsider), 0,
             "『Aqours』のメンバー is the printed condition, so a non-『Aqours』 "
             "member on the same board gains nothing");
    for (int round = 1; round <= 5; round++) {
        rb_recalc_constants(&game.state);
        char msg[96];
        snprintf(msg, sizeof(msg), "round %d: the grant is stable, not cumulative", round);
        CHECK_EQ(blade_mod(&game, a), 1, msg);
        CHECK_EQ(blade_mod(&game, b), 1, msg);
        CHECK_EQ(blade_mod(&game, outsider), 0, msg);
    }
}

static void test_jump_up_high_does_not_reach_a_late_arrival_nor_outlive_a_departure(void)
{
    static TestGame game;
    test_game_new(&game);
    int a, b, outsider;
    jump_up_board(&game, &a, &b, &outsider);
    int live = test_id(&game, JUMP_UP);
    CHECK(fire_trigger(&game, live, RB_TSTR_LIVE_START), "the ライブ開始時 fires");
    drain_targets(&game);
    CHECK_EQ(blade_mod(&game, a), 1, "precondition: member A holds the blade");
    CHECK_EQ(blade_mod(&game, b), 1, "precondition: member B holds the blade");

    /* A departure goes through the engine's own zone-exit choke point. */
    game.state.p[0].stage[0] = RB_EMPTY_SLOT;
    rb_on_cards_left_zones(&game.state, a);
    rb_recalc_constants(&game.state);
    CHECK_EQ(blade_mod(&game, a), 0,
             "rule 4.1.4: a member that left the stage must not keep a grant "
             "resolved while it was on the board");
    CHECK_EQ(blade_mod(&game, b), 1,
             "the other member's grant is unaffected by one departure");

    /* An arrival AFTER the trigger does not receive it: the condition is read
     * when ライブ開始時 fires, not on every scan. */
    int latecomer = test_new_id(&game, AQOURS_A);
    game.state.p[0].stage[0] = latecomer;
    rb_recalc_constants(&game.state);
    CHECK_EQ(blade_mod(&game, latecomer), 0,
             "『自分のステージにいる』 is assessed when the trigger fires, so a "
             "member arriving later does not gain the blade");
}

static void test_jump_up_high_grant_is_scoped_to_the_live_card(void)
{
    static TestGame game;
    test_game_new(&game);
    int a, b, outsider;
    jump_up_board(&game, &a, &b, &outsider);
    int live = test_id(&game, JUMP_UP);

    CHECK_EQ(blade_mod(&game, a), 0,
             "precondition: the grant exists only after ライブ開始時 fires");
    CHECK(fire_trigger(&game, live, RB_TSTR_LIVE_START), "the live card's trigger fires");
    drain_targets(&game);
    CHECK_EQ(blade_mod(&game, a), 1, "the live card's trigger does grant it");

    int extra = test_new_id(&game, AQOURS_B);
    game.state.p[0].stage[1] = extra;
    rb_recalc_constants(&game.state);
    CHECK_EQ(blade_mod(&game, extra), 0,
             "a member added after the grant does not gain it (the membership "
             "latch, pinned from the other side)");
    CHECK_EQ(blade_mod(&game, a), 1,
             "and the original holder is not topped up by the later scan");
    CHECK(rb_card_no_eq(extra, AQOURS_B),
          "the late arrival is a genuine 『Aqours』 member print, and the grant "
          "host really is the ライブ print, not a member printing the same text");
}

/* ===================================================================== */
/* J. debut_watch / three_way_choices_and_look_at_three_test.rs            */
/*     The two THREE-WAY choices. The look-and-select pair in that file is  */
/*     explicitly left unportable by its own author (prefix card numbers).  */
/* ===================================================================== */

static void test_ericho_gains_only_the_chosen_heart_two_per_success_card(void)
{
    static TestGame game;
    test_game_new(&game);
    int ericho = test_id(&game, ERICHO);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(ericho, ERICHO), "the chooser is the PL!-bp5-011-N (絢瀬絵里) print");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].success.n = 0;
    game.state.p[0].stage[0] = ericho;
    for (int i = 0; i < 40; i++) { test_add_to_deck_pl(&game, 0, filler); test_add_to_deck_pl(&game, 1, filler); }
    /* Two ライブ in the own SUCCESS zone → the chosen heart is gained twice. */
    test_add_to_success(&game, test_new_id(&game, LIVE_MU));
    test_add_to_success(&game, test_new_id(&game, LIVE_MU));
    CHECK_EQ(game.state.p[0].success.n, 2, "precondition: two success-zone cards");

    CHECK(fire_trigger(&game, ericho, RB_TSTR_LIVE_START), "the ライブ開始時 fires");
    CHECK(pending(&game), "precondition: the 3-way heart choice is actually offered");
    {
        EXPECTED_GAP(choice_n_heart_options(&game) >= 3,
                     "the heart choice publishes all THREE options with their "
                     "colours, so \"option 2 == heart06\" is checkable from C");
    }
    /* The LAST option, not the first. */
    rb_resume_with_choice(&game.state, 2);

    CHECK_EQ(heart_mod(&game, ericho, "heart06"), 2,
             "『カード1枚につき、選んだハートを1つ得る』 with 2 success cards = 2 of "
             "the CHOSEN heart; a first-option implementation lands on heart04");
    CHECK_EQ(heart_mod(&game, ericho, "heart04"), 0,
             "the FIRST option was not chosen, so heart04 is not granted — the "
             "assertion an 'applies all three' reading fails");
    CHECK_EQ(heart_mod(&game, ericho, "heart05"), 0, "and the middle option is not granted either");
}

static void test_ericho_gains_nothing_with_an_empty_success_zone(void)
{
    static TestGame game;
    test_game_new(&game);
    int ericho = test_id(&game, ERICHO);
    int filler = test_id(&game, FILLER);
    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].stage[0] = ericho;
    game.state.p[0].success.n = 0;
    for (int i = 0; i < 40; i++) { test_add_to_deck_pl(&game, 0, filler); test_add_to_deck_pl(&game, 1, filler); }

    CHECK(fire_trigger(&game, ericho, RB_TSTR_LIVE_START), "the ライブ開始時 fires");
    CHECK(pending(&game), "precondition: the choice is offered regardless of the success zone");
    rb_resume_with_choice(&game.state, 2);

    CHECK_EQ(heart_mod(&game, ericho, "heart04"), 0,
             "『カード1枚につき』 multiplies a count of 0: no heart of any colour, "
             "including the chosen one");
    CHECK_EQ(heart_mod(&game, ericho, "heart05"), 0, "…heart05 is 0 too");
    CHECK_EQ(heart_mod(&game, ericho, "heart06"), 0, "…and the chosen heart06 is 0 as well");
}

static void test_wakana_the_chosen_heart_replaces_her_printed_hearts(void)
{
    static TestGame game;
    test_game_new(&game);
    int wakana = test_id(&game, WAKANA);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(wakana, WAKANA), "the chooser is the PL!SP-pb2-030-N (若菜四季) print");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    game.state.p[0].stage[0] = wakana;
    for (int i = 0; i < 40; i++) { test_add_to_deck_pl(&game, 0, filler); test_add_to_deck_pl(&game, 1, filler); }

    /* She PRINTS heart06, so the test must pick a different option. */
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&game.state.mods, wakana), -1,
             "precondition: no colour override is applied before the choice");

    CHECK(fire_trigger(&game, wakana, RB_TSTR_LIVE_START), "the ライブ開始時 fires");
    CHECK(pending(&game), "precondition: the 3-way heart choice is offered");
    /* The MIDDLE option, heart03 — not the one she already prints. */
    rb_resume_with_choice(&game.state, 1);

    int applied = rb_mods_get_heart_color_multiplier(&game.state.mods, wakana);
    CHECK_EQ(applied, RB_HEART_YELLOW,
             "『元々持つハートは選んだハートになる』 — the chosen heart is applied as "
             "a duration-scoped colour override, replacing the heart06 she printed");
    CHECK(applied != RB_HEART_ORANGE,
          "and it is NOT the heart she printed — a 'choose the first option' or "
          "'keep the original' reading lands on heart06 and fails only here");
}

/* ===================================================================== */
/* K. debut_watch / edel_note_ally_debut_waits_opponent_active_member     */
/*     PL!HS-bp6-007 セラス 柳田 自動(ターン1回)                          */
/*     「自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身 */
/*      のステージにいるアクティブ状態のメンバー1人をウェイトにする。」    */
/* ===================================================================== */

static int is_wait(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && strcmp(o, "wait") == 0;
}

static void ceras_appearance(TestGame *tg, int p2_active)
{
    int ceras = test_new_id(tg, CERAS_R);
    int filler = test_id(tg, FILLER);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    clear_hand(tg);
    clear_deck(tg);
    clear_recently_moved(tg);
    tg->state.p[0].stage[0] = ceras;
    int ids[1] = { ceras };
    set_recently_moved(tg, ids, 1);
    for (int i = 0; i < p2_active; i++) tg->state.p[1].stage[i] = test_new_id(tg, FILLER);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
    tg->state.phase = RB_PHASE_MAIN;
}

static void test_edel_note_waits_lone_active_opponent_member(void)
{
    static TestGame game;
    test_game_new(&game);
    ceras_appearance(&game, 1);
    int ceras = game.state.p[0].stage[0];
    int opp   = game.state.p[1].stage[0];

    CHECK(!is_wait(&game, opp), "precondition: the opponent member starts ACTIVE");
    CHECK(fire_trigger(&game, ceras, RB_TSTR_AUTO), "the 自動 is queued");
    CHECK(!pending(&game), "1 target → the auto-resolved path, no prompt");
    CHECK(is_wait(&game, opp), "Q250: the lone active opponent member is put to WAIT");
}

static void test_edel_note_waits_the_selected_opponent_member_only(void)
{
    static TestGame game;
    test_game_new(&game);
    ceras_appearance(&game, 2);
    int ceras = game.state.p[0].stage[0];
    int m0 = game.state.p[1].stage[0];
    int m1 = game.state.p[1].stage[1];
    CHECK(m0 != m1, "the two opponent members are distinct instances");

    CHECK(!is_wait(&game, m0), "m0 starts active");
    CHECK(!is_wait(&game, m1), "m1 starts active");
    CHECK(fire_trigger(&game, ceras, RB_TSTR_AUTO), "the 自動 is queued");
    CHECK(pending(&game), "2 candidates → the opponent picks which one to wait");

    int idx[1] = { 1 };
    rb_resume_with_choice_indices(&game.state, idx, 1);
    CHECK(!pending(&game), "the choice resolves completely");

    CHECK(!is_wait(&game, m0), "m0 stays active — only the SELECTED member waits");
    CHECK(is_wait(&game, m1), "m1 is the one put to wait");
}

static void test_edel_note_without_recent_debut_offers_no_wait(void)
{
    static TestGame game;
    test_game_new(&game);
    int ceras = test_new_id(&game, CERAS_R);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    clear_recently_moved(&game);
    game.state.p[0].stage[0] = ceras;
    game.state.phase = RB_PHASE_MAIN;

    CHECK(fire_trigger(&game, ceras, RB_TSTR_AUTO),
          "the 自動 can be queued directly (the Rust test drives the same path)");
    CHECK(!pending(&game),
          "no appearance (no recently-moved card) → no wait prompt is offered");
}

static void test_edel_note_turn_limit_blocks_second_trigger(void)
{
    static TestGame game;
    test_game_new(&game);
    ceras_appearance(&game, 2);
    int ceras = game.state.p[0].stage[0];
    int m0 = game.state.p[1].stage[0];

    CHECK(fire_trigger(&game, ceras, RB_TSTR_AUTO), "first trigger is queued");
    CHECK(pending(&game), "first trigger works and offers the selection");
    rb_resume_with_choice(&game.state, 0);
    CHECK(!pending(&game), "first trigger resolves");
    CHECK(is_wait(&game, m0), "and really waited m0");

    /* Re-trigger with the SAME copy: use_limit (1/turn) blocks it. */
    int ids[1] = { ceras };
    set_recently_moved(&game, ids, 1);
    fire_trigger(&game, ceras, RB_TSTR_AUTO);
    CHECK(!pending(&game),
          "second trigger in the same turn is blocked by ターン1回 — same copy");
    CHECK(is_wait(&game, m0), "and m0 is not touched again");
}

static void test_edel_note_p_self_debut_waits_lone_opponent_member(void)
{
    static TestGame game;
    test_game_new(&game);
    int ceras = test_id(&game, CERAS_P);
    int p2m   = test_id(&game, FILLER);
    SETUP_BUG(rb_card_no_eq(ceras, CERAS_P), "the P print fixture resolves");
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    clear_hand(&game);
    game.state.p[1].stage[1] = p2m;
    test_add_to_hand(&game, ceras);
    test_add_to_hand(&game, test_new_id(&game, FILLER));
    test_give_energy(&game, 15);

    CHECK(test_play_to_stage(&game, ceras, 1), "the P print debuts to Center");
    drain_auto(&game);

    CHECK(is_wait(&game, p2m), "the P print's debut still waits the lone opponent member");
    CHECK(!pending(&game), "and nothing is left pending");
}

/* ===================================================================== */
/* L. debut_watch / baton_touch_arrival_self_or_other_draws_one_twice_...  */
/*     PL!N-PR-025-PR 優木せつ菜 自動(ターン2回)                          */
/*     「自分のステージに、このメンバーか、ほかのメンバーがバトンタッチして  */
/*      登場したとき、カードを1枚引く。」                                   */
/* ===================================================================== */

static int setsuna_use_count(TestGame *tg, int setsuna)
{
    return rb_use_count(&tg->state.queue, setsuna, 0, tg->state.turn);
}

static void setsuna_setup(TestGame *tg, int *out_setsuna)
{
    int setsuna = test_id(tg, SETSUNA);
    int filler  = test_id(tg, FILLER);
    CHECK(rb_card_no_eq(setsuna, SETSUNA), "the watcher is the PL!N-PR-025-PR (優木せつ菜) print");
    clear_stage(tg, 0);
    clear_hand(tg);
    clear_deck(tg);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 0, filler);
    test_give_energy(tg, 30);
    *out_setsuna = setsuna;
}

static void test_setsuna_self_baton_arrival_draws_one(void)
{
    static TestGame game;
    test_game_new(&game);
    int setsuna;
    setsuna_setup(&game, &setsuna);
    (void)test_id(&game, FILLER);   /* template resolved for the pool; the board uses a fresh copy */
    int filler_c = test_new_id(&game, FILLER);

    baton_touch(&game, filler_c, setsuna, 1);
    resolve_accepting_optionals(&game);

    CHECK_EQ(hand_len(&game), 1, "self baton-touch arrival draws exactly 1");
    CHECK_EQ(setsuna_use_count(&game, setsuna), 1, "one use is recorded");
}

static void test_setsuna_turn2_budget_two_draws_third_refused(void)
{
    static TestGame game;
    test_game_new(&game);
    int setsuna;
    setsuna_setup(&game, &setsuna);
    int filler_l = test_new_id(&game, FILLER);
    int filler_c = test_new_id(&game, FILLER);
    int filler_r = test_new_id(&game, FILLER);
    int arr2 = test_new_id(&game, ARRIVER);
    int arr3 = test_new_id(&game, FILLER);   /* ability-free arriver */

    /* Fire 1 (self). */
    baton_touch(&game, filler_c, setsuna, 1);
    resolve_accepting_optionals(&game);
    CHECK_EQ(hand_len(&game), 1, "first firing draws");
    CHECK_EQ(setsuna_use_count(&game, setsuna), 1, "one use recorded");

    /* Fire 2 (other member baton-touches onto an occupied Left). */
    game.state.p[0].stage[0] = filler_l;
    test_add_to_hand(&game, arr2);
    test_play_to_stage(&game, arr2, 0);
    resolve_accepting_optionals(&game);
    CHECK_EQ(hand_len(&game), 2, "second firing draws (hand 1+1-1+1)");
    CHECK_EQ(setsuna_use_count(&game, setsuna), 2, "two uses recorded");

    /* Fire 3: budget exhausted — the arrival happens, no draw. */
    game.state.p[0].stage[2] = filler_r;
    test_add_to_hand(&game, arr3);
    test_play_to_stage(&game, arr3, 2);
    resolve_accepting_optionals(&game);
    CHECK_EQ(hand_len(&game), 2,
             "third arrival in the same turn: ターン2回 is spent, no draw");
    CHECK_EQ(setsuna_use_count(&game, setsuna), 2, "and no third use is recorded");
}

static void test_setsuna_normal_debut_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);
    int setsuna;
    setsuna_setup(&game, &setsuna);
    int filler_l = test_new_id(&game, FILLER);

    /* Left occupied, Right EMPTY → the arrival to Right is a plain debut. */
    game.state.p[0].stage[0] = filler_l;
    test_add_to_hand(&game, setsuna);
    test_play_to_stage(&game, setsuna, 2);
    resolve_accepting_optionals(&game);

    CHECK_EQ(hand_len(&game), 0, "a plain debut (no baton touch) must not fire");
    CHECK_EQ(setsuna_use_count(&game, setsuna), 0, "and no use is recorded");
}

static void test_other_member_baton_arrival_draws_one(void)
{
    static TestGame game;
    test_game_new(&game);
    int setsuna = test_id(&game, SETSUNA);
    int filler  = test_id(&game, FILLER);
    int arriver = test_new_id(&game, ARRIVER);
    append_twenty_filler(&game);

    clear_stage(&game, 0);
    clear_hand(&game);
    game.state.p[0].stage[0] = setsuna;
    baton_touch(&game, filler, arriver, 1);
    resolve_accepting_optionals(&game);

    CHECK_EQ(hand_len(&game), 1,
             "せつ菜 ab#0 draws 1 when ANOTHER member debuts via baton touch");
}

/* ===================================================================== */
/* M. debut_watch / third_debut_this_turn_draws_until_hand_five +           */
/*     third_debut_draws_and_second_debut_live_start_score                 */
/*     PL!N-bp3-005 宮下 愛 ab#0 「自分のステージにメンバーが3枚登場したと  */
/*     き、手札が5枚になるまでカードを引く」 (Q160/Q161/Q162)              */
/* ===================================================================== */

/* hand = ai + 2 members + hand_extra, deck = deck_count. Returns ai. */
static int ai_bp3_setup(TestGame *tg, int hand_extra, int deck_count)
{
    int ai  = test_id(tg, AI_P2);
    int mem = test_id(tg, FILLER);
    clear_hand(tg);
    clear_deck(tg);
    clear_waitroom(tg);
    tg->state.debut_count_this_turn[0] = 0;
    test_give_energy(tg, 30);
    test_add_to_hand(tg, ai);
    test_add_to_hand(tg, mem);
    test_add_to_hand(tg, mem);
    for (int i = 0; i < hand_extra; i++) test_add_to_hand(tg, mem);
    for (int i = 0; i < deck_count; i++) test_add_to_deck_pl(tg, 0, mem);
    return ai;
}

static void test_ai_bp3_three_deploys_draws_to_five(void)
{
    static TestGame game;
    test_game_new(&game);
    int ai = ai_bp3_setup(&game, 3, 10);
    int mem = test_id(&game, FILLER);
    int deck_before = deck_len(&game);
    int wait_before = wait_len(&game);

    test_play_to_stage(&game, mem, 0);
    test_play_to_stage(&game, mem, 1);
    test_play_to_stage(&game, ai, 2);

    CHECK_EQ(p1_debut_count(&game), 3, "Q161: the count reaches 3 and includes 宮下 愛's own debut");
    CHECK_EQ(hand_len(&game), 5, "the 自動 draws until the hand holds 5");
    CHECK_EQ(deck_len(&game), deck_before - 2, "the deck loses exactly the 2 cards drawn");
    CHECK_EQ(wait_len(&game), wait_before, "nothing is discarded — the 自動 has no cost");
    CHECK_EQ(game.state.p[0].stage[2], ai, "宮下 愛 is on Right");
}

static void test_ai_bp3_only_two_deploys_no_draw(void)
{
    static TestGame game;
    test_game_new(&game);
    int ai = ai_bp3_setup(&game, 2, 10);
    int mem = test_id(&game, FILLER);
    int deck_before = deck_len(&game);

    test_play_to_stage(&game, mem, 0);
    test_play_to_stage(&game, ai, 1);

    CHECK_EQ(p1_debut_count(&game), 2, "only 2 debuits happened");
    CHECK_EQ(hand_len(&game), 3, "『メンバーが3枚』 is false, so nothing is drawn");
    CHECK_EQ(deck_len(&game), deck_before, "and the deck is untouched");
}

static void test_ai_bp3_draw_from_empty_hand_and_empty_deck(void)
{
    static TestGame game;
    test_game_new(&game);
    int ai = ai_bp3_setup(&game, 0, 10);     /* hand = 3, all deployed */
    int mem = test_id(&game, FILLER);
    int deck_before = deck_len(&game);

    test_play_to_stage(&game, mem, 0);
    test_play_to_stage(&game, mem, 1);
    test_play_to_stage(&game, ai, 2);

    CHECK_EQ(hand_len(&game), 5, "the boundary: from an EMPTY hand the draw fills to 5");
    CHECK_EQ(deck_len(&game), deck_before - 5, "and the deck loses exactly 5");

    /* The other edge: an empty deck must not hang and must not draw. */
    static TestGame empty;
    test_game_new(&empty);
    int ai2 = ai_bp3_setup(&empty, 3, 0);
    int mem2 = test_id(&empty, FILLER);
    CHECK_EQ(deck_len(&empty), 0, "precondition: the deck really is empty");
    test_play_to_stage(&empty, mem2, 0);
    test_play_to_stage(&empty, mem2, 1);
    test_play_to_stage(&empty, ai2, 2);
    CHECK_EQ(hand_len(&empty), 3, "an empty deck yields no card and consumes nothing");
    CHECK_EQ(deck_len(&empty), 0, "and the deck stays empty");
}

static void test_ai_bp3_q160_displaced_debuts_still_count(void)
{
    static TestGame game;
    test_game_new(&game);
    int ai  = test_id(&game, AI_P);
    int mem = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(ai, AI_P), "the watcher is the PL!N-bp3-005-P print");

    clear_stage(&game, 0);
    clear_hand(&game);
    clear_deck(&game);
    clear_waitroom(&game);
    game.state.debut_count_this_turn[0] = 0;
    test_give_energy(&game, 25);
    test_add_to_hand(&game, mem);
    test_add_to_hand(&game, mem);
    test_add_to_hand(&game, ai);
    for (int i = 0; i < 10; i++) test_add_to_deck_pl(&game, 0, mem);

    test_play_to_stage(&game, mem, 0);
    CHECK_EQ(p1_debut_count(&game), 1, "debut #1 counted");
    test_play_to_stage(&game, mem, 1);
    CHECK_EQ(p1_debut_count(&game), 2, "debut #2 counted");

    /* The second debuted member genuinely leaves the stage. */
    int removed = game.state.p[0].stage[1];
    game.state.p[0].stage[1] = RB_EMPTY_SLOT;
    rb_waitroom_add(&game.state.p[0], removed);
    rb_record_card_movement(&game.state, removed, RB_ZONEID_STAGE,
                            RB_ZONEID_DISCARD, 0, 0);
    CHECK_EQ(p1_debut_count(&game), 2, "the departure does not decrement the count");

    test_play_to_stage(&game, ai, 2);
    CHECK_EQ(p1_debut_count(&game), 3,
             "Q160: the count still reaches 3 even though a debuted member left");
    CHECK_EQ(game.state.p[0].stage[2], ai, "宮下 愛 is at Right");
    CHECK_EQ(hand_len(&game), 5, "Q160: the 自動 fires and draws to 5");
}

/* ===================================================================== */
/* N. debut_watch / bare_draw_and_zone_disjunction_test.rs                 */
/*     平安名すみれ ライブ成功時 — an OR of two zone conditions → draw 1.    */
/*     大沢瑠璃乃 登場 — 「カードを1枚引く」 at the DECK EDGE.              */
/* ===================================================================== */

/* Returns how many cards reached hand; asserts the hand grew by exactly what
 * the deck lost, so a copy-instead-of-move implementation fails here. */
static int sumire_draw_under(TestGame *tg, int revealed_live, int boost_live_zone)
{
    int sumire = test_id(tg, SUMIRE);
    (void)test_id(tg, FILLER);    /* template resolved; the board uses fresh copies */
    clear_stage(tg, 0);
    clear_live(tg);
    clear_hand(tg);
    tg->state.n_revealed = 0;
    tg->state.p[0].stage[0] = sumire;

    int live = test_new_id(tg, LIVE_MU);
    test_add_to_live(tg, live);
    if (boost_live_zone) rb_mods_add_score(&tg->state.mods, live, 5);
    if (revealed_live) {
        int rl = test_new_id(tg, LIVE_MU);
        rb_mods_add_score(&tg->state.mods, rl, 5);
        tg->state.revealed_cards[0] = rl;
        tg->state.n_revealed = 1;
    }
    for (int i = 0; i < 3; i++) test_add_to_hand(tg, test_new_id(tg, FILLER));
    int hand_before = hand_len(tg);
    int deck_before = deck_len(tg);

    fire_trigger(tg, sumire, RB_TSTR_LIVE_SUCCESS);
    drain_targets(tg);

    int drawn = hand_len(tg) - hand_before;
    int lost = deck_before - deck_len(tg);
    CHECK_EQ(drawn, lost,
             "the hand grew by exactly what the deck lost — a draw moves one card, "
             "it does not duplicate or source it elsewhere");
    return drawn;
}

static void test_sumire_draws_when_either_zone_condition_alone_holds(void)
{
    static TestGame game;
    test_game_new(&game);
    int filler = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(test_id(&game, SUMIRE), SUMIRE),
          "the draw host is the PL!SP-pb2-004-R (平安名すみれ) print");
    clear_deck(&game);
    game.state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) { test_add_to_deck_pl(&game, 0, filler); test_add_to_deck_pl(&game, 1, filler); }

    CHECK_EQ(sumire_draw_under(&game, 0, 0), 0,
             "NEITHER clause: nothing is drawn, so the printed comma is not an AND "
             "that happens to be satisfied");
    CHECK_EQ(sumire_draw_under(&game, 0, 1), 1,
             "「元々のスコアより高いスコアのライブカードがあるか」 alone satisfies the OR");
    CHECK_EQ(sumire_draw_under(&game, 1, 0), 1,
             "「エールにより公開された…スコアを持つライブカードがある場合」 alone "
             "satisfies the OR — the case an AND reading fails");
    CHECK_EQ(sumire_draw_under(&game, 1, 1), 1,
             "both clauses true still draws exactly ONE card — the printed text is a "
             "single draw guarded by a disjunction, not one draw per branch");
}

static void test_rurino_draws_one_card_and_nothing_from_an_empty_deck(void)
{
    static TestGame stocked;
    test_game_new(&stocked);
    int rurino = test_id(&stocked, RURINO_D);
    int filler = test_id(&stocked, FILLER);
    CHECK(rb_card_no_eq(rurino, RURINO_D), "the draw host is the PL!HS-bp5-011-N print");
    clear_deck(&stocked);
    clear_hand(&stocked);
    for (int i = 0; i < 5; i++) test_add_to_deck_pl(&stocked, 0, filler);
    test_add_to_hand(&stocked, rurino);
    test_give_energy(&stocked, 20);
    CHECK_EQ(deck_len(&stocked), 5, "precondition: a stocked deck");

    test_play_to_stage(&stocked, rurino, 1);
    drain_targets(&stocked);

    CHECK_EQ(hand_len(&stocked), 1,
             "「カードを1枚引く」 — exactly one card, and it came from the deck");
    CHECK_EQ(deck_len(&stocked), 4, "5 - 1 = 4: a real deck→hand move, not a hand-only shuffle");

    static TestGame empty;
    test_game_new(&empty);
    int rurino2 = test_id(&empty, RURINO_D);
    (void)test_id(&empty, FILLER);
    clear_deck(&empty);
    clear_hand(&empty);
    for (int i = 0; i < 3; i++) test_add_to_hand(&empty, test_new_id(&empty, FILLER));
    test_add_to_hand(&empty, rurino2);
    test_give_energy(&empty, 20);
    CHECK_EQ(deck_len(&empty), 0, "precondition: the deck really is empty");

    test_play_to_stage(&empty, rurino2, 1);
    drain_targets(&empty);

    CHECK_EQ(hand_len(&empty), 3,
             "an empty deck yields no card and consumes nothing — a fallback to the "
             "waitroom or the hand would show up here");
    CHECK_EQ(wait_len(&empty), 0, "and nothing was taken from the waitroom as a substitute");
}

/* ===================================================================== */
/* O. debut_watch / deck_contents_decide_debut_branch_test.rs              */
/*     徒町小鈴 「デッキの上からカードを3枚控え室に置く。それらがすべて      */
/*     メンバーカードの場合、カードを1枚引く。」                            */
/*     津島善子 mill 10 / 村野さやか mill 5 — requested size vs deck size.  */
/* ===================================================================== */

static void kosuzu_run(TestGame *tg, const int *top, int n_top, int tail,
                       int *out_deck, int *out_wait, int *out_hand)
{
    int kosuzu = test_id(tg, KOSUZU);
    int filler = test_id(tg, FILLER);
    clear_hand(tg);
    clear_deck(tg);
    clear_waitroom(tg);
    clear_stage(tg, 0);
    clear_recently_moved(tg);
    test_give_energy(tg, 10);
    for (int i = 0; i < n_top; i++) test_add_to_deck_pl(tg, 0, top[i]);
    for (int i = 0; i < tail; i++) test_add_to_deck_pl(tg, 0, filler);
    test_add_to_hand(tg, kosuzu);
    test_play_to_stage(tg, kosuzu, 1);
    drain_targets(tg);
    *out_deck = deck_len(tg);
    *out_wait = wait_len(tg);
    *out_hand = hand_len(tg);
}

static void test_kosuzu_branch_is_decided_by_the_milled_cards(void)
{
    /* The ALL-members branch: three members on top, so the draw runs. */
    static TestGame all;
    test_game_new(&all);
    int m1 = test_new_id(&all, FILLER), m2 = test_new_id(&all, FILLER), m3 = test_new_id(&all, FILLER);
    int top_all[3] = { m1, m2, m3 };
    int d, w, h;
    kosuzu_run(&all, top_all, 3, 10, &d, &w, &h);
    /* DIVERGENCE FROM THE RUST EXPECTATION, reported rather than copied.
     * deck_contents_decide_debut_branch_test.rs stacks 13 cards, calls
     * `add_to_hand(kosuzu)` (which does NOT take from the deck) and then
     * asserts `deck == 9` with the comment "13 - 1 - 3 = 9" — it counts the
     * HAND card as if it had left the deck. 瑠璃乃 is played FROM HAND, so the
     * deck loses exactly the 3 milled cards: 13 - 3 = 10. The C assertion is
     * the arithmetic the fixture actually implies; the Rust "9" is a
     * suspected off-by-one in that test. See the report. */
    CHECK_EQ(d, 10, "playing her from hand does not touch the deck, and her 登場 mills 3: 13 - 3 = 10");
    CHECK_EQ(w, 3, "exactly the three milled cards are in the waitroom");
    CHECK_EQ(h, 1, "『それらがすべてメンバーカードの場合』 holds, so one card is drawn");

    /* The NEGATIVE branch: ONE ライブ among the three, same mill. */
    static TestGame some;
    test_game_new(&some);
    int s1 = test_new_id(&some, FILLER);
    int nm = test_new_id(&some, LIVE_MU);
    int s3 = test_new_id(&some, FILLER);
    int top_some[3] = { s1, nm, s3 };
    kosuzu_run(&some, top_some, 3, 10, &d, &w, &h);
    CHECK_EQ(w, 3, "the same three cards are milled either way — the first clause "
                   "does not depend on the branch");
    CHECK_EQ(h, 0, "『すべてメンバーカード』 is false with one ライブ among the three, "
                   "so NO card is drawn — the quantifier assertion");

    /* A deck with FEWER than three cards. */
    static TestGame two;
    test_game_new(&two);
    int t1 = test_new_id(&two, FILLER), t2 = test_new_id(&two, FILLER);
    int top_two[2] = { t1, t2 };
    kosuzu_run(&two, top_two, 2, 0, &d, &w, &h);
    CHECK_EQ(d, 0, "an under-supplied mill empties the deck rather than refusing");
    CHECK_EQ(h, 0, "and a short mill is not read as a satisfied one — no draw");
}

static void test_mill_sizes_are_honoured_at_the_printed_count(void)
{
    static TestGame ten;
    test_game_new(&ten);
    int yoshiko = test_id(&ten, YOSHIKO);
    int filler  = test_id(&ten, FILLER);
    CHECK(rb_card_no_eq(yoshiko, YOSHIKO), "the miller is the PL!S-bp5-015-N (津島善子) print");
    clear_hand(&ten); clear_deck(&ten); clear_waitroom(&ten); clear_stage(&ten, 0);
    test_give_energy(&ten, 20);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&ten, 0, filler);
    test_add_to_hand(&ten, yoshiko);
    test_play_to_stage(&ten, yoshiko, 1);
    drain_targets(&ten);
    CHECK_EQ(wait_len(&ten), 10, "『デッキの上からカードを10枚控え室に置く』 moves exactly 10");
    CHECK_EQ(deck_len(&ten), 10, "20 - 10 (milled) = 10");

    /* Short deck: three available, ten requested. */
    static TestGame short_deck;
    test_game_new(&short_deck);
    int y2 = test_id(&short_deck, YOSHIKO);
    int f2 = test_id(&short_deck, FILLER);
    clear_hand(&short_deck); clear_deck(&short_deck); clear_waitroom(&short_deck); clear_stage(&short_deck, 0);
    test_give_energy(&short_deck, 20);
    for (int i = 0; i < 3; i++) test_add_to_deck_pl(&short_deck, 0, f2);
    test_add_to_hand(&short_deck, y2);
    test_play_to_stage(&short_deck, y2, 1);
    drain_targets(&short_deck);
    CHECK_EQ(deck_len(&short_deck), 0,
             "an under-supplied mill empties the deck rather than refusing");
    CHECK_EQ(wait_len(&short_deck), 3,
             "and the waitroom receives those three — not ten. The reading the "
             "text does not state, pinned so a change to it is deliberate");

    /* A different requested size, so the pair separates 'the mill runs' from
     * 'the mill runs at the printed size'. */
    static TestGame five;
    test_game_new(&five);
    int sayaka = test_id(&five, SAYAKA_MILL);
    int f5     = test_id(&five, FILLER);
    CHECK(rb_card_no_eq(sayaka, SAYAKA_MILL),
          "the second miller is the PL!HS-bp2-011-PR (村野さやか) print");
    clear_hand(&five); clear_deck(&five); clear_waitroom(&five); clear_stage(&five, 0);
    test_give_energy(&five, 10);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(&five, 0, f5);
    test_add_to_hand(&five, sayaka);
    test_play_to_stage(&five, sayaka, 1);
    drain_targets(&five);
    CHECK_EQ(wait_len(&five), 5,
             "『デッキの上からカードを5枚控え室に置く』 moves 5 — 津島善子's 10 from the "
             "same fixture is what shows the printed size is honoured");
    CHECK_EQ(deck_len(&five), 15, "20 - 5 (milled) = 15");
}

/* ===================================================================== */
/* P. debut_watch / simultaneous_triggers_active_player_resolves_first     */
/*     Q84: when auto abilities trigger simultaneously, the ordering of    */
/*     two DIFFERENT watchers inside one action.                          */
/* ===================================================================== */

static void test_q84_baton_touch_appearance_triggers_resolve_ordered(void)
{
    static TestGame game;
    test_game_new(&game);
    int hanafu   = test_id(&game, HANAFU);
    int seras    = test_id(&game, CERAS_R);
    int edelnote = test_new_id(&game, CERAS_SD);   /* cost 4, no baton-touch trigger */
    int opp      = test_new_id(&game, FILLER);
    int filler   = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(hanafu, HANAFU), "the baton-touch watcher is the PL!HS-sd1-001-SD print");
    CHECK(rb_card_matches_group_str(hanafu, "スリーズブーケ"),
          "花帆 really is a 『スリーズブeke』 member");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    clear_hand(&game);
    clear_deck(&game);
    for (int i = 0; i < 40; i++) test_add_to_deck_pl(&game, 0, filler);
    test_give_energy(&game, 20);
    game.state.p[0].stage[0] = seras;
    game.state.p[0].stage[1] = hanafu;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].stage[0] = opp;

    int e_before = active_energy(&game);
    test_add_to_hand(&game, edelnote);
    test_play_to_stage(&game, edelnote, 1);
    resolve_accepting_optionals(&game);

    CHECK_EQ(game.state.p[0].stage[1], edelnote,
             "the cost-4 EdelNote print really did baton-touch 花帆 off Center");
    CHECK_EQ(active_energy(&game), e_before,
             "花帆 requires a cost>=10 『スリーズブeke』 partner, and the arriver costs 4, "
             "so her 自動 must not activate any energy");
    CHECK(is_wait(&game, opp),
          "セラス fires on the same 『EdelNote』 appearance and waits the opponent");
}

/* ===================================================================== */
/* per-test isolation: forked child, signal handler naming the test in     */
/* flight. NO watchdog — an in-child alarm() never fires on this toolchain */
/* and a parent-side waitpid(WNOHANG) deadline's kill(SIGKILL) does not take */
/* effect, so one would be dead code that only pretends to protect the run. */
/* ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_SETUPBUG  3

static int n_tests_ok, n_tests_failed, n_tests_crashed, n_tests_setup;

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
        int a0 = assertions, f0 = failures, s0 = n_setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0), ns = (int)(n_setup_bugs - s0);
        if (nf) n_tests_failed++;
        else if (ns) n_tests_setup++;
        else n_tests_ok++;
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
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_tests_setup++;
        printf("%-8s %s  <-- SETUP BUG (fixture could not be built)\n", "SETUPBUG", name);
    } else {
        n_tests_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
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
        return 2;
    }

    printf("--- §A discard_watch / on_hand_to_discard_each_time_gain_heart01_and_blade ---\n");
    run("rurino_use_limit_blocks_second_same_turn",
        test_rurino_use_limit_blocks_second_same_turn);

    printf("--- §B discard_watch / hand_discard_batch_heart_blade_q241 ---\n");
    run("hand_discard_batch_q241_two_discarded_auto_fires_once",
        test_hand_discard_batch_q241_two_discarded_auto_fires_once);
    run("hand_discard_batch_q241_three_discarded_fires_once",
        test_hand_discard_batch_q241_three_discarded_fires_once);
    run("hand_discard_batch_q241_zero_discarded_no_trigger",
        test_hand_discard_batch_q241_zero_discarded_no_trigger);

    printf("--- §C discard_watch / live_start_heart_requirement_reduction (Q213) ---\n");
    run("hanamusubi_q213_member_card_moved_before_live_start",
        test_hanamusubi_q213_member_card_moved_before_live_start);

    printf("--- §D discard_watch / deck_to_discard_optional_hand_discard_recover_self ---\n");
    run("deck_to_discard_self_recovery_accept_discards_one_and_recovers_self",
        test_deck_to_discard_self_recovery_accept_discards_one_and_recovers_self);
    run("deck_to_discard_self_recovery_skip_no_discard_no_recover",
        test_deck_to_discard_self_recovery_skip_no_discard_no_recover);
    run("deck_to_discard_self_recovery_empty_hand_no_recover",
        test_deck_to_discard_self_recovery_empty_hand_no_recover);
    run("deck_to_discard_self_recovery_recovers_only_self",
        test_deck_to_discard_self_recovery_recovers_only_self);
    run("deck_to_discard_self_recovery_does_not_fire_for_other_card",
        test_deck_to_discard_self_recovery_does_not_fire_for_other_card);
    run("deck_to_discard_self_recovery_does_not_fire_on_hand_to_discard",
        test_deck_to_discard_self_recovery_does_not_fire_on_hand_to_discard);

    printf("--- §E discard_watch / real_mill_optional_discard_recover_self ---\n");
    run("real_deck_top_mill_offers_discard_and_recovers_self",
        test_real_deck_top_mill_offers_discard_and_recovers_self);
    run("real_hand_discard_does_not_trigger_deck_to_discard_self_recovery",
        test_real_hand_discard_does_not_trigger_deck_to_discard_self_recovery);

    printf("--- §F discard_watch / deck_to_discard_self_recovery_yell_refresh (Q269) ---\n");
    run("q269_yell_reveal_does_not_trigger_deck_to_discard_recovery",
        test_q269_yell_reveal_does_not_trigger_deck_to_discard_recovery);
    run("q269_control_deck_to_discard_triggers",
        test_q269_control_deck_to_discard_triggers);

    printf("--- §G discard_watch / Q252 桜内梨子 own live-zone discard ---\n");
    run("riko_q252_no_movement_no_trigger", test_riko_q252_no_movement_no_trigger);
    run("riko_q252_no_live_cards_anywhere_no_trigger", test_riko_q252_no_live_cards_anywhere_no_trigger);
    run("riko_q252_non_aqours_live_alone_no_trigger", test_riko_q252_non_aqours_live_alone_no_trigger);
    run("riko_q252_empty_and_null_moved_cards_no_trigger", test_riko_q252_empty_and_null_moved_cards_no_trigger);
    run("riko_q252_watcher_off_stage_no_trigger", test_riko_q252_watcher_off_stage_no_trigger);
    run("riko_q252_mixed_batch_filters_correctly", test_riko_q252_mixed_batch_filters_correctly);
    run("riko_q252_single_card_deck_top_exact_identity", test_riko_q252_single_card_deck_top_exact_identity);
    run("riko_q252_turn_limit_blocks_second_trigger", test_riko_q252_turn_limit_blocks_second_trigger);
    run("riko_q252_decline_keeps_cards_in_waitroom", test_riko_q252_decline_keeps_cards_in_waitroom);
    run("riko_q252_r_plus_printing_selects_the_second_card", test_riko_q252_r_plus_printing_selects_the_second_card);

    printf("--- §H debut_watch / ally_member_debut_pay_energy_or_center_ally_blade ---\n");
    run("hana_001_ally_appears_offers_payment", test_hana_001_ally_appears_offers_payment);
    run("hana_001_non_matching_ally_no_trigger", test_hana_001_non_matching_ally_no_trigger);
    run("hana_001_self_play_no_trigger", test_hana_001_self_play_no_trigger);
    run("hana_009_center_ally_appears_gains_blade", test_hana_009_center_ally_appears_gains_blade);
    run("hana_009_p_plus_center_ally_appears_gains_blade", test_hana_009_p_plus_center_ally_appears_gains_blade);
    run("hana_009_not_center_no_trigger", test_hana_009_not_center_no_trigger);
    run("hana_009_q245_self_deploy_to_center_triggers", test_hana_009_q245_self_deploy_to_center_triggers);
    run("hana_009_q245_self_deploy_non_center_no_trigger", test_hana_009_q245_self_deploy_non_center_no_trigger);

    printf("--- §I debut_watch / jump_up_high_multi_target_blade_grant_lifecycle ---\n");
    run("jump_up_high_gives_each_aqours_member_exactly_one_blade",
        test_jump_up_high_gives_each_aqours_member_exactly_one_blade);
    run("jump_up_high_does_not_reach_a_late_arrival_nor_outlive_a_departure",
        test_jump_up_high_does_not_reach_a_late_arrival_nor_outlive_a_departure);
    run("jump_up_high_grant_is_scoped_to_the_live_card",
        test_jump_up_high_grant_is_scoped_to_the_live_card);

    printf("--- §J debut_watch / three_way_choices (絢瀬絵里 / 若菜四季) ---\n");
    run("ericho_gains_only_the_chosen_heart_two_per_success_card",
        test_ericho_gains_only_the_chosen_heart_two_per_success_card);
    run("ericho_gains_nothing_with_an_empty_success_zone",
        test_ericho_gains_nothing_with_an_empty_success_zone);
    run("wakana_the_chosen_heart_replaces_her_printed_hearts",
        test_wakana_the_chosen_heart_replaces_her_printed_hearts);

    printf("--- §K debut_watch / edel_note_ally_debut_waits_opponent_active_member (Q250) ---\n");
    run("edel_note_waits_lone_active_opponent_member", test_edel_note_waits_lone_active_opponent_member);
    run("edel_note_waits_the_selected_opponent_member_only", test_edel_note_waits_the_selected_opponent_member_only);
    run("edel_note_without_recent_debut_offers_no_wait", test_edel_note_without_recent_debut_offers_no_wait);
    run("edel_note_turn_limit_blocks_second_trigger", test_edel_note_turn_limit_blocks_second_trigger);
    run("edel_note_p_self_debut_waits_lone_opponent_member", test_edel_note_p_self_debut_waits_lone_opponent_member);

    printf("--- §L debut_watch / baton_touch_arrival_self_or_other_draws_one (せつ菜) ---\n");
    run("setsuna_self_baton_arrival_draws_one", test_setsuna_self_baton_arrival_draws_one);
    run("setsuna_turn2_budget_two_draws_third_refused", test_setsuna_turn2_budget_two_draws_third_refused);
    run("setsuna_normal_debut_does_not_fire", test_setsuna_normal_debut_does_not_fire);
    run("other_member_baton_arrival_draws_one", test_other_member_baton_arrival_draws_one);

    printf("--- §M debut_watch / third_debut_this_turn_draws_until_hand_five (宮下 愛) ---\n");
    run("ai_bp3_three_deploys_draws_to_five", test_ai_bp3_three_deploys_draws_to_five);
    run("ai_bp3_only_two_deploys_no_draw", test_ai_bp3_only_two_deploys_no_draw);
    run("ai_bp3_draw_from_empty_hand_and_empty_deck", test_ai_bp3_draw_from_empty_hand_and_empty_deck);
    run("ai_bp3_q160_displaced_debuts_still_count", test_ai_bp3_q160_displaced_debuts_still_count);

    printf("--- §N debut_watch / bare_draw_and_zone_disjunction ---\n");
    run("sumire_draws_when_either_zone_condition_alone_holds",
        test_sumire_draws_when_either_zone_condition_alone_holds);
    run("rurino_draws_one_card_and_nothing_from_an_empty_deck",
        test_rurino_draws_one_card_and_nothing_from_an_empty_deck);

    printf("--- §O debut_watch / deck_contents_decide_debut_branch ---\n");
    run("kosuzu_branch_is_decided_by_the_milled_cards", test_kosuzu_branch_is_decided_by_the_milled_cards);
    run("mill_sizes_are_honoured_at_the_printed_count", test_mill_sizes_are_honoured_at_the_printed_count);

    printf("--- §P debut_watch / simultaneous_triggers_active_player_resolves_first (Q84) ---\n");
    run("q84_baton_touch_appearance_triggers_resolve_ordered",
        test_q84_baton_touch_appearance_triggers_resolve_ordered);

    rb_unload();

    /* The assertion / failure / setup-bug counters live in the CHILD (each case
     * runs in its own forked process), so the parent's own counters are
     * structurally 0. The summary must therefore be built from the per-case
     * verdicts, not from `failures`. */
    int n_run = n_tests_ok + n_tests_failed + n_tests_crashed + n_tests_setup;
    printf("\n==== parity_jidou_watch: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_tests_ok, n_tests_failed, n_tests_crashed, n_tests_setup);
    printf("     per-assertion counts and any GAP lines are printed by each child "
           "above.\n");
    if (n_tests_crashed) {
        fprintf(stderr, "%d test case(s) CRASHED — a fatal signal in a child "
                        "(a SIGSEGV/SIGBUS/SIGABRT handler in the child names the "
                        "case). Treat as an engine fault.\n", n_tests_crashed);
    }
    if (n_tests_setup) {
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n",
                n_tests_setup);
    }
    if (n_tests_failed) {
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file "
                        "is classified in the header comment (E1..E13 = engine "
                        "bugs, H1 = C harness limit); none is a weakened "
                        "assertion.\n", n_tests_failed);
        return 1;
    }
    printf("ALL JIDOU-WATCH PARITY CHECKS PASSED\n");
    return 0;
}

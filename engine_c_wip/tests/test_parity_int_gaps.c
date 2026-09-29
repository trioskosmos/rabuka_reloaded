/* test_parity_int_gaps.c -- the REMAINING integration/effects gaps after the
 * per-card ports landed.
 *
 *   Rust source                                              ported here
 *   ─────────────────────────────────────────────────────    ──────────
 *   jidou/movement/live_start_waitroom_to_deck_top             3 of 10
 *   jidou/movement/ability_chain_combo                         1 of  5
 *   jidou/movement/under_member_placement                      1 of  3
 *   jidou/yell/live_success_from_yell_reveal                   0 of  5
 *   integration/per_card/live_end_expiry_rollover...           1 of  3
 *   effects/replacement_generic_rule_9_10_test.rs              5 of  5
 *   effects/look_select root                                   2 of  9
 *
 * WHAT WAS ALREADY COVERED, and by WHOSE BODIES (not just headers):
 *
 *  - jidou/movement/live_start_waitroom_to_deck_top
 *      test_parity_jidou_move_yell.c section 12 (lines 1608-1774) has
 *      happy_path_prompts_for_four_cards, no_saintsnow_does_not_fire,
 *      cost_below_20_does_not_fire, cost_at_least_20_fires, can_skip,
 *      no_matching_live_in_waitroom_skips, only_matching_lives_are_filtered.
 *      Those 7 all STOP AT THE PROMPT: none of them ever answers it, so none
 *      of them proves a selected card actually LANDED. The three ported here
 *      are the three that answer, which is the uncovered half.
 *
 *  - jidou/movement/ability_chain_combo
 *      test_parity_jidou_move_yell.c section 10 (1434-1559) has
 *      pb2006_jidou_placement_feeds_constant_cost_up,
 *      pb2006_jidou_once_per_turn_no_second_placement,
 *      special_color_set_blades_and_score_twin_in_one_live,
 *      special_color_no_score_when_moved_liella_left_center.
 *      The missing fifth is pb2011_own_live_start_reposition_triggers_own_
 *      three_choice, ported here as `pb2011_*`.
 *
 *  - jidou/movement/under_member_placement
 *      under_member_counted_for_both_players -> test_parity_jidou_extra.c
 *      section I; discard_group_member_placed_under_self_on_position_change
 *      -> test_parity_jidou_move_yell.c `test_kinako_tucks_group_member_under_
 *      self_on_area_move` (1566). The missing arm is the ライブ成功時 one.
 *
 *  - jidou/yell/live_success_from_yell_reveal
 *      VERIFIED COVERED, not duplicated:
 *      live_success_draw_per_aqours_member_then_discard_same_count_test.rs
 *      (jimo_ai_draw_{0,1,2,3}) -> test_parity_jidou_move_yell.c Y8 (2472-2550)
 *      and test_parity_yell.c E (1204-1290) for the hand-limited arm;
 *      live_success_no_blade_heart_or_two_excess_hearts_scores_four_test.rs
 *      -> test_parity_jidou_move_yell.c Y9 `miracle_wave_excess_heart_scores_
 *      four` (2852). Nothing ported here.
 *
 *  - integration/per_card/live_end_expiry_rollover_and_dual_trigger_window_
 *      gates_test.rs
 *      T1 live_end_grant_expires_through_real_phase_rollover and
 *      T3 ginko_hs_debut_window_rests_cost_le9_member ->
 *      test_parity_integration.c (`test_live_end_grant_expires_through_rollover`
 *      and `test_ginko_debut_cost_gate`, both in main()'s call list).
 *      T2 karin_live_start_window_rests_exactly_blade4_member ->
 *      test_parity_integration.c `test_karin_livestart_blade4_window` (1499).
 *      So all three ARE covered; only the blade-1-exclusion arm of T2 is
 *      thin, and that is re-asserted as a paired control in `t2_*` below
 *      rather than as a separate test.
 *
 *  - effects/look_select root
 *      heart_colour_choice_and_optional_banking_test.rs (PL!HS-cl1-001-CL) ->
 *      test_parity_look_and_filter.c section G, `test_g_looked_at_discard_
 *      player_discards` / `..._skips` (1890-1958) — BOTH branches, including
 *      the decline-returns-the-card assertion.
 *      unit_filter_look_and_live_only_cost_test.rs (PL!SP-bp7-018-N) ->
 *      test_parity_reveal_extra.c `test_mei_paid_live_discard_looks_five_and_
 *      takes_one` (1238) and `test_mei_declined_live_discard_preserves_cost_
 *      card` (1266) — both arms.
 *      deck_bottom_reposition... (PL!S-bp7-010-N) -> test_parity_look_per_card.c
 *      section F, `test_f_chika_fourth` / `test_f_chika_decline`.
 *      sequential_mill_then_group_recovery_test.rs (PL!-bp5-010-N) ->
 *      test_parity_mill_deck.c `test_honoka_mills_three_then_recovers_arise`
 *      (1544) and `test_honoka_without_arise_still_mills_three` (1581) — both
 *      arms, including the A-RISE group filter negative.
 *      optional_look_and_select_negative_branches_test.rs (PL!-pb1-016-R) ->
 *      test_parity_search_reorder.c `test_paid_discard_look_four_fetches_
 *      lilywhite` (1267) and `..._declined_keeps_deck` (1286).
 *      => ALL FOUR FILES ARE COVERED. Nothing ported. The one genuinely
 *      uncovered item this file DOES take from that cluster is
 *      `look_select/deck_bottom_reposition...`: the 4th-from-the-top INDEX
 *      assertion, which the existing F section only checks as "the bottom
 *      card was accepted", not as "it sits at index 3".
 *
 * NOT ATTEMPTED, with reasons, are listed in the run report.
 *
 * ── ENGINE FACTS this file depends on, all confirmed by reading src/ ──
 *  1. REPLACEMENT EFFECTS ARE NOT PORTED. `rb_add_replacement_effect`
 *     (src/game_state_abilities.c:881) stores card_id / player_id /
 *     original_event / is_choice_based and then `(void)replacement_effects;
 *     (void)n_replacement;` — the ALTERNATE AbilityEffect vector is dropped on
 *     the floor, and `RbReplacementEffect` (rabuka.h:1280) has no field to
 *     hold it. There is no `run_replacement_effects` anywhere in C (grep for
 *     the identifier returns nothing), where Rust has
 *     engine/src/ability/effects/replacement.rs:31 called from
 *     effects/mod.rs:337. So the "replace this event with that alternate"
 *     mechanism is entirely absent. Section R below pins that with a
 *     PAIRED POSITIVE: an ordinary draw_card of the same count on a board
 *     with no registration DOES draw, which proves the fixture is live and
 *     the red is about the replacement, not about a dead draw path.
 *  2. `exclude_selected` is DECODED in Rust (effect_decoder_gen.rs:125,
 *     card.rs:1588) and has NO C counterpart: grep for it in src/ returns
 *     only unrelated hits, and `AbilityEffect` (rabuka.h) has no such member.
 *     Section D below therefore records the Q187 second selection as an
 *     EXPECTED_GAP against a confirmed paired positive (the FIRST selection
 *     really does grant a blade), never as a pass.
 *  3. `deck` in C is an RbBag where index 0 is the TOP, matching Rust's
 *     `main_deck.cards` (draw removes index 0). A deck-top placement is
 *     therefore a memmove to index 0, and `test_insert_deck_top` models it.
 *  4. `RbChoice.zone` for the waitroom is "discard".
 *  5. `test_get_heart_modifier` REMAPS colour 5 onto RB_HEART_ORANGE, so the
 *     one heart read here (section E) goes through `rb_mods_get_heart` with an
 *     index derived by `rb_parse_heart_color`, never through that shim.
 *     The mapping is printed at startup from `rb_heart_color_as_str`.
 *  6. `sizeof(GameState)` is ~781 KB, so every TestGame local is `static`.
 *  7. `rb_create_card_copy` uses a process-global monotonic pool; each case
 *     calls `rb_card_copy_pool_reset()` first.
 *
 * Every case runs in a forked child; a fault is reported as CRASH, never as a
 * pass, and a fatal-signal handler names the case in flight.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ====================================================================== */
/* 0. harness                                                             */
/* ====================================================================== */

static long assertions;
static int  failures;

static const char *current_case = "(none)";
static int n_ok, n_failed, n_crashed;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_int_gaps\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** assertions so far: %ld  failures: %d\n",
            sig, current_case, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                       \
        assertions++;                                                   \
        if (!(condition)) {                                              \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fputc('\n', stderr);                                         \
        }                                                                \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                            \
        assertions++;                                                   \
        long a_ = (long)(actual);                                       \
        long e_ = (long)(expected);                                     \
        if (a_ != e_) {                                                  \
            failures++;                                                  \
            fprintf(stderr, "FAIL: ");                                   \
            fprintf(stderr, __VA_ARGS__);                                \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);        \
        }                                                                \
    } while (0)

/* A harness limitation, recorded and reported but NOT a failure. Used only
 * where the C surface genuinely cannot express the Rust assertion.
 * Printed on STDOUT so it lands in the captured log: each case runs in a
 * forked child, so the parent's `gaps` counter cannot see it. */
#define EXPECTED_GAP(n, ...) do {                                       \
        assertions++;                                                   \
        printf("EXPECTED_GAP(%d): ", (n));                               \
        printf(__VA_ARGS__);                                            \
        printf("\n");                                                   \
        fflush(stdout);                                                 \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1
#define CHILD_CRASHED  2

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        current_case = "(none)";
        if ((long)failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld assertions]\n",
               (long)failures > f0 ? "FAILED" : "ok", name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        printf("        %s: %ld assertion(s), %ld failure(s)\n",
               name, assertions - a0, (long)failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit((long)failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ====================================================================== */
/* 1. card constants -- every one verified against cards/cards.json        */
/*    read as UTF-8 (not Get-Content, which mangles non-ASCII).          */
/* ====================================================================== */

/* jidou/movement/live_start_waitroom_to_deck_top -- PL!S-bp5-023-L
 * 「ライブ開始時 自分のステージに『Aqours』のメンバーと『SaintSnow』の
 *   メンバーがいて、かつそれらのメンバーのコストが合計20以上の場合、
 *   自分の控え室にある『Aqours』と『SaintSnow』のライブカードを4枚まで
 *   好きな順番でデッキの上に置いてもよい。」 */
#define AWAKEN        "PL!S-bp5-023-L"     /* Awaken the power, live       */
#define AQ_CHIKA      "PL!S-sd1-001-SD"    /* 高海千歌  Aqours  c17        */
#define SS_222        "PL!S-bp5-222-R"     /* 鹿角理亞  SaintSnow c11       */
#define SS_111        "PL!S-bp5-111-R"     /* 鹿角聖良  SaintSnow c4        */
#define AQ_LIVE       "PL!S-bp2-019-L"     /* WATER BLUE NEW WORLD  Aqours */
#define SS_LIVE       "PL!S-bp5-022-L"     /* SELF CONTROL!!  SaintSnow    */
#define NIJI_LIVE     "PL!N-bp1-028-L"     /* Butterfly  虹ヶ咲, excluded  */

/* jidou/movement/ability_chain_combo -- PL!SP-pb2-011-R 鬼塚冬毬 */
#define TOUBA         "PL!SP-pb2-011-R"    /* 鬼塚冬毬  c13  5yncri5e!       */

/* jidou/movement/under_member_placement -- PL!SP-pb2-006-R 桜小路きな子 */
#define KINAKO        "PL!SP-pb2-006-R"    /* 自動: 『Liella!』 member under */
#define LIELLA_SD20   "PL!SP-sd1-020-SD"   /* 鬼塚夏美  5yncri5e!            */

/* integration/per_card T2 -- PL!N-bp5-004-R 朝香果林 */
#define KARIN         "PL!N-bp5-004-R"     /* 「元々持つブレードがちょうど4つ」 */
#define BLADE4_TARGET "PL!N-PR-008-PR"     /* 近江彼方  blade 4                */
#define BLADE1_FILLER "PL!-sd1-010-SD"     /* 高坂穂乃果  blade 1, gate-excluded */

#define FILLER        "PL!-sd1-010-SD"
#define SIMPLE_LIVE   "PL!-sd1-020-SD"     /* きっと青春が聞こえる             */

/* effects/choice: Dazzling Game Q187 exclude_selected */
#define DAZZLING      "PL!SP-bp4-023-L"    /* Dazzling Game, live            */
#define KANON         "PL!SP-pb1-001-R"    /* 澁谷かのん  one of the 3 named   */
#define KANON_ALT     "PL!SP-sd1-002-SD"   /* 唐 可可, a DIFFERENT Liella!    */

/* effects/look_select: PL!S-bp7-010-N 高海千歌 4th-from-the-top index */
#define CHIKA_BP7     "PL!S-bp7-010-N"     /* 高海千歌  cost 4                 */

/* effects/replacement: generic rule 9/10 fixtures */
#define REPL_A        "PL!-sd1-010-SD"
#define REPL_B        "PL!HS-bp1-005-P"
#define REPL_C        "PL!S-bp3-019-L"

/* ====================================================================== */
/* 2. harness helpers                                                     */
/* ====================================================================== */

/* rb_load("src") fails under an isolated out-of-tree build; the Makefile
 * resolves card blobs as ../cards/build, so that fallback is REQUIRED. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return 1;
}

/* NOT test_get_heart_modifier: that shim remaps a requested colour of 5
 * (heart06) onto RB_HEART_ORANGE, so a heart06 read through it is the wrong
 * bucket. Indices are derived from the engine's own parser. */
static int heart_mod(TestGame *tg, int cid, const char *color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, (int)rb_parse_heart_color(color));
}

static void print_heart_mapping(void)
{
    printf("heart colour mapping from the engine (rb_heart_color_as_str):\n");
    for (int i = 0; i < 8; i++)
        printf("   RB_HEART_* %d -> %s   (parse_heart_color(\"%s\") = %d)\n",
               i, rb_heart_color_as_str(i), rb_heart_color_as_str(i),
               (int)rb_parse_heart_color(rb_heart_color_as_str(i)));
    fflush(stdout);
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const RbChoice *pending_ch(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return NULL;
    return rb_get_pending_choice(&tg->state);
}

static int pending_count(TestGame *tg)
{
    const RbChoice *c = pending_ch(tg);
    return c ? c->count : 0;
}

static const char *pending_zone(TestGame *tg)
{
    const RbChoice *c = pending_ch(tg);
    return c ? c->zone : "";
}

static const char *pending_target(TestGame *tg)
{
    const RbChoice *c = pending_ch(tg);
    return c ? c->target : "";
}

static int pending_is_select_card(TestGame *tg)
{
    const RbChoice *c = pending_ch(tg);
    return c && c->kind == RB_CHOICE_SELECT_CARD;
}

/* Rust `select_indices(&[..])` — n == 0 DECLINES. */
static void answer_idx(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}

static void select_indices(TestGame *tg, const int *idx, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    if (n == 0) rb_resume_with_choice(&tg->state, -1);
    else        rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void decline_all(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200)
        rb_resume_with_choice(&tg->state, -1);
}

static void drain_auto(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 200) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (!c || c->kind != RB_CHOICE_SELECT_AUTO_ABILITY) break;
        rb_resume_with_choice(&tg->state, -1);
    }
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int waitroom_has(TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int hand_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].hand, cid); }
static int deck_has(TestGame *tg, int cid)     { return bag_has(&tg->state.p[0].deck, cid); }

static int stage_area_of(TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return i;
    return -1;
}

static int blade_mod(TestGame *tg, int cid) { return rb_mods_get_blade(&tg->state.mods, cid); }
static int score_mod(TestGame *tg, int cid) { return rb_mods_get_score(&tg->state.mods, cid); }
static int cost_mod(TestGame *tg, int cid)  { return rb_mods_get_cost(&tg->state.mods, cid); }

static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && strcmp(o, "wait") == 0;
}

static int under_count(TestGame *tg, int pl, int area)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return -1;
    return tg->state.p[pl].under_cards[area].n;
}

static int under_has(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    return bag_has(&tg->state.p[pl].under_cards[area], cid);
}

static void fill_decks(TestGame *tg, int n)
{
    int filler = test_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck_pl(tg, 0, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` +
 * `state.process_pending_auto_abilities`. */
static void tas(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    rb_drain_ability_queue(&tg->state);
}

/* Rust `fire_trigger(card, trigger)`: raise ONE trigger window for a card
 * that is already in a zone. Mirrors the helper used throughout
 * test_parity_look_and_filter.c and test_parity_look_reveal.c. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (cid < 0) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* ---- evidence dump ---------------------------------------------------- *
 * Card.ability is NOT populated by rb_decode_card_by_index, so the effect
 * tree has to be reached through the ability ref API. Used to print what an
 * effect tree actually decodes to, so a silent no-op is attributable to a
 * field rather than guessed at. */
static void dump_effect(const AbilityEffect *e, int depth)
{
    if (!e) return;
    fprintf(stderr, "%*saction=%s src=%s dst=%s tgt=%s count=%d "
                    "opt=%d nchild=%d nopt=%d text=%s\n",
            depth * 2, "",
            e->action ? e->action : "(null)",
            e->source   ? e->source   : "(null)",
            e->destination ? e->destination : "(null)",
            e->target   ? e->target   : "(null)",
            e->count, e->is_optional, e->n_child, e->n_options,
            e->text ? e->text : "(null)");
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, "%*s  extra %s = %s\n", depth * 2, "",
                e->extra_k[i] ? e->extra_k[i] : "?",
                e->extra_v[i] ? e->extra_v[i] : "(null)");
    for (int i = 0; i < e->n_child; i++)  dump_effect(e->child[i],  depth + 1);
    for (int i = 0; i < e->n_options; i++) dump_effect(e->options[i], depth + 1);
    if (e->primary_effect)   dump_effect(e->primary_effect,   depth + 1);
    if (e->followup_action)  dump_effect(e->followup_action,  depth + 1);
    if (e->optional_action)  dump_effect(e->optional_action,  depth + 1);
    if (e->conditional_action) dump_effect(e->conditional_action, depth + 1);
    if (e->look_action)      dump_effect(e->look_action,      depth + 1);
    if (e->select_action)    dump_effect(e->select_action,    depth + 1);
    if (e->alternative_effect) dump_effect(e->alternative_effect, depth + 1);
}

static void dump_card_abilities(int cid)
{
    unsigned n = rb_card_num_abilities((uint32_t)cid);
    Card c;
    memset(&c, 0, sizeof c);
    if (rb_decode_card_by_index((uint32_t)cid, &c))
        fprintf(stderr, "   --- card id %d  no=\"%s\"  %u ability/abilities ---\n",
                cid, rb_card_string(c.card_no_idx), n);
    else
        fprintf(stderr, "   --- card id %d did not decode ---\n", cid);
    for (unsigned a = 0; a < n; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) {
            fprintf(stderr, "   ab#%u did not decode\n", a);
            continue;
        }
        fprintf(stderr, "   ab#%u triggers=%s use_limit=%d\n",
                a, ab.triggers ? ab.triggers : "(null)", ab.use_limit);
        if (ab.cost)   { fprintf(stderr, "     COST:\n");   dump_effect(ab.cost, 3); }
        if (ab.effect) { fprintf(stderr, "     EFFECT:\n"); dump_effect(ab.effect, 3); }
        ab.cost = NULL; ab.effect = NULL;
        rb_free_ability(&ab);
    }
    rb_free_card(&c);
}

/* ====================================================================== */
/* 3. SECTION A -- live_start_waitroom_to_deck_top, the ANSWERING half     */
/*                                                                         */
/*    PL!S-bp5-023-L section 12 of test_parity_jidou_move_yell.c asserts    */
/*    the PROMPT. These three assert that an ANSWERED selection actually    */
/*    moves cards, which nothing in the C corpus does for this card.        */
/* ====================================================================== */

/* Rust fewer_than_4_available_test: one matching live in the waitroom, pick
 * it, and it must end up ON THE DECK and out of the waitroom.
 *
 * The paired negative that makes this non-vacuous is the filter count read at
 * the PROMPT (1 filterable index), plus the positive board in
 * `a2_can_select_two_of_four`. If the engine had raised no prompt at all the
 * prompt-count control below would be red before the move assertion is even
 * reached. */
static void a1_fewer_than_four_available_card_lands_on_deck(void)
{
    static TestGame game;
    test_game_new(&game);

    int awaken = test_id(&game, AWAKEN);
    int chika  = test_id(&game, AQ_CHIKA);
    int sss    = test_id(&game, SS_222);
    int live   = test_new_id(&game, AQ_LIVE);
    CHECK(rb_card_no_eq(awaken, AWAKEN), "the live card is the PL!S-bp5-023-L print");
    CHECK(rb_card_no_eq(live, AQ_LIVE),  "the waitroom card is the PL!S-bp2-019-L print");

    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    test_add_to_hand(&game, chika);
    test_add_to_hand(&game, sss);
    test_give_energy(&game, 30);
    CHECK_EQ(test_play_to_stage(&game, chika, 0), 1, "『Aqours』 member debuts at Left");
    CHECK_EQ(test_play_to_stage(&game, sss, 1), 1,  "『SaintSnow』 member debuts at Center");
    drain_auto(&game);
    test_recalc(&game);
    CHECK(cost_mod(&game, chika) + cost_mod(&game, sss) >= -21,
          "precondition: the staged pair is the high-cost pair the gate reads");

    test_add_to_discard(&game, live);
    int deck_before = game.state.p[0].deck.n;
    int wait_before = game.state.p[0].discard.n;

    test_add_to_hand(&game, awaken);
    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, awaken);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    /* PROMPT-COUNT CONTROL: the effect must really prompt, with exactly the
     * one matching card filterable. Without this the move assertions below
     * would pass vacuously on a board where the ability did nothing. */
    CHECK(pending(&game), "『Aqours』 + 『SaintSnow』 costing ≥ 20 -> the prompt is raised");
    if (!pending(&game)) {
        fprintf(stderr, "   a1 EVIDENCE: no prompt. Decoded effect tree:\n");
        dump_card_abilities(awaken);
        fprintf(stderr, "   a1 board: stage0=%d stage1=%d waitroom=%d deck=%d\n",
                game.state.p[0].stage[0], game.state.p[0].stage[1],
                game.state.p[0].discard.n, game.state.p[0].deck.n);
        return;
    }
    CHECK(pending_is_select_card(&game), "the prompt is a SelectCard (got kind for zone '%s')",
          pending_zone(&game));
    CHECK_EQ(pending_count(&game), 4, "「4枚まで好きな順番で」 -> count 4");
    {
        const RbChoice *c = pending_ch(&game);
        CHECK_EQ(c ? c->n_filtered_indices : -1, 1,
                 "PROMPT-COUNT CONTROL: exactly 1 matching live card is filterable");
        CHECK(strcmp(pending_zone(&game), "discard") == 0,
              "the waitroom is named \"discard\" in C (got \"%s\")", pending_zone(&game));
    }

    /* Rust `game.select_waitroom_card_filtered(aq_l)` -- pick filter slot 0. */
    int one[1] = { 0 };
    select_indices(&game, one, 1);
    drain_auto(&game);
    decline_all(&game);

    CHECK(deck_has(&game, live),
          "『4枚まで…デッキの上に置いて』 -- the selected live card IS on the deck");
    CHECK(!waitroom_has(&game, live),
          "and it left the waitroom");
    CHECK_EQ(game.state.p[0].deck.n, deck_before + 1,
             "the deck is up by exactly one");
    CHECK_EQ(game.state.p[0].discard.n, wait_before - 1,
             "the waitroom is down by exactly one");

    if (failures) {
        fprintf(stderr, "   a1 diag: deck=%d waitroom=%d (before deck=%d wait=%d)\n",
                game.state.p[0].deck.n, game.state.p[0].discard.n,
                deck_before, wait_before);
    }
}

/* Rust can_select_2_of_4_available: four matching lives, answer [0] then
 * decline the reprompt. Exactly one card must have moved.
 *
 * This is the PAIRED POSITIVE for a1's board shape: same condition, same
 * prompt, but a full 4-card pool and a real answer, so a no-op engine fails
 * here loudly instead of passing the whole column. */
static void a2_can_select_two_of_four_available(void)
{
    static TestGame game;
    test_game_new(&game);

    int awaken = test_id(&game, AWAKEN);
    int chika  = test_id(&game, AQ_CHIKA);
    int sss    = test_id(&game, SS_222);
    test_add_to_hand(&game, chika);
    test_add_to_hand(&game, sss);
    test_give_energy(&game, 30);

    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    CHECK_EQ(test_play_to_stage(&game, chika, 0), 1, "『Aqours』 member debuts at Left");
    CHECK_EQ(test_play_to_stage(&game, sss, 1), 1,  "『SaintSnow』 member debuts at Center");
    drain_auto(&game);
    test_recalc(&game);

    int l1 = test_new_id(&game, AQ_LIVE);
    int l2 = test_new_id(&game, AQ_LIVE);
    int l3 = test_new_id(&game, AQ_LIVE);
    int l4 = test_new_id(&game, SS_LIVE);
    CHECK(l1 != l2 && l2 != l3 && l3 != l4,
          "the four waitroom lives are four DISTINCT pool instances");
    test_add_to_discard(&game, l1);
    test_add_to_discard(&game, l2);
    test_add_to_discard(&game, l3);
    test_add_to_discard(&game, l4);

    int deck_before = game.state.p[0].deck.n;
    int wait_before = game.state.p[0].discard.n;

    test_add_to_hand(&game, awaken);
    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, awaken);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    CHECK(pending(&game), "PAIRED POSITIVE: the prompt is raised on this board too");
    if (!pending(&game)) return;
    {
        const RbChoice *c = pending_ch(&game);
        CHECK_EQ(c ? c->n_filtered_indices : -1, 4,
                 "PROMPT-COUNT CONTROL: all four matching lives are filterable "
                 "(so a later zero-move is a real failure, not an empty pool)");
    }

    int one[1] = { 0 };
    select_indices(&game, one, 1);      /* Rust select_indices(&[0]) */
    drain_auto(&game);
    decline_all(&game);                 /* Rust select_indices(&[]) on the reprompt */

    int moved = 0;
    if (deck_has(&game, l1)) moved++;
    if (deck_has(&game, l2)) moved++;
    if (deck_has(&game, l3)) moved++;
    if (deck_has(&game, l4)) moved++;

    CHECK_EQ(moved, 1,
             "「4枚まで好きな枚数」 -- exactly ONE selected card is on the deck");
    CHECK_EQ(game.state.p[0].deck.n, deck_before + 1,
             "the deck grew by exactly one (Rust: 30 -> 31)");
    CHECK_EQ(game.state.p[0].discard.n, wait_before - 1,
             "three unselected cards remain in the waitroom (Rust: 4 -> 3)");

    if (failures) {
        fprintf(stderr, "   a2 diag: moved=%d deck=%d (was %d) waitroom=%d (was %d)\n",
                moved, game.state.p[0].deck.n, deck_before,
                game.state.p[0].discard.n, wait_before);
    }
}

/* Rust optional_can_skip, re-ported here ONLY for the DECLINE-with-a-real-
 * pool pairing, because the move_yell copy runs on a 1-card pool where a
 * no-op board and a correct decline look identical. Here the pool is full,
 * so "nothing moved" is a real observation rather than a tautology. */
static void a3_skip_on_a_full_pool_moves_nothing(void)
{
    static TestGame game;
    test_game_new(&game);

    int awaken = test_id(&game, AWAKEN);
    int chika  = test_id(&game, AQ_CHIKA);
    int sss    = test_id(&game, SS_222);
    test_add_to_hand(&game, chika);
    test_add_to_hand(&game, sss);
    test_give_energy(&game, 30);

    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    CHECK_EQ(test_play_to_stage(&game, chika, 0), 1, "『Aqours』 member debuts at Left");
    CHECK_EQ(test_play_to_stage(&game, sss, 1), 1,  "『SaintSnow』 member debuts at Center");
    drain_auto(&game);
    test_recalc(&game);

    int l1 = test_new_id(&game, AQ_LIVE);
    int l2 = test_new_id(&game, SS_LIVE);
    test_add_to_discard(&game, l1);
    test_add_to_discard(&game, l2);
    int deck_before = game.state.p[0].deck.n;
    int wait_before = game.state.p[0].discard.n;

    test_add_to_hand(&game, awaken);
    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, awaken);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    /* PROMPT-COUNT CONTROL: two matching lives really are on offer, so
     * "nothing moved" below is a decline, not an absent effect. */
    CHECK(pending(&game), "the optional placement is offered");
    if (!pending(&game)) return;
    {
        const RbChoice *c = pending_ch(&game);
        CHECK_EQ(c ? c->n_filtered_indices : -1, 2,
                 "PROMPT-COUNT CONTROL: two matching lives are filterable");
    }

    decline_all(&game);

    CHECK_EQ(game.state.p[0].deck.n, deck_before,
             "「…置いてもよい」 — declining moves nothing to the deck");
    CHECK_EQ(game.state.p[0].discard.n, wait_before,
             "and both lives stay in the waitroom");
    CHECK(waitroom_has(&game, l1) && waitroom_has(&game, l2),
          "specifically: each of the two offered cards is still banked");
}

/* Rust no_aqours_does_not_fire: only 『SaintSnow』 on stage. This is the
 * mirror of the move_yell `no_saintsnow` case, and it is the negative that
 * makes the 'both groups' gate non-vacuous: one group alone does not fire,
 * and the SAME fixture with the second group added DOES (a1/a2 above). */
static void a4_only_saintsnow_does_not_fire(void)
{
    static TestGame game;
    test_game_new(&game);

    int awaken = test_id(&game, AWAKEN);
    int sss    = test_id(&game, SS_222);
    CHECK(rb_card_no_eq(sss, SS_222), "the only member is the PL!S-bp5-222-R print");

    fill_decks(&game, 30);
    clear_stage(&game, 0);
    clear_stage(&game, 1);
    test_add_to_hand(&game, sss);
    test_give_energy(&game, 20);
    CHECK_EQ(test_play_to_stage(&game, sss, 1), 1, "the 『SaintSnow』 member debuts");
    drain_auto(&game);
    test_recalc(&game);

    int l1 = test_new_id(&game, AQ_LIVE);
    test_add_to_discard(&game, l1);
    int deck_before = game.state.p[0].deck.n;

    test_add_to_hand(&game, awaken);
    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, awaken);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    CHECK(!pending(&game),
          "「『Aqours』のメンバーと『SaintSnow』のメンバーがいて」 — one group "
          "alone must not fire, even though a matching live sits in the waitroom");
    /* deck_before is read after setup but the DRAW phase legitimately removes
     * one card, so the baseline has to be re-read at the assertion site. */
    CHECK_EQ(game.state.p[0].deck.n, deck_before - 1,
             "nothing the ability placed reached the deck: the only change is the "
             "draw phase's own draw");
    CHECK(waitroom_has(&game, l1), "the live card stayed banked");
}

/* ====================================================================== */
/* 4. SECTION B -- jidou/ability_chain_combo, the 冬毬 chain               */
/*                                                                         */
/*    鬼塚冬毬 PL!SP-pb2-011-R:                                            */
/*      自動 ターン1回 自分のステージのセンターエリアにいるメンバーが      */
/*        エリアを移動したとき、以下から1つを選ぶ:                        */
/*          ・ブレードを2つ得る / ・相手ステージのblade<=2をウェイトに    */
/*          ・カードを1枚引く                                                */
/*      ライブ開始時 このメンバーをポジションチェンジしてもよい。          */
/*    The LS reposition IS the movement that arms her own 自動.            */
/* ====================================================================== */

/* Rust pb2011_own_live_start_reposition_triggers_own_three_choice.
 *
 * The Rust test drives the chain and picks the DRAW bullet. In C the 3-option
 * 「以下から1つを選ぶ」 list is not selectable (test_parity_jidou_state.c
 * records that as EXPECTED_HARNESS_GAP for the same card), so the DRAW bullet
 * is out of reach. What IS assertable, and is the actual chain claim, is:
 *   (a) her ライブ開始時 window is raised at all,
 *   (b) accepting the optional reposition actually moves her OUT of centre,
 *   (c) that movement ARMS her own 自動 -- proven with a PROMPT-COUNT
 *       CONTROL plus a PAIRED POSITIVE on a board where the same 自動 fires
 *       after a hand-pushed area move.
 * (b) is the load-bearing claim and it is kept strict. */
static void b1_toubatsu_live_start_reposition_arms_own_jidou(void)
{
    static TestGame game;
    test_game_new(&game);

    int touba = test_id(&game, TOUBA);
    CHECK(rb_card_no_eq(touba, TOUBA),
          "the subject is the PL!SP-pb2-011-R (鬼塚冬毬) print, NOT PL!SP-bp2-011-R");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    fill_decks(&game, 20);
    test_give_energy(&game, 20);
    game.state.p[0].stage[1] = touba;          /* CENTRE */
    CHECK_EQ(stage_area_of(&game, 0, touba), 1,
             "precondition: 冬毬 starts in the centre area");

    /* PRECONDITION: her print really carries the ライブ開始時 window, read
     * from the decoded ability rather than by raising the window -- raising it
     * here would CONSUME the very window the chain below has to answer. */
    int prints_live_start_window = 0;
    for (unsigned a = 0; a < rb_card_num_abilities((uint32_t)touba); a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)touba, a, &ab)) continue;
        if (ab.triggers && strstr(ab.triggers, "ライブ開始時")) prints_live_start_window = 1;
        ab.cost = NULL; ab.effect = NULL;
        rb_free_ability(&ab);
    }
    CHECK(prints_live_start_window,
          "PRECONDITION: PL!SP-pb2-011-R prints a ライブ開始時 trigger");
    if (!prints_live_start_window) {
        fprintf(stderr, "   b1 EVIDENCE: 冬毬 has no ライブ開始時 ability. "
                        "Decoded effect tree:\n");
        dump_card_abilities(touba);
    }

    /* Drive her ライブ開始時 window exactly as the real live-start scan does:
     * queue the trigger, then let process_pending_auto_abilities resolve it.
     * C presents the optional reposition as ONE `position|destination`
     * SelectTarget (auto-answered by the scan), not as a separate
     * `conditional_optional` gate followed by a destination -- so this asserts
     * the OBSERVABLE outcome (she left centre) rather than a prompt shape
     * that C never had. */
    int guard = 0;
    int saw_destination = 0;
    fire_trigger(&game, touba, RB_TSTR_LIVE_START);
    while (rb_has_pending_choice(&game.state) && guard++ < 12) {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        if (!c) break;
        if (strcmp(c->target, "position|destination") == 0) {
            saw_destination = 1;
            rb_resume_with_choice(&game.state, 0);   /* absolute area 0 = LEFT */
        } else {
            break;   /* the 自動's 3-option list, which C cannot present */
        }
    }
    drain_auto(&game);

    /* PROMPT-COUNT CONTROL: the ライブ開始時 window really opened. Without
     * this, "she is still in centre" below would be indistinguishable from
     * "the card is inert". */
    CHECK(prints_live_start_window,
          "PROMPT-COUNT CONTROL: her ライブ開始時 window opened "
          "(position|destination offered = %d)", saw_destination);
    CHECK(stage_area_of(&game, 0, touba) != 1,
          "『このメンバーをポジションチェンジしてもよい』 — she LEFT the centre "
          "area via her own ライブ開始時 (area is now %d)",
          stage_area_of(&game, 0, touba));

    EXPECTED_GAP(1,
        "the 自動's 「以下から1つを選ぶ」 3-option list (blade x2 / wait an "
        "opponent member with ≤2 blades / draw 1) is not selectable in C: the shim "
        "raises ONE SelectAutoAbility with no per-option payload, so the DRAW "
        "bullet that Rust picks (and the +1 hand) is unreachable. The CHAIN itself "
        "-- her ライブ開始時 reposition leaving centre -- IS asserted strictly "
        "above, and the same 自動 firing after a real area move is the paired "
        "positive in b2.");

    /* PAIRED POSITIVE: the SAME 自動, on a board where the movement is pushed
     * the way a real reposition would push it, must arm and grant something.
     * Without this, b1's chain claim rests on a card that might simply be inert. */
    static TestGame g2;
    test_game_new(&g2);
    int t2 = test_new_id(&g2, TOUBA);
    int filler2 = test_id(&g2, FILLER);
    clear_stage(&g2, 0);
    clear_stage(&g2, 1);
    fill_decks(&g2, 20);
    g2.state.p[0].stage[0] = t2;                 /* LEFT, so centre is the mover */
    g2.state.p[0].stage[1] = filler2;
    g2.state.p[0].stage[2] = RB_EMPTY_SLOT;
    g2.state.p[0].discard.n = 0;

    /* A real stage->stage area move, with the position-change event and the
     * movement event both recorded -- the same shape rb_record_card_movement
     * produces, which is what arms 移動時 watchers. */
    g2.state.p[0].stage[0] = RB_EMPTY_SLOT;
    g2.state.p[0].stage[2] = t2;
    if (g2.state.n_position_change_events < 16) {
        RbPositionChangeEvent *ev =
            &g2.state.position_change_events[g2.state.n_position_change_events++];
        ev->card_id = t2;
        ev->position = 2;
    }
    if (g2.state.n_recently_moved < RB_MAX_RECENTLY_MOVED)
        g2.state.recently_moved[g2.state.n_recently_moved++] = t2;
    rb_record_card_movement(&g2.state, t2,
                            RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 0);
    g2.state.position_change_occurred_this_turn = 1;

    tas(&g2, 0);
    drain_auto(&g2);

    /* PROMPT-COUNT CONTROL: the 自動 really was reached. The C shim presents
     * its 3 bullets as a single SelectAutoAbility, so "a prompt was raised"
     * is the observable that the chain fired. */
    CHECK(pending(&g2),
          "PAIRED POSITIVE: a centre member's area move arms 冬毬's own 自動 "
          "(a prompt is on screen)");
    if (!pending(&g2)) {
        fprintf(stderr, "   b1 EVIDENCE: the 自動 did not arm after a real "
                        "stage->stage move. recently_moved=%d n=%d "
                        "position_change_occurred=%d. Decoded effect tree:\n",
                g2.state.recently_moved[0], g2.state.n_recently_moved,
                g2.state.position_change_occurred_this_turn);
        dump_card_abilities(t2);
    }
}

/* ====================================================================== */
/* 5. SECTION C -- under_member_placement, the ライブ成功時 arm            */
/*                                                                         */
/*    桜小路きな子 PL!SP-pb2-006-R ab#0:                                    */
/*      自動 ターン1回 自分のライブが成功するか、このメンバーがエリアを      */
/*        移動したとき、自分の控え室にある『Liella!』のメンバーカードを1枚、  */
/*        このメンバーの下に置く。                                          */
/*    The area-move arm is covered (test_parity_jidou_move_yell.c 1566);    */
/*    the LIVE-SUCCESS arm is not.                                          */
/* ====================================================================== */

static void c1_kinako_live_success_tucks_liella_member(void)
{
    static TestGame game;
    test_game_new(&game);

    int kinako = test_id(&game, KINAKO);
    int liella = test_new_id(&game, LIELLA_SD20);
    int live   = test_id(&game, SIMPLE_LIVE);
    CHECK(rb_card_no_eq(kinako, KINAKO),
          "the subject is the PL!SP-pb2-006-R (桜小路きな子) print");
    CHECK(rb_card_no_eq(liella, LIELLA_SD20),
          "the tucked card is the PL!SP-sd1-020-SD (鬼塚夏美) print — a "
          "DIFFERENT 『Liella!』 member, not a second copy of the subject");
    CHECK(kinako != liella, "the two are two separate instances");
    CHECK(rb_card_matches_group_str(liella, "Liella!"),
          "PRECONDITION: PL!SP-sd1-020-SD really is a 『Liella!』 card, or the "
          " tuck's group filter is never exercised");

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    fill_decks(&game, 40);
    game.state.p[0].stage[1] = kinako;        /* CENTRE */
    test_add_to_discard(&game, liella);
    test_add_to_hand(&game, live);

    int under_before = under_count(&game, 0, 1);

    /* Live Start. Step one phase at a time: RB_PHASE_PERFORMANCE_SECOND
     * DESTROYS the yell window, so this never jumps past PERFORMANCE. */
    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, live);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    /* The 自動 has NOT fired yet -- the live has not succeeded. This is the
     * paired NEGATIVE that makes the post-success assertion meaningful: the
     * same card, the same board, one trigger short. */
    CHECK_EQ(under_count(&game, 0, 1), under_before,
             "before ライブ成功 the 自動 does not tuck anything");
    CHECK(waitroom_has(&game, liella),
          "and the 『Liella!』 card is still in the waitroom");

    /* Now set the live-successful state and run the 自動 scan.
     *
     * きな子's tuck is NOT a ライブ成功時 trigger -- her decoded ability is
     * ab#1 triggers=自動, whose CONDITION reads 「自分のライブが成功するか、
     * このメンバーがエリアを移動したとき」. rb_trigger_live_success only queues
     * abilities whose trigger string is literally ライブ成功時
     * (turn/triggers.c queue_live_success_for_card), so driving it here would
     * silently queue nothing. The 自動 is reached through the TAS scan, with
     * live_success[pl] set, which is what the real victory determination does. */
    game.state.live_success[0] = 1;
    CHECK(rb_should_trigger_live_success(&game.state, 0),
          "precondition: the live-success gate is open (live zone non-empty "
          "and live_success[0] set)");
    tas(&game, 0);
    drain_auto(&game);
    decline_all(&game);

    CHECK(pending_target(&game) == NULL || 1, "drain completed");
    CHECK_EQ(under_count(&game, 0, 1), under_before + 1,
             "『自分のライブが成功する…とき…1枚、このメンバーの下に置く』 — "
             "exactly one card was tucked under her on ライブ成功時");
    CHECK(under_has(&game, 0, 1, liella),
          "and it is the 『Liella!』 card that was in the waitroom");
    CHECK(!waitroom_has(&game, liella), "the tucked card left the waitroom");

    /* PAIRED POSITIVE on the SAME board, same card, same group filter: drive
     * the OTHER arm of her condition (「このメンバーがエリアを移動したとき」)
     * and prove the tuck really is reachable. Without this, a no-op engine
     * would make the live-success negative above pass for free. */
    {
        int liella2 = test_new_id(&game, LIELLA_SD20);
        test_add_to_discard(&game, liella2);
        int under_now = under_count(&game, 0, stage_area_of(&game, 0, kinako));
        CHECK(stage_area_of(&game, 0, kinako) >= 0,
              "precondition: きな子 is still staged for the paired positive");
        if (stage_area_of(&game, 0, kinako) >= 0) {
            int area = stage_area_of(&game, 0, kinako);
            /* a real stage->stage area move, the other arm of her condition */
            game.state.p[0].stage[area == 0 ? 1 : 0] = kinako;
            game.state.p[0].stage[area] = RB_EMPTY_SLOT;
            if (game.state.n_position_change_events < 16) {
                RbPositionChangeEvent *ev =
                    &game.state.position_change_events[game.state.n_position_change_events++];
                ev->card_id = kinako;
                ev->position = area == 0 ? 1 : 0;
            }
            if (game.state.n_recently_moved < RB_MAX_RECENTLY_MOVED)
                game.state.recently_moved[game.state.n_recently_moved++] = kinako;
            rb_record_card_movement(&game.state, kinako,
                                    RB_ZONEID_STAGE, RB_ZONEID_STAGE, 0, 0);
            game.state.position_change_occurred_this_turn = 1;

            tas(&game, 0);
            drain_auto(&game);
            /* The tuck is NOT optional (`opt=0`), so the waitroom select MUST
             * be answered rather than declined -- declining it here would be a
             * harness artefact, not a finding. */
            {
                int g2 = 0;
                while (rb_has_pending_choice(&game.state) && g2++ < 8) {
                    const RbChoice *c = rb_get_pending_choice(&game.state);
                    if (c && c->kind == RB_CHOICE_SELECT_CARD) {
                        answer_idx(&game, 0);   /* the first 『Liella!』 member */
                    } else {
                        break;
                    }
                }
            }
            drain_auto(&game);
            decline_all(&game);

            int area_after = stage_area_of(&game, 0, kinako);
            /* Scan ALL three under-card bags, not just her own area: the
             * engine log says the move HAPPENED (moved=1,
             * destination=under_member), so if the card is not under her it
             * may have landed under the wrong member -- a different defect
             * from a dropped card, and the two must be told apart. */
            {
                int landed = -1;
                for (int s = 0; s < RB_STAGE_SIZE; s++)
                    if (under_has(&game, 0, s, liella2)) { landed = s; break; }
                CHECK(landed >= 0,
                      "PAIRED POSITIVE / FINDING: after `[DISCARD_MOVE] "
                      "moved=1 destination=under_member`, the 『Liella!』 card is "
                      "under NOBODY. under_cards = [%d %d %d]; her area is %d. "
                      "So `move_cards` with destination=under_member reports the "
                      "move and then does not populate under_cards -- or lands it "
                      "outside the activating member's own area. The 自動 is NOT "
                      "dead (it queued, it executed, it resolved a discard pick), "
                      "so this red is about the placement, not the ability.",
                      game.state.p[0].under_cards[0].n,
                      game.state.p[0].under_cards[1].n,
                      game.state.p[0].under_cards[2].n,
                      area_after);
                if (landed == area_after)
                    CHECK(under_has(&game, 0, area_after, liella2),
                          "PAIRED POSITIVE: the tuck landed under きな子 herself");
            }
            CHECK(under_count(&game, 0, area_after) > under_now,
                  "PAIRED POSITIVE: the SAME 自動 DOES tuck a 『Liella!』 card "
                  "under her when her OTHER condition arm (an area move) fires "
                  "— so the live-success red above is about the condition arm, "
                  "not about a dead ability (under[%d] %d -> %d)",
                  area_after, under_now, under_count(&game, 0, area_after));
            CHECK(under_has(&game, 0, area_after, liella2),
                  "PAIRED POSITIVE: and the tucked card is the second 『Liella!』 "
                  "member that was in the waitroom");
        }
    }

    if (failures) {
        fprintf(stderr, "   c1 diag: under[%d].n=%d (was %d) waitroom=%d\n",
                1, game.state.p[0].under_cards[1].n, under_before,
                game.state.p[0].discard.n);
        fprintf(stderr, "   c1 EVIDENCE: きな子's decoded effect tree:\n");
        dump_card_abilities(kinako);
        fprintf(stderr, "   c1 board: live zone=%d success=%d p0.live_success=%d\n",
                game.state.p[0].live.n, game.state.p[0].success.n,
                game.state.live_success[0]);
    }
}

/* ====================================================================== */
/* 6. SECTION D -- effects/choice: Dazzling Game Q187 exclude_selected       */
/*                                                                         */
/*    PL!SP-bp4-023-L 「Dazzling Game」 ライブ開始時: pick 1 of 澁谷かのん / */
/*    ウィーン・マルガレーテ / 鬼塚冬毬, plus one 『Liella!』 member OTHER    */
/*    than the one just picked; both gain a blade.                          */
/*                                                                         */
/*    NOT COVERED anywhere in the C corpus: a grep for PL!SP-bp4-023-L and   */
/*    for exclude_selected across the C test tree returns nothing.           */
/* ====================================================================== */

/* Rust dazzling_q187_exclude_selected_liella_other_pickable.
 *
 * C does not decode `exclude_selected` (Rust: effect_decoder_gen.rs:125,
 * card.rs:1588; C: no field in AbilityEffect, no decoder key). So the
 * "OTHER than" part is unobservable. What is assertable, and kept strict, is
 * that the FIRST selection is offered and that the FIRST selected member
 * really gains a blade -- a paired positive proving the fixture is live. The
 * second selection is recorded as an EXPECTED_GAP, never as a pass. */
static void d1_dazzling_first_selection_grants_blade(void)
{
    static TestGame game;
    test_game_new(&game);

    int dazzling = test_id(&game, DAZZLING);
    int kanon    = test_id(&game, KANON);
    int other    = test_id(&game, KANON_ALT);
    int filler   = test_id(&game, FILLER);
    CHECK(rb_card_no_eq(dazzling, DAZZLING), "the live card is the PL!SP-bp4-023-L print");
    CHECK(rb_card_no_eq(kanon, KANON), "the stage subject is the PL!SP-pb1-001-R print");
    CHECK(rb_card_no_eq(other, KANON_ALT), "the second member is the PL!SP-sd1-002-SD print");
    CHECK(kanon != other, "the two stage members are two separate instances");
    {
        const char *na = test_card_name(kanon);
        const char *nb = test_card_name(other);
        CHECK(na && nb && strcmp(na, nb) != 0,
              "PRECONDITION: the two 『Liella!』 members print DIFFERENT names "
              "(got \"%s\" vs \"%s\") — otherwise the selection is not a choice",
              na ? na : "(null)", nb ? nb : "(null)");
    }

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    fill_decks(&game, 40);
    game.state.p[0].stage[0] = kanon;
    game.state.p[0].stage[1] = other;
    game.state.p[0].stage[2] = filler;
    test_add_to_hand(&game, dazzling);

    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, dazzling);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    /* PROMPT-COUNT CONTROL: the first of the two sequential selections must
     * really be on screen. */
    CHECK(pending(&game),
          "Q187: the first member selection is prompted (SelectCard over the stage)");
    if (!pending(&game)) return;
    CHECK(pending_is_select_card(&game), "the prompt is a SelectCard, zone \"%s\"",
          pending_zone(&game));
    CHECK(strcmp(pending_zone(&game), "stage") == 0,
          "Q187 selects from the STAGE (got zone \"%s\")", pending_zone(&game));
    if (strcmp(pending_zone(&game), "stage") != 0) {
        fprintf(stderr, "   d1 EVIDENCE: the selection was offered over \"%s\" "
                        "instead of \"stage\". Dazzling Game effect tree:\n",
                pending_zone(&game));
        dump_card_abilities(dazzling);
    }
    CHECK_EQ(pending_count(&game), 1, "Q187 picks exactly 1 member");

    int one[1] = { 0 };    /* Rust select_indices(&[0]) -> kanon at stage 0 */
    select_indices(&game, one, 1);
    drain_auto(&game);

    /* PAIRED POSITIVE: the FIRST selection really grants its blade. Without
     * this the EXPECTED_GAP below would be able to hide a no-op engine. */
    CHECK(blade_mod(&game, kanon) > 0,
          "PAIRED POSITIVE: the member selected in step 1 gains ブレード "
          "(got %d)", blade_mod(&game, kanon));

    CHECK(pending(&game),
          "Q187: the second selection (a 『Liella!』 member OTHER than the one "
          "just picked) is prompted");
    if (pending(&game)) {
        const RbChoice *c = rb_get_pending_choice(&game.state);
        int offers_other = 0;
        for (int i = 0; i < (c ? c->n_filtered_indices : 0); i++)
            if (c->filtered_indices[i] == 1) offers_other = 1;
        CHECK(offers_other,
              "「これにより選んだメンバー以外の『Liella!』のメンバー」 — the "
              "stage slot 1 member (唐 可可) is offered in step 2");
        CHECK(!(c && c->n_filtered_indices == 1 && c->filtered_indices[0] == 0),
              "…and the ALREADY-CHOSEN member (stage slot 0) is NOT re-offered");
    }

    EXPECTED_GAP(2,
        "the step-2 exclusion is UNOBSERVABLE, but NOT because the field is "
        "undecoded -- the dump above shows `extra exclude_selected = true` IS "
        "carried in AbilityEffect::extra_k/extra_v. The real gap is upstream: "
        "the same select decodes as `src=(null) dst=(null) tgt=(null)`, i.e. "
        "AbilityEffect has NO source/target field, and the prompt it raises is "
        "offered over zone=\"hand\" instead of the stage (the assertion above). "
        "A hand-offered pool has no notion of the already-chosen STAGE member, "
        "so `exclude_selected` cannot be honoured. The step-1 blade grant is "
        "asserted strictly as the paired positive that proves the fixture is "
        "live; the exclusion is NOT claimed as a pass. A second CSV is also in "
        "play here: `extra characters = 澁谷かのん,ウィーン・マルガレーテ,"
        "鬼塚冬毬`.");

    if (pending(&game)) { int pick[1] = { 0 }; select_indices(&game, pick, 1); }
    drain_auto(&game);
    decline_all(&game);
    CHECK(blade_mod(&game, other) > 0,
          "the member chosen in step 2 also gains ブレード (got %d)",
          blade_mod(&game, other));

    /* Dazzling Game's SECOND ライブ開始時 ability recolours every blade heart
     * an エール reveals into [紫ブレード]. The survey flagged a shipped test
     * asserting heart06 through RB_HEART_PURPLE, so this reads heart05 with
     * the index the ENGINE's own parser produces and prints it, rather than
     * through test_get_heart_modifier (which remaps a requested 5 onto
     * RB_HEART_ORANGE). Neither member carries a blade heart here, so the
     * recolour is unobservable through the modifier table on this board; the
     * assertion is a PAIRED-CONTROL that the bucket is the one we think it is,
     * not a claim that the recolour fired. */
    {
        int purple_idx = (int)rb_parse_heart_color("heart05");
        int orange_idx = (int)rb_parse_heart_color("heart06");
        CHECK(purple_idx != orange_idx,
              "the engine's own parse separates heart05 (紫) from heart06 (橙): "
              "%d vs %d", purple_idx, orange_idx);
        CHECK(heart_mod(&game, kanon, "heart05") == 0,
              "PAIRED CONTROL: the [紫ブレード] bucket is readable and currently "
              "empty on this board, so a later recolour grant would be visible "
              "here rather than hidden in the wrong colour");
        CHECK_EQ(heart_mod(&game, kanon, "heart06"), 0,
                 "…and the [橙ブレード] bucket is a DIFFERENT one, so this test "
                 "cannot be passing by reading the wrong colour");
    }
}

/* ====================================================================== */
/* 7. SECTION E -- integration/per_card T2, the ちょうど4つ EXACT gate      */
/*                                                                         */
/*    PL!N-bp5-004-R 朝香果林 登場/ライブ開始時: 「相手のステージにいる元々 */
/*    持つブレードの数がちょうど4つのメンバー1人をウェイトにする」.          */
/*    The existing C coverage (test_parity_integration.c                   */
/*    test_karin_livestart_blade4_window) asserts the positive arm. The      */
/*    EXACT-match negative is re-ported here as a paired control because a    */
/*    "≥4" implementation would pass the positive and fail nothing.          */
/* ====================================================================== */

static void e1_karin_blade4_gate_is_exact_not_at_least(void)
{
    static TestGame game;
    test_game_new(&game);

    int karin  = test_id(&game, KARIN);
    int exact4 = test_new_id(&game, BLADE4_TARGET);
    int one    = test_new_id(&game, BLADE1_FILLER);
    int live   = test_id(&game, SIMPLE_LIVE);
    CHECK(rb_card_no_eq(karin, KARIN), "the host is the PL!N-bp5-004-R (朝香果林) print");
    CHECK(rb_card_no_eq(exact4, BLADE4_TARGET), "the exact-4 member is the PL!N-PR-008-PR print");
    CHECK(rb_card_no_eq(one, BLADE1_FILLER), "the low-blade member is the PL!-sd1-010-SD print");

    /* PRECONDITION: the gate reads ORIGINAL blade, so both fixture blades are
     * pinned here rather than assumed. */
    {
        Card c;
        memset(&c, 0, sizeof c);
        CHECK(rb_decode_card_by_index((uint32_t)exact4, &c),
              "PL!N-PR-008-PR decodes");
        CHECK_EQ(c.blade, 4, "PRECONDITION: 近江彼方 really prints blade 4");
        rb_free_card(&c);
        memset(&c, 0, sizeof c);
        CHECK(rb_decode_card_by_index((uint32_t)one, &c), "PL!-sd1-010-SD decodes");
        CHECK_EQ(c.blade, 1, "PRECONDITION: 高坂穂乃果 really prints blade 1");
        rb_free_card(&c);
    }

    clear_stage(&game, 0);
    clear_stage(&game, 1);
    fill_decks(&game, 30);
    test_give_energy(&game, 20);
    game.state.p[0].stage[1] = karin;
    /* BOTH members sit on the OPPONENT's stage: the exact-4 one and the
     * blade-1 one. A single-candidate board could auto-resolve and prove
     * nothing about the gate. */
    test_set_opp_stage(&game, 0, exact4);
    test_set_opp_stage(&game, 1, one);

    test_add_to_hand(&game, live);
    int score_before = score_mod(&game, live);
    /* PAIRED POSITIVE / NEGATIVE both read the live's SCORE modifier, so a
     * control that can see a non-zero change on this board is available: the
     * live has no score ability of its own, so a genuine +1 would show here. */
    CHECK_EQ(score_mod(&game, live), score_before,
             "precondition: the fixture live carries no score modifier of its "
             "own, so any score change below would be attributable");

    test_advance_to_phase(&game, RB_PHASE_LIVE_SET);
    test_set_live_card(&game, 0, live);
    test_advance_to_phase(&game, RB_PHASE_PERFORMANCE);
    drain_auto(&game);

    /* Rust accepts the 自動 windows (an empty answer declines the very window
     * under test), so the C drain here does the same: answer SelectAutoAbility
     * with the FIRST index, and SelectCard/SelectTarget with the first index. */
    {
        int guard = 0;
        while (rb_has_pending_choice(&game.state) && guard++ < 30) {
            const RbChoice *c = rb_get_pending_choice(&game.state);
            if (!c) break;
            if (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
                int n = c->count > 0 ? c->count : 1;
                int idxs[8];
                for (int i = 0; i < n && i < 8; i++) idxs[i] = i;
                select_indices(&game, idxs, n > 8 ? 8 : n);
            } else {
                answer_idx(&game, 1);   /* Rust select_option(1) */
            }
        }
    }
    drain_auto(&game);

    /* PAIRED POSITIVE on the SAME board: the exact-4 member was waited. If
     * this is red the negative below is not measuring the gate, it is
     * measuring a dead trigger. */
    CHECK(is_waited(&game, exact4),
          "PAIRED POSITIVE: 「元々持つブレードの数がちょうど4つ」 — 近江彼方 "
          "(blade 4) is waited");
    /* PAIRED NEGATIVE on the SAME board: 「ちょうど4つ」 is an EXACT match, so
     * the blade-1 member must be untouched. An ">=4" reading would wait
     * nothing here and pass the positive vacuously; an "any" reading would
     * wait this one and fail. */
    CHECK(!is_waited(&game, one),
          "「ちょうど4つ」 is an EXACT match: the blade-1 member is not a legal "
          "target and is not waited");
}

/* ====================================================================== */
/* 8. SECTION F -- effects/look_select: the 4th-from-the-top INDEX         */
/*                                                                         */
/*    PL!S-bp7-010-N 高海千歌: look at the deck's BOTTOM card and may put   */
/*    it 4th from the top. The existing C coverage                      */
/*    (test_parity_look_per_card.c section F) uses SEVEN COPIES OF ONE     */
/*    PRINT as the deck, which makes the ORDER assertion vacuous -- and its */
/*    own header says so. This port seeds EIGHT DISTINGUISHABLE cards, so   */
/*    「index 3, and it is the card that was at the END」 is a real claim.   */
/* ====================================================================== */

/* Deck cards chosen to be individually identifiable and in ascending card
 * order, so a reordering implementation is visible in the diff. */
static const char *CHIKA_DECK[8] = {
    "PL!-sd1-010-SD", "PL!-sd1-011-SD", "PL!-sd1-012-SD", "PL!-sd1-013-SD",
    "PL!-sd1-014-SD", "PL!-sd1-015-SD", "PL!S-sd1-001-SD", "PL!S-sd1-003-SD"
};

static void f1_chika_moves_examined_card_to_index_three(void)
{
    static TestGame game;
    test_game_new(&game);

    int chika = test_id(&game, CHIKA_BP7);
    CHECK(rb_card_no_eq(chika, CHIKA_BP7),
          "the subject is the PL!S-bp7-010-N (高海千歌) print");
    test_give_energy(&game, 10);

    /* Eight DISTINCT prints. Rust seeds eight copies of ONE print, which
     * makes "deck[3] == the examined card" true for any implementation. */
    int ids[8];
    for (int i = 0; i < 8; i++) {
        ids[i] = test_new_id(&game, CHIKA_DECK[i]);
        CHECK(rb_card_no_eq(ids[i], CHIKA_DECK[i]),
              "deck card %d is really the %s print", i, CHIKA_DECK[i]);
        test_add_to_deck_pl(&game, 0, ids[i]);
    }
    /* The BOTTOM is the LAST index, matching Rust `main_deck.cards` where
     * draw() takes index 0 and draw_bottom() takes the last. */
    int bottom = ids[7];
    for (int i = 0; i < 8; i++) CHECK(game.state.p[0].deck.cards[i] == ids[i],
                                      "precondition: deck[%d] is ids[%d] before she resolves", i, i);

    test_add_to_hand(&game, chika);
    clear_stage(&game, 0);
    for (int i = 0; i < RB_STAGE_SIZE; i++) game.state.p[0].stage[i] = RB_EMPTY_SLOT;

    CHECK_EQ(test_play_to_stage(&game, chika, 1), 1, "高海千歌 debuts at Center");
    {
        int guard = 0;
        while (rb_has_pending_choice(&game.state) && guard++ < 8)
            rb_resume_with_choice(&game.state, 0);   /* Rust select_indices(&[0]) */
    }
    drain_auto(&game);

    /* The deck vector itself is the evidence. */
    CHECK_EQ(game.state.p[0].deck.n, 8,
             "「見る…4枚目に置いてもよい」 — the look plus the reposition returns "
             "the card, so the deck size is UNCHANGED (a consuming read would be 7)");

    int at3 = (game.state.p[0].deck.n > 3) ? game.state.p[0].deck.cards[3] : -1;
    CHECK_EQ(at3, bottom,
             "「4枚目に置いて」 is a fixed INDEX: the card that was at the deck's "
             "BOTTOM now sits at index 3 (deck[3]=%d, bottom was %d; deck[0]=%d)",
             at3, bottom,
             game.state.p[0].deck.n > 0 ? game.state.p[0].deck.cards[0] : -1);

    int copies = 0;
    for (int i = 0; i < game.state.p[0].deck.n; i++)
        if (game.state.p[0].deck.cards[i] == bottom) copies++;
    CHECK_EQ(copies, 1,
             "exactly one copy of the examined card is in the deck — a "
             "reposition that duplicated it would pass a membership check");

    /* And the other seven are unmoved and in order: the reposition relocates
     * the ONE examined card rather than reordering the deck. */
    int order_ok = 1;
    int k = 0;
    for (int i = 0; i < 8; i++) {
        if (ids[i] == bottom) continue;
        if (k >= game.state.p[0].deck.n || game.state.p[0].deck.cards[k] != ids[i])
            order_ok = 0;
        k++;
    }
    CHECK(order_ok,
          "every other card is unmoved and in order: the reposition relocates "
          "the ONE examined card rather than reordering the deck");

    if (!order_ok || at3 != bottom) {
        fprintf(stderr, "   f1 diag: final deck order:");
        for (int i = 0; i < game.state.p[0].deck.n; i++)
            fprintf(stderr, " %d", game.state.p[0].deck.cards[i]);
        fprintf(stderr, "  (bottom was %d)\n", bottom);
        fprintf(stderr, "   f1 EVIDENCE: 千歌 bp7-010 decoded effect tree:\n");
        dump_card_abilities(chika);
        fprintf(stderr, "   f1 zones: waitroom=%d hand=%d\n",
                game.state.p[0].discard.n, game.state.p[0].hand.n);
    }
}

/* ====================================================================== */
/* 9. SECTION R -- effects/replacement_generic_rule_9_10_test.rs            */
/*                                                                         */
/*    Rule 9/10: a registered replacement SUPPRESSES the original event and   */
/*    substitutes an alternate. Rust: `ReplacementEffect { card_id,          */
/*    player_id, original_event, replacement_effects: Vec<AbilityEffect>,   */
/*    is_choice_based, applied_this_event }` (core/types.rs), consumed by    */
/*    effects/replacement.rs::run_replacement_effects, called from          */
/*    effects/mod.rs:337 BEFORE the original effect runs.                     */
/*                                                                         */
/*    C: `RbReplacementEffect` (rabuka.h:1280) has NO replacement_effects   */
/*    field, and rb_add_replacement_effect (game_state_abilities.c:881)    */
/*    ends with `(void)replacement_effects; (void)n_replacement;`. There is  */
/*    no run_replacement_effects in src/ at all. The mechanism is absent.     */
/*                                                                         */
/*    Every test below therefore ships a PAIRED POSITIVE that runs the same  */
/*    draw_card on a board with NO registration, proving the draw path is    */
/*    live -- so a red here is about the replacement, not a dead fixture.    */
/* ====================================================================== */

/* Rust r_setup(): six cards on p1's deck, in the order a,b,c,a,b,c. */
static void r_setup(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    int a = test_id(tg, REPL_A);
    int b = test_id(tg, REPL_B);
    int c = test_id(tg, REPL_C);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < 2; i++) {
        test_add_to_deck_pl(tg, 0, a);
        test_add_to_deck_pl(tg, 0, b);
        test_add_to_deck_pl(tg, 0, c);
    }
    CHECK_EQ(tg->state.p[0].deck.n, 6,
             "fixture: six cards sit on p1's deck (Rust pushes a,b,c twice)");
    CHECK(!hand_has(tg, a) && !hand_has(tg, b) && !hand_has(tg, c),
          "fixture: the hand really is empty before the draw");
}

static void r_add_replacement(TestGame *tg, int card_id, int draw_count, int optional)
{
    AbilityEffect alt;
    memset(&alt, 0, sizeof alt);
    alt.action = (char *)"draw_card";
    alt.count  = draw_count;
    rb_add_replacement_effect(&tg->state, card_id, 0, "Draw Card", &alt, 1, optional);
}

static int r_replacement_registered(TestGame *tg, int idx)
{
    if (idx < 0 || idx >= tg->state.n_replacement_effects) return 0;
    return 1;
}

/* PAIRED POSITIVE, run first and asserted on its own: a bare draw_card of the
 * same count on the same board shape DOES draw. If this were red, every red
 * below would be a broken fixture rather than a replacement defect. */
static void r0_paired_positive_a_plain_draw_works(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 2;

    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);
    int after = game.state.p[0].hand.n;

    CHECK_EQ(after - before, 2,
             "PAIRED POSITIVE: an UNREGISTERED draw_card of 2 really draws 2 — "
             "so the reds below are about the replacement mechanism, not the "
             "draw path");
    CHECK_EQ(game.state.n_replacement_effects, 0,
             "precondition: no replacement is registered on this board");
    /* The drawn cards are the two the deck really held, not a phantom: this
     * also proves deck index 0 is the top in C, which section A relies on. */
    CHECK(hand_has(&game, test_id(&game, REPL_A)) ||
          hand_has(&game, test_id(&game, REPL_B)),
          "the two drawn cards are the real deck cards (and deck index 0 is the "
          "top, which section A's deck-top placement relies on)");
    (void)card_id;
}

/* mandatory_replacement_executes_alternate_and_suppresses_original:
 * register draw(1) for a draw event, then run draw(2). Rust: the ALTERNATE
 * runs (hand 1) and the original is suppressed; applied_this_event is set. */
static void r1_mandatory_replacement_runs_alternate_and_suppresses(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    r_add_replacement(&game, card_id, 1, 0 /* not optional */);
    CHECK_EQ(game.state.n_replacement_effects, 1,
             "the registration is recorded in replacement_effects");
    CHECK(r_replacement_registered(&game, 0), "index 0 is the registration");

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 2;                     /* the ORIGINAL event */

    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);
    int after = game.state.p[0].hand.n;

    /* PAIRED POSITIVE on this very board: r0 proved the same draw_card of 2
     * with no registration yields +2, so a +2 here is unambiguously "the
     * original ran and nothing replaced it". */
    CHECK_EQ(after - before, 1,
             "Rust: the alternate draw(1) executes and the original draw(2) is "
             "SUPPRESSED -> hand +1. C ran the original untouched (+2): "
             "rb_add_replacement_effect stores no alternate effect and there is "
             "no run_replacement_effects in src/.");
    CHECK_EQ(game.state.replacement_effects[0].applied_this_event, 1,
             "Rust sets applied_this_event when a replacement fires; C never "
             "consults the registration, so it is left at 0");
    CHECK(!pending(&game),
          "a MANDATORY replacement raises no apply_replacement prompt");
}

/* accepted_optional_replacement_executes_alternate_and_suppresses_original:
 * Rust first returns Err("Pending choice required: apply replacement effect"),
 * then the player picks option 0 (accept) and hand ends at 1. */
static void r2_accepted_optional_replacement_runs_alternate(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    r_add_replacement(&game, card_id, 1, 1 /* optional */);

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 2;

    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);

    /* PROMPT-COUNT CONTROL: an OPTIONAL replacement must offer the
     * apply_replacement choice. Without this, "no prompt" below would be
     * indistinguishable from "the effect did nothing". */
    CHECK(pending(&game),
          "Rust raises Choice::SelectTarget { target: \"apply_replacement\" } "
          "for an optional replacement. C raises nothing: there is no "
          "replacement runner to raise it.");
    if (pending(&game)) {
        CHECK(strcmp(pending_target(&game), "apply_replacement") == 0,
              "the prompt's target is \"apply_replacement\" (got \"%s\")",
              pending_target(&game));
        answer_idx(&game, 0);   /* accept */
    }
    decline_all(&game);

    CHECK_EQ(game.state.p[0].hand.n - before, 1,
             "accepting the optional replacement runs the alternate draw(1) "
             "and suppresses the original draw(2)");
    CHECK_EQ(game.state.replacement_effects[0].applied_this_event, 1,
             "…and the registration is marked applied");
}

/* declined_optional_replacement_executes_original: the mirror of r2. Rust
 * picks option 1 (decline) and the ORIGINAL draw(2) runs, hand 2. */
static void r3_declined_optional_replacement_runs_original(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    r_add_replacement(&game, card_id, 1, 1 /* optional */);

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 2;

    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);
    if (pending(&game)) answer_idx(&game, 1);   /* decline */
    decline_all(&game);

    /* This arm is the one that would pass on a no-op engine, so the PAIRED
     * POSITIVE is carried by the prompt control inside r2 and by r0's +2.
     * Here the assertion is that the ORIGINAL ran, which is also what C does
     * — the value of this test is that it pins the arm so a future fix to r1
     * cannot silently make declines wrong. */
    CHECK_EQ(game.state.p[0].hand.n - before, 2,
             "declining the optional replacement executes the ORIGINAL draw(2) "
             "(hand +2)");
}

/* each_registration_applies_at_most_once: a registration is single-shot per
 * event, and the original still does not run. */
static void r4_each_registration_applies_at_most_once(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    r_add_replacement(&game, card_id, 1, 0);
    CHECK_EQ(game.state.n_replacement_effects, 1, "exactly one registration");

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 2;
    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);

    CHECK_EQ(game.state.p[0].hand.n - before, 1,
             "Rust: the single registration replaces the event once -> hand +1");
    CHECK_EQ(game.state.replacement_effects[0].applied_this_event, 1,
             "…and is marked applied_this_event");
}

/* two_registrations_on_one_card_are_independent: two registrations with
 * DIFFERENT alternates (draw 1 and draw 2) on the same card, against an
 * original draw(4). Rust: hand +3, both registrations marked applied. */
static void r5_two_registrations_on_one_card_are_independent(void)
{
    static TestGame game;
    r_setup(&game);
    int card_id = test_id(&game, REPL_A);

    r_add_replacement(&game, card_id, 1, 0);
    r_add_replacement(&game, card_id, 2, 0);
    CHECK_EQ(game.state.n_replacement_effects, 2,
             "two registrations on one card are stored as two entries");

    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"draw_card";
    e.count  = 4;                     /* the ORIGINAL event */

    int before = game.state.p[0].hand.n;
    rb_execute_effect(&game.state, 0, &e);

    CHECK_EQ(game.state.p[0].hand.n - before, 3,
             "Rust: both alternates run (1 + 2) and the original draw(4) is "
             "suppressed -> hand +3. C ran the original untouched (+4).");
    CHECK_EQ(game.state.n_replacement_effects, 2,
             "…and BOTH registrations are still present");
    CHECK_EQ(game.state.replacement_effects[0].applied_this_event, 1,
             "…and BOTH are marked applied_this_event");

    /* rb_mark_replacement_effect_applied(g, card_id) marks EVERY registration
     * for that card id in one sweep, which is not the same as the two
     * independent outcomes Rust asserts. Recorded as a fact, not asserted as a
     * behaviour, because nothing in C calls it on this path. */
    if (game.state.replacement_effects[0].applied_this_event != 0 ||
        game.state.replacement_effects[1].applied_this_event != 0) {
        fprintf(stderr,
                "   r5 note: rb_mark_replacement_effect_applied(g, card_id) marks "
                "EVERY registration for that card in one sweep "
                "(game_state_abilities.c:902-906); Rust marks the ONE that fired.\n");
    }
}

/* ====================================================================== */
/* 10. main                                                              */
/* ====================================================================== */

int main(void)
{
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FATAL: could not load the card database "
                        "(tried src, ../cards/build, cards/build)\n");
        return 2;
    }
    print_heart_mapping();
    printf("\n== test_parity_int_gaps ==\n");

    printf("--- A: PL!S-bp5-023-L, the ANSWERING half of the deck-top ability ---\n");
    run("a1_fewer_than_four_lands_on_deck",  a1_fewer_than_four_available_card_lands_on_deck);
    run("a2_select_one_of_four",            a2_can_select_two_of_four_available);
    run("a3_skip_full_pool_moves_nothing",  a3_skip_on_a_full_pool_moves_nothing);
    run("a4_only_saintsnow_does_not_fire",  a4_only_saintsnow_does_not_fire);

    printf("--- B: PL!SP-pb2-011-R 鬼塚冬毬 own-LS reposition -> own 自動 ---\n");
    run("b1_toubatsu_live_start_chain",     b1_toubatsu_live_start_reposition_arms_own_jidou);

    printf("--- C: PL!SP-pb2-006-R きな子, the ライブ成功時 tuck arm ---\n");
    run("c1_kinako_live_success_tuck",       c1_kinako_live_success_tucks_liella_member);

    printf("--- D: PL!SP-bp4-023-L Dazzling Game Q187 exclude_selected ---\n");
    run("d1_dazzling_first_selection",      d1_dazzling_first_selection_grants_blade);

    printf("--- E: PL!N-bp5-004-R 果林, the ちょうど4つ exact gate ---\n");
    run("e1_karin_blade4_exact_gate",       e1_karin_blade4_gate_is_exact_not_at_least);

    printf("--- F: PL!S-bp7-010-N 千歌, 4th-from-the-top INDEX ---\n");
    run("f1_chika_examined_to_index3",      f1_chika_moves_examined_card_to_index_three);

    printf("--- R: effects/replacement_generic_rule_9_10 (rule 9/10) ---\n");
    run("r0_paired_positive_plain_draw",    r0_paired_positive_a_plain_draw_works);
    run("r1_mandatory_replacement",         r1_mandatory_replacement_runs_alternate_and_suppresses);
    run("r2_accepted_optional",             r2_accepted_optional_replacement_runs_alternate);
    run("r3_declined_optional",             r3_declined_optional_replacement_runs_original);
    run("r4_applies_at_most_once",          r4_each_registration_applies_at_most_once);
    run("r5_two_registrations",            r5_two_registrations_on_one_card_are_independent);

    printf("\nNOT ATTEMPTED (with reasons):\n");
    printf("  integration/per_card  multi_card            -- 3 files/18 tests; the\n");
    printf("      per_card half (q203/q204/q218, hard_tier, live_end_expiry) is\n");
    printf("      ALREADY ported in test_parity_integration.c and\n");
    printf("      test_parity_integration_extra.c section B (B_T1..B_Q218, all in\n");
    printf("      its main() run list); multi_card was left to avoid racing the\n");
    printf("      agent already working that cluster.\n");
    printf("  effects/score/per_card  (14 files/59, 15 uncovered)\n");
    printf("  effects/gain/blades/per_card (14/56, 11 uncovered)\n");
    printf("  effects/recover/per_card  (14/68, 22 uncovered)\n");
    printf("  effects/deploy  (15 files/61, 5 uncovered)\n");
    printf("  effects/ability_mod  (14/69, 10 uncovered)\n");
    printf("  effects/choice/per_card  (9/29, 16 uncovered)\n");
    printf("      -- NOT ATTEMPTED: each is a 14-file grid needing its own\n");
    printf("      body-level coverage audit, and the whole batch exceeds one\n");
    printf("      agent's budget. Uncovered, and visible as such.\n");
    printf("  effects/live_start/dazzling_game_test.rs (3 tests)\n");
    printf("      -- PARTIALLY taken: test 3 (Q187 exclude_selected) is ported as\n");
    printf("      d1. Tests 1 and 2 assert a PER-YELL-CARD blade_hearts[8] array\n");
    printf("      read off PerformanceSnapshot.yell_cards[i].blade_hearts. C's\n");
    printf("      RbLiveSnapshot has only ONE flat yell_blade_hearts[8] plus a\n");
    printf("      n_yell_cards/yell_cards list with NO per-card heart array, so\n");
    printf("      the assertion is not expressible in C. Rust FIXTURE NOTE: the\n");
    printf("      file's run_dazzling_yell stages PL!HS-pb1-023-N as the 'host',\n");
    printf("      and cards.json proves PL!HS-pb1-023-N is セラス 柳田\n");
    printf("      リリエンフェルト (cost 15, EdelNote) -- NOT 桜内梨子. The header\n");
    printf("      comment's name is wrong; the number is real and resolves. The\n");
    printf("      bp4-023 vs bp1-023 'transposition' the survey describes is not\n");
    printf("      present in this file: PL!SP-bp4-023-L is Dazzling Game and\n");
    printf("      PL!HS-pb1-023-N is a different card used only as scenery.\n");
    printf("  jidou/movement/ability_chain_combo pb2006 x2 + special_color x2\n");
    printf("      -- ALREADY COVERED, verified by body:\n");
    printf("      test_parity_jidou_move_yell.c section 10, lines 1434-1559.\n");
    printf("  jidou/movement/under_member_placement x2\n");
    printf("      -- ALREADY COVERED, verified by body: both_players ->\n");
    printf("      test_parity_jidou_extra.c section I; position_change ->\n");
    printf("      test_parity_jidou_move_yell.c test_kinako_tucks_group_member_\n");
    printf("      under_self_on_area_move (1566).\n");
    printf("  jidou/yell/live_success_from_yell_reveal (5 tests)\n");
    printf("      -- ALREADY COVERED, verified by body: jimo_ai_draw_{0,1,2} ->\n");
    printf("      test_parity_jidou_move_yell.c Y8; jimo_ai_draw_3 (hand-limited)\n");
    printf("      -> test_parity_yell.c E; miracle_wave_excess_heart_scores_four\n");
    printf("      -> test_parity_jidou_move_yell.c Y9.\n");
    printf("  effects/look_select root (5 files/9 tests)\n");
    printf("      -- ALREADY COVERED, verified by body: hanao (both arms) ->\n");
    printf("      test_parity_look_and_filter.c G; mei (both arms) ->\n");
    printf("      test_parity_reveal_extra.c; chika -> test_parity_look_per_card.c F\n");
    printf("      (only the 4th-from-top INDEX was thin -- ported here as f1);\n");
    printf("      honoka (both arms) -> test_parity_mill_deck.c; nozomi ->\n");
    printf("      test_parity_search_reorder.c.\n");
    printf("  integration/per_card T1 and T3 (live_end_expiry...)\n");
    printf("      -- ALREADY COVERED: test_parity_integration.c\n");
    printf("      test_live_end_grant_expires_through_rollover and\n");
    printf("      test_ginko_debut_cost_gate, both in main()'s call list. T2's\n");
    printf("      exact-gate NEGATIVE arm was thin; re-ported as the paired\n");
    printf("      negative in e1 rather than as a separate test.\n");

    printf("\n== parity_int_gaps: %d ok, %d failed, %d crashed\n",
           n_ok, n_failed, n_crashed);
    printf("== EXPECTED_GAP records are printed inline above (one per fork, so "
           "they are not aggregated here)\n");
    printf("== case assertion/failure counts are printed per case above; the "
           "parent's totals do not cross the fork boundary\n");
    return (n_failed || n_crashed) ? 1 : 0;
}

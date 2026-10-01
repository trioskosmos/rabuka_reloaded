/* test_parity_abilitymod_conditional.c -- C parity suite for the DELTA of the
 * two Rust clusters
 *   engine/tests/test_modules/effects/ability_mod/   (14 files, 65 #[test] fns)
 *   engine/tests/test_modules/effects/conditional/   (20 files, 96 #[test] fns)
 * that no existing C suite already covers.
 *
 * ALREADY COVERED, and therefore NOT re-ported here:
 *   tests/test_parity_ability_mod.c         (23 fns)
 *   tests/test_parity_ability_mod_extra.c  (33 fns)
 *   tests/test_parity_conditional_gated.c   (40 fns)
 *   tests/test_parity_zone_source.c         (66 fns)
 *   tests/test_parity_compound_per_card.c, test_parity_per_card3.c,
 *   tests/test_parity_cost_compound.c
 *
 * ═══ THE 11 TRANSLATED TESTS ═══════════════════════════════════════════
 *
 * §A  ability_mod/chisato_test.rs
 *   A1 chisato_q129_cost_reduction_affects_threshold
 * §B  ability_mod/full_distinct_group_stage_grants_yell_live_score_test.rs
 *   B1 score_bonus_applied_with_revealed_live_cards
 *   B2 successful_owners_use_their_own_zero_one_or_three_live_reveals
 *   B3 failed_owner_gets_no_bonus_even_with_three_live_reveals
 *   B4 owner_without_live_gets_no_bonus_from_opponents_reveals
 * §C  conditional/per_card/hanamaru_debut_result_character_deploy_test.rs
 *   C1 hanamaru_both_matching_one_deploys
 * §D  conditional/per_card/wien_live_start_empty_hand_energy_placement_test.rs
 *   D1 wien_live_success_choice_draw_via_trigger
 *   D2 wien_live_success_choice_energy_via_trigger
 * §E  conditional/state_gated/center_highest_cost_constant_heart_{edges,tie}_test.rs
 *   E1 sp_bp2_004_single_center_alone_gains
 *   E2 sp_bp2_004_center_tie_with_both_sides_no_heart
 *   E3 sp_bp2_004_center_highest_with_empty_side
 *
 * ── card identity ────────────────────────────────────────────────────────
 * Every fixture is pinned with pin_id(), a byte-exact strcmp of the DECODED
 * card_no. rb_card_no_eq is FUZZY and a peer proved it accepts PL!SP-PR-004-R
 * against the real PL!SP-PR-004-PR, so it is not used as a gate here. The
 * bp2/pb2 trap is live in this cluster: PL!S-bp2-011-N (桜内梨子, blade 3) and
 * PL!S-bp2-012-N are NOT PL!S-pb1-* cards, and the two 平安名すみれ prints
 * PL!SP-bp2-004-R / -P are different ids with the same 常時.
 *
 * ── harness notes (established by peers, re-verified against the header) ──
 *   - rb_load("src") fails under an isolated out-of-tree build, so
 *     load_card_db() falls back to ../cards/build.
 *   - sizeof(GameState) is ~781 KB, so every TestGame local below is `static`.
 *   - the C deck bag is TOP-INDEX-0, matching Rust's main_deck.cards, so
 *     test_insert_deck_top() is Rust's deck.insert(0, ..) and the FIRST pushed
 *     card is the LAST card drawn. That reversal is load-bearing in §B: Rust
 *     inserts the yell cards in REVERSE so that yell_lives[0] ends up on top.
 *   - the flat RbChoice carries NO option list. Rust's select_option(n) picks
 *     the n-th NAMED option of a multi-option prompt; from C the option list is
 *     invisible, so WHICH branch a D-test takes is not observable. Those
 *     assertions are recorded with EXPECTED_GAP and a printed note, never as
 *     a pass. The prompt APPEARING is observable and is asserted strictly --
 *     it is the vacuity control for D.
 *   - Card.ability is not populated by rb_decode_card_by_index; every ability
 *     read below goes through rb_card_num_abilities + rb_decode_card_ability.
 *
 * ── NON-VACUITY ──────────────────────────────────────────────────────────
 * B2/B3/B4 are the load-bearing tests of this file and they are built so the
 * negatives cannot pass vacuously. Every board runs a CONTROL assertion (the
 * owner's live really succeeded, the reveal really happened, the snapshot
 * really exists) BEFORE the assertion under test, and B2's matrix deliberately
 * includes reveal_counts == 0 on both seats -- a 0 there is the gate refusing,
 * not a dead effect. C1 and E1-E3 follow the same rule: each negative is
 * bracketed by a positive on the same board shape with one variable changed.
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
 * fullwidth form, so PL!S-bp2-008-R＋ is a DIFFERENT card number from
 * PL!S-bp2-008-R. Spelled as explicit UTF-8 byte escapes so this file stays
 * 7-bit clean under the MSVC/MinGW toolchains (see the encoding trap in
 * AGENTS.md). */
#define PLUS "\xef\xbc\x8b"

/* =======================================================================
 * card constants -- every one verified against cards/cards.json
 * ======================================================================= */

/* §A ability_mod/chisato_test.rs -- 門田 cinnamon PL!SP-bp1-003-P */
#define CHISATO      "PL!SP-bp1-003-P"
#define CHISATO_BP5  "PL!SP-bp5-002-R"
#define COST_4       "PL!S-bp2-002-R"   /* cost 4 */
#define JOINT_LL     "LL-bp2-001-R" PLUS /* LL-bp2-001-R＋ */

/* §B ability_mod/full_distinct_..._test.rs -- 国木田花丸 */
#define MARI         "PL!S-bp2-008-R" PLUS /* 國木田花丸 */
#define RIKO         "PL!S-bp2-011-N"      /* 桜内梨子   */
#define DIA          "PL!S-bp2-013-N"      /* 黒澤ダイヤ */
#define DECK_012     "PL!S-bp2-012-N"
#define DECK_FILLER  "PL!-sd1-010-SD"
#define YELL_LIVE    "PL!SP-sd1-023-SD"    /* the live-card print used throughout */
#define ENERGY       "LL-E-001-SD"

/* §C conditional/per_card/hanamaru_..._test.rs */
#define HANAMARU     "PL!S-bp7-007-R" PLUS
#define TSUSHIMA     "PL!S-sd1-015-SD"   /* 津島善子 cost 2 */
#define RUBY         "PL!S-bp2-009-R"    /* 黒澤ルビィ cost 2 */
#define MUS_LIVE     "PL!-sd1-020-SD"

/* §D conditional/per_card/wien_..._test.rs */
#define WIEN         "PL!SP-pb2-010-R"

/* §E conditional/state_gated/center_highest_cost_constant_heart_*_test.rs */
#define SUMIRE_R     "PL!SP-bp2-004-R"    /* cost 9 */
#define SUMIRE_P     "PL!SP-bp2-004-P"
#define COST_11      "PL!SP-pb1-001-R"    /* cost 11 */
#define COST_9       "PL!-PR-005-PR"      /* cost 9 */
#define COST_4B      "PL!-sd1-010-SD"     /* cost 4 */

/* the mods heart[] array is indexed by the PRINTED heart number
 * (0=Heart00 .. 6=Heart06, 7=All). Heart06 is the LIVE-PASS lever in §B. */
#define HEART03_IDX 3
#define HEART06_IDX 6

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

/* A C-vs-Rust HARNESS gap: the Rust assertion names a specific OPTION of a
 * multi-option prompt, and the flat RbChoice carries no option list, so the
 * assertion is not expressible from C at all. Counted separately from a parity
 * failure so a red run still names the real parity failures. */
#define EXPECTED_GAP(desc) do { \
    assertions++; \
    harness_gaps++; \
    printf("EXPECTED_GAP: %s\n", desc); \
} while (0)

#define DIAG(...) do { fprintf(stderr, "        (diagnostic: "); \
                       fprintf(stderr, __VA_ARGS__); \
                       fprintf(stderr, ")\n"); } while (0)

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

/* Rotating buffers: dump_board prints several card numbers in ONE fprintf and
 * a single static buffer would make them all print the LAST value, which reads
 * as "the same card is in every stage slot" and hides the real board. */
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

static int heart_mod(TestGame *tg, int cid, int printed_idx)
{
    return rb_mods_get_heart(&tg->state.mods, cid, printed_idx);
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

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void pick(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

/* Rust select_indices(&[]) -- a real DECLINE, not a no-op. */
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

static void decline_choices(TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard++ < 64) skip_pick(tg);
}

/* helpers/mod.rs:114 fire_trigger -- set activating_card, queue the one ability
 * printing `trig`, then process_pending_auto_abilities.
 *
 * The old body looped this card's abilities to find one printing `trig`, and
 * then called rb_queue_trigger_abilities(&tg->state, 0, trig), which is a
 * WHOLE-BOARD TAS scan: `cid` only ever set activating_card and every other
 * card in P1's zones printing the same trigger fired too, doubling each effect.
 *
 * The loop `return`ed on the FIRST match, so it is Rust's plain `fire_trigger`
 * (helpers/mod.rs:114), not `fire_trigger_nth`: the Rust twin for these cases,
 * engine/tests/test_modules/effects/conditional/per_card/
 * wien_live_start_empty_hand_energy_placement_test.rs:145 and :179, calls
 * crate::helpers::fire_trigger for both. Hence nth == 0, spelled through the
 * _nth entry point to keep the loop -> nth mapping explicit. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    return test_fire_trigger_on_card_nth(tg, cid, trig, 0);
}

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
            "live=%d success=%d revealed=%d bonus=%d\n",
            card_no_of(P->stage[0]), card_no_of(P->stage[1]), card_no_of(P->stage[2]),
            P->stage_wait[0], P->stage_wait[1], P->stage_wait[2],
            P->live.n, P->success.n, tg->state.n_revealed,
            (int)tg->state.mods.p1_constant_total_score_bonus);
    dump_zone("hand", &P->hand);
    dump_zone("deck", &P->deck);
    dump_zone("waitroom", &P->discard);
    dump_zone("live", &P->live);
    dump_zone("success", &P->success);
    RbPlayer *Q = &tg->state.p[1];
    fprintf(stderr, "        board p2: stage=[%s,%s,%s] live=%d success=%d\n",
            card_no_of(Q->stage[0]), card_no_of(Q->stage[1]), card_no_of(Q->stage[2]),
            Q->live.n, Q->success.n);
}

static void dump_snapshots(TestGame *tg)
{
    fprintf(stderr, "        snapshots n=%d\n", tg->state.n_snapshots);
    for (int i = 0; i < tg->state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &tg->state.snapshots[i];
        fprintf(stderr, "          [%d] player=%d success=%d n_lives=%d total=%d "
                "n_yell=%d\n", i, s->player, s->success, s->n_lives,
                s->total_score, s->n_yell_cards);
        for (int j = 0; j < s->n_lives && j < 8; j++)
            fprintf(stderr, "             live[%d]=%s score=%d passed=%d\n", j,
                    card_no_of(s->lives[j]), s->live_score_detail[j], s->live_passed[j]);
        for (int j = 0; j < s->n_yell_cards && j < 12; j++)
            fprintf(stderr, "             yell[%d]=%s\n", j, card_no_of(s->yell_cards[j]));
    }
}

static void dump_choice(TestGame *tg)
{
    const RbChoice *ch = rb_get_pending_choice(&tg->state);
    if (!ch) { DIAG("no pending choice"); return; }
    DIAG("choice: type=%s zone=%s count=%d allow_skip=%d actor=%d",
         pending_kind(tg), ch->zone, ch->count, ch->allow_skip, ch->actor);
}

/* =======================================================================
 * §A  ability_mod/chisato_test.rs
 *      chisato_q129_cost_reduction_affects_threshold
 *
 * 門田 PL!SP-bp1-003-P 起動/ターン1回: 「自分の手札のカードを1枚公開する。
 * 公開したカードのコストの合計が10, 20なら、【常時】ライブの合計スコアを＋１する。」
 *
 * The Rust twin hands Chisato exactly ONE card in hand -- the joint LL card --
 * whose printed cost is 2, so the ladder is missed at 2. The assertion is that
 * the revealed card STAYS IN HAND (公開する is a reveal, not a move) and that
 * the gained ability never lands. The C suite already covers 20/10/8/12 on a
 * five-card hand; the one-card hand is the case where the pool is EMPTY of any
 * ladder-matching combination, so it is a different code path for the filter.
 * ======================================================================= */
static void A1_chisato_q129_cost_reduction_affects_threshold(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int chisato = test_new_id(&tg, CHISATO);
    int joint    = test_new_id(&tg, JOINT_LL);
    int filler   = test_id(&tg, DECK_FILLER);
    if (!pin_id(chisato, CHISATO) || !pin_id(joint, JOINT_LL)) return;

    /* The whole point of the fixture: the ONLY card in hand costs 2, so no
     * subset of it can total 10 or 20. Pin the printed cost from the ENGINE's
     * decoded Card so a cards.json/cards.bin disagreement cannot hide. */
    CHECK_EQ(printed_cost(joint), 2,
             "A1: the single hand card really costs 2, so 10 and 20 are unreachable");

    set_stage(&tg, 0, chisato, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    test_add_to_hand(&tg, joint);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "A1 setup: exactly one card is in hand");
    test_give_energy(&tg, 15);

    int hand_before = tg.state.p[0].hand.n;
    int queued = test_activate_ability(&tg, chisato);
    dump_board(&tg);

    /* VACUITY CONTROL: the reveal prompt must actually open. If it did not, the
     * "no bonus" assertion below would pass for the wrong reason -- the ability
     * never ran at all, which is a different defect from the gate refusing. */
    CHECK(queued, "A1 CONTROL: the 起動 is accepted with 15 energy against a 0 cost");
    CHECK(pending(&tg), "A1 CONTROL: 公開する opens a prompt from the one-card hand");
    if (pending(&tg)) {
        dump_choice(&tg);
        CHECK(pending_kind(&tg) && !strcmp(pending_kind(&tg), "SelectCard"),
              "A1 CONTROL: the prompt is a SelectCard");
        /* Rust: game.select_indices(&[]) then drain_choices(). An empty
         * selection is Rust's DECLINE, which still completes the reveal step. */
        test_select_indices(&tg, NULL, 0);
    }
    decline_choices(&tg);

    /* The reveal is a reveal, not a move. */
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "A1: 公開する does not move the card -- the hand size is unchanged");
    CHECK(test_hand_has(&tg, joint), "A1: the revealed joint card stayed in hand");

    /* THE NEGATIVE HALF: the gate must refuse the +1. */
    CHECK_EQ((int)tg.state.mods.p1_constant_total_score_bonus, 0,
             "A1: a revealed total of 2 is on neither 10 nor 20, so no live-total bonus");
    CHECK_EQ(test_get_score_modifier(&tg, chisato), 0,
             "A1: no per-card score modifier is granted when the gate fails");
}

/* =======================================================================
 * §B  ability_mod/full_distinct_group_stage_grants_yell_live_score_test.rs
 *      Four tests whose positive half is a FULL live performance, which the
 *      existing C suite never drives: it evaluates Mari's delayed gained effect
 *      in isolation (test_zero_live_cards_no_bonus and siblings). These run the
 *      real yell -> snapshot path.
 *
 *      国木田花丸 PL!S-bp2-008-R＋ 【常時】: all three stage areas hold 『Aqours』
 *      members with DIFFERENT names -> on the ライブ成功時, count the LIVE CARDS
 *      among the yell reveals: 0 -> nothing, 1 or 2 -> live total score +1,
 *      3 or more -> +2 (the conditional_alternative branch).
 *
 *      The four Rust tests are B1 (one owner's full performance) and B2/B3/B4,
 *      which all funnel through assert_owner_gameplay over a 5x5 reveal matrix
 *      and two failure modes. That helper is ported verbatim below.
 * ======================================================================= */

/* Rust setup_stage_with_3_aqours. Uses the two abilityless N-rares so their
 * own abilities cannot interfere. Returns 1 when all three resolved. */
static int mari_stage_p1(TestGame *tg, int out[3])
{
    out[0] = test_new_id(tg, MARI);
    out[1] = test_new_id(tg, RIKO);
    out[2] = test_new_id(tg, DIA);
    if (out[0] < 0 || out[1] < 0 || out[2] < 0) return 0;
    set_stage(tg, 0, out[0], out[1], out[2]);
    return 1;
}

/* Rust fill_unique_decks: 4 copies each of four prints per seat, plus one
 * energy card per energy deck. Distinct instances matter because the yells are
 * matched by identity later. */
static void fill_unique_decks(TestGame *tg)
{
    static const char *const prints[4] = {DECK_FILLER, RIKO, DECK_012, DIA};
    for (int r = 0; r < 4; r++)
        for (int k = 0; k < 4; k++)
            for (int pl = 0; pl < 2; pl++)
                test_add_to_deck_pl(tg, pl, test_new_id(tg, prints[k]));
    for (int pl = 0; pl < 2; pl++)
        test_add_to_energy_deck(tg, pl, test_new_id(tg, ENERGY));
}

/* Rust run_live_through_victory: five passes to LiveCardSet, set the live,
 * two more passes into the performance, then execute_live_victory_determination.
 * The C batch path (live_batch_mode + rb_perform_live) is the idiom peers use
 * for a deterministic performance; the hand-rolled pass loop is
 * test_advance_to_phase_rust's job and is NOT used here because it depends on
 * a drained walk. */
static void run_performance(TestGame *tg)
{
    tg->state.live_batch_mode = 1;
    rb_perform_live(&tg->state, 0);
    rb_perform_live(&tg->state, 1);
    tg->state.live_batch_mode = 0;
    rb_execute_live_victory_determination(&tg->state);
    drain_auto(tg);
}

static const RbLiveSnapshot *snapshot_for(const TestGame *tg, int pl)
{
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == pl)
            return &tg->state.snapshots[i];
    return NULL;
}

/* how many of the first `n` yell entries are the YELL_LIVE print */
static int yell_live_prefix(const RbLiveSnapshot *s, int n)
{
    int c = 0;
    for (int i = 0; i < s->n_yell_cards && i < n; i++)
        if (strcmp(card_no_of(s->yell_cards[i]), YELL_LIVE) == 0) c++;
    return c;
}

/* ── B1: score_bonus_applied_with_revealed_live_cards ─────────────────── */
static void B1_score_bonus_applied_with_revealed_live_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int stage_ids[3];
    if (!mari_stage_p1(&tg, stage_ids)) { CHECK(0, "B1 setup: the three Aqours members must resolve"); return; }
    int mari = stage_ids[0];
    if (!pin_id(mari, MARI) || !pin_id(stage_ids[1], RIKO) ||
        !pin_id(stage_ids[2], DIA)) return;

    fill_unique_decks(&tg);

    int live_card = test_new_id(&tg, YELL_LIVE);
    int yell_lives[3];
    for (int i = 0; i < 3; i++) yell_lives[i] = test_new_id(&tg, YELL_LIVE);
    if (!pin_id(live_card, YELL_LIVE)) return;
    CHECK_EQ(printed_score(live_card), 1, "B1: the live card's printed score is 1");

    /* Rust: `let draw_card = deck.remove(0)` then three `deck.insert(0, ..)` in
     * REVERSE. Both bags are top-index-0, so the net effect on the deck is:
     * the first four cards become [yell_lives[0], yell_lives[1], yell_lives[2],
     * draw_card], and every card below index 4 is untouched. */
    RbPlayer *P1 = &tg.state.p[0];
    if (P1->deck.n < 4) { CHECK(0, "B1 setup: the filled deck must have at least 4 cards"); return; }
    int saved[4] = {P1->deck.cards[0], P1->deck.cards[1],
                    P1->deck.cards[2], P1->deck.cards[3]};
    int draw_card = saved[0];

    P1->deck.cards[0] = yell_lives[0];
    P1->deck.cards[1] = yell_lives[1];
    P1->deck.cards[2] = yell_lives[2];
    P1->deck.cards[3] = draw_card;

    CHECK_EQ(P1->deck.cards[0], yell_lives[0],
             "B1 setup: yell_lives[0] really is the top of p1's deck");
    CHECK_EQ(P1->deck.cards[2], yell_lives[2],
             "B1 setup: yell_lives[2] really is the third card");
    CHECK_EQ(P1->deck.cards[3], draw_card,
             "B1 setup: the drawn card sits directly under the three yell lives");
    test_add_to_hand(&tg, live_card);
    test_set_live_card(&tg, 0, live_card);

    /* +1 Heart06 on the CENTRE member is what makes a score-1 live pass. */
    rb_mods_add_heart(&tg.state.mods, stage_ids[1], HEART06_IDX, 1);
    for (int i = 0; i < 3; i++) test_recalc(&tg);
    CHECK_EQ(tg.state.n_delayed_gained_effects, 1,
             "B1 CONTROL: Mari's delayed gained effect is registered before the live");

    run_performance(&tg);
    dump_board(&tg);
    dump_snapshots(&tg);

    CHECK(!pending(&tg), "B1: the performance left no prompt parked");
    CHECK_EQ((int)tg.state.mods.p1_constant_total_score_bonus, 2,
             "B1: 3 live cards among the reveals -> the alternative branch grants +2");
    CHECK_EQ((int)tg.state.mods.p2_constant_total_score_bonus, 0,
             "B1: the opponent gains nothing from Mari's yell");
    CHECK_EQ(test_get_score_modifier(&tg, mari), 0,
             "B1: the printed target is the live total score, not Mari's own score");
    CHECK_EQ(test_get_score_modifier(&tg, live_card), 0,
             "B1: the printed target is the live total score, not the live card's score");

    const RbLiveSnapshot *s = snapshot_for(&tg, 0);
    if (!s) { CHECK(0, "B1 CONTROL: Mari's owner must have a performance snapshot"); return; }
    CHECK_EQ(s->success, 1, "B1 CONTROL: the owner's live really succeeded");
    CHECK_EQ(s->n_lives, 1, "B1 CONTROL: the performance holds exactly one live card");
    CHECK_EQ(s->lives[0], live_card, "B1 CONTROL: that live card is the identity we staged");
    CHECK_EQ(s->live_score_detail[0], 1, "B1 CONTROL: the live card scored 1");
    CHECK_EQ(s->total_score, 6, "B1: total 1 (live) + 2 (yell lives) + 2 (all three) + 1 (the bonus)");

    /* The yell reveal must be the three lives we put on top, in order, and must
     * NOT contain the live card or the drawn card. */
    CHECK_EQ(yell_live_prefix(s, 3), 3,
             "B1: the first three yell cards are the three 『PL!SP-sd1-023-SD』 lives");
    for (int i = 0; i < s->n_yell_cards; i++)
        CHECK(s->yell_cards[i] != live_card,
              "B1: the live card itself is never among the yell cards");
}

/* ── B2/B3/B4 shared driver: Rust assert_owner_gameplay ────────────────────
 *   reveal_counts[2] : how many YELL_LIVE cards each seat puts on top
 *   sets_live[2]     : whether that seat sets a live card at all
 *   succeeds[2]      : whether that seat's +1 Heart06 is applied (live passes)
 *
 * The expected bonus is 0 / 1 / 1 / 2 for reveal counts 0 / 1 / 2 / 3+, gated on
 * `success`, which is `sets_live && succeeds`. Every board asserts the CONTROL
 * facts (snapshot exists, live really is/isn't there, live really passed/didn't)
 * BEFORE the bonus assertion, so a 0 in the matrix can only be the gate. */
static void assert_owner_gameplay(const int reveal_counts[2], const int sets_live[2],
                                 const int succeeds[2])
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int p1_stage[3];
    if (!mari_stage_p1(&tg, p1_stage)) { CHECK(0, "B* setup: p1's three Aqours members must resolve"); return; }
    if (!pin_id(p1_stage[0], MARI)) return;

    int p2_stage[3];
    p2_stage[0] = test_new_id(&tg, MARI);
    p2_stage[1] = test_new_id(&tg, RIKO);
    p2_stage[2] = test_new_id(&tg, DIA);
    if (p2_stage[0] < 0 || p2_stage[1] < 0 || p2_stage[2] < 0) {
        CHECK(0, "B* setup: p2's three Aqours members must resolve");
        return;
    }
    set_stage(&tg, 1, p2_stage[0], p2_stage[1], p2_stage[2]);

    fill_unique_decks(&tg);

    int live_cards[2];
    live_cards[0] = test_new_id(&tg, YELL_LIVE);
    live_cards[1] = test_new_id(&tg, YELL_LIVE);
    if (!pin_id(live_cards[0], YELL_LIVE) || !pin_id(live_cards[1], YELL_LIVE)) return;

    int stages[2][3] = {{p1_stage[0], p1_stage[1], p1_stage[2]},
                        {p2_stage[0], p2_stage[1], p2_stage[2]}};
    for (int owner = 0; owner < 2; owner++) {
        if (succeeds[owner]) rb_mods_add_heart(&tg.state.mods, stages[owner][1], HEART06_IDX, 1);
        if (sets_live[owner])   test_add_to_hand_for(&tg, owner, live_cards[owner]);
    }

    for (int i = 0; i < 3; i++) {
        test_recalc(&tg);
        CHECK_EQ(tg.state.n_delayed_gained_effects, 2,
                 "B*: CONTROL -- BOTH owners register a delayed gained effect");
    }

    /* Yell lives go on top IN REVERSE, per owner, so yell_lives[owner][0] ends
     * up on top of that seat's deck. */
    int yell_lives[2][4];
    for (int owner = 0; owner < 2; owner++)
        for (int i = 0; i < reveal_counts[owner]; i++)
            yell_lives[owner][i] = test_new_id(&tg, YELL_LIVE);
    for (int owner = 0; owner < 2; owner++)
        for (int i = reveal_counts[owner] - 1; i >= 0; i--)
            test_insert_deck_top(&tg, owner, yell_lives[owner][i]);

    for (int owner = 0; owner < 2; owner++)
        if (sets_live[owner]) test_set_live_card_for(&tg, owner, live_cards[owner]);

    run_performance(&tg);
    if (getenv("AMC_DEBUG")) { dump_board(&tg); dump_snapshots(&tg); }

    for (int owner = 0; owner < 2; owner++) {
        int success = sets_live[owner] && succeeds[owner];
        int bonus = 0;
        if (success)
            bonus = (reveal_counts[owner] == 0) ? 0
                 : (reveal_counts[owner] <= 2) ? 1 : 2;

        char msg[192];
        snprintf(msg, sizeof msg,
                 "B*: owner=%d reveals=%d sets_live=%d succeeds=%d -> success=%d bonus=%d",
                 owner, reveal_counts[owner], sets_live[owner], succeeds[owner],
                 success, bonus);

        const RbLiveSnapshot *s = snapshot_for(&tg, owner);
        if (!s) {
            snprintf(msg, sizeof msg,
                     "B*: CONTROL -- owner=%d must have a performance snapshot "
                     "(reveals=%d sets_live=%d)", owner, reveal_counts[owner], sets_live[owner]);
            CHECK(0, msg);
            continue;
        }

        snprintf(msg, sizeof msg, "B*: owner=%d snapshot.success must be %d", owner, success);
        CHECK_EQ(s->success, success, msg);
        snprintf(msg, sizeof msg, "B*: owner=%d performed %d live card(s)", owner,
                 sets_live[owner]);
        CHECK_EQ(s->n_lives, sets_live[owner], msg);

        if (sets_live[owner]) {
            snprintf(msg, sizeof msg, "B*: owner=%d's live card is the identity staged",
                     owner);
            CHECK_EQ(s->lives[0], live_cards[owner], msg);
            snprintf(msg, sizeof msg, "B*: owner=%d's live passed=%d", owner, succeeds[owner]);
            CHECK_EQ(s->live_passed[0], succeeds[owner], msg);

            /* CONTROL: the yell really contained the lives we put on top. */
            snprintf(msg, sizeof msg,
                     "B*: CONTROL -- owner=%d revealed %d live card(s) in the yell",
                     owner, reveal_counts[owner]);
            CHECK_EQ(yell_live_prefix(s, reveal_counts[owner]), reveal_counts[owner], msg);
        }

        /* THE ASSERTION UNDER TEST. total_score is 1 (live) + reveals + bonus
         * when the live succeeded, and 0 when it did not. */
        int expected_total = success ? (1 + reveal_counts[owner] + bonus) : 0;
        snprintf(msg, sizeof msg, "B*: owner=%d reveals=%d -> total_score",
                 owner, reveal_counts[owner]);
        CHECK_EQ(s->total_score, expected_total, msg);

        int actual_bonus = owner == 0 ? (int)tg.state.mods.p1_constant_total_score_bonus
                                      : (int)tg.state.mods.p2_constant_total_score_bonus;
        snprintf(msg, sizeof msg, "B*: owner=%d reveals=%d -> constant live-total bonus",
                 owner, reveal_counts[owner]);
        CHECK_EQ(actual_bonus, bonus, msg);

        snprintf(msg, sizeof msg,
                 "B*: owner=%d -- no per-card modifier on its live card", owner);
        CHECK_EQ(test_get_score_modifier(&tg, live_cards[owner]), 0, msg);
        snprintf(msg, sizeof msg,
                 "B*: owner=%d -- no per-card modifier on Mari", owner);
        CHECK_EQ(test_get_score_modifier(&tg, stages[owner][0]), 0, msg);
    }
}

/* successful_owners_use_their_own_zero_one_or_three_live_reveals: the full 5x5
 * matrix of reveal counts. reveal_counts == 0 on BOTH seats is the
 * zero-live-reveal control that makes every non-zero cell meaningful. */
static void B2_successful_owners_use_their_own_zero_one_or_three_live_reveals(void)
{
    for (int p1 = 0; p1 <= 4; p1++)
        for (int p2 = 0; p2 <= 4; p2++) {
            int reveals[2] = {p1, p2};
            int sets[2] = {1, 1};
            int succ[2] = {1, 1};
            assert_owner_gameplay(reveals, sets, succ);
        }
}

/* failed_owner_gets_no_bonus_even_with_three_live_reveals: BOTH seats reveal
 * three live cards, but one of them FAILS. That owner's bonus must be 0 even
 * though its reveal set is identical to the passing owner's -- this is the
 * half that separates "the gate read the reveals" from "the gate always fires". */
static void B3_failed_owner_gets_no_bonus_even_with_three_live_reveals(void)
{
    for (int owner = 0; owner < 2; owner++) {
        int reveals[2] = {3, 3};
        int sets[2] = {1, 1};
        int succ[2] = {1, 1};
        succ[owner] = 0;
        assert_owner_gameplay(reveals, sets, succ);
    }
}

/* owner_without_live_gets_no_bonus_from_opponents_reveals: BOTH seats would
 * reveal three live cards and both would pass, but one seat never SETS a live
 * card. An owner with no live card performed has nothing for its own yell to
 * score against, and must not pick up the OPPONENT's reveal count. */
static void B4_owner_without_live_gets_no_bonus_from_opponents_reveals(void)
{
    for (int owner = 0; owner < 2; owner++) {
        int reveals[2] = {3, 3};
        int sets[2] = {1, 1};
        int succ[2] = {1, 1};
        sets[owner] = 0;
        assert_owner_gameplay(reveals, sets, succ);
    }
}

/* =======================================================================
 * §C  conditional/per_card/hanamaru_debut_result_character_deploy_test.rs
 *      hanamaru_both_matching_one_deploys
 *
 * 國木田花丸 PL!S-bp7-007-R＋ 登場: 「自分の控え室からコスト2以下のメンバーカードを
 * 1枚手札に加える。これによって『津島善子』か『黒澤ルビィ』を手札に加えた場合、
 * そのカードを自分のステージのメンバーのいないエリアに登場させてもよい。」
 *
 * The existing gated suite covers 6 of this file's 7 tests. The missing one is
 * the ONLY case where two candidates are present AND index 0 is pinned to a
 * named card, so it is also the only one that can prove the UNCHOSEN card stays
 * out of hand. test_parity_conditional_gated.c's
 * hanamaru_multiple_valid_forces_selection accepts EITHER card landing on
 * stage[2] and never checks the loser.
 * ======================================================================= */
static void hanamaru_fire_debut(TestGame *tg, int hanamaru)
{
    test_fire_debut(tg, hanamaru);
    rb_process_pending_auto_abilities(&tg->state);
}

static void C1_hanamaru_both_matching_one_deploys(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int hanamaru = test_new_id(&tg, HANAMARU);
    int filler   = test_id(&tg, DECK_FILLER);
    int tsushima = test_new_id(&tg, TSUSHIMA);
    int ruby     = test_new_id(&tg, RUBY);
    if (!pin_id(hanamaru, HANAMARU) || !pin_id(tsushima, TSUSHIMA) || !pin_id(ruby, RUBY))
        return;

    /* Pin both printed facts the ability's TWO gates lean on: the cost filter
     * (「コスト2以下」) and the result condition's `characters` list. */
    CHECK_EQ(printed_cost(tsushima), 2, "C1: 津島善子 really costs 2 (inside the <=2 cost gate)");
    CHECK_EQ(printed_cost(ruby), 2, "C1: 黒澤ルビィ really costs 2 (inside the <=2 cost gate)");
    CHECK(tsushima != ruby, "C1 CONTROL: the two candidates are DISTINCT instances");

    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(&tg, pl, filler);

    set_stage(&tg, 0, filler, hanamaru, RB_EMPTY_SLOT);
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT,
             "C1 setup guard: the right area really is the empty one");
    test_add_to_discard(&tg, tsushima);
    test_add_to_discard(&tg, ruby);
    CHECK_EQ(tg.state.p[0].discard.n, 2, "C1 setup guard: both candidates are in the waitroom");

    hanamaru_fire_debut(&tg, hanamaru);

    /* VACUITY CONTROL: two valid candidates MUST force a pick. */
    CHECK(pending(&tg), "C1 CONTROL: two matching cards force a selection prompt");
    if (!pending(&tg)) return;
    dump_choice(&tg);
    CHECK(pending_kind(&tg) && !strcmp(pending_kind(&tg), "SelectCard"),
          "C1 CONTROL: the prompt is a SelectCard");

    /* Rust: game.select_indices(&[0]) -> 津島善子. */
    pick(&tg, 0);
    int guard = 0;
    while (pending(&tg) && guard++ < 12) {
        const char *k = pending_kind(&tg);
        if (k && (!strcmp(k, "SelectCard") || !strcmp(k, "SelectTarget") ||
                  !strcmp(k, "SelectPosition") || !strcmp(k, "SelectAutoAbility")))
            pick(&tg, 0);
        else
            skip_pick(&tg);
    }
    dump_board(&tg);

    CHECK(!pending(&tg), "C1: the pick resolves the prompt");

    /* With stage[2] the only free area, the deploy target is exact. */
    CHECK_EQ(tg.state.p[0].stage[2], tsushima,
             "C1: the selected 津島善子 deploys to the empty stage[2]");
    /* THE OTHER HALF, which no existing C test asserts: the unchosen card must
     * NOT have been deployed and must NOT have reached hand. */
    CHECK(!test_hand_has(&tg, ruby),
          "C1: the UNCHOSEN 黒澤ルビィ stays out of hand");
    CHECK(tg.state.p[0].stage[0] != ruby && tg.state.p[0].stage[1] != ruby,
          "C1: the UNCHOSEN 黒澤ルビィ did not deploy to any area");
}

/* =======================================================================
 * §D  conditional/per_card/wien_live_start_empty_hand_energy_placement_test.rs
 *      wien_live_success_choice_draw_via_trigger
 *      wien_live_success_choice_energy_via_trigger
 *
 * ウィーン・マルガレーテ PL!SP-pb2-010-R ライブ成功時: an OPTIONAL prompt
 * offering TWO named branches -- option 0 = 「手札を1枚控え室に置く」 (draw 2),
 * option 1 = 「自分のエネルギー1枚をエネルギーデッキに置く」.
 *
 * ── HARNESS GAP, and it is load-bearing ─────────────────────────────────
 * Rust's select_option(n) picks the n-th NAMED option. The flat RbChoice in C
 * carries no option list, so from here "option 0" and "option 1" are the same
 * single integer with no way to establish which branch it names. Asserting
 * "hand grew by 2" therefore cannot distinguish the Rust option 0 from a
 * mis-mapped Rust option 1. Those two assertions are recorded as
 * EXPECTED_GAP with a printed note rather than being reported as passes or
 * quietly dropped. What IS observable and is asserted strictly is the prompt
 * APPEARING and the queue draining -- which is the vacuity control: without it
 * both Rust tests would pass on a Wiener whose LiveSuccess never fired.
 * ======================================================================= */
/* Vienna on the centre with both decks fed; returns 0 if the identity failed. */
static int wien_board(TestGame *tg, int *wien_out)
{
    int wien   = test_new_id(tg, WIEN);
    int filler = test_id(tg, DECK_FILLER);
    if (!pin_id(wien, WIEN)) return 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < 10; i++) test_add_to_deck_pl(tg, pl, filler);
    set_stage(tg, 0, RB_EMPTY_SLOT, wien, RB_EMPTY_SLOT);
    *wien_out = wien;
    return 1;
}

static void D1_wien_live_success_choice_draw_via_trigger(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int wien = 0;
    if (!wien_board(&tg, &wien)) return;

    int hand_before = tg.state.p[0].hand.n;
    int fired = fire_trigger(&tg, wien, "ライブ成功時");
    dump_board(&tg);

    /* VACUITY CONTROL -- the load-bearing assertion of §D. */
    CHECK(fired, "D1 CONTROL: Wien's ライブ成功時 ability is queued and runs");
    CHECK(pending(&tg), "D1 CONTROL: the optional branch prompt APPEARS");
    DIAG("D1: hand_before=%d; RbChoice carries no option list, so branch identity "
         "is not observable from C (see the EXPECTED_GAP below).", hand_before);
    if (pending(&tg)) {
        dump_choice(&tg);
        DIAG("D1: RbChoice carries no option list, so branch identity is not "
             "observable from C; see the EXPECTED_GAP below.");
        pick(&tg, 0);
    }
    drain_auto(&tg);

    EXPECTED_GAP("D1: Rust asserts hand == hand_before + 2 after select_option(0) "
                 "(the 「手札を1枚控え室に置く」 draw-2 branch). The flat RbChoice "
                 "exposes no option list, so index 0 cannot be shown to name the "
                 "draw branch, and the C hand delta is therefore NOT asserted "
                 "here. If this gap is closed, assert "
                 "p[0].hand.n == hand_before + 2.");
}

static void D2_wien_live_success_choice_energy_via_trigger(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int wien = 0;
    if (!wien_board(&tg, &wien)) return;

    /* Rust gives Wien energy here so the second branch is legal at all. */
    test_give_energy(&tg, 3);
    CHECK(tg.state.p[0].energy.n >= 1, "D2 setup guard: the energy branch is legal");

    int hand_before = tg.state.p[0].hand.n;
    int fired = fire_trigger(&tg, wien, "ライブ成功時");
    dump_board(&tg);

    CHECK(fired, "D2 CONTROL: Wien's ライブ成功時 ability is queued and runs");
    CHECK(pending(&tg), "D2 CONTROL: the optional branch prompt APPEARS");
    DIAG("D2: hand_before=%d; RbChoice carries no option list, so branch identity "
         "is not observable from C (see the EXPECTED_GAP below).", hand_before);
    if (pending(&tg)) {
        dump_choice(&tg);
        DIAG("D2: RbChoice carries no option list, so branch identity is not "
             "observable from C; see the EXPECTED_GAP below.");
        pick(&tg, 1);
    }
    drain_auto(&tg);

    EXPECTED_GAP("D2: Rust asserts hand == hand_before after select_option(1) "
                 "(the 「エネルギー1枚をエネルギーデッキに置く」 branch) and that the "
                 "choice resolves. The flat RbChoice exposes no option list, so "
                 "index 1 cannot be shown to name the energy branch; the draw "
                 "count is therefore NOT asserted here. If this gap is closed, "
                 "assert p[0].hand.n == hand_before and !has_pending_choice.");
}

/* =======================================================================
 * §E  conditional/state_gated/center_highest_cost_constant_heart_{edges,tie}_test.rs
 *      Three tests the gated suite folds in as controls rather than as tests of
 *      their own:
 *        E1 sp_bp2_004_single_center_alone_gains
 *        E2 sp_bp2_004_center_tie_with_both_sides_no_heart
 *        E3 sp_bp2_004_center_highest_with_empty_side
 *
 * 平安名すみれ PL!SP-bp2-004-R 【常時】: 「自分のステージにいるメンバーのうち、
 * センターエリアにいるメンバーが最も大きいコストを持つ場合、heart03を得る。」
 *
 * "最も大きい" is STRICTLY greater. E1 is the vacuously-highest base case with
 * no competitors at all; E2 is the strictness test (a TIE is not "greatest");
 * E3 is the asymmetric case where only ONE side has a member, so the centre can
 * be highest while a side is empty. All three read the mods store directly,
 * which is what makes them immune to the constant-scan dispatch: the gate has
 * already been evaluated by the time the value is in mods.heart[].
 * ======================================================================= */
static void E1_sp_bp2_004_single_center_alone_gains(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int sumire = test_new_id(&tg, SUMIRE_R);
    if (!pin_id(sumire, SUMIRE_R)) return;
    CHECK_EQ(printed_cost(sumire), 9, "E1: the host really costs 9");

    /* Vacuum: nothing else is on stage, so the centre cannot lose. */
    set_stage(&tg, 0, RB_EMPTY_SLOT, sumire, RB_EMPTY_SLOT);
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "E1 setup guard: the left area is empty");
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT, "E1 setup guard: the right area is empty");

    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 1,
             "E1: alone in the centre it is vacuously the highest-cost member -> heart03");

    /* And the converse, so E1's +1 is not "any recalculation grants heart03": */
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX) - 1, 0,
             "E1 CONTROL: heart03 is exactly +1, not some larger value");
}

static void E2_sp_bp2_004_center_tie_with_both_sides_no_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int sumire = test_new_id(&tg, SUMIRE_R);
    int nine_a = test_new_id(&tg, COST_9);
    int nine_b = test_new_id(&tg, COST_9);
    if (!pin_id(sumire, SUMIRE_R)) return;
    CHECK_EQ(printed_cost(nine_a), 9, "E2: the centre rival really costs 9");
    CHECK_EQ(printed_cost(nine_b), 9, "E2: the right rival really costs 9");
    CHECK_EQ(printed_cost(sumire), 9, "E2: the host itself really costs 9");
    CHECK(nine_a != nine_b, "E2 setup guard: the two rivals are DISTINCT instances");

    /* All three cost 9 -> the centre is NOT strictly greatest. */
    set_stage(&tg, 0, sumire, nine_a, nine_b);
    test_recalc(&tg);

    /* THE NEGATIVE HALF. */
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 0,
             "E2: a three-way tie at cost 9 is not 「最も大きい」 -> no heart03");

    /* NON-VACUITY CONTROL: break the tie by one point on the RIGHT only, and
     * the identical board must now grant. Nothing else changed, so the 0 above
     * is the tie and not a dead evaluator. */
    int eleven = test_new_id(&tg, COST_11);
    if (!pin_id(eleven, COST_11)) return;
    CHECK_EQ(printed_cost(eleven), 11, "E2 CONTROL: the breaking rival really costs 11");
    set_stage(&tg, 0, sumire, nine_a, eleven);
    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 1,
             "E2 CONTROL: with the right raised to 11 the centre IS strictly greatest -> heart03");

    /* And restoring the tie removes it again -- the gate re-evaluates. */
    set_stage(&tg, 0, sumire, nine_a, nine_b);
    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 0,
             "E2 CONTROL: restoring the tie removes heart03 again");
}

static void E3_sp_bp2_004_center_highest_with_empty_side(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    rb_card_copy_pool_reset();

    int sumire     = test_new_id(&tg, SUMIRE_R);
    int center_high= test_new_id(&tg, COST_11);
    int right_low  = test_new_id(&tg, COST_4B);
    if (!pin_id(sumire, SUMIRE_R)) return;
    CHECK_EQ(printed_cost(center_high), 11, "E3: the centre member really costs 11");
    CHECK_EQ(printed_cost(right_low), 4, "E3: the right member really costs 4");

    /* Asymmetric: the centre is populated, the right is EMPTY. An absent member
     * is not a rival, so the centre is still strictly greatest. */
    set_stage(&tg, 0, sumire, center_high, RB_EMPTY_SLOT);
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT,
             "E3 setup guard: the right area really is empty, not cost 0");
    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 1,
             "E3: centre 11 > host 9 with the right area EMPTY -> heart03");

    /* NON-VACUITY CONTROL: putting a HIGHER member in that empty area must
     * remove it. Same board otherwise, so the +1 above is the empty-side case
     * and not "a populated centre always grants". */
    int right15 = test_new_id(&tg, "PL!-sd1-009-SD");
    if (!pin_id(right15, "PL!-sd1-009-SD")) return;
    CHECK(printed_cost(right15) > 11,
          "E3 CONTROL: the right fixture really costs more than the centre (11)");
    set_stage(&tg, 0, sumire, center_high, right15);
    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 0,
             "E3 CONTROL: a right member above 11 outranks the centre -> no heart03");

    /* And emptying it again restores the grant. */
    set_stage(&tg, 0, sumire, center_high, RB_EMPTY_SLOT);
    test_recalc(&tg);
    CHECK_EQ(heart_mod(&tg, sumire, HEART03_IDX), 1,
             "E3 CONTROL: emptying the right area restores heart03");
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
        printf("        %-64s %4d assertion(s), %d failure(s), %d harness gap(s)\n",
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

    printf("--- A. ability_mod/chisato_test.rs ---\n");
    run("A1_q129_cost_reduction", A1_chisato_q129_cost_reduction_affects_threshold);

    printf("--- B. ability_mod/full_distinct_group_stage_grants_yell_live_score_test.rs ---\n");
    run("B1_score_bonus_full_performance", B1_score_bonus_applied_with_revealed_live_cards);
    run("B2_reveal_matrix_5x5",
        B2_successful_owners_use_their_own_zero_one_or_three_live_reveals);
    run("B3_failed_owner_no_bonus", B3_failed_owner_gets_no_bonus_even_with_three_live_reveals);
    run("B4_no_live_no_opponent_bonus",
        B4_owner_without_live_gets_no_bonus_from_opponents_reveals);

    printf("--- C. conditional/per_card/hanamaru_debut_result_character_deploy_test.rs ---\n");
    run("C1_both_matching_one_deploys", C1_hanamaru_both_matching_one_deploys);

    printf("--- D. conditional/per_card/wien_live_start_empty_hand_energy_placement_test.rs ---\n");
    run("D1_live_success_draw_branch", D1_wien_live_success_choice_draw_via_trigger);
    run("D2_live_success_energy_branch", D2_wien_live_success_choice_energy_via_trigger);

    printf("--- E. conditional/state_gated/center_highest_cost_constant_heart_*_test.rs ---\n");
    run("E1_single_center_alone", E1_sp_bp2_004_single_center_alone_gains);
    run("E2_center_tie_no_heart", E2_sp_bp2_004_center_tie_with_both_sides_no_heart);
    run("E3_highest_with_empty_side", E3_sp_bp2_004_center_highest_with_empty_side);

    rb_unload();
    printf("\n==== ability_mod/conditional DELTA parity: %d cases passed, %d cases failed, "
           "%d crashed, %d harness gap(s), %d/%d assertions ====\n",
           n_ok, n_failed, n_crashed, harness_gaps,
           assertions - failures, assertions);
    return (failures || n_crashed) ? 1 : 0;
}
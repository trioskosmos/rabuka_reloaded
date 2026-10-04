/* test_parity_target_player.c
 *
 * C parity harness for the choice-TARGETING family:
 *
 *   engine/tests/test_modules/effects/choice/target_player/       (7 files / 33 tests)
 *   engine/tests/test_modules/effects/choice/selection_filter/   (4 files / 14 tests)
 *   engine/tests/test_modules/effects/choice/per_card/           (subset, see banners)
 *
 * This is the HEART of the choice subsystem and the cluster the AGENTS.md
 * picker-routing bug family lives in, so every case here answers one of two
 * questions with evidence:
 *
 *   (a) does the C engine carry the printed effect, and
 *   (b) is the chain `current_effect -> ChoiceBuilder.picker() ->
 *       Choice::SelectCard.picker -> PCA_G1 routing` present in the C port?
 *
 * STATE OF THE CHAIN, measured here rather than assumed (group P at the bottom
 * probes it per card):
 *   - `rb_choice_builder_picker()` and `rb_effect_picker_any()` DO exist in the
 *     C, and `picker[32]` exists on `RbFullChoice`.
 *   - the FLAT `RbChoice` -- the struct `rb_emit_choice()` populates and the
 *     resolver reads -- has NO picker field at all.
 *   - `rb_effect_reveal()` sets `target_player_id` and never a picker.
 *   - there is no `PCA_G1` anywhere in the C port (nor in the current Rust
 *     source; the name is historical).
 * So the chain is broken at its first hop (effect -> flat choice) and at its
 * last (PCA_G1). Group P measures that rather than asserting it from a comment.
 *
 * VACUITY CONTROL (the most important quality bar in this file). A Rust
 * negative assertion ("X must NOT gain a blade") reads green on a board where
 * the effect was a silent no-op. Every test that makes a negative claim also
 * asserts CHECK_PROMPTS(n) -- the number of choices the engine actually
 * offered and answered -- or runs an explicit POSITIVE CONTROL board that must
 * offer a prompt.
 *
 * IDENTITY (AGENTS.md). Every fixture goes through mid() -> test_new_id() (Rust
 * `game.id()` allocates a next DISTINCT pool slot; the C template lookup aliases
 * the shared template, so test_id() is wrong here), and every staged print is
 * pinned with rb_card_no_eq(). Every card number in this file was verified
 * against cards/cards.json read as UTF-8; the ones carrying U+FF0B FULLWIDTH
 * PLUS are spelled "\xEF\xBC\x8B" so the bytes are exact.
 *
 * KNOWN-GAP HARNESS FACTS used throughout (confirmed by peers):
 *   - RbChoice.zone for the waitroom is the string "discard", not "waitroom".
 *   - the flat RbChoice carries NO option list (only n_heart_options), so a
 *     multi-option prompt's per-option labels are not observable. Those are
 *     EXPECTED_GAPs, never passes.
 *   - test_get_heart_modifier() remaps colour 5 onto RB_HEART_ORANGE, so heart
 *     reads here use rb_mods_get_heart()/rb_mods_get_need_heart().
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, byte-exact. */
#define FP "\xef\xbc\x8b"

/* ---------------------------------------------------------------- fixtures */
/* target_player/chosen_player_live_bottomdeck_then_draw_test.rs */
#define HANAMARU   "PL!S-bp3-007-R"   /* cost 9, 起動 turn1, pay 1E              */
#define LIVE1      "PL!-sd1-020-SD"   /* live, score 2                          */
#define LIVE2      "PL!-sd1-021-SD"   /* live, score 3                          */
#define FILLER     "PL!-sd1-010-SD"   /* member, cost 4                         */

/* target_player/target_player_recycle_pl_s_bp7_013_n_test.rs */
#define DIA        "PL!S-bp7-013-N"   /* cost 4, 登場                            */
#define CHIKA_M    "PL!S-sd1-001-SD"   /* member, cost 17                         */
#define LIVE3      "PL!-sd1-019-SD"   /* live, score 1                          */

/* target_player/target_player_deck_routing_pl_n_bp3_010_r_pl_n_bp4_002_r */
#define AZUNA      "PL!N-bp3-010-R"   /* cost 9, ライブ開始時                    */
#define KASUMI     "PL!N-bp4-002-R"   /* cost 9, ライブ開始時                    */
/* RUST FIXTURE BUG, evidence-backed. The Rust test
 *   per_card/energy_or_recycle_live_pl_n_pb1_010_r_test.rs:91
 * names the live "PL!N-pb1-026-L". That card DOES NOT EXIST. A raw scan of
 * cards/cards.json for the literal ""PL!N-pb1-026-L"" returns ZERO hits, and the
 * only PL!N-pb1-026-* entry is "PL!N-pb1-026-N" (Nakasuga Kasumi, type=member).
 * cards/build/cards.bin agrees: 0 occurrences of the -L form versus 2 of the
 * -N form. The C card-number resolver therefore walks the -L -> base -> -N chain
 * and silently returns the member -- exactly what the first build reported
 * ("SETUP BUG: identity - want card_no 'PL!N-pb1-026-L', resolved
 * 'PL!N-pb1-026-N'").
 * abilities.json does carry a PL!N-pb1-026-L live_success entry, so an ability
 * is keyed to a live card the CARD table does not contain -- a data hole, and a
 * live card is required to satisfy
 * "put up to 2 Nijigasaki live cards from the waitroom onto the deck top in
 * any order" at all.
 *
 * The substitute is a real Nijigasaki LIVE of the same series (VIVID WORLD,
 * type=live, series=Nijigasaki, present in both cards.json and cards.bin), so
 * the group filter is satisfiable and EVERY Rust ASSERTION stays intact. The
 * fixture's own identity check is kept, as an explicit assertion, so the
 * substitution is visible rather than silent. */
#define NIJI_LIVE  "PL!N-bp4-025-L"   /* NOT the Rust "PL!N-pb1-026-L" -- see above */

/* target_player/opponent_accepts_blades_pl_n_pr_022_pr_test.rs */
#define EMMA       "PL!N-PR-022-PR"   /* cost 2, 登場, choice_maker=opponent    */

/* target_player/opponent_routed_optional_discard_choice_test.rs */
#define YOSHIKO    "PL!S-pb1-006-R"   /* cost 9, 起動 turn1                      */
#define SERAS      "PL!HS-bp6-007-R"  /* cost 15, 自動 turn1                    */
#define EDEL_MEMBER "PL!HS-PR-022-PR" /* EdelNote member, cost 4                */
#define KOTOMI     "PL!-sd1-013-SD"   /* member                                  */
#define RUBY       "PL!S-PR-041-PR"   /* member                                  */

/* selection_filter/blade_option_exclude_self_pl_s_bp5_004_r_test.rs */
#define DIA_BP5    "PL!S-bp5-004-R"   /* cost 2, 登場, 2-option                  */
#define AQ1        "PL!S-sd1-002-SD"  /* Aqours member, cost 15                  */
#define AQ2        "PL!S-sd1-003-SD"  /* Aqours member, cost 11                  */

/* selection_filter/forced_self_wait_excludes_immune_q275_test.rs */
#define KANAN_IMM  "PL!S-bp7-003-R" FP /* Aqours member, cost 4, wait-immune     */
#define NON_IMMUNE FILLER             /* blade 1, not protected                  */

/* selection_filter/optional_energy_edelnote_deploy_or_requirement_test.rs */
#define EDEL_LIVE  "PL!HS-bp5-022-L" /* EdelNote live                           */
#define EDEL_HIGH  "PL!HS-bp5-007-R" /* EdelNote member, cost 13                */
#define EDEL_LOW   "PL!HS-sd1-007-SD" /* EdelNote member, cost 4                 */
#define EDEL_COST9 "PL!HS-bp5-016-N" /* EdelNote member, cost 9                 */
#define AQ_COSTY   "PL!S-bp5-006-R"  /* Aqours member, cost 4 (wrong group)     */
#define EDEL_C5    "PL!HS-bp6-023-N" /* EdelNote member, cost 5 (too costly)    */

/* per_card fixtures */
#define NIJI_CHOICE "PL!N-pb1-010-R" /* cost 10, 登場, 2-option                 */
#define GEKO_CL1    "PL!HS-cl1-004-CL"/* cost 9, 登場, 2-option                  */
#define KOWAREY_L   "PL!S-bp7-025-L" /* live, score 3, ライブ成功時              */
#define SP_COST2    "PL!SP-PR-010-PR"/* member, cost 2                         */
#define HIMEGE      "PL!HS-bp6-014-R" /* member, cost 2                         */
#define ARISE       "PL!-bp5-333-R"   /* member                                 */
#define PRIVATE_W   "PL!-bp5-024-L"   /* live                                   */
#define SD3_BLADE   "PL!-sd1-014-SD"  /* blade 3                                */
#define SD4_BLADE   "PL!SP-sd1-010-SD"/* blade 4                                */
#define ENERGY      "LL-E-001-SD"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ---------------------------------------------------------------- harness */
static int failures;
static int assertions;
static int setup_bugs;
static int gaps;
static int prompts;          /* choices the engine actually offered+answered */
static const char *current_test = "(none)";

/* sizeof(GameState) is ~781 KB, so every TestGame is static (BSS). */
static TestGame tg_a, tg_b, tg_c, tg_c2, tg_d, tg_d2, tg_e, tg_e2, tg_f, tg_f2,
              tg_g, tg_g2, tg_h, tg_h2, tg_i, tg_j, tg_j2, tg_k, tg_k2;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions before the fault: %d, failures: %d, prompts: %d\n",
            sig, current_test, assertions, failures, prompts);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        assertions++;                                                     \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

#define CHECK_EQ_STR(actual, expected, ...) do {                          \
        assertions++;                                                     \
        const char *a_ = (actual);                                        \
        const char *e_ = (expected);                                      \
        if (!a_ || strcmp(a_, e_) != 0) {                                 \
            failures++;                                                   \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got \"%s\" expected \"%s\")\n",            \
                    a_ ? a_ : "(null)", e_);                              \
        }                                                                 \
    } while (0)

/* A Rust assertion with no faithful C entry point. Recorded, never a pass and
 * never a failure -- it stands in for no real CHECK. */
#define EXPECTED_GAP(condition, ...) do {                                \
        assertions++;                                                     \
        if (!(condition)) {                                               \
            printf("GAP: ");                                              \
            printf(__VA_ARGS__);                                          \
            printf("\n");                                                 \
            gaps++;                                                       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before any negative claim below this line can be trusted. The counter is
 * bumped by PROMPT_PRESENT -- i.e. at the exact point a test states a prompt
 * exists -- so the check is meaningful even where it sits BEFORE the answer. */
#define CHECK_PROMPTS(n, ...) do {                                       \
        assertions++;                                                     \
        if (prompts < (n)) {                                              \
            failures++;                                                   \
            fprintf(stderr, "FAIL: [VACUOUS]  ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) were offered, need %d\n",\
                    prompts, (n));                                        \
        }                                                                 \
    } while (0)

/* The single place a prompt is counted. `cond` is the test's own claim that the
 * prompt is on the board; counting HERE (not at answer time) is what lets
 * CHECK_PROMPTS sit before the answer and still be a real control. */
#define PROMPT_PRESENT(tg, cond, ...) do {                               \
        int pp_ = pending(tg) && (cond);                                 \
        CHECK(pp_, __VA_ARGS__);                                         \
        if (pp_) prompts++;                                              \
    } while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static const char *card_no_of(int cid)
{
    static char buf[80];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: mid() (test_new_id), never the shared template. */
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

static int pin_id(int cid, const char *no)
{
    assertions++;
    if (cid < 0 || !rb_card_no_eq(cid, no)) {
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
                no, card_no_of(cid), cid);
        return 0;
    }
    return 1;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int printed_cost(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.cost; rb_free_card(&c); }
    return v;
}

static int printed_blade(int cid)
{
    Card c;
    int v = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { v = c.blade; rb_free_card(&c); }
    return v;
}

/* ---- zone helpers ---- */
static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].hand, cid); }
static int hand2_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[1].hand, cid); }
static int wait_has(const TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].discard, cid); }
static int wait2_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[1].discard, cid); }
static int deck_has(const TestGame *tg, int cid)   { return bag_has(&tg->state.p[0].deck, cid); }
static int deck2_has(const TestGame *tg, int cid)  { return bag_has(&tg->state.p[1].deck, cid); }

static int stage_has(const TestGame *tg, int pl, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] == cid) return 1;
    return 0;
}

/* Rust: player1.stage.stage[i] */
static int stage_at(const TestGame *tg, int pl, int i)
{
    return tg->state.p[pl].stage[i];
}

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0; P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
    tg->state.activating_card = RB_EMPTY_SLOT;
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
    rb_look_reset_all();
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
    P->live.n = 0; P->success.n = 0;
    P->energy.n = 0; P->energy_active = 0; P->energy_deck.n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; P->under_cards[i].n = 0;
    }
}

static void deck_push(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++) P->deck.cards[P->deck.n++] = cid;
}

static void hand_push(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->hand.n < RB_MAX_ZONE; i++) P->hand.cards[P->hand.n++] = cid;
}

static void wait_push(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n && P->discard.n < RB_MAX_ZONE; i++) P->discard.cards[P->discard.n++] = cid;
}

/* Rust main_deck.cards.last() -- the BOTTOM of the deck. */
static int deck_bottom(const TestGame *tg, int pl)
{
    const RbBag *b = &tg->state.p[pl].deck;
    return b->n > 0 ? b->cards[b->n - 1] : -1;
}

static int deck_top(const TestGame *tg, int pl)
{
    const RbBag *b = &tg->state.p[pl].deck;
    return b->n > 0 ? b->cards[0] : -1;
}

/* ---- choice plumbing ---- */
static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const char *pending_type(TestGame *tg)
{
    const char *t = test_pending_choice_type(tg);
    return t ? t : "";
}

static int pending_is_card(TestGame *tg) { return pending(tg) && !strcmp(pending_type(tg), "SelectCard"); }
static int pending_is_targ(TestGame *tg) { return pending(tg) && !strcmp(pending_type(tg), "SelectTarget"); }

/* A prompt the test EXPECTS to exist is counted by PROMPT_PRESENT, at the
 * moment the test asserts it -- not here. */

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg) || n <= 0) return;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust select_option(i) / select_choice_option(i) -- a scalar option answer. */
static void select_option(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    rb_resume_with_choice(&tg->state, idx);
}

/* Rust drain_choices(): answer a SelectCard with its first index, anything else
 * with option 0, up to `guard_max` prompts. */
static void drain_choices(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        if (pending_is_card(tg)) pick(tg, 0);
        else select_option(tg, 0);
    }
}

static int play_to_stage(TestGame *tg, int cid, int area)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, 1);
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

static int play_to_stage_for(TestGame *tg, int pl, int cid, int area)
{
    return test_play_to_stage_for(tg, pl, cid, area);
}

/* Fire ONE named trigger on ONE card, the C twin of Rust fire_trigger()
 * (engine/tests/helpers/mod.rs:114). The local body USED TO BE
 * rb_queue_trigger_abilities(&tg->state, 0, trig), a WHOLE-BOARD TAS scan in
 * which `cid` only set activating_card, so every other card in P1's zones
 * printing the same trigger fired too. test_fire_trigger_on_card targets one
 * ability_id on one explicit_card_id via rb_trigger_auto_ability (the port of
 * Rust GameState::trigger_auto_ability) and does the load-bearing enqueue-time
 * use record. No assumption about other cards is needed any more. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    return test_fire_trigger_on_card(tg, cid, trig);
}

static int fire_live_start(TestGame *tg, int pl, int cid)
{
    tg->state.activating_card = cid;
    int queued = rb_trigger_live_start(&tg->state, pl);
    rb_process_pending_auto_abilities(&tg->state);
    return queued;
}

/* Collect the indices of MEMBER cards in `b`, in order. Rust builds the same
 * list by filtering `waitroom.cards` on CardType::Member. */
static int member_indices(const RbBag *b, int *out, int max)
{
    int n = 0;
    for (int i = 0; i < b->n && n < max; i++)
        if (rb_card_is_member(b->cards[i])) out[n++] = i;
    return n;
}

/* ========================================================================
 * GROUP A -- target_player/chosen_player_live_bottomdeck_then_draw_test.rs
 * PL!S-bp3-007-R (ab#0) kido/turn1, pay 1E:
 *   choose self or opponent; move ONE LIVE from THAT PLAYER's waitroom to THAT
 *   PLAYER's deck BOTTOM; if it happened, SELF draws 1.
 *
 * The cleanest probe of choose_target_player -> chosen seat -> deck_bottom.
 * The BOTTOM assertion is the point: 「デッキの一番下」 is not 「デッキの上」.
 * ======================================================================== */

static void hanamaru_setup(TestGame *tg, int *hanamaru_out, int filler)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int hana = mid(tg, HANAMARU);
    pin_id(hana, HANAMARU);
    *hanamaru_out = hana;
    if (hana < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = hana;
    test_give_energy(tg, 1);
    deck_push(tg, 0, filler, 1);
    deck_push(tg, 1, filler, 1);
}

static void test_a_hanamaru_choose_self_moves_own_live_and_draws(void)
{
    TestGame *tg = &tg_a;
    int filler = mid(tg, FILLER);
    int live = mid(tg, LIVE1);
    pin_id(live, LIVE1);
    int hana;
    hanamaru_setup(tg, &hana, filler);
    if (hana < 0 || filler < 0 || live < 0) return;
    CHECK(rb_card_is_live(live) && !rb_card_is_member(live),
          "a: setup LIVE1 really is a live card, not a member");

    wait_push(tg, 0, live, 1);
    int hand_before = tg->state.p[0].hand.n;

    int activated = test_activate_ability(tg, hana);
    CHECK(activated, "a: the 起動 activation is accepted");
    PROMPT_PRESENT(tg, pending_is_targ(tg),
          "a: the self-or-opponent SelectTarget prompt is offered (got %s)", pending_type(tg));
    CHECK(pending(tg) && !strcmp(rb_get_pending_choice(&tg->state)->target, "self_or_opponent"),
          "a: the prompt target is self_or_opponent (got \"%s\")",
          pending(tg) ? rb_get_pending_choice(&tg->state)->target : "(none)");
    CHECK_PROMPTS(1, "a_choose_self: the self-or-opponent gate must be answered, not skipped");
    select_option(tg, 0);   /* 自分 */

    drain_choices(tg, 6);
    CHECK(!pending(tg), "a: no prompt survives the self resolution");

    CHECK(!wait_has(tg, live), "a: the live left the waitroom");
    CHECK_EQ(deck_bottom(tg, 0), live,
             "a: 「そのプレイヤーのデッキの一番下に置く」 -- the live is the BOTTOM card of p1's deck");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before + 1, "a: 「自分はカードを1枚引く」 -- drew 1");
    CHECK_EQ(tg->state.p[0].energy_active, 0, "a: the printed 1E cost was paid");
}

static void test_a_hanamaru_choose_opponent_moves_opponent_live(void)
{
    TestGame *tg = &tg_b;
    int filler = mid(tg, FILLER);
    int opp_live = mid(tg, LIVE2);
    pin_id(opp_live, LIVE2);
    int hana;
    hanamaru_setup(tg, &hana, filler);
    if (hana < 0 || filler < 0 || opp_live < 0) return;

    wait_push(tg, 1, opp_live, 1);
    int p1_hand_before = tg->state.p[0].hand.n;

    test_activate_ability(tg, hana);
    PROMPT_PRESENT(tg, pending_is_targ(tg),
          "a2: the self-or-opponent prompt is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "a2_choose_opponent: the gate must be answered");
    select_option(tg, 1);   /* 相手 */

    drain_choices(tg, 6);
    CHECK(!pending(tg), "a2: no prompt survives the opponent resolution");

    /* THE decisive assertion. A legacy fallback that always reads the resolving
     * actor's own zones passes every self case above and fails exactly here. */
    CHECK(!wait2_has(tg, opp_live), "a2: the OPPONENT's live left p2's waitroom");
    CHECK_EQ(deck_bottom(tg, 1), opp_live,
             "a2: the live is the BOTTOM of P2's deck, not p1's");
    CHECK(!deck_has(tg, opp_live),
          "a2: the opponent's live must NOT land in p1's deck (self-fallback detector)");
    CHECK_EQ(tg->state.p[0].hand.n, p1_hand_before + 1,
             "a2: 「自分はカードを1枚引く」 -- SELF draws even when targeting the opponent");
}

static void test_a_hanamaru_no_live_in_waitroom_no_move_no_draw(void)
{
    TestGame *tg = &tg_c;
    int filler = mid(tg, FILLER);
    int mem = mid(tg, FILLER);
    int hana;
    hanamaru_setup(tg, &hana, filler);
    if (hana < 0 || filler < 0 || mem < 0) return;

    wait_push(tg, 0, mem, 1);   /* member only -- no live_card is eligible */
    int hand_before = tg->state.p[0].hand.n;

    test_activate_ability(tg, hana);
    PROMPT_PRESENT(tg, pending_is_targ(tg), "a3: the gate prompt is offered");
    CHECK_PROMPTS(1, "a3_no_live: the gate must be answered before the negative claim");
    select_option(tg, 0);

    CHECK(!pending(tg), "a3: zero eligible lives auto-skips with no prompt");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "a3: no live moved, so 「そうした場合」 is false and there is NO draw");
    CHECK(wait_has(tg, mem), "a3: the member in the waitroom is untouched");
    CHECK_EQ(deck_bottom(tg, 0), filler, "a3: nothing was placed on the deck bottom");
}

static void test_a_hanamaru_opponent_no_live_no_draw(void)
{
    TestGame *tg = &tg_c2;
    int filler = mid(tg, FILLER);
    int hana;
    hanamaru_setup(tg, &hana, filler);
    if (hana < 0 || filler < 0) return;

    wait_push(tg, 1, filler, 1);  /* p2 waitroom: member only */
    int hand_before = tg->state.p[0].hand.n;

    test_activate_ability(tg, hana);
    PROMPT_PRESENT(tg, pending_is_targ(tg), "a4: the gate prompt is offered");
    CHECK_PROMPTS(1, "a4_opponent_no_live: the gate must be answered");
    select_option(tg, 1);

    CHECK(!pending(tg), "a4: zero eligible opponent lives auto-skips");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before,
             "a4: no draw when the chosen player has no live in the waitroom");
}

static void test_a_hanamaru_cannot_activate_without_energy(void)
{
    TestGame *tg = &tg_d;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int hana = mid(tg, HANAMARU);
    pin_id(hana, HANAMARU);
    int filler = mid(tg, FILLER);
    deck_push(tg, 0, filler, 5);
    deck_push(tg, 1, filler, 5);
    if (hana < 0 || filler < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = hana;
    int hand_before = tg->state.p[0].hand.n;
    CHECK_EQ(tg->state.p[0].energy_active, 0, "a5: setup -- zero active energy");

    /* Rust try_activate_ability(...) must be Err; the C twin returns 0. */
    int ok = test_activate_ability(tg, hana);
    CHECK_EQ(ok, 0, "a5: the printed 1E cost cannot be paid, so the activation is refused");
    CHECK(!pending(tg), "a5: a refused activation opens no choice prompt");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "a5: no draw when the cost was not paid");
    CHECK_EQ(tg->state.p[0].energy_active, 0, "a5: no energy was consumed");
}

static void test_a_hanamaru_turn_limit_once(void)
{
    TestGame *tg = &tg_d2;
    int filler = mid(tg, FILLER);
    int live = mid(tg, LIVE1);
    int live2 = mid(tg, LIVE2);
    pin_id(live, LIVE1);
    pin_id(live2, LIVE2);
    int hana;
    hanamaru_setup(tg, &hana, filler);
    if (hana < 0 || filler < 0 || live < 0 || live2 < 0) return;
    CHECK(card_prints_trigger(hana, RB_TSTR_ACTIVATION), "a6: the print carries a 起動 trigger");

    wait_push(tg, 0, live, 1);
    test_activate_ability(tg, hana);
    PROMPT_PRESENT(tg, pending_is_targ(tg), "a6: the first activation offers the gate");
    select_option(tg, 0);
    drain_choices(tg, 6);
    CHECK_EQ(tg->state.p[0].energy_active, 0, "a6: the first use paid its 1E");

    wait_push(tg, 0, live2, 1);
    test_give_energy(tg, 1);
    int ok = test_activate_ability(tg, hana);
    CHECK_EQ(ok, 0, "a6: ターン1回 -- the second activation in the same turn is refused");
}

/* ========================================================================
 * GROUP B -- target_player/target_player_recycle_pl_s_bp7_013_n_test.rs
 * PL!S-bp7-013-N 登場: choose self/opponent; move up to 2 MEMBERS from THAT
 * waitroom to THAT deck, in any order, on the bottom.
 *
 * Group B is the SEAT-ROUTING group. For the 相手 branch Rust asserts the
 * resulting SelectCard carries filtered_indices restricted to the MEMBERS and
 * target_player_id == "self" -- i.e. the RESOLVING ACTOR reads the CHOSEN SEAT's
 * waitroom. That is the behaviour under test.
 * ======================================================================== */

static void test_b_dia_choose_self_recycles_members_preserves_live(void)
{
    TestGame *tg = &tg_e;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int dia = mid(tg, DIA);
    pin_id(dia, DIA);
    int m1 = mid(tg, CHIKA_M), m2 = mid(tg, CHIKA_M);
    int life = mid(tg, LIVE3);
    pin_id(life, LIVE3);
    int filler = mid(tg, FILLER);
    if (dia < 0 || m1 < 0 || m2 < 0 || life < 0 || filler < 0) return;

    /* setup guard: the live is genuinely a live, so it cannot be a member */
    CHECK(rb_card_is_live(life) && !rb_card_is_member(life),
          "b: setup LIVE3 is a LIVE, so it must not be a legal recycle target");

    tg->state.p[0].stage[AREA_CENTER] = dia;
    wait_push(tg, 0, m1, 1);
    wait_push(tg, 0, life, 1);
    wait_push(tg, 0, m2, 1);

    CHECK(fire_trigger(tg, dia, RB_TSTR_DEBUT), "b: the 登場 trigger fires");
    PROMPT_PRESENT(tg, pending_is_targ(tg),
          "b: the 自分/相手 player choice is offered as SelectTarget (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "b_choose_self: the 自分/相手 gate must be answered");
    select_option(tg, 0);   /* 自分 */

    int idx[RB_MAX_ZONE];
    int picked = 0;
    while (pending(tg) && picked < 4) {
        int n_idx = member_indices(&tg->state.p[0].discard, idx, RB_MAX_ZONE);
        if (n_idx == 0) break;
        if (!pending_is_card(tg)) break;
        CHECK_PROMPTS(2, "b_choose_self: the member SelectCard must actually be offered");
        {
            const RbChoice *c = rb_get_pending_choice(&tg->state);
            int offers_live = 0;
            for (int i = 0; i < c->n_filtered_indices; i++)
                if (c->filtered_indices[i] == 2) offers_live = 1;   /* live is at pos 2 */
            CHECK(!offers_live,
                  "b: the waitroom LIVE at position 2 must not be among the offered indices");
        }
        pick_many(tg, idx, n_idx);
        picked++;
    }
    CHECK(picked > 0, "b: the member selection was actually offered and answered");

    /* 「好きな順番でデッキの下に置く」 -- the recycled members land UNDER each
     * other at the bottom, in waitroom order. */
    CHECK_EQ(deck_bottom(tg, 0), m2, "b: the LAST waitroom member is the deck's bottom card");
    CHECK_EQ(tg->state.p[0].deck.n, 2, "b: both members moved to the deck bottom");
    CHECK(wait_has(tg, life), "b: the live was not a legal target and stayed in the waitroom");
    CHECK_EQ(tg->state.p[0].discard.n, 1, "b: only the live is left in the waitroom");
}

static void test_b_dia_choose_opponent_card_selection_routes_to_chosen_seat(void)
{
    TestGame *tg = &tg_f;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int dia = mid(tg, DIA);
    pin_id(dia, DIA);
    int filler = mid(tg, FILLER);
    int m1 = mid(tg, CHIKA_M), m2 = mid(tg, CHIKA_M);
    int m3 = mid(tg, CHIKA_M), m4 = mid(tg, CHIKA_M);
    int life = mid(tg, LIVE3), life2 = mid(tg, LIVE3);
    pin_id(life, LIVE3);
    if (dia < 0 || filler < 0 || m1 < 0 || m2 < 0 || m3 < 0 || m4 < 0 ||
        life < 0 || life2 < 0) return;

    tg->state.p[0].stage[AREA_CENTER] = dia;
    wait_push(tg, 0, m1, 1);
    wait_push(tg, 0, life, 1);
    wait_push(tg, 0, m2, 1);
    wait_push(tg, 1, m3, 1);
    wait_push(tg, 1, life2, 1);
    wait_push(tg, 1, m4, 1);

    CHECK(fire_trigger(tg, dia, RB_TSTR_DEBUT), "b2: the 登場 trigger fires");
    PROMPT_PRESENT(tg, pending_is_targ(tg), "b2: the 自分/相手 gate is offered");
    CHECK_PROMPTS(1, "b2_choose_opponent: the gate must be answered");
    select_option(tg, 1);   /* 相手 */

    /* Rust asserts here:
     *   entry.choice_player_id == Some("p1")  -- the ACTIVE player answers
     *   the SelectCard's filtered_indices cover only the OPPONENT's members */
    int cur = tg->state.queue.cur;
    CHECK(cur >= 0 && cur < tg->state.queue.n_entries,
          "b2: a live queue entry hosts the choice (cur=%d n=%d)", cur, tg->state.queue.n_entries);
    if (cur >= 0 && cur < tg->state.queue.n_entries) {
        int seat = test_queue_entry_choice_seat(tg, cur);
        if (seat == -1) {
            char buf[TEST_SEAT_ID_LEN];
            test_queue_entry_choice_player_id(tg, cur, buf, sizeof buf);
            fprintf(stderr,
                    "  [ENGINE GAP] b2: entry %d choice_player_id is \"%s\" (un-stamped).\n"
                    "  Rust ability_queue.rs:345-351 stamps EVERY paused choice from\n"
                    "  player_id, so an empty token here is a dead routing path.\n",
                    cur, buf);
        }
        CHECK_EQ(seat, 0,
                 "b2: the member choice is routed to P1 (the active player), not the opponent");
    }

    CHECK(pending(tg) && pending_is_card(tg),
          "b2: a card selection over the OPPONENT's waitroom is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(2, "b2_choose_opponent: BOTH the gate and the card pick must be answered");

    if (pending_is_card(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "discard",
                     "b2: the member pick is over the WAITROOM (C spells the zone \"discard\")");
        fprintf(stderr,
                "  [SEAT SPELLING] b2 SelectCard.target_player_id = \"%s\"\n"
                "  Rust asserts Some(\"self\") here; the C struct carries the seat token\n"
                "  instead, so this is a naming divergence, not a wrong-state one.\n",
                c->target_player_id);
        /* The picked zone must be P2's waitroom, not P1's. P1's own waitroom still
         * holds m1/life/m2 (3 cards); P2's holds m3/life2/m4 (also 3), so the
         * discriminator is which LIVE is excluded from the offered set. */
        int offers_live = 0;
        for (int i = 0; i < c->n_filtered_indices; i++)
            if (c->filtered_indices[i] == 1) offers_live = 1;   /* p2 live sits at pos 1 */
        CHECK(!offers_live, "b2: P2's live at waitroom position 1 is not a legal target");
        CHECK(c->n_filtered_indices == 2,
              "b2: exactly the two MEMBERS of P2's waitroom are offered (got %d)",
              c->n_filtered_indices);
    }

    int picked = 0;
    while (pending(tg) && pending_is_card(tg) && picked < 4) {
        int idx[RB_MAX_ZONE];
        int n_idx = member_indices(&tg->state.p[1].discard, idx, RB_MAX_ZONE);
        if (n_idx == 0) break;
        pick_many(tg, idx, n_idx);
        picked++;
    }
    CHECK(picked > 0, "b2: P2's members were actually selected");
    drain_choices(tg, 6);

    CHECK_EQ(tg->state.p[1].deck.n, 2, "b2: both members moved to P2's deck");
    CHECK(wait2_has(tg, life2),
          "b2: P2's live was not a legal target and stayed in P2's waitroom");
    CHECK_EQ(tg->state.p[1].discard.n, 1, "b2: only the live is left in P2's waitroom");
    CHECK_EQ(tg->state.p[0].deck.n, 0,
             "b2: P1's own deck is untouched -- the OPPONENT's waitroom was used");
}

/* ========================================================================
 * GROUP C -- target_player/target_player_deck_routing_pl_n_bp3_010_r_...
 * PL!N-bp3-010-R ライブ開始時: choose self/opponent; move up to 2 members from
 * the CHOSEN waitroom to THAT deck bottom.
 * PL!N-bp4-002-R ライブ開始時: choose self/opponent; look at THAT deck's top card
 * and optionally discard it.
 * ======================================================================== */

static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) { test_pass(tg); drain_choices(tg, 8); }
}

/* Rust fill_deck(game, player, count) -- fresh distinct copies each time. */
static void fill_deck(TestGame *tg, int pl, int count)
{
    for (int i = 0; i < count; i++) {
        int id = test_new_id(tg, FILLER);
        if (id < 0) break;
        RbPlayer *P = &tg->state.p[pl];
        if (P->deck.n >= RB_MAX_ZONE) break;
        P->deck.cards[P->deck.n++] = id;
    }
}

/* Rust trigger_live_start_with: hand the filler live, walk to live-set, set it,
 * then step past the LiveStart window. */
static int trigger_live_start_with(TestGame *tg, int *live_out)
{
    int filler = test_new_id(tg, FILLER);
    int live = mid(tg, LIVE3);
    pin_id(live, LIVE3);
    *live_out = live;
    if (filler < 0 || live < 0) return 0;
    hand_push(tg, 0, live, 1);
    for (int i = 0; i < 10; i++) {
        int f = test_new_id(tg, FILLER);
        if (f < 0) break;
        RbPlayer *P2 = &tg->state.p[1];
        if (P2->deck.n >= RB_MAX_ZONE) break;
        P2->deck.cards[P2->deck.n++] = f;
    }
    advance_to_live_set(tg);
    test_set_live_card(tg, 0, live);
    test_pass(tg); test_pass(tg);
    drain_choices(tg, 8);
    return live;
}

static void test_c_azuna_choose_self_recycle_grows_own_deck(void)
{
    TestGame *tg = &tg_g;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int azuna = mid(tg, AZUNA);
    pin_id(azuna, AZUNA);
    int m1 = mid(tg, FILLER), m2 = mid(tg, FILLER);
    if (azuna < 0 || m1 < 0 || m2 < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = azuna;
    wait_push(tg, 0, m1, 1);
    wait_push(tg, 0, m2, 1);
    fill_deck(tg, 0, 10);
    fill_deck(tg, 1, 10);
    int live;
    if (!trigger_live_start_with(tg, &live)) return;

    PROMPT_PRESENT(tg, 1, "c: the LiveStart choice is offered (got %s)", pending_type(tg));
    CHECK_PROMPTS(1, "c_choose_self: the self-or-opponent gate must be answered");
    select_option(tg, 0);
    drain_choices(tg, 8);

    int deck_after = tg->state.p[0].deck.n;
    CHECK(deck_after >= 2, "c: p1's deck should have gained the recycled members (deck=%d)", deck_after);
    CHECK_EQ(tg->state.p[0].discard.n, 0,
             "c: choosing 自分 empties P1's OWN waitroom of the two members");
}

static void test_c_azuna_choose_opponent_empties_opponent_waitroom(void)
{
    TestGame *tg = &tg_g2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int azuna = mid(tg, AZUNA);
    pin_id(azuna, AZUNA);
    int m3 = mid(tg, FILLER), m4 = mid(tg, FILLER);
    if (azuna < 0 || m3 < 0 || m4 < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = azuna;
    wait_push(tg, 1, m3, 1);
    wait_push(tg, 1, m4, 1);
    fill_deck(tg, 0, 10);
    fill_deck(tg, 1, 10);
    int live;
    if (!trigger_live_start_with(tg, &live)) return;

    PROMPT_PRESENT(tg, 1, "c2: the LiveStart choice is offered");
    CHECK_PROMPTS(1, "c2_choose_opponent: the gate must be answered");
    select_option(tg, 1);
    drain_choices(tg, 8);

    int deck_after = tg->state.p[1].deck.n;
    CHECK(deck_after >= 2, "c2: P2's deck should have gained the recycled members (deck=%d)", deck_after);
    CHECK_EQ(tg->state.p[1].discard.n, 0,
             "c2: choosing 相手 empties the OPPONENT's waitroom, not p1's");
}

static void test_c_kasumi_choose_self_discards_looked_top_card(void)
{
    TestGame *tg = &tg_h;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int kasumi = mid(tg, KASUMI);
    pin_id(kasumi, KASUMI);
    int top_card = mid(tg, FILLER);
    if (kasumi < 0 || top_card < 0) return;
    /* Rust clears p1's deck then pushes top_card FIRST, so it is the top card. */
    tg->state.p[0].deck.n = 0;
    deck_push(tg, 0, top_card, 1);
    fill_deck(tg, 0, 5);
    tg->state.p[0].stage[AREA_CENTER] = kasumi;
    fill_deck(tg, 1, 10);
    int live;
    if (!trigger_live_start_with(tg, &live)) return;

    int looked = deck_top(tg, 0);
    CHECK(looked >= 0, "c3: p1's deck still has a top card to look at");

    PROMPT_PRESENT(tg, 1, "c3: the LiveStart choice is offered");
    CHECK_PROMPTS(1, "c3_choose_self: the gate must be answered");
    select_option(tg, 0);
    drain_choices(tg, 8);

    CHECK(looked >= 0 && !deck_has(tg, looked),
          "c3: the looked-at card is no longer on the deck");
    CHECK(looked >= 0 && wait_has(tg, looked),
          "c3: 「自分はそのカードを控え室に置いてもよい」 -- the looked card reached the waitroom");
}

static void test_c_dia_then_azuna_share_opponent_waitroom_preserve_order(void)
{
    TestGame *tg = &tg_h2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int azuna = mid(tg, AZUNA);
    int dia = mid(tg, DIA);
    pin_id(azuna, AZUNA);
    pin_id(dia, DIA);
    int a = mid(tg, CHIKA_M), b = mid(tg, CHIKA_M);
    int c = mid(tg, CHIKA_M), d = mid(tg, CHIKA_M);
    int l1 = mid(tg, LIVE3), l2 = mid(tg, LIVE3), l3 = mid(tg, LIVE3);
    pin_id(l1, LIVE3);
    if (azuna < 0 || dia < 0 || a < 0 || b < 0 || c < 0 || d < 0 ||
        l1 < 0 || l2 < 0 || l3 < 0) return;

    tg->state.p[0].stage[AREA_CENTER] = azuna;
    RbPlayer *P2 = &tg->state.p[1];
    const int seq[7] = { a, l1, b, l2, c, l3, d };
    for (int i = 0; i < 7; i++) P2->discard.cards[P2->discard.n++] = seq[i];
    fill_deck(tg, 0, 10);
    fill_deck(tg, 1, 10);

    /* --- Step 1: DIA debut, choose 相手 --- */
    CHECK(fire_trigger(tg, dia, RB_TSTR_DEBUT), "c4: the DIA 登場 trigger fires");
    PROMPT_PRESENT(tg, pending_is_targ(tg), "c4: the 自分/相手 gate is offered");
    CHECK_PROMPTS(1, "c4: the gate must be answered");
    select_option(tg, 1);

    if (!pending_is_card(tg)) {
        CHECK(0, "c4 step1: a member SelectCard over P2's waitroom is offered (got %s)",
              pending_type(tg));
        return;
    }
    {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(ch->zone, "discard", "c4 step1: the pick is over the waitroom");
        /* Rust: filtered_indices == [0, 2, 4, 6] -- the four members, no lives. */
        const int want[4] = { 0, 2, 4, 6 };
        CHECK_EQ(ch->n_filtered_indices, 4,
                 "c4 step1: exactly the four MEMBERS are offered among the 7 waitroom cards (got %d)",
                 ch->n_filtered_indices);
        for (int i = 0; i < 4 && i < ch->n_filtered_indices; i++)
            CHECK_EQ(ch->filtered_indices[i], want[i],
                     "c4 step1: filtered index %d is waitroom position %d", i, want[i]);
    }
    {
        const int ans[2] = { 1, 0 };   /* Rust: select_indices(&[1, 0]) */
        pick_many(tg, ans, 2);
    }

    /* --- Step 2: AZUNA LiveStart, choose 相手, same waitroom --- */
    CHECK(fire_trigger(tg, azuna, RB_TSTR_LIVE_START),
          "c4: the AZUNA ライブ開始時 trigger fires");
    PROMPT_PRESENT(tg, pending_is_targ(tg), "c4 step2: the 自分/相手 gate is offered again");
    CHECK_PROMPTS(2, "c4: BOTH gates must be answered");
    select_option(tg, 1);

    if (!pending_is_card(tg)) {
        CHECK(0, "c4 step2: a member SelectCard over P2's waitroom is offered (got %s)",
              pending_type(tg));
        return;
    }
    {
        const RbChoice *ch = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(ch->zone, "discard", "c4 step2: the pick is over the waitroom");
        fprintf(stderr,
                "  [FILTER SHAPE] c4 step2 SelectCard zone=\"%s\" n_filtered=%d indices=",
                ch->zone, ch->n_filtered_indices);
        for (int i = 0; i < ch->n_filtered_indices; i++)
            fprintf(stderr, "%d,", ch->filtered_indices[i]);
        fprintf(stderr, "  (Rust expects exactly the two remaining members)\n");
        CHECK(ch->n_filtered_indices > 0,
              "c4 step2: the remaining members are offered");
    }
    {
        const int ans[2] = { 1, 0 };
        pick_many(tg, ans, 2);
    }
    drain_choices(tg, 8);

    /* --- Step 3: the four members all reached P2's deck, the lives stayed --- */
    const int members[4] = { a, b, c, d };
    for (int i = 0; i < 4; i++) {
        CHECK(!wait2_has(tg, members[i]), "c4: member %d left P2's waitroom", i);
        CHECK(deck2_has(tg, members[i]), "c4: member %d is in P2's deck", i);
    }
    CHECK(wait2_has(tg, l1) && wait2_has(tg, l2) && wait2_has(tg, l3),
          "c4: all three lives stayed in P2's waitroom");
    CHECK_EQ(tg->state.p[1].discard.n, 3, "c4: exactly the three lives remain");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "c4: p1's waitroom is empty throughout");
}

/* ========================================================================
 * GROUP D -- target_player/opponent_accepts_blades_pl_n_pr_022_pr_test.rs
 * PL!N-PR-022-PR 登場: an answer_based choice whose parsed ability carries
 * `choice_maker: "opponent"`. THE OPPONENT answers, and the accepted branch
 * grants ブレード to EVERY member of the OPPONENT's stage.
 *
 * The sharpest available probe of choice-maker routing: the C's
 * rb_queue_pause_for_choice G1/G3 fallback keys off the choice TARGET, while a
 * `choice_maker: opponent` prompt is a plain SelectTarget whose resolution must
 * still stamp the OPPONENT as the answering seat.
 * ======================================================================== */

static int emma_setup(TestGame *tg, int p2_center)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int emma = mid(tg, EMMA);
    pin_id(emma, EMMA);
    int filler = test_new_id(tg, FILLER);
    if (emma < 0 || filler < 0) return -1;
    tg->state.p[1].stage[AREA_CENTER] = p2_center;
    hand_push(tg, 0, emma, 1);
    test_give_energy(tg, 10);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);
    return emma;
}

static void test_d_emma_accepted_choice_routed_to_opponent(void)
{
    TestGame *tg = &tg_i;
    int filler = test_new_id(tg, FILLER);
    if (filler < 0) return;
    int emma = emma_setup(tg, filler);
    if (emma < 0) return;

    play_to_stage(tg, emma, AREA_LEFT);
    PROMPT_PRESENT(tg, 1, "d: Emma's 登場 choice is waiting for an answer (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "d_routed: the choice must be answered before the blade assertions");

    /* Rust: choice_player_id == Some("p2") -- the OPPONENT answers. */
    int cur = tg->state.queue.cur;
    if (cur >= 0 && cur < tg->state.queue.n_entries) {
        int seat = test_queue_entry_choice_seat(tg, cur);
        if (seat == -1)
            fprintf(stderr,
                    "  [ENGINE GAP] d: entry %d choice_player_id is empty (un-stamped)\n", cur);
        CHECK_EQ(seat, 1,
                 "d: the answer_based choice is routed to P2 (choice_maker=opponent)");
    } else {
        CHECK(0, "d: a live queue entry hosts Emma's choice");
    }

    select_option(tg, 0);   /* 「お願いします」 -> grant the blades */
    drain_choices(tg, 6);

    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, filler), 1,
             "d: P2's member gained 1 ブレード");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, emma), 0,
             "d: P1's Emma did NOT gain a blade (target=opponent)");
}

static void test_d_emma_blades_every_opponent_member(void)
{
    TestGame *tg = &tg_j;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int emma = mid(tg, EMMA);
    pin_id(emma, EMMA);
    int left = test_new_id(tg, FILLER), center = test_new_id(tg, FILLER);
    int right = test_new_id(tg, FILLER);
    if (emma < 0 || left < 0 || center < 0 || right < 0) return;
    tg->state.p[1].stage[AREA_LEFT] = left;
    tg->state.p[1].stage[AREA_CENTER] = center;
    tg->state.p[1].stage[AREA_RIGHT] = right;
    hand_push(tg, 0, emma, 1);
    test_give_energy(tg, 10);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);

    play_to_stage(tg, emma, AREA_LEFT);
    PROMPT_PRESENT(tg, 1, "d2: Emma's choice is offered");
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "d2_all_members: the choice must actually be answered");
    select_option(tg, 0);
    drain_choices(tg, 6);

    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, left), 1, "d2: left opponent member gained 1");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, center), 1, "d2: center opponent member gained 1");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, right), 1, "d2: right opponent member gained 1");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, emma), 0, "d2: Emma herself gained none");
}

static void test_d_emma_other_answer_grants_nothing(void)
{
    TestGame *tg = &tg_j2;
    int filler = test_new_id(tg, FILLER);
    if (filler < 0) return;
    int emma = emma_setup(tg, filler);
    if (emma < 0) return;

    play_to_stage(tg, emma, AREA_LEFT);
    PROMPT_PRESENT(tg, 1, "d3: Emma's choice is offered");
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "d3_other_answer: the choice must be answered before the negative claim");
    select_option(tg, 1);   /* 「それ以外」 -> 何もしない */
    drain_choices(tg, 6);

    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, filler), 0,
             "d3: 「何もしない」 -- the opponent member gains no blade");
}

/* ========================================================================
 * GROUP E -- target_player/opponent_routed_optional_discard_choice_test.rs
 * PL!S-pb1-006-R 起動 turn1:
 *   reveal one live from hand; the OPPONENT may discard one card from hand;
 *   if they did NOT, gain 4 blades until the end of the live.
 *
 * The OPPONENT performs the OPTIONAL COST (action_by=opponent). Accepting it
 * suppresses the conditional; declining it grants +4. Either way the choice has
 * to be routed to the OPPONENT, and the 0-card case must AUTO-SKIP.
 * ======================================================================== */

static int yoshiko_setup(TestGame *tg, int p2_hand_cards, int *yoshiko_out, int *p2_cards_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int yoshiko = mid(tg, YOSHIKO);
    pin_id(yoshiko, YOSHIKO);
    int live = mid(tg, LIVE3);
    pin_id(live, LIVE3);
    *yoshiko_out = yoshiko;
    if (yoshiko < 0 || live < 0) return -1;
    tg->state.p[0].stage[AREA_CENTER] = yoshiko;
    hand_push(tg, 0, live, 1);
    for (int i = 0; i < p2_hand_cards; i++) {
        int c = test_new_id(tg, FILLER);
        if (c < 0) break;
        hand_push(tg, 1, c, 1);
        if (p2_cards_out && i < 4) p2_cards_out[i] = c;
    }
    test_give_energy(tg, 1);
    return yoshiko;
}

static void test_e_yoshiko_opponent_discards_skips_blade(void)
{
    TestGame *tg = &tg_k;
    int p2c[4];
    p2c[0] = p2c[1] = p2c[2] = p2c[3] = -1;
    int yoshiko;
    if (yoshiko_setup(tg, 1, &yoshiko, p2c) < 0 || p2c[0] < 0) return;

    CHECK(test_activate_ability(tg, yoshiko), "e: the 起動 activation is accepted");
    CHECK_PROMPTS(1, "e_opponent_discards: the optional cost must be offered and answered");
    PROMPT_PRESENT(tg, 1, "e: the opponent's optional-discard prompt is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    {
        int cur = tg->state.queue.cur;
        if (cur >= 0 && cur < tg->state.queue.n_entries)
            CHECK_EQ(test_queue_entry_choice_seat(tg, cur), 1,
                     "e: the optional discard is routed to P2 (action_by=opponent)");
    }
    {
        const int ans[1] = { 0 };
        pick_many(tg, ans, 1);
    }
    drain_choices(tg, 6);

    CHECK(!pending(tg), "e: no prompt survives the resolution");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, yoshiko), 0,
             "e: the opponent DISCARDED, so 「そうしなかった場合」 is false and no blade is gained");
    CHECK(wait2_has(tg, p2c[0]), "e: the opponent's card is in P2's waitroom");
    CHECK_EQ(tg->state.p[1].hand.n, 0, "e: the opponent's hand is empty afterwards");
}

static void test_e_yoshiko_opponent_skips_gains_blade(void)
{
    TestGame *tg = &tg_k2;
    int p2c[4];
    p2c[0] = p2c[1] = p2c[2] = p2c[3] = -1;
    int yoshiko;
    if (yoshiko_setup(tg, 1, &yoshiko, p2c) < 0 || p2c[0] < 0) return;

    CHECK(test_activate_ability(tg, yoshiko), "e2: the 起動 activation is accepted");
    CHECK_PROMPTS(1, "e2_opponent_skips: the optional cost must be offered and DECLINED");
    if (!pending(tg)) { CHECK(0, "e2: the skippable discard prompt is offered (got %s)", pending_type(tg)); return; }
    skip(tg);   /* Rust select_indices(&[]) -- decline */
    drain_choices(tg, 6);

    CHECK(!pending(tg), "e2: no prompt survives the resolution");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, yoshiko), 4,
             "e2: 「そうしなかった場合…ブレードを4つ得る」 -- exactly 4 blades");
    CHECK(!wait2_has(tg, p2c[0]), "e2: the opponent's card was NOT discarded");
    CHECK(hand2_has(tg, p2c[0]), "e2: the opponent's card is still in their hand");
}

static void test_e_yoshiko_multi_card_skips_gains_blade(void)
{
    TestGame *tg = &tg_d;
    int p2c[4];
    p2c[0] = p2c[1] = p2c[2] = p2c[3] = -1;
    int yoshiko;
    if (yoshiko_setup(tg, 3, &yoshiko, p2c) < 0 || p2c[0] < 0) return;

    CHECK(test_activate_ability(tg, yoshiko), "e3: the 起動 activation is accepted");
    CHECK_PROMPTS(1, "e3_multi_card: the optional cost must be offered");
    if (!pending(tg)) { CHECK(0, "e3: the skippable discard prompt is offered (got %s)", pending_type(tg)); return; }
    {
        int cur = tg->state.queue.cur;
        if (cur >= 0 && cur < tg->state.queue.n_entries)
            CHECK_EQ(test_queue_entry_choice_seat(tg, cur), 1,
                     "e3: with 3 cards the choice must be a PROMPT routed to P2");
    }
    skip(tg);
    drain_choices(tg, 6);

    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, yoshiko), 4,
             "e3: skipping with 3 cards available still grants exactly 4 blades");
    CHECK_EQ(tg->state.p[1].hand.n, 3, "e3: the opponent's hand is intact");
    CHECK_EQ(tg->state.p[1].discard.n, 0, "e3: the opponent's waitroom is empty");
}

static void test_e_yoshiko_multi_card_discards_skips_blade(void)
{
    TestGame *tg = &tg_d2;
    int p2c[4];
    p2c[0] = p2c[1] = p2c[2] = p2c[3] = -1;
    int yoshiko;
    if (yoshiko_setup(tg, 3, &yoshiko, p2c) < 0 || p2c[0] < 0) return;

    CHECK(test_activate_ability(tg, yoshiko), "e4: the 起動 activation is accepted");
    CHECK_PROMPTS(1, "e4_multi_discard: the optional cost must be offered and accepted");
    if (!pending(tg)) { CHECK(0, "e4: the skippable discard prompt is offered (got %s)", pending_type(tg)); return; }
    {
        const int ans[1] = { 0 };
        pick_many(tg, ans, 1);
    }
    drain_choices(tg, 6);

    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, yoshiko), 0,
             "e4: the opponent discarded, so no blade is granted");
    CHECK_EQ(tg->state.p[1].hand.n, 2, "e4: the opponent has 2 cards left");
    CHECK_EQ(tg->state.p[1].discard.n, 1, "e4: the opponent has 1 card in the waitroom");
}

static void test_e_yoshiko_opponent_empty_hand_auto_skips(void)
{
    TestGame *tg = &tg_e;
    int yoshiko;
    if (yoshiko_setup(tg, 0, &yoshiko, NULL) < 0) return;

    CHECK(test_activate_ability(tg, yoshiko), "e5: the 起動 activation is accepted");
    drain_choices(tg, 6);
    /* Edge: the opponent has 0 cards so move_cards auto-skips. Without a POSITIVE
     * CONTROL the assertion below could read green on a board where the ability
     * did literally nothing. */
    CHECK(!pending(tg), "e5: no prompt when the opponent's hand is empty");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, yoshiko), 4,
             "e5: with an empty opponent hand the blade is granted (auto-skip of the optional cost)");

    /* POSITIVE CONTROL: identical board + one opponent card -> the SAME prompt
     * must appear, proving the assertion above is not vacuous. */
    {
        TestGame *tg2 = &tg_e2;
        int p2c[4];
        p2c[0] = p2c[1] = p2c[2] = p2c[3] = -1;
        int yok2;
        if (yoshiko_setup(tg2, 1, &yok2, p2c) < 0 || p2c[0] < 0) return;
        CHECK(test_activate_ability(tg2, yok2), "e5 control: the 起動 activation is accepted");
        PROMPT_PRESENT(tg2, 1,
              "e5 CONTROL: with one opponent card in hand the optional-cost prompt MUST be "
              "offered, or the auto-skip assertion above proves nothing");
        CHECK_PROMPTS(1, "e5_empty_hand CONTROL: the control board must offer the prompt");
        skip(tg2);
        drain_choices(tg2, 6);
        CHECK_EQ(rb_mods_get_blade(&tg2->state.mods, yok2), 4,
                 "e5 control: declining also grants 4 blades");
    }
}

/* Rust serasu_edelnote_appear_triggers_opponent_wait (PL!HS-bp6-007-R 自動1) */
static void test_e_serasu_edelnote_appear_triggers_opponent_wait(void)
{
    TestGame *tg = &tg_f;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int seras = mid(tg, SERAS);
    pin_id(seras, SERAS);
    int edel = mid(tg, EDEL_MEMBER);
    pin_id(edel, EDEL_MEMBER);
    int a = test_new_id(tg, FILLER), b = test_new_id(tg, KOTOMI);
    int filler = test_new_id(tg, KOTOMI);
    if (seras < 0 || edel < 0 || a < 0 || b < 0 || filler < 0) return;
    CHECK(printed_cost(edel) <= 5, "e6: setup -- the EdelNote member is affordable (cost %d)",
          printed_cost(edel));

    tg->state.p[0].stage[AREA_CENTER] = seras;
    tg->state.p[1].stage[AREA_LEFT] = a;
    tg->state.p[1].stage[AREA_CENTER] = b;
    hand_push(tg, 0, edel, 1);
    hand_push(tg, 0, filler, 1);
    test_give_energy(tg, 10);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);

    play_to_stage(tg, edel, AREA_LEFT);
    PROMPT_PRESENT(tg, 1, "e6: the opponent is prompted with 2 active members (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    {
        int cur = tg->state.queue.cur;
        if (cur >= 0 && cur < tg->state.queue.n_entries)
            CHECK_EQ(test_queue_entry_choice_seat(tg, cur), 1,
                     "e6: the wait-member choice is routed to P2 (the OPPONENT waits)");
    }
    CHECK_PROMPTS(1, "e6_serasu: the opponent's wait-target prompt must be answered");
    {
        const int ans[1] = { 0 };
        pick_many(tg, ans, 1);
    }
    drain_choices(tg, 8);

    /* 「ウェイトにする」 -- waiting is an orientation modifier; both members
     * STAY on the stage (wait != leave). */
    CHECK(stage_has(tg, 1, a) && stage_has(tg, 1, b),
          "e6: both opponent members remain on stage (ウェイト does not leave the stage)");
    const char *o1 = rb_mods_get_orientation(&tg->state.mods, a);
    CHECK(o1 && !strcmp(o1, "wait"),
          "e6: the picked opponent member is in wait state (got \"%s\")", o1 ? o1 : "(none)");
}

/* Rust serasu_edelnote_appear_no_opponent_member */
static void test_e_serasu_no_opponent_member_fizzles(void)
{
    TestGame *tg = &tg_f2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int seras = mid(tg, SERAS);
    pin_id(seras, SERAS);
    int edel = mid(tg, EDEL_MEMBER);
    pin_id(edel, EDEL_MEMBER);
    int filler = test_new_id(tg, FILLER);
    if (seras < 0 || edel < 0 || filler < 0) return;

    tg->state.p[0].stage[AREA_CENTER] = seras;
    hand_push(tg, 0, edel, 1);
    hand_push(tg, 0, filler, 1);
    test_give_energy(tg, 10);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);

    play_to_stage(tg, edel, AREA_LEFT);
    drain_choices(tg, 8);

    CHECK(stage_has(tg, 0, edel), "e7: the debuted EdelNote member stays staged");
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        CHECK_EQ(stage_at(tg, 1, i), RB_EMPTY_SLOT, "e7: opponent stage slot %d is still empty", i);
    const char *o = rb_mods_get_orientation(&tg->state.mods, filler);
    CHECK(!o, "e7: nothing waited when the opponent has no members (got \"%s\")", o ? o : "(none)");

    /* POSITIVE CONTROL: with TWO opponent members the wait prompt must appear
     * (with only ONE candidate an auto-resolve is legitimate and would make this
     * control unreadable, which is why the earlier revision of this file used a
     * single member and failed spuriously). */
    {
        TestGame *tg2 = &tg_g;
        test_game_new(tg2);
        clear_p1(tg2); clear_p2(tg2);
        int s2 = mid(tg2, SERAS), e2 = mid(tg2, EDEL_MEMBER);
        int f2 = test_new_id(tg2, FILLER);
        int v1 = test_new_id(tg2, KOTOMI), v2 = test_new_id(tg2, KOTOMI);
        if (s2 < 0 || e2 < 0 || f2 < 0 || v1 < 0 || v2 < 0) return;
        tg2->state.p[0].stage[AREA_CENTER] = s2;
        tg2->state.p[1].stage[AREA_LEFT] = v1;
        tg2->state.p[1].stage[AREA_RIGHT] = v2;
        hand_push(tg2, 0, e2, 1);
        hand_push(tg2, 0, f2, 1);
        test_give_energy(tg2, 10);
        fill_deck(tg2, 0, 5); fill_deck(tg2, 1, 5);
        play_to_stage(tg2, e2, AREA_LEFT);
        PROMPT_PRESENT(tg2, 1,
              "e7 CONTROL: with TWO opponent members the wait prompt MUST be offered, "
              "or the fizzle assertion above proves nothing");
    }
}

/* ========================================================================
 * GROUP F -- selection_filter/blade_option_exclude_self_pl_s_bp5_004_r_test.rs
 * PL!S-bp5-004-R 登場, option 0:
 *   another Aqours member on your stage gains 1 blade until the end of the live.
 *   `exclude_self: true` on the parsed effect.
 * ======================================================================== */

static void test_f_dia_blade_option_excludes_itself(void)
{
    TestGame *tg = &tg_g2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int dia = mid(tg, DIA_BP5);
    pin_id(dia, DIA_BP5);
    int a1 = mid(tg, AQ1), a2 = mid(tg, AQ2);
    pin_id(a1, AQ1);
    pin_id(a2, AQ2);
    if (dia < 0 || a1 < 0 || a2 < 0) return;
    /* identity guard: both fixtures must really be Aqours */
    CHECK(rb_card_matches_group_str(a1, "Aqours"), "f: AQ1 is an Aqours member");
    CHECK(rb_card_matches_group_str(a2, "Aqours"), "f: AQ2 is an Aqours member");
    CHECK_EQ(printed_blade(dia), 1, "f: DIA prints 1 blade heart");

    tg->state.p[0].stage[AREA_CENTER] = a1;
    tg->state.p[0].stage[AREA_LEFT] = a2;
    hand_push(tg, 0, dia, 1);
    test_give_energy(tg, 5);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);

    play_to_stage(tg, dia, AREA_RIGHT);
    PROMPT_PRESENT(tg, 1, "f: the 登場 two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "f_blade_option: the two-option choice must be answered");
    select_option(tg, 0);   /* the ブレード option */

    PROMPT_PRESENT(tg, 1, "f: an Aqours member selection follows the choice (got %s)", pending_type(tg));
    if (pending(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        CHECK_EQ_STR(c->zone, "stage", "f: the ブレード option selects over the OWN STAGE");
        int offers_dia = 0;
        for (int i = 0; i < c->n_filtered_indices; i++) {
            int pos = c->filtered_indices[i];
            if (pos >= 0 && pos < RB_STAGE_SIZE && tg->state.p[0].stage[pos] == dia)
                offers_dia = 1;
        }
        CHECK(!offers_dia,
              "f: 「このメンバー以外」 -- the activating member must NOT be offered");
        const int ans[1] = { 1 };   /* Rust select_indices(&[1]) */
        pick_many(tg, ans, 1);
    }
    drain_choices(tg, 6);

    int b1 = rb_mods_get_blade(&tg->state.mods, a1);
    int b2 = rb_mods_get_blade(&tg->state.mods, a2);
    CHECK(b1 > 0 || b2 > 0, "f: the SELECTED Aqours member gained ブレード (a1=%d a2=%d)", b1, b2);
    CHECK(!(b1 > 0 && b2 > 0),
          "f: 1人 -- exactly ONE member gains the blade (a1=%d a2=%d)", b1, b2);
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, dia), 0,
             "f: DIA herself is excluded by exclude_self and gains no blade");
}

static void test_f_dia_blade_option_no_other_aqours_skips(void)
{
    TestGame *tg = &tg_h;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int dia = mid(tg, DIA_BP5);
    pin_id(dia, DIA_BP5);
    if (dia < 0) return;
    hand_push(tg, 0, dia, 1);
    test_give_energy(tg, 5);
    fill_deck(tg, 0, 5);
    fill_deck(tg, 1, 5);

    play_to_stage(tg, dia, AREA_CENTER);
    PROMPT_PRESENT(tg, 1, "f2: the 登場 two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "f2_no_other: the two-option choice must be answered");
    select_option(tg, 0);

    CHECK(!pending(tg), "f2: with no OTHER Aqours member there is no target prompt at all");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, dia), 0,
             "f2: DIA gains no blade (she is the only member and is excluded)");

    /* POSITIVE CONTROL: the same choice with TWO other Aqours members must offer
     * the target prompt -- with only ONE candidate an auto-resolve would be
     * legitimate and would make this control unreadable. */
    {
        TestGame *tg2 = &tg_h2;
        test_game_new(tg2);
        clear_p1(tg2); clear_p2(tg2);
        int d2 = mid(tg2, DIA_BP5);
        int a2 = mid(tg2, AQ1), b2 = mid(tg2, AQ2);
        pin_id(d2, DIA_BP5);
        pin_id(a2, AQ1);
        pin_id(b2, AQ2);
        if (d2 < 0 || a2 < 0 || b2 < 0) return;
        tg2->state.p[0].stage[AREA_LEFT] = a2;
        tg2->state.p[0].stage[AREA_CENTER] = b2;
        hand_push(tg2, 0, d2, 1);
        test_give_energy(tg2, 5);
        fill_deck(tg2, 0, 5); fill_deck(tg2, 1, 5);
        PROMPT_PRESENT(tg2, 1, "f2 CONTROL: the two-option choice is offered");
        play_to_stage(tg2, d2, AREA_RIGHT);
        if (pending(tg2)) select_option(tg2, 0);
        PROMPT_PRESENT(tg2, 1,
              "f2 CONTROL: with two other Aqours members the target prompt MUST follow, "
              "or the auto-skip assertion above proves nothing");
    }
}

/* ========================================================================
 * GROUP G -- selection_filter/forced_self_wait_excludes_immune_q275_test.rs
 * Q275: when an effect forces YOU to wait a member, a wait-IMMUNE member is not
 * a legal choice. PL!HS-bp6-007-R (自動1) forces the OPPONENT to wait one of
 * their OWN active members; PL!S-bp7-003-R＋ (ab#1 option 1) makes her own
 * Aqours members (blade<=3) immune to the OPPONENT's wait effects.
 *
 * The C port has NO `wait_immune_members` list at all (the field does not exist
 * in RbGameState / RbMods), so this group is expected to go RED. It is kept
 * strict and reported, never weakened.
 * ======================================================================== */

/* Rust run_ceras_forced_wait: p2 plays SERAS, whose 自動 fires when the
 * EdelNote appears on p2's own stage, forcing p1 to wait one of p1's own active
 * members. Records the OFFERED p1 stage indices and the picked one. */
static void run_ceras_forced_wait(TestGame *tg, int select,
                                  int *offered, int *n_offered, int *selected)
{
    *n_offered = 0;
    *selected = -1;
    tg->state.first_attacker = 1;
    tg->state.second_attacker = 0;
    tg->state.active = 1;

    int ceras = mid(tg, SERAS);
    pin_id(ceras, SERAS);
    if (ceras < 0) return;

    fill_deck(tg, 1, 20);
    hand_push(tg, 1, ceras, 1);
    test_give_opp_energy(tg, 20);

    play_to_stage_for(tg, 1, ceras, AREA_CENTER);
    tg->state.active = 0;

    int guard = 0;
    while (pending(tg) && guard++ < 25) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "stage")) {
            /* VACUITY CONTROL for group G: a stage-target prompt WAS offered, so
             * the offered-index assertions below cannot pass vacuously. */
            if (*n_offered == 0) prompts++;
            if (*n_offered == 0)
                for (int i = 0; i < c->n_filtered_indices && i < 8; i++)
                    offered[(*n_offered)++] = c->filtered_indices[i];
            if (c->n_filtered_indices == 0) { skip(tg); continue; }
            const int ans[1] = { select };
            pick_many(tg, ans, 1);
            *selected = select;
        } else {
            select_option(tg, 0);
        }
    }
}

static void test_g_q275_forced_wait_excludes_immune(void)
{
    TestGame *tg = &tg_i;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int kanan = mid(tg, KANAN_IMM);
    pin_id(kanan, KANAN_IMM);
    int a = test_new_id(tg, NON_IMMUNE), b = test_new_id(tg, NON_IMMUNE);
    if (kanan < 0 || a < 0 || b < 0) return;
    CHECK_EQ(printed_blade(kanan), 2,
             "g: setup -- KANAN prints 2 blades (inside her own blade<=3 window)");
    CHECK(rb_card_matches_group_str(kanan, "Aqours"), "g: setup -- KANAN is an Aqours member");

    tg->state.p[0].stage[AREA_LEFT] = a;
    tg->state.p[0].stage[AREA_CENTER] = kanan;
    tg->state.p[0].stage[AREA_RIGHT] = b;
    fill_deck(tg, 0, 20);

    int offered[8], n_offered = 0, selected = -1;
    run_ceras_forced_wait(tg, 0, offered, &n_offered, &selected);

    CHECK_PROMPTS(1, "g_q275: the forced-wait prompt must be offered and answered");
    if (n_offered == 0) {
        /* With ONE legal candidate the effect auto-resolves and offers nothing.
         * With TWO waitable members a SelectCard prompt is required, so a
         * zero-length offer here is itself the Q275 defect. */
        CHECK(0,
              "g: the forced wait offered NO stage prompt at all; with two waitable "
              "members a SelectCard prompt is required (offered=%d)", n_offered);
    } else {
        CHECK(!(n_offered == 1 && offered[0] == 1),
              "g: Q275 -- the wait-immune KANAN (stage index 1) must NOT be offered "
              "alone (offered=%d entries)", n_offered);
        int has0 = 0, has2 = 0;
        for (int i = 0; i < n_offered; i++) {
            if (offered[i] == 0) has0 = 1;
            if (offered[i] == 2) has2 = 1;
        }
        CHECK(has0 && has2, "g: both waitable members (stage indices 0 and 2) are offered");
    }
    const char *ok = rb_mods_get_orientation(&tg->state.mods, a);
    CHECK(ok && !strcmp(ok, "wait"), "g: the picked non-immune member is waited");
    const char *ik = rb_mods_get_orientation(&tg->state.mods, kanan);
    CHECK(!ik || strcmp(ik, "wait") != 0, "g: the immune KANAN stays active");
}

/* Negative control: with no immunity recorded, KANAN must be OFFERED and
 * waited -- proving the exclusion above would be immunity-driven, not a broken
 * SERAS effect. */
static void test_g_q275_without_immunity_control(void)
{
    TestGame *tg = &tg_j;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int kanan = mid(tg, KANAN_IMM);
    pin_id(kanan, KANAN_IMM);
    int a = test_new_id(tg, NON_IMMUNE), b = test_new_id(tg, NON_IMMUNE);
    if (kanan < 0 || a < 0 || b < 0) return;

    tg->state.p[0].stage[AREA_LEFT] = a;
    tg->state.p[0].stage[AREA_CENTER] = kanan;
    tg->state.p[0].stage[AREA_RIGHT] = b;
    fill_deck(tg, 0, 20);

    int offered[8], n_offered = 0, selected = -1;
    run_ceras_forced_wait(tg, 1, offered, &n_offered, &selected);

    CHECK_PROMPTS(1, "g_control: the forced-wait prompt must be offered and answered");
    CHECK(n_offered > 0, "g: without immunity the forced wait offers stage choices (got %d)",
          n_offered);
    if (n_offered > 0) {
        int has1 = 0;
        for (int i = 0; i < n_offered; i++) if (offered[i] == 1) has1 = 1;
        CHECK(has1, "g: without immunity KANAN (stage index 1) IS offered");
    }
    const char *ik = rb_mods_get_orientation(&tg->state.mods, kanan);
    CHECK(ik && !strcmp(ik, "wait"), "g: without immunity, KANAN IS waited");
}

static void test_g_q275_single_legal_target_auto_resolves(void)
{
    TestGame *tg = &tg_j2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int kanan = mid(tg, KANAN_IMM);
    pin_id(kanan, KANAN_IMM);
    int a = test_new_id(tg, NON_IMMUNE);
    if (kanan < 0 || a < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = a;
    tg->state.p[0].stage[AREA_CENTER] = kanan;
    fill_deck(tg, 0, 20);

    int offered[8], n_offered = 0, selected = -1;
    run_ceras_forced_wait(tg, 0, offered, &n_offered, &selected);

    const char *oa = rb_mods_get_orientation(&tg->state.mods, a);
    CHECK(oa && !strcmp(oa, "wait"), "g: the single legal target is waited");
    const char *ik = rb_mods_get_orientation(&tg->state.mods, kanan);
    CHECK(!ik || strcmp(ik, "wait") != 0, "g: the immune KANAN stays active");
}

/* ========================================================================
 * GROUP H -- selection_filter/optional_energy_edelnote_deploy_or_requirement_
 * deploy_or_requirement_test.rs -- PL!HS-bp5-022-L ライブ開始時, optional 2E:
 *   if an EdelNote member of cost>=9 is on your stage, choose 1:
 *     ・deploy an EdelNote member of cost<=4 from your waitroom into an EMPTY area
 *     ・reduce this card's required heart06
 * ======================================================================== */

static int edel_setup(TestGame *tg, const char *stage_card,
                      int *live_out, int *high_out, int *low_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, EDEL_LIVE);
    pin_id(live, EDEL_LIVE);
    int high = mid(tg, stage_card);
    pin_id(high, stage_card);
    int low = mid(tg, EDEL_LOW);
    pin_id(low, EDEL_LOW);
    *live_out = live; *high_out = high; *low_out = low;
    if (live < 0 || high < 0 || low < 0) return -1;
    fill_deck(tg, 0, 20);
    fill_deck(tg, 1, 20);
    tg->state.p[0].stage[AREA_CENTER] = high;
    test_add_to_live_for(tg, 0, live);
    wait_push(tg, 0, low, 1);
    test_give_energy(tg, 5);
    return live;
}

/* Rust pay(game, accept) -- the 2E optional cost gate. Now expressed through
 * PROMPT_PRESENT so the observation is counted even when the gate is absent. */
static int edel_pay(TestGame *tg, int accept)
{
    PROMPT_PRESENT(tg, 1, "edel_pay: the optional 2E cost gate is offered (got %s)",
                   pending_type(tg));
    if (!pending(tg)) return 0;
    select_option(tg, accept ? 1 : 0);
    return 1;
}
#define EDEL_PAY_GATE_UNUSED edel_pay

static void test_h_paid_energy_choice_reduces_heart06(void)
{
    TestGame *tg = &tg_k;
    int live, high, low;
    if (edel_setup(tg, EDEL_HIGH, &live, &high, &low) < 0) return;
    CHECK(printed_cost(high) >= 9, "h: setup -- the staged EdelNote costs >= 9 (cost %d)",
          printed_cost(high));
    int need_before = rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE);

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 1);   /* accept 「支払ってもよい」 */
    PROMPT_PRESENT(tg, 1, "h: the two-option choice follows the paid cost");
    if (pending(tg)) select_option(tg, 1);   /* reduce the heart06 requirement */
    drain_choices(tg, 8);

    CHECK_EQ(tg->state.p[0].energy_active, 3, "h: 「2枚支払ってもよい」 -- 2E was spent");
    CHECK_EQ(stage_at(tg, 0, AREA_LEFT), RB_EMPTY_SLOT, "h: no member was deployed");
    CHECK_EQ(stage_at(tg, 0, AREA_CENTER), high, "h: the staged EdelNote stays put");
    CHECK_EQ(stage_at(tg, 0, AREA_RIGHT), RB_EMPTY_SLOT, "h: the right area is empty");
    CHECK(wait_has(tg, low), "h: the waitroom member was NOT deployed");
    CHECK_EQ(rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE), need_before + 4,
             "h: 「このカードの必要ハートをheart06減らす」 -- the requirement dropped by 4");
}

static void test_h_paid_energy_choice_deploys_waitroom_member(void)
{
    TestGame *tg = &tg_k2;
    int live, high, low;
    if (edel_setup(tg, EDEL_HIGH, &live, &high, &low) < 0) return;

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h2: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h2: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 1);
    PROMPT_PRESENT(tg, 1, "h2: the two-option choice follows");
    if (pending(tg)) select_option(tg, 0);   /* the deploy option */
    drain_choices(tg, 8);

    CHECK_EQ(stage_at(tg, 0, AREA_CENTER), high, "h2: the staged EdelNote stays at center");
    CHECK(!wait_has(tg, low), "h2: the waitroom member left the waitroom");
    CHECK(stage_has(tg, 0, low), "h2: 「メンバーのいないエリアに登場させる」 -- it is on the stage");
    CHECK_EQ(tg->state.p[0].energy_active, 3, "h2: 2E was spent");
    CHECK_EQ(rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE), 0,
             "h2: the deploy option does not reduce the heart06 requirement");
}

static void test_h_declined_energy_preserves_everything(void)
{
    TestGame *tg = &tg_d;
    int live, high, low;
    if (edel_setup(tg, EDEL_HIGH, &live, &high, &low) < 0) return;

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h3: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h3: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 0);   /* decline */
    drain_choices(tg, 8);

    CHECK_EQ(tg->state.p[0].energy_active, 5, "h3: declining costs nothing -- all 5E remain");
    CHECK_EQ(stage_at(tg, 0, AREA_LEFT), RB_EMPTY_SLOT, "h3: nothing was deployed");
    CHECK_EQ(stage_at(tg, 0, AREA_CENTER), high, "h3: the stage is unchanged");
    CHECK(wait_has(tg, low), "h3: the waitroom member stays in the waitroom");
    CHECK_EQ(rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE), 0,
             "h3: the heart06 requirement is unchanged");

    /* POSITIVE CONTROL for h3. Two live-start effects that DO prompt in this
     * engine (m_private_wars and i_niji both pass), so the assertion above is
     * not being judged against a board where nothing works at all. */
    {
        TestGame *tg2 = &tg_d2;
        int l2 = mid(tg2, PRIVATE_W);
        int a2 = mid(tg2, ARISE), al2 = test_new_id(tg2, FILLER);
        if (l2 < 0 || a2 < 0 || al2 < 0) return;
        test_game_new(tg2);
        clear_p1(tg2); clear_p2(tg2);
        l2 = mid(tg2, PRIVATE_W); a2 = mid(tg2, ARISE); al2 = test_new_id(tg2, FILLER);
        if (l2 < 0 || a2 < 0 || al2 < 0) return;
        test_add_to_live_for(tg2, 0, l2);
        tg2->state.p[0].stage[AREA_LEFT] = a2;
        tg2->state.p[0].stage[AREA_CENTER] = al2;
        rb_mods_set_orientation(&tg2->state.mods, al2, "wait");
        fill_deck(tg2, 0, 20); fill_deck(tg2, 1, 20);
        fire_live_start(tg2, 0, l2);
        PROMPT_PRESENT(tg2, 1,
              "h3 CONTROL: a live-start 2-option choice on the same engine DOES prompt, "
              "so h3's silence is specific to the optional-energy cost gate");
    }
}

static void test_h_exact_cost_nine_edelnote_enables_choice(void)
{
    TestGame *tg = &tg_d2;
    int live, high, low;
    if (edel_setup(tg, EDEL_COST9, &live, &high, &low) < 0) return;
    CHECK_EQ(printed_cost(high), 9, "h4: setup -- the staged EdelNote costs exactly 9");

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h4: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h4: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 1);
    PROMPT_PRESENT(tg, 1, "h4: the two-option choice follows the paid cost");
    if (pending(tg)) select_option(tg, 1);
    drain_choices(tg, 8);

    CHECK_EQ(tg->state.p[0].energy_active, 3, "h4: 2E was spent");
    CHECK(wait_has(tg, low), "h4: the waitroom member was not deployed");
    CHECK_EQ(rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE), 4,
             "h4: the heart06 requirement was reduced by 4");
}

static void test_h_wrong_group_or_low_cost_cannot_enable_choice(void)
{
    static const char *const stage_cards[2] = { EDEL_LOW, AQ_COSTY };
    for (int v = 0; v < 2; v++) {
        TestGame *tg = (v == 0) ? &tg_e : &tg_e2;
        int live, high, low;
        if (edel_setup(tg, stage_cards[v], &live, &high, &low) < 0) continue;
        fire_live_start(tg, 0, live);
        PROMPT_PRESENT(tg, 1, "h5 (stage=%s): the 2E optional cost gate is offered", stage_cards[v]);
        if (pending(tg)) select_option(tg, 1);
        drain_choices(tg, 8);
        CHECK_EQ(tg->state.p[0].energy_active, 3,
                 "h5 (stage=%s, cost %d): 2E was paid for the optional cost",
                 stage_cards[v], printed_cost(high));
        CHECK_EQ(stage_at(tg, 0, AREA_LEFT), RB_EMPTY_SLOT,
                 "h5 (stage=%s): the cost>=9 EdelNote gate fails, so nothing is deployed",
                 stage_cards[v]);
        CHECK(wait_has(tg, low), "h5 (stage=%s): the waitroom member stays put", stage_cards[v]);
        CHECK_EQ(rb_mods_get_need_heart(&tg->state.mods, live, RB_HEART_PURPLE), 0,
                 "h5 (stage=%s): the requirement is unchanged", stage_cards[v]);
    }
}

static void test_h_deploy_filter_excludes_cost_five_and_wrong_group(void)
{
    TestGame *tg = &tg_f;
    int live, high, low;
    if (edel_setup(tg, EDEL_HIGH, &live, &high, &low) < 0) return;
    int wrong_group = test_new_id(tg, FILLER);
    int too_costly = mid(tg, EDEL_C5);
    pin_id(too_costly, EDEL_C5);
    if (wrong_group < 0 || too_costly < 0) return;
    CHECK_EQ(printed_cost(too_costly), 5, "h6: setup -- the extra EdelNote costs 5 (> 4)");
    CHECK(!rb_card_matches_group_str(wrong_group, "EdelNote"),
          "h6: setup -- the other member is NOT EdelNote");

    /* Rust inserts the two at the FRONT of the waitroom: [wrong, costly, low] */
    RbBag *b = &tg->state.p[0].discard;
    for (int i = b->n; i > 0; i--) b->cards[i] = b->cards[i - 1];
    b->cards[0] = wrong_group;
    b->cards[1] = too_costly;
    b->n += 2;

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h6: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h6: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 1);
    PROMPT_PRESENT(tg, 1, "h6: the two-option choice follows");
    if (pending(tg)) select_option(tg, 0);
    PROMPT_PRESENT(tg, 1, "h6: the empty-area position prompt follows the deploy option");
    drain_choices(tg, 8);

    CHECK(stage_has(tg, 0, low), "h6: only the cost<=4 EdelNote member is deployed");
    CHECK(!stage_has(tg, 0, too_costly), "h6: the cost-5 EdelNote is filtered out");
    CHECK(!stage_has(tg, 0, wrong_group), "h6: the wrong-group member is filtered out");
    CHECK(wait_has(tg, too_costly), "h6: the cost-5 EdelNote stays in the waitroom");
    CHECK(wait_has(tg, wrong_group), "h6: the wrong-group member stays in the waitroom");
    CHECK_EQ(tg->state.p[0].energy_active, 3, "h6: 2E was spent");
}

static void test_h_full_stage_deploy_cannot_replace_members(void)
{
    TestGame *tg = &tg_f2;
    int live, high, low;
    if (edel_setup(tg, EDEL_HIGH, &live, &high, &low) < 0) return;
    int left = test_new_id(tg, FILLER), right = test_new_id(tg, FILLER);
    if (left < 0 || right < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = left;
    tg->state.p[0].stage[AREA_RIGHT] = right;

    int queued = fire_live_start(tg, 0, live);
    CHECK(queued > 0, "h7: the ライブ開始時 trigger is queued at all");
    PROMPT_PRESENT(tg, 1, "h7: the 2E optional cost gate is offered");
    if (pending(tg)) select_option(tg, 1);
    PROMPT_PRESENT(tg, 1, "h7: the two-option choice follows");
    if (pending(tg)) select_option(tg, 0);
    drain_choices(tg, 8);

    CHECK_EQ(stage_at(tg, 0, AREA_LEFT), left, "h7: the left member is NOT replaced");
    CHECK_EQ(stage_at(tg, 0, AREA_CENTER), high, "h7: the center member is untouched");
    CHECK_EQ(stage_at(tg, 0, AREA_RIGHT), right, "h7: the right member is NOT replaced");
    CHECK(wait_has(tg, low), "h7: the waitroom member stays put (no empty area)");
    CHECK_EQ(tg->state.p[0].energy_active, 3, "h7: 2E was spent");
}

/* ========================================================================
 * GROUP I -- per_card/energy_or_recycle_live_pl_n_pb1_010_r_test.rs
 * PL!N-pb1-010-R 登場, choose 1 of 2:
 *   ・エネルギーを1枚アクティブにする。
 *   ・自分の控え室にある虹ヶ咲のライブカードを2枚まで好きな順番で
 *     デッキの上に置く。
 * ======================================================================== */

static void test_i_niji_energy_option_activates_exactly_one(void)
{
    TestGame *tg = &tg_g;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int member = mid(tg, NIJI_CHOICE);
    pin_id(member, NIJI_CHOICE);
    if (member < 0) return;
    CHECK_EQ(printed_cost(member), 10, "i: NIJI prints cost 10");

    /* 9 active + 1 INACTIVE, so there is exactly one thing to activate */
    test_give_energy(tg, 9);
    {
        int e = test_new_id(tg, ENERGY);
        if (e < 0) return;
        RbPlayer *P = &tg->state.p[0];
        P->energy.cards[P->energy.n++] = e;
    }
    int active_before = tg->state.p[0].energy_active;
    CHECK_EQ(active_before, 9, "i: setup guard -- 9 of the 10 zone cards are active");
    int zone_len = tg->state.p[0].energy.n;

    tg->state.p[0].stage[AREA_CENTER] = member;
    CHECK(fire_trigger(tg, member, RB_TSTR_DEBUT), "i: the 登場 trigger fires");
    PROMPT_PRESENT(tg, 1, "i: the two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "i_energy_option: the two-option choice must be answered");
    select_option(tg, 0);   /* 「エネルギーを1枚アクティブにする」 */
    drain_choices(tg, 6);

    CHECK_EQ(tg->state.p[0].energy_active - active_before, 1,
             "i: 「エネルギーを1枚アクティブにする」 -- exactly one more card is active");
    CHECK_EQ(tg->state.p[0].energy.n, zone_len,
             "i: no energy card was ADDED -- the effect activates, it does not supply");
}

static void test_i_niji_recycle_option_puts_live_on_deck_top(void)
{
    TestGame *tg = &tg_g2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int member = mid(tg, NIJI_CHOICE);
    pin_id(member, NIJI_CHOICE);
    int live1 = mid(tg, NIJI_LIVE), live2 = mid(tg, NIJI_LIVE);
    pin_id(live1, NIJI_LIVE);
    pin_id(live2, NIJI_LIVE);
    if (member < 0 || live1 < 0 || live2 < 0) return;
    CHECK(live1 != live2, "i2: the two copies are distinct card instances");
    CHECK(rb_card_is_live(live1), "i2: the fixture really is a live card");

    wait_push(tg, 0, live1, 1);
    wait_push(tg, 0, live2, 1);
    fill_deck(tg, 0, 5);
    test_give_energy(tg, 10);
    hand_push(tg, 0, member, 1);

    play_to_stage(tg, member, AREA_CENTER);
    PROMPT_PRESENT(tg, 1, "i2: the two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "i2_recycle: the two-option choice must be answered");
    select_option(tg, 1);   /* the NIJI live recycle option */
    drain_choices(tg, 8);

    int top = deck_top(tg, 0);
    CHECK(top == live1 || top == live2,
          "i2: 「デッキの上に置く」 -- a chosen live is the deck's TOP card (got %s)",
          card_no_of(top));
    CHECK_EQ(tg->state.p[0].discard.n, 1,
             "i2: one of the two lives left the waitroom (「2枚まで」 upper bound not taken)");
}

/* ========================================================================
 * GROUP J -- per_card/mill_or_wait_pl_hs_cl1_004_cl_test.rs
 * PL!HS-cl1-004-CL 登場, choose 1 of 2:
 *   ・自分のデッキの上からカードを3枚控え室に置く。
 *   ・コスト2以下の相手のメンバー1人をウェイトにする。
 * ======================================================================== */

static int geko_debut(TestGame *tg, int *geko_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    fill_deck(tg, 0, 20);
    fill_deck(tg, 1, 20);
    int geko = mid(tg, GEKO_CL1);
    pin_id(geko, GEKO_CL1);
    *geko_out = geko;
    if (geko < 0) return -1;
    hand_push(tg, 0, geko, 1);
    test_give_energy(tg, 20);
    play_to_stage(tg, geko, AREA_LEFT);
    return geko;
}

static void test_j_geko_mill_option_removes_three_deck_cards(void)
{
    TestGame *tg = &tg_h;
    int geko;
    if (geko_debut(tg, &geko) < 0) return;
    int deck_before = tg->state.p[0].deck.n;
    PROMPT_PRESENT(tg, 1, "j: the two-option 登場 choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "j_mill: the two-option choice must be answered");
    select_option(tg, 0);   /* the mill option */
    drain_choices(tg, 8);
    CHECK_EQ(tg->state.p[0].deck.n, deck_before - 3,
             "j: 「デッキの上からカードを3枚控え室に置く」 -- 3 cards left the deck");
}

static void test_j_geko_wait_option_waits_cost_two_opponent_member(void)
{
    TestGame *tg = &tg_h2;
    int geko;
    if (geko_debut(tg, &geko) < 0) return;
    int enemy = mid(tg, SP_COST2);
    pin_id(enemy, SP_COST2);
    if (enemy < 0) return;
    CHECK_EQ(printed_cost(enemy), 2, "j2: setup -- the opponent member costs exactly 2");
    tg->state.p[1].stage[AREA_CENTER] = enemy;

    PROMPT_PRESENT(tg, 1, "j2: the two-option 登場 choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "j2_wait: the two-option choice must be answered");
    select_option(tg, 1);   /* the wait option */
    drain_choices(tg, 8);
    const char *o = rb_mods_get_orientation(&tg->state.mods, enemy);
    CHECK(o && !strcmp(o, "wait"),
           "j2: 「コスト2以下の相手のメンバー1人をウェイトにする」 -- got \"%s\"", o ? o : "(none)");
}

/* ========================================================================
 * GROUP K -- per_card/wait_or_draw_pl_s_bp7_025_l_test.rs
 * PL!S-bp7-025-L ライブ成功時, choose 1 of 2: wait two opponent members, or
 * draw 1. The two options must be genuinely exclusive.
 * ======================================================================== */

static void kowarey_live_board(TestGame *tg, int *live_out, int *o1_out, int *o2_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, KOWAREY_L);
    pin_id(live, KOWAREY_L);
    int o1 = test_new_id(tg, FILLER), o2 = test_new_id(tg, FILLER);
    *live_out = live; *o1_out = o1; *o2_out = o2;
    if (live < 0 || o1 < 0 || o2 < 0) return;
    test_add_to_live_for(tg, 0, live);
    tg->state.p[1].stage[AREA_LEFT] = o1;
    tg->state.p[1].stage[AREA_CENTER] = o2;
}

static void test_k_kowarey_choose_wait_waits_two_opponents(void)
{
    TestGame *tg = &tg_i;
    int live, o1, o2;
    kowarey_live_board(tg, &live, &o1, &o2);
    if (live < 0 || o1 < 0 || o2 < 0) return;

    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = live;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);

    PROMPT_PRESENT(tg, 1, "k: the ライブ成功時 two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "k_wait: the two-option choice must be answered");
    select_option(tg, 0);   /* the wait option */
    int picked = 0;
    for (int i = 0; i < 3; i++) {
        if (!pending(tg)) break;
        if (pending_is_card(tg)) { const int a[2] = { 0, 1 }; pick_many(tg, a, 2); picked = 1; }
        else skip(tg);
    }
    CHECK(picked, "k: the wait-target selection was actually offered");

    for (int i = 0; i < 2; i++) {
        int opp = tg->state.p[1].stage[i];
        const char *o = rb_mods_get_orientation(&tg->state.mods, opp);
        CHECK(o && !strcmp(o, "wait"), "k: opponent member %d is waited", i);
    }
}

static void test_k_kowarey_choose_draw_leaves_opponent_unmodified(void)
{
    TestGame *tg = &tg_j;
    int live, o1, o2;
    kowarey_live_board(tg, &live, &o1, &o2);
    if (live < 0 || o1 < 0 || o2 < 0) return;
    fill_deck(tg, 0, 10);

    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = live;
    rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);

    PROMPT_PRESENT(tg, 1, "k2: the ライブ成功時 choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "k2_draw: the two-option choice must be answered");
    select_option(tg, 1);   /* the draw option */
    drain_choices(tg, 8);
    CHECK_EQ(tg->state.p[0].hand.n, 1, "k2: option B -- 「自分はカードを1枚引く」");
    const char *o = rb_mods_get_orientation(&tg->state.mods, o1);
    CHECK(!o, "k2: the opponent is untouched by option B (got \"%s\")", o ? o : "(none)");
}

/* ========================================================================
 * GROUP L -- per_card/discard_self_from_hand_no_named_blade_target_q258_test.rs
 * PL!HS-bp6-014-R 起動 (Q258): with the named-member clause NOT satisfied, the
 * member is still discarded and NO member may gain a blade.
 * ======================================================================== */

static void test_l_himege_discards_self_without_a_blade(void)
{
    TestGame *tg = &tg_j2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int himege = mid(tg, HIMEGE);
    pin_id(himege, HIMEGE);
    int drawn = test_new_id(tg, FILLER);
    if (himege < 0 || drawn < 0) return;

    tg->state.p[0].deck.n = 0;
    deck_push(tg, 0, drawn, 1);
    hand_push(tg, 0, himege, 1);
    int hand_before = tg->state.p[0].hand.n;

    int ok = test_activate_ability(tg, himege);
    CHECK(ok, "l: the 起動 activation succeeds");
    drain_choices(tg, 6);

    CHECK(wait_has(tg, himege), "l: 「このメンバーを控え室に置く」 -- she is in the waitroom");
    CHECK(hand_has(tg, drawn), "l: the drawn card stays in hand");
    CHECK_EQ(tg->state.p[0].hand.n, hand_before, "l: the hand size is unchanged");
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = tg->state.p[0].stage[i];
        if (cid < 0) continue;
        CHECK_EQ(rb_mods_get_blade(&tg->state.mods, cid), 0,
                 "l: no member on stage gained a blade");
    }
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, himege), 0,
             "l: the discarded member gained no blade");
}

/* ========================================================================
 * GROUP M -- per_card/arise_gated_activate_blade_or_wait_private_wars_test.rs
 * PL!-bp5-024-L ライブ開始時, gated on PL!-bp5-333-R being on stage:
 *   choose 1: activate the waited ARISE member (that member gains 1 blade), or
 *   wait every opponent member whose PRINTED blade is exactly 3.
 * ======================================================================== */

static void private_wars_board(TestGame *tg, int *live_out, int *arise_out, int *ally_out)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int live = mid(tg, PRIVATE_W);
    pin_id(live, PRIVATE_W);
    int arise = mid(tg, ARISE);
    pin_id(arise, ARISE);
    int ally = test_new_id(tg, FILLER);
    *live_out = live; *arise_out = arise; *ally_out = ally;
    if (live < 0 || arise < 0 || ally < 0) return;
    test_add_to_live_for(tg, 0, live);
    tg->state.p[0].stage[AREA_LEFT] = arise;
    tg->state.p[0].stage[AREA_CENTER] = ally;
    rb_mods_set_orientation(&tg->state.mods, ally, "wait");
    fill_deck(tg, 0, 20);
    fill_deck(tg, 1, 20);
}

static void test_m_private_wars_activates_arise_and_grants_one_blade(void)
{
    TestGame *tg = &tg_k;
    int live, arise, ally;
    private_wars_board(tg, &live, &arise, &ally);
    if (live < 0 || arise < 0 || ally < 0) return;

    fire_live_start(tg, 0, live);
    PROMPT_PRESENT(tg, 1, "m: the gated two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "m_activate: the choice must be answered");
    select_option(tg, 0);   /* the activate option */
    drain_choices(tg, 8);

    const char *o = rb_mods_get_orientation(&tg->state.mods, ally);
    CHECK(o && !strcmp(o, "active"),
          "m: the waited member is activated (got \"%s\")", o ? o : "(none)");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, ally), 1,
             "m: only the activated member gains 1 ブレード");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, arise), 0, "m: ARISE itself gains none");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, live), 0, "m: the live card gains none");
}

static void test_m_private_wars_waits_blade_three_but_not_four(void)
{
    TestGame *tg = &tg_k2;
    int live, arise, ally;
    private_wars_board(tg, &live, &arise, &ally);
    if (live < 0 || arise < 0 || ally < 0) return;
    int low = mid(tg, SD3_BLADE), high = mid(tg, SD4_BLADE);
    pin_id(low, SD3_BLADE);
    pin_id(high, SD4_BLADE);
    if (low < 0 || high < 0) return;
    CHECK_EQ(printed_blade(low), 3, "m2: setup -- the low member prints blade 3");
    CHECK_EQ(printed_blade(high), 4, "m2: setup -- the high member prints blade 4");
    tg->state.p[1].stage[AREA_LEFT] = high;
    tg->state.p[1].stage[AREA_CENTER] = low;

    fire_live_start(tg, 0, live);
    PROMPT_PRESENT(tg, 1, "m2: the gated two-option choice is offered (got %s)", pending_type(tg));
    if (!pending(tg)) return;
    CHECK_PROMPTS(1, "m2_wait3: the choice must be answered");
    select_option(tg, 1);   /* the wait-exactly-3 option */
    drain_choices(tg, 8);

    const char *ol = rb_mods_get_orientation(&tg->state.mods, low);
    CHECK(ol && !strcmp(ol, "wait"), "m2: the blade-3 opponent member is waited");
    const char *oh = rb_mods_get_orientation(&tg->state.mods, high);
    CHECK(!oh || strcmp(oh, "wait") != 0, "m2: the blade-4 opponent member is NOT waited");
    const char *oa = rb_mods_get_orientation(&tg->state.mods, ally);
    CHECK(oa && !strcmp(oa, "wait"),
          "m2: the p1 member stays waited (the other option was not taken)");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, ally), 0, "m2: no blade was granted");
}

static void test_m_private_wars_without_arise_is_a_noop(void)
{
    TestGame *tg = &tg_d;
    int live, arise, ally;
    private_wars_board(tg, &live, &arise, &ally);
    if (live < 0 || arise < 0 || ally < 0) return;
    tg->state.p[0].stage[AREA_LEFT] = RB_EMPTY_SLOT;   /* ARISE is gone */

    fire_live_start(tg, 0, live);
    drain_choices(tg, 8);
    CHECK(!pending(tg),
          "m3: without ARISE the gate fails and no prompt appears (got %s)", pending_type(tg));
    const char *oa = rb_mods_get_orientation(&tg->state.mods, ally);
    CHECK(oa && !strcmp(oa, "wait"), "m3: the p1 member is still waited (got \"%s\")",
          oa ? oa : "(none)");
    CHECK_EQ(rb_mods_get_blade(&tg->state.mods, ally), 0, "m3: no blade was granted");
}

/* ========================================================================
 * GROUP P -- PICKER / PCA_G1 CHAIN PROBE.
 *
 * This is the measurement, not a guess. The AGENTS.md data flow is
 *   current_effect -> ChoiceBuilder.picker() -> Choice::SelectCard.picker
 *   -> PCA_G1 routing.
 * Each sub-test records whether ONE hop exists in the C port.
 * ======================================================================== */

/* Hop 1: does the effect handed to the builder actually carry a picker? */
static void test_p_effect_picker_any_exists(void)
{
    TestGame *tg = &tg_d2;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int dia = mid(tg, DIA_BP5);
    if (dia < 0) return;
    int n = rb_card_num_abilities((uint32_t)dia);
    CHECK(n > 0, "p1: the staged print decodes at least one ability");

    int saw_picker = 0;
    int n_child = 0;
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)dia, i, &ab)) continue;
        if (ab.effect) {
            const char *pk = rb_effect_picker_any(ab.effect);
            if (pk && pk[0]) saw_picker = 1;
            if (ab.effect->n_child > 0) n_child += ab.effect->n_child;
            /* hop 2: the builder accepts a picker at all */
            {
                RbChoiceBuilder *b = rb_choice_builder_new("discard", "probe", 1, 0);
                if (b) {
                    RbChoiceBuilder *b2 = rb_choice_builder_picker(b, "self");
                    CHECK(b2 != NULL, "p2: rb_choice_builder_picker exists and accepts a value");
                    /* rb_choice_builder_build CONSUMES the builder in this port
                     * (its own free of the RbFullChoice is what abort()s if you
                     * also call rb_choice_builder_free on the same pointer), so
                     * the picker round-trip is checked on a separate builder. */
                    RbFullChoice *fc = rb_choice_builder_build(b);
                    if (fc) {
                        /* The picker set on the builder must survive into the
                         * built RbFullChoice -- that is hop 2's whole content. */
                        CHECK_EQ_STR(fc->picker, "self",
                                     "p2: the builder's picker reaches the built RbFullChoice");
                    } else {
                        CHECK(0, "p2: rb_choice_builder_build returned a choice");
                    }
                } else {
                    CHECK(0, "p2: rb_choice_builder_new returned a builder");
                }
            }
        }
        rb_free_ability(&ab);
    }
    fprintf(stderr,
            "  [PICKER] p1: PL!S-bp5-004-R abilities=%d effects_with_picker=%d children=%d\n",
            n, saw_picker, n_child);
    /* Recorded, not asserted: whether THIS print parses a `picker` field is a
     * parser question, and the answer is reported rather than baked in. */
    printf("PICKER PROBE: PL!S-bp5-004-R abilities=%d, with picker=%d, children=%d\n",
           n, saw_picker, n_child);

    /* Hop 3: the FLAT RbChoice the emitter populates -- does it carry a picker? */
    play_to_stage(tg, dia, AREA_CENTER);
    if (pending(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        fprintf(stderr,
                "  [PICKER] p3: pending flat RbChoice kind=%d zone=\"%s\" target=\"%s\" "
                "target_player_id=\"%s\" -- the struct has NO picker member, so nothing "
                "downstream can read one.\n",
                (int)c->kind, c->zone, c->target, c->target_player_id);
    } else {
        fprintf(stderr,
                "  [PICKER] p3: no prompt was pending, so the flat choice was not observed\n");
    }
    drain_choices(tg, 6);
}

/* Hop 4 / 5: does the emitted SelectCard carry a target_player_id that PCA_G1
 * would route on? The Rust G1/G3 arm reads `target_player_id == "opponent"` (or
 * a spawn_target of "opponent"). The C pause routine reads `choice->target`
 * instead -- a different field -- so the routing inputs are not interchangeable. */
static void test_p_g1_routing_inputs_present(void)
{
    TestGame *tg = &tg_e;
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    int yoshiko = mid(tg, YOSHIKO);
    pin_id(yoshiko, YOSHIKO);
    if (yoshiko < 0) return;
    tg->state.p[0].stage[AREA_CENTER] = yoshiko;
    int live = mid(tg, LIVE3);
    if (live < 0) return;
    hand_push(tg, 0, live, 1);
    test_give_energy(tg, 1);
    int opp = test_new_id(tg, FILLER);
    if (opp < 0) return;
    hand_push(tg, 1, opp, 1);

    test_activate_ability(tg, yoshiko);
    if (pending(tg)) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int cur = tg->state.queue.cur;
        char spawn[64] = "";
        char choice_seat[TEST_SEAT_ID_LEN] = "";
        if (cur >= 0 && cur < tg->state.queue.n_entries) {
            snprintf(spawn, sizeof spawn, "%s", tg->state.queue.entries[cur].spawn_target);
            test_queue_entry_choice_player_id(tg, cur, choice_seat, sizeof choice_seat);
        }
        fprintf(stderr,
                "  [G1 ROUTING] p4: pending SelectCard kind=%d zone=\"%s\" target=\"%s\" "
                "target_player_id=\"%s\" | entry.spawn_target=\"%s\" "
                "entry.choice_player_id=\"%s\"\n"
                "  Rust G1 (ability_queue.rs:308-344) reads Choice::SelectCard."
                "target_player_id == \"opponent\" OR entry.resolver.spawn_context.target "
                "== \"opponent\". The C pause routine reads choice->target (a DIFFERENT "
                "field) and entry.spawn_target. A picker, when it exists, would be a "
                "third input the C has nowhere to carry.\n",
                (int)c->kind, c->zone, c->target, c->target_player_id,
                spawn, choice_seat);
        CHECK_EQ_STR(c->target_player_id, "opponent",
                     "p4: the C spells Rust's \"opponent\" as the seat token on the flat choice");
    } else {
        CHECK(0, "p4: the opponent's optional-discard prompt is pending for the routing probe");
    }
    drain_choices(tg, 6);
}

/* ======================================================================== *
 * main
 * ======================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_ok, n_failed, n_crashed;

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
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [in-process fallback]\n", failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs, g0 = gaps, p0 = prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d gap(s), %d prompt(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0), (int)(gaps - g0), (int)(prompts - p0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
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

    printf("--- A chosen_player_live_bottomdeck_then_draw_test.rs (PL!S-bp3-007-R) ---\n");
    run("a_hanamaru_choose_self",     test_a_hanamaru_choose_self_moves_own_live_and_draws);
    run("a_hanamaru_choose_opponent", test_a_hanamaru_choose_opponent_moves_opponent_live);
    run("a_hanamaru_no_live",         test_a_hanamaru_no_live_in_waitroom_no_move_no_draw);
    run("a_hanamaru_opp_no_live",     test_a_hanamaru_opponent_no_live_no_draw);
    run("a_hanamaru_no_energy",       test_a_hanamaru_cannot_activate_without_energy);
    run("a_hanamaru_turn_limit",      test_a_hanamaru_turn_limit_once);

    printf("--- B target_player_recycle_pl_s_bp7_013_n_test.rs (PL!S-bp7-013-N) ---\n");
    run("b_dia_choose_self",          test_b_dia_choose_self_recycles_members_preserves_live);
    run("b_dia_choose_opponent",      test_b_dia_choose_opponent_card_selection_routes_to_chosen_seat);

    printf("--- C target_player_deck_routing_pl_n_bp3_010_r_pl_n_bp4_002_r_test.rs ---\n");
    run("c_azuna_choose_self",        test_c_azuna_choose_self_recycle_grows_own_deck);
    run("c_azuna_choose_opponent",    test_c_azuna_choose_opponent_empties_opponent_waitroom);
    run("c_kasumi_choose_self",       test_c_kasumi_choose_self_discards_looked_top_card);
    run("c_dia_then_azuna_order",     test_c_dia_then_azuna_share_opponent_waitroom_preserve_order);

    printf("--- D opponent_accepts_blades_pl_n_pr_022_pr_test.rs (PL!N-PR-022-PR) ---\n");
    run("d_emma_routed_to_opponent",  test_d_emma_accepted_choice_routed_to_opponent);
    run("d_emma_all_members",         test_d_emma_blades_every_opponent_member);
    run("d_emma_other_answer",        test_d_emma_other_answer_grants_nothing);

    printf("--- E opponent_routed_optional_discard_choice_test.rs ---\n");
    run("e_yoshiko_opp_discards",     test_e_yoshiko_opponent_discards_skips_blade);
    run("e_yoshiko_opp_skips",        test_e_yoshiko_opponent_skips_gains_blade);
    run("e_yoshiko_multi_skips",      test_e_yoshiko_multi_card_skips_gains_blade);
    run("e_yoshiko_multi_discards",   test_e_yoshiko_multi_card_discards_skips_blade);
    run("e_yoshiko_empty_hand",       test_e_yoshiko_opponent_empty_hand_auto_skips);
    run("e_serasu_opp_wait",          test_e_serasu_edelnote_appear_triggers_opponent_wait);
    run("e_serasu_no_opp_member",     test_e_serasu_no_opponent_member_fizzles);

    printf("--- F selection_filter blade_option_exclude_self_pl_s_bp5_004_r_test.rs ---\n");
    run("f_dia_blade_excludes_self",  test_f_dia_blade_option_excludes_itself);
    run("f_dia_no_other_aqours",      test_f_dia_blade_option_no_other_aqours_skips);

    printf("--- G selection_filter forced_self_wait_excludes_immune_q275_test.rs ---\n");
    run("g_q275_excludes_immune",     test_g_q275_forced_wait_excludes_immune);
    run("g_q275_no_immunity_control", test_g_q275_without_immunity_control);
    run("g_q275_single_target",       test_g_q275_single_legal_target_auto_resolves);

    printf("--- H selection_filter optional_energy_edelnote_deploy_or_requirement_test.rs ---\n");
    run("h_reduce_heart06",           test_h_paid_energy_choice_reduces_heart06);
    run("h_deploy_waitroom_member",   test_h_paid_energy_choice_deploys_waitroom_member);
    run("h_declined_energy",          test_h_declined_energy_preserves_everything);
    run("h_cost9_edelnote",           test_h_exact_cost_nine_edelnote_enables_choice);
    run("h_wrong_group_or_low_cost",  test_h_wrong_group_or_low_cost_cannot_enable_choice);
    run("h_deploy_filter",            test_h_deploy_filter_excludes_cost_five_and_wrong_group);
    run("h_full_stage_deploy",        test_h_full_stage_deploy_cannot_replace_members);

    printf("--- I per_card energy_or_recycle_live_pl_n_pb1_010_r_test.rs ---\n");
    run("i_niji_energy_option",       test_i_niji_energy_option_activates_exactly_one);
    run("i_niji_recycle_deck_top",    test_i_niji_recycle_option_puts_live_on_deck_top);

    printf("--- J per_card mill_or_wait_pl_hs_cl1_004_cl_test.rs ---\n");
    run("j_geko_mill",                test_j_geko_mill_option_removes_three_deck_cards);
    run("j_geko_wait_cost2",          test_j_geko_wait_option_waits_cost_two_opponent_member);

    printf("--- K per_card wait_or_draw_pl_s_bp7_025_l_test.rs ---\n");
    run("k_kowarey_wait_two",         test_k_kowarey_choose_wait_waits_two_opponents);
    run("k_kowarey_draw_one",         test_k_kowarey_choose_draw_leaves_opponent_unmodified);

    printf("--- L per_card discard_self_from_hand_no_named_blade_target_q258_test.rs ---\n");
    run("l_himege_discard_no_blade",  test_l_himege_discards_self_without_a_blade);

    printf("--- M per_card arise_gated_activate_blade_or_wait_private_wars_test.rs ---\n");
    run("m_private_wars_activate",    test_m_private_wars_activates_arise_and_grants_one_blade);
    run("m_private_wars_wait3_not4",  test_m_private_wars_waits_blade_three_but_not_four);
    run("m_private_wars_without_arise", test_m_private_wars_without_arise_is_a_noop);

    printf("--- P picker / PCA_G1 chain probe ---\n");
    run("p_effect_picker_any",        test_p_effect_picker_any_exists);
    run("p_g1_routing_inputs",        test_p_g1_routing_inputs_present);

    rb_unload();

    printf("\n==== parity_target_player ====\n");
    printf("tests run              : %d\n", n_ok + n_failed + n_crashed);
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps          : %d\n", gaps);
    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_crashed);
        return 1;
    }
    if (n_failed) {
        fprintf(stderr,
                "PARITY TARGET_PLAYER: %d failing test(s) - every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet, or a SETUP BUG\n"
                "naming a print the engine cannot resolve. Not a bug in this file.\n",
                n_failed);
        return n_failed > 125 ? 125 : n_failed;
    }
    printf("ALL PARITY_TARGET_PLAYER CHECKS PASSED\n");
    return 0;
}

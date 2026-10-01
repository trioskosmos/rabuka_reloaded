/* test_parity_recover_score_state_delta.c -- the UNTRANSLATED DELTA of the
 * three biggest Rust effects families: effects/recover/, effects/score/ and
 * effects/state/.
 *
 * ── WHY THIS FILE IS SMALLER THAN THE CLUSTER SIZES ─────────────────────────
 * The brief scoped this file to "every remaining subcluster not listed as
 * already covered". Measuring that delta cluster by cluster (Rust `#[test] fn`
 * names enumerated, then each name grepped against every existing C suite)
 * leaves exactly TWO unowned subclusters, and one of the two is already done:
 *
 *   effects/recover/to_discard/       21 Rust tests -> 21 already in
 *                                     test_parity_small_clusters.c SECTION B
 *                                     (b1_*, b2_*, b3_*, b4_*). NOT REPEATED.
 *   effects/state/orientation_gated/  42 Rust tests -> 21 already in
 *                                     test_parity_small_clusters.c SECTION C
 *                                     (c1_* erena_self_wait, c2_* heart06,
 *                                     c3_* wait_state_condition, c4_* nico).
 *                                     21 REMAIN. Those 21 are this file.
 *
 * The 21 remaining are ALL from one Rust file, and the file's own doc comment
 * explains why: "All 21 other tree references to Erena use her as a *fixture*
 * for someone else's 常時 ... the **登場 half was executed by nothing**."
 * The whole delta is the debut half of 統堂英玲奈, plus the two Nico Q180
 * phase-advance cases the existing C4 section does not separate.
 *
 *   erena_debut_cost_nine_or_less_opponent_wait_test.rs   20 tests -> §A
 *   nico_cannot_activate_test.rs                          1 test  -> §B
 *                                                       (nico_p1_active_phase
 *                                                        _still_activates;
 *                                                        the C section covers
 *                                                        nico_active_phase_
 *                                                        still_activates)
 *
 * ── WHAT THIS CLUSTER IS ACTUALLY ABOUT ──────────────────────────────────────
 * 統堂英玲奈 `PL!-bp5-333-R` (cost 7):
 *   登場 このメンバーをウェイトにしてもよい：相手のステージにいるコスト9以下の
 *       メンバー1人をウェイトにする。
 *   常時 このメンバーがウェイト状態であるかぎり、heart05 を得る。
 *
 * It is a cost-then-effect pair, and the cost IS her own orientation. Three
 * separate mechanisms are under test and they are the reason this file exists:
 *
 *  1. THE COST-9 BOUND. 「コスト9以下」 is `<= 9`, inclusive. A cost-9 member
 *     qualifies, a cost-10 member does not. The Rust twin pins the OPERATOR by
 *     putting a cost-4 member on the board too, because `=` , `<` and `>=` all
 *     predict the SAME offered set from {cost 9, cost 10} alone; only the
 *     cost-4 member separates the four parses. §A2 reproduces that four-way
 *     discrimination.
 *
 *  2. THE CANDIDATE SET IS filtered_indices, AND IT IS NOT ALWAYS PROMPTED.
 *     `execute_change_state` prompts only when `candidates.len() > count`
 *     (change_state.rs:504). With ONE qualifying member and count == 1 the
 *     effect AUTO-RESOLVES and no member prompt ever opens. So "read the
 *     offered set" is only observable on a board with 2+ qualifying members,
 *     while the bound itself is observable anywhere as an OUTCOME. Both halves
 *     are ported, and the choice_picker test (§A13) is the one place the
 *     prompt's own candidate list is the thing under test.
 *
 *  3. 登場 IS SCOPED TO THE ENTRY EVENT. `use_limit` is null, so a member that
 *     leaves the stage and returns debuts AGAIN and re-offers the pay/skip
 *     gate (§A16). Every other orientation test in the tree writes the
 *     orientation by hand, so this re-entry path was executed by nothing.
 *
 * ── THE CONSTANT OWNERSHIP CLAUSE ───────────────────────────────────────────
 * `the_constant_condition_follows_the_card_not_the_resolving_player` (§A11) is
 * the only place the 常時 is asserted across BOTH seats at once. Our copy
 * active, opponent's copy waited: the OPPONENT must get the heart05. A
 * single-owner test cannot distinguish "evaluated for the constant's own
 * controller" from "evaluated against gs.activating_card", because with one
 * Erena both readings agree. Two copies, one per side, is the only board where
 * they diverge.
 *
 * ── NON-VACUITY ─────────────────────────────────────────────────────────────
 * Nearly every case here is a NEGATIVE ("not waited", "no heart05", "grants
 * nothing") and a negative passes trivially on a board where the debut never
 * ran. Three controls are used, labelled CONTROL in the assertion text:
 *   (a) a prompt-shape control -- the 登場 must open its own
 *       `SelectTarget` pay/skip gate before anything is asserted;
 *   (b) a paired POSITIVE sibling on the same card, where the same assertion
 *       is inverted (pay vs decline, two candidates vs one);
 *   (c) a board-content control -- the wait is an ORIENTATION, so the stage
 *       array must be unchanged; a "waited" result achieved by MOVING the card
 *       out would pass an orientation check and be wrong.
 *
 * ── THE PAY/SKIP GATE ───────────────────────────────────────────────────────
 * `pay_optional_cost:skip_optional_cost` is an options-less `SelectTarget`
 * whose option 0 is SKIP and option 1 is PAY. It is a DIFFERENT variant from
 * the member `SelectCard` that follows it, and the two are easy to confuse, so
 * every case asserts the gate's own kind before answering it. `a_pay()` answers
 * 1 on a SelectTarget and 0 elsewhere; `a_decline()` answers 0.
 *
 * ── C-vs-Rust SURFACE GAPS, reported not papered over ───────────────────────
 * EXPECTED_GAP marks a Rust assertion that the C shim cannot express at all.
 * These are counted and printed separately and are NEVER reported as passes:
 *   * `wait_immune_members` does not exist in the C GameState at all (no
 *     `immune` symbol anywhere in include/ or src/), so §A17's immunity
 *     suppression has nothing to install and the case is reduced to the
 *     PROMPT-SHAPE half that is expressible, with the outcome half recorded as
 *     a gap. See the report: this is a real engine-surface gap, not a test bug.
 *   * `mods.get_orientation_modifier` returns Option; C's `rb_mods_get_orientation`
 *     returns NULL for absent, so "Some(active)" and None are the same read.
 *     Cases that need the DISTINCTION use `is_waited` (a positive test), never
 *     the negative, because a NULL and a "active" both read as not-waited.
 *
 * ── MECHANICS ───────────────────────────────────────────────────────────────
 *  * sizeof(GameState) is ~781 KB, so every TestGame is `static`.
 *  * test_new_id() allocates a distinct instance (Rust's `game.new_id`).
 *    test_id() aliases a shared template, which would make two copies of one
 *    print indistinguishable and every copy-identity assertion vacuous.
 *  * Every fixture is pinned with a byte-exact strcmp of the DECODED card_no
 *    (pin_id) because rb_card_no_eq is FUZZY and resolves a mistyped print to
 *    some other real card instead of failing.
 *  * rb_card_copy_pool_reset() between cases: the pool is a process-global
 *    monotonic counter that runs dry after ~1457 allocations, after which
 *    test_new_id returns -1 and every copy-identity assertion silently passes.
 *  * Fullwidth characters are \xNN escapes (AGENTS.md encoding trap).
 *  * Every case runs in a forked child; a signal is reported as CRASH, never
 *    as a pass, and the handler names the test in flight.
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
   fullwidth form, so "PL!-bp5-333-P＋" is a DIFFERENT card number from
   "PL!-bp5-333-P". Explicit byte escape, per AGENTS.md. */
#define FF_PLUS "\xef\xbc\x8b"

/* heart colour slots, matching cards/compile_cards.py HEART_COLORS:
   0=heart00 .. 6=heart06, 7=icon_all. test_get_heart_modifier is a plain
   passthrough into rb_mods_get_heart's raw 0..7 slot. */
#define H05 5
#define H06 6

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

/* A C-vs-Rust SURFACE gap: the Rust assertion is not expressible through the C
   shim at all (missing API surface, not a behaviour difference). Counted and
   printed apart from parity failures, so a red run still names the parity
   failures. NEVER counted as a pass. */
#define EXPECTED_GAP(condition, desc) do { \
    assertions++; \
    if (!(condition)) { \
        printf("HARNESS GAP: %s\n", desc); \
        harness_gaps++; \
    } else { \
        printf("ok (harness gap closed): %s\n", desc); \
    } \
} while (0)

/* ======================================================================= */
/* card constants -- verified byte-exactly by pin_id in every case         */
/* ======================================================================= */

#define ERENA_R   "PL!-bp5-333-R"          /* 統堂英玲奈 A-RISE, cost 7     */
#define ERENA_PP  "PL!-bp5-333-P" FF_PLUS   /* the reprint, same ability text */
#define COST_9    "PL!-bp5-017-N"          /* 小泉花陽, cost 9, the bound   */
#define COST_10   "PL!-bp5-005-AR"         /* 星空凛, cost 10, one above    */
#define COST_4    "PL!-sd1-010-SD"         /* cost 4, a second qualifier    */
#define IZUMI_N   "PL!HS-bp5-016-N"        /* 桂城 泉, 2+ opponent waiters  */
#define KANATA_P  "PL!N-bp3-006-P"         /* 近江彼方, cost 9, self-wait 登場 */
#define NICO      "PL!-pb1-009-R"          /* 矢澤にこ, cannot_activate aura */
#define SD_010    COST_4

/* ======================================================================= */
/* harness                                                                  */
/* ======================================================================= */

/* rb_load("src") fails under an isolated out-of-tree build root, so the
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
   fprintf, and one static buffer would print the LAST value three times. */
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

/* The identity rule (AGENTS.md): rb_card_no_eq is FUZZY, so every fixture is
   pinned with a byte-exact strcmp of the DECODED card_no. */
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
   lookup aliases, so test_new_id is the faithful mapping. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    assertions++;
    if (id < 0) {
        fprintf(stderr, "FAIL: fixture card \"%s\" is not in the database "
                        "(or the copy pool is dry)\n", no);
        failures++;
    }
    return id;
}

/* A fresh board for one case: the copy pool is reset and every zone of both
   seats is emptied, so no assertion can read another case's residue. */
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
    tg->state.n_cannot_active_cards = 0;
}

/* ── board helpers ────────────────────────────────────────────────────────── */

static void set_stage(TestGame *tg, int pl, int a, int b, int c)
{
    RbPlayer *P = &tg->state.p[pl];
    P->stage[0] = a; P->stage[1] = b; P->stage[2] = c;
    for (int i = 0; i < RB_STAGE_SIZE; i++) P->stage_wait[i] = 0;
}

static void fill_decks(TestGame *tg, int filler, int per_player)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < per_player && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

static int heart_of(TestGame *tg, int cid, int slot)
{
    return rb_mods_get_heart(&tg->state.mods, cid, slot);
}

/* 「ウェイト状態であるか」 -- a POSITIVE test. The Rust twin's
   `get_orientation_modifier(id) == Some("wait")`; C returns NULL for absent and
   a non-NULL string otherwise, and "active" is also non-NULL but not "wait".
   Never used in the negative direction, because absent and "active" are the
   same read in C. */
static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && !strcmp(o, "wait");
}

static int printed_cost(int cid)
{
    Card c; int s = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) { s = c.cost; rb_free_card(&c); }
    return s;
}

static int n_abilities(int cid)
{
    return (cid >= 0) ? rb_card_num_abilities((uint32_t)cid) : -1;
}

/* One-line report of every ability a card decodes to, so a data gap names
   itself instead of surfacing as a behavioural gap with the wrong cause. */
static void dump_abilities(int cid, const char *tag)
{
    if (cid < 0) return;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return;
    int n = n_abilities(cid);
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

/* ── choice plumbing ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }
static const char *pending_kind(TestGame *tg) { return test_pending_choice_type(tg); }

static void skip_pick(TestGame *tg)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, -1);
}

/* PAY the optional self-wait gate: index 1 on a SelectTarget, 0 elsewhere
   (a card pick). Answers nothing else with a skip. Returns the number of
   prompts driven, which is the non-vacuity counter. */
static int a_pay(TestGame *tg, int guard_max)
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

/* DECLINE the optional self-wait gate: index 0 on a SelectTarget. */
static int a_decline(TestGame *tg, int guard_max)
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

/* Rust's select_indices(&[..]) -- the member-card answer path. */
static void select_indices(TestGame *tg, const int *idxs, int n)
{
    if (!pending(tg)) return;
    if (n == 0) rb_resume_with_choice(&tg->state, -1);
    else rb_resume_with_choice_indices(&tg->state, idxs, n);
}

/* Rust drain_choices(): answer what cannot be declined, decline what can. */
static int a_drain(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) { skip_pick(tg); n++; }
    return n;
}

/* Diagnostic: dump the pending prompt's shape, so a red check is classified
   from evidence rather than guessed at. */
static void describe_pending(TestGame *tg, const char *tag)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) { fprintf(stderr, "  (%s) no pending choice\n", tag); return; }
    fprintf(stderr, "  (%s) kind=%s zone=%s target=%s tpid=%s count=%d "
                    "allow_skip=%d route=%d n_filtered=%d desc=%s\n",
            tag, test_pending_choice_type(tg), c->zone, c->target, c->target_player_id,
            c->count, c->allow_skip, (int)c->route, c->n_filtered_indices,
            c->description);
}

/* The stage POSITIONS the pending member prompt actually OFFERS, resolved to
   card ids against the TARGET seat.
 *
 * `filtered_indices` is the candidate set; `selection_cards` is not (it lists
 * the whole zone so the UI can grey the rest), and the indices are STAGE
 * POSITIONS, not card ids. This is the faithful read of the Rust
 * `offered_candidate_ids`.
 *
 * Returns the number of offered ids written into `out`, or -1 if the standing
 * prompt is not the member SelectCard. */
static int offered_candidate_ids(TestGame *tg, int *out, int max)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return -1;
    if (c->kind != RB_CHOICE_SELECT_CARD) return -1;
    int pl = (c->target_player_id[0] == 'p' && c->target_player_id[1] == '2') ? 1 : 0;
    int n = 0;
    for (int i = 0; i < c->n_filtered_indices && n < max; i++) {
        int pos = c->filtered_indices[i];
        if (pos < 0 || pos >= RB_STAGE_SIZE) continue;
        out[n++] = tg->state.p[pl].stage[pos];
    }
    return n;
}

/* Does the standing prompt offer `cid`? The Rust `offered.contains(&m9)`. */
static int offer_contains(TestGame *tg, int cid)
{
    int buf[8];
    int n = offered_candidate_ids(tg, buf, 8);
    if (n < 0) return 0;
    for (int i = 0; i < n; i++) if (buf[i] == cid) return 1;
    return 0;
}

/* How many of `cards` are waited. */
static int n_waited(TestGame *tg, const int *cards, int n)
{
    int k = 0;
    for (int i = 0; i < n; i++) if (is_waited(tg, cards[i])) k++;
    return k;
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION A -- state/orientation_gated/
 *           erena_debut_cost_nine_or_less_opponent_wait_test.rs (20 #[test])
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust debuted_with_opponent_stage: P2 gets `opponent`, P1's stage is EMPTY
   (the centre slot is reserved for Erena's debut), both decks get 30
   COST_4 filler, energy 15, then Erena is played to the centre -- leaving the
   caller's chain sitting on the pay/skip gate.
 *
 * Returns the Erena id, or -1 if a fixture failed to pin. */
static int erena_debut(TestGame *tg, int a, int b, int c)
{
    int erena = mid(tg, ERENA_R);
    int filler = mid(tg, COST_4);
    if (!pin_id(erena, ERENA_R) || !pin_id(filler, COST_4)) return -1;

    set_stage(tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(tg, 1, a, b, c);
    fill_decks(tg, filler, 30);
    test_give_energy(tg, 15);

    test_add_to_hand(tg, erena);
    if (test_play_to_stage(tg, erena, 1) != 1) {
        fprintf(stderr, "FAIL: Erena's debut to the centre was refused\n");
        failures++;
        return -1;
    }
    if (!pending(tg)) {
        fprintf(stderr, "FAIL: the 登場 opened no prompt at all -- the whole "
                        "cluster is measuring a dead ability\n");
        failures++;
        return -1;
    }
    if (pending_kind(tg) && strcmp(pending_kind(tg), "SelectTarget") != 0) {
        fprintf(stderr, "FAIL: expected the SelectTarget pay/skip gate, got '%s'\n",
                pending_kind(tg));
        describe_pending(tg, "debut gate");
        failures++;
        return -1;
    }
    return erena;
}

/* ── A0. the_two_boundary_fixtures_print_cost_nine_and_cost_ten ─────────────
 * The bound is only meaningful if the fixtures sit exactly on it. A
 * transposed print would otherwise leave the whole file testing a different
 * clause -- and every other assertion in §A reads a cost. */
static void a0_boundary_fixtures_print_cost_nine_and_cost_ten(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m10 = mid(&tg, COST_10);
    int m4  = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m10, COST_10) || !pin_id(m4, COST_4)) return;

    CHECK_EQ(printed_cost(m9), 9,  "A0: PL!-bp5-017-N really costs 9 (the bound)");
    CHECK_EQ(printed_cost(m10), 10, "A0: PL!-bp5-005-AR really costs 10 (one above)");
    CHECK_EQ(printed_cost(m4), 4,   "A0: PL!-sd1-010-SD really costs 4 (second qualifier)");
}

/* ── A1. paying_the_cost_waits_the_cost_nine_member_and_not_the_cost_ten ────
 * The bound is INCLUSIVE. This is the OUTCOME read of the filter, which --
 * unlike the offered-set read -- needs only ONE qualifying member, because
 * with one candidate and count == 1 the effect AUTO-RESOLVES (no prompt).
 *
 * Also asserts the negative-control structure: BOTH opponent members must
 * still be standing on the stage afterwards. ウェイト is an orientation, not a
 * move, so a result achieved by relocating the card would pass the waited
 * check and be wrong. */
static void a1_pay_waits_cost_nine_not_cost_ten(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m10 = mid(&tg, COST_10);
    if (!pin_id(m9, COST_9) || !pin_id(m10, COST_10)) return;

    int erena = erena_debut(&tg, m9, m10, RB_EMPTY_SLOT);
    if (erena < 0) return;
    CHECK_EQ(tg.state.p[1].stage[0], m9,
             "A1 CONTROL: the cost-9 member really is at opponent slot 0");
    CHECK_EQ(tg.state.p[1].stage[1], m10,
             "A1 CONTROL: the cost-10 member really is at opponent slot 1");
    CHECK(printed_cost(m9) <= 9 && printed_cost(m10) > 9,
          "A1 CONTROL: the two fixtures really straddle the bound");

    int n = a_pay(&tg, 12);
    fprintf(stderr, "  (A1) prompts driven = %d\n", n);

    CHECK(is_waited(&tg, erena),
          "A1: paying the optional cost must put Erena HERSELF to wait -- the cost IS "
          "her own orientation");
    CHECK(is_waited(&tg, m9),
          "A1: 「コスト9以下のメンバー」 -- a cost-9 opponent member must be waited");
    CHECK(!is_waited(&tg, m10),
          "A1: a cost-10 opponent member must NOT be waited. Waiting it means the bound "
          "is exclusive, or has been read as a minimum (>= 9)");
    CHECK_EQ(tg.state.p[1].stage[0], m9,
             "A1: neither opponent member may LEAVE the stage -- ウェイト is an "
             "orientation, not a move");
    CHECK_EQ(tg.state.p[1].stage[1], m10,
             "A1: the cost-10 member is still at its slot, untouched");
}

/* ── A2. the_offered_candidate_set_excludes_the_cost_ten_member ────────────
 * The offered SET, read at the prompt. Needs TWO qualifying members so that
 * `candidates.len() > count` holds and the prompt opens.
 *
 * THE COST-4 MEMBER IS WHAT PINS THE OPERATOR, and it is the point of the
 * test. Each mis-parse of 「コスト9以下」 predicts a DIFFERENT offered set:
 *
 *   | parsed operator | offered              |
 *   |-----------------|----------------------|
 *   | `<=`  (correct) | [m9, m4]             |
 *   | `=`             | [m9]                 |
 *   | `<`             | [m4]                 |
 *   | `>=`            | [m9, m10]            |
 *
 * A board of only {cost 9, cost 10} would pass under three of those four, and
 * would then look like coverage of the operator when it is coverage of
 * nothing. */
static void a2_offered_set_excludes_cost_ten(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m4  = mid(&tg, COST_4);
    int m10 = mid(&tg, COST_10);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4) || !pin_id(m10, COST_10)) return;

    /* m9 at slot 0, m4 at slot 1, m10 at slot 2. */
    int erena = erena_debut(&tg, m9, m4, m10);
    if (erena < 0) return;
    int prompts = a_pay(&tg, 12);
    fprintf(stderr, "  (A2) prompts driven = %d\n", prompts);

    CHECK(pending(&tg),
          "A2 CONTROL: with two qualifying members the prompt really does open "
          "(change_state.rs:504 needs candidates.len() > count)");
    if (!pending(&tg)) { describe_pending(&tg, "A2"); return; }
    if (pending_kind(&tg) && strcmp(pending_kind(&tg), "SelectCard") != 0) {
        fprintf(stderr, "FAIL: A2 expected the member SelectCard, got '%s'\n",
                pending_kind(&tg));
        describe_pending(&tg, "A2");
        failures++; assertions++;
        return;
    }

    int offered[8];
    int n = offered_candidate_ids(&tg, offered, 8);
    fprintf(stderr, "  (A2) offered %d candidate(s)\n", n);
    CHECK_EQ(n, 2, "A2: the prompt offers exactly TWO candidates, not three");
    if (n == 2) {
        CHECK_EQ(offered[0], m9, "A2: candidate 0 is the cost-9 member at slot 0");
        CHECK_EQ(offered[1], m4, "A2: candidate 1 is the cost-4 member at slot 1");
    }
    CHECK(!offer_contains(&tg, m10),
          "A2: 「コスト9以下のメンバー」 -- the cost-10 member must NOT be offered, "
          "even though it is a legal stage member sitting right there");

    /* Answer it, and check the choice landed on the card that was offered. */
    int zero[1] = { 0 };
    select_indices(&tg, zero, 1);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena), "A2: Erena paid her own orientation");
    CHECK(is_waited(&tg, m9),
          "A2: selecting candidate 0 must wait the cost-9 member at stage slot 0");
    CHECK(!is_waited(&tg, m4),
          "A2: the UNSELECTED cost-4 member must stay active -- the effect waits 1人, "
          "not all of them");
    CHECK(!is_waited(&tg, m10), "A2: the cost-10 member must stay active");
}

/* ── A3. exactly_one_member_is_waited_even_when_all_three_qualify ─────────
 * `count: 1` is a real limit, not a formality. */
static void a3_exactly_one_waited_of_three_qualify(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m4  = mid(&tg, COST_4);
    int m4b = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4) || !pin_id(m4b, COST_4)) return;
    CHECK(m4 != m4b, "A3 fixture: the two COST_4 cards are DISTINCT instances");

    int erena = erena_debut(&tg, m9, m4, m4b);
    if (erena < 0) return;
    a_pay(&tg, 12);
    CHECK(pending(&tg), "A3 CONTROL: three candidates, so the prompt opens");
    if (!pending(&tg)) { describe_pending(&tg, "A3"); return; }

    int offered[8];
    CHECK_EQ(offered_candidate_ids(&tg, offered, 8), 3,
             "A3 CONTROL: all three are コスト9以下, so all three are offered");

    int one[1] = { 1 };              /* candidate index 1 == stage slot 1 */
    select_indices(&tg, one, 1);
    a_drain(&tg, 12);

    const int trio[3] = { m9, m4, m4b };
    CHECK_EQ(n_waited(&tg, trio, 3), 1,
             "A3: 「メンバー1人」 -- exactly ONE of three qualifying members is waited");
    CHECK(is_waited(&tg, m4),
          "A3: candidate index 1 is stage slot 1, the first cost-4 copy");
    CHECK(is_waited(&tg, erena), "A3: Erena paid her own orientation too");
}

/* ── A4. an_already_waited_member_is_not_offered_and_the_rest_resolves ────
 * Q137: an opponent member already in wait is not a legal 「ウェイトにする」
 * target. This also pins the AUTO-RESOLVE branch from the other side: with one
 * candidate left, `candidates.len() > count` is false, so the engine must NOT
 * open a prompt and must still apply the wait. */
static void a4_already_waited_not_offered_auto_resolves(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9 = mid(&tg, COST_9);
    int m4 = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4)) return;

    int erena = erena_debut(&tg, m9, m4, RB_EMPTY_SLOT);
    if (erena < 0) return;
    /* Put the cost-4 member in wait BEFORE paying, so it drops out of the
       candidate set. */
    rb_mods_set_orientation(&tg.state.mods, m4, "wait");
    CHECK(is_waited(&tg, m4), "A4 premise: the cost-4 member is waited beforehand");

    a_pay(&tg, 12);
    fprintf(stderr, "  (A4) pending after pay = %d\n", pending(&tg));
    CHECK(!pending(&tg),
          "A4: with the cost-4 member already waited only one candidate remains, so "
          "candidates.len() > count is false and NO member prompt may open "
          "(change_state.rs:504)");
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena), "A4: Erena paid her own orientation");
    CHECK(is_waited(&tg, m9),
          "A4: the one remaining コスト9以下 member must be waited by the auto-resolve");
    CHECK(is_waited(&tg, m4),
          "A4: the already-waited member KEEPS its state and is not disturbed");
}

/* ── A5. declining_the_optional_self_wait_does_not_wait_the_opponent ───────
 * 「…てもよい：A。」 is one optional step gating a mandatory one. Declining
 * the self-wait must cancel the opponent wait too, or the player pays with
 * their own orientation for a wait they did not choose. */
static void a5_decline_does_not_wait_opponent(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9 = mid(&tg, COST_9);
    int m4 = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4)) return;

    int erena = erena_debut(&tg, m9, m4, RB_EMPTY_SLOT);
    if (erena < 0) return;
    /* Two qualifying members, so the chained effect WOULD prompt if it ran --
       that is what makes the "no opponent wait" below a cancellation and not
       merely a non-firing effect. */
    a_decline(&tg, 12);
    a_drain(&tg, 12);

    CHECK(!pending(&tg),
          "A5 CONTROL: the chain terminated -- a drain that stopped early would "
          "satisfy every absence assertion in this case");
    CHECK(!is_waited(&tg, erena),
          "A5: Erena herself must NOT be waited, since the optional step was declined");
    CHECK(!is_waited(&tg, m9),
          "A5: declining 「このメンバーをウェイトにしてもよい」 must cancel the chained "
          "opponent wait too -- otherwise an opponent member is waited for a choice "
          "the player refused");
    CHECK(!is_waited(&tg, m4), "A5: no opponent member may be waited after a decline");
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "A5: Erena is active after a decline, so her 常時 grants no heart05 -- the "
             "decline must not leave her waited-but-unrecalculated");
}

/* ── A6. a_cost_ten_only_opponent_stage_waits_nobody ───────────────────────
 * Nothing on the opponent's stage is コスト9以下, so the cost is still payable
 * (it is optional and the player chose it) and the effect then has no target.
 *
 * THIS TEST EXISTS BECAUSE OF AN ENGINE DEFECT. Paying puts Erena to wait, so
 * her own 常時 must grant heart05 -- but `execute_change_state` used to return
 * early on the empty candidate list WITHOUT calling recalculate_constants(),
 * which every other exit from that function does. So she was in wait and
 * granted nothing. Only PL!-bp5-333-R and its P＋ in the entire pool BOTH pay
 * an optional self-wait AND gate a 常時 on their own wait, so for all 33 other
 * prints of this cost shape the missing call was unobservable -- which is
 * exactly why it survived. */
static void a6_cost_ten_only_stage_waits_nobody_but_erena_still_gains(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m10a = mid(&tg, COST_10);
    int m10b = mid(&tg, COST_10);
    if (!pin_id(m10a, COST_10) || !pin_id(m10b, COST_10)) return;
    CHECK(m10a != m10b, "A6 fixture: the two COST_10 cards are DISTINCT instances");

    int erena = erena_debut(&tg, m10a, m10b, RB_EMPTY_SLOT);
    if (erena < 0) return;
    int prompts = a_pay(&tg, 12);
    fprintf(stderr, "  (A6) prompts driven = %d\n", prompts);

    CHECK(is_waited(&tg, erena),
          "A6: the cost was paid and it is Erena herself who pays it");
    CHECK(!is_waited(&tg, m10a) && !is_waited(&tg, m10b),
          "A6: neither cost-10 member is コスト9以下, so neither may be waited");
    CHECK(!pending(&tg),
          "A6 CONTROL: the empty-candidate early return must terminate the ability "
          "rather than leave a prompt parked");
    CHECK_EQ(heart_of(&tg, erena, H05), 1,
             "A6: the orientation her own debut cost wrote must STILL reach her 常時 "
             "even when the chained effect had no target -- the empty-candidate early "
             "return in execute_change_state used to skip recalculate_constants(), "
             "leaving her in wait and granting no heart05");
}

/* ── A7. paying_the_debut_cost_switches_on_her_own_constant_heart05 ───────
 * The 登場 and the 常時 in one resolution. Every other test in the tree that
 * touches her 常時 writes the orientation by hand and calls
 * recalculate_constants() itself; nothing drives the real chain from the 登場
 * to the heart, which is the only way a regression in the pay path's
 * orientation WRITE would be caught from the outside. */
static void a7_pay_switches_on_her_own_constant_heart05(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m10 = mid(&tg, COST_10);
    if (!pin_id(m9, COST_9) || !pin_id(m10, COST_10)) return;

    int erena = erena_debut(&tg, m9, m10, RB_EMPTY_SLOT);
    if (erena < 0) return;
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "A7 premise: Erena is active on debut, so her 常時 grants nothing yet");

    a_pay(&tg, 12);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena),
          "A7: the paid cost is Erena's own orientation");
    CHECK_EQ(heart_of(&tg, erena, H05), 1,
             "A7: Erena's 常時 must turn on from the orientation her own debut cost "
             "just wrote -- not from a hand-set modifier plus a manual recalc");
}

/* ── A8. the_waited_opponent_member_crosses_another_cards_constant ─────────
 * The state change Erena causes re-triggers the auto-ability machinery, and
 * another card on the board can read it. 桂城 泉 PL!HS-bp5-016-N:
 *   「相手のステージにウェイト状態のメンバーが2人以上いるかぎり、heart06 を得る」
 *
 * This is the only board in the tree that produces a waited OPPONENT member
 * through a real ability resolution and then reads another card's 常時 off it;
 * every sibling hand-writes the orientation and recalculates itself, leaving
 * the re-trigger in change_state.rs:750-757 unpinned. */
static void a8_waited_opponent_crosses_another_constant(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = mid(&tg, ERENA_R);
    int izumi = mid(&tg, IZUMI_N);
    int m9    = mid(&tg, COST_9);
    int m4    = mid(&tg, COST_4);
    int filler = mid(&tg, COST_4);
    if (!pin_id(erena, ERENA_R) || !pin_id(izumi, IZUMI_N) ||
        !pin_id(m9, COST_9) || !pin_id(m4, COST_4) || !pin_id(filler, COST_4)) return;

    /* Izumi is already on OUR stage; the centre slot is free for Erena's debut. */
    set_stage(&tg, 0, izumi, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, m9, m4, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);
    /* One opponent member already waited, so Izumi is one short of her
       threshold and Erena paying the cost is what COMPLETES it. This is also
       what keeps the effect to a single candidate, so no member prompt is
       needed and the threshold is crossed purely by the effect. */
    rb_mods_set_orientation(&tg.state.mods, m4, "wait");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, izumi, H06), 0,
             "A8 premise: with 1 of the 2 needed opponent members waited, Izumi grants "
             "nothing -- so the heart below can only come from Erena's effect");

    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, erena);
    if (test_play_to_stage(&tg, erena, 1) != 1) {
        fprintf(stderr, "FAIL: A8 Erena was refused\n"); failures++; assertions++; return;
    }
    a_pay(&tg, 12);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, m9), "A8 premise for the threshold: the effect waited the cost-9 member");
    CHECK_EQ(heart_of(&tg, izumi, H06), 1,
             "A8: the wait Erena's 登場 caused must satisfy ANOTHER card's 常時 -- "
             "「2人以上」 is a threshold, so exactly one heart06");
}

/* ── A9. an_empty_opponent_stage_resolves_cleanly ──────────────────────────
 * The degenerate end of the same empty-candidate branch, and the one where a
 * missing `return` would be most visible. The "no prompt parked" assertion is
 * load-bearing: a drain that stopped early would satisfy every absence
 * assertion in this case. */
static void a9_empty_opponent_stage_resolves_cleanly(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena = erena_debut(&tg, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    if (erena < 0) return;

    a_pay(&tg, 12);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena),
          "A9: the cost is optional and the player paid it, so Erena is waited "
          "regardless of whether the effect had a target");
    CHECK(!pending(&tg),
          "A9: the ability must TERMINATE rather than leave a prompt parked -- a drain "
          "that stopped early would otherwise satisfy every absence assertion here");
    CHECK_EQ(tg.state.p[1].stage[0], RB_EMPTY_SLOT, "A9: an empty stage stays empty");
    CHECK_EQ(tg.state.p[1].stage[1], RB_EMPTY_SLOT, "A9: an empty stage stays empty");
    CHECK_EQ(tg.state.p[1].stage[2], RB_EMPTY_SLOT, "A9: an empty stage stays empty");
}

/* ── A10. two_copies_each_pay_their_own_orientation_and_see_the_board ─────
 * The queue-independence question. `play_to_stage` enqueues both 登場
 * abilities, and the first Erena to resolve waits a member -- which removes it
 * from the candidate set the SECOND Erena sees. If the two shared resolver
 * state, the second would either re-offer an already-waited member or skip
 * its own cost.
 *
 * KNOWN ENGINE DEFECT, documented by the Rust twin: `trigger_debut_abilities`
 * used to match the debut by CARD NUMBER and `break` out of the area loop on
 * the first area carrying that number, so copy B's debut re-ran COPY A's
 * ability. With activating_card still pointing at copy A (already in wait),
 * `get_change_state_candidates` found no ACTIVE candidate, `pay_cost_change_
 * state` returned Ok(()) with no choice, and the optional cost was treated as
 * satisfied without ever being offered. The player never got to choose and
 * copy B's orientation was never written. Generic, not Erena-specific. */
static void a10_two_copies_each_pay_their_own_orientation(void)
{
    static TestGame tg;
    reset_board(&tg);
    int erena_a = mid(&tg, ERENA_R);
    int erena_b = mid(&tg, ERENA_R);
    int m9  = mid(&tg, COST_9);
    int m4  = mid(&tg, COST_4);
    int m4b = mid(&tg, COST_4);
    int filler = mid(&tg, COST_4);
    if (!pin_id(erena_a, ERENA_R) || !pin_id(erena_b, ERENA_R) ||
        !pin_id(m9, COST_9) || !pin_id(m4, COST_4) || !pin_id(m4b, COST_4) ||
        !pin_id(filler, COST_4)) return;
    CHECK(erena_a != erena_b, "A10 fixture: the two Erenas are DISTINCT instances");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, m9, m4, m4b);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 15);

    /* --- copy A debuts left --- */
    test_add_to_hand(&tg, erena_a);
    if (test_play_to_stage(&tg, erena_a, 0) != 1) {
        fprintf(stderr, "FAIL: A10 copy A refused\n"); failures++; assertions++; return;
    }
    CHECK(pending(&tg),
          "A10 CONTROL: the first copy must open its OWN self-wait gate");
    if (!pending(&tg)) { describe_pending(&tg, "A10 copy A"); return; }
    if (pending_kind(&tg) && strcmp(pending_kind(&tg), "SelectTarget") != 0) {
        fprintf(stderr, "FAIL: A10 copy A: expected the SelectTarget gate, got '%s'\n",
                pending_kind(&tg));
        describe_pending(&tg, "A10 copy A");
        failures++; assertions++;
        return;
    }
    a_pay(&tg, 12);
    CHECK(pending(&tg),
          "A10 CONTROL: three qualifying members, so copy A's member prompt opens");
    {
        int offered[8];
        int n = offered_candidate_ids(&tg, offered, 8);
        CHECK_EQ(n, 3, "A10 CONTROL: copy A is offered all three コスト9以下 members");
    }
    int zero[1] = { 0 };
    select_indices(&tg, zero, 1);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena_a), "A10: the first copy paid its own orientation");
    CHECK(is_waited(&tg, m9), "A10: the first copy waited the cost-9 member");

    /* --- copy B debuts onto a board where m9 is already waited --- */
    test_add_to_hand(&tg, erena_b);
    if (test_play_to_stage(&tg, erena_b, 1) != 1) {
        fprintf(stderr, "FAIL: A10 copy B refused\n"); failures++; assertions++; return;
    }
    CHECK(pending(&tg),
          "A10: the second copy's debut must open its OWN optional self-wait gate -- the "
          "debut must be attributed to the card that PLAYED, not to the lowest-area "
          "copy sharing its card number (the known trigger_scan defect: otherwise the "
          "gate is skipped entirely and copy B's orientation is never written)");
    if (!pending(&tg)) { describe_pending(&tg, "A10 copy B"); return; }
    if (pending_kind(&tg) && strcmp(pending_kind(&tg), "SelectTarget") != 0) {
        fprintf(stderr, "FAIL: A10 copy B: expected the SelectTarget gate, got '%s' -- "
                        "this is the trigger_scan card-NUMBER attribution defect\n",
                pending_kind(&tg));
        describe_pending(&tg, "A10 copy B");
        failures++; assertions++;
        return;
    }
    a_pay(&tg, 12);
    {
        int offered[8];
        int n = offered_candidate_ids(&tg, offered, 8);
        CHECK_EQ(n, 2,
                 "A10: the already-waited cost-9 member must have dropped OUT of the "
                 "second copy's candidate set (Q137), leaving the two cost-4 members");
    }
    int zero2[1] = { 0 };
    select_indices(&tg, zero2, 1);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena_b),
          "A10: the second copy paid its OWN orientation -- the two must not share cost "
          "state");
    {
        const int trio[3] = { m9, m4, m4b };
        CHECK_EQ(n_waited(&tg, trio, 3), 2,
                 "A10: two debuts, one member each -- total 2 waited, and neither copy "
                 "double-waits");
    }
}

/* ── A11. the_prompted_path_applies_the_wait_to_the_chosen_member_only ────
 * When the effect DOES prompt, the wait is not applied by the prompt itself.
 * `execute_change_state` stashes a re-apply action built with
 * `..Default::default()`, so it carries only action/target/count/kind -- NOT
 * state_change, card_type or cost_limit. Read naively that would be a bug: the
 * re-apply's `is_member_op` would be false and the wait would divert to the
 * energy branch. The Rust twin MEASURED that it does NOT happen and locks in
 * the observable outcome, so a future change names itself instead of surfacing
 * as a stray "member not waited" elsewhere.
 *
 * It is also the only place the prompted path is exercised with a selection
 * OTHER than candidate 0. */
static void a11_prompted_path_waits_chosen_member_only(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9  = mid(&tg, COST_9);
    int m4  = mid(&tg, COST_4);
    int m4b = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4) || !pin_id(m4b, COST_4)) return;

    int erena = erena_debut(&tg, m9, m4, m4b);
    if (erena < 0) return;
    a_pay(&tg, 12);
    CHECK(pending(&tg), "A11 CONTROL: three candidates, so the prompt opens");
    if (!pending(&tg)) { describe_pending(&tg, "A11"); return; }
    {
        int offered[8];
        int n = offered_candidate_ids(&tg, offered, 8);
        CHECK_EQ(n, 3, "A11 CONTROL: all three are コスト9以下");
    }

    /* Choose the SECOND candidate (stage slot 1). */
    int one[1] = { 1 };
    select_indices(&tg, one, 1);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, m4),
          "A11: candidate index 1 is stage slot 1, the first cost-4 copy -- the "
          "re-apply must apply the wait to the CHOSEN member, not to candidate 0");
    CHECK(!is_waited(&tg, m9), "A11: candidate 0 was not chosen, so m9 stays active");
    CHECK(!is_waited(&tg, m4b), "A11: candidate 2 was not chosen either");
    CHECK(is_waited(&tg, erena),
          "A11: the paid cost is Erena's own orientation, independent of which member "
          "was chosen");
    CHECK_EQ(tg.state.p[1].stage[0], m9,
             "A11: no member left the stage -- this is an orientation, not a move, on "
             "the PROMPTED path as well as the auto-resolve one");
    CHECK_EQ(tg.state.p[1].stage[1], m4,  "A11: the chosen member is still in its slot");
    CHECK_EQ(tg.state.p[1].stage[2], m4b, "A11: the unchosen member is still in its slot");
}

/* ── A12. the_constant_condition_follows_the_card_not_the_resolving_player ─
 * The 常時 condition is `self_target: true` on the card's OWN wait, so it must
 * follow the CARD, not the player who happens to be resolving something.
 *
 * One Erena per side: the OPPONENT's copy waited, OURS active. Getting this
 * wrong is easy and SILENT -- a condition evaluated against gs.activating_card,
 * or against ability_master_id instead of the constant's own controller, would
 * hand the heart to the wrong side, and a single-owner test on our own copy
 * cannot tell the difference, because with one Erena both readings agree.
 * Two copies, one per side, is the only board where they diverge. */
static void a12_constant_condition_follows_card_not_player(void)
{
    static TestGame tg;
    reset_board(&tg);
    int ours   = mid(&tg, ERENA_R);
    int theirs = mid(&tg, ERENA_R);
    if (!pin_id(ours, ERENA_R) || !pin_id(theirs, ERENA_R)) return;
    CHECK(ours != theirs, "A12 fixture: the two Erenas are DISTINCT instances");

    set_stage(&tg, 0, ours,   RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, theirs, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    /* The OPPONENT's copy is the one in wait; ours is active. */
    rb_mods_set_orientation(&tg.state.mods, theirs, "wait");
    test_recalc(&tg);
    CHECK(is_waited(&tg, theirs), "A12 premise: the opponent's copy really is waited");

    CHECK_EQ(heart_of(&tg, theirs, H05), 1,
             "A12: the opponent's waited Erena grants the OPPONENT a heart05 -- the "
             "常時 condition is `self_target` on her own wait, so it must be evaluated "
             "for her controller");
    CHECK_EQ(heart_of(&tg, ours, H05), 0,
             "A12: OUR copy is ACTIVE, so it must grant nothing. A heart here means the "
             "condition was evaluated against the resolving player (activating_card / "
             "ability_master_id) instead of the constant's own card");
}

/* ── A13. waiting_an_opponent_erena_grants_the_opponent_her_own_heart ─────
 * The same ownership clause, reached through Erena HERSELF waiting an
 * opponent's Erena. This is the one board where BOTH copies' 常時 fire at once
 * and the two sides are distinguishable. Erena's cost is 7, so she qualifies as
 * a コスト9以下 target. */
static void a13_waiting_opponent_erena_grants_opponent_her_heart(void)
{
    static TestGame tg;
    reset_board(&tg);
    int ours   = mid(&tg, ERENA_R);
    int theirs = mid(&tg, ERENA_R);
    int filler = mid(&tg, COST_4);
    if (!pin_id(ours, ERENA_R) || !pin_id(theirs, ERENA_R) ||
        !pin_id(filler, COST_4)) return;
    CHECK(ours != theirs, "A13 fixture: the two Erenas are DISTINCT instances");
    CHECK(printed_cost(theirs) <= 9,
          "A13 fixture: the opponent's Erena costs 7, so she qualifies as a コスト9以下 "
          "target");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, theirs, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 15);
    CHECK_EQ(heart_of(&tg, theirs, H05), 0,
             "A13 premise: the opponent's Erena is active, so she grants nothing yet");

    test_add_to_hand(&tg, ours);
    if (test_play_to_stage(&tg, ours, 1) != 1) {
        fprintf(stderr, "FAIL: A13 our Erena was refused\n"); failures++; assertions++;
        return;
    }
    a_pay(&tg, 12);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, theirs),
          "A13: the opponent's Erena is コスト7, so she is a legal target for the effect");
    CHECK(is_waited(&tg, ours), "A13: our copy is waited by the optional cost we paid");
    CHECK_EQ(heart_of(&tg, theirs, H05), 1,
             "A13: the opponent's waited copy grants the OPPONENT her heart05");
    CHECK_EQ(heart_of(&tg, ours, H05), 1,
             "A13: our waited copy grants US her heart05 -- each side resolves its own "
             "constant, which is the ownership clause read from both directions");
}

/* ── A14. the_resolving_player_chooses_which_opponent_member_is_waited ────
 * The printed card says 「相手のステージにいる…メンバー1人をウェイトにする」. The
 * cards being waited belong to the opponent, but the CHOICE of which one is
 * made by the player whose ability is resolving -- Erena, not her opponent.
 * In LLSS target selection always belongs to the resolving player; there is no
 * rule handing the pick to the owner of the targeted cards.
 *
 * This matters mechanically, not just formally: if the OPPONENT picked, they
 * would choose the least damaging member for themselves, turning a cost to the
 * card's player into a cost the card's player does not control. The engine has
 * explicit machinery for the OPPOSITE behaviour (queue.rs:308-343 routes a
 * SelectCard whose target_player_id is "opponent" to the opponent whenever the
 * resolver's spawn context says the effect targets the opponent). */
static void a14_resolving_player_chooses_which_opponent_member(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9 = mid(&tg, COST_9);
    int m4 = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4)) return;

    if (erena_debut(&tg, m9, m4, RB_EMPTY_SLOT) < 0) return;
    a_pay(&tg, 12);
    CHECK(pending(&tg), "A14 CONTROL: two candidates, so the member prompt opens");
    if (!pending(&tg)) { describe_pending(&tg, "A14"); return; }
    {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        CHECK(c && c->target_player_id[0] == 'o',
              "A14 premise: the prompt really does target the opponent -- the test is "
              "about WHO answers an opponent-targeted SelectCard");
        if (c) fprintf(stderr, "  (A14) target_player_id=\"%s\"\n", c->target_player_id);
    }

    int picker = test_queue_entry_choice_seat(&tg, 0);
    fprintf(stderr, "  (A14) choice seat = %d (-1 = unstamped, 0 = p1, 1 = p2)\n", picker);
    CHECK_EQ(picker, 0,
             "the player RESOLVING Erena's 登場 (p1) chooses which of the opponent's "
             "members is waited. If this reads 1, the OPPONENT is choosing which of "
             "their own members takes the ウェイト -- both a rules error and a "
             "mechanical one, see queue.rs:308-343");
}

/* ── A15. a_cost_free_self_wait_debut_repeats_on_the_first_copy ───────────
 * 近江彼方 PL!N-bp3-006-P (cost 9):
 *   登場 このメンバーをウェイトにする。
 * A cost-free, prompt-free, self-target debut -- the simplest 登場 in the pool.
 * Two copies: first in LeftSide, then one in Center. The correct result is
 * that BOTH end up waited, one per debut.
 *
 * This isolates the trigger-scan defect from everything else in Erena's
 * ability: no optional cost, no gate, no prompt, no opponent selection, so the
 * ONLY thing that can go wrong is WHICH COPY the scan attributes the debut to.
 * Unlike the Erena case that bug is SILENT -- no missing prompt, just a member
 * that never receives its own 登場 -- which is why nothing in the tree caught
 * it and why this test asserts the BOARD rather than a prompt. */
static void a15_cost_free_self_wait_debut_repeats_on_first_copy(void)
{
    static TestGame tg;
    reset_board(&tg);
    int kanata_a = mid(&tg, KANATA_P);
    int kanata_b = mid(&tg, KANATA_P);
    int filler   = mid(&tg, COST_4);
    if (!pin_id(kanata_a, KANATA_P) || !pin_id(kanata_b, KANATA_P) ||
        !pin_id(filler, COST_4)) return;
    CHECK(kanata_a != kanata_b, "A15 fixture: the two Kanatas are DISTINCT instances");
    CHECK_EQ(printed_cost(kanata_a), 9,
             "A15 fixture: PL!N-bp3-006-P costs 9 -- irrelevant to the ability, which is "
             "self-target and cost-free, but pinned so the fixture is unmistakably the "
             "card the Rust doc names");

    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 30);

    test_add_to_hand(&tg, kanata_a);
    if (test_play_to_stage(&tg, kanata_a, 0) != 1) {
        fprintf(stderr, "FAIL: A15 copy A refused\n"); failures++; assertions++; return;
    }
    a_drain(&tg, 12);
    CHECK(is_waited(&tg, kanata_a),
          "A15 premise: the first copy's 登場 waited ITSELF -- it is unconditional, so no "
          "prompt and the orientation must be written directly");

    /* Take copy A out of wait so the second debut's outcome is UNAMBIGUOUS. If the
       trigger scan attributes the debut correctly, playing copy B waits copy B and A
       stays active. If it attributes to the first copy by card number, playing B
       instead re-waits A. */
    rb_mods_set_orientation(&tg.state.mods, kanata_a, "active");
    test_recalc(&tg);
    CHECK(!is_waited(&tg, kanata_a),
          "A15 premise: copy A is forced ACTIVE, so any wait after B's debut must be "
          "B's own");

    test_add_to_hand(&tg, kanata_b);
    if (test_play_to_stage(&tg, kanata_b, 1) != 1) {
        fprintf(stderr, "FAIL: A15 copy B refused\n"); failures++; assertions++; return;
    }
    a_drain(&tg, 12);

    CHECK(!is_waited(&tg, kanata_a),
          "A15: the FIRST copy must NOT be waited by the second copy's debut -- that "
          "means the debut was attributed by card NUMBER to the lowest-area copy "
          "(trigger_scan.rs) rather than to the copy that actually played");
    CHECK(is_waited(&tg, kanata_b),
          "A15: the second copy's debut must wait the SECOND copy. Before the "
          "trigger-scan fix the debut resolved to the first copy instead, so B stayed "
          "active");
}

/* ── A16. a_baton_touch_debut_belongs_to_the_arriving_copy ─────────────────
 * The baton-touch debut is the OTHER call site of `trigger_debut_abilities`
 * (turn/phases.rs:1076, the one passing baton_touch_used = true) -- a genuinely
 * different path from an ordinary play, and one nothing in the tree reached
 * through the duplicate case.
 *
 * Board: copy 1 in LeftSide, copy 2 in Center, and copy 3 is played INTO Center
 * -- an occupied area, so that play is a baton touch and copy 2 goes to the
 * waitroom while copy 3 arrives. Two copies of one card number remain staged,
 * and the LEFTMOST is the one that did not just debut.
 *
 * The copies must be a card whose 登場 is visible PER INSTANCE. Three identical
 * cost-4 members whose 登場 was "draw 1, mill 1" would be worthless as a
 * discriminator, because whichever copy fired produces the same draw. */
static void a16_baton_touch_debut_belongs_to_arriving_copy(void)
{
    static TestGame tg;
    reset_board(&tg);
    int first    = mid(&tg, KANATA_P);
    int second   = mid(&tg, KANATA_P);
    int arriving = mid(&tg, KANATA_P);
    int filler   = mid(&tg, COST_4);
    if (!pin_id(first, KANATA_P) || !pin_id(second, KANATA_P) ||
        !pin_id(arriving, KANATA_P) || !pin_id(filler, COST_4)) return;
    CHECK(first != second && second != arriving && first != arriving,
          "A16 fixture: the three Kanatas are DISTINCT instances");

    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 30);

    /* Two copies already staged, as fixtures. Neither has debuted. */
    test_add_to_stage(&tg, 0, first);
    test_add_to_stage(&tg, 1, second);
    CHECK_EQ(tg.state.p[0].stage[0], first,
             "A16 premise: two copies staged, the leftmost is NOT the one about to debut");
    CHECK_EQ(tg.state.p[0].stage[1], second, "A16 premise: copy 2 is in the centre");

    /* Playing into an OCCUPIED area is a baton touch. test_play_to_stage drives
       the ordinary main-phase play and the engine resolves the occupied
       destination as a baton touch (test_baton_touch.c:78 exercises the same
       call for exactly this); there is no separate rb_play_baton_touch entry
       point in the C surface. */
    test_add_to_hand(&tg, arriving);
    CHECK_EQ(test_play_to_stage(&tg, arriving, 1), 1,
             "A16: the play into the occupied centre is accepted as a baton touch");
    rb_process_pending_auto_abilities(&tg.state);
    a_drain(&tg, 12);

    fprintf(stderr, "  (A16) stage = [%d, %d, %d], waitroom = %d\n",
            tg.state.p[0].stage[0], tg.state.p[0].stage[1], tg.state.p[0].stage[2],
            tg.state.p[0].discard.n);
    CHECK_EQ(tg.state.p[0].stage[1], arriving,
             "A16 premise: playing into an occupied area must REPLACE the occupant, so "
             "copy 2 left the stage and copy 3 took its slot. If the slot is empty the "
             "baton touch did not happen and the wait assertions below are vacuous.");
    CHECK_EQ(tg.state.p[0].stage[0], first, "A16: copy 1 is untouched by the baton touch");

    CHECK(is_waited(&tg, arriving),
          "A16: the ARRIVING copy's own 登場 must fire. The baton-touch call site must "
          "attribute the debut by the played card's id, not by card number -- with copy "
          "1 in the leftmost area, a number-based scan breaks on copy 1 and leaves the "
          "arriving copy active.");
    CHECK(!is_waited(&tg, first),
          "A16: the leftmost copy did not debut, so it must NOT be waited");
    CHECK(!is_waited(&tg, second),
          "A16: copy 2 was replaced by the baton touch; its 登場 must not run");
}

/* ── A17. a_second_entry_to_the_stage_offers_the_debut_gate_again ─────────
 * 登場 is not a once-per-game or once-per-turn ability -- `use_limit` is null
 * in abilities.json -- it is scoped to the ENTRY EVENT, so a member that leaves
 * the stage and returns debuts AGAIN. This matters for this card in
 * particular because her 登場 is an OPTIONAL self-wait plus a chained opponent
 * wait: if re-entry were silently ignored, the second copy of Erena to arrive
 * in a turn would cost nothing and would hand the player a free ウェイト on a
 * コスト9以下 opponent member while she herself never went to wait.
 *
 * The removal is hand-built, but the second debut is a real play, so the whole
 * 登場 chain runs for real the second time. */
static void a17_second_entry_offers_debut_gate_again(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9 = mid(&tg, COST_9);
    int m4 = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4)) return;

    int erena = erena_debut(&tg, m9, m4, RB_EMPTY_SLOT);
    if (erena < 0) return;
    a_pay(&tg, 12);
    int zero[1] = { 0 };
    select_indices(&tg, zero, 1);
    a_drain(&tg, 12);
    CHECK(is_waited(&tg, erena) && is_waited(&tg, m9),
          "A17 premise: the FIRST debut resolved -- Erena waited herself and the cost-9 "
          "member is waited");

    /* Erena leaves the stage for the waitroom. Leaving the stage clears
       orientation, and it is that CLEAR which makes the next entry a real debut
       rather than a no-op. */
    int slot = -1;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg.state.p[0].stage[i] == erena) slot = i;
    if (slot < 0) {
        fprintf(stderr, "FAIL: A17 Erena is not on our stage\n"); failures++; assertions++;
        return;
    }
    tg.state.p[0].stage[slot] = RB_EMPTY_SLOT;
    rb_waitroom_add(&tg.state.p[0], erena);
    rb_mods_clear_card(&tg.state.mods, erena);
    test_recalc(&tg);
    CHECK(!is_waited(&tg, erena),
          "A17 premise: off-stage she is no longer waited, so a re-entry is a genuine "
          "debut");
    CHECK_EQ(heart_of(&tg, erena, H05), 0,
             "A17 premise: and her 常時 is off while she is off-stage");

    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, erena);
    if (test_play_to_stage(&tg, erena, 2) != 1) {
        fprintf(stderr, "FAIL: A17 the re-entry play was refused\n");
        failures++; assertions++;
        return;
    }
    CHECK(pending(&tg),
          "A17: the SECOND entry must open the 登場 self-wait gate AGAIN -- 登場 is scoped "
          "to the entry event, not to the first time the card is seen");
    if (!pending(&tg)) { describe_pending(&tg, "A17 re-entry"); return; }
    if (pending_kind(&tg) && strcmp(pending_kind(&tg), "SelectTarget") != 0) {
        fprintf(stderr, "FAIL: A17 re-entry: expected the SelectTarget gate, got '%s'\n",
                pending_kind(&tg));
        describe_pending(&tg, "A17 re-entry");
        failures++; assertions++;
        return;
    }
    a_decline(&tg, 12);
    a_drain(&tg, 12);

    CHECK(!is_waited(&tg, erena),
          "A17: declining the second time must leave Erena active and must NOT put a "
          "second コスト9以下 member down -- otherwise a re-entering Erena gets a free wait");
    {
        const int two[2] = { m9, m4 };
        CHECK_EQ(n_waited(&tg, two, 2), 1,
                 "A17: still exactly ONE waited opponent member -- the declined second "
                 "debut added none");
    }
}

/* ── A18. a_wait_immune_opponent_member_is_offered_but_not_actually_waited ─
 * An opponent member protected by 「相手の効果によってはウェイトしない」 is still
 * OFFERED (Erena chooses freely, Q274) but the wait is SUPPRESSED at
 * application time (change_state.rs:574-588).
 *
 * C-SURFACE GAP: the Rust `GameState.wait_immune_members` list -- the field
 * restriction.rs:191 writes, keyed by (member, owner), and the one the
 * `controller != owner` check reads -- has NO counterpart in the C engine.
 * There is no `immune` symbol anywhere in engine_c_wip/include/ or
 * engine_c_wip/src/, so there is nothing to install and the suppression half is
 * not expressible. That is reported as an EXPECTED_GAP, never as a pass.
 * The PROMPT-SHAPE half, which is what Q274 is really about (the immune member
 * stays a legal CHOICE), is fully expressible and is asserted. */
static void a18_wait_immune_member_offered_but_not_waited(void)
{
    static TestGame tg;
    reset_board(&tg);
    int m9 = mid(&tg, COST_9);
    int m4 = mid(&tg, COST_4);
    if (!pin_id(m9, COST_9) || !pin_id(m4, COST_4)) return;

    int erena = erena_debut(&tg, m9, m4, RB_EMPTY_SLOT);
    if (erena < 0) return;

    /* Rust: game.state.wait_immune_members.push((m9, "p2")). No C equivalent
       exists, so the immunity is recorded as a gap rather than faked. */
    EXPECTED_GAP(0,
                 "A18 OUTCOME: the wait-suppression half cannot be translated -- C has no "
                 "GameState.wait_immune_members / (member, owner) immunity list (no "
                 "'immune' symbol in include/ or src/), so an immune member is not "
                 "distinguishable from a normal one. The Rust twin suppresses the wait at "
                 "application time (change_state.rs:574-588) with "
                 "wait_immune_members=(m9,\"p2\").");

    a_pay(&tg, 12);
    /* PROMPT-SHAPE half -- Q274: an immune member is still a legal CHOICE, the
       suppression happens at application time, so the prompt must still open. */
    CHECK(pending(&tg),
          "A18 CONTROL: the prompt still opens, so the candidate set is the thing under "
          "test and not a non-firing effect");
    if (!pending(&tg)) { describe_pending(&tg, "A18"); return; }
    CHECK(offer_contains(&tg, m9),
          "A18: the (immune) cost-9 member must remain SELECTABLE -- the player chooses "
          "freely and the wait would simply be suppressed");
    CHECK(offer_contains(&tg, m4),
          "A18: the other qualifying member is offered too");

    int zero[1] = { 0 };
    select_indices(&tg, zero, 1);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena),
          "A18: the cost was still paid -- suppression voids the OPPONENT wait, not "
          "Erena's own");
    CHECK(is_waited(&tg, m9),
          "A18 ENGINE GAP (see the EXPECTED_GAP above): with no immunity list in C the "
          "wait on m9 is NOT suppressed, which is the expected divergence from the Rust "
          "twin's !is_waited(m9). Recorded as an engine-surface gap, not a test bug.");
    CHECK(!is_waited(&tg, m4),
          "A18: the unselected member is untouched -- 1人 was chosen and that 1人 was the "
          "cost-9 member, so nothing is silently swapped in");
}

/* ── A19. the_p_plus_print_has_the_same_debut ──────────────────────────────
 * PL!-bp5-333-P＋ shares the ability text, so abilities.json registers the
 * 登場 for BOTH prints. The existing SECTION C1 covers only the P＋'s 常時; the
 * debut half was not exercised. */
static void a19_p_plus_print_has_same_debut(void)
{
    static TestGame tg;
    reset_board(&tg);
    /* test_new_id, NOT test_id: the fullwidth plus must resolve to the P＋ print
       and not alias something else. */
    int erena = mid(&tg, ERENA_PP);
    int m9    = mid(&tg, COST_9);
    int m10   = mid(&tg, COST_10);
    int filler = mid(&tg, COST_4);
    if (!pin_id(erena, ERENA_PP) || !pin_id(m9, COST_9) || !pin_id(m10, COST_10) ||
        !pin_id(filler, COST_4)) return;
    CHECK_EQ(printed_cost(erena), 7, "A19: the P＋ print costs 7 like the base print");
    dump_abilities(erena, "A19 P+");

    set_stage(&tg, 0, RB_EMPTY_SLOT, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    set_stage(&tg, 1, m9, m10, RB_EMPTY_SLOT);
    fill_decks(&tg, filler, 30);
    test_give_energy(&tg, 15);
    test_add_to_hand(&tg, erena);
    if (test_play_to_stage(&tg, erena, 1) != 1) {
        fprintf(stderr, "FAIL: A19 the P＋ debut was refused\n");
        failures++; assertions++;
        return;
    }
    CHECK(pending(&tg),
          "A19: the P＋ print must open the SAME optional self-wait gate (SelectTarget)");
    if (!pending(&tg)) { describe_pending(&tg, "A19 P+"); return; }
    if (pending_kind(&tg) && strcmp(pending_kind(&tg), "SelectTarget") != 0) {
        fprintf(stderr, "FAIL: A19: expected the SelectTarget gate, got '%s'\n",
                pending_kind(&tg));
        describe_pending(&tg, "A19 P+");
        failures++; assertions++;
        return;
    }
    a_pay(&tg, 12);
    a_drain(&tg, 12);

    CHECK(is_waited(&tg, erena), "A19: the P＋ debut must pay its own orientation when accepted");
    CHECK(is_waited(&tg, m9), "A19: the P＋ debut must wait the コスト9以下 member");
    CHECK(!is_waited(&tg, m10), "A19: the P＋ debut must respect the same コスト9以下 bound");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION B -- state/orientation_gated/nico_cannot_activate_test.rs
 *
 * The existing SECTION C4 covers nico_blocks_kotori_activation,
 * kotori_activate_works_without_restriction and nico_active_phase_still_
 * activates. The ONE test it does not separate is nico_p1_active_phase_still_
 * activates, which differs in a way that matters: the Rust version drives the
 * phase machine for OUR OWN seat (P1) with the member on OUR stage, whereas the
 * P2 variant stages the filler on the opponent's side. The C4 version happens to
 * use P1's stage but reaches Active via two bare rb_advance_phase calls from
 * wherever the board left the phase, rather than by SETTING Phase::Active and
 * TurnPhase::FirstAttackerNormal and calling TurnEngine::advance_phase
 * directly. Those are different entry conditions, so the P1 case is ported here
 * with the Rust entry condition.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Q180, OUR side: the cannot_activate_by_effect aura blocks EFFECTS only; the
   natural Active phase is a game rule, not an effect, and still activates.
   The member is on OUR OWN stage, which is the case the aura names ("p1") and
   which the P2-only sibling cannot cover. */
static void b1_nico_p1_active_phase_still_activates(void)
{
    static TestGame tg;
    reset_board(&tg);
    int nico   = mid(&tg, NICO);
    int filler = mid(&tg, COST_4);
    if (!pin_id(nico, NICO) || !pin_id(filler, COST_4)) return;
    CHECK_EQ(printed_cost(nico), 4, "B1 fixture: PL!-pb1-009-R costs 4");

    fill_decks(&tg, filler, 20);
    test_give_energy(&tg, 6);
    set_stage(&tg, 0, filler, RB_EMPTY_SLOT, RB_EMPTY_SLOT);
    rb_mods_set_orientation(&tg.state.mods, filler, "wait");

    test_add_to_hand(&tg, nico);
    if (test_play_to_stage(&tg, nico, 1) != 1) {
        fprintf(stderr, "FAIL: B1 Nico was refused\n"); failures++; assertions++; return;
    }
    a_drain(&tg, 12);
    CHECK_EQ(tg.state.p[0].stage[1], nico, "B1 CONTROL: Nico is really on our centre stage");

    /* CONTROL: the aura must really be flagging OUR seat, or the result below
       would be a missing flag rather than a rule exemption. Nico's ab#1 is
       「自分のステージのメンバーと相手のステージのメンバーは、このターン、
       効果によってアクティブにできない」 -- both sides. */
    {
        int flagged = rb_mods_is_delayed_cannot_active(&tg.state.mods, filler);
        fprintf(stderr, "  (B1) filler flagged = %d, n_cannot_active_cards = %d\n",
                flagged, tg.state.n_cannot_active_cards);
        CHECK(flagged,
              "B1 CONTROL: the cannot_activate_by_effect aura really is flagging our "
              "waited member, so the Active-phase result below is a rule exemption and "
              "not an absent flag");
    }

    /* Rust entry condition: set Phase::Active + TurnPhase::FirstAttackerNormal,
       then call TurnEngine::advance_phase directly. */
    tg.state.phase = RB_PHASE_ACTIVE;
    tg.state.first_attacker = 1;
    rb_advance_phase(&tg.state);
    rb_advance_phase(&tg.state);
    fprintf(stderr, "  (B1) orientation after Active = %s\n",
            rb_mods_get_orientation(&tg.state.mods, filler)
                ? rb_mods_get_orientation(&tg.state.mods, filler) : "(null)");

    CHECK(!is_waited(&tg, filler),
          "B1 Q180: P1's Active phase MUST activate its waited member despite the aura -- "
          "the restriction blocks effects that set an orientation to active, not the "
          "natural phase activation");
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
    if (pid < 0) { fprintf(stderr, "fork failed\n"); n_failed++; return; }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, g0 = harness_gaps;
        current_test = name;
        fn();
        printf("        %-52s %3d assertion(s), %d failure(s), %d harness gap(s)\n",
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

    printf("--- A0. orientation_gated/erena_debut bound fixtures (PL!-bp5-017-N / -005-AR) ---\n");
    run("A0_bound_fixtures",           a0_boundary_fixtures_print_cost_nine_and_cost_ten);

    printf("--- A1. erena_debut: paying the cost (PL!-bp5-333-R) ---\n");
    run("A1_pay_waits_m9_not_m10",     a1_pay_waits_cost_nine_not_cost_ten);

    printf("--- A2. erena_debut: the offered candidate set / operator ---\n");
    run("A2_offered_excludes_m10",     a2_offered_set_excludes_cost_ten);

    printf("--- A3. erena_debut: count 1 is a real limit ---\n");
    run("A3_exactly_one_of_three",     a3_exactly_one_waited_of_three_qualify);

    printf("--- A4. erena_debut: Q137 already-waited / auto-resolve ---\n");
    run("A4_already_waited_resolves",  a4_already_waited_not_offered_auto_resolves);

    printf("--- A5. erena_debut: declining the optional self-wait ---\n");
    run("A5_decline_cancels_chain",    a5_decline_does_not_wait_opponent);

    printf("--- A6. erena_debut: empty candidate set / stale-recalc defect ---\n");
    run("A6_empty_cand_still_recalcs",  a6_cost_ten_only_stage_waits_nobody_but_erena_still_gains);

    printf("--- A7. erena_debut: 登場 + 常時 in one resolution ---\n");
    run("A7_pay_switches_heart05",     a7_pay_switches_on_her_own_constant_heart05);

    printf("--- A8. erena_debut: cross-ability constant threshold (PL!HS-bp5-016-N) ---\n");
    run("A8_crosses_izumi_heart06",    a8_waited_opponent_crosses_another_constant);

    printf("--- A9. erena_debut: empty opponent stage ---\n");
    run("A9_empty_stage_clean",        a9_empty_opponent_stage_resolves_cleanly);

    printf("--- A10. erena_debut: two copies / trigger-scan attribution ---\n");
    run("A10_two_copies_independent",  a10_two_copies_each_pay_their_own_orientation);

    printf("--- A11. erena_debut: the prompted re-apply path ---\n");
    run("A11_waits_chosen_only",       a11_prompted_path_waits_chosen_member_only);

    printf("--- A12. erena_debut: 常時 ownership follows the card ---\n");
    run("A12_condition_follows_card",  a12_constant_condition_follows_card_not_player);

    printf("--- A13. erena_debut: both sides' 常時 fire at once ---\n");
    run("A13_both_erenas_gated",       a13_waiting_opponent_erena_grants_opponent_her_heart);

    printf("--- A14. erena_debut: WHO picks the opponent's member ---\n");
    run("A14_resolver_picks",          a14_resolving_player_chooses_which_opponent_member);

    printf("--- A15. trigger-scan isolation: 近江彼方 self-wait (PL!N-bp3-006-P) ---\n");
    run("A15_debut_repeats_per_copy",  a15_cost_free_self_wait_debut_repeats_on_first_copy);

    printf("--- A16. baton-touch call site of the debut scan ---\n");
    run("A16_baton_debut_arriving",    a16_baton_touch_debut_belongs_to_arriving_copy);

    printf("--- A17. 登場 is scoped to the entry event ---\n");
    run("A17_reentry_offers_gate",     a17_second_entry_offers_debut_gate_again);

    printf("--- A18. Q274 wait-immunity: offered but suppressed ---\n");
    run("A18_immune_offered_suppressed", a18_wait_immune_member_offered_but_not_waited);

    printf("--- A19. the reprint PL!-bp5-333-P＋ ---\n");
    run("A19_p_plus_same_debut",       a19_p_plus_print_has_same_debut);

    printf("--- B1. nico_cannot_activate: Q180 on P1's own side (PL!-pb1-009-R) ---\n");
    run("B1_p1_active_still_activates", b1_nico_p1_active_phase_still_activates);

    rb_unload();
    printf("\n==== recover/score/state delta parity: %d cases passed, %d cases failed, "
           "%d crashed, %d harness gap(s) ====\n",
           n_ok, n_failed, n_crashed, harness_gaps);
    if (failures) fprintf(stderr, "%d assertion-level failure(s)\n", failures);
    return (failures || n_crashed) ? 1 : 0;
}

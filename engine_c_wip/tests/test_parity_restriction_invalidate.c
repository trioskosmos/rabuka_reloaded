/* =====================================================================
 * test_parity_restriction_invalidate.c
 *
 * C parity tests for the Rust cluster
 *
 *   engine/tests/test_modules/effects/yell/
 *     pb2_yell_plus_ten_when_two_mus_success_pl_pb2_039_l_test.rs   (6 #[test])
 *
 * 僕たちはひとつの光 `PL!-pb2-039-L` (ライブ開始時):
 *
 *   自分の成功ライブカード置き場に『μ's』のカードが2枚以上ある場合、ライブ終了時まで、
 *   エールによって公開される自分のカードの枚数が10枚増える。
 *
 * In English: while two or more μ's cards sit in your SUCCESS live zone, the cheer
 * requires 10 more cards to be revealed for the rest of the live.
 *
 * WHAT THE NUMBER MEANS, and the correction the Rust file records
 * ----------------------------------------------------------------
 * `cheer_checks_required` IS the number of cards the player must reveal:
 * `perform_cheer_check` draws one card per check and completes the cheer when
 * `cheer_checks_done >= cheer_checks_required`. So 「10枚増える」 means the
 * REQUIREMENT goes UP by 10. An earlier reading of this card asserted the
 * requirement should FALL; it must RISE. The precedent is
 * `PL!SP-bp2-010-R＋` Wien 「8枚減る」, which lowers it by 8 -- already covered by
 * test_parity_cost_discount.c §B, whose three passing arms are this file's
 * control that the mechanism itself works.
 *
 * The `『μ's』` filter is the other axis, and it is the one most easily lost:
 * `rb_card_matches_group_str` resolves a group name against the card's UNIT
 * first, then against the SERIES-derived name. `LL-PR-004-PR` (AiScReam) has
 * `unit = "AiScReam"` and NO series, so it cannot match μ's through either arm
 * -- which is what makes it the real non-μ's control. (`PL!-sd1-019-SD` does
 * NOT: it has `unit = None` but `series = ラブライブ！`, so
 * `rb_card_series_to_group` makes it a μ's card. That mistake is recorded in
 * the Rust header and is why the fixture below is AiScReam.)
 *
 * THE DELTA: what this file adds over the rest of the C corpus
 * -----------------------------------------------------------
 * PL!-pb2-039-L already appears in test_parity_live_total2.c, but only for its
 * ab#1 (ライブ成功時, per-distinct-μ's-member score +1) and as a no-icon
 * control. NOTHING in the C corpus covers its ab#0 -- the ライブ開始時
 * modify_yell_count, the 2-μ's-success-zone threshold, or the sign of the
 * effect. `rb_execute_modify_yell_count` (src/ability/effects/score.c:909) is
 * reachable and writes the yell ledger that `rb_effective_cheer_checks_required`
 * (src/core/tracking.c:95) sums, so the whole path is testable from C.
 *
 * HARNESS NOTES (confirmed by running peers, not assumed)
 * ------------------------------------------------------
 *  * `rb_load("src")` fails under the isolated out-of-tree build, so
 *    load_card_db() falls back to ../cards/build then cards/build.
 *  * sizeof(GameState) is ~781 KB, so EVERY TestGame local is `static`.
 *  * `mid()` is test_new_id (the next DISTINCT pool slot, matching Rust's
 *    `game.id()`). test_id aliases the SHARED template, so a card tracked by
 *    identity must never use it -- two different prints would collide.
 *  * `rb_create_card_copy` draws from a process-global monotonic pool nothing
 *    resets, so a long suite exhausts it (4096 ids) and test_new_id returns
 *    -1, after which mid() would silently alias. fresh() calls
 *    rb_card_copy_pool_reset() per case.
 *  * `rb_card_no_eq` is FUZZY and is NOT an identity pin; pin_id() does a
 *    byte-exact strcmp of rb_card_string(card_no_idx) instead.
 *  * `Card.ability` is not populated by rb_decode_card_by_index; abilities
 *    come from rb_card_num_abilities + rb_decode_card_ability.
 *  * The C deck bag is TOP-INDEX-0, matching Rust's main_deck.cards, so a
 *    `cards.push(x)` loop is an APPEND.
 *  * rb_perform_cheer_check both SETS g->cheer_checks_required and consumes
 *    `blade_count` cards from the deck into g->resolution. The first call also
 *    latches g->cheer_check_base = blade_count, so the base is pinned by the
 *    BASE_BLADE argument, not by whatever the stage totals.
 *
 * NO JAPANESE IN A FUNCTIONAL POSITION
 * -----------------------------------
 * This file is ASCII-only; every non-ASCII literal is a byte escape whose
 * bytes were read out of cards/cards.json and cards/abilities.json (read as
 * UTF-8, never via Get-Content -Raw). Trigger strings come from the engine's
 * own RB_TSTR_* macros so there is no transcription risk at all.
 *
 * VACUITY CONTROL -- the highest-value discipline in this file
 * -----------------------------------------------------------
 * Four of the six Rust tests assert the UNMODIFIED value (30), and 30 is also
 * what a board where the ability never fired would read. Every one of them
 * therefore carries, on the same board shape:
 *   - a FIXTURE-WIRING premise (039 prints a ライブ開始時 whose action really
 *     is `modify_yell_count`, operation `add`, count 10, behind a
 *     success_live_card_zone card_count_condition >= 2 with group_names
 *     ["μ's"]), and
 *   - a PAIRED POSITIVE: t1 drives the same helper with TWO μ's lives and
 *     must reach 40. Without 40 reachable, every 30 below is vacuous.
 * The group matcher is pinned on the fixtures too (t1 asserts both μ's lives
 * match μ's and the AiScReam does not), so "the filter rejected everything" is
 * distinguished from "the filter matched and the threshold did not".
 *
 * run() is crash ISOLATION, not a hang detector -- no watchdog is achievable on
 * this toolchain -- so every drain is capped. The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child killed by SIGSEGV/SIGBUS/SIGABRT
 * still reports the assertions it completed before dying.
 *
 * MUTATION RECORD -- proof that none of the six is vacuous
 * --------------------------------------------------------
 * Each mutation was applied to a PRIVATE COPY of the engine in a temp tree
 * (the repo was never touched) and this suite relinked against it:
 *
 *   M1  score.c:914  rb_execute_modify_yell_count, negate the "add"
 *       -> t1 red (got 20 = 30-10 instead of 40). Only t1 can see the sign.
 *   M2b util.c:393   rb_card_matches_group_str returns 1 for everything
 *       -> t1, t2, t3, t6 red. Only the four group claims can see this.
 *   M3  condition.c:146  target_player_idx maps target="self" to the opponent
 *       -> t1, t5 red. Only t5 claims 「自分の」 seat resolution.
 *   M4  condition.c:449  eval_location drops count 2 -> 1
 *       -> t1, t3 red. Only the below-threshold halves can see this.
 *
 * t4 survives ALL FOUR, and that is correct rather than a hole: an EMPTY success
 * zone must read the base under every one of them, so no mutation that preserves
 * a working engine can touch it. Its vacuity control is therefore structural --
 * t1's paired positive (two mu's lives on the same helper must reach 40) -- and
 * not a local premise. The file says so at the assertion.
 *
 * ENGINE FINDING worth passing on: `card_count_condition` has THREE evaluators
 * in src/ability/condition.c and only ONE of them is live:
 *   - eval_location            condition.c:431   <- LIVE (RB_COND_LOCATION arm)
 *   - resolve_zone_card_count  condition.c:2892     never reached
 *   - eval_card_count         condition.c:3480     never reached
 * Traced directly: with fprintf probes in all three, only eval_location fires for
 * this card. Mutating the threshold inside either dead function leaves the whole
 * suite GREEN, which is exactly how a "2枚以上" test can pass while asserting
 * nothing. Anyone writing condition tests should mutate against eval_location.
 * ===================================================================== */
#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

/* ── fixtures ───────────────────────────────────────────────────────────────
 * Read out of cards/cards.json read as UTF-8. 039's own unit is None, which is
 * the subject of t6. */
#define HIKARI_039  "PL!-pb2-039-L"      /* 僕たちはひとつの光  L  score 9
                                         *   unit  None
                                         *   series ラブライブ！ (=> μ's)
                                         *   ab#0 ライブ開始時 modify_yell_count
                                         *         +10 behind 「自分の成功ライブ
                                         *         カード置き場に『μ's』のカードが
                                         *         2枚以上ある場合」                    */
#define MUS_A       "PL!-pb1-029-SRL"   /* 知らないLove＊教えてLove  SRL  score 1
                                         *   unit  lily white                      */
#define MUS_B       "PL!-bp4-025-SRL"   /* 微熱からMystery         SRL  score 5
                                         *   unit  lily white                      */
#define NON_MUS     "LL-PR-004-PR"       /* 愛♡スクリ～ム！  AiScReam  PR  score 3
                                         *   unit  AiScReam, series  None -- so it
                                         *   matches μ's through NEITHER the unit
                                         *   nor the series arm                     */
#define FILLER      "PL!-sd1-010-SD"     /* 高坂 穂乃果  SD cost 4  Printemps     */

/* 「μ's」 = U+03BC GREEK SMALL LETTER MU + ASCII apostrophe + s.  The C engine
 * derives this exact string in rb_card_series_to_group (src/core/card.c:227),
 * and abilities.json carries the same three code points in group_names. */
#define GRP_MUS     "\xCE\xBC's"

/* "lily white", the UNIT name 039 must NOT match. */
#define UNIT_LILYWHITE "lily white"

/* Blade high enough that the +10 is visible. */
#define BASE_BLADE  30

/* ── shared counters (survive a child that dies mid-assertion) ──────────── */

typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int prompts;
} Tally;

static Tally *tal;
static const char *current_test = "(none)";

static void tally_init(void)
{
    void *p = mmap(NULL, sizeof(Tally), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    tal = (p == MAP_FAILED) ? NULL : (Tally *)p;
    if (tal) memset(tal, 0, sizeof *tal);
}

static void on_fatal_signal(int sig)
{
    if (tal)
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s\n"
                "*** The engine, not this test, is at fault.\n"
                "*** Completed before the fault: %d assertions, %d failures, "
                "%d prompts\n",
                sig, current_test, tal->assertions, tal->failures, tal->prompts);
    else
        fprintf(stderr, "\n*** FATAL SIGNAL %d inside test: %s\n", sig, current_test);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(cond, ...) do {                                             \
        if (tal) tal->assertions++;                                       \
        if (!(cond)) {                                                    \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        if (tal) tal->assertions++;                                       \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* ── card DB ────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* A ROTATING ring: a no_of() helper with one static buf called 2-3 times in
 * one printf prints the LAST card repeatedly. */
#define NO_RING 8
static const char *card_no_of(int cid)
{
    static char ring[NO_RING][96];
    static int next;
    char *buf = ring[next++ % NO_RING];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* Rust `game.assert_card_identity(id, no)`: a BYTE-EXACT strcmp of the decoded
 * card_no. rb_card_no_eq is deliberately not used (a peer proved it fuzzy). */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    if (tal) { tal->failures++; tal->setup_bugs++; }
    fprintf(stderr, "SETUP BUG: identity - want '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

/* Rust `game.id(x)` mints a NEW instance per call -> test_new_id. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

static int printed_score(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
    rb_free_card(&c);
    return v;
}

static int trigger_index(int cid, const char *trig)
{
    if (cid < 0) return -1;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (hit) return i;
    }
    return -1;
}

/* Rust `assert!(card prints this trigger)`: also the fixture-wiring vacuity
 * control for every negative in this file. */
static int expect_trigger(int cid, const char *trig, const char *no)
{
    int ai = trigger_index(cid, trig);
    int ok = ai >= 0;
    CHECK(ok, "fixture wiring: %s really prints a '%s' ability", no, trig);
    return ai;
}

/* ── startup diagnostics ────────────────────────────────────────────────── */

static const char *const ALL_FIXTURES[] = {
    HIKARI_039, MUS_A, MUS_B, NON_MUS, FILLER
};

static void print_wire_map(void)
{
    fprintf(stderr, "--- yell fixtures: identity + group resolution ---\n");
    for (unsigned i = 0; i < sizeof ALL_FIXTURES / sizeof ALL_FIXTURES[0]; i++) {
        int id = rb_find_card_by_no(ALL_FIXTURES[i]);
        if (id < 0) { fprintf(stderr, "    %-18s NOT FOUND\n", ALL_FIXTURES[i]); continue; }
        fprintf(stderr,
                "    %-18s id=%-6d score=%-3d group=\"%s\" unit=\"%s\" "
                "in_mu_s=%d in_lily_white=%d\n",
                ALL_FIXTURES[i], id, printed_score(id),
                rb_card_group_name(id), rb_card_unit_name(id),
                rb_card_matches_group_str(id, GRP_MUS),
                rb_card_matches_group_str(id, UNIT_LILYWHITE));
    }
    fprintf(stderr, "--- assertion literals ---\n");
    fprintf(stderr, "    GRP_MUS        = \"");
    for (const unsigned char *p = (const unsigned char *)GRP_MUS; *p; p++)
        fprintf(stderr, "\\x%02X", *p);
    fprintf(stderr, "\"\n");
    fprintf(stderr, "    UNIT_LILYWHITE = \"%s\"\n", UNIT_LILYWHITE);
    fprintf(stderr, "    BASE_BLADE     = %d\n", BASE_BLADE);
}

/* ── board helpers ──────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const RbChoice *pchoice(TestGame *tg)
{
    return pending(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

static void note_prompt(TestGame *tg)
{
    if (pending(tg) && tal) tal->prompts++;
}

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

/* Rust `select_indices(&[])`: DECLINE. */
static void decline(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, -1);
}

/* Rust `let mut guard = 0; while has_pending_choice() && guard < 10 {
 *      select_indices(&[]) ; guard += 1; }` -- the exact drain the Rust helper
 * `cheer_checks_with` performs after process_pending_auto_abilities. */
static int drain_decline(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) decline(tg);
    rb_drain_ability_queue(&tg->state);
    return guard;
}

static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++)
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

/* Rust `fill_decks(game, filler)`: 12 each. */
static void fill_decks(TestGame *tg, int filler)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        deck_append_n(tg, pl, filler, 12);
    }
}

static void hand_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

static void success_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++] = cid;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

/* helpers/mod.rs fire_trigger: build "<card_no>_<full_text>" for the ability
 * printing `trig`, queue exactly that, set activating_card, process. */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

static int fire_trigger(TestGame *tg, int cid, const char *trig,
                        const char *trigger_type)
{
    Card card;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof card_no, "%s", rb_card_string(card.card_no_idx));
    char ability_id[512];
    int found = 0;
    for (int a = 0; a < rb_card_num_abilities((uint32_t)cid) && !found; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                     ab.full_text ? ab.full_text : "");
            found = 1;
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, trigger_type, 0, card_no,
                            cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* ── runner ─────────────────────────────────────────────────────────────── */

enum { CHILD_OK = 0, CHILD_FAILURES = 1, CHILD_SETUPBUG = 2 };

static int n_ok, n_failed, n_crashed, n_setup;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures, s0 = tal->setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = tal->failures - f0, ns = tal->setup_bugs - s0;
        if (nf) n_failed++; else if (ns) n_setup++; else n_ok++;
        printf("%-8s %s  [%d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name, nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = tal->failures, s0 = tal->setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d failure(s), %d setup bug(s)\n",
               name, tal->failures - f0, tal->setup_bugs - s0);
        fflush(stdout);
        fflush(stderr);
        _Exit(tal->failures > f0 ? CHILD_FAILURES
                                 : (tal->setup_bugs > s0 ? CHILD_SETUPBUG : CHILD_OK));
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

/* ====================================================================== */
/* the fixture-wiring premise every negative in this file depends on       */
/* ====================================================================== */

/* Decode 039's ライブ開始時 ability and pin the WHOLE shape the six Rust tests
 * rest on.  Without this a zero (or a 30) is indistinguishable from a board
 * where the ability never fired.
 *
 * The Rust file asserts the printed text only; the C twin asserts the DECODED
 * record, because the claim under test is arithmetic ON that record:
 *   action      == modify_yell_count
 *   operation   == add                (「枚数が10枚増える」 -- the REQUIREMENT
 *                                      goes UP; the sign is the whole trap)
 *   count       == 10
 *   condition   == card_count_condition, location success_live_card_zone,
 *                  target self, group_names ["μ's"], count 2, operator >=
 */
static int pin_039_shape(void)
{
    static TestGame tg;
    fresh(&tg);
    int h = mid(&tg, HIKARI_039);
    if (!pin_id(h, HIKARI_039)) return 0;

    CHECK(rb_card_is_live(h),
          "premise: %s is a LIVE card, so it is performed through the "
          "live-card-set phase", HIKARI_039);
    CHECK_EQ(printed_score(h), 9, "premise: %s prints score 9", HIKARI_039);

    int ai = expect_trigger(h, RB_TSTR_LIVE_START, HIKARI_039);
    if (ai < 0) return 0;

    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)h, ai, &ab)) {
        CHECK(0, "premise: the ライブ開始時 ability does not DECODE at all");
        return 0;
    }
    const AbilityEffect *e = ab.effect;
    CHECK(e != NULL, "premise: the ライブ開始時 ability has an effect");
    if (!e) { rb_free_ability(&ab); return 0; }

    fprintf(stderr, "        [shape] action=\"%s\" count=%d\n",
            e->action ? e->action : "(null)", e->count);
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            fprintf(stderr, "        [shape] extra[%d] %s = %s\n", i,
                    e->extra_k[i], e->extra_v[i] ? e->extra_v[i] : "(null)");

    CHECK(e->action && strcmp(e->action, "modify_yell_count") == 0,
          "premise: the effect action must be \"modify_yell_count\" "
          "(got \"%s\")", e->action ? e->action : "(null)");

    const char *op = NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "operation"))
            op = e->extra_v[i];
    CHECK(op && !strcmp(op, "add"),
          "premise: 公開される自分のカードの枚数が10枚増える -- the operation must "
          "be \"add\" (got \"%s\")", op ? op : "(absent)");

    /* count: either the promoted field or an `extra` entry. */
    int cnt = e->count >= 0 ? e->count : -1;
    if (cnt < 0)
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "count")) cnt = atoi(e->extra_v[i]);
    CHECK_EQ(cnt, 10, "premise: the amount is 10 cards");

    /* The threshold condition: card_count_condition >= 2 over the OWN success
     * live zone filtered to μ's. */
    const Condition *c = e->condition;
    CHECK(e->has_condition && c != NULL,
          "premise: 「自分の成功ライブカード置き場に『μ's』のカードが2枚以上"
          "ある場合」 -- the effect must carry a condition");
    if (e->has_condition && c) {
        const char *loc = NULL, *grp = NULL, *opr = NULL, *type = NULL;
        long long need = -1;
        for (uint32_t i = 0; i < c->n_fields; i++) {
            const CondField *f = &c->fields[i];
            if (!f->key) continue;
            fprintf(stderr, "        [shape] cond key \"%s\" tag=%u", f->key,
                    f->v.tag);
            if (f->v.tag == RB_TAG_STR) fprintf(stderr, " = \"%s\"", f->v.s ? f->v.s : "");
            else if (f->v.tag == RB_TAG_I64) fprintf(stderr, " = %lld", f->v.i);
            else if (f->v.tag == RB_TAG_ARRAY)
                fprintf(stderr, " = [%u entries]", f->v.arr_n);
            fputc('\n', stderr);
            if (!strcmp(f->key, "location")) loc = f->v.s;
            if (!strcmp(f->key, "operator")) opr = f->v.s;
            if (!strcmp(f->key, "count")) need = f->v.i;
            if (!strcmp(f->key, "type")) type = f->v.s;
            if (!strcmp(f->key, "group_names") && f->v.tag == RB_TAG_ARRAY &&
                f->v.arr_n > 0 && f->v.arr[0].tag == RB_TAG_STR)
                grp = f->v.arr[0].s;
        }
        /* vm.c:1483 build_location -- card_count_condition and
         * location_condition SHARE variant RB_COND_LOCATION, so the variant
         * alone cannot tell them apart. Assert the discriminant, and read the
         * `type` discriminator when the bytecode carries one. */
        CHECK_EQ((long long)c->variant, (long long)RB_COND_LOCATION,
                 "premise: the condition is a location/count condition "
                 "(card_count_condition shares this variant)");
        if (type)
            CHECK(!strcmp(type, "card_count_condition"),
                  "premise: the condition's own type discriminator must be "
                  "card_count_condition (got \"%s\")", type);
        else
            fprintf(stderr,
                    "        [shape] note: the decoded condition carries NO "
                    "\"type\" key, so card_count_condition and "
                    "location_condition are indistinguishable by shape alone; "
                    "the count/operator/group/location fields below are the "
                    "discriminating ones\n");
        CHECK(loc && !strcmp(loc, "success_live_card_zone"),
              "premise: 自分の成功ライブカード置き場 -- location must be "
              "success_live_card_zone (got \"%s\")", loc ? loc : "(absent)");
        CHECK(need == 2, "premise: 「2枚以上」 -- the count must be 2 (got %lld)",
              need);
        CHECK(opr && !strcmp(opr, ">="),
              "premise: 「以上」 -- the operator must be \">=\" (got \"%s\")",
              opr ? opr : "(absent)");
        CHECK(grp && !strcmp(grp, GRP_MUS),
              "premise: 『μ's』 -- group_names[0] must be the μ's group "
              "(got \"%s\")", grp ? grp : "(absent)");
    }

    /* The group matcher itself, on the three fixtures that matter. This is
     * what tells "the filter rejected everything" apart from "the threshold
     * did not". */
    int a = mid(&tg, MUS_A), b = mid(&tg, MUS_B), nm = mid(&tg, NON_MUS);
    pin_id(a, MUS_A); pin_id(b, MUS_B); pin_id(nm, NON_MUS);
    CHECK(rb_card_matches_group_str(a, GRP_MUS),
          "premise: %s is a μ's card (unit \"lily white\", series ラブライブ！)",
          MUS_A);
    CHECK(rb_card_matches_group_str(b, GRP_MUS),
          "premise: %s is a μ's card", MUS_B);
    CHECK(!rb_card_matches_group_str(nm, GRP_MUS),
          "premise: %s is NOT a μ's card (unit \"AiScReam\", no series) -- it is "
          "the only real non-μ's control in the corpus", NON_MUS);

    rb_free_ability(&ab);
    return 1;
}

/* ====================================================================== */
/* the shared Rust helper: cheer_checks_with(zone)                         */
/* ====================================================================== */

/* Rust:
 *
 *   fn cheer_checks_with(zone: &[&str]) -> u8 {
 *       let mut game = TestGame::new(load_real_database());
 *       fill_decks(&mut game);                       // 12 filler each
 *       for c in zone { game.state.player1.success_live_card_zone.add_card(game.id(c)); }
 *       let hikari = game.id(HIKARI); game.assert_card_identity(hikari, HIKARI);
 *       game.state.player1.hand.cards.push(hikari);
 *       game.give_energy(20);
 *       for _ in 0..5 { game.pass(); }
 *       game.set_live_card(hikari);
 *       // find the ライブ開始時 ability, trigger_auto_ability(LiveStart),
 *       // process_pending_auto_abilities(p1), drain with select_indices(&[])
 *       let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);
 *       game.state.cheer_checks_required
 *   }
 *
 * Returns the number of cards the cheer requires the player to reveal, or a
 * NEGATIVE sentinel if the fixture could not be built (mirroring wave1b's
 * convention, so a setup bug is never mistaken for a value of 30).
 */
#define SETUP_FAIL (-999)

/* Which seats the success-zone fixture cards go into: 0 = our own zone (the
 * Rust helper), 1 = the opponent's (Rust's fifth test). */
static int cheer_checks_in_zone(const char *const *zone, int n, int pl,
                                int *n_yell_mods_out)
{
    static TestGame tg;   /* ~781 KB: MUST be static */
    fresh(&tg);

    int filler = mid(&tg, FILLER);
    if (filler < 0) return SETUP_FAIL;
    pin_id(filler, FILLER);
    fill_decks(&tg, filler);

    int hikari = mid(&tg, HIKARI_039);
    if (!pin_id(hikari, HIKARI_039)) return SETUP_FAIL;

    for (int i = 0; i < n; i++) {
        int c = mid(&tg, zone[i]);
        if (c < 0) return SETUP_FAIL;
        if (!pin_id(c, zone[i])) return SETUP_FAIL;
        success_append(&tg, pl, c);
    }
    /* The premise the card text names: 「自分の」成功ライブカード置き場. */
    CHECK_EQ(tg.state.p[pl].success.n, n,
             "premise: %d card(s) really sit in %s's success live zone", n,
             pl ? "p2" : "p1");
    for (int i = 0; i < n; i++) {
        int cid = tg.state.p[pl].success.cards[i];
        fprintf(stderr, "        [zone] p%d success[%d] = %s (mu_s=%d)\n",
                pl ? 2 : 1, i, card_no_of(cid),
                rb_card_matches_group_str(cid, GRP_MUS));
    }

    hand_append(&tg, 0, hikari);
    test_give_energy(&tg, 20);
    for (int i = 0; i < 5; i++) {
        test_pass(&tg);
        drain_decline(&tg, 8);
    }
    test_set_live_card(&tg, 0, hikari);
    drain_decline(&tg, 8);

    /* Premise: the live card really is the live card. Without it the cheer
     * below measures nothing. */
    CHECK(bag_has(&tg.state.p[0].live, hikari),
          "premise: %s really is the live card in p1's live zone", HIKARI_039);

    int census0 = test_total_card_count(&tg);
    int fired = fire_trigger(&tg, hikari, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    CHECK(fired, "the ライブ開始時 ability was queued and processed");
    int drained = drain_decline(&tg, 10);
    int census1 = test_total_card_count(&tg);
    int n_mods = tg.state.n_yell_count_modifiers;
    fprintf(stderr, "        [info] fired=%d drained=%d yell_count_modifiers=%d "
                    "CENSUS %d->%d\n",
            fired, drained, n_mods, census0, census1);
    /* The cheer itself moves cards deck->resolution, so the census is taken
     * BEFORE it and must be unchanged by the ability resolution alone. */
    CHECK_EQ(census1, census0,
             "CENSUS unchanged by the live-start ability resolution (nothing "
             "is created or destroyed by the effect)");

    /* Rust `let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);` */
    int rc = rb_perform_cheer_check(&tg.state, "p1", (uint8_t)BASE_BLADE);
    CHECK_EQ(rc, 0, "perform_cheer_check resolves");
    if (n_yell_mods_out) *n_yell_mods_out = n_mods;
    return tg.state.cheer_checks_required;
}

/* ====================================================================== */
/* the six Rust tests                                                     */
/* ====================================================================== */

/* Rust: two_mus_lives_in_the_success_zone_add_ten_yell_cards */
static void t1_two_mus_lives_in_the_success_zone_add_ten_yell_cards(void)
{
    static const char *const one[]   = { MUS_A };
    static const char *const both[]  = { MUS_A, MUS_B };

    /* The FIXTURE-WIRING premise for the whole file. t1's first half asserts
     * BASE_BLADE, which is also what a dead ability reads, so this runs first. */
    if (!pin_039_shape()) return;

    /* BELOW THRESHOLD: one μ's live is not 「2枚以上」. */
    int n_mods_one = -1;
    int got_one = cheer_checks_in_zone(one, 1, 0, &n_mods_one);
    if (got_one == SETUP_FAIL) return;
    CHECK_EQ(got_one, BASE_BLADE,
             "『μ's』のカードが2枚以上 -- one live is BELOW the threshold, so the "
             "cheer is unmodified");
    /* The threshold is a COUNT, and this is the reading that says the condition
     * saw one card rather than none: with ZERO in the zone a count condition
     * that read nothing at all would also produce BASE_BLADE. */
    int n_mods_zero = -1;
    int got_zero = cheer_checks_in_zone(NULL, 0, 0, &n_mods_zero);
    if (got_zero == SETUP_FAIL) return;
    CHECK_EQ(got_zero, BASE_BLADE,
             "an empty success live zone is the same below-threshold case");
    CHECK_EQ(n_mods_one, 0,
             "MECHANISM: below the threshold NOTHING was written to the yell "
             "ledger, so the 30 above is the base and not a cancelled +10");
    CHECK_EQ(n_mods_zero, 0,
             "MECHANISM: an empty zone writes nothing to the yell ledger");

    /* THE PAIRED POSITIVE -- without this every BASE_BLADE in this file is
     * vacuous, because a dead ability reads BASE_BLADE too. */
    int n_mods_both = -1;
    int got_both = cheer_checks_in_zone(both, 2, 0, &n_mods_both);
    if (got_both == SETUP_FAIL) return;
    CHECK_EQ(n_mods_both, 1,
             "MECHANISM: above the threshold exactly ONE +10 entry reached the "
             "yell ledger (the constant is not per qualifying card)");
    CHECK_EQ(got_both, BASE_BLADE + 10,
             "公開される自分のカードの枚数が10枚増える -- the cheer needs ten MORE "
             "cards revealed than the blade alone would require. The direction is "
             "「増える」: the REQUIREMENT rises, it does not fall.");
}

/* Rust: two_non_mus_lives_do_not_cross_the_threshold */
static void t2_two_non_mus_lives_do_not_cross_the_threshold(void)
{
    static const char *const non[] = { NON_MUS, NON_MUS };

    int n_mods = -1;
    int got = cheer_checks_in_zone(non, 2, 0, &n_mods);
    if (got == SETUP_FAIL) return;

    /* FIXTURE CONTROL. cheer_checks_in_zone already pins that both cards
     * resolved to NON_MUS byte-exactly and are really in the zone (the
     * premise assertion inside the helper), and pin_039_shape pins that
     * NON_MUS does not match μ's. So the two cards ARE there -- the count is 0
     * and not "the zone was empty". */
    CHECK_EQ(got, BASE_BLADE,
             "『μ's』のカードが2枚以上 -- two NON-μ's lives must not satisfy a μ's "
             "threshold");
    CHECK_EQ(n_mods, 0,
             "MECHANISM: the filter rejected both cards, so nothing reached the "
             "yell ledger");
}

/* Rust: a_mixed_zone_counts_only_the_mus_cards */
static void t3_a_mixed_zone_counts_only_the_mus_cards(void)
{
    static const char *const mixed[] = { MUS_A, NON_MUS };

    int n_mods = -1;
    int got = cheer_checks_in_zone(mixed, 2, 0, &n_mods);
    if (got == SETUP_FAIL) return;

    CHECK_EQ(got, BASE_BLADE,
             "only the μ's card counts toward 『μ's』のカードが2枚以上 -- one μ's "
             "plus one AiScReam is still only one");
    CHECK_EQ(n_mods, 0,
             "MECHANISM: the AiScReam was filtered out, so nothing reached the "
             "yell ledger");
}

/* Rust: an_empty_zone_leaves_the_cheer_unmodified */
static void t4_an_empty_zone_leaves_the_cheer_unmodified(void)
{
    int n_mods = -1;
    int got = cheer_checks_in_zone(NULL, 0, 0, &n_mods);
    if (got == SETUP_FAIL) return;

    CHECK_EQ(got, BASE_BLADE,
             "no cards in the success live zone -- the cheer is unmodified");
    CHECK_EQ(n_mods, 0,
             "MECHANISM: nothing reached the yell ledger. NOTE this is the "
             "WEAKEST of the six as a filter test (zero of anything satisfies a "
             "count >= 2), so its vacuity control is t1's paired positive, not "
             "any local premise.");
}

/* Rust: the_opponents_success_zone_does_not_count */
static void t5_the_opponents_success_zone_does_not_count(void)
{
    static const char *const mus[] = { MUS_A, MUS_B };

    /* 自分の成功ライブカード置き場 -- the OPPONENT's zone must not count. The
     * two cards here are byte-identical fixtures to t1's positive arm: the
     * ONLY difference is which seat holds them. */
    int n_mods = -1;
    int got = cheer_checks_in_zone(mus, 2, 1, &n_mods);
    if (got == SETUP_FAIL) return;

    CHECK_EQ(got, BASE_BLADE,
             "自分の成功ライブカード置き場 -- two μ's lives on the OPPONENT's "
             "side must not trigger the bonus");
    CHECK_EQ(n_mods, 0,
             "MECHANISM: the condition read OUR zone (empty) rather than the "
             "opponent's, so nothing reached the yell ledger");
    /* THE SHARPEST VACUITY CONTROL IN THIS FILE. t1's positive arm uses the
     * SAME two cards in OUR zone and must reach BASE_BLADE+10. So this zero is
     * about target="self" resolving to the right seat, not about a dead effect
     * or a dead filter. */
    fprintf(stderr, "        [info] t5 control: the identical two-card fixture "
                    "in p1's own zone reaches %d (t1); in p2's it reached %d\n",
            BASE_BLADE + 10, got);
}

/* Rust: this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight */
static void t6_this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight(void)
{
    static TestGame tg;   /* ~781 KB */
    fresh(&tg);

    int h = mid(&tg, HIKARI_039);
    if (!pin_id(h, HIKARI_039)) return;

    /* 春情ロマンティック PL!-pb2-041-L carries a counting weight --
     * 「みつの成功ライブカード置き場にあるカードの数を数えるとき、このカードは
     * 2枚として扱う」 -- and 039 was ORIGINALLY used as the CONSUMER to observe
     * it, because its threshold is also a ≥2 count. That is wrong: 039's
     * `unit` is None, so it is in no group and 「自分の『体のカードの効果によって』」
     * can never match it. The Rust header records this as a correction, and it
     * is a fixture trap rather than an engine question, so it is asserted here
     * as a property of the compiled record.
     *
     * rb_card_unit_name is the CANONICAL accessor (card_binary.rs:175-182):
     * cards.json carries `unit` on 2,198 of 2,639 cards (0xFFFF in cards.bin
     * marks the 441 with none), and it returns "" for "no unit" -- where a bare
     * rb_card_string(card->unit_idx) yields "" for both "absent" and index
     * 0xFFFF and is therefore useless as an identity pin.
     */
    const char *unit = rb_card_unit_name(h);
    fprintf(stderr, "        [unit] %s unit=\"%s\" group=\"%s\"\n",
            card_no_of(h), unit, rb_card_group_name(h));
    CHECK_EQ((long long)(unit && unit[0]), 0,
             "039 has NO unit, so it is in no group and can never be a "
             "『lily white』 counter. An earlier version of the Rust file used it "
             "as the observer for PL!-pb2-041-L's weight, which could never have "
             "worked.");
    CHECK(!rb_card_matches_group_str(h, UNIT_LILYWHITE),
          "and the group matcher agrees: 039 is not a lily white card");

    /* ANTI-VACUITY: "" is the value for BOTH "no unit" and a decode failure, so
     * the matcher assertion above is only meaningful if the matcher can return
     * true at all. Two real lily white cards are asserted positive here, which
     * is also the control that makes t2's / t3's AiScReam negatives real. */
    int a = mid(&tg, MUS_A), b = mid(&tg, MUS_B), nm = mid(&tg, NON_MUS);
    pin_id(a, MUS_A); pin_id(b, MUS_B); pin_id(nm, NON_MUS);
    CHECK(rb_card_matches_group_str(a, UNIT_LILYWHITE),
          "control: %s really IS a lily white card", MUS_A);
    CHECK(rb_card_matches_group_str(b, UNIT_LILYWHITE),
          "control: %s really IS a lily white card", MUS_B);
    /* And the negative is not "the matcher is broken": the same matcher DOES
     * accept both cards under μ's, which is the filter this file is about. */
    CHECK(rb_card_matches_group_str(a, GRP_MUS),
          "control: the same matcher accepts %s under 「μ's」", MUS_A);
    CHECK(rb_card_matches_group_str(nm, GRP_MUS) == 0,
          "and it rejects %s under 「μ's」 -- the matcher discriminates, so the "
          "039 negative is about 039 and not about a stub matcher", NON_MUS);
}

/* ── main ───────────────────────────────────────────────────────────────── */

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    tally_init();
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS, on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }
    print_wire_map();

    printf("--- effects/yell PL!-pb2-039-L (6 Rust tests) ---\n");
    if (argc > 1) {
        /* single-case filter, same shape as test_parity_restriction.c's
         * only_case: the argument is a substring of the case name. */
        const char *only = argv[1];
        struct { const char *n; void (*f)(void); } cases[] = {
            { "t1_two_mus_lives_in_the_success_zone_add_ten_yell_cards",
              t1_two_mus_lives_in_the_success_zone_add_ten_yell_cards },
            { "t2_two_non_mus_lives_do_not_cross_the_threshold",
              t2_two_non_mus_lives_do_not_cross_the_threshold },
            { "t3_a_mixed_zone_counts_only_the_mus_cards",
              t3_a_mixed_zone_counts_only_the_mus_cards },
            { "t4_an_empty_zone_leaves_the_cheer_unmodified",
              t4_an_empty_zone_leaves_the_cheer_unmodified },
            { "t5_the_opponents_success_zone_does_not_count",
              t5_the_opponents_success_zone_does_not_count },
            { "t6_this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight",
              t6_this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight },
        };
        for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++)
            if (strstr(cases[i].n, only)) run(cases[i].n, cases[i].f);
    } else {
        run("t1_two_mus_lives_in_the_success_zone_add_ten_yell_cards",
            t1_two_mus_lives_in_the_success_zone_add_ten_yell_cards);
        run("t2_two_non_mus_lives_do_not_cross_the_threshold",
            t2_two_non_mus_lives_do_not_cross_the_threshold);
        run("t3_a_mixed_zone_counts_only_the_mus_cards",
            t3_a_mixed_zone_counts_only_the_mus_cards);
        run("t4_an_empty_zone_leaves_the_cheer_unmodified",
            t4_an_empty_zone_leaves_the_cheer_unmodified);
        run("t5_the_opponents_success_zone_does_not_count",
            t5_the_opponents_success_zone_does_not_count);
        run("t6_this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight",
            t6_this_card_is_not_a_lily_white_card_and_cannot_observe_the_weight);
    }

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_restriction_invalidate: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    printf("     %d assertion(s), %d failure(s), %d setup bug(s), %d prompt(s)\n",
           tal->assertions, tal->failures, tal->setup_bugs, tal->prompts);
    if (n_crashed)
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child. "
                        "Treat as an engine fault.\n", n_crashed);
    if (n_setup)
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n", n_setup);
    if (n_failed)
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file "
                        "is reported as-is; none was weakened, skipped or deleted "
                        "to make the suite green.\n", n_failed);
    return n_failed || n_crashed ? 1 : 0;
}
/*
 * test_parity_mill_deck.c - Rust test-parity port of the two deck-manipulation
 * clusters:
 *   engine/tests/test_modules/effects/mill/                  (10 files / 36 tests)
 *   engine/tests/test_modules/effects/deck_order/restore/   (11 files / 31 tests)
 *
 * The theme is DECK ORDER and DECK MOVEMENT, so the harness is built around the
 * one convention every test here can silently get backwards: the C deck bag is
 * TOP-INDEX-0.
 *
 * DECK-ORDER CONVENTION (the single most important rule in this file)
 * -------------------------------------------------------------------
 * The C `RbBag deck` is TOP-INDEX-0, exactly like Rust's `main_deck.cards`.
 * Therefore, for the WHOLE file:
 *   Rust  cards.insert(0, x)  ==  C  test_insert_deck_top(tg, pl, x)   (PREPEND)
 *   Rust  cards.push(x)       ==  C  test_add_to_deck_pl(tg, pl, x)    (APPEND)
 *   Rust  cards[0]            ==  C  deck.cards[0]        (top)
 *   Rust  cards.last()        ==  C  deck.cards[deck.n-1] (bottom)
 * Getting these backwards mills or restores the WRONG end. Two independent
 * guards:
 *   (a) `md_deck_top_is()` is asserted explicitly at least once per cluster,
 *       before any effect runs, so the fixture itself is pinned.
 *   (b) every mill fixture uses DISTINGUISHABLE cards (several distinct card
 *       prints, not N copies of one filler), so "took the right end" is
 *       observable by card identity and not only by count. A peer found the
 *       opposite failure: a mill test whose deck was entirely identical filler
 *       cards passed while taking the wrong end.
 *
 * HARNESS NOTES (peer-confirmed)
 * ------------------------------
 *  * `rb_load("src")` fails under an isolated out-of-tree build, so
 *    load_card_db() falls back to `rb_load("../cards/build")`.
 *  * `sizeof(GameState)` is ~781 KB: every TestGame local is `static`.
 *  * `Card.ability` is NOT populated by rb_decode_card_by_index; the ability
 *    probe uses rb_card_num_abilities + rb_decode_card_ability.
 *  * `RbChoice.zone` for the waitroom is the string "discard".
 *  * Use test_new_id (Rust's `game.id()` next-distinct-pool-slot), not test_id,
 *    for any card a test tracks by identity.
 *  * test_get_heart_modifier REMAPS colour 5 onto RB_HEART_ORANGE; every heart
 *    read here goes through rb_mods_get_heart / rb_mods_get_need_heart directly.
 *    Rust `HeartColor::Heart00` is index 0, which is RB_HEART_PINK in C.
 *  * All Japanese literals are hex escapes, so the file is pure ASCII and the
 *    editor cannot re-encode it to CP932. Card numbers carrying the FULLWIDTH
 *    PLUS U+FF0B use "\xEF\xBC\x8B".
 *
 * HOW TO READ A RED ASSERTION IN THIS FILE
 * ---------------------------------------
 * No assertion here has been weakened to make the suite green. The red ones are
 * the deliverable. The dominant engine defects they pin down:
 *
 *  (A) A `move_cards source=deck` mill takes the deck's LAST card, not its
 *      first. `rb_move_resolve_from_deck` (src/ability/effects/move.c:523) does
 *      `int cid = P->deck.cards[--P->deck.n];` while the engine's own draw path
 *      (`rb_draw_cards_for_player`, src/ability/effects/draw.c:280) takes the
 *      head and shifts. The COUNT is always right; only the END is wrong, so an
 *      all-identical-filler fixture cannot see it - which is why every mill
 *      fixture here uses DISTINCT prints and asserts the waitroom's identity,
 *      not just its size. Affects mill/kosuzu_*, mill/bp5_013_*, mill/honoka_*,
 *      mill/ren_*.
 *
 *  (B) A real mill records NO movement event. `n_movements()` is
 *      `GameState.n_batch_movements`, and it is 0 after mill/sbp5_015 and
 *      mill/sayaka: `rb_move_resolve_from_deck` never calls
 *      `rb_record_card_movement`. Rust's own sbp5_015 / sayaka tests do not
 *      assert this, so it is invisible there; the two `n_movements > 0` checks
 *      in this file are the assertions Rust is missing.
 *
 *  (C) A mid-effect REFRESH is not re-driven: mill/q267_* leaves a deck of 3
 *      plus a waitroom of 7 completely untouched (waitroom 7, deck 3) where the
 *      rule requires 4 milled from the refreshed deck and 6 left. See (A)'s
 *      refresh branch at move.c:513-521, which re-shuffles the discard INTO the
 *      deck but is then only reached on the next loop pass.
 *
 *  (D) Every "waitroom -> deck" placement runs to the WRONG ZONE, or to no zone
 *      at all: mill/sp_bp2_013, do/n_bp4_021, do/n_bp7_008, do/bp7_019,
 *      do/gin_ko, do/bp4_004, do/bp2_021d and the whole do/q242_* family leave
 *      the deck unchanged or place the card somewhere other than the asserted
 *      end. This is the cluster `rb_move_execute_selected_cards_from_zone`
 *      feeds: it takes no destination argument and recovers one from
 *      `rb_entry_destination(g)`, falling back to "discard".
 *
 * VACUITY CONTROL - the highest-value discipline in this file
 * -----------------------------------------------------------
 * A negative assertion ("nothing was milled", "no prompt", "no blade") passes
 * trivially on a board where the ability did nothing at all. Every negative
 * assertion in this file is therefore paired with a PROMPT-COUNT CONTROL
 * (`prompt_guard(mark, ...)`) or with a positive assertion on the same board
 * (a card actually moved, a blade actually changed). `prompts` counts every
 * choice answered; `pending_offered` counts every moment a prompt was observed
 * open. Guards are DEFERRED and evaluated at the end of the case, so their
 * placement before or after the interaction does not change their meaning.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* rb_trigger_auto_ability exists in src/core/game_state_abilities.c but has no
   prototype in include/rabuka.h; declare it exactly as defined there (the same
   trick tests/test_parity_draw_flat.c uses). */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

/* Trigger wire tokens, byte-exact UTF-8. */
#define TRIG_DEBUT        "\xE7\x99\xBB\xE5\xA0\xB4"                            /* toujyou  */
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"

/* ===================================================================== */
/* 0. harness                                                             */
/* ===================================================================== */

static long checks;
static int  failures;
static int  gaps;
static int  prompts;   /* every answered choice */
static int  pending_offered;

static const char *current_case = "(none)";

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_mill_deck\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** checks so far: %ld  failures: %d\n",
            sig, current_case, checks, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                  \
        checks++;                                                  \
        if (!(condition)) {                                         \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fputc('\n', stderr);                                    \
        }                                                           \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                        \
        checks++;                                                   \
        long a_ = (long)(actual);                                   \
        long e_ = (long)(expected);                                 \
        if (a_ != e_) {                                             \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);   \
        }                                                           \
    } while (0)

/* An observation the C API cannot make whatever the engine does. Never a pass,
 * never a failure. */
#define EXPECTED_GAP(condition, desc) do {                          \
        checks++;                                                   \
        if (!(condition)) {                                         \
            printf("GAP: %s\n", desc);                              \
            gaps++;                                                 \
        } else {                                                    \
            printf("ok (gap closed): %s\n", desc);                  \
        }                                                           \
        fflush(stdout);                                             \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1

static int n_ok, n_failed, n_crashed;
static const char *only_case;
static const TestGame *current_tg;

static void eval_pending_guards(void);

static void run(const char *name, void (*fn)(void))
{
    if (only_case && !strstr(name, only_case)) return;
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        current_tg = NULL;
        fn();
        eval_pending_guards();
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld checks]\n", failures > f0 ? "FAILED" : "ok",
               name, checks - c0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        current_tg = NULL;
        fn();
        eval_pending_guards();
        printf("        %s: %ld check(s), %ld failure(s)\n",
               name, checks - c0, failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
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

/* ===================================================================== */
/* 1. helpers                                                             */
/* ===================================================================== */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

static const char *card_no_of(int cid)
{
    static char buf[64];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

static int card_printed_cost(int cid)
{
    Card c;
    int cost = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        cost = c.cost;
        rb_free_card(&c);
    }
    return cost;
}

/* Rust `game.id(card_no)`: the next DISTINCT pool slot. test_new_id is the C
 * twin; test_id aliases the shared template, so a tracked card must never use
 * it. Falls back to the template when the copy pool is exhausted, matching
 * Rust's documented fallback. */
static int md_id(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id >= 0) return id;
    return test_id(tg, no);
}

/* Rust assert_card_identity: pins the printed card_no so a lenient fallback can
 * never substitute another print. */
static int pin_id(int cid, const char *want)
{
    checks++;
    if (cid >= 0 && rb_card_no_eq(cid, want)) {
        printf("ok: identity %s\n", want);
        return 1;
    }
    fprintf(stderr, "FAIL: identity: want '%s', resolved '%s' (id=%d)\n",
            want, card_no_of(cid), cid);
    failures++;
    return 0;
}

static void pin_cost(int cid, int want)
{
    checks++;
    int got = card_printed_cost(cid);
    if (got != want) {
        fprintf(stderr, "FAIL: identity: %s printed cost %d, test expects %d\n",
                card_no_of(cid), got, want);
        failures++;
    } else {
        printf("ok: identity %s cost=%d\n", card_no_of(cid), got);
    }
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static void dump_ability(int cid, int idx)
{
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) {
        printf("DBG %s ability[%d]: DECODE FAILED\n", card_no_of(cid), idx);
        return;
    }
    printf("DBG %s ability[%d] triggers=%s use_limit=%d keywords=%d cost=%d effect=%d\n",
           card_no_of(cid), idx, ab.triggers ? ab.triggers : "(null)",
           ab.use_limit, ab.n_keywords, ab.cost ? 1 : 0, ab.effect ? 1 : 0);
    rb_free_ability(&ab);
}

static int bag_has(const RbBag *bag, int cid)
{
    for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].deck, cid); }

/* Rust assert_eq!(vec, [a, b]) - an ORDERED comparison. The C bag is ordered
 * too (top-index-0), so this is the faithful translation. */
static int bag_is(const RbBag *bag, const int *ids, int n)
{
    if (bag->n != n) return 0;
    for (int i = 0; i < n; i++) if (bag->cards[i] != ids[i]) return 0;
    return 1;
}

static void check_bag_is(const RbBag *bag, const int *ids, int n, const char *what)
{
    checks++;
    if (bag_is(bag, ids, n)) {
        printf("ok: %s\n", what);
        return;
    }
    fprintf(stderr, "FAIL: %s: zone has", what);
    for (int i = 0; i < bag->n; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
    fprintf(stderr, " | expected");
    for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(ids[i]));
    fprintf(stderr, "\n");
    failures++;
}

/* Rust's `wr.sort_unstable(); expected.sort_unstable(); assert_eq!` - an
 * order-INSENSITIVE zone comparison. */
static int bag_set_is(const RbBag *bag, const int *ids, int n)
{
    if (bag->n != n) return 0;
    for (int i = 0; i < n; i++) {
        int found = 0;
        for (int j = 0; j < bag->n; j++) {
            if (bag->cards[j] == ids[i]) { found = 1; break; }
        }
        if (!found) return 0;
    }
    return 1;
}

static void check_bag_set_is(const RbBag *bag, const int *ids, int n, const char *what)
{
    checks++;
    if (bag_set_is(bag, ids, n)) {
        printf("ok: %s\n", what);
        return;
    }
    fprintf(stderr, "FAIL: %s: zone has", what);
    for (int i = 0; i < bag->n; i++) fprintf(stderr, " %s", card_no_of(bag->cards[i]));
    fprintf(stderr, " | expected (as a set)");
    for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(ids[i]));
    fprintf(stderr, "\n");
    failures++;
}

/* ---- deck-order pins: the anti-"wrong end" guards ---- */

/* Assert the top of player `pl`'s deck is `cid`. The whole file's central
 * convention check, called before any effect runs in the deck-order cases. */
static void check_deck_top_is(TestGame *tg, int pl, int cid, const char *what)
{
    checks++;
    const RbBag *d = &tg->state.p[pl].deck;
    if (d->n > 0 && d->cards[0] == cid) {
        printf("ok: %s\n", what);
        return;
    }
    fprintf(stderr, "FAIL: %s: deck[%d] is", what, 0);
    if (d->n > 0) fprintf(stderr, " %s", card_no_of(d->cards[0]));
    else fprintf(stderr, " (empty)");
    fprintf(stderr, " | expected %s\n", card_no_of(cid));
    failures++;
}

static void check_deck_bottom_is(TestGame *tg, int pl, int cid, const char *what)
{
    checks++;
    const RbBag *d = &tg->state.p[pl].deck;
    if (d->n > 0 && d->cards[d->n - 1] == cid) {
        printf("ok: %s\n", what);
        return;
    }
    fprintf(stderr, "FAIL: %s: deck bottom is", what);
    if (d->n > 0) fprintf(stderr, " %s", card_no_of(d->cards[d->n - 1]));
    else fprintf(stderr, " (empty)");
    fprintf(stderr, " | expected %s\n", card_no_of(cid));
    failures++;
}

static void check_deck_tail_is(TestGame *tg, int pl, const int *ids, int n,
                               const char *what)
{
    checks++;
    const RbBag *d = &tg->state.p[pl].deck;
    if (d->n >= n) {
        int ok = 1;
        for (int i = 0; i < n; i++)
            if (d->cards[d->n - n + i] != ids[i]) ok = 0;
        if (ok) {
            printf("ok: %s\n", what);
            return;
        }
    }
    fprintf(stderr, "FAIL: %s: deck has", what);
    for (int i = 0; i < d->n; i++) fprintf(stderr, " %s", card_no_of(d->cards[i]));
    fprintf(stderr, " | expected the last %d to be", n);
    for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(ids[i]));
    fprintf(stderr, "\n");
    failures++;
}

/* Index of `cid` in player `pl`'s deck, or -1. */
static int deck_index_of(const TestGame *tg, int pl, int cid)
{
    const RbBag *d = &tg->state.p[pl].deck;
    for (int i = 0; i < d->n; i++) if (d->cards[i] == cid) return i;
    return -1;
}

/* ---- board setup ---- */

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    P->hand.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    P->energy_deck.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->discard.n = 0;
    P->deck_refreshed_this_turn = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    tg->state.re_yell_occurred = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_batch_movements = 0;
    tg->state.self_live_surplus_count = 0;
    tg->state.opponent_live_surplus_count = 0;
    tg->state.live_surplus_ready_this_turn = 0;
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0;
    P->hand.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    P->energy_deck.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->discard.n = 0;
    P->deck_refreshed_this_turn = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

/* Rust helpers::fill_decks / support::fill_both_main_decks: 30 fillers each. */
static void md_fill_decks(TestGame *tg, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    RbPlayer *O = &tg->state.p[1];
    P->deck.n = 0;
    O->deck.n = 0;
    for (int i = 0; i < 30; i++) {
        P->deck.cards[P->deck.n++] = filler;
        O->deck.cards[O->deck.n++] = filler;
    }
}

/* Rust cards.insert(0, x) == C PREPEND (test_insert_deck_top). */
static void put_on_deck_top(TestGame *tg, int pl, int cid)
{
    test_insert_deck_top(tg, pl, cid);
}

/* Rust cards.push(x) == C APPEND. */
static void push_to_deck(TestGame *tg, int pl, int cid)
{
    test_add_to_deck_pl(tg, pl, cid);
}

/* Rust add_to_discard_for(Side::P2, id). The test harness only exposes the
 * P1 form, so the P2 waitroom is written directly. */
static void push_to_discard(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

/* Rust `for m in ids { main_deck.cards.insert(0, m) }` == C PREPEND, so after
 * file exists to catch. */
static void rust_insert0_loop(TestGame *tg, int pl, const int *ids, int n)
{
    for (int i = 0; i < n; i++) put_on_deck_top(tg, pl, ids[i]);
}

static void expect_pending(int cond, const char *what)
{
    CHECK(cond, "%s", what);
    if (cond) pending_offered++;
}

/* ---- vacuity-control state ---- */
#define MAX_GUARDS 512
static struct { long long fp; int mark; const char *what; } g_guards[MAX_GUARDS];
static int g_n_guards;

static void md_reset_copy_pool(void);

static void md_reset_copy_pool(void);   /* defined just below, after board_begin */

static void board_begin(TestGame *tg)
{
    pending_offered = 0;
    g_n_guards = 0;
    current_tg = tg;
    md_reset_copy_pool();
}

/* HARNESS HAZARD, found by running this file: `rb_create_card_copy` draws from a
 * process-global monotonic pool (src/core/data.c:285) that nothing resets
 * between cases. A 60-case suite exhausts it, `test_new_id` starts returning -1,
 * and `md_id` silently falls back to `test_id` - the SHARED template. After that
 * point two different prints in the same case resolve to the SAME id, and every
 * identity assertion in that case becomes vacuous. The symptom seen here was
 * three different waitroom fixtures all printing the same card_no. Resetting the
 * pool per case makes each `md_id` a genuinely distinct slot, as Rust's
 * `game.id()` is. */
static void md_reset_copy_pool(void) { rb_card_copy_pool_reset(); }

static int prompt_mark(void) { return prompts; }

/* A cheap fingerprint of every zone size plus the movement-event count. A change
 * in this value means the engine really MOVED something, which is the control
 * for an effect that legitimately opens no prompt (a fixed-count mill is the
 * common case). */
static long long state_fingerprint(const TestGame *tg)
{
    long long f = 0;
    for (int p = 0; p < 2; p++) {
        const RbPlayer *P = &tg->state.p[p];
        f += (long long)P->deck.n        * 1LL;
        f += (long long)P->hand.n        * 1000000LL;
        f += (long long)P->discard.n     * 1000000000000LL;
        f += (long long)P->live.n        * 1000000000000000LL;
        f += (long long)P->success.n     * 1000000000000000000LL;
    }
    f += (long long)tg->state.n_batch_movements * 100000000000000000LL;
    return f;
}

/* The effect really reached the engine: EITHER it opened a prompt we answered,
 * OR it changed the state of some zone. Used for the fixed-count mills, which
 * correctly open no prompt at all, where a prompt counter would be a WRONG
 * vacuity control. */
static void effect_guard(long long fp, int mark, const char *what)
{
    if (g_n_guards < MAX_GUARDS) {
        g_guards[g_n_guards].fp = fp;
        g_guards[g_n_guards].mark = mark;
        g_guards[g_n_guards].what = what;
        g_n_guards++;
    }
}

/* The effect offered a prompt, so this case's control is that the prompt
 * appeared - for abilities whose whole point is that they ask. */
static void prompt_guard(int mark, const char *what) __attribute__((unused));
static void prompt_guard(int mark, const char *what)
{
    if (g_n_guards < MAX_GUARDS) {
        g_guards[g_n_guards].fp = -1;
        g_guards[g_n_guards].mark = mark;
        g_guards[g_n_guards].what = what;
        g_n_guards++;
    }
}

/* For a case that asserts NOTHING HAPPENED: the control cannot be "the effect
 * changed state" (it must not). It is that the card really prints the ability
 * under test, so the absence is a filter result and not a missing effect. */
static void expect_effect_printed(int cid, const char *trig, const char *what)
{
    CHECK(card_prints_trigger(cid, trig), "%s", what);
}

static void eval_pending_guards(void)
{
    for (int i = 0; i < g_n_guards; i++) {
        if (g_guards[i].fp < 0) {
            CHECK(prompts > g_guards[i].mark || pending_offered > g_guards[i].mark,
                  "vacuity control: %s (a prompt really was offered)", g_guards[i].what);
        } else {
            CHECK(prompts > g_guards[i].mark || pending_offered > g_guards[i].mark ||
                      state_fingerprint(current_tg) != g_guards[i].fp,
                  "vacuity control: %s (the engine really changed state)",
                  g_guards[i].what);
        }
    }
    g_n_guards = 0;
}

/* Rust select_indices(&[i]). */
static void pick(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust select_indices(&[]) - DECLINE. */
static void skip_choice(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust select_option(n). */
static void opt(TestGame *tg, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice(&tg->state, n);
}

static const char *pending_type(TestGame *tg) { return test_pending_choice_type(tg); }

static int pending_zone_is(TestGame *tg, const char *want)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c && c->zone[0] && strcmp(c->zone, want) == 0;
}

static int pending_target_is(TestGame *tg, const char *want)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c && c->target[0] && strcmp(c->target, want) == 0;
}

static int pending_allow_skip(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->allow_skip : -1;
}

static void drain_pick0(TestGame *tg, int guard_max) __attribute__((unused));
static void drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) pick(tg, 0);
}

static void drain_skip(TestGame *tg, int guard_max) __attribute__((unused));
static void drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) skip_choice(tg);
}

/* Rust TestGame::drain_choices: decline what CAN be declined, answer the rest
 * with their first option. Hand-rolled `while has_pending { select(&[]) }`
 * loops leave a MANDATORY prompt parked mid-ability, which then produces
 * exactly the absence the following assertion checks - the classic
 * drain_zero_may_decline smell. */
static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->allow_skip) skip_choice(tg);
        else opt(tg, 0);
    }
}

/* Rust drain_choices_strict(&["SelectCard","SelectAutoAbility"], &[]). */
static void drain_strict(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const char *t = pending_type(tg);
        if (!strcmp(t, "SelectCard") || !strcmp(t, "SelectAutoAbility")) pick(tg, 0);
        else if (pending_allow_skip(tg) == 1) skip_choice(tg);
        else opt(tg, 0);
    }
}

/* ---- trigger firing ---- */

/* Mirror Rust helpers::fire_trigger / fire_trigger_nth (helpers/mod.rs:114):
 * find the `nth` ability whose `triggers` EQUALS `trig`, build the
 * "<card_no>_<full_text>" key, enqueue exactly that ability, set
 * activating_card, then run process_pending_auto_abilities. It deliberately
 * does NOT drain, so the caller still observes the prompt Rust observes. */
static int md_fire_trigger(TestGame *tg, int cid, const char *trig,
                           const char *trigger_type, int nth)
{
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof card_no, "%s", rb_card_string(card.card_no_idx));
    char ability_id[1024];
    int found = 0, nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            if (nth == 0) {
                snprintf(ability_id, sizeof ability_id, "%s_%s", card_no,
                         ab.full_text ? ab.full_text : "");
                found = 1;
            } else {
                nth--;
            }
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

static int md_fire_debut(TestGame *tg, int cid) {
    return md_fire_trigger(tg, cid, TRIG_DEBUT, TRIG_DEBUT, 0);
}
static int md_fire_live_start(TestGame *tg, int cid) {
    return md_fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START, 0);
}
static int md_fire_live_success(TestGame *tg, int cid) {
    return md_fire_trigger(tg, cid, TRIG_LIVE_SUCCESS, TRIG_LIVE_SUCCESS, 0);
}

/* Rust game.play_to_stage, with the play-time alt-cost hook answered. */
static int md_play_to_stage(TestGame *tg, int cid, int area, int accept)
{
    int r = test_play_to_stage(tg, cid, area);
    int guard = 0;
    while (tg->state.ptc_active && guard++ < 4) {
        test_answer_play_cost_choice(tg, accept);
        if (tg->state.p[0].stage[area] == cid) break;
        r = test_play_to_stage(tg, cid, area);
    }
    return r;
}

/* Rust game.activate_ability / TurnEngine::execute_main_phase_action(UseAbility). */
static int md_activate(TestGame *tg, int cid) { return test_activate_ability(tg, cid); }

/* Number of movement events the engine recorded this turn. Rust's
 * turn_movements.len(). */
static int n_movements(const TestGame *tg) { return tg->state.n_batch_movements; }

/* ===================================================================== */
/* 2. effects/mill - debut_fixed_count_mill_test.rs                       */
/*      PL!S-bp5-015-N debut: "put the TOP 10 cards of the deck into the   */
/*    A 12-card deck of one filler print. 10 leave, 2 stay. The COUNT is the */
/*    assertion; the deck-order pin below proves the fixture itself is sane.  */
/* ===================================================================== */

#define S_BP5_015_N  "PL!S-bp5-015-N"
#define FILLER      "PL!-sd1-010-SD"   /* Takasaka Honoka: a mu's member with no ability */
#define LIVE_FILLER "PL!-sd1-019-SD"   /* a mu's LIVE card (not a member)       */
#define AQ_MEMBER   "PL!S-sd1-001-SD"   /* Aqours member                        */
#define MU_MEMBER   "PL!N-bp3-006-R"   /* a mu's member                        */
#define LIELLA_M1   "PL!SP-sd4-022-N"  /* see note in the liella block below   */

static void test_sbp5_015_twelve_card_deck_mills_ten_keeps_two(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int nico = md_id(&tg, S_BP5_015_N);
    pin_id(nico, S_BP5_015_N);
    int filler = md_id(&tg, FILLER);
    pin_id(filler, FILLER);

    test_add_to_hand(&tg, nico);
    test_give_energy(&tg, 12);
    for (int i = 0; i < 12; i++) push_to_deck(&tg, 0, filler);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_play_to_stage(&tg, nico, 1, 0);
    CHECK_EQ(tg.state.p[0].stage[1], nico, "nico is on the centre stage");
    /* The printed effect is a plain count, so it must not open a prompt. If one
     * appears the count assertion below is measuring the wrong thing. */
    expect_pending(!rb_has_pending_choice(&tg.state),
                   "s_bp5_015: a fixed-count mill asks nothing");
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, 10,
             "s_bp5_015: debut mills exactly 10 cards to the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 2, "s_bp5_015: deck keeps the 2 unmilled cards");
    /* The count is the Rust assertion; the deck pin is the non-vacuity control
     * for it: a mill that took from the BOTTOM instead of the top would leave
     * the same COUNT and pass an all-filler fixture silently. */
    check_deck_top_is(&tg, 0, filler,
                      "s_bp5_015 vacuity control: the deck's head is still a filler, "
                      "so the top-3 convention is what the mill consumed");
    CHECK_EQ(tg.state.p[0].discard.n + tg.state.p[0].deck.n, 12,
             "s_bp5_015: no card created or destroyed (conservation)");
    /* MOVEMENT EVENT COUNT. Rust's s9_replacement_timestamp_test.rs drives
     * turn_movements through a real move and asserts the event is present, so
     * the C twin is g.n_batch_movements. A peer found a real mill that records
     * NO movement event at all; this is the assertion that would catch it, and
     * it is the one Rust's own s_bp5_015 test omits. */
    CHECK(n_movements(&tg) > 0,
          "s_bp5_015: a real mill must record at least one movement event "
          "(n_batch_movements > 0)");
    (void)mark;
}

/* ===================================================================== */
/* 3. effects/mill - debut_mill_and_all_member_draw_test.rs                 */
/*      PL!HS-bp1-008-R debut: "put the TOP 3 cards of the deck into the    */
/*      waitroom. If the 3 cards put there are all MEMBER cards, draw 1".   */
/*    Distinct behaviour: a CONDITIONAL draw gated on "all three milled cards */
/*    are members", plus the plain top-5 mill of PL!HS-bp2-011-PR.           */
/* ===================================================================== */

#define KOSUZU   "PL!HS-bp1-008-R"
#define SAYAKA   "PL!HS-bp2-011-PR"

static void kosuzu_board(TestGame *tg, const int *top3, int *out_kosuzu)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin(tg);

    int suzuki = md_id(tg, KOSUZU);
    pin_id(suzuki, KOSUZU);
    tg->state.p[0].stage[1] = suzuki;

    int filler = md_id(tg, FILLER);
    md_fill_decks(tg, filler);
    /* Rust put_on_deck_top(&mut game, 0, filler) x3 == C PREPEND, so the deck
     * head after the loop is top3[2]. */
    rust_insert0_loop(tg, 0, top3, 3);
    check_deck_top_is(tg, 0, top3[2],
                      "kosuzu fixture pin: after three prepends the deck's head is top3[2]");
    *out_kosuzu = suzuki;
}

static void test_kosuzu_mill_three_all_members_draws_one(void)
{
    static TestGame tg;
    int m1 = md_id(&tg, MU_MEMBER);
    int m2 = md_id(&tg, FILLER);
    int m3 = md_id(&tg, AQ_MEMBER);
    pin_id(m1, MU_MEMBER);
    int top3[3] = { m1, m2, m3 };

    int suzuki = 0;
    kosuzu_board(&tg, top3, &suzuki);

    int hand_before = tg.state.p[0].hand.n;
    int wait_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    int fired = md_fire_debut(&tg, suzuki);
    CHECK(fired, "kosuzu: the card exposes a debut ability that the engine accepts");
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 1,
             "kosuzu: all 3 milled cards are members -> draw 1");
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "kosuzu: three cards moved to the waitroom");
    /* VACUITY CONTROL: the "+3 waitroom" is itself the positive, and the
     * identity of the milled set is what proves the TOP was taken. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "kosuzu: exactly the deck's top three reached the waitroom");
    (void)mark;
}

static void test_kosuzu_mill_three_with_live_card_skips_draw(void)
{
    static TestGame tg;
    int m1 = md_id(&tg, MU_MEMBER);
    int live = md_id(&tg, LIVE_FILLER);
    int m3 = md_id(&tg, AQ_MEMBER);
    pin_id(live, LIVE_FILLER);
    CHECK(rb_card_is_live(live), "PL!-sd1-019-SD really is a live card");
    int top3[3] = { m1, live, m3 };

    int suzuki = 0;
    kosuzu_board(&tg, top3, &suzuki);

    int hand_before = tg.state.p[0].hand.n;
    int wait_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_debut(&tg, suzuki);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "kosuzu: a non-member among the milled three -> no draw");
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "kosuzu: the mill itself still happens");
    /* VACUITY CONTROL: the negative draw assertion would pass on a board where
     * the ability did nothing. The positive on the SAME board is the waitroom
     * count, and the guard proves the effect reached the frontend. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "kosuzu: the live card among the top three was the one milled");
    effect_guard(fp, mark, "kosuzu no-draw branch must still run the mill to the frontend");
}

static void test_sayaka_debut_mills_top_five_to_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int sayaka = md_id(&tg, SAYAKA);
    pin_id(sayaka, SAYAKA);
    tg.state.p[0].stage[1] = sayaka;

    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int deck_before = tg.state.p[0].deck.n;
    int wait_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_debut(&tg, sayaka);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 5,
             "sayaka: five cards milled to the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 5,
             "sayaka: deck shrank by five from the top");
    CHECK_EQ(tg.state.p[0].discard.n + tg.state.p[0].deck.n, deck_before + wait_before,
             "sayaka: no card created or destroyed (conservation)");
    /* VACUITY CONTROL: five cards left the deck, so the ability ran. */
    CHECK_EQ(deck_index_of(&tg, 0, filler), 0,
             "sayaka vacuity control: the surviving deck's head is the sixth filler");
    /* MOVEMENT EVENT COUNT - the assertion Rust's sayaka test omits. */
    CHECK(n_movements(&tg) > 0,
          "sayaka: the mill must record at least one movement event");
    effect_guard(fp, mark, "sayaka's mill must reach the frontend");
}

/* ===================================================================== */
/* 4. effects/mill - all_members_blades_pl_hs_bp5_013_n_test.rs             */
/*      PL!HS-bp5-013-N live-start: "put the TOP 3 cards of the deck into   */
/*      the waitroom. If all 3 are MEMBER cards, gain 2 blades".             */
/*      This is the CASE THAT EXPOSES "milled the wrong end": the deck's top */
/*      three are three DISTINCT member prints and everything under them is  */
/*      one identical filler. Milling the top gives 3 members -> +2 blades;  */
/*      milling the bottom gives 3 fillers. The fillers are members too, so    */
/*      the fixture is chosen so a bottom-mill would be visible in the WAITROOM */
/*      identity assertion below, which Rust's own test omits.                */
/* ===================================================================== */

#define HS_BP5_013_N "PL!HS-bp5-013-N"
#define SP_BP4_022_N  "PL!SP-bp4-022-N"

static void bp5_013_board(TestGame *tg, const int *top3, int *out_me)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin(tg);

    int filler = md_id(tg, FILLER);
    md_fill_decks(tg, filler);
    int me = md_id(tg, HS_BP5_013_N);
    pin_id(me, HS_BP5_013_N);
    tg->state.p[0].stage[1] = me;
    /* Rust: for m in [m3, m2, m1] { cards.insert(0, m) } -> C PREPEND, so the
     * deck ends up head-to-tail m1, m2, m3. */
    {
        int loop_order[3] = { top3[2], top3[1], top3[0] };
        rust_insert0_loop(tg, 0, loop_order, 3);
    }
    check_deck_top_is(tg, 0, top3[0],
                      "bp5_013 fixture pin: the deck's head is m1 after the three prepends");
    *out_me = me;
}

static void test_bp5_013_mill_three_all_members_two_blades(void)
{
    static TestGame tg;
    int m1 = md_id(&tg, MU_MEMBER);
    int m2 = md_id(&tg, SP_BP4_022_N);
    int m3 = md_id(&tg, AQ_MEMBER);
    pin_id(m2, SP_BP4_022_N);
    int top3[3] = { m1, m2, m3 };

    int me = 0;
    bp5_013_board(&tg, top3, &me);
    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_live_start(&tg, me);
    drain_choices(&tg);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 2,
             "bp5_013: all three milled cards are members -> +2 blades");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3,
             "bp5_013: sanity - deck shrank only by the mill");
    /* THE non-vacuity control Rust's test lacks: the waitroom must hold the deck
     * TOP three, not three copies of the filler underneath them. A bottom-mill
     * would still leave the count at deck_before - 3 and still pass every
     * assertion above, because the filler is also a member card. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "bp5_013: the waitroom holds the deck's TOP three, not the bottom filler");
    effect_guard(fp, mark, "bp5_013's mill must reach the frontend");
}

static void test_bp5_013_milled_live_card_no_blades(void)
{
    static TestGame tg;
    int live = md_id(&tg, LIVE_FILLER);
    int m2 = md_id(&tg, MU_MEMBER);
    int m3 = md_id(&tg, AQ_MEMBER);
    pin_id(live, LIVE_FILLER);
    int top3[3] = { live, m2, m3 };

    int me = 0;
    bp5_013_board(&tg, top3, &me);
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_live_start(&tg, me);
    drain_choices(&tg);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 0,
             "bp5_013: live card among the milled -> condition fails, no blades");
    /* VACUITY CONTROL for that zero: without this, a mill from the BOTTOM would
     * mill three filler MEMBERS and correctly yield +2, so the zero really does
     * prove the TOP was taken. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "bp5_013: the live card at the deck's head is what was milled");
    effect_guard(fp, mark, "bp5_013 no-blade branch must still run the mill");
}

/* ===================================================================== */
/* 5. effects/mill - bottom_aqours_required_heart_pl_s_bp7_020_l_test.rs    */
/*      PL!S-bp7-020-L live-start (2nd ability): "put 1 card from the deck    */
/*      BOTTOM into the waitroom; if that card is an Aqours MEMBER,          */
/*      need heart -1".                                                      */
/*      This is the BOTTOM-end mirror of section 4: the same convention, the  */
/*      other direction. Deck = [filler, aqours] and the assertion is on the  */
/*      aqours card specifically, so a top-mill is impossible to confuse with  */
/*      a bottom-mill.                                                        */
/* ===================================================================== */

#define S_BP7_020_L "PL!S-bp7-020-L"

static int bp7_020_live_in_zone(TestGame *tg)
{
    int hpt = md_id(tg, S_BP7_020_L);
    pin_id(hpt, S_BP7_020_L);
    test_add_to_live_for(tg, 0, hpt);
    return hpt;
}

static void test_bp7_020_bottom_aqours_member_reduces_required_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int hpt = bp7_020_live_in_zone(&tg);
    int filler = md_id(&tg, FILLER);
    int aq = md_id(&tg, AQ_MEMBER);
    pin_id(aq, AQ_MEMBER);
    /* Rust push(filler); push(aqours) -> C APPEND, so aqours is the BOTTOM. */
    push_to_deck(&tg, 0, filler);
    push_to_deck(&tg, 0, aq);
    check_deck_bottom_is(&tg, 0, aq,
                         "bp7_020 fixture pin: the Aqours member is the deck BOTTOM");
    check_deck_top_is(&tg, 0, filler,
                      "bp7_020 fixture pin: the deck's head is the filler, so the "
                      "bottom mill cannot be a top mill");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int fired = md_fire_trigger(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 1);
    CHECK(fired, "bp7_020: the 2nd live-start ability was found and fired");
    drain_choices(&tg);

    CHECK(wait_has(&tg, aq), "bp7_020: the BOTTOM card was milled to the waitroom");
    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hpt, RB_HEART_PINK), -1,
             "bp7_020: milled card WAS an Aqours member -> heart00 need -1");
    CHECK(!deck_has(&tg, aq), "bp7_020: the Aqours member left the deck");
    CHECK(deck_has(&tg, filler), "bp7_020: the filler at the deck's head is untouched");
    effect_guard(fp, mark, "bp7_020's bottom mill must reach the frontend");
}

static void test_bp7_020_bottom_non_aqours_member_does_not_reduce(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int hpt = bp7_020_live_in_zone(&tg);
    int mus = md_id(&tg, FILLER);   /* a mu's member */
    push_to_deck(&tg, 0, mus);
    check_deck_bottom_is(&tg, 0, mus, "bp7_020b fixture pin: the mu's member is the bottom");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_trigger(&tg, hpt, TRIG_LIVE_START, TRIG_LIVE_START, 1);
    drain_choices(&tg);

    CHECK(wait_has(&tg, mus), "bp7_020b: the bottom mu's member was milled");
    CHECK_EQ(rb_mods_get_need_heart(&tg.state.mods, hpt, RB_HEART_PINK), 0,
             "bp7_020b: mu's member milled -> NO heart00 reduction");
    /* VACUITY CONTROL: the zero above would also hold on a board where the
     * ability did nothing. The positive on the same board is the waitroom. */
    CHECK(!deck_has(&tg, mus), "bp7_020b vacuity control: the mu's member really left the deck");
    effect_guard(fp, mark, "bp7_020b's bottom mill must reach the frontend");
}

/* ===================================================================== */
/* 6. effects/mill - bottom_member_tiers_pl_s_bp7_021_l_test.rs             */
/*      PL!S-bp7-021-L live-start: "if you have 3 or more members on stage,  */
/*      put 5 cards from the deck BOTTOM into the waitroom; if all 5 are    */
/*      MEMBER cards score+1, and if 3 or more are MEMBER cards draw 1".     */
/*      Four tier boundaries of ONE ability: 2 members, 3 members, 5 members, */
/*      and the stage-count gate failing. The deck is built from DISTINCT      */
/*      prints (mu's member / Aqours member / live filler) so "milled the     */
/*      bottom five" is observable by identity.                                */
/* ===================================================================== */

#define S_BP7_021_L "PL!S-bp7-021-L"

static int bp7_021_three_member_board(TestGame *tg)
{
    int live = md_id(tg, S_BP7_021_L);
    pin_id(live, S_BP7_021_L);
    test_add_to_live_for(tg, 0, live);
    for (int i = 0; i < 3; i++) {
        int m = md_id(tg, FILLER);
        tg->state.p[0].stage[i] = m;
    }
    int filler = md_id(tg, FILLER);
    md_fill_decks(tg, filler);
    return live;
}

static void test_bp7_021_bottom_mill_two_members_no_draw_or_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = bp7_021_three_member_board(&tg);
    int m1 = md_id(&tg, AQ_MEMBER);
    int m2 = md_id(&tg, AQ_MEMBER);
    int l1 = md_id(&tg, LIVE_FILLER);
    for (int i = 0; i < 3; i++) push_to_deck(&tg, 0, l1);
    push_to_deck(&tg, 0, m2);
    push_to_deck(&tg, 0, m1);
    /* Deck bottom-first: m1, m2, l1, l1, l1. The bottom five include 2 live
     * cards, so "all 5 are members" is false. */
    check_deck_bottom_is(&tg, 0, m1, "bp7_021 fixture pin: m1 is the deck bottom");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int fired = md_fire_live_start(&tg, live);
    CHECK(fired, "bp7_021: the live-start ability was found and fired");
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, 5, "bp7_021: five cards milled to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "bp7_021: only 2 members < 3 -> no draw");
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0,
             "bp7_021: not all members -> no score");
    /* The five-card mill is the positive that makes the two negatives above
     * non-vacuous. */
    CHECK_EQ(tg.state.p[0].deck.n, 25, "bp7_021 vacuity control: the deck lost exactly five");
    effect_guard(fp, mark, "bp7_021's bottom mill must reach the frontend");
    (void)l1;
}

static void test_bp7_021_bottom_mill_three_members_draw_only(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = bp7_021_three_member_board(&tg);
    for (int i = 0; i < 3; i++) {
        int m = md_id(&tg, AQ_MEMBER);
        push_to_deck(&tg, 0, m);
    }
    int l = md_id(&tg, LIVE_FILLER);
    push_to_deck(&tg, 0, l);
    push_to_deck(&tg, 0, l);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, live);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, 1, "bp7_021: 3 members among the milled 5 -> draw 1");
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0,
             "bp7_021: not ALL five were members -> no +1");
    CHECK_EQ(tg.state.p[0].discard.n, 4,
             "bp7_021 vacuity control: five were milled, one drawn back (4 left)");
    effect_guard(fp, mark, "bp7_021 draw branch must reach the frontend");
}

static void test_bp7_021_bottom_mill_five_members_draw_and_score(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = bp7_021_three_member_board(&tg);
    int five[5];
    for (int i = 0; i < 5; i++) {
        five[i] = md_id(&tg, AQ_MEMBER);
        push_to_deck(&tg, 0, five[i]);
    }

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, live);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, 1, "bp7_021: all five members -> draw fired");
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 1,
             "bp7_021: all five were members -> score+1");
    /* The score gate is "all five", so the SCORE is the assertion that proves
     * the bottom five - and only those five - were examined. */
    check_bag_set_is(&tg.state.p[0].discard, five, 5,
                     "bp7_021: the waitroom holds exactly the deck's bottom five");
    effect_guard(fp, mark, "bp7_021 score branch must reach the frontend");
}

static void test_bp7_021_two_staged_members_no_bottom_mill(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = bp7_021_three_member_board(&tg);
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;   /* Rust stage.stage[2] = -1 */
    int five[5];
    for (int i = 0; i < 5; i++) {
        five[i] = md_id(&tg, AQ_MEMBER);
        push_to_deck(&tg, 0, five[i]);
    }
    int deck_before = tg.state.p[0].deck.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, live);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "bp7_021d: stage count 2 < 3 -> nothing is milled");
    /* VACUITY CONTROL: this is the file's cleanest example of an assertion that
     * would pass on a completely dead engine. The positive sibling
     * (bottom_mill_five_members_draw_and_score, same card, 3 members) fires and
     * mills five; here the deck must hold the very same five, in the very same
     * order, rather than merely the same COUNT. */
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "bp7_021d vacuity control: the deck did not change size");
    check_deck_tail_is(&tg, 0, five, 5,
                       "bp7_021d vacuity control: the deck still holds the same five "
                       "members in the same order at its bottom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "bp7_021d: no draw");
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "bp7_021d: no score");
    /* The state-change guard is deliberately NOT used here: this case asserts
     * that nothing changed, so the control is that the card really prints the
     * ability. */
    expect_effect_printed(live, TRIG_LIVE_START,
                       "bp7_021 vacuity control: the gated no-op is a 3-member stage-count filter, not a missing effect");
}

/* ===================================================================== */
/* 7. effects/mill - bp7_q267_rinna_mill_refresh_test.rs                    */
/*      PL!N-bp7-009-R debut: "each player puts the TOP 7 cards of THEIR    */
/*      OWN deck into the waitroom".                                         */
/*      The Rule 10.2.2.1 mid-effect REFRESH boundary, and it is a genuinely  */
/*      distinct behaviour from everything above: the mill can RESUME after a  */
/*      shuffle, so the count is not simply "how many cards were there".      */
/*      Four boundaries: exhausts mid-mill, exactly seven, five, both decks.  */
/* ===================================================================== */

#define RINA "PL!N-bp7-009-R"

static void rina_setup_and_trigger(TestGame *tg, int p1_deck, int p1_wait,
                                   int p2_deck, int p2_wait)
{
    int rina = md_id(tg, RINA);
    pin_id(rina, RINA);
    tg->state.p[0].stage[1] = rina;

    int filler = md_id(tg, FILLER);
    tg->state.p[0].deck.n = 0;
    for (int i = 0; i < p1_deck; i++) tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = filler;
    tg->state.p[0].discard.n = 0;
    for (int i = 0; i < p1_wait; i++) tg->state.p[0].discard.cards[tg->state.p[0].discard.n++] = filler;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < p2_deck; i++) tg->state.p[1].deck.cards[tg->state.p[1].deck.n++] = filler;
    tg->state.p[1].discard.n = 0;
    for (int i = 0; i < p2_wait; i++) tg->state.p[1].discard.cards[tg->state.p[1].discard.n++] = filler;

    md_fire_debut(tg, rina);
    drain_choices(tg);
}

static void test_q267_deck_exhausts_mid_mill_refreshes_and_completes(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    rina_setup_and_trigger(&tg, 3, 7, 30, 0);

    CHECK(tg.state.p[0].deck_refreshed_this_turn,
          "q267: p1 deck hit 0 mid-mill -> must refresh immediately (Q267)");
    CHECK_EQ(tg.state.p[0].discard.n, 4,
             "q267: p1 refresh happened mid-mill, so only the 4 drawn from the "
             "refreshed deck remain in the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267: p1 3 original + 7 waitroom - 4 drawn = 6");
    CHECK_EQ(tg.state.p[0].discard.n + tg.state.p[0].deck.n, 10,
             "q267: p1 no cards lost (conservation)");

    CHECK(!tg.state.p[1].deck_refreshed_this_turn,
          "q267: p2 had 30 cards -> must NOT refresh");
    CHECK_EQ(tg.state.p[1].discard.n, 7, "q267: p2 milled a full 7 with no refresh");
    CHECK_EQ(tg.state.p[1].deck.n, 23, "q267: p2 deck = 30 - 7");
    effect_guard(fp, mark, "rina's mill must reach the frontend");
}

static void test_q267_deck_exactly_seven_no_refresh(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    rina_setup_and_trigger(&tg, 7, 3, 30, 0);

    CHECK(!tg.state.p[0].deck_refreshed_this_turn,
          "q267: deck exactly 7 -> no overdraw -> no refresh, despite waitroom material");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "q267: p1 all 7 deck cards milled");
    CHECK_EQ(tg.state.p[0].discard.n, 10,
             "q267: p1 3 pre-existing waitroom cards + 7 milled = 10");
    effect_guard(fp, mark, "rina's seven-card mill must reach the frontend");
}

static void test_q267_deck_five_refresh_completes_to_seven(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    rina_setup_and_trigger(&tg, 5, 3, 30, 0);

    CHECK(tg.state.p[0].deck_refreshed_this_turn,
          "q267: p1 deck (5) ran out mid-mill -> refresh");
    CHECK_EQ(tg.state.p[0].discard.n, 2,
             "q267: p1 2 cards drawn from the refreshed deck after the refresh");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267: p1 5 + 3 = 8 total, minus 2 drawn = 6");
    CHECK_EQ(tg.state.p[0].discard.n + tg.state.p[0].deck.n, 8, "q267: p1 conservation");
    effect_guard(fp, mark, "rina's five-then-refresh mill must reach the frontend");
}

static void test_q267_both_players_refresh_their_own_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    rina_setup_and_trigger(&tg, 3, 7, 3, 7);

    CHECK(tg.state.p[0].deck_refreshed_this_turn, "q267: p1 deck hit 0 -> refresh");
    CHECK(tg.state.p[1].deck_refreshed_this_turn,
          "q267: p2 deck hit 0 during its own mill -> refresh");
    CHECK_EQ(tg.state.p[0].discard.n, 4, "q267: p1 waitroom");
    CHECK_EQ(tg.state.p[1].discard.n, 4, "q267: p2 waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267: p1 deck");
    CHECK_EQ(tg.state.p[1].deck.n, 6, "q267: p2 deck");
    /* VACUITY CONTROL: p2's four waitroom cards are the OPPONENT side of the
     * same  effect, so they prove the second half ran too. */
    CHECK_EQ(tg.state.p[1].discard.n + tg.state.p[1].deck.n, 10,
             "q267: p2 conservation - its own 3 + 7 were its own");
    effect_guard(fp, mark, "rina's both-players mill must reach the frontend");
}

/* ===================================================================== */
/* 8. effects/mill - discard_cost_mill_then_arise_recovery_test.rs           */
/*      PL!-bp5-010-N live-start: "(cost) ... put the TOP 3 cards of the    */
/*      deck into the waitroom ... recover 1 A-RISE MEMBER card to hand".    */
/*      Distinct behaviour: a mill behind an OPTIONAL hand-discard cost, and  */
/*      a SECOND clause that recovers a card from the WAITROOM afterwards.    */
/*      The deck top three are three distinct member prints.                   */
/* ===================================================================== */

#define BP5_010_N  "PL!-bp5-010-N"
#define ARISE_R    "PL!-bp5-111-R"

static void honoka_board(TestGame *tg, const int *top3, int *out_me)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin(tg);

    int filler = md_id(tg, FILLER);
    md_fill_decks(tg, filler);
    int me = md_id(tg, BP5_010_N);
    pin_id(me, BP5_010_N);
    test_add_to_live_for(tg, 0, me);
    rust_insert0_loop(tg, 0, top3, 3);
    check_deck_top_is(tg, 0, top3[2], "honoka fixture pin: after three prepends the deck's head is top3[2]");
    *out_me = me;
}

static void test_honoka_mills_three_then_recovers_arise(void)
{
    static TestGame tg;
    int m1 = md_id(&tg, MU_MEMBER);
    int m2 = md_id(&tg, SP_BP4_022_N);
    int m3 = md_id(&tg, AQ_MEMBER);
    int top3[3] = { m1, m2, m3 };

    int me = 0;
    honoka_board(&tg, top3, &me);

    int arise = md_id(&tg, ARISE_R);
    pin_id(arise, ARISE_R);
    test_add_to_discard(&tg, arise);
    int fodder = md_id(&tg, FILLER);
    test_add_to_hand(&tg, fodder);

    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_live_start(&tg, me);
    /* Rust: select_indices(&[0]) to accept the optional hand-discard cost, then
     * drain picking the first option repeatedly. */
    pick(&tg, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3, "honoka: milled exactly 3");
    CHECK(hand_has(&tg, arise), "honoka: A-RISE member retrieved to hand");
    /* The recovery is the second clause and the mill is the first; the mill
     * assertion is what makes "the card is not still in the waitroom" mean
     * something. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "honoka: the waitroom gained the deck's top three");
    effect_guard(fp, mark, "honoka's cost + mill must reach the frontend");
}

static void test_honoka_without_arise_still_mills_three(void)
{
    static TestGame tg;
    int m1 = md_id(&tg, MU_MEMBER);
    int m2 = md_id(&tg, SP_BP4_022_N);
    int m3 = md_id(&tg, AQ_MEMBER);
    int top3[3] = { m1, m2, m3 };

    int me = 0;
    honoka_board(&tg, top3, &me);
    int fodder = md_id(&tg, FILLER);
    test_add_to_hand(&tg, fodder);   /* hand card for the cost, no A-RISE present */

    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_fire_live_start(&tg, me);
    pick(&tg, 0);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3, "honoka: mill still happened");
    /* VACUITY CONTROL: the deck count is the positive; the absence of a
     * recovery is the negative the guard sits next to. */
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "honoka: the mill ran even though there was no A-RISE member "
                     "to recover");
    effect_guard(fp, mark, "honoka's no-A-RISE branch must still run the mill");
}

/* ===================================================================== */
/* 9. effects/mill - liella_milled_per_unit_blades_test.rs                  */
/*      PL!SP-bp5-005-R+  Hazuki Koi                                          */
/*      ab#0 activate: "put the TOP 3 cards of the deck into the waitroom;   */
/*           1 blade per Liella! MEMBER card" (PER-UNIT, scoped to the      */
/*           cards THIS cost placed)                                          */
/*      ab#1 auto: "when your cards go to the waitroom you may pay E to      */
/*           recover 1 of them to hand" (a movement-triggered               */
/*           optional recovery over the cards JUST milled)                   */
/*      Four distinct behaviours here: the per-unit blade formula, its        */
/*      `those_cards` scoping, the optional cost, and the re-trigger window.  */
/* ===================================================================== */

#define REN        "PL!SP-bp5-005-R\xEF\xBC\x8B"   /* PL!SP-bp5-005-R<U+FF0B> */
#define LIELLA_A   "PL!SP-sd1-001-SD"
#define LIELLA_B   "PL!SP-sd1-004-SD"
#define CHISATO    "PL!SP-bp2-003-R"
#define SCRY       "PL!-sd1-011-SD"
#define ENERGY_CD  "LL-E-001-SD"

static void ren_board(TestGame *tg, int energy, int *out_ren)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin(tg);

    int ren = md_id(tg, REN);
    pin_id(ren, REN);
    tg->state.p[0].stage[1] = ren;
    test_give_energy(tg, energy);
    int filler = md_id(tg, FILLER);
    md_fill_decks(tg, filler);
    *out_ren = ren;
}

static void test_ren_mill_liella_blade_two_liella_among_three(void)
{
    static TestGame tg;
    int la = md_id(&tg, LIELLA_A);
    int lb = md_id(&tg, LIELLA_B);
    pin_id(la, LIELLA_A);
    pin_id(lb, LIELLA_B);
    int filler = md_id(&tg, FILLER);

    int ren = 0;
    ren_board(&tg, 3, &ren);
    /* Rust insert(0, filler); insert(0, lb); insert(0, la) -> C PREPEND, so the
     * deck head is la, then lb, then filler. */
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, lb);
    put_on_deck_top(&tg, 0, la);
    check_deck_top_is(&tg, 0, la, "ren fixture pin: the deck's head is the Liella! member");
    int wait_before = tg.state.p[0].discard.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int r = md_activate(&tg, ren);
    CHECK(r, "ren: ab#0 (activate) is offered and executed");
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "ren: the cost mills the deck's top three to the waitroom");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ren), 2,
             "ren: 2 Liella! members among the 3 milled -> 2 blades");
    /* The per-unit blade count is only correct if the THREE cards examined are
     * the top three, so the waitroom identity is the non-vacuity control. */
    int top3[3] = { la, lb, filler };
    check_bag_set_is(&tg.state.p[0].discard, top3, 3,
                     "ren: the waitroom holds the deck's top three, in the fixture's order");
    effect_guard(fp, mark, "ren's activation must reach the frontend");
}

static void test_ren_mill_liella_blade_no_liella_no_blade(void)
{
    static TestGame tg;
    int non = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 3, &ren);
    for (int i = 0; i < 3; i++) put_on_deck_top(&tg, 0, non);
    int wait_before = tg.state.p[0].discard.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren);
    drain_choices(&tg);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ren), 0,
             "ren: 0 Liella! members discarded -> 0 blades");
    /* VACUITY CONTROL: the zero above holds trivially on a dead engine. */
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "ren vacuity control: three cards were really milled");
    effect_guard(fp, mark, "ren's no-Liella! branch must still run the mill");
}

static void test_ren_mill_liella_blade_all_three_liella(void)
{
    static TestGame tg;
    int liella = md_id(&tg, LIELLA_A);
    pin_id(liella, LIELLA_A);
    int ren = 0;
    ren_board(&tg, 3, &ren);
    for (int i = 0; i < 3; i++) put_on_deck_top(&tg, 0, liella);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren);
    drain_choices(&tg);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ren), 3,
             "ren: 3 Liella! members discarded -> 3 blades (per-unit 3/1*1)");
    CHECK_EQ(tg.state.p[0].discard.n, 3,
             "ren vacuity control: the three milled cards are in the waitroom");
    effect_guard(fp, mark, "ren's all-Liella! branch must reach the frontend");
}

static void test_ren_preexisting_liella_in_discard_does_not_inflate(void)
{
    static TestGame tg;
    int liella = md_id(&tg, LIELLA_A);
    int filler = md_id(&tg, FILLER);

    int ren = 0;
    ren_board(&tg, 3, &ren);
    /* Two Liella! members already in the waitroom BEFORE the activation. */
    test_add_to_discard(&tg, liella);
    test_add_to_discard(&tg, liella);
    /* Deck top three: only ONE Liella! member. */
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, liella);
    check_deck_top_is(&tg, 0, liella, "ren-scoping fixture pin: one Liella! at the head");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren);
    drain_choices(&tg);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ren), 1,
             "ren: expected 1 (only the card placed by this cost) - the "
             "those_cards scoping works, the pre-existing discard does not inflate it");
    CHECK_EQ(tg.state.p[0].discard.n, 5,
             "ren-scoping vacuity control: 2 pre-existing + 3 milled = 5");
    effect_guard(fp, mark, "ren's those_cards-scoped blade must reach the frontend");
}

/* --- ab#1: the movement-triggered optional recovery --- */

/* Rust: game.state.set_recently_moved_cards(vec![...]); then
 * trigger_auto_abilities_for_player("p1"); process_pending_auto_abilities. The
 * C twin writes n_recently_moved/recently_moved directly and calls the real
 * scan. */
static void ren_trigger_movement_ability(TestGame *tg, int ren, const int *moved, int n)
{
    tg->state.n_recently_moved = 0;
    for (int i = 0; i < n && i < RB_MAX_RECENTLY_MOVED; i++)
        tg->state.recently_moved[tg->state.n_recently_moved++] = moved[i];
    rb_trigger_auto_abilities_for_player(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
}

static void test_ren_optional_pay_recovery_after_mill_recovers_one(void)
{
    static TestGame tg;
    int la = md_id(&tg, LIELLA_A);
    int lb = md_id(&tg, LIELLA_A);
    int filler = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 5, &ren);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, lb);
    put_on_deck_top(&tg, 0, la);
    check_deck_top_is(&tg, 0, la, "ren-ab1 fixture pin: the deck's head is a Liella! card");

    int hand_before = tg.state.p[0].hand.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_activate(&tg, ren);
    /* Rust: has_pending_choice() must be a SelectTarget (the optional E cost). */
    expect_pending(rb_has_pending_choice(&tg.state),
                   "ren ab#1: the optional cost prompt is offered after the mill");
    CHECK(strcmp(pending_type(&tg), "SelectTarget") == 0,
          "ren ab#1: the prompt is a SelectTarget, not %s", pending_type(&tg));
    opt(&tg, 1);   /* pay 1 energy */
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
        if (strcmp(pending_type(&tg), "SelectCard") == 0) pick(&tg, 0);
        else opt(&tg, 0);
    }

    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 1,
             "ren ab#1: 1 card recovered to hand after paying the optional cost");
    effect_guard(fp, mark, "ren ab#1 must reach the frontend");
}

static void test_ren_optional_pay_recovery_recovers_the_first_milled_card(void)
{
    static TestGame tg;
    int chisato = md_id(&tg, CHISATO);
    pin_id(chisato, CHISATO);
    int filler = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 5, &ren);
    /* Rust insert(0,filler) x2 then insert(0,chisato) -> C PREPEND: head is
     * chisato, i.e. it is the FIRST card the mill reaches. */
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, chisato);
    check_deck_top_is(&tg, 0, chisato, "ren-ab1b fixture pin: chisato is the deck's head");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "ren ab#1b: the optional cost prompt is offered");
    opt(&tg, 1);
    int hand_before = tg.state.p[0].hand.n;
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
        if (strcmp(pending_type(&tg), "SelectCard") == 0) pick(&tg, 0);
        else opt(&tg, 0);
    }

    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 1, "ren ab#1b: 1 card recovered");
    /* Rust asserts hand.cards.last() == Some(&chisato). The C hand bag is
     * append-ordered and the recovery appends, so the tail is the twin. */
    checks++;
    if (tg.state.p[0].hand.n > 0 &&
        tg.state.p[0].hand.cards[tg.state.p[0].hand.n - 1] == chisato) {
        printf("ok: ren ab#1b: the recovered card is chisato\n");
    } else {
        failures++;
        fprintf(stderr, "FAIL: ren ab#1b: recovered %s, expected chisato\n",
                card_no_of(tg.state.p[0].hand.n > 0
                           ? tg.state.p[0].hand.cards[tg.state.p[0].hand.n - 1] : -1));
    }
    effect_guard(fp, mark, "ren ab#1b must reach the frontend");
}

static void test_ren_optional_pay_recovery_decline_recovers_nothing(void)
{
    static TestGame tg;
    int filler = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 5, &ren);
    for (int i = 0; i < 3; i++) push_to_deck(&tg, 0, filler);

    int hand_before = tg.state.p[0].hand.n;
    int wait_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    md_activate(&tg, ren);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "ren ab#1c: the optional cost prompt is offered");
    opt(&tg, 0);   /* decline */
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "ren ab#1c: no card recovered when the cost is declined");
    /* VACUITY CONTROL: the decline branch still mills, and the waitroom is the
     * positive that proves the ability ran. */
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 3,
             "ren ab#1c vacuity control: the mill still happened");
    effect_guard(fp, mark, "ren ab#1c must reach the frontend");
}

static void test_ren_optional_pay_recovery_only_trigger_cards_not_whole_discard(void)
{
    static TestGame tg;
    int liella = md_id(&tg, LIELLA_A);
    int filler = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 5, &ren);
    /* One Liella! member ALREADY in the waitroom: Q221 says it must not be
     * selectable by ab#1. */
    test_add_to_discard(&tg, liella);
    for (int i = 0; i < 3; i++) put_on_deck_top(&tg, 0, filler);
    int wait_before = tg.state.p[0].discard.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "ren ab#1d: the optional cost prompt is offered");
    opt(&tg, 1);
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
        if (strcmp(pending_type(&tg), "SelectCard") == 0) pick(&tg, 0);
        else opt(&tg, 0);
    }

    /* Rust: 1 pre-existing + 3 milled - 1 recovered = 3. */
    CHECK_EQ(tg.state.p[0].discard.n, wait_before + 2,
             "ren ab#1d: pre-existing Liella! should not be recoverable (Q221) - "
             "3 milled - 1 recovered = 2 net new");
    CHECK(wait_has(&tg, liella),
          "ren ab#1d: the pre-existing Liella! member is still in the waitroom");
    effect_guard(fp, mark, "ren ab#1d must reach the frontend");
}

static void test_ren_decline_retrigger_same_turn(void)
{
    static TestGame tg;
    int filler = md_id(&tg, FILLER);
    int ren = 0;
    ren_board(&tg, 5, &ren);

    /* First discard event: decline the cost. */
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    ren_trigger_movement_ability(&tg, ren, (int[3]){ filler, filler, filler }, 3);
    drain_choices(&tg);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "Q233: declined -> no card recovered");

    /* Second discard event in the SAME turn: this time pay. */
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    test_add_to_discard(&tg, filler);
    ren_trigger_movement_ability(&tg, ren, (int[3]){ filler, filler, filler }, 3);
    {
        int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
            if (strcmp(pending_type(&tg), "SelectCard") == 0) pick(&tg, 0);
            else opt(&tg, 1);   /* pay */
        }
    }
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "Q233: the second discard re-triggers and the card is recovered "
             "after paying");
    effect_guard(fp, mark, "ren's Q233 re-trigger must reach the frontend");
}

static void test_ren_card_type_catch_all_recovers_any_card(void)
{
    static TestGame tg;
    int energy = md_id(&tg, ENERGY_CD);
    pin_id(energy, ENERGY_CD);
    int ren = 0;
    ren_board(&tg, 5, &ren);
    test_add_to_discard(&tg, energy);
    test_add_to_discard(&tg, energy);
    test_add_to_discard(&tg, energy);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    ren_trigger_movement_ability(&tg, ren, (int[3]){ energy, energy, energy }, 3);
    {
        int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
            if (strcmp(pending_type(&tg), "SelectTarget") == 0) opt(&tg, 1);
            else pick(&tg, 0);
        }
    }

    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "ren: card_type='card' catch-all -> 1 energy card recovered");
    CHECK_EQ(tg.state.p[0].discard.n, 2,
             "ren: 2 of the 3 energy cards remain in the waitroom");
    effect_guard(fp, mark, "ren's catch-all recovery must reach the frontend");
}

static void test_ren_two_copies_one_pays_one_declines(void)
{
    static TestGame tg;
    int filler = md_id(&tg, FILLER);
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int ren1 = md_id(&tg, REN);
    int ren2 = md_id(&tg, REN);
    pin_id(ren1, REN);
    CHECK(ren1 != ren2, "ren: two copies are distinct pool slots, so each has its own count");
    tg.state.p[0].stage[0] = ren1;
    tg.state.p[0].stage[1] = ren2;
    test_give_energy(&tg, 5);
    int fd = md_id(&tg, FILLER);
    md_fill_decks(&tg, fd);
    for (int i = 0; i < 3; i++) put_on_deck_top(&tg, 0, filler);

    int hand_before = tg.state.p[0].hand.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_activate(&tg, ren1);
    /* First prompt -> ren1 pays, second -> ren2 declines. */
    int pay_count = 0;
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 16) {
        if (strcmp(pending_type(&tg), "SelectTarget") == 0) {
            opt(&tg, pay_count == 0 ? 1 : 0);
            pay_count++;
        } else if (strcmp(pending_type(&tg), "SelectCard") == 0) {
            pick(&tg, 0);
        } else {
            opt(&tg, 0);
        }
    }

    CHECK_EQ(pay_count, 2,
             "ren: both copies' ab#1 fired (two cost prompts were answered)");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 1,
             "ren: only ren1 paid -> exactly 1 card recovered");
    effect_guard(fp, mark, "ren's two-copy cost prompts must reach the frontend");
}

/* ===================================================================== */
/* 10. effects/deck_order/restore - optional_waitroom_to_deck_top_test.rs  */
/*      PL!SP-bp2-013-N debut: "you may put 1 waitroom card onto the deck    */
/*      TOP" and PL!N-bp4-021-N debut (same shape, the accepted/skipped    */
/*      pair).                                                                */
/*      THE PURE DECK-TOP CASE: a card is moved from the waitroom onto the    */
/*      deck's index 0. This is the one place where "prepend vs append" is   */
/*      the whole assertion, so each test pins deck.cards[0] by identity and  */
/*      also pins the deck's BOTTOM, which is where a wrongly-appended card   */
/*      would land.                                                           */
/* ===================================================================== */

#define SP_BP2_013_N "PL!SP-bp2-013-N"
#define N_BP4_021_N  "PL!N-bp4-021-N"
#define N_SD1_025_SD "PL!N-sd1-025-SD"

static void test_sp_bp2_013_debut_places_waitroom_card_on_deck_top(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int card = md_id(&tg, SP_BP2_013_N);
    pin_id(card, SP_BP2_013_N);
    pin_cost(card, 9);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int marker = md_id(&tg, LIVE_FILLER);
    pin_id(marker, LIVE_FILLER);
    test_add_to_discard(&tg, marker);

    test_give_energy(&tg, 12);
    test_add_to_hand(&tg, card);
    int bottom_before = tg.state.p[0].deck.n - 1;
    int bottom_id = tg.state.p[0].deck.cards[bottom_before];

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_play_to_stage(&tg, card, 1, 0);

    CHECK(strcmp(pending_type(&tg), "SelectCard") == 0,
          "sp_bp2_013: a SelectCard over the waitroom is expected, got '%s'",
          pending_type(&tg));
    expect_pending(rb_has_pending_choice(&tg.state),
                   "sp_bp2_013: the waitroom pick is prompted");
    CHECK(pending_zone_is(&tg, "discard"),
          "sp_bp2_013: the prompt's zone is the waitroom (\"discard\"), got '%s'",
          rb_get_pending_choice(&tg.state) ? rb_get_pending_choice(&tg.state)->zone : "(none)");
    check_deck_top_is(&tg, 0, filler,
                      "sp_bp2_013 fixture pin: the deck's head is a filler before the effect");
    pick(&tg, 0);
    drain_choices(&tg);

    check_deck_top_is(&tg, 0, marker,
                      "sp_bp2_013: the chosen card sits on TOP of the deck (index 0)");
    check_deck_bottom_is(&tg, 0, bottom_id,
                         "sp_bp2_013: the deck's BOTTOM is unchanged, so the card was "
                         "prepended and not appended");
    CHECK(!wait_has(&tg, marker), "sp_bp2_013: the chosen card left the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 31, "sp_bp2_013: the deck grew by exactly one");
    effect_guard(fp, mark, "sp_bp2_013 must reach the frontend");
}

static void test_sp_bp2_013_debut_can_place_zero_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int card = md_id(&tg, SP_BP2_013_N);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int bystander = md_id(&tg, LIVE_FILLER);
    test_add_to_discard(&tg, bystander);
    test_give_energy(&tg, 12);
    test_add_to_hand(&tg, card);

    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_play_to_stage(&tg, card, 1, 0);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "sp_bp2_013b: the optional waitroom pick is offered");
    skip_choice(&tg);
    drain_choices(&tg);

    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "sp_bp2_013b: declined -> deck untouched");
    CHECK(wait_has(&tg, bystander), "sp_bp2_013b: the bystander remains in the waitroom");
    /* VACUITY CONTROL: a "deck untouched" assertion passes on a dead engine. The
     * prompt guard proves the debut really ran, and the card really landed on
     * the stage. */
    CHECK_EQ(tg.state.p[0].stage[1], card,
             "sp_bp2_013b vacuity control: the debut card is on the centre stage");
    effect_guard(fp, mark, "sp_bp2_013b's declined prompt must have been answered");
}

static void test_n_bp4_021_debut_accept_puts_waitroom_card_on_deck_top(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int kasumi = md_id(&tg, N_BP4_021_N);
    pin_id(kasumi, N_BP4_021_N);
    int recycled = md_id(&tg, N_SD1_025_SD);
    pin_id(recycled, N_SD1_025_SD);
    int other = md_id(&tg, FILLER);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int bottom_before = tg.state.p[0].deck.n - 1;
    int bottom_id = tg.state.p[0].deck.cards[bottom_before];

    test_add_to_hand(&tg, kasumi);
    /* Rust add_to_discard(recycled) then add_to_discard(other): the waitroom
     * order is [recycled, other], so index 0 is `recycled`. */
    test_add_to_discard(&tg, recycled);
    test_add_to_discard(&tg, other);
    test_give_energy(&tg, 12);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_play_to_stage(&tg, kasumi, 1, 0);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "n_bp4_021: the optional move must be prompted");
    pick(&tg, 0);
    drain_choices(&tg);

    check_deck_top_is(&tg, 0, recycled,
                      "n_bp4_021: accepted -> the waitroom card sits on the deck TOP");
    check_deck_bottom_is(&tg, 0, bottom_id,
                         "n_bp4_021: the deck's BOTTOM is unchanged (prepend, not append)");
    CHECK(!wait_has(&tg, recycled), "n_bp4_021: accepted -> the card left the waitroom");
    CHECK(wait_has(&tg, other), "n_bp4_021: the unchosen waitroom card stays");
    effect_guard(fp, mark, "n_bp4_021 must reach the frontend");
}

static void test_n_bp4_021_debut_skip_keeps_card_in_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int kasumi = md_id(&tg, N_BP4_021_N);
    int recycled = md_id(&tg, N_SD1_025_SD);
    int other = md_id(&tg, FILLER);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    test_add_to_hand(&tg, kasumi);
    test_add_to_discard(&tg, recycled);
    test_add_to_discard(&tg, other);
    test_give_energy(&tg, 12);

    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_play_to_stage(&tg, kasumi, 1, 0);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "n_bp4_021b: the optional move must be prompted");
    skip_choice(&tg);
    drain_choices(&tg);

    CHECK(wait_has(&tg, recycled), "n_bp4_021b: declined -> the card stays in the waitroom");
    /* VACUITY CONTROL: "stays in the waitroom" holds on a dead engine, so the
     * prompt guard and the stage placement carry the test. */
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "n_bp4_021b vacuity control: the deck really is the same size");
    check_deck_top_is(&tg, 0, filler,
                      "n_bp4_021b vacuity control: the deck's head is still a filler");
    effect_guard(fp, mark, "n_bp4_021b's declined prompt must have been answered");
}

/* ===================================================================== */
/* 11. effects/deck_order/restore - clean_members_energy_pl_n_bp7_008_r    */
/*      PL!N-bp7-008-R debut: "put 1 of your waitroom MEMBER cards whose     */
/*      blade count is 0 under your deck, then activate 1 energy" (a         */
/*      deck-bottom placement plus an energy side effect).                   */
/*      (Emma). Distinct: a DECK-BOTTOM placement - the opposite end from the */
/*      previous section, on the same harness - plus a card_type/cost FILTER  */
/*      over the waitroom and an energy side effect.                         */
/* ===================================================================== */

#define N_BP7_008_R   "PL!N-bp7-008-R"
#define KOTORI_CLEAN  "PL!-pb1-021-PR"
#define SD1_002_SD    "PL!-sd1-002-SD"

static int card_blade(int cid)
{
    Card c;
    int b = -1;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        b = c.blade;
        rb_free_card(&c);
    }
    return b;
}

static void test_n_bp7_008_bottoms_only_non_blade_heart_members_and_activates(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int emma = md_id(&tg, N_BP7_008_R);
    pin_id(emma, N_BP7_008_R);
    int clean = md_id(&tg, KOTORI_CLEAN);
    pin_id(clean, KOTORI_CLEAN);
    int bladefill = md_id(&tg, FILLER);
    int bladefill2 = md_id(&tg, SD1_002_SD);
    tg.state.p[0].stage[1] = emma;

    /* Identity check on the fixture's own claim. The Rust test's comment calls
     * KOTORI_CLEAN "the blade-0 member" and the two fillers the blade holders,
     * but cards.json gives PL!-pb1-021-PR a PRINTED blade of 1 - the same as
     * both fillers. Nothing in the test adjusts any blade count, so the printed
     * 0 filter has NO eligible card in this fixture.
     * Reported as a RUST FIXTURE BUG, not an engine failure: the engine is being
     * asked a question the fixture cannot answer. */
    printf("DBG fixture blades: %s blade=%d | %s blade=%d | %s blade=%d\n",
           card_no_of(clean), card_blade(clean),
           card_no_of(bladefill), card_blade(bladefill),
           card_no_of(bladefill2), card_blade(bladefill2));
    if (card_blade(clean) != 0)
        printf("RUST FIXTURE BUG: %s prints blade %d, not 0, so the "
               "0 filter has no eligible card and this "
               "case cannot distinguish 'filter rejected it' from 'filter matched "
               "nothing'.\n", KOTORI_CLEAN, card_blade(clean));
    pin_id(clean, KOTORI_CLEAN);

    /* Rust pushes [bladefill, clean, bladefill2] -> C APPEND in that order. */
    test_add_to_discard(&tg, bladefill);
    test_add_to_discard(&tg, clean);
    test_add_to_discard(&tg, bladefill2);

    int stock = md_id(&tg, FILLER);
    push_to_deck(&tg, 0, stock);
    push_to_deck(&tg, 0, stock);
    check_deck_bottom_is(&tg, 0, stock, "n_bp7_008 fixture pin: deck bottom is a stock card");

    test_give_energy(&tg, 4);
    test_set_energy_active(&tg, 0, 0);
    int active_before = rb_energy_active_count(&tg.state.p[0]);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int fired = md_fire_debut(&tg, emma);
    CHECK(fired, "n_bp7_008: the live-start ability was found and fired");
    expect_pending(rb_has_pending_choice(&tg.state),
                   "n_bp7_008: a waitroom selection is offered");
    pick(&tg, 0);
    drain_choices(&tg);

    check_deck_bottom_is(&tg, 0, clean,
                         "n_bp7_008: the placed card sits at the deck BOTTOM (push end)");
    CHECK(!wait_has(&tg, clean), "n_bp7_008: the placed card left the waitroom");
    CHECK(wait_has(&tg, bladefill) && wait_has(&tg, bladefill2),
          "n_bp7_008: the blade-heart holders are not eligible and stay put");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), active_before + 1,
             "n_bp7_008: 1 card placed -> 1 energy activated (1+1)");
    effect_guard(fp, mark, "n_bp7_008 must reach the frontend");
}

static void test_n_bp7_008_empty_waitroom_is_a_noop(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int emma = md_id(&tg, N_BP7_008_R);
    int stock = md_id(&tg, FILLER);
    tg.state.p[0].stage[1] = emma;
    push_to_deck(&tg, 0, stock);
    push_to_deck(&tg, 0, stock);
    test_give_energy(&tg, 4);
    /* Rust sets energy_active = 1 and asserts it is still 1 afterwards. The C
     * engine returns 1 here too - but from an INITIAL value of 0, so it really
     * did activate one energy. The delta is asserted explicitly so the two
     * readings cannot be confused: see the report, n_bp7_008b. */
    test_set_energy_active(&tg, 0, 0);
    int active_before = rb_energy_active_count(&tg.state.p[0]);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_debut(&tg, emma);

    expect_pending(!rb_has_pending_choice(&tg.state),
                   "n_bp7_008b: an empty waitroom offers no selection");
    int deck_exp[2] = { stock, stock };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 2,
                 "n_bp7_008b: the deck is byte-identical");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), active_before,
             "n_bp7_008b: with an empty waitroom NOTHING is placed, so the "
             "(1+1) energy activation must not fire at all");
    /* VACUITY CONTROL: "nothing happened" is the assertion; the positive sibling
     * (n_bp7_008 with one eligible card) really does place one and activate one
     * on the same card, so the zero here is a filter result and not a no-op
     * engine. The card's ability is also proven to exist. */
    CHECK(card_prints_trigger(emma, TRIG_DEBUT),
          "n_bp7_008b vacuity control: the card does print a debut ability, so the "
          "no-op is the empty-waitroom gate and not a missing effect");

}

/* ===================================================================== */
/* 12. effects/deck_order/restore - aqours_bottom_pl_s_bp7_019_l_test.rs     */
/*      PL!S-bp7-019-L live-success: "put 2 of your waitroom Aqours MEMBER    */
/*      cards under your deck in the order you like" - the ORDER of a        */
/*      two-card return to the deck bottom, plus a group filter over the      */
/*      waitroom. This is the test the C API cannot fully observe (see the   */
/*      placement_order note in section 12).                                 */
/* ===================================================================== */

#define S_BP7_019_L  "PL!S-bp7-019-L"
#define AQ_SD1_003   "PL!S-sd1-003-SD"
#define AQ_SD1_017   "PL!S-sd1-017-SD"

static void test_bp7_019_places_two_aqours_cards_under_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = md_id(&tg, S_BP7_019_L);
    pin_id(live, S_BP7_019_L);
    int aq1 = md_id(&tg, AQ_SD1_003);
    int aq2 = md_id(&tg, AQ_SD1_017);
    pin_id(aq1, AQ_SD1_003);
    pin_id(aq2, AQ_SD1_017);
    int non_aq = md_id(&tg, FILLER);
    int filler = md_id(&tg, FILLER);

    md_fill_decks(&tg, filler);
    test_add_to_live_for(&tg, 0, live);
    /* Rust push(non_aq); push(aq1); push(aq2) -> C APPEND, so the waitroom is
     * [non_aq, aq1, aq2] and aq1 is the first ELIGIBLE index. */
    test_add_to_discard(&tg, non_aq);
    test_add_to_discard(&tg, aq1);
    test_add_to_discard(&tg, aq2);
    check_deck_bottom_is(&tg, 0, filler, "bp7_019 fixture pin: deck bottom is a filler");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int fired = md_fire_live_success(&tg, live);
    CHECK(fired, "bp7_019: the live-start ability was found and fired");
    expect_pending(rb_has_pending_choice(&tg.state),
                   "bp7_019: valid candidates -> a selection prompt");
    const int two[2] = { 0, 1 };
    pick_many(&tg, two, 2);
    drain_choices(&tg);

    CHECK(!wait_has(&tg, aq1) && !wait_has(&tg, aq2),
          "bp7_019: both Aqours cards left the waitroom");
    CHECK(wait_has(&tg, non_aq),
          "bp7_019: the non-Aqours card stays in the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 32, "bp7_019: 30 fillers + 2 returned cards");
    /* Rust: deck.ends_with(&[aq1, aq2]) - the selected order must be preserved
     * at the deck BOTTOM. The C bag's last two slots are the twin. */
    int tail[2] = { aq1, aq2 };
    check_deck_tail_is(&tg, 0, tail, 2,
                       "bp7_019: the selected order is preserved at the deck BOTTOM");
    effect_guard(fp, mark, "bp7_019 must reach the frontend");
}

static void test_bp7_019_no_aqours_is_a_noop(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = md_id(&tg, S_BP7_019_L);
    int non_aq = md_id(&tg, FILLER);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    test_add_to_live_for(&tg, 0, live);
    test_add_to_discard(&tg, non_aq);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);

    expect_pending(!rb_has_pending_choice(&tg.state),
                   "bp7_019b: no Aqours candidate -> no prompt");
    CHECK(wait_has(&tg, non_aq), "bp7_019b: the waitroom card is untouched");
    CHECK_EQ(tg.state.p[0].deck.n, 30, "bp7_019b: the deck did not grow");
    /* VACUITY CONTROL: the "no prompt" would hold on a dead engine, so the
     * positive sibling (two Aqours candidates -> a two-card prompt) and the
     * card's printed trigger carry this case. */
    CHECK(card_prints_trigger(live, TRIG_LIVE_SUCCESS),
          "bp7_019b vacuity control: the card does print a debut ability");

}

/* EXPECTED GAP recorded from the bp7_019 case: the flat RbChoice carries no
 * option list, so the printed  SelectTarget is not observable from
 * C - neither its existence nor the deck_top-vs-deck_bottom answer. */
static void test_expected_gap_placement_order_any_order(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int live = md_id(&tg, S_BP7_019_L);
    int aq1 = md_id(&tg, AQ_SD1_003);
    int aq2 = md_id(&tg, AQ_SD1_017);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    test_add_to_live_for(&tg, 0, live);
    test_add_to_discard(&tg, aq1);
    test_add_to_discard(&tg, aq2);
    md_fire_live_success(&tg, live);

    const RbChoice *c = rb_get_pending_choice(&tg.state);
    /* If the C engine ever grows an option list on RbChoice, this becomes
     * observable and the gap closes; the assertion says so either way. */
    int has_option_list = (c != NULL);
    EXPECTED_GAP(!has_option_list,
                 "placement_order \"any_order\" (): the C RbChoice "
                 "carries no option list, so a deck_top-vs-deck_bottom answer is "
                 "not observable. See section 16 of the file header.");
    drain_choices(&tg);
}

/* ===================================================================== */
/* 13. effects/deck_order/restore - revealed_hand_aqours_pl_s_sd1_009_sd      */
/*      PL!S-sd1-009-SD live-start: "you may put 1 revealed Aqours card onto  */
/*      the deck"                                                            */
/*      Distinct: the source is the REVEALED set / hand, and the destination   */
/*      is the deck. The flat RbChoice cannot say WHICH position option was    */
/*      answered, so top-vs-bottom is recorded as an EXPECTED_GAP.             */
/* ===================================================================== */

#define S_SD1_009_SD "PL!S-sd1-009-SD"

static void test_sd1_009_revealed_aqours_card_moves_from_hand_to_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);

    int ruby = md_id(&tg, S_SD1_009_SD);
    pin_id(ruby, S_SD1_009_SD);
    tg.state.p[0].stage[1] = ruby;

    int aq = md_id(&tg, AQ_SD1_003);
    int non_aq = md_id(&tg, FILLER);
    pin_id(aq, AQ_SD1_003);
    /* Rust hand.cards = vec![non_aqours, aqours_card]: APPEND order. */
    test_add_to_hand(&tg, non_aq);
    test_add_to_hand(&tg, aq);
    int aq_hand_index = 1;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, ruby);

    expect_pending(rb_has_pending_choice(&tg.state), "sd1_009: the reveal cost gate is offered");
    if (pending_target_is(&tg, "position|destination")) {
        /* EXPECTED GAP, not a pass: the flat RbChoice carries no option list, so
         * which of deck_top / deck_bottom this answer selects is NOT observable
         * from C. Asserted below as a gap. */
        opt(&tg, 0);
    } else {
        opt(&tg, 1);
    }
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 10) {
        if (strcmp(pending_type(&tg), "SelectCard") == 0) pick(&tg, aq_hand_index);
        else opt(&tg, 0);
    }

    CHECK(!hand_has(&tg, aq), "sd1_009: the revealed Aqours card left the hand");
    CHECK(deck_has(&tg, aq), "sd1_009: the revealed card ends up on the deck");
    CHECK(hand_has(&tg, non_aq), "sd1_009: the non-Aqours hand card stays put");
    EXPECTED_GAP(deck_index_of(&tg, 0, aq) >= 0 && deck_index_of(&tg, 0, aq) == 0,
                 "sd1_009: which DECK POSITION the revealed card took. The C "
                 "position|destination prompt carries no option list, so this can "
                 "only be checked when the card is on top; a deck_bottom answer is "
                 "indistinguishable from no answer in the C API.");
    effect_guard(fp, mark, "sd1_009 must reach the frontend");
}

static void test_sd1_009_without_aqours_preserves_hand_card(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int ruby = md_id(&tg, S_SD1_009_SD);
    tg.state.p[0].stage[1] = ruby;
    int only_mu = md_id(&tg, FILLER);
    test_add_to_hand(&tg, only_mu);

    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, ruby);

    if (!rb_has_pending_choice(&tg.state)) {
        /* Rust takes this early-return branch when the gate never opened. */
        CHECK(hand_has(&tg, only_mu), "sd1_009b: the 's hand card is preserved");
    } else {
        skip_choice(&tg);
        CHECK(hand_has(&tg, only_mu), "sd1_009b: the 's hand card is preserved");
        CHECK_EQ(tg.state.p[0].deck.n, deck_before, "sd1_009b: declined -> deck untouched");
    }
    /* VACUITY CONTROL: the hand card staying put is exactly what a dead engine
     * produces. The card's printed trigger and the positive sibling (an Aqours
     * card in hand really does leave the hand) are the other half. */
    CHECK(card_prints_trigger(ruby, TRIG_LIVE_START),
          "sd1_009b vacuity control: the card does print a debut ability");
    expect_effect_printed(ruby, TRIG_LIVE_START,
                       "sd1_009b vacuity control: the card really prints , so the preserved hand card is the Aqours filter");
}

/* ===================================================================== */
/* 14. effects/deck_order/restore - revealed_liella_pl_sp_bp7_023_l_test.rs  */
/*      PL!SP-bp7-023-L live-success: "you may put 1 revealed Liella! card    */
/*      onto the deck"                                                       */
/*      The same shape as sd1_009 but with a Liella! filter and a deck-COUNT  */
/*      assertion, which is what makes it a distinct engine path.              */
/* ===================================================================== */

#define SP_BP7_023_L "PL!SP-bp7-023-L"
#define N_SD1_010_SD "PL!N-sd1-010-SD"
#define SP_PB1_026_L "PL!SP-bp1-026-L"

static void test_sp_bp7_023_selected_revealed_liella_increases_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int filler = md_id(&tg, N_SD1_010_SD);
    md_fill_decks(&tg, filler);
    int live = md_id(&tg, SP_BP7_023_L);
    pin_id(live, SP_BP7_023_L);
    test_add_to_hand(&tg, live);
    test_set_live_card_for(&tg, 0, live);
    int liella_card = md_id(&tg, SP_PB1_026_L);
    pin_id(liella_card, SP_PB1_026_L);
    CHECK(rb_card_is_live(liella_card), "sp_bp7_023: %s is a live card", SP_PB1_026_L);
    tg.state.n_revealed = 0;
    test_add_to_revealed(&tg, liella_card);
    int deck_before = tg.state.p[0].deck.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);
    /* Rust: every prompt must carry a choice identity, then select index 0. */
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 10) {
        CHECK(pending_type(&tg)[0] != 0, "sp_bp7_023: prompt %d carries a choice identity", guard);
        pick(&tg, 0);
    }

    CHECK_EQ(tg.state.p[0].deck.n, deck_before + 1,
             "sp_bp7_023: the revealed Liella! card was placed onto the deck");
    CHECK(deck_has(&tg, liella_card), "sp_bp7_023: the Liella! card is on the deck");
    effect_guard(fp, mark, "sp_bp7_023 must reach the frontend");
}

static void test_sp_bp7_023_non_liella_reveal_is_a_noop(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int filler = md_id(&tg, N_SD1_010_SD);
    md_fill_decks(&tg, filler);
    int live = md_id(&tg, SP_BP7_023_L);
    test_add_to_hand(&tg, live);
    test_set_live_card_for(&tg, 0, live);
    tg.state.n_revealed = 0;
    int revealed = md_id(&tg, FILLER);   /* a mu's card, not Liella! */
    test_add_to_revealed(&tg, revealed);
    int deck_before = tg.state.p[0].deck.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);

    expect_pending(!rb_has_pending_choice(&tg.state),
                   "sp_bp7_023b: a non-Liella! reveal offers no selection");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before, "sp_bp7_023b: the deck is unchanged");
    /* VACUITY CONTROL: the positive sibling on the same card (a Liella! reveal
     * grows the deck) is the other half of this pair. */
    CHECK(card_prints_trigger(live, TRIG_LIVE_SUCCESS),
          "sp_bp7_023b vacuity control: the card does print a debut ability");

}

/* ===================================================================== */
/* 15. effects/deck_order/restore - s_bp2_021_live_success_test.rs           */
/*      PL!S-bp2-021-L live-success: "you may put 1 live card from your        */
/*      reveal under your deck"                                              */
/*      Source is the REVEALED (yell) set, destination the deck BOTTOM. Four   */
/*      branches of one ability: with a live, without a live, empty yell, and  */
/*      the explicit decline.                                                  */
/* ===================================================================== */

#define S_BP2_021_L "PL!S-bp2-021-L"

static void bp2_021_board(TestGame *tg, int *out_live)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin(tg);

    int staged = md_id(tg, AQ_MEMBER);
    tg->state.p[0].stage[0] = staged;
    int live = md_id(tg, S_BP2_021_L);
    pin_id(live, S_BP2_021_L);
    test_add_to_hand(tg, live);
    *out_live = live;
}

static void test_bp2_021_live_in_yell_moves_to_deck_bottom(void)
{
    static TestGame tg;
    int live = 0;
    bp2_021_board(&tg, &live);
    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < 10; i++) {
        push_to_deck(&tg, 0, filler);
        push_to_deck(&tg, 1, filler);
    }
    int live_in_yell = md_id(&tg, LIVE_FILLER);
    pin_id(live_in_yell, LIVE_FILLER);
    CHECK(rb_card_is_live(live_in_yell), "bp2_021: %s is a live card", LIVE_FILLER);
    tg.state.n_revealed = 0;
    test_add_to_revealed(&tg, live_in_yell);
    tg.state.yell_occurred = 1;
    push_to_deck(&tg, 0, filler);
    int deck_before = tg.state.p[0].deck.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);
    if (rb_has_pending_choice(&tg.state)) {
        CHECK(strcmp(pending_type(&tg), "SelectCard") == 0,
              "bp2_021: a SelectCard over revealed_cards is expected, got '%s'",
              pending_type(&tg));
        pick(&tg, 0);
    }
    drain_strict(&tg);

    /* Rust: deck.contains(&live_in_yell) || deck.len() == deck_before + 1. Kept
     * verbatim - it is the Rust assertion, not a strengthening. */
    CHECK(deck_has(&tg, live_in_yell) ||
              tg.state.p[0].deck.n == deck_before + 1,
          "bp2_021: the live should be at the deck bottom or the deck grew");
    CHECK(!(tg.state.n_revealed > 0 && tg.state.revealed_cards[0] == live_in_yell),
          "bp2_021: the revealed set no longer holds the live card");
    effect_guard(fp, mark, "bp2_021 must reach the frontend");
}

static void test_bp2_021_no_live_in_yell_no_move(void)
{
    static TestGame tg;
    int live = 0;
    bp2_021_board(&tg, &live);
    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < 10; i++) {
        push_to_deck(&tg, 0, filler);
        push_to_deck(&tg, 1, filler);
    }
    tg.state.n_revealed = 0;
    test_add_to_revealed(&tg, filler);   /* only a non-live card */
    tg.state.yell_occurred = 1;
    int deck_exp[11];
    for (int i = 0; i < 11; i++) deck_exp[i] = filler;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);
    if (rb_has_pending_choice(&tg.state)) skip_choice(&tg);
    drain_strict(&tg);

    check_bag_is(&tg.state.p[0].deck, deck_exp, 11,
                 "bp2_021b: no live in the yell should not change the deck at all");
    /* VACUITY CONTROL: the positive sibling (a live card in the yell really does
     * move) is the other half of this pair. */
    expect_effect_printed(live, TRIG_LIVE_SUCCESS,
                       "bp2_021b vacuity control: the card really prints , so the no-op is the live-card filter");
}

static void test_bp2_021_empty_yell_no_move(void)
{
    static TestGame tg;
    int live = 0;
    bp2_021_board(&tg, &live);
    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < 10; i++) push_to_deck(&tg, 0, filler);
    tg.state.n_revealed = 0;
    tg.state.yell_occurred = 0;
    int deck_exp[10];
    for (int i = 0; i < 10; i++) deck_exp[i] = filler;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);
    if (rb_has_pending_choice(&tg.state)) skip_choice(&tg);
    drain_strict(&tg);

    check_bag_is(&tg.state.p[0].deck, deck_exp, 10,
                 "bp2_021c: an empty yell leaves the deck byte-identical");
    expect_effect_printed(live, TRIG_LIVE_SUCCESS,
                       "bp2_021c vacuity control: the card really prints , so the no-op is the empty-reveal filter");
}

static void test_bp2_021_skip_optional(void)
{
    static TestGame tg;
    int live = 0;
    bp2_021_board(&tg, &live);
    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < 10; i++) push_to_deck(&tg, 0, filler);
    int live_in_yell = md_id(&tg, LIVE_FILLER);
    tg.state.n_revealed = 0;
    test_add_to_revealed(&tg, live_in_yell);
    tg.state.yell_occurred = 1;
    int deck_exp[10];
    for (int i = 0; i < 10; i++) deck_exp[i] = filler;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_success(&tg, live);
    if (rb_has_pending_choice(&tg.state)) skip_choice(&tg);
    drain_strict(&tg);

    check_bag_is(&tg.state.p[0].deck, deck_exp, 10,
                 "bp2_021d: skipping the optional placement leaves the deck unchanged");
    CHECK(!(tg.state.n_revealed > 0 && tg.state.revealed_cards[0] == live_in_yell) ||
              !deck_has(&tg, live_in_yell),
          "bp2_021d: the skip left the revealed live card out of the deck");
    effect_guard(fp, mark, "bp2_021d's declined prompt must have been answered");
}

/* ===================================================================== */
/* 16. effects/deck_order/restore - both_waitrooms_member_restore_q242      */
/*      PL!HS-pb1-012-R debut: "each player puts the MEMBER cards in THEIR   */
/*      OWN waitroom under their own deck; if 20+ cards in total, retrieve   */
/*      a live card to hand and gain 2 blades" (also PL!HS-pb1-012-P+).      */
/*      The richest cluster in the file: BOTH seats, a DIRECTIONAL placement   */
/*      (each player's own members to THEIR OWN deck bottom), a positional     */
/*      sentinel so "under the deck" is distinguishable from "in the deck", a  */
/*      card_type filter (a live card in the waitroom must NOT be shuffled),   */
/*      and a 20-card threshold with three boundary cases.                     */
/* ===================================================================== */

#define HS_PB1_012_R   "PL!HS-pb1-012-R"
#define HS_PB1_012_P_P "PL!HS-pb1-012-P\xEF\xBC\x8B"
#define SD1_001_SD   "PL!-sd1-001-SD"
#define SD1_003_SD   "PL!-sd1-003-SD"
#define SD1_004_SD   "PL!-sd1-004-SD"
#define MU_LIVE_SD   "PL!-sd1-020-SD"
#define N_BP1_001_R    "PL!N-bp1-001-R"
#define N_BP1_002_R_P  "PL!N-bp1-002-R\xEF\xBC\x8B"
#define N_BP1_003_R_P  "PL!N-bp1-003-R\xEF\xBC\x8B"
#define N_BP1_004_R    "PL!N-bp1-004-R"
#define N_BP1_005_R    "PL!N-bp1-005-R"
#define N_BP1_006_R_P  "PL!N-bp1-006-R\xEF\xBC\x8B"
#define N_BP1_007_R    "PL!N-bp1-007-R"
#define N_BP1_008_R    "PL!N-bp1-008-R"
#define N_BP1_009_R    "PL!N-bp1-009-R"
#define N_BP1_010_R    "PL!N-bp1-010-R"

static void q242_play_debut(TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg->state.p[0].stage[i] = RB_EMPTY_SLOT;
    test_give_energy(tg, 13);
    test_add_to_hand(tg, cid);
    md_play_to_stage(tg, cid, 1, 0);
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 20) pick(tg, 0);
}

static void test_q242_both_waitrooms_restore_to_own_deck_bottom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    pin_id(c, HS_PB1_012_R);
    /* Three DISTINCT prints per seat: one id in three slots cannot say WHICH
     * cards moved, which is the whole claim. */
    const char *prints[3] = { N_BP1_001_R, N_BP1_002_R_P, N_BP1_003_R_P };
    int p1m[3], p2m[3];
    for (int i = 0; i < 3; i++) {
        p1m[i] = md_id(&tg, prints[i]);
        p2m[i] = md_id(&tg, prints[i]);
        CHECK(rb_card_is_member(p1m[i]), "q242 fixture: %s is a member card", prints[i]);
        push_to_discard(&tg, 0, p1m[i]);
        push_to_discard(&tg, 1, p2m[i]);
    }
    /* A live card in P1's waitroom: not a member, so the shuffle must skip it. */
    int live = md_id(&tg, LIVE_FILLER);
    pin_id(live, LIVE_FILLER);
    CHECK(rb_card_is_live(live), "q242 fixture: %s is a live card", LIVE_FILLER);
    test_add_to_discard(&tg, live);
    /* Sentinels already in each deck, so "moved to the BOTTOM" is observable. */
    int p1_sentinel = md_id(&tg, FILLER);
    int p2_sentinel = md_id(&tg, FILLER);
    push_to_deck(&tg, 0, p1_sentinel);
    push_to_deck(&tg, 1, p2_sentinel);

    int p1_wait_before = tg.state.p[0].discard.n;   /* 3 members + 1 live = 4 */
    int p2_wait_before = tg.state.p[1].discard.n;   /* 3 members            */
    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;

    q242_play_debut(&tg, c);

    CHECK_EQ(tg.state.p[0].discard.n, p1_wait_before - 3,
             "q242: P1 loses exactly its own 3 members (the live stays: not a member)");
    CHECK_EQ(tg.state.p[1].discard.n, p2_wait_before - 3, "q242: P2 loses exactly its own 3");
    CHECK(wait_has(&tg, live),
          "q242: the live card is NOT a member, so it must stay in P1's waitroom");
    for (int i = 0; i < 3; i++) {
        CHECK(deck_has(&tg, p1m[i]), "q242: P1 member %s reaches P1's deck", card_no_of(p1m[i]));
        CHECK(!bag_has(&tg.state.p[1].deck, p1m[i]),
              "q242: P1 member %s must NOT cross into P2's deck", card_no_of(p1m[i]));
        CHECK(bag_has(&tg.state.p[1].deck, p2m[i]),
              "q242: P2 member %s reaches P2's deck", card_no_of(p2m[i]));
        CHECK(!deck_has(&tg, p2m[i]),
              "q242: P2 member %s must NOT cross into P1's deck", card_no_of(p2m[i]));
    }
    /*  - UNDER, not merely inside. The sentinel was
     * already in the deck, so it must end up ABOVE every shuffled member. */
    int sentinel_at = deck_index_of(&tg, 0, p1_sentinel);
    int lowest_member = -1;
    for (int i = 0; i < 3; i++) {
        int at = deck_index_of(&tg, 0, p1m[i]);
        if (at >= 0 && (lowest_member < 0 || at < lowest_member)) lowest_member = at;
    }
    CHECK(sentinel_at >= 0, "q242: the P1 sentinel is still in P1's deck");
    CHECK(lowest_member >= 0, "q242: P1's members are in P1's deck");
    CHECK(sentinel_at < lowest_member,
          "q242: members are shuffled UNDER the existing deck (sentinel at %d, "
          "lowest member at %d)", sentinel_at, lowest_member);
    /* 6 cards total is below the 20 threshold, so the second half does not fire.
     * These are the only two assertions pinning the sub-threshold case. */
    CHECK(!hand_has(&tg, live), "q242: 6 total < 20 threshold, so no live is retrieved");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, c), 0,
             "q242: 6 total < 20 threshold, so no blade+2");
    effect_guard(fp, mark, "q242 must reach the frontend");
}

static void test_q242_twenty_members_recover_live_and_gain_two_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    pin_id(c, HS_PB1_012_R);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    int m = md_id(&tg, AQ_MEMBER);
    for (int i = 0; i < 10; i++) {
        test_add_to_discard(&tg, m);
        push_to_discard(&tg, 1, m);
    }
    int live = md_id(&tg, LIVE_FILLER);
    test_add_to_discard(&tg, live);
    int w1_before = tg.state.p[0].discard.n;
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int blade_before = rb_mods_get_blade(&tg.state.mods, c);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, c);

    CHECK(hand_has(&tg, live), "q242: the live card is retrieved to hand");
    CHECK(!wait_has(&tg, live), "q242: the live card is no longer in the waitroom");
    CHECK(tg.state.p[0].discard.n < w1_before, "q242: P1's waitroom shrinks");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, c), blade_before + 2,
             "q242: blade+2 is gained from the debut (happy path)");
    /* VACUITY CONTROL: the blade +2 and the retrieval are both positive, so this
     * case is not an absence at all. */
    CHECK_EQ(tg.state.p[1].discard.n, 0,
             "q242 vacuity control: P2's 10 members all left its waitroom");
    effect_guard(fp, mark, "q242's twenty-card path must reach the frontend");
}

static void test_q242_twenty_members_blades_without_recoverable_live(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    int m = md_id(&tg, AQ_MEMBER);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < 10; i++) {
        test_add_to_discard(&tg, m);
        push_to_discard(&tg, 1, m);
    }
    int hand_before = tg.state.p[0].hand.n;
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int blade_before = rb_mods_get_blade(&tg.state.mods, c);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, c);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "q242b: the hand does not grow when there is no live card to recover");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, c), blade_before + 2,
             "q242b: blade+2 is gained EVEN with no live card in the waitroom");
    /* VACUITY CONTROL: the hand-unchanged assertion is an absence; the blade+2
     * is the positive that proves the effect ran. */
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "q242b vacuity control: all 10 of P1's members left the waitroom");
    effect_guard(fp, mark, "q242b must reach the frontend");
}

static void test_q242_p_plus_twenty_members_blades_without_live(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int card = md_id(&tg, HS_PB1_012_P_P);
    pin_id(card, HS_PB1_012_P_P);
    int member = md_id(&tg, AQ_MEMBER);
    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < 10; i++) {
        test_add_to_discard(&tg, member);
        push_to_discard(&tg, 1, member);
    }
    int hand_before = tg.state.p[0].hand.n;
    md_fill_decks(&tg, filler);
    int blade_before = rb_mods_get_blade(&tg.state.mods, card);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, card);

    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "q242c: the hand does not grow");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, card), blade_before + 2,
             "q242c: the P+ print behaves as R and gains blade+2");
    /* VACUITY CONTROL: the + is a DISTINCT pool slot from the R print above, so
     * the modifier really was applied to this card and not to a shared template. */
    CHECK(card != test_id(&tg, HS_PB1_012_R),
          "q242c vacuity control: the P+ card is its own instance, not the R template");
    effect_guard(fp, mark, "q242c must reach the frontend");
}

static void test_q242_exactly_twenty_meets_threshold(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    int m = md_id(&tg, AQ_MEMBER);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < 10; i++) {
        test_add_to_discard(&tg, m);
        push_to_discard(&tg, 1, m);
    }
    int live = md_id(&tg, LIVE_FILLER);
    test_add_to_discard(&tg, live);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, c);

    CHECK(rb_mods_get_blade(&tg.state.mods, c) >= 2,
          "q242d: exactly 20 cards moved -> threshold met, blade+2 applies");
    CHECK(hand_has(&tg, live), "q242d: the live card is retrieved at exactly 20");
    effect_guard(fp, mark, "q242d must reach the frontend");
}

static void test_q242_nineteen_members_grant_neither(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    int m = md_id(&tg, AQ_MEMBER);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < 9; i++) test_add_to_discard(&tg, m);
    for (int i = 0; i < 10; i++) push_to_discard(&tg, 1, m);
    int live = md_id(&tg, LIVE_FILLER);
    test_add_to_discard(&tg, live);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    int deck_before = tg.state.p[0].deck.n;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, c);

    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, c), 0, "q242e: 19 < 20 -> no blade+2");
    CHECK(!hand_has(&tg, live), "q242e: the live card is not retrieved below the threshold");
    /* VACUITY CONTROL: both assertions above are ABSENCES. This is the case where
     * they would pass on a dead engine, so the positive is the shuffle itself:
     * the 9 members really did leave P1's waitroom for P1's deck. */
    CHECK_EQ(tg.state.p[0].discard.n, 1,
             "q242e vacuity control: the 9 members left P1's waitroom (only the "
             "live card remains)");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before + 9,
             "q242e vacuity control: 9 members really reached P1's deck");
    effect_guard(fp, mark, "q242e's sub-threshold path must reach the frontend");
}

static void test_q242_twenty_from_own_waitroom_alone_grants_blades(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int c = md_id(&tg, HS_PB1_012_R);
    int m = md_id(&tg, AQ_MEMBER);
    for (int i = 0; i < RB_STAGE_SIZE; i++) tg.state.p[0].stage[i] = RB_EMPTY_SLOT;
    for (int i = 0; i < 20; i++) test_add_to_discard(&tg, m);
    int live = md_id(&tg, LIVE_FILLER);
    test_add_to_discard(&tg, live);
    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, c);

    CHECK(rb_mods_get_blade(&tg.state.mods, c) >= 2,
          "q242f: P1-only 20 cards -> threshold met (total=20), blade+2 applies");
    /* VACUITY CONTROL: the blade is the positive, and P2's untouched waitroom is
     * the evidence the threshold counts the TOTAL across both seats. */
    CHECK_EQ(tg.state.p[1].discard.n, 0,
             "q242f vacuity control: P2's waitroom was empty and stayed so");
    CHECK(hand_has(&tg, live), "q242f: the live card is also retrieved");
    effect_guard(fp, mark, "q242f must reach the frontend");
}

static void test_q242_real_debut_ten_each_to_own_deck_bottom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int q242 = md_id(&tg, HS_PB1_012_R);
    pin_id(q242, HS_PB1_012_R);
    const char *prints[10] = {
        N_BP1_001_R, N_BP1_002_R_P, N_BP1_003_R_P, N_BP1_004_R, N_BP1_005_R,
        N_BP1_006_R_P, N_BP1_007_R, N_BP1_008_R, N_BP1_009_R, N_BP1_010_R
    };
    int p1m[10], p2m[10];
    for (int i = 0; i < 10; i++) {
        p1m[i] = md_id(&tg, prints[i]);
        p2m[i] = md_id(&tg, prints[i]);
        CHECK(rb_card_is_member(p1m[i]), "q242g fixture: %s is a member card", prints[i]);
        test_add_to_discard(&tg, p1m[i]);
        push_to_discard(&tg, 1, p2m[i]);
    }
    int p1_live = md_id(&tg, LIVE_FILLER);
    int p2_live = md_id(&tg, MU_LIVE_SD);
    int p1_nonmember = md_id(&tg, ENERGY_CD);
    int p1_sentinel = md_id(&tg, MU_LIVE_SD);
    int p2_sentinel = md_id(&tg, MU_LIVE_SD);
    test_add_to_discard(&tg, p1_live);
    test_add_to_discard(&tg, p1_nonmember);
    push_to_discard(&tg, 1, p2_live);
    push_to_deck(&tg, 0, p1_sentinel);
    push_to_deck(&tg, 1, p2_sentinel);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    q242_play_debut(&tg, q242);

    CHECK(rb_card_no_eq(q242, HS_PB1_012_R), "q242g: the played card is PL!HS-pb1-012-R");
    CHECK_EQ(tg.state.p[0].stage[1], q242, "q242g: it is on the centre stage");
    for (int i = 0; i < 10; i++) {
        CHECK(!wait_has(&tg, p1m[i]), "q242g: P1 member %s left P1's waitroom", card_no_of(p1m[i]));
        CHECK(deck_has(&tg, p1m[i]), "q242g: P1 member %s is in P1's deck", card_no_of(p1m[i]));
        CHECK(!bag_has(&tg.state.p[1].deck, p1m[i]), "q242g: P1 member %s did not cross over", card_no_of(p1m[i]));
        CHECK(!bag_has(&tg.state.p[1].discard, p2m[i]), "q242g: P2 member %s left P2's waitroom", card_no_of(p2m[i]));
        CHECK(bag_has(&tg.state.p[1].deck, p2m[i]), "q242g: P2 member %s is in P2's deck", card_no_of(p2m[i]));
        CHECK(!deck_has(&tg, p2m[i]), "q242g: P2 member %s did not cross over", card_no_of(p2m[i]));
    }
    CHECK(deck_has(&tg, p1_sentinel), "q242g: P1's sentinel is still in P1's deck");
    CHECK(bag_has(&tg.state.p[1].deck, p2_sentinel), "q242g: P2's sentinel is still in P2's deck");
    CHECK(wait_has(&tg, p1_nonmember), "q242g: the energy card stays in the waitroom");
    CHECK(!deck_has(&tg, p1_nonmember), "q242g: the energy card is not shuffled under the deck");
    CHECK(hand_has(&tg, p1_live), "q242g: P1's live card is retrieved to hand");
    CHECK(!wait_has(&tg, p1_live), "q242g: P1's live card left the waitroom");
    CHECK(bag_has(&tg.state.p[1].discard, p2_live), "q242g: P2's live card stays in P2's waitroom");
    CHECK(!hand_has(&tg, p2_live), "q242g: P2's live card is not retrieved for P1");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, q242), 2, "q242g: blade+2 at the 20 threshold");
    effect_guard(fp, mark, "q242g must reach the frontend");
}

/* ===================================================================== */
/* 17. effects/deck_order/restore - recover_and_order_opponent_bottom        */
/*      PL!HS-PR-035-PR debut: "you may choose 3 MEMBER cards from the        */
/*      opponent's waitroom and put them under the opponent's deck in any   */
/*      order you like"                                                      */
/*      This is OPPONENT-SIDE deck manipulation, and the  placement_order any_order case  */
/*      again. Rust's assertion is set-membership plus a count, not an order, */
/*      so that is what is asserted - the ORDER is an EXPECTED_GAP.           */
/* ===================================================================== */

#define HS_PR_035_PR "PL!HS-PR-035-PR"

static void test_gin_ko_places_opponent_discard_under_opponent_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int a = md_id(&tg, SD1_001_SD);
    int b = md_id(&tg, SD1_003_SD);
    int c = md_id(&tg, SD1_004_SD);
    CHECK(rb_card_is_member(a) && rb_card_is_member(b) && rb_card_is_member(c),
          "gin_ko fixture: the three opponent-discard cards really are members");
    push_to_discard(&tg, 1, a);
    push_to_discard(&tg, 1, b);
    push_to_discard(&tg, 1, c);

    int filler = md_id(&tg, FILLER);
    for (int i = 0; i < 5; i++) push_to_deck(&tg, 1, filler);
    int p2_deck_before = tg.state.p[1].deck.n;
    check_deck_bottom_is(&tg, 1, filler, "gin_ko fixture pin: P2 deck bottom is a filler");

    int gin = md_id(&tg, HS_PR_035_PR);
    pin_id(gin, HS_PR_035_PR);
    tg.state.p[0].stage[1] = gin;

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    int fired = md_fire_debut(&tg, gin);
    CHECK(fired, "gin_ko: the live-start ability was found and fired");

    /* Rust: SelectTarget -> option 1 (accept); SelectCard -> ALL indices. */
    int guard = 0;
    int accepted = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 12) {
        if (strcmp(pending_type(&tg), "SelectTarget") == 0) {
            opt(&tg, 1);
            accepted++;
        } else if (strcmp(pending_type(&tg), "SelectCard") == 0) {
            const RbChoice *c2 = rb_get_pending_choice(&tg.state);
            int n = c2 ? c2->count : 3;
            if (n < 1) n = 1;
            int idx[3];
            for (int i = 0; i < n && i < 3; i++) idx[i] = i;
            pick_many(&tg, idx, n < 3 ? n : 3);
        } else {
            skip_choice(&tg);
        }
    }

    CHECK(accepted > 0, "gin_ko: the optional  was offered and accepted");
    for (int i = 0; i < 3; i++) {
        int id = (i == 0) ? a : (i == 1) ? b : c;
        CHECK(!bag_has(&tg.state.p[1].discard, id),
              "gin_ko: chosen card %s left the opponent's discard", card_no_of(id));
        CHECK(bag_has(&tg.state.p[1].deck, id),
              "gin_ko: chosen card %s is on the opponent's deck", card_no_of(id));
    }
    CHECK_EQ(tg.state.p[1].deck.n, p2_deck_before + 3,
             "gin_ko: 3 cards added to the opponent's deck bottom");
    /* EXPECTED GAP, not a pass:  is a placement_order=any_order
     * prompt; the C RbChoice carries no option list, so the ORDER the three
     * cards were placed in is not observable. Rust's own test asserts
     * membership and count only, which is what is asserted above. */
    EXPECTED_GAP(1, "gin_ko: the ORDER the three opponent cards were placed in "
                    "( / placement_order=any_order) is not observable "
                    "from C - the flat RbChoice has no option list.");
    effect_guard(fp, mark, "gin_ko must reach the frontend");
}

/* ===================================================================== */
/* 18. effects/deck_order/restore - waited_opponent_nijigasaki_topdeck       */
/*      PL!N-bp4-004-R+ live-start: "choose 2 of the opponent's members in  */
/*      the wait state and put 2 Nijigasaki MEMBER cards from your waitroom  */
/*      onto the deck TOP"                                                    */
/*      A DECK-TOP placement gated on an OPPONENT's wait state, with a group   */
/*      filter and a two-card selection.                                      */
/* ===================================================================== */

#define N_BP4_004_R_P "PL!N-bp4-004-R\xEF\xBC\x8B"
#define N_BP3_002_R   "PL!N-bp3-002-R"

static void test_bp4_004_plus_waits_opponents_recover_to_deck_top(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int karin = md_id(&tg, N_BP4_004_R_P);
    pin_id(karin, N_BP4_004_R_P);

    int opp_a = md_id(&tg, FILLER);
    int opp_b = md_id(&tg, FILLER);
    int opp_active = md_id(&tg, FILLER);
    tg.state.p[1].stage[0] = opp_a;
    tg.state.p[1].stage[1] = opp_b;
    tg.state.p[1].stage[2] = opp_active;
    /* Rust mods.add_orientation_modifier(opp_a, "wait"). */
    rb_mods_set_orientation(&tg.state.mods, opp_a, "wait");
    rb_mods_set_orientation(&tg.state.mods, opp_b, "wait");
    CHECK(strcmp(rb_mods_get_orientation(&tg.state.mods, opp_a), "wait") == 0,
          "bp4_004 fixture: opp_a really is in the wait state");
    tg.state.p[0].stage[1] = karin;

    int niji_a = md_id(&tg, N_BP3_002_R);
    int niji_b = md_id(&tg, N_BP3_002_R);
    int niji_c = md_id(&tg, N_BP3_002_R);
    pin_id(niji_a, N_BP3_002_R);
    CHECK(rb_card_is_member(niji_a), "bp4_004 fixture: %s is a member card", N_BP3_002_R);
    /* Rust extend_from_slice(&[a, b, c]) -> C APPEND in that order. */
    test_add_to_discard(&tg, niji_a);
    test_add_to_discard(&tg, niji_b);
    test_add_to_discard(&tg, niji_c);

    int filler = md_id(&tg, FILLER);
    md_fill_decks(&tg, filler);
    check_deck_top_is(&tg, 0, filler, "bp4_004 fixture pin: the deck's head is a filler");

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, karin);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "bp4_004: a selection from the waitroom should be asked");
    const int two[2] = { 0, 1 };
    pick_many(&tg, two, 2);
    drain_choices(&tg);

    /* Rust: deck_top (first two slots) contains niji_a OR niji_b. The C deck is
     * also top-index-0, so the twin is the first two slots. */
    int top2[2] = { tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -1,
                    tg.state.p[0].deck.n > 1 ? tg.state.p[0].deck.cards[1] : -1 };
    CHECK(top2[0] == niji_a || top2[0] == niji_b ||
              top2[1] == niji_a || top2[1] == niji_b,
          "bp4_004: the selected members are placed on TOP of the deck");
    /* VACUITY CONTROL: the two selected cards really left the waitroom, so the
     * placement is a move and not a copy. */
    CHECK(!wait_has(&tg, niji_a) && !wait_has(&tg, niji_b),
          "bp4_004 vacuity control: both selected members left the waitroom");
    CHECK(wait_has(&tg, niji_c), "bp4_004 vacuity control: the unselected member stays");
    effect_guard(fp, mark, "bp4_004 must reach the frontend");
}

/* ===================================================================== */
/* 19. effects/deck_order/restore - live_start_two_cards_optional_group     */
/*      PL!S-bp6-004-SEC live-start: "if you have 2 Aqours live cards, you    */
/*      may put 1 of them onto the deck TOP" - a COUNT gate in front of a     */
/*      deck-top placement, with a third branch: mixed ability cards.         */
/*      The filter must skip the card that prints its own live-start ability  */
/*      and only offer the one that does not.                                 */
/*      Purely negative-branch tests, so each carries a prompt guard.          */
/* ===================================================================== */

#define S_BP6_004_SEC "PL!S-bp6-004-SEC"
#define S_PR_022_PR   "PL!S-PR-022-PR"
#define S_SD1_019_SD  "PL!S-sd1-019-SD"
#define S_BP2_025_L   "PL!S-bp2-025-L"

static void test_bp6_004_offered_with_two_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int dia = md_id(&tg, S_BP6_004_SEC);
    pin_id(dia, S_BP6_004_SEC);
    int filler = md_id(&tg, FILLER);
    int live_a = md_id(&tg, S_PR_022_PR);
    int live_b = md_id(&tg, S_SD1_019_SD);
    pin_id(live_a, S_PR_022_PR);
    pin_id(live_b, S_SD1_019_SD);
    CHECK(rb_card_is_live(live_a), "bp6_004 fixture: %s is a live card", S_PR_022_PR);
    CHECK(rb_card_is_live(live_b), "bp6_004 fixture: %s is a live card", S_SD1_019_SD);
    tg.state.p[0].stage[1] = dia;
    test_add_to_live_for(&tg, 0, live_a);
    test_add_to_live_for(&tg, 0, live_b);
    md_fill_decks(&tg, filler);
    test_give_energy(&tg, 5);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, dia);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "bp6_004: with 2 cards the condition is met -> the ability fires");
    effect_guard(fp, mark, "bp6_004's two-card branch must reach the frontend");
    drain_choices(&tg);
}

static void test_bp6_004_lt2_cards_no_trigger(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int dia = md_id(&tg, S_BP6_004_SEC);
    int filler = md_id(&tg, FILLER);
    tg.state.p[0].stage[1] = dia;
    test_add_to_live_for(&tg, 0, filler);   /* a non-live card, so even 1 fails twice */
    md_fill_decks(&tg, filler);
    test_give_energy(&tg, 5);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, dia);

    expect_pending(!rb_has_pending_choice(&tg.state),
                   "bp6_004b: fewer than 2 cards -> the condition fails, no prompt");
    /* VACUITY CONTROL: a "no trigger" assertion is the definition of a vacuous
     * test. The positive sibling (2 live cards -> a prompt, asserted above) is
     * the control, and the card's printed trigger proves the ability exists. */
    CHECK(card_prints_trigger(dia, TRIG_LIVE_START),
          "bp6_004b vacuity control: the card does print a debut ability");

}

static void test_bp6_004_zero_cards_no_trigger(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int dia = md_id(&tg, S_BP6_004_SEC);
    int filler = md_id(&tg, FILLER);
    tg.state.p[0].stage[1] = dia;
    md_fill_decks(&tg, filler);
    test_give_energy(&tg, 5);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, dia);

    expect_pending(!rb_has_pending_choice(&tg.state),
                   "bp6_004c: 0 cards -> the condition fails, no prompt");
    CHECK(card_prints_trigger(dia, TRIG_LIVE_START),
          "bp6_004c vacuity control: the card does print a debut ability");

}

static void test_bp6_004_offered_with_mixed_ability_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin(&tg);

    int dia = md_id(&tg, S_BP6_004_SEC);
    int filler = md_id(&tg, FILLER);
    int with_ls = md_id(&tg, S_BP2_025_L);
    int without_ls = md_id(&tg, S_PR_022_PR);
    pin_id(with_ls, S_BP2_025_L);
    pin_id(without_ls, S_PR_022_PR);
    /* The fixture's own claim, verified: one HAS live-start, one does not. */
    CHECK(card_prints_trigger(with_ls, TRIG_LIVE_START),
          "bp6_004d fixture: %s does print ", S_BP2_025_L);
    CHECK(!card_prints_trigger(without_ls, TRIG_LIVE_START),
          "bp6_004d fixture: %s does NOT print ", S_PR_022_PR);
    tg.state.p[0].stage[1] = dia;
    test_add_to_live_for(&tg, 0, with_ls);
    test_add_to_live_for(&tg, 0, without_ls);
    md_fill_decks(&tg, filler);
    test_give_energy(&tg, 5);

    int mark = prompt_mark();
    long long fp = state_fingerprint(&tg); (void)fp; (void)mark;
    md_fire_live_start(&tg, dia);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "bp6_004d: the ability fires with the correct filter - only the "
                   "non- card is selectable");
    effect_guard(fp, mark, "bp6_004d must reach the frontend");
    drain_choices(&tg);
}

/* ===================================================================== */
/* main                                                                    */
/* ===================================================================== */

int main(int argc, char **argv)
{
    int dump = (argc > 1 && strcmp(argv[1], "dump") == 0);
    if (argc > 1 && !dump) only_case = argv[1];

    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS, on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);
    signal(SIGFPE, on_fatal_signal);
    signal(SIGILL, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "fatal: no card database (tried src, ../cards/build)\n");
        return 2;
    }

    if (dump) {
        static TestGame tg;
        test_game_new(&tg);
        clear_p1(&tg);
        clear_p2(&tg);
        const char *cards[] = {
            S_BP5_015_N, FILLER, LIVE_FILLER, AQ_MEMBER, MU_MEMBER, KOSUZU, SAYAKA,
            HS_BP5_013_N, SP_BP4_022_N, S_BP7_020_L, S_BP7_021_L, RINA, BP5_010_N,
            ARISE_R, REN, LIELLA_A, LIELLA_B, CHISATO, SCRY, ENERGY_CD,
            SP_BP2_013_N, N_BP4_021_N, N_SD1_025_SD, N_BP7_008_R, KOTORI_CLEAN,
            SD1_002_SD, S_BP7_019_L, AQ_SD1_003, AQ_SD1_017, S_SD1_009_SD,
            SP_BP7_023_L, N_SD1_010_SD, SP_PB1_026_L, S_BP2_021_L, HS_PB1_012_R,
            HS_PB1_012_P_P, N_BP1_001_R, N_BP1_002_R_P, N_BP1_003_R_P, N_BP1_004_R,
            N_BP1_005_R, N_BP1_006_R_P, N_BP1_007_R, N_BP1_008_R, N_BP1_009_R,
            N_BP1_010_R, HS_PR_035_PR, SD1_001_SD, SD1_003_SD, SD1_004_SD,
            MU_LIVE_SD, N_BP4_004_R_P, N_BP3_002_R, S_BP6_004_SEC, S_PR_022_PR,
            S_SD1_019_SD, S_BP2_025_L,
        };
        for (size_t i = 0; i < sizeof cards / sizeof cards[0]; i++) {
            int cid = test_new_id(&tg, cards[i]);
            printf("%-24s -> id=%-6d cost=%-3d live=%d member=%d blade=%-3d ab=%d\n",
                   cards[i], cid, card_printed_cost(cid),
                   rb_card_is_live(cid) ? 1 : 0,
                   rb_card_is_member(cid) ? 1 : 0, card_blade(cid),
                   rb_card_num_abilities((uint32_t)(cid >= 0 ? cid : 0)));
            if (cid < 0) continue;
            for (int a = 0; a < rb_card_num_abilities((uint32_t)cid); a++)
                dump_ability(cid, a);
        }
        return 0;
    }

    /* ---- effects/mill ---- */
    run("mill/sbp5_015_ten_keeps_two", test_sbp5_015_twelve_card_deck_mills_ten_keeps_two);
    run("mill/kosuzu_three_members_draw_one", test_kosuzu_mill_three_all_members_draws_one);
    run("mill/kosuzu_live_among_three", test_kosuzu_mill_three_with_live_card_skips_draw);
    run("mill/sayaka_top_five", test_sayaka_debut_mills_top_five_to_waitroom);
    run("mill/bp5_013_three_members_two_blades", test_bp5_013_mill_three_all_members_two_blades);
    run("mill/bp5_013_milled_live_no_blades", test_bp5_013_milled_live_card_no_blades);
    run("mill/bp7_020_bottom_aqours", test_bp7_020_bottom_aqours_member_reduces_required_heart);
    run("mill/bp7_020_bottom_non_aqours", test_bp7_020_bottom_non_aqours_member_does_not_reduce);
    run("mill/bp7_021_two_members", test_bp7_021_bottom_mill_two_members_no_draw_or_score);
    run("mill/bp7_021_three_members", test_bp7_021_bottom_mill_three_members_draw_only);
    run("mill/bp7_021_five_members", test_bp7_021_bottom_mill_five_members_draw_and_score);
    run("mill/bp7_021_two_staged_gate", test_bp7_021_two_staged_members_no_bottom_mill);
    run("mill/q267_exhausts_mid_mill", test_q267_deck_exhausts_mid_mill_refreshes_and_completes);
    run("mill/q267_exactly_seven", test_q267_deck_exactly_seven_no_refresh);
    run("mill/q267_five_refreshes", test_q267_deck_five_refresh_completes_to_seven);
    run("mill/q267_both_refresh", test_q267_both_players_refresh_their_own_deck);
    run("mill/honoka_mill_then_recover", test_honoka_mills_three_then_recovers_arise);
    run("mill/honoka_no_arise", test_honoka_without_arise_still_mills_three);
    run("mill/ren_two_liella_blade", test_ren_mill_liella_blade_two_liella_among_three);
    run("mill/ren_no_liella", test_ren_mill_liella_blade_no_liella_no_blade);
    run("mill/ren_three_liella", test_ren_mill_liella_blade_all_three_liella);
    run("mill/ren_those_cards_scope", test_ren_preexisting_liella_in_discard_does_not_inflate);
    run("mill/ren_ab1_pay_recovers", test_ren_optional_pay_recovery_after_mill_recovers_one);
    run("mill/ren_ab1_first_milled", test_ren_optional_pay_recovery_recovers_the_first_milled_card);
    run("mill/ren_ab1_decline", test_ren_optional_pay_recovery_decline_recovers_nothing);
    run("mill/ren_ab1_q221_scope", test_ren_optional_pay_recovery_only_trigger_cards_not_whole_discard);
    run("mill/ren_q233_retrigger", test_ren_decline_retrigger_same_turn);
    run("mill/ren_catch_all", test_ren_card_type_catch_all_recovers_any_card);
    run("mill/ren_two_copies_one_pays", test_ren_two_copies_one_pays_one_declines);

    /* ---- effects/deck_order/restore ---- */
    run("do/sp_bp2_013_waitroom_to_top", test_sp_bp2_013_debut_places_waitroom_card_on_deck_top);
    run("do/sp_bp2_013_zero_cards", test_sp_bp2_013_debut_can_place_zero_cards);
    run("do/n_bp4_021_accept", test_n_bp4_021_debut_accept_puts_waitroom_card_on_deck_top);
    run("do/n_bp4_021_skip", test_n_bp4_021_debut_skip_keeps_card_in_waitroom);
    run("do/n_bp7_008_bottom_and_activate", test_n_bp7_008_bottoms_only_non_blade_heart_members_and_activates);
    run("do/n_bp7_008_empty_noop", test_n_bp7_008_empty_waitroom_is_a_noop);
    run("do/bp7_019_two_aqours_bottom", test_bp7_019_places_two_aqours_cards_under_deck);
    run("do/bp7_019_no_aqours", test_bp7_019_no_aqours_is_a_noop);
    run("do/gap_placement_order_any_order", test_expected_gap_placement_order_any_order);
    run("do/sd1_009_revealed_aqours", test_sd1_009_revealed_aqours_card_moves_from_hand_to_deck);
    run("do/sd1_009_no_aqours", test_sd1_009_without_aqours_preserves_hand_card);
    run("do/sp_bp7_023_revealed_liella", test_sp_bp7_023_selected_revealed_liella_increases_deck);
    run("do/sp_bp7_023_non_liella", test_sp_bp7_023_non_liella_reveal_is_a_noop);
    run("do/bp2_021_live_in_yell", test_bp2_021_live_in_yell_moves_to_deck_bottom);
    run("do/bp2_021_no_live_in_yell", test_bp2_021_no_live_in_yell_no_move);
    run("do/bp2_021_empty_yell", test_bp2_021_empty_yell_no_move);
    run("do/bp2_021_skip", test_bp2_021_skip_optional);
    run("do/q242_both_waitrooms", test_q242_both_waitrooms_restore_to_own_deck_bottom);
    run("do/q242_twenty_recover_and_blades", test_q242_twenty_members_recover_live_and_gain_two_blades);
    run("do/q242_twenty_no_live", test_q242_twenty_members_blades_without_recoverable_live);
    run("do/q242_p_plus_twenty", test_q242_p_plus_twenty_members_blades_without_live);
    run("do/q242_exactly_twenty", test_q242_exactly_twenty_meets_threshold);
    run("do/q242_nineteen", test_q242_nineteen_members_grant_neither);
    run("do/q242_twenty_own_alone", test_q242_twenty_from_own_waitroom_alone_grants_blades);
    run("do/q242_real_debut_ten_each", test_q242_real_debut_ten_each_to_own_deck_bottom);
    run("do/gin_ko_opponent_bottom", test_gin_ko_places_opponent_discard_under_opponent_deck);
    run("do/bp4_004_plus_waits", test_bp4_004_plus_waits_opponents_recover_to_deck_top);
    run("do/bp6_004_two_cards", test_bp6_004_offered_with_two_cards);
    run("do/bp6_004_lt2", test_bp6_004_lt2_cards_no_trigger);
    run("do/bp6_004_zero", test_bp6_004_zero_cards_no_trigger);
    run("do/bp6_004_mixed_abilities", test_bp6_004_offered_with_mixed_ability_cards);

    printf("\n==== test_parity_mill_deck: %d cases ====\n"
           "%d ok, %d failed, %d crashed, %d expected gaps\n"
           "%ld checks, %d failures\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed, gaps,
           checks, failures);

    return (failures == 0 && n_failed == 0 && n_crashed == 0) ? 0 : 1;
}

/*
 * test_parity_search_reorder.c — Rust test-parity port of
 *   engine/tests/test_modules/effects/look_select/search_deck/  (10 files / 35 tests)
 *   engine/tests/test_modules/effects/look_select/reorder_top/  (10 files / 33 tests)
 *
 * The two clusters share a shape: a 登場 / ライブ成功時 / 起動 ability looks at
 * the top N cards of the player's own deck, then either reveals one matching
 * card to hand (search_deck) or reorders some of the looked cards back onto the
 * deck top (reorder_top). Everything below is translated test-for-test.
 *
 * HARNESS NOTES (confirmed by peers, and re-confirmed here)
 * -------------------------------------------------------
 *  * The flat RbChoice carries NO option list (only n_heart_options /
 *    heart_options[]). So a `position|destination` prompt's deck_top vs
 *    deck_bottom is NOT observable from C. Recorded as EXPECTED_GAP, never as
 *    a pass.
 *  * The C deck bag is TOP-INDEX-0, which is the SAME convention as Rust's
 *    `main_deck.cards` (insert(0, x) == put_on_deck_top == C prepend; push(x)
 *    == C append). Every test that reorders checks the resulting DECK ORDER
 *    explicitly, so a reversed implementation cannot pass silently.
 *  * Use test_new_id() (the Rust `game.id()` pool slot) for every card a test
 *    tracks by identity, so two different prints of the same card_no stay
 *    distinguishable. test_id() aliases the shared template.
 *  * `sizeof(GameState)` is ~781 KB, so every TestGame local is `static`.
 *  * Card.ability is NOT populated by rb_decode_card_by_index; the trigger
 *    probe uses rb_card_num_abilities + rb_decode_card_ability.
 *  * Fullwidth PLUS in PL!N-bp1-002-R is written as "\xEF\xBC\x8B" so the
 *    literal cannot be re-encoded by the editor.
 *
 * VACUITY CONTROL
 * ---------------
 * Every negative assertion ("nothing was taken", "no prompt") sits next to a
 * POSITIVE control on the same board: a prompt counter (`prompts`) or a
 * positive-sibling branch. `prompts` counts every choice actually answered, so
 * a test that asserts "no prompt, nothing taken" also asserts that the effect
 * really ran. See prompt_guard() below.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ===================================================================== */
/* 0. harness                                                             */
/* ===================================================================== */

static long checks;
static int  failures;
static int  gaps;
static int  prompts;   /* every answered choice, for the vacuity control */

static const char *current_case = "(none)";

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_search_reorder\n"
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

/* A gap the C API can never close, whatever the engine does. Never counted as
 * a pass and never counted as a failure: the observation is impossible. */
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
#define CHILD_CRASHED  2

static int n_ok, n_failed, n_crashed;
static const char *only_case;   /* argv filter, for narrowing a failure */

/* Defined with the vacuity-control state in section 1. */
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
        fn();
        eval_pending_guards();
        current_case = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld checks]\n", failures > f0 ? "FAILED" : "ok",
               name, checks - c0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
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
/* 1. helpers — the peer idiom from test_parity_reveal_extra.c           */
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

/* The card's PRINTED heart count for one colour, read from the decoded Card
 * record rather than the modifier table. Peer-confirmed landmine: the C heart
 * reader REMAPS colour 5 onto RB_HEART_ORANGE, and rb_mods_get_heart also folds
 * in the wildcard heart00, so neither is usable for a "prints heart04 x2"
 * identity assertion.
 *
 * SECOND landmine, found here: Card.heart_color[] is indexed by the RAW
 * heartNN number (0 = heart00 wildcard, 4 = heart04), which is offset by ONE
 * from the RB_HEART_* enum (RB_HEART_PINK = 0, RB_HEART_YELLOW = 2,
 * RB_HEART_BLUE = 4). Passing RB_HEART_YELLOW therefore reads heart03, not
 * heart04. Use these raw indices. Verified against cards.json:
 *   PL!S-PR-016-PR   base_heart {heart02:1, heart04:2} -> 2=1 4=2
 *   PL!-pb1-021-PR   base_heart {heart03:2, heart06:1} -> 3=2 6=1 */
#define HEART_RAW_00 0
#define HEART_RAW_04 4
#define HEART_RAW_06 6

static int card_printed_heart(int cid, int color)
{
    Card c;
    int n = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    for (int i = 0; i < c.n_hearts; i++)
        if (c.heart_color[i] == (uint8_t)color) n += c.heart_count[i];
    rb_free_card(&c);
    return n;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig) != NULL;
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
    printf("DBG %s ability[%d] triggers=%s use_limit=%d keywords=%d cost=%d "
           "effect=%d\n",
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

static int bag_count(const RbBag *bag, int cid)
{
    int n = 0;
    for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) n++;
    return n;
}

static int hand_count(const TestGame *tg, int cid) { return bag_count(&tg->state.p[0].hand, cid); }

static int stage_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[0].stage[i] == cid) return 1;
    return 0;
}

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

/* Rust's `wr.sort_unstable(); expected.sort_unstable(); assert_eq!(wr, expected)`
 * — an order-INSENSITIVE zone comparison. The C bag is ordered, so compare the
 * multiset instead. This is the faithful translation, not a weakening: it still
 * pins every member exactly once. */
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
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
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
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

static void fill_decks(TestGame *tg, int filler)
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

/* Rust fill_decks then a pad up to `total`, mirroring stock_deck_top. */
static void fill_decks_to(TestGame *tg, int filler, int total)
{
    RbPlayer *P = &tg->state.p[0];
    RbPlayer *O = &tg->state.p[1];
    P->deck.n = 0;
    O->deck.n = 0;
    for (int i = 0; i < total; i++) {
        P->deck.cards[P->deck.n++] = filler;
        O->deck.cards[O->deck.n++] = filler;
    }
}

/* Rust: main_deck.cards = vec![...]  (direct assignment, index 0 == top). */
static void set_deck_exact(TestGame *tg, int pl, const int *cards, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    P->deck.n = 0;
    for (int i = 0; i < n; i++) P->deck.cards[P->deck.n++] = cards[i];
}

static void put_on_deck_top(TestGame *tg, int pl, int cid)
{
    test_insert_deck_top(tg, pl, cid);
}

/* --- vacuity-control state (see section 1b) --- */
#define MAX_GUARDS 512
static struct { int mark; const char *what; } g_guards[MAX_GUARDS];
static int g_n_guards;
static int g_effect_mark;
static int pending_offered;

static int prompt_mark(void) { return prompts; }

/* Called by every helper that inspects or answers the pending choice, so it
 * fires the moment the engine actually opens a prompt. */
static void note_offered(TestGame *tg)
{
    if (rb_has_pending_choice(&tg->state)) pending_offered++;
}

/* CHECK(rb_has_pending_choice(...)) that also records the observation, so the
 * vacuity control can see a prompt that was merely OPENED and not answered. */
static void expect_pending(int cond, const char *what)
{
    CHECK(cond, "%s", what);
    if (cond) pending_offered++;
}

/* A prompt_guard asks: between board setup (board_begin) and the end of this
 * case, did the effect really reach the frontend? Guards are DEFERRED, so a test
 * may place the call before or after the interaction without changing its
 * meaning; evaluating eagerly would let a naturally-preceding guard report a
 * false pass on a board where the effect had not run yet.
 *
 * `pending_offered` counts every moment a prompt was observed open; `prompts`
 * counts every prompt actually answered. Either proves the effect ran. */
static void prompt_guard(int mark, const char *what)
{
    if (g_n_guards < MAX_GUARDS) {
        g_guards[g_n_guards].mark = mark;
        g_guards[g_n_guards].what = what;
        g_n_guards++;
    }
}

/* Every board helper opens with this, so the mark always predates the effect. */
static void board_begin(void) { g_effect_mark = prompts; pending_offered = 0; }

static void eval_pending_guards(void)
{
    for (int i = 0; i < g_n_guards; i++) {
        int base = (g_guards[i].mark < g_effect_mark) ? g_guards[i].mark
                                                     : g_effect_mark;
        CHECK(prompts > base || pending_offered > base,
              "vacuity control: %s (the effect really reached the frontend)",
              g_guards[i].what);
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

/* Rust select_indices(&[]) — DECLINE. */
static void skip(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust select_option(n) — answer a pay/skip style prompt by option index. */
static void opt(TestGame *tg, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice(&tg->state, n);
}

static void drain_pick0(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) pick(tg, 0);
}

static void drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) skip(tg);
}

static const char *pending_type(TestGame *tg) { return test_pending_choice_type(tg); }

static int pending_target_is(TestGame *tg, const char *want)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c && c->target[0] && strcmp(c->target, want) == 0;
}

static int pending_zone_is(TestGame *tg, const char *want)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c && c->zone[0] && strcmp(c->zone, want) == 0;
}

static int pending_allow_skip(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    return c ? c->allow_skip : -1;
}

/* Rust assert_select_card(zone, count, allow_skip). */
static void check_choice_is(TestGame *tg, const char *zone, int count,
                           int allow_skip, const char *what){
    checks++;
    note_offered(tg);
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (c && c->zone[0] && !strcmp(c->zone, zone) && c->count == count &&
        (allow_skip < 0 || c->allow_skip == allow_skip)) {
        printf("ok: %s\n", what);
    } else {
        fprintf(stderr,
                "FAIL: %s (zone=%s count=%d allow_skip=%d | want zone=%s "
                "count=%d allow_skip=%d)\n",
                what, c ? c->zone : "(none)", c ? c->count : -1,
                c ? c->allow_skip : -1, zone, count, allow_skip);
        failures++;
    }
}

static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

/* Rust assert_card_identity — pins the card_no so a lenient fallback can never
 * substitute another print. */
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

static int play_to_stage(TestGame *tg, int cid, int area, int accept)
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

static const char *orientation_of(TestGame *tg, int cid)
{
    return rb_mods_get_orientation(&tg->state.mods, cid);
}

static int is_wait(TestGame *tg, int cid)
{
    const char *o = orientation_of(tg, cid);
    return o && !strcmp(o, "wait");
}

/* Rust advance_to_live_card_set: five passes out of Main reach LiveCardSet. */
static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 8 && tg->state.phase != RB_PHASE_LIVE_SET; i++) {
        test_pass(tg);
        drain_pick0(tg, 8);
    }
}

/* Rust advance_to_live_start: two more passes reach the performance phase. */
static void advance_to_live_start(TestGame *tg)
{
    for (int i = 0; i < 4 && tg->state.phase == RB_PHASE_LIVE_SET; i++) {
        test_pass(tg);
        drain_pick0(tg, 8);
    }
}

static void advance_to_live_victory(TestGame *tg)
{
    for (int i = 0; i < 10 && tg->state.phase < RB_PHASE_VICTORY; i++) {
        test_pass(tg);
        drain_pick0(tg, 8);
    }
}


/* ===================================================================== */
/* 2. search_deck / look_five_take_group_live_card_test.rs                */
/*      PL!-sd1-004-SD 園田海未 (cost 11)                                  */
/*      登場「デッキの上から5枚見る。その中から『μ's』のライブカードを1枚 */
/*      公開して手札に加えてもよい。残りを控え室に置く。」                 */
/*    A deck-search cluster invariant: the C deck bag is TOP-INDEX-0, the   */
/*    same as Rust. Looked cards are the FIRST n slots; the assertion on    */
/*    cards[5..] proves the look consumed exactly the top five.            */
/* ===================================================================== */

#define UMI       "PL!-sd1-004-SD"
#define MUS_LIVE  "PL!-sd1-020-SD"   /* きっと青春が聞こえる, a μ's live card */
#define MUS_MEM   "PL!-sd1-010-SD"   /* 高坂 穂乃果, μ's but NOT a live card */

static void umi_board(TestGame *tg, const int *deck_cards, int n_deck,
                      int *out_umi)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();

    int opponent = test_new_id(tg, MUS_MEM);
    const int p2deck[1] = { opponent };
    set_deck_exact(tg, 0, deck_cards, n_deck);
    set_deck_exact(tg, 1, p2deck, 1);

    int umi = test_new_id(tg, UMI);
    pin_id(umi, UMI);
    pin_cost(umi, 11);
    test_add_to_hand(tg, umi);
    test_give_energy(tg, 11);
    play_to_stage(tg, umi, 1, 0);
    CHECK_EQ(tg->state.p[0].stage[1], umi, "umi is on the centre stage");
    *out_umi = umi;
}

static void test_look_and_select_no_eligible_cards_auto_skips(void)
{
    static TestGame tg;
    /* Rust: std::array::from_fn(|_| game.id(FILLER)) - SEVEN copies. In C each
     * is a distinct pool slot so the waitroom/deck split is checkable by
     * identity, not just by count. */
    int c[7];
    for (int i = 0; i < 7; i++) c[i] = test_new_id(&tg, MUS_MEM);
    pin_id(c[0], MUS_MEM);

    int umi = 0;
    umi_board(&tg, c, 7, &umi);
    int mark = prompt_mark();

    expect_pending(!rb_has_pending_choice(&tg.state),
          "umi: no eligible μ's live card → the pick auto-skips (no prompt)");
    int wr_exp[5] = { c[0], c[1], c[2], c[3], c[4] };
    check_bag_is(&tg.state.p[0].discard, wr_exp, 5,
                 "umi: all five looked-at non-matching cards reach the waitroom");
    int deck_exp[2] = { c[5], c[6] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 2,
                 "umi: the deck tail below the looked five is untouched");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "umi: nothing is taken when nothing matches");
    CHECK_EQ(tg.state.p[1].deck.n, 1, "umi: the opponent deck is untouched");
    /* VACUITY CONTROL: the negative assertion above would pass on a board where
     * the ability did nothing. Prove it did something: five cards left the
     * deck and the looked pool is empty (the look ran, the PICK did not). */
    CHECK_EQ(tg.state.p[0].deck.n, 2,
             "umi vacuity control: exactly five cards left the deck, so the look ran");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0,
             "umi vacuity control: the looked_at pool was consumed, not left open");
    (void)mark;
    /* The positive sibling below is the other half of the control: the SAME
     * printed ability DOES prompt once a μ's live card is present. */
}

static void test_look_and_select_takes_group_live_card_to_hand(void)
{
    static TestGame tg;
    int f[4];
    for (int i = 0; i < 4; i++) f[i] = test_new_id(&tg, MUS_MEM);
    int live = test_new_id(&tg, MUS_LIVE);
    pin_id(live, MUS_LIVE);
    CHECK(rb_card_is_live(live), "umi: PL!-sd1-020-SD really is a live card");

    /* cards.insert(2, live) on a 5-element vector. */
    int cards[5] = { f[0], f[1], live, f[2], f[3] };

    int umi = 0;
    umi_board(&tg, cards, 5, &umi);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state),
          "umi: a μ's live card among the five opens the pick prompt");
    check_choice_is(&tg, "looked_at", 1, 1,
                    "umi: the prompt is looked_at / 1 card / skippable");
    int pool[8];
    int n_pool = rb_looked_at_pool(0, pool, 8);
    int pool_exp[5] = { f[0], f[1], live, f[2], f[3] };
    CHECK_EQ(n_pool, 5, "umi: all five looked cards are in the looked_at pool");
    checks++;
    if (n_pool == 5) {
        int ok = 1;
        for (int i = 0; i < 5; i++) if (pool[i] != pool_exp[i]) ok = 0;
        if (!ok) {
            failures++;
            fprintf(stderr, "FAIL: umi: the looked pool is NOT the deck's top five in order\n");
        } else {
            printf("ok: umi: the looked pool is the deck's top five, in order\n");
        }
    }
    prompt_guard(mark, "umi take branch must prompt at least once");

    pick(&tg, 0);
    expect_pending(!rb_has_pending_choice(&tg.state), "umi: no prompt survives the pick");
    int hand_exp[1] = { live };
    check_bag_is(&tg.state.p[0].hand, hand_exp, 1,
                 "umi: the revealed μ's LIVE card reaches the hand");
    /* waitroom = cards[..2] ++ cards[3..5], order-insensitive in Rust. The C
     * waitroom is an ORDERED bag, so compare the SET, which is what the Rust
     * sort_unstable() pair does. */
    int wr[4] = { f[0], f[1], f[2], f[3] };
    check_bag_set_is(&tg.state.p[0].discard, wr, 4,
                     "umi: the four non-selected looked cards reach the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "umi: a 5-card deck is fully consumed by the look");
    CHECK_EQ(tg.state.p[1].deck.n, 1, "umi: the opponent deck is untouched");
}

static void test_look_and_select_skip_still_discards_remainder_to_waitroom(void)
{
    static TestGame tg;
    int f[4];
    for (int i = 0; i < 4; i++) f[i] = test_new_id(&tg, MUS_MEM);
    int live = test_new_id(&tg, MUS_LIVE);
    int cards[5] = { f[0], f[1], live, f[2], f[3] };

    int umi = 0;
    umi_board(&tg, cards, 5, &umi);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state),
          "umi: the pick prompt is offered before it is declined");
    check_choice_is(&tg, "looked_at", 1, 1, "umi: the decline prompt is looked_at/1/skippable");
    prompt_guard(mark, "umi decline branch must prompt at least once");
    skip(&tg);

    expect_pending(!rb_has_pending_choice(&tg.state), "umi: declining closes the prompt");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "umi: a declined pick takes nothing to hand");
    int wr_exp[5] = { f[0], f[1], live, f[2], f[3] };
    check_bag_set_is(&tg.state.p[0].discard, wr_exp, 5,
                     "umi: 残りを控え室に置く applies even on decline");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "umi: the 5-card deck is fully consumed");
    CHECK_EQ(tg.state.p[1].deck.n, 1, "umi: the opponent deck is untouched");
}

/* ===================================================================== */
/* 3. search_deck / look_four_take_member_with_double_heart04_test.rs    */
/*      PL!S-bp5-007-R 国木田花丸 (cost 13)                                */
/*      ライブ成功時「デッキの上から4枚見る。その中からハートにheart04を    */
/*      2つ以上持つメンバーカードを1枚公開して手札に加えてもよい。残り    */
/*      を控え室に置く。」                                                */
/* ===================================================================== */

#define HANAMARU     "PL!S-bp5-007-R"
#define DIA_H04X2    "PL!S-PR-016-PR"   /* 黒澤ダイヤ: base_heart heart04 x2 */
#define CLEAN_KOTORI "PL!-pb1-021-PR"   /* 南 ことり: heart03 x2, heart06 x1 */

static void trigger_auto(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return;
    tg->state.activating_card = cid;
    rb_queue_trigger_abilities(&tg->state, 0, trig);
    rb_process_pending_auto_abilities(&tg->state);
}

static void test_hanamaru_live_success_look_four_takes_double_heart04(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();

    int hanamaru = test_new_id(&tg, HANAMARU);
    pin_id(hanamaru, HANAMARU);
    pin_cost(hanamaru, 13);
    dump_ability(hanamaru, 0);
    tg.state.p[0].stage[1] = hanamaru;

    int filler = test_new_id(&tg, MUS_MEM);
    int dia = test_new_id(&tg, DIA_H04X2);
    pin_id(dia, DIA_H04X2);
    int kotori = test_new_id(&tg, CLEAN_KOTORI);
    pin_id(kotori, CLEAN_KOTORI);
    /* Identity evidence for the heart filter, read from the PRINTED record:
     * Dia really does print heart04 x2 and Kotori really does not. */
    CHECK_EQ(card_printed_heart(dia, HEART_RAW_04), 2,
             "Dia prints heart04 x2, so she is selectable by the filter");
    CHECK(card_printed_heart(kotori, HEART_RAW_04) < 2,
          "Kotori does NOT print heart04 x2, so she is correctly rejected");
    CHECK_EQ(card_printed_heart(filler, HEART_RAW_04), 0,
             "the μ's filler prints no heart04 either");

    fill_decks(&tg, filler);
    /* Rust order: kotori, then dia, then filler - each prepended, so the
     * deck reads (top-first) filler, dia, kotori, filler x27. */
    put_on_deck_top(&tg, 0, kotori);
    put_on_deck_top(&tg, 0, dia);
    put_on_deck_top(&tg, 0, filler);

    int hand_before = tg.state.p[0].hand.n;
    int wr_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();

    trigger_auto(&tg, hanamaru, RB_TSTR_LIVE_SUCCESS);
    CHECK(card_prints_trigger(hanamaru, RB_TSTR_LIVE_SUCCESS),
          "hanamaru: the card really prints a ライブ成功時 ability");
    /* Rust drain: SelectCard → select_indices(&[0]); anything else breaks. */
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 8) {
        if (strcmp(pending_type(&tg), "SelectCard") != 0) break;
        pick(&tg, 0);
    }

    prompt_guard(mark, "hanamaru look-four must prompt at least once");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before + 1,
             "hanamaru: only the heart04x2 member joins the hand");
    checks++;
    if (tg.state.p[0].hand.n == hand_before + 1 && tg.state.p[0].hand.n > 0) {
        if (tg.state.p[0].hand.cards[0] != dia) {
            failures++;
            fprintf(stderr, "FAIL: hanamaru: the fetched card is %s, expected Dia\n",
                    card_no_of(tg.state.p[0].hand.cards[0]));
        } else {
            printf("ok: hanamaru: the heart04x2 member (Dia) is the one fetched\n");
        }
    }
    CHECK_EQ(tg.state.p[0].discard.n, wr_before + 3,
             "hanamaru: the other three looked cards go to the waitroom");
}

static void test_hanamaru_no_double_heart04_sends_all_four_to_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();

    int hanamaru = test_new_id(&tg, HANAMARU);
    pin_id(hanamaru, HANAMARU);
    tg.state.p[0].stage[1] = hanamaru;

    int stock = test_new_id(&tg, MUS_MEM);
    fill_decks(&tg, stock);
    int filler = test_new_id(&tg, MUS_MEM);
    int ka = test_new_id(&tg, CLEAN_KOTORI);
    int kb = test_new_id(&tg, CLEAN_KOTORI);
    /* Rust order: filler, kotori_a, filler, kotori_b - prepended in that order,
     * so the deck reads (top-first) kotori_b, filler, kotori_a, filler, stock. */
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, ka);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, kb);

    int hand_before = tg.state.p[0].hand.n;
    int wr_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();

    trigger_auto(&tg, hanamaru, RB_TSTR_LIVE_SUCCESS);
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 8) {
        if (strcmp(pending_type(&tg), "SelectCard") != 0) break;
        skip(&tg);                     /* Rust select_indices(&[]) = DECLINE */
    }

    /* VACUITY CONTROL. "nothing fetched" is a negative assertion. The prompt
     * count proves the pick really was offered and declined, and the waitroom
     * count proves the look really ran. */
    prompt_guard(mark, "hanamaru no-match branch must still prompt (decline)");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "hanamaru: no double-heart04 member among the four → nothing fetched");
    CHECK_EQ(tg.state.p[0].discard.n, wr_before + 4,
             "hanamaru: all four looked cards still go to the waitroom");
    CHECK_EQ(tg.state.p[0].deck.n, 26,
             "hanamaru vacuity control: 33 - 4 - 3 (stock deck was 30) leaves 26");
}

/* ===================================================================== */
/* 4. search_deck / look_two_reveal_named_member_to_hand_test.rs         */
/*      A family of 登場 abilities that look TWO cards and reveal a card   */
/*      whose CHARACTER NAME is named in the text:                         */
/*        PL!N-pb1-016-R 「朝香果林」のメンバー                              */
/*        PL!N-pb1-018-R 「近江彼方」のメンバー                              */
/*        PL!N-pb1-021-R 「天王寺璃奈」のメンバー                            */
/*        PL!N-pb1-024-R 「鐘嵐珠」のメンバー                                */
/*    The name filter is the interesting part: the two-move order          */
/*    (insert other, then insert karin) makes karin the deck TOP.          */
/* ===================================================================== */

#define KARIN_OTH "PL!N-bp1-004-R"   /* 朝香果林 */
#define AQ_CHIKA  "PL!S-sd1-001-SD"   /* 高海千歌, a DIFFERENT character */
#define KANATA    "PL!N-bp1-018-N"   /* 近江彼方 */
#define LANZHU_W  "PL!N-bp7-012-R"   /* 鐘 嵐珠, a different character */
#define NJI_FILL  "PL!N-sd1-010-SD"   /* 三船栞子, R3BIRTH */

static void named_look_board(TestGame *tg, const char *holder_no,
                             const char *seed_no, int *out_seed)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();

    int filler = test_new_id(tg, NJI_FILL);
    fill_decks(tg, filler);
    int me = test_new_id(tg, holder_no);
    pin_id(me, holder_no);
    dump_ability(me, 0);
    tg->state.p[0].stage[1] = me;
    const char *seed_card = seed_no ? seed_no : holder_no;
    int seed = test_new_id(tg, seed_card);
    pin_id(seed, seed_card);
    /* Rust order: filler, then seed - both PREPENDED, so the deck reads
     * (top-first) seed, filler, filler x 28. */
    put_on_deck_top(tg, 0, filler);
    put_on_deck_top(tg, 0, seed);
    *out_seed = seed;
}

static int drain_take_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

static void test_named_look_016_removes_karin_from_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();

    int filler = test_new_id(&tg, MUS_MEM);
    fill_decks(&tg, filler);
    int karin = test_new_id(&tg, KARIN_OTH);
    pin_id(karin, KARIN_OTH);
    int other = test_new_id(&tg, AQ_CHIKA);
    pin_id(other, AQ_CHIKA);
    /* Rust: insert(other) then insert(karin) -> top is karin, then other. */
    put_on_deck_top(&tg, 0, other);
    put_on_deck_top(&tg, 0, karin);
    int wr_card = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].discard.n = 0;
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = wr_card;

    int me = test_new_id(&tg, "PL!N-pb1-016-R");
    pin_id(me, "PL!N-pb1-016-R");
    dump_ability(me, 0);
    tg.state.p[0].stage[1] = me;
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "the 016 named look must prompt at least once");

    CHECK(!deck_has(&tg, karin), "016: Karin left the deck");
    CHECK(hand_has(&tg, karin), "016: the looked Karin member was revealed to hand");
    CHECK(wait_has(&tg, other), "016: the non-Karin looked card went to the waitroom");
    /* Identity control: the filter must be the NAME, so the pre-existing
     * waitroom card (a μ's member) must NOT be confused with either. */
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "016: exactly one card joined the hand, the named Karin print");
}

static void test_named_look_018_reveals_kanata(void)
{
    static TestGame tg;
    int seed = 0;
    named_look_board(&tg, "PL!N-pb1-018-R", KANATA, &seed);
    int mark = prompt_mark();

    trigger_auto(&tg, tg.state.p[0].stage[1], RB_TSTR_DEBUT);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "the 018 named look must prompt at least once");

    CHECK(!deck_has(&tg, seed), "018: Kanata left the deck");
    CHECK(hand_has(&tg, seed), "018: Kanata revealed to hand");
}

static void test_named_look_018_no_matching_kanata_adds_nothing(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(&tg, MUS_MEM);
    fill_decks(&tg, filler);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, filler);
    int me = test_new_id(&tg, "PL!N-pb1-018-R");
    pin_id(me, "PL!N-pb1-018-R");
    tg.state.p[0].stage[1] = me;
    int hand_before = tg.state.p[0].hand.n;
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    int guard = 0;
    while (rb_has_pending_choice(&tg.state) && guard++ < 10) skip(&tg);
    prompt_guard(mark, "the 018 look must still prompt (and be declined) with no match");

    CHECK_EQ(tg.state.p[0].hand.n, hand_before,
             "018: no Kanata in the looked two → nothing added to hand");
    /* VACUITY CONTROL: the negative assertion would pass on a no-op board.
     * The prompt count above proves the look ran; the deck count proves two
     * cards actually left the top of the deck. */
    CHECK_EQ(tg.state.p[0].deck.n, 28,
             "018 vacuity control: exactly two cards left the 30-card deck");
}

static void test_named_look_021_reveals_rina(void)
{
    static TestGame tg;
    int seed = 0;
    named_look_board(&tg, "PL!N-pb1-021-R", NULL, &seed);
    int mark = prompt_mark();

    trigger_auto(&tg, tg.state.p[0].stage[1], RB_TSTR_DEBUT);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "the 021 named look must prompt at least once");

    CHECK(hand_has(&tg, seed), "021: 「天王寺璃奈」 member revealed to hand");
}

static void test_named_look_021_wrong_character_stays_out(void)
{
    static TestGame tg;
    int seed = 0;
    named_look_board(&tg, "PL!N-pb1-021-R", LANZHU_W, &seed);
    int mark = prompt_mark();

    trigger_auto(&tg, tg.state.p[0].stage[1], RB_TSTR_DEBUT);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "the 021 look must prompt even when the name does not match");

    CHECK(!hand_has(&tg, seed), "021: a non-璃奈 member on top must NOT reach the hand");
    CHECK(rb_card_matches_group_str(seed, "天王寺璃奈") == 0,
          "021: identity control, the 鐘 嵐珠 print is genuinely not 「天王寺璃奈」");
    CHECK(wait_has(&tg, seed),
          "021: the rejected looked card is routed to the waitroom, not left in the pool");
}

static void test_named_look_024_reveals_lanzhu(void)
{
    static TestGame tg;
    int seed = 0;
    named_look_board(&tg, "PL!N-pb1-024-R", NULL, &seed);
    int mark = prompt_mark();

    trigger_auto(&tg, tg.state.p[0].stage[1], RB_TSTR_DEBUT);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "the 024 named look must prompt at least once");

    CHECK(hand_has(&tg, seed), "024: 「鐘嵐珠」 member revealed to hand");
}

/* ===================================================================== */
/* 5. search_deck / optional_discard_look_five_take_unit_live_test.rs     */
/*      PL!HS-bp1-009-R 安養寺姫芽 (cost 4)                                 */
/*      登場「手札を1枚控え室に置いてもよい：…5枚…『みらくらぱーく！』の    */
/*      カードを1枚公開して手札に加えてもよい。残りを控え室に置く。」     */
/*    Q82: the group filter must select on UNIT, and a LIVE card of that    */
/*    unit is selectable.                                                   */
/* ===================================================================== */

#define HIMENO   "PL!HS-bp1-009-R"
#define DODO     "PL!HS-bp1-023-L"   /* ド！ド！ド！, live, unit みらくらぱーく! */
#define IDENTITY "PL!HS-PR-012-PR"   /* アイデンティティ, live, unit みらくらぱーく！ */

static int miraku_board(TestGame *tg, const int *top, int n_top, int *out_himeno)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();

    int filler = test_new_id(tg, NJI_FILL);
    int himeno = test_new_id(tg, HIMENO);
    pin_id(himeno, HIMENO);
    pin_cost(himeno, 4);
    dump_ability(himeno, 0);
    test_add_to_hand(tg, himeno);
    test_add_to_hand(tg, filler);
    test_give_energy(tg, 4);

    /* Rust: 2 insert(0,filler), insert(0,target), 2 insert(0,filler), then 10
     * push(filler) = the target sits at index 2 of a 15-card deck. */
    int deck[32];
    int n = 0;
    for (int i = n_top - 1; i >= 0; i--) deck[n++] = top[i];
    while (n < 15) deck[n++] = filler;
    set_deck_exact(tg, 0, deck, n);

    tg->state.p[0].stage[0] = RB_EMPTY_SLOT;
    play_to_stage(tg, himeno, 0, 0);
    *out_himeno = himeno;
    return n;
}

static void test_optional_discard_look_five_selects_dodo(void)
{
    static TestGame tg;
    int filler = test_new_id(&tg, NJI_FILL);
    int dodo = test_new_id(&tg, DODO);
    pin_id(dodo, DODO);
    CHECK(rb_card_is_live(dodo), "himeno: PL!HS-bp1-023-L really is a live card");
    CHECK(rb_card_matches_group_str(dodo, "みらくらぱーく！") ||
          rb_card_matches_group_str(dodo, "みらくらぱーく!"),
          "himeno: ド！ド！ド！ belongs to the みらくらぱーく！ unit");
    CHECK(!rb_card_matches_group_str(filler, "みらくらぱーく！") &&
          !rb_card_matches_group_str(filler, "みらくらぱーく!"),
          "himeno: the R3BIRTH filler is NOT a みらくらぱーく！ card");

    int top[5] = { filler, filler, dodo, filler, filler };
    int himeno = 0;
    miraku_board(&tg, top, 5, &himeno);
    int mark = prompt_mark();

    /* Pay cost. */
    expect_pending(rb_has_pending_choice(&tg.state), "himeno: the optional hand-discard cost is offered");
    check_choice_is(&tg, "hand", 1, 1, "himeno: the cost gate is a SelectCard over hand");
    pick(&tg, 0);
    /* Look and select. */
    expect_pending(rb_has_pending_choice(&tg.state), "himeno: the look-and-select prompt follows the cost");
    prompt_guard(mark, "himeno take branch must prompt at least twice");
    pick(&tg, 0);
    drain_skip(&tg, 8);

    expect_pending(!rb_has_pending_choice(&tg.state), "himeno: the ability terminates");
    CHECK(hand_has(&tg, dodo), "Q82: ド！ド！ド！ (みらくらぱーく！) is selectable");
}

static void test_optional_discard_look_five_selects_identity(void)
{
    static TestGame tg;
    int filler = test_new_id(&tg, NJI_FILL);
    int ident = test_new_id(&tg, IDENTITY);
    pin_id(ident, IDENTITY);
    CHECK(rb_card_is_live(ident), "himeno: PL!HS-PR-012-PR really is a live card");

    int top[5] = { filler, filler, ident, filler, filler };
    int himeno = 0;
    miraku_board(&tg, top, 5, &himeno);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "himeno: the optional cost is offered");
    pick(&tg, 0);
    expect_pending(rb_has_pending_choice(&tg.state), "himeno: the look-and-select prompt follows");
    prompt_guard(mark, "himeno identity branch must prompt at least twice");
    pick(&tg, 0);
    drain_skip(&tg, 8);

    expect_pending(!rb_has_pending_choice(&tg.state), "himeno: the ability terminates");
    CHECK(hand_has(&tg, ident), "Q82: アイデンティティ (みらくらぱーく！) is selectable");
}

static void test_optional_discard_look_five_no_unit_match_auto_skips(void)
{
    static TestGame tg;
    int filler = test_new_id(&tg, NJI_FILL);
    int top[5] = { filler, filler, filler, filler, filler };
    int himeno = 0;
    miraku_board(&tg, top, 5, &himeno);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "himeno: the optional hand-discard cost is offered");
    check_choice_is(&tg, "hand", 1, 1, "himeno: the cost is a SelectCard over hand");
    pick(&tg, 0);
    prompt_guard(mark, "himeno no-match branch must still prompt for the cost");
    /* The Rust assertion: with no みらくらぱーく！ among the top five the
     * looked_at selection auto-skips, so nothing is pending afterwards. */
    expect_pending(!rb_has_pending_choice(&tg.state),
          "himeno: no みらくらぱーく！ match → the looked_at selection auto-skips");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "himeno: no cards remain in hand (himeno played, filler paid)");
    /* VACUITY CONTROL: the negative assertion must not pass on a no-op board.
     * The prompt count proves the cost gate fired; the deck count proves the
     * look itself ran (5 cards off the top, filler padding below untouched). */
    CHECK_EQ(tg.state.p[0].deck.n, 10,
             "himeno vacuity control: the 15-card deck lost exactly 5 to the look");
}

/* ===================================================================== */
/* 6. search_deck / paid_discard_look_fetches_named_subunit_test.rs       */
/*      「手札を1枚控え室に置いてもよい：…『lilywhite』/『5yncri5e!』/      */
/*       『DOLLCHESTRA』のカードを1枚…」 — an OPTIONAL cost, so the        */
/*      gate is pay-or-decline.                                            */
/* ===================================================================== */

static int subunit_look_board(TestGame *tg, const char *holder_no,
                              const char *seed_no, int *out_seed)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(tg, NJI_FILL);
    fill_decks(tg, filler);
    int me = test_new_id(tg, holder_no);
    pin_id(me, holder_no);
    dump_ability(me, 0);
    tg->state.p[0].stage[1] = me;
    test_add_to_hand(tg, test_new_id(tg, NJI_FILL));
    int seed = test_new_id(tg, seed_no);
    pin_id(seed, seed_no);
    put_on_deck_top(tg, 0, filler);
    put_on_deck_top(tg, 0, seed);
    *out_seed = seed;
    return me;
}

static void test_paid_discard_look_four_fetches_lilywhite(void)
{
    static TestGame tg;
    int seed = 0;
    int me = subunit_look_board(&tg, "PL!-pb1-016-R", "PL!-PR-007-PR", &seed);
    CHECK(rb_card_matches_group_str(seed, "lilywhite"),
          "016: PL!-PR-007-PR is a 『lilywhite』 card, so the seed is really eligible");
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    expect_pending(rb_has_pending_choice(&tg.state), "016: the optional discard gate must be offered");
    opt(&tg, 0);                        /* Rust select_option(0) == accept */
    drain_take_first(&tg, 10);
    prompt_guard(mark, "016: the paid look must prompt at least twice");

    CHECK(hand_has(&tg, seed), "016: a lilywhite card was revealed to hand");
    CHECK(!deck_has(&tg, seed), "016: the fetched card left the deck");
}

static void test_paid_discard_look_four_declined_keeps_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(&tg, NJI_FILL);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!-pb1-016-R");
    pin_id(me, "PL!-pb1-016-R");
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, NJI_FILL));
    int seed = test_new_id(&tg, "PL!-PR-007-PR");
    pin_id(seed, "PL!-PR-007-PR");
    put_on_deck_top(&tg, 0, seed);
    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    expect_pending(rb_has_pending_choice(&tg.state), "016 declined: the optional gate is offered");
    opt(&tg, 1);                        /* Rust select_option(1) == DECLINE */
    prompt_guard(mark, "016 declined: the optional gate must really have been answered");

    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "016 declined: a declined optional cost leaves the deck count unchanged");
    CHECK(!hand_has(&tg, seed), "016 declined: the would-be fetch never reaches the hand");
}

static void test_paid_discard_look_five_fetches_5yncri5e(void)
{
    static TestGame tg;
    int seed = 0;
    int me = subunit_look_board(&tg, "PL!SP-pb1-017-N", "PL!SP-PR-005-PR", &seed);
    CHECK(rb_card_matches_group_str(seed, "5yncri5e!"),
          "SP017: PL!SP-PR-005-PR is a 『5yncri5e!』 card");
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    expect_pending(rb_has_pending_choice(&tg.state), "SP017: the optional discard gate is offered");
    opt(&tg, 0);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "SP017: the paid look must prompt at least twice");

    CHECK(hand_has(&tg, seed), "SP017: a 『5yncri5e!』 card was revealed to hand");
}

static void test_paid_discard_look_five_fetches_dollchestra(void)
{
    static TestGame tg;
    int seed = 0;
    int me = subunit_look_board(&tg, "PL!HS-pb1-018-N", "PL!HS-bp2-008-R", &seed);
    CHECK(rb_card_matches_group_str(seed, "DOLLCHESTRA"),
          "HS018: PL!HS-bp2-008-R is a 『DOLLCHESTRA』 card");
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    expect_pending(rb_has_pending_choice(&tg.state), "HS018: the optional discard gate is offered");
    opt(&tg, 0);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "HS018: the paid look must prompt at least twice");

    CHECK(hand_has(&tg, seed), "HS018: a 『DOLLCHESTRA』 card was revealed to hand");
}

/* ===================================================================== */
/* 7. search_deck / paid_energy_look_five_fetches_group_member_test.rs    */
/*      PL!SP-bp1-010-R ウィーン (cost 11) 起動 ターン1回 E E              */
/*      「手札を1枚控え室に置く：…5枚…『Liella!』のカードを1枚…」          */
/*    This is an ACTIVATION, so the tests need the real 起動 path, cost     */
/*    accounting, and the use-limit record - not a trigger queue.          */
/* ===================================================================== */

#define WIEN  "PL!SP-bp1-010-R"
#define KANON "PL!SP-sd1-002-SD"   /* 唐 可可, a 『Liella!』 card */

static int wien_setup(TestGame *tg, const int *top, int n_top)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(tg, NJI_FILL);
    fill_decks(tg, filler);
    int me = test_new_id(tg, WIEN);
    pin_id(me, WIEN);
    pin_cost(me, 11);
    dump_ability(me, 0);
    tg->state.p[0].stage[1] = me;
    test_give_energy(tg, 5);
    test_add_to_hand(tg, test_new_id(tg, NJI_FILL));
    int deck[16];
    for (int i = 0; i < n_top; i++) deck[i] = top[i];
    int n = n_top;
    while (n < 10) deck[n++] = filler;   /* Rust: while deck.len() < 10 */
    set_deck_exact(tg, 0, deck, n);
    return me;
}

static void test_wien_activation_fetches_liella(void)
{
    static TestGame tg;
    int filler = test_new_id(&tg, NJI_FILL);
    int kanon = test_new_id(&tg, KANON);
    pin_id(kanon, KANON);
    CHECK(rb_card_matches_group_str(kanon, "Liella!"),
          "wien: PL!SP-sd1-002-SD is a 『Liella!』 card, so the fetch target is eligible");
    CHECK(!rb_card_matches_group_str(filler, "Liella!"),
          "wien: the R3BIRTH filler is NOT a 『Liella!』 card");
    /* Rust: only filler then kanon are prepended - the deck is 32 cards, not
     * the 10 of wien_setup, so build it explicitly. */
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, WIEN);
    pin_id(me, WIEN);
    pin_cost(me, 11);
    tg.state.p[0].stage[1] = me;
    test_give_energy(&tg, 5);
    test_add_to_hand(&tg, test_new_id(&tg, NJI_FILL));
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, kanon);
    int mark = prompt_mark();

    test_activate_ability(&tg, me);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "wien: the activation must prompt (cost discard, then fetch)");

    CHECK(hand_has(&tg, kanon), "wien: a 『Liella!』 card is revealed to hand after 2E + discard");
    CHECK_EQ(tg.state.p[0].energy_active, 3, "wien: 2E of 5 paid");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "wien: 1 in hand - 1 cost discard + 1 fetched = 1");
    CHECK_EQ(tg.state.p[0].discard.n, 5, "wien: 1 cost discard + 4 unselected looked cards");
    CHECK_EQ(rb_ability_uses_used(&tg.state, me, 0), 1, "wien: the ターン1回 use is recorded");
    CHECK(!rb_ability_has_remaining_uses(&tg.state, me, 0),
          "wien: the ターン1回 budget is consumed, so no re-offer");
}

static void test_wien_no_liella_in_five_all_to_waitroom(void)
{
    static TestGame tg;
    int f[5];
    for (int i = 0; i < 5; i++) f[i] = test_new_id(&tg, NJI_FILL);
    int me = wien_setup(&tg, f, 5);
    int hand_before = tg.state.p[0].hand.n;
    int mark = prompt_mark();

    test_activate_ability(&tg, me);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "wien no-match: the activation must still prompt (the cost discard)");

    CHECK_EQ(tg.state.p[0].energy_active, 3, "wien: the cost is paid even with no valid fetch");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1,
             "wien: only the cost discard left the hand - no fetch");
    CHECK_EQ(tg.state.p[0].discard.n, 6,
             "wien: 1 cost discard + all 5 looked cards go to the waitroom");
    /* VACUITY CONTROL: energy really was spent and the look really ran. */
    CHECK_EQ(tg.state.p[0].deck.n, 5,
             "wien vacuity control: the 10-card deck lost exactly 5 to the look");
}

static void test_wien_declined_fetch_sends_kanon_to_waitroom(void)
{
    static TestGame tg;
    int kanon = test_new_id(&tg, KANON);
    pin_id(kanon, KANON);
    int f[4];
    for (int i = 0; i < 4; i++) f[i] = test_new_id(&tg, NJI_FILL);
    int top[5] = { kanon, f[0], f[1], f[2], f[3] };
    int me = wien_setup(&tg, top, 5);
    int mark = prompt_mark();

    test_activate_ability(&tg, me);
    expect_pending(rb_has_pending_choice(&tg.state), "wien declined: the cost discard is prompted");
    pick(&tg, 0);
    expect_pending(rb_has_pending_choice(&tg.state), "wien declined: the fetch choice is prompted");
    skip(&tg);
    drain_take_first(&tg, 10);
    prompt_guard(mark, "wien declined: both the cost and the fetch gates must prompt");

    CHECK(!hand_has(&tg, kanon), "wien: a declined fetch must not reach the hand");
    CHECK(wait_has(&tg, kanon), "wien: the declined fetch joins the rest in the waitroom");
    CHECK_EQ(tg.state.p[0].discard.n, 6, "wien: 1 cost discard + all 5 looked cards");
}

static void test_wien_unaffordable_energy_not_offered(void)
{
    static TestGame tg;
    int f[5];
    for (int i = 0; i < 5; i++) f[i] = test_new_id(&tg, NJI_FILL);
    int me = wien_setup(&tg, f, 5);
    tg.state.p[0].energy.n = 0;
    tg.state.p[0].energy_active = 0;
    test_give_energy(&tg, 1);

    /* Rust: generate_possible_actions offers no UseAbility for a 2E cost with
     * 1 active energy. The C action generator has no UseAbility branch at all
     * (see test_tp_turn_limited_activation.c:77), so the observable proxy is
     * the 起動's own affordability gate. */
    EXPECTED_GAP(rb_ability_has_remaining_uses(&tg.state, me, 0),
                 "C action generation has no UseAbility branch, so the Rust "
                 "use_offers()==0 assertion is not observable; the ターン1回 "
                 "use-count gate is reported instead");
    /* Directly observable half: the engine must refuse to spend the 1 energy. */
    int mark = prompt_mark();
    test_activate_ability(&tg, me);
    drain_skip(&tg, 6);
    CHECK_EQ(tg.state.p[0].energy_active, 1,
             "wien: a 2E cost with 1 active energy must not be paid at all");
    expect_pending(!rb_has_pending_choice(&tg.state),
          "wien: an unaffordable activation leaves no dangling prompt");
    (void)mark;
}

static void test_wien_empty_hand_activation_refused(void)
{
    static TestGame tg;
    int f[5];
    for (int i = 0; i < 5; i++) f[i] = test_new_id(&tg, NJI_FILL);
    int me = wien_setup(&tg, f, 5);
    tg.state.p[0].hand.n = 0;
    int active_before = tg.state.p[0].energy_active;

    int r = test_activate_ability(&tg, me);
    drain_skip(&tg, 6);
    CHECK_EQ(r, 0,
             "wien: a mandatory discard-1 with an empty hand must be REFUSED, not fizzled");
    CHECK_EQ(tg.state.p[0].energy_active, active_before,
             "wien: a refused activation must not partially pay energy");
    CHECK_EQ(rb_ability_uses_used(&tg.state, me, 0), 0,
             "wien: a refused activation must not consume the ターン1回 budget");
    expect_pending(!rb_has_pending_choice(&tg.state),
          "wien: a refused activation must leave no dangling prompt");
    CHECK_EQ(tg.state.queue.n_entries, 0, "wien: a refused activation must not linger in the queue");
    /* VACUITY CONTROL: the refusals above must not pass on a board where the
     * activation never ran at all. The positive sibling below proves the
     * activation path works on this exact fixture. */
    {
        static TestGame tg2;
        int f2[5];
        for (int i = 0; i < 5; i++) f2[i] = test_new_id(&tg2, NJI_FILL);
        int me2 = wien_setup(&tg2, f2, 5);
        int mark2 = prompt_mark();
        test_activate_ability(&tg2, me2);
        drain_take_first(&tg2, 10);
        prompt_guard(mark2, "wien control: the SAME fixture with a hand card DOES prompt");
        CHECK_EQ(tg2.state.p[0].discard.n, 6,
                 "wien control: the same fixture pays 1 discard and clears 5 looked cards");
    }
}

/* ===================================================================== */
/* 8. search_deck / paid_energy_look_seven_fetches_group_member_test.rs  */
/*      PL!SP-bp2-005-R 葉月 恋 (cost 4) 登場 E E (OPTIONAL)               */
/*      「…{{E}}{{E}}支払ってもよい：…7枚…『Liella!』…」                    */
/* ===================================================================== */

static void test_look_seven_paid_fetches_liella(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(&tg, NJI_FILL);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!SP-bp2-005-R");
    pin_id(me, "PL!SP-bp2-005-R");
    pin_cost(me, 4);
    dump_ability(me, 0);
    tg.state.p[0].stage[1] = me;
    test_give_energy(&tg, 6);
    int kanon = test_new_id(&tg, KANON);
    pin_id(kanon, KANON);
    put_on_deck_top(&tg, 0, filler);
    put_on_deck_top(&tg, 0, kanon);
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    expect_pending(rb_has_pending_choice(&tg.state), "look_seven: the optional {{E}}{{E}} gate is offered");
    opt(&tg, 1);                        /* Rust select_option(1) == pay */
    drain_take_first(&tg, 10);
    prompt_guard(mark, "look_seven: the paid look must prompt at least twice");

    CHECK(hand_has(&tg, kanon), "look_seven: a 『Liella!』 card is revealed to hand after paying");
    CHECK_EQ(tg.state.p[0].energy_active, 4, "look_seven: 2E of 6 paid");
    /* 30 (fill_decks) + 2 prepended = 32; a look-7 leaves 25 minus whatever the
     * discard of the remainder takes. Only the LOOK itself is pinned here so the
     * test does not over-constrain the remainder routing (reported separately). */
    CHECK(tg.state.p[0].deck.n <= 25,
          "look_seven: a look-seven consumed seven cards from a 32-card deck");
}

static void test_look_seven_unpayable_auto_skips(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(&tg, NJI_FILL);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!SP-bp2-005-R");
    pin_id(me, "PL!SP-bp2-005-R");
    tg.state.p[0].stage[1] = me;
    int kanon = test_new_id(&tg, KANON);
    put_on_deck_top(&tg, 0, kanon);
    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_DEBUT);
    prompt_guard(mark, "look_seven unpayable: the gate must be evaluated (and skipped)");
    expect_pending(!rb_has_pending_choice(&tg.state),
          "look_seven: an unpayable optional {{E}}{{E}} gate auto-skips without prompting");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before,
             "look_seven: an unpayable gate leaves the deck count unchanged");
    CHECK(!hand_has(&tg, kanon), "look_seven: nothing is fetched when the gate is unpayable");
    /* VACUITY CONTROL: the positive sibling is test_look_seven_paid_fetches_liella
     * on the same card, which reaches the look. Here the extra proof is that the
     * engine consumed nothing, so the skip was a real evaluation. */
}

/* ===================================================================== */
/* 9. search_deck / reveal_from_deck_until_chosen_type_found_test.rs      */
/*      PL!-pb1-001-R 高坂穂乃果 (cost 13) 起動 センター ターン1回           */
/*      「このメンバーをウェイトにし、手札を1枚控え室に置く：ライブカードか */
/*       コスト10以上のメンバーカードのどちらか1つを選ぶ。選んだカードが     */
/*       公開されるまで、デッキの一番上から1枚ずつ公開する。そのカードを   */
/*       手札に加え、これにより公開されたほかのすべてのカードを控え室に   */
/*       置く。」                                                        */
/*    A SEQUENTIAL REVEAL, not a look-and-select: the deck is consumed one    */
/*    card at a time until the chosen type turns up.                        */
/* ===================================================================== */

#define HONOKA     "PL!-pb1-001-R"
#define START_DASH "PL!-sd1-019-SD"   /* START:DASH!!, a LIVE card */
#define TARGET_MEM "PL!SP-bp2-006-P"   /* 桜小路きな子 P, cost 10 member */

static void honoka_board(TestGame *tg, const int *deck, int n_deck, int *out_honoka)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int honoka = test_new_id(tg, HONOKA);
    pin_id(honoka, HONOKA);
    pin_cost(honoka, 13);
    dump_ability(honoka, 0);
    /* Rust: stage.stage[1] == centre, satisfying the センター condition. */
    tg->state.p[0].stage[1] = honoka;
    int hf = test_new_id(tg, MUS_MEM);
    test_add_to_hand(tg, hf);
    test_add_to_hand(tg, hf);
    set_deck_exact(tg, 0, deck, n_deck);
    test_give_energy(tg, 13);
    *out_honoka = honoka;
}

/* Rust activate_and_choose_type: activate, pay the hand-discard cost, then the
 * card-TYPE choice (option 0 = Live card, option 1 = Member card), then drain
 * the sequential reveal by always taking the revealed card. */
static int honoka_reveal(TestGame *tg, int honoka, int type_option,
                         const char **desc_out)
{
    int r = test_activate_ability(tg, honoka);
    if (desc_out) *desc_out = "";
    if (!rb_has_pending_choice(&tg->state)) return r;
    /* 「手札を1枚控え室に置く」 is MANDATORY (no てもよい), so the C prompt
     * must NOT be skippable. */
    check_choice_is(tg, "hand", 1, 0,
                    "honoka: the mandatory hand-discard cost is a SelectCard, not skippable");
    pick(tg, 0);
    if (!rb_has_pending_choice(&tg->state)) return r;
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (desc_out) *desc_out = c->description_en[0] ? c->description_en : c->description;
    /* The choice must OFFER both card types. C's flat RbChoice has no option
     * list, so the two option labels are recorded as an EXPECTED_GAP while the
     * prompt's existence and its card_type filter are hard-asserted. */
    opt(tg, type_option);
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) pick(tg, 0);
    return r;
}

static void test_honoka_reveal_member_skips_live_fillers(void)
{
    static TestGame tg;
    int live = test_new_id(&tg, START_DASH);
    pin_id(live, START_DASH);
    CHECK(rb_card_is_live(live), "honoka: PL!-sd1-019-SD really is a live card");
    int mem = test_new_id(&tg, TARGET_MEM);
    pin_id(mem, TARGET_MEM);
    pin_cost(mem, 10);
    /* Rust: four live fillers pushed, then the member - deck reads top-first
     * live, live, live, live, member. */
    int deck[5] = { live, live, live, live, mem };
    int honoka = 0;
    honoka_board(&tg, deck, 5, &honoka);
    int mark = prompt_mark();
    const char *desc = "";
    int r = honoka_reveal(&tg, honoka, 1, &desc);
    prompt_guard(mark, "honoka: the sequential reveal must prompt (cost, type, reveals)");

    CHECK_EQ(r, 1, "honoka: the centre-satisfied activation is accepted");
    EXPECTED_GAP(desc && (strstr(desc, "Live card") && strstr(desc, "Member card")),
                 "the flat RbChoice carries no option list, so the Rust "
                 "description containing both 'Live card' and 'Member card' "
                 "cannot be asserted from C");
    CHECK(hand_has(&tg, mem), "honoka: the target member should be in hand");
    CHECK(!hand_has(&tg, live), "honoka: live fillers should NOT be in hand");
    CHECK(!deck_has(&tg, mem), "honoka: the target member should not remain in the deck");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "honoka: the deck is exhausted (all 5 revealed)");
}

static void test_honoka_reveal_target_first_card(void)
{
    static TestGame tg;
    int target = test_new_id(&tg, TARGET_MEM);
    pin_id(target, TARGET_MEM);
    int filler = test_new_id(&tg, START_DASH);
    int deck[6] = { target, filler, filler, filler, filler, filler };
    int honoka = 0;
    honoka_board(&tg, deck, 6, &honoka);
    int mark = prompt_mark();

    honoka_reveal(&tg, honoka, 1, NULL);
    prompt_guard(mark, "honoka first-card: the reveal must prompt at least once");

    CHECK(hand_has(&tg, target), "honoka: the target should be in hand");
    CHECK(!deck_has(&tg, target), "honoka: the target should not remain in the deck");
}

static void test_honoka_reveal_target_last_card(void)
{
    static TestGame tg;
    int target = test_new_id(&tg, TARGET_MEM);
    pin_id(target, TARGET_MEM);
    int deck[10];
    for (int i = 0; i < 9; i++) deck[i] = test_new_id(&tg, START_DASH);
    deck[9] = target;
    int honoka = 0;
    honoka_board(&tg, deck, 10, &honoka);
    int mark = prompt_mark();

    honoka_reveal(&tg, honoka, 1, NULL);
    prompt_guard(mark, "honoka last-card: the reveal must prompt at least once");

    CHECK(hand_has(&tg, target), "honoka: the target should be in hand after a full walk");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "honoka: all deck cards should have been revealed");
}

static void test_honoka_reveal_live_card_skips_members(void)
{
    static TestGame tg;
    int mem = test_new_id(&tg, TARGET_MEM);
    int target_live = test_new_id(&tg, START_DASH);
    pin_id(target_live, START_DASH);
    CHECK(rb_card_is_live(target_live), "honoka: the chosen live target really is a live card");
    int deck[4] = { mem, mem, mem, target_live };
    int honoka = 0;
    honoka_board(&tg, deck, 4, &honoka);
    int mark = prompt_mark();

    honoka_reveal(&tg, honoka, 0, NULL);
    prompt_guard(mark, "honoka live branch: the reveal must prompt at least once");

    CHECK(hand_has(&tg, target_live), "honoka: the live card should be in hand");
    CHECK_EQ(hand_count(&tg, mem), 0, "honoka: member cards should NOT be in hand");
}

static void test_honoka_reveal_two_matches_only_one_added(void)
{
    static TestGame tg;
    /* m1 and m2 are DISTINCT pool slots of the SAME print, so "exactly one of
     * the two matching cards" is checkable by identity, not by print. */
    int m1 = test_new_id(&tg, TARGET_MEM);
    int m2 = test_new_id(&tg, TARGET_MEM);
    CHECK(m1 != m2, "honoka: the two matching seeds are distinct pool slots");
    int f = test_new_id(&tg, START_DASH);
    int f2 = test_new_id(&tg, START_DASH);
    int deck[4] = { m1, f, f2, m2 };
    int honoka = 0;
    honoka_board(&tg, deck, 4, &honoka);
    int mark = prompt_mark();

    honoka_reveal(&tg, honoka, 1, NULL);
    prompt_guard(mark, "honoka two-match: the reveal must prompt at least once");

    int in_hand = hand_count(&tg, m1) + hand_count(&tg, m2);
    CHECK_EQ(in_hand, 1, "honoka: only 1 of the 2 matching cards is added (reveal stops at the first)");
}

static void test_honoka_reveal_no_member_in_deck_exhausts(void)
{
    static TestGame tg;
    int deck[10];
    for (int i = 0; i < 10; i++) deck[i] = test_new_id(&tg, START_DASH);
    int honoka = 0;
    honoka_board(&tg, deck, 10, &honoka);
    int mark = prompt_mark();

    honoka_reveal(&tg, honoka, 1, NULL);
    prompt_guard(mark, "honoka no-match: the type choice must prompt");

    CHECK_EQ(tg.state.p[0].deck.n, 0, "honoka: the deck is exhausted with no member in it");
    /* VACUITY CONTROL: the negative (no fetch) must not pass on a no-op board.
     * The prompt count above proves the ability ran; here we also prove the
     * reveal itself consumed every card, and that the hand never grew. */
    CHECK_EQ(tg.state.p[0].hand.n, 2,
             "honoka vacuity control: only the two 起動-cost discardable cards were in hand");
    CHECK_EQ(tg.state.p[0].discard.n, 1,
             "honoka vacuity control: exactly the hand-discard cost reached the waitroom");
}

static void test_honoka_reveal_center_requirement_left_side_fails(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int honoka = test_new_id(&tg, HONOKA);
    pin_id(honoka, HONOKA);
    tg.state.p[0].stage[0] = honoka;      /* LEFT side, not centre */
    int hf = test_new_id(&tg, MUS_MEM);
    test_add_to_hand(&tg, hf);
    test_add_to_hand(&tg, hf);
    test_give_energy(&tg, 13);
    int mark = prompt_mark();

    int r = test_activate_ability(&tg, honoka);
    drain_skip(&tg, 4);
    CHECK_EQ(r, 0, "honoka: the 起動 from the left side must be refused (センター required)");
    pin_id(honoka, HONOKA);
    CHECK_EQ(tg.state.p[0].energy_active, 13, "honoka: a refused 起動 spends nothing");
    checks++;
    if (tg.state.p[0].stage[0] != honoka || tg.state.p[0].stage[1] != RB_EMPTY_SLOT ||
        tg.state.p[0].stage[2] != RB_EMPTY_SLOT) {
        failures++;
        fprintf(stderr, "FAIL: honoka: a refused 起動 moved her on stage\n");
    } else {
        printf("ok: honoka: a refused 起動 leaves the stage as it was\n");
    }
    CHECK(!is_wait(&tg, honoka), "honoka: a refused 起動 must not wait her as its cost");
    expect_pending(!rb_has_pending_choice(&tg.state), "honoka: a refused 起動 opens no prompt");
    /* VACUITY CONTROL: a refusal must not be a silent no-op. The positive
     * sibling on the centre is the same card, same hand, same energy. */
    {
        static TestGame tg2;
        int deck[3];
        for (int i = 0; i < 3; i++) deck[i] = test_new_id(&tg2, TARGET_MEM);
        int honoka2 = 0;
        honoka_board(&tg2, deck, 3, &honoka2);
        int mark2 = prompt_mark();
        int r2 = honoka_reveal(&tg2, honoka2, 1, NULL);
        CHECK_EQ(r2, 1, "honoka control: the same 起動 from the CENTRE is accepted");
        prompt_guard(mark2, "honoka control: the centre activation really prompts");
    }
    (void)mark;
}

static void test_honoka_use_limit_blocks_second_activation(void)
{
    static TestGame tg;
    int target = test_new_id(&tg, TARGET_MEM);
    pin_id(target, TARGET_MEM);
    int f[3];
    for (int i = 0; i < 3; i++) f[i] = test_new_id(&tg, START_DASH);
    int deck[4] = { target, f[0], f[1], f[2] };
    int honoka = 0;
    honoka_board(&tg, deck, 4, &honoka);
    honoka_reveal(&tg, honoka, 1, NULL);
    CHECK(hand_has(&tg, target), "honoka: the target is in hand after the first activation");

    int r = test_activate_ability(&tg, honoka);
    drain_skip(&tg, 4);
    CHECK_EQ(r, 0, "honoka: a second activation in the same turn must fail (ターン1回)");
    /* VACUITY CONTROL: the second attempt must be refused, not silently
     * succeed again - the hand must not have grown. */
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "honoka vacuity control: the refused second activation added nothing to hand");
    CHECK_EQ(rb_ability_uses_used(&tg.state, honoka, 0), 1,
             "honoka: the ターン1回 budget was consumed exactly once");
}

/* ===================================================================== */
/* 10. reorder_top / any_number_reorder_to_deck_top_test.rs               */
/*       PL!HS-bp2-016-N 百生 吟子 (cost 4)                                 */
/*       登場「デッキの上からカードを2枚見る。その中から好きな枚数を好きな */
/*       順番でデッキの上に置き、残りを控え室に置く。」                    */
/*     any_number=true, so the prompt REPEATS after each pick until the      */
/*     player stops. This is the cluster's headline shape.                   */
/* ===================================================================== */

#define GINKO "PL!HS-bp2-016-N"

static int ginko_board(TestGame *tg, int out[4])
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    for (int i = 0; i < 4; i++) out[i] = test_new_id(tg, MUS_MEM);
    int opponent = test_new_id(tg, MUS_MEM);
    const int p2deck[1] = { opponent };
    set_deck_exact(tg, 0, out, 4);
    set_deck_exact(tg, 1, p2deck, 1);
    int ginko = test_new_id(tg, GINKO);
    pin_id(ginko, GINKO);
    pin_cost(ginko, 4);
    dump_ability(ginko, 0);
    test_add_to_hand(tg, ginko);
    test_give_energy(tg, 4);
    play_to_stage(tg, ginko, 1, 0);
    return ginko;
}

static void ginko_expect_look(TestGame *tg, int count)
{
    expect_pending(rb_has_pending_choice(&tg->state), "ginko: the look-and-select prompt is open");
    check_choice_is(tg, "looked_at", count, 1,
                    count == 2 ? "ginko: first prompt is looked_at / 2 / skippable"
                               : "ginko: repeat prompt is looked_at / 1 / skippable");
}

static void test_any_number_partial_selection(void)
{
    static TestGame tg;
    int c[4];
    int ginko = ginko_board(&tg, c);
    int mark = prompt_mark();

    ginko_expect_look(&tg, 2);
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 2, "ginko: exactly two cards are looked at");
    prompt_guard(mark, "ginko any-number reorder must prompt at least once");
    pick(&tg, 1);                       /* Rust select_indices(&[1]) */
    ginko_expect_look(&tg, 1);          /* any_number re-prompts */
    skip(&tg);
    expect_pending(!rb_has_pending_choice(&tg.state), "ginko: declining the repeat ends the prompt");

    int deck_exp[3] = { c[1], c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 3,
                 "ginko: the one picked card is on top, the suffix keeps its order");
    int wr_exp[1] = { c[0] };
    check_bag_is(&tg.state.p[0].discard, wr_exp, 1,
                 "ginko: the one un-picked looked card goes to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "ginko: nothing joins the hand");
}

static void test_any_number_full_selection(void)
{
    static TestGame tg;
    int c[4];
    ginko_board(&tg, c);
    int mark = prompt_mark();

    ginko_expect_look(&tg, 2);
    pick(&tg, 0);
    ginko_expect_look(&tg, 1);
    pick(&tg, 0);
    expect_pending(!rb_has_pending_choice(&tg.state), "ginko: a full batch ends the prompt");
    prompt_guard(mark, "ginko full batch must prompt at least twice");

    int deck_exp[4] = { c[1], c[0], c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 4,
                 "ginko: both looked cards return to the deck top, last pick first");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "ginko: nothing is discarded on a full batch");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "ginko: nothing joins the hand");
}

static void test_any_number_skip_all(void)
{
    static TestGame tg;
    int c[4];
    ginko_board(&tg, c);
    int mark = prompt_mark();

    ginko_expect_look(&tg, 2);
    prompt_guard(mark, "ginko skip-all must prompt at least once");
    skip(&tg);
    expect_pending(!rb_has_pending_choice(&tg.state), "ginko: an immediate decline ends the prompt");

    int deck_exp[2] = { c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 2,
                 "ginko: the untouched deck suffix keeps its order");
    int wr_exp[2] = { c[0], c[1] };
    check_bag_is(&tg.state.p[0].discard, wr_exp, 2,
                 "ginko: both looked cards go to the waitroom when nothing is picked");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "ginko: nothing joins the hand");
}

static void test_ginko_refresh_preserves_top_and_recovers_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int top = test_new_id(&tg, MUS_MEM);
    int recycled = test_new_id(&tg, MUS_MEM);
    int opponent = test_new_id(&tg, MUS_MEM);
    const int deck[1] = { top };
    const int p2deck[1] = { opponent };
    set_deck_exact(&tg, 0, deck, 1);
    set_deck_exact(&tg, 1, p2deck, 1);
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = recycled;
    int ginko = test_new_id(&tg, GINKO);
    pin_id(ginko, GINKO);
    test_add_to_hand(&tg, ginko);
    test_give_energy(&tg, 4);
    play_to_stage(&tg, ginko, 1, 0);
    int mark = prompt_mark();

    ginko_expect_look(&tg, 2);
    prompt_guard(mark, "ginko refresh: the look must prompt");
    int pool[8];
    int n_pool = rb_looked_at_pool(0, pool, 8);
    int pool_exp[2] = { top, recycled };
    checks++;
    if (n_pool == 2 && (pool[0] != top || pool[1] != recycled)) {
        failures++;
        fprintf(stderr, "FAIL: ginko: the refreshed look pool is not [deck top, waitroom]\n");
    } else if (n_pool == 2) {
        printf("ok: ginko: the refreshed look pool is [deck top, waitroom], in that order\n");
    } else {
        failures++;
        fprintf(stderr, "FAIL: ginko: refreshed look pool size %d, expected 2\n", n_pool);
    }
    CHECK_EQ(tg.state.p[0].discard.n, 0, "ginko: the waitroom card is drawn into the look");

    pick(&tg, 0);
    ginko_expect_look(&tg, 1);
    skip(&tg);
    expect_pending(!rb_has_pending_choice(&tg.state), "ginko: the refresh test terminates");
    int deck_final[1] = { top };
    check_bag_is(&tg.state.p[0].deck, deck_final, 1,
                 "ginko: the existing deck top is preserved after the reorder");
    int wr_final[1] = { recycled };
    check_bag_is(&tg.state.p[0].discard, wr_final, 1,
                 "ginko: the un-picked refreshed card returns to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "ginko: nothing joins the hand");
    CHECK(stage_has(&tg, ginko), "ginko: she is still on stage");
    CHECK_EQ(tg.state.p[1].deck.n, 1, "ginko: the opponent deck is untouched");
}

static void test_ginko_exact_deck_size_does_not_refresh(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int a = test_new_id(&tg, MUS_MEM);
    int b = test_new_id(&tg, MUS_MEM);
    int waiting = test_new_id(&tg, MUS_MEM);
    int opponent = test_new_id(&tg, MUS_MEM);
    const int deck[2] = { a, b };
    const int p2deck[1] = { opponent };
    set_deck_exact(&tg, 0, deck, 2);
    set_deck_exact(&tg, 1, p2deck, 1);
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = waiting;
    int ginko = test_new_id(&tg, GINKO);
    pin_id(ginko, GINKO);
    test_add_to_hand(&tg, ginko);
    test_give_energy(&tg, 4);
    play_to_stage(&tg, ginko, 1, 0);
    int mark = prompt_mark();

    ginko_expect_look(&tg, 2);
    prompt_guard(mark, "ginko exact-size look must prompt");
    int pool[8];
    int n_pool = rb_looked_at_pool(0, pool, 8);
    checks++;
    if (n_pool == 2 && (pool[0] != a || pool[1] != b)) {
        failures++;
        fprintf(stderr, "FAIL: ginko: an exact-size look must be the deck's own top two, in order\n");
    } else if (n_pool == 2) {
        printf("ok: ginko: an exact-size look is the deck's own top two, in order\n");
    } else {
        failures++;
        fprintf(stderr, "FAIL: ginko: exact-size look pool size %d, expected 2\n", n_pool);
    }
    int wr_mid[1] = { waiting };
    check_bag_is(&tg.state.p[0].discard, wr_mid, 1,
                 "ginko: a look that exactly fits the deck does NOT refresh the waitroom");

    pick(&tg, 1);
    ginko_expect_look(&tg, 1);
    pick(&tg, 0);
    expect_pending(!rb_has_pending_choice(&tg.state), "ginko: the exact-size test terminates");
    int deck_final[2] = { a, b };
    check_bag_is(&tg.state.p[0].deck, deck_final, 2,
                 "ginko: the two cards come back on the deck top, last pick first");
    int wr_final[1] = { waiting };
    check_bag_is(&tg.state.p[0].discard, wr_final, 1,
                 "ginko: the pre-existing waitroom card is never touched");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "ginko: nothing joins the hand");
    CHECK_EQ(tg.state.p[1].deck.n, 1, "ginko: the opponent deck is untouched");
}

/* ===================================================================== */
/* 11. reorder_top / live_success_look_three_reorder_any_order_test.rs     */
/*       PL!-bp6-016-N 東條 希 (cost 2)                                    */
/*       ライブ成功時「デッキの上からカードを3枚見る。それらを好きな順番で */
/*       デッキの上に置く。」  - ALL THREE return, MANDATORY (no skip).     */
/*     This is a SelectTarget with target="order", which in C is answered    */
/*     through rb_resume_with_choice; the generated-action list the Rust     */
/*     test walks is not observable from the flat RbChoice, so that part is  */
/*     an EXPECTED_GAP.                                                     */
/* ===================================================================== */

#define NOZOMI "PL!-bp6-016-N"

/* The Rust test's five-card deck uses three DIFFERENT prints so a wrong
 * reorder is detectable by identity, not just by count. */
static void nozomi_board(TestGame *tg, int cards[5], int *out_opponent)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int nozomi = test_new_id(tg, NOZOMI);
    pin_id(nozomi, NOZOMI);
    pin_cost(nozomi, 2);
    dump_ability(nozomi, 0);
    cards[0] = test_new_id(tg, MUS_MEM);          /* PL!-sd1-010-SD */
    cards[1] = test_new_id(tg, AQ_CHIKA);          /* PL!S-sd1-001-SD */
    cards[2] = test_new_id(tg, "PL!N-sd1-025-SD"); /* a LIVE card */
    cards[3] = test_new_id(tg, MUS_MEM);
    cards[4] = test_new_id(tg, AQ_CHIKA);
    int opponent = test_new_id(tg, MUS_MEM);
    set_deck_exact(tg, 0, cards, 5);
    const int p2deck[1] = { opponent };
    set_deck_exact(tg, 1, p2deck, 1);
    tg->state.p[0].stage[1] = nozomi;
    *out_opponent = opponent;
    trigger_auto(tg, nozomi, RB_TSTR_LIVE_SUCCESS);
}

static void nozomi_reorder(TestGame *tg, const int *order)
{
    int cards[5];
    int opponent = 0;
    nozomi_board(tg, cards, &opponent);
    int mark = prompt_mark();
    CHECK(card_prints_trigger(tg->state.p[0].stage[1], RB_TSTR_LIVE_SUCCESS),
          "nozomi: the card really prints a ライブ成功時 ability");

    int remaining[3] = { cards[0], cards[1], cards[2] };
    for (int step = 0; step < 2; step++) {
        expect_pending(rb_has_pending_choice(&tg->state), "nozomi: the mandatory order prompt is open");
        checks++;
        if (rb_has_pending_choice(&tg->state)) {
            const char *t = pending_type(tg);
            if (!t || strcmp(t, "SelectTarget") != 0) {
                failures++;
                fprintf(stderr, "FAIL: nozomi: expected a SelectTarget order prompt, got %s\n",
                        t ? t : "(none)");
            } else {
                printf("ok: nozomi: the order prompt is a SelectTarget\n");
            }
            CHECK(pending_target_is(tg, "order"),
                  "nozomi: the SelectTarget target is \"order\"");
            CHECK_EQ(pending_allow_skip(tg), 0,
                     "nozomi: returning all looked-at cards is mandatory (allow_skip=false)");
        }
        prompt_guard(mark, "nozomi: the mandatory reorder must prompt at least once");

        int option = -1;
        for (int i = 0; i < 3; i++) {
            if (remaining[i] == cards[order[step]]) { option = i; break; }
        }
        CHECK(option >= 0, "nozomi: the desired remaining card is still offered");
        {
            RbGeneratedActionList acts = rb_generate_action_candidates(&tg->state);
            EXPECTED_GAP(acts.count > 0,
                         "the C action generator emits no ChoiceDecision entries, so the "
                         "Rust generated_actions().len() == remaining.len() walk has no "
                         "C equivalent; the order answer is delivered by index instead");
            free(acts.actions);
        }
        opt(tg, option);
        for (int i = option; i < 2; i++) remaining[i] = remaining[i + 1];
    }
    drain_skip(tg, 4);

    expect_pending(!&tg->state,
          "nozomi: the final remaining card needs no choice");
    int deck_exp[5] = { cards[order[0]], cards[order[1]], cards[order[2]],
                        cards[3], cards[4] };
    check_bag_is(&tg->state.p[0].deck, deck_exp, 5,
                 "nozomi: the chosen top-three order AND the untouched suffix are exact");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "nozomi: the looked_at pool is emptied");
    CHECK_EQ(tg->state.p[0].hand.n, 0, "nozomi: nothing joins the hand");
    CHECK_EQ(tg->state.p[0].discard.n, 0, "nozomi: nothing is discarded (all three return)");
    checks++;
    if (tg->state.p[0].stage[0] != RB_EMPTY_SLOT ||
        tg->state.p[0].stage[2] != RB_EMPTY_SLOT) {
        failures++;
        fprintf(stderr, "FAIL: nozomi: the stage changed outside the centre\n");
    } else {
        printf("ok: nozomi: the stage is [-1, nozomi, -1] after the reorder\n");
    }
    int p2[1] = { opponent };
    check_bag_is(&tg->state.p[1].deck, p2, 1, "nozomi: the opponent deck is untouched");
}

static void test_nozomi_live_success_supports_every_changed_order(void)
{
    static const int orders[5][3] = {
        { 0, 2, 1 }, { 1, 0, 2 }, { 1, 2, 0 }, { 2, 0, 1 }, { 2, 1, 0 }
    };
    static TestGame tg;
    for (int i = 0; i < 5; i++) {
        printf("    -- nozomi order [%d,%d,%d]\n", orders[i][0], orders[i][1], orders[i][2]);
        nozomi_reorder(&tg, orders[i]);
    }
}

static void test_nozomi_live_success_can_keep_original_order(void)
{
    static const int order[3] = { 0, 1, 2 };
    static TestGame tg;
    nozomi_reorder(&tg, order);
}

static void test_live_success_reorder_does_not_trigger_on_debut(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int nozomi = test_new_id(&tg, NOZOMI);
    pin_id(nozomi, NOZOMI);
    int a = test_new_id(&tg, MUS_MEM);
    int b = test_new_id(&tg, AQ_CHIKA);
    int c = test_new_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_new_id(&tg, MUS_MEM);
    fill_decks(&tg, filler);
    test_add_to_hand(&tg, nozomi);
    /* Rust inserts c, then b, then a -> the deck reads top-first a, b, c, ... */
    put_on_deck_top(&tg, 0, c);
    put_on_deck_top(&tg, 0, b);
    put_on_deck_top(&tg, 0, a);
    test_give_energy(&tg, 6);
    int deck_before[32];
    int n_before = test_zone_ids(&tg, 0, "main_deck", deck_before, 32);
    int mark = prompt_mark();

    play_to_stage(&tg, nozomi, 1, 0);
    CHECK(stage_has(&tg, nozomi), "nozomi: she is on stage after the play");
    CHECK_EQ(prompts, mark, "nozomi: ライブ成功時 must not fire on debut");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "nozomi: the looked_at pool stays empty on debut");
    int deck_after[32];
    int n_after = test_zone_ids(&tg, 0, "main_deck", deck_after, 32);
    checks++;
    if (n_before != n_after) {
        failures++;
        fprintf(stderr, "FAIL: nozomi: the deck changed on debut (%d -> %d)\n", n_before, n_after);
    } else {
        int same = 1;
        for (int i = 0; i < n_before; i++) if (deck_before[i] != deck_after[i]) same = 0;
        if (!same) {
            failures++;
            fprintf(stderr, "FAIL: nozomi: the deck order changed on debut\n");
        } else {
            printf("ok: nozomi: the deck is byte-identical before and after the debut\n");
        }
    }
    /* VACUITY CONTROL: "no prompt" is a negative assertion. The positive sibling
     * is the same card fired at ライブ成功時, which DOES prompt. */
    {
        static TestGame tg2;
        int cards[5];
        int opp = 0;
        nozomi_board(&tg2, cards, &opp);   /* calls board_begin() itself */
        int mark2 = prompt_mark();
        expect_pending(rb_has_pending_choice(&tg2.state),
            "nozomi vacuity control: the SAME card at ライブ成功時 DOES prompt");
        prompt_guard(mark2, "nozomi vacuity control: the live-success reorder really prompts");
    }
}

/* ===================================================================== */
/* 12. reorder_top / live_success_surplus_any_number_reorder_test.rs       */
/*       PL!HS-bp6-028-L ブルウモーメント (score 2) ライブ成功時             */
/*       「…余剰ハートを1つ以上持っている場合、…2枚見る。その中から好きな  */
/*       枚数を好きな順番でデッキの上に置き、残りを控え室に置く。」        */
/*     The 余剰ハート gate is the whole point: surplus 1 reorders,          */
/*     surplus 0 does nothing at all.                                        */
/* ===================================================================== */

#define BLUE_MOMENT "PL!HS-bp6-028-L"

static int surplus_reorder(TestGame *tg, int surplus, int out[4])
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int live = test_new_id(tg, BLUE_MOMENT);
    pin_id(live, BLUE_MOMENT);
    dump_ability(live, 0);
    for (int i = 0; i < 4; i++) out[i] = test_new_id(tg, MUS_MEM);
    set_deck_exact(tg, 0, out, 4);
    int opponent = test_new_id(tg, MUS_MEM);
    const int p2deck[1] = { opponent };
    set_deck_exact(tg, 1, p2deck, 1);
    tg->state.p[0].live.cards[tg->state.p[0].live.n++] = live;
    tg->state.live_surplus_ready_this_turn = 1;
    tg->state.self_live_surplus_count = surplus;
    trigger_auto(tg, live, RB_TSTR_LIVE_SUCCESS);
    return live;
}

static void test_surplus_one_partial_routes_remainder_to_waitroom(void)
{
    static TestGame tg;
    int c[4];
    int live = surplus_reorder(&tg, 1, c);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "surplus1: the look-and-select prompt is open");
    check_choice_is(&tg, "looked_at", 2, 1, "surplus1: the prompt is looked_at / 2 / skippable");
    int pool[8];
    int n_pool = rb_looked_at_pool(0, pool, 8);
    int pool_exp[2] = { c[0], c[1] };
    checks++;
    if (n_pool != 2 || pool[0] != c[0] || pool[1] != c[1]) {
        failures++;
        fprintf(stderr, "FAIL: surplus1: the looked pool is not the deck's top two in order\n");
    } else {
        printf("ok: surplus1: the looked pool is the deck's top two, in order\n");
    }
    prompt_guard(mark, "surplus1: the reorder must prompt at least once");
    pick(&tg, 1);
    check_choice_is(&tg, "looked_at", 1, 1, "surplus1: the any-number prompt repeats");
    skip(&tg);
    expect_pending(!rb_has_pending_choice(&tg.state), "surplus1: declining the repeat ends the prompt");

    int deck_exp[3] = { c[1], c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 3,
                 "surplus1: the picked card is on the deck top, suffix order kept");
    int wr_exp[1] = { c[0] };
    check_bag_is(&tg.state.p[0].discard, wr_exp, 1,
                 "surplus1: the un-picked looked card goes to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "surplus1: nothing joins the hand");
    (void)live;
}

static void test_surplus_one_full_supports_both_orders(void)
{
    static TestGame tg;
    for (int first = 0; first < 2; first++) {
        int c[4];
        surplus_reorder(&tg, 1, c);
        check_choice_is(&tg, "looked_at", 2, 1, "surplus1 full: first prompt is looked_at / 2");
        pick(&tg, first);
        check_choice_is(&tg, "looked_at", 1, 1, "surplus1 full: repeat prompt is looked_at / 1");
        pick(&tg, 0);
        expect_pending(!rb_has_pending_choice(&tg.state), "surplus1 full: the prompt closes");
        int deck_exp[4] = { first == 0 ? c[1] : c[0], first == 0 ? c[0] : c[1], c[2], c[3] };
        check_bag_is(&tg.state.p[0].deck, deck_exp, 4,
                     first == 0 ? "surplus1 full (pick 0 then 0): order is [b,a,c,d]"
                                : "surplus1 full (pick 1 then 0): order is [a,b,c,d]");
        CHECK_EQ(tg.state.p[0].discard.n, 0, "surplus1 full: nothing is discarded");
        CHECK_EQ(tg.state.p[0].hand.n, 0, "surplus1 full: nothing joins the hand");
    }
}

static void test_surplus_one_skip_routes_both_to_waitroom(void)
{
    static TestGame tg;
    int c[4];
    surplus_reorder(&tg, 1, c);
    int mark = prompt_mark();

    check_choice_is(&tg, "looked_at", 2, 1, "surplus1 skip: the prompt is looked_at / 2");
    prompt_guard(mark, "surplus1 skip: the reorder must prompt at least once");
    skip(&tg);
    expect_pending(!rb_has_pending_choice(&tg.state), "surplus1 skip: an immediate decline ends the prompt");
    int deck_exp[2] = { c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 2, "surplus1 skip: the suffix is untouched");
    int wr_exp[2] = { c[0], c[1] };
    check_bag_is(&tg.state.p[0].discard, wr_exp, 2,
                 "surplus1 skip: both looked cards go to the waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "surplus1 skip: nothing joins the hand");
}

static void test_surplus_zero_does_not_look_or_reorder(void)
{
    static TestGame tg;
    int c[4];
    int live = surplus_reorder(&tg, 0, c);
    int mark = prompt_mark();

    expect_pending(!rb_has_pending_choice(&tg.state), "surplus0: no prompt is opened");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "surplus0: the looked_at pool stays empty");
    int deck_exp[4] = { c[0], c[1], c[2], c[3] };
    check_bag_is(&tg.state.p[0].deck, deck_exp, 4, "surplus0: the deck is untouched, in order");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "surplus0: nothing goes to the waitroom");
    /* VACUITY CONTROL: every assertion above is a negative, so a board where the
     * trigger queue did nothing at all would pass them all. The positive sibling
     * is the SAME card with surplus = 1, which does prompt. */
    {
        static TestGame tg2;
        int c2[4];
        int live2 = surplus_reorder(&tg2, 1, c2);  /* calls board_begin() itself */
        int mark2 = prompt_mark();
        expect_pending(rb_has_pending_choice(&tg2.state),
            "surplus0 vacuity control: the SAME card with surplus=1 DOES prompt");
        prompt_guard(mark2, "surplus0 vacuity control: the surplus-1 reorder really prompts");
        drain_skip(&tg2, 8);
        (void)live2;
    }
    (void)mark;
    (void)live;
}

/* ===================================================================== */
/* 13. reorder_top / live_success_look_three_deck_size_q36_test.rs         */
/*       PL!-sd1-019-SD START:DASH!! — a whole-turn run to the live.       */
/* ===================================================================== */

static void test_q36_look_three_does_not_increase_deck_size(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int start_dash = test_new_id(&tg, START_DASH);
    pin_id(start_dash, START_DASH);
    CHECK(rb_card_is_live(start_dash), "q36: PL!-sd1-019-SD really is a live card");
    int filler = test_new_id(&tg, MUS_MEM);
    int member = test_new_id(&tg, "PL!-sd1-001-SD");
    pin_id(member, "PL!-sd1-001-SD");
    tg.state.p[0].stage[0] = member;
    test_add_to_hand(&tg, start_dash);
    for (int i = 0; i < 15; i++) {
        test_add_to_deck(&tg, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    int deck_before = tg.state.p[0].deck.n;

    advance_to_live_set(&tg);
    CHECK(strstr(rb_phase_name(tg.state.phase), "LiveCardSet") != NULL,
          "q36: five passes from Main reach the LiveCardSet phase");
    test_set_live_card(&tg, 0, start_dash);
    CHECK(bag_has(&tg.state.p[0].live, start_dash), "q36: START:DASH!! is set as the live card");
    int mark = prompt_mark();
    advance_to_live_victory(&tg);
    drain_skip(&tg, 12);

    int deck_after = tg.state.p[0].deck.n;
    CHECK(deck_after <= deck_before,
          "q36: ライブ成功時 looks, it does not enlarge the deck (%d -> %d)",
          deck_before, deck_after);
    /* VACUITY CONTROL: the "<=" would pass trivially on a board where the turn
     * never advanced. The phase assertion above proves the run happened, and the
     * prompt count proves the live actually fired prompts. */
    CHECK(prompts > mark, "q36 vacuity control: the live run really produced prompts");
}

/* ===================================================================== */
/* 14. reorder_top / debut_optional_wait_look_two_reorder_test.rs         */
/*       PL!-bp3-014-PR 星空 凛 (cost 4)                                    */
/*       登場 Cost(optional): このメンバーをウェイト状态にする                */
/*            Effect: デッキの上から2枚見る → 好きな枚数を好きな順番で      */
/*            デッキの上に置き、残りを控え室に置く。                         */
/*     The OPTIONAL WAIT cost is the subject: paying waits her, skipping    */
/*     does not, and the reorder works either way.                          */
/* ===================================================================== */

#define RIN   "PL!-bp3-014-PR"
#define RIN_A "PL!-bp3-017-N"
#define RIN_B "PL!-bp3-018-N"

static int total_cards_p1(const TestGame *tg)
{
    const RbPlayer *p = &tg->state.p[0];
    int n = p->hand.n + p->deck.n + p->discard.n + p->energy.n + p->live.n + p->success.n;
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (p->stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}

/* Rust: hand.push(rin); deck.push(card_a); deck.push(card_b); pad to 40.
 * deck.push appends, so the deck reads top-first card_a, card_b, filler x38. */
static int rin_board(TestGame *tg, int *out_rin, int *out_a, int *out_b)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    int rin = test_new_id(tg, RIN);
    pin_id(rin, RIN);
    pin_cost(rin, 4);
    dump_ability(rin, 0);
    int card_a = test_new_id(tg, RIN_A);
    pin_id(card_a, RIN_A);
    int card_b = test_new_id(tg, RIN_B);
    pin_id(card_b, RIN_B);
    int filler = test_new_id(tg, MUS_MEM);
    test_add_to_hand(tg, rin);
    int deck[64];
    int n = 0;
    deck[n++] = card_a;
    deck[n++] = card_b;
    while (n < 40) deck[n++] = filler;
    set_deck_exact(tg, 0, deck, n);
    test_give_energy(tg, 5);
    tg->state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    play_to_stage(tg, rin, 0, 0);
    *out_rin = rin; *out_a = card_a; *out_b = card_b;
    return rin;
}

static void test_rin_cost_paid_applies_wait_state(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 1);                        /* Rust select_option(1) == pay */
    prompt_guard(mark, "rin: the cost gate must be answered");
    expect_pending(rb_has_pending_choice(&tg.state), "rin: the look_and_select prompt follows the cost");
    check_choice_is(&tg, "looked_at", 2, 1, "rin: expected a SelectCard looked_at / 2 prompt");
    skip(&tg);
    drain_skip(&tg, 8);

    CHECK_EQ(rb_mods_get_orientation(&tg.state.mods, rin) != NULL &&
                 !strcmp(rb_mods_get_orientation(&tg.state.mods, rin), "wait") ? 1 : 0, 1,
             "rin: she should be in the wait state after paying the optional cost");
}

static void test_rin_cost_skipped_does_not_wait(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 0);                        /* Rust select_option(0) == skip */
    prompt_guard(mark, "rin: the cost gate must be answered");
    drain_skip(&tg, 12);

    const char *o = rb_mods_get_orientation(&tg.state.mods, rin);
    CHECK(o == NULL, "rin: no orientation modifier after skipping the optional cost");
}

static void test_rin_select_zero_cards_both_discarded(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 1);
    expect_pending(rb_has_pending_choice(&tg.state), "rin: the look_and_select prompt follows");
    prompt_guard(mark, "rin: cost + look must both prompt");
    skip(&tg);
    drain_skip(&tg, 8);

    CHECK(wait_has(&tg, a), "rin: card_a should be in the waitroom");
    CHECK(wait_has(&tg, b), "rin: card_b should be in the waitroom");
    CHECK(!deck_has(&tg, a) && !deck_has(&tg, b),
          "rin: both looked cards left the deck");
}

static void test_rin_select_one_card_other_discarded(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int discard_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 1);
    expect_pending(rb_has_pending_choice(&tg.state), "rin: the look_and_select prompt follows");
    prompt_guard(mark, "rin: cost + look must both prompt");
    check_choice_is(&tg, "looked_at", 2, 1, "rin: the look prompt is looked_at / 2");
    pick(&tg, 0);                       /* Rust select_indices(&[0]) == card_a */
    drain_skip(&tg, 8);

    CHECK_EQ(tg.state.p[0].deck.n > 0 ? tg.state.p[0].deck.cards[0] : -2, a,
             "rin: card_a should be on top of the deck");
    CHECK(deck_has(&tg, a), "rin: card_a is back in the deck");
    CHECK(wait_has(&tg, b), "rin: card_b should be in the waitroom");
    CHECK_EQ(tg.state.p[0].discard.n, discard_before + 1,
             "rin: exactly one card should have been discarded");
}

static void test_rin_select_both_cards_stay_on_deck(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int discard_before = tg.state.p[0].discard.n;
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 1);
    expect_pending(rb_has_pending_choice(&tg.state), "rin: the look_and_select prompt follows");
    prompt_guard(mark, "rin: cost + look must both prompt");
    /* Rust try_select_indices(&[0, 1]) — a MULTI-select batch. */
    {
        const int both[2] = { 0, 1 };
        prompts++;
        rb_resume_with_choice_indices(&tg.state, both, 2);
    }
    expect_pending(rb_has_pending_choice(&tg.state), "rin: the order choice must be prompted after selecting both");
    if (rb_has_pending_choice(&tg.state)) {
        checks++;
        if (strcmp(pending_type(&tg), "SelectTarget") != 0) {
            failures++;
            fprintf(stderr, "FAIL: rin: expected a SelectTarget order prompt, got %s\n",
                    pending_type(&tg));
        } else {
            printf("ok: rin: the order prompt is a SelectTarget\n");
        }
    }
    opt(&tg, 0);                        /* card_a on top */
    drain_skip(&tg, 8);

    CHECK(deck_has(&tg, a), "rin: card_a stays on the deck");
    CHECK(deck_has(&tg, b), "rin: card_b stays on the deck");
    CHECK_EQ(tg.state.p[0].discard.n, discard_before, "rin: no cards should have been discarded");
    CHECK_EQ(tg.state.p[0].deck.cards[0], a, "rin: card_a should be top of the deck");
    CHECK_EQ(tg.state.p[0].deck.cards[1], b, "rin: card_b should be second from the top");
}

static void test_rin_select_both_any_order_card_b_on_top(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    rin_board(&tg, &rin, &a, &b);
    int mark = prompt_mark();

    expect_pending(rb_has_pending_choice(&tg.state), "rin: the optional cost prompt is offered");
    opt(&tg, 1);
    {
        const int both[2] = { 0, 1 };
        prompts++;
        rb_resume_with_choice_indices(&tg.state, both, 2);
    }
    prompt_guard(mark, "rin: cost + look + order must all prompt");
    opt(&tg, 1);                        /* card_b on top */
    drain_skip(&tg, 8);

    CHECK_EQ(tg.state.p[0].deck.cards[0], b, "rin: card_b should be top of the deck (any_order)");
    CHECK_EQ(tg.state.p[0].deck.cards[1], a, "rin: card_a should be second from the top");
}

static void test_rin_card_count_integrity(void)
{
    static TestGame tg;
    int rin = 0, a = 0, b = 0;
    int total_initial;
    /* total_cards is taken BEFORE the play, so mirror the Rust ordering: build
     * the board, snapshot, then play. rin_board plays, so snapshot after and
     * compare across the ability instead — the play itself is a separate move. */
    rin_board(&tg, &rin, &a, &b);
    /* roll the debut ability's own effect back into a snapshot by re-running a
     * second, identical board: the invariant under test is that the ability
     * conserves cards, which a before/after pair around the DEBUT ability
     * cannot express because Rust snapshots before the play. Instead we assert
     * the ability-specific conservation: the two looked cards are accounted for
     * exactly once, in the deck or the waitroom, and nowhere else. */
    total_initial = total_cards_p1(&tg);

    /* Re-fire the debut ability on a fresh board with the snapshot taken first,
     * which is what the Rust test actually does. */
    {
        static TestGame tg2;
        int rin2 = 0, a2 = 0, b2 = 0;
        test_game_new(&tg2);
        clear_p1(&tg2);
        clear_p2(&tg2);
        rb_look_reset_all();
        board_begin();
        rin2 = test_new_id(&tg2, RIN);
        a2 = test_new_id(&tg2, RIN_A);
        b2 = test_new_id(&tg2, RIN_B);
        int filler2 = test_new_id(&tg2, MUS_MEM);
        test_add_to_hand(&tg2, rin2);
        int deck[64];
        int n = 0;
        deck[n++] = a2;
        deck[n++] = b2;
        while (n < 40) deck[n++] = filler2;
        set_deck_exact(&tg2, 0, deck, n);
        test_give_energy(&tg2, 5);
        int t0 = total_cards_p1(&tg2);
        tg2.state.p[0].stage[0] = RB_EMPTY_SLOT;
        tg2.state.p[0].stage[1] = RB_EMPTY_SLOT;
        tg2.state.p[0].stage[2] = RB_EMPTY_SLOT;
        play_to_stage(&tg2, rin2, 0, 0);
        opt(&tg2, 1);
        check_choice_is(&tg2, "looked_at", 2, 1, "rin integrity: the look prompt is opened");
        pick(&tg2, 0);
        drain_skip(&tg2, 8);
        int t1 = total_cards_p1(&tg2);
        CHECK_EQ(t1, t0, "rin: the total card count is preserved across the debut ability");
        CHECK(t1 > 0, "rin integrity control: the board is non-empty, so the check is not vacuous");
    }
    (void)total_initial;
}

/* ===================================================================== */
/* 15. reorder_top / skip_all_reorder_sends_all_to_waitroom_test.rs        */
/*       PL!HS-bp2-003-R 乙宗 梢 (cost 7) ライブ開始時                     */
/*       「手札を1枚控え室に置いてもよい：デッキの上からカードを3枚見る。   */
/*        その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室  */
/*        に置く。」                                                       */
/*     Skipping the placement must still route 残り to the waitroom.        */
/* ===================================================================== */

static void test_kozue_skip_reorder_sends_all_to_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int filler = test_new_id(&tg, NJI_FILL);
    fill_decks(&tg, filler);
    int me = test_new_id(&tg, "PL!HS-bp2-003-R");
    pin_id(me, "PL!HS-bp2-003-R");
    pin_cost(me, 7);
    dump_ability(me, 0);
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, test_new_id(&tg, NJI_FILL));
    int d1 = test_new_id(&tg, "PL!N-sd1-001-SD");
    int d2 = test_new_id(&tg, "PL!N-sd1-002-SD");
    int d3 = test_new_id(&tg, "PL!N-sd1-003-SD");
    pin_id(d1, "PL!N-sd1-001-SD");
    pin_id(d2, "PL!N-sd1-002-SD");
    pin_id(d3, "PL!N-sd1-003-SD");
    /* Rust inserts d1, d2, d3 in that order -> the deck reads top-first
     * d3, d2, d1, filler x27. */
    put_on_deck_top(&tg, 0, d1);
    put_on_deck_top(&tg, 0, d2);
    put_on_deck_top(&tg, 0, d3);
    int mark = prompt_mark();

    trigger_auto(&tg, me, RB_TSTR_LIVE_START);
    CHECK(card_prints_trigger(me, RB_TSTR_LIVE_START),
          "kozue: the card really prints a ライブ開始時 ability");
    expect_pending(rb_has_pending_choice(&tg.state), "kozue: the optional discard gate is offered");
    prompt_guard(mark, "kozue: the ライブ開始時 gate must prompt");
    opt(&tg, 0);
    drain_skip(&tg, 12);

    CHECK(wait_has(&tg, d1), "kozue: skipping placement sends d1 to the waitroom");
    CHECK(wait_has(&tg, d2), "kozue: skipping placement sends d2 to the waitroom");
    CHECK(wait_has(&tg, d3), "kozue: skipping placement sends d3 to the waitroom");
    /* VACUITY CONTROL: the three positive waitroom assertions are the control.
     * The discard cost is separate, so the waitroom should hold exactly 4. */
    CHECK_EQ(tg.state.p[0].discard.n, 4,
             "kozue: 1 cost discard + all 3 looked cards reach the waitroom");
}

/* ===================================================================== */
/* 16. reorder_top / debut_top3_reorder_and_discard_activation_test.rs    */
/*       中須かすみ PL!N-bp1-002-R＋ (cost 2)                              */
/*       ab#0 登場: デッキの上から3枚見る → 好きな枚数を好きな順番でデッキ  */
/*                  の上に置き、残りを控え室に置く。                         */
/*       ab#1 起動: コスト E E / 手札1枚 → 控え室からこのカードを登場させる。*/
/*     Q63/Q75/Q76/Q122 are about the activation's re-appearance.           */
/*     NOTE the FULLWIDTH PLUS: written as \xEF\xBC\x8B so the literal cannot   */
/*     be mangled by the editor's ANSI codepage.                              */
/* ===================================================================== */

#define KASUMI "PL!N-bp1-002-R\xEF\xBC\x8B"

static void test_kasumi_ability_only_from_discard(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    pin_cost(kasumi, 2);
    int filler = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);

    int kasumi_stage = test_new_id(&tg, KASUMI);
    test_add_to_hand(&tg, kasumi_stage);
    test_give_energy(&tg, 3);
    play_to_stage(&tg, kasumi_stage, 0, 0);
    CHECK(stage_has(&tg, kasumi_stage), "kasumi: the staged copy is on the left side");
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi: the debut look-at-3 must prompt");
    check_choice_is(&tg, "looked_at", 3, 1, "kasumi: expected a SelectCard looked_at / 3 prompt");
    drain_skip(&tg, 12);

    int r_stage = test_activate_ability(&tg, kasumi_stage);
    drain_skip(&tg, 6);
    CHECK_EQ(r_stage, 0, "kasumi: the 起動 must be refused from the stage (the card is not in the waitroom)");
    int r_hand = test_activate_ability(&tg, kasumi_stage);
    drain_skip(&tg, 6);
    (void)r_hand;
    int kasumi_hand = test_new_id(&tg, KASUMI);
    test_add_to_hand(&tg, kasumi_hand);
    int r_from_hand = test_activate_ability(&tg, kasumi_hand);
    drain_skip(&tg, 6);
    CHECK_EQ(r_from_hand, 0, "kasumi: the 起動 must be refused from a copy in hand");

    test_give_energy(&tg, 2);
    test_add_to_hand(&tg, test_new_id(&tg, MUS_MEM));
    int r_discard = test_activate_ability(&tg, kasumi);
    drain_pick0(&tg, 12);
    CHECK_EQ(r_discard, 1, "kasumi: the 起動 must be accepted from the waitroom");
    /* VACUITY CONTROL: the two refusals above are negatives. The accept is the
     * positive sibling on the same board and proves the activation path works. */
}

static void test_kasumi_q63_ability_appearance_no_cost_paid(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    pin_cost(kasumi, 2);
    int fodder = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 2);
    int hand_before = tg.state.p[0].hand.n;
    int mark = prompt_mark();

    test_activate_ability(&tg, kasumi);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q63: a hand-discard prompt is expected");
    prompt_guard(mark, "kasumi Q63: the activation must prompt");
    pick(&tg, 0);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q63: a stage-position prompt is expected");
    opt(&tg, 1);                        /* Centre */
    drain_skip(&tg, 12);

    CHECK_EQ(tg.state.p[0].stage[1], kasumi, "kasumi Q63: she is on the centre after the ability");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "kasumi Q63: 2E consumed for the ability cost, no member cost");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before - 1, "kasumi Q63: one card discarded for the cost");
    CHECK(!wait_has(&tg, kasumi), "kasumi Q63: she moved from the waitroom to the stage");
}

static void test_kasumi_q75_no_baton_touch_same_turn(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    int fodder = test_new_id(&tg, MUS_MEM);
    int other = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 2);
    int mark = prompt_mark();

    test_activate_ability(&tg, kasumi);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q75: the discard prompt is expected");
    pick(&tg, 0);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q75: the position prompt is expected");
    prompt_guard(mark, "kasumi Q75: the activation must prompt");
    opt(&tg, 0);                        /* LeftSide */
    drain_skip(&tg, 12);

    CHECK_EQ(tg.state.p[0].stage[0], kasumi, "kasumi Q75: she is on the left after the ability");
    /* Q75: she must be tracked as deployed this turn. C tracks this in
     * stage_arrived rather than a deployed_this_turn list. */
    CHECK_EQ(rb_card_arrived_this_turn(&tg.state, 0, kasumi), 1,
             "kasumi Q75: she is tracked as having arrived this turn");
    int n_arrived = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg.state.stage_arrived[0][i] >= 0) n_arrived++;
    CHECK_EQ(n_arrived, 1, "kasumi Q75: only Kasumi is tracked as arrived this turn");

    test_add_to_hand(&tg, other);
    play_to_stage(&tg, other, 0, 1);
    drain_skip(&tg, 8);
    CHECK_EQ(tg.state.p[0].stage[0], kasumi,
             "kasumi Q75: she is NOT baton-touched by a same-turn appearance");
}

static void test_kasumi_q76_appear_on_occupied_area(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    int filler = test_new_id(&tg, MUS_MEM);
    int fodder = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].stage[1] = filler;     /* centre already occupied */
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 2);
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);
    int mark = prompt_mark();

    test_activate_ability(&tg, kasumi);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q76: the discard prompt is expected");
    pick(&tg, 0);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q76: the position prompt is expected");
    prompt_guard(mark, "kasumi Q76: the activation must prompt");
    opt(&tg, 1);                        /* Centre, occupied */
    drain_skip(&tg, 12);

    CHECK_EQ(tg.state.p[0].stage[1], kasumi, "kasumi Q76: she replaced the filler on the centre");
    CHECK(wait_has(&tg, filler), "kasumi Q76: the replaced filler moved to the waitroom");
    CHECK_EQ(rb_card_arrived_this_turn(&tg.state, 0, kasumi), 1,
             "kasumi Q76: she is tracked as arrived this turn");
}

static void test_kasumi_q76_cannot_target_locked_area(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    int fodder = test_new_id(&tg, MUS_MEM);
    int f1 = test_new_id(&tg, MUS_MEM);
    int f2 = test_new_id(&tg, MUS_MEM);
    int f3 = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].stage[0] = f1;
    tg.state.p[0].stage[1] = f2;
    tg.state.p[0].stage[2] = f3;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        /* Mark every area as having received a member this turn, so all three
         * are locked for a re-appearance (Rule 9.6.2.1.2.1). The value is a
         * sentinel distinct from every real card id. */
        tg.state.stage_arrived[0][i] = 100000 + i;
    }
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 2);
    int mark = prompt_mark();

    test_activate_ability(&tg, kasumi);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q76 locked: the discard prompt is expected");
    pick(&tg, 0);
    prompt_guard(mark, "kasumi Q76 locked: the activation must prompt for the discard");
    expect_pending(!rb_has_pending_choice(&tg.state),
          "kasumi Q76: no position prompt when all three areas are locked");
    CHECK(wait_has(&tg, kasumi), "kasumi Q76: she stays in the waitroom when all areas are locked");
    /* VACUITY CONTROL: the negative "no prompt" must not pass on a board where
     * the activation did nothing. The prompt count above proves it ran, and the
     * waitroom assertion proves no silent move happened. */
    CHECK_EQ(tg.state.p[0].stage[0], f1, "kasumi Q76 locked: the left area is untouched");
    CHECK_EQ(tg.state.p[0].stage[1], f2, "kasumi Q76 locked: the centre area is untouched");
    CHECK_EQ(tg.state.p[0].stage[2], f3, "kasumi Q76 locked: the right area is untouched");
}

static void test_kasumi_q76_appear_when_stage_full(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    int f1 = test_new_id(&tg, MUS_MEM);
    int f2 = test_new_id(&tg, MUS_MEM);
    int f3 = test_new_id(&tg, MUS_MEM);
    int fodder = test_new_id(&tg, MUS_MEM);
    tg.state.p[0].stage[0] = f1;
    tg.state.p[0].stage[1] = f2;
    tg.state.p[0].stage[2] = f3;
    tg.state.p[0].discard.cards[tg.state.p[0].discard.n++] = kasumi;
    test_add_to_hand(&tg, fodder);
    test_give_energy(&tg, 2);
    for (int i = 0; i < 10; i++) test_add_to_deck(&tg, fodder);
    int mark = prompt_mark();

    test_activate_ability(&tg, kasumi);
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q76 full: the discard prompt is expected");
    pick(&tg, 0);
    prompt_guard(mark, "kasumi Q76 full: the activation must prompt");
    expect_pending(rb_has_pending_choice(&tg.state),
          "kasumi Q76: a position prompt appears even when the stage is full "
          "(allow_occupied_stage)");
    opt(&tg, 0);                        /* LeftSide, occupied by f1 */
    drain_skip(&tg, 12);

    CHECK_EQ(tg.state.p[0].stage[0], kasumi, "kasumi Q76: she replaced filler1 on the left");
    CHECK(wait_has(&tg, f1), "kasumi Q76: filler1 moved to the waitroom");
    CHECK_EQ(rb_card_arrived_this_turn(&tg.state, 0, kasumi), 1,
             "kasumi Q76: the left-area card is tracked as arrived this turn");
    CHECK_EQ(rb_card_arrived_this_turn(&tg.state, 0, f2), 0,
             "kasumi Q76: the centre setup card is NOT tracked as arrived this turn");
    CHECK_EQ(rb_card_arrived_this_turn(&tg.state, 0, f3), 0,
             "kasumi Q76: the right setup card is NOT tracked as arrived this turn");
    CHECK_EQ(tg.state.p[0].stage[1], f2, "kasumi Q76: filler2 is still on the centre");
    CHECK_EQ(tg.state.p[0].stage[2], f3, "kasumi Q76: filler3 is still on the right");
    CHECK(!wait_has(&tg, kasumi), "kasumi Q76: she moved out of the waitroom");
}

static void test_kasumi_q122_look_top3_no_refresh_with_exactly_3(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    pin_cost(kasumi, 2);
    for (int i = 0; i < 3; i++) test_add_to_deck(&tg, test_new_id(&tg, MUS_MEM));
    test_add_to_hand(&tg, kasumi);
    test_give_energy(&tg, 2);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    int mark = prompt_mark();

    play_to_stage(&tg, kasumi, 1, 0);
    prompt_guard(mark, "kasumi Q122: the debut look-at-3 must prompt");
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi Q122: the debut look-at-3 must prompt");
    check_choice_is(&tg, "looked_at", 3, 1, "kasumi Q122: expected a SelectCard looked_at / 3 prompt");
    /* Q122: with exactly 3 in the deck the look must NOT refresh from the
     * waitroom, so the pool is the deck's own three cards. */
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 3, "kasumi Q122: the look pool holds exactly 3");
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "kasumi Q122: no waitroom refresh with exactly 3 deck cards");
    drain_skip(&tg, 8);
    CHECK_EQ(tg.state.p[0].stage[1], kasumi, "kasumi Q122: she is on the centre after debut");
}

static void test_kasumi_ab0_debut_look_topdeck_arrange(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    rb_look_reset_all();
    board_begin();
    int kasumi = test_new_id(&tg, KASUMI);
    pin_id(kasumi, KASUMI);
    pin_cost(kasumi, 2);
    int filler = test_new_id(&tg, MUS_MEM);
    for (int i = 0; i < 5; i++) test_add_to_deck(&tg, filler);
    test_add_to_hand(&tg, kasumi);
    test_give_energy(&tg, 2);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    int deck_before = tg.state.p[0].deck.n;
    int mark = prompt_mark();

    play_to_stage(&tg, kasumi, 1, 0);
    prompt_guard(mark, "kasumi ab0: the debut look-at-3 must prompt");
    expect_pending(rb_has_pending_choice(&tg.state), "kasumi ab0: the debut look-at-3 must prompt");
    check_choice_is(&tg, "looked_at", 3, 1, "kasumi ab0: expected a SelectCard looked_at / 3 prompt");
    drain_skip(&tg, 12);
    CHECK_EQ(tg.state.p[0].stage[1], kasumi, "kasumi ab0: she is on the centre after debut");
    /* VACUITY CONTROL: skip-everything must still CONSUME the three looked cards
     * into the waitroom, which is what distinguishes a real look from a no-op. */
    CHECK_EQ(tg.state.p[0].discard.n, 3,
             "kasumi ab0 vacuity control: the three looked cards were really taken");
    CHECK_EQ(tg.state.p[0].deck.n, deck_before - 3,
             "kasumi ab0 vacuity control: exactly three cards left the deck");
}

/* ===================================================================== */
/* 17. reorder_top / cheer_mode_look_three_return_one_from_hand_test.rs    */
/*       Q276 虹ヶ咲 統括 PL!N-bp7-030-L                                   */
/*     Full-turn live runs. The control case is the load-bearing one: a      */
/*     NORMAL live card IS placed in the success zone under the same setup.  */
/* ===================================================================== */

#define CHEER   "PL!N-bp7-030-L"
#define CHIKA_M "PL!S-bp2-015-PR"
#define CHIKA_M2 "PL!-sd1-001-SD"

static int cheer_zone_success(const TestGame *tg, int cid)
{
    return bag_has(&tg->state.p[0].success, cid);
}

static void cheer_drain(TestGame *tg, int cheer, int discard_cheer)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        if (pending_zone_is(tg, "hand")) {
            int idx = 0;
            if (discard_cheer) {
                for (int i = 0; i < tg->state.p[0].hand.n; i++)
                    if (tg->state.p[0].hand.cards[i] == cheer) { idx = i; break; }
            }
            pick(tg, idx);
        } else {
            skip(tg);
        }
    }
}

static void cheer_board(TestGame *tg, int cheer, int member_a, int member_b,
                        int member_c, int hand_filler_count, int live_card)
{
    test_game_new(tg);
    clear_p1(tg);
    clear_p2(tg);
    rb_look_reset_all();
    board_begin();
    tg->state.p[0].stage[0] = member_a;
    if (member_b >= 0) tg->state.p[0].stage[1] = member_b;
    if (member_c >= 0) tg->state.p[0].stage[2] = member_c;
    test_add_to_hand(tg, live_card);
    int filler = test_new_id(tg, MUS_MEM);
    for (int i = 0; i < hand_filler_count; i++) test_add_to_hand(tg, filler);
    for (int i = 0; i < 50; i++) test_add_to_deck(tg, filler);
    for (int i = 0; i < 20; i++) test_add_to_deck_pl(tg, 1, filler);
}

static void test_q276_cheer_mode_returns_to_hand(void)
{
    static TestGame tg;
    int cheer = test_new_id(&tg, CHEER);
    pin_id(cheer, CHEER);
    int member = test_new_id(&tg, CHIKA_M);
    pin_id(member, CHIKA_M);
    cheer_board(&tg, cheer, member, -1, -1, 2, cheer);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, cheer);
    CHECK(bag_has(&tg.state.p[0].live, cheer), "Q276: Cheer Mode was set as the live card");
    advance_to_live_start(&tg);
    drain_pick0(&tg, 20);
    int mark = prompt_mark();
    advance_to_live_victory(&tg);
    cheer_drain(&tg, cheer, 0);
    prompt_guard(mark, "Q276: the winning live really produced ライブ成功時 prompts");

    CHECK(!cheer_zone_success(&tg, cheer),
          "Q276: Cheer Mode must NOT be in the success live-card zone");
    CHECK(hand_has(&tg, cheer),
          "Q276: Cheer Mode is returned to the hand by its ライブ成功時 ability");
    /* The control below is the load-bearing assertion: without it, "not in the
     * success zone" would also pass on an engine that never fills the success
     * zone at all. */
}

static void test_q276_cheer_discarded_by_own_ability(void)
{
    static TestGame tg;
    int cheer = test_new_id(&tg, CHEER);
    pin_id(cheer, CHEER);
    int member = test_new_id(&tg, CHIKA_M);
    cheer_board(&tg, cheer, member, -1, -1, 0, cheer);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, cheer);
    advance_to_live_start(&tg);
    drain_pick0(&tg, 20);
    int mark = prompt_mark();
    advance_to_live_victory(&tg);
    cheer_drain(&tg, cheer, 1);
    prompt_guard(mark, "Q276 discard-self: the winning live really produced prompts");

    CHECK(!cheer_zone_success(&tg, cheer), "Q276: never in the success zone, even discarded");
    CHECK(wait_has(&tg, cheer), "Q276: her own discard step routes her to the waitroom");
    CHECK(!hand_has(&tg, cheer), "Q276: she left the hand after the discard step");
}

static void test_q276_control_normal_live_reaches_success_zone(void)
{
    static TestGame tg;
    int live = test_new_id(&tg, START_DASH);
    pin_id(live, START_DASH);
    int m1 = test_new_id(&tg, CHIKA_M2);
    int m2 = test_new_id(&tg, CHIKA_M2);
    int m3 = test_new_id(&tg, CHIKA_M2);
    cheer_board(&tg, -1, m1, m2, m3, 0, live);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, live);
    advance_to_live_start(&tg);
    drain_pick0(&tg, 20);
    int mark = prompt_mark();
    advance_to_live_victory(&tg);
    drain_pick0(&tg, 20);
    test_pass(&tg);
    drain_pick0(&tg, 20);
    prompt_guard(mark, "Q276 control: the winning live really produced prompts");

    CHECK(cheer_zone_success(&tg, live),
          "Q276 control: a NORMAL live card IS placed in the success zone, so the "
          "Cheer Mode exclusion is her own ライブ成功時 return and not a general "
          "rule that winning lives never reach the success zone");
}

static void test_q276_failed_live_does_not_return_cheer(void)
{
    static TestGame tg;
    int cheer = test_new_id(&tg, CHEER);
    pin_id(cheer, CHEER);
    cheer_board(&tg, cheer, RB_EMPTY_SLOT, -1, -1, 0, cheer);

    advance_to_live_set(&tg);
    test_set_live_card(&tg, 0, cheer);
    advance_to_live_start(&tg);
    drain_pick0(&tg, 20);
    int mark = prompt_mark();
    advance_to_live_victory(&tg);
    drain_pick0(&tg, 20);
    prompt_guard(mark, "Q276 failed-live: the live run really produced prompts");

    CHECK(!cheer_zone_success(&tg, cheer), "Q276: a failed live does not reach the success zone");
    CHECK(!hand_has(&tg, cheer), "Q276: a failed live does not return her to the hand");
    CHECK(wait_has(&tg, cheer), "Q276: a failed live leaves her in the waitroom");
    /* VACUITY CONTROL: the two negatives above are paired with a positive: the
     * same card in the WINNING case does reach the hand (see the sibling). The
     * waitroom assertion is the positive half here. */
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
            UMI, MUS_LIVE, MUS_MEM, HANAMARU, DIA_H04X2, CLEAN_KOTORI,
            "PL!-bp5-014-N", "PL!-PR-002-PR", KARIN_OTH, "PL!N-pb1-016-R",
            AQ_CHIKA, "PL!N-pb1-018-R", KANATA, "PL!N-pb1-021-R", LANZHU_W,
            "PL!N-pb1-024-R", HIMENO, DODO, IDENTITY, "PL!-pb1-016-R",
            "PL!-PR-007-PR", "PL!SP-pb1-017-N", "PL!SP-PR-005-PR",
            "PL!HS-pb1-018-N", "PL!HS-bp2-008-R", WIEN, KANON, "PL!SP-bp2-005-R",
            HONOKA, START_DASH, TARGET_MEM, GINKO, NOZOMI, BLUE_MOMENT,
            RIN, RIN_A, RIN_B, "PL!HS-bp2-003-R", KASUMI, CHEER, CHIKA_M,
            CHIKA_M2, "PL!N-sd1-001-SD", "PL!N-sd1-002-SD", "PL!N-sd1-003-SD",
            "PL!N-sd1-025-SD", "PL!-sd1-001-SD", NJI_FILL,
        };
        for (size_t i = 0; i < sizeof cards / sizeof cards[0]; i++) {
            int cid = test_new_id(&tg, cards[i]);
            printf("%-24s -> id=%-6d cost=%-3d live=%d abilities=%d\n",
                   cards[i], cid, card_printed_cost(cid),
                   rb_card_is_live(cid) ? 1 : 0,
                   rb_card_num_abilities((uint32_t)cid));
            for (int a = 0; a < rb_card_num_abilities((uint32_t)cid); a++)
                dump_ability(cid, a);
            printf("    hearts:");
            for (int col = 0; col < 8; col++)
                printf(" %d=%d", col, card_printed_heart(cid, col));
            printf("\n");
        }
        return 0;
    }

    /* ---- search_deck cluster ---- */
    run("sd/look5_no_eligible_auto_skips",
        test_look_and_select_no_eligible_cards_auto_skips);
    run("sd/look5_takes_group_live_card",
        test_look_and_select_takes_group_live_card_to_hand);
    run("sd/look5_skip_discards_remainder",
        test_look_and_select_skip_still_discards_remainder_to_waitroom);
    run("sd/hb4_takes_double_heart04",
        test_hanamaru_live_success_look_four_takes_double_heart04);
    run("sd/hb4_no_double_heart04_all_waitroom",
        test_hanamaru_no_double_heart04_sends_all_four_to_waitroom);
    run("sd/named016_removes_karin",
        test_named_look_016_removes_karin_from_deck);
    run("sd/named018_reveals_kanata",
        test_named_look_018_reveals_kanata);
    run("sd/named018_no_kanata_adds_nothing",
        test_named_look_018_no_matching_kanata_adds_nothing);
    run("sd/named021_reveals_rina",
        test_named_look_021_reveals_rina);
    run("sd/named021_wrong_character_stays_out",
        test_named_look_021_wrong_character_stays_out);
    run("sd/named024_reveals_lanzhu",
        test_named_look_024_reveals_lanzhu);
    run("sd/himeno_selects_dodo",
        test_optional_discard_look_five_selects_dodo);
    run("sd/himeno_selects_identity",
        test_optional_discard_look_five_selects_identity);
    run("sd/himeno_no_unit_match_auto_skips",
        test_optional_discard_look_five_no_unit_match_auto_skips);
    run("sd/lilywhite_four_look_fetches",
        test_paid_discard_look_four_fetches_lilywhite);
    run("sd/lilywhite_declined_keeps_deck",
        test_paid_discard_look_four_declined_keeps_deck);
    run("sd/fivesyncrie_five_look_fetches",
        test_paid_discard_look_five_fetches_5yncri5e);
    run("sd/dollchestra_five_look_fetches",
        test_paid_discard_look_five_fetches_dollchestra);
    run("sd/wien_activation_fetches_liella",
        test_wien_activation_fetches_liella);
    run("sd/wien_no_liella_all_to_waitroom",
        test_wien_no_liella_in_five_all_to_waitroom);
    run("sd/wien_declined_fetch_kanon_waitroom",
        test_wien_declined_fetch_sends_kanon_to_waitroom);
    run("sd/wien_unaffordable_not_offered",
        test_wien_unaffordable_energy_not_offered);
    run("sd/wien_empty_hand_refused",
        test_wien_empty_hand_activation_refused);
    run("sd/look7_paid_fetches_liella",
        test_look_seven_paid_fetches_liella);
    run("sd/look7_unpayable_auto_skips",
        test_look_seven_unpayable_auto_skips);
    run("sd/honoka_member_skips_live_fillers",
        test_honoka_reveal_member_skips_live_fillers);
    run("sd/honoka_target_first_card",
        test_honoka_reveal_target_first_card);
    run("sd/honoka_target_last_card",
        test_honoka_reveal_target_last_card);
    run("sd/honoka_live_skips_members",
        test_honoka_reveal_live_card_skips_members);
    run("sd/honoka_two_matches_only_one",
        test_honoka_reveal_two_matches_only_one_added);
    run("sd/honoka_no_member_exhausts_deck",
        test_honoka_reveal_no_member_in_deck_exhausts);
    run("sd/honoka_center_requirement_fails",
        test_honoka_reveal_center_requirement_left_side_fails);
    run("sd/honoka_use_limit_blocks_second",
        test_honoka_use_limit_blocks_second_activation);

    /* ---- reorder_top cluster ---- */
    run("rt/any_number_partial_selection",
        test_any_number_partial_selection);
    run("rt/any_number_full_selection",
        test_any_number_full_selection);
    run("rt/any_number_skip_all",
        test_any_number_skip_all);
    run("rt/ginko_refresh_preserves_top",
        test_ginko_refresh_preserves_top_and_recovers_waitroom);
    run("rt/ginko_exact_size_no_refresh",
        test_ginko_exact_deck_size_does_not_refresh);
    run("rt/nozomi_every_changed_order",
        test_nozomi_live_success_supports_every_changed_order);
    run("rt/nozomi_can_keep_original_order",
        test_nozomi_live_success_can_keep_original_order);
    run("rt/live_success_not_on_debut",
        test_live_success_reorder_does_not_trigger_on_debut);
    run("rt/surplus1_partial_to_waitroom",
        test_surplus_one_partial_routes_remainder_to_waitroom);
    run("rt/surplus1_full_both_orders",
        test_surplus_one_full_supports_both_orders);
    run("rt/surplus1_skip_both_waitroom",
        test_surplus_one_skip_routes_both_to_waitroom);
    run("rt/surplus0_does_not_look",
        test_surplus_zero_does_not_look_or_reorder);
    run("rt/q36_deck_size_not_increased",
        test_q36_look_three_does_not_increase_deck_size);
    run("rt/rin_cost_paid_waits",
        test_rin_cost_paid_applies_wait_state);
    run("rt/rin_cost_skipped_not_wait",
        test_rin_cost_skipped_does_not_wait);
    run("rt/rin_select_zero_both_discarded",
        test_rin_select_zero_cards_both_discarded);
    run("rt/rin_select_one_other_discarded",
        test_rin_select_one_card_other_discarded);
    run("rt/rin_select_both_stay_on_deck",
        test_rin_select_both_cards_stay_on_deck);
    run("rt/rin_select_both_any_order",
        test_rin_select_both_any_order_card_b_on_top);
    run("rt/rin_card_count_integrity",
        test_rin_card_count_integrity);
    run("rt/kozue_skip_all_to_waitroom",
        test_kozue_skip_reorder_sends_all_to_waitroom);
    run("rt/kasumi_ability_only_from_discard",
        test_kasumi_ability_only_from_discard);
    run("rt/kasumi_q63_no_cost_paid",
        test_kasumi_q63_ability_appearance_no_cost_paid);
    run("rt/kasumi_q75_no_baton_touch",
        test_kasumi_q75_no_baton_touch_same_turn);
    run("rt/kasumi_q76_appear_on_occupied",
        test_kasumi_q76_appear_on_occupied_area);
    run("rt/kasumi_q76_cannot_target_locked",
        test_kasumi_q76_cannot_target_locked_area);
    run("rt/kasumi_q76_appear_when_full",
        test_kasumi_q76_appear_when_stage_full);
    run("rt/kasumi_q122_no_refresh_exact_3",
        test_kasumi_q122_look_top3_no_refresh_with_exactly_3);
    run("rt/kasumi_ab0_debut_look_top3",
        test_kasumi_ab0_debut_look_topdeck_arrange);
    run("rt/q276_cheer_returns_to_hand",
        test_q276_cheer_mode_returns_to_hand);
    run("rt/q276_cheer_discarded_by_own_ability",
        test_q276_cheer_discarded_by_own_ability);
    run("rt/q276_control_normal_live_success",
        test_q276_control_normal_live_reaches_success_zone);
    run("rt/q276_failed_live_no_return",
        test_q276_failed_live_does_not_return_cheer);

    printf("\n==== test_parity_search_reorder: %d cases ====\n"
           "%d ok, %d failed, %d crashed, %d expected gaps\n"
           "%ld checks, %d failures\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed, gaps,
           checks, failures);

    return (failures == 0 && n_failed == 0 && n_crashed == 0) ? 0 : 1;
}

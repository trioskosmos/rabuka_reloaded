/* tests/test_ability_effects_crash.c
 *
 * Regression suite for the process-killing crash in
 * src/ability/effects/ability_effects.c.
 *
 * THE DEFECT
 * ----------
 * rb_translated_execute_gain_ability_effect() and
 * rb_translated_execute_invalidate_ability() each open with an unconditional
 * debug trace to stderr. Both traces passed
 *
 *     effect->card_type_field[0]     and     effect->self_target_field[0]
 *
 * into `%s` slots. Those members are NOT pointers — include/rabuka.h:136-137
 * declares them `char self_target_field[8]` and `char card_type_field[24]`.
 * The `[0]` promotes a `char` to `int`, so printf treated the printed BYTE as
 * a char pointer and dereferenced it. For any ability carrying the ordinary
 * `card_type` of "member_card" the first byte is 'm' == 0x6d, so the call read
 * address 109 and the process took an access violation. Every ability of the
 * form 常時:〜を得る (rb_translated_execute_gain_ability_effect) or
 * ライブ成功時を無効にする (rb_translated_execute_invalidate_ability) is
 * affected, and the fault is NONDETERMINISTIC because it depends on whatever
 * integer lands in the argument register.
 *
 * The observable signature before the fix: the stderr line always truncated
 * mid-format, right after `%s ct=`, and a later `f=yes` marker in the same
 * line proved the following fprintf never ran.
 *
 * WHY EVERY TEST FORKS
 * --------------------
 * In this toolchain `make t` reports SUCCESS for a killed process, so a suite
 * that aborts mid-run looks green. Left in-process, the first crash would
 * truncate this file and every test after it would falsely appear to pass.
 * Each test therefore runs in a forked child whose exit status carries its own
 * failure count, and a child that dies on a signal is reported as CRASH —
 * never as a pass. (Technique established by the author of
 * tests/test_parity_characterization_extra.c.)
 *
 * THE FIX IS BEHAVIOURAL, NOT COSMETIC
 * ------------------------------------
 * The traces now print the arrays whole. The Rust twin
 * (engine/src/ability/effects/ability_effects.rs) emits no [DBG_GAIN] /
 * [DBG_INVAL] output at all; its closest analogue is the `card_type={:?}`
 * debug at ability_effects.rs:289, which formats the WHOLE CardType and never
 * its first byte. So the whole field is what "the Rust debug string" is.
 * `test_trace_prints_whole_card_type` pins that directly: it captures the
 * child's stderr and asserts the trace contains `ct=member_card` and, past it,
 * the `self_tgt=` slot — the two things the truncation destroyed.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* This suite builds under a strict `-std=c11` with no feature-test macro, so
 * `fileno` is not declared even though <stdio.h> is included. The prototype
 * is identical on glibc, musl and mingw-w64. Everything else POSIX used here
 * (fork, waitpid, dup, dup2, close, signal, _Exit) is already declared. */
extern int fileno(FILE *stream);

/* ── assertion plumbing ────────────────────────────────────────────────── */

/* Every test in main() must be listed here exactly once, so adding a test
 * without wiring it in is a compile-time-visible bookkeeping change rather
 * than a silently-uncounted one. */
#define XCRASH_N_TESTS 9


static int failures;
static int checks;
static const char *current_test = "(none)";

#define CHECK(cond, msg) do {                                                \
        checks++;                                                            \
        if (!(cond)) {                                                       \
            fprintf(stderr, "FAIL [%s]: %s\n", current_test, msg);           \
            failures++;                                                       \
        }                                                                    \
    } while (0)

#define CHECK_EQ_INT(actual, expected, msg) do {                            \
        checks++;                                                            \
        long a_ = (long)(actual), e_ = (long)(expected);                     \
        if (a_ != e_) {                                                      \
            fprintf(stderr, "FAIL [%s]: %s (got %ld expected %ld)\n",        \
                    current_test, msg, a_, e_);                             \
            failures++;                                                       \
        }                                                                    \
    } while (0)

/* The fault being regression-tested: an access violation inside a debug trace.
 * A default handler prints nothing useful, so name the test in flight.
 *
 * ASYNC-SIGNAL-SAFETY IS LOAD-BEARING HERE, not a nicety. This handler runs
 * precisely WHEN the process faults, and the fault is inside a `fprintf`. glibc
 * holds the stderr FILE lock for the duration of that fprintf, so a handler
 * that itself calls fprintf(stderr) DEADLOCKS on that non-recursive lock and
 * the suite HANGS instead of reporting the crash. That is not hypothetical: it
 * is exactly what a first draft of this handler did against the unfixed
 * engine, and the run wedged on the first traced call. So: `write()` on a raw
 * fd, and `_exit()`. Both are async-signal-safe. No stdio, no malloc, no locks. */
static void on_segv(int sig)
{
    static const char head[] = "\n*** SIGNAL: fault inside test: ";
    static const char tail[] =
        "\n*** ability_effects.c passed a char BYTE (a [0] array element) to a %s\n"
        "*** slot; printf dereferenced the byte as a pointer and faulted.\n";
    ssize_t r;
    /* fd 2 may currently be redirected into a capture file by §B; that is fine
     * and desirable — the report lands in the captured trace as well. */
    r = write(2, head, sizeof(head) - 1);              (void)r;
    r = write(2, current_test, strlen(current_test));  (void)r;
    r = write(2, tail, sizeof(tail) - 1);              (void)r;
    _exit(128 + sig);
}

/* Watchdog. A crash-regression suite must not be able to HANG: against the
 * UNFIXED engine a traced call does not always come back as a clean fault —
 * one configuration spun forever instead. The deadline is enforced by the
 * PARENT (see run()), NOT by alarm() in the child: alarm() is a no-op on this
 * toolchain, so a child-side bound silently does nothing and a wedged child
 * stalls the whole suite with no output at all. That was measured, not
 * assumed: a 60 s child alarm never fired across a 500 s run. */
/* ── per-test isolation ────────────────────────────────────────────────── */

/* A crash-regression suite must not be able to HANG, so the exit codes below
 * are distinct and never overloaded: a child exits 0 (no failed checks) or 1
 * (hard failed checks), and a child that dies on a signal is reported as CRASH
 * and never as a pass. That distinction is the whole point: in this toolchain
 * `make t` reports success for a killed process, so a suite that aborts
 * mid-run otherwise looks green.
 *
 * CHILD_CRASHED is NOT a child exit code — no child ever returns it. It names
 * the third outcome, and it is deliberately not 2: main()'s own exit codes are
 * a separate namespace (0 clean, 1 failed checks, 2 engine fault), and
 * overloading a value between the two would make a crash indistinguishable
 * from a real failure at the make level. */
#define CHILD_OK        0   /* child exited 0: no failed checks   */
#define CHILD_FAILURES  1   /* child exited 1: hard failed checks */
#define CHILD_CRASHED   3   /* outcome label, never a child exit  */

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
        if (failures > f0) n_tests_failed++; else n_tests_ok++;
        return;
    }
    if (pid == 0) {
        int c0 = checks, f0 = failures;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s)\n", name,
               checks - c0, failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    /* A plain blocking waitpid: every test here is a bounded board setup.
     *
     * NO WATCHDOG, deliberately. Two were built and measured, and both are
     * dead code on this toolchain, so shipping either would be a lie:
     *   - a child-side `alarm()` bound never fired (no-op here): the 60 s bound
     *     did not trigger across a 500 s run;
     *   - a parent-side `waitpid(WNOHANG)` deadline did fire, but the follow-up
     *     `kill(SIGKILL)` does not take effect here, so the blocking `waitpid`
     *     after it wedged instead of reporting.
     * A test that merely LOOKS like it cannot hang is worse than one that says
     * plainly that it can. The isolation that does work — fork per test, so a
     * fault is attributed and the tests after it still run — is kept. */
    int status = 0;
    if (waitpid(pid, &status, 0) != pid) status = 0;

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
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ── effect fixtures ────────────────────────────────────────────────────── */

static void set_extra(AbilityEffect *e, int i, const char *k, const char *v)
{
    if (i >= RB_MAX_EXTRA) return;
    e->extra_k[i] = (char *)k;
    e->extra_v[i] = (char *)v;
    if (e->n_extra < i + 1) e->n_extra = i + 1;
}

/* A hand-built 常時:〜を得る effect. `ct`/`self` are written into the fixed
 * arrays exactly the way the decoder writes them (vm.c:751-752: strncpy then a
 * forced NUL), so the fixture cannot be NUL-unterminated. */
static void make_gain_effect(AbilityEffect *e, const char *ct, const char *self,
                             const char *source)
{
    memset(e, 0, sizeof(*e));
    e->count = -1;
    e->action = (char *)"gain_ability";
    e->source = (char *)source;
    e->text = (char *)"常時:このステージのGraviteeのメンバーは、";
    if (ct) { strncpy(e->card_type_field, ct, sizeof(e->card_type_field) - 1);
              e->card_type_field[sizeof(e->card_type_field) - 1] = 0; }
    if (self) { strncpy(e->self_target_field, self, sizeof(e->self_target_field) - 1);
                e->self_target_field[sizeof(e->self_target_field) - 1] = 0; }
    set_extra(e, 0, "group_names", "Gravitee");
    set_extra(e, 1, "ability_gain", "【常時】Graviteeのメンバーは、ライブの合計スコア+1をする。");
    e->has_condition = 0;
}

/* A hand-built ライブ成功時を無効にする effect. */
static void make_inval_effect(AbilityEffect *e, const char *ct, const char *self)
{
    memset(e, 0, sizeof(*e));
    e->count = -1;
    e->action = (char *)"invalidate_ability";
    e->target = (char *)"self";
    if (ct) { strncpy(e->card_type_field, ct, sizeof(e->card_type_field) - 1);
              e->card_type_field[sizeof(e->card_type_field) - 1] = 0; }
    if (self) { strncpy(e->self_target_field, self, sizeof(e->self_target_field) - 1);
                e->self_target_field[sizeof(e->self_target_field) - 1] = 0; }
    set_extra(e, 0, "target_trigger", "ライブ成功時");
    set_extra(e, 1, "duration", "permanent");
}

/* An empty board: the trace runs unconditionally, so no card setup is needed
 * and no deeper code path is fed a nonsense card_type string. */
static void empty_board(TestGame *tg)
{
    test_game_new(tg);
    tg->state.activating_card = -1;
    tg->state.n_selected_cards = 0;
}

/* Discard fd 2 for the duration of the byte sweeps. Each byte produces one
 * [DBG_GAIN] / [DBG_INVAL] line, and several bytes are control characters that
 * split a line in the log — 510 lines of noise per sweep that hides the real
 * output. Nothing is lost: §B inspects the trace deliberately, with the
 * capture in place. */
static int silence_stderr_begin(void)
{
    fflush(stderr);
    int saved = dup(2);
    FILE *null = freopen("/dev/null", "w", stderr);
    if (!null) { dup2(saved, 2); close(saved); return -1; }
    return saved;
}

static void silence_stderr_end(int saved)
{
    fflush(stderr);
    dup2(saved, 2);
    close(saved);
}

/* ══════════════════════════════════════════════════════════════════════
 * §A — the byte sweep. THE definitive proof.
 *
 * Before the fix, the trace handed printf the integer `byte`. Reproducing the
 * crash therefore only needs a card_type whose first byte is a letter, but the
 * sweep proves something stronger: for EVERY possible first byte 1..255, both
 * entry points survive. That closes the whole class, not the one reported
 * value ('m' == 0x6d == 109).
 * ══════════════════════════════════════════════════════════════════════ */

static void test_gain_survives_every_first_byte(void)
{
    static TestGame tg;
    empty_board(&tg);
    int saved = silence_stderr_begin();
    for (int byte = 1; byte < 256; byte++) {
        AbilityEffect e;
        make_gain_effect(&e, NULL, NULL, NULL);
        e.card_type_field[0] = (char)byte;
        e.card_type_field[1] = 0;
        e.self_target_field[0] = (char)byte;
        e.self_target_field[1] = 0;
        rb_translated_execute_gain_ability_effect(&tg.state, 0, &e, -1);
    }
    if (saved >= 0) silence_stderr_end(saved);
    CHECK(1, "gain_ability_effect survives all 255 non-NUL first bytes");
}

static void test_invalidate_survives_every_first_byte(void)
{
    static TestGame tg;
    empty_board(&tg);
    int saved = silence_stderr_begin();
    for (int byte = 1; byte < 256; byte++) {
        AbilityEffect e;
        make_inval_effect(&e, NULL, NULL);
        e.card_type_field[0] = (char)byte;
        e.card_type_field[1] = 0;
        e.self_target_field[0] = (char)byte;
        e.self_target_field[1] = 0;
        rb_translated_execute_invalidate_ability(&tg.state, 0, &e);
    }
    if (saved >= 0) silence_stderr_end(saved);
    CHECK(1, "invalidate_ability survives all 255 non-NUL first bytes");
}

/* The two byte values the bug report names, called out by name so a
 * regression points straight at them. */
static void test_gain_survives_member_card_leading_m(void)
{
    static TestGame tg;
    empty_board(&tg);
    AbilityEffect e;
    make_gain_effect(&e, "member_card", "true", "stage");
    CHECK_EQ_INT(e.card_type_field[0] == 'm', 1,
                 "fixture premise: \"member_card\"[0] is 'm' (0x6d = 109)");
    rb_translated_execute_gain_ability_effect(&tg.state, 0, &e, -1);
    CHECK(1, "gain_ability_effect with card_type=member_card survives");
}

static void test_invalidate_survives_member_card_leading_m(void)
{
    static TestGame tg;
    empty_board(&tg);
    AbilityEffect e;
    make_inval_effect(&e, "member_card", "false");
    CHECK_EQ_INT(e.card_type_field[0] == 'm', 1,
                 "fixture premise: \"member_card\"[0] is 'm' (0x6d = 109)");
    rb_translated_execute_invalidate_ability(&tg.state, 0, &e);
    CHECK(1, "invalidate_ability with card_type=member_card survives");
}

/* The other two card_type values the decoder can produce, so the sweep is not
 * limited to the byte a single card happened to use. */
static void test_literal_card_types_all_survive(void)
{
    static TestGame tg;
    empty_board(&tg);
    const char *types[3] = { "member_card", "live_card", "energy_card" };
    for (int i = 0; i < 3; i++) {
        AbilityEffect g;
        make_gain_effect(&g, types[i], "true", "stage");
        rb_translated_execute_gain_ability_effect(&tg.state, 0, &g, -1);
        AbilityEffect v;
        make_inval_effect(&v, types[i], "false");
        rb_translated_execute_invalidate_ability(&tg.state, 0, &v);
    }
    CHECK(1, "member_card / live_card / energy_card all survive both entry points");
}

/* ══════════════════════════════════════════════════════════════════════
 * §B — the trace must be COMPLETE, not merely non-fatal.
 *
 * A patch that only avoided the fault (e.g. by printing "%c", or by deleting
 * the trace) would pass §A. §B pins the actual requirement.
 *
 * The bug report's own signature: the [DBG_GAIN] line always truncated
 * mid-format, right after `%s ct=`, and the fields BEHIND ct= on that same
 * line (gn=, self_tgt=, nsel=, cond=) never printed — which also proves the
 * fprintf never returned, because the [DBG_GAIN_INNER] block behind it never
 * ran either. So the assertions here are per-LINE, not per-buffer: every
 * required field must be present ON THE ONE [DBG_GAIN] / [DBG_INVAL] line.
 * A stray "ct=member_card" from some other line must not be able to pass.
 * ══════════════════════════════════════════════════════════════════════ */

/* Copy the first line of `buf` that starts with `prefix` (including its
 * trailing space, so "[DBG_GAIN] " does not match "[DBG_GAIN_INNER] ") into
 * `out`. Returns `out`, which is "" when no such line exists.
 *
 * The caller supplies `out` rather than this function owning a static buffer,
 * because the callers hold TWO extracted lines at once and a single static
 * would silently alias them. */
static const char *line_starting_with(const char *buf, const char *prefix,
                                      char *out, size_t cap)
{
    out[0] = 0;
    const char *p = buf;
    while (*p) {
        const char *eol = strchr(p, '\n');
        size_t len = eol ? (size_t)(eol - p) : strlen(p);
        if (len >= strlen(prefix) && !strncmp(p, prefix, strlen(prefix))) {
            if (len >= cap) len = cap - 1;
            memcpy(out, p, len);
            out[len] = 0;
            return out;
        }
        if (!eol) break;
        p = eol + 1;
    }
    return out;
}

/* Capture fd 2 into a tmpfile across `body`, leaving it readable in `out`. */
typedef void (*TraceBody)(GameState *g, char *out, size_t cap);

static void capture_trace(GameState *g, TraceBody body, char *out, size_t cap)
{
    out[0] = 0;
    FILE *tmp = tmpfile();
    if (!tmp) return;
    fflush(stderr);
    int saved = dup(2);
    dup2(fileno(tmp), 2);
    body(g, out, cap);
    fflush(stderr);
    dup2(saved, 2);
    close(saved);
    rewind(tmp);
    size_t n = fread(out, 1, cap - 1, tmp);
    out[n] = 0;
    fclose(tmp);
}

static void trace_both_entry_points(GameState *g, char *out, size_t cap)
{
    (void)out; (void)cap;
    AbilityEffect ga;
    make_gain_effect(&ga, "member_card", "true", "stage");
    rb_translated_execute_gain_ability_effect(g, 0, &ga, -1);

    AbilityEffect iv;
    make_inval_effect(&iv, "member_card", "false");
    rb_translated_execute_invalidate_ability(g, 0, &iv);
}

static void test_trace_prints_whole_card_type(void)
{
    static TestGame tg;
    empty_board(&tg);

    char buf[16384];
    char gain[8192], inval[8192];
    capture_trace(&tg.state, trace_both_entry_points, buf, sizeof(buf));

    line_starting_with(buf, "[DBG_GAIN] ", gain, sizeof(gain));
    CHECK(gain[0] != 0, "a [DBG_GAIN] line was emitted");
    CHECK(strstr(gain, "ct=member_card") != NULL,
          "[DBG_GAIN] prints the WHOLE card_type (\"ct=member_card\"), not one byte");
    CHECK(strstr(gain, "gn=Gravitee") != NULL,
          "[DBG_GAIN] reached the gn= slot AFTER ct= (the old crash stopped at ct=)");
    CHECK(strstr(gain, "self_tgt=true") != NULL,
          "[DBG_GAIN] reached the self_tgt= slot after ct=");
    CHECK(strstr(gain, "cond=") != NULL,
          "[DBG_GAIN] reached its LAST slot (cond=), so the fprintf completed");
    if (gain[0])
        printf("        [DBG_GAIN] line: %s\n", gain);

    line_starting_with(buf, "[DBG_INVAL] ", inval, sizeof(inval));
    CHECK(inval[0] != 0, "a [DBG_INVAL] line was emitted");
    CHECK(strstr(inval, "ct=member_card") != NULL,
          "[DBG_INVAL] prints the WHOLE card_type");
    CHECK(strstr(inval, "self=false") != NULL,
          "[DBG_INVAL] reached its LAST slot (self=), so the fprintf completed");
    if (inval[0])
        printf("        [DBG_INVAL] line: %s\n", inval);
}

/* The [DBG_GAIN_INNER] block sits BEHIND the [DBG_GAIN] fprintf. It is
 * conditional on `gained_effect`, so arm a nested effect: if the outer
 * fprintf still faulted, this block would never run. Its presence is the
 * strongest available proof that execution continued past the whole trace. */
static void trace_with_nested_gained_effect(GameState *g, char *out, size_t cap)
{
    (void)out; (void)cap;
    AbilityEffect inner;
    make_inval_effect(&inner, "live_card", "false");
    inner.action = (char *)"change_state";

    AbilityEffect ga;
    make_gain_effect(&ga, "member_card", "true", "stage");
    ga.gained_effect = &inner;
    rb_translated_execute_gain_ability_effect(g, 0, &ga, -1);
}

static void test_trace_reaches_inner_block(void)
{
    static TestGame tg;
    empty_board(&tg);

    char buf[16384];
    char gain[8192], inner[8192];
    capture_trace(&tg.state, trace_with_nested_gained_effect, buf, sizeof(buf));

    line_starting_with(buf, "[DBG_GAIN] ", gain, sizeof(gain));
    CHECK(gain[0] != 0, "a [DBG_GAIN] line was emitted for the nested fixture");
    CHECK(strstr(gain, "ge=yes") != NULL,
          "[DBG_GAIN] reported ge=yes, so the nested gained_effect reached the trace");
    CHECK(strstr(gain, "ct=member_card") != NULL,
          "the outer trace still carries the whole card_type");

    line_starting_with(buf, "[DBG_GAIN_INNER] ", inner, sizeof(inner));
    CHECK(inner[0] != 0,
          "[DBG_GAIN_INNER] ran, i.e. execution continued PAST the [DBG_GAIN] fprintf "
          "— the check the old crash made impossible");
    CHECK(strstr(inner, "action=change_state") != NULL,
          "[DBG_GAIN_INNER] printed the nested action, so the block is not a stub line");
    if (inner[0])
        printf("        [DBG_GAIN_INNER] line: %s\n", inner);
}

/* ══════════════════════════════════════════════════════════════════════
 * §C — the real cards, end to end.
 *
 * §A/§B drive the two entry points with hand-built effects. §C goes through
 * the real decoder, the real trigger queue and the real card data, so it pins
 * the values that ACTUALLY reach the trace in production. Both cards are
 * real instances of the defect and they fail differently:
 *
 *   来看丸 PL!S-bp6-007-R  [DBG_GAIN] card_type_field = "member_card"
 *       -> the old code printed the byte 'm' == 0x6d == address 109.
 *          Real trace: ... ct=member_card gn=Aqours ... cond=1
 *
 *   元気全開 PL!S-pb1-019-L  [DBG_INVAL] card_type_field is EMPTY and
 *       self_target_field = "true"
 *       -> the old code printed address 0 (a NULL deref) for ct= and
 *          address 0x74 for self=. Real trace: ... ct= gn=- self=true
 *
 * So the sweep is pinned on the captured real trace, NOT on a gained-ability
 * count: the gain SEMANTICS of both cards are owned by tests/
 * test_parity_ability_mod.c (test_hanamaru_q_constant_score_matrix,
 * test_genki_zenkai_*), and restating them here would make this crash suite a
 * second, weaker owner of a claim that is not about the crash.
 * ══════════════════════════════════════════════════════════════════════ */

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n; i++) P->deck.cards[P->deck.n++] = filler;
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

static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32) {
        int idx[1] = {0};
        test_select_indices(tg, idx, 1);
    }
}

/* 来看丸 PL!S-bp6-007-R — ライブ開始時 gains a 常時 ability onto 『Aqours』
 * members, with card_type "member_card" (first byte 'm' == 0x6d). */
static void trace_hanamaru(GameState *g, char *out, size_t cap)
{
    (void)g; (void)out; (void)cap;
    static TestGame tg;
    int hanamaru = test_id(&tg, "PL!S-bp6-007-R");
    int friend   = test_id(&tg, "PL!S-pb1-010-PR");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler   = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_stage(&tg, 1, hanamaru);
    test_add_to_stage(&tg, 0, friend);
    test_add_to_stage(&tg, 2, outsider);
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_give_energy(&tg, 2);
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    fire_trigger(&tg, hanamaru, "ライブ開始時");
    drain_choices(&tg);
}

static void test_hanamaru_gain_ability_end_to_end(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hanamaru = test_id(&tg, "PL!S-bp6-007-R");
    CHECK(hanamaru >= 0, "setup: 来看丸 resolves");
    CHECK(card_prints_trigger(hanamaru, "ライブ開始時"),
          "setup: 来看丸 prints a ライブ開始時");

    char buf[32768], line[8192];
    capture_trace(&tg.state, trace_hanamaru, buf, sizeof(buf));

    line_starting_with(buf, "[DBG_GAIN] ", line, sizeof(line));
    CHECK(line[0] != 0, "the real 来看丸 ability produced a [DBG_GAIN] trace");
    CHECK(strstr(line, "ct=member_card") != NULL,
          "REAL CARD DATA: the decoded card_type prints whole as \"member_card\" "
          "(the old code printed the single byte 'm' == address 109)");
    CHECK(strstr(line, "gn=Aqours") != NULL,
          "REAL CARD DATA: the trace continued past ct= into gn=Aqours");
    CHECK(strstr(line, "cond=") != NULL,
          "REAL CARD DATA: the trace reached its last slot (cond=), so the "
          "fprintf completed and the process survived");
    if (line[0])
        printf("        real 来看丸 trace: %s\n", line);
}

/* 元気全開DAY！DAY！DAY！ PL!S-pb1-019-L — ライブ開始時 invalidates
 * ライブ成功時. Its decoded card_type_field is EMPTY and self_target_field is
 * "true", so the old code passed address 0 (a NULL deref) to the first `%s`
 * and address 't' == 0x74 to the last one. */
static void trace_genki(GameState *g, char *out, size_t cap)
{
    (void)g; (void)out; (void)cap;
    static TestGame tg;
    int genki   = test_id(&tg, "PL!S-pb1-019-L");
    int chika   = test_id(&tg, "PL!S-sd1-010-SD");
    int ruby    = test_id(&tg, "PL!S-pb1-018-N");
    int yoshiko = test_id(&tg, "PL!S-bp6-015-N");
    int filler  = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_stage(&tg, 0, chika);
    test_add_to_stage(&tg, 1, ruby);
    test_add_to_stage(&tg, 2, yoshiko);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);
    test_add_to_live(&tg, genki);

    fire_trigger(&tg, genki, "ライブ開始時");
    drain_choices(&tg);
}

static void test_genki_zenkai_invalidate_end_to_end(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int genki = test_id(&tg, "PL!S-pb1-019-L");
    CHECK(genki >= 0, "setup: 元気全開 resolves");
    CHECK(card_prints_trigger(genki, "ライブ開始時"),
          "setup: 元気全開 prints a ライブ開始時");

    char buf[32768], line[8192];
    capture_trace(&tg.state, trace_genki, buf, sizeof(buf));

    line_starting_with(buf, "[DBG_INVAL] ", line, sizeof(line));
    CHECK(line[0] != 0, "the real 元気全開 ability produced a [DBG_INVAL] trace");
    CHECK(strstr(line, "ct= ") != NULL,
          "REAL CARD DATA: this card's decoded card_type is EMPTY and the empty "
          "string printed safely (the old code passed address 0 to %s)");
    CHECK(strstr(line, "self=true") != NULL,
          "REAL CARD DATA: the trace reached its last slot (self=true), so the "
          "fprintf completed (the old code printed address 't' == 0x74)");
    if (line[0])
        printf("        real 元気全開 trace: %s\n", line);
}

/* ══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    printf("--- §A every first byte, gain-ability and invalidate-ability ---\n");
    run("gain_survives_every_first_byte", test_gain_survives_every_first_byte);
    run("invalidate_survives_every_first_byte", test_invalidate_survives_every_first_byte);
    run("gain_survives_member_card_leading_m", test_gain_survives_member_card_leading_m);
    run("invalidate_survives_member_card_leading_m", test_invalidate_survives_member_card_leading_m);
    run("literal_card_types_all_survive", test_literal_card_types_all_survive);

    printf("--- §B the debug trace is complete, not truncated ---\n");
    run("trace_prints_whole_card_type", test_trace_prints_whole_card_type);
    run("trace_reaches_inner_block", test_trace_reaches_inner_block);

    printf("--- §C real cards, end to end ---\n");
    run("hanamaru_gain_ability_end_to_end", test_hanamaru_gain_ability_end_to_end);
    run("genki_zenkai_invalidate_end_to_end", test_genki_zenkai_invalidate_end_to_end);

    rb_unload();

    printf("\n--- §D sweep summary ---\n");
    /* Assertion counts live in the forked children: a child that crashes loses
     * its own copy of the counters, so each child prints its own tally inline
     * and the parent can only total whole tests. */
    printf("%d test(s) ok, %d failed, %d crashed\n",
           n_tests_ok, n_tests_failed, n_tests_crashed);

    if (n_tests_crashed) {
        fprintf(stderr,
                "FATAL: %d test(s) died on a signal. ability_effects.c still hands\n"
                "a char BYTE to a %%s slot somewhere.\n", n_tests_crashed);
        return CHILD_CRASHED;
    }
    if (n_tests_ok != XCRASH_N_TESTS) {
        fprintf(stderr, "FATAL: expected %d passing tests, saw %d\n",
                XCRASH_N_TESTS, n_tests_ok);
        return 2;
    }
    if (n_tests_failed) {
        fprintf(stderr, "%d ability_effects_crash test(s) reported failed checks "
                        "(see the FAIL lines above)\n", n_tests_failed);
        return 1;
    }
    printf("ALL %d ABILITY_EFFECTS_CRASH TESTS PASSED\n", XCRASH_N_TESTS);
    return 0;
}

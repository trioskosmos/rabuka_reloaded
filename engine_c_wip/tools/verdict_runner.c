/* ---------------------------------------------------------------------------
 * tools/verdict_runner.c -- THE CORPUS VERDICT LAYER
 * ---------------------------------------------------------------------------
 *
 *   usage: rb_verdict <binary> <suite-name> [args...]
 *
 * Runs ONE suite, tees its stdout and stderr through itself, normalises every
 * result prefix to the single canonical shape `FAIL: `, and prints exactly ONE
 * verdict line as the last thing the suite's output says:
 *
*     PASSED: <suite>  [exit=0, 0 result line(s)]   -- exit 0, zero results
 *     FAILED: <suite>  [exit=<n>, <breakdown>]      -- a result line, or a
 *                                                            non-zero exit
 *     (nothing, plus one `FAIL: [crash]` line)       -- died on a signal
 *
 * The `[exit=` marker is private to this file -- no suite emits it -- so a
 * verdict is distinguishable from a suite's own `FAILED: <case>` line by
 * shape, which is what tests/test_coverage_gate.c relies on.
 *
 * WHY THIS EXISTS.  The corpus emitted 990 `FAILED` verdict lines and ZERO
 * `PASSED` lines, so NOTHING anywhere said "this suite was clean".  Therefore a
 * suite that SEGFAULTS before its first assertion produced exactly the same
 * output shape as a suite that passed every assertion -- unless you read the
 * exit code, and COVERAGE_HARNESS.md 3 proves the exit code disagrees with the
 * output in BOTH directions (12 suites exit 0 carrying 548 FAIL lines; 1 exits
 * 139 carrying 0).  That is how tmp_list, test_audit_conditions,
 * test_state_effects, test_p1_helpers and the nine bracket-form suites survived
 * in a project whose every report quotes a FAIL count.
 *
 * A third, distinct state now exists and is NOT "green": a suite that dies on
 * a signal emits NO verdict line at all, only a canonical `FAIL: [crash]`.
 *
 * `PASSED` requires BOTH halves.  It is not "zero FAIL lines" (that is what
 * test_audit_conditions scored today while running zero assertions) and it is
 * not `$?` (that is what 12 suites exit 0 while carrying FAIL lines).  It is
 * exit 0 AND zero result lines, which is strictly stronger than either.
 *
 * WHY A SEPARATE PROCESS AND NOT A MACRO IN EACH SUITE.  The shared reporting
 * macro the corpus already has is FLAT_VERDICT() in include/test_game.h, and it
 * only serves the FLAT-suite dialect.  The other three dialects (named case
 * functions in a run table, one main() with inline scenarios, and
 * run(name, fn) dispatch) count failures in their own differently-named local
 * counters, so reaching all of them would be 156 hand edits to files other
 * agents currently own.  One Makefile change routes the whole corpus through
 * here instead.
 *
 * PREFIX NORMALISATION, and why it is not a source edit.  The corpus emitted
 * SIX result prefixes -- `FAIL:`, `FAILED:`, `FAILED`, `FAIL [`,
 * `CRASH`, `VACUOUS`, `SETUP BUG:` -- of which only two carry a colon, so a
 * `^FAIL:` scan missed 1209 of 3486 failures and called nine suites perfectly
 * green.  Normalising at the source is impossible for the `printf("%-8s %s\n",
 * "FAILED", name)` form: there the literal is an ARGUMENT, not the start of
 * the format.  Doing it here, on the bytes the suite actually emitted, covers
 * every form including that one, in one place.
 *
 * ORDERING.  Output is forwarded the moment it arrives, so the interleaving of
 * a suite's own stdout and stderr is preserved as closely as a pipe allows.
 * Only the first VS_KEEP bytes of a line are held, and only until that line's
 * newline; a longer line is forwarded untouched (no result prefix is that
 * long).  On EOF any partial line is flushed, so nothing is swallowed.
 *
 * EXIT STATUS mirrors the child, so every existing `|| { echo RED }` in the
 * Makefile keeps meaning what it meant.
 *
 * ASCII ONLY, like every other file in this tree.
 * --------------------------------------------------------------------------- */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/select.h>
#include <sys/stat.h>

/* -- how many LEADING bytes of a line are held for normalisation ----------
 * Normalisation only ever rewrites the PREFIX, so the window has to cover the
 * longest prefix plus one boundary byte -- not the whole line.  The longest
 * prefix in the table is "SETUP BUG:" (10 bytes), so 32 is ample.
 *
 * The first version of this file buffered 96 bytes and gave up on any line
 * longer than that, and that was measurably wrong: it left 242 real result
 * lines unnormalised across 15 suites, because the corpus's longest result
 * lines -- test_audit_actions' TEST BUG notes and test_parity_cost_discount's
 * VACUOUS case notes -- run to several hundred bytes.  Deciding on the prefix
 * window alone and copying the rest straight through normalises every line at
 * any length while holding back at most 32 bytes. */
#define VS_HEAD 32

/* A prefix only matches if what follows it is a boundary.  Without this,
 * "FAILURE: ..." would be rewritten as "FAIL:URE: ..." -- and the corpus does
 * print lines that merely start with the letters. */
static int vs_boundary(const char *head, size_t len, size_t plen)
{
    if (len == plen) return 1;                 /* the prefix is the whole line */
    switch (head[plen]) {
    case ':': case ' ': case '\t': case '[': return 1;
    default: return 0;
    }
}

typedef struct { const char *from; const char *to; } VsMap;

/* LONGEST first within each word: `FAILED:` before `FAILED`, and `FAIL:`
 * before `FAIL ` before `FAIL`.  The `FAIL:` identity row is what makes an
 * already-canonical line COUNT without being rewritten.
 *
 * The tagged replacements carry NO trailing space on purpose: every corpus
 * site writes `TAG: <text>`, so the space already lives in the remainder and
 * adding one here produced `FAIL: [vacuous]  f_center_...`.  The bare (no
 * colon) forms do carry one, because nothing guarantees a separator there. */
static const VsMap VS_PREFIXES[] = {
    { "SETUP BUG:", "FAIL: [setup-bug]" },
    { "SETUPBUG ",  "FAIL: [setup-bug]" },
    { "SETUPBUG",   "FAIL: [setup-bug] " },
    { "TEST BUG:",  "FAIL: [test-bug]" },
    { "VACUOUS:",   "FAIL: [vacuous]" },
    { "VACUOUS ",   "FAIL: [vacuous]" },
    { "VACUOUS",    "FAIL: [vacuous] " },
    { "FATAL:",     "FAIL: [fatal]" },
    { "CRASH:",     "FAIL: [crash]" },
    { "CRASH",      "FAIL: [crash] " },
    { "FAILED:",    "FAIL: " },
    { "FAILED",     "FAIL:" },
    { "FAIL:",      "FAIL:" },
    { "FAIL ",      "FAIL: " },
    { "FAIL",       "FAIL:" }
};
#define VS_NPREFIX ((int)(sizeof VS_PREFIXES / sizeof VS_PREFIXES[0]))

/* -- tallies, shared across both streams so a per-case join still works ---- */
static long VS_N_ASSERT, VS_N_VACUOUS, VS_N_SETUP, VS_N_CRASH;
static long VS_N_TESTBUG, VS_N_FATAL, VS_N_REWRITTEN, VS_N_TOTAL;

/* -- one stream's head window ---------------------------------------------- */
typedef struct {
    FILE  *out;
    int    decided;      /* the prefix decision is made; copy through to \n */
    int    len;          /* bytes currently in head                        */
    char   head[VS_HEAD];
} VsStream;

/* Decide the prefix from `head[0..len)`, count the line, and write it.  Leaves
 * len == 0.  Called either because the line ended, or because the head window
 * filled up -- in the second case the rest of the line is copied through
 * untouched, which is why an arbitrarily long result line is still normalised. */
static void vs_decide(VsStream *st)
{
    int i, hit = -1;
    size_t plen = 0;
    for (i = 0; i < VS_NPREFIX; i++) {
        plen = strlen(VS_PREFIXES[i].from);
        if ((size_t)st->len >= plen &&
            memcmp(st->head, VS_PREFIXES[i].from, plen) == 0 &&
            vs_boundary(st->head, (size_t)st->len, plen)) { hit = i; break; }
    }
    if (hit >= 0) {
        const char *f = VS_PREFIXES[hit].from;
        VS_N_TOTAL++;
        if      (!strncmp(f, "VACUOUS", 7))  VS_N_VACUOUS++;
        else if (!strncmp(f, "SETUP BUG", 9) || !strncmp(f, "SETUPBUG", 8)) VS_N_SETUP++;
        else if (!strncmp(f, "TEST BUG", 8)) VS_N_TESTBUG++;
        else if (!strncmp(f, "FATAL", 5))    VS_N_FATAL++;
        else if (!strncmp(f, "CRASH", 5))    VS_N_CRASH++;
        else                                 VS_N_ASSERT++;
        if (strcmp(VS_PREFIXES[hit].from, VS_PREFIXES[hit].to) != 0) {
            fputs(VS_PREFIXES[hit].to, st->out);
            if (plen < (size_t)st->len)
                fwrite(st->head + plen, 1, (size_t)st->len - plen, st->out);
            VS_N_REWRITTEN++;
            st->len = 0;
            return;
        }
    }
    fwrite(st->head, 1, (size_t)st->len, st->out);
    st->len = 0;
}

/* -- the byte pump --------------------------------------------------------- */
static void vs_feed(VsStream *st, const char *buf, size_t n)
{
    size_t i = 0;
    while (i < n) {
        const char *nl;
        size_t take, room;
        if (st->decided) {
            nl = memchr(buf + i, '\n', n - i);
            take = nl ? (size_t)(nl - (buf + i)) + 1 : n - i;
            fwrite(buf + i, 1, take, st->out);
            i += take;
            if (nl) { st->decided = 0; st->len = 0; }
            continue;
        }
        nl = memchr(buf + i, '\n', n - i);
        take = nl ? (size_t)(nl - (buf + i)) + 1 : n - i;   /* up to and incl \n */
        room = (size_t)VS_HEAD - (size_t)st->len;
        if (take <= room) {
            memcpy(st->head + st->len, buf + i, take);
            st->len += (int)take;
            i += take;
            if (nl) vs_decide(st);            /* the newline decided the line */
            continue;
        }
        /* Head window is full and there is no newline in this chunk: the prefix
         * is decided by the window alone. */
        memcpy(st->head + st->len, buf + i, room);
        st->len += (int)room;
        i += room;
        vs_decide(st);
        st->decided = 1;
    }
}

static void vs_flush(VsStream *st)
{
    if (st->decided) { st->decided = 0; }
    else if (st->len) vs_decide(st);
    fflush(st->out);
}

/* -- THE STRUCTURED VERDICT ------------------------------------------------
 *
 * The text verdict line above is still printed -- it is what a human reads at
 * the bottom of a log, and the Makefile's run-all-tests greps for it -- but a
 * CONSUMER should never have to scrape a log to learn whether a suite passed.
 * That is the defect this file exists to remove, and leaving the answer only in
 * stdout re-creates it one layer up: a gate that parses logs is measuring the
 * size and shape of text, and the day a suite's prose changes the gate's
 * verdict changes with it.
 *
 * So every run ALSO writes ONE JSON object per suite, and that -- not stdout --
 * is the corpus's measurement surface:
 *
 *     verdicts/<suite>.json
 *
 * The directory comes from $RB_VERDICT_JSON_DIR, defaulting to `verdicts`
 * beside the runner's working directory (the engine_c_wip root, which is where
 * every suite is launched from).  $RB_VERDICT_JSON=0 turns the file off for a
 * one-off run that must not leave artefacts.
 *
 * WHY THE THREE VERDICT VALUES, and why INCONCLUSIVE is a value rather than an
 * absence: a suite killed on a signal emitted NO verdict line at all, so every
 * log-scraping consumer counted it as "no failures seen" -- green.  Written
 * down, it is a third state that cannot be confused with either of the other
 * two, because it is spelled out.
 *
 * `exit` is recorded for forensics only.  It is NOT the verdict, and nothing
 * downstream may read it as one: COVERAGE_HARNESS.md 3 shows suites exiting 0
 * while carrying FAIL lines AND suites exiting non-zero carrying none.
 * `n_result_lines` is the assertion count the verdict was actually decided on.
 */
static void vs_write_json(const char *name, const char *verdict,
                          int exit_code, int signalled)
{
    const char *dir = getenv("RB_VERDICT_JSON_DIR");
    char path[1024];
    FILE *f;
    const char *off = getenv("RB_VERDICT_JSON");

    if (off && strcmp(off, "0") == 0) return;
    if (!dir || !*dir) dir = "verdicts";
    if (mkdir(dir, 0777) != 0 && errno != EEXIST) {
        fprintf(stderr, "FAIL: [setup-bug] rb_verdict: mkdir %s failed: %s\n",
                dir, strerror(errno));
        return;
    }
    snprintf(path, sizeof path, "%s/%s.json", dir, name);
    f = fopen(path, "wb");
    if (!f) {
        fprintf(stderr, "FAIL: [setup-bug] rb_verdict: cannot write %s: %s\n",
                path, strerror(errno));
        return;
    }
    fprintf(f,
        "{\n"
        "  \"suite\": \"%s\",\n"
        "  \"verdict\": \"%s\",\n"
        "  \"exit\": %d,\n"
        "  \"signal\": %d,\n"
        "  \"n_result_lines\": %ld,\n"
        "  \"n_assert\": %ld,\n"
        "  \"n_vacuous\": %ld,\n"
        "  \"n_setup_bug\": %ld,\n"
        "  \"n_test_bug\": %ld,\n"
        "  \"n_fatal\": %ld,\n"
        "  \"n_crash\": %ld,\n"
        "  \"n_renormalised\": %ld\n"
        "}\n",
        name, verdict, exit_code, signalled,
        VS_N_TOTAL, VS_N_ASSERT, VS_N_VACUOUS, VS_N_SETUP,
        VS_N_TESTBUG, VS_N_FATAL, VS_N_CRASH, VS_N_REWRITTEN);
    fclose(f);
}

/* -- main ------------------------------------------------------------------ */
int main(int argc, char **argv)
{
    int outp[2], errp[2];
    pid_t pid;
    int status = 0;
    const char *bin, *name;
    VsStream so, se;
    int open_out = 1, open_err = 1;
    int signalled = 0, exit_code = 0;

    if (argc < 3) {
        fprintf(stderr, "usage: %s <binary> <suite-name> [args...]\n", argv[0]);
        return 2;
    }
    bin  = argv[1];
    name = argv[2];

    if (pipe(outp) != 0 || pipe(errp) != 0) {
        fprintf(stderr, "FAIL: [setup-bug] rb_verdict: pipe failed: %s\n",
                strerror(errno));
        return 2;
    }
    fflush(NULL);
    pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: [setup-bug] rb_verdict: fork failed: %s\n",
                strerror(errno));
        return 2;
    }
    if (pid == 0) {
        /* The child must see [bin, argv[3], ...] -- argv[2] is the SUITE NAME,
         * which belongs to the runner, not to the suite. */
        char **cargv = (char **)malloc(sizeof(char *) * (size_t)(argc - 1));
        int i;
        if (!cargv) _exit(127);
        cargv[0] = (char *)bin;
        for (i = 3; i < argc; i++) cargv[i - 2] = argv[i];
        cargv[argc - 2] = NULL;
        dup2(outp[1], STDOUT_FILENO);
        dup2(errp[1], STDERR_FILENO);
        close(outp[0]); close(outp[1]);
        close(errp[0]); close(errp[1]);
        execv(bin, cargv);
        fprintf(stderr, "FAIL: [setup-bug] rb_verdict: cannot exec %s: %s\n",
                bin, strerror(errno));
        _exit(127);
    }
    close(outp[1]);
    close(errp[1]);

    so.out = stdout; so.decided = 0; so.len = 0;
    se.out = stderr; se.decided = 0; se.len = 0;

    while (open_out || open_err) {
        fd_set rf;
        int maxfd = -1, n;
        struct timeval tv;
        FD_ZERO(&rf);
        if (open_out) { FD_SET(outp[0], &rf); if (outp[0] > maxfd) maxfd = outp[0]; }
        if (open_err) { FD_SET(errp[0], &rf); if (errp[0] > maxfd) maxfd = errp[0]; }
        tv.tv_sec = 1; tv.tv_usec = 0;
        n = select(maxfd + 1, &rf, NULL, NULL, &tv);
        if (n < 0) {
            if (errno == EINTR) continue;
            break;
        }
        if (n == 0) continue;                 /* no data yet: keep waiting */
        if (open_out && FD_ISSET(outp[0], &rf)) {
            char buf[65536];
            ssize_t got = read(outp[0], buf, sizeof buf);
            if (got > 0) vs_feed(&so, buf, (size_t)got);
            else if (got == 0) { vs_flush(&so); close(outp[0]); open_out = 0; }
        }
        if (open_err && FD_ISSET(errp[0], &rf)) {
            char buf[65536];
            ssize_t got = read(errp[0], buf, sizeof buf);
            if (got > 0) vs_feed(&se, buf, (size_t)got);
            else if (got == 0) { vs_flush(&se); close(errp[0]); open_err = 0; }
        }
    }
    vs_flush(&so);
    vs_flush(&se);

    while (waitpid(pid, &status, 0) < 0 && errno == EINTR) { /* retry */ }

    if (WIFSIGNALED(status)) {
        signalled = WTERMSIG(status);
        exit_code = 128 + signalled;
    } else {
        exit_code = WEXITSTATUS(status);
    }

    fflush(NULL);
    if (signalled) {
        /* Canonical, so `^FAIL` sees it -- and NO verdict line, because a suite
         * that died is neither PASSED nor FAILED. */
        fprintf(stderr, "FAIL: [crash] %s died on signal %d (128+%d): NO VERDICT "
                        "WAS EMITTED. Treat this suite as INCONCLUSIVE, not "
                        "green.\n", name, signalled, signalled);
        fflush(NULL);
        vs_write_json(name, "INCONCLUSIVE", exit_code, signalled);
        return exit_code;
    }
    if (VS_N_TOTAL == 0 && exit_code == 0) {
        printf("PASSED: %s  [exit=0, 0 result line(s), renormalised=%ld]\n",
               name, VS_N_REWRITTEN);
        vs_write_json(name, "PASSED", exit_code, 0);
    } else {
        printf("FAILED: %s  [exit=%d, %ld result line(s): assert=%ld vacuous=%ld "
               "setup-bug=%ld crash=%ld test-bug=%ld fatal=%ld renormalised=%ld]\n",
               name, exit_code, VS_N_TOTAL, VS_N_ASSERT, VS_N_VACUOUS,
               VS_N_SETUP, VS_N_CRASH, VS_N_TESTBUG, VS_N_FATAL,
               VS_N_REWRITTEN);
        vs_write_json(name, "FAILED", exit_code, 0);
    }
    fflush(NULL);
    return exit_code;
}
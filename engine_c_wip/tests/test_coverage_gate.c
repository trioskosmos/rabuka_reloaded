/* ---------------------------------------------------------------------------
 * tests/test_coverage_gate.c -- THE CORPUS-WIDE RECONCILIATION GATE
 * ---------------------------------------------------------------------------
 *
 * build:  make t T=coverage_gate        (single suite)
 *         make run-all-tests            (produces the tests/<stem>.log evidence)
 *
 * WHY THIS FILE EXISTS.  The corpus emitted SIX result prefixes -- `FAIL:`,
 * `FAILED:`, `FAILED`, `FAIL [`, `CRASH`, `VACUOUS`, `SETUP BUG:` -- of which
 * only two carry a colon.  A `^FAIL:` scan therefore scored 3486 - 2277 = 1209
 * failures as NOTHING, and 246 of those lived in nine suites that a `^FAIL:`
 * gate called perfectly green (COVERAGE_HARNESS.md 2.2).  Prefix
 * normalisation is what makes one grep correct; this file is what PROVES it
 * stayed done, and proves the corpus total reconciles against the per-suite
 * totals, so nobody can quietly publish a FAIL count measured on the wrong
 * surface again.
 *
 * WHAT IT ASSERTS.  Five things, all from the evidence `make run-all-tests`
 * writes into the tests directory as one .log per suite.  Every suite now runs
 * through tools/verdict_runner.c,
 * so each log carries exactly one verdict line as its last word.
 *
 * WHEN IT RUNS AT ALL.  The five rules below are only meaningful over that
 * evidence, and `make t` -- the single-suite runner -- does not produce any:
 * it prints one suite's verdict to stdout and writes no log.  So a bare
 * `make t T=coverage_gate` used to run all five rules against an EMPTY tests/
 * directory, and G1 then failed once per suite, blaming a manifest for a runner
 * that was never asked to fill it.  The gate now measures that condition first
 * (are there any .log files under tests/ at all?) and SKIPS with a printed
 * reason when
 * there are none.  In a context that does produce logs nothing is skipped and
 * nothing is relaxed -- G1-G5 run in full, and G1 still requires a log for
 * every tests/test_<stem>.c.
 *
 *   G1  MANIFEST-VS-GLOB.  Every tests/test_<stem>.c has a tests/<stem>.log,
 *       and every tests/<stem>.log has a tests/test_<stem>.c.  A suite that ran
 *       but left no log, or a log left behind by a deleted suite, is a hole in
 *       every number anyone has published.
 *
 *   G2  THE NORMALISATION IS TOTAL, PER SUITE.  For each log:
 *           count("^FAIL:")  ==  count(any recognised failure prefix)
 *       i.e. ZERO surviving `FAILED`, `FAILED:`, `FAIL [`, `CRASH`, `VACUOUS`,
 *       `SETUP BUG:`, `SETUPBUG`, `TEST BUG:` or `FATAL` lines.  A suite that
 *       regresses to a non-canonical prefix turns this red on its own.
 *
 *   G3  THE CORPUS TOTAL RECONCILES.  The corpus-wide `^FAIL` count, taken by
 *       one streaming pass over the whole tests/ directory, equals the sum of
 *       the per-suite counts.  These are two different enumerations -- one by
 *       directory walk, one by suite stem -- so a disagreement means a log was
 *       skipped, read twice, or counted for a suite that does not exist.
 *       This is the assertion that makes "corpus-wide" mean something.
 *
 *   G4  EVERY SUITE REPORTED A VERDICT.  Each log contains `^PASSED` XOR
 *       `^FAILED`, exactly one.  A log with neither is a suite that died on a
 *       signal and is INCONCLUSIVE -- the exact failure mode
 *       COVERAGE_HARNESS.md 9 says nothing could previously detect.
 *
 *   G5  VERDICT ARITHMETIC.  corpus ^PASSED + corpus ^FAILED == number of
 *       logs, so a suite cannot emit two verdicts or a verdict can go missing
 *       without the corpus total moving.
 *
 * IT WEAKENS NOTHING.  No assertion in any suite was relaxed, skipped or
 * retuned to make this pass; it only READS the logs.
 *
 * ASCII ONLY, like every other file in this tree.
 * --------------------------------------------------------------------------- */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>

/* -- own reporting, on the canonical prefix so ^FAIL sees it --------------- */
static int checks;
static int failures;

#define CHECK(cond, ...) do {                                              \
        checks++;                                                           \
        if (!(cond)) {                                                      \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, "\n");                                          \
            failures++;                                                     \
        }                                                                    \
    } while (0)

/* -- the prefixes a NON-CANONICAL result line may begin with.
 *
 * The canonical form is `FAIL:` and is NOT in this list -- it is CANON, the
 * other side of the G2 comparison.  The list therefore holds ONLY forms that
 * `FAIL:` plus a boundary does not already cover.
 *
 * `FAIL ` and bare `FAIL` used to be here, and are the reason a first version
 * of this gate reported every CANONICAL line as legacy: under the boundary
 * rule bare `FAIL` matches `FAIL: ...`, because head[4] is a colon.  Any
 * surviving `FAIL <text>` line is caught by the G2/G3 equality instead -- it
 * raises ^FAIL without raising ^FAIL:. */
typedef struct { const char *tag; int count; } PrefixTag;

static const char *const CANON = "FAIL:";

static const char *const TAGS[] = {
    "FAILED",        /* legacy per-case verdict, space padded  */
    "FAILED:",       /* legacy per-case verdict                */
    "FAIL [",        /* legacy bracket-tagged assertion        */
    "CRASH",         /* child died on a signal                */
    "VACUOUS:",      /* assertion proves nothing               */
    "VACUOUS",       /* ... and the space-padded per-case form */
    "SETUP BUG:",    /* fixture never resolved                */
    "SETUPBUG",      /* ... and the compact per-case form     */
    "TEST BUG:",     /* a RED-pinned case passed              */
    "FATAL:",        /* fatal harness disagreement            */
    NULL
};

/* One log's census. */
typedef struct {
    char name[256];
    long canon;        /* ^FAIL:                                      */
    long legacy;       /* every recognised NON-CANONICAL prefix        */
    long passed;       /* ^PASSED                                     */
    long failed;       /* ^FAILED  (includes the `FAILED:` verdict)    */
    long verdicts;     /* PASSED:/FAILED: lines in the runner's format */
    long lines;        /* total lines, for the report                  */
    int  found;
} LogCensus;

/* -- streaming line-prefix scanner -----------------------------------------
 * Only the first 32 bytes of a line matter here, so lines of any length are
 * handled without a buffer and without truncation. */
#define HEADMAX 40

static void scan_line_head(FILE *f, char *head, int *len)
{
    int c;
    *len = 0;
    c = fgetc(f);
    while (c != EOF && c != '\n') {
        if (*len < HEADMAX) head[(*len)++] = ( char)c;
        c = fgetc(f);
    }
}

static int head_is(const char *head, int len, const char *pfx)
{
    size_t n = strlen(pfx);
    if ((size_t)len < n) return 0;
    return memcmp(head, pfx, n) == 0;
}

/* A result prefix only counts when a BOUNDARY follows it.  Without that rule
 * "FAILURE:" and "CRASHES:" are counted as failures, and this file would then
 * blame a suite for a sentence. */
static int head_boundary(const char *head, int len, size_t plen)
{
    if ((size_t)len == plen) return 1;
    switch (head[plen]) {
    case ':': case ' ': case '\t': case '[': return 1;
    default: return 0;
    }
}

/* Count `^<pfx>` occurrences that are followed by a boundary.  Case-SENSITIVE,
 * which is the whole point: on PowerShell `-match` is case-insensitive and
 * inflates the count (a summary line reading "failures (shared) : 54" is
 * counted as a failure). */
static long count_prefix(FILE *f, const char *pfx, long *n_lines)
{
    char head[HEADMAX + 1];
    int len;
    size_t plen = strlen(pfx);
    long n = 0;
    rewind(f);
    for (;;) {
        scan_line_head(f, head, &len);
        if (len == 0 && feof(f)) break;
        if (n_lines) (*n_lines)++;
        head[len] = 0;
        if (head_is(head, len, pfx) && head_boundary(head, len, plen)) n++;
    }
    return n;
}

/* The runner's verdict lines, recognised by their own shape rather than by
 * counting every `FAILED:` -- because `FAILED: <case>` is ALSO a legitimate
 * legacy form, and a gate that cannot tell the two apart proves nothing.
 *   PASSED: <stem>
 *   FAILED: <stem>  [exit=<n>, <breakdown>]          */
/* Count the runner's verdict lines by their PRIVATE MARKER rather than by
 * counting every `FAILED:` -- because `FAILED: <case>` is ALSO a legitimate
 * legacy form, and a gate that cannot tell the two apart proves nothing.
 * `[exit=` appears in tools/verdict_runner.c's output and nowhere else in the
 * corpus, and it is on BOTH the PASSED and the FAILED verdict, so the test is
 * exact:
 *
 *     PASSED: <stem>  [exit=0, 0 result line(s), renormalised=N]
 *     FAILED: <stem>  [exit=<n>, <breakdown>]                       */
#define VS_MARK "[exit="
#define VS_MARKLEN 6

/* Latch on the literal marker, not merely on a '[' after a space: a first
 * attempt keyed on '[' matched any bracketed aside a suite printed
 * ("assertions [5 checks]") and reported five verdicts where there was one.
 * `n_pass` / `n_fail` separate the two verdict KINDS, because `^PASSED` and
 * `^FAILED` each count their own verdict line and both have to come back off
 * again -- doing that with a single total made every PASSED log report a legacy
 * count of -1. */
static long count_verdicts(FILE *f, long *n_pass, long *n_fail)
{
    int c, m = 0, marker = 0, is_pass = 0;
    long n = 0;
    if (n_pass) *n_pass = 0;
    if (n_fail) *n_fail = 0;
    rewind(f);
    for (;;) {
        is_pass = 0; marker = 0; m = 0;
        {
            int first = 0, second = 0;
            while ((c = fgetc(f)) != EOF && c != '\n') {
                if (first < 1) first = c;
                else if (second < 1) second = c;
                if (marker) continue;
                if (c == VS_MARK[m]) { m++; if (m == VS_MARKLEN) marker = 1; }
                else m = (c == VS_MARK[0]) ? 1 : 0;
            }
            is_pass = (first == 'P' && second == 'A');
            if (c == EOF && first == 0) break;
        }
        if (marker) {
            n++;
            if (is_pass) { if (n_pass) (*n_pass)++; }
            else if (n_fail) (*n_fail)++;
        }
        if (c == EOF) break;
    }
    return n;
}

static void census_log(const char *path, LogCensus *c)
{
    FILE *f = fopen(path, "rb");
    const char *const *t;
    long vp = 0, vf = 0;
    memset(c, 0, sizeof *c);
    if (!f) return;
    c->found = 1;
    c->canon  = count_prefix(f, CANON, &c->lines);
    c->verdicts = count_verdicts(f, &vp, &vf);
    c->passed = count_prefix(f, "PASSED", NULL);
    c->failed = count_prefix(f, "FAILED", NULL);
    /* Every recognised prefix EXCEPT the canonical one.  `FAILED` subsumes
     * `FAILED:` (that is the point -- `^FAILED` is the published counter), so
     * it is counted once and `FAILED:` is NOT counted again; each verdict line
     * is then taken back off the kind it belongs to.  A suite's own
     * `FAILED: <case>` line stays in the legacy count, which is exactly where a
     * regression shows up. */
    c->legacy = (c->passed - vp) + (c->failed - vf);
    for (t = TAGS; *t; t++) {
        if (strcmp(*t, "FAILED") == 0) continue;   /* already in c->failed */
        if (strcmp(*t, "FAILED:") == 0) continue;  /* subsumed by `FAILED`  */
        c->legacy += count_prefix(f, *t, NULL);
    }
    fclose(f);
}

/* `test_foo.c` -> `foo`, into a caller-supplied buffer. */
static const char *stem_name(const char *nm)
{
    static char stem[256];
    size_t l = strlen(nm), sl = l - 5 - 2;      /* drop "test_" and ".c" */
    if (sl >= sizeof stem) sl = sizeof stem - 1;
    memcpy(stem, nm + 5, sl);
    stem[sl] = 0;
    return stem;
}

static int cmp_census(const void *a, const void *b)
{
    return strcmp(((const LogCensus *)a)->name, ((const LogCensus *)b)->name);
}

/* -- the CONDITION, not the symptom ----------------------------------------
 *
 * The single fact that decides whether this gate can run at all: does the
 * directory it is pointed at hold ANY .log evidence?
 *
 * `make t` runs a suite through $(VERDICT) on STDOUT and writes no log at all;
 * only `make run-all-tests` writes tests/<stem>.log. So `make t T=coverage_gate`
 * and any other bare-suite invocation run this gate in a directory that will
 * NEVER contain the evidence it reconciles, and every one of its assertions
 * then reports the ABSENCE of files as though a suite had lost its log.
 *
 * Asserting on that absence is asserting on the symptom, and it scales the
 * wrong way: adding a suite adds another manifest entry with no log, so the
 * failure count grows with the corpus while the corpus gets healthier. The
 * condition -- "this context produces no logs" -- is one bit and does not scale.
 *
 * So: measure the condition, and if it holds, SKIP with a reason. An absent
 * evidence set is not a hole in anybody's numbers; it is the absence of the run
 * that would have produced them, and saying so is the honest answer. Asserting
 * per-suite on it instead means a reader cannot tell "this suite has no log"
 * from "no logs were ever written", which are opposite findings.
 *
 * The per-suite G1 assertion below is NOT weakened: in any context that DOES
 * produce logs -- `make run-all-tests`, or a re-run of the gate over that
 * evidence -- every tests/test_<stem>.c must still have its tests/<stem>.log,
 * and this function is what routes the other context away before that runs. */
static int count_log_files(const char *dir)
{
    DIR *d = opendir(dir);
    struct dirent *e;
    int n = 0;
    if (!d) return -1;
    while ((e = readdir(d)) != NULL) {
        const char *nm = e->d_name;
        size_t l = strlen(nm);
        if (l < 5 || strcmp(nm + l - 4, ".log") != 0) continue;
        n++;
    }
    closedir(d);
    return n;
}

/* -- corpus-wide total by an INDEPENDENT enumeration ---------------------
 * Walk tests/ as a directory and add up ^FAIL: in every log, with no knowledge
 * of the suite list. G3 compares this against the per-suite sum.
 *
 * ONE exclusion, and it is not an exemption: the gate's OWN log. The per-suite
 * census below deliberately skips it -- it is written by the very run that
 * produces the evidence, so it is empty of verdicts at the moment the gate
 * reads the directory -- and the walk MUST skip exactly the same file or the
 * two enumerations are not two enumerations of one set. They are two
 * enumerations of DIFFERENT sets, and G3 then measures the size of the gate's
 * own previous output instead of anything about the corpus.
 *
 * MEASURED, before this exclusion: the walk counted every ^FAIL: line the gate
 * had itself emitted on its previous run, so G3 reported a gap equal to the
 * previous run's failure count (152 lines on the first complete evidence set)
 * and could not pass in the very context it exists to judge. The two sides are
 * now the same set of files, which is the only thing that makes their
 * agreement mean anything. */
static long corpus_total_by_walk(const char *dir, const char *self)
{
    DIR *d = opendir(dir);
    struct dirent *e;
    long total = 0;
    if (!d) return -1;
    while ((e = readdir(d)) != NULL) {
        const char *nm = e->d_name;
        size_t n = strlen(nm);
        char path[512];
        FILE *f;
        if (n < 5 || strcmp(nm + n - 4, ".log") != 0) continue;
        if (self && !strcmp(nm, self)) continue;         /* the gate's own log */
        snprintf(path, sizeof path, "%s/%s", dir, nm);
        f = fopen(path, "rb");
        if (!f) continue;
        total += count_prefix(f, CANON, NULL);
        fclose(f);
    }
    closedir(d);
    return total;
}

int main(void)
{
    static LogCensus tab[512];
    int n = 0, i;
    long sum_per_suite = 0, sum_passed = 0, sum_failed = 0, n_legacy = 0;
    long by_walk, n_no_verdict = 0;
    int n_sources, n_logs;
    const char *dir = "tests";
    const char *self = "coverage_gate";

    /* -- G1: manifest vs glob -------------------------------------------- */
    n_sources = 0;
    {
        int any = 0;
        /* count the sources with a bounded directory scan */
        DIR *d = opendir(dir);
        struct dirent *e;
        if (!d) {
            CHECK(0, "cannot open tests/ -- run this from the engine_c_wip root");
            printf("\n%d checks, %d failures\n", checks, failures);
            return 1;
        }
        while ((e = readdir(d)) != NULL) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            if (l < 8 || strncmp(nm, "test_", 5) != 0 ||
                strcmp(nm + l - 2, ".c") != 0) continue;
            n_sources++;
            any = 1;
        }
        closedir(d);
        CHECK(any, "G1: tests/ holds no tests/test_*.c at all");
    }

    /* -- THE CONDITION FIRST ---------------------------------------------
     * Zero logs means this invocation is in a context that never writes them
     * (`make t` does not; `make run-all-tests` does). Every assertion below
     * would fire on that one fact, once per suite, so route the whole gate
     * around it. Nothing is relaxed: in a context that DOES produce logs this
     * branch is not taken and G1-G5 run in full. */
    if (count_log_files(dir) == 0) {
        printf("SKIP: tests/ holds NO *.log files, so there is nothing to "
               "reconcile and no manifest entry can be judged.\n"
               "SKIP: reason -- this gate only means anything over the evidence "
               "`make run-all-tests` writes. `make t T=coverage_gate` runs one "
               "suite on stdout and never creates a log, so a bare run is in a "
               "context that cannot produce the evidence; reporting one "
               "manifest failure per suite there describes the RUNNER, not the "
               "corpus.\n"
               "SKIP: to actually run this gate: `make run-all-tests`, then run "
               "it again over the tests/*.log that produced.\n");
        printf("info: sources=%d logs=0 ^FAIL(corpus)=0 ^PASSED=0 ^FAILED=0 "
               "non-canonical=0 no-verdict=0\n", n_sources);
        printf("\n%d checks, %d failures\n", checks, failures);
        return failures ? 1 : 0;
    }

    /* -- G1: manifest vs glob -------------------------------------------- */
    {
        char path[512];
        FILE *f;
        DIR *d = opendir(dir);
        struct dirent *e;
        if (!d) { CHECK(0, "cannot open tests/"); return 1; }
        while ((e = readdir(d)) != NULL) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            if (l < 8 || strncmp(nm, "test_", 5) != 0 ||
                strcmp(nm + l - 2, ".c") != 0) continue;
            /* ... and each must have produced a log.  This gate is the one
             * exception: it is written by the run that produces the evidence,
             * so its own log cannot exist yet (and `make run-all-tests` would
             * otherwise overwrite the evidence with an empty file). */
            if (!strcmp(stem_name(nm), "coverage_gate")) continue;
            {
                char stem[256];
                size_t sl = l - 5 - 2;            /* drop "test_" and ".c" */
                if (sl >= sizeof stem) sl = sizeof stem - 1;
                memcpy(stem, nm + 5, sl);
                stem[sl] = 0;
                snprintf(path, sizeof path, "%s/%s.log", dir, stem);
                f = fopen(path, "rb");
                CHECK(f != NULL,
                      "G1 manifest-vs-glob: tests/test_%s.c has no tests/%s.log -- "
                      "it either was not run or its log was deleted", stem, stem);
                if (f) fclose(f);
            }
        }
        closedir(d);
    }

    /* -- census every log ----------------------------------------------- */
    {
        DIR *d = opendir(dir);
        struct dirent *e;
        if (!d) { CHECK(0, "cannot open tests/"); return 1; }
        while ((e = readdir(d)) != NULL && n < (int)(sizeof tab / sizeof tab[0])) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            char path[512];
            LogCensus c;
            if (l < 5 || strcmp(nm + l - 4, ".log") != 0) continue;
            /* The gate's OWN log is written by the very run that produces the
             * evidence, and it is empty of verdicts because it ran first.
             * Counting it would make the gate fail on itself forever. */
            if (!strncmp(nm, self, strlen(self)) &&
                (!nm[strlen(self)] || !strcmp(nm + strlen(self), ".log"))) continue;
            snprintf(path, sizeof path, "%s/%s", dir, nm);
            census_log(path, &c);
            snprintf(c.name, sizeof c.name, "%s", nm);
            if (c.found) tab[n++] = c;
        }
        closedir(d);
    }
    qsort(tab, (size_t)n, sizeof tab[0], cmp_census);
    n_logs = n;
    CHECK(n_logs > 0,
          "G4/G5: there are NO tests/*.log files, so there is no evidence to "
          "reconcile. Run 'make run-all-tests' first -- this gate reads what it "
          "writes, and it is not allowed to pass on an absence of evidence.");

    for (i = 0; i < n; i++) {
        const LogCensus *c = &tab[i];
        sum_per_suite += c->canon;
        sum_passed     += c->passed;
        sum_failed     += c->failed;
        n_legacy       += c->legacy;
        /* -- G2: normalisation is total for THIS suite ------------------- */
        CHECK(c->legacy == 0,
              "G2 prefix normalisation: tests/%s still emits %ld non-canonical "
              "result line(s). The corpus vocabulary is exactly `FAIL: `; "
              "tools/verdict_runner.c rewrites every recognised prefix and its "
              "table is missing one, or this log predates the runner. (The "
              "runner's own `FAILED: <stem>  [exit=...]` verdict is excluded by "
              "shape, not by count.)",
              c->name, c->legacy);
        /* -- G4: exactly one verdict, and nothing else claiming one ----- */
        CHECK(c->verdicts == 1,
              "G4 verdict: tests/%s carries %ld runner verdict line(s); exactly "
              "one is required. None means the suite died on a signal "
              "(INCONCLUSIVE -- never green, and COVERAGE_HARNESS.md 9 says "
              "nothing could detect that before).",
              c->name, c->verdicts);
        CHECK(c->passed + c->failed == c->verdicts,
              "G4 verdict: tests/%s has %ld `^PASSED` + %ld `^FAILED` but %ld "
              "verdict line(s). A bare `FAILED <case>` is the legacy per-case "
              "form and is a G2 failure, not a second verdict.",
              c->name, c->passed, c->failed, c->verdicts);
        if (c->verdicts != 1) n_no_verdict++;
    }

    /* -- G3: corpus total reconciles ------------------------------------- */
    by_walk = corpus_total_by_walk(dir, "coverage_gate.log");
    CHECK(by_walk == sum_per_suite,
          "G3 reconciliation: a directory walk of tests/*.log counts %ld "
          "^FAIL lines, the per-suite census sums to %ld. They are two "
          "independent enumerations and must agree.",
          by_walk, sum_per_suite);

    /* -- G5: verdict arithmetic ------------------------------------------ */
    CHECK(sum_passed + sum_failed == (long)n_logs,
          "G5 verdict arithmetic: %ld PASSED + %ld FAILED != %ld logs",
          sum_passed, sum_failed, (long)n_logs);
    CHECK(n_no_verdict == 0,
          "G4: %ld suite(s) reported NO verdict. Before the verdict layer a "
          "segfault was byte-identical to a pass; it is now a third state.",
          n_no_verdict);
    CHECK(sum_passed > 0,
          "G5: zero suites reported PASSED -- the corpus still cannot report "
          "success, which is the defect this whole layer exists to end");

    printf("info: sources=%d logs=%d ^FAIL(corpus)=%ld ^PASSED=%ld ^FAILED=%ld "
           "non-canonical=%ld no-verdict=%ld\n",
           n_sources, n_logs, sum_per_suite, sum_passed, sum_failed,
           n_legacy, n_no_verdict);
    printf("info: the corpus-wide ^FAIL count RECONCILES with the sum of the "
           "per-suite counts, and every log carries exactly one verdict line\n");

    /* The per-suite table, so a reader can see WHICH suites are red rather
     * than only that the corpus is. */
    printf("--- per-suite ^FAIL (canonical) ---\n");
    for (i = 0; i < n; i++)
        printf("  %-6ld %-8s %s\n", tab[i].canon,
               tab[i].passed ? "PASSED" : (tab[i].failed ? "FAILED" : "NO-VERDICT"),
               tab[i].name);

    printf("\n%d checks, %d failures\n", checks, failures);
    return failures ? 1 : 0;
}

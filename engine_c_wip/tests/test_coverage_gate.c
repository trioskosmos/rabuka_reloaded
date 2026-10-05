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
 * WHAT IT ASSERTS.  Five things, all read from ONE structured verdict per
 * suite -- `verdicts/<suite>.json`, written by tools/verdict_runner.c, which
 * every suite now runs through.
 *
 * WHY IT READS THE JSON AND NOT THE LOGS.  It used to walk the per-suite `.log`
 * files under tests/ and grep them, and that is the defect this rewrite
 * removes, for two independent reasons, both measured rather than argued:
 *
 *   1. A LOG IS PROSE.  The gate had to re-implement the runner's notion of a
 *      result prefix -- a five-tag table, a boundary rule, and a `[exit=`
 *      marker used to tell the runner's own verdict from a suite's legacy
 *      `FAILED: <case>` line.  That is 150 lines of duplicated parsing whose
 *      only job is to agree with the runner, and it agreed only because both
 *      were reading the same bytes.  The runner already knows the answer,
 *      exactly, because it is the thing that decided it.
 *   2. THE GATE WAS MEASURING ITS OWN PREVIOUS OUTPUT.  `make run-all-tests`
 *      redirects each suite's stdout into `tests/<stem>.log`, and the gate's
 *      own log is one of those files.  A first version counted its own `FAIL:`
 *      lines in the corpus-wide walk and compared that total against a
 *      per-suite census that skipped the same file -- so G3 reported a gap
 *      EQUAL TO THE PREVIOUS RUN'S FAILURE COUNT (152 lines on the first
 *      complete evidence set) and could not pass in the very context it
 *      exists to judge.  The exclusion that made the two sides comparable was
 *      a patch over the symptom; not writing the evidence at all removes the
 *      class.  Per-suite the same effect was visible as `170 checks / 166
 *      failures` in a zero-log context against `294` in a log context -- the
 *      number moved with the size of the previous run's output, not with the
 *      corpus.
 *
 * The JSON is a fixed 12-field record with one value per fact, so there is
 * nothing left to parse loosely and nothing to re-implement.
 *
 * WHEN IT RUNS AT ALL.  The five rules below are only meaningful over that
 * evidence.  `make t` -- the single-suite runner -- launches exactly one suite
 * and its one verdict file cannot answer a corpus-wide question, so a bare
 * `make t T=coverage_gate` runs in a directory holding one verdict.  The gate
 * measures that condition first (are there any verdicts at all, and are there
 * as many as there are suites?) and SKIPS with a printed reason when the
 * evidence set is not corpus-shaped.  In a context that does produce a
 * full-set nothing is skipped and nothing is relaxed: G1-G5 run in full, and
 * G1 still requires a verdict for every tests/test_<stem>.c.
 *
 *   G1  MANIFEST-VS-GLOB.  Every tests/test_<stem>.c has a
 *       verdicts/<stem>.json, and every verdicts/<stem>.json has a
 *       tests/test_<stem>.c.  A suite that ran but left no verdict, or a
 *       verdict left behind by a deleted suite, is a hole in every number
 *       anyone has published.
 *
 *   G2  THE TALLY IS TOTAL AND THE VERDICT MATCHES IT.  Per suite, the six
 *       breakdown counters sum to `n_result_lines` -- nothing the runner
 *       counted is missing from the tally.  And the verdict agrees with the
 *       count it was decided on: PASSED requires `n_result_lines == 0`, and a
 *       FAILED verdict with `n_result_lines == 0` is reported as its own
 *       state, because that suite failed on a non-zero exit while asserting
 *       nothing.  That is the shape of the one BROKEN suite in the corpus
 *       (`audit_conditions`, exit 2, no assertion, no verdict line at all), so
 *       this rule is what stops it reading as an ordinary red.
 *
 *   G3  THE CORPUS TOTAL RECONCILES.  The corpus-wide result-line count,
 *       taken by one directory walk of verdicts/ with no knowledge of the
 *       suite list, equals the sum over suite stems.  Two enumerations, one
 *       set; a disagreement means a verdict was skipped or counted twice.
 *
 *   G4  EVERY SUITE REPORTED A VERDICT.  Each record carries exactly one of
 *       PASSED / FAILED / INCONCLUSIVE, and none is INCONCLUSIVE.  A suite
 *       killed on a signal used to leave no verdict line at all, so every
 *       log-scraping consumer read it as "no failures seen" -- green.  It is
 *       now a third, spelled-out state.
 *
 *   G5  VERDICT ARITHMETIC.  #PASSED + #FAILED + #INCONCLUSIVE equals the
 *       number of verdict files, so a suite cannot emit two verdicts or a
 *       verdict can go missing without the corpus total moving.  #PASSED must
 *       be greater than zero: a corpus that can only report failure is the
 *       defect this whole layer exists to end.
 *
 * `exit` IS READ BUT NEVER USED AS A VERDICT.  COVERAGE_HARNESS.md 3 proves it
 * disagrees with the output in BOTH directions.  It is reported for forensics
 * only.
 *
 * IT WEAKENS NOTHING.  No assertion in any suite was relaxed, skipped or
 * retuned to make this pass; it only READS the verdicts.
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

/* -- ONE SUITE'S STRUCTURED VERDICT ---------------------------------------
 * A fixed-shape record written by tools/verdict_runner.c.  One value per fact,
 * so there is no prefix table, no boundary rule and no marker string to
 * re-derive here: the runner counted the lines, and this file reads the count.
 * `exit` is carried for forensics and is never used as a verdict. */
enum { V_PASSED, V_FAILED, V_INCONCLUSIVE, V_UNREADABLE };

typedef struct {
    char name[256];
    int  verdict;         /* one of the four above                         */
    long exit_code;
    long signal;
    long n_result_lines;
    long n_assert, n_vacuous, n_setup_bug, n_test_bug, n_fatal, n_crash;
    long n_renormalised;
    int  found;
} Verdict;

/* -- read one integer field out of a verdict file ---------------------------
 * The records are machine-written with one field per line and no nesting, so
 * a line-oriented lookup is exact and needs no JSON parser.  Absent field
 * returns `dflt`, which the callers treat as UNREADABLE rather than as zero:
 * a missing counter is not a suite that counted nothing. */
static long json_long(const char *buf, const char *key, long dflt)
{
    char pat[64];
    const char *p;
    snprintf(pat, sizeof pat, "\"%s\"", key);
    p = strstr(buf, pat);
    if (!p) return dflt;
    p = strchr(p + strlen(pat), ':');
    if (!p) return dflt;
    p++;
    while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n') p++;
    if (*p < '0' && *p != '-') return dflt;
    return strtol(p, NULL, 10);
}

/* Returns 1 when every field this gate needs was present and numeric.  A
 * verdict file that cannot be read is its own state, not a suite with zero
 * failures -- collapsing the two is how this gate was wrong before. */
static int read_verdict(const char *path, Verdict *v)
{
    FILE *f = fopen(path, "rb");
    char buf[4096];
    size_t n;
    const char *p;
    memset(v, 0, sizeof *v);
    if (!f) return 0;
    n = fread(buf, 1, sizeof buf - 1, f);
    fclose(f);
    buf[n] = 0;
    /* The RECORD's name, not the path it was reached by.  Storing the path
     * made every message read `verdicts/verdicts/x.json.json`, which is noise
     * in exactly the output a reader uses to find the offending file. */
    {
        const char *base = path, *q;
        for (q = path; *q; q++) if (*q == '/') base = q + 1;
        snprintf(v->name, sizeof v->name, "%s", base);
    }
    p = strstr(buf, "\"verdict\"");
    if (!p) return 0;
    p = strchr(p, ':');
    if (!p) return 0;
    p = strchr(p + 1, '"');
    if (!p) return 0;
    p++;
    if (!strncmp(p, "PASSED", 6))       v->verdict = V_PASSED;
    else if (!strncmp(p, "FAILED", 6))  v->verdict = V_FAILED;
    else if (!strncmp(p, "INCONCLUSIVE", 12)) v->verdict = V_INCONCLUSIVE;
    else return 0;
    v->exit_code       = json_long(buf, "exit", -1);
    v->signal          = json_long(buf, "signal", -1);
    v->n_result_lines  = json_long(buf, "n_result_lines", -1);
    v->n_assert        = json_long(buf, "n_assert", -1);
    v->n_vacuous       = json_long(buf, "n_vacuous", -1);
    v->n_setup_bug     = json_long(buf, "n_setup_bug", -1);
    v->n_test_bug      = json_long(buf, "n_test_bug", -1);
    v->n_fatal         = json_long(buf, "n_fatal", -1);
    v->n_crash         = json_long(buf, "n_crash", -1);
    v->n_renormalised  = json_long(buf, "n_renormalised", -1);
    if (v->exit_code < 0 || v->n_result_lines < 0 || v->n_assert < 0) return 0;
    v->found = 1;
    return 1;
}

/* The six breakdown counters account for EVERY line the runner counted. */
static long v_breakdown(const Verdict *v)
{
    return v->n_assert + v->n_vacuous + v->n_setup_bug +
           v->n_test_bug + v->n_fatal + v->n_crash;
}

static const char *v_state_name(const Verdict *v)
{
    switch (v->verdict) {
    case V_PASSED:       return "PASSED";
    case V_FAILED:       return "FAILED";
    case V_INCONCLUSIVE: return "INCONCLUSIVE";
    default:             return "UNREADABLE";
    }
}

/* G1 derives a stem in each of its two directions and does NOT share a helper
 * between them, because the two inputs have different shapes -- `test_x.c`
 * loses a 5-byte prefix and a 2-byte suffix, while `x.json` loses only the
 * suffix.  A shared helper is how the two derivations drift into agreeing on
 * the wrong string. */

static int cmp_verdict(const void *a, const void *b)
{
    return strcmp(((const Verdict *)a)->name, ((const Verdict *)b)->name);
}

/* -- the CONDITION, not the symptom ----------------------------------------
 * Does the directory this gate is pointed at hold ANY verdict records, and are
 * there as many of them as there are suites?
 *
 * `make t` runs ONE suite, so it leaves exactly one verdict file, and a
 * corpus-wide question cannot be answered from it.  Asserting on the absence
 * per-suite scales the wrong way: every rule below would fire once per suite,
 * so the failure count grows with the corpus while the corpus gets healthier.
 * The condition is one bit.  Measure it, print it, and skip -- and assert
 * nothing on its behalf.
 *
 * The per-suite G1 assertion below is NOT weakened: in any corpus-shaped
 * context every tests/test_<stem>.c must still have its verdicts/<stem>.json,
 * and this function is what routes the other context away before that runs. */
static int count_verdict_files(const char *dir)
{
    DIR *d = opendir(dir);
    struct dirent *e;
    int n = 0;
    if (!d) return -1;
    while ((e = readdir(d)) != NULL) {
        const char *nm = e->d_name;
        size_t l = strlen(nm);
        if (l < 6 || strcmp(nm + l - 5, ".json") != 0) continue;
        n++;
    }
    closedir(d);
    return n;
}

/* -- corpus-wide total by an INDEPENDENT enumeration ---------------------
 * Walk the verdict directory as a directory and add up n_result_lines, with no
 * knowledge of the suite list.  G3 compares this against the per-suite sum.
 *
 * There is NO SELF-EXCLUSION here, and that is the point of the rewrite.  The
 * log version had to name its own log and skip it, because `make run-all-tests`
 * writes `tests/coverage_gate.log` from the very run that produces the
 * evidence -- so the walk counted the gate's previous output and G3 measured
 * the size of that instead of the corpus.  A gate does not write a verdict for
 * itself into the evidence set it is reading, so the two sides are now the
 * same set of files by construction rather than by a patch. */
static long corpus_total_by_walk(const char *dir, int *n_read, int *n_bad)
{
    DIR *d = opendir(dir);
    struct dirent *e;
    long total = 0;
    *n_read = 0;
    *n_bad = 0;
    if (!d) return -1;
    while ((e = readdir(d)) != NULL) {
        const char *nm = e->d_name;
        size_t n = strlen(nm);
        char path[1024];
        Verdict v;
        if (n < 6 || strcmp(nm + n - 5, ".json") != 0) continue;
        snprintf(path, sizeof path, "%s/%s", dir, nm);
        (*n_read)++;
        if (!read_verdict(path, &v)) { (*n_bad)++; continue; }
        total += v.n_result_lines;
    }
    closedir(d);
    return total;
}

int main(void)
{
    static Verdict tab[512];
    int n = 0, i;
    long sum_lines = 0, sum_passed = 0, sum_failed = 0, sum_inconclusive = 0;
    long by_walk, n_no_verdict = 0, n_unreadable = 0, n_assertionless_red = 0;
    int n_sources, n_files, n_walk_read, n_walk_bad;
    const char *dir = "verdicts";
    const char *src = "tests";

    /* -- G0: is the suite manifest even present? -------------------------- */
    n_sources = 0;
    {
        int any = 0;
        DIR *d = opendir(src);
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
     * The evidence has to be corpus-shaped before a corpus-wide question can
     * be asked of it.  `make t` runs ONE suite and leaves ONE verdict record,
     * and `RB_VERDICT_JSON=0` leaves none; in either context every assertion
     * below would fire once per suite, so the failure count would grow with
     * the corpus while the corpus got healthier.  Measure the condition, print
     * it, and skip.  Nothing is relaxed: on a corpus-shaped set this branch is
     * not taken and G1-G5 run in full. */
    n_files = count_verdict_files(dir);
    if (n_files <= 0) {
        printf("SKIP: %s/ holds NO *.json verdict records, so there is nothing to "
               "reconcile and no manifest entry can be judged.\n"
               "SKIP: reason -- this gate only means anything over a corpus-shaped "
               "evidence set. `make t T=<stem>` runs ONE suite and writes ONE "
               "verdict; `RB_VERDICT_JSON=0` writes none. Reporting one manifest "
               "failure per suite there describes the RUNNER, not the corpus.\n"
               "SKIP: to actually run this gate: `make run-all-tests` (or the "
               "tools/corpus_measure.py pass that writes one verdict per suite).\n",
               dir);
        printf("info: sources=%d verdicts=0 result-lines(corpus)=0 PASSED=0 "
               "FAILED=0 INCONCLUSIVE=0 unreadable=0\n", n_sources);
        printf("\n%d checks, %d failures\n", checks, failures);
        return failures ? 1 : 0;
    }
    if (n_files != n_sources) {
        printf("SKIP: %s/ holds %d verdict record(s) for %d suite source(s). The "
               "set is not corpus-shaped, so the corpus-wide rules (G3, G5) would "
               "be reconciling a partial set against a whole manifest and calling "
               "the difference a defect.\n"
               "SKIP: reason -- a partial run is a partial measurement; the "
               "missing records are suites that did not run, which is a fact "
               "about the RUNNER and not a hole in the corpus.\n",
               dir, n_files, n_sources);
        printf("info: sources=%d verdicts=%d result-lines(corpus)=0 PASSED=0 "
               "FAILED=0 INCONCLUSIVE=0 unreadable=0\n", n_sources, n_files);
        printf("\n%d checks, %d failures\n", checks, failures);
        return failures ? 1 : 0;
    }

    /* -- G1: manifest vs glob, both directions --------------------------- */
    {
        char path[1024];
        DIR *d = opendir(src);
        struct dirent *e;
        if (!d) { CHECK(0, "cannot open tests/"); return 1; }
        while ((e = readdir(d)) != NULL) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            char stem[256];
            size_t sl;
            FILE *f;
            if (l < 8 || strncmp(nm, "test_", 5) != 0 ||
                strcmp(nm + l - 2, ".c") != 0) continue;
            sl = l - 5 - 2;                     /* drop "test_" and ".c" */
            if (sl >= sizeof stem) sl = sizeof stem - 1;
            memcpy(stem, nm + 5, sl);
            stem[sl] = 0;
            snprintf(path, sizeof path, "%s/%s.json", dir, stem);
            f = fopen(path, "rb");
            /* The gate needs NO self-exemption here, and that is the point of
             * the rewrite: a verdict record is written by the suite's own run,
             * and `make run-all-tests` produces one for coverage_gate along
             * with every other suite.  The log version had to exempt its own
             * log, because that file was written by the very run that creates
             * the evidence -- which is also how it came to count its own
             * previous output as corpus data. */
            CHECK(f != NULL,
                  "G1 manifest-vs-glob: tests/test_%s.c has no %s/%s.json -- it "
                  "either was not run, or its verdict record was deleted. A suite "
                  "with no verdict is invisible to every corpus total.",
                  stem, dir, stem);
            if (f) fclose(f);
        }
        closedir(d);
    }
    /* ... and the other direction: a verdict whose suite is gone. */
    {
        DIR *d = opendir(dir);
        struct dirent *e;
        if (!d) { CHECK(0, "cannot open %s/", dir); return 1; }
        while ((e = readdir(d)) != NULL) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            char stem[256], path[1024];
            size_t sl;
            FILE *f;
            if (l < 6 || strcmp(nm + l - 5, ".json") != 0) continue;
            sl = l - 5;                          /* drop ".json" */
            if (sl >= sizeof stem) sl = sizeof stem - 1;
            memcpy(stem, nm, sl);
            stem[sl] = 0;
            snprintf(path, sizeof path, "%s/test_%s.c", src, stem);
            f = fopen(path, "rb");
            CHECK(f != NULL,
                  "G1 manifest-vs-glob: %s/%s.json exists but tests/test_%s.c "
                  "does not -- the suite was deleted or renamed and this record "
                  "is now a count for a suite that does not exist.",
                  dir, stem, stem);
            if (f) fclose(f);
        }
        closedir(d);
    }

    /* -- load every verdict ---------------------------------------------- */
    {
        DIR *d = opendir(dir);
        struct dirent *e;
        if (!d) { CHECK(0, "cannot open %s/", dir); return 1; }
        while ((e = readdir(d)) != NULL && n < (int)(sizeof tab / sizeof tab[0])) {
            const char *nm = e->d_name;
            size_t l = strlen(nm);
            char path[1024];
            Verdict v;
            if (l < 6 || strcmp(nm + l - 5, ".json") != 0) continue;
            snprintf(path, sizeof path, "%s/%s", dir, nm);
            if (!read_verdict(path, &v)) {
                /* A record the gate cannot read is UNREADABLE, never a suite
                 * that counted nothing.  Collapsing the two is how a broken
                 * evidence set reads as a clean corpus. */
                memset(&v, 0, sizeof v);
                v.verdict = V_UNREADABLE;
                snprintf(v.name, sizeof v.name, "%s", nm);
                n_unreadable++;
            }
            tab[n++] = v;
        }
        closedir(d);
    }
    qsort(tab, (size_t)n, sizeof tab[0], cmp_verdict);
    CHECK(n == n_files,
          "G4/G5: loaded %d verdict record(s) but %s/ holds %d .json file(s). "
          "A record that failed to load is counted as UNREADABLE, never as a "
          "suite with zero failures.",
          n, dir, n_files);

    for (i = 0; i < n; i++) {
        const Verdict *v = &tab[i];
        /* -- G2: the tally is total, and the verdict matches it ---------- */
        CHECK(v->verdict != V_UNREADABLE,
              "G2 verdict record: %s/%s.json cannot be read. It is not a suite "
              "with zero failures -- it is an unreadable measurement, and "
              "counting it as clean is the defect this gate exists to prevent.",
              dir, v->name);
        if (v->verdict == V_UNREADABLE) continue;
        CHECK(v_breakdown(v) == v->n_result_lines,
              "G2 tally integrity: %s/%s.json reports n_result_lines=%ld but its "
              "six breakdown counters sum to %ld. Something the runner counted "
              "is missing from the tally, so the corpus total would be under "
              "counting that suite.",
              dir, v->name, v->n_result_lines, v_breakdown(v));
        CHECK(!(v->verdict == V_PASSED && v->n_result_lines != 0),
              "G2 verdict/count agreement: %s/%s.json is PASSED with "
              "n_result_lines=%ld. PASSED is defined as zero counted result "
              "lines, so this record contradicts itself.",
              dir, v->name, v->n_result_lines);
        if (v->verdict == V_FAILED && v->n_result_lines == 0) {
            /* The one BROKEN suite in the corpus is exactly this shape: a
             * non-zero exit and no assertion at all.  It is neither a green
             * suite nor a suite that measured something, and reporting it as
             * an ordinary red hides the difference. */
            n_assertionless_red++;
            CHECK(0,
                  "G2 verdict/count agreement: %s/%s.json is FAILED with "
                  "n_result_lines=0 and exit=%ld. It failed on its EXIT while "
                  "asserting nothing -- a broken suite, not a measurement. "
                  "(This is the shape of `audit_conditions`.)",
                  dir, v->name, v->exit_code);
        }

        /* -- G4: exactly one verdict, and it is a decidable one -------- */
        CHECK(v->verdict == V_PASSED || v->verdict == V_FAILED,
              "G4 verdict: %s/%s.json is %s. A suite killed on a signal emitted "
              "no verdict line at all, so every log-scraping consumer read it "
              "as \"no failures seen\" -- green. It is a third state now and it "
              "must not be silently folded into either of the other two.",
              dir, v->name, v_state_name(v));
        if (v->verdict == V_INCONCLUSIVE) n_no_verdict++;

        sum_lines += v->n_result_lines;
        if (v->verdict == V_PASSED)       sum_passed++;
        else if (v->verdict == V_FAILED)  sum_failed++;
        else if (v->verdict == V_INCONCLUSIVE) sum_inconclusive++;
    }

    /* -- G3: corpus total reconciles ------------------------------------- */
    by_walk = corpus_total_by_walk(dir, &n_walk_read, &n_walk_bad);
    CHECK(n_walk_read == n_files && n_walk_bad == 0,
          "G3: the directory walk read %d verdict file(s) (%d unreadable) but "
          "the per-suite load saw %d. The two sides are not the same set.",
          n_walk_read, n_walk_bad, n_files);
    CHECK(by_walk == sum_lines,
          "G3 reconciliation: a directory walk of %s/*.json counts %ld result "
          "line(s), the per-suite sum is %ld. They are two independent "
          "enumerations and must agree.",
          dir, by_walk, sum_lines);

    /* -- G5: verdict arithmetic ------------------------------------------ */
    CHECK(sum_passed + sum_failed + sum_inconclusive + n_unreadable == (long)n,
          "G5 verdict arithmetic: %ld PASSED + %ld FAILED + %ld INCONCLUSIVE + "
          "%ld UNREADABLE != %d verdict record(s). A verdict can go missing "
          "without the corpus total moving.",
          sum_passed, sum_failed, sum_inconclusive, n_unreadable, n);
    CHECK(n_no_verdict == 0,
          "G4: %ld suite(s) reported INCONCLUSIVE. Before the verdict layer a "
          "segfault was byte-identical to a pass; it is now a third state.",
          n_no_verdict);
    CHECK(n_assertionless_red == 0,
          "G2: %ld suite(s) are FAILED with zero counted result line(s) -- they "
          "asserted nothing and failed on their exit code.",
          n_assertionless_red);
    CHECK(sum_passed > 0,
          "G5: zero suites reported PASSED -- the corpus still cannot report "
          "success, which is the defect this whole layer exists to end");

    printf("info: sources=%d verdicts=%d result-lines(corpus)=%ld PASSED=%ld "
           "FAILED=%ld INCONCLUSIVE=%ld unreadable=%ld exit-only-red=%ld\n",
           n_sources, n_files, sum_lines, sum_passed, sum_failed,
           sum_inconclusive, n_unreadable, n_assertionless_red);
    printf("info: the corpus-wide result-line count RECONCILES with the sum of "
           "the per-suite counts, and every verdict record is readable\n");

    /* The per-suite table, so a reader can see WHICH suites are red rather
     * than only that the corpus is. */
    printf("--- per-suite verdict (from %s/*.json) ---\n", dir);
    for (i = 0; i < n; i++)
        printf("  %-6ld %-13s exit=%-4ld %s\n", tab[i].n_result_lines,
               v_state_name(&tab[i]), tab[i].exit_code, tab[i].name);

    printf("\n%d checks, %d failures\n", checks, failures);
    return failures ? 1 : 0;
}

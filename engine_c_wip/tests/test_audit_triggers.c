/* test_audit_triggers.c -- audit of the TRIGGER and COST chains.
 *
 * SCOPE
 * -----
 * Every ability in cards/abilities.json reaches the engine through two
 * contracts: the `triggers` string that says WHEN it fires, and the `cost`
 * tree that says WHAT must be paid first. This suite traces both, one card
 * at a time, end to end:
 *
 *     cards/abilities.json triggers[]  ->  cards.json ability text
 *       ->  RB_TSTR_* constant in include/rabuka.h
 *       ->  matcher (triggers.c rb_trigger_is / rb_parse_triggers)
 *       ->  scan (engine/src/turn/trigger_scan.rs is the reference)
 *       ->  consumer
 *
 * METHOD
 * ------
 * The unit of work is ONE card's trigger set, read by hand out of the JSON
 * and then compared byte for byte against the C. Nothing here enumerates the
 * corpus to prove a bug.
 *
 * PREMISE DISCIPLINE. The `(ab#N)` labels in abilities.json's `cards[]` lists
 * are NOT reliable indices into the decoded ability array: the exclude_characters
 * stage cost is labelled "PL!SP-pb1-010-P+ | ... (ab#0)" in abilities.json, but
 * that card number decodes to a single 常時 ability. The card that really
 * carries it is PL!SP-pb1-011-P+/PL!SP-pb1-011-R (cards.json cost 13, ability
 * "{{toujyou.png|登場}}「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから
 * 控え室に置いてもよい"). Every row below therefore re-derives its premise from
 * the DECODER and asserts that premise, so a stale label cannot silently
 * relocate a row onto a different ability.
 *
 * The [TRIGGER-MISS] helper exists because a hand-typed escape typo in a
 * trigger literal makes a negative assertion VACUOUS -- it compares two
 * different wrong strings and passes. Every trigger assertion goes through a
 * hex dump first, so a wrong constant cannot hide.
 *
 * RED IS EXPECTED. Each red row asserts the RUST behaviour on a real corpus
 * card and is classified. Never weaken one to go green; flip the ENGINE.
 *
 *   ENGINE BUG    the C reads the field but does the wrong thing with it.
 *   PARSER GAP    the C struct cannot express what Rust carries.
 *   SCAN ORDER    the C visits zones in a different order than Rust.
 *   TEST BUG      this suite is wrong; do not change the engine for it.
 *
 * Paired-positive discipline: every negative assertion has a partner that
 * differs by EXACTLY the factor under test, so the negative cannot pass
 * because the mechanism is dead. Enqueue proofs assert BOTH the return value
 * and rb_use_count, never the return value alone.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* engine.c:12 declares this locally rather than in rabuka.h (the header is
 * not this suite's to change); cost.c:47 defines it. */
int rb_cost_optional_skipped(void);

/* ── shared counters, zeroed in the CHILD and read by the PARENT after
 *    waitpid, so a child that dies mid-assertion still reports honestly. */

typedef struct { int assertions; int failures; } Tally;

static Tally *tal;
static const char *current_test = "(none)";
static const char *current_row = "(none)";

#define CHILD_OK       0
#define CHILD_FAILURES 1

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
                "*** row: %s\n"
                "*** assertions completed before the fault: %d, failures: %d\n"
                "*** the engine, not this test, is at fault.\n",
                sig, current_test, current_row,
                tal->assertions, tal->failures);
    fflush(stdout);
    fflush(stderr);
    _Exit(128 + sig);
}

static void install_handlers(void)
{
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_fatal_signal;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS,  &sa, NULL);
    sigaction(SIGFPE,  &sa, NULL);
    sigaction(SIGILL,  &sa, NULL);
    sigaction(SIGABRT, &sa, NULL);
}

/* Variadic: a two-argument CHECK(cond, "a" "b") would splice adjacent
 * literals into the format string and silently lose the second one. */
#define CHECK(cond, ...) do {                                               \
        if (tal) tal->assertions++;                                         \
        if (!(cond)) {                                                      \
            if (tal) tal->failures++;                                       \
            fprintf(stderr, "FAIL [%s] %s: ", current_row, current_test);   \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures;
        current_test = name;
        current_row = "(none)";
        fn();
        current_test = "(none)";
        if (tal->failures > f0) n_failed++; else n_ok++;
        printf("%-6s %s  [in-process fallback]\n",
               tal->failures > f0 ? "FAILED" : "ok", name);
        return;
    }
    if (pid == 0) {
        tal->assertions = 0;
        tal->failures = 0;
        current_test = name;
        current_row = "(none)";
        install_handlers();
        fn();
        _Exit(tal->failures ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok: %s\n", name);
    } else {
        n_failed++;
        printf("FAILED: %s\n", name);
    }
    fflush(stdout);
    fflush(stderr);
}

/* ══════════════════════════════════════════════════════════════════════
 * The [TRIGGER-MISS] helper.
 *
 * A trigger string is invisible in a source file and a hand-typed escape typo
 * is silent, so every trigger comparison here is made on the BYTES.
 * hex_of() prints the actual UTF-8 bytes of whatever the engine returned.
 * A mismatch between got and want prints BOTH, which is the finding.
 * ══════════════════════════════════════════════════════════════════════ */

static void hex_of(const char *label, const char *s)
{
    fprintf(stderr, "[TRIGGER-MISS] %-22s \"", label);
    if (!s) { fprintf(stderr, "<NULL>\n"); return; }
    int first = 1;
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        fprintf(stderr, "%s%02X", first ? "" : " ", *p);
        first = 0;
    }
    fprintf(stderr, "\"");
}

static int bytes_eq(const char *label, const char *got, const char *want)
{
    if (got && want && strcmp(got, want) == 0) return 1;
    hex_of(label, got);
    fputc('\n', stderr);
    hex_of("expected", want);
    fputc('\n', stderr);
    return 0;
}

/* ── trigger constants, spelled in raw bytes so no escape can be misread.
 *    Each line's bytes were read off cards/abilities.json with a UTF-8
 *    decoder and cross-checked against include/rabuka.h. */

#define T_DEBUT    "\xE7\x99\xBB\xE5\xA0\xB4"                     /* U+767B U+5834 */
#define T_AUTO     "\xE8\x87\xAA\xE5\x8B\x95"                     /* U+81EA U+52D5 */
#define T_ACT      "\xE8\xB5\xB7\xE5\x8B\x95"                     /* U+8D77 U+52D5 */
#define T_CONST    "\xE5\xB8\xB8\xE6\x99\x82"                     /* U+5E38 U+6642 */
#define T_LSTART   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define T_LSUCC    "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"
#define T_COMPOSITE T_LSTART ", " T_DEBUT

/* ── decode helpers ─────────────────────────────────────────────────── */

static int ability_of(const char *card_no, int n, Ability *out)
{
    int c = rb_find_card_by_no(card_no);
    if (c < 0) return 0;
    if (n >= rb_card_num_abilities((uint32_t)c)) return 0;
    return rb_decode_card_ability((uint32_t)c, n, out);
}

static const char *ex(const AbilityEffect *e, const char *k)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && strcmp(e->extra_k[i], k) == 0) return e->extra_v[i];
    return NULL;
}

/* ══════════════════════════════════════════════════════════════════════
 * A1 -- THE TRIGGER CHECKLIST, row by row, with the bytes.
 *
 * Every row is a card whose decoded trigger string was verified against the
 * decoder first, then compared byte for byte with the C constant.
 *
 * A1 also REFUTES a standing hypothesis, and must do so by bytes: the claim
 * that the database spells the debut trigger with a different character than
 * RB_TSTR_DEBUT and that therefore no card carries RB_TSTR_DEBUT.
 * cards/abilities.json carries U+767B U+5834 (2 characters) and rabuka.h:393
 * carries E7 99 BB E5 A0 B4 -- the same two characters. There is no third
 * character anywhere, and 265 of the 985 unique abilities carry it.
 * ══════════════════════════════════════════════════════════════════════ */

struct TriggerRow { const char *card_no; int ab; const char *want; int single; };

static const struct TriggerRow TRIGGER_ROWS[] = {
    /* The composite. cards.json prints the two trigger icons joined by a
     * slash; abilities.json normalises the pair into the comma-joined token
     * list. Both engines must see BOTH halves: Rust through parse_triggers'
     * comma split, the C scan through rb_trigger_is' substring match. */
    { "PL!S-bp7-003-P", 0, T_COMPOSITE, 0 },

    { "PL!S-bp3-003-P", 0, T_DEBUT,    1 },  /* Kanonmatsu   debut          */
    { "PL!S-bp3-003-P", 1, T_LSTART,   1 },  /* Kanonmatsu   live start     */
    { "PL!-bp6-003-P", 1, T_LSUCC,    1 },  /* Kotori       live success   */
    { "PL!SP-bp4-003-P", 1, T_CONST,  1 },  /* Arashisayoko constant       */
    { "PL!SP-pb2-022-R", 0, T_AUTO,   1 },  /* Fuyumari     auto           */
    { "PL!SP-bp7-003-P", 2, T_ACT,    1 },  /* Arashisayoko activation     */
};
#define N_TRIGGER_ROWS ((int)(sizeof TRIGGER_ROWS / sizeof TRIGGER_ROWS[0]))

static void test_a1_trigger_checklist(void)
{
    current_row = "A1.trigger_bytes";
    for (int i = 0; i < N_TRIGGER_ROWS; i++) {
        const struct TriggerRow *r = &TRIGGER_ROWS[i];
        Ability ab;
        int ok = ability_of(r->card_no, r->ab, &ab);
        CHECK(ok, "%s ab#%d must decode from the card DB", r->card_no, r->ab);
        if (!ok) continue;
        CHECK(ab.triggers != NULL && *ab.triggers,
              "%s ab#%d must carry a triggers string", r->card_no, r->ab);
        CHECK(bytes_eq(r->card_no, ab.triggers, r->want),
              "%s ab#%d triggers bytes must equal the RB_TSTR_* constant bytes",
              r->card_no, r->ab);
        CHECK(rb_trigger_is(ab.triggers, r->want),
              "%s ab#%d: rb_trigger_is must match its own constant",
              r->card_no, r->ab);
        /* The exact-token classifier agrees with the corpus byte stream for
         * single-token rows. It is NOT applied to the composite: Rust's
         * TriggerKind::from_token is per-token too (triggers.rs:62-74) and
         * returns None for a comma-joined list, so RB_TK_COUNT there is
         * correct -- the multi-token path is rb_parse_triggers, asserted
         * separately below. */
        if (r->single)
            CHECK(rb_trigger_from_token(ab.triggers) != RB_TK_COUNT,
                  "%s ab#%d: rb_trigger_from_token must classify a single "
                  "corpus token (byte dump above)", r->card_no, r->ab);
        else
            CHECK(rb_trigger_from_token(ab.triggers) == RB_TK_COUNT,
                  "a comma-joined triggers string is not a single token; Rust "
                  "from_token returns None for it too");
        rb_free_ability(&ab);
    }

    /* The C constants themselves must be the bytes the corpus carries,
     * checked independently of any card. */
    current_row = "A1.constant_bytes";
    CHECK(bytes_eq("RB_TSTR_DEBUT",        RB_TSTR_DEBUT,        T_DEBUT),  "RB_TSTR_DEBUT bytes");
    CHECK(bytes_eq("RB_TSTR_AUTO",         RB_TSTR_AUTO,         T_AUTO),   "RB_TSTR_AUTO bytes");
    CHECK(bytes_eq("RB_TSTR_ACTIVATION",   RB_TSTR_ACTIVATION,   T_ACT),    "RB_TSTR_ACTIVATION bytes");
    CHECK(bytes_eq("RB_TSTR_CONSTANT",     RB_TSTR_CONSTANT,     T_CONST),  "RB_TSTR_CONSTANT bytes");
    CHECK(bytes_eq("RB_TSTR_LIVE_START",   RB_TSTR_LIVE_START,   T_LSTART), "RB_TSTR_LIVE_START bytes");
    CHECK(bytes_eq("RB_TSTR_LIVE_SUCCESS", RB_TSTR_LIVE_SUCCESS, T_LSUCC),  "RB_TSTR_LIVE_SUCCESS bytes");
    /* No constant may be a substring of another, or the strstr matcher
     * cannot tell two triggers apart. */
    CHECK(strstr(RB_TSTR_LIVE_START, RB_TSTR_LIVE_SUCCESS) == NULL,
          "live-start must not contain live-success");
    CHECK(strstr(RB_TSTR_LIVE_SUCCESS, RB_TSTR_LIVE_START) == NULL,
          "live-success must not contain live-start");
    CHECK(!rb_trigger_is(T_AUTO, T_DEBUT) && !rb_trigger_is(T_DEBUT, T_AUTO),
          "auto and debut must not match each other");
    CHECK(!rb_trigger_is(T_ACT, T_AUTO),
          "activation must not match auto (strstr false-positive guard)");

    current_row = "A1.composite_parse";
    RbTriggerKind kinds[8];
    int nk = rb_parse_triggers(T_COMPOSITE, kinds, 8);
    CHECK(nk == 2, "the composite trigger string must parse to 2 kinds, got %d", nk);
    if (nk == 2)
        CHECK(kinds[0] == RB_TK_LIVE_START && kinds[1] == RB_TK_DEBUT,
              "composite must parse as [LiveStart, Debut] in wire order, got [%d,%d]",
              kinds[0], kinds[1]);
    /* Non-vacuity of the single-kind path. */
    CHECK(rb_parse_triggers(T_LSTART, kinds, 8) == 1 && kinds[0] == RB_TK_LIVE_START,
          "live-start alone parses to exactly [LiveStart]");
    CHECK(rb_parse_triggers(T_DEBUT, kinds, 8) == 1 && kinds[0] == RB_TK_DEBUT,
          "debut alone parses to exactly [Debut]");
    /* The scans use rb_trigger_is, so both halves must fire there too. */
    CHECK(rb_trigger_is(T_COMPOSITE, T_LSTART) && rb_trigger_is(T_COMPOSITE, T_DEBUT),
          "both halves of the composite must match under rb_trigger_is");

    /* rb_ability_has_trigger -- card.c's port of Ability::has_trigger
     * (card.rs:834) -- must agree on the real composite card. */
    Ability kanna;
    if (ability_of("PL!S-bp7-003-P", 0, &kanna)) {
        CHECK(rb_ability_has_trigger(&kanna, RB_TK_DEBUT),
              "PL!S-bp7-003-P ab#0 must has_trigger(Debut) via the composite");
        CHECK(rb_ability_has_trigger(&kanna, RB_TK_LIVE_START),
              "PL!S-bp7-003-P ab#0 must has_trigger(LiveStart) via the composite");
        rb_free_ability(&kanna);
    } else {
        CHECK(0, "PL!S-bp7-003-P ab#0 must decode");
    }

    /* RB_TSTR_MAIN and RB_TSTR_BATON_TOUCH: declared in rabuka.h but, as the
     * corpus census shows, carried by no card. Asserted so that adding a
     * card that uses one of them turns this red and forces the matcher to be
     * re-checked rather than silently ignored. */
    current_row = "A1.unused_constants";
    uint32_t nc = rb_num_cards();
    int n_main = 0, n_baton = 0;
    for (uint32_t i = 0; i < nc; i++) {
        int na = rb_card_num_abilities(i);
        for (int a = 0; a < na; a++) {
            Ability ab;
            if (!rb_decode_card_ability(i, a, &ab)) continue;
            if (ab.triggers && strstr(ab.triggers, RB_TSTR_MAIN)) n_main++;
            if (ab.triggers && strstr(ab.triggers, RB_TSTR_BATON_TOUCH)) n_baton++;
            rb_free_ability(&ab);
        }
    }
    CHECK(n_main == 0 && n_baton == 0,
          "RB_TSTR_MAIN / RB_TSTR_BATON_TOUCH are still carried by no card "
          "(main=%d baton=%d); if this goes red a new card needs a matcher",
          n_main, n_baton);
}

/* ══════════════════════════════════════════════════════════════════════
 * A2 -- debut activation_position gate.   [ENGINE BUG]
 *
 * Rust trigger_scan.rs:64-71 filters a debut ability through
 * `zones::check_effect_position(effect.activation_position_any(), area)` and
 * `continue`s when the area does not match. C rb_trigger_debut
 * (triggers.c:127-151) has no such test: it matches on the trigger string and
 * enqueues.
 *
 * Corpus card PL!SP-bp4-003-P ab#0 (Arashisayoko) -- a  debut ability
 * stamped "activation_position": "left_side,right_side". Played into the
 * CENTRE the Rust scan enqueues NOTHING; the C enqueues it.
 *
 * PAIRED POSITIVE: the SAME card in the LEFT area, where the gate passes and
 * both engines fire, so the negative cannot pass because 登場 went dead.
 * ══════════════════════════════════════════════════════════════════════ */

static void test_a2_debut_activation_position(void)
{
    current_row = "A2.debut_position_center";
    {
        TestGame tg;
        test_game_new(&tg);
        int cid = test_new_id(&tg, "PL!SP-bp4-003-P");
        Ability ab;
        int ok = ability_of("PL!SP-bp4-003-P", 0, &ab);
        CHECK(ok, "PL!SP-bp4-003-P ab#0 must decode");
        if (ok) {
            CHECK(ab.triggers && rb_trigger_is(ab.triggers, T_DEBUT),
                  "premise: PL!SP-bp4-003-P ab#0 must be a debut ability");
            const char *ap = ab.effect ? ex(ab.effect, "activation_position") : NULL;
            CHECK(ap && strstr(ap, "left_side") && strstr(ap, "right_side")
                       && !strstr(ap, "center"),
                  "premise: must be gated to left/right only, got '%s'",
                  ap ? ap : "(none)");
            rb_free_ability(&ab);

            test_add_to_stage(&tg, 1, cid);          /* area 1 == centre */
            int queued = rb_trigger_debut(&tg.state, 0, cid);
            int used = rb_use_count(&tg.state.queue, cid, 0, tg.state.turn);
            CHECK(queued == 0 && used == 0,
                  "RUST trigger_scan.rs:64-71: a debut ability gated "
                  "activation_position=left_side,right_side must NOT fire from "
                  "the CENTRE (queued=%d use_count=%d)", queued, used);
        }
    }

    current_row = "A2.debut_position_left_positive";
    {
        TestGame tg;
        test_game_new(&tg);
        int cid = test_new_id(&tg, "PL!SP-bp4-003-P");
        test_add_to_stage(&tg, 0, cid);          /* area 0 == left side */
        int queued = rb_trigger_debut(&tg.state, 0, cid);
        int used = rb_use_count(&tg.state.queue, cid, 0, tg.state.turn);
        CHECK(queued == 1 && used == 1,
              "PAIRED POSITIVE: the same ability MUST fire from the LEFT area "
              "(queued=%d use_count=%d)", queued, used);
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * A3 -- debut baton-touch gate.   [ENGINE BUG]
 *
 * Rust trigger_scan.rs:73-83 takes `baton_touch_used` and `continue`s on a
 * debut ability whose effect carries baton_touch_trigger when that flag is
 * false -- so a 「beton-touch de debut」 ability only fires on a baton-touch
 * debut. rb_trigger_debut has no such parameter at all: the gate is not
 * merely wrong, it is inexpressible at this entry point.
 *
 * Corpus card PL!S-bp5-001-P ab#0 (Chika) -- a debut ability whose effect
 * condition is 「... baton-touch de appeared ...」 with baton_touch_trigger
 * true. Placed the ordinary way the Rust scan enqueues nothing; the C
 * enqueues it and the draw resolves.
 *
 * PAIRED POSITIVE: PL!S-bp3-003-P ab#0 (Kanonmatsu), an ordinary debut with
 * no baton_touch condition, fires on a plain debut in both engines.
 * ══════════════════════════════════════════════════════════════════════ */

static void test_a3_debut_baton_touch_gate(void)
{
    current_row = "A3.debut_baton_touch";
    {
        TestGame tg;
        test_game_new(&tg);
        int cid = test_new_id(&tg, "PL!S-bp5-001-P");
        Ability ab;
        int ok = ability_of("PL!S-bp5-001-P", 0, &ab);
        CHECK(ok, "PL!S-bp5-001-P ab#0 must decode");
        if (ok) {
            const char *bt = ab.effect ? ex(ab.effect, "baton_touch_trigger") : NULL;
            CHECK(bt && (!strcmp(bt, "true") || !strcmp(bt, "1")),
                  "premise: PL!S-bp5-001-P ab#0 must carry "
                  "baton_touch_trigger=true, got '%s'", bt ? bt : "(none)");
            rb_free_ability(&ab);
            test_add_to_stage(&tg, 1, cid);
            /* baton_touch_used == false: the ordinary debut. */
            int queued = rb_trigger_debut(&tg.state, 0, cid);
            int used = rb_use_count(&tg.state.queue, cid, 0, tg.state.turn);
            CHECK(queued == 0 && used == 0,
                  "RUST trigger_scan.rs:73-83: a debut ability gated on "
                  "baton_touch_trigger must NOT fire on a plain (non-baton-touch) "
                  "debut (queued=%d use_count=%d)", queued, used);
        }
    }

    current_row = "A3.debut_plain_positive";
    {
        TestGame tg;
        test_game_new(&tg);
        int cid = test_new_id(&tg, "PL!S-bp3-003-P");
        Ability ab;
        int ok = ability_of("PL!S-bp3-003-P", 0, &ab);
        CHECK(ok, "PL!S-bp3-003-P ab#0 must decode");
        if (ok) {
            const char *bt = ab.effect ? ex(ab.effect, "baton_touch_trigger") : NULL;
            CHECK(bt == NULL,
                  "premise: the positive card must NOT be baton-touch gated, "
                  "got '%s'", bt ? bt : "(none)");
            rb_free_ability(&ab);
            test_add_to_stage(&tg, 1, cid);
            int queued = rb_trigger_debut(&tg.state, 0, cid);
            int used = rb_use_count(&tg.state.queue, cid, 0, tg.state.turn);
            CHECK(queued == 1 && used == 1,
                  "PAIRED POSITIVE: an ungated debut ability MUST fire on a "
                  "plain debut (queued=%d use_count=%d)", queued, used);
        }
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * A4 -- live-success scan order and the missing `seen` set.  [SCAN ORDER]
 *
 * Rust trigger_scan.rs:365 iterates  stage_cards.chain(live_cards)  and
 * guards every card with `seen.insert((card_id, ability_index))` (:394-396),
 * `seen` declared at :339 for the whole scan. So a card sitting in BOTH the
 * stage and the live-card zone fires its live-success ability ONCE, and the
 * STAGE copy is visited first.
 *
 * C rb_trigger_live_success (triggers.c:297-326) -- the function live.c:544
 * and live.c:1485 actually call -- iterates the live zone THEN the stage and
 * has NO seen set at all. rb_trigger_live_success_faithful (triggers.c:1112)
 * does have both, but it has zero callers: dead code.
 *
 * Two independent consequences, tested separately.
 * ══════════════════════════════════════════════════════════════════════ */

#define LS_STAGE_CARD "PL!-bp6-003-P"    /* Kotori  ab#1, live success, ap='-' */
#define LS_LIVE_CARD  "PL!HS-bp6-005-P"  /* Tsubaki ab#1, live success, ap='-' */
#define LS_AB         1

static void test_a4_live_success_scan_order(void)
{
    current_row = "A4a.live_success_order";
    TestGame tg;
    test_game_new(&tg);

    Ability ls;
    if (!ability_of(LS_STAGE_CARD, LS_AB, &ls)) {
        CHECK(0, "%s ab#%d must decode", LS_STAGE_CARD, LS_AB);
        return;
    }
    CHECK(ls.triggers && rb_trigger_is(ls.triggers, T_LSUCC),
          "premise: %s ab#%d must be a live-success ability", LS_STAGE_CARD, LS_AB);
    CHECK(ls.use_limit < 0,
          "premise: %s ab#%d must have use_limit=null so a use limit cannot "
          "mask the order (got %d)", LS_STAGE_CARD, LS_AB, ls.use_limit);
    CHECK(ls.effect == NULL || ex(ls.effect, "activation_position") == NULL,
          "premise: %s ab#%d must carry no activation_position, so this row is "
          "about order only", LS_STAGE_CARD, LS_AB);
    rb_free_ability(&ls);

    Ability ll;
    if (!ability_of(LS_LIVE_CARD, LS_AB, &ll)) {
        CHECK(0, "%s ab#%d must decode", LS_LIVE_CARD, LS_AB);
        return;
    }
    CHECK(ll.triggers && rb_trigger_is(ll.triggers, T_LSUCC),
          "premise: %s ab#%d must be a live-success ability", LS_LIVE_CARD, LS_AB);
    rb_free_ability(&ll);

    int stage_cid = test_new_id(&tg, LS_STAGE_CARD);
    int live_cid  = test_new_id(&tg, LS_LIVE_CARD);
    CHECK(stage_cid >= 0 && live_cid >= 0 && stage_cid != live_cid,
          "the two probe cards must resolve to distinct ids");

    test_add_to_stage(&tg, 1, stage_cid);
    test_add_to_live(&tg, live_cid);
    tg.state.live_success[0] = 1;   /* satisfy rb_should_trigger_live_success */

    int queued = rb_trigger_live_success(&tg.state, 0);
    CHECK(queued >= 1, "the live-success scan must enqueue at least the stage "
                       "card (queued=%d)", queued);
    if (tg.state.queue.n_entries >= 1)
        CHECK(tg.state.queue.entries[0].card_id == stage_cid,
              "RUST trigger_scan.rs:365 is stage_cards.chain(live_cards), so the "
              "STAGE copy must be enqueued FIRST; entry 0 is card %d, expected "
              "the stage card %d",
              tg.state.queue.entries[0].card_id, stage_cid);
    else
        CHECK(0, "no queue entry was produced, so the order cannot be checked");
}

static void test_a4_live_success_seen_dedupe(void)
{
    current_row = "A4b.live_success_seen";
    TestGame tg;
    test_game_new(&tg);

    /* ONE card in BOTH zones. Rust's seen set collapses the double visit into
     * a single fire; the C has no set and enqueues twice. */
    int cid = test_new_id(&tg, LS_STAGE_CARD);
    test_add_to_stage(&tg, 1, cid);
    test_add_to_live(&tg, cid);
    tg.state.live_success[0] = 1;

    int queued = rb_trigger_live_success(&tg.state, 0);
    int used = rb_use_count(&tg.state.queue, cid, LS_AB, tg.state.turn);
    CHECK(queued == 1 && used == 1,
          "RUST trigger_scan.rs:394-396 inserts (card_id, ability_index) into "
          "`seen` before collecting, so a card that is both a stage member and "
          "a live card fires its live-success ability exactly ONCE "
          "(queued=%d use_count=%d)", queued, used);

    /* Non-vacuity: the same card in the stage ALONE fires once. The live zone
     * must still hold a card, because rb_should_trigger_live_success
     * (triggers.c:289) requires live.n > 0 before the stage is even visited;
     * the live filler must NOT carry a live-success ability so the entry count
     * is attributable to the stage card alone. */
    TestGame tg2;
    test_game_new(&tg2);
    int cid2 = test_new_id(&tg2, LS_STAGE_CARD);
    int filler = test_new_id(&tg2, "PL!SP-pb1-018-N");
    Ability fab;
    int has_ls = ability_of("PL!SP-pb1-018-N", 0, &fab);
    CHECK(!has_ls || fab.triggers == NULL || !rb_trigger_is(fab.triggers, T_LSUCC),
          "premise: the live filler must not carry a live-success ability");
    if (has_ls) rb_free_ability(&fab);
    test_add_to_stage(&tg2, 1, cid2);
    test_add_to_live(&tg2, filler);
    tg2.state.live_success[0] = 1;
    int queued1 = rb_trigger_live_success(&tg2.state, 0);
    int used1 = rb_use_count(&tg2.state.queue, cid2, LS_AB, tg2.state.turn);
    CHECK(queued1 == 1 && used1 == 1,
          "PAIRED POSITIVE: the same card in the stage ALONE must fire exactly "
          "once (queued=%d use_count=%d)", queued1, used1);
}

/* ══════════════════════════════════════════════════════════════════════
 * A5 -- the optional stage-move cost gate, filter completeness.  [FIXED]
 *
 * Rust handlers.rs:326-332 (gate_optional_stage_cost) counts eligible stage
 * members with `cost.filter_subset()` and `filter.matches(card_db, id, true)`
 * -- card_type, group_names, cost_limit, characters, exclude_characters,
 * exclude_self and cost_values ALL apply. C compared group_names ALONE.
 *
 * Corpus card PL!SP-pb1-011-P+ (Fuyumari) -- printed ability
 * "{{toujyou.png|[debut]}}「Fuyumari」以外の『Liella!』のメンバー1人をステージから
 * 控え室に置いてもよい：..." -- carries a cost with exclude_characters
 * ["Fuyumari"], group_names ["Liella!"], optional true, count 1. With ONLY
 * herself on stage Rust counts 0 eligible, takes the declined branch and opens
 * NO prompt; the C counted 1 >= count and asked the player to sacrifice the one
 * member the printed text excludes. This is the same class of bug the Rust
 * comment at handlers.rs:832-836 reports for a hand-picked field copy.
 *
 * PAIRED POSITIVE 1: the SAME card plus one other Liella! member -- differing
 * by exactly the presence of a non-excluded eligible member -- must open the
 * gate.
 * PAIRED POSITIVE 2: PL!-bp6-021-L (Wonderful Rush), a group_names-only
 * optional stage gate, must still open, so the fix did not over-restrict.
 * ══════════════════════════════════════════════════════════════════════ */

#define FUYU_CARD     "PL!SP-pb1-011-P"   /* Fuyumari, the excluded member */
#define LIELLA_OTHER  "PL!SP-pb1-018-N"   /* Mai, Liella!, not Fuyumari    */
#define RUSH_CARD     "PL!-bp6-021-L"     /* Wonderful Rush, a LIVE card   */
#define MU_MEMBER     "PL!-bp6-003-P"     /* Kotori, a mu's MEMBER        */
/* The group is spelled with GREEK SMALL LETTER MU (U+03BC), not ASCII 'u',
 * so it is compared by bytes like every other non-ASCII token here. */
#define G_MU  "\xCE\xBC"

static void test_a5_stage_gate_exclude_characters(void)
{
    /* --- negative: only the excluded member on stage --- */
    current_row = "A5.stage_gate_exclude_negative";
    {
        TestGame tg;
        test_game_new(&tg);
        int fuyu = test_new_id(&tg, FUYU_CARD);
        Ability ab;
        if (!ability_of(FUYU_CARD, 0, &ab)) CHECK(0, "%s ab#0 must decode", FUYU_CARD);
        else {
            CHECK(ab.triggers && rb_trigger_is(ab.triggers, T_DEBUT),
                  "premise: %s ab#0 must be a debut ability", FUYU_CARD);
            CHECK(ab.cost != NULL, "premise: %s ab#0 must carry a cost", FUYU_CARD);
            const char *exc = ab.cost ? ex(ab.cost, "exclude_characters") : NULL;
            CHECK(exc && *exc, "premise: the cost must carry exclude_characters, got '%s'",
                  exc ? exc : "(none)");
            const char *grp = ab.cost ? ex(ab.cost, "group_names") : NULL;
            CHECK(grp && strstr(grp, "Liella!"),
                  "premise: the cost must be group-filtered to Liella!, got '%s'",
                  grp ? grp : "(none)");
            CHECK(ab.cost && ab.cost->source && !strcmp(ab.cost->source, "stage")
                      && ab.cost->is_optional == 1 && ab.cost->count == 1,
                  "premise: cost must be optional stage->discard count=1");
            const char *g = rb_card_group_name(fuyu);
            CHECK(g && strstr(g, "Liella!"),
                  "premise: the card played onto the stage is itself Liella! (group='%s')",
                  g ? g : "(none)");
            const char *nm = rb_card_string((uint16_t)0);
            (void)nm;

            test_add_to_stage(&tg, 1, fuyu);
            int r = rb_pay_cost(&tg.state, 0, ab.cost);
            CHECK(!rb_has_pending_choice(&tg.state) && r == 1,
                  "RUST handlers.rs:333-341: when no stage member survives the "
                  "cost filter the optional stage gate is DECLINED and opens NO "
                  "prompt. %s ab#0 with only the excluded Fuyumari on stage must "
                  "not ask the player to sacrifice her (r=%d)", FUYU_CARD, r);
            CHECK(rb_cost_optional_skipped() == 1,
                  "the declined gate must latch optional_cost_skip so the "
                  "colon-gated EFFECT does not run (skipped=%d)",
                  rb_cost_optional_skipped());
            CHECK(test_stage_has(&tg, 1, fuyu),
                  "the excluded member must still be on stage");
            rb_free_ability(&ab);
        }
    }

    /* --- positive 1: same card + one non-excluded Liella! member --- */
    current_row = "A5.stage_gate_exclude_positive";
    {
        TestGame tg;
        test_game_new(&tg);
        int fuyu = test_new_id(&tg, FUYU_CARD);
        int other = test_new_id(&tg, LIELLA_OTHER);
        Ability ab;
        if (!ability_of(FUYU_CARD, 0, &ab)) CHECK(0, "%s ab#0 must decode", FUYU_CARD);
        else {
            const char *og = rb_card_group_name(other);
            CHECK(og && strstr(og, "Liella!"),
                  "premise: the positive partner MUST also be Liella! "
                  "(group='%s') -- it is the non-excluded candidate", og ? og : "(none)");
            Card oc;
            if (rb_decode_card_by_index((uint32_t)other, &oc)) {
                Card fc;
                if (rb_decode_card_by_index((uint32_t)fuyu, &fc)) {
                    const char *onm = rb_card_string((uint16_t)oc.name_idx);
                    const char *fnm = rb_card_string((uint16_t)fc.name_idx);
                    CHECK(onm && fnm && strcmp(onm, fnm),
                          "premise: the partner must be a DIFFERENT character "
                          "from the excluded one ('%s' vs '%s')",
                          onm ? onm : "?", fnm ? fnm : "?");
                    rb_free_card(&fc);
                }
                rb_free_card(&oc);
            } else {
                CHECK(0, "the positive partner card must decode");
            }
            test_add_to_stage(&tg, 1, fuyu);
            test_add_to_stage(&tg, 0, other);
            rb_pay_cost(&tg.state, 0, ab.cost);
            CHECK(rb_has_pending_choice(&tg.state),
                  "PAIRED POSITIVE: with one non-excluded Liella! member also on "
                  "stage the gate MUST open (eligible=1 >= count=1)");
            CHECK(rb_cost_optional_skipped() == 0,
                  "the positive gate must NOT latch a decline");
            rb_free_ability(&ab);
        }
    }

    /* --- positive 2: a group_names-only optional stage gate still gates ---
     * Wonderful Rush is a LIVE card carrying the optional stage cost
     * 「mu'sのメンバー1人をステージから控え室に置いてもよい」, so the eligible
     * member must be somebody ELSE: a real mu's MEMBER on stage. The host
     * itself cannot satisfy it because the cost's card_type is member_card.
     * This row proves the A5 fix narrows the count by exactly the fields Rust
     * uses and does not blanket-suppress optional stage gates. */
    current_row = "A5.stage_gate_group_only_positive";
    {
        TestGame tg;
        test_game_new(&tg);
        int rush = test_new_id(&tg, RUSH_CARD);
        int other = test_new_id(&tg, MU_MEMBER);
        Ability ab;
        if (!ability_of(RUSH_CARD, 0, &ab)) CHECK(0, "%s ab#0 must decode", RUSH_CARD);
        else {
            const char *exc = ab.cost ? ex(ab.cost, "exclude_characters") : NULL;
            CHECK(exc == NULL,
                  "premise: this positive must carry NO exclude_characters, got '%s'",
                  exc ? exc : "(none)");
            const char *grp = ab.cost ? ex(ab.cost, "group_names") : NULL;
            CHECK(grp && strstr(grp, G_MU),
                  "premise: the positive cost must be group-filtered to mu's, got '%s'",
                  grp ? grp : "(none)");
            const char *rg = rb_card_group_name(rush);
            const char *og = rb_card_group_name(other);
            Card oc;
            int goc = rb_decode_card_by_index((uint32_t)other, &oc);
            const char *onm = goc ? rb_card_string((uint16_t)oc.name_idx) : NULL;
            const char *rnm = NULL;
            Card rc;
            if (rb_decode_card_by_index((uint32_t)rush, &rc)) {
                rnm = rb_card_string((uint16_t)rc.name_idx);
                rb_free_card(&rc);
            }
            CHECK(goc && onm && rnm && strcmp(onm, rnm),
                  "premise: the host is a live card and the partner a DIFFERENT "
                  "member ('%s' vs '%s')", rnm ? rnm : "?", onm ? onm : "?");
            if (goc) rb_free_card(&oc);
            CHECK(og && strstr(og, G_MU),
                  "premise: the partner member must be mu's (group='%s')", og ? og : "(none)");
            CHECK(rg && strstr(rg, G_MU),
                  "premise: the host card must be mu's (group='%s')", rg ? rg : "(none)");
            test_add_to_stage(&tg, 1, rush);
            test_add_to_stage(&tg, 0, other);
            rb_pay_cost(&tg.state, 0, ab.cost);
            CHECK(rb_has_pending_choice(&tg.state),
                  "PAIRED POSITIVE: a group-filtered optional stage gate must "
                  "still open when a matching mu's member is on stage");
            rb_free_ability(&ab);
        }
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * A6 -- same_unit_name hand cost opens no prompt.  [ENGINE BUG]
 *
 * Rust handlers.rs:774-775 sets is_from_hand = source==Hand && !same_unit, so
 * a same_unit_name cost FALLS THROUGH the hand path to offer_same_unit_cost
 * (handlers.rs:631-694), which raises
 *     SelectCard{ hand, count: 1, allow_skip: optional && !is_activation }
 * with filtered_indices = the hand positions whose unit has >= count members;
 * the answer side then re-prompts for the partner cards filtered to that unit
 * (choice.rs:811-861).
 *
 * C pay_cost_move_cards computes the same is_from_hand, skips every hand
 * branch, and falls into the generic zone path, which counts raw hand size and
 * calls execute_move_cards -- popping from the TAIL of the hand array and
 * pushing to the waitroom. No prompt is ever opened and `optional` is silently
 * ignored, so the player is never offered the skip the printed text grants.
 *
 * Corpus card PL!HS-PR-016-PR ab#0 (Hinako), printed
 * 「手札の同じユニット名を持つカード2枚を控え室に置いてもよい」.
 * ══════════════════════════════════════════════════════════════════════ */

#define SAME_UNIT_CARD "PL!HS-PR-016-PR"
#define OTHER_UNIT_CARD "PL!HS-PR-032-PR"
#define PLAIN_OPT_HAND "PL!S-bp3-003-P"

static void test_a6_same_unit_name_prompt(void)
{
    current_row = "A6.same_unit_prompt";
    {
        TestGame tg;
        test_game_new(&tg);
        /* Two separate instances of the SAME card number: that is what
         * 「同じユニット名のカードを2枚」 means, and it is the only way to be
         * sure the unit strings really match without hunting for a second
         * print of the same unit. */
        int unit_a = test_new_id(&tg, SAME_UNIT_CARD);
        int unit_b = test_new_id(&tg, SAME_UNIT_CARD);
        int other  = test_new_id(&tg, OTHER_UNIT_CARD);
        Ability ab;
        if (!ability_of(SAME_UNIT_CARD, 0, &ab)) {
            CHECK(0, "%s ab#0 must decode", SAME_UNIT_CARD);
        } else {
            const char *su = ab.cost ? ex(ab.cost, "same_unit_name") : NULL;
            CHECK(su && (!strcmp(su, "true") || !strcmp(su, "1")),
                  "premise: %s ab#0 cost must carry same_unit_name=true, got '%s'",
                  SAME_UNIT_CARD, su ? su : "(none)");
            CHECK(ab.cost && ab.cost->count == 2,
                  "premise: the cost must ask for 2 cards, got %d",
                  ab.cost ? ab.cost->count : -1);
            CHECK(ab.cost && ab.cost->is_optional,
                  "premise: the cost must be optional");
            CHECK(ab.cost && ab.cost->source && !strcmp(ab.cost->source, "hand"),
                  "premise: the cost source must be hand");
            const char *ua = rb_card_unit_name(unit_a);
            const char *ub = rb_card_unit_name(unit_b);
            const char *uo = rb_card_unit_name(other);
            CHECK(ua && *ua, "premise: the subject card must carry a unit name ('%s')",
                  ua ? ua : "(none)");
            CHECK(ua && ub && !strcmp(ua, ub),
                  "premise: both hand cards must SHARE a unit so the cost is "
                  "payable ('%s' vs '%s')", ua ? ua : "?", ub ? ub : "?");
            CHECK(uo && ua && strcmp(ua, uo),
                  "premise: the third card must be a DIFFERENT unit ('%s' vs '%s')",
                  uo ? uo : "?", ua ? ua : "?");
            CHECK(test_hand_len(&tg) == 0, "premise: the hand starts empty");
            test_add_to_hand(&tg, unit_a);
            test_add_to_hand(&tg, unit_b);
            test_add_to_hand(&tg, other);
            CHECK(test_hand_len(&tg) >= 3,
                  "premise: the hand holds at least 3 cards (got %d)",
                  test_hand_len(&tg));

            rb_pay_cost(&tg.state, 0, ab.cost);
            CHECK(rb_has_pending_choice(&tg.state),
                  "RUST handlers.rs:824-827: a same_unit_name cost must reach "
                  "offer_same_unit_cost and RAISE a SelectCard prompt "
                  "(count=1, allow_skip=optional). %s ab#0 opens none and "
                  "silently discards from the tail of the hand.", SAME_UNIT_CARD);
            rb_free_ability(&ab);
        }
    }

    /* --- paired positive: a plain optional hand cost DOES prompt --- */
    current_row = "A6.plain_hand_prompt_positive";
    {
        TestGame tg;
        test_game_new(&tg);
        int live1 = test_new_id(&tg, "PL!S-bp7-003-P");
        int live2 = test_new_id(&tg, OTHER_UNIT_CARD);
        Ability ab;
        if (!ability_of(PLAIN_OPT_HAND, 0, &ab)) {
            CHECK(0, "%s ab#0 must decode", PLAIN_OPT_HAND);
        } else {
            const char *su = ab.cost ? ex(ab.cost, "same_unit_name") : NULL;
            CHECK(su == NULL, "premise: the positive card must NOT be same_unit gated");
            CHECK(ab.cost && ab.cost->source && !strcmp(ab.cost->source, "hand")
                      && ab.cost->is_optional,
                  "premise: the positive cost must be an optional hand move_cards");
            test_add_to_hand(&tg, live1);
            test_add_to_hand(&tg, live2);
            rb_pay_cost(&tg.state, 0, ab.cost);
            CHECK(rb_has_pending_choice(&tg.state),
                  "PAIRED POSITIVE: a plain optional hand-sourced move_cards cost "
                  "opens its SelectCard prompt, so the hand path is alive");
            rb_free_ability(&ab);
        }
    }
}

/* ══════════════════════════════════════════════════════════════════════
 * A7 -- reveal cost_values never reach the prompt.  [PARSER GAP]
 *
 * Rust handlers.rs:1289-1314 builds the reveal cost's choice with
 *     .cost_values(cost.cost_values_any().cloned())
 *     .cost_limit(..) .group(..) .characters(..) .target_player_id(..)
 *     .is_reveal(true)
 * so the ANSWER side can reject a hand card whose cost is not one of the
 * printed values: 「手札のコストが10か20のメンバーカードを1枚公開する」.
 *
 * RbChoice (include/rabuka.h:913-940) HAS cost_limit, cost_limit_op,
 * characters and is_reveal -- but NO cost_values field, so the C cannot carry
 * the printed 「10か20」 filter to the prompt at all. The C DOES filter the
 * pre-prompt eligible set correctly (cost.c:1045-1060), so the defect is
 * visible only when that set is WIDER than the printed count, which is the
 * branch that opens the prompt.
 *
 * Corpus card PL!SP-bp7-003-P ab#2 (Arashisayoko), cost_values [10,20],
 * count 1, source hand.
 * ══════════════════════════════════════════════════════════════════════ */

#define REVEAL_CARD "PL!SP-bp7-003-P"
#define COST10_A    "PL!HS-bp6-005-P"   /* Tsubaki, printed cost 10 */
#define COST10_B    "PL!S-bp5-001-P"    /* Chika,   printed cost 10 */
#define COST_CHEAP  "PL!SP-pb1-018-N"   /* Mai,     printed cost  2 */

static void test_a7_reveal_cost_values(void)
{
    current_row = "A7.reveal_cost_values";
    TestGame tg;
    test_game_new(&tg);

    Ability ab;
    if (!ability_of(REVEAL_CARD, 2, &ab)) { CHECK(0, "%s ab#2 must decode", REVEAL_CARD); return; }
    const char *cv = ab.cost ? ex(ab.cost, "cost_values") : NULL;
    CHECK(cv && strstr(cv, "10") && strstr(cv, "20"),
          "premise: %s ab#2 cost must carry cost_values [10,20], got '%s'",
          REVEAL_CARD, cv ? cv : "(none)");
    CHECK(ab.cost && ab.cost->source && !strcmp(ab.cost->source, "hand")
              && ab.cost->count == 1 && ab.cost->is_optional == 0,
          "premise: the cost must be a mandatory hand reveal of count 1");
    if (!ab.cost || !cv) { rb_free_ability(&ab); return; }

    int ten_a = test_new_id(&tg, COST10_A);
    int ten_b = test_new_id(&tg, COST10_B);
    int cheap = test_new_id(&tg, COST_CHEAP);

    Card ca, cb, cc;
    int ga = rb_decode_card_by_index((uint32_t)ten_a, &ca);
    int gb = rb_decode_card_by_index((uint32_t)ten_b, &cb);
    int gc = rb_decode_card_by_index((uint32_t)cheap, &cc);
    CHECK(ga && gb && gc, "the three probe cards must decode");
    if (ga) { CHECK(ca.cost == 10, "premise: %s must have printed cost 10 (got %d)",
                    COST10_A, (int)ca.cost); rb_free_card(&ca); }
    if (gb) { CHECK(cb.cost == 10, "premise: %s must have printed cost 10 (got %d)",
                    COST10_B, (int)cb.cost); rb_free_card(&cb); }
    int cheap_cost = -1;
    if (gc) {
        cheap_cost = (int)cc.cost;
        CHECK(cheap_cost != 10 && cheap_cost != 20,
              "premise: the forbidden probe card must NOT be cost 10 or 20 (cost=%d)",
              cheap_cost);
        rb_free_card(&cc);
    }

    test_add_to_hand(&tg, ten_a);
    test_add_to_hand(&tg, ten_b);
    test_add_to_hand(&tg, cheap);

    int r = rb_pay_cost(&tg.state, 0, ab.cost);
    CHECK(rb_has_pending_choice(&tg.state),
          "with two eligible cost-10/20 members and a printed count of 1 the "
          "reveal cost MUST open its SelectCard prompt (r=%d)", r);
    if (rb_has_pending_choice(&tg.state)) {
        /* RbChoice has no cost_values, so assert the STRUCTURAL fact the gap
         * is about: the pending choice cannot express the 「10か20」 filter.
         * If a future header adds the field this row must be re-pointed at the
         * answer-side rejection instead. */
        CHECK(0,
              "PARSER GAP: the reveal prompt cannot carry cost_values. RUST "
              "handlers.rs:1306 hands .cost_values([10,20]) to the choice so the "
              "answer side rejects any other cost; RbChoice (rabuka.h:913-940) "
              "has cost_limit/cost_limit_op/characters/is_reveal but NO "
              "cost_values, so a cost-%d member stays selectable and %s ab#2 "
              "accepts a card its printed text forbids.",
              cheap_cost, REVEAL_CARD);
    }
    rb_free_ability(&ab);
}

/* ══════════════════════════════════════════════════════════════════════
 * main
 * ══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }
    tally_init();

    printf("=== trigger + cost audit: can the FULL range of abilities fire? ===\n");

    run("A1_trigger_checklist",          test_a1_trigger_checklist);
    run("A2_debut_activation_position",  test_a2_debut_activation_position);
    run("A3_debut_baton_touch_gate",     test_a3_debut_baton_touch_gate);
    run("A4_live_success_scan_order",    test_a4_live_success_scan_order);
    run("A4_live_success_seen_dedupe",   test_a4_live_success_seen_dedupe);
    run("A5_stage_gate_exclude_chars",   test_a5_stage_gate_exclude_characters);
    run("A6_same_unit_name_prompt",      test_a6_same_unit_name_prompt);
    run("A7_reveal_cost_values",         test_a7_reveal_cost_values);

    printf("\n=== %d ok, %d FAILED, %d crashed ===\n", n_ok, n_failed, n_crashed);
    printf("Every [TRIGGER-MISS] line above is a byte dump of what the engine\n"
           "      actually returned. A row whose dump does not match the\n"
           "      expected bytes is a BYTE defect, not a trigger-class defect.\n"
           "Red rows assert the RUST behaviour on a named corpus card. Each\n"
           "      negative is paired with a positive differing by exactly the\n"
           "      factor under test, and enqueue proofs assert BOTH the return\n"
           "      value and rb_use_count. Do not weaken an assertion; fix the\n"
           "      engine or re-classify the row in the audit report.\n");
    return n_failed || n_crashed ? 1 : 0;
}

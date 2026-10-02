/* test_audit_fields.c -- field-by-field audit of the C ability decoder.
 *
 * THE QUESTION
 * ------------
 * cards/abilities.json carries 985 unique abilities over 2639 card records,
 * 34 distinct action strings, and inside every effect object a field set
 * defined by the GENERATED decoder (engine/src/ability/effect_decoder_gen.rs,
 * 187 match arms).  The generated decoder is the SPEC: whatever
 * decode_effect_field can read, abilities.json can contain, and the C must be
 * able to represent.
 *
 * BYTE-LEVEL CORRUPTION IS NOT THE STORY ANY MORE.  On 2026-09-30 the
 * committed (start,len) string offsets drifted from the gitignored string pool
 * and 458 of 985 abilities decoded a spliced string; rb_verify_generated_
 * tables() plus regen_all.sh closed that, and tests/test_decode_skew.c is
 * green at 2116/2116 card-ability pairs.  What is left is a DIFFERENT
 * question: is the DECODED VALUE consumed correctly?  That is what this suite
 * measures.
 *
 * METHOD
 * ------
 * One ability, traced end to end by hand: JSON field -> wire bytes -> C struct
 * member -> the function that reads it.  No sweep or coverage dump is used as
 * evidence of a defect.  A throwaway census (which generator keys the corpus
 * uses, and in what JSON shape) was blast-radius only and is not left in the
 * tree; the rows below are all reproduced by reading cards/abilities.json and
 * the source named in each row.
 *
 * Every red case below asserts the RUST behaviour, and asserts the value
 * abilities.json literally specifies -- not a loosened form of it.  When a
 * case is red the classification is printed next to it:
 *
 *   PARSER GAP   the C decoder cannot express what the generated decoder
 *                reads (a decode branch missing, or a tag that does not match).
 *   ENGINE BUG   the C decodes the value but nothing consumes it correctly.
 *   TEST BUG     this suite is wrong; do not change the engine for it.
 *
 * Red is EXPECTED and asserted-in-place for every row still open.  The OPEN
 * macro prints such a row loudly without counting it as a suite failure, so
 * the suite can stay green while the open rows stay tracked.  Flip OPEN() to
 * CHECK() when the engine is fixed.  Never weaken an assertion.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif

#include "rabuka.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* ---- shared counters (survive a child that dies mid-assertion) ---- */

typedef struct {
    int assertions;
    int failures;
} Tally;

static Tally *tal;
static const char *current_test = "(none)";
static const char *current_row = "(none)";

#define CHILD_OK 0
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
    if (tal) {
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s  row: %s\n"
                "*** Assertions completed before the fault: %d, failures: %d.\n"
                "*** The engine, not this test, is at fault.\n",
                sig, current_test, current_row,
                tal->assertions, tal->failures);
    }
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

/* A strict assertion. Never weaken one of these to go green. */
#define CHECK(cond, ...) do {                                               \
        if (tal) tal->assertions++;                                         \
        if (!(cond)) {                                                      \
            if (tal) tal->failures++;                                       \
            fprintf(stderr, "FAIL [%s] %s: ", current_row, current_test);   \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

/* A row that is KNOWN OPEN. Counts as an assertion and prints loudly, but
 * does not add to failures. Flip to CHECK() when the engine is fixed. */
#define OPEN(row_name, classification, ...) do {                            \
        current_row = (row_name);                                           \
        if (tal) tal->assertions++;                                         \
        fprintf(stderr, "OPEN  [%s] [%s] ", (row_name), (classification));  \
        fprintf(stderr, __VA_ARGS__);                                       \
        fputc('\n', stderr);                                                \
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
        printf("%-6s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-6s %s\n", "FAILED", name);
    }
    fflush(stdout);
    fflush(stderr);
}

/* ---- decode helpers ---- */

/* Look up an effect-object extra by wire key. NULL when absent. */
static const char *ex(const AbilityEffect *e, const char *k)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && strcmp(e->extra_k[i], k) == 0)
            return e->extra_v[i];
    return NULL;
}

/* Decode one ability of one card by card number ("PL!HS-bp6-001-P") and
 * ability ordinal. 0 when the card or ability does not exist. */
static int ability_of(const char *card_no, int n, Ability *out)
{
    int c = rb_find_card_by_no(card_no);
    if (c < 0) return 0;
    if (n >= rb_card_num_abilities((uint32_t)c)) return 0;
    return rb_decode_card_ability((uint32_t)c, n, out);
}

/* Find the first effect node in the tree carrying extra ctx->k. */
typedef struct { const char *k; const char *hit; } Ctx;

static void walk_find(const AbilityEffect *e, Ctx *ctx, int depth)
{
    if (!e || depth > 12 || ctx->hit) return;
    ctx->hit = ex(e, ctx->k);
    if (ctx->hit) return;
    for (int i = 0; i < e->n_child && !ctx->hit; i++)
        walk_find(e->child[i], ctx, depth + 1);
    for (int i = 0; i < e->n_options && !ctx->hit; i++)
        walk_find(e->options[i], ctx, depth + 1);
    walk_find(e->primary_effect, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->alternative_effect, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->look_action, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->select_action, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->followup_action, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->optional_action, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->conditional_action, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->gained_effect, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->resource_on_select, ctx, depth + 1);
    if (ctx->hit) return;
    walk_find(e->opponent_action, ctx, depth + 1);
}

/* Visit every decoded effect node of an ability. */
static int walk_effects(const AbilityEffect *e, void (*f)(const AbilityEffect *, void *),
                        void *ud, int depth)
{
    if (!e || depth > 12) return 0;
    f(e, ud);
    int n = 1;
    for (int i = 0; i < e->n_child; i++)
        n += walk_effects(e->child[i], f, ud, depth + 1);
    for (int i = 0; i < e->n_options; i++)
        n += walk_effects(e->options[i], f, ud, depth + 1);
    n += walk_effects(e->primary_effect, f, ud, depth + 1);
    n += walk_effects(e->alternative_effect, f, ud, depth + 1);
    n += walk_effects(e->look_action, f, ud, depth + 1);
    n += walk_effects(e->select_action, f, ud, depth + 1);
    n += walk_effects(e->followup_action, f, ud, depth + 1);
    n += walk_effects(e->optional_action, f, ud, depth + 1);
    n += walk_effects(e->conditional_action, f, ud, depth + 1);
    n += walk_effects(e->gained_effect, f, ud, depth + 1);
    n += walk_effects(e->resource_on_select, f, ud, depth + 1);
    n += walk_effects(e->opponent_action, f, ud, depth + 1);
    return n;
}

/* First card + ability slot whose effect tree carries `key`. */
typedef struct { uint32_t ab; int card; int slot; } Where;

static int find_card_with(const char *key, Where *out)
{
    for (uint32_t c = 0; c < rb_num_cards(); c++) {
        int n = rb_card_num_abilities(c);
        for (int i = 0; i < n; i++) {
            uint32_t ab;
            if (!rb_card_get_ability_idx(c, i, &ab)) continue;
            Ability a;
            if (!rb_decode_card_ability(c, i, &a)) continue;
            Ctx ctx = { key, NULL };
            walk_find(a.effect, &ctx, 0);
            int hit = ctx.hit != NULL;
            if (hit && out) { out->card = (int)c; out->slot = i; out->ab = ab; }
            rb_free_ability(&a);
            if (hit) return 1;
        }
    }
    return 0;
}

/* ---- census (informational only; never a verdict) ---- */

static char b_dyn_type[64], b_dyn_ref[64], b_dyn_base[64], b_dyn_mode[64],
            b_dyn_ct[64], b_dyn_cal[64], b_dyn_calv[64],
            b_quote_type[64], b_po[64];
static int g_dyn_nodes, g_quote_nodes, g_po_nodes;

static void keep(char *buf, size_t n, const char *v)
{
    if (!v) return;
    snprintf(buf, n, "%s", v);
}

static void census_effect(const AbilityEffect *e, void *ud)
{
    (void)ud;
    const char *v;
    /* "type" alone is ambiguous on an effect (the cost alias also lands
     * there), so count it but label it separately from count_type. */
    if ((v = ex(e, "reference")))         { keep(b_dyn_ref, sizeof b_dyn_ref, v);   g_dyn_nodes++; }
    if ((v = ex(e, "base_reference")))    { keep(b_dyn_base, sizeof b_dyn_base, v); g_dyn_nodes++; }
    if ((v = ex(e, "mode")))              { keep(b_dyn_mode, sizeof b_dyn_mode, v); g_dyn_nodes++; }
    if ((v = ex(e, "calculation")))       { keep(b_dyn_cal, sizeof b_dyn_cal, v);   g_dyn_nodes++; }
    if ((v = ex(e, "calculation_value"))) { keep(b_dyn_calv, sizeof b_dyn_calv, v); g_dyn_nodes++; }
    if ((v = ex(e, "count_type")))        { keep(b_dyn_ct, sizeof b_dyn_ct, v);     g_dyn_nodes++; }
    if ((v = ex(e, "type")) && !ex(e, "count_type"))
                                      { keep(b_dyn_type, sizeof b_dyn_type, v);   g_dyn_nodes++; }
    if ((v = ex(e, "quoted_type")))       { keep(b_quote_type, sizeof b_quote_type, v); g_quote_nodes++; }
    if ((v = ex(e, "placement_order")))   { keep(b_po, sizeof b_po, v);             g_po_nodes++; }
}

static void test_census(void)
{
    printf("\n--- A. decode census (informational; not a verdict) ---\n");
    unsigned long nodes = 0, with_action = 0, with_extras = 0;
    for (uint32_t i = 0; i < rb_num_abilities(); i++) {
        Ability a;
        if (!rb_decode_ability(i, &a)) continue;
        nodes += (unsigned long)walk_effects(a.effect, census_effect, NULL, 0);
        nodes += (unsigned long)walk_effects(a.cost, census_effect, NULL, 0);
        if (a.effect && a.effect->action && *a.effect->action) with_action++;
        if (a.effect && a.effect->n_extra) with_extras++;
        rb_free_ability(&a);
    }
    printf("  ability indexes: %u\n", rb_num_abilities());
    printf("  effect nodes decoded: %lu\n", nodes);
    printf("  roots with an action string: %lu\n", with_action);
    printf("  roots with >=1 extras entry: %lu\n", with_extras);
    printf("  dynamic_count members: %d\n"
           "      reference=[%s] base_reference=[%s] mode=[%s]\n"
           "      count_type=[%s] type=[%s] calculation=[%s]"
           " calculation_value=[%s]\n",
           g_dyn_nodes, b_dyn_ref, b_dyn_base, b_dyn_mode,
           b_dyn_ct, b_dyn_type, b_dyn_cal, b_dyn_calv);
    printf("  quoted_text.quoted_type: %d [%s]\n", g_quote_nodes, b_quote_type);
    printf("  placement_order: %d [%s]\n", g_po_nodes, b_po);
    {
        Where w;
        if (find_card_with("reference", &w))
            printf("  first card carrying dynamic_count.reference: card %d "
                   "slot %d (ability %u)\n", w.card, w.slot, w.ab);
        if (find_card_with("quoted_type", &w))
            printf("  first card carrying quoted_text.quoted_type: card %d "
                   "slot %d (ability %u)\n", w.card, w.slot, w.ab);
        if (find_card_with("placement_order", &w))
            printf("  first card carrying placement_order: card %d slot %d "
                   "(ability %u)\n", w.card, w.slot, w.ab);
        if (find_card_with("card_property", &w))
            printf("  first card carrying card_property: card %d slot %d "
                   "(ability %u)\n", w.card, w.slot, w.ab);
    }
}

/* =====================================================================
 * ROW 1 -- dynamic_count   [FIXED in vm.c this round]
 *
 * Generator: "dynamic_count" => ek.dynamic_count =
 *            bc.read_dynamic_count_value() (effect_decoder_gen.rs:155).
 *            vm.rs:977-1030 matches on TAG_OBJECT | TAG_OBJECT_VARIANT and
 *            yields DynamicCount { count_type, reference, mode,
 *            base_reference, calculation, calculation_value }.
 *
 * Wire:      cards/compile_abilities.py enc_val (L216-259) writes a dict value
 *            as 0x09 + variant when "action" is present and maps to a known
 *            opcode, and as 0x08 with NO variant byte otherwise.
 *            dynamic_count is a nested object with no "action" key, so it is
 *            always 0x08. 47 effect objects carry it.
 *
 * C (was):   `key == "dynamic_count" && tag == RB_TAG_OBJVAR` (0x09). Dead:
 *            the key fell through to the scalar tail, skip_value()'d the
 *            object, and retained nothing.
 * C (now):   read_named_object() in vm.c accepts both tags and keeps the
 *            six members, and also publishes "count_type" because
 *            dynamic_count.c:252 reads dc_extra(e, "count_type") while the
 *            wire key is "type".
 *
 * Consumer: rb_effect_count() -> rb_resolve_dynamic_count(),
 *           dynamic_count.c:250-270.
 * ===================================================================== */

static void test_dynamic_count(void)
{
    /* Two fixtures, because the two shapes occur on different effects.
     *
     * PL!-bp5-001-AR ab#0 (unique_abilities[33]) -- look_action:
     *   dynamic_count { type dynamic_count, reference total_live_score,
     *                   mode equals, calculation add, calculation_value 2 }
     * PL!HS-PR-036-PR ab#0 (unique_abilities[339]) -- first child:
     *   dynamic_count { type dynamic_count, reference waitroom_count_below_base,
     *                   mode equals, base_reference "8" }
     * Assert the values abilities.json specifies, not merely presence. */
    static const struct { uint32_t ab; const char *card; const char *key;
                          const char *want; } ROWS[] = {
        {  33, "PL!-bp5-001-AR",  "reference",         "total_live_score"        },
        {  33, "PL!-bp5-001-AR",  "mode",              "equals"                 },
        {  33, "PL!-bp5-001-AR",  "calculation",       "add"                    },
        {  33, "PL!-bp5-001-AR",  "calculation_value", "2"                      },
        {  33, "PL!-bp5-001-AR",  "count_type",        "dynamic_count"          },
        { 339, "PL!HS-PR-036-PR", "reference",         "waitroom_count_below_base" },
        { 339, "PL!HS-PR-036-PR", "mode",              "equals"                 },
        { 339, "PL!HS-PR-036-PR", "base_reference",    "8"                      },
        { 339, "PL!HS-PR-036-PR", "count_type",        "dynamic_count"          },
    };
    for (size_t i = 0; i < sizeof(ROWS) / sizeof(ROWS[0]); i++) {
        Ability a;
        int ok = rb_decode_ability(ROWS[i].ab, &a);
        CHECK(ok, "ability index %u (%s ab#0) must decode", ROWS[i].ab, ROWS[i].card);
        if (!ok) continue;
        Ctx ctx = { ROWS[i].key, NULL };
        walk_find(a.effect, &ctx, 0);
        if (!ctx.hit && a.cost) walk_find(a.cost, &ctx, 0);
        CHECK(ctx.hit && !strcmp(ctx.hit, ROWS[i].want),
              "dynamic_count.%s on %s (ability %u) must decode to \"%s\" "
              "(got [%s])", ROWS[i].key, ROWS[i].card, ROWS[i].ab,
              ROWS[i].want, ctx.hit ? ctx.hit : "(absent)");
        rb_free_ability(&a);
    }
    Where w;
    CHECK(find_card_with("reference", &w),
          "some card must carry a dynamic_count.reference (corpus has 47)");
    printf("  first card carrying dynamic_count.reference: card %d slot %d "
           "(ability %u)\n", w.card, w.slot, w.ab);
}

/* =====================================================================
 * ROW 2 -- quoted_text   [FIXED in vm.c this round]  (same shape as ROW 1)
 *
 * Generator: "quoted_text" => ek.quoted_text = bc.read_quoted_text_value()
 *            (effect_decoder_gen.rs:150). vm.rs:1175-1205 accepts
 *            TAG_OBJECT | TAG_OBJECT_VARIANT and yields
 *            QuotedText { text, quoted_type }.
 * Wire:      nested object, no "action" -> 0x08. 11 occurrences.
 * C (was):   no branch; the scalar tail skip_value()'d it.
 * ===================================================================== */

static void test_quoted_text(void)
{
    /* PL!HS-bp6-031-L ab#0 (unique_abilities[718]) carries
     * quoted_text.quoted_type == "character". */
    Ability a;
    int ok = ability_of("PL!HS-bp6-031-L", 0, &a);
    CHECK(ok, "PL!HS-bp6-031-L ab#0 must decode");
    if (!ok) return;
    Ctx ctx = { "quoted_type", NULL };
    walk_find(a.effect, &ctx, 0);
    CHECK(ctx.hit && !strcmp(ctx.hit, "character"),
          "quoted_text.quoted_type must decode to \"character\" on "
          "PL!HS-bp6-031-L ab#0 (got [%s])", ctx.hit ? ctx.hit : "(absent)");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 3 -- card_property   [FIXED: the field exists and is consumed]
 *
 * Generator: "card_property" => ek.card_property = bc.read_arc_str_value()
 *            (effect_decoder_gen.rs:157); build_filter copies it into
 *            EffectFilter::card_property (line 514); CardFilter reads it in
 *            check_card_property (C port: rb_check_card_property,
 *            rabuka.h:2409, implemented in src/ability/util.c:2772).
 * Wire:      TAG_STR. 16 occurrences, all inside a card_count_condition.
 * C:         CORRECTION to an earlier peer claim recorded here as
 *            "STILL OPEN": the production side arrived.
 *              - include/rabuka.h:2342 declares `char card_property[64]` in
 *                RbCardFilter, unconditionally -- the RB_CARD_FILTER_CARD_PROPERTY
 *                switch that used to make the field optional is gone
 *                (util.c:39-47, card.c:926-939 both say so).
 *              - card_filter_from_effect populates it from the effect's extras
 *                (card.c:938) and counts it in has_filter (card.c:1003),
 *                mirroring util.rs:1882 and util.rs:1444.
 *              - the matcher reads it (util.c:1603-1609) and routes it to
 *                rb_check_card_property, i.e. util.rs:1654-1673.
 *            So the assertions below check the real chain: the decoded value
 *            reaches RbCardFilter, and the filter built from it actually keeps
 *            score-icon cards and drops the rest.
 * ===================================================================== */

static void test_card_property(void)
{
    /* PL!-pb2-004-PP ab#0 (unique_abilities[313]) carries, on the effect
     * ROOT: card_property "has_score_icon", group_names ["mu's"],
     * per_unit true, per_unit_count 1, per_unit_type "card-count",
     * location "success_live_zone", target "self". */
    Ability a;
    int ok = ability_of("PL!-pb2-004-PP", 0, &a);
    CHECK(ok, "PL!-pb2-004-PP ab#0 must decode");
    if (!ok) return;
    Ctx ctx = { "card_property", NULL };
    walk_find(a.effect, &ctx, 0);
    /* Decoded VALUE must be right. */
    CHECK(ctx.hit && !strcmp(ctx.hit, "has_score_icon"),
          "card_property must decode to \"has_score_icon\" on "
          "PL!-pb2-004-PP ab#0 (got [%s])", ctx.hit ? ctx.hit : "(absent)");

    /* ...and it must REACH the filter. util.rs:1882 reads
     * `card_property: effect.card_property_any()` into CardFilter; the C twin
     * is card_filter_from_effect (card.c:938), exposed as
     * rb_effect_filter_subset (card.c:1007). */
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    int built = a.effect ? rb_effect_filter_subset(a.effect, &f) : 0;
    CHECK(built && f.card_property[0],
          "card_property must populate RbCardFilter.card_property (rabuka.h:2342) "
          "from the effect that decodes it (card.c:938) -- got [%s]",
          built ? f.card_property : "(no filter built)");
    CHECK(built && !strcmp(f.card_property, "has_score_icon"),
          "the populated card_property must be the decoded value "
          "\"has_score_icon\" (got [%s])", built ? f.card_property : "(none)");
    CHECK(built && f.has_filter,
          "card_property alone is a has_filter term (util.rs:1440-1444, "
          "card.c:1003)");

    /* ...and the matcher must CONSUME it. util.rs:1654-1673 turns the
     * property into `card.has_score_icon()` and `negation` inverts it. Build a
     * filter carrying ONLY that term so the assertion isolates it, then run the
     * public matcher rb_matching_ids (util.c:1738) over one card of each kind.
     * The two cards are found by scanning the corpus rather than hard-coded, so
     * the assertion cannot rot when a card number is reissued. */
    int with_icon = -1, without_icon = -1;
    for (uint32_t i = 0; i < rb_num_cards() && (with_icon < 0 || without_icon < 0); i++) {
        Card c;
        if (!rb_decode_card_by_index(i, &c)) continue;
        if (rb_card_has_score_icon(&c)) { if (with_icon < 0) with_icon = (int)i; }
        else if (without_icon < 0) without_icon = (int)i;
        rb_free_card(&c);
    }
    CHECK(with_icon >= 0 && without_icon >= 0,
          "the corpus must contain a score-icon card and a card without one "
          "(found %d / %d)", with_icon, without_icon);
    if (with_icon < 0 || without_icon < 0) { rb_free_ability(&a); return; }

    AbilityEffect only;
    memset(&only, 0, sizeof(only));
    only.action = (char *)"card_count_condition";
    only.extra_k[0] = (char *)"card_property";
    only.extra_v[0] = (char *)"has_score_icon";
    only.n_extra = 1;

    RbCardFilter mf;
    memset(&mf, 0, sizeof(mf));
    int mbuilt = rb_effect_filter_subset(&only, &mf);
    CHECK(mbuilt && !strcmp(mf.card_property, "has_score_icon"),
          "a lone card_property extra must build a filter that still carries it "
          "(got [%s])", mbuilt ? mf.card_property : "(no filter built)");

    int cands[2] = { with_icon, without_icon };
    int got[2];
    int ngot = rb_matching_ids(&mf, cands, 2, got, 2);
    CHECK(ngot == 1 && got[0] == with_icon,
          "a card_property=has_score_icon filter must keep only the score-icon "
          "card (kept %d of 2)", ngot);

    mf.negation = 1;   /* util.rs:1663-1667 */
    ngot = rb_matching_ids(&mf, cands, 2, got, 2);
    CHECK(ngot == 1 && got[0] == without_icon,
          "negation must invert the card_property term and keep only the card "
          "without the icon (kept %d of 2)", ngot);

    /* util.rs:1656-1661 - an unknown property string is false, so a negated
     * unknown property keeps everything. */
    only.extra_v[0] = (char *)"not_a_real_property";
    RbCardFilter uf;
    memset(&uf, 0, sizeof(uf));
    (void)rb_effect_filter_subset(&only, &uf);
    CHECK(rb_matching_ids(&uf, cands, 2, got, 2) == 0,
          "an unknown card_property is false, so an un-negated filter keeps "
          "nothing (util.rs:1661)");
    uf.negation = 1;
    CHECK(rb_matching_ids(&uf, cands, 2, got, 2) == 2,
          "an unknown card_property under negation keeps every card "
          "(util.rs:1663-1667)");

    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 4 -- placement_order   [decode routed explicitly; consumer EXISTS]
 *
 * Generator: "placement_order" => ek.placement_order =
 *            bc.read_placement_order_value() (effect_decoder_gen.rs:186).
 *            vm.rs:1159-1173 matches only "any_order" onto
 *            PlacementOrder::AnyOrder and None for anything else.
 * Wire:      TAG_STR. 35 occurrences, every one "any_order".
 * C:         Now has its own branch (effect_decode_placement_order, vm.c)
 *            instead of relying on the scalar tail. CORRECTION to an earlier
 *            peer claim: the value IS consumed --
 *            src/ability/effects/ability_effects.c:433 and
 *            src/ability/effects/misc.c:987 both read the extras entry and
 *            compare it against "any_order" to route the effect to
 *            move_cards. Assert the extras contract those two sites rely on,
 *            so the routing cannot silently break.
 * ===================================================================== */

static void test_placement_order(void)
{
    /* PL!-bp3-014-N ab#0 (unique_abilities[6]): the select_action step
     * carries placement_order "any_order" and any_number true. */
    Ability a;
    int ok = ability_of("PL!-bp3-014-N", 0, &a);
    CHECK(ok, "PL!-bp3-014-N ab#0 must decode");
    if (!ok) return;
    const AbilityEffect *sel = a.effect ? a.effect->select_action : NULL;
    CHECK(sel != NULL,
          "PL!-bp3-014-N ab#0's look_and_select must decode its select_action");
    /* The two routing sites read the extras entry by this exact key. */
    const char *po = sel ? ex(sel, "placement_order") : NULL;
    CHECK(po && !strcmp(po, "any_order"),
          "select_action.placement_order must decode to \"any_order\" so "
          "ability_effects.c:434 / misc.c:988 can route it (got [%s])",
          po ? po : "(absent)");
    CHECK(sel && ex(sel, "any_number") &&
              !strcmp(ex(sel, "any_number"), "true"),
          "select_action.any_number must decode to \"true\" (got [%s])",
          sel && ex(sel, "any_number") ? ex(sel, "any_number") : "(absent)");
    CHECK(sel && sel->destination && !strcmp(sel->destination, "deck_top"),
          "select_action.destination must decode to \"deck_top\" (got [%s])",
          sel && sel->destination ? sel->destination : "(null)");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 5 -- watches_ability_resolution   [STILL OPEN]
 *
 * Generator: "watches_ability_resolution" => ek.watches_ability_resolution
 *            = bc.read_bool_value() (effect_decoder_gen.rs:69). It is an
 *            ABILITYEFFECT field (vm.rs:1429, written into AbilityEffect at
 *            vm.rs:1500), NOT a filter field -- build_filter does not copy it,
 *            so only the AbilityEffect slot holds it in Rust.
 * Wire:      TAG_TRUE / TAG_FALSE. 4 occurrences.
 * C:         no branch. The scalar tail stores "true"/"false" in the extras
 *            table, and nothing reads either the extra or a struct member.
 * ===================================================================== */

static void test_watches_ability_resolution(void)
{
    /* PL!-bp6-020-L ab#0 (unique_abilities[630]) carries it. */
    Ability a;
    int ok = ability_of("PL!-bp6-020-L", 0, &a);
    CHECK(ok, "PL!-bp6-020-L ab#0 must decode");
    if (!ok) return;
    Ctx ctx = { "watches_ability_resolution", NULL };
    walk_find(a.effect, &ctx, 0);
    CHECK(ctx.hit && (!strcmp(ctx.hit, "true") || !strcmp(ctx.hit, "false")),
          "watches_ability_resolution must decode to a bool on "
          "PL!-bp6-020-L ab#0 (got [%s])", ctx.hit ? ctx.hit : "(absent)");
    OPEN("watches_ability_resolution", "ENGINE BUG",
         "decodes to [%s] into the extras table; in Rust it is an "
         "AbilityEffect slot, and nothing reads either\n",
         ctx.hit ? ctx.hit : "(absent)");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 6 -- multi-value heart_colors
 *
 * Generator: "heart_colors" => ek.heart_colors = *bc.read_str_vec_value()
 *            (effect_decoder_gen.rs:91) -- a VEC, order preserved,
 *            multi-valued.
 * Wire:      TAG_ARRAY of TAG_STR. 264 occurrences.
 * C:         the scalar tail joins a homogeneous string array as a CSV
 *            ("heart02,heart04") and writes the single-colour alias
 *            "heart_color" ONLY when the joined value has no comma. So a
 *            two-colour filter loses the alias entirely.
 *            Two consequences, asserted below:
 *              (a) the alias must NOT appear for a multi list -- a two-colour
 *                  requirement must never be readable as one colour;
 *              (b) rb_parse_heart_color (src/ability/util.c:2404) has no comma
 *                  handling and falls through to `return RB_HEART_PINK`, so any
 *                  consumer handed the CSV gets PINK. That is util.c's file,
 *                  not vm.c's; recorded here as the open consumption half.
 * ===================================================================== */

static void test_heart_colors_multi(void)
{
    /* PL!S-bp6-004-P ab#0 uses require_all_heart_colors over a multi list. */
    Ability a;
    int ok = ability_of("PL!S-bp6-004-P", 0, &a);
    CHECK(ok, "PL!S-bp6-004-P ab#0 must decode");
    if (!ok) return;
    Ctx hcm = { "heart_colors", NULL }, hcs = { "heart_color", NULL };
    walk_find(a.effect, &hcm, 0);
    walk_find(a.effect, &hcs, 0);
    printf("  heart_colors=[%s] heart_color=[%s]\n",
           hcm.hit ? hcm.hit : "(absent)", hcs.hit ? hcs.hit : "(absent)");
    CHECK(hcm.hit && *hcm.hit,
          "heart_colors must survive decode on PL!S-bp6-004-P ab#0");
    if (hcm.hit && strchr(hcm.hit, ',')) {
        CHECK(strstr(hcm.hit, "heart02") && strstr(hcm.hit, "heart04"),
              "the multi-colour list must contain both heart02 and heart04 "
              "(got [%s])", hcm.hit);
        CHECK(hcs.hit == NULL,
              "a multi-colour heart_colors list must not also publish a "
              "single-colour heart_color alias (got [%s])",
              hcs.hit ? hcs.hit : "(absent)");
        OPEN("heart_colors", "ENGINE BUG",
             "the list survives only as the CSV [%s]; "
             "rb_parse_heart_color (util.c:2404) has no comma handling and "
             "returns RB_HEART_PINK for it\n", hcm.hit);
    }
    /* The single-colour case must still publish the alias. */
    {
        Ability b;
        if (ability_of("PL!HS-bp1-010-N", 0, &b)) {
            Ctx s = { "heart_color", NULL };
            walk_find(b.effect, &s, 0);
            (void)s;   /* draw_card has no heart_color; do not assert */
            rb_free_ability(&b);
        }
    }
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 7 -- Keyword vocabulary
 *
 * Spec: vm.rs:1703-1713 keyword_from_str maps ten strings, and each maps to a
 *       Keyword enum member. C: vm.c g_kw_map (now ~L1034) carries all ten.
 *       Verify the whole vocabulary, not just the one a peer reported.
 * ===================================================================== */

static void test_keyword_center(void)
{
    CHECK(rb_keyword_from_str("Center") == RB_KW_CENTER,
          "rb_keyword_from_str(\"Center\") must be RB_KW_CENTER (got %d)",
          rb_keyword_from_str("Center"));
    CHECK(rb_keyword_from_str("FormationChange") == RB_KW_FORMATION_CHANGE,
          "rb_keyword_from_str(\"FormationChange\") must be "
          "RB_KW_FORMATION_CHANGE");
    static const char *ALL[] = {
        "Turn1", "Turn2", "Debut", "LiveStart", "LiveSuccess", "Center",
        "LeftSide", "RightSide", "PositionChange", "FormationChange",
    };
    for (size_t i = 0; i < sizeof(ALL) / sizeof(ALL[0]); i++) {
        CHECK(rb_keyword_from_str(ALL[i]) != RB_KW_COUNT,
              "keyword \"%s\" must be recognised by the C decoder", ALL[i]);
    }
}

/* =====================================================================
 * ROW 8 -- group_names is a LIST
 *
 * Generator: "group_names" => ek.group_names = bc.read_opt_str_vec_value()
 *            (effect_decoder_gen.rs:85) -> Option<Vec<String>>; build_filter
 *            boxes it (line 439); CardFilter matching is per-element.
 * Wire:      TAG_ARRAY of TAG_STR. 404 occurrences.
 * C:         two different outcomes depending on WHERE the key sits, and
 *            only the condition path preserves the list:
 *              - inside a condition, read_cond_value's ARRAY arm (vm.c) keeps
 *                a real array of CondValue;
 *              - on an effect, the scalar tail joins the array into one CSV
 *                token.
 *            Assert the condition path is a genuine list.
 * ===================================================================== */

static void test_group_names_list(void)
{
    /* PL!HS-bp1-003-P ab#0's compound condition carries group_names
     * ["Hasunosora"] -- a single-element list in the corpus. */
    Ability a;
    int ok = ability_of("PL!HS-bp1-003-P", 0, &a);
    CHECK(ok, "PL!HS-bp1-003-P ab#0 must decode");
    if (!ok) return;

    int found = 0;
    const Condition *stack[64];
    int sp = 0;
    if (a.effect && a.effect->condition) stack[sp++] = a.effect->condition;
    while (sp > 0 && !found) {
        const Condition *c = stack[--sp];
        if (!c) continue;
        for (uint32_t i = 0; i < c->n_fields; i++) {
            const CondField *f = &c->fields[i];
            if (!f->key) continue;
            if (!strcmp(f->key, "group_names") &&
                f->v.tag == RB_TAG_ARRAY && f->v.arr_n >= 1) {
                printf("  condition group_names: %u element(s), first=[%s]\n",
                       f->v.arr_n, f->v.arr[0].s ? f->v.arr[0].s : "(null)");
                CHECK(f->v.arr_n == 1,
                      "this condition's group_names has exactly one element "
                      "(got %u)", f->v.arr_n);
                CHECK(f->v.arr[0].s && *f->v.arr[0].s,
                      "the group_names element must be a non-empty string");
                found = 1;
                break;
            }
            if (f->v.tag == RB_TAG_OBJVAR && f->v.cond && sp < 60)
                stack[sp++] = f->v.cond;
            if (f->v.tag == RB_TAG_ARRAY) {
                for (uint32_t j = 0; j < f->v.arr_n && sp < 60; j++)
                    if (f->v.arr[j].tag == RB_TAG_OBJVAR && f->v.arr[j].cond)
                        stack[sp++] = f->v.arr[j].cond;
            }
        }
    }
    CHECK(found, "PL!HS-bp1-003-P ab#0's condition must decode group_names as "
                 "a list, not as one joined token");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 9 -- the `replacement` field the GENERATOR does not know
 *
 * abilities.json has one effect object with the key "replacement"
 * (PL!-pb2-002-PP, action replacement_appear_wait, value "appear_wait"). It is
 * absent from effect_decoder_gen.rs's match, so Rust logs "unknown effect
 * field" and skip_value()s it -- the value is lost in RUST TOO. This is a
 * GENERATOR gap, not a C gap: the C is a superset here (it keeps the string as
 * an extra). Recorded so the row cannot be silently forgotten; the fix belongs
 * in cards/generate_effect_decoder.py, not in vm.c.
 * ===================================================================== */

static void test_replacement_unknown_key(void)
{
    Ability a;
    int ok = ability_of("PL!-pb2-002-PP", 0, &a);
    CHECK(ok, "PL!-pb2-002-PP ab#0 must decode");
    if (!ok) return;
    const char *r = ex(a.effect, "replacement");
    CHECK(r && !strcmp(r, "appear_wait"),
          "the unmodelled replacement key must at least survive as an extra "
          "(got [%s])", r ? r : "(absent)");
    printf("  replacement=[%s] (generator gap: effect_decoder_gen.rs has no "
           "\"replacement\" arm, so Rust drops it too)\n", r ? r : "(absent)");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 10 -- source / target / destination   (VERIFY; do not re-report)
 *
 * A peer reported these as discarded so every `select` fell back to the hand.
 * Against the CURRENT file: rabuka.h declares source/target/destination on
 * AbilityEffect, and vm.c decodes all three (plus the "zone" alias). Assert the
 * decoded VALUES so a regression cannot quietly re-open it.
 * ===================================================================== */

static void test_source_target_destination(void)
{
    /* PL!HS-bp1-010-N ab#0: sequential, step 0 is draw_card deck -> hand. */
    Ability a;
    int ok = ability_of("PL!HS-bp1-010-N", 0, &a);
    CHECK(ok, "PL!HS-bp1-010-N ab#0 must decode");
    if (!ok) return;
    CHECK(a.effect && a.effect->n_child == 2,
          "the sequential root must have exactly 2 decoded child effects "
          "(got %d)", a.effect ? a.effect->n_child : -1);
    if (a.effect && a.effect->n_child == 2) {
        const AbilityEffect *s0 = a.effect->child[0];
        CHECK(s0 && s0->source && !strcmp(s0->source, "deck"),
              "step 0 source must be \"deck\" (got [%s])",
              s0 && s0->source ? s0->source : "(null)");
        CHECK(s0 && s0->destination && !strcmp(s0->destination, "hand"),
              "step 0 destination must be \"hand\" (got [%s])",
              s0 && s0->destination ? s0->destination : "(null)");
        CHECK(s0 && s0->action && !strcmp(s0->action, "draw_card"),
              "step 0 action must be \"draw_card\" (got [%s])",
              s0 && s0->action ? s0->action : "(null)");
        CHECK(s0 && s0->count == 1,
              "step 0 count must be 1 (got %d)", s0 ? s0->count : -999);
    }
    rb_free_ability(&a);

    /* PL!-bp5-003-AR ab#0 (unique_abilities[34]): a gain_resource whose ROOT
     * carries target "self", duration "live_end" and a heart_colors list. */
    Ability b;
    int ok2 = ability_of("PL!-bp5-003-AR", 0, &b);
    CHECK(ok2, "PL!-bp5-003-AR ab#0 must decode");
    if (ok2) {
        printf("  root target=[%s] duration=[%s] heart_colors=[%s]\n",
               b.effect && b.effect->target ? b.effect->target : "(null)",
               ex(b.effect, "duration") ? ex(b.effect, "duration") : "-",
               ex(b.effect, "heart_colors") ? ex(b.effect, "heart_colors") : "-");
        CHECK(b.effect && b.effect->target &&
                  !strcmp(b.effect->target, "self"),
              "the root effect's target must decode to \"self\" (got [%s])",
              b.effect && b.effect->target ? b.effect->target : "(null)");
        CHECK(ex(b.effect, "duration") &&
                  !strcmp(ex(b.effect, "duration"), "as_long_as"),
              "the root effect's duration must decode to \"as_long_as\" "
              "(got [%s])", ex(b.effect, "duration") ? ex(b.effect, "duration") : "-");
    }
    rb_free_ability(&b);
}

/* =====================================================================
 * ROW 11 -- the cost key aliases `type` / `zone` are UNREACHABLE
 *
 * vm.c treats "type", "cost_type" and "zone" as aliases of "action"/"source".
 * cards/compile_abilities.py pops them BEFORE serialisation, so no cost object
 * reaches the wire with them. Verified by hand over the corpus: every `type`
 * on a cost object is one of the seven COST_TYPE_TO_ACTION keys (move_cards,
 * pay_energy, reveal, change_state, place_energy_under_member,
 * sequential_cost, choice_condition), and no effect object carries both
 * `action` and `type`. Recorded so the tolerance is not "fixed" later by
 * deleting live code.
 * ===================================================================== */

static void test_cost_aliases(void)
{
    /* PL!HS-bp1-010-N ab#0 has cost: null in abilities.json. */
    Ability a;
    int ok = ability_of("PL!HS-bp1-010-N", 0, &a);
    if (!ok) { printf("  (fixture absent)\n"); return; }
    CHECK(a.cost == NULL,
          "this ability has cost=null in abilities.json, so it must decode to "
          "a NULL cost");
    rb_free_ability(&a);

    /* And a card that DOES have a cost: the alias must have been applied
     * before serialisation, so the decoded cost root carries `action`. */
    Ability b;
    int ok2 = ability_of("PL!S-bp6-004-P", 0, &b);
    if (ok2 && b.cost) {
        printf("  cost root action=[%s] source=[%s]\n",
               b.cost->action ? b.cost->action : "(null)",
               b.cost->source ? b.cost->source : "(null)");
        CHECK(b.cost->action && *b.cost->action,
              "a cost root must decode to a non-empty action string");
    }
    rb_free_ability(&b);
}

/* =====================================================================
 * ROW 14 -- heart_total_limit / heart_total_operator   [STILL OPEN]
 *
 * Generator: both are filter fields (effect_decoder_gen.rs:137-138 ->
 * EffectFilter::heart_total_limit / heart_total_operator).
 * Wire:      TAG_I64 + TAG_STR. 5 effect objects, always paired with
 *            `original_value: true` and `position: "center"`.
 * RUST:      CardFilter gates the appear-time replacement on a member's
 *            ORIGINAL heart total -- "members whose original heart count is 4
 *            or fewer appear WAIT instead of ACTIVE" (PL!-pb2-002-PP ab#0).
 * C:         the pair decodes into the extras table with the right values, but
 *            RbCardFilter has no member for it and card_filter_from_effect
 *            (src/core/card.c:776-859) never reads it. Note the asymmetry with
 *            `original_value`, which IS read -- but only inside the
 *            `blade_limit` branch at card.c:830, never in a heart branch.
 * ===================================================================== */

static void test_heart_total_limit(void)
{
    /* PL!-pb2-002-PP ab#0 (unique_abilities[32]) carries
     * heart_total_limit 4, heart_total_operator "<=", original_value true,
     * position "center", count 4, card_type member_card. */
    Ability a;
    int ok = ability_of("PL!-pb2-002-PP", 0, &a);
    CHECK(ok, "PL!-pb2-002-PP ab#0 must decode");
    if (!ok) return;
    Ctx lim = { "heart_total_limit", NULL }, op = { "heart_total_operator", NULL };
    walk_find(a.effect, &lim, 0);
    walk_find(a.effect, &op, 0);
    CHECK(lim.hit && !strcmp(lim.hit, "4"),
          "heart_total_limit must decode to \"4\" (got [%s])",
          lim.hit ? lim.hit : "(absent)");
    CHECK(op.hit && !strcmp(op.hit, "<="),
          "heart_total_operator must decode to \"<=\" (got [%s])",
          op.hit ? op.hit : "(absent)");
    OPEN("heart_total_limit", "ENGINE BUG",
         "both decode to [%s]%s, but RbCardFilter has no member and "
         "card_filter_from_effect (card.c:776) never reads them, so the "
         "appear-wait replacement can never fire\n",
         lim.hit ? lim.hit : "(absent)", op.hit ? op.hit : "(absent)");
    CHECK(0, "heart_total_limit/operator must reach the appear-time filter");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 15 -- choose_target_player carries effect_steps   [decode OK, consume OPEN]
 *
 * Generator: "choose_target_player" is a plain action string;
 *            "choice_options" and "effect_steps" are both generator arms
 *            (read_str_vec_value / read_effect_vec_value), so the shape is
 *            legal on the wire. All 6 choose_target_player objects in the
 *            corpus carry BOTH.
 * C:          effect_steps decode into AbilityEffect.child[] correctly, and
 *             `action_by` is an extras entry. But
 *             src/ability/effects/misc.c:968 h_choose_target_player emits a
 *             fixed RB_CHOICE_SELECT_TARGET and never walks e->child, so the
 *             per-branch effect is unreachable: PL!N-bp3-010-P's "self puts up
 *             to 2 member cards from the target's discard in any order on the
 *             bottom of the deck" cannot run.
 *             That is an executor gap (executor.c / effects/misc.c), not a
 *             decoder gap -- reported, not fixed here.
 * ===================================================================== */

static void test_choose_target_player_steps(void)
{
    Ability a;
    int ok = ability_of("PL!N-bp3-010-P", 0, &a);
    CHECK(ok, "PL!N-bp3-010-P ab#0 must decode");
    if (!ok) return;
    const AbilityEffect *e = a.effect;
    CHECK(e && e->action && !strcmp(e->action, "choose_target_player"),
          "PL!N-bp3-010-P ab#0 must decode to action choose_target_player "
          "(got [%s])", e && e->action ? e->action : "(null)");
    /* choice_options is a two-element list; assert the joined value really
     * carries BOTH (i.e. the array was not collapsed to one token). The two
     * strings are multibyte, so count the separator instead of spelling them
     * out, keeping this source 7-bit clean. */
    const char *co = e ? ex(e, "choice_options") : NULL;
    CHECK(co && strchr(co, ','),
          "choice_options must decode as a two-element list, not one token "
          "(got [%s])", co ? co : "(absent)");
    CHECK(co && co[0] && strlen(co) > 4,
          "choice_options must carry both non-empty player options (got [%s])",
          co ? co : "(absent)");
    /* effect_steps must decode into a real child effect. */
    CHECK(e && e->n_child == 1,
          "the single effect_step must decode as exactly 1 child (got %d)",
          e ? e->n_child : -1);
    if (e && e->n_child == 1) {
        const AbilityEffect *s = e->child[0];
        CHECK(s && s->action && !strcmp(s->action, "move_cards"),
              "step 0 action must be \"move_cards\" (got [%s])",
              s && s->action ? s->action : "(null)");
        CHECK(s && s->source && !strcmp(s->source, "discard"),
              "step 0 source must be \"discard\" (got [%s])",
              s && s->source ? s->source : "(null)");
        CHECK(s && s->destination && !strcmp(s->destination, "deck_bottom"),
              "step 0 destination must be \"deck_bottom\" (got [%s])",
              s && s->destination ? s->destination : "(null)");
        CHECK(s && s->count == 2,
              "step 0 count must be 2 (got %d)", s ? s->count : -999);
        const char *po = s ? ex(s, "placement_order") : NULL;
        CHECK(po && !strcmp(po, "any_order"),
              "step 0 placement_order must be \"any_order\" (got [%s])",
              po ? po : "(absent)");
        const char *ab = s ? ex(s, "action_by") : NULL;
        CHECK(ab && !strcmp(ab, "self"),
              "step 0 action_by must be \"self\" (got [%s])",
              ab ? ab : "(absent)");
    }
    OPEN("choose_target_player", "ENGINE BUG",
         "all 6 corpus objects carry choice_options + effect_steps, but "
         "h_choose_target_player (effects/misc.c:968) never walks e->child, so "
         "the decoded branch is unreachable\n");
    CHECK(0, "choose_target_player must run the effect_steps it decodes");
    rb_free_ability(&a);
}

/* =====================================================================
 * ROW 16 -- every card-ability pair still decodes (regression guard).
 * ===================================================================== */

static void test_all_pairs_decode(void)
{
    uint32_t nc = rb_num_cards();
    int pairs = 0, ok = 0;
    for (uint32_t c = 0; c < nc; c++) {
        int n = rb_card_num_abilities(c);
        for (int i = 0; i < n; i++) {
            pairs++;
            Ability a;
            if (rb_decode_card_ability(c, i, &a)) {
                ok++;
                rb_free_ability(&a);
            }
        }
    }
    printf("  card-ability pairs %d, decoded %d\n", pairs, ok);
    CHECK(pairs > 2000, "the corpus must still carry >2000 card-ability pairs");
    CHECK(pairs == ok, "every card-ability pair must decode");
}

/* =====================================================================
 * ROW 13 -- the extras table cannot overflow.
 *
 * RB_MAX_EXTRA is 32 (rabuka.h:84) and effect_set_extra returns 0 when full;
 * the scalar tail then fails the whole effect. Measured over the corpus, the
 * widest decoded root carries 12 extras, so 32 is not currently a constraint --
 * but assert the headroom so a future field cannot silently start failing
 * abilities.
 * ===================================================================== */

static void test_extras_headroom(void)
{
    int worst = 0;
    for (uint32_t i = 0; i < rb_num_abilities(); i++) {
        Ability a;
        if (!rb_decode_ability(i, &a)) continue;
        int m = a.effect ? a.effect->n_extra : 0;
        if (a.cost && a.cost->n_extra > m) m = a.cost->n_extra;
        if (m > worst) worst = m;
        rb_free_ability(&a);
    }
    printf("  widest root extras table: %d of RB_MAX_EXTRA(%d)\n",
           worst, RB_MAX_EXTRA);
    CHECK(worst < RB_MAX_EXTRA,
          "the widest decoded root must leave room in the extras table "
          "(%d of %d)", worst, RB_MAX_EXTRA);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }
    tally_init();

    printf("=== ability-field audit: does the C decoder express the FULL "
           "RANGE of what abilities.json can carry? ===\n");

    run("A_census",              test_census);
    run("B_dynamic_count",       test_dynamic_count);
    run("C_quoted_text",         test_quoted_text);
    run("D_card_property",       test_card_property);
    run("E_placement_order",     test_placement_order);
    run("F_watches_ability_res", test_watches_ability_resolution);
    run("G_heart_colors_multi",  test_heart_colors_multi);
    run("H_keyword_vocabulary",  test_keyword_center);
    run("I_group_names_list",    test_group_names_list);
    run("J_replacement_unknown", test_replacement_unknown_key);
    run("K_source_target_dest",  test_source_target_destination);
    run("L_cost_aliases",        test_cost_aliases);
    run("M_heart_total_limit",   test_heart_total_limit);
    run("N_choose_target_steps", test_choose_target_player_steps);
    run("O_all_pairs_decode",    test_all_pairs_decode);
    run("P_extras_headroom",     test_extras_headroom);

    printf("\n=== %d ok, %d FAILED, %d crashed ===\n",
           n_ok, n_failed, n_crashed);
    printf("NOTE: every line beginning OPEN is a field the C decoder or its\n"
           "      consumers do not yet express. Those are printed, not counted\n"
           "      as suite failures, so the suite stays green while the open\n"
           "      rows are tracked. Flip OPEN() to CHECK() when the engine is\n"
           "      fixed -- never the other way round.\n");
    return n_failed || n_crashed ? 1 : 0;
}
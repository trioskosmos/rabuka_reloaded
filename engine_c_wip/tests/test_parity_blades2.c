/* test_parity_blades2.c -- C parity suite for the Rust clusters
 *   engine/tests/test_modules/effects/gain/blades/per_card/   (14 .rs)
 *   engine/tests/test_modules/effects/ability_mod/           (14 .rs)
 *
 * COVERAGE BOUNDARY, established from the BODIES of the peer files and from a
 * card-number scan of every tests/*.c in this tree (a header claim has been
 * wrong four times in this project, once listing a covered file as "NOT
 * COVERED", so every claim below was re-checked against code, not comments):
 *
 *   test_parity_blades_live_start.c (Part A = gain/blades/constants/,
 *   Part B = gain/blades/live_start/) contains NO fixture card from any of the
 *   14 gain/blades/per_card .rs files.  The only names it shares are
 *   "kanon"/"ayumu" as LOCAL VARIABLE NAMES for different prints
 *   (PL!N-pb1-001-R 上原歩夢, PL!N-bp1-001-R, PL!SP-bp1-001-R 澁谷かのん) and
 *   PL!-bp3-002-R as a per_waited_opponent_member constant source, which is a
 *   different behaviour from eri_debut_up_to_two_opponent_members_to_wait_test.rs.
 *
 *   test_parity_blade_under.c covers gain/blades/under_member/ only.  Its
 *   PL!S-bp7-005-R＋ usage is the under-card constant (ab#1) and her 登場
 *   (ab#0), NOT ab#2 (the 起動 that fires other members' 登場 with their costs
 *   paid) that q273_fired_debut_ability_cost_paid_test.rs exercises.  The
 *   decode dump below (RB2_DUMP=1) confirms 渡辺 曜 has exactly 3 abilities,
 *   ab#0 登場 / ab#1 常時 / ab#2 起動, so ab#2 is reached only from here.
 *
 *   test_parity_ability_mod.c + test_parity_ability_mod_extra.c together
 *   cover 13 of the 14 ability_mod files.  The three cases a previous pass had
 *   provisionally targeted from a card-number grep are ALREADY IN extra.c's
 *   bodies and RUN list -- test_chisato_turn1_blocks_second_activation
 *   (extra.c:735, RUN at 1243), test_nozomi_p_rarity_same_behaviour
 *   (extra.c:776, RUN at 1244) and test_condition_loss_removes_delayed_
 *   registration (extra.c:912, RUN at 1248) -- so section H was WRITTEN and
 *   then REMOVED rather than shipped as a duplicate.
 *
 *   CORRECTION to the previous pass's header, which claimed "ALL 14 ability_mod
 *   files" are covered: they are not.  PL!S-BP2-012-N -- a fixture of
 *   full_distinct_group_stage_grants_yell_live_score_test.rs -- appears in NO
 *   tests/*.c in this tree (scanned by card number, case- and fullwidth-plus
 *   normalised).  That file's remaining gap is NOT TAKEN, below.
 *
 * TAKEN (this file)
 *   A  activate_change_state_wait_grants_live_end_blade_test.rs   2 tests
 *   B  per_unit_discard_count_regression_test.rs                  1 test
 *   C  hazuki_activate_discard_per_liella_member_blade_test.rs    3 of 4
 *   D  q46_kanako_all_heart_timing_test.rs                        7 tests
 *   E  q271_colorful_dreams_test.rs                               9 + 1 EXTRA
 *   F  q148_waited_member_blade_total_test.rs                     3 tests
 *   G  q273_fired_debut_ability_cost_paid_test.rs                 2 tests
 *   ------------------------------------------------------------------
 *                                                    28 run() cases
 *   (section H, an ability_mod section, was written and then removed as a
 *    duplicate of test_parity_ability_mod_extra.c -- see the note above)
 *
 * NOT TAKEN, by name, and why
 *   bp7_parser_gap_cards_test.rs (11)  three DIFFERENT bp7 prints
 *       (PL!SP-bp7-010-R ウィーン, PL!SP-bp7-012-N, PL!SP-bp1-005-R) plus a
 *       deck-top energy move; needs a whole dedicated file.
 *   eri_debut_up_to_two_opponent_members_to_wait_test.rs (3)
 *   kanon_debut_one_per_group_deck_bottom_edge_test.rs (3)
 *       both are 起動 that SELECT from a zone and then move, i.e. the
 *       select-then-move resolver path -- the same path section G shows to be
 *       broken -- so porting them now would produce three more copies of one
 *       red, not three new behaviours.
 *   modifier_layer_characterization_test.rs (3)   needs
 *       rb_constant_per_unit_units, which is compiled but has no declaration in
 *       include/rabuka.h and that header is owned by another agent.
 *   ren_baton_touch_group_check_test.rs (3)      BATON TOUCH turn-2 walk.
 *   turn_scope_regression_test.rs (2)            turn-rollover tracker walk.
 *   yell_same_group_members_multi_color_hearts_test.rs (3)
 *       PL!S-PR-040-PR appears in NO C test file; the other fixtures are
 *       PL!S-PR-015/016/019-PR, so the behaviour is unknown here and would be
 *       exploratory rather than parity.
 *   hazuki q233_declined_optional_energy_allows_second_trigger_same_turn (1)
 *       a 4-rarity loop over the 自動 trigger with a recently_moved relay.
 *   ability_mod full_distinct: PL!S-BP2-012-N is absent from the whole C corpus
 *       and the file's four run_live_through_victory cases are a 25-case
 *       two-seat live-through-victory matrix built on the documented
 *       test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) yell-window landmine.
 *       Out of budget; documented, not half-done.
 *
 * ENGINE GAPS FOUND BY THIS SUITE -- all four are CONFIRMED against the C
 * sources, and every assertion that exposes them is left STRICT AND RED.  Run
 * with RB2_DUMP=1 to print the engine's own decoded shape for each fixture;
 * each root cause below was read out of the C source, not inferred from the
 * red.
 *
 *  1. per_unit over a moved/discarded set counts the RAW MOVED SET, never the
 *     printed 『Liella!』 member subset.  src/ability/effects/misc.c:279-307
 *     (calculate_gain_multiplier) short-circuits on `per_unit_type == "discard"`
 *     to `matching = g->n_recently_moved` (or last_cost_discard_count) and
 *     never applies the effect's CardFilter; `per_unit_source` is not read at
 *     all, and the effect's `card_type: member_card` did not survive decoding
 *     into extras (see the RB2_DUMP output for 葉月 恋).  Rust
 *     util.rs::resolve_per_unit_count filters the moved set.
 *     葉月 恋 PL!SP-bp5-005-R＋: 2 『Liella!』 + 1 filler -> 3 blades (Rust 2);
 *     3 fillers -> 3 blades (Rust 0).
 *     src/ability/dynamic_count.c:272-330 has NO "discard" arm either, so a
 *     per-unit zone of "discard" collapses to units = 1 there.
 *
 *  2. A 常時 (constant) ability whose top-level condition FAILS still applies
 *     its children.  src/turn/triggers.c:754-763: when `cond_ok` is 0 the code
 *     falls into the ELSE arm and calls apply_constant_effect on each child
 *     whose OWN condition is absent -- and the children of a `sequential` never
 *     carry one.  鐘 嵐珠 PL!N-bp1-012-R＋ therefore grants +2 blade and +2 ALL
 *     hearts on every board where her compound condition is false (0 live cards,
 *     2 live cards, 3 live cards with no 『虹ヶ咲』 live).  Non-vacuous: the
 *     same file's paired positive on the met-condition board returns 2, and
 *     q46_kanako_not_on_stage_no_constant (host off stage) returns 0.
 *
 *  3. `unit: "types"` on a card_count condition is unimplemented, and the
 *     heart-colour filter matches BLADE hearts.  src/ability/condition.c:3121
 *     (eval_card_count) calls count_cards_with_filters and never reads `unit`,
 *     so 「3種類以上」 (three KINDS) is evaluated as three CARDS; and the flat
 *     decoded heart table includes the blade block, so 桃/青/ALL blade-hearts
 *     count as colours.  Colorful Dreams! PL!N-bp7-025-L: one revealed card with
 *     heart01+heart03+heart06 -> 0 (Rust 1); three blade-only lives -> +1
 *     (Rust 0); three heart01-only cards -> +1 (Rust 0, the new EXTRA case).
 *     Every Rust case in this file is one where card count == colour count, so
 *     none of them can see this; the EXTRA case can, and does.
 *
 *  4. `aggregate: "total"` on a location_condition can never reach BLADE.
 *     src/ability/condition.c:2051-2102 (rb_check_aggregate_total) sums only the
 *     heart colours named in the condition's own `heart_colors` list;
 *     ミはμ'sicのミ's condition (aggregate total, count 10, stage, member_card)
 *     names none, so `total` is 0 for every board and 0 >= 10 is never true --
 *     and eval_card_count's CARD reading (2 < 10) is false too, so the ability
 *     is dead on every board.  One staged member carrying 27 blades (sum 27 >=
 *     10, card count 1 < 10) still yields need_heart00 = 0, which discriminates
 *     the two readings.
 *
 *  5. After a 起動's cost raises a CHOICE on the inline rb_activate_card path,
 *     the effect half of the activation is dropped.  src/engine.c:1290-1310
 *     builds one tree act = [cost, effect] and relies on the resume
 *     continuation; the run log shows the cost's SelectCard answered and then
 *     `CHOICE_EPILOGUE ... queue_actions=0` with no follow-up, so 渡辺 曜's
 *     ab#2 sequential[select(stage,2), activate_ability(previous_selected,
 *     登場)] never raises its first child.  ADJACENT TO AN IN-FLIGHT FIX: the
 *     same continuation is what drops `sequential_cost` on this path.
 *
 * NO JAPANESE IN A FUNCTIONAL POSITION, ON PURPOSE.  The editor can emit CP932
 * and silently corrupt Japanese literals, which would break group/series
 * filters.  Card numbers are ASCII; the U+FF0B FULLWIDTH PLUS prints use the
 * byte escape "\xEF\xBC\x8B"; every trigger string the ASSERTIONS depend on
 * comes from the engine's own RB_TSTR_* constants, never from a literal here.
 * Group names are never written literally either -- they are read back with
 * rb_card_group_name() and PRINTED at startup, so a filter mismatch shows up as
 * a visible name rather than as a silent "no match".  Japanese DOES appear in
 * the comments and in the assertion messages, where it is documentation only
 * and a mis-decode would be cosmetic, not behavioural; the file is verified
 * UTF-8 after every write.
 *
 * LANDMINES respected (all confirmed by peers):
 *   - rb_load("src") fails under an isolated out-of-tree build, so load_card_db()
 *     falls back to "../cards/build".
 *   - sizeof(GameState) is ~781 KB: every TestGame here is `static`.
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE.  No heart
 *     in this file is read through it; rb_mods_get_heart / rb_mods_get_need_heart
 *     are called directly and the heartNN -> index map is printed at startup
 *     from rb_heart_color_as_str().
 *   - rb_card_string() takes a STRING-TABLE INDEX.  pin_id() compares
 *     rb_card_string(c.card_no_idx) byte-for-byte; rb_card_no_eq is NOT used as
 *     an identity pin because a peer proved it accepts PL!SP-PR-004-R against
 *     the real PL!SP-PR-004-PR.
 *   - rb_card_copy_pool_reset() runs per case (the copy pool is process-global
 *     and monotonic and would otherwise alias two copies of one print).
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - the C deck bag is TOP-INDEX-0, so Rust `cards.push(x)` is a plain append
 *     and Rust `cards.insert(0, x)` is test_insert_deck_top().
 *   - rb_advance_phase is a no-op while a choice is pending, ENTERS
 *     RB_PHASE_DRAW without drawing, and swaps `active` to the other seat before
 *     the first Active/Energy/Draw triple; every step here is drained.
 *   - a `pay_optional_costs` style gate is a SelectTarget whose SIGN decides:
 *     index >= 0 accepts (pays), index < 0 declines.  For q273 that matters:
 *     Rust answers a SelectAutoAbility with select_indices(&[]) while the C fold
 *     of an empty answer is index -1 = DECLINE, which would silently skip the
 *     fired 登場 abilities and make the whole test vacuous.  This file uses
 *     index 0 (PROCEED) on a SelectAutoAbility, matching the peers'
 *     bl_drain_proceed idiom, and says so at the call site.
 *
 * ANTI-VACUITY.  A per_card cluster is a grid of one card x one behaviour, so a
 * single silent no-op makes a whole column of negatives pass for free.  Every
 * negative here therefore carries either (a) an in-test POSITIVE CONTROL -- the
 * identical fixture with the one variable the negative is about restored, which
 * must produce the effect -- or (b) a zone/prompt control proving the mechanism
 * really ran (the cost really moved N cards, at least one prompt was offered,
 * the use was really recorded).  CHECK_PROMPTS counts the prompt still ON SCREEN
 * at the assertion site, not only at answer time.  Section E's Rust cases were
 * audited for this and THREE of them were found VACUOUS (card count == colour
 * count by construction); the EXTRA case added here is the fix.
 *
 * run() is crash ISOLATION, not a hang detector: no watchdog is achievable on
 * this toolchain.  Every drain is capped.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child killed by SIGSEGV/SIGBUS/SIGABRT
 * still reports the assertions it completed before dying.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as the byte escape. */
#define FP "\xef\xbc\x8b"

/* ── fixtures ───────────────────────────────────────────────────────────
 * Identity for every one of these was read out of cards/cards.json read as
 * UTF-8 (never Get-Content -Raw, which mangles non-ASCII and reports false
 * "missing" cards).  cards.json evidence is repeated at each test. */
#define CL003     "PL!HS-cl1-003-CL"          /* 大沢瑠璃乃  みらくらぱーく！ c4  */
#define FILLER    "PL!-sd1-010-SD"           /* 高坂 穂乃果, Printemps, c4     */
#define FILLER2   "PL!-sd1-001-SD"           /* 高坂 穂乃果, Printemps, c11    */
#define HAZUKI    "PL!SP-bp5-005-R" FP       /* 葉月 恋, KALEIDOSCORE          */
#define LIELLA    "PL!SP-bp1-004-R"          /* 平安名すみれ, CatChu! c15       */
#define KANAKO    "PL!N-bp1-012-R" FP        /* 鐘 嵐珠, R3BIRTH, c15          */
#define NIJI_LIVE "PL!N-sd1-025-SD"          /* 虹ヶ咲 live card, s1           */
#define OTHER_LIVE "PL!-sd1-019-SD"          /* START:DASH!! s1, non-虹ヶ咲      */
#define CD        "PL!N-bp7-025-L"           /* Colorful Dreams! s1            */
#define H01       "PL!-sd1-005-SD"           /* base heart01:1  + b_heart01    */
#define H03       "PL!-PR-001-PR"            /* base heart03:2  (no blade heart) */
#define H06       "PL!-sd1-002-SD"           /* base heart06:1  + b_heart06    */
#define H136      "PL!-sd1-003-SD"           /* base heart01:1 03:2 06:2       */
#define B_PEACH   "PL!-bp3-022-L"            /* need-only, b_heart01, s5        */
#define B_BLUE    "PL!N-bp1-027-L"           /* need-only, b_heart05, s0        */
#define B_ALL     "PL!-sd1-020-SD"           /* need-only, b_all,     s2        */
#define B_YELLOW  "PL!-sd1-022-SD"           /* need-only, b_heart03, s4        */
#define NIJI_MEM  "PL!N-sd1-004-SD"          /* 朝香果林, DiverDiva             */
#define NON_NIJI  "PL!-sd1-010-SD"           /* ラブライブ！ only               */
#define MI_LIVE   "PL!-bp3-023-L"            /* ミはμ'sicのミ, s3               */
#define SETSUNA   "PL!N-pb1-007-R"           /* 優木せつ菜, blade 7             */
#define KOTORI7   "PL!-bp6-003-R" FP        /* 南ことり,     blade 7           */
#define WATANABE  "PL!S-bp7-005-R" FP        /* 渡辺 曜, CYARON!, c15           */
#define HANAMARU  "PL!S-PR-019-PR"           /* 国木田花丸, AZALEA, c11         */
#define DECKCARD  "PL!-sd1-001-SD"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── shared counters (survive a child that dies mid-assertion) ───────── */

typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int gaps;
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
    if (tal) {
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s\n"
                "*** The engine, not this test, is at fault.\n"
                "*** Assertions completed before the fault: %d, failures: %d, prompts: %d\n",
                sig, current_test, tal->assertions, tal->failures, tal->prompts);
    } else {
        fprintf(stderr, "\n*** FATAL SIGNAL %d inside test: %s\n", sig, current_test);
    }
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (!(condition)) {                                               \
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

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before a negative claim can be trusted.  The prompt that is still ON SCREEN
 * at the assertion site is counted HERE, because counting only at answer time
 * (inside pick/skip) reports 0 for a prompt that is plainly open. */
#define CHECK_PROMPTS(n, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (tal) tal->prompts++;                                          \
        if (!tal || tal->prompts < (n)) {                                 \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) offered, need %d\n",   \
                    tal ? tal->prompts : -1, (int)(n));                   \
        }                                                                 \
    } while (0)

/* ── card DB + fixtures ───────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static void dump_cond(const char *pfx, const Condition *c, int depth);
static void dump_effect(const char *pfx, const AbilityEffect *e, int depth);

/* Diagnostics at startup: the heartNN -> index map and the group name of every
 * card whose group/series filter the assertions depend on.  Printed, never
 * asserted -- this is what makes a group-filter red attributable. */
static void dump_fixture_abilities(void)
{
    static const char *const cards[] = { HAZUKI, KANAKO, CD, MI_LIVE, WATANABE, HANAMARU };
    for (unsigned i = 0; i < sizeof cards / sizeof cards[0]; i++) {
        int cid = rb_find_card_by_no(cards[i]);
        if (cid < 0) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        fprintf(stderr, "=== decode dump: %s (id=%d, %d abilit%s) ===\n",
                cards[i], cid, n, n == 1 ? "y" : "ies");
        for (int a = 0; a < n; a++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            fprintf(stderr, "  ab#%d triggers=%s use_limit=%d\n", a,
                    ab.triggers ? ab.triggers : "-", ab.use_limit);
            if (ab.cost)   { fprintf(stderr, "  [cost]\n");   dump_effect("  ", ab.cost, 2); }
            if (ab.effect) { fprintf(stderr, "  [effect]\n"); dump_effect("  ", ab.effect, 2); }
            rb_free_ability(&ab);
        }
    }
}

static void print_wire_map(void)
{
    if (getenv("RB2_DUMP")) dump_fixture_abilities();
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++) {
        RbHeartColor hc = rb_parse_heart_color(rb_heart_color_as_str(i));
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), (int)hc);
    }
    fprintf(stderr, "--- group names (rb_card_group_name) ---\n");
    static const char *watch[] = {
        CD, H01, H03, H06, H136, B_PEACH, B_BLUE, B_ALL, B_YELLOW,
        NIJI_MEM, NON_NIJI, KANAKO, NIJI_LIVE, OTHER_LIVE, HAZUKI, LIELLA,
        CL003, WATANABE, HANAMARU
    };
    for (unsigned i = 0; i < sizeof watch / sizeof watch[0]; i++) {
        int id = rb_find_card_by_no(watch[i]);
        if (id < 0) { fprintf(stderr, "    %-20s NOT FOUND\n", watch[i]); continue; }
        fprintf(stderr, "    %-20s id=%-6d group=\"%s\"\n",
                watch[i], id, rb_card_group_name(id));
    }
}

/* A fresh game with the copy pool reset: a long suite must never exhaust the
 * process-global pool and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

static const char *card_no_of(int cid)
{
    static char buf[96];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: byte-exact strcmp of the decoded card_no, because
 * rb_card_no_eq is FUZZY. */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    if (tal) { tal->failures++; tal->setup_bugs++; }
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

/* ── decode dump (diagnostic, off by default) ───────────────────────────
 * RB2_DUMP=1 prints the ENGINE's decoded effect tree for a fixture.  Every
 * engine gap in this file is a SHAPE gap -- "the cost moved 3 cards and the
 * per-unit filter was not applied", "the condition was never checked" -- and
 * a shape claim can only be settled by looking at what the C decoder
 * actually produced, not at what abilities.json says. */
static void dump_cond_fields(const char *pfx, const Condition *c)
{
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        const CondValue *v = &f->v;
        fprintf(stderr, "%s   cond.variant=%u field[%u] %s = ", pfx, c->variant, i, f->key);
        if (v->tag == RB_TAG_STR)            fprintf(stderr, "\"%s\"\n", v->s ? v->s : "(null)");
        else if (v->tag == RB_TAG_I64)       fprintf(stderr, "%lld\n", (long long)v->i);
        else if (v->tag == RB_TAG_ARRAY)     fprintf(stderr, "[array of %u]\n", v->arr_n);
        else                                 fprintf(stderr, "(tag %u)\n", v->tag);
    }
}

static void dump_cond(const char *pfx, const Condition *c, int depth)
{
    if (!c) return;
    fprintf(stderr, "%s  condition(variant=%u) {\n", pfx, c->variant);
    dump_cond_fields(pfx, c);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_ARRAY) {
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                fprintf(stderr, "%s     %s[%u] tag=%u s=%s cond=%s\n", pfx, f->key, j,
                        f->v.arr[j].tag, f->v.arr[j].s ? f->v.arr[j].s : "(null)",
                        f->v.arr[j].cond ? "DECODED" : "NULL");
                if (f->v.arr[j].cond) {
                    for (int k = 0; k < depth + 4; k++) fputc(' ', stderr);
                    dump_cond(pfx, f->v.arr[j].cond, depth + 2);
                }
            }
        }
        if (f->v.cond) {
            for (int k = 0; k < depth + 3; k++) fputc(' ', stderr);
            dump_cond(pfx, f->v.cond, depth + 1);
        }
    }
    fprintf(stderr, "%s  }\n", pfx);
}

static void dump_effect(const char *pfx, const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int k = 0; k < depth; k++) fputc(' ', stderr);
    fprintf(stderr, "%s effect action=\"%s\" count=%d per_unit=%d per_unit_count=%d "
                    "n_child=%d has_condition=%d src=%s dst=%s target=%s\n",
            pfx, e->action ? e->action : "(none)", e->count, e->per_unit,
            e->per_unit_count, e->n_child, e->has_condition,
            e->source ? e->source : "-", e->destination ? e->destination : "-",
            e->target ? e->target : "-");
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            fprintf(stderr, "%s   extra[%d] %s = %s\n", pfx, i, e->extra_k[i],
                    e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition && e->condition) dump_cond(pfx, e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++) dump_effect(pfx, e->child[i], depth + 2);
    for (int i = 0; i < e->n_options; i++) dump_effect(pfx, e->options[i], depth + 2);
}

/* Rust game.new_id("X") -> a distinct pool slot.  C twin: test_new_id. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* Rust game.id("X") -> the shared template slot.  C twin: test_id. */
static int sid(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

static int printed_cost(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static int printed_score(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
    rb_free_card(&c);
    return v;
}

static int printed_blade(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.blade;
    rb_free_card(&c);
    return v;
}

/* The decoded heart table is one flat (colour, count) list, MEASURED per card
 * type (see test_parity_look_per_card.c):
 *   MEMBER  n_hearts = num_base + num_blade   [0,num_base) base, rest blade
 *   LIVE    n_hearts = num_blade + num_need   [0,num_blade) blade, rest need
 * The BLADE block is FIRST for a live card and LAST for a member.  Summing the
 * WHOLE table folds a blade heart into the requirement, so only the base range
 * is read here -- which is the whole point of Q271. */
static int printed_base_heart(int cid, int color)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int total = 0;
    for (int i = 0; i < c.num_base && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

static int printed_blade_heart(int cid, int color)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int lo, hi;
    if (rb_card_is_live(cid)) { lo = 0; hi = c.num_blade; }  /* blade block FIRST */
    else                      { lo = c.num_base; hi = c.n_hearts; }
    int total = 0;
    for (int i = lo; i < hi && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == color) total += c.heart_count[i];
    rb_free_card(&c);
    return total;
}

static int card_prints_trigger(int cid, const char *trig)
{
    if (cid < 0) return 0;
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

static int card_trigger_index(int cid, const char *trig)
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

static int blade_of(const TestGame *tg, int cid)
{
    return rb_mods_get_blade((RbMods *)&tg->state.mods, cid);
}

static int score_of(const TestGame *tg, int cid)
{
    return rb_mods_get_score((RbMods *)&tg->state.mods, cid);
}

static int need_heart_of(const TestGame *tg, int cid, int color)
{
    return rb_mods_get_need_heart((RbMods *)&tg->state.mods, cid, color);
}

static const char *orientation_of(const TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation((RbMods *)&tg->state.mods, cid);
    return o ? o : "";
}

static int is_wait(const TestGame *tg, int cid)
{
    return strcmp(orientation_of(tg, cid), "wait") == 0;
}

/* ── board helpers ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

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

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg) || n <= 0) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

static void skip(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, -1);
}

/* helpers/mod.rs: `while has_pending_choice() { select_indices(&[0]) }` --
 * the unconditional index 0, which in the C engine means ACCEPT / PROCEED. */
static int drain_pick_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

/* Same, but a SelectAutoAbility is DECLINED first (index -1). */
static int drain_decline_auto(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) skip(tg);
        else pick(tg, 0);
    }
    return guard;
}

static int hand_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == cid) return 1;
    return 0;
}

static int wait_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        if (tg->state.p[0].discard.cards[i] == cid) return 1;
    return 0;
}

static int wait_count(const TestGame *tg)
{
    return tg->state.p[0].discard.n;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void set_stage_wait(TestGame *tg, int pl, int area, int wait)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage_wait[area] = wait;
}

static void live_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->live.n < RB_MAX_ZONE) P->live.cards[P->live.n++] = cid;
}

static void wait_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void hand_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

/* Rust `main_deck.cards.push(x)` -- an APPEND, so x lands at the BOTTOM. */
static void deck_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    for (int i = 0; i < n; i++) deck_append(tg, pl, cid);
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        deck_append_n(tg, pl, filler, n);
    }
}

static int deck_len(const TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }

static void revealed_add(TestGame *tg, int cid)
{
    if (tg->state.n_revealed < RB_MAX_REVEALED_CARDS)
        tg->state.revealed_cards[tg->state.n_revealed++] = cid;
}

/* helpers/mod.rs:114 fire_trigger -- resolve ONE ability on ONE card, key it
 * "{card_no}_{full_text}", fire it with `cid` as the activating card, then
 * process the queue.
 *
 * The body used to be hand-rolled over a raw rb_trigger_auto_ability call,
 * which had two real defects:
 *   1. NO ENQUEUE-TIME USE RECORD. rb_trigger_auto_ability builds its entry
 *      with `use_limit_recorded = 0`, so a 自動/ターン1回 ability is still
 *      unspent while it resolves and the post-resolution re-scan in
 *      rb_process_player_abilities_depth_full enqueues it a SECOND time.
 *   2. It has to keep its own copy of the "{card_no}_{full_text}" key, and the
 *      copy silently TRUNCATES at 512 bytes — a long full_text produced a
 *      truncated ability id, which the engine then failed to match.
 * test_fire_trigger_on_card is the shared shape and does both correctly; it
 * also returns 1 only when one entry really was produced, so the suite's
 * `if (!fire_trigger(...))` premise check keeps its meaning. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* helpers/mod.rs:114 fire_trigger with a SEAT argument (used where a 起動
 * fires an opponent's 登場). test_fire_trigger_on_card_seat is the shared
 * seat-aware shape — same use record, same owner stamp, same size-safe key. */
static int fire_trigger_for(TestGame *tg, int pl, int cid, const char *trig,
                            const char *trigger_type)
{
    (void)trigger_type;
    return test_fire_trigger_on_card_seat(tg, cid, trig, pl);
}

/* Rust `game.state.activating_card = Some(cid)` + process_pending_auto_abilities
 * WITHOUT queueing anything -- used to prove the queue, not the activation, is
 * what makes an effect run. */
static void reprocess_only(TestGame *tg, int cid)
{
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
}

/* Rust advance_to_live_card_set: 5 blind passes with NO drain -- the same
 * shape as test_parity_blades_live_start.c's bl_advance_to_live_set.  A drain
 * here is actively WRONG: the live-set window raises the set-live prompt, and
 * answering it with index 0 would place a card the test did not choose. */
static void advance_to_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust advance_to_live_start: 2 more bare passes. */
static void advance_to_live_start(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* The heartNN -> RbHeartColor map, derived at run time from the engine's own
 * rb_heart_color_as_str / rb_parse_heart_color rather than hard-coded, because
 * a peer found a SHIPPED test asserting heart06 through RB_HEART_PURPLE.  The
 * map is PRINTED at startup (print_wire_map) so a mismatch is visible in the
 * log instead of hiding inside an arithmetic result. */
static int HC[7];

static void hcolor_init(void)
{
    for (int i = 0; i < 7; i++)
        HC[i] = (int)rb_parse_heart_color(rb_heart_color_as_str(i));
}

static int distinct_base_colors(const int *cards, int n)
{
    int seen[16] = { 0 };
    for (int c = 0; c < n; c++) {
        Card card;
        if (cards[c] < 0 || !rb_decode_card_by_index((uint32_t)cards[c], &card)) continue;
        for (int i = 0; i < card.num_base && i < card.n_hearts; i++) {
            int col = (int)card.heart_color[i];
            if (col >= 0 && col < 16 && card.heart_count[i] > 0) seen[col] = 1;
        }
        rb_free_card(&card);
    }
    int k = 0;
    for (int c = 0; c < 16; c++) k += seen[c];
    return k;
}

/* ═══════════════════════════════════════════════════════════════════════════
 * A. activate_change_state_wait_grants_live_end_blade_test.rs
 *    PL!HS-cl1-003-CL 大沢瑠璃乃 (cards.json: series
 *    ラブライブ！蓮ノ空女学院スクールアイドルクラブ, unit みらくらぱーく！,
 *    cost 4, blade 2)
 *
 *    ab#0 起動/1
 *      cost:   change_state{state_change:"wait", self_cost, member_card}
 *      effect: gain_resource{blade 1, duration live_end, member_card,
 *                           target self, target_count 1,
 *                           group_names ["みらくるーぱー！"]}
 *
 *    The 起動 is the card's ONLY 起動, so rb_activate_card's first-起動 scan
 *    lands on it.  Both Rust tests use the SAME fixture; the first asserts
 *    cost + effect, the second cost only.
 * ═════════════════════════════════════════════════════════════════════════ */

static void a_cl_cost_pay_wait_gain_blade(void)
{
    static TestGame tg;
    fresh(&tg);
    int card = sid(&tg, CL003);
    if (!pin_id(card, CL003)) return;
    CHECK_EQ(printed_cost(card), 4, "premise: 大沢瑠璃乃 costs 4 (cards.json cost=4)");
    /* Decode pin: the 起動 really is present in the compiled record, so a
     * refusal below is a payment failure and not a missing-ability artefact. */
    fprintf(stderr, "        [info] PL!HS-cl1-003-CL: abilities in the compiled "
                    "record = %d, group = \"%s\"\n",
            rb_card_num_abilities((uint32_t)card), rb_card_group_name(card));
    CHECK(card_prints_trigger(card, RB_TSTR_ACTIVATION),
          "premise: the card really prints an 起動 ability in the compiled record");
    CHECK_EQ(card_trigger_index(card, RB_TSTR_ACTIVATION), 0,
             "premise: ab#0 is the only 起動, so rb_activate_card's scan lands on it");
    /* rb_card_group_name returns a SERIES-level group, not the unit: the print
     * is みらくらぱーく！ but the engine reports 蓮ノ空 (see the startup
     * [group names] dump).  Recorded as a fact, not asserted as a premise --
     * see the report for what that means for the みらくらぱーく！ filter. */
    fprintf(stderr, "        [info] PL!HS-cl1-003-CL group = \"%s\"\n",
            rb_card_group_name(card));

    set_stage(&tg, 0, AREA_CENTER, card);
    int ok = test_activate_ability(&tg, card);
    CHECK(ok, "ab#0 is accepted from the centre");
    drain_pick_first(&tg, 32);

    CHECK(is_wait(&tg, card),
          "Rust cl_gain_resource_pay_cost_wait_gain_blade: the change_state(wait) cost is paid");
    CHECK(blade_of(&tg, card) >= 1,
          "Rust cl_gain_resource_pay_cost_wait_gain_blade: the card has at least 1 blade");
    /* target_count is 1, so exactly one みらくるーぱー！ member gains. */
    CHECK_EQ(blade_of(&tg, card), 1,
             "exactly one blade: 「メンバー1人」 is target_count 1, not all");
}

static void a_cl_cost_pays_without_effect_target(void)
{
    /* Rust cl_gain_resource_cost_pays_when_no_condition_target: the same
     * fixture, asserting only that the cost is UNCONDITIONAL. */
    static TestGame tg;
    fresh(&tg);
    int card = sid(&tg, CL003);
    if (!pin_id(card, CL003)) return;
    set_stage(&tg, 0, AREA_CENTER, card);
    test_activate_ability(&tg, card);
    drain_pick_first(&tg, 32);
    CHECK(is_wait(&tg, card),
          "Rust cl_gain_resource_cost_pays_when_no_condition_target: the cost is paid "
          "even when the effect has nobody to target");

    /* ANTI-VACUITY CONTROL.  The assertion above is a negative about the COST,
     * and a card whose 起動 never ran at all would satisfy it.  The control is
     * the identical board: if the engine really executes this 起動, the blade
     * is granted. */
    static TestGame ctl;
    fresh(&ctl);
    int card2 = sid(&ctl, CL003);
    if (!pin_id(card2, CL003)) return;
    set_stage(&ctl, 0, AREA_CENTER, card2);
    int ok2 = test_activate_ability(&ctl, card2);
    drain_pick_first(&ctl, 32);
    CHECK(ok2 && is_wait(&ctl, card2),
          "ANTI-VACUITY CONTROL: the same 起動 activates and pays on a fresh board");
    CHECK_EQ(blade_of(&ctl, card2), 1,
             "ANTI-VACUITY CONTROL: the 起動 really grants a blade when the group filter matches");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * B. per_unit_discard_count_regression_test.rs
 *
 *    The Rust test claims to guard "per_unit effects must not multiply discard
 *    counts" but its fixture never enters a draw at all: TestGame::new leaves
 *    the game in Main, and TurnEngine::advance_phase from Main goes to the
 *    live-card-set window, not to a draw.  Both halves are ported -- the
 *    faithful blind advance, and an EXPLICIT draw-phase drive that really
 *    draws, so the assertion is not vacuous.
 * ═════════════════════════════════════════════════════════════════════════ */

static void b_draw_phase_discards_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int filler_a = sid(&tg, FILLER);
    int filler_b = mid(&tg, FILLER);
    if (!pin_id(filler_a, FILLER)) return;
    set_stage(&tg, 0, AREA_LEFT, filler_a);
    set_stage(&tg, 0, AREA_CENTER, filler_b);
    tg.state.p[0].hand.n = 0;
    tg.state.p[0].deck.n = 0;
    deck_append_n(&tg, 0, mid(&tg, FILLER), 30);
    deck_append_n(&tg, 1, mid(&tg, FILLER), 30);

    const int before = wait_count(&tg);
    const int census0 = test_total_card_count(&tg);
    const int ph0 = (int)tg.state.phase;
    rb_advance_phase(&tg.state);
    drain_pick_first(&tg, 16);
    CHECK((int)tg.state.phase != ph0,
          "the phase really advanced, so the step under test is not a no-op");
    CHECK_EQ(wait_count(&tg), before,
             "Rust test_per_unit_discard_bug_fix: the draw step discards nothing");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "no card was created or destroyed by the step");

    /* EXPLICIT draw-phase drive -- the part the Rust fixture never reaches.
     * rb_advance_phase ENTERS RB_PHASE_DRAW without drawing, and the draw runs
     * on the Active/Energy/Draw triple, so the walk starts at RB_PHASE_ACTIVE
     * and runs to the next Main.  The active seat is swapped to
     * second_attacker BEFORE that first triple, so the first draw of a round is
     * served to the OTHER seat -- both seats are checked. */
    tg.state.p[0].hand.n = 0;
    tg.state.p[1].hand.n = 0;
    const int wait1 = wait_count(&tg);
    const int deck1 = deck_len(&tg, 0) + deck_len(&tg, 1);
    tg.state.phase = RB_PHASE_ACTIVE;
    test_advance_to_phase(&tg, RB_PHASE_MAIN);
    drain_pick_first(&tg, 16);
    CHECK(tg.state.p[0].hand.n + tg.state.p[1].hand.n > 0,
          "ANTI-VACUITY CONTROL: a real draw landed a card in some hand");
    CHECK(deck_len(&tg, 0) + deck_len(&tg, 1) < deck1,
          "ANTI-VACUITY CONTROL: the deck really shrank");
    CHECK_EQ(wait_count(&tg), wait1,
             "a real draw sends the card to hand, never to the waitroom");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * C. hazuki_activate_discard_per_liella_member_blade_test.rs
 *    PL!SP-bp5-005-R＋ 葉月 恋 (cards.json: unit KALEIDOSCORE, cost 11)
 *
 *    ab#0 起動/1
 *      cost:   move_cards{deck_top -> discard, count 3}
 *      effect: gain_resource{blade 1, per_unit, per_unit_count 1,
 *                           per_unit_type "discard", per_unit_source
 *                           "previous_moved_cards", member_card,
 *                           group_names ["Liella!"], duration live_end}
 *
 *    This is the per_unit-over-a-just-moved-set shape, NOT the waitroom scan:
 *    the counted set is `previous_moved_cards` (the three the cost moved), and
 *    the filter is 『Liella!』.  The negative cases are each paired with the
 *    same helper driven on the positive board, so a silent no-op 起動 cannot
 *    make them pass.
 * ═════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int accepted;
    int wait_n;
    int deck_moved;
    int blade;
    int hazuki;
} HazRun;

/* `top[0..n_top-1]` is the deck in DRAW order (top[0] is taken first).
 * The C deck bag is TOP-INDEX-0, so the cards are inserted at 0 in reverse. */
static HazRun hazuki_run(TestGame *tg, const char *const *top, int n_top, int energy)
{
    HazRun r;
    r.accepted = 0; r.wait_n = -1; r.deck_moved = -1; r.blade = -1; r.hazuki = -1;
    fresh(tg);
    int hazuki = mid(tg, HAZUKI);
    if (!pin_id(hazuki, HAZUKI)) return r;
    r.hazuki = hazuki;
    set_stage(tg, 0, AREA_CENTER, hazuki);
    tg->state.p[0].hand.n = 0;
    hand_append(tg, 0, mid(tg, FILLER));
    tg->state.p[0].deck.n = 0;
    for (int i = n_top - 1; i >= 0; i--) test_insert_deck_top(tg, 0, mid(tg, top[i]));
    test_give_energy(tg, energy);

    int deck_before = deck_len(tg, 0);
    r.accepted = test_activate_ability(tg, hazuki);
    drain_pick_first(tg, 32);
    r.wait_n = wait_count(tg);
    r.deck_moved = deck_before - deck_len(tg, 0);
    r.blade = blade_of(tg, hazuki);
    /* The three numbers that separate "the count is the number of MOVED cards"
     * from "the count is the number of matching 『Liella!』 members in the moved
     * set" (Rust) from "the per-unit filter collapsed to 1" (dynamic_count.c has
     * no `discard` arm, so per_unit_type "discard" falls through to units = 1 and
     * per_unit_source "previous_moved_cards" becomes the whole moved set). */
    fprintf(stderr, "        [info] HAZUKI top[0]=%s top[1]=%s | accepted=%d "
                    "deck_moved=%d wait_n=%d blade=%d\n",
            n_top > 0 ? top[0] : "-", n_top > 1 ? top[1] : "-",
            r.accepted, r.deck_moved, r.wait_n, r.blade);
    return r;
}

static void c_hazuki_two_liella_discarded_gain_two_blade(void)
{
    static TestGame tg;
    static TestGame ctl;
    const char *top[3] = { LIELLA, LIELLA, FILLER };
    /* ANTI-VACUITY CONTROL FIRST: the identical helper on the positive board.
     * If this is red, every negative in this section proves nothing. */
    HazRun pos = hazuki_run(&ctl, top, 3, 13);
    if (pos.hazuki < 0) return;
    CHECK_EQ(pos.blade, 2,
             "ANTI-VACUITY CONTROL: 2 『Liella!』 members discarded -> 2 blades");
    /* Zone control: the cost really moved the three cards.  Without it a free
     * blade grant would satisfy the Rust assertion. */
    CHECK_EQ(pos.deck_moved, 3, "CONTROL: the deck-top cost really moved 3 cards");
    CHECK_EQ(pos.wait_n, 3, "CONTROL: all three landed in the waitroom");

    HazRun r = hazuki_run(&tg, top, 3, 13);
    if (r.hazuki < 0) return;
    CHECK_EQ(r.blade, 2,
             "Rust hazuki_activate_two_liella_discarded_gain_2_blade: "
             "2 『Liella!』 members discarded -> 2 blade on the activating card");
    CHECK_EQ(r.deck_moved, 3, "the cost paid: 3 cards left the deck");
    CHECK_EQ(r.wait_n, 3, "the cost's 3 cards are in the waitroom");
}

static void c_hazuki_no_liella_discarded_gain_zero_blade(void)
{
    static TestGame tg;
    static TestGame ctl;
    const char *none[3] = { FILLER, FILLER, FILLER };
    const char *two[3]  = { LIELLA, LIELLA, FILLER };

    HazRun pos = hazuki_run(&ctl, two, 3, 13);
    if (pos.hazuki < 0) return;
    CHECK_EQ(pos.blade, 2,
             "ANTI-VACUITY CONTROL: with 2 『Liella!』 in the discarded set the 起動 does grant 2");

    HazRun r = hazuki_run(&tg, none, 3, 13);
    if (r.hazuki < 0) return;
    CHECK_EQ(r.deck_moved, 3,
             "CONTROL: the cost really ran (3 cards moved) on the negative board too");
    CHECK_EQ(r.blade, 0,
             "Rust hazuki_activate_no_liella_discarded_gain_0_blade: "
             "0 『Liella!』 members discarded -> 0 blade");
}

static void c_hazuki_not_enough_deck_no_blade(void)
{
    static TestGame tg;
    static TestGame ctl;
    const char *two[3] = { LIELLA, LIELLA, FILLER };

    HazRun pos = hazuki_run(&ctl, two, 3, 13);
    if (pos.hazuki < 0) return;
    CHECK_EQ(pos.blade, 2, "ANTI-VACUITY CONTROL: a 3-card deck grants 2 blades");

    static const char *empty[1] = { FILLER };
    HazRun r = hazuki_run(&tg, empty, 0, 13);   /* deck cleared */
    if (r.hazuki < 0) return;
    CHECK_EQ(deck_len(&tg, 0), 0, "CONTROL: the deck really is empty");
    CHECK_EQ(r.blade, 0,
             "Rust hazuki_activate_not_enough_deck_no_blade: "
             "cost failed -> no blade applied");
    /* The Rust ignores the activation result; the C reports one, and an
     * unpayable cost must not report success. */
    CHECK(!r.accepted || r.deck_moved == 0,
          "an empty deck is either refused or moves nothing -- never both "
          "accepted and paid");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * D. q46_kanako_all_heart_timing_test.rs
 *    PL!N-bp1-012-R＋ 鐘 嵐珠 (cards.json: series 虹ヶ咲…, unit R3BIRTH,
 *    cost 15, blade 4, base heart01:3 heart04:2 heart06:1)
 *
 *    ab#0 常時
 *      condition: compound( card_count(live_card_zone, >=3, self)
 *                      AND card_count(live_card_zone, >=1, live_card,
 *                                     group_names ["虹ヶ咲"]) )
 *      effect:    sequential[ gain_resource(blade 2),
 *                             gain_resource(heart all 2) ]
 *
 *    This is the CONSTANT path (rb_recalc_constants) with a zone + group
 *    condition, and the effect is a `sequential`, not a bare gain_resource.
 * ═════════════════════════════════════════════════════════════════════════ */

/* Board: stage = [filler, kanako, filler2] (or [filler, EMPTY, filler2] when
 * `kanako_on_stage` is 0), live zone = `n_iji_live` 虹ヶ咲 lives followed by
 * non-虹ヶ咲 lives up to `n_live_total`.  Returns Kanako's blade after
 * recalculate_constants. */
static int q46_run(TestGame *tg, int n_live_total, int n_iji_live, int kanako_on_stage)
{
    fresh(tg);
    int kanako = mid(tg, KANAKO);
    if (!pin_id(kanako, KANAKO)) return -1;
    CHECK_EQ(printed_cost(kanako), 15, "premise: 鐘 嵐珠 costs 15 (cards.json cost=15)");

    int filler_a = mid(tg, FILLER);
    int filler_b = mid(tg, FILLER);
    set_stage(tg, 0, AREA_LEFT, filler_a);
    set_stage(tg, 0, AREA_CENTER, kanako_on_stage ? kanako : RB_EMPTY_SLOT);
    set_stage(tg, 0, AREA_RIGHT, filler_b);

    for (int i = 0; i < n_iji_live; i++) {
        int lv = mid(tg, NIJI_LIVE);
        if (i == 0 && !pin_id(lv, NIJI_LIVE)) return -1;
        live_append(tg, 0, lv);
    }
    for (int i = n_iji_live; i < n_live_total; i++) {
        int lv = mid(tg, OTHER_LIVE);
        if (i == n_iji_live && !pin_id(lv, OTHER_LIVE)) return -1;
        live_append(tg, 0, lv);
    }

    /* PREMISE: the 虹ヶ咲 live and START:DASH!! really are different groups,
     * because the whole test is one group's presence gating the constant. */
    if (n_iji_live > 0) {
        int ij = mid(tg, NIJI_LIVE), ot = mid(tg, OTHER_LIVE);
        CHECK(strcmp(rb_card_group_name(ij), rb_card_group_name(ot)) != 0,
              "premise: the 虹ヶ咲 live and START:DASH!! have different group names "
              "(see the [group names] block in the startup dump)");
    }
    /* PREMISE, corrected after a first run.  rb_card_group_name returns a
     * SERIES-level group, not the unit: Kanako (unit R3BIRTH) and the
     * PL!N-sd1-025-SD live card (series 虹ヶ咲…) BOTH report "虹ヶ咲", so
     * "Kanako's own group differs" is FALSE and asserting it was a test bug.
     * The premise that actually matters is that the 虹ヶ咲 live differs from
     * the non-虹ヶ咲 one -- checked above. */
    if (kanako_on_stage) {
        int ij = mid(tg, NIJI_LIVE);
        fprintf(stderr, "        [info] Kanako group = \"%s\", 虹ヶ咲 live group = \"%s\"\n",
                rb_card_group_name(kanako), rb_card_group_name(ij));
    }

    test_recalc(tg);
    return blade_of(tg, kanako);
}

static void d_q46_condition_met_grants_two_blades(void)
{
    static TestGame tg;
    int blade = q46_run(&tg, 3, 1, 1);
    CHECK_EQ(blade, 2,
             "Rust q46_kanako_constant_grants_blades_when_condition_met: "
             "3 live cards (1 虹ヶ咲) -> the 常時 fires -> +2 blades");
    /* EXTRA (not asserted in the Rust test, but it is the SECOND child of the
     * same `sequential`, and the file's own doc comment -- "when is the colour
     * of the ALL hearts decided?" -- is about it).  Tracked separately so a red
     * here is never confused with the blade assertion above.  RB_HEART_ALL is
     * the engine's own enum constant, not an HC index: HC[] only covers 0..6. */
    int kanako = tg.state.p[0].stage[AREA_CENTER];
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, kanako, RB_HEART_ALL), 2,
             "EXTRA (not asserted in Rust): the same sequential also grants 2 ALL "
             "hearts to the staged member");
    /* DIAGNOSTIC: does the compound condition gate the constant at all?  With
     * NO live cards in the zone the printed condition (>= 3 live, >= 1 虹ヶ咲)
     * is false, so the blade must be 0.  If this is 2 the condition is not
     * evaluated on the 常時 sequential path at all. */
    {
        static TestGame diag;
        int b = q46_run(&diag, 0, 0, 1);
        CHECK_EQ(b, 0,
                 "DIAGNOSTIC: with an EMPTY live zone the compound condition must be "
                 "false and no constant may be granted");
    }
}

/* The two-run helper so every negative in this section carries its own
 * positive control on the IDENTICAL board shape. */
static void d_q46_pair(int n_live_total, int n_iji_live, int kanako_on_stage,
                       int expected_positive, int expected_negative,
                       const char *what)
{
    static TestGame ctl, neg;
    int pos = q46_run(&ctl, 3, 1, 1);          /* the met-condition board */
    CHECK_EQ(pos, expected_positive,
             "ANTI-VACUITY CONTROL: the met-condition board grants the printed 2 blades");
    int got = q46_run(&neg, n_live_total, n_iji_live, kanako_on_stage);
    CHECK_EQ(got, expected_negative, what);
}

static void d_q46_less_than_three_live_cards_no_gain(void)
{
    d_q46_pair(2, 1, 1, 2, 0,
               "Rust q46_kanako_condition_less_than_3_live_cards_no_gain: "
               "< 3 live cards -> no blade gain");
}

static void d_q46_no_nijigasaki_live_no_gain(void)
{
    d_q46_pair(3, 0, 1, 2, 0,
               "Rust q46_kanako_no_nijigasaki_live_card_no_gain: "
               "3 live cards but no 虹ヶ咲 -> no blade gain");
}

static void d_q46_not_on_stage_no_constant(void)
{
    d_q46_pair(3, 1, 0, 2, 0,
               "Rust q46_kanako_not_on_stage_no_constant: "
               "Kanako off stage -> the constant is not evaluated");
}

static void d_q46_leaves_stage_blade_removed(void)
{
    static TestGame tg;
    fresh(&tg);
    int kanako = mid(&tg, KANAKO);
    if (!pin_id(kanako, KANAKO)) return;
    int f1 = mid(&tg, FILLER), f2 = mid(&tg, FILLER);
    set_stage(&tg, 0, AREA_LEFT, f1);
    set_stage(&tg, 0, AREA_CENTER, kanako);
    set_stage(&tg, 0, AREA_RIGHT, f2);
    live_append(&tg, 0, mid(&tg, NIJI_LIVE));
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));
    CHECK_EQ(tg.state.p[0].live.n, 3, "setup: exactly 3 live cards, 1 of them 虹ヶ咲");

    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, kanako), 2,
             "Rust q46_kanako_leaves_stage_blade_removed (before): condition met -> +2 blades");

    int f3 = mid(&tg, FILLER);
    set_stage(&tg, 0, AREA_LEFT, f1);
    set_stage(&tg, 0, AREA_CENTER, RB_EMPTY_SLOT);
    set_stage(&tg, 0, AREA_RIGHT, f3);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "setup: Kanako really left the centre slot");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, kanako), 0,
             "Rust q46_kanako_leaves_stage_blade_removed (after): "
             "Kanako leaves stage -> the constant is removed -> 0 blades");
}

static void d_q46_live_card_removed_blade_removed(void)
{
    static TestGame tg;
    fresh(&tg);
    int kanako = mid(&tg, KANAKO);
    if (!pin_id(kanako, KANAKO)) return;
    int f1 = mid(&tg, FILLER), f2 = mid(&tg, FILLER);
    set_stage(&tg, 0, AREA_LEFT, f1);
    set_stage(&tg, 0, AREA_CENTER, kanako);
    set_stage(&tg, 0, AREA_RIGHT, f2);
    int niji = mid(&tg, NIJI_LIVE);
    pin_id(niji, NIJI_LIVE);
    live_append(&tg, 0, niji);
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));

    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, kanako), 2,
             "Rust q46_live_card_removed_condition_fails_blade_removed (before): +2");

    tg.state.p[0].live.n = 0;
    live_append(&tg, 0, niji);
    CHECK_EQ(tg.state.p[0].live.n, 1, "setup: only the 虹ヶ咲 live is left");
    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, kanako), 0,
             "Rust q46_live_card_removed_condition_fails_blade_removed (after): "
             "live cards removed -> < 3 -> condition fails -> blade removed");
}

static void d_q46_two_kanako_each_gains(void)
{
    static TestGame tg;
    fresh(&tg);
    int k1 = mid(&tg, KANAKO);
    int k2 = mid(&tg, KANAKO);
    if (!pin_id(k1, KANAKO)) return;
    if (!pin_id(k2, KANAKO)) return;
    CHECK(k1 != k2, "setup: two DISTINCT instances of the print (rb_create_card_copy)");
    /* Both instances must carry the same printed name, else this is not the
     * "two copies of one card" case the Rust means. */
    {
        Card a, b;
        int same = 0;
        if (rb_decode_card_by_index((uint32_t)k1, &a) &&
            rb_decode_card_by_index((uint32_t)k2, &b) && a.name && b.name)
            same = !strcmp(a.name, b.name);
        if (a.name) rb_free_card(&a);
        if (b.name) rb_free_card(&b);
        CHECK(same, "setup: both copies carry the same printed Card.name (鐘 嵐珠)");
    }
    set_stage(&tg, 0, AREA_LEFT, k1);
    set_stage(&tg, 0, AREA_CENTER, k2);
    set_stage(&tg, 0, AREA_RIGHT, mid(&tg, FILLER));
    live_append(&tg, 0, mid(&tg, NIJI_LIVE));
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));
    live_append(&tg, 0, mid(&tg, OTHER_LIVE));

    test_recalc(&tg);
    CHECK_EQ(blade_of(&tg, k1), 2,
             "Rust q46_multiple_kanako_each_gains_blades: the first Kanako gains 2 blades");
    CHECK_EQ(blade_of(&tg, k2), 2,
             "Rust q46_multiple_kanako_each_gains_blades: the second Kanako gains 2 blades");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * E. q271_colorful_dreams_test.rs
 *    PL!N-bp7-025-L Colorful Dreams! Colorful Smiles! (cards.json: series
 *    虹ヶ咲…, score 1, need heart01:1 heart03:1 heart05:1 heart0:1,
 *    blade_heart b_all:1)
 *
 *    ab#0 ライブ開始時  gain_resource{blade 1, duration live_end,
 *                                  member_card, group_names ["虹ヶ咲"],
 *                                  target_count 1}
 *    ab#1 ライブ成功時  modify_score +1, self, condition
 *        card_count{count 3, ">=", unit "types", location "revealed_cards",
 *                   heart_colors [heart01..heart06]}
 *
 *    Official QA Q271: 桃/青/ALL blade-hearts revealed by the yell do NOT
 *    satisfy the "3+ heart-colour types" condition, because only BASE heart
 *    colours count.  That is exactly what the premise pins below prove, read
 *    off the ENGINE's own decoded heart table rather than off cards.json.
 * ═════════════════════════════════════════════════════════════════════════ */

static void e_q271_pin_premises(TestGame *tg)
{
    static const char *const cards[] = { H01, H03, H06, H136,
                                         B_PEACH, B_BLUE, B_ALL, B_YELLOW };
    int ids[8];
    for (int i = 0; i < 8; i++) {
        ids[i] = sid(tg, cards[i]);
        if (!pin_id(ids[i], cards[i])) return;
    }
    /* The three single-base-colour members really carry heart01/03/06. */
    CHECK_EQ(printed_base_heart(ids[0], HC[1]), 1,
             "premise: PL!-sd1-005-SD has base heart01 x1 (cards.json base_heart heart01:1)");
    CHECK_EQ(printed_base_heart(ids[1], HC[3]), 2,
             "premise: PL!-PR-001-PR has base heart03 x2 (cards.json base_heart heart03:2)");
    CHECK_EQ(printed_base_heart(ids[2], HC[6]), 1,
             "premise: PL!-sd1-002-SD has base heart06 x1 (cards.json base_heart heart06:1)");

    int trio[3] = { ids[0], ids[1], ids[2] };
    CHECK_EQ(distinct_base_colors(trio, 3), 3,
             "premise: the three reveals carry exactly 3 DISTINCT base heart colours");
    CHECK_EQ(distinct_base_colors(&ids[3], 1), 3,
             "premise: PL!-sd1-003-SD alone carries 3 distinct base heart colours");

    /* The four blade-only lives really carry NO base heart at all -- this is
     * the load-bearing premise of Q271 and it must be read from the engine's
     * decoded table, not from the print. */
    for (int i = 4; i < 8; i++) {
        int base = 0;
        for (int c = 1; c <= 6; c++) base += printed_base_heart(ids[i], HC[c]);
        CHECK_EQ(base, 0,
                 "premise: the blade-only live really contributes 0 base heart colours");
    }
    /* ...and each of them does carry a BLADE heart, so the negatives below are
     * not passing because the fixture is empty of hearts altogether. */
    for (int i = 4; i < 8; i++) {
        Card card;
        int has = 0;
        if (rb_decode_card_by_index((uint32_t)ids[i], &card)) {
            has = rb_card_has_blade_heart_strict(&card);
            rb_free_card(&card);
        }
        CHECK(has, "premise: the blade-only live really does carry a blade heart");
    }
}

/* Rust q271 trigger_success: put CD in P1's live zone, add `n` cards to
 * revealed_cards, fire ab#1, return CD's score modifier. */
static int q271_success(TestGame *tg, const int *revealed, int n)
{
    fresh(tg);
    int cd = sid(tg, CD);
    if (!pin_id(cd, CD)) return -1;
    e_q271_pin_premises(tg);
    if (card_trigger_index(cd, RB_TSTR_LIVE_SUCCESS) < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: PL!N-bp7-025-L has no live-success ability\n");
        return -1;
    }
    live_append(tg, 0, cd);
    for (int i = 0; i < n; i++) revealed_add(tg, revealed[i]);
    if (!fire_trigger(tg, cd, RB_TSTR_LIVE_SUCCESS, RB_TSTR_LIVE_SUCCESS)) return -1;
    drain_pick_first(tg, 16);
    return score_of(tg, cd);
}

/* The positive twin, run in every negative below.  Returns the +1, or -1 on a
 * setup failure. */
static int q271_positive_control(TestGame *tg)
{
    int ids[3];
    fresh(tg);
    ids[0] = sid(tg, H01);
    ids[1] = sid(tg, H03);
    ids[2] = sid(tg, H06);
    if (ids[0] < 0 || ids[1] < 0 || ids[2] < 0) return -1;
    return q271_success(tg, ids, 3);
}

static void e_q271_three_distinct_base_hearts_scores(void)
{
    static TestGame tg;
    int ids[3];
    fresh(&tg);
    ids[0] = sid(&tg, H01);
    ids[1] = sid(&tg, H03);
    ids[2] = sid(&tg, H06);
    CHECK_EQ(q271_success(&tg, ids, 3), 1,
             "Rust q271_three_distinct_base_hearts_scores: "
             "3 distinct base heart colours -> +1 score");
}

static void e_q271_one_card_with_three_base_hearts_scores(void)
{
    static TestGame tg;
    int ids[1];
    fresh(&tg);
    ids[0] = sid(&tg, H136);
    CHECK_EQ(q271_success(&tg, ids, 1), 1,
             "Rust q271_one_card_with_three_base_hearts_scores: "
             "one revealed card with heart01+heart03+heart06 -> 3 types -> +1 score");
}

static void e_q271_blade_hearts_do_not_satisfy_condition(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    int ids[3];
    fresh(&tg);
    ids[0] = sid(&tg, B_PEACH);
    ids[1] = sid(&tg, B_BLUE);
    ids[2] = sid(&tg, B_ALL);
    CHECK_EQ(q271_success(&tg, ids, 3), 0,
             "Rust q271_blade_hearts_do_not_satisfy_condition: "
             "桃/青/ALL blade-hearts are not heart colours -> condition NOT met");
}

static void e_q271_blade_hearts_mapping_to_colors_still_not_met(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    int ids[3];
    fresh(&tg);
    ids[0] = sid(&tg, B_PEACH);
    ids[1] = sid(&tg, B_YELLOW);
    ids[2] = sid(&tg, B_BLUE);
    CHECK_EQ(q271_success(&tg, ids, 3), 0,
             "Rust q271_blade_hearts_mapping_to_colors_still_not_met: "
             "even 3 blade-heart colours do not count as heart colours");
}

static void e_q271_two_base_hearts_no_score(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    int ids[2];
    fresh(&tg);
    ids[0] = sid(&tg, H01);
    ids[1] = sid(&tg, H03);
    CHECK_EQ(q271_success(&tg, ids, 2), 0,
             "Rust q271_two_base_hearts_no_score: 2 distinct base heart colours -> no score");
}

/* EXTRA -- NOT in the Rust file, and added because every Rust positive and
 * negative in this section is one where the CARD count and the COLOUR-TYPE
 * count happen to be equal (3 cards / 3 colours, 2 cards / 2 colours, ...), so
 * not one of them can tell a correct `unit: "types"` evaluator from a plain
 * CARD COUNT.  These three are DISTINCT prints (cards.json: PL!-sd1-005-SD
 * base_heart {heart01:1}, PL!-PR-007-PR 東條 希 {heart01:1}, PL!HS-bp6-014-R
 * 安養寺 姫芽 {heart01:1}; none of the three carries a blade_heart) so the
 * reveal is three cards but ONE colour type.  The card text says 「3種類以上」
 * (three KINDS) and the official Q271 answer counts kinds, so Rust must give 0;
 * a card-counting engine gives +1.  Discriminating half of the pair with
 * q271_three_distinct_base_hearts_scores (3 cards / 3 kinds -> +1). */
#define H01B   "PL!-PR-007-PR"      /* 東條 希,    base heart01:1, no blade_heart */
#define H01C   "PL!HS-bp6-014-R"     /* 安養寺 姫芽, base heart01:1, no blade_heart */

static void e_q271_three_cards_one_colour_no_score(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    int ids[3];
    fresh(&tg);
    ids[0] = sid(&tg, H01);
    ids[1] = sid(&tg, H01B);
    ids[2] = sid(&tg, H01C);
    if (ids[0] < 0 || ids[1] < 0 || ids[2] < 0) return;
    if (!pin_id(ids[0], H01) || !pin_id(ids[1], H01B) || !pin_id(ids[2], H01C)) return;
    CHECK(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2],
          "setup: three DISTINCT prints, not one card counted three times");
    CHECK_EQ(distinct_base_colors(ids, 3), 1,
             "premise: the three reveals really carry only ONE distinct base heart "
             "colour, so 「3種類以上」 is false however many cards there are");
    CHECK_EQ(q271_success(&tg, ids, 3), 0,
             "EXTRA (not asserted in Rust): 3 cards but 1 heart-colour type -> no score "
             "(a plain card count would wrongly give +1)");
}

static void e_q271_blade_hearts_do_not_add_a_colour(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    int ids[4];
    fresh(&tg);
    ids[0] = sid(&tg, H01);
    ids[1] = sid(&tg, H03);
    ids[2] = sid(&tg, B_PEACH);
    ids[3] = sid(&tg, B_ALL);
    CHECK_EQ(q271_success(&tg, ids, 4), 0,
             "Rust q271_blade_hearts_do_not_add_a_color: "
             "2 base + 2 blade-only is still 2 types -> no score");
}

static void e_q271_no_revealed_cards_no_score(void)
{
    static TestGame ctl, tg;
    CHECK_EQ(q271_positive_control(&ctl), 1,
             "ANTI-VACUITY CONTROL: 3 base heart colours really do score +1 on this engine");
    fresh(&tg);
    CHECK_EQ(q271_success(&tg, NULL, 0), 0,
             "Rust q271_no_revealed_cards_no_score: no revealed cards -> no score");
    /* Zone control: the live card really was in the zone when the trigger ran,
     * so a 0 here is not "the ability never saw its host". */
}

/* ab#0 -- one 虹ヶ咲 member on stage gains 1 blade.  The negative arm carries
 * the positive arm on the same board, changing only the staged member. */
static void e_q271_ab0_niji_member_gains_blade(void)
{
    static TestGame tg;
    fresh(&tg);
    int cd = sid(&tg, CD);
    int m = mid(&tg, NIJI_MEM);
    if (!pin_id(cd, CD) || !pin_id(m, NIJI_MEM)) return;
    CHECK(card_prints_trigger(cd, RB_TSTR_LIVE_START),
          "setup: ab#0 really is a live-start ability");
    live_append(&tg, 0, cd);
    set_stage(&tg, 0, AREA_CENTER, m);
    CHECK(fire_trigger(&tg, cd, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START),
          "the live-start ability is queued and processed");
    drain_pick_first(&tg, 16);
    CHECK_EQ(blade_of(&tg, m), 1,
             "Rust q271_ab0_niji_member_gains_blade: a 虹ヶ咲 member gains 1 blade");
}

static void e_q271_ab0_non_niji_member_no_blade(void)
{
    /* ANTI-VACUITY CONTROL on the identical board, only the member swapped. */
    static TestGame ctl, tg;
    fresh(&ctl);
    int cd = sid(&ctl, CD);
    int m = mid(&ctl, NIJI_MEM);
    if (!pin_id(cd, CD) || !pin_id(m, NIJI_MEM)) return;
    live_append(&ctl, 0, cd);
    set_stage(&ctl, 0, AREA_CENTER, m);
    fire_trigger(&ctl, cd, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    drain_pick_first(&ctl, 16);
    CHECK_EQ(blade_of(&ctl, m), 1,
             "ANTI-VACUITY CONTROL: the 虹ヶ咲 member really does gain the blade");

    fresh(&tg);
    int cd2 = sid(&tg, CD);
    int bad = mid(&tg, NON_NIJI);
    if (!pin_id(cd2, CD) || !pin_id(bad, NON_NIJI)) return;
    live_append(&tg, 0, cd2);
    set_stage(&tg, 0, AREA_CENTER, bad);
    fire_trigger(&tg, cd2, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    drain_pick_first(&tg, 16);
    CHECK_EQ(blade_of(&tg, bad), 0,
             "Rust q271_ab0_non_niji_member_no_blade: a non-虹ヶ咲 member gains no blade");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * F. q148_waited_member_blade_total_test.rs
 *    PL!-bp3-023-L ミはμ'sicのミ (cards.json: score 3, need heart01:2
 *    heart06:2 heart0:4, blade_heart b_all:1)
 *
 *    ab#0 ライブ開始時  modify_required_hearts{heart_colors [heart00],
 *      operation decrease, value 2, self_target}
 *      condition: location_condition{location "stage", card_type
 *                  member_card, aggregate "total", count 10, ">="}
 *
 *    The RULING under test: a WAITED member's blade is part of the total.
 * ═════════════════════════════════════════════════════════════════════════ */

static void q148_board(TestGame *tg, int wait_center)
{
    int live  = sid(tg, MI_LIVE);
    int m_a   = mid(tg, SETSUNA);
    int m_b   = mid(tg, KOTORI7);
    if (!pin_id(live, MI_LIVE)) return;
    if (!pin_id(m_a, SETSUNA)) return;
    if (!pin_id(m_b, KOTORI7)) return;
    /* Premise pins, read off the ENGINE's decoded record (not just the print):
     * the 10-blade threshold is only reachable with both members. */
    CHECK_EQ(printed_blade(m_a), 7, "premise: 優木せつ菜 prints 7 blades");
    CHECK_EQ(printed_blade(m_b), 7, "premise: 南ことり prints 7 blades");
    CHECK_EQ(printed_score(live), 3, "premise: ミはμ'sicのミ prints score 3");
    set_stage(tg, 0, AREA_LEFT, m_a);
    set_stage(tg, 0, AREA_CENTER, m_b);
    set_stage(tg, 0, AREA_RIGHT, RB_EMPTY_SLOT);
    if (wait_center) set_stage_wait(tg, 0, AREA_CENTER, 1);
    hand_append(tg, 0, live);
    int d = mid(tg, "PL!-sd1-002-SD");
    deck_append_n(tg, 0, d, 20);
    deck_append_n(tg, 1, d, 20);
}

static void f_q148_blade_total_active_members(void)
{
    static TestGame tg;
    fresh(&tg);
    q148_board(&tg, 0);
    advance_to_live_set(&tg);
    int live = tg.state.p[0].live.n > 0 ? tg.state.p[0].live.cards[0] : -1;
    /* the live card was set by advance_to_live_set? No -- the Rust sets it
     * explicitly after reaching the live-card-set window. */
    (void)live;
    /* Re-find the live: it is the only card in hand that is a live card. */
    int liv = -1;
    for (int i = 0; i < tg.state.p[0].hand.n; i++)
        if (rb_card_is_live(tg.state.p[0].hand.cards[i]))
            liv = tg.state.p[0].hand.cards[i];
    if (liv < 0 || !pin_id(liv, MI_LIVE)) return;
    test_set_live_card(&tg, 0, liv);
    CHECK_EQ(tg.state.p[0].live.n, 1, "setup: the live card really is in the live zone");
    advance_to_live_start(&tg);
    int steps = drain_pick_first(&tg, 32);
    CHECK_PROMPTS(1,
                  "the live-start ability was really surfaced and proceeded on the board "
                  "under test (steps=%d)", steps);
    CHECK_EQ(need_heart_of(&tg, liv, HC[0]), -2,
             "Rust q148_blade_total_active_members: blade 7+7=14 >= 10 -> need_heart00 -2");

    /* DIAGNOSTIC -- is the blocker the CONDITION or the EFFECT?  The condition
     * is `location_condition{location "stage", card_type member_card, aggregate
     * "total", count 10, ">="}`, i.e. a SUM OF BLADE over the staged members.
     * One member carrying 27 blades makes the SUM 27 >= 10 while the CARD COUNT
     * is 1 < 10, so this single board separates the two readings.  ROOT FOUND
     * (src/ability/condition.c:2051-2102, rb_check_aggregate_total): the
     * aggregate arm sums ONLY the heart colours named in the condition's own
     * `heart_colors` list, and ミはμ'sicのミ's condition names none, so `total`
     * is 0 for every board and `0 >= 10` is never true.  It can never read
     * blade at all, so this stays red until that function sums the printed +
     * modified blade of the matching cards. */
    {
        static TestGame diag;
        fresh(&diag);
        q148_board(&diag, 0);
        /* drop the centre member: one member, blade sum 27 >= 10, card count 1 */
        int a = diag.state.p[0].stage[AREA_LEFT];
        set_stage(&diag, 0, AREA_CENTER, RB_EMPTY_SLOT);
        rb_mods_add_blade((RbMods *)&diag.state.mods, a, 20);
        CHECK_EQ(blade_of(&diag, a), 20, "CONTROL: the blade MODIFIER really is +20, "
                                           "on top of 優木せつ菜's printed 7 (blade_of "
                                           "reads the modifier, not the printed value)");
        advance_to_live_set(&diag);
        int dl = -1;
        for (int i = 0; i < diag.state.p[0].hand.n; i++)
            if (rb_card_is_live(diag.state.p[0].hand.cards[i]))
                dl = diag.state.p[0].hand.cards[i];
        if (dl >= 0) {
            test_set_live_card(&diag, 0, dl);
            advance_to_live_start(&diag);
            drain_pick_first(&diag, 32);
            CHECK_EQ(need_heart_of(&diag, dl, HC[0]), -2,
                     "DIAGNOSTIC: ONE staged member with 27 blades (sum 27 >= 10, card "
                     "count 1 < 10) -- -2 here proves `aggregate: total` is read as a "
                     "blade SUM; 0 proves the aggregate never reaches blade at all "
                     "(root: rb_check_aggregate_total sums only the condition's own "
                     "heart_colors list, and this condition names none)");
        }
    }
}

static void f_q148_blade_total_includes_waited_member(void)
{
    static TestGame ctl, tg;
    /* ANTI-VACUITY CONTROL: the same two members, NEITHER waited. */
    fresh(&ctl);
    q148_board(&ctl, 0);
    advance_to_live_set(&ctl);
    int liv0 = -1;
    for (int i = 0; i < ctl.state.p[0].hand.n; i++)
        if (rb_card_is_live(ctl.state.p[0].hand.cards[i]))
            liv0 = ctl.state.p[0].hand.cards[i];
    if (liv0 < 0 || !pin_id(liv0, MI_LIVE)) return;
    test_set_live_card(&ctl, 0, liv0);
    advance_to_live_start(&ctl);
    drain_pick_first(&ctl, 32);
    CHECK_EQ(need_heart_of(&ctl, liv0, HC[0]), -2,
             "ANTI-VACUITY CONTROL: two ACTIVE blade-7 members reach 14 -> need_heart00 -2");

    fresh(&tg);
    q148_board(&tg, 1);
    CHECK_EQ(tg.state.p[0].stage_wait[AREA_CENTER], 1,
             "setup: the centre member really is waited");
    advance_to_live_set(&tg);
    int liv = -1;
    for (int i = 0; i < tg.state.p[0].hand.n; i++)
        if (rb_card_is_live(tg.state.p[0].hand.cards[i]))
            liv = tg.state.p[0].hand.cards[i];
    if (liv < 0 || !pin_id(liv, MI_LIVE)) return;
    test_set_live_card(&tg, 0, liv);
    advance_to_live_start(&tg);
    drain_pick_first(&tg, 32);
    CHECK_EQ(need_heart_of(&tg, liv, HC[0]), -2,
             "Rust q148_blade_total_includes_waited_member: "
             "waited blade counts -> total 14 >= 10 -> need_heart00 -2");
}

static void f_q148_blade_total_below_threshold(void)
{
    static TestGame ctl, tg;
    /* ANTI-VACUITY CONTROL: with BOTH members the same card does fire. */
    fresh(&ctl);
    q148_board(&ctl, 0);
    advance_to_live_set(&ctl);
    int liv0 = -1;
    for (int i = 0; i < ctl.state.p[0].hand.n; i++)
        if (rb_card_is_live(ctl.state.p[0].hand.cards[i]))
            liv0 = ctl.state.p[0].hand.cards[i];
    if (liv0 < 0 || !pin_id(liv0, MI_LIVE)) return;
    test_set_live_card(&ctl, 0, liv0);
    advance_to_live_start(&ctl);
    drain_pick_first(&ctl, 32);
    CHECK_EQ(need_heart_of(&ctl, liv0, HC[0]), -2,
             "ANTI-VACUITY CONTROL: with 2 members the ability really does fire");

    /* Negative: ONE blade-7 member only. */
    fresh(&tg);
    int live = sid(&tg, MI_LIVE);
    int m_a = mid(&tg, SETSUNA);
    if (!pin_id(live, MI_LIVE) || !pin_id(m_a, SETSUNA)) return;
    CHECK_EQ(printed_blade(m_a), 7, "premise: 優木せつ菜 alone contributes 7 blades");
    set_stage(&tg, 0, AREA_LEFT, m_a);
    set_stage(&tg, 0, AREA_CENTER, RB_EMPTY_SLOT);
    set_stage(&tg, 0, AREA_RIGHT, RB_EMPTY_SLOT);
    hand_append(&tg, 0, live);
    int d = mid(&tg, "PL!-sd1-002-SD");
    deck_append_n(&tg, 0, d, 20);
    deck_append_n(&tg, 1, d, 20);
    advance_to_live_set(&tg);
    int liv = -1;
    for (int i = 0; i < tg.state.p[0].hand.n; i++)
        if (rb_card_is_live(tg.state.p[0].hand.cards[i]))
            liv = tg.state.p[0].hand.cards[i];
    if (liv < 0) return;
    test_set_live_card(&tg, 0, liv);
    advance_to_live_start(&tg);
    drain_pick_first(&tg, 32);
    CHECK_EQ(need_heart_of(&tg, liv, HC[0]), 0,
             "Rust q148_blade_total_below_threshold: blade 7 < 10 -> no heart modifier");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * G. q273_fired_deboot_ability_cost_paid_test.rs
 *    PL!S-bp7-005-R＋ 渡辺 曜 (cards.json: unit CYARON!, cost 15, blade 6)
 *    PL!S-PR-019-PR  国木田花丸 (cards.json: unit AZALEA, cost 11, blade 3)
 *
 *    渡辺 ab#2 起動/1, センター:
 *      cost:   move_cards{hand -> discard, count 2}
 *      effect: sequential[
 *                 select{stage, count 2, member_card, group_names ["Aqours"]},
 *                 activate_ability{source_card "previous_selected",
 *                                  target_trigger 登場, multiple_targets true} ]
 *    花丸 ab#0 登場:
 *      cost:   move_cards{hand -> discard, count 1, optional}   <-- Q273
 *      effect: look_and_select{look_at deck_top 3, take 1 to hand, rest to waitroom}
 *
 *    Q273: a 登場 ability fired BY an activation still pays its own cost.
 *
 *    DIVERGENCE, stated at the call site: Rust answers a SelectAutoAbility with
 *    select_indices(&[]), but the C fold of an empty answer is index -1, which
 *    the C engine treats as DECLINE.  Declining would skip the very 登場
 *    abilities this test exists to observe, so this file answers 0 (PROCEED),
 *    which is the peers' bl_drain_proceed idiom and is what "fire them" means.
 * ═════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int accepted;
    int saw_hanamaru_cost;
    int saw_stage_select;
    int deck_moved;
    int hand_added;
    int under_center;
} Q273Run;

static Q273Run q273_run(TestGame *tg)
{
    Q273Run r;
    r.accepted = 0; r.saw_hanamaru_cost = 0; r.saw_stage_select = 0;
    r.deck_moved = -1; r.hand_added = -1; r.under_center = -1;
    fresh(tg);
    int wata = mid(tg, WATANABE);
    int hana = mid(tg, HANAMARU);
    if (!pin_id(wata, WATANABE)) return r;
    if (!pin_id(hana, HANAMARU)) return r;
    /* SETUP PIN: ab#2 is the card's ONLY 起動 (ab#0 is 登場, ab#1 is 常時), so
     * rb_activate_card's first-起動 scan lands on exactly the ability the Rust
     * addresses by index 2.  If this ever goes red the port is invalid. */
    CHECK_EQ(card_trigger_index(wata, RB_TSTR_ACTIVATION), 2,
             "setup: 渡辺 曜's 起動 is ability index 2, the one the Rust activates by index");
    CHECK_EQ(printed_cost(wata), 15, "premise: 渡辺 曜 costs 15 (cards.json cost=15)");
    CHECK_EQ(printed_cost(hana), 11, "premise: 国木田花丸 costs 11 (cards.json cost=11)");
    CHECK(card_prints_trigger(hana, RB_TSTR_DEBUT),
          "setup: 花丸 really does have a 登場 ability to fire");

    set_stage(tg, 0, AREA_LEFT, hana);
    set_stage(tg, 0, AREA_CENTER, wata);
    set_stage(tg, 0, AREA_RIGHT, RB_EMPTY_SLOT);
    tg->state.p[0].hand.n = 0;
    hand_append(tg, 0, mid(tg, FILLER));
    hand_append(tg, 0, mid(tg, FILLER));
    hand_append(tg, 0, mid(tg, FILLER));
    wait_append(tg, 0, mid(tg, DECKCARD));
    tg->state.p[0].deck.n = 0;
    deck_append_n(tg, 0, mid(tg, DECKCARD), 3);

    int deck_before = deck_len(tg, 0);
    int hand_before = tg->state.p[0].hand.n;
    r.accepted = test_activate_ability(tg, wata);

    int guard = 0;
    while (pending(tg) && guard++ < 60) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int kind = c ? (int)c->kind : (int)RB_CHOICE_NONE;
        const char *zone = (c && c->zone[0]) ? c->zone : "";
        /* FULL choice trace: distinguishes "no select was ever raised" (the
         * sequential bailed) from "a select WAS raised with zero candidates"
         * (the group / exclude_self / activation_position filter emptied the
         * set).  The two are different defects and the assertions below cannot
         * tell them apart. */
        fprintf(stderr, "        [Q273] choice kind=%d zone=\"%s\" count=%d "
                        "n_filtered=%d filter_group=\"%s\" card_type=\"%s\" allow_skip=%d\n",
                kind, zone, c ? c->count : -1, c ? c->n_filtered_indices : -1,
                (c && c->filter_group[0]) ? c->filter_group : "-",
                (c && c->card_type[0]) ? c->card_type : "-", c ? c->allow_skip : -1);
        if (kind == RB_CHOICE_SELECT_AUTO_ABILITY) {
            pick(tg, 0);                     /* PROCEED with the fired 登場 */
        } else if (kind == RB_CHOICE_SELECT_CARD && !strcmp(zone, "hand") && c->count == 2) {
            const int two[2] = { 0, 1 };     /* 渡辺's own cost: 2 from hand   */
            pick_many(tg, two, 2);
        } else if (kind == RB_CHOICE_SELECT_CARD && !strcmp(zone, "stage")) {
            r.saw_stage_select = 1;         /* the sequential's own select     */
            const int two[2] = { 0, 1 };     /* 花丸 + 渡辺                    */
            pick_many(tg, two, 2);
        } else if (kind == RB_CHOICE_SELECT_CARD && !strcmp(zone, "discard")) {
            pick(tg, 0);                     /* 渡辺's 登場: the under-card    */
        } else if (kind == RB_CHOICE_SELECT_CARD && !strcmp(zone, "hand")) {
            r.saw_hanamaru_cost = 1;         /* 花丸's 登場 COST -- pay it     */
            pick(tg, 0);
        } else if (kind == RB_CHOICE_SELECT_CARD && !strcmp(zone, "looked_at")) {
            pick(tg, 0);                     /* take 1 of the looked-at 3      */
        } else {
            pick(tg, 0);
        }
    }
    r.deck_moved = deck_before - deck_len(tg, 0);
    r.hand_added = tg->state.p[0].hand.n - hand_before;
    r.under_center = tg->state.p[0].under_cards[AREA_CENTER].n;
    fprintf(stderr, "        [info] Q273 accepted=%d saw_stage_select=%d "
                    "saw_hanamaru_cost=%d deck_moved=%d hand_added=%d under_center=%d\n",
            r.accepted, r.saw_stage_select, r.saw_hanamaru_cost,
            r.deck_moved, r.hand_added, r.under_center);
    return r;
}

static void g_q273_fired_debut_cost_is_paid(void)
{
    static TestGame tg;
    Q273Run r = q273_run(&tg);
    if (r.accepted == 0 && r.saw_hanamaru_cost == 0 && r.under_center < 0) return;
    CHECK(r.accepted, "渡辺 曜's ab#2 activates");
    /* LOCALISER, added after a run in which the cost was paid and nothing else
     * happened.  渡辺's ab#2 effect is sequential[select(stage, 2),
     * activate_ability(previous_selected, 登場)].  If the stage select is never
     * offered, the failure is "the sequential's first child never surfaced",
     * NOT "the fired 登場 was not fired" -- those are different defects and the
     * two assertions below say which one it is. */
    CHECK(r.saw_stage_select,
          "the sequential's FIRST child -- select{stage, count 2, 『Aqours』} -- "
          "must be offered after 渡辺's own cost is paid");
    CHECK(r.saw_hanamaru_cost,
          "Rust q273_fired_debut_ability_cost_is_paid: the fired 登場 ability's own "
          "cost (discard 1) must be offered and paid");
    /* The Rust deliberately does not settle for "the deck shrank": it pins the
     * whole looked-at set leaving, which distinguishes a 3-card outcome from a
     * 1-card outcome and from a shrink caused by the cost's discard. */
    CHECK_EQ(r.deck_moved, 3,
             "Rust q273_fired_debut_ability_cost_is_paid: the whole looked-at set "
             "leaves the deck -- 1 to hand, 2 discarded");
    CHECK_EQ(r.hand_added, 1,
             "the looked-at card really went to hand (not just out of the deck)");
}

static void g_q273_watanabe_own_debut_fires(void)
{
    static TestGame tg;
    Q273Run r = q273_run(&tg);
    if (r.under_center < 0) return;
    CHECK(r.accepted, "渡辺 曜's ab#2 activates");
    CHECK(r.saw_hanamaru_cost,
          "CONTROL: the OTHER member's 登場 cost was paid, so both were fired");
    CHECK_EQ(r.under_center, 1,
             "Rust q273_watanabe_own_debut_fires: 渡辺's own  등장 fired and placed "
             "a member under her");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * main -- each case runs in a FORKED child with a SIGSEGV/SIGBUS/SIGABRT
 * handler naming the test in flight.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child that dies mid-assertion still
 * reports what it completed.  run() is crash isolation, not a hang detector:
 * no watchdog is achievable on this toolchain, and every drain is capped.
 * ═════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK        0
#define CHILD_FAILURES  1

static int n_ok, n_failed, n_crashed, n_setup_bugs;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (tal->failures > f0) n_failed++; else n_ok++;
        printf("%-6s %s  [in-process fallback]\n", tal->failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = tal->assertions, f0 = tal->failures;
        int s0 = tal->setup_bugs, p0 = tal->prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d prompt(s)\n",
               name, tal->assertions - a0, tal->failures - f0,
               tal->setup_bugs - s0, tal->prompts - p0);
        fflush(stdout);
        fflush(stderr);
        _Exit(tal->failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-6s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-6s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-6s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    tally_init();
    if (!tal) { fprintf(stderr, "FAIL: could not map the shared tally\n"); return 1; }
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    hcolor_init();
    print_wire_map();

    printf("--- A activate_change_state_wait_grants_live_end_blade_test.rs "
           "(PL!HS-cl1-003-CL) ---\n");
    run("cl_gain_resource_pay_cost_wait_gain_blade",        a_cl_cost_pay_wait_gain_blade);
    run("cl_gain_resource_cost_pays_when_no_condition_target", a_cl_cost_pays_without_effect_target);

    printf("--- B per_unit_discard_count_regression_test.rs ---\n");
    run("test_per_unit_discard_bug_fix",                    b_draw_phase_discards_nothing);

    printf("--- C hazuki_activate_discard_per_liella_member_blade_test.rs "
           "(PL!SP-bp5-005-R+) ---\n");
    run("hazuki_activate_two_liella_discarded_gain_2_blade", c_hazuki_two_liella_discarded_gain_two_blade);
    run("hazuki_activate_no_liella_discarded_gain_0_blade",  c_hazuki_no_liella_discarded_gain_zero_blade);
    run("hazuki_activate_not_enough_deck_no_blade",          c_hazuki_not_enough_deck_no_blade);

    printf("--- D q46_kanako_all_heart_timing_test.rs (PL!N-bp1-012-R+) ---\n");
    run("q46_kanako_constant_grants_blades_when_condition_met", d_q46_condition_met_grants_two_blades);
    run("q46_kanako_condition_less_than_3_live_cards_no_gain",  d_q46_less_than_three_live_cards_no_gain);
    run("q46_kanako_no_nijigasaki_live_card_no_gain",          d_q46_no_nijigasaki_live_no_gain);
    run("q46_kanako_not_on_stage_no_constant",                 d_q46_not_on_stage_no_constant);
    run("q46_kanako_leaves_stage_blade_removed",               d_q46_leaves_stage_blade_removed);
    run("q46_live_card_removed_condition_fails_blade_removed", d_q46_live_card_removed_blade_removed);
    run("q46_multiple_kanako_each_gains_blades",               d_q46_two_kanako_each_gains);

    printf("--- E q271_colorful_dreams_test.rs (PL!N-bp7-025-L) ---\n");
    run("q271_blade_hearts_do_not_satisfy_condition",              e_q271_blade_hearts_do_not_satisfy_condition);
    run("q271_blade_hearts_mapping_to_colors_still_not_met",      e_q271_blade_hearts_mapping_to_colors_still_not_met);
    run("q271_three_distinct_base_hearts_scores",                 e_q271_three_distinct_base_hearts_scores);
    run("q271_one_card_with_three_base_hearts_scores",            e_q271_one_card_with_three_base_hearts_scores);
    run("q271_two_base_hearts_no_score",                          e_q271_two_base_hearts_no_score);
    run("q271_extra_three_cards_one_colour_no_score",             e_q271_three_cards_one_colour_no_score);
    run("q271_blade_hearts_do_not_add_a_color",                   e_q271_blade_hearts_do_not_add_a_colour);
    run("q271_no_revealed_cards_no_score",                        e_q271_no_revealed_cards_no_score);
    run("q271_ab0_niji_member_gains_blade",                       e_q271_ab0_niji_member_gains_blade);
    run("q271_ab0_non_niji_member_no_blade",                      e_q271_ab0_non_niji_member_no_blade);

    printf("--- F q148_waited_member_blade_total_test.rs (PL!-bp3-023-L) ---\n");
    run("q148_blade_total_active_members",        f_q148_blade_total_active_members);
    run("q148_blade_total_includes_waited_member", f_q148_blade_total_includes_waited_member);
    run("q148_blade_total_below_threshold",       f_q148_blade_total_below_threshold);

    printf("--- G q273_fired_debut_ability_cost_paid_test.rs "
           "(PL!S-bp7-005-R+ / PL!S-PR-019-PR) ---\n");
    run("q273_fired_debut_ability_cost_is_paid", g_q273_fired_debut_cost_is_paid);
    run("q273_watanabe_own_debut_fires",         g_q273_watanabe_own_debut_fires);

    n_setup_bugs = tal->setup_bugs;
    rb_unload();

    printf("\n=== test_parity_blades2 summary ===\n");
    printf("  cases: %d ok, %d failed, %d crashed\n", n_ok, n_failed, n_crashed);
    printf("  assertions: %d, failures: %d, setup bugs: %d, prompts seen: %d\n",
           tal->assertions, tal->failures, n_setup_bugs, tal->prompts);
    if (n_failed || n_crashed || tal->failures) {
        fprintf(stderr, "%d failure(s) across %d crashed case(s)\n",
                tal->failures, n_crashed);
        return 1;
    }
    printf("ALL %d BLADES2 PARITY CHECKS PASSED\n", tal->assertions);
    return 0;
}

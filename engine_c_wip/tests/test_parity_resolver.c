/* tests/test_parity_resolver.c
 *
 * A suite for src/ability/resolver.c.  It had NO suite at all: the Makefile
 * recorded `parity_resolver=GENUINELY_UNTESTED_no_resolver_suite_exists` in
 * SUITE_GAPS, i.e. a name in circulation with nothing behind it.  resolver.c is
 * 1 223 lines of the choice/resolution resolver and every function below is a
 * public entry point declared in include/rabuka.h around lines 2773-2810.
 *
 * WHY A SUITE FOR THIS FILE SPECIFICALLY, NOW.
 * The failure class this catches is LITERAL ROUTING.  A peer's recent fix in
 * this area was a routing bug where a literal carried an UNDERSCORE where Rust
 * had a COLON, so `strstr` could never match the string Rust matches, and the
 * arm silently did nothing forever.  Nothing in the corpus would have reported
 * it.  Every function tested below is a place where one wrong separator, one
 * wrong spelling or one missing arm turns a live predicate into a no-op, and
 * each test asserts BOTH arms plus a neighbouring negative so a predicate that
 * has collapsed to `return 1` or `return 0` is red either way.
 *
 * RUST TWINS, cited BY CONTENT because the Rust tree is being edited.
 *
 *  1. `rb_resolver_card_matches_type` -> ability/util.rs:585-621,
 *     `card_matches_type_for`, the card-type filter normaliser.  Its arms,
 *     verbatim in intent:
 *         match filter {                          // after `.map(str::trim)`
 *           live_card            -> card.is_live()
 *           member_card | member -> card.is_member()
 *           energy_card          -> card.is_energy()
 *           card | ""            -> true      // names no restriction
 *           None                 -> true
 *           other                -> FALSE     // "fail CLOSED, and say so"
 *         }
 *     Matching is eq_ignore_ascii_case.  The fail-closed arm is the one that
 *     matters: the Rust comment records that an unrecognised filter previously
 *     reached a fall-through that decided the match, and PL!N-bp7-028-L's
 *     `card_type: "member"` reached it and applied to ANY stage card.
 *     >>> ALL THREE DEFECTS LANDED 2026-10-04 in src/ability/util.c: the
 *     `member` alias, the ASCII case-insensitive comparison, and the fail-CLOSED
 *     fall-through.  This suite's rows for all three are in
 *     card_matcher_aliases_and_operators_are_rust().
 *
 *  2. `rb_resolver_card_matches_cost_limit` -> ability/util.rs:1095-1116,
 *     `card_matches_cost_limit_op_for`: `Some(limit)` compares
 *     `card.cost.or(card.score)` through the single canonical table
 *     `compare_with_operator` (util.rs:2301-2311) with `comparison` defaulting
 *     to "<="; `None` is true.  "min" is a card-text-only spelling mapped to
 *     ">="; an operator the engine does not know keeps the "<=" reading; a card
 *     with neither cost nor score is FALSE.  The C wrapper additionally short
 *     circuits `cost_limit < 0` to 1, which is the same answer None gives.
 *     >>> THE MISSING `"=="` AND `"!="` ARMS LANDED 2026-10-04 in
 *     src/ability/util.c.  `"=="` was the REACHABLE one (PL!S-PR-045-PR).  The
 *     QUANTITY divergence (card-type branch vs the `cost.or(score)` Option) is
 *     still open and is still red on purpose, in its own function:
 *     known_divergence_cost_limit_quantity().
 *
 *  3. `rb_resolver_use_limit_reached` -> ability/gates.rs, the use-limit gate:
 *         let Some(use_limit) = ability.use_limit else { return Continue };
 *         if gs.ability_uses_used(card_id, ability_index) >= use_limit { stop }
 *     The C folds "no limit" and "limit below one" into `use_limit <= 0`, and
 *     that is pinned here as the contract rather than left to a comment.
 *
 *  4. `rb_resolver_current_ability_is_activation` -> ability/resolver.rs,
 *     documented as "Whether the ability currently being resolved is a
 *     (activation)", implemented as `a.has_trigger(TriggerKind::Activation)`.
 *     The ability is Option, so a null ability is false.
 *
 *  5. `rb_resolver_zone_for_card` -> ability/resolver.rs, the zone-name helper.
 *     Per player, in this exact order: stage, live_card_zone,
 *     success_live_card_zone, hand, waitroom, energy_zone; otherwise "?".
 *     The names are asserted as strings because they reach condition and effect
 *     payloads verbatim.
 *
 *  6. `rb_resolver_cached_condition_verdict` / `rb_resolver_store_condition_verdict`
 *     -> ability/resolver.rs: "Cached verdict for a condition marked `cache: true`
 *     on the current queue entry"; lookup is by the condition's debug key and
 *     store REPLACES any previously stored verdict for the same key (the Rust
 *     retains-then-pushes).  These two are the highest-value rows here: the C
 *     entry points are documented as having been a `(void)` no-op that always
 *     reported a miss, so nothing a string-keyed caller stored was ever read
 *     back -- a silent cache that is always cold.
 *
 *  7. `rb_resolver_get_pending_choice` / `rb_resolver_pending_choice` /
 *     `rb_resolver_store_pending_choice` -> ability/resolver.rs
 *     `get_pending_choice` returning `self.awaiting.choice.as_ref()`, i.e. an
 *     Option that is None until something is stored.
 *
 *  8. `rb_resolver_fmt_card` / `rb_resolver_fmt_ids` /
 *     `rb_resolver_merge_group_names` -> the resolver's small formatters, which
 *     feed log/prompt text.  They are buffer-backed statics, so a stale-buffer
 *     or off-by-one bug is user-visible text.
 *
 * CARD NUMBERS verified against cards/cards.json before use:
 *   PL!-sd1-010-SD   member (type メンバー),    cost 4
 *   PL!SP-bp5-222-R  member (type メンバー),    cost 8
 *   LL-E-001-SD      energy (type エネルギー),  no cost, no score
 *   PL!N-sd1-025-SD  live   (type ライブ),      score 1, no cost
 * All four are exact keys; rb_find_card_by_no's substring fallback has already
 * mis-resolved a near-miss card number in this tree, so none of these is spelled
 * from memory.
 *
 * Fullwidth trigger tokens are written as \xNN escapes per the repo rule:
 *   (activation) E8 B5 B7 E5 8B 95
 *   (auto)        E8 87 AA E5 8B 95
 *
 * sizeof(GameState) is 1 195 656 B and is pinned by RB_GAMESTATE_BYTES_PINNED:
 * every TestGame below is static.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

#define TK_ACTIVATION "\xE8\xB5\xB7\xE5\x8B\x95"   /* the activation trigger */
#define TK_AUTO        "\xE8\x87\xAA\xE5\x8B\x95"   /* the auto trigger       */

/* Declared here rather than added to include/rabuka.h: the header is being edited
 * concurrently and these are declaration-only additions with no ABI effect.
 * rb_resolver_merge_group_names is a NON-STATIC definition in
 * src/ability/resolver.c with no header declaration at all, so the resolver's
 * public-by-definition surface is not reachable from a test without this. */
const char *rb_resolver_merge_group_names(const char **groups, int n);

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", (msg)); failures++; FLAT_VERDICT(); } \
    else printf("ok: %s\n", (msg)); \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", (msg), a_, e_); \
        failures++; FLAT_VERDICT(); \
    } else printf("ok: %s\n", (msg)); \
} while (0)

static const char *CARD_MEMBER4  = "PL!-sd1-010-SD";
static const char *CARD_MEMBER8  = "PL!SP-bp5-222-R";
static const char *CARD_ENERGY   = "LL-E-001-SD";
static const char *CARD_LIVE     = "PL!N-sd1-025-SD";

/* ── 1 ── the card-type filter, all six Rust arms. */
static void card_matches_type_covers_every_arm(void) {
static    TestGame tg;
    test_game_new(&tg);
    int member = test_id(&tg, CARD_MEMBER4);
    int energy = test_id(&tg, CARD_ENERGY);
    int live   = test_id(&tg, CARD_LIVE);
    if (member < 0 || energy < 0 || live < 0) {
        CHECK(0, "fixtures resolve (a member, an energy card and a live card)");
        return;
    }
    /* Positive arms. */
    CHECK_EQ(rb_resolver_card_matches_type(member, "member_card"), 1,
             "member_card filter accepts a member card");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "energy_card"), 1,
             "energy_card filter accepts an energy card");
    CHECK_EQ(rb_resolver_card_matches_type(live, "live_card"), 1,
             "live_card filter accepts a live card");
    /* The `member` alias, and case-insensitivity, are asserted in
     * card_matcher_aliases_and_operators_are_rust(): they are negative or
     * near-miss rows that would read as ordinary coverage here, and their
     * Rust citations belong with them. */
    /* Negative arms: each filter must refuse the other two types. Without these
     * a matcher that ignored the filter and returned 1 would pass the positives. */
    CHECK_EQ(rb_resolver_card_matches_type(member, "live_card"), 0,
             "live_card filter refuses a member card");
    CHECK_EQ(rb_resolver_card_matches_type(member, "energy_card"), 0,
             "energy_card filter refuses a member card");
    CHECK_EQ(rb_resolver_card_matches_type(live, "member_card"), 0,
             "member_card filter refuses a live card");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "live_card"), 0,
             "live_card filter refuses an energy card");
    CHECK_EQ(rb_resolver_card_matches_type(live, "energy_card"), 0,
             "energy_card filter refuses a live card");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "member_card"), 0,
             "member_card filter refuses an energy card");
    /* `card` and the empty filter name no restriction at all. */
    CHECK_EQ(rb_resolver_card_matches_type(member, "card"), 1, "`card` accepts a member");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "card"), 1, "`card` accepts an energy card");
    CHECK_EQ(rb_resolver_card_matches_type(live, "card"), 1, "`card` accepts a live card");
    CHECK_EQ(rb_resolver_card_matches_type(member, ""), 1,
             "an EMPTY filter names no restriction and accepts a member");
    /* A NULL filter is Option::None in Rust, which is also true. */
    CHECK_EQ(rb_resolver_card_matches_type(member, NULL), 1,
             "a NULL filter is None in Rust and accepts everything");
}

/* ── 2 ── the cost-limit comparison. */
static void card_matches_cost_limit_uses_cost_or_score(void) {
static    TestGame tg;
    test_game_new(&tg);
    int m4   = test_id(&tg, CARD_MEMBER4);   /* cost 4 */
    int m8   = test_id(&tg, CARD_MEMBER8);   /* cost 8 */
    int live = test_id(&tg, CARD_LIVE);      /* no cost, score 1 */
    int en   = test_id(&tg, CARD_ENERGY);    /* neither */
    if (m4 < 0 || m8 < 0 || live < 0 || en < 0) {
        CHECK(0, "fixtures resolve");
        return;
    }
    /* No limit at all: Option::None is true, and the C wrapper's `cost_limit < 0`
     * short circuit is the same answer. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, -1, ">="), 1,
             "a NEGATIVE cost limit means no limit and accepts a cost-4 member");
    /* Default operator is "<=", i.e. a cost cap. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 4, NULL), 1,
             "cost 4 <= 4 under the default operator");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 3, NULL), 0,
             "cost 4 <= 3 is FALSE (a cap is a ceiling, not a floor)");
    /* Explicit operators. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 4, ">="), 1, "cost 4 >= 4");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 5, ">="), 0, "cost 4 >= 5 is FALSE");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 4, "<="), 0, "cost 8 <= 4 is FALSE");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 8, "<="), 1, "cost 8 <= 8");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 8, "=="), 1, "cost 8 == 8");
    /* `min` is the card-text-only spelling mapped to >=. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 8, "min"), 1, "`min` reads as >= : cost 8 >= 8");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 8, "min"), 0, "`min` reads as >= : cost 4 >= 8 is FALSE");
    /* An operator this engine does not know KEEPS the "<=" reading. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m4, 8, "wat"), 1,
             "an unknown operator keeps the <= reading: cost 4 <= 8");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 4, "wat"), 0,
             "an unknown operator keeps the <= reading: cost 8 <= 4 is FALSE");
    /* A LIVE card is scored, not costed: cost.or(score). */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(live, 1, "<="), 1,
             "a live card is compared on its SCORE when it has no cost (score 1 <= 1)");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(live, 0, "<="), 0,
             "the same live card fails a limit of 0 on its score");
    /* A card with neither cost nor score is asserted in
     * known_divergence_cost_limit_quantity() below: the C reads a zeroed cost
     * rather than Rust's Option::None, so that row is a divergence, not a
     * pass, and it is red on purpose. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(en, -1, "<="), 1,
             "the same card is accepted when there is no limit");
}

/* ══ THE OPERATOR TABLE AND THE TYPE ALIAS — LANDED 2026-10-04 ═════════════
 *
 * These rows used to be RED on purpose, kept red in a block named
 * `known_divergences_in_the_cards_matcher`, and they are now GREEN. NOTHING
 * HERE WAS WEAKENED, DELETED, SKIPPED OR `if (0)`-ed: every assertion below
 * asserts the same expected value it asserted when it was red, and the Rust
 * citation travels with it. What changed is the engine, in src/ability/util.c,
 * which this suite's author now holds.
 *
 * (a) rb_card_matches_type — util.c, twin of `card_matches_type_for`
 *     (engine/src/ability/util.rs:585-621). Three defects, all fixed there:
 *       * the `member` alias was MISSING (util.rs:598 spells the member arm
 *         `member_card` OR `member`; the C had only `member_card`);
 *       * the comparison was CASE-SENSITIVE where every Rust arm is
 *         `eq_ignore_ascii_case` (util.rs:597-605);
 *       * it FAILED OPEN — the fall-through was `return 1`, i.e. a filter that
 *         should REJECT a card ACCEPTED it, where util.rs:607-619 is
 *         `Some(other) => { log::warn!(...); false }` under the comment
 *         "Fail CLOSED, and say so". Rust's own comment at util.rs:613-616
 *         names the card that reached that arm, PL!N-bp7-028-L, arriving as
 *         `card_type: "member"` and so applying to anything on the stage.
 *     That third one is VERIFIED-NOOP: 0 on today's corpus — `card_type` there
 *     is exactly `member_card` / `live_card` / `energy_card` — and it was still
 *     worth landing, because a wrong-value bug is not a missing feature.
 *     The trim (util.rs:595 `card_type_filter.map(str::trim)`) is transcribed
 *     with Rust's Unicode White_Space predicate, not an ASCII strip.
 *
 * (b) rb_card_matches_cost_limit — util.c, twin of
 *     `card_matches_cost_limit_op_for` (util.rs:1095-1116), which routes every
 *     operator through the single canonical table `compare_with_operator`
 *     (util.rs:2301-2311). The C's table was missing two arms: `"=="` and
 *     `"!="`. `"=="` IS THE REACHABLE ONE — `cost_limit_operator == "=="`
 *     appears on exactly ONE condition node in the whole corpus,
 *     PL!S-PR-045-PR (card id 1591),
 *     「コスト7のメンバーからバトンタッチして登場した場合」, with
 *     `cost_limit: 7, cost_limit_operator: "=="`. Rust asks for a
 *     cost-EXACTLY-7 member and the C asked for a cost-AT-MOST-7 one, so a
 *     cheaper member's baton touch satisfied the debut condition in C and not
 *     in Rust. The other three spellings carried by the 40 corpus nodes that
 *     have a `cost_limit` key (`>=`, `<=`, `=`) were already in the C.
 *
 * The block was NOT fixed in the resolver's own wrappers, which was considered
 * and rejected for the reason Rust's `compare_with_operator` doc comment gives
 * (util.rs:2290-2295): "This is the one place the operator spellings are known.
 * Every comparison in the engine routes through it, so adding a spelling -- or
 * fixing one -- is a single edit instead of one per call site, and an operator
 * two sites spell differently can no longer mean two different things." A
 * second table in resolver.c would create exactly that divergence and would
 * leave condition.c, move.c, choice.c, cost.c, look.c and util.c still wrong.
 *
 * WHAT IS STILL RED, AND IT IS NOT IN THIS BLOCK: row (c), the QUANTITY rather
 * than the operator, is split out into its own function below so that this one
 * is honestly green instead of carrying a red row it no longer owns.
 */
static void card_matcher_aliases_and_operators_are_rust(void) {
    static TestGame tg;
    test_game_new(&tg);
    int member = test_id(&tg, CARD_MEMBER4);
    int energy = test_id(&tg, CARD_ENERGY);
    int live   = test_id(&tg, CARD_LIVE);
    int m8     = test_id(&tg, CARD_MEMBER8);
    if (member < 0 || energy < 0 || live < 0 || m8 < 0) {
        CHECK(0, "fixtures resolve");
        return;
    }
    /* (a) the `member` alias. Rust: `member_card | member` both mean is_member.
     * The C had no `member` arm, so it fell through to "no restriction" and
     * matched EVERYTHING. */
    CHECK_EQ(rb_resolver_card_matches_type(energy, "member"), 0,
             "(util.c, util.rs:598): the short `member` alias means is_member, "
             "so it must NOT match an energy card");
    /* (a) case insensitivity. Rust matches with eq_ignore_ascii_case; the C used
     * strcmp. The rows below USED TO PASS, and they passed only because the C
     * failed open -- which is why they lived in a block that said so. */
    CHECK_EQ(rb_resolver_card_matches_type(member, "MEMBER_CARD"), 1,
             "(util.c, util.rs:597-598): `MEMBER_CARD` matches as member_card "
             "(ASCII case-insensitive)");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "MEMBER_CARD"), 0,
             "(util.c, util.rs:597-598): `MEMBER_CARD` does NOT match an energy "
             "card");
    CHECK_EQ(rb_resolver_card_matches_type(live, "Live_Card"), 1,
             "(util.c, util.rs:597): `Live_Card` matches as live_card");
    CHECK_EQ(rb_resolver_card_matches_type(energy, "Live_Card"), 0,
             "(util.c, util.rs:597): `Live_Card` does NOT match an energy card");
    /* (a) fail closed. Rust: `Some(other) => { log::warn!(...); false }`. */
    CHECK_EQ(rb_resolver_card_matches_type(member, "not_a_card_type"), 0,
             "(util.c, util.rs:607-619): an UNRECOGNISED filter matches NOTHING "
             "-- the C returned 1, i.e. it failed OPEN");
    CHECK_EQ(rb_resolver_card_matches_type(member, "live-card"), 0,
             "(util.c, util.rs:597): a near-miss spelling with a HYPHEN instead "
             "of an underscore matches nothing");
    /* (b) the `==` spelling. This is the REACHABLE one: 1 condition node in the
     * corpus, PL!S-PR-045-PR. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 9, "=="), 0,
             "(util.c, util.rs:2307): cost 8 == 9 is FALSE -- the C could not "
             "spell `==` and answered the `<=` reading instead");
    CHECK_EQ(rb_resolver_card_matches_cost_limit(m8, 7, "=="), 0,
             "(util.c, util.rs:2307): cost 8 == 7 is FALSE -- the pin for "
             "PL!S-PR-045-PR's cost-7 baton-touch condition");
}

/* ══ STILL RED, ON PURPOSE — the QUANTITY, not the operator ════════════════
 *
 * Row (c), split out of the block above when (a) and (b) landed so that neither
 * the green block nor this one carries a row it does not own. Kept red. Not
 * weakened, not deleted, not skipped.
 *
 * Rust (util.rs:1100-1116) branches on the OPTION:
 *     cost_limit: Some(limit) => card.and_then(|c| c.cost.or(c.score))
 *                                    .map(|v| compare_with_operator(..))
 *                                    .unwrap_or(false)
 * A card with NEITHER cost nor score has no value, so the predicate is FALSE.
 * The C branches on card TYPE instead --
 *     int value = rb_card_is_live(card_id) ? (int)c.score : (int)c.cost;
 * -- and reads a zeroed `cost` for a card that has none, so `0 <= 99` and the
 * card PASSES a limit it fails in Rust.
 *
 * NOT LANDED WITH (a)/(b) ON PURPOSE: this is a different defect with its own
 * reachability question — "which cards carry neither cost nor score, and are
 * any of them candidates under a cost_limit filter" — and it was not part of
 * the change that fixed the two operator/alias defects. Landing it on the
 * strength of a single red assertion would be guessing at that census. */
static void known_divergence_cost_limit_quantity(void) {
    static TestGame tg;
    test_game_new(&tg);
    int energy = test_id(&tg, CARD_ENERGY);
    if (energy < 0) { CHECK(0, "fixtures resolve"); return; }
    /* (c) a card with neither cost nor score. Rust: None => false. */
    CHECK_EQ(rb_resolver_card_matches_cost_limit(energy, 99, "<="), 0,
             "KNOWN DIVERGENCE (util.c, util.rs:1101-1115): a card with neither "
             "cost nor score FAILS a present cost limit -- the C reads a zeroed "
             "cost and passes it");
}

/* ── 3 ── the use-limit ladder. */
static void use_limit_reached_is_a_ladder(void) {
static    TestGame tg;
    test_game_new(&tg);
    int cid = test_id(&tg, CARD_MEMBER4);
    if (cid < 0) { CHECK(0, "fixture resolves"); return; }
    /* No limit. */
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 0), 0,
             "a limit of ZERO means no limit (Rust: a use_limit below one is never "
             "reached)");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, -1), 0,
             "a NEGATIVE limit means no limit");
    /* Before any use. */
    CHECK_EQ(rb_ability_uses_used(&tg.state, cid, 0), 0, "the ability starts with 0 uses");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 1), 0,
             "0 uses against a limit of 1 is NOT reached");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 2), 0,
             "0 uses against a limit of 2 is NOT reached");
    /* After one use: a limit of 1 is reached, a limit of 2 is not. This pair is
     * the whole ladder -- a gate that used `>` instead of `>=` passes the first
     * row and fails this one, and vice versa. */
    rb_record_ability_use(&tg.state, cid, 0);
    CHECK_EQ(rb_ability_uses_used(&tg.state, cid, 0), 1, "one use is recorded");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 1), 1,
             "1 use against a limit of 1 IS reached (>=, not >)");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 2), 0,
             "1 use against a limit of 2 is NOT reached");
    /* After two uses. */
    rb_record_ability_use(&tg.state, cid, 0);
    CHECK_EQ(rb_ability_uses_used(&tg.state, cid, 0), 2, "a second use is recorded");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 2), 1,
             "2 uses against a limit of 2 IS reached");
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 0, 3), 0,
             "2 uses against a limit of 3 is NOT reached");
    /* Per (card, ability) keying: a different ability index is a different key. */
    CHECK_EQ(rb_resolver_use_limit_reached(&tg.state, cid, 1, 1), 0,
             "the use counter is keyed per (card_id, ability_index): index 1 is untouched");
}

/* ── 4 ── the activation classifier. */
static void activation_classifier_reads_the_activation_trigger(void) {
static    TestGame tg;
    test_game_new(&tg);
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    CHECK_EQ(rb_resolver_current_ability_is_activation(NULL), 0,
             "no current ability at all is NOT an activation (Rust: Option is None)");
    ab.triggers = (char *)TK_ACTIVATION;
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 1,
             "an ability carrying the activation trigger IS an activation");
    ab.triggers = (char *)TK_AUTO;
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 0,
             "an ability carrying only the auto trigger is NOT an activation");
    ab.triggers = (char *)"";
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 0,
             "an ability with an EMPTY trigger list is NOT an activation");
    /* Both tokens at once: the classifier is a positive test, so presence of the
     * activation token decides it, in either order. */
    ab.triggers = (char *)"\xE8\xB5\xB7\xE5\x8B\x95,\xE8\x87\xAA\xE5\x8B\x95";
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 1,
             "activation listed FIRST alongside auto is still an activation");
    ab.triggers = (char *)"\xE8\x87\xAA\xE5\x8B\x95,\xE8\xB5\xB7\xE5\x8B\x95";
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 1,
             "activation listed SECOND alongside auto is still an activation");
    /* A near-miss separator must NOT match: this is the underscore-versus-colon
     * class, stated as an assertion rather than left to review. */
    ab.triggers = (char *)"activation";
    CHECK_EQ(rb_resolver_current_ability_is_activation(&ab), 0,
             "the ENGLISH word `activation` alone is NOT the activation trigger -- "
             "the classifier matches the per-token trigger, not a raw substring");
}

/* ── 5 ── the zone-name helper, one row per zone Rust names. */
static void zone_for_card_names_every_zone(void) {
static    TestGame tg;
    test_game_new(&tg);
    int a = test_new_id(&tg, CARD_MEMBER4);
    int b = test_new_id(&tg, CARD_MEMBER4);
    int c = test_new_id(&tg, CARD_MEMBER4);
    int d = test_new_id(&tg, CARD_MEMBER4);
    int e = test_new_id(&tg, CARD_MEMBER4);
    int f = test_new_id(&tg, CARD_MEMBER4);
    int g = test_new_id(&tg, CARD_MEMBER4);
    if (a < 0 || b < 0 || c < 0 || d < 0 || e < 0 || f < 0 || g < 0) {
        CHECK(0, "fixtures resolve (seven DISTINCT card instances)");
        return;
    }
    tg.state.p[0].stage[0]  = a;
    if (tg.state.p[0].live.n       < RB_MAX_LIVE_CARDS) tg.state.p[0].live.cards[tg.state.p[0].live.n++]       = b;
    if (tg.state.p[0].success.n    < RB_MAX_ZONE)      tg.state.p[0].success.cards[tg.state.p[0].success.n++]    = c;
    if (tg.state.p[0].hand.n       < RB_MAX_ZONE)      tg.state.p[0].hand.cards[tg.state.p[0].hand.n++]          = d;
    if (tg.state.p[0].discard.n    < RB_MAX_ZONE)      tg.state.p[0].discard.cards[tg.state.p[0].discard.n++]    = e;
    if (tg.state.p[0].energy.n     < RB_MAX_ZONE)      tg.state.p[0].energy.cards[tg.state.p[0].energy.n++]      = f;
    int nowhere = g;

    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, a), "stage") == 0,
          "a card on the stage reports \"stage\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, b), "live_card_zone") == 0,
          "a card in the live-card zone reports \"live_card_zone\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, c), "success_live_card_zone") == 0,
          "a card in the success zone reports \"success_live_card_zone\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, d), "hand") == 0,
          "a card in hand reports \"hand\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, e), "waitroom") == 0,
          "a card in the waitroom reports \"waitroom\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, f), "energy_zone") == 0,
          "a card in the active energy zone reports \"energy_zone\"");
    CHECK(strcmp(rb_resolver_zone_for_card(&tg.state, nowhere), "?") == 0,
          "a card in no zone at all reports \"?\"");
}

/* ── 6 ── the condition-verdict cache.  The highest-value rows in this file. */
static void condition_verdict_cache_round_trips_and_replaces(void) {
static    TestGame tg;
    test_game_new(&tg);
    int cid = test_id(&tg, CARD_MEMBER4);
    if (cid < 0) { CHECK(0, "fixture resolves"); return; }
    int idx = rb_queue_enqueue(&tg.state, cid, 0, 0, "p1", NULL);
    if (idx < 0) { CHECK(0, "an entry could be enqueued to give the cache a current entry"); return; }
    /* rb_queue_enqueue builds the entry but does not move the cursor; the C cache
     * reads g->queue.cur, exactly as Rust reads ability_queue.current_entry().
     * Point the cursor at the entry so "the current entry" is unambiguous. */
    tg.state.queue.cur = idx;
    CHECK_EQ(tg.state.queue.cur, idx, "the queue's current entry is the one just enqueued");

    int result = -1;
    /* Cold cache. */
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_A", &result), 0,
             "an unknown condition text is a MISS on a cold cache");
    /* Store then read back. The old implementation was a (void) no-op that always
     * reported a miss, so this row is the regression guard for "the cache is always
     * cold and nobody notices because a cold cache is still correct, just slow"
     * -- except it is also WRONG, because the store then re-evaluates a condition
     * whose earlier verdict was the point. */
    rb_resolver_store_condition_verdict(&tg.state, 0, "cond_A", 1);
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_A", &result), 1,
             "a stored verdict is a HIT");
    CHECK_EQ(result, 1, "the hit carries the value that was stored (true)");
    /* A different key on the same entry is still a miss. */
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_B", &result), 0,
             "a DIFFERENT condition text is still a miss -- the key is not ignored");
    /* REPLACE: storing the same key again with the opposite value replaces it.
     * Rust retains-then-pushes, so the entry count does not grow. */
    rb_resolver_store_condition_verdict(&tg.state, 0, "cond_A", 0);
    int n_after_second_store = tg.state.queue.entries[idx].n_cond_cache;
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_A", &result), 1,
             "re-storing the same key is still a HIT");
    CHECK_EQ(result, 0, "the replaced value is the one now read back (false)");
    CHECK_EQ(n_after_second_store, 1,
             "re-storing the same key did NOT grow the cache -- still one entry, "
             "because the store replaces rather than appends");
    /* A second key is stored independently. */
    rb_resolver_store_condition_verdict(&tg.state, 0, "cond_B", 1);
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_B", &result), 1,
             "a second condition text caches independently");
    CHECK_EQ(result, 1, "the second key carries its own value");
    /* An empty key is not a key. */
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "", &result), 0,
             "an EMPTY condition text is a MISS, not a wildcard hit");
    result = -1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, NULL, &result), 0,
             "a NULL condition text is a MISS");
    /* A NULL out-pointer is refused rather than written. */
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_A", NULL), 0,
             "a NULL result pointer is a MISS, not a crash and not a hit");
}

/* ── 7 ── the cache needs a current queue entry; without one it is inert. */
static void condition_verdict_cache_is_inert_without_a_current_entry(void) {
static    TestGame tg;
    test_game_new(&tg);
    int result = -1;
    rb_resolver_store_condition_verdict(&tg.state, 0, "cond_A", 1);
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "cond_A", &result), 0,
             "with no queue entry at all, a store is inert and the read is a MISS");
    CHECK(result == -1,
          "a miss left the out-parameter UNTOUCHED (still the sentinel) rather than "
          "fabricating a verdict -- Rust's miss is Option::None, which carries no value");
}

/* ── 8 ── the pending-choice accessor round trip. */
static void pending_choice_is_none_until_something_is_stored(void) {
static    TestGame tg;
    test_game_new(&tg);
    CHECK(rb_resolver_get_pending_choice(&tg.state) == NULL,
          "no pending choice before anything is stored (Rust: awaiting.choice is None)");
    CHECK_EQ(rb_resolver_pending_choice(&tg.state), -1,
             "the pending-choice route is -1 (no route) before anything is stored");
    memset(&tg.state.queue.pending, 0, sizeof(tg.state.queue.pending));
    tg.state.queue.pending.route = 3;
    rb_resolver_store_pending_choice(&tg.state);
    const RbChoice *ch = rb_resolver_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "a pending choice is readable after store_pending_choice");
    if (ch) CHECK_EQ((int)ch->route, 3, "the stored route reads back unchanged");
    CHECK_EQ(rb_resolver_pending_choice(&tg.state), 3,
             "the route accessor agrees with the stored choice");
    CHECK_EQ(tg.state.queue.has_pending, 1, "store_pending_choice set has_pending");
    CHECK_EQ(tg.state.queue.snapshot_requested, 1,
             "store_pending_choice also raised snapshot_requested");
    /* Storing twice with the SAME signature is the dedupe path: has_pending stays
     * 1 and nothing is pushed twice. */
    rb_resolver_store_pending_choice(&tg.state);
    CHECK_EQ(tg.state.queue.has_pending, 1, "a repeated store of the same offer is idempotent");
}

/* ── 9 ── the formatters. */
static void formatters_render_what_they_are_given(void) {
static    TestGame tg;
    test_game_new(&tg);
    int member = test_id(&tg, CARD_MEMBER4);
    if (member < 0) { CHECK(0, "fixture resolves"); return; }

    char buf[128];
    memset(buf, 'X', sizeof(buf));
    rb_resolver_fmt_card(member, buf, sizeof(buf));
    CHECK(buf[0] != '\0', "fmt_card rendered a non-empty name for a real card");
    CHECK(buf[0] != 'X', "fmt_card wrote into the caller's buffer");
    /* An unknown card id renders empty, not garbage. */
    memset(buf, 'X', sizeof(buf));
    rb_resolver_fmt_card(-1, buf, sizeof(buf));
    CHECK(buf[0] == '\0', "fmt_card renders the EMPTY STRING for an unknown card id");
    /* A zero-sized buffer must not be written at all. */
    char guard = 'Z';
    rb_resolver_fmt_card(member, &guard, 0);
    CHECK_EQ(guard, 'Z', "fmt_card with a zero size left the target byte untouched");

    int ids[3] = { 7, 42, 1234 };
    CHECK(strcmp(rb_resolver_fmt_ids(ids, 3), "7,42,1234") == 0,
          "fmt_ids joins three ids with commas, no spaces");
    CHECK(strcmp(rb_resolver_fmt_ids(ids, 1), "7") == 0,
          "fmt_ids renders a single id with no separator");
    CHECK(strcmp(rb_resolver_fmt_ids(ids, 0), "") == 0,
          "fmt_ids of zero ids is the empty string");
    CHECK(strcmp(rb_resolver_fmt_ids(NULL, 0), "") == 0,
          "fmt_ids of a NULL array with count 0 is the empty string");

    const char *groups[4];
    groups[0] = NULL;
    groups[1] = "unit:school-idol";
    groups[2] = NULL;
    groups[3] = "unit:lia";
    CHECK(strcmp(rb_resolver_merge_group_names(groups, 4), "unit:school-idol,unit:lia") == 0,
          "merge_group_names skips NULL entries and joins the rest with commas");
    const char *one[1] = { "unit:lia" };
    CHECK(strcmp(rb_resolver_merge_group_names(one, 1), "unit:lia") == 0,
          "merge_group_names of one group emits no leading separator");
    CHECK(strcmp(rb_resolver_merge_group_names(one, 0), "") == 0,
          "merge_group_names of zero groups is the empty string");
    const char *nul[1] = { NULL };
    CHECK(strcmp(rb_resolver_merge_group_names(nul, 1), "") == 0,
          "merge_group_names of a single NULL group is the empty string");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    card_matches_type_covers_every_arm();
    card_matches_cost_limit_uses_cost_or_score();
    card_matcher_aliases_and_operators_are_rust();
    known_divergence_cost_limit_quantity();
    use_limit_reached_is_a_ladder();
    activation_classifier_reads_the_activation_trigger();
    zone_for_card_names_every_zone();
    condition_verdict_cache_round_trips_and_replaces();
    condition_verdict_cache_is_inert_without_a_current_entry();
    pending_choice_is_none_until_something_is_stored();
    formatters_render_what_they_are_given();
    rb_unload();
    if (failures) {
        printf("parity_resolver: %d failures\n", failures);
        return 1;
    }
    printf("parity_resolver: all checks passed\n");
    return 0;
}
/* test_parity_conc_cost_card.c
 *
 * Regression tests for the two concentrations closed in this pass:
 * `ability/cost/handlers.rs` -> src/ability/cost.c, and
 * `core/card.rs` -> src/core/card.c.
 *
 * Every assertion below cites the Rust line it mirrors. A test with no Rust
 * citation is not a parity test.
 *
 * Nothing in this file weakens, skips or deletes another suite's assertion; it
 * is a NEW suite.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    long actual_value = (long)(actual); \
    long expected_value = (long)(expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %ld, want %ld)\n", message, \
                actual_value, expected_value); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_STR(actual, expected, message) do { \
    assertions++; \
    const char *a_ = (actual); const char *e_ = (expected); \
    if (!a_ || !e_ || strcmp(a_, e_) != 0) { \
        fprintf(stderr, "FAIL: %s (got \"%s\", want \"%s\")\n", message, \
                a_ ? a_ : "(null)", e_ ? e_ : "(null)"); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* GameState is 1 090 528 B; every one of these locals is `static`. */
static TestGame tg;

/* strdup is not declared under this toolchain's C11 mode; own it. */
static char *dup_s(const char *s)
{
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (!p) { fprintf(stderr, "out of memory\n"); exit(2); }
    memcpy(p, s, n);
    return p;
}

/* an effect carrying exactly the wire extras the test needs */
static AbilityEffect *make_effect(void)
{
    AbilityEffect *e = (AbilityEffect *)malloc(sizeof(AbilityEffect));
    if (!e) { fprintf(stderr, "out of memory\n"); exit(2); }
    memset(e, 0, sizeof(*e));
    return e;
}

static void fx_set(AbilityEffect *e, const char *k, const char *v)
{
    int i = e->n_extra;
    if (i >= RB_MAX_EXTRA) return;
    e->extra_k[i] = dup_s(k);
    e->extra_v[i] = dup_s(v);
    e->n_extra = i + 1;
}

/* ══════════════════════════════════════════════════════════════════════════
 * 1. offer_all_hand_discard -- the WHOLE arm, and that pending->target is SET.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/ability/cost/handlers.rs:378-391:
 *     self.awaiting.choice = Some(Choice::SelectTarget {
 *         target: "pay_cost_all:discard_all".to_string(),
 *         description: format!("Discard entire hand ({} cards)?", hand_len),
 *         allow_skip: is_optional,
 *         options: None,
 *     });
 *     if let Some(entry) = gs.ability_queue.current_entry_mut() {
 *         entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
 *     }
 *
 * The defect this pins: the C set `kind` and left `target` all-zero, so
 * BOTH dispatchers (choice.c's `pending.target` arm and its `tgt` arm) had
 * nothing to match and rb_handle_pay_cost_all_discard was unreachable. The
 * arm is only correct if the TARGET STRING is present, so that is what is
 * asserted -- not the kind, which was already right.
 *
 * REACHABILITY: measured over cards/abilities.json (985 unique abilities),
 * 2 abilities print 「手札をすべて控え室に置いてもよい」 (PL!SP-bp7-011-{P,R}
 * ab#0). Not zero.
 */
static void test_offer_all_hand_discard_sets_the_routing_target(void)
{
    /* Open the prompt through the public cost entry with a hand-discard-all
     * cost, then read what the producer left on the pending choice. */
    int id_a = test_id(&tg, "PL!SP-bp7-011-P");
    CHECK(id_a >= 0 && rb_card_no_eq(id_a, "PL!SP-bp7-011-P"),
          "fixture card really is PL!SP-bp7-011-P -- verified with the "
          "EXACT compare rb_card_no_eq, never rb_find_card_by_no's "
          "substring fallback, which resolves a wrong card on a near-miss");
    if (id_a < 0) return;
    test_add_to_hand(&tg, id_a);
    test_add_to_hand(&tg, test_id(&tg, "PL!N-bp5-021-N"));

    AbilityEffect *cost = make_effect();
    cost->action = dup_s("move_cards");
    cost->source = dup_s("hand");
    cost->destination = dup_s("discard");
    cost->is_optional = 1;   /* Rust `cost.optional`, handlers.rs:358/386 */
    fx_set(cost, "all", "true");

    rb_pay_cost(&tg.state, 0, cost);

    CHECK(tg.state.queue.has_pending,
          "an all-hand-discard cost with a non-empty hand opens a prompt "
          "(handlers.rs:378)");
    if (tg.state.queue.has_pending) {
        CHECK_STR(tg.state.queue.pending.target, "pay_cost_all:discard_all",
                  "the prompt carries Rust's EXACT routing literal, "
                  "enums.rs:705 -- a zeroed target made both dispatchers inert");
        CHECK_EQ(tg.state.queue.pending.kind, RB_CHOICE_SELECT_TARGET,
                 "and it is a SelectTarget choice (handlers.rs:378)");
        CHECK_EQ(tg.state.queue.pending.allow_skip, 1,
                 "allow_skip is is_optional (handlers.rs:386)");
        CHECK_EQ(tg.state.queue.pending.route, RB_ROUTE_OPTIONAL_COST,
                 "the entry is stamped OptionalCost (handlers.rs:389-391)");
        CHECK(strstr(tg.state.queue.pending.description, "Discard entire hand") != NULL,
              "the description is Rust's own wording (handlers.rs:380)");
    }

    rb_effect_free(cost);
}

/* handlers.rs:370-377: an EMPTY hand opens NOTHING and auto-skips. */
static void test_offer_all_hand_discard_empty_hand_opens_nothing(void)
{
    test_game_new(&tg);
    AbilityEffect *cost = make_effect();
    cost->action = dup_s("move_cards");
    cost->source = dup_s("hand");
    cost->destination = dup_s("discard");
    fx_set(cost, "all", "true");

    rb_pay_cost(&tg.state, 0, cost);

    CHECK_EQ(tg.state.queue.has_pending, 0,
             "an empty hand auto-skips and opens NO choice (handlers.rs:370-377)");
    rb_effect_free(cost);
}

/* ══════════════════════════════════════════════════════════════════════════
 * 2. pay_cost_all_discard -- the per-card movement events.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/ability/cost/handlers.rs:1683-1706:
 *     let target_str = cost.as_ref().and_then(|c| c.target.as_deref())
 *                          .unwrap_or("self");
 *     let source     = cost.as_ref().and_then(|c| c.source_str())
 *                          .unwrap_or("hand");
 *     let player = gs.resolve_target_player_mut(target_str);
 *     let hand_ids: Vec<i16> = player.hand.cards.drain(..).collect();
 *     player.waitroom.cards.extend(hand_ids.iter().copied());
 *     for &cid in &hand_ids {
 *         gs.push_movement_event(cid, source, "discard", None, target_str, false);
 *     }
 *
 * `effect_only` is Rust's literal `false`: cost payment is a player action,
 * not an effect (handlers.rs:1702-1704). The C folded this to ZERO events, so
 * every cost-discard move was missing from batch_movements / recently_moved /
 * cards_moved_this_turn. This is the assertion that catches its return.
 */
static void test_pay_cost_all_discard_records_one_event_per_card(void)
{
    test_game_new(&tg);
    int a = test_id(&tg, "PL!N-bp5-021-N");
    int b = test_id(&tg, "PL!N-bp5-028-L");
    CHECK(a >= 0 && rb_card_no_eq(a, "PL!N-bp5-021-N") &&
          b >= 0 && rb_card_no_eq(b, "PL!N-bp5-028-L"),
          "both fixture cards are exactly the card_no they claim "
          "(rb_card_no_eq, an exact string compare)");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);

    int before_batch = tg.state.n_batch_movements;
    int before_recent = tg.state.n_recently_moved;

    rb_handle_pay_cost_all_discard(&tg.state, 0, "1");

    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "the actor's whole hand is drained (handlers.rs:1696)");
    CHECK_EQ(tg.state.p[0].discard.n, 2,
             "and every card lands in the waitroom (handlers.rs:1697-1700)");
    CHECK_EQ(tg.state.n_batch_movements - before_batch, 2,
             "ONE movement event per discarded card (handlers.rs:1701-1706) -- "
             "the C recorded none, so this delta was 0");
    CHECK_EQ(tg.state.n_recently_moved - before_recent, 2,
             "and both ids reach recently_moved (modifiers.rs:2060-2061)");
}

/* The same arm with NO cost at all. Rust's `unwrap_or("self")` means a
 * missing cost is the ACTOR's own hand, never a crash and never the
 * opponent's -- the C's original NULL deref lived exactly here, on the 登場
 * path where no queue entry exists so rb_entry_cost returns NULL. */
static void test_pay_cost_all_discard_without_a_cost_is_the_actors_hand(void)
{
    test_game_new(&tg);
    int a = test_id(&tg, "PL!N-bp5-021-N");
    int b = test_id(&tg, "PL!N-bp5-028-L");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    int opp_hand_before = tg.state.p[1].hand.n;

    /* No queue entry, no cost: rb_entry_cost(g) is NULL here. */
    int r = rb_handle_pay_cost_all_discard(&tg.state, 0, "1");
    CHECK_EQ(r, 1, "a missing cost still pays -- unwrap_or(\"self\"), "
                   "handlers.rs:1687-1690, never a NULL deref");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "the ACTOR's hand is the one discarded");
    CHECK_EQ(tg.state.p[1].hand.n, opp_hand_before,
             "the opponent's hand is untouched (unwrap_or(\"self\"))");
}

/* handlers.rs:1677: `selected != "skip_optional_cost" && selected != "0"`. */
static void test_pay_cost_all_discard_declining_discards_nothing(void)
{
    test_game_new(&tg);
    int a = test_id(&tg, "PL!N-bp5-021-N");
    if (a < 0) return;
    test_add_to_hand(&tg, a);

    rb_handle_pay_cost_all_discard(&tg.state, 0, "skip_optional_cost");
    CHECK_EQ(tg.state.p[0].hand.n, 1,
             "a declined cost discards nothing (handlers.rs:1677, :1682)");
    CHECK_EQ(tg.state.p[0].discard.n, 0,
             "and pushes no movement event for it either");
}

/* ══════════════════════════════════════════════════════════════════════════
 * 3. pay_cost_change_state -- "wait" only, and the empty-hand refusal.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/ability/cost/handlers.rs:917:
 *     if state_change == "wait" { ... }
 * ...and handlers.rs:936-938:
 *     if candidates.is_empty() {
 *         return Err("No matching members on stage to change state".to_string());
 *     }
 *
 * The C additionally matched "rest"/"rested" and wrote orientation "rest",
 * which no Rust cost site can produce. REACHABILITY of that arm: measured 0
 * of 985 corpus abilities carry "rest"/"rested" anywhere, and state_change on
 * a COST node is "wait" 53 of 53 -- so this is VERIFIED-NOOP: 0. The guard
 * is Rust's regardless, so a regenerated corpus cannot reopen the hole. */
static void test_change_state_ignores_a_non_wait_state_change(void)
{
    test_game_new(&tg);
    int id = test_id(&tg, "PL!-bp4-003-P");
    CHECK(id >= 0 && rb_card_no_eq(id, "PL!-bp4-003-P"),
          "fixture member is exactly PL!-bp4-003-P (rb_card_no_eq)");
    if (id < 0) return;
    test_add_to_stage(&tg, 1, id);
    test_recalc(&tg);

    AbilityEffect *cost = make_effect();
    cost->action = dup_s("change_state");
    cost->target = dup_s("self");
    fx_set(cost, "state_change", "rest");

    int r = rb_pay_cost_change_state(&tg.state, 0, cost, id, 0);
    CHECK_EQ(r, 1,
             "a non-\"wait\" state_change is a no-op that SUCCEEDS "
             "(handlers.rs:917 falls through to Ok(()) at :970)");
    /* The observable the removed `rest` branch used to write. Reading the BLADE
     * modifier here would pass with or without the fix, which is why the
     * mutation test caught nothing at first. */
    const char *orient = rb_mods_get_orientation(&tg.state.mods, id);
    CHECK(orient == NULL ||
              (strcmp(orient, "active") == 0 && strcmp(orient, "wait") != 0),
          "and ORIENTATION is untouched (NULL or \"active\") -- Rust's mandatory "
          "arm writes only \"wait\" (handlers.rs:947), and no Rust site writes "
          "\"rest\" from a cost; the C's rest/rested branch did");
    CHECK_EQ(tg.state.n_last_cost_waited_members, 0,
             "and nothing joins last_cost_waited_members (handlers.rs:948 is "
             "inside the wait arm only)");
    rb_effect_free(cost);
}

/* The wait arm itself must still work, so the guard above cannot be satisfied
 * by simply refusing everything. handlers.rs:940-949. */
static void test_change_state_still_waits_a_mandatory_cost(void)
{
    test_game_new(&tg);
    int id = test_id(&tg, "PL!-bp4-003-P");
    if (id < 0) return;
    test_add_to_stage(&tg, 1, id);
    test_recalc(&tg);

    AbilityEffect *cost = make_effect();
    cost->action = dup_s("change_state");
    cost->target = dup_s("self");
    fx_set(cost, "state_change", "wait");
    cost->count = 1;

    int r = rb_pay_cost_change_state(&tg.state, 0, cost, id, 0);
    CHECK_EQ(r, 1, "a mandatory wait cost pays (handlers.rs:940-949)");
    CHECK_EQ(tg.state.n_last_cost_waited_members, 1,
             "and records the member in last_cost_waited_members "
             "(handlers.rs:948)");
    rb_effect_free(cost);
}

/* handlers.rs:936-938: no active candidate => the cost cannot be paid. */
static void test_change_state_refuses_with_no_eligible_member(void)
{
    test_game_new(&tg);
    AbilityEffect *cost = make_effect();
    cost->action = dup_s("change_state");
    cost->target = dup_s("self");
    fx_set(cost, "state_change", "wait");
    cost->count = 1;

    int r = rb_pay_cost_change_state(&tg.state, 0, cost, -1, 0);
    CHECK_EQ(r, 0,
             "with no active member the cost is REFUSED, not silently paid "
             "(handlers.rs:936-938)");
    rb_effect_free(cost);
}

/* ══════════════════════════════════════════════════════════════════════════
 * 4. The six RbCardFilter carriers -- produced, or accounted for.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/ability/util.rs:1868-1933 (CardFilter::from_effect):
 *     exclude_group_names: exclude_group_names.map(Vec::as_slice),   :1884
 *     groups:  group_names.map(Vec::as_slice),                       :1874
 *     cost_limit_min:  effect.cost_limit_min_any(),                  :1878
 *     cost_limit_max:  effect.cost_limit_max_any(),                  :1879
 *     original_heart_total:          effect.heart_total_limit_any(), :1924
 *     original_heart_total_operator: ...or(Some("<=")),              :1925
 *     exclude_cards: None,                                           :1926
 *     exclude_names: None,                                           :1927
 *
 * The last two are quoted `None` AT THIS SITE, so they are not producer
 * sites at all: they are filled programmatically after construction, which is
 * also why `has_filter` does not count them (util.rs:1456-1477). The C matches
 * by leaving them zero, and this test states that as the reason rather than as
 * an omission.
 *
 * MEASURED corpus reachability (985 unique abilities, nested keys walked):
 * exclude_group_names 4, cost_limit_min 1, cost_limit_max 1,
 * heart_total_limit 5, group_names 605, exclude_cards 0, exclude_names 0.
 */
static void test_filter_carrier_exclude_group_names_is_produced(void)
{
    AbilityEffect *e = make_effect();
    fx_set(e, "exclude_group_names", "スリーズブーケ");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_STR(f.exclude_group_names, "スリーズブーケ",
              "exclude_group_names reaches the filter (util.rs:1884)");
    CHECK_EQ(f.has_filter, 1,
             "and has_filter counts it (util.rs:1460), so the filter is built");
    rb_effect_free(e);
}

static void test_filter_carrier_groups_is_produced(void)
{
    AbilityEffect *e = make_effect();
    fx_set(e, "group_names", "μ's,Aqours");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_EQ(f.n_groups, 2,
             "groups[] carries the WHOLE list, not just .first() (util.rs:1874)");
    CHECK_STR(f.groups[0], "μ's", "groups[0] is the first listed group");
    CHECK_STR(f.groups[1], "Aqours", "groups[1] is the second");
    /* util.rs:1870-1873 puts .first() in `group`; the C stores the WHOLE
     * CSV there too and splits at match time (util.c:1753-1761,
     * rb_group_name_first). The FIELD differs, the VERDICT does not, and
     * changing the field would mean moving the split into every caller. */
    CHECK_STR(f.group, "μ's,Aqours",
              "`group` holds the whole comma-joined list (util.c:1753-1756); "
              "the matcher splits it, which is verdict-identical to Rust's "
              ".first() at util.rs:1870-1873");
    CHECK_EQ(f.has_group, 1, "and `group` counts in has_filter (util.rs:1458)");
    rb_effect_free(e);
}

static void test_filter_carrier_cost_limit_min_max_is_produced(void)
{
    /* util.rs:1878-1879 -- separate Option<u8>s, passed to the matcher with
     * their operators literally, NOT folded into `cost_limit`. */
    AbilityEffect *e = make_effect();
    fx_set(e, "cost_limit_min", "3");
    fx_set(e, "cost_limit_max", "9");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_EQ(f.has_cost_limit_min, 1, "cost_limit_min is produced (util.rs:1878)");
    CHECK_EQ(f.cost_limit_min, 3, "with its value");
    CHECK_EQ(f.has_cost_limit_max, 1, "cost_limit_max is produced (util.rs:1879)");
    CHECK_EQ(f.cost_limit_max, 9, "with its value");
    rb_effect_free(e);
}

static void test_filter_carrier_original_heart_total_is_produced(void)
{
    /* util.rs:1924-1925 -- NOT gated on original_value, unlike the blade
     * bounds, and the operator defaults to "<=" so it is ALWAYS Some whenever
     * the total is. */
    AbilityEffect *e = make_effect();
    fx_set(e, "heart_total_limit", "4");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_EQ(f.original_heart_total, 4,
             "original_heart_total is produced from heart_total_limit "
             "(util.rs:1924)");
    CHECK_STR(f.original_heart_operator, "<=",
              "and the operator defaults to \"<=\" when the wire omits it "
              "(util.rs:1925 `.or(Some(\"<=\"))`)");
    CHECK_EQ(f.has_filter, 1,
             "has_filter counts the total (util.rs:1471) -- util.c's matcher "
             "keys off the operator string as the presence flag");

    /* An explicit operator is passed through verbatim. */
    AbilityEffect *e2 = make_effect();
    fx_set(e2, "heart_total_limit", "4");
    fx_set(e2, "heart_total_operator", ">=");
    RbCardFilter f2;
    memset(&f2, 0, sizeof(f2));
    rb_effect_filter_subset(e2, &f2);
    CHECK_STR(f2.original_heart_operator, ">=",
              "an explicit heart_total_operator is not overwritten (util.rs:1925)");
    rb_effect_free(e2);
    rb_effect_free(e);
}

/* The two carriers Rust quotes as None at the from_effect site. They are NOT
 * deleted from the struct -- the matchers at util.c read them, and Rust's
 * `matches` reads its own -- but they have no producer here BY DESIGN. */
static void test_filter_carriers_rust_quotes_none_stay_empty(void)
{
    /* util.rs:1926-1927: exclude_cards: None, exclude_names: None. */
    AbilityEffect *e = make_effect();
    fx_set(e, "group_names", "μ's");
    /* Even if the wire carried these keys, there is no producer. */
    fx_set(e, "exclude_cards", "123");
    fx_set(e, "exclude_names", "高坂穂乃果");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_EQ(f.n_exclude_cards, 0,
             "exclude_cards has NO producer, matching util.rs:1926's literal "
             "`None` (it is filled programmatically after construction)");
    CHECK_EQ(f.n_exclude_names, 0,
             "exclude_names likewise, matching util.rs:1927's literal `None`");
    /* And they must not change has_filter: util.rs:1456-1477 does not count
     * them even though matches() reads them (util.rs:1489-1513). */
    CHECK_EQ(f.has_filter, 1,
             "has_filter still says yes -- on group_names, not on the two "
             "programmatic carriers");
    rb_effect_free(e);
}

/* util.rs:1864-1867: an inclusion equal to the exclusion is dropped, because
 * the two can never both be satisfied. This is the 百生 吟子 node. */
static void test_filter_group_names_dropped_when_equal_to_the_exclusion(void)
{
    AbilityEffect *e = make_effect();
    fx_set(e, "group_names", "スリーズブーケ");
    fx_set(e, "exclude_group_names", "スリーズブーケ");
    RbCardFilter f;
    memset(&f, 0, sizeof(f));
    rb_effect_filter_subset(e, &f);
    CHECK_EQ(f.n_groups, 0,
             "the redundant inclusion is dropped (util.rs:1864-1867) so the "
             "candidate set is not permanently empty");
    CHECK_STR(f.exclude_group_names, "スリーズブーケ",
              "while the exclusion -- the intended reading -- survives");
    CHECK_EQ(f.has_filter, 1, "and the filter is still built (util.rs:1460)");
    rb_effect_free(e);
}

/* ══════════════════════════════════════════════════════════════════════════
 * 5. map_series_to_group across EVERY group and every series form.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/core/card.rs:925-936 -- the complete match, all five
 * groups and both 蓮ノ空 spellings, plus the `_ => Box::from("")` default.
 *
 * A peer verified only 虹ヶ咲 and 蓮ノ空; a group mismatch is silent, so this
 * covers the remaining three groups, both Hasuno forms, and the default arm
 * for every series form the corpus carries that is NOT one of the five.
 */
static void test_series_to_group_every_group_and_form(void)
{
    /* NOTE the long-vowel ラブライブ！ spelling: both engines carry it and
     * any "corrected" ASCII form matches nothing. */
    CHECK_STR(rb_card_series_to_group("ラブライブ！"), "μ's",
              "card.rs:927 -- μ's");
    CHECK_STR(rb_card_series_to_group("ラブライブ！サンシャイン!!"), "Aqours",
              "card.rs:928 -- Aqours");
    CHECK_STR(rb_card_series_to_group("ラブライブ！虹ヶ咲学園スクールアイドル同好会"),
              "虹ヶ咲", "card.rs:929 -- 虹ヶ咲");
    CHECK_STR(rb_card_series_to_group("ラブライブ！スーパースター!!"), "Liella!",
              "card.rs:930 -- Liella!");
    CHECK_STR(rb_card_series_to_group("蓮ノ空女学院スクールアイドルクラブ"), "蓮ノ空",
              "card.rs:931 (first form) -- 蓮ノ空");
    CHECK_STR(rb_card_series_to_group("ラブライブ！蓮ノ空女学院スクールアイドルクラブ"),
              "蓮ノ空", "card.rs:931 (second form) -- 蓮ノ空");

    /* card.rs:934 -- the default arm. Every other series form, including the
     * ones the C might be tempted to guess at, must answer "". */
    CHECK_STR(rb_card_series_to_group("Hello!Project"), "",
              "card.rs:934 -- an unlisted series maps to \"\", not to a guess");
    CHECK_STR(rb_card_series_to_group(""), "",
              "card.rs:934 -- the empty string is not one of the five");
    CHECK_STR(rb_card_series_to_group(NULL), "",
              "a NULL series is \"\" too, never a crash");

    /* The out-file spelling must agree with the pointer form for every one of
     * the six, because rb_map_series_to_group is a separate code path. */
    const char *series[6];
    series[0] = "ラブライブ！";
    series[1] = "ラブライブ！サンシャイン!!";
    series[2] = "ラブライブ！虹ヶ咲学園スクールアイドル同好会";
    series[3] = "ラブライブ！スーパースター!!";
    series[4] = "蓮ノ空女学院スクールアイドルクラブ";
    series[5] = "ラブライブ！蓮ノ空女学院スクールアイドルクラブ";
    for (int i = 0; i < 6; i++) {
        char out[64];
        out[0] = 0;
        rb_map_series_to_group(series[i], out, sizeof(out));
        CHECK_STR(out, rb_card_series_to_group(series[i]),
                  "rb_map_series_to_group agrees with rb_card_series_to_group "
                  "on every one of the six series forms");
    }
}

/* ══════════════════════════════════════════════════════════════════════════
 * 6. CardProperty -- the fourth variant, and the corpus node that needs it.
 * ════════════════════════════════════════════════════════════════════════ */

/* Rust, engine/src/core/card.rs:2750-2785: FOUR variants. The C tables had
 * three, so index 3 (`has_draw_icon`) round-tripped to index 0
 * (`has_blade_heart`). The live C path does not go through these tables --
 * rb_check_card_property matches the raw string -- so the corpus does not
 * move, but abilities.json:13305 (PL!-pb2-001-{PP,R} ab#0, 高坂穂乃果) does
 * carry the string and 1 of 49 card_property nodes is it. */
static void test_card_property_draw_icon_round_trips(void)
{
    CHECK_STR(rb_card_property_str(3), "has_draw_icon",
              "card.rs:2750-2765 -- HasDrawIcon is the FOURTH variant, not the "
              "third; without it index 3 answered has_blade_heart");
    CHECK_EQ(rb_card_property_from_str("has_draw_icon"), 3,
             "and from_str decodes it back to 3 (card.rs:2778-2785)");
    for (int i = 0; i < 4; i++) {
        const char *s = rb_card_property_str(i);
        CHECK(s && s[0], "every CardProperty index 0..3 has a name");
        if (!s || !s[0]) continue;
        CHECK_EQ(rb_card_property_from_str(s), i,
                 "CardProperty round-trips through its own name");
    }
}

/* ════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    printf("=== test_parity_conc_cost_card: cost/card concentration ===\n");

    /* The card database MUST be loaded, or every test_id in this file returns
     * -1 and the suite reads green while asserting nothing -- a broken build
     * and a corpus-less build look identical from the outside. Refuse rather
     * than pass vacuously. */
    if (rb_load("src") != 0) {
        printf("FAIL: rb_load(\"src\") failed -- without the corpus every "
               "card lookup is vacuous\n");
        printf("      Rust: engine/tests/test_modules/rules/"
               "group_name_equivalence.rs:65 -- load_real_database()\n");
        return 2;
    }

    test_game_new(&tg);
    test_offer_all_hand_discard_sets_the_routing_target();
    test_offer_all_hand_discard_empty_hand_opens_nothing();

    test_pay_cost_all_discard_records_one_event_per_card();
    test_pay_cost_all_discard_without_a_cost_is_the_actors_hand();
    test_pay_cost_all_discard_declining_discards_nothing();

    test_change_state_ignores_a_non_wait_state_change();
    test_change_state_still_waits_a_mandatory_cost();
    test_change_state_refuses_with_no_eligible_member();

    test_filter_carrier_exclude_group_names_is_produced();
    test_filter_carrier_groups_is_produced();
    test_filter_carrier_cost_limit_min_max_is_produced();
    test_filter_carrier_original_heart_total_is_produced();
    test_filter_carriers_rust_quotes_none_stay_empty();
    test_filter_group_names_dropped_when_equal_to_the_exclusion();

    test_series_to_group_every_group_and_form();
    test_card_property_draw_icon_round_trips();

    printf("\n%d assertions, %d failures\n", assertions, failures);
    if (failures == 0) {
        printf("PASSED: test_parity_conc_cost_card\n");
        return 0;
    }
    printf("FAILED: test_parity_conc_cost_card (%d failures)\n", failures);
    return 1;
}
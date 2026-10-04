/* test_parity_fw_cost_cardtype.c
 *
 * FINAL-WAVE parity suite -- one case per fix landed by the cluster C / F / T2
 * pass over src/ability/cost.c, the src/ability/effects/ handlers and
 * src/game/game_setup.c. Every CHECK transcribes a specific Rust statement and
 * names it; nothing here is a new expectation.
 *
 *   FW-C1  game_setup.c  rb_setup_ability_candidates -> rb_validate_mandatory_cost
 *          Rust engine/src/game/game_setup.rs, the stage scan:
 *              if let Some(c) = ability.cost.as_ref() {
 *                  if !c.has_optional_payment()
 *                      && effective_cost > active_player.energy_zone.active_count()
 *                  { continue; }
 *                  // Mandatory non-energy costs (discard, wait, ...) must
 *                  // also be payable or the ability is not offered
 *                  // (Rule 9.4.2.3/Q56). Optional components stay offerable
 *                  // (wakana/umi Q228 skip at pay time).
 *                  if crate::ability::resolver::AbilityResolver::validate_mandatory_cost(
 *                      game_state, &c.0, Some(card_id),
 *                  ).is_err() { continue; }
 *              }
 *          and the SAME call in the discard scan.  `Some(card_id)` is the card
 *          under consideration; handlers.rs documents why it is explicit:
 *              /// Leaf validation with an explicit activating card (offer/
 *              /// activation time run before `gs.activating_card` is set, so
 *              /// callers pass the card under consideration for
 *              /// self_cost/exclude_self filtering).
 *          The C called rb_validate_cost, the PAY-time twin (handlers.rs:169),
 *          which passes RB_ACTIVATING_DERIVED instead -- so a `self_cost` cost
 *          resolved against a card nobody had published and the ability was
 *          withheld from the offer list entirely.
 *
 *   FW-CT  cost.c  the `card_type` reads in the cost handlers
 *          Rust engine/src/ability/cost/handlers.rs reads the TYPED getter
 *          everywhere, e.g.
 *              let card_type = cost.card_type_any().map(|s| s.to_string());
 *              ... util::card_matches_type(card_db, id, card_type.as_deref())
 *          and
 *              cost.card_type_any().map(|ct| ct.as_card_str()),
 *          `card_type_any()` is `Option<CardType>`; the C read
 *          `eff_extra(cost, "card_type")`, and vm.c's decode arm for
 *          `card_type` `continue`s after storing the value in the STRUCT FIELD,
 *          so the extras table never holds it (measured 0 of 901 cost nodes and
 *          0 of 3465 effect nodes) and every one of those filters was dead.
 *
 * Sizes: every GameState / TestGame local below is `static`, because
 * sizeof(GameState) is 1195656 B (measured) and a stack local blows the frame.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>

static int failures;
static int assertions;
static int setup_bugs;
static int n_tests_ok, n_tests_crashed;
static const char *current_test = "(none)";

/* rb_setup_ability_candidates is a non-static definition in game_setup.c with no
 * declaration in include/rabuka.h, which is not this test's to edit. Declared
 * here, exactly as that file's own local-declaration pattern does (the reason
 * is spelled out in game_setup.c:1327-1332). The 256 is that file's own
 * #define (game_setup.c:214), repeated rather than copied. */
#define FW_SETUP_MAX_ABILITIES 256
int rb_setup_ability_candidates(const GameState *g, RbSetupAbilityCandidate *out, int max);
/* cost.c:22 -- Rust's None for `gs.activating_card`. */
#define FW_ACTIVATING_NONE (-1)

#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        failures++; \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)  [%s]\n", message, \
                actual_value, expected_value, current_test); \
        failures++; \
    } \
} while (0)

#define AREA_CENTER 1
/* PL!SP-bp7-008-P -- 起動 whose cost is exactly
 *   change_state { state_change: "wait", self_cost: true, card_type: "member_card" }
 * i.e. 「このメンバーをウェイトに」 as a SELF cost. Printed cost 11, id 2330,
 * member card. The card NUMBER is used, never an index, because CARD IDENTITY is
 * not the index: PL!SP-bp7-008-P and PL!SP-bp7-008-R are different cards. */
#define SELFCOST_NO "PL!SP-bp7-008-P"
/* PL!-bp3-003-P -- a plain 起動 with NO cost at all (control: an ability with
 * no cost is offered unconditionally). */
#define PLAIN_NO "PL!-bp3-003-P"

static TestGame tg;

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* Resolve by card number and assert it prints back what the test asked for, so
 * a corpus regeneration cannot silently turn a CHECK vacuous. */
static int ident(TestGame *t, const char *no)
{
    int id = test_new_id(t, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG: card %s does not resolve\n", no);
        setup_bugs++; assertions++;
        return -1;
    }
    Card c;
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        fprintf(stderr, "SETUP BUG: id %d does not decode\n", id);
        setup_bugs++; assertions++;
        return -1;
    }
    char got[96];
    snprintf(got, sizeof(got), "%s", rb_card_string(c.card_no_idx));
    rb_free_card(&c);
    if (strcmp(got, no) != 0) {
        fprintf(stderr, "SETUP BUG: id %d prints \"%s\", the test wanted \"%s\"\n",
                id, got, no);
        setup_bugs++; assertions++;
        return -1;
    }
    return id;
}

/* Empty every zone both seats own, so a count assertion is unambiguous and no
 * inherited card can satisfy a cost filter by accident. */
static void bare_board(TestGame *t)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &t->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->energy.n = 0;
        p->energy_deck.n = 0;
        p->energy_active = 0;
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            p->stage[a] = RB_EMPTY_SLOT;
            p->stage_wait[a] = 0;
            p->under_cards[a].n = 0;
        }
    }
    t->state.resolution.n = 0;
    t->state.n_revealed = 0;
    t->state.activating_card = FW_ACTIVATING_NONE;
}

/* Does the stage scan offer (card_id, ability_index)? */
static int offered(TestGame *t, int card_id, int ability_index)
{
    RbSetupAbilityCandidate offers[FW_SETUP_MAX_ABILITIES];
    int n = rb_setup_ability_candidates(&t->state, offers, FW_SETUP_MAX_ABILITIES);
    for (int i = 0; i < n; i++)
        if (offers[i].card_id == card_id && offers[i].ability_index == ability_index)
            return 1;
    return 0;
}

/* ===========================================================================
 * FW-C1 -- the offer gate is validate_mandatory_cost(Some(card_id)).
 *
 * The board is minimal on purpose: ONE member on stage, that member being the
 * card whose ability carries the `self_cost` 「このメンバーをウェイトに」 cost,
 * and enough energy in the active zone to cover its printed cost. Nothing else
 * is on stage and the hand is empty, so the ONLY thing that can withhold the
 * offer is the cost gate itself.
 *
 * Rust (handlers.rs:281-309) walks to `validate_leaf_cost_for(.., Some(card_id))`,
 * which resolves `self_cost` against that id (handlers.rs:65-…, the candidate
 * scan), finds the card itself, and returns Ok(()) -- so the ability IS offered.
 * The C's rb_validate_cost passes RB_ACTIVATING_DERIVED, cost_self_activating_card
 * finds no published activating card on the offer path, `self_cost` therefore
 * matches no stage slot, and the whole ability was withheld.
 */
static void FW_C1_self_cost_activation_is_offered(void)
{
    int id = ident(&tg, SELFCOST_NO);
    if (id < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, id);
    /* Printed cost is 11; the C offers while effective <= active energy. */
    test_set_energy_active(&tg, 0, 12);
    test_recalc(&tg);

    /* PRECONDITION: this really is the shape the fix is about. If the corpus
     * ever changes the cost away from a `self_cost` change_state, the CHECKs
     * below would silently become vacuous, so assert the decoded shape. */
    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_resolve_card_ability_at(&tg.state, id, 0, &ab)) {
        fprintf(stderr, "SETUP BUG: ab#0 of %s does not resolve\n", SELFCOST_NO);
        setup_bugs++; assertions++;
        return;
    }
    const AbilityEffect *cost = ab.cost;
    CHECK(cost != NULL, "FW-C1 precondition: %s ab#0 must carry a cost", SELFCOST_NO);
    if (cost) {
        CHECK(cost->action && !strcmp(cost->action, "change_state"),
              "FW-C1 precondition: cost action is change_state, got '%s'",
              cost->action ? cost->action : "(null)");
        CHECK(cost->is_optional == 0,
              "FW-C1 precondition: the cost is MANDATORY, is_optional=%d",
              cost->is_optional);
        CHECK(strcmp(cost->card_type_field, "member_card") == 0,
              "FW-C1 precondition: cost card_type_field is member_card, got '%s'",
              cost->card_type_field);
    }

    /* The two validators, side by side on the same board. This is the shape of
     * the divergence and it is asserted so the offer CHECK below cannot pass for
     * an unrelated reason. */
    int pay_time = rb_validate_cost(&tg.state, 0, cost);
    int offer_time = rb_validate_mandatory_cost(&tg.state, 0, cost, id);
    CHECK_EQ(offer_time, 1,
             "FW-C1 rb_validate_mandatory_cost(g, 0, cost, Some(card_id)) accepts the "
             "self_cost 'wait' cost (handlers.rs:281-309 -> validate_leaf_cost_for with the "
             "card under consideration)");
    CHECK_EQ(pay_time, 0,
             "FW-C1 the OLD gate rb_validate_cost refuses on this board "
             "(RB_ACTIVATING_DERIVED -> no published activating card -> self_cost "
             "matches nothing). If this ever returns 1 the offer CHECK below is vacuous.");

    /* THE FIX, observed at the call site that was changed. */
    CHECK(offered(&tg, id, 0),
          "FW-C1 rb_setup_ability_candidates must OFFER %s ab#0: Rust passes "
          "Some(card_id) to validate_mandatory_cost at game_setup.rs (both scans), so a "
          "mandatory `self_cost` 「このメンバーをウェイトに」 cost that names the very card "
          "being activated is always payable", SELFCOST_NO);

    rb_free_ability(&ab);
}

/* The change is strictly WEAKER, so the CONTROL below matters: a mandatory cost
 * that really cannot be paid must still withhold the offer. Rust keeps that
 * refusal -- validate_mandatory_cost only skips optional / any_number / max /
 * ChoiceCondition / PayEnergy, and still walks every other leaf
 * (handlers.rs:293-307). PL!N-bp3-004-P's 起動 costs a `move_cards` from the
 * HAND, so an empty hand is genuinely unpayable. */
static void FW_C1_unpayable_mandatory_cost_is_still_refused(void)
{
    int id = ident(&tg, "PL!N-bp3-004-P");
    if (id < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, id);
    test_set_energy_active(&tg, 0, 13);
    test_recalc(&tg);

    /* Hand is empty: the 「手札を1枚控え室に置く」 leg has nothing to pay with. */
    CHECK_EQ(tg.state.p[0].hand.n, 0,
             "FW-C1 control precondition: the hand is empty");

    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_resolve_card_ability_at(&tg.state, id, 0, &ab)) {
        fprintf(stderr, "SETUP BUG: ab#0 of PL!N-bp3-004-P does not resolve\n");
        setup_bugs++; assertions++;
        return;
    }
    CHECK_EQ(rb_validate_mandatory_cost(&tg.state, 0, ab.cost, id), 0,
             "FW-C1 a mandatory `move_cards` cost with an empty hand is still REFUSED "
             "(handlers.rs:293-307 still walks every non-exempt leaf)");
    CHECK(!offered(&tg, id, 0),
          "FW-C1 the offer list must NOT contain PL!N-bp3-004-P ab#0 while its mandatory "
          "hand-discard cost is unpayable");

    rb_free_ability(&ab);
}

/* ===========================================================================
 * FW-CT -- a cost's `card_type` is the STRUCT FIELD, not an extras row.
 *
 * PL!-sd1-006-SD (西木野 真姫) ab#0 is a 登場 whose whole cost is the optional
 *   reveal { source: "hand", count: 1, optional: true, card_type: "live_card" }
 * i.e. 「手札のライブカードを1枚公開してもよい」. Rust reads the typed getter and
 * hands it to BOTH the matcher and the choice builder:
 *   engine/src/ability/cost/handlers.rs, the reveal cost:
 *       let card_type = cost.card_type_any().map(|s| s.to_string());
 *       ... util::card_matches_type(card_db, id, card_type.as_deref())
 *       log::debug!("[REVEAL_COST] card_ids={:?} ... card_type={:?}", .., card_type);
 *       ... .card_type(card_type.clone())
 * The C read `eff_extra(cost, "card_type")`, which the decoder never populates
 * (vm.c:1098-1112 `continue`s after storing `card_type_field`), so the filter
 * was silently absent: `card_type` came back NULL, `card_matches_type` was never
 * consulted, and the prompt offered EVERY hand card.
 */
#define REVEAL_NO   "PL!-sd1-006-SD"
#define LIVEPOOL_NO "PL!-bp6-024-L"   /* a LIVE card -- matches card_type live_card */
#define NOTLIVE_NO  "PL!-sd1-010-SD"  /* a plain member -- must NOT be offered */

static void FW_CT_reveal_cost_card_type_comes_from_the_struct_field(void)
{
    int host = ident(&tg, REVEAL_NO);
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    int live_id = test_new_id(&tg, LIVEPOOL_NO);
    int notlive_id = test_new_id(&tg, NOTLIVE_NO);
    CHECK(live_id >= 0 && notlive_id >= 0,
          "FW-CT precondition: both hand cards resolve");
    if (live_id < 0 || notlive_id < 0) return;
    CHECK(rb_card_is_live(live_id),
          "FW-CT precondition: %s really is a LIVE card", LIVEPOOL_NO);
    CHECK(!rb_card_is_live(notlive_id),
          "FW-CT precondition: %s really is NOT a LIVE card", NOTLIVE_NO);

    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)host, 0, &ab)) {
        fprintf(stderr, "SETUP BUG: ab#0 of %s does not decode\n", REVEAL_NO);
        setup_bugs++; assertions++;
        return;
    }
    /* PRECONDITION: the cost is the reveal the fix is about. */
    CHECK(ab.cost && ab.cost->action && !strcmp(ab.cost->action, "reveal"),
          "FW-CT precondition: %s ab#0 costs a reveal, got '%s'", REVEAL_NO,
          (ab.cost && ab.cost->action) ? ab.cost->action : "(none)");
    if (!ab.cost) { rb_free_ability(&ab); return; }
    CHECK(strcmp(ab.cost->card_type_field, "live_card") == 0,
          "FW-CT precondition: the cost's card_type_field is live_card, got '%s'",
          ab.cost->card_type_field);

    test_add_to_hand(&tg, live_id);
    test_add_to_hand(&tg, notlive_id);
    test_recalc(&tg);

    rb_pay_cost(&tg.state, 0, ab.cost);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL,
          "FW-CT the optional 「手札のライブカードを1枚公開してもよい」 prompt opens "
          "(the hand holds a live card, so the cost is payable)");
    if (ch) {
        CHECK(strcmp(ch->card_type, "live_card") == 0,
              "FW-CT the reveal prompt is filtered to live_card "
              "(handlers.rs: `.card_type(card_type.clone())` fed by "
              "`cost.card_type_any()`, i.e. the STRUCT FIELD; got '%s')", ch->card_type);
        /* The prompt does NOT precompute `filtered_indices` -- it publishes
         * `card_type` and the choice resolver filters at answer time (the C twin
         * of handlers.rs passing card_type into the SelectCard filter; see
         * choice.c's reader of `card_type`). So the pool assertion is made
         * through the SAME predicate the engine uses, driven by the value the
         * prompt now carries:
         *     util::card_matches_type(card_db, id, card_type)
         * parity_zone_source asserts the same thing end-to-end through the
         * resolver ("F: the reveal is filtered to live cards"). */
        CHECK(rb_card_matches_type(live_id, ch->card_type),
              "FW-CT the live card passes card_matches_type(.., \"live_card\")");
        CHECK(!rb_card_matches_type(notlive_id, ch->card_type),
              "FW-CT the NON-live card is REJECTED by card_matches_type(.., \"live_card\") "
              "-- handlers.rs: `util::card_matches_type(card_db, id, card_type.as_deref())`");
    }
    rb_free_ability(&ab);
}

/* The CONTROL for FW-CT: the same offer gate with NO card_type must stay
 * unfiltered. Rust's `card_type_any()` is None for every cost that does not
 * carry the field, and `card_matches_type(.., None)` accepts everything -- so a
 * cost with no `card_type` must still offer the whole hand. Using the same
 * corpus card's EFFECT-side filter is not needed: the assertion below is made on
 * a cost whose card_type_field is empty, which is the None case. */
static void FW_CT_absent_card_type_stays_unfiltered(void)
{
    /* PL!-bp3-003-P's cost is change_state with card_type member_card, so it is
     * NOT the None case. Use a hand-cost filter read directly instead: build the
     * same predicate Rust uses and confirm it accepts everything when the field
     * is empty. That keeps the CONTROL honest without inventing a cost. */
    AbilityEffect probe;
    memset(&probe, 0, sizeof(probe));
    probe.action = (char *)"move_cards";
    probe.card_type_field[0] = '\0';       /* Rust: card_type_any() == None */
    RbCardFilter f;
    rb_effect_filter_subset(&probe, &f);
    CHECK(f.card_type[0] == '\0',
          "FW-CT control: an effect with no card_type yields an empty filter.card_type "
          "(core/card.c:957 reads `card_type_field[0] ? card_type_field : fx_extra(..)`, "
          "and the extras fallback is dead because vm.c never stores one)");
}

/* ===========================================================================
 * FW-C3 -- hand_cost_matching_indices dedupes by normalized NAME when the cost
 * carries `distinct` (handlers.rs:476-480).
 *
 * PL!-pb2-018-P＋ ab#1 costs
 *   move_cards { source: hand, count: 3, optional: true,
 *                distinct: "card_name", group_names: "BiBi", destination: discard }
 * i.e. 「手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いてもよい」.
 * The Rust comment on the arm:
 *   // 「名前の異なる…のメンバーカード」 -- the COST compares printed NAMES, not
 *   // card numbers, so two prints of one character count once.
 *   // `PL!-pb2-018-P＋` otherwise paid 絢瀬絵里 -P and 絢瀬絵里 -R as two of its
 *   // three cards, which 名前の異なる forbids.
 *   // Deduping the CANDIDATE SET (rather than filtering after the selection)
 *   // also means the duplicate is never offered, so the player cannot select it.
 *
 * The hand below is three BiBi MEMBERS of which TWO are prints of 絢瀬絵里
 * (PL!-bp3-002-P and PL!-bp3-002-R -- different card NUMBERS, same printed NAME,
 * which is exactly the case the arm exists for). After the dedupe the candidate
 * set is 2, and handlers.rs:541-551 then AUTO-SKIPS the optional cost instead of
 * opening a prompt:
 *     if !is_any_number && matching_indices.len() < count {
 *         if is_optional {
 *             // If optional cost, we should auto-skip if the hand is completely
 *             // empty or doesn't have enough matching cards for name-restricted
 *             // costs.
 *             ... entry.cost_paid = true; entry.optional_cost_result = Some(false);
 *             return Ok(true);
 *         }
 *
 * So the observable is: NO prompt. Before the fix the set was 3 and a prompt
 * opened offering 絢瀬絵里 twice.
 *
 * MEASURED REACH (engine's own decoder, whole corpus): 2 of the 901 decoded cost
 * nodes carry `distinct`, and both are this exact shape. The checklist's older
 * "0 of 138 costs carry distinct" counted only TOP-LEVEL costs.
 */
#define DISTINCT_NO "PL!-pb2-018-P＋"
#define SAME_NAME_A "PL!-bp3-002-P"   /* 絢瀬絵里 */
#define SAME_NAME_B "PL!-bp3-002-R"   /* 絢瀬絵里 -- the SAME printed name */
#define OTHER_NAME  "PL!-bp3-009-P"   /* 矢澤にこ  */

static void FW_C3_hand_cost_dedupes_by_normalized_name(void)
{
    int host = ident(&tg, DISTINCT_NO);
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    /* CARD IDENTITY: -P and -R are DIFFERENT cards that share a template, so
     * test_new_id (a distinct copy) is required, never test_id. */
    int a = test_new_id(&tg, SAME_NAME_A);
    int b = test_new_id(&tg, SAME_NAME_B);
    int o = test_new_id(&tg, OTHER_NAME);
    CHECK(a >= 0 && b >= 0 && o >= 0, "FW-C3 precondition: all three hand cards resolve");
    if (a < 0 || b < 0 || o < 0) return;
    CHECK(a != b,
          "FW-C3 precondition: %s and %s are DISTINCT card ids (test_new_id, not test_id)",
          SAME_NAME_A, SAME_NAME_B);

    /* PRECONDITION: the two really do print the same normalized name, and all
     * three are BiBi MEMBERS. Without this the CHECKs below could pass for an
     * unrelated reason. */
    char na[128], nb[128];
    {
        Card ca, cb;
        if (rb_decode_card_by_index((uint32_t)a, &ca) &&
            rb_decode_card_by_index((uint32_t)b, &cb)) {
            rb_card_normalize_name(ca.name, na, sizeof na);
            rb_card_normalize_name(cb.name, nb, sizeof nb);
            rb_free_card(&ca);
            rb_free_card(&cb);
        } else {
            na[0] = nb[0] = '\0';
        }
    }
    CHECK(na[0] && !strcmp(na, nb),
          "FW-C3 precondition: %s and %s share a normalized NAME ('%s' vs '%s')",
          SAME_NAME_A, SAME_NAME_B, na, nb);
    CHECK(rb_card_is_member(a) && rb_card_is_member(b) && rb_card_is_member(o),
          "FW-C3 precondition: all three are member cards");
    CHECK(rb_card_matches_group_str(a, "BiBi"),
          "FW-C3 precondition: %s is in group BiBi", SAME_NAME_A);

    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)host, 1, &ab)) {
        fprintf(stderr, "SETUP BUG: ab#1 of %s does not decode\n", DISTINCT_NO);
        setup_bugs++; assertions++;
        return;
    }
    CHECK(ab.cost && ab.cost->action && !strcmp(ab.cost->action, "move_cards"),
          "FW-C3 precondition: ab#1 costs a move_cards");
    if (!ab.cost) { rb_free_ability(&ab); return; }
    CHECK(rb_effect_count_or(ab.cost, 1) == 3,
          "FW-C3 precondition: the cost asks for 3 cards, got %d",
          rb_effect_count_or(ab.cost, 1));
    CHECK(ab.cost->is_optional,
          "FW-C3 precondition: the cost is OPTIONAL, so a short candidate set "
          "auto-skips rather than refusing (handlers.rs:541-551)");

    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, o);
    test_recalc(&tg);
    CHECK_EQ(tg.state.p[0].hand.n, 3, "FW-C3 precondition: the hand really holds 3 cards");

    rb_pay_cost(&tg.state, 0, ab.cost);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch == NULL,
          "FW-C3 the 「手札の名前の異なる『BiBi』のメンバーカードを3枚」 cost must "
          "AUTO-SKIP: handlers.rs:476-480 dedupes the CANDIDATE SET by normalized "
          "name, leaving 2 of the 3 asked-for cards, and handlers.rs:541-551 turns "
          "an optional short-of-count hand cost into `optional_cost_result = "
          "Some(false)` with no prompt");
    rb_free_ability(&ab);
}

/* The CONTROL for FW-C3: the SAME three cards, the SAME 3-card cost, but with
 * `distinct` absent -- then the candidate set is 3 and the prompt DOES open. This
 * is what proves the dedupe is gated on `distinct` and is not an accidental
 * removal of candidates. */
static void FW_C3_control_without_distinct_the_prompt_opens(void)
{
    int host = ident(&tg, DISTINCT_NO);
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    int a = test_new_id(&tg, SAME_NAME_A);
    int b = test_new_id(&tg, SAME_NAME_B);
    int o = test_new_id(&tg, OTHER_NAME);
    if (a < 0 || b < 0 || o < 0) return;

    Ability ab;
    memset(&ab, 0, sizeof(ab));
    if (!rb_decode_card_ability((uint32_t)host, 1, &ab)) {
        fprintf(stderr, "SETUP BUG: ab#1 of %s does not decode\n", DISTINCT_NO);
        setup_bugs++; assertions++;
        return;
    }
    if (!ab.cost) { rb_free_ability(&ab); return; }

    /* Take the `distinct` extra off a CLONE, so the decoded ability is untouched
     * and the control differs from FW_C3 in exactly one key. */
    AbilityEffect stripped = *(AbilityEffect *)ab.cost;
    static char keys[RB_MAX_EXTRA][32];
    static char vals[RB_MAX_EXTRA][64];
    int m = 0;
    for (int i = 0; i < ab.cost->n_extra; i++) {
        if (ab.cost->extra_k[i] && !strcmp(ab.cost->extra_k[i], "distinct")) continue;
        if (m >= RB_MAX_EXTRA) break;
        snprintf(keys[m], 32, "%s", ab.cost->extra_k[i]);
        snprintf(vals[m], 64, "%s", ab.cost->extra_v[i] ? ab.cost->extra_v[i] : "");
        m++;
    }
    stripped.n_extra = 0;
    for (int i = 0; i < m; i++) {
        stripped.extra_k[stripped.n_extra] = keys[i];
        stripped.extra_v[stripped.n_extra] = vals[i];
        stripped.n_extra++;
    }
    CHECK(stripped.n_extra + 1 == ab.cost->n_extra,
          "FW-C3 control precondition: exactly the `distinct` extra was removed "
          "(%d -> %d keys)", ab.cost->n_extra, stripped.n_extra);

    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    test_add_to_hand(&tg, o);
    test_recalc(&tg);

    rb_pay_cost(&tg.state, 0, &stripped);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL,
          "FW-C3 control: with `distinct` removed the same three hand cards all "
          "match, the set reaches 3, and the prompt opens (handlers.rs:541 is not "
          "taken and :575-626 builds the SelectCard)");
    if (ch) CHECK_EQ(ch->count, 3,
                     "FW-C3 control: the prompt asks for all 3 candidates");
    rb_free_ability(&ab);
}

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = failures;
        current_test = name; fn(); current_test = "(none)";
        printf("%-8s %s\n", (failures - f0) ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = failures, a0 = assertions;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s)\n", name,
               assertions - a0, failures - f0);
        fflush(stdout); fflush(stderr);
        _exit(failures > f0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) n_tests_crashed++; else n_tests_ok++;
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FATAL: could not load the card DB\n");
        return 2;
    }
    test_card_copy_pool_reset();
    test_game_new(&tg);

    printf("-- FW-C1 offer gate is validate_mandatory_cost(Some(card_id)) --\n");
    run("fw_c1_self_cost_activation_is_offered", FW_C1_self_cost_activation_is_offered);
    run("fw_c1_unpayable_mandatory_cost_is_still_refused",
        FW_C1_unpayable_mandatory_cost_is_still_refused);

    printf("-- FW-CT the ten cost.c card_type reads --\n");
    run("fw_ct_reveal_cost_card_type_comes_from_the_struct_field",
        FW_CT_reveal_cost_card_type_comes_from_the_struct_field);
    run("fw_ct_absent_card_type_stays_unfiltered",
        FW_CT_absent_card_type_stays_unfiltered);

    printf("-- FW-C3 hand_cost_matching_indices distinct dedupe --\n");
    run("fw_c3_hand_cost_dedupes_by_normalized_name",
        FW_C3_hand_cost_dedupes_by_normalized_name);
    run("fw_c3_control_without_distinct_the_prompt_opens",
        FW_C3_control_without_distinct_the_prompt_opens);

    printf("\n==== parity_fw_cost_cardtype: %d tests ok, %d crashed (%d setup bugs) ====\n",
           n_tests_ok, n_tests_crashed, setup_bugs);
    return failures ? 1 : 0;
}
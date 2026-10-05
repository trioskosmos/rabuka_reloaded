/* test_parity_fw_effects.c
 *
 * FINAL-WAVE parity suite for the cluster F / T2 work in
 * src/ability/effects/{misc,rules,custom,executor}.c. One case per fix, each
 * transcribing a specific Rust statement and naming it. Nothing here is a new
 * expectation.
 *
 *   FW-F1   the `custom` action is dispatched to custom.c's single owner, and
 *           the routed move_cards gets a POOLED DEEP CLONE, not a stack struct
 *   FW-F2   the two gain_heart sites write through add_heart_modifier_with_trace
 *   FW-F3   grant_blade / grant_heart_distribution write through the traced twins
 *           with the activating card as SOURCE and effect.text as LABEL (E13)
 *   FW-T2C  choose_target_player's prompt carries allow_skip:false
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

#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: "); \
        fprintf(stderr, __VA_ARGS__); \
        fprintf(stderr, "  [%s]\n", current_test); \
        failures++; \
        FLAT_VERDICT(); \
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
        FLAT_VERDICT(); \
    } \
} while (0)

#define AREA_CENTER 1

static TestGame tg;

/* rules.c's latch writer is a non-static definition with no declaration in
 * include/rabuka.h, which is not this test's to edit. Declared here for the same
 * reason src/ability/effects/state.c:26 declares it, and for the same reason. */
void rb_rules_note_changed_state_members(GameState *g, const int *cids, int n);

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

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
    t->state.activating_card = -1;
    t->state.queue.has_pending = 0;
    t->state.queue.resume_eff = NULL;
    t->state.queue.n_entries = 0;
    t->state.queue.cur = -1;
}

/* A minimal hand-built gain_resource effect. The corpus is not used here
 * because the point of FW-F3 is the WRITE, and a decoded card would drag a cost,
 * a trigger and a target filter into the assertion for no extra evidence.
 * `text` is the LABEL Rust passes as `&effect.text`. */
static void mk_gain_resource(AbilityEffect *e, const char *resource, int amount,
                             const char *text)
{
    memset(e, 0, sizeof(*e));
    e->action = (char *)"gain_resource";
    e->text = (char *)text;
    e->target = (char *)"self";
    e->count = amount;
    e->extra_k[0] = (char *)"resource";
    e->extra_v[0] = (char *)resource;
    e->n_extra = 1;
}

/* ===========================================================================
 * FW-F3 (E13) 窶・blade and heart applications reach the trace ring.
 *
 * Rust, resource.rs:223-232 and :194-221, both of which are the ONLY writers of
 * the blade / heart effect_types in `gs.ability_applications`:
 *     fn grant_blade(gs, effect, card_id, amount) {
 *         gs.mods.add_blade_modifier_with_trace(
 *             card_id, amount, &mut gs.ability_applications,
 *             gs.activating_card.unwrap_or(-1), &effect.text);
 *     }
 *     fn grant_heart_distribution(gs, effect_text, card_id, distribution,
 *                                 is_negative) {
 *         for &(color, dist_count) in distribution {
 *             let dist_amount = if is_negative { -i16::from(dist_count) }
 *                               else            {  i16::from(dist_count) };
 *             gs.mods.add_heart_modifier_with_trace(
 *                 card_id, color, dist_amount, &mut gs.ability_applications,
 *                 gs.activating_card.unwrap_or(-1), effect_text);
 *         }
 *     }
 *
 * The C's grant_blade / grant_heart_distribution called the UNTRACED
 * rb_mods_add_blade / rb_mods_add_heart, so src/core/modifiers.c recorded
 * "0 production callers" for rb_mods_add_blade_with_trace and
 * rb_mods_add_heart_with_trace and left a CROSS-FILE REQUEST naming this file:
 *     笵・BLOCKED-ON: src/ability/effects/misc.c 窶・widen `grant_blade`
 *        (misc.c:218) and `grant_heart_distribution` (misc.c:224) to take the
 *        ability text, then call the two *_with_trace twins.
 *
 * The SOURCE of the trace is `gs.activating_card`, NOT the target. That
 * distinction is the part a "just add the trace" patch gets wrong, so it is
 * asserted on its own below with two different card ids.
 */
static void FW_F3_blade_grant_reaches_the_trace_ring(void)
{
    int host = ident(&tg, "PL!N-bp3-004-P");
    int other = ident(&tg, "PL!N-bp3-008-P");
    if (host < 0 || other < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, host);
    /* `activating_card` is the ability's own card and is what Rust attributes
     * the application to; it is deliberately NOT the card being credited. */
    tg.state.activating_card = host;
    test_recalc(&tg);

    RbMods before = tg.state.mods;
    int n_before = rb_mods_trace_len(&tg.state.mods);
    (void)before;

    AbilityEffect e;
    mk_gain_resource(&e, "blade", 3,
        /* The LABEL is a real card's printed ability line, because that is what Rust
         * hands the trace writer: resource.rs:229-230 passes
         * `gs.activating_card.unwrap_or(-1)` and `&effect.text`, and on a real
         * ability `effect.text` IS that card's ability line. The literal here
         * used to be a 9-byte Japanese word meaning "broad", which is 0
         * occurrences across cards/cards.json, engine/src and engine/tests --
         * fabricated, so nothing downstream could fail on a wrong label.
         *
         * PL!S-bp6-011-N is used because its ability line is 123 bytes and so
         * FITS the C's `ability_text[RB_MODS_TRACE_TEXT]` (128, rabuka.h:205).
         * That is deliberate: an exact compare would otherwise also be asserting
         * that the C does not truncate, and it DOES --
         * rb_mods_trace_push (modifiers.c:337-341) copies at most 127 bytes,
         * while Rust's field is a String (core/modifiers.rs AbilityApplication)
         * and keeps all of it. That capacity divergence is real, is NOT what
         * this test is for, and is reported separately. Choosing a fitting line
         * keeps this assertion about the LABEL RULE (resource.rs:230).
         * Escapes, because the Write path here can emit CP932. */
        "{{toujyou.png|\xE7\x99\xBB\xE5\xA0\xB4}}\xE6\x8E\xA7\xE3\x81\x88\xE5\xAE\xA4\xE3\x81\x8B\xE3\x82\x89"
        "\xE7\x99\xBB\xE5\xA0\xB4\xE3\x81\x97\xE3\x81\xA6\xE3\x81\x84\xE3\x82\x8B\xE5\xA0\xB4\xE5\x90\x88"
        "\xE3\x80\x81\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89\xE3\x82\x922\xE6\x9E\x9A\xE5\xBC\x95\xE3\x81\x8D"
        "\xE6\x89\x8B\xE6\x9C\xAD\xE3\x82\x921\xE6\x9E\x9A\xE6\x8E\xA7\xE3\x81\x88\xE5\xAE\xA4\xE3\x81\xAB"
        "\xE7\xBD\xAE\xE3\x81\x8F\xE3\x80\x82"); /* 繝悶Ξ繝ｼ繝峨・omic text */

    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &e, NULL);

    int n_after = rb_mods_trace_len(&tg.state.mods);
    CHECK(n_after > n_before,
          "FW-F3 a blade grant must PUSH an AbilityApplication: rb_mods_add_blade_"
          "with_trace is called (trace %d -> %d). Before this wiring the trace "
          "ring received score / cost / need-heart applications and NO blade one.",
          n_before, n_after);
    if (n_after > n_before) {
        const RbAbilityTraceEntry *t = &tg.state.mods.trace[n_after - 1];
        CHECK_EQ(t->effect_type, RB_EFFECT_BLADE_BONUS,
                 "FW-F3 the pushed entry's effect_type is RB_EFFECT_BLADE_BONUS");
        CHECK_EQ(t->source_card_id, host,
                 "FW-F3 the trace SOURCE is gs.activating_card (resource.rs:229), "
                 "NOT the credited card");
        CHECK_EQ(t->target_card_id, host,
                 "FW-F3 the credited card is the trace target");
        CHECK_EQ(t->amount, 3, "FW-F3 the trace amount is the granted blade count");
        CHECK(strcmp(t->ability_text, e.text) == 0,
              "FW-F3 the trace LABEL is effect.text EXACTLY (resource.rs:230 "
              "`&effect.text`), not merely non-empty -- a truncated or "
              "mis-copied label now fails this");

        /* Non-vacuity: the label is a REAL card's ability line and is longer
         * than a short ASCII token, so a silently blank or stubbed label cannot
         * satisfy an exact compare. */
        CHECK(strlen(e.text) >= 100,
              "FW-F3 the label fixture is a full 123-byte corpus ability line, "
              "so the exact compare above is not satisfiable by a stub");
    }

    /* The attribution rule on its own: credit a DIFFERENT card than the
     * activating one and the source must still be the activating card. */
    if (other >= 0 && other != host) {
        tg.state.mods.n_trace = 0;
        test_add_to_stage(&tg, AREA_CENTER, other);
        test_recalc(&tg);
        rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &e, NULL);
        int n2 = rb_mods_trace_len(&tg.state.mods);
        CHECK(n2 > 0, "FW-F3 the second blade grant also traces");
        if (n2 > 0) {
            const RbAbilityTraceEntry *t = &tg.state.mods.trace[n2 - 1];
            CHECK_EQ(t->source_card_id, host,
                     "FW-F3 with two members on stage the source is STILL "
                     "gs.activating_card -- Rust passes the activating card, not "
                     "the target, precisely so an attribution can be traced back "
                     "to the ability that caused it");
            CHECK_EQ(t->target_card_id, other,
                     "FW-F3 the credited member is the target");
        }
    }
}

static void FW_F3_heart_grant_reaches_the_trace_ring(void)
{
    int host = ident(&tg, "PL!N-bp3-004-P");
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, host);
    tg.state.activating_card = host;
    test_recalc(&tg);
    tg.state.mods.n_trace = 0;

    /* `heart_color` names one colour, and `resource.rs`'s distribution is
     * [(HeartColor, u8)] pairs, so this is the same shape with one entry.
     * The spelling is util.c:3149 rb_parse_heart_color's wire form
     * ("h01" = RED), not an index. `self_target` pins the self-target branch so
     * the case does not depend on how an unfiltered stage target set resolves. */
    AbilityEffect e;
    mk_gain_resource(&e, "heart", 2, "heart text");
    e.self_target_field[0] = 't'; e.self_target_field[1] = 'r';
    e.self_target_field[2] = 'u'; e.self_target_field[3] = 'e';
    e.self_target_field[4] = '\0';
    e.extra_k[1] = (char *)"heart_color";
    e.extra_v[1] = (char *)"h01";
    e.n_extra = 2;

/* The SELF-TARGET heart branch resolves its card through misc.c's
     * `s_activating_card`, which is set by rb_execute_misc_effect_ex and NOT by
     * the plain entry point -- so this case drives the `_ex` form, exactly as the
     * production path does. (The blade cases above resolve through the stage
     * candidate list instead, which is why they do not need it.) */
    rb_execute_misc_effect_ex(&tg.state, 0, &tg.state.p[0], &e, host, NULL);
    int n = rb_mods_trace_len(&tg.state.mods);
    CHECK(n > 0,
          "FW-F3 a heart grant must PUSH an AbilityApplication "
          "(rb_mods_add_heart_with_trace). Before this wiring "
          "rb_mods_add_heart_with_trace had 0 production callers. "
          "diag: red=%d all=%d n_sel=%d",
          rb_mods_get_heart(&tg.state.mods, host, RB_HEART_RED),
          rb_mods_get_heart(&tg.state.mods, host, RB_HEART_ALL),
          tg.state.n_selected_cards);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, host, RB_HEART_RED), 2,
             "FW-F3 CONTROL: the heart modifier itself is unchanged (+2 RED)");
    if (n > 0) {
        const RbAbilityTraceEntry *t = &tg.state.mods.trace[n - 1];
        CHECK_EQ(t->effect_type, RB_EFFECT_HEART_BONUS,
                 "FW-F3 the pushed entry's effect_type is RB_EFFECT_HEART_BONUS");
        CHECK_EQ(t->source_card_id, host,
                 "FW-F3 the heart trace source is gs.activating_card "
                 "(resource.rs:217), not the target");
        CHECK(t->ability_text[0] != '\0',
              "FW-F3 the heart trace LABEL is effect_text (resource.rs:218)");
    }
}

/* The CONTROL for FW-F3: the modifier itself is unchanged. The trace is an
 * ADDITION; the blade count must be identical with and without it, and this
 * case pins the arithmetic so a future "fix" cannot quietly double it. */
static void FW_F3_control_blade_count_is_unchanged(void)
{
    int host = ident(&tg, "PL!N-bp3-004-P");
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, host);
    tg.state.activating_card = host;
    test_recalc(&tg);

    AbilityEffect e;
    mk_gain_resource(&e, "blade", 3, "blade text");
    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &e, NULL);
    int after_one = rb_mods_get_blade(&tg.state.mods, host);

    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &e, NULL);
    int after_two = rb_mods_get_blade(&tg.state.mods, host);

    CHECK_EQ(after_one, 3,
             "FW-F3 CONTROL: one grant of 3 gives exactly +3 "
             "(rb_mods_add_blade, unchanged by the traced wrapper)");
    CHECK_EQ(after_two, 6,
             "FW-F3 CONTROL: two grants accumulate to +6, so the traced wrapper "
             "did not double-count");
}

/* ===========================================================================
 * FW-F1 窶・the `custom` action has ONE owner, and it hands the mover a POOLED
 * DEEP CLONE.
 *
 * Rust, dispatch.rs:240 in execute_custom:
 *     let mut routed = effect.clone();
 * and dispatch.rs:175 routes there directly:
 *     ActionType::Custom => resolver.execute_custom(gs, effect, effect.action.to_str())
 *
 * `routed` is a Rust LOCAL that outlives the nested execute_move_cards call, and
 * the borrow checker then guarantees anything the mover parks has been cloned by
 * the mover. The C has no such guarantee: rb_effect_move_cards parks the pointer
 * it is handed in `queue.resume_eff` (move.c:687) and choice.c reads it back
 * after the frame is gone. So the surviving copy (custom.c) deep-clones into a
 * keep-alive pool that is PINNED for the call (custom.c:293 `custom_keep_push`,
 * :299 `custom_keep_settle`), while the copy that used to run
 * (misc.c's h_custom) handed the mover the address of a STACK struct.
 *
 * The assertion below reads the parked pointer back. On the fixed tree it is the
 * pooled clone and every field is intact; on the pre-fix tree it is a dead stack
 * frame. The `custom` action literal appears on 0 of the 2116 card/ability pairs,
 * so this is built by hand rather than decoded -- and the count is asserted so
 * the case cannot pass just because the mover happened to be handed nothing.
 */
static void FW_F1_custom_routes_through_the_pooled_clone(void)
{
    int a = ident(&tg, "PL!N-bp3-004-P");
    int b = ident(&tg, "PL!N-bp3-008-P");
    if (a < 0 || b < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    /* Two cards in the HAND, so the routed `move_cards { source: hand,
     * count: 2 }` has something to move. The source is named explicitly rather
     * than defaulted: custom.c's default is "looked_at", whose pool is a
     * file-static in look.c that a test cannot stage. */
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"custom";
    e.text = (char *)"custom text";
    e.source = (char *)"hand";
    e.destination = (char *)"deck_top";
    e.count = 2;
    e.extra_k[0] = (char *)"placement_order";
    e.extra_v[0] = (char *)"any_order";
    e.n_extra = 1;

    rb_executor_execute(&tg.state, 0, &e, -1);

    /* Whichever path the answer took, `count` must have survived: custom.c
     * clones the WHOLE effect, so the routed move_cards still asks for 2. */
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    if (ch) {
        /* The mover parked the routed effect. It must not be a pointer into a
         * frame that has already returned. */
        const AbilityEffect *parked = tg.state.queue.resume_eff;
        CHECK(parked != NULL,
              "FW-F1 the routed move_cards parked a resume effect "
              "(a prompt is open, so rb_effect_move_cards saved context)");
        if (parked) {
            CHECK(parked->action && !strcmp(parked->action, "move_cards"),
                  "FW-F1 the PARKED effect is the routed move_cards -- read back "
                  "after the dispatch frame returned, which is the use-after-free "
                  "F1 was opened for. Got action='%s'.",
                  parked->action ? parked->action : "(null)");
            CHECK_EQ(parked->count, 2,
                     "FW-F1 the parked clone kept count:2 "
                     "(dispatch.rs:240 `let mut routed = effect.clone()`)");
            CHECK(parked->source && !strcmp(parked->source, "hand"),
                  "FW-F1 the parked clone kept source:'hand' "
                  "(custom.c rewrites only action/source/destination on the CLONE). "
                  "Got '%s'.",
                  parked->source ? parked->source : "(null)");
        }
    } else {
        /* No prompt: the move completed. Then the observable is the board. */
        int moved = tg.state.p[0].deck.n;
        CHECK(moved == 2,
              "FW-F1 with no prompt the routed move_cards put both hand cards on "
              "the deck (count survived the clone). deck.n=%d, want 2.", moved);
        CHECK_EQ(tg.state.p[0].hand.n, 0,
                 "FW-F1 the hand was actually drained");
    }
}

/* ===========================================================================
 * FW-T2C 窶・choose_target_player prompts with allow_skip:false.
 *
 * Rust selection.rs:211-234:
 *     self.awaiting.choice = Some(Choice::SelectTarget {
 *         target: "self_or_opponent".to_string(),
 *         description: "Choose self or opponent".to_string(),
 *         description_en: Some("Choose self or opponent".to_string()),
 *         description_ja: Some("閾ｪ蛻・∪縺溘・逶ｸ謇九ｒ驕ｸ謚・.to_string()),
 *         allow_skip: false,
 *         options: Some(options.to_vec()),
 *     });
 * `allow_skip: false` is a LITERAL. The C passed `e->is_optional ? 1 : 0`, so
 * the 2 corpus choose_target_player effects that carry `optional` offered a skip
 * the prompt never has, and answering it left the continuation's chosen seat
 * unset.
 */
static void FW_T2C_choose_target_player_is_not_skippable(void)
{
    int host = ident(&tg, "PL!N-bp3-010-P");
    if (host < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, host);
    test_recalc(&tg);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"choose_target_player";
    e.is_optional = 1;            /* the spelling the C used to honour */
    e.extra_k[0] = (char *)"choice_options";
    e.extra_v[0] = (char *)"self,opponent";
    e.n_extra = 1;

    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &e, NULL);

    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(ch != NULL, "FW-T2C the choose_target_player prompt opens");
    if (!ch) return;
    CHECK_EQ(ch->kind, RB_CHOICE_SELECT_TARGET,
             "FW-T2C the prompt kind is SelectTarget");
    CHECK_EQ(ch->allow_skip, 0,
             "FW-T2C allow_skip is FALSE (selection.rs:226 `allow_skip: false` is a "
             "literal, not effect.optional). The C passed e->is_optional, so an "
             "optional-flagged effect offered a skip Rust's prompt does not have.");
}

/* ===========================================================================
 * FW-T2-10 窶・the restriction handler that RUNS has the changed_state_members arm.
 *
 * rules.c's rb_rules_execute_restriction carried the arm and had ZERO callers;
 * the live copy (misc.c's h_restriction) did not. It is now in both, and the
 * live one CONSUMES the latch (read then clear), matching Rust's
 * assignment-then-consume pair.
 *
 * tests/test_parity_state_family.c's test_delayed_restriction_keys_on_the_
 * changed_members pins the end-to-end behaviour and was RED before this change
 * (3 FAIL lines -> 1). This case pins the CONSUME-ONCE half specifically, which
 * that suite does not: a second restriction in the same ability must not re-apply
 * to the same cards, because the arm clears the latch as it reads it.
 */
static void FW_T2_10_restriction_consumes_the_changed_state_latch(void)
{
    int host = ident(&tg, "PL!N-bp3-004-P");
    int a = ident(&tg, "PL!N-bp3-008-P");
    int b = ident(&tg, "PL!N-bp4-002-P");
    if (host < 0 || a < 0 || b < 0) return;

    bare_board(&tg);
    tg.state.active = 0;
    test_add_to_stage(&tg, AREA_CENTER, host);
    test_add_to_stage(&tg, AREA_CENTER, a);
    test_add_to_stage(&tg, AREA_CENTER, b);
    tg.state.activating_card = host;
    test_recalc(&tg);

    /* The latch, written the way state.c:596 writes it (the twin of Rust's
     * `self.selection.changed_state_members = ...`, an ASSIGNMENT). */
    int members[2] = { a, b };
    rb_rules_note_changed_state_members(&tg.state, members, 2);

    AbilityEffect r;
    memset(&r, 0, sizeof(r));
    r.action = (char *)"restriction";
    r.target = (char *)"opponent";
    r.extra_k[0] = (char *)"restriction_type";
    r.extra_v[0] = (char *)"cannot_activate_by_effect";
    r.extra_k[1] = (char *)"delayed";
    r.extra_v[1] = (char *)"true";
    r.n_extra = 2;

    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &r, NULL);

    CHECK_EQ(tg.state.n_cannot_active_cards, 2,
             "FW-T2-10 the delayed restriction keys on the two CHANGED members, "
             "not on the activating card and not on the whole stage "
             "(rules.c's Q280 comment: the victims are the cards THIS ability "
             "just moved)");
    if (tg.state.n_cannot_active_cards == 2) {
        CHECK(tg.state.cannot_active_cards[0] == a &&
              tg.state.cannot_active_cards[1] == b,
              "FW-T2-10 the keyed members are the ones the latch held, in order");
        CHECK(tg.state.cannot_active_cards[0] != host,
              "FW-T2-10 the ACTIVATING card is not keyed when the latch is "
              "non-empty -- the arm takes precedence over the recently-moved "
              "fallback, which is the whole point of it");
    }

    /* CONSUME-ONCE: the arm clears the latch as it reads it, so a second
     * restriction in the same ability must NOT re-apply to the same cards. */
    int n_after_first = tg.state.n_cannot_active_cards;
    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &r, NULL);
    CHECK_EQ(tg.state.n_cannot_active_cards, n_after_first,
             "FW-T2-10 the latch is CONSUMED: a second delayed restriction with "
             "an empty latch does not re-add the same members "
             "(rules.c's arm ends with `st->n_changed_state_members = 0`)");
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

    printf("-- FW-F3 the traced modifier writers (E13) --\n");
    run("fw_f3_blade_grant_reaches_the_trace_ring",
        FW_F3_blade_grant_reaches_the_trace_ring);
    run("fw_f3_heart_grant_reaches_the_trace_ring",
        FW_F3_heart_grant_reaches_the_trace_ring);
    run("fw_f3_control_blade_count_is_unchanged",
        FW_F3_control_blade_count_is_unchanged);

    printf("-- FW-F1 the custom action has one owner --\n");
    run("fw_f1_custom_routes_through_the_pooled_clone",
        FW_F1_custom_routes_through_the_pooled_clone);

    printf("-- FW-T2C choose_target_player allow_skip --\n");
    run("fw_t2c_choose_target_player_is_not_skippable",
        FW_T2C_choose_target_player_is_not_skippable);

    printf("-- FW-T2-10 the live restriction arm --\n");
    run("fw_t2_10_restriction_consumes_the_changed_state_latch",
        FW_T2_10_restriction_consumes_the_changed_state_latch);

    printf("\n==== parity_fw_effects: %d tests ok, %d crashed (%d setup bugs) ====\n",
           n_tests_ok, n_tests_crashed, setup_bugs);
    return failures ? 1 : 0;
}

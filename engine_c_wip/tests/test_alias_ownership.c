#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

/* tests/test_alias_ownership.c -- the OWNERSHIP TOPOLOGY of the seven parked
 * continuation slots, asserted rather than assumed.
 *
 * WHY THIS SUITE EXISTS
 * ---------------------
 * The alias-class defect was fixed three times by DEFERRING a free (the
 * pca_resolve_entry keep-alive, rb_queue_resume_pending_actions' transfer, and
 * the choice.c:5131 arm). Each deferral was correct only as long as no later
 * producer freed its own tree on a prompt-open pass, so the architectural fix is
 * to make the slots OWN their value. That conversion was ATTEMPTED on
 * g_look_pending_select on 2026-10-05, MEASURED, and REVERTED, because the slot
 * is not an independent owner: on the multi-pick re-prompt path it and
 * g->queue.resume_eff hold THE SAME NODE.
 *
 * This suite is the artifact that attempt produced. It pins the property that
 * makes the conversion unsafe to do one slot at a time, so the next attempt
 * starts from the measured fact instead of from the assumption that each slot
 * owns its own value.
 *
 *   look_emit_pool_choice's body, in this order:
 *       795:  g->queue.resume_eff = (AbilityEffect *)sel;
 *       797:  look_park_select((AbilityEffect *)sel);
 *   and `sel` on that path IS the value look_park_select is replacing.
 *
 *   MEASURED, same-snapshot A/B (tools/alias_probe.sh --prechange vs the
 *   converted arm, one tree state, no rebuild between):
 *       pre-change alias       : 0 alias hits; FAIL 37 / 65 / 44
 *       converted  own+release : 32 alias hits, all queue.resume_eff with
 *                                slot_value == freed_root; FAIL 41 / 73 / 44
 *
 *   The clone did not create the defect; it destroyed the node `resume_eff` was
 *   aliasing, and cost 10 previously-passing assertions.
 *
 * ⛔ NO SANITIZER RAN. gcc 15.3.0 (MSYS2/PE) in this tree has no asan, no ubsan,
 * no valgrind and no gdb. The substitutes are tools/alias_probe.sh (link-time
 * wraps over all seven slots, corpus-wide, with a re-runnable negative control)
 * and this suite's pointer-identity assertions.
 */

/* look.c's two parked slots. They were `static`, so no linker could name them,
 * which is part of why this class went unmeasured for as long as it did. These
 * read-only accessors are the whole of the production surface this suite needs;
 * writing through the returned pointer would make it an alias, which is the
 * defect. */
const AbilityEffect *rb_look_slot_pending_select(void);
const AbilityEffect *rb_look_slot_reprompt_followup(void);

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static const char *extra_of(const AbilityEffect *e, const char *key)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

static void clear_player_cards(TestGame *tg, int pl)
{
    RbPlayer *player = &tg->state.p[pl];
    player->deck.n = 0;
    player->hand.n = 0;
    player->discard.n = 0;
    player->live.n = 0;
    player->success.n = 0;
    player->energy.n = 0;
}

/* ── 1. a fresh select offer: the parked slot IS the executed node ───────────
 * The simplest form of the topology. rb_effect_select parks the very node it
 * executed, and queue.resume_eff gets the same node, so there is exactly ONE
 * owner between them and it is the ability tree -- neither slot may free. */
static void test_fresh_offer_shares_one_node(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    for (int i = 0; i < 4; i++) {
        int cid = test_new_id(&tg, "PL!-sd1-010-SD");
        if (cid < 0) return;
        test_add_to_hand(&tg, cid);
    }

    AbilityEffect select = {0};
    select.action = "select";
    select.source = "hand";
    select.target = "self";
    select.destination = "hand";
    select.count = 2;
    select.is_optional = 1;

    rb_effect_select(&tg.state, 0, &select);
    CHECK(rb_has_pending_choice(&tg.state), "select opens its prompt");
    const AbilityEffect *parked = rb_look_slot_pending_select();
    CHECK(parked == &select, "the parked select is the executed node itself");
    CHECK(tg.state.queue.resume_eff == &select,
          "queue.resume_eff holds the SAME node: one owner, not two");
    CHECK(rb_look_slot_pending_select() == rb_look_slot_reprompt_followup() ||
          rb_look_slot_reprompt_followup() == NULL,
          "a plain select parks no follow-up");

    int indices[2] = {0, 1};
    rb_resume_with_choice_indices(&tg.state, indices, 2);
    CHECK(rb_look_slot_pending_select() == NULL,
          "the parked slot is cleared when the offer is consumed");
}

/* ── 2. THE RE-PARK: the slot and resume_eff converge on one node ───────────
 * This is the case that broke the conversion. A `max` select re-presents the
 * leftover pool, so look_emit_pool_choice runs a second time with `sel` taken
 * from the slot it is about to replace -- and it publishes `sel` into
 * queue.resume_eff one line before parking it. */
static void test_repark_shares_one_node_with_resume_eff(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    for (int i = 0; i < 4; i++) {
        int cid = test_new_id(&tg, "PL!-sd1-010-SD");
        if (cid < 0) return;
        test_add_to_deck(&tg, cid);
    }

    /* The re-prompt only arms when the offer came from a LOOKED-AT pool
       (g_look_pool_select_offered is set by look_emit_pool_choice and by
       rb_effect_look_and_select, and rb_look_resume_indices gates `sel` on it),
       so this is the look_and_select shape, not a plain hand select. */
    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    look.target = "self";
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 3;
    select.is_optional = 1;
    select.extra_k[0] = "max";          select.extra_v[0] = "true";
    select.extra_k[1] = "any_number";   select.extra_v[1] = "true";
    select.n_extra = 2;
    AbilityEffect followup = {0};
    followup.action = "modify_score";
    followup.count = 3;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    parent.followup_action = &followup;

    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "the looked-at max select opens");
    CHECK(rb_look_slot_pending_select() == &select,
          "first offer parks the select node as-is");

    int one[1] = {0};                       /* take 1 of 3; max wants more */
    rb_resume_with_choice_indices(&tg.state, one, 1);
    CHECK(rb_has_pending_choice(&tg.state),
          "a max select re-presents the leftover pool (the re-prompt path ran)");
    CHECK(rb_look_slot_pending_select() != NULL,
          "the re-prompt parks the select again");
    CHECK(rb_look_slot_pending_select() == tg.state.queue.resume_eff,
          "after the re-park the slot and resume_eff are ONE node -- this is the "
          "identity that makes an owning clone of the slot alone a use-after-free");

    rb_clear_pending_choice(&tg.state);
}

/* ── 3. look_and_select: the follow-up is shared the same way ───────────────
 * g_look_reprompt_followup and queue.resume_after_look are assigned the same
 * expression on the same two lines (look.c:1710/1711), so they carry the same
 * constraint as slot 2. Asserted so a future conversion has to confront both. */
static void test_followup_shares_one_node_with_resume_after_look(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_player_cards(&tg, 0);
    for (int i = 0; i < 4; i++) {
        int cid = test_new_id(&tg, "PL!-sd1-010-SD");
        if (cid < 0) return;
        test_add_to_deck(&tg, cid);
    }

    AbilityEffect look = {0};
    look.action = "look_at";
    look.source = "deck_top";
    look.count = 3;
    look.target = "self";
    AbilityEffect select = {0};
    select.action = "select_cards";
    select.destination = "hand";
    select.count = 2;
    select.extra_k[0] = "discard_remaining";
    select.extra_v[0] = "true";
    select.n_extra = 1;
    AbilityEffect followup = {0};
    followup.action = "modify_score";
    followup.count = 3;
    AbilityEffect parent = {0};
    parent.action = "look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    parent.followup_action = &followup;

    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "look-and-select opens its prompt");
    CHECK(rb_look_slot_pending_select() == &select,
          "the select action is parked as-is (an alias into the parent tree)");
    CHECK(tg.state.queue.resume_eff == &select,
          "resume_eff holds that same node");
    CHECK(rb_look_slot_reprompt_followup() == &followup,
          "the follow-up is parked as-is");
    CHECK(tg.state.queue.resume_after_look == &followup,
          "resume_after_look holds the SAME follow-up node -- an owning clone of "
          "either slot alone would again be a second owner of one tree");
    CHECK(extra_of(rb_look_slot_reprompt_followup(), "nothing_here") == NULL,
          "the parked follow-up is readable");

    rb_clear_pending_choice(&tg.state);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_fresh_offer_shares_one_node();
    test_repark_shares_one_node_with_resume_eff();
    test_followup_shares_one_node_with_resume_after_look();
    rb_unload();
    if (failures) return 1;
    printf("ALL ALIAS OWNERSHIP CHECKS PASSED\n");
    return 0;
}
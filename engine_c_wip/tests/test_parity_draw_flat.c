/* tests/test_parity_draw_flat.c
 *
 * Parity coverage for the Rust cluster
 *   engine/tests/test_modules/effects/draw/flat/   (23 files)
 * i.e. plain `draw_card` (a fixed number of cards out of a zone into a
 * destination), as opposed to draw/until_count (already covered by
 * tests/test_parity_draw_until_count.c) and draw/chains (draw + a second
 * verb).
 *
 * Engine code under test:
 *   engine_c_wip/src/ability/effects/draw.c
 *      rb_draw_cards_for_player   (draw.rs:16-81   zone take / deck refresh)
 *      rb_effect_draw_card        (draw.rs:355-611 execute_draw_wrapper +
 *                                              execute_draw: count resolution,
 *                                              per_unit, target routing,
 *                                              card_type filter, distinct dedupe)
 *
 * Rust tests mirrored here (all under
 * engine/tests/test_modules/effects/draw/flat/):
 *   b9_more_test.rs                                  NOT PORTED - that file is
 *                                                    not a draw cluster at all
 *                                                    (blade / heart / need-heart
 *                                                    / score modifiers).
 *   boosted_member_live_success_draw_test.rs         -> 1
 *   costly_aqours_pl_s_bp7_002_r_test.rs             -> 3
 *   debut_one_pl_hs_bp5_011_n_test.rs                -> 4
 *   distinct_draw_regression_test.rs                 -> 5
 *   dollchestra_live_zone_gated_draw_test.rs         -> 6
 *   dream_believers_test.rs                          NOT PORTED - score-modifier
 *                                                    gate, needs advance_to_phase
 *   higher_cost_member_live_success_draw_test.rs     -> 7 (the three looked_at
 *                                                    ordering tests at the end of
 *                                                    that file are a look/select
 *                                                    case, not a draw case)
 *   live_success_compare_revealed_draw_behaviour_..  NOT PORTED - asserts on
 *                                                    GameState::rule_log, which
 *                                                    the C engine does not keep
 *   live_success_compare_revealed_draw_test.rs        -> 8 (the two parse-shape
 *                                                    tests; the live-flow one
 *                                                    needs advance_to_phase)
 *   live_success_optional_three_energy_draw_test.rs  -> 9
 *   live_zone_count_gated_draw_test.rs               NOT PORTED - needs
 *                                                    advance_to_phase plus the
 *                                                    real live-zone refill window
 *   miyashita_ai_pb1_test.rs                         -> 10
 *   optional_energy_catchu_trio_draw_sumire_...rs    -> 11
 *   optional_energy_live_success_draw_test.rs        -> 12
 *   optional_wait_members_draw_per_cost_member_...rs -> 13
 *   per_own_stage_member_draw_then_discard_q146_..rs -> 14
 *   position_ability_test.rs                         -> 15
 *   stage_member_identity_and_cost_gated_draw_...rs  -> 16
 *   success_zone_gated_debut_draw_test.rs            -> 17
 *   success_zone_group_live_success_draw_test.rs     NOT PORTED - its own
 *                                                    documented KNOWN GAP is
 *                                                    measured across the
 *                                                    advance_to_phase live window
 *   surplus_heart01_live_success_draw_test.rs         NOT PORTED - needs
 *                                                    advance_to_phase
 *   turn1_energy_draw_discard_pl_sp_bp1_009_r_...rs  -> 18
 *   waitroom_count_gated_debut_draw_test.rs          -> 19
 *
 * All Japanese literals are hex escapes so the file stays pure ASCII (the
 * same convention tests/test_parity_stats_pipeline.c uses).
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* Trigger wire tokens (engine/src/ability/enums.rs), byte-exact UTF-8. */
#define TRIG_DEBUT        "\xE7\x99\xBB\xE5\xA0\xB4"                            /* toujyou  */
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"
#define TRIG_CONSTANT     "\xE5\xB8\xB8\xE6\x99\x82"                            /* jouji    */
#define TRIG_ACTIVATE     "\xE8\xB5\xB7\xE5\x8B\x95"                            /* kidou    */
/* seikou live kaado - the success-live-card-zone restriction phrase. */
#define TEXT_SUCCESS_LIVE_CARD \
    "\xE6\x88\x90\xE5\x8A\x9F\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89"

/* rb_trigger_auto_ability exists in src/core/game_state_abilities.c but has no
   prototype in include/rabuka.h; declare it exactly as defined there (the same
   trick tests/test_parity_stats_pipeline.c and test_parity_auto_abilities.c use). */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id);

/* „Ÿ„Ÿ shared helpers „Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ„Ÿ */

/* Rust `select_indices(&[])` - answer the pending prompt with "nothing". */
static void df_drain(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 64)
        test_resume_choice(tg, -1);
}

/* Rust `select_option(n)` - answer a pay/skip style prompt by option index. */
static void df_select_option(TestGame *tg, int n)
{
    if (test_has_pending_choice(tg)) test_resume_choice(tg, n);
}

/* Rust `try_select_indices(&[0])` in a drain loop (Kumi / Hanayo cases). */
static void df_drain_pick_first(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 32)
        test_resume_choice(tg, 0);
}

/* Mirror Rust fire_trigger / fire_trigger_nth (helpers/mod.rs:114): find the
   `nth` ability whose `triggers` string EQUALS `trig`, build the
   "<card_no>_<full_text>" key, queue exactly that ability, set
   activating_card, then run process_pending_auto_abilities. It deliberately
   does NOT drain, so a caller can still observe the prompt Rust observes. */
static int df_fire_trigger(TestGame *tg, int cid, const char *trig,
                           const char *trigger_type, int nth)
{
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    char card_no[128];
    snprintf(card_no, sizeof(card_no), "%s", rb_card_string(card.card_no_idx));
    char ability_id[1024];
    int found = 0, nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab && !found; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (ab.triggers && strcmp(ab.triggers, trig) == 0) {
            if (nth == 0) {
                snprintf(ability_id, sizeof(ability_id), "%s_%s", card_no,
                         ab.full_text ? ab.full_text : "");
                found = 1;
            } else {
                nth--;
            }
        }
        rb_free_ability(&ab);
    }
    rb_free_card(&card);
    if (!found) return 0;
    rb_trigger_auto_ability(&tg->state, ability_id, trigger_type, 0, card_no,
                            cid, NULL, 0, -1);
    tg->state.activating_card = cid;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static int df_fire_debut(TestGame *tg, int cid) {
    return df_fire_trigger(tg, cid, TRIG_DEBUT, TRIG_DEBUT, 0);
}
static int df_fire_live_success(TestGame *tg, int cid) {
    return df_fire_trigger(tg, cid, TRIG_LIVE_SUCCESS, TRIG_LIVE_SUCCESS, 0);
}
static int df_fire_live_start(TestGame *tg, int cid) {
    return df_fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START, 0);
}

static void df_fill_decks(TestGame *tg, int filler, int n)
{
    tg->state.p[0].deck.n = 0;
    tg->state.p[1].deck.n = 0;
    for (int i = 0; i < n; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

static int df_hand(TestGame *tg)  { return tg->state.p[0].hand.n; }
static int df_deck(TestGame *tg)  { return tg->state.p[0].deck.n; }
static int df_energy(TestGame *tg) { return rb_energy_active_count(&tg->state.p[0]); }

static void df_clear_stage(TestGame *tg)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        tg->state.p[0].stage[a] = RB_EMPTY_SLOT;
        tg->state.p[0].stage_wait[a] = 0;
    }
}

/* Rust `game.new_id(card_no)`: a DISTINCT instance of the same print, which is
   what makes the duplicate-count and distinct-name cases observable. The card
   pool is finite, so fall back to the shared template id rather than failing a
   fixture that has nothing to do with the behaviour under test. */
static int df_new_id(TestGame *tg, const char *card_no)
{
    int id = test_new_id(tg, card_no);
    if (id >= 0) return id;
    return test_id(tg, card_no);
}

/* Build a flat draw_card effect the way the bytecode decoder would. */
static void df_make_draw(AbilityEffect *e, int count, const char *target,
                         const char *source, const char *destination)
{
    memset(e, 0, sizeof(*e));
    e->action      = (char *)"draw_card";
    e->count       = count;
    e->target      = (char *)(target ? target : "self");
    e->source      = (char *)(source ? source : "deck");
    e->destination = (char *)(destination ? destination : "hand");
}

static void df_set_extra(AbilityEffect *e, const char *k, const char *v)
{
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

/* Count the UseAbility offers the action generator makes for `cid` - the C
   equivalent of Rust's game_setup::generate_possible_actions filter. */
static int df_count_use_offers(TestGame *tg, int cid)
{
    RbGeneratedActionList list = rb_generate_action_candidates(&tg->state);
    int n = 0;
    for (int i = 0; i < list.count; i++) {
        if (list.actions[i].action_type == RB_ACTION_ACTIVATE_ABILITY &&
            list.actions[i].has_parameters &&
            list.actions[i].parameters.card_id == cid)
            n++;
    }
    free(list.actions);
    return n;
}

/* Has the per-turn ("turn 1 kai") budget already been spent on this card? */
static int df_turn1_used(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_turn1_abilities_played; i++)
        if (tg->state.turn1_abilities_played[i] == cid) return 1;
    return 0;
}

/* ???????????????????????????????????????????????????????????????????????????
   1. debut_one_pl_hs_bp5_011_n_test.rs - the plainest flat draw:
      PL!HS-bp5-011-N debut "draw 1".
   ????????????????????????????????????????????????????????????????????????? */
static void test_debut_draws_one_pl_hs_bp5_011_n(void)
{
    TestGame game;
    test_game_new(&game);
    int rurino = test_id(&game, "PL!HS-bp5-011-N");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    CHECK(rurino >= 0 && filler >= 0, "PL!HS-bp5-011-N fixtures resolve");
    if (rurino < 0 || filler < 0) return;

    df_fill_decks(&game, filler, 40);
    test_add_to_stage(&game, 1 /* centre */, rurino);
    int hand_before = df_hand(&game);
    int deck_before = df_deck(&game);

    CHECK(df_fire_debut(&game, rurino), "PL!HS-bp5-011-N exposes a debut ability");
    df_drain(&game);

    CHECK_EQ(df_hand(&game), hand_before + 1, "debut -> draw 1 (hand +1)");
    CHECK_EQ(df_deck(&game), deck_before - 1, "the drawn card came off the deck");
    CHECK_EQ(game.state.p[0].stage[1], rurino, "the card stays on the centre stage");
}

/* ???????????????????????????????????????????????????????????????????????????
   2. waitroom_count_gated_debut_draw_test.rs - a COUNT gate in front of the
      draw (PL!HS-bp2-017-N: waitroom >= 10 -> draw 1). The boundary is the
      point: 10 draws, 9 does not.
   ????????????????????????????????????????????????????????????????????????? */
static void test_waitroom_count_gated_debut_draw(void)
{
    for (int pass = 0; pass < 2; pass++) {
        int waitroom_n = pass == 0 ? 10 : 9;
        TestGame game;
        test_game_new(&game);
        int me = test_id(&game, "PL!HS-bp2-017-N");
        CHECK(me >= 0, "PL!HS-bp2-017-N fixture resolves");
        if (me < 0) return;

        game.state.p[0].stage[0] = me;
        for (int i = 0; i < waitroom_n; i++)
            test_add_to_discard(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_deck(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        int hand_before = df_hand(&game);
        int discard_before = game.state.p[0].discard.n;

        CHECK(df_fire_debut(&game, me), "PL!HS-bp2-017-N exposes a debut ability");
        df_drain(&game);

        char msg[128];
        snprintf(msg, sizeof(msg), "waitroom %d cards: expected hand delta %d", waitroom_n,
                 pass == 0 ? 1 : 0);
        CHECK_EQ(df_hand(&game) - hand_before, pass == 0 ? 1 : 0, msg);
        snprintf(msg, sizeof(msg),
                 "waitroom %d cards: the waitroom is not consumed by the gate", waitroom_n);
        CHECK_EQ(game.state.p[0].discard.n, discard_before, msg);
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   3. costly_aqours_pl_s_bp7_002_r_test.rs - a GROUP + COST-THRESHOLD gate in
      front of a flat draw (PL!S-bp7-002-R: an Aqours member costing 9 or more
      on my stage -> draw 1).
   ????????????????????????????????????????????????????????????????????????? */
static void test_costly_aqours_gated_debut_draw(void)
{
    /* positive: an Aqours member at the cost-9 boundary */
    {
        TestGame game;
        test_game_new(&game);
        int riko = test_id(&game, "PL!S-bp7-002-R");
        int fill = test_id(&game, "PL!-sd1-010-SD");
        int aq9  = test_id(&game, "PL!S-pb1-007-R");   /* Aqours / AZALEA, cost 9 */
        CHECK(riko >= 0 && aq9 >= 0, "PL!S-bp7-002-R fixtures resolve");
        if (riko < 0 || aq9 < 0) return;
        df_fill_decks(&game, fill, 40);
        test_add_to_stage(&game, 1, riko);
        test_add_to_stage(&game, 0 /* left */, aq9);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, riko), "PL!S-bp7-002-R exposes a debut ability");
        df_drain(&game);
        CHECK_EQ(df_hand(&game) - before, 1,
                 "cost 9 (boundary >=) Aqours member present -> draw 1");
    }
    /* negative: a non-Aqours member, then an Aqours member under cost 9 */
    {
        TestGame game;
        test_game_new(&game);
        int riko  = test_id(&game, "PL!S-bp7-002-R");
        int fill  = test_id(&game, "PL!-sd1-010-SD");
        int wrong = test_id(&game, "PL!HS-bp6-002-R"); /* DOLLCHESTRA, cost 9 */
        int cheap = test_id(&game, "PL!S-PR-025-PR");  /* Aqours / CYaRon, cost 2 */
        CHECK(riko >= 0 && wrong >= 0 && cheap >= 0,
              "PL!S-bp7-002-R negative fixtures resolve");
        if (riko < 0 || wrong < 0 || cheap < 0) return;
        df_fill_decks(&game, fill, 40);
        test_add_to_stage(&game, 1, riko);
        test_add_to_stage(&game, 0, wrong);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, riko), "PL!S-bp7-002-R negative: debut ability found");
        df_drain(&game);
        CHECK_EQ(df_hand(&game), before, "cost 9 but NOT Aqours -> no draw");

        test_add_to_stage(&game, 0, cheap);
        before = df_hand(&game);
        CHECK(df_fire_debut(&game, riko), "PL!S-bp7-002-R second negative: debut ability found");
        df_drain(&game);
        CHECK_EQ(df_hand(&game), before, "Aqours but cost 2 < 9 -> no draw");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   4. dollchestra_live_zone_gated_draw_test.rs - a ZONE-CONTENT gate in front
      of a flat draw (PL!HS-pb1-021-N: a DOLLCHESTRA card in the live card
      zone -> draw 1).
   ????????????????????????????????????????????????????????????????????????? */
static void test_dollchestra_live_zone_gated_draw(void)
{
    for (int pass = 0; pass < 2; pass++) {
        int dollchestra = pass == 0;
        TestGame game;
        test_game_new(&game);
        int me = test_id(&game, "PL!HS-pb1-021-N");
        int live = test_id(&game, dollchestra ? "PL!HS-bp2-020-L" : "PL!-sd1-020-SD");
        CHECK(me >= 0 && live >= 0, "PL!HS-pb1-021-N live-zone fixtures resolve");
        if (me < 0 || live < 0) return;
        game.state.p[0].stage[0] = me;
        test_add_to_live(&game, live);
        test_add_to_deck(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        int before = df_hand(&game);

        CHECK(df_fire_live_success(&game, me), "PL!HS-pb1-021-N exposes a live-success ability");
        df_drain(&game);

        CHECK_EQ(df_hand(&game) - before, dollchestra ? 1 : 0,
                 dollchestra ? "DOLLCHESTRA in live zone -> draw 1"
                              : "no DOLLCHESTRA in live zone -> no draw");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   5. boosted_member_live_success_draw_test.rs - a STATE-COMPARISON gate over
      a live-zone card (PL!HS-PR-028-PR: a staged member whose CURRENT hearts
      exceed its printed hearts -> draw 1, Q172).
   ????????????????????????????????????????????????????????????????????????? */
static void test_boosted_member_live_success_draw(void)
{
    /* boosted staged member -> draw 1 */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int live = test_id(&game, "PL!HS-PR-028-PR");
        int member = df_new_id(&game, "PL!-sd1-010-SD");
        CHECK(live >= 0 && member >= 0, "PL!HS-PR-028-PR fixtures resolve");
        if (live < 0 || member < 0) return;
        df_fill_decks(&game, filler, 40);
        test_add_to_live(&game, live);
        game.state.p[0].stage[0] = member;
        rb_mods_add_heart(&game.state.mods, member, RB_HEART_PINK, 2);
        int deck_before = df_deck(&game);
        CHECK(df_fire_live_success(&game, live), "PL!HS-PR-028-PR exposes live-success");
        df_drain(&game);
        CHECK_EQ(deck_before - df_deck(&game), 1, "current > original -> draw 1");
    }
    /* unboosted staged member -> no draw */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int live = test_id(&game, "PL!HS-PR-028-PR");
        int member = df_new_id(&game, "PL!-sd1-010-SD");
        if (live < 0 || member < 0) return;
        df_fill_decks(&game, filler, 40);
        test_add_to_live(&game, live);
        game.state.p[0].stage[0] = member;
        int deck_before = df_deck(&game);
        CHECK(df_fire_live_success(&game, live), "PL!HS-PR-028-PR unboosted: trigger found");
        df_drain(&game);
        CHECK_EQ(df_deck(&game), deck_before, "current == original is not 'more than' -> no draw");
    }
    /* empty stage -> no draw */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int live = test_id(&game, "PL!HS-PR-028-PR");
        if (live < 0) return;
        df_fill_decks(&game, filler, 40);
        test_add_to_live(&game, live);
        df_clear_stage(&game);
        int deck_before = df_deck(&game);
        CHECK(df_fire_live_success(&game, live), "PL!HS-PR-028-PR empty stage: trigger found");
        df_drain(&game);
        CHECK_EQ(df_deck(&game), deck_before, "an empty stage cannot satisfy the gate -> no draw");
    }
    /* a boosted NON-member in the waitroom must not satisfy the gate while the
       stage holds an unboosted member */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int live = test_id(&game, "PL!HS-PR-028-PR");
        int wait_card = test_id(&game, "PL!-sd1-019-SD");
        int stage_member = test_id(&game, "PL!S-sd1-001-SD");
        CHECK(live >= 0 && wait_card >= 0 && stage_member >= 0,
              "PL!HS-PR-028-PR non-member fixtures resolve");
        if (live < 0 || wait_card < 0 || stage_member < 0) return;
        df_fill_decks(&game, filler, 40);
        test_add_to_live(&game, live);
        test_add_to_discard(&game, wait_card);
        rb_mods_add_heart(&game.state.mods, wait_card, RB_HEART_PINK, 2);
        game.state.p[0].stage[0] = stage_member;
        int deck_before = df_deck(&game);
        CHECK(df_fire_live_success(&game, live), "PL!HS-PR-028-PR non-member: trigger found");
        df_drain(&game);
        CHECK_EQ(df_deck(&game), deck_before, "boosted non-member must not satisfy the gate");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   6. higher_cost_member_live_success_draw_test.rs - a COST-THRESHOLD scan over
      staged members (PL!HS-pb1-013-R, cost 9: a member costing MORE than 9 on
      stage -> draw 1). Strictly greater: an equal-cost member must not count.
   ????????????????????????????????????????????????????????????????????????? */
static void test_higher_cost_member_live_success_draw(void)
{
    static const struct { const char *no; int draws; } CASES[] = {
        { "PL!HS-bp5-004-R", 1 },   /* cost 15, > 9 */
        { "PL!SP-PR-007-PR", 0 },   /* cost  2, < 9 */
        { "PL!-sd1-014-SD",  0 },   /* cost  9, = 9 */
    };
    for (unsigned i = 0; i < sizeof(CASES) / sizeof(CASES[0]); i++) {
        TestGame game;
        test_game_new(&game);
        int me = test_id(&game, "PL!HS-pb1-013-R");
        int other = test_id(&game, CASES[i].no);
        int drawn = df_new_id(&game, "PL!-sd1-010-SD");
        CHECK(me >= 0 && other >= 0 && drawn >= 0, "PL!HS-pb1-013-R fixtures resolve");
        if (me < 0 || other < 0 || drawn < 0) return;
        game.state.p[0].stage[0] = me;
        game.state.p[0].stage[1] = other;
        test_add_to_deck(&game, drawn);
        int hand_before = df_hand(&game);
        int deck_before = df_deck(&game);

        CHECK(df_fire_live_success(&game, me), "PL!HS-pb1-013-R exposes live-success");
        df_drain(&game);

        char msg[128];
        snprintf(msg, sizeof(msg), "%s on stage: expected %d card(s) drawn", CASES[i].no,
                 CASES[i].draws);
        CHECK_EQ(df_hand(&game) - hand_before, CASES[i].draws, msg);
        snprintf(msg, sizeof(msg), "%s on stage: expected deck delta %d", CASES[i].no,
                 CASES[i].draws);
        CHECK_EQ(deck_before - df_deck(&game), CASES[i].draws, msg);
        if (CASES[i].draws)
            CHECK(test_hand_has(&game, drawn), "the drawn card is the one stocked on the deck");
        else
            CHECK(test_zone_has_id(&game, 0, "deck", drawn),
                  "the stocked card is left in the deck when the gate fails");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   7. optional_energy_catchu_trio_draw_sumire_bp7_015_n_test.rs - a
      GROUP-COUNT gate plus an OPTIONAL ENERGY cost in front of a flat draw
      (PL!SP-bp7-015-N: 3 CatChu! members on stage, may pay E, draw 1).
   ????????????????????????????????????????????????????????????????????????? */
static void test_sumire_optional_energy_catchu_trio_draw(void)
{
    /* three CatChu! members, pay the optional energy */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int sumire = test_id(&game, "PL!SP-bp7-015-N");
        int c1 = test_id(&game, "PL!SP-PR-003-PR");
        int c2 = test_id(&game, "PL!SP-PR-006-PR");
        CHECK(sumire >= 0 && c1 >= 0 && c2 >= 0, "PL!SP-bp7-015-N fixtures resolve");
        if (sumire < 0 || c1 < 0 || c2 < 0) return;
        df_fill_decks(&game, filler, 40);
        game.state.p[0].stage[0] = sumire;
        game.state.p[0].stage[1] = c1;
        game.state.p[0].stage[2] = c2;
        int deck_before = df_deck(&game);
        test_give_energy(&game, 5);

        CHECK(df_fire_live_start(&game, sumire), "PL!SP-bp7-015-N exposes live-start");
        CHECK(test_has_pending_choice(&game), "optional energy cost prompted");
        CHECK(test_pending_choice_type(&game) != NULL,
              "energy cost prompt must carry a choice identity");
        df_select_option(&game, 1); /* pay */
        df_drain(&game);
        CHECK_EQ(deck_before - df_deck(&game), 1,
                 "3 CatChu! members staged + paid -> draw 1");
    }
    /* only two CatChu! members: no prompt, no draw */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int sumire = test_id(&game, "PL!SP-bp7-015-N");
        int c1 = test_id(&game, "PL!SP-PR-003-PR");
        int mu = test_id(&game, "PL!-sd1-010-SD");
        CHECK(sumire >= 0 && c1 >= 0 && mu >= 0, "PL!SP-bp7-015-N negative fixtures resolve");
        if (sumire < 0 || c1 < 0 || mu < 0) return;
        df_fill_decks(&game, filler, 40);
        game.state.p[0].stage[0] = sumire;
        game.state.p[0].stage[1] = c1;
        game.state.p[0].stage[2] = mu;
        int deck_before = df_deck(&game);
        CHECK(df_fire_live_start(&game, sumire), "PL!SP-bp7-015-N negative: live-start found");
        df_drain(&game);
        CHECK_EQ(df_deck(&game), deck_before, "only 2 CatChu! members -> no draw");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   8. stage_member_identity_and_cost_gated_draw_test.rs - several DIFFERENT
      flat draws, each gated on a different staged-member property:
      per-presence (a named member present -> 1 extra), cost threshold
      (cost 13 present -> draw 1), and a cross-player stage-TOTAL comparison
      (my stage total cheaper than the opponent's -> draw 1).
   ????????????????????????????????????????????????????????????????????????? */
static void test_stage_member_identity_and_cost_gated_draw(void)
{
    /* PL!SP-bp1-008-R (Wakashi): no Mei on stage -> draw 1 */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int wakashi = test_id(&game, "PL!SP-bp1-008-R");
        CHECK(wakashi >= 0, "PL!SP-bp1-008-R fixture resolves");
        if (wakashi < 0) return;
        game.state.p[0].stage[1] = wakashi;
        df_fill_decks(&game, filler, 40);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, wakashi), "PL!SP-bp1-008-R exposes debut");
        df_drain(&game);
        CHECK_EQ(df_hand(&game) - before, 1, "no Mei on stage -> single draw");
    }
    /* PL!SP-bp1-008-R: Mei on stage -> draw 2 */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int wakashi = test_id(&game, "PL!SP-bp1-008-R");
        int mei = test_id(&game, "PL!SP-pb1-007-R");
        CHECK(wakashi >= 0 && mei >= 0, "PL!SP-bp1-008-R + Mei fixtures resolve");
        if (wakashi < 0 || mei < 0) return;
        game.state.p[0].stage[1] = wakashi;
        game.state.p[0].stage[0] = mei;
        df_fill_decks(&game, filler, 40);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, wakashi), "PL!SP-bp1-008-R with Mei: debut found");
        df_drain(&game);
        CHECK_EQ(df_hand(&game) - before, 2, "Mei on stage -> additional draw");
    }
    /* PL!-bp3-009-R+ (Niko): cost-13 member present -> draw 1 */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int niko = test_id(&game, "PL!-bp3-009-R\xEF\xBC\x8B");
        int wakashi = test_id(&game, "PL!SP-bp1-008-R");   /* cost 13 */
        CHECK(niko >= 0 && wakashi >= 0, "PL!-bp3-009-R+ fixtures resolve");
        if (niko < 0 || wakashi < 0) return;
        game.state.p[0].stage[0] = wakashi;
        game.state.p[0].stage[1] = niko;
        df_fill_decks(&game, filler, 40);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, niko), "PL!-bp3-009-R+ exposes debut");
        df_drain(&game);
        CHECK_EQ(df_hand(&game) - before, 1, "cost-13 member present -> draw 1");
    }
    /* PL!-bp3-009-R+: only herself (cost 2) -> no draw */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int niko = test_id(&game, "PL!-bp3-009-R\xEF\xBC\x8B");
        CHECK(niko >= 0, "PL!-bp3-009-R+ negative fixture resolves");
        if (niko < 0) return;
        game.state.p[0].stage[1] = niko;
        df_fill_decks(&game, filler, 40);
        int before = df_hand(&game);
        CHECK(df_fire_debut(&game, niko), "PL!-bp3-009-R+ negative: debut found");
        df_drain(&game);
        CHECK_EQ(df_hand(&game), before, "only herself (cost 2) on stage -> no draw");
    }
    /* PL!-bp4-001-R (Honoka): own stage total cheaper than the opponent's -> 1 */
    {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int honoka = test_id(&game, "PL!-bp4-001-R");    /* cost 9 */
        int big = test_id(&game, "PL!S-bp5-009-R");       /* cost 15 */
        int small = test_id(&game, "PL!-pb1-021-PR");    /* cost 5 */
        CHECK(honoka >= 0 && big >= 0 && small >= 0, "PL!-bp4-001-R fixtures resolve");
        if (honoka < 0 || big < 0 || small < 0) return;
        df_fill_decks(&game, filler, 40);
        game.state.p[0].stage[1] = honoka;
        game.state.p[1].stage[1] = big;
        int before = df_hand(&game);
        CHECK(df_fire_live_start(&game, honoka), "PL!-bp4-001-R exposes live-start");
        df_drain(&game);
        CHECK_EQ(df_hand(&game) - before, 1, "stage total 9 < opponent 15 -> draw 1");

        game.state.p[1].stage[1] = small;
        before = df_hand(&game);
        CHECK(df_fire_live_start(&game, honoka), "PL!-bp4-001-R second case: live-start found");
        df_drain(&game);
        CHECK_EQ(df_hand(&game), before, "stage total 9 > opponent 5 -> no draw");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   9. live_success_optional_three_energy_draw_test.rs - the optional-E3 cost
      in front of a flat draw (PL!SP-pb1-004-R), measured on BOTH sides of
      the pay/skip prompt.
   ????????????????????????????????????????????????????????????????????????? */
static void test_live_success_optional_three_energy_draw(void)
{
    for (int pay = 1; pay >= 0; pay--) {
        TestGame game;
        test_game_new(&game);
        int sumire = test_id(&game, "PL!SP-pb1-004-R");
        int filler = test_id(&game, "PL!-sd1-010-SD");
        CHECK(sumire >= 0 && filler >= 0, "PL!SP-pb1-004-R fixtures resolve");
        if (sumire < 0 || filler < 0) return;
        df_fill_decks(&game, filler, 10);
        game.state.p[0].stage[1] = sumire;
        test_give_energy(&game, 5);
        int hand_before = df_hand(&game);

        CHECK(df_fire_live_success(&game, sumire), "PL!SP-pb1-004-R exposes live-success");
        CHECK(test_has_pending_choice(&game),
              pay ? "LiveSuccess should present the pay 3E choice"
                  : "LiveSuccess should present the pay/skip choice when skipped");
        df_select_option(&game, pay ? 1 : 0);
        df_drain(&game);

        if (pay) {
            CHECK_EQ(df_hand(&game), hand_before + 1, "should draw 1 on pay");
            CHECK_EQ(df_energy(&game), 2, "5-3=2 active left");
        } else {
            CHECK_EQ(df_hand(&game), hand_before, "skip should not draw");
            CHECK_EQ(df_energy(&game), 5, "skip should not pay");
        }
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   10. optional_energy_live_success_draw_test.rs - the same optional-energy
       shape at E and at EEE, again with the ability's OWN delta measured (the
       Rust file replaced a vacuous "the deck got smaller" assertion with exact
       energy + deck deltas; the C port keeps the exact deltas). The five
       blind `pass()` calls of the Rust setup only moved the live card from the
       hand into the live zone, so the C setup puts the live card straight into
       the live zone - the measured window is unchanged.
   ????????????????????????????????????????????????????????????????????????? */
static void test_optional_energy_live_success_draw(void)
{
    struct { const char *live_no; int energy; int spend; int draws; } CASES[] = {
        { "PL!SP-pb1-004-R", 6, 3, 1 },   /* EEE, pay   */
        { "PL!SP-bp5-020-N", 5, 1, 1 },   /* E,   pay   */
        { "PL!SP-bp5-020-N", 5, 0, 0 },   /* E,   skip  */
        { "PL!SP-bp5-020-N", 0, 0, 0 },   /* no energy: cannot pay */
    };
    for (unsigned i = 0; i < sizeof(CASES) / sizeof(CASES[0]); i++) {
        TestGame game;
        test_game_new(&game);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int live = test_id(&game, CASES[i].live_no);
        int m1 = df_new_id(&game, "PL!-sd1-001-SD");
        int m2 = df_new_id(&game, "PL!-sd1-001-SD");
        int m3 = df_new_id(&game, "PL!-sd1-001-SD");
        CHECK(live >= 0 && m1 >= 0 && m2 >= 0 && m3 >= 0,
              "optional-energy live fixtures resolve");
        if (live < 0 || m1 < 0 || m2 < 0 || m3 < 0) return;
        df_fill_decks(&game, filler, 20);
        game.state.p[0].stage[0] = m1;
        game.state.p[0].stage[1] = m2;
        game.state.p[0].stage[2] = m3;
        test_add_to_live(&game, live);
        test_give_energy(&game, CASES[i].energy);
        int deck_before = df_deck(&game);
        int energy_before = df_energy(&game);

        CHECK(df_fire_live_success(&game, live), "optional-energy live exposes live-success");
        if (test_has_pending_choice(&game)) {
            const RbChoice *ch = rb_get_pending_choice(&game.state);
            char msg[256];
            snprintf(msg, sizeof(msg),
                     "%s: the optional cost prompt must be skippable (allow_skip=%d)",
                     CASES[i].live_no, ch ? ch->allow_skip : -1);
            CHECK(ch && ch->allow_skip, msg);
            snprintf(msg, sizeof(msg),
                     "%s: the pay/skip prompt must name what it is paying for (got '%s')",
                     CASES[i].live_no, ch ? ch->description : "(null)");
            CHECK(ch && ch->description[0] != '\0', msg);
            df_select_option(&game, CASES[i].spend > 0 ? 1 : 0);
        }
        df_drain(&game);

        char msg[256];
        snprintf(msg, sizeof(msg), "%s (%dE): expected energy delta %d", CASES[i].live_no,
                 CASES[i].energy, CASES[i].spend);
        CHECK_EQ(energy_before - df_energy(&game), CASES[i].spend, msg);
        snprintf(msg, sizeof(msg), "%s (%dE): expected deck delta %d", CASES[i].live_no,
                 CASES[i].energy, CASES[i].draws);
        CHECK_EQ(deck_before - df_deck(&game), CASES[i].draws, msg);
        snprintf(msg, sizeof(msg), "%s (%dE): no prompt may be left dangling", CASES[i].live_no,
                 CASES[i].energy);
        CHECK(!test_has_pending_choice(&game), msg);
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   11. per_own_stage_member_draw_then_discard_q146_test.rs - a PER-UNIT
       multiplier in front of a flat draw (PL!-bp3-004-R+ Kumi: draw 1 per
       member on MY stage, then discard 1). Driven through a real play so the
       debut ability fires the way Rust drives it. Q146: the activating member
       counts, the OPPONENT's members do not.
   ????????????????????????????????????????????????????????????????????????? */
#define KUMI "PL!-bp3-004-R\xEF\xBC\x8B"
static void test_per_own_stage_member_draw_then_discard_q146(void)
{
    struct { int own_before; int opp_members; int net; const char *msg; } CASES[] = {
        { 0, 0, -1, "Q146: 1 member -> draw 1 discard 1 = net 0" },
        { 1, 0,  0, "Q146: 2 members -> draw 2 discard 1 = net 0" },
        { 2, 0,  1, "Q146: 3 members -> draw 3 discard 1 = net +1" },
        { 0, 3, -1, "Q146: opponent members not counted -> net 0" },
    };
    for (unsigned i = 0; i < sizeof(CASES) / sizeof(CASES[0]); i++) {
        TestGame game;
        test_game_new(&game);
        int kumi = test_id(&game, KUMI);
        int live = test_id(&game, "PL!-sd1-019-SD");
        int filler = test_id(&game, "PL!-sd1-002-SD");
        CHECK(kumi >= 0 && live >= 0 && filler >= 0, "PL!-bp3-004-R+ fixtures resolve");
        if (kumi < 0 || live < 0 || filler < 0) return;

        for (int a = 0; a < CASES[i].own_before; a++)
            game.state.p[0].stage[a] = df_new_id(&game, "PL!-sd1-002-SD");
        for (int a = 0; a < CASES[i].opp_members; a++)
            game.state.p[1].stage[a] = df_new_id(&game, "PL!-sd1-002-SD");
        test_add_to_hand(&game, kumi);
        test_add_to_hand(&game, live);
        df_fill_decks(&game, filler, 20);
        test_give_energy(&game, 20);

        int hand_before = df_hand(&game);
        int deck_before = df_deck(&game);
        int wait_before = game.state.p[0].discard.n;
        /* Rust plays into the first free area; the C shim needs it spelled out. */
        int area = CASES[i].own_before; /* 0 left, 1 centre, 2 right */
        test_play_to_stage(&game, kumi, area);
        df_drain_pick_first(&game);

        CHECK_EQ(df_hand(&game), hand_before + CASES[i].net, CASES[i].msg);
        if (CASES[i].own_before == 2)
            CHECK_EQ(deck_before - df_deck(&game), 3,
                     "Q146: 3 members -> exactly 3 cards left the deck");
        if (CASES[i].opp_members == 3)
            CHECK(game.state.p[1].stage[0] != RB_EMPTY_SLOT,
                  "Q146: the opponent's stage is untouched");
        if (CASES[i].own_before == 0)
            CHECK_EQ(game.state.p[0].discard.n, wait_before + 1,
                     "Q146: exactly one card is discarded after the draw");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   12. optional_wait_members_draw_per_cost_member_q183_test.rs - a per-UNIT
       draw whose multiplier is the number of members THIS COST waited
       (PL!-pb1-008-R Hanayo). The interesting assertions are the scope ones:
       the cost is optional, it never touches the opponent, and a member that
       was ALREADY waited does not inflate the count (Q183 / Q137).
   ????????????????????????????????????????????????????????????????????????? */
static void test_optional_wait_members_draw_per_cost_member_q183(void)
{
    const char *HANAYO = "PL!-pb1-008-R";

    /* skip: nothing waited, nothing drawn */
    {
        TestGame game;
        test_game_new(&game);
        int hanayo = test_id(&game, HANAYO);
        int friend = test_id(&game, "PL!-sd1-001-SD");
        CHECK(hanayo >= 0 && friend >= 0, "PL!-pb1-008-R fixtures resolve");
        if (hanayo < 0 || friend < 0) return;
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        game.state.p[0].stage[0] = friend;
        test_add_to_hand(&game, hanayo);
        test_give_energy(&game, 15);
        int deck_before = df_deck(&game);

        test_play_to_stage(&game, hanayo, 1);
        CHECK(test_has_pending_choice(&game), "debut must prompt for the optional wait cost");
        df_select_option(&game, 0); /* skip */
        df_drain(&game);
        CHECK(!test_has_pending_choice(&game), "skip resolves without further prompts");
        CHECK(rb_mods_get_orientation(&game.state.mods, hanayo) == NULL,
              "skipping the cost must not wait Hanayo");
        CHECK(rb_mods_get_orientation(&game.state.mods, friend) == NULL,
              "skipping the cost must not wait other members");
        CHECK_EQ(df_hand(&game), 0, "skip draws nothing (-1 played, +0 drawn)");
        CHECK_EQ(deck_before - df_deck(&game), 0, "deck untouched when the cost is skipped");
    }

    /* pay with an opponent member on stage: 2 own members waited -> draw 2,
       the OPPONENT's member is never touched (Q183) */
    {
        TestGame game;
        test_game_new(&game);
        int hanayo = test_id(&game, HANAYO);
        int friend = test_id(&game, "PL!-sd1-001-SD");
        int opp = df_new_id(&game, "PL!-sd1-002-SD");
        CHECK(hanayo >= 0 && friend >= 0, "PL!-pb1-008-R pay fixtures resolve");
        if (hanayo < 0 || friend < 0 || opp < 0) return;
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        game.state.p[0].stage[0] = friend;
        game.state.p[1].stage[0] = opp;
        test_add_to_hand(&game, hanayo);
        test_give_energy(&game, 15);
        int deck_before = df_deck(&game);

        test_play_to_stage(&game, hanayo, 1);
        CHECK(test_has_pending_choice(&game), "pay/skip prompt must appear");
        df_select_option(&game, 1); /* pay */
        df_drain(&game);
        CHECK(!test_has_pending_choice(&game),
              "with <=3 candidates the wait applies automatically - no further prompts");

        CHECK(rb_mods_get_orientation(&game.state.mods, hanayo) != NULL &&
              !strcmp(rb_mods_get_orientation(&game.state.mods, hanayo), "wait"),
              "Hanayo herself is a valid cost target and must be waited");
        CHECK(rb_mods_get_orientation(&game.state.mods, friend) != NULL &&
              !strcmp(rb_mods_get_orientation(&game.state.mods, friend), "wait"),
              "own stage member must be waited");
        CHECK(rb_mods_get_orientation(&game.state.mods, opp) == NULL,
              "Q183: opponent member must NEVER be waited by this cost");
        CHECK_EQ(game.state.p[1].stage[0], opp, "opponent member stays on stage");
        CHECK_EQ(df_hand(&game), 2, "waited 2 members -> drew 2");
        CHECK_EQ(deck_before - df_deck(&game), 2, "exactly 2 cards left the deck");
    }

    /* an ALREADY-waited friend must not inflate the count: only Hanayo is newly
       waited, so exactly one card is drawn (Q137 + the "by this" scoping) */
    {
        TestGame game;
        test_game_new(&game);
        int hanayo = test_id(&game, HANAYO);
        int friend = test_id(&game, "PL!-sd1-001-SD");
        CHECK(hanayo >= 0 && friend >= 0, "PL!-pb1-008-R pre-waited fixtures resolve");
        if (hanayo < 0 || friend < 0) return;
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        game.state.p[0].stage[0] = friend;
        rb_mods_set_orientation(&game.state.mods, friend, "wait");
        test_add_to_hand(&game, hanayo);
        test_give_energy(&game, 15);

        test_play_to_stage(&game, hanayo, 1);
        CHECK(test_has_pending_choice(&game),
              "Hanayo is active, so a cost candidate exists - prompt must appear");
        df_select_option(&game, 1); /* pay */
        df_drain(&game);
        CHECK(rb_mods_get_orientation(&game.state.mods, hanayo) != NULL &&
              !strcmp(rb_mods_get_orientation(&game.state.mods, hanayo), "wait"),
              "Hanayo gets waited by the cost");
        CHECK(rb_mods_get_orientation(&game.state.mods, friend) != NULL &&
              !strcmp(rb_mods_get_orientation(&game.state.mods, friend), "wait"),
              "friend keeps her pre-existing wait");
        CHECK_EQ(df_hand(&game), 1,
                 "1 newly-waited member -> drew exactly 1 (-1 played +1)");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   13. turn1_energy_draw_discard_pl_sp_bp1_009_r_test.rs - draw + discard over
       an ACTIVATION energy cost with a per-turn budget (PL!SP-bp1-009-R
       Natsumi). The Rust file pins the offer, the payment, the net hand delta,
       the waitroom gain, the recorded use and the refusal to re-fire; the C
       port keeps all six, with rb_generate_action_candidates standing in for
       Rust's game_setup::generate_possible_actions offer count.
   ????????????????????????????????????????????????????????????????????????? */
#define NATSUMI "PL!SP-bp1-009-R"
static void test_turn1_energy_draw_discard_natsumi(void)
{
    /* full behaviour */
    {
        TestGame game;
        test_game_new(&game);
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        int natsumi = test_id(&game, NATSUMI);
        CHECK(natsumi >= 0, "PL!SP-bp1-009-R fixture resolves");
        if (natsumi < 0) return;
        game.state.p[0].stage[1] = natsumi;
        test_give_energy(&game, 2);
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));

        int energy_before = df_energy(&game);
        int hand_before = df_hand(&game);
        int wait_before = game.state.p[0].discard.n;

        CHECK_EQ(df_count_use_offers(&game, natsumi), 1,
                 "fresh turn offers the ability exactly once");
        CHECK(rb_activate_card(&game.state, 0, natsumi) == 1, "the ability activates");
        df_drain_pick_first(&game);
        CHECK(!test_has_pending_choice(&game), "prompts terminate after answering");

        CHECK_EQ(df_energy(&game), energy_before - 1, "1E cost was paid");
        CHECK_EQ(df_hand(&game), hand_before, "drew 1 then discarded 1 (net unchanged)");
        CHECK_EQ(game.state.p[0].discard.n, wait_before + 1,
                 "the discarded hand card reached the waitroom");
        CHECK(df_turn1_used(&game, natsumi), "the per-turn use is recorded");
        CHECK_EQ(df_count_use_offers(&game, natsumi), 0,
                 "consumed per-turn ability must not be re-offered");
    }

    /* with 0 energy the cost is unpayable: not offered, nothing is drawn */
    {
        TestGame game;
        test_game_new(&game);
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        int natsumi = test_id(&game, NATSUMI);
        if (natsumi < 0) return;
        game.state.p[0].stage[1] = natsumi;
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        CHECK_EQ(df_count_use_offers(&game, natsumi), 0,
                 "1E cost with 0 energy must not be offered");
    }

    /* exactly 1 energy pays the cost and leaves 0 active */
    {
        TestGame game;
        test_game_new(&game);
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        int natsumi = test_id(&game, NATSUMI);
        if (natsumi < 0) return;
        game.state.p[0].stage[1] = natsumi;
        test_give_energy(&game, 1);
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        CHECK_EQ(df_count_use_offers(&game, natsumi), 1,
                 "exactly 1 energy still offers the ability");
        rb_activate_card(&game.state, 0, natsumi);
        df_drain_pick_first(&game);
        CHECK_EQ(df_energy(&game), 0, "exact 1E paid, none left");
    }

    /* the per-turn budget resets on the next turn */
    {
        TestGame game;
        test_game_new(&game);
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        int natsumi = test_id(&game, NATSUMI);
        if (natsumi < 0) return;
        game.state.p[0].stage[1] = natsumi;
        test_give_energy(&game, 2);
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        rb_activate_card(&game.state, 0, natsumi);
        df_drain_pick_first(&game);
        CHECK(df_turn1_used(&game, natsumi), "the first firing consumes the per-turn budget");
        CHECK_EQ(df_count_use_offers(&game, natsumi), 0,
                 "the consumed ability is not offered again this turn");
        game.state.turn += 1;
        CHECK_EQ(df_count_use_offers(&game, natsumi), 1,
                 "per-turn limit must reset on the next turn");
    }

    /* hostile: bypass the action generator and fire twice directly at the
       resolver. The generation filter is not the only defense - the resolver
       gate must refuse the second firing. */
    {
        TestGame game;
        test_game_new(&game);
        df_fill_decks(&game, test_id(&game, "PL!-sd1-010-SD"), 40);
        int natsumi = test_id(&game, NATSUMI);
        if (natsumi < 0) return;
        game.state.p[0].stage[1] = natsumi;
        test_give_energy(&game, 3);
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        test_add_to_hand(&game, df_new_id(&game, "PL!-sd1-010-SD"));
        rb_activate_card(&game.state, 0, natsumi);
        df_drain_pick_first(&game);
        int hand_after_first = df_hand(&game);
        int wait_after_first = game.state.p[0].discard.n;
        int energy_after_first = df_energy(&game);

        rb_activate_card(&game.state, 0, natsumi);
        df_drain_pick_first(&game);
        CHECK_EQ(df_hand(&game), hand_after_first, "refused activation must not draw");
        CHECK_EQ(game.state.p[0].discard.n, wait_after_first,
                 "refused activation must not discard");
        CHECK_EQ(df_energy(&game), energy_after_first,
                 "refused activation must not pay the 1E cost");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   14. miyashita_ai_pb1_test.rs - a flat draw behind an APPEARANCE watcher
       (PL!N-pb1-005-R: when a cost-10 member APPEARS on your stage, draw 1),
       plus the Q197 negative: she must not fire when the cost-10 member
       appears by REPLACING her (baton touch).
   ????????????????????????????????????????????????????????????????????????? */
static void test_miyashita_ai_cost10_appears_draw(void)
{
    /* cost-10 member appears in a DIFFERENT area -> draw 1 */
    {
        TestGame game;
        test_game_new(&game);
        int ai = test_id(&game, "PL!N-pb1-005-R");
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int cost10 = test_id(&game, "PL!SP-bp2-006-P");   /* cost 10 */
        CHECK(ai >= 0 && cost10 >= 0, "PL!N-pb1-005-R fixtures resolve");
        if (ai < 0 || cost10 < 0) return;
        game.state.p[0].deck.n = 0;
        for (int i = 0; i < 30; i++) test_add_to_deck(&game, filler);
        game.state.p[0].stage[0] = ai;
        game.state.p[0].stage[1] = filler;
        test_add_to_hand(&game, cost10);
        test_add_to_hand(&game, filler);
        test_give_energy(&game, 12);

        test_play_to_stage(&game, cost10, 2 /* right */);
        df_drain_pick_first(&game);
        CHECK_EQ(df_hand(&game), 2,
                 "Ai should draw 1 when cost-10 member appears on a different area");
    }
    /* cost-10 member REPLACES Ai via baton touch -> Q197: Ai must not fire */
    {
        TestGame game;
        test_game_new(&game);
        int ai = test_id(&game, "PL!N-pb1-005-R");
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int cost10 = test_id(&game, "PL!SP-bp2-006-P");
        CHECK(ai >= 0 && cost10 >= 0, "PL!N-pb1-005-R baton fixtures resolve");
        if (ai < 0 || cost10 < 0) return;
        game.state.p[0].deck.n = 0;
        for (int i = 0; i < 30; i++) test_add_to_deck(&game, filler);
        game.state.p[0].stage[0] = ai;
        game.state.p[0].stage[1] = filler;
        test_add_to_hand(&game, cost10);
        test_add_to_hand(&game, filler);
        test_give_energy(&game, 12);

        test_play_to_stage(&game, cost10, 0 /* left, over Ai */);
        df_drain_pick_first(&game);
        CHECK_EQ(df_hand(&game), 1,
                 "Q197: Ai's auto should NOT trigger when she is replaced by baton touch");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   15. position_ability_test.rs - the same flat draw GATED ON THE CARD'S OWN
       POSITION (PL!SP-bp1-002-R+ Coco: draw 2 only from the left side;
       PL!SP-bp4-003-R Chisato: draw 2 + discard 2 from either side but never
       from the centre).
   ????????????????????????????????????????????????????????????????????????? */
#define COCO    "PL!SP-bp1-002-R\xEF\xBC\x8B"
#define CHISATO "PL!SP-bp4-003-R"
static void test_position_gated_draw(void)
{
    /* Coco: left draws, centre and right do not */
    {
        struct { int area; int net; const char *msg; } C[] = {
            { 0,  0, "Coco on Left: should draw (play -1, draw +2 => +1 net of -1)" },
            { 1, -1, "Coco on Center: should NOT draw" },
            { 2, -1, "Coco on Right: should NOT draw" },
        };
        for (unsigned i = 0; i < 3; i++) {
            TestGame game;
            test_game_new(&game);
            int coco = test_id(&game, COCO);
            int filler = test_id(&game, "PL!-sd1-010-SD");
            CHECK(coco >= 0, "PL!SP-bp1-002-R+ fixture resolves");
            if (coco < 0) return;
            df_fill_decks(&game, filler, 30);
            df_clear_stage(&game);
            test_add_to_hand(&game, coco);
            test_add_to_hand(&game, filler);
            test_add_to_hand(&game, filler);
            test_give_energy(&game, 10);
            int before = df_hand(&game);
            test_play_to_stage(&game, coco, C[i].area);
            if (C[i].area == 0)
                CHECK(test_has_pending_choice(&game),
                      "Coco on Left: the optional 2E cost must be offered");
            df_select_option(&game, 1);
            df_drain(&game);
            CHECK_EQ(df_hand(&game) - before, C[i].net, C[i].msg);
        }
    }
    /* Chisato: left and right both draw 2 and discard 2 (net -1 overall) */
    {
        struct { int area; const char *msg; } C[] = {
            { 0, "Chisato on Left: draw 2 + discard 2 should net hand_before - 1" },
            { 2, "Chisato on Right: draw 2 + discard 2 should net hand_before - 1" },
        };
        for (unsigned i = 0; i < 2; i++) {
            TestGame game;
            test_game_new(&game);
            int chisato = test_id(&game, CHISATO);
            int filler = test_id(&game, "PL!-sd1-010-SD");
            CHECK(chisato >= 0, "PL!SP-bp4-003-R fixture resolves");
            if (chisato < 0) return;
            df_fill_decks(&game, filler, 30);
            df_clear_stage(&game);
            test_add_to_hand(&game, chisato);
            test_add_to_hand(&game, filler);
            test_add_to_hand(&game, filler);
            test_give_energy(&game, 10);
            int before = df_hand(&game);
            test_play_to_stage(&game, chisato, C[i].area);
            CHECK(test_has_pending_choice(&game), "Chisato: ability should have triggered");
            df_drain_pick_first(&game);
            CHECK_EQ(df_hand(&game), before - 1, C[i].msg);
        }
    }
    /* Chisato on the centre must NOT activate at all */
    {
        TestGame game;
        test_game_new(&game);
        int chisato = test_id(&game, CHISATO);
        int filler = test_id(&game, "PL!-sd1-010-SD");
        CHECK(chisato >= 0, "PL!SP-bp4-003-R centre fixture resolves");
        if (chisato < 0) return;
        df_fill_decks(&game, filler, 30);
        df_clear_stage(&game);
        test_add_to_hand(&game, chisato);
        test_add_to_hand(&game, filler);
        test_give_energy(&game, 10);
        int before = df_hand(&game);
        test_play_to_stage(&game, chisato, 1);
        while (test_has_pending_choice(&game)) {
            const RbChoice *ch = rb_get_pending_choice(&game.state);
            if (ch && ch->kind == RB_CHOICE_SELECT_CARD) {
                fprintf(stderr, "FAIL: Chisato on Center should NOT have fired, but got "
                                "SelectCard zone=%s count=%d\n",
                        ch->zone, ch->count);
                checks++; failures++;
                break;
            }
            test_resume_choice(&game, -1);
        }
        CHECK_EQ(df_hand(&game), before - 1, "Chisato on Center: should NOT activate");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   16. success_zone_gated_debut_draw_test.rs - a flat draw gated on the
       SUCCESS LIVE CARD ZONE being non-empty (PL!-pb1-005-R). The positive
       asserts the NET (play -1, draw +1 => unchanged) plus that the card is
       really on stage; the negative asserts the deck does not move at all.
   ????????????????????????????????????????????????????????????????????????? */
static void test_success_zone_gated_debut_draw(void)
{
    {
        TestGame game;
        test_game_new(&game);
        int card = test_id(&game, "PL!-pb1-005-R");
        int filler = test_id(&game, "PL!-sd1-010-SD");
        int won = test_id(&game, "PL!-sd1-020-SD");
        CHECK(card >= 0 && won >= 0, "PL!-pb1-005-R fixtures resolve");
        if (card < 0 || won < 0) return;
        df_fill_decks(&game, filler, 30);
        test_add_to_success(&game, won);
        test_give_energy(&game, 10);
        test_add_to_hand(&game, card);
        int before = df_hand(&game);
        int deck_before = df_deck(&game);
        test_play_to_stage(&game, card, 1);
        df_drain(&game);
        CHECK_EQ(df_hand(&game), before,
                 "play (-1) + conditional draw (+1) => hand count unchanged");
        CHECK_EQ(deck_before - df_deck(&game), 1, "exactly one card was drawn");
        CHECK(!test_hand_has(&game, card), "card itself must be on stage, not back in hand");
        CHECK_EQ(game.state.p[0].stage[1], card, "the card is on the centre stage");
    }
    {
        TestGame game;
        test_game_new(&game);
        int card = test_id(&game, "PL!-pb1-005-R");
        int filler = test_id(&game, "PL!-sd1-010-SD");
        CHECK(card >= 0, "PL!-pb1-005-R negative fixture resolves");
        if (card < 0) return;
        df_fill_decks(&game, filler, 30);
        CHECK_EQ(game.state.p[0].success.n, 0, "precondition: empty success zone");
        test_give_energy(&game, 10);
        test_add_to_hand(&game, card);
        int deck_before = df_deck(&game);
        test_play_to_stage(&game, card, 1);
        df_drain(&game);
        CHECK_EQ(df_hand(&game), 0, "without success-zone cards the debut must not draw");
        CHECK_EQ(deck_before - df_deck(&game), 0, "the deck is untouched");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   17. distinct_draw_regression_test.rs - the draw.rs DISTINCT filter, called
       straight on the resolver entry point with both sources. These are
       unit-level in Rust too (they build the AbilityEffect by hand), so they
       translate one-for-one. `first` and `duplicate` are two DISTINCT card
       instances of the SAME print, which is what makes the dedupe observable.
   ????????????????????????????????????????????????????????????????????????? */
static void test_distinct_draw_regression(void)
{
    /* 17a. distinct keeps a duplicate in the pile and scans on for another
           NAME. deck: [first, duplicate, other] / waitroom: [other, dup, first] */
    for (int src = 0; src < 2; src++) {
        TestGame game;
        test_game_new(&game);
        int first = test_id(&game, "PL!-sd1-010-SD");
        int duplicate = df_new_id(&game, "PL!-sd1-010-SD");
        int other = test_id(&game, "PL!HS-bp1-005-P");
        CHECK(first >= 0 && duplicate >= 0 && other >= 0, "distinct fixtures resolve");
        if (first < 0 || other < 0) return;
        CHECK(first != duplicate, "the two instances are distinct card ids");
        CHECK(rb_card_no_eq(first, "PL!-sd1-010-SD") &&
              rb_card_no_eq(duplicate, "PL!-sd1-010-SD"),
              "both instances are the SAME print (CRITICAL IDENTITY)");

        RbPlayer *P = &game.state.p[0];
        if (src == 0) {
            test_add_to_deck(&game, first);
            test_add_to_deck(&game, duplicate);
            test_add_to_deck(&game, other);
        } else {
            test_add_to_discard(&game, other);
            test_add_to_discard(&game, duplicate);
            test_add_to_discard(&game, first);
        }
        AbilityEffect e;
        df_make_draw(&e, 2, "self", src == 0 ? "deck" : "discard", "hand");
        df_set_extra(&e, "distinct", "card_name");
        rb_effect_draw_card(&game.state, 0, &e, -1);

        RbBag *pile = (src == 0) ? &P->deck : &P->discard;
        const char *S = src == 0 ? "deck" : "discard";
        char msg[160];
        snprintf(msg, sizeof(msg), "%s: distinct draw puts 2 cards in hand", S);
        CHECK_EQ(P->hand.n, 2, msg);
        snprintf(msg, sizeof(msg), "%s: hand[0] is the first instance", S);
        CHECK_EQ(P->hand.n > 0 ? P->hand.cards[0] : -1, first, msg);
        snprintf(msg, sizeof(msg), "%s: hand[1] is the other NAME", S);
        CHECK_EQ(P->hand.n > 1 ? P->hand.cards[1] : -1, other, msg);
        snprintf(msg, sizeof(msg), "%s: the duplicate is left behind", S);
        CHECK_EQ(pile->n, 1, msg);
        snprintf(msg, sizeof(msg), "%s: the leftover IS the duplicate", S);
        CHECK_EQ(pile->n > 0 ? pile->cards[0] : -1, duplicate, msg);
    }

    /* 17b. distinct stops when only duplicates remain (asks 3, gets 1) */
    for (int src = 0; src < 2; src++) {
        TestGame game;
        test_game_new(&game);
        int first = test_id(&game, "PL!-sd1-010-SD");
        int duplicate = df_new_id(&game, "PL!-sd1-010-SD");
        CHECK(first >= 0 && duplicate >= 0, "distinct stop fixtures resolve");
        if (first < 0) return;
        RbPlayer *P = &game.state.p[0];
        if (src == 0) {
            test_add_to_deck(&game, first);
            test_add_to_deck(&game, duplicate);
        } else {
            test_add_to_discard(&game, duplicate);
            test_add_to_discard(&game, first);
        }
        AbilityEffect e;
        df_make_draw(&e, 3, "self", src == 0 ? "deck" : "discard", "hand");
        df_set_extra(&e, "distinct", "true");
        rb_effect_draw_card(&game.state, 0, &e, -1);

        RbBag *pile = (src == 0) ? &P->deck : &P->discard;
        const char *S = src == 0 ? "deck" : "discard";
        char msg[160];
        snprintf(msg, sizeof(msg), "%s: only one distinct name remains => hand has 1 card", S);
        CHECK_EQ(P->hand.n, 1, msg);
        snprintf(msg, sizeof(msg), "%s: the drawn card is the first instance", S);
        CHECK_EQ(P->hand.n > 0 ? P->hand.cards[0] : -1, first, msg);
        snprintf(msg, sizeof(msg), "%s: the duplicate stays in the pile", S);
        CHECK_EQ(pile->n, 1, msg);
        snprintf(msg, sizeof(msg), "%s: the leftover IS the duplicate", S);
        CHECK_EQ(pile->n > 0 ? pile->cards[0] : -1, duplicate, msg);
    }

    /* 17c. an ORDINARY draw keeps same-name cards */
    for (int src = 0; src < 2; src++) {
        TestGame game;
        test_game_new(&game);
        int first = test_id(&game, "PL!-sd1-010-SD");
        int duplicate = df_new_id(&game, "PL!-sd1-010-SD");
        CHECK(first >= 0 && duplicate >= 0, "ordinary draw fixtures resolve");
        if (first < 0) return;
        RbPlayer *P = &game.state.p[0];
        if (src == 0) {
            test_add_to_deck(&game, first);
            test_add_to_deck(&game, duplicate);
        } else {
            test_add_to_discard(&game, duplicate);
            test_add_to_discard(&game, first);
        }
        AbilityEffect e;
        df_make_draw(&e, 2, "self", src == 0 ? "deck" : "discard", "hand");
        rb_effect_draw_card(&game.state, 0, &e, -1);
        const char *S = src == 0 ? "deck" : "discard";
        char msg[160];
        snprintf(msg, sizeof(msg), "%s: an ordinary draw keeps both same-name cards", S);
        CHECK_EQ(P->hand.n, 2, msg);
        snprintf(msg, sizeof(msg), "%s: hand[0] is the first instance", S);
        CHECK_EQ(P->hand.n > 0 ? P->hand.cards[0] : -1, first, msg);
        snprintf(msg, sizeof(msg), "%s: hand[1] is the duplicate", S);
        CHECK_EQ(P->hand.n > 1 ? P->hand.cards[1] : -1, duplicate, msg);
        snprintf(msg, sizeof(msg), "%s: the source pile is empty", S);
        CHECK_EQ(P->deck.n, 0, msg);
        snprintf(msg, sizeof(msg), "%s: the other pile is untouched", S);
        CHECK_EQ(P->discard.n, 0, msg);
    }

    /* 17d. a filtered draw stops after scanning a non-matching deck, with and
           without distinct */
    for (int with_distinct = 0; with_distinct < 2; with_distinct++) {
        TestGame game;
        test_game_new(&game);
        int member = test_id(&game, "PL!-sd1-010-SD");
        CHECK(member >= 0, "filtered draw fixture resolves");
        if (member < 0) return;
        test_add_to_deck(&game, member);
        AbilityEffect e;
        df_make_draw(&e, 2, "self", "deck", "hand");
        df_set_extra(&e, "card_type", "live_card");
        if (with_distinct) df_set_extra(&e, "distinct", "card_name");
        rb_effect_draw_card(&game.state, 0, &e, -1);
        char msg[160];
        snprintf(msg, sizeof(msg),
                 "live_card filter%s: a member on top of the deck is never drawn",
                 with_distinct ? " + distinct" : "");
        CHECK_EQ(game.state.p[0].hand.n, 0, msg);
        snprintf(msg, sizeof(msg), "live_card filter%s: the member is still in the deck",
                 with_distinct ? " + distinct" : "");
        CHECK_EQ(game.state.p[0].deck.n, 1, msg);
        snprintf(msg, sizeof(msg), "live_card filter%s: the member is untouched",
                 with_distinct ? " + distinct" : "");
        CHECK_EQ(game.state.p[0].deck.n > 0 ? game.state.p[0].deck.cards[0] : -1, member, msg);
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   18. The FLAT-DRAW engine surface itself (rb_effect_draw_card), for the
       behaviours the card-level tests above reach only indirectly: fixed
       counts, deck exhaustion and waitroom refresh, per-unit multipliers
       (stage members and this-cost-waited members), the card_type filter, an
       alternate destination, and the three target routings. draw.rs:355-611.
   ????????????????????????????????????????????????????????????????????????? */
static void test_flat_draw_engine_surface(void)
{
    /* fixed counts, from the deck and from the waitroom, into the hand */
    for (int src = 0; src < 2; src++) {
        static const int COUNTS[] = { 1, 2, 5 };
        const char *S = src == 0 ? "deck" : "discard";
        for (unsigned c = 0; c < 3; c++) {
            TestGame tg;
            test_game_new(&tg);
            int filler = test_id(&tg, "PL!-sd1-010-SD");
            RbPlayer *P = &tg.state.p[0];
            for (int i = 0; i < 12; i++) {
                if (src == 0) test_add_to_deck(&tg, filler);
                else test_add_to_discard(&tg, filler);
            }
            AbilityEffect e;
            df_make_draw(&e, COUNTS[c], "self", S, "hand");
            rb_effect_draw_card(&tg.state, 0, &e, -1);
            char msg[160];
            snprintf(msg, sizeof(msg), "%s: a fixed draw of %d moves exactly %d cards to hand",
                     S, COUNTS[c], COUNTS[c]);
            CHECK_EQ(P->hand.n, COUNTS[c], msg);
            snprintf(msg, sizeof(msg), "%s: a fixed draw of %d empties that many from the source",
                     S, COUNTS[c]);
            CHECK_EQ(src == 0 ? P->deck.n : P->discard.n, 12 - COUNTS[c], msg);
            snprintf(msg, sizeof(msg), "%s: a fixed draw of %d reports last_draw_count",
                     S, COUNTS[c]);
            CHECK_EQ(tg.state.last_draw_count, COUNTS[c], msg);
        }
    }

    /* deck exhaustion: an empty deck refills from the waitroom and keeps going
       (Q104 / Rule 10.2.1, draw.rs:70-77) */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        RbPlayer *P = &tg.state.p[0];
        for (int i = 0; i < 2; i++) test_add_to_deck(&tg, filler);
        for (int i = 0; i < 4; i++) test_add_to_discard(&tg, filler);
        AbilityEffect e;
        df_make_draw(&e, 5, "self", "deck", "hand");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(P->hand.n, 5, "an exhausted deck refills from the waitroom and still draws 5");
        CHECK_EQ(P->deck.n, 1, "the refreshed deck keeps the surplus");
        CHECK_EQ(P->discard.n, 0, "the waitroom is spent on the refresh");
    }
    /* a draw larger than deck+waitroom takes everything and stops */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        RbPlayer *P = &tg.state.p[0];
        for (int i = 0; i < 2; i++) test_add_to_deck(&tg, filler);
        test_add_to_discard(&tg, filler);
        AbilityEffect e;
        df_make_draw(&e, 9, "self", "deck", "hand");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(P->hand.n, 3, "a short deck+waitroom yields only what exists");
        CHECK_EQ(P->deck.n, 0, "the deck ends empty");
        CHECK_EQ(P->discard.n, 0, "the waitroom ends empty");
    }

    /* per_unit multiplier over the ACTOR's own stage members */
    for (int members = 0; members <= 3; members++) {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        RbPlayer *P = &tg.state.p[0];
        for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);
        for (int i = 0; i < members; i++) P->stage[i] = df_new_id(&tg, "PL!-sd1-010-SD");
        AbilityEffect e;
        df_make_draw(&e, 1, "self", "deck", "hand");
        df_set_extra(&e, "per_unit", "true");
        df_set_extra(&e, "per_unit_count", "1");
        df_set_extra(&e, "per_unit_type", "member");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        char msg[160];
        snprintf(msg, sizeof(msg), "per_unit member: %d staged member(s) draw %d card(s)",
                 members, members);
        CHECK_EQ(P->hand.n, members, msg);
    }
    /* per_unit over the OPPONENT's stage members must not count */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);
        for (int i = 0; i < 3; i++)
            tg.state.p[1].stage[i] = df_new_id(&tg, "PL!-sd1-010-SD");
        AbilityEffect e;
        df_make_draw(&e, 1, "self", "deck", "hand");
        df_set_extra(&e, "per_unit", "true");
        df_set_extra(&e, "per_unit_type", "member");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(tg.state.p[0].hand.n, 0,
                 "per_unit member counts only the ACTOR's own stage");
    }
    /* per_unit_source = this_cost_waited: the multiplier is the number of
       members THIS cost waited, not the number waiting on stage (Q183) */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        RbPlayer *P = &tg.state.p[0];
        for (int i = 0; i < 10; i++) test_add_to_deck(&tg, filler);
        /* three members on stage, all already waited by a PREVIOUS effect */
        for (int i = 0; i < 3; i++) {
            int m = df_new_id(&tg, "PL!-sd1-010-SD");
            P->stage[i] = m;
            rb_mods_set_orientation(&tg.state.mods, m, "wait");
            P->stage_wait[i] = 1;
        }
        /* this cost waited exactly one member */
        tg.state.last_cost_waited_members[0] = P->stage[0];
        tg.state.n_last_cost_waited_members = 1;
        AbilityEffect e;
        df_make_draw(&e, 1, "self", "deck", "hand");
        df_set_extra(&e, "per_unit", "true");
        df_set_extra(&e, "per_unit_type", "member");
        df_set_extra(&e, "per_unit_source", "this_cost_waited");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(P->hand.n, 1,
                 "the per-cost 'waited by this' count is members waited BY THIS COST");
    }

    /* card_type filter: member_card over a mixed deck */
    {
        TestGame tg;
        test_game_new(&tg);
        int member = test_id(&tg, "PL!-sd1-010-SD");
        int live = test_id(&tg, "PL!-sd1-020-SD");
        RbPlayer *P = &tg.state.p[0];
        test_add_to_deck(&tg, live);
        test_add_to_deck(&tg, member);
        test_add_to_deck(&tg, live);
        AbilityEffect e;
        df_make_draw(&e, 1, "self", "deck", "hand");
        df_set_extra(&e, "card_type", "member_card");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(P->hand.n, 1, "member_card filter draws exactly one member");
        CHECK_EQ(P->hand.n > 0 ? P->hand.cards[0] : -1, member,
                 "member_card filter skipped the live cards on top of the deck");
        CHECK_EQ(P->deck.n, 2, "member_card filter leaves both live cards in the deck");
    }

    /* a different source AND destination: deck_bottom -> deck_bottom moves the
       bottom card back onto the bottom of the same pile */
    {
        TestGame tg;
        test_game_new(&tg);
        int a = test_id(&tg, "PL!-sd1-010-SD");
        int b = df_new_id(&tg, "PL!-sd1-010-SD");
        int c = df_new_id(&tg, "PL!-sd1-010-SD");
        RbPlayer *P = &tg.state.p[0];
        test_add_to_deck(&tg, a);
        test_add_to_deck(&tg, b);
        test_add_to_deck(&tg, c);
        AbilityEffect e;
        df_make_draw(&e, 1, "self", "deck_bottom", "deck_bottom");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(P->hand.n, 0, "a deck_bottom -> deck_bottom draw does not touch the hand");
        CHECK_EQ(P->deck.n, 3, "a deck_bottom -> deck_bottom draw keeps the pile size");
    }

    /* target routing: opponent */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        for (int i = 0; i < 6; i++) test_add_to_deck(&tg, filler);
        for (int i = 0; i < 6; i++) test_add_to_deck_pl(&tg, 1, filler);
        AbilityEffect e;
        df_make_draw(&e, 2, "opponent", "deck", "hand");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(tg.state.p[1].hand.n, 2, "target=opponent draws for player2 only");
        CHECK_EQ(tg.state.p[1].deck.n, 4, "target=opponent draws from player2's deck");
        CHECK_EQ(tg.state.p[0].hand.n, 0, "target=opponent leaves the actor alone");
        CHECK_EQ(tg.state.p[0].deck.n, 6, "target=opponent leaves the actor deck alone");
    }
    /* target routing: both */
    {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        for (int i = 0; i < 6; i++) test_add_to_deck(&tg, filler);
        for (int i = 0; i < 6; i++) test_add_to_deck_pl(&tg, 1, filler);
        AbilityEffect e;
        df_make_draw(&e, 2, "both", "deck", "hand");
        rb_effect_draw_card(&tg.state, 0, &e, -1);
        CHECK_EQ(tg.state.p[0].hand.n, 2, "target=both draws 2 for player1");
        CHECK_EQ(tg.state.p[1].hand.n, 2, "target=both draws 2 for player2");
        CHECK_EQ(tg.state.p[0].deck.n, 4, "target=both draws from player1's deck");
        CHECK_EQ(tg.state.p[1].deck.n, 4, "target=both draws from player2's deck");
        CHECK_EQ(tg.state.last_draw_count, 2,
                 "target=both reports the requested count once, not summed");
    }
}

/* ???????????????????????????????????????????????????????????????????????????
   19. live_success_compare_revealed_draw_test.rs - the two PARSE-SHAPE checks
       that file kept after its behavioural half moved to
       live_success_compare_revealed_draw_behaviour_test.rs. That file's
       comparison cases are not re-derived here: it documents a confirmed
       unsatisfiable-condition finding and asserts on GameState::rule_log,
       which the C engine does not keep.
   ????????????????????????????????????????????????????????????????????????? */
static int df_card_has_ability_text(TestGame *tg, const char *card_no, const char *needle)
{
    int cid = test_id(tg, card_no);
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < n; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = (ab.full_text && strstr(ab.full_text, needle)) != NULL;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int df_card_has_trigger(TestGame *tg, const char *card_no, const char *trig)
{
    int cid = test_id(tg, card_no);
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < n; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        int hit = (ab.triggers && !strcmp(ab.triggers, trig)) != 0;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static void test_success_zone_restriction_and_constant_parsed(void)
{
    TestGame tg;
    test_game_new(&tg);
    CHECK(df_card_has_ability_text(&tg, "PL!-sd1-006-SD", TEXT_SUCCESS_LIVE_CARD),
          "PL!-sd1-006-SD should have the success-live-card-zone restriction ability");
    CHECK(df_card_has_trigger(&tg, "PL!S-bp3-016-N", TRIG_CONSTANT),
          "PL!S-bp3-016-N should have at least one constant ability");
    CHECK(df_card_has_trigger(&tg, "PL!S-bp3-005-R", TRIG_LIVE_SUCCESS),
          "PL!S-bp3-005-R should expose its live-success ability to the resolver");
}

/* Permanent diagnostic: RB_DUMP_TRIGS=1 prints, for every card this cluster
   fires, its card_no, each ability's decoded `triggers` / `full_text`, and
   each draw effect's decoded CONDITION tree. A df_fire_trigger that cannot
   find its trigger string, or a gate in front of a flat draw that fires (or
   does not) when the printed text says otherwise, is then diagnosable without
   a rebuild - and so is the hex-escape table above, which this dump checks
   against the decoder. */
static void df_dump_cond(const Condition *c, int depth)
{
    if (!c) { fprintf(stderr, "%*s(cond = NULL)\n", depth * 2, ""); return; }
    fprintf(stderr, "%*svariant=%u\n", depth * 2, "", c->variant);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        fprintf(stderr, "%*s  %s = tag%u/%lld '%s'\n", depth * 2, "",
                f->key ? f->key : "?", f->v.tag, (long long)f->v.i,
                f->v.s ? f->v.s : "");
        if (f->v.cond) df_dump_cond(f->v.cond, depth + 2);
    }
}

static void df_dump_triggers(void)
{
    static const char *CARDS[] = {
        "PL!HS-bp5-011-N", "PL!HS-bp2-017-N", "PL!S-bp7-002-R",
        "PL!HS-pb1-021-N", "PL!HS-PR-028-PR", "PL!HS-pb1-013-R",
        "PL!SP-bp7-015-N", "PL!SP-bp1-008-R", "PL!-bp3-009-R\xEF\xBC\x8B",
        "PL!-bp4-001-R", "PL!SP-pb1-004-R", "PL!SP-bp5-020-N",
        "PL!-bp3-004-R\xEF\xBC\x8B", "PL!-pb1-008-R", "PL!SP-bp1-009-R",
        "PL!N-pb1-005-R", "PL!SP-bp1-002-R\xEF\xBC\x8B", "PL!SP-bp4-003-R",
        "PL!-pb1-005-R", "PL!S-bp3-016-N", "PL!S-bp3-005-R",
    };
    TestGame tg;
    test_game_new(&tg);
    for (unsigned i = 0; i < sizeof(CARDS) / sizeof(CARDS[0]); i++) {
        int cid = test_id(&tg, CARDS[i]);
        if (cid < 0) { fprintf(stderr, "MISSING %s\n", CARDS[i]); continue; }
        int n = rb_card_num_abilities((uint32_t)cid);
        fprintf(stderr, "== %s id=%d abilities=%d\n", CARDS[i], cid, n);
        for (int a = 0; a < n; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            fprintf(stderr, "   [%d] triggers=[%s] full=[%s]\n", a,
                    ab.triggers ? ab.triggers : "(null)",
                    ab.full_text ? ab.full_text : "(null)");
            if (ab.effect) {
                fprintf(stderr, "       effect action=%s count=%d target=%s src=%s dst=%s\n",
                        ab.effect->action ? ab.effect->action : "-", ab.effect->count,
                        ab.effect->target ? ab.effect->target : "-",
                        ab.effect->source ? ab.effect->source : "-",
                        ab.effect->destination ? ab.effect->destination : "-");
                for (int k = 0; k < ab.effect->n_extra; k++)
                    fprintf(stderr, "         extra[%d] %s = %s\n", k,
                            ab.effect->extra_k[k] ? ab.effect->extra_k[k] : "-",
                            ab.effect->extra_v[k] ? ab.effect->extra_v[k] : "-");
                if (ab.effect->has_condition) df_dump_cond(ab.effect->condition, 4);
            }
            rb_free_ability(&ab);
        }
    }
}

/* Permanent diagnostic: RB_DUMP_ACTIONS=1 shows what the action generator
   offers for the PL!SP-bp1-009-R fixture, so a df_count_use_offers() == 0 can
   be read as "the generator offers nothing" instead of "the filter is wrong". */
static void df_dump_actions(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int natsumi = test_id(&tg, NATSUMI);
    for (int i = 0; i < 40; i++) test_add_to_deck(&tg, filler);
    tg.state.p[0].stage[1] = natsumi;
    test_give_energy(&tg, 2);
    test_add_to_hand(&tg, df_new_id(&tg, "PL!-sd1-010-SD"));
    test_add_to_hand(&tg, df_new_id(&tg, "PL!-sd1-010-SD"));
    RbGeneratedActionList list = rb_generate_action_candidates(&tg.state);
    fprintf(stderr, "[ACTIONS] natsumi=%d generated=%d\n", natsumi, list.count);
    for (int i = 0; i < list.count; i++)
        fprintf(stderr, "   act[%d] type=%d (%s) has_params=%d card_id=%d\n", i,
                list.actions[i].action_type,
                rb_action_type_to_str(list.actions[i].action_type),
                list.actions[i].has_parameters,
                list.actions[i].has_parameters ? list.actions[i].parameters.card_id : -1);
    free(list.actions);
}

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    if (getenv("RB_DUMP_TRIGS")) { df_dump_triggers(); rb_unload(); return 0; }
    if (getenv("RB_DUMP_ACTIONS")) { df_dump_actions(); rb_unload(); return 0; }

    test_debut_draws_one_pl_hs_bp5_011_n();                  /*  1 */
    test_waitroom_count_gated_debut_draw();                  /*  2 */
    test_costly_aqours_gated_debut_draw();                   /*  3 */
    test_dollchestra_live_zone_gated_draw();                 /*  4 */
    test_boosted_member_live_success_draw();                 /*  5 */
    test_higher_cost_member_live_success_draw();             /*  6 */
    test_sumire_optional_energy_catchu_trio_draw();          /*  7 */
    test_stage_member_identity_and_cost_gated_draw();        /*  8 */
    test_live_success_optional_three_energy_draw();          /*  9 */
    test_optional_energy_live_success_draw();                /* 10 */
    test_per_own_stage_member_draw_then_discard_q146();      /* 11 */
    test_optional_wait_members_draw_per_cost_member_q183();  /* 12 */
    test_turn1_energy_draw_discard_natsumi();                /* 13 */
    test_miyashita_ai_cost10_appears_draw();                 /* 14 */
    test_position_gated_draw();                              /* 15 */
    test_success_zone_gated_debut_draw();                    /* 16 */
    test_distinct_draw_regression();                         /* 17 */
    test_flat_draw_engine_surface();                         /* 18 */
    test_success_zone_restriction_and_constant_parsed();     /* 19 */

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d of %d check(s) failed\n", failures, checks);
        return 1;
    }
    printf("all %d draw-flat parity checks passed\n", checks);
    return 0;
}

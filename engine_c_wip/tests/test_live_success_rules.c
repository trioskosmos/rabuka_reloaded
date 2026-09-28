#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(c,msg) do { if (!(c)) { fprintf(stderr, "FAIL: %s\n", msg); failures++; } else printf("ok: %s\n", msg); } while (0)
#define CHECK_EQ(a,b,msg) do { int aa=(a), bb=(b); if (aa != bb) { fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, aa, bb); failures++; } else printf("ok: %s\n", msg); } while (0)

static void fill_decks(TestGame *tg, int filler, int p1_count, int p2_count) {
    for (int i = 0; i < p1_count; i++) test_add_to_deck(tg, filler);
    for (int i = 0; i < p2_count; i++) test_add_to_deck_pl(tg, 1, filler);
}

static void clear_choices(TestGame *tg) {
    for (int i = 0; i < 100 && test_has_pending_choice(tg); i++) {
        test_resume_choice(tg, 0);
        rb_drain_ability_queue(&tg->state);
    }
}

static void stage_member(TestGame *tg, int pl, int area, int card) {
    if (pl == 0) test_add_to_stage(tg, area, card);
    else test_set_opp_stage(tg, area, card);
}

static void run_performance(TestGame *tg) {
    tg->state.live_batch_mode = 1;
    rb_perform_live(&tg->state, 0);
    rb_perform_live(&tg->state, 1);
    tg->state.live_batch_mode = 0;
    rb_execute_live_victory_determination(&tg->state);
    if (test_has_pending_choice(tg) && strcmp(test_pending_choice_type(tg), "SelectTarget") != 0) {
        clear_choices(tg);
    }
    if (!test_has_pending_choice(tg) && tg->state.p[0].live.n > 0) {
        int p1 = 0, p2 = 0;
        rb_determine_live_winners(&tg->state, &p1, &p2);
        rb_process_player_live_result(&tg->state, 0, p1, 0, p1);
        rb_process_player_live_result(&tg->state, 1, p2, 0, p2);
    }
}

static void test_insufficient_hearts_failure(void) {
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    CHECK(rb_card_is_live(live), "insufficient-heart fixture is a live card");
    test_add_to_live(&tg, live);
    run_performance(&tg);
    CHECK(test_zone_has_id(&tg, 0, "discard", live), "insufficient hearts move live to waitroom");
    CHECK_EQ(tg.state.p[0].success.n, 0, "insufficient hearts do not place live");
}

static void test_both_player_score_comparison_and_tie(void) {
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int member = test_id(&tg, "PL!-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 3; i++) stage_member(&tg, 0, i, member);
    for (int i = 0; i < 3; i++) stage_member(&tg, 1, i, member);
    test_add_to_live(&tg, live);
    fill_decks(&tg, filler, 50, 50);
    run_performance(&tg);
    CHECK_EQ(tg.state.p[0].success.n, 1, "first attacker wins equal-score comparison");
    CHECK_EQ(tg.state.p[1].success.n, 0, "second attacker does not win equal-score comparison");

    test_game_new(&tg);
    live = test_id(&tg, "PL!-sd1-019-SD");
    member = test_id(&tg, "PL!-sd1-001-SD");
    filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 3; i++) stage_member(&tg, 0, i, member);
    for (int i = 0; i < 3; i++) stage_member(&tg, 1, i, member);
    test_add_to_live(&tg, live);
    test_add_to_opp_live(&tg, live);
    fill_decks(&tg, filler, 50, 50);
    run_performance(&tg);
    CHECK_EQ(tg.state.p[0].success.n, 1, "tie places first attacker's live");
    CHECK_EQ(tg.state.p[1].success.n, 1, "tie places second attacker's live");
}

static void test_score_zero_success_and_single_live(void) {
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    int member = test_id(&tg, "PL!-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 3; i++) stage_member(&tg, 0, i, member);
    test_add_to_live(&tg, live);
    fill_decks(&tg, filler, 50, 20);
    run_performance(&tg);
    CHECK(test_has_pending_choice(&tg) || test_zone_has_id(&tg, 0, "success", live), "single live can succeed without opponent");
    clear_choices(&tg);
    rb_execute_live_victory_determination(&tg.state);
    CHECK_EQ(tg.state.p[0].success.n, 1, "single live auto-win reaches success zone");
    CHECK_EQ(tg.state.p[1].success.n, 0, "single live leaves opponent success empty");
}

static void test_no_live_no_success(void) {
    TestGame tg; test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int member = test_id(&tg, "PL!S-sd1-003-SD");
    stage_member(&tg, 0, 0, member);
    fill_decks(&tg, filler, 50, 20);
    run_performance(&tg);
    CHECK(tg.state.p[0].deck.n >= 45, "no live does not perform a yell");
    CHECK_EQ(tg.state.p[0].success.n + tg.state.p[1].success.n, 0, "no live produces no success");
}

static void test_multiple_live_all_fail(void) {
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!-sd1-019-SD");
    stage_member(&tg, 0, 0, test_id(&tg, "PL!S-sd1-003-SD"));
    test_add_to_live(&tg, live);
    test_add_to_live(&tg, live);
    run_performance(&tg);
    CHECK_EQ(tg.state.p[0].success.n, 0, "failed live batch does not place any card");
    CHECK_EQ(tg.state.p[0].discard.n >= 2, 1, "all failed live cards go to waitroom");
}

static void test_winner_placement_and_second_selection(void) {
    for (int selected = 0; selected < 2; selected++) {
        TestGame tg; test_game_new(&tg);
        int rin = test_id(&tg, "PL!-sd1-014-SD");
        int hanayo = test_id(&tg, "PL!-sd1-017-SD");
        int live = test_id(&tg, "PL!HS-bp1-019-L");
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        fill_decks(&tg, filler, 30, 10);
        test_add_to_hand(&tg, rin);
        test_add_to_hand(&tg, hanayo);
        test_add_to_hand(&tg, live);
        test_add_to_hand(&tg, live);
        test_give_energy(&tg, 18);
        test_play_to_stage(&tg, rin, 1);
        test_play_to_stage(&tg, hanayo, 0);
        test_set_live_card(&tg, 0, live);
        test_set_live_card(&tg, 1, live);
        run_performance(&tg);
        if (test_has_pending_choice(&tg)) test_resume_choice(&tg, selected);
        CHECK_EQ(tg.state.p[0].success.n, 1, "winner places only one live");
        CHECK_EQ(tg.state.p[0].live.n, 0, "winner live zone is empty after selection");
        CHECK_EQ(tg.state.p[0].discard.n >= 1, 1, "unselected live goes to waitroom");
    }
}

static void test_success_zone_legality_and_selection(void) {
    TestGame tg;
    test_game_new(&tg);
    int restricted = test_id(&tg, "PL!S-bp2-024-L");
    int legal = test_id(&tg, "PL!HS-bp1-019-L");
    CHECK(rb_card_is_live(restricted), "restricted success fixture is a live card");
    CHECK(rb_card_is_live(legal), "legal success fixture is a live card");
    test_add_to_live(&tg, restricted);
    test_add_to_live(&tg, legal);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);

    const RbChoice *choice = rb_get_pending_choice(&tg.state);
    CHECK(choice != NULL, "winner chooses among multiple live cards");
    CHECK(choice && choice->n_filtered_indices == 1,
          "success choice excludes the restricted live card");
    CHECK(choice && choice->n_filtered_indices == 1 && choice->filtered_indices[0] == 1,
          "success choice maps to the legal live card");
    if (choice) test_resume_choice(&tg, 0);

    CHECK_EQ(tg.state.p[0].success.n, 1, "legal live reaches the success zone");
    CHECK(tg.state.p[0].success.n == 1 && tg.state.p[0].success.cards[0] == legal,
          "selected legal live is the success-zone card");
    CHECK(test_zone_has_id(&tg, 0, "discard", restricted),
          "unselected restricted live goes to the waitroom");
    CHECK_EQ(tg.state.p[0].live.n, 0, "live zone is empty after success choice");
}

static void test_single_restricted_live_goes_to_waitroom(void) {
    TestGame tg;
    test_game_new(&tg);
    int restricted = test_id(&tg, "PL!S-bp2-024-L");
    test_add_to_live(&tg, restricted);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);

    CHECK(!test_has_pending_choice(&tg), "single restricted live needs no choice");
    CHECK_EQ(tg.state.p[0].success.n, 0,
             "single restricted live cannot enter the success zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", restricted),
          "single restricted live goes to the waitroom");
}

static void test_success_choice_resumes_second_winner(void) {
    TestGame tg;
    test_game_new(&tg);
    int restricted = test_id(&tg, "PL!S-bp2-024-L");
    int legal = test_id(&tg, "PL!HS-bp1-019-L");
    test_add_to_live(&tg, restricted);
    test_add_to_live(&tg, legal);
    test_add_to_opp_live(&tg, restricted);
    test_add_to_opp_live(&tg, legal);
    tg.state.p1_live_won = 1;
    tg.state.p2_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    const RbChoice *choice = rb_get_pending_choice(&tg.state);
    CHECK(choice && choice->actor == 0, "first winner receives the success choice");
    if (choice) test_resume_choice(&tg, 0);

    tg.state.live_victory_stage = 3;
    rb_execute_live_victory_determination(&tg.state);
    choice = rb_get_pending_choice(&tg.state);
    CHECK(choice && choice->actor == 1, "victory resume offers the second winner choice");
    if (choice) test_resume_choice(&tg, 0);
    rb_execute_live_victory_determination(&tg.state);

    CHECK_EQ(tg.state.p[0].success.n, 1, "first winner places one legal live");
    CHECK_EQ(tg.state.p[1].success.n, 1, "second winner places one legal live");
    CHECK(tg.state.p[0].success.n == 1 && tg.state.p[0].success.cards[0] == legal,
          "first winner stores the legal live");
    CHECK(tg.state.p[1].success.n == 1 && tg.state.p[1].success.cards[0] == legal,
          "second winner stores the legal live");
    CHECK(test_zone_has_id(&tg, 0, "discard", restricted) &&
          test_zone_has_id(&tg, 1, "discard", restricted),
          "both restricted live cards go to the waitroom");
    CHECK_EQ(tg.state.live_victory_stage, 0, "victory placement resumes to completion");
}

static void test_success_replacement_choice(int accept) {
    TestGame tg;
    test_game_new(&tg);
    int original = test_id(&tg, "PL!-bp6-024-L");
    int other = test_id(&tg, "PL!S-bp2-024-L");
    int replacement = test_id(&tg, "PL!HS-bp1-019-L");
    CHECK(rb_card_is_live(original), "replacement original is a live card");
    CHECK(rb_card_is_live(replacement), "replacement candidate is a live card");
    test_add_to_live(&tg, original);
    test_add_to_discard(&tg, other);
    test_add_to_discard(&tg, replacement);
    tg.state.p1_live_won = 1;

    rb_move_live_to_success_and_handle_wins(&tg.state);
    const RbChoice *choice = rb_get_pending_choice(&tg.state);
    CHECK(choice != NULL, "success replacement prompts for a waitroom live");
    CHECK(choice && choice->kind == RB_CHOICE_SELECT_CARD &&
          !strcmp(choice->zone, "discard") &&
          !strcmp(choice->target, "success_replacement") && choice->allow_skip,
          "success replacement exposes an optional waitroom choice");
    CHECK(choice && choice->n_filtered_indices == 1 &&
          choice->filtered_indices[0] == 1,
          "success replacement filters and maps the physical waitroom index");
    if (choice) test_resume_choice(&tg, accept ? 0 : -1);

    CHECK(!test_has_pending_choice(&tg), "success replacement resolves its prompt");
    CHECK_EQ(tg.state.p[0].live.n, 0, "success replacement empties the live zone");
    if (accept) {
        CHECK(test_zone_has_id(&tg, 0, "success", replacement),
              "selected replacement enters the success zone");
        CHECK(test_zone_has_id(&tg, 0, "discard", original),
              "original live enters the waitroom after replacement");
    } else {
        CHECK(test_zone_has_id(&tg, 0, "success", original),
              "skipping replacement places the original live");
        CHECK(test_zone_has_id(&tg, 0, "discard", replacement),
              "skipped replacement stays in the waitroom");
    }
}

static void test_daydream_mermaid_choices(void) {
    for (int with_niji = 0; with_niji <= 1; with_niji++) {
        TestGame tg; test_game_new(&tg);
        int live = test_id(&tg, "PL!N-bp4-030-L");
        int h05 = test_id(&tg, "PL!S-bp2-015-PR");
        int h06a = test_id(&tg, "PL!-sd1-003-SD");
        int h06b = test_id(&tg, "PL!-sd1-001-SD");
        int niji = test_id(&tg, "PL!N-bp1-025-L");
        int target = test_id(&tg, "PL!-sd1-001-SD");
        int energy = test_id(&tg, "LL-E-001-SD");
        tg.state.p[0].stage[0] = h05;
        tg.state.p[0].stage[1] = h06a;
        tg.state.p[0].stage[2] = h06b;
        test_add_to_live(&tg, live);
        rb_waitroom_add(&tg.state.p[0], target);
        if (with_niji) test_add_to_success(&tg, niji);
        for (int i = 0; i < 10; i++) test_add_to_energy_deck(&tg, 0, energy);
        fill_decks(&tg, h05, 40, 20);
        run_performance(&tg);
        CHECK(test_has_pending_choice(&tg), with_niji ? "Mermaid with niji prompts choice" : "Mermaid without niji prompts choice");
        /* Options reach the player in PRINTED order, as Rust keeps them
           (cards/abilities.json PL!N-bp4-030-L):
             index 0 「自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く」
             index 1 「自分の控え室からメンバーカードを1枚手札に加える」
           The any-number (niji) branch stays on 0 — it only needs SOME pick and
           then asserts the energy branch placed a card. The no-niji branch
           must pick the RECOVER option, which is printed second:
           engine/tests/.../live_success_rules_test.rs:494
             game.select_option(1); // Pick option 1 (recover)
           and daydream_mermaid_live_success_energy_or_recover_choice_test.rs:69
           select_option(0) -> energy, :112 select_option(1) -> recover. */
        if (test_has_pending_choice(&tg)) test_resume_choice(&tg, with_niji ? 0 : 1);
        if (with_niji) {
            CHECK(test_has_pending_choice(&tg), "Mermaid recovery target selection appears");
            if (test_has_pending_choice(&tg)) test_resume_choice(&tg, 0);
            CHECK(test_has_pending_choice(&tg), "Mermaid any-number reprompt appears");
            if (test_has_pending_choice(&tg)) test_resume_choice(&tg, 0);
        } else {
            clear_choices(&tg);
        }
        CHECK(test_hand_has(&tg, target), with_niji ? "Mermaid recovers member in any-number flow" : "Mermaid recovers member in one-choice flow");
        if (with_niji) CHECK(tg.state.p[0].energy.n > 0, "Mermaid energy option places energy");
    }
}

static void test_shared_heart_pool(void) {
    for (int count = 1; count <= 3; count++) {
        TestGame tg; test_game_new(&tg);
        int energy = test_id(&tg, "LL-E-001-SD");
        int rin = test_id(&tg, "PL!-sd1-014-SD");
        int live = test_id(&tg, "PL!HS-bp1-019-L");
        int member = test_id(&tg, "PL!-sd1-014-SD");
        fill_decks(&tg, member, 0, 0);
        test_add_to_energy_deck(&tg, 1, energy);
        test_add_to_hand(&tg, rin);
        for (int i = 0; i < count; i++) test_add_to_hand(&tg, live);
        test_give_energy(&tg, 9);
        test_play_to_stage(&tg, rin, 1);
        for (int i = 0; i < count; i++) test_set_live_card(&tg, i, live);
        run_performance(&tg);
        if (count == 1) {
            CHECK_EQ(tg.state.p[0].success.n, 1, "shared pool satisfies one live");
        } else {
            CHECK_EQ(tg.state.p[0].success.n, 0, "shared pool depletion fails all lives");
            CHECK(tg.state.p[0].discard.n >= count, "depleted shared pool sends all lives to waitroom");
        }
        CHECK_EQ(tg.state.p[1].success.n, 0, "opponent does not place in shared-pool scenario");
    }
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_insufficient_hearts_failure();
    test_both_player_score_comparison_and_tie();
    test_score_zero_success_and_single_live();
    test_no_live_no_success();
    test_multiple_live_all_fail();
    test_winner_placement_and_second_selection();
    test_success_zone_legality_and_selection();
    test_single_restricted_live_goes_to_waitroom();
    test_success_choice_resumes_second_winner();
    test_success_replacement_choice(1);
    test_success_replacement_choice(0);
    test_daydream_mermaid_choices();
    test_shared_heart_pool();
    rb_unload();
    if (failures) return 1;
    printf("ALL LIVE SUCCESS RULE CHECKS PASSED\n");
    return 0;
}

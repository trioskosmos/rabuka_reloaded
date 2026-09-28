use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_state::Phase;

fn advance_to_first_live_card_set(game: &mut TestGame) {
    let mut guard = 0;
    while game.state.current_phase != Phase::LiveCardSetFirstAttacker {
        guard += 1;
        assert!(guard < 20, "did not reach the first live-card set phase");
        game.pass();
        // Answer what cannot be declined, decline what can: a mandatory
        // prompt left parked mid-ability produces exactly the absence
        // the next assertion checks, so nothing fails.
        game.drain_choices();
    }
}

fn choose_deck_order(game: &mut TestGame, choices: &[i16]) {
    for &choice in choices {
        assert!(
            matches!(
                game.get_pending_choice(),
                Choice::SelectTarget { target, .. } if target == "order"
            ),
            "NEO SKY must offer the printed deck-order choice: {:?}",
            game.get_pending_choice()
        );
        game.select_option(choice);
    }
}

#[test]
fn neo_sky_live_start_draws_three_and_places_them_in_chosen_order() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id("PL!N-bp4-031-L");
    let n13 = g.id("PL!N-bp4-022-N");
    let n4 = g.id("PL!N-bp4-014-N");
    let n5 = g.id("PL!N-bp4-015-N");
    let draw_a = g.id("PL!N-bp1-026-L");
    let draw_b = g.id("PL!SP-bp1-023-L");
    let draw_c = g.id("PL!-sd1-010-SD");
    let remainder = g.id("PL!S-sd1-001-SD");

    g.state.player1.stage.stage = [n13, n4, n5];
    fill_both_main_decks(&mut g, remainder);
    advance_to_first_live_card_set(&mut g);
    g.state.player1.hand.cards.push(live);
    g.set_live_card(live);
    g.pass();
    assert_eq!(g.state.current_phase, Phase::LiveCardSetSecondAttacker);
    g.state.player1.hand.cards.clear();
    g.state.player1.main_deck.cards.clear();
    g.state.player1.main_deck.cards.push(draw_a);
    g.state.player1.main_deck.cards.push(draw_b);
    g.state.player1.main_deck.cards.push(draw_c);
    g.state.player1.main_deck.cards.push(remainder);

    let p2_live = g.id("PL!-sd1-020-SD");
    g.state.player2.hand.cards.push(p2_live);
    g.set_live_card(p2_live);
    g.pass();

    choose_deck_order(&mut g, &[1, 0]);

    assert!(g.state.player1.hand.cards.is_empty());
    assert_eq!(
        g.state.player1.main_deck.cards.as_slice(),
        &[draw_b, draw_a, draw_c, remainder],
        "NEO SKY must place the chosen cards above the untouched deck suffix"
    );
}

#[test]
fn neo_sky_live_start_selects_three_from_larger_hand_before_ordering() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id("PL!N-bp4-031-L");
    let n15 = g.id("PL!N-bp4-022-N");
    let n4 = g.id("PL!N-bp4-014-N");
    let n5 = g.id("PL!N-bp4-015-N");
    let extra = g.id("PL!S-sd1-001-SD");
    let draw_a = g.id("PL!N-bp1-026-L");
    let draw_b = g.id("PL!SP-bp1-023-L");
    let draw_c = g.id("PL!-sd1-010-SD");
    let remainder = g.id("PL!S-bp7-001-R");

    g.state.player1.stage.stage = [n15, n4, n5];
    fill_both_main_decks(&mut g, remainder);
    advance_to_first_live_card_set(&mut g);
    g.state.player1.hand.cards.push(live);
    g.set_live_card(live);
    g.pass();
    assert_eq!(g.state.current_phase, Phase::LiveCardSetSecondAttacker);
    g.state.player1.hand.cards.clear();
    g.state.player1.hand.cards.push(extra);
    g.state.player1.main_deck.cards.clear();
    g.state.player1.main_deck.cards.push(draw_a);
    g.state.player1.main_deck.cards.push(draw_b);
    g.state.player1.main_deck.cards.push(draw_c);
    g.state.player1.main_deck.cards.push(remainder);

    let p2_live = g.id("PL!-sd1-020-SD");
    g.state.player2.hand.cards.push(p2_live);
    g.set_live_card(p2_live);
    g.pass();

    assert!(
        matches!(
            g.get_pending_choice(),
            Choice::SelectCard { zone, count: 3, .. } if zone == "hand"
        ),
        "NEO SKY must select three cards when four are in hand: {:?}",
        g.get_pending_choice()
    );
    g.select_indices(&[0, 1, 2]);
    choose_deck_order(&mut g, &[1, 0]);

    assert_eq!(g.state.player1.hand.cards.as_slice(), &[draw_c]);
    assert_eq!(
        g.state.player1.main_deck.cards.as_slice(),
        &[draw_a, extra, draw_b, remainder],
        "selected cards must use the chosen order above the untouched suffix"
    );
}

#[test]
fn neo_sky_live_start_does_not_fire_at_total_cost_nineteen() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id("PL!N-bp4-031-L");
    let n13 = g.id("PL!N-bp7-002-R");
    let n4 = g.id("PL!N-bp4-014-N");
    let n2 = g.id("PL!N-bp4-020-N");
    let sentinel_a = g.id("PL!N-bp1-026-L");
    let sentinel_b = g.id("PL!SP-bp1-023-L");

    g.state.player1.stage.stage = [n13, n4, n2];
    fill_both_main_decks(&mut g, sentinel_a);
    advance_to_first_live_card_set(&mut g);
    g.state.player1.hand.cards.push(live);
    g.set_live_card(live);
    g.pass();
    assert_eq!(g.state.current_phase, Phase::LiveCardSetSecondAttacker);
    g.state.player1.hand.cards.clear();
    g.state.player1.main_deck.cards.clear();
    g.state.player1.main_deck.cards.push(sentinel_a);
    g.state.player1.main_deck.cards.push(sentinel_b);

    let p2_live = g.id("PL!-sd1-020-SD");
    g.state.player2.hand.cards.push(p2_live);
    g.set_live_card(p2_live);
    g.pass();

    assert!(g.state.player1.hand.cards.is_empty());
    assert_eq!(
        g.state.player1.main_deck.cards.as_slice(),
        &[sentinel_a, sentinel_b],
        "cost total 19 must not draw or place cards"
    );
}

#[test]
fn neo_sky_live_start_requires_every_area_to_be_nijigasaki() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id("PL!N-bp4-031-L");
    let n13 = g.id("PL!N-bp4-022-N");
    let n4 = g.id("PL!N-bp4-014-N");
    let non_niji = g.id("PL!S-sd1-001-SD");
    let sentinel_a = g.id("PL!N-bp1-026-L");
    let sentinel_b = g.id("PL!SP-bp1-023-L");

    g.state.player1.stage.stage = [n13, n4, non_niji];
    fill_both_main_decks(&mut g, sentinel_a);
    advance_to_first_live_card_set(&mut g);
    g.state.player1.hand.cards.push(live);
    g.set_live_card(live);
    g.pass();
    assert_eq!(g.state.current_phase, Phase::LiveCardSetSecondAttacker);
    g.state.player1.hand.cards.clear();
    g.state.player1.main_deck.cards.clear();
    g.state.player1.main_deck.cards.push(sentinel_a);
    g.state.player1.main_deck.cards.push(sentinel_b);

    let p2_live = g.id("PL!-sd1-020-SD");
    g.state.player2.hand.cards.push(p2_live);
    g.set_live_card(p2_live);
    g.pass();

    assert!(g.state.player1.hand.cards.is_empty());
    assert_eq!(
        g.state.player1.main_deck.cards.as_slice(),
        &[sentinel_a, sentinel_b],
        "a non- Nijigasaki member must block the LiveStart effect"
    );
}

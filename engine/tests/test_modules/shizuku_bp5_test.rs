use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

#[test]
fn shizuku_bp5_dynamic_energy_payment_tests() {
    let db = load_real_database();

    // Find a live card with score 2
    let live_score_2_no = db
        .cards
        .values()
        .find(|c| c.card_type == rabuka_engine::core::card::CardType::Live && c.score == Some(2))
        .expect("should find a live card with score 2")
        .card_no
        .clone();

    // Find a live card with score 0
    let live_score_0_no = db
        .cards
        .values()
        .find(|c| c.card_type == rabuka_engine::core::card::CardType::Live && c.score == Some(0))
        .expect("should find a live card with score 0")
        .card_no
        .clone();

    // 1. Test case: Normal flow (Live card score = 2, Player has enough energy)
    {
        let mut game = TestGame::new(db.clone());
        let shizuku = game.id("PL!N-bp5-003-R");
        let discard_live = game.id(&live_score_2_no);
        let hand_cost_filler = game.id("PL!-sd1-010-SD");
        let filler = game.id("PL!-sd1-010-SD");

        game.state.player1.stage.stage = [filler, shizuku, filler];
        game.state.player1.hand.cards.push(hand_cost_filler);
        game.state.player1.waitroom.cards.push(discard_live);
        game.give_energy(5);

        // Activate Shizuku (ab#0) from stage (it's a 起動 ability)
        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::UseAbility,
            Some(shizuku),
            None,
            None,
            None,
        )
        .expect("activate Shizuku's ability");

        // Cost phase: Select hand card to discard
        assert!(
            game.has_pending_choice(),
            "should prompt to select hand card for cost"
        );
        game.select_indices(&[0]); // Discard hand_cost_filler

        // Select live card from waitroom
        assert!(
            game.has_pending_choice(),
            "should prompt to select live card from waitroom"
        );
        game.select_indices(&[0]); // Select discard_live

        // Optional payment prompt: Pay 2 energy?
        assert!(game.has_pending_choice(), "should prompt to pay 2 energy");
        game.select_option(1); // Select Option 1 (Yes/Confirm)

        // Verify state
        assert_eq!(
            game.state.player1.energy_zone.active_energy_count, 3,
            "should pay 2 energy (5 -> 3)"
        );
        assert!(
            game.state.player1.hand.cards.contains(&discard_live),
            "live card should be moved to hand"
        );
        assert!(
            !game.state.player1.waitroom.cards.contains(&discard_live),
            "live card should no longer be in discard"
        );
    }

    // 2. Test case: Q214 (Live card score = 0, energy paid = 0)
    {
        let mut game = TestGame::new(db.clone());
        let shizuku = game.id("PL!N-bp5-003-R");
        let discard_live = game.id(&live_score_0_no);
        let hand_cost_filler = game.id("PL!-sd1-010-SD");
        let filler = game.id("PL!-sd1-010-SD");

        game.state.player1.stage.stage = [filler, shizuku, filler];
        game.state.player1.hand.cards.push(hand_cost_filler);
        game.state.player1.waitroom.cards.push(discard_live);
        game.give_energy(1); // Player has 1 energy

        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::UseAbility,
            Some(shizuku),
            None,
            None,
            None,
        )
        .expect("activate Shizuku's ability");

        // Cost phase: Select hand card to discard
        game.select_indices(&[0]); // Discard hand_cost_filler

        // Select live card from waitroom
        game.select_indices(&[0]); // Select discard_live

        // Optional payment prompt: Pay 0 energy?
        assert!(game.has_pending_choice(), "should prompt to pay 0 energy");
        game.select_option(1); // Select Option 1 (Yes/Confirm)

        // Verify state
        assert_eq!(
            game.state.player1.energy_zone.active_energy_count, 1,
            "should pay 0 energy (1 -> 1)"
        );
        assert!(
            game.state.player1.hand.cards.contains(&discard_live),
            "live card should be moved to hand"
        );
        assert!(
            !game.state.player1.waitroom.cards.contains(&discard_live),
            "live card should no longer be in discard"
        );
    }

    // 3. Test case: Insufficient energy (Live card score = 2, Player has 1 energy, choice is skipped or fails)
    {
        let mut game = TestGame::new(db.clone());
        let shizuku = game.id("PL!N-bp5-003-R");
        let discard_live = game.id(&live_score_2_no);
        let hand_cost_filler = game.id("PL!-sd1-010-SD");
        let filler = game.id("PL!-sd1-010-SD");

        game.state.player1.stage.stage = [filler, shizuku, filler];
        game.state.player1.hand.cards.push(hand_cost_filler);
        game.state.player1.waitroom.cards.push(discard_live);
        game.give_energy(1); // Player only has 1 energy, needs 2

        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::UseAbility,
            Some(shizuku),
            None,
            None,
            None,
        )
        .expect("activate Shizuku's ability");

        // Cost phase: Select hand card to discard
        game.select_indices(&[0]); // Discard hand_cost_filler

        // Select live card from waitroom
        game.select_indices(&[0]); // Select discard_live

        // Should NOT prompt to pay energy since player has insufficient energy.
        // It should have skipped/completed immediately.
        assert!(
            !game.has_pending_choice(),
            "should not have pending choice due to insufficient energy"
        );

        // Verify state: Live card is NOT in hand, still in waitroom
        assert_eq!(
            game.state.player1.energy_zone.active_energy_count, 1,
            "energy should remain unchanged"
        );
        assert!(
            !game.state.player1.hand.cards.contains(&discard_live),
            "live card should NOT be moved to hand"
        );
        assert!(
            game.state.player1.waitroom.cards.contains(&discard_live),
            "live card should still be in discard"
        );
    }

    // 4. Test case: Decline optional payment (Live card score = 2, Player has enough energy but declines)
    {
        let mut game = TestGame::new(db);
        let shizuku = game.id("PL!N-bp5-003-R");
        let discard_live = game.id(&live_score_2_no);
        let hand_cost_filler = game.id("PL!-sd1-010-SD");
        let filler = game.id("PL!-sd1-010-SD");

        game.state.player1.stage.stage = [filler, shizuku, filler];
        game.state.player1.hand.cards.push(hand_cost_filler);
        game.state.player1.waitroom.cards.push(discard_live);
        game.give_energy(5);

        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::UseAbility,
            Some(shizuku),
            None,
            None,
            None,
        )
        .expect("activate Shizuku's ability");

        // Cost phase: Select hand card to discard
        game.select_indices(&[0]); // Discard hand_cost_filler

        // Select live card from waitroom
        game.select_indices(&[0]); // Select discard_live

        // Optional payment prompt: Pay 2 energy?
        assert!(game.has_pending_choice(), "should prompt to pay 2 energy");
        game.select_option(0); // Select Option 0 (No/Decline)

        // Verify state: Live card is NOT in hand, still in waitroom
        assert_eq!(
            game.state.player1.energy_zone.active_energy_count, 5,
            "energy should remain unchanged"
        );
        assert!(
            !game.state.player1.hand.cards.contains(&discard_live),
            "live card should NOT be moved to hand"
        );
        assert!(
            game.state.player1.waitroom.cards.contains(&discard_live),
            "live card should still be in discard"
        );
    }
}

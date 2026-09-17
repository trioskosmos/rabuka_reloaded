use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn lower_stage_cost_draws_two_then_topdecks_one_pl_n_bp4_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let opp_center = game.id("PL!-bp5-008-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin);
    game.state.player2.stage.stage[1] = opp_center;
    game.state.player2.stage.stage[0] = game.new_id(FILLER);

    let keep = game.id("PL!N-PR-019-PR");
    let sacrifice = game.new_id(FILLER);
    game.add_to_hand(keep);
    game.add_to_hand(sacrifice);

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(game.has_pending_choice(), "put-back choice must be offered");
    game.assert_select_card("hand", 1, false);

    let idx = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|&c| c == sacrifice)
        .expect("drawn cards joined the hand before the put-back choice");
    game.select_indices(&[idx]);

    assert_eq!(
        game.state.player1.main_deck.cards.first().copied(),
        Some(sacrifice),
        "chosen card sits at deck index 0 (TOP)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2 + 2 - 1,
        "drew 2, returned exactly 1 to the deck"
    );
    assert!(game.state.player1.hand.cards.contains(&keep));
}

#[test]
fn equal_stage_cost_skips_draw_and_topdeck_pl_n_bp4_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let opp_equal = game.id("PL!-bp5-008-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin);
    game.state.player2.stage.stage[1] = opp_equal;
    game.add_to_hand(game.id("PL!N-PR-019-PR"));
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        !game.has_pending_choice(),
        "equal totals: strict < must not fire"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand_before, "no draw");
}

#[test]
fn both_empty_stages_skip_draw_and_topdeck_pl_n_bp4_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        !game.has_pending_choice(),
        "0 vs 0 → equal → strict < must not fire"
    );
}

#[test]
fn higher_own_stage_cost_skips_draw_and_topdeck_pl_n_bp4_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let cheap = game.id(FILLER);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::Center, rin);
    game.state.player2.stage.stage[1] = cheap;
    game.add_to_hand(game.id("PL!N-PR-019-PR"));
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(!game.has_pending_choice(), "higher own total → no fire");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no draws happened"
    );
}

#[test]
fn empty_own_stage_counts_as_zero_for_draw_and_topdeck_pl_n_bp4_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp4-009-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.state.player2.stage.stage[1] = game.new_id("PL!S-sd1-001-SD");
    game.add_to_hand(filler);

    fire_trigger(&mut game, rin, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "0 < 17 → condition met even with an empty own stage"
    );
    assert!(
        game.pending_choice_type().is_some(),
        "put-back prompt must carry a choice identity"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.main_deck.cards.first().copied(),
        Some(filler),
        "put-back landed on top"
    );
}

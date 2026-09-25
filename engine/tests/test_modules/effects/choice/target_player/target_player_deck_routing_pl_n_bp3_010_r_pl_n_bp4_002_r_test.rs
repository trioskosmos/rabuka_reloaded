use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";
const FILLER_LIVE: &str = "PL!-sd1-019-SD";
const AZUNA_TARGET: &str = "PL!N-bp3-010-R";
const AIKO: &str = "PL!N-bp4-002-R";

fn fill_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn drain_auto(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    drain_auto(game);
}

fn trigger_live_start_with(game: &mut TestGame, filler_live: i16) {
    game.state.player1.hand.cards.push(filler_live);
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    advance_to_live_set(game);
    game.set_live_card(filler_live);
    advance_to_live_start(game);
}

#[test]
fn pl_n_bp3_010_r_choose_self_recycle_leaves_at_least_two_deck_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(AZUNA_TARGET);
    let m1 = game.id(FILLER);
    let m2 = game.id(FILLER);
    game.state.player1.waitroom.cards.push(m1);
    game.state.player1.waitroom.cards.push(m2);
    game.state.player1.stage.stage[1] = member;
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);
    trigger_live_start_with(&mut game, filler_live);
    while game.has_pending_choice() {
        let choice = game.get_pending_choice();
        let is_target = matches!(
            choice,
            rabuka_engine::ability::types::Choice::SelectTarget { .. }
        );
        if is_target {
            game.select_option(0);
        } else {
            game.select_indices(&[0]);
        }
    }
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after >= 2,
        "deck should have gained cards after choosing self, deck_size={}",
        deck_after
    );
}

#[test]
fn pl_n_bp3_010_r_choose_opponent_recycle_empties_opponent_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(AZUNA_TARGET);
    let m3 = game.id(FILLER);
    let m4 = game.id(FILLER);
    game.state.player2.waitroom.cards.push(m3);
    game.state.player2.waitroom.cards.push(m4);
    game.state.player1.stage.stage[1] = member;
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);
    trigger_live_start_with(&mut game, filler_live);
    while game.has_pending_choice() {
        let choice = game.get_pending_choice();
        let is_target = matches!(
            choice,
            rabuka_engine::ability::types::Choice::SelectTarget { .. }
        );
        if is_target {
            game.select_option(1);
        } else {
            game.select_indices(&[0, 1]);
        }
    }
    let deck_after = game.state.player2.main_deck.cards.len();
    assert!(
        deck_after >= 2,
        "P2's deck should have gained cards after choosing opponent, deck_size={}",
        deck_after
    );
    assert_eq!(game.state.player2.waitroom.cards.len(), 0);
}

#[test]
fn pl_n_bp4_002_r_choose_self_discards_looked_top_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(AIKO);
    let top_card = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(top_card);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player1.stage.stage[1] = member;
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);
    trigger_live_start_with(&mut game, filler_live);
    let looked = game
        .state
        .player1
        .main_deck
        .cards
        .first()
        .copied()
        .expect("deck has a top card");
    while game.has_pending_choice() {
        let choice = game.get_pending_choice();
        let is_target = matches!(
            choice,
            rabuka_engine::ability::types::Choice::SelectTarget { .. }
        );
        if is_target {
            game.select_option(0);
        } else {
            game.select_indices(&[0]);
        }
    }
    assert!(
        !game.state.player1.main_deck.cards.contains(&looked),
        "the discarded card is no longer on the deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&looked),
        "choosing the discard option moves the looked-at top card to the waitroom"
    );
}

#[test]
fn dia_then_azuna_share_opponent_waitroom_and_preserve_selection_order() {
    use rabuka_engine::ability::types::Choice;
    use rabuka_engine::core::types::AbilityTrigger;

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let azuna = game.id(AZUNA_TARGET);
    let dia = game.id("PL!S-bp7-013-N");
    let a = game.new_id("PL!S-sd1-001-SD");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!S-sd1-001-SD");
    let d = game.new_id("PL!S-sd1-001-SD");
    let live1 = game.new_id(FILLER_LIVE);
    let live2 = game.new_id(FILLER_LIVE);
    let live3 = game.new_id(FILLER_LIVE);
    game.state.player1.stage.stage[1] = azuna;
    game.state.player2.waitroom.cards.extend([a, live1, b, live2, c, live3, d]);
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);

    fire_trigger(&mut game, dia, AbilityTrigger::Debut, "登場");
    game.select_option(1);
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            filtered_indices,
            target_player_id,
            ..
        } => {
            assert_eq!(zone, "discard");
            assert_eq!(filtered_indices.as_deref(), Some(&[0, 2, 4, 6][..]));
            assert_eq!(target_player_id.as_deref(), Some("self"));
        }
        other => panic!("expected Dia member selection, got {other:?}"),
    }
    game.select_indices(&[1, 0]);

    fire_trigger(&mut game, azuna, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(1);
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            filtered_indices,
            target_player_id,
            ..
        } => {
            assert_eq!(zone, "discard");
            assert_eq!(filtered_indices.as_deref(), Some(&[2, 4][..]));
            assert_eq!(target_player_id.as_deref(), Some("self"));
        }
        other => panic!("expected Azuna member selection, got {other:?}"),
    }
    game.select_indices(&[1, 0]);

    assert!(!game.has_pending_choice());
    for card in [a, b, c, d] {
        assert!(!game.state.player2.waitroom.cards.contains(&card));
        assert!(game.state.player2.main_deck.cards.contains(&card));
    }
    let p2_deck = &game.state.player2.main_deck.cards;
    assert_eq!(&p2_deck[p2_deck.len() - 4..], &[b, a, d, c]);
    assert_eq!(p2_deck.len(), 14);
    for live in [live1, live2, live3] {
        assert!(game.state.player2.waitroom.cards.contains(&live));
    }
    assert_eq!(game.state.player2.waitroom.cards.len(), 3);
    assert!(game.state.player1.waitroom.cards.is_empty());
}

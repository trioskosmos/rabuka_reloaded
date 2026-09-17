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

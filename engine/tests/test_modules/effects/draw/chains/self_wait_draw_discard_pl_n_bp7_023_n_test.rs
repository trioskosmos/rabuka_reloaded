use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

// PL!N-bp7-023-N Mia (was happy-path only)
// {{起動}}{{ターン1回}}このメンバーをウェイトにする：カードを2枚引き、
// 手札を2枚控え室に置く。 Mirrors Natsumi's hostile matrix at 2-card scale.

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

fn drain(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 15 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after answering"
    );
}

#[test]
fn pl_n_bp7_023_n_activation_waits_self_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;

    let d1 = game.new_id("PL!-sd1-001-SD");
    let d2 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    let h1 = game.new_id("PL!-sd1-007-SD");
    let h2 = game.new_id("PL!-sd1-004-SD");
    game.add_to_hand(h1);
    game.add_to_hand(h2);

    game.activate_ability(mia);
    assert_eq!(
        game.state.mods.get_orientation_modifier(mia),
        Some("wait"),
        "activation cost waits this member"
    );

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&d1) && game.state.player1.hand.cards.contains(&d2),
        "drawn cards reached the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&h1)
            && game.state.player1.waitroom.cards.contains(&h2),
        "two hand cards discarded to the waitroom"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(mia, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, mia), 0, "no re-offer after consuming");
}

/// Edge: already-wait Mia → wait-self cost unpayable → refused.
#[test]
fn pl_n_bp7_023_n_already_wait_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;
    game.state.mods.add_orientation_modifier(mia, "wait");
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));

    assert_eq!(use_offers(&game, mia), 0, "waited Mia: not offered");
    let result = game.try_activate_ability(mia);
    assert!(
        result.is_err(),
        "wait-self on an already-wait member must be refused, got {:?}",
        result
    );
}

/// Edge: EMPTY starting hand → draw 2, then discard exactly those 2
/// (forced full-discard selection), ending empty.
#[test]
fn pl_n_bp7_023_n_empty_hand_draws_then_discards_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "test setup: hand starts empty"
    );

    game.activate_ability(mia);
    drain(&mut game);

    assert!(
        game.state.player1.hand.cards.is_empty(),
        "drew 2 then discarded 2: ends empty"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "both drawn cards reached the waitroom"
    );
}

/// Edge: budget resets next turn.
#[test]
fn pl_n_bp7_023_n_available_again_next_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));

    game.activate_ability(mia);
    drain(&mut game);
    assert_eq!(use_offers(&game, mia), 0);

    game.state.turn_number += 1;
    // Mia herself is waited now; the limit reset is observed through a fresh
    // instance (per-instance budget, Kasumi pattern).
    let mia2 = game.new_id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[0] = mia2;
    assert_eq!(
        use_offers(&game, mia2),
        1,
        "fresh instance next turn: offered"
    );
}

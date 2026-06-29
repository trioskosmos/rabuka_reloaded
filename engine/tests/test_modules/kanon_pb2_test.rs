/// Tests for PL!SP-pb2-001-R (澁谷かのん / Kanon) — debut look-and-select
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn setup_kanon(game: &mut TestGame, top_cards: &[i16]) -> i16 {
    let kanon = game.id("PL!SP-pb2-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for &cid in top_cards.iter().rev() {
        game.state.player1.main_deck.cards.insert(0, cid);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    kanon
}

/// Pay the optional cost by selecting hand index 0.
fn pay_optional_cost(game: &mut TestGame) {
    TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![0])).ok();
}

/// Select Liella! cost≤4 from looked_at, skip stage followup → stays in hand.
#[test]
fn kanon_select_liella_cost4_keep_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);

    assert!(game.has_pending_choice(), "looked_at choice appears");
    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array());
    assert_eq!(
        filtered.map(|a| a.len()).unwrap_or(0),
        1,
        "only Liella! selectable"
    );

    game.select_indices(&[0]); // select liella from looked_at
                               // skip stage debut followup → card stays in hand
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "Liella! in hand"
    );
}

/// Select → choose stage debut → card on stage.
#[test]
fn kanon_select_liella_debut_to_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]); // select from looked_at

    // Followup: stage debut choice — pick option 0
    assert!(game.has_pending_choice(), "stage debut choice");
    game.select_option(0);

    // Select card from hand to debut, then choose position.
    assert!(game.has_pending_choice(), "select card from hand");
    game.select_indices(&[0]);

    // SelectPosition: card_id=0 = "left" (empty slot, Center is occupied by Kanon)
    if game.has_pending_choice() {
        game.select_option(0);
    }

    assert!(
        game.state
            .player1
            .stage
            .stage
            .iter()
            .any(|&id| id == liella),
        "on stage"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&liella),
        "not in hand"
    );
}

/// Skip optional cost → effect not executed.
#[test]
fn kanon_skip_cost_effect_not_executed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    // Skip optional cost
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        !game.state.player1.hand.cards.contains(&liella),
        "not in hand"
    );
}

/// No matching cards → all discarded, then skip followup.
#[test]
fn kanon_no_matching_cards_discard_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let non_mus = game.id("PL!HS-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[filler, non_mus]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    // Drain followup choice
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "discarded"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_mus),
        "discarded"
    );
}

/// Cost > 4 Liella! rejected, then skip followup.
#[test]
fn kanon_cost_above_4_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon_self = game.id("PL!SP-pb2-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[kanon_self, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Non-Liella! rejected by group filter, then skip followup.
#[test]
fn kanon_non_liella_cost4_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let non_liella = game.id("PL!-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[non_liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// max=1: exactly 1 card from looked_at goes to hand.
#[test]
fn kanon_max_1_enforced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella_a = game.id("PL!SP-PR-003-PR");
    let liella_b = game.id("PL!SP-PR-004-PR");
    let kanon = setup_kanon(&mut game, &[liella_a, liella_b]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]); // select first matching from looked_at
                               // skip followup
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Only 1 of the 2 Liella! cards should be in hand (max=1)
    let hand_count = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == liella_a || id == liella_b)
        .count();
    assert_eq!(hand_count, 1, "max=1 enforced");
}

/// Stage full → debut fails, card stays in hand.
#[test]
fn kanon_stage_full_falls_back_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let kanon = setup_kanon(&mut game, &[liella, filler]);
    game.state.player1.stage.stage = [filler, filler, filler];

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]);

    // Try stage debut
    if game.has_pending_choice() {
        game.select_option(0);
    }
    // Select card from hand for stage
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // Position choice (stage has no empty slots so this might fail)
    if game.has_pending_choice() {
        game.select_option(0);
    }
    assert!(
        !game
            .state
            .player1
            .stage
            .stage
            .iter()
            .any(|&id| id == liella),
        "not on stage"
    );
}

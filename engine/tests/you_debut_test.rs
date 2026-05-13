/// You (渡辺 曜 PL!S-bp2-005-R+) — Debut look + select from deck
///
/// Ab#0 (登場): Optional discard from hand: look at top 7, select up to 3 heart02/04/05
///   member cards, add to hand, discard rest.
mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

fn resolve_all_up_to(game: &mut TestGame, max: usize) {
    for _ in 0..max {
        if !game.has_pending_choice() {
            return;
        }
        game.select_indices(&[]);
    }
    panic!("resolve_all_up_to: exceeded {} iters", max);
}

#[test]
fn you_q124_blade_heart_excluded_base_heart_included() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, blade_only, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_ability_ends_and_discard_only_grows_at_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        filler, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let initial_discard = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    resolve_all_up_to(&mut game, 30);
    assert_eq!(
        game.state.player1.waitroom.cards.len() - initial_discard,
        8,
        "Expected 8 in discard (1 cost + 7 looked-at)"
    );
}

#[test]
fn you_ability_select_1_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.extend(vec![
        qualifying, filler, filler, filler, filler, filler, filler, filler,
    ]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]);
    resolve_all_up_to(&mut game, 30);
    assert!(
        game.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card in hand"
    );
}

#[test]
fn you_ability_select_multiple_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0, 1, 2]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}

#[test]
fn you_ability_user_scenario_select_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(
        !game.state.player1.hand.cards.contains(&q2),
        "Q2 NOT in hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&q3),
        "Q3 NOT in hand"
    );
}

#[test]
fn you_q124_two_plays_both_reject_blade_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![filler, filler, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(
        !game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only NOT in hand"
    );
}

#[test]
fn you_select_2_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, filler, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
}

#[test]
fn you_select_3_then_ends() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let q1 = game.id("PL!S-sd1-001-SD");
    let q2 = game.id("PL!S-sd1-002-SD");
    let q3 = game.id("PL!S-sd1-003-SD");
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);
    game.state
        .player1
        .main_deck
        .cards
        .extend(vec![q1, q2, q3, filler, filler, filler, filler]);
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0, 1, 2]);
    }
    resolve_all_up_to(&mut game, 30);
    assert!(game.state.player1.hand.cards.contains(&q1), "Q1 in hand");
    assert!(game.state.player1.hand.cards.contains(&q2), "Q2 in hand");
    assert!(game.state.player1.hand.cards.contains(&q3), "Q3 in hand");
}

use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

const AURORA: &str = "PL!HS-bp5-018-L";
const FILLER: &str = "PL!-sd1-010-SD";

/// 3 members, all different names AND different costs → score +1
#[test]
fn aurora_flower_all_distinct_names_and_costs_grants_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id(AURORA);
    let m_a = game.id("PL!HS-bp1-012-PR"); // cost=4, 乙宗梢
    let m_b = game.id("PL!HS-bp5-012-N"); // cost=5, 百生吟子
    let m_c = game.id("PL!HS-bp5-010-N"); // cost=7, 村野さやか

    game.state.player1.stage.stage = [m_a, m_b, m_c];
    game.add_to_hand(aurora);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player2.hand.cards.push(game.id(FILLER));
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.set_live_card(aurora);
    finish_live_setup(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(aurora);
    assert_eq!(
        score, 1,
        "3 members with distinct names & costs should grant +1 score"
    );
}

/// 3 members, all different names but only 2 distinct costs → score 0
#[test]
fn aurora_flower_same_cost_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id(AURORA);
    let m_a = game.id("PL!HS-bp1-012-PR"); // cost=4, 乙宗梢
    let m_b = game.id("PL!HS-PR-004-PR"); // cost=4, 夕霧綴理 (same cost as m_a)
    let m_c = game.id("PL!HS-bp5-010-N"); // cost=7, 村野さやか

    game.state.player1.stage.stage = [m_a, m_b, m_c];
    game.add_to_hand(aurora);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player2.hand.cards.push(game.id(FILLER));
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.set_live_card(aurora);
    finish_live_setup(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(aurora);
    assert_eq!(score, 0, "2 distinct costs should NOT grant score (need 3)");
}

/// 3 members, all different costs but only 2 distinct names → score 0
#[test]
fn aurora_flower_same_name_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id(AURORA);
    let m_a = game.id("PL!HS-sd1-007-SD"); // cost=4, セラス柳田リリエンフェルト
    let m_b = game.id("PL!HS-pb1-023-N"); // cost=15, セラス柳田リリエンフェルト (same name as m_a)
    let m_c = game.id("PL!HS-bp5-010-N"); // cost=7,  村野さやか

    game.state.player1.stage.stage = [m_a, m_b, m_c];
    game.add_to_hand(aurora);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player2.hand.cards.push(game.id(FILLER));
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.set_live_card(aurora);
    finish_live_setup(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(aurora);
    assert_eq!(score, 0, "2 distinct names should NOT grant score (need 3)");
}

/// Only 2 members on stage → score 0
#[test]
fn aurora_flower_two_members_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id(AURORA);
    let m_a = game.id("PL!HS-bp1-012-PR"); // cost=4, 乙宗梢
    let m_b = game.id("PL!HS-bp5-010-N"); // cost=7, 村野さやか

    game.state.player1.stage.stage = [m_a, m_b, -1];
    game.add_to_hand(aurora);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player2.hand.cards.push(game.id(FILLER));
    game.give_energy(10);

    advance_to_live_set(&mut game);
    game.set_live_card(aurora);
    finish_live_setup(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(aurora);
    assert_eq!(score, 0, "Only 2 members should NOT grant score");
}

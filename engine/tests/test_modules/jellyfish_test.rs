/// Tests for Jellyfish (PL!SP-pb1-025-L) — LiveStart ability:
///
/// {{live_start.png|ライブ開始時}}自分のステージにいる、このターン中に登場、
/// またはエリアを移動した「5yncri5e!」のメンバー1人につき、
/// このカードを成功させる為の必要ハートを{{heart_00.png|heart0}}減らす。
///
/// For each 5yncri5e! member on your stage that debuted or moved this turn,
/// reduce this card's required heart00 by 1.
///
/// Q99: Two qualifying members each count once → reduction is 2.
/// Q98: A single member that both debuted AND moved → counted once, not twice.
///      (appeared_or_moved is OR logic, not additive)
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q99: 2 5yncri5e! members that both debuted this turn → each reduces by 1.
#[test]
fn jellyfish_q99_two_qualifying_members_reduce_by_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let member_a = game.id("PL!SP-PR-010-PR"); // 鬼塚冬毬, cost=2, 5yncri5e!
    let member_b = game.id("PL!SP-pb1-014-N"); // 嵐 千砂都, cost=2, 5yncri5e!
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 2 5yncri5e! members
    game.state.player1.stage.stage = [member_a, member_b, -1];

    // Hand: Jellyfish + filler
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);

    // Record appearances AFTER phase advancement (Active phase resets tracking)
    game.state.record_card_appearance(member_a);
    game.state.record_card_appearance(member_b);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    // LiveStart fired: counting 2 5yncri5e! members with appeared_or_moved
    use rabuka_engine::card::HeartColor;
    let reduction = game
        .state
        .mods
        .get_need_heart_modifier(jellyfish, HeartColor::Heart00);
    assert_eq!(
        reduction, -2,
        "Q99: 2 qualifying members should reduce heart00 by 2"
    );
}

/// Q98: 1 5yncri5e! member with BOTH appeared AND moved → counted once.
#[test]
fn jellyfish_q98_same_card_appeared_and_moved_counts_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let member = game.id("PL!SP-PR-010-PR"); // 鬼塚冬毬, cost=2, 5yncri5e!
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 1 5yncri5e! member
    game.state.player1.stage.stage = [member, -1, -1];

    // Record as BOTH appeared AND moved (OR logic should still count 1)
    game.state.record_card_appearance(member);
    game.state.record_card_movement(member);

    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    use rabuka_engine::card::HeartColor;
    let reduction = game
        .state
        .mods
        .get_need_heart_modifier(jellyfish, HeartColor::Heart00);
    assert_eq!(
        reduction, -1,
        "Q98: Same card with both flags should count once (OR logic), not twice"
    );
}

/// Negative: A non-5yncri5e! member moves via position change → group filter excludes it.
#[test]
fn jellyfish_non_5yncri5e_member_moves_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let non_5yncri5e = game.id("PL!-sd1-010-SD"); // 南ことり (Printemps), not 5yncri5e!
    let filler = game.id("PL!-sd1-013-SD");

    // Stage: non-5yncri5e! member in Center
    game.state.player1.stage.stage = [-1, non_5yncri5e, -1];

    // Record as moved (as if a position change moved it)
    game.state.record_card_movement(non_5yncri5e);

    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    use rabuka_engine::card::HeartColor;
    let reduction = game
        .state
        .mods
        .get_need_heart_modifier(jellyfish, HeartColor::Heart00);
    assert_eq!(
        reduction, 0,
        "Negative: Non-5yncri5e! member should not reduce heart00"
    );
}

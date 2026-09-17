/// Gameplay tests for レディバグ (PL!HS-bp2-024-L) — Q114:
///
/// {{live_start.png|ライブ開始時}}自分のステージに「徒町小鈴」が登場しており、
/// かつ「徒町小鈴」よりコストの大きい「村野さやか」が登場している場合、
/// このカードを成功させるための必要ハートを{{heart_00.png|heart0}}×3減らす。
///
/// Q114: Members just need to be on stage when the ability fires.
/// They do NOT need to have debuted that turn.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Both required members on stage (deployed on previous turns).
/// Heart requirement should be reduced by 3 heart0.
#[test]
fn ladybug_q114_both_members_on_stage_reduces_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ladybug = game.id("PL!HS-bp2-024-L");
    let kosuzu = game.id("PL!HS-sd1-013-SD"); // cost 2
    let sayaka = game.id("PL!HS-sd1-002-SD"); // cost 11 (> 2)
    let filler = game.id("PL!-sd1-010-SD");

    // Members already on stage from previous turn
    game.add_to_stage(MemberArea::LeftSide, kosuzu);
    game.add_to_stage(MemberArea::Center, sayaka);

    game.state.player1.hand.cards.push(ladybug);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ladybug);
    advance_to_live_start(&mut game);
    game.drain_auto_ability_choices();

    // LiveStart abilities fire. Ladybug's condition checks for
    // 小鈴 and さやか on stage (both present).
    // Verify the engine processed without error and reduced hearts.
    // The exact reduction is visible in game state need_heart_modifiers.
    let heart_mods = &game.state.mods.need_heart_modifiers;
    eprintln!("[LADYBUG] need_heart_modifiers: {:?}", heart_mods);
    let reduction: i32 = heart_mods
        .get(&ladybug)
        .and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart00))
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    let all_mods = heart_mods.get(&ladybug);
    eprintln!("[LADYBUG] all mods for ladybug: {:?}", all_mods);
    assert_eq!(
        reduction, -3,
        "Q114: Ladybug should reduce heart0 requirement by 3"
    );
}

/// Only one member on stage (missing さやか). Condition fails.
#[test]
fn ladybug_q114_missing_sayaka_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ladybug = game.id("PL!HS-bp2-024-L");
    let kosuzu = game.id("PL!HS-sd1-013-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, kosuzu);
    // No さやか on stage

    game.state.player1.hand.cards.push(ladybug);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ladybug);
    advance_to_live_start(&mut game);

    let reduction: i32 = game
        .state
        .mods
        .need_heart_modifiers
        .get(&ladybug)
        .and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart00))
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(reduction, 0, "Q114: Missing さやか → no heart reduction");
}

#[test]
fn ladybug_q114_missing_kosuzu_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let ladybug = game.id("PL!HS-bp2-024-L");
    let sayaka = game.id("PL!HS-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_stage(MemberArea::Center, sayaka);
    game.state.player1.hand.cards.push(ladybug);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ladybug);
    advance_to_live_start(&mut game);
    let reduction: i32 = game.state.mods.need_heart_modifiers.get(&ladybug).and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart00)).map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(reduction, 0, "Missing kosuzu → no reduction");
}

#[test]
fn ladybug_q114_wrong_character_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let ladybug = game.id("PL!HS-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    let other = game.id("PL!HS-bp1-001-R"); // not kosuzu/sayaka
    game.add_to_stage(MemberArea::LeftSide, other);
    game.add_to_stage(MemberArea::Center, other);
    game.state.player1.hand.cards.push(ladybug);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ladybug);
    advance_to_live_start(&mut game);
    let reduction: i32 = game.state.mods.need_heart_modifiers.get(&ladybug).and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart00)).map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(reduction, 0, "Wrong characters should not trigger");
}

#[test]
fn ladybug_q114_no_members_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let ladybug = game.id("PL!HS-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(ladybug);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(ladybug);
    advance_to_live_start(&mut game);
    let reduction: i32 = game.state.mods.need_heart_modifiers.get(&ladybug).and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart00)).map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(reduction, 0, "No members → no reduction");
}

/// Tests for PL!HS-bp5-006-R / P / AR (安養寺 姫芽) ab#0 — LiveStart:
///   手札の同じグループ名を持つカード2枚を控え室に置いてもよい：
///   ライブ終了時まで、heart01×2を得る。
///
/// Ability: [LiveStart] You may discard 2 cards from hand that have the same
/// group name (as each other): until end of live, gain +2 heart01.
///
/// "同じグループ名" means the 2 discarded cards share a group name with each
/// other — NOT necessarily the activating card's group.
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

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
}

fn get_heart01(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, rabuka_engine::card::HeartColor::Heart01)
}

// ---------------------------------------------------------------------------
// Test: 蓮ノ空 pair → cost prompt, pay, gain heart
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_same_group_莲ノ空_gains_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let same1 = game.new_id("PL!HS-bp6-011-R");
    let same2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(same1);
    game.state.player1.hand.cards.push(same2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: Printemps pair (different from activating card's group) → still works
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_different_group_pair_still_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let p1 = game.new_id("PL!-sd1-010-SD"); // Printemps
    let p2 = game.new_id("PL!-sd1-010-SD"); // Printemps

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(p1);
    game.state.player1.hand.cards.push(p2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "2 same-group cards (different from activating card) should trigger cost prompt"
    );
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: 1 蓮ノ空 + 1 Printemps → no matching pair → auto-skip
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_mixed_groups_auto_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let a = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let b = game.new_id("PL!-sd1-010-SD"); // Printemps

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(a);
    game.state.player1.hand.cards.push(b);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No matching same-group pair → auto-skip"
    );
    assert_eq!(get_heart01(&game, himeno), 0);
}

// ---------------------------------------------------------------------------
// Test: only 1 same-group card → auto-skip (need 2)
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_only_one_same_group_auto_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let same1 = game.new_id("PL!HS-bp6-011-R");
    let wrong = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(same1);
    game.state.player1.hand.cards.push(wrong);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice(), "Only 1 same-group → need 2");
    assert_eq!(get_heart01(&game, himeno), 0);
}

// ---------------------------------------------------------------------------
// Test: empty hand → auto-skip
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_empty_hand_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice());
    assert_eq!(get_heart01(&game, himeno), 0);
}

// ---------------------------------------------------------------------------
// Test: 2 no-group cards → auto-skip
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_no_group_cards_auto_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let n1 = game.new_id("PL!-bp5-111-R");
    let n2 = game.new_id("PL!-bp5-111-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(n1);
    game.state.player1.hand.cards.push(n2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice());
    assert_eq!(get_heart01(&game, himeno), 0);
}

// ---------------------------------------------------------------------------
// Test: 3 same-group cards → picks 2, 1 remains
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_three_same_group_picks_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");
    let s3 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(s3);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    let remaining = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&c| c == s1 || c == s2 || c == s3)
        .count();
    assert_eq!(remaining, 1);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: 3 cards, 2 share a group → only those 2 selectable
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_three_cards_two_share_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let pa = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let pb = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let lone = game.new_id("PL!-sd1-010-SD"); // Printemps

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(pa);
    game.state.player1.hand.cards.push(pb);
    game.state.player1.hand.cards.push(lone);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    if let rabuka_engine::ability::types::Choice::SelectCard {
        filtered_indices: Some(fi),
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            fi.as_slice(),
            &[0usize, 1],
            "Only the 蓮ノ空 pair should be selectable"
        );
    }
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&lone));
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: each rarity variant
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_p_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-P");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

#[test]
fn himeno_bp5_ar_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-AR");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: skip cost → no hearts, cards stay in hand
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_skip_cost_via_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    let _ = game.try_select_indices(&[]);
    assert_eq!(get_heart01(&game, himeno), 0);
    assert!(game.state.player1.hand.cards.contains(&s1));
    assert!(game.state.player1.hand.cards.contains(&s2));
}

// ---------------------------------------------------------------------------
// Test: himeno at left/right stage positions
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_left_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [himeno, -1, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

#[test]
fn himeno_bp5_right_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, -1, himeno];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: cross-unit same group
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_cross_unit_same_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空, みらくらぱーく！
    let s2 = game.new_id("PL!HS-bp1-012-PR"); // 蓮ノ空, スリーズブーケ

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert_eq!(get_heart01(&game, himeno), 2);
}

// ---------------------------------------------------------------------------
// Test: discarded cards end up in waitroom
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_discarded_go_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-R");
    let s1 = game.new_id("PL!HS-bp6-011-R");
    let s2 = game.new_id("PL!HS-bp6-011-R");

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(s1);
    game.state.player1.hand.cards.push(s2);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.state.player1.waitroom.cards.contains(&s1));
    assert!(game.state.player1.waitroom.cards.contains(&s2));
}

// ---------------------------------------------------------------------------
// Test: P variant wrong group auto-skips
// ---------------------------------------------------------------------------

#[test]
fn himeno_bp5_p_variant_mixed_groups_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp5-006-P");
    let a = game.new_id("PL!HS-bp6-011-R"); // 蓮ノ空
    let b = game.new_id("PL!-sd1-010-SD"); // Printemps

    game.state.player1.stage.stage = [-1, himeno, -1];
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.give_energy(10);
    advance_to_live_set(&mut game);

    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(a);
    game.state.player1.hand.cards.push(b);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    assert!(!game.has_pending_choice());
    assert_eq!(get_heart01(&game, himeno), 0);
}

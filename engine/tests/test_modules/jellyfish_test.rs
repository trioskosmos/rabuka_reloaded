/// Tests for Jellyfish (PL!SP-pb1-025-L) — LiveStart ability:
///
/// {{live_start.png|ライブ開始時}}自分のステージにいる、このターン中に登場、
/// またはエリアを移動した「5yncri5e!」のメンバー1人につき、
/// このカードを成功させる為の必要ハートを{{heart_00.png|heart0}}減らす。
///
/// For each 5yncri5e! member on your stage that debuted or moved this turn,
/// reduce this card's required heart00 by 1.
///
/// The timing condition "appeared_or_moved_this_turn" uses OR logic:
/// a member that both debuted AND moved in the same turn is counted once.
///
/// NOTE: `reset_keyword_tracking()` clears appearance/movement flags during
/// the Active phase (phases.rs:23). Tests that use direct state + manual
/// flag-setting must set flags AFTER advancing past Active (i.e. after
/// advance_to_live_card_set_p1 returns).
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

fn check_heart_reduction(game: &TestGame, card_id: i16, expected: i32) {
    use rabuka_engine::card::HeartColor;
    let reduction = game
        .state
        .mods
        .get_need_heart_modifier(card_id, HeartColor::Heart00);
    assert_eq!(reduction, expected);
}

fn fill_decks(game: &mut TestGame) {
    for _ in 0..10 {
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player2.main_deck.cards.push(f);
    }
}

// ── Two qualifying 5yncri5e! members, both appeared only → -2 ──
#[test]
fn jellyfish_two_members_appeared_reduce_by_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 嵐 千砂都, 5yncri5e!
    let wakana_pr = game.id("PL!SP-PR-010-PR"); // 若菜四季 PR, 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, wakana_pr, -1];

    advance_to_live_card_set_p1(&mut game);
    game.state.record_card_appearance(chisato, "");
    game.state.record_card_appearance(wakana_pr, "");

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

// ── One 5yncri5e! member that both appeared AND moved → counted once ──
#[test]
fn jellyfish_one_member_appeared_and_moved_counts_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 嵐 千砂都, 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);
    // Set BOTH flags — OR logic must count this card once, not twice
    game.state.record_card_appearance(chisato, "");
    game.state.record_card_movement(chisato);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -1);
}

// ── 2 qualifying + 1 non-5yncri5e! member → only 2 count ──
#[test]
fn jellyfish_mixed_qualifying_and_non_qualifying() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!
    let wakana_pr = game.id("PL!SP-PR-010-PR"); // 5yncri5e!
    let honoka = game.id("PL!-sd1-010-SD"); // Printemps, not 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [honoka, chisato, wakana_pr];

    advance_to_live_card_set_p1(&mut game);
    game.state.record_card_appearance(chisato, "");
    game.state.record_card_appearance(wakana_pr, "");
    game.state.record_card_appearance(honoka, "");

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -2);
}

// ── No qualifying members → no reduction ──
#[test]
fn jellyfish_no_qualifying_members_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let honoka = game.id("PL!-sd1-010-SD"); // Printemps, not 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [honoka, -1, -1];

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, 0);
}

// ── Three qualifying 5yncri5e! members → each counted once → -3 ──
#[test]
fn jellyfish_three_members_all_qualify_reduce_by_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 嵐 千砂都, 5yncri5e!
    let wakana_pr = game.id("PL!SP-PR-010-PR"); // 若菜四季 PR, 5yncri5e!
    let natsumi = game.id("PL!SP-pb1-009-R"); // 鬼塚夏美, 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, wakana_pr, natsumi];

    advance_to_live_card_set_p1(&mut game);
    // All 3 both appeared AND moved → each counted once, not 6
    for &m in &[chisato, wakana_pr, natsumi] {
        game.state.record_card_appearance(m, "");
        game.state.record_card_movement(m);
    }

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -3);
}

// ── Member appears only (no movement) → still counts ──
#[test]
fn jellyfish_member_only_appeared_still_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);
    game.state.record_card_appearance(chisato, "");

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -1);
}

// ── Member only moved (no appearance) → still counts ──
#[test]
fn jellyfish_member_only_moved_still_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);
    game.state.record_card_movement(chisato);

    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, -1);
}

// ── Member on stage with NEITHER flag → does NOT count ──
#[test]
fn jellyfish_member_neither_appeared_nor_moved_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let jellyfish = game.id("PL!SP-pb1-025-L");
    let chisato = game.id("PL!SP-pb1-014-N"); // 5yncri5e!

    fill_decks(&mut game);
    game.state.player1.hand.cards.push(jellyfish);
    game.state.player1.stage.stage = [chisato, -1, -1];

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(jellyfish);
    advance_to_live_start(&mut game);

    check_heart_reduction(&game, jellyfish, 0);
}

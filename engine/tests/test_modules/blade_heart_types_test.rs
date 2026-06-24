use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Test card: PL!N-bp5-001-R＋ (上原歩夢, A・ZU・NA, cost 5, blade 4)
/// Ability: When you yell, if ≥3 different blade heart types (heart01-heart06)
/// among revealed cards → gain heart01 until live end.
/// If ≥6 types → additionally gain "常時 ライブの合計スコアを+1する".

const ABILITY_CARD: &str = "PL!N-bp5-001-R＋";

/// Blade heart color reference cards (each contributes exactly 1 color via blade_heart)
const B_HEART01: &str = "PL!-sd1-013-SD";
const B_HEART02: &str = "PL!S-PR-017-PR";
const B_HEART03: &str = "PL!-sd1-010-SD";
const B_HEART04: &str = "PL!S-PR-015-PR";
const B_HEART05: &str = "PL!S-bp2-015-PR";
const B_HEART06: &str = "PL!N-bp1-021-N";
const B_ALL: &str = "PL!-sd1-020-SD";

/// Card with base_heart (heart01, heart03, heart06) but NO blade_heart.
/// Should NOT count toward blade heart type conditions.
const BASE_HEART_ONLY: &str = "PL!-sd1-014-SD";

fn setup(game: &mut TestGame, revealed_card_ids: &[i16]) -> i16 {
    let ability_card = game.id(ABILITY_CARD);
    game.state.player1.stage.stage = [-1, ability_card, -1];
    for &id in revealed_card_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    ability_card
}

fn get_heart_modifier(game: &TestGame, card_id: i16, color: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card_id, color)
}

fn get_score_modifier(game: &TestGame, card_id: i16) -> i32 {
    game.state.mods.get_score_modifier(card_id)
}

/// 0 blade heart types among revealed cards → both conditions fail
#[test]
fn zero_blade_heart_types_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Use only cards with NO blade_heart (base_heart only)
    let bh_only = game.id(BASE_HEART_ONLY);
    let ability_card = setup(&mut game, &[bh_only]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        0,
        "No heart01 granted with 0 blade heart types"
    );
    assert_eq!(
        get_score_modifier(&game, ability_card),
        0,
        "No score modifier with 0 blade heart types"
    );
}

/// 2 blade heart types among revealed cards → both conditions fail (need 3+)
#[test]
fn two_blade_heart_types_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let ability_card = setup(&mut game, &[h01, h02]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        0,
        "No heart01 granted with only 2 blade heart types"
    );
    assert_eq!(
        get_score_modifier(&game, ability_card),
        0,
        "No score modifier with only 2 blade heart types"
    );
}

/// 3 blade heart types → first condition passes (heart01), second fails
#[test]
fn three_blade_heart_types_grants_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let h03 = game.id(B_HEART03);
    let ability_card = setup(&mut game, &[h01, h02, h03]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "Should have heart01 ×1 with 3 blade heart types"
    );
    assert_eq!(
        get_score_modifier(&game, ability_card),
        0,
        "No score modifier with only 3 blade heart types (need 6)"
    );
}

/// 5 blade heart types → first condition passes (heart01), second still fails
#[test]
fn five_blade_heart_types_grants_heart01_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let h03 = game.id(B_HEART03);
    let h04 = game.id(B_HEART04);
    let h05 = game.id(B_HEART05);
    let ability_card = setup(&mut game, &[h01, h02, h03, h04, h05]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "Should have heart01 ×1 with 5 blade heart types"
    );
    assert_eq!(
        get_score_modifier(&game, ability_card),
        0,
        "No score modifier with 5 blade heart types (need 6)"
    );
}

/// 6 blade heart types → BOTH conditions pass
#[test]
fn six_blade_heart_types_grants_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let h03 = game.id(B_HEART03);
    let h04 = game.id(B_HEART04);
    let h05 = game.id(B_HEART05);
    let h06 = game.id(B_HEART06);
    let ability_card = setup(&mut game, &[h01, h02, h03, h04, h05, h06]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "Should have heart01 ×1 with 6 blade heart types"
    );
    // The gain_ability action grants modify_score to the member card
    assert_eq!(
        get_score_modifier(&game, ability_card),
        1,
        "Score should be +1 with 6 blade heart types (applied to member card)"
    );
}

/// All 6 blade heart types + b_all (wildcard) → both conditions pass
#[test]
fn all_blade_heart_types_with_b_all_grants_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let h03 = game.id(B_HEART03);
    let h04 = game.id(B_HEART04);
    let h05 = game.id(B_HEART05);
    let h06 = game.id(B_HEART06);
    let ball = game.id(B_ALL);
    let ability_card = setup(&mut game, &[h01, h02, h03, h04, h05, h06, ball]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "Should have heart01 ×1 with all blade heart types + b_all"
    );
    assert_eq!(
        get_score_modifier(&game, ability_card),
        1,
        "Score should be +1 with b_all contributing all 6+ types"
    );
}

/// Cards with base_heart but NO blade_heart should NOT count toward
/// the blade heart type condition (heart_source: "blade").
#[test]
fn base_heart_only_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // BASE_HEART_ONLY has base_heart {heart01, heart03, heart06} but NO blade_heart
    let b1 = game.id(BASE_HEART_ONLY);
    let b2 = game.id(BASE_HEART_ONLY);
    let b3 = game.id(BASE_HEART_ONLY);
    let ability_card = setup(&mut game, &[b1, b2, b3]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        0,
        "Cards with only base_heart should NOT trigger blade heart condition"
    );
}

/// Mix of blade hearts and base-heart-only cards:
/// 3 blade heart types → condition passes, base-heart-only cards ignored
#[test]
fn blade_hearts_count_base_hearts_ignored() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01 = game.id(B_HEART01);
    let h02 = game.id(B_HEART02);
    let h03 = game.id(B_HEART03);
    let bh_only = game.id(BASE_HEART_ONLY); // base heart only, no blade
    let bh_only2 = game.id(BASE_HEART_ONLY);
    let ability_card = setup(&mut game, &[h01, h02, h03, bh_only, bh_only2]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "3 blade heart types should grant heart01 even with base-heart-only cards present"
    );
}

/// Duplicate blade heart colors should NOT count as extra types.
/// 3 cards, but only 2 distinct blade heart colors → condition fails.
#[test]
fn duplicate_blade_heart_colors_do_not_stack() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let h01a = game.id(B_HEART01);
    let h01b = game.id(B_HEART01); // same color as h01a
    let h02 = game.id(B_HEART02);
    let ability_card = setup(&mut game, &[h01a, h01b, h02]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        0,
        "Only 2 distinct blade heart types — condition needs 3"
    );
}

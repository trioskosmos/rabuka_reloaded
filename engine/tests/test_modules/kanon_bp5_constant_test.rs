/// Tests for PL!SP-bp5-012-N (澁谷かのん) — 常時 ability
///
/// Ability:
///   自分のライブカード置き場に必要ハートの合計が8以上の『Liella!』のライブカードがあるかぎり、
///   heart03を得る。
///
/// Uses group_condition with aggregate=total, location=live_card_zone,
/// group_names=["Liella!"], count=8, operator=>=.
///
/// The critical bugfix: get_group_card_count previously always summed base_heart on
/// the stage (via sum_group_hearts_in_stage) ignoring condition.location, so even an
/// empty live_card_zone could incorrectly satisfy the condition if stage members had
/// enough hearts. The fix makes the aggregate sum zone-aware.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn setup_cards() -> (i16, i16, i16, i16, i16) {
    let db = load_real_database();
    let game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp5-012-N");
    let tiny_stars = game.id("PL!SP-bp1-024-L");   // Liella!, need_heart total = 8
    let start_true = game.id("PL!SP-bp1-023-L");   // Liella!, need_heart total = 4
    let start_dash = game.id("PL!-sd1-019-SD");     // non-Liella, need_heart total = 3
    let filler = game.id("PL!-sd1-010-SD");
    (kanon, tiny_stars, start_true, start_dash, filler)
}

/// Empty live_card_zone → condition not met → no heart03 bonus.
#[test]
fn kanon_constant_empty_live_card_zone_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, _, _, _, filler) = setup_cards();

    game.add_to_stage(MemberArea::Center, kanon);
    game.state.player1.stage.stage = [filler, kanon, -1];
    // live_card_zone intentionally left empty
    // Also put some cards on stage that could erroneously satisfy the old buggy check
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 0,
        "Empty live_card_zone → condition fails, expected heart03=0, got {}", heart);
}

/// Liella! live card with total need_heart = 8 → condition met → +1 heart03.
#[test]
fn kanon_constant_liella_live_need_heart_8_gains_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, tiny_stars, _, _, filler) = setup_cards();

    game.state.player1.stage.stage = [filler, kanon, -1];
    game.state.player1.live_card_zone.cards.push(tiny_stars);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 1,
        "Liella! live card need_heart=8 → condition met, expected heart03=1, got {}", heart);
}

/// Liella! live card with total need_heart = 4 (< 8) → condition not met → no bonus.
#[test]
fn kanon_constant_liella_live_need_heart_4_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, _, start_true, _, filler) = setup_cards();

    game.state.player1.stage.stage = [filler, kanon, -1];
    game.state.player1.live_card_zone.cards.push(start_true);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 0,
        "Liella! live card need_heart=4 (<8) → condition fails, expected heart03=0, got {}", heart);
}

/// Non-Liella! live card → group filter rejects it → condition not met → no bonus.
#[test]
fn kanon_constant_non_liella_live_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, _, _, start_dash, filler) = setup_cards();

    game.state.player1.stage.stage = [filler, kanon, -1];
    game.state.player1.live_card_zone.cards.push(start_dash);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 0,
        "Non-Liella! live card → group filter fails, expected heart03=0, got {}", heart);
}

/// Two Liella! live cards totalling 4+4=8 → condition met → +1 heart03.
#[test]
fn kanon_constant_two_liella_live_cards_sum_to_8() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, _, start_true, _, filler) = setup_cards();
    let start_true2 = game.new_id("PL!SP-bp1-023-L"); // second copy, need_heart=4

    game.state.player1.stage.stage = [filler, kanon, -1];
    game.state.player1.live_card_zone.cards.push(start_true);
    game.state.player1.live_card_zone.cards.push(start_true2);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 1,
        "Two Liella! live cards (4+4=8) → condition met, expected heart03=1, got {}", heart);
}

/// Cards in success_live_card_zone should NOT satisfy the condition (wrong zone).
#[test]
fn kanon_constant_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, tiny_stars, _, _, filler) = setup_cards();

    game.state.player1.stage.stage = [filler, kanon, -1];
    // Put qualifying card in the WRONG zone (success, not live_card_zone)
    game.state.player1.success_live_card_zone.cards.push(tiny_stars);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 0,
        "Card in success_live_card_zone → wrong zone, expected heart03=0, got {}", heart);
}

/// Condition met → bonus applied; then cards removed → bonus removed.
#[test]
fn kanon_constant_bonus_removed_when_cards_leaves_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, tiny_stars, _, _, filler) = setup_cards();

    game.state.player1.stage.stage = [filler, kanon, -1];
    game.state.player1.live_card_zone.cards.push(tiny_stars);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    // Condition met → bonus active
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03), 1);

    // Remove the live card → condition fails → bonus removed
    game.state.player1.live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let heart = game.state.mods.get_heart_modifier(kanon, HeartColor::Heart03);
    assert_eq!(heart, 0,
        "Cards removed from live_card_zone → bonus should be 0, got {}", heart);
}

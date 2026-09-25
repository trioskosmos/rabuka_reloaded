use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// PL!SP-pb2-032-N (ウィーン・マルガレーテ)
// 常時: 自分のエネルギーが6枚以上あるかぎり、heart06を得る。
//       8枚以上あるかぎり、さらにheart06を得る。
//
// Sequential as_long_as: if ≥6 energy → +1 heart06.
// If also ≥8 energy → +1 more heart06.
// ====================================================================

fn setup_energy_threshold_hearts(energy_count: usize) -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb2-032-N"); // cost 4
    let filler = game.id("PL!-sd1-010-SD");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.add_to_hand(wien);
    // Give enough energy to play Wien (cost 4), then set exact total count
    game.give_energy(energy_count + 4);
    game.play_to_stage(wien, MemberArea::Center);
    // Now total energy cards = energy_count + 4, minus 4 consumed = energy_count
    // But play_to_stage doesn't remove cards from energy_zone — only decrements
    // active_energy_count. So the total cards in energy_zone is energy_count + 4.
    // For the condition to check total cards matching `energy_count`,
    // we need exactly `energy_count` cards in energy_zone.
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..energy_count {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state
        .player1
        .energy_zone
        .set_active_count(energy_count as u8);

    (game, wien)
}

const H06: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart06;

/// 0 energy → no heart06
#[test]
fn energy_threshold_zero_grants_no_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(0);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 0, "0 energy → 0 heart06, got {}", h06);
}

/// 5 energy (<6) → 0 heart06
#[test]
fn energy_threshold_five_grants_no_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(5);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 0, "5 energy (<6) → 0 heart06, got {}", h06);
}

/// 6 energy (≥6, <8) → 1 heart06
#[test]
fn energy_threshold_six_grants_one_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(6);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 1, "6 energy (≥6) → 1 heart06, got {}", h06);
}

/// 7 energy (≥6, <8) → 1 heart06
#[test]
fn energy_threshold_seven_grants_one_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(7);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 1, "7 energy (≥6, <8) → 1 heart06, got {}", h06);
}

/// 8 energy (≥6, ≥8) → 2 heart06
#[test]
fn energy_threshold_eight_grants_two_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(8);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 2, "8 energy (≥6, ≥8) → 2 heart06, got {}", h06);
}

/// 10 energy (≥6, ≥8) → 2 heart06
#[test]
fn energy_threshold_ten_grants_two_heart06() {
    let (mut game, wien) = setup_energy_threshold_hearts(10);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 2, "10 energy (≥6, ≥8) → 2 heart06, got {}", h06);
}

/// Heart06 not granted to wrong heart color (heart02)
#[test]
fn energy_threshold_ten_grants_no_heart02() {
    let (mut game, wien) = setup_energy_threshold_hearts(10);
    game.state.recalculate_constants();

    let h02 = game
        .state
        .mods
        .get_heart_modifier(wien, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(h02, 0, "10 energy → heart02 should be 0, got {}", h02);
}

/// Energy dropped below threshold → heart removed (dynamic as_long_as)
#[test]
fn energy_threshold_heart06_updates_when_energy_falls_and_rises() {
    let (mut game, wien) = setup_energy_threshold_hearts(10);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(wien, H06),
        2,
        "10 energy → 2 heart06"
    );

    // Drop to 5 energy (below both thresholds)
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..5 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(5);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(
        h06, 0,
        "After dropping to 5 energy → 0 heart06, got {}",
        h06
    );

    // Raise to 7 energy (only first threshold)
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(7);
    game.state.recalculate_constants();

    let h06 = game.state.mods.get_heart_modifier(wien, H06);
    assert_eq!(h06, 1, "After raising to 7 energy → 1 heart06, got {}", h06);
}

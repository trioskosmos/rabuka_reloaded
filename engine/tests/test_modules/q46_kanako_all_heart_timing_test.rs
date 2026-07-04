/// Q46: 鐘 嵐珠 (PL!N-bp1-012-R+) — 常時: 3+ live cards in zone including
/// 1+ 虹ヶ咲 live card → gain 2 ALL hearts + 2 blades.
///
/// Q: When is the color of the ALL hearts decided?
/// A: At heart-check time during performance phase.
///
/// This means the constant grants the blades and hearts immediately on
/// recalculate_constants, but the ALL heart COLOR selection is deferred
/// to the moment the engine checks need_heart satisfaction.
use crate::helpers::*;

/// Q46: Kanako on stage + 3 live cards (1 虹ヶ咲) → gains 2 blades.
/// The ALL hearts are granted but their color is chosen at heart-check time.
#[test]
fn q46_kanako_constant_grants_blades_when_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD"); // 虹ヶ咲 live card
    let other_live = game.id("PL!-sd1-019-SD"); // non-虹ヶ咲 live card
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kanako, filler];

    // 3 live cards in zone: 1 虹ヶ咲 + 2 others → condition met
    game.state.player1.live_card_zone.cards.push(niji_live);
    game.state.player1.live_card_zone.cards.push(other_live);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade, 2,
        "Q46: 3 live cards (1 虹ヶ咲) → constant fires → +2 blades, got {}",
        blade
    );
}

/// Q46 edge: < 3 live cards → condition fails → no gain.
#[test]
fn q46_kanako_condition_less_than_3_live_cards_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kanako, filler];

    // Only 2 live cards → condition fails
    game.state.player1.live_card_zone.cards.push(niji_live);
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade, 0,
        "Q46: < 3 live cards → no blade gain"
    );
}

/// Q46 edge: 3+ live cards but NONE are 虹ヶ咲 → condition fails.
#[test]
fn q46_kanako_no_nijigasaki_live_card_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kanako, filler];

    // 3 live cards, none are 虹ヶ咲
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade, 0,
        "Q46: 3 live cards but no 虹ヶ咲 → no blade gain"
    );
}

/// Q46 edge: Kanako not on stage → constant doesn't evaluate.
#[test]
fn q46_kanako_not_on_stage_no_constant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Kanako in hand, not on stage
    game.state.player1.stage.stage = [filler, -1, filler];

    game.state.player1.live_card_zone.cards.push(niji_live);
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade, 0,
        "Q46: Kanako not on stage → constant not evaluated"
    );
}

/// Q46 edge: Condition met → Kanako leaves stage → blade removed.
#[test]
fn q46_kanako_leaves_stage_blade_removed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kanako, filler];

    game.state.player1.live_card_zone.cards.push(niji_live);
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade_before = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(blade_before, 2, "Condition met → +2 blades");

    // Remove Kanako from stage
    game.state.player1.stage.stage = [filler, -1, filler];
    game.state.recalculate_constants();

    let blade_after = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade_after, 0,
        "Q46: Kanako leaves stage → constant removed → 0 blades"
    );
}

/// Q46 edge: Condition met → live card removed from zone → condition fails → blade removed.
#[test]
fn q46_live_card_removed_condition_fails_blade_removed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kanako, filler];

    game.state.player1.live_card_zone.cards.push(niji_live);
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(kanako), 2);

    // Remove 2 live cards → only 1 left → condition fails
    game.state.player1.live_card_zone.cards.clear();
    game.state.player1.live_card_zone.cards.push(niji_live);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(kanako);
    assert_eq!(
        blade, 0,
        "Q46: Live cards removed → < 3 → condition fails → blade removed"
    );
}

/// Q46 edge: Multiple Kanako copies — each gains independently.
#[test]
fn q46_multiple_kanako_each_gains_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanako1 = game.id("PL!N-bp1-012-R\u{ff0b}");
    let kanako2 = game.id("PL!N-bp1-012-R\u{ff0b}");
    let niji_live = game.id("PL!N-sd1-025-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [kanako1, kanako2, filler];

    game.state.player1.live_card_zone.cards.push(niji_live);
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));
    game
        .state
        .player1
        .live_card_zone
        .cards
        .push(game.new_id("PL!-sd1-019-SD"));

    game.state.recalculate_constants();

    let blade1 = game.state.mods.get_blade_modifier(kanako1);
    let blade2 = game.state.mods.get_blade_modifier(kanako2);
    assert_eq!(
        blade1, 2,
        "Q46: First Kanako gains 2 blades"
    );
    assert_eq!(
        blade2, 2,
        "Q46: Second Kanako gains 2 blades"
    );
}

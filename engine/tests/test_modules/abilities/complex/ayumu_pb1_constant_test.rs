/// Tests for PL!N-pb1-001-R / PL!N-pb1-001-P+ 上原歩夢 (ab#1) — 常時 ability
///
/// Ability:
///   自分のライブ中のライブカードが2枚以上あるかぎり、ブレード×2を得る。
///
/// Uses card_count_condition with location=live_card_zone, count=2, operator=>=.
/// The critical parser fix: "ライブ中のライブカード" must map to location=live_card_zone
/// (previously only "ライブ中のカード" was recognized, missing this variant).
use crate::helpers::*;

/// Empty live_card_zone → condition not met → no blade bonus.
#[test]
fn ayumu_constant_empty_live_card_zone_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    // live_card_zone intentionally left empty
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 0,
        "Empty live_card_zone → condition fails, expected blade=0, got {}",
        blade
    );
}

/// 1 card in live_card_zone → condition not met (needs ≥2) → no blade bonus.
#[test]
fn ayumu_constant_one_live_card_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 0,
        "1 card in live_card_zone (<2) → condition fails, expected blade=0, got {}",
        blade
    );
}

/// 1 live card in live_card_zone + 2 member cards on stage → condition NOT met
/// (member cards on stage don't count toward live_card_zone count).
#[test]
fn ayumu_constant_member_cards_dont_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let member1 = game.id("PL!-sd1-010-SD");
    let member2 = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member1, ayumu, member2];
    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 0,
        "1 live card + 2 member cards on stage → only live_card_zone counts, expected blade=0, got {}",
        blade
    );
}

/// 2 cards in live_card_zone → condition met → +2 blade.
#[test]
fn ayumu_constant_two_live_cards_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let live_card1 = game.id("PL!-sd1-019-SD");
    let live_card2 = game.new_id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    game.state.player1.live_card_zone.cards.push(live_card1);
    game.state.player1.live_card_zone.cards.push(live_card2);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 2,
        "2 cards in live_card_zone → condition met, expected blade=2, got {}",
        blade
    );
}

/// 3 cards in live_card_zone → condition still met (≥2) → +2 blade.
#[test]
fn ayumu_constant_three_live_cards_still_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let live_card1 = game.id("PL!-sd1-019-SD");
    let live_card2 = game.new_id("PL!-sd1-019-SD");
    let live_card3 = game.new_id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    game.state.player1.live_card_zone.cards.push(live_card1);
    game.state.player1.live_card_zone.cards.push(live_card2);
    game.state.player1.live_card_zone.cards.push(live_card3);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 2,
        "3 cards in live_card_zone → condition still met, expected blade=2, got {}",
        blade
    );
}

/// Cards in success_live_card_zone should NOT satisfy the condition (wrong zone).
#[test]
fn ayumu_constant_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    // Put cards in the WRONG zone (success, not live_card_zone)
    for _ in 0..3 {
        let c = game.new_id("PL!-sd1-019-SD");
        game.state.player1.success_live_card_zone.cards.push(c);
    }
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 0,
        "Cards in success_live_card_zone → wrong zone, expected blade=0, got {}",
        blade
    );
}

/// Condition met → bonus applied; then cards removed → bonus removed.
#[test]
fn ayumu_constant_bonus_removed_when_cards_leave_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("PL!N-pb1-001-R");
    let live_card1 = game.id("PL!-sd1-019-SD");
    let live_card2 = game.new_id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, ayumu, -1];
    game.state.player1.live_card_zone.cards.push(live_card1);
    game.state.player1.live_card_zone.cards.push(live_card2);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    // Condition met → bonus active
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ayumu),
        2,
        "Should have blade+2 when 2 cards in zone"
    );

    // Remove live cards → condition fails → bonus removed
    game.state.player1.live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(ayumu);
    assert_eq!(
        blade, 0,
        "Cards removed from live_card_zone → bonus should be 0, got {}",
        blade
    );
}

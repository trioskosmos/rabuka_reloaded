/// Tests for PL!-bp4-020-L (Love wing bell) ab#1 — success zone constant ability
///
/// Card text:
///   常時 このカードが自分の成功ライブカード置き場にあるかぎり、
///   自分のセンターエリアにいる『μ's』のメンバーはブレードを得る。
///
/// While this card is in your success live card zone, your center area
/// μ's member gains blade.
use crate::helpers::*;

/// The card must have a 常時 ability with an effect (the success-zone constant).
#[test]
fn love_wing_bell_debug_card_abilities() {
    let db = load_real_database();

    let love_wing = db.get_card_id("PL!-bp4-020-L").expect("card exists");
    let card = db.get_card(love_wing).unwrap();
    let constants: Vec<_> = card
        .resolved_abilities()
        .filter(|ab| ab.triggers.as_deref() == Some("常時"))
        .collect();
    assert!(
        !constants.is_empty(),
        "Love wing bell must have a 常時 ability"
    );
    assert!(
        constants.iter().any(|ab| ab.effect.is_some()),
        "the 常時 ability must have a parsed effect"
    );
}

/// Love wing bell in success zone + μ's member in center → gets blade +1
#[test]
fn love_wing_bell_grants_blade_to_center_mus_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_center = game.id("PL!-PR-001-PR"); // μ's/Printemps member
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [filler, muse_center, -1];

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_center);
    assert_eq!(
        blade, 1,
        "Center μ's member should get blade+1 from Love wing bell, got {blade}"
    );
}

/// Love wing bell in success zone + non-μ's member in center → no blade
#[test]
fn love_wing_bell_non_mus_center_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let non_mus_center = game.id("PL!S-bp2-008-P"); // Aqours member, not μ's
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [filler, non_mus_center, -1];

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(non_mus_center);
    assert_eq!(
        blade, 0,
        "Non-μ's center member should NOT get blade, got {blade}"
    );
}

/// Love wing bell in success zone + μ's member in LEFT → no blade
#[test]
fn love_wing_bell_mus_left_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_left = game.id("PL!-sd1-010-SD"); // μ's member in left
    let filler = game.id("PL!-PR-001-PR"); // μ's member in center (not targeted)

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [muse_left, filler, -1];

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_left);
    assert_eq!(
        blade, 0,
        "Left μ's member should NOT get blade (only center), got {blade}"
    );
    // Center μ's member should also not get blade (only center is targeted,
    // but the effect grants to center, so actually center WOULD get blade.
    // This test just verifies left does not.)
}

/// Love wing bell in success zone + μ's member in RIGHT → no blade
#[test]
fn love_wing_bell_mus_right_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_right = game.id("PL!-sd1-010-SD"); // μ's member in right
    let center = game.id("PL!-PR-001-PR"); // μ's filler for center

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [-1, center, muse_right];

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_right);
    assert_eq!(
        blade, 0,
        "Right μ's member should NOT get blade (only center), got {blade}"
    );
}

/// Love wing bell NOT in success zone (in waitroom) → no blade
#[test]
fn love_wing_bell_not_in_success_zone_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_center = game.id("PL!-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Love wing bell in waitroom, NOT success zone
    game.state.player1.waitroom.cards.push(love_wing);
    game.state.player1.stage.stage = [filler, muse_center, -1];

    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_center);
    assert_eq!(
        blade, 0,
        "Love wing bell not in success zone → no blade, got {blade}"
    );
}

/// Remove Love wing bell from success zone → blade removed (as_long_as expiry)
#[test]
fn love_wing_bell_removed_from_success_zone_loses_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_center = game.id("PL!-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [filler, muse_center, -1];

    // Initially: blade should be granted
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(muse_center),
        1,
        "Initial state: blade should be 1"
    );

    // Remove from success zone
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_center);
    assert_eq!(
        blade, 0,
        "After removal from success zone → blade should be 0, got {blade}"
    );
}

/// Move μ's member out of center → blade lost (no longer matches position filter)
#[test]
fn love_wing_bell_member_leaves_center_loses_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse_center = game.id("PL!-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [filler, muse_center, -1];

    // Initially in center → blade
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(muse_center),
        1,
        "Center: blade should be 1"
    );

    // Move to left side
    game.state.player1.stage.stage = [muse_center, filler, -1];
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(muse_center);
    assert_eq!(
        blade, 0,
        "After moving to left → blade should be 0, got {blade}"
    );
}

/// Both players have Love wing bell in success zone → each gets blade on own center
#[test]
fn love_wing_bell_both_players_get_own_center_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing_p1 = game.id("PL!-bp4-020-L");
    let love_wing_p2 = game.id("PL!-bp4-020-L");
    let muse_center_p1 = game.id("PL!-PR-001-PR");
    let muse_center_p2 = game.id("PL!-PR-001-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Both players set up
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing_p1);
    game.state.player1.stage.stage = [filler, muse_center_p1, -1];

    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(love_wing_p2);
    game.state.player2.stage.stage = [filler, muse_center_p2, -1];

    game.state.recalculate_constants();

    let blade_p1 = game.state.mods.get_blade_modifier(muse_center_p1);
    let blade_p2 = game.state.mods.get_blade_modifier(muse_center_p2);
    assert_eq!(
        blade_p1, 1,
        "Player 1 center μ's should get blade, got {blade_p1}"
    );
    assert_eq!(
        blade_p2, 1,
        "Player 2 center μ's should get blade, got {blade_p2}"
    );
}

/// Center only — left and right μ's members should NOT get blade
#[test]
fn love_wing_bell_only_center_gets_blade_not_left_or_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let left = game.id("PL!-sd1-010-SD");
    let center = game.id("PL!-PR-001-PR");
    let right = game.id("PL!-sd1-005-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    game.state.player1.stage.stage = [left, center, right];

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(left),
        0,
        "Left should NOT get blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(center),
        1,
        "Center SHOULD get blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(right),
        0,
        "Right should NOT get blade"
    );
}

/// Empty stage → evaluation is a no-op: no panic, and no blade granted to
/// anyone (there is no one to grant to). Adding a μ's member afterwards must
/// still grant the blade, proving the empty pass didn't wedge the modifiers.
#[test]
fn love_wing_bell_empty_stage_no_crash() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_wing = game.id("PL!-bp4-020-L");
    let muse = game.id("PL!-PR-001-PR");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(love_wing);
    // Empty stage
    game.state.player1.stage.stage = [-1, -1, -1];

    game.state.recalculate_constants();

    // Now put a μ's member at center and re-evaluate.
    game.state.player1.stage.stage[1] = muse;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(muse),
        1,
        "blade still granted after an earlier empty-stage evaluation"
    );
}

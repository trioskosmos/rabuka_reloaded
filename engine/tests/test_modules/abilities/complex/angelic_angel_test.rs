/// Tests for PL!-bp4-019-L (Angelic Angel) — constant score modifier in success zone
///
/// 常時: このカードが自分の成功ライブカード置き場にあり、かつ
/// 自分のステージに『μ's』のメンバーがいるかぎり、
/// 自分の成功ライブカード置き場にあるこのカードのスコアを＋５する。
use crate::helpers::*;

/// Ang. Angel in success zone + μ's on stage → gets +5 score on itself
#[test]
fn angelic_angel_in_success_with_mus_gets_plus5() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L"); // base score 4
    let honoka = game.id("PL!-sd1-010-SD"); // μ's member

    // Angelic Angel in success zone
    game.state.player1.success_live_card_zone.cards.push(angel);
    // μ's member on stage
    game.state.player1.stage.stage = [-1, honoka, -1];

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(angel);
    assert_eq!(
        score_mod, 5,
        "Angelic Angel should get +5 score in success zone"
    );
}

/// Ang. Angel in success zone + NO μ's on stage → no modifier
#[test]
fn angelic_angel_no_mus_on_stage_no_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L");

    game.state.player1.success_live_card_zone.cards.push(angel);
    // Stage has no μ's member (non-μ's filler)
    game.state.player1.stage.stage = [-1, -1, -1];

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(angel);
    assert_eq!(score_mod, 0, "no +5 when no μ's on stage");
}

/// Ang. Angel in live set zone (NOT success zone) → ability not evaluated, no bleed
#[test]
fn angelic_angel_in_live_set_zone_not_success_no_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L");
    let honoka = game.id("PL!-sd1-010-SD");

    // Angelic Angel in live set zone (NOT success zone)
    game.state.player1.live_card_zone.cards.push(angel);
    // μ's member on stage
    game.state.player1.stage.stage = [-1, honoka, -1];

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(angel);
    assert_eq!(
        score_mod, 0,
        "Angelic Angel in live set zone should NOT get +5"
    );

    // Also verify the live set zone card itself has no modifier
    // (should only have its base score 4, no +5)
    let base_score = game.db.get_card(angel).unwrap().get_score() as i32;
    let effective = base_score + score_mod;
    assert_eq!(
        effective, 4,
        "live set zone card effective score should be base 4, not 9"
    );
}

/// μ's leaves stage dynamically → +5 removed
#[test]
fn angelic_angel_mus_leaves_stage_loses_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L");
    let honoka = game.id("PL!-sd1-010-SD");

    game.state.player1.success_live_card_zone.cards.push(angel);
    game.state.player1.stage.stage = [-1, honoka, -1];

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_score_modifier(angel),
        5,
        "+5 initially with μ's on stage"
    );

    // Remove μ's from stage
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_score_modifier(angel),
        0,
        "mod removed after μ's leaves stage"
    );
}

/// Ang. Angel removed from success zone → +5 cleared
#[test]
fn angelic_angel_removed_from_success_clears_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L");
    let honoka = game.id("PL!-sd1-010-SD");

    game.state.player1.success_live_card_zone.cards.push(angel);
    game.state.player1.stage.stage = [-1, honoka, -1];

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_score_modifier(angel), 5, "+5 initially");

    // Remove from success zone
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_score_modifier(angel),
        0,
        "mod cleared after card leaves success zone"
    );
}

/// Live set zone cards should NOT receive the +5 bonus from Angelic Angel
/// in the success zone. This confirms no bleed from success to live set zone.
#[test]
fn angelic_angel_does_not_bleed_to_live_set_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L"); // in success zone
    let honoka = game.id("PL!-sd1-010-SD"); // μ's on stage
    let live_card = game.id("PL!-sd1-021-SD"); // a live card in live set zone (score=3)

    // Angelic Angel in success zone
    game.state.player1.success_live_card_zone.cards.push(angel);
    // μ's on stage
    game.state.player1.stage.stage = [-1, honoka, -1];
    // Another live card set for the live
    game.state.player1.live_card_zone.cards.push(live_card);

    game.state.recalculate_constants();

    // Angelic Angel in success zone gets +5 (self-targeted)
    assert_eq!(
        game.state.mods.get_score_modifier(angel),
        5,
        "Angelic Angel gets +5 in success zone"
    );

    // Live set zone card should NOT get the +5
    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "live set zone card should NOT get +5 from Angelic Angel"
    );
}

/// Compound "and" condition: both must be met. Only location condition
/// (in success zone) → but NO μ's on stage → no modifier.
#[test]
fn angelic_angel_compound_and_both_required() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let angel = game.id("PL!-bp4-019-L");

    // Angel in success zone ✓
    game.state.player1.success_live_card_zone.cards.push(angel);
    // But empty stage ✗ (no μ's)
    game.state.player1.stage.stage = [-1, -1, -1];

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_score_modifier(angel),
        0,
        "both conditions must be met: missing μ's on stage"
    );
}

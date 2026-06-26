/// Tests for PL!-bp6-022-L (Dreamin' Go! Go!!) — modify_required_hearts with
/// original_value filter (13-key ability).
///
/// Card text:
///   常時 このカードが自分の成功ライブカード置き場にあるかぎり、
///   自分の元々のスコアが５以上の『μ's』のライブカードの
///   必要ハートをheart00heart00減らす。この効果は重複しない。
///
/// Translation:
///   Ongoing: As long as this card is in your success live card zone,
///   reduce heart00 requirement by 2 for your μ's live cards with
///   original score ≥ 5. Non-stackable.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// μ's live card with score ≥ 5 in live_card_zone → heart00 is reduced by 2
#[test]
fn high_score_mus_live_gets_heart_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let high_score = game.id("PL!-bp3-021-L"); // μ's, score 6

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state.player1.live_card_zone.cards.push(high_score);

    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        mod_val, -2,
        "score 6 μ's live → heart00 should be -2, got {mod_val}"
    );
}

/// μ's live card with score < 5 in live_card_zone → NO heart reduction
#[test]
fn low_score_mus_live_gets_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let low_score = game.id("PL!-sd1-020-SD"); // μ's, score 2

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state.player1.live_card_zone.cards.push(low_score);

    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(low_score, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "score 2 μ's live → should have no modifier, got {mod_val}"
    );
}

/// Both high and low score μ's live cards present → only high score gets reduction
#[test]
fn mixed_scores_filter_correctly() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let high_score = game.id("PL!-bp3-021-L"); // μ's, score 6
    let low_score = game.id("PL!-sd1-020-SD"); // μ's, score 2

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state.player1.live_card_zone.cards.push(high_score);
    game.state.player1.live_card_zone.cards.push(low_score);

    game.state.evaluate_success_zone_constant_abilities();

    let high_mod = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        high_mod, -2,
        "score 6 μ's live → heart00 should be -2, got {high_mod}"
    );

    let low_mod = game
        .state
        .mods
        .get_need_heart_modifier(low_score, HeartColor::Heart00);
    assert_eq!(
        low_mod, 0,
        "score 2 μ's live → should have no modifier, got {low_mod}"
    );
}

/// Non-μ's live card with score ≥ 5 → NO heart reduction (group filter)
#[test]
fn non_mus_high_score_gets_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let non_mus = game.id("PL!S-PR-024-PR"); // Aqours, score 5

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state.player1.live_card_zone.cards.push(non_mus);

    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(non_mus, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "Aqours score 5 live → should have no modifier, got {mod_val}"
    );
}

/// Activator NOT in success_live_card_zone → condition fails → no effect
#[test]
fn activator_not_in_success_zone_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let high_score = game.id("PL!-bp3-021-L"); // μ's, score 6

    // DON'T place activator in success zone — put it in live_card_zone instead
    game.state.player1.live_card_zone.cards.push(activator);
    game.state.player1.live_card_zone.cards.push(high_score);

    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "Activator not in success zone → no modifier, got {mod_val}"
    );
}

/// Activator in success zone, but no live cards at all → no crash, no-modifiers
#[test]
fn no_live_cards_no_crash() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);

    game.state.evaluate_success_zone_constant_abilities();

    // live_card_zone is empty — no modifiers should be applied
    assert!(
        game.state.mods.need_heart_modifiers.is_empty(),
        "No live cards → no need_heart modifiers should exist"
    );
}

/// Non-stackable: two copies of the 13-key card in success zone → only -2 applied (not -4)
#[test]
fn non_stackable_duplicate_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let high_score = game.id("PL!-bp3-021-L"); // μ's, score 6

    // Two copies of the 13-key card in success zone
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(game.id("PL!-bp6-022-L"));
    game.state.player1.live_card_zone.cards.push(high_score);

    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        mod_val, -2,
        "Two copies non-stackable → heart00 should be -2 (not -4), got {mod_val}"
    );
}

/// as_long_as expiration: when activator leaves success zone, re-evaluation produces no modifier
#[test]
fn as_long_as_expiration_on_zone_exit() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let activator = game.id("PL!-bp6-022-L");
    let high_score = game.id("PL!-bp3-021-L"); // μ's, score 6

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(activator);
    game.state.player1.live_card_zone.cards.push(high_score);

    // First evaluation: activator IS in success zone → reduction applied
    game.state.evaluate_success_zone_constant_abilities();

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        mod_val, -2,
        "Before removal → heart00 should be -2, got {mod_val}"
    );

    // Remove activator from success zone
    game.state.player1.success_live_card_zone.cards.clear();

    // Re-evaluate: no activator in zone → no reduction
    // Note: we must also clear the local non_stackable tracker, which
    // a fresh evaluate call does automatically.
    game.state.evaluate_success_zone_constant_abilities();

    let mod_val2 = game
        .state
        .mods
        .get_need_heart_modifier(high_score, HeartColor::Heart00);
    assert_eq!(
        mod_val2, 0,
        "After removal → heart00 should be 0 (no activator), got {mod_val2}"
    );
}

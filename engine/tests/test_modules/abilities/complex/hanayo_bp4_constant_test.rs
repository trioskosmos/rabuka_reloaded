/// Tests for PL!-bp4-008-R / PL!-bp4-008-P (小泉花陽 / Hanayo Koizumi)
///
/// 常時: 自分の成功ライブカード置き場にあるカードのスコアの合計が
/// ６以上であるかぎり、ステージにいるこのメンバーのコストを＋３する。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper: place a live card with score=3 in success zone (returns the card id)
fn add_score3_live(game: &mut TestGame) -> i16 {
    let live = game.id("PL!-sd1-021-SD");
    game.state.player1.success_live_card_zone.cards.push(live);
    live
}

/// Score=3 (<6) → no cost modifier
#[test]
fn hanayo_bp4_below_threshold_no_cost_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game); // total = 3

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "score=3 (<6) should have no cost modifier"
    );
}

/// Score=6 (= threshold) → +3 cost modifier
#[test]
fn hanayo_bp4_at_threshold_has_cost_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "score=6 (>=6) should have +3 cost modifier"
    );
}

/// Score=9 (> threshold) → +3 cost modifier
#[test]
fn hanayo_bp4_above_threshold_has_cost_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 9

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "score=9 (>=6) should have +3 cost modifier"
    );
}

/// Dynamic: score increases from 3 → 6
#[test]
fn hanayo_bp4_dynamic_increase() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game); // total = 3

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "initially no mod at score=3"
    );

    add_score3_live(&mut game); // total = 6
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "+3 mod after score reaches 6"
    );
}

/// Dynamic: score drops from 6 → 3
#[test]
fn hanayo_bp4_dynamic_decrease() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "initially +3 mod at score=6"
    );

    game.state.player1.success_live_card_zone.cards.pop(); // total = 3
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "mod removed after score drops to 3"
    );
}

/// Hanayo leaves stage → modifier cleared
#[test]
fn hanayo_bp4_removed_from_stage_clears_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [-1, hanayo, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "+3 mod while on stage"
    );

    game.state.player1.stage.stage[1] = -1;
    game.state.mods.clear_all_for_card(hanayo);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "mod cleared after leaving stage"
    );
}

/// Hanayo not on stage → no modifier (stage-only effect)
#[test]
fn hanayo_bp4_not_on_stage_no_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6, but Hanayo not on stage

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "no mod when Hanayo is not on stage"
    );
}

/// Play cost from hand is base cost 4 (modifier is on-stage only)
#[test]
fn hanayo_bp4_play_cost_unaffected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");

    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.add_to_hand(hanayo);
    game.give_energy(10);

    game.play_to_stage(hanayo, MemberArea::Center);

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining, 6,
        "played for base cost 4 (modifier is on-stage only)"
    );

    // After playing, on-stage modifier should be active
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "+3 mod active after Hanayo is on stage"
    );
}

/// Two copies of Hanayo on stage: each gets independent +3
#[test]
fn hanayo_bp4_two_copies_each_gets_mod() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo_a = game.id("PL!-bp4-008-R");
    let hanayo_b = game.id("PL!-bp4-008-R");

    game.state.player1.stage.stage = [hanayo_a, hanayo_b, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo_a),
        3,
        "copy A gets +3"
    );
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo_b),
        3,
        "copy B gets +3"
    );
}

/// Non-stackable: two copies' effects DON'T stack (each gets +3, not +6)
/// Actually, the ability is NOT marked non_stackable, so they DO stack per-card.
/// Each card independently checks the condition and adds +3 to itself.
/// The effect targets "this member", so each Hanayo gets +3 to its own cost.
#[test]
fn hanayo_bp4_non_stackable_per_card_not_shared() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-bp4-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, hanayo, -1];
    add_score3_live(&mut game);
    add_score3_live(&mut game); // total = 6

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(filler),
        0,
        "filler card should not get cost mod"
    );
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "Hanayo gets +3 cost"
    );
}

/// Baton touch: when replacing Hanayo on stage, her effective cost includes the +3 modifier
/// Hanayo (base cost 4, +3 mod = 7 on stage) replaced by Honoka (base cost 11)
/// Cost to pay = 11 - 7 = 4 (not 11 - 4 = 7)
#[test]
fn hanayo_bp4_baton_touch_uses_modified_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp4-008-R"); // base cost 4, +3 mod = 7
    let arriver = game.id("PL!-sd1-001-SD"); // Honoka, cost 11

    // Place Hanayo on stage
    game.state.player1.stage.stage[1] = hanayo;

    // Add 2 score-3 live cards → total = 6, triggers +3 cost
    add_score3_live(&mut game);
    add_score3_live(&mut game);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        3,
        "Hanayo has +3 cost modifier before baton touch"
    );

    // Give enough energy for either cost calculation
    // Base-only: cost = 11 - 4 = 7 energy needed
    // With mod:  cost = 11 - 7 = 4 energy needed
    // Give 7 energy → base-only succeeds, with-mod also succeeds
    // Give 4 energy → base-only FAILS, with-mod succeeds
    // Let's give exactly 5 energy → with-mod passes (5 >= 4), base-only would fail (5 < 7)
    game.give_energy(5);

    game.state.player1.hand.cards.push(arriver);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Should have succeeded: 5 - 4 = 1 energy remaining
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining, 1,
        "paid 4 energy (11 - 7 = 4 with modifier), not 7 (11 - 4 = base only)"
    );
}

/// Baton touch scenario A: score=3 (<6) so no +3 mod → cost = 11 - 4 = 7
/// With only 5 energy, this should FAIL since 5 < 7
#[test]
fn hanayo_bp4_baton_touch_base_cost_when_below_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp4-008-R"); // base cost 4
    let arriver = game.id("PL!-sd1-001-SD"); // Honoka, cost 11

    game.state.player1.stage.stage[1] = hanayo;
    add_score3_live(&mut game); // total = 3 (< 6), no +3 mod

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(hanayo),
        0,
        "no cost modifier when score=3"
    );

    // 5 energy is enough for 11-4=7? No, 5 < 7. So this should fail.
    game.give_energy(5);
    game.state.player1.hand.cards.push(arriver);

    let result = game.try_play_to_stage(arriver, MemberArea::Center);
    assert!(
        result.is_err(),
        "baton touch should fail: need 7 energy but only have 5"
    );
}

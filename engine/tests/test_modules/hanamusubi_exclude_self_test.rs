/// Tests for ハナムスビ (PL!HS-bp5-019-L) — `modify_required_hearts` with
/// `exclude_self` + `per_unit`.
///
/// Ability:
///   ライブ開始時 自分のライブカード置き場にあるこのカード以外の『蓮ノ空』の
///   カード1枚につき、このカードの必要ハートをheart04×2減らす。
///
/// Translation:
///   On LiveStart: for each 蓮ノ空 (Renosora) card OTHER than this card in your
///   live card zone, reduce this card's required heart04 by 2.
///
/// Base requirement: heart04=9, heart0=5
/// Per other Renosora live card: heart04 -= 2
///
/// These tests verify the bug fix where the activating card was incorrectly
/// counting itself in the per_unit loop (giving -2 even when alone) and where
/// the per-unit formula computed `count * per_unit_count` instead of
/// `value * (count / per_unit_count)` (giving -1 per card instead of -2).
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Advance from the initial phase through to LiveCardSetP1.
fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass(); // → ActivePhase
    game.pass(); // → EnergyPhase
    game.pass(); // → DrawPhase
    game.pass(); // → MainPhase
    game.pass(); // → LiveCardSetP1
}

/// Advance from LiveCardSetP1 through LiveStart (triggers fire here).
fn finish_live_setup(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart (triggers fire here)
                 // Other Renosora live cards (e.g. PL!HS-bp5-017-L) have their own live_start
                 // abilities with optional energy costs that enqueue SelectAutoAbility choices.
                 // Drain those so hanamusubi's effect (which is cost-free) runs.
    game.drain_auto_ability_choices();
}

// ─────────────────────────────────────────────────────────────
// Test 1: Hanamusubi is the ONLY card in the live card zone
// → exclude_self means count = 0, so NO heart reduction.
// BUG: previously gave -2 (counted itself).
// ─────────────────────────────────────────────────────────────
#[test]
fn hanamusubi_alone_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamusubi = game.id("PL!HS-bp5-019-L");
    let filler = game.id("PL!-sd1-010-SD"); // non-Renosora filler

    game.state.player1.stage.stage = [hanamusubi, filler, filler];
    game.state.player1.hand.cards.push(hanamusubi);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(hanamusubi);
    finish_live_setup(&mut game);

    // Hanamusubi is the only live card. exclude_self=true → it does not count
    // itself → 0 other Renosora cards → no reduction.
    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart04);
    assert_eq!(
        mod_val, 0,
        "hanamusubi alone should get 0 reduction (exclude_self), got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 2: Hanamusubi + 1 other Renosora live card
// → count = 1, value = 2 * 1 = 2 → heart04 -= 2.
// ─────────────────────────────────────────────────────────────
#[test]
fn hanamusubi_with_one_other_renosora_reduces_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamusubi = game.id("PL!HS-bp5-019-L");
    let other_renosora = game.id("PL!HS-bp5-017-L"); // 蓮ノ空 live card
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hanamusubi, filler, filler];
    game.state.player1.hand.cards.push(hanamusubi);
    game.state.player1.hand.cards.push(other_renosora);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(hanamusubi);
    game.set_live_card(other_renosora);
    finish_live_setup(&mut game);

    // 1 other Renosora card → 2 * 1 = 2 reduction on heart04.
    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart04);
    assert_eq!(
        mod_val, -2,
        "1 other Renosora → heart04 should be -2, got {mod_val}"
    );

    // The other Renosora card itself should NOT be modified by hanamusubi's
    // self-targeted ability (target=self).
    let other_mod = game
        .state
        .mods
        .get_need_heart_modifier(other_renosora, HeartColor::Heart04);
    assert_eq!(
        other_mod, 0,
        "other Renosora card should not be modified (target=self), got {other_mod}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 3: Hanamusubi + 2 other Renosora live cards
// → count = 2, value = 2 * 2 = 4 → heart04 -= 4.
// This also verifies the per-unit value formula: 2 per card, not 1.
// ─────────────────────────────────────────────────────────────
#[test]
fn hanamusubi_with_two_other_renosora_reduces_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamusubi = game.id("PL!HS-bp5-019-L");
    let other_a = game.id("PL!HS-bp5-017-L"); // 蓮ノ空 live card
    let other_b = game.id("PL!HS-bp5-018-L"); // 蓮ノ空 live card
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hanamusubi, filler, filler];
    game.state.player1.hand.cards.push(hanamusubi);
    game.state.player1.hand.cards.push(other_a);
    game.state.player1.hand.cards.push(other_b);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(hanamusubi);
    game.set_live_card(other_a);
    game.set_live_card(other_b);
    finish_live_setup(&mut game);

    // 2 other Renosora cards → 2 * 2 = 4 reduction on heart04.
    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart04);
    assert_eq!(
        mod_val, -4,
        "2 other Renosora → heart04 should be -4 (2 per card), got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 4: Hanamusubi + non-Renosora live card
// → the non-Renosora card does NOT count → 0 reduction.
// Verifies the group_name filter still works alongside exclude_self.
// ─────────────────────────────────────────────────────────────
#[test]
fn hanamusubi_with_non_renosora_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamusubi = game.id("PL!HS-bp5-019-L");
    let non_renosora = game.id("PL!-sd1-019-SD"); // μ's live card (non-Renosora)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hanamusubi, filler, filler];
    game.state.player1.hand.cards.push(hanamusubi);
    game.state.player1.hand.cards.push(non_renosora);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(hanamusubi);
    game.set_live_card(non_renosora);
    finish_live_setup(&mut game);

    // Non-Renosora card doesn't match group filter → 0 other Renosora → no reduction.
    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart04);
    assert_eq!(
        mod_val, 0,
        "non-Renosora card should not count → 0 reduction, got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 5: Only heart04 is reduced, not heart0.
// Hanamusubi base needs heart04=9, heart0=5. The ability only reduces heart04.
// ─────────────────────────────────────────────────────────────
#[test]
fn hanamusubi_reduces_only_heart04_not_heart0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamusubi = game.id("PL!HS-bp5-019-L");
    let other_renosora = game.id("PL!HS-bp5-017-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hanamusubi, filler, filler];
    game.state.player1.hand.cards.push(hanamusubi);
    game.state.player1.hand.cards.push(other_renosora);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(hanamusubi);
    game.set_live_card(other_renosora);
    finish_live_setup(&mut game);

    let heart04_mod = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart04);
    assert_eq!(
        heart04_mod, -2,
        "heart04 should be reduced by 2, got {heart04_mod}"
    );

    // heart0 (Heart00) should be untouched.
    let heart0_mod = game
        .state
        .mods
        .get_need_heart_modifier(hanamusubi, HeartColor::Heart00);
    assert_eq!(
        heart0_mod, 0,
        "heart0 should NOT be reduced, got {heart0_mod}"
    );
}

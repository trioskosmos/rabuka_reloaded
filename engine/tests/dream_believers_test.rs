/// Q212: Dream Believers (PL!HS-bp5-017-L) — LiveStart: 2+ members including
/// ≥1 蓮ノ空, with distinct unit names. Multi-name card (渡辺曜&鬼塚夏美&大沢瑠璃乃)
/// should NOT count as having a 蓮ノ空 member per Q212.
mod helpers;
use helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Stage: 大沢瑠璃乃 (蓮ノ空) + filler (non-蓮ノ空). Total=2 members, 1 蓮ノ空.
/// Distinct names: 大沢瑠璃乃 ≠ filler name → passes.
/// Condition requires ≥2 members including ≥1 蓮ノ空 → should pass.
#[test]
fn dream_believers_one_hasetsu_plus_other_pass() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dream = game.id("PL!HS-bp5-017-L");
    let filler = game.id("PL!-sd1-010-SD");
    let rurino = game.id("PL!HS-bp1-005-P"); // 大沢瑠璃乃, 蓮ノ空

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [rurino, filler, -1];
    game.state.player1.hand.cards.push(dream);
    game.state.player1.hand.cards.push(filler);

    game.give_energy(1);
    advance_to_live_set(&mut game);
    game.set_live_card(dream);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_option(1);
        if game.has_pending_choice() { game.select_indices(&[]); }
    }

    let mod_val = game.state.mods.get_score_modifier(dream);
    eprintln!("[DREAM] 1 hasetsu + 1 other: mod={}", mod_val);
    // Should be 1 (2 members, 1 is 蓮ノ空, distinct names)
    assert_eq!(mod_val, 1, "Condition passes with 1 蓮ノ空 + 1 non-蓮ノ空");
}

/// Stage: 大沢瑠璃乃 (蓮ノ空) + multi-name card. The multi card should NOT
/// count as 蓮ノ空 (Q212). Total=2 members but only 1 is 蓮ノ空? Or
/// the distinct check fails (大沢瑠璃乃 in both). Either way → NO.
#[test]
fn dream_believers_q212_multiname_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dream = game.id("PL!HS-bp5-017-L");
    let filler = game.id("PL!-sd1-010-SD");
    let rurino = game.id("PL!HS-bp1-005-P"); // 大沢瑠璃乃, 蓮ノ空
    let multi = game.id("LL-bp2-001-R\u{ff0b}");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [rurino, multi, filler];
    game.state.player1.hand.cards.push(dream);
    game.state.player1.hand.cards.push(filler);

    game.give_energy(1);
    advance_to_live_set(&mut game);
    game.set_live_card(dream);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_option(1);
        if game.has_pending_choice() { game.select_indices(&[]); }
    }

    let mod_val = game.state.mods.get_score_modifier(dream);
    eprintln!("[DREAM] rurino + multi: mod={} (expected 0)", mod_val);
    // Q212: condition should NOT apply
    assert_eq!(mod_val, 0, "Q212: multi-name card breaks condition");
}

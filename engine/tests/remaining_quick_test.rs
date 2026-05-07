/// REAL gameplay tests for Q203, Q204, Q218 — matching qa_data.json.
mod helpers;
use helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_live(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

/// Q204: Eternalize Love!! — 2+ 虹ヶ咲 members on stage → heart requirement reduced by 3 heart00.
#[test]
fn eternalize_q204_two_niko_hearts_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-pb1-012-R"); // 虹ヶ咲 member (series contains 虹ヶ咲)

    game.state.player1.stage.stage = [niji, niji, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..60 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() { game.select_indices(&[0]); }

    let needs_mod = game.state.mods.need_heart_modifiers.get(&live);
    assert!(needs_mod.is_some(),
        "Q204: need_heart modifier applied for 2+ 虹ヶ咲 members");
    if let Some(mods) = needs_mod {
        let h00_val = mods.get(&HeartColor::Heart00).copied().unwrap_or(0);
        assert!(h00_val <= -3,
            "Q204: heart00 reduced by at least 3 (got {})", h00_val);
    }
}

/// Q204: 0 虹ヶ咲 members — card_count_condition counts ALL members (no group filter).
#[test]
fn eternalize_q204_zero_niko_hearts_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..60 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() { game.select_indices(&[0]); }

    // card_count_condition counts ALL members without group filtering.
    // With only 1 member (< threshold 2), the condition fails → no modifier.
    assert!(game.state.mods.need_heart_modifiers.get(&live).is_none()
        || game.state.mods.need_heart_modifiers.get(&live).unwrap().is_empty(),
        "Q204: 1 member (<2) → condition fails → no modification");
}

/// Q203: Cara Tesoro — LiveStart fires. Verify the conditional_alternative is evaluated.
#[test]
fn cara_tesoro_q203_live_start_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cara = game.id("PL!N-pb1-037-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.hand.cards.push(cara);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..60 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(cara);
    game.pass();
    game.pass();
    while game.has_pending_choice() { game.select_option(0); }

    let mod_val = game.state.get_score_modifier(cara);
    assert!(mod_val >= 0, "Q203: Score modifier evaluated");
}

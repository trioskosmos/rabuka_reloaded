/// Tests for 園田海未 (PL!-bp3-013-N):
///
/// ab#0 (ライブ開始時): Choose one of heart01/heart03/heart06.
///   Until live end, gain 1 of the chosen heart per card in success live zone.
///
/// Bug: The heart color select choice caused a softlock because the frontend
/// didn't render plain string options. Fixed in ChoiceView.js.
use crate::helpers::*;

/// Umi's ability: at live start, select a heart color, then gain
/// 1 of that heart per card in success live zone for the rest of the live.
#[test]
fn umi_bp3_live_start_select_heart_and_scale_with_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let umi = game.id("PL!-bp3-013-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[1] = umi;
    // Set 3 success zone cards so the per-unit scaling gives 3 hearts
    for _ in 0..3 {
        game.state.player1.success_live_card_zone.cards.push(filler);
    }
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;

    // Advance until live start triggers the pending choice.
    // Stop passing as soon as pending_choice is set (next pass would auto-resolve it).
    for _ in 0..20 {
        if game.has_pending_choice() {
            break;
        }
        game.pass();
    }

    assert!(
        game.has_pending_choice(),
        "Umi ability should create a heart color choice at live start"
    );

    // Resolve the first choice (heart color select — SelectTarget option)
    game.select_option(0);
    // The heart gain resolves directly after the color choice; no card selection appears.
    assert!(
        !game.has_pending_choice(),
        "per-unit heart gain must resolve without a further prompt"
    );

    // The ability should have granted 3 heart01 (1 per success zone card)
    // The heart modifier is on Umi's card via gs.mods, not stage_hearts (which is None outside live performance)
    let heart01_mod = game
        .state
        .mods
        .get_heart_modifier(umi, rabuka_engine::card::HeartColor::Heart01);
    assert_eq!(
        heart01_mod, 3,
        "Gained 3 heart01 from Umi's ability (1 per 3 success zone cards)"
    );
}

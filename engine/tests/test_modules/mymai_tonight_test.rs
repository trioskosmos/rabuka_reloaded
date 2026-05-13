/// Tests for MY 舞 TONIGHT (PL!S-bp2-023-L) — LiveStart: give blade to ALL stage members.
///
/// Q121: Blade is gained by ALL stage members, not just one.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn mymai_tonight_q121_blade_given_to_all_stage_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let mymai = game.id("PL!S-bp2-023-L");
    // Another Aqours live card
    let aqours_live = game.id("LL-bp5-002-L"); // Bring the LOVE! is Aqours

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Two members on stage to receive blade
    game.state.player1.stage.stage = [mymai, filler, filler]; // MY舞TONIGHT is itself a member AND live card

    // Wait, MY舞TONIGHT is a live card (type=ライブ), not a member (type=メンバー)
    // It can't be placed on stage. Let me use actual member cards.
    // Actually it's a live card with need_heart: {...}
    // I need members on stage for the blade to apply to
    let member_a = game.id("PL!S-sd1-001-SD");
    let member_b = game.id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [member_a, member_b, -1];

    game.state.player1.hand.cards.push(mymai);
    game.state.player1.hand.cards.push(aqours_live);

    advance_to_live_set(&mut game);
    game.set_live_card(mymai);
    game.set_live_card(aqours_live);

    game.pass(); // P1Turn draw
    game.pass(); // P2Turn → FirstAttackerPerformance → LiveStart

    // Handle MY舞TONIGHT's LiveStart ability
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Verify blade modifiers were applied to both stage members
    let bm = &game.state.mods.blade_modifiers;
    eprintln!("[MYMAI] blade modifiers: {:?}", bm);
    let has_a = bm.get(&member_a).copied().unwrap_or(0) > 0;
    let has_b = bm.get(&member_b).copied().unwrap_or(0) > 0;
    eprintln!(
        "[MYMAI] member_a has blade: {}, member_b has blade: {}",
        has_a, has_b
    );
    assert!(
        has_a && has_b,
        "Both stage members should have blade from LiveStart"
    );
}

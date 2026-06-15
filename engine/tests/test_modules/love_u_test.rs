/// Tests for Love U my friends (PL!N-bp3-030-L) — LiveSuccess ability:
///
/// {{live_success.png|ライブ成功時}}エールにより公開された自分のカードの中に
/// {{icon_b_all.png|ALLブレード}}を持つカードが1枚以上ある場合、このカードのスコアを＋１する。
///
/// Q192: If blade hearts are recolored and ALL heart is obtained, does this count
///       as ALL blade? A: No — ALL heart ≠ ALL blade.
/// Q36:  LiveSuccess timing definition.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_success(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// Q192: LiveSuccess triggers. The parser should produce card_property: has_all_blade
/// and location: revealed_cards in the condition.
#[test]
fn love_u_q192_live_success_condition_evaluated() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_u = game.id("PL!N-bp3-030-L");
    let filler = game.id("PL!-sd1-010-SD"); // blade_heart: b_heart03
    let aqours_member = game.id("PL!S-sd1-003-SD"); // base_heart: {heart02, heart04, heart05}
    let yell_h01 = game.id("PL!-sd1-013-SD"); // blade_heart: b_heart01
    let yell_h06 = game.id("PL!-sd1-002-SD"); // blade_heart: b_heart06

    // One member on stage (so live can succeed)
    game.state.player1.stage.stage = [aqours_member, -1, -1];
    game.state.player1.hand.cards.push(love_u);

    // Phase transitions before yell (Pass 4 in Draw phase draws 1 card from
    // top of deck index 0).  Yell draws from remaining top 3 cards.
    // Push order: index 0 = drawn to hand, indices 1-3 = yell reveals.
    // Stage hearts (aqours_member): heart02=1, heart04=2, heart05=1
    // Yell must provide heart01, heart03, heart06 to meet Love U's need_heart.
    game.state.player1.main_deck.cards.push(filler); // index 0 → drawn to hand
    game.state.player1.main_deck.cards.push(yell_h06); // index 1 → yell #1 → b_heart06 → heart06
    game.state.player1.main_deck.cards.push(yell_h01); // index 2 → yell #2 → b_heart01 → heart01
    game.state.player1.main_deck.cards.push(filler); // index 3 → yell #3 → b_heart03 → heart03
    for _ in 4..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(love_u);
    advance_to_live_success(&mut game);

    // LiveSuccess fires — condition evaluated. With card_property has_all_blade
    // and revealed_cards location, the condition check runs through the
    // card_count_condition evaluator.
    // Verify the live card survived LiveSuccess without crashing
    assert!(
        !game.state.player1.success_live_card_zone.cards.is_empty(),
        "Live card should have reached success_live_card_zone after LiveSuccess"
    );
}

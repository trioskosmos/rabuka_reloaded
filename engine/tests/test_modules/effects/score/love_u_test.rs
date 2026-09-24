/// Tests for Love U my friends (PL!N-bp3-030-L) — LiveSuccess ability:
///
/// {{live_success.png|ライブ成功時}}エールにより公開された自分のカードの中に
/// {{icon_b_all.png|ALLブレード}}を持つカードが1枚以上ある場合、このカードのスコアを＋１する。
///
/// Q192: If blade hearts are recolored and ALL heart is obtained, does this count
///       as ALL blade? A: No — ALL heart ≠ ALL blade.
/// Q36:  LiveSuccess timing definition.
use crate::helpers::*;
use rabuka_engine::core::card::{BaseHeart, BladeColor, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

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

/// Q192: LiveSuccess triggers. With a b_all card among yell-revealed cards,
/// the condition should pass and the score should be +1.
#[test]
fn love_u_q192_live_success_all_blade_score_up() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let love_u = game.id("PL!N-bp3-030-L");
    let filler = game.id("PL!-sd1-010-SD"); // blade_heart: b_heart03
    let aqours_member = game.id("PL!S-sd1-003-SD"); // blade: 3, blade_heart: b_heart04, base_heart: {heart02, heart04, heart05}
    let yell_h06 = game.id("PL!-sd1-002-SD"); // blade_heart: b_heart06
    let b_all_card = game.id("PL!-sd1-020-SD"); // blade_heart: {b_all: 1}

    // One member on stage (so live can succeed)
    game.state.player1.stage.stage = [aqours_member, -1, -1];
    game.state.player1.hand.cards.push(love_u);

    // Phase transitions before yell (Pass 4 in Draw phase draws 1 card from
    // top of deck index 0).  Yell draws from remaining top 3 cards (blade=3).
    // Push order: index 0 = drawn to hand, indices 1-3 = yell reveals.
    // Stage hearts (aqours_member): heart02=1, heart04=2, heart05=1
    // Yell must provide heart01, heart03, heart06 to meet Love U's need_heart.
    // The b_all card provides a wildcard heart00 that can cover heart01.
    game.state.player1.main_deck.cards.push(filler); // index 0 → drawn to hand
    game.state.player1.main_deck.cards.push(yell_h06); // index 1 → yell #1 → b_heart06 → heart06
    game.state.player1.main_deck.cards.push(b_all_card); // index 2 → yell #2 → b_all → heart00
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

    // LiveSuccess fires — condition evaluated. A b_all card was among yell-revealed
    // cards, so has_all_blade condition should pass and score should be +1.
    assert!(
        !game.state.player1.success_live_card_zone.cards.is_empty(),
        "Live card should have reached success_live_card_zone after LiveSuccess"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(love_u),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
}

#[test]
fn love_u_q192_recolored_all_blade_does_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let love_u = game.new_id("PL!N-bp3-030-L");
    let b_all = game.new_id("PL!-sd1-020-SD");
    let stage_member = game.new_id("PL!HS-pb1-023-N");

    game.state.player1.stage.stage = [stage_member, -1, -1];
    let mut stage_hearts = BaseHeart {
        hearts: HeartMap::new(),
    };
    for color in [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ] {
        stage_hearts.hearts.insert(color, 1);
    }
    game.state.player1.stage_hearts = Some(stage_hearts);
    game.state.player1.live_card_zone.cards.push(love_u);
    game.state.revealed_cards.push(b_all);
    game.state
        .mods
        .set_blade_type_modifier(stage_member, BladeColor::Purple);
    game.state.current_phase = Phase::LiveVictoryDetermination;

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(game.state.mods.get_score_modifier(love_u), 0);
}

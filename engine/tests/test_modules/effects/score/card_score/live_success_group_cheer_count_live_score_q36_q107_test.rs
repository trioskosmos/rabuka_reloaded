/// Tests for PL!HS-bp1-022-L (AWOKE) ab#0 — Q107, Q36
///
/// LiveSuccess: エールにより公開されたカードの中に『蓮ノ空』の
///   メンバーカードが10枚以上ある場合、スコア+1。
///
/// Q107: Re-yell only counts second yell's cards
/// Q36: LiveSuccess timing
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// 3 members with heart05. Deck is 蓮ノ空 cards WITH blade_heart for wildcard.
/// Cheers 10+ 蓮ノ空 → LiveSuccess condition met → score +1.
#[test]
fn awake_q36_10_plus_hasetsu_cheers_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let awake = game.id("PL!HS-bp1-022-L");
    let filler = game.id("PL!-sd1-010-SD");
    // Stage: 3 members with heart05=2 each → heart05=6 (meets requirement)
    // PL!S-PR-014-PR: base heart05=2, blade=6 → 3× = blade=18
    let heart_member = game.id("PL!S-PR-014-PR");

    game.state.player1.stage.stage = [heart_member, heart_member, heart_member];
    game.state.player1.hand.cards.push(awake);
    game.state.player1.hand.cards.push(filler);

    // Deck: 蓮ノ空 cards WITH blade_heart so cheered cards contribute to wildcard.
    // PL!HS-PR-020-PR: 蓮ノ空 member, blade_heart={'b_heart05': 1}, blade=6
    let hasetsu = game.id("PL!HS-PR-020-PR");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(hasetsu);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(awake);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_score_modifier(awake),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
}

/// Only 1 member with blade=2 → <10 cheered → condition fails.
#[test]
fn awake_q36_low_cheers_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let awake = game.id("PL!HS-bp1-022-L");
    let filler = game.id("PL!-sd1-010-SD");
    let low = game.id("PL!HS-sd1-001-SD"); // blade=2

    game.state.player1.stage.stage = [low, -1, -1];
    game.state.player1.hand.cards.push(awake);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(awake);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_score_modifier(awake),
        0,
        "<10 cheered cards → no score"
    );
}

/// Non-蓮ノ空 cheered cards don't count toward the 10.
/// Deck has 0 蓮ノ空 members → condition fails even with 15+ cheered.
#[test]
fn awake_q36_non_hasetsu_cheered_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let awake = game.id("PL!HS-bp1-022-L");
    let filler = game.id("PL!-sd1-010-SD");
    let high_blade = game.id("PL!-sd1-009-SD"); // μ's, blade=5

    game.state.player1.stage.stage = [high_blade, high_blade, high_blade];
    game.state.player1.hand.cards.push(awake);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(awake);
    advance_to_live_start(&mut game);
    game.pass();
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_score_modifier(awake),
        0,
        "Non-蓮ノ空 cheered cards should NOT count toward 10"
    );
}

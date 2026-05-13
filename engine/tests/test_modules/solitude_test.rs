/// Tests for Solitude Rain (PL!N-bp1-027-L) — Q67: LiveStart modify_score
/// counts unique heart colors {01-06} across 虹ヶ咲 members. Heart00 (ALL)
/// does NOT count as an arbitrary color.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// A 虹ヶ咲 member with heart01 on stage → 1 unique color → score +1.
#[test]
fn solitude_q67_hasetsu_member_with_heart01_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let solitude = game.id("PL!N-bp1-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    // 虹ヶ咲 member with heart01 in base_heart
    // PL!N-sd1-001-SD (上原歩夢) — check her hearts
    let hasetsu = game.id("PL!N-sd1-001-SD"); // 上原歩夢, 虹ヶ咲

    // Both players need deck cards for draws
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // 虹ヶ咲 member on stage
    game.state.player1.stage.stage = [hasetsu, filler, filler];
    game.state.player1.hand.cards.push(solitude);

    advance_to_live_set(&mut game);
    game.set_live_card(solitude);

    // Pass through to trigger LiveStart
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score_mod = game.state.mods.get_score_modifier(solitude);
    eprintln!("[SOLITUDE] score_mod with heart01 member: {}", score_mod);
    // hasetsu has heart01 → 1 unique color → +1
    assert_eq!(
        score_mod, 1,
        "hasetsu member with heart01 should give score+1"
    );
    eprintln!("[SOLITUDE] Q67 validated: modify_score counts heart colors per member");
}

/// Non-虹ヶ咲 member on stage → no match → score 0.
#[test]
fn solitude_q67_non_hasetsu_member_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let solitude = game.id("PL!N-bp1-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    // μ's member (not 虹ヶ咲)
    let non_hasetsu = game.id("PL!-sd1-002-SD"); // 絢瀬絵里, μ's

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [non_hasetsu, filler, filler];
    game.state.player1.hand.cards.push(solitude);

    advance_to_live_set(&mut game);
    game.set_live_card(solitude);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score_mod = game.state.mods.get_score_modifier(solitude);
    eprintln!("[SOLITUDE] score_mod with non-虹ヶ咲: {}", score_mod);
    assert_eq!(score_mod, 0, "Non-虹ヶ咲 should not contribute score");
}

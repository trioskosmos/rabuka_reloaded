/// Tests for 螳ｮ荳・諢・(PL!N-bp3-005-R+) 窶・the ONLY draw_until_count card.
///
/// ab#0: 縺薙・繧ｿ繝ｼ繝ｳ縲∬・蛻・・繧ｹ繝・・繧ｸ縺ｫ繝｡繝ｳ繝舌・縺・蝗樒匳蝣ｴ縺励◆縺ｨ縺阪・///       謇区惆縺・譫壹↓縺ｪ繧九∪縺ｧ繧ｫ繝ｼ繝峨ｒ蠑輔￥縲・///   condition: temporal=this_turn, trigger_event=temporal_count(count=3),
///              location=stage, card_type=member_card, target=self
///   effect:    draw_until_count(target_count=5, source=deck, destination=hand)
///
/// Gameplay edge cases covered:
///   1. 3rd appearance this turn 竊・draws up to 5 cards in hand
///   2. Hand already >= 5 at 3rd appearance 竊・draws nothing
///   3. Only 2 appearances 竊・no draw
///   4. Deck smaller than needed 竊・draws out deck, resolves without crash
///   5. Draw amount is exact (hand deficit, not a fixed number)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MIYATA: &str = "PL!N-bp3-005-R+";
const KASUMI: &str = "PL!N-sd1-002-SD";
const SHIZUKU: &str = "PL!N-sd1-003-SD";
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_deck(game: &mut TestGame, n: usize) {
    let filler = game.id(FILLER);
    for _ in 0..n {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..n {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn drain(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        game.select_indices(&[]);
        guard += 1;
    }
}

/// Play miyata + kasumi + shizuku to the three stage slots.
/// Each play is one appearance; returns nothing.
fn three_appearances(game: &mut TestGame) {
    let miyata = game.id(MIYATA);
    let kasumi = game.id(KASUMI);
    let shizuku = game.id(SHIZUKU);
    game.state.player1.hand.cards.push(miyata);
    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(shizuku);

    game.play_to_stage(miyata, MemberArea::Center);
    drain(game);
    game.play_to_stage(kasumi, MemberArea::LeftSide);
    drain(game);
    game.play_to_stage(shizuku, MemberArea::RightSide);
    drain(game);
}

// =========================================================================
// 1. Happy path: 3rd appearance triggers draw-to-5.
//    Hand after 3 plays = 1 card 竊・draws exactly 4.
// =========================================================================
#[test]
fn third_appearance_draws_to_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.give_energy(60);
    fill_deck(&mut game, 10);

    // Extra hand card so hand after plays = 1 (miyata+kasumi+shizuku+extra,
    // three played away).
    let extra = game.new_id(FILLER);
    game.state.player1.hand.cards.push(extra);

    three_appearances(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Draw-until-5 should bring hand from 1 to exactly 5"
    );
}

// =========================================================================
// 2. Hand already >= 5 when the 3rd appearance happens 竊・no draw.
// =========================================================================
#[test]
fn hand_at_five_or_more_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.give_energy(60);
    fill_deck(&mut game, 10);

    // 6 extras + 3 members pushed by three_appearances − 3 played = 6 in
    // hand at the 3rd appearance → draw_until_count must no-op.
    for _ in 0..6 {
        let e = game.new_id(FILLER);
        game.state.player1.hand.cards.push(e);
    }

    three_appearances(&mut game);

    let after = game.state.player1.hand.cards.len();
    assert_eq!(
        after, 6,
        "Hand had 6 (>=5) at 3rd appearance; must stay 6, got {}",
        after
    );
}

// =========================================================================
// 3. Only 2 appearances this turn 竊・condition unmet, no draw.
// =========================================================================
#[test]
fn two_appearances_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.give_energy(60);
    fill_deck(&mut game, 10);

    let miyata = game.id(MIYATA);
    let kasumi = game.id(KASUMI);
    game.state.player1.hand.cards.push(miyata);
    game.state.player1.hand.cards.push(kasumi);
    let extra = game.new_id(FILLER);
    game.state.player1.hand.cards.push(extra);

    game.play_to_stage(miyata, MemberArea::Center);
    drain(&mut game);
    game.play_to_stage(kasumi, MemberArea::LeftSide);
    drain(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Two appearances must not trigger the draw"
    );
}

// =========================================================================
// 4. Deck smaller than the deficit 竊・draws out the deck, no crash/refresh
//    loop. Hand 0 after plays, deck 2 竊・draws exactly 2.
// =========================================================================
#[test]
fn small_deck_draws_out_and_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.give_energy(60);
    fill_deck(&mut game, 2); // only 2 cards to draw

    three_appearances(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Should draw the 2 remaining deck cards and stop"
    );
    assert!(
        game.state.player1.main_deck.cards.is_empty(),
        "Deck should be empty after drawing out"
    );
}

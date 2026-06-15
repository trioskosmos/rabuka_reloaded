/// Tests for 宮下 愛 BP03 (PL!N-bp3-005-P＋ / PL!N-bp3-005-SEC) — 自動:
/// このターン、自分のステージにメンバーが3枚登場したとき、手札が5枚になるまでカードを引く。
///
/// The ability is 自動 (auto) with a temporal condition that checks
/// debut_count_this_turn >= 3. When 3+ members have been deployed this turn,
/// draw_until_count (target=5) fires. No cost involved.
///
/// The same ability is on ab#0 of all four printings (R+, P, P+, SEC).
/// Only P+ is tested here.
///
/// All tests use full game flow: set up hand/deck → play_to_stage sequentially
/// → verify final state. Not just calling the helper directly.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper: set up a fresh game with the Ai BP03 card in hand, TWO filler
/// members already in hand (so calling code can deploy Ai + 2 fillers), and
/// `hand_extra` additional filler cards. Deck gets `deck_count` filler cards.
/// Returns (ai_id, filler_id, hand_len_before_any_play, deck_len_before, waitroom_len_before).
fn setup_ai_bp3(
    game: &mut TestGame,
    hand_extra: usize,
    deck_count: usize,
) -> (i16, i16, usize, usize, usize) {
    let ai = game.id("PL!N-bp3-005-P＋");
    let filler_member = game.id("PL!-sd1-010-SD");

    // Energy: ai cost 15 + 2*filler cost 4 = 23, give 30.
    game.give_energy(30);

    // Hand: ai + 2 filler members + hand_extra filler cards.
    game.add_to_hand(ai);
    game.add_to_hand(filler_member);
    game.add_to_hand(filler_member);
    for _ in 0..hand_extra {
        game.add_to_hand(filler_member);
    }
    let hand_before = game.state.player1.hand.cards.len();

    // Deck
    for _ in 0..deck_count {
        game.state.player1.main_deck.cards.push(filler_member);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    (ai, filler_member, hand_before, deck_before, discard_before)
}

/// POSITIVE: Deploy 3 members (filler1, filler2, Ai) → debut_count=3 →
/// condition passes → draw-until-5 fires.
///
/// Hand starts at 6 (Ai + 2 fillers + 3 extra). After 3 deploys → hand=3.
/// draw-until-5 draws 2 → hand=5.
#[test]
fn ai_bp3_three_deploys_draws_to_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, filler, _hand_before, deck_before, discard_before) = setup_ai_bp3(&mut game, 3, 10);

    // Deploy #1: filler to Left. debut_count becomes 1. Ai not on stage → no trigger.
    game.play_to_stage(filler, MemberArea::LeftSide);

    // Deploy #2: filler to Center. debut_count becomes 2. Ai not on stage → no trigger.
    game.play_to_stage(filler, MemberArea::Center);

    // Deploy #3 (Ai) to Right. debut_count becomes 3. Ai is now on stage →
    // auto-ability scanned → condition debut_count >= 3 passes →
    // draw-until-count (target=5) fires.
    game.play_to_stage(ai, MemberArea::RightSide);

    // After 3 deploys, hand went from 6 → 3 (3 cards used).
    // draw-until-5: current=3, target=5, draws 2.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Hand should be 5 (3 after deploys + 2 drawn)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "Deck should lose 2 cards (drawn to reach 5)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No cards should be in waitroom (no cost)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], ai,
        "Ai should be on Right stage"
    );
}

/// NEGATIVE: Only 2 members deployed (filler, Ai) → debut_count=2 < 3 →
/// condition fails → no draw.
///
/// Hand starts at 5 (Ai + 1 filler + 3 extra). After 2 deploys → hand=3.
/// Condition check fails. Hand stays at 3.
#[test]
fn ai_bp3_only_two_deploys_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, filler, _hand_before, deck_before, discard_before) = setup_ai_bp3(&mut game, 2, 10);

    // Deploy #1: filler to Left. debut_count=1. Ai not on stage.
    game.play_to_stage(filler, MemberArea::LeftSide);

    // Deploy #2: Ai to Center. debut_count=2. Ai on stage, auto-scan fires,
    // condition checks debut_count >= 3 → false. No draw.
    game.play_to_stage(ai, MemberArea::Center);

    // Hand: 5 start - 2 deploys = 3. No draw happened (condition failed).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Hand should be 3 (2 deploys, no draw)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "Deck should be untouched (no draw)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No discard"
    );
}

/// POSITIVE: Deploy 3 members starting from an almost-empty hand
/// (Ai + 2 fillers, 0 extra). After all 3 deploys, hand is empty (0 cards).
/// draw-until-5 draws all 5 from deck.
///
/// This tests the boundary where hand is below 5 after deploys.
#[test]
fn ai_bp3_draw_from_empty_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, filler, _hand_before, deck_before, discard_before) = setup_ai_bp3(&mut game, 0, 10);

    // Hand = [ai, filler, filler] = 3 cards.
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(filler, MemberArea::Center);
    game.play_to_stage(ai, MemberArea::RightSide);

    // After 3 deploys: hand = 0. draw-until-5 draws 5 → hand = 5.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Hand should be 5 (0 after deploys + 5 drawn)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "Deck should lose 5 cards"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No discard"
    );
}

/// BOUNDARY: Empty deck. 3 members deployed, condition passes,
/// execute_draw_until_count runs but can't draw anything. No hang/crash.
#[test]
fn ai_bp3_empty_deck_no_hang() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, filler, _hand_before, deck_before, discard_before) = setup_ai_bp3(&mut game, 3, 0);

    assert_eq!(deck_before, 0, "Deck should be empty");

    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(filler, MemberArea::Center);
    game.play_to_stage(ai, MemberArea::RightSide);

    // After 3 deploys: hand = 3. draw-until-5 tries but deck is empty → 0 drawn.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Hand should be 3 (nothing to draw, empty deck)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck should remain empty"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No discard"
    );
}

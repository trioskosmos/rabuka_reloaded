/// Tests for Q38 — "ライブカード" (live card) refers to cards in the live card zone.
///
/// Cards tested:
///   PL!N-bp1-029-L (Eutopia) ab#0: ライブ開始時 — live_card_zone ≥ 3 → score +2
///   PL!HS-bp1-004-R+ (レインボー) ab#1: ライブ開始時 — member on stage, pay 1E → per live card, gain 1 blade
///
/// Q38: "ライブカード" means cards placed in the live card zone.
mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

/// Eutopia: LiveStart condition checks live_card_zone count (≥3 → +2 score).
#[test]
fn eutopia_q38_score_condition_checks_live_card_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eutopia = game.id("PL!N-bp1-029-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(eutopia);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [filler, filler, -1];

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(eutopia);
    game.pass(); // LiveCardSetP1 → P2
    game.pass(); // LiveCardSetP2 → FirstAttackerPerformance (LiveStart triggers)

    // Only 1 live card in zone, needs 3 → no bonus
    assert_eq!(game.state.get_score_modifier(eutopia), 0,
        "Q38: Only 1 live card, needs 3 → no score");
}

/// Eutopia with 3 live cards → condition met → score +2.
#[test]
fn eutopia_q38_three_live_cards_score_plus_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eutopia = game.id("PL!N-bp1-029-L");
    let filler = game.id("PL!-sd1-010-SD");
    let live2 = game.id("PL!-sd1-019-SD");
    let live3 = game.id("PL!-sd1-028-SD");

    game.state.player1.hand.cards.push(eutopia);
    game.state.player1.hand.cards.push(live2);
    game.state.player1.hand.cards.push(live3);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [filler, filler, -1];

    game.state.player1.main_deck.cards.clear();
    for _ in 0..60 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..60 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(eutopia);
    game.set_live_card(live2);
    game.set_live_card(live3);
    game.pass(); // LiveCardSetP1 → P2 (draws 3 for P1)
    game.pass(); // LiveCardSetP2 → FirstAttackerPerformance

    while game.has_pending_choice() { game.select_indices(&[0]); }

    assert_eq!(game.state.get_score_modifier(eutopia), 2,
        "Q38: 3 live cards → score +2");
}

/// Rainbow: member on stage with LiveStart ability. Pays 1E → per live card in zone, gain 1 blade.
#[test]
fn rainbow_q38_member_on_stage_per_live_card_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rainbow = game.id("PL!HS-bp1-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live1 = game.id("PL!-sd1-019-SD");

    // Rainbow is a MEMBER card — place on stage
    game.state.player1.stage.stage = [rainbow, filler, -1];
    // Set a live card in the zone
    game.state.player1.hand.cards.push(live1);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live1);
    game.pass(); // LiveCardSetP1 → P2
    game.pass(); // LiveCardSetP2 → FirstAttackerPerformance

    // Pay optional energy (1) for Rainbow's LiveStart ability
    while game.has_pending_choice() { game.select_option(1); }
    while game.has_pending_choice() { game.select_indices(&[0]); }

    // 1 live card in zone → per_unit → gain 1 blade
    assert_eq!(game.state.get_blade_modifier(rainbow), 1,
        "Q38: 1 live card in zone, Rainbow on stage → 1 blade");
}

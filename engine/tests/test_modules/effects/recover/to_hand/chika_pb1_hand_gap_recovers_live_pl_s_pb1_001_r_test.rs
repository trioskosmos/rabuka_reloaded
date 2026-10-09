use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const CHIKA: &str = "PL!S-pb1-001-R";
const FILLER: &str = "PL!-sd1-010-SD";
const LIVE: &str = "PL!-sd1-020-SD";

fn setup() -> TestGame {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game
}

#[test]
fn chika_pb1_hand_gap_over_two_recovers_live_from_waitroom() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    // P1: chika only (1 → 0 after debut). P2: 4 → gap 4 ≥ 2.
    game.add_to_hand(chika);
    for _ in 0..4 {
        game.add_to_hand_for(Side::P2, game.new_id(FILLER));
    }
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "gap 4 ≥ 2 → live recovered"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "live left the waitroom"
    );
}

#[test]
fn chika_pb1_hand_gap_two_recovers_live_from_waitroom() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    // P1 hand: chika only (1). P2 hand: 3 → gap 2.
    game.add_to_hand(chika);
    for _ in 0..3 {
        game.add_to_hand_for(Side::P2, game.new_id(FILLER));
    }
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "P2 hand - P1 hand = 2 → live recovered"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "live left the waitroom"
    );
}

#[test]
fn chika_pb1_hand_gap_one_does_not_recover() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    // P1: chika + residual (2 → 1 after she leaves). P2: 2 → gap 1 < 2.
    game.add_to_hand(chika);
    game.add_to_hand(game.new_id(FILLER));
    for _ in 0..2 {
        game.add_to_hand_for(Side::P2, game.new_id(FILLER));
    }
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "gap 1 is below the printed 2 → no recovery"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "live stays in the waitroom"
    );
}

#[test]
fn chika_pb1_equal_hands_do_not_recover() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    // P1 hand: chika + filler (2). P2 hand: 2 → gap 0.
    game.add_to_hand(chika);
    game.add_to_hand(game.new_id(FILLER));
    for _ in 0..2 {
        game.add_to_hand_for(Side::P2, game.new_id(FILLER));
    }
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "equal hands → no recovery"
    );
}

#[test]
fn chika_pb1_empty_p2_hand_does_not_recover() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    game.add_to_hand(chika);
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "P2 hand empty → no recovery"
    );
}

#[test]
fn chika_pb1_empty_own_hand_meets_gap_of_two() {
    let mut game = setup();
    let chika = game.id(CHIKA);
    let live = game.id(LIVE);
    game.state.player1.waitroom.cards.push(live);
    // P1 hand is empty after the debut (chika left it), so own_count = 0 and
    // the printed threshold is 0 + 2 = 2. P2 holds exactly 2 → must fire.
    game.add_to_hand(chika);
    for _ in 0..2 {
        game.add_to_hand_for(Side::P2, game.new_id(FILLER));
    }
    game.give_energy(15);
    game.play_to_stage(chika, MemberArea::Center);
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "own hand empty, opponent holds 2 → gap of 2 is met"
    );
}

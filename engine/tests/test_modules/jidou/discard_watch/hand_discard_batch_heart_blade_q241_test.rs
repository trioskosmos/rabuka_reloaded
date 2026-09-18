use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn heart01_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart01)
}

fn blade_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .blade_modifiers
        .get(&card_id)
        .map(|e| e.total())
        .unwrap_or(0)
}

/// PL!HS-pb1-003-R 大沢瑠璃乃: Debut + Auto each_time (Q241).
///
/// ab#0 (Debut): Discard any number of みらくらぱーく！ cards from hand,
///               then draw (discarded + 1) cards.
/// ab#1 (Auto/turn2): Each time 1+ cards go hand→discard, gain heart01+blade.
///               Fires once per batch regardless of card count. (Q241)
#[test]
fn hand_discard_batch_q241_two_discarded_auto_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-pb1-003-R");
    let miraku = game.id("PL!HS-sd1-011-SD");
    let miraku2 = game.new_id("PL!HS-sd1-011-SD");
    let miraku3 = game.new_id("PL!HS-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.add_to_hand(miraku);
    game.add_to_hand(miraku2);
    game.add_to_hand(miraku3);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Drain choices: first SelectAutoAbility for debut auto-ordering,
    // then SelectCard for choosing which cards to discard
    let mut step = 0;
    while game.has_pending_choice() && step < 20 {
        step += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectCard") => {
                if step <= 2 {
                    // First SelectCard: pick 2 cards to discard
                    game.select_indices(&[0, 1]);
                } else {
                    // Second SelectCard ("select more?"): skip (done)
                    game.select_indices(&[]);
                }
            }
            _ => game.select_indices(&[]),
        }
    }

    // Q241: 2 cards discarded simultaneously → each_time fires ONCE per batch
    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 2 cards discarded in one batch → auto fires once (heart01=1)"
    );
    assert_eq!(
        blade_mod(&game, rurino),
        1,
        "Q241: 2 cards discarded in one batch → auto fires once (blade=1)"
    );
}

/// Q241 edge: 1 card discarded → fires once (control).
#[test]
fn hand_discard_batch_q241_one_discarded_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-pb1-003-R");
    let miraku = game.id("PL!HS-sd1-011-SD");
    let miraku2 = game.new_id("PL!HS-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.add_to_hand(miraku);
    game.add_to_hand(miraku2);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    let mut step = 0;
    while game.has_pending_choice() && step < 20 {
        step += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectCard") => {
                if step <= 2 {
                    game.select_indices(&[0]); // discard 1
                } else {
                    game.select_indices(&[]); // done
                }
            }
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 1 card discarded → fires once (heart01=1)"
    );
}

/// Q241 edge: 3 cards discarded → still fires once.
#[test]
fn hand_discard_batch_q241_three_discarded_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-pb1-003-R");
    let miraku = game.id("PL!HS-sd1-011-SD");
    let miraku2 = game.new_id("PL!HS-sd1-011-SD");
    let miraku3 = game.new_id("PL!HS-sd1-011-SD");
    let miraku4 = game.new_id("PL!HS-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.add_to_hand(miraku);
    game.add_to_hand(miraku2);
    game.add_to_hand(miraku3);
    game.add_to_hand(miraku4);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    let mut step = 0;
    while game.has_pending_choice() && step < 20 {
        step += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectCard") => {
                if step <= 2 {
                    game.select_indices(&[0, 1, 2]); // discard 3
                } else {
                    game.select_indices(&[]); // done
                }
            }
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 3 cards discarded in one batch → still fires once (heart01=1)"
    );
}

/// Q241 edge: discard 0 → no trigger.
#[test]
fn hand_discard_batch_q241_zero_discarded_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-pb1-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    // No miraku cards in hand → can only discard 0

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        heart01_mod(&game, rurino),
        0,
        "Q241: 0 cards discarded → no trigger"
    );
}

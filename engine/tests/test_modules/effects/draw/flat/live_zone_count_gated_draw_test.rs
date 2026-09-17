use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn bp5_006_live_start_draws_when_live_zone_has_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R"); // cost 2
    let filler = game.id("PL!-sd1-010-SD");
    let old_live_a = game.id("PL!-sd1-019-SD");
    let old_live_b = game.id("PL!-sd1-021-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);

    // Two previous lives in the live card zone.
    game.state.player1.live_card_zone.cards.push(old_live_a);
    game.state.player1.live_card_zone.cards.push(old_live_b);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Two passes leave the live-card-set phase:
    //   pass 1 (FA→SA): P1 refills by P1's live-zone count,
    //   pass 2 (SA→performance): P2 refills by P2's (empty) zone → +0,
    //   then ライブ開始時 fires → conditional +1.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone + 1,
        "P1 live-zone refill (+{p1_zone}) + conditional draw (+1)"
    );
}

fn drain_live_start_prompts(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            _ => game.select_indices(&[]),
        }
    }
}

#[test]
fn bp5_006_live_start_draws_at_exactly_two_cards_boundary() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let old_live = game.id("PL!-sd1-019-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(old_live);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Boundary EXACTLY 2: 1 pre-existing live + this live = 2 in the zone
    // at live start → condition met (+1) on top of P1's refill (+2).
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    assert_eq!(p1_zone, 2, "boundary precondition: exactly 2 in zone");
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone + 1,
        "exactly 2 meets 2枚以上 → refill (+2) + conditional draw (+1)"
    );
}

#[test]
fn bp5_006_live_start_no_draw_when_zone_below_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "precondition: empty live card zone"
    );

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Only THIS live is in the zone (1 < 2) → P1 refills +1, no
    // conditional draw.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    assert_eq!(p1_zone, 1, "precondition: only the set live in zone");
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone,
        "zone 1 < 2 → refill (+1) only, no ability draw"
    );
}

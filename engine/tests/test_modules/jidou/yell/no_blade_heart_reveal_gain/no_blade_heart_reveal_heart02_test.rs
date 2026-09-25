use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn reveal_yell_cards_and_scan_auto_abilities(game: &mut TestGame, count: u8) -> Vec<i16> {
    let pid = game.state.player1.id.clone();
    game.state.perform_cheer_check(&pid, count).unwrap();
    let revealed: Vec<i16> = game.state.resolution_zone.cards.iter().copied().collect();
    assert!(
        !revealed.is_empty(),
        "yell should reveal at least one card (test must be meaningful)"
    );
    for &cid in &revealed {
        game.state.push_revealed_card(cid, None, false, Some(0), "yell");
    }
    game.state.yell_occurred = !revealed.is_empty();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);
    game.state.yell_occurred = false;
    game.drain_auto_ability_choices();
    revealed
}

#[test]
fn yell_without_blade_heart_reveal_grants_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp2-020-N");
    game.state.player1.stage.set_area(MemberArea::Center, natsumi);

    // Deck of NON-blade-heart members so the yell reveals none with a blade heart.
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(NO_BLADE_HEART));
    }

    reveal_yell_cards_and_scan_auto_abilities(&mut game, 3);

    assert_eq!(
        heart_modifier(&game, natsumi, HeartColor::Heart02),
        1,
        "鬼塚夏美 ab#0 should gain heart02 when a yell reveals no blade-heart card"
    );
}

#[test]
fn yell_with_blade_heart_reveal_does_not_grant_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp2-020-N");
    game.state.player1.stage.set_area(MemberArea::Center, natsumi);

    // The standard FILLER (PL!-sd1-010-SD) HAS a blade heart (b_heart03), so a yell
    // revealing it must violate the "no blade-heart card" negation.
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    reveal_yell_cards_and_scan_auto_abilities(&mut game, 3);

    assert_eq!(
        heart_modifier(&game, natsumi, HeartColor::Heart02),
        0,
        "鬼塚夏美 must NOT gain heart02 when a blade-heart card is revealed (negation as written)"
    );
}

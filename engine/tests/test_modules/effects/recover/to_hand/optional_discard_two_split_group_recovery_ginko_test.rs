use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn setup(live_count: usize) -> (TestGame, i16, [i16; 2], i16, i16, i16) {
    let mut game = TestGame::new(load_real_database());
    let ginko = game.id("PL!HS-pb1-020-N");
    assert_eq!(game.db.get_card(ginko).unwrap().card_no, "PL!HS-pb1-020-N");
    let cost = [game.new_id("PL!-sd1-010-SD"), game.new_id("PL!-sd1-010-SD")];
    let member = game.id("PL!HS-sd1-001-SD");
    let wrong_member = game.id("PL!HS-sd1-002-SD");
    let live = game.id("PL!HS-pb1-026-L");
    let other_live = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage[1] = ginko;
    game.state.player1.hand.cards.extend(cost);
    game.state.player1.waitroom.cards.extend([wrong_member, member]);
    if live_count > 0 {
        game.state.player1.waitroom.cards.push(live);
    }
    for _ in 1..live_count {
        let copy = game.new_id("PL!-sd1-019-SD");
        game.state.player1.waitroom.cards.push(copy);
    }
    fill_decks(&mut game, other_live);
    (game, ginko, cost, member, wrong_member, live)
}

#[test]
fn ginko_three_lives_pay_two_recovers_only_cerise_member_and_hasunosora_live() {
    let (mut game, ginko, cost, member, wrong_member, live) = setup(3);
    fire_trigger(&mut game, ginko, AbilityTrigger::Debut, "登場");
    game.select_indices(&[0, 1]);
    for _ in 0..4 {
        if !game.has_pending_choice() {
            break;
        }
        game.select_indices(&[0]);
    }
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.len(), 2);
    assert!(game.state.player1.hand.cards.contains(&member));
    assert!(game.state.player1.hand.cards.contains(&live));
    assert!(game.state.player1.waitroom.cards.contains(&wrong_member));
    for cid in cost {
        assert!(game.state.player1.waitroom.cards.contains(&cid));
    }
    assert!(!game.state.player1.waitroom.cards.contains(&member));
    assert!(!game.state.player1.waitroom.cards.contains(&live));
}

#[test]
fn ginko_declines_pay_two_and_recovers_nothing() {
    let (mut game, ginko, cost, member, wrong_member, live) = setup(3);
    let waitroom = game.state.player1.waitroom.cards.clone();
    fire_trigger(&mut game, ginko, AbilityTrigger::Debut, "登場");
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), cost.as_slice());
    assert_eq!(game.state.player1.waitroom.cards, waitroom);
    assert!(game.state.player1.waitroom.cards.contains(&member));
    assert!(game.state.player1.waitroom.cards.contains(&wrong_member));
    assert!(game.state.player1.waitroom.cards.contains(&live));
}

#[test]
fn ginko_two_waitroom_lives_do_not_offer_cost_or_recovery() {
    let (mut game, ginko, cost, _, _, _) = setup(2);
    let waitroom = game.state.player1.waitroom.cards.clone();
    fire_trigger(&mut game, ginko, AbilityTrigger::Debut, "登場");
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), cost.as_slice());
    assert_eq!(game.state.player1.waitroom.cards, waitroom);
}

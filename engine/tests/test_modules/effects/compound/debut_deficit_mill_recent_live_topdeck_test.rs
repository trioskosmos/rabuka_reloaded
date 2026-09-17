use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SOURCE: &str = "PL!S-PR-044-PR";
const MEMBER: &str = "PL!-sd1-010-SD";
const LIVE: &str = "PL!-sd1-019-SD";

fn deficit_mill(wait_count: usize, mill_live: bool, take: bool) {
    let mut game = TestGame::new(load_real_database());
    let source = game.id(SOURCE);
    assert_eq!(game.db.get_card(source).unwrap().card_no, SOURCE);
    let filler = game.id(MEMBER);
    fill_decks(&mut game, filler);
    let old_live = game.new_id(LIVE);
    game.state.player1.waitroom.cards.push(old_live);
    for _ in 1..wait_count {
        let member = game.new_id(MEMBER);
        game.state.player1.waitroom.cards.push(member);
    }
    let initial_wait = game.state.player1.waitroom.cards.to_vec();
    let deficit = 8usize.saturating_sub(wait_count);
    let mut milled = Vec::new();
    for i in 0..deficit {
        milled.push(game.new_id(if mill_live && i == 0 { LIVE } else { MEMBER }));
    }
    for &card in milled.iter().rev() {
        game.state.player1.main_deck.cards.insert(0, card);
    }
    let deck_before = game.state.player1.main_deck.cards.to_vec();
    game.give_energy(10);
    game.state.player1.hand.cards.push(source);
    game.play_to_stage(source, MemberArea::Center);
    let mut expected_wait = initial_wait;
    expected_wait.extend_from_slice(&milled);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), expected_wait);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &deck_before[deficit..]);
    if mill_live && deficit > 0 {
        game.assert_select_card("discard", 1, true);
        game.select_indices(if take { &[0] } else { &[] });
    }
    game.drain_choices_strict(&[], &[]);
    let mut expected_deck = deck_before[deficit..].to_vec();
    if take && mill_live && deficit > 0 {
        expected_wait.retain(|&cid| cid != milled[0]);
        expected_deck.insert(0, milled[0]);
    }
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), expected_wait);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), expected_deck);
    assert!(game.state.player1.waitroom.cards.contains(&old_live));
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player1.stage.stage[1], source);
    assert_eq!(game.state.player1.energy_zone.active_count(), 5);
}

#[test]
fn debut_mills_deficit_and_topdecks_only_recent_live_pr044() {
    deficit_mill(5, true, true);
}

#[test]
fn debut_mill_declined_topdeck_keeps_all_milled_cards_pr044() {
    deficit_mill(5, true, false);
}

#[test]
fn debut_mill_without_recent_live_cannot_take_old_waitroom_live_pr044() {
    deficit_mill(5, false, false);
}

#[test]
fn debut_waitroom_seven_mills_one_live_and_returns_it_pr044() {
    deficit_mill(7, true, true);
}

#[test]
fn debut_waitroom_eight_or_more_does_not_mill_or_recover_pr044() {
    for count in [8, 9] {
        deficit_mill(count, true, false);
    }
}

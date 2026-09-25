use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn skip_optional_card_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

fn advance_live_skipping_optional_cards(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        skip_optional_card_choices(game);
    }
}

fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn live_start_two_distinct_under_members_grant_at_least_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-bp7-003-R\u{ff0b}");
    let under_a = game.new_id("PL!S-bp2-001-R");
    let under_b = game.new_id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place 2 distinct members under this member
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(under_a);
        game.state.player1.stage.under_cards[idx].push(under_b);
    }
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_both_main_decks(&mut game, fid2);
    game.give_energy(15);

    advance_live_skipping_optional_cards(&mut game);

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 2, "2 distinct under-members → >= +2 blade");
}

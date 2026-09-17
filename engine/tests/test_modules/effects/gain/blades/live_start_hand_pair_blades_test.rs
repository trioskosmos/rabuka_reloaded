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
fn live_start_four_hand_cards_grant_at_least_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fid = game.id_ref("PL!-sd1-010-SD");
    let member = game.id("PL!SP-bp2-009-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(&mut game, fid);

    // 4 hand cards → 4/2 = +2 blade
    for _ in 0..4 {
        game.add_to_hand(fid);
    }
    game.give_energy(15);

    advance_live_skipping_optional_cards(&mut game);

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 2, "4 hand cards → >= +2 blade, got {blade}");
}

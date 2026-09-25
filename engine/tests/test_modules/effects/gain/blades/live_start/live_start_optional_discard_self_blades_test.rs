use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn pay_optional_costs_selecting_last_hand_card(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n > 0 { game.select_indices(&[n - 1]); } else { break; }
            }
            _ => game.select_option(1), // pay optional costs by default
        }
    }
}

fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn live_start_optional_discard_gains_at_least_one_self_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-bp1-005-R");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(&mut game, fid);
    // The optional cost needs a hand card to discard — without one the
    // ability correctly self-declines.
    let hf = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hf);
    game.give_energy(15);

    for _ in 0..7 {
        game.pass();
        pay_optional_costs_selecting_last_hand_card(&mut game);
    }

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 1, "optional discard paid → at least +1 blade");
}

#[test]
fn live_start_optional_discard_moves_one_hand_card_and_gains_two_self_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-sd1-004-SD");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = rin;
    fill_both_main_decks(&mut game, fid);
    let hf = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hf);
    game.give_energy(10);

    // Same proven flow as nsd1_001: pass through phases, paying optional
    // costs as they appear.
    for _ in 0..7 {
        game.pass();
        pay_optional_discard_stopping_on_empty_hand(&mut game);
    }

    let blade = game.state.mods.get_blade_modifier(rin);
    assert_eq!(
        blade, 2,
        "paying the optional discard → exactly +2 blade (ブレード2)"
    );

    // The paid cost must have moved one hand card to the waitroom.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "one hand card was discarded as the cost"
    );
}

fn pay_optional_discard_stopping_on_empty_hand(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                let n = game.state.player1.hand.cards.len();
                if n == 0 {
                    break;
                }
                game.select_indices(&[n - 1]);
            }
            _ => game.select_option(1),
        }
    }
}

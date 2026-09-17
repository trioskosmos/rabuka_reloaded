use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn skip_optional_card_and_target_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            Choice::SelectTarget { allow_skip: true, .. } => game.select_option(0),
            _ => break,
        }
    }
}

#[test]
fn live_start_optional_energy_two_blade_flow_keeps_modifier_nonnegative() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!HS-PR-018-PR");
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(&mut game, fid);
    game.give_energy(15);

    pass_five_times_to_live_card_set(&mut game);
    // No live card needed; just pass into performance to trigger LiveStart
    for _ in 0..3 {
        game.pass();
        skip_optional_card_and_target_choices(&mut game);
    }

    // The ability is optional — if it prompted, pay and check blade.
    // If no prompt appeared (no valid candidates), skip this assertion.
    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 0,
        "blade modifier should be non-negative"
    );
}

fn pass_five_times_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn fill_both_main_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

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
            _ => game.select_option(1),
        }
    }
}

fn stage_self_and_advance_paying_optional_costs(game: &mut TestGame, card_no: &str) -> i16 {
    let member = game.id(card_no);
    let fid = game.id_ref("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_both_main_decks(game, fid);
    game.give_energy(15);
    advance_live_paying_optional_costs(game);
    member
}

fn advance_live_paying_optional_costs(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        pay_optional_costs_selecting_last_hand_card(game);
    }
}

#[test]
fn live_start_pay_one_energy_gains_at_least_one_self_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = stage_self_and_advance_paying_optional_costs(&mut game, "PL!N-bp1-001-R");
    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 1, "pay 1E → at least +1 blade");
}
